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

//! Bloom on the GPU against its restatement on the CPU, and its invariants, needing no
//! darktable (`docs/ports/bloom.md`, Test plan): the operation equals the threshold, the
//! shared box mean's Rust twin and the blend, so the Slang box mean is checked against
//! `apertrust_color::box_filter`; lights below the threshold change nothing; output
//! stays finite; and the strength's derivative, `main`'s for the lights past the
//! threshold, against finite differences of whole renders. Each runs the operation as a
//! one-instance plan of the engine, as the golden case does. CI runs them on software
//! GPUs.

use std::sync::OnceLock;

use apertrust_color::box_filter::{BOX_ITERATIONS, box_mean};
use apertrust_color::{WorkingProfile, apply, lab_to_xyz_d50, xyz_to_lab_d50};
use apertrust_engine::compute::GpuImage;
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::pipeline::{self, Derivative, Instance, Pipe, Pipeline, Plan};
use apertrust_operations::bloom::{self, Bloom, Settings};
use apertrust_plugin::{ImageContext, Operation};

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

/// The operation with `settings`, or nothing, planned over `input` for `pipe`.
fn plan_for(settings: Option<&Settings>, input: &GpuImage, pipe: Pipe) -> Plan {
    let desc = input.desc();
    let instances: Vec<Instance> = settings
        .map(|settings| Instance {
            operation: &Bloom,
            params: Bloom.schema().resolve(&bloom::values(settings)).unwrap(),
        })
        .into_iter()
        .collect();
    let image = ImageContext::full(desc.width, desc.height);
    let limits = pipeline::Limits::of(gpu());
    pipeline::plan(&instances, &image, desc, pipe, &limits).unwrap()
}

fn plan(settings: &Settings, input: &GpuImage) -> Plan {
    plan_for(Some(settings), input, Pipe::Export)
}

fn render(settings: &Settings, input: &GpuImage) -> Vec<[f32; 4]> {
    let mut pipeline =
        pollster::block_on(Pipeline::new(gpu(), plan(settings, input), input.clone())).unwrap();
    pipeline.render(gpu()).unwrap();
    pipeline.output().download(gpu()).unwrap()
}

fn settings(size: f32, threshold: f32, strength: f32) -> Settings {
    Settings {
        size,
        threshold,
        strength,
    }
}

/// Working RGB of a Lab colour.
fn rgb_of(lab: [f32; 3]) -> [f32; 3] {
    apply(
        &WorkingProfile::linear_rec2020().xyz_d50_to_rgb,
        lab_to_xyz_d50(lab),
    )
}

/// A `width` × `height` image whose lightness is `lightness(x, y)`, with chroma that
/// varies, and alpha, so it is seen to pass through.
fn image(width: u32, height: u32, lightness: impl Fn(u32, u32) -> f32) -> Vec<[f32; 4]> {
    (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let a = 30.0 * (x as f32 * 0.3).sin();
            let b = 25.0 * (y as f32 * 0.2).cos();
            let [r, g, b] = rgb_of([lightness(x, y), a, b]);
            [r, g, b, (x % 7) as f32 / 6.0]
        })
        .collect()
}

/// Lightness in patches of eight levels, far from 40.3, the lightness the tests'
/// threshold of 50 and strength of 30 make the lights' edge, so no pixel there crosses
/// it as the strength moves by a little, nor rounds to its other side on the GPU.
const LEVELS: [f32; 8] = [5.0, 20.0, 35.0, 47.0, 62.0, 80.0, 96.0, 110.0];

fn patches(width: u32, height: u32) -> Vec<[f32; 4]> {
    image(width, height, |x, y| {
        LEVELS[((x / 5 + y / 4) % LEVELS.len() as u32) as usize]
    })
}

/// Lab of a working RGB colour.
fn lab_of([r, g, b]: [f32; 3]) -> [f32; 3] {
    xyz_to_lab_d50(apply(
        &WorkingProfile::linear_rec2020().rgb_to_xyz_d50,
        [r, g, b],
    ))
}

/// The kernel's guard on a pixel's light (docs/ports/bloom.md, Precision).
const MAX_LIGHT: f32 = 1e6;

/// darktable's blurred lights on the CPU at full resolution: the lightness scaled and
/// thresholded, bounded by the kernel's guard, then `box_mean`.
fn blurred_lights(settings: &Settings, width: u32, height: u32, pixels: &[[f32; 4]]) -> Vec<f32> {
    let scale = bloom::light_scale(settings.strength);
    let mut lights: Vec<f32> = pixels
        .iter()
        .map(|&[r, g, b, _]| {
            let l = (lab_of([r, g, b])[0] * scale).min(MAX_LIGHT);
            if l > settings.threshold { l } else { 0.0 }
        })
        .collect();
    let radius = bloom::full_radius(settings.size) as usize;
    box_mean(
        &mut lights,
        width as usize,
        height as usize,
        radius,
        BOX_ITERATIONS,
    );
    lights
}

/// darktable's screen blend of a pixel's lightness with its blurred lights.
fn blend(rgb: [f32; 3], blur: f32) -> [f32; 3] {
    let lab = lab_of(rgb);
    let l = 100.0 - ((100.0 - lab[0]) * (100.0 - blur)) / 100.0;
    rgb_of([l, lab[1], lab[2]])
}

/// darktable's bloom on the CPU: the lights, `box_mean`, the blend, through Lab.
fn restated(settings: &Settings, width: u32, height: u32, pixels: &[[f32; 4]]) -> Vec<[f32; 4]> {
    let lights = blurred_lights(settings, width, height, pixels);
    pixels
        .iter()
        .zip(&lights)
        .map(|(&[r, g, b, a], &blur)| {
            let [r, g, b] = blend([r, g, b], blur);
            [r, g, b, a]
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
    // Radii of 2 and 10, and 79, wider than the image, where windows reach both ends
    // of every line.
    let (width, height) = (61, 47);
    let pixels = patches(width, height);
    let input = upload(width, height, &pixels);
    for size in [0.0, 3.0, 30.0] {
        let settings = settings(size, 50.0, 30.0);
        let ours = render(&settings, &input);
        let theirs = restated(&settings, width, height, &pixels);
        let gap = largest_relative_difference(&theirs, &ours);
        assert!(gap < 1e-4, "size {size}: {gap}");
        assert!(
            largest_relative_difference(&pixels, &ours) > 1e-2,
            "size {size}"
        );
        assert!(ours.iter().zip(&pixels).all(|(o, p)| o[3] == p[3]));
    }
}

#[test]
fn lights_below_the_threshold_change_nothing() {
    // At strength 0 the lights are the lightness times 2^0.01, below 100 for every
    // lightness here, so the blur is 0 and the blend keeps the lightness.
    let (width, height) = (40, 30);
    let pixels = image(width, height, |x, y| 2.0 + 93.0 * (x + y) as f32 / 68.0);
    let input = upload(width, height, &pixels);
    let output = render(&settings(20.0, 100.0, 0.0), &input);
    let gap = largest_relative_difference(&pixels, &output);
    assert!(gap < 1e-5, "{gap}");
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
        settings(0.0, 0.0, 0.0),
        settings(100.0, 0.0, 100.0),
        settings(100.0, 100.0, 0.0),
        settings(0.0, 100.0, 100.0),
    ] {
        let output = render(&settings, &input);
        assert!(
            output.iter().flatten().all(|v| v.is_finite()),
            "{settings:?}: {output:?}"
        );
    }
}

/// The guard on a pixel's light: an infinite pixel blooms into its neighbours but leaves
/// every other pixel finite, where the box means' running sums would carry it along its
/// row and, subtracted in the columns, make the whole image NaN.
#[test]
fn an_infinite_pixel_leaves_the_rest_of_the_image_finite() {
    let (width, height) = (40, 30);
    let mut pixels = patches(width, height);
    let at = (15 * width + 20) as usize;
    pixels[at] = [f32::INFINITY, f32::INFINITY, f32::INFINITY, 1.0];
    let input = upload(width, height, &pixels);
    let output = render(&settings(0.0, 50.0, 30.0), &input);
    for (i, pixel) in output.iter().enumerate() {
        if i != at {
            assert!(pixel.iter().all(|v| v.is_finite()), "pixel {i}: {pixel:?}");
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

/// Reverse and forward mode of the loss with respect to the strength at `base`, over
/// the scratch its render left.
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

/// Central differences of whole renders, the strength moved by `step`, and with
/// `proportional` the threshold with it in proportion to the lights' scale, so that the
/// pixels past the threshold stay the same.
fn finite_difference(base: &Settings, input: &GpuImage, step: f32, proportional: bool) -> f64 {
    let moved = |delta: f32| {
        let strength = base.strength + delta;
        let threshold = if proportional {
            base.threshold * bloom::light_scale(strength) / bloom::light_scale(base.strength)
        } else {
            base.threshold
        };
        loss(&render(&settings(base.size, threshold, strength), input))
    };
    (moved(step) - moved(-step)) / f64::from(2.0 * step)
}

fn relative_error(actual: f64, expected: f64) -> f64 {
    (actual - expected).abs() / expected.abs().max(1e-6)
}

/// The strength's derivative is `main`'s for the lights past the threshold
/// (`docs/ports/bloom.md`, Gradients): on an image no pixel of which crosses the
/// threshold as the strength moves, it is the finite differences of whole renders; on
/// one with lightness everywhere, it is those with the threshold moved in proportion to
/// the lights, which keeps the pixels past it, and the finite differences of the
/// strength alone see the pixels crossing it too.
#[test]
fn the_strengths_derivative_is_mains_for_the_lights_past_the_threshold() {
    let base = settings(3.0, 50.0, 30.0);

    let (width, height) = (61, 47);
    let input = upload(width, height, &patches(width, height));
    let (reverse, forward) = derivatives(&base, &input);
    let numeric = finite_difference(&base, &input, 1e-2, false);
    assert!(
        relative_error(reverse, numeric) < 1e-2,
        "{reverse} against {numeric}"
    );
    assert!(
        relative_error(forward, reverse) < 1e-4,
        "{forward}, {reverse}"
    );

    let (width, height) = (128, 96);
    let ramp = image(width, height, |x, y| {
        110.0 * (y * width + x) as f32 / (width * height) as f32
    });
    let input = upload(width, height, &ramp);
    let (reverse, forward) = derivatives(&base, &input);
    let proportional = finite_difference(&base, &input, 1e-2, true);
    assert!(
        relative_error(reverse, proportional) < 1e-2,
        "{reverse} against {proportional}"
    );
    assert!(
        relative_error(forward, reverse) < 1e-4,
        "{forward}, {reverse}"
    );
    let alone = finite_difference(&base, &input, 0.5, false);
    assert!(alone > 1.1 * reverse, "{alone} against {reverse}");
}

/// The gradient check's gap on test 0050 (`docs/ports/bloom.md`, Gradients and
/// Results): reverse mode against finite differences of whole renders of mire1, the
/// strength alone and with the threshold in proportion. Needs darktable-cli and the
/// integration images; run by hand.
#[test]
#[ignore = "needs darktable-cli and APERTRUST_TEST_IMAGES"]
fn the_cases_gap_is_the_pixels_crossing_the_threshold() {
    use golden::darktable::DarktableCli;
    use golden::pfm::Image;
    use golden::{cases, ports, render};

    let darktable = DarktableCli::locate().unwrap();
    let images = render::images_dir().unwrap();
    let case = cases::find("bloom").unwrap();
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
    let input = upload(source.width as u32, source.height as u32, &rgba);
    let values = ports::plan_values(case).unwrap().unwrap();
    let base = bloom::settings(&Bloom.schema().resolve(&values).unwrap());
    let (reverse, forward) = derivatives(&base, &input);
    let proportional = finite_difference(&base, &input, 1e-2, true);
    let alone = finite_difference(&base, &input, 1e-2, false);
    eprintln!(
        "reverse {reverse:.6e}, forward {forward:.6e}, the threshold in proportion \
         {proportional:.6e} ({:.1e}), the strength alone {alone:.6e} ({:.1e})",
        relative_error(reverse, proportional),
        relative_error(reverse, alone)
    );
    assert!(relative_error(reverse, proportional) < 1e-2);
}

/// Decision 15's viewport target (`docs/engine.md`, Pipes) on the case's input,
/// darktable's render of mire1 before the module: the viewport, the operation run on the
/// fitted image, against the export downscaled by the viewport's box filter, mean
/// ΔE2000, less the same for the viewport's blend with the export's blurred lights
/// downscaled, the per-pixel gap the blend through Lab shows after the downscale: at most
/// 0.15 at 2400×1600 and at 1/2, 0.4 at 1/4 and 1/8, and within `Tolerance::DARKTABLE`'s
/// mean in all at 2400×1600. Bloom has no per-pixel version that leaves the blur out, so
/// its baseline is decision 15 (4)'s, the per-pixel gap left once the whole-image result
/// is exact (`docs/ports/bloom.md`, Results). Needs darktable-cli and the integration
/// images; run by hand.
#[test]
#[ignore = "needs darktable-cli and APERTRUST_TEST_IMAGES"]
fn the_viewport_is_within_decision_80s_target_of_the_export() {
    use golden::darktable::DarktableCli;
    use golden::delta_e::{self, Tolerance};
    use golden::pfm::Image;
    use golden::{cases, ports, render};

    let darktable = DarktableCli::locate().unwrap();
    let images = render::images_dir().unwrap();
    let downloaded = |plan: Plan, input: &GpuImage| {
        let mut pipeline = pollster::block_on(Pipeline::new(gpu(), plan, input.clone())).unwrap();
        pipeline.render(gpu()).unwrap();
        let output = pipeline.output();
        let desc = output.desc();
        let pixels = output.download(gpu()).unwrap();
        let rgb = pixels.iter().map(|&[r, g, b, _]| [r, g, b]).collect();
        Image::new(desc.width as usize, desc.height as usize, rgb).unwrap()
    };
    let case = cases::find("bloom").unwrap();
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
    let values = ports::plan_values(case).unwrap().unwrap();
    let settings = bloom::settings(&Bloom.schema().resolve(&values).unwrap());
    let export = downloaded(plan(&settings, &input), &input);
    let export_rgba: Vec<[f32; 4]> = export
        .pixels
        .iter()
        .map(|&[r, g, b]| [r, g, b, 1.0])
        .collect();
    let exported = upload(width, height, &export_rgba);
    let blur: Vec<[f32; 4]> = blurred_lights(&settings, width, height, &rgba)
        .into_iter()
        .map(|b| [b, b, b, 1.0])
        .collect();
    let blurred = upload(width, height, &blur);
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
        let viewport = downloaded(plan_for(Some(&settings), &input, pipe), &input);
        let downscaled = downloaded(plan_for(None, &exported, pipe), &exported);
        let fitted = downloaded(plan_for(None, &input, pipe), &input);
        let export_blur = downloaded(plan_for(None, &blurred, pipe), &blurred);
        let held: Vec<[f32; 3]> = fitted
            .pixels
            .iter()
            .zip(&export_blur.pixels)
            .map(|(&rgb, blur)| blend(rgb, blur[0]))
            .collect();
        let held = Image::new(fitted.width, fitted.height, held).unwrap();
        let (_, stats) = delta_e::compare(&downscaled, &viewport, Tolerance::DARKTABLE).unwrap();
        let (_, baseline) = delta_e::compare(&downscaled, &held, Tolerance::DARKTABLE).unwrap();
        let share = stats.mean - baseline.mean;
        eprintln!(
            "bloom at {}×{}: mean {:.4}, p95 {:.4}, max {:.3}; with the export's blur {:.4}; \
             the lights' and blur's share {share:.4}",
            viewport.width, viewport.height, stats.mean, stats.p95, stats.max, baseline.mean
        );
        assert!(share <= target, "at {box_size:?}: {share}, {stats:?}");
        if box_size == [2400, 1600] {
            assert!(stats.mean <= Tolerance::DARKTABLE.mean, "{stats:?}");
        }
    }
}
