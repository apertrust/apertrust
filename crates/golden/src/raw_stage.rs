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

//! Golden checks for the raw stage (architecture decision 21, the `golden-tests`
//! skill). darktable cannot switch demosaic off or export a mosaic, so raw-stage
//! operations are compared as a chain, with rawler's metadata written into darktable's
//! own parameters so that both sides share it.
//!
//! Demosaic's photosite-colour passthrough puts each photosite's value,
//! black-subtracted, scaled to the white point and white balanced, in its own colour's
//! channel, so darktable's raw stage can be read photosite by photosite:
//!
//! - the passthrough check needs no port: a CPU computation from rawler's decode
//!   matches darktable exactly, which confirms that darktable crops the same uncropped
//!   mosaic as rawler and that colorin's linear Rec2020 input profile is an identity;
//! - the raw-stage run compares the ported operations' output on the GPU, with their
//!   parameters written into darktable's history;
//! - the defaults check leaves rawprepare out of the history, recovers darktable's own
//!   crop, black levels and white point from its output, and compares them with ours.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use apertrust_plugin::ParamValues;
use apertrust_raw::{CfaColor, Pixels, RawImage};

use crate::darktable::{DarktableCli, LoaderRejected, REFERENCE_VERSION, Size};
use crate::pfm::Image;
use crate::render::cache_root;
use crate::xmp::{
    all_params, disable_operation, operations, remove_operation, set_params, set_params_and_version,
};

/// Why a raw cannot be checked at all, which is not a failure in itself. Every other
/// error is a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipKind {
    /// A linear or monochrome raw, or a sensor with filters other than red, green and
    /// blue.
    NotColourMosaic,
    /// A mosaic the engine cannot upload, such as one of floating-point values.
    Not16Bit,
    /// rawler cannot decode the file.
    Undecodable,
    /// darktable-cli's loader rejects the file ([`crate::darktable::LoaderRejected`]).
    DarktableCannotLoad,
}

impl std::fmt::Display for SkipKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotColourMosaic => "not a colour mosaic",
            Self::Not16Bit => "not a 16-bit mosaic",
            Self::Undecodable => "rawler cannot decode it",
            Self::DarktableCannotLoad => "darktable-cli cannot load it",
        })
    }
}

/// A raw that cannot be checked: why, and what the checker found.
#[derive(Debug)]
pub struct Skip {
    pub kind: SkipKind,
    pub detail: String,
}

impl std::fmt::Display for Skip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.detail.is_empty() {
            write!(f, "{}", self.kind)
        } else {
            write!(f, "{}: {}", self.kind, self.detail)
        }
    }
}

impl std::error::Error for Skip {}

/// A [`Skip`] of this kind, as an error.
pub fn skip(kind: SkipKind, detail: impl Into<String>) -> anyhow::Error {
    Skip {
        kind,
        detail: detail.into(),
    }
    .into()
}

/// The raw decoded by rawler; a file rawler cannot decode is a [`Skip`], a missing or
/// unreadable one an error.
pub fn decode(image: &Path) -> Result<RawImage> {
    ensure!(image.is_file(), "no file {}", image.display());
    apertrust_raw::decode_file(image).map_err(|error| match error {
        apertrust_raw::Error::Decode(reason) => skip(SkipKind::Undecodable, reason),
        // Its message already ends with the I/O error's, which a chain would repeat.
        other => anyhow::anyhow!("{other}"),
    })
}

/// darktable's history for test 0174 (`mire1.cr2`), whose raw-stage entries the check
/// rewrites. Nothing in it is specific to the image.
const BASE_XMP: &str = include_str!("../cases/agx-default.xmp");

/// The modules that stay on: the raw stage, colorin, and what output needs.
const KEEP: &[&str] = &[
    "rawprepare",
    "temperature",
    "demosaic",
    "colorin",
    "colorout",
    "gamma",
];

/// darktable `DT_IOP_DEMOSAIC_PASSTHROUGH_COLOR` and `DT_IOP_DEMOSAIC_PASSTHR_COLORX`.
const PASSTHROUGH_BAYER: u32 = 4;
const PASSTHROUGH_XTRANS: u32 = 1024 | 5;
/// darktable `DT_COLORSPACE_LIN_REC2020`.
const LINEAR_REC2020: i32 = 4;
/// darktable `DT_IOP_TEMP_USER`: coefficients as given, no late correction.
const TEMPERATURE_USER: i32 = 2;
/// darktable `DT_COLORSPACE_EMBEDDED_MATRIX` and `DT_COLORSPACE_STANDARD_MATRIX`: the
/// file's D65 matrix, which darktable prefers for DNGs, and its own camera table's.
pub const EMBEDDED_MATRIX: i32 = 10;
pub const STANDARD_MATRIX: i32 = 11;

/// The metadata both sides use, from rawler.
#[derive(Debug, Clone, PartialEq)]
pub struct Metadata {
    /// The active area: x, y, width, height.
    pub crop: [u32; 4],
    /// Black level per darktable position: row parity × 2 + column parity, on the
    /// sensor.
    pub black: [u16; 4],
    pub white: u16,
    /// White balance multipliers for red, green and blue, green 1.
    pub coefficients: [f32; 3],
    /// Colour per CFA position, 0 red, 1 green, 2 blue.
    pub cfa: (u32, u32, Vec<u32>),
}

impl Metadata {
    /// The metadata of a colour mosaic; anything else, a linear or monochrome raw or a
    /// sensor with filters other than red, green and blue, is a [`Skip`].
    pub fn of(raw: &RawImage) -> Result<Self> {
        let cfa = raw
            .cfa
            .as_ref()
            .ok_or_else(|| skip(SkipKind::NotColourMosaic, "no colour filter array"))?;
        let colors = cfa
            .colors
            .iter()
            .map(|color| match color {
                CfaColor::Red => Ok(0),
                CfaColor::Green => Ok(1),
                CfaColor::Blue => Ok(2),
                other => Err(skip(
                    SkipKind::NotColourMosaic,
                    format!("CFA colour {other:?}"),
                )),
            })
            .collect::<Result<Vec<u32>>>()?;
        let black = &raw.black_level;
        ensure!(
            !black.values.is_empty() && black.width > 0 && black.height > 0,
            "no black level"
        );
        let black_at = |x: u32, y: u32| {
            black.values[((y % black.height) * black.width + x % black.width) as usize]
        };
        let wb = raw
            .white_balance
            .as_ref()
            .context("no as-shot white balance")?;
        ensure!(
            wb.len() >= 3 && wb[1] > 0.0,
            "unusable white balance {wb:?}"
        );
        let area = raw.active_area;
        Ok(Self {
            crop: [area.x, area.y, area.width, area.height],
            black: [0, 1, 2, 3].map(|id| black_at(id & 1, id >> 1).round() as u16),
            white: *raw.white_levels.first().context("no white level")? as u16,
            coefficients: [wb[0] / wb[1], 1.0, wb[2] / wb[1]],
            cfa: (cfa.width, cfa.height, colors),
        })
    }

    pub fn is_xtrans(&self) -> bool {
        self.cfa.0 == 6 && self.cfa.1 == 6
    }

    /// darktable's rawprepare parameters for this metadata.
    pub fn rawprepare(&self, raw: &RawImage) -> Vec<u8> {
        let [x, y, width, height] = self.crop;
        rawprepare_params(
            [x, y, raw.width - x - width, raw.height - y - height],
            self.black,
            self.white,
        )
    }

    /// The passthrough check's raw stage: rawler's metadata in rawprepare and
    /// temperature.
    pub fn passthrough(&self, raw: &RawImage) -> RawStage {
        RawStage::passthrough(
            Some(self.rawprepare(raw)),
            Some(self.coefficients),
            self.is_xtrans(),
        )
    }

    /// The colour of the photosite at (x, y) on the sensor, 0 red, 1 green, 2 blue.
    pub fn color_at(&self, x: u32, y: u32) -> u32 {
        let (width, height, colors) = &self.cfa;
        colors[((y % height) * width + x % width) as usize]
    }
}

/// What a sidecar sets in darktable's raw stage.
#[derive(Debug, Clone, PartialEq)]
pub struct RawStage {
    /// rawprepare's params in darktable's version 2 layout; `None` leaves rawprepare
    /// out of the history, so darktable uses its own defaults for the image.
    pub rawprepare: Option<Vec<u8>>,
    /// Temperature's coefficients for red, green and blue; `None` leaves temperature
    /// out of the history, so darktable uses its own defaults.
    pub temperature: Option<[f32; 3]>,
    /// Whether the sensor has an X-Trans filter, which needs its own passthrough.
    pub xtrans: bool,
    /// darktable's demosaicing method, its enum's value; `None` for the photosite
    /// colour passthrough.
    pub demosaic: Option<u32>,
    /// Temperature's preset, darktable's value: "user modified" for the passthrough, so
    /// colorin applies no late correction; Apertrust's own for `raw-run` and the
    /// pipeline, "as shot to reference" by default (decision 27), so colorin corrects.
    pub temperature_preset: i32,
    /// colorin's input profile, darktable's value; `None` for linear Rec2020, an
    /// identity.
    pub colorin: Option<i32>,
    /// Highlight reconstruction's method, darktable's value, and its clipping
    /// threshold; `None` leaves highlight reconstruction off.
    pub highlights: Option<(i32, f32)>,
}

impl RawStage {
    /// The raw stage with these parameters, darktable's demosaic passthrough and
    /// colorin's identity.
    pub fn passthrough(
        rawprepare: Option<Vec<u8>>,
        temperature: Option<[f32; 3]>,
        xtrans: bool,
    ) -> Self {
        Self {
            rawprepare,
            temperature,
            xtrans,
            demosaic: None,
            temperature_preset: TEMPERATURE_USER,
            colorin: None,
            highlights: None,
        }
    }
}

/// darktable's rawprepare v2 params: crop margins (left, top, right, bottom as i32),
/// the black level per parity position and the white point (u16), then flat field off.
pub fn rawprepare_params(margins: [u32; 4], black: [u16; 4], white: u16) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(32);
    for margin in margins {
        bytes.extend_from_slice(&(margin as i32).to_le_bytes());
    }
    for black in black {
        bytes.extend_from_slice(&black.to_le_bytes());
    }
    bytes.extend_from_slice(&white.to_le_bytes());
    // Padding, then the flat-field enum.
    bytes.extend_from_slice(&[0; 6]);
    bytes
}

/// darktable's rawprepare params for Apertrust's rawprepare parameters.
pub fn rawprepare_from(values: &ParamValues) -> Vec<u8> {
    let level = |name: &str| values.int(name) as u16;
    rawprepare_params(
        ["left", "top", "right", "bottom"].map(|name| values.int(name) as u32),
        apertrust_operations::rawprepare::BLACK.map(level),
        level("raw_white_point"),
    )
}

/// darktable's history with only the raw stage, demosaic's photosite-colour
/// passthrough and an identity colorin left on, and `stage` written into rawprepare,
/// temperature, highlight reconstruction, demosaic and colorin.
pub fn passthrough_xmp(stage: &RawStage) -> Result<String> {
    history(stage, &[])
}

/// The base history's custom module order, darktable's test's, puts finalscale after
/// colorout, as darktable's v3.0 order does, so an export at a size would resample the
/// output profile's encoded values; this moves it before colorout, darktable's
/// v5.0 place for a new edit, where Apertrust resamples (decision 56;
/// `docs/ports/finalscale.md`). Nothing else in the order changes, and an export at full
/// size runs no finalscale.
pub fn finalscale_before_colorout(xmp: &str) -> Result<String> {
    const V30: &str = "colorout,0,clahe,0,finalscale,0,";
    const V50: &str = "finalscale,0,colorout,0,clahe,0,";
    ensure!(
        xmp.matches(V30).count() == 1,
        "the history's module order has no finalscale after colorout and clahe"
    );
    Ok(xmp.replace(V30, V50))
}

/// darktable's history for the whole pipeline, Phase 1's exit test (decisions 22, 23 and 62): `stage` as [`passthrough_xmp`] writes it, then exposure, color
/// calibration and AgX from Apertrust's parameters, and output colour. Orientation stays
/// on at the base history's auto, darktable's default, which turns the raw as its EXIF
/// says, as Apertrust's default does (decision 16).
///
/// Exposure is written as darktable's version 7, whole, so darktable does not
/// migrate it: manual mode, Apertrust's `black` and effective `exposure`, and both of
/// darktable's compensations off. The camera's exposure bias is already in `exposure`,
/// so the two never read the EXIF differently. Highlight-preservation compensation,
/// which darktable switches on for a new instance, is not ported, since rawler does not
/// read the maker-note fields it needs (decision 21), so the test switches it off on
/// darktable's side; a version 6 entry would do the same silently, as darktable's
/// migration to version 7 sets it to false.
///
/// `channel_mixer` is color calibration's version 3 params
/// ([`crate::ports::channel_mixer_params_v3`]), which every colour raw runs (decision
/// 27) from darktable's own defaults restated, the camera's illuminant under CAT16
/// snapped to the daylight or Planckian locus where darktable's `reload_defaults` snaps
/// it. It replaces the params of the base history's entry, darktable's scene-referred
/// preset, which stays on. The camera's illuminant is only right with temperature at
/// "as shot to reference", which `stage` carries from Apertrust's defaults.
///
/// `agx` is AgX's version 7 params ([`crate::ports::agx_params_v7`]), replacing the base
/// history's version 4 entry, which darktable turns into its defaults whatever it holds.
pub fn pipeline_xmp(
    stage: &RawStage,
    black: f32,
    exposure: f32,
    channel_mixer: &[u8],
    agx: &[u8],
) -> Result<String> {
    let xmp = history(stage, &["exposure", "channelmixerrgb", "agx", "flip"])?;
    let xmp = set_params_and_version(&xmp, "exposure", 7, &exposure_params_v7(black, exposure))?;
    ensure!(
        channel_mixer.len() == 160,
        "channelmixerrgb v3 params of {} bytes",
        channel_mixer.len()
    );
    let xmp = set_params_and_version(&xmp, "channelmixerrgb", 3, channel_mixer)?;
    ensure!(agx.len() == 144, "agx v7 params of {} bytes", agx.len());
    set_params_and_version(&xmp, "agx", 7, agx)
}

/// darktable `dt_iop_exposure_params_t` version 7 in manual mode: mode, black,
/// exposure, deflicker percentile and target level at darktable's defaults, then the
/// flags to compensate the camera's exposure bias and highlight preservation, both off.
fn exposure_params_v7(black: f32, exposure: f32) -> Vec<u8> {
    const MANUAL: i32 = 0;
    [
        MANUAL.to_le_bytes(),
        black.to_le_bytes(),
        exposure.to_le_bytes(),
        50.0f32.to_le_bytes(),
        (-4.0f32).to_le_bytes(),
        0i32.to_le_bytes(),
        0i32.to_le_bytes(),
    ]
    .concat()
}

/// The base history with every module off but [`KEEP`]'s, `also`'s and highlight
/// reconstruction when `stage` sets it, and `stage` written in.
fn history(stage: &RawStage, also: &[&str]) -> Result<String> {
    let mut xmp = BASE_XMP.to_owned();
    for operation in operations(BASE_XMP) {
        let kept = KEEP.contains(&operation)
            || also.contains(&operation)
            || (operation == "highlights" && stage.highlights.is_some());
        if !kept {
            xmp = disable_operation(&xmp, operation)?;
        }
    }
    // highlights v4: the method is the first field and the clipping threshold the
    // fifth; the others are darktable's defaults, which inpaint opposed ignores.
    if let Some((mode, clip)) = stage.highlights {
        xmp = patch(&xmp, "highlights", 4, 0, &mode.to_le_bytes())?;
        xmp = patch(&xmp, "highlights", 4, 16, &clip.to_le_bytes())?;
    }

    xmp = match &stage.rawprepare {
        Some(params) => set_versioned(&xmp, "rawprepare", 2, params.clone())?,
        None => remove_operation(&xmp, "rawprepare")?,
    };

    // temperature v4: red, green, blue and a fourth coefficient, then the preset:
    // "user modified" for the passthrough, so colorin applies no late correction, or
    // Apertrust's for `raw-run` and the pipeline, "as shot to reference" by default, so
    // colorin corrects.
    xmp = match stage.temperature {
        Some(coefficients) => {
            let mut temperature = Vec::with_capacity(20);
            for coefficient in coefficients.into_iter().chain([0.0]) {
                temperature.extend_from_slice(&coefficient.to_le_bytes());
            }
            temperature.extend_from_slice(&stage.temperature_preset.to_le_bytes());
            set_versioned(&xmp, "temperature", 4, temperature)?
        }
        None => remove_operation(&xmp, "temperature")?,
    };

    // demosaic v5: the method is the fourth field.
    let method = match stage.demosaic {
        Some(method) => method,
        None if stage.xtrans => PASSTHROUGH_XTRANS,
        None => PASSTHROUGH_BAYER,
    };
    xmp = patch(&xmp, "demosaic", 5, 12, &method.to_le_bytes())?;
    // colorin v7: the input profile's type is the first field.
    let input_profile = stage.colorin.unwrap_or(LINEAR_REC2020);
    xmp = patch(&xmp, "colorin", 7, 0, &input_profile.to_le_bytes())?;
    Ok(xmp)
}

/// Writes `bytes` as the params of every `operation` entry, which must have `version`
/// and the same size.
fn set_versioned(xmp: &str, operation: &str, version: u32, bytes: Vec<u8>) -> Result<String> {
    for params in all_params(xmp, operation)? {
        ensure!(
            params.version == version && params.bytes.len() == bytes.len(),
            "{operation} v{} has {} bytes, expected v{version} with {}",
            params.version,
            params.bytes.len(),
            bytes.len()
        );
    }
    set_params(xmp, operation, &bytes)
}

/// Overwrites `bytes` at `offset` in every `operation` entry's params.
fn patch(xmp: &str, operation: &str, version: u32, offset: usize, bytes: &[u8]) -> Result<String> {
    let mut params = all_params(xmp, operation)?
        .into_iter()
        .next()
        .with_context(|| format!("no history entry for {operation}"))?;
    ensure!(
        params.version == version && params.bytes.len() >= offset + bytes.len(),
        "{operation} is v{}, expected v{version}",
        params.version
    );
    params.bytes[offset..offset + bytes.len()].copy_from_slice(bytes);
    set_versioned(xmp, operation, version, params.bytes)
}

/// What the passthrough should render: each photosite of the active area,
/// `(raw - black) / (white - black)` times its colour's coefficient, in its colour's
/// channel, as darktable's rawprepare, temperature and passthrough do.
pub fn expected_passthrough(raw: &RawImage, metadata: &Metadata) -> Result<Image> {
    let Pixels::U16(values) = &raw.pixels else {
        return Err(skip(SkipKind::Not16Bit, "floating-point values"));
    };
    let [x0, y0, width, height] = metadata.crop;
    let white = f32::from(metadata.white);
    let mut pixels = Vec::with_capacity(width as usize * height as usize);
    for y in y0..y0 + height {
        for x in x0..x0 + width {
            let black = f32::from(metadata.black[(((y & 1) << 1) | (x & 1)) as usize]);
            let value = f32::from(values[(y * raw.width + x) as usize]);
            let color = metadata.color_at(x, y) as usize;
            let mut pixel = [0.0; 3];
            pixel[color] = (value - black) / (white - black) * metadata.coefficients[color];
            pixels.push(pixel);
        }
    }
    Image::new(width as usize, height as usize, pixels)
}

/// A mosaic of `width × height` photosites from (x0, y0) on the sensor as darktable's
/// passthrough shows it: each photosite in its colour's channel.
pub fn mosaic_image(
    values: &[f32],
    width: u32,
    height: u32,
    origin: (u32, u32),
    metadata: &Metadata,
) -> Result<Image> {
    let pixels = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let mut pixel = [0.0; 3];
            let color = metadata.color_at(x + origin.0, y + origin.1) as usize;
            pixel[color] = values[(y * width + x) as usize];
            pixel
        })
        .collect();
    Image::new(width as usize, height as usize, pixels)
}

/// darktable's rawprepare defaults for an image, recovered from its passthrough render
/// with temperature's coefficients at 1: each photosite there is
/// `(raw - black) / (white - black)` with its parity position's black, so a straight
/// line through the raw values gives black and white, and the crop is where the lines
/// fit best.
#[derive(Debug, Clone, PartialEq)]
pub struct Recovered {
    /// x, y, width, height on the sensor.
    pub crop: [u32; 4],
    /// Per parity position.
    pub black: [f64; 4],
    pub white: [f64; 4],
    /// Root-mean-square distance of darktable's values from the fitted lines.
    pub rms: f64,
}

pub fn recover_defaults(raw: &RawImage, actual: &Image) -> Result<Recovered> {
    let Pixels::U16(values) = &raw.pixels else {
        return Err(skip(SkipKind::Not16Bit, "floating-point values"));
    };
    let (width, height) = (actual.width as u32, actual.height as u32);
    ensure!(
        width <= raw.width && height <= raw.height,
        "darktable's image, {width}×{height}, is larger than the sensor"
    );
    let raw_at = |x: u32, y: u32| f64::from(values[(y * raw.width + x) as usize]);
    let developed = |x: u32, y: u32| {
        let pixel = actual.pixels[y as usize * actual.width + x as usize];
        f64::from(pixel[0]) + f64::from(pixel[1]) + f64::from(pixel[2])
    };
    // Every crop position, on a sample of photosites; then the best one on all of them.
    let fit = |left: u32, top: u32, step: u32| {
        let mut sums = [Sums::default(); 4];
        for y in (0..height).step_by(step as usize) {
            for x in (0..width).step_by(step as usize) {
                let (sx, sy) = (x + left, y + top);
                sums[(((sy & 1) << 1) | (sx & 1)) as usize].add(raw_at(sx, sy), developed(x, y));
            }
        }
        sums
    };
    let step = ((u64::from(width) * u64::from(height) / 4096).isqrt() as u32 | 1).max(1);
    let mut best = (0, 0, f64::INFINITY);
    for top in 0..=raw.height - height {
        for left in 0..=raw.width - width {
            let rms = rms(&fit(left, top, step));
            if rms < best.2 {
                best = (left, top, rms);
            }
        }
    }
    let (left, top, _) = best;
    let sums = fit(left, top, 1);
    let lines = sums.map(|s| s.line());
    Ok(Recovered {
        crop: [left, top, width, height],
        black: lines.map(|(slope, intercept)| -intercept / slope),
        white: lines.map(|(slope, intercept)| (1.0 - intercept) / slope),
        rms: rms(&sums),
    })
}

/// darktable's display-referred workflow, in which temperature's default is the
/// as-shot white balance with no late correction in colorin.
pub const DISPLAY_REFERRED: &str = "plugins/darkroom/workflow=display-referred (legacy)";

/// darktable's temperature defaults, recovered from a passthrough render with
/// `params`, darktable's rawprepare v2 layout, in the history and temperature left to
/// its defaults: each colour's coefficient is the slope through the origin of
/// darktable's values against rawprepare's, computed here from the raw.
pub fn recover_coefficients(
    raw: &RawImage,
    actual: &Image,
    params: &[u8],
    metadata: &Metadata,
) -> Result<[f64; 3]> {
    let Pixels::U16(values) = &raw.pixels else {
        return Err(skip(SkipKind::Not16Bit, "floating-point values"));
    };
    ensure!(
        params.len() == 32,
        "rawprepare params of {} bytes",
        params.len()
    );
    let word = |at: usize| i32::from_le_bytes(params[at..at + 4].try_into().unwrap()) as u32;
    let level = |at: usize| f64::from(u16::from_le_bytes([params[at], params[at + 1]]));
    let (left, top) = (word(0), word(4));
    let black = [16, 18, 20, 22].map(level);
    let white = level(24);
    let (mut product, mut square) = ([0.0f64; 3], [0.0f64; 3]);
    for y in 0..actual.height as u32 {
        for x in 0..actual.width as u32 {
            let (sx, sy) = (x + left, y + top);
            let b = black[(((sy & 1) << 1) | (sx & 1)) as usize];
            let prepared = (f64::from(values[(sy * raw.width + sx) as usize]) - b) / (white - b);
            let pixel = actual.pixels[y as usize * actual.width + x as usize];
            let developed = f64::from(pixel[0]) + f64::from(pixel[1]) + f64::from(pixel[2]);
            let color = metadata.color_at(sx, sy) as usize;
            product[color] += developed * prepared;
            square[color] += prepared * prepared;
        }
    }
    Ok([0, 1, 2].map(|c| product[c] / square[c]))
}

/// Running sums for a least-squares line.
#[derive(Debug, Clone, Copy, Default)]
struct Sums {
    n: f64,
    x: f64,
    y: f64,
    xx: f64,
    xy: f64,
    yy: f64,
}

impl Sums {
    fn add(&mut self, x: f64, y: f64) {
        self.n += 1.0;
        self.x += x;
        self.y += y;
        self.xx += x * x;
        self.xy += x * y;
        self.yy += y * y;
    }

    /// Slope and intercept.
    fn line(&self) -> (f64, f64) {
        let slope = (self.n * self.xy - self.x * self.y) / (self.n * self.xx - self.x * self.x);
        (slope, (self.y - slope * self.x) / self.n)
    }

    /// The sum of squared distances from the line.
    fn squared_error(&self) -> f64 {
        let (slope, intercept) = self.line();
        let error = self.yy - 2.0 * slope * self.xy - 2.0 * intercept * self.y
            + slope * slope * self.xx
            + 2.0 * slope * intercept * self.x
            + intercept * intercept * self.n;
        error.max(0.0)
    }
}

/// Root-mean-square distance from each position's line, over all four.
fn rms(sums: &[Sums; 4]) -> f64 {
    let n: f64 = sums.iter().map(|s| s.n).sum();
    if sums.iter().any(|s| s.n < 3.0) {
        return f64::INFINITY;
    }
    (sums.iter().map(Sums::squared_error).sum::<f64>() / n).sqrt()
}

/// The part of `image` at (x, y), `width × height`.
pub fn crop(image: &Image, x: usize, y: usize, width: usize, height: usize) -> Result<Image> {
    ensure!(
        x + width <= image.width && y + height <= image.height,
        "a {width}×{height} crop at ({x}, {y}) of a {}×{} image",
        image.width,
        image.height
    );
    let pixels = (y..y + height)
        .flat_map(|row| {
            image.pixels[row * image.width + x..row * image.width + x + width]
                .iter()
                .copied()
        })
        .collect();
    Image::new(width, height, pixels)
}

/// darktable's render of `xmp` on `image` as a linear Rec2020 PFM, cached like the cases'
/// references.
pub fn render(darktable: &DarktableCli, image: &Path, xmp: &str, conf: &[&str]) -> Result<PathBuf> {
    render_as(darktable, image, xmp, conf, "LIN_REC2020", "pfm", None)
}

/// darktable's render of `xmp` on `image` in the output profile `icc_type`, as a file
/// with `extension`, at full resolution or at `size`, cached. A raw darktable-cli's
/// loader rejects is a [`Skip`]; any other failure of darktable-cli is an error.
pub fn render_as(
    darktable: &DarktableCli,
    image: &Path,
    xmp: &str,
    conf: &[&str],
    icc_type: &str,
    extension: &str,
    size: Option<Size>,
) -> Result<PathBuf> {
    let raw = std::fs::read(image).with_context(|| format!("reading {}", image.display()))?;
    let mut hasher = DefaultHasher::new();
    (REFERENCE_VERSION, xmp, conf, &raw).hash(&mut hasher);
    if (icc_type, extension) != ("LIN_REC2020", "pfm") {
        (icc_type, extension).hash(&mut hasher);
    }
    // Only a size, so the full-resolution renders keep their keys.
    if let Some(size) = size {
        size.hash(&mut hasher);
    }
    let name = image
        .file_name()
        .context("image has no file name")?
        .to_string_lossy();
    let dir = cache_root()
        .join("raw-stage")
        .join(name.as_ref())
        .join(format!("{:016x}", hasher.finish()));
    let output = dir.join(format!("output.{extension}"));
    if output.is_file() {
        tracing::trace!(target: "golden", "{name}: cached, {}", dir.display());
        return Ok(output);
    }
    darktable.ensure_reference_version()?;
    let work = dir.join("work");
    if work.exists() {
        std::fs::remove_dir_all(&work)?;
    }
    std::fs::create_dir_all(&work)?;
    let sidecar = work.join("raw-stage.xmp");
    std::fs::write(&sidecar, xmp)?;
    let rendered = work.join(format!("output.{extension}"));
    darktable
        .render(
            image,
            &sidecar,
            &rendered,
            &work.join("config"),
            icc_type,
            conf,
            size,
        )
        .map_err(|error| match error.downcast::<LoaderRejected>() {
            Ok(rejected) => skip(SkipKind::DarktableCannotLoad, rejected.to_string()),
            Err(error) => error,
        })?;
    std::fs::rename(&rendered, &output)?;
    std::fs::remove_dir_all(&work)?;
    Ok(output)
}

/// How darktable's image compares with the expected one, at the offset that fits best.
#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    /// darktable's pixel (x, y) shows the expected pixel (x + dx, y + dy).
    pub offset: (i32, i32),
    pub compared: (usize, usize),
    pub max_abs: f32,
    pub mean_abs: f64,
    /// Compared pixels with a channel more than the threshold away.
    pub over: usize,
}

/// Compares at every offset up to `reach` pixels on a sample of rows, then over the
/// whole overlap at the best one, so a crop that is off by a few pixels shows as an
/// offset rather than as noise. Pixels more than `threshold` away are counted.
pub fn compare(expected: &Image, actual: &Image, reach: i32, threshold: f32) -> Comparison {
    let at = |dx: i32, dy: i32, step: usize| {
        let (mut max_abs, mut sum, mut count, mut over) = (0.0f32, 0.0f64, 0usize, 0usize);
        let y_range = (0..actual.height as i32).filter(|y| {
            let ey = y + dy;
            ey >= 0 && (ey as usize) < expected.height
        });
        for y in y_range.step_by(step) {
            for x in 0..actual.width as i32 {
                let ex = x + dx;
                if ex < 0 || ex as usize >= expected.width {
                    continue;
                }
                let a = actual.pixels[y as usize * actual.width + x as usize];
                let e = expected.pixels[(y + dy) as usize * expected.width + ex as usize];
                let mut worst = 0.0f32;
                for c in 0..3 {
                    let d = (a[c] - e[c]).abs();
                    worst = worst.max(d);
                    sum += f64::from(d);
                }
                max_abs = max_abs.max(worst);
                over += usize::from(worst > threshold);
                count += 1;
            }
        }
        (max_abs, sum / (count.max(1) * 3) as f64, count, over)
    };
    let mut best = (0, 0);
    let mut best_mean = f64::INFINITY;
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            let (_, mean, count, _) = at(dx, dy, 17);
            if count > 0 && mean < best_mean {
                (best, best_mean) = ((dx, dy), mean);
            }
        }
    }
    let (max_abs, mean_abs, _, over) = at(best.0, best.1, 1);
    let overlap = |actual: usize, expected: usize, d: i32| {
        (actual as i32).min(expected as i32 - d) - 0.max(-d)
    };
    Comparison {
        offset: best,
        compared: (
            overlap(actual.width, expected.width, best.0).max(0) as usize,
            overlap(actual.height, expected.height, best.1).max(0) as usize,
        ),
        max_abs,
        mean_abs,
        over,
    }
}

#[cfg(test)]
mod tests {
    use apertrust_raw::{BlackLevel, Camera, Orientation, Rect};

    use crate::xmp::{is_enabled, module_params};

    use super::*;

    /// A sensor with these photosites and nothing else of note.
    fn test_raw(width: u32, height: u32, values: Vec<u16>) -> RawImage {
        RawImage {
            camera: Camera {
                make: "Test".into(),
                model: "Sensor".into(),
                alias: "Sensor".into(),
            },
            width,
            height,
            components: 1,
            pixels: Pixels::U16(values),
            active_area: Rect {
                x: 0,
                y: 0,
                width,
                height,
            },
            default_crop: None,
            masked_areas: Vec::new(),
            cfa: None,
            black_level: BlackLevel {
                width: 1,
                height: 1,
                values: vec![0.0],
            },
            white_levels: vec![u32::from(u16::MAX)],
            white_balance: None,
            color_matrices: Vec::new(),
            dng_color: None,
            linear_response_limit: None,
            orientation: Orientation::Normal,
            exposure_bias: None,
            iso: None,
            lens_correction: None,
        }
    }

    fn image(width: usize, height: usize, f: impl Fn(usize, usize) -> f32) -> Image {
        let pixels = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| [f(x, y), 0.0, 2.0 * f(x, y)])
            .collect();
        Image::new(width, height, pixels).unwrap()
    }

    #[test]
    fn compare_finds_a_shifted_crop() {
        // No period: every offset gives different values.
        let pattern = |x: usize, y: usize| ((x * x * 31 + y * y * 17 + x * y * 7) % 97) as f32;
        let expected = image(40, 30, pattern);
        let shifted = image(36, 26, |x, y| pattern(x + 3, y + 2));
        let found = compare(&expected, &shifted, 4, 0.0);
        assert_eq!(found.offset, (3, 2));
        assert_eq!(found.max_abs, 0.0);
        assert_eq!(found.compared, (36, 26));
        assert_eq!(found.over, 0);
    }

    #[test]
    fn recovers_the_crop_and_levels_of_a_passthrough() {
        // A 12×10 sensor with noise-like values; darktable's image is 8×6 from (3, 1).
        let (width, height) = (12u32, 10u32);
        let values: Vec<u16> = (0..width * height)
            .map(|i| (600 + (i * 7919 + i * i * 31) % 3000) as u16)
            .collect();
        let black = [512.0, 513.0, 509.0, 511.0];
        let white = 4000.0;
        let developed = |x: u32, y: u32| {
            let (sx, sy) = (x + 3, y + 1);
            let b = black[(((sy & 1) << 1) | (sx & 1)) as usize];
            let v = (f32::from(values[(sy * width + sx) as usize]) - b) / (white - b);
            [v, 0.0, 0.0]
        };
        let actual = Image::new(
            8,
            6,
            (0..6)
                .flat_map(|y| (0..8).map(move |x| developed(x, y)))
                .collect(),
        )
        .unwrap();
        let sensor = test_raw(width, height, values);
        let found = recover_defaults(&sensor, &actual).unwrap();
        assert_eq!(found.crop, [3, 1, 8, 6]);
        for (got, want) in found.black.iter().zip(black) {
            assert!((got - f64::from(want)).abs() < 1e-2, "{:?}", found.black);
        }
        assert!(
            found.white.iter().all(|w| (w - 4000.0).abs() < 1e-1),
            "{:?}",
            found.white
        );
    }

    #[test]
    fn the_base_history_has_the_entries_the_check_rewrites() {
        // Each entry the checks patch in place, at the version its layout assumes.
        let patched = [
            ("rawprepare", 2),
            ("temperature", 4),
            ("highlights", 4),
            ("demosaic", 5),
            ("colorin", 7),
            ("channelmixerrgb", 3),
        ];
        for (operation, version) in patched {
            let entries = all_params(BASE_XMP, operation).unwrap();
            assert_eq!(entries.len(), 1, "{operation}");
            assert_eq!(entries[0].version, version, "{operation}");
        }
        // And the ones the whole pipeline keeps, replaces whole or switches off.
        for operation in ["exposure", "agx", "colorout", "gamma", "flip"] {
            assert!(operations(BASE_XMP).contains(&operation), "{operation}");
        }
    }

    /// A raw stage with every field set away from the passthrough's.
    fn stage() -> RawStage {
        RawStage {
            demosaic: Some(2),
            temperature_preset: 3,
            colorin: Some(STANDARD_MATRIX),
            highlights: Some((4, 0.987)),
            ..RawStage::passthrough(
                Some(rawprepare_params([8, 4, 2, 6], [510, 511, 512, 513], 16000)),
                Some([2.0, 1.0, 1.5]),
                false,
            )
        }
    }

    fn word(bytes: &[u8], at: usize) -> [u8; 4] {
        bytes[at..at + 4].try_into().unwrap()
    }

    #[test]
    fn history_keeps_what_also_names_on() {
        let without = history(&stage(), &[]).unwrap();
        let with = history(&stage(), &["flip"]).unwrap();
        assert!(!is_enabled(&without, "flip").unwrap());
        assert!(is_enabled(&with, "flip").unwrap());
        for xmp in [&without, &with] {
            for operation in KEEP.iter().chain(&["highlights"]) {
                assert!(is_enabled(xmp, operation).unwrap(), "{operation}");
            }
            for operation in ["exposure", "agx", "channelmixerrgb"] {
                assert!(!is_enabled(xmp, operation).unwrap(), "{operation}");
            }
        }
    }

    /// Color calibration's version 3 params with a custom illuminant under CAT16, as
    /// Apertrust's defaults give a colour raw.
    fn channel_mixer() -> Vec<u8> {
        use apertrust_operations::channelmixerrgb::Settings;
        crate::ports::channel_mixer_params_v3(&Settings {
            illuminant: apertrust_color::Illuminant::Custom,
            x: 0.3892037,
            y: 0.41662326,
            temperature: 3927.1,
            ..Settings::DEFAULT
        })
    }

    #[test]
    fn finalscale_moves_before_colorout_and_nothing_else_moves() {
        let order = |xmp: &str| -> Vec<String> {
            let list = xmp
                .split("darktable:iop_order_list=\"")
                .nth(1)
                .and_then(|rest| rest.split('"').next())
                .unwrap();
            list.split(',').step_by(2).map(str::to_owned).collect()
        };
        let before = order(BASE_XMP);
        let after = order(&finalscale_before_colorout(BASE_XMP).unwrap());
        let at = |order: &[String], op: &str| order.iter().position(|o| o == op).unwrap();
        assert!(at(&before, "finalscale") > at(&before, "colorout"));
        assert_eq!(at(&after, "finalscale") + 1, at(&after, "colorout"));
        let without = |order: &[String]| -> Vec<String> {
            order
                .iter()
                .filter(|o| *o != "finalscale")
                .cloned()
                .collect()
        };
        assert_eq!(without(&before), without(&after));
    }

    #[test]
    fn pipeline_xmp_writes_the_stage_and_every_operation_after_it() {
        let agx = crate::ports::agx_params_v7(
            &apertrust_operations::agx::Settings::scene_referred_default(),
        );
        let stage = stage();
        let xmp = pipeline_xmp(&stage, -1.0 / 4096.0, 0.25, &channel_mixer(), &agx).unwrap();

        // Exposure as darktable's version 7: manual, black, exposure, deflicker
        // defaults, and neither compensation.
        let exposure = module_params(&xmp, "exposure").unwrap();
        assert_eq!(exposure.version, 7);
        assert_eq!(exposure.bytes.len(), 28);
        assert_eq!(word(&exposure.bytes, 0), 0i32.to_le_bytes());
        assert_eq!(word(&exposure.bytes, 4), (-1.0f32 / 4096.0).to_le_bytes());
        assert_eq!(word(&exposure.bytes, 8), 0.25f32.to_le_bytes());
        assert_eq!(word(&exposure.bytes, 12), 50.0f32.to_le_bytes());
        assert_eq!(word(&exposure.bytes, 16), (-4.0f32).to_le_bytes());
        assert_eq!(word(&exposure.bytes, 20), 0i32.to_le_bytes());
        assert_eq!(word(&exposure.bytes, 24), 0i32.to_le_bytes());
        // Color calibration as given, version 3, in the base history's entry.
        let written = module_params(&xmp, "channelmixerrgb").unwrap();
        assert_eq!((written.version, &written.bytes), (3, &channel_mixer()));
        // AgX as given, version 7. `module_params` also requires each to be on.
        let written = module_params(&xmp, "agx").unwrap();
        assert_eq!((written.version, written.bytes), (7, agx));

        // Orientation at darktable's auto, as Apertrust's default.
        let flip = module_params(&xmp, "flip").unwrap();
        assert_eq!(
            (flip.version, flip.bytes),
            (2, (-1i32).to_le_bytes().to_vec())
        );
        let on = ["highlights", "exposure", "channelmixerrgb", "agx", "flip"];
        for operation in KEEP.iter().chain(&on) {
            assert!(is_enabled(&xmp, operation).unwrap(), "{operation}");
        }

        // The raw stage as `stage` sets it.
        let rawprepare = module_params(&xmp, "rawprepare").unwrap();
        assert_eq!(&rawprepare.bytes, stage.rawprepare.as_ref().unwrap());
        let temperature = module_params(&xmp, "temperature").unwrap();
        assert_eq!(temperature.bytes.len(), 20);
        assert_eq!(word(&temperature.bytes, 0), 2.0f32.to_le_bytes());
        assert_eq!(word(&temperature.bytes, 8), 1.5f32.to_le_bytes());
        assert_eq!(word(&temperature.bytes, 16), 3i32.to_le_bytes());
        let highlights = module_params(&xmp, "highlights").unwrap();
        assert_eq!(word(&highlights.bytes, 0), 4i32.to_le_bytes());
        assert_eq!(word(&highlights.bytes, 16), 0.987f32.to_le_bytes());
        let demosaic = module_params(&xmp, "demosaic").unwrap();
        assert_eq!(word(&demosaic.bytes, 12), 2u32.to_le_bytes());
        let colorin = module_params(&xmp, "colorin").unwrap();
        assert_eq!(word(&colorin.bytes, 0), STANDARD_MATRIX.to_le_bytes());
    }

    #[test]
    fn pipeline_xmp_refuses_params_of_another_size() {
        let agx = crate::ports::agx_params_v7(
            &apertrust_operations::agx::Settings::scene_referred_default(),
        );
        assert!(pipeline_xmp(&stage(), 0.0, 0.0, &channel_mixer(), &[0; 140]).is_err());
        assert!(pipeline_xmp(&stage(), 0.0, 0.0, &[0; 156], &agx).is_err());
    }

    #[test]
    fn what_cannot_be_checked_is_a_skip() {
        let kind = |error: &anyhow::Error| error.downcast_ref::<Skip>().map(|skip| skip.kind);
        let mut raw = test_raw(4, 4, vec![0; 16]);
        let error = Metadata::of(&raw).unwrap_err();
        assert_eq!(kind(&error), Some(SkipKind::NotColourMosaic), "{error:#}");
        raw.cfa = Some(apertrust_raw::Cfa {
            width: 2,
            height: 2,
            colors: vec![
                CfaColor::Cyan,
                CfaColor::Yellow,
                CfaColor::Green,
                CfaColor::Magenta,
            ],
        });
        let error = Metadata::of(&raw).unwrap_err();
        assert_eq!(kind(&error), Some(SkipKind::NotColourMosaic), "{error:#}");
        assert_eq!(format!("{error:#}"), "not a colour mosaic: CFA colour Cyan");
        // A file that is not there is an error, not a skip, named as given.
        let error = decode(Path::new("missing/raw.cr2")).unwrap_err();
        assert_eq!(kind(&error), None);
        assert_eq!(format!("{error:#}"), "no file missing/raw.cr2");
    }
}
