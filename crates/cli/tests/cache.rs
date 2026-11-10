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

//! `apertrust-cli cache info` and `cache clear` (decision 76), on a cache folder and a
//! user data folder of each test's own, so none reads or clears a person's.

use std::path::PathBuf;
use std::process::{Command, Output};

use apertrust_base::cache::{Cache, LIBRARIES, PIPELINES, THUMBNAILS, Usage};

/// The bytes of each entry [`Folder::populated`] writes: its mark, the version's length, a
/// one-byte version and a thousand bytes.
const ENTRY: u64 = 4 + 4 + 1 + 1000;

/// A folder of a test's own, removed when the test ends, holding the user data folder,
/// `user`, and the cache, `cache`.
struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Folder {
        let path =
            std::env::temp_dir().join(format!("apertrust-cli-cache-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(path.join("user")).expect("a temporary folder");
        Folder(path)
    }

    /// A folder whose cache holds three thumbnails, a pipeline and a library's cache.
    fn populated(name: &str) -> Folder {
        let folder = Folder::new(name);
        let cache = Cache::open(folder.cache());
        for slot in 0u8..3 {
            cache.write(THUMBNAILS, &[slot], b"v", &[0; 1000]).unwrap();
        }
        cache
            .write(PIPELINES, b"adapter", b"v", &[0; 1000])
            .unwrap();
        let library = cache.kind_folder(LIBRARIES).join("0f3a");
        std::fs::create_dir_all(&library).unwrap();
        std::fs::write(library.join("index"), vec![0; 5000]).unwrap();
        folder
    }

    fn cache(&self) -> PathBuf {
        self.0.join("cache")
    }

    /// Each kind's usage as a walk finds it now.
    fn usage(&self) -> [Usage; 3] {
        let sizes = Cache::open(self.cache()).walk(&|| false).unwrap();
        [THUMBNAILS, PIPELINES, LIBRARIES].map(|kind| sizes.of(kind))
    }

    /// The CLI's `cache` with `args`, on this folder: its stdout, its stderr and its exit
    /// status.
    fn run(&self, args: &[&str]) -> (String, String, Option<i32>) {
        let user = self.0.join("user");
        let cache = self.cache();
        let Output {
            status,
            stdout,
            stderr,
        } = Command::new(env!("CARGO_BIN_EXE_apertrust-cli"))
            .arg("--user-data-dir")
            .arg(&user)
            .arg("--cache-dir")
            .arg(&cache)
            .arg("cache")
            .args(args)
            .env_remove("RUST_LOG")
            .env_remove("APERTRUST_USER_DATA_DIR")
            .env_remove("APERTRUST_CACHE_DIR")
            .output()
            .expect("the CLI runs");
        let text = |bytes: Vec<u8>| String::from_utf8_lossy(&bytes).into_owned();
        (text(stdout), text(stderr), status.code())
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn usage(entries: u64, bytes: u64) -> Usage {
    Usage { entries, bytes }
}

/// `info` prints the folder and how it was chosen, the limit as the user data folder's
/// settings give it, the limited kinds' total, each kind's entries and size and
/// `cache.unused_after`, one to a line, and writes nothing.
#[test]
fn info_prints_the_folder_the_limit_and_each_kind() {
    let folder = Folder::populated("info");
    let settings = "version = 1\n[cache]\nlimit = 12.5\nunused_after = \"days_30\"\n";
    std::fs::write(folder.0.join("user/settings.toml"), settings).unwrap();
    let before = folder.usage();

    let (stdout, stderr, status) = folder.run(&["info"]);
    assert_eq!(status, Some(0), "{stderr}");
    let expected = format!(
        "folder: {}\nchosen by: --cache-dir\nlimit: 12.5 GB\nlimited kinds: 4 entries, 4.0 kB\n\
         thumbnails: 3 entries, 3.0 kB\npipelines: 1 entry, 1.0 kB\n\
         libraries: 1 entry, 5.0 kB\ncopies: 0 entries, 0.0 kB\n\
         cache.unused_after: days_30\n",
        folder.cache().display()
    );
    assert_eq!(stdout, expected);
    assert_eq!(folder.usage(), before, "info writes and removes nothing");

    // Unlimited while `cache.limited` is off, for this run alone.
    let (stdout, _, status) = folder.run(&["--conf", "cache.limited=false", "info"]);
    assert_eq!(status, Some(0));
    assert!(stdout.contains("\nlimit: unlimited\n"), "{stdout}");
}

/// A cache folder that is missing is an empty cache, and stays missing.
#[test]
fn info_on_a_missing_folder_is_an_empty_cache_and_creates_nothing() {
    let folder = Folder::new("info-missing");
    let (stdout, stderr, status) = folder.run(&["info"]);
    assert_eq!(status, Some(0), "{stderr}");
    let first = format!("folder: {} (not created yet)\n", folder.cache().display());
    assert!(stdout.starts_with(&first), "{stdout}");
    assert!(stdout.contains("\nlimit: 4 GB\n"), "{stdout}");
    assert!(
        stdout.contains("\nlimited kinds: 0 entries, 0.0 kB\n"),
        "{stdout}"
    );
    assert!(
        stdout.ends_with("\ncache.unused_after: days_90\n"),
        "{stdout}"
    );
    assert!(!folder.cache().exists());

    let (stdout, _, status) = folder.run(&["clear"]);
    assert_eq!(status, Some(0));
    assert_eq!(
        stdout,
        "thumbnails: removed 0 entries, 0.0 kB\npipelines: removed 0 entries, 0.0 kB\n"
    );
    assert!(!folder.cache().exists());
}

/// `clear KIND` removes that kind's entries alone; `clear` every limited kind's, never a
/// kept kind's.
#[test]
fn clear_removes_the_kinds_named_or_every_limited_one() {
    let folder = Folder::populated("clear");
    let (stdout, stderr, status) = folder.run(&["clear", "thumbnails"]);
    assert_eq!(status, Some(0), "{stderr}");
    assert_eq!(stdout, "thumbnails: removed 3 entries, 3.0 kB\n");
    let library = usage(1, 5000);
    assert_eq!(folder.usage(), [usage(0, 0), usage(1, ENTRY), library]);

    let folder = Folder::populated("clear-all");
    let (stdout, stderr, status) = folder.run(&["clear"]);
    assert_eq!(status, Some(0), "{stderr}");
    assert_eq!(
        stdout,
        "thumbnails: removed 3 entries, 3.0 kB\npipelines: removed 1 entry, 1.0 kB\n"
    );
    assert_eq!(folder.usage(), [usage(0, 0), usage(0, 0), library]);
}

/// A kept kind, or one that does not exist, is refused by name as a usage error before
/// anything is removed.
#[test]
fn clear_refuses_a_kept_kind_by_name_and_removes_nothing() {
    let folder = Folder::populated("clear-kept");
    let before = folder.usage();
    for (kind, said) in [
        ("libraries", "the cache's libraries are kept"),
        ("copies", "the cache's copies are kept"),
        ("previews", "no such kind"),
    ] {
        let (_, stderr, status) = folder.run(&["clear", "thumbnails", kind]);
        assert_eq!(status, Some(2), "{stderr}");
        assert!(stderr.contains(said), "{stderr}");
        assert!(stderr.contains("thumbnails, pipelines"), "{stderr}");
    }
    assert_eq!(folder.usage(), before);
}

/// An entry that cannot be removed is passed over, said so, and the exit status is 1;
/// the rest are removed.
#[cfg(unix)]
#[test]
fn clear_says_what_it_could_not_remove_and_fails() {
    use std::os::unix::fs::PermissionsExt;
    let folder = Folder::populated("clear-failed");
    // The pipeline's subfolder read-only, so its one entry stays.
    let pipelines = folder.cache().join("pipelines");
    let subfolder = std::fs::read_dir(&pipelines)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let mode = |mode| std::fs::set_permissions(&subfolder, std::fs::Permissions::from_mode(mode));
    mode(0o555).unwrap();
    // Where the process can write in it anyway, as root can, there is nothing to test.
    let probe = subfolder.join("probe");
    if std::fs::write(&probe, b"").is_ok() {
        let _ = std::fs::remove_file(&probe);
        mode(0o755).unwrap();
        return;
    }
    let (stdout, stderr, status) = folder.run(&["clear"]);
    mode(0o755).unwrap();
    assert_eq!(status, Some(1), "{stderr}");
    assert_eq!(
        stdout,
        "thumbnails: removed 3 entries, 3.0 kB\npipelines: removed 0 entries, 0.0 kB\n\
         pipelines: could not remove 1 entry\n"
    );
    assert!(stderr.contains("not fully cleared"), "{stderr}");
    assert_eq!(folder.usage()[..2], [usage(0, 0), usage(1, ENTRY)]);
}
