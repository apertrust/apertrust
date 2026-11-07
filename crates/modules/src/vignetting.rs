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

//! Vignetting (`docs/modules/vignetting.md`): darktable's vignetting as the user
//! edits with it, in the first pass (decision 2): darktable's page, its controls in
//! `gui_init`'s order under its labels. Fall-off start, fall-off radius, brightness and
//! saturation; then, under the position / form label, the horizontal and vertical
//! centres, the shape, automatic ratio and the width/height ratio; then dithering.
//!
//! Each control is one `vignette` parameter, shown as darktable's slider shows it, as it
//! is: the radii in percent, the rest as plain numbers. It drives `vignette`, which an
//! unedited photo does not run, and plans it only while brightness or saturation is off
//! 0, whatever the others hold: at both 0 the vignette gives every pixel back, its values
//! exactly, since `unbound`, which no control shows, is fixed on, so planning it would
//! only cost its pass, as Soften's mix at 0 does. So brightness and saturation start at
//! 0, where darktable's new instance has −0.5 each, and an untouched module adds no
//! operation and leaves the frame as it was; darktable's values and its lomo preset are
//! presets (`docs/modules/README.md`).
//!
//! The width/height ratio is unavailable while automatic ratio is on, as darktable makes
//! it insensitive in `gui_changed` (decision 30): it still takes a value and keeps it.
//! Dithering lists darktable's three options, its two noises not ported. Left out: the
//! vignette's outline and handles darktable draws on the image, and the presets.

use apertrust_plugin::{ImageContext, Value};

use crate::definition::{ChoiceStyle, Control, ControlKind, Definition, Row, Slider, Tab, Unit};
use crate::{Base, ControlValue, Controls, Module, ModuleValues, Owned, Setting};

pub struct Vignetting;

/// The operation it drives.
const VIGNETTE: &str = "vignette";

/// The controls darktable's new instance sets otherwise, and so the binding always does
/// once the operation is planned: brightness and saturation, at 0 untouched where
/// darktable's are −0.5. Each binds its parameter of the same name, as it is.
pub const UNTOUCHED_AT_0: [&str; 2] = ["brightness", "saturation"];

/// The other controls that bind their parameter of the same name, as it is, each only
/// once moved off its default, darktable's: the radii, the centre, the shape and the
/// ratio. Dithering takes only its default, the operation's.
pub const SET_ONCE_MOVED: [&str; 7] = [
    "scale",
    "falloff_scale",
    "center_x",
    "center_y",
    "shape",
    "autoratio",
    "whratio",
];

/// Dithering's options, in the order of the operation's choices, `off`, `8bit` and
/// `16bit`: darktable's off, 8-bit output and 16-bit output, an option's id starting
/// with a letter. The binding sets none while the noises are not ported; their port
/// brings the map from each option to the operation's choice at its place.
pub const DITHERING: [&str; 3] = ["off", "output_8bit", "output_16bit"];

/// A number control. `step` is what darktable's slider steps by,
/// `dt_bauhaus_slider_get_step` over its whole range, since it sets no soft range.
const fn number(
    id: &'static str,
    (min, max): (f32, f32),
    step: f32,
    default: f32,
    unit: Option<Unit>,
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
        unported: false,
    }
}

/// darktable's hard ranges and steps; its defaults but for brightness and saturation.
static CONTROLS: [Control; 10] = [
    number("scale", (0.0, 200.0), 1.0, 80.0, Some(Unit::Percent)),
    number(
        "falloff_scale",
        (0.0, 200.0),
        1.0,
        50.0,
        Some(Unit::Percent),
    ),
    number("brightness", (-1.0, 1.0), 0.01, 0.0, None),
    number("saturation", (-1.0, 1.0), 0.01, 0.0, None),
    number("center_x", (-1.0, 1.0), 0.01, 0.0, None),
    number("center_y", (-1.0, 1.0), 0.01, 0.0, None),
    number("shape", (0.0, 5.0), 0.05, 1.0, None),
    Control {
        id: "autoratio",
        kind: ControlKind::Bool { default: false },
        unit: None,
        unported: false,
    },
    number("whratio", (0.0, 2.0), 0.01, 1.0, None),
    Control {
        id: "dithering",
        kind: ControlKind::Choice {
            options: &DITHERING,
            default: 0,
            unported: &["output_8bit", "output_16bit"],
        },
        unit: None,
        unported: false,
    },
];

static DEFINITION: Definition = Definition {
    id: VIGNETTE,
    version: 1,
    layers: true,
    auto: false,
    controls: &CONTROLS,
    tabs: &[Tab {
        id: None,
        rows: &[
            // The radii with two decimals, as `dt_bauhaus_slider_from_params` gives them;
            // brightness and saturation with three, as `gui_init` sets them.
            Row::Slider(Slider::new("scale")),
            Row::Slider(Slider::new("falloff_scale")),
            Row::Slider(Slider::new("brightness").decimals(3)),
            Row::Slider(Slider::new("saturation").decimals(3)),
            Row::Label {
                id: "position_form",
            },
            Row::Slider(Slider::new("center_x").decimals(3)),
            Row::Slider(Slider::new("center_y").decimals(3)),
            Row::Slider(Slider::new("shape")),
            Row::Toggle {
                control: "autoratio",
            },
            Row::Slider(Slider::new("whratio").decimals(3)),
            Row::Choice {
                control: "dithering",
                style: ChoiceStyle::List,
            },
        ],
    }],
    before: &[],
    after: &[],
    presets: &[],
    views: &[],
};

/// Every parameter of `vignette`: the module is darktable's and owns them all, `unbound`
/// fixed on, with no control, as darktable shows none.
static OWNS: [Owned; 11] = [
    Owned::new(VIGNETTE, "scale"),
    Owned::new(VIGNETTE, "falloff_scale"),
    Owned::new(VIGNETTE, "brightness"),
    Owned::new(VIGNETTE, "saturation"),
    Owned::new(VIGNETTE, "center_x"),
    Owned::new(VIGNETTE, "center_y"),
    Owned::new(VIGNETTE, "autoratio"),
    Owned::new(VIGNETTE, "whratio"),
    Owned::new(VIGNETTE, "shape"),
    Owned::new(VIGNETTE, "dithering"),
    Owned::new(VIGNETTE, "unbound"),
];

impl Module for Vignetting {
    fn definition(&self) -> &'static Definition {
        &DEFINITION
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    /// Nothing while brightness and saturation are both 0, whatever the others hold, the
    /// defaults among them. Otherwise brightness and saturation, whose untouched 0 is not
    /// the operation's default, are set whatever they hold, each other control off its
    /// default sets its parameter as it is, and `unbound` is on, so the operation's other
    /// defaults, darktable's, are the module's. Dithering takes only its default, off,
    /// the operation's.
    fn bind(&self, values: &ModuleValues, _image: &ImageContext, _base: &Base) -> Vec<Setting> {
        if UNTOUCHED_AT_0
            .iter()
            .all(|&control| values.number(control).is_none())
        {
            return Vec::new();
        }
        let mut settings: Vec<Setting> = UNTOUCHED_AT_0
            .iter()
            .map(|&control| {
                let value = values.number(control).unwrap_or(0.0);
                Setting::new(VIGNETTE, control, Value::Float(value))
            })
            .collect();
        for control in SET_ONCE_MOVED {
            let value = match values.get(control) {
                Some(&ControlValue::Number(value)) => Value::Float(value),
                Some(&ControlValue::Bool(on)) => Value::Bool(on),
                _ => continue,
            };
            settings.push(Setting::new(VIGNETTE, control, value));
        }
        settings.push(Setting::new(VIGNETTE, "unbound", Value::Bool(true)));
        settings
    }

    /// The width/height ratio acts on nothing while automatic ratio is on, which makes
    /// the vignette's ratio the image's: darktable's `gui_changed`,
    /// `gtk_widget_set_sensitive(g->whratio, !p->autoratio)` (decision 30).
    fn available(&self, control: &str, values: &Controls, _image: &ImageContext) -> bool {
        control != "whratio" || !values.flag("autoratio")
    }
}

#[cfg(test)]
mod tests {
    use apertrust_operations::OPERATIONS;
    use apertrust_operations::vignette::Vignette;
    use apertrust_plugin::{Kind, Operation};

    use super::*;
    use crate::check;
    use crate::testing::DarktableSlider;

    fn bind(set: &[(&str, ControlValue)]) -> Vec<Setting> {
        let image = crate::testing::image();
        let mut values = ModuleValues::new();
        for (control, value) in set {
            values.set(control, value.clone());
        }
        let values = values.off_defaults(&Vignetting.defaults(&image));
        Vignetting.bind(&values, &image, &Base::new(OPERATIONS, &image))
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
        let (accepted, rejected) = check::accept(&[&Vignetting], OPERATIONS);
        assert!(rejected.is_empty(), "{rejected:?}");
        assert_eq!(accepted.len(), 1);
        for image in [crate::testing::image(), ImageContext::full(8, 8)] {
            let problems = check::sweep(&Vignetting, OPERATIONS, &image);
            assert!(problems.is_empty(), "{problems:?}");
        }
    }

    #[test]
    fn its_controls_are_darktable_s_in_gui_init_s_order() {
        let shown: Vec<&str> = DEFINITION
            .rows()
            .flat_map(|row| row.controls().iter().copied())
            .collect();
        let order = [
            "scale",
            "falloff_scale",
            "brightness",
            "saturation",
            "center_x",
            "center_y",
            "shape",
            "autoratio",
            "whratio",
            "dithering",
        ];
        assert_eq!(shown, order);
        let controls: Vec<&str> = CONTROLS.iter().map(|c| c.id).collect();
        assert_eq!(controls, order);
        // The group label after saturation, darktable's "position / form".
        let rows = DEFINITION.tabs[0].rows;
        assert!(matches!(
            rows[4],
            Row::Label {
                id: "position_form"
            }
        ));
    }

    #[test]
    fn it_owns_every_parameter_of_the_operation_its_ranges_and_defaults_darktable_s() {
        let schema = Vignette.schema();
        let owned: Vec<&str> = OWNS.iter().map(|owned| owned.param).collect();
        let params: Vec<&str> = schema.params.iter().map(|param| param.name).collect();
        assert_eq!(owned.len(), params.len());
        for param in &params {
            assert!(owned.contains(param), "{param}");
        }
        // Each number control is its parameter as it is: the same range, and the same
        // default but for brightness and saturation, 0 where darktable's are −0.5.
        for control in &CONTROLS {
            let ControlKind::Number {
                min, max, default, ..
            } = control.kind
            else {
                continue;
            };
            let Kind::Float {
                min: low,
                max: high,
                default: darktable,
            } = schema.param(control.id).unwrap().kind
            else {
                panic!("{} is a float", control.id);
            };
            assert_eq!((min, max), (low, high), "{}", control.id);
            let untouched = if UNTOUCHED_AT_0.contains(&control.id) {
                0.0
            } else {
                darktable
            };
            assert_eq!(default, untouched, "{}", control.id);
        }
        let Kind::Choice { choices, default } = schema.param("dithering").unwrap().kind else {
            panic!("dithering is a choice");
        };
        assert_eq!((choices, default), (&["off", "8bit", "16bit"][..], 0));
        assert_eq!(
            schema.param("autoratio").unwrap().kind,
            Kind::Bool { default: false }
        );
    }

    /// darktable's vignetting sliders (`src/iop/vignette.c`, the params and
    /// `gui_init`), in its parameters' units: `docs/modules/vignetting.md`'s slider table.
    /// None has a soft range, a factor or an offset: `dt_bauhaus_slider_set_format`'s
    /// "%" multiplies by nothing on a range that passes 10.
    const DARKTABLE: [DarktableSlider; 8] = [
        DarktableSlider::new("scale", (0.0, 200.0), 2),
        DarktableSlider::new("falloff_scale", (0.0, 200.0), 2),
        DarktableSlider::new("brightness", (-1.0, 1.0), 3),
        DarktableSlider::new("saturation", (-1.0, 1.0), 3),
        DarktableSlider::new("center_x", (-1.0, 1.0), 3),
        DarktableSlider::new("center_y", (-1.0, 1.0), 3),
        DarktableSlider::new("shape", (0.0, 5.0), 2),
        DarktableSlider::new("whratio", (0.0, 2.0), 3),
    ];

    #[test]
    fn its_sliders_are_darktable_s() {
        let differences =
            crate::testing::slider_differences(&Vignetting, &DEFINITION.defaults(), &DARKTABLE);
        assert!(differences.is_empty(), "{differences:#?}");
    }

    #[test]
    fn at_its_defaults_it_sets_nothing() {
        let image = crate::testing::image();
        let base = Base::new(OPERATIONS, &image);
        assert!(
            Vignetting
                .bind(&ModuleValues::new(), &image, &base)
                .is_empty()
        );
        let defaults: Vec<(&str, ControlValue)> =
            CONTROLS.iter().map(|c| (c.id, c.default())).collect();
        assert!(bind(&defaults).is_empty());
    }

    #[test]
    fn it_plans_only_while_a_strength_is_off_0_then_binds_both_and_unbound_on() {
        // While brightness and saturation are 0 nothing is set, whatever the others hold.
        let geometry = [
            ("scale", ControlValue::Number(40.0)),
            ("falloff_scale", ControlValue::Number(100.0)),
            ("center_x", ControlValue::Number(-0.3)),
            ("shape", ControlValue::Number(3.7)),
            ("autoratio", ControlValue::Bool(true)),
            ("whratio", ControlValue::Number(1.6)),
        ];
        assert!(bind(&geometry[..1]).is_empty());
        assert!(bind(&geometry).is_empty());
        // A radius with a strength plans it, the other strength at 0, the operation's
        // other parameters at darktable's defaults, and clipping off.
        let settings = bind(&[
            ("scale", ControlValue::Number(40.0)),
            ("saturation", ControlValue::Number(0.3)),
        ]);
        assert_eq!(value(&settings, "scale"), Some(Value::Float(40.0)));
        assert_eq!(value(&settings, "brightness"), Some(Value::Float(0.0)));
        assert_eq!(value(&settings, "saturation"), Some(Value::Float(0.3)));
        assert_eq!(value(&settings, "unbound"), Some(Value::Bool(true)));
        assert_eq!(settings.len(), 4);
        // Every control as it is.
        let settings = bind(&[
            ("brightness", ControlValue::Number(-0.968)),
            ("center_y", ControlValue::Number(0.13)),
            ("autoratio", ControlValue::Bool(true)),
            ("whratio", ControlValue::Number(1.172)),
        ]);
        assert_eq!(value(&settings, "brightness"), Some(Value::Float(-0.968)));
        assert_eq!(value(&settings, "saturation"), Some(Value::Float(0.0)));
        assert_eq!(value(&settings, "center_y"), Some(Value::Float(0.13)));
        assert_eq!(value(&settings, "autoratio"), Some(Value::Bool(true)));
        assert_eq!(value(&settings, "whratio"), Some(Value::Float(1.172)));
        assert_eq!(value(&settings, "dithering"), None);
        assert_eq!(settings.len(), 6);
    }

    #[test]
    fn the_width_height_ratio_is_unavailable_while_automatic_ratio_is_on() {
        let image = crate::testing::image();
        let mut values = Vignetting.defaults(&image);
        let available =
            |values: &Controls, control: &str| Vignetting.available(control, values, &image);
        assert!(CONTROLS.iter().all(|c| available(&values, c.id)));
        values.set("whratio", ControlValue::Number(1.6));
        values.set("autoratio", ControlValue::Bool(true));
        for control in &CONTROLS {
            assert_eq!(available(&values, control.id), control.id != "whratio");
        }
        values.set("autoratio", ControlValue::Bool(false));
        assert!(available(&values, "whratio"));
    }

    #[test]
    fn dithering_takes_only_off_the_noises_not_ported() {
        let dithering = DEFINITION.control("dithering").unwrap();
        assert!(dithering.check(&ControlValue::Choice(0)).is_ok());
        assert!(dithering.check(&ControlValue::Choice(1)).is_err());
        assert!(dithering.check(&ControlValue::Choice(2)).is_err());
    }
}
