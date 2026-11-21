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

//! The selection in the session (decision 37; `docs/gui.md`, Selection and the active
//! photo): a set of the collection's photos which holds the active photo whenever there is
//! one, the session's rather than a client's, so a script or an assistant reads and sets it
//! with the same commands as the app.
//!
//! - **The collection** is the folder open, as its reading lists it: the session follows
//!   the reading's `Listed` chunks and its failure ([`super::Inbox::Listing`]). Or it is the
//!   photos opened alone, as a client names them (`Source::Photos`), in its order, their
//!   edit states read by a job of their own ([`super::Inbox::Classified`]). With neither,
//!   the collection is empty.
//! - **The active photo** is the photo open or opening. Opening one outside the selection
//!   selects it alone; opening a selected one keeps the selection. A command whose result
//!   leaves the active photo out closes it, as [`super::Command::Close`] does, so the
//!   selection holds it whenever there is one: a client that knows its sort makes the
//!   nearest selected photo active next, as the Browser's Ctrl-click does.
//! - **A folder listed again,** opened anew while it is open, keeps the selection by
//!   photo, the photos gone from it dropped once its listing is done; another folder, or
//!   none, keeps the active photo alone.
//! - **The sort** is a client's until the protocol carries it (`docs/gui.md`, The browser
//!   bar), so a range comes with the client's order of the collection
//!   ([`SelectionCommand::Range`]).
//! - **One earlier selection,** its photos and its active photo as they stood just before
//!   the last change of the photos selected, by a [`SelectionCommand`] or an open selecting
//!   a photo alone, which [`SelectionCommand::Previous`] puts back (decision 90): view state,
//!   never a history step nor saved, cleared as another folder or collection opens or it
//!   closes.
//! - **One fact,** [`super::Event::Selection`], once a turn where the selection, the
//!   active photo or whether there is an earlier selection changed (decision 59).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use super::library::EditState;
use super::{Engine, Event, LibraryEvent, SessionError};

/// What a client asks of the selection (decision 37). Each names photos by their paths:
/// one named by `Set`, `Add` or `Range`'s `to` must be one of the collection's, the folder
/// open's as listed so far, or the active photo, else the command is refused, as
/// [`SessionError::NotInCollection`], and changes nothing; any other photo `Remove` or
/// `Range`'s `order` names is passed over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionCommand {
    /// These photos in place of the selection; the active photo closes unless it is among
    /// them.
    Set(Vec<PathBuf>),
    /// Adds a photo to the selection; one selected already, nothing.
    Add(PathBuf),
    /// Takes a photo out of the selection, the active photo closing should it be that one;
    /// one not selected, nothing.
    Remove(PathBuf),
    /// The photos from the active photo to `to`, as `order`, the client's sort of the
    /// collection, lists them, in place of the selection, as a Shift-click selects them:
    /// the active photo stays. With no active photo, or one `order` lacks, `to` alone with
    /// it; a photo of `order` not in the collection is passed over.
    Range { to: PathBuf, order: Vec<PathBuf> },
    /// Every photo of the collection, the active photo staying.
    All,
    /// None: the active photo closes (`docs/gui.md`, The pointer: Select None leaves no
    /// active photo).
    Clear,
    /// The photos of the collection not selected, in place of the selection: the active
    /// photo, selected before, closes.
    Invert,
    /// The photos of the collection with no edit, in place of the selection, by the edit
    /// state the Browser's pencil and its Edited sort read: the reading's, the open photo's
    /// as its edit stands, a change earlier in the batch included. One open for viewing
    /// only, one a newer Apertrust wrote, or one whose sidecar cannot be read, counts as
    /// edited. The active photo closes unless it is among them.
    Unedited,
    /// The selection as it stood just before its last change, its photos and its active
    /// photo, which opens as [`super::Command::Open`] opens one where it is not the photo
    /// open; the selection it replaces becomes the earlier one, so a second goes forward
    /// again. A photo no longer in the collection is left out, and where the active photo
    /// is, the nearest of those put back in the listing's order is active, the later of
    /// two as near, or none. With no earlier selection, refused, as
    /// [`SessionError::NoPreviousSelection`], changing nothing.
    Previous,
}

/// The selection and the active photo, the whole of [`super::Fact::Selection`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    /// The photos selected: the collection's in the listing's order, each its folder's
    /// path as opened joined with its name, then the active photo where it is not one of
    /// them.
    pub photos: Vec<PathBuf>,
    /// The photo open or opening, as [`super::Command::Open`] named it.
    pub active: Option<PathBuf>,
    /// Whether there is an earlier selection [`SelectionCommand::Previous`] goes back to.
    pub previous: bool,
    /// How many commands had been applied: every one [`super::Session::send`] numbered up
    /// to this.
    pub through: u64,
}

/// What a [`SelectionCommand`] does to the active photo.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Active {
    Stays,
    /// Left out of the selection, it closes.
    Closes,
    /// Another photo, the one named, opens: Previous's.
    Opens(PathBuf),
}

/// The selection as the engine thread keeps it.
#[derive(Debug, Default)]
pub(super) struct State {
    /// The folder open, none with no folder.
    folder: Option<Folder>,
    /// The photos selected, by their absolute paths ([`key`]): the collection's, and the
    /// active photo; any other is dropped as the selection is told.
    selected: HashSet<PathBuf>,
    /// The selection as it stood just before the last change of the photos selected, or
    /// none: before any, and once another folder or collection opens or it closes.
    earlier: Option<Earlier>,
    /// The photos, the active photo and whether there is an earlier selection, as the last
    /// fact told them, or none told yet.
    told: (Vec<PathBuf>, Option<PathBuf>, bool),
    /// The open photo, by its absolute path, and whether its values last said it is
    /// edited: what `Unedited` reads of it while it is open, over its listed state, which
    /// stays as its sidecar holds it; none once it closes or another opens.
    live: Option<(PathBuf, bool)>,
    /// Whether the photos selected or the folder's listing changed since the last fact, which
    /// is worked out again only then, or when the active photo changes.
    changed: bool,
    /// Whether a `Previous` was taken since the last fact, which is then told though it
    /// tells what the last did, so a client waiting for its answer gets one: the earlier
    /// selection put back may be the selection as it stands, its other photos gone since.
    owed: bool,
    /// How many times a fact was worked out, for a test.
    #[cfg(test)]
    built: usize,
}

/// A selection as it stood, which [`SelectionCommand::Previous`] puts back.
#[derive(Debug)]
struct Earlier {
    /// The photos selected, by their absolute paths, in the collection's order then, each
    /// with its place in the whole listing then, none for one outside it.
    photos: Vec<(Option<usize>, PathBuf)>,
    /// The active photo, as [`super::Command::Open`] named it, and its place in the whole
    /// listing then, none for a photo opened alone outside the collection.
    active: Option<(PathBuf, Option<usize>)>,
}

/// One of the folder's photos, as its listings and the edit writer last found it.
#[derive(Debug)]
struct Entry {
    /// Its file's name in the folder; a photo opened alone's absolute path.
    name: PathBuf,
    /// Its edit state: the latest listing's row, or the edit writer's classification
    /// where that is the later observation; none yet for a photo held as it opened.
    edit: Option<EditState>,
    /// The number of the latest listing that brought it ([`Folder::listing`]); 0 for one
    /// no listing has brought yet, as a photo opened while the folder is listed.
    seen: u64,
    /// Its place in that listing.
    rank: usize,
    /// The number of the listing under way, or last done, when the edit writer last
    /// classified its sidecar, 0 for none: that listing's row, which may have read the
    /// sidecar before the write, leaves `edit` as it is; a later listing's sets it.
    classified: u64,
}

/// The folder open, with its photos: the collection, one list, so every command, the fact
/// and `Unedited` read the same photos. The photos opened alone are one too, with an empty
/// path, each named by its absolute path, listed once and done.
#[derive(Debug)]
struct Folder {
    /// The number of the library command that opened it, whose reading it follows.
    open: u64,
    /// Its path as last asked for, which the photos it reports are joined to.
    path: PathBuf,
    /// The same path, absolute ([`key`]).
    absolute: PathBuf,
    /// The number of its latest listing, from 1 as it opens, one more each time it is
    /// opened again.
    listing: u64,
    /// Whether that listing is under way.
    under_way: bool,
    /// How many photos the listing under way has brought.
    brought: usize,
    /// Its photos: those of the listing last done, in its order, then those the listing
    /// under way brought that it lacks and those opened while it is under way, as they
    /// came. A listing done keeps only the photos it brought, in its order.
    photos: Vec<Entry>,
    /// Each photo's place in `photos`, by its absolute path.
    places: HashMap<PathBuf, usize>,
}

impl Folder {
    fn new(open: u64, path: &Path) -> Folder {
        Folder {
            open,
            path: path.to_owned(),
            absolute: key(path),
            listing: 1,
            under_way: true,
            brought: 0,
            photos: Vec::new(),
            places: HashMap::new(),
        }
    }

    /// The photos at `paths`, opened alone by the library command numbered `open`, in their
    /// order, each once, their edit states not yet known.
    fn alone(open: u64, paths: &[PathBuf]) -> Folder {
        let mut folder = Folder {
            open,
            path: PathBuf::new(),
            absolute: PathBuf::new(),
            listing: 1,
            under_way: false,
            brought: 0,
            photos: Vec::new(),
            places: HashMap::new(),
        };
        for path in paths {
            let name = key(path);
            if !folder.holds(&name) {
                let rank = folder.photos.len();
                folder.push(Entry {
                    name,
                    edit: None,
                    seen: 1,
                    rank,
                    classified: 0,
                });
            }
        }
        folder
    }

    /// The folder opened again: a new listing under way, its photos staying until it is
    /// done.
    fn list_again(&mut self) {
        self.listing += 1;
        self.under_way = true;
        self.brought = 0;
    }

    /// `photos`, a chunk of the listing under way: each in the collection, as that listing
    /// found it but where the edit writer classified it since that listing began.
    fn extend(&mut self, photos: impl IntoIterator<Item = (PathBuf, EditState)>) {
        let listing = self.listing;
        for (name, edit) in photos {
            let rank = self.brought;
            self.brought += 1;
            match self.places.get(&self.absolute.join(&name)) {
                Some(&at) => {
                    let entry = &mut self.photos[at];
                    if entry.classified != listing {
                        entry.edit = Some(edit);
                    }
                    entry.seen = listing;
                    entry.rank = rank;
                }
                None => self.push(Entry {
                    name,
                    edit: Some(edit),
                    seen: listing,
                    rank,
                    classified: 0,
                }),
            }
        }
    }

    fn push(&mut self, entry: Entry) {
        let photo = self.absolute.join(&entry.name);
        self.places.insert(photo, self.photos.len());
        self.photos.push(entry);
    }

    /// The listing under way done: the photos it brought, in its order, alone.
    fn done(&mut self) {
        let listing = self.listing;
        self.photos.retain(|entry| entry.seen == listing);
        self.photos.sort_by_key(|entry| entry.rank);
        self.places.clear();
        for (at, entry) in self.photos.iter().enumerate() {
            self.places.insert(self.absolute.join(&entry.name), at);
        }
        self.under_way = false;
    }

    /// The listing under way failed or was cancelled: the folder holds no photo, as the
    /// Browser shows it.
    fn fail(&mut self) {
        self.photos.clear();
        self.places.clear();
        self.under_way = false;
    }

    /// Whether the photo at `key`, an absolute path, is one of the collection's.
    fn holds(&self, key: &Path) -> bool {
        self.places.contains_key(key)
    }

    /// Whether the listing under way may bring the photo at `key` yet: one in the folder
    /// that it has not brought.
    fn may_bring(&self, key: &Path) -> bool {
        self.under_way && key.parent() == Some(self.absolute.as_path())
    }

    /// The photo at `key`, opened or selected while the listing under way may bring it
    /// yet, in the collection until that listing is done without it.
    fn hold_for_now(&mut self, key: &Path) {
        if self.holds(key) || !self.may_bring(key) {
            return;
        }
        // A listing names its photos in UTF-8: one whose name is not is never brought.
        if let Some(name) = key.file_name().and_then(|name| name.to_str()) {
            self.push(Entry {
                name: name.into(),
                edit: None,
                seen: 0,
                rank: 0,
                classified: 0,
            });
        }
    }

    /// The photo at `key`, held for now, whose open failed: out of the collection, unless a
    /// listing has brought it.
    fn release(&mut self, key: &Path) {
        let Some(&at) = self.places.get(key) else {
            return;
        };
        if self.photos[at].seen != 0 {
            return;
        }
        self.photos.remove(at);
        self.places.clear();
        for (at, entry) in self.photos.iter().enumerate() {
            self.places.insert(self.absolute.join(&entry.name), at);
        }
    }

    /// The photo at `key` as the edit writer classified its sidecar after its last write,
    /// `read`: the later observation than the listing under way's row.
    fn classify(&mut self, key: &Path, read: EditState) {
        self.hold_for_now(key);
        if let Some(&at) = self.places.get(key) {
            let entry = &mut self.photos[at];
            entry.edit = Some(read);
            entry.classified = self.listing;
        }
    }

    /// Each photo's absolute path, name and edit state, none yet for one held as it
    /// opened, in the collection's order.
    fn entries(&self) -> impl Iterator<Item = (PathBuf, &Path, Option<&EditState>)> + '_ {
        (self.photos.iter()).map(|entry| {
            (
                self.absolute.join(&entry.name),
                entry.name.as_path(),
                entry.edit.as_ref(),
            )
        })
    }
}

/// A photo's path made absolute, without asking the disk, so a path relative to the
/// folder the process started in names the same photo as its absolute one.
fn key(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_owned())
}

impl State {
    /// The folder at `path` opened by the library command numbered `open`. The folder open
    /// again is listed anew, the selection and the earlier one kept until its listing is
    /// done; another keeps the active photo alone ([`State::fact`]), with no earlier
    /// selection.
    pub(super) fn folder_opened(&mut self, open: u64, path: &Path) {
        self.changed = true;
        match &mut self.folder {
            Some(folder) if folder.absolute == key(path) => {
                // Its photos as now spelled, the same photos; one selected that no listing
                // has brought, as one opened alone in it, held until this listing is done.
                folder.open = open;
                folder.path = path.to_owned();
                folder.list_again();
                for photo in &self.selected {
                    folder.hold_for_now(photo);
                }
            }
            _ => {
                self.folder = Some(Folder::new(open, path));
                self.selected.clear();
                self.earlier = None;
            }
        }
    }

    /// The photos at `paths` opened alone by the library command numbered `open`: the
    /// collection in their order, in place of the folder or the photos before, the active
    /// photo staying selected alone, with no earlier selection. Each one's edit state is
    /// unknown until their job reads it ([`State::found`]).
    pub(super) fn photos_alone(&mut self, open: u64, paths: &[PathBuf]) {
        self.changed = true;
        self.folder = Some(Folder::alone(open, paths));
        self.selected.clear();
        self.earlier = None;
    }

    /// The folder closed: the active photo stays selected alone, with no earlier selection.
    pub(super) fn folder_closed(&mut self) {
        self.changed = true;
        self.folder = None;
        self.selected.clear();
        self.earlier = None;
    }

    /// What the reading of the folder open found: a chunk of its photos, the last done, or
    /// its failure, which leaves it no photo. A listing of the folder opened again replaces
    /// the photos listed before once it is done, the photos selected that it lacks dropped;
    /// the photos it brings count among the folder's as they come. What an earlier open's
    /// reading sends is dropped.
    pub(super) fn listing(&mut self, event: &LibraryEvent) {
        let Some(folder) = (self.folder.as_mut()).filter(|folder| folder.open == event.open())
        else {
            return;
        };
        self.changed = true;
        match event {
            LibraryEvent::Listed { photos, done, .. } => {
                let listed =
                    (photos.iter()).map(|photo| (PathBuf::from(&photo.name), photo.edit.clone()));
                folder.extend(listed);
                if *done {
                    folder.done();
                }
            }
            LibraryEvent::Failed { .. } => folder.fail(),
            _ => return,
        }
        // A photo of another folder, as the active one opened alone, is the next fact's to
        // keep or drop, which knows the active photo.
        let elsewhere = |photo: &PathBuf| photo.parent() != Some(folder.absolute.as_path());
        (self.selected).retain(|photo| folder.holds(photo) || elsewhere(photo));
    }

    /// The edit states of the photos opened alone by the library command numbered `open`, as
    /// their job read them, each by its path: each its state, as a listing's row is, but
    /// where the edit writer classified it since, the later observation; dropped where
    /// another folder or photos have been opened since.
    pub(super) fn found(&mut self, open: u64, photos: &[(PathBuf, EditState)]) {
        let Some(folder) = (self.folder.as_mut()).filter(|folder| folder.open == open) else {
            return;
        };
        self.changed = true;
        for (path, edit) in photos {
            if let Some(&at) = folder.places.get(&key(path)) {
                let entry = &mut folder.photos[at];
                if entry.classified != folder.listing {
                    entry.edit = Some(edit.clone());
                }
            }
        }
    }

    /// The photo at `path`, open, has an edit or none now, as its values say, so Unedited
    /// reads it so while it is open, as the Browser's pencil does, over its listed state,
    /// which stays as its sidecar holds it.
    pub(super) fn edited(&mut self, path: &Path, edited: bool) {
        self.live = Some((key(path), edited));
    }

    /// The photo at `path`, closed, as the edit writer classified its sidecar after its last
    /// write, `read`, asked while the folder's open was `open`: its listed state from now
    /// on, and that of a listing under way; dropped where the folder has been opened again
    /// or another opened since, whose listing reads the sidecar itself.
    pub(super) fn classified(&mut self, open: u64, path: &Path, read: EditState) {
        if let Some(folder) = (self.folder.as_mut()).filter(|folder| folder.open == open) {
            folder.classify(&key(path), read);
        }
    }

    /// The number of the open of the folder whose listing the selection follows, none with
    /// no folder.
    pub(super) fn folder_open(&self) -> Option<u64> {
        self.folder.as_ref().map(|folder| folder.open)
    }

    /// Whether the selection reads the edit of the photo at `path`: one of the collection's,
    /// or one the listing under way may bring yet.
    pub(super) fn reads(&self, path: &Path) -> bool {
        let photo = key(path);
        (self.folder.as_ref())
            .is_some_and(|folder| folder.holds(&photo) || folder.may_bring(&photo))
    }

    /// The open of the photo at `path` failed: held for now as it opened, it leaves the
    /// collection unless a listing has brought it.
    pub(super) fn open_failed(&mut self, path: &Path) {
        if let Some(folder) = &mut self.folder {
            self.changed = true;
            folder.release(&key(path));
        }
    }

    /// The open photo closed: what its values said no longer counts, its listed state,
    /// as its sidecar holds it, read in its place, as after a panic or a failed write.
    pub(super) fn closed(&mut self) {
        self.live = None;
    }

    /// The reading of the folder opened by the command numbered `open` cancelled, queued or
    /// running: the folder holds no photo, as one whose listing failed, and what the reading
    /// still sends is dropped. A listing done stays, as the Browser keeps it, and a reading
    /// an open has replaced changes nothing.
    pub(super) fn cancelled(&mut self, open: u64) {
        let under_way =
            (self.folder.as_ref()).is_some_and(|folder| folder.open == open && folder.under_way);
        if !under_way {
            return;
        }
        self.listing(&LibraryEvent::Failed {
            open,
            reason: "cancelled".to_owned(),
        });
        if let Some(folder) = &mut self.folder {
            // No command is numbered 0.
            folder.open = 0;
        }
    }

    /// `path` opens, the active photo from now on in place of `active`: selected alone
    /// unless it is selected already, as a click on it does, the selection before it then
    /// the earlier one. A photo outside the folder is selected only while active, so one
    /// `active` leaves goes first, as the next fact would drop it.
    pub(super) fn opening(&mut self, path: &Path, active: Option<&Path>) {
        self.changed = true;
        self.live = None;
        let before = self.selection(active);
        let named = active;
        let active = active.map(key);
        let photo = key(path);
        if let Some(folder) = &mut self.folder {
            folder.hold_for_now(&photo);
        }
        let folder = self.folder.as_ref();
        let held = |photo: &PathBuf| folder.is_some_and(|folder| folder.holds(photo));
        (self.selected).retain(|photo| held(photo) || active.as_ref() == Some(photo));
        // The active photo is selected whatever came before, as the fact tells it.
        if !self.selected.contains(&photo) && active.as_ref() != Some(&photo) {
            self.selected.clear();
        }
        self.selected.insert(photo);
        self.record(before, named, Some(path));
    }

    /// [`super::Command::Selection`], with `active` the photo open or opening: what becomes
    /// of the active photo, which closes where the selection now leaves it out. A photo
    /// named by `Set`, `Add` or `Range`'s `to` that is neither the collection's nor the
    /// active one refuses the command, which changes nothing, as `Previous` with no earlier
    /// selection is. A command that changes the photos selected leaves the selection before
    /// it as the earlier one; `Previous` swaps the two.
    pub(super) fn command(
        &mut self,
        command: SelectionCommand,
        active: Option<&Path>,
    ) -> Result<Active, SessionError> {
        self.changed = true;
        let before = self.selection(active);
        if command == SelectionCommand::Previous {
            return self.previous(before, active);
        }
        let closes = self.change(command, active)?;
        self.record(before, active, active.filter(|_| !closes));
        Ok(if closes {
            Active::Closes
        } else {
            Active::Stays
        })
    }

    /// What [`State::command`] does for any command but `Previous`: whether the selection
    /// now leaves the active photo out.
    fn change(
        &mut self,
        command: SelectionCommand,
        active: Option<&Path>,
    ) -> Result<bool, SessionError> {
        let active = active.map(key);
        // Selected whatever came before, as a folder opened or closed may have cleared it
        // since the last fact.
        self.selected.extend(active.clone());
        let collection =
            || (self.folder.iter()).flat_map(|folder| folder.entries().map(|(photo, ..)| photo));
        let selected = match command {
            SelectionCommand::Set(photos) => {
                let photos = (photos.iter()).map(|photo| self.named(photo, active.as_deref()));
                photos.collect::<Result<_, _>>()?
            }
            SelectionCommand::Add(photo) => {
                let photo = self.named(&photo, active.as_deref())?;
                self.selected.insert(photo);
                return Ok(false);
            }
            SelectionCommand::Remove(photo) => {
                self.selected.remove(&key(&photo));
                return Ok(active.is_some_and(|active| !self.selected.contains(&active)));
            }
            SelectionCommand::Range { to, order } => {
                let to = self.named(&to, active.as_deref())?;
                let order: Vec<PathBuf> = (order.iter()).map(|photo| key(photo)).collect();
                let at = |photo: &Path| order.iter().position(|listed| listed == photo);
                let mut range: HashSet<PathBuf> = match active.as_deref().and_then(at).zip(at(&to))
                {
                    Some((from, to)) => {
                        let (first, last) = (from.min(to), from.max(to));
                        let held = |photo: &&PathBuf| self.holds(photo, active.as_deref());
                        order[first..=last].iter().filter(held).cloned().collect()
                    }
                    None => HashSet::new(),
                };
                range.insert(to);
                range.extend(active.clone());
                range
            }
            SelectionCommand::All => collection().chain(active.clone()).collect(),
            SelectionCommand::Clear => HashSet::new(),
            SelectionCommand::Invert => {
                let unselected = |photo: &PathBuf| !self.selected.contains(photo);
                collection().filter(unselected).collect()
            }
            SelectionCommand::Unedited => {
                // The open photo as its values say, over its listed state; one whose edit
                // state is not known yet, held as it opened, is not taken as unedited.
                let live = self.live.as_ref();
                let unedited = |(photo, _, edit): &(PathBuf, &Path, Option<&EditState>)| match live
                {
                    Some((open, edited)) if open == photo => !edited,
                    _ => *edit == Some(&EditState::None),
                };
                (self.folder.iter())
                    .flat_map(Folder::entries)
                    .filter(unedited)
                    .map(|(photo, ..)| photo)
                    .collect()
            }
            // Taken by `command`.
            SelectionCommand::Previous => return Ok(false),
        };
        self.selected = selected;
        Ok(active.is_some_and(|active| !self.selected.contains(&active)))
    }

    /// The photos selected as the next fact would tell them, with `active` the photo open
    /// or opening, by their absolute paths: those of the collection, and the active photo.
    fn selection(&self, active: Option<&Path>) -> HashSet<PathBuf> {
        let active = active.map(key);
        let folder = self.folder.as_ref();
        let held = |photo: &&PathBuf| folder.is_some_and(|folder| folder.holds(photo));
        let mut photos: HashSet<PathBuf> = self.selected.iter().filter(held).cloned().collect();
        photos.extend(active);
        photos
    }

    /// The selection `before`, with `active` its active photo, as the earlier one, where the
    /// photos selected now, with `now` the active photo, differ from it: a change of the
    /// active photo alone, within the same photos, or nothing changed, keeps the earlier one,
    /// as does a change from no photo, as the first photo's open, so there is nothing to go
    /// back to until a selection is left.
    fn record(&mut self, before: HashSet<PathBuf>, active: Option<&Path>, now: Option<&Path>) {
        if !before.is_empty() && self.selection(now) != before {
            self.earlier = Some(self.earlier_of(before, active));
        }
    }

    /// `photos`, with `active` the active photo, as an earlier selection: the collection's in
    /// its order, for the nearest to be found among them.
    fn earlier_of(&self, photos: HashSet<PathBuf>, active: Option<&Path>) -> Earlier {
        let places = self.folder.as_ref().map(|folder| &folder.places);
        let place = |photo: &PathBuf| places.and_then(|places| places.get(photo).copied());
        let mut photos: Vec<(Option<usize>, PathBuf)> = (photos.into_iter())
            .map(|photo| (place(&photo), photo))
            .collect();
        // An active photo outside the collection, which has no place, last.
        photos.sort_by_key(|(at, _)| at.unwrap_or(usize::MAX));
        let active = active.map(|named| (named.to_owned(), place(&key(named))));
        Earlier { photos, active }
    }

    /// [`SelectionCommand::Previous`], with `before` the photos selected now and `active` the
    /// active photo: the earlier selection put back, the photos gone from the collection
    /// since left out, and this one the earlier one in its place.
    fn previous(
        &mut self,
        before: HashSet<PathBuf>,
        active: Option<&Path>,
    ) -> Result<Active, SessionError> {
        let Some(earlier) = self.earlier.take() else {
            return Err(SessionError::NoPreviousSelection);
        };
        self.owed = true;
        self.earlier = Some(self.earlier_of(before, active));
        let folder = self.folder.as_ref();
        let held = |photo: &PathBuf| folder.is_some_and(|folder| folder.holds(photo));
        // An active photo of the collection's then is put back while it still is one; one
        // outside it then, opened alone, as `Open` would open it.
        let restored = match &earlier.active {
            Some((named, None)) => Some(named.clone()),
            Some((named, Some(_))) if held(&key(named)) => Some(named.clone()),
            // Gone: the nearest of the photos put back in the whole listing then, the later
            // of two as near.
            Some((_, Some(at))) => (earlier.photos.iter())
                .filter_map(|(j, photo)| Some((j.as_ref().copied()?, photo)))
                .filter(|(_, photo)| held(photo))
                .min_by_key(|(j, _)| (j.abs_diff(*at), *j < *at))
                .map(|(_, photo)| self.named_in_folder(photo)),
            None => None,
        };
        let photos = (earlier.photos.into_iter())
            .map(|(_, photo)| photo)
            .filter(held);
        let photos: HashSet<PathBuf> = photos.chain(restored.as_deref().map(key)).collect();
        self.selected = photos;
        let active = active.map(key);
        Ok(match restored {
            Some(photo) if active.as_ref() != Some(&key(&photo)) => {
                // Another photo opens: what the open one's values said no longer counts.
                self.live = None;
                Active::Opens(photo)
            }
            Some(_) => Active::Stays,
            None if active.is_some() => Active::Closes,
            None => Active::Stays,
        })
    }

    /// The photo at `photo`, an absolute path, as the folder open names its photos: its
    /// path as last asked for, joined with its name.
    fn named_in_folder(&self, photo: &Path) -> PathBuf {
        let folder = self.folder.as_ref();
        let entry = folder.and_then(|folder| Some((folder, *folder.places.get(photo)?)));
        match entry {
            Some((folder, at)) => folder.path.join(&folder.photos[at].name),
            None => photo.to_owned(),
        }
    }

    /// The absolute path of `photo`, one of the collection's or the active photo; else
    /// [`SessionError::NotInCollection`].
    fn named(&self, photo: &Path, active: Option<&Path>) -> Result<PathBuf, SessionError> {
        let key = key(photo);
        if self.holds(&key, active) {
            Ok(key)
        } else {
            Err(SessionError::NotInCollection {
                photo: photo.to_owned(),
            })
        }
    }

    /// Whether the photo at `key`, an absolute path, is one of the collection's or the
    /// active photo.
    fn holds(&self, key: &Path, active: Option<&Path>) -> bool {
        active == Some(key) || self.folder.as_ref().is_some_and(|folder| folder.holds(key))
    }

    /// The selection, with `active` the photo open or opening, after `through` commands,
    /// where it, the active photo or whether there is an earlier selection changed since the
    /// last told, or a `Previous` was taken since. The active photo is selected whatever came
    /// before, as no command leaves it out but closes it; a photo selected that is neither the collection's nor the active
    /// one is dropped, as one opened alone once another opens or it closes.
    pub(super) fn fact(&mut self, active: Option<&Path>, through: u64) -> Option<Selection> {
        if !self.changed && self.told.1.as_deref() == active {
            return None;
        }
        self.changed = false;
        #[cfg(test)]
        {
            self.built += 1;
        }
        let active_key = active.map(key);
        let folder = self.folder.as_ref();
        let held = |photo: &PathBuf| folder.is_some_and(|folder| folder.holds(photo));
        self.selected
            .retain(|photo| held(photo) || active_key.as_ref() == Some(photo));
        self.selected.extend(active_key.clone());
        // The photos selected by their places, in the collection's order: as many as are
        // selected, however many are listed.
        let mut photos = Vec::new();
        if let Some(folder) = folder {
            let mut places: Vec<usize> = (self.selected.iter())
                .filter_map(|photo| folder.places.get(photo).copied())
                .collect();
            places.sort_unstable();
            photos.extend(
                places
                    .into_iter()
                    .map(|at| folder.path.join(&folder.photos[at].name)),
            );
        }
        if let (Some(active), Some(active_key)) = (active, &active_key)
            && !held(active_key)
        {
            photos.push(active.to_owned());
        }
        let now = (photos, active.map(Path::to_owned), self.earlier.is_some());
        if now == self.told && !self.owed {
            return None;
        }
        self.owed = false;
        self.told = now.clone();
        let (photos, active, previous) = now;
        Some(Selection {
            photos,
            active,
            previous,
            through,
        })
    }
}

impl Engine {
    /// Tells the selection whether the open photo is edited as its edit stands now, ahead
    /// of the batch's `Values`: before `Unedited` reads it, and as the photo closes, when a
    /// batch sends none of its values. Only for a photo the selection reads the edit of
    /// ([`State::reads`]).
    pub(super) fn settle_edited(&mut self) {
        let Some(open) = &mut self.open else {
            return;
        };
        if !self.selection.reads(&open.photo.path) {
            return;
        }
        let edited = (open.values(self.operations, &self.tools, self.applied)).edited;
        // Open for viewing only, a newer Apertrust's or a problem's, or with a newer
        // sidecar that cannot be read, it counts as edited.
        let edited = edited || open.view_only.is_some() || open.unreadable.is_some();
        self.selection.edited(&open.photo.path, edited);
    }

    /// The photos at `paths` opened alone by the library command numbered `open`: the
    /// selection's collection, their sidecars read as a listing reads them, by a job on
    /// darktable's `SystemBackground` queue with no progress, never by the edit writer, so
    /// an open or a save never waits for them; each state goes to the engine thread as it
    /// is read ([`super::Inbox::Classified`]), and `Unedited` reads them as a listing's once
    /// they have come, a photo whose state has not come being no unedited one. The reads run on a thread of their own that nothing joins, as a listing's
    /// calls into its folder do: a sidecar on a share that does not answer holds neither
    /// the job's worker nor the session's end, and a cancel returns at once, the thread
    /// ending once its read returns.
    pub(super) fn photos_alone(&mut self, open: u64, paths: &[PathBuf]) {
        self.selection.photos_alone(open, paths);
        let paths = paths.to_vec();
        let inbox = self.jobs.inbox.clone();
        let description = format!("reading the edits of {} photos alone", paths.len());
        #[cfg(test)]
        let hold = self.folder_hold.take();
        let job = self.jobs.add(open, super::Queue::SystemBackground, |_| {
            super::jobs::Job::new("alone", description, move |context| {
                let count = paths.len();
                let (read, reading) = mpsc::channel();
                // Each state goes to the engine thread as it is read, so one sidecar that
                // does not answer leaves only the photos after it unknown; the reader stops
                // once the job has gone, or the engine thread.
                let reader = std::thread::Builder::new()
                    .name("photos alone".to_owned())
                    .spawn(move || {
                        for path in paths {
                            #[cfg(test)]
                            stall::wait(&path);
                            let sidecar = super::sidecar::path_for(&path);
                            let edit = super::library::sidecar_state(&sidecar);
                            let photos = vec![(path, edit)];
                            let sent = inbox.send(super::Inbox::Classified { open, photos });
                            if sent.is_err() || read.send(()).is_err() {
                                return;
                            }
                        }
                    });
                if let Err(error) = reader {
                    // Their edit states stay unknown, so `Unedited` takes none of them.
                    tracing::warn!(target: "control", "cannot read the photos' edits: {error}");
                    return Ok(());
                }
                for _ in 0..count {
                    if context.wait(&reading).is_err() {
                        return Ok(());
                    }
                }
                // Held for a test once every state is sent.
                #[cfg(test)]
                if let Some(hold) = &hold {
                    hold.wait();
                }
                Ok(())
            })
        });
        self.alone = Some(job);
    }

    /// [`Command::CancelJob`](super::Command::CancelJob): the job cancelled where it can
    /// be; the folder's reading, queued or running, cancelled then, not one that had ended,
    /// leaves the folder with no photo, as one that failed, and the selection takes it so.
    /// One that had sent its last chunk or its failure already, which the client has or
    /// will have, changes nothing: that word reaches the selection after the cancel.
    pub(super) fn cancel_job(&mut self, job: super::JobId) -> Result<(), SessionError> {
        let accepted = self.jobs.cancel(job)?;
        if accepted
            && let Some((reading, ended)) = &self.folder
            && *reading == job
            && let Some((open, _)) = &self.shown
            && !ended.swap(true, std::sync::atomic::Ordering::AcqRel)
        {
            self.selection.cancelled(*open);
        }
        Ok(())
    }

    /// The open photo closing, by any path, a close, a failed write or a panic: its sidecar
    /// classified as a listing classifies it by the edit writer, after its last write, off
    /// the engine thread ([`Self::take_classified`]), its listed state from then on; what its
    /// values said no longer counts.
    pub(super) fn settle_closed(&mut self) {
        if let Some(open) = &self.open
            && self.selection.reads(&open.photo.path)
            && let Some(number) = self.selection.folder_open()
        {
            self.writer.classify(open.photo.path.clone(), number);
        }
        self.selection.closed();
    }

    /// What the edit writer has classified since, applied to the selection; with `wait`,
    /// once every sidecar asked for is classified, as before `Unedited` reads them.
    pub(super) fn take_classified(&mut self, wait: bool) {
        for (open, photo, read) in self.writer.take_classified(wait) {
            self.selection.classified(open, &photo, read);
        }
    }

    /// The photo open or opening: the newest open's while one is under way.
    pub(super) fn active(&self) -> Option<&Path> {
        match (&self.opening, &self.open) {
            (Some(opening), _) => Some(&opening.path),
            (None, Some(open)) => Some(&open.photo.path),
            (None, None) => None,
        }
    }

    /// The turn's `Selection`, where the selection or the active photo changed in it,
    /// after its other `events`, the latest `Values` among which says whether the open
    /// photo is edited.
    pub(super) fn selection_event(&mut self, events: &[Event]) -> Option<Event> {
        let values = events.iter().rev().find_map(|event| match event {
            Event::Values(values) => Some(values),
            _ => None,
        });
        match (values, &self.open) {
            (Some(values), Some(open)) => {
                let edited = values.edited || open.view_only.is_some() || open.unreadable.is_some();
                self.selection.edited(&open.photo.path, edited);
            }
            (_, None) => self.selection.closed(),
            (None, Some(_)) => {}
        }
        let active = self.active().map(Path::to_owned);
        let fact = self.selection.fact(active.as_deref(), self.applied);
        fact.map(Event::Selection)
    }
}

/// A test's hook: a read of the sidecar of a photo held here stops before it reads, as a
/// call into a share that does not answer would, until the photo is released.
#[cfg(test)]
pub(super) mod stall {
    use std::path::{Path, PathBuf};
    use std::sync::mpsc::{Receiver, Sender, channel};
    use std::sync::{Condvar, Mutex, PoisonError};

    /// Each photo held, and what hears a read reach it.
    static HELD: Mutex<Vec<(PathBuf, Sender<()>)>> = Mutex::new(Vec::new());
    static RELEASED: Condvar = Condvar::new();

    /// Holds every read of `photo`'s sidecar until [`release`]; what this returns hears
    /// each read that reaches it.
    pub(in super::super) fn hold(photo: &Path) -> Receiver<()> {
        let (reached, reaches) = channel();
        HELD.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((photo.to_owned(), reached));
        reaches
    }

    pub(in super::super) fn release(photo: &Path) {
        HELD.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|(held, _)| held != photo);
        RELEASED.notify_all();
    }

    /// Waits while `photo` is held.
    pub(super) fn wait(photo: &Path) {
        let mut held = HELD.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((_, reached)) = held.iter().find(|(held, _)| held == photo) {
            let _ = reached.send(());
        }
        while held.iter().any(|(held, _)| held == photo) {
            held = RELEASED.wait(held).unwrap_or_else(PoisonError::into_inner);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::library::Photo;
    use super::*;

    /// The folder the tests list, which no test reads.
    fn folder() -> PathBuf {
        key(Path::new("selection-tests"))
    }

    fn at(name: &str) -> PathBuf {
        folder().join(name)
    }

    fn photo(name: &str, edit: EditState) -> Photo {
        Photo {
            name: name.to_owned(),
            size: 0,
            modified: None,
            edit,
        }
    }

    fn listed(open: u64, photos: &[(&str, EditState)], done: bool) -> LibraryEvent {
        LibraryEvent::Listed {
            open,
            photos: (photos.iter())
                .map(|(name, edit)| photo(name, edit.clone()))
                .collect(),
            done,
        }
    }

    /// A state with the folder open as the library command numbered 1, its photos A to D
    /// listed in two chunks, C edited.
    fn listed_abcd() -> State {
        let mut state = State::default();
        state.folder_opened(1, &folder());
        let none = EditState::None;
        let first = [("A.ARW", none.clone()), ("B.ARW", none.clone())];
        state.listing(&listed(1, &first, false));
        let second = [("C.ARW", EditState::Edited), ("D.ARW", none)];
        state.listing(&listed(1, &second, true));
        state
    }

    /// The selection told, after its change, by name.
    fn told(state: &mut State, active: Option<&Path>) -> Vec<String> {
        let selection = state.fact(active, 0).expect("the selection changed");
        assert_eq!(selection.active.as_deref(), active);
        (selection.photos.iter())
            .map(|photo| photo.file_name().unwrap().to_string_lossy().into_owned())
            .collect()
    }

    /// Whether `command`, taken, closes the active photo.
    fn run(state: &mut State, command: SelectionCommand, active: Option<&Path>) -> bool {
        state.command(command, active).expect("taken") == Active::Closes
    }

    #[test]
    fn each_command_sets_the_selection_in_the_listing_s_order() {
        let mut state = listed_abcd();
        let a = at("A.ARW");
        let active = Some(a.as_path());
        state.opening(&a, None);
        assert_eq!(told(&mut state, active), ["A.ARW"]);
        // Added and removed, the active photo staying.
        assert!(!run(&mut state, SelectionCommand::Add(at("D.ARW")), active));
        assert!(!run(&mut state, SelectionCommand::Add(at("C.ARW")), active));
        assert_eq!(told(&mut state, active), ["A.ARW", "C.ARW", "D.ARW"]);
        assert!(!run(
            &mut state,
            SelectionCommand::Remove(at("C.ARW")),
            active
        ));
        assert_eq!(told(&mut state, active), ["A.ARW", "D.ARW"]);
        // Set, with the active photo among them.
        let set = SelectionCommand::Set(vec![at("B.ARW"), a.clone()]);
        assert!(!run(&mut state, set, active));
        assert_eq!(told(&mut state, active), ["A.ARW", "B.ARW"]);
        // Every photo, then the same again: no fact.
        assert!(!run(&mut state, SelectionCommand::All, active));
        assert_eq!(
            told(&mut state, active),
            ["A.ARW", "B.ARW", "C.ARW", "D.ARW"]
        );
        assert!(!run(&mut state, SelectionCommand::All, active));
        assert_eq!(state.fact(active, 0), None);
    }

    /// Previous puts back the selection before the last change and swaps with it; the
    /// earlier active photo gone, the nearest of those put back is active, the later of two
    /// as near; one opened alone outside the collection comes back as it was.
    #[test]
    fn previous_puts_back_the_earlier_selection_and_swaps_with_it() {
        let mut state = listed_abcd();
        let (a, c) = (at("A.ARW"), at("C.ARW"));
        let refused = state.command(SelectionCommand::Previous, None);
        assert!(matches!(refused, Err(SessionError::NoPreviousSelection)));
        // The first photo opened, from no photo: nothing to go back to.
        state.opening(&c, None);
        assert!(!state.fact(Some(&c), 0).expect("told").previous);
        let set = SelectionCommand::Set(["B.ARW", "C.ARW", "D.ARW"].map(at).to_vec());
        run(&mut state, set, Some(&c));
        state.opening(&a, Some(&c));
        assert!(state.fact(Some(&a), 0).expect("told").previous);
        // B, C and D with C active, which opens again; then A alone again.
        let back = state.command(SelectionCommand::Previous, Some(&a));
        assert_eq!(back.ok(), Some(Active::Opens(c.clone())));
        assert_eq!(told(&mut state, Some(&c)), ["B.ARW", "C.ARW", "D.ARW"]);
        let forward = state.command(SelectionCommand::Previous, Some(&c));
        assert_eq!(forward.ok(), Some(Active::Opens(a.clone())));
        assert_eq!(told(&mut state, Some(&a)), ["A.ARW"]);
        // C gone from the folder listed again: B and D as near, D, the later, is active.
        state.folder_opened(2, &folder());
        let none = EditState::None;
        let photos = ["A.ARW", "B.ARW", "D.ARW"].map(|name| (name, none.clone()));
        state.listing(&listed(2, &photos, true));
        let back = state.command(SelectionCommand::Previous, Some(&a));
        assert_eq!(back.ok(), Some(Active::Opens(at("D.ARW"))));
        assert_eq!(told(&mut state, Some(&at("D.ARW"))), ["B.ARW", "D.ARW"]);
        // A photo opened alone, outside the collection, comes back as it was.
        let alone = key(Path::new("elsewhere")).join("E.ARW");
        state.opening(&alone, Some(&at("D.ARW")));
        state.opening(&a, Some(&alone));
        let back = state.command(SelectionCommand::Previous, Some(&a));
        assert_eq!(back.ok(), Some(Active::Opens(alone.clone())));
        assert_eq!(told(&mut state, Some(&alone)), ["E.ARW"]);
    }

    /// The earlier active photo gone, its nearest is measured in the whole listing then, not
    /// among the selected photos alone: of A, B and D with B active, A is one place from B
    /// and D two, so A, though both are next to B among the three.
    #[test]
    fn previous_measures_the_nearest_in_the_whole_listing() {
        let mut state = listed_abcd();
        let b = at("B.ARW");
        state.opening(&b, None);
        run(&mut state, SelectionCommand::Add(at("A.ARW")), Some(&b));
        run(&mut state, SelectionCommand::Add(at("D.ARW")), Some(&b));
        assert_eq!(told(&mut state, Some(&b)), ["A.ARW", "B.ARW", "D.ARW"]);
        assert!(run(&mut state, SelectionCommand::Clear, Some(&b)));
        assert_eq!(told(&mut state, None), Vec::<String>::new());
        // B gone from the folder listed again.
        state.folder_opened(2, &folder());
        let photos = ["A.ARW", "C.ARW", "D.ARW"].map(|name| (name, EditState::None));
        state.listing(&listed(2, &photos, true));
        let back = state.command(SelectionCommand::Previous, None);
        assert_eq!(back.ok(), Some(Active::Opens(at("A.ARW"))));
        assert_eq!(told(&mut state, Some(&at("A.ARW"))), ["A.ARW", "D.ARW"]);
    }

    /// A Previous that puts back the selection as it stands, the other photos of the
    /// earlier one gone since, is told all the same, so a client waiting for its answer
    /// gets one.
    #[test]
    fn previous_putting_back_the_selection_as_it_stands_is_told() {
        let mut state = listed_abcd();
        let a = at("A.ARW");
        state.opening(&a, None);
        run(&mut state, SelectionCommand::Add(at("B.ARW")), Some(&a));
        assert_eq!(told(&mut state, Some(&a)), ["A.ARW", "B.ARW"]);
        // B gone from the folder listed again: A alone, and A alone before.
        state.folder_opened(2, &folder());
        let photos = ["A.ARW", "C.ARW", "D.ARW"].map(|name| (name, EditState::None));
        state.listing(&listed(2, &photos, true));
        assert_eq!(told(&mut state, Some(&a)), ["A.ARW"]);
        assert!(!run(&mut state, SelectionCommand::Previous, Some(&a)));
        let fact = state
            .fact(Some(&a), 7)
            .expect("told though nothing changed");
        assert_eq!((fact.through, fact.previous), (7, true));
        // Once: the next turn that changes nothing tells nothing.
        assert_eq!(state.fact(Some(&a), 8), None);
    }

    /// The earlier selection with no active photo, as a Ctrl-click on the active one leaves
    /// it, closes the photo open; after Select None it toggles with the empty selection; a
    /// change from no photo records nothing; photos opened alone, or the folder closed,
    /// leave none.
    #[test]
    fn previous_with_no_earlier_active_photo_closes_the_one_open() {
        let mut state = listed_abcd();
        let b = at("B.ARW");
        state.opening(&b, None);
        run(&mut state, SelectionCommand::Add(at("D.ARW")), Some(&b));
        assert!(run(
            &mut state,
            SelectionCommand::Remove(b.clone()),
            Some(&b)
        ));
        state.opening(&at("A.ARW"), None);
        let back = state.command(SelectionCommand::Previous, Some(&at("A.ARW")));
        assert_eq!(back.ok(), Some(Active::Closes));
        assert_eq!(told(&mut state, None), ["D.ARW"]);
        // Select None, then Previous: D back; again, none, the toggle going through the
        // empty selection Previous replaced.
        assert!(!run(&mut state, SelectionCommand::Clear, None));
        assert_eq!(told(&mut state, None), Vec::<String>::new());
        assert!(!run(&mut state, SelectionCommand::Previous, None));
        assert_eq!(told(&mut state, None), ["D.ARW"]);
        assert!(!run(&mut state, SelectionCommand::Previous, None));
        assert_eq!(told(&mut state, None), Vec::<String>::new());
        assert!(!run(&mut state, SelectionCommand::Previous, None));
        assert_eq!(told(&mut state, None), ["D.ARW"]);
        // Photos opened alone: none; from no photo, Select All records nothing either.
        state.photos_alone(2, &[at("A.ARW"), at("B.ARW")]);
        assert!(!state.fact(None, 0).expect("told").previous);
        run(&mut state, SelectionCommand::All, None);
        assert!(!state.fact(None, 0).expect("told").previous);
        run(&mut state, SelectionCommand::Remove(at("B.ARW")), None);
        assert!(state.fact(None, 0).expect("told").previous);
        state.folder_closed();
        let refused = state.command(SelectionCommand::Previous, None);
        assert!(matches!(refused, Err(SessionError::NoPreviousSelection)));
    }

    /// A range runs from the active photo to the one named in the client's order, which
    /// need not be the listing's, the active photo staying; with no active photo, the one
    /// named alone.
    #[test]
    fn a_range_runs_from_the_active_photo_in_the_client_s_order() {
        let mut state = listed_abcd();
        let c = at("C.ARW");
        state.opening(&c, None);
        let active = Some(c.as_path());
        // The client's sort: D, C, B, A, and a photo it shows that is not the folder's.
        let order = ["D.ARW", "C.ARW", "elsewhere.ARW", "B.ARW", "A.ARW"].map(at);
        let range = |to: &str| SelectionCommand::Range {
            to: at(to),
            order: order.to_vec(),
        };
        assert!(!run(&mut state, range("A.ARW"), active));
        assert_eq!(told(&mut state, active), ["A.ARW", "B.ARW", "C.ARW"]);
        // In place of the selection: back the other way.
        assert!(!run(&mut state, range("D.ARW"), active));
        assert_eq!(told(&mut state, active), ["C.ARW", "D.ARW"]);
        // No active photo: the one named alone.
        assert!(!run(&mut state, range("B.ARW"), None));
        assert_eq!(told(&mut state, None), ["B.ARW"]);
    }

    /// Whatever leaves the active photo out says so, for the session to close it: Select
    /// None, Invert, a Set without it, its Remove and Unedited on an edited one.
    #[test]
    fn a_command_leaving_the_active_photo_out_closes_it() {
        let mut state = listed_abcd();
        let b = at("B.ARW");
        let active = Some(b.as_path());
        state.opening(&b, None);
        assert!(!run(&mut state, SelectionCommand::Add(at("D.ARW")), active));
        // Inverted: the photos not selected, B not among them.
        assert!(run(&mut state, SelectionCommand::Invert, active));
        assert_eq!(told(&mut state, None), ["A.ARW", "C.ARW"]);
        state.opening(&b, None);
        assert_eq!(told(&mut state, active), ["B.ARW"]);
        assert!(run(&mut state, SelectionCommand::Clear, active));
        assert_eq!(told(&mut state, None), Vec::<String>::new());
        state.opening(&b, None);
        let set = SelectionCommand::Set(vec![at("A.ARW")]);
        assert!(run(&mut state, set, active));
        state.opening(&b, None);
        assert!(run(&mut state, SelectionCommand::Remove(b.clone()), active));
        // The active photo, edited now, as its values say, is not among the unedited.
        state.opening(&b, None);
        state.edited(&b, true);
        assert!(run(&mut state, SelectionCommand::Unedited, active));
        assert_eq!(told(&mut state, None), ["A.ARW", "D.ARW"]);
    }

    /// Unedited reads each photo's edit as the listing found it, the open photo's as its
    /// values last said; one whose sidecar cannot be read counts as edited until values,
    /// once it is repaired, say it has none.
    #[test]
    fn unedited_selects_the_photos_with_no_edit() {
        let mut state = State::default();
        state.folder_opened(1, &folder());
        let unreadable = EditState::Unreadable("not an .apxmp".to_owned());
        let photos = [
            ("A.ARW", EditState::None),
            ("B.ARW", EditState::Edited),
            ("C.ARW", unreadable),
            ("D.ARW", EditState::None),
        ];
        state.listing(&listed(1, &photos, true));
        let d = at("D.ARW");
        state.opening(&d, None);
        let active = Some(d.as_path());
        assert!(!run(&mut state, SelectionCommand::Unedited, active));
        assert_eq!(told(&mut state, active), ["A.ARW", "D.ARW"]);
        // B's edit reset, and C's sidecar repaired, as Discard History repairs one, its
        // values saying it has none.
        state.classified(1, &at("B.ARW"), EditState::None);
        state.classified(1, &at("C.ARW"), EditState::None);
        assert!(!run(&mut state, SelectionCommand::Unedited, active));
        assert_eq!(
            told(&mut state, active),
            ["A.ARW", "B.ARW", "C.ARW", "D.ARW"]
        );
    }

    /// Opening a photo outside the selection selects it alone; a selected one keeps the
    /// selection. One from outside the collection is selected alone too, and dropped once
    /// another opens.
    #[test]
    fn opening_a_photo_outside_the_selection_selects_it_alone() {
        let mut state = listed_abcd();
        let (a, c) = (at("A.ARW"), at("C.ARW"));
        state.opening(&a, None);
        run(&mut state, SelectionCommand::All, Some(&a));
        assert_eq!(told(&mut state, Some(&a)).len(), 4);
        // A selected photo made active: the selection stays.
        state.opening(&c, None);
        assert_eq!(told(&mut state, Some(&c)).len(), 4);
        // One from elsewhere: alone.
        let alone = key(Path::new("elsewhere")).join("E.ARW");
        state.opening(&alone, None);
        assert_eq!(told(&mut state, Some(&alone)), ["E.ARW"]);
        // A photo of the collection added: it comes first, the active one after.
        assert!(!run(
            &mut state,
            SelectionCommand::Add(c.clone()),
            Some(&alone)
        ));
        assert_eq!(told(&mut state, Some(&alone)), ["C.ARW", "E.ARW"]);
        // Selected, C keeps the selection as it opens, but for the photo from elsewhere.
        state.opening(&c, None);
        assert_eq!(told(&mut state, Some(&c)), ["C.ARW"]);
    }

    /// A photo neither the collection's nor active refuses the command whole.
    #[test]
    fn a_photo_outside_the_collection_is_refused() {
        let mut state = listed_abcd();
        let a = at("A.ARW");
        state.opening(&a, None);
        told(&mut state, Some(&a));
        let elsewhere = key(Path::new("elsewhere")).join("A.ARW");
        for command in [
            SelectionCommand::Add(elsewhere.clone()),
            SelectionCommand::Set(vec![at("B.ARW"), elsewhere.clone()]),
            SelectionCommand::Range {
                to: elsewhere.clone(),
                order: vec![a.clone(), elsewhere.clone()],
            },
        ] {
            let refused = state.command(command, Some(&a));
            assert!(
                matches!(&refused, Err(SessionError::NotInCollection { photo }) if *photo == elsewhere),
                "{refused:?}"
            );
        }
        assert_eq!(state.fact(Some(&a), 0), None);
    }

    /// The folder listed again keeps the selection by photo once its listing is done,
    /// dropping the photos gone from it; another folder, or none, keeps the active photo
    /// alone; an earlier open's chunks are dropped; a folder whose listing fails holds no
    /// photo.
    #[test]
    fn a_folder_listed_again_drops_the_photos_gone_from_it() {
        let mut state = listed_abcd();
        let a = at("A.ARW");
        let active = Some(a.as_path());
        state.opening(&a, None);
        let set = SelectionCommand::Set(["A.ARW", "B.ARW", "D.ARW"].map(at).to_vec());
        run(&mut state, set, active);
        assert_eq!(told(&mut state, active), ["A.ARW", "B.ARW", "D.ARW"]);
        // Listed again, B gone, E new: nothing changes until the listing is done.
        state.folder_opened(2, &folder());
        let none = EditState::None;
        state.listing(&listed(2, &[("A.ARW", none.clone())], false));
        // The first open's late chunk is not this listing's.
        state.listing(&listed(1, &[("Z.ARW", none.clone())], true));
        assert_eq!(state.fact(active, 0), None);
        let rest = [
            ("C.ARW", none.clone()),
            ("D.ARW", none.clone()),
            ("E.ARW", none),
        ];
        state.listing(&listed(2, &rest, true));
        assert_eq!(told(&mut state, active), ["A.ARW", "D.ARW"]);
        // Another folder: the active photo alone, outside it.
        state.folder_opened(3, &key(Path::new("another")));
        assert_eq!(told(&mut state, active), ["A.ARW"]);
        assert!(!run(&mut state, SelectionCommand::All, active));
        assert_eq!(state.fact(active, 0), None);
        // Closed: still the active photo alone.
        state.folder_closed();
        assert_eq!(state.fact(active, 0), None);
        // A folder whose listing fails holds no photo.
        state.folder_opened(4, &folder());
        state.listing(&listed(4, &[("A.ARW", EditState::None)], true));
        run(&mut state, SelectionCommand::All, active);
        state.folder_opened(5, &folder());
        state.listing(&LibraryEvent::Failed {
            open: 5,
            reason: "gone".to_owned(),
        });
        assert_eq!(state.fact(active, 0), None);
        assert!(run(&mut state, SelectionCommand::Invert, active));
        assert_eq!(told(&mut state, None), Vec::<String>::new());
    }

    /// A folder closed in the same batch as a command, before any fact, leaves the active
    /// photo selected: removing a photo not selected keeps it open.
    #[test]
    fn the_active_photo_stays_selected_before_the_next_fact() {
        let mut state = listed_abcd();
        let a = at("A.ARW");
        let active = Some(a.as_path());
        state.opening(&a, None);
        state.folder_closed();
        assert!(!run(
            &mut state,
            SelectionCommand::Remove(at("B.ARW")),
            active
        ));
        assert_eq!(told(&mut state, active), ["A.ARW"]);
        assert!(run(&mut state, SelectionCommand::Clear, active));
    }

    /// The open photo's edit as its values said is laid over a listing of its folder
    /// again, whose sidecar may not hold it yet; once it closes, the listing's stands.
    #[test]
    fn the_open_photo_s_edit_outlasts_a_listing_of_its_folder() {
        let mut state = listed_abcd();
        let a = at("A.ARW");
        let active = Some(a.as_path());
        state.opening(&a, None);
        state.edited(&a, true);
        let photos = ["A.ARW", "B.ARW"].map(|name| (name, EditState::None));
        state.folder_opened(2, &folder());
        state.listing(&listed(2, &photos, true));
        assert!(run(&mut state, SelectionCommand::Unedited, active));
        assert_eq!(told(&mut state, None), ["B.ARW"]);
        state.closed();
        state.folder_opened(3, &folder());
        state.listing(&listed(3, &photos, true));
        assert!(!run(&mut state, SelectionCommand::Unedited, None));
        assert_eq!(told(&mut state, None), ["A.ARW", "B.ARW"]);
    }

    /// A photo opened and edited before its folder opens keeps its edit through the
    /// folder's first listing, whose sidecar may not hold it yet.
    #[test]
    fn the_open_photo_s_edit_outlasts_its_folder_s_first_listing() {
        let mut state = State::default();
        let a = at("A.ARW");
        let active = Some(a.as_path());
        state.opening(&a, None);
        state.edited(&a, true);
        state.folder_opened(1, &folder());
        let photos = ["A.ARW", "B.ARW"].map(|name| (name, EditState::None));
        state.listing(&listed(1, &photos, false));
        assert!(run(&mut state, SelectionCommand::Unedited, active));
        assert_eq!(told(&mut state, None), ["B.ARW"]);
    }

    /// A photo edited and closed while its folder is listed again keeps its edit through
    /// that listing, as the edit writer classified its sidecar after its save; a listing
    /// begun after keeps its own reading.
    #[test]
    fn a_photo_closed_during_a_listing_keeps_its_edit_through_it() {
        let mut state = listed_abcd();
        let a = at("A.ARW");
        state.opening(&a, None);
        state.folder_opened(2, &folder());
        state.edited(&a, true);
        state.closed();
        state.classified(2, &a, EditState::Edited);
        let photos = ["A.ARW", "B.ARW"].map(|name| (name, EditState::None));
        state.listing(&listed(2, &photos, true));
        assert!(!run(&mut state, SelectionCommand::Unedited, None));
        assert_eq!(told(&mut state, None), ["B.ARW"]);
        // Listed again after the close: its own reading stands.
        state.folder_opened(3, &folder());
        state.listing(&listed(3, &photos, true));
        assert!(!run(&mut state, SelectionCommand::Unedited, None));
        assert_eq!(told(&mut state, None), ["A.ARW", "B.ARW"]);
    }

    /// The writer's classification after a close is the later observation: it wins over the
    /// row of the listing under way; one asked in an earlier open of the folder is dropped.
    #[test]
    fn the_writer_s_classification_wins_over_the_listing_under_way() {
        let mut state = listed_abcd();
        let a = at("A.ARW");
        state.opening(&a, None);
        state.folder_opened(2, &folder());
        state.closed();
        state.classified(2, &a, EditState::None);
        // A stale one, asked before this open.
        state.classified(1, &at("B.ARW"), EditState::None);
        let photos = [("A.ARW", EditState::Edited), ("B.ARW", EditState::Edited)];
        state.listing(&listed(2, &photos, true));
        assert!(!run(&mut state, SelectionCommand::Unedited, None));
        assert_eq!(told(&mut state, None), ["A.ARW"]);
    }

    /// A turn that changed neither the selection, the listing nor the active photo works
    /// nothing out, as during a drag; an edit alone changes none of them.
    #[test]
    fn the_selection_is_worked_out_only_when_it_may_have_changed() {
        let mut state = listed_abcd();
        let a = at("A.ARW");
        let active = Some(a.as_path());
        state.opening(&a, None);
        told(&mut state, active);
        let built = state.built;
        for edited in [true, false, true] {
            state.edited(&a, edited);
            assert_eq!(state.fact(active, 0), None);
        }
        assert_eq!(state.built, built);
        // The active photo closing is worked out, the photo staying selected.
        assert_eq!(told(&mut state, None), ["A.ARW"]);
        assert_eq!(state.built, built + 1);
    }

    /// The folder opened again under another spelling of its path, relative then absolute,
    /// is the same folder: its photos stay selected, told as now spelled at once.
    #[test]
    fn a_folder_opened_again_under_another_spelling_tells_its_photos_so() {
        let mut state = State::default();
        let relative = Path::new("selection-tests");
        state.folder_opened(1, relative);
        state.listing(&listed(1, &[("A.ARW", EditState::None)], true));
        let a = at("A.ARW");
        state.opening(&a, None);
        let told = state.fact(Some(&a), 0).expect("told");
        assert_eq!(told.photos, [relative.join("A.ARW")]);
        state.folder_opened(2, &folder());
        let told = state.fact(Some(&a), 0).expect("told again");
        assert_eq!(told.photos, std::slice::from_ref(&a));
    }

    /// While the folder is listed again, the photos the listing has brought can be
    /// selected, after those listed before; those listed before stay until it is done.
    #[test]
    fn a_listing_under_way_brings_its_photos_into_the_collection() {
        let mut state = listed_abcd();
        let a = at("A.ARW");
        let active = Some(a.as_path());
        state.opening(&a, None);
        assert!(!run(&mut state, SelectionCommand::Add(at("C.ARW")), active));
        told(&mut state, active);
        state.folder_opened(2, &folder());
        let none = EditState::None;
        state.listing(&listed(
            2,
            &[("A.ARW", none.clone()), ("E.ARW", none.clone())],
            false,
        ));
        assert!(!run(&mut state, SelectionCommand::Add(at("E.ARW")), active));
        assert_eq!(told(&mut state, active), ["A.ARW", "C.ARW", "E.ARW"]);
        // Done, C gone: E in its listed place.
        state.listing(&listed(2, &[("B.ARW", none)], true));
        assert_eq!(told(&mut state, active), ["A.ARW", "E.ARW"]);
    }

    /// A photo edited and closed during the folder's first listing, before the listing
    /// brings it, is read as edited when it comes.
    #[test]
    fn a_photo_closed_before_its_first_listing_brings_it_stays_edited() {
        let mut state = State::default();
        state.folder_opened(1, &folder());
        state.listing(&listed(1, &[("B.ARW", EditState::None)], false));
        let a = at("A.ARW");
        state.opening(&a, None);
        assert!(state.reads(&a), "the listing may bring it yet");
        state.edited(&a, true);
        state.closed();
        state.classified(1, &a, EditState::Edited);
        state.listing(&listed(1, &[("A.ARW", EditState::None)], true));
        assert!(!run(&mut state, SelectionCommand::Unedited, None));
        assert_eq!(told(&mut state, None), ["B.ARW"]);
        // Done, the listing reads no photo it lacks.
        assert!(!state.reads(&at("Z.ARW")));
    }

    /// A photo opened alone is selected only while it is active, whatever a turn holds:
    /// opening a folder's photo then the alone one again, in one turn or two, selects it
    /// alone; a listing's chunk in between leaves it to the active photo's rule.
    #[test]
    fn a_photo_opened_alone_is_selected_only_while_active() {
        let mut state = listed_abcd();
        let alone = key(Path::new("elsewhere")).join("E.ARW");
        let b = at("B.ARW");
        state.opening(&alone, None);
        assert!(!run(
            &mut state,
            SelectionCommand::Add(b.clone()),
            Some(&alone)
        ));
        // A chunk of a listing of the folder again keeps the active photo selected.
        state.folder_opened(2, &folder());
        state.listing(&listed(2, &[("B.ARW", EditState::None)], false));
        state.opening(&alone, Some(&alone));
        assert_eq!(told(&mut state, Some(&alone)), ["B.ARW", "E.ARW"]);
        // B, selected, opens and keeps the selection; E again, no longer active, alone.
        state.opening(&b, Some(&alone));
        state.opening(&alone, Some(&b));
        assert_eq!(told(&mut state, Some(&alone)), ["E.ARW"]);
    }

    /// A photo a listing under way brought and that was selected stays selected as the
    /// folder is opened again before it is done, until a listing done without it.
    #[test]
    fn a_photo_selected_from_a_listing_replaced_stays_until_one_is_done_without_it() {
        let mut state = listed_abcd();
        let a = at("A.ARW");
        let active = Some(a.as_path());
        state.opening(&a, None);
        state.folder_opened(2, &folder());
        let none = EditState::None;
        state.listing(&listed(2, &[("E.ARW", none.clone())], false));
        assert!(!run(&mut state, SelectionCommand::Add(at("E.ARW")), active));
        assert_eq!(told(&mut state, active), ["A.ARW", "E.ARW"]);
        // Opened again: E, not brought yet, stays selected, told once brought again.
        state.folder_opened(3, &folder());
        state.listing(&listed(3, &[("B.ARW", none.clone())], false));
        // Still ["A.ARW", "E.ARW"], E as the listing replaced brought it.
        assert_eq!(state.fact(active, 0), None);
        state.listing(&listed(3, &[("E.ARW", none.clone())], false));
        assert_eq!(state.fact(active, 0), None);
        // Done, in its listing's order; then a listing done without E drops it.
        state.listing(&listed(3, &[("A.ARW", none)], true));
        assert_eq!(told(&mut state, active), ["E.ARW", "A.ARW"]);
        state.folder_opened(4, &folder());
        let photos = [("A.ARW", EditState::None)];
        state.listing(&listed(4, &photos, true));
        assert_eq!(told(&mut state, active), ["A.ARW"]);
    }

    /// A photo whose edit was dropped unsaved, as after a panic, is read from the listing
    /// again, not from what its values said before.
    #[test]
    fn a_photo_abandoned_unsaved_is_read_as_its_listing_finds_it() {
        let mut state = State::default();
        state.folder_opened(1, &folder());
        state.listing(&listed(1, &[("B.ARW", EditState::None)], false));
        let a = at("A.ARW");
        state.opening(&a, None);
        state.edited(&a, true);
        state.closed();
        state.listing(&listed(1, &[("A.ARW", EditState::None)], true));
        assert!(!run(&mut state, SelectionCommand::Unedited, None));
        assert_eq!(told(&mut state, None), ["B.ARW", "A.ARW"]);
    }

    /// What the open photo's values say counts only while it is open: closed unsaved, as
    /// after a panic or a write that failed, it reads as its listing found it.
    #[test]
    fn a_photo_closed_unsaved_reads_as_listed() {
        let mut state = listed_abcd();
        let a = at("A.ARW");
        state.opening(&a, None);
        state.edited(&a, true);
        assert!(run(&mut state, SelectionCommand::Unedited, Some(&a)));
        state.opening(&a, None);
        state.edited(&a, true);
        state.closed();
        assert!(!run(&mut state, SelectionCommand::Unedited, None));
        assert_eq!(told(&mut state, None), ["A.ARW", "B.ARW", "D.ARW"]);
    }

    /// A reading cancelled leaves the folder with no photo, and what it still sends is
    /// dropped; the cancel of a reading an open has replaced changes nothing.
    #[test]
    fn a_reading_cancelled_leaves_no_photo_but_not_its_replacement_s() {
        let mut state = listed_abcd();
        let a = at("A.ARW");
        let active = Some(a.as_path());
        state.opening(&a, None);
        state.folder_opened(2, &folder());
        state.folder_opened(3, &folder());
        state.cancelled(2);
        let none = EditState::None;
        state.listing(&listed(
            3,
            &[("A.ARW", none.clone()), ("B.ARW", none.clone())],
            true,
        ));
        assert!(!run(&mut state, SelectionCommand::All, active));
        assert_eq!(told(&mut state, active), ["A.ARW", "B.ARW"]);
        state.folder_opened(4, &folder());
        state.cancelled(4);
        state.listing(&listed(4, &[("C.ARW", none)], true));
        assert!(!run(&mut state, SelectionCommand::All, active));
        assert_eq!(told(&mut state, active), ["A.ARW"]);
    }

    /// While the folder is listed again, a photo opened before any listing has brought it,
    /// then a listed one added and opened: both stay in the selection told, the first after
    /// the photos listed, so opening it again keeps the selection.
    #[test]
    fn a_photo_no_listing_has_brought_stays_in_the_selection_told() {
        let mut state = listed_abcd();
        state.folder_opened(2, &folder());
        let (e, b) = (at("E.ARW"), at("B.ARW"));
        state.opening(&e, None);
        assert_eq!(told(&mut state, Some(&e)), ["E.ARW"]);
        assert!(!run(&mut state, SelectionCommand::Add(b.clone()), Some(&e)));
        state.opening(&b, Some(&e));
        assert_eq!(told(&mut state, Some(&b)), ["B.ARW", "E.ARW"]);
        state.opening(&e, Some(&b));
        assert_eq!(told(&mut state, Some(&e)), ["B.ARW", "E.ARW"]);
    }

    /// A photo the selection tells is one a command may name: one opened while the folder
    /// is listed again, before any listing brought it, is in the collection until a
    /// listing is done without it.
    #[test]
    fn a_photo_told_selected_can_be_named_by_a_command() {
        let mut state = listed_abcd();
        state.folder_opened(2, &folder());
        let (e, b) = (at("E.ARW"), at("B.ARW"));
        state.opening(&e, None);
        assert!(!run(&mut state, SelectionCommand::Add(b.clone()), Some(&e)));
        state.opening(&b, Some(&e));
        assert_eq!(told(&mut state, Some(&b)), ["B.ARW", "E.ARW"]);
        let set = SelectionCommand::Set(vec![b.clone(), e.clone()]);
        assert!(!run(&mut state, set, Some(&b)));
        // Done without E: it goes.
        let none = EditState::None;
        state.listing(&listed(2, &[("B.ARW", none)], true));
        assert_eq!(told(&mut state, Some(&b)), ["B.ARW"]);
        let refused = state.command(SelectionCommand::Add(e.clone()), Some(&b));
        assert!(matches!(refused, Err(SessionError::NotInCollection { .. })));
    }

    /// The latest listing's row stands until a newer listing brings the photo: a refresh
    /// replaced before it is done leaves what it found.
    #[test]
    fn a_refresh_replaced_leaves_the_edit_state_it_found() {
        let mut state = listed_abcd();
        state.folder_opened(2, &folder());
        state.listing(&listed(2, &[("B.ARW", EditState::Edited)], false));
        state.folder_opened(3, &folder());
        state.listing(&listed(3, &[("A.ARW", EditState::None)], false));
        assert!(!run(&mut state, SelectionCommand::Unedited, None));
        assert_eq!(told(&mut state, None), ["A.ARW", "D.ARW"]);
    }

    /// A cancel after the listing is done changes nothing, as the Browser keeps the
    /// photos listed.
    #[test]
    fn a_cancel_after_the_listing_is_done_keeps_the_photos() {
        let mut state = listed_abcd();
        state.cancelled(1);
        assert!(!run(&mut state, SelectionCommand::All, None));
        assert_eq!(told(&mut state, None), ["A.ARW", "B.ARW", "C.ARW", "D.ARW"]);
    }

    /// Reopening the active photo keeps the selection, though a listing dropped it from
    /// the photos selected: the active photo is selected whatever came before.
    #[test]
    fn reopening_the_active_photo_keeps_the_selection() {
        let mut state = listed_abcd();
        let (a, e) = (at("A.ARW"), at("E.ARW"));
        state.opening(&a, None);
        assert!(!run(
            &mut state,
            SelectionCommand::Add(at("B.ARW")),
            Some(&a)
        ));
        // E opened alone in the folder, then listed again without it.
        state.opening(&e, Some(&a));
        state.folder_opened(2, &folder());
        let photos = ["A.ARW", "B.ARW"].map(|name| (name, EditState::None));
        state.listing(&listed(2, &photos, true));
        assert!(!run(
            &mut state,
            SelectionCommand::Add(at("B.ARW")),
            Some(&e)
        ));
        state.listing(&listed(2, &[], true));
        state.opening(&e, Some(&e));
        assert_eq!(told(&mut state, Some(&e)), ["B.ARW", "E.ARW"]);
    }

    /// A photo held as it opened while the folder is listed leaves the collection when its
    /// open fails; one a listing brought stays.
    #[test]
    fn a_photo_whose_open_failed_leaves_the_collection() {
        let mut state = listed_abcd();
        state.folder_opened(2, &folder());
        let (e, b) = (at("E.ARW"), at("B.ARW"));
        state.opening(&e, None);
        state.open_failed(&e);
        let refused = state.command(SelectionCommand::Add(e.clone()), None);
        assert!(matches!(refused, Err(SessionError::NotInCollection { .. })));
        state.opening(&b, None);
        state.open_failed(&b);
        assert!(!run(&mut state, SelectionCommand::Add(b), None));
    }

    /// A photo whose name is not UTF-8 is never one a listing brings: it is not held, and
    /// another photo's row is never read for it.
    #[cfg(unix)]
    #[test]
    fn a_name_not_in_utf_8_is_not_held() {
        use std::os::unix::ffi::OsStrExt;
        let mut state = State::default();
        state.folder_opened(1, &folder());
        let lossy = format!("A{}.ARW", char::REPLACEMENT_CHARACTER);
        state.listing(&listed(1, &[(lossy.as_str(), EditState::None)], false));
        let raw = folder().join(std::ffi::OsStr::from_bytes(b"A\xff.ARW"));
        state.opening(&raw, None);
        assert!(!run(&mut state, SelectionCommand::All, Some(&raw)));
        assert_eq!(state.fact(Some(&raw), 0).expect("told").photos.len(), 2);
    }

    /// A photo held as it opens has no edit state known yet: Unedited leaves it out until
    /// its values, a listing or the edit writer say it has no edit.
    #[test]
    fn a_photo_whose_edit_state_is_not_known_is_not_unedited() {
        let mut state = listed_abcd();
        state.folder_opened(2, &folder());
        let e = at("E.ARW");
        state.opening(&e, None);
        assert!(run(&mut state, SelectionCommand::Unedited, Some(&e)));
        assert_eq!(told(&mut state, None), ["A.ARW", "B.ARW", "D.ARW"]);
        state.listing(&listed(2, &[("E.ARW", EditState::None)], false));
        assert!(!run(&mut state, SelectionCommand::Unedited, None));
        assert_eq!(told(&mut state, None), ["A.ARW", "B.ARW", "D.ARW", "E.ARW"]);
    }

    /// While the folder is listed again, a photo listed before reads as the new listing
    /// found it once it brings it, in its place until that listing is done.
    #[test]
    fn a_listing_under_way_gives_the_photos_it_brought_its_edit_state() {
        let mut state = listed_abcd();
        state.folder_opened(2, &folder());
        let photos = [("B.ARW", EditState::Edited), ("C.ARW", EditState::None)];
        state.listing(&listed(2, &photos, false));
        assert!(!run(&mut state, SelectionCommand::Unedited, None));
        assert_eq!(told(&mut state, None), ["A.ARW", "C.ARW", "D.ARW"]);
    }
}
