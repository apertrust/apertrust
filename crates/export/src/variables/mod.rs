// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2025  darktable developers (ported from src/common/variables.c)
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

//! darktable's variables (`src/common/variables.c`; `docs/ports/variables.md`),
//! which an export's path pattern holds: `$(FILE_FOLDER)/apertrust_exported/$(FILE_NAME)`.
//!
//! A [`Pattern`] is read whole before an export starts, so what darktable would read
//! past is refused at the field (D8): an unknown name, with the nearest one darktable
//! knows, and a variable it cannot close. Every name darktable knows is known here, its
//! legacy aliases among them, with darktable's operations (`$(VAR-default)`,
//! `$(VAR+alternative)`, `$(VAR:offset:length)`, `$(VAR#prefix)`, `$(VAR%suffix)`,
//! `$(VAR/pattern/replacement)` and its `//`, `/#` and `/%`, `$(VAR^)`, `$(VAR^^)`,
//! `$(VAR,)` and `$(VAR,,)`), their parameters (`$(SEQUENCE[n,m])`, `$(ROLL.NAME[n])`)
//! and backslash escapes. [`Pattern::expand`] then expands it for one photo from
//! [`Values`], as darktable's export does (job code "export").
//!
//! D7: `$(DARKTABLE.NAME)` and `$(DARKTABLE.VERSION)` read as Apertrust's, beside
//! `$(APERTRUST.NAME)` and `$(APERTRUST.VERSION)`; a variable Apertrust has no value for
//! expands to nothing, and [`Pattern::not_here`] says which and why, for the dialog's
//! line. [`list`] is darktable's list of variables with its descriptions, for the dialog's
//! `$(` list.

use std::ops::Range;

use thiserror::Error;

mod expand;
mod names;
mod parse;
#[cfg(test)]
mod tests;

pub use names::{Listed, NotHere, list};

use names::Key;

/// A path pattern, read: darktable's text with its variables, ready to expand.
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    parts: Vec<Part>,
}

/// Text, or a variable to expand.
#[derive(Debug, Clone, PartialEq)]
enum Part {
    Text(String),
    Variable(Box<Variable>),
}

#[derive(Debug, Clone, PartialEq)]
struct Variable {
    key: Key,
    /// The name as the pattern spells it.
    name: &'static str,
    params: Params,
    operation: Option<Operation>,
}

/// A variable's parameters after its name.
#[derive(Debug, Clone, PartialEq)]
enum Params {
    None,
    /// The digit of darktable's older forms, `$(SEQUENCE4)` and `$(CATEGORY0(…))`.
    Digit(u8),
    /// `[n]` or `[n,m]`, each as written: darktable reads them as numbers where it
    /// expands.
    Brackets(String, Option<String>),
}

/// darktable's operations on a variable's value, after the bash ones they follow.
#[derive(Debug, Clone, PartialEq)]
enum Operation {
    /// `$(VAR-text)`: the text where the value is empty.
    Default(Vec<Part>),
    /// `$(VAR+text)`: the text where the value is not empty.
    Alternative(Vec<Part>),
    /// `$(VAR:offset)` and `$(VAR:offset:length)`, in characters, counted from the end
    /// where negative.
    Substring { offset: i64, length: Option<i64> },
    /// `$(VAR#text)`: the value without the text it starts with.
    RemovePrefix(Vec<Part>),
    /// `$(VAR%text)`: the value without the text it ends with.
    RemoveSuffix(Vec<Part>),
    /// `$(VAR/pattern/replacement)` and its `//`, `/#` and `/%`.
    Replace {
        replace: Replace,
        pattern: Vec<Part>,
        replacement: Vec<Part>,
    },
    /// `$(VAR^)`, the first character in upper case, and `$(VAR^^)`, every one.
    Upper { all: bool },
    /// `$(VAR,)`, the first character in lower case, and `$(VAR,,)`, every one.
    Lower { all: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Replace {
    /// `/`: the first match.
    First,
    /// `//`: every match.
    All,
    /// `/#`: a match at the start.
    Prefix,
    /// `/%`: a match at the end.
    Suffix,
}

/// What stops a pattern (D8), at its place in the text, in bytes: where darktable
/// expands an unknown variable to nothing and leaves a variable it cannot close as it is.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PatternError {
    /// `$(NAME` where no variable is named NAME, with the nearest name, if one is near.
    #[error(
        "{}{}",
        if name.is_empty() { "a variable's name is missing after \"$(\"".to_owned() }
        else { format!("unknown variable $({})", shown(name)) },
        suggestion.map_or_else(String::new, |s| format!("; did you mean $({s})?"))
    )]
    Unknown {
        name: String,
        /// The name's place, to replace by the suggestion.
        at: Range<usize>,
        suggestion: Option<&'static str>,
    },
    /// A variable that is not closed: no `expected`, `)` or the `/` or `]` it takes, at
    /// `at`, where the pattern ends or something else comes.
    #[error("missing \"{expected}\" after {}", shown(variable))]
    Unclosed {
        /// The variable as far as it was read, from its `$(`.
        variable: String,
        expected: char,
        at: usize,
    },
    /// A character after a variable's name that is no operation darktable has.
    #[error("\"{found}\" after {variable} is no operation")]
    NoOperation {
        /// The variable's `$(` and name.
        variable: String,
        found: char,
        at: usize,
    },
    /// Variables nested deeper than [`MAX_DEPTH`], at the `$(` past it.
    #[error("variables are nested more than {MAX_DEPTH} deep")]
    TooDeep { at: usize },
}

/// How deep variables may nest in one another.
pub const MAX_DEPTH: usize = parse::MAX_DEPTH;

/// Text a message shows, at most 64 characters, cut with an ellipsis.
fn shown(text: &str) -> String {
    match text.char_indices().nth(64) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text.to_owned(),
    }
}

/// A variable that expands to nothing in Apertrust, as a pattern names it, and why
/// (D7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unavailable {
    pub name: &'static str,
    pub why: NotHere,
}

impl Pattern {
    /// Reads `text`, refusing an unknown variable and one not closed (D8).
    pub fn parse(text: &str) -> Result<Self, PatternError> {
        parse::pattern(text).map(|parts| Self { parts })
    }

    /// The pattern expanded for one photo of an export.
    pub fn expand(&self, values: &Values) -> String {
        expand::parts(&self.parts, values)
    }

    /// The variables it holds that Apertrust has no value for, in the order they come,
    /// each once, for the dialog's line saying they expand to nothing (D7).
    pub fn not_here(&self) -> Vec<Unavailable> {
        let mut found = Vec::new();
        collect(&self.parts, &mut found);
        found
    }
}

fn collect(parts: &[Part], found: &mut Vec<Unavailable>) {
    for part in parts {
        let Part::Variable(variable) = part else {
            continue;
        };
        if let Some(why) = variable.key.not_here()
            && !found.iter().any(|u| u.name == variable.name)
        {
            found.push(Unavailable {
                name: variable.name,
                why,
            });
        }
        match &variable.operation {
            Some(
                Operation::Default(parts)
                | Operation::Alternative(parts)
                | Operation::RemovePrefix(parts)
                | Operation::RemoveSuffix(parts),
            ) => collect(parts, found),
            Some(Operation::Replace {
                pattern,
                replacement,
                ..
            }) => {
                collect(pattern, found);
                collect(replacement, found);
            }
            _ => {}
        }
    }
}

/// A date and time as a clock shows it, with no time zone: darktable's `GDateTime` in
/// the zone it reads it in, local for the export's time, UTC for a photo's EXIF time,
/// which it takes as the camera's clock showed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DateTime {
    pub year: i32,
    /// 1 to 12.
    pub month: u8,
    /// 1 to 31.
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    /// 0 to 999999.
    pub microsecond: u32,
}

impl DateTime {
    /// darktable's origin, 0001-01-01 00:00:00, which a photo with no EXIF time takes.
    pub const ORIGIN: Self = Self {
        year: 1,
        month: 1,
        day: 1,
        hour: 0,
        minute: 0,
        second: 0,
        microsecond: 0,
    };
}

/// A width and a height, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

/// What variables expand to for one photo of an export: darktable's
/// `dt_variables_params_t` and the image's fields its `_init_expansion` reads.
///
/// Each EXIF value holds what darktable's image holds when the file records none: 0, or
/// `None` where darktable keeps it apart (`DT_EXIF_TAG_UNINITIALIZED`, -1, NaN).
#[derive(Debug, Clone, PartialEq)]
pub struct Values {
    /// darktable's job code, "export" for an export (`$(JOBCODE)`), which also decides
    /// how `$(EXIF.EXPOSURE)` writes a fraction.
    pub jobcode: String,
    /// The photo's number in the export, from 1 (`$(SEQUENCE)`).
    pub sequence: u32,
    /// When the export started, local time (`$(YEAR)` to `$(MSEC)`, `$(DATE)`).
    pub now: DateTime,
    /// The export's largest size, darktable's `max_width` and `max_height` as its storage
    /// is given them, 0 for none (`$(WIDTH.MAX)`), and whether it may upscale.
    pub max: Size,
    pub upscale: bool,
    /// Apertrust's version (`$(APERTRUST.VERSION)`, `$(DARKTABLE.VERSION)`).
    pub version: String,
    /// The person's login name and folders (`$(USERNAME)`, `$(FOLDER.HOME)`,
    /// `$(FOLDER.PICTURES)`, `$(FOLDER.DESKTOP)`); `None` for a desktop folder the
    /// system has not.
    pub user_name: String,
    pub home: String,
    pub pictures: String,
    pub desktop: Option<String>,
    /// The raw's path, darktable's `filename` (`$(FILE.FOLDER)`, `$(FILE.NAME)`,
    /// `$(FILE.EXTENSION)`, `$(ROLL.NAME)`).
    pub path: String,
    /// The camera's maker and its name as the file gives it, darktable's `camera_maker`
    /// and `camera_alias` (`$(MAKER)`, `$(MODEL)`).
    pub maker: String,
    pub model: String,
    /// The lens, darktable's `exif_lens` (`$(LENS)`).
    pub lens: String,
    /// When the photo was taken, the camera's clock; `None` where the file records none.
    pub taken: Option<DateTime>,
    /// The exposure time in seconds, the aperture's f-number, the focal length in mm,
    /// the focus distance in m and the ISO.
    pub exposure: f32,
    pub aperture: f32,
    pub focal_length: f32,
    pub focus_distance: f32,
    pub iso: f32,
    /// The exposure compensation in EV; `None` where the file records none.
    pub exposure_bias: Option<f32>,
    /// EXIF's ExposureProgram, MeteringMode, WhiteBalance and Flash tags, as numbers.
    pub exposure_program: Option<u16>,
    pub metering_mode: Option<u16>,
    pub white_balance: Option<u16>,
    pub flash: Option<u16>,
    /// The location in degrees, north and east positive, and the elevation in m.
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub elevation: Option<f64>,
    /// The metadata darktable's import takes from the EXIF: the creator from Artist, the
    /// rights from Copyright, the description from UserComment.
    pub creator: Option<String>,
    pub rights: Option<String>,
    pub description: Option<String>,
    /// The raw's whole readout (`$(WIDTH.SENSOR)`), the image raw preparation keeps
    /// (`$(WIDTH.RAW)`) and the image after every geometry, before the export's size
    /// (`$(WIDTH.CROP)`); the export's comes from it and `max` (`$(WIDTH.EXPORT)`).
    pub sensor: Size,
    pub raw: Size,
    pub crop: Size,
}

impl Default for Values {
    /// An export's, job code "export", of the first photo, and nothing of a photo.
    fn default() -> Self {
        Self {
            jobcode: "export".to_owned(),
            sequence: 1,
            now: DateTime::ORIGIN,
            max: Size::default(),
            upscale: false,
            version: String::new(),
            user_name: String::new(),
            home: String::new(),
            pictures: String::new(),
            desktop: None,
            path: String::new(),
            maker: String::new(),
            model: String::new(),
            lens: String::new(),
            taken: None,
            exposure: 0.0,
            aperture: 0.0,
            focal_length: 0.0,
            focus_distance: 0.0,
            iso: 0.0,
            exposure_bias: None,
            exposure_program: None,
            metering_mode: None,
            white_balance: None,
            flash: None,
            latitude: None,
            longitude: None,
            elevation: None,
            creator: None,
            rights: None,
            description: None,
            sensor: Size::default(),
            raw: Size::default(),
            crop: Size::default(),
        }
    }
}
