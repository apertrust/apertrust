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

//! What the Browser's gestures do to the selection and the active photo, both the
//! session's (decision 37; `docs/gui.md`, Selection and the active photo, The pointer):
//!
//! - **A click,** or Enter, selects the photo alone and makes it active, opening it.
//! - **Ctrl-click** (Cmd on the Mac), or Ctrl+Space on the cell focused, adds the photo to
//!   the selection, the active photo staying as it is, or removes it. Removing the active
//!   photo closes it, as File › Close does, and
//!   makes the nearest selected photo in the Browser's order active, the later of two as
//!   near, or none.
//! - **Shift-click** selects the range from the active photo to it in the Browser's sort,
//!   the active photo staying.
//!
//! Each command is shown at once in the mirror's selection, by the session's rules, until
//! the session tells one that includes it ([`Mirror::selecting`]), so a gesture before that
//! reads what the last one did. Photos opened alone are no collection of the session's, so
//! there a click opens the photo and Ctrl and Shift-click do nothing.

use std::path::{Path, PathBuf};

use apertrust_engine::session::{Command, SelectionCommand};
use apertrust_ui::components::BrowserAction;

use crate::mirror::Mirror;
use crate::{SendCommand, open_listed};

/// Does what `action` asks of the Browser's photos, `order` their paths in its sort, which
/// its indices count; `alone` when they are photos opened alone.
pub(crate) fn browse(
    action: BrowserAction,
    order: &[PathBuf],
    alone: bool,
    mirror: &mut Mirror,
    send: &mut SendCommand<'_>,
) {
    let i = match action {
        BrowserAction::Open(i)
        | BrowserAction::SelectRange(i)
        | BrowserAction::ToggleSelected(i) => i,
    };
    let Some(path) = order.get(i).cloned() else {
        return;
    };
    let active = mirror.photo.asked().map(Path::to_owned);
    match action {
        BrowserAction::Open(_) => {
            // The photo open stays as it is when its own cell is pressed again.
            if active.as_ref() != Some(&path) {
                open_listed(path.clone(), mirror, send);
            }
            // Opening a selected photo keeps the selection, which a click narrows to it.
            if !alone && !mirror.selected_alone(&path) {
                select(SelectionCommand::Set(vec![path]), mirror, send);
            }
        }
        _ if alone => {}
        BrowserAction::SelectRange(_) => {
            let order = order.to_vec();
            select(SelectionCommand::Range { to: path, order }, mirror, send);
        }
        BrowserAction::ToggleSelected(_) if active.as_ref() == Some(&path) => {
            // The session closes the photo it takes out (`Mirror::closing`).
            select(SelectionCommand::Remove(path.clone()), mirror, send);
            mirror.closing();
            if let Some(next) = nearest(order, &path, |photo| mirror.is_selected(photo)) {
                open_listed(next, mirror, send);
            }
        }
        BrowserAction::ToggleSelected(_) if mirror.is_selected(&path) => {
            select(SelectionCommand::Remove(path), mirror, send);
        }
        BrowserAction::ToggleSelected(_) => {
            // Added to the selection; the active photo stays as it is.
            select(SelectionCommand::Add(path), mirror, send);
        }
    }
}

/// Sends `command`, shown in the mirror's selection until the session tells it.
fn select(command: SelectionCommand, mirror: &mut Mirror, send: &mut SendCommand<'_>) {
    let number = send(Command::Selection(command.clone()));
    mirror.selecting(&command, number);
}

/// The selected photo of `order` nearest `from`, the later of two as near, `from` itself
/// passed over; the first selected where `order` lacks `from`.
fn nearest(order: &[PathBuf], from: &Path, selected: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    let at = order.iter().position(|photo| photo == from).unwrap_or(0);
    order
        .iter()
        .enumerate()
        .filter(|(_, photo)| photo.as_path() != from && selected(photo))
        .min_by_key(|(j, _)| (j.abs_diff(at), *j < at))
        .map(|(_, photo)| photo.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mirror::PhotoState;
    use crate::mirror::tests::mirror;
    use apertrust_engine::session::{Event, Selection, SessionError};

    /// The folder `/photos` with photos `a` to `e`, in that order, as the Browser shows it,
    /// the mirror, and every command sent, numbered from 1 as the session numbers them.
    struct Folder {
        m: Mirror,
        order: Vec<PathBuf>,
        sent: Vec<Command>,
    }

    impl Folder {
        /// With `active` opened, the first command.
        fn new(active: &str) -> Self {
            let order = ["a", "b", "c", "d", "e"].map(at).to_vec();
            let mut folder = Folder {
                m: mirror(),
                order,
                sent: Vec::new(),
            };
            open_listed(at(active), &mut folder.m, &mut sender(&mut folder.sent));
            folder
        }

        /// What `action` sends in a folder.
        fn browse(&mut self, action: BrowserAction) -> Vec<Command> {
            self.browse_in(action, false)
        }

        /// What `action` sends, in a folder or among photos opened alone.
        fn browse_in(&mut self, action: BrowserAction, alone: bool) -> Vec<Command> {
            let from = self.sent.len();
            browse(
                action,
                &self.order,
                alone,
                &mut self.m,
                &mut sender(&mut self.sent),
            );
            self.sent[from..].to_vec()
        }

        /// Sends `command`, as the app does, returning its number.
        fn select(&mut self, command: SelectionCommand) -> u64 {
            self.sent.push(Command::Selection(command.clone()));
            let number = self.sent.len() as u64;
            self.m.selecting(&command, number);
            number
        }

        /// The session tells `photos` selected, `active` active, through command `through`.
        fn fact(&mut self, photos: &[&str], active: Option<&str>, through: u64) {
            self.m.apply(Event::Selection(Selection {
                photos: photos.iter().map(|name| at(name)).collect(),
                active: active.map(at),
                through,
            }));
        }

        /// The session tells `photos` selected, `active` active, through every command sent.
        fn told(&mut self, photos: &[&str], active: Option<&str>) {
            let through = self.sent.len() as u64;
            self.fact(photos, active, through);
        }

        /// The session refuses command `command`.
        fn refused(&mut self, command: u64, error: SessionError) {
            let error = Box::new(error);
            (self.m).apply(Event::Failed(SessionError::Refused { command, error }));
        }
    }

    fn at(name: &str) -> PathBuf {
        PathBuf::from(format!("/photos/{name}.ARW"))
    }

    fn sender(sent: &mut Vec<Command>) -> impl FnMut(Command) -> u64 + '_ {
        |command| {
            sent.push(command);
            sent.len() as u64
        }
    }

    fn selection(command: SelectionCommand) -> Command {
        Command::Selection(command)
    }

    fn not_listed(name: &str) -> SessionError {
        SessionError::NotInCollection { photo: at(name) }
    }

    /// A click opens the photo, selected alone: by the open where it was not selected, else
    /// by `Set`, its own cell's click too, which opens nothing; with it alone, nothing.
    #[test]
    fn a_click_opens_the_photo_and_selects_it_alone() {
        let mut f = Folder::new("a");
        f.told(&["a"], Some("a"));
        // c, not selected, is selected alone by its open.
        assert_eq!(f.browse(BrowserAction::Open(2)), [Command::Open(at("c"))]);
        assert_eq!(f.m.photo.asked(), Some(at("c").as_path()));
        assert!(f.m.selected_alone(&at("c")));
        f.told(&["b", "c", "d"], Some("c"));
        let alone = selection(SelectionCommand::Set(vec![at("d")]));
        let sent = f.browse(BrowserAction::Open(3));
        assert_eq!(sent, [Command::Open(at("d")), alone.clone()]);
        f.told(&["c", "d"], Some("d"));
        assert_eq!(f.browse(BrowserAction::Open(3)), [alone]);
        // The photo open, alone: nothing.
        f.told(&["d"], Some("d"));
        assert!(f.browse(BrowserAction::Open(3)).is_empty());
    }

    /// Ctrl-click adds a photo to the selection, the active photo staying as it is; on a
    /// selected photo not active it removes it, the active photo staying too.
    #[test]
    fn ctrl_click_adds_or_removes_the_photo_the_active_one_staying() {
        let mut f = Folder::new("a");
        f.told(&["a"], Some("a"));
        let add = selection(SelectionCommand::Add(at("c")));
        assert_eq!(f.browse(BrowserAction::ToggleSelected(2)), [add]);
        assert_eq!(f.m.photo.asked(), Some(at("a").as_path()));
        f.told(&["a", "c"], Some("a"));
        let remove = selection(SelectionCommand::Remove(at("c")));
        assert_eq!(f.browse(BrowserAction::ToggleSelected(2)), [remove]);
        assert_eq!(f.m.photo.asked(), Some(at("a").as_path()));
    }

    /// Ctrl-click on the active photo takes it out, closing it as File › Close does, and
    /// opens the nearest selected photo in the Browser's order, the later of two as near;
    /// with none left selected, no photo is active.
    #[test]
    fn ctrl_click_on_the_active_photo_makes_the_nearest_selected_one_active() {
        let remove = selection(SelectionCommand::Remove(at("c")));
        let mut f = Folder::new("c");
        f.told(&["a", "b", "c", "e"], Some("c"));
        let sent = f.browse(BrowserAction::ToggleSelected(2));
        assert_eq!(sent, [remove.clone(), Command::Open(at("b"))]);
        assert_eq!(f.m.photo.asked(), Some(at("b").as_path()));
        // b and d as near: the later.
        let mut f = Folder::new("c");
        f.told(&["b", "c", "d"], Some("c"));
        let sent = f.browse(BrowserAction::ToggleSelected(2));
        assert_eq!(sent[1], Command::Open(at("d")));
        // The only one selected: closed, none active.
        let mut f = Folder::new("c");
        f.told(&["c"], Some("c"));
        assert_eq!(f.browse(BrowserAction::ToggleSelected(2)), [remove]);
        assert!(matches!(f.m.photo, PhotoState::None));
    }

    /// Gestures before the session tells the selection read what the last ones did: a
    /// click on the active photo after a Ctrl-click narrows the selection to it, and a
    /// click on it again, selected alone, sends nothing.
    #[test]
    fn a_click_after_a_ctrl_click_not_yet_told_selects_the_photo_alone() {
        let mut f = Folder::new("a");
        f.told(&["a"], Some("a"));
        f.browse(BrowserAction::ToggleSelected(1));
        assert_eq!(f.m.selected_count(), 2);
        let alone = selection(SelectionCommand::Set(vec![at("a")]));
        assert_eq!(f.browse(BrowserAction::Open(0)), [alone]);
        assert!(f.m.selected_alone(&at("a")));
        assert!(f.browse(BrowserAction::Open(0)).is_empty());
    }

    /// Ctrl-click twice on a selected photo before the session tells either: the first
    /// takes it out, the second adds it back.
    #[test]
    fn ctrl_click_twice_before_the_selection_is_told_removes_then_adds() {
        let mut f = Folder::new("a");
        f.told(&["a", "b"], Some("a"));
        let remove = selection(SelectionCommand::Remove(at("b")));
        assert_eq!(f.browse(BrowserAction::ToggleSelected(1)), [remove]);
        assert!(!f.m.is_selected(&at("b")));
        let add = selection(SelectionCommand::Add(at("b")));
        assert_eq!(f.browse(BrowserAction::ToggleSelected(1)), [add]);
        assert_eq!(f.m.selected_count(), 2);
    }

    /// The nearest selected photo passes over one whose removal the session has not told.
    #[test]
    fn the_nearest_selected_photo_skips_a_removal_not_yet_told() {
        let mut f = Folder::new("c");
        f.told(&["a", "b", "c", "d"], Some("c"));
        f.browse(BrowserAction::ToggleSelected(3));
        let remove = selection(SelectionCommand::Remove(at("c")));
        let sent = f.browse(BrowserAction::ToggleSelected(2));
        assert_eq!(sent, [remove, Command::Open(at("b"))]);
    }

    /// The mirror lays the commands sent over each fact until one includes them; a refused
    /// command, a selection's, is taken back, another's leaves them, and one that leaves
    /// the selection as shown still waits for its own fact.
    #[test]
    fn the_selection_sent_shows_until_a_fact_includes_it() {
        let mut f = Folder::new("a");
        f.told(&["a"], Some("a"));
        // The Add, 2.
        f.browse(BrowserAction::ToggleSelected(1));
        assert_eq!(f.m.selected_count(), 2);
        // A fact through the open only, before the Add: still shown.
        f.fact(&["a"], Some("a"), 1);
        assert_eq!(f.m.selected_count(), 2);
        // Through the Add: the session's from now on, whatever it says.
        f.fact(&["a"], Some("a"), 2);
        assert!(f.m.selected_alone(&at("a")));
        // A Range refused: back to what the session told.
        let order = f.order.clone();
        let range = f.select(SelectionCommand::Range { to: at("e"), order });
        assert_eq!(f.m.selected_count(), 5);
        f.refused(range, not_listed("e"));
        assert!(f.m.selected_alone(&at("a")));
        // A command not of the selection refused leaves it.
        f.sent.push(Command::Close);
        let other = f.sent.len() as u64;
        f.select(SelectionCommand::Add(at("c")));
        f.refused(other, SessionError::NotLive);
        assert_eq!(f.m.selected_count(), 2);
        // Add(c) again, which changes nothing shown, waits for its own fact all the same.
        let again = f.select(SelectionCommand::Add(at("c")));
        f.fact(&["a"], Some("a"), again - 1);
        assert_eq!(f.m.selected_count(), 2);
        f.told(&["a", "c"], Some("a"));
        assert_eq!(f.m.selected_count(), 2);
    }

    /// A refused selection command takes back its own change alone: a later one sent before
    /// the session answered stays shown, laid over what the session told.
    #[test]
    fn a_refusal_takes_back_its_own_command_alone() {
        let mut f = Folder::new("a");
        f.told(&["a"], Some("a"));
        let order = f.order.clone();
        let range = f.select(SelectionCommand::Range { to: at("d"), order });
        let remove = f.select(SelectionCommand::Remove(at("b")));
        assert_eq!(f.m.selected_count(), 3);
        f.refused(range, not_listed("d"));
        // [a] told, the Remove of b laid over it.
        assert!(f.m.selected_alone(&at("a")));
        f.select(SelectionCommand::Set(vec![at("c")]));
        f.refused(remove, not_listed("b"));
        assert!(f.m.selected_alone(&at("c")));
    }

    /// An open that fails selects nothing: what it would have selected goes.
    #[test]
    fn a_failed_open_selects_nothing() {
        let mut f = Folder::new("a");
        f.told(&["a", "b"], Some("a"));
        open_listed(at("e"), &mut f.m, &mut sender(&mut f.sent));
        assert!(f.m.selected_alone(&at("e")));
        let open = f.sent.len() as u64;
        let error = Box::new(SessionError::Gpu("unreadable".to_owned()));
        f.m.apply(Event::Failed(SessionError::Open { open, error }));
        assert_eq!(f.m.selected_count(), 2);
        assert!(!f.m.is_selected(&at("e")));
    }

    /// Shift-click sends the range with the Browser's order, the active photo staying.
    #[test]
    fn shift_click_selects_the_range_in_the_browser_s_order() {
        let mut f = Folder::new("b");
        f.told(&["b"], Some("b"));
        let range = SelectionCommand::Range {
            to: at("d"),
            order: f.order.clone(),
        };
        assert_eq!(f.browse(BrowserAction::SelectRange(3)), [selection(range)]);
        assert_eq!(f.m.photo.asked(), Some(at("b").as_path()));
        assert_eq!(f.m.selected_count(), 3);
    }

    /// Photos opened alone are no collection of the session's: a click opens one, and
    /// Ctrl and Shift-click send nothing, which the session would refuse.
    #[test]
    fn photos_opened_alone_take_no_selection_gesture() {
        let mut f = Folder::new("a");
        for action in [
            BrowserAction::ToggleSelected(1),
            BrowserAction::ToggleSelected(0),
            BrowserAction::SelectRange(2),
        ] {
            assert!(f.browse_in(action, true).is_empty());
        }
        let open = BrowserAction::Open(1);
        assert_eq!(f.browse_in(open, true), [Command::Open(at("b"))]);
    }
}
