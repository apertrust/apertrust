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

//! Haze removal (`docs/modules/haze-removal.md`): darktable's haze removal as the
//! user edits with it, in the first pass (decision 2): darktable's page, its two sliders
//! in `gui_init`'s order under its labels, Strength and Distance. It has no tabs, groups
//! or presets of its own; darktable's default for a new instance is to be its "darktable's
//! default" preset once the header's presets menu comes.
//!
//! Each control is one `hazeremoval` parameter as it is, as darktable's slider shows it.
//! The module drives `hazeremoval`, which an unedited photo does not run, planned only
//! while Strength is off 0, whatever Distance holds. Strength starts at 0, where the
//! operation removes nothing, not at darktable's 0.2, which removes haze from nearly
//! every pixel (rule 4 of `docs/modules/README.md`); Distance at darktable's 0.2, which
//! changes nothing while Strength is 0. darktable's two flags,
//! `compatibility_mode` and `adaptive`, have no widget and are the port's fixed values, a
//! new instance's: off and on (`docs/ports/hazeremoval.md`, Parameters).

use apertrust_plugin::{ImageContext, Value};

use crate::definition::{Control, ControlKind, Definition, Row, Slider, Tab};
use crate::{Base, Module, ModuleValues, Owned, Setting};

pub struct HazeRemoval;

/// The operation it drives.
const HAZEREMOVAL: &str = "hazeremoval";

/// The controls, darktable's parameter names.
pub const STRENGTH: &str = "strength";
pub const DISTANCE: &str = "distance";

/// The untouched strength: no haze removed, and no operation planned. The operation's
/// own default is darktable's 0.2, so the binding writes the strength whenever it plans
/// it.
const UNTOUCHED: f32 = 0.0;

/// darktable's distance for a new instance, the operation's default too.
const DISTANCE_DEFAULT: f32 = 0.2;

/// darktable's hard ranges, the params' `$MIN` and `$MAX`, are the controls'; it sets no
/// soft range, so the tracks span them. Each steps by what `dt_bauhaus_slider_get_step`
/// gives a range of 1 at its larger end, 0.01.
static CONTROLS: [Control; 2] = [
    Control {
        id: STRENGTH,
        kind: ControlKind::Number {
            min: -1.0,
            max: 1.0,
            step: 0.01,
            default: UNTOUCHED,
        },
        unit: None,
        unported: false,
    },
    Control {
        id: DISTANCE,
        kind: ControlKind::Number {
            min: 0.0,
            max: 1.0,
            step: 0.01,
            default: DISTANCE_DEFAULT,
        },
        unit: None,
        unported: false,
    },
];

static DEFINITION: Definition = Definition {
    id: HAZEREMOVAL,
    version: 1,
    layers: true,
    auto: false,
    controls: &CONTROLS,
    tabs: &[Tab {
        id: None,
        rows: &[
            // `dt_bauhaus_slider_from_params`' two decimals.
            Row::Slider(Slider::new(STRENGTH)),
            // `dt_bauhaus_slider_set_digits(g->distance, 3)`.
            Row::Slider(Slider::new(DISTANCE).decimals(3)),
        ],
    }],
    before: &[],
    after: &[],
    presets: &[],
    views: &[],
};

/// Both parameters of `hazeremoval`: the module is darktable's and owns them all.
static OWNS: [Owned; 2] = [
    Owned::new(HAZEREMOVAL, STRENGTH),
    Owned::new(HAZEREMOVAL, DISTANCE),
];

impl Module for HazeRemoval {
    fn definition(&self) -> &'static Definition {
        &DEFINITION
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    /// The strength off 0 plans the operation, whatever the distance holds: at a strength
    /// of 0 the operation removes nothing, so it stays out of the plan, as grain's does at
    /// an amount of 0 (`tool-bindings`, Which operations run). The strength is then always
    /// written, and the distance while it is off its default, which is the operation's.
    fn bind(&self, values: &ModuleValues, _image: &ImageContext, _base: &Base) -> Vec<Setting> {
        let Some(strength) = values.number(STRENGTH) else {
            return Vec::new();
        };
        let mut settings = vec![Setting::new(HAZEREMOVAL, STRENGTH, Value::Float(strength))];
        if let Some(distance) = values.number(DISTANCE) {
            settings.push(Setting::new(HAZEREMOVAL, DISTANCE, Value::Float(distance)));
        }
        settings
    }
}

#[cfg(test)]
mod tests {
    use apertrust_operations::OPERATIONS;
    use apertrust_operations::hazeremoval::HazeRemoval as HazeOperation;
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
        let values = values.off_defaults(&HazeRemoval.defaults(&image));
        HazeRemoval.bind(&values, &image, &Base::new(OPERATIONS, &image))
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
        let (accepted, rejected) = check::accept(&[&HazeRemoval], OPERATIONS);
        assert!(rejected.is_empty(), "{rejected:?}");
        assert_eq!(accepted.len(), 1);
        for image in [crate::testing::image(), ImageContext::full(8, 8)] {
            let problems = check::sweep(&HazeRemoval, OPERATIONS, &image);
            assert!(problems.is_empty(), "{problems:?}");
        }
    }

    #[test]
    fn its_controls_are_darktable_s_sliders_in_gui_init_s_order() {
        let shown: Vec<&str> = DEFINITION
            .rows()
            .flat_map(|row| row.controls().iter().copied())
            .collect();
        assert_eq!(shown, [STRENGTH, DISTANCE]);
        let controls: Vec<&str> = CONTROLS.iter().map(|c| c.id).collect();
        assert_eq!(controls, shown);
    }

    /// It owns both parameters, each control's range the parameter's; the distance's
    /// default is the operation's, and the strength's is 0, not darktable's 0.2.
    #[test]
    fn it_owns_every_parameter_of_the_operation_each_with_one_control() {
        let schema = HazeOperation.schema();
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
                default: darktable,
            } = schema.param(control.id).unwrap().kind
            else {
                panic!("{} is a float", control.id);
            };
            assert_eq!((min, max), (low, high), "{}", control.id);
            let expected = if control.id == STRENGTH {
                UNTOUCHED
            } else {
                darktable
            };
            assert_eq!(default, expected, "{}", control.id);
        }
    }

    /// darktable's haze removal sliders (`src/iop/hazeremoval.c`, the params and
    /// `gui_init`): `docs/modules/haze-removal.md`'s slider table. Both from params, with
    /// no soft range, factor or offset; the distance with three digits.
    const DARKTABLE: [DarktableSlider; 2] = [
        DarktableSlider::new(STRENGTH, (-1.0, 1.0), 2),
        DarktableSlider::new(DISTANCE, (0.0, 1.0), 3),
    ];

    #[test]
    fn its_sliders_are_darktable_s() {
        let differences =
            crate::testing::slider_differences(&HazeRemoval, &DEFINITION.defaults(), &DARKTABLE);
        assert!(differences.is_empty(), "{differences:#?}");
    }

    #[test]
    fn at_its_defaults_it_sets_nothing() {
        let image = crate::testing::image();
        let base = Base::new(OPERATIONS, &image);
        assert!(
            HazeRemoval
                .bind(&ModuleValues::new(), &image, &base)
                .is_empty()
        );
        assert!(bind(&[(STRENGTH, 0.0), (DISTANCE, 0.2)]).is_empty());
    }

    #[test]
    fn each_control_binds_its_parameter_as_it_is() {
        let settings = bind(&[(STRENGTH, 0.9), (DISTANCE, 0.8)]);
        assert_eq!(value(&settings, STRENGTH), Some(Value::Float(0.9)));
        assert_eq!(value(&settings, DISTANCE), Some(Value::Float(0.8)));
        assert_eq!(settings.len(), 2);
        // The strength alone leaves the distance at the operation's default, the
        // module's.
        let settings = bind(&[(STRENGTH, -0.35)]);
        assert_eq!(value(&settings, STRENGTH), Some(Value::Float(-0.35)));
        assert_eq!(settings.len(), 1);
    }

    /// The distance alone, the strength at 0, binds nothing, so it plans no operation,
    /// at either end of its range.
    #[test]
    fn the_distance_alone_binds_nothing() {
        for distance in [0.0, 0.65, 1.0] {
            assert!(bind(&[(DISTANCE, distance)]).is_empty(), "{distance}");
            assert!(
                bind(&[(STRENGTH, 0.0), (DISTANCE, distance)]).is_empty(),
                "{distance}"
            );
        }
    }
}
