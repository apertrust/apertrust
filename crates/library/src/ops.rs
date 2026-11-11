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

//! The library's operations, layer 4 of decision 54: [`LibraryCommand`] and
//! [`LibraryEvent`], what clients send and hear through the session protocol, which the
//! engine's session serves (`Command::Library`, `Event::Library`), the long ones as jobs
//! of its control (decision 57). Plain data, so they can move into the protocol.
//!
//! Built so far: opening a folder ([`open`]), which reports the folder, its subfolders and
//! its photos, each with what its sidecar says, then their capture times, closing it,
//! walking the whole tree under the folder opened for each folder's raw count and
//! subfolders ([`walk`]), listing a subfolder's own subfolders for Browse's tree
//! ([`subfolders`]), and its photos' thumbnails
//! ([`thumbnail`]), the camera's embedded pictures until Apertrust renders its own. The
//! session runs [`open`] and [`thumbnail`] as jobs' bodies and hands each an observer,
//! which turns each [`LibraryEvent`] into its own event, and a cancel closure, the job's;
//! the library knows nothing of the engine or its events (`docs/events.md`, Lower crates
//! report through an observer).
//!
//! Every event names the open it is about by its number, the one the session gave the
//! command that opened the folder, or closed it: a client keeps the number of its latest
//! `Open` or `Close` and drops what an earlier one sends on its way.

use std::fmt;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

pub use apertrust_raw::DateTime;

use crate::index::{Index, READERS, Sidecar};
use crate::listing::Options;
pub use crate::listing::browser_order;
use crate::sidecar::Newer;
pub use crate::walk::WalkedFolder;

/// How many photos a [`LibraryEvent::Listed`] carries at most.
pub const CHUNK: usize = 256;

/// What the library opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A folder, in folder mode (decision 47): its photos, not its subfolders'.
    Folder(PathBuf),
}

/// What a client asks of the library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryCommand {
    /// Opens a folder in place of the one open, whose reading stops. Opening a folder
    /// opens no photo.
    Open(Source),
    /// Closes the folder open, stopping its reading; answered by
    /// [`LibraryEvent::Closed`].
    Close,
    /// Asks for thumbnails of the open folder's `photos`, each named by its file name, at
    /// most `size` pixels on their longer side, the first the most wanted; answered by a
    /// [`LibraryEvent::Thumbnail`] for each. A new open or a close drops those not yet sent.
    Thumbnails { photos: Vec<String>, size: u32 },
    /// Lists the subfolders of `folder`, one inside the tree of the folder opened, as Browse
    /// opens it: the tree's root is the first folder opened, kept while each later open lies
    /// inside it ([`inside`]). Answered by [`LibraryEvent::SubfoldersOf`]. A new open or a
    /// close drops it.
    Subfolders { folder: PathBuf },
    /// From which size a photo with no edit shows a render of its raw rather than the
    /// camera's embedded picture, the engine's setting `library.raw_thumbnails_from`, for
    /// the thumbnails and opening pictures from now on; answered by nothing.
    RawThumbnails(RawThumbnails),
}

/// What the library tells a client. `open` is the number of the command it answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryEvent {
    /// The folder was listed and its sidecars read: its name, the last part of its path,
    /// and the path as asked for.
    Opened {
        open: u64,
        name: String,
        folder: PathBuf,
    },
    /// The folder's subfolders, hidden ones left out, as the system's file browser sorts
    /// them; sent once, before the photos, empty where there is none.
    Subfolders { open: u64, names: Vec<String> },
    /// The subfolders of `folder`, as [`LibraryCommand::Subfolders`] asked, in the open
    /// numbered `open`: hidden ones left out, as the system's file browser sorts them,
    /// empty where there is none. `unreadable` says why the folder could not be listed, in
    /// plain words, its `names` empty then: a share that is down a moment is not a folder
    /// with no subfolders.
    SubfoldersOf {
        open: u64,
        folder: PathBuf,
        names: Vec<String>,
        unreadable: Option<String>,
    },
    /// More of the folder's photos, in name order across chunks, at most [`CHUNK`] each;
    /// the last says `done`, and an empty folder sends one, empty and done.
    Listed {
        open: u64,
        photos: Vec<Photo>,
        done: bool,
    },
    /// The folder could not be listed, in plain words: missing, no folder, unreadable, or
    /// gone while it was read. It ends the open, in place of [`Opened`].
    ///
    /// [`Opened`]: LibraryEvent::Opened
    Failed { open: u64, reason: String },
    /// When each photo was taken, sent once after the last [`Listed`] chunk: one time for
    /// each photo listed, in the listing's order, the camera's clock as the raw's EXIF
    /// holds it, `None` where it holds none or cannot be read. A client sorts by the
    /// files' modified times until it comes.
    ///
    /// [`Listed`]: LibraryEvent::Listed
    Taken {
        open: u64,
        times: Vec<Option<DateTime>>,
    },
    /// No folder is open: the answer to [`LibraryCommand::Close`], whose number it names.
    Closed { open: u64 },
    /// A photo's thumbnail at `size`, as [`LibraryCommand::Thumbnails`] asked, naming the
    /// open of its folder: the camera's embedded picture, upright, or `None` where the raw
    /// holds none that can be read.
    Thumbnail {
        open: u64,
        photo: String,
        size: u32,
        image: Option<Thumbnail>,
    },
    /// More folders of the tree under the folder opened, walked breadth first after the
    /// folder's own listing ([`walk`]): each with its raws counted and its subfolders
    /// named, or why it could not be read. `open` is the number of the open that made the
    /// folder the tree's root, which later opens inside the tree keep; the last batch says
    /// `done`. A new root or a close stops it.
    Walked {
        open: u64,
        folders: Vec<WalkedFolder>,
        done: bool,
    },
}

impl LibraryEvent {
    /// The number of the open or close it is about.
    pub fn open(&self) -> u64 {
        match *self {
            LibraryEvent::Opened { open, .. }
            | LibraryEvent::Subfolders { open, .. }
            | LibraryEvent::SubfoldersOf { open, .. }
            | LibraryEvent::Listed { open, .. }
            | LibraryEvent::Failed { open, .. }
            | LibraryEvent::Taken { open, .. }
            | LibraryEvent::Closed { open }
            | LibraryEvent::Thumbnail { open, .. }
            | LibraryEvent::Walked { open, .. } => open,
        }
    }
}

/// A thumbnail's picture: 8-bit RGBA, rows top to bottom, four bytes a pixel, upright, its
/// pixels shared so that the event's copies cost nothing.
#[derive(Clone, PartialEq, Eq)]
pub struct Thumbnail {
    pub width: u32,
    pub height: u32,
    pub pixels: Arc<[u8]>,
}

impl fmt::Debug for Thumbnail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The pixels are too many to print.
        write!(f, "Thumbnail({}×{})", self.width, self.height)
    }
}

/// A photo of the open folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Photo {
    /// The raw's file name.
    pub name: String,
    /// The raw's size in bytes.
    pub size: u64,
    /// The raw's modified time, `None` where the system gives none.
    pub modified: Option<SystemTime>,
    pub edit: EditState,
}

/// What a photo's sidecar says of its edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditState {
    /// No edit: no sidecar, or one with nothing in it.
    None,
    /// An edit this build can change.
    Edited,
    /// An edit a newer Apertrust wrote, which opens for viewing only, and why
    /// (decision 43).
    ViewOnly(Newer),
    /// The sidecar could not be read or is no `.apxmp`: the reason, in plain words.
    Unreadable(String),
}

/// How an [`open`] ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ended {
    /// Every photo was listed, the last chunk saying done, and their capture times sent.
    Listed,
    /// It was cancelled, and sent nothing more: no chunk said done, or no capture times
    /// came after the last.
    Cancelled,
    /// It sent [`LibraryEvent::Failed`].
    Failed,
}

/// Lists the subfolders of `folder`, inside the tree's root `root`, for the open numbered
/// `open`, telling `observer` once with [`LibraryEvent::SubfoldersOf`]: a folders-only
/// listing on the listing's own thread ([`listing::subfolders`]), by its rules for hidden
/// names, names that are not text and links, waited for while `cancel` is asked. A folder
/// that cannot be listed says why. A `folder` outside `root`, relative, or with a `..` in
/// it is refused with a warning and nothing sent, so a client lists only the tree of a
/// folder it opened. A blocking call, made by a job's body; once `cancel` answers true nothing is
/// sent.
///
/// [`listing::subfolders`]: crate::listing::subfolders
pub fn subfolders(
    open: u64,
    root: &Path,
    folder: &Path,
    observer: &(dyn Fn(LibraryEvent) + Send + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Ended {
    if !inside(root, folder) {
        tracing::warn!(
            "{}: not inside the folder opened, {}; not listed",
            folder.display(),
            root.display()
        );
        return Ended::Failed;
    }
    let (names, unreadable) = match crate::listing::subfolders(folder, cancel) {
        Ok(Some(names)) => (names, None),
        Ok(None) => return Ended::Cancelled,
        Err(error) => (Vec::new(), Some(error.to_string())),
    };
    if cancel() {
        return Ended::Cancelled;
    }
    observer(LibraryEvent::SubfoldersOf {
        open,
        folder: folder.to_owned(),
        names,
        unreadable,
    });
    Ended::Listed
}

/// Walks the whole tree under `root`, the folder the open numbered `open` made the tree's
/// root, telling `observer` what it finds as [`LibraryEvent::Walked`] batches
/// ([`walk::walk`](crate::walk::walk)): directory entries alone, breadth first, hidden
/// folders and links left out, a folder that cannot be read recorded as such. A relative
/// `root`, or one with a `..` in it, is refused with a warning and nothing sent. A
/// blocking call, made by a job's body; once `cancel` answers true nothing more is sent.
pub fn walk(
    open: u64,
    root: &Path,
    observer: &(dyn Fn(LibraryEvent) + Send + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Ended {
    if !inside(root, root) {
        tracing::warn!("{}: no absolute folder; not walked", root.display());
        return Ended::Failed;
    }
    let mut tell = |folders, done| {
        observer(LibraryEvent::Walked {
            open,
            folders,
            done,
        });
        true
    };
    if crate::walk::walk(root, &mut tell, cancel) {
        Ended::Listed
    } else {
        Ended::Cancelled
    }
}

/// Whether `folder` is `root` or a folder inside it, both absolute, with no `..` that could
/// lead out of it.
pub fn inside(root: &Path, folder: &Path) -> bool {
    let up = |path: &Path| {
        path.components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    };
    root.is_absolute()
        && folder.is_absolute()
        && !up(root)
        && !up(folder)
        && folder.starts_with(root)
}

/// Opens `source` for the command numbered `open`, telling `observer` what it finds, in
/// order: [`Opened`](LibraryEvent::Opened), [`Subfolders`](LibraryEvent::Subfolders),
/// the photos in [`Listed`](LibraryEvent::Listed) chunks and their capture times
/// ([`Taken`](LibraryEvent::Taken)), or [`Failed`](LibraryEvent::Failed) in their place.
/// A blocking call, made by a job's body: it lists the folder and reads its sidecars
/// ([`Index::build`]) before it sends anything, the listing telling a folder missing, a
/// file in its place or one that cannot be read, and reads the raws' EXIF once every
/// photo is sent, [`READERS`] at a time, so the photos show before their times are known.
/// `cancel` is asked between calls into the folder and before each event; once it answers
/// true, the call returns and nothing more is sent, without waiting for a call into the
/// folder that has not returned.
pub fn open(
    open: u64,
    source: &Source,
    observer: &(dyn Fn(LibraryEvent) + Send + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Ended {
    open_in_chunks(open, source, CHUNK, observer, cancel)
}

/// [`open`] with chunks of at most `chunk` photos.
fn open_in_chunks(
    open: u64,
    source: &Source,
    chunk: usize,
    observer: &(dyn Fn(LibraryEvent) + Send + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Ended {
    let Source::Folder(folder) = source;
    // Each event is sent only while the open still stands.
    let tell = |event| {
        if cancel() {
            return false;
        }
        observer(event);
        true
    };
    if cancel() {
        return Ended::Cancelled;
    }
    let index = match Index::build(folder, Options::default(), cancel) {
        Ok(index) if !index.cancelled => index,
        Ok(_) => return Ended::Cancelled,
        Err(error) => {
            let reason = error.to_string();
            return if tell(LibraryEvent::Failed { open, reason }) {
                Ended::Failed
            } else {
                Ended::Cancelled
            };
        }
    };
    let name = folder
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| folder.display().to_string());
    let opened = LibraryEvent::Opened {
        open,
        name,
        folder: folder.clone(),
    };
    let subfolders = LibraryEvent::Subfolders {
        open,
        names: index.subfolders,
    };
    if !tell(opened) || !tell(subfolders) {
        return Ended::Cancelled;
    }

    let chunk = chunk.max(1);
    let photos: Vec<Photo> = index.photos.into_iter().map(photo).collect();
    let names: Vec<String> = photos.iter().map(|photo| photo.name.clone()).collect();
    let chunks = photos.len().div_ceil(chunk).max(1);
    let mut photos = photos.into_iter();
    for sent in 1..=chunks {
        let listed = LibraryEvent::Listed {
            open,
            photos: photos.by_ref().take(chunk).collect(),
            done: sent == chunks,
        };
        if !tell(listed) {
            return Ended::Cancelled;
        }
    }
    let Some(times) = capture_times(folder, &names, cancel) else {
        return Ended::Cancelled;
    };
    if !tell(LibraryEvent::Taken { open, times }) {
        return Ended::Cancelled;
    }
    Ended::Listed
}

/// When each of `names`, raws of `folder`, was taken, from their EXIF and not their
/// pixels, read [`READERS`] at a time; `None` once `cancel` answers true. A raw that
/// cannot be read, or whose reading panics, has no time.
fn capture_times(
    folder: &Path,
    names: &[String],
    cancel: &(dyn Fn() -> bool + Sync),
) -> Option<Vec<Option<DateTime>>> {
    let next = AtomicUsize::new(0);
    let stopped = AtomicBool::new(false);
    let read: Vec<Vec<(usize, Option<DateTime>)>> = thread::scope(|scope| {
        let workers: Vec<_> = (0..READERS.min(names.len()))
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let at = next.fetch_add(1, Ordering::Relaxed);
                        let Some(name) = names.get(at) else {
                            break;
                        };
                        if cancel() {
                            stopped.store(true, Ordering::Relaxed);
                            break;
                        }
                        let path = folder.join(name);
                        let taken = panic::catch_unwind(AssertUnwindSafe(|| {
                            apertrust_raw::read_exif_file(&path).ok()?.taken
                        }))
                        .unwrap_or(None);
                        done.push((at, taken));
                    }
                    done
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap_or_else(|p| panic::resume_unwind(p)))
            .collect()
    });
    if stopped.load(Ordering::Relaxed) || cancel() {
        return None;
    }
    let mut times = vec![None; names.len()];
    for (at, taken) in read.into_iter().flatten() {
        times[at] = taken;
    }
    Some(times)
}

/// Reads the thumbnail of `photo`, a file of `folder`, at most `size` pixels on its longer
/// side, for the open numbered `open`, and tells `observer` as one
/// [`LibraryEvent::Thumbnail`], unless `cancel` answers true by then. A blocking call, made
/// by a job's body. Until Apertrust renders its own, a thumbnail is the camera's embedded
/// picture, shown as darktable shows it; a raw with none, or one that cannot be read, has
/// none, which is told too. A name that is no plain file name reads nothing.
pub fn thumbnail(
    open: u64,
    folder: &Path,
    photo: &str,
    size: u32,
    observer: &(dyn Fn(LibraryEvent) + Send + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) {
    if cancel() {
        return;
    }
    let plain = Path::new(photo).file_name() == Some(std::ffi::OsStr::new(photo));
    let image = plain
        .then(|| folder.join(photo))
        .and_then(|path| match apertrust_raw::read_preview_file(&path, size) {
            Ok(preview) => preview,
            Err(error) => {
                tracing::debug!(target: "raw", "no thumbnail of {photo}: {error}");
                None
            }
        })
        .map(|preview| Thumbnail {
            width: preview.width,
            height: preview.height,
            pixels: preview.pixels.into(),
        });
    if cancel() {
        return;
    }
    observer(LibraryEvent::Thumbnail {
        open,
        photo: photo.to_owned(),
        size,
        image,
    });
}

/// What a thumbnail is kept by in the disk cache's thumbnails (decision 76): its slot, what
/// it stands for, and the version of the picture it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbnailKey {
    /// The photo, its virtual copy and the size asked. Until the photo's fingerprint is
    /// built (the library's step 8), the photo is its path.
    pub slot: Vec<u8>,
    /// What the picture depends on: the embedded picture's reading
    /// ([`apertrust_raw::PREVIEW_VERSION`]) and the file, by its size and modified time.
    pub version: Vec<u8>,
}

/// The cache's key of the thumbnail of `photo`, a file of `folder`, at `size`, as
/// [`thumbnail`] reads it; `None` where the name is no plain file name or the file's size
/// and modified time cannot be read, so it is read and not kept.
pub fn thumbnail_key(folder: &Path, photo: &str, size: u32) -> Option<ThumbnailKey> {
    let plain = Path::new(photo).file_name() == Some(std::ffi::OsStr::new(photo));
    if !plain {
        return None;
    }
    let path = folder.join(photo);
    let stamp = raw_stamp(&path)?;
    let slot = thumbnail_slot(&path, size);
    let mut version = EMBEDDED.to_vec();
    version.extend_from_slice(apertrust_raw::PREVIEW_VERSION.as_bytes());
    version.push(0);
    version.extend_from_slice(&stamp);
    Some(ThumbnailKey { slot, version })
}

/// A file as a thumbnail's version knows it: its size in bytes, a little-endian `u64`,
/// then its modified time in nanoseconds since the Unix epoch, a little-endian `u128`.
pub type RawStamp = [u8; 24];

/// The [`RawStamp`] of the file at `path`; `None` where its size or modified time cannot
/// be read.
pub fn raw_stamp(path: &Path) -> Option<RawStamp> {
    let meta = std::fs::metadata(path).ok()?;
    let since = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    let mut stamp = [0; 24];
    stamp[..8].copy_from_slice(&meta.len().to_le_bytes());
    stamp[8..].copy_from_slice(&since.as_nanos().to_le_bytes());
    Some(stamp)
}

/// The sizes a thumbnail is kept at in the cache, smallest first (`docs/cache.md`,
/// Entries).
pub const THUMBNAIL_SIZES: [u32; 4] = [128, 256, 512, 1024];

/// What the version of the camera's embedded picture kept as a thumbnail starts with.
pub const EMBEDDED: &[u8] = b"embedded\0";

/// What the version of Apertrust's own render of the photo's edit kept as a thumbnail
/// starts with, its edit's hash and the render version after it (the library's step 6),
/// which an opening photo shows first, stale or not (decision 77).
pub const RENDERED: &[u8] = b"rendered\0";

/// The operations and viewport pipeline used for rendered thumbnails. Increment this
/// whenever an operation or the pipeline changes the pixels a saved edit produces,
/// including colour conversion, quantisation and thumbnail averaging, and whenever a
/// module's or an operation's defaults change, which every render of a photo with no edit
/// ([`UNALTERED`]) holds. This is separate
/// from the application's build version so unrelated changes keep the cache useful.
pub const THUMBNAIL_RENDER_VERSION: u32 = 1;

/// A rendered thumbnail's version: [`RENDERED`], the BLAKE3 hash of the sidecar's
/// exact saved bytes ([`crate::index::hash`]), then [`THUMBNAIL_RENDER_VERSION`] as
/// four little-endian bytes.
pub fn rendered_version(edit_hash: &[u8; 32]) -> Vec<u8> {
    let mut version = RENDERED.to_vec();
    version.extend_from_slice(edit_hash);
    version.extend_from_slice(&THUMBNAIL_RENDER_VERSION.to_le_bytes());
    version
}

/// Whether a rendered thumbnail was made from these saved bytes and the current
/// render version. Other rendered versions may still be shown while a new one comes. A
/// photo with no edit's render ([`UNALTERED`]) never matches here: it is current only with
/// the facts of the raw as it is now, which [`unaltered_facts`] checks.
pub fn rendered_matches(version: &[u8], edit_hash: &[u8; 32]) -> bool {
    *edit_hash != UNALTERED && version == rendered_version(edit_hash)
}

/// A rendered thumbnail's version for a photo with no edit: [`rendered_version`] of
/// [`UNALTERED`], then the raw's facts darktable's rule read as it was written, so a grid
/// read of it decides with no look at the raw: 1 or 0 for an embedded picture, its stored
/// width and height, the raw's width and height, each a little-endian `u32`, 1 or 0 for
/// Phase One, then the raw's [`RawStamp`] as they were read. A render whose facts could not
/// be read is [`rendered_version`] of [`UNALTERED`] alone, so a later read looks again.
pub fn unaltered_version(facts: &RawFacts, stamp: &RawStamp) -> Vec<u8> {
    let mut version = rendered_version(&UNALTERED);
    let (width, height) = facts.embedded.unwrap_or((0, 0));
    version.push(u8::from(facts.embedded.is_some()));
    for word in [width, height, facts.raw.0, facts.raw.1] {
        version.extend_from_slice(&word.to_le_bytes());
    }
    version.push(u8::from(facts.phase_one));
    version.extend_from_slice(stamp);
    version
}

/// The facts a rendered thumbnail of a photo with no edit records, at the current render
/// version and of the raw at `stamp` ([`unaltered_version`]); `None` for any other version,
/// one of a raw since replaced or one that records none.
pub fn unaltered_facts(version: &[u8], stamp: &RawStamp) -> Option<RawFacts> {
    let recorded = version.strip_prefix(rendered_version(&UNALTERED).as_slice())?;
    let (facts, recorded) = recorded.split_at_checked(18)?;
    if recorded != stamp {
        return None;
    }
    let [present, rest @ .., phase_one] = facts else {
        return None;
    };
    let words: Vec<u32> = rest
        .as_chunks::<4>()
        .0
        .iter()
        .map(|word| u32::from_le_bytes(*word))
        .collect();
    let [width, height, raw_width, raw_height] = words[..] else {
        return None;
    };
    (*present <= 1 && *phase_one <= 1).then_some(RawFacts {
        embedded: (*present == 1).then_some((width, height)),
        raw: (raw_width, raw_height),
        phase_one: *phase_one == 1,
    })
}

/// The edit hash a rendered thumbnail of a photo with no edit carries in its version, in
/// place of its sidecar's: every such photo, with no sidecar or one at the defaults,
/// renders alike.
pub const UNALTERED: [u8; 32] = [0; 32];

/// Whether `version` is a rendered thumbnail of a photo with no edit, whatever its render
/// version.
pub fn rendered_unaltered(version: &[u8]) -> bool {
    version
        .strip_prefix(RENDERED)
        .is_some_and(|rest| rest.starts_with(&UNALTERED))
}

/// darktable's `plugins/lighttable/thumbnail_raw_min_level` (Preferences › Lighttable ›
/// Thumbnails), the engine's setting `library.raw_thumbnails_from`: from which size a photo
/// with no edit shows a render of its raw rather than the camera's embedded picture. Each
/// size is one of darktable's thumbnail levels, `dt_mipmap_size_t` ([`LEVELS`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RawThumbnails {
    Always,
    Small,
    Vga,
    P720,
    P1080,
    Wqxga,
    K4,
    K5,
    K6,
    K8,
    #[default]
    Never,
    Auto,
}

/// Kept by darktable's own strings in `settings.toml`, in its order.
impl apertrust_base::settings::Choice for RawThumbnails {
    const OPTIONS: &'static [(RawThumbnails, &'static str)] = &[
        (RawThumbnails::Always, "always"),
        (RawThumbnails::Small, "small"),
        (RawThumbnails::Vga, "VGA"),
        (RawThumbnails::P720, "720p"),
        (RawThumbnails::P1080, "1080p"),
        (RawThumbnails::Wqxga, "WQXGA"),
        (RawThumbnails::K4, "4K"),
        (RawThumbnails::K5, "5K"),
        (RawThumbnails::K6, "6K"),
        (RawThumbnails::K8, "8K"),
        (RawThumbnails::Never, "never"),
        (RawThumbnails::Auto, "auto"),
    ];
}

/// What darktable's raw-thumbnail rule reads of a raw besides the setting (`_init_8` of
/// `mipmap_cache.c`): the stored size of the largest embedded picture, `None` where it
/// holds none or it cannot be read; the raw's size, its whole readout, as `dt_image_t`'s
/// width and height; and whether Phase One made it, whose embedded picture darktable
/// never takes, its orientation misread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawFacts {
    pub embedded: Option<(u32, u32)>,
    pub raw: (u32, u32),
    pub phase_one: bool,
}

impl RawFacts {
    /// Whether a camera of `make` is Phase One's, as darktable tells it, by the make's
    /// start.
    pub fn phase_one(make: &str) -> bool {
        make.starts_with("Phase One")
    }

    /// A raw nothing could be read of: no embedded picture, so rendered.
    pub const UNREADABLE: RawFacts = RawFacts {
        embedded: None,
        raw: (0, 0),
        phase_one: false,
    };
}

/// darktable's thumbnail levels 0 to 9, `DT_MIPMAP_0` to `DT_MIPMAP_9`: the largest
/// picture each holds, width by height (`mipmap_cache.c`, `mipsizes`, at the reference
/// release). Level 10 holds any size.
pub const LEVELS: [(u32, u32); 10] = [
    (180, 110),
    (360, 225),
    (720, 450),
    (1440, 900),
    (1920, 1200),
    (2560, 1600),
    (4096, 2560),
    (5120, 3200),
    (6144, 3456),
    (7680, 4320),
];

impl RawThumbnails {
    /// The level from which a render is shown, as darktable's
    /// `dt_mipmap_cache_get_min_mip_from_pref` gives it: always the first, each size its
    /// own, auto every one (10, `DT_MIPMAP_LDR_MAX`), never none.
    fn threshold(self) -> Option<usize> {
        match self {
            RawThumbnails::Always => Some(0),
            RawThumbnails::Small => Some(1),
            RawThumbnails::Vga => Some(2),
            RawThumbnails::P720 => Some(3),
            RawThumbnails::P1080 => Some(4),
            RawThumbnails::Wqxga => Some(5),
            RawThumbnails::K4 => Some(6),
            RawThumbnails::K5 => Some(7),
            RawThumbnails::K6 => Some(8),
            RawThumbnails::K8 => Some(9),
            RawThumbnails::Auto => Some(LEVELS.len()),
            RawThumbnails::Never => None,
        }
    }

    /// Whether a photo with no edit shows a render of its raw, where one is in hand, for a
    /// picture of at most `asked`, rather than the camera's embedded picture, as
    /// darktable's `_init_8` chooses:
    ///
    /// - a Phase One raw, or one with no embedded picture or one that cannot be read, is
    ///   rendered whatever the choice;
    /// - never: the embedded picture;
    /// - asked at a level above the choice's: the render;
    /// - at or below it: the embedded picture, unless it is smaller on both sides than the
    ///   level's own size (any size at the last level, darktable's 999999999 by 999999999)
    ///   and than the raw's less 4 pixels.
    pub fn shows_render(self, asked: (u32, u32), facts: &RawFacts) -> bool {
        let Some((width, height)) = facts.embedded.filter(|_| !facts.phase_one) else {
            return true;
        };
        let Some(threshold) = self.threshold() else {
            return false;
        };
        let level = level(asked);
        if level > threshold {
            return true;
        }
        // darktable's last level, `mipsizes[10]`, is 999999999 on both sides.
        let (level_width, level_height) =
            LEVELS.get(level).copied().unwrap_or((u32::MAX, u32::MAX));
        let (raw_width, raw_height) = facts.raw;
        width < level_width
            && height < level_height
            && width < raw_width.saturating_sub(4)
            && height < raw_height.saturating_sub(4)
    }

    /// Whether [`Self::shows_render`] at `asked` is the render whatever the raw holds.
    pub fn renders_at(self, asked: (u32, u32)) -> bool {
        self.threshold()
            .is_some_and(|threshold| level(asked) > threshold)
    }
}

/// darktable's level for a picture of at most `width` by `height`, as
/// `dt_mipmap_cache_get_matching_size` chooses it: the first that holds it, else 10.
pub fn level((width, height): (u32, u32)) -> usize {
    LEVELS
        .iter()
        .position(|&(w, h)| w >= width && h >= height)
        .unwrap_or(LEVELS.len())
}

/// The cache's slot of the thumbnail of the raw at `path` at `size`, whatever picture it
/// holds: the photo, its virtual copy and the size.
pub fn thumbnail_slot(path: &Path, size: u32) -> Vec<u8> {
    // The virtual copy: the photo itself until virtual copies are built.
    let copy = 0u32;
    let mut slot = b"thumbnail\0".to_vec();
    slot.extend_from_slice(path.as_os_str().as_encoded_bytes());
    slot.push(0);
    slot.extend_from_slice(&copy.to_le_bytes());
    slot.extend_from_slice(&size.to_le_bytes());
    slot
}

/// A photo of the index as a client sees it.
fn photo(entry: crate::index::Entry) -> Photo {
    let edit = match entry.sidecar {
        Sidecar::Read(edit) => match edit.view_only {
            Some(newer) => EditState::ViewOnly(newer),
            None if edit.edited => EditState::Edited,
            None => EditState::None,
        },
        Sidecar::Unreadable(reason) => EditState::Unreadable(reason),
        // An index that was not cancelled has read every sidecar.
        Sidecar::None | Sidecar::NotRead => EditState::None,
    };
    Photo {
        name: entry.name,
        size: entry.raw.size,
        modified: entry.raw.modified,
        edit,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    use tempfile::TempDir;

    use super::*;
    use crate::sidecar::{self, Document, Entry, VERSION};

    /// The events an open sent, and how it ended.
    fn run(
        dir: &Path,
        chunk: usize,
        cancel: &(dyn Fn() -> bool + Sync),
    ) -> (Vec<LibraryEvent>, Ended) {
        let heard = Mutex::new(Vec::new());
        let ended = open_in_chunks(
            7,
            &Source::Folder(dir.to_owned()),
            chunk,
            &|event| heard.lock().unwrap().push(event),
            cancel,
        );
        (heard.into_inner().unwrap(), ended)
    }

    fn write_sidecar(dir: &Path, raw: &str, document: &Document) {
        sidecar::write_xml(&sidecar::path_for(&dir.join(raw)), &document.to_xml()).unwrap();
    }

    fn edited() -> Document {
        let mut document = Document::new("test");
        document.entries.push(Entry::Operation {
            operation: "exposure".to_owned(),
            schema: Some(1),
            params: vec![("exposure".to_owned(), "1".to_owned())],
        });
        document
    }

    /// A folder with a photo of each edit state, two other raws and subfolders.
    fn folder() -> TempDir {
        let dir = TempDir::new().unwrap();
        for raw in ["A.RAF", "B.CR3", "C.NEF", "D.ARW", "E.DNG", "F.ORF"] {
            fs::write(dir.path().join(raw), [0u8; 4]).unwrap();
        }
        write_sidecar(dir.path(), "B.CR3", &edited());
        let mut newer = edited();
        newer.version = VERSION + 1;
        write_sidecar(dir.path(), "C.NEF", &newer);
        fs::write(sidecar::path_for(&dir.path().join("D.ARW")), "not xml").unwrap();
        write_sidecar(dir.path(), "E.DNG", &Document::new("test"));
        fs::write(dir.path().join("notes.txt"), "").unwrap();
        for sub in ["Day 10", "day 2", ".hidden", "Archive"] {
            fs::create_dir(dir.path().join(sub)).unwrap();
        }
        dir
    }

    /// A folder's subfolders, listed alone: their names in the file browser's order, the
    /// hidden one, the files and the raws left out; a folder that cannot be listed says so;
    /// nothing once cancelled.
    #[test]
    fn a_folder_s_subfolders_are_listed_alone() {
        let dir = folder();
        let root = dir.path();
        fs::create_dir(root.join("Day 10").join("Morning")).unwrap();
        let heard = Mutex::new(Vec::new());
        let listen = |event| heard.lock().unwrap().push(event);
        assert_eq!(subfolders(7, root, root, &listen, &|| false), Ended::Listed);
        let gone = root.join("gone");
        assert_eq!(
            subfolders(7, root, &gone, &listen, &|| false),
            Ended::Listed
        );
        assert_eq!(
            subfolders(7, root, root, &listen, &|| true),
            Ended::Cancelled
        );
        let heard = heard.into_inner().unwrap();
        assert_eq!(
            heard[0],
            LibraryEvent::SubfoldersOf {
                open: 7,
                folder: root.to_owned(),
                names: vec!["Archive".into(), "day 2".into(), "Day 10".into()],
                unreadable: None,
            }
        );
        let LibraryEvent::SubfoldersOf {
            names, unreadable, ..
        } = &heard[1]
        else {
            panic!("{heard:?}");
        };
        assert!(names.is_empty() && unreadable.is_some(), "{heard:?}");
        assert_eq!(heard.len(), 2);
    }

    /// Only the open folder's tree is listed: a folder outside it, a relative one and one
    /// with a `..` are refused, and nothing is sent.
    #[test]
    fn only_a_folder_inside_the_open_one_is_listed() {
        let dir = folder();
        let root = dir.path();
        let heard = Mutex::new(Vec::new());
        let listen = |event| heard.lock().unwrap().push(event);
        let outside = root.parent().unwrap().to_owned();
        for folder in [
            outside,
            root.join("Archive").join("..").join(".."),
            PathBuf::from("Archive"),
        ] {
            assert_eq!(
                subfolders(7, root, &folder, &listen, &|| false),
                Ended::Failed,
                "{folder:?}"
            );
        }
        assert!(heard.into_inner().unwrap().is_empty());
    }

    /// Links count as what they point to: one to a folder is listed, one to a file or a
    /// broken one is not; a name that is not text is not listed either.
    #[cfg(unix)]
    #[test]
    fn links_and_names_that_are_not_text_follow_the_listing_s_rules() {
        use std::os::unix::ffi::OsStrExt as _;
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        fs::create_dir(root.join("Real")).unwrap();
        fs::write(root.join("file.txt"), "").unwrap();
        std::os::unix::fs::symlink(root.join("Real"), root.join("To a folder")).unwrap();
        std::os::unix::fs::symlink(root.join("file.txt"), root.join("To a file")).unwrap();
        std::os::unix::fs::symlink(root.join("nowhere"), root.join("Broken")).unwrap();
        let not_text = std::ffi::OsStr::from_bytes(b"bad\xff");
        // Some file systems, as APFS, refuse such a name: then there is nothing to test.
        let made = fs::create_dir(root.join(not_text)).is_ok();
        let heard = Mutex::new(Vec::new());
        let listen = |event| heard.lock().unwrap().push(event);
        subfolders(7, root, root, &listen, &|| false);
        let heard = heard.into_inner().unwrap();
        let LibraryEvent::SubfoldersOf { names, .. } = &heard[0] else {
            panic!("{heard:?}");
        };
        assert_eq!(
            names,
            &["Real", "To a folder"],
            "made a non-text name: {made}"
        );
    }

    #[test]
    fn an_open_reports_the_folder_its_subfolders_and_every_photo_with_its_edit() {
        let dir = folder();

        let (events, ended) = run(dir.path(), 4, &|| false);

        assert_eq!(ended, Ended::Listed);
        assert!(events.iter().all(|event| event.open() == 7));
        let name = dir
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            events[0],
            LibraryEvent::Opened {
                open: 7,
                name,
                folder: dir.path().to_owned(),
            }
        );
        assert_eq!(
            events[1],
            LibraryEvent::Subfolders {
                open: 7,
                names: vec!["Archive".into(), "day 2".into(), "Day 10".into()],
            }
        );
        let (last, listed) = events[2..].split_last().unwrap();
        // Raws of four zero bytes hold no EXIF: each has no time.
        assert_eq!(
            *last,
            LibraryEvent::Taken {
                open: 7,
                times: vec![None; 6],
            }
        );
        let chunks: Vec<(&[Photo], bool)> = listed
            .iter()
            .map(|event| match event {
                LibraryEvent::Listed { photos, done, .. } => (photos.as_slice(), *done),
                other => panic!("unexpected {other:?}"),
            })
            .collect();
        let shape: Vec<(usize, bool)> = chunks.iter().map(|(p, done)| (p.len(), *done)).collect();
        assert_eq!(shape, [(4, false), (2, true)]);
        let photos: Vec<&Photo> = chunks.iter().flat_map(|(p, _)| p.iter()).collect();
        let names: Vec<&str> = photos.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            ["A.RAF", "B.CR3", "C.NEF", "D.ARW", "E.DNG", "F.ORF"]
        );
        assert!(photos.iter().all(|p| p.size == 4 && p.modified.is_some()));
        let edits: Vec<&EditState> = photos.iter().map(|p| &p.edit).collect();
        assert_eq!(edits[0], &EditState::None);
        assert_eq!(edits[1], &EditState::Edited);
        assert_eq!(edits[2], &EditState::ViewOnly(Newer::Version(VERSION + 1)));
        assert!(matches!(edits[3], EditState::Unreadable(reason) if !reason.is_empty()));
        // A sidecar with nothing in it is no edit.
        assert_eq!(edits[4], &EditState::None);
        assert_eq!(edits[5], &EditState::None);
    }

    #[test]
    fn an_empty_folder_sends_one_empty_chunk_done() {
        let dir = TempDir::new().unwrap();

        let (events, ended) = run(dir.path(), 4, &|| false);

        assert_eq!(ended, Ended::Listed);
        assert!(matches!(&events[1], LibraryEvent::Subfolders { names, .. } if names.is_empty()));
        assert_eq!(
            events[2..],
            [
                LibraryEvent::Listed {
                    open: 7,
                    photos: Vec::new(),
                    done: true,
                },
                LibraryEvent::Taken {
                    open: 7,
                    times: Vec::new(),
                },
            ]
        );
    }

    #[test]
    fn a_missing_folder_or_a_file_fails_without_opening() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("A.RAF");
        fs::write(&file, []).unwrap();

        for path in [dir.path().join("gone"), file] {
            let (events, ended) = run(&path, 4, &|| false);

            assert_eq!(ended, Ended::Failed);
            let [LibraryEvent::Failed { open: 7, reason }] = events.as_slice() else {
                panic!("{events:?}");
            };
            assert!(reason.contains(&*path.to_string_lossy()), "{reason}");
        }
    }

    #[test]
    fn a_cancel_after_the_last_chunk_sends_no_capture_times() {
        let dir = TempDir::new().unwrap();
        for i in 0..40 {
            fs::write(dir.path().join(format!("IMG_{i:02}.CR2")), []).unwrap();
        }
        let stop = AtomicBool::new(false);
        let heard = Mutex::new(Vec::new());

        let ended = open_in_chunks(
            7,
            &Source::Folder(dir.path().to_owned()),
            CHUNK,
            &|event| {
                if matches!(event, LibraryEvent::Listed { done: true, .. }) {
                    stop.store(true, Ordering::SeqCst);
                }
                heard.lock().unwrap().push(event);
            },
            &|| stop.load(Ordering::SeqCst),
        );

        assert_eq!(ended, Ended::Cancelled);
        let events = heard.into_inner().unwrap();
        assert!(matches!(
            events.last(),
            Some(LibraryEvent::Listed { done: true, .. })
        ));
    }

    #[test]
    fn a_cancel_between_chunks_ends_the_open_with_no_chunk_done() {
        let dir = TempDir::new().unwrap();
        for i in 0..10 {
            fs::write(dir.path().join(format!("IMG_{i:02}.CR2")), []).unwrap();
        }
        let stop = AtomicBool::new(false);
        let heard = Mutex::new(Vec::new());

        // The first chunk cancels the open, as a client's next Open would.
        let ended = open_in_chunks(
            7,
            &Source::Folder(dir.path().to_owned()),
            3,
            &|event| {
                if matches!(event, LibraryEvent::Listed { .. }) {
                    stop.store(true, Ordering::SeqCst);
                }
                heard.lock().unwrap().push(event);
            },
            &|| stop.load(Ordering::SeqCst),
        );

        assert_eq!(ended, Ended::Cancelled);
        let events = heard.into_inner().unwrap();
        let listed: Vec<bool> = events
            .iter()
            .filter_map(|event| match event {
                LibraryEvent::Listed { done, .. } => Some(*done),
                _ => None,
            })
            .collect();
        assert_eq!(listed, [false]);
    }

    #[test]
    fn a_cancel_while_the_folder_is_read_sends_nothing() {
        let dir = folder();
        let asked = std::sync::atomic::AtomicUsize::new(0);

        // Cancelled once the reading has begun.
        let (events, ended) = run(dir.path(), 4, &|| asked.fetch_add(1, Ordering::SeqCst) >= 1);

        assert_eq!(ended, Ended::Cancelled);
        assert!(events.is_empty(), "{events:?}");
    }

    /// The thumbnail events `thumbnail` sent for `photo` of `dir`.
    fn thumbnails(
        dir: &Path,
        photo: &str,
        cancel: &(dyn Fn() -> bool + Sync),
    ) -> Vec<LibraryEvent> {
        let heard = Mutex::new(Vec::new());
        thumbnail(
            7,
            dir,
            photo,
            256,
            &|event| heard.lock().unwrap().push(event),
            cancel,
        );
        heard.into_inner().unwrap()
    }

    #[test]
    fn a_raw_with_no_picture_that_can_be_read_has_no_thumbnail_and_says_so() {
        let dir = folder();

        // A file that is no raw, one that is missing, and a name that is no file name.
        for photo in ["A.RAF", "gone.ARW", "../A.RAF"] {
            assert_eq!(
                thumbnails(dir.path(), photo, &|| false),
                [LibraryEvent::Thumbnail {
                    open: 7,
                    photo: photo.to_owned(),
                    size: 256,
                    image: None,
                }]
            );
        }
        // Cancelled, it tells nothing.
        assert!(thumbnails(dir.path(), "A.RAF", &|| true).is_empty());
    }

    /// A thumbnail's slot is the photo's path, copy and size; its version follows the file
    /// and the reading, so a file changed reads again. No key for a name that is no file
    /// name, or a file that is not there.
    #[test]
    fn a_thumbnail_s_key_follows_the_photo_its_size_and_its_file() {
        let dir = folder();
        let key = thumbnail_key(dir.path(), "A.RAF", 256).expect("a key");
        assert_eq!(thumbnail_key(dir.path(), "A.RAF", 256), Some(key.clone()));
        let larger = thumbnail_key(dir.path(), "A.RAF", 512).expect("a key");
        assert_ne!(larger.slot, key.slot);
        assert_eq!(larger.version, key.version);
        let version = String::from_utf8_lossy(&key.version).into_owned();
        assert!(
            version.contains(apertrust_raw::PREVIEW_VERSION),
            "{version}"
        );

        std::fs::write(dir.path().join("A.RAF"), b"another file, longer").unwrap();
        let changed = thumbnail_key(dir.path(), "A.RAF", 256).expect("a key");
        assert_eq!(changed.slot, key.slot);
        assert_ne!(changed.version, key.version);

        assert_eq!(thumbnail_key(dir.path(), "gone.ARW", 256), None);
        assert_eq!(thumbnail_key(dir.path(), "../A.RAF", 256), None);
    }

    #[test]
    fn a_cancel_returns_at_once_while_a_call_into_the_folder_is_stuck() {
        let dir = folder();
        crate::listing::stall::hold(dir.path());
        // Should the open wait for the listing after all, it ends once the folder is
        // released, and the test fails on the time rather than hanging.
        let (done, finished) = std::sync::mpsc::channel::<()>();
        let releaser = {
            let path = dir.path().to_owned();
            std::thread::spawn(move || {
                let _ = finished.recv_timeout(Duration::from_secs(5));
                crate::listing::stall::release(&path);
            })
        };
        let started = Instant::now();

        let (events, ended) = run(dir.path(), 4, &|| {
            started.elapsed() >= Duration::from_millis(100)
        });

        let took = started.elapsed();
        assert_eq!(ended, Ended::Cancelled);
        assert!(events.is_empty(), "{events:?}");
        assert!(took < Duration::from_secs(2), "returned after {took:?}");
        drop(done);
        releaser.join().unwrap();
    }

    #[test]
    fn a_rendered_version_names_the_exact_saved_edit_and_renderer() {
        let hash = crate::index::hash(b"saved sidecar bytes");
        let version = rendered_version(&hash);
        assert_eq!(&version[..RENDERED.len()], RENDERED);
        assert_eq!(&version[RENDERED.len()..RENDERED.len() + 32], &hash);
        assert_eq!(
            &version[RENDERED.len() + 32..],
            &THUMBNAIL_RENDER_VERSION.to_le_bytes()
        );
        assert!(rendered_matches(&version, &hash));
        assert!(!rendered_matches(
            &version,
            &crate::index::hash(b"another edit")
        ));
        let mut older = version.clone();
        *older.last_mut().unwrap() ^= 1;
        assert!(!rendered_matches(&older, &hash));
        assert!(!rendered_matches(RENDERED, &hash));
        assert!(!rendered_unaltered(&version));
        assert!(rendered_unaltered(&rendered_version(&UNALTERED)));
        assert!(rendered_unaltered(&[RENDERED, &UNALTERED, b"old"].concat()));
        assert!(!rendered_unaltered(&[EMBEDDED, &UNALTERED].concat()));
    }

    /// A photo with no edit's render records the facts darktable's rule read, each read
    /// back as written for the raw it read; none for a raw since replaced, a version that
    /// records none or another edit's, and none of them matches as an edit's render does.
    #[test]
    fn an_unaltered_render_records_the_raws_facts() {
        let stamp = [7; 24];
        for facts in [
            RawFacts {
                embedded: Some((1616, 1080)),
                raw: (9504, 6336),
                phase_one: false,
            },
            RawFacts {
                embedded: None,
                raw: (6048, 4024),
                phase_one: true,
            },
        ] {
            let version = unaltered_version(&facts, &stamp);
            assert!(rendered_unaltered(&version));
            assert!(!rendered_matches(&version, &UNALTERED));
            assert_eq!(unaltered_facts(&version, &stamp), Some(facts));
            let mut replaced = stamp;
            replaced[0] ^= 1;
            assert_eq!(unaltered_facts(&version, &replaced), None);
        }
        let plain = rendered_version(&UNALTERED);
        assert_eq!(unaltered_facts(&plain, &stamp), None);
        assert!(!rendered_matches(&plain, &UNALTERED));
        let hash = crate::index::hash(b"saved sidecar bytes");
        assert_eq!(unaltered_facts(&rendered_version(&hash), &stamp), None);
        let facts = RawFacts {
            embedded: None,
            raw: (1, 1),
            phase_one: false,
        };
        let mut longer = unaltered_version(&facts, &stamp);
        longer.push(0);
        assert_eq!(unaltered_facts(&longer, &stamp), None);
    }

    /// A grid cell's size takes darktable's level whose picture holds it: 128 takes 360 by
    /// 225, as 110 rows are too few; 1024 takes 1920 by 1200; past 7680 by 4320, the last.
    #[test]
    fn a_size_takes_darktable_s_first_level_that_holds_it() {
        assert_eq!(level((128, 128)), 1);
        assert_eq!(level((180, 110)), 0);
        assert_eq!(level((256, 256)), 2);
        assert_eq!(level((512, 512)), 3);
        assert_eq!(level((1024, 1024)), 4);
        assert_eq!(level((2400, 1600)), 5);
        assert_eq!(level((8000, 8000)), 10);
    }

    fn facts(embedded: Option<(u32, u32)>) -> RawFacts {
        RawFacts {
            embedded,
            raw: (6048, 4024),
            phase_one: false,
        }
    }

    /// darktable's `_init_8`, from the reference release: a 1024 cell is 1080p's level,
    /// 1920 by 1200, so a 1620 by 1080 embedded picture is smaller than it and rendered at
    /// or below that level, and shown under never or above it.
    #[test]
    fn a_1024_cell_compares_the_embedded_picture_with_its_level_s_size() {
        let embedded = facts(Some((1620, 1080)));
        let cell = (1024, 1024);
        assert!(!RawThumbnails::Never.shows_render(cell, &embedded));
        assert!(RawThumbnails::P1080.shows_render(cell, &embedded));
        assert!(RawThumbnails::Auto.shows_render(cell, &embedded));
        assert!(RawThumbnails::P720.shows_render(cell, &embedded));
        // At a level above it, as large as its level: the embedded picture.
        let full = facts(Some((6000, 4000)));
        assert!(!RawThumbnails::Auto.shows_render(cell, &full));
        assert!(!RawThumbnails::P1080.shows_render(cell, &full));
        assert!(RawThumbnails::P720.shows_render(cell, &full));
    }

    /// A 128 cell is small's level, 360 by 225: a 320 by 240 embedded picture is not
    /// smaller on both sides, so it is shown; one of 160 by 120 is not.
    #[test]
    fn a_128_cell_shows_an_embedded_picture_as_large_as_its_level() {
        let cell = (128, 128);
        assert!(!RawThumbnails::Auto.shows_render(cell, &facts(Some((320, 240)))));
        assert!(!RawThumbnails::Small.shows_render(cell, &facts(Some((320, 240)))));
        assert!(RawThumbnails::Auto.shows_render(cell, &facts(Some((160, 120)))));
        assert!(RawThumbnails::Always.shows_render(cell, &facts(Some((320, 240)))));
    }

    /// An embedded picture within 4 pixels of the raw's own size is shown, however small
    /// beside the level.
    #[test]
    fn an_embedded_picture_the_raw_s_size_is_shown() {
        let small_raw = RawFacts {
            embedded: Some((640, 480)),
            raw: (644, 484),
            phase_one: false,
        };
        assert!(!RawThumbnails::Auto.shows_render((1024, 1024), &small_raw));
        let larger_raw = RawFacts {
            raw: (645, 485),
            ..small_raw
        };
        assert!(RawThumbnails::Auto.shows_render((1024, 1024), &larger_raw));
    }

    /// With no embedded picture, or one that cannot be read, and for a Phase One raw, a
    /// render is shown under every choice, never included.
    #[test]
    fn no_embedded_picture_or_phase_one_is_rendered_even_under_never() {
        let phase_one = RawFacts {
            phase_one: true,
            ..facts(Some((6000, 4000)))
        };
        for raw in [RawThumbnails::Never, RawThumbnails::Auto, RawThumbnails::K8] {
            assert!(raw.shows_render((256, 256), &facts(None)), "{raw:?}");
            assert!(
                raw.shows_render((256, 256), &RawFacts::UNREADABLE),
                "{raw:?}"
            );
            assert!(raw.shows_render((256, 256), &phase_one), "{raw:?}");
        }
        assert!(RawFacts::phase_one("Phase One A/S"));
        assert!(!RawFacts::phase_one("Hasselblad"));
    }

    /// The last level holds any size, as darktable's `mipsizes[10]` of 999999999 does: an
    /// embedded picture is smaller than it however large.
    #[test]
    fn darktable_s_choices_and_ids() {
        use apertrust_base::settings::Choice;
        let central = (2400, 1600);
        assert_eq!(level((10_000, 8_000)), 10);
        assert!(RawThumbnails::Auto.shows_render((10_000, 8_000), &facts(Some((6000, 4000)))));
        assert!(!RawThumbnails::Auto.shows_render((5000, 3000), &facts(Some((6000, 4000)))));
        // Larger than the size asked at the last level, still smaller than it.
        let large = RawFacts {
            embedded: Some((11_000, 8_500)),
            raw: (12_000, 9_000),
            phase_one: false,
        };
        assert!(RawThumbnails::Auto.shows_render((10_000, 8_000), &large));
        // The central panel is WQXGA's level, 2560 by 1600.
        assert!(!RawThumbnails::Wqxga.shows_render(central, &facts(Some((6000, 4000)))));
        assert!(RawThumbnails::Wqxga.shows_render(central, &facts(Some((1620, 1080)))));
        assert!(RawThumbnails::P1080.renders_at(central));
        assert!(!RawThumbnails::Wqxga.renders_at(central));
        assert!(!RawThumbnails::Never.renders_at((128, 128)));
        assert!(RawThumbnails::Always.renders_at((128, 128)));
        let ids: Vec<&str> = RawThumbnails::OPTIONS.iter().map(|(_, id)| *id).collect();
        assert_eq!(
            ids,
            [
                "always", "small", "VGA", "720p", "1080p", "WQXGA", "4K", "5K", "6K", "8K",
                "never", "auto"
            ]
        );
        assert_eq!(RawThumbnails::default(), RawThumbnails::Never);
    }
}
