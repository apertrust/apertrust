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

//! The folder listing, on temporary folders.

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use apertrust_library::{Error, Event, Listing, Options};
use tempfile::TempDir;

/// Writes a file of `size` bytes.
fn file(dir: &Path, name: &str, size: usize) {
    fs::write(dir.join(name), vec![0u8; size]).unwrap();
}

fn list(dir: &Path) -> apertrust_library::Folder {
    Listing::start(dir, Options::default()).collect().unwrap()
}

fn names(photos: &[apertrust_library::Photo]) -> Vec<&str> {
    photos.iter().map(|p| p.name.as_str()).collect()
}

#[test]
fn a_sidecar_is_paired_with_its_raw() {
    let dir = TempDir::new().unwrap();
    file(dir.path(), "DSCF0002.RAF", 300);
    file(dir.path(), "DSCF0001.RAF", 100);
    file(dir.path(), "DSCF0001.RAF.apxmp", 7);
    let modified = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    fs::File::options()
        .write(true)
        .open(dir.path().join("DSCF0001.RAF"))
        .unwrap()
        .set_modified(modified)
        .unwrap();

    let folder = list(dir.path());

    assert_eq!(names(&folder.photos), ["DSCF0001.RAF", "DSCF0002.RAF"]);
    let first = &folder.photos[0];
    assert_eq!(first.raw.size, 100);
    assert_eq!(first.raw.modified, Some(modified));
    assert_eq!(first.sidecar.map(|s| s.size), Some(7));
    assert_eq!(folder.photos[1].raw.size, 300);
    assert_eq!(folder.photos[1].sidecar, None);
    assert!(folder.orphans.is_empty());
    assert_eq!(folder.summary.raws, 2);
    assert_eq!(folder.summary.with_sidecar, 1);
    assert_eq!(folder.summary.orphans, 0);
    assert!(!folder.summary.cancelled && !folder.deadline_passed);
}

#[test]
fn a_sidecar_without_its_raw_is_an_orphan() {
    let dir = TempDir::new().unwrap();
    file(dir.path(), "kept.RAF", 1);
    file(dir.path(), "kept.RAF.apxmp", 2);
    file(dir.path(), "gone.RAF.apxmp", 3);
    // Not a raw, so its sidecar has none to follow.
    file(dir.path(), "picture.jpg.apxmp", 4);

    let folder = list(dir.path());

    assert_eq!(names(&folder.photos), ["kept.RAF"]);
    let orphans: Vec<_> = folder
        .orphans
        .iter()
        .map(|o| (&*o.name, o.file.size))
        .collect();
    assert_eq!(orphans, [("gone.RAF.apxmp", 3), ("picture.jpg.apxmp", 4)]);
    assert_eq!(folder.summary.orphans, 2);
    assert_eq!(folder.summary.with_sidecar, 1);
}

#[test]
fn extensions_count_in_any_case_and_other_files_are_skipped() {
    let dir = TempDir::new().unwrap();
    for name in ["a.raf", "b.Cr2", "C.DNG", "d.ARW", "e.nef"] {
        file(dir.path(), name, 1);
    }
    for name in [
        "notes.txt",
        "photo.jpg",
        "noextension",
        "trailing.",
        "a.raf.APXMP",
    ] {
        file(dir.path(), name, 1);
    }

    let folder = list(dir.path());

    assert_eq!(
        names(&folder.photos),
        ["C.DNG", "a.raf", "b.Cr2", "d.ARW", "e.nef"]
    );
    // The upper-case suffix is no sidecar.
    assert_eq!(folder.summary.other, 5);
    assert!(folder.orphans.is_empty());
}

#[test]
fn hidden_files_are_skipped_and_subfolders_named_without_descending() {
    let dir = TempDir::new().unwrap();
    file(dir.path(), "shown.RAF", 1);
    file(dir.path(), ".hidden.RAF", 1);
    // AppleDouble companions that macOS leaves on other file systems.
    file(dir.path(), "._shown.RAF", 1);
    file(dir.path(), ".hidden.RAF.apxmp", 1);
    fs::create_dir(dir.path().join("sub")).unwrap();
    file(&dir.path().join("sub"), "inner.RAF", 1);
    fs::create_dir(dir.path().join(".git")).unwrap();
    file(&dir.path().join(".git"), "inside.RAF", 1);

    let folder = list(dir.path());

    assert_eq!(names(&folder.photos), ["shown.RAF"]);
    assert!(folder.orphans.is_empty());
    assert_eq!(folder.summary.hidden, 4);
    assert_eq!(folder.summary.folders, 1);
    assert_eq!(folder.subfolders, ["sub"]);
    assert_eq!(folder.summary.other, 0);
}

/// Makes `name` a file last modified `age` ago.
fn aged(dir: &Path, name: &str, age: Duration) {
    file(dir, name, 5);
    fs::File::options()
        .write(true)
        .open(dir.join(name))
        .unwrap()
        .set_modified(SystemTime::now() - age)
        .unwrap();
}

const DAY: Duration = Duration::from_secs(24 * 60 * 60);

#[test]
fn leftover_temporaries_older_than_a_day_are_removed_and_counted() {
    let dir = TempDir::new().unwrap();
    file(dir.path(), "shown.RAF", 1);
    // Two of a crashed writer's, a sidecar's and an export's, a week and two days old.
    aged(dir.path(), ".~shown.RAF.apxmp.AbC123.tmp", 7 * DAY);
    aged(dir.path(), ".~IMG_0001.jpg.xYz789.tmp", 2 * DAY);
    // Young ones may be a write in progress.
    aged(
        dir.path(),
        ".~shown.RAF.apxmp.QwErTy.tmp",
        Duration::from_secs(60),
    );
    aged(
        dir.path(),
        ".~other.RAF.apxmp.ZzZzZz.tmp",
        DAY - Duration::from_secs(600),
    );

    let folder = list(dir.path());

    assert_eq!(folder.summary.stale_removed, 2);
    // The young ones are hidden files like any other.
    assert_eq!(folder.summary.hidden, 2);
    let mut left: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    left.sort();
    assert_eq!(
        left,
        [
            ".~other.RAF.apxmp.ZzZzZz.tmp",
            ".~shown.RAF.apxmp.QwErTy.tmp",
            "shown.RAF"
        ]
    );
    assert_eq!(names(&folder.photos), ["shown.RAF"]);
}

#[test]
fn a_file_that_merely_looks_like_a_temporary_is_never_touched() {
    let dir = TempDir::new().unwrap();
    let similar = [
        ".~notes.tmp",
        ".~shown.RAF.apxmp.AbC12.tmp",
        ".~shown.RAF.apxmp.AbC1234.tmp",
        ".shown.RAF.apxmp.AbC123.tmp",
        ".DSCF0001.RAF.apxmp.0123abcd.tmp",
        "~shown.RAF.apxmp.AbC123.tmp",
        "shown.RAF.apxmp.AbC123.tmp",
        ".~shown.RAF.apxmp.AbC123.tmp.bak",
        ".hidden.RAF",
    ];
    for name in similar {
        aged(dir.path(), name, 30 * DAY);
    }
    // A folder with a temporary's name, empty and old.
    fs::create_dir(dir.path().join(".~folder.AbC123.tmp")).unwrap();

    let folder = list(dir.path());

    assert_eq!(folder.summary.stale_removed, 0);
    // Seven of the names and the folder start with a dot.
    assert_eq!(folder.summary.hidden, 8);
    for name in similar {
        assert!(dir.path().join(name).exists(), "{name}");
    }
    assert!(dir.path().join(".~folder.AbC123.tmp").is_dir());
}

#[test]
fn chunks_cover_everything_once_in_order() {
    let dir = TempDir::new().unwrap();
    for i in 0..250 {
        let name = format!("IMG_{i:04}.CR2");
        file(dir.path(), &name, i);
        if i % 3 == 0 {
            file(dir.path(), &format!("{name}.apxmp"), 1);
        }
    }
    let listing = Listing::start(
        dir.path(),
        Options {
            chunk: 64,
            ..Options::default()
        },
    );

    let mut chunks = Vec::new();
    let mut ended = false;
    for event in listing.events().iter() {
        assert!(!ended, "an event after the end");
        match event {
            Event::Chunk(photos) => chunks.push(photos),
            Event::Done(summary) => {
                ended = true;
                assert_eq!(summary.raws, 250);
                assert_eq!(summary.with_sidecar, 84);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    assert!(ended);
    let sizes: Vec<usize> = chunks.iter().map(Vec::len).collect();
    assert_eq!(sizes, [64, 64, 64, 58]);
    let all: Vec<&str> = chunks.iter().flatten().map(|p| p.name.as_str()).collect();
    let expected: Vec<String> = (0..250).map(|i| format!("IMG_{i:04}.CR2")).collect();
    assert_eq!(all, expected);
}

#[test]
fn cancelling_stops_the_listing_early() {
    let dir = TempDir::new().unwrap();
    for i in 0..200 {
        file(dir.path(), &format!("IMG_{i:04}.CR2"), 1);
    }
    let listing = Listing::start(
        dir.path(),
        Options {
            chunk: 1,
            ..Options::default()
        },
    );

    // The channel holds a few events, so the worker is held back until we read: it cannot
    // have reached the end when the first chunk arrives.
    let Ok(Event::Chunk(first)) = listing.events().recv() else {
        panic!("no first chunk");
    };
    assert_eq!(first.len(), 1);
    listing.cancel();
    let mut photos = first.len();
    let mut summary = None;
    for event in listing.events().iter() {
        match event {
            Event::Chunk(chunk) => photos += chunk.len(),
            Event::Done(done) => summary = Some(done),
            other => panic!("unexpected {other:?}"),
        }
    }

    let summary = summary.expect("a cancelled listing still ends with Done");
    assert!(summary.cancelled);
    assert_eq!(summary.raws, photos);
    assert!(photos < 200, "listed all {photos} after the cancel");
}

#[test]
fn a_deadline_passed_is_reported_once_and_listing_goes_on() {
    let dir = TempDir::new().unwrap();
    for i in 0..40 {
        file(dir.path(), &format!("IMG_{i:02}.ARW"), 1);
    }

    // A deadline of nothing has passed as soon as the listing starts.
    let folder = Listing::start(
        dir.path(),
        Options {
            deadline: Some(Duration::ZERO),
            chunk: 10,
            ..Options::default()
        },
    );
    let mut passed = 0;
    let mut photos = 0;
    for event in folder.events().iter() {
        match event {
            Event::DeadlinePassed { limit } => {
                assert_eq!(limit, Duration::ZERO);
                passed += 1;
            }
            Event::Chunk(chunk) => photos += chunk.len(),
            Event::Done(summary) => assert!(!summary.cancelled),
            other => panic!("unexpected {other:?}"),
        }
    }
    assert_eq!(passed, 1);
    assert_eq!(photos, 40);

    // One far away is not reported, and collect notes the one that was.
    let folder = Listing::start(
        dir.path(),
        Options {
            deadline: Some(Duration::from_secs(3600)),
            ..Options::default()
        },
    )
    .collect()
    .unwrap();
    assert!(!folder.deadline_passed);
    let folder = Listing::start(
        dir.path(),
        Options {
            deadline: Some(Duration::ZERO),
            ..Options::default()
        },
    )
    .collect()
    .unwrap();
    assert!(folder.deadline_passed);
    assert_eq!(folder.summary.raws, 40);
}

#[test]
fn a_missing_folder_is_an_error() {
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("nowhere");

    let error = Listing::start(&missing, Options::default())
        .collect()
        .unwrap_err();

    let Error::List { path, source } = error else {
        panic!("expected a listing error");
    };
    assert_eq!(path, missing);
    assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn a_file_in_place_of_a_folder_is_an_error() {
    let dir = TempDir::new().unwrap();
    file(dir.path(), "a.RAF", 1);

    let error = Listing::start(dir.path().join("a.RAF"), Options::default())
        .collect()
        .unwrap_err();

    assert!(matches!(error, Error::List { .. }), "{error}");
}

#[cfg(unix)]
#[test]
fn an_unreadable_folder_is_an_error() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().unwrap();
    let locked = dir.path().join("locked");
    fs::create_dir(&locked).unwrap();
    file(&locked, "a.RAF", 1);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    // A superuser reads it anyway, and then there is nothing to test.
    let readable = fs::read_dir(&locked).is_ok();

    let result = Listing::start(&locked, Options::default()).collect();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();

    if !readable {
        let error = result.unwrap_err();
        let Error::List { source, .. } = error else {
            panic!("expected a listing error");
        };
        assert_eq!(source.kind(), std::io::ErrorKind::PermissionDenied);
    }
}

#[test]
fn an_empty_folder_lists_nothing() {
    let dir = TempDir::new().unwrap();

    let folder = list(dir.path());

    assert!(folder.photos.is_empty() && folder.orphans.is_empty());
    assert_eq!(folder.summary.raws, 0);
}

#[test]
fn nothing_follows_the_end_even_with_a_deadline_that_passes_during_it() {
    let dir = TempDir::new().unwrap();
    for i in 0..20 {
        file(dir.path(), &format!("IMG_{i:02}.ARW"), 1);
    }

    // How long a listing takes here, at best.
    let took = (0..20)
        .map(|_| list(dir.path()).summary.elapsed)
        .min()
        .unwrap();

    // Deadlines swept across that time, so that some timer fires as the worker ends: its
    // event must come before the end or not at all.
    for round in 0..2000u32 {
        let listing = Listing::start(
            dir.path(),
            Options {
                deadline: Some(took.mul_f64(0.5 + f64::from(round % 100) / 50.0)),
                chunk: 5,
                ..Options::default()
            },
        );
        let mut ended = false;
        for event in listing.events().iter() {
            assert!(!ended, "{event:?} came after the end (round {round})");
            ended = matches!(event, Event::Done(_) | Event::Failed(_));
        }
        assert!(ended);
    }
}

#[test]
fn an_upper_case_sidecar_suffix_is_another_file() {
    let dir = TempDir::new().unwrap();
    file(dir.path(), "a.RAF", 1);
    file(dir.path(), "a.RAF.APXMP", 1);

    let folder = list(dir.path());

    assert_eq!(folder.photos[0].sidecar, None);
    assert!(folder.orphans.is_empty());
    assert_eq!(folder.summary.other, 1);
}

#[cfg(unix)]
mod links {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::symlink;

    use super::*;

    #[test]
    fn a_link_to_a_raw_is_listed_with_the_raws_size() {
        let dir = TempDir::new().unwrap();
        let elsewhere = TempDir::new().unwrap();
        file(elsewhere.path(), "real.RAF", 42);
        symlink(
            elsewhere.path().join("real.RAF"),
            dir.path().join("link.RAF"),
        )
        .unwrap();

        let folder = list(dir.path());

        assert_eq!(names(&folder.photos), ["link.RAF"]);
        assert_eq!(folder.photos[0].raw.size, 42);
        assert_eq!(folder.summary.unreadable, 0);
    }

    #[test]
    fn a_link_to_a_folder_counts_as_a_folder() {
        let dir = TempDir::new().unwrap();
        let elsewhere = TempDir::new().unwrap();
        file(elsewhere.path(), "inner.RAF", 1);
        symlink(elsewhere.path(), dir.path().join("folder.RAF")).unwrap();

        let folder = list(dir.path());

        assert!(folder.photos.is_empty());
        assert_eq!(folder.summary.folders, 1);
        assert_eq!(folder.subfolders, ["folder.RAF"]);
        assert_eq!(folder.summary.unreadable, 0);
    }

    #[test]
    fn a_broken_link_is_unreadable_and_not_listed() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("missing");
        symlink(&missing, dir.path().join("broken.RAF")).unwrap();
        symlink(&missing, dir.path().join("gone.RAF.apxmp")).unwrap();
        file(dir.path(), "ok.RAF", 1);

        let folder = list(dir.path());

        assert_eq!(names(&folder.photos), ["ok.RAF"]);
        assert!(folder.orphans.is_empty());
        // The raw's link and the orphan sidecar's.
        assert_eq!(folder.summary.unreadable, 2);
    }

    #[test]
    fn a_name_that_is_not_text_is_counted_apart_and_not_listed() {
        let dir = TempDir::new().unwrap();
        file(dir.path(), "ok.RAF", 1);
        let bad = dir.path().join(OsStr::from_bytes(b"bad\xff.RAF"));
        // Some file systems, macOS's among them, refuse such a name.
        if fs::write(&bad, [0u8]).is_err() {
            return;
        }

        let folder = list(dir.path());

        assert_eq!(names(&folder.photos), ["ok.RAF"]);
        assert_eq!(folder.summary.not_text, 1);
        assert_eq!(folder.summary.other, 0);
    }
}
