// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/iop/vignette.c)
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

//! Invariants of the vignetting kernel on the GPU, needing no darktable
//! (`docs/ports/vignette.md`, Test plan): it follows darktable's `process()`, is a no-op
//! at zero strength, stays finite, has the derivatives finite differences see, and lies
//! in the same place in a tile and in the viewport as in the whole image. CI runs them
//! on software GPUs.

use std::sync::OnceLock;

use apertrust_engine::compute::{GpuImage, ImageKernel};
use apertrust_engine::export::{Tiling, export};
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe, Pipeline};
use apertrust_operations::exposure::Exposure;
use apertrust_operations::vignette::{self, Dithering, Parameter, Prepared, Settings, Vignette};
use apertrust_plugin::{ImageContext, ParamValues, Value};

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
        let wgsl = vignette::KERNEL.wgsl;
        let entry = |name: &str| {
            pollster::block_on(ImageKernel::with_entry_point(
                &context, "vignette", wgsl, name,
            ))
            .unwrap()
        };
        let (main, backward, forward) = (
            entry("main"),
            entry("vignette_backward"),
            entry("vignette_forward_diff"),
        );
        Gpu {
            context,
            main,
            backward,
            forward,
        }
    })
}

/// An image to vignette, `width` × `height`, as RGBA rows.
struct Image {
    width: u32,
    height: u32,
    pixels: Vec<[f32; 4]>,
}

impl Image {
    /// Colours varying across and down, with channels below 0 and above 1, and alpha
    /// varying too, so that it is seen to pass through.
    fn gradient(width: u32, height: u32) -> Self {
        let mut pixels = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let (u, v) = (x as f32 / width as f32, y as f32 / height as f32);
                pixels.push([
                    -0.05 + 1.3 * u,
                    1.2 - 1.3 * v,
                    0.3 + 0.25 * (7.0 * u + 3.0 * v).sin(),
                    0.5 + 0.5 * u,
                ]);
            }
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    fn upload(&self, gpu: &GpuContext) -> GpuImage {
        GpuImage::upload(gpu, self.width, self.height, &self.pixels).unwrap()
    }
}

/// The kernel's `main` over the whole image, at full resolution.
fn render(prepared: &Prepared, image: &Image) -> Vec<[f32; 4]> {
    let Gpu {
        context: gpu, main, ..
    } = gpu();
    let input = image.upload(gpu);
    let output = GpuImage::new(gpu, image.width, image.height);
    main.dispatch(gpu, &input, &output, &prepared.params(), &[]);
    output.download(gpu).unwrap()
}

/// darktable `process()` for the pixel at column `x` and row `y` of an image of `width`
/// × `height` pixels at full resolution (`piece->buf_in`), rendered at `scale` from its
/// top-left corner, without dithering: its expressions in f32, with f64 where darktable
/// uses it. Returns the pixel and its weight before darktable clips it to [0, 1], which is
/// within rounding of 0 where the pixel lies on the inner radius. The ratio is
/// `prepare`'s, kept off 0 and 2.
///
/// The centre's product `center.x * buf_in->width` is a float times an int, so f32; the
/// halving and the addition are in f64 but exact, so the centre is the kernel's f32
/// expression bit for bit. The minimum fall-off is f64 throughout.
fn darktable(
    p: &Prepared,
    size: [u32; 2],
    scale: f32,
    [x, y]: [u32; 2],
    rgb: [f32; 3],
) -> ([f32; 3], f32) {
    let [width, height] = size;
    let (w, h) = (width as f32, height as f32);
    let center = [
        (f64::from(w * 0.5) + f64::from(p.center[0] * w) / 2.0) as f32,
        (f64::from(h * 0.5) + f64::from(p.center[1] * h) / 2.0) as f32,
    ];
    let roi_center = [center[0] * scale, center[1] * scale];
    let (xscale, yscale) = if p.autoratio {
        (2.0 / (w * scale), 2.0 / (h * scale))
    } else {
        let basis = 2.0 / (w.max(h) * scale);
        if p.whratio <= 1.0 {
            (basis / p.whratio, basis)
        } else {
            (basis, basis / (2.0 - p.whratio))
        }
    };
    let dscale = p.scale / 100.0;
    let min_falloff = 100.0 / f64::from(width.min(height));
    let fscale = (f64::from(p.falloff_scale).max(min_falloff) / 100.0) as f32;
    let shape = p.shape.max(0.001);
    let (exp1, exp2) = (2.0 / shape, shape / 2.0);
    let scaled = [roi_center[0] * xscale, roi_center[1] * yscale];
    let pv = [
        (x as f32 * xscale - scaled[0]).abs(),
        (y as f32 * yscale - scaled[1]).abs(),
    ];
    let cplen = (pv[0].powf(exp1) + pv[1].powf(exp1)).powf(exp2);
    let ramp = (cplen - dscale) / fscale;
    let mut weight = 0.0;
    if cplen >= dscale {
        weight = ramp.clamp(0.0, 1.0);
    }
    if weight <= 0.0 {
        return (rgb, ramp);
    }
    let clip = |v: f32| if p.unbound { v } else { v.clamp(0.0, 1.0) };
    let col = if p.brightness < 0.0 {
        rgb.map(|c| clip(c * (1.0 + weight * p.brightness)))
    } else {
        rgb.map(|c| clip(c + weight * p.brightness))
    };
    let mv = (col[0] + col[1] + col[2]) / 3.0;
    let wss = weight * p.saturation;
    (col.map(|c| clip(c - (mv - c) * wss)), ramp)
}

fn prepared(settings: &Settings) -> Prepared {
    vignette::prepare(settings).unwrap()
}

/// The golden cases' settings (`crates/golden/cases/vignette*.xmp`).
fn cases() -> Vec<Settings> {
    let case = |scale, falloff_scale, brightness, saturation, center, autoratio, whratio, shape| {
        Settings {
            scale,
            falloff_scale,
            brightness,
            saturation,
            center,
            autoratio,
            whratio,
            shape,
            dithering: Dithering::Off,
            unbound: true,
        }
    };
    vec![
        case(
            69.4,
            32.08,
            -0.968,
            -0.82,
            [-0.116, 0.13],
            false,
            1.172,
            1.49,
        ),
        case(41.3, 77.7, -0.613, -0.271, [0.237, -0.168], true, 1.0, 0.83),
        case(23.7, 112.3, -0.57, 0.29, [0.071, 0.311], false, 0.63, 0.37),
        case(31.7, 83.6, -0.81, -0.37, [-0.29, -0.077], false, 1.61, 3.7),
        Settings {
            unbound: false,
            ..case(52.9, 46.3, 0.413, 0.372, [-0.043, 0.197], false, 0.87, 1.13)
        },
        case(47.3, 61.9, 0.37, -0.43, [0.153, 0.087], false, 1.37, 1.71),
    ]
}

/// The cases, darktable's defaults and its "lomo" preset, and settings across
/// darktable's ranges: both ratio branches near their ends, shapes from 0.2 to 5, the
/// minimum fall-off, no inner radius, a radius beyond the image, the centre at the
/// image's corners, each sign of the strengths, clipped and not.
fn settings_across_the_ranges() -> Vec<Settings> {
    let base = cases()[0].clone();
    let mut all = cases();
    all.push(Settings::default());
    all.push(Settings::lomo());
    for whratio in [0.05, 0.4, 1.0, 1.6, 1.95] {
        all.push(Settings {
            whratio,
            ..base.clone()
        });
    }
    for shape in [0.2, 0.6, 1.0, 2.0, 2.7, 5.0] {
        all.push(Settings {
            shape,
            ..base.clone()
        });
    }
    all.extend([
        Settings {
            falloff_scale: 0.0,
            ..base.clone()
        },
        Settings {
            scale: 0.0,
            falloff_scale: 140.0,
            ..base.clone()
        },
        Settings {
            scale: 150.0,
            ..base.clone()
        },
        Settings {
            center: [1.0, -1.0],
            autoratio: true,
            ..base.clone()
        },
        Settings {
            center: [-1.0, 1.0],
            brightness: 0.8,
            saturation: 0.6,
            ..base.clone()
        },
        Settings {
            brightness: -1.0,
            saturation: 1.0,
            unbound: false,
            ..base.clone()
        },
        Settings {
            brightness: 1.0,
            saturation: -1.0,
            unbound: false,
            autoratio: true,
            ..base
        },
    ]);
    all
}

fn close(a: f32, b: f32, tolerance: f32) -> bool {
    (a - b).abs() <= tolerance * b.abs().max(1.0)
}

/// How far the kernel may stray from the CPU model of darktable's process, relative to
/// the value or absolute below 1. On the M4 the two agree within f32 rounding; CI's
/// paravirtual Metal device rounds its transcendentals differently, 2.2e-5 at shape
/// 1.49 (`docs/ports/vignette.md`, Precision).
const TOLERANCE: f32 = 5e-5;

#[test]
fn the_kernel_follows_darktables_process() {
    let image = Image::gradient(160, 96);
    let size = [image.width, image.height];
    // The worst pixel over every setting, so a failure says how far off the GPU is.
    let mut worst = (0.0f32, String::new());
    for settings in settings_across_the_ranges() {
        let p = prepared(&settings);
        let out = render(&p, &image);
        let mut skipped = 0;
        for (i, (input, output)) in image.pixels.iter().zip(&out).enumerate() {
            let at = [i as u32 % image.width, i as u32 / image.width];
            let (want, ramp) = darktable(&p, size, 1.0, at, [input[0], input[1], input[2]]);
            // Clipped, a pixel whose weight is 0 within rounding may be clipped on one
            // side and not on the other, where its value lies outside [0, 1].
            if !p.unbound && ramp.abs() < 1e-5 {
                skipped += 1;
                continue;
            }
            for c in 0..3 {
                let error = (output[c] - want[c]).abs() / want[c].abs().max(1.0);
                if error > worst.0 {
                    worst = (
                        error,
                        format!(
                            "pixel {at:?} {input:?} gave {output:?}, darktable {want:?} (ramp {ramp}), for {settings:?}"
                        ),
                    );
                }
            }
            assert_eq!(
                output[3].to_bits(),
                input[3].to_bits(),
                "alpha passes through"
            );
        }
        assert!(
            skipped < 20,
            "{skipped} pixels on the gate for {settings:?}"
        );
    }
    assert!(
        worst.0 <= TOLERANCE,
        "{:e} off at worst: {}",
        worst.0,
        worst.1
    );
}

#[test]
fn zero_strength_unbound_is_a_no_op() {
    // Brightness and saturation 0 give every pixel back bit for bit, whatever the
    // geometry, as darktable's process() does.
    let image = Image::gradient(150, 101);
    for settings in settings_across_the_ranges() {
        let settings = Settings {
            brightness: 0.0,
            saturation: 0.0,
            unbound: true,
            ..settings
        };
        let out = render(&prepared(&settings), &image);
        for (input, output) in image.pixels.iter().zip(&out) {
            assert_eq!(
                output.map(f32::to_bits),
                input.map(f32::to_bits),
                "{input:?} gave {output:?} for {settings:?}"
            );
        }
    }
}

#[test]
fn output_stays_finite() {
    let values = [
        [0.0, 0.0, 0.0, 1.0],
        [1e6, 1e6, 1e6, 1.0],
        [-1e6, -1e6, -1e6, 1.0],
        [1e6, -1e6, 1e-3, 1.0],
        [1e-30, 1e-30, 1e-30, 1.0],
        [-0.5, 0.2, 0.1, 1.0],
    ];
    // Every value at every position of a small image.
    let (width, height) = (64, 48);
    let pixels: Vec<[f32; 4]> = (0..width * height)
        .map(|i| values[i as usize % values.len()])
        .collect();
    let image = Image {
        width,
        height,
        pixels,
    };
    let base = Settings::default();
    for shape in [0.0, 0.001, 0.02, 1.0, 5.0] {
        for whratio in [0.0, 1e-4, 2.0] {
            for (scale, falloff_scale) in [(0.0, 0.0), (200.0, 200.0), (10.0, 0.0)] {
                for center in [[-1.0, -1.0], [1.0, 1.0], [0.0, 0.0]] {
                    for (brightness, saturation, unbound) in
                        [(-1.0, -1.0, true), (1.0, 1.0, true), (1.0, -1.0, false)]
                    {
                        let settings = Settings {
                            scale,
                            falloff_scale,
                            brightness,
                            saturation,
                            center,
                            whratio,
                            shape,
                            unbound,
                            ..base.clone()
                        };
                        for (input, output) in image
                            .pixels
                            .iter()
                            .zip(render(&prepared(&settings), &image))
                        {
                            assert!(
                                output.iter().all(|c| c.is_finite()),
                                "{input:?} gave {output:?} for {settings:?}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn a_rectangle_keeps_its_inner_radius() {
    // At shape 0 darktable's (pv.x^2000 + pv.y^2000)^0.0005 underflows to 0 wherever
    // both are below about 0.95, leaving no vignette there; the kernel's length is the
    // larger of the two, a rectangle: with the automatic ratio, a pixel half way from the
    // centre to the edge lies at 0.5, inside an inner radius of 0.4's fall-off.
    let image = Image::gradient(200, 100);
    let settings = Settings {
        scale: 40.0,
        falloff_scale: 50.0,
        brightness: -1.0,
        saturation: 0.0,
        autoratio: true,
        shape: 0.0,
        ..Settings::default()
    };
    let out = render(&prepared(&settings), &image);
    let at = |x: u32, y: u32| (y * image.width + x) as usize;
    // Column 150 is at 0.5 of the half-width from the centre: weight (0.5 - 0.4) / 0.5.
    for y in [30, 50, 70] {
        let (input, output) = (image.pixels[at(150, y)], out[at(150, y)]);
        assert!(
            close(output[0], input[0] * 0.8, 1e-5),
            "{input:?} {output:?}"
        );
    }
    // Inside the rectangle, nothing changes.
    let (input, output) = (image.pixels[at(110, 55)], out[at(110, 55)]);
    assert_eq!(output, input);
}

/// The derivatives of a loss weighting the channels unequally, `Σ (0.25 r + 0.5 g + b)`
/// over the image, for the four slots' parameters: reverse mode, forward mode and
/// central finite differences of `main`. The golden check's loss, R + G + B, cannot see
/// the saturation, whose step keeps the channels' mean.
fn weighted_gradients(prepared: &Prepared, image: &Image, step: f32) -> Vec<(f64, f64, f64)> {
    const WEIGHTS: [f32; 3] = [0.25, 0.5, 1.0];
    let Gpu {
        context: gpu,
        backward,
        forward,
        ..
    } = gpu();
    let pixels = image.pixels.len();
    let input = image.upload(gpu);
    let out = GpuImage::new(gpu, image.width, image.height);
    let weights = vec![[WEIGHTS[0], WEIGHTS[1], WEIGHTS[2], 0.0]; pixels];
    let weights = GpuImage::upload(gpu, image.width, image.height, &weights).unwrap();
    let loss = |pixels: &[[f32; 4]]| -> f64 {
        pixels
            .iter()
            .map(|p| (0..3).map(|c| f64::from(WEIGHTS[c] * p[c])).sum::<f64>())
            .sum()
    };
    backward.dispatch_derivative(gpu, &input, &weights, &out, &prepared.params(), &[]);
    let reverse = out.download(gpu).unwrap();
    (0..4)
        .map(|slot| {
            let reverse: f64 = reverse.iter().map(|g| f64::from(g[slot])).sum();
            let mut direction = [0.0; 4];
            direction[slot] = 1.0;
            let directions =
                GpuImage::upload(gpu, image.width, image.height, &vec![direction; pixels]).unwrap();
            forward.dispatch_derivative(gpu, &input, &directions, &out, &prepared.params(), &[]);
            let forward = loss(&out.download(gpu).unwrap());
            let moved = |delta: f32| {
                let mut moved = prepared.clone();
                *moved.value_mut(prepared.gradient[slot]) += delta;
                loss(&render(&moved, image))
            };
            let numeric = (moved(step) - moved(-step)) / (2.0 * f64::from(step));
            (reverse, forward, numeric)
        })
        .collect()
}

#[test]
fn derivatives_match_finite_differences_for_every_parameter() {
    let image = Image::gradient(192, 128);
    let groups = [
        [
            Parameter::Saturation,
            Parameter::Brightness,
            Parameter::Scale,
            Parameter::FalloffScale,
        ],
        [
            Parameter::CenterX,
            Parameter::CenterY,
            Parameter::Whratio,
            Parameter::Shape,
        ],
    ];
    // Test 0045's settings, and a ratio below 1 with positive saturation. The second set
    // has curved sides (shape 0.9): at the narrow case's shape, 0.37, its nearly straight
    // sides lie close to this image's columns, and the centre's x came within a factor of
    // 2 of the limit (5.1e-3). A float64 emulation of this check gives these settings at
    // most 5.6e-4 for every parameter, also when any of them moves a little (the spec's
    // Gradients section).
    let curved = Settings {
        scale: 31.3,
        falloff_scale: 112.3,
        brightness: -0.57,
        saturation: 0.29,
        center: [-0.09, 0.21],
        autoratio: false,
        whratio: 0.71,
        shape: 0.9,
        dithering: Dithering::Off,
        unbound: true,
    };
    for settings in [cases()[0].clone(), curved] {
        for gradient in groups {
            let mut p = prepared(&settings);
            p.gradient = gradient;
            for (slot, (reverse, forward, numeric)) in
                weighted_gradients(&p, &image, 1e-3).into_iter().enumerate()
            {
                let parameter = gradient[slot];
                let scale = numeric.abs().max(reverse.abs());
                // Shown with --nocapture, for the spec's Gradients section.
                println!(
                    "{parameter:?}: reverse {reverse:.6e}, finite differences {numeric:.6e} \
                     ({:.1e}), forward {forward:.6e} ({:.1e})",
                    (reverse - numeric).abs() / scale,
                    (reverse - forward).abs() / scale
                );
                assert!(
                    (reverse - numeric).abs() <= 1e-2 * scale,
                    "{parameter:?}: reverse {reverse}, finite differences {numeric}, for {settings:?}"
                );
                assert!(
                    (reverse - forward).abs() <= 1e-4 * scale,
                    "{parameter:?}: reverse {reverse}, forward {forward}, for {settings:?}"
                );
            }
        }
    }
}

/// `settings` as parameter values for the operation.
fn values(settings: &Settings) -> ParamValues {
    let mut values = ParamValues::new();
    for parameter in Parameter::ALL {
        let mut p = prepared(settings);
        values.set(parameter.name(), Value::Float(*p.value_mut(parameter)));
    }
    values.set("autoratio", Value::Bool(settings.autoratio));
    values.set("unbound", Value::Bool(settings.unbound));
    values
}

#[test]
fn a_tiled_export_equals_the_whole_one() {
    // Exposure, at its identity for an image that is not a raw, ends the part a tiled
    // export runs whole; the vignette then runs in tiles that do not divide the image.
    let gpu = &gpu().context;
    let image = Image::gradient(150, 101);
    let context = ImageContext::full(image.width, image.height);
    for settings in cases() {
        let instances = [
            Instance {
                operation: &Exposure,
                params: ParamValues::new(),
            },
            Instance {
                operation: &Vignette,
                params: values(&settings),
            },
        ];
        let run = |tiling| {
            pollster::block_on(export(gpu, &instances, &context, image.upload(gpu), tiling))
                .unwrap()
        };
        let whole = run(Tiling::Whole);
        assert_eq!(whole.tiles, 1);
        // The export is the kernel's own output.
        assert_eq!(whole.pixels, render(&prepared(&settings), &image));
        for size in [[37, 33], [64, 1000], [1000, 40]] {
            let tiled = run(Tiling::Tiles(size));
            assert!(tiled.tiles > 1, "{size:?}");
            if let Some(i) = tiled
                .pixels
                .iter()
                .zip(&whole.pixels)
                .position(|(a, b)| a.map(f32::to_bits) != b.map(f32::to_bits))
            {
                panic!(
                    "tiles of {size:?}: pixel ({}, {}) is {:?} tiled, {:?} whole, for {settings:?}",
                    i as u32 % image.width,
                    i as u32 / image.width,
                    tiled.pixels[i],
                    whole.pixels[i]
                );
            }
        }
    }
}

#[test]
fn the_viewport_places_the_vignette_as_darktables_roi_scale_does() {
    // A uniform image, so that the viewport's downscale changes no value, at scales
    // 0.5 and 0.3: every pixel of the viewport is darktable's process() at that scale,
    // with the full-resolution size as darktable's buf_in.
    let gpu = &gpu().context;
    let (width, height) = (400, 300);
    let colour = [0.5, 0.25, 0.75, 1.0];
    let image = Image {
        width,
        height,
        pixels: vec![colour; (width * height) as usize],
    };
    let context = ImageContext::full(width, height);
    for (viewport, scale) in [((200, 150), 0.5), ((120, 90), 0.3)] {
        for settings in cases() {
            let instances = [Instance {
                operation: &Vignette,
                params: values(&settings),
            }];
            let plan = pipeline::plan(
                &instances,
                &context,
                image.upload(gpu).desc(),
                Pipe::Viewport {
                    width: viewport.0,
                    height: viewport.1,
                },
                &Limits::of(gpu),
            )
            .unwrap();
            let vignette = plan
                .operations
                .iter()
                .find(|op| op.id == "vignette")
                .unwrap();
            assert_eq!(vignette.scale, scale);
            let mut pipeline =
                pollster::block_on(Pipeline::new(gpu, plan, image.upload(gpu))).unwrap();
            pipeline.render(gpu).unwrap();
            let output = pipeline.output();
            assert_eq!((output.width(), output.height()), viewport);
            let pixels = output.download(gpu).unwrap();
            let p = prepared(&settings);
            for (i, output) in pixels.iter().enumerate() {
                let at = [i as u32 % viewport.0, i as u32 / viewport.0];
                let rgb = [colour[0], colour[1], colour[2]];
                let (want, ramp) = darktable(&p, [width, height], scale, at, rgb);
                if !p.unbound && ramp.abs() < 1e-5 {
                    continue;
                }
                assert!(
                    (0..3).all(|c| close(output[c], want[c], 2e-5)),
                    "scale {scale}, pixel {at:?} gave {output:?}, darktable {want:?}, for {settings:?}"
                );
            }
        }
    }
}
