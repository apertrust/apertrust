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

//! `apertrust-cli import` (decision 85) as a script runs it, on a card's folder, a home
//! folder and a user data folder of each test's own: what it copies, what it prints and
//! its exit status. Its raws are no raws a decoder reads, so their capture times are their
//! files' modified times.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A folder of a test's own, removed when the test ends: `card/DCIM/100CANON` with three
/// raws, `home/Pictures`, and `user`, the user data folder.
struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Folder {
        let path = std::env::temp_dir().join(format!(
            "apertrust-cli-import-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        let folder = Folder(path);
        std::fs::create_dir_all(folder.card()).unwrap();
        std::fs::create_dir_all(folder.pictures()).unwrap();
        std::fs::create_dir_all(folder.0.join("user")).unwrap();
        // Linux's Pictures folder is the one the user's folders file names.
        let config = folder.0.join("home/.config");
        std::fs::create_dir_all(&config).unwrap();
        std::fs::write(
            config.join("user-dirs.dirs"),
            "XDG_PICTURES_DIR=\"$HOME/Pictures\"\n",
        )
        .unwrap();
        for i in 1..=3 {
            let bytes = format!("raw number {i}").repeat(i);
            std::fs::write(folder.card().join(format!("IMG_000{i}.CR3")), bytes).unwrap();
        }
        folder
    }

    fn card(&self) -> PathBuf {
        self.0.join("card/DCIM/100CANON")
    }

    fn pictures(&self) -> PathBuf {
        self.0.join("home/Pictures")
    }

    fn apertrust(&self) -> PathBuf {
        self.pictures().join("Apertrust")
    }

    /// `path` as a pattern names it, with forward slashes, which a pattern never takes as
    /// an escape.
    fn pattern(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }

    /// `--conf` for both folders, so the check walks the test's own trees alone.
    fn going_to(&self, folder: &Path) -> Vec<String> {
        let folder = Self::pattern(folder);
        vec![
            "--conf".to_owned(),
            format!("import.default_folder={folder}"),
            "--conf".to_owned(),
            format!("import.base_folder={folder}"),
        ]
    }

    /// The CLI with `args`, its home this folder's: its stdout, its stderr and its exit
    /// status.
    fn run(&self, args: &[&str]) -> (String, String, Option<i32>) {
        let home = self.0.join("home");
        let Output {
            status,
            stdout,
            stderr,
        } = Command::new(env!("CARGO_BIN_EXE_apertrust-cli"))
            .arg("--user-data-dir")
            .arg(self.0.join("user"))
            .args(args)
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", home.join(".config"))
            .env_remove("XDG_PICTURES_DIR")
            .env_remove("RUST_LOG")
            .env_remove("APERTRUST_USER_DATA_DIR")
            .output()
            .expect("the CLI runs");
        let text = |bytes: Vec<u8>| String::from_utf8_lossy(&bytes).into_owned();
        (text(stdout), text(stderr), status.code())
    }

    /// `import` of the card's folder with `args` after it.
    fn import(&self, before: &[String], args: &[&str]) -> (String, String, Option<i32>) {
        let card = self.card();
        let mut all: Vec<&str> = before.iter().map(String::as_str).collect();
        all.extend(["import", card.to_str().unwrap()]);
        all.extend(args);
        self.run(&all)
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Every file under `root`, by its path relative to it, with its bytes.
fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut found = BTreeMap::new();
    let mut queue = vec![root.to_owned()];
    while let Some(folder) = queue.pop() {
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                queue.push(path);
            } else {
                let bytes = std::fs::read(&path).unwrap();
                found.insert(path.strip_prefix(root).unwrap().to_owned(), bytes);
            }
        }
    }
    found
}

/// The defaults: a card's new photos copied into a folder of today's under
/// `$(PICTURES_FOLDER)/Apertrust`, one already there from an earlier import left out and
/// said so, the source unchanged.
#[cfg(unix)]
#[test]
fn a_card_s_new_photos_are_copied_and_those_imported_before_left_out() {
    let folder = Folder::new("defaults");
    let earlier = folder.apertrust().join("20261101_Studio");
    std::fs::create_dir_all(&earlier).unwrap();
    std::fs::copy(
        folder.card().join("IMG_0002.CR3"),
        earlier.join("IMG_0002.CR3"),
    )
    .unwrap();
    let card = files(&folder.card());
    let (stdout, stderr, status) = folder.import(&[], &["--name", "Wedding"]);
    assert_eq!(status, Some(0), "{stderr}");
    assert!(stdout.contains("; 1 imported before left out"), "{stdout}");
    let landed = files(&folder.apertrust());
    let today: Vec<&PathBuf> = (landed.keys())
        .filter(|path| !path.starts_with("20261101_Studio"))
        .collect();
    assert_eq!(today.len(), 2, "{landed:?}");
    let shoot = today[0].parent().unwrap().to_string_lossy().into_owned();
    assert!(shoot.ends_with("_Wedding") && shoot.len() == 16, "{shoot}");
    let names: Vec<_> = today.iter().map(|path| path.file_name().unwrap()).collect();
    assert_eq!(names, ["IMG_0001.CR3", "IMG_0003.CR3"]);
    assert_eq!(files(&folder.card()), card, "the source as it was");
}

/// `--folder`, `--subfolder` and `--name` each change the destination; `--conf
/// import.subfolder=` empties Subfolder; a flag wins over `--conf`.
#[test]
fn the_flags_and_conf_choose_the_destination() {
    let folder = Folder::new("flags");
    let album = folder.0.join("Album");
    let conf = folder.going_to(&folder.apertrust());
    let subfolder = |sub: &str| vec!["--conf".to_owned(), format!("import.subfolder={sub}")];
    let cases: [(Vec<String>, Vec<String>, &str); 4] = [
        (
            conf.clone(),
            vec![
                "--folder".to_owned(),
                Folder::pattern(&album),
                "--subfolder".to_owned(),
                "$(IMPORT_NAME)".to_owned(),
            ],
            "Album/no_name",
        ),
        (
            [conf.clone(), subfolder("$(IMPORT_NAME)")].concat(),
            vec!["--name".to_owned(), "Lisbon".to_owned()],
            "Pictures/Apertrust/Lisbon",
        ),
        (
            [conf.clone(), subfolder("")].concat(),
            vec![],
            "Pictures/Apertrust",
        ),
        (
            [conf.clone(), subfolder("Ignored")].concat(),
            vec!["--subfolder".to_owned(), "Chosen".to_owned()],
            "Pictures/Apertrust/Chosen",
        ),
    ];
    for (before, after, expected) in cases {
        let after: Vec<&str> = after.iter().map(String::as_str).collect();
        let (stdout, stderr, status) =
            folder.import(&before, &[&after[..], &["--dry-run"]].concat());
        assert_eq!(status, Some(0), "{stderr}");
        let line = (stdout.lines())
            .find(|line| line.starts_with("IMG_0001.CR3 → "))
            .unwrap_or_else(|| panic!("{expected}: {stdout}"));
        let line = line.replace('\\', "/");
        let tail = format!("/{expected}/IMG_0001.CR3");
        assert!(line.ends_with(&tail), "{expected}: {stdout}");
    }
}

/// `--dry-run` prints each photo's destination, the folder and the summary, and writes
/// nothing.
#[test]
fn a_dry_run_prints_the_paths_and_writes_nothing() {
    let folder = Folder::new("dry-run");
    let conf = folder.going_to(&folder.apertrust());
    let (stdout, stderr, status) = folder.import(
        &conf,
        &[
            "--subfolder",
            "$(IMPORT_NAME)",
            "--name",
            "Wedding",
            "--dry-run",
        ],
    );
    assert_eq!(status, Some(0), "{stderr}");
    let lines: Vec<&str> = stdout.lines().collect();
    // Under the home folder, shown with ~, where the home folder is the test's own.
    #[cfg(unix)]
    let to = |name: &str| format!("{name} → ~/Pictures/Apertrust/Wedding/{name}");
    #[cfg(unix)]
    assert_eq!(
        lines[..3],
        [to("IMG_0001.CR3"), to("IMG_0002.CR3"), to("IMG_0003.CR3")]
    );
    assert!(lines[3].starts_with("new folder: "), "{stdout}");
    assert!(
        lines[3]
            .replace('\\', "/")
            .ends_with("Apertrust/Wedding, 3 to come")
    );
    assert!(lines[4].starts_with("3 to copy, "), "{stdout}");
    assert!(
        lines[4].ends_with("; 0 imported before left out"),
        "{stdout}"
    );
    assert!(files(&folder.pictures()).is_empty());
    assert!(!folder.apertrust().exists());
}

/// A name taken at the destination gives `_01`.
#[test]
fn a_name_taken_gives_01() {
    let folder = Folder::new("taken");
    std::fs::create_dir_all(folder.apertrust()).unwrap();
    std::fs::write(folder.apertrust().join("IMG_0001.CR3"), b"another photo").unwrap();
    let conf = folder.going_to(&folder.apertrust());
    let (stdout, stderr, status) = folder.import(&conf, &["--subfolder", ""]);
    assert_eq!(status, Some(0), "{stderr}");
    let landed = files(&folder.apertrust());
    assert_eq!(landed[Path::new("IMG_0001.CR3")], b"another photo");
    assert!(
        landed.contains_key(Path::new("IMG_0001_01.CR3")),
        "{landed:?}"
    );
    assert!(landed.contains_key(Path::new("IMG_0001_01.CR3.apxmp")));
    assert!(stdout.contains("Imported 3 photos into"), "{stdout}");
}

/// An unknown variable, and a destination inside the source, refuse the run with status 1,
/// said with the row and the fix, and nothing is copied.
#[test]
fn a_problem_refuses_the_run_and_copies_nothing() {
    let folder = Folder::new("problems");
    let conf = folder.going_to(&folder.apertrust());
    let (_, stderr, status) = folder.import(&conf, &["--subfolder", "$(YEAR)$(MONT)"]);
    assert_eq!(status, Some(1));
    assert!(
        stderr.contains("Subfolder: unknown variable $(MONT). Did you mean $(MONTH)?"),
        "{stderr}"
    );
    let inside = Folder::pattern(&folder.card().join("Copies"));
    let (_, stderr, status) = folder.import(&conf, &["--folder", &inside, "--subfolder", ""]);
    assert_eq!(status, Some(1));
    assert!(stderr.contains("Folder: "), "{stderr}");
    assert!(stderr.contains("lies inside the source"), "{stderr}");
    assert!(!folder.apertrust().exists() && !folder.card().join("Copies").exists());
}

/// A usage error exits with 2: no SOURCE, a flag without its value, a flag unknown.
#[test]
fn a_usage_error_exits_with_2() {
    let folder = Folder::new("usage");
    for args in [
        &["import"][..],
        &["import", "card", "--folder"],
        &["import", "card", "--all"],
    ] {
        let (_, stderr, status) = folder.run(args);
        assert_eq!(status, Some(2), "{args:?}: {stderr}");
    }
    assert!(files(&folder.pictures()).is_empty());
}

/// A photo that cannot be read gives status 1, named on stderr, and the others are copied.
#[cfg(unix)]
#[test]
fn a_photo_that_fails_gives_1_and_the_others_are_copied() {
    use std::os::unix::fs::PermissionsExt;
    let folder = Folder::new("fails");
    let locked = folder.card().join("IMG_0002.CR3");
    let conf = folder.going_to(&folder.apertrust());
    // Its size and name must be read, its bytes not.
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o200)).unwrap();
    let (stdout, stderr, status) = folder.import(&conf, &["--subfolder", ""]);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(status, Some(1), "{stdout}");
    assert!(stderr.contains("IMG_0002.CR3: not imported"), "{stderr}");
    let landed: Vec<PathBuf> = files(&folder.apertrust()).into_keys().collect();
    assert_eq!(
        landed,
        [PathBuf::from("IMG_0001.CR3"), PathBuf::from("IMG_0003.CR3")]
    );
    assert!(stdout.contains("Imported 2 photos"), "{stdout}");
}

/// `--help` describes the command and each flag.
#[test]
fn the_help_describes_the_command_and_its_flags() {
    let folder = Folder::new("help");
    let (stdout, _, status) = folder.run(&["import", "--help"]);
    assert_eq!(status, Some(0));
    for flag in [
        "--folder <DIR>",
        "--subfolder <PATTERN>",
        "--name <NAME>",
        "--dry-run",
    ] {
        assert!(stdout.contains(flag), "{flag}: {stdout}");
    }
    assert!(
        stdout.contains("only those not imported before"),
        "{stdout}"
    );
}
