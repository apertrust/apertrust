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

//! The Toolbar's customise mode's draft (decision 92; `docs/gui.md`, The customise mode):
//! the list as the person arranges it, the labels, whether the list is Reset's, and the
//! item selected, seeded from the workspace's list and `toolbar.labels` as the mode starts.
//! Nothing is written until Done, which writes it once ([`CustomiseToolbar::written`]);
//! Esc, closing the window and Quit drop it.

use crate::layout::Move;
use crate::toolbar::{self, Kind};

/// The draft the customise mode arranges.
#[derive(Clone, Debug, PartialEq)]
pub struct CustomiseToolbar {
    /// The workspace's list as the mode started, ids this build does not know among them,
    /// which Done merges back ([`toolbar::merged`]).
    start: Vec<String>,
    /// What the list drew as the mode started, and this build's default for the workspace.
    started: Vec<&'static str>,
    default: Vec<&'static str>,
    /// Whether the workspace's file held a list as the mode started.
    kept: bool,
    /// The list as arranged: ids the set holds, each part and action once.
    list: Vec<&'static str>,
    /// The labels, and as they were.
    labels: bool,
    labels_start: bool,
    /// Whether Reset ran: the list is then Reset's while it is the default.
    reset: bool,
    /// The item selected, by its place in the list.
    selected: Option<usize>,
    /// When the mode started, in egui's input time, for the wiggle and the scrim's fade,
    /// and whether the system asked for reduced motion then, so neither moves.
    pub started_at: f64,
    pub still: bool,
}

/// What Done writes ([`CustomiseToolbar::written`]): a change to the workspace's list, if
/// any, and the labels, if they were flipped.
#[derive(Clone, Debug, PartialEq)]
pub struct Written {
    pub list: Option<Move>,
    pub labels: Option<bool>,
}

impl CustomiseToolbar {
    /// The draft of the list `start`, the workspace's as shown, whose default is `default`,
    /// `kept` saying whether its file holds a list, with the labels `labels`, `item` the one
    /// the toolbar menu opened on, if any, selected, at `now`, `still` where the system asks
    /// for reduced motion.
    pub fn new(
        start: &[String],
        default: &[String],
        kept: bool,
        labels: bool,
        item: Option<&str>,
        now: f64,
        still: bool,
    ) -> Self {
        let drawn = |list: &[String]| -> Vec<&'static str> {
            toolbar::listed(list.iter().map(String::as_str))
                .iter()
                .map(|placed| placed.id)
                .collect()
        };
        let list = drawn(start);
        let selected = item.and_then(|item| list.iter().position(|id| *id == item));
        CustomiseToolbar {
            start: start.to_vec(),
            started: list.clone(),
            default: drawn(default),
            kept,
            list,
            labels,
            labels_start: labels,
            reset: false,
            selected,
            started_at: now,
            still,
        }
    }

    /// The list as arranged, by id.
    pub fn list(&self) -> &[&'static str] {
        &self.list
    }

    pub fn labels(&self) -> bool {
        self.labels
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected.filter(|at| *at < self.list.len())
    }

    /// Selects the item at `at`, or nothing.
    pub fn select(&mut self, at: Option<usize>) {
        self.selected = at.filter(|at| *at < self.list.len());
    }

    /// Moves the item at `from` so it stands at `to`, and selects it.
    pub fn move_item(&mut self, from: usize, to: usize) -> bool {
        if from >= self.list.len() || to >= self.list.len() || from == to {
            return false;
        }
        let id = self.list.remove(from);
        self.list.insert(to, id);
        self.selected = Some(to);
        true
    }

    /// Takes the item at `at` out, the next selected, or the one before at the end.
    pub fn remove(&mut self, at: usize) -> Option<&'static str> {
        if at >= self.list.len() {
            return None;
        }
        let id = self.list.remove(at);
        self.selected = match self.list.len() {
            0 => None,
            len => Some(at.min(len - 1)),
        };
        Some(id)
    }

    /// Adds `id` at `at`, and selects it: where the list holds it already, a part or an
    /// action, nothing.
    pub fn add(&mut self, id: &'static str, at: usize) -> Option<usize> {
        let spacing = toolbar::set()
            .iter()
            .any(|placed| placed.id == id && placed.kind == Kind::Spacing);
        if !spacing && self.list.contains(&id) {
            return None;
        }
        let at = at.min(self.list.len());
        self.list.insert(at, id);
        self.selected = Some(at);
        Some(at)
    }

    /// Makes the list this build's default, marked as Reset's; the labels are untouched.
    /// The ids this build does not know go with the list, never merged back.
    pub fn reset(&mut self) {
        self.list.clone_from(&self.default);
        self.start.clear();
        self.selected = None;
        self.reset = true;
    }

    /// Whether the list is Reset's: Reset ran, and the list is the default, though moved
    /// away from it and back.
    fn resets(&self) -> bool {
        self.reset && self.list == self.default
    }

    /// Flips the labels.
    pub fn flip_labels(&mut self) {
        self.labels = !self.labels;
    }

    /// Whether Reset Toolbar can run: the list differs from the default, or the workspace's
    /// file held a list as the mode started and the draft is not Reset's.
    pub fn resettable(&self) -> bool {
        self.list != self.default || (self.kept && !self.resets())
    }

    /// What Done writes: nothing for a list unchanged, as a file with no list keeps
    /// following each build's default; `Move::ResetToolbar` for a Reset with the list still
    /// the default; otherwise the whole list, the ids this build does not know merged back. The
    /// labels only where they were flipped.
    pub fn written(&self) -> Written {
        let list = if self.resets() {
            self.kept.then_some(Move::ResetToolbar)
        } else if !self.reset && self.list == self.started {
            None
        } else {
            Some(Move::Toolbar(toolbar::merged(&self.start, &self.list)))
        };
        Written {
            list,
            labels: (self.labels != self.labels_start).then_some(self.labels),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| (*id).to_owned()).collect()
    }

    const DEFAULT: [&str; 4] = ["import", "undo", "flexible-space", "guides"];

    fn draft(start: &[&str], kept: bool) -> CustomiseToolbar {
        CustomiseToolbar::new(
            &strings(start),
            &strings(&DEFAULT),
            kept,
            true,
            None,
            0.0,
            false,
        )
    }

    /// Done with nothing changed writes nothing, a list a file never held included.
    #[test]
    fn nothing_changed_writes_nothing() {
        let draft = draft(&DEFAULT, false);
        assert_eq!(
            draft.written(),
            Written {
                list: None,
                labels: None
            }
        );
    }

    /// The menu opened on an item starts with it selected.
    #[test]
    fn the_item_asked_on_starts_selected() {
        let start = strings(&DEFAULT);
        let draft = CustomiseToolbar::new(&start, &start, false, true, Some("undo"), 0.0, false);
        assert_eq!(draft.selected(), Some(1));
    }

    /// A move, an add and a remove change the list, which Done writes whole.
    #[test]
    fn moves_adds_and_removes_are_written_whole() {
        let mut draft = draft(&DEFAULT, false);
        assert!(draft.move_item(1, 0));
        assert_eq!(draft.selected(), Some(0));
        assert_eq!(draft.add("open", 1), Some(1));
        assert_eq!(draft.add("open", 3), None, "a part or an action once");
        assert_eq!(draft.add("space", 0), Some(0));
        assert_eq!(
            draft.add("space", 0),
            Some(0),
            "a spacing any number of times"
        );
        assert_eq!(draft.remove(6), Some("guides"));
        assert_eq!(draft.selected(), Some(5), "the one before at the end");
        assert_eq!(draft.remove(0), Some("space"));
        assert_eq!(draft.selected(), Some(0), "the next");
        let list = ["space", "undo", "open", "import", "flexible-space"];
        assert_eq!(draft.list(), list);
        assert_eq!(draft.written().list, Some(Move::Toolbar(strings(&list))));
    }

    /// Reset then Done takes the list out of a file that held one, also where a change after
    /// it was undone by hand; Reset then a change writes a list, the ids this build does not
    /// know gone with the Reset; Reset on a file with none and nothing after writes nothing.
    #[test]
    fn reset_writes_reset_toolbar_unless_something_changed_after() {
        let mut held = draft(&["undo", "import"], true);
        assert!(held.resettable());
        held.reset();
        assert_eq!(held.list(), DEFAULT);
        assert!(!held.resettable());
        assert_eq!(held.written().list, Some(Move::ResetToolbar));
        held.move_item(0, 1);
        assert!(matches!(held.written().list, Some(Move::Toolbar(_))));
        // Moved back: Reset's again.
        held.move_item(1, 0);
        assert_eq!(held.written().list, Some(Move::ResetToolbar));
        let mut unknown = draft(&["newer-thing", "undo"], true);
        unknown.reset();
        unknown.remove(0);
        let Some(Move::Toolbar(list)) = unknown.written().list else {
            panic!("a list");
        };
        assert!(!list.iter().any(|id| id == "newer-thing"), "{list:?}");
        // Reset, then the order put back as it started: the unknown ids still go.
        let mut back = draft(&["undo", "newer-thing", "import"], true);
        back.reset();
        back.move_item(0, 1);
        back.remove(2);
        back.remove(2);
        assert_eq!(back.list(), ["undo", "import"]);
        let expected = Move::Toolbar(strings(&["undo", "import"]));
        assert_eq!(back.written().list, Some(expected));
        let mut none = draft(&["undo"], false);
        none.reset();
        assert_eq!(none.written().list, None);
    }

    /// The labels are written only where they were flipped.
    #[test]
    fn the_labels_are_written_only_when_flipped() {
        let mut draft = draft(&DEFAULT, false);
        draft.flip_labels();
        assert_eq!(draft.written().labels, Some(false));
        draft.flip_labels();
        assert_eq!(draft.written().labels, None);
    }

    /// The unknown ids of the list as it started are written back after their anchors;
    /// a repeat is drawn and written once; an old `menu-button` is dropped.
    #[test]
    fn unknown_ids_come_back_and_menu_button_goes() {
        let start = [
            "menu-button",
            "import",
            "newer-thing",
            "undo",
            "undo",
            "guides",
        ];
        let mut draft = draft(&start, true);
        assert_eq!(draft.list(), ["import", "undo", "guides"]);
        draft.move_item(0, 2);
        let written = ["undo", "guides", "import", "newer-thing"];
        assert_eq!(draft.written().list, Some(Move::Toolbar(strings(&written))));
    }
}
