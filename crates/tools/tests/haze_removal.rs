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

//! Haze removal's invariants (`docs/tools/haze-removal.md`, Invariants), one test per
//! row, each named after its row: a switched tool's (decision 32), Z0 off, Z1 the
//! first step, on at darktable's defaults, Z2 moved and reset, Z3 the order, Z4 the
//! darktable match, Z7 on at darktable's defaults on the CPU and Z8 version 1's edits; Z5,
//! on a layer, waits for Phase 4, and Z6 is withdrawn. The synthetic ramp and sweep run on
//! CI's software GPUs, Z4, Z7 and Z8 on the CPU. The raw.pixls.us corpus runs locally,
//! with `APERTRUST_RAW_CORPUS` naming it, and darktable's chart with
//! `APERTRUST_TEST_IMAGES`:
//!
//! ```sh
//! cargo test --release -p apertrust-tools --test haze_removal -- --ignored --nocapture
//! ```
//!
//! Synthetic images enter after demosaic, already in the working space: their rows plan
//! the operations from haze removal on and leave colorin out. Every row reads the frame,
//! colorout's output before it is rounded to 8 bits, clipped to 0 to 1 as a PNG holds it.

#[expect(
    dead_code,
    reason = "Haze removal's rows read neither the extended ramp nor a luminance"
)]
mod common;
mod corpus;
mod switched;

use apertrust_engine::edit::{self, Edit};
use apertrust_engine::pipeline::Pipe;
use apertrust_operations::OPERATIONS;
use apertrust_operations::hazeremoval::{self, Settings};
use apertrust_plugin::{ImageContext, ImageDesc, Operation, Value};
use apertrust_tools::haze_removal::{DISTANCE, HazeRemoval, STRENGTH};
use apertrust_tools::{ControlValue, TOOLS};

use common::{Bench, Picture, id, jump, sweep};
use switched::{Frame, Rows};

const HAZEREMOVAL: &str = "hazeremoval";

/// The ramp (`docs/tools/README.md`, Test images): neutral greys one per 1/16 EV from
/// 2⁻¹⁶ to 2⁴, then RGB 0.18. One row, over a colour raw's context with an exposure bias
/// of −0.67 EV.
fn ramp() -> (Picture, ImageContext) {
    common::ramp(&[0.18], -0.67)
}

/// The rows every switched tool's table has the same way ([`switched::Rows`]).
const ROWS: Rows = Rows {
    tool: &HazeRemoval,
    letter: "Z",
};

// Z0: off.

#[test]
fn z0_off_renders_as_no_edit() {
    ROWS.off_renders_as_no_edit(ramp());
}

/// Z0 on every corpus raw.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn z0_on_the_corpus() {
    ROWS.off_on_the_corpus();
}

// Z1: the first step, on at darktable's defaults, measured and reported without a bound
// (decision 22).

/// The first step's jump on the ramp and on the sweep: darktable's new instance removing
/// haze at a strength of 0.2.
#[test]
fn z1_the_first_step_is_measured() {
    for (what, (picture, image)) in [("on the ramp", ramp()), ("on the sweep", sweep(-0.67))] {
        let mut bench = Bench::picture(&picture, image, Pipe::Export);
        let untouched = bench.edit(&Edit::default(), Frame);
        let first = ROWS.on_at_defaults(&bench);
        let first = bench.render(&first);
        let step = jump(&untouched.pixels, &first.pixels, |_| true);
        let moved = untouched
            .pixels
            .iter()
            .zip(&first.pixels)
            .filter(|(a, b)| a.map(f32::to_bits) != b.map(f32::to_bits))
            .count();
        println!(
            "Z1 {what}: {step} (mean {:.1e}, max {:.1e}); {moved} of {} pixels not \
             byte-identical",
            step.mean, step.max, step.pixels
        );
        assert!(
            step.pixels > 0 && step.mean.is_finite() && step.max.is_finite(),
            "Z1 {what}: {step}"
        );
    }
}

/// Z1 on darktable's chart, `mire1.cr2`.
#[test]
#[ignore = "needs darktable's integration images (APERTRUST_TEST_IMAGES)"]
fn z1_on_the_chart() {
    ROWS.first_step_on_the_chart();
}

/// Z1 on every corpus raw the pipeline takes.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn z1_on_the_corpus() {
    ROWS.first_step_on_the_corpus();
}

// Z2: each control moved, then reset.

/// Moves of every control: the strength removing haze and adding it, a write of it at
/// darktable's 0.2, which turns the tool on, the distance alone, and both at test
/// 0166's values. Each changes the frame.
const MOVES: [&[(&str, f32)]; 6] = [
    &[(STRENGTH, 0.5)],
    &[(STRENGTH, -0.3)],
    &[(STRENGTH, 0.2)],
    &[(DISTANCE, 0.8)],
    &[(STRENGTH, 0.4), (DISTANCE, 0.8)],
    &[(STRENGTH, 0.9), (DISTANCE, 0.8)],
];

#[test]
fn z2_a_control_moved_and_reset_renders_as_on_and_the_tool_reset_as_no_edit() {
    ROWS.moved_and_reset(&MOVES);
}

// Z3: the order controls were set in.

const PAIRS: [[(&str, f32); 2]; 2] = [
    [(STRENGTH, 0.6), (DISTANCE, 0.5)],
    [(STRENGTH, -0.4), (DISTANCE, 0.1)],
];

#[test]
fn z3_controls_render_the_same_whichever_came_first() {
    ROWS.order(&PAIRS, sweep(-0.67));
}

/// Z2 and Z3 on the 90D, through the viewport's frame and the export's.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn z2_and_z3_on_the_corpus() {
    ROWS.moved_and_order_on_the_corpus(&MOVES, &PAIRS);
}

// Z4: darktable's golden cases.

/// Z4: the tool's controls set to the values of each `hazeremoval` golden case,
/// darktable's tests 0026 and 0166, bind the case's parameters bit for bit, and through
/// the operation's `prepare` give the very passes the golden harness checks against
/// darktable's render. So the tool renders those values as the cases measure the port.
#[test]
fn z4_the_golden_cases_values_bind_to_darktable_s_parameters() {
    let schema = hazeremoval::HazeRemoval.schema();
    // The cases' image, mire1.cr2's demosaiced size, at full resolution.
    let (width, height) = (3908, 2602);
    let image = ImageContext::full(width, height);
    let desc = ImageDesc::rgba(width, height);
    let cases: Vec<_> = golden::cases::CASES
        .iter()
        .filter(|case| case.operation == HAZEREMOVAL)
        .collect();
    assert_eq!(cases.len(), 2);
    for case in cases {
        let darktable = golden::ports::plan_values(case).unwrap().unwrap();
        let darktable = schema.resolve(&darktable).unwrap();
        let mut edit = Edit::default();
        for control in [STRENGTH, DISTANCE] {
            let Some(Value::Float(value)) = darktable.get(control) else {
                panic!("{}: {control}", case.name);
            };
            edit.set_control(
                &HazeRemoval,
                control,
                ControlValue::Number(value),
                &common::photo(),
            )
            .unwrap();
        }
        let instances = edit::instances(&edit, OPERATIONS, TOOLS, &image)
            .all()
            .unwrap();
        let instance = instances.iter().find(|i| id(i) == HAZEREMOVAL).unwrap();
        let ours = schema.resolve(&instance.params).unwrap();
        for control in [STRENGTH, DISTANCE] {
            let bits = |values: &apertrust_plugin::ParamValues| match values.get(control) {
                Some(Value::Float(value)) => value.to_bits(),
                other => panic!("{}: {control} is {other:?}", case.name),
            };
            assert_eq!(bits(&ours), bits(&darktable), "{}: {control}", case.name);
        }
        let passes = |values| {
            hazeremoval::HazeRemoval
                .prepare(values, &image, &desc)
                .unwrap()
                .passes
                .into_iter()
                .map(|pass| (pass.entry_point, pass.params))
                .collect::<Vec<_>>()
        };
        assert_eq!(passes(&ours), passes(&darktable), "{}", case.name);
        println!(
            "Z4 {}: strength {:?}, distance {:?}, bit for bit",
            case.name,
            ours.get(STRENGTH),
            ours.get(DISTANCE)
        );
    }
}

// Z7: on at darktable's defaults, on the CPU.

/// The tool's defaults, each made explicit, bind haze removal's own, and the tool on
/// at its defaults plans it with no parameter set, darktable's new instance; off, it
/// plans none.
#[test]
fn z7_on_at_darktable_s_defaults_binds_the_operation_s_defaults() {
    switched::on_at_defaults_holds(&HazeRemoval, HAZEREMOVAL);
}

// Z8: version 1's edits.

/// Edits version 1 saved, read as a photo opens them, bind the parameters version 1
/// bound: haze removal planned where the strength was off 0, version 1's untouched value,
/// with the strength version 1 always wrote, and not planned, the tool off, where it
/// was not.
#[test]
fn z8_version_1_s_edits_bind_what_they_bound() {
    let image = common::photo();
    for (saved, strength, distance) in [
        (&[(STRENGTH, "0.9"), (DISTANCE, "0.8")][..], 0.9, 0.8),
        (&[(STRENGTH, "-0.35")], -0.35, 0.2),
        (&[(STRENGTH, "0.2")], 0.2, 0.2),
    ] {
        let edit = switched::version_1(HAZEREMOVAL, saved);
        assert!(edit.is_on(&HazeRemoval), "{saved:?}");
        let params = switched::bound(&edit, HAZEREMOVAL, &image).expect("planned");
        let schema = apertrust_operations::operation(HAZEREMOVAL)
            .unwrap()
            .schema();
        let bound = hazeremoval::settings(&schema.resolve(&params).unwrap());
        assert_eq!(bound, Settings { strength, distance }, "{saved:?}");
    }
    for saved in [&[(DISTANCE, "0.8")][..], &[(STRENGTH, "0")], &[]] {
        let edit = switched::version_1(HAZEREMOVAL, saved);
        assert!(!edit.is_on(&HazeRemoval), "{saved:?}");
        assert!(edit.is_empty(), "{saved:?}");
        assert!(
            switched::bound(&edit, HAZEREMOVAL, &image).is_none(),
            "{saved:?}"
        );
    }
}
