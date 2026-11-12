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

//! Grain (`docs/modules/grain.md`): darktable's grain as the user edits with it, in
//! the first pass (decision 2): darktable's page, its three sliders in `gui_init`'s order
//! under its labels, coarseness, strength and mid-tones bias. It has no tabs, groups or
//! presets of its own, and works on the background only.
//!
//! Each control is one `grain` parameter, shown as darktable's slider shows it:
//! coarseness as ISO, the parameter times darktable's `GRAIN_SCALE_FACTOR`, 213.2, with
//! no decimals, and the strength and the mid-tones bias as the percentages they are, with
//! two. The binding goes back as darktable's slider does with a typed value, in f32.
//!
//! It is switched (decision 32), as darktable leaves grain off on a new photo: off, it
//! renders nothing; any write of a control turns it on, with darktable's defaults for the
//! others, 1600 ISO, 25 % and 100 %, the operation's own, until its reset turns it off. So
//! the binding sets each control only while it is off its default, and on at its defaults
//! the session plans `grain` at the operation's defaults, darktable's new instance. Version
//! 1 started the strength at 0 and planned `grain` only while it was off 0; its edits
//! migrate on where the strength was off 0, and off, their values dropped, where it was
//! not. darktable's `channel`, which its GUI does not show and its `process()` ignores, is
//! not ported.

use apertrust_plugin::{ImageContext, Value};

use crate::definition::{Control, ControlKind, Definition, Row, Slider, Tab, Unit};
use crate::{Base, Module, ModuleValues, Owned, Setting, switched_from};

pub struct Grain;

/// The operation it drives.
const GRAIN: &str = "grain";

/// darktable's `GRAIN_SCALE_FACTOR`, as its coarseness slider's f32 factor holds it: the
/// slider shows `scale` times this, as ISO.
pub const SCALE_FACTOR: f32 = 213.2;

/// The `scale` a coarseness in ISO binds: darktable's slider's `value / factor`, in f32.
/// 20, 1600 and 6400 ISO give darktable's minimum, default and maximum exactly.
pub fn scale(iso: f32) -> f32 {
    iso / SCALE_FACTOR
}

/// The coarseness in ISO darktable's slider shows for `scale`: `scale × factor`, in f32.
pub fn iso(scale: f32) -> f32 {
    scale * SCALE_FACTOR
}

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
/// names, and the defaults darktable's, its params' introspection defaults.
static CONTROLS: [Control; 3] = [
    // 20 to 6400 ISO, darktable's 1600 the default; darktable steps it by 50 ISO.
    number("scale", (20.0, 6400.0), 50.0, 1600.0, Unit::Iso),
    // 0 % to 100 %, darktable's 25 % the default; darktable steps it by 1 %.
    number("strength", (0.0, 100.0), 1.0, 25.0, Unit::Percent),
    // 0 % to 100 %, darktable's 100 % the default; darktable steps it by 1 %.
    number("midtones_bias", (0.0, 100.0), 1.0, 100.0, Unit::Percent),
];

static DEFINITION: Definition = Definition {
    id: "grain",
    version: 2,
    layers: false,
    auto: false,
    switched: true,
    controls: &CONTROLS,
    tabs: &[Tab {
        id: None,
        rows: &[
            // darktable's `dt_bauhaus_slider_set_digits(g->scale, 0)`: "1600 ISO".
            Row::Slider(Slider::new("scale").decimals(0)),
            // Two decimals, as `dt_bauhaus_slider_from_params` gives a range of 100:
            // "25.00%".
            Row::Slider(Slider::new("strength")),
            Row::Slider(Slider::new("midtones_bias")),
        ],
    }],
    before: &[],
    after: &[],
    presets: &[],
    views: &[],
};

/// Every parameter of `grain`: the module is darktable's and owns them all.
static OWNS: [Owned; 3] = [
    Owned::new(GRAIN, "scale"),
    Owned::new(GRAIN, "strength"),
    Owned::new(GRAIN, "midtones_bias"),
];

impl Module for Grain {
    fn definition(&self) -> &'static Definition {
        &DEFINITION
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    /// Each control off its default sets its parameter, as darktable's sliders convert
    /// what they show; a control at its default leaves its parameter at the operation's
    /// default, which is the same value.
    fn bind(&self, values: &ModuleValues, _image: &ImageContext, _base: &Base) -> Vec<Setting> {
        let mut settings = Vec::new();
        if let Some(iso) = values.number("scale") {
            settings.push(Setting::new(GRAIN, "scale", Value::Float(scale(iso))));
        }
        if let Some(strength) = values.number("strength") {
            settings.push(Setting::new(GRAIN, "strength", Value::Float(strength)));
        }
        if let Some(bias) = values.number("midtones_bias") {
            settings.push(Setting::new(GRAIN, "midtones_bias", Value::Float(bias)));
        }
        settings
    }

    /// Version 1 planned `grain` only while the strength was off 0, its default then: such
    /// an edit is on, its values as they were, the strength among them; any other rendered
    /// nothing, so it is off, its values dropped.
    fn migrate(&self, from: u32, values: ModuleValues) -> ModuleValues {
        if from >= 2 {
            return values;
        }
        let rendered = values.number("strength").is_some_and(|s| s != 0.0);
        switched_from(values, rendered)
    }
}

#[cfg(test)]
mod tests {
    use apertrust_operations::OPERATIONS;
    use apertrust_operations::grain::Grain as GrainOperation;
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
        let values = values.off_defaults(&Grain.defaults(&image));
        Grain.bind(&values, &image, &Base::new(OPERATIONS, &image))
    }

    fn value(settings: &[Setting], param: &str) -> Option<Value> {
        settings
            .iter()
            .find(|setting| setting.param == param)
            .map(|setting| setting.value)
    }

    #[test]
    fn the_definition_holds_and_its_binding_sweeps_inside_the_schema() {
        DEFINITION.check().unwrap();
        let (accepted, rejected) = check::accept(&[&Grain], OPERATIONS);
        assert!(rejected.is_empty(), "{rejected:?}");
        assert_eq!(accepted.len(), 1);
        for image in [crate::testing::image(), ImageContext::full(8, 8)] {
            let problems = check::sweep(&Grain, OPERATIONS, &image);
            assert!(problems.is_empty(), "{problems:?}");
        }
    }

    #[test]
    fn its_controls_are_darktable_s_sliders_in_gui_init_s_order() {
        let shown: Vec<&str> = DEFINITION
            .rows()
            .flat_map(|row| row.controls().iter().copied())
            .collect();
        assert_eq!(shown, ["scale", "strength", "midtones_bias"]);
        let controls: Vec<&str> = CONTROLS.iter().map(|c| c.id).collect();
        assert_eq!(controls, shown);
    }

    /// The sweep leaves the coarseness and the bias at the strength's default, where they
    /// bind nothing, so this holds their ends to the schema with the strength on: each
    /// control's ends, and the defaults of the two that start at darktable's, bind the
    /// operation's own minimum, maximum and default exactly.
    #[test]
    fn it_owns_every_parameter_of_the_operation_and_its_ends_are_the_schema_s() {
        let schema = GrainOperation.schema();
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
            let bound = |shown: f32| match control.id {
                "scale" => scale(shown),
                _ => shown,
            };
            assert_eq!(bound(min).to_bits(), low.to_bits(), "{}", control.id);
            assert_eq!(bound(max).to_bits(), high.to_bits(), "{}", control.id);
            assert_eq!(
                bound(default).to_bits(),
                untouched.to_bits(),
                "{}",
                control.id
            );
        }
        // The strength alone sets only itself; the others take the operation's defaults.
        assert_eq!(
            bind(&[("strength", 30.0)]),
            [Setting::new(GRAIN, "strength", Value::Float(30.0))]
        );
        let image = crate::testing::image();
        assert!(check::switched_defaults(&Grain, OPERATIONS, &image).is_empty());
    }

    #[test]
    fn at_its_defaults_it_sets_nothing() {
        let image = crate::testing::image();
        let base = Base::new(OPERATIONS, &image);
        assert!(Grain.bind(&ModuleValues::new(), &image, &base).is_empty());
        let defaults: Vec<(&str, f32)> = CONTROLS
            .iter()
            .map(|c| match c.kind {
                ControlKind::Number { default, .. } => (c.id, default),
                _ => unreachable!(),
            })
            .collect();
        assert!(bind(&defaults).is_empty());
    }

    /// Each control sets its parameter alone, whatever the strength holds: grain is
    /// switched, so the session plans it while the module is on, not while the strength is
    /// off 0.
    #[test]
    fn each_control_sets_its_parameter_alone() {
        assert_eq!(
            bind(&[("scale", 3200.0)]),
            [Setting::new(GRAIN, "scale", Value::Float(scale(3200.0)))]
        );
        assert_eq!(
            bind(&[("midtones_bias", 0.0)]),
            [Setting::new(GRAIN, "midtones_bias", Value::Float(0.0))]
        );
        assert_eq!(
            bind(&[("strength", 0.0)]),
            [Setting::new(GRAIN, "strength", Value::Float(0.0))]
        );
    }

    /// Version 1's edits: on where the strength was off 0, with their values; else off,
    /// their values dropped and their marks kept.
    #[test]
    fn version_1_s_edits_migrate_on_where_they_rendered() {
        let v1 = |set: &[(&str, f32)]| {
            let mut values = ModuleValues::new();
            for &(control, value) in set {
                values.set(control, ControlValue::Number(value));
                values.set_mark(control, Some(crate::Mark::Kept));
            }
            Grain.migrate(1, values)
        };
        let on = v1(&[("strength", 37.3), ("scale", 800.0)]);
        assert!(on.enabled());
        assert_eq!(on.number("strength"), Some(37.3));
        assert_eq!(on.number("scale"), Some(800.0));
        for set in [&[("scale", 3200.0)][..], &[("strength", 0.0)], &[]] {
            let off = v1(set);
            assert!(!off.enabled(), "{set:?}");
            assert_eq!(off.values().count(), 0, "{set:?}");
            assert_eq!(off.marks().count(), set.len(), "{set:?}");
        }
    }

    #[test]
    fn the_coarseness_binds_as_darktable_s_slider_converts_iso() {
        let settings = bind(&[
            ("scale", 800.0),
            ("strength", 37.3),
            ("midtones_bias", 52.7),
        ]);
        assert_eq!(
            value(&settings, "scale"),
            Some(Value::Float(800.0 / 213.2_f32))
        );
        assert_eq!(value(&settings, "strength"), Some(Value::Float(37.3)));
        assert_eq!(value(&settings, "midtones_bias"), Some(Value::Float(52.7)));
        assert_eq!(settings.len(), 3);
    }

    /// darktable's coarseness range in its parameter's units, as its params compute it,
    /// in double: `20.0/GRAIN_SCALE_FACTOR` to `6400.0/GRAIN_SCALE_FACTOR`.
    const SCALE: (f32, f32) = ((20.0 / 213.2_f64) as f32, (6400.0 / 213.2_f64) as f32);

    /// darktable's grain sliders (`src/iop/grain.c`, the params and `gui_init`), in
    /// its parameters' units: `docs/modules/grain.md`'s slider table.
    const DARKTABLE: [DarktableSlider; 3] = [
        // `dt_bauhaus_slider_set_factor(g->scale, GRAIN_SCALE_FACTOR)`, no digits, " ISO".
        DarktableSlider::new("scale", SCALE, 0).shown_as(213.2, 0.0),
        // `dt_bauhaus_slider_set_format(…, "%")` over a hard maximum of 100, beyond ±10,
        // so no factor of 100 and the two digits `dt_bauhaus_slider_from_params` gives.
        DarktableSlider::new("strength", (0.0, 100.0), 2),
        DarktableSlider::new("midtones_bias", (0.0, 100.0), 2),
    ];

    #[test]
    fn its_sliders_are_darktable_s() {
        let differences =
            crate::testing::slider_differences(&Grain, &DEFINITION.defaults(), &DARKTABLE);
        assert!(differences.is_empty(), "{differences:#?}");
        // darktable's defaults, its params' introspection defaults.
        let defaults = DEFINITION.defaults();
        assert_eq!(defaults.number("scale"), 1600.0);
        assert_eq!(defaults.number("strength"), 25.0);
        assert_eq!(defaults.number("midtones_bias"), 100.0);
        // The factor darktable's coarseness shows by is the one the binding undoes.
        assert_eq!(DARKTABLE[0].factor, SCALE_FACTOR);
        assert_eq!(scale(iso(SCALE.1)), SCALE.1);
    }
}
