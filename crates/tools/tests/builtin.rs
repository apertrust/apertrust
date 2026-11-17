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

//! The built-in tools against the built-in operations and the interface's messages
//! (decision 28): each definition holds together, no operation parameter has two owners,
//! every parameter a tool owns exists, its binding sets nothing at its defaults and
//! only what it owns, within the schema, across every control's range, its preset rows'
//! options hold their controls' ranges and read back, its choices list options for each
//! photo that hold together (decision 30), its lossless views are given and their
//! coordinates' ranges keep their controls in theirs, its pickers are darktable's, and
//! every message its definition implies is in `crates/ui/locales`.

use std::collections::BTreeSet;

use apertrust_operations::OPERATIONS;
use apertrust_plugin::{Camera, ImageContext, Pattern};
use apertrust_tools::utility::UTILITIES;
use apertrust_tools::{
    Base, Control, ControlKind, ControlValue, Controls, Owned, ProcessingDefinition,
    ProcessingTool, Quad, Reset, Row, Setting, Slider, TOOLS, Tab, ToolDefinition, ToolValues,
    Unit, check, messages,
};

/// Image contexts the bindings are swept over: an image that is not a raw, a Bayer raw,
/// an X-Trans raw with an exposure bias, a Bayer4 raw and a monochrome one.
fn images() -> Vec<ImageContext> {
    let camera = |cfa: Pattern<u32>, exposure_bias| Camera {
        make: String::new(),
        model: String::new(),
        alias: String::new(),
        width: 6048,
        height: 4024,
        cfa: Some(cfa),
        black: Pattern {
            width: 1,
            height: 1,
            values: vec![512.0],
        },
        white: 16383.0,
        white_balance: Some(vec![2.1, 1.0, 1.6]),
        xyz_to_camera: Some([[0.7, -0.2, -0.05], [-0.4, 1.2, 0.2], [-0.05, 0.15, 0.6]]),
        crop: [0, 0, 6048, 4024],
        declared: [12, 8, 6000, 4000],
        exposure_bias,
        iso: 0.0,
        linear_response_limit: 1.0,
        lens_correction: None,
        highlight_preservation: 0.0,
        orientation: 0,
    };
    let bayer = Pattern {
        width: 2,
        height: 2,
        values: vec![0, 1, 1, 2],
    };
    let x_trans = Pattern {
        width: 6,
        height: 6,
        values: vec![
            1, 1, 0, 1, 1, 2, 1, 1, 2, 1, 1, 0, 2, 0, 1, 0, 2, 1, 1, 1, 2, 1, 1, 0, 1, 1, 0, 1, 1,
            2, 0, 2, 1, 2, 0, 1,
        ],
    };
    let four_colours = Pattern {
        values: vec![0, 1, 3, 2],
        ..bayer.clone()
    };
    let raw = |camera| ImageContext {
        camera: Some(camera),
        ..ImageContext::full(6000, 4000)
    };
    vec![
        ImageContext::full(6000, 4000),
        raw(camera(bayer.clone(), 0.0)),
        raw(camera(x_trans, -0.7)),
        // darktable's Bayer4, four colours in a 2×2 pattern, and a raw with no colour
        // filters, which Demosaic takes as darktable's monochrome sraw though a linear raw
        // has none either: kinds of photo Demosaic lists methods of their own for
        // (decision 30).
        raw(camera(four_colours, 0.0)),
        raw(Camera {
            cfa: None,
            ..camera(bayer, 0.0)
        }),
    ]
}

#[test]
fn every_built_in_tool_is_accepted() {
    let (accepted, rejected) = check::accept(TOOLS, OPERATIONS);
    assert!(rejected.is_empty(), "{rejected:?}");
    assert_eq!(accepted.len(), TOOLS.len());
}

/// darktable's pickers on the built tools' sliders, each named by its quad (decision
/// 79; `docs/pickers.md`, The first pass's pickers). Colour calibration's and AgX's Auto
/// tune levels come with their buttons' rows; Exposure's reference picker is on its
/// reference row (decision 88), and White balance's turned on by Settings' from image
/// area until its button bar comes.
#[test]
fn the_built_in_tools_declare_darktable_s_pickers_on_their_sliders() {
    let pickers: Vec<(&str, Vec<&str>)> = TOOLS
        .iter()
        .map(|tool| tool.definition())
        .filter(|definition| !definition.pickers.is_empty())
        .map(|definition| {
            (
                definition.tool.id,
                definition.pickers.iter().map(|p| p.id).collect(),
            )
        })
        .collect();
    assert_eq!(
        pickers,
        [
            ("agx", vec!["white", "black", "pivot", "pivot_output"]),
            ("temperature", vec!["spot"]),
            ("exposure", vec!["exposure", "reference"]),
            (
                "colorbalancergb",
                vec![
                    "global_hue",
                    "shadows_hue",
                    "midtones_hue",
                    "highlights_hue",
                    "white_fulcrum",
                    "grey_fulcrum",
                ]
            ),
        ]
    );
    // Every slider's quad picks: a line only where darktable draws one, as rotate and
    // perspective's horizon, which no built tool has; which picker each names, and that
    // each picker is in one row, are the tools' tables and `ProcessingDefinition::check`.
    let not_pickers: Vec<(&str, &str, Quad)> = TOOLS
        .iter()
        .flat_map(|tool| {
            let id = tool.definition().tool.id;
            tool.definition().rows().filter_map(move |row| match row {
                Row::Slider(Slider {
                    control,
                    quad: Some(quad),
                    ..
                }) if !matches!(quad, Quad::Picker(_)) => Some((id, *control, *quad)),
                _ => None,
            })
        })
        .collect();
    assert_eq!(not_pickers, []);
}

/// What a tool fixes, with no control, is among what it owns, so the refusal that names
/// it names its owner.
#[test]
fn every_fixed_parameter_is_one_its_tool_owns() {
    for tool in TOOLS {
        for fixed in tool.fixed() {
            assert!(
                tool.owns().contains(fixed),
                "{}: {}.{}",
                tool.definition().tool.id,
                fixed.operation,
                fixed.param
            );
        }
    }
}

/// The switched tools (decision 32): those darktable leaves off on a new photo, each
/// over operations an unedited photo does not run. The Tone curve, over the optional
/// `rgbcurve`, is not among them: it is to be reshaped into RGB curve, darktable's module,
/// which is switched as it is built (`docs/tools/README.md`, Planned changes).
const SWITCHED: [&str; 7] = [
    "toneequal",
    "colorbalancergb",
    "hazeremoval",
    "primaries",
    "grain",
    "soften",
    "vignette",
];

/// Exactly the tools darktable leaves off are switched, and each one's defaults, made
/// explicit, bind its operations' own defaults for every photo (`check::switched_defaults`),
/// so the tool on and untouched is darktable's new instance.
#[test]
fn every_switched_tool_s_defaults_bind_its_operations_defaults() {
    let switched: Vec<&str> = TOOLS
        .iter()
        .map(|tool| tool.definition())
        .filter(|definition| definition.switched)
        .map(|definition| definition.tool.id)
        .collect();
    assert_eq!(
        switched.iter().copied().collect::<BTreeSet<_>>(),
        SWITCHED.into_iter().collect::<BTreeSet<_>>()
    );
    for image in images() {
        for tool in TOOLS {
            let problems = check::switched_defaults(*tool, OPERATIONS, &image);
            assert!(
                problems.is_empty(),
                "{}: {problems:?}",
                tool.definition().tool.id
            );
        }
    }
}

/// A switched tool over an operation every photo runs is refused, since off it could not
/// leave that operation untouched.
#[test]
fn a_switched_tool_over_an_operation_every_photo_runs_is_refused() {
    struct Switched(&'static ProcessingDefinition);
    impl ProcessingTool for Switched {
        fn definition(&self) -> &'static ProcessingDefinition {
            self.0
        }
        fn owns(&self) -> &'static [Owned] {
            apertrust_tools::exposure::Exposure.owns()
        }
        fn bind(&self, _: &ToolValues, _: &ImageContext, _: &Base) -> Vec<Setting> {
            Vec::new()
        }
    }
    let over_exposure = Box::leak(Box::new(ProcessingDefinition {
        switched: true,
        ..*apertrust_tools::exposure::Exposure.definition()
    }));
    let (_, rejected) = check::accept(&[&Switched(over_exposure)], OPERATIONS);
    assert!(
        matches!(
            rejected[..],
            [check::Rejected {
                error: check::ToolError::SwitchedOverBase { .. },
                ..
            }]
        ),
        "{rejected:?}"
    );
}

#[test]
fn every_built_in_binding_sets_only_what_it_owns_within_the_schema() {
    for image in images() {
        for tool in TOOLS {
            let problems = check::sweep(*tool, OPERATIONS, &image);
            assert!(problems.is_empty(), "{problems:?}");
        }
    }
}

/// The preset rows whose label reads the values but for one control, which records where
/// they came from: White balance's Settings, labelled by its coefficients alone as
/// darktable's `gui_update` labels it, so on an image whose as-shot and D65 coefficients
/// coincide, one that is not a raw, camera reference reads as shot, darktable's label.
const LABELLED_WITHOUT: &[(&str, &str)] = &[("temperature.settings", "preset")];

/// What is wrong with the tool's preset rows on `image` (`docs/engine.md`, Preset rows
/// and lossless views, Tests): a row with no option a colour raw can have, which a tool
/// that declares the row and leaves `ProcessingTool::preset` as provided has; an option
/// missing a value for a control its row writes, or giving one the control cannot take; and
/// an option `ProcessingTool::matching` does not give back, or one with the same values,
/// but for the control a row in [`LABELLED_WITHOUT`] names.
fn preset_problems(tool: &dyn ProcessingTool, image: &ImageContext) -> Vec<String> {
    let definition = tool.definition();
    let colour_raw = image.camera.as_ref().is_some_and(|c| c.cfa.is_some());
    let mut problems = Vec::new();
    for row in definition.preset_rows {
        let name = format!("{}.{}", definition.tool.id, row.id);
        let aside = LABELLED_WITHOUT
            .iter()
            .find(|(id, _)| *id == name)
            .map(|(_, control)| *control);
        let given: Vec<(&str, Controls)> = row
            .options
            .iter()
            .filter_map(|&option| Some((option, tool.preset(row.id, option, image)?)))
            .collect();
        if colour_raw && given.is_empty() {
            problems.push(format!("{name}: no option on a colour raw"));
        }
        for (option, values) in &given {
            let same = |other: &Controls| {
                row.controls
                    .iter()
                    .filter(|c| Some(**c) != aside)
                    .all(|c| other.get(c) == values.get(c))
            };
            let mut holding = tool.defaults(image);
            for &control in row.controls {
                let Some(value) = values.get(control) else {
                    problems.push(format!("{name}: {option} gives no {control}"));
                    continue;
                };
                if let Some(Err(error)) = definition.control(control).map(|c| c.check(value)) {
                    problems.push(format!("{name}: {option}: {error}"));
                }
                holding.set(control, value.clone());
            }
            let matching = tool.matching(row.id, &holding, image);
            let back = given
                .iter()
                .any(|(other, its)| Some(*other) == matching && same(its));
            if !back {
                problems.push(format!("{name}: {option} reads as {matching:?}"));
            }
        }
    }
    problems
}

#[test]
fn every_built_in_preset_row_has_options_a_photo_can_have_that_read_back() {
    for image in images() {
        for tool in TOOLS {
            let problems = preset_problems(*tool, &image);
            assert!(problems.is_empty(), "{problems:?}");
        }
    }
}

/// The one segmented choice listed per photo, Exposure's Mode, an exception to decision
/// 30's "a list, never segments" (decision 88): its switch draws every option, those the
/// photo does not list disabled with their reasons.
const SEGMENTS_PER_PHOTO: [(&str, &str); 1] = [("exposure", "mode")];

/// What is wrong with the options the tool's choices list on `images` (decision 30):
/// on any of them a list that is empty, not the definition's options each once in its
/// order, or without the photo's default ([`check::listed`]); an option no image lists; a
/// choice drawn as segments that lists other options on another image; and a preset
/// option an image can have that writes an option the image does not list.
fn list_problems(tool: &dyn ProcessingTool, images: &[ImageContext]) -> Vec<String> {
    let definition = tool.definition();
    let id = definition.tool.id;
    let segmented: BTreeSet<&str> = definition
        .rows()
        .filter_map(|row| match row {
            Row::Choice {
                control,
                style: apertrust_tools::ChoiceStyle::Segmented,
            } => Some(*control),
            _ => None,
        })
        .collect();
    let mut problems = Vec::new();
    for control in definition.controls {
        let ControlKind::Choice { options, .. } = control.kind else {
            continue;
        };
        let lists: Vec<Vec<&str>> = images
            .iter()
            .map(|image| tool.options(control.id, image))
            .collect();
        for (image, listed) in images.iter().zip(&lists) {
            let default = tool.defaults(image).get(control.id).cloned();
            let default = default.unwrap_or_else(|| control.default());
            if let Some(problem) = check::listed(control, listed, &default) {
                problems.push(format!("{id}: {problem}"));
            }
        }
        for option in options {
            if !lists.iter().any(|listed| listed.contains(option)) {
                problems.push(format!("{id}.{}: no photo lists {option}", control.id));
            }
        }
        if segmented.contains(control.id)
            && !SEGMENTS_PER_PHOTO.contains(&(id, control.id))
            && lists.iter().any(|listed| *listed != lists[0])
        {
            problems.push(format!("{id}.{}: segments, listed per photo", control.id));
        }
    }
    for (image, row) in images
        .iter()
        .flat_map(|image| definition.preset_rows.iter().map(move |row| (image, row)))
    {
        for &option in row.options {
            let Some(values) = tool.preset(row.id, option, image) else {
                continue;
            };
            for &control in row.controls {
                let (Some(found), Some(ControlValue::Choice(i))) =
                    (definition.control(control), values.get(control))
                else {
                    continue;
                };
                let ControlKind::Choice { options, .. } = found.kind else {
                    continue;
                };
                let written = options.get(*i as usize).copied().unwrap_or_default();
                if !tool.options(control, image).contains(&written) {
                    problems.push(format!(
                        "{id}.{}: {option} writes {control} {written}, which a photo does not list",
                        row.id
                    ));
                }
            }
        }
    }
    problems
}

#[test]
fn every_built_in_choice_lists_options_that_hold_on_every_photo() {
    for tool in TOOLS {
        let problems = list_problems(*tool, &images());
        assert!(problems.is_empty(), "{problems:?}");
    }
}

/// A tool whose lists break the rules, as a bug may: `tone`, a list, lists warm before
/// cool on a raw, and never its third option, sepia; `grid`, segments, lists fewer
/// options on a raw; and its preset row's `warm` option, which every photo can have,
/// writes the grid's three, which a raw does not list.
struct Listing;

static LISTING: ProcessingDefinition = ProcessingDefinition {
    tool: ToolDefinition {
        id: "listing",
        tabs: &[Tab {
            id: None,
            rows: &[
                Row::Preset {
                    preset: "style",
                    style: apertrust_tools::ChoiceStyle::List,
                },
                Row::Choice {
                    control: "tone",
                    style: apertrust_tools::ChoiceStyle::List,
                },
                Row::Choice {
                    control: "grid",
                    style: apertrust_tools::ChoiceStyle::Segmented,
                },
            ],
        }],
        before: &[],
        after: &[],
        reset: Reset::Tool,
        presets: true,
        menus: &[],
    },
    version: 1,
    layers: false,
    auto: false,
    switched: false,
    controls: &[
        Control {
            id: "tone",
            kind: ControlKind::Choice {
                options: &["cool", "warm", "sepia"],
                default: 0,
                unported: &[],
            },
            unit: None,
            unported: false,
        },
        Control {
            id: "grid",
            kind: ControlKind::Choice {
                options: &["two", "three"],
                default: 0,
                unported: &[],
            },
            unit: None,
            unported: false,
        },
    ],
    preset_rows: &[apertrust_tools::PresetRow {
        id: "style",
        options: &["warm"],
        controls: &["tone", "grid"],
        pickers: &[],
    }],
    views: &[],
    pickers: &[],
    facts: None,
};

impl ProcessingTool for Listing {
    fn definition(&self) -> &'static ProcessingDefinition {
        &LISTING
    }

    fn owns(&self) -> &'static [Owned] {
        &[]
    }

    fn bind(&self, _: &ToolValues, _: &ImageContext, _: &Base) -> Vec<Setting> {
        Vec::new()
    }

    fn options(&self, control: &str, image: &ImageContext) -> Vec<&'static str> {
        match (control, image.camera.is_some()) {
            ("tone", true) => vec!["warm", "cool"],
            ("tone", false) => vec!["cool", "warm"],
            ("grid", true) => vec!["two"],
            ("grid", false) => vec!["two", "three"],
            _ => Vec::new(),
        }
    }

    fn preset(&self, _: &str, _: &str, _: &ImageContext) -> Option<Controls> {
        let mut values = Controls::new();
        values.set("tone", ControlValue::Choice(1));
        values.set("grid", ControlValue::Choice(1));
        Some(values)
    }
}

#[test]
fn a_choice_s_lists_are_held_to_the_rules() {
    let images = images();
    assert_eq!(
        list_problems(&Listing, &images[..2]),
        [
            "listing: tone lists an option twice or out of order",
            "listing.tone: no photo lists sepia",
            "listing.grid: segments, listed per photo",
            "listing.style: warm writes grid three, which a photo does not list",
        ]
    );
}

/// What is wrong with the tool's lossless views on `image` (`docs/engine.md`, Preset
/// rows and lossless views, Tests): a view the tool does not give at its defaults, which
/// one that declares the view and leaves `ProcessingTool::view` as provided has, or whose
/// inverse gives nothing; a coordinate whose default lies outside its range; and
/// coordinates in their ranges that put a control outside its own, for a view
/// [`GRID_EXEMPT`] names only with `exempt`. With none of these, neither setting a
/// coordinate in its range nor resetting one is ever refused while the others are in
/// theirs. The ranges are tried on a grid, ends included, of about 10,000 points. A view
/// may read the camera (decision 27): the session's `Photo` carries the image context the
/// app's pending display reads.
fn view_problems(tool: &dyn ProcessingTool, image: &ImageContext, exempt: bool) -> Vec<String> {
    let definition = tool.definition();
    let mut problems = Vec::new();
    for view in definition.views {
        let name = format!("{}.{}", definition.tool.id, view.id);
        let n = view.coordinates.len();
        let defaults = tool.defaults(image);
        match tool.view(view.id, &defaults, image) {
            Some(defaults) if defaults.len() == n => {
                for (coordinate, value) in view.coordinates.iter().zip(&defaults) {
                    if !(coordinate.min..=coordinate.max).contains(value) {
                        let id = coordinate.id;
                        problems.push(format!("{name}: {id} is {value} at the defaults"));
                    }
                }
            }
            other => problems.push(format!("{name}: {other:?} at the defaults")),
        }
        let steps = (10_000f64.powf(1.0 / n as f64) as usize).max(2);
        for at in 0..steps.pow(n as u32) {
            let coordinates: Vec<f32> = (0..n)
                .map(|i| {
                    let (c, k) = (&view.coordinates[i], at / steps.pow(i as u32) % steps);
                    match k {
                        k if k == steps - 1 => c.max,
                        k => c.min + (c.max - c.min) * k as f32 / (steps - 1) as f32,
                    }
                })
                .collect();
            let Some(values) = tool.unview(view.id, &coordinates, image) else {
                problems.push(format!("{name}: nothing for {coordinates:?}"));
                continue;
            };
            if !exempt && GRID_EXEMPT.iter().any(|(id, _)| *id == name) {
                continue;
            }
            for &control in view.controls {
                // `check::accept` holds a view's controls to be the tool's.
                let found = definition.control(control).expect("the view's control");
                match values.get(control).map(|value| found.check(value)) {
                    Some(Ok(())) => {}
                    Some(Err(error)) => {
                        problems.push(format!("{name} at {coordinates:?}: {error}"))
                    }
                    None => problems.push(format!("{name} at {coordinates:?}: no {control}")),
                }
            }
        }
    }
    problems
}

/// The views exempt from the grid, each with the coordinate held to a narrower check in
/// its place: White balance's temperature and tint, darktable's ranges, reach
/// coefficients outside the operation's 0 to 8 on every camera at extreme tints, negative
/// ones among them, which the session refuses (`EditError::Coordinate`; on the corpus 10
/// to 28 % of the grid, `crates/tools/tests/white_balance.rs`, `w_rows_on_the_corpus`),
/// so every temperature is held in range at a colour raw's as-shot tint instead, which
/// holds on the corpus. The whole grid is an ignored test until the architect decides what the
/// coefficients' range should hold (decision 30).
const GRID_EXEMPT: &[(&str, &str)] = &[("temperature.temperature_tint", "temperature")];

/// What is wrong with `tool`'s exempt views on `image`, a colour raw: a value of the
/// exempt coordinate over its range, ends included, by 1 % of it, the others at their
/// defaults for the photo, that puts a control the view covers outside its range. Not on
/// an image that is not a raw, which Apertrust does not open, where darktable reads the
/// coefficients through sRGB's matrix, its blue at 1901 K 4852.
fn exempt_problems(tool: &dyn ProcessingTool, image: &ImageContext) -> Vec<String> {
    let definition = tool.definition();
    let mut problems = Vec::new();
    if !image
        .camera
        .as_ref()
        .is_some_and(|camera| camera.cfa.is_some())
    {
        return problems;
    }
    for view in definition.views {
        let name = format!("{}.{}", definition.tool.id, view.id);
        let Some((_, swept)) = GRID_EXEMPT.iter().find(|(id, _)| *id == name) else {
            continue;
        };
        let at = view
            .coordinates
            .iter()
            .position(|c| c.id == *swept)
            .unwrap();
        let coordinate = &view.coordinates[at];
        let Some(defaults) = tool.view(view.id, &tool.defaults(image), image) else {
            problems.push(format!("{name}: no view at the defaults"));
            continue;
        };
        for step in 0..=100 {
            let mut coordinates = defaults.clone();
            coordinates[at] =
                coordinate.min + (coordinate.max - coordinate.min) * step as f32 / 100.0;
            let Some(values) = tool.unview(view.id, &coordinates, image) else {
                problems.push(format!("{name}: nothing for {coordinates:?}"));
                continue;
            };
            for &control in view.controls {
                let value = values.get(control).unwrap();
                if let Err(error) = definition.control(control).unwrap().check(value) {
                    problems.push(format!("{name}: {coordinates:?}: {error}"));
                }
            }
        }
    }
    problems
}

#[test]
fn every_built_in_view_is_given_and_its_coordinates_ranges_keep_its_controls_in_theirs() {
    // Colour calibration's hue and chroma, an exact view, reach darktable's chroma of 300
    // inside the operation's widened x and y (decision 27). White balance's temperature
    // and tint, darktable's model of the coefficients, keep every temperature in range at
    // a colour raw's as-shot tint ([`GRID_EXEMPT`]).
    for image in images() {
        for tool in TOOLS {
            let mut problems = view_problems(*tool, &image, false);
            problems.extend(exempt_problems(*tool, &image));
            assert!(problems.is_empty(), "{problems:?}");
        }
    }
}

/// The grid over the views [`GRID_EXEMPT`] names: darktable's temperature and tint range
/// over lights whose coefficients leave the operation's 0 to 8 on every camera (decision
/// 30).
#[test]
#[ignore = "darktable's temperature and tint reach coefficients outside 0 to 8 on every camera (decision 30)"]
fn an_exempt_view_s_coordinates_keep_its_controls_in_their_ranges() {
    for image in images() {
        for tool in TOOLS {
            let problems = view_problems(*tool, &image, true);
            assert!(problems.is_empty(), "{} problems", problems.len());
        }
    }
}

/// Decision 88 hides Exposure's two camera rows and no other: every other control a
/// photo cannot use is drawn disabled, and hiding one needs a decision of its own.
#[test]
fn only_exposure_s_camera_rows_are_ever_hidden() {
    let hiding = [
        ("exposure", "camera_bias"),
        ("exposure", "compensate_hilite_pres"),
    ];
    for tool in TOOLS {
        let definition = tool.definition();
        for control in definition.controls {
            if hiding.contains(&(definition.tool.id, control.id)) {
                continue;
            }
            for image in images() {
                assert!(
                    tool.visible(control.id, &image),
                    "{}.{} hidden",
                    definition.tool.id,
                    control.id
                );
            }
        }
    }
}

#[test]
fn every_message_a_definition_implies_is_translated() {
    let known: BTreeSet<String> = apertrust_ui::i18n::message_ids().into_iter().collect();
    // The closed list of units is in ui.ftl, whether a tool uses each yet or not, with
    // the message that writes a value with each.
    let mut wanted: Vec<String> = Unit::ALL.map(messages::unit).into();
    wanted.extend(Unit::ALL.map(messages::unit_value));
    // The photos the label values are asked on: the sweep's, and one whose camera held
    // exposure back.
    let mut images = images();
    let mut held = images[1].clone();
    held.camera.as_mut().unwrap().highlight_preservation = 1.0;
    images.push(held);
    for tool in TOOLS {
        let definition = tool.definition();
        wanted.extend(messages::of(definition));
        // A label's value is the tool's code's, so each control that has one on some
        // photo needs its message.
        for control in definition.controls {
            if images
                .iter()
                .any(|image| tool.label_value(control.id, image).is_some())
            {
                wanted.push(messages::label_value(definition.tool.id, control.id));
            }
        }
        // A segmented choice draws an option a photo does not list disabled, with its
        // reason (decision 88), so each option some photo leaves out needs one.
        for row in definition.rows() {
            let Row::Choice {
                control,
                style: apertrust_tools::ChoiceStyle::Segmented,
            } = row
            else {
                continue;
            };
            let Some(ControlKind::Choice { options, .. }) =
                definition.control(control).map(|found| found.kind)
            else {
                continue;
            };
            for option in options {
                if images
                    .iter()
                    .any(|image| !tool.options(control, image).contains(option))
                {
                    wanted.push(messages::option_unavailable(
                        definition.tool.id,
                        control,
                        option,
                    ));
                }
            }
        }
        // A picker that is not ready is drawn disabled with its reason (decision 88), so
        // each picker some option of its words' setting leaves unready, with only the
        // settings that have a default, needs one.
        for picker in definition.pickers {
            let options = picker
                .words_by
                .and_then(|setting| Some((setting, definition.choice_setting(setting)?)));
            let tried: Vec<Vec<(&str, ControlValue)>> = match options {
                Some((setting, options)) => (0..options.len())
                    .map(|at| vec![(setting, ControlValue::Choice(at as u32))])
                    .collect(),
                None => vec![Vec::new()],
            };
            if tried
                .iter()
                .any(|settings| !tool.picker_ready(picker.id, settings))
            {
                wanted.push(messages::picker_unavailable(definition.tool.id, picker.id));
            }
        }
    }
    let missing: Vec<&String> = wanted.iter().filter(|id| !known.contains(*id)).collect();
    assert!(missing.is_empty(), "no message for {missing:?}");
}

/// Every utility's labels, groups and settings have their words, as a processing tool's
/// rows do.
#[test]
fn every_message_a_utility_s_rows_imply_is_translated() {
    let known: BTreeSet<String> = apertrust_ui::i18n::message_ids().into_iter().collect();
    let wanted = UTILITIES.iter().flat_map(messages::of_utility);
    let missing: Vec<String> = wanted.filter(|id| !known.contains(id)).collect();
    assert!(missing.is_empty(), "no message for {missing:?}");
}
