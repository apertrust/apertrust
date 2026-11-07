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

//! The engine's resample on a GPU adapter, hardware or software, against darktable's
//! `dt_interpolation_resample` restated on the CPU (`resample::reference`), down and up,
//! on synthetic images, through an export at a size with nothing but the resample
//! (decision 56; `docs/ports/finalscale.md`).

use apertrust_engine::compute::GpuImage;
use apertrust_engine::export::{Tiling, export_at};
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::pipeline::{self, Limits, Pipe, RESAMPLE, Resample};
use apertrust_engine::resample::{self, AxisPlan, input_length};
use apertrust_plugin::ImageContext;

/// An image with edges, a gradient and fine detail in every channel, alpha included.
fn image(width: u32, height: u32) -> Vec<[f32; 4]> {
    (0..width * height)
        .map(|i| {
            let (x, y) = ((i % width) as f32, (i / width) as f32);
            let check = if (x as u32 / 3 + y as u32 / 5).is_multiple_of(2) {
                0.8
            } else {
                0.1
            };
            [
                check,
                x / width as f32,
                (0.37 * x + 0.11 * y).sin().abs(),
                0.5 + 0.5 * (0.2 * y).cos(),
            ]
        })
        .collect()
}

/// The resample of `input` at `scale`, at `finalscale`'s place's input lengths, on the
/// GPU and on the CPU. Alone in its plan, it runs whole; `tiles.rs` checks its tiles.
fn resampled(
    gpu: &GpuContext,
    input: &[[f32; 4]],
    size: [u32; 2],
    scale: f64,
    finalscale: bool,
) -> (Vec<[f32; 4]>, Vec<[f32; 4]>) {
    let output = size.map(|side| (scale * f64::from(side)).floor() as u32);
    let resample = Resample {
        output,
        scale: scale as f32,
        finalscale,
    };
    let image = ImageContext::full(size[0], size[1]);
    let upload = GpuImage::upload(gpu, size[0], size[1], input).unwrap();
    let whole = pollster::block_on(export_at(
        gpu,
        &[],
        &image,
        upload,
        Tiling::Whole,
        Some(resample),
    ))
    .unwrap();
    assert_eq!([whole.desc.width, whole.desc.height], output);
    let [across, down] = [0, 1].map(|axis| {
        let length = input_length(output[axis], scale as f32, size[axis], finalscale);
        AxisPlan::new(length.min(size[axis]), output[axis], scale as f32)
    });
    let cpu = resample::reference(input, size, &across, &down);
    let plan = pipeline::plan(
        &[],
        &image,
        GpuImage::upload(gpu, size[0], size[1], input)
            .unwrap()
            .desc(),
        Pipe::Resampled(resample),
        &Limits::NONE,
    )
    .unwrap();
    let ids: Vec<&str> = plan.operations.iter().map(|op| op.id).collect();
    assert_eq!(ids, [RESAMPLE.name]);
    (whole.pixels, cpu)
}

/// The GPU's sums are the CPU's, but for the last bits a fused multiply and add may
/// leave.
fn assert_close(gpu: &[[f32; 4]], cpu: &[[f32; 4]], what: &str) {
    assert_eq!(gpu.len(), cpu.len(), "{what}");
    let worst = gpu
        .iter()
        .flatten()
        .zip(cpu.iter().flatten())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(worst < 2e-6, "{what}: {worst}");
}

#[test]
fn the_resample_is_darktables_down_and_up() {
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default()))
        .expect("a GPU adapter, hardware or software");
    let size = [97, 61];
    let input = image(size[0], size[1]);
    for scale in [0.5, 1.0 / 3.0, 0.2154882154882155, 0.77, 1.5, 2.0, 3.25] {
        for finalscale in [false, true] {
            let what = format!("scale {scale}, finalscale {finalscale}");
            let (gpu, cpu) = resampled(&gpu, &input, size, scale, finalscale);
            assert_close(&gpu, &cpu, &what);
        }
    }
}

#[test]
fn a_flat_image_stays_flat() {
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default()))
        .expect("a GPU adapter, hardware or software");
    let size = [64, 40];
    let input = vec![[0.25, 0.5, 0.75, 1.0]; 64 * 40];
    for scale in [0.3, 2.5] {
        let (resampled, _) = resampled(&gpu, &input, size, scale, true);
        for pixel in resampled {
            for (c, value) in pixel.iter().enumerate() {
                assert!((value - input[0][c]).abs() < 1e-5, "{scale}: {pixel:?}");
            }
        }
    }
}
