// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2022-2026  darktable developers (ported from src/iop/hlreconstruct/opposed.c
// and src/iop/hlreconstruct/segbased.c)
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

//! Highlight reconstruction's inpaint opposed on a GPU adapter, hardware or software,
//! against darktable's CPU loops restated in Rust: the masks, their dilation, the
//! chrominance reduction and the inpaint, edges and darktable's reads past its masks
//! included; and in row strips on a GPU that binds less than the mosaic at once, the
//! same as the whole mosaic bit for bit (decision 14).

use apertrust_engine::compute::GpuImage;
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe, Pipeline};
use apertrust_operations::highlights::Highlights;
use apertrust_operations::rawprepare::Rawprepare;
use apertrust_operations::temperature::Temperature;
use apertrust_plugin::{
    Camera, Format, ImageContext, ImageDesc, Operation, ParamValues, Pattern, Value,
};

const COEFFICIENTS: [f32; 3] = [2.0, 1.0, 1.5];
const WHITE: f32 = 4000.0;
// XYZ to sRGB with its rows scaled so the camera's daylight multipliers are about 1.9,
// 1 and 1.6: the late correction is then near 0.95, 1 and 1.07.
const XYZ_TO_CAMERA: [[f32; 3]; 3] = [
    [1.7056, -0.8091, -0.2624],
    [-0.9689, 1.8758, 0.0415],
    [0.0348, -0.1275, 0.6606],
];

const BAYER: [u32; 4] = [0, 1, 1, 2];
#[rustfmt::skip]
const X_TRANS: [u32; 36] = [
    1, 1, 0, 1, 1, 2,
    1, 1, 2, 1, 1, 0,
    2, 0, 1, 0, 2, 1,
    1, 1, 2, 1, 1, 0,
    1, 1, 0, 1, 1, 2,
    0, 2, 1, 2, 0, 1,
];

/// A highlight: its centre, its radius and how far each colour rises, as fractions of
/// white.
type Highlight = (f32, f32, f32, [f32; 3]);

/// A raw `width` × `height` photosites, cropped by one on every side: a background
/// rising from `base` to 1.57 × `base` towards the right, a little brighter in green and
/// blue, with highlights where some colours clip.
struct Scene {
    cfa: Pattern<u32>,
    width: u32,
    height: u32,
    base: f32,
    highlights: Vec<Highlight>,
}

impl Scene {
    /// A gradient from 35% to 60% of white with three highlights, one at the
    /// bottom-right corner: enough photosites around the clipped ones give every colour
    /// its offset.
    fn busy(cfa: Pattern<u32>, width: u32, height: u32) -> Self {
        let (w, h) = (width as f32, height as f32);
        Self {
            cfa,
            width,
            height,
            base: 0.35,
            highlights: vec![
                (16.0, 13.0, 8.0, [0.9, 1.25, 0.7]),
                (0.55 * w, 0.5 * h, 11.0, [1.0, 0.85, 1.3]),
                (w - 7.0, h - 6.0, 7.0, [1.1, 1.15, 1.05]),
            ],
        }
    }

    fn crop(&self) -> [u32; 4] {
        [1, 1, self.width - 2, self.height - 2]
    }

    fn color(&self, x: u32, y: u32) -> usize {
        let cfa = &self.cfa;
        cfa.values[((y % cfa.height) * cfa.width + x % cfa.width) as usize] as usize
    }

    fn raw(&self) -> Vec<u16> {
        let w = self.width as f32;
        (0..self.width * self.height)
            .map(|i| {
                let (x, y) = (i % self.width, i / self.width);
                let c = self.color(x, y);
                let mut level = self.base * (1.0 + 4.0 / 7.0 * x as f32 / w + c as f32 / 7.0);
                for &(cx, cy, sigma, amplitude) in &self.highlights {
                    let r2 = (x as f32 - cx).powi(2) + (y as f32 - cy).powi(2);
                    level += amplitude[c] * (-r2 / (2.0 * sigma * sigma)).exp();
                }
                (level.min(1.0) * WHITE).round() as u16
            })
            .collect()
    }

    fn image(&self) -> ImageContext {
        let crop = self.crop();
        ImageContext {
            camera: Some(Camera {
                make: String::new(),
                model: String::new(),
                alias: String::new(),
                width: self.width,
                height: self.height,
                cfa: Some(self.cfa.clone()),
                black: Pattern {
                    width: 1,
                    height: 1,
                    values: vec![0.0],
                },
                white: WHITE,
                white_balance: Some(COEFFICIENTS.to_vec()),
                xyz_to_camera: Some(XYZ_TO_CAMERA),
                crop,
                declared: crop,
                exposure_bias: 0.0,
                iso: 0.0,
                linear_response_limit: 1.0,
                lens_correction: None,
                orientation: 0,
            }),
            ..ImageContext::full(crop[2], crop[3])
        }
    }

    /// The prepared, white-balanced mosaic, then the same through highlight
    /// reconstruction, as the GPU renders them, in row strips where it binds less than
    /// the mosaic at once.
    fn render(&self, gpu: &GpuContext, late_correction: bool) -> (Vec<f32>, Vec<f32>) {
        let image = self.image();
        let raw = self.raw();
        // "As shot to reference", the default with a matrix (decision 27), or "as shot".
        let mut temperature = ParamValues::new();
        let preset = if late_correction { 4 } else { 0 };
        temperature.set("preset", Value::Choice(preset));
        let render = |operations: &[&'static dyn Operation]| {
            let instances: Vec<Instance> = operations
                .iter()
                .map(|&operation| Instance {
                    operation,
                    params: if operation.manifest().id == "temperature" {
                        temperature.clone()
                    } else {
                        ParamValues::new()
                    },
                })
                .collect();
            let input = ImageDesc::new(self.width, self.height, Format::Mosaic16);
            let limits = Limits::of(gpu);
            let plan = pipeline::plan(&instances, &image, input, Pipe::Export, &limits).unwrap();
            let input = GpuImage::upload_mosaic(gpu, self.width, self.height, &raw).unwrap();
            let mut pipeline = pollster::block_on(Pipeline::new(gpu, plan, input)).unwrap();
            pipeline.render(gpu).unwrap();
            pipeline.output().download_mosaic(gpu).unwrap()
        };
        (
            render(&[&Rawprepare, &Temperature]),
            render(&[&Rawprepare, &Temperature, &Highlights]),
        )
    }
}

/// darktable `_process_opposed` for a mosaic, its CPU loops in order and its masks laid
/// out as it lays them out: the output, the chrominance offsets and how many photosites
/// gave each. darktable keeps a colour's offset when more than `min_count`, 100,
/// photosites gave it.
fn opposed(
    input: &[f32],
    width: usize,
    height: usize,
    color: impl Fn(usize, usize) -> usize,
    clips: [f32; 3],
    correction: [f32; 3],
    min_count: u32,
) -> (Vec<f32>, [f32; 3], [u32; 3]) {
    let (mwidth, mheight) = (width / 3, height / 3);
    let msize = mwidth.next_multiple_of(8) * mheight.next_multiple_of(8);
    let mut mask = vec![0u8; 6 * msize];
    for mrow in 0..mheight - 1 {
        for mcol in 0..mwidth - 1 {
            let mut clipped = [0u8; 3];
            for y in 0..3 {
                for x in 0..3 {
                    let (row, col) = (3 * mrow + y, 3 * mcol + x);
                    let c = color(col, row);
                    clipped[c] += u8::from(input[row * width + col] >= clips[c]);
                }
            }
            for c in 0..3 {
                mask[c * msize + mrow * mwidth + mcol] = u8::from(clipped[c] > 0);
            }
        }
    }
    // `_mask_dilated`'s offsets, as darktable lists them.
    let w1 = mwidth as isize;
    let (w2, w3) = (2 * w1, 3 * w1);
    #[rustfmt::skip]
    let offsets = [
        0, -w1 - 1, -w1, -w1 + 1, -1, 1, w1 - 1, w1, w1 + 1,
        -w3 - 2, -w3 - 1, -w3, -w3 + 1, -w3 + 2,
        -w2 - 3, -w2 - 2, -w2 - 1, -w2, -w2 + 1, -w2 + 2, -w2 + 3,
        -w1 - 3, -w1 - 2, -w1 + 2, -w1 + 3,
        -3, -2, 2, 3,
        w1 - 3, w1 - 2, w1 + 2, w1 + 3,
        w2 - 3, w2 - 2, w2 - 1, w2, w2 + 1, w2 + 2, w2 + 3,
        w3 - 2, w3 - 1, w3, w3 + 1, w3 + 2,
    ];
    for row in 0..mheight {
        for col in 0..mwidth {
            let mx = row * mwidth + col;
            let safe = col >= 3 && row >= 3 && col + 4 < mwidth && row + 4 < mheight;
            for c in 0..3 {
                let plane = c * msize;
                mask[(3 + c) * msize + mx] = if safe {
                    u8::from(
                        offsets
                            .iter()
                            .any(|&o| mask[(plane as isize + mx as isize + o) as usize] != 0),
                    )
                } else {
                    mask[plane + mx]
                };
            }
        }
    }

    // `_calc_refavg`, linear.
    let refavg = |row: usize, col: usize| {
        let mut mean = [0.0f32; 3];
        let mut count = [0.0f32; 3];
        for dy in row.saturating_sub(1)..(height - 1).min(row + 2) {
            for dx in col.saturating_sub(1)..(width - 1).min(col + 2) {
                let c = color(dx, dy);
                mean[c] += input[dy * width + dx].max(0.0);
                count[c] += 1.0;
            }
        }
        let root: [f32; 3] = std::array::from_fn(|c| {
            if count[c] > 0.0 {
                (correction[c] * mean[c] / count[c]).cbrt()
            } else {
                0.0
            }
        });
        let opposing = [
            0.5 * (root[1] + root[2]),
            0.5 * (root[0] + root[2]),
            0.5 * (root[0] + root[1]),
        ];
        opposing[color(col, row)].powi(3)
    };

    let mut sums = [0.0f32; 3];
    let mut counts = [0u32; 3];
    for row in 0..height {
        for col in 0..width {
            let c = color(col, row);
            let value = input[row * width + col];
            // Past the end of a dilated mask lies the next colour's, and past the last
            // one memory darktable does not own, read here as 0.
            let entry = (3 + c) * msize + (row / 3) * mwidth + col / 3;
            let selected = mask.get(entry).copied().unwrap_or(0) != 0;
            if value < clips[c] && value > 0.2 * clips[c] && selected {
                sums[c] += value - refavg(row, col);
                counts[c] += 1;
            }
        }
    }
    let chroma: [f32; 3] = std::array::from_fn(|c| {
        if counts[c] > min_count {
            sums[c] / counts[c] as f32
        } else {
            0.0
        }
    });

    let output = (0..width * height)
        .map(|i| {
            let (row, col) = (i / width, i % width);
            let c = color(col, row);
            let value = input[i];
            if value >= clips[c] {
                value.max(refavg(row, col) + chroma[c])
            } else {
                value
            }
        })
        .collect();
    (output, chroma, counts)
}

/// What the GPU and darktable's loops both give for a scene.
struct Checked {
    balanced: Vec<f32>,
    reconstructed: Vec<f32>,
    expected: Vec<f32>,
    chroma: [f32; 3],
    counts: [u32; 3],
    /// darktable's loops keeping every offset, however few photosites gave it.
    any_count: Vec<f32>,
}

/// Renders `scene` on the GPU and checks every photosite against darktable's loops.
fn check(scene: &Scene, late_correction: bool) -> Checked {
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default()))
        .expect("a GPU adapter, hardware or software");
    check_on(&gpu, scene, late_correction)
}

/// [`check`] on this GPU.
fn check_on(gpu: &GpuContext, scene: &Scene, late_correction: bool) -> Checked {
    let (balanced, reconstructed) = scene.render(gpu, late_correction);
    let [x0, y0, width, height] = scene.crop();
    let clips = COEFFICIENTS.map(|c| 0.987 * c);
    let correction = if late_correction {
        apertrust_color::late_correction(Some(&COEFFICIENTS), &XYZ_TO_CAMERA)
    } else {
        [1.0; 3]
    };
    let reference = |min_count| {
        opposed(
            &balanced,
            width as usize,
            height as usize,
            |col, row| scene.color(x0 + col as u32, y0 + row as u32),
            clips,
            correction,
            min_count,
        )
    };
    let (expected, chroma, counts) = reference(100);
    // Cube roots and the order of the sums differ by rounding.
    for (i, (&actual, &expected)) in reconstructed.iter().zip(&expected).enumerate() {
        assert!(
            (actual - expected).abs() <= 2e-6 * expected.abs().max(1.0),
            "photosite {i} ({}, {}): {actual} against {expected}",
            i % width as usize,
            i / width as usize
        );
    }
    let any_count = reference(0).0;
    Checked {
        balanced,
        reconstructed,
        expected,
        chroma,
        counts,
        any_count,
    }
}

/// How many photosites highlight reconstruction changed.
fn changed(checked: &Checked) -> usize {
    checked
        .expected
        .iter()
        .zip(&checked.balanced)
        .filter(|(after, before)| after != before)
        .count()
}

#[test]
fn inpaint_opposed_matches_darktables_loops_on_bayer() {
    // A mask grid of 48 × 40, both multiples of 8: the last rows read past the end of
    // one dilated mask into the next, as in darktable.
    let bayer = Pattern {
        width: 2,
        height: 2,
        values: BAYER.to_vec(),
    };
    let checked = check(&Scene::busy(bayer, 148, 124), false);
    // The scene clips every colour and gives each an offset.
    assert!(
        checked.counts.iter().all(|&n| n > 100),
        "{:?}",
        checked.counts
    );
    assert!(
        checked.chroma.iter().all(|&c| c != 0.0),
        "{:?}",
        checked.chroma
    );
    assert!(
        changed(&checked) > 200,
        "{} reconstructed",
        changed(&checked)
    );
}

#[test]
fn inpaint_opposed_matches_darktables_loops_on_x_trans_with_the_late_correction() {
    let x_trans = Pattern {
        width: 6,
        height: 6,
        values: X_TRANS.to_vec(),
    };
    let checked = check(&Scene::busy(x_trans, 153, 121), true);
    assert!(
        checked.counts.iter().all(|&n| n > 100),
        "{:?}",
        checked.counts
    );
    assert!(
        checked.chroma.iter().all(|&c| c != 0.0),
        "{:?}",
        checked.chroma
    );
    assert!(
        changed(&checked) > 200,
        "{} reconstructed",
        changed(&checked)
    );
}

#[test]
fn an_offset_needs_more_than_100_photosites() {
    // One small green highlight on a background too dark to count: fewer than 100
    // greens around it give green an offset, which darktable drops. The GPU must drop
    // it too, or the clipped greens differ from darktable's loops.
    let scene = Scene {
        cfa: Pattern {
            width: 2,
            height: 2,
            values: BAYER.to_vec(),
        },
        width: 62,
        height: 52,
        base: 0.1,
        highlights: vec![(31.0, 26.0, 2.5, [0.45, 1.6, 0.45])],
    };
    let checked = check(&scene, false);
    assert!(
        (1..=100).contains(&checked.counts[1]),
        "{:?}",
        checked.counts
    );
    assert_eq!(checked.chroma, [0.0; 3]);
    // Red and blue alone stay below green's clipping level, so nothing changes; with
    // green's offset kept, the clipped greens would rise.
    assert_eq!(changed(&checked), 0);
    assert_ne!(checked.any_count, checked.expected);
}

/// A GPU that binds at most `bytes` of a buffer at once, at offsets aligned to WebGPU's
/// largest alignment, 256 bytes: on any machine, the engine runs as on one that binds
/// less than the image (decision 14).
fn binding_at_most(bytes: u64) -> GpuContext {
    let options = GpuOptions {
        max_storage_binding: Some(bytes),
        storage_offset_alignment: Some(256),
        ..GpuOptions::default()
    };
    let gpu =
        pollster::block_on(GpuContext::new(&options)).expect("a GPU adapter, hardware or software");
    assert_eq!(gpu.device.limits().max_storage_buffer_binding_size, bytes);
    gpu
}

/// How many strips each pass of highlight reconstruction runs in on this GPU.
fn strips(gpu: &GpuContext, scene: &Scene) -> Vec<(&'static str, usize)> {
    let instances: Vec<Instance> = [&Rawprepare as &dyn Operation, &Temperature, &Highlights]
        .into_iter()
        .map(|operation| Instance {
            operation,
            params: ParamValues::new(),
        })
        .collect();
    let input = ImageDesc::new(scene.width, scene.height, Format::Mosaic16);
    let plan = pipeline::plan(
        &instances,
        &scene.image(),
        input,
        Pipe::Export,
        &Limits::of(gpu),
    )
    .unwrap();
    let highlights = &plan.operations[2];
    highlights
        .passes
        .iter()
        .zip(&highlights.dispatches)
        .map(|(pass, dispatches)| (pass.entry_point, dispatches.len()))
        .collect()
}

/// In row strips, every pass that reads the mosaic covers a band of it, the sums'
/// strips are whole rows of workgroups, 64 rows of photosites, and the dilation, which
/// binds only the masks, runs in bands of the masks' rows: the offsets, and every
/// photosite, must be the whole mosaic's.
fn strips_equal_the_whole(scene: &Scene, late_correction: bool) {
    let whole = check(scene, late_correction);
    // The busy scenes' mosaics take about 70 KB; the sums need 66 rows of one at once.
    let gpu = binding_at_most(40 << 10);
    let strips = strips(&gpu, scene);
    for pass in [
        "opposed_mask",
        "opposed_dilate",
        "opposed_sums",
        "opposed_inpaint",
    ] {
        let (_, count) = strips.iter().find(|(name, _)| *name == pass).unwrap();
        assert!(*count > 1, "{strips:?}");
    }
    let banded = check_on(&gpu, scene, late_correction);
    assert_eq!(banded.chroma, whole.chroma);
    assert_eq!(banded.counts, whole.counts);
    if let Some(i) = banded
        .reconstructed
        .iter()
        .zip(&whole.reconstructed)
        .position(|(a, b)| a.to_bits() != b.to_bits())
    {
        panic!(
            "photosite {i}: {} in strips, {} whole",
            banded.reconstructed[i], whole.reconstructed[i]
        );
    }
}

#[test]
fn inpaint_opposed_in_row_strips_is_the_whole_mosaics_on_bayer() {
    let bayer = Pattern {
        width: 2,
        height: 2,
        values: BAYER.to_vec(),
    };
    // The middle highlight crosses row 64, where the sums' strips meet.
    strips_equal_the_whole(&Scene::busy(bayer, 148, 124), false);
}

#[test]
fn inpaint_opposed_in_row_strips_is_the_whole_mosaics_on_x_trans() {
    let x_trans = Pattern {
        width: 6,
        height: 6,
        values: X_TRANS.to_vec(),
    };
    strips_equal_the_whole(&Scene::busy(x_trans, 153, 121), true);
}
