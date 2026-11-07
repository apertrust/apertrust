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

//! Colour balance RGB (`docs/modules/colour-balance-rgb.md`): darktable's color
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
//! and EV as they are. It drives `colorbalancergb`, which an unedited photo does not run,
//! planned only while a control is off its default, as in darktable (decision 32): at
//! its defaults it sets nothing, so it adds no operation and leaves the frame as it was.
//! Its first step brings the operation's gamut mapping, as darktable's does.
//!
//! The pickers on the hues and the fulcrums are in the definition, as darktable's page
//! has them; the renderer draws a quad once its tool runs. Left out until the renderer
//! can draw them: the luminance
//! ranges' graph and the fall-offs' and the mask fulcrum's mask display; and the mask
//! preview's settings, which are the app's (`docs/modules/README.md`, rule 12).

use apertrust_plugin::{ImageContext, Value};

use crate::definition::{
    ChoiceStyle, Control, ControlKind, Definition, Quad, Row, Slider, Tab, Track, Unit,
};
use crate::{Base, Module, ModuleValues, Owned, Setting};

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

/// A percentage from −100 % to +100 %, untouched at 0: darktable's −1 to 1.
const fn signed(id: &'static str, step: f32) -> Control {
    number(id, (-100.0, 100.0), step, 0.0, Unit::Percent)
}

/// A wheel's hue, 0° to 360°, untouched at 0.
const fn wheel_hue(id: &'static str) -> Control {
    number(id, (0.0, 360.0), 1.0, 0.0, Unit::Degrees)
}

/// A wheel's chroma, 0 % to 100 %, untouched at 0.
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

/// A wheel's hue, on darktable's track of the hues, unfilled, with darktable's picker.
const fn hue_row(control: &'static str) -> Row {
    Row::Slider(
        Slider::new(control)
            .track(Track::Hue)
            .no_fill()
            .quad(Quad::Picker),
    )
}

/// A fulcrum, with darktable's picker, over its soft range.
const fn fulcrum_row(control: &'static str, (min, max): (f32, f32)) -> Row {
    Row::Slider(Slider::new(control).quad(Quad::Picker).soft(min, max))
}

/// A wheel's chroma, on darktable's track of its hue's chroma, over its soft range.
const fn chroma_row(control: &'static str, (min, max): (f32, f32)) -> Row {
    Row::Slider(Slider::new(control).track(Track::Saturation).soft(min, max))
}

static DEFINITION: Definition = Definition {
    id: "colorbalancergb",
    version: 1,
    layers: true,
    auto: false,
    controls: &CONTROLS,
    tabs: &[
        Tab {
            id: Some("master"),
            rows: &[
                slider("hue_angle"),
                soft("vibrance", -50.0, 50.0),
                soft("contrast", -50.0, 50.0),
                Row::Label {
                    id: "linear_chroma_grading",
                },
                soft("chroma_global", -50.0, 50.0),
                slider("chroma_shadows"),
                slider("chroma_midtones"),
                slider("chroma_highlights"),
                Row::Label {
                    id: "perceptual_saturation_grading",
                },
                slider("saturation_global"),
                slider("saturation_shadows"),
                slider("saturation_midtones"),
                slider("saturation_highlights"),
                Row::Label {
                    id: "perceptual_brilliance_grading",
                },
                slider("brilliance_global"),
                slider("brilliance_shadows"),
                slider("brilliance_midtones"),
                slider("brilliance_highlights"),
            ],
        },
        Tab {
            id: Some("four_ways"),
            rows: &[
                Row::Label {
                    id: "global_offset",
                },
                luminance_row("global_y", (-5.0, 5.0)),
                hue_row("global_h"),
                chroma_row("global_c", (0.0, 1.0)),
                Row::Label { id: "shadows_lift" },
                // darktable sets the shadows' soft range to the whole range.
                luminance_row("shadows_y", (-100.0, 100.0)),
                hue_row("shadows_h"),
                chroma_row("shadows_c", (0.0, 50.0)),
                Row::Label {
                    id: "highlights_gain",
                },
                luminance_row("highlights_y", (-50.0, 50.0)),
                hue_row("highlights_h"),
                chroma_row("highlights_c", (0.0, 20.0)),
                Row::Label { id: "power" },
                luminance_row("midtones_y", (-25.0, 25.0)),
                hue_row("midtones_h"),
                chroma_row("midtones_c", (0.0, 10.0)),
            ],
        },
        Tab {
            id: Some("masks"),
            rows: &[
                Row::Choice {
                    control: FORMULA,
                    style: ChoiceStyle::List,
                },
                // darktable's graph of the three ranges comes first, once a widget draws it.
                Row::Label {
                    id: "luminance_ranges",
                },
                slider("shadows_weight"),
                slider("mask_grey_fulcrum"),
                slider("highlights_weight"),
                Row::Label { id: "threshold" },
                fulcrum_row("white_fulcrum", (-2.0, 2.0)),
                fulcrum_row("grey_fulcrum", (10.0, 50.0)),
            ],
        },
    ],
    before: &[],
    after: &[],
    presets: &[],
    views: &[],
};

/// Every parameter of `colorbalancergb`: the module is darktable's and owns them all.
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

impl Module for ColourBalanceRgb {
    fn definition(&self) -> &'static Definition {
        &DEFINITION
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    /// Each control off its default sets its parameter, a percentage over 100; the
    /// formula sets the operation's choice of the same index. Any of them plans the
    /// operation; none, at the defaults, plans nothing.
    fn bind(&self, values: &ModuleValues, _image: &ImageContext, _base: &Base) -> Vec<Setting> {
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
}

#[cfg(test)]
mod tests {
    use apertrust_operations::OPERATIONS;
    use apertrust_operations::colorbalancergb::ColorBalanceRgb as ColourBalanceOperation;
    use apertrust_plugin::{Kind, Operation};

    use super::*;
    use crate::check;
    use crate::testing::DarktableSlider;
    use crate::values::ControlValue;

    fn bind(set: &[(&str, ControlValue)]) -> Vec<Setting> {
        let image = crate::testing::image();
        let mut values = ModuleValues::new();
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
    /// params and `gui_init`), in its parameters' units: `docs/modules/colour-balance-rgb.md`'s
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

    #[test]
    fn at_its_defaults_it_sets_nothing() {
        let image = crate::testing::image();
        let base = Base::new(OPERATIONS, &image);
        assert!(
            ColourBalanceRgb
                .bind(&ModuleValues::new(), &image, &base)
                .is_empty()
        );
        let defaults: Vec<(&str, ControlValue)> =
            CONTROLS.iter().map(|c| (c.id, c.default())).collect();
        assert!(bind(&defaults).is_empty());
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
