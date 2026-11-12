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
//! or presets of its own.
//!
//! Each control is one `hazeremoval` parameter as it is, as darktable's slider shows it.
//! The module is switched (decision 32), as darktable leaves haze removal off on a new
//! photo: off, it renders nothing; any write of a control turns it on, with darktable's
//! defaults for the other, a strength and a distance of 0.2, the operation's own, until
//! its reset turns it off. So the binding sets each control only while it is off its
//! default, and on at its defaults the session plans `hazeremoval` at the operation's
//! defaults, darktable's new instance. Version 1 started the strength at 0 and planned the
//! operation only while it was off 0; its edits migrate on where the strength was off 0,
//! and off, their values dropped, where it was not. darktable's two flags,
//! `compatibility_mode` and `adaptive`, have no widget and are the port's fixed values, a
//! new instance's: off and on (`docs/ports/hazeremoval.md`, Parameters).

use apertrust_plugin::{ImageContext, Value};

use crate::definition::{Control, ControlKind, Definition, Row, Slider, Tab};
use crate::{Base, Module, ModuleValues, Owned, Setting, switched_from};

pub struct HazeRemoval;

/// The operation it drives.
const HAZEREMOVAL: &str = "hazeremoval";

/// The controls, darktable's parameter names.
pub const STRENGTH: &str = "strength";
pub const DISTANCE: &str = "distance";

/// darktable's strength for a new instance, the operation's default too.
const STRENGTH_DEFAULT: f32 = 0.2;

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
            default: STRENGTH_DEFAULT,
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
    version: 2,
    layers: true,
    auto: false,
    switched: true,
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
    pickers: &[],
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

    /// Each control off its default sets its parameter, as it is; a control at its
    /// default leaves its parameter at the operation's default, which is the same value.
    fn bind(&self, values: &ModuleValues, _image: &ImageContext, _base: &Base) -> Vec<Setting> {
        CONTROLS
            .iter()
            .filter_map(|control| {
                let value = values.number(control.id)?;
                Some(Setting::new(HAZEREMOVAL, control.id, Value::Float(value)))
            })
            .collect()
    }

    /// Version 1 planned `hazeremoval` only while the strength was off 0, its default
    /// then, and wrote it whenever it did: such an edit is on, its values as they were,
    /// the strength among them; any other rendered nothing, so it is off, its values
    /// dropped.
    fn migrate(&self, from: u32, values: ModuleValues) -> ModuleValues {
        if from >= 2 {
            return values;
        }
        let rendered = values.number(STRENGTH).is_some_and(|s| s != 0.0);
        switched_from(values, rendered)
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

    /// It owns both parameters, each control's range and default the parameter's,
    /// darktable's, and its defaults bind the operation's.
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
            assert_eq!(default.to_bits(), darktable.to_bits(), "{}", control.id);
        }
        let image = crate::testing::image();
        assert!(check::switched_defaults(&HazeRemoval, OPERATIONS, &image).is_empty());
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
        assert!(bind(&[(STRENGTH, 0.2), (DISTANCE, 0.2)]).is_empty());
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

    /// The distance alone binds itself, whatever the strength holds: haze removal is
    /// switched, so the session plans it while the module is on, not while the strength
    /// is off 0.
    #[test]
    fn the_distance_alone_binds_itself() {
        for distance in [0.0, 0.65, 1.0] {
            assert_eq!(
                bind(&[(DISTANCE, distance)]),
                [Setting::new(HAZEREMOVAL, DISTANCE, Value::Float(distance))]
            );
        }
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
            HazeRemoval.migrate(1, values)
        };
        let on = v1(&[(STRENGTH, 0.2), (DISTANCE, 0.5)]);
        assert!(on.enabled());
        assert_eq!(on.number(STRENGTH), Some(0.2));
        assert_eq!(on.number(DISTANCE), Some(0.5));
        for set in [&[(DISTANCE, 0.8)][..], &[(STRENGTH, 0.0)], &[]] {
            let off = v1(set);
            assert!(!off.enabled(), "{set:?}");
            assert_eq!(off.values().count(), 0, "{set:?}");
            assert_eq!(off.marks().count(), set.len(), "{set:?}");
        }
    }
}
