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

//! Soften's invariants (`docs/modules/soften.md`, Invariants), one test per row, each
//! named after its row: S0 to S4, the rows a first-pass table has (decision 22), and S6,
//! the other controls while the mix is 0; S5, on a layer, waits for Phase 4. The
//! synthetic ramp and sweep run on CI's software GPUs, S4 on the CPU. The raw.pixls.us
//! corpus runs locally, with `APERTRUST_RAW_CORPUS` naming it:
//!
//! ```sh
//! cargo test --release -p apertrust-modules --test soften -- --ignored --nocapture
//! ```
//!
//! Synthetic images enter after demosaic, already in the working space: their rows plan
//! the operations from exposure on and leave colorin out. Every row reads the frame,
//! colorout's output before it is rounded to 8 bits, clipped to 0 to 1 as a PNG holds it.

#[expect(
    dead_code,
    reason = "Soften's rows read neither the extended ramp nor a luminance"
)]
mod common;
mod corpus;

use apertrust_engine::compute::GpuImage;
use apertrust_engine::edit::{self, Edit};
use apertrust_engine::export::{BUDGET, ExportError, Tiling, export};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe};
use apertrust_modules::soften::Soften;
use apertrust_modules::{Base, ControlValue, MODULES, Module};
use apertrust_operations::OPERATIONS;
use apertrust_operations::soften::Soften as SoftenOperation;
use apertrust_plugin::{ImageContext, Operation, ParamValues, Value};

use common::{
    BEFORE_THE_WORKING_SPACE, Bench, Jump, Picture, Rendered, exact, gpu, id, jump, small_raw,
    sweep,
};
use corpus::{CORPUS_VIEWPORT, NINETY_D, corpus_bench, each_corpus_raw, export_bench};

const SOFTEN: &str = "soften";

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
            &Soften,
            control,
            ControlValue::Number(value),
            &common::photo(),
        )
        .unwrap();
    }
    edit
}

impl Bench {
    /// The first step (row S1): soften planned at the module's own defaults made
    /// explicit, a mix of 0 and darktable's size, saturation and brightness, the
    /// operation's, over an unedited photo: what the mix's first step off 0 brings besides
    /// its blur.
    fn first_step(&self) -> Vec<Instance<'static>> {
        let mut instances = edit::instances(&Edit::default(), OPERATIONS, MODULES, &self.image)
            .all()
            .unwrap();
        let mut params = ParamValues::new();
        params.set("amount", Value::Float(0.0));
        instances.push(Instance {
            operation: apertrust_operations::operation(SOFTEN).unwrap(),
            params,
        });
        self.keep(instances, Frame)
    }

    fn set(&mut self, set: &[(&str, f32)]) -> Rendered {
        self.edit(&edit_of(set), Frame)
    }
}

/// Whether two frames' pixels hold the same values, a zero of either sign equal to the
/// other, whatever their plans' keys.
fn same_values(a: &[[f32; 4]], b: &[[f32; 4]]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x == y)
}

/// How many of two frames' channels differ in their bits.
fn bits_apart(a: &[[f32; 4]], b: &[[f32; 4]]) -> usize {
    a.iter()
        .flatten()
        .zip(b.iter().flatten())
        .filter(|(x, y)| x.to_bits() != y.to_bits())
        .count()
}

// S0: every control at its default.

/// Edits that leave every control at its default: each written at its default, and each
/// kept there.
fn untouched() -> Vec<Edit> {
    let mut written = Edit::default();
    let mut kept = Edit::default();
    for control in Soften.definition().controls {
        written
            .set_control(&Soften, control.id, control.default(), &common::photo())
            .unwrap();
        kept.keep(&Soften, control.id, true).unwrap();
    }
    vec![written, kept]
}

/// S0 on one pipeline, as the session renders: no edit, then soften joining and leaving
/// the plan, then each untouched edit, every frame as the first.
fn s0_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    let joined = bench.first_step();
    bench.render(&joined);
    for edit in untouched() {
        let values = edit.module(SOFTEN).unwrap();
        let defaults = Soften.defaults(&bench.image);
        let base = Base::new(OPERATIONS, &bench.image);
        assert!(
            Soften
                .bind(&values.off_defaults(&defaults), &bench.image, &base)
                .is_empty()
        );
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "S0 on {:?}",
            bench.pipe
        );
    }
}

#[test]
fn s0_every_control_at_its_default_renders_as_no_edit() {
    for pipe in [
        Pipe::Export,
        Pipe::Viewport {
            width: 40,
            height: 30,
        },
    ] {
        let (mosaic, image) = small_raw(-0.67);
        s0_holds(&mut Bench::mosaic(&mosaic, image, pipe));
    }
    for (picture, image) in [ramp(), sweep(-0.67)] {
        s0_holds(&mut Bench::picture(&picture, image, Pipe::Export));
    }
}

/// S0 on every corpus raw: the module at its defaults, written or kept, plans the very
/// instances and cache keys no edit does, so the same frame.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn s0_on_the_corpus() {
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
            assert_eq!(keys(&bench.instances(&edit, Frame)), none, "S0 on {name}");
        }
        raws += 1;
    });
    println!("S0 holds on {raws} raws");
}

// S1: the first step, measured and reported without a bound (decision 22).

/// The first step on the ramp and on the sweep: its jump, and whether the frame's values
/// are the untouched frame's.
#[test]
fn s1_the_first_step_is_measured() {
    for (what, (picture, image)) in [("on the ramp", ramp()), ("on the sweep", sweep(-0.67))] {
        let mut bench = Bench::picture(&picture, image, Pipe::Export);
        let untouched = bench.edit(&Edit::default(), Frame);
        let first = bench.first_step();
        let first = bench.render(&first);
        let step = jump(&untouched.pixels, &first.pixels, |_| true);
        let same = same_values(&untouched.pixels, &first.pixels);
        let apart = bits_apart(&untouched.pixels, &first.pixels);
        println!("S1 {what}: {step}; the same values: {same}, {apart} channels' bits apart");
        assert!(
            step.pixels > 0 && step.mean.is_finite() && step.max.is_finite(),
            "S1 {what}: {step}"
        );
    }
}

/// An export of `instances` within the engine's default budget, tiled when the raw does
/// not fit.
fn export_of(
    instances: &[Instance],
    image: &ImageContext,
    input: &GpuImage,
) -> Result<Vec<[f32; 4]>, ExportError> {
    let tiling = Tiling::Auto { budget: BUDGET };
    pollster::block_on(export(gpu(), instances, image, input.clone(), tiling))
        .map(|exported| exported.pixels)
}

/// S1 on every corpus raw the pipeline takes. Soften's whole-image scratch, 24 bytes a
/// pixel, is held across the export's tiles, so the export refuses it within the default
/// budget on the largest raws: those are reported, and the rest measured.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn s1_on_the_corpus() {
    let mut jumps = Vec::new();
    let mut same = 0;
    let mut refused = Vec::new();
    each_corpus_raw(|name, image, input| {
        let bench = export_bench(image.clone(), input.clone());
        let untouched = &bench.instances(&Edit::default(), Frame);
        let before = export_of(untouched, &image, &input).unwrap();
        let after = match export_of(&bench.first_step(), &image, &input) {
            Ok(after) => after,
            Err(error @ ExportError::NoTiles { .. }) => {
                println!("{name}: soften refused, {error}");
                refused.push(name.to_owned());
                return;
            }
            Err(error) => panic!("{name}: {error}"),
        };
        let step = jump(&before, &after, |_| true);
        let values = same_values(&before, &after);
        let apart = bits_apart(&before, &after);
        println!("{name}: {step}; the same values: {values}, {apart} channels' bits apart");
        assert!(
            step.mean.is_finite() && step.max.is_finite(),
            "{name}: {step}"
        );
        same += usize::from(values);
        jumps.push(step);
    });
    let worst_mean = jumps.iter().map(|j| j.mean).fold(0.0, f64::max);
    let worst_max = jumps.iter().map(|j| j.max).fold(0.0, f64::max);
    let worst_share = jumps.iter().map(Jump::share_over_one).fold(0.0, f64::max);
    println!(
        "S1 on {} raws: mean at most {worst_mean:.4}, max at most {worst_max:.3}, at most {:.3}% over 1.0; the same values on {same}; refused within the budget on {}: {refused:?}",
        jumps.len(),
        100.0 * worst_share,
        refused.len()
    );
}

// S2: each control moved, then reset.

/// Moves of every control: the mix's first step and darktable's 50 %, then, with the mix
/// at 50 %, the size, the saturation and the brightness each moved, and all four at the
/// golden case's values. Each changes the frame. The size moves the blur's radius in
/// whole pixels, 1 % of the image's diagonal at 100 %, so on the small images here only a
/// large move does: 100 % gives the sweep a radius of 4 where 50 % gives 2.
const MOVES: [&[(&str, f32)]; 6] = [
    &[("amount", 1.0)],
    &[("amount", 50.0)],
    &[("amount", 50.0), ("size", 100.0)],
    &[("amount", 50.0), ("saturation", 50.0)],
    &[("amount", 50.0), ("brightness", -1.0)],
    &[("size", 33.0), ("brightness", 0.5), ("amount", 75.0)],
];

/// Each move, then reset by `ResetControl` (a double-click or Delete) and by
/// `ResetModule` (the header's reset), on the bench's one pipeline, which soften joins
/// with each move and leaves with each reset.
fn s2_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    for set in MOVES {
        let mut edit = edit_of(set);
        let moved = bench.edit(&edit, Frame);
        assert!(!exact(&none, &moved), "{set:?} changes the frame");
        for (control, _) in set {
            edit.reset_control(&Soften, control).unwrap();
        }
        assert_eq!(edit, Edit::default());
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "{set:?}, reset control by control"
        );
        let mut edit = edit_of(set);
        bench.edit(&edit, Frame);
        edit.reset_module(&Soften);
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "{set:?}, the module reset"
        );
    }
}

#[test]
fn s2_a_control_moved_and_reset_renders_as_no_edit() {
    let (picture, image) = sweep(-0.67);
    s2_holds(&mut Bench::picture(&picture, image, Pipe::Export));
    let (mosaic, image) = small_raw(-0.67);
    let viewport = Pipe::Viewport {
        width: 40,
        height: 30,
    };
    s2_holds(&mut Bench::mosaic(&mosaic, image, viewport));
}

// S3: the order controls were set in.

const PAIRS: [[(&str, f32); 2]; 2] = [
    [("amount", 50.0), ("size", 100.0)],
    [("brightness", -0.5), ("amount", 75.0)],
];

/// One pipeline sets `first`, then both; another sets `second`, then both: the two
/// frames are the same, byte for byte.
fn s3_holds(new: impl Fn() -> Bench, [first, second]: [(&str, f32); 2]) {
    let mut one = new();
    one.set(&[first]);
    let one = one.set(&[first, second]);
    let mut other = new();
    other.set(&[second]);
    let other = other.set(&[second, first]);
    assert!(exact(&one, &other), "S3 with {first:?} and {second:?}");
}

#[test]
fn s3_controls_render_the_same_whichever_came_first() {
    let (picture, image) = sweep(-0.67);
    for pair in PAIRS {
        s3_holds(
            || Bench::picture(&picture, image.clone(), Pipe::Export),
            pair,
        );
    }
}

/// S2 and S3 on the 90D, through the viewport's frame and the export's.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn s2_and_s3_on_the_corpus() {
    for pipe in [CORPUS_VIEWPORT, Pipe::Export] {
        s2_holds(&mut corpus_bench(NINETY_D, pipe));
        for pair in PAIRS {
            s3_holds(|| corpus_bench(NINETY_D, pipe), pair);
        }
    }
}

// S4: darktable's golden case.

/// S4: the module's controls set to the values of the `soften` golden case, darktable's
/// test 0051-soften, each field as darktable's slider shows it, as it is, bind the case's
/// parameters bit for bit: the very parameter values the golden harness plans the
/// operation with against darktable's render. So the module renders those values as the
/// case measures the port: a ΔE2000 mean of 0.0001, max 0.0042 (`docs/ports/soften.md`).
#[test]
fn s4_the_golden_case_s_values_bind_to_darktable_s_parameters_bit_for_bit() {
    let schema = SoftenOperation.schema();
    let image = ImageContext::full(1, 1);
    let case = golden::cases::find(SOFTEN).unwrap();
    let darktable = golden::xmp::module_params(case.xmp, SOFTEN).unwrap();
    // darktable's fields, in its order, are the schema's.
    let fields: Vec<f32> = darktable
        .bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|word| f32::from_le_bytes(*word))
        .collect();
    let set: Vec<(&str, f32)> = schema
        .params
        .iter()
        .map(|param| param.name)
        .zip(fields.iter().copied())
        .collect();
    assert_eq!(set.len(), 4);
    let edit = edit_of(&set);

    let instances = edit::instances(&edit, OPERATIONS, MODULES, &image)
        .all()
        .unwrap();
    let instance = instances.iter().find(|i| id(i) == SOFTEN).unwrap();
    let ours = schema.resolve(&instance.params).unwrap();
    let harness = golden::ports::plan_values(case).unwrap().unwrap();
    let theirs = schema.resolve(&harness).unwrap();
    for (param, field) in set {
        let bits = |values: &ParamValues| match values.get(param) {
            Some(Value::Float(value)) => value.to_bits(),
            other => panic!("{param}: {other:?}"),
        };
        assert_eq!(bits(&ours), field.to_bits(), "{param}, darktable's {field}");
        assert_eq!(bits(&ours), bits(&theirs), "{param}, the harness's");
    }
}

// S6: the size, the saturation and the brightness do nothing while the mix is 0.

/// The size, the saturation and the brightness, each moved, at either end of its range
/// and together, while the mix is 0: the binding sets nothing, so soften stays out of the
/// plan and the frame is no edit's, `exact` comparing the plan's operations with their
/// cache keys as well as the pixels. They stay enabled there, as darktable's do.
#[test]
fn s6_the_other_controls_at_a_mix_of_0_render_as_no_edit() {
    let (picture, image) = sweep(-0.67);
    let mut bench = Bench::picture(&picture, image, Pipe::Export);
    let none = bench.edit(&Edit::default(), Frame);
    for set in [
        &[("size", 0.0)][..],
        &[("size", 100.0)],
        &[("saturation", 0.0)],
        &[("brightness", -2.0)],
        &[("brightness", 2.0)],
        &[("size", 33.0), ("saturation", 50.0), ("brightness", 0.5)],
    ] {
        let edit = edit_of(set);
        let values = edit.module(SOFTEN).unwrap();
        let defaults = Soften.defaults(&bench.image);
        let base = Base::new(OPERATIONS, &bench.image);
        assert!(
            Soften
                .bind(&values.off_defaults(&defaults), &bench.image, &base)
                .is_empty(),
            "{set:?}"
        );
        assert!(exact(&none, &bench.edit(&edit, Frame)), "S6 with {set:?}");
    }
}
