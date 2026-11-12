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

//! The Tone equalizer's invariants (`docs/modules/tone-equalizer.md`, Invariants), one
//! test per row, each named after its row: a switched module's (decision 32), Q0 off, Q1
//! the first step, on at darktable's defaults, Q2 moved and reset, Q3 the order, Q4 the
//! darktable match, Q6 on at darktable's defaults on the CPU and Q7 version 1's edits; Q5,
//! on a layer, waits for Phase 4. The synthetic ramp and sweep run on CI's software GPUs,
//! Q4, Q6 and Q7 on the CPU. The raw.pixls.us corpus runs locally, with
//! `APERTRUST_RAW_CORPUS` naming it, and darktable's chart with `APERTRUST_TEST_IMAGES`:
//!
//! ```sh
//! cargo test --release -p apertrust-modules --test tone_equalizer -- --ignored --nocapture
//! ```
//!
//! Synthetic images enter after demosaic, already in the working space: their rows plan
//! the operations from exposure on and leave colorin out, so the tone equalizer, in camera
//! RGB in a raw's pipeline, reads them in the working space. Every row reads the frame,
//! colorout's output before it is rounded to 8 bits, clipped to 0 to 1 as a PNG holds it,
//! but Q1's correction, which reads the tone equalizer's own output.

#[expect(
    dead_code,
    reason = "The Tone equalizer's rows read neither the extended ramp nor a luminance"
)]
mod common;
mod corpus;
mod switched;

use std::path::PathBuf;

use apertrust_engine::compute::GpuImage;
use apertrust_engine::edit::{self, Edit};
use apertrust_engine::export::{BUDGET, Tiling, export};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe};
use apertrust_modules::tone_equalizer::{
    BANDS, DETAILS, ITERATIONS, METHOD, SMOOTHING, ToneEqualizer,
};
use apertrust_modules::{ControlValue, MODULES};
use apertrust_operations::OPERATIONS;
use apertrust_operations::toneequal::{self, Geometry, ToneEqual};
use apertrust_plugin::{ImageContext, Operation, Value};
use golden::synthetic;

use common::{
    BEFORE_THE_WORKING_SPACE, Bench, Jump, Picture, Rendered, exact, gpu, id, jump, small_raw,
    sweep,
};
use corpus::{CORPUS_VIEWPORT, NINETY_D, corpus_bench, decoded, each_corpus_raw, export_bench};

const TONE_EQUALIZER: &str = "toneequal";

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

/// What every row reads but Q1's correction: the frame, colorout's output, from exposure
/// on for a synthetic image.
#[derive(Debug, Clone, Copy)]
struct Frame;

impl common::Measure for Frame {
    fn keep(self, instances: &mut Vec<Instance<'static>>, synthetic: bool) {
        instances.retain(|i| !(synthetic && BEFORE_THE_WORKING_SPACE.contains(&id(i))));
    }
}

/// The image leaving the operations up to `last`, from exposure on: Q1's correction is
/// the tone equalizer's output over exposure's.
#[derive(Debug, Clone, Copy)]
struct Through(&'static str);

impl common::Measure for Through {
    fn keep(self, instances: &mut Vec<Instance<'static>>, synthetic: bool) {
        Frame.keep(instances, synthetic);
        let last = instances.iter().position(|i| id(i) == self.0).unwrap();
        instances.truncate(last + 1);
    }
}

fn value_of(control: &str, value: f32) -> ControlValue {
    if control == METHOD || control == DETAILS {
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
            &ToneEqualizer,
            control,
            value_of(control, value),
            &common::photo(),
        )
        .unwrap();
    }
    edit
}

impl Bench {
    /// The module on at darktable's defaults: the tone equalizer planned at the
    /// operation's defaults, darktable's new instance (rows Q1 and Q2).
    fn on_at_defaults(&self, measure: impl common::Measure) -> Vec<Instance<'static>> {
        self.instances(
            &switched::on_at_defaults(&ToneEqualizer, &self.image),
            measure,
        )
    }

    fn set(&mut self, set: &[(&str, f32)]) -> Rendered {
        self.edit(&edit_of(set), Frame)
    }
}

// Q0: off.

/// Q0 on one pipeline, as the session renders: no edit, then the tone equalizer joining
/// the plan, on at darktable's defaults, and leaving it, then each edit that leaves the
/// module off, every frame as the first.
fn q0_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    let joined = bench.on_at_defaults(Frame);
    bench.render(&joined);
    for edit in switched::off(&ToneEqualizer) {
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "Q0 on {:?}",
            bench.pipe
        );
    }
}

#[test]
fn q0_off_renders_as_no_edit() {
    for pipe in [
        Pipe::Export,
        Pipe::Viewport {
            width: 40,
            height: 30,
        },
    ] {
        let (mosaic, image) = small_raw(-0.67);
        q0_holds(&mut Bench::mosaic(&mosaic, image, pipe));
    }
    for (picture, image) in [ramp(), sweep(-0.67)] {
        q0_holds(&mut Bench::picture(&picture, image, Pipe::Export));
    }
}

// Q1: the first step, on at darktable's defaults, measured and reported without a bound
// (decision 22).

/// The zero bands' curve at the default smoothing, darktable's ripple: its least and
/// largest gain over the mask's range, −8 EV to 0 EV, from the setup's own curve in f64.
fn ripple() -> (f64, f64) {
    let prepared = toneequal::prepare(&toneequal::Settings::defaults(), &Geometry::full(8, 8))
        .expect("darktable's defaults are ported");
    (0..=8000)
        .map(|k| prepared.correction(-8.0 + f64::from(k) / 1000.0))
        .fold((f64::INFINITY, 0.0), |(low, high), c| {
            (low.min(c), high.max(c))
        })
}

/// The first step on the ramp: every grey's correction, the tone equalizer's output over
/// its input, lies on darktable's zero-band curve; and the frame's jump on the ramp and
/// on the sweep: all of it, inside the gamut (a saturation of at most 1) and within 80% of
/// it.
#[test]
fn q1_the_first_step_is_measured() {
    let (picture, image) = ramp();
    let mut bench = Bench::picture(&picture, image, Pipe::Export);
    let input = bench.edit(&Edit::default(), Through("exposure"));
    let output = bench.on_at_defaults(Through(TONE_EQUALIZER));
    let output = bench.render(&output);
    let gains: Vec<f64> = input
        .pixels
        .iter()
        .zip(&output.pixels)
        .flat_map(|(a, b)| (0..3).map(move |c| (a[c], b[c])))
        .filter(|&(a, _)| a > 1e-30)
        .map(|(a, b)| f64::from(b) / f64::from(a))
        .collect();
    let low = gains.iter().copied().fold(f64::INFINITY, f64::min);
    let high = gains.iter().copied().fold(0.0, f64::max);
    let (least, largest) = ripple();
    println!(
        "Q1 on the ramp: the greys' correction {:+.4} to {:+.4} EV; darktable's ripple {:+.4} to {:+.4} EV",
        low.log2(),
        high.log2(),
        least.log2(),
        largest.log2()
    );
    // One gain per pixel, on the curve: within the f32 rounding the operation's own GPU
    // test allows its curve, 2e-5 relative and 32 steps of the terms it adds up
    // (`every_band_at_zero_ripples_as_darktables`), here 5e-5 in all.
    assert_eq!(gains.len(), 3 * picture.pixels.len());
    assert!(
        low >= least * (1.0 - 5e-5) && high <= largest * (1.0 + 5e-5),
        "Q1: {low} to {high} outside darktable's ripple, {least} to {largest}"
    );

    let jumps = |bench: &mut Bench| {
        let untouched = bench.edit(&Edit::default(), Frame);
        let first = bench.on_at_defaults(Frame);
        let first = bench.render(&first);
        let of = |limit: f32| jump(&untouched.pixels, &first.pixels, |i| saturation(i) <= limit);
        [of(f32::INFINITY), of(1.0), of(0.8)]
    };
    let [greys, ..] = jumps(&mut bench);
    let (picture, image) = sweep(-0.67);
    let [whole, gamut, inside] = jumps(&mut Bench::picture(&picture, image, Pipe::Export));
    for (what, step) in [
        ("on the ramp", greys),
        ("on the sweep", whole),
        ("on the sweep, inside the gamut", gamut),
        ("on the sweep, within 80% of the gamut's saturation", inside),
    ] {
        println!("Q1 {what}: {step}");
        assert!(
            step.pixels > 0 && step.mean.is_finite() && step.max.is_finite(),
            "Q1 {what}: {step}"
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
    let after = export_of(&bench.on_at_defaults(Frame), image, input);
    jump(&before, &after, |_| true)
}

/// Q1 on darktable's chart, `mire1.cr2`, the golden cases' input.
#[test]
#[ignore = "needs darktable's integration images (APERTRUST_TEST_IMAGES)"]
fn q1_on_the_chart() {
    let dir = PathBuf::from(
        std::env::var_os("APERTRUST_TEST_IMAGES")
            .expect("APERTRUST_TEST_IMAGES names darktable's integration images"),
    );
    let (_, image, input) = decoded(&dir.join("mire1.cr2"));
    let step = first_step_on(&image, &input);
    println!("Q1 on the chart: {step}");
    assert!(step.mean.is_finite() && step.max.is_finite(), "{step}");
}

/// Q1 on every corpus raw the pipeline takes.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn q1_on_the_corpus() {
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
        "Q1 on {} raws: mean at most {worst_mean:.4}, max at most {worst_max:.3}, at most {:.3}% over 1.0",
        jumps.len(),
        100.0 * worst_share
    );
}

// Q2: each control moved, then reset.

/// Moves on every tab, and a write of a band at its default, which turns the module on:
/// each changes the frame, as the first step at least does.
const MOVES: [&[(&str, f32)]; 7] = [
    &[("midtones", 0.0)],
    // The −4 EV band.
    &[("shadows", 1.0)],
    &[("noise", -0.5), ("speculars", 0.75)],
    &[(SMOOTHING, 0.5), ("blacks", 0.5)],
    &[("exposure_boost", -1.0), ("contrast_boost", 0.5)],
    // No filter, on the power norm.
    &[(DETAILS, 0.0), (METHOD, 5.0)],
    // Averaged EIGF, with its settings.
    &[
        (DETAILS, 3.0),
        (ITERATIONS, 2.0),
        ("blending", 3.0),
        ("feathering", 3.0),
        ("quantization", 0.5),
    ],
];

/// Each move, then reset by `ResetControl` (a double-click or Delete), control by
/// control, which leaves the module on at darktable's defaults, and by `ResetModule` (the
/// header's reset), which turns it off, as no edit; on the bench's one pipeline, which
/// the tone equalizer joins with each move and leaves with each module's reset.
fn q2_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    let on = bench.on_at_defaults(Frame);
    let on = bench.render(&on);
    for set in MOVES {
        let mut edit = edit_of(set);
        let moved = bench.edit(&edit, Frame);
        assert!(!exact(&none, &moved), "{set:?} changes the frame");
        for (control, _) in set {
            edit.reset_control(&ToneEqualizer, control).unwrap();
        }
        assert!(
            edit.is_on(&ToneEqualizer),
            "{set:?}, reset control by control"
        );
        assert!(
            exact(&on, &bench.edit(&edit, Frame)),
            "{set:?}, reset control by control"
        );
        let mut edit = edit_of(set);
        bench.edit(&edit, Frame);
        edit.reset_module(&ToneEqualizer);
        assert_eq!(edit, Edit::default());
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "{set:?}, the module reset"
        );
    }
}

#[test]
fn q2_a_control_moved_and_reset_renders_as_on_and_the_module_reset_as_no_edit() {
    let (picture, image) = ramp();
    q2_holds(&mut Bench::picture(&picture, image, Pipe::Export));
    let (mosaic, image) = small_raw(-0.67);
    let viewport = Pipe::Viewport {
        width: 40,
        height: 30,
    };
    q2_holds(&mut Bench::mosaic(&mosaic, image, viewport));
}

// Q3: the order controls were set in.

const PAIRS: [[(&str, f32); 2]; 2] = [
    [("shadows", 1.0), ("exposure_boost", -1.0)],
    [(DETAILS, 0.0), (METHOD, 5.0)],
];

/// One pipeline sets `first`, then both; another sets `second`, then both: the two
/// frames are the same, byte for byte.
fn q3_holds(new: impl Fn() -> Bench, [first, second]: [(&str, f32); 2]) {
    let mut one = new();
    one.set(&[first]);
    let one = one.set(&[first, second]);
    let mut other = new();
    other.set(&[second]);
    let other = other.set(&[second, first]);
    assert!(exact(&one, &other), "Q3 with {first:?} and {second:?}");
}

#[test]
fn q3_controls_render_the_same_whichever_came_first() {
    let (picture, image) = sweep(-0.67);
    for pair in PAIRS {
        q3_holds(
            || Bench::picture(&picture, image.clone(), Pipe::Export),
            pair,
        );
    }
}

/// Q2 and Q3 on the 90D, through the viewport's frame and the export's.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn q2_and_q3_on_the_corpus() {
    for pipe in [CORPUS_VIEWPORT, Pipe::Export] {
        q2_holds(&mut corpus_bench(NINETY_D, pipe));
        for pair in PAIRS {
            q3_holds(|| corpus_bench(NINETY_D, pipe), pair);
        }
    }
}

/// Q0 on every corpus raw: the module off, untouched or kept, plans the very instances
/// and cache keys no edit does, so the same frame.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn q0_on_the_corpus() {
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
        for edit in switched::off(&ToneEqualizer) {
            assert_eq!(keys(&bench.instances(&edit, Frame)), none, "Q0 on {name}");
        }
        raws += 1;
    });
    println!("Q0 holds on {raws} raws");
}

// Q4: darktable's golden cases.

/// Little-endian bytes as four-byte words.
fn words(bytes: &[u8]) -> Vec<[u8; 4]> {
    bytes.as_chunks::<4>().0.to_vec()
}

/// How far a uniform word of ours is from the case's: 0 for the same bits, else the
/// relative difference of the two as floats.
fn apart(ours: [u8; 4], theirs: [u8; 4]) -> f32 {
    let (a, b) = (f32::from_le_bytes(ours), f32::from_le_bytes(theirs));
    if ours == theirs {
        0.0
    } else {
        (a - b).abs() / b.abs()
    }
}

/// Q4: the module's controls set to the values of the tone equalizer's golden cases,
/// darktable's tests 0080, 0129, 0131, 0133 and 0134 and the two edited from them, as
/// darktable's sliders show them, bind to the parameters the harness renders each case
/// with and, through the operation's setup, to the same kernel inputs, which the golden
/// harness checks against darktable's render: ΔE2000 means of 0.0002 to 0.0004
/// (`docs/ports/toneequal.md`, Results). Curve smoothing's round trip through darktable's
/// slider, `gui_update`'s log over log √2 less 1 and back, is the one step that can move a
/// bit; the curve it gives is held within 1e-6 relative.
#[test]
fn q4_the_golden_cases_values_bind_to_darktable_s_parameters() {
    let schema = ToneEqual.schema();
    let (width, height) = (3908, 2602);
    let image = ImageContext::full(width, height);
    let cases: Vec<_> = golden::cases::CASES
        .iter()
        .filter(|case| case.operation == TONE_EQUALIZER)
        .collect();
    assert_eq!(cases.len(), 7);
    for case in cases {
        let name = case.name;
        let darktable = golden::xmp::module_params(case.xmp, TONE_EQUALIZER).unwrap();
        // darktable's fields, in its order, are the schema's: nine bands, blending,
        // smoothing, feathering, quantization, contrast_boost and exposure_boost (f32),
        // then details, method and iterations (i32).
        let fields = words(&darktable.bytes);
        let field = |param: &str| {
            let i = schema.params.iter().position(|p| p.name == param).unwrap();
            fields[i]
        };
        let float = |param: &str| f32::from_le_bytes(field(param));
        let int = |param: &str| i32::from_le_bytes(field(param));
        let mut edit = Edit::default();
        let mut set = |control: &str, value: ControlValue| {
            edit.set_control(&ToneEqualizer, control, value, &common::photo())
                .unwrap();
        };
        let floats = BANDS.iter().copied().chain([
            "exposure_boost",
            "contrast_boost",
            "blending",
            "feathering",
            "quantization",
        ]);
        for param in floats {
            set(param, ControlValue::Number(float(param)));
        }
        // darktable's slider shows σ as `s`, its `gui_update`'s log over log √2 less 1.
        let sigma = float(SMOOTHING);
        let s = sigma.ln() / std::f32::consts::SQRT_2.ln() - 1.0;
        set(SMOOTHING, ControlValue::Number(s));
        set(ITERATIONS, ControlValue::Number(int(ITERATIONS) as f32));
        set(METHOD, ControlValue::Choice(int(METHOD) as u32));
        set(DETAILS, ControlValue::Choice(int(DETAILS) as u32));

        let instances = edit::instances(&edit, OPERATIONS, MODULES, &image)
            .all()
            .unwrap();
        let instance = instances.iter().find(|i| id(i) == TONE_EQUALIZER).unwrap();
        let ours = schema.resolve(&instance.params).unwrap();
        let theirs = schema
            .resolve(&golden::ports::plan_values(case).unwrap().unwrap())
            .unwrap();
        for param in schema.params {
            let (a, b) = (ours.get(param.name), theirs.get(param.name));
            match (a, b) {
                (Some(Value::Float(a)), Some(Value::Float(b))) if param.name == SMOOTHING => {
                    assert!(
                        (a - b).abs() <= 2.0 * f32::EPSILON * b,
                        "{name}: σ is {a}, darktable's {b}"
                    );
                    if a != b {
                        println!("Q4 {name}: σ {a}, darktable's {b}");
                    }
                }
                _ => assert_eq!(a, b, "{name}: {}", param.name),
            }
        }

        let geometry = Geometry::full(width, height);
        let ours = toneequal::prepare(&toneequal::settings(&ours), &geometry).unwrap();
        let theirs = toneequal::prepare(&toneequal::settings(&theirs), &geometry).unwrap();
        let (our_passes, their_passes) = (ours.passes(), theirs.passes());
        assert_eq!(our_passes.len(), their_passes.len(), "{name}");
        let mut worst = 0.0_f32;
        for (a, b) in our_passes.iter().zip(&their_passes) {
            assert_eq!(a.entry_point, b.entry_point, "{name}");
            assert_eq!(a.tables, b.tables, "{name}: {}", a.entry_point);
            assert_eq!(a.params.len(), b.params.len(), "{name}: {}", a.entry_point);
            for (x, y) in words(&a.params).into_iter().zip(words(&b.params)) {
                worst = worst.max(apart(x, y));
            }
        }
        let curve = (0..=800)
            .map(|k| -8.0 + f64::from(k) / 100.0)
            .map(|e| (ours.correction(e) / theirs.correction(e) - 1.0).abs())
            .fold(0.0, f64::max);
        assert!(curve <= 1e-6, "{name}: the curve {curve:e} apart");
        println!(
            "Q4 {name}: the kernel's uniforms at most {worst:.1e} from the case's, the curve {curve:.1e}"
        );
    }
}

// Q6: on at darktable's defaults, on the CPU.

/// The module's defaults, each made explicit, bind the tone equalizer's own, and the
/// module on at its defaults plans it with no parameter set, darktable's new instance;
/// off, it plans none.
#[test]
fn q6_on_at_darktable_s_defaults_binds_the_operation_s_defaults() {
    switched::on_at_defaults_holds(&ToneEqualizer, TONE_EQUALIZER);
}

// Q7: version 1's edits.

/// Edits version 1 saved, read as a photo opens them, bind the parameters version 1
/// bound: the tone equalizer planned where a value was off its default, with what the
/// binding gives those values, and not planned, the module off, where none was.
#[test]
fn q7_version_1_s_edits_bind_what_they_bound() {
    let image = common::photo();
    for (saved, values) in [
        (&[("shadows", "1")][..], &[("shadows", 1.0)][..]),
        (
            &[("noise", "-0.5"), ("speculars", "0.75"), ("midtones", "0")],
            &[("noise", -0.5), ("speculars", 0.75)],
        ),
        (&[(SMOOTHING, "0.5")], &[(SMOOTHING, 0.5)]),
    ] {
        let edit = switched::version_1(TONE_EQUALIZER, saved);
        assert!(edit.is_on(&ToneEqualizer), "{saved:?}");
        let params = switched::bound(&edit, TONE_EQUALIZER, &image).expect("planned");
        let then = switched::bound(&edit_of(values), TONE_EQUALIZER, &image).unwrap();
        assert_eq!(params, then, "{saved:?}");
        assert!(params.iter().next().is_some(), "{saved:?}");
    }
    for saved in [&[("whites", "0")][..], &[]] {
        let edit = switched::version_1(TONE_EQUALIZER, saved);
        assert!(!edit.is_on(&ToneEqualizer), "{saved:?}");
        assert!(edit.is_empty(), "{saved:?}");
        assert!(
            switched::bound(&edit, TONE_EQUALIZER, &image).is_none(),
            "{saved:?}"
        );
    }
}
