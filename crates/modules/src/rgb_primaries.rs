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

//! RGB primaries (`docs/modules/rgb-primaries.md`): darktable's rgb primaries as
//! the user edits with it, in the first pass (decision 2): darktable's page, its eight
//! sliders in `gui_init`'s order under its labels, red, green and blue hue and purity,
//! then the tint's hue and purity. It has no tabs, groups or presets.
//!
//! Each control is one `primaries` parameter, shown as darktable's slider shows it: a
//! hue in degrees, a primary's purity as a percentage less 100, so 1, on the gamut's
//! edge, is 0 %, and the tint's purity as a percentage. The binding goes back as
//! darktable's slider does, in f32. It is switched (decision 32), as
//! darktable leaves rgb primaries off on a new photo: off, it renders nothing; any write
//! of a control turns it on, with darktable's defaults for the others, until its reset
//! turns it off, `primaries` planned while it is on. Its first step, on at its defaults,
//! brings the operation's small leak of red into green and blue, as darktable's does
//! (`docs/ports/primaries.md`, Per pixel).
//!
//! Each slider's track spans darktable's soft range, ±20° for a primary's hue, −50 % to
//! +50 % for its purity and 0 % to 20 % for the tint's, and takes a typed value to the
//! hard range. Left out until the renderer can draw them: its tracks painted in the
//! colours each primary turns through.

use apertrust_plugin::{ImageContext, Value};

use crate::definition::{Control, ControlKind, Definition, Row, Slider, Tab, Unit};
use crate::{Base, Module, ModuleValues, Owned, Setting, switched_from};

pub struct RgbPrimaries;

/// The operation it drives.
const PRIMARIES: &str = "primaries";

/// darktable's `RAD_2_DEG`, 180/π, as its slider's f32 factor holds it.
const RAD_2_DEG: f32 = (180.0 / std::f64::consts::PI) as f32;

/// A control's parameter, and how the control shows it, as darktable's slider does: the
/// parameter times `factor`, plus `offset`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parameter {
    pub control: &'static str,
    pub param: &'static str,
    pub factor: f32,
    pub offset: f32,
}

impl Parameter {
    /// The parameter for a value the control shows: darktable's
    /// `(value − offset) / factor`, in f32.
    pub fn param(&self, shown: f32) -> f32 {
        (shown - self.offset) / self.factor
    }

    /// What the control shows for a parameter: darktable's `parameter × factor + offset`.
    pub fn shown(&self, param: f32) -> f32 {
        param * self.factor + self.offset
    }
}

/// A hue in radians, shown in degrees.
const fn degrees(control: &'static str) -> Parameter {
    Parameter {
        control,
        param: control,
        factor: RAD_2_DEG,
        offset: 0.0,
    }
}

/// A primary's purity, shown as a percentage less 100: 1, darktable's default, is 0 %.
const fn purity(control: &'static str) -> Parameter {
    Parameter {
        control,
        param: control,
        factor: 100.0,
        offset: -100.0,
    }
}

/// Every control's parameter, in the page's order. The controls are darktable's parameter
/// names.
pub static PARAMETERS: [Parameter; 8] = [
    degrees("red_hue"),
    purity("red_purity"),
    degrees("green_hue"),
    purity("green_purity"),
    degrees("blue_hue"),
    purity("blue_purity"),
    degrees("achromatic_tint_hue"),
    Parameter {
        control: "achromatic_tint_purity",
        param: "achromatic_tint_purity",
        factor: 100.0,
        offset: 0.0,
    },
];

/// A number control, darktable's default 0. `step` is what darktable's slider steps by,
/// `dt_bauhaus_slider_get_step` over its soft range, which a key steps by here.
const fn number(id: &'static str, (min, max): (f32, f32), step: f32, unit: Unit) -> Control {
    Control {
        id,
        kind: ControlKind::Number {
            min,
            max,
            step,
            default: 0.0,
        },
        unit: Some(unit),
        unported: false,
    }
}

/// A primary's hue, −180° to 180°; darktable steps it by 0.1° over its soft range of ±20°.
const fn hue(id: &'static str) -> Control {
    number(id, (-180.0, 180.0), 0.1, Unit::Degrees)
}

/// A primary's purity, −99 % to +400 % (darktable's 0.01 to 5); darktable steps it by 1 %
/// over its soft range of −50 % to +50 %.
const fn primary_purity(id: &'static str) -> Control {
    number(id, (-99.0, 400.0), 1.0, Unit::Percent)
}

/// darktable's hard ranges are the controls', in the units its sliders show; its soft
/// ranges are the rows'.
static CONTROLS: [Control; 8] = [
    hue("red_hue"),
    primary_purity("red_purity"),
    hue("green_hue"),
    primary_purity("green_purity"),
    hue("blue_hue"),
    primary_purity("blue_purity"),
    // No soft range, so darktable steps it by 1° over the whole turn.
    number("achromatic_tint_hue", (-180.0, 180.0), 1.0, Unit::Degrees),
    // 0 % to 99 %; darktable steps it by 0.1 % over its soft range of 0 % to 20 %.
    number("achromatic_tint_purity", (0.0, 99.0), 0.1, Unit::Percent),
];

/// A slider row as darktable's: one decimal, a plain track, filled.
const fn slider(control: &'static str) -> Slider {
    Slider::new(control).decimals(1)
}

/// A primary's hue, its track over darktable's soft range of ±20°.
const fn hue_row(control: &'static str) -> Row {
    Row::Slider(slider(control).soft(-20.0, 20.0))
}

/// A primary's purity, its track over darktable's soft range of −50 % to +50 %.
const fn purity_row(control: &'static str) -> Row {
    Row::Slider(slider(control).soft(-50.0, 50.0))
}

static DEFINITION: Definition = Definition {
    id: "primaries",
    version: 2,
    layers: true,
    auto: false,
    switched: true,
    controls: &CONTROLS,
    tabs: &[Tab {
        id: None,
        rows: &[
            hue_row("red_hue"),
            purity_row("red_purity"),
            hue_row("green_hue"),
            purity_row("green_purity"),
            hue_row("blue_hue"),
            purity_row("blue_purity"),
            // The tint's hue spans the whole turn, as darktable sets it no soft range.
            Row::Slider(slider("achromatic_tint_hue")),
            Row::Slider(slider("achromatic_tint_purity").soft(0.0, 20.0)),
        ],
    }],
    before: &[],
    after: &[],
    presets: &[],
    views: &[],
    pickers: &[],
};

/// Every parameter of `primaries`: the module is darktable's and owns them all.
static OWNS: [Owned; 8] = owns();

const fn owns() -> [Owned; 8] {
    let mut owns = [Owned::new(PRIMARIES, ""); 8];
    let mut i = 0;
    while i < PARAMETERS.len() {
        owns[i] = Owned::new(PRIMARIES, PARAMETERS[i].param);
        i += 1;
    }
    owns
}

impl Module for RgbPrimaries {
    fn definition(&self) -> &'static Definition {
        &DEFINITION
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    /// Each control off its default sets its parameter, as darktable's slider converts
    /// what it shows.
    fn bind(&self, values: &ModuleValues, _image: &ImageContext, _base: &Base) -> Vec<Setting> {
        PARAMETERS
            .iter()
            .filter_map(|p| {
                let shown = values.number(p.control)?;
                Some(Setting::new(
                    PRIMARIES,
                    p.param,
                    Value::Float(p.param(shown)),
                ))
            })
            .collect()
    }

    /// Version 1 planned `primaries` while a control was off its default, darktable's then
    /// as now, and bound every one that was: such an edit is on, its values as they were;
    /// any other rendered nothing, so it is off, its values dropped (decision 32).
    fn migrate(&self, from: u32, values: ModuleValues) -> ModuleValues {
        if from >= 2 {
            return values;
        }
        let off = values.off_defaults(&DEFINITION.defaults());
        let rendered = off.values().next().is_some();
        switched_from(values, rendered)
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use apertrust_operations::OPERATIONS;
    use apertrust_operations::primaries::Primaries;
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
        let values = values.off_defaults(&RgbPrimaries.defaults(&image));
        RgbPrimaries.bind(&values, &image, &Base::new(OPERATIONS, &image))
    }

    #[test]
    fn the_definition_holds_and_its_binding_sweeps_inside_the_schema() {
        DEFINITION.check().unwrap();
        let (accepted, rejected) = check::accept(&[&RgbPrimaries], OPERATIONS);
        assert!(rejected.is_empty(), "{rejected:?}");
        assert_eq!(accepted.len(), 1);
        for image in [crate::testing::image(), ImageContext::full(8, 8)] {
            let problems = check::sweep(&RgbPrimaries, OPERATIONS, &image);
            assert!(problems.is_empty(), "{problems:?}");
        }
    }

    #[test]
    fn its_controls_are_darktable_s_sliders_in_gui_init_s_order() {
        let shown: Vec<&str> = DEFINITION
            .rows()
            .flat_map(|row| row.controls().iter().copied())
            .collect();
        assert_eq!(
            shown,
            [
                "red_hue",
                "red_purity",
                "green_hue",
                "green_purity",
                "blue_hue",
                "blue_purity",
                "achromatic_tint_hue",
                "achromatic_tint_purity",
            ]
        );
        let controls: Vec<&str> = CONTROLS.iter().map(|c| c.id).collect();
        assert_eq!(controls, shown);
        let parameters: Vec<&str> = PARAMETERS.iter().map(|p| p.control).collect();
        assert_eq!(parameters, shown);
    }

    #[test]
    fn it_owns_every_parameter_of_the_operation_each_with_one_control() {
        let schema = Primaries.schema();
        let owned: Vec<&str> = OWNS.iter().map(|owned| owned.param).collect();
        let params: Vec<&str> = schema.params.iter().map(|param| param.name).collect();
        assert_eq!(owned.len(), params.len());
        for param in &params {
            assert!(owned.contains(param), "{param}");
        }
        // Each control's default and ends are its parameter's, as the slider shows them,
        // within an f32 step: 180° binds 3.1415925, one step inside darktable's π.
        for p in &PARAMETERS {
            let ControlKind::Number {
                min, max, default, ..
            } = DEFINITION.control(p.control).unwrap().kind
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
                assert!((low..=high).contains(&bound), "{}: {bound}", p.control);
                assert_eq!(p.shown(darktable), shown, "{}", p.control);
            }
        }
    }

    /// darktable's `deg2radf(20.f)`, a primary's hue's soft range.
    const TWENTY: f32 = 20.0 * PI / 180.0;

    /// A primary's hue, `_setup_hue_slider`: "°", one digit, `RAD_2_DEG`, ±20°.
    const fn hue(id: &'static str) -> DarktableSlider {
        DarktableSlider::new(id, (-PI, PI), 1)
            .shown_as(RAD_2_DEG, 0.0)
            .soft(-TWENTY, TWENTY)
    }

    /// A primary's purity, `_setup_purity_slider`: "%", one digit, a factor of 100 and
    /// an offset of −100, 0.5 to 1.5.
    const fn purity(id: &'static str) -> DarktableSlider {
        DarktableSlider::new(id, (0.01, 5.0), 1)
            .shown_as(100.0, -100.0)
            .soft(0.5, 1.5)
    }

    /// darktable's rgb primaries sliders (`src/iop/primaries.c`, the params and
    /// `gui_init`), in its parameters' units: `docs/modules/rgb-primaries.md`'s slider
    /// table.
    const DARKTABLE: [DarktableSlider; 8] = [
        hue("red_hue"),
        purity("red_purity"),
        hue("green_hue"),
        purity("green_purity"),
        hue("blue_hue"),
        purity("blue_purity"),
        // The tint's hue: no soft range.
        DarktableSlider::new("achromatic_tint_hue", (-PI, PI), 1).shown_as(RAD_2_DEG, 0.0),
        DarktableSlider::new("achromatic_tint_purity", (0.0, 0.99), 1)
            .shown_as(100.0, 0.0)
            .soft(0.0, 0.2),
    ];

    #[test]
    fn its_sliders_are_darktable_s() {
        let differences =
            crate::testing::slider_differences(&RgbPrimaries, &DEFINITION.defaults(), &DARKTABLE);
        assert!(differences.is_empty(), "{differences:#?}");
        // The factor and offset each shows are the ones its binding undoes.
        for p in &PARAMETERS {
            let darktable = DARKTABLE.iter().find(|s| s.id == p.control).unwrap();
            assert_eq!(
                (darktable.factor, darktable.offset),
                (p.factor, p.offset),
                "{}",
                p.control
            );
        }
    }

    #[test]
    fn at_its_defaults_it_sets_nothing() {
        let image = crate::testing::image();
        let base = Base::new(OPERATIONS, &image);
        assert!(
            RgbPrimaries
                .bind(&ModuleValues::new(), &image, &base)
                .is_empty()
        );
        let defaults: Vec<(&str, f32)> = CONTROLS.iter().map(|c| (c.id, 0.0)).collect();
        assert!(bind(&defaults).is_empty());
    }

    #[test]
    fn hues_bind_in_radians_and_purities_as_darktable_s_ratios() {
        let settings = bind(&[
            ("red_hue", 9.5),
            ("green_purity", 23.5),
            ("blue_purity", -18.0),
            ("achromatic_tint_purity", 13.4),
        ]);
        let value = |param: &str| {
            settings
                .iter()
                .find(|setting| setting.param == param)
                .map(|setting| setting.value)
        };
        assert_eq!(value("red_hue"), Some(Value::Float(9.5 / RAD_2_DEG)));
        assert_eq!(value("green_purity"), Some(Value::Float(1.235)));
        assert_eq!(value("blue_purity"), Some(Value::Float(0.82)));
        assert_eq!(value("achromatic_tint_purity"), Some(Value::Float(0.134)));
        assert_eq!(settings.len(), 4);
    }
}
