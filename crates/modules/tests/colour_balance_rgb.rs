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

//! Colour balance RGB's invariants (`docs/modules/colour-balance-rgb.md`, Invariants),
//! one test per row, each named after its row: a switched module's (decision 32), B0
//! off, B1 the first step, on at darktable's defaults, B2 moved and reset, B3 the order,
//! B4 the darktable match, B6 on at darktable's defaults on the CPU and B7 version 1's
//! edits; B5, on a layer, waits for Phase 4. The synthetic ramp and sweep run on CI's
//! software GPUs, B4, B6 and B7 on the CPU. The raw.pixls.us corpus runs locally, with
//! `APERTRUST_RAW_CORPUS` naming it, and darktable's chart with `APERTRUST_TEST_IMAGES`:
//!
//! ```sh
//! cargo test --release -p apertrust-modules --test colour_balance_rgb -- --ignored --nocapture
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

use std::path::PathBuf;

use apertrust_engine::compute::GpuImage;
use apertrust_engine::edit::{self, Edit};
use apertrust_engine::export::{BUDGET, Tiling, export};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe};
use apertrust_modules::colour_balance_rgb::{ColourBalanceRgb, FORMULA, PARAMETERS};
use apertrust_modules::{ControlValue, MODULES};
use apertrust_operations::OPERATIONS;
use apertrust_operations::colorbalancergb::ColorBalanceRgb as ColourBalanceOperation;
use apertrust_plugin::{ImageContext, ImageDesc, Operation, Value};
use golden::synthetic;

use common::{
    BEFORE_THE_WORKING_SPACE, Bench, Jump, Picture, Rendered, exact, gpu, id, jump, small_raw,
    sweep,
};
use corpus::{CORPUS_VIEWPORT, NINETY_D, corpus_bench, decoded, each_corpus_raw, export_bench};

const COLOR_BALANCE: &str = "colorbalancergb";

/// The ramp (`docs/modules/README.md`, Test images): neutral greys one per 1/16 EV from
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

/// What every row reads: the frame, colorout's output, from exposure on for a synthetic
/// image.
#[derive(Debug, Clone, Copy)]
struct Frame;

impl common::Measure for Frame {
    fn keep(self, instances: &mut Vec<Instance<'static>>, synthetic: bool) {
        instances.retain(|i| !(synthetic && BEFORE_THE_WORKING_SPACE.contains(&id(i))));
    }
}

fn value_of(control: &str, value: f32) -> ControlValue {
    if control == FORMULA {
        ControlValue::Choice(value as u32)
    } else {
        ControlValue::Number(value)
    }
}

/// An edit with these controls set, in this order.
fn edit_of(set: &[(&str, f32)]) -> Edit {
    let mut edit = Edit::default();
    for &(control, value) in set {
        edit.set_control(
            &ColourBalanceRgb,
            control,
            value_of(control, value),
            &common::photo(),
        )
        .unwrap();
    }
    edit
}

impl Bench {
    /// The module on at darktable's defaults: colour balance rgb planned at the
    /// operation's defaults, darktable's new instance (rows B1 and B2).
    fn on_at_defaults(&self) -> Vec<Instance<'static>> {
        self.instances(
            &switched::on_at_defaults(&ColourBalanceRgb, &self.image),
            Frame,
        )
    }

    fn set(&mut self, set: &[(&str, f32)]) -> Rendered {
        self.edit(&edit_of(set), Frame)
    }
}

// B0: off.

/// B0 on one pipeline, as the session renders: no edit, then colour balance rgb joining
/// the plan, on at darktable's defaults, and leaving it, then each edit that leaves the
/// module off, every frame as the first.
fn b0_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    let joined = bench.on_at_defaults();
    bench.render(&joined);
    for edit in switched::off(&ColourBalanceRgb) {
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "B0 on {:?}",
            bench.pipe
        );
    }
}

#[test]
fn b0_off_renders_as_no_edit() {
    for pipe in [
        Pipe::Export,
        Pipe::Viewport {
            width: 40,
            height: 30,
        },
    ] {
        let (mosaic, image) = small_raw(-0.67);
        b0_holds(&mut Bench::mosaic(&mosaic, image, pipe));
    }
    for (picture, image) in [ramp(), sweep(-0.67)] {
        b0_holds(&mut Bench::picture(&picture, image, Pipe::Export));
    }
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
        let first = bench.on_at_defaults();
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

/// An export of `instances`, tiled when the raw does not fit.
fn export_of(instances: &[Instance], image: &ImageContext, input: &GpuImage) -> Vec<[f32; 4]> {
    let tiling = Tiling::Auto { budget: BUDGET };
    pollster::block_on(export(gpu(), instances, image, input.clone(), tiling))
        .unwrap()
        .pixels
}

/// The first step's jump on an export of `image`.
fn first_step_on(image: &ImageContext, input: &GpuImage) -> Jump {
    let bench = export_bench(image.clone(), input.clone());
    let before = export_of(&bench.instances(&Edit::default(), Frame), image, input);
    let after = export_of(&bench.on_at_defaults(), image, input);
    jump(&before, &after, |_| true)
}

/// B1 on darktable's chart, `mire1.cr2`, test 0083's input.
#[test]
#[ignore = "needs darktable's integration images (APERTRUST_TEST_IMAGES)"]
fn b1_on_the_chart() {
    let dir = PathBuf::from(
        std::env::var_os("APERTRUST_TEST_IMAGES")
            .expect("APERTRUST_TEST_IMAGES names darktable's integration images"),
    );
    let (_, image, input) = decoded(&dir.join("mire1.cr2"));
    let step = first_step_on(&image, &input);
    println!("B1 on the chart: {step}");
    assert!(step.mean.is_finite() && step.max.is_finite(), "{step}");
}

/// B1 on every corpus raw the pipeline takes.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn b1_on_the_corpus() {
    let mut jumps = Vec::new();
    each_corpus_raw(|name, image, input| {
        let step = first_step_on(&image, &input);
        println!("{name}: {step}");
        assert!(
            step.mean.is_finite() && step.max.is_finite(),
            "{name}: {step}"
        );
        jumps.push(step);
    });
    let worst_mean = jumps.iter().map(|j| j.mean).fold(0.0, f64::max);
    let worst_max = jumps.iter().map(|j| j.max).fold(0.0, f64::max);
    let worst_share = jumps.iter().map(Jump::share_over_one).fold(0.0, f64::max);
    println!(
        "B1 on {} raws: mean at most {worst_mean:.4}, max at most {worst_max:.3}, at most {:.3}% over 1.0",
        jumps.len(),
        100.0 * worst_share
    );
}

// B2: each control moved, then reset.

/// Moves on every tab, the formula included, and a write of the contrast at its default,
/// which turns the module on: each changes the frame, as the first step at least does.
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

/// Each move, then reset by `ResetControl` (a double-click or Delete), control by
/// control, which leaves the module on at darktable's defaults, and by `ResetModule` (the
/// header's reset), which turns it off, as no edit; on the bench's one pipeline, which
/// colour balance rgb joins with each move and leaves with each module's reset.
fn b2_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    let on = bench.on_at_defaults();
    let on = bench.render(&on);
    for set in MOVES {
        let mut edit = edit_of(set);
        let moved = bench.edit(&edit, Frame);
        assert!(!exact(&none, &moved), "{set:?} changes the frame");
        for (control, _) in set {
            edit.reset_control(&ColourBalanceRgb, control).unwrap();
        }
        assert!(
            edit.is_on(&ColourBalanceRgb),
            "{set:?}, reset control by control"
        );
        assert!(
            exact(&on, &bench.edit(&edit, Frame)),
            "{set:?}, reset control by control"
        );
        let mut edit = edit_of(set);
        bench.edit(&edit, Frame);
        edit.reset_module(&ColourBalanceRgb);
        assert_eq!(edit, Edit::default());
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "{set:?}, the module reset"
        );
    }
}

#[test]
fn b2_a_control_moved_and_reset_renders_as_on_and_the_module_reset_as_no_edit() {
    let (picture, image) = ramp();
    b2_holds(&mut Bench::picture(&picture, image, Pipe::Export));
    let (mosaic, image) = small_raw(-0.67);
    let viewport = Pipe::Viewport {
        width: 40,
        height: 30,
    };
    b2_holds(&mut Bench::mosaic(&mosaic, image, viewport));
}

// B3: the order controls were set in.

const PAIRS: [[(&str, f32); 2]; 2] = [
    [("contrast", 20.0), ("saturation_global", -30.0)],
    [(FORMULA, 0.0), ("brilliance_global", 10.0)],
];

/// One pipeline sets `first`, then both; another sets `second`, then both: the two
/// frames are the same, byte for byte.
fn b3_holds(new: impl Fn() -> Bench, [first, second]: [(&str, f32); 2]) {
    let mut one = new();
    one.set(&[first]);
    let one = one.set(&[first, second]);
    let mut other = new();
    other.set(&[second]);
    let other = other.set(&[second, first]);
    assert!(exact(&one, &other), "B3 with {first:?} and {second:?}");
}

#[test]
fn b3_controls_render_the_same_whichever_came_first() {
    let (picture, image) = sweep(-0.67);
    for pair in PAIRS {
        b3_holds(
            || Bench::picture(&picture, image.clone(), Pipe::Export),
            pair,
        );
    }
}

/// B2 and B3 on the 90D, through the viewport's frame and the export's.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn b2_and_b3_on_the_corpus() {
    for pipe in [CORPUS_VIEWPORT, Pipe::Export] {
        b2_holds(&mut corpus_bench(NINETY_D, pipe));
        for pair in PAIRS {
            b3_holds(|| corpus_bench(NINETY_D, pipe), pair);
        }
    }
}

/// B0 on every corpus raw: the module off, untouched or kept, plans the very instances
/// and cache keys no edit does, so the same frame.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn b0_on_the_corpus() {
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
        for edit in switched::off(&ColourBalanceRgb) {
            assert_eq!(keys(&bench.instances(&edit, Frame)), none, "B0 on {name}");
        }
        raws += 1;
    });
    println!("B0 holds on {raws} raws");
}

// B4: darktable's golden cases.

/// Little-endian bytes as four-byte words.
fn words(bytes: &[u8]) -> Vec<[u8; 4]> {
    bytes.as_chunks::<4>().0.to_vec()
}

/// How far a uniform word of ours is from the golden case's: 0 for the same bits, else
/// the relative difference of the two as floats.
fn apart(ours: [u8; 4], theirs: [u8; 4]) -> f32 {
    let (a, b) = (f32::from_le_bytes(ours), f32::from_le_bytes(theirs));
    if ours == theirs {
        0.0
    } else {
        (a - b).abs() / b.abs()
    }
}

/// How far apart B4 lets a uniform word be: two f32 steps, relative. A percentage over
/// 100 is within one step of darktable's parameter, and the setup may round once more.
const TWO_STEPS: f32 = 2.4e-7;

/// B4: the module's controls set to the values of `colorbalancergb`'s golden cases,
/// darktable's tests 0083 (params version 4, JzAzBz) and 0093 (version 5, darktable
/// UCS), bind to the case's parameters and, through the operation's `prepare`, to the
/// kernel inputs the golden harness checks against darktable's render. So the module
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

        let instances = edit::instances(&edit, OPERATIONS, MODULES, &image)
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

/// The module's defaults, each made explicit, bind colour balance rgb's own, within one
/// f32 step for a percentage divided by 100, and the module on at its defaults plans it
/// with no parameter set, darktable's new instance; off, it plans none.
#[test]
fn b6_on_at_darktable_s_defaults_binds_the_operation_s_defaults() {
    switched::on_at_defaults_holds(&ColourBalanceRgb, COLOR_BALANCE);
}

// B7: version 1's edits.

/// Edits version 1 saved, read as a photo opens them, bind the parameters version 1
/// bound: colour balance rgb planned where a value was off its default, with what the
/// binding gives those values, and not planned, the module off, where none was.
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
        let then = switched::bound(&edit_of(values), COLOR_BALANCE, &image).unwrap();
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
