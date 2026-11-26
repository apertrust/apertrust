// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/libs/metadata_view.c and src/common/utility.c)
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

//! The Metadata tool's facts in words (decision 110; `docs/gui.md`, The Metadata tool): one
//! value for each row, from the session's `Photo`, its EXIF and its history, in the
//! language's words and numbers, with what its tooltip adds; `None` where the photo has no
//! value, which the row reads as "—".
//!
//! The exposure time follows the caption's rule (`crate::exposure`), darktable's
//! `dt_util_format_exposure`; the coordinates darktable's degrees and decimal minutes
//! (`dt_util_latitude_str`); a subject distance of 0xFFFFFF00 m or more reads Infinity, as
//! darktable's image information has it (`metadata_view.c:862-876`).

use std::path::Path;

use apertrust_engine::session::facts::focal_length_35mm;
use apertrust_engine::session::library::{DateTime, Shot};
use apertrust_engine::session::{Exif, Photo, size_in_unit};
use apertrust_engine::session::{History, StepSource};

use apertrust_ui::Translator;
use apertrust_ui::i18n::Args;

/// What a row shows: its value as shown, `None` for none, and what its tooltip says beside
/// it, where it says more.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shown {
    pub value: Option<String>,
    pub note: Option<String>,
}

impl Shown {
    fn value(value: impl Into<String>) -> Shown {
        Shown {
            value: Some(value.into()),
            note: None,
        }
    }

    fn noted(self, note: Option<String>) -> Shown {
        Shown { note, ..self }
    }
}

/// What the facts are read from: the active photo, its history and the person's home, which
/// a folder is shown under as `~`, and the words.
/// What a row is worded from: the photo open's, or one photo's facts as a summary holds
/// them ([`Of::shot`]), which have no path and no size on disk.
#[derive(Clone, Copy, Debug)]
pub struct Of<'a> {
    pub path: &'a Path,
    pub exif: &'a Exif,
    pub format: &'a str,
    pub file_size: u64,
    /// The developed size, and the sensor's readout.
    pub size: [u32; 2],
    pub sensor: [u32; 2],
    pub iso: Option<f32>,
    /// The camera's maker and model as the decoder spells them.
    pub make: &'a str,
    pub alias: &'a str,
}

impl<'a> Of<'a> {
    pub fn photo(photo: &'a Photo) -> Of<'a> {
        let camera = photo.image.camera.as_ref();
        Of {
            path: &photo.path,
            exif: &photo.exif,
            format: &photo.format,
            file_size: photo.file_size,
            size: [photo.width, photo.height],
            sensor: photo.sensor,
            iso: camera.map(|camera| camera.iso),
            make: camera.map_or("", |camera| camera.make.as_str()),
            alias: camera.map_or("", |camera| camera.alias.as_str()),
        }
    }

    pub fn shot(shot: &'a Shot) -> Of<'a> {
        Of {
            path: Path::new(""),
            exif: &shot.exif,
            format: &shot.format,
            file_size: 0,
            size: shot.size,
            sensor: shot.sensor,
            iso: shot.iso,
            make: &shot.make,
            alias: &shot.alias,
        }
    }
}

pub struct Source<'a> {
    pub of: Of<'a>,
    pub history: Option<&'a History>,
    pub home: Option<&'a Path>,
    pub words: &'a Translator,
}

impl Source<'_> {
    fn exif(&self) -> &Exif {
        self.of.exif
    }

    pub(super) fn say(&self, id: &str, args: &[(&str, String)]) -> String {
        say(self.words, id, args)
    }

    /// `value` with at most `decimals` decimals, its trailing zeros trimmed, in the
    /// language's numbers, signed where asked.
    fn trimmed(&self, value: f64, decimals: usize, signed: bool) -> String {
        let text = self.words.format_number(value, decimals, signed);
        let separator = self.words.decimal_separator();
        if !text.contains(separator) {
            return text;
        }
        let text = text.trim_end_matches('0');
        text.trim_end_matches(separator).to_owned()
    }

    fn number(&self, value: f64, decimals: usize) -> String {
        self.words.format_number(value, decimals, false)
    }

    /// A code's words, `metadata-<tag>-<code>`, or "Unknown (7)" for one the standard does
    /// not list.
    fn code(&self, tag: &str, code: u16) -> String {
        let id = format!("metadata-{tag}-{code}");
        if self.words.has(&id) {
            self.words.text(&id)
        } else {
            self.say("metadata-code-unknown", &[("code", code.to_string())])
        }
    }
}

/// The value of the row `id` of the photo in `source`, with what its tooltip adds.
pub fn fact(id: &str, source: &Source<'_>) -> Shown {
    let exif = source.exif();
    let photo = &source.of;
    let text = |value: &Option<String>| value.clone().map(Shown::value).unwrap_or_default();
    match id {
        "file_name" => match photo.path.file_name() {
            Some(name) => {
                Shown::value(name.to_string_lossy()).noted(Some(photo.path.display().to_string()))
            }
            None => Shown::default(),
        },
        "folder" => photo
            .path
            .parent()
            .map(|folder| Shown::value(crate::export::shown(folder, source.home)))
            .unwrap_or_default(),
        "type" => Shown::value(photo.format),
        "size_on_disk" => Shown::value(size(photo.file_size, source.words)),
        "dimensions" => dimensions(source),
        "edited" => edited(source),
        "make" => Shown::value(exif.make_written.as_deref().unwrap_or(photo.make)),
        "model" => Shown::value(exif.model_written.as_deref().unwrap_or(photo.alias)),
        "serial_number" => text(&exif.serial_number),
        "owner_name" => text(&exif.owner_name),
        "software" => text(&exif.software),
        "date_time" => exif
            .date_time
            .and_then(|at| naive(&at))
            .map(|at| Shown::value(source.words.format_date_time_to_second(at)))
            .unwrap_or_default(),
        "orientation" => exif
            .orientation_code
            .map(|code| Shown::value(source.code("orientation", code)))
            .unwrap_or_default(),
        "resolution" => exif
            .resolution
            .map(|r| {
                let id = match r.unit {
                    2 => "metadata-resolution-inches",
                    3 => "metadata-resolution-centimetres",
                    _ => "metadata-resolution",
                };
                let x = source.trimmed(f64::from(r.x), 2, false);
                let y = source.trimmed(f64::from(r.y), 2, false);
                Shown::value(source.say(id, &[("x", x), ("y", y)]))
            })
            .unwrap_or_default(),
        "artist" => text(&exif.artist),
        "copyright" => text(&exif.copyright),
        "image_description" => text(&exif.image_description),
        "date_taken" => dated(exif.taken, source),
        "date_digitised" => dated(exif.date_digitised, source),
        "exposure_time" => recorded(exif.exposure_time)
            .map(|seconds| {
                let value = crate::exposure::shutter(seconds, source.words);
                Shown::value(source.say("photo-shutter", &[("value", value)]))
            })
            .unwrap_or_default(),
        "aperture" => recorded(exif.aperture)
            .map(|f| Shown::value(f_number(f, source)))
            .unwrap_or_default(),
        "iso" => iso(source),
        "exposure_bias" => exif
            .exposure_bias
            .map(|ev| Shown::value(ev_value(source.trimmed(f64::from(ev), 2, true), source)))
            .unwrap_or_default(),
        "exposure_program" => coded(exif.exposure_program, "exposure_program", source),
        "exposure_mode" => coded(exif.exposure_mode, "exposure_mode", source),
        "metering_mode" => coded(exif.metering_mode, "metering_mode", source),
        "flash" => exif
            .flash
            .map(|bits| flash(bits, source))
            .unwrap_or_default(),
        "white_balance" => coded(exif.white_balance, "white_balance", source).noted(
            exif.light_source.map(|code| {
                let light = source.code("light_source", code);
                source.say("metadata-light-source", &[("source", light)])
            }),
        ),
        "scene_type" => coded(exif.scene_type, "scene_type", source),
        "subject_distance" => subject_distance(source),
        "digital_zoom" => exif
            .digital_zoom
            .map(|ratio| {
                Shown::value(if ratio == 0.0 {
                    source.words.text("metadata-digital-zoom-none")
                } else {
                    times(f64::from(ratio), 1, source)
                })
            })
            .unwrap_or_default(),
        "contrast" => coded(exif.contrast, "contrast", source),
        "saturation" => coded(exif.saturation, "saturation", source),
        "sharpness" => coded(exif.sharpness, "sharpness", source),
        "brightness" => exif
            .brightness
            .map(|ev| Shown::value(ev_value(source.trimmed(f64::from(ev), 2, false), source)))
            .unwrap_or_default(),
        "colour_space" => coded(exif.colour_space, "colour_space", source),
        "unique_image_id" => text(&exif.unique_image_id),
        "lens" => text(&exif.lens),
        "lens_make" => text(&exif.lens_make),
        "lens_serial_number" => text(&exif.lens_serial_number),
        "lens_specification" => exif
            .lens_specification
            .and_then(|spec| lens_specification(spec, source))
            .map(Shown::value)
            .unwrap_or_default(),
        "focal_length" => recorded(exif.focal_length)
            .map(|mm| Shown::value(millimetres(source.trimmed(f64::from(mm), 1, false), source)))
            .unwrap_or_default(),
        "focal_length_35mm" => focal_length_35mm(exif)
            .map(|mm| Shown::value(millimetres(source.number(f64::from(mm), 0), source)))
            .unwrap_or_default(),
        "crop_factor" => recorded(exif.crop_factor)
            .map(|crop| Shown::value(times(f64::from(crop), 2, source)))
            .unwrap_or_default(),
        "max_aperture" => recorded(exif.max_aperture)
            .map(|f| Shown::value(f_number(f, source)))
            .unwrap_or_default(),
        "latitude" => coordinate(exif.latitude, ["metadata-north", "metadata-south"], source),
        "longitude" => coordinate(exif.longitude, ["metadata-east", "metadata-west"], source),
        "altitude" => exif
            .elevation
            .map(|metres| {
                let id = if metres < 0.0 {
                    "metadata-altitude-below"
                } else {
                    "metadata-altitude-above"
                };
                let value = source.number(metres.abs(), 0);
                Shown::value(source.say(id, &[("value", value)]))
            })
            .unwrap_or_default(),
        "direction" => exif
            .gps_direction
            .map(|direction| {
                let id = match direction.magnetic {
                    Some(false) => "metadata-direction-true",
                    Some(true) => "metadata-direction-magnetic",
                    None => "metadata-direction",
                };
                let degrees = source.trimmed(direction.degrees, 1, false);
                Shown::value(source.say(id, &[("degrees", degrees)]))
            })
            .unwrap_or_default(),
        "gps_time" => exif
            .gps_time
            .and_then(|at| naive(&at))
            .map(|at| {
                let time = source.words.format_date_time_to_second(at);
                Shown::value(source.say("metadata-utc", &[("time", time)]))
            })
            .unwrap_or_default(),
        "speed" => exif
            .gps_speed
            .map(|speed| {
                let id = match speed.unit {
                    'M' => "metadata-speed-mph",
                    'N' => "metadata-speed-knots",
                    _ => "metadata-speed-kmh",
                };
                let value = source.trimmed(speed.value, 1, false);
                Shown::value(source.say(id, &[("value", value)]))
            })
            .unwrap_or_default(),
        "accuracy" => exif
            .gps_accuracy
            .map(|metres| Shown::value(metres_value(source.trimmed(metres, 1, false), source)))
            .unwrap_or_default(),
        "satellites" => text(&exif.gps_satellites),
        "map_datum" => text(&exif.gps_map_datum),
        _ => Shown::default(),
    }
}

/// The message `id` with `args`, in `words`.
pub(super) fn say(words: &Translator, id: &str, args: &[(&str, String)]) -> String {
    let mut fluent = Args::new();
    for (name, value) in args {
        fluent.set(*name, value.as_str());
    }
    words.text_with(id, Some(&fluent))
}

/// A value the photo records: more than 0, and a number.
fn recorded(value: Option<f32>) -> Option<f32> {
    value.filter(|v| v.is_finite() && *v > 0.0)
}

/// A clock's date and time, as chrono holds it.
pub(super) fn naive(at: &DateTime) -> Option<chrono::NaiveDateTime> {
    chrono::NaiveDate::from_ymd_opt(i32::from(at.year), u32::from(at.month), u32::from(at.day))?
        .and_hms_opt(
            u32::from(at.hour),
            u32::from(at.minute),
            u32::from(at.second),
        )
}

/// A size on disk, in the units disks count in: "32 MB".
pub fn size(bytes: u64, words: &Translator) -> String {
    let (size, decimals, unit) = size_in_unit(bytes);
    words.format_bytes(size, decimals, unit)
}

/// The developed size and its megapixels, "6000 × 4000 · 24.0 MP", the readout in the
/// tooltip where it differs, as darktable's "6000 (6048)".
fn dimensions(source: &Source<'_>) -> Shown {
    let photo = &source.of;
    let [developed_width, developed_height] = photo.size;
    let megapixels = f64::from(developed_width) * f64::from(developed_height) / 1e6;
    let value = source.say(
        "metadata-dimensions",
        &[
            ("width", developed_width.to_string()),
            ("height", developed_height.to_string()),
            ("megapixels", source.number(megapixels, 1)),
        ],
    );
    let [width, height] = photo.sensor;
    let differs = [width, height] != photo.size && width > 0;
    let note = differs.then(|| {
        source.say(
            "metadata-dimensions-sensor",
            &[("width", width.to_string()), ("height", height.to_string())],
        )
    });
    Shown::value(value).noted(note)
}

/// When the edit last changed and how many steps made it, "8 Nov 2026, 10:42 · 14 steps":
/// the last applied step's time, darktable's change timestamp, and the applied steps but the
/// default ones; Never with none.
fn edited(source: &Source<'_>) -> Shown {
    let Some(history) = source.history else {
        return Shown::default();
    };
    // The person's steps applied: those undone, and the defaults a new photo starts with,
    // are no edit.
    let applied = &history.steps[..history.end.min(history.steps.len())];
    let edits: Vec<_> = (applied.iter())
        .filter(|step| step.source != StepSource::Default)
        .collect();
    let Some(last) = edits.last() else {
        return Shown::value(source.words.text("metadata-edited-never"));
    };
    let mut args = Args::new();
    args.set("when", source.words.format_date_time(last.clock.when));
    args.set("steps", edits.len() as f64);
    Shown::value(source.words.text_with("metadata-edited", Some(&args)))
}

/// A date and time on the camera's clock, with its weekday and offset; the milliseconds, or
/// that no time zone is recorded, in the tooltip.
fn dated(at: Option<DateTime>, source: &Source<'_>) -> Shown {
    let Some(at) = at else {
        return Shown::default();
    };
    let Some(naive) = naive(&at) else {
        return Shown::default();
    };
    let value = source.words.format_long_date_time(naive, at.offset_minutes);
    let note = if at.millisecond > 0 {
        let time = format!(
            "{:02}:{:02}:{:02}{}{:03}",
            at.hour,
            at.minute,
            at.second,
            source.words.decimal_separator(),
            at.millisecond
        );
        Some(source.say("metadata-milliseconds", &[("time", time)]))
    } else if at.offset_minutes.is_none() {
        Some(source.words.text("metadata-no-zone"))
    } else {
        None
    };
    Shown::value(value).noted(note)
}

/// "f/5.6", one decimal, as darktable's `%.1f`.
fn f_number(f: f32, source: &Source<'_>) -> String {
    let value = source.number(f64::from(f), 1);
    source.say("photo-aperture", &[("value", value)])
}

/// The ISO by darktable's rule, the photo's image context's as the caption's, the
/// sensitivity type in the tooltip.
fn iso(source: &Source<'_>) -> Shown {
    let iso = source.of.iso;
    let Some(iso) = recorded(iso) else {
        return Shown::default();
    };
    let value = source.say("photo-iso", &[("value", source.number(f64::from(iso), 0))]);
    let note = source.exif().sensitivity_type.map(|code| {
        let kind = source.code("sensitivity_type", code);
        source.say("metadata-sensitivity", &[("type", kind)])
    });
    Shown::value(value).noted(note)
}

fn ev_value(value: String, source: &Source<'_>) -> String {
    source.say("unit-ev-value", &[("value", value)])
}

fn millimetres(value: String, source: &Source<'_>) -> String {
    source.say("metadata-millimetres", &[("value", value)])
}

fn metres_value(value: String, source: &Source<'_>) -> String {
    source.say("metadata-metres", &[("value", value)])
}

/// A ratio, "1.53×".
fn times(value: f64, decimals: usize, source: &Source<'_>) -> String {
    let value = source.trimmed(value, decimals, false);
    source.say("metadata-times", &[("value", value)])
}

/// A tag's code in words.
fn coded(code: Option<u16>, tag: &str, source: &Source<'_>) -> Shown {
    code.map(|code| Shown::value(source.code(tag, code)))
        .unwrap_or_default()
}

/// The flash from its bits (EXIF 3.0): its mode and whether it fired, red-eye reduction
/// where set, "Auto, fired, red-eye reduction"; the return light in the tooltip.
pub fn flash(bits: u16, source: &Source<'_>) -> Shown {
    if bits & 0x20 != 0 {
        return Shown::value(source.words.text("metadata-flash-no-function"));
    }
    let mode = match (bits >> 3) & 3 {
        1 => "on",
        2 => "off",
        3 => "auto",
        _ => "unknown",
    };
    let yes = |on: bool| if on { "yes" } else { "no" };
    let value = source.say(
        "metadata-flash",
        &[
            ("mode", mode.to_owned()),
            ("fired", yes(bits & 1 != 0).to_owned()),
            ("red_eye", yes(bits & 0x40 != 0).to_owned()),
        ],
    );
    let returned = (bits >> 1) & 3;
    let note = (returned != 0).then(|| source.code("flash_return", returned));
    Shown::value(value).noted(note)
}

/// The subject distance, "3.20 m", or Infinity for darktable's infinity; its range in the
/// tooltip.
fn subject_distance(source: &Source<'_>) -> Shown {
    let exif = source.exif();
    let note = exif.subject_distance_range.map(|code| {
        let range = source.code("subject_distance_range", code);
        source.say("metadata-subject-range", &[("range", range)])
    });
    let Some(metres) = recorded(exif.focus_distance) else {
        return Shown::default().noted(note);
    };
    let value = if metres >= 0xFFFF_FF00_u32 as f32 {
        source.words.text("metadata-infinity")
    } else {
        metres_value(source.number(f64::from(metres), 2), source)
    };
    Shown::value(value).noted(note)
}

/// A lens's specification, "24–70 mm f/2.8–4", a part the lens does not record (0) left
/// out; none where it records nothing.
fn lens_specification(spec: [f32; 4], source: &Source<'_>) -> Option<String> {
    let range = |low: f32, high: f32, decimals: usize| -> Option<String> {
        let part = |v: f32| source.trimmed(f64::from(v), decimals, false);
        match (low > 0.0, high > 0.0) {
            (true, true) if (low - high).abs() < f32::EPSILON => Some(part(low)),
            (true, true) => Some(format!("{}–{}", part(low), part(high))),
            (true, false) => Some(part(low)),
            (false, true) => Some(part(high)),
            (false, false) => None,
        }
    };
    let focal = range(spec[0], spec[1], 1).map(|mm| millimetres(mm, source));
    let aperture =
        range(spec[2], spec[3], 1).map(|f| source.say("photo-aperture", &[("value", f)]));
    match (focal, aperture) {
        (Some(focal), Some(aperture)) => Some(source.say(
            "metadata-lens-specification",
            &[("focal", focal), ("aperture", aperture)],
        )),
        (focal, aperture) => focal.or(aperture),
    }
}

/// A coordinate in darktable's degrees and decimal minutes to one decimal, "38° 42.6′ N",
/// with its decimal degrees in the tooltip; `ids` its words north or east, then south or
/// west.
fn coordinate(value: Option<f64>, ids: [&str; 2], source: &Source<'_>) -> Shown {
    let Some(value) = value.filter(|v| v.is_finite()) else {
        return Shown::default();
    };
    let id = if value < 0.0 { ids[1] } else { ids[0] };
    let whole = value.abs().trunc();
    let minutes = (value.abs() - whole) * 60.0;
    // Minutes that round up to 60 carry into the degrees.
    let (whole, minutes) = if (minutes * 10.0).round() >= 600.0 {
        (whole + 1.0, 0.0)
    } else {
        (whole, minutes)
    };
    let shown = source.say(
        id,
        &[
            ("degrees", format!("{whole:.0}")),
            ("minutes", source.number(minutes, 1)),
        ],
    );
    let decimal = source.number(value, 6);
    Shown::value(shown).noted(Some(decimal))
}

/// The coordinates as Copy Coordinates gives them, "38.710000, -9.130000": six decimals, a
/// full stop whatever the language, for a map or another app to read.
pub fn coordinates(exif: &Exif) -> Option<String> {
    let (latitude, longitude) = (exif.latitude?, exif.longitude?);
    Some(format!("{latitude:.6}, {longitude:.6}"))
}
