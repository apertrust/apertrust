// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2012-2026  darktable developers (ported from src/common/interpolation.c)
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

//! An export's resample (decision 56; `docs/ports/finalscale.md`): darktable's
//! `dt_interpolation_resample` with its default scaling interpolator, lanczos3. Each axis
//! has a plan, made here in f32 as darktable's `_prepare_resampling_plan` makes it:
//! per output pixel, its taps' input pixels and weights, normalised. The engine's
//! `resample` kernel applies the two plans, across and then down; the planner walks an
//! export's tiles back through them ([`AxisPlan::input_span`]). [`reference()`] restates
//! darktable's loop on the CPU, a test reference only.

use std::ops::Range;

/// lanczos3's half width, darktable's `itor->width`.
const WIDTH: i32 = 3;

/// darktable's `DT_LANCZOS_EPSILON`.
const EPSILON: f32 = 1e-9;

/// darktable's `M_PI_F`.
const PI: f32 = std::f32::consts::PI;

/// One axis's resampling plan: for each output pixel, its taps, each an input pixel and
/// a weight, the weights adding up to 1 in f32 as darktable normalises them.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisPlan {
    /// Per output pixel, where its taps start in `index` and `weight`, and how many.
    pub spans: Vec<[u32; 2]>,
    /// Each tap's input pixel, clipped to the input as darktable's `BORDER_REPLICATE`.
    pub index: Vec<u32>,
    pub weight: Vec<f32>,
}

impl AxisPlan {
    /// darktable's plan of `output` pixels from `input` at `scale` (f32, darktable's
    /// region of interest's), output pixel `x` at input `x / scale`: lanczos3 taps at the
    /// input pixels within its window, normalised by their sum. `scale` must not be 1,
    /// at which darktable copies instead.
    pub fn new(input: u32, output: u32, scale: f32) -> Self {
        assert!(input > 0 && scale > 0.0 && scale != 1.0, "a resample");
        let mut plan = Self {
            spans: Vec::with_capacity(output as usize),
            index: Vec::new(),
            weight: Vec::new(),
        };
        let last = input as i32 - 1;
        for x in 0..output as i32 {
            // `_compute_upsampling_kernel` and `_compute_downsampling_kernel`.
            let (first, taps) = if scale > 1.0 {
                let fx = x as f32 / scale;
                let first = fx.floor() as i32 - WIDTH + 1;
                (
                    first,
                    lanczos_taps(2 * WIDTH as usize, fx - first as f32, -1.0),
                )
            } else {
                let xin = ceil_fast((x as f32 - WIDTH as f32) / scale);
                let t = xin * scale - x as f32;
                let count = ((WIDTH as f32 - t) / scale) as i32;
                (xin as i32, lanczos_taps(count.max(0) as usize, t, scale))
            };
            // `_prepare_resampling_plan`: the inverse of the taps' sum, in their order.
            let norm = 1.0 / taps.iter().fold(0.0f32, |sum, &tap| sum + tap);
            plan.spans
                .push([plan.index.len() as u32, taps.len() as u32]);
            for (k, tap) in taps.into_iter().enumerate() {
                plan.weight.push(tap * norm);
                plan.index.push((first + k as i32).clamp(0, last) as u32);
            }
        }
        plan
    }

    /// The input pixels the output pixels `range` read: the first and one past the
    /// last, darktable's taps clipped to the input as they are.
    pub fn input_span(&self, range: Range<u32>) -> Range<u32> {
        let taps = |x: u32| {
            let [start, count] = self.spans[x as usize];
            &self.index[start as usize..(start + count) as usize]
        };
        let low = range.clone().flat_map(taps).min().copied();
        let high = range.flat_map(taps).max().copied();
        match (low, high) {
            (Some(low), Some(high)) => low..high + 1,
            _ => 0..0,
        }
    }

    /// The plan of output pixels `range` alone, each tap's input pixel counted from
    /// `base`, for a tile that reads its input from there.
    pub fn slice(&self, range: Range<u32>, base: u32) -> Self {
        let mut slice = Self {
            spans: Vec::with_capacity(range.len()),
            index: Vec::new(),
            weight: Vec::new(),
        };
        for x in range {
            let [start, count] = self.spans[x as usize];
            let taps = start as usize..(start + count) as usize;
            slice.spans.push([slice.index.len() as u32, count]);
            slice
                .index
                .extend(self.index[taps.clone()].iter().map(|&i| i - base));
            slice.weight.extend_from_slice(&self.weight[taps]);
        }
        slice
    }

    /// The three tables the kernel binds for this axis: the spans, the taps' input
    /// pixels and their weights, little-endian.
    pub fn tables(&self) -> Vec<Vec<u8>> {
        let words = |words: &mut dyn Iterator<Item = u32>| {
            words.flat_map(u32::to_le_bytes).collect::<Vec<u8>>()
        };
        vec![
            words(&mut self.spans.iter().flatten().copied()),
            words(&mut self.index.iter().copied()),
            words(&mut self.weight.iter().map(|w| w.to_bits())),
        ]
    }
}

/// The input length darktable's pipe gives the resample of `output` pixels at `scale`
/// from an image `image` pixels long: after demosaic, demosaic's `modify_roi_in`,
/// `max(8, (int)(output / scale))`; at finalscale's place, finalscale's,
/// `max(16, min(ceilf(output / scale), image))`. It can be a pixel or two short of the
/// image, whose last pixels darktable's resample then never reads.
pub fn input_length(output: u32, scale: f32, image: u32, finalscale: bool) -> u32 {
    let length = output as f32 / scale;
    if finalscale {
        (length.ceil().min(image as f32) as u32).max(16)
    } else {
        (length as u32).max(8)
    }
}

/// darktable's `_maketaps_lanczos` for lanczos3: `count` taps from `first`, a step of
/// `interval` apart, computed four at a time as darktable's vectors do, each lane adding
/// four steps a round.
fn lanczos_taps(count: usize, first: f32, interval: f32) -> Vec<f32> {
    let width = WIDTH as f32;
    let step = 4.0 * interval;
    let mut lanes = [0, 1, 2, 3].map(|c| first + c as f32 * interval);
    let mut taps = Vec::with_capacity(count.next_multiple_of(4));
    for _ in 0..count.div_ceil(4) {
        for v in lanes {
            let a = v as i32;
            let r = v - a as f32;
            let sign = if a & 1 != 0 { -1.0 } else { 1.0 };
            let sine1 = sin_fast(PI * r);
            let sine2 = sin_fast(PI * v / width);
            let num = width * sign * sine1 * sine2 + EPSILON;
            let denom = PI * PI * v * v + EPSILON;
            taps.push(num / denom);
        }
        for lane in &mut lanes {
            *lane += step;
        }
    }
    taps.truncate(count);
    taps
}

/// darktable's `dt_vector_sin`: a parabola fitted to the sine over −π to π, then
/// sharpened, exact at 0, ±π/2 and ±π.
fn sin_fast(x: f32) -> f32 {
    let a = 4.0 / (PI * PI);
    let p = 0.225;
    let scaled = a * x * (PI - x.abs());
    scaled * (p * (scaled.abs() - 1.0) + 1.0)
}

/// darktable's `ceil_fast`, through `int`, so an integer above 0 rounds up by one.
fn ceil_fast(x: f32) -> f32 {
    if x <= 0.0 {
        x as i32 as f32
    } else {
        -((-x) as i32 as f32) + 1.0
    }
}

/// darktable's `dt_interpolation_resample` on the CPU, a test reference: `input`,
/// `input_size` RGBA pixels, resampled to `output_size` by the two plans, each output
/// pixel adding, for each of its rows' taps in order, that row's sum of its column taps
/// in order.
pub fn reference(
    input: &[[f32; 4]],
    input_size: [u32; 2],
    across: &AxisPlan,
    down: &AxisPlan,
) -> Vec<[f32; 4]> {
    let width = input_size[0] as usize;
    assert_eq!(input.len(), width * input_size[1] as usize);
    let mut output = Vec::with_capacity(across.spans.len() * down.spans.len());
    for &[row_start, rows] in &down.spans {
        for &[column_start, columns] in &across.spans {
            let mut pixel = [0.0f32; 4];
            for k in row_start..row_start + rows {
                let row = down.index[k as usize] as usize * width;
                let mut sum = [0.0f32; 4];
                for j in column_start..column_start + columns {
                    let tap = across.weight[j as usize];
                    let value = input[row + across.index[j as usize] as usize];
                    for c in 0..4 {
                        sum[c] += value[c] * tap;
                    }
                }
                let tap = down.weight[k as usize];
                for c in 0..4 {
                    pixel[c] += sum[c] * tap;
                }
            }
            output.push(pixel);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    /// darktable's lanczos taps, from its reference formula in double: the fast sine is
    /// within about 0.1 % of it, and the window's edges are 0.
    #[test]
    fn the_taps_follow_lanczos3() {
        let exact = |t: f64| {
            if t.abs() < 1e-9 {
                1.0
            } else {
                let pi = std::f64::consts::PI;
                3.0 * (pi * t).sin() * (pi * t / 3.0).sin() / (pi * pi * t * t)
            }
        };
        let taps = lanczos_taps(13, -3.0, 0.5);
        assert_eq!(taps.len(), 13);
        for (k, &tap) in taps.iter().enumerate() {
            let t = -3.0 + 0.5 * k as f64;
            assert!(
                (f64::from(tap) - exact(t)).abs() < 2e-3,
                "{t}: {tap} against {}",
                exact(t)
            );
        }
        assert_eq!(taps[6], 1.0, "1 at the centre");
        assert!(taps[0].abs() < 1e-6 && taps[12].abs() < 1e-6);
    }

    #[test]
    fn the_fast_sine_is_exact_where_darktable_says() {
        for x in [0.0, PI / 2.0, PI, -PI / 2.0, -PI] {
            assert!((sin_fast(x) - x.sin()).abs() < 1e-6, "{x}");
        }
        assert!((sin_fast(1.0) - 1.0f32.sin()).abs() < 2e-3);
    }

    #[test]
    fn ceil_fast_rounds_an_integer_above_0_up() {
        assert_eq!(ceil_fast(2.3), 3.0);
        assert_eq!(ceil_fast(2.0), 3.0);
        assert_eq!(ceil_fast(-2.7), -2.0);
        assert_eq!(ceil_fast(0.0), 0.0);
    }

    /// darktable's plan for the first output pixels of a halving: output 0 at input 0,
    /// its window from −3 / 0.5 = −6 on, the taps clipped to the input's first pixel.
    #[test]
    fn a_halving_takes_twelve_taps_a_pixel_clipped_at_the_edges() {
        let plan = AxisPlan::new(100, 50, 0.5);
        assert_eq!(plan.spans.len(), 50);
        // xin = ceil_fast(−3 / 0.5) = −6, t = −3, (3 − t) / 0.5 = 12 taps.
        assert_eq!(plan.spans[0], [0, 12]);
        assert_eq!(&plan.index[..12], &[0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5]);
        // Output 10 at input 20: xin = ceil_fast(7 / 0.5) = 15, past 14, as darktable's
        // ceil rounds an integer up; t = −2.5, so 5.5 / 0.5 = 11 taps, from 15 to 25.
        let [start, count] = plan.spans[10];
        assert_eq!(count, 11);
        assert_eq!(plan.index[start as usize], 15);
        assert_eq!(plan.index[(start + count - 1) as usize], 25);
        for &[start, count] in &plan.spans {
            let weights = &plan.weight[start as usize..(start + count) as usize];
            let sum: f32 = weights.iter().sum();
            assert!((sum - 1.0).abs() < 1e-5, "{sum}");
        }
        // The last pixels clip at the input's last.
        let [start, count] = plan.spans[49];
        assert_eq!(plan.index[(start + count - 1) as usize], 99);
    }

    #[test]
    fn an_upscale_takes_six_taps_a_pixel() {
        let plan = AxisPlan::new(10, 20, 2.0);
        assert!(plan.spans.iter().all(|&[_, count]| count == 6));
        // Output 5 at input 2.5: from floor(2.5) − 2 = 0.
        let [start, _] = plan.spans[5];
        assert_eq!(
            &plan.index[start as usize..start as usize + 6],
            &[0, 1, 2, 3, 4, 5]
        );
        // Output 0 at input 0: the first tap's weight 0, the centre's 1.
        let [start, _] = plan.spans[0];
        let weights = &plan.weight[start as usize..start as usize + 6];
        assert!(
            weights[0].abs() < 1e-6 && (weights[2] - 1.0).abs() < 1e-6,
            "{weights:?}"
        );
    }

    #[test]
    fn an_input_span_covers_every_tap_of_its_outputs_and_a_slice_rebases_them() {
        let plan = AxisPlan::new(1000, 300, 0.3);
        let span = plan.input_span(100..140);
        let slice = plan.slice(100..140, span.start);
        assert_eq!(slice.spans.len(), 40);
        assert_eq!(slice.index.iter().min(), Some(&0));
        assert_eq!(slice.index.iter().max(), Some(&(span.end - span.start - 1)));
        assert_eq!(
            slice.weight,
            plan.weight
                [plan.spans[100][0] as usize..(plan.spans[139][0] + plan.spans[139][1]) as usize]
        );
        assert_eq!(plan.input_span(0..300), 0..1000);
    }

    /// darktable's input lengths for the 7R V's 9504 × 6336 at 2048 × 1365, scale
    /// 2048 / 9504: the last rows are never read.
    #[test]
    fn the_input_is_the_region_darktable_s_pipe_gives_the_resample() {
        let scale = (2048.0f64 / 9504.0) as f32;
        assert_eq!(input_length(2048, scale, 9504, false), 9504);
        assert_eq!(input_length(1365, scale, 6336, false), 6334);
        assert_eq!(input_length(1365, scale, 6336, true), 6335);
        assert_eq!(input_length(2000, 2.0, 1000, true), 1000);
        assert_eq!(input_length(4, 0.5, 8, true), 16, "at least 16");
    }

    #[test]
    fn the_reference_keeps_a_flat_image_flat() {
        let input = vec![[0.25, 0.5, 0.75, 1.0]; 40 * 30];
        let across = AxisPlan::new(40, 13, 1.0 / 3.0);
        let down = AxisPlan::new(30, 10, 1.0 / 3.0);
        let output = reference(&input, [40, 30], &across, &down);
        assert_eq!(output.len(), 130);
        for pixel in output {
            for (c, &v) in pixel.iter().enumerate() {
                assert!((v - input[0][c]).abs() < 1e-5, "{pixel:?}");
            }
        }
    }
}
