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

//! RGB primaries' invariants (`docs/modules/rgb-primaries.md`, Invariants), one test per
//! row, each named after its row: P0 to P4, the rows a first-pass table has (decision
//! 22), and P6, the tint's hue at a purity of 0. The synthetic ramp and sweep run on CI's
//! software GPUs, P4 on the CPU. The raw.pixls.us corpus runs locally, with
//! `APERTRUST_RAW_CORPUS` naming it, and darktable's chart with `APERTRUST_TEST_IMAGES`:
//!
//! ```sh
//! cargo test --release -p apertrust-modules --test rgb_primaries -- --ignored --nocapture
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

use std::path::PathBuf;

use apertrust_engine::compute::GpuImage;
use apertrust_engine::edit::{self, Edit};
use apertrust_engine::export::{BUDGET, Tiling, export};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe};
use apertrust_modules::rgb_primaries::{PARAMETERS, RgbPrimaries};
use apertrust_modules::{Base, ControlValue, MODULES, Module};
use apertrust_operations::OPERATIONS;
use apertrust_operations::primaries::{PARAMS_SIZE, Primaries};
use apertrust_plugin::{ImageContext, ImageDesc, Operation, ParamValues, Value};
use golden::synthetic;

use common::{
    BEFORE_THE_WORKING_SPACE, Bench, Jump, Picture, Rendered, exact, gpu, id, jump, small_raw,
    sweep,
};
use corpus::{CORPUS_VIEWPORT, NINETY_D, corpus_bench, decoded, each_corpus_raw, export_bench};

const PRIMARIES: &str = "primaries";

/// The ramp (`docs/modules/README.md`, Test images): neutral greys one per 1/16 EV from
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
            &RgbPrimaries,
            control,
            ControlValue::Number(value),
            &common::photo(),
        )
        .unwrap();
    }
    edit
}

impl Bench {
    /// The first step (row P1): rgb primaries planned at the module's own defaults, the
    /// operation's, over an unedited photo, as any control's first move off its default
    /// adds it.
    fn first_step(&self) -> Vec<Instance<'static>> {
        let mut instances = edit::instances(&Edit::default(), OPERATIONS, MODULES, &self.image)
            .all()
            .unwrap();
        instances.push(Instance {
            operation: apertrust_operations::operation(PRIMARIES).unwrap(),
            params: ParamValues::new(),
        });
        self.keep(instances, Frame)
    }

    fn set(&mut self, set: &[(&str, f32)]) -> Rendered {
        self.edit(&edit_of(set), Frame)
    }
}

/// Whether two frames' pixels are byte for byte the same, whatever their plans' keys.
fn same_pixels(a: &Rendered, b: &Rendered) -> bool {
    a.pixels.len() == b.pixels.len()
        && a.pixels
            .iter()
            .zip(&b.pixels)
            .all(|(x, y)| x.map(f32::to_bits) == y.map(f32::to_bits))
}

// P0: every control at its default.

/// Edits that leave every control at its default: each written at its default, and each
/// kept there.
fn untouched() -> Vec<Edit> {
    let mut written = Edit::default();
    let mut kept = Edit::default();
    for control in RgbPrimaries.definition().controls {
        written
            .set_control(
                &RgbPrimaries,
                control.id,
                control.default(),
                &common::photo(),
            )
            .unwrap();
        kept.keep(&RgbPrimaries, control.id, true).unwrap();
    }
    vec![written, kept]
}

/// P0 on one pipeline, as the session renders: no edit, then rgb primaries joining and
/// leaving the plan, then each untouched edit, every frame as the first.
fn p0_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    let joined = bench.first_step();
    bench.render(&joined);
    for edit in untouched() {
        let values = edit.module(PRIMARIES).unwrap();
        let defaults = RgbPrimaries.defaults(&bench.image);
        let base = Base::new(OPERATIONS, &bench.image);
        assert!(
            RgbPrimaries
                .bind(&values.off_defaults(&defaults), &bench.image, &base)
                .is_empty()
        );
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "P0 on {:?}",
            bench.pipe
        );
    }
}

#[test]
fn p0_every_control_at_its_default_renders_as_no_edit() {
    for pipe in [
        Pipe::Export,
        Pipe::Viewport {
            width: 40,
            height: 30,
        },
    ] {
        let (mosaic, image) = small_raw(-0.67);
        p0_holds(&mut Bench::mosaic(&mosaic, image, pipe));
    }
    for (picture, image) in [ramp(), sweep(-0.67)] {
        p0_holds(&mut Bench::picture(&picture, image, Pipe::Export));
    }
}

/// P0 on every corpus raw: the module at its defaults, written or kept, plans the very
/// instances and cache keys no edit does, so the same frame.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn p0_on_the_corpus() {
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
            assert_eq!(keys(&bench.instances(&edit, Frame)), none, "P0 on {name}");
        }
        raws += 1;
    });
    println!("P0 holds on {raws} raws");
}

// P1: the first step, measured and reported without a bound (decision 22).

/// The first step's jump on the ramp and on the sweep: all of it, inside the gamut (a
/// saturation of at most 1) and within 80% of it.
#[test]
fn p1_the_first_step_is_measured() {
    let jumps = |(picture, image): (Picture, ImageContext)| {
        let mut bench = Bench::picture(&picture, image, Pipe::Export);
        let untouched = bench.edit(&Edit::default(), Frame);
        let first = bench.first_step();
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

/// P1 on darktable's chart, `mire1.cr2`, test 0152's input.
#[test]
#[ignore = "needs darktable's integration images (APERTRUST_TEST_IMAGES)"]
fn p1_on_the_chart() {
    let dir = PathBuf::from(
        std::env::var_os("APERTRUST_TEST_IMAGES")
            .expect("APERTRUST_TEST_IMAGES names darktable's integration images"),
    );
    let (_, image, input) = decoded(&dir.join("mire1.cr2"));
    let step = first_step_on(&image, &input);
    println!("P1 on the chart: {step}");
    assert!(step.mean.is_finite() && step.max.is_finite(), "{step}");
}

/// P1 on every corpus raw the pipeline takes.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn p1_on_the_corpus() {
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
        "P1 on {} raws: mean at most {worst_mean:.4}, max at most {worst_max:.3}, at most {:.3}% over 1.0",
        jumps.len(),
        100.0 * worst_share
    );
}

// P2: each control moved, then reset.

/// Moves of every control: a primary's hue, a primary's purity, both of one primary, the
/// tint, and all eight at test 0152's values as its sliders show them. Each changes the
/// frame.
const MOVES: [&[(&str, f32)]; 5] = [
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

/// Each move, then reset by `ResetControl` (a double-click or Delete) and by
/// `ResetModule` (the header's reset), on the bench's one pipeline, which rgb primaries
/// joins with each move and leaves with each reset.
fn p2_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    for set in MOVES {
        let mut edit = edit_of(set);
        let moved = bench.edit(&edit, Frame);
        assert!(!exact(&none, &moved), "{set:?} changes the frame");
        for (control, _) in set {
            edit.reset_control(&RgbPrimaries, control).unwrap();
        }
        assert_eq!(edit, Edit::default());
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "{set:?}, reset control by control"
        );
        let mut edit = edit_of(set);
        bench.edit(&edit, Frame);
        edit.reset_module(&RgbPrimaries);
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "{set:?}, the module reset"
        );
    }
}

#[test]
fn p2_a_control_moved_and_reset_renders_as_no_edit() {
    let (picture, image) = sweep(-0.67);
    p2_holds(&mut Bench::picture(&picture, image, Pipe::Export));
    let (mosaic, image) = small_raw(-0.67);
    let viewport = Pipe::Viewport {
        width: 40,
        height: 30,
    };
    p2_holds(&mut Bench::mosaic(&mosaic, image, viewport));
}

// P3: the order controls were set in.

const PAIRS: [[(&str, f32); 2]; 2] = [
    [("red_hue", 9.5), ("achromatic_tint_purity", 13.4)],
    [("green_purity", 23.5), ("blue_hue", 13.1)],
];

/// One pipeline sets `first`, then both; another sets `second`, then both: the two
/// frames are the same, byte for byte.
fn p3_holds(new: impl Fn() -> Bench, [first, second]: [(&str, f32); 2]) {
    let mut one = new();
    one.set(&[first]);
    let one = one.set(&[first, second]);
    let mut other = new();
    other.set(&[second]);
    let other = other.set(&[second, first]);
    assert!(exact(&one, &other), "P3 with {first:?} and {second:?}");
}

#[test]
fn p3_controls_render_the_same_whichever_came_first() {
    let (picture, image) = sweep(-0.67);
    for pair in PAIRS {
        p3_holds(
            || Bench::picture(&picture, image.clone(), Pipe::Export),
            pair,
        );
    }
}

/// P2 and P3 on the 90D, through the viewport's frame and the export's.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn p2_and_p3_on_the_corpus() {
    for pipe in [CORPUS_VIEWPORT, Pipe::Export] {
        p2_holds(&mut corpus_bench(NINETY_D, pipe));
        for pair in PAIRS {
            p3_holds(|| corpus_bench(NINETY_D, pipe), pair);
        }
    }
}

// P4: darktable's golden case.

/// Little-endian bytes as four-byte words.
fn words(bytes: &[u8]) -> Vec<[u8; 4]> {
    bytes.as_chunks::<4>().0.to_vec()
}

/// How far apart P4 lets a matrix entry of the kernel's be from the golden case's:
/// two f32 steps of an entry of 1. A setting one step from darktable's moves an entry by
/// about that step times the entry's derivative, which is below 1 here.
const MATRIX: f32 = 2.4e-7;

/// P4: the module's controls set to the values of the `primaries` golden case,
/// darktable's test 0152-rgb-primaries, each field × factor + offset, unrounded, bind to
/// the case's parameters within one f32 step, the round trip's rounding, and, through the
/// operation's `prepare`, to the kernel inputs the golden harness checks against
/// darktable's render. So the module renders those values as the case measures the port:
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

    let instances = edit::instances(&edit, OPERATIONS, MODULES, &image)
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

/// The tint's hue moved while its purity is 0 plans rgb primaries, as any control's
/// first move does, and renders exactly the first step's pixels: the white moves by its
/// purity, so the matrix is the defaults'. It stays enabled there, as darktable's does.
#[test]
fn p6_the_tint_s_hue_at_purity_0_renders_as_the_first_step() {
    let (picture, image) = sweep(-0.67);
    let mut bench = Bench::picture(&picture, image, Pipe::Export);
    let first = bench.first_step();
    let first = bench.render(&first);
    for hue in [-180.0, -33.2, 90.0, 180.0] {
        let hued = bench.set(&[("achromatic_tint_hue", hue)]);
        assert!(same_pixels(&first, &hued), "the tint's hue at {hue}°");
    }
}
