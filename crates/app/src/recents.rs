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

//! The recents (`docs/gui.md`, File › Open Recent; `docs/settings.md`, `state.toml`): the
//! one list of what was opened, which Open Recent, Browse and the start window all read.
//!
//! - [`Recents`] holds the [`Entry`]s, newest first, at most [`MOST`]: one opened goes to
//!   the top, after the newest even when the clock was set back; [`Recents::remove`] is
//!   Remove from the List, [`Recents::repoint`] what Locate… does once a path is chosen,
//!   [`Recents::clear`] Clear Menu.
//! - They are kept in `state.toml` in the user data folder, beside `settings.toml` and
//!   apart from it, since they name photos' places. A writer thread of the list's own
//!   writes them through the safe write ([`apertrust_base::fs`]), merging each change made
//!   since its last write into the file as it then is, so two Apertrusts on one folder
//!   keep each other's entries; the file's other keys stay. A change it could not write
//!   is kept and merged again with the next one, and on exit.
//! - An entry this build cannot read, of a kind it lacks or malformed, stays in the file
//!   and in its place, unshown.
//! - Whether an entry's path is there is asked by [`Recents::check`] on a thread an entry,
//!   never the caller's, so a slow share never holds it, and a path still being asked is
//!   not asked again; an entry found missing is marked ([`Recents::missing`]), one not yet
//!   answered is not, and an answer to a question asked before the entry was opened again
//!   is dropped.
//! - **What was open last** ([`Last`]) is kept beside the list, as `[last]`, for the next
//!   start to reopen (`docs/gui.md`, The start window): the folder a way in opened, the
//!   folder chosen in its tree and the photo active as the app ended, never a photo opened
//!   alone; File › Close takes it out ([`Recents::set_last`]). A write merges it as it does the list, the newest winning;
//!   one this build cannot read is read as none and left in the file until replaced.
//! - A damaged `state.toml`, one that is not TOML or not UTF-8, is set aside as
//!   `state.toml.bak` with a warning naming no path in it: read at the start, the list
//!   starts empty; met by a later write, what the last write left is written. One a newer
//!   Apertrust wrote is read as empty and never written.

use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

use toml::{Table, Value};

/// How many entries are kept, Open Recent's ten (`docs/gui.md`, File).
pub const MOST: usize = 10;

/// The file's version: what this build writes, and the newest it reads.
const VERSION: i64 = 1;

/// The key of the list in the file.
const KEY: &str = "recent";

/// The key of what was open last.
const LAST: &str = "last";

/// What an entry is. Libraries and projects join folders in Phase 5.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Folder,
}

impl Kind {
    /// Its id in the file.
    const fn id(self) -> &'static str {
        match self {
            Kind::Folder => "folder",
        }
    }

    fn from_id(id: &str) -> Option<Kind> {
        match id {
            "folder" => Some(Kind::Folder),
            _ => None,
        }
    }
}

/// One thing opened: its kind, its path and when it was last opened, in whole seconds
/// since 1970.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub kind: Kind,
    pub path: PathBuf,
    pub opened: u64,
}

/// What was open last, for the next start to reopen: a folder a way in opened, the tree's
/// root, never a photo opened alone; the folder of its tree active as the app ended where
/// it was another, chosen in the tree; and the file name of the photo active then, in the
/// active folder, if any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Last {
    pub kind: Kind,
    pub path: PathBuf,
    pub folder: Option<PathBuf>,
    pub photo: Option<String>,
}

impl Last {
    /// What `value` holds, or None where this build cannot read it.
    fn read(value: &Value) -> Option<Last> {
        let table = value.as_table()?;
        let kind = Kind::from_id(table.get("kind")?.as_str()?)?;
        let path = PathBuf::from(table.get("path")?.as_str()?);
        let folder = match table.get("folder") {
            Some(folder) => Some(PathBuf::from(folder.as_str()?)),
            None => None,
        };
        let photo = match table.get("photo") {
            Some(photo) => Some(photo.as_str()?.to_owned()),
            None => None,
        };
        Some(Last {
            kind,
            path,
            folder,
            photo,
        })
    }

    fn value(&self) -> Value {
        let mut table = Table::new();
        table.insert("kind".into(), self.kind.id().into());
        let path = self.path.to_string_lossy().into_owned();
        table.insert("path".into(), path.into());
        if let Some(folder) = &self.folder {
            let folder = folder.to_string_lossy().into_owned();
            table.insert("folder".into(), folder.into());
        }
        if let Some(photo) = &self.photo {
            table.insert("photo".into(), photo.clone().into());
        }
        Value::Table(table)
    }
}

/// An entry as the file holds it: one this build reads, or one it cannot, kept as it is.
#[derive(Clone, Debug, PartialEq)]
enum Item {
    Known(Entry),
    Unknown(Value),
}

impl Item {
    /// The entry `value` holds, or None where it is one this build cannot read: of a kind
    /// it lacks, or malformed.
    fn known(value: &Value) -> Option<Entry> {
        let table = value.as_table()?;
        let kind = Kind::from_id(table.get("kind")?.as_str()?)?;
        let path = PathBuf::from(table.get("path")?.as_str()?);
        let opened = u64::try_from(table.get("opened")?.as_integer()?).ok()?;
        Some(Entry { kind, path, opened })
    }

    fn value(&self) -> Value {
        match self {
            Item::Known(entry) => {
                let mut table = Table::new();
                table.insert("kind".into(), entry.kind.id().into());
                let path = entry.path.to_string_lossy().into_owned();
                table.insert("path".into(), path.into());
                let opened = i64::try_from(entry.opened).unwrap_or(i64::MAX);
                table.insert("opened".into(), opened.into());
                Value::Table(table)
            }
            Item::Unknown(value) => value.clone(),
        }
    }

    fn opened(&self) -> u64 {
        match self {
            Item::Known(entry) => entry.opened,
            Item::Unknown(value) => value
                .get("opened")
                .and_then(Value::as_integer)
                .and_then(|n| u64::try_from(n).ok())
                .unwrap_or(0),
        }
    }

    fn is(&self, kind: Kind, path: &Path) -> bool {
        matches!(self, Item::Known(e) if e.kind == kind && e.path == path)
    }
}

/// What changed in the list, as its writer merges it into the file.
#[derive(Clone, Debug, PartialEq)]
enum Change {
    Opened(Entry),
    Removed(Kind, PathBuf),
    Repointed(Kind, PathBuf, PathBuf),
    Cleared,
    /// The list written as it is, even empty: the move of the old setting's.
    Kept,
    /// What was open last, or none.
    Last(Option<Last>),
}

/// Applies `change` to `items`, newest first, keeping at most [`MOST`]. An entry opened
/// goes to the top: after the newest of the others, should the clock have been set back.
fn apply(items: &mut Vec<Item>, change: &Change) {
    match change {
        Change::Opened(entry) => {
            items.retain(|item| !item.is(entry.kind, &entry.path));
            let newest = items.iter().map(Item::opened).max();
            let opened = newest.map_or(entry.opened, |n| entry.opened.max(n.saturating_add(1)));
            let entry = Entry {
                opened,
                ..entry.clone()
            };
            items.insert(0, Item::Known(entry));
        }
        Change::Removed(kind, path) => items.retain(|item| !item.is(*kind, path)),
        Change::Repointed(kind, from, to) => {
            if from == to || !items.iter().any(|item| item.is(*kind, from)) {
                return;
            }
            items.retain(|item| !item.is(*kind, to));
            for item in items.iter_mut() {
                if let Item::Known(entry) = item
                    && entry.kind == *kind
                    && entry.path == *from
                {
                    entry.path.clone_from(to);
                }
            }
        }
        Change::Cleared => items.retain(|item| matches!(item, Item::Unknown(_))),
        Change::Kept | Change::Last(_) => {}
    }
    // Newest first; the sort is stable, so an entry opened in the same second as another
    // stays where it was put.
    items.sort_by_key(|item| std::cmp::Reverse(item.opened()));
    items.truncate(MOST);
}

/// What the file holds.
#[derive(Debug, Default)]
struct Read {
    table: Table,
    /// None where the file has no list yet.
    items: Option<Vec<Item>>,
    /// What was open last, where the file holds one this build reads.
    last: Option<Last>,
    /// Whether it was damaged, and set aside.
    damaged: bool,
    /// Whether it may be written: not one a newer Apertrust wrote, nor one that could not
    /// be read.
    writable: bool,
    warnings: Vec<String>,
}

/// Reads `file`. A file that is not there holds no list; a damaged one, not TOML or not
/// UTF-8, is set aside as `state.toml.bak`.
fn read(file: &Path) -> Read {
    let mut read = Read {
        writable: true,
        ..Read::default()
    };
    let text = match std::fs::read(file) {
        Ok(bytes) => String::from_utf8(bytes).map_err(|_| "not UTF-8".to_owned()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => return read,
        Err(e) => {
            read.warnings
                .push(format!("not read: {e}; nothing written to it"));
            read.writable = false;
            read.items = Some(Vec::new());
            return read;
        }
    };
    // What the error says, never `Display`'s quote of the line, which may name a photo's
    // folder.
    let table = text.and_then(|text| {
        text.parse::<Table>()
            .map_err(|e| e.message().replace('\n', " "))
    });
    let table = match table {
        Ok(table) => table,
        Err(why) => {
            let aside = beside(file, ".bak");
            let moved = match std::fs::rename(file, &aside) {
                Ok(()) => format!("set aside as {}", aside.display()),
                Err(e) => format!("could not be set aside: {}", e.kind()),
            };
            read.warnings.push(format!("damaged, {moved}: {why}"));
            read.damaged = true;
            read.items = Some(Vec::new());
            return read;
        }
    };
    let version = table.get("version").and_then(Value::as_integer);
    if let Some(version) = version.filter(|v| *v > VERSION) {
        let warning = format!("version {version} is newer than this build's; never written");
        read.warnings.push(warning);
        read.writable = false;
        read.items = Some(Vec::new());
        return read;
    }
    if let Some(list) = table.get(KEY) {
        let mut items = Vec::new();
        for (i, value) in list.as_array().into_iter().flatten().enumerate() {
            items.push(match Item::known(value) {
                Some(entry) => Item::Known(entry),
                None => {
                    let known = value
                        .get("kind")
                        .and_then(Value::as_str)
                        .and_then(Kind::from_id);
                    if known.is_some() {
                        read.warnings
                            .push(format!("{KEY}[{i}] cannot be read; kept as it is"));
                    }
                    Item::Unknown(value.clone())
                }
            });
        }
        if !list.is_array() {
            read.warnings
                .push(format!("{KEY} is no list; read as empty"));
        }
        items.sort_by_key(|item| std::cmp::Reverse(item.opened()));
        items.truncate(MOST);
        read.items = Some(items);
    }
    read.last = table.get(LAST).and_then(Last::read);
    read.table = table;
    read
}

/// `path` with `suffix` after its whole name.
fn beside(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Merges `changes` into `file` as it is now and writes it through the safe write, its
/// other keys kept. A damaged file, set aside, gives way to `last`, what the last write
/// left; one that may not be written is an error. Returns the list written.
fn merge(file: &Path, changes: &[Change], last: &[Item]) -> io::Result<Vec<Item>> {
    let read = read(file);
    for warning in &read.warnings {
        tracing::warn!(target: "settings", "{}: {warning}", file.display());
    }
    if !read.writable {
        return Err(io::Error::other("it may not be written"));
    }
    let (mut table, mut items) = if read.damaged {
        (Table::new(), last.to_vec())
    } else {
        (read.table, read.items.unwrap_or_default())
    };
    for change in changes {
        apply(&mut items, change);
        if let Change::Last(last) = change {
            match last {
                Some(last) => table.insert(LAST.into(), last.value()),
                None => table.remove(LAST),
            };
        }
    }
    table.insert("version".into(), VERSION.into());
    let list = items.iter().map(Item::value);
    table.insert(KEY.into(), Value::Array(list.collect()));
    let text = toml::to_string(&table).map_err(io::Error::other)?;
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder)?;
    }
    apertrust_base::fs::write(file, text.as_bytes())?;
    Ok(items)
}

/// What the writer thread is asked.
enum Ask {
    Merge(Vec<Change>),
    /// Answer once everything asked before has been written, or tried again.
    Done(Sender<()>),
}

/// The thread that writes the file, one merge at a time, so two of this process's never
/// overlap. It keeps the changes it could not write, for the next merge.
struct Writer {
    asks: Option<Sender<Ask>>,
    thread: Option<JoinHandle<()>>,
}

impl Writer {
    /// A writer of `file`, whose last write left `last`.
    fn start(file: PathBuf, last: Vec<Item>) -> Option<Writer> {
        let (asks, asked) = mpsc::channel::<Ask>();
        let thread = std::thread::Builder::new()
            .name("recents writer".into())
            .spawn(move || {
                let mut last = last;
                let mut unwritten: Vec<Change> = Vec::new();
                let mut write = |unwritten: &mut Vec<Change>| {
                    match merge(&file, unwritten, &last) {
                        Ok(written) => {
                            last = written;
                            unwritten.clear();
                        }
                        Err(e) => {
                            let file = file.display();
                            tracing::warn!(target: "settings", "{file}: not written, kept for the next write: {e}");
                        }
                    }
                };
                for ask in asked {
                    match ask {
                        Ask::Merge(changes) => {
                            unwritten.extend(changes);
                            write(&mut unwritten);
                        }
                        Ask::Done(done) => {
                            if !unwritten.is_empty() {
                                write(&mut unwritten);
                            }
                            let _ = done.send(());
                        }
                    }
                }
            });
        match thread {
            Ok(thread) => Some(Writer {
                asks: Some(asks),
                thread: Some(thread),
            }),
            Err(e) => {
                tracing::warn!(target: "settings", "recents never written: no thread: {e}");
                None
            }
        }
    }

    fn send(&self, ask: Ask) -> bool {
        self.asks
            .as_ref()
            .is_some_and(|asks| asks.send(ask).is_ok())
    }
}

impl Drop for Writer {
    /// Writes what was asked, then ends the thread.
    fn drop(&mut self) {
        self.asks.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// A path asked about, with the question's answers still to come.
type Key = (Kind, PathBuf);

/// The list of what was opened, newest first, kept in `state.toml`.
pub struct Recents {
    items: Vec<Item>,
    /// What was open last.
    last: Option<Last>,
    /// What changed since the last [`save`](Recents::save).
    changes: Vec<Change>,
    /// The entries a check found missing.
    missing: HashSet<Key>,
    /// The paths being asked about, each with whether its answer still counts: not once
    /// its entry was opened, removed or re-pointed after the question.
    asking: HashMap<Key, bool>,
    /// Where the checks' threads answer, and the end they answer to.
    answers: Receiver<(Kind, PathBuf, Option<bool>)>,
    answer: Sender<(Kind, PathBuf, Option<bool>)>,
    writer: Option<Writer>,
}

impl Recents {
    /// A list kept nowhere, empty: for a system with no user data folder, and for tests.
    pub fn nowhere() -> Recents {
        let (answer, answers) = mpsc::channel();
        Recents {
            items: Vec::new(),
            last: None,
            changes: Vec::new(),
            missing: HashSet::new(),
            asking: HashMap::new(),
            answers,
            answer,
            writer: None,
        }
    }

    /// The list `file` holds, with the warnings its reading gave, a writer thread started
    /// for it. Where the file holds no list yet, the folders of `old`, newest first, become
    /// it and are written at once: settings' `library.recent_folders`, read for the move,
    /// so it moves once.
    pub fn open(file: PathBuf, old: impl FnOnce() -> Vec<PathBuf>) -> (Recents, Vec<String>) {
        let Read {
            items,
            last,
            writable,
            warnings,
            ..
        } = read(&file);
        let mut recents = Recents::nowhere();
        recents.last = last;
        if writable {
            recents.writer = Writer::start(file, items.clone().unwrap_or_default());
        }
        match items {
            Some(items) => recents.items = items,
            None => {
                let now = now();
                // Oldest first, a second apart, so the newest stays at the top.
                for (i, path) in old().into_iter().take(MOST).enumerate().rev() {
                    let opened = now.saturating_sub(i as u64);
                    recents.opened_at(Kind::Folder, path, opened);
                }
                // The list, even an empty one, so the old setting is never read again.
                recents.changes.push(Change::Kept);
                recents.save();
            }
        }
        (recents, warnings)
    }

    /// The entries this build reads, newest first.
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.items.iter().filter_map(|item| match item {
            Item::Known(entry) => Some(entry),
            Item::Unknown(_) => None,
        })
    }

    /// `path` was opened now: it goes to the top, once, and is no longer missing.
    pub fn opened(&mut self, kind: Kind, path: PathBuf) {
        self.opened_at(kind, path, now());
    }

    /// `path` was opened at `opened`, in seconds since 1970, or after the newest entry
    /// where that is later.
    pub fn opened_at(&mut self, kind: Kind, path: PathBuf, opened: u64) {
        self.forget(kind, &path);
        self.change(Change::Opened(Entry { kind, path, opened }));
    }

    /// Remove from the List: takes `path` out. Returns whether it was there.
    pub fn remove(&mut self, kind: Kind, path: &Path) -> bool {
        let there = self.items.iter().any(|item| item.is(kind, path));
        self.forget(kind, path);
        self.change(Change::Removed(kind, path.to_owned()));
        there
    }

    /// What Locate… does once a new path is chosen: the entry at `from` points to `to`,
    /// keeping its place, no longer missing. Returns whether it was there; `to` the same as
    /// `from` changes nothing.
    pub fn repoint(&mut self, kind: Kind, from: &Path, to: PathBuf) -> bool {
        let there = self.items.iter().any(|item| item.is(kind, from));
        if !there || from == to {
            return there;
        }
        self.forget(kind, from);
        self.forget(kind, &to);
        self.change(Change::Repointed(kind, from.to_owned(), to));
        there
    }

    /// Clear Menu: every entry this build reads goes. What was open last stays.
    pub fn clear(&mut self) {
        self.missing.clear();
        for still in self.asking.values_mut() {
            *still = false;
        }
        self.change(Change::Cleared);
    }

    /// What was open last, as the file held it at the start or as last set.
    pub fn last(&self) -> Option<&Last> {
        self.last.as_ref()
    }

    /// What was open last is now `last`, or none: written with the list's next save, and
    /// not at all where it is the same.
    pub fn set_last(&mut self, last: Option<Last>) {
        if self.last != last {
            self.last.clone_from(&last);
            self.changes.push(Change::Last(last));
        }
    }

    /// Marks `path` there or missing, as a check would.
    #[cfg(test)]
    pub fn mark(&mut self, kind: Kind, path: &Path, there: bool) {
        let key = (kind, path.to_owned());
        if there {
            self.missing.remove(&key);
        } else if self.items.iter().any(|item| item.is(kind, path)) {
            self.missing.insert(key);
        }
    }

    /// Whether a check found `path` missing.
    pub fn missing(&self, kind: Kind, path: &Path) -> bool {
        self.missing.contains(&(kind, path.to_owned()))
    }

    /// Asks whether each entry's path is there, each on a thread of its own, never the
    /// caller's, but none still being asked: [`take`](Recents::take) marks the answers as
    /// they come. A path that cannot be asked, as a share that refuses, leaves its entry
    /// unmarked. A share that hangs keeps its thread until it answers, and so does a drop's
    /// question of the disk (`dropped::Drops`), which asks the same way.
    pub fn check(&mut self) {
        let keys: Vec<Key> = self.entries().map(|e| (e.kind, e.path.clone())).collect();
        for key in keys {
            if self.asking.contains_key(&key) {
                continue;
            }
            let ((kind, path), answer) = (key.clone(), self.answer.clone());
            let asked = std::thread::Builder::new()
                .name("recents check".into())
                .spawn(move || {
                    let there = there(kind, &path);
                    let _ = answer.send((kind, path, there));
                });
            match asked {
                Ok(_) => {
                    self.asking.insert(key, true);
                }
                Err(e) => tracing::warn!(target: "settings", "recents not checked: no thread: {e}"),
            }
        }
    }

    /// Marks the answers that have come, without waiting for the others, but an answer to
    /// a question asked before its entry was opened, removed or re-pointed. Returns
    /// whether any marked something.
    pub fn take(&mut self) -> bool {
        let mut marked = false;
        while let Ok((kind, path, there)) = self.answers.try_recv() {
            let key = (kind, path);
            let counts = self.asking.remove(&key).unwrap_or(false);
            let listed = self.items.iter().any(|item| item.is(kind, &key.1));
            match there {
                Some(false) if counts && listed => marked |= self.missing.insert(key),
                Some(true) if counts => marked |= self.missing.remove(&key),
                _ => {}
            }
        }
        marked
    }

    /// Hands what changed since the last save to the writer thread, which merges it into
    /// the file; nothing waits for it.
    pub fn save(&mut self) {
        if self.changes.is_empty() {
            return;
        }
        let changes = std::mem::take(&mut self.changes);
        if let Some(writer) = &self.writer {
            writer.send(Ask::Merge(changes));
        }
    }

    /// Saves, and waits until the writer thread has written everything, or tried what it
    /// could not again: on exit, and in tests.
    pub fn flush(&mut self) {
        self.save();
        if let Some(writer) = &self.writer {
            let (done, wait) = mpsc::channel();
            if writer.send(Ask::Done(done)) {
                let _ = wait.recv();
            }
        }
    }

    /// Forgets what is known of `path`'s being there: it is no longer missing, and a
    /// question about it still under way no longer counts.
    fn forget(&mut self, kind: Kind, path: &Path) {
        let key = (kind, path.to_owned());
        self.missing.remove(&key);
        if let Some(still) = self.asking.get_mut(&key) {
            *still = false;
        }
    }

    fn change(&mut self, change: Change) {
        apply(&mut self.items, &change);
        self.changes.push(change);
    }
}

/// Whether `path` is there as a `kind`: a folder that is a folder. None where it cannot be
/// told.
fn there(kind: Kind, path: &Path) -> Option<bool> {
    match std::fs::metadata(path) {
        Ok(metadata) => Some(match kind {
            Kind::Folder => metadata.is_dir(),
        }),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Some(false),
        Err(_) => None,
    }
}

/// Now, in whole seconds since 1970.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests;
