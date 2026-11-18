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

//! colorin's gamut clipping and blue mapping on the GPU, needing no darktable
//! (`docs/ports/colorin.md`, Test plan): the clamp to 0 to 1 in the clipping profile's
//! linear RGB at its edges, upper bound included, and blue mapping as darktable's
//! `_apply_blue_mapping`, against the kernel's uniforms applied on the CPU. CI runs them
//! on software GPUs.

use std::sync::OnceLock;

use apertrust_color::{Mat3d, inverse_d};
use apertrust_engine::compute::{GpuImage, ImageKernel};
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_operations::colorin::{self, Colorin, GAMUT_CLIPPING, PROFILES};
use apertrust_plugin::{ImageContext, ImageDesc, Operation, ParamValues, Prepared, Value};

struct Gpu {
    context: GpuContext,
    main: ImageKernel,
}

fn gpu() -> &'static Gpu {
    static GPU: OnceLock<Gpu> = OnceLock::new();
    GPU.get_or_init(|| {
        let context = pollster::block_on(GpuContext::new(&GpuOptions::default()))
            .expect("a GPU adapter, hardware or software");
        let main = pollster::block_on(ImageKernel::new(&context, "colorin", colorin::KERNEL.wgsl))
            .unwrap();
        Gpu { context, main }
    })
}

/// A colour raw whose matrix is XYZ to linear sRGB under D65: the tools' small Bayer raw
/// (`apertrust_tools::testing::image`), made 8 × 8 with its white at 1.
fn raw() -> ImageContext {
    let mut camera = apertrust_tools::testing::image()
        .camera
        .expect("the tools' test raw has a camera");
    camera.width = 8;
    camera.height = 8;
    camera.white = 1.0;
    camera.xyz_to_camera = Some([
        [3.240_454, -1.537_138_5, -0.498_531_4],
        [-0.969_266, 1.876_010_8, 0.041_556],
        [0.055_643_4, -0.204_025_9, 1.057_225_2],
    ]);
    camera.crop = [0, 0, 8, 8];
    camera.declared = [0, 0, 8, 8];
    ImageContext {
        camera: Some(camera),
        ..ImageContext::full(8, 8)
    }
}

fn prepare(image: &ImageContext, set: &[(&str, Value)]) -> Prepared {
    let mut values = ParamValues::new();
    for (name, value) in set {
        values.set(name, *value);
    }
    let params = Colorin
        .schema()
        .resolve_over(&Colorin.defaults(image), &values)
        .unwrap();
    Colorin
        .prepare(&params, image, &ImageDesc::rgba(8, 8))
        .unwrap()
}

/// The kernel's two matrices, by rows, as it reads them.
fn matrices(prepared: &Prepared) -> [[[f32; 3]; 3]; 2] {
    let bytes = &prepared.passes[0].params;
    let float = |at: usize| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    [0, 48].map(|base| [0, 1, 2].map(|r| [0, 1, 2].map(|c| float(base + 16 * r + 4 * c))))
}

fn apply(m: &[[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    m.map(|row| row[0] * v[0] + row[1] * v[1] + row[2] * v[2])
}

/// `main` on these pixels, alpha 1.
fn render(prepared: &Prepared, pixels: &[[f32; 3]]) -> Vec<[f32; 3]> {
    let Gpu { context, main } = gpu();
    let rgba: Vec<[f32; 4]> = pixels.iter().map(|&[r, g, b]| [r, g, b, 1.0]).collect();
    let width = rgba.len() as u32;
    let input = GpuImage::upload(context, width, 1, &rgba).unwrap();
    let output = GpuImage::new(context, width, 1);
    main.dispatch(context, &input, &output, &prepared.passes[0].params, &[]);
    output
        .download(context)
        .unwrap()
        .into_iter()
        .map(|[r, g, b, a]| {
            assert_eq!(a, 1.0);
            [r, g, b]
        })
        .collect()
}

fn close(a: [f32; 3], b: [f32; 3], tolerance: f32) -> bool {
    a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= tolerance)
}

/// Camera RGB that the kernel's first matrix takes to `clipped`, solved in f64.
fn camera_for(rows: &[[f32; 3]; 3], clipped: [f64; 3]) -> [f32; 3] {
    let m: Mat3d = rows.map(|row| row.map(f64::from));
    let inverse = inverse_d(&m);
    inverse.map(|row| (row[0] * clipped[0] + row[1] * clipped[1] + row[2] * clipped[2]) as f32)
}

/// Each clipping profile clamps its linear RGB to 0 to 1, darktable's `dt_vector_clip`:
/// a channel just below 0 renders as 0, one just above 1 as 1, the bounds themselves and
/// what lies between them unchanged.
#[test]
fn gamut_clipping_clamps_each_channel_to_0_to_1_upper_bound_included() {
    let image = raw();
    for (index, name) in GAMUT_CLIPPING.iter().enumerate().skip(1) {
        let prepared = prepare(&image, &[("normalize", Value::Choice(index as u32))]);
        let [rows, clipped] = matrices(&prepared);
        let targets: [[f64; 3]; 6] = [
            [-1e-3, 0.5, 0.5],
            [0.0, 0.5, 0.5],
            [0.25, 1.0, 0.5],
            [0.25, 1.0 + 1e-3, 0.5],
            [-0.4, 0.3, 1.6],
            [0.2, 0.4, 0.6],
        ];
        let pixels: Vec<[f32; 3]> = targets.iter().map(|&t| camera_for(&rows, t)).collect();
        let rendered = render(&prepared, &pixels);
        for ((pixel, target), out) in pixels.iter().zip(&targets).zip(&rendered) {
            let expected = apply(&clipped, apply(&rows, *pixel).map(|v| v.clamp(0.0, 1.0)));
            assert!(
                close(*out, expected, 1e-5),
                "{name} {target:?}: {out:?} {expected:?}"
            );
            let clamped = target.map(|v| v.clamp(0.0, 1.0) as f32);
            assert!(
                close(*out, apply(&clipped, clamped), 1e-5),
                "{name} {target:?}: {out:?}"
            );
        }
        // Just outside each bound renders as the bound.
        assert!(close(rendered[0], rendered[1], 1e-5), "{name}");
        assert!(close(rendered[2], rendered[3], 1e-5), "{name}");
    }
}

/// Blue mapping, through linear Rec2020 in so that the matrix is near the identity: the
/// GPU moves blue to green as darktable's `_apply_blue_mapping` does.
#[test]
fn blue_mapping_on_the_gpu_is_darktable_s() {
    let image = raw();
    let rec2020 = PROFILES.iter().position(|&p| p == "lin_rec2020").unwrap() as u32;
    let prepared = prepare(
        &image,
        &[
            ("type", Value::Choice(rec2020)),
            ("blue_mapping", Value::Bool(true)),
        ],
    );
    let [rows, _] = matrices(&prepared);
    let pixels = [
        [0.1, 0.1, 0.8],
        [0.05, 0.05, 0.15],
        [0.2, 0.3, 1.5],
        [0.25, 0.25, 0.5],
        [0.5, 0.3, 0.2],
        [0.0, 0.0, 0.0],
        [-0.5, 0.1, 0.3],
        [0.01, 0.02, 0.4],
    ];
    let rendered = render(&prepared, &pixels);
    for (pixel, out) in pixels.iter().zip(&rendered) {
        let expected = apply(&rows, colorin::blue_mapping(*pixel));
        assert!(
            close(*out, expected, 1e-6),
            "{pixel:?}: {out:?} {expected:?}"
        );
    }
    // It moved the first three and the eighth, and none of the others.
    let plain = render(
        &prepare(&image, &[("type", Value::Choice(rec2020))]),
        &pixels,
    );
    for (k, (mapped, plain)) in rendered.iter().zip(&plain).enumerate() {
        let moved = matches!(k, 0 | 1 | 2 | 7);
        assert_eq!(!close(*mapped, *plain, 1e-4), moved, "{:?}", pixels[k]);
    }
}
