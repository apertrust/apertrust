// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! Apertrust's folders (architecture decision 43; `docs/settings.md`, User data
//! folders): [`Folders`], the one place that decides where a program
//! keeps what it keeps, and how each folder was chosen. Never a path built by hand
//! elsewhere.
//!
//! The **user data folder** holds everything the person made or chose: `settings.toml`
//! today, later `state.toml`, `themes/`, styles, presets, the library index and reports.
//! It is chosen in this order:
//!
//! - `--user-data-dir DIR`, or one of its aliases, `--config-dir` and darktable's
//!   `--configdir` ([`FLAGS`]), which each binary reads from its own command line and
//!   hands over as a [`Given`]; given more than once, the last;
//! - then [`VARIABLE`], `APERTRUST_USER_DATA_DIR`, unless it is empty, as XDG's are;
//! - then the platform's folder, where settings have always been, so nothing moves:
//!   `~/Library/Application Support/Apertrust` on macOS, `%APPDATA%\Apertrust\config` on
//!   Windows, and on Linux `$XDG_CONFIG_HOME/apertrust`, by default `~/.config/apertrust`,
//!   one whole folder, never split by XDG's kinds. A development build's is Apertrust
//!   Dev's instead (`platform_folder`): `Apertrust Dev` for `Apertrust` on macOS and
//!   Windows, `apertrust-dev` on Linux (decision 43).
//!
//! A leading `~` is the home folder, as darktable has it: `~` alone or `~/…`, never another
//! user's `~ana`, since the shell leaves it after `=` and in quotes. A relative folder is
//! taken from the current folder. For a program that writes, as the app ([`Folders::find`]),
//! a missing one is created, and one given by flag or variable that cannot be created or
//! written stops the start, naming it ([`Error`]), and never falls back to the platform's,
//! which is what the person asked to keep away from. The platform's, when it cannot be
//! created, is warned of and used all the same, as before: the store then warns of each
//! write that fails. A program that only reads, as the CLI ([`Folders::find_to_read`]),
//! creates and probes nothing: one given that is missing or cannot be read stops it, and
//! a missing platform folder stays missing, its settings the defaults. `-d settings` logs
//! the folder and how it was chosen.
//!
//! Until edits are kept in it (decision 43's edits task), a user data folder other than
//! the platform's separates settings only: edits are still written beside the photos.
//!
//! The **cache folder**, for what Apertrust can compute again, comes with the first cache,
//! the thumbnails: `<user data>/cache` for a user data folder given explicitly
//! ([`Chosen::explicit`]), the platform's cache folder otherwise, unless `--cache-dir` or
//! `APERTRUST_CACHE_DIR` names another. It joins [`Folders`] beside the user data folder.

use std::ffi::OsString;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use directories::{BaseDirs, ProjectDirs};

/// The flags that name the user data folder: `--user-data-dir`, as Chromium's, VS Code's
/// and Electron's, then its aliases, `--config-dir` and darktable's `--configdir`, so that
/// darktable users' habit works.
pub const FLAGS: [&str; 3] = ["--user-data-dir", "--config-dir", "--configdir"];

/// The environment variable that names the user data folder where no flag does.
pub const VARIABLE: &str = "APERTRUST_USER_DATA_DIR";

/// A user data folder named on the command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Given {
    /// The flag that named it, one of [`FLAGS`].
    pub flag: &'static str,
    /// The folder as given, relative or not.
    pub folder: PathBuf,
}

/// How the user data folder was chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chosen {
    /// By a flag on the command line, the one given, one of [`FLAGS`].
    Flag(&'static str),
    /// By [`VARIABLE`].
    Variable,
    /// The platform's, as neither a flag nor the variable named one.
    Platform,
}

impl Chosen {
    /// Whether the folder was given, by flag or variable, rather than the platform's: such
    /// a folder never falls back to the platform's, and will keep its cache inside it.
    pub fn explicit(self) -> bool {
        self != Chosen::Platform
    }
}

impl fmt::Display for Chosen {
    /// As the log and errors name it: the flag, the variable, or the platform.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Chosen::Flag(flag) => flag,
            Chosen::Variable => VARIABLE,
            Chosen::Platform => "the platform",
        })
    }
}

/// Why a user data folder given by flag or variable cannot be used: the start stops.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{flag} names no folder")]
    Empty { flag: &'static str },
    #[error(
        "cannot find the user data folder {} given by {chosen}: no home folder on this system",
        folder.display()
    )]
    Home { folder: PathBuf, chosen: Chosen },
    #[error(
        "cannot find the user data folder {} given by {chosen}: the current folder: {source}",
        folder.display()
    )]
    Relative {
        folder: PathBuf,
        chosen: Chosen,
        source: io::Error,
    },
    #[error("cannot create the user data folder {} given by {chosen}: {source}", folder.display())]
    Create {
        folder: PathBuf,
        chosen: Chosen,
        source: io::Error,
    },
    #[error("cannot write to the user data folder {} given by {chosen}: {source}", folder.display())]
    Write {
        folder: PathBuf,
        chosen: Chosen,
        source: io::Error,
    },
    #[error("cannot find the user data folder {} given by {chosen}: no such folder", folder.display())]
    Missing { folder: PathBuf, chosen: Chosen },
    #[error("cannot read the user data folder {} given by {chosen}: {source}", folder.display())]
    Read {
        folder: PathBuf,
        chosen: Chosen,
        source: io::Error,
    },
}

/// Apertrust's folders for a run, chosen once as it starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folders {
    /// The user data folder, absolute, which exists unless the platform's could not be
    /// created or, for a program that only reads, is missing.
    user_data: PathBuf,
    chosen: Chosen,
    // The cache folder joins here with the first cache (the module's docs).
}

impl Folders {
    /// Chooses the folders: the user data folder `given` names on the command line, else
    /// [`VARIABLE`]'s, else the platform's, Apertrust Dev's for a `dev` build
    /// (`Program::dev`), created where it is missing, and logs it under `-d settings`.
    /// None, with a warning, where neither names one and the system gives none, as on a
    /// system with no home folder: nothing is kept then.
    pub fn find(given: Option<Given>, dev: bool) -> Result<Option<Folders>, Error> {
        Folders::look(given, dev, true)
    }

    /// Chooses the folders as [`find`](Folders::find) does, for a program that reads them
    /// and writes nothing, as the CLI: it creates no folder and writes nothing to see
    /// whether one can be written. One given by flag or variable that is missing or cannot
    /// be read is an [`Error`] naming it; the platform's, when missing, is chosen all the
    /// same and stays missing, so its settings are the defaults.
    pub fn find_to_read(given: Option<Given>, dev: bool) -> Result<Option<Folders>, Error> {
        Folders::look(given, dev, false)
    }

    /// Chooses the folders for a `dev` build or not, for a program that `writes` them,
    /// or only reads them.
    fn look(given: Option<Given>, dev: bool, writes: bool) -> Result<Option<Folders>, Error> {
        let platform = ProjectDirs::from_path(platform_folder(dev).into());
        let platform = platform.map(|dirs| dirs.config_dir().to_owned());
        let home = BaseDirs::new().map(|dirs| dirs.home_dir().to_owned());
        let variable = std::env::var_os(VARIABLE);
        choose(
            given,
            variable,
            platform,
            home,
            std::env::current_dir,
            writes,
        )
    }

    /// The user data folder.
    pub fn user_data(&self) -> &Path {
        &self.user_data
    }

    /// How the user data folder was chosen.
    pub fn chosen(&self) -> Chosen {
        self.chosen
    }

    /// The settings file, `settings.toml` in the user data folder, which may not exist
    /// yet.
    pub fn settings(&self) -> PathBuf {
        self.user_data.join("settings.toml")
    }
}

/// The platform folder's own part, which `directories` puts in the system's place:
/// Apertrust's, or a development build's, Apertrust Dev's, beside it. Lower case on
/// Linux, with a hyphen for the space, as the stable folder's `apertrust` is.
fn platform_folder(dev: bool) -> &'static str {
    let lower = !cfg!(any(target_os = "macos", target_os = "windows"));
    match (dev, lower) {
        (false, false) => "Apertrust",
        (false, true) => "apertrust",
        (true, false) => "Apertrust Dev",
        (true, true) => "apertrust-dev",
    }
}

/// [`Folders::find`], or for a program that does not write, [`Folders::find_to_read`], with
/// what it reads handed in: the variable's value, the platform's folder, the home folder
/// and the current folder, asked for only for a relative folder, so that tests give
/// folders of their own.
fn choose(
    given: Option<Given>,
    variable: Option<OsString>,
    platform: Option<PathBuf>,
    home: Option<PathBuf>,
    current: impl FnOnce() -> io::Result<PathBuf>,
    writes: bool,
) -> Result<Option<Folders>, Error> {
    let named = match (given, variable) {
        (Some(Given { flag, folder }), _) if folder.as_os_str().is_empty() => {
            return Err(Error::Empty { flag });
        }
        (Some(Given { flag, folder }), _) => Some((folder, Chosen::Flag(flag))),
        (None, Some(folder)) if !folder.is_empty() => Some((folder.into(), Chosen::Variable)),
        (None, _) => None,
    };
    if let Some((folder, chosen)) = named {
        return explicit(folder, chosen, home, current, writes).map(Some);
    }
    let Some(folder) = platform else {
        tracing::warn!(target: "settings", "no user data folder on this system: nothing kept");
        return Ok(None);
    };
    let missing = !folder.is_dir();
    if writes
        && missing
        && let Err(e) = std::fs::create_dir_all(&folder)
    {
        let folder = folder.display();
        tracing::warn!(target: "settings", "cannot create the user data folder {folder}: {e}");
    }
    let note = match (missing, folder.is_dir()) {
        (true, true) => ", created",
        (true, false) if !writes => ", missing",
        _ => "",
    };
    log(&folder, Chosen::Platform, note);
    Ok(Some(Folders {
        user_data: folder,
        chosen: Chosen::Platform,
    }))
}

/// The user data folder given by flag or variable, a leading `~` the home folder, taken
/// from the current folder if relative; for a program that `writes`, created if missing
/// and refused unless it can be written, and otherwise refused unless it is there and can
/// be read.
fn explicit(
    folder: PathBuf,
    chosen: Chosen,
    home: Option<PathBuf>,
    current: impl FnOnce() -> io::Result<PathBuf>,
    writes: bool,
) -> Result<Folders, Error> {
    // By component, so `~ana` and `dev/~` stay as they are.
    let below = folder.strip_prefix("~").ok().map(Path::to_owned);
    let folder = match (below, home) {
        (None, _) => folder,
        (Some(below), Some(home)) if below.as_os_str().is_empty() => home,
        (Some(below), Some(home)) => home.join(below),
        (Some(_), None) => return Err(Error::Home { folder, chosen }),
    };
    let folder = if folder.is_relative() {
        match current() {
            Ok(current) => current.join(folder),
            Err(source) => {
                return Err(Error::Relative {
                    folder,
                    chosen,
                    source,
                });
            }
        }
    } else {
        folder
    };
    if !writes {
        if !folder.is_dir() {
            return Err(Error::Missing { folder, chosen });
        }
        if let Err(source) = std::fs::read_dir(&folder) {
            return Err(Error::Read {
                folder,
                chosen,
                source,
            });
        }
        log(&folder, chosen, "");
        return Ok(Folders {
            user_data: folder,
            chosen,
        });
    }
    let missing = !folder.is_dir();
    if let Err(source) = std::fs::create_dir_all(&folder) {
        return Err(Error::Create {
            folder,
            chosen,
            source,
        });
    }
    if let Err(source) = writable(&folder) {
        return Err(Error::Write {
            folder,
            chosen,
            source,
        });
    }
    log(&folder, chosen, if missing { ", created" } else { "" });
    Ok(Folders {
        user_data: folder,
        chosen,
    })
}

/// Whether `folder` can be written, as the store will write it: a file of this process's
/// own written in it, then removed. Its permissions alone cannot tell, for a folder
/// another user owns, a read-only volume or an access list.
fn writable(folder: &Path) -> io::Result<()> {
    let probe = folder.join(format!(".apertrust-write-check-{}.tmp", std::process::id()));
    let written = std::fs::write(&probe, b"");
    let _ = std::fs::remove_file(&probe);
    written
}

/// The folder chosen, under `-d settings`, with `note` (`, created` or `, missing`): its
/// whole path, as the store's first line has the file's.
fn log(folder: &Path, chosen: Chosen, note: &str) {
    let folder = folder.display();
    tracing::debug!(target: "settings", "user data folder: {folder}, from: {chosen}{note}");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of a test's own, empty, removed when the test ends: never the user's.
    struct Folder(PathBuf);

    impl Folder {
        fn new(name: &str) -> Folder {
            let path = std::env::temp_dir().join(format!(
                "apertrust-base-folders-{name}-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("a temporary folder");
            Folder(path)
        }

        fn join(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A current folder no test with an absolute folder may ask for.
    fn unasked() -> io::Result<PathBuf> {
        panic!("the current folder asked for an absolute folder")
    }

    /// The folders a program that writes chooses ([`Folders::find`]).
    fn choose(
        given: Option<Given>,
        variable: Option<OsString>,
        platform: Option<PathBuf>,
        home: Option<PathBuf>,
        current: impl FnOnce() -> io::Result<PathBuf>,
    ) -> Result<Option<Folders>, Error> {
        super::choose(given, variable, platform, home, current, true)
    }

    /// The folders a program that only reads chooses ([`Folders::find_to_read`]).
    fn read(
        given: Option<Given>,
        variable: Option<OsString>,
        platform: Option<PathBuf>,
        home: Option<PathBuf>,
        current: impl FnOnce() -> io::Result<PathBuf>,
    ) -> Result<Option<Folders>, Error> {
        super::choose(given, variable, platform, home, current, false)
    }

    fn given(flag: &'static str, folder: impl Into<PathBuf>) -> Option<Given> {
        let folder = folder.into();
        Some(Given { flag, folder })
    }

    fn found(chosen: Result<Option<Folders>, Error>) -> Folders {
        chosen.expect("usable").expect("a folder")
    }

    /// A development build's platform folder is Apertrust Dev's, beside Apertrust's, and
    /// Apertrust's stays where settings have always been.
    #[test]
    fn a_development_build_has_a_platform_folder_of_its_own() {
        let config = |dev| {
            let dirs = ProjectDirs::from_path(platform_folder(dev).into());
            dirs.map(|dirs| dirs.config_dir().to_owned())
        };
        let stable = ProjectDirs::from("", "", "Apertrust").map(|d| d.config_dir().to_owned());
        assert_eq!(config(false), stable, "Apertrust's, as before");
        let (Some(stable), Some(dev)) = (config(false), config(true)) else {
            return; // a system with no home folder
        };
        assert_ne!(dev, stable);
        let wanted = if cfg!(any(target_os = "macos", target_os = "windows")) {
            "Apertrust Dev"
        } else {
            "apertrust-dev"
        };
        let named = |path: &Path, name| path.components().any(|part| part.as_os_str() == name);
        assert!(named(&dev, wanted), "{}", dev.display());
    }

    #[test]
    fn a_flag_comes_first_each_alias_as_the_flag_then_the_variable_then_the_platform_s() {
        let folder = Folder::new("order");
        let [flagged, variable, platform] =
            ["flag", "variable", "platform"].map(|n| folder.join(n));
        for flag in FLAGS {
            let chosen = choose(
                given(flag, &flagged),
                Some(variable.clone().into()),
                Some(platform.clone()),
                None,
                unasked,
            );
            let chosen = found(chosen);
            assert_eq!(chosen.user_data(), flagged);
            assert_eq!(chosen.chosen(), Chosen::Flag(flag));
            assert!(chosen.chosen().explicit());
        }
        // Neither the variable's folder nor the platform's was touched.
        assert!(!variable.exists() && !platform.exists());

        let chosen = choose(
            None,
            Some(variable.clone().into()),
            Some(platform.clone()),
            None,
            unasked,
        );
        let chosen = found(chosen);
        assert_eq!(
            (chosen.user_data(), chosen.chosen()),
            (variable.as_path(), Chosen::Variable)
        );
        assert!(chosen.chosen().explicit());
        assert!(!platform.exists());

        let chosen = found(choose(None, None, Some(platform.clone()), None, unasked));
        assert_eq!(
            (chosen.user_data(), chosen.chosen()),
            (platform.as_path(), Chosen::Platform)
        );
        assert!(!chosen.chosen().explicit());
        // An empty variable is as none.
        let chosen = choose(None, Some("".into()), Some(platform.clone()), None, unasked);
        assert_eq!(found(chosen).chosen(), Chosen::Platform);
        // Nothing names one and the system gives none: nothing is kept.
        assert_eq!(
            choose(None, None, None, None, unasked).expect("no error"),
            None
        );
    }

    #[test]
    fn a_relative_folder_is_taken_from_the_current_folder() {
        let folder = Folder::new("relative");
        let current = folder.join("current");
        std::fs::create_dir_all(&current).expect("the current folder");
        let here = || Ok(current.clone());

        let chosen = found(choose(given(FLAGS[0], "udd/dev"), None, None, None, here));
        assert_eq!(chosen.user_data(), current.join("udd/dev"));
        assert!(chosen.user_data().is_dir());
        let chosen = found(choose(None, Some("variable".into()), None, None, here));
        assert_eq!(chosen.user_data(), current.join("variable"));

        // Without a current folder, a relative one cannot be found, and is refused.
        let platform = folder.join("platform");
        let gone = || Err(io::Error::new(io::ErrorKind::NotFound, "gone"));
        let refused = choose(
            given(FLAGS[2], "udd"),
            None,
            Some(platform.clone()),
            None,
            gone,
        );
        assert!(
            matches!(refused, Err(Error::Relative { ref folder, .. }) if folder == Path::new("udd")),
            "{refused:?}"
        );
        assert!(!platform.exists());
    }

    #[test]
    fn a_leading_tilde_is_the_home_folder_however_it_is_given() {
        let folder = Folder::new("home");
        let home = folder.join("home");
        let current = folder.join("current");
        let here = || Ok(current.clone());
        let at_home = |given: Option<Given>, variable: Option<&str>| {
            let variable = variable.map(OsString::from);
            found(choose(given, variable, None, Some(home.clone()), unasked))
        };
        // `--user-data-dir=~/dev`, which the shell leaves, or `~/dev` quoted: as `~/dev`.
        let chosen = at_home(given(FLAGS[0], "~/dev"), None);
        assert_eq!(chosen.user_data(), home.join("dev"));
        assert!(home.join("dev").is_dir());
        assert_eq!(at_home(given(FLAGS[2], "~"), None).user_data(), home);
        // The variable's too.
        let chosen = at_home(None, Some("~/variable"));
        assert_eq!(
            (chosen.user_data(), chosen.chosen()),
            (home.join("variable").as_path(), Chosen::Variable)
        );
        // Never another user's home, nor a `~` past the start: those are relative.
        for (named, taken) in [("~ana/dev", "~ana/dev"), ("dev/~", "dev/~")] {
            let given = given(FLAGS[0], named);
            let chosen = found(choose(given, None, None, Some(home.clone()), here));
            assert_eq!(chosen.user_data(), current.join(taken));
        }
        // Without a home folder: refused, never a folder named `~`.
        let platform = folder.join("platform");
        let refused = choose(
            given(FLAGS[0], "~/dev"),
            None,
            Some(platform.clone()),
            None,
            here,
        );
        let Err(error @ Error::Home { .. }) = refused else {
            panic!("refused: {refused:?}");
        };
        assert_eq!(
            error.to_string(),
            "cannot find the user data folder ~/dev given by --user-data-dir: no home folder \
             on this system"
        );
        assert!(!current.join("~").exists() && !platform.exists());
    }

    #[test]
    fn a_missing_folder_is_created_and_its_settings_file_is_in_it() {
        let folder = Folder::new("created");
        let nested = folder.join("one/two/udd");
        let chosen = found(choose(given(FLAGS[1], &nested), None, None, None, unasked));
        assert!(nested.is_dir());
        assert_eq!(chosen.settings(), nested.join("settings.toml"));
        // The settings file is the store's to write, and nothing of the write check stays.
        assert_eq!(std::fs::read_dir(&nested).expect("there").count(), 0);
        // An existing one is used as it is.
        std::fs::write(nested.join("settings.toml"), "version = 1\n").expect("written");
        let again = found(choose(given(FLAGS[1], &nested), None, None, None, unasked));
        assert_eq!(again, chosen);
        let kept = std::fs::read_to_string(nested.join("settings.toml")).expect("there");
        assert_eq!(kept, "version = 1\n");
        // The platform's too.
        let platform = folder.join("platform/apertrust");
        let chosen = found(choose(None, None, Some(platform.clone()), None, unasked));
        assert!(platform.is_dir());
        assert_eq!(chosen.user_data(), platform);
    }

    #[test]
    fn a_folder_given_that_cannot_be_used_stops_the_start_and_never_falls_back() {
        let folder = Folder::new("refused");
        let file = folder.join("a-file");
        std::fs::write(&file, "").expect("written");
        // Under a file: no folder can be created there, whoever runs the test.
        let under = file.join("udd");
        let platform = folder.join("platform");
        let flagged = choose(
            given(FLAGS[0], &under),
            None,
            Some(platform.clone()),
            None,
            unasked,
        );
        let Err(error @ Error::Create { .. }) = flagged else {
            panic!("refused: {flagged:?}");
        };
        let named = format!(
            "cannot create the user data folder {} given by --user-data-dir: ",
            under.display()
        );
        assert!(error.to_string().starts_with(&named), "{error}");
        let variable = choose(
            None,
            Some(under.clone().into()),
            Some(platform.clone()),
            None,
            unasked,
        );
        let Err(error @ Error::Create { .. }) = variable else {
            panic!("refused: {variable:?}");
        };
        assert!(
            error.to_string().contains("APERTRUST_USER_DATA_DIR"),
            "{error}"
        );
        // A flag that names nothing.
        let empty = choose(
            given(FLAGS[2], ""),
            None,
            Some(platform.clone()),
            None,
            unasked,
        );
        assert_eq!(
            empty.expect_err("refused").to_string(),
            "--configdir names no folder"
        );
        assert!(!platform.exists(), "fell back to the platform's");

        // The platform's own is warned of and used all the same, as before.
        let chosen = found(choose(None, None, Some(under.clone()), None, unasked));
        assert_eq!(chosen.user_data(), under);
    }

    #[cfg(unix)]
    #[test]
    fn a_folder_given_that_cannot_be_written_stops_the_start() {
        use std::os::unix::fs::PermissionsExt;
        let folder = Folder::new("unwritable");
        let locked = folder.join("locked");
        std::fs::create_dir_all(&locked).expect("created");
        let mode = |mode| std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(mode));
        mode(0o555).expect("read-only");
        // As root, which writes anywhere, there is nothing to refuse.
        if writable(&locked).is_err() {
            let platform = folder.join("platform");
            let chosen = choose(
                given(FLAGS[0], &locked),
                None,
                Some(platform.clone()),
                None,
                unasked,
            );
            let Err(error @ Error::Write { .. }) = chosen else {
                panic!("refused: {chosen:?}");
            };
            let named = format!(
                "cannot write to the user data folder {} given by --user-data-dir: ",
                locked.display()
            );
            assert!(error.to_string().starts_with(&named), "{error}");
            assert!(!platform.exists(), "fell back to the platform's");
        }
        mode(0o755).expect("writable again, to be removed");
    }

    /// A program that only reads, the CLI, creates nothing and probes nothing: a folder
    /// given that is missing is refused and stays missing, and the platform's, missing, is
    /// chosen all the same, its settings the defaults.
    #[test]
    fn a_program_that_reads_creates_no_folder() {
        let folder = Folder::new("to-read");
        let missing = folder.join("typo/deep/udd");
        let platform = folder.join("platform/apertrust");
        for (given, variable, chosen) in [
            (given(FLAGS[2], &missing), None, "--configdir"),
            (
                None,
                Some(missing.clone().into()),
                "APERTRUST_USER_DATA_DIR",
            ),
        ] {
            let refused = read(given, variable, Some(platform.clone()), None, unasked);
            let Err(error @ Error::Missing { .. }) = refused else {
                panic!("refused: {refused:?}");
            };
            let named = format!(
                "cannot find the user data folder {} given by {chosen}: no such folder",
                missing.display()
            );
            assert_eq!(error.to_string(), named);
        }
        assert!(!folder.join("typo").exists() && !platform.exists());
        // A file where the folder should be is no folder either.
        let file = folder.join("a-file");
        std::fs::write(&file, "").expect("written");
        let refused = read(given(FLAGS[0], &file), None, None, None, unasked);
        assert!(matches!(refused, Err(Error::Missing { .. })), "{refused:?}");

        let chosen = found(read(None, None, Some(platform.clone()), None, unasked));
        assert_eq!(chosen.user_data(), platform);
        assert_eq!(chosen.chosen(), Chosen::Platform);
        assert!(!platform.exists() && !folder.join("platform").exists());

        // One that is there, by its `~` or relative to the current folder, as for writing.
        let home = folder.join("home");
        std::fs::create_dir_all(home.join("dev")).expect("a folder");
        let chosen = found(read(
            given(FLAGS[0], "~/dev"),
            None,
            None,
            Some(home.clone()),
            unasked,
        ));
        assert_eq!(chosen.user_data(), home.join("dev"));
        let here = || Ok(home.clone());
        let chosen = found(read(None, Some("dev".into()), None, None, here));
        assert_eq!(chosen.user_data(), home.join("dev"));
        assert_eq!(
            std::fs::read_dir(home.join("dev")).expect("there").count(),
            0
        );
    }

    /// A read-only folder, which a program that writes refuses, is read; one that cannot
    /// be read is refused, naming it.
    #[cfg(unix)]
    #[test]
    fn a_read_only_folder_is_read_and_an_unreadable_one_refused() {
        use std::os::unix::fs::PermissionsExt;
        let folder = Folder::new("read-only");
        let locked = folder.join("locked");
        std::fs::create_dir_all(&locked).expect("created");
        std::fs::write(
            locked.join("settings.toml"),
            "version = 1
",
        )
        .expect("written");
        let mode = |mode| std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(mode));
        mode(0o555).expect("read-only");
        let chosen = found(read(given(FLAGS[0], &locked), None, None, None, unasked));
        assert_eq!(chosen.user_data(), locked);
        assert_eq!(chosen.settings(), locked.join("settings.toml"));
        // As root, which reads anywhere, there is nothing to refuse.
        mode(0o000).expect("unreadable");
        if std::fs::read_dir(&locked).is_err() {
            let refused = read(given(FLAGS[0], &locked), None, None, None, unasked);
            let Err(error @ Error::Read { .. }) = refused else {
                panic!("refused: {refused:?}");
            };
            let named = format!(
                "cannot read the user data folder {} given by --user-data-dir: ",
                locked.display()
            );
            assert!(error.to_string().starts_with(&named), "{error}");
        }
        mode(0o755).expect("writable again, to be removed");
        let names: Vec<_> = std::fs::read_dir(&locked)
            .expect("there")
            .flatten()
            .collect();
        assert_eq!(names.len(), 1, "only settings.toml: nothing was probed");
    }
}
