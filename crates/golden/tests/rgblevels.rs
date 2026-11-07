// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2019-2026  darktable developers (ported from src/iop/rgblevels.c)
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

//! Invariants of the RGB levels kernel on the GPU, needing no darktable
//! (`docs/ports/rgblevels.md`, Test plan). CI runs them on software GPUs.

use std::sync::OnceLock;

use apertrust_color::{RgbNorm, WorkingProfile};
use apertrust_engine::compute::{GpuImage, ImageKernel};
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_operations::rgblevels::{self, Levels, Mode, Prepared, Settings};

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
            "rgblevels",
            rgblevels::KERNEL.wgsl,
        ))
        .unwrap();
        Gpu { context, kernel }
    })
}

/// The RGB levels kernel on one row of pixels, with these kernel inputs.
fn render_prepared(prepared: &Prepared, pixels: &[[f32; 3]]) -> Vec<[f32; 3]> {
    let Gpu {
        context: gpu,
        kernel: compiled,
    } = gpu();
    let rgba: Vec<[f32; 4]> = pixels.iter().map(|&[r, g, b]| [r, g, b, 1.0]).collect();
    let width = u32::try_from(rgba.len()).unwrap();
    let input = GpuImage::upload(gpu, width, 1, &rgba).unwrap();
    let output = GpuImage::new(gpu, width, 1);
    compiled.dispatch(gpu, &input, &output, &prepared.params(), &[]);
    output
        .download(gpu)
        .unwrap()
        .into_iter()
        .map(|[r, g, b, _]| [r, g, b])
        .collect()
}

fn render(settings: &Settings, pixels: &[[f32; 3]]) -> Vec<[f32; 3]> {
    render_prepared(
        &rgblevels::prepare(settings, &WorkingProfile::linear_rec2020()).unwrap(),
        pixels,
    )
}

/// Every mode: independent channels, then linked with each way of preserving colours.
fn every_mode(levels: [Levels; 3]) -> Vec<Settings> {
    let mut all = vec![Settings {
        mode: Mode::Independent,
        preserve_colors: RgbNorm::Luminance,
        levels,
    }];
    all.extend(RgbNorm::ALL.map(|preserve_colors| Settings {
        mode: Mode::Linked,
        preserve_colors,
        levels,
    }));
    all
}

fn channels(settings: &Settings) -> bool {
    settings.mode == Mode::Independent || settings.preserve_colors == RgbNorm::None
}

/// Greys, colours and out-of-gamut colours from deep shadow to well above white.
fn sweep() -> Vec<[f32; 3]> {
    let mut pixels = Vec::new();
    for level in [0.0, 1e-4, 0.003, 0.012, 0.05, 0.18, 0.5, 0.9, 1.0, 1.5, 4.0] {
        for [r, g, b] in [
            [1.0, 1.0, 1.0],
            [1.0, 0.5, 0.2],
            [0.1, 1.0, 0.3],
            [0.2, 0.3, 1.0],
            [1.0, -0.05, 0.4],
            [-0.3, -0.2, -0.1],
        ] {
            pixels.push([r * level, g * level, b * level]);
        }
    }
    pixels
}

fn close(a: f32, b: f32, tolerance: f32) -> bool {
    (a - b).abs() <= tolerance * b.abs().max(1.0)
}

/// darktable's table entry for t, `(int)(t · 65536)`, within the table's first
/// [`TABLE_STEPS`] entries above black, which the kernel copies; `None` elsewhere.
fn table_step(t: f32) -> Option<f32> {
    (t > 0.0 && t * 65536.0 < TABLE_STEPS).then(|| (t * 65536.0).floor())
}

/// The entries above black the kernel copies from darktable's table.
const TABLE_STEPS: f32 = 4.0;

/// darktable `process()` with the kernel's deliberate differences in value: its table
/// copied only within its first [`TABLE_STEPS`] entries above black, the curve evaluated
/// beyond, and t = (v - black) · scale stopped at the kernel's limit in the power. A
/// channel is 0 at or below black, `(i / 65536)^power` for table entry i, `t^power`
/// beyond; a norm above black scales every channel by its curve over it, which past the
/// limit grows linearly.
fn darktable(settings: &Settings, rgb: [f32; 3]) -> [f32; 3] {
    let levels = |c: usize| match settings.mode {
        Mode::Linked => settings.levels[0],
        Mode::Independent => settings.levels[c],
    };
    let limit = |l: Levels| (64.0 / l.power().max(1.0)).exp2();
    let entry = |l: Levels, step: f32| (step / 65536.0).powf(l.power());
    if channels(settings) {
        return [0, 1, 2].map(|c| {
            let l = levels(c);
            let t = (rgb[c] - l.black) * l.scale();
            match table_step(t) {
                Some(step) => entry(l, step),
                None if t > 0.0 => t.min(limit(l)).powf(l.power()),
                None => 0.0,
            }
        });
    }
    let luminance = WorkingProfile::linear_rec2020().rgb_to_xyz_d50[1];
    let norm = settings.preserve_colors.of(rgb, luminance);
    let l = levels(0);
    if norm <= l.black {
        return [0.0; 3];
    }
    let t = (norm - l.black) * l.scale();
    let ratio = match table_step(t) {
        Some(step) => entry(l, step) / norm,
        None => t / norm * t.min(limit(l)).powf(l.power() - 1.0),
    };
    rgb.map(|c| c * ratio)
}

#[test]
fn the_default_levels_change_only_the_darkest_and_out_of_gamut_pixels() {
    // darktable's defaults, in every mode. A channel or norm from 4/65536 up keeps its
    // value; below, within the table's first entries, it becomes darktable's entry, a
    // multiple of 1/65536. In the modes that put each channel through the levels,
    // independent and linked without a norm, a channel at or below 0 becomes 0, and with
    // a norm, a pixel whose norm is at or below 0 becomes black, both as in darktable.
    // The operation at its defaults is therefore no no-op on out-of-gamut pixels and the
    // darkest ones; the Tone curve's binding owns that invariant
    // (`docs/ports/rgblevels.md`, Tool binding notes).
    let luminance = WorkingProfile::linear_rec2020().rgb_to_xyz_d50[1];
    let entry = |v: f32| table_step(v).map_or(v, |step| step / 65536.0);
    let pixels = sweep();
    for settings in every_mode([Levels::DEFAULT; 3]) {
        for (input, out) in pixels.iter().zip(render(&settings, &pixels)) {
            let norm = settings.preserve_colors.of(*input, luminance);
            let expected = if channels(&settings) {
                input.map(|c| entry(c.max(0.0)))
            } else if norm > 0.0 {
                input.map(|c| c * (entry(norm) / norm))
            } else {
                [0.0; 3]
            };
            assert!(
                (0..3).all(|c| close(out[c], expected[c], 1e-6)),
                "{input:?} gave {out:?} for {settings:?}"
            );
        }
    }
}

/// Test 0054's red levels, and levels across darktable's range: a power below 1, the
/// powers its handles reach at their limits (grey 0.05 from black or white, 0.126 and
/// 7.94), black and white 0.05 apart, and the powers at the ends of what `prepare`
/// accepts, grey at black (0.1) or at white (10), its pickers' reach.
fn level_sets() -> Vec<[Levels; 3]> {
    let levels = |black, grey, white| Levels { black, grey, white };
    let test_0054 = levels(0.011636, 0.355420, 0.6208815);
    vec![
        [
            test_0054,
            levels(0.0630, 0.4458, 0.7481),
            levels(0.0036, 0.4212, 0.6725),
        ],
        [levels(0.02, 0.3, 0.8), test_0054, levels(0.0, 0.05, 1.0)],
        [levels(0.0, 0.95, 1.0), levels(0.2, 0.22, 0.25), test_0054],
        [levels(0.0, 0.05, 1.0); 3],
        [levels(0.0, 0.95, 1.0); 3],
        [levels(0.0, 0.0, 1.0); 3],
        [levels(0.0, 1.0, 1.0); 3],
        [
            levels(0.2, 0.25, 0.25),
            levels(0.2, 0.2, 0.25),
            levels(0.0, 1.0, 1.0),
        ],
    ]
}

#[test]
fn the_kernel_follows_darktables_levels() {
    let pixels = sweep();
    for levels in level_sets() {
        for settings in every_mode(levels) {
            for (input, out) in pixels.iter().zip(render(&settings, &pixels)) {
                let want = darktable(&settings, *input);
                assert!(
                    (0..3).all(|c| close(out[c], want[c], 2e-5)),
                    "{input:?} gave {out:?}, darktable {want:?}, for {settings:?}"
                );
            }
        }
    }
}

#[test]
fn linked_reads_the_first_channels_levels_alone() {
    // Linked, whatever the other channels' levels hold, as darktable's commit_params()
    // overwrites them.
    let pixels = sweep();
    let levels = level_sets()[0];
    for settings in every_mode(levels)
        .into_iter()
        .filter(|s| s.mode == Mode::Linked)
    {
        let mut prepared =
            rgblevels::prepare(&settings, &WorkingProfile::linear_rec2020()).unwrap();
        let expected = render_prepared(&prepared, &pixels);
        prepared.levels[1] = levels[1];
        prepared.levels[2] = levels[2];
        assert_eq!(
            render_prepared(&prepared, &pixels),
            expected,
            "{settings:?}"
        );
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
        [1e6, -1e6, 1e-3],
        [-0.5, 0.2, 0.1],
        [1e-30, 1e-30, 1e-30],
        [1e-30, 0.0, 0.0],
        [100.0, 1e-3, 0.0],
        [0.18, 0.18, 0.180001],
    ];
    for levels in level_sets() {
        for settings in every_mode(levels) {
            for (input, out) in inputs.iter().zip(render(&settings, &inputs)) {
                assert!(
                    out.iter().all(|c| c.is_finite()),
                    "{input:?} gave {out:?} for {settings:?}"
                );
            }
        }
    }
}
