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
//! - **A click,** or Enter, makes the photo active, opening it: one of the selection keeps
//!   it, so a click moves through several selected; any other is selected alone.
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
//! reads what the last one did. Among photos opened alone a click opens the photo and Ctrl
//! and Shift-click do nothing yet.
//!
//! The Select menu's items run here too ([`edit`]): Select All, Select None, Invert
//! Selection and Select Unedited, each drawn disabled where it would change nothing
//! ([`selectable`]), over the folder's photos or the photos opened alone, the session's
//! collection either way. So do Next Photo and Previous Photo ([`step`]): with several
//! photos selected, the active photo moves through them in the Browser's order, the
//! selection staying; else through the folder or the photos opened alone, the photo
//! reached selected alone. And an export takes every selected photo ([`exported`]). Each
//! reads the collection as the Browser shows it ([`Collection`]), so a folder and photos
//! opened alone take one path.

use std::path::{Path, PathBuf};

use apertrust_engine::session::library::EditState;
use apertrust_engine::session::{Command, SelectionCommand};
use apertrust_ui::components::BrowserAction;

use crate::actions::Selectable;
use crate::folder::{self, Folder, Listing};
use crate::mirror::{Mirror, PhotoState};
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
            // The photo open stays as it is when its own cell is pressed again. Its open
            // keeps a selection that holds it and selects any other alone, as the
            // session's does.
            if active.as_ref() != Some(&path) {
                let outside = !mirror.is_selected(&path);
                open_listed(path.clone(), mirror, send);
                // All, Invert or Unedited not yet told may select it before the open
                // comes: a photo shown outside the selection is selected alone all the same.
                if outside && mirror.selection_pending() {
                    select(SelectionCommand::Set(vec![path]), mirror, send);
                }
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

/// Opens the photo at `i` of `order`, the Browser's, as its cell's menu opens (decision
/// 91), unless it is the active one: a photo in the selection keeps it, so the menu acts on
/// the selection; one outside it is selected alone, as a click does.
pub(crate) fn open_for_menu(
    i: usize,
    order: &[PathBuf],
    alone: bool,
    mirror: &mut Mirror,
    send: &mut SendCommand<'_>,
) {
    let Some(path) = order.get(i) else {
        return;
    };
    if mirror.photo.asked() == Some(path.as_path()) {
        return;
    }
    if mirror.is_selected(path) {
        open_listed(path.clone(), mirror, send);
    } else {
        browse(BrowserAction::Open(i), order, alone, mirror, send);
    }
}

/// Runs one of the Select menu's items, `command` (`All`, `Clear`, `Invert` or `Unedited`),
/// as the session will (decision 37): Select All keeps the active photo; Select None
/// closes it, as File › Close does, and opens none; Invert Selection closes it, and Select
/// Unedited where it has an edit, as its cell's pencil says; among photos opened alone, whose
/// edits the app does not know, Select Unedited leaves it to the session, whose selection
/// closes it if it does. Where those two close it,
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
        SelectionCommand::Unedited => match alone(mirror) {
            Some(_) => edited(mirror),
            None => !listing(mirror).is_some_and(|l| unedited(l, &active)),
        },
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
    let first = collection(mirror).and_then(|collection| {
        (mirror.folder.order().iter())
            .map(|&i| collection.path(i))
            .find(|photo| mirror.is_selected(photo))
    });
    if let Some(first) = first {
        open_listed(first, mirror, send);
    }
}

/// What the Select menu's items would change, as the mirror shows the selection and the
/// folder's photos, or the photos opened alone: nothing to select, or to take out, with
/// neither. Worked out for each frame's actions, so the folder's photos are looked up one
/// by one only while as many are selected. The app knows no edit of the photos opened
/// alone, so Select Unedited is the session's to decide with any of them.
pub(crate) fn selectable(mirror: &Mirror) -> Selectable {
    let active = mirror.photo.asked();
    let alone_selected = active.is_some_and(|active| mirror.is_selected(active));
    let none = mirror.selected_count() > usize::from(alone_selected);
    if let Some(paths) = alone(mirror) {
        let all = mirror.selected_count() < paths.len()
            || paths.iter().any(|photo| !mirror.is_selected(photo));
        // Whether Unedited leaves the active photo out is read from its values
        // ([`edited`]), so it waits until the photo is live.
        let settled = active.is_none() || mirror.live();
        return Selectable {
            all,
            none,
            invert: !paths.is_empty(),
            unedited: !paths.is_empty() && settled,
        };
    }
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

/// Next Photo, or Previous Photo where `next` is false: opens the photo [`stepped`] finds,
/// which a selected one keeps the selection with and any other is selected alone by, as
/// the session's `Open` does; at the end, nothing.
pub(crate) fn step(next: bool, mirror: &mut Mirror, send: &mut SendCommand<'_>) {
    if let Some(photo) = stepped(next, mirror) {
        open_listed(photo, mirror, send);
    }
}

/// The photo Next Photo (`next`) or Previous Photo reaches from the active one, in the
/// Browser's order: the next or previous selected photo while several are selected, else
/// the next or previous of the folder, or of the photos opened alone; with no active photo,
/// the first, or the last. None at the end, where the keys stop rather than wrap round,
/// with neither open, and while All, Invert or Unedited waits for the session to tell what
/// it selected, so a step never reads the selection before them
/// ([`Mirror::selection_pending`]). Worked out for each frame's actions, so only the
/// photos up to the one reached are looked up.
pub(crate) fn stepped(next: bool, mirror: &Mirror) -> Option<PathBuf> {
    if mirror.selection_pending() {
        return None;
    }
    let collection = collection(mirror)?;
    let order = mirror.folder.order();
    let several = mirror.selected_count() > 1;
    let reached = |i: &usize| !several || mirror.is_selected(&collection.path(*i));
    // From the photo asked for last, one that failed to open included, so the keys move on
    // past it rather than back to the start.
    let at = (mirror.photo.path())
        .and_then(|active| collection.position(active))
        .and_then(|active| order.iter().position(|&i| i == active));
    let found = if next {
        let from = at.map_or(0, |at| at + 1);
        order[from..].iter().copied().find(reached)
    } else {
        let to = at.unwrap_or(order.len());
        order[..to].iter().rev().copied().find(reached)
    };
    found.map(|i| collection.path(i))
}

/// The photos an export takes, and how many it leaves out ([`exported`]).
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Exported {
    /// Their paths as the Browser names them, in its order.
    pub photos: Vec<PathBuf>,
    /// The selected photos open for viewing only, which the export leaves out.
    pub left_out: usize,
    /// Whether the photo open is one of `left_out`, which the Export tool counts itself.
    pub open_left_out: bool,
}

/// What Export and Export Again take (decision 90): with several selected, every selected
/// photo of the folder, or of the photos opened alone, in the Browser's order, the
/// selection staying as it is; else the photo open. Nothing while All, Invert or Unedited
/// is not yet told, as Next Photo waits for them ([`Mirror::selection_pending`]). A photo
/// open for viewing only is left out, as the Export tool leaves it out: the open one by its
/// own state, the folder's others by their listed edit, a newer Apertrust's or an
/// unreadable one ([`Collection::view_only`]). The other photos alone have no edit the app
/// knows, so none is left out: the export job exports one for viewing only as this build
/// reads its edit, its result saying so, or fails it where the edit cannot be read.
pub(crate) fn exported(mirror: &Mirror) -> Exported {
    let mut exported = Exported::default();
    // All, Invert or Unedited not yet told: what is selected waits for the session.
    if mirror.selection_pending() {
        return exported;
    }
    match several(mirror) {
        Some(collection) => {
            let open = mirror.photo.asked().map(Path::to_owned);
            for (photo, takes) in selected(mirror, collection) {
                if takes {
                    exported.photos.push(photo);
                } else {
                    exported.open_left_out |= open.as_ref() == Some(&photo);
                    exported.left_out += 1;
                }
            }
        }
        None => match &mirror.photo {
            PhotoState::Open(photo) if photo.view_only.is_none() => {
                exported.photos.push(photo.path.clone());
            }
            PhotoState::Open(_) => {
                exported.left_out = 1;
                exported.open_left_out = true;
            }
            _ => {}
        },
    }
    exported
}

/// Whether an export would take any photo ([`exported`]), stopping at the first: worked out
/// for each frame's actions.
pub(crate) fn exportable(mirror: &Mirror) -> bool {
    if mirror.selection_pending() {
        return false;
    }
    match several(mirror) {
        Some(collection) => selected(mirror, collection).any(|(_, takes)| takes),
        None => matches!(&mirror.photo, PhotoState::Open(photo) if photo.view_only.is_none()),
    }
}

/// The collection, while an export takes its selection: several photos selected, or one
/// with no photo open or opening, as a Ctrl-click with none active leaves it.
fn several(mirror: &Mirror) -> Option<Collection<'_>> {
    let selected = mirror.selected_count();
    let open = mirror.photo.asked().is_some();
    collection(mirror).filter(|_| selected > 1 || selected == 1 && !open)
}

/// How many photos an export takes ([`exported`]), for each frame's words: the photos
/// selected less those left out, only the folder's view-only photos and the one open
/// looked up; among photos opened alone, whose edits the app does not know, only the one
/// open. While the folder is listed, a photo selected may not be listed yet, which the
/// export passes over: the photos it takes are counted one by one then.
pub(crate) fn exported_count(mirror: &Mirror) -> usize {
    if mirror.selection_pending() {
        return 0;
    }
    let Some(collection) = several(mirror) else {
        return usize::from(exportable(mirror));
    };
    if !collection.done() {
        return exported(mirror).photos.len();
    }
    let open = match &mirror.photo {
        PhotoState::Open(photo) => Some(photo),
        _ => None,
    };
    // The folder's view-only photos selected, but the one open, which its own state says.
    let listed = (0..collection.len())
        .filter(|&i| collection.view_only(i))
        .map(|i| collection.path(i))
        .filter(|path| mirror.is_selected(path))
        .filter(|path| open.is_none_or(|open| open.path != *path))
        .count();
    // The active photo, which the selection keeps where the listing lacks it, is no photo
    // of the Browser's order, so the export passes it over.
    let active = mirror
        .photo
        .asked()
        .filter(|active| mirror.is_selected(active));
    let unlisted = active.is_some_and(|active| collection.position(active).is_none());
    let open = !unlisted
        && open.is_some_and(|open| open.view_only.is_some() && mirror.is_selected(&open.path));
    let left_out = listed + usize::from(unlisted) + usize::from(open);
    mirror.selected_count().saturating_sub(left_out)
}

/// The selected photos of `collection`, in the Browser's order, each with whether an
/// export takes it ([`exported`]).
fn selected<'a>(
    mirror: &'a Mirror,
    collection: Collection<'a>,
) -> impl Iterator<Item = (PathBuf, bool)> + 'a {
    let open = match &mirror.photo {
        PhotoState::Open(photo) => Some(photo),
        _ => None,
    };
    mirror.folder.order().iter().filter_map(move |&i| {
        let path = collection.path(i);
        if !mirror.is_selected(&path) {
            return None;
        }
        let takes = match open.filter(|open| open.path == path) {
            Some(open) => open.view_only.is_none(),
            None => !collection.view_only(i),
        };
        Some((path, takes))
    })
}

/// The session's collection as the Browser shows it (decision 37): the folder open's
/// photos, or the photos opened alone, each by its place there, which the Browser's order
/// (`FolderState::order`) sorts.
#[derive(Clone, Copy)]
enum Collection<'a> {
    Folder(&'a Listing),
    Alone(&'a [PathBuf]),
}

impl Collection<'_> {
    /// How many photos it holds so far.
    fn len(self) -> usize {
        match self {
            Collection::Folder(listing) => listing.photos.len(),
            Collection::Alone(paths) => paths.len(),
        }
    }

    /// The path of the photo at `i`, as the Browser names it.
    fn path(self, i: usize) -> PathBuf {
        match self {
            Collection::Folder(listing) => listing.path.join(&listing.photos[i].name),
            Collection::Alone(paths) => paths[i].clone(),
        }
    }

    /// The place of the photo at `path`, if it is one of the collection's.
    fn position(self, path: &Path) -> Option<usize> {
        match self {
            Collection::Folder(listing) => listing.position(path),
            Collection::Alone(paths) => paths.iter().position(|alone| folder::same(alone, path)),
        }
    }

    /// Whether the photo at `i` is known to be open for viewing only, which an export
    /// leaves out: a folder's by its listed edit, a newer Apertrust's or an unreadable one.
    /// None of the photos opened alone is, their edits unknown to the app.
    fn view_only(self, i: usize) -> bool {
        match self {
            Collection::Folder(listing) => matches!(
                listing.photos[i].edit,
                EditState::ViewOnly(_) | EditState::Unreadable(_)
            ),
            Collection::Alone(_) => false,
        }
    }

    /// Whether every photo has come: a folder's once listed, the photos alone at once.
    fn done(self) -> bool {
        match self {
            Collection::Folder(listing) => listing.done,
            Collection::Alone(_) => true,
        }
    }
}

/// The session's collection, the folder open or the photos opened alone; none with
/// neither.
fn collection(mirror: &Mirror) -> Option<Collection<'_>> {
    match &mirror.folder.folder {
        Folder::Open(listing) => Some(Collection::Folder(listing)),
        Folder::Alone { paths, .. } => Some(Collection::Alone(paths)),
        _ => None,
    }
}

/// The folder open, the session's collection; none with no folder or photos opened alone.
fn listing(mirror: &Mirror) -> Option<&Listing> {
    match &mirror.folder.folder {
        Folder::Open(listing) => Some(listing),
        _ => None,
    }
}

/// The photos opened alone, in the order opened, the session's collection; none with a
/// folder or nothing open.
fn alone(mirror: &Mirror) -> Option<&[PathBuf]> {
    match &mirror.folder.folder {
        Folder::Alone { paths, .. } => Some(paths),
        _ => None,
    }
}

/// Whether the open photo has an edit as the session's `Unedited` reads it among photos
/// opened alone, which have no pencils: its edit as it stands, a view-only photo counting
/// as edited. Select Unedited waits there until the photo is live ([`selectable`]), so the
/// values read are its own.
fn edited(mirror: &Mirror) -> bool {
    match &mirror.photo {
        PhotoState::Open(photo) => mirror.values.edited || photo.view_only.is_some(),
        _ => false,
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

        /// The photos `names` opened alone in the mirror, in that order, the Browser's.
        fn alone(&mut self, names: &[&str]) {
            let paths = names.iter().map(|name| at(name)).collect();
            self.m.folder.alone(50, paths);
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

        /// What Next Photo (`next`) or Previous Photo sends.
        fn step(&mut self, next: bool) -> Vec<Command> {
            let from = self.sent.len();
            step(next, &mut self.m, &mut sender(&mut self.sent));
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

    /// A click opens the photo: one outside the selection is selected alone by its open,
    /// one of several selected keeps the selection, and the active photo's own cell sends
    /// nothing.
    #[test]
    fn a_click_opens_the_photo_keeping_a_selection_that_holds_it() {
        let mut f = Folder::new("a");
        f.told(&["a"], Some("a"));
        // c, not selected, is selected alone by its open.
        assert_eq!(f.browse(BrowserAction::Open(2)), [Command::Open(at("c"))]);
        assert_eq!(f.m.photo.asked(), Some(at("c").as_path()));
        assert!(f.m.selected_alone(&at("c")));
        // d, one of three selected: active, the three staying.
        f.told(&["b", "c", "d"], Some("c"));
        assert_eq!(f.browse(BrowserAction::Open(3)), [Command::Open(at("d"))]);
        f.told(&["b", "c", "d"], Some("d"));
        assert_eq!(f.m.selected_count(), 3);
        // The active photo's own cell: nothing, however many are selected.
        assert!(f.browse(BrowserAction::Open(3)).is_empty());
        assert_eq!(f.m.selected_count(), 3);
    }

    /// A click on a photo shown outside the selection selects it alone though Select All,
    /// sent before it and not yet told, would select it first and the open keep them all.
    #[test]
    fn a_click_outside_the_selection_before_select_all_is_told_selects_the_photo_alone() {
        let mut f = Folder::new("a");
        f.listed(&[], false);
        f.told(&["a"], Some("a"));
        f.edit(SelectionCommand::All);
        let all = f.sent.len() as u64;
        let alone = selection(SelectionCommand::Set(vec![at("c")]));
        let sent = f.browse(BrowserAction::Open(2));
        assert_eq!(sent, [Command::Open(at("c")), alone]);
        assert!(f.m.selected_alone(&at("c")));
        // Select All told, the click not yet: still c alone, as the session will make it.
        f.fact(&["a", "b", "c", "d", "e"], Some("a"), all);
        assert!(f.m.selected_alone(&at("c")));
        // With nothing pending, a click outside the selection sends the open alone.
        f.told(&["c"], Some("c"));
        assert_eq!(f.browse(BrowserAction::Open(3)), [Command::Open(at("d"))]);
    }

    /// A cell's menu opens its photo first, as a click does: one in the selection keeps it;
    /// one outside it is selected alone; the active one opens nothing.
    #[test]
    fn a_cell_s_menu_opens_its_photo_keeping_the_selection_that_holds_it() {
        let mut f = Folder::new("c");
        f.listed(&[], false);
        f.told(&["b", "c", "d"], Some("c"));
        let menu = |f: &mut Folder, i: usize| {
            let from = f.sent.len();
            open_for_menu(i, &f.order, false, &mut f.m, &mut sender(&mut f.sent));
            f.sent[from..].to_vec()
        };
        assert_eq!(menu(&mut f, 3), [Command::Open(at("d"))]);
        f.told(&["b", "c", "d"], Some("d"));
        assert!(menu(&mut f, 3).is_empty());
        // Outside the selection, the open selects it alone, as the session does.
        assert_eq!(menu(&mut f, 0), [Command::Open(at("a"))]);
        assert!(f.m.selected_alone(&at("a")));
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
    /// click on a photo a Ctrl-click has just added opens it, the two staying selected.
    #[test]
    fn a_click_after_a_ctrl_click_not_yet_told_keeps_both_selected() {
        let mut f = Folder::new("a");
        f.told(&["a"], Some("a"));
        f.browse(BrowserAction::ToggleSelected(1));
        assert_eq!(f.m.selected_count(), 2);
        assert_eq!(f.browse(BrowserAction::Open(1)), [Command::Open(at("b"))]);
        assert_eq!(f.m.selected_count(), 2);
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

    /// Among photos opened alone a click opens one, and Ctrl and Shift-click send nothing
    /// yet.
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

    /// Each of the Select menu's items sends its command; Select All keeps the active photo.
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
    /// Select Unedited with no collection, Select Unedited with every photo edited.
    #[test]
    fn edit_s_selection_items_rest_where_they_would_change_nothing() {
        let mut f = Folder::new("a");
        f.told(&["a"], Some("a"));
        // No folder open and no photos alone: no collection.
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

    /// With several selected, Next Photo and Previous Photo move the active photo through
    /// them in the Browser's order, the selection staying; at either end they stop.
    #[test]
    fn next_and_previous_move_through_the_selection_which_stays() {
        let mut f = Folder::new("b");
        f.listed(&[], false);
        f.told(&["b", "d", "e"], Some("b"));
        assert_eq!(f.step(true), [Command::Open(at("d"))]);
        assert_eq!(f.step(true), [Command::Open(at("e"))]);
        assert_eq!(f.m.selected_count(), 3);
        // The last selected: nothing further, the item disabled.
        assert_eq!(stepped(true, &f.m), None);
        assert!(f.step(true).is_empty());
        assert_eq!(f.step(false), [Command::Open(at("d"))]);
        assert_eq!(f.step(false), [Command::Open(at("b"))]);
        assert!(f.step(false).is_empty());
        assert_eq!(f.m.selected_count(), 3);
        assert_eq!(f.m.photo.asked(), Some(at("b").as_path()));
    }

    /// With one selected, they go to the next or previous photo of the folder, selected
    /// alone; at the folder's ends they stop.
    #[test]
    fn with_one_selected_next_and_previous_go_through_the_folder() {
        let mut f = Folder::new("b");
        f.listed(&[], false);
        f.told(&["b"], Some("b"));
        assert_eq!(f.step(true), [Command::Open(at("c"))]);
        assert!(f.m.selected_alone(&at("c")));
        assert_eq!(f.step(false), [Command::Open(at("b"))]);
        assert_eq!(f.step(false), [Command::Open(at("a"))]);
        assert!(f.step(false).is_empty());
        let mut f = Folder::new("e");
        f.listed(&[], false);
        f.told(&["e"], Some("e"));
        assert!(f.step(true).is_empty());
        assert_eq!(f.step(false), [Command::Open(at("d"))]);
    }

    /// They follow the Browser's sort: in descending file names, → from c is a among the
    /// selected and b in the folder.
    #[test]
    fn next_and_previous_follow_the_browser_s_sort() {
        let mut f = Folder::new("c");
        // e, d, c, b, a.
        f.listed(&[], true);
        f.told(&["a", "c", "e"], Some("c"));
        assert_eq!(stepped(true, &f.m), Some(at("a")));
        assert_eq!(stepped(false, &f.m), Some(at("e")));
        f.told(&["c"], Some("c"));
        assert_eq!(f.step(true), [Command::Open(at("b"))]);
        assert_eq!(f.step(false), [Command::Open(at("c"))]);
        assert_eq!(f.step(false), [Command::Open(at("d"))]);
    }

    /// With no active photo, Next Photo opens the first selected and Previous Photo the
    /// last; with no folder open, neither has a photo to go to.
    #[test]
    fn with_no_active_photo_next_and_previous_open_the_first_and_the_last() {
        let mut f = Folder::new("c");
        f.listed(&[], false);
        f.m.closing();
        f.told(&["b", "d"], None);
        assert_eq!(stepped(true, &f.m), Some(at("b")));
        assert_eq!(stepped(false, &f.m), Some(at("d")));
        let mut f = Folder::new("c");
        f.told(&["c"], Some("c"));
        assert_eq!((stepped(true, &f.m), stepped(false, &f.m)), (None, None));
    }

    /// A photo that fails to open is still where the keys go on from: Next Photo moves past
    /// it, not back to the first.
    #[test]
    fn next_photo_moves_on_past_a_photo_that_failed_to_open() {
        let mut f = Folder::new("a");
        f.listed(&[], false);
        f.told(&["a"], Some("a"));
        assert_eq!(f.step(true), [Command::Open(at("b"))]);
        let open = f.sent.len() as u64;
        let error = Box::new(SessionError::Gpu("unreadable".to_owned()));
        f.m.apply(Event::Failed(SessionError::Open { open, error }));
        assert_eq!(stepped(true, &f.m), Some(at("c")));
    }

    /// Next and Previous Photo wait for Invert Selection's answer: before it they would read
    /// the selection Invert replaces, and their open would undo it.
    #[test]
    fn next_and_previous_wait_for_what_invert_selected() {
        let mut f = Folder::new("c");
        f.listed(&[], false);
        f.told(&["a", "c", "e"], Some("c"));
        f.edit(SelectionCommand::Invert);
        assert_eq!((stepped(true, &f.m), stepped(false, &f.m)), (None, None));
        f.told(&["b", "d"], None);
        assert_eq!(stepped(true, &f.m), Some(at("b")));
    }

    /// The photo open, `view_only` or not, as the session's `Opened` makes it: `a`, which
    /// [`Folder::new`] opened as command 1.
    fn opened_a(f: &mut Folder, view_only: bool) {
        use apertrust_engine::sidecar::{Problem, Reason, ViewOnly};
        let mut photo = crate::mirror::tests::photo(1);
        if view_only {
            photo.view_only = Some(ViewOnly {
                build: None,
                reason: Reason::Unreadable(Problem::End {
                    copy: "00000000".to_owned(),
                    end: "00000000000000ff".to_owned(),
                }),
            });
        }
        f.m.apply(Event::Opened(photo));
    }

    /// With one photo selected an export takes the photo open, as before; with none open,
    /// or one open for viewing only, nothing, that one left out.
    #[test]
    fn an_export_of_one_photo_takes_the_photo_open() {
        let mut f = Folder::new("a");
        f.listed(&[], false);
        f.told(&["a"], Some("a"));
        // Still opening: nothing to export yet.
        assert_eq!(exported(&f.m), Exported::default());
        assert!(!exportable(&f.m));
        opened_a(&mut f, false);
        let one = Exported {
            photos: vec![at("a")],
            left_out: 0,
            open_left_out: false,
        };
        assert_eq!(exported(&f.m), one);
        assert!(exportable(&f.m));
        let mut f = Folder::new("a");
        f.listed(&[], false);
        f.told(&["a"], Some("a"));
        opened_a(&mut f, true);
        let none = Exported {
            photos: Vec::new(),
            left_out: 1,
            open_left_out: true,
        };
        assert_eq!(exported(&f.m), none);
        assert!(!exportable(&f.m));
    }

    /// With several selected an export takes every one in the Browser's order, whatever is
    /// open, leaving out those open for viewing only, the open one by its own state and the
    /// rest by their listed edit; the selection stays as it was.
    #[test]
    fn an_export_of_several_takes_every_selected_photo_in_the_browser_s_order() {
        let mut f = Folder::new("a");
        // e, d, c, b, a.
        f.listed(&[], true);
        f.told(&["a", "c", "e"], Some("a"));
        let all = Exported {
            photos: vec![at("e"), at("c"), at("a")],
            left_out: 0,
            open_left_out: false,
        };
        assert_eq!(exported(&f.m), all);
        assert!(exportable(&f.m));
        assert_eq!(exported_count(&f.m), 3);
        assert_eq!(f.m.selected_count(), 3);
        // c's sidecar unreadable, the open a view-only: e alone.
        let mut f = Folder::new("a");
        let folder = PathBuf::from("/photos");
        f.m.folder.opening(folder.clone(), 100);
        let name = "photos".to_owned();
        f.m.apply(Event::Library(LibraryEvent::Opened {
            open: 100,
            name,
            folder,
        }));
        let newer = EditState::Unreadable("not an .apxmp".to_owned());
        let photos = [
            ("a", EditState::None),
            ("c", newer),
            ("e", EditState::Edited),
        ]
        .map(|(name, edit)| Photo {
            name: format!("{name}.ARW"),
            size: 1,
            modified: None,
            edit,
        })
        .to_vec();
        f.m.apply(Event::Library(LibraryEvent::Listed {
            open: 100,
            photos,
            done: true,
        }));
        f.told(&["a", "c", "e"], Some("a"));
        opened_a(&mut f, true);
        let e = Exported {
            photos: vec![at("e")],
            left_out: 2,
            open_left_out: true,
        };
        assert_eq!(exported(&f.m), e);
        assert!(exportable(&f.m));
        assert_eq!(exported_count(&f.m), 1);
    }

    /// While the folder is listed, a photo selected that the listing has not brought yet
    /// is passed over, and the count says what the export takes.
    #[test]
    fn while_the_folder_is_listed_the_count_is_what_the_export_takes() {
        let mut f = Folder::new("a");
        let folder = PathBuf::from("/photos");
        f.m.folder.opening(folder.clone(), 100);
        let name = "photos".to_owned();
        f.m.apply(Event::Library(LibraryEvent::Opened {
            open: 100,
            name,
            folder,
        }));
        let a = Photo {
            name: "a.ARW".to_owned(),
            size: 1,
            modified: None,
            edit: EditState::None,
        };
        f.m.apply(Event::Library(LibraryEvent::Listed {
            open: 100,
            photos: vec![a],
            done: false,
        }));
        f.told(&["a", "c"], Some("a"));
        assert_eq!(exported(&f.m).photos, [at("a")]);
        assert_eq!(exported_count(&f.m), 1);
    }

    /// The active photo, kept selected though the listing lacks it, is no photo of the
    /// Browser's order: the export passes it over, and the count with it.
    #[test]
    fn an_active_photo_the_listing_lacks_is_not_counted() {
        let mut f = Folder::new("z");
        f.listed(&[], false);
        f.told(&["a", "b", "z"], Some("z"));
        assert_eq!(exported(&f.m).photos, [at("a"), at("b")]);
        assert_eq!(exported_count(&f.m), 2);
    }

    /// A photo selected alone with none open, as a Ctrl-click with none active leaves it,
    /// is what an export takes; the count follows what it takes.
    #[test]
    fn an_export_takes_a_photo_selected_with_none_open() {
        let mut f = Folder::new("a");
        f.listed(&[], false);
        f.m.closing();
        f.told(&["c"], None);
        let c = Exported {
            photos: vec![at("c")],
            left_out: 0,
            open_left_out: false,
        };
        assert_eq!(exported(&f.m), c);
        assert!(exportable(&f.m));
        assert_eq!(exported_count(&f.m), 1);
        // Nothing selected and none open: nothing.
        f.told(&[], None);
        assert_eq!(exported(&f.m), Exported::default());
        assert_eq!(exported_count(&f.m), 0);
    }

    /// While Select All is not yet told an export takes nothing, as the selection it reads
    /// is the one Select All replaces; once told, every photo.
    #[test]
    fn an_export_waits_for_select_all_to_be_told() {
        let mut f = Folder::new("a");
        f.listed(&[], false);
        f.told(&["a", "c"], Some("a"));
        f.edit(SelectionCommand::All);
        assert_eq!(exported(&f.m), Exported::default());
        assert!(!exportable(&f.m));
        f.told(&["a", "b", "c", "d", "e"], Some("a"));
        assert_eq!(exported(&f.m).photos.len(), 5);
        assert!(exportable(&f.m));
    }

    /// Export Again exports the photos selected as it runs, as the app makes its command
    /// (`App::ask_export`): a selection changed since the last export is the one it takes.
    #[test]
    fn export_again_takes_the_selection_as_it_is_now() {
        let values = crate::export::Values::default();
        let places = crate::export::places();
        let again = |m: &Mirror| {
            let photos = exported(m).photos;
            crate::export_again::export(photos, &values, &places)
        };
        let command = |names: &[&str]| Command::Export {
            photos: names.iter().map(|name| at(name)).collect(),
            settings: values.settings(),
            places: places.clone(),
        };
        let mut f = Folder::new("a");
        f.listed(&[], false);
        f.told(&["a", "c"], Some("a"));
        assert_eq!(again(&f.m), command(&["a", "c"]));
        f.browse(BrowserAction::ToggleSelected(3));
        assert_eq!(again(&f.m), command(&["a", "c", "d"]));
    }

    /// Among photos opened alone, the session's collection, Select's items are enabled as in a
    /// folder: Select All while one is not selected, Invert Selection with any, and Select
    /// Unedited with any once the active photo is live, whose values say whether it leaves.
    #[test]
    fn edit_s_selection_items_rest_among_photos_alone_as_in_a_folder() {
        let mut f = Folder::new("a");
        f.alone(&["c", "a", "b"]);
        f.told(&["a"], Some("a"));
        let alone = Selectable {
            all: true,
            none: false,
            invert: true,
            unedited: true,
        };
        // A still opening.
        let opening = Selectable {
            unedited: false,
            ..alone
        };
        assert_eq!(selectable(&f.m), opening);
        f.m.apply(Event::Opened(crate::mirror::tests::photo(1)));
        f.m.apply(Event::Values(crate::mirror::tests::report(0.0, 1)));
        assert_eq!(selectable(&f.m), alone);
        f.told(&["a", "b", "c"], Some("a"));
        let every = Selectable {
            all: false,
            none: true,
            ..alone
        };
        assert_eq!(selectable(&f.m), every);
        f.alone(&[]);
        assert!(!selectable(&f.m).invert && !selectable(&f.m).unedited);
    }

    /// Among photos opened alone, each item sends its command. Select Unedited leaves the
    /// active photo to the session, which knows their edits. Invert Selection closes it and
    /// opens the first of the new selection in the Browser's order, the order opened.
    #[test]
    fn edit_s_selection_items_act_over_photos_alone() {
        for command in [
            SelectionCommand::All,
            SelectionCommand::Clear,
            SelectionCommand::Invert,
            SelectionCommand::Unedited,
        ] {
            let mut f = Folder::new("a");
            f.alone(&["c", "a", "b"]);
            f.told(&["a"], Some("a"));
            assert_eq!(f.edit(command.clone()), [selection(command)]);
        }
        let mut f = Folder::new("a");
        f.alone(&["c", "a", "b"]);
        f.told(&["a"], Some("a"));
        f.edit(SelectionCommand::Unedited);
        assert_eq!(f.m.photo.asked(), Some(at("a").as_path()));
        assert_eq!(f.waiting, None);
        // Open with an edit, the active photo is left out by Unedited: it closes, and the
        // first of the new selection in the order opened opens once the session tells it.
        let mut f = Folder::new("a");
        f.alone(&["c", "a", "b"]);
        f.told(&["a"], Some("a"));
        f.m.apply(Event::Opened(crate::mirror::tests::photo(1)));
        let edited = crate::mirror::tests::report(0.5, 1);
        f.m.apply(Event::Values(apertrust_engine::session::Values {
            edited: true,
            ..edited
        }));
        f.edit(SelectionCommand::Unedited);
        assert!(matches!(f.m.photo, PhotoState::None));
        f.told(&["b", "c"], None);
        assert_eq!(f.open_first(), [Command::Open(at("c"))]);
        let mut f = Folder::new("a");
        f.alone(&["c", "a", "b"]);
        f.told(&["a"], Some("a"));
        f.edit(SelectionCommand::Invert);
        assert!(matches!(f.m.photo, PhotoState::None));
        assert!(f.open_first().is_empty());
        f.told(&["b", "c"], None);
        assert_eq!(f.open_first(), [Command::Open(at("c"))]);
        assert_eq!(f.waiting, None);
    }

    /// Among photos opened alone, Next Photo and Previous Photo move through the selection
    /// with several selected, the selection staying, and through the photos alone in the
    /// order opened with one, the photo reached selected alone; at either end they stop,
    /// and while Invert Selection is not yet told they wait.
    #[test]
    fn next_and_previous_move_through_photos_opened_alone() {
        let mut f = Folder::new("a");
        f.alone(&["c", "a", "b"]);
        f.told(&["a"], Some("a"));
        assert_eq!(f.step(true), [Command::Open(at("b"))]);
        assert!(f.m.selected_alone(&at("b")));
        // The last opened: nothing further, the item disabled.
        assert_eq!(stepped(true, &f.m), None);
        assert!(f.step(true).is_empty());
        assert_eq!(f.step(false), [Command::Open(at("a"))]);
        assert_eq!(f.step(false), [Command::Open(at("c"))]);
        assert!(f.step(false).is_empty());
        // c and b of three selected: a passed over, the two staying.
        f.told(&["c", "b"], Some("c"));
        assert_eq!(f.step(true), [Command::Open(at("b"))]);
        assert!(f.step(true).is_empty());
        assert_eq!(f.step(false), [Command::Open(at("c"))]);
        assert!(f.step(false).is_empty());
        assert_eq!(f.m.selected_count(), 2);
        f.edit(SelectionCommand::Invert);
        assert_eq!((stepped(true, &f.m), stepped(false, &f.m)), (None, None));
        f.told(&["a", "b"], None);
        assert_eq!(stepped(true, &f.m), Some(at("a")));
    }

    /// With several of the photos opened alone selected, an export takes every one in the
    /// order opened, the selection staying, whatever is open; the open photo, open for
    /// viewing only, is left out and counted, the others' edits unknown to the app.
    #[test]
    fn an_export_of_several_photos_opened_alone_takes_every_selected_one() {
        let mut f = Folder::new("a");
        f.alone(&["c", "d", "a", "b"]);
        f.told(&["a", "b", "c"], Some("a"));
        let all = Exported {
            photos: vec![at("c"), at("a"), at("b")],
            left_out: 0,
            open_left_out: false,
        };
        assert_eq!(exported(&f.m), all);
        assert!(exportable(&f.m));
        assert_eq!(f.m.selected_count(), 3);
        opened_a(&mut f, true);
        let others = Exported {
            photos: vec![at("c"), at("b")],
            left_out: 1,
            open_left_out: true,
        };
        assert_eq!(exported(&f.m), others);
        assert!(exportable(&f.m));
        assert_eq!(f.m.selected_count(), 3);
    }

    /// The count of an export among photos opened alone is the photos selected, less the
    /// open one where it is for viewing only; with one selected, the photo open.
    #[test]
    fn exported_count_counts_the_photos_opened_alone_selected() {
        let mut f = Folder::new("a");
        f.alone(&["e", "a", "c"]);
        f.told(&["a"], Some("a"));
        // Still opening: nothing to export yet.
        assert_eq!(exported_count(&f.m), 0);
        opened_a(&mut f, false);
        assert_eq!(exported_count(&f.m), 1);
        f.told(&["a", "c", "e"], Some("a"));
        assert_eq!(exported_count(&f.m), 3);
        assert_eq!(exported_count(&f.m), exported(&f.m).photos.len());
        let mut f = Folder::new("a");
        f.alone(&["e", "a", "c"]);
        f.told(&["a", "c", "e"], Some("a"));
        opened_a(&mut f, true);
        assert_eq!(exported_count(&f.m), 2);
        // One selected with none open, as Select None then a Ctrl-click would leave it.
        f.m.closing();
        f.told(&["c"], None);
        assert_eq!(exported_count(&f.m), 1);
        assert_eq!(exported(&f.m).photos, [at("c")]);
    }
}
