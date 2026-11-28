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

//! Import's preview (decision 85; `docs/library.md`, Import in folder mode, The preview):
//! where each checked photo would go and what stands in the way, before anything is
//! copied, which the Import tool shows in its rows' tooltips and warning buttons and
//! `apertrust-cli import --dry-run` prints. It writes nothing and makes nothing, on the
//! source or the destination: a folder that is not there is asked about through the
//! deepest folder of its path that is.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use apertrust_base::variables::{DateTime, PatternError};
use apertrust_base::wire::Id;
use serde::{Deserialize, Serialize};

use super::plan::{self, Place};
use super::{Ended, Fingerprints, LibraryEvent, SourceState, Trees, confirm, raw_state};
use crate::copy::PhotoFiles;
use crate::destination::{self, Patterns, Row, Shot, Uses};
use crate::imported::{self, Sought, Walked};
use crate::index::READERS;
use crate::listing::is_raw;
use crate::settings::ImportSettings;
use crate::source::{self, Listed, SourceFile};
use crate::volumes::FreeSpace;

/// The photos an import takes: the source's folder listed, the card's `DCIM` or the folder,
/// and the checked files by their paths relative to it, as the listing names them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportPhotos {
    pub folder: PathBuf,
    pub files: Vec<PathBuf>,
}

impl ImportPhotos {
    /// Its files in the order the copy takes them, the order of their paths, as darktable's
    /// import job sorts them (`_sort_filename`), each once; a path that is absolute or holds
    /// a `..`, which could lead out of the folder, left out and logged.
    pub fn in_order(&self) -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = (self.files.iter())
            .filter(|file| {
                let inside = super::relative_inside(file);
                if !inside {
                    tracing::warn!("{}: not inside the source; not imported", file.display());
                }
                inside
            })
            .cloned()
            .collect();
        files.sort_unstable();
        files.dedup();
        files
    }
}

/// What the Import tool shows for the latest photos and settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportPreview {
    /// Each checked photo, in the order the copy takes them.
    pub photos: Vec<PhotoPreview>,
    /// The folders the patterns make, in the order the photos first reach them.
    pub folders: Vec<FolderPreview>,
    /// What Folder's pattern and Subfolder's read: today's date, or the day taken.
    pub folder_uses: Uses,
    pub subfolder_uses: Uses,
    /// How many of the photos to copy were imported before, by the check against Folder's
    /// and the default folder's trees, and will be copied again: those to be skipped as
    /// already at their destination left out.
    pub imported_before: usize,
    /// The bytes the copy may write: the photos' files, those of a photo to be skipped if
    /// identical among them, which the copy writes under the next `_01` name where the
    /// whole files differ.
    pub size: u64,
    /// The free space of the destination's volume, where every destination is on one,
    /// read at the deepest folder of each that is there; `None` where it cannot be read or
    /// they are on several.
    pub free: Option<FreeSpace>,
    /// What stops the import, each with its row and its fix.
    pub problems: Vec<ImportProblem>,
}

impl ImportPreview {
    /// How many photos have each fate: copied under their names, copied with `_01` added,
    /// to be skipped as already there, and skipped with an edit of their own there.
    pub fn counts(&self) -> FateCounts {
        let mut counts = FateCounts::default();
        for photo in &self.photos {
            match &photo.fate {
                ImportFate::Copy { renamed: false, .. } => counts.copied += 1,
                ImportFate::Copy { renamed: true, .. } => {
                    counts.copied += 1;
                    counts.renamed += 1;
                }
                ImportFate::There { .. } => counts.there += 1,
                ImportFate::Diverged { .. } => counts.diverged += 1,
                ImportFate::Unreadable { .. } => counts.unreadable += 1,
            }
        }
        counts
    }
}

/// How many photos of a preview have each fate ([`ImportPreview::counts`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FateCounts {
    /// To copy, those `renamed` with `_01` added among them.
    pub copied: usize,
    pub renamed: usize,
    pub there: usize,
    pub diverged: usize,
    pub unreadable: usize,
}

/// One checked photo of the preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhotoPreview {
    /// Its path relative to the source's folder, as the listing names it.
    pub source: PathBuf,
    pub fate: ImportFate,
}

/// What the copy would do with a photo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ImportFate {
    /// Copied to `to`, under its camera name, or `renamed` with `_01` and on added where the
    /// name is taken.
    Copy { to: PathBuf, renamed: bool },
    /// A file of its size is at `at`, under its name or one of its `_01` names: to be
    /// skipped if identical, which the copy confirms by comparing the whole files, its
    /// missing sidecars copied alone.
    There { at: PathBuf },
    /// A file of its size is at `at` with a sidecar of its own that differs from the
    /// source's, `sidecar`: skipped if identical, its sidecar not copied, its edit on the
    /// source left there.
    Diverged { at: PathBuf, sidecar: PathBuf },
    /// Its files could not be read, in plain words.
    Unreadable { reason: String },
}

/// A folder the patterns make.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderPreview {
    pub folder: PathBuf,
    /// How many of the checked photos go into it.
    pub photos: usize,
    /// How many raws it holds, where it is there already; `None` for a new folder.
    pub there: Option<usize>,
}

/// What stops an import, on the Import tool's `row`, with its `fix`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportProblem {
    pub row: Row,
    pub problem: Problem,
    pub fix: Option<Fix>,
}

/// What stops an import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Problem {
    /// The row's pattern cannot be read (D8): an unknown variable, with the nearest name,
    /// one not closed, a character that is no operation, variables nested too deep.
    Pattern(PatternError),
    /// Subfolder expands for a photo to `subfolder`, which holds a `..`, a root or a drive
    /// and so would lead out of Folder or replace it.
    OutsideFolder { subfolder: String },
    /// A destination folder that cannot be made or written, found at `at`, the deepest
    /// folder of its path that is there, or the name in the way.
    Unwritable { at: PathBuf, why: Unwritable },
    /// A destination that is the source, lies inside it, or is anywhere on a card the
    /// source is on: `folder`, the destination folder.
    OnSource { folder: PathBuf, why: OnSource },
    /// Not enough space: `needed` bytes, `available` free on `volume`.
    Space {
        needed: u64,
        available: u64,
        volume: String,
    },
}

/// Why a destination folder cannot be made or written, asked without making it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Unwritable {
    /// Folder is no absolute folder.
    NotAbsolute,
    /// Not even its volume's root is there: a disk not connected.
    NotConnected,
    /// A file, or a link that leads nowhere, is where a folder is wanted.
    NotAFolder,
    /// The deepest folder there has no write permission for anyone.
    ReadOnly,
    /// The system would not say, for this reason.
    #[serde(with = "apertrust_base::wire::error_kind")]
    Unreadable(io::ErrorKind),
}

/// How a destination is on the source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum OnSource {
    /// It is the source's folder.
    IsSource,
    /// It lies inside it.
    InsideSource,
    /// It is on the card the source is on, mounted at `card`.
    OnCard { card: PathBuf },
}

/// A problem's fix, which its warning button's menu holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Fix {
    /// Use `$(variable)` in place of the name typed, at the place the problem's
    /// [`PatternError::Unknown`] gives.
    Use {
        #[serde(deserialize_with = "apertrust_base::wire::id")]
        variable: Id,
    },
    /// Choose a folder…
    ChooseFolder,
}

/// What the preview finds out about the world beside the files: the import's time, a
/// folder's mount point (the card rule's, [`source::mount_point`]) and a path's free
/// space ([`crate::volumes::free_space`]), a test's own in their place.
pub(crate) struct World<'a> {
    pub now: DateTime,
    pub mount: &'a (dyn Fn(&Path) -> Option<PathBuf> + Sync),
    pub free_space: &'a (dyn Fn(&Path) -> Option<FreeSpace> + Sync),
}

/// Previews the import of `photos` with `settings` for the command numbered `preview`, the
/// import's time `now`, telling `observer` once with [`LibraryEvent::ImportPreview`]: each
/// photo's destination and fate, the folders, the dates the patterns read, how many were
/// imported before (the check, its fingerprints through the session's memo), the size, the
/// free space and the problems. A photo's EXIF is read only where a pattern reads its
/// capture time or camera, [`READERS`] at a time, a cloud file not on the disk not at all.
/// A blocking call, made by a job's body; once `cancel` answers true it returns and sends
/// nothing. Nothing is written or made.
pub fn preview_import(
    preview: u64,
    photos: &ImportPhotos,
    settings: &ImportSettings,
    now: DateTime,
    fingerprints: &Arc<Fingerprints>,
    observer: &(dyn Fn(LibraryEvent) + Send + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Ended {
    let world = World {
        now,
        mount: &source::mount_point,
        free_space: &crate::volumes::free_space,
    };
    let Some(found) = preview_in(photos, settings, &world, fingerprints, cancel) else {
        return Ended::Cancelled;
    };
    if cancel() {
        return Ended::Cancelled;
    }
    observer(LibraryEvent::ImportPreview {
        open: preview,
        preview: Box::new(found),
    });
    Ended::Listed
}

/// The preview of `photos` with `settings` in `world`; `None` once `cancel` answers true.
pub(crate) fn preview_in(
    photos: &ImportPhotos,
    settings: &ImportSettings,
    world: &World<'_>,
    fingerprints: &Arc<Fingerprints>,
    cancel: &(dyn Fn() -> bool + Sync),
) -> Option<ImportPreview> {
    let patterns = match Patterns::parse(settings) {
        Ok(patterns) => patterns,
        Err(errors) => {
            return Some(ImportPreview {
                problems: errors.into_iter().map(pattern_problem).collect(),
                ..ImportPreview::default()
            });
        }
    };
    let (folder_uses, subfolder_uses) = patterns.uses();
    let files = photos.in_order();
    let shots = shots(&photos.folder, &files, &patterns, world.now, cancel)?;
    let destinations: Result<Vec<PathBuf>, String> = (shots.iter())
        .map(|shot| patterns.destination(world.now, shot))
        .collect();
    let destinations = match destinations {
        Ok(destinations) => destinations,
        Err(subfolder) => {
            return Some(ImportPreview {
                problems: vec![ImportProblem {
                    row: Row::Subfolder,
                    problem: Problem::OutsideFolder { subfolder },
                    fix: None,
                }],
                ..ImportPreview::default()
            });
        }
    };
    let mut preview = ImportPreview {
        folder_uses,
        subfolder_uses,
        ..ImportPreview::default()
    };
    let mut claimed = HashSet::new();
    let mut claimed_sizes = HashMap::new();
    let mut folders: BTreeMap<PathBuf, usize> = BTreeMap::new();
    let mut order = Vec::new();
    let mut copied = Vec::new();
    for (file, into) in files.iter().zip(destinations) {
        if cancel() {
            return None;
        }
        let fate = fate(
            &photos.folder.join(file),
            &into,
            &mut claimed,
            &mut claimed_sizes,
            &mut preview.size,
        );
        if !folders.contains_key(&into) {
            order.push(into.clone());
        }
        *folders.entry(into).or_default() += 1;
        if matches!(fate, ImportFate::Copy { .. }) {
            copied.push(file.clone());
        }
        preview.photos.push(PhotoPreview {
            source: file.clone(),
            fate,
        });
    }
    preview.folders = (order.into_iter())
        .map(|folder| FolderPreview {
            photos: folders[&folder],
            there: raws_in(&folder),
            folder,
        })
        .collect();
    preview.imported_before = imported_before(photos, &copied, settings, fingerprints, cancel)?;
    preview.free = free(&preview.folders, world);
    preview.problems = problems(&photos.folder, &preview, world);
    Some(preview)
}

/// The free space of the one volume every folder of `folders` is on, asked at the deepest
/// folder of each that is there; `None` where it cannot be read for one, or they are on
/// several, where the copy's own stop on a full disk stands in for the preview's check.
fn free(folders: &[FolderPreview], world: &World<'_>) -> Option<FreeSpace> {
    let mut there: Vec<&Path> = (folders.iter())
        .filter_map(|folder| destination::deepest_existing(&folder.folder))
        .collect();
    there.sort_unstable();
    there.dedup();
    let (first, rest) = there.split_first()?;
    let free = (world.free_space)(first)?;
    let same = |other: &FreeSpace| other.volume == free.volume && other.total == free.total;
    (rest.iter())
        .all(|folder| (world.free_space)(folder).is_some_and(|other| same(&other)))
        .then_some(free)
}

/// A pattern's error on `row` as a problem, with the nearest name to use where there is one.
fn pattern_problem((row, error): (Row, PatternError)) -> ImportProblem {
    let fix = match error {
        PatternError::Unknown {
            suggestion: Some(variable),
            ..
        } => Some(Fix::Use { variable }),
        _ => None,
    };
    ImportProblem {
        row,
        problem: Problem::Pattern(error),
        fix,
    }
}

/// Each photo's [`Shot`] for the patterns, in the order of `files`: its name alone where no
/// pattern reads its capture time or camera ([`Patterns::reads_exif`]), else its EXIF's capture time, make and model, [`READERS`] at a
/// time, its file's modified time where its EXIF holds no time, the import's where it has
/// neither ([`destination::taken`]); a cloud file not on the disk's EXIF never read. `None`
/// once `cancel` answers true.
pub(crate) fn shots(
    folder: &Path,
    files: &[PathBuf],
    patterns: &Patterns,
    now: DateTime,
    cancel: &(dyn Fn() -> bool + Sync),
) -> Option<Vec<Shot>> {
    let name = |file: &Path| {
        (file.file_name()).map_or_else(String::new, |name| name.to_string_lossy().into_owned())
    };
    if !patterns.reads_exif() {
        return Some(
            (files.iter())
                .map(|file| Shot {
                    name: name(file),
                    ..Shot::default()
                })
                .collect(),
        );
    }
    let paths = files.iter().map(|file| folder.join(file)).collect();
    let (read, stopped) = crate::reads::read(
        paths,
        READERS,
        move |path| {
            let modified = fs::metadata(path).and_then(|meta| meta.modified()).ok();
            let exif = (raw_state(path).is_some_and(|state| state.local))
                .then(|| {
                    #[cfg(any(test, feature = "testing"))]
                    crate::testing::read(path);
                    std::panic::catch_unwind(|| apertrust_raw::read_exif_file(path).ok())
                        .ok()
                        .flatten()
                })
                .flatten();
            (exif, modified)
        },
        cancel,
    );
    if stopped {
        return None;
    }
    let mut shots: Vec<Shot> = files
        .iter()
        .map(|file| Shot {
            name: name(file),
            taken: Some(now),
            ..Shot::default()
        })
        .collect();
    for (at, (exif, modified)) in read {
        let shot = &mut shots[at];
        let exif = exif.unwrap_or_default();
        shot.taken = Some(destination::taken(exif.taken, modified, now));
        shot.maker = exif.make_written.unwrap_or_default();
        shot.model = exif.model_written.unwrap_or_default();
    }
    Some(shots)
}

/// What the copy would do with the photo at `path` going into `into`, `claimed` holding the
/// names the photos before it take and `claimed_sizes` their main files' sizes, which it
/// adds its own to, and `size` the bytes to copy, which it adds its own to, whether a copy
/// there is found or not: a copy under the first name free, or its copy there, found by a
/// file of its size on the way, or by a photo of its size before it claiming the name, whose
/// copy the job finds there.
fn fate(
    path: &Path,
    into: &Path,
    claimed: &mut HashSet<PathBuf>,
    claimed_sizes: &mut HashMap<PathBuf, u64>,
    size: &mut u64,
) -> ImportFate {
    let unreadable = |error: io::Error| ImportFate::Unreadable {
        reason: format!("cannot read {}: {error}", path.display()),
    };
    let (Some(folder), Some(main)) = (path.parent(), path.file_name()) else {
        return unreadable(io::ErrorKind::InvalidInput.into());
    };
    let main = main.to_string_lossy();
    let source = match PhotoFiles::find(folder, &main) {
        Ok(source) => source,
        Err(error) => return unreadable(error),
    };
    let length = |name: &str| fs::metadata(folder.join(name)).map(|meta| meta.len());
    let raw = match length(&source.main) {
        Ok(raw) => raw,
        Err(error) => return unreadable(error),
    };
    let mut same = |there: &Path| {
        Ok(match claimed_sizes.get(there) {
            Some(&claimed) => claimed == raw,
            None => fs::metadata(there).is_ok_and(|meta| meta.len() == raw),
        })
    };
    let place = match plan::place(into, &source.main, claimed, &mut same) {
        Ok(place) => place,
        Err(error) => {
            return ImportFate::Unreadable {
                reason: error.to_string(),
            };
        }
    };
    // The build only names a new sidecar, never compared, so the preview's is any.
    let companions = |source: &PhotoFiles, names: &[String]| -> u64 {
        (names.iter())
            .filter(|name| source.companions.contains(name))
            .filter_map(|name| length(name).ok())
            .sum()
    };
    match place {
        Place::Free(name) => {
            *size += raw + companions(&source, &source.companions);
            claimed.extend(plan::names(&name).map(|name| into.join(name)));
            claimed_sizes.insert(into.join(&name), raw);
            ImportFate::Copy {
                renamed: name != source.main,
                to: into.join(name),
            }
        }
        Place::There(name) => {
            let sidecar = match plan::renamed_sidecar(&source, &name, "") {
                Ok(sidecar) => sidecar,
                Err(error) => return unreadable(error),
            };
            let beside = match plan::beside(&source, into, &name, sidecar.as_deref()) {
                Ok(beside) => beside,
                Err(error) => return unreadable(error),
            };
            // Found by its size alone: the copy compares the whole files and writes the
            // photo under the next `_01` name where they differ, so its files are counted.
            *size += raw + companions(&source, &source.companions);
            let at = into.join(&name);
            match beside.diverged {
                Some(sidecar) => ImportFate::Diverged { at, sidecar },
                None => ImportFate::There { at },
            }
        }
    }
}

/// How many raws the folder holds, where it is there; `None` for one that is not.
fn raws_in(folder: &Path) -> Option<usize> {
    let entries = fs::read_dir(folder).ok()?;
    Some(
        entries
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
            .filter(|entry| entry.file_name().to_str().is_some_and(is_raw))
            .count(),
    )
}

/// How many of the photos `copied`, of the source's `photos`, were imported before, by the
/// check against `settings`' trees ([`imported::trees`]), the source's own folders left
/// out; `None` once `cancel` answers true.
fn imported_before(
    photos: &ImportPhotos,
    copied: &[PathBuf],
    settings: &ImportSettings,
    fingerprints: &Arc<Fingerprints>,
    cancel: &(dyn Fn() -> bool + Sync),
) -> Option<usize> {
    if copied.is_empty() {
        return Some(0);
    }
    let listed = listed(photos, copied);
    let trees = Trees {
        base_folder: settings.base_folder.clone(),
        default_folder: settings.default_folder.clone(),
        places: settings.places.clone(),
    };
    let states = check(&listed, &trees, fingerprints, cancel)?;
    Some(
        (states.iter())
            .filter(|state| !matches!(state, SourceState::New))
            .count(),
    )
}

/// The photos `files` of the source `photos` as a listing would give them, for the check:
/// each measured, the folders holding them and the source's own as the folders listed.
pub(crate) fn listed(photos: &ImportPhotos, files: &[PathBuf]) -> Listed {
    let mut folders = vec![photos.folder.clone()];
    let mut seen: HashSet<PathBuf> = folders.iter().cloned().collect();
    let files = (files.iter())
        .map(|file| {
            let path = photos.folder.join(file);
            if let Some(parent) = path.parent()
                && seen.insert(parent.to_owned())
            {
                folders.push(parent.to_owned());
            }
            let meta = fs::metadata(&path).ok();
            SourceFile {
                path: file.clone(),
                size: meta.as_ref().map_or(0, fs::Metadata::len),
                modified: meta.as_ref().and_then(|meta| meta.modified().ok()),
                local: raw_state(&path).is_some_and(|state| state.local),
            }
        })
        .collect();
    Listed {
        folder: photos.folder.clone(),
        files,
        folders,
        left_out: source::LeftOut::default(),
    }
}

/// Each of `listed`'s photos told new or imported before against `trees`, as the source's
/// check tells it ([`super::check_source`]), in one go; `None` once `cancel` answers true.
pub(crate) fn check(
    listed: &Listed,
    trees: &Trees,
    fingerprints: &Arc<Fingerprints>,
    cancel: &(dyn Fn() -> bool + Sync),
) -> Option<Vec<SourceState>> {
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
            Walked::Cancelled => return None,
            // Every photo new, as a tree that is not there tells it, as the source's check does.
            Walked::Failed => {
                let walked: Vec<_> = folders.iter().map(|folder| folder.display()).collect();
                tracing::warn!("the check against {walked:?} failed: every photo told new");
                vec![Vec::new(); of]
            }
        };
    confirm(listed, &found, 0..of, fingerprints, cancel)
}

/// What stops the import of the source `source` as `preview` foresees it, in `world`: the
/// folders that cannot be made or written, each once, then a destination on the source,
/// then too little space.
pub(crate) fn problems(
    source: &Path,
    preview: &ImportPreview,
    world: &World<'_>,
) -> Vec<ImportProblem> {
    let mut problems = Vec::new();
    let folder = |problem| ImportProblem {
        row: Row::Folder,
        problem,
        fix: Some(Fix::ChooseFolder),
    };
    let mut blocked = HashSet::new();
    for destination in &preview.folders {
        if let Some((at, why)) = unwritable(&destination.folder)
            && blocked.insert(at.clone())
        {
            problems.push(folder(Problem::Unwritable { at, why }));
        }
    }
    let folders: Vec<&Path> = (preview.folders.iter())
        .map(|folder| folder.folder.as_path())
        .collect();
    if let Some((folder_on, why)) = on_source(source, &folders, world) {
        problems.push(folder(Problem::OnSource {
            folder: folder_on,
            why,
        }));
    }
    if let Some(free) = &preview.free
        && free.available < preview.size
    {
        problems.push(folder(Problem::Space {
            needed: preview.size,
            available: free.available,
            volume: free.volume.clone(),
        }));
    }
    problems
}

/// Where and why `folder`, which the copy makes if it is missing, cannot be made or
/// written, as the export's preflight asks: the deepest folder of its path that is there
/// must be a folder and writable. Metadata alone, so it makes nothing, and it advises: it
/// sees the permission bits, not an access control list or a read-only mount, and the copy
/// reports what actually fails.
fn unwritable(folder: &Path) -> Option<(PathBuf, Unwritable)> {
    if !full(folder) {
        return Some((folder.to_owned(), Unwritable::NotAbsolute));
    }
    let mut at = folder;
    let metadata = loop {
        match fs::metadata(at) {
            Ok(metadata) => break metadata,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                ) =>
            {
                if fs::symlink_metadata(at).is_ok() {
                    return Some((at.to_owned(), Unwritable::NotAFolder));
                }
                match at.parent() {
                    Some(parent) => at = parent,
                    None => return Some((at.to_owned(), Unwritable::NotConnected)),
                }
            }
            Err(error) => return Some((at.to_owned(), Unwritable::Unreadable(error.kind()))),
        }
    };
    if !metadata.is_dir() {
        return Some((at.to_owned(), Unwritable::NotAFolder));
    }
    (!writable(&metadata)).then(|| (at.to_owned(), Unwritable::ReadOnly))
}

/// Whether anyone may write the folder `metadata` describes: its write bits on Unix. On
/// Windows a folder's read-only attribute means something else, so it is not asked.
fn writable(metadata: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o222 != 0
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        true
    }
}

/// The first of `folders` that is the source `source`, lies inside it, or is on a card the
/// source is on, by the card rule ([`source::card_at`]) on `world`'s mount point, each
/// compared as the system resolves it, the part not there yet as written.
pub(crate) fn on_source(
    source: &Path,
    folders: &[&Path],
    world: &World<'_>,
) -> Option<(PathBuf, OnSource)> {
    let card = (world.mount)(source)
        .and_then(|mount| source::card_at(source, &mount))
        .map(|card| (resolved(&card.mount), card.mount));
    let source = resolved(source);
    folders.iter().find_map(|&folder| {
        let resolved = resolved(folder);
        let why = if resolved == source {
            OnSource::IsSource
        } else if resolved.starts_with(&source) {
            OnSource::InsideSource
        } else {
            match &card {
                Some((mount, card)) if resolved.starts_with(mount) => {
                    OnSource::OnCard { card: card.clone() }
                }
                _ => return None,
            }
        };
        Some((folder.to_owned(), why))
    })
}

/// Whether `folder` is absolute and holds no `..`, which, past a folder not there yet,
/// the source check could not follow.
pub(super) fn full(folder: &Path) -> bool {
    folder.is_absolute()
        && !folder
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
}

/// `path` as the system resolves the deepest folder of it that is there, the rest after it
/// as written.
fn resolved(path: &Path) -> PathBuf {
    let Some(there) = destination::deepest_existing(path) else {
        return path.to_owned();
    };
    let rest = path.strip_prefix(there).unwrap_or(Path::new(""));
    match fs::canonicalize(there) {
        Ok(real) if rest.as_os_str().is_empty() => real,
        Ok(real) => real.join(rest),
        Err(_) => path.to_owned(),
    }
}

#[cfg(test)]
mod tests;
