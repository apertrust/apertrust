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

//! What an operation knows about the image: its data format at each stage, and the
//! camera's metadata.

use std::fmt;

/// How a buffer holds the image (`docs/engine.md`, Data).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// The mosaic as decoded: one 16-bit value per photosite, two to a 32-bit word.
    Mosaic16,
    /// The mosaic as one `f32` per photosite.
    Mosaic,
    /// Four `f32` per pixel, alpha passing through.
    Rgba,
}

impl Format {
    /// Bytes a buffer of `pixels` needs, rounded up to whole 32-bit words.
    pub fn buffer_size(self, pixels: u64) -> u64 {
        match self {
            Format::Mosaic16 => (pixels * 2).next_multiple_of(4),
            Format::Mosaic => pixels * 4,
            Format::Rgba => pixels * 16,
        }
    }

    pub fn is_mosaic(self) -> bool {
        matches!(self, Format::Mosaic16 | Format::Mosaic)
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Format::Mosaic16 => "16-bit mosaic",
            Format::Mosaic => "mosaic",
            Format::Rgba => "RGBA",
        })
    }
}

/// The image a pass reads or writes: its size and format, the part of its stage it is,
/// and what earlier operations did to it that later ones need, as darktable's buffer
/// descriptor carries it (`dt_iop_buffer_dsc_t`, decision 21).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageDesc {
    pub width: u32,
    pub height: u32,
    pub format: Format,
    /// Where the image's top-left pixel sits on the sensor, in photosites at full
    /// resolution: (0, 0) for the readout, rawprepare's crop corner after it. For a
    /// mosaic it fixes each photosite's filter colour and row and column parity, as
    /// darktable's shifted filter pattern does (decision 21).
    pub sensor_origin: [u32; 2],
    /// The value each of red, green and blue reaches where the sensor saturates:
    /// darktable's `processed_maximum`, 1 after rawprepare, times the white balance
    /// coefficients after temperature.
    pub maximum: [f32; 3],
    /// The white balance coefficients applied so far, 1 before temperature: darktable's
    /// `dsc.temperature.coeffs`.
    pub white_balance: [f32; 3],
    /// Whether input colour must still correct the white balance towards the camera's
    /// D65 reference, as temperature's preset "as shot to reference" asks: darktable's
    /// `dev->chroma.late_correction`.
    pub late_correction: bool,
    /// Its stage's image at full resolution (decision 16): the developed image, or after
    /// an operation that moves pixels, that operation's output. The planner sets it for
    /// every RGBA image; a mosaic's is (0, 0), since it lies on the sensor, in no stage.
    pub extent: [u32; 2],
    /// Its stage's image at the pipe's scale (decision 15), which whole-image passes
    /// cover: `extent` up to the viewport's downscale or an export's resample, the fitted
    /// size after the downscale, with geometry each stage's [`Extent::scaled`], and after
    /// the resample its output, darktable's `floor(scale × W)` by `floor(scale × H)`. The
    /// planner sets it for every RGBA image; a mosaic's is (0, 0), as its `extent` is.
    pub scaled_extent: [u32; 2],
    /// Where its top-left pixel lies in its stage's image at the pipe's scale (decision
    /// 16): (0, 0) for the whole image, a region's corner otherwise, so the image is the
    /// region `origin`, `width` and `height`. The planner sets it for every RGBA image; a
    /// mosaic's is (0, 0), since `sensor_origin` places it.
    pub origin: [u32; 2],
}

impl fmt::Display for ImageDesc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}×{} {}", self.width, self.height, self.format)
    }
}

impl ImageDesc {
    /// An image whose top-left pixel is the sensor's, untouched by any operation, and
    /// which, in RGBA, is its stage's whole image, at full resolution.
    pub const fn new(width: u32, height: u32, format: Format) -> Self {
        let extent = match format {
            Format::Rgba => [width, height],
            Format::Mosaic16 | Format::Mosaic => [0, 0],
        };
        Self {
            width,
            height,
            format,
            sensor_origin: [0, 0],
            maximum: [1.0; 3],
            white_balance: [1.0; 3],
            late_correction: false,
            extent,
            scaled_extent: extent,
            origin: [0, 0],
        }
    }

    /// The part of its stage's image at the pipe's scale it holds: x, y, width, height.
    pub fn region(&self) -> [u32; 4] {
        [self.origin[0], self.origin[1], self.width, self.height]
    }

    /// darktable `dt_iop_get_processed_maximum`: the largest channel's maximum, at
    /// least 1.
    pub fn processed_maximum(&self) -> f32 {
        self.maximum.into_iter().fold(1.0, f32::max)
    }

    pub const fn rgba(width: u32, height: u32) -> Self {
        Self::new(width, height, Format::Rgba)
    }

    /// Whether a buffer for `other` can hold this image: the same size and format,
    /// wherever on the sensor it sits.
    pub fn fits(&self, other: &ImageDesc) -> bool {
        (self.width, self.height, self.format) == (other.width, other.height, other.format)
    }

    pub fn buffer_size(&self) -> u64 {
        self.format
            .buffer_size(u64::from(self.width) * u64::from(self.height))
    }
}

/// A stage's image, as a [`Geometry`](crate::Geometry) maps regions of it (decision 16).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extent {
    /// The image at full resolution.
    pub full: [u32; 2],
    /// The image at the pipe's scale: `full` times the scale, rounded once by the planner
    /// for every stage, so that the sides a quarter turn swaps stay equal.
    pub scaled: [u32; 2],
    /// Pixels at the pipe's scale per pixel at full resolution, as the pass header has it.
    pub scale: f32,
}

/// Which render a pipe makes, as its operations see it (decision 16): darktable's pipe
/// type, for the pipes Apertrust has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeKind {
    /// An export at full resolution, whole or tile by tile.
    Export,
    /// The viewport, fitted to its panel, and the stills, which are its plans: its
    /// operations after its downscale are never tiled.
    Viewport,
}

/// What an operation knows about the image it runs on, besides its parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageContext {
    /// The developed image at full resolution: after the raw's crop.
    pub width: u32,
    pub height: u32,
    /// Pixels at the pipe's scale per pixel at full resolution, at most 1.
    pub scale: f32,
    /// The part of the developed image the pipe produces, at full resolution: x, y,
    /// width, height. The whole image, but for a tile of a tiled export (decision 13).
    pub region: [u32; 4],
    /// Which render the pipe makes, which the planner sets (decision 16).
    pub pipe: PipeKind,
    /// The camera's metadata, for a raw; `None` for an image that is not one.
    pub camera: Option<Camera>,
    /// The pipe's output profile, for `prepare` alone: what colorout reports through
    /// [`Operation::output_profile`](crate::Operation::output_profile), which the planner
    /// asks every instance before any `prepare`; `None`, the working profile, when no
    /// instance of the pipe reports one (decision 26). `defaults` never sees it.
    pub output_profile: Option<OutputProfile>,
    /// In the viewport, for `prepare` alone and only for an operation that opts in
    /// through [`Operation::whole_image_at_full_resolution`](crate::Operation), the
    /// full-resolution image its downscale read, which its whole-image passes may bind as
    /// [`Resource::FullResolution`](crate::Resource) to compute their results at full
    /// resolution (decision 15). `None` everywhere else, and where the viewport cannot
    /// give it: its full resolution in tiles, or an operation between it and the
    /// downscale.
    pub full_resolution: Option<ImageDesc>,
}

impl ImageContext {
    /// An image that is not a raw, exported at full resolution.
    pub fn full(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            scale: 1.0,
            region: [0, 0, width, height],
            pipe: PipeKind::Export,
            camera: None,
            output_profile: None,
            full_resolution: None,
        }
    }

    /// The whole image at the pipe's scale that a whole-image pass covers and lays its
    /// scratch out as: its input's stage, after a geometry that geometry's output
    /// (decision 16), as the planner gives it in every pipe, `scaled_extent` (decision
    /// 15). A tile of an export holds the part of it at its `origin`, which the pass
    /// header's origin carries to the kernel.
    pub fn whole_image(&self, input: &ImageDesc) -> [u32; 2] {
        input.scaled_extent
    }
}

/// An output profile as the matrices between its linear RGB and XYZ D50, f32 by rows:
/// what darktable's filmic rgb reads of the pipe's output profile
/// (`dt_ioppr_get_pipe_output_profile_info`, decision 26).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutputProfile {
    /// Linear RGB to XYZ D50: darktable's `matrix_in`.
    pub rgb_to_xyz_d50: [[f32; 3]; 3],
    /// XYZ D50 to linear RGB: darktable's `matrix_out`.
    pub xyz_d50_to_rgb: [[f32; 3]; 3],
}

/// A raw's metadata, as the decoder reports it.
#[derive(Debug, Clone, PartialEq)]
pub struct Camera {
    /// The camera's maker, as darktable's `camera_maker` (decision 24).
    pub make: String,
    /// The camera's model, as darktable's `camera_model`: an alias resolved to the
    /// camera it is another name for.
    pub model: String,
    /// The name the file's camera has, as darktable's `camera_alias`: `model`, unless
    /// the camera is an alias of another.
    pub alias: String,
    /// The whole sensor readout, the mosaic as uploaded.
    pub width: u32,
    pub height: u32,
    /// The colour filter array; `None` for a monochrome sensor.
    pub cfa: Option<Pattern<u32>>,
    /// Black level per position of a repeating pattern.
    pub black: Pattern<f32>,
    pub white: f32,
    /// As-shot multipliers per CFA colour, green 1; `None` if the file records none.
    pub white_balance: Option<Vec<f32>>,
    /// XYZ to camera RGB under D65, by rows; `None` if the decoder has none.
    pub xyz_to_camera: Option<[[f32; 3]; 3]>,
    /// The active area, every valid photosite, which rawprepare keeps: x, y, width,
    /// height on the readout.
    pub crop: [u32; 4],
    /// The manufacturer's declared image, inside the active area, which demosaic
    /// writes: x, y, width, height on the readout (decision 23).
    pub declared: [u32; 4],
    /// The exposure compensation set on the camera, in EV; 0 if unrecorded.
    pub exposure_bias: f32,
    /// The ISO the photo was taken at, as darktable's `exif_iso`; 0 if unrecorded.
    pub iso: f32,
    /// A DNG's LinearResponseLimit: the fraction of the white point up to which the
    /// sensor responds linearly; 1 if unrecorded, as darktable takes it.
    pub linear_response_limit: f32,
    /// The corrections the raw records for its lens, as darktable reads them, its
    /// `exif_correction_type` and `exif_correction_data` (decision 25); `None` where
    /// darktable finds none. Only Fujifilm's are read so far.
    pub lens_correction: Option<LensCorrection>,
    /// How the photo is to be turned upright, as darktable's `dt_image_t::orientation`
    /// holds it, from the EXIF Orientation tag (`dt_image_orientation_to_flip_bits`): its
    /// flip bits, 1 flipping the rows (`ORIENTATION_FLIP_Y`), 2 the columns
    /// (`ORIENTATION_FLIP_X`) and 4 swapping the axes after them
    /// (`ORIENTATION_SWAP_XY`); 0 where the raw records none.
    pub orientation: u8,
}

/// A lens's corrections as the raw records them: darktable's `dt_image_correction_data_t`
/// by its type, each value as darktable stores it after its checks (`src/common/exif.cc`).
/// Lens correction computes every table from them (decision 25).
#[derive(Debug, Clone, PartialEq)]
pub enum LensCorrection {
    Sony(SonyCorrection),
    Fujifilm(FujifilmCorrection),
    Dng(DngCorrection),
    Olympus(OlympusCorrection),
}

/// A Sony raw's DistortionCorrParams, ChromaticAberrationCorrParams and
/// VignettingCorrParams, darktable's `sony`: knot i lies at radius (i + 0.5) / (nc − 1)
/// of the half-diagonal.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SonyCorrection {
    /// The knots, at most 16; every array is 0 past them.
    pub nc: u32,
    /// The input radius over the output radius is 1 + distortion · 2⁻¹⁴.
    pub distortion: [i16; 16],
    /// Red's radius factor, 1 + ca_r · 2⁻²¹.
    pub ca_r: [i16; 16],
    /// Blue's radius factor, 1 + ca_b · 2⁻²¹.
    pub ca_b: [i16; 16],
    /// The brightness, 2^(0.5 − 2^(vignetting · 2⁻¹³ − 1)).
    pub vignetting: [i16; 16],
}

/// A RAF's GeometricDistortionParams, ChromaticAberrationParams and VignettingParams,
/// from its FujiIFD, darktable's `fuji`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FujifilmCorrection {
    /// The knots: 9 in the X-Trans IV and V layout, 11 in the X-Trans I to III one; every
    /// array is 0 past them.
    pub nc: u32,
    /// The knots' scale: 1.25 in the camera's 1.25× crop modes, else 1.
    pub cropf: f32,
    /// The radii, normalised to the half-diagonal before `cropf`.
    pub knots: [f32; 11],
    /// The input radius over the output radius is 1 + distortion / 100.
    pub distortion: [f32; 11],
    /// Red's radius factor, 1 + ca_r.
    pub ca_r: [f32; 11],
    /// Blue's radius factor, 1 + ca_b.
    pub ca_b: [f32; 11],
    /// The brightness, in percent.
    pub vignetting: [f32; 11],
}

/// A DNG's OpcodeList3, its last WarpRectilinear and FixVignetteRadial, darktable's `dng`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DngCorrection {
    /// WarpRectilinear's planes, 1 or 3.
    pub planes: u32,
    /// Per plane, kr0 to kr3, kt0 and kt1: the input radius over the output radius is
    /// kr0 + kr1 r² + kr2 r⁴ + kr3 r⁶, r normalised to the half-diagonal.
    pub cwarp: [[f32; 6]; 3],
    /// WarpRectilinear's centre, which darktable does not read.
    pub centre_warp: [f32; 2],
    /// FixVignetteRadial's k0 to k4: the gain is 1 + k0 r² + k1 r⁴ + k2 r⁶ + k3 r⁸ + k4 r¹⁰.
    pub cvig: [f32; 5],
    /// FixVignetteRadial's centre, which darktable does not read.
    pub centre_vig: [f32; 2],
    pub has_warp: bool,
    pub has_vignette: bool,
}

/// An Olympus or OM System raw's maker note tags 0x150a and 0x150c, of its ImageProcessing
/// directory, darktable's `olympus`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OlympusCorrection {
    /// Whether `dist` corrects anything: one of its first three values is not 0.
    pub has_dist: bool,
    /// dk2, dk4, dk6 and drs, the corner's radius.
    pub dist: [f32; 4],
    /// Whether `ca` corrects anything: one of its values is not 0.
    pub has_ca: bool,
    /// car0, car2, car4, cab0, cab2 and cab4.
    pub ca: [f32; 6],
}

/// Values repeating from the top-left of the mosaic, row by row: a CFA's colours (0
/// red, 1 green, 2 blue) or black levels.
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern<T> {
    pub width: u32,
    pub height: u32,
    pub values: Vec<T>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_whole_image_pass_covers_its_inputs_stage_at_the_pipes_scale() {
        // An image made outside a plan is its stage's whole image at full resolution.
        assert_eq!(ImageDesc::rgba(3908, 2602).scaled_extent, [3908, 2602]);
        assert_eq!(ImageDesc::new(8, 6, Format::Mosaic).scaled_extent, [0, 0]);
        let export = ImageContext::full(3908, 2602);
        assert_eq!(
            export.whole_image(&ImageDesc::rgba(3908, 2602)),
            [3908, 2602]
        );
        // A tile at (200, 160) of its stage, and one of a quarter turn's stage.
        let tile = ImageDesc {
            origin: [200, 160],
            extent: [3908, 2602],
            scaled_extent: [3908, 2602],
            ..ImageDesc::rgba(100, 80)
        };
        assert_eq!(export.whole_image(&tile), [3908, 2602]);
        let turned = ImageDesc {
            extent: [2602, 3908],
            scaled_extent: [2602, 3908],
            ..tile
        };
        assert_eq!(export.whole_image(&turned), [2602, 3908]);
        // After an export's resample, the output: the D5100 at 640, 639 × 423 by
        // darktable's double floor, of which a tile holds a part (decision 15).
        let at_size = ImageContext {
            scale: (640.0f64 / 4928.0) as f32,
            ..ImageContext::full(4928, 3264)
        };
        let resampled = ImageDesc {
            origin: [320, 200],
            extent: [4928, 3264],
            scaled_extent: [639, 423],
            ..ImageDesc::rgba(64, 48)
        };
        assert_eq!(at_size.whole_image(&resampled), [639, 423]);
        // In the viewport the input is all of its stage at the pipe's scale, even where
        // the scale, taken from the width, rounds back to 1 while the height is smaller.
        let viewport = ImageContext {
            pipe: PipeKind::Viewport,
            ..ImageContext::full(100, 200)
        };
        let input = ImageDesc {
            extent: [100, 200],
            scaled_extent: [100, 199],
            ..ImageDesc::rgba(100, 199)
        };
        assert_eq!(viewport.whole_image(&input), [100, 199]);
    }
}
