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

//! The walk of a folder's whole tree, on temporary folders: its counts, its order and
//! batches, what it leaves out and what stops it.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use apertrust_library::ops::{self, Ended, LibraryEvent, WalkedFolder};
use apertrust_library::walk::BATCH;
use tempfile::TempDir;

fn raws(dir: &Path, names: &[&str]) {
    fs::create_dir_all(dir).unwrap();
    for name in names {
        fs::write(dir.join(name), [0u8; 4]).unwrap();
    }
}

/// The batches a walk of `root` sent, each with whether it said done, and how it ended.
fn walk(
    root: &Path,
    cancel: &(dyn Fn() -> bool + Sync),
) -> (Vec<(Vec<WalkedFolder>, bool)>, Ended) {
    let heard = Mutex::new(Vec::new());
    let ended = ops::walk(
        5,
        root,
        &|event| match event {
            LibraryEvent::Walked {
                open,
                folders,
                done,
            } => {
                assert_eq!(open, 5);
                heard.lock().unwrap().push((folders, done));
            }
            other => panic!("{other:?}"),
        },
        cancel,
    );
    (heard.into_inner().unwrap(), ended)
}

/// Every folder the batches hold, in their order.
fn folders(batches: &[(Vec<WalkedFolder>, bool)]) -> Vec<WalkedFolder> {
    batches
        .iter()
        .flat_map(|(folders, _)| folders.clone())
        .collect()
}

fn find<'a>(walked: &'a [WalkedFolder], folder: &Path) -> &'a WalkedFolder {
    walked
        .iter()
        .find(|walked| walked.folder == folder)
        .unwrap_or_else(|| panic!("{folder:?} walked: {walked:?}"))
}

/// Each folder's own raws counted by the decoder's extensions, in any case, the other
/// files not; its subfolders named in the file browser's order; the root first, then each
/// level before the next; hidden folders left out and not walked.
#[test]
fn the_walk_counts_each_folder_s_raws_level_by_level() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().canonicalize().unwrap();
    raws(&root, &["a.ARW", "b.arw", "notes.txt", "c.DNG.apxmp"]);
    raws(&root.join("Day 10"), &["x.CR3"]);
    raws(&root.join("Day 2"), &[]);
    raws(
        &root.join("Day 2").join("Picks"),
        &["p.NEF", "q.NEF", "r.NEF"],
    );
    raws(&root.join("Day 10").join("Deep").join("Deeper"), &["z.RAF"]);
    raws(&root.join(".hidden"), &["h.ARW"]);
    raws(&root.join("Day 2").join(".thumbs"), &["t.ARW"]);

    let (batches, ended) = walk(&root, &|| false);

    assert_eq!(ended, Ended::Listed);
    assert_eq!(batches.len(), 1, "a small tree is one batch");
    assert!(batches[0].1, "the last batch says done");
    let walked = folders(&batches);
    let order: Vec<PathBuf> = walked.iter().map(|w| w.folder.clone()).collect();
    assert_eq!(
        order,
        [
            root.clone(),
            root.join("Day 2"),
            root.join("Day 10"),
            root.join("Day 2").join("Picks"),
            root.join("Day 10").join("Deep"),
            root.join("Day 10").join("Deep").join("Deeper"),
        ]
    );
    let top = find(&walked, &root);
    assert_eq!(top.raws, 2);
    assert_eq!(top.subfolders, ["Day 2", "Day 10"]);
    assert_eq!(top.unreadable, None);
    assert_eq!(find(&walked, &root.join("Day 2")).subfolders, ["Picks"]);
    assert_eq!(find(&walked, &root.join("Day 2").join("Picks")).raws, 3);
    assert_eq!(find(&walked, &root.join("Day 10")).raws, 1);
    let deeper = root.join("Day 10").join("Deep").join("Deeper");
    assert_eq!(find(&walked, &deeper).raws, 1);
    assert!(
        walked
            .iter()
            .all(|w| !w.folder.to_string_lossy().contains(".hidden"))
    );
    assert!(walked.iter().all(|w| !w.folder.ends_with(".thumbs")));
}

/// A big tree comes in batches of at most [`BATCH`] folders, only the last saying done,
/// a shallower folder never after a deeper one.
#[test]
fn a_big_tree_comes_in_batches_shallow_folders_first() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().canonicalize().unwrap();
    for day in 0..BATCH {
        raws(&root.join(format!("Day {day}")).join("Picks"), &["a.ARW"]);
    }

    let (batches, ended) = walk(&root, &|| false);

    assert_eq!(ended, Ended::Listed);
    assert!(batches.len() >= 2, "{} batches", batches.len());
    assert!(batches.iter().all(|(folders, _)| folders.len() <= BATCH));
    let done: Vec<bool> = batches.iter().map(|(_, done)| *done).collect();
    assert_eq!(done.iter().filter(|done| **done).count(), 1);
    assert_eq!(done.last(), Some(&true));
    let depths: Vec<usize> = folders(&batches)
        .iter()
        .map(|w| w.folder.strip_prefix(&root).unwrap().components().count())
        .collect();
    assert_eq!(depths.len(), 1 + 2 * BATCH);
    assert!(
        depths.windows(2).all(|pair| pair[0] <= pair[1]),
        "{depths:?}"
    );
}

/// A link to a folder is named, as the listing names it, but never walked into, so a loop
/// of links ends; a folder that cannot be read is recorded so, and the walk goes on.
#[cfg(unix)]
#[test]
fn links_are_not_followed_and_an_unreadable_folder_is_recorded() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = TempDir::new().unwrap();
    let root = dir.path().canonicalize().unwrap();
    raws(&root.join("Real"), &["a.ARW"]);
    std::os::unix::fs::symlink(&root, root.join("Real").join("Back to the top")).unwrap();
    std::os::unix::fs::symlink(root.join("Real"), root.join("Also real")).unwrap();
    raws(&root.join("Locked"), &["b.ARW"]);
    raws(&root.join("Open"), &["c.ARW"]);
    let locked = root.join("Locked");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    // As root the folder can still be read: then there is nothing to test of it.
    let readable = fs::read_dir(&locked).is_ok();

    let (batches, ended) = walk(&root, &|| false);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();

    assert_eq!(ended, Ended::Listed);
    let walked = folders(&batches);
    assert_eq!(walked.len(), 4, "{walked:?}");
    assert_eq!(
        find(&walked, &root).subfolders,
        ["Also real", "Locked", "Open", "Real"]
    );
    assert_eq!(
        find(&walked, &root.join("Real")).subfolders,
        ["Back to the top"]
    );
    assert!(walked.iter().all(|w| !w.folder.ends_with("Also real")));
    assert_eq!(find(&walked, &root.join("Open")).raws, 1);
    if !readable {
        let locked = find(&walked, &locked);
        assert!(locked.unreadable.is_some(), "{locked:?}");
        assert_eq!((locked.raws, locked.subfolders.len()), (0, 0));
    }
}

/// Cancelled, as a new root or a close cancels its job, it sends nothing more; and a root
/// that is no absolute folder is not walked.
#[test]
fn a_cancelled_walk_sends_nothing_more_and_a_relative_root_none() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().canonicalize().unwrap();
    for day in 0..3 * BATCH {
        raws(&root.join(format!("Day {day}")), &["a.ARW"]);
    }
    let (batches, ended) = walk(&root, &|| true);
    assert_eq!((batches.len(), ended), (0, Ended::Cancelled));

    // Cancelled once the first batch is out.
    let sent = AtomicBool::new(false);
    let heard = Mutex::new(Vec::new());
    let ended = ops::walk(
        5,
        &root,
        &|event| {
            heard.lock().unwrap().push(event);
            sent.store(true, Ordering::Relaxed);
        },
        &|| sent.load(Ordering::Relaxed),
    );
    assert_eq!(ended, Ended::Cancelled);
    assert_eq!(heard.into_inner().unwrap().len(), 1);

    let (batches, ended) = walk(Path::new("relative"), &|| false);
    assert_eq!((batches.len(), ended), (0, Ended::Failed));
}

/// Finder's hidden flag hides a folder from the walk and from the listing alike, and a Mac
/// package is no folder to either: one rule for which folders exist.
#[cfg(target_os = "macos")]
#[test]
fn a_folder_finder_hides_and_a_package_are_no_folders_to_the_walk_or_the_listing() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().canonicalize().unwrap();
    raws(&root.join("Shown"), &["a.ARW"]);
    raws(&root.join("Flagged"), &["b.ARW"]);
    raws(&root.join("Holiday.photoslibrary"), &["c.ARW"]);
    let flagged = std::process::Command::new("chflags")
        .arg("hidden")
        .arg(root.join("Flagged"))
        .status()
        .unwrap();
    assert!(flagged.success());

    let (batches, ended) = walk(&root, &|| false);
    assert_eq!(ended, Ended::Listed);
    let walked = folders(&batches);
    assert_eq!(find(&walked, &root).subfolders, ["Shown"]);
    assert_eq!(walked.len(), 2, "{walked:?}");

    let options = apertrust_library::Options::default();
    let listed = apertrust_library::Listing::start(&root, options)
        .collect()
        .unwrap();
    assert_eq!(listed.subfolders, ["Shown"]);
}
