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

//! CIEDE2000 colour difference between linear Rec2020 images, or images in another
//! encoding with [`compare_with`].
//!
//! Pixels are converted from linear Rec2020 to CIELAB under D65, with 1.0 as reference
//! white, so L = 100 at scene white and scene values above 1.0 give L above 100. sRGB
//! and linear Rec709 have their own conversions, for the whole-pipeline test.

use anyhow::{Result, ensure};

use crate::pfm::Image;

/// Linear Rec2020 to CIE XYZ, D65 white.
const REC2020_TO_XYZ: [[f64; 3]; 3] = [
    [0.636_958_0, 0.144_616_9, 0.168_881_0],
    [0.262_700_2, 0.677_998_1, 0.059_301_7],
    [0.000_000_0, 0.028_072_7, 1.060_985_1],
];

pub fn rec2020_to_lab(rgb: [f32; 3]) -> [f64; 3] {
    let rgb = rgb.map(f64::from);
    let xyz = REC2020_TO_XYZ.map(|row| row[0] * rgb[0] + row[1] * rgb[1] + row[2] * rgb[2]);
    // Rec2020 white (1, 1, 1) is D65, so the matrix row sums are the white point.
    let white = REC2020_TO_XYZ.map(|row| row[0] + row[1] + row[2]);
    let [fx, fy, fz] = [0, 1, 2].map(|i| lab_f(xyz[i] / white[i]));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// sRGB, encoded with its transfer function, to CIELAB under D65, white at 1.
pub fn srgb_to_lab(rgb: [f32; 3]) -> [f64; 3] {
    let linear = rgb.map(|c| {
        let c = f64::from(c);
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    linear_rec709_to_lab(linear)
}

/// Linear Rec709, sRGB's primaries without its transfer function, to CIELAB under D65,
/// white at 1.
pub fn rec709_to_lab(rgb: [f32; 3]) -> [f64; 3] {
    linear_rec709_to_lab(rgb.map(f64::from))
}

fn linear_rec709_to_lab(rgb: [f64; 3]) -> [f64; 3] {
    const REC709_TO_XYZ: [[f64; 3]; 3] = [
        [0.412_456_4, 0.357_576_1, 0.180_437_5],
        [0.212_672_9, 0.715_152_2, 0.072_175_0],
        [0.019_333_9, 0.119_192_0, 0.950_304_1],
    ];
    let xyz = REC709_TO_XYZ.map(|row| row[0] * rgb[0] + row[1] * rgb[1] + row[2] * rgb[2]);
    let white = REC709_TO_XYZ.map(|row| row[0] + row[1] + row[2]);
    let [fx, fy, fz] = [0, 1, 2].map(|i| lab_f(xyz[i] / white[i]));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// CIELAB companding. The linear segment also keeps negative (out-of-gamut) values finite.
fn lab_f(t: f64) -> f64 {
    const DELTA: f64 = 6.0 / 29.0;
    if t > DELTA * DELTA * DELTA {
        t.cbrt()
    } else {
        t / (3.0 * DELTA * DELTA) + 4.0 / 29.0
    }
}

/// CIEDE2000 with kL = kC = kH = 1, following Sharma, Wu and Dalal (2005).
pub fn ciede2000(lab1: [f64; 3], lab2: [f64; 3]) -> f64 {
    let [l1, a1, b1] = lab1;
    let [l2, a2, b2] = lab2;
    let pow7 = |x: f64| x.powi(7);
    let twenty_five_7 = pow7(25.0);

    let c_bar = (a1.hypot(b1) + a2.hypot(b2)) / 2.0;
    let g = 0.5 * (1.0 - (pow7(c_bar) / (pow7(c_bar) + twenty_five_7)).sqrt());
    let a1p = (1.0 + g) * a1;
    let a2p = (1.0 + g) * a2;
    let c1p = a1p.hypot(b1);
    let c2p = a2p.hypot(b2);
    let hue = |b: f64, a: f64| {
        if a == 0.0 && b == 0.0 {
            0.0
        } else {
            b.atan2(a).to_degrees().rem_euclid(360.0)
        }
    };
    let h1p = hue(b1, a1p);
    let h2p = hue(b2, a2p);

    let dl = l2 - l1;
    let dc = c2p - c1p;
    let chroma_product = c1p * c2p;
    let dh = if chroma_product == 0.0 {
        0.0
    } else {
        let d = h2p - h1p;
        if d > 180.0 {
            d - 360.0
        } else if d < -180.0 {
            d + 360.0
        } else {
            d
        }
    };
    let dh_big = 2.0 * chroma_product.sqrt() * (dh / 2.0).to_radians().sin();

    let l_bar = (l1 + l2) / 2.0;
    let c_bar_p = (c1p + c2p) / 2.0;
    let h_bar = if chroma_product == 0.0 {
        h1p + h2p
    } else if (h1p - h2p).abs() <= 180.0 {
        (h1p + h2p) / 2.0
    } else if h1p + h2p < 360.0 {
        (h1p + h2p + 360.0) / 2.0
    } else {
        (h1p + h2p - 360.0) / 2.0
    };

    let cos_deg = |d: f64| d.to_radians().cos();
    let t = 1.0 - 0.17 * cos_deg(h_bar - 30.0)
        + 0.24 * cos_deg(2.0 * h_bar)
        + 0.32 * cos_deg(3.0 * h_bar + 6.0)
        - 0.20 * cos_deg(4.0 * h_bar - 63.0);
    let d_theta = 30.0 * (-((h_bar - 275.0) / 25.0).powi(2)).exp();
    let r_c = 2.0 * (pow7(c_bar_p) / (pow7(c_bar_p) + twenty_five_7)).sqrt();
    let l_offset = (l_bar - 50.0).powi(2);
    let s_l = 1.0 + 0.015 * l_offset / (20.0 + l_offset).sqrt();
    let s_c = 1.0 + 0.045 * c_bar_p;
    let s_h = 1.0 + 0.015 * c_bar_p * t;
    let r_t = -(2.0 * d_theta).to_radians().sin() * r_c;

    let (tl, tc, th) = (dl / s_l, dc / s_c, dh_big / s_h);
    (tl * tl + tc * tc + th * th + r_t * tc * th).sqrt()
}

/// Pass thresholds for one comparison.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tolerance {
    pub mean: f64,
    pub max: f64,
}

impl Tolerance {
    /// The thresholds darktable's own integration tests use.
    pub const DARKTABLE: Self = Self {
        mean: 2.3 / 3.0,
        max: 2.3,
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stats {
    pub mean: f64,
    pub p95: f64,
    pub p99: f64,
    pub max: f64,
    /// Pixels whose difference exceeds `Tolerance::max`.
    pub over_max: usize,
}

impl Stats {
    pub fn passes(&self, tolerance: Tolerance) -> bool {
        self.mean <= tolerance.mean && self.max <= tolerance.max
    }
}

/// Per-pixel CIEDE2000 between two linear Rec2020 images of the same size, plus summary
/// statistics.
pub fn compare(
    expected: &Image,
    actual: &Image,
    tolerance: Tolerance,
) -> Result<(Vec<f32>, Stats)> {
    compare_with(expected, actual, tolerance, rec2020_to_lab)
}

/// [`compare`] for images in another encoding, `to_lab` turning their pixels into CIELAB.
pub fn compare_with(
    expected: &Image,
    actual: &Image,
    tolerance: Tolerance,
    to_lab: fn([f32; 3]) -> [f64; 3],
) -> Result<(Vec<f32>, Stats)> {
    ensure!(
        (expected.width, expected.height) == (actual.width, actual.height),
        "size mismatch: expected {}×{}, actual {}×{}",
        expected.width,
        expected.height,
        actual.width,
        actual.height
    );
    ensure!(!expected.pixels.is_empty(), "empty image");

    let differences: Vec<f32> = expected
        .pixels
        .iter()
        .zip(&actual.pixels)
        .map(|(e, a)| ciede2000(to_lab(*e), to_lab(*a)) as f32)
        .collect();

    let mean = differences.iter().map(|&d| f64::from(d)).sum::<f64>() / differences.len() as f64;
    let mut sorted = differences.clone();
    let mut percentile = |share: usize| {
        let index = (sorted.len() - 1) * share / 100;
        let (_, value, _) = sorted.select_nth_unstable_by(index, f32::total_cmp);
        f64::from(*value)
    };
    let p95 = percentile(95);
    let p99 = percentile(99);
    let max = f64::from(differences.iter().copied().fold(0.0, f32::max));
    let over_max = differences
        .iter()
        .filter(|&&d| f64::from(d) > tolerance.max)
        .count();

    Ok((
        differences,
        Stats {
            mean,
            p95,
            p99,
            max,
            over_max,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test pairs from Sharma, Wu and Dalal, "The CIEDE2000 color-difference formula" (2005).
    #[test]
    fn matches_sharma_test_data() {
        let pairs = [
            ([50.0, 2.6772, -79.7751], [50.0, 0.0, -82.7485], 2.0425),
            ([50.0, 0.0, 0.0], [50.0, -1.0, 2.0], 2.3669),
            ([50.0, 2.49, -0.001], [50.0, -2.49, 0.0009], 7.1792),
            ([50.0, 2.5, 0.0], [73.0, 25.0, -18.0], 27.1492),
            (
                [60.2574, -34.0099, 36.2677],
                [60.4626, -34.1751, 39.4387],
                1.2644,
            ),
        ];
        for (lab1, lab2, expected) in pairs {
            let d = ciede2000(lab1, lab2);
            assert!(
                (d - expected).abs() < 1e-4,
                "{lab1:?} {lab2:?}: {d} != {expected}"
            );
            assert!(
                (ciede2000(lab2, lab1) - expected).abs() < 1e-4,
                "not symmetric"
            );
        }
    }

    #[test]
    fn white_is_neutral_l100() {
        let [l, a, b] = rec2020_to_lab([1.0, 1.0, 1.0]);
        assert!((l - 100.0).abs() < 1e-9 && a.abs() < 1e-9 && b.abs() < 1e-9);
    }

    fn assert_lab(lab: [f64; 3], expected: [f64; 3], tolerance: f64) {
        assert!(
            lab.iter()
                .zip(expected)
                .all(|(v, e)| (v - e).abs() <= tolerance),
            "{lab:?} != {expected:?}"
        );
    }

    #[test]
    fn srgb_white_black_and_known_values() {
        assert_lab(srgb_to_lab([1.0; 3]), [100.0, 0.0, 0.0], 1e-9);
        assert_lab(srgb_to_lab([0.0; 3]), [0.0, 0.0, 0.0], 1e-9);
        // Mid-grey: 0.5 encoded is 0.21404 linear, L* 53.389.
        assert_lab(srgb_to_lab([0.5; 3]), [53.389, 0.0, 0.0], 1e-3);
        // sRGB's red primary under D65, as colour references list it.
        assert_lab(srgb_to_lab([1.0, 0.0, 0.0]), [53.241, 80.092, 67.203], 1e-2);
    }

    #[test]
    fn linear_rec709_is_srgb_without_its_curve() {
        let encoded = [0.5, 0.2, 0.9];
        let linear = encoded.map(|c: f32| ((c + 0.055) / 1.055).powf(2.4));
        assert_lab(rec709_to_lab(linear), srgb_to_lab(encoded), 1e-4);
        assert_lab(rec709_to_lab([1.0; 3]), [100.0, 0.0, 0.0], 1e-9);
    }

    #[test]
    fn negative_values_stay_finite() {
        let lab = rec2020_to_lab([-0.1, 0.2, -0.05]);
        assert!(lab.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn identical_images_have_zero_difference() {
        let image = Image::new(2, 1, vec![[0.2, 0.4, 0.6], [1.5, 0.0, -0.1]]).unwrap();
        let (_, stats) = compare(&image, &image, Tolerance::DARKTABLE).unwrap();
        assert_eq!(stats.max, 0.0);
        assert!(stats.passes(Tolerance::DARKTABLE));
    }
}
