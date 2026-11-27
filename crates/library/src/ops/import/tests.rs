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

//! Import's source listed as the session's job lists it: its events, their order and
//! chunks, the capture times after them, and a cancel or a failure ending it.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use tempfile::TempDir;

use super::*;
use crate::source::LeftOut;

/// The events listing `source` sent, in chunks of `chunk`, as a card where `card` is its
/// volume's mount point, and the listing it ended with.
fn run(
    source: &Path,
    card: Option<PathBuf>,
    chunk: usize,
    cancel: &(dyn Fn() -> bool + Sync),
) -> (Vec<LibraryEvent>, Option<Listed>) {
    let heard = Mutex::new(Vec::new());
    let how = How {
        chunk,
        mount: move |_: &Path| card.clone(),
    };
    let listed = list_source_on(
        7,
        source,
        false,
        how,
        &|event| heard.lock().unwrap().push(event),
        cancel,
    );
    (heard.into_inner().unwrap(), listed)
}

/// A folder of `count` raws, `IMG_0000.CR2` on, and a JPEG.
fn raws(count: usize) -> TempDir {
    let dir = TempDir::new().unwrap();
    for i in 0..count {
        fs::write(dir.path().join(format!("IMG_{i:04}.CR2")), [0u8; 4]).unwrap();
    }
    fs::write(dir.path().join("IMG_0000.JPG"), [0u8; 4]).unwrap();
    dir
}

/// More raws than a chunk holds come in several chunks, in path order, the last done with
/// what was left out; then one capture time each, in the listing's order.
#[test]
fn a_listing_comes_in_chunks_then_its_capture_times() {
    let dir = raws(CHUNK + 44);
    let (events, listed) = run(dir.path(), None, CHUNK, &|| false);
    let listed = listed.expect("listed");
    let (last, chunks) = events.split_last().unwrap();
    let LibraryEvent::SourceTaken { open: 7, shots } = last else {
        panic!("{last:?}");
    };
    // Raws of four zero bytes hold no EXIF.
    assert_eq!(shots, &vec![SourceShot::default(); CHUNK + 44]);
    let mut names = Vec::new();
    let mut shape = Vec::new();
    for event in chunks {
        let LibraryEvent::SourceListed {
            open: 7,
            folder,
            files,
            done,
            left_out,
        } = event
        else {
            panic!("{event:?}");
        };
        assert_eq!(folder, dir.path());
        shape.push((files.len(), *done, left_out.clone()));
        names.extend(files.iter().map(|file| file.path.clone()));
    }
    let left_out = LeftOut {
        non_raws: 1,
        ..LeftOut::default()
    };
    assert_eq!(
        shape,
        [(CHUNK, false, None), (44, true, Some(left_out))],
        "{shape:?}"
    );
    let expected: Vec<PathBuf> = (0..CHUNK + 44)
        .map(|i| PathBuf::from(format!("IMG_{i:04}.CR2")))
        .collect();
    assert_eq!(names, expected);
    assert_eq!(listed.files.len(), CHUNK + 44);
}

/// A card listed from one of its folders names its `DCIM` as the folder listed, each raw by
/// its path under it; an empty source sends one chunk, empty and done.
#[test]
fn a_card_s_listing_names_its_dcim() {
    let dir = TempDir::new().unwrap();
    let day = dir.path().join("DCIM/100CANON");
    fs::create_dir_all(&day).unwrap();
    fs::write(day.join("IMG_0001.CR2"), [0u8; 4]).unwrap();
    let (events, _) = run(&day, Some(dir.path().to_owned()), 4, &|| false);
    let LibraryEvent::SourceListed { folder, files, .. } = &events[0] else {
        panic!("{events:?}");
    };
    assert_eq!(folder, &dir.path().join("DCIM"));
    assert_eq!(files[0].path, Path::new("100CANON/IMG_0001.CR2"));

    let empty = TempDir::new().unwrap();
    let (events, _) = run(empty.path(), None, 4, &|| false);
    assert_eq!(
        events,
        [
            LibraryEvent::SourceListed {
                open: 7,
                folder: empty.path().to_owned(),
                files: Vec::new(),
                done: true,
                left_out: Some(LeftOut::default()),
            },
            LibraryEvent::SourceTaken {
                open: 7,
                shots: Vec::new(),
            },
        ]
    );
}

/// A cancel after the first chunk, as the next listing's or a `CancelListing`'s, sends
/// nothing more: no chunk says done and no capture times come.
#[test]
fn a_cancel_sends_nothing_more() {
    let dir = raws(10);
    let stop = AtomicBool::new(false);
    let heard = Mutex::new(Vec::new());
    let how = How {
        chunk: 3,
        mount: |_: &Path| None,
    };
    let listed = list_source_on(
        7,
        dir.path(),
        false,
        how,
        &|event| {
            stop.store(true, Ordering::SeqCst);
            heard.lock().unwrap().push(event);
        },
        &|| stop.load(Ordering::SeqCst),
    );
    assert_eq!(listed, None);
    let heard = heard.into_inner().unwrap();
    assert!(
        matches!(
            heard.as_slice(),
            [LibraryEvent::SourceListed { done: false, .. }]
        ),
        "{heard:?}"
    );
}

/// A source that cannot be listed is one event in plain words, naming it, and nothing else.
#[test]
fn a_source_that_cannot_be_listed_fails_in_one_event() {
    let dir = TempDir::new().unwrap();
    let gone = dir.path().join("gone");
    let (events, listed) = run(&gone, None, 4, &|| false);
    assert_eq!(listed, None);
    let [LibraryEvent::SourceFailed { open: 7, reason }] = events.as_slice() else {
        panic!("{events:?}");
    };
    assert!(reason.contains(&*gone.to_string_lossy()), "{reason}");
}

/// A cloud file not on the disk is listed and its capture time never read; the others' are.
#[test]
fn a_cloud_file_s_capture_time_is_not_read() {
    let dir = raws(2);
    let (cloud, local) = (
        dir.path().join("IMG_0000.CR2"),
        dir.path().join("IMG_0001.CR2"),
    );
    crate::testing::evict(&cloud);
    let (events, _) = run(dir.path(), None, 4, &|| false);
    crate::testing::restore(&cloud);
    assert!(matches!(
        events.last(),
        Some(LibraryEvent::SourceTaken { shots, .. }) if shots.len() == 2
    ));
    assert_eq!(crate::testing::reads(&cloud), 0);
    assert_eq!(crate::testing::reads(&local), 1);
}

/// `path` as a folder pattern names it, with forward slashes, which a pattern never takes
/// as an escape.
fn pattern(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// The trees of `base` and `default`, folders given whole.
fn trees(base: &Path, default: &Path) -> Trees {
    Trees {
        base_folder: pattern(base),
        default_folder: pattern(default),
        places: Places::default(),
    }
}

/// The events of check number 2 of `source`'s listing against `trees`, in chunks of
/// `chunk`, as the job would send them.
fn checked(
    source: &Path,
    trees: &Trees,
    chunk: usize,
    cancel: &(dyn Fn() -> bool + Sync),
) -> (Vec<LibraryEvent>, Ended) {
    let listed = source::list(source, false, |_| None, &|| false)
        .unwrap()
        .unwrap();
    let heard = Mutex::new(Vec::new());
    let fingerprints = Arc::new(Fingerprints::default());
    let ended = check_in_chunks(
        (7, 2),
        &listed,
        trees,
        (&fingerprints, chunk),
        &|event| heard.lock().unwrap().push(event),
        cancel,
    );
    (heard.into_inner().unwrap(), ended)
}

/// Every state the checks in `events` gave, in order.
fn states(events: &[LibraryEvent]) -> Vec<SourceState> {
    (events.iter())
        .flat_map(|event| match event {
            LibraryEvent::SourceChecked { states, .. } => states.clone(),
            other => panic!("{other:?}"),
        })
        .collect()
}

/// A card of five raws, each its own bytes, and a destination holding three of them, one
/// renamed with `_01` and keeping its camera name in its `.apxmp`; and a sixth on the card
/// whose name and size one there shares, but not its bytes.
struct Imported {
    _dir: TempDir,
    card: PathBuf,
    destination: PathBuf,
}

fn imported() -> Imported {
    let dir = TempDir::new().unwrap();
    let card = dir.path().join("DCIM/100CANON");
    let shoot = dir.path().join("Apertrust/20261108_Studio");
    for folder in [&card, &shoot] {
        fs::create_dir_all(folder).unwrap();
    }
    for i in 1..=5 {
        let bytes = format!("raw number {i}");
        fs::write(card.join(format!("IMG_000{i}.CR2")), &bytes).unwrap();
    }
    fs::write(card.join("IMG_0006.CR2"), b"raw number 6").unwrap();
    for i in [1, 2] {
        let bytes = format!("raw number {i}");
        fs::write(shoot.join(format!("IMG_000{i}.CR2")), &bytes).unwrap();
    }
    fs::write(shoot.join("IMG_0003_01.CR2"), b"raw number 3").unwrap();
    let mut edits = crate::sidecar::Edits::new("test");
    edits
        .other
        .push(crate::imported::tests::preserved("IMG_0003.CR2"));
    let sidecar = shoot.join("IMG_0003_01.CR2.apxmp");
    fs::write(&sidecar, edits.to_xml()).unwrap();
    // Its name and size, not its bytes.
    fs::write(shoot.join("IMG_0006.CR2"), b"raw number 7").unwrap();
    Imported {
        destination: dir.path().join("Apertrust"),
        card,
        _dir: dir,
    }
}

/// Three of the card's raws are imported before, each naming its copy, the renamed one by
/// the name its sidecar keeps; the two with no candidate are new and never read, and the
/// one whose candidate's bytes differ is new.
#[test]
fn the_photos_imported_before_are_told_with_their_copies() {
    let at = imported();
    let shoot = at.destination.join("20261108_Studio");
    let trees = trees(&at.destination, &at.destination);
    let (events, ended) = checked(&at.card, &trees, CHUNK, &|| false);
    assert_eq!(ended, Ended::Listed);
    let copy = |name: &str| SourceState::ImportedBefore {
        copy: shoot.join(name),
    };
    assert_eq!(
        states(&events),
        [
            copy("IMG_0001.CR2"),
            copy("IMG_0002.CR2"),
            copy("IMG_0003_01.CR2"),
            SourceState::New,
            SourceState::New,
            SourceState::New,
        ]
    );
    let [
        LibraryEvent::SourceChecked {
            open: 7,
            check: 2,
            folders,
            checked: 6,
            of: 6,
            done: true,
            ..
        },
    ] = events.as_slice()
    else {
        panic!("{events:?}");
    };
    assert_eq!(folders, &[PathBuf::from(pattern(&at.destination))]);
    for new in ["IMG_0004.CR2", "IMG_0005.CR2"] {
        assert_eq!(crate::testing::reads(&at.card.join(new)), 0, "{new}");
    }
    assert_eq!(crate::testing::reads(&at.card.join("IMG_0006.CR2")), 1);
}

/// A file of a photo's name and size whose first or last 64 KiB differ is no copy of it;
/// one that differs only between them is taken for one, a probable match.
#[test]
fn the_fingerprint_reads_the_first_and_last_64_kib() {
    let dir = TempDir::new().unwrap();
    let (card, destination) = (dir.path().join("card"), dir.path().join("Apertrust"));
    for folder in [&card, &destination] {
        fs::create_dir_all(folder).unwrap();
    }
    let raw: Vec<u8> = (0..200 * 1024).map(|i| (i % 251) as u8).collect();
    let changed = |at: usize| {
        let mut bytes = raw.clone();
        bytes[at] ^= 0xff;
        bytes
    };
    for name in ["HEAD.CR2", "MIDDLE.CR2", "TAIL.CR2"] {
        fs::write(card.join(name), &raw).unwrap();
    }
    fs::write(destination.join("HEAD.CR2"), changed(10)).unwrap();
    fs::write(destination.join("MIDDLE.CR2"), changed(100 * 1024)).unwrap();
    fs::write(destination.join("TAIL.CR2"), changed(raw.len() - 10)).unwrap();
    let (events, _) = checked(&card, &trees(&destination, &destination), CHUNK, &|| false);
    let middle = PathBuf::from(pattern(&destination)).join("MIDDLE.CR2");
    assert_eq!(
        states(&events),
        [
            SourceState::New,
            SourceState::ImportedBefore { copy: middle },
            SourceState::New,
        ]
    );
}

/// A source with no photo is checked with no tree walked: one chunk, empty and done, even
/// where walking the tree would crash.
#[test]
fn an_empty_source_is_checked_with_no_walk() {
    let dir = TempDir::new().unwrap();
    let (card, destination) = (dir.path().join("card"), dir.path().join("Apertrust"));
    for folder in [&card, &destination] {
        fs::create_dir_all(folder).unwrap();
    }
    let tree = PathBuf::from(pattern(&destination));
    crate::imported::crash::mark(&tree);
    let sought = crate::imported::Sought::new([]);
    let walked = crate::imported::candidates(&[tree], &card, &[], sought, &|| false);
    assert_eq!(walked, Walked::Found(Vec::new()));
    let (events, ended) = checked(&card, &trees(&destination, &destination), CHUNK, &|| false);
    assert_eq!(ended, Ended::Listed);
    assert!(
        matches!(
            events.as_slice(),
            [LibraryEvent::SourceChecked {
                states,
                checked: 0,
                of: 0,
                done: true,
                ..
            }] if states.is_empty()
        ),
        "{events:?}"
    );
}

/// A walk that crashes is no cancel: every photo is told new, in chunks with their counts,
/// the last done, so the sheet never waits for a check that will not come.
#[test]
fn a_walk_that_crashes_tells_every_photo_new() {
    let at = imported();
    crate::imported::crash::mark(&PathBuf::from(pattern(&at.destination)));
    let trees = trees(&at.destination, &at.destination);
    let (events, ended) = checked(&at.card, &trees, 4, &|| false);
    assert_eq!(ended, Ended::Listed);
    assert_eq!(states(&events), vec![SourceState::New; 6]);
    assert!(matches!(
        events.last(),
        Some(LibraryEvent::SourceChecked {
            checked: 6,
            of: 6,
            done: true,
            ..
        })
    ));
}

/// A candidate that is a cloud file not on the disk makes its photo probably imported,
/// nothing of it read.
#[test]
fn a_cloud_candidate_is_probably_imported_and_never_read() {
    let at = imported();
    let cloud = at.destination.join("20261108_Studio/IMG_0002.CR2");
    crate::testing::evict(&cloud);
    let trees = trees(&at.destination, &at.destination);
    let (events, _) = checked(&at.card, &trees, CHUNK, &|| false);
    crate::testing::restore(&cloud);
    assert_eq!(
        states(&events)[1],
        SourceState::ProbablyImported {
            candidate: PathBuf::from(pattern(&at.destination)).join("20261108_Studio/IMG_0002.CR2"),
        }
    );
    assert_eq!(crate::testing::reads(&cloud), 0);
}

/// A source inside the tree walked never matches itself; a tree that is not there holds
/// nothing, every photo new, with no error.
#[test]
fn a_source_never_matches_itself_and_a_missing_tree_holds_nothing() {
    let at = imported();
    let inside = at.destination.join("20261108_Studio");
    let trees_inside = trees(&at.destination, &at.destination);
    let (events, _) = checked(&inside, &trees_inside, CHUNK, &|| false);
    assert!(
        states(&events)
            .iter()
            .all(|state| *state == SourceState::New)
    );
    let gone = at.destination.join("gone");
    let (events, ended) = checked(&at.card, &trees(&gone, &gone), CHUNK, &|| false);
    assert_eq!(ended, Ended::Listed);
    assert_eq!(states(&events), vec![SourceState::New; 6]);
}

/// More photos than a chunk holds are told in several chunks, each saying how many are
/// checked of how many, the last done; a cancel after the first sends nothing more.
#[test]
fn a_check_comes_in_chunks_and_a_cancel_ends_it() {
    let at = imported();
    let trees = trees(&at.destination, &at.destination);
    let (events, _) = checked(&at.card, &trees, 4, &|| false);
    let shape: Vec<(usize, usize, usize, bool)> = (events.iter())
        .map(|event| match event {
            LibraryEvent::SourceChecked {
                states,
                checked,
                of,
                done,
                ..
            } => (states.len(), *checked, *of, *done),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(shape, [(4, 4, 6, false), (2, 6, 6, true)]);

    let stop = AtomicBool::new(false);
    let listed = source::list(&at.card, false, |_| None, &|| false)
        .unwrap()
        .unwrap();
    let heard = Mutex::new(Vec::new());
    let fingerprints = Arc::new(Fingerprints::default());
    let ended = check_in_chunks(
        (7, 2),
        &listed,
        &trees,
        (&fingerprints, 4),
        &|event| {
            stop.store(true, Ordering::SeqCst);
            heard.lock().unwrap().push(event);
        },
        &|| stop.load(Ordering::SeqCst),
    );
    assert_eq!(ended, Ended::Cancelled);
    assert_eq!(heard.into_inner().unwrap().len(), 1);
}

/// Everything under `root`, each file with its bytes and its modified time.
fn snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>, std::time::SystemTime)> {
    let mut found = Vec::new();
    let mut queue = vec![root.to_owned()];
    while let Some(folder) = queue.pop() {
        for entry in fs::read_dir(&folder).unwrap() {
            let path = entry.unwrap().path();
            let meta = fs::metadata(&path).unwrap();
            if meta.is_dir() {
                queue.push(path.clone());
                continue;
            }
            found.push((
                path.clone(),
                fs::read(&path).unwrap(),
                meta.modified().unwrap(),
            ));
        }
    }
    found.sort();
    found
}

/// Nothing is written on the source or in the trees.
#[test]
fn a_check_writes_nothing() {
    let at = imported();
    let root = at.destination.parent().unwrap().to_owned();
    let before = snapshot(&root);
    let trees = trees(&at.destination, &at.destination);
    checked(&at.card, &trees, CHUNK, &|| false);
    assert_eq!(snapshot(&root), before);
}

/// A source's photo is asked for by a path inside the folder listed: relative, each part
/// a name.
#[test]
fn a_source_s_photo_is_a_path_inside_it() {
    for inside in ["IMG_0001.CR2", "100CANON/IMG_0001.CR2"] {
        assert!(relative_inside(Path::new(inside)), "{inside}");
    }
    for outside in [
        "",
        "/IMG_0001.CR2",
        "../IMG_0001.CR2",
        "100CANON/../../x.CR2",
        "./x.CR2",
    ] {
        assert!(!relative_inside(Path::new(outside)), "{outside}");
    }
}
