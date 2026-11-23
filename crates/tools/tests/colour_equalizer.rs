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

//! The Colour equalizer's invariants (`docs/tools/colour-equalizer.md`, Invariants), one
//! test per row, each named after its row: a switched tool's (decision 32), H0 off, H1
//! the first step, on at darktable's defaults, H2 moved and reset, H3 the order, H4 the
//! darktable match and H6 on at darktable's defaults on the CPU; H5, on a layer, waits for
//! Phase 4. The synthetic ramp and sweep run on CI's software GPUs, H4 and H6 on the CPU.
//! The raw.pixls.us corpus runs locally, with `APERTRUST_RAW_CORPUS` naming it, and
//! darktable's chart with `APERTRUST_TEST_IMAGES`:
//!
//! ```sh
//! cargo test --release -p apertrust-tools --test colour_equalizer -- --ignored --nocapture
//! ```
//!
//! Synthetic images enter after demosaic, already in the working space: their rows plan
//! the operations from exposure on and leave colorin out. Every row reads the frame,
//! colorout's output before it is rounded to 8 bits, clipped to 0 to 1 as a PNG holds it.

#[expect(
    dead_code,
    reason = "the Colour equalizer's rows read neither the extended ramp nor a luminance"
)]
mod common;
mod corpus;
#[expect(
    dead_code,
    reason = "the Colour equalizer has no earlier version, so no row reads an older edit"
)]
mod switched;

use apertrust_engine::edit::{self, Edit};
use apertrust_engine::pipeline::Pipe;
use apertrust_operations::OPERATIONS;
use apertrust_operations::colorequal::ColorEqual;
use apertrust_plugin::{ImageContext, ImageDesc, Operation, Value};
use apertrust_tools::colour_equalizer::{ColourEqualizer, PARAMETERS, USE_FILTER};
use apertrust_tools::{ControlValue, TOOLS};
use golden::synthetic;

use common::{Bench, Picture, id, jump, sweep};
use switched::{Frame, Rows};

const COLOR_EQUAL: &str = "colorequal";

/// The ramp (`docs/tools/README.md`, Test images): neutral greys one per 1/16 EV from
/// 2⁻¹⁶ to 2⁴, then RGB 0.18. One row, over a colour raw's context with an exposure bias
/// of −0.67 EV.
fn ramp() -> (Picture, ImageContext) {
    common::ramp(&[0.18], -0.67)
}

/// The sweep's saturation at pixel `index`: 1 is a pure hue with one channel at 0, the
/// gamut's edge.
fn saturation(index: usize) -> f32 {
    synthetic::SATURATIONS[(index / synthetic::HUES) % synthetic::SATURATIONS.len()]
}

/// The rows every switched tool's table has the same way ([`switched::Rows`]).
const ROWS: Rows = Rows {
    tool: &ColourEqualizer,
    letter: "H",
};

// H0: off.

#[test]
fn h0_off_renders_as_no_edit() {
    ROWS.off_renders_as_no_edit(ramp());
}

/// H0 on every corpus raw.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn h0_on_the_corpus() {
    ROWS.off_on_the_corpus();
}

// H1: the first step, on at darktable's defaults, measured and reported without a bound
// (decision 22).

/// The first step's jump on the ramp and on the sweep: all of it, inside the gamut (a
/// saturation of at most 1) and within 80% of it.
#[test]
fn h1_the_first_step_is_measured() {
    let jumps = |(picture, image): (Picture, ImageContext)| {
        let mut bench = Bench::picture(&picture, image, Pipe::Export);
        let untouched = bench.edit(&Edit::default(), Frame);
        let first = ROWS.on_at_defaults(&bench);
        let first = bench.render(&first);
        let of = |limit: f32| jump(&untouched.pixels, &first.pixels, |i| saturation(i) <= limit);
        [of(f32::INFINITY), of(1.0), of(0.8)]
    };
    let [greys, ..] = jumps(ramp());
    let [whole, gamut, inside] = jumps(sweep(-0.67));
    for (what, step) in [
        ("on the ramp", greys),
        ("on the sweep", whole),
        ("on the sweep, inside the gamut", gamut),
        ("on the sweep, within 80% of the gamut's saturation", inside),
    ] {
        println!("H1 {what}: {step}");
        assert!(
            step.mean.is_finite() && step.max.is_finite(),
            "H1 {what}: {step}"
        );
    }
    // No value of the colour equalizer's is an identity (docs/ports/colorequal.md, Tool
    // binding notes): on at its defaults it changes the sweep's colours.
    assert!(whole.max > 0.0, "H1 on the sweep: {whole}");
}

/// H1 on darktable's chart, `mire1.cr2`.
#[test]
#[ignore = "needs darktable's integration images (APERTRUST_TEST_IMAGES)"]
fn h1_on_the_chart() {
    ROWS.first_step_on_the_chart();
}

/// H1 on every corpus raw the pipeline takes.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn h1_on_the_corpus() {
    ROWS.first_step_on_the_corpus();
}

// H2: each control moved, then reset.

/// Moves of every kind of control: a write of a node at its default, which turns the
/// tool on, a hue node, a saturation node, a brightness node with node placement, the
/// hue curve with the white level, and the guided filter off, then its settings with it
/// on. Each changes the frame.
const MOVES: [&[(&str, f32)]; 7] = [
    &[("hue_red", 0.0)],
    &[("hue_green", 30.0)],
    &[("sat_blue", 40.0)],
    &[("bright_orange", 50.0), ("hue_shift", 10.0)],
    &[
        ("hue_yellow", -20.0),
        ("smoothing_hue", 0.5),
        ("white_level", -1.0),
    ],
    &[(USE_FILTER, 0.0), ("sat_red", -50.0)],
    &[
        ("sat_cyan", 60.0),
        ("chroma_size", 3.0),
        ("threshold", 5.0),
        ("contrast", 0.5),
        ("param_size", 8.0),
    ],
];

#[test]
fn h2_a_control_moved_and_reset_renders_as_on_and_the_tool_reset_as_no_edit() {
    ROWS.moved_and_reset(&MOVES);
}

// H3: the order controls were set in.

const PAIRS: [[(&str, f32); 2]; 2] = [
    [("hue_green", 30.0), ("sat_blue", 40.0)],
    [("bright_orange", 50.0), (USE_FILTER, 0.0)],
];

#[test]
fn h3_controls_render_the_same_whichever_came_first() {
    ROWS.order(&PAIRS, sweep(-0.67));
}

/// H2 and H3 on the 90D, through the viewport's frame and the export's.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn h2_and_h3_on_the_corpus() {
    ROWS.moved_and_order_on_the_corpus(&MOVES, &PAIRS);
}

// H4: darktable's golden cases.

/// H4: the tool's controls set to the values of each golden case, darktable's tests
/// 0158-coloreq-no-guided-filter and 0159-coloreq-guided-filter, migrated to version 4 as
/// darktable's `legacy_params` does, each shown as its slider shows it, unrounded, bind
/// to the case's parameters within one f32 step, the round trip's rounding, and the
/// operation prepares from them the passes and uniforms the golden harness checks against
/// darktable's render, within that step. So the tool renders those values as the cases
/// measure the port (`docs/ports/colorequal.md`, Golden results).
#[test]
fn h4_the_golden_cases_values_bind_to_darktable_s_parameters() {
    let schema = ColorEqual.schema();
    let image = ImageContext::full(64, 48);
    for name in ["colorequal", "colorequal-no-filter"] {
        let case = golden::cases::find(name).unwrap();
        let theirs = schema
            .resolve(&golden::ports::plan_values(case).unwrap().unwrap())
            .unwrap();
        let mut edit = Edit::default();
        for p in &PARAMETERS {
            let Some(Value::Float(darktable)) = theirs.get(p.param) else {
                panic!("{name}: {}", p.param);
            };
            let value = ControlValue::Number(p.shown(darktable));
            edit.set_control(&ColourEqualizer, p.control, value, &common::photo())
                .unwrap();
        }
        let Some(Value::Bool(filter)) = theirs.get(USE_FILTER) else {
            panic!("{name}: {USE_FILTER}");
        };
        edit.set_control(
            &ColourEqualizer,
            USE_FILTER,
            ControlValue::Bool(filter),
            &common::photo(),
        )
        .unwrap();

        let instances = edit::instances(&edit, OPERATIONS, TOOLS, &image)
            .all()
            .unwrap();
        let instance = instances.iter().find(|i| id(i) == COLOR_EQUAL).unwrap();
        let ours = schema.resolve(&instance.params).unwrap();
        let mut off = 0;
        for param in schema.params {
            match (ours.get(param.name), theirs.get(param.name)) {
                (Some(Value::Float(a)), Some(Value::Float(b))) => {
                    assert!(
                        (a - b).abs() <= f32::EPSILON * b.abs().max(1.0),
                        "{name}: {} is {a}, darktable's {b}",
                        param.name
                    );
                    off += usize::from(a != b);
                }
                (a, b) => assert_eq!(a, b, "{name}: {}", param.name),
            }
        }

        let input = ImageDesc::rgba(64, 48);
        let ours = ColorEqual.prepare(&ours, &image, &input).unwrap();
        let theirs = ColorEqual.prepare(&theirs, &image, &input).unwrap();
        assert_eq!(ours.passes.len(), theirs.passes.len(), "{name}");
        let mut worst = 0.0_f32;
        for (a, b) in ours.passes.iter().zip(&theirs.passes) {
            assert_eq!(a.entry_point, b.entry_point, "{name}");
            assert_eq!(a.params.len(), b.params.len(), "{name}: {}", a.entry_point);
            for (x, y) in a
                .params
                .as_chunks::<4>()
                .0
                .iter()
                .zip(b.params.as_chunks::<4>().0)
            {
                let (x, y) = (f32::from_le_bytes(*x), f32::from_le_bytes(*y));
                worst = worst.max((x - y).abs() / y.abs().max(1.0));
            }
        }
        assert!(
            worst <= 4.0 * f32::EPSILON,
            "{name}: uniforms {worst:e} apart"
        );
        println!(
            "H4 {name}: {off} parameters one f32 step from darktable's, the uniforms at most {worst:.1e} apart"
        );
    }
}

// H6: on at darktable's defaults, on the CPU.

/// The tool's defaults, each made explicit, bind colorequal's own, and the tool on at
/// its defaults plans it with no parameter set, darktable's new instance; off, it plans
/// none.
#[test]
fn h6_on_at_darktable_s_defaults_binds_the_operation_s_defaults() {
    switched::on_at_defaults_holds(&ColourEqualizer, COLOR_EQUAL);
}
