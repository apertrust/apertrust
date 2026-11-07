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

//! The tone equalizer's correction curve (darktable `src/iop/toneequal.c`: `centers_ops`,
//! `centers_params`, `build_interpolation_matrix`, `pixel_correction`, and `pseudo_solve`
//! of `src/iop/choleski.h`): the gain at a mask's exposure is a sum of eight Gaussians
//! 8/7 EV apart, of width σ, the curve's smoothing, their weights fitted by least squares
//! to the gains of the nine bands at −8 EV to 0 EV, clamped to ¼ to 4.
//!
//! Shared by the operation's setup (`apertrust-op-toneequal`), whose kernel sums the
//! Gaussians with the weights fitted here, and by the Tone equalizer module's band graph
//! (`Module::curve`, decision 31), which draws the curve the kernel renders (decision
//! 65). It runs on the CPU, to set the kernel up and to draw, never on a pixel.

/// The nine bands' centres, in EV of the mask: darktable `centers_params`.
pub const BAND_CENTRES: [f32; 9] = [-8.0, -7.0, -6.0, -5.0, -4.0, -3.0, -2.0, -1.0, 0.0];

/// The eight Gaussians' centres the curve is a sum of, -8 + 8j/7 EV: darktable
/// `centers_ops`, one fewer than the bands, so the curve approximates them rather than
/// interpolates.
pub const CURVE_CENTRES: [f32; 8] = [
    -56.0 / 7.0,
    -48.0 / 7.0,
    -40.0 / 7.0,
    -32.0 / 7.0,
    -24.0 / 7.0,
    -16.0 / 7.0,
    -8.0 / 7.0,
    0.0,
];

/// The mask's exposures the curve covers, EV: darktable `DT_TONEEQ_MIN_EV` and
/// `DT_TONEEQ_MAX_EV`. A mask outside them takes the gain at the nearer end.
pub const EXPOSURES: (f64, f64) = (-8.0, 0.0);

/// The gains the curve is clamped to, ±2 EV: darktable's `fast_clamp(result, 0.25f, 4.0f)`.
pub const GAINS: (f64, f64) = (0.25, 4.0);

/// The curve as the kernel renders it: the Gaussians' weights, and `1 / (2σ²)`, both in
/// f32 as the kernel takes them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Curve {
    pub weights: [f32; 8],
    pub inverse_denominator: f32,
}

impl Curve {
    /// darktable's `commit_params` for the bands' gains in EV, `bands`, at the smoothing
    /// σ, `smoothing`: the linear gains `exp2f(band)` fitted in f64 ([`fit`]) and each
    /// weight rounded once to f32, and darktable's `gaussian_denom`, 2σ², in f32. `None`
    /// where the fit fails, darktable's "the interpolation is unstable".
    pub fn new(bands: &[f32; 9], smoothing: f32) -> Option<Self> {
        let gains = bands.map(|ev| f64::from(ev).exp2());
        let weights = fit(f64::from(smoothing), &gains)?.map(|w| w as f32);
        let denominator = 2.0 * smoothing * smoothing;
        Some(Self {
            weights,
            inverse_denominator: 1.0 / denominator,
        })
    }

    /// The gain for a mask at `exposure` EV, clamped to the curve's exposures and gains,
    /// as the kernel computes it, in f64: darktable `pixel_correction`.
    pub fn gain(&self, exposure: f64) -> f64 {
        let exposure = exposure.clamp(EXPOSURES.0, EXPOSURES.1);
        let curve: f64 = CURVE_CENTRES
            .iter()
            .zip(&self.weights)
            .map(|(&centre, &weight)| {
                let d = exposure - f64::from(centre);
                f64::from(weight) * (-d * d * f64::from(self.inverse_denominator)).exp()
            })
            .sum();
        curve.clamp(GAINS.0, GAINS.1)
    }

    /// The correction in EV for a mask at `exposure` EV: `log2` of [`Self::gain`], from
    /// −2 EV to +2 EV, the curve darktable's graph draws.
    pub fn ev(&self, exposure: f64) -> f64 {
        self.gain(exposure).log2()
    }
}

/// darktable `build_interpolation_matrix()`: `A_ij = G(μ_i - ν_j)` for the bands' and
/// the curve's centres, `G(d) = exp(-d² / 2σ²)`.
pub fn interpolation_matrix(sigma: f64) -> [[f64; 8]; 9] {
    let denominator = 2.0 * sigma * sigma;
    std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            let d = f64::from(BAND_CENTRES[i]) - f64::from(CURVE_CENTRES[j]);
            (-d * d / denominator).exp()
        })
    })
}

/// The eight Gaussians' weights whose sum best fits `values` at the nine bands, least
/// squares: darktable's `pseudo_solve()`, which solves the normal equations by a
/// Cholesky factorisation in f32, here a Householder QR factorisation of the 9 × 8
/// matrix itself in f64. Its condition is the square root of the normal equations',
/// 2.2e5 at the top of the smoothing range, so the fit holds to about 1e-11 there.
///
/// `None` where it fails, as `pseudo_solve` returns false: where the triangle has a zero
/// pivot or a weight is not a number, which a σ of 0 or one that is not a number gives.
/// Over the operation's smoothing range, √2^-1.33 to √2^2.67, it never fails, where
/// darktable's f32 Cholesky does from about σ = 1.98 (`docs/ports/toneequal.md`, Setup).
pub fn fit(sigma: f64, values: &[f64; 9]) -> Option<[f64; 8]> {
    let mut a = interpolation_matrix(sigma);
    let mut b = *values;
    for k in 0..8 {
        let norm = (k..9).map(|i| a[i][k] * a[i][k]).sum::<f64>().sqrt();
        if norm == 0.0 {
            continue;
        }
        let alpha = if a[k][k] > 0.0 { -norm } else { norm };
        let mut v = [0.0; 9];
        for i in k..9 {
            v[i] = a[i][k];
        }
        v[k] -= alpha;
        let vv: f64 = (k..9).map(|i| v[i] * v[i]).sum();
        // H = I - 2vvᵀ/vᵀv on the remaining columns and on the right-hand side: each
        // loses 2(vᵀcolumn)/vᵀv times v.
        let mut factors = [0.0; 8];
        for (j, factor) in factors.iter_mut().enumerate().skip(k) {
            *factor = 2.0 * (k..9).map(|i| v[i] * a[i][j]).sum::<f64>() / vv;
        }
        let factor_b = 2.0 * (k..9).map(|i| v[i] * b[i]).sum::<f64>() / vv;
        for i in k..9 {
            for (value, factor) in a[i][k..].iter_mut().zip(&factors[k..]) {
                *value -= factor * v[i];
            }
            b[i] -= factor_b * v[i];
        }
    }
    // Back substitution through the triangle R.
    let mut x = [0.0; 8];
    for i in (0..8).rev() {
        if a[i][i] == 0.0 {
            return None;
        }
        let sum: f64 = (i + 1..8).map(|j| a[i][j] * x[j]).sum();
        x[i] = (b[i] - sum) / a[i][i];
    }
    x.iter().all(|w| w.is_finite()).then_some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The smoothing slider's ends, `√2^(1 + s)` at s of -2.33 and 1.67, correctly rounded.
    const SMOOTHING: (f32, f32) = (0.6306887, 2.5227547);

    #[test]
    fn the_fit_never_fails_over_the_smoothing_range() {
        let bands = [
            -1.283, -0.788, 0.445, 1.148, 1.321, 0.994, 0.211, -0.449, -0.755,
        ];
        for k in 0..=200 {
            let smoothing = SMOOTHING.0 + (SMOOTHING.1 - SMOOTHING.0) * k as f32 / 200.0;
            for bands in [bands, [0.0; 9], [2.0; 9], [-2.0; 9]] {
                assert!(Curve::new(&bands, smoothing).is_some(), "σ {smoothing}");
            }
        }
    }

    #[test]
    fn the_fit_fails_where_no_curve_solves_it() {
        for sigma in [0.0, f64::NAN, f64::INFINITY] {
            assert_eq!(fit(sigma, &[1.0; 9]), None, "σ {sigma}");
        }
        assert_eq!(fit(1.0, &[f64::NAN; 9]), None);
        assert_eq!(Curve::new(&[0.0; 9], 0.0), None);
    }

    #[test]
    fn the_curve_is_clamped_to_two_stops_either_way_and_to_its_exposures() {
        let up = Curve::new(&[2.0; 9], std::f32::consts::SQRT_2).unwrap();
        let down = Curve::new(&[-2.0; 9], std::f32::consts::SQRT_2).unwrap();
        for k in 0..=80 {
            let exposure = -9.0 + f64::from(k) / 8.0;
            assert!(
                up.ev(exposure) <= 2.0 && down.ev(exposure) >= -2.0,
                "{exposure}"
            );
        }
        let curve = Curve::new(&[0.3, -0.2, 0.1, 0.0, 0.4, 0.0, -0.3, 0.2, 0.1], 1.2).unwrap();
        assert_eq!(curve.gain(-12.0), curve.gain(-8.0));
        assert_eq!(curve.gain(3.0), curve.gain(0.0));
    }
}
