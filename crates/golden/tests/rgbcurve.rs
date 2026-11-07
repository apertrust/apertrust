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

//! Invariants of the RGB curve kernel on the GPU, needing no darktable
//! (`docs/ports/rgbcurve.md`, Test plan). CI runs them on software GPUs.

use std::sync::OnceLock;

use apertrust_color::{WorkingProfile, curve};
use apertrust_engine::compute::{GpuImage, ImageKernel};
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_operations::rgbcurve::{
    self, Curve, Interpolation, Knot, Mode, Node, Norm, Settings,
};

/// The GPU and the compiled kernel, made once for every test in this file: compiling
/// takes tens of seconds on a software GPU.
struct Gpu {
    context: GpuContext,
    kernel: ImageKernel,
}

fn gpu() -> &'static Gpu {
    static GPU: OnceLock<Gpu> = OnceLock::new();
    GPU.get_or_init(|| {
        let context = pollster::block_on(GpuContext::new(&GpuOptions::default()))
            .expect("a GPU adapter, hardware or software");
        let kernel = pollster::block_on(ImageKernel::new(
            &context,
            "rgbcurve",
            rgbcurve::KERNEL.wgsl,
        ))
        .unwrap();
        Gpu { context, kernel }
    })
}

/// The RGB curve kernel on one row of pixels.
fn render(settings: &Settings, pixels: &[[f32; 3]]) -> Vec<[f32; 3]> {
    let Gpu {
        context: gpu,
        kernel: compiled,
    } = gpu();
    let rgba: Vec<[f32; 4]> = pixels.iter().map(|&[r, g, b]| [r, g, b, 1.0]).collect();
    let width = u32::try_from(rgba.len()).unwrap();
    let input = GpuImage::upload(gpu, width, 1, &rgba).unwrap();
    let output = GpuImage::new(gpu, width, 1);
    let prepared = rgbcurve::prepare(settings, &WorkingProfile::linear_rec2020()).unwrap();
    compiled.dispatch(
        gpu,
        &input,
        &output,
        &prepared.params(),
        &[&prepared.knots()],
    );
    output
        .download(gpu)
        .unwrap()
        .into_iter()
        .map(|[r, g, b, _]| [r, g, b])
        .collect()
}

/// Every mode: independent channels, then linked with each way of preserving colours,
/// with middle grey compensated and not.
fn every_mode(curves: [Curve; 3]) -> Vec<Settings> {
    let mut all = Vec::new();
    for compensate_middle_grey in [false, true] {
        all.push(Settings {
            curves: curves.clone(),
            mode: Mode::Independent,
            compensate_middle_grey,
            preserve_colors: Norm::Luminance,
        });
        for preserve_colors in Norm::ALL {
            all.push(Settings {
                curves: curves.clone(),
                mode: Mode::Linked,
                compensate_middle_grey,
                preserve_colors,
            });
        }
    }
    all
}

/// Greys, colours and out-of-gamut colours from deep shadow to well above white.
fn sweep() -> Vec<[f32; 3]> {
    let mut pixels = Vec::new();
    for level in [0.0, 1e-4, 0.003, 0.05, 0.18, 0.5, 0.9, 1.0, 1.5, 4.0] {
        for [r, g, b] in [
            [1.0, 1.0, 1.0],
            [1.0, 0.5, 0.2],
            [0.1, 1.0, 0.3],
            [0.2, 0.3, 1.0],
            [1.0, -0.05, 0.4],
        ] {
            pixels.push([r * level, g * level, b * level]);
        }
    }
    pixels
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() <= 1e-5 * b.abs().max(1.0)
}

#[test]
fn the_identity_curve_changes_only_negative_channels() {
    // darktable's default curve, in every mode. With a norm it changes nothing: a norm
    // of 0 or less leaves the pixel as it is. In the modes that put each channel through
    // a curve, independent and linked without a norm, a channel below the first node
    // takes its y, so a negative channel becomes 0, as in darktable. The operation at
    // its defaults is therefore no no-op on out-of-gamut pixels in those modes; the
    // Tone curve's binding owns that invariant (`docs/ports/rgbcurve.md`, Tool
    // binding notes).
    let identity = [Curve::identity(), Curve::identity(), Curve::identity()];
    let pixels = sweep();
    for settings in every_mode(identity) {
        let clips = settings.mode == Mode::Independent || settings.preserve_colors == Norm::None;
        for (input, out) in pixels.iter().zip(render(&settings, &pixels)) {
            let expected = input.map(|c| if clips { c.max(0.0) } else { c });
            assert!(
                (0..3).all(|c| close(out[c], expected[c])),
                "{input:?} gave {out:?} for {settings:?}"
            );
        }
    }
}

/// Test 0060's curves (`crates/golden/cases/rgbcurve.xmp`), with an interpolation each.
fn test_0060_curves(interpolation: Interpolation) -> [Curve; 3] {
    let curve = |points: &[[f32; 2]]| Curve {
        nodes: points.iter().map(|&[x, y]| Node { x, y }).collect(),
        interpolation,
    };
    [
        curve(&[
            [0.056114, 0.0],
            [0.125585, 0.13339],
            [0.182916, 0.290952],
            [0.277205, 0.397451],
            [0.409264, 0.563564],
            [0.5911, 0.654994],
            [0.797724, 0.764868],
            [0.882749, 0.906857],
            [1.0, 1.0],
        ]),
        curve(&[
            [0.0, 0.0],
            [0.224564, 0.197457],
            [0.549181, 0.447213],
            [0.711884, 0.76362],
            [0.902742, 1.0],
        ]),
        curve(&[
            [0.0, 0.0],
            [0.155665, 0.380814],
            [0.501287, 0.600884],
            [0.703589, 0.89668],
            [1.0, 1.0],
        ]),
    ]
}

#[test]
fn the_kernel_follows_the_curve() {
    // Independent channels: each output channel is its curve at the input, as the CPU
    // twin of the kernel's curve evaluates it below the last node, and darktable's
    // power law past it.
    let pixels = sweep();
    for interpolation in [
        Interpolation::CubicSpline,
        Interpolation::CatmullRom,
        Interpolation::MonotoneHermite,
    ] {
        let settings = Settings {
            curves: test_0060_curves(interpolation),
            mode: Mode::Independent,
            compensate_middle_grey: true,
            preserve_colors: Norm::Luminance,
        };
        let prepared = rgbcurve::prepare(&settings, &WorkingProfile::linear_rec2020()).unwrap();
        let expected = |c: usize, v: f32| {
            let curve = &prepared.curves[c];
            let knots: &[Knot] = &curve.knots;
            let last = knots[knots.len() - 1].x;
            if v < last {
                curve::evaluate(knots, v)
            } else {
                curve.end_value * (v / last).powf(curve.end_power)
            }
        };
        for (input, out) in pixels.iter().zip(render(&settings, &pixels)) {
            for c in 0..3 {
                let want = expected(c, input[c]);
                assert!(
                    (out[c] - want).abs() <= 1e-5 * want.abs().max(1.0),
                    "{interpolation:?}: channel {c} of {input:?} gave {}, the curve {want}",
                    out[c]
                );
            }
        }
    }
}

#[test]
fn output_stays_finite() {
    let inputs = [
        [0.0, 0.0, 0.0],
        [1e6, 1e6, 1e6],
        [-1e6, -1e6, -1e6],
        [1e6, 0.0, 0.0],
        [-1e6, 1.0, 1.0],
        [-0.5, 0.2, 0.1],
        [1e-30, 1e-30, 1e-30],
        [100.0, 1e-3, 0.0],
        [0.18, 0.18, 0.180001],
    ];
    // A curve that lifts black, so a tiny norm meets a large ratio, and one ending at
    // 0, whose extrapolation is flat.
    let lifted = Curve {
        nodes: vec![
            Node { x: 0.0, y: 0.1 },
            Node { x: 0.5, y: 0.7 },
            Node { x: 1.0, y: 1.0 },
        ],
        interpolation: Interpolation::CubicSpline,
    };
    let falling = Curve {
        nodes: vec![Node { x: 0.0, y: 1.0 }, Node { x: 0.8, y: 0.0 }],
        interpolation: Interpolation::CatmullRom,
    };
    let mut settings = every_mode(test_0060_curves(Interpolation::CubicSpline));
    settings.extend(every_mode([lifted.clone(), lifted, falling.clone()]));
    settings.extend(every_mode([falling.clone(), falling.clone(), falling]));
    for settings in settings {
        for (input, out) in inputs.iter().zip(render(&settings, &inputs)) {
            assert!(
                out.iter().all(|c| c.is_finite()),
                "{input:?} gave {out:?} for {settings:?}"
            );
        }
    }
}
