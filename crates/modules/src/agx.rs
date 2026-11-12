// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2025-2026  darktable developers (ported from src/iop/agx.c)
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

//! AgX (`docs/modules/agx.md`): darktable's AgX as the user edits with it, in the
//! first pass (decision 2), the tone mapper of the Base tab: darktable's page, its
//! controls in `gui_init`'s order under its labels.
//!
//! - **Settings:** the input exposure range, white and black relative exposure and
//!   dynamic range scaling, with Auto tune levels and Read exposure; the basic curve
//!   parameters, the pivot, contrast and the shoulder's and toe's powers; then darktable's
//!   collapsible sections, Show curve, Advanced curve parameters and Look.
//! - **Primaries:** the base primaries and Disable adjustments; while adjustments are on,
//!   Reset primaries, each primary's attenuation and rotation before tone mapping, Reverse
//!   all and Set from above, and while Reverse all is off the purity boosts and reverse
//!   rotations after it.
//!
//! Each control is one `agx` parameter, its id the parameter's name, shown as darktable's
//! slider shows it: its percentages as percentages, so the binding divides them by 100,
//! its rotations in degrees, and the pivot in EV against the range, as darktable's slider
//! shows `curve_pivot_x` with the range's width as its factor and its black as its offset.
//! The module stores the pivot in EV and writes `curve_pivot_x` from it and the range, so
//! moving the range keeps the pivot's exposure, as darktable's `_update_pivot_x` does. Its
//! slider is darktable's for the range (decision 30): it holds black to white relative
//! exposure, its track the same, and steps by darktable's step for the range's width; a
//! pivot the range's move leaves outside is clamped to the range's end and stored in the
//! same change, as `_update_pivot_x` stores it.
//!
//! It drives `agx`, which every photo runs, so each control starts at the operation's
//! default (rule 4 of `docs/modules/README.md`), darktable's scene-referred defaults, and
//! the binding sets a parameter only while its control is off it, the pivot's while it or
//! the range is. Dynamic range scaling, which only darktable's pickers and Read exposure
//! read, is not ported, nor is the base primaries' export profile: both are drawn disabled
//! and refused. The pickers on the exposures and the pivot are in the definition, as
//! darktable's page has them, and the renderer draws a quad once its tool runs; the four
//! actions are drawn disabled until theirs come. Show curve is empty until a read-only
//! graph row exists. darktable's presets wait for the header's presets menu.

#![allow(
    clippy::excessive_precision,
    reason = "defaults are copied digit for digit from darktable"
)]

use apertrust_plugin::{ImageContext, Value};

use crate::definition::{
    ChoiceStyle, Control, ControlKind, Definition, Picker, PickerKind, Quad, Range, Row, Slider,
    Space, Tab, Unit, slider_step,
};
use crate::{Base, Controls, Module, ModuleValues, Owned, Setting};

pub struct Agx;

/// The operation it drives.
const AGX: &str = "agx";

/// darktable's `RAD_2_DEG`, 180/π, as its slider's f32 factor holds it.
const RAD_2_DEG: f32 = (180.0 / std::f64::consts::PI) as f32;

/// The largest rotation darktable allows, in radians: about 30°.
#[allow(
    clippy::approx_constant,
    reason = "darktable's own limit, 0.5236, which is not π/6"
)]
const ROTATION: f32 = 0.5236;

/// White and black relative exposure, the input range, in EV around mid-grey.
pub const WHITE: &str = "range_white_relative_ev";
pub const BLACK: &str = "range_black_relative_ev";

/// The pivot relative exposure, stored in EV around mid-grey, as darktable's slider shows
/// `curve_pivot_x`, its parameter ([`pivot_x`]).
pub const PIVOT: &str = "curve_pivot_x";

/// Dynamic range scaling: the margin darktable's pickers and Read exposure add to the
/// range they compute, and by which its `gui_changed` rescales the range. No render reads
/// it, so the operation has no parameter for it, and it is not ported.
pub const SCALING: &str = "dynamic_range_scaling";

/// The base primaries, by the operation's choices, darktable's order; the export
/// profile is not ported.
pub const BASE_PRIMARIES: &str = "base_primaries";
pub const PRIMARIES: [&str; 6] = [
    "export_profile",
    "working_profile",
    "rec2020",
    "display_p3",
    "adobe_rgb",
    "srgb",
];

/// The toggles, each one `agx` parameter.
pub const FLAGS: [&str; 3] = [
    "auto_gamma",
    "disable_primaries_adjustments",
    "completely_reverse_primaries",
];

/// A number control's parameter, and what the control shows of it: the parameter times
/// `factor`, 100 where darktable's slider shows a percentage, `RAD_2_DEG` for a rotation
/// in degrees, 1 for EV and plain numbers. The pivot is apart ([`PIVOT`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parameter {
    pub control: &'static str,
    pub param: &'static str,
    pub factor: f32,
}

impl Parameter {
    /// The parameter for a value the control shows: darktable's `value / factor`, in f32.
    pub fn param(&self, shown: f32) -> f32 {
        shown / self.factor
    }

    /// What the control shows for a parameter: darktable's `parameter × factor`.
    pub fn shown(&self, param: f32) -> f32 {
        param * self.factor
    }
}

const fn shown_as(id: &'static str, factor: f32) -> Parameter {
    Parameter {
        control: id,
        param: id,
        factor,
    }
}

/// Every number control's parameter but the pivot's, in the page's order.
pub static PARAMETERS: [Parameter; 30] = [
    shown_as(WHITE, 1.0),
    shown_as(BLACK, 1.0),
    shown_as("curve_pivot_y_linear_output", 100.0),
    shown_as("curve_contrast_around_pivot", 1.0),
    shown_as("curve_shoulder_power", 1.0),
    shown_as("curve_toe_power", 1.0),
    shown_as("curve_linear_ratio_above_pivot", 100.0),
    shown_as("curve_target_display_white_ratio", 100.0),
    shown_as("curve_linear_ratio_below_pivot", 100.0),
    shown_as("curve_target_display_black_ratio", 100.0),
    shown_as("curve_gamma", 1.0),
    shown_as("look_slope", 1.0),
    shown_as("look_lift", 1.0),
    shown_as("look_brightness", 1.0),
    shown_as("look_saturation", 100.0),
    shown_as("look_original_hue_mix_ratio", 100.0),
    shown_as("red_inset", 100.0),
    shown_as("red_rotation", RAD_2_DEG),
    shown_as("green_inset", 100.0),
    shown_as("green_rotation", RAD_2_DEG),
    shown_as("blue_inset", 100.0),
    shown_as("blue_rotation", RAD_2_DEG),
    shown_as("master_outset_ratio", 100.0),
    shown_as("master_unrotation_ratio", 100.0),
    shown_as("red_outset", 100.0),
    shown_as("red_unrotation", RAD_2_DEG),
    shown_as("green_outset", 100.0),
    shown_as("green_unrotation", RAD_2_DEG),
    shown_as("blue_outset", 100.0),
    shown_as("blue_unrotation", RAD_2_DEG),
];

/// The pivot's exposure as darktable's slider shows `curve_pivot_x`: its position in the
/// range, times the range's width plus its black, in f32 as darktable computes it.
pub fn pivot_ev(pivot_x: f32, black: f32, white: f32) -> f32 {
    pivot_x * (white - black) + black
}

/// `curve_pivot_x` for the pivot at `ev` in the range from `black` to `white`, the pivot
/// kept inside it: darktable's `_update_pivot_x`, which writes it when the range moves,
/// and its slider's `(value − offset) / factor`, in f32 as darktable computes them. The
/// session keeps the pivot inside the range (decision 30), but a binding takes any value
/// an edit holds, so it keeps it inside too. The range is never empty: black is at most
/// −0.1 EV and white at least 0.1.
pub fn pivot_x(ev: f32, black: f32, white: f32) -> f32 {
    (ev.clamp(black, white) - black) / (white - black)
}

/// A number control, `range` and `default` in the parameter's units as darktable's params
/// hold them, shown times `factor`. `step` is darktable's, in the units shown: what its
/// slider steps by over its soft range, which a key steps by here.
const fn number(
    id: &'static str,
    (min, max): (f32, f32),
    factor: f32,
    step: f32,
    default: f32,
    unit: Option<Unit>,
) -> Control {
    Control {
        id,
        kind: ControlKind::Number {
            min: min * factor,
            max: max * factor,
            step,
            default: default * factor,
        },
        unit,
        unported: false,
    }
}

/// A percentage of a parameter from 0 to 1, darktable's "%" slider, stepping by 1 %.
const fn percent(id: &'static str, default: f32) -> Control {
    number(id, (0.0, 1.0), 100.0, 1.0, default, Some(Unit::Percent))
}

/// A primary's attenuation or purity boost: 0 % to 99 %, stepping by 1 %.
const fn purity(id: &'static str, default: f32) -> Control {
    number(id, (0.0, 0.99), 100.0, 1.0, default, Some(Unit::Percent))
}

/// A primary's rotation, ±30° in darktable's radians, stepping by 0.1°.
const fn rotation(id: &'static str, default: f32) -> Control {
    number(
        id,
        (-ROTATION, ROTATION),
        RAD_2_DEG,
        0.1,
        default,
        Some(Unit::Degrees),
    )
}

const fn flag(id: &'static str) -> Control {
    Control {
        id,
        kind: ControlKind::Bool { default: false },
        unit: None,
        unported: false,
    }
}

/// darktable's scene-referred default primaries, Blender's (`_set_blenderlike_primaries`):
/// the insets and rotations before tone mapping, the purity boosts after it, and the
/// reverse rotations equal to the rotations.
const INSETS: [f32; 3] = [0.29462451, 0.25861925, 0.14641371];
const ROTATIONS: [f32; 3] = [0.03540329, -0.02108586, -0.06305724];
const OUTSETS: [f32; 3] = [0.290776401758, 0.263155400753, 0.045810721815];

/// darktable's hard ranges are the controls', in the units its sliders show; its soft
/// ranges are the rows' (`Slider::soft`). Every default is the operation's, darktable's
/// scene-referred default (`reload_defaults`).
static CONTROLS: [Control; 36] = [
    number(WHITE, (0.1, 20.0), 1.0, 0.1, 6.5, Some(Unit::Ev)),
    number(BLACK, (-20.0, -0.1), 1.0, 0.1, -10.0, Some(Unit::Ev)),
    Control {
        unported: true,
        ..number(SCALING, (-0.5, 2.0), 100.0, 0.5, 0.1, Some(Unit::Percent))
    },
    // Every pivot any range gives: its range for the module's values is the input range,
    // black to white, as darktable's slider has it (`Agx::range`).
    number(PIVOT, (-20.0, 20.0), 1.0, 0.1, 0.0, Some(Unit::Ev)),
    percent("curve_pivot_y_linear_output", 0.18),
    number(
        "curve_contrast_around_pivot",
        (0.1, 10.0),
        1.0,
        0.05,
        3.0,
        None,
    ),
    number("curve_shoulder_power", (0.0, 10.0), 1.0, 0.05, 3.3, None),
    number("curve_toe_power", (0.0, 10.0), 1.0, 0.05, 1.5, None),
    percent("curve_linear_ratio_above_pivot", 0.0),
    number(
        "curve_target_display_white_ratio",
        (0.2, 1.0),
        100.0,
        0.5,
        1.0,
        Some(Unit::Percent),
    ),
    percent("curve_linear_ratio_below_pivot", 0.0),
    number(
        "curve_target_display_black_ratio",
        (0.0, 0.15),
        100.0,
        0.01,
        0.0,
        Some(Unit::Percent),
    ),
    flag("auto_gamma"),
    number("curve_gamma", (0.01, 100.0), 1.0, 0.05, 2.2, None),
    number("look_slope", (0.0, 10.0), 1.0, 0.01, 1.0, None),
    number("look_lift", (-1.0, 1.0), 1.0, 0.005, 0.0, None),
    number("look_brightness", (0.0, 100.0), 1.0, 0.01, 1.0, None),
    number(
        "look_saturation",
        (0.0, 10.0),
        100.0,
        1.0,
        1.0,
        Some(Unit::Percent),
    ),
    percent("look_original_hue_mix_ratio", 0.6),
    Control {
        id: BASE_PRIMARIES,
        kind: ControlKind::Choice {
            options: &PRIMARIES,
            default: 2,
            unported: &["export_profile"],
        },
        unit: None,
        unported: false,
    },
    flag("disable_primaries_adjustments"),
    purity("red_inset", INSETS[0]),
    rotation("red_rotation", ROTATIONS[0]),
    purity("green_inset", INSETS[1]),
    rotation("green_rotation", ROTATIONS[1]),
    purity("blue_inset", INSETS[2]),
    rotation("blue_rotation", ROTATIONS[2]),
    flag("completely_reverse_primaries"),
    number(
        "master_outset_ratio",
        (0.0, 2.0),
        100.0,
        1.0,
        1.0,
        Some(Unit::Percent),
    ),
    number(
        "master_unrotation_ratio",
        (0.0, 2.0),
        100.0,
        1.0,
        0.0,
        Some(Unit::Percent),
    ),
    purity("red_outset", OUTSETS[0]),
    rotation("red_unrotation", ROTATIONS[0]),
    purity("green_outset", OUTSETS[1]),
    rotation("green_unrotation", ROTATIONS[1]),
    purity("blue_outset", OUTSETS[2]),
    rotation("blue_unrotation", ROTATIONS[2]),
];

/// A slider row as darktable's: two decimals, filled, its track over the whole range.
const fn slider(control: &'static str) -> Row {
    Row::Slider(Slider::new(control))
}

/// A slider row over darktable's soft range, `min` to `max` in the units shown.
const fn soft(control: &'static str, min: f32, max: f32) -> Row {
    Row::Slider(Slider::new(control).soft(min, max))
}

/// An exposure or the pivot, with darktable's picker `picker`, over its soft range.
const fn picked(control: &'static str, picker: &'static str, (min, max): (f32, f32)) -> Row {
    Row::Slider(
        Slider::new(control)
            .quad(Quad::Picker(picker))
            .soft(min, max),
    )
}

/// darktable's pickers on sliders (`DT_COLOR_PICKER_AREA | DT_COLOR_PICKER_DENOISE`): white
/// or black relative exposure from the area's largest maximum or smallest minimum, the
/// pivot kept within the new range; the pivot from the mean's luminance, and with Pivot
/// target output also the output that keeps its brightness. Auto tune levels, both
/// exposures at once, is still an action ([`SETTINGS`]).
static PICKERS: [Picker; 4] = [
    picker("white", &[WHITE, PIVOT]),
    picker("black", &[BLACK, PIVOT]),
    picker("pivot", &[PIVOT]),
    picker("pivot_output", &[PIVOT, "curve_pivot_y_linear_output"]),
];

/// A denoised area picker of the module's input writing `writes`.
const fn picker(id: &'static str, writes: &'static [&'static str]) -> Picker {
    Picker {
        id,
        operation: AGX,
        kind: PickerKind::Area,
        denoise: true,
        output: false,
        space: Space::Operation,
        keep: false,
        writes,
        tools: &[],
    }
}

/// A primary's attenuation or purity boost, unfilled, as darktable's `set_feedback(0)`
/// draws it.
const fn purity_row(control: &'static str) -> Row {
    Row::Slider(Slider::new(control).no_fill())
}

/// A primary's rotation, unfilled, with one decimal.
const fn rotation_row(control: &'static str) -> Row {
    Row::Slider(Slider::new(control).decimals(1).no_fill())
}

/// darktable's Settings page: the exposure box, the basic curve box, then its three
/// collapsible sections, as `gui_init` builds them without the optional curve tab.
static SETTINGS: [Row; 15] = [
    Row::Label {
        id: "input_exposure_range",
    },
    picked(WHITE, "white", (1.0, 10.0)),
    picked(BLACK, "black", (-20.0, -1.0)),
    // darktable sets only its soft maximum, 0.5; its soft minimum is its hard one.
    soft(SCALING, -50.0, 50.0),
    // darktable's picker setting both exposures, and its camera button on the same line.
    Row::Action {
        id: "auto_tune_levels",
    },
    Row::Action {
        id: "read_exposure",
    },
    Row::Label {
        id: "basic_curve_parameters",
    },
    // The pivot's track spans the input range, here its default (`Agx::range`).
    picked(PIVOT, "pivot", (-10.0, 6.5)),
    Row::Slider(Slider::new("curve_pivot_y_linear_output").quad(Quad::Picker("pivot_output"))),
    soft("curve_contrast_around_pivot", 0.1, 5.0),
    soft("curve_shoulder_power", 1.0, 5.0),
    soft("curve_toe_power", 1.0, 5.0),
    // darktable's read-only graph of the curve, once a definition can name one.
    Row::Group {
        id: "show_curve",
        rows: &[],
    },
    Row::Group {
        id: "advanced_curve_parameters",
        rows: &[
            slider("curve_linear_ratio_above_pivot"),
            soft("curve_target_display_white_ratio", 50.0, 100.0),
            slider("curve_linear_ratio_below_pivot"),
            soft("curve_target_display_black_ratio", 0.0, 2.5),
            Row::Toggle {
                control: "auto_gamma",
            },
            // darktable hides the gamma while it keeps the pivot on the diagonal.
            Row::When {
                control: "auto_gamma",
                is: &["false"],
                rows: &[soft("curve_gamma", 1.0, 5.0)],
            },
        ],
    },
    Row::Group {
        id: "look",
        rows: &[
            soft("look_slope", 0.0, 2.0),
            soft("look_lift", -0.5, 0.5),
            soft("look_brightness", 0.0, 2.0),
            soft("look_saturation", 0.0, 200.0),
            slider("look_original_hue_mix_ratio"),
        ],
    },
];

/// darktable's Primaries page: the base and Disable adjustments, then, while adjustments
/// are on, the rest, the purity boosts and reverse rotations while Reverse all is off.
static PRIMARIES_PAGE: [Row; 3] = [
    Row::Choice {
        control: BASE_PRIMARIES,
        style: ChoiceStyle::List,
    },
    Row::Toggle {
        control: "disable_primaries_adjustments",
    },
    Row::When {
        control: "disable_primaries_adjustments",
        is: &["false"],
        rows: &[
            // darktable's menu of three primaries to start from: Blender-like, Smooth and
            // Unmodified.
            Row::Action {
                id: "reset_primaries",
            },
            Row::Label {
                id: "before_tone_mapping",
            },
            purity_row("red_inset"),
            rotation_row("red_rotation"),
            purity_row("green_inset"),
            rotation_row("green_rotation"),
            purity_row("blue_inset"),
            rotation_row("blue_rotation"),
            Row::Label {
                id: "after_tone_mapping",
            },
            Row::Toggle {
                control: "completely_reverse_primaries",
            },
            // darktable's button on Reverse all's line, insensitive while it is on.
            Row::Action {
                id: "set_from_above",
            },
            Row::When {
                control: "completely_reverse_primaries",
                is: &["false"],
                rows: &[
                    slider("master_outset_ratio"),
                    slider("master_unrotation_ratio"),
                    purity_row("red_outset"),
                    rotation_row("red_unrotation"),
                    purity_row("green_outset"),
                    rotation_row("green_unrotation"),
                    purity_row("blue_outset"),
                    rotation_row("blue_unrotation"),
                ],
            },
        ],
    },
];

static DEFINITION: Definition = Definition {
    id: AGX,
    version: 1,
    // Background only: the tone mapper's parameters are global (decision 33).
    layers: false,
    auto: false,
    switched: false,
    controls: &CONTROLS,
    tabs: &[
        Tab {
            id: Some("settings"),
            rows: &SETTINGS,
        },
        Tab {
            id: Some("primaries"),
            rows: &PRIMARIES_PAGE,
        },
    ],
    before: &[],
    after: &[],
    presets: &[],
    views: &[],
    pickers: &PICKERS,
};

/// Every parameter of `agx`: the module is darktable's and owns them all, each with its
/// control.
static OWNS: [Owned; 35] = owns();

const fn owns() -> [Owned; 35] {
    let mut owns = [Owned::new(AGX, PIVOT); 35];
    let mut i = 0;
    while i < PARAMETERS.len() {
        owns[i] = Owned::new(AGX, PARAMETERS[i].param);
        i += 1;
    }
    owns[30] = Owned::new(AGX, BASE_PRIMARIES);
    owns[31] = Owned::new(AGX, FLAGS[0]);
    owns[32] = Owned::new(AGX, FLAGS[1]);
    owns[33] = Owned::new(AGX, FLAGS[2]);
    // owns[34] is the pivot's.
    owns
}

impl Module for Agx {
    fn definition(&self) -> &'static Definition {
        &DEFINITION
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    /// The pivot's range is the input range, black to white relative exposure, its track
    /// the same and its step darktable's for the range's width, as darktable's slider shows
    /// `curve_pivot_x` from 0 to 1 with the width as its factor and black as its offset
    /// (decision 30): 0.1 EV at the default 16.5 EV. Every other control's is the
    /// definition's.
    fn range(&self, control: &str, values: &Controls) -> Option<Range> {
        let range = DEFINITION.range(control)?;
        if control != PIVOT {
            return Some(range);
        }
        let (black, white) = (values.number(BLACK), values.number(WHITE));
        Some(Range {
            min: black,
            max: white,
            soft: (black, white),
            step: slider_step(0.0, 1.0, white - black),
        })
    }

    /// Each number off its default sets its parameter, the value shown over its factor;
    /// the pivot sets `curve_pivot_x` while it or the range is off its default, from all
    /// three; each toggle and the base primaries set theirs. None, at the defaults, sets
    /// anything. Dynamic range scaling takes only its default, and sets nothing.
    fn bind(&self, values: &ModuleValues, _image: &ImageContext, _base: &Base) -> Vec<Setting> {
        let mut settings: Vec<Setting> = PARAMETERS
            .iter()
            .filter_map(|p| {
                let value = values.number(p.control)?;
                Some(Setting::new(AGX, p.param, Value::Float(p.param(value))))
            })
            .collect();
        if [PIVOT, BLACK, WHITE]
            .iter()
            .any(|id| values.number(id).is_some())
        {
            let defaults = DEFINITION.defaults();
            let of = |id| values.number(id).unwrap_or_else(|| defaults.number(id));
            let at = pivot_x(of(PIVOT), of(BLACK), of(WHITE));
            settings.push(Setting::new(AGX, PIVOT, Value::Float(at)));
        }
        if let Some(base) = values.choice(BASE_PRIMARIES) {
            settings.push(Setting::new(AGX, BASE_PRIMARIES, Value::Choice(base)));
        }
        for flag in FLAGS {
            if let Some(on) = values.flag(flag) {
                settings.push(Setting::new(AGX, flag, Value::Bool(on)));
            }
        }
        settings
    }
}

#[cfg(test)]
mod tests {
    use apertrust_operations::OPERATIONS;
    use apertrust_operations::agx::Agx as AgxOperation;
    use apertrust_plugin::{Kind, Operation};

    use super::*;
    use crate::check;
    use crate::testing::{DarktablePicker, DarktableSlider};
    use crate::values::ControlValue;

    fn bind(set: &[(&str, ControlValue)]) -> Vec<Setting> {
        let image = crate::testing::image();
        let mut values = ModuleValues::new();
        for (control, value) in set {
            values.set(control, value.clone());
        }
        let values = values.off_defaults(&Agx.defaults(&image));
        Agx.bind(&values, &image, &Base::new(OPERATIONS, &image))
    }

    /// What `settings` sets `param` to.
    fn value(settings: &[Setting], param: &str) -> Option<Value> {
        settings
            .iter()
            .find(|setting| setting.param == param)
            .map(|setting| setting.value)
    }

    #[test]
    fn the_definition_holds_and_its_binding_sweeps_inside_the_schema() {
        DEFINITION.check().unwrap();
        let (accepted, rejected) = check::accept(&[&Agx], OPERATIONS);
        assert!(rejected.is_empty(), "{rejected:?}");
        assert_eq!(accepted.len(), 1);
        for image in [crate::testing::image(), ImageContext::full(8, 8)] {
            let problems = check::sweep(&Agx, OPERATIONS, &image);
            assert!(problems.is_empty(), "{problems:?}");
        }
    }

    #[test]
    fn its_page_is_darktable_s_in_gui_init_s_order() {
        let shown: Vec<&str> = DEFINITION
            .rows()
            .flat_map(|row| row.controls().iter().copied())
            .collect();
        let controls: Vec<&str> = CONTROLS.iter().map(|c| c.id).collect();
        assert_eq!(controls, shown);
        let tabs: Vec<Option<&str>> = DEFINITION.tabs.iter().map(|tab| tab.id).collect();
        assert_eq!(tabs, [Some("settings"), Some("primaries")]);
        let groups: Vec<&str> = DEFINITION
            .rows()
            .filter_map(|row| match row {
                Row::Group { id, .. } => Some(*id),
                _ => None,
            })
            .collect();
        assert_eq!(groups, ["show_curve", "advanced_curve_parameters", "look"]);
        let actions: Vec<&str> = DEFINITION
            .rows()
            .filter_map(|row| match row {
                Row::Action { id } => Some(*id),
                _ => None,
            })
            .collect();
        assert_eq!(
            actions,
            [
                "auto_tune_levels",
                "read_exposure",
                "reset_primaries",
                "set_from_above"
            ]
        );
        assert!(!DEFINITION.layers && !DEFINITION.auto);
    }

    /// The module owns every parameter of `agx`, each with its control of the same name,
    /// and each number control's range and default are its parameter's, shown as its
    /// slider shows them, within an f32 step: 29.462451 % over 100 is not always
    /// darktable's 0.29462451 to the bit, and the binding never writes a default. The
    /// pivot's range is the widest any range gives it, and dynamic range scaling has no
    /// parameter.
    #[test]
    fn it_owns_every_parameter_of_the_operation_each_with_its_control() {
        let schema = AgxOperation.schema();
        let mut owned: Vec<&str> = OWNS.iter().map(|owned| owned.param).collect();
        let mut params: Vec<&str> = schema.params.iter().map(|param| param.name).collect();
        owned.sort_unstable();
        params.sort_unstable();
        assert_eq!(owned, params);
        for param in &params {
            assert!(DEFINITION.control(param).is_some(), "{param}");
        }
        assert!(schema.param(SCALING).is_none());
        for p in &PARAMETERS {
            let control = DEFINITION.control(p.control).unwrap();
            let ControlKind::Number {
                min, max, default, ..
            } = control.kind
            else {
                panic!("{} is a number", p.control);
            };
            let Kind::Float {
                min: low,
                max: high,
                default: untouched,
            } = schema.param(p.param).unwrap().kind
            else {
                panic!("{} is a float", p.param);
            };
            for (shown, darktable) in [(min, low), (max, high), (default, untouched)] {
                let bound = p.param(shown);
                assert!(
                    (bound - darktable).abs() <= f32::EPSILON * darktable.abs(),
                    "{}: {bound} for {darktable}",
                    p.control
                );
            }
        }
        let Kind::Choice { default, choices } = schema.param(BASE_PRIMARIES).unwrap().kind else {
            panic!("the base primaries are a choice");
        };
        assert_eq!(choices, PRIMARIES);
        assert_eq!(DEFINITION.defaults().choice(BASE_PRIMARIES), default);
        for flag in FLAGS {
            let Kind::Bool { default } = schema.param(flag).unwrap().kind else {
                panic!("{flag} is a boolean");
            };
            assert_eq!(DEFINITION.defaults().flag(flag), default, "{flag}");
        }
        // The pivot's default, 0 EV over the default range, is the schema's 10 / 16.5.
        let Kind::Float { default, .. } = schema.param(PIVOT).unwrap().kind else {
            panic!("the pivot is a float");
        };
        assert_eq!(pivot_x(0.0, -10.0, 6.5), default);
    }

    #[test]
    fn at_its_defaults_it_sets_nothing() {
        let image = crate::testing::image();
        let base = Base::new(OPERATIONS, &image);
        assert!(Agx.bind(&ModuleValues::new(), &image, &base).is_empty());
        let defaults: Vec<(&str, ControlValue)> =
            CONTROLS.iter().map(|c| (c.id, c.default())).collect();
        assert!(bind(&defaults).is_empty());
    }

    #[test]
    fn a_percentage_sets_its_parameter_over_100_and_degrees_in_radians() {
        let number = ControlValue::Number;
        let settings = bind(&[
            ("look_saturation", number(140.0)),
            ("curve_contrast_around_pivot", number(2.5)),
            ("red_rotation", number(-15.0)),
            (BASE_PRIMARIES, ControlValue::Choice(5)),
            ("completely_reverse_primaries", ControlValue::Bool(true)),
        ]);
        assert_eq!(
            value(&settings, "look_saturation"),
            Some(Value::Float(140.0 / 100.0))
        );
        assert_eq!(
            value(&settings, "curve_contrast_around_pivot"),
            Some(Value::Float(2.5))
        );
        assert_eq!(
            value(&settings, "red_rotation"),
            Some(Value::Float(-15.0 / RAD_2_DEG))
        );
        assert_eq!(value(&settings, BASE_PRIMARIES), Some(Value::Choice(5)));
        assert_eq!(
            value(&settings, "completely_reverse_primaries"),
            Some(Value::Bool(true))
        );
        assert_eq!(settings.len(), 5);
    }

    /// The pivot is kept in EV: moving the range alone writes `curve_pivot_x` so that the
    /// pivot stays at 0 EV, as darktable's `_update_pivot_x` does, and a pivot outside the
    /// range is kept at its end, as darktable clamps it.
    #[test]
    fn the_pivot_keeps_its_exposure_as_the_range_moves() {
        let number = ControlValue::Number;
        let pivot = |set: &[(&str, ControlValue)]| match value(&bind(set), PIVOT) {
            Some(Value::Float(at)) => at,
            other => panic!("{other:?}"),
        };
        assert_eq!(pivot(&[(WHITE, number(8.0))]), 10.0 / 18.0);
        assert_eq!(pivot(&[(BLACK, number(-6.0))]), 6.0 / 12.5);
        assert_eq!(pivot(&[(PIVOT, number(1.0))]), 11.0 / 16.5);
        assert_eq!(
            pivot(&[
                (PIVOT, number(2.0)),
                (WHITE, number(4.0)),
                (BLACK, number(-8.0))
            ]),
            10.0 / 12.0
        );
        assert_eq!(pivot(&[(PIVOT, number(12.0))]), 1.0);
        assert_eq!(pivot(&[(PIVOT, number(-15.0))]), 0.0);
        // Shown and written back, darktable's pivot lands where it was.
        let at = pivot_x(pivot_ev(0.6529, -9.12, 4.85), -9.12, 4.85);
        assert!((at - 0.6529).abs() <= f32::EPSILON);
        // The range's other controls leave it alone.
        assert_eq!(value(&bind(&[("look_slope", number(1.2))]), PIVOT), None);
    }

    #[test]
    fn the_export_profile_and_dynamic_range_scaling_are_refused() {
        let base = DEFINITION.control(BASE_PRIMARIES).unwrap();
        assert_eq!(
            base.check(&ControlValue::Choice(0)),
            Err(crate::ControlError::Unported {
                control: BASE_PRIMARIES,
                option: "export_profile"
            })
        );
        for working in 1..PRIMARIES.len() as u32 {
            assert_eq!(base.check(&ControlValue::Choice(working)), Ok(()));
        }
        let scaling = DEFINITION.control(SCALING).unwrap();
        assert!(scaling.unported);
        assert_eq!(
            scaling.check(&ControlValue::Number(20.0)),
            Err(crate::ControlError::NotPorted { control: SCALING })
        );
        assert_eq!(scaling.check(&scaling.default()), Ok(()));
    }

    /// A percentage darktable formats with "%": two digits, the factor of 100.
    const fn percent(id: &'static str, hard: (f32, f32)) -> DarktableSlider {
        DarktableSlider::new(id, hard, 2).shown_as(100.0, 0.0)
    }

    /// A rotation: `set_digits(1)`, shown in degrees.
    const fn degrees(id: &'static str) -> DarktableSlider {
        DarktableSlider::new(id, (-ROTATION, ROTATION), 1).shown_as(RAD_2_DEG, 0.0)
    }

    const UNIT: (f32, f32) = (0.0, 1.0);
    const PURITY: (f32, f32) = (0.0, 0.99);

    /// darktable's AgX sliders (`src/iop/agx.c`, the params and `gui_init`), in its
    /// parameters' units: `docs/modules/agx.md`'s slider table. The pivot's is as
    /// darktable draws it at the default range, −10 to +6.5 EV, its factor the range's
    /// width and its offset the black ([`pivot_slider`] at others).
    const DARKTABLE: [DarktableSlider; 32] = [
        DarktableSlider::new(WHITE, (0.1, 20.0), 2).soft(1.0, 10.0),
        DarktableSlider::new(BLACK, (-20.0, -0.1), 2).soft(-20.0, -1.0),
        percent(SCALING, (-0.5, 2.0)).soft(-0.5, 0.5),
        DarktableSlider::new(PIVOT, UNIT, 2).shown_as(16.5, -10.0),
        percent("curve_pivot_y_linear_output", UNIT),
        DarktableSlider::new("curve_contrast_around_pivot", (0.1, 10.0), 2).soft(0.1, 5.0),
        DarktableSlider::new("curve_shoulder_power", (0.0, 10.0), 2).soft(1.0, 5.0),
        DarktableSlider::new("curve_toe_power", (0.0, 10.0), 2).soft(1.0, 5.0),
        percent("curve_linear_ratio_above_pivot", UNIT),
        percent("curve_target_display_white_ratio", (0.2, 1.0)).soft(0.5, 1.0),
        percent("curve_linear_ratio_below_pivot", UNIT),
        percent("curve_target_display_black_ratio", (0.0, 0.15)).soft(0.0, 0.025),
        DarktableSlider::new("curve_gamma", (0.01, 100.0), 2).soft(1.0, 5.0),
        DarktableSlider::new("look_slope", (0.0, 10.0), 2).soft(0.0, 2.0),
        DarktableSlider::new("look_lift", (-1.0, 1.0), 2).soft(-0.5, 0.5),
        DarktableSlider::new("look_brightness", (0.0, 100.0), 2).soft(0.0, 2.0),
        percent("look_saturation", (0.0, 10.0)).soft(0.0, 2.0),
        percent("look_original_hue_mix_ratio", UNIT),
        percent("red_inset", PURITY),
        degrees("red_rotation"),
        percent("green_inset", PURITY),
        degrees("green_rotation"),
        percent("blue_inset", PURITY),
        degrees("blue_rotation"),
        percent("master_outset_ratio", (0.0, 2.0)),
        percent("master_unrotation_ratio", (0.0, 2.0)),
        percent("red_outset", PURITY),
        degrees("red_unrotation"),
        percent("green_outset", PURITY),
        degrees("green_unrotation"),
        percent("blue_outset", PURITY),
        degrees("blue_unrotation"),
    ];

    /// darktable's pivot slider for the input range from `black` to `white`: 0 to 1 shown
    /// times the range's width plus its black, two digits (`_update_pivot_slider_settings`).
    const fn pivot_slider(black: f32, white: f32) -> DarktableSlider {
        DarktableSlider::new(PIVOT, UNIT, 2).shown_as(white - black, black)
    }

    /// Every slider is darktable's at the defaults (`docs/modules/agx.md`, Sliders against
    /// darktable).
    #[test]
    fn its_sliders_are_darktable_s() {
        let differences =
            crate::testing::slider_differences(&Agx, &DEFINITION.defaults(), &DARKTABLE);
        assert!(differences.is_empty(), "{differences:#?}");
        assert_eq!(DARKTABLE[3], pivot_slider(-10.0, 6.5));
        // The factor each shows is the one its binding divides by.
        for p in &PARAMETERS {
            let darktable = DARKTABLE.iter().find(|s| s.id == p.control).unwrap();
            assert_eq!(darktable.factor, p.factor, "{}", p.control);
        }
    }

    /// darktable's pickers on sliders (`agx.c:1823`, `:1832`, `:2043`, `:2052`), each a
    /// denoised area: white and black relative exposure, the pivot kept within the new
    /// range (`:1103-1133`, `:1021-1040`), and the pivot, with its target output too
    /// (`:1171-1220`). Auto tune levels (`:2072`) is still an action.
    #[test]
    fn its_pickers_are_darktable_s() {
        const fn area(
            id: &'static str,
            slider: &'static str,
            writes: &'static [&'static str],
        ) -> DarktablePicker {
            DarktablePicker {
                id,
                slider,
                kind: PickerKind::Area,
                denoise: true,
                output: false,
                writes,
                tools: &[],
            }
        }
        const OUTPUT: &str = "curve_pivot_y_linear_output";
        let darktable = [
            area("white", WHITE, &[WHITE, PIVOT]),
            area("black", BLACK, &[BLACK, PIVOT]),
            area("pivot", PIVOT, &[PIVOT]),
            area("pivot_output", OUTPUT, &[PIVOT, OUTPUT]),
        ];
        assert_eq!(crate::testing::pickers_on_sliders(&DEFINITION), darktable);
    }

    /// The pivot's slider follows the input range as darktable's does (decision 30): it
    /// holds black to white, its track the same, and steps by darktable's step for the
    /// range's width, 0.5 EV over the widest 40 EV, 0.05 over 5, 0.01 over 3 and 0.1 over 12.
    #[test]
    fn the_pivot_s_slider_is_darktable_s_at_every_input_range() {
        for ((black, white), step) in [
            ((-20.0, 20.0), 0.5),
            ((-3.0, 2.0), 0.05),
            ((-1.0, 2.0), 0.01),
            ((-8.0, 4.0), 0.1),
        ] {
            let mut values = DEFINITION.defaults();
            values.set(BLACK, ControlValue::Number(black));
            values.set(WHITE, ControlValue::Number(white));
            let mut darktable = DARKTABLE;
            darktable[3] = pivot_slider(black, white);
            let differences = crate::testing::slider_differences(&Agx, &values, &darktable);
            assert!(
                differences.is_empty(),
                "{black} to {white}: {differences:#?}"
            );
            let range = Agx.range(PIVOT, &values).unwrap();
            assert_eq!(
                (range.min, range.max, range.soft),
                (black, white, (black, white)),
                "{black} to {white}"
            );
            // In f32, as darktable computes it: 5 × 10⁻² is 0.049999997.
            assert!(
                (range.step - step).abs() < 1e-6,
                "{black} to {white}: {}",
                range.step
            );
        }
    }
}
