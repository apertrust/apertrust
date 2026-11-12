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

//! Highlight reconstruction (`docs/modules/highlight-reconstruction.md`): darktable's
//! highlight reconstruction as the user edits with it, in the first pass (decision 2):
//! darktable's page, every control in `gui_init`'s order under its labels, each shown for
//! the methods darktable shows it for (`gui_changed`). It has no tabs, groups or presets of
//! its own, and works on the background only, as the whole raw stage does.
//!
//! - **Method** lists darktable's methods for a Bayer raw, in its order (`reload_defaults`):
//!   inpaint opposed, ported and the default, then reconstruct in LCh, clip highlights,
//!   segmentation based and guided laplacians, which are not ported, so they are drawn
//!   disabled and refused. It lists them on every photo for now, an interim until
//!   `Module::options` lands (decision 30): then each photo lists darktable's own, clip
//!   highlights alone and the default on an image that is not a raw or a four-colour
//!   sensor's raw, and a stored method the photo lacks binds as darktable's `gui_changed`
//!   rewrites it (`docs/modules/highlight-reconstruction.md`, Controls).
//! - **Clipping threshold,** 0 to 2 with darktable's three decimals: where clipping starts,
//!   as a fraction of each colour's white point.
//! - **With segmentation based:** Combine, Candidating and Rebuild, and with a rebuild
//!   mode Strength and Noise level; **with guided laplacians:** Noise level, Iterations,
//!   Inpaint a flat colour and Diameter of reconstruction. Their methods are not ported, so
//!   these controls are not either (`Control::unported`): they take only darktable's
//!   defaults, and their rows never show, as their methods cannot be chosen. Their
//!   parameters come with their methods (decision 51): the operation's schema holds only
//!   the method and the threshold, which the module owns.
//!
//! It drives `highlights`, which every raw runs, so the threshold starts at the
//! operation's default for the photo (rule 4 of `docs/modules/README.md`, decision 21),
//! the lower of 1 and the raw's linear response limit, as darktable's `reload_defaults`
//! has it, and the binding sets it only while the control is off it. darktable's showmask
//! buttons beside Clipping threshold, Combine, Candidating and Strength wait for the
//! renderer's quad (`docs/modules/highlight-reconstruction.md`, Gaps).
//!
//! A change runs demosaic again, as every raw-stage control does (decision 12).

use apertrust_plugin::{ImageContext, Value};

use crate::definition::{ChoiceStyle, Control, ControlKind, Definition, Row, Slider, Tab, Unit};
use crate::{Base, ControlValue, Controls, Module, ModuleValues, Owned, Setting};

pub struct HighlightReconstruction;

/// The operation it drives.
const HIGHLIGHTS: &str = "highlights";

/// The method, by the operation's choice ids, in the order darktable lists them for a
/// Bayer raw: from inpaint opposed to guided laplacians in its enum's declaration order.
/// darktable lists reconstruct colour, its enum's last, for no sensor.
pub const METHODS: [&str; 5] = ["opposed", "lch", "clip", "segments", "laplacian"];

/// The clipping threshold.
pub const CLIP: &str = "clip";

/// darktable's rebuild modes, in its enum's declaration order.
const REBUILDS: [&str; 7] = [
    "off",
    "generic",
    "flat_generic",
    "small_segments",
    "large_segments",
    "flat_small_segments",
    "flat_large_segments",
];

/// darktable's diameters of reconstruction, 2 to 4096 px, one per wavelet scale.
const DIAMETERS: [&str; 12] = [
    "px_2", "px_4", "px_8", "px_16", "px_32", "px_64", "px_128", "px_256", "px_512", "px_1024",
    "px_2048", "px_4096",
];

/// A number control. `step` is what darktable's slider steps by,
/// `dt_bauhaus_slider_get_step` over its range, since `gui_init` sets none, and what a key
/// steps by here. `unported` for a control of a method not ported.
const fn number(
    id: &'static str,
    (min, max): (f32, f32),
    step: f32,
    default: f32,
    unit: Option<Unit>,
    unported: bool,
) -> Control {
    Control {
        id,
        kind: ControlKind::Number {
            min,
            max,
            step,
            default,
        },
        unit,
        unported,
    }
}

/// A choice of a method not ported, which takes only darktable's default.
const fn unported_choice(
    id: &'static str,
    options: &'static [&'static str],
    default: u32,
) -> Control {
    Control {
        id,
        kind: ControlKind::Choice {
            options,
            default,
            unported: &[],
        },
        unit: None,
        unported: true,
    }
}

/// The controls in `gui_init`'s order, darktable's parameter names, ranges in the units
/// its sliders show and defaults: a percentage is the parameter times 100, as darktable's
/// `"%"` format shows a factor whose hard maximum is within ±10.
static CONTROLS: [Control; 10] = [
    Control {
        id: "mode",
        kind: ControlKind::Choice {
            options: &METHODS,
            default: 0,
            unported: &["lch", "clip", "segments", "laplacian"],
        },
        unit: None,
        unported: false,
    },
    // 0 to 2, the photo's default (`defaults`); darktable steps it by 0.01.
    number(CLIP, (0.0, 2.0), 0.01, 1.0, None, false),
    // 0 to 8, shown with no decimals; darktable steps it by 0.1.
    number("combine", (0.0, 8.0), 0.1, 2.0, None, true),
    // 0 % to 100 %, darktable's 40 %; darktable steps it by 1 %.
    number(
        "candidating",
        (0.0, 100.0),
        1.0,
        40.0,
        Some(Unit::Percent),
        true,
    ),
    unported_choice("recovery", &REBUILDS, 0),
    number(
        "strength",
        (0.0, 100.0),
        1.0,
        0.0,
        Some(Unit::Percent),
        true,
    ),
    // 0 to 0.5 with three decimals; darktable steps it by 0.005.
    number("noise_level", (0.0, 0.5), 0.005, 0.0, None, true),
    // A whole number, 1 to 256, darktable's 30.
    number("iterations", (1.0, 256.0), 1.0, 30.0, None, true),
    number(
        "solid_color",
        (0.0, 100.0),
        1.0,
        0.0,
        Some(Unit::Percent),
        true,
    ),
    // darktable's 128 px (slow).
    unported_choice("scales", &DIAMETERS, 6),
];

/// A slider with no decimals.
const fn whole(control: &'static str) -> Row {
    Row::Slider(Slider::new(control).decimals(0))
}

/// Noise level's row: darktable's three digits.
const NOISE_LEVEL: Row = Row::Slider(Slider::new("noise_level").decimals(3));

static DEFINITION: Definition = Definition {
    id: "highlights",
    version: 1,
    layers: false,
    auto: false,
    switched: false,
    controls: &CONTROLS,
    tabs: &[Tab {
        id: None,
        rows: &[
            Row::Choice {
                control: "mode",
                style: ChoiceStyle::List,
            },
            // `dt_bauhaus_slider_set_digits(g->clip, 3)`: "1.000".
            Row::Slider(Slider::new(CLIP).decimals(3)),
            // `gui_changed`: segmentation based shows combine, candidating and rebuild,
            // and a rebuild mode strength and noise level.
            Row::When {
                control: "mode",
                is: &["segments"],
                rows: &[
                    whole("combine"),
                    whole("candidating"),
                    Row::Choice {
                        control: "recovery",
                        style: ChoiceStyle::List,
                    },
                    Row::When {
                        control: "recovery",
                        is: &[
                            "generic",
                            "flat_generic",
                            "small_segments",
                            "large_segments",
                            "flat_small_segments",
                            "flat_large_segments",
                        ],
                        rows: &[whole("strength"), NOISE_LEVEL],
                    },
                ],
            },
            // Guided laplacians, on a Bayer raw, shows noise level, iterations, a flat
            // colour and the diameter.
            Row::When {
                control: "mode",
                is: &["laplacian"],
                rows: &[
                    NOISE_LEVEL,
                    whole("iterations"),
                    whole("solid_color"),
                    Row::Choice {
                        control: "scales",
                        style: ChoiceStyle::List,
                    },
                ],
            },
        ],
    }],
    before: &[],
    after: &[],
    presets: &[],
    views: &[],
};

/// Every parameter of `highlights`, the method and the threshold: the module is
/// darktable's and owns them all.
static OWNS: [Owned; 2] = [Owned::new(HIGHLIGHTS, "mode"), Owned::new(HIGHLIGHTS, CLIP)];

impl Module for HighlightReconstruction {
    fn definition(&self) -> &'static Definition {
        &DEFINITION
    }

    /// The definition's, but the threshold is the photo's ([`threshold`]), the
    /// operation's default for it.
    fn defaults(&self, image: &ImageContext) -> Controls {
        let mut defaults = DEFINITION.defaults();
        defaults.set(CLIP, ControlValue::Number(threshold(image)));
        defaults
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    /// Every control changes nothing on an image that is not a colour raw: one that is
    /// not a raw runs no raw stage, and on a monochrome raw darktable shows its "not
    /// applicable" page in place of the controls (`reload_defaults`, `gui_init`). Neither
    /// depends on the module's values. The controls not ported are unavailable
    /// everywhere, as the session reports every such control. A raw with no colour filter
    /// array is read as monochrome; a linear raw has none either, so the image context
    /// must tell the two apart before `Module::options` lists a linear raw's two methods.
    fn available(&self, _: &str, _: &Controls, image: &ImageContext) -> bool {
        image
            .camera
            .as_ref()
            .is_some_and(|camera| camera.cfa.is_some())
    }

    /// The threshold off its default for the photo sets `clip`, as it is. The method
    /// takes only inpaint opposed, its default, and the other controls only darktable's
    /// defaults, with no parameter yet, so nothing else is set.
    fn bind(&self, values: &ModuleValues, _image: &ImageContext, _base: &Base) -> Vec<Setting> {
        values
            .number(CLIP)
            .map(|clip| Setting::new(HIGHLIGHTS, CLIP, Value::Float(clip)))
            .into_iter()
            .collect()
    }
}

/// The photo's clipping threshold, as the highlights operation's defaults take it
/// (darktable's `reload_defaults`): the lower of 1 and the raw's linear response limit,
/// where the sensor stops responding linearly, a DNG's LinearResponseLimit, at least 0; 1
/// for an image that is not a raw or a limit that is not finite. This crate cannot call
/// the operation's own, so it is restated here; the module's tests check the two agree.
pub fn threshold(image: &ImageContext) -> f32 {
    match &image.camera {
        Some(camera) if camera.linear_response_limit.is_finite() => {
            camera.linear_response_limit.clamp(0.0, 1.0)
        }
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use apertrust_operations::OPERATIONS;
    use apertrust_operations::highlights::{Highlights, METHODS as CHOICES};
    use apertrust_plugin::{Kind, Operation, ParamValues};

    use super::*;
    use crate::check;
    use crate::testing::DarktableSlider;

    /// The test raw's context with this linear response limit.
    fn limited(limit: f32) -> ImageContext {
        let mut image = crate::testing::image();
        image.camera.as_mut().unwrap().linear_response_limit = limit;
        image
    }

    /// A monochrome raw: the test raw with no colour filter array.
    fn monochrome() -> ImageContext {
        let mut image = crate::testing::image();
        image.camera.as_mut().unwrap().cfa = None;
        image
    }

    /// Contexts whose thresholds differ: limits of 1 (unrecorded), below 1, above 1,
    /// below 0 and not finite, a monochrome raw and an image that is not a raw.
    fn images() -> Vec<ImageContext> {
        vec![
            crate::testing::image(),
            limited(0.9),
            limited(1.2),
            limited(-0.5),
            limited(f32::NAN),
            monochrome(),
            ImageContext::full(8, 8),
        ]
    }

    /// The operation's defaults for the photo, every parameter resolved over its schema.
    fn operation_defaults(image: &ImageContext) -> ParamValues {
        Highlights
            .schema()
            .resolve_over(&Highlights.defaults(image), &ParamValues::new())
            .unwrap()
    }

    /// What the binding sets for these values, as a session binds them.
    fn bind(set: &[(&'static str, ControlValue)], image: &ImageContext) -> Vec<Setting> {
        let mut values = ModuleValues::new();
        for (control, value) in set {
            values.set(control, value.clone());
        }
        let values = values.off_defaults(&HighlightReconstruction.defaults(image));
        HighlightReconstruction.bind(&values, image, &Base::new(OPERATIONS, image))
    }

    #[test]
    fn the_definition_holds_and_its_binding_sweeps_inside_the_schema() {
        DEFINITION.check().unwrap();
        let (accepted, rejected) = check::accept(&[&HighlightReconstruction], OPERATIONS);
        assert!(rejected.is_empty(), "{rejected:?}");
        assert_eq!(accepted.len(), 1);
        for image in images() {
            let problems = check::sweep(&HighlightReconstruction, OPERATIONS, &image);
            assert!(problems.is_empty(), "{problems:?}");
        }
    }

    #[test]
    fn its_page_is_darktable_s_in_gui_init_s_order_on_one_untitled_tab() {
        assert_eq!(DEFINITION.tabs.len(), 1);
        assert_eq!(DEFINITION.tabs[0].id, None);
        let controls: Vec<&str> = CONTROLS.iter().map(|c| c.id).collect();
        assert_eq!(
            controls,
            [
                "mode",
                "clip",
                "combine",
                "candidating",
                "recovery",
                "strength",
                "noise_level",
                "iterations",
                "solid_color",
                "scales",
            ]
        );
        // Noise level has a row for each method that shows it.
        let rows: Vec<&str> = DEFINITION
            .rows()
            .flat_map(|row| row.controls().iter().copied())
            .collect();
        assert_eq!(
            rows,
            [
                "mode",
                "clip",
                "combine",
                "candidating",
                "recovery",
                "strength",
                "noise_level",
                "noise_level",
                "iterations",
                "solid_color",
                "scales",
            ]
        );
        assert!(!DEFINITION.layers && !DEFINITION.auto);
    }

    /// The controls `rows` show while the module's controls hold `values`, every control,
    /// as darktable's `gui_changed` shows its widgets.
    fn shown(rows: &'static [Row], values: &Controls, out: &mut Vec<&'static str>) {
        for row in rows {
            match row {
                Row::When { control, is, rows } => {
                    let found = DEFINITION.control(control).unwrap();
                    if found.is_one_of(values.get(control).unwrap(), is) {
                        shown(rows, values, out);
                    }
                }
                _ => out.extend(row.controls()),
            }
        }
    }

    /// Each method shows darktable's rows for it, whether or not it can be chosen yet.
    #[test]
    fn each_method_shows_darktable_s_rows_for_it() {
        let image = crate::testing::image();
        let with = |method: &str, recovery: &str| {
            let mut values = HighlightReconstruction.defaults(&image);
            let at = |list: &[&str], id: &str| list.iter().position(|o| *o == id).unwrap() as u32;
            values.set("mode", ControlValue::Choice(at(&METHODS, method)));
            values.set("recovery", ControlValue::Choice(at(&REBUILDS, recovery)));
            let mut out = Vec::new();
            shown(DEFINITION.tabs[0].rows, &values, &mut out);
            out
        };
        for method in ["opposed", "lch", "clip"] {
            assert_eq!(with(method, "generic"), ["mode", "clip"], "{method}");
        }
        assert_eq!(
            with("segments", "off"),
            ["mode", "clip", "combine", "candidating", "recovery"]
        );
        for rebuild in &REBUILDS[1..] {
            assert_eq!(
                with("segments", rebuild),
                [
                    "mode",
                    "clip",
                    "combine",
                    "candidating",
                    "recovery",
                    "strength",
                    "noise_level"
                ],
                "{rebuild}"
            );
        }
        assert_eq!(
            with("laplacian", "generic"),
            [
                "mode",
                "clip",
                "noise_level",
                "iterations",
                "solid_color",
                "scales"
            ]
        );
    }

    /// The module owns every parameter of `highlights`, the method's options are the
    /// operation's choices, and the threshold's range is the schema's.
    #[test]
    fn it_owns_every_parameter_of_the_operation_and_its_ranges_are_the_schema_s() {
        let schema = Highlights.schema();
        let owned: Vec<&str> = OWNS.iter().map(|owned| owned.param).collect();
        let params: Vec<&str> = schema.params.iter().map(|param| param.name).collect();
        assert_eq!(owned, params);
        for method in METHODS {
            assert!(CHOICES.contains(&method), "{method}");
        }
        // darktable's reconstruct colour, `inpaint`, is listed for no sensor.
        assert!(!METHODS.contains(&"inpaint"));
        let Kind::Float { min, max, .. } = schema.param(CLIP).unwrap().kind else {
            panic!("clip is a number");
        };
        let Some(ControlKind::Number {
            min: low,
            max: high,
            ..
        }) = DEFINITION.control(CLIP).map(|control| control.kind)
        else {
            panic!("clip is a number");
        };
        assert_eq!((low, high), (min, max));
    }

    /// Every default is the operation's for the photo: the method inpaint opposed, its
    /// default, and the threshold its `defaults`, the restatement agreeing with it on
    /// every context.
    #[test]
    fn its_defaults_are_the_operation_s_for_the_photo() {
        for image in images() {
            let ours = HighlightReconstruction.defaults(&image);
            let theirs = operation_defaults(&image);
            assert_eq!(
                ours.number(CLIP).to_bits(),
                theirs.float(CLIP).to_bits(),
                "{:?}",
                image.camera.as_ref().map(|c| c.linear_response_limit)
            );
            let method = METHODS[ours.choice("mode") as usize];
            assert_eq!(CHOICES[theirs.choice("mode") as usize], method);
            assert_eq!(method, "opposed");
        }
        let clip = |image: &ImageContext| HighlightReconstruction.defaults(image).number(CLIP);
        assert_eq!(clip(&limited(0.9)), 0.9);
        assert_eq!(clip(&limited(1.2)), 1.0);
        assert_eq!(clip(&limited(-0.5)), 0.0);
        assert_eq!(clip(&limited(f32::NAN)), 1.0);
        assert_eq!(clip(&ImageContext::full(8, 8)), 1.0);
    }

    #[test]
    fn at_its_defaults_it_sets_nothing() {
        for image in images() {
            let base = Base::new(OPERATIONS, &image);
            assert!(
                HighlightReconstruction
                    .bind(&ModuleValues::new(), &image, &base)
                    .is_empty()
            );
            let defaults = HighlightReconstruction.defaults(&image);
            let written: Vec<(&str, ControlValue)> = CONTROLS
                .iter()
                .map(|control| (control.id, defaults.get(control.id).unwrap().clone()))
                .collect();
            assert!(bind(&written, &image).is_empty());
        }
    }

    #[test]
    fn the_threshold_off_its_default_sets_clip_as_it_is() {
        let image = limited(0.9);
        for clip in [0.0, 0.5, 1.0, 1.234, 2.0] {
            assert_eq!(
                bind(&[(CLIP, ControlValue::Number(clip))], &image),
                [Setting::new(HIGHLIGHTS, CLIP, Value::Float(clip))]
            );
        }
        assert!(bind(&[(CLIP, ControlValue::Number(0.9))], &image).is_empty());
    }

    #[test]
    fn what_is_not_ported_takes_only_its_default() {
        let mode = DEFINITION.control("mode").unwrap();
        assert_eq!(mode.check(&ControlValue::Choice(0)), Ok(()));
        for (index, method) in METHODS.iter().enumerate().skip(1) {
            assert_eq!(
                mode.check(&ControlValue::Choice(index as u32)),
                Err(crate::ControlError::Unported {
                    control: "mode",
                    option: method
                })
            );
        }
        let unported: Vec<&str> = CONTROLS
            .iter()
            .filter(|control| control.unported)
            .map(|control| control.id)
            .collect();
        assert_eq!(
            unported,
            [
                "combine",
                "candidating",
                "recovery",
                "strength",
                "noise_level",
                "iterations",
                "solid_color",
                "scales"
            ]
        );
        for control in CONTROLS.iter().filter(|control| control.unported) {
            assert_eq!(control.check(&control.default()), Ok(()), "{}", control.id);
            let other = match control.kind {
                ControlKind::Number { max, .. } => ControlValue::Number(max),
                ControlKind::Choice { default, .. } => ControlValue::Choice(default + 1),
                _ => unreachable!(),
            };
            assert_eq!(
                control.check(&other),
                Err(crate::ControlError::NotPorted {
                    control: control.id
                })
            );
        }
    }

    #[test]
    fn its_controls_act_on_a_colour_raw_only() {
        for image in images() {
            let colour = image.camera.as_ref().is_some_and(|c| c.cfa.is_some());
            let defaults = HighlightReconstruction.defaults(&image);
            for control in &CONTROLS {
                assert_eq!(
                    HighlightReconstruction.available(control.id, &defaults, &image),
                    colour,
                    "{}",
                    control.id
                );
            }
        }
    }

    /// darktable's highlight reconstruction sliders (`src/iop/highlights.c`, the
    /// params and `gui_init`), in its parameters' units: `docs/modules/highlight-
    /// reconstruction.md`'s slider table. None has a soft range.
    const DARKTABLE: [DarktableSlider; 7] = [
        // `dt_bauhaus_slider_set_digits(g->clip, 3)`.
        DarktableSlider::new("clip", (0.0, 2.0), 3),
        // `dt_bauhaus_slider_set_digits(g->combine, 0)`.
        DarktableSlider::new("combine", (0.0, 8.0), 0),
        // `dt_bauhaus_slider_set_format(…, "%")` over a hard maximum within ±10, a factor
        // of 100, then `dt_bauhaus_slider_set_digits(…, 0)`.
        DarktableSlider::new("candidating", (0.0, 1.0), 0).shown_as(100.0, 0.0),
        DarktableSlider::new("strength", (0.0, 1.0), 0).shown_as(100.0, 0.0),
        // `dt_bauhaus_slider_from_params`' three digits for a range of 0.5.
        DarktableSlider::new("noise_level", (0.0, 0.5), 3),
        // An int: no digits, a step of 1.
        DarktableSlider::whole("iterations", (1.0, 256.0)),
        // `"%"` takes two of `dt_bauhaus_slider_from_params`' two digits.
        DarktableSlider::new("solid_color", (0.0, 1.0), 0).shown_as(100.0, 0.0),
    ];

    #[test]
    fn its_sliders_are_darktable_s() {
        let differences = crate::testing::slider_differences(
            &HighlightReconstruction,
            &DEFINITION.defaults(),
            &DARKTABLE,
        );
        assert!(differences.is_empty(), "{differences:#?}");
    }
}
