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

//! Grain's invariants (`docs/modules/grain.md`, Invariants), one test per row, each
//! named after its row: a switched module's (decision 32), G0 off, G1 the first step, on
//! at darktable's defaults, G2 moved and reset, G3 the order, G4 the darktable match, G6
//! on at darktable's defaults on the CPU and G7 version 1's edits; G5 is withdrawn. The
//! synthetic ramp and sweep run on CI's software GPUs, G4, G6 and G7 on the CPU. The
//! raw.pixls.us corpus runs locally, with `APERTRUST_RAW_CORPUS` naming it:
//!
//! ```sh
//! cargo test --release -p apertrust-modules --test grain -- --ignored --nocapture
//! ```
//!
//! Synthetic images enter after demosaic, already in the working space: their rows plan
//! the operations from exposure on and leave colorin out. Every row reads the frame,
//! colorout's output before it is rounded to 8 bits, clipped to 0 to 1 as a PNG holds it.

#[expect(
    dead_code,
    reason = "Grain's rows read neither the extended ramp nor a luminance"
)]
mod common;
mod corpus;
mod switched;

use apertrust_engine::compute::GpuImage;
use apertrust_engine::edit::{self, Edit};
use apertrust_engine::export::{BUDGET, Tiling, export};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe};
use apertrust_modules::grain::{Grain, iso, scale};
use apertrust_modules::{ControlValue, MODULES};
use apertrust_operations::OPERATIONS;
use apertrust_operations::grain::{
    Grain as GrainOperation, PARAMS_SIZE, PROVISIONAL_OFFSET, noise_offsets,
};
use apertrust_plugin::{ImageContext, ImageDesc, Operation, ParamValues, Value};

use common::{
    BEFORE_THE_WORKING_SPACE, Bench, Jump, Picture, exact, gpu, id, jump, small_raw, sweep,
};
use corpus::{CORPUS_VIEWPORT, NINETY_D, corpus_bench, each_corpus_raw, export_bench};

const GRAIN: &str = "grain";

/// The ramp (`docs/modules/README.md`, Test images): neutral greys one per 1/16 EV from
/// 2⁻¹⁶ to 2⁴, then RGB 0.18. One row, over a colour raw's context with an exposure bias
/// of −0.67 EV.
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

/// An edit with these controls set, in this order.
fn edit_of(set: &[(&str, f32)]) -> Edit {
    let mut edit = Edit::default();
    for &(control, value) in set {
        edit.set_control(
            &Grain,
            control,
            ControlValue::Number(value),
            &common::photo(),
        )
        .unwrap();
    }
    edit
}

impl Bench {
    /// The module on at darktable's defaults, 1600 ISO, 25 % and 100 %: grain planned at
    /// the operation's defaults, darktable's new instance (rows G1 and G2).
    fn on_at_defaults(&self) -> Vec<Instance<'static>> {
        self.instances(&switched::on_at_defaults(&Grain, &self.image), Frame)
    }

    fn set(&mut self, set: &[(&str, f32)]) -> common::Rendered {
        self.edit(&edit_of(set), Frame)
    }
}

// G0: off.

/// G0 on one pipeline, as the session renders: no edit, then grain joining the plan, on
/// at darktable's defaults, and leaving it, then each edit that leaves the module off,
/// every frame as the first.
fn g0_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    let joined = bench.on_at_defaults();
    bench.render(&joined);
    for edit in switched::off(&Grain) {
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "G0 on {:?}",
            bench.pipe
        );
    }
}

#[test]
fn g0_off_renders_as_no_edit() {
    for pipe in [
        Pipe::Export,
        Pipe::Viewport {
            width: 40,
            height: 30,
        },
    ] {
        let (mosaic, image) = small_raw(-0.67);
        g0_holds(&mut Bench::mosaic(&mosaic, image, pipe));
    }
    for (picture, image) in [ramp(), sweep(-0.67)] {
        g0_holds(&mut Bench::picture(&picture, image, Pipe::Export));
    }
}

/// G0 on every corpus raw: the module off, untouched or kept, plans the very instances
/// and cache keys no edit does, so the same frame.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn g0_on_the_corpus() {
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
        for edit in switched::off(&Grain) {
            assert_eq!(keys(&bench.instances(&edit, Frame)), none, "G0 on {name}");
        }
        raws += 1;
    });
    println!("G0 holds on {raws} raws");
}

// G1: the first step, on at darktable's defaults, measured and reported without a bound
// (decision 22).

/// The first step's jump on the ramp and on the sweep.
#[test]
fn g1_the_first_step_is_measured() {
    for (what, (picture, image)) in [("on the ramp", ramp()), ("on the sweep", sweep(-0.67))] {
        let mut bench = Bench::picture(&picture, image, Pipe::Export);
        let untouched = bench.edit(&Edit::default(), Frame);
        let first = bench.on_at_defaults();
        let first = bench.render(&first);
        let step = jump(&untouched.pixels, &first.pixels, |_| true);
        println!("G1 {what}: {step}");
        assert!(
            step.pixels > 0 && step.mean.is_finite() && step.max.is_finite(),
            "G1 {what}: {step}"
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

/// G1 on every corpus raw the pipeline takes.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn g1_on_the_corpus() {
    let mut jumps = Vec::new();
    each_corpus_raw(|name, image, input| {
        let bench = export_bench(image.clone(), input.clone());
        let before = export_of(&bench.instances(&Edit::default(), Frame), &image, &input);
        let after = export_of(&bench.on_at_defaults(), &image, &input);
        let step = jump(&before, &after, |_| true);
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
        "G1 on {} raws: mean at most {worst_mean:.4}, max at most {worst_max:.3}, at most {:.3}% over 1.0",
        jumps.len(),
        100.0 * worst_share
    );
}

// G2: each control moved, then reset.

/// Moves of every control, each by its step and by more: the strength at 0 and one step
/// off darktable's 25 %, the coarseness and the bias each one step off its default, and
/// all three at the `grain-agx` golden case's values; and a write of the strength at its
/// default, which turns the module on. Each changes the frame.
const MOVES: [&[(&str, f32)]; 6] = [
    &[("strength", 0.0)],
    &[("strength", 26.0)],
    &[("strength", 25.0)],
    &[("scale", 1650.0)],
    &[("midtones_bias", 99.0)],
    &[
        ("scale", 3200.0),
        ("strength", 71.3),
        ("midtones_bias", 83.0),
    ],
];

/// Each move, then reset by `ResetControl` (a double-click or Delete), control by
/// control, which leaves the module on at darktable's defaults, and by `ResetModule` (the
/// header's reset), which turns it off, as no edit; on the bench's one pipeline, which
/// grain joins with each move and leaves with each module's reset.
fn g2_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    let on = bench.on_at_defaults();
    let on = bench.render(&on);
    for set in MOVES {
        let mut edit = edit_of(set);
        let moved = bench.edit(&edit, Frame);
        assert!(!exact(&none, &moved), "{set:?} changes the frame");
        for (control, _) in set {
            edit.reset_control(&Grain, control).unwrap();
        }
        assert!(edit.is_on(&Grain), "{set:?}, reset control by control");
        assert!(
            exact(&on, &bench.edit(&edit, Frame)),
            "{set:?}, reset control by control"
        );
        let mut edit = edit_of(set);
        bench.edit(&edit, Frame);
        edit.reset_module(&Grain);
        assert_eq!(edit, Edit::default());
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "{set:?}, the module reset"
        );
    }
}

#[test]
fn g2_a_control_moved_and_reset_renders_as_on_and_the_module_reset_as_no_edit() {
    let (picture, image) = sweep(-0.67);
    g2_holds(&mut Bench::picture(&picture, image, Pipe::Export));
    let (mosaic, image) = small_raw(-0.67);
    let viewport = Pipe::Viewport {
        width: 40,
        height: 30,
    };
    g2_holds(&mut Bench::mosaic(&mosaic, image, viewport));
}

// G3: the order controls were set in.

const PAIRS: [[(&str, f32); 2]; 2] = [
    [("strength", 25.0), ("scale", 800.0)],
    [("midtones_bias", 52.7), ("strength", 37.3)],
];

/// One pipeline sets `first`, then both; another sets `second`, then both: the two
/// frames are the same, byte for byte.
fn g3_holds(new: impl Fn() -> Bench, [first, second]: [(&str, f32); 2]) {
    let mut one = new();
    one.set(&[first]);
    let one = one.set(&[first, second]);
    let mut other = new();
    other.set(&[second]);
    let other = other.set(&[second, first]);
    assert!(exact(&one, &other), "G3 with {first:?} and {second:?}");
}

#[test]
fn g3_controls_render_the_same_whichever_came_first() {
    let (picture, image) = sweep(-0.67);
    for pair in PAIRS {
        g3_holds(
            || Bench::picture(&picture, image.clone(), Pipe::Export),
            pair,
        );
    }
}

/// G2 and G3 on the 90D, through the viewport's frame and the export's.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn g2_and_g3_on_the_corpus() {
    for pipe in [CORPUS_VIEWPORT, Pipe::Export] {
        g2_holds(&mut corpus_bench(NINETY_D, pipe));
        for pair in PAIRS {
            g3_holds(|| corpus_bench(NINETY_D, pipe), pair);
        }
    }
}

// G4: darktable's golden cases.

/// Little-endian bytes as four-byte words.
fn words(bytes: &[u8]) -> Vec<[u8; 4]> {
    bytes.as_chunks::<4>().0.to_vec()
}

/// The golden grain cases, test 0059's four and the one after AgX.
const CASES: [&str; 5] = [
    "grain",
    "grain-fine",
    "grain-channel",
    "grain-v1",
    "grain-agx",
];

/// The uniform's words that hold darktable's offset per octave, which an edit has no
/// field for: the engine renders at the provisional offset (`docs/ports/grain.md`,
/// Deliberate differences), the golden cases at darktable's hash of the file name.
const OFFSET_WORDS: std::ops::Range<usize> = 4..7;

/// The uniform's words that name the derivative entry points' parameters, which `main`
/// does not read and the golden harness chooses for its gradient check.
const GRADIENT_WORDS: std::ops::Range<usize> = 32..36;

/// G4: the module's controls set to each golden grain case's values as darktable's
/// sliders show them, the coarseness its field × 213.2 in f32, unrounded, and a version 1
/// case's bias 0, as darktable's `legacy_params` gives it, bind the case's parameters bit
/// for bit, and, through the operation's `prepare`, every word of the kernel's inputs the
/// golden harness checks against darktable's render but the offset's, which are the
/// provisional offset's. So the module renders those values as the cases measure the
/// port, at darktable's offset: ΔE2000 means of 0.0008 at most (`docs/ports/grain.md`).
/// The coarseness rounded to the whole ISO the panel shows binds the field too, but for
/// `grain`'s, one f32 step under darktable's maximum, which only an unrounded value, from
/// the CLI or a script, reaches: its 6400 ISO binds the maximum, one step above.
#[test]
fn g4_the_golden_cases_values_bind_to_darktable_s_parameters_bit_for_bit() {
    let schema = GrainOperation.schema();
    let image = ImageContext::full(1, 1);
    let bound = |edit: &Edit| {
        let instances = edit::instances(edit, OPERATIONS, MODULES, &image)
            .all()
            .unwrap();
        let instance = instances.iter().find(|i| id(i) == GRAIN).unwrap();
        schema.resolve(&instance.params).unwrap()
    };
    for name in CASES {
        let case = golden::cases::find(name).unwrap();
        let darktable = golden::xmp::module_params(case.xmp, GRAIN).unwrap();
        // `channel`, then darktable's fields in the schema's order; version 1 has no bias.
        let fields = words(&darktable.bytes);
        let field = |i: usize| f32::from_le_bytes(fields[i]);
        let bias = if darktable.version == 1 {
            0.0
        } else {
            field(3)
        };
        let expected = [
            ("scale", field(1)),
            ("strength", field(2)),
            ("midtones_bias", bias),
        ];
        let edit = edit_of(&[
            ("scale", iso(field(1))),
            ("strength", field(2)),
            ("midtones_bias", bias),
        ]);

        let params = bound(&edit);
        for (param, darktable) in expected {
            let Some(Value::Float(bound)) = params.get(param) else {
                panic!("{name}: {param}");
            };
            assert_eq!(
                bound.to_bits(),
                darktable.to_bits(),
                "{name}: {param} is {bound}, darktable's {darktable}"
            );
        }

        let prepared = GrainOperation
            .prepare(&params, &image, &ImageDesc::rgba(1, 1))
            .unwrap();
        let ours = words(&prepared.passes[0].params);
        let inputs = golden::ports::kernel_inputs(case).unwrap().unwrap();
        let theirs = words(&inputs.params);
        assert_eq!(
            (ours.len(), theirs.len()),
            (PARAMS_SIZE / 4, PARAMS_SIZE / 4)
        );
        assert_eq!(
            prepared.passes[0].tables, inputs.tables,
            "{name}: the tables"
        );
        let provisional = noise_offsets(PROVISIONAL_OFFSET, field(1));
        for (i, (a, b)) in ours.iter().zip(&theirs).enumerate() {
            if OFFSET_WORDS.contains(&i) {
                let offset = provisional[i - OFFSET_WORDS.start];
                assert_eq!(*a, offset.to_le_bytes(), "{name}: offset word {i}");
            } else if !GRADIENT_WORDS.contains(&i) {
                assert_eq!(a, b, "{name}: uniform word {i}");
            }
        }

        let shown = iso(field(1)).round();
        let rounded = bound(&edit_of(&[("scale", shown), ("strength", field(2))]));
        let Some(Value::Float(scale)) = rounded.get("scale") else {
            panic!("{name}: scale");
        };
        let steps = scale.to_bits().abs_diff(field(1).to_bits());
        assert_eq!(
            steps,
            u32::from(name == "grain"),
            "{name}: {shown} ISO binds {scale}, {steps} f32 steps from darktable's {}",
            field(1)
        );
    }
}

// G6: on at darktable's defaults, on the CPU.

/// The module's defaults, each made explicit, bind grain's own, and the module on at its
/// defaults plans grain with no parameter set, darktable's new instance; off, it plans
/// none.
#[test]
fn g6_on_at_darktable_s_defaults_binds_the_operation_s_defaults() {
    switched::on_at_defaults_holds(&Grain, GRAIN);
}

// G7: version 1's edits.

/// Edits version 1 saved, read as a photo opens them, bind the parameters version 1
/// bound, resolved over the schema: grain planned where the strength was off 0, version
/// 1's untouched value, and not planned, the module off, where it was not.
#[test]
fn g7_version_1_s_edits_bind_what_they_bound() {
    let image = common::photo();
    let schema = GrainOperation.schema();
    let resolved = |set: &[(&str, f32)]| {
        let mut params = ParamValues::new();
        for &(param, value) in set {
            params.set(param, Value::Float(value));
        }
        schema.resolve(&params).unwrap()
    };
    for (saved, bound) in [
        (
            &[("strength", "37.3"), ("scale", "800")][..],
            &[("strength", 37.3), ("scale", scale(800.0))][..],
        ),
        (
            &[("strength", "25"), ("midtones_bias", "52.7")],
            &[("strength", 25.0), ("midtones_bias", 52.7)],
        ),
        (
            &[("midtones_bias", "0"), ("strength", "100")],
            &[("midtones_bias", 0.0), ("strength", 100.0)],
        ),
    ] {
        let edit = switched::version_1(GRAIN, saved);
        assert!(edit.is_on(&Grain), "{saved:?}");
        let params = switched::bound(&edit, GRAIN, &image).expect("grain planned");
        assert_eq!(
            schema.resolve(&params).unwrap(),
            resolved(bound),
            "{saved:?}"
        );
    }
    for saved in [&[("scale", "3200")][..], &[("strength", "0")], &[]] {
        let edit = switched::version_1(GRAIN, saved);
        assert!(!edit.is_on(&Grain), "{saved:?}");
        assert!(edit.is_empty(), "{saved:?}");
        assert!(switched::bound(&edit, GRAIN, &image).is_none(), "{saved:?}");
    }
    // The strength at 0 is version 1's untouched value, which planned nothing: no
    // instance at all, as the plan of no edit.
    let none = edit::instances(&Edit::default(), OPERATIONS, MODULES, &image)
        .all()
        .unwrap()
        .len();
    let off = switched::version_1(GRAIN, &[("strength", "0"), ("scale", "3200")]);
    let planned = edit::instances(&off, OPERATIONS, MODULES, &image)
        .all()
        .unwrap();
    assert_eq!(planned.len(), none);
}
