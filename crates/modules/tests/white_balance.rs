// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/iop/temperature.c)
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

//! White balance's invariants (`docs/modules/white-balance.md`, Invariants), one test per
//! row, each named after its row. The small raw runs on CI's software GPUs, the CPU rows
//! everywhere; the raw.pixls.us corpus runs locally, with `APERTRUST_RAW_CORPUS` naming
//! it:
//!
//! ```sh
//! cargo test --release -p apertrust-modules --test white_balance -- --ignored --nocapture
//! ```
//!
//! White balance acts in the raw stage, which a synthetic picture in the working space
//! skips, so every rendering row runs the small raw, from its mosaic, and reads the frame,
//! colorout's output before it is rounded to 8 bits.

#[expect(
    dead_code,
    reason = "White balance's rows are exact and run the small raw alone"
)]
mod common;
#[expect(
    dead_code,
    reason = "White balance's corpus rows read each raw's context and plan, so they need no corpus bench"
)]
mod corpus;

use apertrust_color::white_balance::{Matrices, TEMPERATURE_RANGE, TINT_RANGE, temperature_to_xyz};
use apertrust_engine::edit::{self, Edit, Held};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe};
use apertrust_modules::white_balance::{
    COEFFICIENTS, SETTINGS, SETTINGS_ROW, TEMPERATURE_TINT, WhiteBalance,
};
use apertrust_modules::{Base, ControlValue, Controls, MODULES, Mark, Module, Row};
use apertrust_operations::OPERATIONS;
use apertrust_plugin::{ImageContext, ParamValues, Value};
use golden::cases::CASES;

use common::{Bench, Rendered, exact, id, raw_context, small_raw};
use corpus::each_corpus_raw;

const MODULE: &str = "temperature";

/// What every rendering row reads: the frame, colorout's output.
#[derive(Debug, Clone, Copy)]
struct Frame;

impl common::Measure for Frame {
    fn keep(self, _: &mut Vec<Instance<'static>>, _: bool) {}
}

fn number(value: f32) -> ControlValue {
    ControlValue::Number(value)
}

/// The setting called `option`, as `preset` holds it.
fn setting(option: &str) -> ControlValue {
    ControlValue::Choice(SETTINGS.iter().position(|&o| o == option).unwrap() as u32)
}

/// The small raw through the viewport, where White balance's moves show.
fn bench() -> Bench {
    let (mosaic, image) = small_raw(0.0);
    let viewport = Pipe::Viewport {
        width: 40,
        height: 30,
    };
    Bench::mosaic(&mosaic, image, viewport)
}

/// Every control as `edit` leaves it on `image`: its value over its default for the photo.
fn every(edit: &Edit, image: &ImageContext) -> Controls {
    let definition = WhiteBalance.definition();
    let mut values = WhiteBalance.defaults(image);
    for (control, value) in edit.module(MODULE).iter().flat_map(|set| set.values()) {
        values.set(definition.control(control).unwrap().id, value.clone());
    }
    values
}

fn coefficients(values: &Controls) -> [f32; 3] {
    COEFFICIENTS.map(|control| values.number(control))
}

/// `control`, a control or a coordinate of the view, set on `image` as the session sets it,
/// with its follow writes (decision 27).
fn set(edit: &mut Edit, held: &mut Held, control: &str, value: ControlValue, image: &ImageContext) {
    let before = edit.clone();
    if WhiteBalance.definition().coordinate(control).is_some() {
        edit.set_coordinate(held, &WhiteBalance, control, value, image)
            .unwrap();
    } else {
        edit.set_control(&WhiteBalance, control, value, image)
            .unwrap();
    }
    let set = edit::written(&WhiteBalance, control);
    edit.follow(&before, &WhiteBalance, set, image).unwrap();
}

/// `control` reset on `image` as the session resets it, with its follow writes.
fn reset(edit: &mut Edit, held: &mut Held, control: &str, image: &ImageContext) {
    let before = edit.clone();
    if WhiteBalance.definition().coordinate(control).is_some() {
        edit.reset_coordinate(held, &WhiteBalance, control, image)
            .unwrap();
    } else {
        edit.reset_control(&WhiteBalance, control).unwrap();
    }
    let set = edit::written(&WhiteBalance, control);
    edit.follow(&before, &WhiteBalance, set, image).unwrap();
}

/// Settings' `option` applied, as the session applies it: the option's values, no follow
/// writes.
fn apply(edit: &mut Edit, option: &str, image: &ImageContext) {
    edit.apply_preset(&WhiteBalance, SETTINGS_ROW, option, image)
        .unwrap();
}

/// What the operation renders an unedited `image` with, every parameter resolved.
fn operation_defaults(image: &ImageContext) -> ParamValues {
    let operation = apertrust_operations::operation(MODULE).unwrap();
    operation
        .schema()
        .resolve_over(&operation.defaults(image), &ParamValues::new())
        .unwrap()
}

/// A control's value as its parameter holds it.
fn as_param(value: &ControlValue) -> Value {
    match value {
        ControlValue::Number(value) => Value::Float(*value),
        ControlValue::Choice(index) => Value::Choice(*index),
        other => panic!("no White balance control holds {other:?}"),
    }
}

// W0: every control at its default.

/// Edits that leave every control at its default for `image`: each written at it, and
/// each kept there.
fn untouched(image: &ImageContext) -> Vec<Edit> {
    let controls = WhiteBalance.definition().controls;
    let defaults = WhiteBalance.defaults(image);
    let mut written = Edit::default();
    let mut kept = Edit::default();
    for control in controls {
        let value = defaults.get(control.id).unwrap().clone();
        written
            .set_control(&WhiteBalance, control.id, value, image)
            .unwrap();
        kept.keep(&WhiteBalance, control.id, true).unwrap();
    }
    vec![written, kept]
}

/// The module's untouched values are the operation's own on `image`.
fn defaults_are_the_operation_s(image: &ImageContext) {
    let resolved = operation_defaults(image);
    let defaults = WhiteBalance.defaults(image);
    for control in WhiteBalance.definition().controls {
        let default = as_param(defaults.get(control.id).unwrap());
        assert_eq!(resolved.get(control.id), Some(default), "{}", control.id);
    }
}

#[test]
fn w0_every_control_at_its_default_renders_as_no_edit() {
    for pipe in [
        Pipe::Export,
        Pipe::Viewport {
            width: 40,
            height: 30,
        },
    ] {
        let (mosaic, image) = small_raw(0.0);
        let mut bench = Bench::mosaic(&mosaic, image.clone(), pipe);
        defaults_are_the_operation_s(&image);
        let none = bench.edit(&Edit::default(), Frame);
        let mut moved = Edit::default();
        moved
            .set_control(&WhiteBalance, "red", number(2.5), &image)
            .unwrap();
        bench.edit(&moved, Frame);
        for edit in untouched(&image) {
            let values = edit.module(MODULE).unwrap();
            let off = values.off_defaults(&WhiteBalance.defaults(&image));
            let base = Base::new(OPERATIONS, &image);
            assert!(WhiteBalance.bind(&off, &image, &base).is_empty());
            assert!(exact(&none, &bench.edit(&edit, Frame)), "W0 on {pipe:?}");
        }
    }
}

// W1: each control moved, then reset.

/// What each move sets, as the session sets it: a coefficient, a temperature, a tint.
fn moves() -> Vec<(&'static str, ControlValue)> {
    vec![
        ("red", number(2.5)),
        ("green", number(1.2)),
        ("blue", number(0.9)),
        ("temperature", number(3500.0)),
        ("tint", number(1.2)),
    ]
}

/// W1 as far as it holds. Each control moved as the session moves it changes the frame,
/// and each Settings option but as shot to reference, which the defaults hold; each
/// renders as no edit once the module is reset, and each option once the controls it
/// wrote are reset, control by control. A coefficient, temperature or tint reset control
/// by control leaves its follow write, `preset` user modified, so the photo renders as
/// shot, darktable's own state then (its `gui_update` names the as-shot coefficients as
/// shot), the frame Settings' as shot gives.
#[test]
fn w1_a_control_moved_and_reset_renders_as_no_edit_or_as_shot() {
    let mut bench = bench();
    let image = bench.image.clone();
    let none = bench.edit(&Edit::default(), Frame);
    let mut as_shot = Edit::default();
    apply(&mut as_shot, "as_shot", &image);
    let as_shot = bench.edit(&as_shot, Frame);
    assert_ne!(
        none.pixels, as_shot.pixels,
        "as shot keeps no late correction"
    );
    for (control, value) in moves() {
        let (mut edit, mut held) = (Edit::default(), Held::default());
        set(&mut edit, &mut held, control, value, &image);
        let moved: Rendered = bench.edit(&edit, Frame);
        assert_ne!(none.pixels, moved.pixels, "{control} changes the frame");
        let mut reset_module = edit.clone();
        reset_module.reset_module(&WhiteBalance);
        assert!(exact(&none, &bench.edit(&reset_module, Frame)), "{control}");
        reset(&mut edit, &mut held, control, &image);
        let left = every(&edit, &image);
        assert_eq!(
            coefficients(&left),
            coefficients(&WhiteBalance.defaults(&image))
        );
        assert_eq!(left.get("preset"), Some(&setting("user")), "{control}");
        assert!(exact(&as_shot, &bench.edit(&edit, Frame)), "{control}");
    }
    for option in ["as_shot", "d65", "d65_late"] {
        let mut edit = Edit::default();
        apply(&mut edit, option, &image);
        let moved = bench.edit(&edit, Frame);
        assert_eq!(
            none.pixels == moved.pixels,
            option == "d65_late",
            "{option}"
        );
        let mut held = Held::default();
        let row = WhiteBalance.definition().preset(SETTINGS_ROW).unwrap();
        for control in row.controls {
            reset(&mut edit, &mut held, control, &image);
        }
        assert_eq!(edit, Edit::default(), "{option}");
        assert!(exact(&none, &bench.edit(&edit, Frame)), "{option}");
    }
}

// W2: the order temperature and tint were set in.

/// Temperature then tint, and tint then temperature, the session holding the view's
/// other coordinate: the same coefficients, bit for bit, and the same frame.
#[test]
fn w2_temperature_and_tint_give_the_same_coefficients_whichever_came_first() {
    let mut one = bench();
    let image = one.image.clone();
    let both = |first: (&str, f32), second: (&str, f32)| {
        let (mut edit, mut held) = (Edit::default(), Held::default());
        set(&mut edit, &mut held, first.0, number(first.1), &image);
        set(&mut edit, &mut held, second.0, number(second.1), &image);
        edit
    };
    let a = both(("temperature", 4300.0), ("tint", 1.1));
    let b = both(("tint", 1.1), ("temperature", 4300.0));
    assert_eq!(a, b);
    let mut other = bench();
    assert!(exact(&one.edit(&a, Frame), &other.edit(&b, Frame)));
}

// W3: the view.

/// A grid of temperature and tint over their ranges, by `steps` each, ends included.
fn grid(steps: usize) -> impl Iterator<Item = [f64; 2]> {
    let (t0, t1) = TEMPERATURE_RANGE;
    let (k0, k1) = TINT_RANGE;
    (0..=steps).flat_map(move |i| {
        (0..=steps).map(move |j| {
            let (a, b) = (i as f64 / steps as f64, j as f64 / steps as f64);
            [t0 + (t1 - t0) * a, k0 + (k1 - k0) * b]
        })
    })
}

/// Temperature and tint as the module's view gives them for `coefficients`.
fn viewed(coefficients: [f32; 3], image: &ImageContext) -> [f32; 2] {
    let mut values = WhiteBalance.defaults(image);
    for (control, value) in COEFFICIENTS.into_iter().zip(coefficients) {
        values.set(control, number(value));
    }
    let coordinates = WhiteBalance.view(TEMPERATURE_TINT, &values, image).unwrap();
    [coordinates[0], coordinates[1]]
}

/// The coefficients the view's inverse gives for a temperature and a tint.
fn unviewed(coordinates: [f32; 2], image: &ImageContext) -> [f32; 3] {
    let values = WhiteBalance
        .unview(TEMPERATURE_TINT, &coordinates, image)
        .unwrap();
    COEFFICIENTS.map(|control| match values.get(control) {
        Some(ControlValue::Number(value)) => *value,
        other => panic!("{control} is {other:?}"),
    })
}

/// The temperatures darktable's model gives twins across 4000 K, where its light switches
/// from a black body to daylight and Z/X steps down by 4 %: a coordinate there may come
/// back as its twin, with the same coefficients (`apertrust_color::white_balance`).
const TWINS: (f32, f32) = (3900.33, 4102.41);

fn in_range(coefficients: [f32; 3]) -> bool {
    coefficients.iter().all(|&c| (0.0..=8.0).contains(&c))
}

/// W3 on `image`, the worst of its round trips.
#[derive(Debug, Clone, Copy)]
struct RoundTrips {
    /// The coordinates', where the coefficients they give lie in their range: the
    /// temperature's in kelvin and relative to it, and the tint's.
    kelvin: f64,
    relative: f64,
    tint: f64,
    /// The as-shot coefficients' through the view and back, in `f32` units.
    units: f32,
}

/// W3's round trips on `image`, the twins across 4000 K left out unless `twins`.
fn round_trips(image: &ImageContext, twins: bool) -> RoundTrips {
    let (mut kelvin, mut relative, mut tint) = (0.0f64, 0.0f64, 0.0f64);
    for [t, k] in grid(60) {
        let coordinates = [t as f32, k as f32];
        let coefficients = unviewed(coordinates, image);
        if !in_range(coefficients) || (!twins && (TWINS.0..=TWINS.1).contains(&coordinates[0])) {
            continue;
        }
        let [back, back_tint] = viewed(coefficients, image);
        kelvin = kelvin.max(f64::from((back - coordinates[0]).abs()));
        relative = relative.max(f64::from((back / coordinates[0] - 1.0).abs()));
        tint = tint.max(f64::from((back_tint - coordinates[1]).abs()));
    }
    let shot = coefficients(&WhiteBalance.defaults(image));
    let back = unviewed(viewed(shot, image), image);
    let units = (0..3)
        .map(|c| (back[c] - shot[c]).abs() / shot[c].abs().max(1.0) / f32::EPSILON)
        .fold(0.0, f32::max);
    RoundTrips {
        kelvin,
        relative,
        tint,
        units,
    }
}

impl RoundTrips {
    /// What W3 holds in CI: the temperature within 1e-6 of itself, a few `f32` units of it,
    /// the tint within 1e-6, the coefficients within 4 `f32` units.
    fn hold(&self) -> bool {
        self.relative <= 1e-6 && self.tint <= 1e-6 && self.units <= 4.0
    }
}

/// The image contexts W3 to W5 run on without the corpus: the small raw's camera, and two
/// cameras of darktable's integration tests, the Canon EOS 40D and the Fujifilm X-T4.
fn cameras() -> Vec<ImageContext> {
    let with = |white_balance: Vec<f32>, matrix: [[f32; 3]; 3]| {
        let mut image = raw_context(8, 8, 0.0);
        let camera = image.camera.as_mut().unwrap();
        camera.white_balance = Some(white_balance);
        camera.xyz_to_camera = Some(matrix.map(|row| row.map(|v| v / 10000.0)));
        image
    };
    vec![
        raw_context(8, 8, 0.0),
        with(
            vec![2.115_234_4, 1.0, 1.398_437_5],
            [
                [6071.0, -747.0, -856.0],
                [-7653.0, 15365.0, 2441.0],
                [-2025.0, 2553.0, 7315.0],
            ],
        ),
        with(
            vec![1.9139073, 1.0, 2.1887417],
            [
                [13426.0, -6334.0, -1177.0],
                [-4244.0, 12136.0, 2371.0],
                [-580.0, 1303.0, 5980.0],
            ],
        ),
    ]
}

/// W3: temperature and tint, a view of the coefficients through darktable's model. Where
/// the coefficients a coordinate gives lie in their range, it comes back within 1e-6 of
/// its temperature and 1e-6 in tint, but for darktable's twins across 4000 K; and the
/// as-shot coefficients come back, normalised to green 1, within a few `f32` units.
#[test]
fn w3_temperature_and_tint_round_trip() {
    for image in cameras() {
        let trips = round_trips(&image, false);
        println!("W3: {trips:?}");
        assert!(trips.hold(), "{trips:?}");
    }
}

/// W3's proposed bound, 0.01 K and 1e-6 in tint, everywhere in the coordinates' ranges.
/// It fails twice: from 3900.33 K to 4102.41 K, where darktable's model gives a
/// temperature a twin across 4000 K with the same Z/X, so the view of its coefficients may
/// give the twin; and above about 20000 K, where an `f32` temperature steps by 0.002 K
/// and the coefficients' own rounding moves it by a few steps, 0.014 K at most, 6e-7 of
/// it (decision 30).
#[test]
#[ignore = "fails across 4000 K, darktable's twins, and above 20000 K by f32's steps (decision 30)"]
fn w3_temperature_and_tint_round_trip_to_0_01_k_everywhere() {
    for image in cameras() {
        let trips = round_trips(&image, true);
        assert!(trips.kelvin <= 0.01 && trips.tint <= 1e-6, "{trips:?}");
    }
}

// W4: darktable's model.

/// darktable's `_mul2temp`: `_mul2xyz`, then `_XYZ_to_temperature`'s bisection to 1 K,
/// the tint from the last temperature it tried, both clamped to their ranges. The last
/// temperature tried is given too.
fn darktable_mul2temp(matrices: &Matrices, coefficients: [f64; 3]) -> (f64, f64, f64) {
    let xyz = matrices.xyz(coefficients);
    let (mut max, mut min) = (TEMPERATURE_RANGE.1, TEMPERATURE_RANGE.0);
    let mut t = (max + min) / 2.0;
    let mut tried = t;
    let mut light = temperature_to_xyz(t);
    while max - min > 1.0 {
        tried = t;
        light = temperature_to_xyz(t);
        if light[2] / light[0] > xyz[2] / xyz[0] {
            max = t;
        } else {
            min = t;
        }
        t = (max + min) / 2.0;
    }
    let tint = (light[1] / light[0]) / (xyz[1] / xyz[0]);
    let (t, tint) = (
        t.clamp(TEMPERATURE_RANGE.0, TEMPERATURE_RANGE.1),
        tint.clamp(TINT_RANGE.0, TINT_RANGE.1),
    );
    (t, tint, tried)
}

/// darktable's `_temp2mul`, normalised as `_temp_tint_callback` does.
fn darktable_temp2mul(matrices: &Matrices, t: f64, tint: f64) -> [f64; 3] {
    let mut xyz = temperature_to_xyz(t);
    xyz[1] /= tint;
    let camera: [f64; 3] =
        std::array::from_fn(|k| (0..3).map(|i| matrices.xyz_to_camera[k][i] * xyz[i]).sum());
    let mul = camera.map(|c| 1.0 / c);
    [mul[0] / mul[1], 1.0, mul[2] / mul[1]]
}

/// W4: the view against darktable's `_mul2temp` and `_temp2mul` restated in `f64`: the
/// same coefficients from the same coordinates within 1e-9 relative, the same temperature
/// within darktable's 1 K bisection, and darktable's tint the model's at the temperature
/// it last tried, within 1e-6.
fn w4_holds(image: &ImageContext) {
    let matrices = Matrices::camera(&image.camera.as_ref().unwrap().xyz_to_camera.unwrap());
    for [t, k] in grid(40) {
        let ours = matrices.coefficients(t, k);
        let theirs = darktable_temp2mul(&matrices, t, k);
        for c in 0..3 {
            assert!(
                (ours[c] - theirs[c]).abs() <= 1e-9 * theirs[c].abs(),
                "{t} {k}"
            );
        }
        if !ours.iter().all(|&c| c > 0.0 && c <= 8.0) {
            continue;
        }
        let [our_t, _] = matrices.temperature_tint(ours);
        let (their_t, their_tint, tried) = darktable_mul2temp(&matrices, ours);
        assert!((our_t - their_t).abs() <= 1.0, "{t} {k}: {our_t} {their_t}");
        let at = apertrust_color::white_balance::tint_at(tried, matrices.xyz(ours));
        if (TINT_RANGE.0..=TINT_RANGE.1).contains(&at) {
            assert!((at - their_tint).abs() <= 1e-6, "{t} {k}");
        }
    }
}

#[test]
fn w4_the_view_is_darktable_s_model() {
    for image in cameras() {
        w4_holds(&image);
    }
}

/// W4's anchor, hard-coded numbers alone, so a wrong table entry, constant or
/// pseudoinverse cannot pass it as it would the restatement above, which calls the model:
/// the Fujifilm X-T50's as-shot coefficients in darktable's sidecar, under rawler's X-T50
/// D65 matrix, read 6311 K and tint 1.022 at darktable's digits, as darktable's
/// panel shows them; and 6311 K at tint 1.022 gives coefficients within the panel's
/// rounding of the sidecar's, pinned at the model's value.
#[test]
fn w4_the_x_t50_s_as_shot_reads_as_darktable_s_panel() {
    let mut image = raw_context(8, 8, 0.0);
    let camera = image.camera.as_mut().unwrap();
    camera.white_balance = Some(vec![2.1655629, 1.0, 1.5264901]);
    camera.xyz_to_camera = Some([
        [1.1809, -0.5358, -0.1141],
        [-0.4248, 1.2164, 0.2343],
        [-0.0514, 0.1097, 0.5848],
    ]);
    let [t, tint] = viewed(coefficients(&WhiteBalance.defaults(&image)), &image);
    assert_eq!(
        (t.round(), (tint * 1000.0).round()),
        (6311.0, 1022.0),
        "{t} K, {tint}"
    );
    let given = unviewed([6311.0, 1.022], &image);
    let sidecar = [2.1655629f32, 1.0, 1.5264901];
    // Within the panel's rounding, half a kelvin and half a thousandth of tint, the red
    // and blue coefficients move by 0.0026 and 0.0009 at most.
    for c in 0..3 {
        assert!((given[c] - sidecar[c]).abs() <= 0.003, "{given:?}");
    }
    let pinned = [2.163_987_5f32, 1.0, 1.525_930_2];
    for c in 0..3 {
        assert!((given[c] - pinned[c]).abs() <= 1e-6, "{given:?}");
    }
}

// W5: Settings.

/// W5 on `image`: each option writes darktable's values and reads back; a moved
/// coefficient reads user modified; from image area and user modified are never given.
fn w5_holds(image: &ImageContext) {
    let camera = image.camera.as_ref().unwrap();
    let as_shot = coefficients(&WhiteBalance.defaults(image));
    // darktable's D65coeffs: from the colour matrix, else every one 1.
    let daylight = camera.xyz_to_camera.map_or([1.0; 3], |matrix| {
        apertrust_color::daylight_multipliers(&matrix).map(|v| v as f32)
    });
    let fourth = WhiteBalance.defaults(image).get("various").cloned();
    for (option, expected, various) in [
        ("as_shot", as_shot, fourth.clone()),
        ("d65", daylight, Some(number(1.0))),
        ("d65_late", as_shot, fourth),
    ] {
        let values = WhiteBalance.preset(SETTINGS_ROW, option, image).unwrap();
        assert_eq!(coefficients(&values), expected, "{option}");
        // All four, as `_temp_params_from_array` writes them.
        assert_eq!(values.get("various").cloned(), various, "{option}");
        assert_eq!(values.get("preset"), Some(&setting(option)), "{option}");
        let mut every = WhiteBalance.defaults(image);
        for (control, value) in values.iter() {
            every.set(control, value.clone());
        }
        let matching = WhiteBalance.matching(SETTINGS_ROW, &every, image);
        assert_eq!(matching, Some(option), "{option}");
    }
    for option in ["spot", "user"] {
        assert_eq!(WhiteBalance.preset(SETTINGS_ROW, option, image), None);
    }
    let defaults = WhiteBalance.defaults(image);
    let untouched = if camera.xyz_to_camera.is_some() {
        "d65_late"
    } else {
        "as_shot"
    };
    assert_eq!(
        WhiteBalance.matching(SETTINGS_ROW, &defaults, image),
        Some(untouched)
    );
    let mut moved = defaults.clone();
    moved.set("red", number(as_shot[0] + 0.01));
    assert_eq!(
        WhiteBalance.matching(SETTINGS_ROW, &moved, image),
        Some("user")
    );
    // Within darktable's 1e-5 the as-shot coefficients still read as such.
    moved.set("red", number(as_shot[0] + 5e-6));
    assert_eq!(
        WhiteBalance.matching(SETTINGS_ROW, &moved, image),
        Some(untouched)
    );
}

#[test]
fn w5_settings_writes_darktable_s_values_and_names_them() {
    for image in cameras() {
        w5_holds(&image);
        let mut without = image.clone();
        without.camera.as_mut().unwrap().xyz_to_camera = None;
        w5_holds(&without);
    }
}

// W6: follow writes.

/// W6: a coefficient, temperature and tint, each moved from as shot to reference, write
/// `preset` user modified, kept, in the same change, so the operation renders no late
/// correction. The session's own test is
/// `a_white_balance_coefficient_writes_user_modified_and_a_setting_writes_none` in
/// `crates/engine/src/session.rs`.
#[test]
fn w6_a_coefficient_temperature_or_tint_writes_user_modified() {
    let image = raw_context(8, 8, 0.0);
    for (control, value) in moves() {
        let (mut edit, mut held) = (Edit::default(), Held::default());
        set(&mut edit, &mut held, control, value, &image);
        let values = edit.module(MODULE).unwrap();
        assert_eq!(values.get("preset"), Some(&setting("user")), "{control}");
        assert_eq!(values.mark("preset"), Some(Mark::Kept), "{control}");
        let instances = edit::instances(&edit, OPERATIONS, MODULES, &image)
            .all()
            .unwrap();
        let temperature = instances.iter().find(|i| id(i) == MODULE).unwrap();
        assert_eq!(
            temperature.params.get("preset"),
            Some(Value::Choice(2)),
            "{control}"
        );
    }
}

// W7: darktable's match.

/// W7: the temperature parameters of every golden sidecar that has them, version 4, set
/// as the module's controls, bind exactly darktable's, so the raw stage's checks against
/// darktable are the module's.
#[test]
fn w7_each_golden_sidecar_s_values_bind_darktable_s_parameters() {
    let image = raw_context(8, 8, 0.0);
    let mut sidecars = 0;
    for case in CASES {
        // darktable migrates an older sidecar's with `preset` unknown, which no control
        // holds; version 4's alone are W7's.
        let Ok(params) = golden::xmp::module_params(case.xmp, MODULE) else {
            continue;
        };
        if params.version != 4 {
            continue;
        }
        assert_eq!(params.bytes.len(), 20, "{}", case.name);
        let word = |i: usize| <[u8; 4]>::try_from(&params.bytes[4 * i..4 * i + 4]).unwrap();
        let theirs: Vec<(&str, Value)> = vec![
            ("red", Value::Float(f32::from_le_bytes(word(0)))),
            ("green", Value::Float(f32::from_le_bytes(word(1)))),
            ("blue", Value::Float(f32::from_le_bytes(word(2)))),
            ("preset", Value::Choice(i32::from_le_bytes(word(4)) as u32)),
        ];
        let mut edit = Edit::default();
        for (control, value) in &theirs {
            let value = match value {
                Value::Float(v) => number(*v),
                Value::Choice(i) => ControlValue::Choice(*i),
                other => panic!("{other:?}"),
            };
            edit.set_control(&WhiteBalance, control, value, &image)
                .unwrap();
        }
        let instances = edit::instances(&edit, OPERATIONS, MODULES, &image)
            .all()
            .unwrap();
        let instance = instances.iter().find(|i| id(i) == MODULE).unwrap();
        let operation = instance.operation;
        let ours = operation
            .schema()
            .resolve_over(&operation.defaults(&image), &instance.params)
            .unwrap();
        for (param, value) in theirs {
            assert_eq!(ours.get(param), Some(value), "{}: {param}", case.name);
        }
        sidecars += 1;
    }
    println!("W7 holds on {sidecars} sidecars");
    assert!(sidecars >= 1);
}

/// W8: background only, as the raw stage: the module never offers itself on a layer.
#[test]
fn w8_it_works_on_the_background_only() {
    let definition = WhiteBalance.definition();
    assert!(!definition.layers);
    assert!(!definition.auto);
}

/// W9: temperature and tint are unavailable without the camera's colour matrix (decision
/// 30), the coefficients and the setting available, and Settings offers as shot, camera
/// reference and as shot to reference with a matrix or without, as darktable does;
/// Emerald, a four-colour sensor's `various`, which darktable shows on such a sensor alone
/// and temperature does not port, is a control not ported in no row, Settings writing it.
#[test]
fn w9_temperature_and_tint_need_the_camera_s_matrix() {
    let with = raw_context(8, 8, 0.0);
    let mut without = with.clone();
    without.camera.as_mut().unwrap().xyz_to_camera = None;
    let definition = WhiteBalance.definition();
    for (image, matrix) in [(&with, true), (&without, false)] {
        let values = WhiteBalance.defaults(image);
        for id in ["temperature", "tint"] {
            assert_eq!(WhiteBalance.available(id, &values, image), matrix, "{id}");
        }
        for control in definition.controls {
            assert!(WhiteBalance.available(control.id, &values, image));
        }
        let offered: Vec<&str> = SETTINGS
            .into_iter()
            .filter(|option| WhiteBalance.preset(SETTINGS_ROW, option, image).is_some())
            .collect();
        assert_eq!(offered, ["as_shot", "d65", "d65_late"]);
    }
    let various = definition.control("various").unwrap();
    assert!(various.unported);
    assert!(
        definition
            .rows()
            .all(|row| !row.controls().contains(&"various"))
    );
    assert!(
        definition
            .preset(SETTINGS_ROW)
            .unwrap()
            .controls
            .contains(&"various")
    );
    assert!(matches!(
        definition.tabs[0].rows.last(),
        Some(Row::Group {
            id: "channel_coefficients",
            ..
        })
    ));
}

// The corpus, locally.

/// W0, W3, W4 and W5 on every corpus raw, every temperature at its as-shot tint in range,
/// as `crates/modules/tests/builtin.rs` holds the view, and the reach of temperature and
/// tint on each camera: the coefficients the corners of their ranges give, which lie
/// outside the operation's 0 to 8 on every one.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn w_rows_on_the_corpus() {
    let mut raws = 0;
    each_corpus_raw(|name, image, input| {
        defaults_are_the_operation_s(&image);
        let keys = |edit: &Edit| -> Vec<(&str, u64)> {
            let instances = edit::instances(edit, OPERATIONS, MODULES, &image)
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
            assert_eq!(keys(&edit), none, "W0 on {name}");
        }
        let trips = round_trips(&image, false);
        assert!(trips.hold(), "W3 on {name}: {trips:?}");
        // builtin.rs's check for the view: every temperature at the as-shot tint.
        let [_, tint] = viewed(coefficients(&WhiteBalance.defaults(&image)), &image);
        for step in 0..=100 {
            let t = TEMPERATURE_RANGE.0
                + (TEMPERATURE_RANGE.1 - TEMPERATURE_RANGE.0) * step as f64 / 100.0;
            let given = unviewed([t as f32, tint], &image);
            assert!(in_range(given), "{name}: {t} K at tint {tint}: {given:?}");
        }
        w4_holds(&image);
        w5_holds(&image);
        let shot = viewed(coefficients(&WhiteBalance.defaults(&image)), &image);
        let (mut low, mut high) = (f32::MAX, f32::MIN);
        let mut corners = Vec::new();
        let (mut outside, mut points) = (0, 0);
        // The tints at which every temperature gives coefficients in range.
        let mut whole: Vec<f64> = Vec::new();
        for [t, k] in grid(60) {
            let given = unviewed([t as f32, k as f32], &image);
            for c in given {
                (low, high) = (low.min(c), high.max(c));
            }
            points += 1;
            outside += usize::from(!in_range(given));
            let corner = (t == TEMPERATURE_RANGE.0 || t == TEMPERATURE_RANGE.1)
                && (k == TINT_RANGE.0 || k == TINT_RANGE.1);
            if corner && !in_range(given) {
                corners.push(format!("{t} K {k}"));
            }
        }
        for [_, k] in grid(60).take(61) {
            let every = grid(60)
                .filter(|&[_, tint]| tint == k)
                .all(|[t, tint]| in_range(unviewed([t as f32, tint as f32], &image)));
            if every {
                whole.push(k);
            }
        }
        let band = match (whole.first(), whole.last()) {
            (Some(a), Some(b)) => format!("{a:.3} to {b:.3}"),
            _ => "none".to_owned(),
        };
        println!(
            "{name}: as shot {:.0} K, tint {:.3}; coefficients {low:.3} to {high:.3}, {:.0} % of the grid outside 0 to 8; every temperature in range for tints {band}; corners outside: {}",
            shot[0],
            shot[1],
            100.0 * outside as f64 / points as f64,
            if corners.is_empty() {
                "none".to_owned()
            } else {
                corners.join(", ")
            }
        );
        raws += 1;
    });
    println!("W0, W3, W4, W5 and the as-shot tint's temperatures hold on {raws} raws");
}
