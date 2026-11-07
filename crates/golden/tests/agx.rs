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

//! Invariants of the AgX kernel on the GPU, needing no darktable (`docs/ports/agx.md`,
//! Test plan). CI runs them on software GPUs.

use apertrust_color::RgbSpace;
use apertrust_engine::compute::{GpuImage, ImageKernel};
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_operations::agx::{self, BasePrimaries, Settings};

/// The AgX kernel on one row of pixels.
fn render(settings: &Settings, pixels: &[[f32; 3]]) -> Vec<[f32; 3]> {
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default()))
        .expect("a GPU adapter, hardware or software");
    let kernel = agx::KERNEL;
    let compiled = pollster::block_on(ImageKernel::new(&gpu, "agx", kernel.wgsl)).unwrap();
    let rgba: Vec<[f32; 4]> = pixels.iter().map(|&[r, g, b]| [r, g, b, 1.0]).collect();
    let width = u32::try_from(rgba.len()).unwrap();
    let input = GpuImage::upload(&gpu, width, 1, &rgba).unwrap();
    let output = GpuImage::new(&gpu, width, 1);
    let params = agx::prepare(settings, &RgbSpace::LINEAR_REC2020)
        .unwrap()
        .params();
    compiled.dispatch(&gpu, &input, &output, &params, &[]);
    output
        .download(&gpu)
        .unwrap()
        .into_iter()
        .map(|[r, g, b, _]| [r, g, b])
        .collect()
}

fn defaults() -> Settings {
    Settings::scene_referred_default()
}

/// Range -2 to +2.5 EV with the pivot at mid-grey: both fallback curves.
fn narrow(base_primaries: BasePrimaries) -> Settings {
    let (black, white) = (-2.0, 2.5);
    Settings {
        range_black_relative_ev: black,
        range_white_relative_ev: white,
        curve_pivot_x: -black / (white - black),
        base_primaries,
        completely_reverse_primaries: true,
        ..defaults()
    }
}

#[test]
fn mid_grey_renders_to_the_pivot_target() {
    // With the pivot at mid-grey and a neutral look, grey stays grey through every
    // matrix and the curve sends 0.18 to the pivot target output.
    for settings in [
        defaults(),
        Settings {
            base_primaries: BasePrimaries::AdobeRgb,
            ..defaults()
        },
        narrow(BasePrimaries::Srgb),
        narrow(BasePrimaries::DisplayP3),
    ] {
        let out = render(&settings, &[[0.18; 3]])[0];
        for c in out {
            assert!((c - 0.18).abs() < 2e-4, "{out:?} for {settings:?}");
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
    for settings in [
        defaults(),
        narrow(BasePrimaries::Srgb),
        // Pivot target output at target white with the hardest toe and shoulder, where
        // darktable's curve is NaN above the pivot.
        Settings {
            curve_pivot_y_linear_output: 1.0,
            curve_shoulder_power: 10.0,
            curve_toe_power: 10.0,
            ..defaults()
        },
        // The sliders' other extremes.
        Settings {
            look_brightness: 0.0,
            look_slope: 10.0,
            look_lift: -1.0 + 1e-3,
            look_saturation: 10.0,
            look_original_hue_mix_ratio: 1.0,
            curve_contrast_around_pivot: 0.1,
            curve_gamma: 0.01,
            curve_target_display_black_ratio: 0.15,
            curve_target_display_white_ratio: 0.2,
            ..defaults()
        },
        Settings {
            auto_gamma: true,
            curve_pivot_y_linear_output: 1.0,
            curve_contrast_around_pivot: 10.0,
            curve_linear_ratio_below_pivot: 1.0,
            curve_linear_ratio_above_pivot: 1.0,
            range_black_relative_ev: -20.0,
            range_white_relative_ev: 20.0,
            curve_pivot_x: 0.5,
            ..defaults()
        },
        // Two outbound primaries on the white, where darktable reads an
        // uninitialised matrix and the setup renders black.
        Settings {
            master_outset_ratio: 2.0,
            outset: [0.5, 0.5, 0.0],
            ..defaults()
        },
    ] {
        for (input, out) in inputs.iter().zip(render(&settings, &inputs)) {
            assert!(
                out.iter().all(|c| c.is_finite()),
                "{input:?} gave {out:?} for {settings:?}"
            );
        }
    }
}
