// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2017-2023  darktable developers (ported from
// src/common/colorspaces_inline_conversions.h)
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

//! darktable UCS 22 (Aurélien Pierre's uniform colour space of 2022) and CIE xyY, as
//! darktable converts to them (`colorspaces_inline_conversions.h`), in f32 like
//! darktable, for the setup of a kernel that works in it. The kernels' twins are in
//! `color.slang`.

#![allow(
    clippy::excessive_precision,
    reason = "constants are copied digit for digit from darktable"
)]

/// darktable's D65 white, `D65xyY`, as its chromaticity.
pub const D65_XY: [f32; 2] = [0.31271, 0.32902];

/// darktable `DT_UCS_L_STAR_RANGE`: the lightness L* tends to as Y grows.
pub const L_STAR_RANGE: f32 = 2.098883786377;

/// darktable `DT_UCS_L_STAR_UPPER_LIMIT`: the largest L* converted back to Y, just under
/// the range, so the conversion never divides by zero.
pub const L_STAR_UPPER_LIMIT: f32 = 2.09885;

/// darktable `dt_D65_XYZ_to_xyY`: chromaticity and luminance, negative XYZ clipped to 0
/// first; black has D65's chromaticity.
pub fn d65_xyz_to_xyy(xyz: [f32; 3]) -> [f32; 3] {
    let [x, y, z] = xyz.map(|v| v.max(0.0));
    let sum = x + y + z;
    if sum > 0.0 {
        [x / sum, y / sum, y]
    } else {
        [D65_XY[0], D65_XY[1], y]
    }
}

/// darktable `dt_xyY_to_XYZ`: black where y is 0.
pub fn xyy_to_xyz(xyy: [f32; 3]) -> [f32; 3] {
    let [x, y, luminance] = xyy;
    if y == 0.0 {
        return [0.0; 3];
    }
    [luminance * x / y, luminance, luminance * (1.0 - x - y) / y]
}

/// darktable `Y_to_dt_UCS_L_star`: the lightness L* of luminance `y`, 0 for black.
pub fn y_to_l_star(y: f32) -> f32 {
    let y_hat = y.powf(0.631651345306265);
    L_STAR_RANGE * y_hat / (y_hat + 1.12426773749357)
}

/// darktable `dt_UCS_L_star_to_Y`, the inverse of [`y_to_l_star`] below the range.
pub fn l_star_to_y(l_star: f32) -> f32 {
    (1.12426773749357 * l_star / (L_STAR_RANGE - l_star)).powf(1.5831518565279648)
}

/// darktable `xyY_to_dt_UCS_UV`: the chromaticity as darktable UCS U*′ and V*′, whose
/// angle is the hue and whose squared length is the colourfulness M², up to a power.
pub fn xy_to_uv_star_prime(xy: [f32; 2]) -> [f32; 2] {
    const X_FACTORS: [f32; 3] = [-0.783941002840055, 0.745273540913283, 0.318707282433486];
    const Y_FACTORS: [f32; 3] = [0.277512987809202, -0.205375866083878, 2.16743692732158];
    const OFFSETS: [f32; 3] = [0.153836578598858, -0.165478376301988, 0.291320554395942];
    const FACTORS: [f32; 2] = [1.39656225667, 1.4513954287];
    const HALF_VALUES: [f32; 2] = [1.49217352929, 1.52488637914];

    let uvd: [f32; 3] =
        std::array::from_fn(|c| X_FACTORS[c] * xy[0] + Y_FACTORS[c] * xy[1] + OFFSETS[c]);
    let div = if uvd[2] >= 0.0 {
        uvd[2].max(f32::MIN_POSITIVE)
    } else {
        uvd[2].min(-f32::MIN_POSITIVE)
    };
    let uv = [uvd[0] / div, uvd[1] / div];
    let uv_star: [f32; 2] =
        std::array::from_fn(|c| FACTORS[c] * uv[c] / (uv[c].abs() + HALF_VALUES[c]));
    [
        -1.124983854323892 * uv_star[0] - 0.980483721769325 * uv_star[1],
        1.86323315098672 * uv_star[0] + 1.971853092390862 * uv_star[1],
    ]
}

/// darktable `dt_UCS_LUV_to_JCH`: lightness J, chroma C and hue H in radians from a
/// lightness L* and the chromaticity's U*′ and V*′, relative to the white's L*.
pub fn luv_to_jch(l_star: f32, l_white: f32, uv: [f32; 2]) -> [f32; 3] {
    let m2 = uv[0] * uv[0] + uv[1] * uv[1];
    [
        l_star / l_white,
        15.932993652962535 * l_star.powf(0.6523997524738018) * m2.powf(0.6007557017508491)
            / l_white,
        uv[1].atan2(uv[0]),
    ]
}

/// darktable `dt_UCS_JCH_to_xyY`, L* clipped to the range it converts back from.
pub fn jch_to_xyy(jch: [f32; 3], l_white: f32) -> [f32; 3] {
    const FACTORS: [f32; 2] = [1.39656225667, 1.4513954287];
    const HALF_VALUES: [f32; 2] = [1.49217352929, 1.52488637914];
    const U_FACTORS: [f32; 3] = [0.167171472114775, -0.150959086409163, 0.940254742367256];
    const V_FACTORS: [f32; 3] = [0.141299802443708, -0.155185060382272, 1.000000000000000];
    const OFFSETS: [f32; 3] = [
        -0.00801531300850582,
        -0.00843312433578007,
        -0.0256325967652889,
    ];
    let l_star = (jch[0] * l_white).clamp(0.0, L_STAR_UPPER_LIMIT);
    let m = if l_star != 0.0 {
        (jch[1] * l_white / (15.932993652962535 * l_star.powf(0.6523997524738018)))
            .powf(0.8322850678616855)
    } else {
        0.0
    };
    let (u_star_prime, v_star_prime) = (m * jch[2].cos(), m * jch[2].sin());
    let uv_star = [
        -5.037522385190711 * u_star_prime - 2.504856328185843 * v_star_prime,
        4.760029407436461 * u_star_prime + 2.874012963239247 * v_star_prime,
    ];
    let uv: [f32; 2] =
        std::array::from_fn(|c| -HALF_VALUES[c] * uv_star[c] / (uv_star[c].abs() - FACTORS[c]));
    let xyd: [f32; 3] =
        std::array::from_fn(|c| U_FACTORS[c] * uv[0] + V_FACTORS[c] * uv[1] + OFFSETS[c]);
    let div = if xyd[2] >= 0.0 {
        xyd[2].max(f32::MIN_POSITIVE)
    } else {
        xyd[2].min(-f32::MIN_POSITIVE)
    };
    [xyd[0] / div, xyd[1] / div, l_star_to_y(l_star)]
}

/// darktable `dt_UCS_JCH_to_HSB`: hue, saturation S = C / B and brightness B.
pub fn jch_to_hsb(jch: [f32; 3]) -> [f32; 3] {
    let b = jch[0] * (jch[1].powf(1.33654221029386) + 1.0);
    let s = if b > 0.0 { jch[1] / b } else { 0.0 };
    [jch[2], s, b]
}

/// darktable `dt_UCS_HSB_to_JCH`.
pub fn hsb_to_jch(hsb: [f32; 3]) -> [f32; 3] {
    let c = hsb[1] * hsb[2];
    [hsb[2] / (c.powf(1.33654221029386) + 1.0), c, hsb[0]]
}

/// darktable `dt_UCS_22_build_gamut_LUT` (`src/common/darktable_ucs_22_helpers.h`): the
/// colourfulness squared, M², of the working gamut's boundary in darktable UCS, per
/// hue, [`crate::GAMUT_LUT_SIZE`] entries over [-π, π). It marches the triangle of the
/// primaries in CIE xy around D65 at 50 rays per entry, and averages the samples whose
/// UCS hue rounds to each entry; an entry no sample reaches is 0. `rgb_to_xyz_d65` maps
/// working RGB to XYZ D65. In f32, as darktable, but for the order of each entry's sum,
/// which darktable's threads share. Colour balance rgb's own copy
/// (`apertrust_op_colorbalancergb::ucs_gamut_lut`) is to call this (decision 65); the
/// colour equalizer does.
pub fn gamut_lut(rgb_to_xyz_d65: &crate::Mat3) -> Vec<f32> {
    use std::f32::consts::PI;
    const SIZE: usize = crate::GAMUT_LUT_SIZE;
    const SAMPLES: usize = 50 * SIZE;
    const TWO_PI: f32 = 2.0 * PI;
    let white = D65_XY;
    let primary = |rgb: [f32; 3]| {
        let [x, y, _] = d65_xyz_to_xyy(crate::apply(rgb_to_xyz_d65, rgb));
        [x, y]
    };
    let [red, green, blue] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]].map(primary);
    // The primaries' angles in xy around D65.
    let angle_of = |p: [f32; 2]| (p[1] - white[1]).atan2(p[0] - white[0]);
    let (h_red, h_green, h_blue) = (angle_of(red), angle_of(green), angle_of(blue));
    // darktable `Delta_H`: the difference of two angles, within [-π, π].
    let delta_h = |h_1: f32, h_2: f32| {
        let mut diff = h_1 - h_2;
        if diff < -PI {
            diff += TWO_PI;
        }
        if diff > PI {
            diff -= TWO_PI;
        }
        diff
    };
    let within = |t: f32| (0.0..=1.0).contains(&t);

    let mut sums = vec![0.0f32; SIZE];
    let mut counts = vec![0.0f32; SIZE];
    for i in 0..SAMPLES {
        let angle = -PI + i as f32 / SAMPLES as f32 * TWO_PI;
        let tan_angle = angle.tan();
        // Where the ray from D65 at this angle crosses the edge from `from` to `to`.
        let crossing = |from: [f32; 2], to: [f32; 2]| {
            let t = (white[1] - from[1] + tan_angle * (from[0] - white[0]))
                / (to[1] - from[1] + tan_angle * (from[0] - to[0]));
            [
                from[0] + t * (to[0] - from[0]),
                from[1] + t * (to[1] - from[1]),
            ]
        };
        let xy = if within(delta_h(angle, h_blue) / delta_h(h_red, h_blue)) {
            crossing(blue, red)
        } else if within(delta_h(angle, h_red) / delta_h(h_green, h_red)) {
            crossing(red, green)
        } else if within(delta_h(angle, h_green) / delta_h(h_blue, h_green)) {
            crossing(green, blue)
        } else {
            [0.0, 0.0]
        };
        let [u, v] = xy_to_uv_star_prime(xy);
        let hue = v.atan2(u);
        let index = ((SIZE - 1) as f32 * (hue + PI) / TWO_PI).round() as i32;
        let index = index.rem_euclid(SIZE as i32) as usize;
        sums[index] += u * u + v * v;
        counts[index] += 1.0;
    }
    sums.iter()
        .zip(&counts)
        .map(|(sum, count)| sum / count.max(1.0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_has_lightness_one_and_black_zero() {
        // darktable UCS normalises its lightness curve so that Y = 1 is about L* 0.988,
        // and the L_white of colour balance rgb's default white fulcrum.
        assert_eq!(y_to_l_star(0.0), 0.0);
        let white = y_to_l_star(1.0);
        assert!((white - L_STAR_RANGE / (1.0 + 1.12426773749357)).abs() < 1e-6);
        assert!((l_star_to_y(white) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn lightness_round_trips_through_luminance() {
        for y in [1e-4, 0.01, 0.18, 1.0, 16.0, 1e4] {
            let back = l_star_to_y(y_to_l_star(y));
            assert!((back - y).abs() <= 1e-4 * y, "{y}: {back}");
        }
        // The limit converts back to a luminance near 4.5e7, still finite.
        let limit = l_star_to_y(L_STAR_UPPER_LIMIT);
        assert!(limit.is_finite() && limit > 1e7, "{limit}");
    }

    #[test]
    fn d65_is_achromatic() {
        // The white the space is built on lands on U*′ = V*′ = 0.
        let [u, v] = xy_to_uv_star_prime(D65_XY);
        assert!(u.abs() < 1e-4 && v.abs() < 1e-4, "{u} {v}");
        // Black has D65's chromaticity, as darktable gives it.
        assert_eq!(d65_xyz_to_xyy([0.0; 3]), [D65_XY[0], D65_XY[1], 0.0]);
    }

    #[test]
    fn xyy_round_trips_through_xyz() {
        let xyz = [0.3, 0.4, 0.2];
        let back = xyy_to_xyz(d65_xyz_to_xyy(xyz));
        for c in 0..3 {
            assert!((back[c] - xyz[c]).abs() < 1e-6, "{back:?}");
        }
        assert_eq!(xyy_to_xyz([0.3, 0.0, 1.0]), [0.0; 3]);
    }
}
