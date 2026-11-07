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

//! Invariants of the sigmoid kernel on the GPU, needing no darktable
//! (`docs/ports/sigmoid.md`, Test plan): greys follow darktable's curve in both colour
//! processings, output and derivatives stay finite, and the derivatives the golden
//! check's loss cannot see match finite differences. CI runs them on software GPUs.

use std::sync::OnceLock;

use apertrust_color::RgbSpace;
use apertrust_engine::compute::{GpuImage, ImageKernel};
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_operations::sigmoid::{
    self, BasePrimaries, ColorProcessing, MIDDLE_GREY, Prepared, Settings,
};

/// The GPU and the compiled kernel's entry points, made once for every test in this
/// file: compiling takes tens of seconds on a software GPU.
struct Gpu {
    context: GpuContext,
    main: ImageKernel,
    backward: ImageKernel,
    forward: ImageKernel,
}

fn gpu() -> &'static Gpu {
    static GPU: OnceLock<Gpu> = OnceLock::new();
    GPU.get_or_init(|| {
        let context = pollster::block_on(GpuContext::new(&GpuOptions::default()))
            .expect("a GPU adapter, hardware or software");
        let wgsl = sigmoid::KERNEL.wgsl;
        let entry = |name: &str| {
            pollster::block_on(ImageKernel::with_entry_point(
                &context, "sigmoid", wgsl, name,
            ))
            .unwrap()
        };
        let (main, backward, forward) = (
            entry("main"),
            entry("sigmoid_backward"),
            entry("sigmoid_forward_diff"),
        );
        Gpu {
            context,
            main,
            backward,
            forward,
        }
    })
}

fn prepared(settings: &Settings) -> Prepared {
    sigmoid::prepare(settings, &RgbSpace::LINEAR_REC2020)
}

fn upload(pixels: &[[f32; 3]]) -> GpuImage {
    let rgba: Vec<[f32; 4]> = pixels.iter().map(|&[r, g, b]| [r, g, b, 1.0]).collect();
    let width = u32::try_from(rgba.len()).unwrap();
    GpuImage::upload(&gpu().context, width, 1, &rgba).unwrap()
}

/// The kernel's `main` on one row of pixels.
fn render(prepared: &Prepared, pixels: &[[f32; 3]]) -> Vec<[f32; 3]> {
    let Gpu {
        context: gpu, main, ..
    } = gpu();
    let input = upload(pixels);
    let output = GpuImage::new(gpu, input.width(), 1);
    main.dispatch(gpu, &input, &output, &prepared.params(), &[]);
    output
        .download(gpu)
        .unwrap()
        .into_iter()
        .map(|[r, g, b, _]| [r, g, b])
        .collect()
}

fn defaults() -> Settings {
    Settings::default()
}

/// The golden cases' settings, each in both colour processings.
fn settings() -> Vec<Settings> {
    let test_0097 = Settings {
        middle_grey_contrast: 1.788,
        contrast_skewness: 0.36,
        hue_preservation: 50.62,
        ..defaults()
    };
    let primaries = Settings {
        middle_grey_contrast: 1.6,
        contrast_skewness: -0.15,
        display_black_target: 0.05,
        hue_preservation: 70.0,
        inset: [0.12, 0.08, 0.2],
        rotation: [0.0349, -0.0262, -0.0611],
        purity: 0.4,
        base_primaries: BasePrimaries::Srgb,
        ..defaults()
    };
    let ratio = Settings {
        middle_grey_contrast: 1.35,
        contrast_skewness: -0.3,
        display_white_target: 95.0,
        display_black_target: 0.8,
        color_processing: ColorProcessing::RgbRatio,
        ..defaults()
    };
    [defaults(), test_0097, primaries, ratio]
        .into_iter()
        .flat_map(|s| {
            [ColorProcessing::PerChannel, ColorProcessing::RgbRatio].map(|color_processing| {
                Settings {
                    color_processing,
                    ..s
                }
            })
        })
        .collect()
}

#[test]
fn greys_follow_darktables_curve() {
    // The matrices keep white and the hue correction leaves equal channels alone, so a
    // grey goes through the curve alone in either processing, whatever the primaries.
    let levels: Vec<f32> = (0..=120).map(|k| (k as f32 / 4.0 - 20.0).exp2()).collect();
    let greys: Vec<[f32; 3]> = std::iter::once(0.0).chain(levels).map(|v| [v; 3]).collect();
    for settings in settings() {
        let p = prepared(&settings);
        for (grey, out) in greys.iter().zip(render(&p, &greys)) {
            let expected = p.curve.darktable(grey[0]);
            for c in out {
                assert!(
                    (c - expected).abs() <= 1e-4 * expected + 1e-7,
                    "{grey:?} gave {out:?}, darktable {expected} for {settings:?}"
                );
            }
        }
        // Middle grey stays.
        let out = render(&p, &[[MIDDLE_GREY; 3]])[0];
        assert!(
            out.iter().all(|c| (c - MIDDLE_GREY).abs() < 1e-4),
            "{out:?}"
        );
    }
}

#[test]
fn darktables_breakdown_at_the_ranges_corners_is_followed() {
    // Where darktable's f32 paper exposure or film response leaves f32's range: all white
    // at contrast 10, skew -1, white 20%, black 15%, and with the paper exposure
    // underflowed; black with it overflowed but the film response finite, white where
    // that overflows too.
    let corner = |contrast, skew, white, black| Settings {
        middle_grey_contrast: contrast,
        contrast_skewness: skew,
        display_white_target: white,
        display_black_target: black,
        ..defaults()
    };
    let levels = std::iter::once(0.0).chain((0..=200).map(|k| (k as f32 / 2.0 - 40.0).exp2()));
    let greys: Vec<[f32; 3]> = levels.chain([1e30]).map(|v| [v; 3]).collect();
    for settings in [
        corner(10.0, -1.0, 20.0, 15.0),
        corner(9.0, 1.0, 1600.0, 15.0),
        corner(6.0, 1.0, 20.0, 0.0),
    ] {
        for color_processing in [ColorProcessing::PerChannel, ColorProcessing::RgbRatio] {
            let settings = Settings {
                color_processing,
                ..settings
            };
            let p = prepared(&settings);
            assert_ne!(p.curve.paper(), sigmoid::PaperExposure::Finite);
            for (grey, out) in greys.iter().zip(render(&p, &greys)) {
                let expected = p.curve.darktable(grey[0]);
                assert!(expected == 0.0 || expected == p.curve.white_target);
                for c in out {
                    assert!(
                        (c - expected).abs() <= 1e-5 * expected,
                        "{grey:?} gave {out:?}, darktable {expected} for {settings:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn output_and_derivatives_stay_finite() {
    let inputs = [
        [0.0, 0.0, 0.0],
        [1e-30, 1e-30, 1e-30],
        [1e30, 1e30, 1e30],
        [-1e30, -1e30, -1e30],
        [1e30, 0.0, 0.0],
        [-1e30, 1.0, 1.0],
        [1e6, -1e6, 0.5],
        [-0.5, 0.2, 0.1],
        [100.0, 1e-3, 0.0],
        [0.18, 0.18, 0.180001],
        [f32::MAX, f32::MAX, -f32::MAX],
    ];
    let mut all = Vec::new();
    // The corners of darktable's ranges, where its own paper exposure leaves f32's range.
    for contrast in [0.1, 10.0] {
        for skew in [-1.0, 1.0] {
            for white in [20.0, 1600.0] {
                for black in [0.0, 15.0] {
                    all.push(Settings {
                        middle_grey_contrast: contrast,
                        contrast_skewness: skew,
                        display_white_target: white,
                        display_black_target: black,
                        ..defaults()
                    });
                }
            }
        }
    }
    // The primaries at their extremes, in every base.
    for base_primaries in BasePrimaries::ALL {
        for (inset, rotation, purity) in [(0.99, 0.4, 0.0), (0.99, -0.4, 1.0), (0.5, 0.4, 0.5)] {
            all.push(Settings {
                inset: [inset; 3],
                rotation: [rotation, -rotation, rotation],
                purity,
                base_primaries,
                hue_preservation: 0.0,
                ..defaults()
            });
        }
    }
    let Gpu {
        context: gpu,
        backward,
        forward,
        ..
    } = gpu();
    let input = upload(&inputs);
    let out = GpuImage::new(gpu, input.width(), 1);
    let ones = vec![[1.0, 1.0, 1.0, 1.0]; inputs.len()];
    let ones = GpuImage::upload(gpu, input.width(), 1, &ones).unwrap();
    for settings in all.into_iter().flat_map(|s| {
        [ColorProcessing::PerChannel, ColorProcessing::RgbRatio].map(|color_processing| Settings {
            color_processing,
            ..s
        })
    }) {
        let p = prepared(&settings);
        for (input, out) in inputs.iter().zip(render(&p, &inputs)) {
            assert!(
                out.iter().all(|c| c.is_finite()),
                "{input:?} gave {out:?} for {settings:?}"
            );
        }
        for (name, kernel) in [("reverse", backward), ("forward", forward)] {
            kernel.dispatch_derivative(gpu, &input, &ones, &out, &p.params(), &[]);
            let derivatives = out.download(gpu).unwrap();
            for (input, d) in inputs.iter().zip(derivatives) {
                assert!(
                    d.iter().all(|c| c.is_finite()),
                    "{name} mode at {input:?} gave {d:?} for {settings:?}"
                );
            }
        }
    }
}

/// `prepared` with gradient slot `slot` moved by `delta`, in the kernel's own units, as
/// the golden harness moves it (`ports.rs`).
fn moved(prepared: &Prepared, slot: usize, delta: f32) -> Prepared {
    let mut p = *prepared;
    match (slot, p.color_processing) {
        (0, _) => p.curve.film_power += delta,
        (1, _) => p.curve.paper_power += delta,
        (2, _) => p.curve.white_target += delta,
        (3, ColorProcessing::PerChannel) => p.hue_preservation += delta,
        (3, ColorProcessing::RgbRatio) => p.curve.black_target += delta,
        _ => unreachable!(),
    }
    p
}

#[test]
fn derivatives_match_finite_differences_with_each_pixels_own_weights() {
    // The golden check's loss, R + G + B, cannot see RGB ratio's chroma compression,
    // which keeps the channels' mean, and so not the target black at all. Nor can any
    // loss weighting the channels alike at every pixel on the synthetic sweep, whose
    // hues cancel such a change out. This loss weights each pixel's channels by its own
    // colour, `Σ (w + max(input, 0)) · output` with w (0.25, 0.5, 1).
    const WEIGHTS: [f32; 3] = [0.25, 0.5, 1.0];
    let Gpu {
        context: gpu,
        main,
        backward,
        forward,
    } = gpu();
    let sweep = golden::synthetic::sweep();
    let (width, height) = (sweep.width as u32, sweep.height as u32);
    let rgba: Vec<[f32; 4]> = sweep
        .pixels
        .iter()
        .map(|&[r, g, b]| [r, g, b, 1.0])
        .collect();
    let pixels = rgba.len();
    let input = GpuImage::upload(gpu, width, height, &rgba).unwrap();
    let out = GpuImage::new(gpu, width, height);
    let weights: Vec<[f32; 4]> = sweep
        .pixels
        .iter()
        .map(|rgb| {
            let w = |c: usize| WEIGHTS[c] + rgb[c].max(0.0);
            [w(0), w(1), w(2), 0.0]
        })
        .collect();
    let loss = |pixels: &[[f32; 4]]| -> f64 {
        pixels
            .iter()
            .zip(&weights)
            .map(|(p, w)| (0..3).map(|c| f64::from(w[c] * p[c])).sum::<f64>())
            .sum()
    };
    let weights = GpuImage::upload(gpu, width, height, &weights).unwrap();
    let step = 1e-3;
    for settings in settings() {
        let p = prepared(&settings);
        backward.dispatch_derivative(gpu, &input, &weights, &out, &p.params(), &[]);
        let reverse = out.download(gpu).unwrap();
        for slot in 0..4 {
            let reverse: f64 = reverse.iter().map(|g| f64::from(g[slot])).sum();
            let mut direction = [0.0; 4];
            direction[slot] = 1.0;
            let directions =
                GpuImage::upload(gpu, width, height, &vec![direction; pixels]).unwrap();
            forward.dispatch_derivative(gpu, &input, &directions, &out, &p.params(), &[]);
            let forward = loss(&out.download(gpu).unwrap());
            let at = |delta: f32| {
                main.dispatch(gpu, &input, &out, &moved(&p, slot, delta).params(), &[]);
                loss(&out.download(gpu).unwrap())
            };
            let numeric = (at(step) - at(-step)) / (2.0 * f64::from(step));
            let scale = numeric.abs().max(reverse.abs());
            // Shown with --nocapture, for the spec's Gradients section.
            println!(
                "{:?} slot {slot}: reverse {reverse:.6e}, finite differences {numeric:.6e} \
                 ({:.1e}), forward {forward:.6e} ({:.1e})",
                settings.color_processing,
                (reverse - numeric).abs() / scale,
                (reverse - forward).abs() / scale
            );
            assert!(
                (reverse - numeric).abs() <= 1e-2 * scale,
                "slot {slot}: reverse {reverse}, finite differences {numeric}, for {settings:?}"
            );
            assert!(
                (reverse - forward).abs() <= 1e-4 * scale,
                "slot {slot}: reverse {reverse}, forward {forward}, for {settings:?}"
            );
        }
    }
}
