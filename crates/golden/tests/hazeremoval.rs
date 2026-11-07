// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2017-2026  darktable developers (ported from src/iop/hazeremoval.c and
// src/common/guided_filter.c)
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

//! Haze removal on the GPU, needing no darktable (`docs/ports/hazeremoval.md`, Test
//! plan): its passes, run as a one-instance plan of the engine, follow darktable's
//! `process()` restated here, with exact quick selects and the guided filter in f64, at
//! full resolution and at a smaller pipe's windows, on an image with ties, negative
//! values and edges across the passes' tiles; a strength of 0 leaves the image as it
//! was but for one rounding; and the output stays finite for black, negative and very
//! bright images. CI runs them on software GPUs.

use std::sync::OnceLock;

use apertrust_engine::compute::GpuImage;
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::pipeline::{self, Derivative, Instance, Limits, Pipe, Pipeline};
use apertrust_operations::hazeremoval::{self, HazeRemoval, Settings};
use apertrust_plugin::ImageContext;

fn gpu() -> &'static GpuContext {
    static GPU: OnceLock<GpuContext> = OnceLock::new();
    GPU.get_or_init(|| {
        pollster::block_on(GpuContext::new(&GpuOptions::default()))
            .expect("a GPU adapter, hardware or software")
    })
}

/// An image, rows top to bottom.
struct Image {
    width: usize,
    height: usize,
    pixels: Vec<[f32; 3]>,
}

impl Image {
    /// A hazy scene: a bright, low-contrast sky over darker, more saturated ground,
    /// with edges, noise from a fixed sequence, plateaus of equal values for the
    /// selections' ties, and a few negative and -0.0 channels.
    fn hazy(width: usize, height: usize) -> Self {
        Self::scene(width, height, true)
    }

    /// [`Image::hazy`] without its negative and -0.0 channels. They lie in nearly every
    /// box of the dark channel, which takes its critical haze level to 0 or below, the
    /// depth to its largest and t_min to its lowest, so that the distance changes
    /// nothing.
    fn hazy_positive(width: usize, height: usize) -> Self {
        Self::scene(width, height, false)
    }

    fn scene(width: usize, height: usize, signed: bool) -> Self {
        let mut state = 0x2545_f491_u32;
        let mut noise = move || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state >> 8) as f32 / (1 << 24) as f32
        };
        let mut pixels = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                let depth = 1.0 - y as f32 / height as f32;
                let haze = 0.55 * depth;
                let object = if (x / 9 + y / 13) % 3 == 0 { 0.6 } else { 0.15 };
                let mut rgb = [0.8, 0.5, 0.3].map(|c| {
                    let value = haze + (1.0 - depth) * object * c + 0.02 * noise();
                    // Plateaus: values on a coarse grid in a band of rows.
                    if (40..60).contains(&y) {
                        (value * 16.0).round() / 16.0
                    } else {
                        value
                    }
                });
                if signed && (x * 7 + y * 3) % 97 == 0 {
                    rgb[2] = -0.01;
                }
                if signed && (x * 5 + y) % 131 == 0 {
                    rgb[1] = -0.0;
                }
                pixels.push(rgb);
            }
        }
        Self {
            width,
            height,
            pixels,
        }
    }
}

/// Haze removal's plan on the GPU over `image` at the pipe's `scale`, rendered once, as
/// the engine renders it.
fn rendered(image: &Image, settings: Settings, scale: f32) -> Pipeline {
    let gpu = gpu();
    let rgba: Vec<[f32; 4]> = image
        .pixels
        .iter()
        .map(|&[r, g, b]| [r, g, b, 1.0])
        .collect();
    let (width, height) = (image.width as u32, image.height as u32);
    let input = GpuImage::upload(gpu, width, height, &rgba).unwrap();
    let context = ImageContext {
        scale,
        ..ImageContext::full(width, height)
    };
    let instances = [Instance {
        operation: &HazeRemoval,
        params: hazeremoval::values(&settings),
    }];
    let plan = pipeline::plan(
        &instances,
        &context,
        input.desc(),
        Pipe::Export,
        &Limits::of(gpu),
    )
    .unwrap();
    let mut pipeline = pollster::block_on(Pipeline::new(gpu, plan, input)).unwrap();
    pipeline.render(gpu).unwrap();
    pipeline
}

/// Haze removal's output on the GPU over `image` at the pipe's `scale`.
fn render(image: &Image, settings: Settings, scale: f32) -> Vec<[f32; 3]> {
    let gpu = gpu();
    let output = rendered(image, settings, scale)
        .output()
        .download(gpu)
        .unwrap();
    output
        .into_iter()
        .map(|[r, g, b, a]| {
            assert_eq!(a, 1.0, "alpha passes through");
            [r, g, b]
        })
        .collect()
}

/// darktable's `dt_box_min` or `dt_box_max`: the extreme over a box of radius `w`,
/// clipped to the image.
fn box_extreme(plane: &[f32], width: usize, height: usize, w: usize, max: bool) -> Vec<f32> {
    let pick = |a: f32, b: f32| if max { a.max(b) } else { a.min(b) };
    let mut rows = vec![0.0; plane.len()];
    for y in 0..height {
        for x in 0..width {
            let (lo, hi) = (x.saturating_sub(w), (x + w).min(width - 1));
            rows[y * width + x] = (lo + 1..=hi).fold(plane[y * width + lo], |m, xx| {
                pick(m, plane[y * width + xx])
            });
        }
    }
    let mut out = vec![0.0; plane.len()];
    for y in 0..height {
        for x in 0..width {
            let (lo, hi) = (y.saturating_sub(w), (y + w).min(height - 1));
            out[y * width + x] =
                (lo + 1..=hi).fold(rows[lo * width + x], |m, yy| pick(m, rows[yy * width + x]));
        }
    }
    out
}

/// darktable's `dt_box_mean`, in f64: the mean over a box of radius `r` clipped to the
/// image, along the rows, then the columns.
fn box_mean(plane: &[f64], width: usize, height: usize, r: usize) -> Vec<f64> {
    let mut rows = vec![0.0; plane.len()];
    for y in 0..height {
        for x in 0..width {
            let (lo, hi) = (x.saturating_sub(r), (x + r).min(width - 1));
            let sum: f64 = (lo..=hi).map(|xx| plane[y * width + xx]).sum();
            rows[y * width + x] = sum / (hi - lo + 1) as f64;
        }
    }
    let mut out = vec![0.0; plane.len()];
    for y in 0..height {
        for x in 0..width {
            let (lo, hi) = (y.saturating_sub(r), (y + r).min(height - 1));
            let sum: f64 = (lo..=hi).map(|yy| rows[yy * width + x]).sum();
            out[y * width + x] = sum / (hi - lo + 1) as f64;
        }
    }
    out
}

/// darktable's `guided_filter()` of `p` with `guide`, in f64, solving each pixel's
/// regularised 3×3 system directly.
fn guided_filter(guide: &[[f32; 3]], p: &[f32], width: usize, height: usize, r: usize) -> Vec<f64> {
    let eps = f64::from(hazeremoval::eps());
    let mean = |f: &dyn Fn(usize) -> f64| {
        let plane: Vec<f64> = (0..p.len()).map(f).collect();
        box_mean(&plane, width, height, r)
    };
    let g = |i: usize, c: usize| f64::from(guide[i][c]);
    let mean_p = mean(&|i| f64::from(p[i]));
    let mean_i: Vec<Vec<f64>> = (0..3).map(|c| mean(&|i| g(i, c))).collect();
    let mean_ip: Vec<Vec<f64>> = (0..3)
        .map(|c| mean(&|i| g(i, c) * f64::from(p[i])))
        .collect();
    // The guide's products with itself, for a ≤ b; the others stay empty.
    let mean_ii: Vec<Vec<Vec<f64>>> = (0..3)
        .map(|a| {
            (0..3)
                .map(|b| {
                    if a <= b {
                        mean(&|i| g(i, a) * g(i, b))
                    } else {
                        Vec::new()
                    }
                })
                .collect()
        })
        .collect();
    let mut a_planes = vec![vec![0.0; p.len()]; 3];
    let mut b_plane = vec![0.0; p.len()];
    for i in 0..p.len() {
        let sigma = |a: usize, b: usize| {
            let (a, b) = (a.min(b), a.max(b));
            mean_ii[a][b][i] - mean_i[a][i] * mean_i[b][i] + if a == b { eps } else { 0.0 }
        };
        let m = [0, 1, 2].map(|a| [0, 1, 2].map(|b| sigma(a, b)));
        let cov = [0, 1, 2].map(|c| mean_ip[c][i] - mean_i[c][i] * mean_p[i]);
        let det = |m: [[f64; 3]; 3]| {
            m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
                - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
                + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
        };
        let det0 = det(m);
        let mut b = mean_p[i];
        for c in 0..3 {
            let mut column = m;
            for (row, value) in column.iter_mut().zip(cov) {
                row[c] = value;
            }
            let a = det(column) / det0;
            a_planes[c][i] = a;
            b -= a * mean_i[c][i];
        }
        b_plane[i] = b;
    }
    let mean_a: Vec<Vec<f64>> = a_planes
        .iter()
        .map(|plane| box_mean(plane, width, height, r))
        .collect();
    let mean_b = box_mean(&b_plane, width, height, r);
    (0..p.len())
        .map(|i| (0..3).map(|c| mean_a[c][i] * g(i, c)).sum::<f64>() + mean_b[i])
        .collect()
}

/// darktable's `process()` with compatibility mode off, restated: the dark
/// channel, exact quick selects by sorting, A0 in f64, the transmission filtered as
/// darktable filters it (1 − strength × m, its box maximum, then its box minimum), the
/// guided filter in f64, and the haze-free pixels.
fn darktable(image: &Image, settings: Settings, [w1, w2]: [usize; 2]) -> Vec<[f32; 3]> {
    let (width, height) = (image.width, image.height);
    let rgb = &image.pixels;
    let n = rgb.len();
    let smallest = |p: [f32; 3]| p[0].min(p[1]).min(p[2]);
    let brightness = |p: [f32; 3]| p[0] + p[1] + p[2];
    let plain: Vec<f32> = rgb.iter().map(|&p| smallest(p)).collect();
    let dark = box_extreme(&plain, width, height, w1, false);
    let mut sorted = dark.clone();
    sorted.sort_by(f32::total_cmp);
    let critical_haze = sorted[hazeremoval::rank(n as u64) as usize];
    let mut hazy: Vec<f32> = (0..n)
        .filter(|&i| dark[i] >= critical_haze)
        .map(|i| brightness(rgb[i]))
        .collect();
    hazy.sort_by(f32::total_cmp);
    let critical_brightness = hazy[(hazy.len() as f32 * 0.95) as usize];
    let (mut sum, mut count) = ([0.0f64; 3], 0usize);
    for i in 0..n {
        if dark[i] >= critical_haze && brightness(rgb[i]) >= critical_brightness {
            for (s, c) in sum.iter_mut().zip(rgb[i]) {
                *s += f64::from(c);
            }
            count += 1;
        }
    }
    let ambient = sum.map(|s| (s / count as f64) as f32);
    let depth = if critical_haze > 0.0 {
        -1.125 * critical_haze.ln()
    } else {
        f32::MAX.ln() / 2.0
    };
    let inverse = ambient.map(|a| 1.0 / a);
    let transmission: Vec<f32> = rgb
        .iter()
        .map(|p| {
            let m = (p[0] * inverse[0])
                .min(p[1] * inverse[1])
                .min(p[2] * inverse[2]);
            1.0 - m * settings.strength
        })
        .collect();
    let transmission = box_extreme(&transmission, width, height, w1, true);
    let transmission = box_extreme(&transmission, width, height, w1, false);
    let filtered = guided_filter(rgb, &transmission, width, height, w2);
    let t_min = (-settings.distance * depth).exp().clamp(1.0 / 1024.0, 1.0);
    (0..n)
        .map(|i| {
            let t = filtered[i].max(f64::from(t_min)) as f32;
            [0, 1, 2].map(|c| (rgb[i][c] - ambient[c]) / t + ambient[c])
        })
        .collect()
}

/// The largest difference between `actual` and `expected`, relative to the expected
/// value where it exceeds 1.
fn largest_difference(actual: &[[f32; 3]], expected: &[[f32; 3]]) -> f32 {
    actual
        .iter()
        .zip(expected)
        .flat_map(|(a, e)| (0..3).map(move |c| (a[c] - e[c]).abs() / e[c].abs().max(1.0)))
        .fold(0.0, f32::max)
}

#[test]
fn the_passes_follow_darktables_process() {
    // 70 × 300: many of the guided filter's 16×16 tiles, partial ones at the right and
    // the bottom, and many workgroups of the counts.
    let image = Image::hazy(70, 300);
    for (strength, distance) in [(0.2, 0.2), (0.9, 0.8), (-0.5, 0.4), (1.0, 1.0)] {
        let settings = Settings { strength, distance };
        let actual = render(&image, settings, 1.0);
        let expected = darktable(&image, settings, [6, 9]);
        let difference = largest_difference(&actual, &expected);
        // The settings move the image by 0.06 to 0.56; the GPU stays within about 1e-6
        // of the restatement on the M4.
        assert!(largest_difference(&expected, &image.pixels) > 0.05);
        assert!(
            difference < 2e-5,
            "strength {strength}, distance {distance}: {difference}"
        );
    }
}

#[test]
fn a_smaller_pipe_scales_the_windows() {
    // At a pipe's scale of a half the windows are 3 and 5 pixels, round(6 / 2) and
    // round(9 / 2): the same processing as darktable's at those windows.
    let image = Image::hazy(97, 61);
    let settings = Settings {
        strength: 0.7,
        distance: 0.5,
    };
    let actual = render(&image, settings, 0.5);
    let expected = darktable(&image, settings, [3, 5]);
    let difference = largest_difference(&actual, &expected);
    assert!(difference < 2e-5, "{difference}");
}

#[test]
fn strength_zero_changes_each_value_by_one_rounding_at_most() {
    let image = Image::hazy(70, 60);
    let settings = Settings {
        strength: 0.0,
        distance: 0.6,
    };
    let actual = render(&image, settings, 1.0);
    for (a, e) in actual.iter().zip(&image.pixels) {
        for c in 0..3 {
            // (I - A0) + A0: one rounding at A0's magnitude, under 1.
            assert!((a[c] - e[c]).abs() <= f32::EPSILON, "{a:?} from {e:?}");
        }
    }
}

/// The engine's downscale: output pixel `o` of `output` covers the source pixels from
/// `o × source / output` up to `(o + 1) × source / output`, and averages them.
fn downscale(image: &Image, width: usize, height: usize) -> Image {
    let bounds = |o: usize, out: usize, source: usize| (o * source / out, (o + 1) * source / out);
    let mut pixels = Vec::with_capacity(width * height);
    for oy in 0..height {
        let (y0, y1) = bounds(oy, height, image.height);
        for ox in 0..width {
            let (x0, x1) = bounds(ox, width, image.width);
            let mut sum = [0.0f64; 3];
            for y in y0..y1 {
                for x in x0..x1 {
                    for (s, c) in sum.iter_mut().zip(image.pixels[y * image.width + x]) {
                        *s += f64::from(c);
                    }
                }
            }
            let n = ((y1 - y0) * (x1 - x0)) as f64;
            pixels.push(sum.map(|s| (s / n) as f32));
        }
    }
    Image {
        width,
        height,
        pixels,
    }
}

fn mean_delta_e(expected: &Image, actual: &Image) -> f64 {
    let image = |image: &Image| {
        golden::pfm::Image::new(image.width, image.height, image.pixels.clone()).unwrap()
    };
    let (_, stats) = golden::delta_e::compare(
        &image(expected),
        &image(actual),
        golden::delta_e::Tolerance::DARKTABLE,
    )
    .unwrap();
    stats.mean
}

/// Haze removal's one-instance plan in the viewport over `image`, fitted in `bounds`, as
/// the session renders it: the viewport downscales the image by its box filter, and haze
/// removal's whole-image results come from the image at full resolution (decision 15).
fn viewport(image: &Image, settings: Settings, bounds: [u32; 2]) -> Image {
    let gpu = gpu();
    let rgba: Vec<[f32; 4]> = image
        .pixels
        .iter()
        .map(|&[r, g, b]| [r, g, b, 1.0])
        .collect();
    let (width, height) = (image.width as u32, image.height as u32);
    let input = GpuImage::upload(gpu, width, height, &rgba).unwrap();
    let instances = [Instance {
        operation: &HazeRemoval,
        params: hazeremoval::values(&settings),
    }];
    let pipe = Pipe::Viewport {
        width: bounds[0],
        height: bounds[1],
    };
    let context = ImageContext::full(width, height);
    let plan = pipeline::plan(&instances, &context, input.desc(), pipe, &Limits::of(gpu)).unwrap();
    let haze = plan
        .operations
        .iter()
        .find(|op| op.id == "hazeremoval")
        .unwrap();
    assert!(
        haze.full_resolution.is_some(),
        "the estimate at full resolution"
    );
    let mut pipeline = pollster::block_on(Pipeline::new(gpu, plan, input)).unwrap();
    pipeline.render(gpu).unwrap();
    let output = pipeline.output();
    let desc = output.desc();
    Image {
        width: desc.width as usize,
        height: desc.height as usize,
        pixels: output
            .download(gpu)
            .unwrap()
            .into_iter()
            .map(|[r, g, b, _]| [r, g, b])
            .collect(),
    }
}

/// The viewport's boxes decision 15 measures at: 2400×1600, and a half, a quarter and an
/// eighth of the image, with its target at each, 0.15, 0.15, 0.4 and 0.4.
fn boxes(width: usize, height: usize) -> [([u32; 2], f64); 4] {
    let part = |n: usize| [(width / n) as u32, (height / n) as u32];
    [
        ([2400, 1600], 0.15),
        (part(2), 0.15),
        (part(4), 0.4),
        (part(8), 0.4),
    ]
}

/// The viewport against the export downscaled at each of decision 15's boxes: the mean
/// ΔE2000 of the viewport's frame against the export's downscaled by the same box filter,
/// and of the viewport as it was before decision 15, estimating at its own scale.
///
/// Decision 15's baseline is the viewport with every value a pixel reads from other
/// pixels computed at full resolution and downscaled by the viewport's box filter, and
/// every per-pixel step at the viewport's scale: for haze removal, whose whole-image
/// passes give only such values (A0, the depth, the filtered transmission), that is the
/// viewport as decision 15 builds it, so the filtering's share, the part decision 15's
/// target applies to, is 0 by construction. The gap measured is the per-pixel gap
/// itself: `main`'s at the viewport's scale.
fn viewport_against_export(full: &Image, settings: Settings) -> Vec<([u32; 2], f64, f64, f64)> {
    let export = Image {
        width: full.width,
        height: full.height,
        pixels: render(full, settings, 1.0),
    };
    boxes(full.width, full.height)
        .into_iter()
        .map(|(bounds, target)| {
            let frame = viewport(full, settings, bounds);
            let expected = downscale(&export, frame.width, frame.height);
            let gap = mean_delta_e(&expected, &frame);
            let scale = frame.width as f32 / full.width as f32;
            let small = downscale(full, frame.width, frame.height);
            let before = Image {
                width: frame.width,
                height: frame.height,
                pixels: render(&small, settings, scale),
            };
            let at_its_scale = mean_delta_e(&expected, &before);
            (
                [frame.width as u32, frame.height as u32],
                gap,
                at_its_scale,
                target,
            )
        })
        .collect()
}

/// The viewport against the export downscaled (decision 15's measure and
/// baseline; [`viewport_against_export`]) on the golden cases' images. Before decision
/// 15 the viewport estimated at its own scale, 0.126 to 0.434 mean ΔE2000 from the export
/// downscaled at test 0026's strength of 0.2 and 1.333 to 4.637 at test 0166's 0.9; now it
/// estimates at full resolution, and the first measurement on the M4 is the baseline: a
/// rise above it by more than 0.02 is a regression (`docs/ports/hazeremoval.md`,
/// Neighbourhood and scale).
#[test]
#[ignore = "needs the golden cases' references: darktable-cli and APERTRUST_TEST_IMAGES"]
fn the_viewport_follows_the_export_downscaled() {
    let darktable = golden::darktable::DarktableCli::locate().unwrap();
    let images = golden::render::images_dir().unwrap();
    // At 2400×1600, a half, a quarter and an eighth, on the M4.
    let measured = [
        ("hazeremoval", [0.0005, 0.0008, 0.0025, 0.0064]),
        ("hazeremoval-v2", [0.0052, 0.0079, 0.0234, 0.0540]),
    ];
    let mut over = Vec::new();
    for (name, baselines) in measured {
        let case = golden::cases::find(name).unwrap();
        let references = golden::render::render_case(&darktable, case, &images).unwrap();
        let input = golden::pfm::Image::read(&references.input).unwrap();
        let full = Image {
            width: input.width,
            height: input.height,
            pixels: input.pixels,
        };
        let values = golden::ports::plan_values(case).unwrap().unwrap();
        let settings = hazeremoval::settings(&values);
        for (([width, height], gap, before, target), measured) in
            viewport_against_export(&full, settings)
                .into_iter()
                .zip(baselines)
        {
            eprintln!(
                "{name} at {width}×{height}: viewport {gap:.4} against {target} (share 0), \
                 at its own scale before {before:.4}"
            );
            if gap > measured + 0.02 {
                over.push(format!("{name} at {width}×{height}"));
            }
        }
    }
    assert!(over.is_empty(), "over the measured baseline: {over:?}");
}

/// The same on the raw.pixls.us corpus, as soften's viewport is measured: each raw
/// developed up to haze removal by an export, the raw stage and demosaic, then haze
/// removal at test 0166's settings, its export against its viewport at decision 15's
/// boxes. Printed, raw by raw, with the gaps over decision 15's target counted; the
/// share the target applies to is 0 by construction ([`viewport_against_export`]).
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn the_viewport_follows_the_export_downscaled_on_the_corpus() {
    use std::cmp::Ordering;

    use apertrust_engine::edit::{self, Edit};
    use apertrust_engine::export::{BUDGET, Tiling, export};
    use apertrust_plugin::Operation;

    let corpus = golden::render::corpus_dir().unwrap();
    let names = include_str!("../../raw/tests/corpus/files.sha256")
        .lines()
        .filter_map(|line| line.split_once("  ").map(|(_, name)| name.trim()));
    let settings = Settings {
        strength: 0.9,
        distance: 0.8,
    };
    let place = HazeRemoval.manifest().slot;
    let (mut measured, mut worst, mut over) = (0, [0.0f64; 4], Vec::new());
    for name in names {
        let raw = match apertrust_raw::decode_file(corpus.join(name)) {
            Ok(raw) => raw,
            Err(error) => {
                eprintln!("{name}: skipped, failed decoding: {error}");
                continue;
            }
        };
        let Some(mosaic) = apertrust_engine::raw::upload(gpu(), &raw) else {
            eprintln!("{name}: skipped, not a colour mosaic the pipeline takes");
            continue;
        };
        let context = apertrust_engine::raw::image_context(&raw);
        let before: Vec<Instance> = edit::instances(
            &Edit::default(),
            apertrust_operations::OPERATIONS,
            apertrust_modules::MODULES,
            &context,
        )
        .all()
        .unwrap()
        .into_iter()
        .filter(|i| i.operation.manifest().slot.pipeline_order(&place) == Ordering::Less)
        .collect();
        let tiling = Tiling::Auto { budget: BUDGET };
        let developed =
            pollster::block_on(export(gpu(), &before, &context, mosaic.unwrap(), tiling)).unwrap();
        let full = Image {
            width: developed.desc.width as usize,
            height: developed.desc.height as usize,
            pixels: developed
                .pixels
                .iter()
                .map(|&[r, g, b, _]| [r, g, b])
                .collect(),
        };
        drop(developed);
        let scales = viewport_against_export(&full, settings);
        for (k, ([width, height], gap, before, target)) in scales.into_iter().enumerate() {
            eprintln!(
                "{name} at {width}×{height}: viewport {gap:.4}, at its own scale {before:.4}"
            );
            assert!(gap.is_finite(), "{name}");
            worst[k] = worst[k].max(gap);
            if gap > target {
                over.push(format!(
                    "{name} at {width}×{height}: {gap:.4} against {target}"
                ));
            }
        }
        measured += 1;
    }
    eprintln!(
        "measured on {measured} raws; the largest gaps {:.4}, {:.4}, {:.4} and {:.4}; {} over the target: {over:#?}",
        worst[0],
        worst[1],
        worst[2],
        worst[3],
        over.len()
    );
    assert!(over.is_empty(), "{over:#?}");
}

/// CI's check of the viewport's full-resolution path (decision 15), on the hazy scene at
/// test 0166's strength: the viewport within `BOUND` mean ΔE2000 of the export downscaled
/// at a quarter of the image, where the estimate at the viewport's own scale misses it,
/// so a wrong footprint in `plane_downscale`, a wrong width in `main` or the estimate
/// back at the pipe's scale fails here, on every GPU CI runs.
#[test]
fn the_viewport_follows_the_export_downscaled_on_a_hazy_scene() {
    // Measured on the M4: the viewport 0.0089, its own scale's estimate 12.09.
    const BOUND: f64 = 0.1;
    let full = Image::hazy(320, 240);
    let settings = Settings {
        strength: 0.9,
        distance: 0.8,
    };
    let export = Image {
        width: full.width,
        height: full.height,
        pixels: render(&full, settings, 1.0),
    };
    let frame = viewport(&full, settings, [80, 60]);
    assert_eq!((frame.width, frame.height), (80, 60));
    let expected = downscale(&export, frame.width, frame.height);
    let gap = mean_delta_e(&expected, &frame);
    let small = downscale(&full, frame.width, frame.height);
    let before = Image {
        width: frame.width,
        height: frame.height,
        pixels: render(&small, settings, 0.25),
    };
    let at_its_scale = mean_delta_e(&expected, &before);
    eprintln!("the hazy scene at 80×60: viewport {gap:.4}, at its own scale {at_its_scale:.4}");
    assert!(
        gap <= BOUND,
        "the viewport is {gap:.4} from the export downscaled"
    );
    assert!(
        at_its_scale > BOUND,
        "the estimate at the viewport's scale, {at_its_scale:.4}, would pass too"
    );
}

#[test]
fn output_stays_finite() {
    let images = [
        Image {
            width: 40,
            height: 30,
            pixels: vec![[0.0; 3]; 1200],
        },
        Image {
            width: 40,
            height: 30,
            pixels: vec![[-0.2, -0.1, -0.3]; 1200],
        },
        Image {
            width: 40,
            height: 30,
            pixels: (0..1200)
                .map(|i| [1e4, 2e3, 1e4].map(|c| c * (1.0 + (i % 7) as f32)))
                .collect(),
        },
    ];
    for image in &images {
        for (strength, distance) in [(1.0, 1.0), (-1.0, 0.0), (0.2, 0.2)] {
            let settings = Settings { strength, distance };
            let actual = render(image, settings, 1.0);
            assert!(
                actual.iter().flatten().all(|v| v.is_finite()),
                "{:?} at {strength}, {distance}: {:?}",
                image.pixels[0],
                actual[0]
            );
        }
    }
}

/// The derivatives of a loss weighting the channels unequally, `Σ (0.25 r + 0.5 g + b)`
/// over the image, for strength and distance: reverse and forward mode of `main` over
/// the scratch a render at `settings` left, and central finite differences of whole
/// renders, as the golden check takes them with an equal weighting.
fn weighted_gradients(image: &Image, settings: Settings, step: f32) -> [(f64, f64, f64); 2] {
    const WEIGHTS: [f32; 3] = [0.25, 0.5, 1.0];
    let gpu = gpu();
    let (width, height) = (image.width as u32, image.height as u32);
    let pixels = image.pixels.len();
    let loss = |output: &[[f32; 4]]| -> f64 {
        output
            .iter()
            .map(|p| (0..3).map(|c| f64::from(WEIGHTS[c] * p[c])).sum::<f64>())
            .sum()
    };
    let mut pipeline = rendered(image, settings, 1.0);
    let out = GpuImage::new(gpu, width, height);
    let weights = vec![[WEIGHTS[0], WEIGHTS[1], WEIGHTS[2], 0.0]; pixels];
    let weights = GpuImage::upload(gpu, width, height, &weights).unwrap();
    pollster::block_on(pipeline.derivative(gpu, Derivative::Backward, &weights, &out)).unwrap();
    let reverse = out.download(gpu).unwrap();
    [0, 1].map(|slot| {
        let reverse: f64 = reverse.iter().map(|g| f64::from(g[slot])).sum();
        let mut direction = [0.0; 4];
        direction[slot] = 1.0;
        let directions = GpuImage::upload(gpu, width, height, &vec![direction; pixels]).unwrap();
        pollster::block_on(pipeline.derivative(gpu, Derivative::ForwardDiff, &directions, &out))
            .unwrap();
        let forward = loss(&out.download(gpu).unwrap());
        let moved = |delta: f32| {
            let mut moved = settings;
            match slot {
                0 => moved.strength += delta,
                _ => moved.distance += delta,
            }
            let output = rendered(image, moved, 1.0).output().download(gpu).unwrap();
            loss(&output)
        };
        let numeric = (moved(step) - moved(-step)) / (2.0 * f64::from(step));
        (reverse, forward, numeric)
    })
}

#[test]
fn derivatives_match_finite_differences_for_both_parameters() {
    // Test 0166's settings and a lighter strength nearly as far, where the sky's
    // transmission falls below t_min, so the distance's derivative is not zero; and a
    // negative strength, which adds haze and leaves every transmission above 1, so the
    // distance's is zero. The finite differences' steps stay inside the schema's ranges.
    let image = Image::hazy_positive(70, 300);
    for (strength, distance, distance_seen) in
        [(0.9, 0.8, true), (0.7, 0.95, true), (-0.5, 0.4, false)]
    {
        let settings = Settings { strength, distance };
        let gradients = weighted_gradients(&image, settings, 1e-3);
        for (name, (reverse, forward, numeric)) in ["strength", "distance"].iter().zip(gradients) {
            let scale = numeric.abs().max(reverse.abs());
            // Shown with --nocapture, for the spec's Gradients section.
            println!(
                "{strength}, {distance}: {name}: reverse {reverse:.6e}, finite differences \
                 {numeric:.6e} ({:.1e}), forward {forward:.6e} ({:.1e})",
                (reverse - numeric).abs() / scale.max(f64::MIN_POSITIVE),
                (reverse - forward).abs() / scale.max(f64::MIN_POSITIVE)
            );
            assert!(
                (reverse - numeric).abs() <= 1e-2 * scale,
                "{name} at {settings:?}: reverse {reverse}, finite differences {numeric}"
            );
            assert!(
                (reverse - forward).abs() <= 1e-4 * scale,
                "{name} at {settings:?}: reverse {reverse}, forward {forward}"
            );
        }
        let (distance_reverse, _, distance_numeric) = gradients[1];
        assert_eq!(
            distance_numeric != 0.0 && distance_reverse != 0.0,
            distance_seen,
            "the distance's derivative at {settings:?}"
        );
        assert!(gradients[0].2.abs() > 1.0, "the strength's at {settings:?}");
    }
}
