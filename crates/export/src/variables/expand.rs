// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2025  darktable developers (ported from src/common/variables.c, src/common/utility.c, src/common/image.c, src/common/datetime.c and src/common/exif.cc)
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

//! A read pattern expanded for one photo: `_get_base_value`'s values, as darktable
//! formats them, and `_variable_get_value`'s operations on them.

use super::names::Key;
use super::parse::strtol;
use super::{DateTime, Operation, Params, Part, Replace, Size, Values, Variable};

pub(super) fn parts(parts: &[Part], values: &Values) -> String {
    let mut expanded = String::new();
    for part in parts {
        match part {
            Part::Text(text) => expanded.push_str(text),
            Part::Variable(variable) => expanded.push_str(&variable_value(variable, values)),
        }
    }
    expanded
}

fn variable_value(variable: &Variable, values: &Values) -> String {
    let base = base_value(variable, values);
    match &variable.operation {
        None => base,
        Some(operation) => operate(operation, base, values),
    }
}

/// `_variable_get_value`'s operations, which darktable takes from bash's parameter
/// expansion, each a plain comparison of text.
fn operate(operation: &Operation, base: String, values: &Values) -> String {
    match operation {
        Operation::Default(text) if base.is_empty() => parts(text, values),
        Operation::Alternative(text) if !base.is_empty() => parts(text, values),
        Operation::Default(_) | Operation::Alternative(_) => base,
        Operation::Substring { offset, length } => substring(&base, *offset, *length),
        Operation::RemovePrefix(pattern) => {
            let pattern = parts(pattern, values);
            base.strip_prefix(pattern.as_str())
                .map_or(base.clone(), str::to_owned)
        }
        Operation::RemoveSuffix(pattern) => {
            let pattern = parts(pattern, values);
            base.strip_suffix(pattern.as_str())
                .map_or(base.clone(), str::to_owned)
        }
        Operation::Replace {
            replace,
            pattern,
            replacement,
        } => {
            let pattern = parts(pattern, values);
            let replacement = parts(replacement, values);
            match replace {
                // darktable finds empty text at the start and writes the replacement
                // alone, dropping the value: its, kept.
                Replace::First if pattern.is_empty() => replacement,
                Replace::First => base.replacen(&pattern, &replacement, 1),
                // darktable reads past its text there; nothing to replace.
                Replace::All if pattern.is_empty() => base,
                // Each match after the last, left to right, as bash replaces; darktable's
                // own fails where two matches overlap.
                Replace::All => base.replace(&pattern, &replacement),
                Replace::Prefix => match base.strip_prefix(pattern.as_str()) {
                    Some(rest) => replacement + rest,
                    None => base,
                },
                Replace::Suffix => match base.strip_suffix(pattern.as_str()) {
                    Some(rest) => rest.to_owned() + &replacement,
                    None => base,
                },
            }
        }
        Operation::Upper { all: true } => base.to_uppercase(),
        Operation::Lower { all: true } => base.to_lowercase(),
        Operation::Upper { all: false } => first_changed(&base, |c| {
            let mut upper = c.to_uppercase();
            single(&mut upper).unwrap_or(c)
        }),
        Operation::Lower { all: false } => first_changed(&base, |c| {
            let mut lower = c.to_lowercase();
            single(&mut lower).unwrap_or(c)
        }),
    }
}

/// The one character a case mapping gives, as glib's `g_unichar_toupper` and
/// `g_unichar_tolower` map a character to one; `None` where it gives several.
fn single(mapped: &mut impl Iterator<Item = char>) -> Option<char> {
    let first = mapped.next()?;
    mapped.next().is_none().then_some(first)
}

fn first_changed(text: &str, change: impl Fn(char) -> char) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => {
            let mut changed = String::with_capacity(text.len());
            changed.push(change(first));
            changed.push_str(chars.as_str());
            changed
        }
        None => String::new(),
    }
}

/// `$(VAR:offset:length)`, in characters: from the end where `offset` is negative, and up
/// to `length` before the end where it is.
fn substring(text: &str, offset: i64, length: Option<i64>) -> String {
    let chars: Vec<char> = text.chars().collect();
    let count = chars.len() as i64;
    let start = if offset >= 0 {
        offset.min(count)
    } else {
        count + offset.max(-count)
    };
    let end = match length {
        None => count,
        Some(length) if length >= 0 => start + length.min(count - start),
        Some(length) => count + length.max(start - count),
    };
    chars[start as usize..end as usize].iter().collect()
}

/// A variable's value before its operation: `_get_base_value`'s branch for its name.
fn base_value(variable: &Variable, values: &Values) -> String {
    let now = &values.now;
    // darktable rebuilds the photo's time from whole seconds (`dt_datetime_img_to_gdatetime`)
    // and, for a photo with no EXIF time, from its origin.
    let taken = values.taken.map_or(DateTime::ORIGIN, |taken| DateTime {
        microsecond: 0,
        ..taken
    });
    let size = |size: Size, height: bool| if height { size.height } else { size.width }.to_string();
    match variable.key {
        Key::YearShort => year_short(now),
        Key::Year => now.year.to_string(),
        Key::MonthShort => month_short(now).to_owned(),
        Key::MonthLong => month_long(now).to_owned(),
        Key::Month => format!("{:02}", now.month),
        Key::Day => format!("{:02}", now.day),
        Key::HourAmPm => hour_am_pm(now),
        Key::Hour => format!("{:02}", now.hour),
        Key::Minute => format!("{:02}", now.minute),
        Key::Second => format!("{:02}", now.second),
        Key::Msec => msec(now),
        Key::Date => exif_text(now),
        Key::ExifDateRegional => {
            format!("{:02}/{:02}/{}", taken.month, taken.day, year_short(&taken))
        }
        Key::ExifTimeRegional => {
            format!("{:02}:{:02}:{:02}", taken.hour, taken.minute, taken.second)
        }
        Key::ExifYearShort => year_short(&taken),
        Key::ExifYear => taken.year.to_string(),
        Key::ExifMonthShort => month_short(&taken).to_owned(),
        Key::ExifMonthLong => month_long(&taken).to_owned(),
        Key::ExifMonth => format!("{:02}", taken.month),
        Key::ExifDay => format!("{:02}", taken.day),
        Key::ExifHourAmPm => hour_am_pm(&taken),
        Key::ExifHour => format!("{:02}", taken.hour),
        Key::ExifMinute => format!("{:02}", taken.minute),
        Key::ExifSecond => format!("{:02}", taken.second),
        Key::ExifMsec => msec(&taken),
        Key::ExifDate => exif_text(&taken),
        Key::ExifIso => (values.iso as i32).to_string(),
        Key::ExposureBias => values
            .exposure_bias
            .map_or_else(String::new, |bias| format!("{:+.2}", f64::from(bias))),
        Key::ExposureProgram => values
            .exposure_program
            .map_or("", exposure_program)
            .to_owned(),
        Key::Exposure => {
            let exposure = exposure(values.exposure);
            // Every job but darktable's image information writes a fraction with "_".
            if values.jobcode == "infos" {
                exposure
            } else {
                exposure.replace('/', "_")
            }
        }
        Key::Aperture => fixed(f64::from(values.aperture), 1),
        Key::FocalLength => fixed(f64::from(values.focal_length), 1),
        Key::FocusDistance => {
            let distance = values.focus_distance;
            let distance = if distance.is_nan() { 0.0 } else { distance };
            fixed(f64::from(distance), 2)
        }
        Key::FlashIcon => match values.flash {
            Some(flash) if flash & 1 != 0 => "⚡".to_owned(),
            _ => String::new(),
        },
        Key::Flash => match values.flash {
            None => "no info",
            Some(flash) if flash & 1 != 0 => "yes",
            Some(_) => "no",
        }
        .to_owned(),
        Key::Metering => values.metering_mode.map_or("", metering_mode).to_owned(),
        Key::WhiteBalance => values.white_balance.map_or("", white_balance).to_owned(),
        Key::Longitude => values.longitude.map_or_else(String::new, longitude),
        Key::Latitude => values.latitude.map_or_else(String::new, latitude),
        Key::Elevation => fixed(values.elevation.unwrap_or(f64::NAN), 2),
        Key::LocationIcon => {
            if values.latitude.is_some() && values.longitude.is_some() {
                "📌".to_owned()
            } else {
                String::new()
            }
        }
        Key::Location => [
            values.latitude.map(latitude),
            values.longitude.map(longitude),
            values.elevation.map(|elevation| fixed(elevation, 2)),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", "),
        Key::Maker => values.maker.clone(),
        Key::Model => values.model.clone(),
        Key::Lens => values.lens.clone(),
        // `dt_image_print_exif`.
        Key::ImageExif => format!(
            "{} f/{} {}mm ISO {}",
            exposure(values.exposure),
            fixed(f64::from(values.aperture), 1),
            values.focal_length as i32,
            values.iso as i32
        ),
        Key::Jobcode => values.jobcode.clone(),
        Key::RollName => {
            let levels = match &variable.params {
                Params::Brackets(first, _) => match strtol(first).0 as i32 {
                    n if n > 0 => n as u8,
                    _ => 1,
                },
                _ => 1,
            };
            roll_name(&dirname(&values.path), levels.clamp(1, 5)).to_owned()
        }
        Key::FileFolder => dirname(&values.path),
        Key::ImageFilename => values.path.clone(),
        Key::FileName => {
            let mut name = basename(&values.path);
            if let Some(dot) = name.rfind('.') {
                name.truncate(dot);
            }
            name
        }
        // From the last "." of the whole path, as darktable finds it.
        Key::FileExtension => values
            .path
            .rfind('.')
            .map_or("", |dot| &values.path[dot + 1..])
            .to_owned(),
        Key::Sequence => sequence(&variable.params, values.sequence),
        Key::Username => values.user_name.clone(),
        Key::Home => values.home.clone(),
        Key::Pictures => values.pictures.clone(),
        Key::Desktop => values.desktop.clone().unwrap_or_default(),
        Key::MaxWidth => size(values.max, false),
        Key::MaxHeight => size(values.max, true),
        Key::SensorWidth => size(values.sensor, false),
        Key::SensorHeight => size(values.sensor, true),
        Key::RawWidth => size(values.raw, false),
        Key::RawHeight => size(values.raw, true),
        Key::CropWidth => size(values.crop, false),
        Key::CropHeight => size(values.crop, true),
        Key::ExportWidth => size(export_size(values), false),
        Key::ExportHeight => size(export_size(values), true),
        Key::ProgramName => "Apertrust".to_owned(),
        Key::ProgramVersion => values.version.clone(),
        Key::Creator => values.creator.clone().unwrap_or_default(),
        Key::Rights => values.rights.clone().unwrap_or_default(),
        Key::Description => values.description.clone().unwrap_or_default(),
        // D7: nothing here.
        Key::NewLine
        | Key::CropFactor
        | Key::FocalLengthEquivalent
        | Key::ImageIdNext
        | Key::ImageId
        | Key::VersionName
        | Key::VersionIfMulti
        | Key::Version
        | Key::Stars
        | Key::RatingIcons
        | Key::LabelsIcons
        | Key::Labels
        | Key::OpenCl
        | Key::WorkspaceLabel
        | Key::Category
        | Key::TagsHierarchy
        | Key::Tags
        | Key::SidecarTxt
        | Key::Title
        | Key::Publisher
        | Key::Notes
        | Key::ImageIdMetadata
        | Key::PreservedFileName => String::new(),
    }
}

/// C's `%.Nf`: `value` with `decimals` decimals, `nan` and `inf` as C writes them.
fn fixed(value: f64, decimals: usize) -> String {
    if value.is_nan() {
        "nan".to_owned()
    } else if value.is_infinite() {
        if value > 0.0 { "inf" } else { "-inf" }.to_owned()
    } else {
        format!("{value:.decimals$}")
    }
}

/// `$(SEQUENCE)`: `%.*u` of the photo's number plus the start less 1, with 4 digits and
/// a start of 1 unless the parameters say otherwise.
fn sequence(params: &Params, number: u32) -> String {
    let (mut digits, mut start) = (4_u8, 1_u32);
    match params {
        Params::Digit(digit) => digits = *digit,
        Params::Brackets(first, second) => {
            // darktable reads each into an int, the digits then into a byte.
            match strtol(first).0 as i32 {
                n if n > 0 => digits = n as u8,
                _ => {}
            }
            if let Some(second) = second {
                match strtol(second).0 as i32 {
                    n if n > 0 => start = n as u32,
                    _ => {}
                }
            }
        }
        Params::None => {}
    }
    let value = number.wrapping_add(start).wrapping_sub(1);
    // C's precision: no digit at all for 0 with none asked.
    if digits == 0 && value == 0 {
        String::new()
    } else {
        format!("{value:0width$}", width = usize::from(digits))
    }
}

/// `_init_expansion`'s export size: the size after every geometry, scaled in float to fit
/// the largest size, where that shrinks it or upscaling is allowed, rounded.
fn export_size(values: &Values) -> Size {
    let (max, crop) = (values.max, values.crop);
    if max.width == 0 && max.height == 0 {
        return crop;
    }
    let largest = |side: u32| {
        if side == 0 {
            i32::MAX as f32
        } else {
            side as f32
        }
    };
    let scale =
        (largest(max.height) / crop.height as f32).min(largest(max.width) / crop.width as f32);
    if scale < 1.0 || values.upscale {
        Size {
            width: (crop.width as f32 * scale).round() as u32,
            height: (crop.height as f32 * scale).round() as u32,
        }
    } else {
        crop
    }
}

/// `dt_util_format_exposure`: whole seconds with ″, fractions as 1/n.
fn exposure(time: f32) -> String {
    let reciprocal = 1.0 / f64::from(time);
    if time >= 1.0 {
        if time.round_ties_even() == time {
            format!("{}″", fixed(f64::from(time), 0))
        } else {
            format!("{}″", fixed(f64::from(time), 1))
        }
    } else if time < 0.29 || (1.0 / time).round_ties_even() == 1.0 / time {
        format!("1/{}", fixed(reciprocal, 0))
    } else if 10.0 * (10.0 / time).round_ties_even() == (100.0 / time).round_ties_even() {
        format!("1/{}", fixed(reciprocal, 1))
    } else {
        format!("{}″", fixed(f64::from(time), 1))
    }
}

/// `_variables_get_longitude`, outside darktable's image information: E or W and degrees.
fn longitude(longitude: f64) -> String {
    let side = if longitude < 0.0 { 'W' } else { 'E' };
    format!("{side}{:010.6}", longitude.abs())
}

/// `_variables_get_latitude`: N or S and degrees.
fn latitude(latitude: f64) -> String {
    let side = if latitude < 0.0 { 'S' } else { 'N' };
    format!("{side}{:09.6}", latitude.abs())
}

// glib's `g_date_time_format` in the C locale.

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

fn month_long(time: &DateTime) -> &'static str {
    MONTHS[usize::from(time.month.clamp(1, 12)) - 1]
}

/// `%b`.
fn month_short(time: &DateTime) -> &'static str {
    &month_long(time)[..3]
}

/// `%y`.
fn year_short(time: &DateTime) -> String {
    format!("{:02}", time.year.rem_euclid(100))
}

/// `%I %p`.
fn hour_am_pm(time: &DateTime) -> String {
    let hour = match time.hour % 12 {
        0 => 12,
        hour => hour,
    };
    let half = if time.hour < 12 { "AM" } else { "PM" };
    format!("{hour:02} {half}")
}

/// `%f` cut to its first three digits.
fn msec(time: &DateTime) -> String {
    format!("{:06}", time.microsecond)[..3].to_owned()
}

/// `dt_datetime_gdatetime_to_exif` without milliseconds, darktable's default
/// (`lighttable/ui/milliseconds`): `%Y:%m:%d %H:%M:%S`.
fn exif_text(time: &DateTime) -> String {
    format!(
        "{}:{:02}:{:02} {:02}:{:02}:{:02}",
        time.year, time.month, time.day, time.hour, time.minute, time.second
    )
}

// Exiv2 0.28's names for the EXIF values darktable shows (`_exif_value_str`): none for 0
// where darktable skips it, and none for a value Exiv2 has no name for.

/// ExposureProgram, which darktable skips at 0.
fn exposure_program(value: u16) -> &'static str {
    match value {
        1 => "Manual",
        2 => "Auto",
        3 => "Aperture priority",
        4 => "Shutter priority",
        5 => "Creative program",
        6 => "Action program",
        7 => "Portrait mode",
        8 => "Landscape mode",
        _ => "",
    }
}

/// MeteringMode, which darktable skips at 0.
fn metering_mode(value: u16) -> &'static str {
    match value {
        1 => "Average",
        2 => "Center weighted average",
        3 => "Spot",
        4 => "Multi-spot",
        5 => "Multi-segment",
        6 => "Partial",
        255 => "Other",
        _ => "",
    }
}

/// WhiteBalance.
fn white_balance(value: u16) -> &'static str {
    match value {
        0 => "Auto",
        1 => "Manual",
        _ => "",
    }
}

// glib's paths, as darktable cuts its image's path.

fn is_separator(c: char) -> bool {
    c == '/' || (cfg!(windows) && c == '\\')
}

/// `g_path_get_dirname`: the path to the last separator, without the separators before
/// it; "." where there is none.
pub(super) fn dirname(path: &str) -> String {
    let Some(last) = path.rfind(is_separator) else {
        return ".".to_owned();
    };
    let end = path[..last]
        .char_indices()
        .rev()
        .find(|(_, c)| !is_separator(*c))
        .map_or(1, |(i, c)| i + c.len_utf8());
    path[..end.max(1)].to_owned()
}

/// `g_path_get_basename`: what follows the last separator, the separators at the end
/// ignored; "." for empty text, a separator for separators alone.
pub(super) fn basename(path: &str) -> String {
    if path.is_empty() {
        return ".".to_owned();
    }
    let trimmed = path.trim_end_matches(is_separator);
    if trimmed.is_empty() {
        return std::path::MAIN_SEPARATOR.to_string();
    }
    let start = trimmed.rfind(is_separator).map_or(0, |i| i + 1);
    trimmed[start..].to_owned()
}

/// `dt_image_film_roll_name_levels`: the folder's last `levels` parts, or the whole path
/// where it has fewer.
fn roll_name(folder: &str, levels: u8) -> &str {
    let mut count = 0;
    for (i, c) in folder.char_indices().rev() {
        if i == 0 {
            break;
        }
        if is_separator(c) {
            count += 1;
            if count >= levels {
                return &folder[i + 1..];
            }
        }
    }
    folder
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_exposure_times_as_darktable() {
        assert_eq!(exposure(1.0 / 250.0), "1/250");
        assert_eq!(exposure(1.0 / 50.0), "1/50");
        assert_eq!(exposure(0.5), "1/2");
        assert_eq!(exposure(1.0 / 3.0), "1/3");
        assert_eq!(exposure(1.0 / 1.3), "1/1.3");
        // 10 / 0.4 and 100 / 0.4 round to 25 and 250 in float, so a tenth of a second
        // in the denominator, as darktable writes it.
        assert_eq!(exposure(0.4), "1/2.5");
        assert_eq!(exposure(0.7), "0.7″");
        assert_eq!(exposure(2.0), "2″");
        assert_eq!(exposure(2.5), "2.5″");
        assert_eq!(exposure(0.0), "1/inf");
    }

    #[test]
    fn cuts_paths_as_glib() {
        assert_eq!(
            dirname("/home/test/Images/IMG_0123.CR2"),
            "/home/test/Images"
        );
        assert_eq!(dirname("/a"), "/");
        assert_eq!(dirname("a//b"), "a");
        assert_eq!(dirname("/a/b/"), "/a/b");
        assert_eq!(dirname("name"), ".");
        assert_eq!(basename("/home/test/IMG.CR2"), "IMG.CR2");
        assert_eq!(basename("/a/b/"), "b");
        assert_eq!(basename(""), ".");
        assert_eq!(roll_name("/home/test/Images", 1), "Images");
        assert_eq!(roll_name("/home/test/Images", 2), "test/Images");
        assert_eq!(roll_name("/home/test/Images", 5), "/home/test/Images");
    }

    #[test]
    fn cuts_text_by_characters() {
        let name = "abcdef12345abcdef";
        assert_eq!(substring(name, -7, Some(-2)), "5abcd");
        assert_eq!(substring("äöü", 1, Some(1)), "ö");
        assert_eq!(substring("äöü", -1, None), "ü");
    }

    #[test]
    fn sizes_an_export_as_darktable_names_it() {
        let values = |max: Size, upscale: bool| Values {
            max,
            upscale,
            crop: Size {
                width: 6000,
                height: 4000,
            },
            ..Values::default()
        };
        let size = |width, height| Size { width, height };
        assert_eq!(export_size(&values(size(0, 0), false)), size(6000, 4000));
        assert_eq!(
            export_size(&values(size(2048, 2048), false)),
            size(2048, 1365)
        );
        assert_eq!(export_size(&values(size(0, 1000), false)), size(1500, 1000));
        assert_eq!(export_size(&values(size(9000, 0), false)), size(6000, 4000));
        assert_eq!(export_size(&values(size(9000, 0), true)), size(9000, 6000));
    }
}
