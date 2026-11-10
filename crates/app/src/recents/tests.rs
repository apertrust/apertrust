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

use std::time::{Duration, Instant};

use super::*;
use crate::settings::testing::Folder;

fn paths(recents: &Recents) -> Vec<PathBuf> {
    recents.entries().map(|e| e.path.clone()).collect()
}

fn folder(name: &str) -> PathBuf {
    PathBuf::from(format!("/photos/{name}"))
}

/// A user data folder of a test's own, and its `state.toml`.
fn state(name: &str) -> (Folder, PathBuf) {
    let dir = Folder::new(&format!("recents-{name}"));
    let file = dir.0.join("state.toml");
    (dir, file)
}

fn none() -> Vec<PathBuf> {
    Vec::new()
}

fn text(file: &Path) -> String {
    std::fs::read_to_string(file).expect("written")
}

/// Takes the checks' answers until none is still to come, for at most ten seconds.
fn answered(recents: &mut Recents) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !recents.asking.is_empty() && Instant::now() < deadline {
        recents.take();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(recents.asking.is_empty(), "a check never answered");
}

#[test]
fn the_newest_comes_first_an_opened_one_moves_to_the_top_and_ten_are_kept() {
    let mut recents = Recents::nowhere();
    for (i, name) in ["a", "b", "c"].into_iter().enumerate() {
        recents.opened_at(Kind::Folder, folder(name), 100 + i as u64);
    }
    assert_eq!(paths(&recents), [folder("c"), folder("b"), folder("a")]);
    // Opened again, once, at the top.
    recents.opened_at(Kind::Folder, folder("a"), 200);
    assert_eq!(paths(&recents), [folder("a"), folder("c"), folder("b")]);
    for i in 0..12 {
        recents.opened_at(Kind::Folder, folder(&format!("n{i}")), 300 + i);
    }
    assert_eq!(recents.entries().count(), MOST);
    assert_eq!(paths(&recents)[0], folder("n11"));
    assert!(!paths(&recents).contains(&folder("a")));
}

/// A clock set back never puts an opened folder last, where a full list would cut it off:
/// in memory, nor in the file another Apertrust filled.
#[test]
fn a_folder_opened_after_the_clock_was_set_back_still_goes_to_the_top() {
    let mut recents = Recents::nowhere();
    for i in 0..10 {
        recents.opened_at(Kind::Folder, folder(&format!("n{i}")), 1000 + i);
    }
    recents.opened_at(Kind::Folder, folder("late"), 5);
    assert_eq!(recents.entries().count(), MOST);
    let top = recents.entries().next().cloned();
    assert_eq!(
        top.map(|e| (e.path, e.opened)),
        Some((folder("late"), 1010))
    );

    let (_dir, file) = state("clock");
    let (mut one, _) = Recents::open(file.clone(), none);
    let (mut two, _) = Recents::open(file.clone(), none);
    for i in 0..10 {
        one.opened_at(Kind::Folder, folder(&format!("n{i}")), 1000 + i);
    }
    one.flush();
    two.opened_at(Kind::Folder, folder("late"), 5);
    two.flush();
    let (read, _) = Recents::open(file, none);
    assert_eq!(read.entries().count(), MOST);
    assert_eq!(paths(&read)[0], folder("late"));
}

#[test]
fn the_list_goes_through_state_toml_and_back() {
    let (_dir, file) = state("round-trip");
    let (mut recents, warnings) = Recents::open(file.clone(), none);
    assert!(warnings.is_empty(), "{warnings:?}");
    recents.opened_at(Kind::Folder, folder("a"), 100);
    recents.opened_at(Kind::Folder, folder("b"), 200);
    recents.flush();
    let expected = "version = 1\n\n\
        [[recent]]\nkind = \"folder\"\nopened = 200\npath = \"/photos/b\"\n\n\
        [[recent]]\nkind = \"folder\"\nopened = 100\npath = \"/photos/a\"\n";
    assert_eq!(text(&file), expected);
    let (again, _) = Recents::open(file, none);
    let entries = |r: &Recents| r.entries().cloned().collect::<Vec<_>>();
    assert_eq!(entries(&again), entries(&recents));
}

/// Two Apertrusts on one folder each merge what they changed into the file as it is, so
/// neither loses the other's entries, and one's removal is not undone by the other.
#[test]
fn two_instances_keep_each_other_s_entries() {
    let (_dir, file) = state("two");
    let (mut one, _) = Recents::open(file.clone(), none);
    let (mut two, _) = Recents::open(file.clone(), none);
    one.flush();
    two.flush();
    one.opened_at(Kind::Folder, folder("a"), 100);
    one.flush();
    two.opened_at(Kind::Folder, folder("b"), 200);
    two.flush();
    let (read, _) = Recents::open(file.clone(), none);
    assert_eq!(paths(&read), [folder("b"), folder("a")]);
    // The first removes a, which the second never saw; the second's next write keeps it gone.
    let (mut one, _) = Recents::open(file.clone(), none);
    one.remove(Kind::Folder, &folder("a"));
    one.flush();
    two.opened_at(Kind::Folder, folder("c"), 300);
    two.flush();
    let (read, _) = Recents::open(file, none);
    assert_eq!(paths(&read), [folder("c"), folder("b")]);
}

/// Settings' `library.recent_folders` becomes the list where `state.toml` has none, once:
/// written at once, and never read again, even after the list is cleared.
#[test]
fn the_old_setting_moves_once() {
    let (_dir, file) = state("old");
    let (mut recents, _) = Recents::open(file.clone(), || vec![folder("b"), folder("a")]);
    assert_eq!(paths(&recents), [folder("b"), folder("a")]);
    recents.flush();
    let (again, _) = Recents::open(file.clone(), || vec![folder("c")]);
    assert_eq!(paths(&again), [folder("b"), folder("a")]);
    let (mut cleared, _) = Recents::open(file.clone(), none);
    cleared.clear();
    cleared.flush();
    let (read, _) = Recents::open(file, || vec![folder("c")]);
    assert_eq!(read.entries().count(), 0);
    // An empty old list moves too, as an empty list.
    let (_other, file) = state("old-empty");
    let (mut empty, _) = Recents::open(file.clone(), none);
    empty.flush();
    assert!(text(&file).contains("recent = []"));
}

#[test]
fn a_missing_folder_is_marked_off_the_caller_s_thread_and_unmarked_once_it_is_back() {
    let (dir, _) = state("missing");
    let gone = dir.0.join("gone");
    let mut recents = Recents::nowhere();
    recents.opened_at(Kind::Folder, dir.0.clone(), 100);
    recents.opened_at(Kind::Folder, gone.clone(), 200);
    recents.check();
    // Nothing waits: what has not been answered is unmarked.
    let start = Instant::now();
    recents.take();
    assert!(start.elapsed() < Duration::from_secs(1));
    // A path still being asked is not asked again.
    recents.check();
    assert!(recents.asking.len() <= 2);
    answered(&mut recents);
    assert!(recents.missing(Kind::Folder, &gone));
    assert!(!recents.missing(Kind::Folder, &dir.0));
    // Back, as a share reconnected: the next check, on focus or as Browse shows, unmarks it.
    std::fs::create_dir(&gone).expect("made");
    recents.check();
    answered(&mut recents);
    assert!(!recents.missing(Kind::Folder, &gone));
}

/// An answer to a question asked before its folder was opened again says nothing of it.
#[test]
fn a_late_answer_never_marks_a_folder_opened_since() {
    let (dir, _) = state("late");
    let gone = dir.0.join("gone");
    let mut recents = Recents::nowhere();
    recents.opened_at(Kind::Folder, gone.clone(), 100);
    recents.check();
    // Opened, as a folder made meanwhile would be, before the answer is taken.
    recents.opened_at(Kind::Folder, gone.clone(), 200);
    answered(&mut recents);
    assert!(!recents.missing(Kind::Folder, &gone));
}

#[test]
fn an_entry_is_removed_or_pointed_elsewhere_and_kept_so() {
    let (_dir, file) = state("remove");
    let (mut recents, _) = Recents::open(file.clone(), none);
    for (i, name) in ["a", "b", "c"].into_iter().enumerate() {
        recents.opened_at(Kind::Folder, folder(name), 100 + i as u64);
    }
    recents.mark(Kind::Folder, &folder("b"), false);
    assert!(recents.remove(Kind::Folder, &folder("c")));
    assert!(!recents.remove(Kind::Folder, &folder("z")));
    // Pointed at itself, it stays as it is.
    assert!(recents.repoint(Kind::Folder, &folder("a"), folder("a")));
    assert_eq!(paths(&recents), [folder("b"), folder("a")]);
    // Re-pointed, it keeps its place and is no longer missing.
    assert!(recents.repoint(Kind::Folder, &folder("b"), folder("b2")));
    assert_eq!(paths(&recents), [folder("b2"), folder("a")]);
    assert!(!recents.missing(Kind::Folder, &folder("b2")));
    // Onto an entry already there: one entry, in the re-pointed one's place.
    assert!(recents.repoint(Kind::Folder, &folder("b2"), folder("a")));
    assert_eq!(paths(&recents), [folder("a")]);
    recents.flush();
    let (read, _) = Recents::open(file, none);
    assert_eq!(paths(&read), [folder("a")]);
}

/// A damaged file, not TOML or not UTF-8, is set aside, the list starting empty, and its
/// warning quotes nothing of it: no photo's folder reaches the log.
#[test]
fn a_damaged_state_toml_is_set_aside_and_the_list_starts_empty() {
    for (name, bytes) in [
        (
            "toml",
            b"[[recent]]\npath = \"/photos/Secret\" oops\n".to_vec(),
        ),
        (
            "utf-8",
            b"[[recent]]\npath = \"/photos/Secret\xff\"\n".to_vec(),
        ),
    ] {
        let (dir, file) = state(&format!("damaged-{name}"));
        std::fs::write(&file, bytes).expect("written");
        let (mut recents, warnings) = Recents::open(file.clone(), || vec![folder("a")]);
        assert_eq!(warnings.len(), 1, "{name}: {warnings:?}");
        assert!(warnings[0].starts_with("damaged"), "{name}: {warnings:?}");
        assert!(!warnings[0].contains("Secret"), "{name}: {warnings:?}");
        assert_eq!(recents.entries().count(), 0, "{name}");
        assert!(dir.0.join("state.toml.bak").exists(), "{name}");
        recents.opened_at(Kind::Folder, folder("b"), 100);
        recents.flush();
        let (read, warnings) = Recents::open(file, none);
        assert!(warnings.is_empty(), "{name}: {warnings:?}");
        assert_eq!(paths(&read), [folder("b")], "{name}");
    }
}

/// A file damaged after the start, met by a later write, is set aside, and what the last
/// write left is written with the change: this Apertrust's other entries stay.
#[test]
fn damage_met_by_a_later_write_keeps_the_last_write_s_entries() {
    let (dir, file) = state("damaged-later");
    let (mut recents, _) = Recents::open(file.clone(), none);
    recents.opened_at(Kind::Folder, folder("a"), 100);
    recents.opened_at(Kind::Folder, folder("b"), 200);
    recents.flush();
    std::fs::write(&file, "not = [toml").expect("written");
    recents.opened_at(Kind::Folder, folder("c"), 300);
    recents.flush();
    assert!(dir.0.join("state.toml.bak").exists());
    let (read, _) = Recents::open(file, none);
    assert_eq!(paths(&read), [folder("c"), folder("b"), folder("a")]);
}

/// A write that fails keeps its changes, which the next write, or the one on exit, merges.
#[test]
fn a_write_that_failed_is_tried_again() {
    let (_dir, file) = state("retry");
    let (mut recents, _) = Recents::open(file.clone(), none);
    recents.flush();
    // A folder in the file's place: it can be neither read nor written.
    std::fs::remove_file(&file).expect("removed");
    std::fs::create_dir(&file).expect("made");
    recents.opened_at(Kind::Folder, folder("a"), 100);
    recents.flush();
    assert!(file.is_dir());
    std::fs::remove_dir(&file).expect("removed");
    recents.opened_at(Kind::Folder, folder("b"), 200);
    recents.flush();
    let (read, _) = Recents::open(file, none);
    assert_eq!(paths(&read), [folder("b"), folder("a")]);
}

/// An entry this build cannot read, of a kind it lacks or malformed, stays in the file, and
/// so do the file's other keys; a file a newer version wrote is never written.
#[test]
fn what_this_build_cannot_read_is_kept() {
    let (_dir, file) = state("unread");
    let text_kept = "version = 1\nlast = \"kept\"\n\n\
        [[recent]]\nkind = \"library\"\npath = \"/photos/Family.aplibrary\"\nopened = 150\n\n\
        [[recent]]\nkind = \"folder\"\npath = \"/photos/Odd\"\nopened = \"yesterday\"\n";
    std::fs::write(&file, text_kept).expect("written");
    let (mut recents, warnings) = Recents::open(file.clone(), none);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(recents.entries().count(), 0);
    recents.opened_at(Kind::Folder, folder("a"), 100);
    recents.opened_at(Kind::Folder, folder("b"), 200);
    recents.flush();
    let written = text(&file);
    assert!(written.contains("last = \"kept\""), "{written}");
    assert!(written.contains("/photos/Odd"), "{written}");
    // Each folder opened goes to the top, above the library of 150.
    let at = |text: &str| written.find(text).expect(text);
    assert!(at("/photos/b") < at("/photos/a"), "{written}");
    assert!(at("/photos/a") < at("Family.aplibrary"), "{written}");

    let newer = "version = 2\n\n[[recent]]\nkind = \"folder\"\npath = \"/photos/x\"\nopened = 1\n";
    std::fs::write(&file, newer).expect("written");
    let (mut recents, warnings) = Recents::open(file.clone(), none);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    recents.opened_at(Kind::Folder, folder("a"), 100);
    recents.flush();
    assert_eq!(text(&file), newer);
}
