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

//! The disk cache (decision 76; `docs/cache.md`), behind the `cache` feature, with no
//! `unsafe`: what Apertrust can compute again, kept in one folder split by kind.
//!
//! - [`KINDS`], each [`Kind`] a folder of its own with its [`Policy`]: limited kinds count
//!   together against one [`Limit`], kept ones are never removed by size.
//! - [`Cache`], the handle on a folder its caller chose. A limited entry is one file per
//!   slot, named by the slot's hash in 256 subfolders, holding its version; a write
//!   replaces the slot's file whole through the safe write ([`crate::fs`]), so a stale
//!   version goes at once; a read returns the bytes only for the version asked. Last use is
//!   the file's modified time, set as it is written and as it is read, at most once a day.
//! - [`Cache::walk`] gives each kind's [`Usage`] and writes nothing; [`Cache::sizes`] is
//!   the last walk's with the handle's own writes and removals since; [`Cache::tidy`]
//!   removes the temporaries a crash left.
//! - [`Cache::evict`] removes the oldest limited entries to 90% of the limit once they are
//!   past it, and [`Cache::clear`] a limited kind's entries, both cancellable.
//! - An item that cannot be read or removed is passed over, and a removal counted as
//!   failed: only a kind's own folder that cannot be listed is an error.
//! - [`UnusedAfter`], the [`Choice`] of the engine's setting `cache.unused_after`; the
//!   engine turns `cache.limited` and `cache.limit` into a [`Limit`].
//!
//! No index file and no lock for limited kinds:
//!
//! - two processes writing one slot each rename a whole file, the last rename winning;
//! - a removal that finds the file gone is ignored, and a read whose file vanished finds
//!   nothing;
//! - eviction may remove a slot another process rewrote between its walk and its removal,
//!   which costs only that entry made again;
//! - on Windows, a reader's open file may hold another process's rename back for a moment,
//!   which the safe write's retry covers ([`crate::fs::Retry`]).
//!
//! A kept kind is only its folder here.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use crate::settings::Choice;

/// Whether a kind's entries count against the limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Policy {
    /// Counted together against one limit, the oldest last use removed first.
    Limited,
    /// Never removed by size: only by the person, or by its owner's rule.
    Kept,
}

/// A kind of entry, a folder of its own in the cache: one of [`KINDS`], the only ones
/// there are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Kind {
    /// Its place in [`KINDS`].
    index: usize,
    id: &'static str,
    folder: &'static str,
    policy: Policy,
}

impl Kind {
    /// Its name, as `apertrust-cli cache clear` takes it.
    pub fn id(self) -> &'static str {
        self.id
    }

    /// Its folder in the cache folder.
    pub fn folder(self) -> &'static str {
        self.folder
    }

    pub fn policy(self) -> Policy {
        self.policy
    }
}

/// The library's thumbnails: one per photo, virtual copy and size.
pub const THUMBNAILS: Kind = Kind {
    index: 0,
    id: "thumbnails",
    folder: "thumbnails",
    policy: Policy::Limited,
};

/// The engine's wgpu pipeline cache, one per adapter, Vulkan only, if measured worthwhile.
pub const PIPELINES: Kind = Kind {
    index: 1,
    id: "pipelines",
    folder: "pipelines",
    policy: Policy::Limited,
};

/// The libraries' caches, one folder per library id.
pub const LIBRARIES: Kind = Kind {
    index: 2,
    id: "libraries",
    folder: "libraries",
    policy: Policy::Kept,
};

/// Local copies of raws, one per fingerprint, for offline drives and server roots.
pub const COPIES: Kind = Kind {
    index: 3,
    id: "copies",
    folder: "copies",
    policy: Policy::Kept,
};

/// Every kind, a closed list.
pub const KINDS: [Kind; 4] = [THUMBNAILS, PIPELINES, LIBRARIES, COPIES];

/// How long a read leaves an entry's last use alone: it is set at most once a day.
pub const TOUCH_AFTER: Duration = Duration::from_secs(24 * 60 * 60);

/// What eviction leaves of the limit, in tenths: past the limit, it removes down to 90%,
/// so it does not run again at the next write.
const EVICT_TO_TENTHS: u64 = 9;

/// What starts every limited entry's file, its format's mark: a file without it is no
/// entry of this build's and reads as none.
const MAGIC: [u8; 4] = *b"APC1";

/// How much the limited kinds may hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limit {
    /// Nothing is removed by size: `cache.limited` off.
    Unlimited,
    /// At most this many bytes: `cache.limit`.
    Bytes(u64),
}

/// How long a library's cache may go unopened on this machine before the Storage page
/// marks it and offers to remove it, or Never: `cache.unused_after`. Nothing is removed by
/// age alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnusedAfter {
    Days30,
    Days90,
    Days180,
    Days365,
    Never,
}

/// Kept by these ids in `settings.toml`, listed in this order.
impl Choice for UnusedAfter {
    const OPTIONS: &'static [(UnusedAfter, &'static str)] = &[
        (UnusedAfter::Days30, "days_30"),
        (UnusedAfter::Days90, "days_90"),
        (UnusedAfter::Days180, "days_180"),
        (UnusedAfter::Days365, "days_365"),
        (UnusedAfter::Never, "never"),
    ];
}

/// A kind's entries and their bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    pub entries: u64,
    pub bytes: u64,
}

impl Usage {
    /// Takes away what was removed, never below nothing.
    fn subtract(&mut self, removed: Usage) {
        self.entries = self.entries.saturating_sub(removed.entries);
        self.bytes = self.bytes.saturating_sub(removed.bytes);
    }
}

/// Each kind's [`Usage`], in [`KINDS`]' order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Sizes {
    usage: [Usage; KINDS.len()],
}

impl Sizes {
    /// `kind`'s usage.
    pub fn of(&self, kind: Kind) -> Usage {
        self.usage[kind.index]
    }

    /// The limited kinds' together, what the limit is measured against.
    pub fn limited(&self) -> Usage {
        limited_kinds().fold(Usage::default(), |sum, kind| {
            let usage = self.of(kind);
            Usage {
                entries: sum.entries + usage.entries,
                bytes: sum.bytes + usage.bytes,
            }
        })
    }

    fn of_mut(&mut self, kind: Kind) -> &mut Usage {
        &mut self.usage[kind.index]
    }
}

/// What [`Cache::evict`] or [`Cache::clear`] removed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Removed {
    pub entries: u64,
    pub bytes: u64,
    /// Entries that could not be removed, passed over and still counted.
    pub failed: u64,
    /// Whether it stopped at a cancel, before it was done.
    pub cancelled: bool,
}

/// The cache in one folder, which the caller chose; shared between threads.
#[derive(Debug)]
pub struct Cache {
    folder: PathBuf,
    /// The last walk's sizes with this handle's writes and removals since; `None` before
    /// the first walk.
    sizes: Mutex<Option<Sizes>>,
}

/// A limited entry found by a walk.
struct Entry {
    path: PathBuf,
    bytes: u64,
    used: SystemTime,
    kind: Kind,
}

impl Cache {
    /// The cache in `folder`. Nothing is read or made on disk: a missing folder is an empty
    /// cache, and the first write makes the folders it needs.
    pub fn open(folder: impl Into<PathBuf>) -> Self {
        Self {
            folder: folder.into(),
            sizes: Mutex::new(None),
        }
    }

    /// The cache in `folder`, as a program that writes it opens it: the folder created
    /// where it is missing, then a file of this process's own written in it and removed,
    /// since its permissions alone cannot tell, for a folder another user owns, a
    /// read-only volume or an access list. That is the cache's first write: an error here
    /// is the folder that cannot be created or written, and its caller runs with no disk
    /// cache (decision 76).
    pub fn create(folder: impl Into<PathBuf>) -> io::Result<Self> {
        let cache = Self::open(folder);
        fs::create_dir_all(&cache.folder)?;
        let probe = cache
            .folder
            .join(format!(".apertrust-write-check-{}.tmp", std::process::id()));
        let written = fs::write(&probe, b"");
        let _ = fs::remove_file(&probe);
        written.map(|()| cache)
    }

    /// The cache folder.
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// `kind`'s folder, where its owner keeps a kept kind's entries.
    pub fn kind_folder(&self, kind: Kind) -> PathBuf {
        self.folder.join(kind.folder)
    }

    /// Writes the entry for `slot` of the limited `kind`: `bytes` at `version`, replacing
    /// whatever the slot held, any version, whole. Its last use is now.
    pub fn write(&self, kind: Kind, slot: &[u8], version: &[u8], bytes: &[u8]) -> io::Result<()> {
        limited(kind)?;
        let length = u32::try_from(version.len())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "the version is too long"))?;
        let mut file = Vec::with_capacity(MAGIC.len() + 4 + version.len() + bytes.len());
        file.extend_from_slice(&MAGIC);
        file.extend_from_slice(&length.to_le_bytes());
        file.extend_from_slice(version);
        file.extend_from_slice(bytes);

        let path = self.entry_path(kind, slot);
        fs::create_dir_all(path.parent().expect("an entry's file is in its subfolder"))?;
        let before = fs::symlink_metadata(&path)
            .ok()
            .filter(|meta| meta.is_file())
            .map(|meta| meta.len());
        if matches!(
            crate::fs::write(&path, &file)?,
            crate::fs::Written::Unchanged
        ) {
            // The same bytes were not written again, so its time is set by hand.
            touch(&path, SystemTime::now());
        }
        self.count(kind, |usage| {
            match before {
                Some(old) => usage.bytes = usage.bytes.saturating_sub(old),
                None => usage.entries += 1,
            }
            usage.bytes += file.len() as u64;
        });
        Ok(())
    }

    /// The bytes of `slot`'s entry in the limited `kind`, if it holds `version`; `None` if
    /// it holds another, or none is there. A read sets its last use to now when that was
    /// more than [`TOUCH_AFTER`] ago.
    pub fn read(&self, kind: Kind, slot: &[u8], version: &[u8]) -> io::Result<Option<Vec<u8>>> {
        self.read_at(kind, slot, version, SystemTime::now())
    }

    /// [`Cache::read`] as at `now`.
    fn read_at(
        &self,
        kind: Kind,
        slot: &[u8],
        version: &[u8],
        now: SystemTime,
    ) -> io::Result<Option<Vec<u8>>> {
        limited(kind)?;
        let path = self.entry_path(kind, slot);
        let mut file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let used = file.metadata()?.modified().ok();
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;
        drop(file);
        let Some(bytes) = payload(&contents, version) else {
            return Ok(None);
        };
        let stale = used
            .and_then(|used| now.duration_since(used).ok())
            .is_some_and(|age| age > TOUCH_AFTER);
        if stale {
            touch(&path, now);
        }
        Ok(Some(bytes.to_vec()))
    }

    /// Walks every kind, writing nothing: each limited kind's entries, one listing per
    /// subfolder; each kept kind's entries, the items in its folder, and every byte under
    /// them. An item that cannot be read is passed over; a kind's folder that cannot be
    /// listed is the error. The handle keeps the result, which [`Cache::sizes`] then keeps
    /// up to date with its own writes and removals.
    pub fn walk(&self) -> io::Result<Sizes> {
        let mut sizes = Sizes::default();
        for kind in KINDS {
            *sizes.of_mut(kind) = match kind.policy {
                Policy::Limited => {
                    self.scan(kind)?
                        .iter()
                        .fold(Usage::default(), |usage, entry| Usage {
                            entries: usage.entries + 1,
                            bytes: usage.bytes + entry.bytes,
                        })
                }
                Policy::Kept => kept_usage(&self.kind_folder(kind))?,
            };
        }
        *self.lock() = Some(sizes);
        Ok(sizes)
    }

    /// The sizes as the last walk found them, with this handle's own writes and removals
    /// since; another process's are counted at the next walk. `None` before the first walk.
    pub fn sizes(&self) -> Option<Sizes> {
        *self.lock()
    }

    /// Removes the temporaries more than a day old that writes left in the limited kinds'
    /// subfolders, by name, as the session does as it starts; how many it removed. One that
    /// cannot be removed is left for the next time.
    pub fn tidy(&self) -> io::Result<u64> {
        let now = SystemTime::now();
        let mut removed = 0;
        for kind in limited_kinds() {
            for subfolder in self.subfolders(kind)? {
                let Ok(files) = fs::read_dir(&subfolder) else {
                    continue;
                };
                for file in files.flatten() {
                    let name = file.file_name();
                    let Some(name) = name.to_str() else {
                        continue;
                    };
                    if crate::fs::remove_if_stale(&subfolder, name, now).unwrap_or(false) {
                        removed += 1;
                    }
                }
            }
        }
        Ok(removed)
    }

    /// Walks the limited kinds and, if they hold more than `limit`, removes their entries,
    /// oldest last use first, across kinds, until they hold at most 90% of it; with
    /// [`Limit::Unlimited`], nothing. An entry that cannot be removed is passed over and
    /// counted as failed, its bytes still held. `cancel` is asked before each removal; once
    /// it answers true, eviction stops.
    pub fn evict(&self, limit: Limit, cancel: &(dyn Fn() -> bool + Sync)) -> io::Result<Removed> {
        let Limit::Bytes(limit) = limit else {
            return Ok(Removed::default());
        };
        let mut entries = Vec::new();
        for kind in limited_kinds() {
            entries.extend(self.scan(kind)?);
        }
        let mut held: u64 = entries.iter().map(|entry| entry.bytes).sum();
        if held <= limit {
            return Ok(Removed::default());
        }
        // Equal times in the paths' order, so that eviction's order is always the same.
        entries.sort_by(|a, b| a.used.cmp(&b.used).then_with(|| a.path.cmp(&b.path)));

        let target = (u128::from(limit) * u128::from(EVICT_TO_TENTHS) / 10) as u64;
        let mut removed = Removed::default();
        let mut taken = Sizes::default();
        for entry in &entries {
            if held <= target {
                break;
            }
            if cancel() {
                removed.cancelled = true;
                break;
            }
            match remove(&entry.path) {
                Ok(true) => {
                    held -= entry.bytes;
                    removed.entries += 1;
                    removed.bytes += entry.bytes;
                    let usage = taken.of_mut(entry.kind);
                    usage.entries += 1;
                    usage.bytes += entry.bytes;
                }
                // Another process removed it.
                Ok(false) => held -= entry.bytes,
                Err(_) => removed.failed += 1,
            }
        }
        for kind in limited_kinds() {
            let usage = taken.of(kind);
            self.count(kind, |sizes| sizes.subtract(usage));
        }
        Ok(removed)
    }

    /// Removes the limited `kind`'s entries listed as it starts. An entry that cannot be
    /// removed is passed over and counted as failed. `progress` is told the entries done and
    /// their total, first with none done, then after each; `cancel` is asked before each
    /// removal, and once it answers true, clearing stops. A kept kind is refused
    /// (`Unsupported`): a library's cache waits for its lock, a local copy for its own rule.
    pub fn clear(
        &self,
        kind: Kind,
        progress: &(dyn Fn(u64, u64) + Sync),
        cancel: &(dyn Fn() -> bool + Sync),
    ) -> io::Result<Removed> {
        if kind.policy == Policy::Kept {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("the cache's {} are not cleared yet", kind.id),
            ));
        }
        let entries = self.scan(kind)?;
        let total = entries.len() as u64;
        progress(0, total);
        let mut removed = Removed::default();
        for (done, entry) in (1..).zip(&entries) {
            if cancel() {
                removed.cancelled = true;
                break;
            }
            match remove(&entry.path) {
                Ok(true) => {
                    removed.entries += 1;
                    removed.bytes += entry.bytes;
                }
                Ok(false) => {}
                Err(_) => removed.failed += 1,
            }
            progress(done, total);
        }
        self.count(kind, |usage| {
            usage.subtract(Usage {
                entries: removed.entries,
                bytes: removed.bytes,
            })
        });
        Ok(removed)
    }

    /// The file of `slot`'s entry in `kind`: `<kind>/<first byte>/<name>` ([`name`]).
    fn entry_path(&self, kind: Kind, slot: &[u8]) -> PathBuf {
        let name = name(slot);
        self.kind_folder(kind).join(&name[..2]).join(name)
    }

    /// The limited `kind`'s subfolders. Only its folder's own listing is an error; an item
    /// that cannot be read is passed over.
    fn subfolders(&self, kind: Kind) -> io::Result<Vec<PathBuf>> {
        let Some(items) = listing(&self.kind_folder(kind))? else {
            return Ok(Vec::new());
        };
        Ok(items
            .flatten()
            .filter(|item| item.file_type().is_ok_and(|kind| kind.is_dir()))
            .map(|item| item.path())
            .collect())
    }

    /// The limited `kind`'s entries, one listing per subfolder, temporaries left out.
    /// A subfolder or a file that cannot be read, or has gone meanwhile, is passed over.
    fn scan(&self, kind: Kind) -> io::Result<Vec<Entry>> {
        let mut entries = Vec::new();
        for subfolder in self.subfolders(kind)? {
            let Ok(files) = fs::read_dir(&subfolder) else {
                continue;
            };
            for file in files.flatten() {
                let name = file.file_name();
                if name.to_str().is_some_and(crate::fs::is_temporary_name) {
                    continue;
                }
                let Ok(meta) = file.metadata() else {
                    continue;
                };
                if meta.is_file() {
                    entries.push(Entry {
                        path: file.path(),
                        bytes: meta.len(),
                        used: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                        kind,
                    });
                }
            }
        }
        Ok(entries)
    }

    /// Changes `kind`'s usage with `change`, once a walk has found it.
    fn count(&self, kind: Kind, change: impl FnOnce(&mut Usage)) {
        if let Some(sizes) = self.lock().as_mut() {
            change(sizes.of_mut(kind));
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<Sizes>> {
        // The sizes stay whole whatever panicked while they were held.
        self.sizes.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The limited kinds, in [`KINDS`]' order.
fn limited_kinds() -> impl Iterator<Item = Kind> {
    KINDS
        .into_iter()
        .filter(|kind| kind.policy == Policy::Limited)
}

/// An error for a kept `kind`, whose entries are its owner's.
fn limited(kind: Kind) -> io::Result<()> {
    match kind.policy {
        Policy::Limited => Ok(()),
        Policy::Kept => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("the cache's {} have no entries of the cache's own", kind.id),
        )),
    }
}

/// The 128-bit FNV-1a hash of `bytes`, a slot's: stable across builds and platforms, so
/// every program finds an entry under the same name.
fn hash(bytes: &[u8]) -> u128 {
    const OFFSET: u128 = 0x6c62272e07bb014262b821756295c58d;
    const PRIME: u128 = 0x0000000001000000000000000000013b;
    bytes.iter().fold(OFFSET, |hash, byte| {
        (hash ^ u128::from(*byte)).wrapping_mul(PRIME)
    })
}

/// The name of `slot`'s file: its [`hash`] in hex, least significant byte first, the byte
/// every byte of the slot changes, so that slots alike in all but their last bytes still
/// spread over the 256 subfolders.
fn name(slot: &[u8]) -> String {
    hash(slot)
        .to_le_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The bytes an entry's file holds after its mark and its version, if that is `version`.
fn payload<'a>(file: &'a [u8], version: &[u8]) -> Option<&'a [u8]> {
    let rest = file.strip_prefix(&MAGIC)?;
    let (length, rest) = rest.split_first_chunk::<4>()?;
    let length = usize::try_from(u32::from_le_bytes(*length)).ok()?;
    if rest.len() < length {
        return None;
    }
    let (found, bytes) = rest.split_at(length);
    (found == version).then_some(bytes)
}

/// Sets the file's modified time, its last use, to `now`. One that cannot be set, gone or
/// in a folder that cannot be written, keeps its time and is evicted sooner.
fn touch(path: &Path, now: SystemTime) {
    if let Ok(file) = File::options().write(true).open(path) {
        let _ = file.set_modified(now);
    }
}

/// Removes an entry's file: true if this removal took it, false if it had gone.
fn remove(path: &Path) -> io::Result<bool> {
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

/// `folder`'s listing, `None` if it is missing.
fn listing(folder: &Path) -> io::Result<Option<fs::ReadDir>> {
    match fs::read_dir(folder) {
        Ok(listing) => Ok(Some(listing)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// A kept kind's usage: the items in its folder, and every byte of the files under them,
/// links not followed. Only the folder's own listing is an error; an item that cannot be
/// read is passed over.
fn kept_usage(folder: &Path) -> io::Result<Usage> {
    let mut usage = Usage::default();
    let Some(items) = listing(folder)? else {
        return Ok(usage);
    };
    for item in items.flatten() {
        usage.entries += 1;
        usage.bytes += bytes_under(&item.path());
    }
    Ok(usage)
}

/// The bytes of the file at `path`, or of every file under the folder there; what cannot
/// be read counts nothing.
fn bytes_under(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return 0;
    };
    if meta.is_file() {
        return meta.len();
    }
    if !meta.is_dir() {
        return 0;
    }
    let Ok(items) = fs::read_dir(path) else {
        return 0;
    };
    items.flatten().map(|item| bytes_under(&item.path())).sum()
}

#[cfg(test)]
mod tests;
