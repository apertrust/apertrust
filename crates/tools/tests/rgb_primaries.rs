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

//! RGB primaries' invariants (`docs/tools/rgb-primaries.md`, Invariants), one test per
//! row, each named after its row: a switched tool's (decision 32), P0 off, P1 the
//! first step, on at darktable's defaults, P2 moved and reset, P3 the order, P4 the
//! darktable match, P7 on at darktable's defaults on the CPU and P8 version 1's edits; P5,
//! on a layer, waits for Phase 4; and P6, the tint's hue at a purity of 0. The synthetic
//! ramp and sweep run on CI's software GPUs, P4, P7 and P8 on the CPU. The raw.pixls.us
//! corpus runs locally, with `APERTRUST_RAW_CORPUS` naming it, and darktable's chart with
//! `APERTRUST_TEST_IMAGES`:
//!
//! ```sh
//! cargo test --release -p apertrust-tools --test rgb_primaries -- --ignored --nocapture
//! ```
//!
//! Synthetic images enter after demosaic, already in the working space: their rows plan
//! the operations from exposure on and leave colorin out. Every row reads the frame,
//! colorout's output before it is rounded to 8 bits, clipped to 0 to 1 as a PNG holds it.

#[expect(
    dead_code,
    reason = "RGB primaries' rows read neither the extended ramp nor a luminance"
)]
mod common;
mod corpus;
mod switched;
#[expect(
    dead_code,
    reason = "only colour balance rgb and the tone equalizer measure how far apart two words are"
)]
mod uniforms;

use apertrust_engine::edit::{self, Edit};
use apertrust_engine::pipeline::Pipe;
use apertrust_operations::OPERATIONS;
use apertrust_operations::primaries::{PARAMS_SIZE, Primaries};
use apertrust_plugin::{ImageContext, ImageDesc, Operation, Value};
use apertrust_tools::rgb_primaries::{PARAMETERS, RgbPrimaries};
use apertrust_tools::{ControlValue, TOOLS};
use golden::synthetic;

use common::{Bench, Picture, id, jump, sweep};
use switched::{Frame, Rows};
use uniforms::words;

const PRIMARIES: &str = "primaries";

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
    tool: &RgbPrimaries,
    letter: "P",
};

// P0: off.

#[test]
fn p0_off_renders_as_no_edit() {
    ROWS.off_renders_as_no_edit(ramp());
}

/// P0 on every corpus raw.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn p0_on_the_corpus() {
    ROWS.off_on_the_corpus();
}

// P1: the first step, on at darktable's defaults, measured and reported without a bound
// (decision 22).

/// The first step's jump on the ramp and on the sweep: all of it, inside the gamut (a
/// saturation of at most 1) and within 80% of it.
#[test]
fn p1_the_first_step_is_measured() {
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
        println!("P1 {what}: {step}");
        assert!(
            step.pixels > 0 && step.mean.is_finite() && step.max.is_finite(),
            "P1 {what}: {step}"
        );
    }
}

/// P1 on darktable's chart, `mire1.cr2`.
#[test]
#[ignore = "needs darktable's integration images (APERTRUST_TEST_IMAGES)"]
fn p1_on_the_chart() {
    ROWS.first_step_on_the_chart();
}

/// P1 on every corpus raw the pipeline takes.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn p1_on_the_corpus() {
    ROWS.first_step_on_the_corpus();
}

// P2: each control moved, then reset.

/// Moves of every control: a write of a hue at its default, which turns the tool on, a
/// primary's hue, a primary's purity, both of one primary, the tint, and all eight at
/// test 0152's values as its sliders show them. Each changes the frame.
const MOVES: [&[(&str, f32)]; 6] = [
    &[("red_hue", 0.0)],
    &[("red_hue", 10.0)],
    &[("green_purity", 25.0)],
    &[("blue_hue", -15.0), ("blue_purity", -30.0)],
    &[
        ("achromatic_tint_hue", 120.0),
        ("achromatic_tint_purity", 10.0),
    ],
    &[
        ("red_hue", 9.5),
        ("red_purity", -15.7),
        ("green_hue", -8.2),
        ("green_purity", 23.5),
        ("blue_hue", 13.1),
        ("blue_purity", -18.0),
        ("achromatic_tint_hue", 33.2),
        ("achromatic_tint_purity", 13.4),
    ],
];

#[test]
fn p2_a_control_moved_and_reset_renders_as_on_and_the_tool_reset_as_no_edit() {
    ROWS.moved_and_reset(&MOVES, sweep(-0.67));
}

// P3: the order controls were set in.

const PAIRS: [[(&str, f32); 2]; 2] = [
    [("red_hue", 9.5), ("achromatic_tint_purity", 13.4)],
    [("green_purity", 23.5), ("blue_hue", 13.1)],
];

#[test]
fn p3_controls_render_the_same_whichever_came_first() {
    ROWS.order(&PAIRS, sweep(-0.67));
}

/// P2 and P3 on the 90D, through the viewport's frame and the export's.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn p2_and_p3_on_the_corpus() {
    ROWS.moved_and_order_on_the_corpus(&MOVES, &PAIRS);
}

// P4: darktable's golden case.

/// How far apart P4 lets a matrix entry of the kernel's be from the golden case's:
/// two f32 steps of an entry of 1. A setting one step from darktable's moves an entry by
/// about that step times the entry's derivative, which is below 1 here.
const MATRIX: f32 = 2.4e-7;

/// P4: the tool's controls set to the values of the `primaries` golden case,
/// darktable's test 0152-rgb-primaries, each field × factor + offset, unrounded, bind to
/// the case's parameters within one f32 step, the round trip's rounding, and, through the
/// operation's `prepare`, to the kernel inputs the golden harness checks against
/// darktable's render. So the tool renders those values as the case measures the port:
/// a ΔE2000 mean of 0.0000, max 0.0002 (`docs/ports/primaries.md`).
#[test]
fn p4_the_golden_case_s_values_bind_to_darktable_s_parameters() {
    let schema = Primaries.schema();
    let image = ImageContext::full(1, 1);
    let case = golden::cases::find(PRIMARIES).unwrap();
    let darktable = golden::xmp::module_params(case.xmp, PRIMARIES).unwrap();
    // darktable's fields, in its order, are the schema's.
    let fields = words(&darktable.bytes);
    let field = |param: &str| {
        let i = schema.params.iter().position(|p| p.name == param).unwrap();
        f32::from_le_bytes(fields[i])
    };
    let mut edit = Edit::default();
    for p in &PARAMETERS {
        let value = ControlValue::Number(p.shown(field(p.param)));
        edit.set_control(&RgbPrimaries, p.control, value, &common::photo())
            .unwrap();
    }

    let instances = edit::instances(&edit, OPERATIONS, TOOLS, &image)
        .all()
        .unwrap();
    let instance = instances.iter().find(|i| id(i) == PRIMARIES).unwrap();
    let params = schema.resolve(&instance.params).unwrap();
    for p in &PARAMETERS {
        let Some(Value::Float(bound)) = params.get(p.param) else {
            panic!("{}", p.param);
        };
        let expected = field(p.param);
        assert!(
            (bound - expected).abs() <= f32::EPSILON * expected.abs(),
            "{} is {bound}, darktable's {expected}",
            p.param
        );
    }

    let prepared = Primaries
        .prepare(&params, &image, &ImageDesc::rgba(1, 1))
        .unwrap();
    let ours = words(&prepared.passes[0].params);
    let theirs = words(&golden::ports::kernel_inputs(case).unwrap().unwrap().params);
    assert_eq!(
        (ours.len(), theirs.len()),
        (PARAMS_SIZE / 4, PARAMS_SIZE / 4)
    );
    let float = |word: [u8; 4]| f32::from_le_bytes(word);
    // The matrix by rows, its fourth column unused; then the hues and purities; then
    // what only the derivative entry points read, which no setting moves.
    let mut matrix = 0.0_f32;
    for (i, (a, b)) in ours.iter().zip(&theirs).enumerate() {
        let (a, b) = (*a, *b);
        match i {
            0..12 => {
                let apart = (float(a) - float(b)).abs();
                assert!(apart <= MATRIX, "matrix word {i}, {apart:e} apart");
                matrix = matrix.max(apart);
            }
            12..20 => assert!(
                (float(a) - float(b)).abs() <= f32::EPSILON * float(b).abs(),
                "setting word {i}"
            ),
            _ => assert_eq!(a, b, "uniform word {i}"),
        }
    }
    println!("P4: the kernel's matrix at most {matrix:.1e} from the golden case's");
}

// P6: the tint's hue does nothing while its purity is 0.

/// The tint's hue moved while its purity is 0 turns rgb primaries on, as any control's
/// write does, and renders exactly the first step's pixels, the tool on at its
/// defaults: the white moves by its purity, so the matrix is the defaults'. It stays
/// enabled there, as darktable's does.
#[test]
fn p6_the_tint_s_hue_at_purity_0_renders_as_the_first_step() {
    let (picture, image) = sweep(-0.67);
    let mut bench = Bench::picture(&picture, image, Pipe::Export);
    let first = ROWS.on_at_defaults(&bench);
    let first = bench.render(&first);
    for hue in [-180.0, -33.2, 90.0, 180.0] {
        let hued = ROWS.set(&mut bench, &[("achromatic_tint_hue", hue)]);
        assert!(
            common::same_pixels(&first, &hued),
            "the tint's hue at {hue}°"
        );
    }
}

// P7: on at darktable's defaults, on the CPU.

/// The tool's defaults, each made explicit, bind rgb primaries' own, and the tool on
/// at its defaults plans it with no parameter set, darktable's new instance; off, it plans
/// none.
#[test]
fn p7_on_at_darktable_s_defaults_binds_the_operation_s_defaults() {
    switched::on_at_defaults_holds(&RgbPrimaries, PRIMARIES);
}

// P8: version 1's edits.

/// Edits version 1 saved, read as a photo opens them, bind the parameters version 1
/// bound: rgb primaries planned where a value was off its default, with what the binding
/// gives those values, and not planned, the tool off, where none was.
#[test]
fn p8_version_1_s_edits_bind_what_they_bound() {
    let image = common::photo();
    for (saved, values) in [
        (&[("red_hue", "10")][..], &[("red_hue", 10.0)][..]),
        (
            &[
                ("achromatic_tint_hue", "120"),
                ("achromatic_tint_purity", "10"),
                ("green_purity", "0"),
            ],
            &[
                ("achromatic_tint_hue", 120.0),
                ("achromatic_tint_purity", 10.0),
            ],
        ),
        (
            &[("achromatic_tint_hue", "-33.2")],
            &[("achromatic_tint_hue", -33.2)],
        ),
    ] {
        let edit = switched::version_1(PRIMARIES, saved);
        assert!(edit.is_on(&RgbPrimaries), "{saved:?}");
        let params = switched::bound(&edit, PRIMARIES, &image).expect("planned");
        let then = switched::bound(&ROWS.edit_of(values), PRIMARIES, &image).unwrap();
        assert_eq!(params, then, "{saved:?}");
        assert!(params.iter().next().is_some(), "{saved:?}");
    }
    for saved in [&[("red_purity", "0")][..], &[]] {
        let edit = switched::version_1(PRIMARIES, saved);
        assert!(!edit.is_on(&RgbPrimaries), "{saved:?}");
        assert!(edit.is_empty(), "{saved:?}");
        assert!(
            switched::bound(&edit, PRIMARIES, &image).is_none(),
            "{saved:?}"
        );
    }
}
