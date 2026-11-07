// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2018-2026  darktable developers (ported from src/iop/toneequal.c)
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

//! Tone equalizer: setup for the `toneequal` kernel, ported from darktable's
//! `commit_params()`, `modify_roi_in()` and `get_channels_factors()`, with the band fit,
//! darktable's `pseudo_solve()` in f32, solved in f64 instead, and the mask's passes. The
//! curve and its fit are `apertrust-color`'s (`tone_equalizer`), which the module's band
//! graph draws too (decision 65). See `docs/ports/toneequal.md`.

use apertrust_color::tone_equalizer::{self, Curve};
pub use apertrust_color::tone_equalizer::{BAND_CENTRES, CURVE_CENTRES, fit};
use apertrust_plugin::{Binding, NotPorted, Pass, PassKind, Resource, Scratch};

/// darktable's `smoothing` slider, σ of the curve's Gaussians in EV: `√2^(1 + s)` for
/// `s` in -2.33 to 1.67, soft from -1 to 1, so 1 to 2. darktable's introspection gives
/// the field no range; the schema takes the slider's, its ends exactly as darktable's
/// `smoothing_callback` computes them in f32, `powf(M_SQRT2_F, 1.0f + s)`, correctly
/// rounded.
pub const SMOOTHING_MIN: f32 = 0.6306887;
pub const SMOOTHING_MAX: f32 = 2.5227547;

/// darktable `dt_iop_toneequalizer_filter_t`: how the mask is smoothed before the curve
/// reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    /// `DT_TONEEQ_NONE`, "no": the estimator per pixel, a tone curve by brightness.
    None,
    /// `DT_TONEEQ_AVG_GUIDED`, "averaged guided filter".
    AveragedGuided,
    /// `DT_TONEEQ_GUIDED`, "guided filter".
    Guided,
    /// `DT_TONEEQ_AVG_EIGF`, "averaged EIGF".
    AveragedEigf,
    /// `DT_TONEEQ_EIGF`, "EIGF", the exposure-independent guided filter, darktable's
    /// default.
    Eigf,
}

impl Filter {
    pub fn from_darktable(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::AveragedGuided),
            2 => Some(Self::Guided),
            3 => Some(Self::AveragedEigf),
            4 => Some(Self::Eigf),
            _ => None,
        }
    }

    pub fn to_darktable(self) -> i32 {
        match self {
            Self::None => 0,
            Self::AveragedGuided => 1,
            Self::Guided => 2,
            Self::AveragedEigf => 3,
            Self::Eigf => 4,
        }
    }
}

/// darktable `dt_iop_luminance_mask_method_t` (`src/common/luminance_mask.h`): how a
/// pixel's RGB becomes its mask value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Estimator {
    /// `DT_TONEEQ_MEAN`, "RGB average": `(R + G + B) / 3`.
    Average,
    /// `DT_TONEEQ_LIGHTNESS`, "HSL lightness": `(max + min) / 2`.
    Lightness,
    /// `DT_TONEEQ_VALUE`, "HSV value / RGB max": `max(R, G, B)`.
    Value,
    /// `DT_TONEEQ_NORM_1`, "RGB sum": `|R| + |G| + |B|`.
    Sum,
    /// `DT_TONEEQ_NORM_2`, "RGB euclidean norm", darktable's default.
    EuclideanNorm,
    /// `DT_TONEEQ_NORM_POWER`, "RGB power norm": `Σ|c|³ / Σc²`.
    PowerNorm,
    /// `DT_TONEEQ_GEOMEAN`, "RGB geometric mean": `|R·G·B|^(1/3)`.
    GeometricMean,
}

impl Estimator {
    pub const ALL: [Self; 7] = [
        Self::Average,
        Self::Lightness,
        Self::Value,
        Self::Sum,
        Self::EuclideanNorm,
        Self::PowerNorm,
        Self::GeometricMean,
    ];

    pub fn from_darktable(value: i32) -> Option<Self> {
        Self::ALL.get(usize::try_from(value).ok()?).copied()
    }

    pub fn to_darktable(self) -> i32 {
        Self::ALL.iter().position(|&e| e == self).expect("in ALL") as i32
    }
}

/// The module's settings: darktable params version 2, field for field.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// The bands' gains in EV, at mask -8 EV to 0 EV ([`BAND_CENTRES`]): darktable's
    /// `noise`, `ultra_deep_blacks`, `deep_blacks`, `blacks`, `shadows`, `midtones`,
    /// `highlights`, `whites` and `speculars`, whose names are not the bands' labels
    /// below -2 EV (`docs/ports/toneequal.md`, Parameters).
    pub bands: [f32; 9],
    /// The mask filter's diameter, percent of the image's longer side.
    pub blending: f32,
    /// σ of the curve's Gaussians, EV.
    pub smoothing: f32,
    /// The filter's edge refinement; its regularisation is the inverse.
    pub feathering: f32,
    /// The filter guide's posterisation step, EV; 0 off.
    pub quantization: f32,
    /// EV; the guided filter and EIGF only.
    pub contrast_boost: f32,
    /// EV the mask is moved by before the curve reads it.
    pub exposure_boost: f32,
    pub details: Filter,
    pub method: Estimator,
    pub iterations: i32,
}

impl Settings {
    /// darktable's defaults (its introspection's; it has no `reload_defaults`), its
    /// filter EIGF.
    pub fn defaults() -> Self {
        Self {
            bands: [0.0; 9],
            blending: 5.0,
            smoothing: std::f32::consts::SQRT_2,
            feathering: 1.0,
            quantization: 0.0,
            contrast_boost: 0.0,
            exposure_boost: 0.0,
            details: Filter::Eigf,
            method: Estimator::EuclideanNorm,
            iterations: 1,
        }
    }
}

/// Directions in band space, EV per unit, one per derivative output: the band gains
/// move by `t_k · directions[k]`, and the derivative entry points differentiate
/// against `t` at 0.
pub type Directions = [[f32; 9]; 4];

/// The four directions `prepare` chooses until a binding gives its own: the -6, -4,
/// -2 and 0 EV bands alone (`deep_blacks`, `shadows`, `highlights`, `speculars`).
pub const DEFAULT_DIRECTIONS: Directions = {
    let mut directions = [[0.0; 9]; 4];
    directions[0][2] = 1.0;
    directions[1][4] = 1.0;
    directions[2][6] = 1.0;
    directions[3][8] = 1.0;
    directions
};

/// Where the operation runs, for the mask's filter: darktable's radius comes from the
/// image at full resolution, the grid from the whole image at the pipe's scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    /// darktable's `piece->iwidth` and `iheight`, which its radius is measured on: the
    /// pipe's input at full resolution, for a raw its readout before rawprepare's crop.
    pub full: [u32; 2],
    /// Pixels at the pipe's scale per pixel at full resolution.
    pub scale: f32,
    /// The whole image at the pipe's scale, which the mask covers in every pipe
    /// (decision 15).
    pub whole: [u32; 2],
    /// The part of it the operation's input holds: all of it, or an export's tile.
    pub input: [u32; 2],
}

impl Geometry {
    /// The whole image at full resolution.
    pub fn full(width: u32, height: u32) -> Self {
        Self {
            full: [width, height],
            scale: 1.0,
            whole: [width, height],
            input: [width, height],
        }
    }
}

/// What the kernel needs besides the image: the curve, and the mask. The correction for
/// a mask at `E` EV is darktable's curve, `clamp(Σ weights_j G_j, 0.25, 4)` with
/// `G_j = exp(-(E - ν_j)² · inverse_denominator)` for the [`CURVE_CENTRES`] ν_j.
#[derive(Debug, Clone, PartialEq)]
pub struct Prepared {
    pub weights: [f32; 8],
    /// `1 / (2σ²)`, σ the smoothing.
    pub inverse_denominator: f32,
    /// The derivative of `weights` along each of the directions.
    pub directions: [[f32; 8]; 4],
    pub mask: Mask,
}

/// How the mask is built: darktable's `compute_luminance_mask()` settings, from
/// `commit_params()` and `modify_roi_in()`.
#[derive(Debug, Clone, PartialEq)]
pub struct Mask {
    pub method: Estimator,
    /// [`Filter::None`], [`Filter::AveragedEigf`] or [`Filter::Eigf`].
    pub filter: Filter,
    pub iterations: u32,
    /// darktable's `exposure_boost`, linear: `2^exposure_boost`.
    pub exposure_boost: f32,
    /// `luminance_mask()`'s fulcrum and contrast: 2^-4 and `2^contrast_boost` for
    /// EIGF, 0 and 1 for the others, which compensate no contrast.
    pub fulcrum: f32,
    pub contrast_boost: f32,
    /// The filter's regularisation, `1 / feathering`: darktable's `d->feathering`.
    pub feathering: f32,
    /// The guide's quantisation step, EV; 0 for none.
    pub quantization: f32,
    /// The filter's grid; `None` without a filter.
    pub grid: Option<Grid>,
}

/// EIGF's grid (`fast_eigf_surface_blur()`) for the whole image at the pipe's scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    /// The whole image at the pipe's scale, W × H, and the part the input holds.
    pub whole: [u32; 2],
    pub input: [u32; 2],
    /// darktable's radius, truncated at full resolution, times the pipe's scale: EIGF's
    /// `sigma` (decision 15).
    pub sigma: f32,
    /// The grid, w × h: `(size_t)(W / scaling)` with `scaling = clamp(sigma, 1, 4)`, at
    /// least 1 on each side.
    pub size: [u32; 2],
    /// The Gaussian's σ on the grid, `max(sigma / scaling, 1)`, and its coefficients.
    pub grid_sigma: f32,
    pub deriche: Deriche,
}

impl Grid {
    /// The range pass's workgroups across, and in all.
    fn partials(&self) -> (u32, u32) {
        let [across, down] = self.size.map(|side| side.div_ceil(WORKGROUP));
        (across, across * down)
    }
}

/// `dt_gaussian_blur()`'s coefficients for order 0: darktable `_compute_gauss_params()`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Deriche {
    pub a0: f32,
    pub a1: f32,
    pub a2: f32,
    pub a3: f32,
    pub b1: f32,
    pub b2: f32,
    pub coefp: f32,
    pub coefn: f32,
}

impl Deriche {
    /// darktable `_compute_gauss_params()` for `DT_IOP_GAUSSIAN_ZERO`, in its f32
    /// arithmetic: `e^-2α` computed apart from `(e^-α)²`.
    pub fn new(sigma: f32) -> Self {
        let alpha = 1.695 / sigma;
        let ema = (-alpha).exp();
        let ema2 = (-2.0 * alpha).exp();
        let k = (1.0 - ema) * (1.0 - ema) / (1.0 + (2.0 * alpha * ema) - ema2);
        let (a0, a1, a2, a3) = (
            k,
            k * (alpha - 1.0) * ema,
            k * (alpha + 1.0) * ema,
            -k * ema2,
        );
        let (b1, b2) = (-2.0 * ema, ema2);
        Self {
            a0,
            a1,
            a2,
            a3,
            b1,
            b2,
            coefp: (a0 + a1) / (1.0 + b1 + b2),
            coefn: (a2 + a3) / (1.0 + b1 + b2),
        }
    }
}

/// darktable `modify_roi_in()`'s radius for `blending` percent of the longer side of
/// `full`, at full resolution: `(int)((diameter - 1) / 2)`.
pub fn radius(blending: f32, full: [u32; 2]) -> i32 {
    let diameter = blending / 100.0 * full[0].max(full[1]) as f32;
    ((diameter - 1.0) / 2.0) as i32
}

/// darktable `CONTRAST_FULCRUM`, 2^-4: where EIGF's contrast compensation pivots.
const CONTRAST_FULCRUM: f32 = 0.0625;

/// Bytes of `ToneEqualParams`, the curve, in the kernel's parameters after the pass
/// header.
pub const CURVE_SIZE: usize = 176;
/// Bytes of `MaskParams`, which follow it.
pub const MASK_SIZE: usize = 112;
/// Bytes of the kernel's parameters after the pass header.
pub const PARAMS_SIZE: usize = CURVE_SIZE + MASK_SIZE;

/// The kernels' workgroup side.
const WORKGROUP: u32 = 16;

/// The scratch buffers, in the order of the kernel's bindings 10 to 17.
const SAMPLES: usize = 0;
const PARTIALS: usize = 1;
const RANGES: usize = 2;
const COLUMNS_GUIDE: usize = 3;
const COLUMNS_MASK: usize = 4;
const STATISTICS_GUIDE: usize = 5;
const STATISTICS_MASK: usize = 6;
const MASK: usize = 7;
const SCRATCH: usize = 8;
/// A scratch buffer the settings leave unused: bound, never read.
const UNUSED: u64 = 16;

impl Prepared {
    /// The output pass's parameters: little-endian `ToneEqualParams`, then `MaskParams`,
    /// in the kernel's field order.
    pub fn params(&self) -> Vec<u8> {
        let mut bytes = self.curve_params();
        bytes.extend(self.mask_params());
        debug_assert_eq!(bytes.len(), PARAMS_SIZE);
        bytes
    }

    /// The mask's passes' parameters: zeros in place of the curve, then `MaskParams`, so
    /// a band's change leaves them as they are (decision 15).
    pub fn whole_image_params(&self) -> Vec<u8> {
        let mut bytes = vec![0; CURVE_SIZE];
        bytes.extend(self.mask_params());
        bytes
    }

    fn curve_params(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(PARAMS_SIZE);
        let mut put = |word: [u8; 4]| bytes.extend_from_slice(&word);
        for vector in std::iter::once(self.weights).chain(self.directions) {
            for v in vector {
                put(v.to_le_bytes());
            }
        }
        put(self.inverse_denominator.to_le_bytes());
        for _ in 0..3 {
            put(0u32.to_le_bytes());
        }
        debug_assert_eq!(bytes.len(), CURVE_SIZE);
        bytes
    }

    fn mask_params(&self) -> Vec<u8> {
        let mask = &self.mask;
        let grid = mask.grid;
        let (across, partials) = grid.map_or((0, 0), |grid| grid.partials());
        let [width, height] = grid.map_or([0; 2], |grid| grid.whole);
        let [grid_width, grid_height] = grid.map_or([0; 2], |grid| grid.size);
        let [input_width, input_height] = grid.map_or([0; 2], |grid| grid.input);
        let d = grid.map_or(Deriche::new(1.0), |grid| grid.deriche);
        let mut words: Vec<[u8; 4]> = Vec::with_capacity(MASK_SIZE / 4);
        let uint = |v: u32| v.to_le_bytes();
        let float = |v: f32| v.to_le_bytes();
        words.extend([
            uint(mask.method.to_darktable() as u32),
            uint(mask.filter.to_darktable() as u32),
            uint(mask.iterations),
            uint(u32::from(mask.quantization != 0.0)),
            float(mask.exposure_boost),
            float(mask.fulcrum),
            float(mask.contrast_boost),
            float(mask.feathering),
            float(mask.quantization),
            uint(across),
            uint(partials),
            uint(0),
            uint(width),
            uint(height),
            uint(grid_width),
            uint(grid_height),
            uint(input_width),
            uint(input_height),
            uint(0),
            uint(0),
        ]);
        words.extend([d.a0, d.a1, d.a2, d.a3, d.b1, d.b2, d.coefp, d.coefn].map(float));
        let bytes = words.concat();
        debug_assert_eq!(bytes.len(), MASK_SIZE);
        bytes
    }

    /// The passes, in order (`slang/toneequal.slang`): without a filter, `main` alone;
    /// with EIGF the mask's whole-image passes first, then `main`, which reads their
    /// result. Only the first whole-image pass reads the input, so they are one run, and
    /// an export's tiles take two rounds (decision 15).
    pub fn passes(&self) -> Vec<Pass> {
        let main = self.pass(
            "main",
            PassKind::PerPixel,
            None,
            &[
                (0, Resource::Input),
                (1, Resource::Output),
                (15, Resource::Scratch(STATISTICS_GUIDE)),
                (16, Resource::Scratch(STATISTICS_MASK)),
                (17, Resource::Scratch(MASK)),
            ],
        );
        let Some(grid) = self.mask.grid else {
            return vec![main];
        };
        let [w, h] = grid.size;
        let samples = Some([w, h]);
        let lines = |count: u32| Some([WORKGROUP, count.div_ceil(WORKGROUP)]);
        let scratch = |binding: u32, index: usize| (binding, Resource::Scratch(index));
        let whole = |entry_point, size, bindings: &[(u32, Resource)]| {
            self.pass(entry_point, PassKind::Reduction, size, bindings)
        };
        let iterating = self.mask.iterations > 1;
        let mut passes = Vec::new();
        if iterating {
            // The compensated mask at W × H, from the input; in a tile, its part.
            passes.push(whole(
                "eigf_mask",
                None,
                &[(0, Resource::Input), scratch(17, MASK)],
            ));
        }
        for iteration in 0..self.mask.iterations {
            passes.push(if iterating {
                whole(
                    "eigf_downsample_mask",
                    samples,
                    &[scratch(17, MASK), scratch(10, SAMPLES)],
                )
            } else {
                whole(
                    "eigf_downsample",
                    samples,
                    &[(0, Resource::Input), scratch(10, SAMPLES)],
                )
            });
            passes.extend([
                whole(
                    "eigf_range_partial",
                    samples,
                    &[scratch(10, SAMPLES), scratch(11, PARTIALS)],
                ),
                // One workgroup.
                whole(
                    "eigf_range",
                    Some([WORKGROUP, WORKGROUP]),
                    &[scratch(11, PARTIALS), scratch(12, RANGES)],
                ),
                whole(
                    "eigf_columns",
                    lines(w),
                    &[
                        scratch(10, SAMPLES),
                        scratch(12, RANGES),
                        scratch(13, COLUMNS_GUIDE),
                        scratch(14, COLUMNS_MASK),
                    ],
                ),
                whole(
                    "eigf_rows",
                    lines(h),
                    &[
                        scratch(12, RANGES),
                        scratch(13, COLUMNS_GUIDE),
                        scratch(14, COLUMNS_MASK),
                        scratch(15, STATISTICS_GUIDE),
                        scratch(16, STATISTICS_MASK),
                    ],
                ),
            ]);
            if iteration + 1 < self.mask.iterations {
                passes.push(whole(
                    "eigf_blend",
                    Some(grid.whole),
                    &[
                        scratch(15, STATISTICS_GUIDE),
                        scratch(16, STATISTICS_MASK),
                        scratch(17, MASK),
                    ],
                ));
            }
        }
        passes.push(main);
        passes
    }

    /// A pass of `entry_point` binding the uniforms and `bindings`: the output pass's
    /// parameters for `main`, the mask's alone for the whole-image passes.
    fn pass(
        &self,
        entry_point: &'static str,
        kind: PassKind,
        size: Option<[u32; 2]>,
        bindings: &[(u32, Resource)],
    ) -> Pass {
        let params = match kind {
            PassKind::Reduction => self.whole_image_params(),
            _ => self.params(),
        };
        Pass {
            entry_point,
            kind,
            params,
            tables: Vec::new(),
            bindings: [(2, Resource::Uniforms)]
                .iter()
                .chain(bindings)
                .map(|&(binding, resource)| Binding::new(binding, resource))
                .collect(),
            size,
            strips: None,
        }
    }

    /// The scratch buffers, in bytes, each in the order of the kernel's bindings 10 to
    /// 17: those the settings do not need are 16 bytes, bound and never read, so `main`
    /// binds the same buffers in every mode.
    pub fn scratch(&self) -> Vec<Scratch> {
        let mut bytes = [UNUSED; SCRATCH];
        if let Some(grid) = self.mask.grid {
            let cells = u64::from(grid.size[0]) * u64::from(grid.size[1]);
            let float2 = cells * 8;
            let (_, partials) = grid.partials();
            bytes[SAMPLES] = float2;
            bytes[PARTIALS] = u64::from(partials) * 32;
            bytes[RANGES] = 32;
            bytes[COLUMNS_GUIDE] = float2;
            bytes[STATISTICS_GUIDE] = float2;
            if self.mask.quantization != 0.0 {
                bytes[COLUMNS_MASK] = float2;
                bytes[STATISTICS_MASK] = float2;
            }
            if self.mask.iterations > 1 {
                bytes[MASK] = u64::from(grid.whole[0]) * u64::from(grid.whole[1]) * 4;
            }
        }
        bytes.map(|bytes| Scratch { bytes }).to_vec()
    }

    /// The correction for a mask at `exposure` EV, clamped to the curve's range, as the
    /// kernel computes it, in f64: for tests.
    pub fn correction(&self, exposure: f64) -> f64 {
        let curve = Curve {
            weights: self.weights,
            inverse_denominator: self.inverse_denominator,
        };
        curve.gain(exposure)
    }
}

const OPERATION: &str = "toneequal";

/// darktable's `commit_params` and `modify_roi_in` for the image `geometry` describes,
/// with the four default gradient directions.
pub fn prepare(settings: &Settings, geometry: &Geometry) -> Result<Prepared, NotPorted> {
    prepare_with_directions(settings, geometry, &DEFAULT_DIRECTIONS)
}

/// [`prepare`] with the derivative entry points' four directions in band space.
pub fn prepare_with_directions(
    settings: &Settings,
    geometry: &Geometry,
    directions: &Directions,
) -> Result<Prepared, NotPorted> {
    let mask = mask(settings, geometry)?;
    // darktable's linear gains, exp2f of the bands' EV, fitted as the module's graph draws
    // them. The schema's smoothing range never fails the fit; darktable's own message for a
    // curve that does is "the interpolation is unstable".
    let curve = Curve::new(&settings.bands, settings.smoothing).ok_or(UNSTABLE)?;
    let sigma = f64::from(settings.smoothing);
    let gains = settings.bands.map(|ev| f64::from(ev).exp2());
    // The weights are linear in the gains, so their derivative along a direction v is
    // the fit of ln 2 · gain_i · v_i.
    let mut along = [[0.0; 8]; 4];
    for (weights, direction) in along.iter_mut().zip(directions) {
        let rhs: [f64; 9] =
            std::array::from_fn(|i| std::f64::consts::LN_2 * gains[i] * f64::from(direction[i]));
        *weights = to_f32(tone_equalizer::fit(sigma, &rhs).ok_or(UNSTABLE)?);
    }
    Ok(Prepared {
        weights: curve.weights,
        inverse_denominator: curve.inverse_denominator,
        directions: along,
        mask,
    })
}

/// A curve smoothing whose fit fails: none in the schema's range does.
const UNSTABLE: NotPorted = NotPorted {
    operation: OPERATION,
    what: "a curve smoothing whose fit fails",
};

/// The mask's settings, darktable's `commit_params()`, and for EIGF its grid: the radius
/// from the image at full resolution, truncated there as `modify_roi_in()` truncates it,
/// times the pipe's scale, unrounded, as EIGF's `sigma` (decision 15), and
/// `fast_eigf_surface_blur()`'s downscaling and σ on the grid from it.
fn mask(settings: &Settings, geometry: &Geometry) -> Result<Mask, NotPorted> {
    let (fulcrum, contrast_boost) = match settings.details {
        Filter::None | Filter::AveragedEigf => (0.0, 1.0),
        Filter::Eigf => (CONTRAST_FULCRUM, settings.contrast_boost.exp2()),
        Filter::Guided | Filter::AveragedGuided => {
            return Err(NotPorted {
                operation: OPERATION,
                what: "the guided filter",
            });
        }
    };
    let grid = (settings.details != Filter::None).then(|| {
        let sigma = radius(settings.blending, geometry.full) as f32 * geometry.scale;
        let scaling = sigma.clamp(1.0, 4.0);
        let grid_sigma = (sigma / scaling).max(1.0);
        // darktable's grid is empty below 4 pixels; at least 1, as master's OpenCL path.
        let size = geometry
            .whole
            .map(|side| ((side as f32 / scaling) as u32).max(1));
        Grid {
            whole: geometry.whole,
            input: geometry.input,
            sigma,
            size,
            grid_sigma,
            deriche: Deriche::new(grid_sigma),
        }
    });
    Ok(Mask {
        method: settings.method,
        filter: settings.details,
        iterations: settings.iterations.max(1) as u32,
        exposure_boost: settings.exposure_boost.exp2(),
        fulcrum,
        contrast_boost,
        feathering: 1.0 / settings.feathering,
        quantization: settings.quantization,
        grid,
    })
}

fn to_f32(values: [f64; 8]) -> [f32; 8] {
    values.map(|v| v as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn per_pixel(bands: [f32; 9], smoothing: f32) -> Settings {
        Settings {
            bands,
            smoothing,
            details: Filter::None,
            ..Settings::defaults()
        }
    }

    /// The per-pixel path's geometry: the curve does not depend on the image.
    const WHOLE: Geometry = Geometry {
        full: [1, 1],
        scale: 1.0,
        whole: [1, 1],
        input: [1, 1],
    };

    /// The curve darktable fits through `bands`, in EV, at mask `exposure`.
    fn curve_ev(prepared: &Prepared, exposure: f64) -> f64 {
        prepared.correction(exposure).log2()
    }

    /// Test 0131's bands, the golden cases' `toneequal-max`, `-geomean` and
    /// `-smoothing`.
    const TEST_0131: [f32; 9] = [
        -1.283, -0.788, 0.445, 1.148, 1.321, 0.994, 0.211, -0.449, -0.755,
    ];

    /// darktable `pseudo_solve()` (`src/iop/choleski.h`) restated in f32, in its source's
    /// order: the lower triangles of AᵀA and Aᵀy as sequential sums, then
    /// `_choleski_decompose` (each sum of `L·L` from 0, subtracted from `A` once; failing
    /// on a first pivot of 0 or less, a negative pivot or a zero divisor), the descent and
    /// the ascent, which walks `j` downwards. darktable's fast-math build may fuse or
    /// reorder these sums (`docs/ports/toneequal.md`, Precision). `None` where it fails.
    fn darktable_solve(sigma: f32, gains: [f32; 9]) -> Option<[f32; 8]> {
        let denominator = 2.0 * sigma * sigma;
        let a: [[f32; 8]; 9] = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                let d = BAND_CENTRES[i] - CURVE_CENTRES[j];
                (-d * d / denominator).exp()
            })
        });
        let mut ata = [[0.0f32; 8]; 8];
        let mut aty = [0.0f32; 8];
        for i in 0..8 {
            for j in 0..=i {
                let mut sum = 0.0f32;
                for row in &a {
                    sum += row[i] * row[j];
                }
                ata[i][j] = sum;
            }
            let mut sum = 0.0f32;
            for (row, &y) in a.iter().zip(&gains) {
                sum += row[i] * y;
            }
            aty[i] = sum;
        }
        if ata[0][0] <= 0.0 {
            return None;
        }
        let mut l = [[0.0f32; 8]; 8];
        for i in 0..8 {
            for j in 0..=i {
                let mut sum = 0.0f32;
                for (lik, ljk) in l[i][..j].iter().zip(&l[j][..j]) {
                    sum += lik * ljk;
                }
                if i == j {
                    if ata[i][i] - sum < 0.0 {
                        return None;
                    }
                    l[i][i] = (ata[i][i] - sum).sqrt();
                } else {
                    if l[j][j] == 0.0 {
                        return None;
                    }
                    l[i][j] = (ata[i][j] - sum) / l[j][j];
                }
            }
        }
        // _triangular_descent: L z = Aᵀy.
        let mut z = [0.0f32; 8];
        for i in 0..8 {
            let mut sum = aty[i];
            for (lij, zj) in l[i][..i].iter().zip(&z[..i]) {
                sum -= lij * zj;
            }
            if l[i][i] == 0.0 {
                return None;
            }
            z[i] = sum / l[i][i];
        }
        // _triangular_ascent: Lᵀ x = z, j from the last down.
        let mut x = [0.0f32; 8];
        for i in (0..8).rev() {
            let mut sum = z[i];
            for j in (i + 1..8).rev() {
                sum -= l[j][i] * x[j];
            }
            if l[i][i] == 0.0 {
                return None;
            }
            x[i] = sum / l[i][i];
        }
        Some(x)
    }

    fn darktable_curve_ev(sigma: f32, weights: [f32; 8], exposure: f32) -> f32 {
        let denominator = 2.0 * sigma * sigma;
        let mut result = 0.0f32;
        for (&centre, &w) in CURVE_CENTRES.iter().zip(&weights) {
            let d = exposure - centre;
            result += (-d * d / denominator).exp() * w;
        }
        result.clamp(0.25, 4.0).log2()
    }

    /// The lowest and highest correction over the curve's range, every 0.01 EV.
    fn ripple(prepared: &Prepared) -> (f64, f64) {
        (0..=800)
            .map(|k| prepared.correction(-8.0 + f64::from(k) / 100.0))
            .fold((f64::MAX, f64::MIN), |(lo, hi), c| (lo.min(c), hi.max(c)))
    }

    #[test]
    fn zero_bands_ripple_as_darktables() {
        // darktable's curve, not an identity (decision 22): the fit of eight Gaussians
        // to nine ones ripples, at σ = √2 from 0.9907 to 1.0177, and over the smoothing
        // range, every 0.01, from 0.9708 (σ 0.80) to 1.0661 (σ 0.72)
        // (docs/ports/toneequal.md, Setup).
        let zero = |smoothing| ripple(&prepare(&per_pixel([0.0; 9], smoothing), &WHOLE).unwrap());
        let (low, high) = zero(std::f32::consts::SQRT_2);
        assert!((low - 0.9907).abs() < 2e-4, "{low}");
        assert!((high - 1.0177).abs() < 2e-4, "{high}");
        let (low, high) = (0..190)
            .map(|k| zero(SMOOTHING_MIN + k as f32 / 100.0))
            .fold((f64::MAX, f64::MIN), |(lo, hi), (l, h)| {
                (lo.min(l), hi.max(h))
            });
        assert!((low - 0.9708).abs() < 2e-4, "{low}");
        assert!((high - 1.0661).abs() < 2e-4, "{high}");
    }

    #[test]
    fn a_uniform_offset_scales_the_ripple() {
        // The fit is linear in the gains, so k EV on every band is the zero bands'
        // curve times 2^k, up to the clamp: at most 1.8e-7 apart, from the weights'
        // rounding to f32.
        for smoothing in [1.0, std::f32::consts::SQRT_2, 2.0] {
            let zero = prepare(&per_pixel([0.0; 9], smoothing), &WHOLE).unwrap();
            for k in [-2.0f32, -0.5, 1.0, 2.0] {
                let prepared = prepare(&per_pixel([k; 9], smoothing), &WHOLE).unwrap();
                for step in 0..=80 {
                    let exposure = -8.0 + f64::from(step) / 10.0;
                    let correction = prepared.correction(exposure);
                    let expected =
                        (f64::from(k).exp2() * zero.correction(exposure)).clamp(0.25, 4.0);
                    assert!(
                        (correction / expected - 1.0).abs() < 1e-6,
                        "{k} EV at σ {smoothing}, {exposure} EV: {correction} against {expected}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_fit_matches_darktables_where_darktable_is_reproducible() {
        // Up to σ = 1.5 darktable's f32 solve and an f64 one agree within 1e-3 EV
        // (docs/ports/toneequal.md, Setup): 8e-4 EV at √2, 3e-5 at 1.2. Zero bands
        // too, whose ripple is then darktable's within 2e-5 EV (measured 4.6e-6 at 1.2,
        // 1.6e-5 at √2).
        for bands in [TEST_0131, [0.0; 9]] {
            let zero = bands == [0.0; 9];
            for (smoothing, tolerance) in [(1.2f32, 1e-4), (std::f32::consts::SQRT_2, 1e-3)] {
                let tolerance = if zero { 2e-5 } else { tolerance };
                let prepared = prepare(&per_pixel(bands, smoothing), &WHOLE).unwrap();
                let gains = bands.map(f32::exp2);
                let theirs = darktable_solve(smoothing, gains).expect("darktable's solve succeeds");
                for step in 0..=800 {
                    let exposure = -8.0 + step as f32 / 100.0;
                    let ours = curve_ev(&prepared, f64::from(exposure));
                    let gap =
                        (ours - f64::from(darktable_curve_ev(smoothing, theirs, exposure))).abs();
                    assert!(
                        gap < tolerance,
                        "{bands:?}, σ {smoothing}, {exposure} EV: {gap}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_smoothing_range_is_darktable_s_slider_s() {
        // σ at the slider's ends, -2.33 and 1.67, as `powf(M_SQRT2_F, 1.0f + s)` in f32,
        // correctly rounded from f64.
        let sigma = |s: f32| f64::from(std::f32::consts::SQRT_2).powf(f64::from(1.0 + s)) as f32;
        assert_eq!(SMOOTHING_MIN, sigma(-2.33));
        assert_eq!(SMOOTHING_MAX, sigma(1.67));
    }

    #[test]
    fn the_fit_holds_over_the_whole_smoothing_range() {
        // Where darktable's f32 solve fails, from about 1.98, the f64 one still fits:
        // its weights reproduce the gains the bands' least squares allow, and the
        // residual is as small as at the default smoothing.
        for smoothing in [SMOOTHING_MIN, 1.0, 2.0, 2.2, SMOOTHING_MAX] {
            let sigma = f64::from(smoothing);
            let gains = TEST_0131.map(|ev| f64::from(ev).exp2());
            let x = fit(sigma, &gains).unwrap();
            let a = tone_equalizer::interpolation_matrix(sigma);
            // The normal equations hold: Aᵀ(Ax - y) = 0.
            let residual: [f64; 9] =
                std::array::from_fn(|i| (0..8).map(|j| a[i][j] * x[j]).sum::<f64>() - gains[i]);
            for normal in (0..8).map(|j| (0..9).map(|i| a[i][j] * residual[i]).sum::<f64>()) {
                assert!(normal.abs() < 1e-9, "σ {smoothing}: {normal}");
            }
        }
    }

    #[test]
    fn the_curve_follows_smooth_bands() {
        // A Gaussian bump of +1 EV with a standard deviation of 1 EV is followed within
        // 0.05 EV at every node (docs/ports/toneequal.md, Setup).
        let bands: [f32; 9] = std::array::from_fn(|i| {
            let d = BAND_CENTRES[i] + 4.0;
            (-d * d / 2.0).exp()
        });
        let prepared = prepare(&per_pixel(bands, std::f32::consts::SQRT_2), &WHOLE).unwrap();
        for (i, &centre) in BAND_CENTRES.iter().enumerate() {
            let ev = curve_ev(&prepared, f64::from(centre));
            assert!((ev - f64::from(bands[i])).abs() < 0.05, "{centre} EV: {ev}");
        }
    }

    #[test]
    fn directions_are_the_weights_derivatives() {
        // Against central differences of the fit, along each default direction.
        let settings = per_pixel(TEST_0131, std::f32::consts::SQRT_2);
        let prepared = prepare(&settings, &WHOLE).unwrap();
        let h = 1e-3f64;
        for (k, direction) in DEFAULT_DIRECTIONS.iter().enumerate() {
            let moved = |delta: f64| {
                let gains: [f64; 9] = std::array::from_fn(|i| {
                    (f64::from(settings.bands[i]) + delta * f64::from(direction[i])).exp2()
                });
                fit(f64::from(settings.smoothing), &gains).unwrap()
            };
            let (plus, minus) = (moved(h), moved(-h));
            for j in 0..8 {
                let numeric = (plus[j] - minus[j]) / (2.0 * h);
                let analytic = f64::from(prepared.directions[k][j]);
                assert!(
                    (numeric - analytic).abs() < 1e-5 * analytic.abs().max(1.0),
                    "direction {k}, weight {j}: {numeric} against {analytic}"
                );
            }
        }
    }

    /// The curve's uniforms as the setup gave them before the curve moved to
    /// `apertrust-color` (decision 65), measured on the M4: for each case its bands and
    /// smoothing, its eight weights and `1 / 2σ²`, and the weights' derivatives along the
    /// four default directions, as f32 bits.
    #[allow(clippy::type_complexity, reason = "one recorded case a tuple")]
    const BEFORE: [([f32; 9], f32, [u32; 9], [[u32; 8]; 4]); 4] = [
        (
            [0.0; 9],
            std::f32::consts::SQRT_2,
            [
                1066019424, 3202452325, 1062765668, 1041154187, 1041154170, 1062765670, 3202452321,
                1066019424, 1048576001,
            ],
            [
                [
                    1078411078, 3238341602, 1095973081, 3244813738, 1095502744, 3239451140,
                    1085292610, 3221434198,
                ],
                [
                    3214050509, 1076063480, 3225346833, 1069531139, 1069531169, 3225346843,
                    1076063483, 3214050511,
                ],
                [
                    3221434202, 1085292615, 3239451143, 1095502745, 3244813737, 1095973077,
                    3238341597, 1078411071,
                ],
                [
                    3203999143, 1068211790, 3222837717, 1079992429, 3230687506, 1084580830,
                    3231540830, 1078183784,
                ],
            ],
        ),
        (
            TEST_0131,
            1.2,
            [
                1059235964, 3207849381, 1067712991, 1060054588, 1070520019, 1013422577, 1045576028,
                1054665948, 1051838236,
            ],
            [
                [
                    1067325331, 3226508884, 1085304132, 3232952038, 1083400060, 3226772544,
                    1073875219, 3210352524,
                ],
                [
                    3214898512, 1076302819, 3226147602, 1072552355, 1072552386, 3226147611,
                    1076302823, 3214898514,
                ],
                [
                    3208212516, 1071456692, 3224686015, 1081777513, 3231196267, 1083572952,
                    3224461823, 1065774056,
                ],
                [
                    3183944546, 1047601662, 3200992752, 1058383678, 3209537416, 1065696989,
                    3214615441, 1065531908,
                ],
            ],
        ),
        (
            [-1.0, -0.75, -0.5, -0.25, 0.0, 0.25, 0.5, 0.75, 1.0],
            SMOOTHING_MAX,
            [
                3230513151, 1103048345, 3262279194, 1120616519, 3270717499, 1121245112, 3262926061,
                1101546888, 1033954770,
            ],
            [
                [
                    1141382773, 3305957981, 1167270473, 3318276046, 1170333104, 3313793839,
                    1156438370, 3285762901,
                ],
                [
                    3265929165, 1132906040, 3283335615, 1126970129, 1126970166, 3283335626,
                    1132906043, 3265929167,
                ],
                [
                    3294151515, 1164826984, 3322182449, 1178721713, 3326664653, 1175659080,
                    3314346586, 1149771378,
                ],
                [
                    3273691990, 1144394114, 3302163952, 1159853856, 3308372740, 1158380513,
                    3297832566, 1134159150,
                ],
            ],
        ),
        (
            [2.0, -2.0, 2.0, -2.0, 2.0, -2.0, 2.0, -2.0, 2.0],
            SMOOTHING_MIN,
            [
                1082149250, 3209396946, 1078980181, 1063710817, 1063710856, 1078980173, 3209396921,
                1082149249, 1067509200,
            ],
            [
                [
                    1023635075, 3198197976, 1078357889, 3218011532, 1062430171, 3200621842,
                    1042388797, 3177085061,
                ],
                [
                    3185772034, 1050945482, 3208938017, 1068805844, 1068805861, 3208938034,
                    1050945495, 3185772051,
                ],
                [
                    3177085061, 1042388797, 3200621842, 1062430172, 3218011533, 1078357888,
                    3198197954, 1023635019,
                ],
                [
                    3150055114, 1015869107, 3173801094, 1035499265, 3191168453, 1052288149,
                    3211076222, 1077695968,
                ],
            ],
        ),
    ];

    #[test]
    fn the_curve_is_the_setup_s_from_before_it_moved() {
        // Exact on the M4; elsewhere within an f32 step, should another platform's f64
        // exp round a last bit otherwise.
        let step = |ours: f32, theirs: u32| (i64::from(ours.to_bits()) - i64::from(theirs)).abs();
        for (bands, smoothing, curve, directions) in BEFORE {
            let prepared = prepare(&per_pixel(bands, smoothing), &WHOLE).unwrap();
            let ours = prepared
                .weights
                .into_iter()
                .chain([prepared.inverse_denominator]);
            for (i, (ours, theirs)) in ours.zip(curve).enumerate() {
                assert!(step(ours, theirs) <= 1, "σ {smoothing}, {i}: {ours}");
            }
            for (k, (ours, theirs)) in prepared.directions.iter().zip(directions).enumerate() {
                for (j, (&ours, theirs)) in ours.iter().zip(theirs).enumerate() {
                    assert!(step(ours, theirs) <= 1, "σ {smoothing}, {k}, {j}: {ours}");
                }
            }
        }
    }

    #[test]
    fn the_guided_filters_are_not_ported() {
        for details in [Filter::AveragedGuided, Filter::Guided] {
            let settings = Settings {
                details,
                ..Settings::defaults()
            };
            assert!(prepare(&settings, &WHOLE).is_err(), "{details:?}");
        }
        for details in [Filter::None, Filter::AveragedEigf, Filter::Eigf] {
            let settings = Settings {
                details,
                ..Settings::defaults()
            };
            assert!(prepare(&settings, &WHOLE).is_ok(), "{details:?}");
        }
    }

    /// The golden image, `mire1.cr2`: its readout, which darktable's radius is measured
    /// on, and the developed image the operation gets.
    const MIRE: Geometry = Geometry {
        full: [3944, 2622],
        scale: 1.0,
        whole: [3908, 2602],
        input: [3908, 2602],
    };

    #[test]
    fn the_grid_is_darktables_at_full_resolution() {
        // Blending 5%, 3% and 8% as the tests store them (docs/ports/toneequal.md, Setup
        // and Test plan), of the readout's 3944: radii 98, 58 and 157, so a downscaling
        // of 4, a grid of 977 × 650 and σ 24.5, 14.5 and 39.25 on it; 2%, the iterations
        // case's, 38 and 9.5.
        for (blending, radius_px, grid_sigma) in [
            (5.0, 98, 24.5),
            (2.9999998, 58, 14.5),
            (7.999999, 157, 39.25),
            (2.0, 38, 9.5),
        ] {
            assert_eq!(radius(blending, MIRE.full), radius_px, "{blending}");
            let settings = Settings {
                blending,
                ..Settings::defaults()
            };
            let grid = prepare(&settings, &MIRE).unwrap().mask.grid.unwrap();
            assert_eq!(grid.sigma, radius_px as f32);
            assert_eq!(grid.size, [977, 650], "{blending}");
            assert_eq!(grid.grid_sigma, grid_sigma, "{blending}");
        }
    }

    #[test]
    fn the_radius_scales_unrounded_and_small_radii_downscale_less() {
        let settings = Settings::defaults();
        // At an eighth, σ 98 / 8 = 12.25: still a downscaling of 4, from the image at
        // that scale.
        let eighth = Geometry {
            scale: 0.125,
            whole: [489, 325],
            input: [489, 325],
            ..MIRE
        };
        let grid = prepare(&settings, &eighth).unwrap().mask.grid.unwrap();
        assert_eq!(grid.sigma, 12.25);
        assert_eq!(grid.size, [122, 81]);
        assert_eq!(grid.grid_sigma, 12.25 / 4.0);
        // A radius of 2 downscales by 2; below 1, not at all, with σ 1 on the grid.
        for (blending, scaling, grid_sigma) in [(0.13, 2.0, 1.0), (0.02, 1.0, 1.0)] {
            let settings = Settings {
                blending,
                ..Settings::defaults()
            };
            let grid = prepare(&settings, &MIRE).unwrap().mask.grid.unwrap();
            let expected = [3908, 2602].map(|side: u32| (side as f32 / scaling) as u32);
            assert_eq!(grid.size, expected, "{blending}");
            assert_eq!(grid.grid_sigma, grid_sigma, "{blending}");
        }
    }

    #[test]
    fn deriche_coefficients_are_darktables() {
        // _compute_gauss_params() restated in f64 for σ 24.25: f32 rounding apart.
        let sigma = 24.25f64;
        let alpha = 1.695 / sigma;
        let (ema, ema2) = ((-alpha).exp(), (-2.0 * alpha).exp());
        let k = (1.0 - ema) * (1.0 - ema) / (1.0 + 2.0 * alpha * ema - ema2);
        let d = Deriche::new(sigma as f32);
        for (ours, theirs) in [
            (d.a0, k),
            (d.a1, k * (alpha - 1.0) * ema),
            (d.a2, k * (alpha + 1.0) * ema),
            (d.a3, -k * ema2),
            (d.b1, -2.0 * ema),
            (d.b2, ema2),
        ] {
            assert!((f64::from(ours) - theirs).abs() < 1e-6, "{ours} {theirs}");
        }
        // Order 0 preserves a constant: (a0 + a1 + a2 + a3) / (1 + b1 + b2) = 1.
        let gain = (d.a0 + d.a1 + d.a2 + d.a3) / (1.0 + d.b1 + d.b2);
        assert!((gain - 1.0).abs() < 1e-4, "{gain}");
    }

    #[test]
    fn eigf_takes_one_run_of_whole_image_passes_before_main() {
        // Only the first pass reads the input, whatever the iterations: one run, and two
        // rounds of an export's tiles (decision 15).
        for (iterations, quantization) in [(1, 0.0), (1, 0.5), (5, 0.0), (3, 1.0)] {
            let settings = Settings {
                iterations,
                quantization,
                ..Settings::defaults()
            };
            let prepared = prepare(&settings, &MIRE).unwrap();
            let passes = prepared.passes();
            let n = iterations as usize;
            assert_eq!(passes.len(), 6 * n + usize::from(n > 1), "{iterations}");
            let (main, whole) = passes.split_last().unwrap();
            assert_eq!(main.entry_point, "main");
            assert_eq!(main.kind, PassKind::PerPixel);
            assert!(whole.iter().all(|pass| pass.kind == PassKind::Reduction));
            let reads_input =
                |pass: &Pass| pass.bindings.iter().any(|b| b.resource == Resource::Input);
            assert!(reads_input(&whole[0]));
            assert!(!whole[1..].iter().any(reads_input), "{iterations}");
            // The whole-image passes' uniforms hold no band weight.
            for pass in whole {
                assert_eq!(pass.params, prepared.whole_image_params());
                assert!(pass.params[..CURVE_SIZE].iter().all(|&b| b == 0));
            }
            assert_eq!(main.params, prepared.params());
            let scratch = prepared.scratch();
            assert_eq!(scratch.len(), SCRATCH);
            let used = |index: usize| scratch[index].bytes > UNUSED;
            assert_eq!(used(COLUMNS_MASK), quantization != 0.0);
            assert_eq!(used(STATISTICS_MASK), quantization != 0.0);
            assert_eq!(used(MASK), iterations > 1);
            assert_eq!(scratch[SAMPLES].bytes, 977 * 650 * 8);
        }
    }

    #[test]
    fn a_band_change_leaves_the_whole_image_passes_unchanged() {
        // So a band drag, the module's sliders, is main alone (decision 15).
        let settings = Settings::defaults();
        let moved = Settings {
            bands: TEST_0131,
            smoothing: 1.2,
            ..Settings::defaults()
        };
        let (before, after) = (
            prepare(&settings, &MIRE).unwrap(),
            prepare(&moved, &MIRE).unwrap(),
        );
        let (before, after) = (before.passes(), after.passes());
        for (a, b) in before.iter().zip(&after) {
            assert_eq!(a.kind == PassKind::Reduction, a.params == b.params);
        }
    }

    #[test]
    fn without_a_filter_main_alone_binds_unused_scratch() {
        let prepared = prepare(&per_pixel(TEST_0131, 1.2), &WHOLE).unwrap();
        let passes = prepared.passes();
        assert_eq!(passes.len(), 1);
        assert_eq!(passes[0].entry_point, "main");
        assert!(prepared.scratch().iter().all(|s| s.bytes == UNUSED));
    }

    #[test]
    fn layout_matches_the_kernel() {
        let mut settings = Settings {
            bands: TEST_0131,
            smoothing: 1.2,
            exposure_boost: 0.5,
            contrast_boost: 1.0,
            feathering: 4.0,
            quantization: 0.5,
            method: Estimator::GeometricMean,
            ..Settings::defaults()
        };
        settings.iterations = 3;
        let prepared = prepare(&settings, &MIRE).unwrap();
        let params = prepared.params();
        assert_eq!(params.len(), PARAMS_SIZE);
        let word = |i: usize| u32::from_le_bytes(params[4 * i..4 * i + 4].try_into().unwrap());
        let float = |i: usize| f32::from_bits(word(i));
        assert_eq!(float(0), prepared.weights[0]);
        assert_eq!(float(7), prepared.weights[7]);
        assert_eq!(float(8), prepared.directions[0][0]);
        assert_eq!(float(39), prepared.directions[3][7]);
        assert_eq!(float(40), 1.0 / (2.0 * 1.2 * 1.2));
        assert_eq!([word(41), word(42), word(43)], [0; 3]);
        // MaskParams.
        let m = CURVE_SIZE / 4;
        assert_eq!(
            [word(m), word(m + 1), word(m + 2), word(m + 3)],
            [6, 4, 3, 1]
        );
        assert_eq!(float(m + 4), 0.5f32.exp2());
        assert_eq!(float(m + 5), 0.0625);
        assert_eq!(float(m + 6), 2.0);
        assert_eq!(float(m + 7), 0.25);
        assert_eq!(float(m + 8), 0.5);
        let grid = prepared.mask.grid.unwrap();
        let (across, partials) = grid.partials();
        assert_eq!([word(m + 9), word(m + 10)], [across, partials]);
        assert_eq!([across, partials], [62, 62 * 41]);
        assert_eq!(
            [word(m + 12), word(m + 13), word(m + 14), word(m + 15)],
            [3908, 2602, 977, 650]
        );
        assert_eq!([word(m + 16), word(m + 17)], [3908, 2602]);
        assert_eq!(float(m + 20), grid.deriche.a0);
        assert_eq!(float(m + 27), grid.deriche.coefn);
        assert_eq!(
            prepared.whole_image_params()[CURVE_SIZE..],
            params[CURVE_SIZE..]
        );
    }

    #[test]
    fn darktable_enums_round_trip() {
        for value in 0..5 {
            assert_eq!(Filter::from_darktable(value).unwrap().to_darktable(), value);
        }
        for value in 0..7 {
            assert_eq!(
                Estimator::from_darktable(value).unwrap().to_darktable(),
                value
            );
        }
        assert_eq!(Filter::from_darktable(5), None);
        assert_eq!(Estimator::from_darktable(7), None);
        assert_eq!(Estimator::from_darktable(-1), None);
    }
}
