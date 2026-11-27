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
