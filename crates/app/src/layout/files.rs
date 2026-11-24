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

//! Writing a workspace's file (decision 96, The workspace's file): each move a person makes
//! is kept, and written whole, through the safe write ([`apertrust_base::fs`]), on a
//! thread of its own, once the layout has stood still for [`DELAY`], and at once when
//! another workspace is chosen and on exit ([`Files`]).
//!
//! - **A write reads the file again first** and applies the moves made since the last
//!   write to it as it is then ([`write()`]), so a hand edit made meanwhile stays: each move
//!   names its tool and tabs by id ([`Move`]), and a Toolbar change is the whole list as it
//!   now is, written over the file's `toolbar`, the one change that writes it, and Reset
//!   Toolbar takes it out (decision 92). The rest of the file, its tabs, its `name`, a key
//!   this build does not know, stays as it is; ids this build does not know keep their
//!   places ([`Layout::settled`]).
//! - **A built-in workspace** has no file until a person first changes it: its first write
//!   starts from its bundled file, and so does one whose file was deleted by hand while
//!   the app runs, the moves not yet written applied to it. A person's own whose file is
//!   gone is never written again: it was taken away, and goes at the next start.
//! - **Never written:** a newer build's file, which this build never overwrites, the app
//!   told of one it did not know as such ([`Files::newer`]), with a warning. One listing
//!   no tab, perhaps for a moment as a person edits it, is left as it is, its moves tried
//!   again as a failed write's are. One that is not TOML since it was read is set aside as
//!   `<id>.toml.bak`, as reading sets it aside, and written from the last write, or a
//!   built-in's bundled file ([`Kept`]).
//! - **A write that fails** keeps its moves, tried again with the next write, the next
//!   switch and on exit, a file it set aside then written as it would have been.
//! - **Written** with `toml`'s pretty form, one list item a line; a comment written by hand
//!   is not kept, which needs `toml_edit`.
//! - **Nothing is written** while nothing moved: opening and closing the app writes
//!   nothing.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use toml::Table;

use super::{BUILT_IN, DEFAULT, Layout, Move, VERSION, Workspace, aside, known, table_of};

/// How long the layout stands still before its moves are written.
pub const DELAY: Duration = Duration::from_secs(1);

/// The workspaces' files in the user data folder's `workspaces`: the moves made since the
/// last write, written once the layout has stood still ([`Files::poll`]), at once on a
/// switch ([`Files::write_now`]) and on exit ([`Files::flush`]).
pub struct Files {
    /// Where the files are, none for nowhere: nothing is kept.
    folder: Option<PathBuf>,
    /// The moves made since the last write, by workspace id, each with whether it is built
    /// in.
    moved: BTreeMap<String, (bool, Vec<Move>)>,
    /// Since when the layout has stood still with moves unwritten.
    changed: Option<Instant>,
    writer: Option<Writer>,
    /// The ids of the workspaces whose file the writer found a newer build's, which the
    /// app did not know as it read them ([`Files::newer`]).
    newer: Option<Receiver<String>>,
}

impl Files {
    /// Keeps nothing: with no user data folder, and in tests that write none.
    pub fn nowhere() -> Files {
        Files {
            folder: None,
            moved: BTreeMap::new(),
            changed: None,
            writer: None,
            newer: None,
        }
    }

    /// [`Files::waking`] with no one to wake: [`Files::newer`] says what the writer found
    /// when next asked.
    #[cfg(test)]
    pub fn new(folder: PathBuf) -> Files {
        Files::waking(folder, || {})
    }

    /// Keeps the workspaces' files in `folder`, the user data folder's `workspaces`,
    /// written by a thread of its own, which calls `wake` once it has found a newer build's
    /// file, so the window, idle, draws a frame that takes it in ([`Files::newer`]): the
    /// app's `egui::Context::request_repaint`.
    pub fn waking(folder: PathBuf, wake: impl Fn() + Send + 'static) -> Files {
        let (told, newer) = mpsc::channel();
        Files {
            writer: Writer::start(folder.clone(), told, Box::new(wake)),
            folder: Some(folder),
            newer: Some(newer),
            ..Files::nowhere()
        }
    }

    /// The ids of the workspaces whose file the writer has found a newer build's since
    /// last asked, which it left as they are: a newer Apertrust wrote them after this one
    /// read them.
    pub fn newer(&self) -> Vec<String> {
        let found = self.newer.iter().flat_map(Receiver::try_iter);
        found.collect()
    }

    /// Takes in `change`, applied at `now` to `workspace`, to be written once the layout
    /// has stood still. A newer build's file is never written, nor anything with no
    /// folder.
    pub fn moved(&mut self, workspace: &Workspace, change: Move, now: Instant) {
        if self.folder.is_none() || workspace.newer {
            return;
        }
        let entry = self.moved.entry(workspace.id.clone());
        entry
            .or_insert((workspace.builtin, Vec::new()))
            .1
            .push(change);
        self.changed = Some(now);
    }

    /// Looks at the layout at `now`: hands the moves to the writer once it has stood still
    /// for [`DELAY`], and otherwise says how long until it will have, for a frame then.
    pub fn poll(&mut self, now: Instant) -> Option<Duration> {
        let still = now.saturating_duration_since(self.changed?);
        if still < DELAY {
            return Some(DELAY - still);
        }
        self.write_now();
        None
    }

    /// Hands every move made since the last write to the writer at once: as another
    /// workspace is chosen. With none, it has the writer try again what a write could not
    /// write, if anything.
    pub fn write_now(&mut self) {
        self.changed = None;
        let (Some(folder), Some(writer)) = (&self.folder, &self.writer) else {
            self.moved.clear();
            return;
        };
        if self.moved.is_empty() {
            writer.send(Ask::Retry);
        }
        for (id, (builtin, moves)) in std::mem::take(&mut self.moved) {
            let file = folder.join(format!("{id}.toml"));
            writer.send(Ask::Write {
                file,
                id,
                builtin,
                moves,
            });
        }
    }

    /// Waits until everything handed to the writer has been written, or tried.
    #[cfg(test)]
    pub fn wait(&self) {
        let Some(writer) = &self.writer else {
            return;
        };
        let (done, waited) = mpsc::channel();
        if writer.send(Ask::Done(done)) {
            let _ = waited.recv();
        }
    }

    /// On exit: writes every move made since the last write, waits for it, and ends the
    /// writer. Nothing is written after.
    pub fn flush(&mut self) {
        self.write_now();
        self.writer = None;
    }
}

/// What the writer thread is asked.
enum Ask {
    /// Applies `moves` to the workspace `id`'s file, `file`, and writes it.
    Write {
        file: PathBuf,
        id: String,
        builtin: bool,
        moves: Vec<Move>,
    },
    /// Tries again what a write could not write.
    Retry,
    /// Answers once everything asked before has been written, or tried.
    #[cfg(test)]
    Done(Sender<()>),
}

/// The moves a write could not write, by file, with the workspace's id and whether it is
/// built in.
type Unwritten = BTreeMap<PathBuf, (String, bool, Vec<Move>)>;

/// The thread that writes the files, one at a time, keeping what each write left and the
/// moves a failed write could not write.
struct Writer {
    asks: Option<Sender<Ask>>,
    thread: Option<JoinHandle<()>>,
}

impl Writer {
    /// A writer of the files in `folder`, or none where no thread starts, with a warning;
    /// it tells `told` the id of each workspace whose file it finds a newer build's, then
    /// calls `wake`.
    fn start(folder: PathBuf, told: Sender<String>, wake: Box<dyn Fn() + Send>) -> Option<Writer> {
        let (asks, asked) = mpsc::channel::<Ask>();
        let thread = std::thread::Builder::new()
            .name("workspaces writer".into())
            .spawn(move || {
                // What it keeps of each file between writes, by file.
                let mut kept: HashMap<PathBuf, Kept> = HashMap::new();
                // The moves a write could not write, by file, with the workspace's id and
                // whether it is built in.
                let mut unwritten = Unwritten::new();
                // The files found a newer build's: never written again this run, whatever
                // is asked after, one asked before the app knew included.
                let mut newer: HashSet<PathBuf> = HashSet::new();
                let mut write_all = |unwritten: &mut Unwritten| {
                    for (file, (id, builtin, moves)) in std::mem::take(unwritten) {
                        if newer.contains(&file) {
                            let shown = file.display();
                            tracing::debug!(target: "settings", "{shown}: a newer Apertrust's; moves dropped");
                            continue;
                        }
                        let kept = kept.entry(file.clone()).or_default();
                        match write(&file, &id, builtin, &moves, kept) {
                            Ok(Wrote::Newer) => {
                                newer.insert(file);
                                if told.send(id).is_ok() {
                                    wake();
                                }
                            }
                            Ok(Wrote::Written | Wrote::Left) => {}
                            Err(e) => {
                                let shown = file.display();
                                tracing::warn!(target: "settings", "{shown}: not written, kept for the next write: {e}");
                                unwritten.insert(file, (id, builtin, moves));
                            }
                        }
                    }
                };
                for ask in asked {
                    match ask {
                        Ask::Write {
                            file,
                            id,
                            builtin,
                            moves,
                        } => {
                            let entry = unwritten.entry(file).or_insert((id, builtin, Vec::new()));
                            entry.2.extend(moves);
                            write_all(&mut unwritten);
                        }
                        Ask::Retry => write_all(&mut unwritten),
                        #[cfg(test)]
                        Ask::Done(done) => {
                            write_all(&mut unwritten);
                            let _ = done.send(());
                        }
                    }
                }
                write_all(&mut unwritten);
            });
        match thread {
            Ok(thread) => Some(Writer {
                asks: Some(asks),
                thread: Some(thread),
            }),
            Err(e) => {
                let shown = folder.display();
                tracing::warn!(target: "settings", "{shown}: workspaces never written: no thread: {e}");
                None
            }
        }
    }

    fn send(&self, ask: Ask) -> bool {
        (self.asks.as_ref()).is_some_and(|asks| asks.send(ask).is_ok())
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

/// The bundled file of the built-in workspace `id`, as a table, with no `toolbar`: only a
/// Toolbar change writes one, so a file a tab move wrote keeps following each build's
/// default list (decision 92).
fn bundled(id: &str) -> Option<Table> {
    let (_, text) = BUILT_IN.iter().find(|(built, _)| *built == id)?;
    let mut table: Table = text.parse().expect("a bundled workspace reads");
    table.remove("toolbar");
    Some(table)
}

/// What the writer keeps of a workspace's file between writes ([`write()`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Kept {
    /// What its last write left, for a file damaged since.
    pub last: Option<Table>,
    /// Whether a write set the file aside as damaged and nothing has taken its place
    /// since, a write of its own or a file put there, so its file missing is no person's
    /// taking it away.
    pub set_aside: bool,
}

/// What a write did ([`write()`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wrote {
    /// The file holds the moves.
    Written,
    /// Nothing written, on purpose, with a warning: a person's own that is gone.
    Left,
    /// Nothing written: the file is a newer build's, which this build never overwrites.
    Newer,
}

/// The files whose next write fails, once each, as a full disk would make it.
#[cfg(test)]
pub static FAIL: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

/// Makes the next write of `file` fail, once, on whatever thread writes it.
#[cfg(test)]
pub fn fail_next_write(file: &Path) {
    FAIL.lock().unwrap().push(file.to_owned());
}

/// Whether a test asked the write of `file` to fail, taking the ask.
#[cfg(test)]
fn fails(file: &Path) -> bool {
    let mut fail = FAIL.lock().unwrap();
    let at = fail.iter().position(|failing| failing == file);
    at.map(|at| fail.remove(at)).is_some()
}

/// Applies `moves` to the workspace `id`'s file, `file`, as it is on disk now, and writes it
/// whole through the safe write, built in or not as `builtin` says; `kept` is what the
/// writer keeps of it between writes, brought up to date. Returns what it did. It writes
/// nothing, with a warning, for a newer build's file and a person's own that is gone. A
/// built-in's file missing, never written or deleted by hand while the app runs, is
/// written from its bundled file. An error where it could not read or write the file, it
/// lists no tab, or a person's own was set aside as damaged with nothing written of it
/// yet, its moves kept for the next write, a file it set aside then written from what
/// was set in its place, or from a valid file a person puts back.
pub fn write(
    file: &Path,
    id: &str,
    builtin: bool,
    moves: &[Move],
    kept: &mut Kept,
) -> io::Result<Wrote> {
    let shown = file.display();
    let bundled = || builtin.then(|| bundled(id)).flatten();
    // What a file this writer set aside as damaged gives way to: what the last write left,
    // or a built-in's bundled file.
    let instead = |kept: &Kept| kept.last.clone().or_else(bundled);
    let read = match std::fs::read(file) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };
    let text = read.map(|bytes| String::from_utf8(bytes).map_err(|_| "not UTF-8".to_owned()));
    let table = match text.map(|text| text.and_then(|text| table_of(&text))) {
        None if kept.set_aside => instead(kept),
        None if builtin => bundled(),
        None => {
            tracing::warn!(target: "settings", "{shown}: gone since read; not written again");
            return Ok(Wrote::Left);
        }
        // A file in its place, whether this build writes it or not: one gone after it is
        // a person's taking it away.
        Some(Ok(table)) => {
            kept.set_aside = false;
            Some(table)
        }
        Some(Err(why)) => {
            let aside = aside(file);
            std::fs::rename(file, &aside)?;
            kept.set_aside = true;
            let aside = aside.display();
            tracing::warn!(target: "settings", "{shown}: damaged since read, set aside as {aside}: {why}");
            instead(kept)
        }
    };
    // A person's own set aside with nothing written of it yet: its moves kept, for a valid
    // file a person puts back.
    let Some(mut table) = table else {
        return Err(io::Error::other(
            "damaged, nothing of it to write in its place",
        ));
    };
    let version = table.get("version").and_then(toml::Value::as_integer);
    if version.is_some_and(|version| version > VERSION) {
        tracing::warn!(target: "settings", "{shown}: a newer Apertrust's; never written");
        return Ok(Wrote::Newer);
    }
    let Ok((layout, _)) = Layout::from_table(&table) else {
        // Perhaps for a moment, as a person edits it: its moves kept to try again.
        return Err(io::Error::other("it lists no tab"));
    };
    let known = known();
    let defaults = Layout::defaults(if builtin { id } else { DEFAULT }).expect("built in");
    let (shown_now, _) = super::resolve(&layout, &[&defaults], &known);
    let mut layout = layout.settled(&shown_now, &known);
    for change in moves {
        layout.apply(change, &known);
    }
    layout.write_into(&mut table);
    // One list item a line, so a person reads and edits the lists by hand.
    let text = toml::to_string_pretty(&table).map_err(io::Error::other)?;
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder)?;
    }
    #[cfg(test)]
    if fails(file) {
        return Err(io::Error::other("a write failed, as a test asked"));
    }
    apertrust_base::fs::write(file, text.as_bytes())?;
    tracing::debug!(target: "settings", "{shown}: written");
    *kept = Kept {
        last: Some(table),
        set_aside: false,
    };
    Ok(Wrote::Written)
}
