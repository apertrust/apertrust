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

//! Restart now (decision 82; `docs/gui.md`, The Settings dialog): the `restart` action quits
//! as Quit does, through the app's own quit path (edits and settings saved, the dialog
//! open closed as its Cancel, the quit alert asked where a job runs), and once the app has
//! ended, its window gone and its session dropped, starts it again:
//!
//! - **from the same app** ([`Launcher`]), found before the app quits, so a restart that
//!   could not start it never quits: its executable on Windows and Linux (on Linux
//!   `/proc/self/exe`, as the adapter helper is run, or the AppImage it runs from, whose
//!   mount goes as the app ends); on macOS, started from the Finder or the Dock, its app
//!   bundle through `open -n -a`, else, from a terminal or outside a bundle, the executable;
//! - **with the same flags** as this run, `-d`, the user data and cache folders, made
//!   absolute, those an environment variable chose among them, and `--conf`, and
//!   `--safe-mode` where this run had it, unless the graphics card or API was changed in
//!   it or Try again asked for the settings ([`Restart::ended`]). Where the executable is
//!   started itself the environment and the terminal are this process's; `open` passes
//!   neither, so through it `WGPU_BACKEND`, `WGPU_POWER_PREF` and the log's terminal are
//!   not carried, the folders being flags by then;
//! - **with what is open**: photos alone by their paths, the active one first, a folder in
//!   folder mode by the hidden [`REOPEN_FLAG`], which reopens what was open last, kept as
//!   the app ended with the folder and the photo active in it, whatever
//!   `library.reopen_last` says; for nothing, or for photos or a folder no longer there,
//!   the hidden [`NO_REOPEN_FLAG`], so the start window comes back.
//!
//! The new process is started as `main`'s last step, after the event loop has returned, so
//! it never races this one for the settings file, the recents or a photo's edit.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::os::OperatingSystem;

/// `--safe-mode`: the app starts with the graphics card and API at Automatic for this run,
/// the settings kept in the file, as darktable's `--disable-opencl` does for OpenCL
/// (decision 82).
pub const SAFE_MODE_FLAG: &str = "--safe-mode";

/// The hidden `--reopen-last`, which a restart passes in place of the folder open: what was
/// open last reopens, whatever `library.reopen_last` says.
pub const REOPEN_FLAG: &str = "--reopen-last";

/// The hidden `--no-reopen-last`, which a restart passes where nothing was open: the start
/// window comes back, whatever `library.reopen_last` says.
pub const NO_REOPEN_FLAG: &str = "--no-reopen-last";

/// How long a restart through the Mac's `open` waits for its answer, to log a failure.
const OPEN_WAITS: Duration = Duration::from_secs(5);

/// The longest command line a restart passes every photo alone on, in characters, each
/// argument counted with its quotes and the space before it: safely under Windows' 32,767
/// for `CreateProcess`, past which the new start would not start. Longer, it passes the
/// active photo alone.
const MOST_CHARACTERS: usize = 30_000;

/// Where the new start goes once the app has ended, for `main` to start it.
pub type Slot = Arc<Mutex<Option<Relaunch>>>;

/// The new start: the app, and its arguments.
#[derive(Clone, Debug, PartialEq)]
pub struct Relaunch {
    pub launcher: Launcher,
    pub arguments: Vec<OsString>,
}

/// What is open as the app ends for a restart, for the new start to open again.
#[derive(Clone, Debug, PartialEq)]
pub enum Open {
    Nothing,
    /// Photos alone, by their paths, the one to be active first.
    Photos(Vec<PathBuf>),
    /// A folder in folder mode, by its root: reopened as what was open last where that is
    /// it (`last`), else opened at its root.
    Folder {
        root: PathBuf,
        last: bool,
    },
}

impl Open {
    /// What is open: Browse's `root`, where a folder is, `last` the root of what was open
    /// last; else the photos open `alone`, in the order opened, `active` the one active
    /// among them, which goes first, so it is active again.
    pub fn of(
        root: Option<&Path>,
        alone: &[PathBuf],
        active: Option<&Path>,
        last: Option<&Path>,
    ) -> Open {
        if let Some(root) = root {
            return Open::Folder {
                root: root.to_owned(),
                last: last == Some(root),
            };
        }
        if alone.is_empty() {
            return Open::Nothing;
        }
        let first = active.filter(|active| alone.iter().any(|path| path == active));
        let first = first.map(Path::to_owned);
        let rest = alone.iter().filter(|path| Some(*path) != first.as_ref());
        Open::Photos(first.iter().chain(rest).cloned().collect())
    }

    /// It, where `there` finds its folder, or the photos of it `there` finds; else
    /// nothing, logged.
    fn there(self, there: impl Fn(&Path) -> bool) -> Open {
        let gone = |path: &Path| {
            let gone = path.display();
            tracing::warn!(target: "control", "{gone} is gone: the restart leaves it");
        };
        match self {
            Open::Nothing => Open::Nothing,
            Open::Folder { ref root, .. } if there(root) => self,
            Open::Folder { root, .. } => {
                gone(&root);
                Open::Nothing
            }
            Open::Photos(paths) => {
                let (kept, left): (Vec<PathBuf>, Vec<PathBuf>) =
                    paths.into_iter().partition(|path| there(path));
                left.iter().for_each(|path| gone(path));
                if kept.is_empty() {
                    Open::Nothing
                } else {
                    Open::Photos(kept)
                }
            }
        }
    }

    /// Its arguments on the new start's command line.
    fn arguments(&self) -> Vec<OsString> {
        match self {
            Open::Nothing => vec![NO_REOPEN_FLAG.into()],
            Open::Photos(paths) => paths.iter().map(OsString::from).collect(),
            Open::Folder { root, last: false } => vec![root.into()],
            Open::Folder { last: true, .. } => vec![REOPEN_FLAG.into()],
        }
    }
}

/// The characters `argument` takes on a command line: on Windows its UTF-16 units, as
/// `CreateProcess` counts them; elsewhere, where [`MOST_CHARACTERS`] is only a bound kept
/// alike, its characters, a name that is not UTF-8 counted as its lossy text.
fn characters(argument: &OsStr) -> usize {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt as _;
        argument.encode_wide().count()
    }
    #[cfg(not(windows))]
    {
        argument.to_string_lossy().chars().count()
    }
}

/// The new start's command line after the program's name: this run's `flags`, then
/// `--safe-mode` where `safe`, then what is `open`; of photos alone, the active one alone,
/// first, where all of them would take the line past [`MOST_CHARACTERS`].
pub fn arguments(flags: &[OsString], safe: bool, open: &Open) -> Vec<OsString> {
    let mut arguments = flags.to_vec();
    if safe {
        arguments.push(SAFE_MODE_FLAG.into());
    }
    let opened = open.arguments();
    let length = |arguments: &[OsString]| -> usize {
        arguments
            .iter()
            .map(|argument| characters(argument) + 3)
            .sum()
    };
    if length(&arguments) + length(&opened) > MOST_CHARACTERS
        && let Open::Photos(paths) = open
    {
        let count = paths.len();
        tracing::warn!(
            target: "control",
            "restart: {count} photos too long a command line, the active one alone"
        );
        arguments.extend(opened.into_iter().take(1));
    } else {
        arguments.extend(opened);
    }
    arguments
}

/// The app's restart: whether it was asked, the app it starts, and this run's flags to
/// start it with.
#[derive(Debug)]
pub struct Restart {
    /// This run's flags, its paths left out, as `main` read them.
    flags: Vec<OsString>,
    /// Whether this run started with `--safe-mode`.
    safe_mode: bool,
    /// Asked, with the app found to start, keeping `--safe-mode` where this run had it
    /// (Restart now) or not (Try again).
    asked: Option<(Launcher, bool)>,
    /// How the app to start is found.
    find: fn() -> std::io::Result<Launcher>,
    slot: Slot,
}

impl Restart {
    /// The restart of a run with `flags` and `safe_mode`, whose new start goes to `slot`.
    pub fn new(flags: Vec<OsString>, safe_mode: bool, slot: Slot) -> Restart {
        Restart {
            flags,
            safe_mode,
            asked: None,
            find: Launcher::here,
            slot,
        }
    }

    /// The same, finding the app to start with `find`, as a test's.
    #[cfg(test)]
    pub fn finding(self, find: fn() -> std::io::Result<Launcher>) -> Restart {
        Restart { find, ..self }
    }

    /// Asked for: by Restart now, `--safe-mode` kept where the graphics settings are as the
    /// run started; by Try again, which starts with the settings, never kept, whatever is
    /// asked after it. The app to start is found now, so a restart that could not start it
    /// is refused, the error returned, before anything quits.
    pub fn ask(&mut self, keep_safe_mode: bool) -> std::io::Result<()> {
        let launcher = (self.find)()?;
        let kept = self.asked.as_ref().is_none_or(|(_, kept)| *kept);
        self.asked = Some((launcher, kept && keep_safe_mode));
        Ok(())
    }

    /// The quit it asked for was answered no: the app stays, and a later Quit only quits.
    pub fn cancel(&mut self) {
        self.asked = None;
    }

    #[cfg(test)]
    pub fn asked(&self) -> bool {
        self.asked.is_some()
    }

    /// The app ends, with `open` open and the graphics card or API `changed` since the
    /// start or not: where a restart was asked, the new start goes to the slot, opening
    /// what is open where it is still there.
    pub fn ended(&mut self, open: Open, changed: bool) {
        let Some((launcher, keep)) = self.asked.take() else {
            return;
        };
        let safe = self.safe_mode && keep && !changed;
        let open = open.there(Path::exists);
        let arguments = arguments(&self.flags, safe, &open);
        let relaunch = Relaunch {
            launcher,
            arguments,
        };
        *self.slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(relaunch);
    }
}

/// The program, and the arguments before the app's own, that start the app again.
#[derive(Clone, Debug, PartialEq)]
pub struct Launcher {
    pub program: PathBuf,
    pub first: Vec<OsString>,
    /// Whether it answers at once, as `open` does, its exit status saying whether the app
    /// started.
    pub answers: bool,
}

/// What [`Launcher::of`] finds the app by besides its executable.
#[derive(Clone, Debug, Default)]
pub struct Found {
    /// `APPIMAGE`, the AppImage the app runs from, and `APPDIR`, where it is mounted.
    pub appimage: Option<OsString>,
    pub appdir: Option<OsString>,
    /// The executable's path with its links followed: on Linux `/proc/self/exe`'s target,
    /// which may lie in `APPDIR`.
    pub resolved: Option<PathBuf>,
    /// Whether the app runs from a terminal, its standard output one.
    pub terminal: bool,
}

impl Launcher {
    /// The app as this process runs it: its executable, the environment and whether a
    /// terminal started it ([`Launcher::of`]); an error where the executable cannot be
    /// found or the program is not there.
    pub fn here() -> std::io::Result<Launcher> {
        use std::io::IsTerminal as _;
        let exe = crate::hardware::own_executable()?;
        let found = Found {
            appimage: std::env::var_os("APPIMAGE"),
            appdir: std::env::var_os("APPDIR"),
            resolved: std::fs::canonicalize(&exe).ok(),
            terminal: std::io::stdout().is_terminal(),
        };
        let launcher = Launcher::of(&exe, OperatingSystem::from_target_os(), &found);
        if !launcher.program.exists() {
            let missing = launcher.program.display();
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{missing} is not there"),
            ));
        }
        Ok(launcher)
    }

    /// The app as this process runs from `exe` on `os`, with what was `found`: on the Mac,
    /// where `exe` is in an app bundle and no terminal started it, `open -n -a` with the
    /// bundle, a new instance though this one may not have gone yet; on Linux, where `exe`,
    /// or the file it links to, lies in the AppImage's mount, `APPDIR`, the AppImage
    /// `APPIMAGE` names, since the mount goes as the app ends; else `exe` itself.
    pub fn of(exe: &Path, os: OperatingSystem, found: &Found) -> Launcher {
        let alone = Launcher {
            program: exe.to_owned(),
            first: Vec::new(),
            answers: false,
        };
        match os {
            OperatingSystem::Mac if !found.terminal => match bundle(exe) {
                Some(bundle) => Launcher {
                    program: PathBuf::from("/usr/bin/open"),
                    first: vec!["-n".into(), "-a".into(), bundle.into(), "--args".into()],
                    answers: true,
                },
                None => alone,
            },
            OperatingSystem::Nix => {
                let mounted = found
                    .appdir
                    .as_deref()
                    .filter(|appdir| !appdir.is_empty())
                    .is_some_and(|appdir| {
                        let lies = found.resolved.as_deref().unwrap_or(exe);
                        lies.starts_with(appdir)
                    });
                match found.appimage.as_deref() {
                    Some(appimage) if mounted && !appimage.is_empty() => Launcher {
                        program: PathBuf::from(appimage),
                        ..alone
                    },
                    _ => alone,
                }
            }
            _ => alone,
        }
    }
}

/// The app bundle `exe` runs in: `<name>.app` where `exe` is in its `Contents/MacOS`.
fn bundle(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let bundle = contents.parent()?;
    let named = |path: &Path, name: &str| path.file_name() == Some(OsStr::new(name));
    let app = bundle.extension() == Some(OsStr::new("app"));
    (named(macos, "MacOS") && named(contents, "Contents") && app).then(|| bundle.to_owned())
}

/// The command that starts `relaunch`; its standard output and error this process's, so
/// `-d`'s lines go on where they went where the executable is started itself.
pub fn command(relaunch: &Relaunch) -> Command {
    let mut command = Command::new(&relaunch.launcher.program);
    command
        .args(&relaunch.launcher.first)
        .args(&relaunch.arguments)
        .stdin(Stdio::null());
    command
}

/// Starts `relaunch`, not waiting for the app: `main`'s last step, once this one has saved
/// everything. Through the Mac's `open`, which answers at once, its answer is waited for a
/// moment, to log a failure. A failure is logged; the app has ended either way.
pub fn start(relaunch: Relaunch) {
    let mut command = command(&relaunch);
    tracing::debug!(target: "control", "restarting: {command:?}");
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            tracing::error!(target: "control", "not restarted: {error}");
            return;
        }
    };
    if !relaunch.launcher.answers {
        return;
    }
    let deadline = Instant::now() + OPEN_WAITS;
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return,
            Ok(Some(status)) => {
                tracing::error!(target: "control", "not restarted: open ended: {status}");
                return;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(error) => {
                tracing::error!(target: "control", "not restarted: {error}");
                return;
            }
        }
    }
    tracing::warn!(target: "control", "restart: open has not answered in {} s", OPEN_WAITS.as_secs());
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn os(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    /// The app this test starts in place of its own.
    pub(crate) fn launcher() -> std::io::Result<Launcher> {
        Ok(Launcher {
            program: PathBuf::from("apertrust"),
            first: Vec::new(),
            answers: false,
        })
    }

    fn unfound() -> std::io::Result<Launcher> {
        Err(std::io::ErrorKind::NotFound.into())
    }

    /// What is open becomes what the new start opens: photos alone by their paths, the
    /// active one first, a folder by `--reopen-last` where it is what was open last, else
    /// by its root; nothing, or what is no longer there, by `--no-reopen-last`, the start
    /// window.
    #[test]
    fn the_new_start_opens_what_is_open() {
        let (root, photo) = (Path::new("/photos/Trips"), Path::new("/a/IMG_1.CR3"));
        let flags = os(&["-d", "settings", "--conf", "scale=2"]);
        let open = Open::of(Some(root), &[], None, Some(root));
        assert_eq!(
            arguments(&flags, false, &open),
            os(&["-d", "settings", "--conf", "scale=2", "--reopen-last"])
        );
        let open = Open::of(Some(root), &[], None, Some(Path::new("/elsewhere")));
        assert_eq!(arguments(&[], false, &open), os(&["/photos/Trips"]));
        let alone = [photo.to_owned()];
        let open = Open::of(None, &alone, Some(photo), Some(root));
        assert_eq!(open, Open::Photos(alone.to_vec()));
        assert_eq!(
            arguments(&flags[..2], true, &open),
            os(&["-d", "settings", "--safe-mode", "/a/IMG_1.CR3"])
        );
        assert_eq!(open.clone().there(|_| true), open);
        assert_eq!(open.there(|_| false), Open::Nothing);
        // Several photos alone: every one, the active one first; those gone left out.
        let several = [
            PathBuf::from("/a/IMG_1.CR3"),
            PathBuf::from("/b/DSC_2.NEF"),
            PathBuf::from("/a/IMG_3.CR3"),
        ];
        let active = Some(Path::new("/b/DSC_2.NEF"));
        let open = Open::of(None, &several, active, None);
        assert_eq!(
            arguments(&[], false, &open),
            os(&["/b/DSC_2.NEF", "/a/IMG_1.CR3", "/a/IMG_3.CR3"])
        );
        let kept = open.there(|path| path != Path::new("/b/DSC_2.NEF"));
        assert_eq!(
            arguments(&[], false, &kept),
            os(&["/a/IMG_1.CR3", "/a/IMG_3.CR3"])
        );
        let open = Open::of(None, &several, Some(Path::new("/c/x.ARW")), None);
        assert_eq!(open, Open::Photos(several.to_vec()));
        // So many that the line would pass Windows' limit: the active one alone.
        let many: Vec<PathBuf> = (0..800)
            .map(|i| PathBuf::from(format!("/photos/Wedding 2026/Selects/IMG_{i:04}.CR3")))
            .collect();
        let active = Some(many[7].as_path());
        let open = Open::of(None, &many, active, None);
        let line = arguments(&flags, false, &open);
        let mut wanted = flags.clone();
        wanted.push(many[7].clone().into());
        assert_eq!(line, wanted);
        let fits = Open::of(None, &many[..600], active, None);
        assert_eq!(arguments(&flags, false, &fits).len(), flags.len() + 600);
        // Names not in ASCII are counted in characters, as Windows counts its UTF-16 units,
        // never in bytes: 650 of these fit, though their bytes would not.
        let named: Vec<PathBuf> = (0..650)
            .map(|i| PathBuf::from(format!("/фото/Весілля 2026/Вибране/IMG_{i:04}.CR3")))
            .collect();
        let bytes: usize = named.iter().map(|path| path.as_os_str().len() + 3).sum();
        assert!(bytes > MOST_CHARACTERS);
        let fits = Open::of(None, &named, Some(named[0].as_path()), None);
        assert_eq!(arguments(&flags, false, &fits).len(), flags.len() + 650);
        let folder = Open::of(Some(root), &[], None, Some(root));
        assert_eq!(folder.there(|_| false), Open::Nothing);
        assert_eq!(Open::of(None, &[], None, Some(root)), Open::Nothing);
        assert_eq!(
            arguments(&[], false, &Open::Nothing),
            os(&["--no-reopen-last"])
        );
    }

    /// Nothing goes to the slot until a restart is asked and the app ends; a no to the quit
    /// alert takes it back; an app that cannot be found refuses the restart. `--safe-mode`
    /// stays only where the run had it, Restart now asked and the graphics settings are as
    /// they started, and Try again's dropping it stays.
    #[test]
    fn a_restart_asked_gives_its_arguments_as_the_app_ends() {
        let slot = Slot::default();
        let taken = |slot: &Slot| slot.lock().expect("slot").take().map(|r| r.arguments);
        let mut restart = Restart::new(os(&["-d", "gpu"]), true, slot.clone()).finding(launcher);
        restart.ended(Open::Nothing, false);
        assert_eq!(taken(&slot), None);
        restart.ask(true).expect("found");
        assert!(restart.asked());
        restart.cancel();
        restart.ended(Open::Nothing, false);
        assert_eq!(taken(&slot), None);
        let nothing = "--no-reopen-last";
        for (keeps, changed, expected) in [
            (
                &[true][..],
                false,
                os(&["-d", "gpu", "--safe-mode", nothing]),
            ),
            (&[true], true, os(&["-d", "gpu", nothing])),
            (&[false], false, os(&["-d", "gpu", nothing])),
            (&[false, true], false, os(&["-d", "gpu", nothing])),
        ] {
            for keep in keeps {
                restart.ask(*keep).expect("found");
            }
            restart.ended(Open::Nothing, changed);
            assert_eq!(taken(&slot), Some(expected), "{keeps:?} {changed}");
            assert!(!restart.asked());
        }
        // A run without it never gains it.
        let mut restart = Restart::new(Vec::new(), false, slot.clone()).finding(launcher);
        restart.ask(true).expect("found");
        restart.ended(Open::Nothing, false);
        assert_eq!(taken(&slot), Some(os(&[nothing])));
        // The app not found: refused, nothing asked.
        let mut restart = Restart::new(Vec::new(), false, slot.clone()).finding(unfound);
        assert!(restart.ask(true).is_err());
        assert!(!restart.asked());
    }

    /// The same app: on the Mac its bundle through `open -n -a`, but from a terminal, a
    /// bundle's look-alike or outside one, the executable; on Linux the AppImage only where
    /// the executable lies in its mount.
    #[test]
    fn the_launcher_is_the_bundle_the_appimage_or_the_executable() {
        let mac = OperatingSystem::Mac;
        let finder = Found::default();
        let exe = Path::new("/Applications/Apertrust Nightly.app/Contents/MacOS/apertrust-nightly");
        let launcher = Launcher::of(exe, mac, &finder);
        assert_eq!(launcher.program, Path::new("/usr/bin/open"));
        let bundle = "/Applications/Apertrust Nightly.app";
        assert_eq!(launcher.first, os(&["-n", "-a", bundle, "--args"]));
        assert!(launcher.answers);
        let terminal = Found {
            terminal: true,
            ..Found::default()
        };
        let direct = Launcher::of(exe, mac, &terminal);
        assert_eq!((direct.program.as_path(), direct.answers), (exe, false));
        let alone = Path::new("/Users/me/apertrust/target/release/apertrust");
        assert_eq!(Launcher::of(alone, mac, &finder).program, alone);
        let near = Path::new("/x/Apertrust.app/Contents/Resources/apertrust");
        assert_eq!(Launcher::of(near, mac, &finder).program, near);
        let appimage = Found {
            appimage: Some("/home/me/apertrust_0.2.0_x86_64.AppImage".into()),
            appdir: Some("/tmp/.mount_aperXYZ".into()),
            resolved: None,
            terminal: false,
        };
        let mounted = Path::new("/tmp/.mount_aperXYZ/usr/bin/apertrust");
        let launcher = Launcher::of(mounted, OperatingSystem::Nix, &appimage);
        assert_eq!(
            launcher.program.as_os_str(),
            appimage.appimage.as_deref().expect("set")
        );
        assert!(launcher.first.is_empty());
        // `/proc/self/exe`, as Linux runs it, by the file it links to in the mount.
        let proc = Path::new("/proc/self/exe");
        let linked = Found {
            resolved: Some(mounted.to_owned()),
            ..appimage.clone()
        };
        let launcher = Launcher::of(proc, OperatingSystem::Nix, &linked);
        assert_eq!(
            launcher.program.as_os_str(),
            appimage.appimage.as_deref().expect("set")
        );
        let outside = Found {
            resolved: Some("/usr/bin/apertrust".into()),
            ..appimage.clone()
        };
        let launcher = Launcher::of(proc, OperatingSystem::Nix, &outside);
        assert_eq!(launcher.program, proc);
        // An executable outside the mount, as one an AppImage's environment started.
        let other = Path::new("/usr/bin/apertrust");
        assert_eq!(
            Launcher::of(other, OperatingSystem::Nix, &appimage).program,
            other
        );
        let no_appdir = Found {
            appdir: None,
            ..appimage.clone()
        };
        assert_eq!(
            Launcher::of(mounted, OperatingSystem::Nix, &no_appdir).program,
            mounted
        );
        let windows = Path::new(r"C:\Program Files\Apertrust\apertrust.exe");
        let launcher = Launcher::of(windows, OperatingSystem::Windows, &appimage);
        assert_eq!(launcher.program, windows);
        // The command: the launcher's arguments first, then the app's.
        let relaunch = Relaunch {
            launcher: Launcher::of(exe, mac, &finder),
            arguments: os(&["-d", "gpu"]),
        };
        let command = command(&relaunch);
        let given: Vec<&OsStr> = command.get_args().collect();
        assert_eq!(given, ["-n", "-a", bundle, "--args", "-d", "gpu"]);
    }
}
