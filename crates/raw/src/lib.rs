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

//! Raw decoding through rawler, a raw decoder in Rust (architecture decision 23).
//!
//! [`decode_file`] returns the sensor data as the camera recorded it: uncropped, before
//! black subtraction and scaling, with the areas, levels, white balance and colour
//! matrices that raw preparation applies later. The types here are Apertrust's own, so
//! changes in rawler's API stay inside this crate.
//!
//! A photo that opens in stages (decision 77) maps its file once, as a [`RawFile`], and
//! reads from it its metadata without the pixels ([`RawFile::metadata`], in milliseconds),
//! its EXIF, its embedded picture, then the decode. A corpus test holds the metadata read
//! equal to the decode's but for the pixels.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::time::Instant;

use rawler::cfa::CFAColor;
use rawler::decoders::{Orientation as RawlerOrientation, RawDecodeParams, WellKnownIFD};
use rawler::imgop::xyz::Illuminant as RawlerIlluminant;
use rawler::rawimage::{RawImageData, RawPhotometricInterpretation};
use rawler::rawsource::RawSource;
use rawler::tags::{DngTag, ExifTag};

mod exif;
mod identity;
mod lens;
mod masked;
mod preview;

pub use exif::{DateTime, Exif};
pub use lens::{
    DngCorrection, FujifilmCorrection, LensCorrection, OlympusCorrection, SonyCorrection,
};
pub use preview::{PREVIEW_VERSION, Preview, read_jpeg, read_preview, read_preview_file};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("reading {}: {source}", path.display())]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("rawler: {0}")]
    Decode(String),
}

/// A decoded raw.
#[derive(Debug, Clone, PartialEq)]
pub struct RawImage {
    /// The camera's names, as rawler's camera definitions spell them.
    pub camera: Camera,
    /// Size of `pixels` in pixels: the whole readout, masked borders included.
    pub width: u32,
    pub height: u32,
    /// Values per pixel: 1 for a mosaic, 3 for a linear DNG or a demosaiced format.
    pub components: u32,
    /// Rows top to bottom, `width × components` values each.
    pub pixels: Pixels,
    /// The part of the readout holding image data; the whole readout if the decoder
    /// does not know it.
    pub active_area: Rect,
    /// The image the camera itself outputs, a few pixels inside the active area, as
    /// in a DNG's default crop; `None` if unknown.
    pub default_crop: Option<Rect>,
    /// Covered borders of the sensor, from which black can be measured.
    pub masked_areas: Vec<Rect>,
    /// The colour filter array, or `None` if the data is not a mosaic.
    pub cfa: Option<Cfa>,
    pub black_level: BlackLevel,
    /// White level per component.
    pub white_levels: Vec<u32>,
    /// The camera's as-shot white balance multipliers, one per CFA colour, green 1;
    /// `None` if the file records none.
    pub white_balance: Option<Vec<f32>>,
    /// XYZ to camera RGB matrices for the illuminants they were calibrated under.
    pub color_matrices: Vec<ColorMatrix>,
    /// A DNG's colour calibration tags, from which darktable derives the camera's D65
    /// matrix; `None` for a raw that is not a DNG.
    pub dng_color: Option<DngColor>,
    /// A DNG's LinearResponseLimit: the fraction of the white level up to which the
    /// sensor responds linearly; `None` if the file records none or is not a DNG.
    pub linear_response_limit: Option<f32>,
    /// How the photo is to be shown, from its EXIF Orientation tag as darktable reads
    /// it; `Normal` where it records none.
    pub orientation: Orientation,
    /// The exposure compensation set on the camera, in EV, from the EXIF data; `None`
    /// if the file records none.
    pub exposure_bias: Option<f32>,
    /// The ISO the photo was taken at, from the EXIF data by darktable's rule (decision
    /// 24); `None` if the file records none.
    pub iso: Option<f32>,
    /// The corrections the file records for its lens, as darktable reads them (decision
    /// 25); `None` where darktable finds none. Only Fujifilm's are read so far.
    pub lens_correction: Option<LensCorrection>,
}

/// The camera's names, meaning what darktable's do (decision 24).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Camera {
    /// The camera's maker, as darktable's `camera_maker`.
    pub make: String,
    /// The camera's model: for an alias, the camera it is another name for, as
    /// darktable's `camera_model`.
    pub model: String,
    /// The name the file's camera has, as darktable's `camera_alias`: `model`, unless
    /// the camera is an alias of another.
    pub alias: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pixels {
    U16(Vec<u16>),
    /// Floating-point DNGs, e.g. merged HDRs.
    F32(Vec<f32>),
}

/// A colour filter array, repeating from the top-left of the readout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cfa {
    pub width: u32,
    pub height: u32,
    /// Row by row.
    pub colors: Vec<CfaColor>,
}

impl Cfa {
    /// The filter colour at a position of the readout.
    pub fn color_at(&self, x: u32, y: u32) -> CfaColor {
        self.colors[((y % self.height) * self.width + x % self.width) as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CfaColor {
    Red,
    Green,
    Blue,
    Cyan,
    Magenta,
    Yellow,
    White,
    FujiGreen,
    Unknown,
}

/// Black levels repeating over the readout from its top-left, in a pattern of
/// `width × height` positions, usually 1×1, 2×2 or the CFA's size.
#[derive(Debug, Clone, PartialEq)]
pub struct BlackLevel {
    pub width: u32,
    pub height: u32,
    /// Row by row, one value per component at each position.
    pub values: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorMatrix {
    pub illuminant: Illuminant,
    /// Row by row, one row per camera colour.
    pub xyz_to_camera: Vec<[f32; 3]>,
}

/// A DNG's colour calibration tags as its first IFD records them, each `None` where the
/// file has no such tag. The three calibrations are numbered as their tags are, 1 to 3;
/// rawler's camera data keeps ColorMatrix1 and 2 by light source, without their order,
/// and none of the others.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DngColor {
    /// CalibrationIlluminant1 to 3: EXIF light source codes.
    pub calibration_illuminants: [Option<u16>; 3],
    /// ColorMatrix1 to 3: XYZ to camera colour under each light source, row by row.
    pub color_matrices: [Option<Vec<f32>>; 3],
    /// CameraCalibration1 to 3: this camera's deviation from its model's matrices, a
    /// square matrix on camera colour, row by row.
    pub camera_calibrations: [Option<Vec<f32>>; 3],
    /// AnalogBalance: the gain of each camera colour before the values were stored.
    pub analog_balance: Option<Vec<f32>>,
}

/// An EXIF light source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Illuminant {
    A,
    D50,
    D55,
    D65,
    D75,
    /// Another EXIF light source code.
    Other(u16),
}

/// How the image is to be displayed, as in EXIF: the transform that brings the
/// readout upright.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Normal,
    MirrorHorizontal,
    Rotate180,
    MirrorVertical,
    Transpose,
    Rotate90,
    Transverse,
    Rotate270,
    Unknown,
}

/// The extensions of the files rawler decodes, upper case and without the dot, as it lists
/// them: what an open dialog offers.
pub fn extensions() -> &'static [&'static str] {
    rawler::decoders::supported_extensions()
}

/// Decodes a raw file.
pub fn decode_file(path: impl AsRef<Path>) -> Result<RawImage, Error> {
    let path = path.as_ref();
    let start = Instant::now();
    let source = RawSource::new(path).map_err(|source| Error::Read {
        path: path.to_owned(),
        source,
    })?;
    let name = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into());
    decode_source(&source).inspect(|raw| log_decoded(&name, raw, start))
}

/// Decodes a raw file's contents.
pub fn decode(file: &[u8]) -> Result<RawImage, Error> {
    let start = Instant::now();
    decode_source(&RawSource::new_from_slice(file)).inspect(|raw| log_decoded("a raw", raw, start))
}

/// A raw file mapped once, for every reading a photo's open makes of it (decision 77):
/// its metadata, its EXIF, its embedded picture and its decode.
pub struct RawFile {
    source: RawSource,
    name: String,
}

impl RawFile {
    /// Maps the raw file at `path`; an error where it cannot be opened.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let source = RawSource::new(path).map_err(|source| Error::Read {
            path: path.to_owned(),
            source,
        })?;
        let name = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into());
        Ok(Self { source, name })
    }

    /// What [`RawFile::decode`] gives but the pixels, which are empty, of the kind the
    /// decode's are: rawler's decode with its pixels skipped, so in milliseconds.
    pub fn metadata(&self) -> Result<RawImage, Error> {
        let start = Instant::now();
        let raw = decode_raw(&self.source, true)?;
        let ms = start.elapsed().as_secs_f64() * 1e3;
        tracing::debug!(target: "perf", "read the metadata of {} in {ms:.1} ms", self.name);
        Ok(raw)
    }

    /// The shooting data its EXIF records, as [`read_exif_file`] reads it.
    pub fn exif(&self) -> Result<Exif, Error> {
        read_exif_source(&self.source)
    }

    /// Its embedded picture, as [`read_preview_file`] reads it.
    pub fn preview(&self, longest: u32) -> Result<Option<Preview>, Error> {
        preview::read_preview_source(&self.source, longest)
    }

    /// Decodes it, as [`decode_file`] does.
    pub fn decode(&self) -> Result<RawImage, Error> {
        let start = Instant::now();
        decode_source(&self.source).inspect(|raw| log_decoded(&self.name, raw, start))
    }
}

/// Reads a raw file's shooting data from its EXIF, as darktable reads it for its
/// variables, without decoding its image.
pub fn read_exif_file(path: impl AsRef<Path>) -> Result<Exif, Error> {
    let path = path.as_ref();
    let source = RawSource::new(path).map_err(|source| Error::Read {
        path: path.to_owned(),
        source,
    })?;
    read_exif_source(&source)
}

/// Reads a raw file's shooting data from its contents.
pub fn read_exif(file: &[u8]) -> Result<Exif, Error> {
    read_exif_source(&RawSource::new_from_slice(file))
}

fn read_exif_source(source: &RawSource) -> Result<Exif, Error> {
    // A parser bug on an odd file must not take the application down.
    let metadata = catch_unwind(AssertUnwindSafe(|| {
        let params = RawDecodeParams::default();
        rawler::get_decoder(source).and_then(|decoder| decoder.raw_metadata(source, &params))
    }))
    .map_err(|_| Error::Decode("rawler panicked reading the EXIF data".to_owned()))?
    .map_err(|e| Error::Decode(e.to_string()))?;
    Ok(exif::read(&metadata.exif))
}

/// The perf and raw domains' lines for a decoded raw.
fn log_decoded(name: &str, raw: &RawImage, start: Instant) {
    tracing::debug!(
        target: "perf",
        "decoded {name} in {:.0} ms",
        start.elapsed().as_secs_f64() * 1e3
    );
    let cfa = match &raw.cfa {
        Some(cfa) if cfa.width == 6 && cfa.height == 6 => "X-Trans".to_owned(),
        Some(cfa) => cfa.colors.iter().map(|color| color.letter()).collect(),
        None => "no mosaic".to_owned(),
    };
    // Values as a list, or one when they are all the same.
    fn values<T: PartialEq + std::fmt::Display>(values: &[T]) -> String {
        match values {
            [] => "none".to_owned(),
            [first, rest @ ..] if rest.iter().all(|v| v == first) => first.to_string(),
            _ => values
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" "),
        }
    }
    let area = raw.active_area;
    tracing::debug!(
        target: "raw",
        "{} {}: {}×{} {cfa}, active: {}×{} at {},{}",
        raw.camera.make,
        raw.camera.alias,
        raw.width,
        raw.height,
        area.width,
        area.height,
        area.x,
        area.y,
    );
    tracing::debug!(
        target: "raw",
        "levels: black {}, white {}",
        values(&raw.black_level.values),
        values(&raw.white_levels),
    );
    tracing::debug!(
        target: "raw",
        "as shot: white balance {}, bias {}, ISO {}",
        raw.white_balance.as_ref().map_or_else(
            || "none".to_owned(),
            |wb| wb.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>().join(" ")
        ),
        raw.exposure_bias
            .map_or_else(|| "unrecorded".to_owned(), |ev| format!("{ev:+.2} EV")),
        raw.iso
            .map_or_else(|| "unrecorded".to_owned(), |iso| iso.to_string()),
    );
    tracing::debug!(
        target: "raw",
        "lens corrections: {}",
        raw.lens_correction.as_ref().map_or("none", LensCorrection::name),
    );
}

fn decode_source(source: &RawSource) -> Result<RawImage, Error> {
    decode_raw(source, false)
}

/// The decode, or with `dummy` its metadata alone, the pixels left empty: rawler's decode
/// with its decompression skipped.
fn decode_raw(source: &RawSource, dummy: bool) -> Result<RawImage, Error> {
    // A decoder bug on an odd file must not take the application down.
    let raw = catch_unwind(AssertUnwindSafe(|| {
        rawler::global_loader().decode(source, &RawDecodeParams::default(), dummy)
    }))
    .map_err(|panic| {
        let message = panic
            .downcast_ref::<&str>()
            .map(|s| (*s).to_owned())
            .or_else(|| panic.downcast_ref::<String>().cloned())
            .unwrap_or_default();
        Error::Decode(format!("decoder panicked: {message}"))
    })?
    .map_err(|e| Error::Decode(e.to_string()))?;
    // The EXIF data is secondary: a file whose metadata fails to parse still decodes.
    let metadata = catch_unwind(AssertUnwindSafe(|| {
        let params = RawDecodeParams::default();
        rawler::get_decoder(source).and_then(|decoder| decoder.raw_metadata(source, &params))
    }));
    let exif = match metadata {
        Ok(Ok(metadata)) => Some(metadata.exif),
        Ok(Err(error)) => {
            tracing::debug!(target: "raw", "no EXIF: {error}");
            None
        }
        Err(_) => {
            tracing::warn!(target: "raw", "EXIF: rawler panicked reading it, ignored");
            None
        }
    };
    let exposure_bias = exif
        .as_ref()
        .and_then(|exif| exif.exposure_bias)
        .filter(|bias| bias.d != 0)
        .map(|bias| bias.n as f32 / bias.d as f32);
    // darktable reads the orientation from the EXIF Orientation tag, IFD0's or a
    // Panasonic raw's, which rawler's metadata reads from the same IFD
    // (`src/common/exif.cc`); rawler's decoders set it only for a DNG.
    let orientation = exif
        .as_ref()
        .and_then(|exif| exif.orientation)
        .map_or(raw.orientation, RawlerOrientation::from_u16);
    // So are a DNG's colour calibration, linear response limit and ISO tags.
    let (dng_color, linear_response_limit, second_iso_rating) =
        catch_unwind(AssertUnwindSafe(|| dng_tags(source)))
            .unwrap_or_else(|_| {
                tracing::warn!(target: "raw", "DNG tags: rawler panicked reading them, ignored");
                None
            })
            .map_or((None, None, None), |(color, limit, rating)| {
                (Some(color), limit, rating)
            });
    // So are the lens's corrections.
    let lens_correction = catch_unwind(AssertUnwindSafe(|| lens::read(source))).unwrap_or_else(|_| {
        tracing::warn!(target: "raw", "lens corrections: rawler panicked reading them, ignored");
        None
    });
    // The make as the file spells it, which rawler matched it by.
    let iso = exif
        .as_ref()
        .map(|exif| identity::iso(exif, second_iso_rating, raw.camera.make.trim_end()))
        .filter(|&iso| iso > 0.0);
    // rawler names an alias by its own clean name; darktable's model is the camera's.
    let alias = raw.clean_model.clone();
    let model = identity::main_model(&raw.camera.make, &raw.camera.model)
        .map_or_else(|| alias.clone(), str::to_owned);

    let rect = |r: rawler::imgop::Rect| Rect {
        x: r.p.x as u32,
        y: r.p.y as u32,
        width: r.d.w as u32,
        height: r.d.h as u32,
    };
    let (width, height) = (raw.width as u32, raw.height as u32);
    let cfa = match &raw.photometric {
        RawPhotometricInterpretation::Cfa(config) => {
            let pattern = &config.cfa;
            let colors = (0..pattern.height)
                .flat_map(|row| (0..pattern.width).map(move |col| (row, col)))
                .map(|(row, col)| cfa_color(pattern.cfa_color_at(row, col)))
                .collect();
            Some(Cfa {
                width: pattern.width as u32,
                height: pattern.height as u32,
                colors,
            })
        }
        RawPhotometricInterpretation::BlackIsZero | RawPhotometricInterpretation::LinearRaw => None,
    };
    // Multipliers the camera does not have are NaN.
    let colors = raw.wb_coeffs.iter().take_while(|v| !v.is_nan()).count();
    let white_balance = (colors >= 3 && raw.wb_coeffs[..colors].iter().all(|v| *v > 0.0))
        .then(|| raw.wb_coeffs[..colors].to_vec());
    let mut color_matrices: Vec<ColorMatrix> = raw
        .camera
        .color_matrix
        .iter()
        .map(|(illuminant, matrix)| ColorMatrix {
            illuminant: illuminant_from(*illuminant),
            xyz_to_camera: matrix.as_chunks::<3>().0.to_vec(),
        })
        .collect();
    color_matrices.sort_by_key(|m| m.illuminant);

    Ok(RawImage {
        camera: Camera {
            make: raw.clean_make.clone(),
            model,
            alias,
        },
        width,
        height,
        components: raw.cpp as u32,
        active_area: raw.active_area.map(rect).unwrap_or(Rect {
            x: 0,
            y: 0,
            width,
            height,
        }),
        default_crop: raw.crop_area.map(rect),
        masked_areas: raw.blackareas.iter().copied().map(rect).collect(),
        cfa,
        black_level: BlackLevel {
            width: raw.blacklevel.width as u32,
            height: raw.blacklevel.height as u32,
            values: raw.blacklevel.levels.iter().map(|v| v.as_f32()).collect(),
        },
        white_levels: raw.whitelevel.0.clone(),
        white_balance,
        color_matrices,
        dng_color,
        linear_response_limit,
        orientation: orientation_from(orientation),
        exposure_bias,
        iso,
        lens_correction,
        // A dummy decode's buffer holds no values, only its room, which is let go.
        pixels: match raw.data {
            RawImageData::Integer(_) if dummy => Pixels::U16(Vec::new()),
            RawImageData::Float(_) if dummy => Pixels::F32(Vec::new()),
            RawImageData::Integer(values) => Pixels::U16(values),
            RawImageData::Float(values) => Pixels::F32(values),
        },
    })
}

/// A DNG's colour calibration tags and its LinearResponseLimit, from its first IFD,
/// where darktable reads them (as exiv2's `Exif.Image` keys), and ISOSpeedRatings'
/// second value where its EXIF IFD holds two, which rawler's EXIF leaves out; `None` for
/// a raw that is not a DNG, the only one whose IFDs rawler shows.
fn dng_tags(source: &RawSource) -> Option<(DngColor, Option<f32>, Option<u16>)> {
    // Only rawler's DNG decoder exposes its IFDs; it takes every TIFF with a DNGVersion.
    let decoder = rawler::get_decoder(source).ok()?;
    let root = decoder.ifd(WellKnownIFD::Root).ok()??;
    root.get_entry(DngTag::DNGVersion)?;
    let values = |tag: DngTag| {
        root.get_entry(tag).map(|entry| {
            (0..entry.count() as usize)
                // exiv2 reads a rational with a zero denominator as 0.
                .map(|i| {
                    Some(entry.force_f32(i))
                        .filter(|v| v.is_finite())
                        .unwrap_or(0.0)
                })
                .collect()
        })
    };
    let illuminant = |tag: DngTag| {
        root.get_entry(tag)
            .map(|entry| u16::try_from(entry.force_u32(0)).unwrap_or(u16::MAX))
    };
    // darktable takes the limit only when the tag holds one value.
    let linear_response_limit = values(DngTag::LinearResponseLimit)
        .filter(|limit: &Vec<f32>| limit.len() == 1)
        .map(|limit| limit[0]);
    let color = DngColor {
        calibration_illuminants: [
            DngTag::CalibrationIlluminant1,
            DngTag::CalibrationIlluminant2,
            DngTag::CalibrationIlluminant3,
        ]
        .map(illuminant),
        color_matrices: [
            DngTag::ColorMatrix1,
            DngTag::ColorMatrix2,
            DngTag::ColorMatrix3,
        ]
        .map(values),
        camera_calibrations: [
            DngTag::CameraCalibration1,
            DngTag::CameraCalibration2,
            DngTag::CameraCalibration3,
        ]
        .map(values),
        analog_balance: values(DngTag::AnalogBalance),
    };
    let second_iso_rating = decoder
        .ifd(WellKnownIFD::Exif)
        .ok()
        .flatten()
        .and_then(|exif| {
            exif.get_entry(ExifTag::ISOSpeedRatings)
                .filter(|entry| entry.count() > 1)
                .map(|entry| entry.force_u16(1))
        });
    Some((color, linear_response_limit, second_iso_rating))
}

impl CfaColor {
    /// The colour's letter, as CFA patterns are written: R, G, B, C, M, Y, W.
    pub fn letter(self) -> char {
        match self {
            CfaColor::Red => 'R',
            CfaColor::Green | CfaColor::FujiGreen => 'G',
            CfaColor::Blue => 'B',
            CfaColor::Cyan => 'C',
            CfaColor::Magenta => 'M',
            CfaColor::Yellow => 'Y',
            CfaColor::White => 'W',
            CfaColor::Unknown => '?',
        }
    }
}

fn cfa_color(color: CFAColor) -> CfaColor {
    match color {
        CFAColor::RED => CfaColor::Red,
        CFAColor::GREEN => CfaColor::Green,
        CFAColor::BLUE => CfaColor::Blue,
        CFAColor::CYAN => CfaColor::Cyan,
        CFAColor::MAGENTA => CfaColor::Magenta,
        CFAColor::YELLOW => CfaColor::Yellow,
        CFAColor::WHITE => CfaColor::White,
        CFAColor::FUJI_GREEN => CfaColor::FujiGreen,
        _ => CfaColor::Unknown,
    }
}

fn illuminant_from(illuminant: RawlerIlluminant) -> Illuminant {
    match illuminant {
        RawlerIlluminant::A => Illuminant::A,
        RawlerIlluminant::D50 => Illuminant::D50,
        RawlerIlluminant::D55 => Illuminant::D55,
        RawlerIlluminant::D65 => Illuminant::D65,
        RawlerIlluminant::D75 => Illuminant::D75,
        other => Illuminant::Other(other as u16),
    }
}

fn orientation_from(orientation: RawlerOrientation) -> Orientation {
    match orientation {
        RawlerOrientation::Normal => Orientation::Normal,
        RawlerOrientation::HorizontalFlip => Orientation::MirrorHorizontal,
        RawlerOrientation::Rotate180 => Orientation::Rotate180,
        RawlerOrientation::VerticalFlip => Orientation::MirrorVertical,
        RawlerOrientation::Transpose => Orientation::Transpose,
        RawlerOrientation::Rotate90 => Orientation::Rotate90,
        RawlerOrientation::Transverse => Orientation::Transverse,
        RawlerOrientation::Rotate270 => Orientation::Rotate270,
        RawlerOrientation::Unknown => Orientation::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_a_file_that_is_not_a_raw() {
        let err = decode(b"not a raw file at all").unwrap_err();
        assert!(matches!(err, Error::Decode(_)), "{err}");
    }

    #[test]
    fn reports_a_missing_file() {
        let err = decode_file("no/such/file.ARW").unwrap_err();
        assert!(matches!(err, Error::Read { .. }), "{err}");
    }

    #[test]
    fn lists_the_extensions_it_decodes_in_upper_case() {
        let extensions = extensions();
        for raw in ["ARW", "CR2", "CR3", "DNG", "NEF", "ORF", "RAF", "RW2"] {
            assert!(extensions.contains(&raw), "{raw}");
        }
        assert!(extensions.iter().all(|e| *e == e.to_uppercase()));
    }
}
