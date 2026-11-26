// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/common/exif.cc, src/common/datetime.c and src/common/utility.c)
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

//! The shooting data darktable reads from a raw's EXIF into its image
//! (`_exif_decode_exif_data` in `src/common/exif.cc`) for its variables
//! (`docs/ports/variables.md`), as far as rawler's EXIF reaches: the standard EXIF tags
//! rawler reads, where darktable, through Exiv2, prefers a maker note's for some. With
//! them, what the Metadata tool shows of IFD0, the Exif IFD and the GPS IFD (decision
//! 110): rawler's fields first, then the tags it lacks, read through its TIFF parser
//! ([`crate::tags`]).

use rawler::decoders::RawMetadata;
use rawler::exif::Exif as RawlerExif;
use rawler::formats::tiff::{Rational, SRational};
use serde::{Deserialize, Serialize};

use crate::cropfactors::{self, Inputs};
use crate::tags::{self, Tags};

/// The photo's shooting data from its EXIF, each `None` where the file records none, as
/// darktable reads them (`dt_image_t`'s `exif_*` and `geoloc`, and the metadata its import
/// takes from the EXIF), and the rest the Metadata tool shows (decision 110), each with its
/// tag. The camera's names as rawler spells them, its ISO and its exposure bias are
/// [`RawImage`](crate::RawImage)'s too.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Exif {
    /// When the photo was taken, the camera's clock: DateTimeOriginal with
    /// SubSecTimeOriginal's milliseconds and OffsetTimeOriginal; `None` where it does not
    /// hold a valid time.
    pub taken: Option<DateTime>,
    /// The exposure time in seconds: ExposureTime, or 2^−ShutterSpeedValue.
    pub exposure_time: Option<f32>,
    /// The f-number: FNumber, or 2^(ApertureValue / 2).
    pub aperture: Option<f32>,
    /// FocalLength, in mm.
    pub focal_length: Option<f32>,
    /// SubjectDistance, in m; darktable prefers Nikon's, Olympus's, Canon's and Sony's
    /// maker notes, which rawler does not read.
    pub focus_distance: Option<f32>,
    /// ExposureProgram, MeteringMode, WhiteBalance and Flash, as the tags hold them.
    /// darktable prefers a maker note's white balance, which rawler does not read.
    pub exposure_program: Option<u16>,
    pub metering_mode: Option<u16>,
    pub white_balance: Option<u16>,
    pub flash: Option<u16>,
    /// The lens: rawler's name for it from its lens data, or LensModel, with darktable's
    /// changes to the name; darktable's comes from Exiv2's lens tables first.
    pub lens: Option<String>,
    /// The location from the GPS tags, in degrees, north and east positive, and the
    /// elevation in m, above sea level positive.
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub elevation: Option<f64>,
    /// Artist (0x013B), Copyright (0x8298) and an ASCII UserComment, which darktable's
    /// import takes as the photo's creator, rights and description.
    pub artist: Option<String>,
    pub copyright: Option<String>,
    pub user_comment: Option<String>,
    /// Make (0x010F) and Model (0x0110) as IFD0 writes them, trimmed.
    pub make_written: Option<String>,
    pub model_written: Option<String>,
    /// BodySerialNumber (0xA431) and CameraOwnerName (0xA430).
    pub serial_number: Option<String>,
    pub owner_name: Option<String>,
    /// Software (0x0131).
    pub software: Option<String>,
    /// DateTime (0x0132), when the file was last changed by the camera, with SubSecTime
    /// and OffsetTime.
    pub date_time: Option<DateTime>,
    /// Orientation (0x0112), as the tag holds it.
    pub orientation_code: Option<u16>,
    /// XResolution, YResolution and ResolutionUnit (0x011A, 0x011B, 0x0128).
    pub resolution: Option<Resolution>,
    /// ImageDescription (0x010E).
    pub image_description: Option<String>,
    /// DateTimeDigitized (0x9004) with SubSecTimeDigitized and OffsetTimeDigitized.
    pub date_digitised: Option<DateTime>,
    /// ExposureMode (0xA402), LightSource (0x9208), SceneCaptureType (0xA406) and
    /// SubjectDistanceRange (0xA40C), as the tags hold them.
    pub exposure_mode: Option<u16>,
    pub light_source: Option<u16>,
    pub scene_type: Option<u16>,
    pub subject_distance_range: Option<u16>,
    /// DigitalZoomRatio (0xA404); 0 where the camera used none.
    pub digital_zoom: Option<f32>,
    /// Contrast (0xA408), Saturation (0xA409) and Sharpness (0xA40A), as the tags hold
    /// them.
    pub contrast: Option<u16>,
    pub saturation: Option<u16>,
    pub sharpness: Option<u16>,
    /// BrightnessValue (0x9203), in EV.
    pub brightness: Option<f32>,
    /// ColorSpace (0xA001), as the tag holds it.
    pub colour_space: Option<u16>,
    /// ImageUniqueID (0xA420), else a CR3's unique id as 32 hexadecimal digits.
    pub unique_image_id: Option<String>,
    /// SensitivityType (0x8830), as the tag holds it.
    pub sensitivity_type: Option<u16>,
    /// ExposureBiasValue (0x9204), in EV.
    pub exposure_bias: Option<f32>,
    /// LensMake (0xA433) and LensSerialNumber (0xA435), as rawler reads them.
    pub lens_make: Option<String>,
    pub lens_serial_number: Option<String>,
    /// LensSpecification (0xA432): the shortest and longest focal lengths in mm, and the
    /// f-numbers at each, 0 for a part the lens does not record.
    pub lens_specification: Option<[f32; 4]>,
    /// FocalLengthIn35mmFilm (0xA405), in mm.
    pub focal_length_35mm: Option<f32>,
    /// The crop factor, as darktable works it out (`src/cropfactors.rs`).
    pub crop_factor: Option<f32>,
    /// MaxApertureValue (0x9205) as an f-number, 2^(v / 2).
    pub max_aperture: Option<f32>,
    /// GPSImgDirection and GPSImgDirectionRef.
    pub gps_direction: Option<Direction>,
    /// GPSDateStamp and GPSTimeStamp, in UTC.
    pub gps_time: Option<DateTime>,
    /// GPSSpeed and GPSSpeedRef.
    pub gps_speed: Option<Speed>,
    /// GPSHPositioningError, in m.
    pub gps_accuracy: Option<f64>,
    /// GPSSatellites and GPSMapDatum, as written.
    pub gps_satellites: Option<String>,
    pub gps_map_datum: Option<String>,
    /// The fields Apertrust does not read from this file's format yet, by their names
    /// here: those `src/tags.rs` reads, where the IFDs cannot be reached; every one
    /// ([`Exif::unread_all`]) where the file's metadata cannot be read; none otherwise.
    #[serde(deserialize_with = "apertrust_base::wire::id::vec")]
    pub unread: Vec<&'static str>,
}

/// Every field of [`Exif`] but `unread`, by its name, as [`Exif::unread`] lists them.
const FIELDS: &[&str] = &[
    "taken",
    "exposure_time",
    "aperture",
    "focal_length",
    "focus_distance",
    "exposure_program",
    "metering_mode",
    "white_balance",
    "flash",
    "lens",
    "latitude",
    "longitude",
    "elevation",
    "artist",
    "copyright",
    "user_comment",
    "make_written",
    "model_written",
    "serial_number",
    "owner_name",
    "software",
    "date_time",
    "orientation_code",
    "resolution",
    "image_description",
    "date_digitised",
    "exposure_mode",
    "light_source",
    "scene_type",
    "subject_distance_range",
    "digital_zoom",
    "contrast",
    "saturation",
    "sharpness",
    "brightness",
    "colour_space",
    "unique_image_id",
    "sensitivity_type",
    "exposure_bias",
    "lens_make",
    "lens_serial_number",
    "lens_specification",
    "focal_length_35mm",
    "crop_factor",
    "max_aperture",
    "gps_direction",
    "gps_time",
    "gps_speed",
    "gps_accuracy",
    "gps_satellites",
    "gps_map_datum",
];

impl Exif {
    /// Nothing read and every field listed unread: what a file whose metadata cannot be
    /// read gives, so each says why it shows nothing.
    pub fn unread_all() -> Exif {
        Exif {
            unread: FIELDS.to_vec(),
            ..Exif::default()
        }
    }
}

/// A date and time as a clock showed it: the camera's, with the offset from UTC it
/// recorded where it did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateTime {
    /// 1 to 9999.
    pub year: u16,
    /// 1 to 12.
    pub month: u8,
    /// 1 to the month's days.
    pub day: u8,
    /// 0 to 23.
    pub hour: u8,
    /// 0 to 59.
    pub minute: u8,
    /// 0 to 59.
    pub second: u8,
    /// 0 to 999.
    pub millisecond: u16,
    /// The clock's offset from UTC in minutes, east positive, from the offset tag beside
    /// the time ("+01:00"); `None` where none is recorded.
    pub offset_minutes: Option<i16>,
}

/// A resolution as IFD0 records it: XResolution and YResolution in pixels per
/// ResolutionUnit, 2 inches (EXIF's default where it is missing) or 3 centimetres.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Resolution {
    pub x: f32,
    pub y: f32,
    pub unit: u16,
}

/// Where the camera pointed, in degrees from north, and whether north is true or
/// magnetic, by GPSImgDirectionRef's `T` and `M`; `None` for another reference.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Direction {
    pub degrees: f64,
    pub magnetic: Option<bool>,
}

/// How fast the receiver moved, `value` in the unit GPSSpeedRef names: `K` kilometres an
/// hour, `M` miles an hour, `N` knots.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Speed {
    pub value: f64,
    pub unit: char,
}

/// darktable's reading of rawler's EXIF data and lens name, with the rest of rawler's
/// metadata the Metadata tool shows, the tags rawler lacks (`tags`, `None` where the
/// file's IFDs cannot be reached, which [`Exif::unread`] then lists) and the crop factor
/// worked out from them, for the camera the file's Model names, else rawler's model.
pub(crate) fn read(metadata: &RawMetadata, tags: Option<&Tags>) -> Exif {
    let exif = &metadata.exif;
    let mut read = read_rawler(exif);
    let gps = exif.gps.as_ref();
    fn text(text: &Option<String>) -> Option<&str> {
        text.as_deref().map(str::trim).filter(|t| !t.is_empty())
    }
    read.serial_number = text(&exif.serial_number).map(str::to_owned);
    read.owner_name = text(&exif.owner_name).map(str::to_owned);
    read.date_time = (exif.modify_date.as_deref()).and_then(|at| {
        let sub = exif.sub_sec_time.as_deref();
        dated(at, sub, exif.offset_time.as_deref())
    });
    read.date_digitised = (exif.create_date.as_deref()).and_then(|at| {
        let sub = exif.sub_sec_time_digitized.as_deref();
        dated(at, sub, exif.offset_time_digitized.as_deref())
    });
    read.orientation_code = exif.orientation;
    read.exposure_mode = exif.exposure_mode;
    read.light_source = exif.light_source;
    read.scene_type = exif.scene_capture_type;
    read.subject_distance_range = exif.subject_distance_range;
    read.colour_space = exif.color_space;
    read.sensitivity_type = exif.sensitivity_type;
    read.brightness = exif.brightness_value.as_ref().map(signed);
    read.exposure_bias = exif.exposure_bias.as_ref().map(signed);
    read.lens_make = text(&exif.lens_make).map(str::to_owned);
    read.lens_serial_number = text(&exif.lens_serial_number).map(str::to_owned);
    read.lens_specification = exif
        .lens_spec
        .as_ref()
        .map(|spec| spec.each_ref().map(value));
    read.max_aperture = (exif.max_aperture_value.as_ref()).map(|apex| (value(apex) / 2.0).exp2());
    read.gps_direction = gps.and_then(|gps| {
        Some(Direction {
            degrees: f64::from(value(gps.gps_img_direction.as_ref()?)),
            magnetic: match gps.gps_img_direction_ref.as_deref().map(str::trim) {
                Some("T") => Some(false),
                Some("M") => Some(true),
                _ => None,
            },
        })
    });
    read.gps_time =
        gps.and_then(|gps| gps_time(gps.gps_date_stamp.as_deref()?, gps.gps_timestamp.as_ref()?));
    read.gps_speed = gps.and_then(|gps| {
        Some(Speed {
            value: f64::from(value(gps.gps_speed.as_ref()?)),
            unit: gps.gps_speed_ref.as_deref()?.trim().chars().next()?,
        })
    });
    read.gps_accuracy = (gps.and_then(|gps| gps.gps_h_positioning_error.as_ref()))
        .map(|error| f64::from(value(error)));
    read.gps_satellites = gps
        .and_then(|gps| text(&gps.gps_satellites))
        .map(str::to_owned);
    read.gps_map_datum = gps
        .and_then(|gps| text(&gps.gps_map_datum))
        .map(str::to_owned);
    let unique = metadata.unique_image_id.map(|id| format!("{id:032X}"));
    let Some(tags) = tags else {
        read.unique_image_id = unique;
        // darktable's table still gives the crop factor of a camera it lists, by rawler's
        // model; the camera's names as rawler spells them stand for the file's own.
        let crop = cropfactors::crop_factor(&Inputs {
            focal_length: read.focal_length.unwrap_or(0.0),
            model: &metadata.model,
            ..Inputs::default()
        });
        read.crop_factor = (crop.is_finite() && crop > 0.0).then_some(crop);
        read.unread = (tags::FIELDS.iter().copied())
            .filter(|field| match *field {
                "make_written" => metadata.make.trim().is_empty(),
                "model_written" => metadata.model.trim().is_empty(),
                "crop_factor" => read.crop_factor.is_none(),
                _ => true,
            })
            .collect();
        return read;
    };
    read.make_written = tags.make.clone();
    read.model_written = tags.model.clone();
    read.software = tags.software.clone();
    read.image_description = tags.image_description.clone();
    read.resolution = tags.resolution;
    read.digital_zoom = tags.digital_zoom;
    read.contrast = tags.contrast;
    read.saturation = tags.saturation;
    read.sharpness = tags.sharpness;
    read.unique_image_id = tags.unique_image_id.clone().or(unique);
    read.focal_length_35mm = tags.focal_length_35mm;
    let crop = cropfactors::crop_factor(&Inputs {
        focal_length: read.focal_length.unwrap_or(0.0),
        focal_length_35mm: tags.focal_length_35mm.unwrap_or(0.0),
        focal_plane: tags.focal_plane,
        dimensions: tags.dimensions,
        olympus_diagonal: tags.olympus_diagonal,
        model: tags.model.as_deref().unwrap_or(&metadata.model),
    });
    read.crop_factor = (crop.is_finite() && crop > 0.0).then_some(crop);
    read
}

/// Exiv2's `toFloat` of a rational: 0 for a denominator of 0.
fn value(r: &Rational) -> f32 {
    if r.d == 0 {
        0.0
    } else {
        r.n as f32 / r.d as f32
    }
}

/// Exiv2's `toFloat` of a signed rational: 0 for a denominator of 0.
fn signed(r: &SRational) -> f32 {
    if r.d == 0 {
        0.0
    } else {
        r.n as f32 / r.d as f32
    }
}

/// darktable's reading of rawler's EXIF data and lens name: what darktable's image holds.
fn read_rawler(exif: &RawlerExif) -> Exif {
    let exposure_time = exif
        .exposure_time
        .as_ref()
        .map(value)
        .or_else(|| exif.shutter_speed_value.map(|apex| (-signed(&apex)).exp2()));
    let aperture = exif.fnumber.as_ref().map(value).or_else(|| {
        exif.aperture_value
            .as_ref()
            .map(|apex| (value(apex) / 2.0).exp2())
    });
    let gps = exif.gps.as_ref();
    let coordinate = |degrees: Option<&[Rational; 3]>, side: Option<&String>| {
        degrees_to_number(degrees?, side?.chars().next().unwrap_or('\0'))
    };
    Exif {
        taken: exif.date_time_original.as_deref().and_then(|text| {
            let sub = exif.sub_sec_time_original.as_deref();
            dated(text, sub, exif.offset_time_original.as_deref())
        }),
        exposure_time,
        aperture,
        focal_length: exif.focal_length.as_ref().map(value),
        focus_distance: exif.subject_distance.as_ref().map(value),
        exposure_program: exif.exposure_program,
        metering_mode: exif.metering_mode,
        white_balance: exif.white_balance,
        flash: exif.flash,
        lens: exif
            .lens_model
            .as_deref()
            .map(|lens| lens_name(lens, exif.lens_make.as_deref())),
        latitude: gps
            .and_then(|gps| coordinate(gps.gps_latitude.as_ref(), gps.gps_latitude_ref.as_ref())),
        longitude: gps
            .and_then(|gps| coordinate(gps.gps_longitude.as_ref(), gps.gps_longitude_ref.as_ref())),
        elevation: gps.and_then(|gps| elevation(gps.gps_altitude.as_ref()?, gps.gps_altitude_ref?)),
        artist: exif.artist.clone().filter(|text| !text.is_empty()),
        copyright: exif.copyright.clone().filter(|text| !text.is_empty()),
        user_comment: exif
            .user_comment
            .clone()
            .filter(|text| !text.is_empty() && text != "binary comment"),
        ..Exif::default()
    }
}

/// A time as [`taken`] reads it, with the offset `offset` records.
fn dated(text: &str, subsec: Option<&str>, offset: Option<&str>) -> Option<DateTime> {
    let at = taken(text, subsec)?;
    Some(DateTime {
        offset_minutes: offset.and_then(offset_minutes),
        ..at
    })
}

/// An EXIF offset from UTC, "+01:00" or "-05:30", in minutes; `None` for anything else, as
/// a camera's "   :  " for none.
fn offset_minutes(text: &str) -> Option<i16> {
    let bytes = text.trim().as_bytes();
    let [sign @ (b'+' | b'-'), h1, h2, b':', m1, m2] = *bytes else {
        return None;
    };
    let digits = [h1, h2, m1, m2];
    if !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let [h1, h2, m1, m2] = digits.map(|d| i16::from(d - b'0'));
    let (hours, minutes) = (h1 * 10 + h2, m1 * 10 + m2);
    if hours > 14 || minutes > 59 {
        return None;
    }
    let total = hours * 60 + minutes;
    Some(if sign == b'-' { -total } else { total })
}

/// The GPS clock's time, GPSDateStamp's "YYYY:MM:DD" and GPSTimeStamp's hours, minutes and
/// seconds, in UTC; `None` where either is not a valid time.
fn gps_time(date: &str, time: &[Rational; 3]) -> Option<DateTime> {
    let [hours, minutes, seconds] = time
        .each_ref()
        .map(|r| (r.d != 0).then(|| f64::from(r.n) / f64::from(r.d)));
    let (hours, minutes, seconds) = (hours?, minutes?, seconds?);
    let whole = seconds.trunc();
    let text = format!(
        "{} {:02}:{:02}:{:02}",
        date.trim(),
        hours as u32,
        minutes as u32,
        whole as u32
    );
    let millisecond = ((seconds - whole) * 1000.0).round().min(999.0) as u16;
    Some(DateTime {
        millisecond,
        offset_minutes: Some(0),
        ..taken(&text, None)?
    })
}

/// `_find_datetime_taken` and `dt_datetime_exif_to_numbers`: the first 19 characters,
/// "YYYY:MM:DD HH:MM:SS", the separators of the date any character, with the first three
/// characters of the sub-second time, trimmed, as a fraction, validated as glib's ISO
/// 8601 parser validates it.
fn taken(text: &str, subsec: Option<&str>) -> Option<DateTime> {
    let bytes = text.as_bytes().get(..19)?;
    let number = |range: std::ops::Range<usize>| -> Option<u16> {
        let digits = &bytes[range];
        digits
            .iter()
            .all(u8::is_ascii_digit)
            .then(|| digits.iter().fold(0, |n, d| n * 10 + u16::from(d - b'0')))
    };
    if !matches!(bytes[10], b' ' | b'T' | b't' | b'_') || bytes[13] != b':' || bytes[16] != b':' {
        return None;
    }
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let (hour, minute, second) = (number(11..13)?, number(14..16)?, number(17..19)?);
    let fraction: String = subsec.unwrap_or_default().chars().take(3).collect();
    let fraction = fraction.trim();
    if !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let millisecond = format!("{fraction:0<3}").parse().ok()?;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    let valid = (1..=9999).contains(&year)
        && (1..=days).contains(&day)
        && hour < 24
        && minute < 60
        && second < 60;
    valid.then_some(DateTime {
        year,
        month: month as u8,
        day: day as u8,
        hour: hour as u8,
        minute: minute as u8,
        second: second as u8,
        millisecond,
        offset_minutes: None,
    })
}

/// `dt_util_gps_rationale_to_number`: degrees, minutes and seconds to degrees, negative
/// to the south and west; `None` for a denominator of 0 but that of 0/0 seconds.
fn degrees_to_number(degrees: &[Rational; 3], side: char) -> Option<f64> {
    let part = |r: &Rational| (r.d != 0).then(|| f64::from(r.n) / f64::from(r.d));
    let mut result = part(&degrees[0])?;
    result += part(&degrees[1])? / 60.0;
    let seconds = &degrees[2];
    result += if seconds.d == 0 && seconds.n == 0 {
        0.0
    } else {
        part(seconds)? / 3600.0
    };
    Some(if matches!(side, 'S' | 'W') {
        -result
    } else {
        result
    })
}

/// `dt_util_gps_elevation_to_number`: negative unless GPSAltitudeRef, written out, starts
/// with "0".
fn elevation(altitude: &Rational, reference: u8) -> Option<f64> {
    let metres = (altitude.d != 0).then(|| f64::from(altitude.n) / f64::from(altitude.d))?;
    Some(if reference.to_string().starts_with('0') {
        metres
    } else {
        -metres
    })
}

/// darktable's changes to a lens's name: Canon's RF and RF-S lenses named in full, NIKKOR,
/// TAMRON and VILTROX capitalised, and a Sigma lens on a body that names its maker
/// SIGMA given "Sigma ".
fn lens_name(lens: &str, make: Option<&str>) -> String {
    let mut lens = if lens.starts_with("RF-") {
        // darktable skips the "S" of RF-S as it skips any fourth character.
        format!("Canon RF-S {}", lens.get(4..).unwrap_or_default())
    } else if let Some(rest) = lens.strip_prefix("RF") {
        format!("Canon RF {rest}")
    } else {
        lens.to_owned()
    };
    if ["NIKKOR ", "TAMRON ", "VILTROX"]
        .iter()
        .any(|prefix| lens.starts_with(prefix))
    {
        lens = lens[..1].to_owned() + &lens[1..7].to_ascii_lowercase() + &lens[7..];
    }
    if make == Some("SIGMA") && !lens.eq_ignore_ascii_case("Sigma") {
        lens = format!("Sigma {lens}");
    }
    lens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_time_taken_as_darktable() {
        let time = |year, month, day, hour, minute, second, millisecond| DateTime {
            year,
            month,
            day,
            hour,
            minute,
            second,
            millisecond,
            offset_minutes: None,
        };
        assert_eq!(
            taken("2017:07:20 10:52:48", Some("50")),
            Some(time(2017, 7, 20, 10, 52, 48, 500))
        );
        assert_eq!(
            taken("2021:05:15 13:04:45", Some("024")),
            Some(time(2021, 5, 15, 13, 4, 45, 24))
        );
        assert_eq!(
            taken("2022:11:20 15:38:30", None),
            Some(time(2022, 11, 20, 15, 38, 30, 0))
        );
        assert_eq!(
            taken("2024:02:29 00:00:00", Some("123456")),
            Some(time(2024, 2, 29, 0, 0, 0, 123))
        );
        // Cameras that never had their clock set, and a sub-second time glib cannot read.
        assert_eq!(taken("0000:00:00 00:00:00", None), None);
        assert_eq!(taken("2023:02:29 00:00:00", None), None);
        assert_eq!(taken("    :  :     :  :  ", None), None);
        assert_eq!(taken("2017:07:20 10:52:48", Some("5x")), None);
        assert_eq!(taken("2017:07:20", None), None);
    }

    #[test]
    fn reads_gps_as_darktable() {
        let r = |n, d| Rational { n, d };
        let north = [r(47, 1), r(57, 1), r(3063, 100)];
        let latitude = degrees_to_number(&north, 'N').unwrap();
        assert!((latitude - 47.958508).abs() < 1e-6, "{latitude}");
        assert_eq!(degrees_to_number(&north, 'S'), Some(-latitude));
        assert_eq!(
            degrees_to_number(&[r(11, 1), r(30, 1), r(0, 0)], 'W'),
            Some(-11.5)
        );
        assert_eq!(degrees_to_number(&[r(11, 0), r(30, 1), r(0, 1)], 'E'), None);
        assert_eq!(elevation(&r(60235, 100), 0), Some(602.35));
        assert_eq!(elevation(&r(10, 1), 1), Some(-10.0));
        assert_eq!(elevation(&r(10, 0), 0), None);
    }

    #[test]
    fn names_lenses_as_darktable() {
        assert_eq!(
            lens_name("RF24-105mm F4 L IS USM", None),
            "Canon RF 24-105mm F4 L IS USM"
        );
        assert_eq!(
            lens_name("RF-S18-150mm F3.5-6.3 IS STM", None),
            "Canon RF-S 18-150mm F3.5-6.3 IS STM"
        );
        assert_eq!(
            lens_name("NIKKOR Z 24-70mm f/4 S", None),
            "Nikkor Z 24-70mm f/4 S"
        );
        assert_eq!(
            lens_name("24-70mm F2.8 DG DN | Art 019", Some("SIGMA")),
            "Sigma 24-70mm F2.8 DG DN | Art 019"
        );
        assert_eq!(lens_name("XF18mmF2 R", Some("FUJIFILM")), "XF18mmF2 R");
    }

    #[test]
    fn reads_an_offset_from_utc() {
        assert_eq!(offset_minutes("+01:00"), Some(60));
        assert_eq!(offset_minutes("-05:30"), Some(-330));
        assert_eq!(offset_minutes(" +00:00 "), Some(0));
        // A camera's blank offset, and one out of range.
        assert_eq!(offset_minutes("   :  "), None);
        assert_eq!(offset_minutes("+15:00"), None);
        assert_eq!(offset_minutes("01:00"), None);
        let at = dated("2026:07:04 08:10:01", Some("12"), Some("+01:00")).unwrap();
        assert_eq!((at.millisecond, at.offset_minutes), (120, Some(60)));
    }

    #[test]
    fn reads_the_gps_clock_in_utc() {
        let r = |n, d| Rational { n, d };
        let time = gps_time("2026:07:04", &[r(7, 1), r(10, 1), r(1250, 100)]).unwrap();
        let wanted = (2026, 7, 4, 7, 10, 12, 500, Some(0));
        let got = (
            time.year,
            time.month,
            time.day,
            time.hour,
            time.minute,
            time.second,
            time.millisecond,
            time.offset_minutes,
        );
        assert_eq!(got, wanted);
        assert_eq!(gps_time("2026:07:04", &[r(7, 0), r(0, 1), r(0, 1)]), None);
        assert_eq!(gps_time("    :  :  ", &[r(7, 1), r(0, 1), r(0, 1)]), None);
    }

    /// The metadata rawler gives with `exif`, and no unique id.
    fn metadata(exif: RawlerExif) -> RawMetadata {
        RawMetadata {
            exif,
            ..RawMetadata::default()
        }
    }

    #[test]
    fn reads_rawler_s_fields_beside_darktable_s() {
        let r = |n, d| Rational { n, d };
        let exif = RawlerExif {
            serial_number: Some(" 0000417 ".to_owned()),
            owner_name: Some(String::new()),
            create_date: Some("2026:07:04 08:10:01".to_owned()),
            offset_time_digitized: Some("+01:00".to_owned()),
            max_aperture_value: Some(r(297, 100)),
            lens_spec: Some([r(24, 1), r(70, 1), r(28, 10), r(0, 0)]),
            brightness_value: Some(SRational { n: 76, d: 10 }),
            exposure_bias: Some(SRational { n: -1, d: 3 }),
            gps: Some(rawler::exif::ExifGPS {
                gps_img_direction: Some(r(213, 1)),
                gps_img_direction_ref: Some("M".to_owned()),
                gps_speed: Some(r(12, 1)),
                gps_speed_ref: Some("K".to_owned()),
                gps_map_datum: Some("WGS-84".to_owned()),
                ..Default::default()
            }),
            ..RawlerExif::default()
        };
        let read = read(&metadata(exif), Some(&Tags::default()));
        assert_eq!(read.serial_number.as_deref(), Some("0000417"));
        assert_eq!(read.owner_name, None);
        assert_eq!(
            read.date_digitised.and_then(|at| at.offset_minutes),
            Some(60)
        );
        // f/2.8 from its APEX value.
        let max = read.max_aperture.unwrap();
        assert!((max - 2.8).abs() < 0.01, "{max}");
        assert_eq!(read.lens_specification, Some([24.0, 70.0, 2.8, 0.0]));
        assert_eq!(read.brightness, Some(7.6));
        assert!((read.exposure_bias.unwrap() + 1.0 / 3.0).abs() < 1e-6);
        assert_eq!(
            read.gps_direction,
            Some(Direction {
                degrees: 213.0,
                magnetic: Some(true)
            })
        );
        assert_eq!(
            read.gps_speed,
            Some(Speed {
                value: 12.0,
                unit: 'K'
            })
        );
        assert_eq!(read.gps_map_datum.as_deref(), Some("WGS-84"));
        assert!(read.unread.is_empty());
    }

    #[test]
    fn a_file_whose_ifds_cannot_be_reached_lists_the_tags_unread() {
        let mut unique = metadata(RawlerExif::default());
        unique.unique_image_id = Some(0x4f2a);
        let read = read(&unique, None);
        assert_eq!(read.unread, tags::FIELDS);
        assert_eq!(read.software, None);
        // A CR3's unique id stands in for ImageUniqueID, as 32 hexadecimal digits.
        assert_eq!(
            read.unique_image_id.as_deref(),
            Some("00000000000000000000000000004F2A")
        );
        // The camera's names stand for Make and Model; darktable's table gives the crop
        // factor of a camera it lists.
        let mut named = metadata(RawlerExif::default());
        named.make = "Canon".to_owned();
        named.model = "Canon EOS 5D Mark IV".to_owned();
        let read = super::read(&named, None);
        assert_eq!(read.crop_factor, Some(1.0));
        let unread: Vec<&str> = (tags::FIELDS.iter().copied())
            .filter(|field| !matches!(*field, "make_written" | "model_written" | "crop_factor"))
            .collect();
        assert_eq!(read.unread, unread);
    }

    #[test]
    fn a_file_whose_metadata_cannot_be_read_lists_every_field_unread() {
        let read = Exif::unread_all();
        assert_eq!(read.unread, FIELDS);
        assert_eq!(read.exposure_time, None);
        assert!(read.unread.iter().all(|field| *field != "unread"));
    }

    #[test]
    fn works_out_the_crop_factor_from_the_tags() {
        let exif = RawlerExif {
            focal_length: Some(Rational { n: 183, d: 10 }),
            ..RawlerExif::default()
        };
        let tags = Tags {
            focal_length_35mm: Some(28.0),
            ..Tags::default()
        };
        let read = read(&metadata(exif), Some(&tags));
        let crop = read.crop_factor.unwrap();
        assert!((crop - 28.0 / 18.3).abs() < 1e-5, "{crop}");
        assert_eq!(read.focal_length_35mm, Some(28.0));
        let none = super::read(&metadata(RawlerExif::default()), Some(&Tags::default()));
        assert_eq!(none.crop_factor, None);
    }
}
