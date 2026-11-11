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

//! Lists one folder: its raws, by the decoder's extensions, and their `.apxmp` sidecars
//! (decisions 46 and 53).
//!
//! The listing runs on a worker thread of its own with `std::fs` alone and sends what it
//! finds in chunks over a channel, so the caller never waits on the folder: a slow share
//! costs the worker's time, not the interface's. The channel is bounded, so a caller that
//! reads slowly holds the worker back instead of letting results pile up. The caller can
//! cancel, and a deadline passed is reported as an event while the listing goes on. Hidden
//! entries (names starting with a dot, and folders the system's own flag hides) are not
//! listed, nor a Mac package as a folder (`kind`, the rule the tree's walk shares), and
//! subfolders only by their names,
//! sorted as the system's file browser sorts them ([`browser_order`]), without descending
//! into them; bulk listing is planned. The one hidden file it touches is a leftover
//! temporary of Apertrust's own safe write (`.~NAME.XXXXXX.tmp`, decision 53) more than a
//! day old, which it removes and counts in [`Summary::stale_removed`]. A name that is not
//! text (not valid UTF-8) is not listed, whatever it is: it is counted apart in
//! [`Summary::not_text`] and warned of once per listing.
//!
//! A sidecar belongs to the raw whose name it carries plus [`SUFFIX`]
//! (`DSCF0001.RAF.apxmp`), matched byte for byte for now; one with no raw is an orphan.
//! A sidecar's contents are not read here.

use std::cmp;
use std::collections::HashSet;
use std::fs;
use std::iter::Peekable;
use std::path::{Path, PathBuf};
use std::str::Chars;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use crate::sidecar::SUFFIX;

/// How many events the channel holds before the worker waits for the caller.
const CHANNEL_CAPACITY: usize = 16;

/// A file as the folder lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileStat {
    pub size: u64,
    /// `None` where the system gives no modified time.
    pub modified: Option<SystemTime>,
}

/// A raw with its sidecar, if it has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Photo {
    /// The raw's file name, as the system lists it.
    pub name: String,
    pub raw: FileStat,
    /// The sidecar, named `name` plus [`SUFFIX`].
    pub sidecar: Option<FileStat>,
}

/// A sidecar whose raw is not in the folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Orphan {
    /// The sidecar's file name.
    pub name: String,
    pub file: FileStat,
}

/// What a listing counted, sent when it ends.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    /// Raws listed.
    pub raws: usize,
    /// Of them, those with a sidecar.
    pub with_sidecar: usize,
    pub orphans: usize,
    /// Files that are neither raws nor sidecars.
    pub other: usize,
    /// Names that are not text (not valid UTF-8, possible on Linux), which are not
    /// listed, whatever they are.
    pub not_text: usize,
    /// Names starting with a dot, folders among them, but for the temporaries removed,
    /// and folders the system's own flag hides.
    pub hidden: usize,
    /// Temporaries of writes that never ended, found by their name alone
    /// (`apertrust_base::fs::is_temporary_name`), older than a day, and removed.
    pub stale_removed: usize,
    /// Subfolders, whose names alone are listed ([`Event::Subfolders`]), hidden ones
    /// counted in `hidden`.
    pub folders: usize,
    /// Files that vanished or could not be read between the folder's listing and their
    /// size.
    pub unreadable: usize,
    /// Whether the listing was cancelled: the counts are then of what it had reached.
    pub cancelled: bool,
    pub elapsed: Duration,
}

/// What a listing sends, in order: [`Chunk`](Event::Chunk)s, then
/// [`Orphans`](Event::Orphans), then [`Subfolders`](Event::Subfolders), then
/// [`Done`](Event::Done), or [`Failed`](Event::Failed) instead;
/// [`DeadlinePassed`](Event::DeadlinePassed) may come anywhere before the end.
#[derive(Debug)]
pub enum Event {
    /// More photos, sorted by name across chunks.
    Chunk(Vec<Photo>),
    /// Every orphan sidecar, sorted by name; not sent when there is none.
    Orphans(Vec<Orphan>),
    /// The names of every subfolder but the hidden ones, in [`browser_order`]; not sent
    /// when there is none. A link to a folder counts as one.
    Subfolders(Vec<String>),
    /// The listing has taken longer than [`Options::deadline`], and goes on. Sent once,
    /// even while a call into the folder has not returned.
    DeadlinePassed { limit: Duration },
    /// The end.
    Done(Summary),
    /// The folder could not be listed; the end.
    Failed(Error),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot list {}: {source}", path.display())]
    List {
        path: PathBuf,
        source: std::io::Error,
    },
    /// The worker ended without an answer.
    #[error("the listing stopped without an answer")]
    Stopped,
}

/// How to list.
#[derive(Debug, Clone)]
pub struct Options {
    /// Photos a chunk holds at most; at least one.
    pub chunk: usize,
    /// When to report a listing that is still running; `None` never reports.
    pub deadline: Option<Duration>,
    /// Lists the subfolders alone: no raw or sidecar is listed or measured, and nothing is
    /// removed, as Browse's tree looks inside a folder ([`subfolders`]).
    pub folders_only: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            chunk: 256,
            deadline: None,
            folders_only: false,
        }
    }
}

/// How often a caller waiting on a listing asks its cancel.
const CANCEL_POLL: Duration = Duration::from_millis(20);

/// The subfolders of `folder`, by the listing's own rules (hidden names and names that are
/// not text left out, a link counting as what it points to), in [`browser_order`]: a
/// folders-only listing on its worker thread, waited for while `cancel` is asked every
/// 20 ms, so a folder on a share that hangs holds only the listing's own thread, and the
/// caller returns once cancelled. `None` when cancelled.
pub fn subfolders(
    folder: &Path,
    cancel: &(dyn Fn() -> bool + Sync),
) -> Result<Option<Vec<String>>, Error> {
    let options = Options {
        folders_only: true,
        ..Options::default()
    };
    let listing = Listing::start(folder, options);
    let mut names = Vec::new();
    loop {
        if cancel() {
            return Ok(None);
        }
        match listing.events().recv_timeout(CANCEL_POLL) {
            Ok(Event::Subfolders(mut more)) => names.append(&mut more),
            Ok(Event::Done(summary)) if summary.cancelled => return Ok(None),
            Ok(Event::Done(_)) => return Ok(Some(names)),
            Ok(Event::Failed(error)) => return Err(error),
            Ok(_) | Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Err(Error::Stopped),
        }
    }
}

/// A folder listed whole by [`Listing::collect`].
#[derive(Debug)]
pub struct Folder {
    pub photos: Vec<Photo>,
    pub orphans: Vec<Orphan>,
    /// The subfolders' names, in [`browser_order`].
    pub subfolders: Vec<String>,
    pub summary: Summary,
    /// Whether the deadline passed before the end.
    pub deadline_passed: bool,
}

/// A listing running on its worker thread. Dropping it cancels the listing.
#[derive(Debug)]
pub struct Listing {
    events: Receiver<Event>,
    cancel: Arc<AtomicBool>,
}

impl Listing {
    /// Starts listing `folder` (not its subfolders) on a worker thread.
    pub fn start(folder: impl Into<PathBuf>, options: Options) -> Self {
        let (tx, events) = mpsc::sync_channel(CHANNEL_CAPACITY);
        let cancel = Arc::new(AtomicBool::new(false));
        let job = Job {
            folder: folder.into(),
            chunk: options.chunk.max(1),
            folders_only: options.folders_only,
            cancel: cancel.clone(),
            tx,
            started: Instant::now(),
            deadline: options.deadline.map(|limit| Arc::new(Deadline::new(limit))),
        };
        // A thread that cannot start leaves the channel closed, which `collect` reports
        // as `Stopped`.
        let spawned = thread::Builder::new()
            .name("library-listing".into())
            .spawn(move || job.run());
        if let Err(error) = spawned {
            tracing::error!("cannot start the listing thread: {error}");
        }
        Self { events, cancel }
    }

    /// The events as they come: `recv`, `try_recv` or iterate. They end after `Done` or
    /// `Failed`.
    pub fn events(&self) -> &Receiver<Event> {
        &self.events
    }

    /// Asks the listing to stop. It ends with `Done` whose summary says `cancelled`,
    /// after the call it is in and the chunks already sent.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Waits for the whole listing.
    pub fn collect(self) -> Result<Folder, Error> {
        let mut folder = Folder {
            photos: Vec::new(),
            orphans: Vec::new(),
            subfolders: Vec::new(),
            summary: Summary::default(),
            deadline_passed: false,
        };
        for event in self.events.iter() {
            match event {
                Event::Chunk(mut photos) => folder.photos.append(&mut photos),
                Event::Orphans(mut orphans) => folder.orphans.append(&mut orphans),
                Event::Subfolders(mut names) => folder.subfolders.append(&mut names),
                Event::DeadlinePassed { .. } => folder.deadline_passed = true,
                Event::Done(summary) => {
                    folder.summary = summary;
                    return Ok(folder);
                }
                Event::Failed(error) => return Err(error),
            }
        }
        Err(Error::Stopped)
    }
}

impl Drop for Listing {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// The deadline, reported once by whichever of the worker and the timer gets there first.
#[derive(Debug)]
struct Deadline {
    limit: Duration,
    reported: AtomicBool,
}

impl Deadline {
    fn new(limit: Duration) -> Self {
        Self {
            limit,
            reported: AtomicBool::new(false),
        }
    }

    /// Sends the event, unless it has been sent. False when the caller has gone.
    fn report(&self, tx: &SyncSender<Event>) -> bool {
        if self.reported.swap(true, Ordering::Relaxed) {
            return true;
        }
        tx.send(Event::DeadlinePassed { limit: self.limit }).is_ok()
    }
}

/// A listing's worker.
struct Job {
    folder: PathBuf,
    chunk: usize,
    folders_only: bool,
    cancel: Arc<AtomicBool>,
    tx: SyncSender<Event>,
    started: Instant,
    deadline: Option<Arc<Deadline>>,
}

/// What the folder's entries make, before any file is measured.
#[derive(Default)]
struct Names {
    raws: Vec<String>,
    sidecars: HashSet<String>,
    folders: Vec<String>,
}

impl Job {
    fn run(self) {
        // The timer reports a deadline while the worker is inside a call that does not
        // return, which is what a deadline is for; the worker dropping `finished` ends it.
        let (finished, wait) = mpsc::channel::<()>();
        let mut timer = None;
        if let Some(deadline) = self.deadline.clone() {
            let tx = self.tx.clone();
            let spawned = thread::Builder::new()
                .name("library-deadline".into())
                .spawn(move || {
                    if wait.recv_timeout(deadline.limit) == Err(RecvTimeoutError::Timeout) {
                        deadline.report(&tx);
                    }
                });
            match spawned {
                Ok(handle) => timer = Some(handle),
                Err(error) => tracing::warn!("cannot start the deadline's thread: {error}"),
            }
        }
        let end = match self.list() {
            Ok(Some(summary)) => {
                tracing::debug!(
                    target: "perf",
                    "listed {}: {} raws, {} with sidecars, {} orphans, {} old temporaries \
                     removed in {:.1} ms{}",
                    self.folder.display(),
                    summary.raws,
                    summary.with_sidecar,
                    summary.orphans,
                    summary.stale_removed,
                    summary.elapsed.as_secs_f64() * 1e3,
                    if summary.cancelled { " (cancelled)" } else { "" },
                );
                Some(Event::Done(summary))
            }
            // The caller has gone.
            Ok(None) => None,
            Err(error) => Some(Event::Failed(error)),
        };
        // The timer ends before the end is sent, so no deadline event comes after it.
        drop(finished);
        if let Some(timer) = timer {
            let _ = timer.join();
        }
        if let Some(end) = end {
            let _ = self.tx.send(end);
        }
    }

    /// The listing; `None` when the caller has gone.
    fn list(&self) -> Result<Option<Summary>, Error> {
        let mut summary = Summary::default();
        let Some(Names {
            mut raws,
            sidecars,
            mut folders,
        }) = self.names(&mut summary)?
        else {
            return Ok(None);
        };
        if self.folders_only {
            raws.clear();
        }
        raws.sort_unstable();
        let raw_names: HashSet<&str> = raws.iter().map(String::as_str).collect();

        let mut chunk = Vec::with_capacity(self.chunk.min(raws.len()));
        for name in &raws {
            if self.stopped(&mut summary) {
                break;
            }
            if !self.check_deadline() {
                return Ok(None);
            }
            let Some(raw) = stat(&self.folder.join(name)) else {
                summary.unreadable += 1;
                continue;
            };
            let sidecar_name = format!("{name}{SUFFIX}");
            let sidecar = if sidecars.contains(&sidecar_name) {
                let sidecar = stat(&self.folder.join(&sidecar_name));
                summary.unreadable += usize::from(sidecar.is_none());
                sidecar
            } else {
                None
            };
            summary.raws += 1;
            summary.with_sidecar += usize::from(sidecar.is_some());
            chunk.push(Photo {
                name: name.clone(),
                raw,
                sidecar,
            });
            if chunk.len() >= self.chunk && !self.send(Event::Chunk(std::mem::take(&mut chunk))) {
                return Ok(None);
            }
        }
        if !chunk.is_empty() && !self.send(Event::Chunk(chunk)) {
            return Ok(None);
        }

        let mut orphans = Vec::new();
        let mut sidecars: Vec<&String> = sidecars.iter().filter(|_| !self.folders_only).collect();
        sidecars.sort_unstable();
        for name in sidecars {
            if self.stopped(&mut summary) {
                break;
            }
            let photo = name.strip_suffix(SUFFIX).unwrap_or(name);
            if raw_names.contains(photo) {
                continue;
            }
            match stat(&self.folder.join(name)) {
                Some(file) => orphans.push(Orphan {
                    name: name.clone(),
                    file,
                }),
                None => summary.unreadable += 1,
            }
        }
        summary.orphans = orphans.len();
        if !orphans.is_empty() && !self.send(Event::Orphans(orphans)) {
            return Ok(None);
        }
        folders.sort_unstable_by(|a, b| browser_order(a, b));
        if !folders.is_empty() && !self.send(Event::Subfolders(folders)) {
            return Ok(None);
        }
        summary.elapsed = self.started.elapsed();
        Ok(Some(summary))
    }

    /// Reads the folder's entries into raws, sidecars and the counts of the rest in
    /// `summary`, without measuring any file; `None` when the caller has gone. A
    /// cancellation stops the reading and is noted in `summary`.
    fn names(&self, summary: &mut Summary) -> Result<Option<Names>, Error> {
        let fail = |source| Error::List {
            path: self.folder.clone(),
            source,
        };
        if !self.check_deadline() {
            return Ok(None);
        }
        let mut names = Names::default();
        let now = SystemTime::now();
        #[cfg(test)]
        stall::wait(&self.folder);
        for entry in fs::read_dir(&self.folder).map_err(fail)? {
            if self.stopped(summary) {
                break;
            }
            let entry = entry.map_err(fail)?;
            let Ok(name) = entry.file_name().into_string() else {
                summary.not_text += 1;
                continue;
            };
            if name.starts_with('.') && self.folders_only {
                summary.hidden += 1;
                continue;
            }
            if name.starts_with('.') {
                // The one thing Apertrust removes from a person's folder (decision 53):
                // its own writer's leftovers, by their name and age alone.
                match apertrust_base::fs::remove_if_stale(&self.folder, &name, now) {
                    Ok(true) => summary.stale_removed += 1,
                    Ok(false) => summary.hidden += 1,
                    Err(error) => {
                        tracing::warn!(
                            "{}: cannot remove the old temporary file {name}: {error}",
                            self.folder.display()
                        );
                        summary.hidden += 1;
                    }
                }
                continue;
            }
            let is_folder = match kind(&entry, &name) {
                Kind::Folder { .. } => true,
                Kind::File => false,
                Kind::Hidden => {
                    summary.hidden += 1;
                    continue;
                }
                Kind::Unreadable => {
                    summary.unreadable += 1;
                    continue;
                }
            };
            if is_folder {
                summary.folders += 1;
                names.folders.push(name);
            } else if name.ends_with(SUFFIX) {
                names.sidecars.insert(name);
            } else if is_raw(&name) {
                names.raws.push(name);
            } else {
                summary.other += 1;
            }
            if !self.check_deadline() {
                return Ok(None);
            }
        }
        if summary.not_text > 0 {
            tracing::warn!(
                "{}: {} names that are not text, not listed",
                self.folder.display(),
                summary.not_text
            );
        }
        Ok(Some(names))
    }

    /// Whether the listing was cancelled, noted in `summary`.
    fn stopped(&self, summary: &mut Summary) -> bool {
        summary.cancelled |= self.cancel.load(Ordering::Relaxed);
        summary.cancelled
    }

    /// Reports the deadline if it has passed; false when the caller has gone.
    fn check_deadline(&self) -> bool {
        match &self.deadline {
            Some(deadline) if self.started.elapsed() >= deadline.limit => deadline.report(&self.tx),
            _ => true,
        }
    }

    /// False when the caller has gone.
    fn send(&self, event: Event) -> bool {
        self.tx.send(event).is_ok()
    }
}

/// A file's size and modified time, following links; `None` for what is gone or is no
/// file.
fn stat(path: &Path) -> Option<FileStat> {
    let meta = fs::metadata(path).ok().filter(fs::Metadata::is_file)?;
    Some(FileStat {
        size: meta.len(),
        modified: meta.modified().ok(),
    })
}

/// What a folder's entry is, by the one rule the listing and the tree's walk share
/// (`docs/library.md`, The listing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// Hidden: a name that starts with a dot, or a folder the system's own flag hides
    /// (Finder's `UF_HIDDEN` on the Mac, the hidden attribute on Windows).
    Hidden,
    /// A folder, or, `link`, a link to one, named as a subfolder but never walked into.
    Folder { link: bool },
    /// Anything else, a Mac package among them, a link to a file too: a raw where its
    /// name says so.
    File,
    /// Its type could not be read.
    Unreadable,
}

/// What `entry`, named `name`, is ([`Kind`]). A folder's flags are read where the system
/// keeps them, one `lstat` a folder on the Mac and the listing's own data on Windows; a
/// link is followed once to see whether it points to a folder.
pub(crate) fn kind(entry: &fs::DirEntry, name: &str) -> Kind {
    if name.starts_with('.') {
        return Kind::Hidden;
    }
    let Ok(file_type) = entry.file_type() else {
        return Kind::Unreadable;
    };
    if file_type.is_symlink() {
        // A link counts as what it points to; a junction on Windows is one too.
        let folder = fs::metadata(entry.path()).is_ok_and(|meta| meta.is_dir());
        return if folder {
            Kind::Folder { link: true }
        } else {
            Kind::File
        };
    }
    if !file_type.is_dir() || package(name) {
        Kind::File
    } else if hidden_by_flag(entry) {
        Kind::Hidden
    } else {
        Kind::Folder { link: false }
    }
}

/// The extensions of the Mac's packages, folders Finder shows as one file: never listed
/// as subfolders or walked into.
#[cfg(target_os = "macos")]
const PACKAGES: [&str; 5] = ["app", "bundle", "framework", "photoslibrary", "plugin"];

/// Whether the folder `name` is a Mac package, by its extension ([`PACKAGES`]).
#[cfg(target_os = "macos")]
fn package(name: &str) -> bool {
    PACKAGES
        .iter()
        .any(|extension| has_extension(name, extension))
}

/// Elsewhere a folder is a folder, whatever its name.
#[cfg(not(target_os = "macos"))]
fn package(_name: &str) -> bool {
    false
}

/// Whether Finder's hidden flag is set on the folder `entry` names; a folder whose flags
/// cannot be read is shown.
#[cfg(target_os = "macos")]
fn hidden_by_flag(entry: &fs::DirEntry) -> bool {
    use std::os::macos::fs::MetadataExt as _;
    /// `UF_HIDDEN` in `<sys/stat.h>`: Finder hides the item.
    const UF_HIDDEN: u32 = 0x8000;
    entry
        .metadata()
        .is_ok_and(|meta| meta.st_flags() & UF_HIDDEN != 0)
}

/// Whether the hidden attribute is set on the folder `entry` names, from the directory
/// listing's own data; a folder whose attributes cannot be read is shown.
#[cfg(windows)]
fn hidden_by_flag(entry: &fs::DirEntry) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    entry
        .metadata()
        .is_ok_and(|meta| meta.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0)
}

/// Elsewhere a dot name alone hides a folder.
#[cfg(not(any(target_os = "macos", windows)))]
fn hidden_by_flag(_entry: &fs::DirEntry) -> bool {
    false
}

/// Whether `name` ends in `.extension`, in any case.
fn has_extension(name: &str, extension: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(_, found)| found.eq_ignore_ascii_case(extension))
}

/// Whether the decoder reads files of this name, by its extension in any case.
pub(crate) fn is_raw(name: &str) -> bool {
    apertrust_raw::extensions()
        .iter()
        .any(|extension| has_extension(name, extension))
}

/// The order the system's file browser lists names in, as macOS's Finder
/// (`localizedStandardCompare:`) and Windows' Explorer (`StrCmpLogicalW`) do: spaces,
/// punctuation and symbols before digits and digits before letters, letters without
/// regard to case, and a run of digits by its number, so `Day 2` comes before `Day 10`;
/// names equal so are ordered as their text, so the order is total.
///
/// An approximation: Rust's standard library decomposes no character, so an accented
/// letter is not compared by its base letter, as the system's collation compares it, but
/// sorts after every unaccented one (`École` after `Zoo`), and no language's own order of
/// its letters is followed.
pub fn browser_order(a: &str, b: &str) -> cmp::Ordering {
    let (mut left, mut right) = (a.chars().peekable(), b.chars().peekable());
    loop {
        let order = match (left.peek().copied(), right.peek().copied()) {
            (None, None) => return a.cmp(b),
            (None, Some(_)) => return cmp::Ordering::Less,
            (Some(_), None) => return cmp::Ordering::Greater,
            (Some(l), Some(r)) if l.is_ascii_digit() && r.is_ascii_digit() => {
                let (l, r) = (number(&mut left), number(&mut right));
                // Without its leading zeros, the longer run is the larger number.
                l.len().cmp(&r.len()).then_with(|| l.cmp(&r))
            }
            (Some(l), Some(r)) => {
                left.next();
                right.next();
                rank(l)
                    .cmp(&rank(r))
                    .then_with(|| l.to_lowercase().cmp(r.to_lowercase()))
            }
        };
        if order != cmp::Ordering::Equal {
            return order;
        }
    }
}

/// Where a character's kind sorts: spaces, punctuation and symbols first, then digits,
/// then letters.
fn rank(c: char) -> u8 {
    if c.is_numeric() {
        1
    } else if c.is_alphabetic() {
        2
    } else {
        0
    }
}

/// The run of digits `chars` starts with, taken, without its leading zeros.
fn number(chars: &mut Peekable<Chars<'_>>) -> String {
    let mut run = String::new();
    while let Some(c) = chars.next_if(char::is_ascii_digit) {
        if !(run.is_empty() && c == '0') {
            run.push(c);
        }
    }
    run
}

/// A test's hook: a listing of a folder held here stops before it reads the folder, as a
/// call into a share that does not return would, until the folder is released.
#[cfg(test)]
pub(crate) mod stall {
    use std::path::{Path, PathBuf};
    use std::sync::{Condvar, Mutex, PoisonError};

    static HELD: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
    static RELEASED: Condvar = Condvar::new();

    /// Holds every listing of `folder` until [`release`].
    pub(crate) fn hold(folder: &Path) {
        HELD.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(folder.to_owned());
    }

    pub(crate) fn release(folder: &Path) {
        HELD.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|held| held != folder);
        RELEASED.notify_all();
    }

    /// Waits while `folder` is held.
    pub(crate) fn wait(folder: &Path) {
        let mut held = HELD.lock().unwrap_or_else(PoisonError::into_inner);
        while held.iter().any(|held| held == folder) {
            held = RELEASED.wait(held).unwrap_or_else(PoisonError::into_inner);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::browser_order;

    #[test]
    fn names_sort_as_a_file_browser_lists_them() {
        let mut names = vec![
            "day 10", "Day 2", "day 1", "Archive", "archive", "b", "Day 02", "_misc", "2024",
            "100", "(old)", "day-1", "zoo",
        ];
        names.sort_by(|a, b| browser_order(a, b));
        assert_eq!(
            names,
            [
                "(old)", "_misc", "100", "2024", "Archive", "archive", "b", "day 1", "Day 02",
                "Day 2", "day 10", "day-1", "zoo",
            ]
        );
    }
}
