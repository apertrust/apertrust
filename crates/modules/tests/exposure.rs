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

//! Exposure's invariants (`docs/modules/exposure.md`, Invariants), one test per row, each
//! named after its row. The synthetic ramp and sweep run on CI's software GPUs. The
//! raw.pixls.us corpus runs locally, with `APERTRUST_RAW_CORPUS` naming it:
//!
//! ```sh
//! cargo test --release -p apertrust-modules --test exposure -- --ignored --nocapture
//! ```
//!
//! Synthetic images enter after demosaic, already in the working space and white
//! balanced: their rows plan the operations from exposure on and leave colorin out,
//! colour calibration adapting from D50 (`common::Bench::keep`), over a colour raw's
//! image context with the exposure bias the row names, so the relative controls are
//! tested over a base that is not zero (`docs/modules/README.md`). A row reads the
//! scene, the image entering AgX; Exposure's own output, the image leaving the exposure
//! operation; or the frame, colorout's output before it is rounded to 8 bits, clipped to
//! 0 to 1 as a PNG holds it.
//!
//! Contrast's and the pivot's rows left with them for Colour balance RGB (decision 2):
//! E1, the first step, is that module's row B1.

#[expect(
    dead_code,
    reason = "since Contrast left, no Exposure row measures a ΔE2000 jump or a luminance"
)]
mod common;
mod corpus;

use apertrust_engine::edit::{self, Edit};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe};
use apertrust_engine::sidecar;
use apertrust_modules::exposure::{self, Exposure};
use apertrust_modules::{Base, ControlValue, MODULES, Module};
use apertrust_operations::OPERATIONS;
use apertrust_plugin::{ImageContext, ImageDesc, ParamValues, Value};

use common::{
    BEFORE_THE_WORKING_SPACE, Bench, Picture, RAMP_STEPS, Rendered, exact, extended_ramp, id,
    raw_context, small_raw, sweep,
};
use corpus::{CORPUS_VIEWPORT, NINETY_D, corpus_bench, each_corpus_raw, export_bench};

/// The input exposure's default for `image` renders as `scene`: out = (in − b₀) / (w₀ − b₀).
fn landing(image: &ImageContext, scene: f64) -> f32 {
    let base = Base::new(OPERATIONS, image);
    let (default, black) = (
        base.float("exposure", "exposure"),
        base.float("exposure", "black"),
    );
    let white = f64::from(exposure::rendered_white(
        default,
        exposure::camera_bias(image),
    ));
    (scene * (white - f64::from(black)) + f64::from(black)) as f32
}

/// The ramp (`docs/modules/README.md`, Test images): neutral greys one per 1/16 EV from
/// 2⁻¹⁶ to 2⁴, RGB 0.18 and 0.1744, then the inputs `image`'s default exposure brings to
/// those two. One row.
fn ramp(bias: f32) -> (Picture, ImageContext) {
    let image = raw_context((RAMP_STEPS + 4) as u32, 1, bias);
    let then = [0.18, 0.1744, landing(&image, 0.18), landing(&image, 0.1744)];
    common::ramp(&then, bias)
}

/// What a row reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Measure {
    /// The image entering AgX.
    Scene,
    /// The image leaving exposure, the module's one operation: what E4, E30 and E31
    /// measure, since colour calibration, between exposure and AgX on every colour raw
    /// (decision 27), raises the darkest values, clips negative RGB and compresses the
    /// gamut, none of which scales with the image.
    Module,
    /// The output, colorout's.
    Frame,
}

impl common::Measure for Measure {
    /// From exposure on for a synthetic image; ending before AgX for the scene, and after
    /// exposure for the module's output.
    fn keep(self, instances: &mut Vec<Instance<'static>>, synthetic: bool) {
        instances.retain(|i| !(synthetic && BEFORE_THE_WORKING_SPACE.contains(&id(i))));
        let end = match self {
            Measure::Scene => instances.iter().position(|i| id(i) == "agx"),
            Measure::Module => instances
                .iter()
                .position(|i| id(i) == "exposure")
                .map(|exposure| exposure + 1),
            Measure::Frame => None,
        };
        if let Some(end) = end {
            instances.truncate(end);
        }
    }
}

fn value_of(control: &str, value: f32) -> ControlValue {
    match control {
        "camera_bias" | "compensate_hilite_pres" => ControlValue::Bool(value != 0.0),
        "mode" => ControlValue::Choice(value as u32),
        _ => ControlValue::Number(value),
    }
}

impl Bench {
    /// Colour balance rgb joining the plan at its defaults over an unedited photo, as the
    /// Colour balance RGB module's first step adds it: the rows render it between their
    /// edits, so the one pipeline changes shape on the way, as the session's does.
    fn joined(&self, measure: Measure) -> Vec<Instance<'static>> {
        let mut instances = edit::instances(&Edit::default(), OPERATIONS, MODULES, &self.image)
            .all()
            .unwrap();
        instances.push(Instance {
            operation: apertrust_operations::operation("colorbalancergb").unwrap(),
            params: ParamValues::new(),
        });
        self.keep(instances, measure)
    }

    /// These Exposure controls set, through an edit, as a client sets them.
    fn set(&mut self, set: &[(&str, f32)], measure: Measure) -> Rendered {
        self.edit(&edit_of(set), measure)
    }
}

/// An edit with these Exposure controls set, in this order.
fn edit_of(set: &[(&str, f32)]) -> Edit {
    let mut edit = Edit::default();
    for &(control, value) in set {
        edit.set_control(
            &Exposure,
            control,
            value_of(control, value),
            &common::photo(),
        )
        .unwrap();
    }
    edit
}

/// Each pixel's and channel's value over `before`'s, where `before`'s is over 1e-6: the
/// spread of those factors, relative to their median, and the median.
fn factor(before: &[[f32; 4]], after: &[[f32; 4]]) -> (f64, f64) {
    let mut ratios: Vec<f64> = before
        .iter()
        .zip(after)
        .flat_map(|(b, a)| (0..3).map(move |c| (b[c], a[c])))
        .filter(|(b, _)| b.abs() > 1e-6)
        .map(|(b, a)| f64::from(a) / f64::from(b))
        .collect();
    ratios.sort_by(f64::total_cmp);
    let median = ratios[ratios.len() / 2];
    let spread = (ratios[ratios.len() - 1] - ratios[0]) / median;
    (spread, median)
}

const SCENE: Pipe = Pipe::Export;

/// Exposure's default on every image the rows bench, colour raws all: darktable's +0.7 EV.
const DEFAULT: f32 = 0.7;

// E0: every control at its default.

/// Edits that leave every control at its default: each control written at its default,
/// and each kept there.
fn untouched() -> Vec<Edit> {
    let defaults = [
        ("mode", 0.0),
        ("camera_bias", 1.0),
        ("compensate_hilite_pres", 0.0),
        ("exposure", DEFAULT),
        ("deflicker_percentile", 50.0),
        ("deflicker_target_level", -4.0),
        ("black_level", 0.0),
    ];
    let mut kept = Edit::default();
    for (control, _) in defaults {
        kept.keep(&Exposure, control, true).unwrap();
    }
    vec![edit_of(&defaults), kept]
}

/// E0 on one pipeline, as the session renders: no edit, then colour balance rgb joining
/// and leaving the plan, then each untouched edit, every frame as the first.
fn e0_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Measure::Frame);
    let joined = bench.joined(Measure::Frame);
    bench.render(&joined);
    for edit in untouched() {
        let values = edit.module("exposure").unwrap();
        assert!(
            Exposure
                .bind(
                    &values.off_defaults(&Exposure.defaults(&bench.image)),
                    &bench.image,
                    &Base::new(OPERATIONS, &bench.image)
                )
                .is_empty()
        );
        let at_defaults = bench.edit(&edit, Measure::Frame);
        assert!(exact(&none, &at_defaults), "E0 on {:?}", bench.pipe);
    }
}

#[test]
fn e0_every_control_at_its_default_renders_as_no_edit() {
    for bias in [0.0, -0.67] {
        for pipe in [
            Pipe::Export,
            Pipe::Viewport {
                width: 40,
                height: 30,
            },
        ] {
            let (mosaic, image) = small_raw(bias);
            e0_holds(&mut Bench::mosaic(&mosaic, image, pipe));
        }
        let (picture, image) = ramp(bias);
        e0_holds(&mut Bench::picture(&picture, image, Pipe::Export));
        let (picture, image) = sweep(bias);
        e0_holds(&mut Bench::picture(&picture, image, Pipe::Export));
    }
}

// E3: each control moved, then reset.

/// Each move, then reset by `ResetControl` (a double-click or Delete) and by
/// `ResetModule` (the header's reset), on the bench's one pipeline, whose shape changes
/// on the way: colour balance rgb joins and leaves between the moves.
fn e3_holds(bench: &mut Bench, moves: &[&[(&str, f32)]]) {
    let none = bench.edit(&Edit::default(), Measure::Frame);
    for set in moves {
        let mut edit = edit_of(set);
        let moved = bench.edit(&edit, Measure::Frame);
        assert!(!exact(&none, &moved), "{set:?} changes the frame");
        for (control, _) in *set {
            edit.reset_control(&Exposure, control).unwrap();
        }
        assert_eq!(edit, Edit::default());
        assert!(
            exact(&none, &bench.edit(&edit, Measure::Frame)),
            "{set:?}, reset control by control"
        );
        let joined = bench.joined(Measure::Frame);
        bench.render(&joined);
        let mut edit = edit_of(set);
        bench.edit(&edit, Measure::Frame);
        edit.reset_module(&Exposure);
        assert!(
            exact(&none, &bench.edit(&edit, Measure::Frame)),
            "{set:?}, the module reset"
        );
    }
}

const MOVES: [&[(&str, f32)]; 4] = [
    &[("exposure", 1.0)],
    &[("black_level", 0.01)],
    &[("camera_bias", 0.0)],
    &[
        ("exposure", -2.0),
        ("black_level", -0.05),
        ("camera_bias", 0.0),
    ],
];

#[test]
fn e3_a_control_moved_and_reset_renders_as_no_edit() {
    let (picture, image) = ramp(-0.67);
    e3_holds(&mut Bench::picture(&picture, image, Pipe::Export), &MOVES);
    let (mosaic, image) = small_raw(-0.67);
    let viewport = Pipe::Viewport {
        width: 40,
        height: 30,
    };
    e3_holds(&mut Bench::mosaic(&mosaic, image, viewport), &MOVES);
}

// E4 and E8: Exposure scales every pixel alike, by darktable's factor.

/// Exposure at `value` over `untouched`, read where `measure` reads: its factor's spread
/// and its median.
fn exposure_factor(
    bench: &mut Bench,
    untouched: &Rendered,
    value: f32,
    measure: Measure,
) -> (f64, f64) {
    let moved = bench.set(&[("exposure", value)], measure);
    factor(&untouched.pixels, &moved.pixels)
}

/// The moves from the photo's default the rows make, in EV.
const MOVES_EV: [f32; 6] = [-4.0, -1.0, -0.01, 0.01, 1.0, 4.0];

/// The values E4 and E8 set: the moves from the default, and the range's ends,
/// darktable's ±18 EV.
fn exposures() -> Vec<f32> {
    MOVES_EV
        .iter()
        .map(|ev| DEFAULT + ev)
        .chain([-18.0, 18.0])
        .collect()
}

/// The factor darktable's exposure scales its untouched output by at `value` on
/// `image`: (w₀ − b₀) / (w − b₀), w = 2^−(e − β) the white each renders and b₀ the
/// photo's black (`src/iop/exposure.c`, `commit_params`).
fn darktable_factor(image: &ImageContext, value: f32) -> f64 {
    let base = Base::new(OPERATIONS, image);
    let bias = exposure::camera_bias(image);
    let black = f64::from(base.float("exposure", "black"));
    let white = |e: f32| f64::from(exposure::rendered_white(e, bias));
    (white(base.float("exposure", "exposure")) - black) / (white(value) - black)
}

#[test]
fn e4_exposure_scales_every_pixel_alike() {
    for (name, (picture, image)) in [("ramp", ramp(-0.67)), ("sweep", sweep(-0.67))] {
        let mut bench = Bench::picture(&picture, image, SCENE);
        let untouched = bench.edit(&Edit::default(), Measure::Module);
        let values = exposures();
        let spreads: Vec<f64> = values
            .iter()
            .map(|&value| exposure_factor(&mut bench, &untouched, value, Measure::Module).0)
            .collect();
        let worst = spreads.iter().copied().fold(0.0, f64::max);
        println!("E4 on the {name}: a spread of at most {worst:.1e}");
        for (value, spread) in values.iter().zip(spreads) {
            assert!(spread <= 1e-5, "E4 at {value:+} EV: a spread of {spread:e}");
        }
    }
}

/// E8's relative error at `value`: Exposure's factor over its untouched output against
/// darktable's.
fn e8_error(bench: &mut Bench, output: &Rendered, value: f32) -> f64 {
    let (_, factor) = exposure_factor(bench, output, value, Measure::Module);
    let expected = darktable_factor(&bench.image, value);
    (factor - expected).abs() / expected
}

#[test]
fn e8_exposure_renders_darktable_s_factor_for_any_bias() {
    for bias in [0.0, -0.67, -5.0] {
        let (picture, image) = ramp(bias);
        let mut bench = Bench::picture(&picture, image, SCENE);
        let output = bench.edit(&Edit::default(), Measure::Module);
        let mut worst = 0.0_f64;
        for value in exposures() {
            let error = e8_error(&mut bench, &output, value);
            worst = worst.max(error);
            assert!(
                error <= 1e-5,
                "E8 at {value:+} EV, bias {bias}: {error:e} relative"
            );
        }
        println!("E8 with a bias of {bias:+} EV: at most {worst:.1e} relative");
    }
}

#[test]
fn e6_a_pasted_exposure_means_the_same_whatever_the_bias() {
    let factors: Vec<f64> = [0.0, -0.67]
        .into_iter()
        .map(|bias| {
            let (picture, image) = ramp(bias);
            let mut bench = Bench::picture(&picture, image, SCENE);
            let untouched = bench.edit(&Edit::default(), Measure::Scene);
            exposure_factor(&mut bench, &untouched, DEFAULT + 1.0, Measure::Scene)
                .1
                .log2()
        })
        .collect();
    let difference = (factors[0] - factors[1]).abs();
    println!("E6: the factors differ by {difference:.1e} EV");
    assert!(difference <= 0.01, "E6: {factors:?}");
}

// E7: Exposure at its ends stays finite and in order.

/// Whether the extended ramp's frame is finite and never darker for a brighter grey,
/// channel by channel; the first grey where it is not.
fn in_order(frame: &Rendered) -> Result<(), String> {
    for (i, pair) in frame.pixels.windows(2).enumerate() {
        for c in 0..3 {
            if !pair[1][c].is_finite() || pair[1][c] < pair[0][c] {
                return Err(format!("grey {}: {:?} after {:?}", i + 1, pair[1], pair[0]));
            }
        }
    }
    Ok(())
}

#[test]
fn e7_exposure_at_its_ends_renders_finite_and_in_order() {
    let (picture, image) = extended_ramp(-0.67);
    let mut bench = Bench::picture(&picture, image, Pipe::Export);
    for ev in [-18.0, DEFAULT - 4.0, DEFAULT + 4.0, 18.0] {
        let frame = bench.set(&[("exposure", ev)], Measure::Frame);
        assert_eq!(in_order(&frame), Ok(()), "E7 at {ev:+} EV");
    }
}

// E14: the order controls were set in.

/// One pipeline sets `first`, then both; another sets `second`, then both: the two
/// frames are the same, byte for byte.
fn e14_holds(first: (&str, f32), second: (&str, f32)) {
    let (picture, image) = ramp(-0.67);
    let mut one = Bench::picture(&picture, image.clone(), Pipe::Export);
    one.set(&[first], Measure::Frame);
    let one = one.set(&[first, second], Measure::Frame);
    let mut other = Bench::picture(&picture, image, Pipe::Export);
    other.set(&[second], Measure::Frame);
    let other = other.set(&[second, first], Measure::Frame);
    assert!(exact(&one, &other), "E14 with {first:?} and {second:?}");
}

#[test]
fn e14_exposure_and_black_level_render_the_same_whichever_came_first() {
    e14_holds(("exposure", 1.0), ("black_level", 0.01));
    e14_holds(("exposure", -1.0), ("camera_bias", 0.0));
}

// E30 and E31: Black level correction and Compensate camera exposure.

#[test]
fn e30_black_level_sets_black_and_keeps_white() {
    for bias in [0.0, -0.67, -5.0] {
        let (picture, image) = ramp(bias);
        let mut bench = Bench::picture(&picture, image.clone(), SCENE);
        let base = Base::new(OPERATIONS, &image);
        let default = base.float("exposure", "black");
        let white = exposure::rendered_white(
            base.float("exposure", "exposure"),
            exposure::camera_bias(&image),
        );
        // The worst error, and the worst where a black that reaches the white goes 0.01
        // below it, whose margin amplifies the GPU's `exp2` error (decision 30).
        let (mut worst, mut at_stop) = (0.0_f64, 0.0_f64);
        // The range's ends, darktable's ±1, and its soft range's, the track's.
        for level in [-1.0, -0.1, -0.01, 0.01, 0.1, 1.0] {
            let set = (default + level).clamp(-1.0, 1.0);
            let stopped = set >= white;
            let black = f64::from(if stopped { white - 0.01 } else { set });
            let output = bench.set(&[("black_level", level)], Measure::Module);
            for (input, output) in picture.pixels.iter().zip(&output.pixels) {
                let expected = (f64::from(input[0]) - black) / (f64::from(white) - black);
                let error = (f64::from(output[0]) - expected).abs() / expected.abs().max(1e-2);
                worst = worst.max(error);
                if stopped {
                    at_stop = at_stop.max(error);
                }
                assert!(
                    error <= 1e-5,
                    "E30 at {level:+}, bias {bias}: {} renders {} for {expected}",
                    input[0],
                    output[0]
                );
            }
        }
        println!(
            "E30 with a bias of {bias:+} EV: at most {worst:.1e} relative, {at_stop:.1e} at \
             the stop"
        );
    }
}

#[test]
fn e31_use_camera_bias_off_darkens_by_the_bias() {
    let (picture, image) = ramp(-0.67);
    let mut bench = Bench::picture(&picture, image, SCENE);
    let untouched = bench.edit(&Edit::default(), Measure::Module);
    let off = bench.set(&[("camera_bias", 0.0)], Measure::Module);
    let (spread, factor) = factor(&untouched.pixels, &off.pixels);
    println!("E31: a spread of {spread:.1e}, {:+.4} EV", factor.log2());
    assert!(spread <= 1e-5, "E31: a spread of {spread:e}");
    assert!(
        (factor.log2() + 0.67).abs() <= 0.02,
        "E31: {} EV",
        factor.log2()
    );

    let (picture, image) = ramp(0.0);
    let mut bench = Bench::picture(&picture, image, SCENE);
    let untouched = bench.edit(&Edit::default(), Measure::Module);
    let off = bench.set(&[("camera_bias", 0.0)], Measure::Module);
    assert_eq!(
        untouched.pixels, off.pixels,
        "E31: with no bias, nothing changes"
    );
}

// E50: the picker.

/// E50: a pick on a flat patch, by Exposure's pick with the patch's reading, sets the
/// exposure that renders the patch's luminance at the target lightness's grey at
/// Exposure's output, Y read through the input profile darktable's pick reads it through;
/// only Exposure changes. On colour raws with biases 0 and −0.67 EV, compensated, at
/// darktable's default target, 50 %, and at 30 % and 75 %. The photo's own black, −2⁻¹²,
/// which darktable's rule leaves aside, keeps a patch from landing exactly: the darker
/// the patch, the further.
#[test]
fn e50_a_pick_maps_a_flat_patch_to_the_target_lightness() {
    use apertrust_modules::pick::{PickKeys, Reading, Stats};

    let patches = [
        [0.05, 0.05, 0.05],
        [0.18, 0.18, 0.18],
        [0.5, 0.5, 0.5],
        [1.0, 1.0, 1.0],
        [0.3, 0.2, 0.1],
    ];
    let pixels: Vec<[f32; 4]> = patches.iter().map(|&[r, g, b]| [r, g, b, 1.0]).collect();
    let mut worst = 0.0_f64;
    for bias in [0.0, -0.67] {
        let (picture, image) = Picture::row(pixels.clone(), bias);
        let mut bench = Bench::picture(&picture, image.clone(), SCENE);
        let to_xyz = exposure::input_profile(&image).unwrap();
        let luminance = |rgb: &[f32; 4]| {
            (0..3)
                .map(|c| f64::from(to_xyz[1][c] * rgb[c]))
                .sum::<f64>()
        };
        for target in [30.0, 50.0, 75.0] {
            let target_y = ((f64::from(target) + 16.0) / 116.0).powi(3);
            let tools = [
                ("area_mode", ControlValue::Choice(0)),
                ("lightness", ControlValue::Number(target)),
            ];
            for (at, patch) in picture.pixels.iter().enumerate() {
                let stats = Stats {
                    mean: *patch,
                    min: *patch,
                    max: *patch,
                };
                let reading = Reading {
                    input: stats,
                    output: None,
                };
                let values = Exposure.defaults(&image);
                let picked = Exposure
                    .pick(
                        "exposure",
                        &reading,
                        &values,
                        &image,
                        &tools,
                        None,
                        None,
                        PickKeys::default(),
                    )
                    .unwrap();
                let written: Vec<&str> = picked.writes.iter().map(|(c, _)| c).collect();
                assert_eq!(written, ["exposure"], "E50: only Exposure changes");
                let exposure = picked.writes.number("exposure");
                let output = bench.set(&[("exposure", exposure)], Measure::Module);
                let off = (luminance(&output.pixels[at]) / target_y).log2().abs();
                worst = worst.max(off);
                assert!(
                    off <= 0.01,
                    "E50: {patch:?} at L {target}, bias {bias}: {off:.4} EV off"
                );
            }
        }
    }
    println!("E50: at most {worst:.4} EV from the target");
}

// E9: darktable's golden case.

/// E9: Exposure and Black level correction set to the golden exposure case's values as
/// darktable's sliders show them, Exposure the parameter and Black level the case's black
/// over the photo's, bind darktable's parameters bit for bit, and through the operation's
/// `prepare` the kernel's inputs the golden harness checks against darktable's render.
/// On an image that is not a raw, whose defaults are 0, and on a colour raw, whose
/// Exposure starts at +0.7 EV, with its camera's bias not compensated, as the case has it.
#[test]
fn e9_the_golden_case_s_values_bind_to_darktable_s_parameters_bit_for_bit() {
    let case = golden::cases::find("exposure").unwrap();
    let darktable = golden::xmp::module_params(case.xmp, "exposure").unwrap();
    let fields: Vec<f32> = darktable
        .bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|word| f32::from_le_bytes(*word))
        .collect();
    let (black, value) = (fields[1], fields[2]);
    let theirs = golden::ports::kernel_inputs(case).unwrap().unwrap().params;
    let operation = apertrust_operations::operation("exposure").unwrap();
    for image in [ImageContext::full(1, 1), raw_context(1, 1, -0.67)] {
        let own = Base::new(OPERATIONS, &image).float("exposure", "black");
        let mut edit = Edit::default();
        for (control, value) in [
            ("exposure", ControlValue::Number(value)),
            ("black_level", ControlValue::Number(black - own)),
            ("camera_bias", ControlValue::Bool(false)),
        ] {
            edit.set_control(&Exposure, control, value, &image).unwrap();
        }
        let instances = edit::instances(&edit, OPERATIONS, MODULES, &image)
            .all()
            .unwrap();
        let instance = instances.iter().find(|i| id(i) == "exposure").unwrap();
        // Over the operation's defaults for the photo, as the engine resolves them.
        let params = operation
            .schema()
            .resolve_over(&operation.defaults(&image), &instance.params)
            .unwrap();
        for (param, expected) in [("exposure", value), ("black", black)] {
            let Some(Value::Float(bound)) = params.get(param) else {
                panic!("{param}");
            };
            assert_eq!(bound.to_bits(), expected.to_bits(), "E9: {param}");
        }
        let prepared = operation
            .prepare(&params, &image, &ImageDesc::rgba(1, 1))
            .unwrap();
        assert_eq!(prepared.passes[0].params, theirs, "E9: the kernel's inputs");
    }
}

// Edits an earlier version saved.

/// A version 1 edit, whose Exposure solved for exact stops: its value is left out with a
/// warning, its mark with it, so Exposure renders at the photo's default; Black level
/// correction and Compensate camera exposure keep theirs, and the older Contrast and
/// Pivot are left out with a warning each, as before.
#[test]
fn version_1_s_exposure_is_left_out_and_the_rest_reads() {
    let saved = edit_of(&[
        ("exposure", 1.5),
        ("black_level", 0.01),
        ("camera_bias", 0.0),
    ]);
    let version_2 = sidecar::to_xml(&saved, OPERATIONS, MODULES, "0.0.0");
    let version_1 = version_2
        .replace(r#"moduleVersion="2""#, r#"moduleVersion="1""#)
        .replace(
            r#"control:exposure="1.5""#,
            r#"control:exposure="1.5" control:contrast="12" control:pivot="25""#,
        )
        .replace(r#"apertrust:kept=""#, r#"apertrust:kept="contrast "#);
    assert!(version_1.contains(r#"moduleVersion="1""#), "{version_2}");
    assert!(version_1.contains("control:pivot"), "{version_2}");
    assert!(version_1.contains("kept=\"contrast "), "{version_2}");
    let read = |xml: &str| {
        sidecar::from_xml(xml, OPERATIONS, MODULES, apertrust_modules::RETIRED).unwrap()
    };
    let loaded = read(&version_1);
    assert_eq!(loaded.newer, None);
    let expected = edit_of(&[("black_level", 0.01), ("camera_bias", 0.0)]);
    assert_eq!(loaded.edit, expected);
    // Exposure's value, Contrast's and Pivot's, and Contrast's mark.
    assert_eq!(loaded.warnings.len(), 4, "{:?}", loaded.warnings);
    assert!(
        loaded
            .warnings
            .iter()
            .any(|w| w.contains("exposure_in_stops")),
        "{:?}",
        loaded.warnings
    );
    // Through the engine it binds what the photo's default renders with those two.
    let image = common::photo();
    let bound = |edit: &Edit| {
        edit::instances(edit, OPERATIONS, MODULES, &image)
            .all()
            .unwrap()
            .into_iter()
            .find(|i| id(i) == "exposure")
            .unwrap()
            .params
    };
    assert_eq!(bound(&loaded.edit), bound(&expected));
    // Version 2's Exposure reads as it was written.
    let loaded = read(&version_2);
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    assert_eq!(loaded.edit, saved);
}

// E32: the controls whose parameters are not ported.

/// What a script or a pasted edit may give the controls not ported, each off its
/// default, as the edit's file writes it, and automatic.
const NOT_PORTED: [(&str, f32, &str); 6] = [
    ("compensate_hilite_pres", 1.0, "true"),
    ("deflicker_percentile", 0.0, "0"),
    ("deflicker_percentile", 100.0, "100"),
    ("deflicker_target_level", -18.0, "-18"),
    ("deflicker_target_level", 18.0, "18"),
    ("mode", 1.0, "automatic"),
];

/// Each refused as a command, the edit as it was, and left out of a pasted edit with a
/// warning, the control beside it kept, so no edit holds a value the render ignores.
#[test]
fn e32_what_is_not_ported_takes_only_its_default() {
    let exposed = edit_of(&[("exposure", 1.0)]);
    let xml = sidecar::to_xml(&exposed, OPERATIONS, MODULES, "0.0.0");
    for (control, value, text) in NOT_PORTED {
        let mut edit = exposed.clone();
        assert!(
            edit.set_control(
                &Exposure,
                control,
                value_of(control, value),
                &common::photo()
            )
            .is_err(),
            "E32: {control} {value}"
        );
        assert_eq!(edit, exposed, "E32: {control} {value}");

        let pasted = xml.replace(
            "control:exposure=\"1\"",
            &format!("control:exposure=\"1\" control:{control}=\"{text}\""),
        );
        assert_ne!(pasted, xml);
        let loaded =
            sidecar::from_xml(&pasted, OPERATIONS, MODULES, apertrust_modules::RETIRED).unwrap();
        assert_eq!(loaded.edit, exposed, "E32: {control}={text} pasted");
        assert_eq!(loaded.warnings.len(), 1, "{:?}", loaded.warnings);
    }
}

// The corpus, locally.

/// E0 on every corpus raw: Exposure at its defaults, written or kept, plans the very
/// instances and cache keys no edit does, so the same frame.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn e0_on_the_corpus() {
    let mut raws = 0;
    each_corpus_raw(|name, image, input| {
        let bench = export_bench(image.clone(), input.clone());
        let keys = |instances: &[Instance]| -> Vec<(&str, u64)> {
            pipeline::plan(instances, &image, input.desc(), Pipe::Export, &Limits::NONE)
                .unwrap()
                .operations
                .iter()
                .map(|op| (op.id, op.key))
                .collect()
        };
        let none = keys(&bench.instances(&Edit::default(), Measure::Frame));
        for edit in untouched() {
            assert_eq!(
                keys(&bench.instances(&edit, Measure::Frame)),
                none,
                "E0 on {name}"
            );
        }
        raws += 1;
    });
    println!("E0 holds on {raws} raws");
}

/// The other corpus raws rows name, by camera.
const Z6: &str = "Nikon/Z 6/DSC_0750.NEF";
const X_T5: &str = "Fujifilm/X-T5/DSCF0021.RAF";

/// E4 and E8 on the 90D (bias −0.67 EV), the Z 6 (none) and the X-T5 (X-Trans), in the
/// viewport, where Exposure's factor holds pixel by pixel at any scale, on Exposure's
/// output.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn e4_and_e8_on_the_corpus() {
    for name in [NINETY_D, Z6, X_T5] {
        let mut bench = corpus_bench(name, CORPUS_VIEWPORT);
        let bias = bench.image.camera.as_ref().unwrap().exposure_bias;
        let output = bench.edit(&Edit::default(), Measure::Module);
        for value in exposures() {
            let (spread, _) = exposure_factor(&mut bench, &output, value, Measure::Module);
            let error = e8_error(&mut bench, &output, value);
            println!(
                "{name} (bias {bias:+.2}) at {value:+} EV: spread {spread:.1e}, \
                 {error:.1e} relative"
            );
            assert!(spread <= 1e-5, "E4 on {name} at {value:+}: {spread:e}");
            assert!(error <= 1e-5, "E8 on {name} at {value:+}: {error:e}");
        }
    }
}

/// E3 and E14 on the 90D, through the viewport's frame and the export's: each move reset
/// by `ResetControl` and by `ResetModule` on one pipeline whose shape changes on the
/// way, and two orders of the same controls.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn e3_and_e14_on_the_corpus() {
    for pipe in [CORPUS_VIEWPORT, Pipe::Export] {
        e3_holds(&mut corpus_bench(NINETY_D, pipe), &MOVES);
        let mut one = corpus_bench(NINETY_D, pipe);
        one.set(&[("exposure", 1.0)], Measure::Frame);
        let one = one.set(&[("exposure", 1.0), ("black_level", 0.01)], Measure::Frame);
        let mut other = corpus_bench(NINETY_D, pipe);
        other.set(&[("black_level", 0.01)], Measure::Frame);
        let other = other.set(&[("black_level", 0.01), ("exposure", 1.0)], Measure::Frame);
        assert!(exact(&one, &other), "E14 on {pipe:?}");
    }
}

/// Contrast and its pivot are the Colour balance RGB module's now (decision 2): Exposure
/// refuses them, as the session's `SetControl`, the CLI and a saved edit do.
#[test]
fn contrast_and_its_pivot_are_no_longer_exposure_s() {
    let mut edit = Edit::default();
    for control in ["contrast", "pivot"] {
        assert!(
            edit.set_control(
                &Exposure,
                control,
                ControlValue::Number(10.0),
                &common::photo()
            )
            .is_err(),
            "{control}"
        );
    }
    assert_eq!(edit, Edit::default());
}

/// The module's defaults agree with the exposure operation's for every kind of image:
/// Compensate camera exposure as the operation's `compensate_exposure_bias`, and the
/// camera's bias as the operation itself clamps it.
#[test]
fn its_defaults_and_bias_agree_with_the_exposure_operation() {
    let mut monochrome = raw_context(8, 8, -8.0);
    monochrome.camera.as_mut().unwrap().cfa = None;
    for image in [
        raw_context(8, 8, 0.0),
        raw_context(8, 8, -8.0),
        raw_context(8, 8, 6.5),
        monochrome,
        ImageContext::full(8, 8),
    ] {
        let base = Base::new(OPERATIONS, &image);
        let compensated = base.flag("exposure", "compensate_exposure_bias");
        assert_eq!(Exposure.defaults(&image).flag("camera_bias"), compensated);
        let params = base.params("exposure").unwrap();
        let bias = if compensated {
            exposure::camera_bias(&image)
        } else {
            0.0
        };
        assert_eq!(
            apertrust_operations::exposure::effective_exposure(params, &image),
            params.float("exposure") - bias,
            "{:?}",
            image.camera.as_ref().map(|c| c.exposure_bias)
        );
    }
}
