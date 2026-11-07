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

//! Invariants of color balance rgb on the GPU, through its operation's `prepare`, needing
//! no darktable (`docs/ports/colorbalancergb.md`, Test plan). CI runs them on software
//! GPUs.

use std::sync::OnceLock;

use apertrust_color::{self as color, WorkingProfile, XYZ_D50_TO_D65_CAT16, XYZ_D65_TO_LMS_2006};
use apertrust_engine::compute::{GpuImage, ImageKernel};
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_operations::colorbalancergb::{self, ColorBalanceRgb, SaturationFormula};
use apertrust_plugin::{ImageContext, ImageDesc, Kind, Operation, ParamValues, Value};

/// The GPU and the compiled kernels, made once for every test in this file: compiling
/// takes tens of seconds on a software GPU.
struct Gpu {
    context: GpuContext,
    kernel: ImageKernel,
    /// `colorbalancergb_forward_diff`, whose output is not clipped as `main`'s is.
    forward: ImageKernel,
}

fn gpu() -> &'static Gpu {
    static GPU: OnceLock<Gpu> = OnceLock::new();
    GPU.get_or_init(|| {
        let context = pollster::block_on(GpuContext::new(&GpuOptions::default()))
            .expect("a GPU adapter, hardware or software");
        let wgsl = colorbalancergb::KERNEL.wgsl;
        let kernel =
            pollster::block_on(ImageKernel::new(&context, "colorbalancergb", wgsl)).unwrap();
        let forward = pollster::block_on(ImageKernel::with_entry_point(
            &context,
            "colorbalancergb",
            wgsl,
            "colorbalancergb_forward_diff",
        ))
        .unwrap();
        Gpu {
            context,
            kernel,
            forward,
        }
    })
}

/// Both saturation formulas: every invariant here holds for each.
const FORMULAS: [SaturationFormula; 2] = [SaturationFormula::JzAzBz, SaturationFormula::Ucs];

/// The operation's output, and its derivative along every gradient parameter at once,
/// with this saturation formula and these parameters over its defaults, on one row of
/// pixels.
fn render_with_derivative(
    formula: SaturationFormula,
    set: &[(&str, f32)],
    pixels: &[[f32; 3]],
) -> (Vec<[f32; 3]>, Vec<[f32; 3]>) {
    let Gpu {
        context: gpu,
        kernel,
        forward,
    } = gpu();
    let mut values = ParamValues::new();
    values.set("saturation_formula", Value::Choice(formula.to_darktable()));
    for &(name, value) in set {
        values.set(name, Value::Float(value));
    }
    let params = ColorBalanceRgb.schema().resolve(&values).unwrap();
    let rgba: Vec<[f32; 4]> = pixels.iter().map(|&[r, g, b]| [r, g, b, 1.0]).collect();
    let width = u32::try_from(rgba.len()).unwrap();
    let prepared = ColorBalanceRgb
        .prepare(
            &params,
            &ImageContext::full(width, 1),
            &ImageDesc::rgba(width, 1),
        )
        .unwrap();
    let pass = &prepared.passes[0];
    let tables: Vec<&[u8]> = pass.tables.iter().map(Vec::as_slice).collect();
    let input = GpuImage::upload(gpu, width, 1, &rgba).unwrap();
    let output = GpuImage::new(gpu, width, 1);
    kernel.dispatch(gpu, &input, &output, &pass.params, &tables);
    let direction = GpuImage::upload(gpu, width, 1, &vec![[1.0; 4]; rgba.len()]).unwrap();
    let derivative = GpuImage::new(gpu, width, 1);
    forward.dispatch_derivative(gpu, &input, &direction, &derivative, &pass.params, &tables);
    let rgb = |image: &GpuImage| -> Vec<[f32; 3]> {
        image
            .download(gpu)
            .unwrap()
            .into_iter()
            .map(|[r, g, b, _]| [r, g, b])
            .collect()
    };
    (rgb(&output), rgb(&derivative))
}

/// The operation, with these parameters over its defaults, on one row of pixels.
fn render(formula: SaturationFormula, set: &[(&str, f32)], pixels: &[[f32; 3]]) -> Vec<[f32; 3]> {
    render_with_derivative(formula, set, pixels).0
}

/// Filmlight Yrg of a working-space pixel, as the kernel computes it: its luminance Y
/// and its chromaticity (r, g).
fn yrg(rgb: [f32; 3]) -> [f32; 3] {
    let profile = WorkingProfile::linear_rec2020();
    let to_lms = color::mul(
        &XYZ_D65_TO_LMS_2006,
        &color::mul(&XYZ_D50_TO_D65_CAT16, &profile.rgb_to_xyz_d50),
    );
    color::lms_to_yrg(color::apply(&to_lms, rgb))
}

/// Greys and colours well inside Rec2020, from deep shadow to well above white.
fn inside_the_gamut() -> Vec<[f32; 3]> {
    let mut pixels = Vec::new();
    for level in [1e-3, 0.01, 0.05, 0.18, 0.5, 1.0, 4.0] {
        for [r, g, b] in [
            [1.0, 1.0, 1.0],
            [1.0, 0.8, 0.6],
            [0.6, 1.0, 0.7],
            [0.6, 0.7, 1.0],
            [0.9, 0.6, 0.8],
        ] {
            pixels.push([r * level, g * level, b * level]);
        }
    }
    pixels
}

fn close(a: f32, b: f32, relative: f32) -> bool {
    (a - b).abs() <= relative * b.abs().max(1e-3)
}

/// About five times the largest error on the M4: the round trip through Yrg, grading RGB
/// and JzAzBz, whose PQ curve's power of 134 magnifies the GPU's `pow`; darktable UCS's
/// is smaller.
const ROUND_TRIP: f32 = 5e-4;

#[test]
fn defaults_keep_colours_inside_the_gamut() {
    // With every control at its default the operation is an identity, but for its gamut
    // mapping, which only reaches colours near the working gamut's edge (below).
    let pixels = inside_the_gamut();
    for formula in FORMULAS {
        let out = render(formula, &[], &pixels);
        for (input, output) in pixels.iter().zip(&out) {
            for c in 0..3 {
                assert!(
                    close(output[c], input[c], ROUND_TRIP),
                    "{formula:?}: {input:?} became {output:?}"
                );
            }
        }
    }
}

#[test]
fn defaults_still_map_colours_at_the_gamut_edge() {
    // darktable's gamut mapping runs whatever the settings: a pure Rec2020 primary is
    // pulled inside, and a negative channel is clipped. So the operation at its defaults
    // is not a no-op, and a binding leaves it out of the pipeline until a control moves.
    let pixels = [[0.5, 0.0, 0.0], [0.0, 0.0, 0.5], [0.4, -0.05, 0.2]];
    for formula in FORMULAS {
        let out = render(formula, &[], &pixels);
        for (input, output) in pixels.iter().zip(&out) {
            assert!(
                output.iter().all(|v| v.is_finite() && *v >= 0.0),
                "{formula:?}: {output:?}"
            );
            let moved = (0..3).any(|c| !close(output[c], input[c], 1e-3));
            assert!(moved, "{formula:?}: {input:?} stayed {output:?}");
        }
    }
}

#[test]
fn contrast_raises_luminance_to_a_power_around_the_grey_fulcrum() {
    // Y' = fulcrum (Y / fulcrum)^(1 + contrast), at constant chromaticity.
    let pixels = inside_the_gamut();
    let settings = [(0.3, 0.1845), (-0.4, 0.1845), (0.5, 0.3), (-0.2, 0.1)];
    for (formula, (contrast, fulcrum)) in FORMULAS
        .into_iter()
        .flat_map(|formula| settings.map(|setting| (formula, setting)))
    {
        let out = render(
            formula,
            &[("contrast", contrast), ("grey_fulcrum", fulcrum)],
            &pixels,
        );
        for (input, output) in pixels.iter().zip(&out) {
            let [y, r, g] = yrg(*input);
            let [y_out, r_out, g_out] = yrg(*output);
            let want = fulcrum * (y / fulcrum).powf(1.0 + contrast);
            assert!(
                close(y_out, want, ROUND_TRIP),
                "{formula:?}, contrast {contrast}, fulcrum {fulcrum}: {input:?} has Y {y_out}, not {want}"
            );
            assert!(
                (r_out - r).abs() < 2e-4 && (g_out - g).abs() < 2e-4,
                "{formula:?}, contrast {contrast}: {input:?} moved from rg ({r}, {g}) to ({r_out}, {g_out})"
            );
        }
    }
}

#[test]
fn the_grey_at_the_fulcrum_stays_where_it_is() {
    // The grey whose Yrg luminance is the fulcrum: RGB (v, v, v) with Y(v) = fulcrum.
    for formula in FORMULAS {
        for fulcrum in [0.1845f32, 0.25] {
            let v = fulcrum / yrg([1.0; 3])[0];
            for contrast in [-0.9, -0.3, 0.4, 1.0] {
                let out = render(
                    formula,
                    &[("contrast", contrast), ("grey_fulcrum", fulcrum)],
                    &[[v; 3]],
                )[0];
                for c in out {
                    assert!(
                        close(c, v, ROUND_TRIP),
                        "{formula:?}, contrast {contrast}: {v} became {out:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn extreme_inputs_and_settings_stay_finite() {
    let pixels = [
        [0.0, 0.0, 0.0],
        [1e-8, 1e-8, 1e-8],
        [-1.0, -0.5, -2.0],
        [0.3, -0.2, 0.1],
        [1e3, 1e3, 1e3],
        [1e3, 0.0, 10.0],
        [0.0, 50.0, 0.0],
    ];
    // Every control at one end of its range, midtones_Y at -1 included: its luminance
    // power, 1 / (1 + midtones_Y), is then 10^6, which darktable's powf overflows above
    // the white fulcrum and the kernel's bounded power does not.
    let every_control_at = |low: bool| -> Vec<(&str, f32)> {
        ColorBalanceRgb
            .schema()
            .params
            .iter()
            .filter_map(|param| match param.kind {
                Kind::Float { min, max, .. } => Some((param.name, if low { min } else { max })),
                _ => None,
            })
            .collect()
    };
    for set in [
        vec![],
        vec![("contrast", 1.0)],
        vec![("contrast", -1.0)],
        vec![("contrast", 0.5), ("grey_fulcrum", 0.0)],
        vec![("contrast", 0.5), ("grey_fulcrum", 1.0)],
        vec![("mask_grey_fulcrum", 0.0)],
        vec![("midtones_Y", -1.0)],
        vec![("midtones_Y", -1.0), ("white_fulcrum", -16.0)],
        vec![
            ("vibrance", 1.0),
            ("chroma_global", 1.0),
            ("saturation_global", 1.0),
        ],
        every_control_at(true),
        every_control_at(false),
    ] {
        for formula in FORMULAS {
            assert_finite(formula, &set, &pixels);
        }
    }
}

/// The operation's output and its derivatives are finite for every pixel. The output is
/// clipped at 0, which can hide an infinity or NaN inside the kernel; the forward
/// derivatives are not clipped, so they carry one out.
fn assert_finite(formula: SaturationFormula, set: &[(&str, f32)], pixels: &[[f32; 3]]) {
    let (out, derivative) = render_with_derivative(formula, set, pixels);
    for (k, input) in pixels.iter().enumerate() {
        assert!(
            out[k].iter().chain(&derivative[k]).all(|v| v.is_finite()),
            "{formula:?}, {set:?}: {input:?} became {:?}, derivative {:?}",
            out[k],
            derivative[k]
        );
    }
}

#[test]
fn the_midtones_wheel_stays_finite_past_its_pole() {
    // From a chroma of about 0.31 the midtones wheel's 1 + v crosses 0 at some hues,
    // where darktable's power has a negative or infinite exponent: chroma 0.4 at 180.02°
    // gave an exponent of -2.8e6 and 0.6 at 27.34° one of -19831, turning ordinary
    // pixels black through infinity and NaN (docs/ports/colorbalancergb.md, Precision).
    let mut pixels = inside_the_gamut();
    pixels.extend([
        [0.0; 3],
        [1e-6, 2e-6, 1e-6],
        [0.3, 0.0, 0.1],
        [20.0, 15.0, 10.0],
    ]);
    let mut wheels = vec![(0.4f32, 180.02f32), (0.6, 27.34)];
    for chroma in [0.4, 0.6, 0.8, 1.0] {
        wheels.extend((0..120).map(|step| (chroma, step as f32 * 3.0)));
    }
    for (chroma, hue) in wheels {
        for formula in FORMULAS {
            assert_finite(
                formula,
                &[("midtones_C", chroma), ("midtones_H", hue)],
                &pixels,
            );
        }
    }
}
