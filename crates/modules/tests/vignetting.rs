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

//! Vignetting's invariants (`docs/modules/vignetting.md`, Invariants), one test per row,
//! each named after its row: a switched module's (decision 32), V0 off, V1 the first
//! step, on at darktable's defaults, V2 moved and reset, V3 the order, V4 the darktable
//! match, V7 on at darktable's defaults on the CPU and V8 version 1's edits; V5, on a
//! layer, waits for Phase 4; and V6, the width/height ratio while automatic ratio is on
//! (decision 30). The synthetic ramp and sweep and the small raw run on CI's software
//! GPUs, V4, V7 and V8 on the CPU. The raw.pixls.us corpus runs locally, with
//! `APERTRUST_RAW_CORPUS` naming it:
//!
//! ```sh
//! cargo test --release -p apertrust-modules --test vignetting -- --ignored --nocapture
//! ```
//!
//! Synthetic images enter after demosaic, already in the working space: their rows plan
//! the operations from exposure on and leave colorin out. Every row reads the frame,
//! colorout's output before it is rounded to 8 bits, clipped to 0 to 1 as a PNG holds it.

#[expect(
    dead_code,
    reason = "Vignetting's rows read neither the extended ramp nor a luminance"
)]
mod common;
mod corpus;
mod switched;

use apertrust_engine::edit::{self, Edit};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe};
use apertrust_modules::vignetting::Vignetting;
use apertrust_modules::{ControlValue, Controls, MODULES, Module};
use apertrust_operations::OPERATIONS;
use apertrust_operations::vignette::{self, PARAMS_SIZE, Vignette};
use apertrust_plugin::{ImageContext, ImageDesc, Operation, ParamValues, Value};

use common::{
    BEFORE_THE_WORKING_SPACE, Bench, Picture, Rendered, exact, id, jump, small_raw, sweep,
};
use corpus::{CORPUS_VIEWPORT, NINETY_D, corpus_bench, each_corpus_raw, export_bench};

const VIGNETTE: &str = "vignette";

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

/// A number, or automatic ratio on or off.
fn number(value: f32) -> ControlValue {
    ControlValue::Number(value)
}

const ON: ControlValue = ControlValue::Bool(true);

/// An edit with these controls set, in this order.
fn edit_of(set: &[(&str, ControlValue)]) -> Edit {
    let mut edit = Edit::default();
    for (control, value) in set {
        edit.set_control(&Vignetting, control, value.clone(), &common::photo())
            .unwrap();
    }
    edit
}

impl Bench {
    /// The module on at darktable's defaults, brightness and saturation −0.5 among them:
    /// the vignette planned at the operation's defaults, darktable's new instance (rows
    /// V1 and V2).
    fn on_at_defaults(&self) -> Vec<Instance<'static>> {
        self.instances(&switched::on_at_defaults(&Vignetting, &self.image), Frame)
    }

    fn set(&mut self, set: &[(&str, ControlValue)]) -> Rendered {
        self.edit(&edit_of(set), Frame)
    }
}

/// Whether two frames' pixels hold the same values, a zero of either sign equal to the
/// other, whatever their plans' keys.
fn same_values(a: &Rendered, b: &Rendered) -> bool {
    a.pixels.len() == b.pixels.len() && a.pixels.iter().zip(&b.pixels).all(|(x, y)| x == y)
}

/// The pipes the exact rows check on the small raw: the export's and the viewport's.
const PIPES: [Pipe; 2] = [
    Pipe::Export,
    Pipe::Viewport {
        width: 40,
        height: 30,
    },
];

// V0: off.

/// V0 on one pipeline, as the session renders: no edit, then the vignette joining the
/// plan, on at darktable's defaults, and leaving it, then each edit that leaves the
/// module off, every frame as the first.
fn v0_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    let joined = bench.on_at_defaults();
    bench.render(&joined);
    for edit in switched::off(&Vignetting) {
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "V0 on {:?}",
            bench.pipe
        );
    }
}

#[test]
fn v0_off_renders_as_no_edit() {
    for pipe in PIPES {
        let (mosaic, image) = small_raw(-0.67);
        v0_holds(&mut Bench::mosaic(&mosaic, image, pipe));
    }
    for (picture, image) in [ramp(), sweep(-0.67)] {
        v0_holds(&mut Bench::picture(&picture, image, Pipe::Export));
    }
}

/// V0 on every corpus raw: the module off, untouched or kept, plans the very instances
/// and cache keys no edit does, so the same frame.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn v0_on_the_corpus() {
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
        for edit in switched::off(&Vignetting) {
            assert_eq!(keys(&bench.instances(&edit, Frame)), none, "V0 on {name}");
        }
        raws += 1;
    });
    println!("V0 holds on {raws} raws");
}

// V1: the first step, on at darktable's defaults, measured and reported without a bound
// (decision 22).

/// V1 on one pipeline: the module on at darktable's defaults, darktable's new instance
/// darkening and desaturating towards the corners, against no edit; its jump.
fn v1_jump(bench: &mut Bench) -> common::Jump {
    let none = bench.edit(&Edit::default(), Frame);
    let first = bench.on_at_defaults();
    assert!(first.iter().any(|i| id(i) == VIGNETTE));
    let first = bench.render(&first);
    jump(&none.pixels, &first.pixels, |_| true)
}

#[test]
fn v1_the_first_step_is_measured() {
    for (what, (picture, image)) in [("on the ramp", ramp()), ("on the sweep", sweep(-0.67))] {
        let step = v1_jump(&mut Bench::picture(&picture, image, Pipe::Export));
        println!("V1 {what}: {step}");
        assert!(step.mean.is_finite() && step.max.is_finite(), "{step}");
    }
    let (mosaic, image) = small_raw(-0.67);
    let step = v1_jump(&mut Bench::mosaic(&mosaic, image, Pipe::Export));
    println!("V1 on the small raw: {step}");
    assert!(step.pixels > 0 && step.mean > 0.0, "{step}");
}

/// V1 on every corpus raw the pipeline takes, through the export.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn v1_on_the_corpus() {
    let mut jumps = Vec::new();
    each_corpus_raw(|name, image, input| {
        let step = v1_jump(&mut export_bench(image, input));
        println!("{name}: {step}");
        assert!(step.mean.is_finite() && step.max.is_finite(), "{name}");
        jumps.push(step);
    });
    let worst_mean = jumps.iter().map(|j| j.mean).fold(0.0, f64::max);
    let worst_max = jumps.iter().map(|j| j.max).fold(0.0, f64::max);
    println!(
        "V1 on {} raws: mean at most {worst_mean:.4}, max at most {worst_max:.3}",
        jumps.len()
    );
}

// V2: each control moved, then reset.

/// Moves of every control: a write of the brightness at darktable's −0.5, which turns the
/// module on; the strengths, the radii, the centre and shape, automatic ratio, and every
/// control at test 0045's values, the `vignette` golden case's. Each changes the frame.
fn moves() -> [Vec<(&'static str, ControlValue)>; 7] {
    [
        vec![("brightness", number(-0.5))],
        vec![("scale", number(40.0))],
        vec![("saturation", number(0.3)), ("scale", number(40.0))],
        vec![
            ("falloff_scale", number(100.0)),
            ("brightness", number(-1.0)),
        ],
        vec![
            ("center_x", number(-0.116)),
            ("center_y", number(0.13)),
            ("shape", number(1.49)),
            ("brightness", number(0.37)),
        ],
        vec![("autoratio", ON), ("saturation", number(-0.43))],
        vec![
            ("scale", number(69.4)),
            ("falloff_scale", number(32.08)),
            ("brightness", number(-0.968)),
            ("saturation", number(-0.82)),
            ("center_x", number(-0.116)),
            ("center_y", number(0.13)),
            ("shape", number(1.49)),
            ("whratio", number(1.172)),
        ],
    ]
}

/// Each move, then reset by `ResetControl` (a double-click or Delete), control by
/// control, which leaves the module on at darktable's defaults, and by `ResetModule` (the
/// header's reset), which turns it off, as no edit; on the bench's one pipeline, which
/// the vignette joins with each move and leaves with each module's reset.
fn v2_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    let on = bench.on_at_defaults();
    let on = bench.render(&on);
    for set in moves() {
        let mut edit = edit_of(&set);
        let moved = bench.edit(&edit, Frame);
        assert!(!exact(&none, &moved), "{set:?} changes the frame");
        for (control, _) in &set {
            edit.reset_control(&Vignetting, control).unwrap();
        }
        assert!(edit.is_on(&Vignetting), "{set:?}, reset control by control");
        assert!(
            exact(&on, &bench.edit(&edit, Frame)),
            "{set:?}, reset control by control"
        );
        let mut edit = edit_of(&set);
        bench.edit(&edit, Frame);
        edit.reset_module(&Vignetting);
        assert_eq!(edit, Edit::default());
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "{set:?}, the module reset"
        );
    }
}

#[test]
fn v2_a_control_moved_and_reset_renders_as_on_and_the_module_reset_as_no_edit() {
    let (picture, image) = sweep(-0.67);
    v2_holds(&mut Bench::picture(&picture, image, Pipe::Export));
    let (mosaic, image) = small_raw(-0.67);
    v2_holds(&mut Bench::mosaic(&mosaic, image, PIPES[1]));
}

// V3: the order controls were set in.

fn pairs() -> [[(&'static str, ControlValue); 2]; 3] {
    [
        [("brightness", number(-0.6)), ("scale", number(40.0))],
        [("saturation", number(0.3)), ("autoratio", ON)],
        [("brightness", number(-0.81)), ("shape", number(3.7))],
    ]
}

/// One pipeline sets `first`, then both; another sets `second`, then both: the two
/// frames are the same, byte for byte.
fn v3_holds(new: impl Fn() -> Bench, [first, second]: [(&str, ControlValue); 2]) {
    let mut one = new();
    one.set(std::slice::from_ref(&first));
    let one = one.set(&[first.clone(), second.clone()]);
    let mut other = new();
    other.set(std::slice::from_ref(&second));
    let other = other.set(&[second.clone(), first.clone()]);
    assert!(exact(&one, &other), "V3 with {first:?} and {second:?}");
}

#[test]
fn v3_controls_render_the_same_whichever_came_first() {
    let (picture, image) = sweep(-0.67);
    for pair in pairs() {
        v3_holds(
            || Bench::picture(&picture, image.clone(), Pipe::Export),
            pair,
        );
    }
}

/// V2 and V3 on the 90D, through the viewport's frame and the export's.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn v2_and_v3_on_the_corpus() {
    for pipe in [CORPUS_VIEWPORT, Pipe::Export] {
        v2_holds(&mut corpus_bench(NINETY_D, pipe));
        for pair in pairs() {
            v3_holds(|| corpus_bench(NINETY_D, pipe), pair);
        }
    }
}

// V4: darktable's golden cases.

/// Little-endian bytes as four-byte words.
fn words(bytes: &[u8]) -> Vec<[u8; 4]> {
    bytes.as_chunks::<4>().0.to_vec()
}

/// The golden cases a module's values can reach: all but `vignette-clip`, whose clipping
/// (`unbound` off) the module fixes on.
const CASES: [&str; 5] = [
    "vignette",
    "vignette-autoratio",
    "vignette-narrow",
    "vignette-star",
    "vignette-white",
];

/// The words of the kernel's parameters that settings fill: its values and flags, before
/// the gradient slots, which only the derivative entry points read and the harness
/// chooses for each case.
const SETTINGS_WORDS: usize = 12;

/// V4: the module's controls set to each golden case's values, darktable's fields as they
/// are, bind darktable's parameters bit for bit, and, through the operation's `prepare`,
/// the kernel inputs the golden harness checks against darktable's render. So the module
/// renders those values as the cases measure the port: a ΔE2000 mean of 0.0000, max
/// 0.0002 (`docs/ports/vignette.md`, Results).
#[test]
fn v4_the_golden_cases_values_bind_darktable_s_parameters_bit_for_bit() {
    let schema = Vignette.schema();
    let image = ImageContext::full(1, 1);
    for name in CASES {
        let case = golden::cases::find(name).unwrap();
        let darktable = golden::xmp::module_params(case.xmp, VIGNETTE).unwrap();
        assert_eq!(
            (darktable.version, darktable.bytes.len()),
            (4, 44),
            "{name}"
        );
        // darktable's version 4 fields, in its order: the radii, the strengths, the
        // centre, automatic ratio, the ratio, the shape, dithering and clipping.
        let fields = words(&darktable.bytes);
        let float = |i: usize| f32::from_le_bytes(fields[i]);
        let flag = |i: usize| i32::from_le_bytes(fields[i]) != 0;
        assert_eq!(i32::from_le_bytes(fields[9]), 0, "{name}: dithering off");
        assert!(flag(10), "{name}: unbound");
        let set: Vec<(&str, ControlValue)> = vec![
            ("scale", number(float(0))),
            ("falloff_scale", number(float(1))),
            ("brightness", number(float(2))),
            ("saturation", number(float(3))),
            ("center_x", number(float(4))),
            ("center_y", number(float(5))),
            ("autoratio", ControlValue::Bool(flag(6))),
            ("whratio", number(float(7))),
            ("shape", number(float(8))),
        ];
        let edit = edit_of(&set);

        let instances = edit::instances(&edit, OPERATIONS, MODULES, &image)
            .all()
            .unwrap();
        let instance = instances.iter().find(|i| id(i) == VIGNETTE).unwrap();
        let params = schema.resolve(&instance.params).unwrap();
        let ours = vignette::settings(&params);
        assert_eq!(ours.scale.to_bits(), float(0).to_bits(), "{name}");
        assert_eq!(ours.falloff_scale.to_bits(), float(1).to_bits(), "{name}");
        assert_eq!(ours.brightness.to_bits(), float(2).to_bits(), "{name}");
        assert_eq!(ours.saturation.to_bits(), float(3).to_bits(), "{name}");
        assert_eq!(
            ours.center.map(f32::to_bits),
            [4, 5].map(|i| float(i).to_bits())
        );
        assert_eq!(ours.autoratio, flag(6), "{name}");
        assert_eq!(ours.whratio.to_bits(), float(7).to_bits(), "{name}");
        assert_eq!(ours.shape.to_bits(), float(8).to_bits(), "{name}");
        assert_eq!(ours.dithering, vignette::Dithering::Off, "{name}");
        assert!(ours.unbound, "{name}");

        let prepared = Vignette
            .prepare(&params, &image, &ImageDesc::rgba(1, 1))
            .unwrap();
        let ours = words(&prepared.passes[0].params);
        let theirs = words(&golden::ports::kernel_inputs(case).unwrap().unwrap().params);
        assert_eq!(
            (ours.len(), theirs.len()),
            (PARAMS_SIZE / 4, PARAMS_SIZE / 4)
        );
        assert_eq!(ours[..SETTINGS_WORDS], theirs[..SETTINGS_WORDS], "{name}");
    }
}

/// The case the module cannot reach: `vignette-clip` clips, which the module never does.
#[test]
fn v4_the_clipped_case_is_out_of_the_module_s_reach() {
    let case = golden::cases::find("vignette-clip").unwrap();
    let darktable = golden::xmp::module_params(case.xmp, VIGNETTE).unwrap();
    let fields = words(&darktable.bytes);
    assert_eq!(i32::from_le_bytes(fields[10]), 0, "unbound off");
    let vignette_cases = golden::cases::CASES
        .iter()
        .filter(|case| case.operation == VIGNETTE)
        .count();
    assert_eq!(vignette_cases, CASES.len() + 1);
}

// V6: the width/height ratio while automatic ratio is on.

/// V6: with automatic ratio on, the width/height ratio is unavailable, as darktable makes
/// it insensitive, and changes nothing: the frame is the same whatever it holds. With
/// automatic ratio off it is available and acts.
#[test]
fn v6_the_ratio_is_unavailable_and_changes_nothing_while_automatic_ratio_is_on() {
    let (picture, image) = sweep(-0.67);
    let mut values = Vignetting.defaults(&image);
    values.set("autoratio", ON);
    let available = |values: &Controls| Vignetting.available("whratio", values, &image);
    assert!(!available(&values));
    let mut bench = Bench::picture(&picture, image.clone(), Pipe::Export);
    let render = |bench: &mut Bench, autoratio: bool, whratio: f32| {
        bench.set(&[
            ("brightness", number(-0.6)),
            ("autoratio", ControlValue::Bool(autoratio)),
            ("whratio", number(whratio)),
        ])
    };
    let wide = render(&mut bench, true, 0.5);
    let tall = render(&mut bench, true, 1.6);
    assert!(
        same_values(&wide, &tall),
        "the ratio while automatic ratio is on"
    );
    values.set("autoratio", ControlValue::Bool(false));
    assert!(available(&values));
    let wide = render(&mut bench, false, 0.5);
    let tall = render(&mut bench, false, 1.6);
    assert!(
        !same_values(&wide, &tall),
        "the ratio while automatic ratio is off"
    );
}

// V7: on at darktable's defaults, on the CPU.

/// The module's defaults, each made explicit, bind the vignette's own, `unbound` on among
/// them, and the module on at its defaults plans it with no parameter set, darktable's new
/// instance; off, it plans none.
#[test]
fn v7_on_at_darktable_s_defaults_binds_the_operation_s_defaults() {
    switched::on_at_defaults_holds(&Vignetting, VIGNETTE);
}

// V8: version 1's edits.

/// Edits version 1 saved, read as a photo opens them, bind the parameters version 1
/// bound, resolved over the schema: the vignette planned where brightness or saturation
/// was off 0, version 1's untouched value, both bound, the other at 0, and `unbound` on;
/// not planned, the module off, where neither was.
#[test]
fn v8_version_1_s_edits_bind_what_they_bound() {
    let image = common::photo();
    let schema = Vignette.schema();
    let resolved = |set: &[(&str, f32)]| {
        let mut params = ParamValues::new();
        for &(param, value) in set {
            params.set(param, Value::Float(value));
        }
        params.set("unbound", Value::Bool(true));
        schema.resolve(&params).unwrap()
    };
    for (saved, bound) in [
        (
            &[("saturation", "0.3"), ("scale", "40")][..],
            &[("brightness", 0.0), ("saturation", 0.3), ("scale", 40.0)][..],
        ),
        (
            &[("brightness", "-0.5")],
            &[("brightness", -0.5), ("saturation", 0.0)],
        ),
        (
            &[
                ("brightness", "-0.968"),
                ("saturation", "-0.82"),
                ("shape", "1.49"),
            ],
            &[
                ("brightness", -0.968),
                ("saturation", -0.82),
                ("shape", 1.49),
            ],
        ),
    ] {
        let edit = switched::version_1(VIGNETTE, saved);
        assert!(edit.is_on(&Vignetting), "{saved:?}");
        let params = switched::bound(&edit, VIGNETTE, &image).expect("planned");
        assert_eq!(
            schema.resolve(&params).unwrap(),
            resolved(bound),
            "{saved:?}"
        );
    }
    for saved in [
        &[("scale", "40"), ("autoratio", "true")][..],
        &[("brightness", "0"), ("saturation", "0")],
        &[],
    ] {
        let edit = switched::version_1(VIGNETTE, saved);
        assert!(!edit.is_on(&Vignetting), "{saved:?}");
        assert!(edit.is_empty(), "{saved:?}");
        assert!(
            switched::bound(&edit, VIGNETTE, &image).is_none(),
            "{saved:?}"
        );
    }
}
