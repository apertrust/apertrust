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

//! Output colour profile's invariants (`docs/modules/output-colour-profile.md`,
//! Invariants), one test per row, each named after its row: O0, O1, O3 to O5. The small
//! raw and the synthetic sweep run on CI's software GPUs; the raw.pixls.us corpus runs
//! locally, with `APERTRUST_RAW_CORPUS` naming it:
//!
//! ```sh
//! cargo test --release -p apertrust-modules --test output_colour_profile -- --ignored --nocapture
//! ```
//!
//! O2, each profile written as the operation's choice of its name, is a unit test in
//! `crates/modules/src/output_colour_profile.rs`. Synthetic images enter after demosaic,
//! already in the working space, and plan the operations from exposure on. Every row
//! that renders reads the frame, colorout's output before it is rounded to 8 bits.

#[expect(
    dead_code,
    reason = "Output colour profile's rows read neither the ramps nor a jump"
)]
mod common;
mod corpus;

use apertrust_color::PROFILE_TYPES;
use apertrust_engine::edit::{self, Edit, EditError};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe};
use apertrust_modules::output_colour_profile::{HELD_BACK, OutputColourProfile, PROFILES};
use apertrust_modules::{Base, ControlError, ControlValue, MODULES, Module, ModuleValues};
use apertrust_operations::OPERATIONS;
use apertrust_plugin::{ImageContext, Value};

use common::{BEFORE_THE_WORKING_SPACE, Bench, Rendered, exact, id, small_raw, sweep};
use corpus::{CORPUS_VIEWPORT, NINETY_D, corpus_bench, each_corpus_raw, export_bench};

const COLOROUT: &str = "colorout";
const TYPE: &str = "type";

/// What every row reads: the frame, colorout's output, from exposure on for a synthetic
/// image.
#[derive(Debug, Clone, Copy)]
struct Frame;

impl common::Measure for Frame {
    fn keep(self, instances: &mut Vec<Instance<'static>>, synthetic: bool) {
        instances.retain(|i| !(synthetic && BEFORE_THE_WORKING_SPACE.contains(&id(i))));
    }
}

/// The pipes the rows check on the small images: the export's and the viewport's.
const PIPES: [Pipe; 2] = [
    Pipe::Export,
    Pipe::Viewport {
        width: 40,
        height: 30,
    },
];

/// The small images on a bench through `pipe`: the small Bayer raw from its mosaic, and
/// the synthetic sweep in the working space, whose saturated colours lie outside sRGB.
fn small_benches(pipe: Pipe) -> [Bench; 2] {
    let (mosaic, image) = small_raw(-0.67);
    let (picture, swept) = sweep(-0.67);
    [
        Bench::mosaic(&mosaic, image, pipe),
        Bench::picture(&picture, swept, pipe),
    ]
}

fn profile(name: &str) -> ControlValue {
    let index = PROFILES.iter().position(|&p| p == name).unwrap();
    ControlValue::Choice(index as u32)
}

/// The profile colorout renders in for these instances, by its name.
fn rendered_profile(instances: &[Instance], image: &ImageContext) -> (&'static str, u32) {
    let colorout = instances.iter().find(|i| id(i) == COLOROUT).unwrap();
    let params = colorout
        .operation
        .schema()
        .resolve_over(&colorout.operation.defaults(image), &colorout.params)
        .unwrap();
    (
        PROFILE_TYPES[params.choice(TYPE) as usize],
        params.choice("intent"),
    )
}

// O0: the control at its default.

/// Edits that leave `image` untouched: Export profile written at sRGB, and kept there.
fn untouched(image: &ImageContext) -> [Edit; 2] {
    let mut written = Edit::default();
    written
        .set_control(&OutputColourProfile, TYPE, profile("srgb"), image)
        .unwrap();
    let mut kept = Edit::default();
    kept.keep(&OutputColourProfile, TYPE, true).unwrap();
    [written, kept]
}

/// O0 on one pipeline: no edit, then each untouched edit, every frame as the first, and
/// the binding empty for each.
fn o0_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    let base = Base::new(OPERATIONS, &bench.image);
    for edit in untouched(&bench.image) {
        let values = edit.module(COLOROUT).unwrap();
        let values = values.off_defaults(&OutputColourProfile.defaults(&bench.image));
        assert!(
            OutputColourProfile
                .bind(&values, &bench.image, &base)
                .is_empty()
        );
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "O0 on {:?}",
            bench.pipe
        );
    }
}

#[test]
fn o0_the_control_at_its_default_renders_as_no_edit() {
    for pipe in PIPES {
        for mut bench in small_benches(pipe) {
            o0_holds(&mut bench);
        }
    }
}

/// O0 on every corpus raw: the module at its default, written or kept, plans the very
/// instances and cache keys no edit does, so the same frame.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn o0_on_the_corpus() {
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
            assert_eq!(keys(&bench.instances(&edit, Frame)), none, "O0 on {name}");
        }
        raws += 1;
    });
    println!("O0 holds on {raws} raws");
}

// O1: the control moved, then reset.

/// No edit's instances with colorout's profile set by the module's binding for `name`,
/// the binding's values set directly: what the edit would render once the profile is no
/// longer held back.
fn bound(bench: &Bench, name: &str) -> Vec<Instance<'static>> {
    let mut values = ModuleValues::new();
    values.set(TYPE, profile(name));
    let values = values.off_defaults(&OutputColourProfile.defaults(&bench.image));
    let base = Base::new(OPERATIONS, &bench.image);
    let mut instances = bench.instances(&Edit::default(), Frame);
    for setting in OutputColourProfile.bind(&values, &bench.image, &base) {
        let instance = instances
            .iter_mut()
            .find(|i| id(i) == setting.operation)
            .unwrap();
        instance.params.set(setting.param, setting.value);
    }
    instances
}

/// O1 on one pipeline, as the session renders. Each profile an edit can choose, sRGB
/// today, set, then reset by `ResetControl` and by `ResetModule`: as no edit once reset,
/// and while set too for sRGB, the default; another would render in its own profile.
/// Then each profile held back for the display path (decision 26), bound directly: the
/// frame in that profile, not sRGB's, and no edit's again, exactly, once it is gone.
fn o1_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    let control = OutputColourProfile.definition().control(TYPE).unwrap();
    let choosable: Vec<&str> = PROFILES
        .into_iter()
        .filter(|&p| control.check(&profile(p)).is_ok())
        .collect();
    assert_eq!(choosable, ["srgb"]);
    for name in choosable {
        let mut edit = Edit::default();
        edit.set_control(&OutputColourProfile, TYPE, profile(name), &bench.image)
            .unwrap();
        let instances = bench.instances(&edit, Frame);
        assert_eq!(rendered_profile(&instances, &bench.image).0, name);
        let moved = bench.render(&instances);
        assert_eq!(exact(&none, &moved), name == "srgb", "O1, {name}");
        edit.reset_control(&OutputColourProfile, TYPE).unwrap();
        assert_eq!(edit, Edit::default());
        assert!(exact(&none, &bench.edit(&edit, Frame)), "O1, {name} reset");
        edit.set_control(&OutputColourProfile, TYPE, profile(name), &bench.image)
            .unwrap();
        bench.edit(&edit, Frame);
        edit.reset_module(&OutputColourProfile);
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "O1, {name} and the module reset"
        );
    }
    for name in HELD_BACK {
        let instances = bound(bench, name);
        assert_eq!(rendered_profile(&instances, &bench.image).0, name);
        let moved: Rendered = bench.render(&instances);
        assert!(!exact(&none, &moved), "O1, {name} on {:?}", bench.pipe);
        assert!(
            exact(&none, &bench.edit(&Edit::default(), Frame)),
            "O1, {name} gone on {:?}",
            bench.pipe
        );
    }
}

#[test]
fn o1_the_control_moved_and_reset_renders_as_no_edit() {
    for pipe in PIPES {
        for mut bench in small_benches(pipe) {
            o1_holds(&mut bench);
        }
    }
}

/// O1 on the 90D, through the viewport's frame and the export's.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn o1_on_the_corpus() {
    for pipe in [CORPUS_VIEWPORT, Pipe::Export] {
        o1_holds(&mut corpus_bench(NINETY_D, pipe));
    }
}

// O3: what an edit cannot choose.

/// O3: every profile but sRGB is refused as a command, and the edit stays as it was: the
/// two linear ones held back for the display path (decision 26), the nine others not
/// ported, on the small raw and the sweep alike. colorout's parameters are the module's:
/// its profile owned, its intent fixed.
#[test]
fn o3_every_profile_but_srgb_is_refused() {
    let mut edit = Edit::default();
    edit.keep(&OutputColourProfile, TYPE, true).unwrap();
    let before = edit.clone();
    for image in [small_raw(-0.67).1, sweep(-0.67).1] {
        for name in PROFILES.into_iter().filter(|&p| p != "srgb") {
            let refused = edit.set_control(&OutputColourProfile, TYPE, profile(name), &image);
            assert!(
                matches!(
                    refused,
                    Err(EditError::Control {
                        source: ControlError::Unported { .. },
                        ..
                    })
                ),
                "{name}: {refused:?}"
            );
            assert_eq!(edit, before, "{name}");
        }
    }
    let lin_rec2020 = PROFILE_TYPES.iter().position(|&p| p == "lin_rec2020");
    let set = |param, value| {
        edit.clone()
            .set_param(OPERATIONS, MODULES, COLOROUT, param, value)
    };
    assert!(matches!(
        set(TYPE, Value::Choice(lin_rec2020.unwrap() as u32)),
        Err(EditError::Owned {
            module: COLOROUT,
            ..
        })
    ));
    assert!(matches!(
        set("intent", Value::Choice(1)),
        Err(EditError::Fixed {
            module: COLOROUT,
            ..
        })
    ));
}

// O4: on a layer.

/// O4: colorout encodes the whole image for its output, after every layer, so the module
/// is the background's only, as darktable's output color profile is one instance with no
/// mask.
#[test]
fn o4_it_is_never_offered_on_a_layer() {
    let definition = OutputColourProfile.definition();
    assert!(!definition.layers);
    assert!(!definition.auto);
}

// O5: darktable's match.

/// O5: an unedited photo, and each untouched edit, renders colorout at darktable's
/// `$DEFAULT`s, sRGB and perceptual, which `apertrust-golden pipeline` holds to
/// darktable's sRGB export; the panel shows sRGB (web-safe).
#[test]
fn o5_every_photo_renders_darktable_s_srgb() {
    for bench in small_benches(Pipe::Export) {
        let shown = OutputColourProfile.defaults(&bench.image).choice(TYPE);
        assert_eq!(PROFILES[shown as usize], "srgb");
        let mut edits = vec![Edit::default()];
        edits.extend(untouched(&bench.image));
        for edit in edits {
            let instances = bench.instances(&edit, Frame);
            assert_eq!(rendered_profile(&instances, &bench.image), ("srgb", 0));
        }
    }
}

/// O5 on every corpus raw.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn o5_on_the_corpus() {
    let mut raws = 0;
    each_corpus_raw(|name, image, _| {
        let instances = edit::instances(&Edit::default(), OPERATIONS, MODULES, &image)
            .all()
            .unwrap();
        assert_eq!(rendered_profile(&instances, &image), ("srgb", 0), "{name}");
        raws += 1;
    });
    println!("O5 holds on {raws} raws");
}
