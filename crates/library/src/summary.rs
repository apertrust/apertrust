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

//! The Metadata tool's facts and fields of several photos at once (decision 110): for each
//! fact row, the value the photos share, a range or a total where it helps, or how they
//! differ; for each field, by its property, and the copy's name, the value they share,
//! none, or their values with how many photos hold each ([`FieldSummary`]). [`build`] is a
//! pure function over each photo's listing, its [`Shot`] and its [`Fields`]; it knows
//! nothing of the engine, the tools or the interface, its keys the row ids ([`FACTS`]) and
//! the fields' properties, and its values typed, never words, which the client writes in
//! its language.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use apertrust_raw::DateTime;
use serde::{Deserialize, Serialize};

use crate::fields::{self, COPY_NAME, FieldKind, FieldValue, Fields};
use crate::ops::Shot;

/// The fact rows a summary describes, by the ids the Metadata tool's declaration gives
/// them: Basic's, Camera's, Exposure's, Lens's and GPS's.
pub const FACTS: &[&str] = &[
    "file_name",
    "folder",
    "type",
    "size_on_disk",
    "dimensions",
    "edited",
    "make",
    "model",
    "serial_number",
    "owner_name",
    "software",
    "date_time",
    "orientation",
    "resolution",
    "artist",
    "copyright",
    "image_description",
    "date_taken",
    "date_digitised",
    "exposure_time",
    "aperture",
    "iso",
    "exposure_bias",
    "exposure_program",
    "exposure_mode",
    "metering_mode",
    "flash",
    "white_balance",
    "scene_type",
    "subject_distance",
    "digital_zoom",
    "contrast",
    "saturation",
    "sharpness",
    "brightness",
    "colour_space",
    "unique_image_id",
    "lens",
    "lens_make",
    "lens_serial_number",
    "lens_specification",
    "focal_length",
    "focal_length_35mm",
    "crop_factor",
    "max_aperture",
    "latitude",
    "longitude",
    "altitude",
    "direction",
    "gps_time",
    "speed",
    "accuracy",
    "satellites",
    "map_datum",
];

/// The rows summarised as a range, earliest or smallest to latest or largest, where the
/// photos' values differ.
const RANGES: &[&str] = &["date_taken", "exposure_time", "aperture", "iso"];

/// How many of the most common values a [`FactSummary::Different`] and a
/// [`FieldSummary::Different`] keep.
const KEPT: usize = 10;

/// One selected photo as [`build`] reads it: its listing, and its facts as far as read.
#[derive(Debug, Clone, Copy)]
pub struct Summed<'a> {
    pub name: &'a str,
    pub folder: &'a Path,
    /// Its size on disk, in bytes.
    pub size: u64,
    pub edited: bool,
    pub facts: Facts<'a>,
    /// Its fields, as its listing read them or the session wrote them since.
    pub fields: &'a Fields,
}

/// How far a photo's facts are read.
#[derive(Debug, Clone, Copy)]
pub enum Facts<'a> {
    /// Not read yet: the folder's facts come after its listing.
    Waiting,
    /// Read, and there are none: a raw that cannot be read, or a cloud file not on the
    /// disk, which is never read.
    Missing,
    Read(&'a Arc<Shot>),
}

/// A row's value as the photos are compared by it: as the row shows it, a number rounded
/// as it is written, so 1/250 s and 0.004 s are one value. Ordered, so ties are broken and
/// a range found.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Key {
    Text(String),
    /// A number scaled to an integer at the row's precision, a code, a date and time as
    /// `yyyymmddhhmmssmmm`.
    Number(i64),
    Bool(bool),
    /// A value of several parts, each compared in turn.
    Parts(Vec<Key>),
}

/// A value the photos have: its [`Key`], and one photo's [`Shot`] holding it, which the
/// client writes as it writes that photo's row; none for a value of the listing (Folder,
/// Edited).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactValue {
    pub key: Key,
    pub shot: Option<Arc<Shot>>,
}

impl PartialEq for FactValue {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

/// What the photos have of one row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum FactSummary {
    /// Every photo has it, the same.
    Same(FactValue),
    /// No photo has it.
    None,
    /// The photos differ, or some lack it: the ten most common values with their counts,
    /// most common first, ties by value; how many values more; how many photos have none.
    Different {
        values: Vec<(FactValue, usize)>,
        more: usize,
        missing: usize,
    },
    /// Date taken, Exposure time, Aperture and ISO where the values differ: the lowest and
    /// the highest, and how many photos have none.
    Range {
        low: FactValue,
        high: FactValue,
        missing: usize,
    },
    /// Size on disk: the photos' sizes added up.
    Total { bytes: u64 },
    /// Some photos' facts are not read yet: `read` of `of`.
    Waiting { read: usize, of: usize },
}

/// What the photos hold of one field (decision 110).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum FieldSummary {
    /// Every photo holds it, the same.
    Same(FieldValue),
    /// No photo holds it.
    Empty,
    /// The photos differ, or some hold none: the ten most common values with their counts,
    /// most common first, ties by the value as shown; how many values more; how many
    /// photos hold none.
    Different {
        values: Vec<(FieldValue, usize)>,
        more: usize,
        empty: usize,
    },
}

/// What [`build`] gives: each row of [`FACTS`] by its id, and each field the Metadata tool
/// edits by its property ([`fields::FIELDS`]).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    /// How many photos it describes.
    pub photos: usize,
    #[serde(deserialize_with = "apertrust_base::wire::id::pairs")]
    facts: Vec<(&'static str, FactSummary)>,
    #[serde(deserialize_with = "apertrust_base::wire::id::pairs")]
    fields: Vec<(&'static str, FieldSummary)>,
}

impl Summary {
    /// The row `id`'s summary; none for an id [`FACTS`] lacks.
    pub fn get(&self, id: &str) -> Option<&FactSummary> {
        self.facts
            .iter()
            .find(|(row, _)| *row == id)
            .map(|(_, f)| f)
    }

    /// The field `property`'s summary, the copy's name by [`COPY_NAME`]; none for a
    /// property [`summed_fields`] lacks.
    pub fn field(&self, property: &str) -> Option<&FieldSummary> {
        self.fields
            .iter()
            .find(|(field, _)| *field == property)
            .map(|(_, f)| f)
    }
}

/// The fields a summary describes, by their properties: every field of
/// [`fields::FIELDS`] but the rating's, then the copy's name.
pub fn summed_fields() -> impl Iterator<Item = &'static str> {
    (fields::FIELDS.iter())
        .filter(|def| !fields::is_rating(def.property))
        .map(|def| def.property)
        .chain(std::iter::once(COPY_NAME))
}

/// The summary of `photos`, each row of [`FACTS`] in turn.
pub fn build(photos: &[Summed<'_>]) -> Summary {
    let read = photos
        .iter()
        .filter(|photo| !matches!(photo.facts, Facts::Waiting))
        .count();
    let facts = FACTS
        .iter()
        .map(|id| {
            let summary = match *id {
                "size_on_disk" => FactSummary::Total {
                    bytes: photos.iter().map(|photo| photo.size).sum(),
                },
                "file_name" | "folder" | "edited" => of_values(id, photos, |photo| {
                    let key = listed(id, photo)?;
                    Some(FactValue { key, shot: None })
                }),
                _ if read < photos.len() => FactSummary::Waiting {
                    read,
                    of: photos.len(),
                },
                _ => of_values(id, photos, |photo| {
                    let Facts::Read(shot) = photo.facts else {
                        return None;
                    };
                    let key = key(id, shot)?;
                    Some(FactValue {
                        key,
                        shot: Some(Arc::clone(shot)),
                    })
                }),
            };
            (*id, summary)
        })
        .collect();
    let fields = summed_fields()
        .map(|property| (property, of_field(property, photos)))
        .collect();
    Summary {
        photos: photos.len(),
        facts,
        fields,
    }
}

/// The field `property` over `photos`.
fn of_field(property: &str, photos: &[Summed<'_>]) -> FieldSummary {
    // A bag's items in any order are one value; a sequence keeps its order.
    let bag = fields::field(property).is_some_and(|def| def.kind == FieldKind::Bag);
    let mut held: HashMap<FieldValue, usize> = HashMap::new();
    let mut empty = 0;
    for photo in photos {
        match photo.fields.value(property) {
            Some(FieldValue::List(mut items)) if bag => {
                items.sort();
                *held.entry(FieldValue::List(items)).or_insert(0) += 1;
            }
            Some(value) => *held.entry(value).or_insert(0) += 1,
            None => empty += 1,
        }
    }
    let mut counts: Vec<(FieldValue, usize)> = held.into_iter().collect();
    match counts.len() {
        0 => return FieldSummary::Empty,
        1 if empty == 0 => return FieldSummary::Same(counts.remove(0).0),
        _ => {}
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.shown().cmp(&b.0.shown())));
    let more = counts.len().saturating_sub(KEPT);
    counts.truncate(KEPT);
    FieldSummary::Different {
        values: counts,
        more,
        empty,
    }
}

/// The row `id` over `photos`, each photo's value as `value` gives it.
fn of_values(
    id: &str,
    photos: &[Summed<'_>],
    value: impl Fn(&Summed<'_>) -> Option<FactValue>,
) -> FactSummary {
    let mut counts: HashMap<Key, (FactValue, usize)> = HashMap::new();
    let mut missing = 0;
    for photo in photos {
        match value(photo) {
            Some(value) => counts.entry(value.key.clone()).or_insert((value, 0)).1 += 1,
            None => missing += 1,
        }
    }
    let mut values: Vec<(FactValue, usize)> = counts.into_values().collect();
    match values.len() {
        0 => return FactSummary::None,
        1 if missing == 0 => return FactSummary::Same(values.remove(0).0),
        _ => {}
    }
    if RANGES.contains(&id) && values.len() > 1 {
        let low = values.iter().min_by(|a, b| a.0.key.cmp(&b.0.key));
        let high = values.iter().max_by(|a, b| a.0.key.cmp(&b.0.key));
        if let (Some(low), Some(high)) = (low, high) {
            return FactSummary::Range {
                low: low.0.clone(),
                high: high.0.clone(),
                missing,
            };
        }
    }
    values.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.key.cmp(&b.0.key)));
    let more = values.len().saturating_sub(KEPT);
    values.truncate(KEPT);
    FactSummary::Different {
        values,
        more,
        missing,
    }
}

/// The value of a row of the listing.
fn listed(id: &str, photo: &Summed<'_>) -> Option<Key> {
    match id {
        "file_name" => Some(Key::Text(photo.name.to_owned())),
        "folder" => Some(Key::Text(photo.folder.to_string_lossy().into_owned())),
        "edited" => Some(Key::Bool(photo.edited)),
        _ => None,
    }
}

/// `value` scaled to an integer at `decimals` decimals; none for none, 0 or less, as a
/// row shows only what the photo records.
fn recorded(value: Option<f32>, decimals: i32) -> Option<Key> {
    value
        .filter(|value| *value > 0.0)
        .map(|value| scaled(f64::from(value), decimals))
}

fn scaled(value: f64, decimals: i32) -> Key {
    Key::Number((value * 10f64.powi(decimals)).round() as i64)
}

fn text(value: &Option<String>) -> Option<Key> {
    value.as_ref().map(|text| Key::Text(text.trim().to_owned()))
}

fn code(value: Option<u16>) -> Option<Key> {
    value.map(|code| Key::Number(i64::from(code)))
}

/// A date and time as `yyyymmddhhmmssmmm` and its offset, the earliest the least.
fn dated(at: Option<DateTime>) -> Option<Key> {
    at.map(|at| {
        let stamp = [
            (u64::from(at.year), 10_000_000_000_000),
            (u64::from(at.month), 100_000_000_000),
            (u64::from(at.day), 1_000_000_000),
            (u64::from(at.hour), 10_000_000),
            (u64::from(at.minute), 100_000),
            (u64::from(at.second), 1_000),
            (u64::from(at.millisecond), 1),
        ]
        .iter()
        .map(|(part, unit)| part * unit)
        .sum::<u64>();
        let offset = at.offset_minutes.map_or(i64::MIN, i64::from);
        Key::Parts(vec![Key::Number(stamp as i64), Key::Number(offset)])
    })
}

/// An exposure time as darktable writes it (`dt_util_format_exposure`), which a client
/// words: a number of seconds, whole or to a tenth, or a fraction 1/n, n whole or to a
/// tenth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shutter {
    Seconds { value: f32, decimals: usize },
    Fraction { denominator: f32, decimals: usize },
}

/// `seconds` as darktable writes an exposure time: whole seconds, or to a tenth; below
/// 0.29 s or where the fraction is whole, 1/n; where n has one decimal, 1/n to a tenth;
/// else seconds to a tenth.
pub fn shutter(seconds: f32) -> Shutter {
    if seconds >= 1.0 {
        let decimals = usize::from(seconds.round() != seconds);
        Shutter::Seconds {
            value: seconds,
            decimals,
        }
    } else if seconds < 0.29 || (1.0 / seconds).round() == 1.0 / seconds {
        Shutter::Fraction {
            denominator: 1.0 / seconds,
            decimals: 0,
        }
    } else if 10.0 * (10.0 / seconds).round() == (100.0 / seconds).round() {
        Shutter::Fraction {
            denominator: 1.0 / seconds,
            decimals: 1,
        }
    } else {
        Shutter::Seconds {
            value: seconds,
            decimals: 1,
        }
    }
}

impl Shutter {
    /// The time as written, in seconds.
    fn shown(self) -> f64 {
        let at = |value: f32, decimals: usize| {
            let scale = 10f64.powi(decimals as i32);
            (f64::from(value) * scale).round() / scale
        };
        match self {
            Shutter::Seconds { value, decimals } => at(value, decimals),
            Shutter::Fraction {
                denominator,
                decimals,
            } => 1.0 / at(denominator, decimals),
        }
    }
}

/// An exposure time's key: the time as written, in nanoseconds, so two times written
/// apart, 1/8000 and 1/8001 s, stay apart.
fn exposure(seconds: f32) -> Key {
    scaled(shutter(seconds).shown(), 9)
}

/// A coordinate in degrees as the row writes it, to a tenth of a minute.
fn coordinate(degrees: Option<f64>) -> Option<Key> {
    degrees.map(|degrees| Key::Number((degrees * 600.0).round() as i64))
}

/// The row `id`'s value in `shot`, as the row shows it; none where it has none.
fn key(id: &str, shot: &Shot) -> Option<Key> {
    let exif = &shot.exif;
    let parts = |parts: Vec<Option<Key>>| -> Option<Key> {
        let parts: Option<Vec<Key>> = parts.into_iter().collect();
        parts.map(Key::Parts)
    };
    match id {
        "type" => Some(Key::Text(shot.format.clone())),
        "dimensions" => Some(Key::Parts(vec![
            Key::Number(i64::from(shot.size[0])),
            Key::Number(i64::from(shot.size[1])),
        ])),
        "make" => text(&exif.make_written).or_else(|| Some(Key::Text(shot.make.clone()))),
        "model" => text(&exif.model_written).or_else(|| Some(Key::Text(shot.alias.clone()))),
        "serial_number" => text(&exif.serial_number),
        "owner_name" => text(&exif.owner_name),
        "software" => text(&exif.software),
        "date_time" => dated(exif.date_time),
        "orientation" => code(exif.orientation_code),
        "resolution" => exif.resolution.and_then(|r| {
            let unit = Some(Key::Number(i64::from(r.unit)));
            parts(vec![recorded(Some(r.x), 2), recorded(Some(r.y), 2), unit])
        }),
        "artist" => text(&exif.artist),
        "copyright" => text(&exif.copyright),
        "image_description" => text(&exif.image_description),
        "date_taken" => dated(exif.taken),
        "date_digitised" => dated(exif.date_digitised),
        "exposure_time" => (exif.exposure_time.filter(|t| *t > 0.0)).map(exposure),
        "aperture" => recorded(exif.aperture, 1),
        "iso" => recorded(shot.iso, 0),
        "exposure_bias" => exif.exposure_bias.map(|ev| scaled(f64::from(ev), 2)),
        "exposure_program" => code(exif.exposure_program),
        "exposure_mode" => code(exif.exposure_mode),
        "metering_mode" => code(exif.metering_mode),
        "flash" => code(exif.flash),
        "white_balance" => code(exif.white_balance),
        "scene_type" => code(exif.scene_type),
        "subject_distance" => recorded(exif.focus_distance, 2),
        "digital_zoom" => exif.digital_zoom.map(|zoom| scaled(f64::from(zoom), 1)),
        "contrast" => code(exif.contrast),
        "saturation" => code(exif.saturation),
        "sharpness" => code(exif.sharpness),
        "brightness" => exif.brightness.map(|ev| scaled(f64::from(ev), 2)),
        "colour_space" => code(exif.colour_space),
        "unique_image_id" => text(&exif.unique_image_id),
        "lens" => text(&exif.lens),
        "lens_make" => text(&exif.lens_make),
        "lens_serial_number" => text(&exif.lens_serial_number),
        "lens_specification" => exif
            .lens_specification
            .map(|spec| Key::Parts(spec.iter().map(|v| scaled(f64::from(*v), 1)).collect())),
        "focal_length" => recorded(exif.focal_length, 1),
        "focal_length_35mm" => recorded(focal_length_35mm(exif), 0),
        "crop_factor" => recorded(exif.crop_factor, 2),
        "max_aperture" => recorded(exif.max_aperture, 1),
        "latitude" => coordinate(exif.latitude),
        "longitude" => coordinate(exif.longitude),
        "altitude" => exif.elevation.map(|metres| scaled(metres, 0)),
        "direction" => exif.gps_direction.map(|direction| {
            let magnetic = Key::Number(direction.magnetic.map_or(-1, i64::from));
            Key::Parts(vec![scaled(direction.degrees, 1), magnetic])
        }),
        "gps_time" => dated(exif.gps_time),
        "speed" => exif.gps_speed.map(|speed| {
            let unit = Key::Number(i64::from(u32::from(speed.unit)));
            Key::Parts(vec![scaled(speed.value, 1), unit])
        }),
        "accuracy" => exif.gps_accuracy.map(|metres| scaled(metres, 1)),
        "satellites" => text(&exif.gps_satellites),
        "map_datum" => text(&exif.gps_map_datum),
        _ => None,
    }
}

/// The 35 mm equivalent focal length, as the Metadata tool shows it: FocalLengthIn35mmFilm,
/// else the focal length times the crop factor, in whole millimetres.
pub fn focal_length_35mm(exif: &apertrust_raw::Exif) -> Option<f32> {
    exif.focal_length_35mm.filter(|mm| *mm > 0.0).or_else(|| {
        let mm = exif.focal_length.filter(|mm| *mm > 0.0)?;
        let crop = exif.crop_factor.filter(|crop| *crop > 0.0)?;
        Some((mm * crop).round())
    })
}

#[cfg(test)]
mod tests;
