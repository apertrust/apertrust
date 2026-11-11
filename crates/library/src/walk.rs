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

//! The walk of a folder's whole tree (`docs/library.md`, The tree's walk): every folder
//! under the root, breadth first, each read for its directory entries alone, its raws
//! counted by the decoder's extensions and its subfolders named, nothing decoded and no
//! file measured. What is hidden, a folder or a file is the listing's own rule
//! ([`crate::listing`]): a link to a folder is named but never walked into, so a loop of
//! them cannot hold it, and on Unix nor is a folder on another device. A folder that
//! cannot be read is recorded as such, and the walk goes on.
//!
//! The folders are read on a thread of their own, as the listing's are, so a folder on a
//! share that hangs holds that thread alone, never the caller, which batches what has come
//! and sends it on time even while the thread waits on the system.

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use crate::listing::{Kind, browser_order, is_raw, kind};

/// One folder of the tree as the walk found it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkedFolder {
    pub folder: PathBuf,
    /// How many of its files the decoder reads, by their extensions; its subfolders' not
    /// counted.
    pub raws: usize,
    /// Its subfolders, hidden ones left out, a link to a folder named too, as the system's
    /// file browser sorts them.
    pub subfolders: Vec<String>,
    /// Why it could not be read, in plain words; its counts empty then.
    pub unreadable: Option<String>,
}

/// How many folders a batch holds at most.
pub const BATCH: usize = 64;

/// How long a folder found waits for others to share its batch at most, so a slow share
/// still shows its counts as they come, and a folder that hangs never holds back those
/// found before it.
pub const BATCH_WAIT: Duration = Duration::from_millis(250);

/// How often the caller asks its cancel while it waits on the reading thread.
const CANCEL_POLL: Duration = Duration::from_millis(20);

/// How many folders the reading thread finds ahead of the caller before it waits.
const AHEAD: usize = 4 * BATCH;

/// Walks the tree under `root`, `root` first, breadth first, on a thread of its own,
/// handing `batch` what it found since the last batch, at most [`BATCH`] folders, each
/// folder waiting at most [`BATCH_WAIT`] for others, the last batch saying it is the last.
/// `cancel` is asked every 20 ms while the thread reads; once it answers true, or `batch`
/// returns false, nothing more is handed, the thread is told to stop, and it returns
/// false without waiting for it.
pub fn walk(
    root: &Path,
    batch: &mut dyn FnMut(Vec<WalkedFolder>, bool) -> bool,
    cancel: &(dyn Fn() -> bool + Sync),
) -> bool {
    let stop = Arc::new(AtomicBool::new(false));
    let (found, folders) = mpsc::sync_channel(AHEAD);
    let reading = {
        let (root, stop) = (root.to_owned(), Arc::clone(&stop));
        thread::Builder::new()
            .name("tree walk".to_owned())
            .spawn(move || {
                let stopped = || stop.load(Ordering::Relaxed);
                let walked = walk_on(
                    &root,
                    &mut |folder| found.send(Some(folder)).is_ok(),
                    &stopped,
                );
                if walked {
                    let _ = found.send(None);
                }
            })
    };
    if let Err(error) = reading {
        tracing::warn!("{}: the tree's walk cannot start: {error}", root.display());
        return false;
    }
    // Dropped, as the walk ends whichever way, it tells the thread to stop.
    let _stop = Stop(stop);
    let mut pending = Vec::new();
    let mut since = None::<Instant>;
    loop {
        if cancel() {
            return false;
        }
        let done = match folders.recv_timeout(CANCEL_POLL) {
            Ok(Some(folder)) => {
                since.get_or_insert_with(Instant::now);
                pending.push(folder);
                false
            }
            Ok(None) => true,
            Err(RecvTimeoutError::Timeout) => false,
            // The thread ended without saying so: it stopped, or panicked.
            Err(RecvTimeoutError::Disconnected) => return false,
        };
        let due = since.is_some_and(|since| since.elapsed() >= BATCH_WAIT);
        if done || pending.len() >= BATCH || due {
            if cancel() || !batch(std::mem::take(&mut pending), done) {
                return false;
            }
            since = None;
        }
        if done {
            return true;
        }
    }
}

/// Tells the reading thread to stop as it is dropped.
struct Stop(Arc<AtomicBool>);

impl Drop for Stop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

/// The walk itself, on the calling thread: hands `found` each folder as it is read, root
/// first, breadth first; false once `stopped` answers true or `found` does false.
fn walk_on(
    root: &Path,
    found: &mut dyn FnMut(WalkedFolder) -> bool,
    stopped: &dyn Fn() -> bool,
) -> bool {
    let device = device(root);
    let mut queue = VecDeque::from([root.to_owned()]);
    while let Some(folder) = queue.pop_front() {
        if stopped() {
            return false;
        }
        let walked = match read(&folder, device, stopped) {
            Some(Ok(read)) => {
                queue.extend(read.walked.iter().map(|name| folder.join(name)));
                WalkedFolder {
                    folder,
                    raws: read.raws,
                    subfolders: read.subfolders,
                    unreadable: None,
                }
            }
            Some(Err(error)) => WalkedFolder {
                folder,
                raws: 0,
                subfolders: Vec::new(),
                unreadable: Some(error.to_string()),
            },
            None => return false,
        };
        if !found(walked) {
            return false;
        }
    }
    true
}

/// What a folder's entries hold.
struct Read {
    raws: usize,
    /// Its subfolders, in the file browser's order.
    subfolders: Vec<String>,
    /// Those of them walked into: not links, and on the root's device.
    walked: Vec<String>,
}

/// The raws counted and the subfolders named in `folder`'s entries, by the listing's rule
/// ([`kind`]); `None` once stopped.
fn read(
    folder: &Path,
    device: Option<u64>,
    stopped: &dyn Fn() -> bool,
) -> Option<std::io::Result<Read>> {
    #[cfg(test)]
    crate::listing::stall::wait(folder);
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(error) => return Some(Err(error)),
    };
    let mut read = Read {
        raws: 0,
        subfolders: Vec::new(),
        walked: Vec::new(),
    };
    for entry in entries {
        if stopped() {
            return None;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => return Some(Err(error)),
        };
        // A name that is not text is not listed, as the listing leaves it out.
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        match kind(&entry, &name) {
            Kind::Folder { link } => {
                if !link && same_device(&entry, device) {
                    read.walked.push(name.clone());
                }
                read.subfolders.push(name);
            }
            // A link to a raw counts by its name, as the listing lists it.
            Kind::File if is_raw(&name) => read.raws += 1,
            Kind::File | Kind::Hidden | Kind::Unreadable => {}
        }
    }
    read.subfolders.sort_unstable_by(|a, b| browser_order(a, b));
    read.walked.sort_unstable_by(|a, b| browser_order(a, b));
    Some(Ok(read))
}

/// The device `path` is on, where the system says so cheaply: on Unix.
#[cfg(unix)]
fn device(path: &Path) -> Option<u64> {
    use std::os::unix::fs::MetadataExt as _;
    fs::metadata(path).ok().map(|meta| meta.dev())
}

/// Elsewhere, none: a folder on another volume is walked as any other.
#[cfg(not(unix))]
fn device(_path: &Path) -> Option<u64> {
    None
}

/// Whether the folder `entry` names is on `device`, the root's: a mount point inside the
/// tree is named but not walked into. A folder whose device cannot be read is walked.
#[cfg(unix)]
fn same_device(entry: &fs::DirEntry, device: Option<u64>) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    match (device, entry.metadata()) {
        (Some(device), Ok(meta)) => meta.dev() == device,
        _ => true,
    }
}

/// Elsewhere every folder is walked.
#[cfg(not(unix))]
fn same_device(_entry: &fs::DirEntry, _device: Option<u64>) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use tempfile::TempDir;

    use super::*;
    use crate::listing::stall;

    /// A folder that hangs, as one on a share that does not answer, holds neither the
    /// folders found before it, whose batch comes on time, nor the caller, which returns
    /// once cancelled without waiting for the reading thread.
    #[test]
    fn a_folder_that_hangs_holds_back_neither_what_came_before_nor_the_caller() {
        let dir = TempDir::new().unwrap();
        let root = dir.path().canonicalize().unwrap();
        for day in ["A", "B"] {
            fs::create_dir(root.join(day)).unwrap();
        }
        let hung = root.join("B");
        stall::hold(&hung);
        let heard = Mutex::new(Vec::new());
        let started = Instant::now();
        let walked = walk(
            &root,
            &mut |folders, done| {
                heard.lock().unwrap().push((folders, done));
                true
            },
            &|| !heard.lock().unwrap().is_empty(),
        );
        let took = started.elapsed();
        stall::release(&hung);
        assert!(!walked);
        assert!(took < BATCH_WAIT + Duration::from_secs(2), "{took:?}");
        let heard = heard.into_inner().unwrap();
        let (folders, done) = &heard[0];
        assert!(!done);
        let names: Vec<&Path> = folders.iter().map(|f| f.folder.as_path()).collect();
        assert_eq!(names, [root.as_path(), &root.join("A")]);
    }
}
