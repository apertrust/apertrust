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
//! `DCIM` whole or a folder, raws only, with their capture times; and each of its photos
//! told new or imported before, by the destination alone. Nothing is written on the
//! source, nor anywhere else.
//!
//! The session runs [`list_source`] and [`check_source`] as the bodies of jobs of kind
//! `import-source`, handing each an observer and the job's cancel, as it runs
//! [`super::open`]; and [`preview_import`], where each checked photo would go and what
//! stands in the way ([`preview`]), as a preview's job.

use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use apertrust_base::variables::Places;
use serde::{Deserialize, Serialize};

use super::{CHUNK, DateTime, Ended, Fingerprints, LibraryEvent, raw_state};
use crate::imported::{self, Candidate, Sought, Walked};
use crate::index::READERS;
use crate::source::{self, Listed};

mod plan;
mod preview;

pub use preview::{
    FateCounts, Fix, FolderPreview, ImportFate, ImportPhotos, ImportPreview, ImportProblem,
    OnSource, PhotoPreview, Problem, Unwritable, preview_import,
};

/// Whether a photo of the source was imported before, as the destination tells it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SourceState {
    /// No copy of it is in the trees checked.
    New,
    /// A file in the trees, `copy`, has its size, its name or the camera name its `.apxmp`
    /// keeps, and its fingerprint: a probable match, which leaves the photo unchecked and
    /// shown, never skipped by the copy on it alone.
    ImportedBefore { copy: PathBuf },
    /// Its candidate, a file of its size and name, is a cloud file not on the disk, never
    /// fingerprinted (decision 53), or the photo itself is: probably imported, until that
    /// file is on the disk.
    ProbablyImported { candidate: PathBuf },
}

/// The trees a check of the source walks, as the client sends them: the patterns of
/// `import.base_folder` and `import.default_folder` as the settings hold them, and the
/// person's folders and name they expand from, since the engine reads no platform folder
/// (decision 43).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trees {
    pub base_folder: String,
    pub default_folder: String,
    pub places: Places,
}

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

/// Checks the source `listed` against `trees` for the listing numbered `listing`, as its
/// check numbered `check`, telling `observer` each photo's [`SourceState`] in
/// [`LibraryEvent::SourceChecked`] chunks of at most [`CHUNK`], in the listing's order,
/// each saying how many are checked of how many and naming the trees walked, the last
/// saying `done`. The trees ([`imported::trees`]) are walked first for candidates, by name
/// and size ([`imported::candidates`]); then each photo with any is confirmed by its
/// fingerprint and theirs, through the session's memo, [`READERS`] at a time. A photo with
/// none is new and read not at all; a cloud file not on the disk is never fingerprinted.
/// A walk that fails, a panic, is warned of and tells every photo new, never silence.
/// A blocking call, made by a job's body; once `cancel` answers true it returns and nothing
/// more is sent.
pub fn check_source(
    listing: u64,
    check: u64,
    listed: &Listed,
    trees: &Trees,
    fingerprints: &Arc<Fingerprints>,
    observer: &(dyn Fn(LibraryEvent) + Send + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Ended {
    check_in_chunks(
        (listing, check),
        listed,
        trees,
        (fingerprints, CHUNK),
        observer,
        cancel,
    )
}

/// Lists the volumes mounted a person would import from ([`crate::volumes`]) and tells
/// `observer` once, with [`LibraryEvent::Volumes`]. A blocking call, made by a job's body:
/// the mounts are read on a thread of their own, waited for while `cancel` is asked every
/// 20 ms, so a share that hangs holds that thread alone; once `cancel` answers true it
/// returns and sends nothing.
pub fn volumes(
    observer: &(dyn Fn(LibraryEvent) + Send + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Ended {
    let (sent, answer) = std::sync::mpsc::sync_channel(1);
    let reading = std::thread::Builder::new()
        .name("volumes".to_owned())
        .spawn(move || {
            let mounts = crate::volumes::mounts();
            let _ = sent.send(crate::volumes::volumes(
                &mounts,
                crate::volumes::System::THIS,
            ));
        });
    if let Err(error) = reading {
        tracing::warn!("the volumes' listing cannot start: {error}");
        return Ended::Failed;
    }
    loop {
        if cancel() {
            return Ended::Cancelled;
        }
        match answer.recv_timeout(std::time::Duration::from_millis(20)) {
            Ok(volumes) => {
                if cancel() {
                    return Ended::Cancelled;
                }
                observer(LibraryEvent::Volumes { volumes });
                return Ended::Listed;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return Ended::Failed,
        }
    }
}

/// Whether `path` names a file inside the folder it is relative to: not empty, and each of
/// its parts a name, none a root, a drive, `.` or `..`, so that joined to the folder it
/// never leads out of it. A source's thumbnails are asked for by such paths.
pub fn relative_inside(path: &Path) -> bool {
    path.components().next().is_some()
        && (path.components()).all(|part| matches!(part, std::path::Component::Normal(_)))
}

/// [`check_source`] with chunks of at most `chunk` photos.
fn check_in_chunks(
    (listing, check): (u64, u64),
    listed: &Listed,
    trees: &Trees,
    (fingerprints, chunk): (&Arc<Fingerprints>, usize),
    observer: &(dyn Fn(LibraryEvent) + Send + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Ended {
    if cancel() {
        return Ended::Cancelled;
    }
    let folders = imported::trees(&trees.base_folder, &trees.default_folder, &trees.places);
    let name = |path: &Path| {
        (path.file_name()).map_or_else(String::new, |name| name.to_string_lossy().into_owned())
    };
    let names: Vec<String> = listed.files.iter().map(|file| name(&file.path)).collect();
    let sought = Sought::new(
        (names.iter().zip(&listed.files)).map(|(name, file)| (name.as_str(), file.size)),
    );
    let of = listed.files.len();
    let found =
        match imported::candidates(&folders, &listed.folder, &listed.folders, sought, cancel) {
            Walked::Found(found) => found,
            Walked::Cancelled => return Ended::Cancelled,
            // Never silence, which would leave the sheet waiting: every photo new, as a tree
            // that is not there tells it.
            Walked::Failed => {
                let walked: Vec<_> = folders.iter().map(|folder| folder.display()).collect();
                tracing::warn!("the check against {walked:?} failed: every photo told new");
                vec![Vec::new(); of]
            }
        };
    let chunk = chunk.max(1);
    let chunks = of.div_ceil(chunk).max(1);
    for sent in 1..=chunks {
        let photos = (sent - 1) * chunk..(sent * chunk).min(of);
        let Some(states) = confirm(listed, &found, photos.clone(), fingerprints, cancel) else {
            return Ended::Cancelled;
        };
        if cancel() {
            return Ended::Cancelled;
        }
        observer(LibraryEvent::SourceChecked {
            open: listing,
            check,
            folders: folders.clone(),
            states,
            checked: photos.end,
            of,
            done: sent == chunks,
        });
    }
    Ended::Listed
}

/// The states of the listing's `photos`, each with its candidates in `found`: those with
/// any confirmed by their fingerprints, [`READERS`] at a time, through `fingerprints`;
/// `None` once `cancel` answers true.
fn confirm(
    listed: &Listed,
    found: &[Vec<Candidate>],
    photos: std::ops::Range<usize>,
    fingerprints: &Arc<Fingerprints>,
    cancel: &(dyn Fn() -> bool + Sync),
) -> Option<Vec<SourceState>> {
    let mut states = vec![SourceState::New; photos.len()];
    let sought: HashMap<PathBuf, (usize, Vec<Candidate>)> = (photos.clone())
        .filter(|&at| !found[at].is_empty())
        .map(|at| {
            let path = listed.folder.join(&listed.files[at].path);
            (path, (at - photos.start, found[at].clone()))
        })
        .collect();
    let paths: Vec<PathBuf> = sought.keys().cloned().collect();
    let sought = Arc::new(sought);
    let reader = {
        let (sought, fingerprints) = (Arc::clone(&sought), Arc::clone(fingerprints));
        move |path: &Path| {
            let (at, candidates) = &sought[path];
            (*at, state(path, candidates, &fingerprints))
        }
    };
    let (read, stopped) = crate::reads::read(paths, READERS, reader, cancel);
    if stopped {
        return None;
    }
    for (_, (at, state)) in read {
        states[at] = state;
    }
    Some(states)
}

/// The state of the source photo at `path`, which has `candidates`: imported before where
/// a candidate on the disk has its fingerprint, the first that has it; else probably
/// imported where a candidate, or the photo, is a cloud file not on the disk; else new. A
/// cloud file is never fingerprinted, nor the photo where no candidate can be.
fn state(path: &Path, candidates: &[Candidate], fingerprints: &Fingerprints) -> SourceState {
    let probably = |candidate: &Candidate| SourceState::ProbablyImported {
        candidate: candidate.path.clone(),
    };
    let cloud = candidates.iter().find(|candidate| !candidate.local);
    let source = raw_state(path);
    if source.is_some_and(|source| !source.local) {
        return probably(&candidates[0]);
    }
    let fingerprint = (source.filter(|_| candidates.iter().any(|candidate| candidate.local)))
        .and_then(|source| fingerprints.get(path, &source));
    if let Some(fingerprint) = fingerprint {
        for candidate in candidates.iter().filter(|candidate| candidate.local) {
            let theirs = raw_state(&candidate.path)
                .and_then(|state| fingerprints.get(&candidate.path, &state));
            if theirs.as_ref() == Some(&fingerprint) {
                return SourceState::ImportedBefore {
                    copy: candidate.path.clone(),
                };
            }
        }
    }
    cloud.map_or(SourceState::New, probably)
}

#[cfg(test)]
mod tests;
