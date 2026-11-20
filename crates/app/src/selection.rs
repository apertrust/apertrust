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
//!
//! Edit's selection items run here too ([`edit`]): Select All, Select None, Invert
//! Selection and Select Unedited, each drawn disabled where it would change nothing
//! ([`selectable`]).

use std::path::{Path, PathBuf};

use apertrust_engine::session::library::EditState;
use apertrust_engine::session::{Command, SelectionCommand};
use apertrust_ui::components::BrowserAction;

use crate::actions::Selectable;
use crate::folder::{Folder, Listing};
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

/// Runs one of Edit's selection items, `command` (`All`, `Clear`, `Invert` or `Unedited`),
/// as the session will (decision 37): Select All keeps the active photo; Select None
/// closes it, as File › Close does, and opens none; Invert Selection closes it, and Select
/// Unedited where it has an edit, as its cell's pencil says. Where those two close it,
/// `waiting` takes the command's number: once the session tells the selection through it,
/// the first photo of the new selection in the Browser's order opens ([`open_first`]). A
/// wait under way follows the newest command, so the photo opened is in the selection the
/// session ends with. A wait ends with the folder it was sent in: another folder, or a
/// close, opens nothing.
pub(crate) fn edit(
    command: SelectionCommand,
    waiting: &mut Option<Waiting>,
    mirror: &mut Mirror,
    send: &mut SendCommand<'_>,
) {
    let active = mirror.photo.asked().map(Path::to_owned);
    let leaves = active.is_some_and(|active| match &command {
        SelectionCommand::Clear | SelectionCommand::Invert => true,
        SelectionCommand::Unedited => !listing(mirror).is_some_and(|l| unedited(l, &active)),
        _ => false,
    });
    let clear = command == SelectionCommand::Clear;
    let number = select(command, mirror, send);
    // The session closes the photo it leaves out (`Mirror::closing`).
    if leaves {
        mirror.closing();
    }
    let folder = mirror.folder.latest();
    if (leaves && !clear) || waiting.is_some_and(|waiting| waiting.folder == folder) {
        *waiting = Some(Waiting {
            command: number,
            folder,
        });
    }
}

/// What [`edit`] waits for: the command whose new selection's first photo is to open, and
/// the folder's open or close it was sent after (`FolderState::latest`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Waiting {
    command: u64,
    folder: u64,
}

/// Opens the first selected photo in the Browser's order once the session has told the
/// selection through `waiting`, the number [`edit`] left there, and stops waiting; with
/// none selected, nothing. A photo asked for meanwhile, or another folder or a close,
/// ends the wait, opening nothing.
pub(crate) fn open_first(
    waiting: &mut Option<Waiting>,
    mirror: &mut Mirror,
    send: &mut SendCommand<'_>,
) {
    let Some(Waiting { command, folder }) = *waiting else {
        return;
    };
    if mirror.photo.asked().is_some() || mirror.folder.latest() != folder {
        *waiting = None;
        return;
    }
    if mirror.selection.through < command {
        return;
    }
    *waiting = None;
    let first = listing(mirror).and_then(|listing| {
        (mirror.folder.order().iter())
            .map(|&i| listing.path.join(&listing.photos[i].name))
            .find(|photo| mirror.is_selected(photo))
    });
    if let Some(first) = first {
        open_listed(first, mirror, send);
    }
}

/// What Edit's selection items would change, as the mirror shows the selection and the
/// folder's photos: nothing to select, or to take out, without a folder open. Worked out
/// for each frame's actions, so the folder's photos are looked up one by one only while
/// as many are selected.
pub(crate) fn selectable(mirror: &Mirror) -> Selectable {
    let active = mirror.photo.asked();
    let alone = active.is_some_and(|active| mirror.is_selected(active));
    let none = mirror.selected_count() > usize::from(alone);
    let Some(listing) = listing(mirror) else {
        return Selectable {
            none,
            ..Selectable::default()
        };
    };
    let mut photos = (listing.photos.iter()).map(|photo| listing.path.join(&photo.name));
    // Fewer selected than the folder holds leaves one of its photos out.
    let all = mirror.selected_count() < listing.photos.len()
        || photos.any(|photo| !mirror.is_selected(&photo));
    Selectable {
        all,
        none,
        invert: !listing.photos.is_empty(),
        unedited: (listing.photos.iter()).any(|photo| photo.edit == EditState::None),
    }
}

/// The folder open, the session's collection; none with no folder or photos opened alone.
fn listing(mirror: &Mirror) -> Option<&Listing> {
    match &mirror.folder.folder {
        Folder::Open(listing) => Some(listing),
        _ => None,
    }
}

/// Whether the photo at `path` is one of `listing`'s with no edit, as its cell's pencil
/// says, which the session's `Unedited` reads.
fn unedited(listing: &Listing, path: &Path) -> bool {
    (listing.position(path)).is_some_and(|i| listing.photos[i].edit == EditState::None)
}

/// Sends `command`, shown in the mirror's selection until the session tells it, and
/// returns its number.
fn select(command: SelectionCommand, mirror: &mut Mirror, send: &mut SendCommand<'_>) -> u64 {
    let number = send(Command::Selection(command.clone()));
    mirror.selecting(&command, number);
    number
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
    use apertrust_engine::session::library::Photo;
    use apertrust_engine::session::{Event, LibraryEvent, Selection, SessionError};
    use apertrust_ui::components::{Sort, SortKey};

    /// The folder `/photos` with photos `a` to `e`, in that order, as the Browser shows it,
    /// the mirror, and every command sent, numbered from 1 as the session numbers them.
    struct Folder {
        m: Mirror,
        order: Vec<PathBuf>,
        sent: Vec<Command>,
        /// The command whose new selection's first photo is to open ([`edit`]).
        waiting: Option<Waiting>,
    }

    impl Folder {
        /// With `active` opened, the first command.
        fn new(active: &str) -> Self {
            let order = ["a", "b", "c", "d", "e"].map(at).to_vec();
            let mut folder = Folder {
                m: mirror(),
                order,
                sent: Vec::new(),
                waiting: None,
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

        /// The folder listed in the mirror, as the Browser shows it, sorted by file name
        /// with `descending` order: the photos `edited` have an edit, the rest none.
        fn listed(&mut self, edited: &[&str], descending: bool) {
            self.listed_by(100, edited, descending);
        }

        /// The folder listed as [`Folder::listed`] does, by the open numbered `open`.
        fn listed_by(&mut self, open: u64, edited: &[&str], descending: bool) {
            let folder = PathBuf::from("/photos");
            self.m.folder.opening(folder.clone(), open);
            let name = "photos".to_owned();
            let opened = LibraryEvent::Opened { open, name, folder };
            self.m.apply(Event::Library(opened));
            let photos = ["a", "b", "c", "d", "e"]
                .map(|name| Photo {
                    name: format!("{name}.ARW"),
                    size: 1,
                    modified: None,
                    edit: if edited.contains(&name) {
                        EditState::Edited
                    } else {
                        EditState::None
                    },
                })
                .to_vec();
            let listed = LibraryEvent::Listed {
                open,
                photos,
                done: true,
            };
            self.m.apply(Event::Library(listed));
            let key = SortKey::FileName;
            self.m.folder.sort(Sort { key, descending });
        }

        /// What Edit's item sends for `command`.
        fn edit(&mut self, command: SelectionCommand) -> Vec<Command> {
            let from = self.sent.len();
            edit(
                command,
                &mut self.waiting,
                &mut self.m,
                &mut sender(&mut self.sent),
            );
            self.sent[from..].to_vec()
        }

        /// What waiting for the new selection's first photo sends, as after each event.
        fn open_first(&mut self) -> Vec<Command> {
            let from = self.sent.len();
            open_first(&mut self.waiting, &mut self.m, &mut sender(&mut self.sent));
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

    /// Each of Edit's selection items sends its command; Select All keeps the active photo.
    #[test]
    fn edit_s_selection_items_send_their_commands() {
        for command in [
            SelectionCommand::All,
            SelectionCommand::Clear,
            SelectionCommand::Invert,
            SelectionCommand::Unedited,
        ] {
            let mut f = Folder::new("a");
            f.listed(&[], false);
            f.told(&["a"], Some("a"));
            assert_eq!(f.edit(command.clone()), [selection(command)]);
        }
        let mut f = Folder::new("a");
        f.listed(&[], false);
        f.told(&["a"], Some("a"));
        f.edit(SelectionCommand::All);
        assert_eq!(f.m.photo.asked(), Some(at("a").as_path()));
        assert_eq!(f.waiting, None);
    }

    /// Select None closes the active photo, as the session does, and opens none.
    #[test]
    fn select_none_closes_the_active_photo_and_opens_none() {
        let mut f = Folder::new("a");
        f.listed(&[], false);
        f.told(&["a", "b"], Some("a"));
        f.edit(SelectionCommand::Clear);
        assert!(matches!(f.m.photo, PhotoState::None));
        assert_eq!(f.m.selected_count(), 0);
        f.told(&[], None);
        assert!(f.open_first().is_empty());
    }

    /// Invert Selection closes the active photo, which it leaves out, and once the session
    /// tells the new selection opens its first photo in the Browser's order; a fact before
    /// the command's opens nothing, and an empty selection none.
    #[test]
    fn invert_dropping_the_active_photo_opens_the_first_of_the_new_selection() {
        let mut f = Folder::new("c");
        // e, d, c, b, a.
        f.listed(&[], true);
        f.told(&["a", "c"], Some("c"));
        let invert = f.sent.len() as u64 + 1;
        assert_eq!(
            f.edit(SelectionCommand::Invert),
            [selection(SelectionCommand::Invert)]
        );
        assert!(matches!(f.m.photo, PhotoState::None));
        assert!(f.open_first().is_empty());
        f.fact(&["a", "c"], Some("c"), invert - 1);
        assert!(f.open_first().is_empty());
        f.told(&["b", "d", "e"], None);
        assert_eq!(f.open_first(), [Command::Open(at("e"))]);
        assert_eq!(f.waiting, None);
        // Every photo selected: none left to open.
        let mut f = Folder::new("a");
        f.listed(&[], false);
        f.told(&["a", "b", "c", "d", "e"], Some("a"));
        f.edit(SelectionCommand::Invert);
        f.told(&[], None);
        assert!(f.open_first().is_empty());
    }

    /// Select Unedited keeps the active photo where it has no edit; where it has one, it
    /// closes it and opens the first of the new selection once the session tells it.
    #[test]
    fn select_unedited_closes_the_active_photo_only_where_it_has_an_edit() {
        let mut f = Folder::new("a");
        f.listed(&["b", "d"], false);
        f.told(&["a"], Some("a"));
        f.edit(SelectionCommand::Unedited);
        assert_eq!(f.m.photo.asked(), Some(at("a").as_path()));
        assert_eq!(f.waiting, None);
        let mut f = Folder::new("b");
        f.listed(&["b", "d"], false);
        f.told(&["b"], Some("b"));
        f.edit(SelectionCommand::Unedited);
        assert!(matches!(f.m.photo, PhotoState::None));
        f.told(&["a", "c", "e"], None);
        assert_eq!(f.open_first(), [Command::Open(at("a"))]);
    }

    /// A wait follows the newest of Edit's commands, so what opens is in the selection the
    /// session ends with; a photo opened meanwhile ends it.
    #[test]
    fn the_first_photo_waits_for_the_newest_command_and_a_photo_opened_ends_it() {
        let mut f = Folder::new("c");
        f.listed(&[], false);
        f.told(&["c"], Some("c"));
        f.edit(SelectionCommand::Invert);
        let invert = f.sent.len() as u64;
        f.edit(SelectionCommand::Clear);
        f.fact(&["a", "b", "d", "e"], None, invert);
        assert!(f.open_first().is_empty());
        f.told(&[], None);
        assert!(f.open_first().is_empty());
        assert_eq!(f.waiting, None);
        let mut f = Folder::new("c");
        f.listed(&[], false);
        f.told(&["c"], Some("c"));
        f.edit(SelectionCommand::Invert);
        f.browse(BrowserAction::Open(1));
        f.told(&["b"], Some("b"));
        assert!(f.open_first().is_empty());
        assert_eq!(f.waiting, None);
    }

    /// The wait ends with its folder: a close, or the folder opened anew, before the
    /// session tells the selection opens nothing, there or after Select All in the next.
    #[test]
    fn the_first_photo_waits_only_while_its_folder_stays_open() {
        let mut f = Folder::new("c");
        f.listed(&[], false);
        f.told(&["c"], Some("c"));
        f.edit(SelectionCommand::Invert);
        f.m.folder.closing(200);
        f.told(&["a", "b", "d", "e"], None);
        assert!(f.open_first().is_empty());
        assert_eq!(f.waiting, None);
        let mut f = Folder::new("c");
        f.listed(&[], false);
        f.told(&["c"], Some("c"));
        f.edit(SelectionCommand::Invert);
        f.listed_by(300, &[], false);
        f.edit(SelectionCommand::All);
        f.told(&["a", "b", "c", "d", "e"], None);
        assert!(f.open_first().is_empty());
        assert_eq!(f.waiting, None);
    }

    /// Each item is disabled where it would change nothing: Select All with every photo
    /// selected, Select None with none or the active photo alone, Invert Selection and
    /// Select Unedited with no folder, Select Unedited with every photo edited.
    #[test]
    fn edit_s_selection_items_rest_where_they_would_change_nothing() {
        let mut f = Folder::new("a");
        f.told(&["a"], Some("a"));
        // No folder open, as with photos opened alone: no collection.
        assert_eq!(selectable(&f.m), Selectable::default());
        f.listed(&[], false);
        let listed = Selectable {
            all: true,
            none: false,
            invert: true,
            unedited: true,
        };
        assert_eq!(selectable(&f.m), listed);
        f.told(&["a", "b", "c", "d", "e"], Some("a"));
        let every = Selectable {
            all: false,
            none: true,
            ..listed
        };
        assert_eq!(selectable(&f.m), every);
        // None active, one selected: Select None takes it out.
        f.m.closing();
        f.told(&["b"], None);
        assert!(selectable(&f.m).none);
        f.told(&[], None);
        assert!(!selectable(&f.m).none);
        let mut f = Folder::new("a");
        f.listed(&["a", "b", "c", "d", "e"], false);
        assert!(!selectable(&f.m).unedited);
    }
}
