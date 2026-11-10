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
//!   [`FolderState::closing`]); an event naming another is an earlier command's and dropped.
//! - **The photos** accumulate from the `Listed` chunks, in the listing's order, until one says
//!   `done`.
//! - **A failure** is `Failed`, or the reading's job ending cancelled (the jobs window's
//!   Cancel): the open is then over, with its reason, or none for a cancel.
//! - **The thumbnails** come one `Thumbnail` at a time, kept by photo and size with the
//!   listing, so they go with the folder; each change raises [`FolderState::generation`],
//!   which the textures made of them compare (`thumbnails.rs`, `docs/gui.md`, Derived
//!   caches).
//! - **The order** the grid panel shows the photos in is the grid bar's sort
//!   ([`FolderState::sort`], [`sorted`]), worked out again when the sort changes, a chunk
//!   comes or the capture times do (`Taken`, once the listing is done); until they come,
//!   a photo sorts by its file's modified time (`docs/gui.md`, The grid bar). The listing
//!   keeps the folder's own order; the panel draws [`FolderState::order`].

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use apertrust_engine::jobs::{JobEvent, JobId, State};
use apertrust_engine::session::library::{self, DateTime, Photo, Thumbnail};
use apertrust_engine::session::{Event, LibraryEvent};
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
    /// The grid bar's sort, which [`FolderState::order`] follows.
    sort: Option<Sort>,
    /// The listing's photos in the sort's order, by their place in the listing.
    order: Vec<usize>,
    /// Each photo's place in [`FolderState::order`], by its place in the listing.
    place: Vec<usize>,
    /// The subfolders of folders inside the open one that Browse's tree opened, by path, as
    /// the library listed them (`LibraryEvent::SubfoldersOf`).
    inside: HashMap<PathBuf, Result<Vec<String>, String>>,
    /// The folders inside it whose subfolders were asked for, so each is asked once.
    asked: HashSet<PathBuf>,
}

impl FolderState {
    /// The folder at `path` is asked for by the command numbered `number`.
    pub fn opening(&mut self, path: PathBuf, number: u64) {
        self.latest = number;
        self.job = None;
        self.folder = Folder::Opening { path };
        self.inside.clear();
        self.asked.clear();
        self.generation += 1;
        self.reorder();
    }

    /// The folder is closed by the command numbered `number`: nothing is open at once, and
    /// the events of the reading it stops are dropped.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "no action closes a folder yet; the mirror is ready"
        )
    )]
    pub fn closing(&mut self, number: u64) {
        self.latest = number;
        self.job = None;
        self.folder = Folder::None;
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

    /// The photo's thumbnail at `size`, if it has come: `Some(None)` where it has none.
    pub fn thumbnail(&self, photo: &str, size: u32) -> Option<&Kept> {
        match &self.folder {
            Folder::Open(listing) => listing.thumbnails.get(&(photo.to_owned(), size)),
            _ => None,
        }
    }

    /// The largest thumbnail that has come of the raw at `path`, if it is one of the open
    /// folder's photos: what the central panel shows at once as it opens (decision 77).
    pub fn largest_thumbnail(&self, path: &Path) -> Option<&Thumbnail> {
        let Folder::Open(listing) = &self.folder else {
            return None;
        };
        listing.position(path)?;
        let name = path.file_name()?.to_str()?;
        (listing.thumbnails.iter())
            .filter(|((photo, _), _)| photo == name)
            .filter_map(|((_, size), kept)| Some((*size, kept.image.as_ref()?)))
            .max_by_key(|(size, _)| *size)
            .map(|(_, image)| image)
    }

    /// The photo open, where it is in the listing.
    pub fn active(&self) -> Option<usize> {
        // Only a folder that is open has cells.
        self.active
            .filter(|_| matches!(self.folder, Folder::Open(_)))
    }

    /// The grid panel's active cell: the photo open, where it is in the order.
    pub fn active_shown(&self) -> Option<usize> {
        self.active().and_then(|i| self.place.get(i).copied())
    }

    /// The listing's photos as the grid panel shows them: each one's place in the listing,
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

    /// The photo at `path`, open, has an edit or none now, as the session reports its
    /// values: its place in the listing says so, and the Edited sort moves it, without a new
    /// reading of the folder. An edit a newer Apertrust wrote, or a sidecar that could not
    /// be read, stays as the reading found it.
    pub fn edited(&mut self, path: &Path, edited: bool) {
        let Folder::Open(listing) = &mut self.folder else {
            return;
        };
        let Some(photo) = listing.position(path).map(|i| &mut listing.photos[i]) else {
            return;
        };
        let now = if edited {
            library::EditState::Edited
        } else {
            library::EditState::None
        };
        if matches!(
            photo.edit,
            library::EditState::None | library::EditState::Edited
        ) && photo.edit != now
        {
            photo.edit = now;
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
            _ => Vec::new(),
        };
        self.place = vec![0; self.order.len()];
        for (at, &photo) in self.order.iter().enumerate() {
            self.place[photo] = at;
        }
    }

    /// Finds `photo`, the photo open, in the listing again, for [`FolderState::active`]: to
    /// be called when the photo or the listing changes, which is when its place can.
    pub fn locate(&mut self, photo: Option<&Path>) {
        self.active = match (&self.folder, photo) {
            (Folder::Open(listing), Some(photo)) => listing.position(photo),
            _ => None,
        };
    }

    /// Whether the subfolders of `folder`, inside the open folder's tree, are to be asked
    /// for: neither listed nor asked yet. Marks them asked.
    pub fn ask_inside(&mut self, folder: &Path) -> bool {
        let Folder::Open(listing) = &self.folder else {
            return false;
        };
        let listed = listing.path == folder || self.inside.contains_key(folder);
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

    /// The folder shown: the open one, or the one being opened.
    pub fn shown(&self) -> Option<&Path> {
        match &self.folder {
            Folder::Open(listing) => Some(&listing.path),
            Folder::Opening { path } => Some(path),
            Folder::None | Folder::Failed { .. } => None,
        }
    }

    /// The folder to open for the photo at `photo`, when the folder shown is not its own:
    /// File › Open… shows the folder of what it opens (`docs/gui.md`, File). Paths compare
    /// as absolute, since a raw named on the command line may be relative.
    pub fn to_show(&self, photo: &Path) -> Option<PathBuf> {
        let folder = std::path::absolute(photo.parent()?).ok()?;
        let shown = self
            .shown()
            .and_then(|shown| std::path::absolute(shown).ok());
        (shown.as_ref() != Some(&folder)).then_some(folder)
    }

    /// Takes in `event`, if it is the folder's: a `Library` event of the latest command, or
    /// the job that command started. Returns what the app is to do about it, if anything.
    pub fn apply(&mut self, event: &Event) -> Option<Outcome> {
        match event {
            Event::Library(event) if event.open() == self.latest => self.library(event),
            Event::Job { event, command } => {
                self.job_event(event, *command);
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
                Some(Outcome::Opened(folder.clone()))
            }
            LibraryEvent::Subfolders { names, .. } => {
                if let Folder::Open(listing) = &mut self.folder {
                    listing.subfolders = names.clone();
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
                None
            }
            LibraryEvent::Listed { photos, done, .. } => {
                if let Folder::Open(listing) = &mut self.folder {
                    listing.photos.extend(photos.iter().cloned());
                    listing.done |= *done;
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
                self.reorder();
                Some(Outcome::Failed {
                    path,
                    reason: reason.clone(),
                })
            }
            LibraryEvent::Closed { .. } => {
                self.folder = Folder::None;
                self.reorder();
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
                self.generation += 1;
                self.reorder();
            }
            _ => {}
        }
    }
}

/// The photos' order by `sort`, each by its place in `photos`; `taken`, the photos' capture
/// times once they are known, else their files' modified times stand in (`docs/gui.md`, The
/// grid bar). Descending reverses the key, never the ties: photos equal by it stay in
/// shooting order, then the file browser's order of their names, and photos with no time
/// come last whichever way. Rating has no ratings to sort by yet, so only its ties order.
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
            SortKey::Rating => Ordering::Equal,
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

    /// A subfolder's subfolders are asked for once an open and kept by its path; a new
    /// open drops them, and what comes late for the open before it, so the same folder
    /// opened again asks again; Try Again forgets what came.
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
        // Opened again: asked again, and the late answer of the open before is dropped.
        state.opening(PathBuf::from("/photos/Wedding"), 5);
        state.apply(&opened(5, "/photos/Wedding"));
        state.apply(&subfolders_of(4, &edits, &["Picks"], None));
        assert!(state.inside().is_empty());
        assert!(state.ask_inside(&edits));
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

        let x = state.thumbnail("x.ARW", 256).expect("x at 256");
        assert_eq!(x.image, Some(picture(256, 171)));
        assert!(x.generation > before);
        let small = state.thumbnail("x.ARW", 128).expect("x at 128");
        assert_eq!(small.image.as_ref().map(|t| t.width), Some(128));
        // A photo with none is kept as having none, so it is not asked for again.
        assert_eq!(state.thumbnail("y.ARW", 256).map(|k| &k.image), Some(&None));
        assert!(state.thumbnail("y.ARW", 128).is_none());
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
        assert!(state.thumbnail("x.ARW", 256).is_none());
        state.apply(&thumbnail(2, "x.ARW", 256, None));
        assert!(state.thumbnail("x.ARW", 256).is_some());

        // A close takes them too.
        state.closing(3);
        assert!(state.thumbnail("x.ARW", 256).is_none());
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

    #[test]
    fn a_photo_opened_shows_its_folder_unless_that_is_the_one_shown() {
        // The folder shown is absolute, so on Windows it takes the current drive.
        let absolute = |path: &str| std::path::absolute(path).unwrap();
        let mut state = FolderState::default();
        let photo = Path::new("/photos/a/x.ARW");
        assert_eq!(state.to_show(photo), Some(absolute("/photos/a")));
        // Being opened counts as shown.
        state.opening(PathBuf::from("/photos/a"), 1);
        assert_eq!(state.to_show(photo), None);
        state.apply(&opened(1, "/photos/a"));
        assert_eq!(state.to_show(photo), None);
        assert_eq!(
            state.to_show(Path::new("/photos/b/y.ARW")),
            Some(absolute("/photos/b"))
        );
        // A failed folder is not shown: the photo's own is tried.
        state.opening(PathBuf::from("/photos/gone"), 2);
        state.apply(&Event::Library(LibraryEvent::Failed {
            open: 2,
            reason: "gone".to_owned(),
        }));
        assert_eq!(state.to_show(photo), Some(absolute("/photos/a")));
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
        })
    }

    fn names(photos: &[Photo], order: &[usize]) -> Vec<String> {
        order.iter().map(|&i| photos[i].name.clone()).collect()
    }

    fn by(key: SortKey, descending: bool) -> Sort {
        Sort { key, descending }
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
        // No ratings yet: the ties' order alone, whichever way.
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

    #[test]
    fn capture_times_take_the_modified_times_place_and_a_photo_open_for_viewing_is_edited() {
        let newer = apertrust_engine::sidecar::Newer::Version(2);
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
