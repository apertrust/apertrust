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

//! Colour calibration's invariants (`docs/tools/colour-calibration.md`, Invariants), one
//! test per row, each named after its row. The synthetic ramp and sweep and a small raw
//! run on CI's software GPUs; the raw.pixls.us corpus runs locally, with
//! `APERTRUST_RAW_CORPUS` naming it:
//!
//! ```sh
//! cargo test --release -p apertrust-tools --test colour_calibration -- --ignored --nocapture
//! ```
//!
//! Synthetic images enter after demosaic, already in the working space and white
//! balanced: their rows plan the operations from exposure on and leave colorin out,
//! colour calibration adapting from D50 unless an edit sets its illuminant
//! (`common::Bench::keep`). Every row that renders reads the frame, colorout's output
//! before it is rounded to 8 bits.

#[expect(
    dead_code,
    reason = "Colour calibration's rows are exact: they read neither a ΔE2000 jump nor the extended ramp"
)]
mod common;
#[expect(
    dead_code,
    reason = "Colour calibration's corpus rows read each raw's plan, so they need no corpus bench"
)]
mod corpus;

use apertrust_color::Adaptation;
use apertrust_color::illuminants::{self, Illuminant, TEMPERATURE_RANGE};
use apertrust_color::xyz_to_lab_d50;
use apertrust_engine::edit::{self, Edit, Held};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe};
use apertrust_operations::{OPERATIONS, channelmixerrgb};
use apertrust_plugin::{ImageContext, ParamValues, Value};
use apertrust_tools::colour_calibration::{
    CAT, ColourCalibration, HUE_CHROMA, ILLUMINANT_VALUES, ILLUMINANTS, PARAMS,
};
use apertrust_tools::pick::{PickKeys, Picked, Reading, Stats};
use apertrust_tools::{Base, ControlKind, ControlValue, Controls, ProcessingTool, Row, TOOLS};
use golden::cases::CASES;

use common::{
    BEFORE_THE_WORKING_SPACE, Bench, Picture, Rendered, exact, id, raw_context, small_raw, sweep,
};
use corpus::each_corpus_raw;

const TOOL: &str = "channelmixerrgb";

/// What every rendering row reads: the frame, colorout's output, from exposure on for a
/// synthetic image.
#[derive(Debug, Clone, Copy)]
struct Frame;

impl common::Measure for Frame {
    fn keep(self, instances: &mut Vec<Instance<'static>>, synthetic: bool) {
        instances.retain(|i| !(synthetic && BEFORE_THE_WORKING_SPACE.contains(&id(i))));
    }
}

fn number(value: f32) -> ControlValue {
    ControlValue::Number(value)
}

const ON: ControlValue = ControlValue::Bool(true);
const OFF: ControlValue = ControlValue::Bool(false);

/// The option called `option` of the choice `control`.
fn choice(control: &str, option: &str) -> ControlValue {
    match ColourCalibration
        .definition()
        .control(control)
        .unwrap()
        .kind
    {
        ControlKind::Choice { options, .. } => {
            let index = options.iter().position(|&o| o == option).unwrap();
            ControlValue::Choice(index as u32)
        }
        other => panic!("{control} is {other:?}"),
    }
}

fn illuminant(option: &str) -> ControlValue {
    choice("illuminant", option)
}

/// Every control and the `channelmixerrgb` parameter it sets: the CAT tab's, each the
/// parameter of its name, then the mixer's.
fn params() -> impl Iterator<Item = (&'static str, &'static str)> {
    CAT.iter().map(|&control| (control, control)).chain(PARAMS)
}

/// An edit with these Colour calibration controls set, in this order, each value as it is.
fn edit_of(set: &[(&str, ControlValue)]) -> Edit {
    let mut edit = Edit::default();
    for (control, value) in set {
        edit.set_control(&ColourCalibration, control, value.clone(), &common::photo())
            .unwrap();
    }
    edit
}

/// `control` set to `value` on `image` as the session sets it: with its follow writes
/// (decision 27).
fn set_following(edit: &mut Edit, control: &str, value: ControlValue, image: &ImageContext) {
    let before = edit.clone();
    edit.set_control(&ColourCalibration, control, value, image)
        .unwrap();
    let set = edit::written(&ColourCalibration, control);
    edit.follow(&before, &ColourCalibration, set, image)
        .unwrap();
}

/// An edit with these controls set, in this order, as the session sets them.
fn followed(set: &[(&str, ControlValue)], image: &ImageContext) -> Edit {
    let mut edit = Edit::default();
    for (control, value) in set {
        set_following(&mut edit, control, value.clone(), image);
    }
    edit
}

/// Every control as `edit` leaves it on `image`: its value over its default for the photo.
fn every(edit: &Edit, image: &ImageContext) -> Controls {
    let definition = ColourCalibration.definition();
    let mut values = ColourCalibration.defaults(image);
    for (control, value) in edit.tool(TOOL).iter().flat_map(|set| set.values()) {
        values.set(definition.control(control).unwrap().id, value.clone());
    }
    values
}

/// The ramp, neutral greys from 2⁻¹⁶ to 2⁴, over a colour raw's context.
fn ramp() -> (Picture, ImageContext) {
    common::ramp(&[], 0.0)
}

/// What the operation renders an unedited `image` with, every parameter resolved.
fn operation_defaults(image: &ImageContext) -> ParamValues {
    let operation = apertrust_operations::operation(TOOL).unwrap();
    operation
        .schema()
        .resolve_over(&operation.defaults(image), &ParamValues::new())
        .unwrap()
}

/// A control's value as the parameter it sets holds it: an illuminant by its darktable
/// value.
fn as_param(control: &str, value: &ControlValue) -> Value {
    match value {
        ControlValue::Number(value) => Value::Float(*value),
        ControlValue::Bool(on) => Value::Bool(*on),
        ControlValue::Choice(index) if control == "illuminant" => {
            Value::Choice(ILLUMINANT_VALUES[*index as usize].to_darktable() as u32)
        }
        ControlValue::Choice(index) => Value::Choice(*index),
        other => panic!("no Colour calibration control holds {other:?}"),
    }
}

/// A parameter's value as the control setting it holds it: [`as_param`]'s inverse.
fn as_control(control: &str, value: Value) -> ControlValue {
    match value {
        Value::Float(value) => number(value),
        Value::Bool(on) => ControlValue::Bool(on),
        Value::Choice(value) if control == "illuminant" => {
            let listed = ILLUMINANT_VALUES
                .iter()
                .position(|i| i.to_darktable() as u32 == value)
                .unwrap();
            ControlValue::Choice(listed as u32)
        }
        Value::Choice(value) => ControlValue::Choice(value),
        other => panic!("{control} is {other:?}"),
    }
}

// C0: every control at its default.

/// Edits that leave every control at its default for `image`: each control written at
/// its default, and each kept there.
fn untouched(image: &ImageContext) -> Vec<Edit> {
    let controls = ColourCalibration.definition().controls;
    let defaults = ColourCalibration.defaults(image);
    let written: Vec<(&str, ControlValue)> = controls
        .iter()
        .map(|control| (control.id, defaults.get(control.id).unwrap().clone()))
        .collect();
    let mut kept = Edit::default();
    for control in controls {
        kept.keep(&ColourCalibration, control.id, true).unwrap();
    }
    vec![edit_of(&written), kept]
}

/// The tool's untouched values are the operation's own on `image`: As shot's light and
/// adaptation, the identity mixer, every weight 0, no normalisation, and the version it
/// leaves alone, darktable's 3.
fn defaults_are_the_operation_s(image: &ImageContext) {
    let resolved = operation_defaults(image);
    let controls = ColourCalibration.defaults(image);
    for (control, param) in params() {
        let default = as_param(control, controls.get(control).unwrap());
        assert_eq!(resolved.get(param), Some(default), "{control}");
    }
    assert_eq!(resolved.get("version"), Some(Value::Choice(2)));
}

/// C0 on one pipeline, as the session renders: no edit, then a moved control, then each
/// untouched edit, every frame as the first.
fn c0_holds(bench: &mut Bench) {
    defaults_are_the_operation_s(&bench.image);
    let none = bench.edit(&Edit::default(), Frame);
    bench.edit(&edit_of(&[("red_g", number(0.3))]), Frame);
    for edit in untouched(&bench.image) {
        let values = edit.tool(TOOL).unwrap();
        let base = Base::new(OPERATIONS, &bench.image);
        let off = values.off_defaults(&ColourCalibration.defaults(&bench.image));
        assert!(ColourCalibration.bind(&off, &bench.image, &base).is_empty());
        let at_defaults = bench.edit(&edit, Frame);
        assert!(exact(&none, &at_defaults), "C0 on {:?}", bench.pipe);
    }
}

#[test]
fn c0_every_control_at_its_default_renders_as_no_edit() {
    for pipe in [
        Pipe::Export,
        Pipe::Viewport {
            width: 40,
            height: 30,
        },
    ] {
        let (mosaic, image) = small_raw(0.0);
        c0_holds(&mut Bench::mosaic(&mosaic, image, pipe));
    }
    let (picture, image) = ramp();
    c0_holds(&mut Bench::picture(&picture, image, Pipe::Export));
    let (picture, image) = sweep(0.0);
    c0_holds(&mut Bench::picture(&picture, image, Pipe::Export));
}

// C1: each control moved, then reset.

/// A move on every mixer tab, each changing the frame: a weight alone, and with
/// Normalise channels, which with a row at its default changes nothing; then every mixer
/// control at once.
fn moves() -> Vec<Vec<(&'static str, ControlValue)>> {
    let mut moves = vec![
        vec![("red_g", number(0.3))],
        vec![("green_r", number(0.5)), ("green_normalise", ON)],
        vec![("blue_b", number(0.8))],
        vec![
            ("colourfulness_r", number(0.5)),
            ("colourfulness_b", number(-0.4)),
        ],
        vec![
            ("colourfulness_g", number(0.6)),
            ("colourfulness_normalise", ON),
        ],
        vec![("brightness_g", number(-0.3))],
        vec![("brightness_r", number(0.4)), ("brightness_normalise", ON)],
        vec![("grey_g", number(1.0))],
        vec![
            ("grey_r", number(0.3)),
            ("grey_b", number(0.3)),
            ("grey_normalise", ON),
        ],
    ];
    moves.push(
        PARAMS
            .iter()
            .map(|&(control, _)| match control {
                normalise if normalise.ends_with("_normalise") => (control, ON),
                _ => (control, number(0.2)),
            })
            .collect(),
    );
    moves
}

/// A move of every CAT control that leaves the light to its own controls, each changing
/// the small raw's frame, where the camera's own light is the default: the adaptation, a
/// custom light, gamut compression and clipping.
fn cat_moves() -> Vec<Vec<(&'static str, ControlValue)>> {
    vec![
        vec![("adaptation", choice("adaptation", "xyz"))],
        vec![("adaptation", choice("adaptation", "none"))],
        vec![("x", number(0.36)), ("y", number(0.37))],
        vec![("gamut", number(4.0))],
        vec![("gamut", number(0.0)), ("clip", OFF)],
    ]
}

/// A move of the illuminant to each one that describes another light, with a temperature,
/// an F source and an LED source: each writes the light's `x` and `y` as it moves (C6),
/// which its reset leaves. As shot in camera renders the camera's own light, the
/// default's, through CAT16, so it changes nothing on its own (C6 follows it).
fn light_moves() -> Vec<Vec<(&'static str, ControlValue)>> {
    vec![
        vec![("illuminant", illuminant("pipeline"))],
        vec![("illuminant", illuminant("a"))],
        vec![("illuminant", illuminant("d"))],
        vec![("illuminant", illuminant("e"))],
        vec![
            ("illuminant", illuminant("planckian")),
            ("temperature", number(3200.0)),
        ],
        vec![
            ("illuminant", illuminant("f")),
            ("illum_fluo", choice("illum_fluo", "f11")),
        ],
        vec![
            ("illuminant", illuminant("led")),
            ("illum_led", choice("illum_led", "b1")),
        ],
    ]
}

/// `control` reset on `image` as the session resets it: to its default, with its follow
/// writes (decision 27).
fn reset_following(edit: &mut Edit, control: &str, image: &ImageContext) {
    let before = edit.clone();
    edit.reset_control(&ColourCalibration, control).unwrap();
    let set = edit::written(&ColourCalibration, control);
    edit.follow(&before, &ColourCalibration, set, image)
        .unwrap();
}

/// Each move as the session makes it, then reset by `ResetTool` (the header's reset)
/// and by `ResetControl` (a double-click or Delete), on the bench's one pipeline: each
/// renders as no edit once the tool is reset, and, but for the light's moves, once its
/// controls are. A light's control reset keeps the light it wrote, as darktable's
/// `gui_changed` does where the illuminant's default is custom: the controls' resets
/// leave `x` and `y` holding it, kept, and the illuminant at its default, custom.
fn c1_holds(bench: &mut Bench, moves: Vec<Vec<(&'static str, ControlValue)>>, light: bool) {
    let image = bench.image.clone();
    let none = bench.edit(&Edit::default(), Frame);
    for set in moves {
        let mut edit = followed(&set, &image);
        let moved: Rendered = bench.edit(&edit, Frame);
        assert_ne!(none.pixels, moved.pixels, "{set:?} changes the frame");
        let written = every(&edit, &image);
        for (control, _) in &set {
            reset_following(&mut edit, control, &image);
        }
        if light {
            let kept = edit.tool(TOOL).unwrap();
            for control in ["x", "y"] {
                assert_eq!(
                    kept.get(control),
                    written.get(control),
                    "{set:?}: {control}"
                );
                assert_eq!(kept.mark(control), Some(apertrust_tools::Mark::Kept));
            }
            assert_eq!(kept.get("illuminant"), None, "{set:?}");
        } else {
            assert_eq!(edit, Edit::default(), "{set:?}");
            assert!(
                exact(&none, &bench.edit(&edit, Frame)),
                "{set:?}, reset control by control"
            );
        }
        let mut edit = followed(&set, &image);
        bench.edit(&edit, Frame);
        edit.reset_tool(&ColourCalibration);
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "{set:?}, the tool reset"
        );
    }
}

/// The small raw through the viewport, where the CAT tab's moves show.
fn small_raw_bench() -> Bench {
    let (mosaic, image) = small_raw(0.0);
    let viewport = Pipe::Viewport {
        width: 40,
        height: 30,
    };
    Bench::mosaic(&mosaic, image, viewport)
}

#[test]
fn c1_a_control_moved_and_reset_renders_as_no_edit() {
    let (picture, image) = sweep(0.0);
    c1_holds(
        &mut Bench::picture(&picture, image, Pipe::Export),
        moves(),
        false,
    );
    let mut every = moves();
    every.extend(cat_moves());
    c1_holds(&mut small_raw_bench(), every, false);
    c1_holds(&mut small_raw_bench(), light_moves(), true);
}

/// C1's bound for the light's moves, "as C0 once reset", control by control: it fails on
/// the small raw, whose As shot is custom, its light too far from both loci to snap,
/// since a reset of the illuminant keeps the light it wrote into `x` and `y`, so the photo
/// renders that light as a custom one, not As shot, as darktable's `gui_changed` does
/// where the illuminant's default is custom. On a photo whose As shot snaps, the
/// temperature reset too returns As shot
/// (`c1_on_a_photo_whose_as_shot_snaps_a_light_reset_with_its_temperature_returns_as_shot`).
/// Kept until the user decides (decision 30).
#[test]
#[ignore = "where As shot is custom, a reset of the illuminant keeps its light as custom (decision 30)"]
fn c1_a_light_moved_and_reset_control_by_control_renders_as_no_edit() {
    let mut bench = small_raw_bench();
    let image = bench.image.clone();
    let none = bench.edit(&Edit::default(), Frame);
    for set in light_moves() {
        let mut edit = followed(&set, &image);
        for (control, _) in &set {
            reset_following(&mut edit, control, &image);
        }
        assert!(exact(&none, &bench.edit(&edit, Frame)), "{set:?}");
    }
}

/// The Canon EOS 40D of darktable's test 0174 (darktable's matrix and the as-shot
/// multipliers) over the small raw's context: a photo whose As shot snaps, to daylight at
/// 3927.06 K, below 4000 K the Planckian.
fn snapping_raw() -> ImageContext {
    let mut image = raw_context(84, 62, 0.0);
    let camera = image.camera.as_mut().unwrap();
    camera.white_balance = Some(vec![2.115_234_4, 1.0, 1.398_437_5]);
    camera.xyz_to_camera = Some([
        [0.6071, -0.0747, -0.0856],
        [-0.7653, 1.5365, 0.2441],
        [-0.2025, 0.2553, 0.7315],
    ]);
    image
}

/// The light `edit` renders on `image`: its `channelmixerrgb` instance resolved over the
/// operation's defaults, as `prepare` takes it, the camera's at its reference.
fn rendered_light(edit: &Edit, image: &ImageContext) -> [f32; 2] {
    let instances = edit::instances(edit, OPERATIONS, TOOLS, image)
        .all()
        .unwrap();
    let instance = instances.iter().find(|i| id(i) == TOOL).unwrap();
    let operation = instance.operation;
    let params = operation
        .schema()
        .resolve_over(&operation.defaults(image), &instance.params)
        .unwrap();
    let camera = image.camera.as_ref().unwrap();
    let xyz_to_camera = camera.xyz_to_camera.unwrap();
    let wb = camera.white_balance.as_deref().unwrap();
    let white = channelmixerrgb::CameraWhite {
        xyz_to_camera,
        as_shot: [wb[0], wb[1], wb[2]],
        applied: apertrust_color::daylight_multipliers(&xyz_to_camera).map(|v| v as f32),
    };
    channelmixerrgb::illuminant_xy(&channelmixerrgb::settings(&params), Some(&white))
}

/// C1's light moves on a photo whose As shot snaps, on the CPU: each moves the light, and,
/// reset control by control with the temperature reset too, renders As shot's light
/// exactly, since daylight and Planckian read the temperature, not `x` and `y`. Reset
/// without the temperature, as C1's bound has it, only the moves that leave the
/// temperature at its default do (daylight, and Planckian with a temperature); the others'
/// follow writes set the temperature to their light's, which daylight then reads.
#[test]
fn c1_on_a_photo_whose_as_shot_snaps_a_light_reset_with_its_temperature_returns_as_shot() {
    let image = snapping_raw();
    let as_shot = rendered_light(&Edit::default(), &image);
    let defaults = ColourCalibration.defaults(&image);
    assert_eq!(defaults.get("illuminant"), Some(&illuminant("d")));
    let temperature = defaults.number("temperature");
    assert!((temperature - 3927.0596).abs() < 0.01, "{temperature}");
    assert_eq!(as_shot, illuminants::cct_to_xy_blackbody(temperature));
    for set in light_moves() {
        let mut edit = followed(&set, &image);
        let moved = rendered_light(&edit, &image);
        for (control, _) in &set {
            reset_following(&mut edit, control, &image);
        }
        let reset = rendered_light(&edit, &image);
        reset_following(&mut edit, "temperature", &image);
        let with_temperature = rendered_light(&edit, &image);
        println!(
            "{set:?}: moved {moved:?}, reset {reset:?}, with the temperature {with_temperature:?}"
        );
        assert_eq!(with_temperature, as_shot, "{set:?}");
    }
    println!("As shot {as_shot:?}");
}

// C2: the order controls were set in.

/// One pipeline sets `first`, then both; another sets `second`, then both, each as the
/// session sets it, with its follow writes: the two frames are the same, byte for byte.
fn c2_holds(first: (&str, ControlValue), second: (&str, ControlValue)) {
    let (picture, image) = sweep(0.0);
    let both = |a: &(&str, ControlValue), b: &(&str, ControlValue)| {
        followed(&[a.clone(), b.clone()], &image)
    };
    let mut one = Bench::picture(&picture, image.clone(), Pipe::Export);
    one.edit(&followed(std::slice::from_ref(&first), &image), Frame);
    let one = one.edit(&both(&first, &second), Frame);
    let mut other = Bench::picture(&picture, image.clone(), Pipe::Export);
    other.edit(&followed(std::slice::from_ref(&second), &image), Frame);
    let other = other.edit(&both(&second, &first), Frame);
    assert!(exact(&one, &other), "C2 with {first:?} and {second:?}");
}

#[test]
fn c2_two_controls_render_the_same_whichever_came_first() {
    c2_holds(("red_g", number(0.3)), ("colourfulness_b", number(0.5)));
    c2_holds(("grey_r", number(0.3)), ("grey_normalise", ON));
    c2_holds(("brightness_r", number(0.2)), ("green_b", number(-0.4)));
    // The CAT tab's controls of other quantities than the light, and the light with them.
    c2_holds(("illuminant", illuminant("d")), ("gamut", number(2.0)));
    c2_holds(("adaptation", choice("adaptation", "xyz")), ("clip", OFF));
    c2_holds(("illuminant", illuminant("f")), ("red_g", number(0.3)));
}

/// C3: background only: the tool never offers itself on a layer (Phase 4 adds
/// layers).
#[test]
fn c3_it_works_on_the_background_only() {
    let definition = ColourCalibration.definition();
    assert!(!definition.layers);
    assert!(!definition.auto);
}

/// C4: darktable's match. Every channelmixerrgb golden case's parameters, the CAT tab's
/// with the mixer, colourfulness, brightness, grey and normalisations, set as the tool's
/// controls, bind exactly the parameters darktable rendered the case with, so the cases'
/// ΔE2000 against darktable (`docs/ports/channelmixerrgb.md`, Results) is the
/// tool's. `version` is the 3 every case holds.
#[test]
fn c4_each_golden_case_s_values_bind_darktable_s_parameters() {
    let image = raw_context(8, 8, 0.0);
    let cases: Vec<_> = CASES.iter().filter(|case| case.operation == TOOL).collect();
    assert_eq!(cases.len(), 11);
    for case in cases {
        let theirs =
            channelmixerrgb::values(&golden::ports::channelmixerrgb_settings(case.xmp).unwrap());
        let set: Vec<(&str, ControlValue)> = params()
            .map(|(control, param)| (control, as_control(control, theirs.get(param).unwrap())))
            .collect();
        // Refused were a value outside its control's range, or an option it lacks.
        let edit = edit_of(&set);
        let instances = edit::instances(&edit, OPERATIONS, TOOLS, &image)
            .all()
            .unwrap();
        let instance = instances.iter().find(|i| id(i) == TOOL).unwrap();
        let operation = instance.operation;
        let ours = operation
            .schema()
            .resolve_over(&operation.defaults(&image), &instance.params)
            .unwrap();
        for param in params().map(|(_, param)| param).chain(["version"]) {
            assert_eq!(ours.get(param), theirs.get(param), "{}: {param}", case.name);
        }
    }
}

// C5: the CAT tab's view.

/// Hue and chroma as the view gives them for `x` and `y`, the rest at `image`'s defaults.
fn viewed(xy: [f32; 2], image: &ImageContext) -> [f32; 2] {
    let mut values = ColourCalibration.defaults(image);
    values.set("x", number(xy[0]));
    values.set("y", number(xy[1]));
    let coordinates = ColourCalibration.view(HUE_CHROMA, &values, image).unwrap();
    [coordinates[0], coordinates[1]]
}

/// `x` and `y` as the view's inverse gives them for a hue and a chroma.
fn unviewed(coordinates: [f32; 2], image: &ImageContext) -> [f32; 2] {
    let values = ColourCalibration
        .unview(HUE_CHROMA, &coordinates, image)
        .unwrap();
    ["x", "y"].map(|control| match values.get(control) {
        Some(ControlValue::Number(value)) => *value,
        other => panic!("{control} is {other:?}"),
    })
}

/// How far apart two hues are, in degrees, round the circle.
fn hue_apart(a: f32, b: f32) -> f64 {
    let difference = (f64::from(a) - f64::from(b)).rem_euclid(360.0);
    difference.min(360.0 - difference)
}

/// The worst round trips of the view over its coordinates' ranges, a grid of hue by 0.5°
/// and chroma by 0.5 %: the hue's error in degrees where the chroma is at least `chroma`,
/// the chroma's, and the hue's as an arc, in chroma's units, everywhere.
fn round_trips(chroma: f32) -> (f64, f64, f64) {
    let image = raw_context(8, 8, 0.0);
    let (mut hue, mut radius, mut arc) = (0.0f64, 0.0f64, 0.0f64);
    for i in 0..=720 {
        for j in 0..=600 {
            let coordinates = [i as f32 / 2.0, j as f32 / 2.0];
            let [h, c] = viewed(unviewed(coordinates, &image), &image);
            radius = radius.max((f64::from(c) - f64::from(coordinates[1])).abs());
            if coordinates[1] > 0.0 {
                let apart = hue_apart(h, coordinates[0]);
                arc = arc.max(apart.to_radians() * f64::from(coordinates[1]));
                if coordinates[1] >= chroma {
                    hue = hue.max(apart);
                }
            }
        }
    }
    (hue, radius, arc)
}

/// C5: hue and chroma, a lossless view of `x` and `y`. Every coordinate in its range gives
/// `x` and `y` in theirs (the builtin grid, `tests/builtin.rs`), and the view round-trips:
/// chroma to 1e-4 % and the hue's arc to 1e-4 % everywhere, the hue to 1e-4° from a chroma
/// of 12 %, `x` and `y` to a few `f32` units.
#[test]
fn c5_hue_and_chroma_round_trip_over_their_ranges() {
    let (hue, chroma, arc) = round_trips(12.0);
    println!("C5: hue {hue:.2e}° from chroma 12 %, chroma {chroma:.2e} %, arc {arc:.2e} %");
    assert!(hue < 1e-4 && chroma < 1e-4 && arc < 1e-4);
    // x and y through the view and back, over the light every coordinate reaches.
    let image = raw_context(8, 8, 0.0);
    let mut worst = 0.0f32;
    for i in 0..=360 {
        for j in 0..=100 {
            let xy = unviewed([i as f32, j as f32 * 3.0], &image);
            let back = unviewed(viewed(xy, &image), &image);
            let units =
                |a: f32, b: f32| (a - b).abs() / a.abs().max(b.abs()).max(0.1) / f32::EPSILON;
            worst = worst.max(units(xy[0], back[0]).max(units(xy[1], back[1])));
        }
    }
    println!("C5: x and y back within {worst:.1} f32 units");
    assert!(worst <= 4.0, "{worst}");
}

/// C5's proposed bound on the hue, 1e-4°, everywhere a hue has a meaning: it fails at low
/// chroma, where the rounding of `x` and `y` to `f32` moves the light by an arc of up to
/// 4.8e-5 % in any direction, a large angle over a short radius: 1.1e-3° at a chroma of
/// 1 %, 2.2e-3° at 0.5 %, so the hue round-trips to 1e-4° only from a chroma of 12 %. Kept
/// until the user decides (decision 30): the arc, which
/// `c5_hue_and_chroma_round_trip_over_their_ranges` bounds, is the measure that holds at
/// every chroma.
#[test]
#[ignore = "C5's proposed 1e-4° on the hue fails below a chroma of 12 % (decision 30)"]
fn c5_the_hue_round_trips_to_1e_4_degrees_at_every_chroma() {
    let (hue, _, _) = round_trips(f32::MIN_POSITIVE);
    assert!(hue < 1e-4, "{hue}");
}

// C6: follow writes.

/// darktable's parameters for the CAT tab's light, as `gui_changed` and
/// `_illum_xy_callback` read and write them (`src/iop/channelmixerrgb.c`).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Light {
    illuminant: Illuminant,
    fluorescent: usize,
    led: usize,
    adaptation: Adaptation,
    x: f32,
    y: f32,
    temperature: f32,
}

impl Light {
    fn of(values: &Controls) -> Light {
        Light {
            illuminant: ILLUMINANT_VALUES[values.choice("illuminant") as usize],
            fluorescent: values.choice("illum_fluo") as usize,
            led: values.choice("illum_led") as usize,
            adaptation: Adaptation::from_darktable(values.choice("adaptation") as i32).unwrap(),
            x: values.number("x"),
            y: values.number("y"),
            temperature: values.number("temperature"),
        }
    }
}

/// The widget a change came from, darktable's `w` in `gui_changed`, with the illuminant's
/// `previous`.
#[derive(Debug, Clone, Copy)]
enum Widget {
    Illuminant {
        previous: Illuminant,
    },
    Fluorescent,
    Led,
    Temperature,
    /// Hue or chroma, `_illum_xy_callback`, once `x` and `y` hold the light they give.
    HueChroma,
}

/// darktable's `find_temperature_from_raw_coeffs` at the camera's reference, `custom_wb`
/// 1, where the raw stage leaves the image.
fn camera(image: &ImageContext) -> Option<[f32; 2]> {
    let camera = image.camera.as_ref()?;
    let wb = camera.white_balance.as_deref()?;
    illuminants::camera_illuminant_xy(&camera.xyz_to_camera?, [wb[0], wb[1], wb[2]])
}

/// darktable's `_check_if_close_to_daylight` for the temperature alone, held to the
/// parameter's range as the tool holds it.
fn close_to_daylight(x: f32, y: f32) -> f32 {
    let (min, max) = TEMPERATURE_RANGE;
    illuminants::closest_cct_model(x, y).0.clamp(min, max)
}

/// darktable's `gui_changed` for the CAT tab's widgets and `_illum_xy_callback`, restated
/// on its parameters for `image`, but for decision 27's differences: its memory of the
/// last daylight and Planckian temperatures is the stored temperature, and a temperature
/// it stores is held to the parameter's range.
fn gui_changed(p: &mut Light, w: Widget, image: &ImageContext) {
    if let Widget::Illuminant { previous } = w {
        if previous == Illuminant::Camera {
            if let Some([x, y]) = camera(image) {
                (p.x, p.y) = (x, y);
            }
            p.temperature = close_to_daylight(p.x, p.y);
            p.adaptation = Adaptation::Cat16;
        }
        // `p->temperature = g->last_daylight_temperature` and `last_bb_temperature`.
        if p.illuminant == Illuminant::Camera {
            if let Some([x, y]) = camera(image) {
                (p.x, p.y) = (x, y);
            }
            p.temperature = close_to_daylight(p.x, p.y);
            p.adaptation = Adaptation::Cat16;
        }
    }
    if !matches!(w, Widget::HueChroma) {
        if p.illuminant != Illuminant::Custom && p.illuminant != Illuminant::Camera {
            let light = illuminants::illuminant_to_xy(
                p.illuminant,
                p.temperature,
                p.fluorescent,
                p.led,
                None,
            );
            if let Some([x, y]) = light {
                (p.x, p.y) = (x, y);
            }
        }
        if p.illuminant != Illuminant::D
            && p.illuminant != Illuminant::BlackBody
            && p.illuminant != Illuminant::Camera
        {
            p.temperature = close_to_daylight(p.x, p.y);
        }
    } else {
        let mut t = illuminants::xy_to_cct(p.x, p.y);
        if t < 3000.0 {
            t = illuminants::cct_reverse_lookup(p.x, p.y);
        }
        let (min, max) = TEMPERATURE_RANGE;
        p.temperature = t.clamp(min, max);
    }
}

/// C6: each change of the light, through the tool's follow writes as the session
/// applies them, against darktable's `gui_changed` restated: every illuminant from every
/// other, from the defaults, an F and an LED source, a temperature on daylight and on
/// Planckian, hues and chromas through both of darktable's temperatures, the fit and the
/// reverse lookup, as shot in camera chosen and left. The control a change set is never
/// rewritten, and every write is kept, as the change's own. The session has no history
/// yet: a change and its follow writes are one change, rendered once and written once
/// (`a_control_s_follow_writes_are_one_change_with_it` in `crates/engine/src/session.rs`).
#[test]
fn c6_follow_writes_are_darktable_s_gui_changed() {
    let image = raw_context(8, 8, 0.0);
    let defaults = Light::of(&ColourCalibration.defaults(&image));
    let ours = |edit: &Edit| Light::of(&every(edit, &image));
    let mut transitions = 0;
    for from in ILLUMINANTS {
        for to in ILLUMINANTS {
            let mut edit = Edit::default();
            let mut theirs = defaults;
            for option in [from, to] {
                set_following(&mut edit, "illuminant", illuminant(option), &image);
                // darktable's combobox calls `gui_changed` only for another illuminant.
                let chosen =
                    ILLUMINANT_VALUES[ILLUMINANTS.iter().position(|&o| o == option).unwrap()];
                let previous = std::mem::replace(&mut theirs.illuminant, chosen);
                if previous != chosen {
                    gui_changed(&mut theirs, Widget::Illuminant { previous }, &image);
                }
                assert_eq!(ours(&edit), theirs, "{from} then {to}, at {option}");
            }
            transitions += 1;
        }
    }
    assert_eq!(transitions, 81);

    // An F source, an LED source, and a temperature on daylight and on Planckian.
    for (mode, control, value, widget) in [
        (
            "f",
            "illum_fluo",
            choice("illum_fluo", "f7"),
            Widget::Fluorescent,
        ),
        ("led", "illum_led", choice("illum_led", "v1"), Widget::Led),
        ("d", "temperature", number(6500.0), Widget::Temperature),
        (
            "planckian",
            "temperature",
            number(2700.0),
            Widget::Temperature,
        ),
    ] {
        let mut edit = followed(&[("illuminant", illuminant(mode))], &image);
        let mut theirs = ours(&edit);
        set_following(&mut edit, control, value.clone(), &image);
        match (control, &value) {
            ("illum_fluo", ControlValue::Choice(i)) => theirs.fluorescent = *i as usize,
            ("illum_led", ControlValue::Choice(i)) => theirs.led = *i as usize,
            (_, ControlValue::Number(t)) => theirs.temperature = *t,
            other => panic!("{other:?}"),
        }
        gui_changed(&mut theirs, widget, &image);
        assert_eq!(ours(&edit), theirs, "{control} on {mode}");
    }

    // A hue, then a chroma, on a custom light: x and y as the view gives them, and the
    // temperature `_illum_xy_callback` computes from them, by `xy_to_CCT` from 3000 K and
    // by `CCT_reverse_lookup` below: at hue 40° and chroma 80 % the fit gives 2758 K, and
    // at hue 0° and chroma 150 % 1249 K, below 1667 K, where the lookup gives 2794 K.
    let mut edit = followed(&[("illuminant", illuminant("d"))], &image);
    let mut held = Held::default();
    for (coordinate, value) in [
        ("hue", 200.0),
        ("chroma", 35.0),
        ("chroma", 0.5),
        ("hue", 40.0),
        ("chroma", 80.0),
        ("hue", 0.0),
        ("chroma", 150.0),
    ] {
        let before = edit.clone();
        edit.set_coordinate(
            &mut held,
            &ColourCalibration,
            coordinate,
            number(value),
            &image,
        )
        .unwrap();
        let set = edit::written(&ColourCalibration, coordinate);
        edit.follow(&before, &ColourCalibration, set, &image)
            .unwrap();
        let mut theirs = ours(&before);
        let now = ours(&edit);
        (theirs.x, theirs.y) = (now.x, now.y);
        gui_changed(&mut theirs, Widget::HueChroma, &image);
        assert_eq!(now, theirs, "{coordinate} {value}");
    }
    let far = ours(&edit);
    assert!(illuminants::xy_to_cct(far.x, far.y) <= 1667.0);
    assert!(
        (far.temperature - 2794.2).abs() < 0.1,
        "{}",
        far.temperature
    );

    // The same light, a script's x and y under daylight, then chosen as custom: choosing
    // takes `_check_if_close_to_daylight`'s temperature, which keeps the fit below 1667 K,
    // held to the range, where `_illum_xy_callback` looked it up.
    let mut script = followed(&[("illuminant", illuminant("d"))], &image);
    let mut theirs = ours(&script);
    for (control, value) in [("x", far.x), ("y", far.y)] {
        set_following(&mut script, control, number(value), &image);
        if control == "x" {
            theirs.x = value;
        } else {
            theirs.y = value;
        }
        gui_changed(&mut theirs, Widget::HueChroma, &image);
        assert_eq!(ours(&script), theirs, "{control}");
    }
    set_following(&mut script, "illuminant", illuminant("custom"), &image);
    theirs.illuminant = Illuminant::Custom;
    let previous = Illuminant::D;
    gui_changed(&mut theirs, Widget::Illuminant { previous }, &image);
    assert_eq!(ours(&script), theirs);
    assert_eq!(theirs.temperature, 1667.0);

    // As shot in camera forces CAT16 and the camera's light, whatever was set before, and
    // leaving it for custom keeps that light, as darktable's does.
    let edit = followed(
        &[
            ("adaptation", choice("adaptation", "xyz")),
            ("illuminant", illuminant("d")),
            ("illuminant", illuminant("camera")),
        ],
        &image,
    );
    let shot = ours(&edit);
    assert_eq!(shot.adaptation, Adaptation::Cat16);
    assert_eq!(Some([shot.x, shot.y]), camera(&image));
    let mut left = edit.clone();
    set_following(&mut left, "illuminant", illuminant("custom"), &image);
    assert_eq!([ours(&left).x, ours(&left).y], [shot.x, shot.y]);

    // The control a change set is never rewritten: a temperature on a custom light, which
    // only a script reaches, stays as set, where darktable's `gui_changed` would put the
    // light's own back.
    let mut edit = Edit::default();
    set_following(&mut edit, "temperature", number(6000.0), &image);
    assert_eq!(ours(&edit).temperature, 6000.0);
    assert_eq!(ours(&edit).illuminant, Illuminant::Custom);

    // Every write is kept, as the change's own: choosing daylight keeps x and y.
    let edit = followed(&[("illuminant", illuminant("d"))], &image);
    let values = edit.tool(TOOL).unwrap();
    for control in ["illuminant", "x", "y"] {
        assert_eq!(
            values.mark(control),
            Some(apertrust_tools::Mark::Kept),
            "{control}"
        );
    }
}

/// C6 with C0: a hue moved then reset, as the session does both, leaves the edit
/// untouched. The reset takes `x` and `y` back to their defaults, and the temperature their
/// follow write gives is its default for the photo, so that write resets it too, value and
/// mark, rather than keeping it.
#[test]
fn c6_a_hue_moved_and_reset_leaves_the_edit_untouched() {
    let image = raw_context(8, 8, 0.0);
    let (mut edit, mut held) = (Edit::default(), Held::default());
    let set = edit::written(&ColourCalibration, "hue");
    let before = edit.clone();
    edit.set_coordinate(&mut held, &ColourCalibration, "hue", number(200.0), &image)
        .unwrap();
    edit.follow(&before, &ColourCalibration, set, &image)
        .unwrap();
    let moved = edit.tool(TOOL).unwrap();
    assert!(moved.get("temperature").is_some());
    let before = edit.clone();
    edit.reset_coordinate(&mut held, &ColourCalibration, "hue", &image)
        .unwrap();
    edit.follow(&before, &ColourCalibration, set, &image)
        .unwrap();
    assert_eq!(edit, Edit::default());
}

// C7: the CAT tab's rows.

/// The controls and coordinates the CAT tab shows while the tool holds `values`, in the
/// panel's order: rows following a control shown only for the values they follow.
fn shown(values: &Controls) -> Vec<&'static str> {
    fn walk(rows: &'static [Row], values: &Controls, out: &mut Vec<&'static str>) {
        let definition = ColourCalibration.definition();
        for row in rows {
            match row {
                Row::When { control, is, rows } => {
                    let found = definition.control(control).unwrap();
                    if found.is_one_of(values.get(control).unwrap(), is) {
                        walk(rows, values, out);
                    }
                }
                Row::Group(_) => {}
                // The readout row, darktable's CCT, the light's colour and the picker.
                Row::Line { .. } => out.push("readout"),
                row => out.extend(row.controls()),
            }
        }
    }
    let mut out = Vec::new();
    walk(
        ColourCalibration.definition().tool.tabs[0].rows,
        values,
        &mut out,
    );
    out
}

/// C7: the CAT tab's rows as darktable's `_update_illuminants` shows them, for every
/// adaptation and illuminant: with none, only the adaptation, gamut compression and
/// clipping; otherwise the readout row and the illuminant too, with the F source for F, the LED source for
/// LED, the temperature for daylight and Planckian, hue and chroma for custom, and nothing
/// more for the others. The tab is darktable's first, the area colour mapping its last row.
/// The adaptation is unavailable while the illuminant is as shot in camera, which forces
/// CAT16, as `gui_changed` makes it insensitive, and every other control is available with
/// every illuminant (decision 30).
#[test]
fn c7_the_cat_tab_shows_its_rows_as_darktable_does() {
    let definition = ColourCalibration.definition();
    assert_eq!(definition.tool.tabs[0].id, Some("cat"));
    assert!(matches!(
        definition.tool.tabs[0].rows.last(),
        Some(Row::Group(apertrust_tools::Group {
            id: "area_colour_mapping",
            kind: apertrust_tools::GroupKind::Collapsible,
            ..
        }))
    ));
    let image = raw_context(8, 8, 0.0);
    for adaptation in [
        "linear_bradford",
        "cat16",
        "non_linear_bradford",
        "xyz",
        "none",
    ] {
        for option in ILLUMINANTS {
            let mut values = ColourCalibration.defaults(&image);
            values.set("adaptation", choice("adaptation", adaptation));
            values.set("illuminant", illuminant(option));
            let mut expected = vec!["adaptation"];
            if adaptation != "none" {
                expected.extend(["readout", "illuminant"]);
                expected.extend(match option {
                    "f" => &["illum_fluo"][..],
                    "led" => &["illum_led"],
                    "d" | "planckian" => &["temperature"],
                    "custom" => &["hue", "chroma"],
                    _ => &[],
                });
            }
            expected.extend(["gamut", "clip"]);
            assert_eq!(shown(&values), expected, "{adaptation}, {option}");
            for control in definition.controls {
                let available = ColourCalibration.available(control.id, &values, &image);
                let insensitive = control.id == "adaptation" && option == "camera";
                assert_eq!(
                    available, !insensitive,
                    "{}: {adaptation}, {option}",
                    control.id
                );
            }
        }
    }
}

/// C7 along changes made as the session makes them, which asks `available` after each:
/// choosing as shot in camera makes the adaptation unavailable, a value set then is still
/// taken, and leaving it makes the adaptation available again. The session's report is
/// `a_control_s_follow_writes_are_one_change_with_it` in `crates/engine/src/session.rs`.
#[test]
fn c7_the_adaptation_is_unavailable_while_as_shot_in_camera_is_chosen() {
    let image = raw_context(8, 8, 0.0);
    let available =
        |edit: &Edit| ColourCalibration.available("adaptation", &every(edit, &image), &image);
    let mut edit = Edit::default();
    assert!(available(&edit));
    set_following(&mut edit, "illuminant", illuminant("camera"), &image);
    assert!(!available(&edit));
    // A script may still set it: it takes the value, which the camera's light ignores.
    set_following(&mut edit, "adaptation", choice("adaptation", "xyz"), &image);
    assert_eq!(every(&edit, &image).choice("adaptation"), 3);
    assert!(!available(&edit));
    set_following(&mut edit, "illuminant", illuminant("d"), &image);
    assert!(available(&edit));
}

// C8: the picker.

/// What C8 reads: colour calibration alone, on a synthetic image in the working space, so
/// the patch is the operation's input and the output is its own.
#[derive(Debug, Clone, Copy)]
struct Mixer;

impl common::Measure for Mixer {
    fn keep(self, instances: &mut Vec<Instance<'static>>, _: bool) {
        instances.retain(|i| id(i) == TOOL);
    }
}

/// The edit of these controls rendered by colour calibration alone, its illuminant custom.
/// The synthetic raw's As shot is a custom light, so an edit choosing custom leaves the
/// illuminant at its default, unbound, which the bench would take as the pipeline's own:
/// it is written here.
fn custom(bench: &mut Bench, set: &[(&'static str, ControlValue)]) -> Rendered {
    let mut instances = bench.instances(&edit_of(set), Mixer);
    for instance in &mut instances {
        let custom = Illuminant::Custom.to_darktable() as u32;
        instance.params.set("illuminant", Value::Choice(custom));
    }
    bench.render(&instances)
}

/// A flat patch's reading of working RGB `rgb`.
fn flat(rgb: [f32; 3]) -> Reading {
    let mean = [rgb[0], rgb[1], rgb[2], 1.0];
    Reading {
        input: Stats {
            mean,
            min: mean,
            max: mean,
        },
        output: None,
        input_maximum: [1.0; 3],
    }
}

/// Colour calibration's pick of `rgb` with `values` and `settings`.
fn pick(
    rgb: [f32; 3],
    values: &Controls,
    image: &ImageContext,
    settings: &[(&'static str, ControlValue)],
) -> Picked {
    ColourCalibration
        .pick(
            "illuminant",
            &flat(rgb),
            values,
            image,
            settings,
            None,
            None,
            PickKeys::default(),
        )
        .unwrap()
}

/// Working RGB as CIE Lab D50, through the working profile.
fn lab(rgb: [f32; 4]) -> [f32; 3] {
    let profile = common::profile();
    xyz_to_lab_d50(apertrust_color::apply(
        &profile.rgb_to_xyz_d50,
        [rgb[0], rgb[1], rgb[2]],
    ))
}

/// LCh of Lab, h in degrees from 0 to 360.
fn lch([l, a, b]: [f32; 3]) -> [f32; 3] {
    [l, a.hypot(b), b.atan2(a).to_degrees().rem_euclid(360.0)]
}

/// The area colour mapping's target settings, L, C and h.
fn target(mode: &str, [l, c, h]: [f32; 3]) -> Vec<(&'static str, ControlValue)> {
    vec![
        (
            "area_mode",
            ControlValue::Choice(u32::from(mode == "measure")),
        ),
        ("channel_mixing", ON),
        ("target_lightness", number(l)),
        ("target_chroma", number(c)),
        ("target_hue", number(h)),
    ]
}

/// C8's mix: a little of each channel into its neighbour, rows that do not sum to 1.
fn c8_mix() -> Vec<(&'static str, ControlValue)> {
    vec![
        ("red_r", number(1.1)),
        ("red_g", number(-0.05)),
        ("green_b", number(0.1)),
        ("blue_r", number(0.05)),
        ("blue_b", number(0.95)),
    ]
}

/// One of C8's picks: the target, the adaptation, the patch as it renders after the
/// correction pick, as LCh, its ΔE2000 from the target, and what a measure pick of it
/// then hands back, as LCh.
struct C8 {
    target: [f32; 3],
    adaptation: &'static str,
    rendered: [f32; 3],
    off: f64,
    measured: [f32; 3],
}

/// C8's targets, (50, 0, 0°) and (60, 20, 40°), each picked on a flat patch of a brown at
/// its lightness under each adaptation with the mixer `mix`, with Take channel mixing into
/// account, gamut compression and clipping at their defaults: the correction pick
/// rendered by colour calibration alone, then a measure pick of the same patch with the
/// light it wrote.
fn c8_picks(mix: &[(&'static str, ControlValue)]) -> Vec<C8> {
    let image = raw_context(4, 1, 0.0);
    let mut picks = Vec::new();
    for target_lch in [[50.0, 0.0, 0.0], [60.0, 20.0, 40.0]] {
        // A light changes an area's chromaticity alone, keeping its luminance, so the
        // patch is at its target's lightness.
        let y = apertrust_color::lab_to_xyz_d50([target_lch[0], 0.0, 0.0])[1];
        let base = [0.3, 0.2, 0.1];
        let scale = y / apertrust_color::apply(&common::profile().rgb_to_xyz_d50, base)[1];
        let rgb = base.map(|c| c * scale);
        let (picture, _) = Picture::row(vec![[rgb[0], rgb[1], rgb[2], 1.0]; 4], 0.0);
        let mut bench = Bench::picture(&picture, image.clone(), Pipe::Export);
        for adaptation in ["linear_bradford", "cat16", "non_linear_bradford", "xyz"] {
            let mut set = mix.to_vec();
            set.push(("adaptation", choice("adaptation", adaptation)));
            let mut values = ColourCalibration.defaults(&image);
            for (control, value) in &set {
                values.set(control, value.clone());
            }
            let picked = pick(rgb, &values, &image, &target("correction", target_lch));
            let mut written: Vec<&str> = picked.writes.iter().map(|(c, _)| c).collect();
            written.sort_unstable();
            assert_eq!(written, ["illuminant", "temperature", "x", "y"]);
            for (control, value) in picked.writes.iter() {
                set.push((control, value.clone()));
                values.set(control, value.clone());
            }
            let output = custom(&mut bench, &set);
            let got = lab(output.pixels[0]);
            let want = lab_to_xyz_lab(target_lch);
            let off = golden::delta_e::ciede2000(want.map(f64::from), got.map(f64::from));
            let measured = pick(rgb, &values, &image, &target("measure", target_lch));
            assert!(measured.writes.is_empty());
            let measured = ["target_lightness", "target_chroma", "target_hue"].map(|id| {
                match measured.shown.settings.iter().find(|(s, _)| *s == id) {
                    Some((_, ControlValue::Number(n))) => *n,
                    other => panic!("{id}: {other:?}"),
                }
            });
            picks.push(C8 {
                target: target_lch,
                adaptation,
                rendered: lch(got),
                off,
                measured,
            });
        }
    }
    picks
}

/// How far apart two LCh are in L, C and h, h only where both have a hue; each must be
/// finite, so a measurement that is not a number never passes as a match.
fn lch_apart(a: [f32; 3], b: [f32; 3]) -> f32 {
    assert!(a.iter().chain(&b).all(|v| v.is_finite()), "{a:?} {b:?}");
    let hue = if a[1] > 1.0 && b[1] > 1.0 {
        let d = (a[2] - b[2]).rem_euclid(360.0);
        d.min(360.0 - d)
    } else {
        0.0
    };
    (a[0] - b[0]).abs().max((a[1] - b[1]).abs()).max(hue)
}

/// C8's normalised mix: [`c8_mix`] with Normalise channels on each row, so each sums to 1.
fn c8_normalised() -> Vec<(&'static str, ControlValue)> {
    let mut normalised = c8_mix();
    for row in ["red_normalise", "green_normalise", "blue_normalise"] {
        normalised.push((row, ON));
    }
    normalised
}

/// C8: a correction pick of a flat patch of colour makes the patch at colour calibration's
/// output, read as Lab D50 through the working profile, land on the target within ΔE2000
/// 0.5 (proposed), for targets (50, 0, 0°) and (60, 20, 40°), with the identity mixer under
/// each adaptation; and a measure pick of the patch with the illuminant it wrote gives back
/// what the patch renders at within 0.1 in L, C and h, h only where it has a hue. Under
/// both Bradfords that is not the target itself: darktable's correction normalises the
/// target and the area to Y 1 but not the light, and leaves out non-linear Bradford's
/// power, so the patch lands, and measures, where darktable's arithmetic puts it.
#[test]
fn c8_a_correction_pick_maps_a_patch_to_the_target() {
    let (mut worst, mut back) = (0.0_f64, 0.0_f32);
    for C8 {
        target,
        adaptation,
        rendered,
        off,
        measured,
    } in c8_picks(&[])
    {
        println!("C8 under {adaptation}: {rendered:?} for {target:?}, ΔE2000 {off:.4}");
        worst = worst.max(off);
        assert!(
            off <= 0.5,
            "C8: {target:?} under {adaptation}: ΔE2000 {off:.4}, {rendered:?}"
        );
        let apart = lch_apart(measured, rendered);
        back = back.max(apart);
        assert!(
            apart <= 0.1,
            "C8 measured {measured:?}, rendered {rendered:?}"
        );
    }
    println!(
        "C8: at most ΔE2000 {worst:.4} from the target; measured within {back:.2e} of the render"
    );
}

/// C8 with a mix, plain and with Normalise channels on each row: a mix changes the area's
/// luminance, as no light can give back, so darktable's correction maps the chromaticity
/// alone. The patch lands on the target's chroma and hue at the lightness the mix gives it,
/// within ΔE2000 0.5 there, under each adaptation, and a measure pick gives back what it
/// renders at within 0.1.
#[test]
fn c8_a_mix_keeps_the_target_s_chromaticity() {
    let (mut worst, mut back, mut shifted) = (0.0_f64, 0.0_f32, 0.0_f32);
    let picks = [c8_mix(), c8_normalised()].map(|mix| c8_picks(&mix));
    for pick in picks.into_iter().flatten() {
        let C8 {
            target,
            adaptation,
            rendered,
            measured,
            ..
        } = pick;
        let at_its_lightness = lab_to_xyz_lab([rendered[0], target[1], target[2]]);
        let got = lab_to_xyz_lab(rendered);
        let off = golden::delta_e::ciede2000(at_its_lightness.map(f64::from), got.map(f64::from));
        worst = worst.max(off);
        shifted = shifted.max((rendered[0] - target[0]).abs());
        assert!(
            off <= 0.5,
            "{target:?} under {adaptation}: {rendered:?}, ΔE2000 {off:.4}"
        );
        let apart = lch_apart(measured, rendered);
        back = back.max(apart);
        assert!(apart <= 0.1, "measured {measured:?}, rendered {rendered:?}");
    }
    println!(
        "C8 with a mix: at most ΔE2000 {worst:.4} from the target's chroma and hue, L moved \
         by up to {shifted:.2}; measured within {back:.2e} of the render"
    );
}

/// The Lab of LCh `lch`, h in degrees.
fn lab_to_xyz_lab([l, c, h]: [f32; 3]) -> [f32; 3] {
    let h = h.to_radians();
    [l, c * h.cos(), c * h.sin()]
}

/// `chroma_adapt_pixel`, which the picker's measure mode adapts an area by, against the
/// operation's kernel for the same light: a few colours through colour calibration alone,
/// a custom illuminant under each adaptation, the identity mixer, no gamut compression
/// and no clipping, the kernel's output read as XYZ through the working profile, within
/// 1e-4 of the largest channel.
#[test]
fn c8_chroma_adapt_pixel_is_the_kernel_s_adaptation() {
    use apertrust_color::Adaptation;
    use apertrust_color::adaptation::{NORM_MIN, chroma_adapt_pixel};
    let colours = vec![
        [0.18, 0.18, 0.18, 1.0],
        [0.3, 0.2, 0.1, 1.0],
        [0.05, 0.12, 0.4, 1.0],
        [0.6, 0.55, 0.02, 1.0],
    ];
    let (picture, image) = Picture::row(colours.clone(), 0.0);
    let mut bench = Bench::picture(&picture, image, Pipe::Export);
    let to_xyz = common::profile().rgb_to_xyz_d50;
    let mut worst = 0.0_f32;
    for (option, adaptation) in [
        ("linear_bradford", Adaptation::LinearBradford),
        ("cat16", Adaptation::Cat16),
        ("non_linear_bradford", Adaptation::FullBradford),
        ("xyz", Adaptation::Xyz),
    ] {
        for xy in [[0.44757, 0.40745], [0.3127, 0.329], [0.32, 0.36]] {
            let set = [
                ("adaptation", choice("adaptation", option)),
                ("illuminant", illuminant("custom")),
                ("x", number(xy[0])),
                ("y", number(xy[1])),
                ("gamut", number(0.0)),
                ("clip", OFF),
            ];
            let output = custom(&mut bench, &set);
            let white = illuminants::illuminant_xy_to_xyz(xy);
            let cone = apertrust_color::apply(&adaptation.xyz_to_lms(), white);
            let p = (0.818155 / cone[2].max(NORM_MIN)).powf(0.0834);
            for (colour, out) in colours.iter().zip(&output.pixels) {
                let xyz = apertrust_color::apply(&to_xyz, [colour[0], colour[1], colour[2]]);
                let want = chroma_adapt_pixel(xyz, cone, adaptation, p);
                let got = apertrust_color::apply(&to_xyz, [out[0], out[1], out[2]]);
                let scale = want.iter().fold(0.0_f32, |m, v| m.max(v.abs()));
                for (got, want) in got.iter().zip(want) {
                    let off = (got - want).abs() / scale;
                    worst = worst.max(off);
                    assert!(
                        off <= 1e-4,
                        "{option} under {xy:?}, {colour:?}: {got} {want}"
                    );
                }
            }
        }
    }
    println!("chroma_adapt_pixel: at most {worst:.2e} of the largest channel from the kernel");
}

// The corpus, locally.

/// C0 on every corpus raw: the tool's untouched values are the operation's own, and
/// Colour calibration at its defaults, written or kept, plans the very instances and
/// cache keys no edit does, so the same frame.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn c0_on_the_corpus() {
    let mut raws = 0;
    each_corpus_raw(|name, image, input| {
        defaults_are_the_operation_s(&image);
        let keys = |edit: &Edit| -> Vec<(&str, u64)> {
            let instances = edit::instances(edit, OPERATIONS, TOOLS, &image)
                .all()
                .unwrap();
            pipeline::plan(
                &instances,
                &image,
                input.desc(),
                Pipe::Export,
                &Limits::NONE,
            )
            .unwrap()
            .operations
            .iter()
            .map(|op| (op.id, op.key))
            .collect()
        };
        let none = keys(&Edit::default());
        for edit in untouched(&image) {
            assert_eq!(keys(&edit), none, "C0 on {name}");
        }
        raws += 1;
    });
    println!("C0 holds on {raws} raws");
}

/// C5 on every corpus raw: the defaults' hue and chroma lie in their ranges, and give the
/// camera's own light back to a few `f32` units.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn c5_on_the_corpus() {
    let mut raws = 0;
    each_corpus_raw(|name, image, _| {
        let defaults = ColourCalibration.defaults(&image);
        let xy = [defaults.number("x"), defaults.number("y")];
        let [hue, chroma] = viewed(xy, &image);
        assert!((0.0..=360.0).contains(&hue), "{name}: hue {hue}");
        assert!((0.0..=300.0).contains(&chroma), "{name}: chroma {chroma}");
        let back = unviewed([hue, chroma], &image);
        for (a, b) in xy.iter().zip(back) {
            assert!(
                (a - b).abs() <= 4.0 * f32::EPSILON,
                "{name}: {xy:?} {back:?}"
            );
        }
        println!("{name}: hue {hue:.1}°, chroma {chroma:.1} %");
        raws += 1;
    });
    println!("C5 holds on {raws} raws");
}
