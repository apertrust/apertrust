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

//! Import's source (decision 85; `docs/library.md`, Import in folder mode): the card rule,
//! and the listing of a card's `DCIM` or of a folder for its raws.
//!
//! A card is a mounted volume with a `DCIM` folder at its top ([`card_at`]), found by
//! walking up from the source to its volume's mount point ([`mount_point`]): a card's
//! `DCIM` is listed whole, every folder under it, whichever folder of the card was chosen;
//! any other folder alone, or with its subfolders where they are asked for.
//!
//! The listing reads only. Hidden entries and names that are not text are left out, a Mac
//! package is a file and a link to a folder is never walked into, by the listing's own
//! rule ([`crate::listing`]); but unlike a folder's listing it removes no stale temporary,
//! so nothing is ever written on the source. Each raw is measured with one metadata call,
//! which also says whether it is on the disk; a cloud file not on the disk is listed by its
//! name and size alone. Sidecars and the other files are counted, not listed.
//!
//! The folders are read on a thread of their own, as the tree's walk reads them, so a card
//! or a share that hangs holds that thread alone, never the caller, which returns once
//! cancelled.

use std::collections::VecDeque;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::listing::{Kind, is_raw, kind};

/// The folder a camera card holds its photos in, at its top (DCF).
pub const DCIM: &str = "DCIM";

/// How often the caller asks its cancel while it waits on the reading thread.
const CANCEL_POLL: Duration = Duration::from_millis(20);

/// A camera card: a mounted volume with a [`DCIM`] folder at its top.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    /// The volume's mount point.
    pub mount: PathBuf,
    /// Its `DCIM` folder.
    pub dcim: PathBuf,
}

/// The card `path` is on, `mount` being the mount point of its volume: one where `mount`
/// holds a [`DCIM`] folder at its top, a link to one included, and `path` is `mount` or
/// inside it. The one rule that tells a card, for Import and for a card opened as a
/// folder.
pub fn card_at(path: &Path, mount: &Path) -> Option<Card> {
    if !path.starts_with(mount) {
        return None;
    }
    let dcim = mount.join(DCIM);
    dcim.is_dir().then(|| Card {
        mount: mount.to_owned(),
        dcim,
    })
}

/// The card `path` is on, if it is on one: [`card_at`] with its volume's
/// [`mount_point`]. A blocking call: it reads the metadata of `path` and its parents.
pub fn card(path: &Path) -> Option<Card> {
    card_at(path, &mount_point(path)?)
}

/// The mount point of the volume `path` is on: the last folder on its device, walking up
/// from it. None where its metadata cannot be read.
#[cfg(unix)]
pub fn mount_point(path: &Path) -> Option<PathBuf> {
    use std::os::unix::fs::MetadataExt as _;
    let device = fs::metadata(path).ok()?.dev();
    let mut mount = path;
    for parent in path.ancestors().skip(1) {
        match fs::metadata(parent) {
            Ok(meta) if meta.dev() == device => mount = parent,
            _ => break,
        }
    }
    Some(mount.to_owned())
}

/// The mount point of the volume `path` is on: its root, a drive's (`E:\`) or a share's
/// (`\\server\share\`). None for a path with no root.
#[cfg(windows)]
pub fn mount_point(path: &Path) -> Option<PathBuf> {
    use std::path::Component;
    let root: PathBuf = (path.components())
        .take_while(|part| matches!(part, Component::Prefix(_) | Component::RootDir))
        .collect();
    (!root.as_os_str().is_empty()).then_some(root)
}

/// Elsewhere no volume is told apart, so no folder is a card.
#[cfg(not(any(unix, windows)))]
pub fn mount_point(_path: &Path) -> Option<PathBuf> {
    None
}

/// A raw of the source, as its listing finds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFile {
    /// Its path relative to the folder listed, the card's `DCIM` or the folder.
    pub path: PathBuf,
    /// Its size in bytes.
    pub size: u64,
    /// Its modified time, `None` where the system gives none.
    #[serde(default, with = "apertrust_base::wire::system_time::option")]
    pub modified: Option<SystemTime>,
    /// Whether its data is on the disk ([`crate::listing::is_local`]): a cloud file not on
    /// the disk is listed by its name and size alone, and nothing reads it in the
    /// background (decision 53).
    pub local: bool,
}

/// What a source's listing left out of its photos: the other files, counted by kind, and
/// the folders it could not read, which it skipped.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeftOut {
    /// Files that are neither raws nor sidecars.
    pub non_raws: usize,
    /// Sidecars: `.apxmp`, `.apdata` and darktable's `.xmp`, which the copy carries with
    /// their raws.
    pub sidecars: usize,
    /// The folders under the one listed that could not be read, skipped, the rest listed.
    pub unreadable: Vec<Unreadable>,
}

/// A folder of the source that could not be read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unreadable {
    /// Its path relative to the folder listed.
    pub folder: PathBuf,
    /// Why, in plain words.
    pub reason: String,
}

/// A source listed whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    /// The folder listed: the card's `DCIM`, or the folder.
    pub folder: PathBuf,
    /// Its raws, in the order of their paths.
    pub files: Vec<SourceFile>,
    /// Every folder whose files were listed, `folder` first, as they were read.
    pub folders: Vec<PathBuf>,
    pub left_out: LeftOut,
}

/// Why a source could not be listed, in plain words.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{}: no absolute folder", path.display())]
    Relative { path: PathBuf },
    #[error("{} is not there", path.display())]
    Missing { path: PathBuf },
    #[error("{} is not a folder", path.display())]
    NotAFolder { path: PathBuf },
    #[error("cannot read {}: {source}", path.display())]
    Unreadable { path: PathBuf, source: io::Error },
    #[error("{} went away while it was read", path.display())]
    Gone { path: PathBuf },
    /// The reading thread ended without an answer.
    #[error("the listing of {} stopped without an answer", path.display())]
    Stopped { path: PathBuf },
}

/// Lists the source `source`, a folder: where it is on a card, by `mount` its volume's
/// mount point ([`mount_point`]), the card's `DCIM` whole; else the folder alone, or with
/// every folder under it where `subfolders`. On a thread of its own, waited for while
/// `cancel` is asked every 20 ms; `Ok(None)` once cancelled, returning without waiting for
/// a call into the source that has not returned.
pub fn list(
    source: &Path,
    subfolders: bool,
    mount: impl Fn(&Path) -> Option<PathBuf> + Send + 'static,
    cancel: &(dyn Fn() -> bool + Sync),
) -> Result<Option<Listed>, Error> {
    if !crate::ops::inside(source, source) {
        return Err(Error::Relative {
            path: source.to_owned(),
        });
    }
    let stop = Arc::new(AtomicBool::new(false));
    let (sent, answer) = mpsc::sync_channel(1);
    let reading = {
        let (source, stop) = (source.to_owned(), Arc::clone(&stop));
        thread::Builder::new()
            .name("import source".to_owned())
            .spawn(move || {
                let stopped = || stop.load(Ordering::Relaxed);
                let card = mount(&source).and_then(|mount| card_at(&source, &mount));
                let listed = match card {
                    Some(card) => list_on(&card.dcim, true, &stopped),
                    None => list_on(&source, subfolders, &stopped),
                };
                let _ = sent.send(listed);
            })
    };
    if let Err(error) = reading {
        tracing::warn!(
            "{}: the source's listing cannot start: {error}",
            source.display()
        );
        return Err(Error::Stopped {
            path: source.to_owned(),
        });
    }
    // Dropped, as the call returns whichever way, it tells the thread to stop.
    let _stop = Stop(stop);
    loop {
        if cancel() {
            return Ok(None);
        }
        match answer.recv_timeout(CANCEL_POLL) {
            Ok(listed) => return listed,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err(Error::Stopped {
                    path: source.to_owned(),
                });
            }
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

/// The listing itself, on the calling thread: `folder`, and breadth first every folder
/// under it where `subfolders`; `Ok(None)` once `stopped` answers true.
fn list_on(
    folder: &Path,
    subfolders: bool,
    stopped: &dyn Fn() -> bool,
) -> Result<Option<Listed>, Error> {
    match fs::metadata(folder) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => {
            return Err(Error::NotAFolder {
                path: folder.to_owned(),
            });
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(Error::Missing {
                path: folder.to_owned(),
            });
        }
        Err(source) => {
            return Err(Error::Unreadable {
                path: folder.to_owned(),
                source,
            });
        }
    }
    let mut listed = Listed {
        folder: folder.to_owned(),
        files: Vec::new(),
        folders: Vec::new(),
        left_out: LeftOut::default(),
    };
    let mut queue = VecDeque::from([folder.to_owned()]);
    while let Some(read) = queue.pop_front() {
        if stopped() {
            return Ok(None);
        }
        match list_folder(&read, &mut listed, subfolders, stopped) {
            Some(Ok(found)) => queue.extend(found),
            // The source itself gone, a card pulled out, ends the listing.
            Some(Err(_)) if !folder.is_dir() => {
                return Err(Error::Gone {
                    path: folder.to_owned(),
                });
            }
            Some(Err(source)) if read == folder => {
                return Err(Error::Unreadable {
                    path: folder.to_owned(),
                    source,
                });
            }
            Some(Err(error)) => {
                tracing::warn!("{}: skipped, cannot be read: {error}", read.display());
                listed.left_out.unreadable.push(Unreadable {
                    folder: relative(&read, folder),
                    reason: error.to_string(),
                });
            }
            None => return Ok(None),
        }
    }
    if !folder.is_dir() {
        return Err(Error::Gone {
            path: folder.to_owned(),
        });
    }
    listed.files.sort_unstable_by(|a, b| a.path.cmp(&b.path));
    Ok(Some(listed))
}

/// Lists the folder `read` into `listed`: its raws measured, its sidecars and other files
/// counted; the folders under it to read next, where `subfolders`. `None` once stopped.
fn list_folder(
    read: &Path,
    listed: &mut Listed,
    subfolders: bool,
    stopped: &dyn Fn() -> bool,
) -> Option<io::Result<Vec<PathBuf>>> {
    #[cfg(test)]
    crate::listing::stall::wait(read);
    let entries = match fs::read_dir(read) {
        Ok(entries) => entries,
        Err(error) => return Some(Err(error)),
    };
    listed.folders.push(read.to_owned());
    let mut found = Vec::new();
    for entry in entries {
        if stopped() {
            return None;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => return Some(Err(error)),
        };
        // A name that is not text is not listed, as the folder's listing leaves it out.
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        match kind(&entry, &name) {
            Kind::Folder { link: false } if subfolders => found.push(read.join(&name)),
            Kind::Folder { .. } | Kind::Hidden | Kind::Unreadable => {}
            Kind::File if is_raw(&name) => {
                let path = read.join(&name);
                // One that went or is no file between the listing and its size is left out.
                let Ok(meta) = fs::metadata(&path) else {
                    continue;
                };
                if !meta.is_file() {
                    continue;
                }
                let local = crate::listing::is_local(&meta);
                #[cfg(any(test, feature = "testing"))]
                let local = local && !crate::testing::evicted(&path);
                listed.files.push(SourceFile {
                    path: relative(&path, &listed.folder),
                    size: meta.len(),
                    modified: meta.modified().ok(),
                    local,
                });
            }
            Kind::File if sidecar(&name) => listed.left_out.sidecars += 1,
            Kind::File => listed.left_out.non_raws += 1,
        }
    }
    found.sort_unstable();
    Some(Ok(found))
}

/// Whether `name` is a sidecar the copy carries with its raw: an `.apxmp`, an `.apdata` or
/// darktable's `.xmp`.
fn sidecar(name: &str) -> bool {
    [
        crate::sidecar::SUFFIX,
        crate::copy::DATA_SUFFIX,
        crate::darktable::SUFFIX,
    ]
    .iter()
    .any(|suffix| name.ends_with(suffix))
}

/// `path` relative to `folder`, which holds it.
fn relative(path: &Path, folder: &Path) -> PathBuf {
    path.strip_prefix(folder).unwrap_or(path).to_owned()
}

#[cfg(test)]
mod tests;
