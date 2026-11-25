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

//! Colour balance RGB (`docs/tools/colour-balance-rgb.md`): darktable's color
//! balance rgb as the user edits with it, in the first pass (decision 2): darktable's
//! page, its controls in `gui_init`'s order under its labels.
//!
//! - **Master:** hue shift, global vibrance and contrast; then linear chroma grading,
//!   perceptual saturation grading and perceptual brilliance grading, each global and
//!   for the shadows, mid-tones and highlights.
//! - **4 ways:** the global offset, shadows lift, highlights gain and power, each with
//!   its luminance, hue and chroma.
//! - **Masks:** the saturation formula, darktable UCS by default and JzAzBz darktable's
//!   other (decision 22); the luminance ranges' fall-offs and middle-grey fulcrum; the
//!   white fulcrum and the contrast grey fulcrum.
//!
//! Each control is one `colorbalancergb` parameter, shown as darktable's slider shows it:
//! its percentages as percentages, so the binding divides them by 100, and its degrees
//! and EV as they are. It is switched (decision 32), as darktable leaves color balance rgb off
//! on a new photo: off, it renders nothing; any write of a control turns it on, with
//! darktable's defaults for the others, until its reset turns it off, `colorbalancergb`
//! planned while it is on. Its first step, on at its defaults, brings the operation's
//! gamut mapping, as darktable's does.
//!
//! Its six pickers are darktable's, on the four hues and the two fulcrums: a wheel's hue
//! and chroma from an area, the opposite hue unless Ctrl is held, the white fulcrum from
//! its maximum's luminance and the contrast grey fulcrum from its mean's
//! ([`ColourBalanceRgb::pick`]). Left out until the renderer can draw them: the luminance
//! ranges' graph and the fall-offs' and the mask fulcrum's mask display; and the mask
//! preview's settings, which are the app's (`docs/tools/README.md`, rule 12).

use apertrust_color::{
    WorkingProfile, XYZ_D50_TO_D65_CAT16, apply, rad_to_deg, xyz_to_ych, ych_hue,
};
use apertrust_plugin::{ImageContext, Value};

use crate::about::{Fact, Facts, Purpose};
use crate::definition::{
    ChoiceStyle, Control, ControlKind, CursorDef, CursorGroup, End, Group, GroupKind, Picker,
    PickerKind, ProcessingDefinition, Reset, Row, Slider, Space, Surface, Tab, ToolCursor,
    ToolDefinition, Track, Unit,
};
use crate::pick::{PickKeys, Picked, Reading, Shown};
use crate::{
    Base, ControlValue, Controls, Owned, ProcessingTool, Setting, ToolValues, switched_from,
};

pub struct ColourBalanceRgb;

/// The operation it drives.
const COLOR_BALANCE: &str = "colorbalancergb";

/// The saturation formula's control, its options as the operation's choices, and its
/// default: darktable UCS, as darktable's new instance has it (decision 22).
pub const FORMULA: &str = "saturation_formula";
pub const FORMULAS: [&str; 2] = ["jzazbz", "ucs"];
pub const UCS: u32 = 1;

/// A number control's parameter, and what the control shows of it: the parameter times
/// `scale`, 100 where darktable's slider shows a percentage, 1 for degrees and EV.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parameter {
    pub control: &'static str,
    pub param: &'static str,
    pub scale: f32,
}

const fn percent(control: &'static str, param: &'static str) -> Parameter {
    Parameter {
        control,
        param,
        scale: 100.0,
    }
}

const fn plain(control: &'static str, param: &'static str) -> Parameter {
    Parameter {
        control,
        param,
        scale: 1.0,
    }
}

/// Every number control's parameter, in the page's order. Controls are snake_case, so
/// darktable's `shadows_Y` is `shadows_y`.
pub static PARAMETERS: [Parameter; 32] = [
    plain("hue_angle", "hue_angle"),
    percent("vibrance", "vibrance"),
    percent("contrast", "contrast"),
    percent("chroma_global", "chroma_global"),
    percent("chroma_shadows", "chroma_shadows"),
    percent("chroma_midtones", "chroma_midtones"),
    percent("chroma_highlights", "chroma_highlights"),
    percent("saturation_global", "saturation_global"),
    percent("saturation_shadows", "saturation_shadows"),
    percent("saturation_midtones", "saturation_midtones"),
    percent("saturation_highlights", "saturation_highlights"),
    percent("brilliance_global", "brilliance_global"),
    percent("brilliance_shadows", "brilliance_shadows"),
    percent("brilliance_midtones", "brilliance_midtones"),
    percent("brilliance_highlights", "brilliance_highlights"),
    percent("global_y", "global_Y"),
    plain("global_h", "global_H"),
    percent("global_c", "global_C"),
    percent("shadows_y", "shadows_Y"),
    plain("shadows_h", "shadows_H"),
    percent("shadows_c", "shadows_C"),
    percent("highlights_y", "highlights_Y"),
    plain("highlights_h", "highlights_H"),
    percent("highlights_c", "highlights_C"),
    percent("midtones_y", "midtones_Y"),
    plain("midtones_h", "midtones_H"),
    percent("midtones_c", "midtones_C"),
    percent("shadows_weight", "shadows_weight"),
    percent("mask_grey_fulcrum", "mask_grey_fulcrum"),
    percent("highlights_weight", "highlights_weight"),
    plain("white_fulcrum", "white_fulcrum"),
    percent("grey_fulcrum", "grey_fulcrum"),
];

/// A number control. `step` is darktable's: what its slider steps by over its soft
/// range, which a key steps by here.
const fn number(
    id: &'static str,
    (min, max): (f32, f32),
    step: f32,
    default: f32,
    unit: Unit,
) -> Control {
    Control {
        id,
        kind: ControlKind::Number {
            min,
            max,
            step,
            default,
        },
        unit: Some(unit),
        unported: false,
    }
}

/// A percentage from −100 % to +100 %, darktable's default 0: darktable's −1 to 1.
const fn signed(id: &'static str, step: f32) -> Control {
    number(id, (-100.0, 100.0), step, 0.0, Unit::Percent)
}

/// A wheel's hue, 0° to 360°, darktable's default 0.
const fn wheel_hue(id: &'static str) -> Control {
    number(id, (0.0, 360.0), 1.0, 0.0, Unit::Degrees)
}

/// A wheel's chroma, 0 % to 100 %, darktable's default 0.
const fn wheel_chroma(id: &'static str, step: f32) -> Control {
    number(id, (0.0, 100.0), step, 0.0, Unit::Percent)
}

/// darktable's fulcrums' default, 18.45 % of Yrg luminance.
const FULCRUM: f32 = 18.45;

/// darktable's hard ranges are the controls', in the units its sliders show; its soft
/// ranges are the rows' (`Slider::soft`).
static CONTROLS: [Control; 33] = [
    number("hue_angle", (-180.0, 180.0), 1.0, 0.0, Unit::Degrees),
    signed("vibrance", 0.5),
    signed("contrast", 0.5),
    signed("chroma_global", 0.5),
    signed("chroma_shadows", 1.0),
    signed("chroma_midtones", 1.0),
    signed("chroma_highlights", 1.0),
    signed("saturation_global", 1.0),
    signed("saturation_shadows", 1.0),
    signed("saturation_midtones", 1.0),
    signed("saturation_highlights", 1.0),
    signed("brilliance_global", 1.0),
    signed("brilliance_shadows", 1.0),
    signed("brilliance_midtones", 1.0),
    signed("brilliance_highlights", 1.0),
    signed("global_y", 0.05),
    wheel_hue("global_h"),
    wheel_chroma("global_c", 0.01),
    signed("shadows_y", 1.0),
    wheel_hue("shadows_h"),
    wheel_chroma("shadows_c", 0.5),
    signed("highlights_y", 0.5),
    wheel_hue("highlights_h"),
    wheel_chroma("highlights_c", 0.1),
    signed("midtones_y", 0.1),
    wheel_hue("midtones_h"),
    wheel_chroma("midtones_c", 0.1),
    Control {
        id: FORMULA,
        kind: ControlKind::Choice {
            options: &FORMULAS,
            default: UCS,
            unported: &[],
        },
        unit: None,
        unported: false,
    },
    number("shadows_weight", (0.0, 300.0), 1.0, 100.0, Unit::Percent),
    number(
        "mask_grey_fulcrum",
        (0.0, 100.0),
        1.0,
        FULCRUM,
        Unit::Percent,
    ),
    number("highlights_weight", (0.0, 300.0), 1.0, 100.0, Unit::Percent),
    number("white_fulcrum", (-16.0, 16.0), 0.01, 0.0, Unit::Ev),
    number("grey_fulcrum", (0.0, 100.0), 0.5, FULCRUM, Unit::Percent),
];

/// A slider row as darktable's: two decimals, filled, its track over the whole range.
const fn slider(control: &'static str) -> Row {
    Row::Slider(Slider::new(control))
}

/// A slider row over darktable's soft range, `min` to `max` in the units shown.
const fn soft(control: &'static str, min: f32, max: f32) -> Row {
    Row::Slider(Slider::new(control).soft(min, max))
}

/// A wheel's luminance, on darktable's track from black to white, over its soft range.
const fn luminance_row(control: &'static str, (min, max): (f32, f32)) -> Row {
    Row::Slider(Slider::new(control).track(Track::Luminance).soft(min, max))
}

/// A wheel's hue, on darktable's track of the hues, unfilled, with darktable's picker
/// `picker`.
const fn hue_row(control: &'static str, picker: &'static str) -> Row {
    Row::Slider(
        Slider::new(control)
            .track(Track::Hue)
            .no_fill()
            .end(End::Picker(picker)),
    )
}

/// A fulcrum, with darktable's picker of its name, over its soft range.
const fn fulcrum_row(control: &'static str, (min, max): (f32, f32)) -> Row {
    Row::Slider(
        Slider::new(control)
            .end(End::Picker(control))
            .soft(min, max),
    )
}

/// darktable's pickers (`DT_COLOR_PICKER_AREA`, each on a slider): a wheel's hue and
/// chroma from the area, the opposite hue unless Ctrl is held, and the white fulcrum from
/// its maximum's luminance and the contrast grey fulcrum from its mean's.
static PICKERS: [Picker; 6] = [
    picker("global_hue", &["global_h", "global_c"]),
    picker("shadows_hue", &["shadows_h", "shadows_c"]),
    picker("midtones_hue", &["midtones_h", "midtones_c"]),
    picker("highlights_hue", &["highlights_h", "highlights_c"]),
    picker("white_fulcrum", &["white_fulcrum"]),
    picker("grey_fulcrum", &["grey_fulcrum"]),
];

/// darktable's `ANGLE_SHIFT`: Filmlight Yrg puts red at 330°, so its GUI shows a hue 30°
/// on, `YRG_RAD_TO_CONVENTIONAL_DEG` (`colorbalancergb.c:48-50`).
const ANGLE_SHIFT: f32 = -30.0;

/// darktable's `pipe_RGB_to_Ych` (`colorbalancergb.c:1261-1275`): working RGB, linear
/// Rec2020 as the operation reads it, to XYZ D50 through the working profile, to D65 by
/// CAT16, then Ych.
fn ych_of([r, g, b, _]: [f32; 4]) -> [f32; 4] {
    let xyz = apply(&WorkingProfile::linear_rec2020().rgb_to_xyz_d50, [r, g, b]);
    xyz_to_ych(apply(&XYZ_D50_TO_D65_CAT16, xyz))
}

/// A hue picker's hue of Ych `ych`, darktable's: its angle in the GUI's degrees, the
/// opposite unless `command` is held (`colorbalancergb.c:1288-1290`), from −150° to 390°.
fn picked_hue(ych: [f32; 4], command: bool) -> f32 {
    let hue = rad_to_deg(ych_hue(ych)) - ANGLE_SHIFT;
    if command { hue } else { hue + 180.0 }
}

/// `value` of `control`'s parameter as the control shows it: a percentage times 100.
fn shown(control: &str, value: f32) -> ControlValue {
    let scale = PARAMETERS
        .iter()
        .find(|p| p.control == control)
        .map_or(1.0, |p| p.scale);
    ControlValue::Number(value * scale)
}

/// An area picker of the tool's input writing `writes`.
const fn picker(id: &'static str, writes: &'static [&'static str]) -> Picker {
    Picker {
        id,
        operation: COLOR_BALANCE,
        kind: PickerKind::Area,
        denoise: false,
        output: false,
        space: Space::Operation,
        keep: false,
        writes,
        settings: &[],
        words_by: None,
    }
}

/// A wheel's hue picker's cursor (decision 104; `docs/cursors.md`), named for its picker
/// and placed `order` in Pick: the picker's second door, which turns it on as its quad
/// does. No key, as darktable's pickers have none. The four share one icon; the
/// fulcrums' pickers, advanced ones on the Masks tab, get no cursor.
const fn hue_cursor(picker: &'static str, order: u16) -> ToolCursor {
    ToolCursor {
        cursor: CursorDef {
            id: picker,
            group: CursorGroup::Pick,
            icon: "pick-colour-balance-rgb",
            key: None,
            order,
            gestures: &[],
            surfaces: &[Surface::Viewer],
            options: &[],
        },
        picker,
    }
}

/// The four hue pickers' cursors, in the page's order, after Colour calibration's.
static CURSORS: [ToolCursor; 4] = [
    hue_cursor("global_hue", 400),
    hue_cursor("shadows_hue", 500),
    hue_cursor("highlights_hue", 600),
    hue_cursor("midtones_hue", 700),
];

/// A wheel's chroma, on darktable's track of its hue's chroma, over its soft range.
const fn chroma_row(control: &'static str, (min, max): (f32, f32)) -> Row {
    Row::Slider(Slider::new(control).track(Track::Saturation).soft(min, max))
}

static DEFINITION: ProcessingDefinition = ProcessingDefinition {
    tool: ToolDefinition {
        id: "colorbalancergb",
        tabs: &[
            Tab {
                id: Some("master"),
                rows: &[
                    slider("hue_angle"),
                    soft("vibrance", -50.0, 50.0),
                    soft("contrast", -50.0, 50.0),
                    Row::Group(Group {
                        id: "linear_chroma_grading",
                        kind: GroupKind::Plain,
                        rows: &[
                            soft("chroma_global", -50.0, 50.0),
                            slider("chroma_shadows"),
                            slider("chroma_midtones"),
                            slider("chroma_highlights"),
                        ],
                    }),
                    Row::Group(Group {
                        id: "perceptual_saturation_grading",
                        kind: GroupKind::Plain,
                        rows: &[
                            slider("saturation_global"),
                            slider("saturation_shadows"),
                            slider("saturation_midtones"),
                            slider("saturation_highlights"),
                        ],
                    }),
                    Row::Group(Group {
                        id: "perceptual_brilliance_grading",
                        kind: GroupKind::Plain,
                        rows: &[
                            slider("brilliance_global"),
                            slider("brilliance_shadows"),
                            slider("brilliance_midtones"),
                            slider("brilliance_highlights"),
                        ],
                    }),
                ],
            },
            Tab {
                id: Some("four_ways"),
                rows: &[
                    Row::Group(Group {
                        id: "global_offset",
                        kind: GroupKind::Plain,
                        rows: &[
                            luminance_row("global_y", (-5.0, 5.0)),
                            hue_row("global_h", "global_hue"),
                            chroma_row("global_c", (0.0, 1.0)),
                        ],
                    }),
                    Row::Group(Group {
                        id: "shadows_lift",
                        kind: GroupKind::Plain,
                        rows: &[
                            // darktable sets the shadows' soft range to the whole range.
                            luminance_row("shadows_y", (-100.0, 100.0)),
                            hue_row("shadows_h", "shadows_hue"),
                            chroma_row("shadows_c", (0.0, 50.0)),
                        ],
                    }),
                    Row::Group(Group {
                        id: "highlights_gain",
                        kind: GroupKind::Plain,
                        rows: &[
                            luminance_row("highlights_y", (-50.0, 50.0)),
                            hue_row("highlights_h", "highlights_hue"),
                            chroma_row("highlights_c", (0.0, 20.0)),
                        ],
                    }),
                    Row::Group(Group {
                        id: "power",
                        kind: GroupKind::Plain,
                        rows: &[
                            luminance_row("midtones_y", (-25.0, 25.0)),
                            hue_row("midtones_h", "midtones_hue"),
                            chroma_row("midtones_c", (0.0, 10.0)),
                        ],
                    }),
                ],
            },
            Tab {
                id: Some("masks"),
                rows: &[
                    Row::Choice {
                        control: FORMULA,
                        style: ChoiceStyle::List,
                        end: &[],
                    },
                    // darktable's graph of the three ranges comes first, once a widget
                    // draws it.
                    Row::Group(Group {
                        id: "luminance_ranges",
                        kind: GroupKind::Plain,
                        rows: &[
                            slider("shadows_weight"),
                            slider("mask_grey_fulcrum"),
                            slider("highlights_weight"),
                        ],
                    }),
                    Row::Group(Group {
                        id: "threshold",
                        kind: GroupKind::Plain,
                        rows: &[
                            fulcrum_row("white_fulcrum", (-2.0, 2.0)),
                            fulcrum_row("grey_fulcrum", (10.0, 50.0)),
                        ],
                    }),
                ],
            },
        ],
        before: &[],
        after: &[],
        reset: Reset::Tool,
        presets: true,
        open_at_first: false,
        menus: &[],
        toolbar: &[],
        cursors: &CURSORS,
    },
    version: 2,
    layers: true,
    auto: false,
    switched: true,
    controls: &CONTROLS,
    preset_rows: &[],
    views: &[],
    pickers: &PICKERS,
    // darktable's `description()` in src/iop/colorbalancergb.c at the reference release.
    facts: Some(Facts {
        purpose: Purpose::CorrectiveOrCreative,
        input: Fact::LinearRgbSceneReferred,
        process: Fact::NonLinearRgb,
        output: Fact::NonLinearRgbSceneReferred,
    }),
    // src/iop/colorbalancergb.c's flags: `IOP_FLAGS_INCLUDE_IN_STYLES`, not `_UNSAFE_COPY`.
    copy_safe: true,
    in_styles: true,
};

/// Every parameter of `colorbalancergb`: the tool is darktable's and owns them all.
static OWNS: [Owned; 33] = owns();

const fn owns() -> [Owned; 33] {
    let mut owns = [Owned::new(COLOR_BALANCE, FORMULA); 33];
    let mut i = 0;
    while i < PARAMETERS.len() {
        owns[i] = Owned::new(COLOR_BALANCE, PARAMETERS[i].param);
        i += 1;
    }
    owns
}

impl ProcessingTool for ColourBalanceRgb {
    fn definition(&self) -> &'static ProcessingDefinition {
        &DEFINITION
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    /// Each control off its default sets its parameter, a percentage over 100; the
    /// formula sets the operation's choice of the same index.
    fn bind(&self, values: &ToolValues, _image: &ImageContext, _base: &Base) -> Vec<Setting> {
        let mut settings: Vec<Setting> = PARAMETERS
            .iter()
            .filter_map(|p| {
                let value = values.number(p.control)?;
                Some(Setting::new(
                    COLOR_BALANCE,
                    p.param,
                    Value::Float(value / p.scale),
                ))
            })
            .collect();
        if let Some(formula) = values.choice(FORMULA) {
            settings.push(Setting::new(COLOR_BALANCE, FORMULA, Value::Choice(formula)));
        }
        settings
    }

    /// Version 1 planned `colorbalancergb` while a control was off its default, darktable's then
    /// as now, and bound every one that was: such an edit is on, its values as they were;
    /// any other rendered nothing, so it is off, its values dropped (decision 32).
    fn migrate(&self, from: u32, values: ToolValues) -> ToolValues {
        if from >= 2 {
            return values;
        }
        let off = values.off_defaults(&DEFINITION.defaults());
        let rendered = off.values().next().is_some();
        switched_from(values, rendered)
    }

    /// darktable's `color_picker_apply` (`colorbalancergb.c:1278-1338`), on the area's
    /// mean and each channel's maximum at the operation's input, working RGB, each in Ych
    /// (`pipe_RGB_to_Ych`), in the controls' units:
    ///
    /// - **a hue picker** writes its wheel's hue, the area's opposite, or its own with Ctrl
    ///   (Cmd on the Mac) held, from −150° to 390° as darktable's, which the session wraps
    ///   onto 0° to 360°, the same angle; and its chroma, the mean's c times its Y;
    /// - **White fulcrum** `log2` of the maximum's Y, in EV, not finite for a black maximum,
    ///   which the session leaves out;
    /// - **Contrast grey fulcrum** the mean's Y.
    ///
    /// darktable bounds none of them: the session holds them to their controls' ranges.
    /// darktable's `gui_changed` after it repaints the chroma tracks alone, so nothing more
    /// is written, and nothing is shown.
    fn pick(
        &self,
        picker: &str,
        reading: &Reading,
        _values: &Controls,
        _image: &ImageContext,
        _settings: &[(&str, ControlValue)],
        _channel: Option<&str>,
        _last: Option<&Reading>,
        keys: PickKeys,
    ) -> Option<Picked> {
        let declared = PICKERS.iter().find(|p| p.id == picker)?;
        let mean = ych_of(reading.input.mean);
        let mut writes = Controls::new();
        match declared.writes {
            ["white_fulcrum"] => {
                let max = ych_of(reading.input.max);
                writes.set("white_fulcrum", shown("white_fulcrum", max[0].log2()));
            }
            ["grey_fulcrum"] => writes.set("grey_fulcrum", shown("grey_fulcrum", mean[0])),
            [hue, chroma] => {
                writes.set(hue, shown(hue, picked_hue(mean, keys.command)));
                writes.set(chroma, shown(chroma, mean[1] * mean[0]));
            }
            _ => return None,
        }
        Some(Picked {
            writes,
            shown: Shown::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use apertrust_operations::OPERATIONS;
    use apertrust_operations::colorbalancergb::ColorBalanceRgb as ColourBalanceOperation;
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
        let values = values.off_defaults(&ColourBalanceRgb.defaults(&image));
        ColourBalanceRgb.bind(&values, &image, &Base::new(OPERATIONS, &image))
    }

    #[test]
    fn the_definition_holds_and_its_binding_sweeps_inside_the_schema() {
        DEFINITION.check().unwrap();
        let (accepted, rejected) = check::accept(&[&ColourBalanceRgb], OPERATIONS);
        assert!(rejected.is_empty(), "{rejected:?}");
        assert_eq!(accepted.len(), 1);
        for image in [crate::testing::image(), ImageContext::full(8, 8)] {
            let problems = check::sweep(&ColourBalanceRgb, OPERATIONS, &image);
            assert!(problems.is_empty(), "{problems:?}");
        }
    }

    #[test]
    fn it_owns_every_parameter_of_the_operation_each_with_one_control() {
        let schema = ColourBalanceOperation.schema();
        let owned: Vec<&str> = OWNS.iter().map(|owned| owned.param).collect();
        let params: Vec<&str> = schema.params.iter().map(|param| param.name).collect();
        assert_eq!(owned.len(), params.len());
        for param in &params {
            assert!(owned.contains(param), "{param}");
        }
        // Each number control's default and ends are its parameter's, shown as its slider
        // shows them: within an f32 step, as 18.45 % over 100 is 0.18450001 where
        // darktable's default is 0.1845; the binding never writes a default.
        for p in &PARAMETERS {
            let control = DEFINITION.control(p.control).unwrap();
            let ControlKind::Number {
                min, max, default, ..
            } = control.kind
            else {
                panic!("{} is a number", p.control);
            };
            let param = schema.param(p.param).unwrap();
            let Kind::Float {
                min: low,
                max: high,
                default: untouched,
            } = param.kind
            else {
                panic!("{} is a float", p.param);
            };
            for (shown, darktable) in [(min, low), (max, high), (default, untouched)] {
                let bound = shown / p.scale;
                assert!(
                    (bound - darktable).abs() <= f32::EPSILON * darktable.abs(),
                    "{}: {bound} for {darktable}",
                    p.control
                );
            }
        }
        // The formula's options are the operation's choices, in its order, its default
        // darktable UCS.
        let Kind::Choice { default, choices } = schema.param(FORMULA).unwrap().kind else {
            panic!("the formula is a choice");
        };
        assert_eq!(choices, FORMULAS);
        assert_eq!(default, UCS);
    }

    /// A slider darktable shows as a percentage: `set_digits(4)` then the "%" format,
    /// whose factor of 100 takes two of the digits.
    const fn percent(id: &'static str, hard: (f32, f32)) -> DarktableSlider {
        DarktableSlider::new(id, hard, 2).shown_as(100.0, 0.0)
    }

    /// A hue in degrees, `from_params`'s two digits.
    const fn degrees(id: &'static str, hard: (f32, f32)) -> DarktableSlider {
        DarktableSlider::new(id, hard, 2)
    }

    const SIGNED: (f32, f32) = (-1.0, 1.0);
    const UNIT: (f32, f32) = (0.0, 1.0);

    /// darktable's color balance rgb sliders (`src/iop/colorbalancergb.c`, the
    /// params and `gui_init`), in its parameters' units: `docs/tools/colour-balance-rgb.md`'s
    /// slider table.
    const DARKTABLE: [DarktableSlider; 32] = [
        degrees("hue_angle", (-180.0, 180.0)),
        percent("vibrance", SIGNED).soft(-0.5, 0.5),
        percent("contrast", SIGNED).soft(-0.5, 0.5),
        percent("chroma_global", SIGNED).soft(-0.5, 0.5),
        percent("chroma_shadows", SIGNED),
        percent("chroma_midtones", SIGNED),
        percent("chroma_highlights", SIGNED),
        percent("saturation_global", SIGNED),
        percent("saturation_shadows", SIGNED),
        percent("saturation_midtones", SIGNED),
        percent("saturation_highlights", SIGNED),
        percent("brilliance_global", SIGNED),
        percent("brilliance_shadows", SIGNED),
        percent("brilliance_midtones", SIGNED),
        percent("brilliance_highlights", SIGNED),
        percent("global_y", SIGNED).soft(-0.05, 0.05),
        degrees("global_h", (0.0, 360.0)),
        percent("global_c", UNIT).soft(0.0, 0.01),
        percent("shadows_y", SIGNED).soft(-1.0, 1.0),
        degrees("shadows_h", (0.0, 360.0)),
        percent("shadows_c", UNIT).soft(0.0, 0.5),
        percent("highlights_y", SIGNED).soft(-0.5, 0.5),
        degrees("highlights_h", (0.0, 360.0)),
        percent("highlights_c", UNIT).soft(0.0, 0.2),
        percent("midtones_y", SIGNED).soft(-0.25, 0.25),
        degrees("midtones_h", (0.0, 360.0)),
        percent("midtones_c", UNIT).soft(0.0, 0.1),
        percent("shadows_weight", (0.0, 3.0)),
        percent("mask_grey_fulcrum", UNIT),
        percent("highlights_weight", (0.0, 3.0)),
        // The " EV" format gives no factor.
        DarktableSlider::new("white_fulcrum", (-16.0, 16.0), 2).soft(-2.0, 2.0),
        percent("grey_fulcrum", UNIT).soft(0.1, 0.5),
    ];

    #[test]
    fn its_sliders_are_darktable_s() {
        let differences = crate::testing::slider_differences(
            &ColourBalanceRgb,
            &DEFINITION.defaults(),
            &DARKTABLE,
        );
        assert!(differences.is_empty(), "{differences:#?}");
        // The factor each shows is the one its binding divides by.
        for p in &PARAMETERS {
            let darktable = DARKTABLE.iter().find(|s| s.id == p.control).unwrap();
            assert_eq!(darktable.factor, p.scale, "{}", p.control);
        }
    }

    /// darktable's pickers (`colorbalancergb.c:1886`, `:1907`, `:1928`, `:1949`, `:2019`,
    /// `:2024`), each an area on its slider: a wheel's hue writing its hue and chroma, a
    /// fulcrum its own value (`color_picker_apply`, `:1278-1338`).
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
                denoise: false,
                output: false,
                writes,
                settings: &[],
            }
        }
        let darktable = [
            area("global_hue", "global_h", &["global_h", "global_c"]),
            area("shadows_hue", "shadows_h", &["shadows_h", "shadows_c"]),
            area("midtones_hue", "midtones_h", &["midtones_h", "midtones_c"]),
            area(
                "highlights_hue",
                "highlights_h",
                &["highlights_h", "highlights_c"],
            ),
            area("white_fulcrum", "white_fulcrum", &["white_fulcrum"]),
            area("grey_fulcrum", "grey_fulcrum", &["grey_fulcrum"]),
        ];
        assert_eq!(crate::testing::pickers_on_sliders(&DEFINITION), darktable);
    }

    #[test]
    fn at_its_defaults_it_sets_nothing() {
        let image = crate::testing::image();
        let base = Base::new(OPERATIONS, &image);
        assert!(
            ColourBalanceRgb
                .bind(&ToolValues::new(), &image, &base)
                .is_empty()
        );
        let defaults: Vec<(&str, ControlValue)> =
            CONTROLS.iter().map(|c| (c.id, c.default())).collect();
        assert!(bind(&defaults).is_empty());
    }

    // The pickers: darktable's `color_picker_apply` (`colorbalancergb.c:1278-1338`) on
    // readings made for the test, against its arithmetic in f64.

    const HUE_PICKERS: [&str; 4] = [
        "global_hue",
        "shadows_hue",
        "midtones_hue",
        "highlights_hue",
    ];

    /// Working RGB, linear Rec2020: red, green, blue, a skin tone and a cyan.
    const COLOURS: [[f32; 3]; 5] = [
        [0.5, 0.05, 0.05],
        [0.05, 0.4, 0.08],
        [0.04, 0.06, 0.5],
        [0.45, 0.3, 0.22],
        [0.03, 0.26, 0.34],
    ];

    /// A reading at the operation's input whose mean is `mean` and each channel's maximum
    /// `max`.
    fn reading(mean: [f32; 3], max: [f32; 3]) -> Reading {
        let [r, g, b] = mean;
        let [mr, mg, mb] = max;
        Reading {
            input: crate::pick::Stats {
                mean: [r, g, b, 1.0],
                min: [r, g, b, 1.0],
                max: [mr, mg, mb, 1.0],
            },
            output: None,
            input_maximum: [1.0; 3],
        }
    }

    fn pick(picker: &str, reading: &Reading, command: bool) -> Option<Picked> {
        let image = crate::testing::image();
        ColourBalanceRgb.pick(
            picker,
            reading,
            &ColourBalanceRgb.defaults(&image),
            &image,
            &[],
            None,
            None,
            PickKeys {
                command,
                shift: false,
            },
        )
    }

    /// darktable's `pipe_RGB_to_Ych` restated in f64: the working profile's matrix as
    /// lcms builds it for linear Rec2020, CAT16 to D65, LMS 2006 and Yrg, then its Y, c and
    /// hue in radians.
    fn ych_wide(rgb: [f32; 3]) -> [f64; 3] {
        use apertrust_color::{
            LMS_TO_GRADING_RGB, Mat3d, PROFILE_D65, REC2020_PRIMARIES, XYZ_D65_TO_LMS_2006,
            profile_rgb_to_xyz_d50,
        };
        let wide = |m: &[[f32; 3]; 3]| m.map(|row| row.map(f64::from));
        let times = |m: &Mat3d, v: [f64; 3]| -> [f64; 3] {
            std::array::from_fn(|r| (0..3).map(|c| m[r][c] * v[c]).sum())
        };
        let to_xyz = profile_rgb_to_xyz_d50(PROFILE_D65, REC2020_PRIMARIES);
        let xyz = times(&to_xyz, rgb.map(f64::from));
        let xyz = times(&wide(&XYZ_D50_TO_D65_CAT16), xyz);
        let lms = times(&wide(&XYZ_D65_TO_LMS_2006), xyz);
        let y = 0.68990272 * lms[0] + 0.34832189 * lms[1];
        let a: f64 = lms.iter().sum();
        let grading = times(&wide(&LMS_TO_GRADING_RGB), lms.map(|v| v / a));
        let (r, g) = (grading[0] - 0.21902143, grading[1] - 0.54371398);
        [y, g.hypot(r), g.atan2(r)]
    }

    /// darktable's hue in its GUI's degrees, the opposite unless `command`.
    fn hue_wide(rgb: [f32; 3], command: bool) -> f64 {
        let hue = ych_wide(rgb)[2].to_degrees() + 30.0;
        if command { hue } else { hue + 180.0 }
    }

    fn written(picked: &Picked, control: &str) -> f64 {
        f64::from(picked.writes.number(control))
    }

    /// Each hue picker writes the area's opposite hue, or with Ctrl its own, and its chroma
    /// `c · Y` as a percentage, on red, green, blue, a skin tone and a cyan: within 1e-4°
    /// in hue and 1e-5 relative in chroma of darktable's arithmetic in f64; darktable's
    /// hue from −150° to 390°, before the session wraps it.
    #[test]
    fn a_hue_picker_writes_the_opposite_hue_or_with_ctrl_its_own_and_its_chroma() {
        let (mut hue_off, mut chroma_off) = (0.0_f64, 0.0_f64);
        let (mut below, mut above) = (false, false);
        for picker in HUE_PICKERS {
            let writes = DEFINITION.picker(picker).unwrap().writes;
            let [hue, chroma] = writes else {
                panic!("{picker}: {writes:?}");
            };
            for rgb in COLOURS {
                let [y, c, _] = ych_wide(rgb);
                for command in [false, true] {
                    let picked = pick(picker, &reading(rgb, rgb), command).unwrap();
                    let mut ids: Vec<&str> = picked.writes.iter().map(|(id, _)| id).collect();
                    ids.sort_unstable();
                    let mut want = writes.to_vec();
                    want.sort_unstable();
                    assert_eq!(ids, want, "{picker}");
                    let got = written(&picked, hue);
                    let expected = hue_wide(rgb, command);
                    hue_off = hue_off.max((got - expected).abs());
                    assert!(
                        (got - expected).abs() <= 1e-4,
                        "{picker} {rgb:?} {command}: {got} for {expected}"
                    );
                    assert!((-150.0..=390.0).contains(&got), "{got}");
                    below |= got < 0.0;
                    above |= got > 360.0;
                    let got = written(&picked, chroma);
                    let expected = 100.0 * c * y;
                    chroma_off = chroma_off.max((got - expected).abs() / expected);
                    assert!(
                        (got - expected).abs() <= 1e-5 * expected,
                        "{picker} {rgb:?}: {got} for {expected}"
                    );
                    assert_eq!(picked.shown, Shown::default());
                }
                // The two are half a turn apart.
                let opposite = pick(picker, &reading(rgb, rgb), false).unwrap();
                let own = pick(picker, &reading(rgb, rgb), true).unwrap();
                let apart = written(&opposite, hue) - written(&own, hue);
                assert!((apart - 180.0).abs() < 1e-4, "{apart}");
            }
        }
        // Red's and blue's own hues lie below 0°, the cyan's opposite past 360°, as darktable
        // stores them.
        assert!(below && above);
        println!("hue pickers: hue within {hue_off:.1e}°, chroma {chroma_off:.1e} relative");
    }

    /// A neutral area has no chroma: its own hue then is darktable's for a chroma of 0,
    /// `atan2(0, 1)` + 30°, and the opposite 180° on.
    #[test]
    fn a_neutral_area_picks_no_chroma_and_darktable_s_hue_for_none() {
        for grey in [0.02, 0.18, 0.9] {
            let picked = pick("global_hue", &reading([grey; 3], [grey; 3]), false).unwrap();
            let chroma = written(&picked, "global_c");
            assert!(chroma.abs() < 1e-5, "{grey}: {chroma}");
        }
        // The white's own Yrg, as `ych_to_yrg` gives it for no chroma.
        let white = apertrust_color::ych_to_yrg([0.18, 0.0, 1.0, 0.0]);
        let none = apertrust_color::yrg_to_ych(white);
        assert_eq!(none[1], 0.0);
        assert_eq!(picked_hue(none, true), 30.0);
        assert_eq!(picked_hue(none, false), 210.0);
    }

    /// White fulcrum is `log2` of the luminance of each channel's maximum, in EV, from a
    /// reading whose maximum is not its mean; Contrast grey fulcrum the mean's luminance as
    /// a percentage; each within 1e-5 relative of darktable's arithmetic in f64.
    #[test]
    fn the_fulcrums_take_the_maximum_s_and_the_mean_s_luminance() {
        let (mut white_off, mut grey_off) = (0.0_f64, 0.0_f64);
        for (mean, max) in [
            ([0.18, 0.18, 0.18], [0.9, 0.7, 0.5]),
            ([0.45, 0.3, 0.22], [2.5, 1.8, 1.1]),
            ([0.05, 0.4, 0.08], [0.3, 0.6, 0.12]),
        ] {
            let picked = pick("white_fulcrum", &reading(mean, max), false).unwrap();
            assert_eq!(picked.writes.iter().count(), 1);
            let got = written(&picked, "white_fulcrum");
            let expected = ych_wide(max)[0].log2();
            white_off = white_off.max((got - expected).abs() / expected.abs());
            assert!(
                (got - expected).abs() <= 1e-5 * expected.abs(),
                "{max:?}: {got} for {expected}"
            );
            let picked = pick("grey_fulcrum", &reading(mean, max), true).unwrap();
            assert_eq!(picked.writes.iter().count(), 1);
            let got = written(&picked, "grey_fulcrum");
            let expected = 100.0 * ych_wide(mean)[0];
            grey_off = grey_off.max((got - expected).abs() / expected);
            assert!(
                (got - expected).abs() <= 1e-5 * expected,
                "{mean:?}: {got} for {expected}"
            );
        }
        // A black maximum: `log2` of 0, not finite, which the session leaves out.
        let picked = pick("white_fulcrum", &reading([0.0; 3], [0.0; 3]), false).unwrap();
        assert!(!picked.writes.number("white_fulcrum").is_finite());
        println!("fulcrums: white within {white_off:.1e} relative, grey {grey_off:.1e}");
    }

    /// A picker the tool does not declare picks nothing.
    #[test]
    fn an_unknown_picker_picks_nothing() {
        let rgb = [0.3, 0.2, 0.1];
        assert_eq!(pick("hue_angle", &reading(rgb, rgb), false), None);
        assert_eq!(pick("", &reading(rgb, rgb), true), None);
    }

    #[test]
    fn a_percentage_sets_its_parameter_over_100_and_degrees_and_ev_as_they_are() {
        let number = ControlValue::Number;
        let settings = bind(&[
            ("contrast", number(25.0)),
            ("grey_fulcrum", number(30.0)),
            ("global_h", number(120.0)),
            ("white_fulcrum", number(-1.5)),
            ("shadows_weight", number(150.0)),
            (FORMULA, ControlValue::Choice(0)),
        ]);
        let value = |param: &str| {
            settings
                .iter()
                .find(|setting| setting.param == param)
                .map(|setting| setting.value)
        };
        assert_eq!(value("contrast"), Some(Value::Float(0.25)));
        assert_eq!(value("grey_fulcrum"), Some(Value::Float(0.3)));
        assert_eq!(value("global_H"), Some(Value::Float(120.0)));
        assert_eq!(value("white_fulcrum"), Some(Value::Float(-1.5)));
        assert_eq!(value("shadows_weight"), Some(Value::Float(1.5)));
        assert_eq!(value(FORMULA), Some(Value::Choice(0)));
        assert_eq!(settings.len(), 6);
    }
}
