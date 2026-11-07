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

//! The color equalizer on the GPU against its restatement on the CPU, and its
//! invariants, needing no darktable (`docs/ports/colorequal.md`, Test plan): the
//! operation equals darktable's `process()` restated loop by loop
//! (`apertrust_op_colorequal::reference`, on the colour library's Gaussian and bilinear
//! resampling), without the guided filter and with it, on grids that are the image and
//! grids downsampled; alpha passes through; no setting is an identity; output stays
//! finite; and the four gradient slots against finite differences of whole renders.
//! Each runs the operation as a one-instance plan of the engine, as the golden cases do.
//! CI runs them on software GPUs.

use std::sync::OnceLock;

use apertrust_color::WorkingProfile;
use apertrust_engine::compute::GpuImage;
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::pipeline::{self, Derivative, Instance, Pipe, Pipeline, Plan};
use apertrust_operations::colorequal::reference::{self, InPlace};
use apertrust_operations::colorequal::{self, ColorEqual, Grid, Process, Settings, Setup};
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
    plan_in(
        settings,
        input,
        pipe,
        &ImageContext::full(desc.width, desc.height),
    )
}

/// The same, the image as `image` says: an `input` at a smaller scale than its full
/// resolution, as a smaller view renders it.
fn plan_in(
    settings: Option<&Settings>,
    input: &GpuImage,
    pipe: Pipe,
    image: &ImageContext,
) -> Plan {
    let desc = input.desc();
    let instances: Vec<Instance> = settings
        .map(|settings| Instance {
            operation: &ColorEqual,
            params: ColorEqual
                .schema()
                .resolve(&colorequal::values(settings))
                .unwrap(),
        })
        .into_iter()
        .collect();
    let limits = pipeline::Limits::of(gpu());
    pipeline::plan(&instances, image, desc, pipe, &limits).unwrap()
}

fn plan(settings: &Settings, input: &GpuImage) -> Plan {
    plan_for(Some(settings), input, Pipe::Export)
}

fn render(settings: &Settings, input: &GpuImage) -> Vec<[f32; 4]> {
    rendered(plan(settings, input), input)
}

fn rendered(plan: Plan, input: &GpuImage) -> Vec<[f32; 4]> {
    let mut pipeline = pollster::block_on(Pipeline::new(gpu(), plan, input.clone())).unwrap();
    pipeline.render(gpu()).unwrap();
    pipeline.output().download(gpu()).unwrap()
}

/// A colour of hue `hue` in radians, saturation `saturation` and value `value`, with
/// cosines in place of HSV's ramps, so every hue is reached smoothly.
fn colour(hue: f32, saturation: f32, value: f32) -> [f32; 3] {
    let channel = |offset: f32| value * (1.0 - saturation * (0.5 - 0.5 * (hue - offset).cos()));
    [
        channel(0.0),
        channel(2.0 * std::f32::consts::PI / 3.0),
        channel(4.0 * std::f32::consts::PI / 3.0),
    ]
}

/// Every hue across, in patches of 4 × 3 pixels of saturation (0 to 0.9) and value
/// (0.02 to 1.6) down, so the filters see edges and flat areas, and alpha that varies,
/// so it is seen to pass through.
fn patches(width: u32, height: u32) -> Vec<[f32; 4]> {
    (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let hue = 2.0 * std::f32::consts::PI * (x / 4) as f32 / (width / 4) as f32;
            let saturation = [0.0, 0.08, 0.3, 0.6, 0.9][((y / 3) % 5) as usize];
            let value = [0.02, 0.18, 0.6, 1.6][((y / 15) % 4) as usize];
            let [r, g, b] = colour(hue, saturation, value);
            [r, g, b, (x % 7) as f32 / 6.0]
        })
        .collect()
}

/// Test 0159's curves (docs/ports/colorequal.md, Test plan), migrated to version 4.
fn curves(settings: Settings) -> Settings {
    let soften = |v: f32| 1.0 + 0.75 * (v - 1.0);
    Settings {
        saturation: [1.3297, 1.0, 0.667, 1.0, 1.3939, 1.0, 0.6185, 1.0].map(soften),
        hue: [42.75, -22.49, 0.0, 0.0, 40.9, 0.0, -44.75, 0.0],
        brightness: [1.0, 1.1675, 1.0, 0.8463, 1.0, 1.3491, 1.0, 0.6816].map(soften),
        ..settings
    }
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
    let (width, height) = (64, 60);
    let pixels = patches(width, height);
    let input = upload(width, height, &pixels);
    let defaults = Settings::default();
    // Without the filter; with it at darktable's defaults, both grids the image; test
    // 0159's radii, the guide's grid a fourth of the image; both grids downsampled, by 3,
    // with the other settings off their defaults.
    let cases = [
        Settings {
            use_filter: false,
            white_level: 0.9,
            ..curves(defaults)
        },
        defaults,
        Settings {
            chroma_size: 2.0,
            param_size: 25.0,
            threshold: 0.124,
            ..curves(defaults)
        },
        Settings {
            chroma_size: 10.0,
            param_size: 9.0,
            threshold: 0.2,
            contrast: 0.5,
            white_level: 0.5,
            smoothing_hue: 0.3,
            hue_shift: 10.0,
            ..curves(defaults)
        },
    ];
    for settings in cases {
        let scale = 1.0;
        let prefilter = Grid::new(Process::new(&settings, scale).hue_sigma, [width, height]);
        let guide = Grid::new(Process::new(&settings, scale).par_sigma, [width, height]);
        let ours = render(&settings, &input);
        let theirs = reference::process(
            &settings,
            width as usize,
            height as usize,
            &pixels,
            scale,
            &WorkingProfile::linear_rec2020(),
            InPlace::Darktable,
        );
        let gap = largest_relative_difference(&theirs, &ours);
        let effect = largest_relative_difference(&pixels, &ours);
        eprintln!(
            "filter {}, grids {:?} and {:?}: gap {gap:.2e}, effect {effect:.2e}",
            settings.use_filter, prefilter.size, guide.size
        );
        assert!(gap < 2e-4, "{settings:?}: {gap}");
        assert!(effect > 1e-2, "{settings:?}: {effect}");
        assert!(ours.iter().zip(&pixels).all(|(o, p)| o[3] == p[3]));
    }
}

/// A smaller view does the export's processing at its scale (docs/ports/colorequal.md,
/// Neighbourhood and scale): test 0159's effect radius of 25 and a hue analysis radius
/// of 10, whose export downsamples their grids by 4 and 3, at a fourth, where both
/// grids are the image and darktable's preview would blur in place. The operation
/// equals the restatement whose filters blur copies, as the export does, and differs
/// from darktable's preview, whose guide blurs the hue correction.
#[test]
fn a_smaller_view_does_the_exports_processing_at_its_scale() {
    let (width, height) = (64, 60);
    let scale = 0.25;
    let image = ImageContext {
        scale,
        ..ImageContext::full(4 * width, 4 * height)
    };
    let pixels = patches(width, height);
    let input = upload(width, height, &pixels);
    let settings = Settings {
        chroma_size: 10.0,
        param_size: 25.0,
        threshold: 0.124,
        ..curves(Settings::default())
    };
    let profile = WorkingProfile::linear_rec2020();
    let setup = Setup::new(&settings, [width, height], scale, &profile);
    for (grid, full) in [(setup.prefilter, 5.0), (setup.guide, 12.5)] {
        assert!(!grid.resized && !grid.in_place, "{grid:?}");
        assert!(Grid::new(full, [4 * width, 4 * height]).resized, "{full}");
    }
    let ours = rendered(
        plan_in(Some(&settings), &input, Pipe::Export, &image),
        &input,
    );
    let restated = |in_place| {
        reference::process(
            &settings,
            width as usize,
            height as usize,
            &pixels,
            scale,
            &profile,
            in_place,
        )
    };
    let gap = largest_relative_difference(&restated(InPlace::Export), &ours);
    let preview = largest_relative_difference(&restated(InPlace::Darktable), &ours);
    eprintln!("at a fourth: gap {gap:.2e}; from darktable's preview {preview:.2e}");
    assert!(gap < 2e-4, "{gap}");
    assert!(preview > 1e-2, "{preview}");
    assert!(ours.iter().zip(&pixels).all(|(o, p)| o[3] == p[3]));
}

/// darktable's color equalizer has no setting that leaves the image unchanged
/// (docs/ports/colorequal.md, Tool binding notes): at its defaults the guided filter
/// smooths the chromaticity the output is rebuilt from, the curves through nodes at 1
/// ripple between them, and the gamut mapping compresses saturated colours, so its
/// module is switched (decision 32).
#[test]
fn no_setting_is_an_identity() {
    let (width, height) = (64, 60);
    let pixels = patches(width, height);
    let input = upload(width, height, &pixels);
    for settings in [
        Settings::default(),
        Settings {
            use_filter: false,
            ..Settings::default()
        },
    ] {
        let ours = render(&settings, &input);
        let effect = largest_relative_difference(&pixels, &ours);
        eprintln!(
            "filter {} at the defaults: {effect:.2e}",
            settings.use_filter
        );
        assert!(effect > 1e-3, "{settings:?}: {effect}");
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
    let defaults = Settings::default();
    let extremes = |filter: bool, low: bool| Settings {
        use_filter: filter,
        threshold: if low { 0.0 } else { 0.3 },
        smoothing_hue: if low { 0.05 } else { 2.0 },
        contrast: if low { -1.0 } else { 1.0 },
        white_level: if low { -2.0 } else { 16.0 },
        chroma_size: if low { 1.0 } else { 10.0 },
        param_size: if low { 1.0 } else { 128.0 },
        saturation: [if low { 0.0 } else { 2.0 }; 8],
        hue: [if low { -180.0 } else { 180.0 }; 8],
        brightness: [if low { 0.0 } else { 2.0 }; 8],
        hue_shift: if low { -23.0 } else { 23.0 },
    };
    for settings in [
        defaults,
        curves(defaults),
        extremes(true, true),
        extremes(true, false),
        extremes(false, true),
        extremes(false, false),
    ] {
        let output = render(&settings, &input);
        assert!(
            output.iter().flatten().all(|v| v.is_finite()),
            "{settings:?}: {output:?}"
        );
    }
}

/// The sum of an image's RGB, the gradient check's loss.
fn loss(pixels: &[[f32; 4]]) -> f64 {
    pixels
        .iter()
        .map(|p| f64::from(p[0]) + f64::from(p[1]) + f64::from(p[2]))
        .sum()
}

/// Reverse and forward mode of the loss with respect to each gradient slot at `base`.
fn derivatives(base: &Settings, input: &GpuImage) -> [(f64, f64); 4] {
    let desc = input.desc();
    let count = (desc.width * desc.height) as usize;
    let mut pipeline =
        pollster::block_on(Pipeline::new(gpu(), plan(base, input), input.clone())).unwrap();
    pipeline.render(gpu()).unwrap();
    let d_out = GpuImage::new(gpu(), desc.width, desc.height);
    let ones = upload(desc.width, desc.height, &vec![[1.0, 1.0, 1.0, 0.0]; count]);
    pollster::block_on(pipeline.derivative(gpu(), Derivative::Backward, &ones, &d_out)).unwrap();
    let per_pixel = d_out.download(gpu()).unwrap();
    std::array::from_fn(|slot| {
        let reverse: f64 = per_pixel.iter().map(|p| f64::from(p[slot])).sum();
        let mut direction = [0.0; 4];
        direction[slot] = 1.0;
        let directions = upload(desc.width, desc.height, &vec![direction; count]);
        pollster::block_on(pipeline.derivative(
            gpu(),
            Derivative::ForwardDiff,
            &directions,
            &d_out,
        ))
        .unwrap();
        (reverse, loss(&d_out.download(gpu()).unwrap()))
    })
}

/// `settings` with gradient slot `slot` moved by `delta`, in the slots' order: the hue,
/// saturation and brightness curves' red nodes, then the white level.
fn moved(settings: &Settings, slot: usize, delta: f32) -> Settings {
    let mut moved = *settings;
    match slot {
        0 => moved.hue[0] += delta,
        1 => moved.saturation[0] += delta,
        2 => moved.brightness[0] += delta,
        _ => moved.white_level += delta,
    }
    moved
}

fn relative_error(actual: f64, expected: f64) -> f64 {
    (actual - expected).abs() / expected.abs().max(1e-6)
}

/// Without the guided filter every slot is exact (docs/ports/colorequal.md, Gradients),
/// the white level included, whose finite differences on test 0158 near-grey pixels
/// crossing darktable's neutral threshold outweigh: here every pixel is well saturated.
#[test]
fn the_slots_derivatives_are_the_whole_renders_without_the_filter() {
    let (width, height) = (64, 48);
    let pixels: Vec<[f32; 4]> = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let hue = 2.0 * std::f32::consts::PI * x as f32 / width as f32;
            let [r, g, b] = colour(hue, 0.4 + 0.5 * (y % 4) as f32 / 3.0, 0.1 + y as f32 / 30.0);
            [r, g, b, 1.0]
        })
        .collect();
    let input = upload(width, height, &pixels);
    let base = Settings {
        use_filter: false,
        white_level: 0.9,
        ..curves(Settings::default())
    };
    for (slot, (reverse, forward)) in derivatives(&base, &input).into_iter().enumerate() {
        let step = 1e-2;
        let numeric = (loss(&render(&moved(&base, slot, step), &input))
            - loss(&render(&moved(&base, slot, -step), &input)))
            / f64::from(2.0 * step);
        eprintln!("slot {slot}: reverse {reverse:.6e}, finite differences {numeric:.6e}");
        assert!(
            relative_error(reverse, numeric) < 1e-2,
            "slot {slot}: {reverse} against {numeric}"
        );
        assert!(
            relative_error(forward, reverse) < 1e-4,
            "slot {slot}: {forward}, {reverse}"
        );
    }
}

/// Decision 15's viewport target (`docs/engine.md`, Pipes) on test 0159's input,
/// darktable's render of mire1 before the module: the viewport, the operation run on the
/// fitted image, against the export downscaled by the viewport's box filter, mean
/// ΔE2000, less decision 15's baseline, the same gap for the same edit without its
/// filtering, darktable's own mode without the guided filter, as the tone equalizer's
/// baseline is its unfiltered mask: at most 0.15 at 2400×1600 and at 1/2, 0.4 at 1/4 and
/// 1/8, and within `Tolerance::DARKTABLE`'s mean in all at 2400×1600. It also prints
/// decision 15 (4)'s baseline, the last step at the viewport's pixels with the export's
/// prefiltered chromaticity and guided corrections downscaled, restated on the CPU, which
/// also counts the curves' own per-pixel gap, since the corrections come out of a
/// whole-image pass (docs/ports/colorequal.md, Results). Needs darktable-cli and the
/// integration images; run by hand.
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
        (desc.width as usize, desc.height as usize, pixels)
    };
    let image = |(width, height, pixels): (usize, usize, Vec<[f32; 4]>)| {
        let rgb = pixels.iter().map(|&[r, g, b, _]| [r, g, b]).collect();
        Image::new(width, height, rgb).unwrap()
    };
    let case = cases::find("colorequal").unwrap();
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
    let settings = colorequal::settings(&ColorEqual.schema().resolve(&values).unwrap());
    let unfiltered = Settings {
        use_filter: false,
        ..settings
    };
    let profile = WorkingProfile::linear_rec2020();
    let exported = |settings: &Settings| {
        let export = downloaded(plan(settings, &input), &input).2;
        upload(width, height, &export)
    };
    let (exported, exported_unfiltered) = (exported(&settings), exported(&unfiltered));
    // The export's whole-image results, as two images to downscale: the prefiltered
    // chromaticity and the hue correction, then the saturation and brightness ones.
    let guided = reference::guided(
        &settings,
        width as usize,
        height as usize,
        &rgba,
        1.0,
        &profile,
        InPlace::Darktable,
    );
    let first: Vec<[f32; 4]> = guided
        .uv
        .iter()
        .zip(&guided.corrections)
        .map(|(uv, c)| [uv[0], uv[1], c[0], 1.0])
        .collect();
    let second: Vec<[f32; 4]> = guided
        .corrections
        .iter()
        .map(|c| [c[1], c[2], 0.0, 1.0])
        .collect();
    let (first, second) = (
        upload(width, height, &first),
        upload(width, height, &second),
    );
    let mut failures = Vec::new();
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
        let gap = |settings: &Settings, exported: &GpuImage| {
            let viewport = image(downloaded(plan_for(Some(settings), &input, pipe), &input));
            let downscaled = image(downloaded(plan_for(None, exported, pipe), exported));
            delta_e::compare(&downscaled, &viewport, Tolerance::DARKTABLE)
                .unwrap()
                .1
        };
        let stats = gap(&settings, &exported);
        let baseline = gap(&unfiltered, &exported_unfiltered);

        let downscaled = image(downloaded(plan_for(None, &exported, pipe), &exported));
        let (fitted_width, fitted_height, fitted) =
            downloaded(plan_for(None, &input, pipe), &input);
        let first = downloaded(plan_for(None, &first, pipe), &first).2;
        let second = downloaded(plan_for(None, &second, pipe), &second).2;
        let setup = colorequal::Setup::new(
            &settings,
            [fitted_width as u32, fitted_height as u32],
            1.0,
            &profile,
        );
        let held: Vec<[f32; 3]> = fitted
            .iter()
            .zip(first.iter().zip(&second))
            .map(|(pixel, (a, b))| {
                let l_star = reference::l_star(&setup, [pixel[0], pixel[1], pixel[2]]);
                reference::finish(&setup, l_star, [a[0], a[1]], [a[2], b[0], b[1]])
            })
            .collect();
        let held = Image::new(fitted_width, fitted_height, held).unwrap();
        let (_, exact) = delta_e::compare(&downscaled, &held, Tolerance::DARKTABLE).unwrap();

        let share = stats.mean - baseline.mean;
        eprintln!(
            "colorequal at {fitted_width}×{fitted_height}: mean {:.4}, p95 {:.4}, max {:.3}; \
             unfiltered {:.4}; the filter's share {share:.4}; with the export's whole-image \
             results {:.4}",
            stats.mean, stats.p95, stats.max, baseline.mean, exact.mean
        );
        if share > target {
            failures.push(format!("at {box_size:?}: {share}, {stats:?}"));
        }
        if box_size == [2400, 1600] && stats.mean > Tolerance::DARKTABLE.mean {
            failures.push(format!("at {box_size:?}: {stats:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
