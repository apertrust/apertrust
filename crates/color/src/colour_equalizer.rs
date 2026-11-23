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
//! interpolation of radial basis functions, sampled as a table over the hue.
//!
//! Shared by the operation's setup (`apertrust-op-colorequal`), which hands the kernel
//! the three tables, and by the Colour equalizer tool's hue curve graph
//! (`ProcessingTool::curve`, decision 31), which draws the curve the kernel renders
//! (decision 65). It runs on the CPU, to set the kernel up and to draw, never on a pixel.

use std::f32::consts::PI;

use crate::GAMUT_LUT_SIZE;

/// darktable `NODES`: the nodes of each curve, red to magenta.
pub const NODES: usize = 8;
/// darktable `ANGLE_SHIFT`: red sits at 20° of hue in darktable UCS, so the nodes are
/// offset to put it at the curves' origin.
const ANGLE_SHIFT: f32 = 20.0;

/// darktable `_get_hue_node`: the k-th node's hue in darktable UCS, in radians, with the
/// node placement.
pub fn hue_node(k: usize, hue_shift: f32) -> f32 {
    crate::deg_to_rad(k as f32 * 360.0 / NODES as f32 + hue_shift + ANGLE_SHIFT)
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
}
