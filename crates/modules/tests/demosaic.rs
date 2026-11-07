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

//! Demosaic's invariants (`docs/modules/demosaic.md`, Invariants), one test per row, each
//! named after its row: D0 to D4. Demosaic works on the mosaic, so every row renders a
//! raw: the small Bayer raw and a small X-Trans one on CI's software GPUs, and the
//! raw.pixls.us corpus locally, with `APERTRUST_RAW_CORPUS` naming it:
//!
//! ```sh
//! cargo test --release -p apertrust-modules --test demosaic -- --ignored --nocapture
//! ```
//!
//! D5, the rows following the method as darktable's `gui_changed` shows them, is a unit
//! test in `crates/modules/src/demosaic.rs`. Every row reads the frame, colorout's output
//! before it is rounded to 8 bits.

#[expect(
    dead_code,
    reason = "Demosaic's rows read raws only: neither the synthetic pictures nor a jump"
)]
mod common;
mod corpus;

use apertrust_engine::edit::{Edit, EditError};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe};
use apertrust_modules::demosaic::{Demosaic, METHODS};
use apertrust_modules::{Base, ControlError, ControlValue, Module};
use apertrust_operations::OPERATIONS;
use apertrust_plugin::{ImageContext, Pattern, Value};

use common::{Bench, exact, small_raw};
use corpus::{CORPUS_VIEWPORT, NINETY_D, corpus_bench, each_corpus_raw, export_bench};

const DEMOSAIC: &str = "demosaic";
const METHOD: &str = "demosaicing_method";

/// The Fujifilm X-T5, an X-Trans raw.
const X_T5: &str = "Fujifilm/X-T5/DSCF0021.RAF";

/// darktable's X-Trans pattern as the X-T5 and the X100VI carry it.
fn x_trans_pattern() -> Pattern<u32> {
    Pattern {
        width: 6,
        height: 6,
        values: vec![
            1, 1, 0, 1, 1, 2, //
            1, 1, 2, 1, 1, 0, //
            2, 0, 1, 0, 2, 1, //
            1, 1, 2, 1, 1, 0, //
            1, 1, 0, 1, 1, 2, //
            0, 2, 1, 2, 0, 1, //
        ],
    }
}

/// A small X-Trans raw, the small Bayer raw's size, crop and gradient through its
/// mosaic's colours, with an X-Trans pattern: Markesteijn 1-pass in its interior and
/// darktable's linear VNG in its edge band.
fn small_x_trans() -> (Vec<u16>, ImageContext) {
    let (_, mut image) = small_raw(-0.67);
    let pattern = x_trans_pattern();
    let camera = image.camera.as_mut().unwrap();
    let (width, height) = (camera.width, camera.height);
    let mosaic = (0..width * height)
        .map(|i| {
            let (x, y) = (i % width, i / width);
            let c = pattern.values[((y % 6) * 6 + x % 6) as usize] as f32;
            let level =
                0.1 + 0.6 * x as f32 / width as f32 + 0.1 * c + 0.2 * y as f32 / height as f32;
            (level.min(1.0) * 4000.0).round() as u16
        })
        .collect();
    camera.cfa = Some(pattern);
    (mosaic, image)
}

/// The small raws, Bayer then X-Trans, on a bench through `pipe`.
fn small_benches(pipe: Pipe) -> [Bench; 2] {
    let (bayer, image) = small_raw(-0.67);
    let (x_trans, x_image) = small_x_trans();
    [
        Bench::mosaic(&bayer, image, pipe),
        Bench::mosaic(&x_trans, x_image, pipe),
    ]
}

/// The pipes the rows check on the small raws: the export's and the viewport's.
const PIPES: [Pipe; 2] = [
    Pipe::Export,
    Pipe::Viewport {
        width: 40,
        height: 30,
    },
];

/// What every row reads: the frame, colorout's output, from the mosaic.
#[derive(Debug, Clone, Copy)]
struct Frame;

impl common::Measure for Frame {
    fn keep(self, _instances: &mut Vec<Instance<'static>>, _synthetic: bool) {}
}

fn method(name: &str) -> ControlValue {
    let index = METHODS.iter().position(|&m| m == name).unwrap();
    ControlValue::Choice(index as u32)
}

/// The method the photo's sensor takes, and the other sensor's ported one, which the
/// operation falls back from, as darktable's `commit_params` does.
fn methods(image: &ImageContext) -> (&'static str, &'static str) {
    let x_trans = image
        .camera
        .as_ref()
        .and_then(|camera| camera.cfa.as_ref())
        .is_some_and(|cfa| cfa.width == 6);
    if x_trans {
        ("markesteijn", "rcd")
    } else {
        ("rcd", "markesteijn")
    }
}

// D0: every control at its default.

/// Edits that leave the photo untouched: every control written at its default for the
/// photo, the photo's own method among them, and every control kept there.
fn untouched(image: &ImageContext) -> [Edit; 2] {
    let defaults = Demosaic.defaults(image);
    let mut written = Edit::default();
    let mut kept = Edit::default();
    for control in Demosaic.definition().controls {
        let value = defaults.get(control.id).unwrap().clone();
        written
            .set_control(&Demosaic, control.id, value, image)
            .unwrap();
        kept.keep(&Demosaic, control.id, true).unwrap();
    }
    [written, kept]
}

/// D0 on one pipeline: no edit, then each untouched edit, every frame as the first, and
/// the binding empty for each.
fn d0_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    let base = Base::new(OPERATIONS, &bench.image);
    for edit in untouched(&bench.image) {
        let values = edit.module(DEMOSAIC).unwrap();
        let values = values.off_defaults(&Demosaic.defaults(&bench.image));
        assert!(Demosaic.bind(&values, &bench.image, &base).is_empty());
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "D0 on {:?}",
            bench.pipe
        );
    }
}

#[test]
fn d0_every_control_at_its_default_renders_as_no_edit() {
    for pipe in PIPES {
        for mut bench in small_benches(pipe) {
            d0_holds(&mut bench);
        }
    }
}

/// D0 on every corpus raw: the module at its defaults, written or kept, plans the very
/// instances and cache keys no edit does, so the same frame.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn d0_on_the_corpus() {
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
        for edit in untouched(&image) {
            assert_eq!(keys(&bench.instances(&edit, Frame)), none, "D0 on {name}");
        }
        raws += 1;
    });
    println!("D0 holds on {raws} raws");
}

// D1: the method moved, then reset.

/// A photo of the other sensor than `image`'s: the small X-Trans raw's for a Bayer one,
/// the small Bayer raw's for an X-Trans one.
fn other_photo(image: &ImageContext) -> ImageContext {
    match methods(image).0 {
        "markesteijn" => small_raw(-0.67).1,
        _ => small_x_trans().1,
    }
}

/// D1 on one pipeline: the other sensor's ported method, the only other value a ported
/// control can hold, reaches an edit only from that sensor's photo (decision 30): this
/// photo refuses to set it (`NotListed`) and keeps it when the edit holds it. Held, it
/// binds, and the operation falls back to the photo's own, as darktable's
/// `commit_params` does, preparing the very passes it prepares for no edit, so the frame
/// is no edit's, its keys too, a cache key following what an operation runs; reset
/// control by control (`ResetControl`) or by the header's reset (`ResetModule`), the
/// same.
fn d1_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    let (_, other) = methods(&bench.image);
    let mut edit = Edit::default();
    let refused = edit.set_control(&Demosaic, METHOD, method(other), &bench.image);
    assert!(
        matches!(refused, Err(EditError::NotListed { option, .. }) if option == other),
        "{refused:?}"
    );
    let from = other_photo(&bench.image);
    edit.set_control(&Demosaic, METHOD, method(other), &from)
        .unwrap();
    let instances = bench.instances(&edit, Frame);
    let demosaic = instances
        .iter()
        .find(|i| i.operation.manifest().id == DEMOSAIC)
        .unwrap();
    let written = apertrust_operations::demosaic::METHODS
        .iter()
        .position(|m| m.0 == other)
        .unwrap();
    assert_eq!(
        demosaic.params.get(METHOD),
        Some(Value::Choice(written as u32))
    );
    // `exact` holds the plan's operations and cache keys, as well as the pixels.
    let moved = bench.render(&instances);
    assert!(exact(&none, &moved), "D1, {other} on {:?}", bench.pipe);
    edit.reset_control(&Demosaic, METHOD).unwrap();
    assert_eq!(edit, Edit::default());
    assert!(exact(&none, &bench.edit(&edit, Frame)), "D1, reset control");
    let mut edit = Edit::default();
    edit.set_control(&Demosaic, METHOD, method(other), &from)
        .unwrap();
    bench.edit(&edit, Frame);
    edit.reset_module(&Demosaic);
    assert!(
        exact(&none, &bench.edit(&edit, Frame)),
        "D1, the module reset"
    );
}

#[test]
fn d1_the_method_moved_and_reset_renders_as_no_edit() {
    for pipe in PIPES {
        for mut bench in small_benches(pipe) {
            d1_holds(&mut bench);
        }
    }
}

/// D1 on the 90D and the X-T5, through the viewport's frame and the export's.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn d1_on_the_corpus() {
    for name in [NINETY_D, X_T5] {
        for pipe in [CORPUS_VIEWPORT, Pipe::Export] {
            d1_holds(&mut corpus_bench(name, pipe));
        }
    }
}

// D2: on a layer.

/// D2: demosaic acts on the mosaic, before any layer, so the module is the background's
/// only, as darktable's demosaic is one instance with no mask.
#[test]
fn d2_it_is_never_offered_on_a_layer() {
    let definition = Demosaic.definition();
    assert!(!definition.layers);
    assert!(!definition.auto);
}

// D3: darktable's match.

/// darktable's `reload_defaults` method for the photo, as darktable numbers it: Markesteijn
/// 1-pass for an X-Trans sensor, RCD for a Bayer one.
fn darktable_default(image: &ImageContext) -> u32 {
    match methods(image).0 {
        "markesteijn" => 1025,
        _ => 5,
    }
}

/// The method an edit renders on `image`, as darktable numbers it.
fn rendered_method(edit: &Edit, image: &ImageContext) -> u32 {
    let instances =
        apertrust_engine::edit::instances(edit, OPERATIONS, apertrust_modules::MODULES, image)
            .all()
            .unwrap();
    let demosaic = instances
        .iter()
        .find(|i| i.operation.manifest().id == DEMOSAIC)
        .unwrap();
    let params = demosaic
        .operation
        .schema()
        .resolve_over(&demosaic.operation.defaults(image), &demosaic.params)
        .unwrap();
    apertrust_operations::demosaic::METHODS[params.choice(METHOD) as usize].1
}

/// D3: the module's defaults are darktable's `reload_defaults` for the photo, RCD or
/// Markesteijn 1-pass, so an unedited photo renders darktable's method with every other
/// parameter darktable's default, which `apertrust-golden raw-run` holds to darktable's
/// own demosaic (`docs/ports/demosaic.md`, Results); the panel shows that method.
#[test]
fn d3_its_defaults_are_darktable_s_for_the_photo() {
    for (_, image) in [small_raw(-0.67), small_x_trans()] {
        let shown = Demosaic.defaults(&image).choice(METHOD) as usize;
        let darktable = darktable_default(&image);
        assert_eq!(METHODS[shown], methods(&image).0);
        assert_eq!(rendered_method(&Edit::default(), &image), darktable);
        for edit in untouched(&image) {
            assert_eq!(rendered_method(&edit, &image), darktable);
        }
    }
}

/// D3 on every corpus raw: Markesteijn 1-pass on each X-Trans sensor, RCD on each Bayer
/// one, as darktable's `reload_defaults` gives them.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn d3_on_the_corpus() {
    let (mut bayer, mut x_trans) = (0, 0);
    each_corpus_raw(|name, image, _| {
        let darktable = darktable_default(&image);
        assert_eq!(
            rendered_method(&Edit::default(), &image),
            darktable,
            "{name}"
        );
        if darktable == 1025 {
            x_trans += 1;
        } else {
            bayer += 1;
        }
    });
    println!("D3 holds on {bayer} Bayer raws and {x_trans} X-Trans raws");
}

// D4: what is not ported.

/// D4: every control and option not ported is refused as a command, and the edit stays
/// as it was: the methods but RCD and Markesteijn 1-pass, colour smoothing and match
/// greens but disabled, and the parameters of the methods and of capture sharpening not
/// ported, each at its range's ends.
#[test]
fn d4_what_is_not_ported_is_refused() {
    let (_, image) = small_x_trans();
    let mut edit = Edit::default();
    edit.set_control(&Demosaic, METHOD, method("markesteijn"), &image)
        .unwrap();
    let before = edit.clone();
    let mut tries: Vec<(&str, ControlValue)> = METHODS
        .iter()
        .filter(|&&m| m != "rcd" && m != "markesteijn")
        .map(|&m| (METHOD, method(m)))
        .collect();
    tries.extend((1..6).map(|i| ("color_smoothing", ControlValue::Choice(i))));
    tries.extend((1..4).map(|i| ("green_eq", ControlValue::Choice(i))));
    tries.extend([
        ("lmmse_refine", ControlValue::Choice(0)),
        ("cs_enabled", ControlValue::Bool(true)),
    ]);
    for (control, (low, high)) in [
        ("dual_thrs", (0.0, 1.0)),
        ("median_thrs", (0.3, 1.0)),
        ("cs_iter", (1.0, 25.0)),
        ("cs_radius", (0.5, 1.5)),
        ("cs_thrs", (0.0, 1.0)),
        ("cs_boost", (0.5, 1.5)),
        ("cs_center", (50.0, 100.0)),
    ] {
        tries.push((control, ControlValue::Number(low)));
        tries.push((control, ControlValue::Number(high)));
    }
    for (control, value) in tries {
        let refused = edit.set_control(&Demosaic, control, value.clone(), &image);
        let not_ported = matches!(
            refused,
            Err(EditError::Control {
                source: ControlError::Unported { .. } | ControlError::NotPorted { .. },
                ..
            })
        );
        assert!(not_ported, "{control} {value:?}: {refused:?}");
        assert_eq!(edit, before, "{control}");
    }
}
