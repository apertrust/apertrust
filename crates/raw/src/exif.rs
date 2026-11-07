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
//! rawler reads, where darktable, through Exiv2, prefers a maker note's for some.

use rawler::exif::Exif as RawlerExif;
use rawler::formats::tiff::Rational;

/// The photo's shooting data from its EXIF, each `None` where the file records none, as
/// darktable reads them (`dt_image_t`'s `exif_*` and `geoloc`, and the metadata its import
/// takes from the EXIF). The camera's names, its ISO and its exposure bias are
/// [`RawImage`](crate::RawImage)'s.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Exif {
    /// When the photo was taken, the camera's clock: DateTimeOriginal with
    /// SubSecTimeOriginal's milliseconds; `None` where it does not hold a valid time.
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
    /// Artist, Copyright and an ASCII UserComment, which darktable's import takes as the
    /// photo's creator, rights and description.
    pub artist: Option<String>,
    pub copyright: Option<String>,
    pub user_comment: Option<String>,
}

/// A date and time as the camera's clock showed it, with no time zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

/// darktable's reading of rawler's EXIF data and lens name.
pub(crate) fn read(exif: &RawlerExif) -> Exif {
    // Exiv2's `toFloat` of a rational: 0 for a denominator of 0.
    let value = |r: &Rational| {
        if r.d == 0 {
            0.0
        } else {
            r.n as f32 / r.d as f32
        }
    };
    let signed = |n: i32, d: i32| if d == 0 { 0.0 } else { n as f32 / d as f32 };
    let exposure_time = exif.exposure_time.as_ref().map(value).or_else(|| {
        exif.shutter_speed_value
            .map(|apex| (-signed(apex.n, apex.d)).exp2())
    });
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
        taken: exif
            .date_time_original
            .as_deref()
            .and_then(|text| taken(text, exif.sub_sec_time_original.as_deref())),
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
    }
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
}
