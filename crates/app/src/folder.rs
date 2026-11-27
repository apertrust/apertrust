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

//! The open folder as the app last heard of it (`docs/events.md`, The library's events):
//! the part of the mirror (`mirror.rs`) that folds the session's `Event::Library`. It holds
//! no egui type.
//!
//! - **The number:** every `LibraryEvent` names the `Command::Library` it answers, and the app
//!   keeps the number of its latest open or close ([`FolderState::opening`],
//!   [`FolderState::choosing`], [`FolderState::closing`]); an event naming another is an
//!   earlier command's and dropped.
//! - **The root and the active folder:** a way in (Open Folder…, a dropped folder, a
//!   recent, Open This Folder, the command line) opens a folder that becomes Collections'
//!   root, at the top of its tree, and only it enters the recents ([`Outcome::Opened`]). A
//!   folder chosen in the tree is opened too, the library listing its photos, but the root,
//!   the subfolders listed and the recents stay as they were (`docs/gui.md`, Collections).
//! - **The photos** accumulate from the `Listed` chunks, in the listing's order, until one says
//!   `done`.
//! - **The tree's walk** comes in `Walked` batches after the root's own listing, in the
//!   number of the open that made the session's tree, which an open inside it keeps: each
//!   folder's raw count, and its subfolders where nothing has listed them yet, so the tree
//!   asks for none it has. A walk of an earlier tree is dropped ([`FolderState::counts`]).
//! - **A failure** is `Failed`, or the reading's job ending cancelled (the jobs window's
//!   Cancel): the open is then over, with its reason, or none for a cancel.
//! - **The thumbnails** come one `Thumbnail` at a time, kept by photo and size with the
//!   listing, so they go with the folder; each change raises [`FolderState::generation`],
//!   which the textures made of them compare (`thumbnails.rs`, `docs/gui.md`, Derived
//!   caches).
//! - **Photos alone,** one or several opened together by any route, are [`Folder::Alone`]:
//!   no folder is asked for, and the Browser holds just them ([`FolderState::alone`]).
//!   Their thumbnails come one `PhotoThumbnail` at a time, each naming its path, kept with
//!   them by path and size; other photos opened alone keep those of the paths still among
//!   them, and a folder's opening drops them all; one of a path no longer among them is
//!   dropped.
//! - **A cell's key,** what its thumbnails are kept, asked for and drawn by, is a folder's
//!   photo's name, or a photo alone's path, whole ([`CellKey`], [`FolderState::key`]).
//! - **The order** the Browser shows the photos in is the browser bar's sort
//!   ([`FolderState::sort`], [`sorted`]), worked out again when the sort changes, a chunk
//!   comes or the capture times do (`Taken`, once the listing is done); until they come, a
//!   photo sorts by its file's modified time (`docs/gui.md`, The browser bar). The listing
//!   keeps the folder's own order; the panel draws [`FolderState::order`].

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use apertrust_engine::jobs::{JobEvent, JobId, State};
use apertrust_engine::session::library::{self, DateTime, Photo, Thumbnail};
use apertrust_engine::session::{Event, LibraryEvent, Rating};
use apertrust_ui::components::{Sort, SortKey};

/// What is open.
#[derive(Debug, Default, PartialEq)]
pub enum Folder {
    /// Nothing: no folder asked for, or it was closed.
    #[default]
    None,
    /// Asked for, and nothing heard of it yet.
    Opening { path: PathBuf },
    /// Listed, or being listed.
    Open(Listing),
    /// It could not be opened, or its reading was cancelled (`reason` none).
    Failed {
        path: PathBuf,
        reason: Option<String>,
    },
    /// One photo, opened by any route (`docs/library.md`, One photo), or several opened
    /// together, from any folders: no folder listed, the Browser holding just them.
    Alone {
        /// The photos' paths, absolute, in the order opened, each once.
        paths: Vec<PathBuf>,
        /// Their cells, in the same order: each one's name, and its edit state as the
        /// session reports it. Nothing else is read of them, so their size is 0 and their
        /// modified time none.
        photos: Vec<Photo>,
        /// The active one among them, whose folder Open This Folder opens: the first, then
        /// the latest of them opened ([`FolderState::locate`]).
        active: usize,
        /// The thumbnails that have come, by path and size, as a listing keeps its own.
        thumbnails: HashMap<(PathBuf, u32), Kept>,
    },
}

/// What a cell's thumbnails are kept, asked for and drawn by: a folder's photo by its
/// name, a photo alone by its path, whole, so two paths never share one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CellKey {
    Name(String),
    Path(PathBuf),
}

/// An open folder.
#[derive(Debug, PartialEq)]
pub struct Listing {
    /// The folder's own name.
    pub name: String,
    /// Its path, as it was asked for.
    pub path: PathBuf,
    /// Its subfolders, as the file browser sorts them.
    pub subfolders: Vec<String>,
    /// The photos so far, in the listing's order.
    pub photos: Vec<Photo>,
    /// Whether the last chunk has come.
    pub done: bool,
    /// When each photo was taken, in the listing's order, once the reading has read them.
    pub taken: Option<Vec<Option<DateTime>>>,
    /// The thumbnails that have come, by photo and size: each picture, or `None` where the
    /// photo has none, with the [`FolderState::generation`] it came at.
    pub thumbnails: HashMap<(String, u32), Kept>,
}

/// A thumbnail as the mirror keeps it.
#[derive(Debug, PartialEq)]
pub struct Kept {
    pub image: Option<Thumbnail>,
    /// The generation it came at, which a texture made of it remembers.
    pub generation: u64,
}

impl Listing {
    /// The position of the photo at `path` in the folder, if it is one of its photos.
    pub fn position(&self, path: &Path) -> Option<usize> {
        let name = path.file_name()?;
        let parent = std::path::absolute(path.parent()?).ok()?;
        let mine = std::path::absolute(&self.path).ok()?;
        if parent != mine {
            return None;
        }
        self.photos
            .iter()
            .position(|photo| std::ffi::OsStr::new(&photo.name) == name)
    }
}

/// What a library event turned out to say, for the app to act on once.
#[derive(Debug, PartialEq)]
pub enum Outcome {
    /// The folder at this path opened: it goes to the recent folders.
    Opened(PathBuf),
    /// The folder at this path could not be opened, for this reason.
    Failed { path: PathBuf, reason: String },
}

/// The folder, and what is needed to tell its events from an earlier open's.
#[derive(Debug, Default)]
pub struct FolderState {
    pub folder: Folder,
    /// The number of the latest `Command::Library` the app sent.
    latest: u64,
    /// The job reading that command's folder, once the session has said so.
    job: Option<JobId>,
    /// Where the photo open is in the listing, found when the photo or the listing changes
    /// ([`FolderState::locate`]) and not scanned for each frame.
    active: Option<usize>,
    /// Raised whenever the folder or its thumbnails change.
    generation: u64,
    /// The browser bar's sort, which [`FolderState::order`] follows.
    sort: Option<Sort>,
    /// The listing's photos in the sort's order, by their place in the listing.
    order: Vec<usize>,
    /// Each photo's place in [`FolderState::order`], by its place in the listing.
    place: Vec<usize>,
    /// The subfolders of the folders in the root's tree, by path, as the library listed them:
    /// the root's and each chosen folder's with their opens (`LibraryEvent::Subfolders`), the
    /// others' as Collections' tree asked (`LibraryEvent::SubfoldersOf`).
    inside: HashMap<PathBuf, Result<Vec<String>, String>>,
    /// The folders inside it whose subfolders were asked for, so each is asked once an open.
    asked: HashSet<PathBuf>,
    /// Collections' root: the folder a way in opened, at the top of the tree, which a
    /// folder chosen in the tree keeps; none with nothing open, and once a way in failed.
    root: Option<PathBuf>,
    /// The session's tree, whose walk the counts come from: the first folder opened, kept
    /// while each later open lies inside it, as the session keeps it; none once closed.
    tree: Option<PathBuf>,
    /// The number of the open that made [`FolderState::tree`] the session's, whose walk's
    /// batches are taken in.
    walk: u64,
    /// The raws in each folder of the tree, by path: a folder's from its own listing once
    /// every photo is listed, the others' as the tree's walk counted them.
    counts: HashMap<PathBuf, usize>,
    /// The folders listed since the walk began, their subfolders or their photos, which
    /// are fresher than what the walk says of them: it leaves them.
    listed: HashSet<PathBuf>,
    /// Photo listings completed since the walk began, fresher than its counts.
    counted: HashSet<PathBuf>,
    /// Whether the latest open is a way in, which set the root, rather than a folder chosen
    /// in the tree.
    way_in: bool,
}

impl FolderState {
    /// The folder at `path` is asked for by the command numbered `number`, a way in: it is
    /// the root from now on, and what was listed of the tree before is dropped.
    pub fn opening(&mut self, path: PathBuf, number: u64) {
        self.root = Some(path.clone());
        self.way_in = true;
        self.ask(path, number);
    }

    /// The folder at `path`, the root or one inside its tree, is chosen in Collections'
    /// tree and asked for by the command numbered `number`: the Browser shows its photos,
    /// and the root and the subfolders listed stay. Those asked for are asked again, since
    /// the open cancels their listing.
    pub fn choosing(&mut self, path: PathBuf, number: u64) {
        if self.root.is_none() {
            self.opening(path, number);
        } else {
            self.way_in = false;
            self.ask(path, number);
        }
    }

    /// The folder at `path` is asked for by the command numbered `number`. Outside the
    /// session's tree it becomes the tree, as the session makes it, whose walk comes in that
    /// number: what was listed and counted of the tree before is dropped. The tree's root
    /// opened again is walked again, in that number, what it says replacing what is known;
    /// a folder inside the tree keeps the tree and what is known of it.
    fn ask(&mut self, path: PathBuf, number: u64) {
        let kept = self
            .tree
            .as_deref()
            .is_some_and(|tree| library::inside(tree, &path));
        if !kept {
            self.tree = Some(path.clone());
            self.inside.clear();
            self.counts.clear();
        }
        if !kept || self.tree.as_deref() == Some(path.as_path()) {
            self.walk = number;
            self.listed.clear();
            self.counted.clear();
        }
        self.latest = number;
        self.job = None;
        self.folder = Folder::Opening { path };
        self.asked.clear();
        self.generation += 1;
        self.reorder();
    }

    /// The photos at `paths`, absolute, are opened alone, together, the first active
    /// (`docs/library.md`, One photo), by the library command numbered `number`, which
    /// closes what was open as [`FolderState::closing`] does: nothing is listed, and the
    /// Browser holds just them, each once, in the order given, until a folder opens. The
    /// thumbnails of photos alone before that are among them still are kept, as their
    /// thumbnails from now on come in `number`.
    pub fn alone(&mut self, number: u64, paths: Vec<PathBuf>) {
        let mut unique: Vec<PathBuf> = Vec::with_capacity(paths.len());
        for path in paths {
            if !unique.contains(&path) {
                unique.push(path);
            }
        }
        let cell = |path: &PathBuf| Photo {
            name: path
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
            size: 0,
            modified: None,
            edit: library::EditState::None,
            rating: Default::default(),
            fields: Default::default(),
        };
        let photos = unique.iter().map(cell).collect();
        let before = std::mem::take(&mut self.folder);
        self.closing(number);
        let mut thumbnails = match before {
            Folder::Alone { thumbnails, .. } => thumbnails,
            _ => HashMap::new(),
        };
        thumbnails.retain(|(path, _), _| unique.contains(path));
        self.job = None;
        self.active = None;
        self.folder = Folder::Alone {
            paths: unique,
            photos,
            active: 0,
            thumbnails,
        };
        self.generation += 1;
        self.reorder();
    }

    /// The active one of the photos alone, and its folder, which Open This Folder opens;
    /// none in any other mode.
    pub fn alone_active(&self) -> Option<(&Path, &Path)> {
        alone_active(&self.folder)
    }

    /// The paths of the photos alone, in the order opened; none in any other mode.
    pub fn alone_paths(&self) -> &[PathBuf] {
        match &self.folder {
            Folder::Alone { paths, .. } => paths,
            _ => &[],
        }
    }

    /// The folder is closed by the command numbered `number`: nothing is open at once, and
    /// the events of the reading it stops are dropped.
    pub fn closing(&mut self, number: u64) {
        self.latest = number;
        self.job = None;
        self.folder = Folder::None;
        self.root = None;
        self.tree = None;
        self.counts.clear();
        self.listed.clear();
        self.counted.clear();
        self.generation += 1;
        self.reorder();
    }

    /// The number of the latest open or close, which the folder's thumbnails belong to.
    pub fn latest(&self) -> u64 {
        self.latest
    }

    /// Raised whenever the folder or its thumbnails change: a cache made of them, the
    /// thumbnails' textures, is current while it holds the same.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The thumbnails that have come of the open folder's photos, or of the photos alone,
    /// each with its cell's key ([`FolderState::key`]) and size.
    pub fn thumbnails(&self) -> impl Iterator<Item = (CellKey, u32, &Kept)> {
        let (names, paths) = match &self.folder {
            Folder::Open(listing) => (Some(&listing.thumbnails), None),
            Folder::Alone { thumbnails, .. } => (None, Some(thumbnails)),
            _ => (None, None),
        };
        let names = (names.into_iter().flatten())
            .map(|((name, size), kept)| (CellKey::Name(name.clone()), *size, kept));
        let paths = (paths.into_iter().flatten())
            .map(|((path, size), kept)| (CellKey::Path(path.clone()), *size, kept));
        names.chain(paths)
    }

    /// The thumbnail at `size` of the photo whose cell's key is `key`, if it has come:
    /// `Some(None)` where it has none.
    pub fn thumbnail(&self, key: &CellKey, size: u32) -> Option<&Kept> {
        match (&self.folder, key) {
            (Folder::Open(listing), CellKey::Name(name)) => {
                listing.thumbnails.get(&(name.clone(), size))
            }
            (Folder::Alone { thumbnails, .. }, CellKey::Path(path)) => {
                thumbnails.get(&(path.clone(), size))
            }
            _ => None,
        }
    }

    /// The key of the photo at `i` in the listing, which its thumbnails are kept, asked for
    /// and drawn by: a folder's photo's name, a photo alone's path.
    pub fn key(&self, i: usize) -> CellKey {
        match &self.folder {
            Folder::Alone { paths, .. } => CellKey::Path(paths[i].clone()),
            _ => CellKey::Name(self.photos()[i].name.clone()),
        }
    }

    /// The largest thumbnail that has come of the raw at `path`, if it is one of the open
    /// folder's photos or of the photos alone: what the Viewer shows at once as it
    /// opens (decision 77).
    pub fn largest_thumbnail(&self, path: &Path) -> Option<&Thumbnail> {
        let key = match &self.folder {
            Folder::Open(listing) => {
                listing.position(path)?;
                CellKey::Name(path.file_name()?.to_str()?.to_owned())
            }
            Folder::Alone { paths, .. } => {
                CellKey::Path(paths.iter().find(|alone| same(alone, path))?.clone())
            }
            _ => return None,
        };
        (self.thumbnails())
            .filter(|(photo, _, _)| *photo == key)
            .filter_map(|(_, size, kept)| Some((size, kept.image.as_ref()?)))
            .max_by_key(|(size, _)| *size)
            .map(|(_, image)| image)
    }

    /// The photo open, where it is in the listing.
    pub fn active(&self) -> Option<usize> {
        // Only a folder that is open, or a photo alone, has cells.
        self.active
            .filter(|_| matches!(self.folder, Folder::Open(_) | Folder::Alone { .. }))
    }

    /// The photos the Browser holds, in the listing's order: the open folder's, or the
    /// photo alone.
    pub fn photos(&self) -> &[Photo] {
        match &self.folder {
            Folder::Open(listing) => &listing.photos,
            Folder::Alone { photos, .. } => photos,
            _ => &[],
        }
    }

    /// The Browser's active cell: the photo open, where it is in the order.
    pub fn active_shown(&self) -> Option<usize> {
        self.active().and_then(|i| self.place.get(i).copied())
    }

    /// The listing's photos as the Browser shows them: each one's place in the listing,
    /// in the sort's order.
    pub fn order(&self) -> &[usize] {
        &self.order
    }

    /// Sorts the photos by `sort` from now on, at once where it changes.
    pub fn sort(&mut self, sort: Sort) {
        if self.sort != Some(sort) {
            self.sort = Some(sort);
            self.reorder();
        }
    }

    /// The photo at `path`, open, `editable` or for viewing only, has an edit or none now,
    /// as the session reports its values: its place in the listing says so, and the Edited
    /// sort moves it, without a new reading of the folder. An edit a newer Apertrust wrote
    /// stays as the reading found it, and so does a sidecar that could not be read while the
    /// photo is open for viewing only; once it opens editable, its history repaired as
    /// Discard History repairs it, its values say.
    pub fn edited(&mut self, path: &Path, edited: bool, editable: bool) {
        let Some(photo) = self.photo_mut(path) else {
            return;
        };
        let now = if edited {
            library::EditState::Edited
        } else {
            library::EditState::None
        };
        let follows = match photo.edit {
            library::EditState::None | library::EditState::Edited => true,
            library::EditState::Unreadable(_) => editable,
            library::EditState::ViewOnly(_) => false,
        };
        if follows && photo.edit != now {
            photo.edit = now;
            self.reorder();
        }
    }

    /// The photo at `path`, of the folder's listing or the photos alone.
    fn photo_mut(&mut self, path: &Path) -> Option<&mut Photo> {
        match &mut self.folder {
            Folder::Open(listing) => listing.position(path).map(|i| &mut listing.photos[i]),
            Folder::Alone { paths, photos, .. } => {
                let at = paths.iter().position(|alone| same(alone, path));
                at.map(|i| &mut photos[i])
            }
            _ => None,
        }
    }

    /// The photo at `path` is rated `rating` now, as the session's `Rated` says (decision
    /// 110): its cell shows it and the Rating sort moves it.
    pub fn rated(&mut self, path: &Path, rating: Rating) {
        if let Some(photo) = self.photo_mut(path)
            && photo.rating != rating
        {
            photo.rating = rating;
            self.generation += 1;
            self.reorder();
        }
    }

    /// Works the order out again, from the listing as it stands.
    fn reorder(&mut self) {
        let sort = self.sort.unwrap_or(Sort {
            key: SortKey::CaptureTime,
            descending: false,
        });
        self.order = match &self.folder {
            Folder::Open(listing) => sorted(&listing.photos, listing.taken.as_deref(), sort),
            Folder::Alone { photos, .. } => (0..photos.len()).collect(),
            _ => Vec::new(),
        };
        self.place = vec![0; self.order.len()];
        for (at, &photo) in self.order.iter().enumerate() {
            self.place[photo] = at;
        }
    }

    /// Finds `photo`, the photo open, in the listing again, for [`FolderState::active`]: to
    /// be called when the photo or the listing changes, which is when its place can. One of
    /// the photos alone becomes their active one, which stays while none of them is open.
    pub fn locate(&mut self, photo: Option<&Path>) {
        self.active = match (&mut self.folder, photo) {
            (Folder::Open(listing), Some(photo)) => listing.position(photo),
            (Folder::Alone { paths, active, .. }, Some(photo)) => {
                let at = paths.iter().position(|alone| same(alone, photo));
                *active = at.unwrap_or(*active);
                at
            }
            _ => None,
        };
    }

    /// Whether the subfolders of `folder`, inside the root's tree, are to be asked for:
    /// neither listed nor asked yet, the active folder's coming with its own open. Marks
    /// them asked. Asked whenever there is a root, so the tree is still listed after a
    /// folder chosen in it failed, which the session serves as it does any.
    pub fn ask_inside(&mut self, folder: &Path) -> bool {
        if self.root.is_none() {
            return false;
        }
        let listed = self.shown() == Some(folder) || self.inside.contains_key(folder);
        !listed && self.asked.insert(folder.to_owned())
    }

    /// The subfolders of folders inside the open one as the library listed them, by path,
    /// or why it could not.
    pub fn inside(&self) -> &HashMap<PathBuf, Result<Vec<String>, String>> {
        &self.inside
    }

    /// Try Again: what the library said of `folder`'s subfolders is forgotten, so the tree
    /// asks for them again.
    pub fn forget_inside(&mut self, folder: &Path) {
        self.inside.remove(folder);
        self.asked.remove(folder);
    }

    /// Collections' root, at the top of its tree: the folder a way in opened, while it is
    /// being read, open, or a folder chosen in its tree is.
    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// The root's photo count, once a listing of it has given every photo or the walk has
    /// counted it.
    #[cfg(test)]
    pub fn root_count(&self) -> Option<usize> {
        self.root
            .as_ref()
            .and_then(|root| self.counts.get(root).copied())
    }

    /// The raws in each folder of the tree as far as they are known, by path: the root's
    /// from its listing, the rest as the walk has reached them.
    pub fn counts(&self) -> &HashMap<PathBuf, usize> {
        &self.counts
    }

    /// The folder shown, the active folder, whose photos the Browser shows: the open one,
    /// or the one being opened; none for a photo alone, whose folder is not listed.
    pub fn shown(&self) -> Option<&Path> {
        match &self.folder {
            Folder::Open(listing) => Some(&listing.path),
            Folder::Opening { path } => Some(path),
            Folder::None | Folder::Failed { .. } | Folder::Alone { .. } => None,
        }
    }

    /// Takes in `event`, if it is the folder's: a `Library` event of the latest command, or
    /// the job that command started. Returns what the app is to do about it, if anything.
    pub fn apply(&mut self, event: &Event) -> Option<Outcome> {
        match event {
            Event::Library(LibraryEvent::Walked { open, folders, .. }) => {
                if self.tree.is_some() && *open == self.walk {
                    self.walked(folders);
                }
                None
            }
            Event::Library(event) if event.open() == self.latest => self.library(event),
            Event::Job { event, command } => {
                self.job_event(event, *command);
                None
            }
            Event::Rated { photo, rating } => {
                self.rated(photo, *rating);
                None
            }
            _ => None,
        }
    }

    fn library(&mut self, event: &LibraryEvent) -> Option<Outcome> {
        // What follows changes the folder, all but its subfolders and photos.
        if !matches!(
            event,
            LibraryEvent::Subfolders { .. }
                | LibraryEvent::SubfoldersOf { .. }
                | LibraryEvent::Listed { .. }
        ) {
            self.generation += 1;
        }
        match event {
            LibraryEvent::Opened { name, folder, .. } => {
                self.folder = Folder::Open(Listing {
                    name: name.clone(),
                    path: folder.clone(),
                    subfolders: Vec::new(),
                    photos: Vec::new(),
                    done: false,
                    taken: None,
                    thumbnails: HashMap::new(),
                });
                self.reorder();
                self.way_in.then(|| Outcome::Opened(folder.clone()))
            }
            LibraryEvent::Subfolders { names, .. } => {
                if let Folder::Open(listing) = &mut self.folder {
                    listing.subfolders = names.clone();
                    let listed = Ok(names.clone());
                    self.inside.insert(listing.path.clone(), listed);
                    self.listed.insert(listing.path.clone());
                }
                None
            }
            LibraryEvent::SubfoldersOf {
                folder,
                names,
                unreadable,
                ..
            } => {
                let listed = match unreadable {
                    Some(why) => Err(why.clone()),
                    None => Ok(names.clone()),
                };
                self.inside.insert(folder.clone(), listed);
                self.listed.insert(folder.clone());
                None
            }
            LibraryEvent::Listed { photos, done, .. } => {
                if let Folder::Open(listing) = &mut self.folder {
                    listing.photos.extend(photos.iter().cloned());
                    listing.done |= *done;
                    // Every folder of the tree listed whole has its count, fresher than the
                    // walk's.
                    let in_tree = (self.tree.as_deref())
                        .is_some_and(|tree| library::inside(tree, &listing.path));
                    if *done && in_tree {
                        self.counts
                            .insert(listing.path.clone(), listing.photos.len());
                        self.listed.insert(listing.path.clone());
                        self.counted.insert(listing.path.clone());
                    }
                    self.reorder();
                }
                None
            }
            // The times come once, for every photo listed; the panel is sorted by them once.
            LibraryEvent::Taken { times, .. } => {
                if let Folder::Open(listing) = &mut self.folder
                    && times.len() == listing.photos.len()
                {
                    listing.taken = Some(times.clone());
                    self.reorder();
                }
                None
            }
            LibraryEvent::Failed { reason, .. } => {
                let Folder::Opening { path } = &self.folder else {
                    return None;
                };
                let path = path.clone();
                self.folder = Folder::Failed {
                    path: path.clone(),
                    reason: Some(reason.clone()),
                };
                self.unroot();
                self.reorder();
                // A folder chosen in the tree that failed is no recent's to mark.
                self.way_in.then(|| Outcome::Failed {
                    path,
                    reason: reason.clone(),
                })
            }
            // Photos opened alone once the folder was closed stay.
            LibraryEvent::Closed { .. } => {
                if !matches!(self.folder, Folder::Alone { .. }) {
                    self.folder = Folder::None;
                    self.reorder();
                }
                None
            }
            LibraryEvent::Thumbnail {
                photo, size, image, ..
            } => {
                if let Folder::Open(listing) = &mut self.folder {
                    let kept = Kept {
                        image: image.clone(),
                        generation: self.generation,
                    };
                    listing.thumbnails.insert((photo.clone(), *size), kept);
                }
                None
            }
            // One of a photo no longer among those alone is an earlier set's.
            LibraryEvent::PhotoThumbnail {
                path, size, image, ..
            } => {
                if let Folder::Alone {
                    paths, thumbnails, ..
                } = &mut self.folder
                    && paths.contains(path)
                {
                    let kept = Kept {
                        image: image.clone(),
                        generation: self.generation,
                    };
                    thumbnails.insert((path.clone(), *size), kept);
                }
                None
            }
            // Taken in by `apply`, by the tree's number.
            LibraryEvent::Walked { .. } => None,
            // Import's, numbered by its own commands, never the folder's.
            LibraryEvent::SourceListed { .. }
            | LibraryEvent::SourceTaken { .. }
            | LibraryEvent::SourceFailed { .. }
            | LibraryEvent::SourceChecked { .. }
            | LibraryEvent::Volumes { .. }
            | LibraryEvent::ImportPreview { .. } => None,
        }
    }

    /// Takes in a batch of the tree's walk: each folder's raw count and its subfolders, so
    /// the tree asks for none it has, but for a folder listed since the walk began, whose
    /// own listing is fresher; a folder the walk could not read keeps no count, and its
    /// tree row offers to try again.
    fn walked(&mut self, folders: &[library::WalkedFolder]) {
        for walked in folders {
            if walked.unreadable.is_none() && !self.counted.contains(&walked.folder) {
                self.counts.insert(walked.folder.clone(), walked.raws);
            }
            if self.listed.contains(&walked.folder) {
                continue;
            }
            let listed = match &walked.unreadable {
                Some(why) => Err(why.clone()),
                None => Ok(walked.subfolders.clone()),
            };
            self.inside.insert(walked.folder.clone(), listed);
        }
    }

    /// A way in that failed or was cancelled leaves no root: nothing is open. A folder chosen
    /// in the tree that failed leaves the tree as it was.
    fn unroot(&mut self) {
        if self.way_in {
            self.root = None;
        }
    }

    /// The reading's job: noted when it is added for the latest command, and, ending
    /// cancelled before the folder is all listed, as the jobs window's Cancel does, the open
    /// is over.
    fn job_event(&mut self, event: &JobEvent, command: Option<u64>) {
        match event {
            JobEvent::Added { job, kind, .. } if *kind == "folder" => {
                if command == Some(self.latest) {
                    self.job = Some(*job);
                }
            }
            JobEvent::Ended { job, state, .. }
                if self.job == Some(*job) && *state == State::Cancelled =>
            {
                self.job = None;
                let path = match &self.folder {
                    Folder::Opening { path } => path.clone(),
                    Folder::Open(listing) if !listing.done => listing.path.clone(),
                    _ => return,
                };
                self.folder = Folder::Failed { path, reason: None };
                self.unroot();
                self.generation += 1;
                self.reorder();
            }
            _ => {}
        }
    }
}

/// The active one of `folder`'s photos alone, and its folder; none in any other mode.
pub fn alone_active(folder: &Folder) -> Option<(&Path, &Path)> {
    let Folder::Alone { paths, active, .. } = folder else {
        return None;
    };
    let photo = paths.get(*active)?;
    Some((photo, photo.parent()?))
}

/// Whether `alone`, an absolute path, is the photo at `photo`, which may be relative.
pub(crate) fn same(alone: &Path, photo: &Path) -> bool {
    std::path::absolute(photo).is_ok_and(|photo| photo == alone)
}

/// The photos' order by `sort`, each by its place in `photos`; `taken`, the photos' capture
/// times once they are known, else their files' modified times stand in (`docs/gui.md`, The
/// browser bar). Descending reverses the key, never the ties: photos equal by it stay in
/// shooting order, then the file browser's order of their names, and photos with no time
/// come last whichever way. Rating puts rejected photos first, then those with no stars,
/// then one to five (decision 110).
pub fn sorted(photos: &[Photo], taken: Option<&[Option<DateTime>]>, sort: Sort) -> Vec<usize> {
    let when: Vec<Option<u128>> = match taken {
        Some(times) => times.iter().map(|time| time.map(clock)).collect(),
        None => photos
            .iter()
            .map(|photo| {
                let since = photo.modified?.duration_since(UNIX_EPOCH).ok()?;
                Some(since.as_nanos())
            })
            .collect(),
    };
    let edited = |photo: &Photo| {
        matches!(
            photo.edit,
            library::EditState::Edited | library::EditState::ViewOnly(_)
        )
    };
    // Earlier first, and no time after any.
    let by_time = |a: usize, b: usize| match (when[a], when[b]) {
        (Some(a), Some(b)) => a.cmp(&b),
        (a, b) => b.is_some().cmp(&a.is_some()),
    };
    let direction = |order: Ordering| {
        if sort.descending {
            order.reverse()
        } else {
            order
        }
    };
    // Rejected, then no stars, then one to five.
    let rank = |photo: &Photo| match photo.rating {
        Rating { rejected: true, .. } => 0,
        Rating { stars, .. } => 1 + stars,
    };
    let mut order: Vec<usize> = (0..photos.len()).collect();
    order.sort_by(|&a, &b| {
        let (pa, pb) = (&photos[a], &photos[b]);
        let key = match sort.key {
            SortKey::CaptureTime => match (when[a], when[b]) {
                (Some(ta), Some(tb)) => direction(ta.cmp(&tb)),
                _ => Ordering::Equal,
            },
            SortKey::FileName => direction(library::browser_order(&pa.name, &pb.name)),
            SortKey::Edited => direction(edited(pa).cmp(&edited(pb))),
            SortKey::Rating => direction(rank(pa).cmp(&rank(pb))),
        };
        key.then_with(|| by_time(a, b))
            .then_with(|| library::browser_order(&pa.name, &pb.name))
    });
    order
}

/// The camera's clock as one number that orders as the time does.
fn clock(time: DateTime) -> u128 {
    let parts = [
        u128::from(time.year),
        u128::from(time.month),
        u128::from(time.day),
        u128::from(time.hour),
        u128::from(time.minute),
        u128::from(time.second),
    ];
    let seconds = parts
        .iter()
        .zip([10_000, 100, 100, 100, 100, 100])
        .fold(0, |n, (part, base)| n * base + part);
    seconds * 1000 + u128::from(time.millisecond)
}

/// What a photo's edit says, for its cell's marks.
pub fn status(edit: &library::EditState) -> Option<apertrust_ui::components::CellStatus<'_>> {
    use apertrust_ui::components::CellStatus;
    match edit {
        library::EditState::ViewOnly(_) => Some(CellStatus::ViewOnly),
        library::EditState::Unreadable(reason) => Some(CellStatus::Unreadable(reason)),
        library::EditState::None | library::EditState::Edited => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apertrust_engine::jobs::{Message, Progress, Queue};
    use apertrust_engine::session::library::EditState;

    fn photo(name: &str, edit: EditState) -> Photo {
        Photo {
            name: name.to_owned(),
            size: 1,
            modified: None,
            edit,
            rating: Default::default(),
            fields: Default::default(),
        }
    }

    fn opened(open: u64, folder: &str) -> Event {
        Event::Library(LibraryEvent::Opened {
            open,
            name: folder.rsplit('/').next().unwrap_or(folder).to_owned(),
            folder: PathBuf::from(folder),
        })
    }

    fn listed(open: u64, names: &[&str], done: bool) -> Event {
        let photos = names
            .iter()
            .map(|name| photo(name, EditState::None))
            .collect();
        Event::Library(LibraryEvent::Listed { open, photos, done })
    }

    fn listing(state: &FolderState) -> &Listing {
        match &state.folder {
            Folder::Open(listing) => listing,
            other => panic!("not open: {other:?}"),
        }
    }

    #[test]
    fn an_open_runs_to_done_collecting_the_photos_in_order() {
        let mut state = FolderState::default();
        state.opening(PathBuf::from("/photos/Wedding"), 4);
        assert_eq!(
            state.folder,
            Folder::Opening {
                path: PathBuf::from("/photos/Wedding")
            }
        );
        // The folder is reported once, for the recent folders.
        assert_eq!(
            state.apply(&opened(4, "/photos/Wedding")),
            Some(Outcome::Opened(PathBuf::from("/photos/Wedding")))
        );
        state.apply(&Event::Library(LibraryEvent::Subfolders {
            open: 4,
            names: vec!["Edits".to_owned(), "Selects".to_owned()],
        }));
        state.apply(&listed(4, &["a.ARW", "b.ARW"], false));
        let so_far = listing(&state);
        assert_eq!(so_far.name, "Wedding");
        assert_eq!(so_far.subfolders, ["Edits", "Selects"]);
        assert_eq!(so_far.photos.len(), 2);
        assert!(!so_far.done);
        state.apply(&listed(4, &["c.ARW"], true));
        let all = listing(&state);
        let names: Vec<&str> = all.photos.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["a.ARW", "b.ARW", "c.ARW"]);
        assert!(all.done);
    }

    fn subfolders_of(open: u64, folder: &Path, names: &[&str], unreadable: Option<&str>) -> Event {
        Event::Library(LibraryEvent::SubfoldersOf {
            open,
            folder: folder.to_owned(),
            names: names.iter().map(|name| (*name).to_owned()).collect(),
            unreadable: unreadable.map(str::to_owned),
        })
    }

    #[test]
    fn fresh_subfolder_names_do_not_hide_the_walked_photo_count() {
        let mut state = FolderState::default();
        state.opening(PathBuf::from("/photos"), 1);
        state.apply(&opened(1, "/photos"));
        let folder = PathBuf::from("/photos/Trip");
        state.apply(&subfolders_of(1, &folder, &["New"], None));
        state.apply(&walked(1, &[("/photos/Trip", Some(3), &["Old"])]));
        assert_eq!(state.counts().get(&folder), Some(&3));
        assert_eq!(
            state.inside().get(&folder),
            Some(&Ok(vec!["New".to_owned()]))
        );
    }

    /// A subfolder's subfolders are asked for once an open and kept by its path; the root
    /// opened again keeps them, its walk to come in their place, but drops what comes late
    /// for the open before it; Try Again forgets what came.
    #[test]
    fn a_subfolder_s_subfolders_are_asked_once_an_open() {
        let mut state = FolderState::default();
        state.opening(PathBuf::from("/photos/Wedding"), 4);
        state.apply(&opened(4, "/photos/Wedding"));
        let edits = PathBuf::from("/photos/Wedding/Edits");
        assert!(state.ask_inside(&edits));
        assert!(!state.ask_inside(&edits), "asked once");
        assert!(
            !state.ask_inside(Path::new("/photos/Wedding")),
            "the open folder is listed"
        );
        state.apply(&subfolders_of(4, &edits, &["Picks"], None));
        assert_eq!(
            state.inside().get(&edits),
            Some(&Ok(vec!["Picks".to_owned()]))
        );
        // Opened again: kept, and the late answer of the open before is dropped.
        state.opening(PathBuf::from("/photos/Wedding"), 5);
        state.apply(&opened(5, "/photos/Wedding"));
        state.apply(&subfolders_of(4, &edits, &["Late"], None));
        assert_eq!(
            state.inside().get(&edits),
            Some(&Ok(vec!["Picks".to_owned()]))
        );
        assert!(!state.ask_inside(&edits));
        // A folder that could not be listed says why, and Try Again asks again.
        state.apply(&subfolders_of(5, &edits, &[], Some("the share is down")));
        assert_eq!(
            state.inside().get(&edits),
            Some(&Err("the share is down".to_owned()))
        );
        assert!(!state.ask_inside(&edits));
        state.forget_inside(&edits);
        assert!(state.ask_inside(&edits));
    }

    /// A batch of the tree's walk in the open numbered `open`: each folder with its raws and
    /// subfolders, or unreadable where its raws are `None`.
    fn walked(open: u64, folders: &[(&str, Option<usize>, &[&str])]) -> Event {
        let folders = folders
            .iter()
            .map(|(folder, raws, subfolders)| library::WalkedFolder {
                folder: PathBuf::from(folder),
                raws: raws.unwrap_or(0),
                subfolders: subfolders.iter().map(|name| (*name).to_owned()).collect(),
                unreadable: raws.is_none().then(|| "cannot be read".to_owned()),
            })
            .collect();
        Event::Library(LibraryEvent::Walked {
            open,
            folders,
            done: false,
        })
    }

    /// The tree's walk fills each folder's count and its subfolders, which the tree then
    /// asks for no more, but leaves the root's count its listing gave; it goes on while a
    /// folder chosen in the tree is shown. A folder not walked yet is asked for as before; a
    /// walk of an earlier tree, or after a close, is dropped.
    #[test]
    fn the_tree_s_walk_fills_the_counts_and_an_earlier_tree_s_is_dropped() {
        let mut state = FolderState::default();
        let wedding = PathBuf::from("/photos/Wedding");
        let edits = wedding.join("Edits");
        state.opening(wedding.clone(), 4);
        state.apply(&opened(4, "/photos/Wedding"));
        state.apply(&listed(4, &["a.ARW", "b.ARW", "c.ARW"], true));
        assert_eq!(state.counts().get(&edits), None, "not reached yet");
        state.apply(&walked(
            4,
            &[
                ("/photos/Wedding", Some(99), &["Edits", "Locked"]),
                ("/photos/Wedding/Edits", Some(2), &["Picks"]),
                ("/photos/Wedding/Locked", None, &[]),
            ],
        ));
        assert_eq!(state.root_count(), Some(3), "the listing's");
        assert_eq!(state.counts().get(&edits), Some(&2));
        let locked = wedding.join("Locked");
        assert_eq!(state.counts().get(&locked), None);
        assert_eq!(
            state.inside().get(&locked),
            Some(&Err("cannot be read".to_owned()))
        );
        assert!(!state.ask_inside(&edits), "known from the walk");
        // The fallback: a folder the walk has not reached is asked for.
        let picks = edits.join("Picks");
        assert!(state.ask_inside(&picks));

        // A folder chosen in the tree: the walk goes on in the root's number, and the
        // folder's own listing, fresher, keeps its count.
        state.choosing(edits.clone(), 5);
        state.apply(&opened(5, "/photos/Wedding/Edits"));
        state.apply(&listed(5, &["x.ARW", "y.ARW", "z.ARW", "w.ARW"], true));
        assert_eq!(state.counts().get(&edits), Some(&4));
        state.apply(&walked(
            4,
            &[
                ("/photos/Wedding/Edits", Some(2), &["Picks"]),
                ("/photos/Wedding/Edits/Picks", Some(1), &[]),
            ],
        ));
        assert_eq!(state.counts().get(&picks), Some(&1));
        assert_eq!(state.counts().get(&edits), Some(&4), "its listing's");

        // The root opened again, as a recent opens it: walked again, its counts replacing
        // the old walk's, which is dropped; what is known stays meanwhile.
        state.opening(wedding.clone(), 8);
        assert_eq!(state.counts().get(&picks), Some(&1));
        assert!(
            !state.ask_inside(&edits),
            "kept for a way in inside the tree"
        );
        state.apply(&walked(4, &[("/photos/Wedding/Edits/Picks", Some(9), &[])]));
        assert_eq!(state.counts().get(&picks), Some(&1));
        state.apply(&walked(8, &[("/photos/Wedding/Edits/Picks", Some(2), &[])]));
        assert_eq!(state.counts().get(&picks), Some(&2));

        // A new root: the earlier walk is dropped, its own taken in.
        state.opening(PathBuf::from("/photos/Other"), 6);
        assert!(state.counts().is_empty());
        state.apply(&walked(4, &[("/photos/Wedding/Selects", Some(5), &[])]));
        assert!(state.counts().is_empty());
        state.apply(&walked(6, &[("/photos/Other/Day 1", Some(4), &[])]));
        assert_eq!(state.counts().len(), 1);
        // Closed: nothing more.
        state.closing(7);
        state.apply(&walked(6, &[("/photos/Other/Day 2", Some(4), &[])]));
        assert!(state.counts().is_empty());
    }

    fn failed(open: u64) -> Event {
        Event::Library(LibraryEvent::Failed {
            open,
            reason: "gone".to_owned(),
        })
    }

    /// A folder chosen in the tree is opened, and the Browser shows it, but the root,
    /// its count, the subfolders listed and the recents stay; what was asked is asked again,
    /// since the open cancels its listing. A way in makes a new root, enters the recents and
    /// drops what was listed of the tree before.
    #[test]
    fn a_folder_chosen_in_the_tree_keeps_the_root_and_a_way_in_moves_it() {
        let mut state = FolderState::default();
        let wedding = PathBuf::from("/photos/Wedding");
        let (edits, selects) = (wedding.join("Edits"), wedding.join("Selects"));
        state.opening(wedding.clone(), 1);
        assert_eq!(state.root(), Some(wedding.as_path()));
        let way_in = state.apply(&opened(1, "/photos/Wedding"));
        assert_eq!(way_in, Some(Outcome::Opened(wedding.clone())));
        let names = vec!["Edits".to_owned(), "Selects".to_owned()];
        let subfolders = LibraryEvent::Subfolders {
            open: 1,
            names: names.clone(),
        };
        state.apply(&Event::Library(subfolders));
        state.apply(&listed(1, &["a.ARW", "b.ARW"], true));
        assert_eq!(state.root_count(), Some(2));
        assert!(state.ask_inside(&selects));

        state.choosing(edits.clone(), 2);
        assert_eq!(state.shown(), Some(edits.as_path()));
        assert_eq!(state.apply(&opened(2, "/photos/Wedding/Edits")), None);
        state.apply(&listed(2, &["c.ARW"], true));
        assert_eq!(listing(&state).path, edits);
        assert_eq!(state.root(), Some(wedding.as_path()));
        assert_eq!(state.root_count(), Some(2));
        assert_eq!(state.inside().get(&wedding), Some(&Ok(names)));
        assert!(state.ask_inside(&selects), "asked again");
        // Back to the root, chosen: still not a way in.
        state.choosing(wedding.clone(), 3);
        assert_eq!(state.apply(&opened(3, "/photos/Wedding")), None);
        // A choice that fails leaves the tree as it was, still listed as it opens, and
        // marks no recent missing.
        state.choosing(selects.clone(), 4);
        assert_eq!(state.apply(&failed(4)), None);
        assert_eq!(state.root(), Some(wedding.as_path()));
        assert!(state.ask_inside(&edits), "the tree is still listed");
        assert!(!state.ask_inside(&wedding), "the root was listed");

        let portraits = PathBuf::from("/photos/Portraits");
        state.opening(portraits.clone(), 5);
        assert_eq!(state.root(), Some(portraits.as_path()));
        assert_eq!(state.root_count(), None);
        assert!(state.inside().is_empty());
        let way_in = state.apply(&opened(5, "/photos/Portraits"));
        assert_eq!(way_in, Some(Outcome::Opened(portraits)));
        // A way in that fails says so, and leaves no root and nothing to ask.
        state.opening(PathBuf::from("/photos/Gone"), 6);
        let gone = Some(Outcome::Failed {
            path: PathBuf::from("/photos/Gone"),
            reason: "gone".to_owned(),
        });
        assert_eq!(state.apply(&failed(6)), gone);
        assert_eq!(state.root(), None);
        assert!(!state.ask_inside(Path::new("/photos/Gone/Edits")));
    }

    #[test]
    fn a_second_open_drops_the_first_one_s_late_events() {
        let mut state = FolderState::default();
        state.opening(PathBuf::from("/photos/one"), 2);
        state.apply(&opened(2, "/photos/one"));
        state.opening(PathBuf::from("/photos/two"), 3);
        // What the first reading had on its way.
        assert_eq!(state.apply(&listed(2, &["late.ARW"], true)), None);
        assert_eq!(state.apply(&opened(2, "/photos/one")), None);
        assert!(matches!(state.folder, Folder::Opening { .. }));
        // The second's own come in.
        state.apply(&opened(3, "/photos/two"));
        state.apply(&listed(3, &["x.ARW"], true));
        assert_eq!(listing(&state).path, PathBuf::from("/photos/two"));
        assert_eq!(listing(&state).photos.len(), 1);
        // A late failure of the first changes nothing either.
        let late = Event::Library(LibraryEvent::Failed {
            open: 2,
            reason: "gone".to_owned(),
        });
        assert_eq!(state.apply(&late), None);
        assert!(matches!(state.folder, Folder::Open(_)));
    }

    #[test]
    fn a_failure_ends_the_open_with_its_reason() {
        let mut state = FolderState::default();
        state.opening(PathBuf::from("/photos/gone"), 1);
        let failed = Event::Library(LibraryEvent::Failed {
            open: 1,
            reason: "the folder is missing".to_owned(),
        });
        assert_eq!(
            state.apply(&failed),
            Some(Outcome::Failed {
                path: PathBuf::from("/photos/gone"),
                reason: "the folder is missing".to_owned()
            })
        );
        assert_eq!(
            state.folder,
            Folder::Failed {
                path: PathBuf::from("/photos/gone"),
                reason: Some("the folder is missing".to_owned())
            }
        );
    }

    #[test]
    fn a_close_empties_the_folder_and_drops_what_the_reading_still_sends() {
        let mut state = FolderState::default();
        state.opening(PathBuf::from("/photos/one"), 1);
        state.apply(&opened(1, "/photos/one"));
        state.closing(2);
        assert_eq!(state.folder, Folder::None);
        assert_eq!(state.apply(&listed(1, &["late.ARW"], true)), None);
        assert_eq!(state.folder, Folder::None);
        // The session's answer to the close changes nothing more.
        state.apply(&Event::Library(LibraryEvent::Closed { open: 2 }));
        assert_eq!(state.folder, Folder::None);
    }

    /// The jobs' events for the folder's reading: added for a command, then ended.
    fn job(event: JobEvent, command: Option<u64>) -> Event {
        Event::Job { event, command }
    }

    fn added(job: JobId) -> JobEvent {
        JobEvent::Added {
            job,
            kind: "folder",
            queue: Some(Queue::UserBackground),
            title: Message::new("job-folder"),
            cancellable: true,
            destination: None,
            detail: None,
            progress: Progress::default(),
            retries: None,
            quiet: false,
        }
    }

    fn ended(job: JobId, state: State) -> JobEvent {
        JobEvent::Ended {
            job,
            state,
            outcome: None,
            failure: None,
            at: std::time::SystemTime::UNIX_EPOCH,
            offers: Vec::new(),
        }
    }

    #[test]
    fn a_reading_cancelled_through_its_job_ends_the_open_with_no_reason() {
        let ids = crate::jobs::tests::ids(2);
        let (first, second) = (ids[0], ids[1]);
        let mut state = FolderState::default();
        state.opening(PathBuf::from("/photos/slow"), 5);
        state.apply(&job(added(first), Some(5)));
        state.apply(&job(ended(first, State::Cancelled), None));
        assert_eq!(
            state.folder,
            Folder::Failed {
                path: PathBuf::from("/photos/slow"),
                reason: None
            }
        );

        // The job of an earlier open, cancelled by a newer one, is not this open's.
        let mut state = FolderState::default();
        state.opening(PathBuf::from("/photos/one"), 5);
        state.apply(&job(added(first), Some(5)));
        state.opening(PathBuf::from("/photos/two"), 6);
        state.apply(&job(added(second), Some(6)));
        state.apply(&job(ended(first, State::Cancelled), None));
        assert!(matches!(state.folder, Folder::Opening { .. }));

        // A finished listing stays: a late cancel changes nothing.
        state.apply(&opened(6, "/photos/two"));
        state.apply(&listed(6, &["a.ARW"], true));
        state.apply(&job(ended(second, State::Cancelled), None));
        assert!(matches!(state.folder, Folder::Open(_)));
    }

    /// The key of the folder's photo of this name.
    fn name(photo: &str) -> CellKey {
        CellKey::Name(photo.to_owned())
    }

    fn thumbnail(open: u64, photo: &str, size: u32, image: Option<Thumbnail>) -> Event {
        Event::Library(LibraryEvent::Thumbnail {
            open,
            photo: photo.to_owned(),
            size,
            image,
        })
    }

    fn picture(width: u32, height: u32) -> Thumbnail {
        Thumbnail {
            width,
            height,
            pixels: vec![0; (width * height * 4) as usize].into(),
        }
    }

    #[test]
    fn thumbnails_are_kept_by_photo_and_size_each_raising_the_generation() {
        let mut state = FolderState::default();
        state.opening(PathBuf::from("/photos/a"), 1);
        state.apply(&opened(1, "/photos/a"));
        state.apply(&listed(1, &["x.ARW", "y.ARW"], true));
        let before = state.generation();

        state.apply(&thumbnail(1, "x.ARW", 256, Some(picture(256, 171))));
        state.apply(&thumbnail(1, "x.ARW", 128, Some(picture(128, 85))));
        state.apply(&thumbnail(1, "y.ARW", 256, None));

        let x = state.thumbnail(&name("x.ARW"), 256).expect("x at 256");
        assert_eq!(x.image, Some(picture(256, 171)));
        assert!(x.generation > before);
        let small = state.thumbnail(&name("x.ARW"), 128).expect("x at 128");
        assert_eq!(small.image.as_ref().map(|t| t.width), Some(128));
        // A photo with none is kept as having none, so it is not asked for again.
        assert_eq!(
            state.thumbnail(&name("y.ARW"), 256).map(|k| &k.image),
            Some(&None)
        );
        assert!(state.thumbnail(&name("y.ARW"), 128).is_none());
        assert!(state.generation() > x.generation);
    }

    #[test]
    fn thumbnails_go_with_their_folder_and_an_earlier_open_s_are_dropped() {
        let mut state = FolderState::default();
        state.opening(PathBuf::from("/photos/a"), 1);
        state.apply(&opened(1, "/photos/a"));
        state.apply(&listed(1, &["x.ARW"], true));
        state.apply(&thumbnail(1, "x.ARW", 256, Some(picture(4, 3))));
        let generation = state.generation();

        // Another folder: the first's thumbnails are gone, and one on its way is dropped.
        state.opening(PathBuf::from("/photos/b"), 2);
        assert!(state.generation() > generation);
        state.apply(&opened(2, "/photos/b"));
        state.apply(&listed(2, &["x.ARW"], true));
        state.apply(&thumbnail(1, "x.ARW", 256, Some(picture(4, 3))));
        assert!(state.thumbnail(&name("x.ARW"), 256).is_none());
        state.apply(&thumbnail(2, "x.ARW", 256, None));
        assert!(state.thumbnail(&name("x.ARW"), 256).is_some());

        // A close takes them too.
        state.closing(3);
        assert!(state.thumbnail(&name("x.ARW"), 256).is_none());
    }

    #[test]
    fn a_photo_is_found_in_the_folder_by_its_path() {
        let mut state = FolderState::default();
        state.opening(PathBuf::from("/photos/a"), 1);
        state.apply(&opened(1, "/photos/a"));
        state.apply(&listed(1, &["x.ARW", "y.ARW"], true));
        let listing = listing(&state);
        assert_eq!(listing.position(Path::new("/photos/a/y.ARW")), Some(1));
        assert_eq!(listing.position(Path::new("/photos/b/y.ARW")), None);
        assert_eq!(listing.position(Path::new("/photos/a/z.ARW")), None);
    }

    #[test]
    fn the_active_cell_is_found_when_the_photo_or_the_listing_changes() {
        let mut state = FolderState::default();
        state.opening(PathBuf::from("/photos/a"), 1);
        state.apply(&opened(1, "/photos/a"));
        state.apply(&listed(1, &["x.ARW", "y.ARW"], false));
        let photo = Path::new("/photos/a/y.ARW");
        state.locate(Some(photo));
        assert_eq!(state.active(), Some(1));
        // The photo is not in the folder, or none is open.
        state.locate(Some(Path::new("/photos/b/y.ARW")));
        assert_eq!(state.active(), None);
        state.locate(None);
        assert_eq!(state.active(), None);
        // Its cell may arrive in a later chunk.
        state.locate(Some(Path::new("/photos/a/z.ARW")));
        assert_eq!(state.active(), None);
        state.apply(&listed(1, &["z.ARW"], true));
        state.locate(Some(Path::new("/photos/a/z.ARW")));
        assert_eq!(state.active(), Some(2));
        // A closed folder has none.
        state.closing(2);
        state.locate(Some(photo));
        assert_eq!(state.active(), None);
    }

    /// Photos opened together alone: each once, in the order opened, the Browser holding
    /// just them, the first active, then whichever of them opens; their edit marks follow
    /// the session's values, and the close of the folder left, sent before them, leaves
    /// them.
    #[test]
    fn photos_opened_alone_are_held_together_the_first_active() {
        // Compared as absolute, so on Windows they take the current drive.
        let absolute = |path: &str| std::path::absolute(path).unwrap();
        let (x, y, z) = (
            absolute("/photos/a/x.ARW"),
            absolute("/photos/b/y.ARW"),
            absolute("/photos/a/z.ARW"),
        );
        let mut state = FolderState::default();
        state.opening(absolute("/photos/a"), 1);
        state.apply(&opened(1, "/photos/a"));
        state.alone(2, vec![x.clone(), y.clone(), x.clone(), z.clone()]);
        assert_eq!(state.alone_paths(), [x.clone(), y.clone(), z.clone()]);
        let names: Vec<&str> = state.photos().iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["x.ARW", "y.ARW", "z.ARW"]);
        assert_eq!(state.order(), [0, 1, 2]);
        assert_eq!((state.root(), state.shown()), (None, None));
        let folder = |path: &Path| path.parent().unwrap().to_owned();
        let active = |state: &FolderState| {
            let (photo, folder) = state.alone_active()?;
            Some((photo.to_owned(), folder.to_owned()))
        };
        assert_eq!(active(&state), Some((x.clone(), folder(&x))));
        state.apply(&Event::Library(LibraryEvent::Closed { open: 2 }));
        assert!(matches!(state.folder, Folder::Alone { .. }));
        // The first opens, then another of them, which Open This Folder then follows.
        state.locate(Some(&x));
        assert_eq!(state.active_shown(), Some(0));
        state.locate(Some(&y));
        assert_eq!(state.active_shown(), Some(1));
        assert_eq!(active(&state), Some((y.clone(), folder(&y))));
        // None of them open: no active cell, Open This Folder still the last one's.
        state.locate(None);
        assert_eq!(state.active_shown(), None);
        assert_eq!(active(&state), Some((y.clone(), folder(&y))));
        state.edited(&z, true, true);
        assert_eq!(state.photos()[2].edit, EditState::Edited);
        assert_eq!(state.photos()[0].edit, EditState::None);
    }

    /// The thumbnail of the photo alone at `path` at 256, of 4 by 3, in the number `open`.
    fn alone_thumbnail(open: u64, path: &Path) -> Event {
        Event::Library(LibraryEvent::PhotoThumbnail {
            open,
            path: path.to_owned(),
            size: 256,
            image: Some(Thumbnail {
                width: 4,
                height: 3,
                pixels: vec![128; 48].into(),
            }),
        })
    }

    /// The photos alone's thumbnails are kept by path, two of the same name from two
    /// folders apart, and are what one shows as it opens; those of a path no longer among
    /// them, of an earlier open or close, or naming a folder's photo, are dropped. Others
    /// opened alone keep those of the paths among them still, and a folder's opening drops
    /// them all.
    #[test]
    fn photos_alone_keep_their_own_thumbnails_until_they_leave_or_a_folder_opens() {
        // Absolute, as the app opens them, so on Windows they take the current drive.
        let absolute = |path: &str| std::path::absolute(path).unwrap();
        let (x, y, z) = (
            absolute("/photos/a/x.ARW"),
            absolute("/photos/b/x.ARW"),
            absolute("/photos/a/z.ARW"),
        );
        let at = |path: &PathBuf| CellKey::Path(path.clone());
        let mut state = FolderState::default();
        state.alone(2, vec![x.clone(), y.clone()]);
        let generation = state.generation();
        state.apply(&alone_thumbnail(2, &x));
        assert!(state.generation() > generation);
        assert_eq!([state.key(0), state.key(1)], [at(&x), at(&y)]);
        assert!(state.thumbnail(&at(&x), 256).is_some());
        assert_eq!(state.thumbnail(&at(&y), 256), None);
        assert_eq!(state.thumbnail(&name("x.ARW"), 256), None);
        // An earlier command's, one not among them, and a folder's photo's: none kept.
        state.apply(&alone_thumbnail(1, &y));
        state.apply(&alone_thumbnail(2, &z));
        state.apply(&Event::Library(LibraryEvent::Thumbnail {
            open: 2,
            photo: "x.ARW".to_owned(),
            size: 256,
            image: None,
        }));
        assert_eq!(state.thumbnails().count(), 1);
        let largest = state.largest_thumbnail(&x).expect("x's");
        assert_eq!((largest.width, largest.height), (4, 3));
        assert!(state.largest_thumbnail(&y).is_none());
        // Others opened alone, by command 3: x's stays, among them still, and shows as x
        // opens again; one of y, no longer among them, is dropped, and one of command 2.
        state.alone(3, vec![z.clone(), x.clone()]);
        assert!(state.thumbnail(&at(&x), 256).is_some());
        assert!(state.largest_thumbnail(&x).is_some());
        state.apply(&alone_thumbnail(3, &y));
        state.apply(&alone_thumbnail(2, &z));
        assert_eq!(state.thumbnails().count(), 1);
        state.apply(&alone_thumbnail(3, &z));
        assert_eq!(state.thumbnails().count(), 2);
        assert!(state.thumbnail(&at(&z), 256).is_some());
        // Without x, its thumbnail goes.
        state.alone(4, vec![z.clone()]);
        assert_eq!(state.thumbnail(&at(&x), 256), None);
        assert_eq!(state.thumbnails().count(), 1);
        // A folder opens: they go, and a photo alone's that comes after is dropped.
        state.opening(PathBuf::from("/photos/a"), 5);
        state.apply(&opened(5, "/photos/a"));
        state.apply(&alone_thumbnail(5, &z));
        assert_eq!(state.thumbnails().count(), 0);
    }

    #[test]
    fn only_a_view_only_or_unreadable_edit_has_a_status_mark() {
        assert_eq!(status(&EditState::None), None);
        assert_eq!(status(&EditState::Edited), None);
        assert!(status(&EditState::Unreadable("bad".to_owned())).is_some());
    }

    /// A photo modified `seconds` after the epoch, with `edit`.
    fn dated(name: &str, seconds: Option<u64>, edit: EditState) -> Photo {
        Photo {
            modified: seconds.map(|s| UNIX_EPOCH + std::time::Duration::from_secs(s)),
            ..photo(name, edit)
        }
    }

    /// 2026-10-`day` at noon and `second` seconds.
    fn shot(day: u8, second: u8) -> Option<DateTime> {
        Some(DateTime {
            year: 2026,
            month: 10,
            day,
            hour: 12,
            minute: 0,
            second,
            millisecond: 0,
            offset_minutes: None,
        })
    }

    fn names(photos: &[Photo], order: &[usize]) -> Vec<String> {
        order.iter().map(|&i| photos[i].name.clone()).collect()
    }

    fn by(key: SortKey, descending: bool) -> Sort {
        Sort { key, descending }
    }

    /// An unreadable sidecar stays so while its photo is open for viewing only; once it
    /// opens editable, repaired, its values say whether it has an edit. A newer
    /// Apertrust's edit stays as the reading found it.
    #[test]
    fn a_repaired_sidecar_s_photo_follows_its_values() {
        let mut state = FolderState::default();
        state.opening(PathBuf::from("/photos"), 1);
        state.apply(&opened(1, "/photos"));
        let unreadable = EditState::Unreadable("bad".to_owned());
        let photos = vec![photo("a.ARW", unreadable.clone())];
        state.apply(&Event::Library(LibraryEvent::Listed {
            open: 1,
            photos,
            done: true,
        }));
        let a = PathBuf::from("/photos/a.ARW");
        state.edited(&a, false, false);
        assert_eq!(state.photos()[0].edit, unreadable);
        state.edited(&a, false, true);
        assert_eq!(state.photos()[0].edit, EditState::None);
    }

    #[test]
    fn photos_sort_by_each_key_with_ties_by_time_then_file_name() {
        let photos = [
            dated("IMG_10.ARW", Some(30), EditState::Edited),
            dated("IMG_2.ARW", Some(10), EditState::None),
            dated("IMG_3.ARW", Some(10), EditState::Edited),
            dated("img_1.ARW", None, EditState::None),
            dated("IMG_4.ARW", Some(20), EditState::Unreadable("bad".into())),
        ];
        let order = |sort| names(&photos, &sorted(&photos, None, sort));
        // Until the capture times come, the files' modified times: equal ones by name,
        // digits as numbers, and one with none last, either way.
        assert_eq!(
            order(by(SortKey::CaptureTime, false)),
            [
                "IMG_2.ARW",
                "IMG_3.ARW",
                "IMG_4.ARW",
                "IMG_10.ARW",
                "img_1.ARW"
            ]
        );
        assert_eq!(
            order(by(SortKey::CaptureTime, true)),
            [
                "IMG_10.ARW",
                "IMG_4.ARW",
                "IMG_2.ARW",
                "IMG_3.ARW",
                "img_1.ARW"
            ]
        );
        // The file browser's order, case aside.
        assert_eq!(
            order(by(SortKey::FileName, false)),
            [
                "img_1.ARW",
                "IMG_2.ARW",
                "IMG_3.ARW",
                "IMG_4.ARW",
                "IMG_10.ARW"
            ]
        );
        assert_eq!(
            order(by(SortKey::FileName, true)),
            [
                "IMG_10.ARW",
                "IMG_4.ARW",
                "IMG_3.ARW",
                "IMG_2.ARW",
                "img_1.ARW"
            ]
        );
        // Those without an edit first; each side in shooting order, then by name, which
        // descending keeps.
        assert_eq!(
            order(by(SortKey::Edited, false)),
            [
                "IMG_2.ARW",
                "IMG_4.ARW",
                "img_1.ARW",
                "IMG_3.ARW",
                "IMG_10.ARW"
            ]
        );
        assert_eq!(
            order(by(SortKey::Edited, true)),
            [
                "IMG_3.ARW",
                "IMG_10.ARW",
                "IMG_2.ARW",
                "IMG_4.ARW",
                "img_1.ARW"
            ]
        );
        // Unrated alike: the ties' order alone, whichever way.
        let shooting = [
            "IMG_2.ARW",
            "IMG_3.ARW",
            "IMG_4.ARW",
            "IMG_10.ARW",
            "img_1.ARW",
        ];
        assert_eq!(order(by(SortKey::Rating, false)), shooting);
        assert_eq!(order(by(SortKey::Rating, true)), shooting);
    }

    /// Sort › Rating: rejected first, its stars beneath it counting for nothing, then no
    /// stars, then one to five; ties in shooting order, descending the reverse of the key.
    #[test]
    fn the_rating_sort_puts_rejected_first_then_no_stars_then_one_to_five() {
        let rated = |name: &str, stars: u8, rejected: bool| Photo {
            rating: Rating {
                stars,
                rejected,
                labels: 0,
            },
            ..photo(name, EditState::None)
        };
        let photos = [
            rated("A.ARW", 5, false),
            rated("B.ARW", 0, false),
            rated("C.ARW", 4, true),
            rated("D.ARW", 2, false),
            rated("E.ARW", 0, false),
        ];
        let order = |sort| names(&photos, &sorted(&photos, None, sort));
        assert_eq!(
            order(by(SortKey::Rating, false)),
            ["C.ARW", "B.ARW", "E.ARW", "D.ARW", "A.ARW"]
        );
        assert_eq!(
            order(by(SortKey::Rating, true)),
            ["A.ARW", "D.ARW", "B.ARW", "E.ARW", "C.ARW"]
        );
    }

    #[test]
    fn capture_times_take_the_modified_times_place_and_a_photo_open_for_viewing_is_edited() {
        let newer = apertrust_engine::sidecar::Newer::Version(3);
        let photos = [
            dated("A.ARW", Some(1), EditState::None),
            dated("B.ARW", Some(2), EditState::ViewOnly(newer)),
            dated("C.ARW", Some(3), EditState::None),
        ];
        // Taken in the other order; C's camera recorded none.
        let taken = [shot(2, 0), shot(1, 0), None];
        let order = |sort| names(&photos, &sorted(&photos, Some(&taken), sort));
        assert_eq!(
            order(by(SortKey::CaptureTime, false)),
            ["B.ARW", "A.ARW", "C.ARW"]
        );
        assert_eq!(
            order(by(SortKey::CaptureTime, true)),
            ["A.ARW", "B.ARW", "C.ARW"]
        );
        assert_eq!(
            order(by(SortKey::Edited, false)),
            ["A.ARW", "C.ARW", "B.ARW"]
        );
    }

    #[test]
    fn the_order_follows_the_sort_the_chunks_and_the_capture_times_once() {
        let mut state = FolderState::default();
        state.sort(by(SortKey::CaptureTime, false));
        state.opening(PathBuf::from("/photos"), 2);
        state.apply(&opened(2, "/photos"));
        let event = |photos| {
            Event::Library(LibraryEvent::Listed {
                open: 2,
                photos,
                done: false,
            })
        };
        state.apply(&event(vec![
            dated("A.ARW", Some(3), EditState::None),
            dated("B.ARW", Some(1), EditState::None),
        ]));
        assert_eq!(state.order(), [1, 0]);
        state.apply(&event(vec![dated("C.ARW", Some(2), EditState::None)]));
        assert_eq!(state.order(), [1, 2, 0]);
        // The camera's times replace the files', all at once.
        state.apply(&Event::Library(LibraryEvent::Taken {
            open: 2,
            times: vec![shot(1, 0), shot(1, 2), shot(1, 1)],
        }));
        assert_eq!(state.order(), [0, 2, 1]);
        // The active cell is the photo open, where the order puts it.
        state.locate(Some(Path::new("/photos/B.ARW")));
        assert_eq!(state.active(), Some(1));
        assert_eq!(state.active_shown(), Some(2));
        state.sort(by(SortKey::FileName, true));
        assert_eq!(state.order(), [2, 1, 0]);
        assert_eq!(state.active_shown(), Some(1));
        // Times that do not match the listing are not taken; a close empties the order.
        state.apply(&Event::Library(LibraryEvent::Taken {
            open: 2,
            times: vec![None],
        }));
        assert!(listing(&state).taken.as_ref().is_some_and(|t| t.len() == 3));
        state.closing(3);
        assert!(state.order().is_empty());
        assert_eq!(state.active_shown(), None);
    }
}
