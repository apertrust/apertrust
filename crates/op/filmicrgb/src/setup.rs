// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2017-2026  darktable developers (ported from src/iop/filmicrgb.c,
// src/iop/gaussian_elimination.h, src/common/gamut_mapping.h and src/common/bspline.h)
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

//! What darktable's `commit_params` and `process` derive from the parameters before
//! the per-pixel work: the spline (`dt_iop_filmic_rgb_compute_spline`, its polynomials
//! solved by `gauss_solve` in f64), the log encoding's and the colour science's
//! coefficients, the highlight reconstruction's, the Yrg matrices, and the passes. See
//! `docs/ports/filmicrgb.md`.

use std::ops::{Add, Div, Mul, Neg, Sub};

use apertrust_color::{
    self as color, LMS_2006_TO_XYZ_D65, Mat3, WorkingProfile, XYZ_D50_TO_D65_CAT16,
    XYZ_D65_TO_D50_CAT16, XYZ_D65_TO_LMS_2006,
};
use apertrust_plugin::{
    Binding, ImageDesc, NotPorted, Pass, PassKind, Prepared, Resource, Scratch,
};

/// darktable `dt_iop_filmicrgb_methods_type_t`: the norm the curve applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Norm {
    None = 0,
    MaxRgb = 1,
    LuminanceY = 2,
    PowerNorm = 3,
    EuclideanLegacy = 4,
    Euclidean = 5,
}

impl Norm {
    /// The schema's choices, in darktable's order, from its descriptions.
    pub const NAMES: [&'static str; 6] = [
        "none",
        "max_rgb",
        "luminance_y",
        "rgb_power_norm",
        "rgb_euclidean_norm_legacy",
        "rgb_euclidean_norm",
    ];

    pub fn from_index(index: u32) -> Option<Self> {
        [
            Self::None,
            Self::MaxRgb,
            Self::LuminanceY,
            Self::PowerNorm,
            Self::EuclideanLegacy,
            Self::Euclidean,
        ]
        .get(index as usize)
        .copied()
    }
}

/// darktable `dt_iop_filmicrgb_colorscience_type_t`, by darktable's descriptions:
/// `DT_FILMIC_COLORSCIENCE_V1` is "v3 (2019)", `V5` "v7 (2023)".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColorScience {
    V3 = 0,
    V4 = 1,
    V5 = 2,
    V6 = 3,
    V7 = 4,
}

impl ColorScience {
    pub const NAMES: [&'static str; 5] = ["v3_2019", "v4_2020", "v5_2021", "v6_2022", "v7_2023"];

    pub fn from_index(index: u32) -> Option<Self> {
        [Self::V3, Self::V4, Self::V5, Self::V6, Self::V7]
            .get(index as usize)
            .copied()
    }
}

/// darktable `dt_iop_filmicrgb_curve_type_t`: the toe's or the shoulder's curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveType {
    /// A 4th-order polynomial.
    Hard = 0,
    /// A 3rd-order polynomial.
    Soft = 1,
    /// A rational function.
    Safe = 2,
}

impl CurveType {
    pub const NAMES: [&'static str; 3] = ["hard", "soft", "safe"];

    pub fn from_index(index: u32) -> Option<Self> {
        [Self::Hard, Self::Soft, Self::Safe]
            .get(index as usize)
            .copied()
    }
}

/// darktable `dt_noise_distribution_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Noise {
    Uniform = 0,
    Gaussian = 1,
    Poissonian = 2,
}

impl Noise {
    pub const NAMES: [&'static str; 3] = ["uniform", "gaussian", "poissonian"];

    pub fn from_index(index: u32) -> Option<Self> {
        [Self::Uniform, Self::Gaussian, Self::Poissonian]
            .get(index as usize)
            .copied()
    }
}

/// darktable `dt_iop_filmicrgb_spline_version_type_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SplineVersion {
    V1 = 0,
    V2 = 1,
    V3 = 2,
}

impl SplineVersion {
    pub const NAMES: [&'static str; 3] = ["v1_2019", "v2_2020", "v3_2021"];

    pub fn from_index(index: u32) -> Option<Self> {
        [Self::V1, Self::V2, Self::V3].get(index as usize).copied()
    }
}

/// darktable `dt_iop_filmicrgb_params_t` version 6, field for field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// %.
    pub grey_point_source: f32,
    /// EV.
    pub black_point_source: f32,
    /// EV.
    pub white_point_source: f32,
    /// EV.
    pub reconstruct_threshold: f32,
    /// EV.
    pub reconstruct_feather: f32,
    /// %.
    pub reconstruct_bloom_vs_details: f32,
    /// %.
    pub reconstruct_grey_vs_color: f32,
    /// %.
    pub reconstruct_structure_vs_texture: f32,
    /// %; read only by darktable's interface.
    pub security_factor: f32,
    /// %.
    pub grey_point_target: f32,
    /// %.
    pub black_point_target: f32,
    /// %.
    pub white_point_target: f32,
    /// The hardness.
    pub output_power: f32,
    /// %.
    pub latitude: f32,
    pub contrast: f32,
    /// %.
    pub saturation: f32,
    /// %.
    pub balance: f32,
    pub noise_level: f32,
    pub preserve_color: Norm,
    pub version: ColorScience,
    /// Read only by darktable's interface.
    pub auto_hardness: bool,
    pub custom_grey: bool,
    pub high_quality_reconstruction: u32,
    pub noise_distribution: Noise,
    pub shadows: CurveType,
    pub highlights: CurveType,
    /// Read by nothing in darktable.
    pub compensate_icc_black: bool,
    pub spline_version: SplineVersion,
    pub enable_highlight_reconstruction: bool,
}

impl Default for Settings {
    /// darktable's introspection defaults, before `reload_defaults` moves the exposures
    /// and the hardness for a raw ([`Settings::for_raw`]).
    fn default() -> Self {
        Self {
            grey_point_source: 18.45,
            black_point_source: -8.0,
            white_point_source: 4.0,
            reconstruct_threshold: 0.0,
            reconstruct_feather: 3.0,
            reconstruct_bloom_vs_details: 100.0,
            reconstruct_grey_vs_color: 100.0,
            reconstruct_structure_vs_texture: 0.0,
            security_factor: 0.0,
            grey_point_target: 18.45,
            black_point_target: 0.01517634,
            white_point_target: 100.0,
            output_power: 4.0,
            latitude: 0.01,
            contrast: 1.0,
            saturation: 0.0,
            balance: 0.0,
            noise_level: 0.2,
            preserve_color: Norm::PowerNorm,
            version: ColorScience::V7,
            auto_hardness: true,
            custom_grey: false,
            high_quality_reconstruction: 1,
            noise_distribution: Noise::Gaussian,
            shadows: CurveType::Hard,
            highlights: CurveType::Hard,
            compensate_icc_black: false,
            spline_version: SplineVersion::V3,
            enable_highlight_reconstruction: false,
        }
    }
}

impl Settings {
    /// darktable `reload_defaults` for an image whose matrix correction it supports (a
    /// raw, not monochrome), in a scene-referred workflow: the exposures moved by the
    /// camera's exposure bias `bias` (EV), and the hardness from them
    /// (`_compute_output_power`).
    pub fn for_raw(mut self, bias: f32) -> Self {
        // darktable `dt_image_get_exposure_bias`: 0 for a value outside ±5 EV.
        let bias = if (-5.0..=5.0).contains(&bias) {
            bias
        } else {
            0.0
        };
        let exposure = 0.7 - bias;
        self.black_point_source += 0.5 * exposure;
        self.white_point_source += 0.8 * exposure;
        self.output_power = self.auto_output_power();
        self
    }

    /// darktable `_compute_output_power`: the hardness that keeps middle grey on the
    /// identity line, clamped to its range.
    pub fn auto_output_power(&self) -> f32 {
        let value = (self.grey_point_target / 100.0).ln()
            / (-self.black_point_source / (self.white_point_source - self.black_point_source)).ln();
        // darktable's CLAMPF, NaN-safe: NaN becomes the minimum.
        if value >= 1.0 { value.min(10.0) } else { 1.0 }
    }
}

/// Floating point as darktable's setup computes it, f32, or in f64 for the
/// derivatives' central differences.
pub trait Real:
    Copy
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Neg<Output = Self>
{
    /// An f32 value, darktable's literals and parameters.
    fn f(value: f32) -> Self;
    /// A double, rounded to f32 where darktable stores one.
    fn of_f64(value: f64) -> Self;
    fn to_f64(self) -> f64;
    /// `powf`: f64's `pow` rounded once, the same on every platform
    /// (`docs/ports/sigmoid.md`, Precision).
    fn powf(self, exponent: Self) -> Self;
    fn exp2(self) -> Self;
    fn sqrt(self) -> Self;
    fn abs(self) -> Self;
    /// `fmaxf` and `fminf`.
    fn max(self, other: Self) -> Self;
    fn min(self, other: Self) -> Self;
}

impl Real for f32 {
    fn f(value: f32) -> Self {
        value
    }
    fn of_f64(value: f64) -> Self {
        value as f32
    }
    fn to_f64(self) -> f64 {
        f64::from(self)
    }
    fn powf(self, exponent: Self) -> Self {
        f64::from(self).powf(f64::from(exponent)) as f32
    }
    fn exp2(self) -> Self {
        f64::from(self).exp2() as f32
    }
    fn sqrt(self) -> Self {
        f32::sqrt(self)
    }
    fn abs(self) -> Self {
        f32::abs(self)
    }
    fn max(self, other: Self) -> Self {
        f32::max(self, other)
    }
    fn min(self, other: Self) -> Self {
        f32::min(self, other)
    }
}

impl Real for f64 {
    fn f(value: f32) -> Self {
        f64::from(value)
    }
    fn of_f64(value: f64) -> Self {
        value
    }
    fn to_f64(self) -> f64 {
        self
    }
    fn powf(self, exponent: Self) -> Self {
        f64::powf(self, exponent)
    }
    fn exp2(self) -> Self {
        f64::exp2(self)
    }
    fn sqrt(self) -> Self {
        f64::sqrt(self)
    }
    fn abs(self) -> Self {
        f64::abs(self)
    }
    fn max(self, other: Self) -> Self {
        f64::max(self, other)
    }
    fn min(self, other: Self) -> Self {
        f64::min(self, other)
    }
}

/// darktable's `CLAMP(x, low, high)`: `x > high ? high : (x < low ? low : x)`.
fn clamp<T: Real>(x: T, low: T, high: T) -> T {
    if x > high {
        high
    } else if x < low {
        low
    } else {
        x
    }
}

/// darktable `gauss_solve` (`src/iop/gaussian_elimination.h`): solves `a · x = b` in
/// place by Gaussian elimination with partial pivoting, `a` an `n` × `n` matrix in row
/// order. `false` for a singular matrix, leaving `b` unsolved, as darktable does.
pub fn gauss_solve(a: &mut [f64], b: &mut [f64], n: usize) -> bool {
    let mut p = vec![0usize; n];
    // gauss_make_triangular
    p[n - 1] = n - 1;
    for k in 0..n {
        let mut m = k;
        for i in k + 1..n {
            if a[k + n * i].abs() > a[k + n * m].abs() {
                m = i;
            }
        }
        p[k] = m;
        a.swap(k + n * m, k + n * k);
        let t1 = a[k + n * k];
        if t1 == 0.0 {
            return false;
        }
        for i in k + 1..n {
            a[k + n * i] /= -t1;
        }
        if k != m {
            for i in k + 1..n {
                a.swap(i + n * m, i + n * k);
            }
        }
        for j in k + 1..n {
            for i in k + 1..n {
                a[i + n * j] += a[k + j * n] * a[i + k * n];
            }
        }
    }
    // gauss_solve_triangular
    for k in 0..n - 1 {
        b.swap(p[k], k);
        let t = b[k];
        for i in k + 1..n {
            b[i] += a[k + n * i] * t;
        }
    }
    for k in (1..n).rev() {
        b[k] /= a[k + n * k];
        let t = b[k];
        for i in 0..k {
            b[i] -= a[k + n * i] * t;
        }
    }
    b[0] /= a[0];
    true
}

/// darktable `SAFETY_MARGIN`.
const SAFETY_MARGIN: f32 = 0.01;

/// darktable `dt_iop_filmic_rgb_spline_t`: the curve's nodes and its pieces' coefficients.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spline<T> {
    /// The nodes in the log encoding: black, toe, grey, shoulder, white.
    pub x: [T; 5],
    /// Their display values.
    pub y: [T; 5],
    /// darktable's M1 to M5 of the toe (`m[0]`), the shoulder (`m[1]`) and the line
    /// (`m[2]`): a polynomial's coefficients of x⁰ to x⁴, or the rational piece's a, b,
    /// c and its node's display value as M1 to M4.
    pub m: [[T; 5]; 3],
    /// The toe's and the shoulder's coefficients as the kernel takes them: a polynomial
    /// piece's Taylor coefficients at its node, d⁰ to d⁴ of `d = x − node`, or the safe
    /// piece's as in `m`.
    pub taylor: [[T; 5]; 2],
    pub latitude_min: T,
    pub latitude_max: T,
    pub toe: CurveType,
    pub shoulder: CurveType,
    /// Whether spline v3 clamped the contrast to its least value, as darktable's
    /// interface warns.
    pub clamped: bool,
}

/// The float parameters the spline and the coefficients read, as `T`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Floats<T> {
    pub grey_point_source: T,
    pub black_point_source: T,
    pub white_point_source: T,
    pub grey_point_target: T,
    pub black_point_target: T,
    pub white_point_target: T,
    pub output_power: T,
    pub latitude: T,
    pub contrast: T,
    pub saturation: T,
    pub balance: T,
}

impl<T: Real> Floats<T> {
    pub fn of(settings: &Settings) -> Self {
        Self {
            grey_point_source: T::f(settings.grey_point_source),
            black_point_source: T::f(settings.black_point_source),
            white_point_source: T::f(settings.white_point_source),
            grey_point_target: T::f(settings.grey_point_target),
            black_point_target: T::f(settings.black_point_target),
            white_point_target: T::f(settings.white_point_target),
            output_power: T::f(settings.output_power),
            latitude: T::f(settings.latitude),
            contrast: T::f(settings.contrast),
            saturation: T::f(settings.saturation),
            balance: T::f(settings.balance),
        }
    }
}

/// darktable `dt_iop_filmic_rgb_compute_spline` on `settings`, in f32 as darktable
/// computes it but for the polynomials' f64 solves.
pub fn compute_spline(settings: &Settings) -> Spline<f32> {
    spline_of(&Floats::of(settings), settings)
}

/// [`compute_spline`] in `T`, from `p`'s floats and `settings`' choices.
pub fn spline_of<T: Real>(p: &Floats<T>, settings: &Settings) -> Spline<T> {
    let f = T::f;
    let zero = f(0.0);
    let one = f(1.0);
    let hundred = f(100.0);
    let grey_display = if settings.custom_grey {
        clamp(
            p.grey_point_target,
            p.black_point_target,
            p.white_point_target,
        ) / hundred
    } else {
        f(0.1845)
    }
    .powf(one / p.output_power);

    let white_source = p.white_point_source;
    let black_source = p.black_point_source;
    let dynamic_range = white_source - black_source;

    let black_log = zero;
    let grey_log = black_source.abs() / dynamic_range;
    let white_log = one;

    let (black_display, white_display) = if settings.spline_version == SplineVersion::V1 {
        (
            clamp(p.black_point_target, zero, p.grey_point_target) / hundred,
            p.white_point_target.max(p.grey_point_target) / hundred,
        )
    } else {
        (
            (clamp(p.black_point_target, zero, p.grey_point_target) / hundred)
                .powf(one / p.output_power),
            (p.white_point_target.max(p.grey_point_target) / hundred).powf(one / p.output_power),
        )
    };

    let balance = clamp(p.balance, f(-50.0), f(50.0)) / hundred;
    let mut clamped = false;
    let (toe_log, shoulder_log, toe_display, shoulder_display, contrast);
    if settings.spline_version < SplineVersion::V3 {
        let latitude = clamp(p.latitude, zero, hundred) / hundred * dynamic_range;
        let slope = clamp(p.contrast, f(1.00001), f(6.0));
        let mut toe = grey_log - latitude / dynamic_range * (black_source / dynamic_range).abs();
        let mut shoulder =
            grey_log + latitude / dynamic_range * (white_source / dynamic_range).abs();
        let linear_intercept = grey_display - slope * grey_log;
        let mut toe_y = toe * slope + linear_intercept;
        let mut shoulder_y = shoulder * slope + linear_intercept;
        let norm = (slope * slope + one).sqrt();
        let coeff = -((f(2.0) * latitude) / dynamic_range) * balance;
        toe_y = toe_y + coeff * slope / norm;
        shoulder_y = shoulder_y + coeff * slope / norm;
        toe = toe + coeff / norm;
        shoulder = shoulder + coeff / norm;
        (
            toe_log,
            shoulder_log,
            toe_display,
            shoulder_display,
            contrast,
        ) = (toe, shoulder, toe_y, shoulder_y, slope);
    } else {
        let hardness = p.output_power;
        let latitude = clamp(p.latitude, zero, hundred) / hundred;
        let slope = p.contrast * dynamic_range / f(8.0);
        let mut min_contrast = one;
        min_contrast = min_contrast.max((white_display - grey_display) / (white_log - grey_log));
        min_contrast = min_contrast.max((grey_display - black_display) / (grey_log - black_log));
        min_contrast = min_contrast + f(SAFETY_MARGIN);
        let raw = slope / (hardness * grey_display.powf(hardness - one));
        let slope = clamp(raw, min_contrast, hundred);
        clamped = slope != raw;
        let linear_intercept = grey_display - slope * grey_log;
        let margin = f(SAFETY_MARGIN) * (white_display - black_display);
        let xmin = (black_display + margin - linear_intercept) / slope;
        let xmax = (white_display - margin - linear_intercept) / slope;
        let mut toe = (one - latitude) * grey_log + latitude * xmin;
        let mut shoulder = (one - latitude) * grey_log + latitude * xmax;
        let correction = if balance > zero {
            f(2.0) * balance * (shoulder - grey_log)
        } else {
            f(2.0) * balance * (grey_log - toe)
        };
        toe = toe - correction;
        shoulder = shoulder - correction;
        toe = toe.max(xmin);
        // darktable's MIN macro: `a < b ? a : b`.
        shoulder = if shoulder < xmax { shoulder } else { xmax };
        (toe_log, shoulder_log, contrast) = (toe, shoulder, slope);
        toe_display = toe_log * contrast + linear_intercept;
        shoulder_display = shoulder_log * contrast + linear_intercept;
    }

    let x = [black_log, toe_log, grey_log, shoulder_log, white_log];
    let y = [
        black_display,
        toe_display,
        grey_display,
        shoulder_display,
        white_display,
    ];
    let mut m = [[zero; 5]; 3];
    // The line: M2 the slope, M1 the offset.
    m[2][1] = contrast;
    m[2][0] = y[1] - contrast * x[1];

    let toe_node = x[1].to_f64();
    let shoulder_node = x[3].to_f64();
    let mut taylor = [[zero; 5]; 2];
    (m[0], taylor[0]) = match settings.shadows {
        CurveType::Hard => {
            let (t, t2, t3, t4) = powers(toe_node);
            let mut a = [
                0.0,
                0.0,
                0.0,
                0.0,
                1.0, // position in 0
                0.0,
                0.0,
                0.0,
                1.0,
                0.0, // first derivative in 0
                t4,
                t3,
                t2,
                t,
                1.0, // position at the toe's node
                4.0 * t3,
                3.0 * t2,
                2.0 * t,
                1.0,
                0.0, // first derivative there
                12.0 * t2,
                6.0 * t,
                2.0,
                0.0,
                0.0, // second derivative there
            ];
            let mut b = [y[0].to_f64(), 0.0, y[1].to_f64(), contrast.to_f64(), 0.0];
            gauss_solve(&mut a, &mut b, 5);
            piece([b[4], b[3], b[2], b[1], b[0]], toe_node, y[1], contrast)
        }
        CurveType::Soft => {
            let (t, t2, t3, _) = powers(toe_node);
            let mut a = [
                0.0,
                0.0,
                0.0,
                1.0, // position in 0
                t3,
                t2,
                t,
                1.0, // position at the toe's node
                3.0 * t2,
                2.0 * t,
                1.0,
                0.0, // first derivative there
                6.0 * t,
                2.0,
                0.0,
                0.0, // second derivative there
            ];
            let mut b = [y[0].to_f64(), y[1].to_f64(), contrast.to_f64(), 0.0];
            gauss_solve(&mut a, &mut b, 4);
            piece([b[3], b[2], b[1], b[0], 0.0], toe_node, y[1], contrast)
        }
        CurveType::Safe => {
            let [a, b, c] = rational(toe_log - black_log, toe_display - black_display, contrast);
            ([a, b, c, toe_display, zero], [a, b, c, toe_display, zero])
        }
    };
    (m[1], taylor[1]) = match settings.highlights {
        CurveType::Soft => {
            let (s, s2, s3, _) = powers(shoulder_node);
            let mut a = [
                1.0,
                1.0,
                1.0,
                1.0, // position in 1
                s3,
                s2,
                s,
                1.0, // position at the shoulder's node
                3.0 * s2,
                2.0 * s,
                1.0,
                0.0, // first derivative there
                6.0 * s,
                2.0,
                0.0,
                0.0, // second derivative there
            ];
            let mut b = [y[4].to_f64(), y[3].to_f64(), contrast.to_f64(), 0.0];
            gauss_solve(&mut a, &mut b, 4);
            piece([b[3], b[2], b[1], b[0], 0.0], shoulder_node, y[3], contrast)
        }
        CurveType::Hard => {
            let (s, s2, s3, s4) = powers(shoulder_node);
            let mut a = [
                1.0,
                1.0,
                1.0,
                1.0,
                1.0, // position in 1
                4.0,
                3.0,
                2.0,
                1.0,
                0.0, // first derivative in 1
                s4,
                s3,
                s2,
                s,
                1.0, // position at the shoulder's node
                4.0 * s3,
                3.0 * s2,
                2.0 * s,
                1.0,
                0.0, // first derivative there
                12.0 * s2,
                6.0 * s,
                2.0,
                0.0,
                0.0, // second derivative there
            ];
            let mut b = [y[4].to_f64(), 0.0, y[3].to_f64(), contrast.to_f64(), 0.0];
            gauss_solve(&mut a, &mut b, 5);
            piece(
                [b[4], b[3], b[2], b[1], b[0]],
                shoulder_node,
                y[3],
                contrast,
            )
        }
        CurveType::Safe => {
            let [a, b, c] = rational(
                white_log - shoulder_log,
                white_display - shoulder_display,
                contrast,
            );
            let safe = [a, b, c, shoulder_display, zero];
            (safe, safe)
        }
    };

    Spline {
        x,
        y,
        m,
        taylor,
        latitude_min: x[1],
        latitude_max: x[3],
        toe: settings.shadows,
        shoulder: settings.highlights,
        clamped,
    }
}

/// A polynomial piece from darktable's f64 solve, `c` its coefficients of x⁰ to x⁴:
/// darktable's monomials rounded to `T`, and the same polynomial around its node `t`,
/// which the kernel evaluates: through the node's display value `node` with the line's
/// slope and no curvature, the solve's conditions there, and the solve's cubic and
/// quartic terms (docs/ports/filmicrgb.md, Deliberate differences).
fn piece<T: Real>(c: [f64; 5], t: f64, node: T, slope: T) -> ([T; 5], [T; 5]) {
    let zero = T::f(0.0);
    let taylor = [
        node,
        slope,
        zero,
        T::of_f64(c[3] + 4.0 * c[4] * t),
        T::of_f64(c[4]),
    ];
    (c.map(T::of_f64), taylor)
}

/// A node's powers in f64, as darktable's `Tl`, `Tl2`, `Tl3`, `Tl4`.
fn powers(t: f64) -> (f64, f64, f64, f64) {
    let t2 = t * t;
    let t3 = t2 * t;
    (t, t2, t3, t3 * t)
}

/// The safe curve's a, b and c over a piece `x` long and `y` high meeting the line of
/// slope `g`, in darktable's f32 arithmetic.
fn rational<T: Real>(x: T, y: T, g: T) -> [T; 3] {
    let f = T::f;
    let b =
        g / (f(2.0) * y) + ((square(x * g / y + f(1.0)) - f(4.0)).sqrt() - f(1.0)) / (f(2.0) * x);
    let c = y / g * (b * square(x) + x) / (b * square(x) + x - (y / g));
    [c * g, b, c]
}

fn square<T: Real>(x: T) -> T {
    x * x
}

/// Values in the kernel's curve, in the order of [`Curve::vectors`].
pub const CURVE_VALUES: usize = 25;

/// What `main` takes from the setup, the per-pixel tone mapping's coefficients:
/// darktable's spline, `dt_iop_filmicrgb_data_t`'s log encoding and desaturation, and
/// what `process` adds, in the order the kernel's `Curve` holds them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Curve<T> {
    /// The toe's and the shoulder's coefficients ([`Spline::taylor`]), and the line's
    /// offset and slope, darktable's M1 and M2.
    pub m: [[T; 5]; 3],
    pub latitude_min: T,
    pub latitude_max: T,
    pub grey_source: T,
    pub black_source: T,
    pub dynamic_range: T,
    pub output_power: T,
    /// darktable's `d->saturation`: `p / 100` for v6 and v7, `2p / 100 + 1` before.
    pub saturation: T,
    pub sigma_toe: T,
    pub sigma_shoulder: T,
    /// `y₀^hardness` and `y₄^hardness`, as `process` computes them.
    pub display_black: T,
    pub display_white: T,
    /// The scene values that encode to 0 and 1 (`exp_tonemapping_v2`), for v6 and v7.
    pub norm_min: T,
    pub norm_max: T,
}

impl<T: Real> Curve<T> {
    /// The kernel's `curve` uniforms: seven `float4`s, the last three words zero.
    pub fn vectors(&self) -> [[T; 4]; 7] {
        let m = &self.m;
        let zero = T::f(0.0);
        [
            [m[0][0], m[0][1], m[0][2], m[0][3]],
            [m[1][0], m[1][1], m[1][2], m[1][3]],
            [m[0][4], m[1][4], m[2][0], m[2][1]],
            [
                self.latitude_min,
                self.latitude_max,
                self.grey_source,
                self.black_source,
            ],
            [
                self.dynamic_range,
                self.output_power,
                self.saturation,
                self.sigma_toe,
            ],
            [
                self.sigma_shoulder,
                self.display_black,
                self.display_white,
                self.norm_min,
            ],
            [self.norm_max, zero, zero, zero],
        ]
    }
}

/// The curve's coefficients for `settings` in f32, as darktable's `commit_params` and
/// `process` compute them.
pub fn curve(settings: &Settings) -> Curve<f32> {
    curve_of(&Floats::of(settings), settings)
}

/// The least custom grey source v6 and v7 and the reconstruction take, 0.001 %, the least
/// darktable's picker sets (`apply_auto_grey`): a guard, since darktable's slider reaches
/// 0, where its v6 and v7 render NaN and its reconstruction's threshold of 0 fills the
/// image with NaN. v3 to v5 render a grey source of 0 as darktable does, every pixel
/// encoded to 1 ([`encodes_to_white`]; docs/ports/filmicrgb.md, Deliberate differences).
pub const GREY_SOURCE_MIN: f32 = 1e-5;

/// Whether v3 to v5 encode every pixel to 1: with a custom grey source of 0, darktable's
/// `log2f(x / 0)` is infinite for every pixel, which its clamp takes to 1, a uniform
/// white. The kernel's flag gives that encoding without the division.
pub fn encodes_to_white(settings: &Settings) -> bool {
    settings.custom_grey && settings.version < ColorScience::V6 && settings.grey_point_source <= 0.0
}

/// [`curve`] in `T`.
pub fn curve_of<T: Real>(p: &Floats<T>, settings: &Settings) -> Curve<T> {
    let f = T::f;
    let spline = spline_of(p, settings);
    let grey_source = if !settings.custom_grey {
        f(0.1845)
    } else if encodes_to_white(settings) {
        // Never read: every pixel encodes to 1 (`encodes_to_white`).
        f(1.0)
    } else if settings.version >= ColorScience::V6 {
        (p.grey_point_source / f(100.0)).max(f(GREY_SOURCE_MIN))
    } else {
        p.grey_point_source / f(100.0)
    };
    let dynamic_range = p.white_point_source - p.black_point_source;
    let black_source = p.black_point_source;
    let saturation = if settings.version >= ColorScience::V6 {
        p.saturation / f(100.0)
    } else {
        f(2.0) * p.saturation / f(100.0) + f(1.0)
    };
    Curve {
        m: [spline.taylor[0], spline.taylor[1], spline.m[2]],
        latitude_min: spline.latitude_min,
        latitude_max: spline.latitude_max,
        grey_source,
        black_source,
        dynamic_range,
        output_power: p.output_power,
        saturation,
        sigma_toe: (spline.latitude_min / f(3.0)).powf(f(2.0)),
        sigma_shoulder: ((f(1.0) - spline.latitude_max) / f(3.0)).powf(f(2.0)),
        display_black: spline.y[0].powf(p.output_power),
        display_white: spline.y[4].powf(p.output_power),
        norm_min: grey_source * (dynamic_range * f(0.0) + black_source).exp2(),
        norm_max: grey_source * (dynamic_range * f(1.0) + black_source).exp2(),
    }
}

/// A parameter the derivative entry points differentiate along (`docs/ports/filmicrgb.md`,
/// Gradients). None of them reaches the reconstruction's passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Contrast,
    /// As a fraction, the schema's % over 100.
    Latitude,
    /// The hardness.
    OutputPower,
    /// As a fraction, the schema's % over 100.
    Saturation,
}

impl Direction {
    /// The schema parameter it moves.
    pub fn parameter(self) -> &'static str {
        match self {
            Self::Contrast => "contrast",
            Self::Latitude => "latitude",
            Self::OutputPower => "output_power",
            Self::Saturation => "saturation",
        }
    }

    /// The schema parameter's change per unit along the direction: 100 for the
    /// percentages, which the derivatives take as fractions, the kernel's own units.
    pub fn scale(self) -> f32 {
        match self {
            Self::Latitude | Self::Saturation => 100.0,
            Self::Contrast | Self::OutputPower => 1.0,
        }
    }

    fn field(self, floats: &mut Floats<f64>) -> &mut f64 {
        match self {
            Self::Contrast => &mut floats.contrast,
            Self::Latitude => &mut floats.latitude,
            Self::OutputPower => &mut floats.output_power,
            Self::Saturation => &mut floats.saturation,
        }
    }
}

/// The derivative entry points' four gradient slots, in order. Not the black relative
/// exposure: with the contrast set per EV at grey, spline v3 keeps most of the image
/// where it was as the black moves, and its derivatives are near-cancelling sums too
/// small for the gradient check to resolve (docs/ports/filmicrgb.md, Gradients).
pub const DIRECTIONS: [Direction; 4] = [
    Direction::Contrast,
    Direction::OutputPower,
    Direction::Latitude,
    Direction::Saturation,
];

/// The curve's derivative along each of [`DIRECTIONS`], per unit of the direction
/// ([`Direction::scale`]): central differences of [`curve_of`] in f64.
pub fn directions(settings: &Settings) -> [[[f64; 4]; 7]; 4] {
    let base = Floats::<f64>::of(settings);
    DIRECTIONS.map(|direction| {
        let mut plus = base;
        let mut minus = base;
        let value = *direction.field(&mut plus);
        let step = 1e-6 * value.abs().max(1.0);
        *direction.field(&mut plus) = value + step;
        *direction.field(&mut minus) = value - step;
        let (a, b) = (
            curve_of(&plus, settings).vectors(),
            curve_of(&minus, settings).vectors(),
        );
        let scale = f64::from(direction.scale());
        std::array::from_fn(|v| std::array::from_fn(|k| (a[v][k] - b[v][k]) / (2.0 * step) * scale))
    })
}

/// darktable `MAX_NUM_SCALES`.
const MAX_SCALES: i32 = 10;

/// darktable `get_scales`: how many wavelet scales the reconstruction takes for an image
/// whose longer side is `size` pixels at full resolution, at the pipe's `scale`.
pub fn scales(size: u32, scale: f32) -> u32 {
    // (BSPLINE_FSIZE - 1) * BSPLINE_FSIZE, 5 taps.
    let scales = ((2.0 * size as f32 * scale / 20.0) - 1.0).log2().floor();
    // darktable's (int) of floorf, then CLAMP; `as` saturates, as arm64 converts.
    (scales as i32).clamp(1, MAX_SCALES) as u32
}

/// What the reconstruction's passes read, from `commit_params` and `process`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reconstruction {
    pub threshold: f32,
    pub feathering: f32,
    pub normalize: f32,
    /// Structure ↔ texture, grey ↔ colour, bloom ↔ reconstruct, each `(p / 100 + 1) / 2`.
    pub gamma: f32,
    pub beta: f32,
    pub delta: f32,
    /// The noise level at the pipe's scale.
    pub noise_level: f32,
}

impl Reconstruction {
    pub fn of(settings: &Settings, scale: f32) -> Self {
        let grey_source = if settings.custom_grey {
            (settings.grey_point_source / 100.0).max(GREY_SOURCE_MIN)
        } else {
            0.1845
        };
        let threshold = Real::powf(
            2.0f32,
            settings.white_point_source + settings.reconstruct_threshold,
        ) * grey_source;
        let feathering = Real::exp2(12.0f32 / settings.reconstruct_feather);
        let balance = |p: f32| (p / 100.0 + 1.0) / 2.0;
        // darktable `fmaxf(piece->iscale / roi_in->scale, 1.f)`.
        let downscaling = (1.0 / scale).max(1.0);
        Self {
            threshold,
            feathering,
            normalize: feathering / threshold,
            gamma: balance(settings.reconstruct_structure_vs_texture),
            beta: balance(settings.reconstruct_grey_vs_color),
            delta: balance(settings.reconstruct_bloom_vs_details),
            noise_level: settings.noise_level / downscaling,
        }
    }
}

/// The working profile's RGB to CIE 2006 LMS and back (`prepare_RGB_Yrg_matrices`), in
/// f32 as darktable multiplies them.
pub fn yrg_matrices(profile: &WorkingProfile) -> (Mat3, Mat3) {
    let to_lms = color::mul(
        &XYZ_D65_TO_LMS_2006,
        &color::mul(&XYZ_D50_TO_D65_CAT16, &profile.rgb_to_xyz_d50),
    );
    let from_lms = color::mul(
        &profile.xyz_d50_to_rgb,
        &color::mul(&XYZ_D65_TO_D50_CAT16, &LMS_2006_TO_XYZ_D65),
    );
    (to_lms, from_lms)
}

/// The kernel's `variant` of a wavelet pass: darktable `DT_FILMIC_RECONSTRUCT_RGB` and
/// `_RATIOS`.
const RGB: u32 = 0;
const RATIOS: u32 = 1;

/// The uniform block after the pass header, as `FilmicParams` in the kernel lays it out;
/// each pass fills only what it reads (decision 15).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Uniforms {
    /// Width and height at the pipe's scale, the partial counts' workgroups across, the
    /// blur's step.
    image: [u32; 4],
    /// The scale, the last scale, the variant, the noise's distribution.
    wavelet: [u32; 4],
    /// normalize, feathering, the noise level, the threshold.
    mask: [f32; 4],
    /// gamma, beta, delta.
    balance: [f32; 4],
    /// The colour science, the norm, the toe's and the shoulder's curves.
    modes: [u32; 4],
    /// Whether reconstruction runs, the partial counts.
    flags: [u32; 4],
    luminance: [f32; 3],
    to_lms: Mat3,
    from_lms: Mat3,
    export_to_lms: Mat3,
    export_from_lms: Mat3,
    curve: [[f32; 4]; 7],
    directions: [[[f32; 4]; 7]; 4],
}

/// Bytes of `FilmicParams`.
pub const PARAMS_SIZE: usize = 16 * (6 + 1 + 12 + 7 + 28);

impl Uniforms {
    fn to_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(PARAMS_SIZE);
        let mut words = |values: &[u32]| {
            for value in values {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        };
        let bits = |values: &[f32]| values.iter().map(|v| v.to_bits()).collect::<Vec<_>>();
        words(&self.image);
        words(&self.wavelet);
        words(&bits(&self.mask));
        words(&bits(&self.balance));
        words(&self.modes);
        words(&self.flags);
        words(&bits(&[
            self.luminance[0],
            self.luminance[1],
            self.luminance[2],
            0.0,
        ]));
        for matrix in [
            &self.to_lms,
            &self.from_lms,
            &self.export_to_lms,
            &self.export_from_lms,
        ] {
            for row in matrix {
                words(&bits(&[row[0], row[1], row[2], 0.0]));
            }
        }
        for vector in self.curve.iter().chain(self.directions.iter().flatten()) {
            words(&bits(vector));
        }
        bytes
    }
}

/// Where the operation's scratch buffers sit in `Prepared::scratch` while reconstruction
/// is on.
const MASK: usize = 0;
const COUNTS: usize = 1;
const BUFFER_A: usize = 2;
const BUFFER_B: usize = 3;
const TEMP: usize = 4;
const RECONSTRUCTED: usize = 5;
const NORMS: usize = 6;

/// A pass of the kernel with its uniforms and bindings, `size` invocations or the
/// output's.
fn pass(
    entry_point: &'static str,
    uniforms: Uniforms,
    size: Option<[u32; 2]>,
    bindings: &[(u32, Resource)],
) -> Pass {
    Pass {
        entry_point,
        kind: if entry_point == "main" {
            PassKind::PerPixel
        } else {
            PassKind::Reduction
        },
        params: uniforms.to_bytes(),
        tables: Vec::new(),
        bindings: bindings
            .iter()
            .map(|&(binding, resource)| Binding::new(binding, resource))
            .collect(),
        size,
        strips: None,
    }
}

const INPUT: (u32, Resource) = (0, Resource::Input);
const OUTPUT: (u32, Resource) = (1, Resource::Output);
const UNIFORMS: (u32, Resource) = (2, Resource::Uniforms);
fn mask() -> (u32, Resource) {
    (10, Resource::Scratch(MASK))
}
fn detail(buffer: usize) -> (u32, Resource) {
    (12, Resource::Scratch(buffer))
}
fn low(buffer: usize) -> (u32, Resource) {
    (13, Resource::Scratch(buffer))
}
fn temp() -> (u32, Resource) {
    (14, Resource::Scratch(TEMP))
}
fn norms() -> (u32, Resource) {
    (16, Resource::Scratch(NORMS))
}

/// The image the operation runs on, as darktable's `get_scales` and noise level see it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    /// The longer side at full resolution.
    pub size: u32,
    /// Pixels at the pipe's scale per pixel at full resolution, at most 1.
    pub scale: f32,
}

/// The passes for `settings` on `input`, in the working space `profile`, gamut-mapping
/// v6 and v7 against `export`, the pipeline's output profile (darktable's `process`).
pub fn prepare(
    settings: &Settings,
    input: &ImageDesc,
    geometry: Geometry,
    profile: &WorkingProfile,
    export: &WorkingProfile,
) -> Result<Prepared, NotPorted> {
    if settings.spline_version != SplineVersion::V3 {
        return Err(NotPorted {
            operation: "filmicrgb",
            what: "spline handling v1 and v2, which only a params version 5 edit holds",
        });
    }
    let (width, height) = (input.width, input.height);
    let across = width.div_ceil(16);
    let partials = across * height.div_ceil(16);
    let image = [width, height, across, 0];
    let reconstruct = settings.enable_highlight_reconstruction;

    let (to_lms, from_lms) = yrg_matrices(profile);
    let (export_to_lms, export_from_lms) = yrg_matrices(export);
    let curve = curve(settings);
    let directions = directions(settings).map(|vectors| vectors.map(|v| v.map(|d| d as f32)));
    let main = Uniforms {
        image,
        modes: [
            settings.version as u32,
            settings.preserve_color as u32,
            settings.shadows as u32,
            settings.highlights as u32,
        ],
        flags: [
            u32::from(reconstruct),
            0,
            u32::from(encodes_to_white(settings)),
            0,
        ],
        luminance: profile.rgb_to_xyz_d50[1],
        to_lms,
        from_lms,
        export_to_lms,
        export_from_lms,
        curve: curve.vectors(),
        directions,
        ..Uniforms::default()
    };

    if !reconstruct {
        // `main` alone, its scratch bindings 16-byte buffers it never reads.
        let main = pass(
            "main",
            main,
            None,
            &[
                INPUT,
                OUTPUT,
                UNIFORMS,
                (11, Resource::Scratch(0)),
                (15, Resource::Scratch(1)),
            ],
        );
        return Ok(Prepared {
            output: *input,
            passes: vec![main],
            scratch: vec![Scratch { bytes: 16 }; 2],
        });
    }

    let r = Reconstruction::of(settings, geometry.scale);
    let scales = scales(geometry.size, geometry.scale);
    let mut passes = vec![
        pass(
            "hr_mask",
            Uniforms {
                image,
                mask: [r.normalize, r.feathering, 0.0, 0.0],
                ..Uniforms::default()
            },
            None,
            &[INPUT, UNIFORMS, mask(), (11, Resource::Scratch(COUNTS))],
        ),
        pass(
            "hr_count",
            Uniforms {
                flags: [0, partials, 0, 0],
                ..Uniforms::default()
            },
            Some([16, 16]),
            &[UNIFORMS, (11, Resource::Scratch(COUNTS))],
        ),
        pass(
            "hr_inpaint",
            Uniforms {
                image,
                wavelet: [0, 0, 0, settings.noise_distribution as u32],
                mask: [0.0, 0.0, r.noise_level, r.threshold],
                ..Uniforms::default()
            },
            None,
            &[INPUT, UNIFORMS, mask(), detail(BUFFER_A)],
        ),
    ];
    wavelets(&mut passes, image, scales, RGB, &r);
    for _ in 0..settings.high_quality_reconstruction {
        passes.push(pass(
            "hr_ratios",
            Uniforms {
                image,
                ..Uniforms::default()
            },
            None,
            &[UNIFORMS, detail(BUFFER_A), reconstructed(), norms()],
        ));
        wavelets(&mut passes, image, scales, RATIOS, &r);
        passes.push(pass(
            "hr_restore",
            Uniforms {
                image,
                ..Uniforms::default()
            },
            None,
            &[UNIFORMS, reconstructed(), norms()],
        ));
    }
    passes.push(pass(
        "main",
        main,
        None,
        &[
            INPUT,
            OUTPUT,
            UNIFORMS,
            (11, Resource::Scratch(COUNTS)),
            reconstructed(),
        ],
    ));

    let pixels = u64::from(width) * u64::from(height);
    let mut scratch = vec![
        Scratch { bytes: pixels * 4 },
        Scratch {
            bytes: (1 + u64::from(partials)) * 4,
        },
        Scratch { bytes: pixels * 16 },
        Scratch { bytes: pixels * 16 },
        Scratch { bytes: pixels * 16 },
        Scratch { bytes: pixels * 16 },
    ];
    if settings.high_quality_reconstruction > 0 {
        scratch.push(Scratch { bytes: pixels * 4 });
    }
    Ok(Prepared {
        output: *input,
        passes,
        scratch,
    })
}

fn reconstructed() -> (u32, Resource) {
    (15, Resource::Scratch(RECONSTRUCTED))
}

/// darktable `reconstruct_highlights` on buffer A, the inpainted image or the ratios:
/// the reconstructed image initialised from it, then each scale's blurs and its
/// reconstruction, the detail and the low frequencies swapping buffers A and B from
/// scale to scale as darktable's `LF_odd` and `LF_even` do.
fn wavelets(
    passes: &mut Vec<Pass>,
    image: [u32; 4],
    scales: u32,
    variant: u32,
    r: &Reconstruction,
) {
    passes.push(pass(
        "hr_init",
        Uniforms {
            image,
            ..Uniforms::default()
        },
        None,
        &[UNIFORMS, mask(), detail(BUFFER_A), reconstructed()],
    ));
    for s in 0..scales {
        let (d, l) = if s % 2 == 0 {
            (BUFFER_A, BUFFER_B)
        } else {
            (BUFFER_B, BUFFER_A)
        };
        let step = |mult: u32| Uniforms {
            image: [image[0], image[1], image[2], mult],
            ..Uniforms::default()
        };
        passes.push(pass(
            "hr_blur_v",
            step(1 << s),
            None,
            &[UNIFORMS, detail(d), temp()],
        ));
        passes.push(pass(
            "hr_blur_h",
            step(1 << s),
            None,
            &[UNIFORMS, detail(d), low(l), temp()],
        ));
        passes.push(pass(
            "hr_blur_v",
            step(1),
            None,
            &[UNIFORMS, detail(d), temp()],
        ));
        passes.push(pass(
            "hr_reconstruct",
            Uniforms {
                image,
                wavelet: [s, scales - 1, variant, 0],
                balance: [r.gamma, r.beta, r.delta, 0.0],
                ..Uniforms::default()
            },
            None,
            &[UNIFORMS, mask(), detail(d), low(l), temp(), reconstructed()],
        ));
    }
}

/// darktable `splitmix32` (`src/develop/noise_generator.h`), the kernel's twin.
pub fn splitmix32(seed: u64) -> u32 {
    let mut result = (seed ^ (seed >> 33)).wrapping_mul(0x62a9_d9ed_7997_05f5);
    result = (result ^ (result >> 28)).wrapping_mul(0xcb24_d0a5_c88c_35b3);
    (result >> 32) as u32
}

/// darktable `xoshiro128plus`: the next uniform draw in [0, 1), 24 bits, advancing
/// `state`.
pub fn xoshiro128plus(state: &mut [u32; 4]) -> f32 {
    let result = state[0].wrapping_add(state[3]);
    let t = state[1] << 9;
    state[2] ^= state[0];
    state[3] ^= state[1];
    state[1] ^= state[2];
    state[0] ^= state[3];
    state[2] ^= t;
    state[3] = state[3].rotate_left(11);
    (result >> 8) as f32 * f32::from_bits(0x3380_0000)
}

/// The noise generator's state for the pixel at row `i`, column `j`, as `inpaint_noise`
/// seeds and advances it.
pub fn noise_state(i: u32, j: u32) -> [u32; 4] {
    let (i, j) = (u64::from(i), u64::from(j));
    let mut state = [
        splitmix32(j + 1),
        splitmix32((j + 1) * (i + 3)),
        splitmix32(1337),
        splitmix32(666),
    ];
    for _ in 0..4 {
        xoshiro128plus(&mut state);
    }
    state
}

/// darktable `dt_vector_log2`: log₂ from the float's exponent and a degree-5 fit of its
/// mantissa.
#[allow(clippy::excessive_precision)]
pub fn log2_dt(x: f32) -> f32 {
    let bits = x.to_bits();
    let mantissa = f32::from_bits((bits & 0x007F_FFFF) | 0x3F80_0000);
    let exponent = ((bits & 0x7F80_0000) >> 23) as f32 - 127.0;
    // darktable's literals, LOG_POLY_DEGREE 5.
    let fit = ((((0.0596515482674574969533_f32 * mantissa - 0.465725644288844778798) * mantissa
        + 1.48116647521213171641)
        * mantissa
        - 2.52074962577807006663)
        * mantissa)
        + 2.8882704548164776201;
    fit * (mantissa - 1.0) + exponent
}

/// darktable `dt_vector_exp2`: 2ˣ from the exponent built from x's integer part, rounded
/// half away from zero after subtracting ½ (`roundf`, darktable's arm64 path), and a
/// degree-4 fit of the fraction.
pub fn exp2_dt(x: f32) -> f32 {
    let x = x.clamp(-126.99999, 129.0);
    let ipart = (x - 0.5).round();
    let fpart = x - ipart;
    let exponent = f32::from_bits(((127 + ipart as i32) as u32) << 23);
    // darktable's literals, EXP_POLY_DEGREE 4.
    let fit = (((1.3534167e-2_f32 * fpart + 5.2011464e-2) * fpart + 2.4144275e-1) * fpart
        + 6.9300383e-1)
        * fpart
        + 1.0000026;
    exponent * fit
}

/// darktable `dt_vector_powf`: `exp2_dt(p · log2_dt(x))`.
pub fn pow_dt(x: f32, p: f32) -> f32 {
    exp2_dt(log2_dt(x) * p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> WorkingProfile {
        WorkingProfile::linear_rec2020()
    }

    fn geometry() -> Geometry {
        Geometry {
            size: 640,
            scale: 1.0,
        }
    }

    #[test]
    fn gauss_solve_solves() {
        let mut a = [2.0, 1.0, -1.0, -3.0, -1.0, 2.0, -2.0, 1.0, 2.0];
        let mut b = [8.0, -11.0, -3.0];
        assert!(gauss_solve(&mut a, &mut b, 3));
        for (x, expected) in b.iter().zip([2.0, 3.0, -1.0]) {
            assert!((x - expected).abs() < 1e-12, "{b:?}");
        }
        let mut singular = [1.0, 2.0, 2.0, 4.0];
        let mut b = [1.0, 2.0];
        assert!(!gauss_solve(&mut singular, &mut b, 2));
    }

    /// The spline's value at `x`, as the kernel's `spline`.
    fn evaluate(spline: &Spline<f64>, x: f64) -> f64 {
        let m = &spline.m;
        let piece = |part: usize, kind: CurveType, xi: f64, sign: f64| match kind {
            CurveType::Hard => {
                m[part][0] + x * (m[part][1] + x * (m[part][2] + x * (m[part][3] + x * m[part][4])))
            }
            CurveType::Soft => m[part][0] + x * (m[part][1] + x * (m[part][2] + x * m[part][3])),
            CurveType::Safe => {
                let rat = xi * (xi * m[part][1] + 1.0);
                m[part][3] + sign * m[part][0] * rat / (rat + m[part][2])
            }
        };
        if x < spline.latitude_min {
            piece(0, spline.toe, spline.latitude_min - x, -1.0)
        } else if x > spline.latitude_max {
            piece(1, spline.shoulder, x - spline.latitude_max, 1.0)
        } else {
            m[2][0] + x * m[2][1]
        }
    }

    #[test]
    fn every_curve_meets_its_nodes_and_the_line() {
        for shadows in [CurveType::Hard, CurveType::Soft, CurveType::Safe] {
            for highlights in [CurveType::Hard, CurveType::Soft, CurveType::Safe] {
                let settings = Settings {
                    shadows,
                    highlights,
                    latitude: 30.0,
                    contrast: 1.2,
                    ..Settings::default()
                };
                let spline = spline_of(&Floats::<f64>::of(&settings), &settings);
                let (x, y) = (spline.x, spline.y);
                for (node, expected) in x.iter().zip(&y) {
                    let value = evaluate(&spline, *node);
                    assert!(
                        (value - expected).abs() < 1e-6,
                        "{shadows:?}/{highlights:?}: {value} at {node}, not {expected}"
                    );
                }
                // The pieces join the line with its slope.
                let slope = spline.m[2][1];
                for node in [x[1], x[3]] {
                    let h = 1e-6;
                    let left = (evaluate(&spline, node) - evaluate(&spline, node - h)) / h;
                    let right = (evaluate(&spline, node + h) - evaluate(&spline, node)) / h;
                    assert!((left - slope).abs() < 1e-3 * slope, "{left} {slope}");
                    assert!((right - slope).abs() < 1e-3 * slope, "{right} {slope}");
                }
                // The safe curves rise from the black target to the white target. The
                // polynomials need not: darktable's hard toe dips below black near 0 here,
                // a curve its interface warns about.
                if (shadows, highlights) == (CurveType::Safe, CurveType::Safe) {
                    let mut previous = evaluate(&spline, 0.0);
                    for k in 1..=100 {
                        let value = evaluate(&spline, f64::from(k) / 100.0);
                        assert!(value >= previous - 1e-9, "safe at {k}");
                        previous = value;
                    }
                }
            }
        }
    }

    #[test]
    fn the_default_spline_maps_grey_to_its_display_value() {
        let settings = Settings::default();
        let spline = compute_spline(&settings);
        // Grey at |black| / range in the log encoding, 0.1845^(1/4) on the display.
        assert_eq!(spline.x[2], 8.0 / 12.0);
        assert_eq!(spline.y[2], Real::powf(0.1845f32, 0.25));
        assert!(!spline.clamped);
        let curve = curve(&settings);
        // Hardness 4 takes it back to 18.45 %.
        let grey = Real::powf(
            curve.m[2][0] + spline.x[2] * curve.m[2][1],
            curve.output_power,
        );
        assert!((grey - 0.1845).abs() < 1e-6, "{grey}");
        assert_eq!(curve.saturation, 0.0);
        assert!((curve.norm_min - 0.1845 / 256.0).abs() < 1e-9);
        assert!((curve.norm_max - 0.1845 * 16.0).abs() < 1e-6);
    }

    #[test]
    fn the_saturation_coefficient_follows_the_colour_science() {
        let at = |version| {
            curve(&Settings {
                version,
                saturation: 10.0,
                ..Settings::default()
            })
            .saturation
        };
        assert_eq!(at(ColorScience::V7), 0.1);
        assert_eq!(at(ColorScience::V6), 0.1);
        assert_eq!(at(ColorScience::V5), 1.2);
        assert_eq!(at(ColorScience::V3), 1.2);
    }

    #[test]
    fn a_grey_source_of_0_encodes_white_in_v3_to_v5_and_is_floored_in_v6_and_v7() {
        let input = ImageDesc::rgba(64, 48);
        for version in [
            ColorScience::V3,
            ColorScience::V4,
            ColorScience::V5,
            ColorScience::V6,
            ColorScience::V7,
        ] {
            let settings = Settings {
                version,
                custom_grey: true,
                grey_point_source: 0.0,
                ..Settings::default()
            };
            let white = version < ColorScience::V6;
            assert_eq!(encodes_to_white(&settings), white, "{version:?}");
            let prepared = prepare(&settings, &input, geometry(), &profile(), &profile()).unwrap();
            let main = &prepared.passes.last().unwrap().params;
            // flags.z, after six vectors' worth of words: image, wavelet, mask, balance,
            // modes, then flags.x and flags.y.
            let flag = u32::from_le_bytes(main[88..92].try_into().unwrap());
            assert_eq!(flag, u32::from(white), "{version:?}");
            let grey = curve(&settings).grey_source;
            assert_eq!(grey, if white { 1.0 } else { GREY_SOURCE_MIN });
        }
        // A grey source off 0 is darktable's in v3 to v5, below the floor included.
        let small = Settings {
            version: ColorScience::V5,
            custom_grey: true,
            grey_point_source: 1e-4,
            ..Settings::default()
        };
        assert!(!encodes_to_white(&small));
        assert_eq!(curve(&small).grey_source, 1e-6);
    }

    #[test]
    fn spline_v1_and_v2_are_not_ported() {
        for spline_version in [SplineVersion::V1, SplineVersion::V2] {
            let settings = Settings {
                spline_version,
                ..Settings::default()
            };
            assert!(
                prepare(
                    &settings,
                    &ImageDesc::rgba(64, 48),
                    geometry(),
                    &profile(),
                    &profile()
                )
                .is_err()
            );
        }
    }

    #[test]
    fn darktables_scales_follow_the_image_and_the_pipe() {
        assert_eq!(scales(3908, 1.0), 8);
        assert_eq!(scales(5492, 1.0), 9);
        assert_eq!(scales(9504, 1.0), 9);
        assert_eq!(scales(3908, 0.5), 7);
        assert_eq!(scales(100, 1.0), 3);
        assert_eq!(scales(50, 1.0), 2);
        assert_eq!(scales(10, 1.0), 1);
        assert_eq!(scales(1, 1.0), 1);
        assert_eq!(scales(1 << 20, 1.0), 10);
    }

    #[test]
    fn the_raw_defaults_are_darktables() {
        let raw = Settings::default().for_raw(0.0);
        assert_eq!(raw.black_point_source, -7.65);
        assert_eq!(raw.white_point_source, 4.56);
        // As tests 0148 and 0153 store it.
        assert_eq!(raw.output_power, 3.614_816_2);
        let biased = Settings::default().for_raw(1.0);
        assert_eq!(biased.black_point_source, -8.0 - 0.15);
        // Outside ±5 EV darktable reads no bias.
        assert_eq!(Settings::default().for_raw(6.0), raw);
    }

    #[test]
    fn splitmix_and_xoshiro_are_darktables() {
        // splitmix32 of 1: (1 * 0x62a9d9ed799705f5), xored with itself >> 28, times
        // 0xcb24d0a5c88c35b3, the top 32 bits.
        let first = 0x62a9_d9ed_7997_05f5u64;
        let second = (first ^ (first >> 28)).wrapping_mul(0xcb24_d0a5_c88c_35b3);
        assert_eq!(splitmix32(1), (second >> 32) as u32);
        // A uniform draw is 24 bits.
        let mut state = noise_state(5, 7);
        for _ in 0..1000 {
            let u = xoshiro128plus(&mut state);
            assert!((0.0..1.0).contains(&u));
            assert_eq!(u * 16_777_216.0, (u * 16_777_216.0).floor());
        }
    }

    #[test]
    fn darktables_pow_is_close_to_pow() {
        for k in 1..=1000 {
            let x = k as f32 / 1000.0;
            for p in [1.0, 2.5, 4.0, 6.0] {
                let exact = Real::powf(x, p);
                let approx = pow_dt(x, p);
                assert!(
                    (approx - exact).abs() <= 2.5e-4 * exact.max(1e-30),
                    "{x}^{p}: {approx} against {exact}"
                );
            }
        }
        assert_eq!(pow_dt(0.0, 4.0), 0.0);
        // At an integer, roundf's half away from zero leaves a fraction of 0 above zero
        // and 1 at and below it.
        assert_eq!(exp2_dt(1.0), 2.0 * 1.0000026);
        assert!((exp2_dt(0.0) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn the_directions_are_the_curves_derivatives() {
        let settings = Settings {
            latitude: 40.0,
            balance: 10.0,
            saturation: 20.0,
            shadows: CurveType::Safe,
            ..Settings::default()
        };
        let directions = directions(&settings);
        // Along the saturation, a fraction, only v7's coefficient moves, one for one.
        let saturation = directions[3];
        assert!((saturation[4][2] - 1.0).abs() < 1e-8);
        for (v, vector) in saturation.iter().enumerate() {
            for (k, value) in vector.iter().enumerate() {
                if (v, k) != (4, 2) {
                    assert!(value.abs() < 1e-9, "{v} {k}: {value}");
                }
            }
        }
        // The hardness moves its own coefficient one for one.
        assert!((directions[1][4][1] - 1.0).abs() < 1e-8);
        // The contrast moves the line's slope.
        assert!(directions[0][2][3] > 0.0);
    }

    fn reconstruction() -> Settings {
        Settings {
            enable_highlight_reconstruction: true,
            ..Settings::default()
        }
    }

    #[test]
    fn without_reconstruction_main_runs_alone() {
        let input = ImageDesc::rgba(64, 48);
        let prepared = prepare(
            &Settings::default(),
            &input,
            geometry(),
            &profile(),
            &profile(),
        )
        .unwrap();
        assert_eq!(prepared.passes.len(), 1);
        assert_eq!(prepared.passes[0].entry_point, "main");
        assert_eq!(prepared.scratch, vec![Scratch { bytes: 16 }; 2]);
        assert_eq!(prepared.output, input);
    }

    #[test]
    fn the_reconstruction_has_four_passes_a_scale() {
        let input = ImageDesc::rgba(3908, 2602);
        let geometry = Geometry {
            size: 3908,
            scale: 1.0,
        };
        let prepared =
            prepare(&reconstruction(), &input, geometry, &profile(), &profile()).unwrap();
        let entry_points: Vec<&str> = prepared.passes.iter().map(|p| p.entry_point).collect();
        // 3 + (1 + 4 · 8) + (1 + 1 + 4 · 8 + 1) + 1.
        assert_eq!(entry_points.len(), 72);
        assert_eq!(
            entry_points[..4],
            ["hr_mask", "hr_count", "hr_inpaint", "hr_init"]
        );
        assert_eq!(
            entry_points[4..8],
            ["hr_blur_v", "hr_blur_h", "hr_blur_v", "hr_reconstruct"]
        );
        assert_eq!(entry_points[36], "hr_ratios");
        assert_eq!(entry_points[37], "hr_init");
        assert_eq!(entry_points[70], "hr_restore");
        assert_eq!(entry_points[71], "main");
        assert_eq!(prepared.passes[1].size, Some([16, 16]));
        assert_eq!(prepared.scratch.len(), 7);
        let total: u64 = prepared.scratch.iter().map(|s| s.bytes).sum();
        assert_eq!(total, 3908 * 2602 * 72 + (1 + 245 * 163) * 4);
        // No iteration, no norms.
        let none = Settings {
            high_quality_reconstruction: 0,
            ..reconstruction()
        };
        let prepared = prepare(&none, &input, geometry, &profile(), &profile()).unwrap();
        assert_eq!(prepared.passes.len(), 3 + 33 + 1);
        assert_eq!(prepared.scratch.len(), 6);
    }

    #[test]
    fn the_scales_swap_their_buffers() {
        let prepared = prepare(
            &reconstruction(),
            &ImageDesc::rgba(640, 480),
            geometry(),
            &profile(),
            &profile(),
        )
        .unwrap();
        let bound = |pass: &Pass, binding: u32| {
            pass.bindings
                .iter()
                .find(|b| b.binding == binding)
                .map(|b| b.resource)
        };
        let blurs: Vec<&Pass> = prepared
            .passes
            .iter()
            .filter(|p| p.entry_point == "hr_blur_h")
            .collect();
        assert_eq!(bound(blurs[0], 12), Some(Resource::Scratch(BUFFER_A)));
        assert_eq!(bound(blurs[0], 13), Some(Resource::Scratch(BUFFER_B)));
        assert_eq!(bound(blurs[1], 12), Some(Resource::Scratch(BUFFER_B)));
        assert_eq!(bound(blurs[1], 13), Some(Resource::Scratch(BUFFER_A)));
    }

    #[test]
    fn every_pass_binds_within_eight_storage_buffers() {
        let input = ImageDesc::rgba(640, 480);
        for settings in [Settings::default(), reconstruction()] {
            let prepared = prepare(&settings, &input, geometry(), &profile(), &profile()).unwrap();
            for pass in &prepared.passes {
                assert_eq!(pass.params.len(), PARAMS_SIZE);
                let storage = pass
                    .bindings
                    .iter()
                    .filter(|b| b.resource != Resource::Uniforms)
                    .count();
                assert!(storage <= 8, "{}", pass.entry_point);
                for binding in &pass.bindings {
                    if let Resource::Scratch(index) = binding.resource {
                        assert!(index < prepared.scratch.len(), "{}", pass.entry_point);
                    }
                }
                let writes_output = pass.bindings.iter().any(|b| b.resource == Resource::Output);
                assert_eq!(writes_output, pass.entry_point == "main");
                assert_eq!(
                    pass.kind == PassKind::PerPixel,
                    pass.entry_point == "main",
                    "{}",
                    pass.entry_point
                );
            }
            let main = prepared.passes.last().unwrap();
            let bound: Vec<u32> = main.bindings.iter().map(|b| b.binding).collect();
            assert_eq!(bound, [0, 1, 2, 11, 15]);
        }
    }

    #[test]
    fn the_curve_reaches_only_main_and_the_reconstruction_only_its_passes() {
        let input = ImageDesc::rgba(320, 200);
        let at = |settings: &Settings| {
            prepare(settings, &input, geometry(), &profile(), &profile()).unwrap()
        };
        let base = at(&reconstruction());
        let look = at(&Settings {
            contrast: 1.4,
            saturation: 20.0,
            latitude: 30.0,
            ..reconstruction()
        });
        let threshold = at(&Settings {
            reconstruct_threshold: -1.0,
            reconstruct_bloom_vs_details: 20.0,
            noise_level: 1.0,
            ..reconstruction()
        });
        let split = |p: &Prepared| {
            (
                p.passes[..p.passes.len() - 1]
                    .iter()
                    .map(|pass| pass.params.clone())
                    .collect::<Vec<_>>(),
                p.passes.last().unwrap().params.clone(),
            )
        };
        let (base_head, base_main) = split(&base);
        let (look_head, look_main) = split(&look);
        let (threshold_head, threshold_main) = split(&threshold);
        assert_eq!(base_head, look_head);
        assert_ne!(base_main, look_main);
        assert_ne!(base_head, threshold_head);
        assert_eq!(base_main, threshold_main);
    }

    #[test]
    fn the_uniforms_are_laid_out_as_the_kernel_reads_them() {
        let uniforms = Uniforms {
            image: [1, 2, 3, 4],
            mask: [0.5, 0.25, 0.0, 0.0],
            luminance: [0.1, 0.2, 0.3],
            curve: [[7.0; 4]; 7],
            ..Uniforms::default()
        };
        let bytes = uniforms.to_bytes();
        assert_eq!(bytes.len(), PARAMS_SIZE);
        let word =
            |offset: usize| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        let float = |offset: usize| f32::from_bits(word(offset));
        assert_eq!(word(12), 4);
        assert_eq!(float(32), 0.5);
        assert_eq!(float(96), 0.1);
        assert_eq!(float(108), 0.0);
        // The curve after six vectors, the luminance and four matrices.
        assert_eq!(float(16 * 19), 7.0);
    }
}

#[cfg(test)]
mod pieces {
    use super::*;

    /// Test 0014's settings as darktable migrated them (test 0139's first entry): a hard
    /// toe and a soft shoulder.
    fn test_0014() -> Settings {
        Settings {
            grey_point_source: 9.225,
            black_point_source: -10.55,
            white_point_source: 3.45,
            output_power: 5.98,
            latitude: 59.693874,
            contrast: 1.0872912,
            saturation: 5.0,
            balance: 29.234432,
            black_point_target: 0.0,
            shadows: CurveType::Hard,
            highlights: CurveType::Soft,
            custom_grey: true,
            version: ColorScience::V3,
            ..Settings::default()
        }
    }

    #[test]
    fn the_node_centred_pieces_are_darktables_polynomials() {
        for (shadows, highlights) in [
            (CurveType::Hard, CurveType::Soft),
            (CurveType::Soft, CurveType::Hard),
            (CurveType::Hard, CurveType::Hard),
        ] {
            let settings = Settings {
                shadows,
                highlights,
                ..test_0014()
            };
            let spline = spline_of(&Floats::<f64>::of(&settings), &settings);
            for (part, node) in [(0, spline.x[1]), (1, spline.x[3])] {
                let (m, a) = (spline.m[part], spline.taylor[part]);
                for k in 0..=100 {
                    let x = f64::from(k) / 100.0;
                    let d = x - node;
                    let monomial = m[0] + x * (m[1] + x * (m[2] + x * (m[3] + x * m[4])));
                    let taylor = a[0] + d * (a[1] + d * (a[2] + d * (a[3] + d * a[4])));
                    assert!(
                        (monomial - taylor).abs() < 1e-9,
                        "{shadows:?}/{highlights:?} piece {part} at {x}: {monomial} {taylor}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_directions_follow_darktables_f32_setup() {
        // Central differences of the f32 setup, darktable's arithmetic, at a step of
        // 1e-3 in each direction's units, against the f64 derivatives.
        let settings = test_0014();
        let directions = directions(&settings);
        for (direction, expected) in DIRECTIONS.iter().zip(&directions) {
            let h = 1e-3 * direction.scale();
            let moved = |sign: f32| {
                let mut s = settings;
                match direction {
                    Direction::Contrast => s.contrast += sign * h,
                    Direction::OutputPower => s.output_power += sign * h,
                    Direction::Latitude => s.latitude += sign * h,
                    Direction::Saturation => s.saturation += sign * h,
                }
                curve(&s).vectors()
            };
            let (a, b) = (moved(1.0), moved(-1.0));
            let largest = expected
                .iter()
                .flatten()
                .fold(0.0f64, |m, v| m.max(v.abs()));
            for v in 0..7 {
                for k in 0..4 {
                    let numeric = f64::from(a[v][k] - b[v][k]) / f64::from(2e-3_f32);
                    assert!(
                        (numeric - expected[v][k]).abs() <= 1e-2 * largest,
                        "{direction:?} {v} {k}: {numeric} against {}",
                        expected[v][k]
                    );
                }
            }
        }
    }
}
