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

//! Haze removal's invariants (`docs/modules/haze-removal.md`, Invariants), one test per
//! row, each named after its row: Z0 to Z4, the rows a first-pass table has (decision
//! 22), and Z6, the distance at a strength of 0. The synthetic ramp and sweep run on CI's
//! software GPUs, Z4 on the CPU. The raw.pixls.us corpus runs locally, with
//! `APERTRUST_RAW_CORPUS` naming it, and darktable's chart with `APERTRUST_TEST_IMAGES`:
//!
//! ```sh
//! cargo test --release -p apertrust-modules --test haze_removal -- --ignored --nocapture
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

use std::path::PathBuf;

use apertrust_engine::compute::GpuImage;
use apertrust_engine::edit::{self, Edit};
use apertrust_engine::export::{BUDGET, Tiling, export};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe};
use apertrust_modules::haze_removal::{DISTANCE, HazeRemoval, STRENGTH};
use apertrust_modules::{Base, ControlValue, MODULES, Module};
use apertrust_operations::OPERATIONS;
use apertrust_operations::hazeremoval::{self, Settings};
use apertrust_plugin::{ImageContext, ImageDesc, Operation, Value};

use common::{
    BEFORE_THE_WORKING_SPACE, Bench, Jump, Picture, Rendered, exact, gpu, id, jump, small_raw,
    sweep,
};
use corpus::{CORPUS_VIEWPORT, NINETY_D, corpus_bench, decoded, each_corpus_raw, export_bench};

const HAZEREMOVAL: &str = "hazeremoval";

/// The ramp (`docs/modules/README.md`, Test images): neutral greys one per 1/16 EV from
/// 2⁻¹⁶ to 2⁴, then RGB 0.18. One row, over a colour raw's context with an exposure bias
/// of −0.67 EV.
fn ramp() -> (Picture, ImageContext) {
    common::ramp(&[0.18], -0.67)
}

/// What every row reads: the frame, colorout's output, from haze removal on for a
/// synthetic image.
#[derive(Debug, Clone, Copy)]
struct Frame;

impl common::Measure for Frame {
    fn keep(self, instances: &mut Vec<Instance<'static>>, synthetic: bool) {
        instances.retain(|i| !(synthetic && BEFORE_THE_WORKING_SPACE.contains(&id(i))));
    }
}

/// An edit with these controls set, in this order.
fn edit_of(set: &[(&str, f32)]) -> Edit {
    let mut edit = Edit::default();
    for &(control, value) in set {
        edit.set_control(
            &HazeRemoval,
            control,
            ControlValue::Number(value),
            &common::photo(),
        )
        .unwrap();
    }
    edit
}

impl Bench {
    /// The first step (row Z1), made explicit: haze removal planned at the module's own
    /// defaults, a strength of 0 and darktable's distance of 0.2, both written, over an
    /// unedited photo. No edit plans it so, since the module plans haze removal only
    /// while Strength is off 0; its first move off 0 adds this jump to its own.
    fn first_step(&self) -> Vec<Instance<'static>> {
        let mut instances = edit::instances(&Edit::default(), OPERATIONS, MODULES, &self.image)
            .all()
            .unwrap();
        instances.push(Instance {
            operation: apertrust_operations::operation(HAZEREMOVAL).unwrap(),
            params: hazeremoval::values(&Settings {
                strength: 0.0,
                distance: 0.2,
            }),
        });
        self.keep(instances, Frame)
    }

    fn set(&mut self, set: &[(&str, f32)]) -> Rendered {
        self.edit(&edit_of(set), Frame)
    }
}

// Z0: every control at its default.

/// Edits that leave every control at its default: each written at its default, and each
/// kept there.
fn untouched() -> Vec<Edit> {
    let mut written = Edit::default();
    let mut kept = Edit::default();
    for control in HazeRemoval.definition().controls {
        written
            .set_control(
                &HazeRemoval,
                control.id,
                control.default(),
                &common::photo(),
            )
            .unwrap();
        kept.keep(&HazeRemoval, control.id, true).unwrap();
    }
    vec![written, kept]
}

/// Z0 on one pipeline, as the session renders: no edit, then haze removal joining and
/// leaving the plan, then each untouched edit, every frame as the first.
fn z0_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    let joined = bench.first_step();
    bench.render(&joined);
    for edit in untouched() {
        let values = edit.module(HAZEREMOVAL).unwrap();
        let defaults = HazeRemoval.defaults(&bench.image);
        let base = Base::new(OPERATIONS, &bench.image);
        assert!(
            HazeRemoval
                .bind(&values.off_defaults(&defaults), &bench.image, &base)
                .is_empty()
        );
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "Z0 on {:?}",
            bench.pipe
        );
    }
}

#[test]
fn z0_every_control_at_its_default_renders_as_no_edit() {
    for pipe in [
        Pipe::Export,
        Pipe::Viewport {
            width: 40,
            height: 30,
        },
    ] {
        let (mosaic, image) = small_raw(-0.67);
        z0_holds(&mut Bench::mosaic(&mosaic, image, pipe));
    }
    for (picture, image) in [ramp(), sweep(-0.67)] {
        z0_holds(&mut Bench::picture(&picture, image, Pipe::Export));
    }
}

/// Z0 on every corpus raw: the module at its defaults, written or kept, plans the very
/// instances and cache keys no edit does, so the same frame.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn z0_on_the_corpus() {
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
            assert_eq!(keys(&bench.instances(&edit, Frame)), none, "Z0 on {name}");
        }
        raws += 1;
    });
    println!("Z0 holds on {raws} raws");
}

// Z1: the first step, measured and reported without a bound (decision 22).

/// The first step's jump on the ramp and on the sweep. At a strength of 0 each value
/// becomes (I − A0) + A0, the input within one rounding at A0's magnitude.
#[test]
fn z1_the_first_step_is_measured() {
    for (what, (picture, image)) in [("on the ramp", ramp()), ("on the sweep", sweep(-0.67))] {
        let mut bench = Bench::picture(&picture, image, Pipe::Export);
        let untouched = bench.edit(&Edit::default(), Frame);
        let first = bench.first_step();
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
    let after = export_of(&bench.first_step(), image, input);
    jump(&before, &after, |_| true)
}

/// Z1 on darktable's chart, `mire1.cr2`, the golden cases' input.
#[test]
#[ignore = "needs darktable's integration images (APERTRUST_TEST_IMAGES)"]
fn z1_on_the_chart() {
    let dir = PathBuf::from(
        std::env::var_os("APERTRUST_TEST_IMAGES")
            .expect("APERTRUST_TEST_IMAGES names darktable's integration images"),
    );
    let (_, image, input) = decoded(&dir.join("mire1.cr2"));
    let step = first_step_on(&image, &input);
    println!("Z1 on the chart: {step}");
    assert!(step.mean.is_finite() && step.max.is_finite(), "{step}");
}

/// Z1 on every corpus raw the pipeline takes.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn z1_on_the_corpus() {
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
        "Z1 on {} raws: mean at most {worst_mean:.4}, max at most {worst_max:.3}, at most {:.3}% over 1.0",
        jumps.len(),
        100.0 * worst_share
    );
}

// Z2: each control moved, then reset.

/// Moves of every control: the strength removing haze and adding it, the distance with a
/// strength, and both at test 0166's values. Each changes the frame.
const MOVES: [&[(&str, f32)]; 4] = [
    &[(STRENGTH, 0.5)],
    &[(STRENGTH, -0.3)],
    &[(STRENGTH, 0.4), (DISTANCE, 0.8)],
    &[(STRENGTH, 0.9), (DISTANCE, 0.8)],
];

/// Each move, then reset by `ResetControl` (a double-click or Delete) and by
/// `ResetModule` (the header's reset), on the bench's one pipeline, which haze removal
/// joins with each move and leaves with each reset.
fn z2_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    for set in MOVES {
        let mut edit = edit_of(set);
        let moved = bench.edit(&edit, Frame);
        assert!(!exact(&none, &moved), "{set:?} changes the frame");
        for (control, _) in set {
            edit.reset_control(&HazeRemoval, control).unwrap();
        }
        assert_eq!(edit, Edit::default());
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "{set:?}, reset control by control"
        );
        let mut edit = edit_of(set);
        bench.edit(&edit, Frame);
        edit.reset_module(&HazeRemoval);
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "{set:?}, the module reset"
        );
    }
}

#[test]
fn z2_a_control_moved_and_reset_renders_as_no_edit() {
    let (picture, image) = sweep(-0.67);
    z2_holds(&mut Bench::picture(&picture, image, Pipe::Export));
    let (mosaic, image) = small_raw(-0.67);
    let viewport = Pipe::Viewport {
        width: 40,
        height: 30,
    };
    z2_holds(&mut Bench::mosaic(&mosaic, image, viewport));
}

// Z3: the order controls were set in.

const PAIRS: [[(&str, f32); 2]; 2] = [
    [(STRENGTH, 0.6), (DISTANCE, 0.5)],
    [(STRENGTH, -0.4), (DISTANCE, 0.1)],
];

/// One pipeline sets `first`, then both; another sets `second`, then both: the two
/// frames are the same, byte for byte.
fn z3_holds(new: impl Fn() -> Bench, [first, second]: [(&str, f32); 2]) {
    let mut one = new();
    one.set(&[first]);
    let one = one.set(&[first, second]);
    let mut other = new();
    other.set(&[second]);
    let other = other.set(&[second, first]);
    assert!(exact(&one, &other), "Z3 with {first:?} and {second:?}");
}

#[test]
fn z3_controls_render_the_same_whichever_came_first() {
    let (picture, image) = sweep(-0.67);
    for pair in PAIRS {
        z3_holds(
            || Bench::picture(&picture, image.clone(), Pipe::Export),
            pair,
        );
    }
}

/// Z2 and Z3 on the 90D, through the viewport's frame and the export's.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn z2_and_z3_on_the_corpus() {
    for pipe in [CORPUS_VIEWPORT, Pipe::Export] {
        z2_holds(&mut corpus_bench(NINETY_D, pipe));
        for pair in PAIRS {
            z3_holds(|| corpus_bench(NINETY_D, pipe), pair);
        }
    }
}

// Z4: darktable's golden cases.

/// Z4: the module's controls set to the values of each `hazeremoval` golden case,
/// darktable's tests 0026 and 0166, bind the case's parameters bit for bit, and through
/// the operation's `prepare` give the very passes the golden harness checks against
/// darktable's render. So the module renders those values as the cases measure the port.
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
        let instances = edit::instances(&edit, OPERATIONS, MODULES, &image)
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

// Z6: the distance does nothing while the strength is 0.

/// The distance moved while the strength is 0 binds nothing, so haze removal stays out
/// of the plan and the frame is no edit's, exactly, on one pipeline as the session
/// renders. It stays enabled there, as darktable's does.
fn z6_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    let base = Base::new(OPERATIONS, &bench.image);
    let defaults = HazeRemoval.defaults(&bench.image);
    for distance in [0.0, 0.05, 0.65, 1.0] {
        let edit = edit_of(&[(DISTANCE, distance)]);
        let values = edit.module(HAZEREMOVAL).unwrap().off_defaults(&defaults);
        assert!(
            HazeRemoval.bind(&values, &bench.image, &base).is_empty(),
            "the distance at {distance}"
        );
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "the distance at {distance} on {:?}",
            bench.pipe
        );
    }
}

#[test]
fn z6_the_distance_at_strength_0_renders_as_no_edit() {
    let (picture, image) = sweep(-0.67);
    z6_holds(&mut Bench::picture(&picture, image, Pipe::Export));
    let (mosaic, image) = small_raw(-0.67);
    let viewport = Pipe::Viewport {
        width: 40,
        height: 30,
    };
    z6_holds(&mut Bench::mosaic(&mosaic, image, viewport));
}
