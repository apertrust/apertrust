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

//! Exposure's invariants (`docs/tools/exposure.md`, Invariants), one test per row, each
//! named after its row. The synthetic ramp and sweep run on CI's software GPUs. The
//! raw.pixls.us corpus runs locally, with `APERTRUST_RAW_CORPUS` naming it:
//!
//! ```sh
//! cargo test --release -p apertrust-tools --test exposure -- --ignored --nocapture
//! ```
//!
//! Synthetic images enter after demosaic, already in the working space and white
//! balanced: their rows plan the operations from exposure on and leave colorin out,
//! colour calibration adapting from D50 (`common::Bench::keep`), over a colour raw's
//! image context with the exposure bias the row names, so the relative controls are
//! tested over a base that is not zero (`docs/tools/README.md`). A row reads the
//! scene, the image entering AgX; Exposure's own output, the image leaving the exposure
//! operation; or the frame, colorout's output before it is rounded to 8 bits, clipped to
//! 0 to 1 as a PNG holds it.
//!
//! Contrast's and the pivot's rows left with them for Colour balance RGB (decision 2):
//! E1, the first step, is that tool's row B1.

#[expect(
    dead_code,
    reason = "since Contrast left, no Exposure row measures a ΔE2000 jump or a luminance"
)]
mod common;
mod corpus;

use apertrust_engine::edit::{self, Edit};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe};
use apertrust_engine::sidecar;
use apertrust_operations::OPERATIONS;
use apertrust_plugin::{ImageContext, ImageDesc, ParamValues, Value};
use apertrust_tools::exposure::{self, Exposure};
use apertrust_tools::{Base, ControlValue, TOOLS, Tool};

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
    let white = f64::from(exposure::rendered_white(default, taken_off(image)));
    (scene * (white - f64::from(black)) + f64::from(black)) as f32
}

/// What the exposure operation takes off the exposure on `image` at its defaults: the
/// camera's bias less the exposure a highlight-preserving mode held back, both
/// compensated where the operation's defaults compensate them.
fn taken_off(image: &ImageContext) -> f32 {
    let base = Base::new(OPERATIONS, image);
    exposure::taken_off(
        image,
        base.flag("exposure", "compensate_exposure_bias"),
        base.flag("exposure", "compensate_hilite_pres"),
    )
}

/// The ramp (`docs/tools/README.md`, Test images): neutral greys one per 1/16 EV from
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
    /// The image leaving exposure, the tool's one operation: what E4, E30 and E31
    /// measure, since colour calibration, between exposure and AgX on every colour raw
    /// (decision 27), raises the darkest values, clips negative RGB and compresses the
    /// gamut, none of which scales with the image.
    Tool,
    /// The output, colorout's.
    Frame,
}

impl common::Measure for Measure {
    /// From exposure on for a synthetic image; ending before AgX for the scene, and after
    /// exposure for the tool's output.
    fn keep(self, instances: &mut Vec<Instance<'static>>, synthetic: bool) {
        instances.retain(|i| !(synthetic && BEFORE_THE_WORKING_SPACE.contains(&id(i))));
        let end = match self {
            Measure::Scene => instances.iter().position(|i| id(i) == "agx"),
            Measure::Tool => instances
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
    /// Colour balance RGB tool's first step adds it: the rows render it between their
    /// edits, so the one pipeline changes shape on the way, as the session's does.
    fn joined(&self, measure: Measure) -> Vec<Instance<'static>> {
        let mut instances = edit::instances(&Edit::default(), OPERATIONS, TOOLS, &self.image)
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
        ("compensate_hilite_pres", 1.0),
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
        let values = edit.tool("exposure").unwrap();
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
/// `ResetTool` (the header's reset), on the bench's one pipeline, whose shape changes
/// on the way: colour balance rgb joins and leaves between the moves.
fn e3_holds(bench: &mut Bench, moves: &[&[(&str, f32)]]) {
    let none = bench.edit(&Edit::default(), Measure::Frame);
    for set in moves {
        let mut edit = edit_of(set);
        let moved = bench.edit(&edit, Measure::Frame);
        assert!(
            !common::same_pixels(&none, &moved),
            "{set:?} changes the frame"
        );
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
        edit.reset_tool(&Exposure);
        assert!(
            exact(&none, &bench.edit(&edit, Measure::Frame)),
            "{set:?}, the tool reset"
        );
    }
}

/// E3's moves, on images whose camera held exposure back, so Highlight preservation mode
/// off changes the frame.
const MOVES: [&[(&str, f32)]; 5] = [
    &[("exposure", 1.0)],
    &[("black_level", 0.01)],
    &[("camera_bias", 0.0)],
    &[("compensate_hilite_pres", 0.0)],
    &[
        ("exposure", -2.0),
        ("black_level", -0.05),
        ("camera_bias", 0.0),
        ("compensate_hilite_pres", 0.0),
    ],
];

/// `image` with 0.5 EV held back by a highlight-preserving mode, as on the 90D.
fn holding_back(mut image: ImageContext) -> ImageContext {
    image.camera.as_mut().unwrap().highlight_preservation = 0.5;
    image
}

#[test]
fn e3_a_control_moved_and_reset_renders_as_no_edit() {
    let (picture, image) = ramp(-0.67);
    let image = holding_back(image);
    e3_holds(&mut Bench::picture(&picture, image, Pipe::Export), &MOVES);
    let (mosaic, image) = small_raw(-0.67);
    let image = holding_back(image);
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
    let bias = taken_off(image);
    let black = f64::from(base.float("exposure", "black"));
    let white = |e: f32| f64::from(exposure::rendered_white(e, bias));
    (white(base.float("exposure", "exposure")) - black) / (white(value) - black)
}

#[test]
fn e4_exposure_scales_every_pixel_alike() {
    for (name, (picture, image)) in [("ramp", ramp(-0.67)), ("sweep", sweep(-0.67))] {
        let mut bench = Bench::picture(&picture, image, SCENE);
        let untouched = bench.edit(&Edit::default(), Measure::Tool);
        let values = exposures();
        let spreads: Vec<f64> = values
            .iter()
            .map(|&value| exposure_factor(&mut bench, &untouched, value, Measure::Tool).0)
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
    let (_, factor) = exposure_factor(bench, output, value, Measure::Tool);
    let expected = darktable_factor(&bench.image, value);
    (factor - expected).abs() / expected
}

#[test]
fn e8_exposure_renders_darktable_s_factor_for_any_bias() {
    for bias in [0.0, -0.67, -5.0] {
        let (picture, image) = ramp(bias);
        let mut bench = Bench::picture(&picture, image, SCENE);
        let output = bench.edit(&Edit::default(), Measure::Tool);
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
    for (i, pixel) in frame.pixels.iter().enumerate() {
        if pixel[..3].iter().any(|value| !value.is_finite()) {
            return Err(format!("grey {i}: non-finite {pixel:?}"));
        }
    }
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
        let white = exposure::rendered_white(base.float("exposure", "exposure"), taken_off(&image));
        // The worst error, and the worst where a black that reaches the white goes 0.01
        // below it, whose margin amplifies the GPU's `exp2` error (decision 30).
        let (mut worst, mut at_stop) = (0.0_f64, 0.0_f64);
        // The range's ends, darktable's ±1, and its soft range's, the track's.
        for level in [-1.0, -0.1, -0.01, 0.01, 0.1, 1.0] {
            let set = (default + level).clamp(-1.0, 1.0);
            let stopped = set >= white;
            let black = f64::from(if stopped { white - 0.01 } else { set });
            let output = bench.set(&[("black_level", level)], Measure::Tool);
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
    let untouched = bench.edit(&Edit::default(), Measure::Tool);
    let off = bench.set(&[("camera_bias", 0.0)], Measure::Tool);
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
    let untouched = bench.edit(&Edit::default(), Measure::Tool);
    let off = bench.set(&[("camera_bias", 0.0)], Measure::Tool);
    assert_eq!(
        untouched.pixels, off.pixels,
        "E31: with no bias, nothing changes"
    );
}

/// E33: Highlight preservation mode off, on a raw whose camera held 0.5 EV back and one
/// that held 2 EV, with a bias of −0.67 EV compensated, darkens by what was held back,
/// one factor for every pixel; on a raw that held none back, nothing changes.
#[test]
fn e33_highlight_preservation_mode_off_darkens_by_what_was_held_back() {
    for held in [0.5, 2.0] {
        let (picture, mut image) = ramp(-0.67);
        image.camera.as_mut().unwrap().highlight_preservation = held;
        let mut bench = Bench::picture(&picture, image, SCENE);
        let untouched = bench.edit(&Edit::default(), Measure::Tool);
        let off = bench.set(&[("compensate_hilite_pres", 0.0)], Measure::Tool);
        let (spread, factor) = factor(&untouched.pixels, &off.pixels);
        println!(
            "E33: {held} EV held back, a spread of {spread:.1e}, {:+.4} EV",
            factor.log2()
        );
        assert!(spread <= 1e-5, "E33: a spread of {spread:e}");
        assert!(
            (factor.log2() + f64::from(held)).abs() <= 0.02,
            "E33: {} EV",
            factor.log2()
        );
    }

    let (picture, image) = ramp(-0.67);
    let mut bench = Bench::picture(&picture, image, SCENE);
    let untouched = bench.edit(&Edit::default(), Measure::Tool);
    let off = bench.set(&[("compensate_hilite_pres", 0.0)], Measure::Tool);
    assert_eq!(
        untouched.pixels, off.pixels,
        "E33: with none held back, nothing changes"
    );
}

// E50 and E52: the picker.

/// E50's patches: four greys from 0.05 to 1 and one of colour.
const PATCHES: [[f32; 4]; 5] = [
    [0.05, 0.05, 0.05, 1.0],
    [0.18, 0.18, 0.18, 1.0],
    [0.5, 0.5, 0.5, 1.0],
    [1.0, 1.0, 1.0, 1.0],
    [0.3, 0.2, 0.1, 1.0],
];

/// Picker target holding `target`, and Lightness at `lightness` (decision 88).
fn picker_target(
    target: exposure::PickerTarget,
    lightness: f32,
) -> [(&'static str, ControlValue); 2] {
    use apertrust_base::settings::Choice;
    let at = exposure::PickerTarget::OPTIONS
        .iter()
        .position(|(option, _)| *option == target)
        .unwrap();
    [
        ("target", ControlValue::Choice(at as u32)),
        ("lightness", ControlValue::Number(lightness)),
    ]
}

/// Exposure's pick of a flat `patch` at its defaults on `image`, with these settings.
fn flat_pick(
    image: &ImageContext,
    patch: [f32; 4],
    settings: &[(&str, ControlValue)],
) -> apertrust_tools::pick::Picked {
    flat_pick_by("exposure", image, patch, settings)
}

/// `picker`'s pick of a flat `patch` at Exposure's defaults on `image`.
fn flat_pick_by(
    picker: &str,
    image: &ImageContext,
    patch: [f32; 4],
    settings: &[(&str, ControlValue)],
) -> apertrust_tools::pick::Picked {
    use apertrust_tools::pick::{PickKeys, Reading, Stats};
    let stats = Stats {
        mean: patch,
        min: patch,
        max: patch,
    };
    let reading = Reading {
        input: stats,
        output: None,
        input_maximum: [1.0; 3],
    };
    Exposure
        .pick(
            picker,
            &reading,
            &Exposure.defaults(image),
            image,
            settings,
            None,
            None,
            PickKeys::default(),
        )
        .unwrap()
}

/// E50: a pick on a flat patch, by Exposure's pick with the patch's reading, sets the
/// exposure that renders the patch's luminance at the target lightness's grey at
/// Exposure's output, Y read through the input profile darktable's pick reads it through;
/// only Exposure changes. On colour raws with biases 0 and −0.67 EV, compensated, Picker
/// target holding Lightness, at darktable's default target, 50 %, and at 30 % and 75 %.
/// The photo's own black, −2⁻¹², which darktable's rule leaves aside, keeps a patch from
/// landing exactly: the darker the patch, the further.
#[test]
fn e50_a_pick_maps_a_flat_patch_to_the_target_lightness() {
    let mut worst = 0.0_f64;
    let pixels = PATCHES.to_vec();
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
            let settings = picker_target(exposure::PickerTarget::Lightness, target);
            for (at, patch) in picture.pixels.iter().enumerate() {
                let picked = flat_pick(&image, *patch, &settings);
                let written: Vec<&str> = picked.writes.iter().map(|(c, _)| c).collect();
                assert_eq!(written, ["exposure"], "E50: only Exposure changes");
                let exposure = picked.writes.number("exposure");
                let output = bench.set(&[("exposure", exposure)], Measure::Tool);
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

/// E52: a pick toward Middle grey writes exactly what one toward Lightness at 50 % writes,
/// on the same area, whatever Lightness holds, on E50's patches and photos (decision 88).
#[test]
fn e52_middle_grey_picks_exactly_what_lightness_at_50_picks() {
    use exposure::PickerTarget::{Lightness, MiddleGrey};
    for bias in [0.0, -0.67] {
        let image = raw_context(1, 1, bias);
        for patch in PATCHES {
            let lightness = flat_pick(&image, patch, &picker_target(Lightness, 50.0));
            assert!(!lightness.writes.is_empty(), "E52: {patch:?} writes");
            for held in [0.0, 50.0, 92.0] {
                let grey = flat_pick(&image, patch, &picker_target(MiddleGrey, held));
                assert_eq!(
                    grey.writes, lightness.writes,
                    "E52: {patch:?}, bias {bias}, Lightness at {held}"
                );
            }
        }
    }
}

/// The reference's L the reference row's picker hands back for `patch` on `image`.
fn measured(image: &ImageContext, patch: [f32; 4]) -> f32 {
    let picked = flat_pick_by("reference", image, patch, &[]);
    match picked.shown.settings[..] {
        [("reference_lightness", ControlValue::Number(l))] => l,
        _ => panic!("{:?}", picked.shown.settings),
    }
}

/// E53: a reference measured on one photo, then picked toward on another, renders the
/// area at the reference's L at Exposure's output, Black level correction at 0 on both,
/// as E50 holds: E50's patches measured on a colour raw with no bias, picked toward on
/// one with a bias of −0.67 EV, and the other way round (decision 88).
#[test]
fn e53_a_reference_matches_an_area_across_two_photos() {
    use exposure::PickerTarget::Reference;
    let mut worst = 0.0_f64;
    for (from, to) in [(0.0, -0.67), (-0.67, 0.0)] {
        let source = raw_context(1, 1, from);
        let (picture, image) = Picture::row(PATCHES.to_vec(), to);
        let mut bench = Bench::picture(&picture, image.clone(), SCENE);
        let to_xyz = exposure::input_profile(&image).unwrap();
        let luminance = |rgb: &[f32; 4]| {
            (0..3)
                .map(|c| f64::from(to_xyz[1][c] * rgb[c]))
                .sum::<f64>()
        };
        for (at, patch) in picture.pixels.iter().enumerate() {
            let l = measured(&source, PATCHES[at]);
            let mut settings = picker_target(Reference, 50.0).to_vec();
            settings.push(("reference_lightness", ControlValue::Number(l)));
            let picked = flat_pick(&image, *patch, &settings);
            let exposure = picked.writes.number("exposure");
            let output = bench.set(&[("exposure", exposure)], Measure::Tool);
            let target_y = ((f64::from(l) + 16.0) / 116.0).powi(3);
            let off = (luminance(&output.pixels[at]) / target_y).log2().abs();
            worst = worst.max(off);
            assert!(
                off <= 0.01,
                "E53: {patch:?} at L {l}, biases {from} to {to}: {off:.4} EV off"
            );
        }
    }
    println!("E53: at most {worst:.4} EV from the reference");
}

/// E54: the reference row's picker writes nothing, exactly, on every patch and photo; it
/// only hands the reference's L back (decision 88).
#[test]
fn e54_the_reference_picker_writes_nothing() {
    for bias in [0.0, -0.67] {
        let image = raw_context(1, 1, bias);
        for patch in PATCHES {
            let picked = flat_pick_by("reference", &image, patch, &[]);
            assert!(picked.writes.is_empty(), "E54: {patch:?}");
            assert_eq!(picked.shown.settings.len(), 1);
        }
    }
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
        let instances = edit::instances(&edit, OPERATIONS, TOOLS, &image)
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
/// Pivot are left out with a warning each, as before. Highlight preservation mode is off,
/// as every edit before version 3 rendered.
#[test]
fn version_1_s_exposure_is_left_out_and_the_rest_reads() {
    let saved = edit_of(&[
        ("exposure", 1.5),
        ("black_level", 0.01),
        ("camera_bias", 0.0),
    ]);
    let version_3 = sidecar::to_xml(&saved, OPERATIONS, TOOLS, "0.0.0");
    let version_1 = version_3
        .replace(r#"toolVersion="3""#, r#"toolVersion="1""#)
        .replace(
            r#"control:exposure="1.5""#,
            r#"control:exposure="1.5" control:contrast="12" control:pivot="25""#,
        )
        .replace(r#"apertrust:kept=""#, r#"apertrust:kept="contrast "#);
    assert!(version_1.contains(r#"toolVersion="1""#), "{version_3}");
    assert!(version_1.contains("control:pivot"), "{version_3}");
    assert!(version_1.contains("kept=\"contrast "), "{version_3}");
    let read =
        |xml: &str| sidecar::from_xml(xml, OPERATIONS, TOOLS, apertrust_tools::RETIRED).unwrap();
    let loaded = read(&version_1);
    assert_eq!(loaded.newer, None);
    // The migration sets Highlight preservation mode, unkept, as no one set it.
    let mut expected = edit_of(&[
        ("black_level", 0.01),
        ("camera_bias", 0.0),
        ("compensate_hilite_pres", 0.0),
    ]);
    expected
        .keep(&Exposure, "compensate_hilite_pres", false)
        .unwrap();
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
        edit::instances(edit, OPERATIONS, TOOLS, &image)
            .all()
            .unwrap()
            .into_iter()
            .find(|i| id(i) == "exposure")
            .unwrap()
            .params
    };
    assert_eq!(bound(&loaded.edit), bound(&expected));
    // Version 3's Exposure reads as it was written.
    let loaded = read(&version_3);
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    assert_eq!(loaded.edit, saved);
}

/// A version 2 edit, from before Highlight preservation mode was ported: it was off, the
/// one value it could hold, so it reads off and renders as it did, with no warning, on a
/// photo whose camera held exposure back; version 3's default is on.
#[test]
fn version_2_renders_with_highlight_preservation_mode_off() {
    let saved = edit_of(&[("exposure", 1.5)]);
    let version_3 = sidecar::to_xml(&saved, OPERATIONS, TOOLS, "0.0.0");
    let version_2 = version_3.replace(r#"toolVersion="3""#, r#"toolVersion="2""#);
    assert_ne!(version_2, version_3);
    let loaded =
        sidecar::from_xml(&version_2, OPERATIONS, TOOLS, apertrust_tools::RETIRED).unwrap();
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    let mut expected = edit_of(&[("exposure", 1.5), ("compensate_hilite_pres", 0.0)]);
    expected
        .keep(&Exposure, "compensate_hilite_pres", false)
        .unwrap();
    assert_eq!(loaded.edit, expected);
    let mut image = common::photo();
    image.camera.as_mut().unwrap().highlight_preservation = 0.5;
    let exposure = |edit: &Edit| {
        let instances = edit::instances(edit, OPERATIONS, TOOLS, &image)
            .all()
            .unwrap();
        let set = &instances
            .iter()
            .find(|i| id(i) == "exposure")
            .unwrap()
            .params;
        let operation = apertrust_operations::operation("exposure").unwrap();
        let params = operation
            .schema()
            .resolve_over(&operation.defaults(&image), set)
            .unwrap();
        apertrust_operations::exposure::effective_exposure(&params, &image)
    };
    // 1.5 EV less the camera's bias, as version 2 rendered; version 3 gives 0.5 back.
    let bias = exposure::camera_bias(&image);
    assert_eq!(exposure(&loaded.edit), 1.5 - bias);
    assert_eq!(exposure(&saved), 1.5 - bias + 0.5);
}

// E32: automatic mode, its percentile and its target level, every value taken.

/// What a script or a pasted edit may give automatic mode's controls, the ends of their
/// ranges, as the edit's file writes them, and automatic.
const AUTOMATIC: [(&str, f32, &str); 5] = [
    ("deflicker_percentile", 0.0, "0"),
    ("deflicker_percentile", 100.0, "100"),
    ("deflicker_target_level", -18.0, "-18"),
    ("deflicker_target_level", 18.0, "18"),
    ("mode", 1.0, "automatic"),
];

/// Each taken as a command and from a pasted edit, beside Exposure +1, with no warning:
/// nothing of Exposure is refused as not ported any more.
#[test]
fn e32_automatic_mode_and_its_settings_are_taken_over_their_ranges() {
    let exposed = edit_of(&[("exposure", 1.0)]);
    let xml = sidecar::to_xml(&exposed, OPERATIONS, TOOLS, "0.0.0");
    for (control, value, text) in AUTOMATIC {
        let mut edit = exposed.clone();
        edit.set_control(
            &Exposure,
            control,
            value_of(control, value),
            &common::photo(),
        )
        .unwrap_or_else(|error| panic!("E32: {control} {value}: {error}"));
        assert_ne!(edit, exposed, "E32: {control} {value}");

        let pasted = xml.replace(
            "control:exposure=\"1\"",
            &format!("control:exposure=\"1\" control:{control}=\"{text}\""),
        );
        assert_ne!(pasted, xml);
        let loaded =
            sidecar::from_xml(&pasted, OPERATIONS, TOOLS, apertrust_tools::RETIRED).unwrap();
        // A command marks what it sets; a pasted edit holds its values alone.
        let held = loaded.edit.tool("exposure").expect("E32: Exposure pasted");
        assert_eq!(
            held.get(control),
            Some(&value_of(control, value)),
            "E32: {control}={text} pasted"
        );
        assert_eq!(held.get("exposure"), Some(&value_of("exposure", 1.0)));
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    }
}

// E34 to E37: automatic mode.

/// A scene on the small raw's sensor, black 0, white 4000: a gradient of values from 100
/// to 900, each a multiple of 4, so the scene at 2^k for k from −2 to +1 is the same
/// scene exposed k EV apart, exactly, none clipping.
fn bracketed(k: i32) -> Vec<u16> {
    let (width, height) = (96, 72);
    (0..width * height)
        .map(|i| {
            let (x, y) = (i % width, i / width);
            let c = [0, 1, 1, 2][((y % 2) * 2 + x % 2) as usize];
            let level = 100 + 4 * ((150 * x / width + 50 * y / height + 10 * c) as u32);
            (f64::from(level) * 2f64.powi(k)) as u16
        })
        .collect()
}

/// Automatic mode at `percentile` and `target`, and these other controls.
fn automatic(percentile: f32, target: f32, more: &[(&str, f32)]) -> Edit {
    let mut set = vec![
        ("mode", 1.0),
        ("deflicker_percentile", percentile),
        ("deflicker_target_level", target),
    ];
    set.extend_from_slice(more);
    edit_of(&set)
}

/// The exposure automatic mode computes on `bench`'s raw, from the histogram its render
/// counted, as the exposure operation's `computed` gives it on rawprepare's levels.
fn computed(bench: &Bench, percentile: f32, target: f32) -> f32 {
    let histogram = bench.image.raw_histogram.as_ref().expect("counted");
    let base = Base::new(OPERATIONS, &bench.image);
    let blacks: f32 = apertrust_operations::rawprepare::BLACK
        .iter()
        .map(|name| base.params("rawprepare").unwrap().int(name) as f32)
        .sum();
    let white = base.params("rawprepare").unwrap().int("raw_white_point");
    histogram.exposure(
        percentile,
        target,
        (blacks / 4.0).round() as u32,
        white as u32,
    )
}

/// E34: one scene exposed 2 EV under to 1 EV over, in automatic mode, renders the same
/// at Exposure's output, black 0, so its exposures differ by exactly the brackets'.
#[test]
fn e34_a_series_of_one_scene_converges_in_automatic_mode() {
    let (_, image) = small_raw(-0.67);
    // Black level at the photo's own black's opposite: the black 0, so the scale is the
    // exposure's alone.
    let edit = automatic(50.0, -4.0, &[("black_level", 1.0 / 4096.0)]);
    let mut renders = Vec::new();
    for k in [-2, -1, 0, 1] {
        let mut bench = Bench::mosaic(&bracketed(k), image.clone(), Pipe::Export);
        let rendered = bench.edit(&edit, Measure::Tool);
        renders.push((computed(&bench, 50.0, -4.0), rendered));
    }
    let (base, first) = &renders[2];
    for ((exposure, rendered), k) in renders.iter().zip([-2.0, -1.0, 0.0, 1.0]) {
        // k EV apart, to the f32 the operation rounds each to.
        let apart = f64::from(*exposure) + k - f64::from(*base);
        assert!(
            apart.abs() < 1e-6,
            "E34: the computed exposure at {k} EV: {apart:e}"
        );
        // Exposure applies the same factor; demosaic, not scale-invariant to the last
        // bit, leaves a spread.
        let (spread, median) = factor(&first.pixels, &rendered.pixels);
        println!("E34 at {k:+} EV: {exposure} EV, spread {spread:.2e}, median {median:.7}");
        assert!(
            spread < 1e-3 && (median - 1.0).abs() < 1e-5,
            "E34 at {k} EV: spread {spread:.2e}, median {median}"
        );
    }
}

/// E35: automatic mode renders as manual mode at the exposure it computes, with the
/// camera's bias and the held-back exposure not added, darktable replacing them, at
/// percentiles 10, 50 and 90 and target levels −4 and −2.
#[test]
fn e35_automatic_mode_renders_as_manual_mode_at_its_exposure() {
    let viewport = Pipe::Viewport {
        width: 40,
        height: 30,
    };
    for pipe in [Pipe::Export, viewport] {
        let (mosaic, image) = small_raw(-0.67);
        let image = holding_back(image);
        let mut bench = Bench::mosaic(&mosaic, image, pipe);
        for percentile in [10.0, 50.0, 90.0] {
            for target in [-4.0, -2.0] {
                let auto = bench.edit(&automatic(percentile, target, &[]), Measure::Tool);
                let exposure = computed(&bench, percentile, target);
                let manual = edit_of(&[
                    ("exposure", exposure),
                    ("camera_bias", 0.0),
                    ("compensate_hilite_pres", 0.0),
                ]);
                let manual = bench.edit(&manual, Measure::Tool);
                assert!(
                    auto.pixels == manual.pixels,
                    "E35 in {pipe:?} at {percentile} %, {target} EV: {exposure} EV"
                );
            }
        }
    }
}

/// E36: on an image that is not a raw, which has no histogram, automatic mode renders as
/// manual mode, byte for byte, as darktable's `commit_params` leaves it.
#[test]
fn e36_automatic_mode_on_an_image_that_is_not_a_raw_renders_as_manual_mode() {
    let (picture, mut image) = ramp(0.0);
    image.camera = None;
    let mut bench = Bench::picture(&picture, image, Pipe::Export);
    let manual = bench.edit(&edit_of(&[("exposure", 1.0)]), Measure::Frame);
    let auto = bench.edit(&automatic(90.0, -2.0, &[("exposure", 1.0)]), Measure::Frame);
    assert!(exact(&manual, &auto), "E36");
}

/// E37: manual mode counts no histogram, so a photo in it costs nothing more: the raw's
/// histogram is counted only once automatic mode is set, then kept.
#[test]
fn e37_only_automatic_mode_counts_the_histogram_once() {
    let (mosaic, image) = small_raw(0.0);
    let mut bench = Bench::mosaic(&mosaic, image, Pipe::Export);
    bench.edit(&edit_of(&[("exposure", 1.0)]), Measure::Tool);
    assert!(bench.image.raw_histogram.is_none(), "E37: manual mode");
    bench.edit(&automatic(50.0, -4.0, &[]), Measure::Tool);
    let counted = bench
        .image
        .raw_histogram
        .clone()
        .expect("E37: automatic mode");
    // Over the declared image, 84×62.
    assert_eq!(counted.pixels(), 84 * 62);
    bench.edit(&automatic(90.0, -2.0, &[]), Measure::Tool);
    assert_eq!(bench.image.raw_histogram, Some(counted), "E37: kept");
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
        let output = bench.edit(&Edit::default(), Measure::Tool);
        for value in exposures() {
            let (spread, _) = exposure_factor(&mut bench, &output, value, Measure::Tool);
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
/// by `ResetControl` and by `ResetTool` on one pipeline whose shape changes on the
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

/// Contrast and its pivot are the Colour balance RGB tool's now (decision 2): Exposure
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

/// E36's image, one that is not a raw, lists manual alone (decision 88): an edit made on
/// a raw and pasted there keeps automatic, which renders as manual mode (E36), Manual can
/// be chosen, and automatic cannot be set there again.
#[test]
fn e36_a_pasted_automatic_mode_can_be_set_back_to_manual_and_not_again() {
    let mut plain = common::photo();
    plain.camera = None;
    assert_eq!(Exposure.options("mode", &plain), ["manual"]);
    assert_eq!(
        Exposure.options("mode", &common::photo()),
        ["manual", "automatic"]
    );
    let mode = |edit: &Edit| edit.tool("exposure").and_then(|v| v.get("mode")).cloned();
    let mut pasted = edit_of(&[("mode", 1.0)]);
    assert_eq!(mode(&pasted), Some(ControlValue::Choice(1)));
    let before = pasted.clone();
    assert_eq!(
        pasted.set_control(&Exposure, "mode", ControlValue::Choice(1), &plain),
        Err(edit::EditError::NotListed {
            tool: "exposure",
            control: "mode",
            option: "automatic",
        })
    );
    assert_eq!(pasted, before);
    pasted
        .set_control(&Exposure, "mode", ControlValue::Choice(0), &plain)
        .unwrap();
    assert_eq!(mode(&pasted), Some(ControlValue::Choice(0)));
}

/// Automatic mode's page in the board's words (decision 88), the tooltips of Measure at and
/// Place it at starting with darktable's name for their rows; Mode's Automatic saying why an image that is not a raw
/// cannot have it; Black level correction warning what it is not for.
#[test]
fn automatic_mode_s_page_has_the_board_s_words() {
    use apertrust_tools::messages;
    let words = apertrust_ui::Translator::new(apertrust_ui::Locale::EnGb);
    let text = |id: String| words.text(&id);
    for (control, label, darktable) in [
        ("deflicker_percentile", "Measure at", "Percentile"),
        ("deflicker_target_level", "Place it at", "Target level"),
    ] {
        assert_eq!(text(messages::label("exposure", control)), label);
        assert_eq!(text(messages::full("exposure", control)), label);
        let promise = text(messages::promise("exposure", control));
        assert!(promise.starts_with(darktable), "{promise}");
    }
    assert_eq!(
        text(messages::promise("exposure", "deflicker_percentile")),
        "Percentile: the point of the raw histogram measured; 50 % is the median"
    );
    assert_eq!(
        text(messages::promise("exposure", "deflicker_target_level")),
        "Target level: where that point lands, in EV below white"
    );
    let [label, promise] = messages::computed("exposure", "computed_exposure").map(text);
    assert_eq!(label, "This photo");
    assert_eq!(
        promise,
        "The exposure automatic mode computed for this photo from its histogram"
    );
    assert_eq!(
        text(messages::option_unavailable(
            "exposure",
            "mode",
            "automatic"
        )),
        "Automatic needs a raw photo's histogram"
    );
    assert_eq!(
        text(messages::promise("exposure", "black_level")),
        "For lifting clipped blacks, not for deeper blacks: use AgX's black relative exposure"
    );
}

/// The tool's defaults agree with the exposure operation's for every kind of image:
/// Compensate camera exposure as the operation's `compensate_exposure_bias`, Highlight
/// preservation mode as its `compensate_hilite_pres`, and the camera's bias and the
/// held-back exposure as the operation itself bounds them.
#[test]
fn its_defaults_and_bias_agree_with_the_exposure_operation() {
    let mut monochrome = raw_context(8, 8, -8.0);
    monochrome.camera.as_mut().unwrap().cfa = None;
    let held = |bias: f32, held: f32| {
        let mut image = raw_context(8, 8, bias);
        image.camera.as_mut().unwrap().highlight_preservation = held;
        image
    };
    for image in [
        raw_context(8, 8, 0.0),
        raw_context(8, 8, -8.0),
        raw_context(8, 8, 6.5),
        held(-0.67, 0.5),
        held(0.0, 6.0),
        held(0.0, -0.5),
        monochrome,
        ImageContext::full(8, 8),
    ] {
        let base = Base::new(OPERATIONS, &image);
        let compensated = base.flag("exposure", "compensate_exposure_bias");
        let given_back = base.flag("exposure", "compensate_hilite_pres");
        let defaults = Exposure.defaults(&image);
        assert_eq!(defaults.flag("camera_bias"), compensated);
        assert_eq!(defaults.flag("compensate_hilite_pres"), given_back);
        let params = base.params("exposure").unwrap();
        // To a rounding: the operation takes the bias off, then gives the rest back.
        let operation = apertrust_operations::exposure::effective_exposure(params, &image);
        let tool = params.float("exposure") - exposure::taken_off(&image, compensated, given_back);
        assert!(
            (operation - tool).abs() <= 1e-6,
            "{operation} for {tool} on {:?}",
            image.camera.as_ref().map(|c| c.exposure_bias)
        );
    }
}
