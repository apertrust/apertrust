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

//! Invariants of filmic rgb on the GPU, needing no darktable
//! (`docs/ports/filmicrgb.md`, Test plan): middle grey in every colour science, alpha,
//! the highlight reconstruction's no-op where nothing clips and its work where
//! something does, and finite output and derivatives for extreme input and settings.
//! Each runs the operation as a one-instance plan of the engine, as the golden cases do.
//! CI runs them on software GPUs.

use std::sync::OnceLock;

use apertrust_engine::compute::GpuImage;
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::pipeline::{self, Derivative, Instance, Pipe, Pipeline, Plan};
use apertrust_operations::filmicrgb::{
    self, ColorScience, CurveType, FilmicRgb, Noise, Norm, Settings,
};
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

fn plan(settings: &Settings, input: &GpuImage) -> Plan {
    plan_for(Some(settings), input, Pipe::Export)
}

/// The operation with `settings`, or nothing, planned over `input` for `pipe`.
fn plan_for(settings: Option<&Settings>, input: &GpuImage, pipe: Pipe) -> Plan {
    let desc = input.desc();
    let instances: Vec<Instance> = settings
        .map(|settings| Instance {
            operation: &FilmicRgb,
            params: FilmicRgb
                .schema()
                .resolve(&filmicrgb::values(settings))
                .unwrap(),
        })
        .into_iter()
        .collect();
    let image = ImageContext::full(desc.width, desc.height);
    let limits = pipeline::Limits::of(gpu());
    pipeline::plan(&instances, &image, desc, pipe, &limits).unwrap()
}

fn render(settings: &Settings, input: &GpuImage) -> Vec<[f32; 4]> {
    let mut pipeline =
        pollster::block_on(Pipeline::new(gpu(), plan(settings, input), input.clone())).unwrap();
    pipeline.render(gpu()).unwrap();
    pipeline.output().download(gpu()).unwrap()
}

/// Greys, colours and saturated colours from deep shadow to above white, in a 64×48
/// image; alpha varies, so it is seen to pass through.
fn image() -> (u32, u32, Vec<[f32; 4]>) {
    let (width, height) = (64, 48);
    let pixels = (0..height)
        .flat_map(|y| {
            (0..width).map(move |x| {
                let level = 0.002 * 2f32.powf(f32::from(y as u8) / 4.0);
                let hue = x as f32 / width as f32 * std::f32::consts::TAU;
                let saturation = if x % 16 < 8 { 0.0 } else { 0.6 };
                let rgb = [0.0, 2.1, 4.2]
                    .map(|phase: f32| level * (1.0 + saturation * (hue + phase).cos()));
                [rgb[0], rgb[1], rgb[2], (x % 7) as f32 / 6.0]
            })
        })
        .collect();
    (width, height, pixels)
}

const SCIENCES: [ColorScience; 5] = [
    ColorScience::V3,
    ColorScience::V4,
    ColorScience::V5,
    ColorScience::V6,
    ColorScience::V7,
];

/// The norms whose value for a grey is the grey: all but the legacy euclidean norm,
/// which darktable keeps √3 times larger.
const GREY_NORMS: [Norm; 5] = [
    Norm::None,
    Norm::MaxRgb,
    Norm::LuminanceY,
    Norm::PowerNorm,
    Norm::Euclidean,
];

#[test]
fn middle_grey_goes_to_the_grey_target_in_every_colour_science() {
    let input = upload(4, 4, &[[0.1845, 0.1845, 0.1845, 1.0]; 16]);
    for version in SCIENCES {
        for preserve_color in GREY_NORMS {
            for curve in [CurveType::Hard, CurveType::Soft, CurveType::Safe] {
                let settings = Settings {
                    version,
                    preserve_color,
                    shadows: curve,
                    highlights: curve,
                    latitude: 30.0,
                    ..Settings::default()
                };
                let out = render(&settings, &input);
                for &value in &out[0][..3] {
                    assert!(
                        (value - 0.1845).abs() < 2e-3 * 0.1845,
                        "{version:?} {preserve_color:?} {curve:?}: {value}"
                    );
                }
            }
        }
    }
}

#[test]
fn alpha_passes_through_and_greys_rise() {
    let (width, height, pixels) = image();
    let input = upload(width, height, &pixels);
    for version in SCIENCES {
        let settings = Settings {
            version,
            shadows: CurveType::Safe,
            highlights: CurveType::Safe,
            latitude: 20.0,
            ..Settings::default()
        };
        let out = render(&settings, &input);
        for (p, q) in pixels.iter().zip(&out) {
            assert_eq!(p[3], q[3], "{version:?}");
        }
        // The greys, the first eight columns, rise with the level, but for the ripple of
        // darktable's approximate power at the white, a few 1e-7.
        for y in 1..height as usize {
            let (above, below) = (out[(y - 1) * width as usize], out[y * width as usize]);
            assert!(
                below[1] >= above[1] - 1e-6,
                "{version:?} at row {y}: {} after {}",
                below[1],
                above[1]
            );
        }
    }
}

#[test]
fn below_its_zero_the_saturation_renders_v4_and_v5_black() {
    // A saturation below -50 %, its coefficient below 0: darktable's desaturation is the
    // NaN of a negative's square root, which its clip of the curve's value (no norm) and
    // its MAX of the ratios (a norm) take to 0, a uniform black (docs/ports/filmicrgb.md,
    // Precision).
    let (width, height, pixels) = image();
    let input = upload(width, height, &pixels);
    for version in [ColorScience::V4, ColorScience::V5] {
        for preserve_color in [Norm::None, Norm::PowerNorm, Norm::Euclidean] {
            let settings = Settings {
                version,
                preserve_color,
                saturation: -100.0,
                ..Settings::default()
            };
            let out = render(&settings, &input);
            assert!(
                out.iter().all(|p| p[..3] == [0.0; 3]),
                "{version:?} {preserve_color:?}"
            );
        }
        // At -50 %, the coefficient's 0, its limit renders: middle grey stays bright.
        let at_zero = Settings {
            version,
            saturation: -50.0,
            ..Settings::default()
        };
        let grey = upload(1, 1, &[[0.1845, 0.1845, 0.1845, 1.0]]);
        assert!(render(&at_zero, &grey)[0][1] > 0.1, "{version:?}");
    }
}

#[test]
fn a_grey_source_of_0_renders_v3_to_v5_white() {
    // darktable divides each pixel by the grey source, 0, and its clamp takes the log of
    // the infinity to 1: every pixel the curve's white, here 1 less darktable's
    // approximate power at 1 (docs/ports/filmicrgb.md, Precision).
    let (width, height, pixels) = image();
    let input = upload(width, height, &pixels);
    for version in [ColorScience::V3, ColorScience::V4, ColorScience::V5] {
        let settings = Settings {
            version,
            preserve_color: Norm::None,
            custom_grey: true,
            grey_point_source: 0.0,
            ..Settings::default()
        };
        let out = render(&settings, &input);
        for p in &out {
            for &value in &p[..3] {
                assert!((value - 1.0).abs() < 1e-4, "{version:?}: {p:?}");
            }
        }
    }
    // v6 and v7 take the grey source at 0.001 %, a guard where darktable renders NaN.
    for version in [ColorScience::V6, ColorScience::V7] {
        let settings = Settings {
            version,
            custom_grey: true,
            grey_point_source: 0.0,
            ..Settings::default()
        };
        let out = render(&settings, &input);
        assert!(out.iter().flatten().all(|v| v.is_finite()), "{version:?}");
    }
}

fn reconstruction(threshold: f32) -> Settings {
    Settings {
        enable_highlight_reconstruction: true,
        reconstruct_threshold: threshold,
        noise_distribution: Noise::Uniform,
        ..Settings::default()
    }
}

#[test]
fn the_reconstruction_changes_nothing_where_too_little_clips() {
    // At +6 EV over a white of +4 EV nothing reaches the mask's clipping, so darktable's
    // count of more than 9 pixels skips the reconstruction.
    let (width, height, pixels) = image();
    let input = upload(width, height, &pixels);
    let off = render(&Settings::default(), &input);
    let on = render(&reconstruction(6.0), &input);
    assert_eq!(off, on);
}

#[test]
fn the_reconstruction_rebuilds_a_clipped_area() {
    // A gradient with a block of 64 flat pixels, clipped below the white (+4 EV) and
    // over the mask's threshold at -2 EV from it.
    let (width, height) = (48, 32);
    let pixels: Vec<[f32; 4]> = (0..width * height)
        .map(|i| {
            let (x, y) = (i % width, i / width);
            let clipped = (20..28).contains(&x) && (12..20).contains(&y);
            let v = if clipped {
                2.0
            } else {
                0.05 + 0.002 * x as f32
            };
            [v, v * 0.9, v * 0.8, 1.0]
        })
        .collect();
    let input = upload(width, height, &pixels);
    let off = render(&Settings::default(), &input);
    let centre = (16 * width + 24) as usize;
    let corner = 0;
    for settings in [
        reconstruction(-2.0),
        Settings {
            high_quality_reconstruction: 3,
            noise_distribution: Noise::Poissonian,
            reconstruct_bloom_vs_details: -100.0,
            ..reconstruction(-2.0)
        },
    ] {
        let on = render(&settings, &input);
        assert_ne!(on[centre], off[centre], "{settings:?}");
        // Far from it the mask is nearly 0, so the image barely moves.
        for c in 0..3 {
            assert!(
                (on[corner][c] - off[corner][c]).abs() < 1e-3,
                "{settings:?}"
            );
        }
        assert!(on.iter().flatten().all(|v| v.is_finite()));
    }
}

/// Settings at the ends of the parameters' ranges, over every colour science, norm and
/// curve.
fn extremes() -> Vec<Settings> {
    let mut settings = Vec::new();
    for version in SCIENCES {
        for preserve_color in [
            Norm::None,
            Norm::MaxRgb,
            Norm::LuminanceY,
            Norm::PowerNorm,
            Norm::EuclideanLegacy,
            Norm::Euclidean,
        ] {
            for (k, curve) in [CurveType::Hard, CurveType::Soft, CurveType::Safe]
                .into_iter()
                .enumerate()
            {
                let at_end = k % 2 == 0;
                settings.push(Settings {
                    version,
                    preserve_color,
                    shadows: curve,
                    highlights: curve,
                    contrast: if at_end { 5.0 } else { 0.0 },
                    latitude: if at_end { 99.0 } else { 0.01 },
                    output_power: if at_end { 10.0 } else { 1.0 },
                    saturation: if at_end { 200.0 } else { -200.0 },
                    balance: if at_end { 50.0 } else { -50.0 },
                    black_point_source: if at_end { -0.1 } else { -16.0 },
                    white_point_source: if at_end { 16.0 } else { 0.1 },
                    black_point_target: if at_end { 20.0 } else { 0.0 },
                    white_point_target: if at_end { 1600.0 } else { 0.0 },
                    custom_grey: at_end,
                    grey_point_source: 0.0,
                    grey_point_target: 50.0,
                    ..Settings::default()
                });
            }
        }
    }
    settings
}

#[test]
fn output_and_derivatives_stay_finite_for_extreme_input_and_settings() {
    let inputs = [
        [0.0, 0.0, 0.0],
        [1e-30, 1e-30, 1e-30],
        [-1.0, 0.5, 0.2],
        [-1e3, -1e3, -1e3],
        [1e6, 1e6, 1e6],
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 1e3],
        [0.1845, 0.1845, 0.1845],
    ];
    let pixels: Vec<[f32; 4]> = inputs.iter().map(|&[r, g, b]| [r, g, b, 1.0]).collect();
    let (width, height) = (pixels.len() as u32, 1);
    let input = upload(width, height, &pixels);
    let ones = upload(width, height, &vec![[1.0, 1.0, 1.0, 0.0]; pixels.len()]);
    let directions = upload(width, height, &vec![[1.0, 1.0, 1.0, 1.0]; pixels.len()]);
    let d_out = GpuImage::new(gpu(), width, height);
    let all = extremes();
    let mut pipeline =
        pollster::block_on(Pipeline::new(gpu(), plan(&all[0], &input), input.clone())).unwrap();
    for settings in &all {
        pipeline = pollster::block_on(pipeline.update(gpu(), plan(settings, &input))).unwrap();
        pipeline.render(gpu()).unwrap();
        let out = pipeline.output().download(gpu()).unwrap();
        assert!(
            out.iter().flatten().all(|v| v.is_finite()),
            "{settings:?}: {out:?}"
        );
        for mode in [Derivative::Backward, Derivative::ForwardDiff] {
            let along = if mode == Derivative::Backward {
                &ones
            } else {
                &directions
            };
            pollster::block_on(pipeline.derivative(gpu(), mode, along, &d_out)).unwrap();
            let d = d_out.download(gpu()).unwrap();
            assert!(
                d.iter().flatten().all(|v| v.is_finite()),
                "{mode:?} {settings:?}: {d:?}"
            );
        }
    }
}

/// Decision 15's viewport target (`docs/engine.md`, Pipes) on the cases whose
/// reconstruction runs, on their input, darktable's render of mire1 before the module:
/// the viewport against the export downscaled by the viewport's box filter, less the
/// same comparison with the reconstruction off, the whole-image passes' filtering left
/// out and the tone mapping kept, at most 0.15 mean ΔE2000 at 2400×1600 and at 1/2 and
/// 0.4 at 1/4 and 1/8. Needs darktable and the test images, so run by hand:
/// `cargo test --release -p apertrust-golden --test filmicrgb -- --ignored --nocapture`.
#[test]
#[ignore]
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
    let upload_image = |image: &Image| {
        let rgba: Vec<[f32; 4]> = image
            .pixels
            .iter()
            .map(|&[r, g, b]| [r, g, b, 1.0])
            .collect();
        upload(image.width as u32, image.height as u32, &rgba)
    };
    for name in [
        "filmicrgb-reconstruction",
        "filmicrgb-v6",
        "filmicrgb-v7",
        "filmicrgb-v4-nochroma",
        "filmicrgb-v6-euclidean",
    ] {
        let case = cases::find(name).unwrap();
        let source = Image::read(
            &render::render_case(&darktable, case, &images)
                .unwrap()
                .input,
        )
        .unwrap();
        let (width, height) = (source.width as u32, source.height as u32);
        let input = upload_image(&source);
        let values = ports::plan_values(case).unwrap().unwrap();
        let settings = filmicrgb::settings(&FilmicRgb.schema().resolve(&values).unwrap());
        let unfiltered = Settings {
            enable_highlight_reconstruction: false,
            ..settings
        };
        let gap = |settings: &Settings, pipe: Pipe| {
            let export = upload_image(&downloaded(plan(settings, &input), &input));
            let viewport = downloaded(plan_for(Some(settings), &input, pipe), &input);
            let downscaled = downloaded(plan_for(None, &export, pipe), &export);
            let (_, stats) =
                delta_e::compare(&downscaled, &viewport, Tolerance::DARKTABLE).unwrap();
            stats.mean
        };
        for (size, target) in [
            ([2400, 1600], 0.15),
            ([width / 2, height / 2], 0.15),
            ([width / 4, height / 4], 0.4),
            ([width / 8, height / 8], 0.4),
        ] {
            let pipe = Pipe::Viewport {
                width: size[0],
                height: size[1],
            };
            let (filtered, baseline) = (gap(&settings, pipe), gap(&unfiltered, pipe));
            eprintln!(
                "{name} at {}×{}: mean {filtered:.4}, without reconstruction {baseline:.4}",
                size[0], size[1]
            );
            assert!(
                filtered - baseline <= target,
                "{name} at {size:?}: {filtered} against {baseline}"
            );
        }
    }
}
