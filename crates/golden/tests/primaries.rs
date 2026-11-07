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

//! Invariants of the RGB primaries kernel on the GPU, needing no darktable
//! (`docs/ports/primaries.md`, Test plan): the matrix applied, greys at the defaults,
//! finite output and derivatives at every range end, and every setting's derivative
//! against finite differences of the setup's f64 matrix. CI runs them on software GPUs.

use std::f32::consts::PI;
use std::sync::OnceLock;

use apertrust_color::{Mat3d, RgbSpace};
use apertrust_engine::compute::{GpuImage, ImageKernel};
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_operations::primaries::{self, Parameter, Prepared, Settings};

struct Gpu {
    context: GpuContext,
    main: ImageKernel,
    backward: ImageKernel,
    forward: ImageKernel,
}

/// The GPU and the compiled kernels, made once for every test in this file: compiling
/// takes tens of seconds on a software GPU.
fn gpu() -> &'static Gpu {
    static GPU: OnceLock<Gpu> = OnceLock::new();
    GPU.get_or_init(|| {
        let context = pollster::block_on(GpuContext::new(&GpuOptions::default()))
            .expect("a GPU adapter, hardware or software");
        let entry = |name: &str| {
            pollster::block_on(ImageKernel::with_entry_point(
                &context,
                "primaries",
                primaries::KERNEL.wgsl,
                name,
            ))
            .unwrap()
        };
        let (main, backward, forward) = (
            entry("main"),
            entry("primaries_backward"),
            entry("primaries_forward_diff"),
        );
        Gpu {
            context,
            main,
            backward,
            forward,
        }
    })
}

fn rgba(pixels: &[[f32; 3]]) -> Vec<[f32; 4]> {
    // Alpha varies, so it is seen to pass through.
    pixels
        .iter()
        .enumerate()
        .map(|(i, &[r, g, b])| [r, g, b, (i % 7) as f32 / 6.0])
        .collect()
}

fn prepare(settings: &Settings) -> Prepared {
    primaries::prepare(settings, &RgbSpace::LINEAR_REC2020)
}

/// `main` on one row of pixels.
fn render(prepared: &Prepared, pixels: &[[f32; 3]]) -> Vec<[f32; 4]> {
    let Gpu { context, main, .. } = gpu();
    let width = u32::try_from(pixels.len()).unwrap();
    let input = GpuImage::upload(context, width, 1, &rgba(pixels)).unwrap();
    let output = GpuImage::new(context, width, 1);
    main.dispatch(context, &input, &output, &prepared.params(), &[]);
    output.download(context).unwrap()
}

/// A derivative entry point on one row of pixels, with the same `derivative_in` for
/// each.
fn derivative(
    kernel: &ImageKernel,
    prepared: &Prepared,
    pixels: &[[f32; 3]],
    d_in: [f32; 4],
) -> Vec<[f32; 4]> {
    let context = &gpu().context;
    let width = u32::try_from(pixels.len()).unwrap();
    let input = GpuImage::upload(context, width, 1, &rgba(pixels)).unwrap();
    let d_in = GpuImage::upload(context, width, 1, &vec![d_in; pixels.len()]).unwrap();
    let d_out = GpuImage::new(context, width, 1);
    kernel.dispatch_derivative(context, &input, &d_in, &d_out, &prepared.params(), &[]);
    d_out.download(context).unwrap()
}

/// Test 0152's settings (`crates/golden/cases/primaries.xmp`).
fn test_0152() -> Settings {
    Settings {
        achromatic_tint_hue: 0.579_449_4,
        achromatic_tint_purity: 0.134,
        red_hue: 0.165_806_32,
        red_purity: 0.843,
        green_hue: -0.143_117_01,
        green_purity: 1.235,
        blue_hue: 0.228_638_14,
        blue_purity: 0.82,
    }
}

/// Every setting off its default and away from test 0152's: blue turned into the sector
/// opposite it, a stronger tint, purities on both sides of 1. No ray passes through a
/// corner of the triangle.
fn far() -> Settings {
    Settings {
        achromatic_tint_hue: -2.1,
        achromatic_tint_purity: 0.45,
        red_hue: -0.31,
        red_purity: 0.62,
        green_hue: 0.42,
        green_purity: 2.4,
        blue_hue: 2.8,
        blue_purity: 1.37,
    }
}

/// Greys, colours, out-of-gamut colours and channels at zero, from deep shadow to well
/// above white.
fn sweep() -> Vec<[f32; 3]> {
    let mut pixels = Vec::new();
    for level in [0.0, 1e-4, 0.018, 0.18, 1.0, 4.0, 100.0] {
        for [r, g, b] in [
            [1.0, 1.0, 1.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.5, 0.2],
            [0.1, 1.0, 0.3],
            [1.0, -0.05, 0.4],
            [-0.3, -0.2, -0.1],
        ] {
            pixels.push([r * level, g * level, b * level]);
        }
    }
    pixels
}

fn apply(m: &Mat3d, v: [f32; 3]) -> [f64; 3] {
    m.map(|row| (0..3).map(|c| row[c] * f64::from(v[c])).sum())
}

#[test]
fn the_kernel_applies_the_setups_matrix_and_keeps_alpha() {
    let pixels = sweep();
    for settings in [Settings::default(), test_0152(), far()] {
        let m = primaries::adjustment(&settings, &RgbSpace::LINEAR_REC2020);
        let out = render(&prepare(&settings), &pixels);
        for (i, (input, out)) in pixels.iter().zip(&out).enumerate() {
            let expected = apply(&m, *input);
            let size = input.iter().map(|v| v.abs()).fold(0.0, f32::max);
            for c in 0..3 {
                let gap = (f64::from(out[c]) - expected[c]).abs();
                assert!(
                    gap <= 1e-6 * f64::from(size.max(1e-30)),
                    "{input:?} gave {out:?}"
                );
            }
            assert_eq!(out[3], (i % 7) as f32 / 6.0);
        }
    }
}

#[test]
fn greys_stay_grey_at_the_defaults() {
    // Every row of the defaults' matrix adds up to 1: white goes to white whatever the
    // primaries, while the tint's purity is 0.
    let pixels: Vec<[f32; 3]> = [0.0, 1e-6, 0.018, 0.18, 1.0, 16.0, 1e4]
        .map(|v| [v; 3])
        .to_vec();
    for (input, out) in pixels
        .iter()
        .zip(render(&prepare(&Settings::default()), &pixels))
    {
        for c in 0..3 {
            assert!(
                (out[c] - input[c]).abs() <= 2.5e-7 * input[c],
                "{input:?} gave {out:?}"
            );
        }
    }
}

/// Settings at each end of each range, from the defaults and from test 0152's.
fn range_ends() -> Vec<Settings> {
    let ends = |parameter: Parameter| match parameter {
        Parameter::AchromaticTintPurity => [0.0, 0.99],
        Parameter::RedPurity | Parameter::GreenPurity | Parameter::BluePurity => [0.01, 5.0],
        _ => [-PI, PI],
    };
    let mut all = Vec::new();
    for base in [Settings::default(), test_0152()] {
        for parameter in Parameter::ALL {
            for end in ends(parameter) {
                let mut settings = base;
                *settings.get_mut(parameter) = end;
                all.push(settings);
            }
        }
    }
    all
}

#[test]
fn output_and_derivatives_stay_finite_at_every_range_end() {
    // A primary turned onto the opposite edge at purity 1 is singular, and the
    // adjustment 0: the defaults with red, green or blue at ±π are among these.
    let pixels = [
        [0.0, 0.0, 0.0],
        [1e-30, 1e-30, 1e-30],
        [-1.0, -0.5, -2.0],
        [1e6, 1e6, 1e6],
        [1e6, 0.0, -1e6],
        [0.18, 0.02, 0.9],
    ];
    let mut singular = 0;
    for settings in range_ends() {
        let mut prepared = prepare(&settings);
        singular += usize::from(prepared.singular);
        let out = render(&prepared, &pixels);
        assert!(out.iter().flatten().all(|v| v.is_finite()), "{settings:?}");
        for slots in [&Parameter::ALL[..4], &Parameter::ALL[4..]] {
            prepared.gradient = slots.try_into().unwrap();
            let forward = derivative(&gpu().forward, &prepared, &pixels, [1.0; 4]);
            let backward = derivative(&gpu().backward, &prepared, &pixels, [1.0; 4]);
            assert!(
                forward
                    .iter()
                    .chain(&backward)
                    .flatten()
                    .all(|v| v.is_finite()),
                "{settings:?}"
            );
            if prepared.singular {
                assert!(forward.iter().chain(&backward).flatten().all(|v| *v == 0.0));
            }
        }
    }
    assert_eq!(
        singular, 6,
        "red, green and blue at -π and π from the defaults"
    );
}

/// The f64 matrix's derivative along one setting, by central differences of the f32
/// setting, divided by the step it actually took.
fn finite_difference(settings: &Settings, parameter: Parameter) -> Mat3d {
    let step = 1e-3;
    let (mut plus, mut minus) = (*settings, *settings);
    *plus.get_mut(parameter) += step;
    *minus.get_mut(parameter) -= step;
    let width = f64::from(plus.get(parameter)) - f64::from(minus.get(parameter));
    let (a, b) = (
        primaries::adjustment(&plus, &RgbSpace::LINEAR_REC2020),
        primaries::adjustment(&minus, &RgbSpace::LINEAR_REC2020),
    );
    std::array::from_fn(|r| std::array::from_fn(|c| (a[r][c] - b[r][c]) / width))
}

#[test]
fn every_settings_derivative_is_the_setups() {
    // The derivative entry points build the matrix again in f32 from the settings; its
    // derivative must be that of `prepare`'s f64 matrix, which `main` applies. Per
    // pixel, with unequal channel weights: summed over a grey-on-average image, the
    // derivative of a primary's hue or purity cancels, since grey's image is the white,
    // whatever the primaries (`docs/ports/primaries.md`, Gradients).
    let pixels = [
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.18, 0.18, 0.18],
        [0.9, 0.3, 0.05],
        [-0.2, 0.6, 1.4],
    ];
    let weights = [0.3, 0.5, 0.2, 0.0];
    for settings in [test_0152(), far()] {
        let mut prepared = prepare(&settings);
        for slots in [&Parameter::ALL[..4], &Parameter::ALL[4..]] {
            prepared.gradient = slots.try_into().unwrap();
            let backward = derivative(&gpu().backward, &prepared, &pixels, weights);
            for (k, parameter) in slots.iter().enumerate() {
                let mut direction = [0.0; 4];
                direction[k] = 1.0;
                let forward = derivative(&gpu().forward, &prepared, &pixels, direction);
                let expected = finite_difference(&settings, *parameter);
                let scale = expected
                    .iter()
                    .flatten()
                    .fold(0.0, |m: f64, v| m.max(v.abs()));
                for (i, input) in pixels.iter().enumerate() {
                    let fd = apply(&expected, *input);
                    let size = f64::from(input.iter().map(|v| v.abs()).fold(0.0, f32::max));
                    for c in 0..3 {
                        let gap = (f64::from(forward[i][c]) - fd[c]).abs();
                        assert!(
                            gap <= 1e-4 * scale * size,
                            "{parameter:?} on {input:?}: forward {:?}, f64 {fd:?}",
                            forward[i]
                        );
                    }
                    // Reverse mode: the weights against forward mode's output.
                    let along: f64 = (0..3)
                        .map(|c| f64::from(weights[c]) * f64::from(forward[i][c]))
                        .sum();
                    let gap = (f64::from(backward[i][k]) - along).abs();
                    assert!(
                        gap <= 1e-5 * scale * size,
                        "{parameter:?} on {input:?}: reverse {}, forward {along}",
                        backward[i][k]
                    );
                }
            }
        }
    }
}
