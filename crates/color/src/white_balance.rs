// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/iop/temperature.c)
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

//! darktable's white balance module's model of its coefficients as a temperature and a
//! tint (`src/iop/temperature.c`: `_temperature_to_XYZ`, `_mul2temp`, `_temp2mul`), in
//! `f64` as darktable computes it. A temperature's light is a black body below 4000 K and
//! CIE daylight from 4000 K, integrated over the CIE 1931 2° observer at darktable's 5 nm
//! and scaled to a largest component of 1; the coefficients see it through the camera's
//! colour matrix, and tint is the ratio of the light's Y/X to theirs. darktable's tables
//! (`src/external/cie_colorimetric_tables.c`) and lcms2's daylight locus
//! (`cmsWhitePointFromTemp`) are restated here, so no lcms2 is linked (decision 5).
//!
//! darktable bisects the temperature to 1 K; [`Matrices::temperature_tint`] bisects to
//! convergence, so that it inverts [`Matrices::coefficients`] but for a rounding.

use crate::{Mat3, Mat3d};

/// darktable's temperatures, `DT_IOP_LOWEST_TEMPERATURE` to `DT_IOP_HIGHEST_TEMPERATURE`.
pub const TEMPERATURE_RANGE: (f64, f64) = (1901.0, 25000.0);
/// darktable's tints, `DT_IOP_LOWEST_TINT` to `DT_IOP_HIGHEST_TINT`.
pub const TINT_RANGE: (f64, f64) = (0.135, 2.326);

/// Below it a temperature's light is a black body, from it CIE daylight
/// (`INITIALBLACKBODYTEMPERATURE`).
const BLACK_BODY_BELOW: f64 = 4000.0;

/// darktable's D65, `D65xyY`, which `_spd_daylight` starts from.
const D65: [f64; 2] = [0.31271, 0.32902];

/// lcms2's `cmsWhitePointFromTemp` (lcms2 2.x, `cmswtpnt.c`, MIT): the CIE daylight
/// locus's chromaticity at `t` K, from 4000 K to 25000 K, `None` outside.
pub fn daylight_locus(t: f64) -> Option<[f64; 2]> {
    let (t2, t3) = (t * t, t * t * t);
    let x = if (4000.0..=7000.0).contains(&t) {
        -4.6070 * (1E9 / t3) + 2.9678 * (1E6 / t2) + 0.09911 * (1E3 / t) + 0.244063
    } else if t > 7000.0 && t <= 25000.0 {
        -2.0064 * (1E9 / t3) + 1.9018 * (1E6 / t2) + 0.24748 * (1E3 / t) + 0.237040
    } else {
        return None;
    };
    Some([x, -3.000 * (x * x) + 2.870 * x - 0.275])
}

/// Planck's law, darktable's `_spd_blackbody` with its two constants, at `wavelength`
/// nm. darktable takes it in `long double`; `f64` here.
fn black_body(wavelength: u32, t: f64) -> f64 {
    const C1: f64 = 3.7417715246641281639549488324352159753e-16;
    const C2: f64 = 0.014387769599838156481252937624049081933;
    let lambda = f64::from(wavelength) * 1e-9;
    C1 / (lambda.powi(5) * ((C2 / (lambda * t)).exp() - 1.0))
}

/// CIE daylight at `t` K, darktable's `_spd_daylight`: S0 + M1 S1 + M2 S2 at `wavelength`
/// nm, the M from the daylight locus's chromaticity, D65's where lcms2 gives none.
fn daylight(wavelength: u32, t: f64) -> f64 {
    let [x, y] = daylight_locus(t).unwrap_or(D65);
    let m = 0.0241 + 0.2562 * x - 0.7341 * y;
    let m1 = (-1.3515 - 1.7703 * x + 5.9114 * y) / m;
    let m2 = (0.0300 - 31.4424 * x + 30.0717 * y) / m;
    let [s0, s1, s2] = DAYLIGHT[((wavelength - DAYLIGHT_FROM) / STEP) as usize];
    s0 + m1 * s1 + m2 * s2
}

/// darktable's `_spectrum_to_XYZ`: the light `spd` gives at `t` K over the observer,
/// scaled to a largest component of 1.
fn spectrum_to_xyz(t: f64, spd: fn(u32, f64) -> f64) -> [f64; 3] {
    let mut xyz = [0.0; 3];
    for (i, observer) in OBSERVER.iter().enumerate() {
        let power = spd(OBSERVER_FROM + STEP * i as u32, t);
        for c in 0..3 {
            xyz[c] += power * observer[c];
        }
    }
    let max = xyz[0].max(xyz[1]).max(xyz[2]);
    xyz.map(|v| v / max)
}

/// darktable's `_temperature_to_XYZ`: the light at `t` K, held to the temperatures'
/// range, a black body below 4000 K and daylight from it.
pub fn temperature_to_xyz(t: f64) -> [f64; 3] {
    let (min, max) = TEMPERATURE_RANGE;
    let t = t.clamp(min, max);
    if t < BLACK_BODY_BELOW {
        spectrum_to_xyz(t, black_body)
    } else {
        spectrum_to_xyz(t, daylight)
    }
}

/// darktable's `dt_colorspaces_pseudoinverse` of a 3×3 matrix, `m (mᵀm)⁻¹`, by
/// Gauss–Jordan on the normal equations in `f64`, without pivoting.
fn pseudoinverse(m: &Mat3d) -> Mat3d {
    let mut work = [[0.0f64; 6]; 3];
    for (i, row) in work.iter_mut().enumerate() {
        row[i + 3] = 1.0;
        for j in 0..3 {
            for line in m {
                row[j] += line[i] * line[j];
            }
        }
    }
    for i in 0..3 {
        let pivot = work[i][i];
        for value in &mut work[i] {
            *value /= pivot;
        }
        for k in 0..3 {
            if k == i {
                continue;
            }
            let (factor, pivot_row) = (work[k][i], work[i]);
            for (value, pivot) in work[k].iter_mut().zip(pivot_row) {
                *value -= pivot * factor;
            }
        }
    }
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| work[j][k + 3] * m[i][k]).sum()))
}

fn apply(m: &Mat3d, v: [f64; 3]) -> [f64; 3] {
    m.map(|row| row[0] * v[0] + row[1] * v[1] + row[2] * v[2])
}

/// The camera's colour matrices as darktable's white balance reads them: XYZ to camera
/// and back (`dt_colorspaces_conversion_matrices_xyz`, its inverse the pseudoinverse
/// transposed).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrices {
    pub xyz_to_camera: Mat3d,
    pub camera_to_xyz: Mat3d,
}

impl Matrices {
    /// From the camera's XYZ to camera matrix, as the image context holds it: a DNG's
    /// embedded one where valid, else the camera's (`darktable`'s `d65_color_matrix`,
    /// else `adobe_XYZ_to_CAM`).
    pub fn camera(xyz_to_camera: &Mat3) -> Self {
        let xyz_to_camera = xyz_to_camera.map(|row| row.map(f64::from));
        let inverse = pseudoinverse(&xyz_to_camera);
        let camera_to_xyz = std::array::from_fn(|i| std::array::from_fn(|j| inverse[j][i]));
        Self {
            xyz_to_camera,
            camera_to_xyz,
        }
    }

    /// sRGB's, which darktable's `_prepare_matrices` assumes for an image that is not a
    /// raw.
    pub fn srgb() -> Self {
        Self {
            xyz_to_camera: [
                [3.2404542, -1.5371385, -0.4985314],
                [-0.9692660, 1.8760108, 0.0415560],
                [0.0556434, -0.2040259, 1.0572252],
            ],
            camera_to_xyz: [
                [0.4124564, 0.3575761, 0.1804375],
                [0.2126729, 0.7151522, 0.0721750],
                [0.0193339, 0.1191920, 0.9503041],
            ],
        }
    }

    /// darktable's `_mul2xyz`: the light the coefficients neutralise, one over each, 0
    /// where one is not positive, through the camera's matrix.
    pub fn xyz(&self, coefficients: [f64; 3]) -> [f64; 3] {
        let camera = coefficients.map(|c| if c > 0.0 { 1.0 / c } else { 0.0 });
        apply(&self.camera_to_xyz, camera)
    }

    /// darktable's `_temp2mul`, normalised as `_temp_tint_callback` normalises it: the
    /// coefficients that neutralise the light at `t` K with `tint`, its Y divided by it,
    /// over green's, so green is 1.
    pub fn coefficients(&self, t: f64, tint: f64) -> [f64; 3] {
        let mut xyz = temperature_to_xyz(t);
        xyz[1] /= tint;
        let multipliers = apply(&self.xyz_to_camera, xyz).map(|c| 1.0 / c);
        multipliers.map(|m| m / multipliers[1])
    }

    /// The temperature and tint of the coefficients, darktable's `_mul2temp` but bisected
    /// to convergence and unclamped: the temperature, in the range, whose light has the
    /// coefficients' Z/X, and tint its Y/X over theirs. A light outside the model's reach
    /// reads at an end of the temperatures, and a tint with no meaning, of a light with no
    /// X, reads 1.
    pub fn temperature_tint(&self, coefficients: [f64; 3]) -> [f64; 2] {
        let xyz = self.xyz(coefficients);
        let target = xyz[2] / xyz[0];
        let (mut low, mut high) = TEMPERATURE_RANGE;
        loop {
            let middle = 0.5 * (low + high);
            if middle <= low || middle >= high {
                break;
            }
            let light = temperature_to_xyz(middle);
            if light[2] / light[0] > target {
                high = middle;
            } else {
                low = middle;
            }
        }
        let tint = tint_at(low, xyz);
        [low, if tint.is_finite() { tint } else { 1.0 }]
    }
}

/// The tint of the light `xyz` at `t` K: the Y/X of the light at `t` over its own, as
/// darktable's `_XYZ_to_temperature` takes it.
pub fn tint_at(t: f64, xyz: [f64; 3]) -> f64 {
    let light = temperature_to_xyz(t);
    (light[1] / light[0]) / (xyz[1] / xyz[0])
}

/// The tables' first wavelengths and their step, in nm.
const OBSERVER_FROM: u32 = 380;
const DAYLIGHT_FROM: u32 = 300;
const STEP: u32 = 5;

// darktable's CIE tables (`src/external/cie_colorimetric_tables.c`), from CIE 15:2004's
// selected colorimetric tables.
/// The CIE 1931 2° standard colorimetric observer, x̄, ȳ and z̄, from 380 nm to 780 nm by 5 nm.
const OBSERVER: [[f64; 3]; 81] = [
    [0.001368, 0.000039, 0.006450],
    [0.002236, 0.000064, 0.010550],
    [0.004243, 0.000120, 0.020050],
    [0.007650, 0.000217, 0.036210],
    [0.014310, 0.000396, 0.067850],
    [0.023190, 0.000640, 0.110200],
    [0.043510, 0.001210, 0.207400],
    [0.077630, 0.002180, 0.371300],
    [0.134380, 0.004000, 0.645600],
    [0.214770, 0.007300, 1.039050],
    [0.283900, 0.011600, 1.385600],
    [0.328500, 0.016840, 1.622960],
    [0.348280, 0.023000, 1.747060],
    [0.348060, 0.029800, 1.782600],
    [0.336200, 0.038000, 1.772110],
    [0.318700, 0.048000, 1.744100],
    [0.290800, 0.060000, 1.669200],
    [0.251100, 0.073900, 1.528100],
    [0.195360, 0.090980, 1.287640],
    [0.142100, 0.112600, 1.041900],
    [0.095640, 0.139020, 0.812950],
    [0.057950, 0.169300, 0.616200],
    [0.032010, 0.208020, 0.465180],
    [0.014700, 0.258600, 0.353300],
    [0.004900, 0.323000, 0.272000],
    [0.002400, 0.407300, 0.212300],
    [0.009300, 0.503000, 0.158200],
    [0.029100, 0.608200, 0.111700],
    [0.063270, 0.710000, 0.078250],
    [0.109600, 0.793200, 0.057250],
    [0.165500, 0.862000, 0.042160],
    [0.225750, 0.914850, 0.029840],
    [0.290400, 0.954000, 0.020300],
    [0.359700, 0.980300, 0.013400],
    [0.433450, 0.994950, 0.008750],
    [0.512050, 1.000000, 0.005750],
    [0.594500, 0.995000, 0.003900],
    [0.678400, 0.978600, 0.002750],
    [0.762100, 0.952000, 0.002100],
    [0.842500, 0.915400, 0.001800],
    [0.916300, 0.870000, 0.001650],
    [0.978600, 0.816300, 0.001400],
    [1.026300, 0.757000, 0.001100],
    [1.056700, 0.694900, 0.001000],
    [1.062200, 0.631000, 0.000800],
    [1.045600, 0.566800, 0.000600],
    [1.002600, 0.503000, 0.000340],
    [0.938400, 0.441200, 0.000240],
    [0.854450, 0.381000, 0.000190],
    [0.751400, 0.321000, 0.000100],
    [0.642400, 0.265000, 0.000050],
    [0.541900, 0.217000, 0.000030],
    [0.447900, 0.175000, 0.000020],
    [0.360800, 0.138200, 0.000010],
    [0.283500, 0.107000, 0.000000],
    [0.218700, 0.081600, 0.000000],
    [0.164900, 0.061000, 0.000000],
    [0.121200, 0.044580, 0.000000],
    [0.087400, 0.032000, 0.000000],
    [0.063600, 0.023200, 0.000000],
    [0.046770, 0.017000, 0.000000],
    [0.032900, 0.011920, 0.000000],
    [0.022700, 0.008210, 0.000000],
    [0.015840, 0.005723, 0.000000],
    [0.011359, 0.004102, 0.000000],
    [0.008111, 0.002929, 0.000000],
    [0.005790, 0.002091, 0.000000],
    [0.004109, 0.001484, 0.000000],
    [0.002899, 0.001047, 0.000000],
    [0.002049, 0.000740, 0.000000],
    [0.001440, 0.000520, 0.000000],
    [0.001000, 0.000361, 0.000000],
    [0.000690, 0.000249, 0.000000],
    [0.000476, 0.000172, 0.000000],
    [0.000332, 0.000120, 0.000000],
    [0.000235, 0.000085, 0.000000],
    [0.000166, 0.000060, 0.000000],
    [0.000117, 0.000042, 0.000000],
    [0.000083, 0.000030, 0.000000],
    [0.000059, 0.000021, 0.000000],
    [0.000042, 0.000015, 0.000000],
];

/// CIE daylight's components, S0, S1 and S2, from 300 nm to 830 nm by 5 nm.
const DAYLIGHT: [[f64; 3]; 107] = [
    [0.04, 0.02, 0.00],
    [3.02, 2.26, 1.00],
    [6.00, 4.50, 2.00],
    [17.80, 13.45, 3.00],
    [29.60, 22.40, 4.00],
    [42.45, 32.20, 6.25],
    [55.30, 42.00, 8.50],
    [56.30, 41.30, 8.15],
    [57.30, 40.60, 7.80],
    [59.55, 41.10, 7.25],
    [61.80, 41.60, 6.70],
    [61.65, 39.80, 6.00],
    [61.50, 38.00, 5.30],
    [65.15, 40.20, 5.70],
    [68.80, 42.40, 6.10],
    [66.10, 40.45, 4.55],
    [63.40, 38.50, 3.00],
    [64.60, 36.75, 2.10],
    [65.80, 35.00, 1.20],
    [80.30, 39.20, 0.05],
    [94.80, 43.40, -1.10],
    [99.80, 44.85, -0.80],
    [104.80, 46.30, -0.50],
    [105.35, 45.10, -0.60],
    [105.90, 43.90, -0.70],
    [101.35, 40.50, -0.95],
    [96.80, 37.10, -1.20],
    [105.35, 36.90, -1.90],
    [113.90, 36.70, -2.60],
    [119.75, 36.30, -2.75],
    [125.60, 35.90, -2.90],
    [125.55, 34.25, -2.85],
    [125.50, 32.60, -2.80],
    [123.40, 30.25, -2.70],
    [121.30, 27.90, -2.60],
    [121.30, 26.10, -2.60],
    [121.30, 24.30, -2.60],
    [117.40, 22.20, -2.20],
    [113.50, 20.10, -1.80],
    [113.30, 18.15, -1.65],
    [113.10, 16.20, -1.50],
    [111.95, 14.70, -1.40],
    [110.80, 13.20, -1.30],
    [108.65, 10.90, -1.25],
    [106.50, 8.60, -1.20],
    [107.65, 7.35, -1.10],
    [108.80, 6.10, -1.00],
    [107.05, 5.15, -0.75],
    [105.30, 4.20, -0.50],
    [104.85, 3.05, -0.40],
    [104.40, 1.90, -0.30],
    [102.20, 0.95, -0.15],
    [100.00, 0.00, 0.00],
    [98.00, -0.80, 0.10],
    [96.00, -1.60, 0.20],
    [95.55, -2.55, 0.35],
    [95.10, -3.50, 0.50],
    [92.10, -3.50, 1.30],
    [89.10, -3.50, 2.10],
    [89.80, -4.65, 2.65],
    [90.50, -5.80, 3.20],
    [90.40, -6.50, 3.65],
    [90.30, -7.20, 4.10],
    [89.35, -7.90, 4.40],
    [88.40, -8.60, 4.70],
    [86.20, -9.05, 4.90],
    [84.00, -9.50, 5.10],
    [84.55, -10.20, 5.90],
    [85.10, -10.90, 6.70],
    [83.50, -10.80, 7.00],
    [81.90, -10.70, 7.30],
    [82.25, -11.35, 7.95],
    [82.60, -12.00, 8.60],
    [83.75, -13.00, 9.20],
    [84.90, -14.00, 9.80],
    [83.10, -13.80, 10.00],
    [81.30, -13.60, 10.20],
    [76.60, -12.80, 9.25],
    [71.90, -12.00, 8.30],
    [73.10, -12.65, 8.95],
    [74.30, -13.30, 9.60],
    [75.35, -13.10, 9.05],
    [76.40, -12.90, 8.50],
    [69.85, -11.75, 7.75],
    [63.30, -10.60, 7.00],
    [67.50, -11.10, 7.30],
    [71.70, -11.60, 7.60],
    [74.35, -11.90, 7.80],
    [77.00, -12.20, 8.00],
    [71.10, -11.20, 7.35],
    [65.20, -10.20, 6.70],
    [56.45, -9.00, 5.95],
    [47.70, -7.80, 5.20],
    [58.15, -9.50, 6.30],
    [68.60, -11.20, 7.40],
    [66.80, -10.80, 7.10],
    [65.00, -10.40, 6.80],
    [65.50, -10.50, 6.90],
    [66.00, -10.60, 7.00],
    [63.50, -10.15, 6.70],
    [61.00, -9.70, 6.40],
    [57.15, -9.00, 5.95],
    [53.30, -8.30, 5.50],
    [56.10, -8.80, 5.80],
    [58.90, -9.30, 6.10],
    [60.40, -9.55, 6.30],
    [61.90, -9.80, 6.50],
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mul_d;

    /// darktable's camera matrix for the Fujifilm X-T4 of its integration tests.
    fn x_t4() -> Matrices {
        Matrices::camera(&[
            [1.3426, -0.6334, -0.1177],
            [-0.4244, 1.2136, 0.2371],
            [-0.058, 0.1303, 0.598],
        ])
    }

    #[test]
    fn the_daylight_locus_is_darktable_s_and_ends_where_lcms2_s_does() {
        for t in [4000.0, 5003.0, 6500.0, 7000.0, 7001.0, 12000.0, 25000.0] {
            let [x, y] = daylight_locus(t).unwrap();
            let [dx, dy] = crate::illuminants::cct_to_xy_daylight(t as f32);
            assert!((x - f64::from(dx)).abs() < 2e-6 && (y - f64::from(dy)).abs() < 2e-6);
        }
        assert_eq!(daylight_locus(3999.0), None);
        assert_eq!(daylight_locus(25001.0), None);
    }

    /// x and y of a light.
    fn xy([x, y, z]: [f64; 3]) -> [f64; 2] {
        [x / (x + y + z), y / (x + y + z)]
    }

    #[test]
    fn a_temperature_s_light_is_a_black_body_below_4000_k_and_daylight_from_it() {
        // CIE A is the black body at 2856 K; D65 daylight near 6504 K.
        let [x, y] = xy(temperature_to_xyz(2856.0));
        assert!(
            (x - 0.44757).abs() < 1e-3 && (y - 0.40745).abs() < 1e-3,
            "{x} {y}"
        );
        let [x, y] = xy(temperature_to_xyz(6504.0));
        assert!(
            (x - D65[0]).abs() < 1e-3 && (y - D65[1]).abs() < 1e-3,
            "{x} {y}"
        );
        // Scaled to a largest component of 1, and held to the range.
        for t in [1901.0, 3000.0, 4000.0, 9000.0, 25000.0] {
            let light = temperature_to_xyz(t);
            assert_eq!(light.iter().copied().fold(0.0, f64::max), 1.0, "{t}");
        }
        assert_eq!(temperature_to_xyz(100.0), temperature_to_xyz(1901.0));
        assert_eq!(temperature_to_xyz(1e6), temperature_to_xyz(25000.0));
    }

    fn z_over_x(t: f64) -> f64 {
        let light = temperature_to_xyz(t);
        light[2] / light[0]
    }

    #[test]
    fn z_over_x_rises_with_the_temperature_but_steps_down_at_4000_k() {
        // Every kelvin of each side, so bisection finds a temperature on either.
        for range in [1901..=3999, 4000..=25000] {
            let mut last = 0.0;
            for t in range {
                let ratio = z_over_x(f64::from(t));
                assert!(ratio > last, "{t}");
                last = ratio;
            }
        }
        // darktable's switch from a black body to daylight steps Z/X down by 4 %, so a
        // light from 3900.33 K to 4102.41 K has a twin on the other side of 4000 K, with
        // the same Z/X.
        let (below, at) = (z_over_x(4000.0 - 1e-9), z_over_x(4000.0));
        assert!((below - 0.638165).abs() < 1e-6 && (at - 0.611897).abs() < 1e-6);
        let [low, high] = TWINS;
        assert!(z_over_x(low - 0.01) < at && z_over_x(low + 0.01) > at);
        assert!(z_over_x(high - 0.01) < below && z_over_x(high + 0.01) > below);
    }

    /// The temperatures that share their Z/X with a twin across 4000 K.
    const TWINS: [f64; 2] = [3900.33, 4102.41];

    /// How far `m`'s two matrices are from inverting each other.
    fn off_identity(m: &Matrices) -> f64 {
        let product = mul_d(&m.camera_to_xyz, &m.xyz_to_camera);
        let mut off = 0.0f64;
        for (i, row) in product.iter().enumerate() {
            for (j, v) in row.iter().enumerate() {
                off = off.max((v - if i == j { 1.0 } else { 0.0 }).abs());
            }
        }
        off
    }

    #[test]
    fn the_pseudoinverse_inverts_the_camera_s_matrix() {
        assert!(off_identity(&x_t4()) < 1e-12);
        // darktable's sRGB pair is its seven-digit constants, inverse to 1e-6 only.
        assert!(off_identity(&Matrices::srgb()) < 1e-6);
    }

    #[test]
    fn temperature_and_tint_invert_the_coefficients_they_give() {
        // Through a camera's matrix and its pseudoinverse: the X-T4's, and sRGB's as a
        // camera's.
        let srgb = Matrices::camera(&crate::to_f32(Matrices::srgb().xyz_to_camera));
        for matrices in [x_t4(), srgb] {
            for t in [
                1901.0, 2500.0, 3900.0, 3950.0, 3999.0, 4000.0, 4050.0, 4103.0, 5003.0, 6500.0,
                12000.0, 25000.0,
            ] {
                for tint in [0.135, 0.8, 1.0, 1.25, 2.326] {
                    let coefficients = matrices.coefficients(t, tint);
                    assert_eq!(coefficients[1], 1.0);
                    // Only a light the coefficients' range holds, 0 to 8: towards the
                    // corners one tends to 0 or to infinity, where the model turns
                    // ill-conditioned.
                    if coefficients.iter().any(|&c| c <= 0.0 || c > 8.0) {
                        continue;
                    }
                    let [back, back_tint] = matrices.temperature_tint(coefficients);
                    // The coefficients come back from either twin.
                    let again = matrices.coefficients(back, back_tint);
                    for (a, b) in again.iter().zip(coefficients) {
                        assert!((a - b).abs() < 1e-12 * b, "{t} {tint}: {again:?}");
                    }
                    // The coordinates too, but for a twin across 4000 K.
                    let [low, high] = TWINS;
                    if (low..=high).contains(&t) {
                        continue;
                    }
                    assert!((back - t).abs() < 1e-6 * t, "{t} {tint}: {back}");
                    assert!((back_tint - tint).abs() < 1e-9, "{t} {tint}: {back_tint}");
                }
            }
        }
    }
}
