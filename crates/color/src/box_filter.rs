// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2024  darktable developers (ported from src/common/box_filters.cc)
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

//! darktable's box mean (`src/common/box_filters.cc`, `dt_box_mean`) restated on the
//! CPU, in f32 and darktable's order of arithmetic: the reference the kernels' shared
//! `box_filter.slang` is tested against, and each iteration's radius at a pipe's scale
//! (decision 63), which bloom's and soften's setups take. Nothing renders through it.

/// darktable `BOX_ITERATIONS`: the box means bloom and soften run, each along every row,
/// then every column.
pub const BOX_ITERATIONS: u32 = 8;

/// Each iteration's radius for darktable's box mean of `radius` at full resolution,
/// applied [`BOX_ITERATIONS`] times, at `scale` pixels of the pipe per pixel at full
/// resolution (decision 63): radii `r` and `r + 1`, the larger first, whose blur has the
/// export's variance at that scale, `N s² R (R + 1) / 3` pixels² an axis. With
/// `T = N s² R (R + 1)` in f64, `r` is the largest whole number with `N r (r + 1) ≤ T`, and
/// the first `k` iterations take `r + 1`, `k` the nearest whole number to
/// `(T − N r (r + 1)) / (2 (r + 1))`, a half to the larger, at most `N`. Iteration `i`'s
/// row pass and column pass both take its radius. At scale 1 every iteration is `radius`,
/// so an export is darktable's; one radius in all eight, rounded from `s R`, would carry
/// its error eight times.
pub fn iterated_radii(radius: u32, scale: f32) -> [u32; BOX_ITERATIONS as usize] {
    let n = f64::from(BOX_ITERATIONS);
    let s = f64::from(scale);
    let full = f64::from(radius);
    let target = n * s * s * full * (full + 1.0);
    let sum = |r: u64| n * (r as f64) * (r as f64 + 1.0);
    // The largest r with N r (r + 1) ≤ T, from the quadratic's root, then stepped
    // exactly, so that the root's rounding cannot leave it one off.
    let mut r = (((1.0 + 4.0 * target / n).sqrt() - 1.0) / 2.0).max(0.0) as u64;
    while sum(r + 1) <= target {
        r += 1;
    }
    while r > 0 && sum(r) > target {
        r -= 1;
    }
    let step = 2.0 * (r as f64 + 1.0);
    let k = (((target - sum(r)) / step + 0.5).floor() as u64).min(u64::from(BOX_ITERATIONS));
    std::array::from_fn(|i| (r + u64::from((i as u64) < k)) as u32)
}

/// darktable `_blur_horizontal` and `_blur_vertical` on one line, without Kahan
/// summation: each value becomes the mean of the values within `radius` of it that lie
/// inside the line, kept as a running sum. The left half of the window is added first;
/// at each position the value entering is added, in the bulk after the value leaving is
/// subtracted, and the sum over the values in the window stored.
pub fn box_mean_line(line: &[f32], radius: usize) -> Vec<f32> {
    let count = line.len();
    let mut out = vec![0.0; count];
    let mut sum = 0.0_f32;
    let mut hits = 0_usize;
    for &value in &line[..radius.min(count)] {
        hits += 1;
        sum += value;
    }
    let mut x = 0;
    // Up to where values start leaving the window.
    while x <= radius && x + radius < count {
        hits += 1;
        sum += line[x + radius];
        out[x] = sum / hits as f32;
        x += 1;
    }
    // A window wider than half the line: nothing enters or leaves.
    while x <= radius && x < count {
        out[x] = sum / hits as f32;
        x += 1;
    }
    // The bulk of the line.
    while x + radius < count {
        sum -= line[x - radius - 1];
        sum += line[x + radius];
        out[x] = sum / hits as f32;
        x += 1;
    }
    // The right end, where values only leave.
    while x < count {
        hits -= 1;
        sum -= line[x - radius - 1];
        out[x] = sum / hits as f32;
        x += 1;
    }
    out
}

/// darktable `dt_box_mean` on one channel of a `width` × `height` image, rows top to
/// bottom, in place: `iterations` times a box mean along every row, then along every
/// column. darktable's channels are independent, so a four-channel image is four of
/// these.
pub fn box_mean(image: &mut [f32], width: usize, height: usize, radius: usize, iterations: u32) {
    assert_eq!(image.len(), width * height, "a {width}×{height} image");
    for _ in 0..iterations {
        for row in image.chunks_exact_mut(width) {
            let blurred = box_mean_line(row, radius);
            row.copy_from_slice(&blurred);
        }
        for x in 0..width {
            let column: Vec<f32> = (0..height).map(|y| image[y * width + x]).collect();
            for (y, value) in box_mean_line(&column, radius).into_iter().enumerate() {
                image[y * width + x] = value;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Radii written as runs, (radius, count), larger first.
    fn runs(runs: &[(u32, usize)]) -> [u32; BOX_ITERATIONS as usize] {
        let radii: Vec<u32> = runs
            .iter()
            .flat_map(|&(radius, count)| std::iter::repeat_n(radius, count))
            .collect();
        radii.try_into().unwrap()
    }

    /// mire1 fitted to 2400 wide, 1/2, 1/4 and 1/8, each the fitted width over 3908 in
    /// f32, as the planner gives the scale.
    const SCALES: [f32; 4] = [
        2400.0 / 3908.0,
        1954.0 / 3908.0,
        976.0 / 3908.0,
        488.0 / 3908.0,
    ];

    #[test]
    fn the_radii_are_decision_119s_worked_values() {
        // Soften's test 0051, radius 15.
        let soften = [
            runs(&[(9, 8)]),
            runs(&[(8, 2), (7, 6)]),
            runs(&[(4, 3), (3, 5)]),
            runs(&[(2, 3), (1, 5)]),
        ];
        // Bloom's test 0050, radius 104.
        let bloom = [
            runs(&[(64, 5), (63, 3)]),
            runs(&[(52, 6), (51, 2)]),
            runs(&[(26, 5), (25, 3)]),
            runs(&[(13, 4), (12, 4)]),
        ];
        for (i, &scale) in SCALES.iter().enumerate() {
            assert_eq!(iterated_radii(15, scale), soften[i], "15 at {scale}");
            assert_eq!(iterated_radii(104, scale), bloom[i], "104 at {scale}");
        }
        // Bloom's default, 53, at 1/8; its smallest, 1, at a half; none.
        assert_eq!(iterated_radii(53, SCALES[3]), runs(&[(7, 2), (6, 6)]));
        assert_eq!(iterated_radii(1, 0.5), runs(&[(1, 2), (0, 6)]));
        assert_eq!(iterated_radii(0, 0.5), [0; 8]);
        assert_eq!(iterated_radii(0, 1.0), [0; 8]);
    }

    #[test]
    fn at_scale_1_every_iteration_is_darktables_radius() {
        for radius in 0..=2048 {
            assert_eq!(iterated_radii(radius, 1.0), [radius; 8], "{radius}");
        }
    }

    #[test]
    fn the_radii_hold_the_variance_within_half_a_step() {
        // Each radius r or r + 1, the larger first, and the sum of r_i (r_i + 1) within
        // half of one step, 2 (r + 1), of s² R (R + 1) for each of the eight.
        let n = f64::from(BOX_ITERATIONS);
        for radius in 1..=256_u32 {
            for step in 1..=50 {
                let scale = step as f32 / 50.0;
                let radii = iterated_radii(radius, scale);
                let r = radii[7];
                assert!(radii.iter().all(|&ri| ri == r || ri == r + 1), "{radii:?}");
                assert!(radii.windows(2).all(|w| w[0] >= w[1]), "{radii:?}");
                assert!(radii[0] <= radius, "{radius} at {scale}: {radii:?}");
                let target = n
                    * f64::from(scale)
                    * f64::from(scale)
                    * f64::from(radius)
                    * (f64::from(radius) + 1.0);
                let sum: f64 = radii
                    .iter()
                    .map(|&ri| f64::from(ri) * (f64::from(ri) + 1.0))
                    .sum();
                let half_step = f64::from(r) + 1.0;
                assert!(
                    (sum - target).abs() <= half_step + 1e-9,
                    "{radius} at {scale}: {radii:?}, {sum} against {target}"
                );
            }
        }
    }

    /// The mean over each value's window clipped to the line, in f64.
    fn direct(line: &[f32], radius: usize) -> Vec<f64> {
        (0..line.len())
            .map(|x| {
                let window = &line[x.saturating_sub(radius)..(x + radius + 1).min(line.len())];
                window.iter().map(|&v| f64::from(v)).sum::<f64>() / window.len() as f64
            })
            .collect()
    }

    fn line(count: usize) -> Vec<f32> {
        (0..count)
            .map(|i| {
                let i = i as f32;
                40.0 + 30.0 * (i * 0.37).sin() + if i as usize % 11 < 3 { 25.0 } else { 0.0 }
            })
            .collect()
    }

    #[test]
    fn each_value_is_the_mean_over_its_clipped_window() {
        for count in [1, 2, 7, 64, 301] {
            let values = line(count);
            for radius in [0, 1, 3, 10, 150, 300, 400] {
                let ours = box_mean_line(&values, radius);
                for (x, (&a, b)) in ours.iter().zip(direct(&values, radius)).enumerate() {
                    let gap = (f64::from(a) - b).abs() / b.abs();
                    assert!(gap < 1e-5, "{count} values, radius {radius}, at {x}: {gap}");
                }
            }
        }
    }

    #[test]
    fn a_constant_line_stays_constant() {
        let values = vec![0.75; 50];
        for radius in [0, 1, 5, 24, 25, 60] {
            assert_eq!(box_mean_line(&values, radius), values, "radius {radius}");
        }
    }

    #[test]
    fn a_radius_of_zero_copies_the_line() {
        let values = line(97);
        assert_eq!(box_mean_line(&values, 0), values);
        let mut image = line(12 * 9);
        let copy = image.clone();
        box_mean(&mut image, 12, 9, 0, BOX_ITERATIONS);
        assert_eq!(image, copy);
    }

    #[test]
    fn box_mean_blurs_rows_then_columns() {
        // One bright value in a dark image spreads into a square, symmetric about it.
        let (width, height) = (15, 11);
        let mut image = vec![0.0; width * height];
        image[5 * width + 7] = 1.0;
        box_mean(&mut image, width, height, 1, 1);
        let at = |x: usize, y: usize| image[y * width + x];
        assert!((at(7, 5) - 1.0 / 9.0).abs() < 1e-7);
        assert_eq!(at(6, 4), at(8, 6));
        assert_eq!(at(5, 5), 0.0);
        // Its total is kept away from the edges.
        let total: f32 = image.iter().sum();
        assert!((total - 1.0).abs() < 1e-6, "{total}");
    }
}
