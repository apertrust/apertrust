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
//! `_color_picker_work_4ch` and `_color_picker_work_1ch` restated on the CPU with the
//! kernel's tree of sums (`docs/pickers.md`, Tests): the sums, so the mean, the minimum
//! and the maximum to the bit, on boxes of one pixel, a row, a column, odd sizes, the
//! whole buffer and its edges, several in one dispatch, on any adapter; a mosaic per
//! Bayer and X-Trans colour; denoise against `blur_2D_Bspline` restated; a buffer read in
//! row windows equal to the same buffer bound whole; an operation's input and output in
//! one reduction. And the frame samples' cost at 2400×1600, a measurement run by hand.

use apertrust_engine::compute::GpuImage;
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::picker::{
    Cfa, Group, Layout, MAX_BOXES, Reduced, Reducer, Region, Stats, pixels,
};

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
fn reference(pixels: &[[f32; 4]], width: u32, bounds: [u32; 4]) -> Stats {
    let [left, top, right, bottom] = bounds;
    let count = (right - left) * (bottom - top);
    let width = width as usize;
    let all = |x: usize, y: usize| pixels[y * width + x].map(Some);
    tree_stats(bounds, all, [count; 4])
}

/// darktable's statistics of `bounds`, where each pixel at (x, y) adds what `at` gives each
/// channel, a mosaic's photosite to its colour's channel alone, added as the kernel adds
/// them (`reference`); each channel's mean its sum over its `counts`, 0 for none.
fn tree_stats(
    [left, top, right, bottom]: [u32; 4],
    at: impl Fn(usize, usize) -> [Option<f32>; 4],
    counts: [u32; 4],
) -> Stats {
    let (left, top, right, bottom) = (left as usize, top as usize, right as usize, bottom as usize);
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
                        let v = at(x, y);
                        *sum = add(*sum, v.map(|v| v.unwrap_or(0.0)));
                        low = std::array::from_fn(|c| v[c].map_or(low[c], |v| low[c].min(v)));
                        high = std::array::from_fn(|c| v[c].map_or(high[c], |v| high[c].max(v)));
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
    let sums = tree(sums);
    Stats {
        mean: std::array::from_fn(|c| match counts[c] {
            0 => 0.0,
            count => sums[c] / count as f32,
        }),
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

/// A storage buffer holding `values`.
fn buffer(gpu: &GpuContext, values: &[f32]) -> wgpu::Buffer {
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("picked"),
        size: (values.len() * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    gpu.queue
        .write_buffer(&buffer, 0, bytemuck::cast_slice(values));
    buffer
}

/// Each group's statistics, reduced on `gpu` in one reduction.
fn groups(gpu: &GpuContext, groups: &[Group]) -> Vec<Stats> {
    let mut reducer = pollster::block_on(Reducer::new(gpu)).expect("the kernel compiles");
    reducer
        .measure_groups(gpu, groups, ())
        .expect("its windows bind");
    let mut readings: Vec<Reduced<()>> = reducer.wait(gpu).expect("it reads back");
    readings.pop().expect("a reading").stats
}

/// Every statistic's bits.
fn bits(stats: &[Stats]) -> Vec<[[u32; 4]; 3]> {
    (stats.iter())
        .map(|s| [s.mean, s.min, s.max].map(|v| v.map(f32::to_bits)))
        .collect()
}

/// A Bayer RGGB pattern and Fujifilm's X-Trans, each with the mosaic's corner on the
/// sensor somewhere other than the pattern's.
fn patterns() -> [Cfa; 2] {
    #[rustfmt::skip]
    let xtrans = vec![
        1, 1, 0, 1, 1, 2,
        1, 1, 2, 1, 1, 0,
        2, 0, 1, 0, 2, 1,
        1, 1, 2, 1, 1, 0,
        1, 1, 0, 1, 1, 2,
        0, 2, 1, 2, 0, 1,
    ];
    [
        Cfa {
            size: [2, 2],
            colours: vec![0, 1, 1, 2],
            origin: [1, 2],
        },
        Cfa {
            size: [6, 6],
            colours: xtrans,
            origin: [4, 3],
        },
    ]
}

#[test]
fn a_mosaic_is_reduced_per_cfa_colour_as_darktable_counts_it() {
    let gpu = gpu();
    let (width, height) = (203, 151);
    let mosaic: Vec<f32> = frame(width, height).iter().map(|v| v[0]).collect();
    let input = buffer(&gpu, &mosaic);
    let boxes = [
        [0, 0, 1, 1],
        [0, 0, 2, 2],
        [5, 7, 6, 13],
        [3, 4, 70, 69],
        [0, 0, width, height],
        [130, 20, 203, 151],
    ];
    for cfa in patterns() {
        let group = Group {
            buffer: &input,
            size: [width, height],
            layout: Layout::Mosaic(&cfa),
            boxes: &boxes,
        };
        let stats = groups(&gpu, &[group]);
        for (stats, &bounds) in stats.iter().zip(&boxes) {
            // darktable's `_color_picker_bayer` and `_color_picker_xtrans`: each photosite
            // to its colour, from the mosaic's place on the sensor.
            let colour = |x: usize, y: usize| {
                let [px, py] =
                    [0, 1].map(|a| (cfa.origin[a] as usize + [x, y][a]) % cfa.size[a] as usize);
                cfa.colours[py * cfa.size[0] as usize + px] as usize
            };
            let at = |x: usize, y: usize| {
                let mut channels = [None; 4];
                channels[colour(x, y)] = Some(mosaic[y * width as usize + x]);
                channels
            };
            let expected = tree_stats(bounds, at, cfa.counts(bounds));
            assert_eq!(
                bits(std::slice::from_ref(stats)),
                bits(&[expected]),
                "{cfa:?} {bounds:?}: {stats:?}, {expected:?}"
            );
        }
    }
}

/// darktable's `blur_2D_Bspline` at one scale, negatives kept: for each row, the vertical
/// pass over the rows two either side, clamped to the image; then for each pixel the
/// horizontal pass over that row's columns two either side, clamped.
fn blurred(pixels: &[[f32; 4]], width: usize, height: usize) -> Vec<[f32; 4]> {
    const FILTER: [f32; 5] = [1.0 / 16.0, 4.0 / 16.0, 6.0 / 16.0, 4.0 / 16.0, 1.0 / 16.0];
    let product = |taps: [[f32; 4]; 5]| -> [f32; 4] {
        std::array::from_fn(|c| {
            FILTER[0] * taps[0][c]
                + FILTER[1] * taps[1][c]
                + FILTER[2] * taps[2][c]
                + FILTER[3] * taps[3][c]
                + FILTER[4] * taps[4][c]
        })
    };
    let near = |i: usize, k: usize, side: usize| (i + k).saturating_sub(2).min(side - 1);
    let mut out = vec![[0.0; 4]; width * height];
    for row in 0..height {
        let temp: Vec<[f32; 4]> = (0..width)
            .map(|x| {
                product(std::array::from_fn(|k| {
                    pixels[near(row, k, height) * width + x]
                }))
            })
            .collect();
        for x in 0..width {
            out[row * width + x] = product(std::array::from_fn(|k| temp[near(x, k, width)]));
        }
    }
    out
}

#[test]
fn denoise_blurs_each_pixel_by_darktables_b_spline_before_its_statistics() {
    let gpu = gpu();
    let (width, height) = (151, 97);
    let pixels = frame(width, height);
    let image = buffer(&gpu, pixels.as_flattened());
    // Boxes at every edge, where the blur clamps, and inside.
    let boxes = [
        [0, 0, 1, 1],
        [150, 96, 151, 97],
        [0, 0, width, height],
        [1, 1, 3, 2],
        [70, 40, 134, 97],
    ];
    let group = Group {
        buffer: &image,
        size: [width, height],
        layout: Layout::Rgba { denoise: true },
        boxes: &boxes,
    };
    let stats = groups(&gpu, &[group]);
    let blurred = blurred(&pixels, width as usize, height as usize);
    for (stats, &bounds) in stats.iter().zip(&boxes) {
        let expected = reference(&blurred, width, bounds);
        // The GPU may fuse the B-spline's multiply-adds, as darktable's compiler may:
        // within a few units in the last place.
        let near = |a: f32, b: f32| (a - b).abs() <= 4.0 * f32::EPSILON * b.abs().max(1.0);
        let agree = |a: [f32; 4], b: [f32; 4]| (0..4).all(|c| near(a[c], b[c]));
        assert!(
            agree(stats.mean, expected.mean)
                && agree(stats.min, expected.min)
                && agree(stats.max, expected.max),
            "{bounds:?}: {stats:?}, {expected:?}"
        );
    }
}

#[test]
fn a_buffer_larger_than_one_binding_is_read_in_row_windows_as_if_whole() {
    let whole = gpu();
    // At most 256 KiB at once: a 400 × 400 mosaic's 625 KiB in windows of two workgroups'
    // rows, a 150 × 400 RGBA image's 938 KiB in windows of one, with the B-spline's two
    // rows either side where denoised.
    let options = GpuOptions {
        max_storage_binding: Some(256 << 10),
        storage_offset_alignment: Some(256),
        ..GpuOptions::default()
    };
    let windowed = pollster::block_on(GpuContext::new(&options)).expect("the adapter");
    let height = 400;
    let mosaic: Vec<f32> = frame(400, height).iter().map(|v| v[2]).collect();
    let rgba = frame(150, height);
    let [bayer, _] = patterns();
    let reduce = |gpu: &GpuContext| {
        let (rgba, mosaic) = (buffer(gpu, rgba.as_flattened()), buffer(gpu, &mosaic));
        let mut stats = Vec::new();
        for (buffer, width, layout) in [
            (&mosaic, 400, Layout::Mosaic(&bayer)),
            (&rgba, 150, Layout::Rgba { denoise: true }),
            (&rgba, 150, Layout::Rgba { denoise: false }),
        ] {
            let boxes = [
                [0, 0, width, height],
                [13, 61, width - 23, 390],
                [5, 300, 6, 301],
            ];
            let group = Group {
                buffer,
                size: [width, height],
                layout,
                boxes: &boxes,
            };
            stats.extend(groups(gpu, &[group]));
        }
        stats
    };
    assert_eq!(bits(&reduce(&windowed)), bits(&reduce(&whole)));
}

#[test]
fn an_operations_input_and_output_are_reduced_together_each_in_its_own_place() {
    let gpu = gpu();
    let (width, height) = (90, 70);
    let pixels = frame(width, height);
    let output: Vec<[f32; 4]> = pixels.iter().map(|v| v.map(|c| c * 2.0 + 0.5)).collect();
    let input = buffer(&gpu, pixels.as_flattened());
    let written = buffer(&gpu, output.as_flattened());
    let bounds = [[7, 9, 80, 61]];
    let layout = Layout::Rgba { denoise: false };
    let both = groups(
        &gpu,
        &[
            Group {
                buffer: &input,
                size: [width, height],
                layout,
                boxes: &bounds,
            },
            Group {
                buffer: &written,
                size: [width, height],
                layout,
                boxes: &bounds,
            },
        ],
    );
    let expected = [
        reference(&pixels, width, bounds[0]),
        reference(&output, width, bounds[0]),
    ];
    assert_eq!(bits(&both), bits(&expected));
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
