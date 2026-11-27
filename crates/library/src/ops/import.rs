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

//! What the Import sheet and `apertrust-cli import` read before a photo is copied
//! (decision 85; `docs/library.md`, Import in folder mode): the source listed, a card's
//! `DCIM` whole or a folder, raws only, with their capture times. Nothing is written on
//! the source.
//!
//! The session runs [`list_source`] as the body of a job of kind `import-source`, handing
//! it an observer and the job's cancel, as it runs [`super::open`].

use std::panic::{self, AssertUnwindSafe};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{CHUNK, DateTime, LibraryEvent};
use crate::index::READERS;
use crate::source::{self, Listed};

/// When a raw of the source was taken and by which camera, as its EXIF says, for the
/// Import sheet's grid and its tooltips.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceShot {
    /// The camera's clock, `None` where the raw holds none or cannot be read, and for a
    /// cloud file not on the disk, which is not read.
    pub taken: Option<DateTime>,
    /// Make and Model as the camera wrote them.
    pub make: Option<String>,
    pub model: Option<String>,
}

/// Lists Import's source `source` for the command numbered `listing`, telling `observer`
/// in order: [`LibraryEvent::SourceListed`] chunks of at most [`CHUNK`] files, in the order
/// of their paths, the last saying `done` with what it left out, then
/// [`LibraryEvent::SourceTaken`]; or [`LibraryEvent::SourceFailed`] in their place. Where
/// `source` is on a card ([`source::card`]) the card's `DCIM` is listed whole, else the
/// folder, with its subfolders where `include_subfolders`. A blocking call, made by a job's
/// body: the folders are read on a thread of their own ([`source::list`]) and the EXIF
/// [`READERS`] at a time; once `cancel` answers true it returns and nothing more is sent.
/// The listing, for the check that follows; none where it failed or was cancelled.
pub fn list_source(
    listing: u64,
    source: &Path,
    include_subfolders: bool,
    observer: &(dyn Fn(LibraryEvent) + Send + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Option<Listed> {
    list_source_on(
        listing,
        source,
        include_subfolders,
        How {
            chunk: CHUNK,
            mount: source::mount_point,
        },
        observer,
        cancel,
    )
}

/// How [`list_source_on`] lists: its chunks' size, and how it finds a folder's mount point,
/// a test's own in place of [`source::mount_point`].
struct How<M> {
    chunk: usize,
    mount: M,
}

/// [`list_source`] as `how` says.
fn list_source_on<M>(
    listing: u64,
    source: &Path,
    include_subfolders: bool,
    how: How<M>,
    observer: &(dyn Fn(LibraryEvent) + Send + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Option<Listed>
where
    M: Fn(&Path) -> Option<std::path::PathBuf> + Send + 'static,
{
    // Each event is sent only while the listing still stands.
    let tell = |event| {
        if cancel() {
            return false;
        }
        observer(event);
        true
    };
    if cancel() {
        return None;
    }
    let listed = match source::list(source, include_subfolders, how.mount, cancel) {
        Ok(Some(listed)) => listed,
        Ok(None) => return None,
        Err(error) => {
            tracing::debug!("the source's listing failed: {error}");
            tell(LibraryEvent::SourceFailed {
                open: listing,
                reason: error.to_string(),
            });
            return None;
        }
    };
    let chunk = how.chunk.max(1);
    let chunks = listed.files.len().div_ceil(chunk).max(1);
    let mut files = listed.files.iter().cloned();
    for sent in 1..=chunks {
        let done = sent == chunks;
        let chunk = LibraryEvent::SourceListed {
            open: listing,
            folder: listed.folder.clone(),
            files: files.by_ref().take(chunk).collect(),
            done,
            left_out: done.then(|| listed.left_out.clone()),
        };
        if !tell(chunk) {
            return None;
        }
    }
    let shots = shots(&listed, cancel)?;
    if !tell(LibraryEvent::SourceTaken {
        open: listing,
        shots,
    }) {
        return None;
    }
    Some(listed)
}

/// Each listed raw's [`SourceShot`], in the listing's order, from its EXIF and not its
/// pixels, read [`READERS`] at a time; `None` once `cancel` answers true. A cloud file not
/// on the disk is not read, and a raw that cannot be read, or whose reading panics, has
/// nothing.
fn shots(listed: &Listed, cancel: &(dyn Fn() -> bool + Sync)) -> Option<Vec<SourceShot>> {
    let local: Vec<usize> = (listed.files.iter().enumerate())
        .filter(|(_, file)| file.local)
        .map(|(at, _)| at)
        .collect();
    let paths = (local.iter())
        .map(|&at| listed.folder.join(&listed.files[at].path))
        .collect();
    let (read, stopped) = crate::reads::read(
        paths,
        READERS,
        |path| {
            #[cfg(any(test, feature = "testing"))]
            crate::testing::read(path);
            panic::catch_unwind(AssertUnwindSafe(|| {
                let exif = apertrust_raw::read_exif_file(path).ok()?;
                Some(SourceShot {
                    taken: exif.taken,
                    make: exif.make_written,
                    model: exif.model_written,
                })
            }))
            .ok()
            .flatten()
            .unwrap_or_default()
        },
        cancel,
    );
    if stopped {
        return None;
    }
    let mut shots = vec![SourceShot::default(); listed.files.len()];
    for (at, shot) in read {
        shots[local[at]] = shot;
    }
    Some(shots)
}

#[cfg(test)]
mod tests;
