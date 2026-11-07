// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/common/colorspaces.c,
// colorspaces_inline_conversions.h, chromatic_adaptation.h, custom_primaries.c,
// iop_profile.h, rgb_norms.h and src/iop/colorbalancergb.c)
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

//! Colour maths on the CPU, for kernel setup: the values a module computes once before
//! its kernel runs. Each function mirrors the darktable helper named in its doc, in f32
//! like darktable, so setup values match darktable's own.
//!
//! Matrices are row-major: `apply(m, v)` computes m · v.
//!
//! Beside colour maths it holds darktable's numerical helpers that several operations
//! share: the curve interpolation (`curve`), the box mean (`box_filter`, the Rust twin
//! of `slang/box_filter.slang`), the recursive Gaussian (`gaussian`, of
//! `slang/gaussian.slang`), the bilinear resampling between grids (`bilinear`, of
//! `slang/bilinear.slang`), and the tone equalizer's correction curve (`tone_equalizer`),
//! which its operation's setup and its module's band graph share.

#![allow(
    clippy::excessive_precision,
    reason = "constants are copied digit for digit from darktable"
)]

use std::f32::consts::PI;

pub mod adaptation;
pub mod bilinear;
pub mod box_filter;
pub mod curve;
mod dng;
pub mod gaussian;
pub mod illuminants;
pub mod tone_equalizer;
pub mod ucs;
pub mod white_balance;

pub use adaptation::Adaptation;
pub use dng::{DngCalibration, DngLightSource, DngMatrix, dng_light_source, dng_xyz_to_camera};
pub use illuminants::Illuminant;

pub type Mat3 = [[f32; 3]; 3];

pub fn apply(m: &Mat3, v: [f32; 3]) -> [f32; 3] {
    m.map(|row| row[0] * v[0] + row[1] * v[1] + row[2] * v[2])
}

/// darktable `dt_colormatrix_mul`: a · b.
pub fn mul(a: &Mat3, b: &Mat3) -> Mat3 {
    let mut out = [[0.0; 3]; 3];
    for (k, row) in out.iter_mut().enumerate() {
        for (i, value) in row.iter_mut().enumerate() {
            for j in 0..3 {
                *value += a[k][j] * b[j][i];
            }
        }
    }
    out
}

/// darktable `deg2radf`.
pub fn deg_to_rad(degrees: f32) -> f32 {
    degrees * PI / 180.0
}

/// CAT16 chromatic adaptation between D50 and D65 (`chromatic_adaptation.h`).
pub const XYZ_D50_TO_D65_CAT16: Mat3 = [
    [9.89466254e-01, -4.00304626e-02, 4.40530317e-02],
    [-5.40518733e-03, 1.00666069e+00, -1.75551955e-03],
    [-4.03920992e-04, 1.50768030e-02, 1.30210211e+00],
];
pub const XYZ_D65_TO_D50_CAT16: Mat3 = [
    [1.01085433e+00, 4.07086103e-02, -3.41445825e-02],
    [5.42814201e-03, 9.93581926e-01, 1.15592039e-03],
    [2.50722468e-04, -1.14918759e-02, 7.67964947e-01],
];

/// CIE 1931 XYZ D65 to CIE 2006 LMS, Kirk's approximation.
pub const XYZ_D65_TO_LMS_2006: Mat3 = [
    [0.257085, 0.859943, -0.031061],
    [-0.394427, 1.175800, 0.106423],
    [0.064856, -0.076250, 0.559067],
];

/// CIE 2006 LMS back to CIE 1931 XYZ D65, darktable's `LMS_2006_D65_to_XYZ_D65`, the
/// twin of `LMS_2006_TO_XYZ_D65` in `color.slang`.
pub const LMS_2006_TO_XYZ_D65: Mat3 = [
    [1.80794659, -1.29971660, 0.34785879],
    [0.61783960, 0.39595453, -0.04104687],
    [-0.12546960, 0.20478038, 1.74274183],
];

/// Filmlight grading RGB to and from CIE 2006 LMS D65.
pub const GRADING_RGB_TO_LMS: Mat3 = [[0.95, 0.38, 0.0], [0.05, 0.62, 0.03], [0.0, 0.0, 0.97]];
pub const LMS_TO_GRADING_RGB: Mat3 = [
    [1.0877193, -0.66666667, 0.02061856],
    [-0.0877193, 1.66666667, -0.05154639],
    [0.0, 0.0, 1.03092784],
];

/// A working space as darktable's pipeline sees it: RGB to XYZ D50 and back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkingProfile {
    pub rgb_to_xyz_d50: Mat3,
    pub xyz_d50_to_rgb: Mat3,
}

impl WorkingProfile {
    /// darktable's default working space, linear Rec2020, with the matrices lcms derives
    /// for it (`colorspaces.c`): Rec2020 primaries and white (0.31271, 0.32902),
    /// Bradford-adapted to the ICC D50 white.
    pub fn linear_rec2020() -> Self {
        let rgb_to_xyz = to_f32(lcms_rgb_to_xyz_d50([0.31271, 0.32902], REC2020_PRIMARIES));
        Self {
            rgb_to_xyz_d50: rgb_to_xyz,
            xyz_d50_to_rgb: to_f32(invert(to_f64(rgb_to_xyz))),
        }
    }

    /// darktable `dt_ioppr_uncompensate_middle_grey`: a lightness `x`, Lab L / 100, as
    /// the linear working-space value of the grey that has it. Lab (100 x, 0, 0) to XYZ
    /// D50, then the red channel of working RGB, which for a grey is the luminance. Middle
    /// grey, 0.5, gives about 0.1842.
    pub fn uncompensate_middle_grey(&self, x: f32) -> f32 {
        let xyz = lab_to_xyz_d50([100.0 * x, 0.0, 0.0]);
        let row = self.xyz_d50_to_rgb[0];
        row[0] * xyz[0] + row[1] * xyz[1] + row[2] * xyz[2]
    }

    /// The inverse of [`Self::uncompensate_middle_grey`]: the lightness, Lab L / 100, of
    /// the grey whose linear working-space value is `v`. A curve drawn on lightness axes
    /// reads a compensated curve's output back with it.
    pub fn compensate_middle_grey(&self, v: f32) -> f32 {
        let white = self.uncompensate_middle_grey(1.0);
        xyz_to_lab_d50([0.0, v / white, 0.0])[0] / 100.0
    }
}

/// darktable `dt_iop_rgb_norms_t` (`src/common/rgb_norms.h`): the norm a tone curve
/// preserves colours by, applying its curve to the norm and scaling every channel alike.
/// RGB curve and RGB levels share it; the kernels' twin is `rgb_norm` in `color.slang`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RgbNorm {
    /// No norm: the curve on each channel.
    None,
    /// The working space's luminance.
    Luminance,
    Max,
    Average,
    Sum,
    /// √(r² + g² + b²).
    Norm,
    /// "Basic power": (r³ + g³ + b³) / (r² + g² + b²).
    Power,
}

impl RgbNorm {
    pub const ALL: [Self; 7] = [
        Self::None,
        Self::Luminance,
        Self::Max,
        Self::Average,
        Self::Sum,
        Self::Norm,
        Self::Power,
    ];

    pub fn from_darktable(value: i32) -> Option<Self> {
        usize::try_from(value)
            .ok()
            .and_then(|i| Self::ALL.get(i).copied())
    }

    pub fn to_darktable(self) -> i32 {
        Self::ALL.iter().position(|&n| n == self).unwrap() as i32
    }

    /// darktable `dt_rgb_norm`: this norm of a pixel, `luminance` being the Y row of the
    /// working space's RGB to XYZ D50 matrix. `None`, like any norm darktable does not
    /// know, is the average. The basic power of black is 0, where darktable's divides 0
    /// by 0.
    pub fn of(self, rgb: [f32; 3], luminance: [f32; 3]) -> f32 {
        let [r, g, b] = rgb;
        match self {
            Self::Luminance => luminance[0] * r + luminance[1] * g + luminance[2] * b,
            Self::Max => r.max(g).max(b),
            Self::Sum => r + g + b,
            Self::Norm => (r * r + g * g + b * b).sqrt(),
            Self::Power => {
                let squares = r * r + g * g + b * b;
                if squares > 0.0 {
                    (r * r * r + g * g * g + b * b * b) / squares
                } else {
                    0.0
                }
            }
            Self::None | Self::Average => (r + g + b) / 3.0,
        }
    }
}

/// darktable `dt_Lab_to_XYZ`, to XYZ with the D50 white.
pub fn lab_to_xyz_d50(lab: [f32; 3]) -> [f32; 3] {
    const D50: [f32; 3] = [0.9642, 1.0, 0.8249];
    let fy = (lab[0] + 16.0) * (1.0 / 116.0);
    let fx = lab[1] * (1.0 / 500.0) + fy;
    let fz = lab[2] * (-1.0 / 200.0) + fy;
    [
        D50[0] * lab_f_inv(fx),
        D50[1] * lab_f_inv(fy),
        D50[2] * lab_f_inv(fz),
    ]
}

/// darktable `lab_f_inv`.
fn lab_f_inv(x: f32) -> f32 {
    // cbrt(216 / 24389)
    const EPSILON: f32 = 0.20689655172413796;
    const KAPPA: f32 = 24389.0 / 27.0;
    if x > EPSILON {
        x * x * x
    } else {
        (116.0 * x - 16.0) / KAPPA
    }
}

/// darktable `dt_XYZ_to_Lab`, from XYZ with the D50 white; the kernels' twin is
/// `xyz_to_lab_d50` in `color.slang`.
pub fn xyz_to_lab_d50(xyz: [f32; 3]) -> [f32; 3] {
    const D50: [f32; 3] = [0.9642, 1.0, 0.8249];
    let [fx, fy, fz] = [0, 1, 2].map(|c| lab_f(xyz[c] / D50[c]));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), -200.0 * (fz - fy)]
}

/// darktable `lab_f`.
fn lab_f(x: f32) -> f32 {
    const EPSILON: f32 = 216.0 / 24389.0;
    const KAPPA: f32 = 24389.0 / 27.0;
    if x > EPSILON {
        x.cbrt()
    } else {
        (KAPPA * x + 16.0) / 116.0
    }
}

/// Chromaticities of RGB spaces' primaries, red, green, blue, and of D65.
pub const REC2020_PRIMARIES: [[f64; 2]; 3] = [[0.708, 0.292], [0.170, 0.797], [0.131, 0.046]];
pub const SRGB_PRIMARIES: [[f64; 2]; 3] = [[0.64, 0.33], [0.30, 0.60], [0.15, 0.06]];
pub const DISPLAY_P3_PRIMARIES: [[f64; 2]; 3] = [[0.680, 0.320], [0.265, 0.690], [0.150, 0.060]];
pub const D65: [f64; 2] = [0.3127, 0.3290];

/// RGB to XYZ for an RGB space's primaries and white, without chromatic adaptation.
pub fn rgb_to_xyz(white: [f64; 2], primaries: [[f64; 2]; 3]) -> Mat3 {
    to_f32(rgb_to_xyz_d(white, primaries))
}

pub fn inverse(m: &Mat3) -> Mat3 {
    to_f32(invert(to_f64(*m)))
}

pub const ADOBE_RGB_PRIMARIES: [[f64; 2]; 3] = [[0.64, 0.33], [0.21, 0.71], [0.15, 0.06]];
/// The D65 white darktable gives lcms for its built-in RGB profiles (`D65xyY` in
/// `colorspaces.h`).
pub const PROFILE_D65: [f64; 2] = [0.31271, 0.32902];
/// The ICC D50 white, lcms `cmsD50_XYZ`.
pub const D50_XYZ: [f64; 3] = [0.9642, 1.0, 0.8249];

pub type Mat3d = [[f64; 3]; 3];

pub const IDENTITY_D: Mat3d = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// An RGB space as darktable builds its profile in lcms (`colorspaces.c`), from the
/// chromaticities of its primaries and white. Only its matrices and chromaticities
/// matter here, never its transfer curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RgbSpace {
    pub primaries: [[f64; 2]; 3],
    pub white: [f64; 2],
}

/// The chromaticities darktable reads back from a profile
/// (`dt_colorspaces_get_primaries_and_whitepoint_from_profile`): the xy of its colorant
/// tags, which lcms Bradford-adapts to D50, and of its media white, which lcms writes as
/// D50.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chromaticities {
    pub primaries: [[f64; 2]; 3],
    pub white: [f64; 2],
}

impl RgbSpace {
    pub const LINEAR_REC2020: Self = Self {
        primaries: REC2020_PRIMARIES,
        white: PROFILE_D65,
    };
    pub const DISPLAY_P3: Self = Self {
        primaries: DISPLAY_P3_PRIMARIES,
        white: PROFILE_D65,
    };
    pub const ADOBE_RGB: Self = Self {
        primaries: ADOBE_RGB_PRIMARIES,
        white: PROFILE_D65,
    };
    pub const SRGB: Self = Self {
        primaries: SRGB_PRIMARIES,
        white: PROFILE_D65,
    };

    /// RGB to XYZ D50: the profile's colorant matrix.
    pub fn rgb_to_xyz_d50(&self) -> Mat3d {
        lcms_rgb_to_xyz_d50(self.white, self.primaries)
    }

    /// Bradford from the space's white to D50: the profile's chromatic adaptation tag,
    /// `chad`, as lcms writes it.
    pub fn adaptation_to_d50(&self) -> Mat3d {
        lcms_adaptation_to_d50(self.white)
    }

    pub fn chromaticities(&self) -> Chromaticities {
        let m = self.rgb_to_xyz_d50();
        Chromaticities {
            primaries: std::array::from_fn(|c| d50_xyz_to_xy([m[0][c], m[1][c], m[2][c]])),
            white: d50_xyz_to_xy(D50_XYZ),
        }
    }
}

/// darktable `dt_D50_XYZ_to_xyY`, chromaticity only.
fn d50_xyz_to_xy(xyz: [f64; 3]) -> [f64; 2] {
    let xyz = xyz.map(|v| v.max(0.0));
    let sum = xyz[0] + xyz[1] + xyz[2];
    if sum > 0.0 {
        [xyz[0] / sum, xyz[1] / sum]
    } else {
        d50_xyz_to_xy(D50_XYZ)
    }
}

/// darktable `dt_rotate_and_scale_primary` (`custom_primaries.c`): primary `index`
/// rotated about the white by `rotation` radians, then moved towards the white to
/// `scaling` times its distance from the white to the gamut's edge in that direction.
pub fn rotate_and_scale_primary(
    space: &Chromaticities,
    scaling: f64,
    rotation: f64,
    index: usize,
) -> [f64; 2] {
    let [wx, wy] = space.white;
    let [px, py] = space.primaries[index];
    let angle = (py - wy).atan2(px - wx) + rotation;
    let (sin, cos) = angle.sin_cos();

    // darktable `_find_distance_to_edge`: the nearest intersection, in the direction of
    // the ray, with the lines through the triangle's edges.
    let determinant = |a: f64, b: f64, c: f64, d: f64| a * d - b * c;
    let (x1, y1, x2, y2) = (wx, wy, wx + cos, wy + sin);
    let distance = (0..3)
        .map(|i| {
            let [x3, y3] = space.primaries[i];
            let [x4, y4] = space.primaries[(i + 1) % 3];
            let denominator = determinant(x1 - x2, x3 - x4, y1 - y2, y3 - y4);
            if denominator == 0.0 {
                return f64::MAX;
            }
            let t = determinant(x1 - x3, x3 - x4, y1 - y3, y3 - y4) / denominator;
            if t >= 0.0 { t } else { f64::MAX }
        })
        .fold(f64::MAX, f64::min);
    [wx + scaling * distance * cos, wy + scaling * distance * sin]
}

/// darktable `dt_make_transposed_matrices_from_primaries_and_whitepoint`, as RGB to XYZ
/// (not transposed): the matrix whose columns are the primaries, scaled so they sum to
/// the white. Primaries on one point or one line give the zero matrix, as darktable's
/// `mat3SSEinv` refuses a determinant under 1e-7 in magnitude and leaves the inverse 0
/// ([`checked_inverse_d`]).
pub fn rgb_to_xyz_from_chromaticities(primaries: [[f64; 2]; 3], white: [f64; 2]) -> Mat3d {
    // darktable `_sanitizeY`: keep y at least FLT_EPSILON from zero.
    let sanitize = |y: f64| {
        let epsilon = f64::from(f32::EPSILON);
        if (0.0..epsilon).contains(&y) {
            epsilon
        } else if y < 0.0 && y > -epsilon {
            -epsilon
        } else {
            y
        }
    };
    let column = |[x, y]: [f64; 2]| {
        let y = sanitize(y);
        [x / y, 1.0, (1.0 - x - y) / y]
    };
    let columns = primaries.map(column);
    let matrix: Mat3d = std::array::from_fn(|r| std::array::from_fn(|c| columns[c][r]));
    let Some(inverse) = checked_inverse_d(&matrix) else {
        return [[0.0; 3]; 3];
    };
    let scale = apply_d(&inverse, column(white));
    matrix.map(|row| std::array::from_fn(|c| row[c] * scale[c]))
}

/// a · b.
pub fn mul_d(a: &Mat3d, b: &Mat3d) -> Mat3d {
    std::array::from_fn(|k| std::array::from_fn(|i| (0..3).map(|j| a[k][j] * b[j][i]).sum()))
}

pub fn inverse_d(m: &Mat3d) -> Mat3d {
    invert(*m)
}

/// darktable `mat3SSEinv`: the inverse, or `None` where darktable refuses a determinant
/// under 1e-7 in magnitude and leaves its destination as it was. darktable takes the
/// determinant in f32, where a singular matrix's is rounding; in f64 it is far under the
/// threshold there, so this refuses what `mat3SSEinv` is written to refuse.
pub fn checked_inverse_d(m: &Mat3d) -> Option<Mat3d> {
    if determinant(m).abs() < 1e-7 {
        None
    } else {
        Some(invert(*m))
    }
}

pub fn to_f32(m: Mat3d) -> Mat3 {
    m.map(|row| row.map(|v| v as f32))
}

/// lcms `_cmsBuildRGB2XYZtransferMatrix`.
fn rgb_to_xyz_d(white: [f64; 2], primaries: [[f64; 2]; 3]) -> Mat3d {
    let [[xr, yr], [xg, yg], [xb, yb]] = primaries;
    let chromaticities = [
        [xr, xg, xb],
        [yr, yg, yb],
        [1.0 - xr - yr, 1.0 - xg - yg, 1.0 - xb - yb],
    ];
    let [xn, yn] = white;
    let white_xyz = [xn / yn, 1.0, (1.0 - xn - yn) / yn];
    let coef = apply_d(&invert(chromaticities), white_xyz);
    chromaticities.map(|row| [row[0] * coef[0], row[1] * coef[1], row[2] * coef[2]])
}

/// darktable's colour profile types, `dt_colorspaces_color_profile_type_t`, in the order
/// of their values from 0 (`file`) on.
pub const PROFILE_TYPES: &[&str] = &[
    "file",
    "srgb",
    "adobe_rgb",
    "lin_rec709",
    "lin_rec2020",
    "xyz",
    "lab",
    "infrared",
    "display",
    "embedded_icc",
    "embedded_matrix",
    "standard_matrix",
    "enhanced_matrix",
    "vendor_matrix",
    "alternate_matrix",
    "brg",
    "export",
    "softproof",
    "work",
    "display2",
    "rec709",
    "prophoto_rgb",
    "pq_rec2020",
    "hlg_rec2020",
    "pq_p3",
    "hlg_p3",
    "display_p3",
];

/// darktable's D65 for camera profiles, `d65` in `colorspaces.c`, as XYZ.
const CAMERA_D65_XYZ: [f64; 3] = [0.95045471, 1.00000000, 1.08905029];

/// darktable `dt_colorspaces_create_xyzimatrix_profile`, read back as colorin reads it
/// (`dt_colorspaces_get_matrix_from_input_profile`): camera RGB to XYZ D50 for a camera
/// whose XYZ to camera matrix, under D65, is `xyz_to_camera`. darktable keeps only the
/// chromaticities of the inverse's columns and lets lcms scale them so camera white is
/// its D65, then adapts to D50.
pub fn camera_to_xyz_d50(xyz_to_camera: &Mat3) -> Mat3d {
    let inverse = to_f32(invert(to_f64(*xyz_to_camera)));
    let primaries = [0, 1, 2].map(|k| {
        let norm = inverse[0][k] + inverse[1][k] + inverse[2][k];
        [
            f64::from(inverse[0][k] / norm),
            f64::from(inverse[1][k] / norm),
        ]
    });
    let [x, y, z] = CAMERA_D65_XYZ;
    let sum = x + y + z;
    lcms_rgb_to_xyz_d50([x / sum, y / sum], primaries)
}

/// An RGB space's RGB to XYZ D50, as lcms builds it for darktable's profiles
/// (`_create_lcms_profile`): primaries and white chromaticities, Bradford-adapted.
pub fn profile_rgb_to_xyz_d50(white: [f64; 2], primaries: [[f64; 2]; 3]) -> Mat3d {
    lcms_rgb_to_xyz_d50(white, primaries)
}

/// darktable `_calculate_bogus_daylight_wb` through
/// `dt_colorspaces_conversion_matrices_rgb`: the multipliers that make sRGB white, seen
/// through the camera's matrix, equal in every channel: one over the row sums of the
/// camera's matrix times darktable's sRGB, normalised so green is 1.
pub fn daylight_multipliers(xyz_to_camera: &Mat3) -> [f64; 3] {
    // darktable's sRGB to XYZ (D65), as it uses here.
    const RGB_TO_XYZ: Mat3d = [
        [0.412453, 0.357580, 0.180423],
        [0.212671, 0.715160, 0.072169],
        [0.019334, 0.119193, 0.950227],
    ];
    let mul = xyz_to_camera.map(|row| {
        let sum: f64 = (0..3)
            .map(|j| {
                (0..3)
                    .map(|k| f64::from(row[k]) * RGB_TO_XYZ[k][j])
                    .sum::<f64>()
            })
            .sum();
        1.0 / sum
    });
    mul.map(|m| m / mul[1])
}

/// darktable `_find_coeffs`, normalised as `reload_defaults` normalises it: the as-shot
/// multipliers when the camera recorded three normal ones, else daylight's from the
/// camera's matrix, else darktable's last resort, 2, 1, 1.5; green 1. The fourth is the
/// fourth recorded, or green's. darktable also looks up its camera presets before the
/// last resort; Apertrust has no such table.
pub fn as_shot_multipliers(
    white_balance: Option<&[f32]>,
    xyz_to_camera: Option<&Mat3>,
) -> [f64; 4] {
    let recorded = white_balance
        .filter(|wb| wb.len() >= 3 && wb[..3].iter().all(|v| v.is_normal() && *v > 0.0));
    let raw: [f64; 4] = match (recorded, xyz_to_camera) {
        (Some(wb), _) => [0, 1, 2, 3].map(|i| f64::from(wb.get(i).copied().unwrap_or(wb[1]))),
        (None, Some(matrix)) => {
            let [r, g, b] = daylight_multipliers(matrix);
            [r, g, b, g]
        }
        (None, None) => [2.0, 1.0, 1.5, 1.0],
    };
    raw.map(|v| v / raw[1])
}

/// darktable's late correction, `chr->D65coeffs / chr->as_shot`: the camera's daylight
/// multipliers over its as-shot ones. Temperature's preset "as shot to reference" leaves
/// it to colorin, and highlight reconstruction's inpaint opposed weighs its averages by
/// it meanwhile.
pub fn late_correction(white_balance: Option<&[f32]>, xyz_to_camera: &Mat3) -> [f32; 3] {
    let daylight = daylight_multipliers(xyz_to_camera);
    let as_shot = as_shot_multipliers(white_balance, Some(xyz_to_camera));
    [0, 1, 2].map(|c| (daylight[c] / as_shot[c]) as f32)
}

/// lcms `_cmsBuildRGB2XYZtransferMatrix` followed by `_cmsAdaptMatrixToD50`.
fn lcms_rgb_to_xyz_d50(white: [f64; 2], primaries: [[f64; 2]; 3]) -> Mat3d {
    mul_d(
        &lcms_adaptation_to_d50(white),
        &rgb_to_xyz_d(white, primaries),
    )
}

/// lcms `_cmsAdaptationMatrix` with no cone matrix given, so Bradford's, from `white` to
/// D50.
fn lcms_adaptation_to_d50(white: [f64; 2]) -> Mat3d {
    let [xn, yn] = white;
    let white_xyz = [xn / yn, 1.0, (1.0 - xn - yn) / yn];

    // lcms `ComputeChromaticAdaptation`.
    const BRADFORD: Mat3d = [
        [0.8951, 0.2664, -0.1614],
        [-0.7502, 1.7135, 0.0367],
        [0.0389, -0.0685, 1.0296],
    ];
    const D50: [f64; 3] = [0.9642, 1.0, 0.8249];
    let source = apply_d(&BRADFORD, white_xyz);
    let target = apply_d(&BRADFORD, D50);
    let gains = [
        [target[0] / source[0], 0.0, 0.0],
        [0.0, target[1] / source[1], 0.0],
        [0.0, 0.0, target[2] / source[2]],
    ];
    mul_d(&invert(BRADFORD), &mul_d(&gains, &BRADFORD))
}

fn apply_d(m: &Mat3d, v: [f64; 3]) -> [f64; 3] {
    m.map(|row| row[0] * v[0] + row[1] * v[1] + row[2] * v[2])
}

fn invert(m: Mat3d) -> Mat3d {
    let det = determinant(&m);
    std::array::from_fn(|r| std::array::from_fn(|c| cofactor(&m, c, r) / det))
}

fn cofactor(m: &Mat3d, r: usize, c: usize) -> f64 {
    let (r1, r2) = ((r + 1) % 3, (r + 2) % 3);
    let (c1, c2) = ((c + 1) % 3, (c + 2) % 3);
    m[r1][c1] * m[r2][c2] - m[r1][c2] * m[r2][c1]
}

fn determinant(m: &Mat3d) -> f64 {
    m[0][0] * cofactor(m, 0, 0) + m[0][1] * cofactor(m, 0, 1) + m[0][2] * cofactor(m, 0, 2)
}

fn to_f64(m: Mat3) -> Mat3d {
    m.map(|row| row.map(f64::from))
}

/// White of grading-RGB (r, g) chromaticity: sRGB (1, 1, 1) through D50 XYZ, CAT16 to
/// D65 and LMS 2006.
const YRG_WHITE: [f32; 2] = [0.21902143, 0.54371398];

/// darktable `LMS_to_Yrg`.
pub fn lms_to_yrg(lms: [f32; 3]) -> [f32; 3] {
    let y = 0.68990272 * lms[0] + 0.34832189 * lms[1];
    let a = lms[0] + lms[1] + lms[2];
    let normalized = lms.map(|v| if a == 0.0 { 0.0 } else { v / a });
    let rgb = apply(&LMS_TO_GRADING_RGB, normalized);
    [y, rgb[0], rgb[1]]
}

/// darktable `Yrg_to_LMS`.
pub fn yrg_to_lms(yrg: [f32; 3]) -> [f32; 3] {
    let [y, r, g] = yrg;
    let lms = apply(&GRADING_RGB_TO_LMS, [r, g, 1.0 - r - g]);
    let denominator = 0.68990272 * lms[0] + 0.34832189 * lms[1];
    let a = if denominator == 0.0 {
        0.0
    } else {
        y / denominator
    };
    lms.map(|v| v * a)
}

/// darktable `make_Ych`: luminance, chroma, and the hue as its cosine and sine.
pub fn make_ych(y: f32, c: f32, hue: f32) -> [f32; 4] {
    [y, c, hue.cos(), hue.sin()]
}

/// darktable `Ych_to_Yrg`.
pub fn ych_to_yrg(ych: [f32; 4]) -> [f32; 3] {
    let [y, c, cos_h, sin_h] = ych;
    [y, c * cos_h + YRG_WHITE[0], c * sin_h + YRG_WHITE[1]]
}

/// darktable `Ych_to_gradingRGB`.
pub fn ych_to_grading_rgb(ych: [f32; 4]) -> [f32; 3] {
    apply(&LMS_TO_GRADING_RGB, yrg_to_lms(ych_to_yrg(ych)))
}

/// darktable `dt_XYZ_2_JzAzBz`, from XYZ D65.
pub fn xyz_to_jzazbz(xyz: [f32; 3]) -> [f32; 3] {
    const B: f32 = 1.15;
    const G: f32 = 0.66;
    const C1: f32 = 0.8359375;
    const C2: f32 = 18.8515625;
    const C3: f32 = 18.6875;
    const N: f32 = 0.159301758;
    const P: f32 = 134.034375;
    const D: f32 = -0.56;
    const D0: f32 = 1.6295499532821566e-11;
    const XYZ_TO_LMS: Mat3 = [
        [0.41478972, 0.57999900, 0.01464800],
        [-0.2015100, 1.1206490, 0.0531008],
        [-0.0166008, 0.2648000, 0.6684799],
    ];
    const LMS_TO_IAB: Mat3 = [
        [0.5, 0.5, 0.0],
        [3.524000, -4.066708, 0.542708],
        [0.199076, 1.096799, -1.295875],
    ];

    let xyz_prime = [
        B * xyz[0] - (B - 1.0) * xyz[2],
        G * xyz[1] - (G - 1.0) * xyz[0],
        xyz[2],
    ];
    let lms = apply(&XYZ_TO_LMS, xyz_prime).map(|v| {
        let v = (v / 10000.0).max(0.0).powf(N);
        ((C1 + C2 * v) / (1.0 + C3 * v)).powf(P)
    });
    let [iz, az, bz] = apply(&LMS_TO_IAB, lms);
    [(((1.0 + D) * iz) / (1.0 + D * iz) - D0).max(0.0), az, bz]
}

/// Entries in a hue-indexed gamut LUT, over hue angles [-π, π).
pub const GAMUT_LUT_SIZE: usize = 512;

/// The largest JzAzBz saturation (Cz / Jz) the working gamut reaches per hue, from
/// darktable colorbalancergb `commit_params()`: every RGB on a 92³ grid in [0, 1], then a
/// circular 5-tap box average. `rgb_to_xyz_d65` maps working RGB to XYZ D65.
pub fn jzazbz_gamut_lut(rgb_to_xyz_d65: &Mat3) -> Vec<f32> {
    const STEPS: usize = 92;
    let last = (STEPS - 1) as f32;
    let mut sampler = vec![0.0f32; GAMUT_LUT_SIZE];
    for r in 0..STEPS {
        for g in 0..STEPS {
            for b in 0..STEPS {
                let rgb = [r as f32 / last, g as f32 / last, b as f32 / last];
                let [jz, az, bz] = xyz_to_jzazbz(apply(rgb_to_xyz_d65, rgb));
                let saturation = if jz > 0.0 { bz.hypot(az) / jz } else { 0.0 };
                let hue = bz.atan2(az);
                let index = ((GAMUT_LUT_SIZE - 1) as f32 * (hue + PI) / (2.0 * PI)).round() as i32;
                let index = index.rem_euclid(GAMUT_LUT_SIZE as i32) as usize;
                sampler[index] = sampler[index].max(saturation);
            }
        }
    }
    (0..GAMUT_LUT_SIZE)
        .map(|k| {
            let tap = |offset: isize| {
                sampler[(k as isize + offset).rem_euclid(GAMUT_LUT_SIZE as isize) as usize]
            };
            (tap(-2) + tap(-1) + tap(0) + tap(1) + tap(2)) / 5.0
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rec2020_white_maps_to_d50() {
        let xyz = apply(&WorkingProfile::linear_rec2020().rgb_to_xyz_d50, [1.0; 3]);
        for (got, want) in xyz.iter().zip([0.9642, 1.0, 0.8249]) {
            assert!((got - want).abs() < 1e-4, "{xyz:?}");
        }
    }

    #[test]
    fn profile_matrices_are_inverse() {
        let profile = WorkingProfile::linear_rec2020();
        let identity = mul(&profile.xyz_d50_to_rgb, &profile.rgb_to_xyz_d50);
        for (r, row) in identity.iter().enumerate() {
            for (c, value) in row.iter().enumerate() {
                let want = if r == c { 1.0 } else { 0.0 };
                assert!((value - want).abs() < 1e-5, "{identity:?}");
            }
        }
    }

    #[test]
    fn uncompensated_lightness_is_the_grey_luminance() {
        let profile = WorkingProfile::linear_rec2020();
        assert_eq!(profile.uncompensate_middle_grey(0.0), 0.0);
        assert!((profile.uncompensate_middle_grey(1.0) - 1.0).abs() < 1e-5);
        // L 50 is middle grey, Y 0.18419.
        assert!((profile.uncompensate_middle_grey(0.5) - 0.184187).abs() < 1e-5);
        // Below L 8, CIE's linear segment: Y = L / κ.
        let dark = profile.uncompensate_middle_grey(0.05);
        assert!((dark - 5.0 * 27.0 / 24389.0).abs() < 1e-6, "{dark}");
    }

    #[test]
    fn compensating_middle_grey_undoes_uncompensating_it() {
        let profile = WorkingProfile::linear_rec2020();
        assert!(profile.compensate_middle_grey(0.0).abs() < 1e-6);
        assert_eq!(
            profile.compensate_middle_grey(profile.uncompensate_middle_grey(1.0)),
            1.0
        );
        // Both sides of CIE's linear segment, below L 8.
        for x in [0.001, 0.05, 0.079, 0.081, 0.25, 0.5, 0.75, 0.9975] {
            let back = profile.compensate_middle_grey(profile.uncompensate_middle_grey(x));
            assert!((back - x).abs() < 1e-6, "{x}: {back}");
        }
    }

    #[test]
    fn lab_round_trips_and_d50_is_white() {
        let white = xyz_to_lab_d50([0.9642, 1.0, 0.8249]);
        assert!((white[0] - 100.0).abs() < 1e-4 && white[1].abs() < 1e-4 && white[2].abs() < 1e-4);
        // Both sides of the cube root's threshold, and a colour with a negative channel.
        for lab in [
            [50.0, 20.0, -30.0],
            [3.0, 1.0, 2.0],
            [97.0, -60.0, 80.0],
            [20.0, 0.0, 0.0],
        ] {
            let back = xyz_to_lab_d50(lab_to_xyz_d50(lab));
            for c in 0..3 {
                assert!(
                    (back[c] - lab[c]).abs() < 1e-3,
                    "{lab:?} came back {back:?}"
                );
            }
        }
    }

    #[test]
    fn rgb_norms_are_darktables() {
        for norm in RgbNorm::ALL {
            assert_eq!(RgbNorm::from_darktable(norm.to_darktable()), Some(norm));
        }
        assert_eq!(RgbNorm::from_darktable(7), None);
        assert_eq!(RgbNorm::from_darktable(-1), None);
        let luminance = WorkingProfile::linear_rec2020().rgb_to_xyz_d50[1];
        let rgb = [0.2, 0.4, 0.1];
        let values = RgbNorm::ALL.map(|norm| norm.of(rgb, luminance));
        let expected = [
            0.7 / 3.0,
            luminance[0] * 0.2 + luminance[1] * 0.4 + luminance[2] * 0.1,
            0.4,
            0.7 / 3.0,
            0.7,
            0.21f32.sqrt(),
            0.073 / 0.21,
        ];
        for (value, want) in values.iter().zip(expected) {
            assert!((value - want).abs() < 1e-6, "{values:?}");
        }
        assert_eq!(RgbNorm::Power.of([0.0; 3], luminance), 0.0);
    }

    #[test]
    fn yrg_round_trips_through_lms() {
        let yrg = [0.4, 0.3, 0.5];
        let back = lms_to_yrg(yrg_to_lms(yrg));
        for (got, want) in back.iter().zip(yrg) {
            assert!((got - want).abs() < 1e-5, "{back:?}");
        }
    }

    #[test]
    fn jzazbz_of_black_is_zero() {
        assert_eq!(xyz_to_jzazbz([0.0; 3])[0], 0.0);
    }

    fn assert_close(a: &Mat3d, b: &Mat3d, tolerance: f64) {
        for (row_a, row_b) in a.iter().zip(b) {
            for (x, y) in row_a.iter().zip(row_b) {
                assert!((x - y).abs() < tolerance, "{a:?} != {b:?}");
            }
        }
    }

    #[test]
    fn d50_chromaticities_rebuild_the_profile_matrix() {
        // Bradford adaptation is linear, so the adapted primaries and the D50 white give
        // back lcms's adapted matrix, where every colorant is non-negative.
        for space in [RgbSpace::ADOBE_RGB, RgbSpace::SRGB] {
            let c = space.chromaticities();
            let rebuilt = rgb_to_xyz_from_chromaticities(c.primaries, c.white);
            assert_close(&rebuilt, &space.rgb_to_xyz_d50(), 1e-9);
        }
        let white = RgbSpace::LINEAR_REC2020.chromaticities().white;
        assert!((white[0] - 0.3457).abs() < 1e-4 && (white[1] - 0.3585).abs() < 1e-4);
    }

    #[test]
    fn negative_colorants_are_clamped_like_darktable() {
        // Adapted to D50, the red colorants of Rec2020 and Display P3, whose z is 0 in
        // D65, get a slightly negative Z, which darktable's dt_D50_XYZ_to_xyY clamps to 0
        // before taking their chromaticity.
        for space in [RgbSpace::LINEAR_REC2020, RgbSpace::DISPLAY_P3] {
            let m = space.rgb_to_xyz_d50();
            assert!(m[2][0] < 0.0, "{m:?}");
            let red = space.chromaticities().primaries[0];
            let (x, y) = (m[0][0], m[1][0]);
            assert!((red[0] - x / (x + y)).abs() < 1e-12 && (red[1] - y / (x + y)).abs() < 1e-12);
        }
    }

    #[test]
    fn unscaled_unrotated_primary_stays() {
        let c = RgbSpace::LINEAR_REC2020.chromaticities();
        for i in 0..3 {
            let p = rotate_and_scale_primary(&c, 1.0, 0.0, i);
            assert!((p[0] - c.primaries[i][0]).abs() < 1e-12, "{p:?}");
            assert!((p[1] - c.primaries[i][1]).abs() < 1e-12, "{p:?}");
        }
        let half = rotate_and_scale_primary(&c, 0.5, 0.0, 0);
        let expected = [0, 1].map(|k| (c.white[k] + c.primaries[0][k]) / 2.0);
        assert!((half[0] - expected[0]).abs() < 1e-12 && (half[1] - expected[1]).abs() < 1e-12);
    }

    #[test]
    fn checked_inverse_refuses_what_mat3sseinv_refuses() {
        let m = RgbSpace::LINEAR_REC2020.rgb_to_xyz_d50();
        assert_eq!(checked_inverse_d(&m), Some(inverse_d(&m)));
        // Determinants of 1e-6, inverted, and of 1e-8 and 0, refused.
        let diagonal = |a: f64| [[a, 0.0, 0.0], [0.0, a, 0.0], [0.0, 0.0, 1.0]];
        assert!(checked_inverse_d(&diagonal(1e-3)).is_some());
        assert_eq!(checked_inverse_d(&diagonal(1e-4)), None);
        assert_eq!(checked_inverse_d(&[[0.0; 3]; 3]), None);
    }

    #[test]
    fn primaries_on_one_point_or_line_give_the_zero_matrix() {
        // darktable's mat3SSEinv refuses the singular matrix and leaves its inverse 0.
        let c = RgbSpace::LINEAR_REC2020.chromaticities();
        let [red, green, blue] = c.primaries;
        let between = [(red[0] + green[0]) / 2.0, (red[1] + green[1]) / 2.0];
        for primaries in [[green, green, blue], [red, between, green]] {
            let m = rgb_to_xyz_from_chromaticities(primaries, c.white);
            assert_eq!(m, [[0.0; 3]; 3], "{primaries:?}");
        }
        // Just off the line, the matrix is darktable's, large but finite.
        let off = [between[0], between[1] + 1e-6];
        let m = rgb_to_xyz_from_chromaticities([red, off, green], c.white);
        assert!(
            m.iter().flatten().all(|v| v.is_finite()) && m[0][0] != 0.0,
            "{m:?}"
        );
    }
}
