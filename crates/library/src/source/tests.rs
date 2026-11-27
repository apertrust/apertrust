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

//! The card rule and the source's listing, on temporary trees standing in for cards.

use std::time::{Duration, Instant};

use tempfile::TempDir;

use super::*;
use crate::listing::stall;

/// A tree standing in for a card, its root the volume's: `DCIM/100CANON` and
/// `DCIM/101CANON` with raws, a sidecar, a JPEG and a hidden raw, and a raw outside `DCIM`.
fn card_tree() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    for folder in ["DCIM/100CANON", "DCIM/101CANON", "MISC"] {
        fs::create_dir_all(root.join(folder)).unwrap();
    }
    for file in [
        "DCIM/100CANON/IMG_0002.CR2",
        "DCIM/100CANON/IMG_0001.CR2",
        "DCIM/100CANON/IMG_0001.CR2.xmp",
        "DCIM/100CANON/IMG_0001.CR2.apxmp",
        "DCIM/100CANON/IMG_0003.JPG",
        "DCIM/100CANON/.IMG_0004.CR2",
        "DCIM/101CANON/IMG_0101.CR2",
        "MISC/IMG_9999.CR2",
    ] {
        fs::write(root.join(file), b"raw bytes").unwrap();
    }
    dir
}

/// The tree under `root` the volume's, as its mount point stands in for the system's.
fn on_card(root: &Path) -> impl Fn(&Path) -> Option<PathBuf> + Send + 'static {
    let root = root.to_owned();
    move |_| Some(root.clone())
}

fn paths(listed: &Listed) -> Vec<&Path> {
    listed
        .files
        .iter()
        .map(|file| file.path.as_path())
        .collect()
}

/// A card is a volume with `DCIM` at its top: any path on it is on the card; one outside
/// the volume, or a volume with no `DCIM`, is not.
#[test]
fn a_card_is_a_volume_with_dcim_at_its_top() {
    let dir = card_tree();
    let root = dir.path();
    let card = Card {
        mount: root.to_owned(),
        dcim: root.join(DCIM),
    };
    for path in [
        root.to_owned(),
        root.join("DCIM/100CANON"),
        root.join("MISC"),
    ] {
        assert_eq!(card_at(&path, root).as_ref(), Some(&card), "{path:?}");
    }
    let elsewhere = TempDir::new().unwrap();
    assert_eq!(card_at(elsewhere.path(), root), None);
    // DCIM below the top makes no card.
    assert_eq!(
        card_at(&root.join("DCIM/100CANON"), &root.join("DCIM")),
        None
    );
    // A file named DCIM is no folder.
    let file = TempDir::new().unwrap();
    fs::write(file.path().join(DCIM), b"").unwrap();
    assert_eq!(card_at(file.path(), file.path()), None);
}

/// A folder's mount point holds it, and is on its device.
#[cfg(unix)]
#[test]
fn a_folder_s_mount_point_holds_it() {
    let dir = TempDir::new().unwrap();
    let mount = mount_point(dir.path()).expect("a mount point");
    assert!(dir.path().starts_with(&mount), "{mount:?}");
    assert_eq!(mount_point(&dir.path().join("gone")), None);
}

/// Listed from the card's root, from its `DCIM` or from one folder of it, a card gives
/// every raw under `DCIM`, in the order of their paths, and nothing outside it.
#[test]
fn a_card_s_dcim_is_listed_whole_from_any_folder_of_it() {
    let dir = card_tree();
    let root = dir.path();
    for source in [root.to_owned(), root.join(DCIM), root.join("DCIM/100CANON")] {
        let listed = list(&source, false, on_card(root), &|| false)
            .unwrap()
            .unwrap();
        assert_eq!(listed.folder, root.join(DCIM), "{source:?}");
        assert_eq!(
            paths(&listed),
            [
                Path::new("100CANON/IMG_0001.CR2"),
                Path::new("100CANON/IMG_0002.CR2"),
                Path::new("101CANON/IMG_0101.CR2"),
            ],
            "{source:?}"
        );
        assert!(listed.files.iter().all(|file| file.size == 9 && file.local));
        assert_eq!(listed.folders[0], root.join(DCIM));
        assert_eq!(listed.folders.len(), 3);
    }
}

/// A folder that is no card is listed alone, its subfolders' raws left out, unless they
/// are asked for.
#[test]
fn a_folder_is_listed_alone_unless_its_subfolders_are_asked_for() {
    let dir = card_tree();
    let folder = dir.path().join("DCIM");
    fs::write(folder.join("TOP.CR2"), b"raw").unwrap();
    let alone = list(&folder, false, |_| None, &|| false).unwrap().unwrap();
    assert_eq!(alone.folder, folder);
    assert_eq!(paths(&alone), [Path::new("TOP.CR2")]);
    assert_eq!(alone.folders, std::slice::from_ref(&folder));
    let whole = list(&folder, true, |_| None, &|| false).unwrap().unwrap();
    assert_eq!(whole.files.len(), 4);
}

/// Sidecars and other files are counted and left out, hidden ones neither; a cloud file
/// not on the disk is listed by its name and size, none of its bytes read.
#[test]
fn other_files_are_counted_and_a_cloud_file_listed_unread() {
    let dir = card_tree();
    let root = dir.path();
    let cloud = root.join("DCIM/101CANON/IMG_0101.CR2");
    crate::testing::evict(&cloud);
    let listed = list(root, false, on_card(root), &|| false)
        .unwrap()
        .unwrap();
    crate::testing::restore(&cloud);
    assert_eq!(
        listed.left_out,
        LeftOut {
            non_raws: 1,
            sidecars: 2,
            unreadable: Vec::new(),
        }
    );
    let file = &listed.files[2];
    assert_eq!((file.size, file.local), (9, false));
    assert_eq!(crate::testing::reads(&cloud), 0);
}

/// Everything under `root`, each path with its bytes and its modified time.
fn snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>, SystemTime)> {
    let mut found = Vec::new();
    let mut queue = vec![root.to_owned()];
    while let Some(folder) = queue.pop() {
        for entry in fs::read_dir(&folder).unwrap() {
            let path = entry.unwrap().path();
            let meta = fs::symlink_metadata(&path).unwrap();
            if meta.is_dir() {
                queue.push(path.clone());
            }
            let bytes = if meta.is_file() {
                fs::read(&path).unwrap()
            } else {
                Vec::new()
            };
            found.push((path, bytes, meta.modified().unwrap()));
        }
    }
    found.sort();
    found
}

/// Nothing is written on the source: listed, it is the same byte for byte, a stale
/// temporary a folder's listing would remove included.
#[test]
fn the_listing_writes_nothing_on_the_source() {
    let dir = card_tree();
    let root = dir.path();
    let stale = root.join("DCIM/100CANON/.~IMG_0001.CR2.a1b2c3.tmp");
    fs::write(&stale, b"half").unwrap();
    let old = SystemTime::now() - Duration::from_secs(3 * 86_400);
    fs::File::options()
        .write(true)
        .open(&stale)
        .unwrap()
        .set_modified(old)
        .unwrap();
    assert!(apertrust_base::fs::is_temporary_name(
        &stale.file_name().unwrap().to_string_lossy()
    ));
    let before = snapshot(root);
    list(root, false, on_card(root), &|| false)
        .unwrap()
        .unwrap();
    assert!(stale.exists());
    assert_eq!(snapshot(root), before);
}

/// A source that is not there, no folder, or relative fails, naming it.
#[test]
fn a_source_missing_no_folder_or_relative_fails() {
    let dir = card_tree();
    let gone = dir.path().join("gone");
    let file = dir.path().join("MISC/IMG_9999.CR2");
    let error = list(&gone, false, |_| None, &|| false).unwrap_err();
    assert!(matches!(error, Error::Missing { .. }), "{error}");
    assert!(error.to_string().contains(&*gone.to_string_lossy()));
    let error = list(&file, false, |_| None, &|| false).unwrap_err();
    assert!(matches!(error, Error::NotAFolder { .. }), "{error}");
    let error = list(Path::new("DCIM"), false, |_| None, &|| false).unwrap_err();
    assert!(matches!(error, Error::Relative { .. }), "{error}");
}

/// A folder of the card that cannot be read is named and skipped; the rest is listed.
#[cfg(unix)]
#[test]
fn a_folder_that_cannot_be_read_is_named_and_skipped() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = card_tree();
    let root = dir.path();
    let locked = root.join("DCIM/101CANON");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    // As root, nothing is refused: there is nothing to test.
    if fs::read_dir(&locked).is_ok() {
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    let listed = list(root, false, on_card(root), &|| false);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    let listed = listed.unwrap().unwrap();
    assert_eq!(listed.files.len(), 2);
    let [unreadable] = listed.left_out.unreadable.as_slice() else {
        panic!("{:?}", listed.left_out);
    };
    assert_eq!(unreadable.folder, Path::new("101CANON"));
    assert!(!unreadable.reason.is_empty());
}

/// Waits until a listing has come to `folder`, held.
fn reached(folder: &Path) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !stall::waited(folder) {
        assert!(
            Instant::now() < deadline,
            "the listing never came to {folder:?}"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

/// A card pulled out while it is read ends the listing as gone.
#[test]
fn a_source_removed_while_it_is_read_is_gone() {
    let dir = card_tree();
    let root = dir.path().to_owned();
    let held = root.join("DCIM/101CANON");
    stall::hold(&held);
    let listing = {
        let root = root.clone();
        thread::spawn(move || list(&root, false, on_card(&root), &|| false))
    };
    reached(&held);
    fs::remove_dir_all(root.join(DCIM)).unwrap();
    stall::release(&held);
    let error = listing.join().unwrap().unwrap_err();
    assert!(matches!(error, Error::Gone { .. }), "{error}");
}

/// A cancel returns at once while a folder of the source hangs, as one on a share that
/// does not answer, without waiting for it.
#[test]
fn a_cancel_returns_while_a_folder_hangs() {
    let dir = card_tree();
    let root = dir.path();
    let held = root.join(DCIM);
    stall::hold(&held);
    let started = Instant::now();
    let listed = list(root, false, on_card(root), &|| {
        stall::waited(&held) && started.elapsed() > Duration::from_millis(50)
    });
    let took = started.elapsed();
    stall::release(&held);
    assert!(matches!(listed, Ok(None)), "{listed:?}");
    assert!(took < Duration::from_secs(5), "{took:?}");
}
