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

//! Input colour profile's invariants (`docs/tools/input-colour-profile.md`,
//! Invariants), one test per row, each named after its row: I0, I1, I3, I5 and I6. The small
//! raws run on CI's software GPUs; the raw.pixls.us corpus runs locally, with
//! `APERTRUST_RAW_CORPUS` naming it:
//!
//! ```sh
//! cargo test --release -p apertrust-tools --test input_colour_profile -- --ignored --nocapture
//! ```
//!
//! I2, each option written as the operation's choice of its name, and I4, what each
//! photo lists, are unit tests in `crates/tools/src/input_colour_profile.rs`. colorin works before the working space,
//! so every row renders a raw from its mosaic: the small Bayer raw, and the same as a DNG
//! whose image context holds its own matrix. Every row that renders reads the frame,
//! colorout's output before it is rounded to 8 bits.

#[expect(
    dead_code,
    reason = "Input colour profile's rows read neither the ramps nor a jump"
)]
mod common;
#[expect(
    dead_code,
    reason = "Input colour profile's corpus rows read each raw's operations and parameters, so they need no export bench"
)]
mod corpus;

use apertrust_color::PROFILE_TYPES;
use apertrust_engine::edit::{self, Edit, EditError};
use apertrust_engine::pipeline::{Instance, Pipe};
use apertrust_operations::OPERATIONS;
use apertrust_plugin::{ImageContext, ParamValues, Value};
use apertrust_tools::input_colour_profile::{
    GAMUT_CLIPPING, INPUT_PROFILES, InputColourProfile, WORKING_PROFILES,
};
use apertrust_tools::{ControlError, ControlValue, ProcessingTool, TOOLS};

use common::{Bench, Rendered, exact, id, same_pixels, small_raw};
use corpus::{CORPUS_VIEWPORT, NINETY_D, corpus_bench, each_corpus_raw};

const COLORIN: &str = "colorin";
const TYPE: &str = "type";
const TYPE_WORK: &str = "type_work";
const NORMALIZE: &str = "normalize";

/// What every row reads: the frame, colorout's output.
#[derive(Debug, Clone, Copy)]
struct Frame;

impl common::Measure for Frame {
    fn keep(self, _instances: &mut Vec<Instance<'static>>, _synthetic: bool) {}
}

/// The pipes the rows check on the small raws: the export's and the viewport's.
const PIPES: [Pipe; 2] = [
    Pipe::Export,
    Pipe::Viewport {
        width: 40,
        height: 30,
    },
];

/// The small Bayer raw from its mosaic through `pipe`, its matrix the camera table's,
/// then the same raw as a DNG whose image context holds its own matrix.
fn small_benches(pipe: Pipe) -> [Bench; 2] {
    let (mosaic, image) = small_raw(-0.67);
    let mut dng = image.clone();
    dng.camera.as_mut().unwrap().embedded_matrix = true;
    [
        Bench::mosaic(&mosaic, image, pipe),
        Bench::mosaic(&mosaic, dng, pipe),
    ]
}

fn option(options: &[&str], name: &str) -> ControlValue {
    let index = options.iter().position(|&o| o == name).unwrap();
    ControlValue::Choice(index as u32)
}

/// The built-in tools without Input colour profile: the pipeline as it was before the
/// tool, which no tool wrote colorin's parameters in.
fn without_the_tool() -> Vec<&'static dyn ProcessingTool> {
    TOOLS
        .iter()
        .copied()
        .filter(|tool| tool.definition().tool.id != COLORIN)
        .collect()
}

/// The input profile, working profile and gamut clipping colorin renders with for these
/// instances, by their names.
fn rendered(instances: &[Instance], image: &ImageContext) -> [&'static str; 3] {
    let colorin = instances.iter().find(|i| id(i) == COLORIN).unwrap();
    let params = colorin
        .operation
        .schema()
        .resolve_over(&colorin.operation.defaults(image), &colorin.params)
        .unwrap();
    assert_eq!(params.choice("intent"), 0);
    assert!(!params.flag("blue_mapping"));
    [
        PROFILE_TYPES[params.choice(TYPE) as usize],
        PROFILE_TYPES[params.choice(TYPE_WORK) as usize],
        GAMUT_CLIPPING[params.choice(NORMALIZE) as usize],
    ]
}

// I0: the controls at their defaults.

/// Edits that leave `image` untouched: every control written at its default for the
/// photo, and every control kept there.
fn untouched(image: &ImageContext) -> [Edit; 2] {
    let defaults = InputColourProfile.defaults(image);
    let mut written = Edit::default();
    let mut kept = Edit::default();
    for control in [TYPE, TYPE_WORK, NORMALIZE] {
        let value = defaults.get(control).cloned().unwrap();
        written
            .set_control(&InputColourProfile, control, value, image)
            .unwrap();
        kept.keep(&InputColourProfile, control, true).unwrap();
    }
    [written, kept]
}

/// The operations and parameters `edit` gives through `tools`, which the plan, its cache
/// keys and the frame follow from.
fn planned(
    edit: &Edit,
    tools: &[&dyn ProcessingTool],
    image: &ImageContext,
) -> Vec<(&'static str, ParamValues)> {
    edit::instances(edit, OPERATIONS, tools, image)
        .all()
        .unwrap()
        .iter()
        .map(|instance| (id(instance), instance.params.clone()))
        .collect()
}

/// I0 on one pipeline: no edit, then each untouched edit, gives the operations and
/// parameters the pipeline without the tool gives, and renders its frame exactly.
fn i0_holds(bench: &mut Bench) {
    let before = planned(&Edit::default(), &without_the_tool(), &bench.image);
    let none = bench.edit(&Edit::default(), Frame);
    let mut edits = vec![Edit::default()];
    edits.extend(untouched(&bench.image));
    for edit in edits {
        assert_eq!(planned(&edit, TOOLS, &bench.image), before);
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "I0 on {:?}",
            bench.pipe
        );
    }
}

#[test]
fn i0_the_controls_at_their_defaults_render_as_before_the_tool() {
    for pipe in PIPES {
        for mut bench in small_benches(pipe) {
            i0_holds(&mut bench);
        }
    }
}

/// I0 on every corpus raw: the tool at its defaults, untouched, written or kept, gives
/// the very operations and parameters the pipeline without the tool does, so the same
/// plan, cache keys and frame.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn i0_on_the_corpus() {
    let mut raws = 0;
    each_corpus_raw(|name, image, _| {
        let before = planned(&Edit::default(), &without_the_tool(), &image);
        let mut edits = vec![Edit::default()];
        edits.extend(untouched(&image));
        for edit in edits {
            assert_eq!(planned(&edit, TOOLS, &image), before, "I0 on {name}");
        }
        raws += 1;
    });
    println!("I0 holds on {raws} raws");
}

// I1: each input profile an edit can choose, set, then reset.

/// No edit's instances with colorin's `param` set to `name` at operation level: what
/// colorin renders with that profile, with no tool in between.
fn direct(bench: &Bench, param: &str, name: &str) -> Vec<Instance<'static>> {
    let index = PROFILE_TYPES.iter().position(|&p| p == name).unwrap();
    let mut instances = bench.instances(&Edit::default(), Frame);
    let colorin = instances.iter_mut().find(|i| id(i) == COLORIN).unwrap();
    colorin.params.set(param, Value::Choice(index as u32));
    instances
}

/// I1 on one pipeline. Each input profile this photo lists that an edit can choose, set
/// by the tool: colorin renders it, the frame as colorin renders it set directly, no
/// edit's for the photo's own matrix, its default, and another for each linear profile.
/// Then reset by `ResetControl` and by `ResetTool`: no edit's frame and keys, exactly.
fn i1_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    let control = InputColourProfile.definition().control(TYPE).unwrap();
    let default = INPUT_PROFILES[InputColourProfile.defaults(&bench.image).choice(TYPE) as usize];
    let choosable: Vec<&str> = InputColourProfile
        .options(TYPE, &bench.image)
        .into_iter()
        .filter(|&p| control.check(&option(&INPUT_PROFILES, p)).is_ok())
        .collect();
    assert_eq!(choosable, [default, "lin_rec709", "lin_rec2020"]);
    for name in choosable {
        let mut edit = Edit::default();
        let value = option(&INPUT_PROFILES, name);
        edit.set_control(&InputColourProfile, TYPE, value.clone(), &bench.image)
            .unwrap();
        let instances = bench.instances(&edit, Frame);
        assert_eq!(
            rendered(&instances, &bench.image),
            [name, "lin_rec2020", "off"]
        );
        let moved: Rendered = bench.render(&instances);
        let as_colorin = bench.render(&direct(bench, TYPE, name));
        assert!(
            exact(&as_colorin, &moved),
            "I1, {name} as colorin renders it"
        );
        if name == default {
            assert!(exact(&none, &moved), "I1, {name}");
        } else {
            assert!(!same_pixels(&none, &moved), "I1, {name} changes the frame");
        }
        edit.reset_control(&InputColourProfile, TYPE).unwrap();
        assert_eq!(edit, Edit::default());
        assert!(exact(&none, &bench.edit(&edit, Frame)), "I1, {name} reset");
        edit.set_control(&InputColourProfile, TYPE, value, &bench.image)
            .unwrap();
        bench.edit(&edit, Frame);
        edit.reset_tool(&InputColourProfile);
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "I1, {name} and the tool reset"
        );
    }
}

#[test]
fn i1_each_input_profile_renders_as_colorin_and_resets_to_no_edit() {
    for pipe in PIPES {
        for mut bench in small_benches(pipe) {
            i1_holds(&mut bench);
        }
    }
}

/// I1 on the 90D, through the viewport's frame and the export's.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn i1_on_the_corpus() {
    for pipe in [CORPUS_VIEWPORT, Pipe::Export] {
        i1_holds(&mut corpus_bench(NINETY_D, pipe));
    }
}

// I3: what an edit cannot choose.

/// I3: every option not ported is refused as a command, and the edit stays as it was:
/// the input profiles but the camera's matrix and the two linear ones, the working
/// profiles but linear Rec2020, gamut clipping but off. The camera matrix this photo does
/// not list is refused as not listed. colorin's parameters are the tool's: the three
/// choices owned, the intent and blue mapping fixed.
#[test]
fn i3_every_option_not_ported_is_refused() {
    let (_, raw) = small_raw(-0.67);
    let mut dng = raw.clone();
    dng.camera.as_mut().unwrap().embedded_matrix = true;
    let mut edit = Edit::default();
    edit.keep(&InputColourProfile, TYPE, true).unwrap();
    let before = edit.clone();
    for image in [&raw, &dng] {
        for (control, options, ported) in [
            (
                TYPE,
                &INPUT_PROFILES[..],
                &[
                    "embedded_matrix",
                    "standard_matrix",
                    "lin_rec709",
                    "lin_rec2020",
                ][..],
            ),
            (TYPE_WORK, &WORKING_PROFILES[..], &["lin_rec2020"][..]),
            (NORMALIZE, &GAMUT_CLIPPING[..], &["off"][..]),
        ] {
            for &name in options.iter().filter(|o| !ported.contains(o)) {
                let refused =
                    edit.set_control(&InputColourProfile, control, option(options, name), image);
                assert!(
                    matches!(
                        refused,
                        Err(EditError::Control {
                            source: ControlError::Unported { .. },
                            ..
                        })
                    ),
                    "{control} {name}: {refused:?}"
                );
                assert_eq!(edit, before, "{control} {name}");
            }
        }
    }
    for (image, other) in [(&raw, "embedded_matrix"), (&dng, "standard_matrix")] {
        let refused = edit.set_control(
            &InputColourProfile,
            TYPE,
            option(&INPUT_PROFILES, other),
            image,
        );
        assert!(
            matches!(refused, Err(EditError::NotListed { .. })),
            "{other}: {refused:?}"
        );
        assert_eq!(edit, before, "{other}");
    }
    let set = |param, value| {
        edit.clone()
            .set_param(OPERATIONS, TOOLS, COLORIN, param, value)
    };
    for param in [TYPE, TYPE_WORK, NORMALIZE] {
        assert!(
            matches!(
                set(param, Value::Choice(0)),
                Err(EditError::Owned { tool: COLORIN, .. })
            ),
            "{param}"
        );
    }
    assert!(matches!(
        set("intent", Value::Choice(1)),
        Err(EditError::Fixed { tool: COLORIN, .. })
    ));
    assert!(matches!(
        set("blue_mapping", Value::Bool(true)),
        Err(EditError::Fixed { tool: COLORIN, .. })
    ));
}

// I6: on a layer.

/// I6: colorin takes the whole image from the camera's colours to the working space,
/// before every layer, so the tool is the background's only, as darktable's input color
/// profile is one instance with no mask.
#[test]
fn i6_it_is_never_offered_on_a_layer() {
    let definition = InputColourProfile.definition();
    assert!(!definition.layers);
    assert!(!definition.auto);
}

// I5: darktable's match.

/// I5: an unedited photo, and each untouched edit, renders colorin at darktable's
/// defaults for it, `reload_defaults`: the standard matrix on a raw, a DNG's embedded
/// matrix where its image context holds one, linear Rec2020 to work in and no gamut
/// clipping, which `apertrust-golden pipeline` holds to darktable's export; the panel
/// shows the same.
#[test]
fn i5_every_photo_renders_darktable_s_defaults() {
    for (bench, matrix) in small_benches(Pipe::Export)
        .into_iter()
        .zip(["standard_matrix", "embedded_matrix"])
    {
        let shown = InputColourProfile.defaults(&bench.image).choice(TYPE);
        assert_eq!(INPUT_PROFILES[shown as usize], matrix);
        let mut edits = vec![Edit::default()];
        edits.extend(untouched(&bench.image));
        for edit in edits {
            let instances = bench.instances(&edit, Frame);
            assert_eq!(
                rendered(&instances, &bench.image),
                [matrix, "lin_rec2020", "off"]
            );
        }
    }
}

/// I5 on every corpus raw: colorin at the tool's defaults for it.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn i5_on_the_corpus() {
    let mut raws = 0;
    each_corpus_raw(|name, image, _| {
        let instances = edit::instances(&Edit::default(), OPERATIONS, TOOLS, &image)
            .all()
            .unwrap();
        let shown = InputColourProfile.defaults(&image).choice(TYPE);
        assert_eq!(
            rendered(&instances, &image)[0],
            INPUT_PROFILES[shown as usize],
            "{name}"
        );
        raws += 1;
    });
    println!("I5 holds on {raws} raws");
}
