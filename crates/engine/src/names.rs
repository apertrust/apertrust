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

//! What an export's file names are made of: the variables' values for each photo
//! (`apertrust_export::variables::Values`, which `docs/ports/variables.md` says the export
//! fills), from the raw, its EXIF and the export itself. The session's export job fills
//! them ([`crate::export_job`]), and `apertrust-cli export` where it renders a photo
//! outside the job.

use std::path::{Path, PathBuf};

use apertrust_export::Places;
use apertrust_export::variables::{DateTime, Size, Values};
use apertrust_raw::RawImage;
use chrono::{Datelike, Timelike};

/// The local time now, as darktable takes an export's: once, as the storage's parameters
/// are made, so every photo of the export has the same.
pub fn now() -> DateTime {
    let now = chrono::Local::now().naive_local();
    DateTime {
        year: now.year(),
        month: now.month() as u8,
        day: now.day() as u8,
        hour: now.hour() as u8,
        minute: now.minute() as u8,
        second: now.second() as u8,
        microsecond: now.nanosecond() / 1000 % 1_000_000,
    }
}

/// What the variables read of a raw beside its EXIF: its camera's names and the shooting
/// data the decoder reports, kept with an open photo so that its export needs no second
/// decode.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Shot {
    pub maker: String,
    /// The name the file's camera has, an alias's own (`$(EXIF.MODEL)`).
    pub model: String,
    pub iso: Option<f32>,
    pub exposure_bias: Option<f32>,
    /// The exposure time in seconds, the f-number and the focal length in mm, from the
    /// EXIF ([`Shot::with_exif`]), which a client shows in the photo's caption.
    pub exposure_time: Option<f32>,
    pub aperture: Option<f32>,
    pub focal_length: Option<f32>,
}

impl Shot {
    pub fn of(raw: &RawImage) -> Self {
        Self {
            maker: raw.camera.make.clone(),
            model: raw.camera.alias.clone(),
            iso: raw.iso,
            exposure_bias: raw.exposure_bias,
            ..Self::default()
        }
    }

    /// With the shooting data `exif` records.
    pub fn with_exif(self, exif: &apertrust_raw::Exif) -> Self {
        Self {
            exposure_time: exif.exposure_time,
            aperture: exif.aperture,
            focal_length: exif.focal_length,
            ..self
        }
    }
}

/// What is the same for every photo of one export.
#[derive(Debug, Clone)]
pub struct Export {
    /// When the export started, local time: darktable takes it once, as the storage's
    /// parameters are made, so every photo has the same.
    pub now: DateTime,
    /// The export's largest size as the storage is given it, and Allow upscaling.
    pub max: Size,
    pub upscale: bool,
    pub version: String,
    /// The person's folders and name, as the client gives them.
    places: Places,
}

impl Export {
    /// The export started at `now` ([`now`]), to `max` with Allow upscaling `upscale`, by
    /// the build `version`, for the person `places` describes.
    pub fn new(now: DateTime, max: Size, upscale: bool, version: &str, places: Places) -> Self {
        Self {
            now,
            max,
            upscale,
            version: version.to_owned(),
            places,
        }
    }

    /// The home folder, for a leading `~` in the pattern.
    pub fn home(&self) -> Option<&Path> {
        self.places.home.as_deref()
    }

    /// The values for photo number `sequence` of the export: the raw at `path` as the
    /// decoder reads it, `shot`, its shooting data from its EXIF where that reads, and its
    /// sizes, the developed image's and `crop`'s, the image after every geometry. The EXIF
    /// is read from the file here, so this runs where a file may be read: a job's worker.
    pub fn values(&self, path: &Path, shot: &Shot, sequence: u32, sizes: Sizes) -> Values {
        let exif = apertrust_raw::read_exif_file(path).unwrap_or_else(|error| {
            tracing::warn!("{}: no shooting data: {error}", path.display());
            apertrust_raw::Exif::default()
        });
        self.values_with(path, shot, sequence, sizes, exif)
    }

    /// [`values`](Self::values) with the shooting data `exif` already read, or none: the
    /// export preview reads it only for a pattern that uses it.
    pub fn values_with(
        &self,
        path: &Path,
        shot: &Shot,
        sequence: u32,
        sizes: Sizes,
        exif: apertrust_raw::Exif,
    ) -> Values {
        let lossy = |path: &Option<PathBuf>| {
            path.as_ref()
                .map_or_else(String::new, |p| p.to_string_lossy().into_owned())
        };
        Values {
            sequence,
            now: self.now,
            max: self.max,
            upscale: self.upscale,
            version: self.version.clone(),
            user_name: self.places.user_name.clone(),
            home: lossy(&self.places.home),
            pictures: lossy(&self.places.pictures),
            desktop: self
                .places
                .desktop
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned()),
            path: path.to_string_lossy().into_owned(),
            maker: shot.maker.clone(),
            model: shot.model.clone(),
            lens: exif.lens.unwrap_or_default(),
            taken: exif.taken.map(|t| DateTime {
                year: i32::from(t.year),
                month: t.month,
                day: t.day,
                hour: t.hour,
                minute: t.minute,
                second: t.second,
                microsecond: u32::from(t.millisecond) * 1000,
            }),
            exposure: exif.exposure_time.unwrap_or(0.0),
            aperture: exif.aperture.unwrap_or(0.0),
            focal_length: exif.focal_length.unwrap_or(0.0),
            focus_distance: exif.focus_distance.unwrap_or(0.0),
            iso: shot.iso.unwrap_or(0.0),
            exposure_bias: shot.exposure_bias,
            exposure_program: exif.exposure_program,
            metering_mode: exif.metering_mode,
            white_balance: exif.white_balance,
            flash: exif.flash,
            latitude: exif.latitude,
            longitude: exif.longitude,
            elevation: exif.elevation,
            creator: exif.artist,
            rights: exif.copyright,
            description: exif.user_comment,
            sensor: sizes.sensor,
            raw: sizes.developed,
            crop: sizes.crop,
            ..Values::default()
        }
    }
}

/// A photo's sizes, which the variables name.
#[derive(Debug, Clone, Copy)]
pub struct Sizes {
    /// The raw's whole readout (`$(WIDTH.SENSOR)`).
    pub sensor: Size,
    /// The developed image, after the raw's crop (`$(WIDTH.RAW)`).
    pub developed: Size,
    /// After every geometry, before the export's size (`$(WIDTH.CROP)`).
    pub crop: Size,
}
