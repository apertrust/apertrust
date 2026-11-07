// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/iop/channelmixerrgb.c)
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

//! The color calibration kernel on the GPU against darktable's per-pixel code restated,
//! and its invariants, needing no darktable (`docs/ports/channelmixerrgb.md`, Test
//! plan). CI runs them on software GPUs.

#![allow(
    clippy::excessive_precision,
    reason = "constants are copied digit for digit from darktable"
)]

use std::sync::OnceLock;

use apertrust_color::adaptation::{BRADFORD_D50, CAT16_D50, XYZ_D50};
use apertrust_color::illuminants::{D50_XY, illuminant_xy_to_xyz};
use apertrust_color::{Adaptation, Illuminant, WorkingProfile, apply};
use apertrust_engine::compute::{GpuImage, ImageKernel};
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_operations::channelmixerrgb::{self, NegativeDistance, Prepared, Settings};

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
            "channelmixerrgb",
            channelmixerrgb::KERNEL.wgsl,
        ))
        .unwrap();
        Gpu { context, kernel }
    })
}

/// The kernel on one row of pixels.
fn render_prepared(prepared: &Prepared, pixels: &[[f32; 3]]) -> Vec<[f32; 3]> {
    let Gpu {
        context: gpu,
        kernel,
    } = gpu();
    let rgba: Vec<[f32; 4]> = pixels.iter().map(|&[r, g, b]| [r, g, b, 1.0]).collect();
    let width = u32::try_from(rgba.len()).unwrap();
    let input = GpuImage::upload(gpu, width, 1, &rgba).unwrap();
    let output = GpuImage::new(gpu, width, 1);
    kernel.dispatch(gpu, &input, &output, &prepared.params(), &[]);
    output
        .download(gpu)
        .unwrap()
        .into_iter()
        .map(|[r, g, b, _]| [r, g, b])
        .collect()
}

fn prepare(settings: &Settings) -> Prepared {
    channelmixerrgb::prepare(settings, &WorkingProfile::linear_rec2020(), None).unwrap()
}

/// Test 0085's settings: a custom illuminant, normalised mixer rows, colourfulness and
/// brightness.
const TEST_0085: Settings = Settings {
    red: [-0.969, 0.476, 0.0],
    green: [0.0, -0.479, 0.0],
    blue: [0.0, 0.0, 1.0],
    saturation: [0.215, -0.953, -0.544],
    lightness: [0.184, -0.305, 0.138],
    normalize_r: true,
    normalize_g: true,
    normalize_b: true,
    illuminant: Illuminant::Custom,
    x: 0.3098125,
    y: 0.3276207,
    ..Settings::DEFAULT
};

/// darktable's "monochrome | ILFORD HP5+" preset.
const HP5: Settings = Settings {
    grey: [0.25304098, 0.25958747, 0.48737156],
    normalize_grey: true,
    illuminant: Illuminant::Pipe,
    adaptation: Adaptation::Xyz,
    gamut: 0.0,
    ..Settings::DEFAULT
};

/// darktable's no-op starting point for its presets (`init_presets`).
const NO_OP: Settings = Settings {
    illuminant: Illuminant::Pipe,
    adaptation: Adaptation::Xyz,
    gamut: 0.0,
    clip: false,
    x: D50_XY[0],
    y: D50_XY[1],
    ..Settings::DEFAULT
};

/// The synthetic sweep, every hue from neutral to beyond the working gamut at levels
/// from deep shadow to well above white, and black.
fn sweep() -> Vec<[f32; 3]> {
    let mut pixels = golden::synthetic::sweep().pixels;
    pixels.push([0.0; 3]);
    pixels
}

const NORM_MIN: f32 = 1.52587890625e-05;
const INVERSE_SQRT_3: f32 = 0.5773502691896258;

/// The kernel's guards (`slang/channelmixerrgb.slang`), restated.
const INPUT_LIMIT: f32 = 1e8;
const XYZ_LIMIT: f32 = 1e12;
const RATIO_LIMIT: f32 = 1e8;
const SUM_FLOOR: f32 = 1e-6;
const LOG2_POWER_LIMIT: f32 = 20.0;
const BRIGHTNESS_LIMIT: f32 = 1e6;

/// What a restatement of the per-pixel code gives: its output, whether a guard of the
/// kernel's changed a value, and whether every value it computed was finite.
#[derive(Debug, Clone, Copy)]
struct Restated {
    out: [f32; 3],
    fired: bool,
    finite: bool,
}

/// Follows a restatement: with `guards`, the kernel's guards apply, and `fired` records
/// whether one changed a value; `finite` whether every value checked was finite.
struct Trace {
    guards: bool,
    fired: bool,
    finite: bool,
}

impl Trace {
    fn check<const N: usize>(&mut self, v: [f32; N]) -> [f32; N] {
        self.finite &= v.iter().all(|c| c.is_finite());
        v
    }

    fn value(&mut self, v: f32) -> f32 {
        self.check([v])[0]
    }

    /// darktable's `original`, or the kernel's `guarded` with guards.
    fn guard(&mut self, original: f32, guarded: f32) -> f32 {
        if !self.guards {
            return original;
        }
        if guarded != original {
            self.fired = true;
        }
        guarded
    }
}

/// `v` held at least `floor` from 0, keeping its sign, as `away_from_zero` in
/// `color.slang`.
fn away_from_zero(v: f32, floor: f32) -> f32 {
    if v.abs() < floor {
        if v < 0.0 { -floor } else { floor }
    } else {
        v
    }
}

/// darktable `_loop_switch`'s per-pixel code in `f32`, from the prepared matrices.
/// Without guards, as darktable writes it: no guard on the illuminant's cone responses
/// or the u'v' denominators, `powf` giving NaN for a negative base, and `f32::max` and
/// `min` ignoring a NaN, as C's `fmaxf` and `fminf` do. With guards, as the kernel
/// computes it.
fn restated(p: &Prepared, rgb: [f32; 3], guards: bool) -> Restated {
    let mut t = Trace {
        guards,
        fired: false,
        finite: true,
    };
    let min = if p.clip { 0.0 } else { -f32::MAX };
    let clip = |v: [f32; 3]| v.map(|c| c.max(min));
    let clipneg = |v: [f32; 3]| if p.clip { v.map(|c| c.max(0.0)) } else { v };
    let length = |v: [f32; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    // The kernel's norm: scaled by the largest component, the same value but for
    // rounding, so it never overflows.
    let norm_of = |v: [f32; 3], guards: bool| {
        let largest = largest(v);
        if guards && largest > 0.0 {
            length(v.map(|c| c / largest)) * largest
        } else {
            length(v)
        }
    };
    let d50 = match p.adaptation {
        Adaptation::LinearBradford | Adaptation::FullBradford => BRADFORD_D50,
        Adaptation::Cat16 => CAT16_D50,
        Adaptation::Xyz => XYZ_D50,
        Adaptation::Rgb => [1.0; 3],
    };
    assert_eq!(d50, p.d50);

    // The illuminant in the cone space, where an adaptation uses it.
    let illuminant = if p.adaptation == Adaptation::Rgb {
        [1.0; 3]
    } else {
        let cone_white = t.check(apply(&p.cone, illuminant_xy_to_xyz([p.x, p.y])));
        cone_white.map(|v| t.guard(v, away_from_zero(v, NORM_MIN)))
    };

    let input = clip(rgb.map(|v| t.guard(v, v.clamp(-INPUT_LIMIT, INPUT_LIMIT))));
    let lms = match p.adaptation {
        Adaptation::FullBradford => {
            let power = (0.818155f32 / illuminant[2]).powf(0.0834);
            let power = t.guard(power, if illuminant[2] > 0.0 { power } else { 1.0 });
            let xyz = t.check(apply(&p.to_lms, input));
            let y = xyz[1];
            let scale = if y > NORM_MIN { y + NORM_MIN } else { NORM_MIN };
            let lms = t.check(apply(&p.cone, xyz).map(|v| v / scale));
            let mut ratio = t.check([0, 1, 2].map(|c| lms[c] / illuminant[c]));
            if ratio[2] > 0.0 {
                let base = t.guard(ratio[2], ratio[2].min(RATIO_LIMIT));
                ratio[2] = base.powf(power);
            }
            [0, 1, 2].map(|c| d50[c] * ratio[c] * scale)
        }
        Adaptation::LinearBradford => {
            let lms = t.check(apply(&p.to_lms, input));
            [0, 1, 2].map(|c| d50[c] * (lms[c] / illuminant[c]))
        }
        Adaptation::Cat16 | Adaptation::Xyz => {
            let lms = t.check(apply(&p.to_lms, input));
            [0, 1, 2].map(|c| lms[c] * d50[c] / illuminant[c])
        }
        Adaptation::Rgb => input,
    };
    let lms = t.check(lms);
    let xyz = t.check(clipneg(apply(&p.mix_to_xyz, lms)));
    let xyz = xyz.map(|v| t.guard(v, v.clamp(-XYZ_LIMIT, XYZ_LIMIT)));

    // _gamut_mapping
    let sum = t.value(xyz[0] + xyz[1] + xyz[2]);
    let luminance = xyz[1];
    let sizes = xyz[0].abs() + xyz[1].abs() + xyz[2].abs();
    let divisor = t.guard(sum, sum.max(SUM_FLOOR * sizes));
    let mut xy = t.check(if sum > 0.0 {
        [xyz[0] / divisor, xyz[1] / divisor]
    } else {
        D50_XY
    });
    let denominator = -2.0 * xy[0] + 12.0 * xy[1] + 3.0;
    let terms = 2.0 * xy[0].abs() + 12.0 * xy[1].abs() + 3.0;
    let denominator = t.guard(denominator, away_from_zero(denominator, NORM_MIN * terms));
    let mut uv = t.check([4.0 * xy[0] / denominator, 9.0 * xy[1] / denominator]);
    const D50_UV: [f32; 2] = [0.20915914598542354, 0.488075320769787];
    let delta = t.check([D50_UV[0] - uv[0], D50_UV[1] - uv[1]]);
    let distance = t.value(luminance * (delta[0] * delta[0] + delta[1] * delta[1]));
    let mut neutral = false;
    let correction = if p.compression == 0.0 {
        0.0
    } else if !guards {
        distance.powf(p.compression)
    } else {
        // The kernel: the power never formed above 1 and held at most 2^20, which
        // matters only for a negative power, darktable's NaN for a negative base and a
        // compression that is not a whole number taken as D50, then the correction held
        // at most 1, where it lands on D50 whatever its size.
        let base = distance.abs();
        let (magnitude, bounded) = if base < 1.0 {
            (base.powf(p.compression), false)
        } else {
            let exponent = t.value(p.compression * base.log2());
            (
                exponent.min(LOG2_POWER_LIMIT).exp2(),
                exponent > LOG2_POWER_LIMIT,
            )
        };
        let correction = match (distance < 0.0, p.negative_distance) {
            (false, _) | (true, NegativeDistance::Even) => magnitude,
            (true, NegativeDistance::Odd) => {
                t.fired |= bounded;
                -magnitude
            }
            (true, NegativeDistance::Neutral) => {
                neutral = true;
                0.0
            }
        };
        correction.min(1.0)
    };
    for c in 0..2 {
        let moved = t.value(correction.mul_add(delta[c], uv[c]));
        uv[c] = if uv[c] > D50_UV[c] {
            moved.max(D50_UV[c])
        } else {
            moved.min(D50_UV[c])
        };
    }
    if neutral {
        uv = D50_UV;
    }
    let denominator = 6.0 * uv[0] - 16.0 * uv[1] + 12.0;
    let terms = 6.0 * uv[0].abs() + 16.0 * uv[1].abs() + 12.0;
    let denominator = t.guard(denominator, away_from_zero(denominator, NORM_MIN * terms));
    xy = t.check([9.0 * uv[0] / denominator, 4.0 * uv[1] / denominator]);
    if p.clip {
        xy = xy.map(|v| v.max(0.0));
    }
    xy[1] = xy[1].max(NORM_MIN);
    let scale = xy[0] + xy[1];
    if scale >= 1.0 {
        xy = xy.map(|v| v / scale);
    }
    let xyz = t.check([
        luminance * xy[0] / xy[1],
        luminance,
        luminance * (1.0 - xy[0] - xy[1]) / xy[1],
    ]);
    let xyz = xyz.map(|v| t.guard(v, v.clamp(-XYZ_LIMIT, XYZ_LIMIT)));

    let c = t.check(clipneg(apply(&p.to_space, xyz)));
    // _luma_chroma, version 3.
    let mut norm = t.value(norm_of(c, guards)).max(NORM_MIN);
    let average = ((c[0] + c[1] + c[2]) / 3.0).max(NORM_MIN);
    let mix = t.value(c[0] * p.lightness[0] + c[1] * p.lightness[1] + c[2] * p.lightness[2]);
    norm *= INVERSE_SQRT_3;
    let mut ratios = t.check(c.map(|v| v / norm));
    let coefficient =
        (ratios[0] * p.saturation[0] + ratios[1] * p.saturation[1] + ratios[2] * p.saturation[2])
            / 3.0;
    ratios = t.check(ratios.map(|r| (1.0 - r).mul_add(coefficient, r).max(r.min(0.0))));
    norm /= t.value(norm_of(ratios, guards)).max(NORM_MIN) * INVERSE_SQRT_3;
    let brightness = t.value((1.0 + mix / average).max(0.0));
    norm *= t.guard(brightness, brightness.min(BRIGHTNESS_LIMIT));
    let norm = t.value(norm);
    let out = t.check(clipneg(ratios.map(|r| r * norm)));

    let out = if p.apply_grey {
        let grey = (out[0] * p.grey[0] + out[1] * p.grey[1] + out[2] * p.grey[2]).max(0.0);
        [grey; 3]
    } else {
        let xyz = t.check(clipneg(apply(&p.from_space, out)));
        clipneg(apply(&p.to_rgb, xyz))
    };
    Restated {
        out: t.check(out),
        fired: t.fired,
        finite: t.finite,
    }
}

fn largest(v: [f32; 3]) -> f32 {
    v.iter().fold(0.0f32, |m, c| m.max(c.abs()))
}

/// Every adaptation, clipping on and off, and gamut compressions that take each of
/// darktable's paths for a negative distance (none, odd, even, not a whole number).
fn variations(base: &Settings) -> Vec<Settings> {
    let mut all = Vec::new();
    for adaptation in Adaptation::ALL {
        for clip in [true, false] {
            for gamut in [1.0, 0.0, 0.5, 4.0] {
                all.push(Settings {
                    adaptation,
                    clip,
                    gamut,
                    ..*base
                });
            }
        }
    }
    all
}

/// Spectral blue light, 450 nm, to which Bradford's second cone responds negatively
/// (about −3.2 at Y = 1), and spectral yellow, 580 nm, to which its third does.
const BLUE_LIGHT: [f32; 2] = [0.1566, 0.0177];
const YELLOW_LIGHT: [f32; 2] = [0.5125, 0.4866];

fn with_illuminant([x, y]: [f32; 2], base: &Settings) -> Settings {
    Settings {
        illuminant: Illuminant::Custom,
        x,
        y,
        ..*base
    }
}

#[test]
fn the_kernel_is_darktables_per_pixel_code() {
    // Against darktable's code wherever every value it computes is finite and no guard
    // of the kernel's fires, else against the kernel's guarded maths restated, relative
    // to the larger of the output and the input. With clipping, within 5e-5 (2.6e-6 on
    // the M4): f32 rounding and the GPU's own divisions and powers, after the
    // cancellations of test 0085's mixer. Spectral blue and yellow light, whose cone
    // gains reach 32 and change sign, within 5e-3 (1.3e-3). Without clipping,
    // out-of-gamut input can bring X + Y + Z near 0, where x and y grow large; there the
    // maths is ill-conditioned and the two agree within 1e-2 (3.7e-3).
    //
    // With clipping, test 0085's settings and the HP5+ preset fire no guard on the sweep,
    // in any adaptation, nor do blue and violet light with their negative Bradford
    // response, which keeps darktable's negative gain. Yellow light under non-linear
    // Bradford fires one: darktable's power is NaN there.
    let pixels = sweep();
    let mut settings: Vec<(Settings, bool, bool)> = Vec::new();
    for base in [TEST_0085, HP5] {
        settings.extend(variations(&base).into_iter().map(|s| (s, s.clip, false)));
    }
    let blue = with_illuminant(BLUE_LIGHT, &TEST_0085);
    settings.extend(variations(&blue).into_iter().map(|s| (s, s.clip, true)));
    let yellow = with_illuminant(YELLOW_LIGHT, &TEST_0085);
    settings.extend(variations(&yellow).into_iter().map(|s| (s, false, true)));
    settings.push((NO_OP, false, false));
    let mut worst = [0.0f32; 3];
    for (s, no_guard, spectral) in &settings {
        let group = if !s.clip { 2 } else { usize::from(*spectral) };
        let tolerance = [5e-5, 5e-3, 1e-2][group];
        let p = prepare(s);
        for (input, out) in pixels.iter().zip(render_prepared(&p, &pixels)) {
            assert!(
                out.iter().all(|v| v.is_finite()),
                "{input:?} gave {out:?} for {s:?}"
            );
            let darktable = restated(&p, *input, false);
            let kernel = restated(&p, *input, true);
            assert!(kernel.finite, "{input:?} for {s:?}: {kernel:?}");
            assert!(
                !(*no_guard && kernel.fired),
                "{input:?}: a guard fired for {s:?}"
            );
            let expected = if darktable.finite && !kernel.fired {
                darktable.out
            } else {
                kernel.out
            };
            // Relative to the larger of the output and the input: an output that is what
            // is left of cancelling terms carries the input's rounding.
            let error = (0..3).fold(0.0f32, |m, c| m.max((out[c] - expected[c]).abs()))
                / largest(expected).max(largest(*input)).max(1e-3);
            worst[group] = worst[group].max(error);
            assert!(
                error < tolerance,
                "{input:?} gave {out:?}, expected {expected:?}, for {s:?}"
            );
        }
    }
    println!(
        "largest difference: {:.1e} clipped, {:.1e} for spectral lights, {:.1e} unclipped",
        worst[0], worst[1], worst[2]
    );
}

#[test]
fn the_no_op_starting_point_is_the_identity_where_darktables_conditions_hold() {
    // darktable's presets start from the pipeline's illuminant in XYZ, without gamut
    // compression or clipping: an identity only for a norm of at least 2^-16, X + Y + Z
    // above 0, y at least 2^-16 and x + y below 1. Outside, it raises the norm, gives
    // D50's chromaticity, floors y or rescales x and y.
    let p = prepare(&NO_OP);
    let profile = WorkingProfile::linear_rec2020();
    let pixels = sweep();
    for (input, out) in pixels.iter().zip(render_prepared(&p, &pixels)) {
        let xyz = apply(&profile.rgb_to_xyz_d50, *input);
        let sum = xyz[0] + xyz[1] + xyz[2];
        let norm = largest(*input);
        let holds =
            norm >= 1e-4 && sum > 0.0 && xyz[1] / sum >= 1e-3 && xyz.iter().all(|&v| v >= 0.0);
        if !holds {
            continue;
        }
        for c in 0..3 {
            assert!(
                (out[c] - input[c]).abs() <= 1e-5 * norm,
                "{input:?} gave {out:?}"
            );
        }
    }
}

#[test]
fn the_illuminants_colour_comes_out_neutral() {
    // With the identity mixer and no compression, a surface of the illuminant's colour
    // becomes grey: R, G and B within 1e-3 of each other relative to their size, the
    // spread darktable's rounded constants leave (4.1e-4 to 7.5e-4).
    let profile = WorkingProfile::linear_rec2020();
    for adaptation in [
        Adaptation::LinearBradford,
        Adaptation::Cat16,
        Adaptation::FullBradford,
        Adaptation::Xyz,
    ] {
        for (illuminant, temperature) in [
            (Illuminant::Custom, 5000.0),
            (Illuminant::A, 5000.0),
            (Illuminant::D, 6500.0),
            (Illuminant::BlackBody, 3200.0),
            (Illuminant::F, 5000.0),
            (Illuminant::Led, 5000.0),
        ] {
            let settings = Settings {
                adaptation,
                illuminant,
                temperature,
                x: 0.31,
                y: 0.33,
                gamut: 0.0,
                ..Settings::DEFAULT
            };
            let p = prepare(&settings);
            let white = illuminant_xy_to_xyz([p.x, p.y]);
            let pixels: Vec<[f32; 3]> = [0.05f32, 0.18, 1.0]
                .iter()
                .map(|&level| apply(&profile.xyz_d50_to_rgb, white.map(|v| v * level)))
                .collect();
            for (input, out) in pixels.iter().zip(render_prepared(&p, &pixels)) {
                let spread = largest([out[0] - out[1], out[1] - out[2], out[2] - out[0]]);
                assert!(
                    spread <= 1e-3 * largest(out),
                    "{input:?} gave {out:?} for {illuminant:?} in {adaptation:?}"
                );
            }
        }
    }
}

#[test]
fn grey_output_is_grey_and_never_negative() {
    let pixels = sweep();
    for settings in variations(&HP5) {
        let p = prepare(&settings);
        for (input, out) in pixels.iter().zip(render_prepared(&p, &pixels)) {
            assert!(
                out[0] == out[1] && out[1] == out[2] && out[0] >= 0.0,
                "{input:?} gave {out:?} for {settings:?}"
            );
        }
    }
}

#[test]
fn every_value_stays_finite_on_extreme_input() {
    // Not only the output: every value the kernel computes, as its guarded maths
    // restated in f32 computes it, so no NaN or infinity hides behind a max() that drops
    // it. In every adaptation, with clipping on and off, at every compression path, for
    // test 0085's settings and the HP5+ preset, every illuminant at the ends of the
    // schema's ranges and tables, and mixer rows and grey weights whose sums nearly
    // cancel.
    let mut pixels = Vec::new();
    for v in [
        0.0,
        1e-30,
        -1e-30,
        1e-8,
        1.0,
        -1.0,
        1e4,
        -1e4,
        1e8,
        -1e8,
        3e14,
        1e15,
        -1e15,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ] {
        pixels.push([v; 3]);
        pixels.push([v, 0.0, 0.0]);
        pixels.push([0.0, v, -v]);
        pixels.push([v, -v, 1.0]);
    }
    let mut bases = vec![TEST_0085, HP5];
    for illuminant in [
        Illuminant::Pipe,
        Illuminant::A,
        Illuminant::D,
        Illuminant::E,
        Illuminant::F,
        Illuminant::Led,
        Illuminant::BlackBody,
        Illuminant::Custom,
        Illuminant::Camera,
    ] {
        for (temperature, fluorescent, led, x, y) in [
            (1667.0, 0, 0, 0.0, 0.0001),
            (25000.0, 11, 8, 0.9999, 0.0001),
            (4000.0, 5, 3, 0.0, 1.0),
        ] {
            bases.push(Settings {
                illuminant,
                temperature,
                illum_fluo: fluorescent,
                illum_led: led,
                x,
                y,
                ..TEST_0085
            });
        }
    }
    bases.push(with_illuminant(BLUE_LIGHT, &TEST_0085));
    bases.push(with_illuminant(YELLOW_LIGHT, &TEST_0085));
    // A row normalised by a sum of 1e-5 (gains of 1e5), and one left unnormalised.
    bases.push(Settings {
        red: [1.0, -0.99999, 0.0],
        green: [2.0, -1.9999995, 0.0],
        ..TEST_0085
    });
    bases.push(Settings {
        grey: [1.0, -0.99999, 0.0],
        ..HP5
    });
    for base in &bases {
        for s in variations(base) {
            let p = prepare(&s);
            for (input, out) in pixels.iter().zip(render_prepared(&p, &pixels)) {
                assert!(
                    out.iter().all(|v| v.is_finite()),
                    "{input:?} gave {out:?} for {s:?}"
                );
                let kernel = restated(&p, *input, true);
                assert!(
                    kernel.finite,
                    "{input:?}: a value is not finite for {s:?}: {kernel:?}"
                );
            }
        }
    }
}

/// darktable's own defaults for color calibration on the 29 raw.pixls.us samples the exit
/// test compares (the whole-pipeline test), each `<Make>/<Model>/<file>` with the
/// illuminant, x and y and temperature `reload_defaults` stores: read from the history
/// darktable-cli embeds in an unedited export (no sidecar, a fresh configuration,
/// `--apply-custom-presets false`, so the scene-referred workflow's preset, which takes the
/// module's defaults).
const DARKTABLE_DEFAULTS: [(&str, Illuminant, [f32; 2], f32); 29] = [
    (
        "Adobe DNG Converter/Canon EOS 5D Mark III/5G4A9394-compressed-lossless.DNG",
        Illuminant::Custom,
        [0.4252481460571289, 0.4112094044685364],
        3227.179931640625,
    ),
    (
        "Canon/EOS 5D Mark IV/B13A0729.CR2",
        Illuminant::D,
        [0.37760594487190247, 0.38625046610832214],
        4134.7001953125,
    ),
    (
        "Canon/EOS 7D Mark II/capture000106.cr2",
        Illuminant::D,
        [0.3630446791648865, 0.37675556540489197],
        4495.54443359375,
    ),
    (
        "Canon/EOS 90D/Canon-90D-RAW-ISO-100.CR3",
        Illuminant::D,
        [0.3367232382297516, 0.3532083034515381],
        5327.896484375,
    ),
    (
        "Canon/EOS M50/Canon_EOS_M50_CRAW_ISO_100_nocrop_nodual.CR3",
        Illuminant::D,
        [0.4567256271839142, 0.4132149815559387],
        2723.263916015625,
    ),
    (
        "Canon/EOS R5/Canon_EOS_R5_CRAW_ISO_100_nocrop_nodual.CR3",
        Illuminant::Custom,
        [0.39277976751327515, 0.39467599987983704],
        3805.50830078125,
    ),
    (
        "Canon/EOS R6 Mark II/canon-r6m2-img_0346-raw-normal-full.cr3",
        Illuminant::BlackBody,
        [0.34265029430389404, 0.34879687428474426],
        5098.22412109375,
    ),
    (
        "Fujifilm/GFX 50S/20170525_0036TEST.RAF",
        Illuminant::D,
        [0.38204437494277954, 0.3886508047580719],
        4033.01513671875,
    ),
    (
        "Fujifilm/X-H2S/IMG-20221005-0665.raf",
        Illuminant::D,
        [0.37010860443115234, 0.37957122921943665],
        4305.1240234375,
    ),
    (
        "Fujifilm/X-T5/DSCF0021.RAF",
        Illuminant::BlackBody,
        [0.3598983883857727, 0.3653758764266968],
        4547.603515625,
    ),
    (
        "Fujifilm/X100VI/DSCF0074.RAF",
        Illuminant::BlackBody,
        [0.3447182774543762, 0.3526223599910736],
        5026.72265625,
    ),
    (
        "Google/Pixel 4a/PXL_20201121_100251397.dng",
        Illuminant::Custom,
        [0.37224987149238586, 0.38830316066741943],
        4285.84375,
    ),
    (
        "Leica/Q2/L1000750.DNG",
        Illuminant::Custom,
        [0.36463215947151184, 0.38676267862319946],
        4487.5615234375,
    ),
    (
        "Nikon/D5100/00.nikon.12b-lossy-raw.nef",
        Illuminant::D,
        [0.4160034656524658, 0.4030682146549225],
        3352.086669921875,
    ),
    (
        "Nikon/D850/Nikon-D850-12bit-lossless-compressed.NEF",
        Illuminant::BlackBody,
        [0.3705316185951233, 0.36893802881240845],
        4242.1796875,
    ),
    (
        "Nikon/D850/Nikon-D850-14bit-compressed.NEF",
        Illuminant::BlackBody,
        [0.3705316185951233, 0.36893802881240845],
        4242.1796875,
    ),
    (
        "Nikon/Z 6/DSC_0750.NEF",
        Illuminant::D,
        [0.4585377871990204, 0.4103979468345642],
        2709.771484375,
    ),
    (
        "Nikon/Z 9/Nikon_-_NIKON_Z_9_-_14bit_compressed_(Lossless).NEF",
        Illuminant::Custom,
        [0.3851586878299713, 0.3888202905654907],
        3955.7734375,
    ),
    (
        "OM System/OM-1 Mark II/20mp-12bit.orf",
        Illuminant::D,
        [0.37147456407546997, 0.3808433413505554],
        4273.0712890625,
    ),
    (
        "Olympus/E-M1MarkIII/_3160529.ORF",
        Illuminant::Custom,
        [0.4003484547138214, 0.4026147127151489],
        3681.38427734375,
    ),
    (
        "Panasonic/DC-GH6/1-1.RW2",
        Illuminant::Custom,
        [0.38433393836021423, 0.39121201634407043],
        3989.471923828125,
    ),
    (
        "Panasonic/DC-S5M2/P1126458_mechanical.RW2",
        Illuminant::D,
        [0.36976757645606995, 0.3793656826019287],
        4313.72607421875,
    ),
    (
        "Panasonic/DC-S5M2/P1126473_96MP.RW2",
        Illuminant::D,
        [0.36976757645606995, 0.3793656826019287],
        4313.72607421875,
    ),
    (
        "Pentax/K-1 Mark II/_IMG2387.DNG",
        Illuminant::D,
        [0.3731016218662262, 0.3772692382335663],
        4210.953125,
    ),
    (
        "Pentax/K-3 Mark III/IMGP9854.PEF",
        Illuminant::D,
        [0.373922199010849, 0.3860223889350891],
        4230.84716796875,
    ),
    (
        "Samsung/NX1/2016-07-23-142101_sam_9364.srw",
        Illuminant::D,
        [0.3813008964061737, 0.38426682353019714],
        4028.301513671875,
    ),
    (
        "Sony/ILCE-1/A1_full_compressed.ARW",
        Illuminant::D,
        [0.37600648403167725, 0.38502421975135803],
        4170.66015625,
    ),
    (
        "Sony/ILCE-6400/DSC00087.ARW",
        Illuminant::D,
        [0.3609854578971863, 0.378569632768631],
        4562.7587890625,
    ),
    (
        "Sony/ILCE-7RM5/7RM5-LosslessCompressedLarge.ARW",
        Illuminant::D,
        [0.36765506863594055, 0.38211026787757874],
        4384.7744140625,
    ),
];

/// The operation's defaults are darktable's on every sample the exit test compares: the
/// illuminant `reload_defaults` snaps to, and the camera's light and temperature from
/// rawler's metadata as darktable's from its own.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn the_defaults_are_darktables_on_the_corpus() {
    use apertrust_operations::channelmixerrgb::ChannelMixerRgb;
    use apertrust_plugin::Operation;

    let dir = golden::render::corpus_dir().unwrap();
    let (mut xy_gap, mut temperature_gap) = (0.0f32, 0.0f32);
    for (name, illuminant, [x, y], temperature) in DARKTABLE_DEFAULTS {
        let raw = golden::raw_stage::decode(&dir.join(name)).unwrap();
        let image = apertrust_engine::raw::image_context(&raw);
        let ours = channelmixerrgb::settings(&ChannelMixerRgb.defaults(&image));
        let gap = (ours.x - x).abs().max((ours.y - y).abs());
        println!(
            "{name}: {:?} at {:.3} K, x {:.7}, y {:.7}; darktable's {illuminant:?} at \
             {temperature:.3} K, x {x:.7}, y {y:.7}; {gap:.1e} apart",
            ours.illuminant, ours.temperature, ours.x, ours.y
        );
        assert_eq!(ours.illuminant, illuminant, "{name}");
        assert_eq!(ours.adaptation, Adaptation::Cat16, "{name}");
        assert!(gap < 2e-7, "{name}: x and y {gap:e} apart");
        // On the Canon M50, below 3000 K, the reverse lookup decides, on a minimum so flat
        // that darktable's fast-math build tips it to the next of its samples, 0.14 K up
        // (the spec's Deliberate differences); every other sample within 0.01 K.
        let m50 = name.starts_with("Canon/EOS M50/");
        let bound = if m50 { 0.15 } else { 0.01 };
        let off = (ours.temperature - temperature).abs();
        assert!(off < bound, "{name}: the temperature {off} K apart");
        xy_gap = xy_gap.max(gap);
        temperature_gap = temperature_gap.max(off);
    }
    println!("x and y within {xy_gap:.1e}, the temperature within {temperature_gap:.3} K");
}
