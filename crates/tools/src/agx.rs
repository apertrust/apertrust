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

//! AgX (`docs/tools/agx.md`): darktable's AgX as the user edits with it, in the
//! first pass (decision 2), the tone mapper of the Adjust tab: darktable's page, its
//! controls in `gui_init`'s order under its labels.
//!
//! - **Settings:** the input exposure range, white and black relative exposure and
//!   dynamic range scaling, with Auto tune levels and Read exposure on one line; the basic
//!   curve
//!   parameters, the pivot, contrast and the shoulder's and toe's powers; then darktable's
//!   collapsible sections, Show curve, Advanced curve parameters and Look.
//! - **Primaries:** the base primaries and Disable adjustments; while adjustments are on,
//!   Reset primaries, each primary's attenuation and rotation before tone mapping, Reverse
//!   all with Set from above at its end, and while Reverse all is off the purity boosts and
//!   reverse rotations after it.
//!
//! Each control is one `agx` parameter, its id the parameter's name, shown as darktable's
//! slider shows it: its percentages as percentages, so the binding divides them by 100,
//! its rotations in degrees, and the pivot in EV against the range, as darktable's slider
//! shows `curve_pivot_x` with the range's width as its factor and its black as its offset.
//! The tool stores the pivot in EV and writes `curve_pivot_x` from it and the range, so
//! moving the range keeps the pivot's exposure, as darktable's `_update_pivot_x` does. Its
//! slider is darktable's for the range (decision 30): it holds black to white relative
//! exposure, its track the same, and steps by darktable's step for the range's width; a
//! pivot the range's move leaves outside is clamped to the range's end and stored in the
//! same change, as `_update_pivot_x` stores it.
//!
//! It drives `agx`, which every photo runs, so each control starts at the operation's
//! default (rule 4 of `docs/tools/README.md`), darktable's scene-referred defaults, and
//! the binding sets a parameter only while its control is off it, the pivot's while it or
//! the range is. Dynamic range scaling, which only darktable's pickers and Read exposure
//! read, is stored and binds nothing: the pickers' margin. The base primaries' export
//! profile is not ported: it is drawn disabled and refused.
//!
//! darktable's five pickers pick as its `color_picker_apply` does ([`Agx::pick`]): the
//! quads on white and black relative exposure, Auto tune levels setting both, a row of
//! its own with its picker at the end, and the quads on the pivot and its target output.
//! Read exposure sets the range from Exposure's published effective exposure; Reset
//! primaries opens a menu of darktable's three primaries to start from, Blender-like, Smooth
//! and Unmodified, each one change; Set from above copies the primaries before tone mapping
//! to after it ([`Agx::action`]), unavailable while Reverse all or Disable adjustments is on
//! ([`Agx::available`]). Show curve is empty until a read-only graph row
//! exists. darktable's presets wait for the presets menu's built-in presets (decision 106).

#![allow(
    clippy::excessive_precision,
    reason = "defaults are copied digit for digit from darktable"
)]

use apertrust_color::WorkingProfile;
use apertrust_color::agx_curve::{Curve, CurveSettings, EPSILON};
use apertrust_plugin::{ImageContext, Value};

use crate::about::{Fact, Facts, Purpose};
use crate::definition::{
    ActionEnd, ButtonIcon, ChoiceStyle, Control, ControlKind, End, Face, Group, GroupKind,
    LineStart, Picker, PickerKind, ProcessingDefinition, Published, Range, Reset, Row, Slider,
    Space, Tab, ToolDefinition, Unit, slider_step,
};
use crate::pick::{PickKeys, Picked, Reading, Shown};
use crate::{
    Base, ControlValue, Controls, Owned, ProcessingTool, PublishedValues, Setting, ToolValues,
};

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
/// range they compute, stored in the edit as darktable stores it. No render reads it, so
/// the operation has no parameter for it and the binding sets none. Moving it writes
/// nothing into the range, where darktable's `gui_changed` rescales it
/// (`docs/tools/agx.md`, Deliberate differences).
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

/// Primaries to start from, as darktable's parameters: the base primaries by their option,
/// each primary's attenuation and rotation before tone mapping, the two masters, and each
/// purity boost and reverse rotation after it.
struct Primaries {
    base: &'static str,
    insets: [f32; 3],
    rotations: [f32; 3],
    master_outset: f32,
    master_unrotation: f32,
    outsets: [f32; 3],
    unrotations: [f32; 3],
}

/// What Read exposure reads: Exposure's effective exposure (decision 107).
const EXPOSURE_READ: Published = Published {
    tool: "exposure",
    id: "effective_exposure",
};

/// Reset primaries' menu, in darktable's order (`agx.c:2100-2121`).
const RESET_PRIMARIES: [&str; 3] = [
    "primaries_blender_like",
    "primaries_smooth",
    "primaries_unmodified",
];

/// darktable's `deg2radf`, in f32 as it computes it: `deg · π / 180`.
fn deg2radf(deg: f32) -> f32 {
    deg * std::f32::consts::PI / 180.0
}

/// The primaries the menu item `id` starts from, darktable's `_set_blenderlike_primaries`,
/// `_set_smooth_primaries` and `_set_unmodified_primaries` (`agx.c:317-397`).
fn primaries(id: &str) -> Option<Primaries> {
    match id {
        // Blender's AgX: the scene-referred defaults, the reverse rotations not applied.
        "primaries_blender_like" => Some(Primaries {
            base: "rec2020",
            insets: INSETS,
            rotations: ROTATIONS,
            master_outset: 1.0,
            master_unrotation: 0.0,
            outsets: OUTSETS,
            unrotations: ROTATIONS,
        }),
        // Sigmoid's smooth primaries over the working profile, the purity not restored.
        "primaries_smooth" => {
            let rotations = [deg2radf(2.0), deg2radf(-1.0), deg2radf(-3.0)];
            let insets = [0.1, 0.1, 0.15];
            Some(Primaries {
                base: "working_profile",
                insets,
                rotations,
                master_outset: 0.0,
                master_unrotation: 1.0,
                outsets: insets,
                unrotations: rotations,
            })
        }
        "primaries_unmodified" => Some(Primaries {
            base: "rec2020",
            insets: [0.0; 3],
            rotations: [0.0; 3],
            master_outset: 1.0,
            master_unrotation: 1.0,
            outsets: [0.0; 3],
            unrotations: [0.0; 3],
        }),
        _ => None,
    }
}

/// What a control shows for darktable's parameter `param`, the control showing its
/// parameter times `factor`: the f32 nearest `param × factor` whose `/ factor` is `param`
/// to the bit, where one within eight f32 steps is, else `param × factor`, one f32 step
/// from it at most (A4's bound, `docs/tools/agx.md`).
fn shown_for(param: f32, factor: f32) -> f32 {
    let near = param * factor;
    let (mut up, mut down) = (near, near);
    for _ in 0..=8 {
        for shown in [up, down] {
            if shown / factor == param {
                return shown;
            }
        }
        (up, down) = (up.next_up(), down.next_down());
    }
    near
}

/// darktable's hard ranges are the controls', in the units its sliders show; its soft
/// ranges are the rows' (`Slider::soft`). Every default is the operation's, darktable's
/// scene-referred default (`reload_defaults`).
static CONTROLS: [Control; 36] = [
    number(WHITE, (0.1, 20.0), 1.0, 0.1, 6.5, Some(Unit::Ev)),
    number(BLACK, (-20.0, -0.1), 1.0, 0.1, -10.0, Some(Unit::Ev)),
    // Read by the pickers alone, so it binds no parameter (`Agx::bind`).
    number(SCALING, (-0.5, 2.0), 100.0, 0.5, 0.1, Some(Unit::Percent)),
    // Every pivot any range gives: its range for the tool's values is the input range,
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
    Row::Slider(Slider::new(control).end(End::Picker(picker)).soft(min, max))
}

/// A power over darktable's soft range, 1 to 5, with its warning quad `warning`.
const fn warned(control: &'static str, warning: &'static str) -> Row {
    Row::Slider(
        Slider::new(control)
            .soft(1.0, 5.0)
            .end(End::Warning(warning)),
    )
}

/// Pivot target output, which the `pivot_output` picker writes with the pivot.
const OUTPUT: &str = "curve_pivot_y_linear_output";

/// darktable's pickers (`DT_COLOR_PICKER_AREA | DT_COLOR_PICKER_DENOISE`), in `gui_init`'s
/// order: white or black relative exposure from the area's largest maximum or smallest
/// minimum, and Auto tune levels' both at once, the pivot kept within the new range; the
/// pivot from the mean's luminance, and with Pivot target output also the output that
/// keeps the curve's linear output there ([`Agx::pick`]).
static PICKERS: [Picker; 5] = [
    picker("white", &[WHITE, PIVOT]),
    picker("black", &[BLACK, PIVOT]),
    picker("range", &[WHITE, BLACK, PIVOT]),
    picker("pivot", &[PIVOT]),
    picker("pivot_output", &[PIVOT, OUTPUT]),
];

/// A denoised area picker of the tool's input writing `writes`.
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
        settings: &[],
        words_by: None,
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

/// darktable's Settings page: the exposure box and the basic curve box under their section
/// labels, then its three collapsible sections, as `gui_init` builds them without the
/// optional curve tab.
static SETTINGS: [Row; 5] = [
    Row::Group(Group {
        id: "input_exposure_range",
        kind: GroupKind::Plain,
        rows: &[
            picked(WHITE, "white", (1.0, 10.0)),
            picked(BLACK, "black", (-20.0, -1.0)),
            // darktable sets only its soft maximum, 0.5; its soft minimum is its hard one.
            soft(SCALING, -50.0, 50.0),
            // darktable's labelled line with its picker setting both exposures, then its
            // camera button setting them from Exposure's effective exposure.
            Row::Line {
                start: LineStart::Label("auto_tune_levels"),
                readout: None,
                end: &[
                    End::Picker("range"),
                    End::Action(ActionEnd {
                        id: "read_exposure",
                        face: Face::Icon(ButtonIcon::Camera),
                        menu: &[],
                        reads: &[EXPOSURE_READ],
                        unported: false,
                    }),
                ],
            },
        ],
    }),
    Row::Group(Group {
        id: "basic_curve_parameters",
        kind: GroupKind::Plain,
        rows: &[
            // The pivot's track spans the input range, here its default (`Agx::range`).
            picked(PIVOT, "pivot", (-10.0, 6.5)),
            Row::Slider(Slider::new(OUTPUT).end(End::Picker("pivot_output"))),
            soft("curve_contrast_around_pivot", 0.1, 5.0),
            // darktable's warning quads, shown while the curve has lost its S shape.
            warned("curve_shoulder_power", "shoulder"),
            warned("curve_toe_power", "toe"),
        ],
    }),
    // darktable's read-only graph of the curve, once a definition can name one.
    Row::Group(Group {
        id: "show_curve",
        kind: GroupKind::Collapsible,
        rows: &[],
    }),
    Row::Group(Group {
        id: "advanced_curve_parameters",
        kind: GroupKind::Collapsible,
        rows: &[
            slider("curve_linear_ratio_above_pivot"),
            soft("curve_target_display_white_ratio", 50.0, 100.0),
            slider("curve_linear_ratio_below_pivot"),
            soft("curve_target_display_black_ratio", 0.0, 2.5),
            Row::Toggle {
                control: "auto_gamma",
                end: &[],
            },
            // darktable hides the gamma while it keeps the pivot on the diagonal.
            Row::When {
                control: "auto_gamma",
                is: &["false"],
                rows: &[soft("curve_gamma", 1.0, 5.0)],
            },
        ],
    }),
    Row::Group(Group {
        id: "look",
        kind: GroupKind::Collapsible,
        rows: &[
            soft("look_slope", 0.0, 2.0),
            soft("look_lift", -0.5, 0.5),
            soft("look_brightness", 0.0, 2.0),
            soft("look_saturation", 0.0, 200.0),
            slider("look_original_hue_mix_ratio"),
        ],
    }),
];

/// darktable's Primaries page: the base and Disable adjustments, then, while adjustments
/// are on, the rest, the purity boosts and reverse rotations while Reverse all is off.
static PRIMARIES_PAGE: [Row; 3] = [
    Row::Choice {
        control: BASE_PRIMARIES,
        style: ChoiceStyle::List,
        end: &[],
    },
    Row::Toggle {
        control: "disable_primaries_adjustments",
        end: &[],
    },
    Row::When {
        control: "disable_primaries_adjustments",
        is: &["false"],
        rows: &[
            // darktable's labelled line with its menu of three primaries to start from:
            // Blender-like, Smooth and Unmodified.
            Row::Line {
                start: LineStart::Label("reset_primaries"),
                readout: None,
                end: &[End::Action(ActionEnd {
                    id: "reset_primaries",
                    face: Face::Icon(ButtonIcon::PrimariesMenu),
                    menu: &RESET_PRIMARIES,
                    reads: &[],
                    unported: false,
                })],
            },
            Row::Group(Group {
                id: "before_tone_mapping",
                kind: GroupKind::Plain,
                rows: &[
                    purity_row("red_inset"),
                    rotation_row("red_rotation"),
                    purity_row("green_inset"),
                    rotation_row("green_rotation"),
                    purity_row("blue_inset"),
                    rotation_row("blue_rotation"),
                ],
            }),
            Row::Group(Group {
                id: "after_tone_mapping",
                kind: GroupKind::Plain,
                rows: &[
                    // darktable's button on Reverse all's line, insensitive while it is on.
                    Row::Toggle {
                        control: "completely_reverse_primaries",
                        end: &[End::Action(ActionEnd {
                            id: "set_from_above",
                            face: Face::Text,
                            menu: &[],
                            reads: &[],
                            unported: false,
                        })],
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
            }),
        ],
    },
];

static DEFINITION: ProcessingDefinition = ProcessingDefinition {
    tool: ToolDefinition {
        id: AGX,
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
        reset: Reset::Tool,
        presets: true,
        open_at_first: false,
        menus: &[],
        toolbar: &[],
        cursors: &[],
        lives: crate::Lives::ToolsPanel,
    },
    version: 1,
    // Background only: the tone mapper's parameters are global (decision 33).
    layers: false,
    auto: false,
    switched: false,
    controls: &CONTROLS,
    preset_rows: &[],
    views: &[],
    pickers: &PICKERS,
    // darktable's `description()` in src/iop/agx.c at the reference release.
    facts: Some(Facts {
        purpose: Purpose::CorrectiveAndCreative,
        input: Fact::LinearRgbSceneReferred,
        process: Fact::NonLinearRgb,
        output: Fact::LinearRgbDisplayReferred,
    }),
    // src/iop/agx.c's flags: `IOP_FLAGS_INCLUDE_IN_STYLES`, not `_UNSAFE_COPY`.
    copy_safe: true,
    in_styles: true,
    publishes: &[],
};

/// Every parameter of `agx`: the tool is darktable's and owns them all, each with its
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

impl ProcessingTool for Agx {
    fn definition(&self) -> &'static ProcessingDefinition {
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

    /// Set from above can run only while the primaries after tone mapping are shown and
    /// act, darktable's `post_curve_primaries_available` (`agx.c:1744-1746`): neither
    /// Reverse all nor Disable adjustments on. Every control can change something.
    fn available(&self, id: &str, values: &Controls, _: &ImageContext) -> bool {
        match id {
            "set_from_above" => {
                !values.flag("completely_reverse_primaries")
                    && !values.flag("disable_primaries_adjustments")
            }
            _ => true,
        }
    }

    /// darktable's warning quads on Toe power and Shoulder power (`agx.c:1727-1735`):
    /// **`toe`** while the toe, **`shoulder`** while the shoulder, has lost its S shape and
    /// the power fallback takes over, darktable's `need_convex_toe` and
    /// `need_concave_shoulder` (`agx.c:903`, `:948`), from the curve the operation renders
    /// ([`Curve::new`]). Shown whatever darktable's preference to hide them says (decision
    /// 107).
    fn warning(&self, id: &str, values: &Controls, _: &ImageContext) -> bool {
        let curve = || Curve::new(&curve_settings(values));
        match id {
            "toe" => curve().toe.fallback,
            "shoulder" => curve().shoulder.fallback,
            _ => false,
        }
    }

    /// darktable's buttons on the values shown:
    ///
    /// - **`read_exposure`,** `_adjust_relative_exposure_from_exposure_params`
    ///   (`agx.c:1042-1052`), with `e` Exposure's effective exposure, read, and `s` Dynamic
    ///   range scaling over 100: black `(−8 + 0.5 e) · (1 + s)` held to −20 to −0.1 EV in
    ///   f32, and white `(4 + 0.8 e) · (1 + s)` held to 0.1 to 20 EV in f64, as darktable's
    ///   `0.8` is a double, then taken to f32. The pivot keeps its exposure, clamped into the
    ///   new range, as a pick keeps it (`keep_pivot`). Curve y gamma is never written
    ///   (decision 63). With no effective exposure read, it writes nothing;
    /// - **Reset primaries' menu,** `primaries_blender_like`, `primaries_smooth` and
    ///   `primaries_unmodified` (`primaries`): adjustments on, Reverse all off, the base
    ///   primaries, and every primaries control as darktable sets it, in the units shown,
    ///   each binding to darktable's parameter to the bit where a value of the control
    ///   reaches it, else one f32 step from it (`shown_for`);
    /// - **`set_from_above`,** `_set_post_curve_primaries_from_pre_callback`
    ///   (`agx.c:2123-2140`): both masters 100 %, each purity boost its primary's
    ///   attenuation and each reverse rotation its rotation, as shown, so each parameter is
    ///   darktable's to the bit: a boost and its attenuation, a reverse rotation and its
    ///   rotation, share a factor.
    fn action(
        &self,
        action: &str,
        values: &Controls,
        read: &PublishedValues,
        _: &ImageContext,
    ) -> Option<Controls> {
        let mut writes = Controls::new();
        match action {
            "read_exposure" => {
                let Some(e) = read.get(&EXPOSURE_READ) else {
                    return Some(writes);
                };
                let scaling = 1.0 + values.number(SCALING) / 100.0;
                let black = ((-8.0 + 0.5 * e) * scaling).clamp(-20.0, -0.1);
                let white = ((4.0 + 0.8 * f64::from(e)) * f64::from(scaling))
                    .clamp(f64::from(0.1f32), 20.0) as f32;
                writes.set(BLACK, ControlValue::Number(black));
                writes.set(WHITE, ControlValue::Number(white));
                keep_pivot(&mut writes, values.number(PIVOT), black, white);
            }
            "set_from_above" => {
                for master in ["master_outset_ratio", "master_unrotation_ratio"] {
                    writes.set(master, ControlValue::Number(100.0));
                }
                for colour in ["red", "green", "blue"] {
                    let [outset, unrotation, inset, rotation] =
                        ["outset", "unrotation", "inset", "rotation"]
                            .map(|part| control_of(colour, part));
                    writes.set(outset, ControlValue::Number(values.number(inset)));
                    writes.set(unrotation, ControlValue::Number(values.number(rotation)));
                }
            }
            id => {
                let primaries = primaries(id)?;
                let base = PRIMARIES
                    .iter()
                    .position(|option| *option == primaries.base)?;
                writes.set(BASE_PRIMARIES, ControlValue::Choice(base as u32));
                for flag in [
                    "disable_primaries_adjustments",
                    "completely_reverse_primaries",
                ] {
                    writes.set(flag, ControlValue::Bool(false));
                }
                let mut number = |id: &'static str, param: f32| {
                    let factor = PARAMETERS.iter().find(|p| p.control == id)?.factor;
                    writes.set(id, ControlValue::Number(shown_for(param, factor)));
                    Some(())
                };
                number("master_outset_ratio", primaries.master_outset)?;
                number("master_unrotation_ratio", primaries.master_unrotation)?;
                for (at, colour) in ["red", "green", "blue"].into_iter().enumerate() {
                    for (part, values) in [
                        ("inset", primaries.insets),
                        ("rotation", primaries.rotations),
                        ("outset", primaries.outsets),
                        ("unrotation", primaries.unrotations),
                    ] {
                        number(control_of(colour, part), values[at])?;
                    }
                }
            }
        }
        Some(writes)
    }

    /// Each number off its default sets its parameter, the value shown over its factor;
    /// the pivot sets `curve_pivot_x` while it or the range is off its default, from all
    /// three; each toggle and the base primaries set theirs. None, at the defaults, sets
    /// anything. Dynamic range scaling, which only the pickers read, sets nothing.
    fn bind(&self, values: &ToolValues, _image: &ImageContext, _base: &Base) -> Vec<Setting> {
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

    /// darktable's `color_picker_apply` (`agx.c:2657-2690`), on the area at the
    /// operation's input, working RGB, with `s` Dynamic range scaling over 100:
    ///
    /// - **`black`** `log2(max(ε, m) / 0.18) · (1 + s)` of `m` the minimum's smallest
    ///   channel, held to −20 to −0.1 EV (`_apply_auto_black_exposure`); **`white`** the
    ///   same of the maximum's largest, held to 0.1 to 20 EV (`_apply_auto_white_exposure`);
    ///   **`range`** both (`_apply_auto_tune_exposure`). The pivot keeps its exposure,
    ///   clamped into the new range, as `_update_pivot_x` keeps it; it is stored in EV, so
    ///   it is written only where the clamp moves it.
    /// - **`pivot`** at `log2(max(ε, Y) / 0.18)` of the mean's luminance through the working
    ///   profile, held within the range (`_apply_auto_pivot_x`); **`pivot_output`** the
    ///   same, and Pivot target output the curve of the values before the pick gives at the
    ///   new pivot's place, raised to the curve's gamma (`_apply_auto_pivot_xy`): Curve y
    ///   gamma, or while Keep the pivot on the diagonal is on the gamma the curve computes,
    ///   which darktable's `gui_changed` keeps in `p->curve_gamma`. The curve's linear
    ///   output there, before the look, stays what it was; a grey of the area's luminance
    ///   keeps its brightness in the frame while Keep the pivot on the diagonal is off or
    ///   the look is at its defaults, since with it on the gamma moves with the pivot.
    ///
    /// Curve y gamma is never written, where darktable's writes the gamma it computes while
    /// Keep the pivot on the diagonal is on (`docs/tools/agx.md`, Deliberate differences;
    /// decision 63). Nothing is shown.
    fn pick(
        &self,
        picker: &str,
        reading: &Reading,
        values: &Controls,
        _image: &ImageContext,
        _settings: &[(&str, ControlValue)],
        _channel: Option<&str>,
        _last: Option<&Reading>,
        _keys: PickKeys,
    ) -> Option<Picked> {
        let (black, white, pivot) = (
            values.number(BLACK),
            values.number(WHITE),
            values.number(PIVOT),
        );
        let scaling = 1.0 + values.number(SCALING) / 100.0;
        let mut writes = Controls::new();
        match picker {
            "black" | "white" | "range" => {
                let [r, g, b, _] = reading.input.min;
                let picked_black = (relative_ev(r.min(g).min(b)) * scaling).clamp(-20.0, -0.1);
                let [r, g, b, _] = reading.input.max;
                let picked_white = (relative_ev(r.max(g).max(b)) * scaling).clamp(0.1, 20.0);
                let (mut black, mut white) = (black, white);
                if picker != "white" {
                    black = picked_black;
                    writes.set(BLACK, ControlValue::Number(black));
                }
                if picker != "black" {
                    white = picked_white;
                    writes.set(WHITE, ControlValue::Number(white));
                }
                keep_pivot(&mut writes, pivot, black, white);
            }
            "pivot" | "pivot_output" => {
                let [r, g, b, _] = reading.input.mean;
                let luminance = WorkingProfile::linear_rec2020().rgb_to_xyz_d50[1];
                let y = luminance[0] * r + luminance[1] * g + luminance[2] * b;
                let picked = relative_ev(y).clamp(black, white);
                writes.set(PIVOT, ControlValue::Number(picked));
                if picker == "pivot_output" {
                    // darktable raises to `p->curve_gamma`, which its `gui_changed` keeps at
                    // the gamma the curve computes while Keep the pivot on the diagonal is
                    // on; Curve y gamma here keeps its value, so the curve's own is taken.
                    let curve = Curve::new(&curve_settings(values));
                    let at = curve.eval(pivot_x(picked, black, white));
                    let output = at.powf(curve.gamma);
                    writes.set(OUTPUT, ControlValue::Number(output * 100.0));
                }
            }
            _ => return None,
        }
        Some(Picked {
            writes,
            shown: Shown::default(),
        })
    }
}

/// The pivot at `pivot` EV kept at its exposure within the new range `black` to `white`,
/// darktable's `_update_pivot_x`, which keeps the pivot's exposure clamped into the range:
/// the pivot is stored in EV, so it is written into `writes` only where the clamp moves it.
fn keep_pivot(writes: &mut Controls, pivot: f32, black: f32, white: f32) {
    let kept = pivot.clamp(black, white);
    if kept != pivot {
        writes.set(PIVOT, ControlValue::Number(kept));
    }
}

/// The control of `colour`'s primary `part`: `red_inset`, `blue_unrotation`.
fn control_of(colour: &str, part: &str) -> &'static str {
    let id = format!("{colour}_{part}");
    let found = CONTROLS.iter().find(|control| control.id == id);
    found.expect("a primary's control").id
}

/// darktable's exposure of `v` relative to mid-grey, floored at its `_epsilon` first:
/// `log2(max(ε, v) / 0.18)` (`agx.c:1110`, `:1126`, `:1178`).
fn relative_ev(v: f32) -> f32 {
    (EPSILON.max(v) / 0.18).log2()
}

/// What AgX's curve is made from for `values`, every control: the parameters the binding
/// writes, the pivot's place in the range and the percentages over 100, as `bind` maps
/// them, so the curve is the one the operation renders.
fn curve_settings(values: &Controls) -> CurveSettings {
    let param = |control: &str| {
        let p = PARAMETERS.iter().find(|p| p.control == control);
        p.expect("a number control of the curve")
            .param(values.number(control))
    };
    let (black, white) = (values.number(BLACK), values.number(WHITE));
    CurveSettings {
        range_black_relative_ev: black,
        range_white_relative_ev: white,
        curve_pivot_x: pivot_x(values.number(PIVOT), black, white),
        curve_pivot_y_linear_output: param(OUTPUT),
        curve_contrast_around_pivot: param("curve_contrast_around_pivot"),
        curve_linear_ratio_below_pivot: param("curve_linear_ratio_below_pivot"),
        curve_linear_ratio_above_pivot: param("curve_linear_ratio_above_pivot"),
        curve_toe_power: param("curve_toe_power"),
        curve_shoulder_power: param("curve_shoulder_power"),
        curve_gamma: param("curve_gamma"),
        auto_gamma: values.flag("auto_gamma"),
        curve_target_display_black_ratio: param("curve_target_display_black_ratio"),
        curve_target_display_white_ratio: param("curve_target_display_white_ratio"),
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
        let mut values = ToolValues::new();
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
        let tabs: Vec<Option<&str>> = DEFINITION.tool.tabs.iter().map(|tab| tab.id).collect();
        assert_eq!(tabs, [Some("settings"), Some("primaries")]);
        let groups: Vec<(&str, GroupKind)> = DEFINITION
            .rows()
            .filter_map(|row| match row {
                Row::Group(group) => Some((group.id, group.kind)),
                _ => None,
            })
            .collect();
        let (plain, collapsible) = (GroupKind::Plain, GroupKind::Collapsible);
        assert_eq!(
            groups,
            [
                ("input_exposure_range", plain),
                ("basic_curve_parameters", plain),
                ("show_curve", collapsible),
                ("advanced_curve_parameters", collapsible),
                ("look", collapsible),
                ("before_tone_mapping", plain),
                ("after_tone_mapping", plain),
            ]
        );
        let actions: Vec<(&str, bool)> = DEFINITION
            .action_ends()
            .map(|end| (end.id, end.unported))
            .collect();
        assert_eq!(
            actions,
            [
                ("read_exposure", false),
                ("reset_primaries", false),
                ("set_from_above", false)
            ]
        );
        assert_eq!(
            DEFINITION.own_actions(),
            [
                ("read_exposure", false),
                ("primaries_blender_like", false),
                ("primaries_smooth", false),
                ("primaries_unmodified", false),
                ("set_from_above", false)
            ]
        );
        assert!(!DEFINITION.layers && !DEFINITION.auto);
    }

    /// The tool owns every parameter of `agx`, each with its control of the same name,
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
        assert!(Agx.bind(&ToolValues::new(), &image, &base).is_empty());
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
    fn the_export_profile_is_refused() {
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
    }

    /// Dynamic range scaling is ported over darktable's whole range and binds nothing: no
    /// render reads it.
    #[test]
    fn dynamic_range_scaling_is_ported_and_binds_nothing() {
        let scaling = DEFINITION.control(SCALING).unwrap();
        assert!(!scaling.unported);
        for shown in [-50.0, 20.0, 200.0] {
            assert_eq!(scaling.check(&ControlValue::Number(shown)), Ok(()));
            assert!(bind(&[(SCALING, ControlValue::Number(shown))]).is_empty());
        }
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
    /// parameters' units: `docs/tools/agx.md`'s slider table. The pivot's is as
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

    /// Every slider is darktable's at the defaults (`docs/tools/agx.md`, Sliders against
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

    /// darktable's pickers (`agx.c:1823`, `:1832`, `:2043`, `:2052`, `:2072`), each a
    /// denoised area: white and black relative exposure and Auto tune levels' both, the
    /// pivot kept within the new range (`:1103-1160`, `:1021-1040`), and the pivot, with
    /// its target output too (`:1171-1220`). Auto tune levels' is on a line of pickers of
    /// its own under its label, as darktable's line has it (`:2068-2076`), and Read
    /// exposure, on that line in darktable, is still an action of its own.
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
                settings: &[],
            }
        }
        let darktable = [
            area("white", WHITE, &[WHITE, PIVOT]),
            area("black", BLACK, &[BLACK, PIVOT]),
            area("range", "", &[WHITE, BLACK, PIVOT]),
            area("pivot", PIVOT, &[PIVOT]),
            area("pivot_output", OUTPUT, &[PIVOT, OUTPUT]),
        ];
        assert_eq!(crate::testing::pickers_on_sliders(&DEFINITION), darktable);
        // Auto tune levels' line: its label, its picker, then Read exposure's camera, as
        // darktable's line has them.
        let line = DEFINITION
            .rows()
            .find(|row| matches!(row, Row::Line { .. }))
            .unwrap();
        assert!(
            matches!(
                line,
                Row::Line {
                    start: LineStart::Label("auto_tune_levels"),
                    readout: None,
                    end: [
                        End::Picker("range"),
                        End::Action(ActionEnd {
                            id: "read_exposure",
                            face: Face::Icon(ButtonIcon::Camera),
                            menu: [],
                            reads: [EXPOSURE_READ],
                            unported: false,
                        }),
                    ],
                }
            ),
            "{line:?}"
        );
    }

    /// Set from above writes darktable's `_set_post_curve_primaries_from_pre_callback`: both
    /// masters 100 %, the boosts the attenuations and the reverse rotations the rotations,
    /// each bound to darktable's parameter, the copy of the one before it, to the bit.
    #[test]
    fn set_from_above_copies_the_primaries_before_tone_mapping_after_it() {
        let image = crate::testing::image();
        let mut values = DEFINITION.defaults();
        let before = [
            ("red_inset", 12.0),
            ("green_inset", 33.0),
            ("blue_inset", 7.5),
            ("red_rotation", -4.3),
            ("green_rotation", 2.0),
            ("blue_rotation", 29.9),
        ];
        for (id, value) in before {
            values.set(id, ControlValue::Number(value));
        }
        let writes = Agx
            .action("set_from_above", &values, &PublishedValues::new(), &image)
            .unwrap();
        let written: Vec<(&str, f32)> = writes
            .iter()
            .map(|(id, value)| match value {
                ControlValue::Number(number) => (id, *number),
                other => panic!("{id}: {other:?}"),
            })
            .collect();
        assert_eq!(
            written,
            [
                ("blue_outset", 7.5),
                ("blue_unrotation", 29.9),
                ("green_outset", 33.0),
                ("green_unrotation", 2.0),
                ("master_outset_ratio", 100.0),
                ("master_unrotation_ratio", 100.0),
                ("red_outset", 12.0),
                ("red_unrotation", -4.3),
            ]
        );
        // Each copy binds to its original's parameter to the bit, as darktable copies it.
        let mut set: Vec<(&str, ControlValue)> = before
            .iter()
            .map(|&(id, value)| (id, ControlValue::Number(value)))
            .collect();
        set.extend(writes.iter().map(|(id, value)| (id, value.clone())));
        let settings = bind(&set);
        for colour in ["red", "green", "blue"] {
            for (after, before) in [("outset", "inset"), ("unrotation", "rotation")] {
                let after = value(&settings, &format!("{colour}_{after}"));
                assert_eq!(after, value(&settings, &format!("{colour}_{before}")));
            }
        }
        assert_eq!(value(&settings, "master_outset_ratio"), None);
        assert_eq!(
            value(&settings, "master_unrotation_ratio"),
            Some(Value::Float(1.0))
        );
        assert!(
            Agx.action("no_such_action", &values, &PublishedValues::new(), &image)
                .is_none()
        );
    }

    /// Each of Reset primaries' menu items writes darktable's primaries: adjustments on,
    /// Reverse all off, its base, and every primaries control, each binding to darktable's
    /// f32 parameter to the bit where some value of the control reaches it, else one f32
    /// step from it, A4's bound; and they leave the rest of the tool as it was.
    #[test]
    fn reset_primaries_menu_writes_darktable_s_primaries() {
        let image = crate::testing::image();
        // darktable's parameters for each, `agx.c:317-397`.
        let rad = |deg: f32| deg * std::f32::consts::PI / 180.0;
        let smooth_rotations = [rad(2.0), rad(-1.0), rad(-3.0)];
        // Each: its id, the base primaries' option, the insets, the rotations, the master
        // outset and unrotation, the outsets and the unrotations.
        type Case = (
            &'static str,
            u32,
            [f32; 3],
            [f32; 3],
            f32,
            f32,
            [f32; 3],
            [f32; 3],
        );
        let cases: [Case; 3] = [
            (
                "primaries_blender_like",
                2,
                [0.29462451, 0.25861925, 0.14641371],
                [0.03540329, -0.02108586, -0.06305724],
                1.0,
                0.0,
                [0.290776401758, 0.263155400753, 0.045810721815],
                [0.03540329, -0.02108586, -0.06305724],
            ),
            (
                "primaries_smooth",
                1,
                [0.1, 0.1, 0.15],
                smooth_rotations,
                0.0,
                1.0,
                [0.1, 0.1, 0.15],
                smooth_rotations,
            ),
            (
                "primaries_unmodified",
                2,
                [0.0; 3],
                [0.0; 3],
                1.0,
                1.0,
                [0.0; 3],
                [0.0; 3],
            ),
        ];
        let mut values = DEFINITION.defaults();
        values.set("disable_primaries_adjustments", ControlValue::Bool(true));
        values.set("completely_reverse_primaries", ControlValue::Bool(true));
        values.set("look_saturation", ControlValue::Number(150.0));
        for (id, base, insets, rotations, outset, unrotation, outsets, unrotations) in cases {
            let writes = Agx
                .action(id, &values, &PublishedValues::new(), &image)
                .unwrap();
            assert_eq!(writes.choice(BASE_PRIMARIES), base, "{id}");
            assert!(!writes.flag("disable_primaries_adjustments"), "{id}");
            assert!(!writes.flag("completely_reverse_primaries"), "{id}");
            assert_eq!(writes.get("look_saturation"), None, "{id}");
            let mut expected = vec![
                ("master_outset_ratio", outset),
                ("master_unrotation_ratio", unrotation),
            ];
            for (at, colour) in ["red", "green", "blue"].into_iter().enumerate() {
                expected.push((control_of(colour, "inset"), insets[at]));
                expected.push((control_of(colour, "rotation"), rotations[at]));
                expected.push((control_of(colour, "outset"), outsets[at]));
                expected.push((control_of(colour, "unrotation"), unrotations[at]));
            }
            // 3 choices and toggles and 14 numbers.
            assert_eq!(writes.iter().count(), 3 + expected.len(), "{id}");
            for (control, darktable) in expected {
                let p = PARAMETERS.iter().find(|p| p.control == control).unwrap();
                let shown = writes.number(control);
                let bound = p.param(shown);
                // Whether some value of the control binds to darktable's exactly.
                let mut reached = false;
                let (mut up, mut down) = (darktable * p.factor, darktable * p.factor);
                for _ in 0..64 {
                    reached |= up / p.factor == darktable || down / p.factor == darktable;
                    (up, down) = (up.next_up(), down.next_down());
                }
                if reached {
                    assert_eq!(bound.to_bits(), darktable.to_bits(), "{id}: {control}");
                } else {
                    let step = (darktable.next_up() - darktable).abs();
                    assert!((bound - darktable).abs() <= step, "{id}: {control}");
                }
                let range = DEFINITION.range(control).unwrap();
                assert!(range.contains(shown), "{id}: {control} {shown}");
            }
        }
        assert!(
            Agx.action(
                "primaries_nonexistent",
                &values,
                &PublishedValues::new(),
                &image
            )
            .is_none()
        );
    }

    /// Read exposure is darktable's `_adjust_relative_exposure_from_exposure_params`,
    /// restated in f64: black `(−8 + 0.5 e) · (1 + s)` and white `(4 + 0.8 e) · (1 + s)`,
    /// each held to its range, at effective exposures of −2, 0, +0.7 and +3 EV, and at
    /// the ends, and Dynamic range scaling at −50 %, +10 % and +200 %; the pivot kept at its
    /// exposure, clamped into the new range as `_update_pivot_x` keeps it; nothing written
    /// with no effective exposure read.
    #[test]
    fn read_exposure_sets_the_range_from_the_effective_exposure() {
        let image = crate::testing::image();
        for e in [-2.0f32, 0.0, 0.7, 3.0, -5.0, 16.0, 30.0] {
            for s in [-50.0f32, 10.0, 200.0] {
                for pivot in [0.0f32, 5.0, -9.0] {
                    let mut values = DEFINITION.defaults();
                    values.set(SCALING, ControlValue::Number(s));
                    values.set(PIVOT, ControlValue::Number(pivot));
                    let mut read = PublishedValues::new();
                    read.set(EXPOSURE_READ, e);
                    let writes = Agx.action("read_exposure", &values, &read, &image).unwrap();
                    let scale = 1.0 + f64::from(s / 100.0);
                    let e = f64::from(e);
                    let black = ((-8.0 + 0.5 * e) * scale).clamp(-20.0, f64::from(-0.1f32));
                    let white = ((4.0 + 0.8 * e) * scale).clamp(f64::from(0.1f32), 20.0);
                    let (got_black, got_white) = (writes.number(BLACK), writes.number(WHITE));
                    assert!((f64::from(got_black) - black).abs() < 1e-5, "{e} {s}");
                    assert!((f64::from(got_white) - white).abs() < 1e-5, "{e} {s}");
                    assert!((-20.0..=-0.1).contains(&got_black));
                    assert!((0.1..=20.0).contains(&got_white));
                    // `_update_pivot_x`: the pivot's exposure, clamped into the new range.
                    let kept = pivot.clamp(got_black, got_white);
                    let written = writes.get(PIVOT).map(|_| writes.number(PIVOT));
                    assert_eq!(written.unwrap_or(pivot), kept, "{e} {s} {pivot}");
                    assert_eq!(written.is_some(), kept != pivot);
                    // Curve y gamma never (decision 63), nor anything else.
                    assert!(
                        writes
                            .iter()
                            .all(|(id, _)| [BLACK, WHITE, PIVOT].contains(&id))
                    );
                }
            }
        }
        // The ends: white at 0.1 and black at −20 at the low end, black at −0.1 and white
        // at 20 at the high end.
        let ends = |e: f32, s: f32| {
            let mut values = DEFINITION.defaults();
            values.set(SCALING, ControlValue::Number(s));
            let mut read = PublishedValues::new();
            read.set(EXPOSURE_READ, e);
            let writes = Agx.action("read_exposure", &values, &read, &image).unwrap();
            (writes.number(BLACK), writes.number(WHITE))
        };
        assert_eq!(ends(-5.0, 10.0).1, 0.1);
        assert_eq!(ends(-20.0, 200.0).0, -20.0);
        assert_eq!(ends(16.0, 10.0).0, -0.1);
        assert_eq!(ends(30.0, 200.0).1, 20.0);
        // No effective exposure read: nothing written.
        let values = DEFINITION.defaults();
        let none = Agx.action("read_exposure", &values, &PublishedValues::new(), &image);
        assert!(none.unwrap().is_empty());
    }

    /// The toe's and the shoulder's warnings show while the curve falls back, its
    /// `fallback`, darktable's `need_convex_toe` and `need_concave_shoulder`: off at the
    /// defaults; the shoulder's, then both, as contrast falls; the toe's with a narrow range
    /// below the pivot and the shoulder's with one above it.
    #[test]
    fn the_curve_s_warnings_follow_its_fallback() {
        let image = crate::testing::image();
        const CONTRAST: &str = "curve_contrast_around_pivot";
        let warned = |set: &[(&'static str, f32)]| {
            let mut values = DEFINITION.defaults();
            for &(id, value) in set {
                values.set(id, ControlValue::Number(value));
            }
            let curve = Curve::new(&curve_settings(&values));
            let toe = Agx.warning("toe", &values, &image);
            let shoulder = Agx.warning("shoulder", &values, &image);
            assert_eq!(
                (toe, shoulder),
                (curve.toe.fallback, curve.shoulder.fallback)
            );
            (toe, shoulder)
        };
        assert_eq!(warned(&[]), (false, false));
        // A low contrast: the shoulder first, then both.
        assert_eq!(warned(&[(CONTRAST, 1.0)]), (false, true));
        assert_eq!(warned(&[(CONTRAST, 0.5)]), (true, true));
        // A narrow range: below the pivot the toe's, above it the shoulder's.
        assert_eq!(warned(&[(BLACK, -2.0)]), (true, false));
        assert_eq!(warned(&[(WHITE, 2.0)]), (false, true));
        assert!(!Agx.warning("other", &DEFINITION.defaults(), &image));
        assert_eq!(DEFINITION.warnings(), ["shoulder", "toe"]);
    }

    /// Set from above can run only while the primaries after tone mapping act, darktable's
    /// `post_curve_primaries_available`: not while Reverse all or Disable adjustments is on.
    #[test]
    fn set_from_above_is_unavailable_while_reverse_all_or_disable_adjustments_is_on() {
        let image = crate::testing::image();
        for (reverse, disable) in [(false, false), (true, false), (false, true), (true, true)] {
            let mut values = DEFINITION.defaults();
            values.set("completely_reverse_primaries", ControlValue::Bool(reverse));
            values.set("disable_primaries_adjustments", ControlValue::Bool(disable));
            let available = Agx.available("set_from_above", &values, &image);
            assert_eq!(available, !reverse && !disable, "{reverse} {disable}");
            // Every control can change something.
            assert!(
                CONTROLS
                    .iter()
                    .all(|c| Agx.available(c.id, &values, &image))
            );
        }
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

    /// A reading at the operation's input: the area's mean, and each channel's minimum
    /// and maximum.
    fn reading(mean: [f32; 3], min: [f32; 3], max: [f32; 3]) -> Reading {
        let four = |[r, g, b]: [f32; 3]| [r, g, b, 1.0];
        Reading {
            input: crate::pick::Stats {
                mean: four(mean),
                min: four(min),
                max: four(max),
            },
            output: None,
            input_maximum: [1.0; 3],
        }
    }

    /// A flat patch of working RGB `rgb`.
    fn flat(rgb: [f32; 3]) -> Reading {
        reading(rgb, rgb, rgb)
    }

    /// Every control: the defaults, with `set` over them.
    fn values_with(set: &[(&'static str, ControlValue)]) -> Controls {
        let mut values = DEFINITION.defaults();
        for (control, value) in set {
            values.set(control, value.clone());
        }
        values
    }

    fn pick(picker: &str, reading: &Reading, values: &Controls) -> Option<Picked> {
        let image = crate::testing::image();
        let keys = PickKeys::default();
        Agx.pick(picker, reading, values, &image, &[], None, None, keys)
    }

    /// The controls `picked` writes, sorted.
    fn written(picked: &Picked) -> Vec<&str> {
        let mut ids: Vec<&str> = picked.writes.iter().map(|(id, _)| id).collect();
        ids.sort_unstable();
        ids
    }

    fn sorted(mut ids: Vec<&str>) -> Vec<&str> {
        ids.sort_unstable();
        ids
    }

    /// darktable's `_epsilon`, widened.
    const WIDE_EPSILON: f64 = 1e-6;

    /// darktable's black or white of `v` at Dynamic range scaling `scaling` in percent,
    /// restated in f64: `clamp(log2(max(ε, v) / 0.18) · (1 + s), low, high)`
    /// (`agx.c:1103-1133`).
    fn darktable_exposure(v: f32, scaling: f32, (low, high): (f64, f64)) -> f64 {
        let s = f64::from(scaling) / 100.0;
        let ev = (f64::from(v).max(WIDE_EPSILON) / 0.18).log2();
        (ev * (1.0 + s)).clamp(low, high)
    }

    const DARKTABLE_BLACK: (f64, f64) = (-20.0, -0.1);
    const DARKTABLE_WHITE: (f64, f64) = (0.1, 20.0);

    /// darktable's `_luminance_from_profile` restated in f64: the working profile's
    /// RGB to XYZ D50, linear Rec2020 as lcms builds it, its Y row.
    fn darktable_luminance([r, g, b]: [f32; 3]) -> f64 {
        let row = WorkingProfile::linear_rec2020().rgb_to_xyz_d50[1].map(f64::from);
        row[0] * f64::from(r) + row[1] * f64::from(g) + row[2] * f64::from(b)
    }

    fn number(picked: &Picked, control: &str) -> f64 {
        f64::from(picked.writes.number(control))
    }

    /// Black and white relative exposure from readings whose minimum and maximum differ by
    /// channel, the minimum's smallest and the maximum's largest, scaled by Dynamic range
    /// scaling at −50 %, 0, +10 % and +200 %, within 1e-5 EV of darktable's arithmetic
    /// in f64; Auto tune levels' `range` writing both. The pivot, at 0 EV, stays inside
    /// each range, so none writes it.
    #[test]
    fn black_and_white_are_darktable_s_from_the_extremes_scaled() {
        let area = reading([0.3, 0.2, 0.25], [0.021, 0.0042, 0.013], [2.4, 1.1, 3.7]);
        let mut off = 0.0_f64;
        for scaling in [-50.0, 0.0, 10.0, 200.0] {
            let values = values_with(&[(SCALING, ControlValue::Number(scaling))]);
            let black = darktable_exposure(0.0042, scaling, DARKTABLE_BLACK);
            let white = darktable_exposure(3.7, scaling, DARKTABLE_WHITE);
            for (picker, want) in [
                ("black", vec![(BLACK, black)]),
                ("white", vec![(WHITE, white)]),
                ("range", vec![(BLACK, black), (WHITE, white)]),
            ] {
                let picked = pick(picker, &area, &values).unwrap();
                let ids = want.iter().map(|(id, _)| *id).collect();
                assert_eq!(written(&picked), sorted(ids), "{picker} at {scaling} %");
                for (control, want) in want {
                    let got = number(&picked, control);
                    off = off.max((got - want).abs());
                    assert!(
                        (got - want).abs() <= 1e-5,
                        "{picker} at {scaling} %: {control} {got} for {want}"
                    );
                }
                assert_eq!(picked.shown, Shown::default());
            }
        }
        println!("black and white within {off:.1e} EV of darktable's");
    }

    /// A minimum of 0, or below, is floored at darktable's `_epsilon` before its `log2`,
    /// −17.46 EV at no scaling, and each of darktable's bounds is reached: black at −20 EV
    /// and −0.1 EV, white at 0.1 EV and 20 EV.
    #[test]
    fn a_pick_floors_at_epsilon_and_reaches_darktable_s_bounds() {
        let at = |scaling: f32| values_with(&[(SCALING, ControlValue::Number(scaling))]);
        let exposure = |picker: &str, v: f32, scaling: f32| {
            let control = if picker == "black" { BLACK } else { WHITE };
            number(&pick(picker, &flat([v; 3]), &at(scaling)).unwrap(), control)
        };
        let floored = exposure("black", 0.0, 0.0);
        let expected = (WIDE_EPSILON / 0.18).log2();
        assert!((floored - expected).abs() <= 1e-5, "{floored}");
        assert!(floored > -20.0, "{floored} is not held at the bound");
        assert_eq!(exposure("black", -0.3, 0.0), floored);
        for (picker, v, scaling, bound) in [
            ("black", 0.0, 200.0, -20.0),
            ("black", 0.18, 10.0, -0.1),
            ("black", 1.0, 10.0, -0.1),
            ("white", 1e6, 10.0, 20.0),
            ("white", 0.18, 10.0, 0.1),
            ("white", 0.0, 10.0, 0.1),
        ] {
            let got = exposure(picker, v, scaling);
            assert_eq!(
                got,
                f64::from(bound as f32),
                "{picker} of {v} at {scaling} %"
            );
            let darktable = match picker {
                "black" => darktable_exposure(v, scaling, DARKTABLE_BLACK),
                _ => darktable_exposure(v, scaling, DARKTABLE_WHITE),
            };
            assert!(
                (got - darktable).abs() <= 1e-6,
                "{picker}: {got} for {darktable}"
            );
        }
    }

    /// The pivot keeps its exposure as `_update_pivot_x` keeps it: where the new range still
    /// holds it, no pick writes it; where the range's move leaves it outside, the pick
    /// writes it at the range's nearer end, the white or black it picked.
    #[test]
    fn the_pivot_is_written_only_where_the_new_range_clamps_it() {
        let pivot_at = |ev: f32| values_with(&[(PIVOT, ControlValue::Number(ev))]);
        // A white of log2(1.5 / 0.18) · 1.1, about 3.4 EV.
        let bright = flat([1.5; 3]);
        let picked = pick("white", &bright, &pivot_at(2.0)).unwrap();
        assert_eq!(written(&picked), [WHITE]);
        let picked = pick("white", &bright, &pivot_at(5.0)).unwrap();
        assert_eq!(written(&picked), sorted(vec![WHITE, PIVOT]));
        assert_eq!(picked.writes.number(PIVOT), picked.writes.number(WHITE));
        // A black of log2(0.01 / 0.18) · 1.1, about −4.6 EV.
        let dark = flat([0.01; 3]);
        let picked = pick("black", &dark, &pivot_at(-4.0)).unwrap();
        assert_eq!(written(&picked), [BLACK]);
        let picked = pick("black", &dark, &pivot_at(-8.0)).unwrap();
        assert_eq!(written(&picked), sorted(vec![BLACK, PIVOT]));
        assert_eq!(picked.writes.number(PIVOT), picked.writes.number(BLACK));
        // Both at once: the pivot held at the end it fell past.
        let area = reading([0.2; 3], [0.01; 3], [1.5; 3]);
        let picked = pick("range", &area, &pivot_at(5.0)).unwrap();
        assert_eq!(written(&picked), sorted(vec![WHITE, BLACK, PIVOT]));
        assert_eq!(picked.writes.number(PIVOT), picked.writes.number(WHITE));
        let picked = pick("range", &area, &pivot_at(1.0)).unwrap();
        assert_eq!(written(&picked), sorted(vec![WHITE, BLACK]));
    }

    /// The pivot picker sets the pivot at `log2(max(ε, Y) / 0.18)` of the mean's luminance
    /// through the working profile, within 1e-5 EV of darktable's in f64, held within the
    /// range: a bright area at white relative exposure, a dark one at black. It writes the
    /// pivot alone.
    #[test]
    fn the_pivot_picker_sets_the_mean_s_exposure_within_the_range() {
        let values = values_with(&[
            (BLACK, ControlValue::Number(-6.0)),
            (WHITE, ControlValue::Number(4.0)),
        ]);
        for (rgb, want) in [
            ([0.18; 3], None),
            ([0.5, 0.3, 0.1], None),
            ([0.02, 0.06, 0.4], None),
            ([30.0, 20.0, 25.0], Some(4.0)),
            ([0.001, 0.002, 0.001], Some(-6.0)),
        ] {
            let picked = pick("pivot", &flat(rgb), &values).unwrap();
            assert_eq!(written(&picked), [PIVOT]);
            let y = darktable_luminance(rgb);
            let darktable = (y.max(WIDE_EPSILON) / 0.18).log2().clamp(-6.0, 4.0);
            let got = number(&picked, PIVOT);
            assert!(
                (got - darktable).abs() <= 1e-5,
                "{rgb:?}: {got} for {darktable}"
            );
            if let Some(end) = want {
                assert_eq!(got, end, "{rgb:?}");
            }
        }
    }

    /// darktable's curve settings at its scene-referred defaults but those given, its
    /// parameters as darktable's params hold them (`_set_scene_referred_default_params`).
    fn darktable_curve(pivot_x: f32, contrast: f32, auto_gamma: bool) -> CurveSettings {
        CurveSettings {
            range_black_relative_ev: -10.0,
            range_white_relative_ev: 6.5,
            curve_pivot_x: pivot_x,
            curve_pivot_y_linear_output: 0.18,
            curve_contrast_around_pivot: contrast,
            curve_linear_ratio_below_pivot: 0.0,
            curve_linear_ratio_above_pivot: 0.0,
            curve_toe_power: 1.5,
            curve_shoulder_power: 3.3,
            curve_gamma: 2.2,
            auto_gamma,
            curve_target_display_black_ratio: 0.0,
            curve_target_display_white_ratio: 1.0,
        }
    }

    /// Pivot target output: the pivot moved to the area's exposure, and the output the
    /// curve made from the values before the pick, so with the old pivot, gives the new
    /// pivot's place, raised to darktable's `p->curve_gamma` (`_apply_auto_pivot_xy`,
    /// `agx.c:1171-1200`), within 1e-5 EV and 1e-5 in the parameter of darktable's
    /// arithmetic, restated in f64 around the curve `apertrust-color` ports. With Keep the
    /// pivot on the diagonal on, darktable's `gui_changed` keeps `p->curve_gamma` at
    /// `log2(output) / log2(pivot)` (`:1798-1804`), not the stored 2.2, which this tool
    /// keeps; either way Curve y gamma is never written.
    #[test]
    fn the_pivot_output_picker_keeps_the_curve_s_linear_output_and_never_writes_the_gamma() {
        let mut off = (0.0_f64, 0.0_f64);
        for auto_gamma in [false, true] {
            for rgb in [[0.05; 3], [0.6, 0.5, 0.4], [2.0; 3]] {
                // The pivot at +1 EV, contrast 2.5: the curve before the pick.
                let values = values_with(&[
                    (PIVOT, ControlValue::Number(1.0)),
                    ("curve_contrast_around_pivot", ControlValue::Number(2.5)),
                    ("auto_gamma", ControlValue::Bool(auto_gamma)),
                ]);
                let picked = pick("pivot_output", &flat(rgb), &values).unwrap();
                assert_eq!(written(&picked), sorted(vec![PIVOT, OUTPUT]));
                let y = darktable_luminance(rgb);
                let ev = (y.max(WIDE_EPSILON) / 0.18).log2().clamp(-10.0, 6.5);
                let x = (ev + 10.0) / 16.5;
                let before = darktable_curve(11.0 / 16.5, 2.5, auto_gamma);
                // darktable's `p->curve_gamma` as its `gui_changed` leaves it.
                let gamma = if auto_gamma {
                    0.18_f64.log2() / (11.0_f64 / 16.5).log2()
                } else {
                    2.2
                };
                let curve = |settings: &CurveSettings| {
                    let at = Curve::new(settings).eval(x as f32);
                    f64::from(at).powf(gamma)
                };
                let want = curve(&before);
                let got = number(&picked, OUTPUT) / 100.0;
                off.0 = off.0.max((number(&picked, PIVOT) - ev).abs());
                off.1 = off.1.max((got - want).abs());
                assert!((number(&picked, PIVOT) - ev).abs() <= 1e-5, "{rgb:?}");
                assert!((got - want).abs() <= 1e-5, "{rgb:?}: {got} for {want}");
                // The curve with the new pivot would give another output.
                let after = darktable_curve(x as f32, 2.5, auto_gamma);
                assert!((curve(&after) - want).abs() > 1e-3, "{rgb:?}");
            }
        }
        println!("pivot within {:.1e} EV, output within {:.1e}", off.0, off.1);
    }

    #[test]
    fn a_picker_it_does_not_run_gives_nothing() {
        let values = DEFINITION.defaults();
        assert_eq!(pick("exposure", &flat([0.18; 3]), &values), None);
        assert_eq!(pick("", &flat([0.18; 3]), &values), None);
    }
}
