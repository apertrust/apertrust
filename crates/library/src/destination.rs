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

//! Where Import puts a photo (decision 85; `docs/library.md`, Import in folder mode): Folder,
//! then Subfolder worked out for that photo, then its camera name, as darktable's import
//! session builds its path (`src/common/import_session.c`, `_import_session_path`) from
//! `session/base_directory_pattern` and `session/sub_directory_pattern`.
//!
//! The patterns expand with darktable's variables ([`apertrust_base::variables`]) as its
//! import expands them for each photo: the job code is Import name (`$(IMPORT_NAME)`, and
//! darktable's `$(JOBCODE)`); `$(YEAR)`, `$(MONTH)`, `$(DAY)` and their kin the import's
//! time, local, taken once as the import starts; `$(EXIF.YEAR)` and its kin the photo's
//! capture time, the camera's clock, or where its EXIF holds none, its file's modified time
//! on the local clock, as darktable's `_control_import_image_copy` takes it, and the
//! import's time where the file has none either, as darktable's variables fall back to it;
//! `$(MAKER)` and `$(MODEL)` the camera's as its EXIF writes them; `$(FILE_NAME)` and its
//! kin the photo's file name; `$(HOME)`, `$(PICTURES_FOLDER)` and the person's other folders
//! the client's [`Places`]. A leading `~` in Folder is the home folder, as the export's
//! path takes it; an empty Subfolder puts the photos straight into the folder.

use std::path::{Component, Path, PathBuf, is_separator};
use std::time::SystemTime;

use apertrust_base::variables::{DateTime, Pattern, PatternError, Places, Values};
use chrono::{Datelike, Timelike};
use serde::{Deserialize, Serialize};

use crate::settings::ImportSettings;

/// The Import tool's row a pattern is in, which a problem names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Row {
    /// Folder, `import.base_folder`.
    Folder,
    /// Subfolder, `import.subfolder`.
    Subfolder,
}

/// What the patterns read of one photo.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shot {
    /// Its file name, its camera name, which `$(FILE_NAME)` and its kin read.
    pub name: String,
    /// When it was taken ([`taken`]), which `$(EXIF.YEAR)` and its kin read.
    pub taken: Option<DateTime>,
    /// The camera's make and model as its EXIF writes them (`$(MAKER)`, `$(MODEL)`).
    pub maker: String,
    pub model: String,
}

/// What a pattern reads beside the places and the import name: today's date, the import's
/// time, and the photo, its capture time or its camera and name.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Uses {
    /// The import's time, `$(YEAR)` and its kin.
    pub today: bool,
    /// The photo's capture time, `$(EXIF.YEAR)` and its kin.
    pub taken: bool,
    /// The photo's camera or file name, `$(MODEL)` or `$(FILE_NAME)`.
    pub photo: bool,
}

/// Folder's and Subfolder's patterns, read whole, with what they expand from.
#[derive(Debug, Clone, PartialEq)]
pub struct Patterns {
    folder: Pattern,
    /// `None` where Subfolder is empty.
    subfolder: Option<Pattern>,
    job_code: String,
    places: Places,
}

impl Patterns {
    /// Folder's and Subfolder's patterns of `settings`, or what stops each, Folder's first:
    /// an unknown variable with the nearest name, one not closed (D8).
    pub fn parse(settings: &ImportSettings) -> Result<Self, Vec<(Row, PatternError)>> {
        let folder = Pattern::parse(&settings.base_folder);
        let subfolder = match settings.subfolder.trim() {
            "" => Ok(None),
            _ => Pattern::parse(&settings.subfolder).map(Some),
        };
        match (folder, subfolder) {
            (Ok(folder), Ok(subfolder)) => Ok(Patterns {
                folder,
                subfolder,
                job_code: settings.job_code.clone(),
                places: settings.places.clone(),
            }),
            (folder, subfolder) => Err([
                folder.err().map(|error| (Row::Folder, error)),
                subfolder.err().map(|error| (Row::Subfolder, error)),
            ]
            .into_iter()
            .flatten()
            .collect()),
        }
    }

    /// Folder expanded for the import at `now`, a leading `~` the home folder: the folder
    /// the photos go under, the same for every photo where it reads none of them.
    pub fn folder(&self, now: DateTime, shot: &Shot) -> PathBuf {
        let text = self.folder.expand(&self.values(now, shot));
        match (text.strip_prefix('~'), &self.places.home) {
            (Some(rest), Some(home)) if rest.chars().next().is_none_or(is_separator) => {
                let rest = rest.trim_start_matches(is_separator);
                if rest.is_empty() {
                    home.clone()
                } else {
                    home.join(rest)
                }
            }
            _ => PathBuf::from(text),
        }
    }

    /// The folder the photo `shot` goes into for the import at `now`: Folder, then
    /// Subfolder worked out for it, straight into Folder where Subfolder is empty or
    /// expands to nothing. Subfolder is always under Folder: a separator it starts with is
    /// dropped, and one that expands to anything but folder names, a `..`, a root or a
    /// drive, which would lead out of Folder or replace it, is refused with what it
    /// expanded to.
    pub fn destination(&self, now: DateTime, shot: &Shot) -> Result<PathBuf, String> {
        let folder = self.folder(now, shot);
        let Some(subfolder) = &self.subfolder else {
            return Ok(folder);
        };
        let text = subfolder.expand(&self.values(now, shot));
        match text.trim_start_matches(is_separator) {
            "" => Ok(folder),
            rest if Path::new(rest)
                .components()
                .all(|part| matches!(part, Component::Normal(_))) =>
            {
                Ok(folder.join(rest))
            }
            _ => Err(text),
        }
    }

    /// What Folder's pattern and Subfolder's each read beside the places and the import
    /// name, told from their variables, nested ones included ([`Pattern::reads`]).
    pub fn uses(&self) -> (Uses, Uses) {
        let uses = |pattern: &Pattern| {
            let reads = pattern.reads();
            Uses {
                today: reads.now,
                taken: reads.taken,
                photo: reads.file || reads.camera,
            }
        };
        let subfolder = self.subfolder.as_ref().map(uses).unwrap_or_default();
        (uses(&self.folder), subfolder)
    }

    /// Whether Folder's pattern or Subfolder's reads what a photo's EXIF gives, its capture
    /// time or its camera, so that each photo's EXIF is read; its file name comes from the
    /// listing ([`Pattern::reads`]).
    pub fn reads_exif(&self) -> bool {
        let exif = |pattern: &Pattern| {
            let reads = pattern.reads();
            reads.taken || reads.camera
        };
        exif(&self.folder) || self.subfolder.as_ref().is_some_and(exif)
    }

    /// What the variables expand to for `shot` in the import at `now`: darktable's import
    /// session's, the job code the import name, the photo's file name as darktable's
    /// session holds it, the person's folders and name the client's, the Pictures folder the
    /// home's `Pictures` where the platform has none, as darktable's `_init_expansion` takes
    /// it.
    fn values(&self, now: DateTime, shot: &Shot) -> Values {
        let lossy = |path: &Option<PathBuf>| {
            (path.as_ref()).map(|path| path.to_string_lossy().into_owned())
        };
        Values {
            jobcode: self.job_code.clone(),
            now,
            taken: shot.taken,
            path: shot.name.clone(),
            maker: shot.maker.clone(),
            model: shot.model.clone(),
            home: lossy(&self.places.home).unwrap_or_default(),
            pictures: lossy(&pictures(&self.places)).unwrap_or_default(),
            desktop: lossy(&self.places.desktop),
            user_name: self.places.user_name.clone(),
            ..Values::default()
        }
    }
}

/// The Pictures folder `places` give, else the home folder's `Pictures`, as darktable's
/// variables take it (`_init_expansion`: `g_get_user_special_dir`, else the home and
/// `Pictures`); `None` with neither. The check's trees ([`crate::imported::tree_root`]) take
/// it too, so the check walks the folder the copy lands in.
pub fn pictures(places: &Places) -> Option<PathBuf> {
    (places.pictures.clone()).or_else(|| places.home.as_ref().map(|home| home.join("Pictures")))
}

/// `time` on the local clock, as darktable reads the import's time
/// (`g_date_time_new_now_local`) and a file's modified time (`dt_datetime_unix_to_exif`).
pub fn local(time: SystemTime) -> DateTime {
    let local = chrono::DateTime::<chrono::Local>::from(time).naive_local();
    DateTime {
        year: local.year(),
        month: local.month() as u8,
        day: local.day() as u8,
        hour: local.hour() as u8,
        minute: local.minute() as u8,
        second: local.second() as u8,
        microsecond: local.nanosecond() / 1000 % 1_000_000,
    }
}

/// When a photo was taken, for its patterns: the camera's clock as its EXIF holds it, else
/// its file's modified time on the local clock, as darktable's import takes it, else the
/// import's time, `now`, as darktable's variables do with neither.
pub fn taken(
    exif: Option<apertrust_raw::DateTime>,
    modified: Option<SystemTime>,
    now: DateTime,
) -> DateTime {
    let exif = exif.map(|t| DateTime {
        year: i32::from(t.year),
        month: t.month,
        day: t.day,
        hour: t.hour,
        minute: t.minute,
        second: t.second,
        microsecond: u32::from(t.millisecond) * 1000,
    });
    exif.or_else(|| modified.map(local)).unwrap_or(now)
}

/// The deepest folder of `path` that is there, `path` itself where it is: what the preview
/// asks whether it can be written and where its free space is read, since nothing is made
/// before a photo lands. `None` where not even a root is there, a disk not connected.
pub fn deepest_existing(path: &Path) -> Option<&Path> {
    path.ancestors()
        .find(|folder| !folder.as_os_str().is_empty() && folder.is_dir())
}

#[cfg(test)]
mod tests;
