// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2022-2026  darktable developers (ported from src/iop/colorequal.c)
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

//! The colour equalizer's curves (darktable `src/iop/colorequal.c`: `_get_hue_node`,
//! `_periodic_RBF_interpolate`, and `pseudo_solve` of `src/iop/choleski.h`): eight
//! nodes, red to magenta, 45° apart in darktable UCS hue, joined by a periodic
//! interpolation of radial basis functions, sampled as a table over the hue; and the
//! colours darktable paints its graph in (`_build_dt_UCS_HSB_gradients`).
//!
//! Shared by the operation's setup (`apertrust-op-colorequal`), which hands the kernel
//! the three tables, and by the Colour equalizer tool's hue curve graph
//! (`ProcessingTool::curve`, decision 31), which draws the curve the kernel renders
//! (decision 65). It runs on the CPU, to set the kernel up and to draw, never on a pixel.

use std::f32::consts::PI;

use crate::{GAMUT_LUT_SIZE, Mat3, ucs};

/// darktable `NODES`: the nodes of each curve, red to magenta.
pub const NODES: usize = 8;
/// darktable `ANGLE_SHIFT`: red sits at 20° of hue in darktable UCS, so the nodes are
/// offset to put it at the curves' origin.
const ANGLE_SHIFT: f32 = 20.0;

/// darktable `SLIDER_BRIGHTNESS`: the brightness, in darktable UCS HSB, its sliders and
/// graph paint the hues at.
pub const SLIDER_BRIGHTNESS: f32 = 0.65;

/// darktable `_conventional_hue_deg_to_ucs_rad`: a hue in degrees from red, as the
/// sliders and the graph show hues, as darktable UCS hue, in radians.
pub fn ucs_hue(degrees: f32) -> f32 {
    crate::deg_to_rad(degrees + ANGLE_SHIFT)
}

/// darktable `_get_hue_node`: the k-th node's hue in darktable UCS, in radians, with the
/// node placement.
pub fn hue_node(k: usize, hue_shift: f32) -> f32 {
    ucs_hue(k as f32 * 360.0 / NODES as f32 + hue_shift)
}

/// darktable `pseudo_solve` (`src/iop/choleski.h`) for a square system: `A x = y` by the
/// normal equations `A'A x = A'y`, Choleski-Banachiewicz on their lower triangle, then
/// the triangular descent and ascent, in f32 and darktable's order. `None` where
/// darktable's solve fails (`A'A` not positive definite), where darktable leaves `y`
/// unsolved.
pub fn pseudo_solve(a: &[[f32; NODES]; NODES], y: &[f32; NODES]) -> Option<[f32; NODES]> {
    const N: usize = NODES;
    let mut a_square = [[0.0f32; N]; N];
    for i in 0..N {
        for j in 0..=i {
            let mut sum = 0.0f32;
            for row in a {
                sum += row[i] * row[j];
            }
            a_square[i][j] = sum;
        }
    }
    let mut y_square = [0.0f32; N];
    for (i, value) in y_square.iter_mut().enumerate() {
        let mut sum = 0.0f32;
        for (row, &y) in a.iter().zip(y) {
            sum += row[i] * y;
        }
        *value = sum;
    }

    // _choleski_decompose
    if a_square[0][0] <= 0.0 {
        return None;
    }
    let mut l = [[0.0f32; N]; N];
    for i in 0..N {
        for j in 0..=i {
            let mut sum = 0.0f32;
            for (a, b) in l[i][..j].iter().zip(&l[j][..j]) {
                sum += a * b;
            }
            if i == j {
                let temp = a_square[i][i] - sum;
                if temp < 0.0 {
                    return None;
                }
                l[i][j] = (a_square[i][i] - sum).sqrt();
            } else {
                let temp = l[j][j];
                if temp == 0.0 {
                    return None;
                }
                l[i][j] = (a_square[i][j] - sum) / temp;
            }
        }
    }
    // _triangular_descent: L b = A'y.
    let mut b = [0.0f32; N];
    for i in 0..N {
        let mut sum = y_square[i];
        for j in 0..i {
            sum -= l[i][j] * b[j];
        }
        if l[i][i] == 0.0 {
            return None;
        }
        b[i] = sum / l[i][i];
    }
    // _triangular_ascent: L' x = b.
    let mut x = [0.0f32; N];
    for i in (0..N).rev() {
        let mut sum = b[i];
        for j in (i + 1..N).rev() {
            sum -= l[j][i] * x[j];
        }
        if l[i][i] == 0.0 {
            return None;
        }
        x[i] = sum / l[i][i];
    }
    Some(x)
}

/// darktable `_cosine_coeffs`.
fn cosine_coeff(l: f32, c: f32) -> f32 {
    (-l * l / c).exp()
}

/// The radial basis of `_periodic_RBF_interpolate` at angular distance `distance`:
/// `exp(Σ_l exp(-l²/c) · cos(l · |distance|))` over its `m` terms.
fn basis(distance: f32, smoothing: f32, terms: i32) -> f32 {
    let mut sum = 0.0f32;
    for l in 0..terms {
        sum += cosine_coeff(l as f32, smoothing) * (l as f32 * distance.abs()).cos();
    }
    sum.exp()
}

/// darktable `_periodic_RBF_interpolate`: the curve through `nodes` at the nodes' hues,
/// a periodic interpolation by radial basis functions, sampled every 360/512 degrees from
/// -π, the table [`crate::GAMUT_LUT_SIZE`] entries long, clipped at 0 with `clip`. In f32 as
/// darktable; where darktable's solve fails, the nodes themselves weigh the bases, as
/// darktable leaves them.
pub fn periodic_rbf(nodes: &[f32; NODES], smoothing: f32, hue_shift: f32, clip: bool) -> Vec<f32> {
    let terms = (3.0 * smoothing.sqrt()).ceil() as i32;
    let mut a = [[0.0f32; NODES]; NODES];
    for (i, row) in a.iter_mut().enumerate() {
        for (j, value) in row.iter_mut().enumerate() {
            *value = basis(
                hue_node(i, hue_shift) - hue_node(j, hue_shift),
                smoothing,
                terms,
            );
        }
    }
    let weights = pseudo_solve(&a, nodes).unwrap_or(*nodes);
    (0..GAMUT_LUT_SIZE)
        .map(|i| {
            let hue = crate::deg_to_rad(i as f32 * 360.0 / GAMUT_LUT_SIZE as f32) - PI;
            let mut value = 0.0f32;
            for (k, &weight) in weights.iter().enumerate() {
                value += weight * basis(hue - hue_node(k, hue_shift), smoothing, terms);
            }
            if clip { value.max(0.0) } else { value }
        })
        .collect()
}

/// darktable's linear Rec. 709 to XYZ D65, which `gui_init` builds the gamut's table from
/// when it has no display profile.
const REC709_TO_XYZ_D65: Mat3 = [
    [0.4124564, 0.3575761, 0.1804375],
    [0.2126729, 0.7151522, 0.0721750],
    [0.0193339, 0.1191920, 0.9503041],
];

/// darktable `lookup_gamut` (`src/common/darktable_ucs_22_helpers.h`): the table `lut`,
/// its length a power of two, at `hue` in radians from -π, linearly interpolated and
/// wrapping around the hue circle.
pub fn lookup_gamut(lut: &[f32], hue: f32) -> f32 {
    let mask = lut.len() as i32 - 1;
    let x = lut.len() as f32 * (hue + PI) / (2.0 * PI);
    let (prev, next) = (x.floor(), x.ceil());
    let i = (prev as i32 & mask) as usize;
    let j = (next as i32 & mask) as usize;
    if i == j {
        lut[i]
    } else {
        lut[i] + (x - prev) * (lut[j] - lut[i])
    }
}

/// darktable `soft_clip`: `x` as it is up to `soft`, then easing exponentially towards
/// `hard`.
fn soft_clip(x: f32, soft: f32, hard: f32) -> f32 {
    let norm = hard - soft;
    if x > soft {
        soft + (1.0 - (-(x - soft) / norm).exp()) * norm
    } else {
        x
    }
}

/// The chroma of the gamut's boundary, for the colourfulness squared `m2` its table
/// gives, at lightness `j`.
fn boundary_chroma(j: f32, m2: f32, l_white: f32) -> f32 {
    15.932993652962535 * (j * l_white).powf(0.6523997524738018) * m2.powf(0.6007557017508491)
        / l_white
}

/// darktable `gamut_map_HSB`: `hsb`'s saturation soft-clipped at constant brightness
/// into the gamut whose table is `lut`.
pub fn gamut_map_hsb(hsb: [f32; 3], lut: &[f32], l_white: f32) -> [f32; 3] {
    let jch = ucs::hsb_to_jch(hsb);
    let chroma = boundary_chroma(jch[0], lookup_gamut(lut, jch[2]), l_white);
    let boundary = ucs::jch_to_hsb([jch[0], chroma, jch[2]]);
    [
        hsb[0],
        soft_clip(hsb[1], 0.8 * boundary[1], boundary[1]),
        hsb[2],
    ]
}

/// darktable `get_minimum_saturation`: the saturation at `lightness` that every hue of
/// the gamut whose table is `lut` holds.
pub fn minimum_saturation(lut: &[f32], lightness: f32, l_white: f32) -> f32 {
    let m2 = lut.iter().copied().fold(f32::MAX, f32::min);
    ucs::jch_to_hsb([lightness, boundary_chroma(lightness, m2, l_white), 0.0])[1]
}

/// The colours darktable paints the colour equalizer's graph and sliders in
/// (`_build_dt_UCS_HSB_gradients`), on sRGB, darktable's fallback without a display
/// profile (`dt_XYZ_to_sRGB`).
#[derive(Debug, Clone)]
pub struct Gradients {
    /// The sRGB gamut's table, `dt_UCS_22_build_gamut_LUT`'s.
    lut: Vec<f32>,
    /// The saturation every hue holds at lightness 0.2, darktable's `max_saturation`,
    /// at which the graph paints its hues.
    pub max_saturation: f32,
}

impl Gradients {
    /// darktable `gui_init`'s, on linear Rec. 709 at D65.
    pub fn srgb() -> Self {
        let lut = ucs::gamut_lut(&REC709_TO_XYZ_D65);
        let max_saturation = minimum_saturation(&lut, 0.2, 1.0);
        Self {
            lut,
            max_saturation,
        }
    }

    /// darktable `_build_dt_UCS_HSB_gradients`: `hsb`, its hue in darktable UCS
    /// radians, mapped into the gamut, as encoded sRGB, each channel within 0 to 1.
    pub fn colour(&self, hsb: [f32; 3]) -> [f32; 3] {
        let hsb = gamut_map_hsb(hsb, &self.lut, 1.0);
        let xyz = ucs::xyy_to_xyz(ucs::jch_to_xyy(ucs::hsb_to_jch(hsb), 1.0));
        let xyz_d50 = crate::apply(&crate::XYZ_D65_TO_D50_CAT16, xyz);
        crate::xyz_d50_to_srgb(xyz_d50).map(|c| c.clamp(0.0, 1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_solve_solves() {
        let mut a = [[0.0f32; NODES]; NODES];
        for (i, row) in a.iter_mut().enumerate() {
            for (j, value) in row.iter_mut().enumerate() {
                *value = basis(hue_node(i, 3.0) - hue_node(j, 3.0), PI, 6);
            }
        }
        let y = [1.0, 2.0, -1.0, 0.5, 0.0, 3.0, 1.0, -2.0];
        let x = pseudo_solve(&a, &y).unwrap();
        for i in 0..NODES {
            let ax: f32 = (0..NODES).map(|j| a[i][j] * x[j]).sum();
            assert!((ax - y[i]).abs() < 1e-3, "{i}: {ax}");
        }
        let singular = [[1.0f32; NODES]; NODES];
        assert_eq!(pseudo_solve(&singular, &y), None);
    }

    #[test]
    fn the_gamut_table_is_looked_up_between_its_entries_and_around_the_circle() {
        let lut: Vec<f32> = (0..8).map(|i| i as f32).collect();
        let step = 2.0 * PI / 8.0;
        assert_eq!(lookup_gamut(&lut, -PI), 0.0);
        assert!((lookup_gamut(&lut, -PI + 2.5 * step) - 2.5).abs() < 1e-5);
        // Past the last entry it runs back to the first.
        assert!((lookup_gamut(&lut, -PI + 7.5 * step) - 3.5).abs() < 1e-5);
    }

    #[test]
    fn the_graphs_colours_are_greys_without_saturation_and_hues_with_it() {
        let gradients = Gradients::srgb();
        let s = gradients.max_saturation;
        assert!(s > 0.05 && s < 1.0, "{s}");
        for degrees in [0.0, 90.0, 200.0, 300.0] {
            let [r, g, b] = gradients.colour([ucs_hue(degrees), 0.0, SLIDER_BRIGHTNESS]);
            assert!(
                (r - g).abs() < 2e-3 && (g - b).abs() < 2e-3,
                "{degrees}: {r} {g} {b}"
            );
        }
        let largest = |degrees: f32| {
            let rgb = gradients.colour([ucs_hue(degrees), s, SLIDER_BRIGHTNESS]);
            assert!(rgb.iter().all(|c| (0.0..=1.0).contains(c)), "{rgb:?}");
            (0..3).max_by(|&a, &b| rgb[a].total_cmp(&rgb[b])).unwrap()
        };
        // Red, green and blue each lead where the sliders show them.
        assert_eq!(largest(0.0), 0);
        assert_eq!(largest(135.0), 1);
        assert_eq!(largest(225.0), 2);
    }

    #[test]
    fn gamut_mapping_keeps_the_hue_and_brightness_and_narrows_only_past_the_boundary() {
        let lut = ucs::gamut_lut(&REC709_TO_XYZ_D65);
        let inside = [ucs_hue(30.0), 0.01, 0.5];
        assert_eq!(gamut_map_hsb(inside, &lut, 1.0), inside);
        let outside = [ucs_hue(30.0), 5.0, 0.5];
        let mapped = gamut_map_hsb(outside, &lut, 1.0);
        assert_eq!([mapped[0], mapped[2]], [outside[0], outside[2]]);
        assert!(mapped[1] < outside[1]);
    }
}
