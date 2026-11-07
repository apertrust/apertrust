// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2012-2026  darktable developers (ported from src/common/gaussian.c)
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

//! darktable's recursive Gaussian (`src/common/gaussian.c`, `dt_gaussian_blur` and
//! `dt_gaussian_blur_4c` of order 0, and `dt_gaussian_mean_blur` in `gaussian.h`):
//! Deriche's recursion down each column, then along each row. The coefficients are the
//! kernels' (`slang/gaussian.slang` takes them in its uniforms); the blur restated on the
//! CPU, in f32 and darktable's order of arithmetic, is the reference the kernels' line
//! passes are tested against, and nothing renders through it.
//!
//! Shared by the colour equalizer (`apertrust-op-colorequal`), its second user, and the
//! tone equalizer's EIGF mask, whose own copy in `apertrust-op-toneequal` is to move here
//! (decision 65).

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

/// darktable `dt_gaussian_mean_blur`'s bounds: an unbounded signal, every value clamped
/// to ±10⁹ as it is read.
pub const MEAN_BLUR_RANGE: f32 = 1.0e9;

/// darktable `CLAMPF`: `mn` when `a` is below it, even when `mn > mx`.
pub fn clampf(a: f32, mn: f32, mx: f32) -> f32 {
    if a >= mn {
        if a <= mx { a } else { mx }
    } else {
        mn
    }
}

/// darktable `dt_gaussian_blur()` (and `dt_gaussian_blur_4c()`, the same arithmetic per
/// channel) of order 0 on `ch` interleaved channels, each clamped to `min` and `max`:
/// Deriche's recursion down each column, then along each row of the result.
pub fn gaussian_blur(
    input: &[f32],
    width: usize,
    height: usize,
    ch: usize,
    min: &[f32],
    max: &[f32],
    sigma: f32,
) -> Vec<f32> {
    assert_eq!(input.len(), width * height * ch, "a {width}×{height} image");
    let d = Deriche::new(sigma);
    let clamp = |v: f32, k: usize| clampf(v, min[k], max[k]);
    let mut temp = vec![0.0f32; input.len()];
    for i in 0..width {
        for k in 0..ch {
            let at = |j: usize| (j * width + i) * ch + k;
            let mut xp = clamp(input[at(0)], k);
            let mut yb = xp * d.coefp;
            let mut yp = yb;
            for j in 0..height {
                let xc = clamp(input[at(j)], k);
                let yc = (d.a0 * xc) + (d.a1 * xp) - (d.b1 * yp) - (d.b2 * yb);
                temp[at(j)] = yc;
                xp = xc;
                yb = yp;
                yp = yc;
            }
            let mut xn = clamp(input[at(height - 1)], k);
            let mut xa = xn;
            let mut yn = xn * d.coefn;
            let mut ya = yn;
            for j in (0..height).rev() {
                let xc = clamp(input[at(j)], k);
                let yc = (d.a2 * xn) + (d.a3 * xa) - (d.b1 * yn) - (d.b2 * ya);
                xa = xn;
                xn = xc;
                ya = yn;
                yn = yc;
                temp[at(j)] += yc;
            }
        }
    }
    let mut out = vec![0.0f32; input.len()];
    for j in 0..height {
        for k in 0..ch {
            let at = |i: usize| (j * width + i) * ch + k;
            let mut xp = clamp(temp[at(0)], k);
            let mut yb = xp * d.coefp;
            let mut yp = yb;
            for i in 0..width {
                let xc = clamp(temp[at(i)], k);
                let yc = (d.a0 * xc) + (d.a1 * xp) - (d.b1 * yp) - (d.b2 * yb);
                out[at(i)] = yc;
                xp = xc;
                yb = yp;
                yp = yc;
            }
            let mut xn = clamp(temp[at(width - 1)], k);
            let mut xa = xn;
            let mut yn = xn * d.coefn;
            let mut ya = yn;
            for i in (0..width).rev() {
                let xc = clamp(temp[at(i)], k);
                let yc = (d.a2 * xn) + (d.a3 * xa) - (d.b1 * yn) - (d.b2 * ya);
                xa = xn;
                xn = xc;
                ya = yn;
                yn = yc;
                out[at(i)] += yc;
            }
        }
    }
    out
}

/// darktable `dt_gaussian_mean_blur()`, in place: [`gaussian_blur`] of an unbounded
/// signal, every channel clamped to ±[`MEAN_BLUR_RANGE`].
pub fn gaussian_mean_blur(image: &mut [f32], width: usize, height: usize, ch: usize, sigma: f32) {
    let max = vec![MEAN_BLUR_RANGE; ch];
    let min = vec![-MEAN_BLUR_RANGE; ch];
    let blurred = gaussian_blur(image, width, height, ch, &min, &max, sigma);
    image.copy_from_slice(&blurred);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_coefficients_sum_to_one_at_dc() {
        // A constant line stays constant: the causal and anticausal halves of the
        // recursion's gain at frequency 0 add up to 1.
        for sigma in [0.2, 0.5, 1.0, 3.125, 20.0] {
            let d = Deriche::new(sigma);
            let gain = (d.a0 + d.a1 + d.a2 + d.a3) / (1.0 + d.b1 + d.b2);
            assert!((gain - 1.0).abs() < 1e-5, "σ {sigma}: {gain}");
            assert_eq!(
                d.coefp + d.coefn,
                (d.a0 + d.a1) / (1.0 + d.b1 + d.b2) + d.coefn
            );
        }
    }

    #[test]
    fn a_constant_image_stays_constant() {
        let (width, height) = (13, 9);
        let mut image = vec![0.75f32; width * height * 2];
        gaussian_mean_blur(&mut image, width, height, 2, 1.5);
        assert!(image.iter().all(|&v| (v - 0.75).abs() < 1e-5), "{image:?}");
    }

    #[test]
    fn an_impulse_spreads_symmetrically_with_unit_mass() {
        let (width, height) = (41, 41);
        let mut image = vec![0.0f32; width * height];
        image[20 * width + 20] = 1.0;
        gaussian_mean_blur(&mut image, width, height, 1, 2.0);
        let total: f32 = image.iter().sum();
        assert!((total - 1.0).abs() < 1e-4, "{total}");
        let at = |x: usize, y: usize| image[y * width + x];
        assert!((at(17, 20) - at(23, 20)).abs() < 1e-6);
        assert!((at(20, 17) - at(20, 23)).abs() < 1e-6);
        assert!((at(17, 20) - at(20, 17)).abs() < 1e-6);
        assert!(at(20, 20) > at(21, 20) && at(21, 20) > at(22, 20));
    }

    #[test]
    fn values_are_clamped_as_they_are_read() {
        // CLAMPF bounds what the recursion sees, darktable's guard against an
        // unbounded value.
        let mut image = vec![0.0f32; 5 * 5];
        image[12] = f32::MAX;
        gaussian_mean_blur(&mut image, 5, 5, 1, 1.0);
        assert!(image.iter().all(|v| v.is_finite() && *v <= MEAN_BLUR_RANGE));
        assert_eq!(clampf(f32::NAN, -1.0, 1.0), -1.0);
        assert_eq!(clampf(4.0, 3.0, 1.0), 1.0);
        assert_eq!(clampf(0.0, 3.0, 1.0), 3.0);
    }
}
