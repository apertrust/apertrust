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

//! AgX's invariants (`docs/modules/agx.md`, Invariants), one test per row, each named
//! after its row: A0 to A7, the rows a first-pass table has (decision 22) but the first
//! step, since every photo runs AgX, with the pivot's coordinates, what is not ported and
//! the pivot's range following the input range (decision 30). The synthetic ramp and
//! sweep and the small raw run on CI's software GPUs, A3, A4's parameters, A6 and A7 on
//! the CPU. The raw.pixls.us corpus runs locally, with `APERTRUST_RAW_CORPUS` naming it:
//!
//! ```sh
//! cargo test --release -p apertrust-modules --test agx -- --ignored --nocapture
//! ```
//!
//! Synthetic images enter after demosaic, already in the working space: their rows plan
//! the operations from exposure on and leave colorin out. Every row reads the frame,
//! colorout's output before it is rounded to 8 bits.

#[expect(
    dead_code,
    reason = "AgX's rows read neither the extended ramp nor a luminance"
)]
mod common;
mod corpus;

use apertrust_engine::edit::{self, Edit, EditError};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe};
use apertrust_modules::agx::{
    Agx, BASE_PRIMARIES, BLACK, FLAGS, PARAMETERS, PIVOT, Parameter, SCALING, WHITE, pivot_ev,
};
use apertrust_modules::{Base, ControlError, ControlValue, MODULES, Mark, Module, Row};
use apertrust_operations::OPERATIONS;
use apertrust_operations::agx::Agx as AgxOperation;
use apertrust_plugin::{ImageContext, ImageDesc, Kind, Operation, ParamValues, Value};

use common::{BEFORE_THE_WORKING_SPACE, Bench, Picture, exact, id, jump, small_raw, sweep};
use corpus::{CORPUS_VIEWPORT, NINETY_D, corpus_bench, each_corpus_raw, export_bench};

const AGX: &str = "agx";

/// The ramp (`docs/modules/README.md`, Test images): neutral greys one per 1/16 EV from
/// 2⁻¹⁶ to 2⁴, then RGB 0.18, mid-grey, the pivot's input. One row, over a colour raw's
/// context with an exposure bias of −0.67 EV.
fn ramp() -> (Picture, ImageContext) {
    common::ramp(&[0.18], -0.67)
}

/// What every row reads: the frame, colorout's output, from exposure on for a synthetic
/// image.
#[derive(Debug, Clone, Copy)]
struct Frame;

impl common::Measure for Frame {
    fn keep(self, instances: &mut Vec<Instance<'static>>, synthetic: bool) {
        instances.retain(|i| !(synthetic && BEFORE_THE_WORKING_SPACE.contains(&id(i))));
    }
}

/// A control's value: the base primaries by index, the toggles as 0 or 1, the numbers as
/// they are shown.
fn value_of(control: &str, value: f32) -> ControlValue {
    if control == BASE_PRIMARIES {
        ControlValue::Choice(value as u32)
    } else if FLAGS.contains(&control) {
        ControlValue::Bool(value != 0.0)
    } else {
        ControlValue::Number(value)
    }
}

/// An edit with these controls set, in this order, each as the session sets it: a
/// number a moved range leaves outside clamped in the same change (decision 30).
fn edit_of(set: &[(&str, f32)]) -> Edit {
    let mut edit = Edit::default();
    for &(control, value) in set {
        change(&mut edit, control, value_of(control, value)).unwrap();
    }
    edit
}

/// `control` set to `value` as the session sets it, its range's moves clamped after.
fn change(edit: &mut Edit, control: &str, value: ControlValue) -> Result<(), EditError> {
    let before = edit.clone();
    let photo = common::photo();
    edit.set_control(&Agx, control, value, &photo)?;
    edit.clamp(&before, MODULES, &photo)
}

impl Bench {
    fn set(&mut self, set: &[(&str, f32)]) -> common::Rendered {
        self.edit(&edit_of(set), Frame)
    }
}

/// The pipes the rows check on the small raw: the export's and the viewport's.
const PIPES: [Pipe; 2] = [
    Pipe::Export,
    Pipe::Viewport {
        width: 40,
        height: 30,
    },
];

// A0: every control at its default.

/// Edits that leave every control at its default: each written at its default, and each
/// kept there.
fn untouched() -> Vec<Edit> {
    let mut written = Edit::default();
    let mut kept = Edit::default();
    for control in Agx.definition().controls {
        written
            .set_control(&Agx, control.id, control.default(), &common::photo())
            .unwrap();
        kept.keep(&Agx, control.id, true).unwrap();
    }
    vec![written, kept]
}

/// A0 on one pipeline, as the session renders: no edit, then each untouched edit, every
/// frame as the first, its binding empty.
fn a0_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    let base = Base::new(OPERATIONS, &bench.image);
    for edit in untouched() {
        let values = edit.module(AGX).unwrap();
        let defaults = Agx.defaults(&bench.image);
        assert!(
            Agx.bind(&values.off_defaults(&defaults), &bench.image, &base)
                .is_empty()
        );
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "A0 on {:?}",
            bench.pipe
        );
    }
}

#[test]
fn a0_every_control_at_its_default_renders_as_no_edit() {
    for pipe in PIPES {
        let (mosaic, image) = small_raw(-0.67);
        a0_holds(&mut Bench::mosaic(&mosaic, image, pipe));
    }
    for (picture, image) in [ramp(), sweep(-0.67)] {
        a0_holds(&mut Bench::picture(&picture, image, Pipe::Export));
    }
}

/// A0 on every corpus raw: the module at its defaults, written or kept, plans the very
/// instances and cache keys no edit does, so the same frame.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn a0_on_the_corpus() {
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
        let none = keys(&bench.instances(&Edit::default(), Frame));
        for edit in untouched() {
            assert_eq!(keys(&bench.instances(&edit, Frame)), none, "A0 on {name}");
        }
        raws += 1;
    });
    println!("A0 holds on {raws} raws");
}

// A1: each control moved, then reset.

/// Moves on both tabs and in every group: the range, the pivot, the curve, the look, the
/// base primaries, the toggles and the primaries before and after tone mapping. Each
/// changes the frame.
const MOVES: [&[(&str, f32)]; 7] = [
    &[(WHITE, 8.0)],
    &[(PIVOT, 1.0)],
    &[
        ("curve_contrast_around_pivot", 2.0),
        ("curve_toe_power", 2.5),
    ],
    &[("look_saturation", 140.0), ("look_lift", 0.05)],
    &[(BASE_PRIMARIES, 5.0)],
    &[("disable_primaries_adjustments", 1.0)],
    &[
        (BLACK, -8.0),
        ("auto_gamma", 1.0),
        ("red_rotation", 10.0),
        ("completely_reverse_primaries", 1.0),
        ("master_unrotation_ratio", 50.0),
    ],
];

/// Each move, then reset by `ResetControl` (a double-click or Delete) and by
/// `ResetModule` (the header's reset), on the bench's one pipeline.
fn a1_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    for set in MOVES {
        let mut edit = edit_of(set);
        let moved = bench.edit(&edit, Frame);
        assert!(!exact(&none, &moved), "{set:?} changes the frame");
        for (control, _) in set {
            edit.reset_control(&Agx, control).unwrap();
        }
        assert_eq!(edit, Edit::default());
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "{set:?}, reset control by control"
        );
        let mut edit = edit_of(set);
        bench.edit(&edit, Frame);
        edit.reset_module(&Agx);
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "{set:?}, the module reset"
        );
    }
}

#[test]
fn a1_a_control_moved_and_reset_renders_as_no_edit() {
    let (picture, image) = sweep(-0.67);
    a1_holds(&mut Bench::picture(&picture, image, Pipe::Export));
    let (mosaic, image) = small_raw(-0.67);
    a1_holds(&mut Bench::mosaic(&mosaic, image, PIPES[1]));
}

// A2: the order controls were set in.

/// The pivot with the range it is relative to, the look with the curve, the base
/// primaries with a primary.
const PAIRS: [[(&str, f32); 2]; 3] = [
    [(WHITE, 8.0), (PIVOT, 1.0)],
    [("look_slope", 1.2), (BLACK, -7.0)],
    [(BASE_PRIMARIES, 4.0), ("green_inset", 40.0)],
];

/// One pipeline sets `first`, then both; another sets `second`, then both: the two
/// frames are the same, byte for byte.
fn a2_holds(new: impl Fn() -> Bench, [first, second]: [(&str, f32); 2]) {
    let mut one = new();
    one.set(&[first]);
    let one = one.set(&[first, second]);
    let mut other = new();
    other.set(&[second]);
    let other = other.set(&[second, first]);
    assert!(exact(&one, &other), "A2 with {first:?} and {second:?}");
}

#[test]
fn a2_controls_render_the_same_whichever_came_first() {
    let (picture, image) = sweep(-0.67);
    for pair in PAIRS {
        a2_holds(
            || Bench::picture(&picture, image.clone(), Pipe::Export),
            pair,
        );
    }
}

/// A0's sibling rows on the 90D: A1 and A2 through the viewport's frame and the
/// export's.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn a1_and_a2_on_the_corpus() {
    for pipe in [CORPUS_VIEWPORT, Pipe::Export] {
        a1_holds(&mut corpus_bench(NINETY_D, pipe));
        for pair in PAIRS {
            a2_holds(|| corpus_bench(NINETY_D, pipe), pair);
        }
    }
}

// A3: on a layer.

#[test]
fn a3_it_works_on_the_background_only() {
    let definition = Agx.definition();
    assert!(
        !definition.layers,
        "the tone mapper's parameters are global"
    );
    assert!(!definition.auto);
}

// A4: darktable's golden cases.

/// darktable `dt_iop_agx_params_t` version 7's fields, in its order: dynamic range
/// scaling, field 7, has no parameter, since only darktable's pickers read it.
const FIELDS: [&str; 36] = [
    "look_lift",
    "look_slope",
    "look_brightness",
    "look_saturation",
    "look_original_hue_mix_ratio",
    BLACK,
    WHITE,
    SCALING,
    PIVOT,
    "curve_pivot_y_linear_output",
    "curve_contrast_around_pivot",
    "curve_linear_ratio_below_pivot",
    "curve_linear_ratio_above_pivot",
    "curve_toe_power",
    "curve_shoulder_power",
    "curve_gamma",
    "auto_gamma",
    "curve_target_display_black_ratio",
    "curve_target_display_white_ratio",
    BASE_PRIMARIES,
    "disable_primaries_adjustments",
    "red_inset",
    "red_rotation",
    "green_inset",
    "green_rotation",
    "blue_inset",
    "blue_rotation",
    "master_outset_ratio",
    "master_unrotation_ratio",
    "red_outset",
    "red_unrotation",
    "green_outset",
    "green_unrotation",
    "blue_outset",
    "blue_unrotation",
    "completely_reverse_primaries",
];

/// darktable's version 7 fields, by name: the case's own, or, for an older version, the
/// scene-referred defaults its `legacy_params` gives, which are the operation's.
fn darktable_params(case: &golden::cases::Case) -> Vec<(&'static str, Value)> {
    let params = golden::xmp::module_params(case.xmp, AGX).unwrap();
    if params.version < 7 {
        let defaults = AgxOperation.schema().defaults();
        return FIELDS
            .iter()
            .filter_map(|&name| Some((name, defaults.get(name)?)))
            .collect();
    }
    assert_eq!(params.version, 7, "{}", case.name);
    let words = params.bytes.as_chunks::<4>().0.to_vec();
    FIELDS
        .iter()
        .zip(words)
        .filter(|(name, _)| **name != SCALING)
        .map(|(&name, word)| {
            let value = if name == BASE_PRIMARIES {
                Value::Choice(u32::from_le_bytes(word))
            } else if FLAGS.contains(&name) {
                Value::Bool(i32::from_le_bytes(word) != 0)
            } else {
                Value::Float(f32::from_le_bytes(word))
            };
            (name, value)
        })
        .collect()
}

/// The module's controls set to darktable's parameters as darktable's sliders show them,
/// unrounded: each number times its factor, the pivot in EV over the case's range.
fn edit_for(darktable: &[(&'static str, Value)]) -> Edit {
    let field = |name: &str| darktable.iter().find(|(n, _)| *n == name).unwrap().1;
    let float = |name: &str| match field(name) {
        Value::Float(value) => value,
        other => panic!("{name} is {other:?}"),
    };
    let mut edit = Edit::default();
    for p in &PARAMETERS {
        let shown = ControlValue::Number(p.shown(float(p.param)));
        edit.set_control(&Agx, p.control, shown, &common::photo())
            .unwrap();
    }
    let pivot = pivot_ev(float(PIVOT), float(BLACK), float(WHITE));
    edit.set_control(&Agx, PIVOT, ControlValue::Number(pivot), &common::photo())
        .unwrap();
    let Value::Choice(base) = field(BASE_PRIMARIES) else {
        panic!("the base primaries are a choice");
    };
    edit.set_control(
        &Agx,
        BASE_PRIMARIES,
        ControlValue::Choice(base),
        &common::photo(),
    )
    .unwrap();
    for flag in FLAGS {
        let Value::Bool(on) = field(flag) else {
            panic!("{flag} is a toggle");
        };
        edit.set_control(&Agx, flag, ControlValue::Bool(on), &common::photo())
            .unwrap();
    }
    edit
}

/// The parameters of the golden cases the binding reaches from no value of its control:
/// none of the f32 values within 64 steps of darktable's value as its slider shows it
/// gives the parameter back through the binding's `value / factor`. A control stores the
/// value as shown (`docs/modules/README.md`, Sliders against darktable), and division by
/// 100 or by `RAD_2_DEG` steps over some f32s: 66.13 % gives 0.66129994 or 0.66130006,
/// never darktable's 0.6613. darktable reaches it because its slider rounds a value set
/// through it to its digits ([`stored_by_darktable`]), so the gap is the binding's. Each
/// binds one f32 step from darktable's.
const UNREACHABLE: [(&str, &str); 3] = [
    ("agx", "red_inset"),
    ("agx", "red_outset"),
    ("agx", "blue_unrotation"),
];

/// Whether some f32 value of the control within 64 steps of darktable's value as shown
/// binds to `param` exactly. The window brackets `param`, its ends binding below and above
/// it, so a value it lacks lies between two the binding gives from neighbouring values.
fn reachable(p: &Parameter, param: f32) -> bool {
    let shown = p.shown(param).to_bits();
    let window = shown - 64..=shown + 64;
    let [a, b] = [*window.start(), *window.end()].map(|bits| p.param(f32::from_bits(bits)));
    assert!(
        a.min(b) < param && param < a.max(b),
        "{}: the window binds {a} to {b}, not round {param}",
        p.param
    );
    window
        .into_iter()
        .any(|bits| p.param(f32::from_bits(bits)).to_bits() == param.to_bits())
}

/// The parameter darktable's slider stores for a value it shows, in f32 as darktable
/// computes it (`src/bauhaus/bauhaus.c`): `dt_bauhaus_slider_set_val` takes the parameter
/// `value / factor`, `dt_bauhaus_slider_set` places it in the visible range from `min` to
/// `max`, `_slider_set_normalized` rounds it to the slider's digits, `roundf(base · x) /
/// base` with `base` 10^digits × its factor, and places it again, and
/// `dt_bauhaus_slider_get` reads it back from that place. A drag takes the same rounding.
fn stored_by_darktable(p: &Parameter, (min, max): (f32, f32), digits: u8, shown: f32) -> f32 {
    let at = (p.param(shown).clamp(min, max) - min) / (max - min);
    let base = 10f32.powf(f32::from(digits)) * p.factor;
    let rounded = (base * (min + (max - min) * at.clamp(0.0, 1.0))).round() / base;
    let placed = (rounded - min) / (max - min);
    min + placed * (max - min)
}

/// The decimals the control's slider row shows, darktable's digits.
fn decimals(control: &str) -> u8 {
    Agx.definition()
        .rows()
        .find_map(|row| match row {
            Row::Slider(slider) if slider.control == control => Some(slider.decimals),
            _ => None,
        })
        .unwrap()
}

/// A frame of the sweep through `agx` with these parameters, every other operation at
/// its defaults.
fn sweep_with(bench: &mut Bench, params: ParamValues) -> common::Rendered {
    let mut instances = bench.instances(&Edit::default(), Frame);
    instances.iter_mut().find(|i| id(i) == AGX).unwrap().params = params;
    bench.render(&instances)
}

/// A4: for each of AgX's golden cases, darktable's tests 0175 (`agx`), 0174 (params
/// version 4, which darktable replaces with its scene-referred defaults, `agx-default`),
/// 0186 (`agx-highlights`) and 0174's sidecar edited (`agx-edges`), the module's controls
/// set to the case's values as darktable's sliders show them, unrounded, bind to the
/// case's parameters bit for bit, but the three [`UNREACHABLE`] ones, which the binding
/// reaches from no value of their control, where darktable's rounding to its digits
/// reaches them, and which bind one f32 step from darktable's; and through the
/// operation's `prepare` to the kernel inputs the golden harness checks against
/// darktable's render, bit for bit for the three cases with none of them. On the sweep the
/// module's frame is the frame of darktable's parameters set directly: byte for byte for
/// those three, within the jump printed for `agx`. So the module renders the cases' values
/// as the cases measure the port: ΔE2000 means of 0.0000 (`docs/ports/agx.md`).
#[test]
fn a4_the_golden_cases_values_bind_to_darktable_s_parameters_bit_for_bit() {
    let schema = AgxOperation.schema();
    let image = ImageContext::full(1, 1);
    let (picture, sweep_image) = sweep(-0.67);
    let mut bench = Bench::picture(&picture, sweep_image, Pipe::Export);
    let mut missed = Vec::new();
    for name in ["agx", "agx-default", "agx-highlights", "agx-edges"] {
        let case = golden::cases::find(name).unwrap();
        let darktable = darktable_params(case);
        assert_eq!(darktable.len(), schema.params.len(), "{name}");
        let edit = edit_for(&darktable);

        let instances = edit::instances(&edit, OPERATIONS, MODULES, &image)
            .all()
            .unwrap();
        let instance = instances.iter().find(|i| id(i) == AGX).unwrap();
        let params = schema.resolve(&instance.params).unwrap();
        let mut set = 0;
        for &(param, theirs) in &darktable {
            let ours = params.get(param).unwrap();
            set += usize::from(instance.params.get(param).is_some());
            match (ours, theirs) {
                (Value::Float(a), Value::Float(b)) if a.to_bits() != b.to_bits() => {
                    let p = PARAMETERS.iter().find(|p| p.param == param).unwrap();
                    assert!(
                        a.to_bits().abs_diff(b.to_bits()) == 1 && !reachable(p, b),
                        "{name}: {param} is {a}, darktable's {b}"
                    );
                    // darktable's own slider reaches it from the value it shows. None of
                    // the three has a soft range, so the visible range is the schema's.
                    let Kind::Float { min, max, .. } = schema.param(param).unwrap().kind else {
                        panic!("{param} is a float");
                    };
                    let stored =
                        stored_by_darktable(p, (min, max), decimals(p.control), p.shown(b));
                    assert_eq!(stored.to_bits(), b.to_bits(), "{name}: {param}");
                    missed.push((name, param));
                }
                (a, b) => assert_eq!(a, b, "{name}: {param}"),
            }
        }

        let prepared = AgxOperation
            .prepare(&params, &image, &ImageDesc::rgba(1, 1))
            .unwrap();
        let theirs = golden::ports::kernel_inputs(case).unwrap().unwrap();
        assert_eq!(
            prepared.passes[0].tables, theirs.tables,
            "{name}: its tables"
        );
        let ours = sweep_with(&mut bench, instance.params.clone());
        let mut direct = ParamValues::new();
        for &(param, value) in &darktable {
            direct.set(param, value);
        }
        let direct = sweep_with(&mut bench, direct);
        if missed.iter().any(|(case, _)| *case == name) {
            let words = |b: &[u8]| b.as_chunks::<4>().0.to_vec();
            let worst = words(&prepared.passes[0].params)
                .into_iter()
                .zip(words(&theirs.params))
                .map(|(a, b)| {
                    let (a, b) = (f32::from_le_bytes(a), f32::from_le_bytes(b));
                    if a == b { 0.0 } else { (a - b).abs() / b.abs() }
                })
                .fold(0.0_f32, f32::max);
            let step = jump(&direct.pixels, &ours.pixels, |_| true);
            println!(
                "A4 {name}: {set} of {} parameters off their defaults, all but the three the binding's value / factor misses darktable's to the bit; the kernel's uniforms at most {worst:.1e} from the case's; on the sweep ΔE2000 mean {:.1e}, max {:.1e}",
                darktable.len(),
                step.mean,
                step.max
            );
            assert!(worst < 1e-6, "{name}: the kernel's uniforms, {worst:e}");
            assert!(step.max < 1e-3, "{name}: on the sweep, {step}");
        } else {
            assert_eq!(
                prepared.passes[0].params, theirs.params,
                "{name}: the kernel's uniforms"
            );
            assert!(exact(&ours, &direct), "{name}: on the sweep");
            println!(
                "A4 {name}: {set} of {} parameters off their defaults, every one darktable's to the bit, and the kernel's uniforms and the sweep's frame too",
                darktable.len()
            );
        }
    }
    assert_eq!(missed, UNREACHABLE);
}

// A5: the pivot in EV.

/// A5: the pivot is stored in EV and the range moves under it: the binding writes
/// `curve_pivot_x` as darktable's `_update_pivot_x` writes it when the range moves, so
/// the pivot keeps its exposure, inside the range, the last one clamped to white as the
/// range moves under it (A7), and the frame is the one darktable's parameters set
/// directly give, byte for byte.
#[test]
fn a5_the_pivot_keeps_its_exposure_as_the_range_moves() {
    let (picture, image) = ramp();
    let mut bench = Bench::picture(&picture, image.clone(), Pipe::Export);
    for (pivot, black, white) in [
        (0.0, -10.0, 8.0),
        (1.0, -6.0, 6.5),
        (-2.0, -12.0, 3.0),
        (5.0, -10.0, 4.0),
    ] {
        let set = [(PIVOT, pivot), (BLACK, black), (WHITE, white)];
        let edit = edit_of(&set);
        let ours = bench.edit(&edit, Frame);
        // darktable's parameters for the same range, its pivot kept at its exposure.
        let expected = (pivot.clamp(black, white) - black) / (white - black);
        let mut direct = bench.instances(&Edit::default(), Frame);
        let agx = direct.iter_mut().find(|i| id(i) == AGX).unwrap();
        agx.params = ParamValues::new();
        agx.params.set(PIVOT, Value::Float(expected));
        agx.params.set(BLACK, Value::Float(black));
        agx.params.set(WHITE, Value::Float(white));
        let theirs = bench.render(&direct);
        assert!(exact(&ours, &theirs), "A5 at {set:?}");
    }
}

// A7: the pivot's range follows the input range.

/// A7: the pivot's range is the input range, as darktable's slider's (decision 30): a
/// black of −4 EV with the pivot at −5 EV stores the pivot at −4 EV, kept, in the same
/// change, and binds it there; a pivot of −5 EV is then refused; black back at −10 EV
/// leaves the pivot at −4 EV, as darktable's `_update_pivot_x` leaves it.
#[test]
fn a7_the_pivot_s_range_follows_the_input_range_and_a_move_clamps_it() {
    let number = ControlValue::Number;
    let mut edit = Edit::default();
    change(&mut edit, PIVOT, number(-5.0)).unwrap();
    edit.keep(&Agx, PIVOT, false).unwrap();
    let pivot = |edit: &Edit| {
        let values = edit.module(AGX).unwrap();
        (values.number(PIVOT), values.mark(PIVOT))
    };
    assert_eq!(pivot(&edit), (Some(-5.0), None));
    change(&mut edit, BLACK, number(-4.0)).unwrap();
    assert_eq!(pivot(&edit), (Some(-4.0), Some(Mark::Kept)));
    let photo = common::photo();
    let instances = edit::instances(&edit, OPERATIONS, MODULES, &photo)
        .all()
        .unwrap();
    let agx = instances.iter().find(|i| id(i) == AGX).unwrap();
    assert_eq!(agx.params.get(PIVOT), Some(Value::Float(0.0)));
    let refused = change(&mut edit, PIVOT, number(-5.0)).unwrap_err();
    assert_eq!(
        refused,
        EditError::Control {
            module: AGX,
            source: ControlError::OutOfRange {
                control: PIVOT,
                value: -5.0,
                min: -4.0,
                max: 6.5
            }
        }
    );
    assert_eq!(pivot(&edit), (Some(-4.0), Some(Mark::Kept)));
    change(&mut edit, BLACK, number(-10.0)).unwrap();
    assert_eq!(pivot(&edit), (Some(-4.0), Some(Mark::Kept)));
}

// A6: what is not ported.

/// A6: the base primaries' export profile and dynamic range scaling off its default are
/// refused as not ported, the edit as it was; the export profile's option is listed, and
/// dynamic range scaling is reported unavailable, as every unported control is.
#[test]
fn a6_what_is_not_ported_is_refused() {
    let mut edit = Edit::default();
    let refused = |result: Result<_, EditError>| match result {
        Err(EditError::Control {
            module: AGX,
            source,
        }) => source,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        refused(edit.set_control(
            &Agx,
            BASE_PRIMARIES,
            ControlValue::Choice(0),
            &common::photo(),
        )),
        ControlError::Unported {
            control: BASE_PRIMARIES,
            option: "export_profile"
        }
    );
    assert_eq!(
        refused(edit.set_control(&Agx, SCALING, ControlValue::Number(20.0), &common::photo(),)),
        ControlError::NotPorted { control: SCALING }
    );
    assert_eq!(edit, Edit::default());
    assert!(Agx.definition().control(SCALING).unwrap().unported);
}
