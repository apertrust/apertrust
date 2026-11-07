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

//! The scopes' count on the GPU against the same count on the CPU, exactly, on any
//! adapter: CI's software GPUs run the workgroup atomics too. The vectorscope against
//! darktable's maths restated (`src/libs/scopes/vectorscope.c`). And the readback's
//! two staging buffers: a frame never waits for a mapping, and the last frame's reading
//! always arrives.

use apertrust_color::{Mat3, SRGB_PRIMARIES, apply, profile_rgb_to_xyz_d50, to_f32};
use apertrust_engine::compute::GpuImage;
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::scopes::{
    BINS, CELLS, COLUMNS, Graphs, Histogram, LEVELS, Meter, Reading, Totals, Waveform,
};

fn gpu() -> GpuContext {
    pollster::block_on(GpuContext::new(&GpuOptions::default()))
        .expect("a GPU adapter, hardware or software")
}

const BOTH: Graphs = Graphs {
    histogram: true,
    waveform: true,
    vectorscope: false,
};
const VECTORSCOPE: Graphs = Graphs {
    histogram: false,
    waveform: false,
    vectorscope: true,
};
const ALL: Graphs = Graphs {
    histogram: true,
    waveform: true,
    vectorscope: true,
};

/// A frame to count: its size and RGBA pixels.
struct Frame {
    width: u32,
    height: u32,
    pixels: Vec<[f32; 4]>,
}

/// The kernel's sRGB curve inverted, in f32 as it computes it.
fn srgb_to_linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v.max(0.04045) + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(v: f32) -> f32 {
    let x = v.max(0.0);
    if x <= 0.0031308 {
        12.92 * x
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    }
}

/// The kernel's encoded relative luminance.
fn luminance([r, g, b]: [f32; 3]) -> f32 {
    if r == g && g == b {
        return g;
    }
    let (lr, lg, lb) = (srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b));
    linear_to_srgb(lg + 0.2126 * (lr - lg) + 0.0722 * (lb - lg))
}

fn bin(v: f32, count: usize) -> usize {
    (v * count as f32).max(0.0).min(count as f32 - 1.0) as usize
}

/// The reading the kernel should give for `frame`, counted on the CPU.
fn expected(frame: &Frame, graphs: Graphs) -> Reading {
    let mut totals = Totals::default();
    let mut histogram = [[0u32; BINS]; 4];
    let mut waveform = vec![[0u32; COLUMNS * LEVELS]; 4];
    for (i, pixel) in frame.pixels.iter().enumerate() {
        let x = i % frame.width as usize;
        let rgb = [pixel[0], pixel[1], pixel[2]];
        let values = [rgb[0], rgb[1], rgb[2], luminance(rgb)];
        totals.pixels += 1;
        totals.shadows += u32::from(rgb.iter().any(|&v| v <= 0.0));
        totals.highlights += u32::from(rgb.iter().any(|&v| v >= 1.0));
        let column = x * COLUMNS / frame.width as usize;
        for (c, &v) in values.iter().enumerate() {
            histogram[c][bin(v, BINS)] += 1;
            waveform[c][column * LEVELS + bin(v, LEVELS)] += 1;
        }
    }
    let [red, green, blue, luminance] = histogram.map(|bins| bins.to_vec());
    let mut waveform = waveform.into_iter().map(|counts| counts.to_vec());
    let mut next = || waveform.next().expect("four channels");
    Reading {
        through: 0,
        width: frame.width,
        height: frame.height,
        totals,
        histogram: graphs.histogram.then_some(Histogram {
            red,
            green,
            blue,
            luminance,
        }),
        waveform: graphs.waveform.then(|| Waveform {
            columns: COLUMNS,
            levels: LEVELS,
            red: next(),
            green: next(),
            blue: next(),
            luminance: next(),
        }),
        vectorscope: None,
        gpu_ms: 0.0,
        latency_ms: 0.0,
    }
}

/// Whether two readings count the same, whatever their times.
fn same_counts(a: &Reading, b: &Reading) -> bool {
    (a.width, a.height, a.totals) == (b.width, b.height, b.totals)
        && a.histogram == b.histogram
        && a.waveform == b.waveform
        && a.vectorscope.as_ref().map(|v| &v.counts) == b.vectorscope.as_ref().map(|v| &v.counts)
}

/// Counts each frame, one after another without picking anything up, then waits for
/// every reading.
fn count(gpu: &GpuContext, meter: &mut Meter, frames: &[&Frame], graphs: Graphs) -> Vec<Reading> {
    for (through, frame) in (1..).zip(frames) {
        let image = GpuImage::upload(gpu, frame.width, frame.height, &frame.pixels)
            .expect("the frame uploads");
        meter.measure(gpu, &image, graphs, through);
    }
    meter.wait(gpu).expect("the readings arrive")
}

/// A grey ramp over every bin and level, from below 0 to above 1: each value is a bin's
/// edge, its centre, or a hair above its edge. A grey's luminance is its value, so all
/// four channels count alike; an odd size leaves partial workgroups at the right and the
/// bottom.
fn ramp() -> Frame {
    let (width, height) = (301u32, 83u32);
    let pixels = (0..width * height)
        .map(|i| {
            let (x, y) = (i % width, i / width);
            let step = (x as f32 - 20.0) * (1.0 + y as f32 * 0.003);
            let v = match y % 3 {
                0 => step / 256.0,
                1 => (step + 0.5) / 256.0,
                _ => step / 256.0 + 1e-6,
            };
            [v, v, v, 1.0]
        })
        .collect();
    Frame {
        width,
        height,
        pixels,
    }
}

/// Colours clipped every way: channels at exactly 0 and 1, below and above, and between,
/// from a fixed sequence. Each pixel's luminance is kept a hundredth of a bin or level
/// clear of their edges, where the two chained power functions of a GPU and the CPU's
/// could round to either side.
fn clipped(seed: u32) -> Frame {
    let (width, height) = (517u32, 389u32);
    let mut state = seed;
    let mut random = || {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (state >> 8) as f32 / (1 << 24) as f32
    };
    // A channel: exactly 0 or 1, below 0, above 1, or between, as `kind` picks.
    let channel = |kind: f32, v: f32| -> f32 {
        match (kind * 8.0) as u32 {
            0 => 0.0,
            1 => 1.0,
            2 => -0.25 * v,
            3 => 1.0 + 0.25 * v,
            _ => v,
        }
    };
    let pixels = (0..width * height)
        .map(|_| {
            loop {
                let rgb = [(); 3].map(|()| channel(random(), random()));
                let y = luminance(rgb);
                let clear = |count: f32| {
                    let at = y * count;
                    (at - at.round()).abs() > 1e-2
                };
                if clear(BINS as f32) && clear(LEVELS as f32) {
                    break [rgb[0], rgb[1], rgb[2], 1.0];
                }
            }
        })
        .collect();
    Frame {
        width,
        height,
        pixels,
    }
}

#[test]
fn the_count_on_the_gpu_equals_the_count_on_the_cpu() {
    let gpu = gpu();
    let mut meter = pollster::block_on(Meter::new(&gpu)).expect("the kernel compiles");
    let frames = [ramp(), clipped(1)];
    for frame in &frames {
        let [reading] = &count(&gpu, &mut meter, &[frame], BOTH)[..] else {
            panic!("one reading");
        };
        let expected = expected(frame, BOTH);
        assert_eq!(reading.totals, expected.totals);
        assert!(
            reading.histogram == expected.histogram,
            "the histograms differ"
        );
        assert!(
            reading.waveform == expected.waveform,
            "the waveforms differ"
        );
        assert_eq!(
            (reading.width, reading.height, reading.through),
            (frame.width, frame.height, 1)
        );
    }

    // The ramp: every grey in the same bin in all four channels, and some of it clips.
    let ramp = expected(&frames[0], BOTH);
    let histogram = ramp.histogram.as_ref().unwrap();
    assert!(histogram.red == histogram.luminance && histogram.green == histogram.blue);
    assert!(histogram.red.iter().all(|&count| count > 0));
    assert!(ramp.totals.shadows > 0 && ramp.totals.highlights > 0);
    // The colours: clipping both ways, and not everywhere.
    let colours = expected(&frames[1], BOTH).totals;
    assert!(colours.shadows > colours.pixels / 4 && colours.shadows < colours.pixels);
    assert!(colours.highlights > colours.pixels / 4 && colours.highlights < colours.pixels);
}

#[test]
fn only_the_totals_unless_a_graph_is_asked_for() {
    let gpu = gpu();
    let mut meter = pollster::block_on(Meter::new(&gpu)).expect("the kernel compiles");
    let frame = clipped(2);
    for graphs in [
        Graphs::default(),
        Graphs {
            histogram: true,
            ..Graphs::default()
        },
        Graphs {
            waveform: true,
            ..Graphs::default()
        },
    ] {
        let readings = count(&gpu, &mut meter, &[&frame], graphs);
        assert!(
            matches!(&readings[..], [reading] if same_counts(reading, &expected(&frame, graphs))),
            "{graphs:?}"
        );
    }
    // Nor the others while only the vectorscope is.
    let [reading] = &count(&gpu, &mut meter, &[&frame], VECTORSCOPE)[..] else {
        panic!("one reading");
    };
    assert!(reading.histogram.is_none() && reading.waveform.is_none());
    assert_eq!(reading.totals, expected(&frame, VECTORSCOPE).totals);
}

#[test]
fn while_both_staging_buffers_map_the_latest_counts_wait_and_arrive() {
    let gpu = gpu();
    let mut meter = pollster::block_on(Meter::new(&gpu)).expect("the kernel compiles");
    let frames: Vec<Frame> = (10..14).map(clipped).collect();
    let refs: Vec<&Frame> = frames.iter().collect();
    // The first two take the staging buffers; nothing picks them up before the fourth is
    // counted, so the third's counts wait and the fourth's replace them.
    let readings = count(&gpu, &mut meter, &refs, BOTH);
    let through: Vec<u64> = readings.iter().map(|r| r.through).collect();
    assert_eq!(through, [1, 2, 4]);
    for reading in &readings {
        let frame = &frames[reading.through as usize - 1];
        assert!(
            same_counts(reading, &expected(frame, BOTH)),
            "frame {}",
            reading.through
        );
    }
    assert!(!meter.pending());

    // Readings of a photo that has closed are dropped, mapping or waiting.
    for (through, frame) in (5..).zip(&frames) {
        let image = GpuImage::upload(&gpu, frame.width, frame.height, &frame.pixels).unwrap();
        meter.measure(&gpu, &image, Graphs::default(), through);
    }
    meter.discard();
    assert!(meter.wait(&gpu).unwrap().is_empty());
}

/// darktable's default vectorscope restated (`src/libs/scopes/vectorscope.c`): CIE
/// 1976 u*v* against D50 of the colour read through darktable's sRGB profile, on its
/// logarithmic scale out to the most saturated colour of sRGB's hue ring.
struct Darktable {
    /// lcms's sRGB to XYZ D50, from darktable's D65 for its built-in profiles.
    to_xyz: Mat3,
    /// `dt_iop_estimate_exp` on sRGB's curve: the power law for values at and above 1.
    exponent: f32,
    /// The hue ring's largest chroma.
    radius: f32,
}

impl Darktable {
    fn new() -> Self {
        let x = [0.7f32, 0.8, 0.9];
        let exponent = x
            .iter()
            .map(|&x| srgb_to_linear(x).ln() / x.ln())
            .sum::<f32>()
            / 3.0;
        let mut darktable = Darktable {
            to_xyz: to_f32(profile_rgb_to_xyz_d50([0.31271, 0.32902], SRGB_PRIMARIES)),
            exponent,
            radius: 0.0,
        };
        // `_lib_histogram_vectorscope_bkgd`: 48 samples along each edge of the RGB cube
        // that touches neither black nor white.
        let mut radius = 0.0f32;
        for k in 0..6 {
            let (from, to) = (CORNERS[k], CORNERS[(k + 1) % 6]);
            for i in 0..48 {
                let rgb = std::array::from_fn(|c| from[c] + (to[c] - from[c]) / 48.0 * i as f32);
                let [u, v] = darktable.chroma(rgb);
                radius = radius.max(u.hypot(v));
            }
        }
        darktable.radius = radius;
        darktable
    }

    /// `_get_chromaticity` for u*v*.
    fn chroma(&self, rgb: [f32; 3]) -> [f32; 2] {
        let linear = rgb.map(|v| {
            if v < 1.0 {
                srgb_to_linear(v.max(0.0))
            } else {
                v.powf(self.exponent)
            }
        });
        let [x, y, z] = apply(&self.to_xyz, linear).map(|v| v.max(0.0));
        let sum = x + y + z;
        let [cx, cy] = if sum > 0.0 {
            [x / sum, y / sum]
        } else {
            [0.34567, 0.35850]
        };
        let denominator = -2.0 * cx + 12.0 * cy + 3.0;
        let (u, v) = (4.0 * cx / denominator, 9.0 * cy / denominator);
        let lstar = if y <= (6.0f32 / 29.0).powi(3) {
            (29.0f32 / 3.0).powi(3) * y
        } else {
            116.0 * y.cbrt() - 16.0
        };
        #[allow(clippy::excessive_precision)]
        let white = [0.20915914598542354f32, 0.488075320769787];
        [13.0 * lstar * (u - white[0]), 13.0 * lstar * (v - white[1])]
    }

    /// Where `_vec_process` bins a sample, before it truncates: `(diam - 1) × (c / (2
    /// radius) + 0.5)` on the logarithmic scale, across u* and v*.
    fn position(&self, rgb: [f32; 3]) -> [f32; 2] {
        let [mut u, mut v] = self.chroma(rgb);
        let h = u.hypot(v);
        if h >= f32::MIN_POSITIVE {
            let scaled = (29.0 * h / self.radius).ln_1p() / 30.0f32.ln() * self.radius;
            (u, v) = (u * (scaled / h), v * (scaled / h));
        }
        [u, v].map(|c| (CELLS - 1) as f32 * (c / (2.0 * self.radius) + 0.5))
    }

    /// The sample's cell in the reading, in darktable's default orientation, u* up and v*
    /// to the left; none off the grid.
    fn cell(&self, rgb: [f32; 3]) -> Option<usize> {
        let [u, v] = self.position(rgb);
        let on = |p: f32| p > -1.0 && p < CELLS as f32;
        (on(u) && on(v)).then(|| (CELLS - 1 - u as usize) * CELLS + (CELLS - 1 - v as usize))
    }
}

/// The hue ring's corners, red, yellow, green, cyan, blue and magenta.
const CORNERS: [[f32; 3]; 6] = [
    [1.0, 0.0, 0.0],
    [1.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 1.0, 1.0],
    [0.0, 0.0, 1.0],
    [1.0, 0.0, 1.0],
];

/// Whether the sample falls a hundredth of a cell clear of every cell's edge, where the
/// GPU's powers and logarithms and the CPU's could round to either side.
fn clear(darktable: &Darktable, rgb: [f32; 3]) -> bool {
    darktable
        .position(rgb)
        .iter()
        .all(|p| (p - p.round()).abs() > 1e-2)
}

/// darktable's sample at `x`, `y`: the mean of the 2 × 2 block there, added down the left
/// column, then the right.
fn block_mean(frame: &Frame, x: usize, y: usize) -> [f32; 3] {
    let width = frame.width as usize;
    let mut rgb = [0.0f32; 3];
    for (dx, dy) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
        let pixel = frame.pixels[(y + dy) * width + x + dx];
        for c in 0..3 {
            rgb[c] += pixel[c] * 0.25;
        }
    }
    rgb
}

/// The vectorscope darktable counts of `frame`: a sample for each 2 × 2 block, a last odd
/// row or column left out.
fn expected_vectorscope(frame: &Frame) -> Vec<u32> {
    let darktable = Darktable::new();
    let (width, height) = (frame.width as usize, frame.height as usize);
    let mut counts = vec![0u32; CELLS * CELLS];
    for y in (0..height - height % 2).step_by(2) {
        for x in (0..width - width % 2).step_by(2) {
            if let Some(cell) = darktable.cell(block_mean(frame, x, y)) {
                counts[cell] += 1;
            }
        }
    }
    counts
}

/// A frame of 2 × 2 blocks: first sRGB's primaries and secondaries, then colours from a
/// fixed sequence, a block's four pixels alike or not: bright, dark, at 0 and 1, below 0
/// and above 1, so some fall off the grid; each block's sample clear of every cell's
/// edge. Odd in both directions, its last row and column pure red, which darktable leaves
/// out.
fn colours(seed: u32) -> Frame {
    let darktable = Darktable::new();
    let (width, height) = (263u32, 151u32);
    let mut pixels = vec![[1.0, 0.0, 0.0, 1.0]; (width * height) as usize];
    let mut state = seed;
    let mut random = || {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (state >> 8) as f32 / (1 << 24) as f32
    };
    let (across, down) = (width as usize / 2, height as usize / 2);
    for block in 0..across * down {
        let four: [[f32; 3]; 4] = match CORNERS.get(block) {
            Some(&rgb) => {
                assert!(clear(&darktable, rgb), "{rgb:?} near a cell's edge");
                [rgb; 4]
            }
            None => loop {
                let kind = (random() * 6.0) as u32;
                let mut colour = || -> [f32; 3] {
                    std::array::from_fn(|_| match kind {
                        0 | 1 => random(),
                        2 => 1.4 * random(),
                        3 => 0.05 * random(),
                        4 => (random() * 2.0).floor(),
                        _ => 1.2 * random() - 0.2,
                    })
                };
                let first = colour();
                let four = if kind == 1 {
                    [first, colour(), colour(), colour()]
                } else {
                    [first; 4]
                };
                let mean = four.iter().fold([0.0f32; 3], |sum, pixel| {
                    std::array::from_fn(|c| sum[c] + pixel[c] * 0.25)
                });
                if clear(&darktable, mean) {
                    break four;
                }
            },
        };
        let (x, y) = (2 * (block % across), 2 * (block / across));
        for ([r, g, b], (dx, dy)) in four.into_iter().zip([(0, 0), (0, 1), (1, 0), (1, 1)]) {
            pixels[(y + dy) * width as usize + x + dx] = [r, g, b, 1.0];
        }
    }
    Frame {
        width,
        height,
        pixels,
    }
}

#[test]
fn the_vectorscope_counts_each_block_where_darktable_puts_it() {
    let gpu = gpu();
    let mut meter = pollster::block_on(Meter::new(&gpu)).expect("the kernel compiles");
    let frame = colours(1);
    let [reading] = &count(&gpu, &mut meter, &[&frame], VECTORSCOPE)[..] else {
        panic!("one reading");
    };
    let vectorscope = reading.vectorscope.as_ref().expect("asked for");
    let expected = expected_vectorscope(&frame);
    assert_eq!(vectorscope.size, CELLS);
    assert_eq!(vectorscope.colours.len(), CELLS * CELLS);
    let differ = (0..CELLS * CELLS)
        .filter(|&i| vectorscope.counts[i] != expected[i])
        .count();
    assert_eq!(
        differ, 0,
        "cells counted otherwise than darktable counts them"
    );

    // Every block a sample, but those off the grid; the last row and column none.
    let blocks = frame.width / 2 * (frame.height / 2);
    let counted: u32 = expected.iter().sum();
    assert!(
        counted < blocks && counted > blocks * 9 / 10,
        "{counted} of {blocks}"
    );

    // As darktable shows it by default: red at the top, yellow to the left, blue to the
    // right, cyan at the bottom.
    let darktable = Darktable::new();
    let at = |rgb| {
        let cell = darktable.cell(rgb).expect("on the grid");
        assert!(vectorscope.counts[cell] > 0);
        (cell / CELLS, cell % CELLS)
    };
    let (row, column) = at([1.0, 0.0, 0.0]);
    assert!(
        row < 8 && column.abs_diff(CELLS / 2) < 16,
        "red at {row}, {column}"
    );
    let (row, column) = at([1.0, 1.0, 0.0]);
    assert!(
        column < 24 && row.abs_diff(CELLS / 2) < 16,
        "yellow at {row}, {column}"
    );
    let (row, column) = at([0.0, 0.0, 1.0]);
    assert!(
        column > CELLS - 16 && row.abs_diff(CELLS / 2) < 16,
        "blue at {row}, {column}"
    );
    let (row, column) = at([0.0, 1.0, 1.0]);
    assert!(
        row > CELLS - 24 && column.abs_diff(CELLS / 2) < 16,
        "cyan at {row}, {column}"
    );
}

#[test]
fn a_neutral_grey_counts_at_the_centre() {
    let gpu = gpu();
    let mut meter = pollster::block_on(Meter::new(&gpu)).expect("the kernel compiles");
    // Greys from black to above white, a block's four pixels alike or not.
    let (width, height) = (96u32, 40u32);
    let pixels: Vec<[f32; 4]> = (0..width * height)
        .map(|i| {
            let (x, y) = (i % width, i / width);
            let v = (x / 2) as f32 / 32.0 + if y % 4 == 3 { 0.01 * x as f32 } else { 0.0 };
            [v, v, v, 1.0]
        })
        .collect();
    let frame = Frame {
        width,
        height,
        pixels,
    };
    let [reading] = &count(&gpu, &mut meter, &[&frame], VECTORSCOPE)[..] else {
        panic!("one reading");
    };
    let counts = &reading.vectorscope.as_ref().expect("asked for").counts;
    let centre = CELLS / 2 * CELLS + CELLS / 2;
    assert_eq!(counts[centre], width * height / 4);
    assert_eq!(counts.iter().sum::<u32>(), counts[centre]);
    assert_eq!(expected_vectorscope(&frame), *counts);
}

#[test]
fn the_vectorscope_counts_the_same_alone_or_with_the_other_graphs() {
    let gpu = gpu();
    let mut meter = pollster::block_on(Meter::new(&gpu)).expect("the kernel compiles");
    for frame in [colours(2), clipped(3)] {
        let [alone] = &count(&gpu, &mut meter, &[&frame], VECTORSCOPE)[..] else {
            panic!("one reading");
        };
        let [all] = &count(&gpu, &mut meter, &[&frame], ALL)[..] else {
            panic!("one reading");
        };
        assert!(alone.vectorscope.is_some() && alone.vectorscope == all.vectorscope);
        assert!(alone.histogram.is_none() && alone.waveform.is_none());
        assert_eq!(alone.totals, all.totals);
    }
    // And the others' counts are as ever beside it.
    let frame = clipped(4);
    let [all] = &count(&gpu, &mut meter, &[&frame], ALL)[..] else {
        panic!("one reading");
    };
    let others = expected(&frame, BOTH);
    assert_eq!(all.totals, others.totals);
    assert!(all.histogram == others.histogram && all.waveform == others.waveform);
}
