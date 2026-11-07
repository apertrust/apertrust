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

//! Soften (`docs/modules/soften.md`): darktable's soften as the user edits with it,
//! in the first pass (decision 2): darktable's page, its four sliders in `gui_init`'s
//! order under its labels, size, saturation, brightness and mix. It has no tabs, groups or
//! presets of its own.
//!
//! Each control is one `soften` parameter, shown as darktable's slider shows it, as it is,
//! with two decimals: the size, the saturation and the mix as percentages, the brightness
//! in EV.
//!
//! It drives `soften`, which an unedited photo does not run, and plans it only while the
//! mix is off 0, whatever the other three hold: the mix is the blurred image's weight, so
//! at 0 the operation returns its input and planning it would only cost its passes
//! (`docs/ports/soften.md`, Tool binding notes). So the mix starts at 0, untouched, where
//! darktable's new instance has 50 %, which becomes the "darktable's default" preset once
//! the header's presets menu is built; the size, the saturation and the brightness start
//! at darktable's 50 %, 100 % and 0.33 EV, the operation's own defaults, so the binding
//! sets each only while it is off them.

use apertrust_plugin::{ImageContext, Value};

use crate::definition::{Control, ControlKind, Definition, Row, Slider, Tab, Unit};
use crate::{Base, Module, ModuleValues, Owned, Setting};

pub struct Soften;

/// The operation it drives.
const SOFTEN: &str = "soften";

/// A number control. `step` is what darktable's slider steps by,
/// `dt_bauhaus_slider_get_step` over its range, since `gui_init` sets none, and what a key
/// steps by here.
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

/// darktable's hard ranges are the controls', in the units its sliders show, and their
/// tracks span them, as darktable sets no soft range. The ids are darktable's parameter
/// names, the mix's `amount`.
static CONTROLS: [Control; 4] = [
    // 0 % to 100 %, darktable's 50 % the default; darktable steps it by 1 %.
    number("size", (0.0, 100.0), 1.0, 50.0, Unit::Percent),
    // 0 % to 100 %, darktable's 100 % the default; darktable steps it by 1 %.
    number("saturation", (0.0, 100.0), 1.0, 100.0, Unit::Percent),
    // −2 to +2 EV, darktable's 0.33 EV the default; darktable steps it by 0.01 EV, a
    // hundredth of its larger end, 2, to the power of ten below.
    number("brightness", (-2.0, 2.0), 0.01, 0.33, Unit::Ev),
    // 0 % to 100 %, untouched at 0; darktable steps it by 1 %.
    number("amount", (0.0, 100.0), 1.0, 0.0, Unit::Percent),
];

static DEFINITION: Definition = Definition {
    id: "soften",
    version: 1,
    layers: true,
    auto: false,
    controls: &CONTROLS,
    tabs: &[Tab {
        id: None,
        // Two decimals each, as `dt_bauhaus_slider_from_params` gives these ranges:
        // "50.00%", "+0.33 EV".
        rows: &[
            Row::Slider(Slider::new("size")),
            Row::Slider(Slider::new("saturation")),
            Row::Slider(Slider::new("brightness")),
            Row::Slider(Slider::new("amount")),
        ],
    }],
    before: &[],
    after: &[],
    presets: &[],
    views: &[],
};

/// Every parameter of `soften`: the module is darktable's and owns them all.
static OWNS: [Owned; 4] = [
    Owned::new(SOFTEN, "size"),
    Owned::new(SOFTEN, "saturation"),
    Owned::new(SOFTEN, "brightness"),
    Owned::new(SOFTEN, "amount"),
];

impl Module for Soften {
    fn definition(&self) -> &'static Definition {
        &DEFINITION
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    /// Nothing while the mix is at 0, its default, whatever the others hold, so the
    /// operation stays out of the plan. Off it, the mix and each other control off its
    /// default set their parameters, as they are; a control at its default leaves its
    /// parameter at the operation's default, which is the same value.
    fn bind(&self, values: &ModuleValues, _image: &ImageContext, _base: &Base) -> Vec<Setting> {
        if values.number("amount").is_none() {
            return Vec::new();
        }
        CONTROLS
            .iter()
            .filter_map(|control| {
                let value = values.number(control.id)?;
                Some(Setting::new(SOFTEN, control.id, Value::Float(value)))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use apertrust_operations::OPERATIONS;
    use apertrust_operations::soften::Soften as SoftenOperation;
    use apertrust_plugin::{Kind, Operation};

    use super::*;
    use crate::check;
    use crate::testing::DarktableSlider;
    use crate::values::ControlValue;

    fn bind(set: &[(&str, f32)]) -> Vec<Setting> {
        let image = crate::testing::image();
        let mut values = ModuleValues::new();
        for &(control, value) in set {
            values.set(control, ControlValue::Number(value));
        }
        let values = values.off_defaults(&Soften.defaults(&image));
        Soften.bind(&values, &image, &Base::new(OPERATIONS, &image))
    }

    #[test]
    fn the_definition_holds_and_its_binding_sweeps_inside_the_schema() {
        DEFINITION.check().unwrap();
        let (accepted, rejected) = check::accept(&[&Soften], OPERATIONS);
        assert!(rejected.is_empty(), "{rejected:?}");
        assert_eq!(accepted.len(), 1);
        for image in [crate::testing::image(), ImageContext::full(8, 8)] {
            let problems = check::sweep(&Soften, OPERATIONS, &image);
            assert!(problems.is_empty(), "{problems:?}");
        }
    }

    #[test]
    fn its_controls_are_darktable_s_sliders_in_gui_init_s_order() {
        let shown: Vec<&str> = DEFINITION
            .rows()
            .flat_map(|row| row.controls().iter().copied())
            .collect();
        assert_eq!(shown, ["size", "saturation", "brightness", "amount"]);
        let controls: Vec<&str> = CONTROLS.iter().map(|c| c.id).collect();
        assert_eq!(controls, shown);
    }

    /// The sweep leaves the other controls at the mix's default, where they bind nothing,
    /// so this holds them to the schema: each control's ends, and the defaults of the
    /// three that start at darktable's, are the operation's own minimum, maximum and
    /// default exactly, and the binding passes a value as it is.
    #[test]
    fn it_owns_every_parameter_of_the_operation_and_its_ends_are_the_schema_s() {
        let schema = SoftenOperation.schema();
        let owned: Vec<&str> = OWNS.iter().map(|owned| owned.param).collect();
        let params: Vec<&str> = schema.params.iter().map(|param| param.name).collect();
        assert_eq!(owned, params);
        for control in &CONTROLS {
            let ControlKind::Number {
                min, max, default, ..
            } = control.kind
            else {
                panic!("{} is a number", control.id);
            };
            let Kind::Float {
                min: low,
                max: high,
                default: untouched,
            } = schema.param(control.id).unwrap().kind
            else {
                panic!("{} is a float", control.id);
            };
            assert_eq!((min, max), (low, high), "{}", control.id);
            if control.id != "amount" {
                assert_eq!(default.to_bits(), untouched.to_bits(), "{}", control.id);
            }
        }
        // The mix alone sets only itself; the others take the operation's defaults.
        assert_eq!(
            bind(&[("amount", 50.0)]),
            [Setting::new(SOFTEN, "amount", Value::Float(50.0))]
        );
    }

    #[test]
    fn at_its_defaults_it_sets_nothing() {
        let image = crate::testing::image();
        let base = Base::new(OPERATIONS, &image);
        assert!(Soften.bind(&ModuleValues::new(), &image, &base).is_empty());
        let defaults: Vec<(&str, f32)> = CONTROLS
            .iter()
            .map(|c| match c.kind {
                ControlKind::Number { default, .. } => (c.id, default),
                _ => unreachable!(),
            })
            .collect();
        assert!(bind(&defaults).is_empty());
    }

    /// The mix is the blurred image's weight, so at 0 soften stays out of the plan,
    /// whatever the other three hold, as grain does at a strength of 0.
    #[test]
    fn at_a_mix_of_0_nothing_else_sets_anything() {
        for set in [
            &[("size", 0.0)][..],
            &[("size", 100.0)],
            &[("saturation", 0.0)],
            &[("brightness", -2.0)],
            &[("brightness", 2.0)],
            &[("size", 33.0), ("saturation", 50.0), ("brightness", 0.5)],
        ] {
            assert!(bind(set).is_empty(), "{set:?}");
        }
    }

    /// darktable's soften sliders (`src/iop/soften.c`, the params and `gui_init`):
    /// `docs/modules/soften.md`'s slider table. Each `dt_bauhaus_slider_set_format(…, "%")`
    /// is over a hard maximum of 100, beyond ±10, so it adds no factor of 100; each keeps
    /// the two digits `dt_bauhaus_slider_from_params` gives its range.
    const DARKTABLE: [DarktableSlider; 4] = [
        DarktableSlider::new("size", (0.0, 100.0), 2),
        DarktableSlider::new("saturation", (0.0, 100.0), 2),
        // " EV".
        DarktableSlider::new("brightness", (-2.0, 2.0), 2),
        // `$DESCRIPTION: "mix"`.
        DarktableSlider::new("amount", (0.0, 100.0), 2),
    ];

    #[test]
    fn its_sliders_are_darktable_s() {
        let differences =
            crate::testing::slider_differences(&Soften, &DEFINITION.defaults(), &DARKTABLE);
        assert!(differences.is_empty(), "{differences:#?}");
    }
}
