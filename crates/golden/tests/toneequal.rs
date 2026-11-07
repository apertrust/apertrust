// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2018-2026  darktable developers (ported from src/iop/toneequal.c and
// src/common/luminance_mask.h)
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

//! Invariants of the tone equalizer on the GPU, needing no darktable
//! (`docs/ports/toneequal.md`, Test plan): darktable's ripple at zero bands, colour
//! ratios, extreme input, every estimator against darktable's C restated here, the
//! derivatives against finite differences of the curve in f64, and EIGF's whole-image
//! passes against darktable's loops restated in `toneequal::filters`, untiled and in an
//! export's two rounds of tiles. CI runs them on software GPUs.

use std::sync::{Mutex, OnceLock};

use apertrust_engine::compute::GpuImage;
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::pipeline::{self, Derivative, Instance, Limits, Pipe, Pipeline};
use apertrust_operations::toneequal;
use apertrust_plugin::{ImageContext, ImageDesc};
use toneequal::{Estimator, Filter, Geometry, Prepared, Settings};

/// The GPU, and one pipeline per image the tests render, each kept for every test in
/// this file, so its kernels compile once: compiling takes tens of seconds on a software
/// GPU. A pipeline takes each test's settings as a new plan over the same input.
struct Gpu {
    context: GpuContext,
    pipelines: Mutex<Vec<Held>>,
}

/// A pipeline over an image, `size` across and down.
struct Held {
    pixels: Vec<[f32; 4]>,
    size: [u32; 2],
    pipeline: Pipeline,
}

fn gpu() -> &'static Gpu {
    static GPU: OnceLock<Gpu> = OnceLock::new();
    GPU.get_or_init(|| Gpu {
        context: pollster::block_on(GpuContext::new(&GpuOptions::default()))
            .expect("a GPU adapter, hardware or software"),
        pipelines: Mutex::new(Vec::new()),
    })
}

/// The tone equalizer alone over `size` with these settings, the export pipe at full
/// resolution: the one-instance plan the golden harness runs (decision 15).
fn plan(settings: &Settings, size: [u32; 2]) -> pipeline::Plan {
    let [width, height] = size;
    let instances = [Instance {
        operation: &toneequal::ToneEqual,
        params: toneequal::values(settings),
    }];
    let image = ImageContext::full(width, height);
    let limits = Limits::of(&gpu().context);
    pipeline::plan(
        &instances,
        &image,
        ImageDesc::rgba(width, height),
        Pipe::Export,
        &limits,
    )
    .unwrap()
}

/// Runs `then` on the pipeline over `pixels`, `size` across and down, planned for these
/// settings and rendered.
fn with_pipeline<T>(
    settings: &Settings,
    pixels: &[[f32; 4]],
    size: [u32; 2],
    then: impl FnOnce(&GpuContext, &mut Pipeline) -> T,
) -> T {
    let shared = gpu();
    let gpu = &shared.context;
    let mut pipelines = shared.pipelines.lock().unwrap_or_else(|e| e.into_inner());
    let held = pipelines
        .iter()
        .position(|held| (held.pixels.as_slice(), held.size) == (pixels, size));
    let held = match held {
        Some(index) => {
            let held = pipelines.swap_remove(index);
            let pipeline =
                pollster::block_on(held.pipeline.update(gpu, plan(settings, size))).unwrap();
            Held { pipeline, ..held }
        }
        None => {
            let input = GpuImage::upload(gpu, size[0], size[1], pixels).unwrap();
            let pipeline =
                pollster::block_on(Pipeline::new(gpu, plan(settings, size), input)).unwrap();
            Held {
                pixels: pixels.to_vec(),
                size,
                pipeline,
            }
        }
    };
    pipelines.push(held);
    let pipeline = &mut pipelines.last_mut().expect("just pushed").pipeline;
    pipeline.render(gpu).unwrap();
    then(gpu, pipeline)
}

fn rgba(pixels: &[[f32; 3]]) -> Vec<[f32; 4]> {
    // Alpha varies, so it is seen to pass through.
    pixels
        .iter()
        .enumerate()
        .map(|(i, &[r, g, b])| [r, g, b, (i % 7) as f32 / 6.0])
        .collect()
}

/// Every pixel the per-pixel tests render, in one row, the sweep first: one pipeline
/// for all of them.
fn row() -> &'static [[f32; 4]] {
    static ROW: OnceLock<Vec<[f32; 4]>> = OnceLock::new();
    ROW.get_or_init(|| rgba(&[sweep(), extreme(), greys()].concat()))
}

/// Where `pixels` lie in the row.
fn in_row(pixels: &[[f32; 3]]) -> std::ops::Range<usize> {
    let rgb: Vec<[f32; 3]> = row().iter().map(|&[r, g, b, _]| [r, g, b]).collect();
    let start = (0..=rgb.len() - pixels.len())
        .find(|&start| rgb[start..start + pixels.len()] == *pixels)
        .expect("pixels of the row");
    start..start + pixels.len()
}

fn row_size() -> [u32; 2] {
    [u32::try_from(row().len()).unwrap(), 1]
}

/// The output for `pixels`, part of the row, rendered with these settings.
fn render(settings: &Settings, pixels: &[[f32; 3]]) -> Vec<[f32; 4]> {
    let range = in_row(pixels);
    with_pipeline(settings, row(), row_size(), |gpu, pipeline| {
        pipeline.output().download(gpu).unwrap()[range].to_vec()
    })
}

/// A derivative entry point for `pixels`, part of the row, with the same
/// `derivative_in` for each pixel.
fn derivative(
    mode: Derivative,
    settings: &Settings,
    pixels: &[[f32; 3]],
    d_in: [f32; 4],
) -> Vec<[f32; 4]> {
    let range = in_row(pixels);
    let [width, height] = row_size();
    with_pipeline(settings, row(), row_size(), |gpu, pipeline| {
        let d_in = GpuImage::upload(gpu, width, height, &vec![d_in; row().len()]).unwrap();
        let d_out = GpuImage::new(gpu, width, height);
        pollster::block_on(pipeline.derivative(gpu, mode, &d_in, &d_out)).unwrap();
        d_out.download(gpu).unwrap()[range].to_vec()
    })
}

/// The curve and the mask's settings, for the curve, which the image does not change.
fn prepare(settings: &Settings) -> Prepared {
    toneequal::prepare(settings, &Geometry::full(1, 1)).unwrap()
}

fn per_pixel(bands: [f32; 9], method: Estimator) -> Settings {
    Settings {
        bands,
        details: Filter::None,
        method,
        ..Settings::defaults()
    }
}

/// Greys, colours, out-of-gamut colours and a channel at zero, from below the curve's
/// range (2^-8) to above it, and very bright.
fn sweep() -> Vec<[f32; 3]> {
    let mut pixels = Vec::new();
    for level in [
        0.0, 1e-6, 1e-3, 0.0039, 0.01, 0.03, 0.07, 0.18, 0.4, 0.75, 1.0, 1.7, 4.0, 60.0,
    ] {
        for [r, g, b] in [
            [1.0, 1.0, 1.0],
            [1.0, 0.5, 0.2],
            [0.1, 1.0, 0.3],
            [0.2, 0.3, 1.0],
            [1.0, -0.05, 0.4],
            [0.0, 0.6, 1.0],
            [-0.3, -0.2, -0.5],
        ] {
            pixels.push([r * level, g * level, b * level]);
        }
    }
    pixels
}

/// Zero, tiny, huge and negative pixels.
fn extreme() -> Vec<[f32; 3]> {
    let mut pixels = Vec::new();
    for v in [0.0f32, 1e-38, 1e-30, 1e-12, 1e6, 1e12, 1e20, 1e30, 3e37] {
        for [r, g, b] in [
            [1.0, 1.0, 1.0],
            [1.0, 0.0, 0.0],
            [1.0, -1.0, 0.5],
            [-1.0, -1.0, -1.0],
        ] {
            pixels.push([r * v, g * v, b * v]);
        }
    }
    pixels
}

/// Greys from -9 to +1 EV of the mask, and colours.
fn greys() -> Vec<[f32; 3]> {
    (0..=100)
        .flat_map(|i| {
            let v = (-9.0 + i as f32 / 10.0).exp2();
            [[v, v, v], [v, 0.4 * v, 0.1 * v]]
        })
        .collect()
}

/// Every estimator, three mask exposures and the smoothing across its range.
fn every_mode(bands: [f32; 9]) -> Vec<Settings> {
    let mut all = Vec::new();
    for method in Estimator::ALL {
        for exposure_boost in [-3.0, 0.0, 2.0] {
            for smoothing in [
                toneequal::SMOOTHING_MIN,
                1.0,
                std::f32::consts::SQRT_2,
                2.0,
                toneequal::SMOOTHING_MAX,
            ] {
                all.push(Settings {
                    exposure_boost,
                    smoothing,
                    ..per_pixel(bands, method)
                });
            }
        }
    }
    all
}

/// darktable's estimators (`src/common/luminance_mask.h`) with no contrast
/// compensation, in f32 as darktable computes them, then `apply_toneequalizer`'s
/// exposure: the mask's log2 clamped to the curve's range. A 0 / 0 in the power norm is
/// NaN, which loses `MAX`'s comparison and becomes the floor, as in darktable.
fn darktable_exposure(rgb: [f32; 3], method: Estimator, exposure_boost: f32) -> f32 {
    let [r, g, b] = rgb;
    let e = exposure_boost.exp2();
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let luminance = match method {
        Estimator::Average => e * (r + g + b) / 3.0,
        Estimator::Lightness => e * (max + min) / 2.0,
        Estimator::Value => e * max,
        Estimator::Sum => e * (r.abs() + g.abs() + b.abs()),
        Estimator::EuclideanNorm => e * (r * r + g * g + b * b).sqrt(),
        Estimator::PowerNorm => {
            let [r, g, b] = [r.abs(), g.abs(), b.abs()];
            e * (r * r * r + g * g * g + b * b * b) / (r * r + g * g + b * b)
        }
        Estimator::GeometricMean => e * (r.abs() * g.abs() * b.abs()).powf(1.0 / 3.0),
    };
    let floor = (-16.0f32).exp2();
    let mask = if luminance > floor { luminance } else { floor };
    mask.log2().clamp(-8.0, 0.0)
}

/// Test 0131's bands, which the golden cases use.
const TEST_0131: [f32; 9] = [
    -1.283, -0.788, 0.445, 1.148, 1.321, 0.994, 0.211, -0.449, -0.755,
];

/// f32 rounding of a sum of about twenty products, relative to the size of its terms:
/// 32 units in the last place.
const ROUNDING: f64 = 32.0 * f32::EPSILON as f64;

/// `Σ_j |w_j| G_j` at a mask exposure: the size of the terms the kernel adds up with
/// weights `w`. The weights of the fit cancel, more as the smoothing grows, so f32
/// rounding is relative to them, not to the result.
fn rounding_scale(prepared: &Prepared, weights: &[f32; 8], exposure: f64) -> f64 {
    toneequal::CURVE_CENTRES
        .iter()
        .zip(weights)
        .map(|(&centre, &weight)| {
            let d = exposure - f64::from(centre);
            f64::from(weight).abs() * (-d * d * f64::from(prepared.inverse_denominator)).exp()
        })
        .sum()
}

#[test]
fn every_band_at_zero_ripples_as_darktables() {
    // darktable's curve, not an identity (decision 22): each pixel takes the zero
    // bands' ripple at its mask, 0.971 to 1.066 over the smoothing range (setup.rs's
    // tests), as the setup's curve in f64 gives it. Within 2e-5, as for any bands
    // (every_estimator_follows_darktables), and f32 rounding of the terms the curve
    // adds up, which cancel more as the smoothing grows: the M4 gives at most 2.3e-6
    // relative, 3.1 ε of the terms.
    let pixels = sweep();
    for settings in every_mode([0.0; 9]) {
        let prepared = prepare(&settings);
        let out = render(&settings, &pixels);
        for (input, out) in rgba(&pixels).iter().zip(&out) {
            let rgb = [input[0], input[1], input[2]];
            let exposure = f64::from(darktable_exposure(
                rgb,
                settings.method,
                settings.exposure_boost,
            ));
            let correction = prepared.correction(exposure);
            let terms = rounding_scale(&prepared, &prepared.weights, exposure);
            for c in 0..3 {
                let expected = f64::from(input[c]) * correction;
                let gap = (f64::from(out[c]) - expected).abs();
                assert!(
                    gap <= (2e-5 * correction + ROUNDING * terms) * f64::from(input[c]).abs(),
                    "{settings:?}: {input:?} gave {out:?}, expected {correction} times"
                );
            }
            assert_eq!(out[3], input[3]);
        }
    }
}

#[test]
fn colour_ratios_are_kept() {
    let pixels = sweep();
    for settings in every_mode(TEST_0131) {
        let out = render(&settings, &pixels);
        for (input, out) in pixels.iter().zip(&out) {
            // One gain for every channel: the brightest channel's.
            let k = (0..3)
                .max_by(|&a, &b| input[a].abs().total_cmp(&input[b].abs()))
                .unwrap();
            if input[k] == 0.0 {
                assert_eq!(&out[..3], &[0.0; 3]);
                continue;
            }
            let gain = out[k] / input[k];
            assert!((0.25..=4.0).contains(&gain), "{gain}");
            for c in 0..3 {
                assert!(
                    (out[c] - gain * input[c]).abs() <= 1e-6 * input[k].abs() * gain,
                    "{input:?} gave {out:?} for {settings:?}"
                );
            }
        }
    }
}

#[test]
fn output_is_finite_on_extreme_input() {
    let pixels = extreme();
    for bands in [
        [2.0; 9],
        [-2.0; 9],
        TEST_0131,
        [2.0, -2.0, 2.0, -2.0, 2.0, -2.0, 2.0, -2.0, 2.0],
    ] {
        for mut settings in every_mode(bands) {
            for exposure_boost in [16.0, -16.0] {
                settings.exposure_boost = exposure_boost;
                for out in render(&settings, &pixels) {
                    assert!(
                        out.iter().all(|c| c.is_finite()),
                        "{out:?} for {settings:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn every_estimator_follows_darktables() {
    // The kernel's correction against the setup's own curve in f64 at darktable's mask
    // exposure, restated from its C: the estimators are what the golden cases (RGB max
    // and the geometric mean) check only in part.
    let pixels = sweep();
    for method in Estimator::ALL {
        for exposure_boost in [-3.0f32, 0.0, 0.13, 2.0] {
            let settings = Settings {
                exposure_boost,
                ..per_pixel(TEST_0131, method)
            };
            let prepared = prepare(&settings);
            let out = render(&settings, &pixels);
            for (input, out) in pixels.iter().zip(&out) {
                let exposure = darktable_exposure(*input, method, exposure_boost);
                let correction = prepared.correction(f64::from(exposure));
                for c in 0..3 {
                    let expected = f64::from(input[c]) * correction;
                    let gap = (f64::from(out[c]) - expected).abs();
                    assert!(
                        gap <= 2e-5 * expected.abs(),
                        "{method:?}, {exposure_boost} EV: {input:?} gave {out:?}, expected {} times",
                        correction
                    );
                }
            }
        }
    }
}

/// darktable's curve in f64 throughout, for bands moved by `delta` EV along
/// `direction`: the weights fitted in f64 and never rounded to f32, so its central
/// differences are exact to about 1e-9 where the kernel's f32 output, whose weights
/// cancel (±70 at σ = 2), would add rounding of the size of the derivative's change.
fn curve(settings: &Settings, direction: &[f32; 9], delta: f64, exposure: f64) -> f64 {
    let sigma = f64::from(settings.smoothing);
    let gains: [f64; 9] = std::array::from_fn(|i| {
        (f64::from(settings.bands[i]) + delta * f64::from(direction[i])).exp2()
    });
    let weights = toneequal::fit(sigma, &gains).expect("the smoothing range fits");
    let sum: f64 = toneequal::CURVE_CENTRES
        .iter()
        .zip(weights)
        .map(|(&centre, weight)| {
            let d = exposure - f64::from(centre);
            weight * (-d * d / (2.0 * sigma * sigma)).exp()
        })
        .sum();
    sum.clamp(0.25, 4.0)
}

#[test]
fn derivatives_agree_with_finite_differences() {
    // Greys from -9 to +1 EV of the mask, and colours, without darktable, so on CI's
    // software GPUs too: forward mode against central differences of the curve in f64
    // (curve), and reverse mode against forward mode. The golden cases' gradient check
    // compares reverse mode with the kernel's own finite differences.
    let pixels = greys();
    let bands = [0.3, -0.5, 0.8, 1.1, -0.2, 0.6, -1.0, 0.4, 0.9];
    let h = 1e-4;
    for smoothing in [1.0, std::f32::consts::SQRT_2, 2.0] {
        let settings = Settings {
            smoothing,
            ..per_pixel(bands, Estimator::EuclideanNorm)
        };
        let prepared = prepare(&settings);
        let reverse = derivative(Derivative::Backward, &settings, &pixels, [1.0; 4]);
        for (k, direction) in toneequal::DEFAULT_DIRECTIONS.iter().enumerate() {
            let mut unit = [0.0; 4];
            unit[k] = 1.0;
            let forward = derivative(Derivative::ForwardDiff, &settings, &pixels, unit);
            for i in 0..pixels.len() {
                let exposure =
                    f64::from(darktable_exposure(pixels[i], Estimator::EuclideanNorm, 0.0));
                let slope = (curve(&settings, direction, h, exposure)
                    - curve(&settings, direction, -h, exposure))
                    / (2.0 * h);
                // The size of the terms the f32 derivative adds up: the direction's
                // weights cancel, up to 2,900 at σ = 2 for slopes of at most 1.3, so
                // rounding is relative to them, not to the slope.
                let terms = rounding_scale(&prepared, &prepared.directions[k], exposure);
                for c in 0..3 {
                    let input = f64::from(pixels[i][c]);
                    let numeric = input * slope;
                    let gap = (f64::from(forward[i][c]) - numeric).abs();
                    assert!(
                        gap <= ROUNDING * input.abs() * terms,
                        "σ {smoothing}, direction {k}, {:?}: forward {} against {numeric}",
                        pixels[i],
                        forward[i][c]
                    );
                }
                // Reverse mode adds the same terms in another order.
                let sum = forward[i][0] + forward[i][1] + forward[i][2];
                let magnitude: f64 = pixels[i].iter().map(|c| f64::from(c.abs())).sum();
                assert!(
                    f64::from((reverse[i][k] - sum).abs()) <= ROUNDING * magnitude * terms,
                    "σ {smoothing}, direction {k}: reverse {} against forward {sum}",
                    reverse[i][k]
                );
            }
        }
    }
}

/// darktable's mask restated on the CPU (`toneequal::filters`), with the curve in f64,
/// renders darktable's own filtered cases: the reference the kernel's whole-image passes
/// are held to is darktable's. Needs darktable-cli and the integration images.
#[test]
#[ignore = "needs darktable-cli and APERTRUST_TEST_IMAGES"]
fn the_restated_filters_render_darktables_cases() {
    use apertrust_plugin::Operation;
    use golden::{cases, darktable::DarktableCli, delta_e, pfm::Image, ports, render};
    let darktable = DarktableCli::locate().unwrap();
    let images = render::images_dir().unwrap();
    for name in [
        "toneequal-eigf",
        "toneequal-average",
        "toneequal-power",
        "toneequal-eigf-iterations",
    ] {
        let case = cases::find(name).unwrap();
        let references = render::render_case(&darktable, case, &images).unwrap();
        let input = Image::read(&references.input).unwrap();
        let expected = Image::read(&references.expected).unwrap();
        let values = ports::plan_values(case).unwrap().unwrap();
        let schema = toneequal::ToneEqual.schema();
        let settings = toneequal::settings(&schema.resolve(&values).unwrap());
        let (width, height) = (input.width, input.height);
        let context = ports::plan_context(case, width as u32, height as u32).unwrap();
        let camera = context.camera.unwrap();
        let full = [camera.width, camera.height];
        let mask = toneequal::filters::compute_luminance_mask(
            &input.pixels,
            width,
            height,
            full,
            &settings,
        );
        let geometry = toneequal::Geometry::full(width as u32, height as u32);
        let prepared = toneequal::prepare(&settings, &geometry).unwrap();
        let pixels = input
            .pixels
            .iter()
            .zip(&mask)
            .map(|(rgb, &mask)| {
                let correction = prepared.correction(f64::from(mask.log2())) as f32;
                rgb.map(|c| c * correction)
            })
            .collect();
        let actual = Image::new(width, height, pixels).unwrap();
        let (_, stats) = delta_e::compare(&expected, &actual, case.tolerance).unwrap();
        println!(
            "{name}: mean {:.4}, p95 {:.4}, max {:.4}",
            stats.mean, stats.p95, stats.max
        );
        assert!(stats.passes(case.tolerance), "{name}");
    }
}

/// The EIGF settings the GPU tests cover, on test 0131's bands: each estimator once, the
/// grid at a downscaling of 4, 2 and 1, a radius past the grid, iterations, and the
/// quantiser's two tracks.
fn eigf_modes() -> Vec<Settings> {
    let eigf = |method, blending, feathering, contrast_boost, exposure_boost| Settings {
        bands: TEST_0131,
        blending,
        feathering,
        contrast_boost,
        exposure_boost,
        details: Filter::Eigf,
        method,
        ..Settings::defaults()
    };
    let averaged = |method, blending, feathering, exposure_boost| Settings {
        details: Filter::AveragedEigf,
        ..eigf(method, blending, feathering, 0.0, exposure_boost)
    };
    // On a 97-pixel side: blending 20% gives a radius of 9, so a grid of 24 × 15 at a
    // downscaling of 4; 6%, a radius of 2 and a downscaling of 2; 5%, a radius of 1,
    // none; 100%, a radius of 48, σ 12 on a grid of 24 × 15.
    vec![
        eigf(Estimator::EuclideanNorm, 20.0, 1.0, 0.79, -0.5),
        averaged(Estimator::Average, 5.0, 3.0, 0.13),
        eigf(Estimator::PowerNorm, 6.0, 3.0, 0.27, 0.15),
        eigf(Estimator::Value, 100.0, 1.0, -0.5, 0.0),
        averaged(Estimator::Lightness, 20.0, 1.0, 0.3),
        eigf(Estimator::Sum, 20.0, 7.0, 0.5, -1.5),
        averaged(Estimator::GeometricMean, 30.0, 2.0, 0.5),
        Settings {
            iterations: 3,
            ..eigf(Estimator::EuclideanNorm, 20.0, 20.0, 0.0, -1.57)
        },
        Settings {
            iterations: 2,
            ..averaged(Estimator::PowerNorm, 6.0, 10.0, 0.0)
        },
        Settings {
            quantization: 0.5,
            ..eigf(Estimator::EuclideanNorm, 20.0, 1.0, 0.79, -0.5)
        },
        Settings {
            quantization: 1.0,
            ..averaged(Estimator::Lightness, 20.0, 3.0, 0.13)
        },
    ]
}

/// A scene `width` × `height` with edges the filter keeps: a gradient, a bright disc, a
/// dark band, fine texture and colours, from about 2^-10 to 4. A pixel whose mask would
/// lie within 1e-4 of a quantisation step under one of `eigf_modes` is moved off it:
/// there the quantised guide follows the last bit of the arithmetic, the CPU's and the
/// GPU's apart, as darktable's own does (docs/ports/toneequal.md, Results).
fn scene(width: usize, height: usize) -> Vec<[f32; 3]> {
    let near_a_step = |rgb: [f32; 3]| {
        eigf_modes()
            .iter()
            .filter(|settings| settings.quantization != 0.0)
            .any(|settings| {
                let (fulcrum, contrast) = match settings.details {
                    Filter::Eigf => (0.0625, settings.contrast_boost.exp2()),
                    _ => (0.0, 1.0),
                };
                let e = settings.exposure_boost.exp2();
                let mask =
                    toneequal::filters::luminance(rgb, settings.method, e, fulcrum, contrast);
                let steps = mask.log2() / settings.quantization;
                (steps - steps.round()).abs() < 1e-4
            })
    };
    (0..width * height)
        .map(|i| {
            let (x, y) = ((i % width) as f32, (i / width) as f32);
            let (u, v) = (x / width as f32, y / height as f32);
            let mut level = 0.002 + 0.3 * u * u + 0.05 * v;
            let disc = (u - 0.65).powi(2) + (v - 0.4).powi(2);
            if disc < 0.03 {
                level += 2.5;
            }
            if (0.2..0.35).contains(&u) {
                level *= 0.05;
            }
            level *= 1.0 + 0.3 * (x * 1.7 + y * 0.9).sin() * (y * 0.37).cos();
            let mut rgb = [level, level * (0.6 + 0.4 * v), level * (1.2 - 0.8 * u)];
            while near_a_step(rgb) {
                rgb = rgb.map(|c| c * 1.001);
            }
            rgb
        })
        .collect()
}

/// The scene's size: odd sides, so the grid samples fall at odd places.
const SCENE: [usize; 2] = [97, 61];

/// The output for every pixel of an image `size` across and down, rendered with these
/// settings.
fn render_image(settings: &Settings, pixels: &[[f32; 4]], size: [u32; 2]) -> Vec<[f32; 4]> {
    with_pipeline(settings, pixels, size, |gpu, pipeline| {
        pipeline.output().download(gpu).unwrap()
    })
}

#[test]
fn eigf_follows_darktables_loops() {
    // The whole-image passes and main on the GPU against darktable's loops restated in
    // toneequal::filters, the curve in f64. The masks differ by f32 rounding, the
    // CPU's and the GPU's, which the recursion carries along its lines: on the M4 at most
    // 2.9e-6 relative in the output without quantisation. With it, the blend's affine
    // term, avg_m - a avg_g with a above 1 where the mask lies far below its
    // neighbourhood, cancels and multiplies the covariance's rounding a hundredfold and
    // more: up to 1.2e-3 by the scene's dark band, where moving the grid by a unit in the
    // last place moves the CPU's own mask by 1.7e-4 (docs/ports/toneequal.md, Precision).
    let [width, height] = SCENE;
    let pixels = scene(width, height);
    let input = rgba(&pixels);
    let size = [width as u32, height as u32];
    for settings in eigf_modes() {
        let bound = if settings.quantization == 0.0 {
            2e-5
        } else {
            4e-3
        };
        let mask =
            toneequal::filters::compute_luminance_mask(&pixels, width, height, size, &settings);
        let prepared = prepare(&settings);
        let out = render_image(&settings, &input, size);
        for ((rgb, &mask), out) in pixels.iter().zip(&mask).zip(&out) {
            let correction = prepared.correction(f64::from(mask.log2()));
            for c in 0..3 {
                let expected = f64::from(rgb[c]) * correction;
                let gap = (f64::from(out[c]) - expected).abs();
                assert!(
                    gap <= bound * expected.abs(),
                    "{settings:?}: {rgb:?} gave {out:?}, expected {correction} times"
                );
            }
        }
    }
}

#[test]
fn eigf_keeps_colour_ratios_and_stays_finite() {
    let [width, height] = SCENE;
    let mut pixels = scene(width, height);
    // Black, negative and very bright pixels at the start.
    for (pixel, extreme) in pixels.iter_mut().zip(extreme()) {
        *pixel = extreme;
    }
    let size = [width as u32, height as u32];
    for settings in eigf_modes() {
        for exposure_boost in [16.0, -16.0, settings.exposure_boost] {
            let settings = Settings {
                exposure_boost,
                ..settings.clone()
            };
            let out = render_image(&settings, &rgba(&pixels), size);
            for (input, out) in pixels.iter().zip(&out) {
                assert!(
                    out.iter().all(|c| c.is_finite()),
                    "{out:?} for {settings:?}"
                );
                let k = (0..3)
                    .max_by(|&a, &b| input[a].abs().total_cmp(&input[b].abs()))
                    .unwrap();
                if input[k] == 0.0 || !input[k].is_normal() || input[k].abs() > 1e30 {
                    continue;
                }
                let gain = out[k] / input[k];
                assert!((0.25..=4.0).contains(&gain), "{gain} for {settings:?}");
                for c in 0..3 {
                    assert!(
                        (out[c] - gain * input[c]).abs() <= 1e-6 * input[k].abs() * gain,
                        "{input:?} gave {out:?} for {settings:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn a_tiled_export_in_two_rounds_equals_the_whole_one() {
    // Exposure, then the tone equalizer, so the tiles start after exposure: a first
    // round builds the mask's result, the last renders through main (decision 15).
    // Tiles of 23 × 17 cut the grid's samples at odd places. Bit for bit.
    use apertrust_engine::export::{Tiling, export};
    let gpu = &gpu().context;
    let [width, height] = SCENE;
    let input = rgba(&scene(width, height));
    let image = ImageContext::full(width as u32, height as u32);
    let modes = eigf_modes();
    for settings in [&modes[0], &modes[8]] {
        let instances = [
            Instance {
                operation: &apertrust_operations::exposure::Exposure,
                params: Default::default(),
            },
            Instance {
                operation: &toneequal::ToneEqual,
                params: toneequal::values(settings),
            },
        ];
        let run = |tiling| {
            let upload = GpuImage::upload(gpu, width as u32, height as u32, &input).unwrap();
            pollster::block_on(export(gpu, &instances, &image, upload, tiling)).unwrap()
        };
        let whole = run(Tiling::Whole);
        let tiled = run(Tiling::Tiles([23, 17]));
        assert_eq!((whole.tiles, tiled.tiles), (1, 20), "{settings:?}");
        assert_eq!((whole.rounds, tiled.rounds), (1, 2), "{settings:?}");
        assert!(
            whole.pixels == tiled.pixels,
            "{settings:?}: the tiles differ from the whole"
        );
    }
}

/// The tone equalizer with its mask compensated as a filtered mode compensates it,
/// contrast included, then left unfiltered: decision 15's baseline for the viewport, the
/// per-pixel gap any nonlinear operation shows after the downscale. A setting no schema,
/// edit or module reaches: only this test plans it.
struct CompensatedUnfiltered;

impl apertrust_plugin::Operation for CompensatedUnfiltered {
    fn manifest(&self) -> &'static apertrust_plugin::Manifest {
        apertrust_plugin::Operation::manifest(&toneequal::ToneEqual)
    }

    fn schema(&self) -> &'static apertrust_plugin::Schema {
        apertrust_plugin::Operation::schema(&toneequal::ToneEqual)
    }

    fn kernel(&self) -> &'static apertrust_plugin::Kernel {
        &toneequal::KERNEL
    }

    fn prepare(
        &self,
        params: &apertrust_plugin::ParamValues,
        image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<apertrust_plugin::Prepared, apertrust_plugin::NotPorted> {
        let settings = toneequal::settings(params);
        let unfiltered = Settings {
            details: Filter::None,
            ..settings.clone()
        };
        let full = image
            .camera
            .as_ref()
            .map_or([image.width, image.height], |c| [c.width, c.height]);
        let geometry = Geometry {
            full,
            scale: image.scale,
            whole: [input.width, input.height],
            input: [input.width, input.height],
        };
        let mut prepared = toneequal::prepare(&unfiltered, &geometry)?;
        if settings.details == Filter::Eigf {
            prepared.mask.fulcrum = 0.0625;
            prepared.mask.contrast_boost = settings.contrast_boost.exp2();
        }
        Ok(apertrust_plugin::Prepared {
            output: *input,
            passes: prepared.passes(),
            scratch: prepared.scratch(),
        })
    }
}

/// The viewport against the export (decision 15; docs/engine.md, Pipes): for each
/// filtered golden case on mire1, the viewport's render at 2400 × 1600 fitted and at
/// 1/2, 1/4 and 1/8 against the export downscaled by the engine's box filter, mean
/// ΔE2000, less the same comparison for the compensated, unfiltered baseline. The filter's
/// share is at most 0.15 at 2400 × 1600 and 1/2 and 0.4 at 1/4 and 1/8, and at 2400 ×
/// 1600 the whole gap is within darktable's mean, 0.767. Needs the integration images.
#[test]
#[ignore = "needs darktable-cli and APERTRUST_TEST_IMAGES"]
fn the_viewport_follows_the_export() {
    use apertrust_plugin::{Operation, ParamValues};
    use golden::{cases, darktable::DarktableCli, delta_e, pfm::Image, ports, render};
    let gpu = &gpu().context;
    let darktable = DarktableCli::locate().unwrap();
    let images = render::images_dir().unwrap();
    let limits = Limits::of(gpu);
    let rgb = |pixels: Vec<[f32; 4]>, desc: ImageDesc| {
        let pixels = pixels.into_iter().map(|[r, g, b, _]| [r, g, b]).collect();
        Image::new(desc.width as usize, desc.height as usize, pixels).unwrap()
    };
    // One render of `operation` with `params` over `input`, in `pipe`.
    let render = |operation: Option<&dyn Operation>,
                  params: &ParamValues,
                  context: &ImageContext,
                  input: &[[f32; 4]],
                  desc: ImageDesc,
                  pipe: Pipe| {
        let instances: Vec<Instance> = operation
            .map(|operation| Instance {
                operation,
                params: params.clone(),
            })
            .into_iter()
            .collect();
        let plan = pipeline::plan(&instances, context, desc, pipe, &limits).unwrap();
        let upload = GpuImage::upload(gpu, desc.width, desc.height, input).unwrap();
        let mut pipeline = pollster::block_on(Pipeline::new(gpu, plan, upload)).unwrap();
        pipeline.render(gpu).unwrap();
        let output = pipeline.output();
        (output.download(gpu).unwrap(), output.desc())
    };
    for name in [
        "toneequal-eigf",
        "toneequal-average",
        "toneequal-power",
        "toneequal-eigf-iterations",
    ] {
        let case = cases::find(name).unwrap();
        let references = render::render_case(&darktable, case, &images).unwrap();
        let source = Image::read(&references.input).unwrap();
        let (width, height) = (source.width as u32, source.height as u32);
        let context = ports::plan_context(case, width, height).unwrap();
        let input: Vec<[f32; 4]> = source
            .pixels
            .iter()
            .map(|&[r, g, b]| [r, g, b, 1.0])
            .collect();
        let desc = ImageDesc::rgba(width, height);
        let values = ports::plan_values(case).unwrap().unwrap();
        let export = |operation: &dyn Operation| {
            render(
                Some(operation),
                &values,
                &context,
                &input,
                desc,
                Pipe::Export,
            )
        };
        let (filtered, base) = (
            export(&toneequal::ToneEqual),
            export(&CompensatedUnfiltered),
        );
        println!("{name}:");
        for (label, size) in [
            ("2400×1600", [2400, 1600]),
            ("1/2", [width / 2, height / 2]),
            ("1/4", [width / 4, height / 4]),
            ("1/8", [width / 8, height / 8]),
        ] {
            let pipe = Pipe::Viewport {
                width: size[0],
                height: size[1],
            };
            let gap = |operation: &dyn Operation,
                       (export, export_desc): &(Vec<[f32; 4]>, ImageDesc)| {
                let (down, down_desc) = render(None, &values, &context, export, *export_desc, pipe);
                let (view, view_desc) =
                    render(Some(operation), &values, &context, &input, desc, pipe);
                assert_eq!(down_desc, view_desc);
                let (_, stats) =
                    delta_e::compare(&rgb(down, down_desc), &rgb(view, view_desc), case.tolerance)
                        .unwrap();
                stats.mean
            };
            let whole = gap(&toneequal::ToneEqual, &filtered);
            let baseline = gap(&CompensatedUnfiltered, &base);
            let share = whole - baseline;
            println!(
                "  {label}: viewport against export {whole:.3}, baseline {baseline:.3}, the filter's share {share:+.3}"
            );
            let target = if matches!(label, "2400×1600" | "1/2") {
                0.15
            } else {
                0.4
            };
            assert!(share <= target, "{name} at {label}: {share}");
            if label == "2400×1600" {
                assert!(whole <= 0.767, "{name}: {whole}");
            }
        }
    }
}
