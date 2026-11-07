// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2024  darktable developers (ported from src/iop/grain.c)
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

//! Invariants of the grain kernel on the GPU, needing no darktable
//! (`docs/ports/grain.md`, Test plan): it follows darktable's `process()` at full
//! resolution and, with its lattice, at the viewport's scales; a tile computes what the
//! whole image does; it stays finite; strength 0 moves lightness as little as
//! darktable's does; and its derivatives, through the noise and the lattice, are a
//! float64 model's. CI runs them on software GPUs.

use std::sync::OnceLock;

use apertrust_color::{WorkingProfile, apply, lab_to_xyz_d50, xyz_to_lab_d50};
use apertrust_engine::compute::{GpuImage, GpuTimer, ImageKernel};
use apertrust_engine::export::{Tiling, export};
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe, Pipeline};
use apertrust_operations::exposure::Exposure;
use apertrust_operations::grain::{self, Grain, Parameter, Prepared, Settings};
use apertrust_plugin::{ImageContext, ParamValues, PassHeader, Value};

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
        let wgsl = grain::KERNEL.wgsl;
        let entry = |name: &str| {
            pollster::block_on(ImageKernel::with_entry_point(&context, "grain", wgsl, name))
                .unwrap()
        };
        let (main, backward, forward) = (
            entry("main"),
            entry("grain_backward"),
            entry("grain_forward_diff"),
        );
        Gpu {
            context,
            main,
            backward,
            forward,
        }
    })
}

/// An image to put grain on, `width` × `height`, as RGBA rows.
struct Image {
    width: u32,
    height: u32,
    pixels: Vec<[f32; 4]>,
}

impl Image {
    /// Colours varying across and down, from deep shadow to above white, with a
    /// negative channel in a corner, and alpha varying too, so that it is seen to pass
    /// through.
    fn gradient(width: u32, height: u32) -> Self {
        let mut pixels = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let (u, v) = (x as f32 / width as f32, y as f32 / height as f32);
                pixels.push([
                    -0.02 + 1.2 * u * u,
                    0.002 + 1.1 * v,
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

    fn uniform(width: u32, height: u32, colour: [f32; 4]) -> Self {
        Self {
            width,
            height,
            pixels: vec![colour; (width * height) as usize],
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
    let table = grain::permutation_table();
    main.dispatch(gpu, &input, &output, &prepared.params(), &[&table]);
    output.download(gpu).unwrap()
}

fn prepared(settings: &Settings) -> Prepared {
    grain::prepare(settings, &WorkingProfile::linear_rec2020()).unwrap()
}

/// `iso` as darktable's coarseness.
fn scale(iso: f64) -> f32 {
    (iso / grain::SCALE_FACTOR) as f32
}

/// The golden cases' settings (`crates/golden/cases/grain*.xmp`), with their offsets.
fn cases() -> Vec<Settings> {
    let mire1 = grain::darktable_offset("mire1.cr2", 3908);
    let case = |scale, strength, midtones_bias, offset| Settings {
        scale,
        strength,
        midtones_bias,
        offset,
    };
    vec![
        case(f32::from_bits(0x41f0_266c), 100.0, 100.0, mire1),
        case(scale(100.0), 37.3, 52.7, mire1),
        case(scale(2300.0), 52.0, 11.5, mire1),
        case(scale(1150.0), 64.5, 0.0, mire1),
        case(
            scale(3200.0),
            71.3,
            83.0,
            grain::darktable_offset("hlrecovery.arw", 5492),
        ),
    ]
}

/// An offset as darktable's hash gives an image 3.3 million pixels wide, far more than any
/// real image's.
const LARGE_OFFSET: u32 = 1_000_000;

/// Settings across darktable's ranges: its defaults, the finest and coarsest grain, no
/// and full mid-tones bias, and a large offset.
fn settings_across_the_ranges() -> Vec<Settings> {
    let (finest, coarsest) = grain::scale_range();
    vec![
        Settings::default(),
        Settings {
            scale: finest,
            strength: 100.0,
            midtones_bias: 0.0,
            offset: 17,
        },
        Settings {
            scale: coarsest,
            strength: 8.0,
            midtones_bias: 100.0,
            offset: LARGE_OFFSET,
        },
        Settings {
            scale: scale(800.0),
            strength: 100.0,
            midtones_bias: 37.0,
            offset: 0,
        },
    ]
}

// darktable's process(), transcribed: the noise in f64 as darktable computes it, the
// paper response's table and its lookup in f32, from the position in the whole image.

const GRAD3: [[f64; 3]; 12] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [1.0, -1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0],
    [0.0, -1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
];
const AMPLITUDE: [f64; 3] = [0.2340, 0.7850, 1.2150];

/// darktable `perm[i]`.
fn perm(i: i64) -> i64 {
    i64::from(grain::PERMUTATION[(i & 255) as usize])
}

/// darktable `FASTFLOOR`, one below a whole number that is not positive.
fn fastfloor(x: f64) -> i64 {
    if x > 0.0 { x as i64 } else { x as i64 - 1 }
}

/// darktable `_simplex_noise`, in f64.
fn simplex(xin: f64, yin: f64, zin: f64) -> f64 {
    const F3: f64 = 1.0 / 3.0;
    const G3: f64 = 1.0 / 6.0;
    let s = (xin + yin + zin) * F3;
    let (i, j, k) = (fastfloor(xin + s), fastfloor(yin + s), fastfloor(zin + s));
    let t = (i + j + k) as f64 * G3;
    let (x0, y0, z0) = (
        xin - (i as f64 - t),
        yin - (j as f64 - t),
        zin - (k as f64 - t),
    );
    let (second, third) = if x0 >= y0 {
        if y0 >= z0 {
            ([1, 0, 0], [1, 1, 0])
        } else if x0 >= z0 {
            ([1, 0, 0], [1, 0, 1])
        } else {
            ([0, 0, 1], [1, 0, 1])
        }
    } else if y0 < z0 {
        ([0, 0, 1], [0, 1, 1])
    } else if x0 < z0 {
        ([0, 1, 0], [0, 1, 1])
    } else {
        ([0, 1, 0], [1, 1, 0])
    };
    let (ii, jj, kk) = (i & 255, j & 255, k & 255);
    let corner = |[di, dj, dk]: [i64; 3], offset: f64| {
        let (x, y, z) = (
            x0 - di as f64 + offset,
            y0 - dj as f64 + offset,
            z0 - dk as f64 + offset,
        );
        let g = GRAD3[(perm(ii + di + perm(jj + dj + perm(kk + dk))) % 12) as usize];
        let t = 0.6 - x * x - y * y - z * z;
        if t < 0.0 {
            0.0
        } else {
            let t = t * t;
            t * t * (g[0] * x + g[1] * y + g[2] * z)
        }
    };
    32.0 * (corner([0, 0, 0], 0.0)
        + corner(second, G3)
        + corner(third, 2.0 * G3)
        + corner([1, 1, 1], 3.0 * G3))
}

/// darktable `_simplex_2d_noise`.
fn simplex_2d(x: f64, y: f64, zoom: f64) -> f64 {
    let mut total = 0.0;
    for (octave, (f, a)) in grain::FREQUENCY.iter().zip(AMPLITUDE).enumerate() {
        total += simplex(x * f / zoom, y * f / zoom, octave as f64) * a;
    }
    total
}

/// darktable's delta, `GRAIN_LUT_DELTA_MAX * expf(mb / 100 * logf(GRAIN_LUT_DELTA_MIN))`.
fn delta_f32(mb: f32) -> f32 {
    (2.0 * f64::from((mb / 100.0 * 0.0001f32.ln()).exp())) as f32
}

/// darktable `paper_resp` and `paper_resp_inverse`, with a paper gamma of 1, in f32.
fn paper_resp(exposure: f32, mb: f32) -> f32 {
    let delta = delta_f32(mb);
    let span = 1.0 + 2.0 * delta;
    span / (1.0 + ((4.0 * (0.5 - exposure)) / span).exp()) - delta
}

fn paper_resp_inverse(density: f32, mb: f32) -> f32 {
    let delta = delta_f32(mb);
    let span = 1.0 + 2.0 * delta;
    -(span / (density + delta) - 1.0).ln() * span / 4.0 + 0.5
}

/// darktable `evaluate_grain_lut`: 128 × 128, row j the lightness, column i the grain.
fn grain_lut(mb: f32) -> Vec<f32> {
    let mut lut = vec![0.0; 128 * 128];
    for j in 0..128 {
        for i in 0..128 {
            let gu = (f64::from(i as f32 / 127.0) - 0.5) as f32;
            let l = j as f32 / 127.0;
            lut[j * 128 + i] = 100.0 * (paper_resp(gu + paper_resp_inverse(l, mb), mb) - l);
        }
    }
    lut
}

/// darktable `dt_lut_lookup_2d_1c`.
fn lut_lookup(lut: &[f32], x: f32, y: f32) -> f32 {
    let x = ((x + 0.5) * 127.0).clamp(0.0, 127.0);
    let y = (y * 127.0).clamp(0.0, 127.0);
    let x0 = x.min(126.0) as usize;
    let y0 = y.min(126.0) as usize;
    let (x_diff, y_diff) = (x - x0 as f32, y - y0 as f32);
    let l00 = lut[y0 * 128 + x0];
    let l01 = lut[y0 * 128 + x0 + 1];
    let l10 = lut[(y0 + 1) * 128 + x0];
    let l11 = lut[(y0 + 1) * 128 + x0 + 1];
    let xy0 = (1.0 - y_diff) * l00 + l10 * y_diff;
    let xy1 = (1.0 - y_diff) * l01 + l11 * y_diff;
    xy0 * (1.0 - x_diff) + xy1 * x_diff
}

/// darktable `process()`'s noise for the pixel at column and row `at` of the image at
/// `scale`, of `full` pixels at full resolution (`piece->buf_in`), from its top-left
/// corner, with `offset` for darktable's hash and an iscale of 1; with its lattice when
/// the scale is further than 0.01 from 1.
fn darktable_noise(settings: &Settings, full: [u32; 2], scale: f32, at: [u32; 2]) -> f32 {
    let wd = f64::from(full[0].min(full[1]) as f32);
    let zoom = grain::zoom(settings.scale);
    let filtermul = 1.0 / (f64::from(scale) * wd);
    let roi_scale = f64::from(scale);
    let x = f64::from(at[0]) / roi_scale / wd;
    let y = f64::from(at[1]) / roi_scale / wd;
    let hash = f64::from(settings.offset);
    if (scale - 1.0).abs() > 0.01 {
        let (fib1, fib2) = (34.0f32, 21.0f32);
        let fib1div2 = fib1 / fib2;
        let mut noise = 0.0f32;
        for l in 0..21 {
            let px = l as f32 / fib2;
            let mut py = l as f32 * fib1div2;
            py -= py as i32 as f32;
            let dx = (f64::from(px) * filtermul) as f32;
            let dy = (f64::from(py) * filtermul) as f32;
            noise = (f64::from(noise)
                + 1.0 / f64::from(fib2)
                    * simplex_2d(x + f64::from(dx) + hash, y + f64::from(dy), zoom))
                as f32;
        }
        noise
    } else {
        simplex_2d(x + hash, y, zoom) as f32
    }
}

/// darktable's working RGB through Lab, `process()` and back, for one pixel.
fn darktable(
    settings: &Settings,
    lut: &[f32],
    full: [u32; 2],
    scale: f32,
    at: [u32; 2],
    rgb: [f32; 3],
) -> [f32; 3] {
    let profile = WorkingProfile::linear_rec2020();
    let lab = xyz_to_lab_d50(apply(&profile.rgb_to_xyz_d50, rgb));
    let noise = darktable_noise(settings, full, scale, at);
    let strength = settings.strength / 100.0;
    let l = lab[0] + lut_lookup(lut, (noise * strength) * 0.15, lab[0] / 100.0);
    apply(&profile.xyz_d50_to_rgb, lab_to_xyz_d50([l, lab[1], lab[2]]))
}

/// Lab lightness of working RGB.
fn lightness(rgb: [f32; 3]) -> f32 {
    xyz_to_lab_d50(apply(&WorkingProfile::linear_rec2020().rgb_to_xyz_d50, rgb))[0]
}

/// How far the kernel's lightness lies from darktable's, at most and on average, and
/// the largest difference of any channel in RGB beyond what the lightness explains.
struct Gap {
    max: f32,
    mean: f64,
}

fn gap(ours: &[[f32; 4]], theirs: &[[f32; 3]]) -> Gap {
    let mut max = 0.0f32;
    let mut sum = 0.0f64;
    for (o, t) in ours.iter().zip(theirs) {
        let d = (lightness([o[0], o[1], o[2]]) - lightness(*t)).abs();
        max = max.max(d);
        sum += f64::from(d);
    }
    Gap {
        max,
        mean: sum / ours.len() as f64,
    }
}

/// The largest gap in Lab lightness from darktable this port allows, and on average:
/// the noise in f32 lies within about 0.0064 of darktable's f64 over a whole 10 MP
/// image (`docs/ports/grain.md`, Precision), at most 0.1 L at full strength.
const MAX_LIGHTNESS_GAP: f32 = 0.15;
const MEAN_LIGHTNESS_GAP: f64 = 0.005;

#[test]
fn the_kernel_follows_darktables_process() {
    let image = Image::gradient(160, 107);
    let full = [image.width, image.height];
    for settings in cases().into_iter().chain(settings_across_the_ranges()) {
        let lut = grain_lut(settings.midtones_bias);
        let ours = render(&prepared(&settings), &image);
        let theirs: Vec<[f32; 3]> = image
            .pixels
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let at = [i as u32 % image.width, i as u32 / image.width];
                darktable(&settings, &lut, full, 1.0, at, [p[0], p[1], p[2]])
            })
            .collect();
        let Gap { max, mean } = gap(&ours, &theirs);
        println!("{settings:?}: lightness from darktable's, max {max:.2e}, mean {mean:.2e}");
        assert!(
            max <= MAX_LIGHTNESS_GAP && mean <= MEAN_LIGHTNESS_GAP,
            "max {max}, mean {mean}, for {settings:?}"
        );
        // Alpha passes through, and the grain moves lightness alone.
        for (o, p) in ours.iter().zip(&image.pixels) {
            assert_eq!(o[3], p[3]);
            let profile = WorkingProfile::linear_rec2020();
            let before = xyz_to_lab_d50(apply(&profile.rgb_to_xyz_d50, [p[0], p[1], p[2]]));
            let after = xyz_to_lab_d50(apply(&profile.rgb_to_xyz_d50, [o[0], o[1], o[2]]));
            assert!(
                (after[1] - before[1]).abs() < 2e-3 && (after[2] - before[2]).abs() < 2e-3,
                "{before:?} became {after:?}, for {settings:?}"
            );
        }
    }
}

#[test]
fn darktables_hash_starts_the_noise_where_darktable_does() {
    // An offset is a whole number of shorter sides: the grain far from the image's left
    // edge, in f64, reduced by the noise's period on the CPU.
    let image = Image::gradient(64, 48);
    let base = cases()[0].clone();
    let lut = grain_lut(base.midtones_bias);
    for offset in [1, 758, 4321, LARGE_OFFSET] {
        let settings = Settings {
            offset,
            ..base.clone()
        };
        let ours = render(&prepared(&settings), &image);
        let theirs: Vec<[f32; 3]> = image
            .pixels
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let at = [i as u32 % image.width, i as u32 / image.width];
                darktable(&settings, &lut, [64, 48], 1.0, at, [p[0], p[1], p[2]])
            })
            .collect();
        let Gap { max, mean } = gap(&ours, &theirs);
        assert!(
            max <= MAX_LIGHTNESS_GAP && mean <= MEAN_LIGHTNESS_GAP,
            "offset {offset}: max {max}, mean {mean}"
        );
    }
}

#[test]
fn strength_zero_moves_lightness_as_little_as_darktables() {
    // Not a no-op: at u = 0 darktable's lookup falls between the table's columns 63 and
    // 64, whose mean is not 0, up to 1.2e-3 L (the spec's Tool binding notes).
    let image = Image::gradient(96, 64);
    for settings in [cases()[0].clone(), cases()[2].clone()] {
        let settings = Settings {
            strength: 0.0,
            ..settings
        };
        let ours = render(&prepared(&settings), &image);
        for (o, p) in ours.iter().zip(&image.pixels) {
            let moved = (lightness([o[0], o[1], o[2]]) - lightness([p[0], p[1], p[2]])).abs();
            assert!(moved < 2e-3, "{p:?} moved {moved} L, for {settings:?}");
        }
    }
}

#[test]
fn output_stays_finite() {
    let values = [0.0, 1e-30, 1e-6, 0.5, 1.0, 4.0, 1e6, -1e-3, -0.5, -1e6];
    let mut pixels = Vec::new();
    for &r in &values {
        for &g in &values {
            for &b in &values {
                pixels.push([r, g, b, 1.0]);
            }
        }
    }
    let image = Image {
        width: 100,
        height: 10,
        pixels,
    };
    for settings in settings_across_the_ranges() {
        for strength in [0.0, 100.0] {
            let settings = Settings {
                strength,
                ..settings.clone()
            };
            let out = render(&prepared(&settings), &image);
            for (o, p) in out.iter().zip(&image.pixels) {
                assert!(
                    o.iter().all(|v| v.is_finite()),
                    "{p:?} gave {o:?}, for {settings:?}"
                );
            }
        }
    }
}

/// `settings` as parameter values for the operation, which renders at the provisional
/// offset, not theirs.
fn values(settings: &Settings) -> ParamValues {
    let mut values = ParamValues::new();
    values.set("scale", Value::Float(settings.scale));
    values.set("strength", Value::Float(settings.strength));
    values.set("midtones_bias", Value::Float(settings.midtones_bias));
    values
}

/// The cases' settings at the offset the operation renders at.
fn cases_at_the_provisional_offset() -> Vec<Settings> {
    cases()
        .into_iter()
        .map(|settings| Settings {
            offset: grain::PROVISIONAL_OFFSET,
            ..settings
        })
        .collect()
}

#[test]
fn a_tiled_export_equals_the_whole_one() {
    // Exposure, at its identity for an image that is not a raw, ends the part a tiled
    // export runs whole; the grain then runs in tiles that do not divide the image.
    let gpu = &gpu().context;
    let image = Image::gradient(150, 101);
    let context = ImageContext::full(image.width, image.height);
    for settings in cases_at_the_provisional_offset() {
        let instances = [
            Instance {
                operation: &Exposure,
                params: ParamValues::new(),
            },
            Instance {
                operation: &Grain,
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
fn the_viewport_follows_darktables_process_at_its_scale() {
    // A uniform image, so that the viewport's downscale changes no value, at scales 0.5
    // and 0.3: every pixel of the viewport is darktable's process() at that scale, its
    // 21-position lattice included, with the full-resolution size as darktable's buf_in,
    // at the operation's offset (darktable's previews hash with their own widths).
    let gpu = &gpu().context;
    let (width, height) = (400, 300);
    let colour = [0.4, 0.25, 0.6, 1.0];
    let image = Image::uniform(width, height, colour);
    let context = ImageContext::full(width, height);
    for (viewport, scale) in [((200, 150), 0.5), ((120, 90), 0.3)] {
        for settings in cases_at_the_provisional_offset() {
            let instances = [Instance {
                operation: &Grain,
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
            let op = plan.operations.iter().find(|op| op.id == "grain").unwrap();
            assert_eq!(op.scale, scale);
            let mut pipeline =
                pollster::block_on(Pipeline::new(gpu, plan, image.upload(gpu))).unwrap();
            pipeline.render(gpu).unwrap();
            let output = pipeline.output();
            assert_eq!((output.width(), output.height()), viewport);
            let ours = output.download(gpu).unwrap();
            let lut = grain_lut(settings.midtones_bias);
            let theirs: Vec<[f32; 3]> = (0..ours.len())
                .map(|i| {
                    let at = [i as u32 % viewport.0, i as u32 / viewport.0];
                    let rgb = [colour[0], colour[1], colour[2]];
                    darktable(&settings, &lut, [width, height], scale, at, rgb)
                })
                .collect();
            let Gap { max, mean } = gap(&ours, &theirs);
            println!("scale {scale}, {settings:?}: max {max:.2e}, mean {mean:.2e}");
            assert!(
                max <= MAX_LIGHTNESS_GAP && mean <= MEAN_LIGHTNESS_GAP,
                "scale {scale}: max {max}, mean {mean}, for {settings:?}"
            );
        }
    }
}

/// A `width` × `height` header for the whole image at `scale` of one `full` at full
/// resolution.
fn header_at(width: u32, height: u32, full: [u32; 2], scale: f32) -> PassHeader {
    PassHeader {
        full_width: full[0],
        full_height: full[1],
        scale,
        ..PassHeader::whole(width, height)
    }
}

#[test]
fn the_lattice_follows_darktables_process_at_darktables_offsets() {
    // The kernel at scales 0.5 and 0.3 on a gradient image, the whole image at that scale,
    // with each case's offset as darktable's export gives it.
    let Gpu {
        context: gpu, main, ..
    } = gpu();
    let image = Image::gradient(96, 64);
    let input = image.upload(gpu);
    let output = GpuImage::new(gpu, image.width, image.height);
    let table = grain::permutation_table();
    for (full, scale) in [([192, 128], 0.5f32), ([320, 213], 0.3)] {
        let header = header_at(image.width, image.height, full, scale);
        for settings in cases() {
            let p = prepared(&settings);
            main.dispatch_with(gpu, &header, &input, &output, &p.params(), &[&table], None);
            let ours = output.download(gpu).unwrap();
            let lut = grain_lut(settings.midtones_bias);
            let theirs: Vec<[f32; 3]> = image
                .pixels
                .iter()
                .enumerate()
                .map(|(i, px)| {
                    let at = [i as u32 % image.width, i as u32 / image.width];
                    darktable(&settings, &lut, full, scale, at, [px[0], px[1], px[2]])
                })
                .collect();
            let Gap { max, mean } = gap(&ours, &theirs);
            println!("scale {scale}, {settings:?}: max {max:.2e}, mean {mean:.2e}");
            assert!(
                max <= MAX_LIGHTNESS_GAP && mean <= MEAN_LIGHTNESS_GAP,
                "scale {scale}: max {max}, mean {mean}, for {settings:?}"
            );
        }
    }
}

// A float64 model of the kernel's own function, for its derivatives: the noise with the
// offsets the setup gives held, as the kernel differentiates it, and the paper
// response's table entries computed where they are read.

/// The kernel's zoom, in f64: `(1 + 0.08 scale) / 800`.
fn model_zoom(scale: f64) -> f64 {
    (1.0 + 0.08 * scale) / 800.0
}

/// The kernel's noise at normalised position `at`, averaged over the lattice of
/// `spacing` when there is one.
fn model_noise(scale: f64, offset: [f32; 3], at: [f64; 2], spacing: Option<f64>) -> f64 {
    let octaves = |at: [f64; 2]| {
        let zoom = model_zoom(scale);
        (0..3)
            .map(|o| {
                let c = grain::FREQUENCY[o] / zoom;
                simplex(at[0] * c + f64::from(offset[o]), at[1] * c, o as f64) * AMPLITUDE[o]
            })
            .sum::<f64>()
    };
    match spacing {
        None => octaves(at),
        Some(spacing) => {
            (0..21)
                .map(|l| {
                    let y = f64::from(l as f32 * (34.0f32 / 21.0));
                    let tap = [f64::from(l) / 21.0, y - y.floor()];
                    octaves([at[0] + tap[0] * spacing, at[1] + tap[1] * spacing])
                })
                .sum::<f64>()
                / 21.0
        }
    }
}

/// The table's entry at column `i` and row `j`, darktable's formula in f64.
fn model_entry(i: f64, j: f64, midtones_bias: f64) -> f64 {
    let delta = 2.0 * (midtones_bias / 100.0 * 0.0001f64.ln()).exp();
    let span = 1.0 + 2.0 * delta;
    let paper = |e: f64| span / (1.0 + (4.0 * (0.5 - e) / span).exp()) - delta;
    let inverse = |d: f64| -(span / (d + delta) - 1.0).ln() * span / 4.0 + 0.5;
    let (gu, l) = (i / 127.0 - 0.5, j / 127.0);
    100.0 * (paper(gu + inverse(l)) - l)
}

fn lab64(rgb: [f64; 3]) -> [f64; 3] {
    let m = WorkingProfile::linear_rec2020().rgb_to_xyz_d50;
    let d50 = [0.9642, 1.0, 0.8249];
    let f = |c: usize| {
        let x = (0..3).map(|k| f64::from(m[c][k]) * rgb[k]).sum::<f64>() / d50[c];
        if x > 216.0 / 24389.0 {
            x.cbrt()
        } else {
            (24389.0 / 27.0 * x + 16.0) / 116.0
        }
    };
    let (fx, fy, fz) = (f(0), f(1), f(2));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), -200.0 * (fz - fy)]
}

fn rgb64(lab: [f64; 3]) -> [f64; 3] {
    let m = WorkingProfile::linear_rec2020().xyz_d50_to_rgb;
    let d50 = [0.9642, 1.0, 0.8249];
    let inv = |x: f64| {
        if x > 6.0 / 29.0 {
            x * x * x
        } else {
            (116.0 * x - 16.0) / (24389.0 / 27.0)
        }
    };
    let fy = (lab[0] + 16.0) / 116.0;
    let xyz = [
        d50[0] * inv(lab[1] / 500.0 + fy),
        d50[1] * inv(fy),
        d50[2] * inv(fy - lab[2] / 200.0),
    ];
    std::array::from_fn(|c| (0..3).map(|k| f64::from(m[c][k]) * xyz[k]).sum())
}

/// The kernel's function in f64 for parameters [strength, scale, midtones_bias].
fn model(
    p: [f64; 3],
    offset: [f32; 3],
    at: [f64; 2],
    spacing: Option<f64>,
    rgb: [f32; 3],
) -> [f64; 3] {
    let lab = lab64(rgb.map(f64::from));
    let n = model_noise(p[1], offset, at, spacing);
    let u = n * (p[0] / 100.0) * 0.15;
    let x = ((u + 0.5) * 127.0).clamp(0.0, 127.0);
    let y = (lab[0] / 100.0 * 127.0).clamp(0.0, 127.0);
    let (x0, y0) = (x.min(126.0).floor(), y.min(126.0).floor());
    let (xd, yd) = (x - x0, y - y0);
    let e = |i: f64, j: f64| model_entry(i, j, p[2]);
    let xy0 = (1.0 - yd) * e(x0, y0) + e(x0, y0 + 1.0) * yd;
    let xy1 = (1.0 - yd) * e(x0 + 1.0, y0) + e(x0 + 1.0, y0 + 1.0) * yd;
    rgb64([lab[0] + xy0 * (1.0 - xd) + xy1 * xd, lab[1], lab[2]])
}

/// The model's derivative with respect to parameter `which`, by central differences in
/// f64 at steps small enough that the noise's simplices rarely change under them.
fn model_derivative(
    p: [f64; 3],
    which: usize,
    offset: [f32; 3],
    at: [f64; 2],
    spacing: Option<f64>,
    rgb: [f32; 3],
) -> [f64; 3] {
    let h = [1e-5, 1e-7, 1e-5][which];
    let (mut plus, mut minus) = (p, p);
    plus[which] += h;
    minus[which] -= h;
    let (a, b) = (
        model(plus, offset, at, spacing, rgb),
        model(minus, offset, at, spacing, rgb),
    );
    std::array::from_fn(|c| (a[c] - b[c]) / (2.0 * h))
}

/// Per pixel and channel, how many of the kernel's forward-mode derivatives lie within
/// 1% of the model's (or within 1% of the image's mean magnitude where the model's is
/// small), as a fraction.
fn agreement(ours: &[[f32; 4]], theirs: &[[f64; 3]]) -> f64 {
    let mean = theirs.iter().flatten().map(|v| v.abs()).sum::<f64>() / (3 * theirs.len()) as f64;
    let mut agree = 0usize;
    for (o, t) in ours.iter().zip(theirs) {
        for c in 0..3 {
            let scale = t[c].abs().max(mean);
            if (f64::from(o[c]) - t[c]).abs() <= 1e-2 * scale {
                agree += 1;
            }
        }
    }
    agree as f64 / (3 * theirs.len()) as f64
}

/// Settings for the derivative checks: test 0059, the finest case's grain, and a
/// moderate one with little mid-tones bias, where the table's slope is largest.
fn derivative_settings() -> Vec<Settings> {
    vec![
        cases()[0].clone(),
        cases()[1].clone(),
        Settings {
            scale: scale(1600.0),
            strength: 61.0,
            midtones_bias: 23.0,
            offset: 758,
        },
    ]
}

/// The fraction of pixels and channels whose derivatives the check allows to differ
/// from the model's: those whose noise crosses into another simplex within the model's
/// step, or which the kernel's f32 places in another simplex or table cell. On the Apple
/// M4 at most 0.37% do, the lattice's at 100 ISO, whose 63 noise evaluations per pixel
/// each may; a wrong derivative would fail nearly every pixel.
const DISAGREEING: f64 = 1e-2;

#[test]
fn derivatives_follow_the_float64_model_per_pixel() {
    let Gpu {
        context: gpu,
        backward,
        forward,
        ..
    } = gpu();
    let image = Image::gradient(96, 64);
    let pixels = image.pixels.len();
    let input = image.upload(gpu);
    let out = GpuImage::new(gpu, image.width, image.height);
    let table = grain::permutation_table();
    let shorter = f64::from(image.width.min(image.height) as f32);
    for settings in derivative_settings() {
        let p = prepared(&settings);
        let params = p.params();
        let ones =
            GpuImage::upload(gpu, image.width, image.height, &vec![[1.0; 4]; pixels]).unwrap();
        backward.dispatch_derivative(gpu, &input, &ones, &out, &params, &[&table]);
        let reverse = out.download(gpu).unwrap();
        for (slot, parameter) in p.gradient.into_iter().enumerate() {
            let mut direction = [0.0; 4];
            direction[slot] = 1.0;
            let directions =
                GpuImage::upload(gpu, image.width, image.height, &vec![direction; pixels]).unwrap();
            forward.dispatch_derivative(gpu, &input, &directions, &out, &params, &[&table]);
            let ours = out.download(gpu).unwrap();
            let which = parameter.index() as usize;
            let values = [p.strength, p.scale, p.midtones_bias].map(f64::from);
            let theirs: Vec<[f64; 3]> = image
                .pixels
                .iter()
                .enumerate()
                .map(|(i, px)| {
                    let at = [
                        f64::from((i as u32 % image.width) as f32) / shorter,
                        f64::from((i as u32 / image.width) as f32) / shorter,
                    ];
                    model_derivative(values, which, p.offset, at, None, [px[0], px[1], px[2]])
                })
                .collect();
            let agree = agreement(&ours, &theirs);
            // Forward against reverse mode per pixel, as the golden check measures it.
            let apart = ours
                .iter()
                .zip(&reverse)
                .map(|(f, r)| f64::from((f[0] + f[1] + f[2] - r[slot]).abs()))
                .sum::<f64>()
                / reverse
                    .iter()
                    .map(|r| f64::from(r[slot].abs()))
                    .sum::<f64>();
            println!(
                "{parameter:?}: {:.2}% of pixels within 1% of the model, forward and reverse \
                 {apart:.1e} apart, for {settings:?}",
                100.0 * agree
            );
            assert!(
                agree >= 1.0 - DISAGREEING,
                "{parameter:?}: {agree}, for {settings:?}"
            );
            assert!(apart <= 1e-4, "{parameter:?}: {apart}, for {settings:?}");
        }
    }
}

#[test]
fn the_lattices_derivatives_follow_the_float64_model() {
    // The viewport at scale 0.5 of a 192 × 128 image: 96 × 64 pixels, each averaging the
    // noise at 21 positions across it.
    let Gpu {
        context: gpu,
        forward,
        ..
    } = gpu();
    let image = Image::gradient(96, 64);
    let pixels = image.pixels.len();
    let input = image.upload(gpu);
    let out = GpuImage::new(gpu, image.width, image.height);
    let table = grain::permutation_table();
    let (full, scale) = ([192, 128], 0.5f32);
    let header = header_at(image.width, image.height, full, scale);
    let shorter = f64::from(128.0f32);
    let spacing = 1.0 / (f64::from(scale) * shorter);
    for settings in derivative_settings() {
        let p = prepared(&settings);
        for (slot, parameter) in p.gradient.into_iter().enumerate() {
            let mut direction = [0.0; 4];
            direction[slot] = 1.0;
            let directions =
                GpuImage::upload(gpu, image.width, image.height, &vec![direction; pixels]).unwrap();
            forward.dispatch_derivative_with(
                gpu,
                &header,
                &input,
                &directions,
                &out,
                &p.params(),
                &[&table],
            );
            let ours = out.download(gpu).unwrap();
            let which = parameter.index() as usize;
            let values = [p.strength, p.scale, p.midtones_bias].map(f64::from);
            let theirs: Vec<[f64; 3]> = image
                .pixels
                .iter()
                .enumerate()
                .map(|(i, px)| {
                    let at = [
                        f64::from((i as u32 % image.width) as f32) / f64::from(scale) / shorter,
                        f64::from((i as u32 / image.width) as f32) / f64::from(scale) / shorter,
                    ];
                    model_derivative(
                        values,
                        which,
                        p.offset,
                        at,
                        Some(spacing),
                        [px[0], px[1], px[2]],
                    )
                })
                .collect();
            let agree = agreement(&ours, &theirs);
            println!(
                "{parameter:?} at scale {scale}: {:.2}% of pixels within 1% of the model, for {settings:?}",
                100.0 * agree
            );
            assert!(
                agree >= 1.0 - DISAGREEING,
                "{parameter:?}: {agree}, for {settings:?}"
            );
        }
    }
}

#[test]
#[ignore = "a timing for the spec, not a check: run with --ignored --nocapture"]
fn the_lattices_cost() {
    // About 10 MP of output, 3908 × 2602, as the viewport of an image twice its size at
    // scale 0.5 (21 positions per pixel) and as that image at full resolution (one).
    let Gpu {
        context: gpu, main, ..
    } = gpu();
    let (width, height) = (3908, 2602);
    let image = Image::uniform(width, height, [0.4, 0.25, 0.6, 1.0]);
    let input = image.upload(gpu);
    let output = GpuImage::new(gpu, width, height);
    let table = grain::permutation_table();
    let params = prepared(&cases()[0]).params();
    let rounds = 20;
    let timer = GpuTimer::new(gpu, rounds).expect("GPU timestamps");
    for (label, header) in [
        (
            "one position, scale 1",
            header_at(width, height, [width, height], 1.0),
        ),
        (
            "21 positions, scale 0.5",
            header_at(width, height, [2 * width, 2 * height], 0.5),
        ),
    ] {
        for _ in 0..rounds {
            main.dispatch_with(gpu, &header, &input, &output, &params, &[&table], None);
        }
        for slot in 0..rounds {
            main.dispatch_with(
                gpu,
                &header,
                &input,
                &output,
                &params,
                &[&table],
                Some((&timer, slot)),
            );
        }
        let mut times = timer.read(gpu, rounds).unwrap();
        times.sort_by(f64::total_cmp);
        println!(
            "{label}: median {:.2} ms, min {:.2} ms per pass over {width} × {height}",
            times[times.len() / 2],
            times[0]
        );
    }
}

#[test]
fn the_scales_derivative_holds_the_offset() {
    // Moving the scale's slot leaves the offsets the setup computed: the gradient is the
    // grain's at a fixed offset, scaling about the image's top-left corner.
    let settings = cases()[0].clone();
    let p = prepared(&settings);
    let mut moved = p.clone();
    *moved.value_mut(Parameter::Scale) += 1.0;
    assert_eq!(moved.offset, p.offset);
    assert_ne!(grain::noise_offsets(settings.offset, moved.scale), p.offset);
}
