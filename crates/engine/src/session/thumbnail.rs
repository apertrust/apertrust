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
//! - **A read first:** the cache's entry for the photo's slot, at the version its file and
//!   the embedded picture's reading give ([`library::thumbnail_key`]), sent as the library
//!   would send it.
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
//! written without subsampled chroma, which that decoder misreads beside the export's
//! optimised tables.
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
use apertrust_library::ops::{self as library, LibraryEvent, Thumbnail, ThumbnailKey};

use super::cache::Upkeep;

/// The library's reading of a thumbnail, [`library::thumbnail`]: a test's own in its place.
pub(crate) type Read =
    fn(u64, &Path, &str, u32, &(dyn Fn(LibraryEvent) + Send + Sync), &(dyn Fn() -> bool + Sync));

/// The JPEG a thumbnail is kept as: quality 90, chroma whole.
const OPTIONS: Options = Options {
    quality: 90,
    subsample: Subsample::S444,
    dpi: 300,
};

/// What a thumbnail job asks for: `photo` of `folder` at `size`, for the open numbered
/// `open`.
pub(crate) struct Asked<'a> {
    pub open: u64,
    pub folder: &'a Path,
    pub photo: &'a str,
    pub size: u32,
}

/// Sends the thumbnail `asked` names to `observer` as [`library::thumbnail`] does, from the
/// cache where `cached` holds it, and keeps one read in the cache; with no cache, the
/// library's reading alone. `read` is [`library::thumbnail`].
pub(crate) fn serve(
    cached: Option<(&Upkeep, &Tally)>,
    asked: &Asked<'_>,
    observer: &(dyn Fn(LibraryEvent) + Send + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
    read: Read,
) {
    let Asked {
        open,
        folder,
        photo,
        size,
    } = *asked;
    let Some((upkeep, tally)) = cached else {
        return read(open, folder, photo, size, observer, cancel);
    };
    if cancel() {
        return;
    }
    let started = Instant::now();
    let key = library::thumbnail_key(folder, photo, size);
    if let Some(image) = key.as_ref().and_then(|key| hit(upkeep, key)) {
        if cancel() {
            return;
        }
        observer(LibraryEvent::Thumbnail {
            open,
            photo: photo.to_owned(),
            size,
            image: Some(image),
        });
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
        if let LibraryEvent::Thumbnail { image, .. } = &event {
            *sent.lock().unwrap_or_else(PoisonError::into_inner) = image.clone();
        }
        observer(event);
    };
    read(open, folder, photo, size, &keep, cancel);
    let took = started.elapsed();
    let image = sent.into_inner().unwrap_or_else(PoisonError::into_inner);
    let written = match (key, image) {
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

/// The picture the cache holds for `key`, or none: missing, of another version, or one
/// that cannot be read, which is read again and written over.
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
fn write(upkeep: &Upkeep, key: &ThumbnailKey, image: &Thumbnail, photo: &str) -> bool {
    let Some(jpeg) = encode(image) else {
        tracing::debug!(target: "perf", "thumbnail of {photo} not cached: it cannot be encoded");
        return false;
    };
    let cache = upkeep.cache();
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
