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

#[test]
fn persistence_recovers_after_an_initial_read_error() {
    let (_dir, file) = state("initial-read-error");
    // Reading a directory as the state file fails on every supported platform.
    std::fs::create_dir(&file).unwrap();
    let (mut recents, warnings) = Recents::open(file.clone(), none);
    assert!(!warnings.is_empty());
    recents.opened_at(OpenKind::Folder, folder("a"), 100);
    recents.flush();
    assert!(file.is_dir());
    std::fs::remove_dir(&file).unwrap();
    recents.flush();
    let (read, warnings) = Recents::open(file, none);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(paths(&read), vec![folder("a")]);
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
        recents.opened_at(OpenKind::Folder, folder(name), 100 + i as u64);
    }
    assert_eq!(paths(&recents), [folder("c"), folder("b"), folder("a")]);
    // Opened again, once, at the top.
    recents.opened_at(OpenKind::Folder, folder("a"), 200);
    assert_eq!(paths(&recents), [folder("a"), folder("c"), folder("b")]);
    for i in 0..12 {
        recents.opened_at(OpenKind::Folder, folder(&format!("n{i}")), 300 + i);
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
        recents.opened_at(OpenKind::Folder, folder(&format!("n{i}")), 1000 + i);
    }
    recents.opened_at(OpenKind::Folder, folder("late"), 5);
    assert_eq!(recents.entries().count(), MOST);
    let top = recents.entries().next().cloned();
    assert_eq!(
        top.map(|e| (e.path, e.opened)),
        Some((folder("late"), 1010))
    );

    let (_dir, file) = state("clock");
    let (mut one, _) = Recents::open(file.clone(), none);
    let (mut two, _) = Recents::open(file.clone(), none);
    // Each open writes the empty list it found, on its writer's thread: both done before
    // the first writes its ten, which the second's would otherwise write over.
    one.flush();
    two.flush();
    for i in 0..10 {
        one.opened_at(OpenKind::Folder, folder(&format!("n{i}")), 1000 + i);
    }
    one.flush();
    two.opened_at(OpenKind::Folder, folder("late"), 5);
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
    recents.opened_at(OpenKind::Folder, folder("a"), 100);
    recents.opened_at(OpenKind::Folder, folder("b"), 200);
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
    one.opened_at(OpenKind::Folder, folder("a"), 100);
    one.flush();
    two.opened_at(OpenKind::Folder, folder("b"), 200);
    two.flush();
    let (read, _) = Recents::open(file.clone(), none);
    assert_eq!(paths(&read), [folder("b"), folder("a")]);
    // The first removes a, which the second never saw; the second's next write keeps it gone.
    let (mut one, _) = Recents::open(file.clone(), none);
    one.remove(OpenKind::Folder, &folder("a"));
    one.flush();
    two.opened_at(OpenKind::Folder, folder("c"), 300);
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
    recents.opened_at(OpenKind::Folder, dir.0.clone(), 100);
    recents.opened_at(OpenKind::Folder, gone.clone(), 200);
    recents.check();
    // Nothing waits: what has not been answered is unmarked.
    let start = Instant::now();
    recents.take();
    assert!(start.elapsed() < Duration::from_secs(1));
    // A path still being asked is not asked again.
    recents.check();
    assert!(recents.asking.len() <= 2);
    answered(&mut recents);
    assert!(recents.missing(OpenKind::Folder, &gone));
    assert!(!recents.missing(OpenKind::Folder, &dir.0));
    // Back, as a share reconnected: the next check, on focus or as Collections shows,
    // unmarks it.
    std::fs::create_dir(&gone).expect("made");
    recents.check();
    answered(&mut recents);
    assert!(!recents.missing(OpenKind::Folder, &gone));
}

/// An answer to a question asked before its folder was opened again says nothing of it.
#[test]
fn a_late_answer_never_marks_a_folder_opened_since() {
    let (dir, _) = state("late");
    let gone = dir.0.join("gone");
    let mut recents = Recents::nowhere();
    recents.opened_at(OpenKind::Folder, gone.clone(), 100);
    recents.check();
    // Opened, as a folder made meanwhile would be, before the answer is taken.
    recents.opened_at(OpenKind::Folder, gone.clone(), 200);
    answered(&mut recents);
    assert!(!recents.missing(OpenKind::Folder, &gone));
}

#[test]
fn an_entry_is_removed_or_pointed_elsewhere_and_kept_so() {
    let (_dir, file) = state("remove");
    let (mut recents, _) = Recents::open(file.clone(), none);
    for (i, name) in ["a", "b", "c"].into_iter().enumerate() {
        recents.opened_at(OpenKind::Folder, folder(name), 100 + i as u64);
    }
    recents.mark(OpenKind::Folder, &folder("b"), false);
    assert!(recents.remove(OpenKind::Folder, &folder("c")));
    assert!(!recents.remove(OpenKind::Folder, &folder("z")));
    // Pointed at itself, it stays as it is.
    assert!(recents.repoint(OpenKind::Folder, &folder("a"), folder("a")));
    assert_eq!(paths(&recents), [folder("b"), folder("a")]);
    // Re-pointed, it keeps its place and is no longer missing.
    assert!(recents.repoint(OpenKind::Folder, &folder("b"), folder("b2")));
    assert_eq!(paths(&recents), [folder("b2"), folder("a")]);
    assert!(!recents.missing(OpenKind::Folder, &folder("b2")));
    // Onto an entry already there: one entry, in the re-pointed one's place.
    assert!(recents.repoint(OpenKind::Folder, &folder("b2"), folder("a")));
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
        recents.opened_at(OpenKind::Folder, folder("b"), 100);
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
    recents.opened_at(OpenKind::Folder, folder("a"), 100);
    recents.opened_at(OpenKind::Folder, folder("b"), 200);
    recents.flush();
    std::fs::write(&file, "not = [toml").expect("written");
    recents.opened_at(OpenKind::Folder, folder("c"), 300);
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
    recents.opened_at(OpenKind::Folder, folder("a"), 100);
    recents.flush();
    assert!(file.is_dir());
    std::fs::remove_dir(&file).expect("removed");
    recents.opened_at(OpenKind::Folder, folder("b"), 200);
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
    recents.opened_at(OpenKind::Folder, folder("a"), 100);
    recents.opened_at(OpenKind::Folder, folder("b"), 200);
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
    recents.opened_at(OpenKind::Folder, folder("a"), 100);
    recents.flush();
    assert_eq!(text(&file), newer);
}

/// What was open last goes through `state.toml` beside the list and back, its photo with
/// it, and goes once taken out; the list and the file's other keys stay as they are.
#[test]
fn what_was_open_last_is_kept_beside_the_list_and_taken_out() {
    let (_dir, file) = state("last");
    std::fs::write(&file, "version = 1\nother = 1\n").expect("written");
    let (mut recents, _) = Recents::open(file.clone(), none);
    assert_eq!(recents.last(), None);
    recents.opened_at(OpenKind::Folder, folder("a"), 100);
    let last = Last {
        kind: OpenKind::Folder,
        path: folder("a"),
        folder: Some(folder("a/Day 1")),
        photo: Some("IMG_2041.ARW".to_owned()),
    };
    recents.set_last(Some(last.clone()));
    recents.flush();
    let written = text(&file);
    let kept = "[last]\nfolder = \"/photos/a/Day 1\"\nkind = \"folder\"\npath = \"/photos/a\"\nphoto = \"IMG_2041.ARW\"\n";
    assert!(written.contains(kept), "{written}");
    assert!(written.contains("other = 1"), "{written}");
    let (mut again, _) = Recents::open(file.clone(), none);
    assert_eq!(again.last(), Some(&last));
    // The root itself active, with no photo active, has neither.
    let bare = Last {
        folder: None,
        photo: None,
        ..last
    };
    again.set_last(Some(bare.clone()));
    again.flush();
    assert_eq!(Recents::open(file.clone(), none).0.last(), Some(&bare));
    // Taken out, as File › Close does: the list stays.
    again.set_last(None);
    again.flush();
    let (read, _) = Recents::open(file.clone(), none);
    assert_eq!(read.last(), None);
    assert_eq!(paths(&read), [folder("a")]);
    assert!(!text(&file).contains("[last]"), "{}", text(&file));
}

/// Two Apertrusts on one folder: what was open last is whichever set it last, and the
/// other's list is kept with it.
#[test]
fn what_was_open_last_is_the_last_set_of_two_instances() {
    let (_dir, file) = state("last-two");
    let (mut one, _) = Recents::open(file.clone(), none);
    let (mut two, _) = Recents::open(file.clone(), none);
    let last = |name: &str| Last {
        kind: OpenKind::Folder,
        path: folder(name),
        folder: None,
        photo: None,
    };
    one.set_last(Some(last("a")));
    one.flush();
    two.set_last(Some(last("b")));
    two.flush();
    one.opened_at(OpenKind::Folder, folder("c"), 100);
    one.flush();
    let (read, _) = Recents::open(file, none);
    assert_eq!(read.last(), Some(&last("b")));
    assert_eq!(paths(&read), [folder("c")]);
}

/// A folder's label, its open kind and its name, with no marks around the name, and the
/// kind's own name, in each language.
#[test]
fn a_folder_s_label_and_its_kind_s_name_in_each_language() {
    use apertrust_ui::i18n::Locale;
    let name = "2026-10 Lisbon";
    for (locale, label, kind) in [
        (Locale::EnGb, "Folder: 2026-10 Lisbon", "Folder"),
        (Locale::EnUs, "Folder: 2026-10 Lisbon", "Folder"),
        (Locale::Uk, "Тека: 2026-10 Lisbon", "Тека"),
        (Locale::Pl, "Folder: 2026-10 Lisbon", "Folder"),
    ] {
        let words = Translator::new(locale);
        assert_eq!(OpenKind::Folder.label(&words, name), label, "{locale:?}");
        assert_eq!(OpenKind::Folder.name(&words), kind, "{locale:?}");
    }
}

/// The tab last shown in each workspace is kept by workspace id beside the list, merged
/// into the file as it then is, so a tab another process set for another workspace stays.
#[test]
fn the_tab_last_shown_is_kept_by_workspace() {
    let (_dir, file) = state("tabs");
    let (mut recents, _) = Recents::open(file.clone(), none);
    assert_eq!(recents.tab("default"), None);
    recents.set_tab("default", "effects");
    recents.flush();
    // Meanwhile, outside this process: another workspace's tab, and a value it cannot read.
    let outside = text(&file).replace("[tab]\n", "[tab]\nportraits = \"skin\"\nodd = 4\n");
    std::fs::write(&file, outside).expect("written");
    recents.set_tab("default", "raw");
    recents.flush();
    let (read, warnings) = Recents::open(file.clone(), none);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(read.tab("default"), Some("raw"));
    assert_eq!(read.tab("portraits"), Some("skin"));
    assert_eq!(read.tab("odd"), None);
    assert!(text(&file).contains("odd = 4"));
}

/// The old settings' tab moves once, where the file holds none for the workspace, written
/// at once; a tab the file holds wins, and no old tab moves nothing.
#[test]
fn the_old_tab_moves_once() {
    let (_dir, file) = state("old-tab");
    let (mut recents, _) = Recents::open(file.clone(), none);
    recents.move_tab("default", || None);
    recents.flush();
    assert!(!text(&file).contains("[tab]"));
    recents.move_tab("default", || Some("geometry".to_owned()));
    drop(recents);
    let (mut read, _) = Recents::open(file.clone(), none);
    assert_eq!(read.tab("default"), Some("geometry"));
    read.move_tab("default", || panic!("read again once moved"));
    assert_eq!(read.tab("default"), Some("geometry"));
}

/// A file damaged after the start, met by a later write, keeps the tabs the last write
/// left, as it keeps its entries, those this process read at the start too.
#[test]
fn damage_met_by_a_later_write_keeps_the_last_write_s_tabs() {
    let (_dir, file) = state("damaged-tabs");
    std::fs::write(&file, "version = 1\n[tab]\nportraits = \"skin\"\n").expect("written");
    let (mut recents, _) = Recents::open(file.clone(), none);
    recents.set_tab("default", "raw");
    recents.flush();
    std::fs::write(&file, "not = [toml").expect("written");
    recents.opened_at(OpenKind::Folder, folder("a"), 100);
    recents.flush();
    let (read, _) = Recents::open(file, none);
    assert_eq!(paths(&read), [folder("a")]);
    assert_eq!(read.tab("default"), Some("raw"));
    assert_eq!(read.tab("portraits"), Some("skin"));
}

/// The old tab's move never replaces a tab another process wrote for the workspace after
/// this one read the file; a tab the user chooses does.
#[test]
fn the_old_tab_s_move_keeps_another_process_s_tab() {
    let (_dir, file) = state("old-tab-race");
    let (mut recents, _) = Recents::open(file.clone(), none);
    recents.flush();
    let outside = text(&file) + "\n[tab]\ndefault = \"effects\"\n";
    std::fs::write(&file, outside).expect("written");
    recents.move_tab("default", || Some("geometry".to_owned()));
    recents.flush();
    assert_eq!(
        Recents::open(file.clone(), none).0.tab("default"),
        Some("effects")
    );
    recents.set_tab("default", "raw");
    recents.flush();
    assert_eq!(Recents::open(file, none).0.tab("default"), Some("raw"));
}

/// A tab the user chooses is written though the old setting's move gave the same one,
/// which another process's tab kept out of the file.
#[test]
fn a_chosen_tab_is_written_though_the_move_gave_it() {
    let (_dir, file) = state("old-tab-chosen");
    let (mut recents, _) = Recents::open(file.clone(), none);
    recents.flush();
    let outside = text(&file) + "\n[tab]\ndefault = \"effects\"\n";
    std::fs::write(&file, outside).expect("written");
    recents.move_tab("default", || Some("geometry".to_owned()));
    recents.flush();
    assert_eq!(recents.tab("default"), Some("geometry"));
    recents.set_tab("default", "geometry");
    recents.flush();
    assert_eq!(Recents::open(file, none).0.tab("default"), Some("geometry"));
}

/// A merge that finds no file, as the retry after a damaged file was set aside and its
/// replacement could not be written, starts from what the last write left, its list and
/// its tabs, not from nothing.
#[test]
fn a_missing_file_is_written_from_the_last_write() {
    let (_dir, file) = state("missing-retry");
    let last = Written {
        items: vec![Item::Known(Entry {
            kind: OpenKind::Folder,
            path: folder("a"),
            opened: 100,
        })],
        tabs: BTreeMap::from([("default".to_owned(), "raw".to_owned())]),
    };
    let entry = Entry {
        kind: OpenKind::Folder,
        path: folder("b"),
        opened: 200,
    };
    let written = merge(&file, &[Change::Opened(entry)], &last).expect("written");
    assert_eq!(written.items.len(), 2);
    let (read, _) = Recents::open(file, none);
    assert_eq!(paths(&read), [folder("b"), folder("a")]);
    assert_eq!(read.tab("default"), Some("raw"));
}
