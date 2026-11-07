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

//! Soften on the GPU against its restatement on the CPU, and its invariants, needing no
//! darktable (`docs/ports/soften.md`, Test plan): the operation equals darktable's HSL
//! with its double steps (`soften::hsl`), the shared box mean's Rust twin on each channel
//! and the mix, so the Slang box mean on three channels is checked against
//! `apertrust_color::box_filter`; a mix of 0 changes nothing; output stays finite; and
//! the amount's derivative against finite differences of whole renders. Each runs the
//! operation as a one-instance plan of the engine, as the golden case does. CI runs them
//! on software GPUs.

use std::sync::OnceLock;

use apertrust_color::box_filter::{BOX_ITERATIONS, box_mean};
use apertrust_engine::compute::GpuImage;
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::pipeline::{self, Derivative, Instance, Pipe, Pipeline, Plan};
use apertrust_operations::soften::{self, Settings, Soften, hsl};
use apertrust_plugin::{Camera, ImageContext, Operation, Pattern};

fn gpu() -> &'static GpuContext {
    static GPU: OnceLock<GpuContext> = OnceLock::new();
    GPU.get_or_init(|| {
        pollster::block_on(GpuContext::new(&GpuOptions::default()))
            .expect("a GPU adapter, hardware or software")
    })
}

fn upload(width: u32, height: u32, pixels: &[[f32; 4]]) -> GpuImage {
    GpuImage::upload(gpu(), width, height, pixels).unwrap()
}

/// The readout the tests' small images are taken from, so that darktable's largest
/// radius, 1% of its diagonal, is 72 pixels: a radius wider than the images is in reach.
const READOUT: [u32; 2] = [6000, 4000];

/// An image `width` × `height` from a raw whose readout is [`READOUT`].
fn context(width: u32, height: u32) -> ImageContext {
    ImageContext {
        camera: Some(Camera {
            make: String::new(),
            model: String::new(),
            alias: String::new(),
            width: READOUT[0],
            height: READOUT[1],
            cfa: None,
            black: Pattern {
                width: 1,
                height: 1,
                values: vec![0.0],
            },
            white: 1.0,
            white_balance: None,
            xyz_to_camera: None,
            crop: [0, 0, READOUT[0], READOUT[1]],
            declared: [0, 0, READOUT[0], READOUT[1]],
            exposure_bias: 0.0,
            iso: 0.0,
            linear_response_limit: 1.0,
            lens_correction: None,
            orientation: 0,
        }),
        ..ImageContext::full(width, height)
    }
}

/// The operation with `settings`, or nothing, planned over `input` in `image` for `pipe`.
fn plan_for(
    settings: Option<&Settings>,
    input: &GpuImage,
    image: &ImageContext,
    pipe: Pipe,
) -> Plan {
    let instances: Vec<Instance> = settings
        .map(|settings| Instance {
            operation: &Soften,
            params: Soften.schema().resolve(&soften::values(settings)).unwrap(),
        })
        .into_iter()
        .collect();
    let limits = pipeline::Limits::of(gpu());
    pipeline::plan(&instances, image, input.desc(), pipe, &limits).unwrap()
}

fn plan(settings: &Settings, input: &GpuImage) -> Plan {
    let desc = input.desc();
    plan_for(
        Some(settings),
        input,
        &context(desc.width, desc.height),
        Pipe::Export,
    )
}

fn render(settings: &Settings, input: &GpuImage) -> Vec<[f32; 4]> {
    let mut pipeline =
        pollster::block_on(Pipeline::new(gpu(), plan(settings, input), input.clone())).unwrap();
    pipeline.render(gpu()).unwrap();
    pipeline.output().download(gpu()).unwrap()
}

fn settings(size: f32, saturation: f32, brightness: f32, amount: f32) -> Settings {
    Settings {
        size,
        saturation,
        brightness,
        amount,
    }
}

/// A `width` × `height` image of patches: hues all round, greys, colours past white and
/// below black, and alpha, so it is seen to pass through.
fn patches(width: u32, height: u32) -> Vec<[f32; 4]> {
    const COLOURS: [[f32; 3]; 10] = [
        [0.8, 0.1, 0.05],
        [0.2, 0.7, 0.1],
        [0.05, 0.15, 0.9],
        [0.18, 0.18, 0.18],
        [0.9, 0.85, 0.2],
        [0.6, 0.05, 0.7],
        [1.6, 1.2, 0.4],
        [0.02, -0.03, 0.01],
        [0.0, 0.0, 0.0],
        [0.45, 0.6, 0.62],
    ];
    (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let [r, g, b] = COLOURS[((x / 5 + y / 4) % COLOURS.len() as u32) as usize];
            let shade = 0.7 + 0.3 * ((x + 2 * y) % 9) as f32 / 8.0;
            [r * shade, g * shade, b * shade, (x % 7) as f32 / 6.0]
        })
        .collect()
}

/// The kernel's guard on each input channel (docs/ports/soften.md, Precision).
const MAX_INPUT: f32 = 1e6;

/// darktable's softened pixel, `process()`'s first loop: the HSL saturation and
/// lightness scaled and clipped, after the kernel's guard.
fn overexposed(settings: &Settings, rgb: [f32; 3]) -> [f32; 3] {
    let [h, s, l] = hsl::rgb_to_hsl(rgb.map(|v| v.clamp(-MAX_INPUT, MAX_INPUT)));
    let s = (s * soften::saturation_factor(settings.saturation)).clamp(0.0, 1.0);
    let l = (l * soften::brightness_factor(settings.brightness)).clamp(0.0, 1.0);
    hsl::hsl_to_rgb([h, s, l])
}

/// darktable's soften on the CPU at full resolution, from [`READOUT`]: the softened
/// image, `box_mean` on each channel, and the mix.
fn restated(settings: &Settings, width: u32, height: u32, pixels: &[[f32; 4]]) -> Vec<[f32; 4]> {
    let softened: Vec<[f32; 3]> = pixels
        .iter()
        .map(|&[r, g, b, _]| overexposed(settings, [r, g, b]))
        .collect();
    let radius = soften::full_radius(settings.size, READOUT) as usize;
    let blurred: Vec<Vec<f32>> = (0..3)
        .map(|c| {
            let mut channel: Vec<f32> = softened.iter().map(|p| p[c]).collect();
            box_mean(
                &mut channel,
                width as usize,
                height as usize,
                radius,
                BOX_ITERATIONS,
            );
            channel
        })
        .collect();
    let mix = soften::mix(settings.amount);
    pixels
        .iter()
        .enumerate()
        .map(|(i, &[r, g, b, a])| {
            let blend = |c: usize, v: f32| mix * blurred[c][i] + (1.0 - mix) * v;
            [blend(0, r), blend(1, g), blend(2, b), a]
        })
        .collect()
}

/// Largest difference of any channel relative to the pixel's largest channel, past a
/// floor for black.
fn largest_relative_difference(a: &[[f32; 4]], b: &[[f32; 4]]) -> f32 {
    a.iter()
        .zip(b)
        .flat_map(|(p, q)| {
            let scale = p[..3].iter().fold(1e-3_f32, |m, v| m.max(v.abs()));
            (0..4).map(move |c| (p[c] - q[c]).abs() / scale)
        })
        .fold(0.0, f32::max)
}

#[test]
fn the_operation_equals_its_restatement_on_the_cpu() {
    // Radii of 0, 2 and 10, and 72, wider than the image, where windows reach both ends
    // of every line; the saturation and the brightness at their ends and between.
    let (width, height) = (61, 47);
    let pixels = patches(width, height);
    let input = upload(width, height, &pixels);
    for settings in [
        settings(0.0, 100.0, 0.33, 50.0),
        settings(3.0, 100.0, 0.5, 75.0),
        settings(13.0, 37.0, -1.2, 60.0),
        settings(100.0, 0.0, 2.0, 100.0),
    ] {
        let ours = render(&settings, &input);
        let theirs = restated(&settings, width, height, &pixels);
        let gap = largest_relative_difference(&theirs, &ours);
        assert!(gap < 1e-5, "{settings:?}: {gap}");
        assert!(
            largest_relative_difference(&pixels, &ours) > 1e-2,
            "{settings:?}"
        );
        assert!(ours.iter().zip(&pixels).all(|(o, p)| o[3] == p[3]));
    }
}

/// The identity (docs/ports/soften.md, Tool binding notes): at a mix of 0 every pixel
/// is its input, whatever the other settings, an infinite one included.
#[test]
fn a_mix_of_zero_changes_nothing() {
    let (width, height) = (40, 30);
    let mut pixels = patches(width, height);
    pixels[(12 * width + 9) as usize] = [f32::INFINITY, 0.5, f32::NEG_INFINITY, 1.0];
    let input = upload(width, height, &pixels);
    for settings in [
        settings(50.0, 100.0, 0.33, 0.0),
        settings(100.0, 0.0, 2.0, 0.0),
        settings(0.0, 100.0, -2.0, 0.0),
    ] {
        let output = render(&settings, &input);
        assert_eq!(output, pixels, "{settings:?}");
    }
}

#[test]
fn output_stays_finite_for_extreme_input_and_settings() {
    let values = [0.0, 1e-30, -0.5, -1e3, 1e-3, 0.18, 1.0, 1e3, 1e6];
    let (width, height) = (12, 9);
    let pixels: Vec<[f32; 4]> = (0..width * height)
        .map(|i| {
            let v = values[i % values.len()];
            let w = values[(i / values.len()) % values.len()];
            [v, w, if i % 3 == 0 { v } else { 0.0 }, 1.0]
        })
        .collect();
    let input = upload(width as u32, height as u32, &pixels);
    for settings in [
        Settings::default(),
        settings(0.0, 0.0, -2.0, 0.0),
        settings(100.0, 100.0, 2.0, 100.0),
        settings(100.0, 0.0, -2.0, 100.0),
        settings(0.0, 100.0, 2.0, 50.0),
    ] {
        let output = render(&settings, &input);
        assert!(
            output.iter().flatten().all(|v| v.is_finite()),
            "{settings:?}: {output:?}"
        );
    }
}

/// The guard on the input: an infinite pixel softens into its neighbours as white but
/// leaves every other pixel finite, where the box means' running sums would carry it
/// along its row and, subtracted in the columns, make the whole image NaN.
#[test]
fn an_infinite_pixel_leaves_the_rest_of_the_image_finite() {
    let (width, height) = (40, 30);
    let mut pixels = patches(width, height);
    let at = (15 * width + 20) as usize;
    for infinite in [
        [f32::INFINITY; 3],
        [f32::INFINITY, 0.0, 0.0],
        [f32::NEG_INFINITY, 0.3, f32::INFINITY],
    ] {
        let [r, g, b] = infinite;
        pixels[at] = [r, g, b, 1.0];
        let input = upload(width, height, &pixels);
        for saturation in [0.0, 100.0] {
            let output = render(&settings(20.0, saturation, 0.33, 50.0), &input);
            for (i, pixel) in output.iter().enumerate() {
                if i != at {
                    assert!(pixel.iter().all(|v| v.is_finite()), "pixel {i}: {pixel:?}");
                }
            }
        }
    }
}

/// The sum of an image's RGB, the gradient check's loss.
fn loss(pixels: &[[f32; 4]]) -> f64 {
    pixels
        .iter()
        .map(|p| f64::from(p[0]) + f64::from(p[1]) + f64::from(p[2]))
        .sum()
}

/// Reverse and forward mode of the loss with respect to the amount at `base`, over the
/// scratch its render left.
fn derivatives(base: &Settings, input: &GpuImage) -> (f64, f64) {
    let desc = input.desc();
    let count = (desc.width * desc.height) as usize;
    let mut pipeline =
        pollster::block_on(Pipeline::new(gpu(), plan(base, input), input.clone())).unwrap();
    pipeline.render(gpu()).unwrap();
    let d_out = GpuImage::new(gpu(), desc.width, desc.height);
    let ones = upload(desc.width, desc.height, &vec![[1.0, 1.0, 1.0, 0.0]; count]);
    pollster::block_on(pipeline.derivative(gpu(), Derivative::Backward, &ones, &d_out)).unwrap();
    let per_pixel = d_out.download(gpu()).unwrap();
    assert!(per_pixel.iter().all(|p| p[1..] == [0.0; 3]));
    let reverse: f64 = per_pixel.iter().map(|p| f64::from(p[0])).sum();
    let directions = upload(desc.width, desc.height, &vec![[1.0, 0.0, 0.0, 0.0]; count]);
    pollster::block_on(pipeline.derivative(gpu(), Derivative::ForwardDiff, &directions, &d_out))
        .unwrap();
    (reverse, loss(&d_out.download(gpu()).unwrap()))
}

fn relative_error(actual: f64, expected: f64) -> f64 {
    (actual - expected).abs() / expected.abs().max(1e-6)
}

/// The amount's derivative is the whole render's (`docs/ports/soften.md`, Gradients):
/// only `main` reads it, and the mix is linear in it, so central differences of whole
/// renders agree with reverse mode, and forward mode with reverse.
#[test]
fn the_amounts_derivative_is_the_whole_renders() {
    let (width, height) = (61, 47);
    let input = upload(width, height, &patches(width, height));
    let base = settings(13.0, 60.0, 0.5, 40.0);
    let (reverse, forward) = derivatives(&base, &input);
    let step = 0.5;
    let moved = |delta: f32| {
        loss(&render(
            &Settings {
                amount: base.amount + delta,
                ..base
            },
            &input,
        ))
    };
    let numeric = (moved(step) - moved(-step)) / f64::from(2.0 * step);
    assert!(numeric.abs() > 1e-2, "{numeric}");
    assert!(
        relative_error(reverse, numeric) < 1e-3,
        "{reverse} against {numeric}"
    );
    assert!(
        relative_error(forward, reverse) < 1e-4,
        "{forward}, {reverse}"
    );
}

/// One viewport size of decision 15's measure (`docs/engine.md`, Pipes): soften's
/// viewport against its export downscaled by the viewport's box filter, mean ΔE2000, and
/// the per-step baseline, the viewport's mix with the export's blur downscaled (decisions
/// 15 (4) and 116).
struct Scale {
    /// The viewport's box, in which the image is fitted.
    box_size: [u32; 2],
    width: usize,
    height: usize,
    stats: golden::delta_e::Stats,
    baseline: f64,
    /// The mean less the baseline, which the target bounds.
    share: f64,
    target: f64,
}

/// Decision 15's measure of soften with `settings` over `input` in `image`, at 2400×1600
/// and at 1/2, 1/4 and 1/8 of the image: the viewport, the operation run on the fitted
/// image, against the export downscaled, less the same for the viewport's mix with the
/// export's blur downscaled, the per-pixel gap left once the whole-image result is exact,
/// every value a pixel reads from other pixels held at the export's and every per-pixel
/// step at the viewport's scale (decisions 15 (4) and 116). Its target is at most 0.15 at
/// 2400×1600 and at 1/2, 0.4 at 1/4 and 1/8, and within `Tolerance::DARKTABLE`'s mean in
/// all at 2400×1600. The mix is linear, so the box filter commutes with it and the
/// baseline is rounding (`docs/ports/soften.md`, Results).
fn viewport_against_export(
    settings: &Settings,
    input: &GpuImage,
    image: &ImageContext,
) -> Vec<Scale> {
    use golden::delta_e::{self, Tolerance};
    use golden::pfm::Image;

    let downloaded = |plan: Plan, input: &GpuImage| {
        let mut pipeline = pollster::block_on(Pipeline::new(gpu(), plan, input.clone())).unwrap();
        pipeline.render(gpu()).unwrap();
        let output = pipeline.output();
        let desc = output.desc();
        let pixels = output.download(gpu()).unwrap();
        let rgb = pixels.iter().map(|&[r, g, b, _]| [r, g, b]).collect();
        Image::new(desc.width as usize, desc.height as usize, rgb).unwrap()
    };
    let rgba = |image: Image| -> Vec<[f32; 4]> {
        image
            .pixels
            .iter()
            .map(|&[r, g, b]| [r, g, b, 1.0])
            .collect()
    };
    let desc = input.desc();
    let (width, height) = (desc.width, desc.height);
    let export = downloaded(plan_for(Some(settings), input, image, Pipe::Export), input);
    let exported = upload(width, height, &rgba(export));
    // The export's blur: the export at a mix of 100%, which is the blur exactly.
    let mix = soften::mix(settings.amount);
    let whole_blur = Settings {
        amount: 100.0,
        ..*settings
    };
    let blur = downloaded(
        plan_for(Some(&whole_blur), input, image, Pipe::Export),
        input,
    );
    let blurred = upload(width, height, &rgba(blur));
    let mut scales = Vec::new();
    for (box_size, target) in [
        ([2400, 1600], 0.15),
        ([width / 2, height / 2], 0.15),
        ([width / 4, height / 4], 0.4),
        ([width / 8, height / 8], 0.4),
    ] {
        let pipe = Pipe::Viewport {
            width: box_size[0],
            height: box_size[1],
        };
        let viewport = downloaded(plan_for(Some(settings), input, image, pipe), input);
        let downscaled = downloaded(plan_for(None, &exported, image, pipe), &exported);
        let fitted = downloaded(plan_for(None, input, image, pipe), input);
        let export_blur = downloaded(plan_for(None, &blurred, image, pipe), &blurred);
        let held: Vec<[f32; 3]> = fitted
            .pixels
            .iter()
            .zip(&export_blur.pixels)
            .map(|(rgb, blur)| [0, 1, 2].map(|c| mix * blur[c] + (1.0 - mix) * rgb[c]))
            .collect();
        let held = Image::new(fitted.width, fitted.height, held).unwrap();
        let (_, stats) = delta_e::compare(&downscaled, &viewport, Tolerance::DARKTABLE).unwrap();
        let (_, baseline) = delta_e::compare(&downscaled, &held, Tolerance::DARKTABLE).unwrap();
        scales.push(Scale {
            box_size,
            width: viewport.width,
            height: viewport.height,
            share: stats.mean - baseline.mean,
            baseline: baseline.mean,
            stats,
            target,
        });
    }
    scales
}

/// What a scale misses of decision 15's target, if anything.
fn missed(scale: &Scale) -> Option<String> {
    use golden::delta_e::Tolerance;

    let over_mean = scale.box_size == [2400, 1600] && scale.stats.mean > Tolerance::DARKTABLE.mean;
    (scale.share > scale.target || over_mean).then(|| {
        format!(
            "at {}×{}: share {:.4} against {}, mean {:.4}",
            scale.width, scale.height, scale.share, scale.target, scale.stats.mean
        )
    })
}

fn report(what: &str, scale: &Scale) {
    eprintln!(
        "{what} at {}×{}: mean {:.4}, p95 {:.4}, max {:.3}; with the export's blur {:.4}; \
         the softened image's and blur's share {:.4}",
        scale.width,
        scale.height,
        scale.stats.mean,
        scale.stats.p95,
        scale.stats.max,
        scale.baseline,
        scale.share
    );
}

/// Decision 15's viewport target on the case's input, darktable's render of mire1 before
/// the module ([`viewport_against_export`]). Needs darktable-cli and the integration
/// images; run by hand.
#[test]
#[ignore = "needs darktable-cli and APERTRUST_TEST_IMAGES"]
fn the_viewport_is_within_decision_80s_target_of_the_export() {
    use golden::darktable::DarktableCli;
    use golden::pfm::Image;
    use golden::{cases, ports, render};

    let darktable = DarktableCli::locate().unwrap();
    let images = render::images_dir().unwrap();
    let case = cases::find("soften").unwrap();
    let source = Image::read(
        &render::render_case(&darktable, case, &images)
            .unwrap()
            .input,
    )
    .unwrap();
    let rgba: Vec<[f32; 4]> = source
        .pixels
        .iter()
        .map(|&[r, g, b]| [r, g, b, 1.0])
        .collect();
    let (width, height) = (source.width as u32, source.height as u32);
    let input = upload(width, height, &rgba);
    let image = ports::plan_context(case, width, height).unwrap();
    let values = ports::plan_values(case).unwrap().unwrap();
    let settings = soften::settings(&Soften.schema().resolve(&values).unwrap());
    for scale in viewport_against_export(&settings, &input, &image) {
        report("soften", &scale);
        assert!(missed(&scale).is_none(), "{:?}", missed(&scale));
    }
}

/// Decision 15's corpus measurement, which comes before the Soften module ships
/// (`docs/engine.md`, Pipes; `docs/ports/soften.md`, Results, On the corpus): on each corpus raw
/// the pipeline takes, its default pipeline up to soften's place, exported at full
/// resolution, as the input, and soften at test 0051's size, saturation and brightness
/// with darktable's mix of 50 % ([`viewport_against_export`]). Each scale is reported;
/// the test fails at the end on any that misses the target. Needs the raw.pixls.us corpus;
/// run by hand.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn the_viewport_is_within_decision_80s_target_on_the_corpus() {
    use std::cmp::Ordering;

    use apertrust_engine::edit::{self, Edit};
    use apertrust_engine::export::{BUDGET, Tiling, export};

    let corpus = golden::render::corpus_dir().unwrap();
    let names = include_str!("../../raw/tests/corpus/files.sha256")
        .lines()
        .filter_map(|line| line.split_once("  ").map(|(_, name)| name.trim()));
    let settings = settings(33.0, 100.0, 0.5, 50.0);
    let place = Soften.manifest().slot;
    let (mut measured, mut misses) = (0, Vec::new());
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
        let (width, height) = (developed.desc.width, developed.desc.height);
        let input = upload(width, height, &developed.pixels);
        drop(developed);
        let image = ImageContext {
            camera: context.camera.clone(),
            ..ImageContext::full(width, height)
        };
        for scale in viewport_against_export(&settings, &input, &image) {
            report(name, &scale);
            if let Some(miss) = missed(&scale) {
                misses.push(format!("{name} {miss}"));
            }
        }
        measured += 1;
    }
    eprintln!("measured on {measured} raws; {} misses", misses.len());
    assert!(misses.is_empty(), "{misses:#?}");
}
