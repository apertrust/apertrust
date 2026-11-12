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

//! The colour pickers' reduction on the GPU against darktable's
//! `_color_picker_work_4ch` restated on the CPU with the kernel's tree of sums
//! (`docs/pickers.md`, Tests): the sums, so the mean, the minimum and the maximum to the
//! bit, on boxes of one pixel, a row, a column, odd sizes, the whole buffer and its
//! edges, several in one dispatch, on any adapter. And the frame samples' cost at
//! 2400×1600, a measurement run by hand.

use apertrust_engine::compute::GpuImage;
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::picker::{MAX_BOXES, Reduced, Reducer, Region, Stats, pixels};

fn gpu() -> GpuContext {
    pollster::block_on(GpuContext::new(&GpuOptions::default()))
        .expect("a GPU adapter, hardware or software")
}

/// The kernel's workgroup: 16 × 16 invocations, each adding 4 × 4 pixels 16 apart.
const SIDE: usize = 16;
const BLOCK: usize = 4;
const TILE: usize = SIDE * BLOCK;

/// A frame of `width` × `height` with values that differ in every channel and pixel, some
/// negative and some above 1, as a scene-referred buffer holds.
fn frame(width: u32, height: u32) -> Vec<[f32; 4]> {
    let mut state = 0x9e37_79b9_u32 ^ width.wrapping_mul(31) ^ height;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state as f32 / u32::MAX as f32
    };
    (0..width * height)
        .map(|_| [next() * 1.4 - 0.2, next(), next() * 3.0, next() * 0.01])
        .collect()
}

/// darktable's statistics of `bounds` in `pixels`, `width` wide, added as the kernel adds
/// them: each invocation's 4 × 4 pixels row by row, the workgroup's 256 sums as a tree,
/// pairs 128 apart first, the workgroups' sums 256 apart in their order, then a tree
/// again; the mean the sum over the number of pixels.
fn reference(pixels: &[[f32; 4]], width: u32, [left, top, right, bottom]: [u32; 4]) -> Stats {
    let (left, top, right, bottom) = (left as usize, top as usize, right as usize, bottom as usize);
    let width = width as usize;
    let add = |a: [f32; 4], b: [f32; 4]| std::array::from_fn(|c| a[c] + b[c]);
    let tree = |mut sums: Vec<[f32; 4]>| {
        let mut stride = sums.len() / 2;
        while stride > 0 {
            for i in 0..stride {
                sums[i] = add(sums[i], sums[i + stride]);
            }
            stride /= 2;
        }
        sums[0]
    };
    let (across, down) = ((right - left).div_ceil(TILE), (bottom - top).div_ceil(TILE));
    let mut partials = Vec::new();
    let (mut low, mut high) = ([f32::MAX; 4], [-f32::MAX; 4]);
    for tile in 0..across * down {
        let corner = [left + tile % across * TILE, top + tile / across * TILE];
        let mut sums = vec![[0.0f32; 4]; SIDE * SIDE];
        for (index, sum) in sums.iter_mut().enumerate() {
            for by in 0..BLOCK {
                for bx in 0..BLOCK {
                    let x = corner[0] + index % SIDE + bx * SIDE;
                    let y = corner[1] + index / SIDE + by * SIDE;
                    if x < right && y < bottom {
                        let v = pixels[y * width + x];
                        *sum = add(*sum, v);
                        low = std::array::from_fn(|c| low[c].min(v[c]));
                        high = std::array::from_fn(|c| high[c].max(v[c]));
                    }
                }
            }
        }
        partials.push(tree(sums));
    }
    let mut sums = vec![[0.0f32; 4]; SIDE * SIDE];
    for (i, partial) in partials.iter().enumerate() {
        sums[i % (SIDE * SIDE)] = add(sums[i % (SIDE * SIDE)], *partial);
    }
    let count = ((right - left) * (bottom - top)) as f32;
    Stats {
        mean: tree(sums).map(|sum| sum / count),
        min: low,
        max: high,
    }
}

fn reduced(gpu: &GpuContext, width: u32, height: u32, boxes: &[[u32; 4]]) -> Vec<Stats> {
    let pixels = frame(width, height);
    let image = GpuImage::upload(gpu, width, height, &pixels).expect("the frame uploads");
    let mut reducer = pollster::block_on(Reducer::new(gpu)).expect("the kernel compiles");
    reducer.measure(gpu, &image, boxes, 7u64);
    let mut readings: Vec<Reduced<u64>> = reducer.wait(gpu).expect("it reads back");
    let reading = readings.pop().expect("a reading");
    assert!(readings.is_empty() && reading.tag == 7);
    assert_eq!(reading.stats.len(), boxes.len());
    for (stats, &bounds) in reading.stats.iter().zip(boxes) {
        let expected = reference(&pixels, width, bounds);
        // To the bit: the same tree adds the same floats, and the CPU divides the sum.
        let bits = |stats: &Stats| [stats.mean, stats.min, stats.max].map(|v| v.map(f32::to_bits));
        assert_eq!(
            bits(stats),
            bits(&expected),
            "{bounds:?}: {stats:?}, {expected:?}"
        );
    }
    reading.stats
}

#[test]
fn every_box_s_mean_minimum_and_maximum_are_the_cpu_s() {
    let gpu = gpu();
    let (width, height) = (301, 203);
    let boxes = [
        // One pixel, at a corner, at the far corner and inside.
        [0, 0, 1, 1],
        [300, 202, 301, 203],
        [150, 101, 151, 102],
        // A row, a column, the whole buffer.
        [0, 77, 301, 78],
        [123, 0, 124, 203],
        [0, 0, 301, 203],
        // Odd sizes, across workgroups' edges and up to the buffer's.
        [3, 5, 70, 69],
        [63, 63, 65, 65],
        [17, 40, 301, 203],
        [250, 0, 301, 130],
    ];
    reduced(&gpu, width, height, &boxes);
}

#[test]
fn a_dispatch_takes_up_to_64_boxes_each_reduced_alone() {
    let gpu = gpu();
    let (width, height) = (97, 61);
    let boxes: Vec<[u32; 4]> = (0..MAX_BOXES as u32)
        .map(|i| {
            let (x, y) = (i * 7 % 90, i * 5 % 55);
            [x, y, x + 1 + i % 7, y + 1 + i % 6]
        })
        .collect();
    let first = reduced(&gpu, width, height, &boxes);
    // The same box alone gives the same bits.
    let alone = reduced(&gpu, width, height, &boxes[40..41]);
    assert_eq!(alone[0], first[40]);
}

#[test]
fn a_point_is_the_pixel_it_falls_in_and_an_area_darktables_box() {
    let size = [40, 30];
    assert_eq!(
        pixels(Region::Point([0.5, 0.5]), size),
        Some([20, 15, 21, 16])
    );
    assert_eq!(pixels(Region::Point([0.0, 0.0]), size), Some([0, 0, 1, 1]));
    assert_eq!(
        pixels(Region::Point([0.999, 0.999]), size),
        Some([39, 29, 40, 30])
    );
    // On the right or bottom edge it falls in no pixel, as in darktable.
    assert_eq!(pixels(Region::Point([1.0, 0.5]), size), None);
    assert_eq!(pixels(Region::Point([0.5, 1.0]), size), None);
    // An area's sides truncated toward zero, at least a pixel wide and high.
    assert_eq!(
        pixels(Region::Area([0.02, 0.02, 0.98, 0.98]), size),
        Some([0, 0, 39, 29])
    );
    assert_eq!(
        pixels(Region::Area([0.1, 0.2, 0.1, 0.2]), size),
        Some([4, 6, 5, 7])
    );
    assert_eq!(
        pixels(Region::Area([0.0, 0.0, 1.0, 1.0]), size),
        Some([0, 0, 40, 30])
    );
    // An area along the right edge misses, as darktable's empty box.
    assert_eq!(pixels(Region::Area([1.0, 0.0, 1.0, 1.0]), size), None);
}

/// The frame samples' cost, `docs/pickers.md`'s Cost: the reduction over a 2400×1600
/// frame, the whole of it, a default area and a point, each its GPU time and its latency
/// from submission to reading, median of 50 after 10 to warm up.
#[test]
#[ignore = "a measurement: cargo test --release -p apertrust-engine --test picker -- --ignored --nocapture"]
fn the_frame_samples_cost_at_2400_by_1600() {
    let gpu = gpu();
    let (width, height) = (2400, 1600);
    let image = GpuImage::upload(&gpu, width, height, &frame(width, height)).expect("uploads");
    let mut reducer = pollster::block_on(Reducer::new(&gpu)).expect("the kernel compiles");
    let cases = [
        ("the whole frame", [0, 0, width, height]),
        ("the default area", [48, 32, 2352, 1568]),
        ("a 5 % area", [1140, 760, 1260, 840]),
        ("a point", [1200, 800, 1201, 801]),
    ];
    let median = |mut values: Vec<f64>| {
        values.sort_by(f64::total_cmp);
        (values[values.len() / 2], values[0])
    };
    for (name, bounds) in cases {
        let mut timed = Vec::new();
        for run in 0..60 {
            reducer.measure(&gpu, &image, &[bounds], ());
            let reading = reducer.wait(&gpu).expect("it reads back").remove(0);
            if run >= 10 {
                timed.push((reading.gpu_ms, reading.latency_ms));
            }
        }
        let (gpu_ms, gpu_min) = median(timed.iter().map(|t| t.0).collect());
        let (latency, latency_min) = median(timed.iter().map(|t| t.1).collect());
        println!(
            "{name} of {width}×{height}: {gpu_ms:.3} ms on the GPU (min {gpu_min:.3}), read back after {latency:.2} ms (min {latency_min:.2}), median of {}",
            timed.len()
        );
    }
}
