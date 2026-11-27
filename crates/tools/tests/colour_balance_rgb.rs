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

//! Colour balance RGB's invariants (`docs/tools/colour-balance-rgb.md`, Invariants),
//! one test per row, each named after its row: a switched tool's (decision 32), B0
//! off, B1 the first step, on at darktable's defaults, B2 moved and reset, B3 the order,
//! B4 the darktable match, B6 on at darktable's defaults on the CPU, B7 version 1's
//! edits and B8 the pickers; B5, on a layer, waits for Phase 4. The synthetic ramp and
//! sweep run on CI's software GPUs, B4, B6, B7 and B8 on the CPU. The raw.pixls.us
//! corpus runs locally, with `APERTRUST_RAW_CORPUS` naming it, and darktable's chart
//! with `APERTRUST_TEST_IMAGES`:
//!
//! ```sh
//! cargo test --release -p apertrust-tools --test colour_balance_rgb -- --ignored --nocapture
//! ```
//!
//! Synthetic images enter after demosaic, already in the working space: their rows plan
//! the operations from exposure on and leave colorin out. Every row reads the frame,
//! colorout's output before it is rounded to 8 bits, clipped to 0 to 1 as a PNG holds it.

#[expect(
    dead_code,
    reason = "Colour balance RGB's rows read neither the extended ramp nor a luminance"
)]
mod common;
mod corpus;
mod switched;
mod uniforms;

use apertrust_engine::edit::{self, Edit};
use apertrust_engine::pipeline::Pipe;
use apertrust_operations::OPERATIONS;
use apertrust_operations::colorbalancergb::ColorBalanceRgb as ColourBalanceOperation;
use apertrust_plugin::{ImageContext, ImageDesc, Operation, Value};
use apertrust_tools::colour_balance_rgb::{ColourBalanceRgb, FORMULA, PARAMETERS};
use apertrust_tools::{ControlValue, TOOLS};
use golden::synthetic;

use common::{Bench, Picture, id, jump, sweep};
use switched::{Frame, Rows};
use uniforms::{apart, words};

const COLOR_BALANCE: &str = "colorbalancergb";

/// The ramp (`docs/tools/README.md`, Test images): neutral greys one per 1/16 EV from
/// 2⁻¹⁶ to 2⁴, then RGB 0.18 and 0.1744, darktable's fulcrum. One row, over a colour
/// raw's context with an exposure bias of −0.67 EV.
fn ramp() -> (Picture, ImageContext) {
    common::ramp(&[0.18, 0.1744], -0.67)
}

/// The sweep's saturation at pixel `index`: 1 is a pure hue with one channel at 0, the
/// gamut's edge.
fn saturation(index: usize) -> f32 {
    synthetic::SATURATIONS[(index / synthetic::HUES) % synthetic::SATURATIONS.len()]
}

/// The rows every switched tool's table has the same way ([`switched::Rows`]).
const ROWS: Rows = Rows {
    tool: &ColourBalanceRgb,
    letter: "B",
};

// B0: off.

#[test]
fn b0_off_renders_as_no_edit() {
    ROWS.off_renders_as_no_edit(ramp());
}

// B1: the first step, on at darktable's defaults, measured and reported without a bound
// (decision 22).

/// The first step's jump on the ramp and on the sweep: all of it, inside the gamut (a
/// saturation of at most 1) and within 80% of it.
#[test]
fn b1_the_first_step_is_measured() {
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
        println!("B1 {what}: {step}");
        assert!(
            step.pixels > 0 && step.mean.is_finite() && step.max.is_finite(),
            "B1 {what}: {step}"
        );
    }
}

/// B1 on darktable's chart, `mire1.cr2`.
#[test]
#[ignore = "needs darktable's integration images (APERTRUST_TEST_IMAGES)"]
fn b1_on_the_chart() {
    ROWS.first_step_on_the_chart();
}

/// B1 on every corpus raw the pipeline takes.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn b1_on_the_corpus() {
    ROWS.first_step_on_the_corpus();
}

// B2: each control moved, then reset.

/// Moves on every tab, the formula included, and a write of the contrast at its default,
/// which turns the tool on: each changes the frame, as the first step at least does.
const MOVES: [&[(&str, f32)]; 6] = [
    &[("contrast", 0.0)],
    &[("contrast", 20.0)],
    &[("brilliance_global", -20.0)],
    &[("global_h", 120.0), ("global_c", 5.0)],
    &[(FORMULA, 0.0)],
    &[
        ("vibrance", 30.0),
        ("shadows_y", 10.0),
        ("grey_fulcrum", 30.0),
        ("white_fulcrum", 1.0),
        (FORMULA, 0.0),
    ],
];

#[test]
fn b2_a_control_moved_and_reset_renders_as_on_and_the_tool_reset_as_no_edit() {
    ROWS.moved_and_reset(&MOVES, ramp());
}

// B3: the order controls were set in.

const PAIRS: [[(&str, f32); 2]; 2] = [
    [("contrast", 20.0), ("saturation_global", -30.0)],
    [(FORMULA, 0.0), ("brilliance_global", 10.0)],
];

#[test]
fn b3_controls_render_the_same_whichever_came_first() {
    ROWS.order(&PAIRS, sweep(-0.67));
}

/// B2 and B3 on the 90D, through the viewport's frame and the export's.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn b2_and_b3_on_the_corpus() {
    ROWS.moved_and_order_on_the_corpus(&MOVES, &PAIRS);
}

/// B0 on every corpus raw.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn b0_on_the_corpus() {
    ROWS.off_on_the_corpus();
}

// B4: darktable's golden cases.

/// How far apart B4 lets a uniform word be: two f32 steps, relative. A percentage over
/// 100 is within one step of darktable's parameter, and the setup may round once more.
const TWO_STEPS: f32 = 2.4e-7;

/// B4: the tool's controls set to the values of `colorbalancergb`'s golden cases,
/// darktable's tests 0083 (params version 4, JzAzBz) and 0093 (version 5, darktable
/// UCS), bind to the case's parameters and, through the operation's `prepare`, to the
/// kernel inputs the golden harness checks against darktable's render. So the tool
/// renders those values as the cases measure the port: ΔE2000 means of 0.0057 and 0.0018
/// (`docs/ports/colorbalancergb.md`).
#[test]
fn b4_the_golden_cases_values_bind_to_darktable_s_parameters() {
    let schema = ColourBalanceOperation.schema();
    let image = ImageContext::full(1, 1);
    for name in ["colorbalancergb", "colorbalancergb-ucs"] {
        let case = golden::cases::find(name).unwrap();
        let darktable = golden::xmp::module_params(case.xmp, COLOR_BALANCE).unwrap();
        // darktable's fields, in its order, are the schema's; a version before 5 has no
        // formula, and darktable reads it as JzAzBz.
        let fields = words(&darktable.bytes);
        let field = |param: &str| {
            let i = schema.params.iter().position(|p| p.name == param).unwrap();
            f32::from_le_bytes(fields[i])
        };
        let mut edit = Edit::default();
        for p in &PARAMETERS {
            let value = ControlValue::Number(field(p.param) * p.scale);
            edit.set_control(&ColourBalanceRgb, p.control, value, &common::photo())
                .unwrap();
        }
        let formula = fields.get(32).map_or(0, |word| u32::from_le_bytes(*word));
        edit.set_control(
            &ColourBalanceRgb,
            FORMULA,
            ControlValue::Choice(formula),
            &common::photo(),
        )
        .unwrap();

        let instances = edit::instances(&edit, OPERATIONS, TOOLS, &image)
            .all()
            .unwrap();
        let instance = instances.iter().find(|i| id(i) == COLOR_BALANCE).unwrap();
        let params = schema.resolve(&instance.params).unwrap();
        for p in &PARAMETERS {
            let Some(Value::Float(bound)) = params.get(p.param) else {
                panic!("{}", p.param);
            };
            let expected = field(p.param);
            assert!(
                (bound - expected).abs() <= f32::EPSILON * expected.abs(),
                "{name}: {} is {bound}, darktable's {expected}",
                p.param
            );
        }
        assert_eq!(params.get(FORMULA), Some(Value::Choice(formula)), "{name}");

        let prepared = ColourBalanceOperation
            .prepare(&params, &image, &ImageDesc::rgba(1, 1))
            .unwrap();
        let ours = &prepared.passes[0];
        let theirs = golden::ports::kernel_inputs(case).unwrap().unwrap();
        assert_eq!(ours.tables, theirs.tables, "{name}: the gamut table");
        assert_eq!(ours.params.len(), theirs.params.len());
        let mut worst = 0.0_f32;
        for (i, (a, b)) in words(&ours.params)
            .into_iter()
            .zip(words(&theirs.params))
            .enumerate()
        {
            let apart = apart(a, b);
            assert!(
                apart <= TWO_STEPS,
                "{name}: uniform word {i}, {apart:e} apart"
            );
            worst = worst.max(apart);
        }
        println!("B4 {name}: the kernel's uniforms at most {worst:.1e} from the golden case's");
    }
}

// B6: on at darktable's defaults, on the CPU.

/// The tool's defaults, each made explicit, bind colour balance rgb's own, within one
/// f32 step for a percentage divided by 100, and the tool on at its defaults plans it
/// with no parameter set, darktable's new instance; off, it plans none.
#[test]
fn b6_on_at_darktable_s_defaults_binds_the_operation_s_defaults() {
    switched::on_at_defaults_holds(&ColourBalanceRgb, COLOR_BALANCE);
}

// B7: version 1's edits.

/// Edits version 1 saved, read as a photo opens them, bind the parameters version 1
/// bound: colour balance rgb planned where a value was off its default, with what the
/// binding gives those values, and not planned, the tool off, where none was.
#[test]
fn b7_version_1_s_edits_bind_what_they_bound() {
    let image = common::photo();
    for (saved, values) in [
        (&[("contrast", "20")][..], &[("contrast", 20.0)][..]),
        (
            &[
                ("vibrance", "30"),
                ("grey_fulcrum", "30"),
                ("contrast", "0"),
            ],
            &[("vibrance", 30.0), ("grey_fulcrum", 30.0)],
        ),
        (&[("saturation_formula", "jzazbz")], &[(FORMULA, 0.0)]),
    ] {
        let edit = switched::version_1(COLOR_BALANCE, saved);
        assert!(edit.is_on(&ColourBalanceRgb), "{saved:?}");
        let params = switched::bound(&edit, COLOR_BALANCE, &image).expect("planned");
        let then = switched::bound(&ROWS.edit_of(values), COLOR_BALANCE, &image).unwrap();
        assert_eq!(params, then, "{saved:?}");
        assert!(params.iter().next().is_some(), "{saved:?}");
    }
    for saved in [&[("contrast", "0")][..], &[]] {
        let edit = switched::version_1(COLOR_BALANCE, saved);
        assert!(!edit.is_on(&ColourBalanceRgb), "{saved:?}");
        assert!(edit.is_empty(), "{saved:?}");
        assert!(
            switched::bound(&edit, COLOR_BALANCE, &image).is_none(),
            "{saved:?}"
        );
    }
}

// B8: the pickers.

/// A flat patch of working RGB `rgb`, as the operation's input reads it.
fn flat(rgb: [f32; 3]) -> apertrust_tools::pick::Reading {
    let mean = [rgb[0], rgb[1], rgb[2], 1.0];
    apertrust_tools::pick::Reading {
        input: apertrust_tools::pick::Stats {
            mean,
            min: mean,
            max: mean,
        },
        output: None,
        input_maximum: [1.0; 3],
    }
}

/// darktable's `pipe_RGB_to_Ych` (`colorbalancergb.c:1261-1275`) restated in f64 on
/// working RGB `rgb`: its Y and its hue in the GUI's degrees, `ANGLE_SHIFT` taken off.
fn darktable_y_and_hue(rgb: [f32; 3]) -> (f64, f64) {
    use apertrust_color::{
        LMS_TO_GRADING_RGB, Mat3d, PROFILE_D65, REC2020_PRIMARIES, XYZ_D50_TO_D65_CAT16,
        XYZ_D65_TO_LMS_2006, profile_rgb_to_xyz_d50,
    };
    let wide = |m: &[[f32; 3]; 3]| m.map(|row| row.map(f64::from));
    let times = |m: &Mat3d, v: [f64; 3]| -> [f64; 3] {
        std::array::from_fn(|r| (0..3).map(|c| m[r][c] * v[c]).sum())
    };
    let xyz = times(
        &profile_rgb_to_xyz_d50(PROFILE_D65, REC2020_PRIMARIES),
        rgb.map(f64::from),
    );
    let lms = times(
        &wide(&XYZ_D65_TO_LMS_2006),
        times(&wide(&XYZ_D50_TO_D65_CAT16), xyz),
    );
    let y = 0.68990272 * lms[0] + 0.34832189 * lms[1];
    let a: f64 = lms.iter().sum();
    let grading = times(&wide(&LMS_TO_GRADING_RGB), lms.map(|v| v / a));
    let (r, g) = (grading[0] - 0.21902143, grading[1] - 0.54371398);
    (y, g.atan2(r).to_degrees() + 30.0)
}

/// How far apart two hues are, the shorter way round.
fn hue_apart(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

/// B8: on a flat patch of colour, Global offset hue's picker with Ctrl writes the patch's
/// own hue in darktable's Ych and without it that hue + 180°, the edit holding each wrapped
/// onto 0° to 360°, and Contrast grey fulcrum's the patch's luminance as a percentage,
/// each read back from the edit, which the pick turns on: within 1e-4° and 1e-5 relative
/// of darktable's arithmetic in f64 on the patch, the operation's input. A blue's own hue
/// and a cyan's opposite are darktable's −95° and 374°, which the edit holds as 265° and
/// 14°.
#[test]
fn b8_a_pick_writes_the_patch_s_hue_and_luminance() {
    use apertrust_tools::ProcessingTool;
    use apertrust_tools::pick::PickKeys;
    let image = common::photo();
    let (mut hue_off, mut grey_off) = (0.0_f64, 0.0_f64);
    for rgb in [
        [0.5, 0.05, 0.05],
        [0.04, 0.06, 0.5],
        [0.45, 0.3, 0.22],
        [0.03, 0.26, 0.34],
    ] {
        let (y, own) = darktable_y_and_hue(rgb);
        match rgb {
            [0.04, 0.06, 0.5] => assert!(own < 0.0, "{own}"),
            [0.03, 0.26, 0.34] => assert!(own + 180.0 > 360.0, "{own}"),
            _ => {}
        }
        for (picker, command, control, want) in [
            ("global_hue", true, "global_h", own.rem_euclid(360.0)),
            (
                "global_hue",
                false,
                "global_h",
                (own + 180.0).rem_euclid(360.0),
            ),
            ("grey_fulcrum", false, "grey_fulcrum", 100.0 * y),
        ] {
            let definition = ColourBalanceRgb.definition();
            let keys = PickKeys {
                command,
                shift: false,
            };
            let defaults = ColourBalanceRgb.defaults(&image);
            let picked = ColourBalanceRgb
                .pick(picker, &flat(rgb), &defaults, &image, &[], None, None, keys)
                .unwrap();
            let mut edit = Edit::default();
            let declared = definition.picker(picker).unwrap();
            edit.pick(&ColourBalanceRgb, declared, &picked.writes, &image)
                .unwrap();
            assert!(edit.is_on(&ColourBalanceRgb), "{picker}");
            let values = edit.tool(COLOR_BALANCE).unwrap();
            let got = f64::from(values.number(control).unwrap());
            if control == "global_h" {
                assert!((0.0..360.0).contains(&got), "{rgb:?}: {got}");
                let off = hue_apart(got, want);
                hue_off = hue_off.max(off);
                assert!(off <= 1e-4, "{rgb:?} {command}: {got} for {want}");
            } else {
                let off = (got - want).abs() / want;
                grey_off = grey_off.max(off);
                assert!(off <= 1e-5, "{rgb:?}: {got} for {want}");
            }
        }
    }
    println!("B8: hue within {hue_off:.1e}°, grey fulcrum {grey_off:.1e} relative");
}
