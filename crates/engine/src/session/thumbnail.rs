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

//! Thumbnails kept in the disk cache's `thumbnails` kind (decision 76; `docs/cache.md`,
//! Kinds; `docs/library.md`, Thumbnails), around the library's reading of the camera's
//! embedded picture ([`library::thumbnail`]).
//!
//! - **A read first:** the cache's entry for the photo's slot, a render at any version
//!   ([`rendered`]) or the embedded picture at the version its file and the embedded
//!   picture's reading give ([`library::thumbnail_key`]), sent as the library would send
//!   it.
//! - **One lock,** `WRITES`, orders the replacements and the sends: held only to look at
//!   the slot's version and send or write, never through a read or a decode, so cached
//!   thumbnails come in parallel, and an embedded picture never follows a render.
//! - **On a miss:** the library reads and sends the picture as without a cache, then the
//!   job writes it to the cache as a JPEG of quality 90, through the export's JPEG writer,
//!   so a write never delays the picture shown. A write past the limit asks for an
//!   eviction ([`Upkeep::check`]). A picture that cannot be encoded, or no picture, is
//!   sent and not kept.
//! - **Without a cache,** the library's reading alone, as before.
//!
//! The encoding is here, in the engine, since the library depends on neither the export
//! nor the cache's writes of its own until its other kinds come, and the engine runs the
//! thumbnail jobs. JPEGs are read back by rawler's decoder ([`apertrust_raw::read_jpeg`]),
//! zune-jpeg, written with chroma halved across (4:2:2): zune-jpeg misreads the export's
//! 4:2:0, whose optimised tables give a scan for each component. An entry that does not
//! end as a JPEG ends, cut short or not a JPEG at all, is a miss, read again and written
//! over.
//!
//! Under `-d perf`, one line for each [`LibraryCommand::Thumbnails`] once its last job has
//! ended: how many came from the cache and how many were read, each with their time, and
//! how many were written; under `-d verbose`, a line for each thumbnail.
//!
//! [`LibraryCommand::Thumbnails`]: super::LibraryCommand::Thumbnails

use std::path::Path;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use apertrust_base::cache::THUMBNAILS;
use apertrust_export::Image;
use apertrust_export::jpeg::{self, Options, Subsample};
use apertrust_library::ops::{
    self as library, LibraryEvent, RawFacts, RawThumbnails, Thumbnail, ThumbnailKey,
};

use super::cache::Upkeep;

/// The library's reading of a thumbnail, [`library::thumbnail`]: a test's own in its place.
pub(crate) type Read =
    fn(u64, &Path, &str, u32, &(dyn Fn(LibraryEvent) + Send + Sync), &(dyn Fn() -> bool + Sync));

/// What darktable's raw-thumbnail rule reads of the raw at a path, [`raw_facts`]: a test's
/// own in its place.
pub(crate) type Facts = fn(&Path) -> RawFacts;

/// The raw's facts for darktable's raw-thumbnail rule, read from the file: its metadata
/// without the pixels, and its embedded pictures. One that cannot be read has none, and is
/// rendered.
pub(crate) fn raw_facts(path: &Path) -> RawFacts {
    let read = || {
        let file = apertrust_raw::RawFile::open(path).ok()?;
        let (raw, _) = file.metadata().ok()?;
        let (_, embedded) = file.embedded(1).unwrap_or((None, None));
        Some(RawFacts {
            embedded,
            raw: (raw.width, raw.height),
            phase_one: RawFacts::phase_one(&raw.camera.make),
        })
    };
    read().unwrap_or(RawFacts::UNREADABLE)
}

/// Orders the thumbnails' cache replacements and the events that send them: an embedded
/// picture never replaces a render, nor is sent after one.
static WRITES: Mutex<()> = Mutex::new(());

/// The JPEG a thumbnail is kept as: quality 90, chroma halved across.
const OPTIONS: Options = Options {
    quality: 90,
    subsample: Subsample::S422,
    dpi: 300,
};

/// What a thumbnail job asks for: `photo` of `folder` at `size`, for the open numbered
/// `open`, under the setting `raw`.
pub(crate) struct Asked<'a> {
    pub open: u64,
    pub folder: &'a Path,
    pub photo: &'a str,
    pub size: u32,
    pub raw: RawThumbnails,
}

/// Sends the thumbnail `asked` names to `observer` as [`library::thumbnail`] does, from the
/// cache where `cached` holds it, and keeps one read in the cache; with no cache, the
/// library's reading alone. `read` is [`library::thumbnail`], `facts` [`raw_facts`].
pub(crate) fn serve(
    cached: Option<(&Upkeep, &Tally)>,
    asked: &Asked<'_>,
    observer: &(dyn Fn(LibraryEvent) + Send + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
    read: Read,
    facts: Facts,
) {
    let Asked {
        open,
        folder,
        photo,
        size,
        raw,
    } = *asked;
    let Some((upkeep, tally)) = cached else {
        return read(open, folder, photo, size, observer, cancel);
    };
    if cancel() {
        return;
    }
    let started = Instant::now();
    let key = library::thumbnail_key(folder, photo, size);
    let send = |image| {
        observer(LibraryEvent::Thumbnail {
            open,
            photo: photo.to_owned(),
            size,
            image,
        })
    };
    // A read first, the slot's render, current or stale, else its embedded picture,
    // decoded outside the replacement lock.
    let found = key.as_ref().and_then(|key| {
        rendered_hit(upkeep, &key.slot)
            .or_else(|| hit(upkeep, key).map(|image| (key.version.clone(), image)))
    });
    // A render of a photo with no edit stands where darktable's rule shows one for this
    // raw at this size, decided from the facts its version records of the raw as it is,
    // the raw read only for a render that records none or of a raw since replaced; else
    // the embedded picture takes its slot, the render's version what it replaces.
    let asks = (size, size);
    let (found, unaltered) = match found {
        Some((version, image)) if library::rendered_unaltered(&version) => {
            let read = || {
                let path = folder.join(photo);
                (library::raw_stamp(&path))
                    .and_then(|stamp| library::unaltered_facts(&version, &stamp))
                    .unwrap_or_else(|| facts(&path))
            };
            let shows = raw.renders_at(asks) || raw.shows_render(asks, &read());
            if shows {
                (Some((version, image)), None)
            } else {
                (None, Some(version))
            }
        }
        found => (found, None),
    };
    if let (Some(key), Some((version, image))) = (&key, found) {
        if !send_current(upkeep, &key.slot, Some(version), Some(image), cancel, &send) {
            return;
        }
        let took = started.elapsed();
        tally.count(|counts| {
            counts.hits += 1;
            counts.hit_time += took;
        });
        let ms = took.as_secs_f64() * 1e3;
        tracing::trace!(target: "perf", "thumbnail of {photo} at {size}: cached, {ms:.1} ms");
        return;
    }
    // A miss: read and sent as without a cache, the picture kept as it goes out.
    let sent = Mutex::new(None);
    let keep = |event: LibraryEvent| {
        let LibraryEvent::Thumbnail { image, .. } = event else {
            return observer(event);
        };
        *sent.lock().unwrap_or_else(PoisonError::into_inner) = image.clone();
        match &key {
            // The slow read may end after a render has updated the grid: it never
            // replaces that render, but the one of a photo with no edit it stands for.
            Some(key) => {
                send_current(upkeep, &key.slot, unaltered.clone(), image, cancel, &send);
            }
            None => send(image),
        }
    };
    read(open, folder, photo, size, &keep, cancel);
    let took = started.elapsed();
    let image = sent.into_inner().unwrap_or_else(PoisonError::into_inner);
    let written = match (key, image) {
        // The embedded picture in place of a render of the photo with no edit that the
        // setting does not show.
        (Some(key), Some(image))
            if !cancel()
                && let Some(over) = &unaltered =>
        {
            replace_unaltered(upkeep, &key, &image, photo, over, cancel)
        }
        (Some(key), Some(image)) if !cancel() => write(upkeep, &key, &image, photo),
        _ => false,
    };
    tally.count(|counts| {
        counts.misses += 1;
        counts.miss_time += took;
        counts.written += u64::from(written);
    });
    let ms = took.as_secs_f64() * 1e3;
    let kept = if written { ", written" } else { "" };
    tracing::trace!(target: "perf", "thumbnail of {photo} at {size}: read, {ms:.1} ms{kept}");
}

/// Sends `picture`, read from `slot` at `version` or chosen over the render there of that
/// version (`None` where it was not read from the cache), unless a render has taken the
/// slot since: then that render, read and decoded outside the replacement lock, which is
/// held only to look at the slot's version and send, so a render's own event never
/// follows an older picture. False, sending nothing, once cancelled.
fn send_current(
    upkeep: &Upkeep,
    slot: &[u8],
    mut version: Option<Vec<u8>>,
    mut picture: Option<Thumbnail>,
    cancel: &(dyn Fn() -> bool + Sync),
    send: &dyn Fn(Option<Thumbnail>),
) -> bool {
    loop {
        {
            let _sending = WRITES.lock().unwrap_or_else(PoisonError::into_inner);
            if cancel() {
                return false;
            }
            let now = (upkeep.cache().version(THUMBNAILS, slot).ok()).flatten();
            let rendered = now.filter(|now| now.starts_with(library::RENDERED));
            if rendered.is_none() || rendered == version {
                send(picture);
                return true;
            }
        }
        // One that cannot be read leaves the picture in hand to be sent.
        let Some((newer, image)) = rendered_hit(upkeep, slot) else {
            let _sending = WRITES.lock().unwrap_or_else(PoisonError::into_inner);
            if cancel() {
                return false;
            }
            send(picture);
            return true;
        };
        (version, picture) = (Some(newer), Some(image));
    }
}

/// The last rendered picture of this slot, current or stale, with its version. A
/// malformed JPEG is a miss, allowing the embedded picture to stand in until rendering
/// succeeds.
pub(super) fn rendered_hit(upkeep: &Upkeep, slot: &[u8]) -> Option<(Vec<u8>, Thumbnail)> {
    let (version, bytes) = upkeep.cache().read_any(THUMBNAILS, slot).ok()??;
    if !version.starts_with(library::RENDERED) {
        return None;
    }
    let picture = apertrust_raw::read_jpeg(&bytes).ok()?;
    let image = Thumbnail {
        width: picture.width,
        height: picture.height,
        pixels: picture.pixels.into(),
    };
    Some((version, image))
}

/// The picture the cache holds for `key`, or none: missing, of another version, or one
/// that cannot be read or is cut short, which is read again and written over.
fn hit(upkeep: &Upkeep, key: &ThumbnailKey) -> Option<Thumbnail> {
    let bytes = upkeep
        .cache()
        .read(THUMBNAILS, &key.slot, &key.version)
        .ok()??;
    let picture = apertrust_raw::read_jpeg(&bytes).ok()?;
    Some(Thumbnail {
        width: picture.width,
        height: picture.height,
        pixels: picture.pixels.into(),
    })
}

/// `image` as the JPEG the cache keeps; `None` where it cannot be encoded.
pub(crate) fn encode(image: &Thumbnail) -> Option<Vec<u8>> {
    let pixels: Vec<[f32; 4]> = image
        .pixels
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| p.map(|v| f32::from(v) / 255.0))
        .collect();
    let image = Image::new(&pixels, image.width, image.height).ok()?;
    let mut jpeg = Vec::new();
    jpeg::write(image, &OPTIONS, &mut jpeg).ok()?;
    Some(jpeg)
}

/// Writes `image` to the cache under `key`, asking for an eviction where that takes the
/// limited kinds past the limit; whether it was written. A picture that cannot be
/// encoded, or a write that fails, is left out, said under `-d perf`.
pub(super) fn write(upkeep: &Upkeep, key: &ThumbnailKey, image: &Thumbnail, photo: &str) -> bool {
    write_checked(upkeep, key, image, photo, Over::Embedded, &|| false)
}

/// Publishes a completed picture in cache replacement order. A superseded worker
/// cannot send an old picture after a newer worker has replaced and published it.
pub(super) fn publish_if_current(cancel: &(dyn Fn() -> bool + Sync), publish: impl FnOnce()) {
    let _writing = WRITES.lock().unwrap_or_else(PoisonError::into_inner);
    if !cancel() {
        publish();
    }
}

/// Writes a render only while its saved edit is still current, checking after JPEG
/// encoding and under the cache replacement lock.
pub(super) fn write_if_current(
    upkeep: &Upkeep,
    key: &ThumbnailKey,
    image: &Thumbnail,
    photo: &str,
    cancel: &(dyn Fn() -> bool + Sync),
) -> bool {
    write_checked(upkeep, key, image, photo, Over::Embedded, cancel)
}

/// Replaces a previous render after the saved edit returns to defaults. Cancellation
/// is checked under the replacement lock, so a superseded reset writes nothing.
pub(super) fn replace_with_embedded(
    upkeep: &Upkeep,
    key: &ThumbnailKey,
    image: &Thumbnail,
    photo: &str,
    cancel: &(dyn Fn() -> bool + Sync),
) -> bool {
    if !key.version.starts_with(library::EMBEDDED) {
        return false;
    }
    write_checked(upkeep, key, image, photo, Over::Any, cancel)
}

/// Drops a previous rendered slot after a reset with no embedded picture. Other
/// versions are kept, and a superseded reset does nothing. Returns whether the
/// desired state holds, including a slot that was already absent.
pub(super) fn remove_rendered_if_current(
    upkeep: &Upkeep,
    path: &Path,
    size: u32,
    cancel: &(dyn Fn() -> bool + Sync),
) -> bool {
    let _writing = WRITES.lock().unwrap_or_else(PoisonError::into_inner);
    if cancel() {
        return false;
    }
    let slot = library::thumbnail_slot(path, size);
    let cache = upkeep.cache();
    match cache.read_any(THUMBNAILS, &slot) {
        Ok(Some((version, _))) if version.starts_with(library::RENDERED) => {
            cache.remove(THUMBNAILS, &slot).is_ok()
        }
        Ok(_) => true,
        Err(_) => false,
    }
}

/// Writes the embedded picture `image` over the render of a photo with no edit, the slot's
/// version `over`, which the setting does not show at this size, unless another render has
/// taken the slot since.
pub(super) fn replace_unaltered(
    upkeep: &Upkeep,
    key: &ThumbnailKey,
    image: &Thumbnail,
    photo: &str,
    over: &[u8],
    cancel: &(dyn Fn() -> bool + Sync),
) -> bool {
    write_checked(upkeep, key, image, photo, Over::Version(over), cancel)
}

/// What a write may replace.
enum Over<'a> {
    /// Anything but a render that can be read.
    Embedded,
    /// Anything, as a reset does.
    Any,
    /// Only the entry of this version.
    Version(&'a [u8]),
}

fn write_checked(
    upkeep: &Upkeep,
    key: &ThumbnailKey,
    image: &Thumbnail,
    photo: &str,
    over: Over<'_>,
    cancel: &(dyn Fn() -> bool + Sync),
) -> bool {
    let Some(jpeg) = encode(image) else {
        tracing::debug!(target: "perf", "thumbnail of {photo} not cached: it cannot be encoded");
        return false;
    };
    let cache = upkeep.cache();
    let rendered = |version: &Option<Vec<u8>>| {
        version
            .as_ref()
            .is_some_and(|version| version.starts_with(library::RENDERED))
    };
    // A render that cannot be read is replaced: told before the lock, which decodes nothing.
    let before = (cache.version(THUMBNAILS, &key.slot).ok()).flatten();
    let unreadable = rendered(&before) && rendered_hit(upkeep, &key.slot).is_none();
    let _writing = WRITES.lock().unwrap_or_else(PoisonError::into_inner);
    if cancel() {
        return false;
    }
    let now = (cache.version(THUMBNAILS, &key.slot).ok()).flatten();
    let replaces = match over {
        // A rendered picture may have arrived while the embedded JPEG was read. Keep
        // it rather than replacing an edit's picture with the camera's original.
        Over::Embedded => {
            !key.version.starts_with(library::EMBEDDED)
                || !rendered(&now)
                || (unreadable && now == before)
        }
        Over::Any => true,
        Over::Version(over) => now.as_deref() == Some(over),
    };
    if !replaces {
        return false;
    }
    if let Err(error) = cache.write(THUMBNAILS, &key.slot, &key.version, &jpeg) {
        tracing::debug!(target: "perf", "thumbnail of {photo} not cached: {error}");
        return false;
    }
    upkeep.check();
    true
}

/// How the thumbnails of one [`LibraryCommand::Thumbnails`] at one size fared, shared by
/// its jobs and logged once the last has ended.
///
/// [`LibraryCommand::Thumbnails`]: super::LibraryCommand::Thumbnails
pub(crate) struct Tally {
    size: u32,
    counts: Mutex<Counts>,
}

#[derive(Default)]
pub(crate) struct Counts {
    pub hits: u64,
    pub hit_time: Duration,
    pub misses: u64,
    pub miss_time: Duration,
    pub written: u64,
}

impl Tally {
    pub(crate) fn new(size: u32) -> Tally {
        Tally {
            size,
            counts: Mutex::default(),
        }
    }

    fn count(&self, change: impl FnOnce(&mut Counts)) {
        change(&mut self.counts.lock().unwrap_or_else(PoisonError::into_inner));
    }

    /// The counts so far, for a test.
    #[cfg(test)]
    pub(crate) fn counts(&self) -> (u64, u64, u64) {
        let counts = self.counts.lock().unwrap_or_else(PoisonError::into_inner);
        (counts.hits, counts.misses, counts.written)
    }
}

impl Drop for Tally {
    fn drop(&mut self) {
        let counts = self
            .counts
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner);
        if counts.hits + counts.misses == 0 {
            return;
        }
        let ms = |time: Duration| time.as_secs_f64() * 1e3;
        let (hits, misses, written) = (counts.hits, counts.misses, counts.written);
        let (hit_ms, miss_ms) = (ms(counts.hit_time), ms(counts.miss_time));
        tracing::debug!(
            target: "perf",
            "thumbnails at {}: cached {hits} in {hit_ms:.1} ms, read {misses} in {miss_ms:.1} ms, \
             written {written}",
            self.size
        );
    }
}

#[cfg(test)]
mod tests;

pub(super) mod rendered;
