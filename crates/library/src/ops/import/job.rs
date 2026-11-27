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

//! Import's copy (decision 85; `docs/library.md`, Import in folder mode, The copy): the
//! checked photos copied into their destination one at a time, in the order of their paths,
//! each through the checked copy ([`crate::copy::land`]), whole or not at all, never
//! overwriting a file and never writing on the source.
//!
//! Each photo's name is worked out again as it goes, since files may have come meanwhile
//! ([`super::plan::place`]): a main file there the same byte for byte, compared whole as
//! the source is read ([`crate::copy::same_file`]), is the photo's copy, which is skipped,
//! its missing sidecars copied alone; a sidecar there that differs from the source's, a
//! diverged edit, skips the photo with nothing written. A photo's own failure is that
//! photo's and the import goes on; the source or the destination failing stops it, the
//! copies made staying. Each folder the import made and left empty is removed as it ends,
//! deepest first; a folder that was there before is never removed.

use std::path::{Path, PathBuf};

use apertrust_base::variables::DateTime;
use serde::{Deserialize, Serialize};

use super::LibraryEvent;
use super::plan::{self, Place};
use super::preview::{self, ImportPhotos, World};
use crate::copy::{self, Check, CopyError, Landing, PhotoFiles};
use crate::destination::Patterns;
use crate::settings::ImportSettings;

/// One photo of an import, as it ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedPhoto {
    /// Its path relative to the source's folder, as the listing names it.
    pub source: PathBuf,
    pub result: ImportResult,
}

/// What became of a photo of an import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ImportResult {
    /// It landed at `copy`, its sidecars with it.
    Copied { copy: PathBuf },
    /// Skipped: its copy is at `copy` already, the same byte for byte; its sidecars missing
    /// there were copied alone.
    There { copy: PathBuf },
    /// Skipped: its copy is at `copy` with a sidecar of its own, `sidecar`, that differs
    /// from the source's, a diverged edit; nothing was written.
    Diverged { copy: PathBuf, sidecar: PathBuf },
    /// It failed, for `cause`, in plain words `reason`; nothing of it is at the
    /// destination.
    Failed { cause: ImportCause, reason: String },
}

/// Why a photo of an import failed, or the import stopped, as a code a client words
/// (`job-cause-<id>`, [`ImportCause::id`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportCause {
    /// A file of the photo could not be read.
    Unreadable,
    /// A file appeared at the destination under the copy's name as it landed: never
    /// replaced.
    Exists,
    /// The destination's disk is full or over its quota.
    DiskFull,
    /// The source, or the destination folder, is no longer there: a card pulled out, a
    /// share dropped.
    Gone,
    /// A file could not be written for another reason.
    Write,
    /// A folder pattern, Folder's or Subfolder's, cannot be read, or Subfolder leads out of
    /// Folder: refused before any photo.
    Pattern,
    /// A destination folder is no absolute folder: refused before any photo.
    Folder,
    /// The destination is the source, lies inside it, or is on its card: refused before
    /// any photo.
    OnSource,
}

impl ImportCause {
    /// Its id, which a client words as `job-cause-<id>`.
    pub fn id(self) -> &'static str {
        match self {
            ImportCause::Unreadable => "unreadable",
            ImportCause::Exists => "exists",
            ImportCause::DiskFull => "disk-full",
            ImportCause::Gone => "gone",
            ImportCause::Write => "write",
            ImportCause::Pattern => "folder-pattern",
            ImportCause::Folder => "folder",
            ImportCause::OnSource => "on-source",
        }
    }
}

/// What stopped an import, or the first photo's failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportFailure {
    pub cause: ImportCause,
    /// The photo it failed on, by its path relative to the source's folder.
    pub photo: Option<PathBuf>,
    pub reason: String,
}

/// How an import ended.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportEnd {
    /// How many photos there were, and how many landed, were skipped as there already or
    /// with an edit of their own, failed, and were not reached.
    pub total: usize,
    pub copied: usize,
    pub there: usize,
    pub diverged: usize,
    pub failed: usize,
    pub not_reached: usize,
    pub cancelled: bool,
    /// What refused the import before any photo, the same settings refused again.
    pub refused: Option<ImportFailure>,
    /// What stopped it, the source or the destination failing; else the first photo's
    /// failure.
    pub failure: Option<ImportFailure>,
    /// The photos Try again copies: those that failed and those not reached, in order;
    /// none where it was cancelled or refused.
    pub left: Vec<PathBuf>,
    /// The folder the photos go to, the deepest common folder of their destinations, and
    /// whether any photo landed in it.
    pub destination: Option<PathBuf>,
    pub landed: bool,
}

/// What the import's row is told as it goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportStep<'a> {
    /// The folder the photos go to, once worked out.
    Destination(&'a Path),
    /// Photo `done` of `total`, from 0, is under way: its file name.
    Photo {
        done: usize,
        total: usize,
        name: &'a str,
    },
    /// `done` of `total` photos are done.
    Done { done: usize, total: usize },
}

/// Copies `photos` with `settings` for the command numbered `import`, the import's time
/// `now`, through the checked copy (the module's rules), telling `observer` each photo as it
/// ends in an [`LibraryEvent::Imported`] of its own, and `step` the row's progress. `build`
/// names the build in a new `.apxmp` a renamed copy keeps its camera name in. A blocking
/// call, made by a job's body; `cancel` is asked before each file and each chunk, and a
/// cancel leaves the copies made, no temporary and no photo half landed.
#[allow(
    clippy::too_many_arguments,
    reason = "the command's own and what the job hands every photo"
)]
pub fn run_import(
    import: u64,
    photos: &ImportPhotos,
    settings: &ImportSettings,
    now: DateTime,
    build: &str,
    observer: &(dyn Fn(LibraryEvent) + Send + Sync),
    step: &dyn Fn(ImportStep<'_>),
    cancel: &(dyn Fn() -> bool + Sync),
) -> ImportEnd {
    let world = World {
        now,
        mount: &crate::source::mount_point,
        free_space: &|_| None,
    };
    let mut land = |source: &PhotoFiles, landing: &Landing<'_>, made: &mut Vec<PathBuf>| {
        copy::land(source, landing, Check::None, cancel, &mut |_| {}, made)
    };
    let how = Lands {
        world: &world,
        land: &mut land,
        before: &|_| {},
    };
    import_in(import, photos, settings, build, how, observer, step, cancel)
}

/// The copy of one photo as a [`Landing`] says, the folders it makes added to the list.
pub(crate) type Land<'a> =
    dyn FnMut(&PhotoFiles, &Landing<'_>, &mut Vec<PathBuf>) -> Result<copy::Copied, CopyError> + 'a;

/// How [`import_in`] copies: the world the preview reads, the copy, and a test's hook
/// before each photo, by its place.
pub(crate) struct Lands<'a> {
    pub world: &'a World<'a>,
    pub land: &'a mut Land<'a>,
    pub before: &'a dyn Fn(usize),
}

/// What became of one photo, and what it tells of the import.
enum Outcome {
    Done(ImportResult),
    /// It failed and the import stops: the source or the destination failed.
    Stop(ImportCause, String),
    Cancelled,
}

/// [`run_import`] as `how` says.
#[allow(
    clippy::too_many_arguments,
    reason = "the command's own and what the job hands every photo"
)]
pub(crate) fn import_in(
    import: u64,
    photos: &ImportPhotos,
    settings: &ImportSettings,
    build: &str,
    how: Lands<'_>,
    observer: &(dyn Fn(LibraryEvent) + Send + Sync),
    step: &dyn Fn(ImportStep<'_>),
    cancel: &(dyn Fn() -> bool + Sync),
) -> ImportEnd {
    let files = photos.in_order();
    let total = files.len();
    let mut end = ImportEnd {
        total,
        ..ImportEnd::default()
    };
    let refuse = |mut end: ImportEnd, cause, reason: String| {
        tracing::debug!(target: "control", "import refused: {reason}");
        end.not_reached = end.total;
        end.refused = Some(ImportFailure {
            cause,
            photo: None,
            reason,
        });
        end
    };
    let patterns = match Patterns::parse(settings) {
        Ok(patterns) => patterns,
        Err(errors) => {
            let reasons: Vec<String> = (errors.iter())
                .map(|(row, error)| format!("{row:?}: {error}"))
                .collect();
            return refuse(end, ImportCause::Pattern, reasons.join("; "));
        }
    };
    let now = how.world.now;
    let not_reached = |mut end: ImportEnd, from: usize, cancelled: bool| {
        end.not_reached = total - from;
        end.cancelled = cancelled;
        if cancelled {
            end.left.clear();
        } else {
            end.left.extend(files[from..].iter().cloned());
        }
        end
    };
    if !photos.folder.is_dir() {
        end.failure = Some(ImportFailure {
            cause: ImportCause::Gone,
            photo: None,
            reason: format!("{} is not there", photos.folder.display()),
        });
        return not_reached(end, 0, false);
    }
    let Some(shots) = preview::shots(&photos.folder, &files, &patterns, now, cancel) else {
        return not_reached(end, 0, true);
    };
    let folders: Result<Vec<PathBuf>, String> = shots
        .iter()
        .map(|shot| patterns.destination(now, shot))
        .collect();
    let folders = match folders {
        Ok(folders) => folders,
        Err(subfolder) => {
            let reason = format!("Subfolder: {subfolder} leads out of Folder");
            return refuse(end, ImportCause::Pattern, reason);
        }
    };
    let mut distinct: Vec<&Path> = folders.iter().map(PathBuf::as_path).collect();
    distinct.dedup();
    // A client that skipped the preview's problems.
    if let Some(folder) = distinct.iter().find(|folder| !preview::full(folder)) {
        let reason = format!("{}: no absolute folder", folder.display());
        return refuse(end, ImportCause::Folder, reason);
    }
    if let Some((folder, why)) = preview::on_source(&photos.folder, &distinct, how.world) {
        let reason = format!(
            "{}: {why:?} of {}",
            folder.display(),
            photos.folder.display()
        );
        return refuse(end, ImportCause::OnSource, reason);
    }
    end.destination = common(&folders);
    if let Some(destination) = &end.destination {
        step(ImportStep::Destination(destination));
    }
    // The deepest folder of each destination there as the import starts, which a write
    // error after which it can no longer be listed tells gone: never a folder above it, such
    // as `/Volumes`, which stays when its disk is unplugged.
    let anchors: Vec<Option<PathBuf>> = (folders.iter())
        .map(|folder| crate::destination::deepest_existing(folder).map(Path::to_owned))
        .collect();
    let mut made = Vec::new();
    // Each photo told as it ends, never held behind the next photo's copy.
    let tell = |photo: ImportedPhoto| {
        observer(LibraryEvent::Imported {
            open: import,
            photos: vec![photo],
        });
    };
    let Lands { land, before, .. } = how;
    for (index, ((file, into), anchor)) in files.iter().zip(&folders).zip(&anchors).enumerate() {
        if cancel() {
            end = not_reached(end, index, true);
            break;
        }
        before(index);
        let name = (file.file_name()).map_or_else(String::new, |n| n.to_string_lossy().into());
        step(ImportStep::Photo {
            done: index,
            total,
            name: &name,
        });
        let path = photos.folder.join(file);
        let at = Bound {
            folder: into,
            anchor: anchor.as_deref(),
        };
        let outcome = photo(&path, &photos.folder, at, build, land, &mut made, cancel);
        let result = match outcome {
            Outcome::Cancelled => {
                end = not_reached(end, index, true);
                break;
            }
            Outcome::Done(result) => result,
            Outcome::Stop(cause, reason) => {
                end.failed += 1;
                end.failure = Some(ImportFailure {
                    cause,
                    photo: Some(file.clone()),
                    reason: reason.clone(),
                });
                end.left.push(file.clone());
                tell(ImportedPhoto {
                    source: file.clone(),
                    result: ImportResult::Failed { cause, reason },
                });
                let rest = total - index - 1;
                tracing::debug!(target: "control", "import stopped at {name}, {rest} not reached");
                end = not_reached(end, index + 1, false);
                break;
            }
        };
        match &result {
            ImportResult::Copied { .. } => {
                end.copied += 1;
                end.landed = true;
            }
            ImportResult::There { .. } => end.there += 1,
            ImportResult::Diverged { .. } => end.diverged += 1,
            ImportResult::Failed { cause, reason } => {
                end.failed += 1;
                end.left.push(file.clone());
                end.failure.get_or_insert_with(|| ImportFailure {
                    cause: *cause,
                    photo: Some(file.clone()),
                    reason: reason.clone(),
                });
            }
        }
        tell(ImportedPhoto {
            source: file.clone(),
            result,
        });
        step(ImportStep::Done {
            done: index + 1,
            total,
        });
    }
    remove_empty(&mut made);
    end
}

/// Where a photo goes: its destination folder, and the deepest folder of it that was there
/// as the import started.
#[derive(Clone, Copy)]
struct Bound<'a> {
    folder: &'a Path,
    anchor: Option<&'a Path>,
}

/// The photo at `path` of the source `root` imported into `at`.
fn photo(
    path: &Path,
    root: &Path,
    at: Bound<'_>,
    build: &str,
    land: &mut Land<'_>,
    made: &mut Vec<PathBuf>,
    cancel: &(dyn Fn() -> bool + Sync),
) -> Outcome {
    let failed = |cause, error: &dyn std::fmt::Display| {
        Outcome::Done(ImportResult::Failed {
            cause,
            reason: error.to_string(),
        })
    };
    let unreadable = |error: std::io::Error| {
        let error = CopyError::Read {
            file: path.to_owned(),
            source: error,
        };
        judge(error, root, at)
    };
    let into = at.folder;
    let (Some(folder), Some(main)) = (path.parent(), path.file_name()) else {
        return failed(ImportCause::Unreadable, &"no file name");
    };
    let main = main.to_string_lossy();
    let source = match PhotoFiles::find(folder, &main) {
        Ok(source) => source,
        Err(error) => return unreadable(error),
    };
    let size = match std::fs::metadata(path) {
        Ok(meta) => meta.len(),
        Err(error) => return unreadable(error),
    };
    let mut same = |there: &Path| -> Result<bool, CopyError> {
        let same_size = std::fs::metadata(there).is_ok_and(|meta| meta.len() == size);
        Ok(same_size && copy::same_file(path, there, cancel)?)
    };
    let place = match plan::place(into, &source.main, &Default::default(), &mut same) {
        Ok(place) => place,
        Err(error) => return judge(error, root, at),
    };
    let name = match &place {
        Place::Free(name) | Place::There(name) => name.clone(),
    };
    let sidecar = match plan::renamed_sidecar(&source, &name, build) {
        Ok(sidecar) => sidecar,
        Err(error) => return unreadable(error),
    };
    let copy = into.join(&name);
    let landed = match place {
        Place::Free(_) => {
            let landing = Landing {
                folder: into,
                main: &name,
                sidecar: sidecar.as_deref(),
                main_file: true,
            };
            land(&source, &landing, made).map(|_| ImportResult::Copied { copy })
        }
        Place::There(_) => {
            let beside = match plan::beside(&source, into, &name, sidecar.as_deref()) {
                Ok(beside) => beside,
                Err(error) => return unreadable(error),
            };
            if let Some(sidecar) = beside.diverged {
                return Outcome::Done(ImportResult::Diverged { copy, sidecar });
            }
            let alone = PhotoFiles {
                companions: beside.missing,
                ..source.clone()
            };
            let landing = Landing {
                folder: into,
                main: &name,
                sidecar: sidecar.as_deref().filter(|_| beside.sidecar_missing),
                main_file: false,
            };
            let lands = !alone.companions.is_empty() || landing.sidecar.is_some();
            let landed = if lands {
                land(&alone, &landing, made).map(|_| ())
            } else {
                Ok(())
            };
            landed.map(|()| ImportResult::There { copy })
        }
    };
    match landed {
        Ok(result) => Outcome::Done(result),
        Err(error) => judge(error, root, at),
    }
}

/// What a copy's failure tells: a cancel; the source failing, a read error after which the
/// source's folder `root` can no longer be listed; the destination failing, a write error
/// for lack of space or after which the deepest folder of `at` there as the import started
/// can no longer be listed; else the photo's own failure.
fn judge(error: CopyError, root: &Path, at: Bound<'_>) -> Outcome {
    let listed = |folder: &Path| std::fs::read_dir(folder).is_ok();
    let reason = error.to_string();
    match &error {
        CopyError::Cancelled { .. } => Outcome::Cancelled,
        CopyError::Read { .. } if !listed(root) => Outcome::Stop(ImportCause::Gone, reason),
        CopyError::Read { .. } => Outcome::Done(ImportResult::Failed {
            cause: ImportCause::Unreadable,
            reason,
        }),
        CopyError::Write { source, .. } if full(source) => {
            Outcome::Stop(ImportCause::DiskFull, reason)
        }
        CopyError::Write { .. } if !at.anchor.is_some_and(listed) => {
            Outcome::Stop(ImportCause::Gone, reason)
        }
        CopyError::Exists { .. } => Outcome::Done(ImportResult::Failed {
            cause: ImportCause::Exists,
            reason,
        }),
        CopyError::Write { .. } | CopyError::Mismatch { .. } => {
            Outcome::Done(ImportResult::Failed {
                cause: ImportCause::Write,
                reason,
            })
        }
    }
}

/// Whether `error` says the disk is full or over its quota.
fn full(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::StorageFull | std::io::ErrorKind::QuotaExceeded
    )
}

/// The deepest folder all of `folders` lie in: the one folder where they are all one.
fn common(folders: &[PathBuf]) -> Option<PathBuf> {
    let (first, rest) = folders.split_first()?;
    let mut common = first.clone();
    for folder in rest {
        while !folder.starts_with(&common) {
            if !common.pop() {
                return None;
            }
        }
    }
    Some(common)
}

/// Removes each folder of `made` left empty, deepest first: one holding anything stays.
fn remove_empty(made: &mut [PathBuf]) {
    made.sort_by_key(|folder| std::cmp::Reverse(folder.components().count()));
    for folder in made.iter() {
        let empty = std::fs::read_dir(folder).is_ok_and(|mut entries| entries.next().is_none());
        if empty && let Err(error) = std::fs::remove_dir(folder) {
            tracing::warn!(
                "{}: made, left empty, not removed: {error}",
                folder.display()
            );
        }
    }
}

#[cfg(test)]
mod tests;
