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

//! Raw black/white point (`docs/modules/raw-black-white-point.md`): darktable's raw
//! black/white point as the user edits with it, in the first pass (decision 2):
//! darktable's page, its four black levels and its white point in `gui_init`'s order under
//! its labels, then flat field correction. It has no tabs, groups or presets of its own,
//! and works on the background only, as the whole raw stage does.
//!
//! Each level is one `rawprepare` parameter, a whole raw value from 0 to 65535 shown as it
//! is, with no decimals, as darktable's sliders show its `uint16_t` fields; their tracks
//! span darktable's soft range, 0 to 16384. A black level is the black of one position of
//! the sensor's 2×2 pattern of row and column parities, row parity × 2 + column parity.
//!
//! It drives `rawprepare`, which every raw runs, so each level starts at the operation's
//! default for the photo (rule 4 of `docs/modules/README.md`, decision 21), the raw's own
//! black levels and white point as darktable's `reload_defaults` takes them from the
//! decoder, and the binding sets a parameter only while its control is off it. Flat field
//! correction lists darktable's two options, disabled and the embedded GainMap, which is
//! not ported: rawler reads no gain maps, so no photo can have one, and the control is
//! drawn disabled on every photo, where darktable hides it. darktable's crop margins, which
//! it shows only while its preference to edit the raw crop is on, off by default, are not
//! shown (rule 12): the module owns them at the raw's values and never sets them.
//!
//! A change runs demosaic again, as every raw-stage control does (decision 12).

use apertrust_plugin::{ImageContext, Value};

use crate::definition::{ChoiceStyle, Control, ControlKind, Definition, Row, Slider, Tab};
use crate::{Base, ControlValue, Controls, Module, ModuleValues, Owned, Setting};

pub struct RawBlackWhitePoint;

/// The operation it drives.
const RAWPREPARE: &str = "rawprepare";

/// The black levels, by parity position: darktable's `raw_black_level_separate[0]` to
/// `[3]`, as the operation names them, so the control ids are its parameter names.
pub const BLACK: [&str; 4] = [
    "raw_black_level_separate_0",
    "raw_black_level_separate_1",
    "raw_black_level_separate_2",
    "raw_black_level_separate_3",
];

/// The white point, the raw value that becomes 1.
pub const WHITE: &str = "raw_white_point";

/// Flat field correction: off, or the embedded GainMap, not ported.
const FLAT_FIELD: &str = "flat_field";

/// The largest level, darktable's `UINT16_MAX`, the parameters' maximum.
const MAX_LEVEL: f32 = 65535.0;

/// darktable's soft maximum for every level, `dt_bauhaus_slider_set_soft_max(…, 16384)`:
/// the track spans 0 to 16384 at rest.
const SOFT_MAX: f32 = 16384.0;

/// A level: a whole raw value from 0 to 65535, stepping by 1, as
/// `dt_bauhaus_slider_from_params` makes a `uint16_t`'s slider. `default` is the one for
/// an image that is not a raw, the operation's schema's; a raw's comes from the photo
/// ([`RawBlackWhitePoint::defaults`]).
const fn level(id: &'static str, default: f32) -> Control {
    Control {
        id,
        kind: ControlKind::Number {
            min: 0.0,
            max: MAX_LEVEL,
            step: 1.0,
            default,
        },
        unit: None,
        unported: false,
    }
}

/// The controls, in the page's order. Without a camera the levels default to darktable's
/// passthrough preset, black 0 and white 65535, as the operation's schema does.
static CONTROLS: [Control; 6] = [
    level(BLACK[0], 0.0),
    level(BLACK[1], 0.0),
    level(BLACK[2], 0.0),
    level(BLACK[3], 0.0),
    level(WHITE, MAX_LEVEL),
    Control {
        id: FLAT_FIELD,
        // darktable's `dt_iop_rawprepare_flat_field_t`, as the operation's choices.
        kind: ControlKind::Choice {
            options: &["off", "embedded"],
            default: 0,
            unported: &["embedded"],
        },
        unit: None,
        unported: false,
    },
];

/// A level's row: no decimals, the track over darktable's soft range.
const fn slider(control: &'static str) -> Row {
    Row::Slider(Slider::new(control).decimals(0).soft(0.0, SOFT_MAX))
}

static DEFINITION: Definition = Definition {
    id: "rawprepare",
    version: 1,
    layers: false,
    auto: false,
    controls: &CONTROLS,
    tabs: &[Tab {
        id: None,
        rows: &[
            slider(BLACK[0]),
            slider(BLACK[1]),
            slider(BLACK[2]),
            slider(BLACK[3]),
            slider(WHITE),
            Row::Choice {
                control: FLAT_FIELD,
                style: ChoiceStyle::List,
            },
        ],
    }],
    before: &[],
    after: &[],
    presets: &[],
    views: &[],
};

/// Every parameter of `rawprepare`: the module is darktable's and owns them all, the crop
/// margins first, at the raw's values, with no control ([`CROP`]).
static OWNS: [Owned; 10] = [
    Owned::new(RAWPREPARE, "left"),
    Owned::new(RAWPREPARE, "top"),
    Owned::new(RAWPREPARE, "right"),
    Owned::new(RAWPREPARE, "bottom"),
    Owned::new(RAWPREPARE, BLACK[0]),
    Owned::new(RAWPREPARE, BLACK[1]),
    Owned::new(RAWPREPARE, BLACK[2]),
    Owned::new(RAWPREPARE, BLACK[3]),
    Owned::new(RAWPREPARE, WHITE),
    Owned::new(RAWPREPARE, FLAT_FIELD),
];

/// How many of [`OWNS`] are the crop margins, which the module fixes.
const CROP: usize = 4;

impl Module for RawBlackWhitePoint {
    fn definition(&self) -> &'static Definition {
        &DEFINITION
    }

    /// The definition's, but a raw's levels are its own ([`levels`]), the operation's
    /// defaults for it.
    fn defaults(&self, image: &ImageContext) -> Controls {
        let mut defaults = DEFINITION.defaults();
        if let Some((black, white)) = levels(image) {
            for (id, level) in BLACK.into_iter().zip(black) {
                defaults.set(id, ControlValue::Number(level));
            }
            defaults.set(WHITE, ControlValue::Number(white));
        }
        defaults
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    /// The crop margins, the sensor's geometry, which darktable shows only under a
    /// preference that is off by default (rule 12).
    fn fixed(&self) -> &'static [Owned] {
        &OWNS[..CROP]
    }

    /// Flat field correction changes nothing on any photo: its only ported option is off,
    /// and darktable shows it only for a raw with usable gain maps, which rawler does not
    /// read. The levels change nothing on an image that is not a raw, which runs no raw
    /// stage, where darktable shows its "only works for the sensors that need it" page in
    /// place of the sliders (`gui_init`, `reload_defaults`). Neither depends on the
    /// module's values.
    fn available(&self, control: &str, _: &Controls, image: &ImageContext) -> bool {
        control != FLAT_FIELD && image.camera.is_some()
    }

    /// Each level off its default for the photo sets its parameter, rounded to the whole
    /// number darktable's `uint16_t` field holds; a level at its default leaves the
    /// operation's own, the same value. Flat field correction takes only off, its
    /// default, so it never sets anything, and the crop margins have no control.
    fn bind(&self, values: &ModuleValues, _image: &ImageContext, _base: &Base) -> Vec<Setting> {
        BLACK
            .into_iter()
            .chain([WHITE])
            .filter_map(|id| {
                let level = values.number(id)?;
                Some(Setting::new(
                    RAWPREPARE,
                    id,
                    Value::Int(level.round() as i64),
                ))
            })
            .collect()
    }
}

/// The raw's black levels by parity position and its white point, as the rawprepare
/// operation's defaults take them from the decoder (darktable's `reload_defaults`): each
/// black the camera's black pattern at the position's row and column, the pattern
/// repeating across the 2×2 one, and every level rounded to a whole number in 16 bits.
/// An empty black pattern leaves the blacks at 0, as the operation's schema does. `None`
/// for an image that is not a raw, whose levels are the schema's, darktable's passthrough
/// preset. This crate cannot call the operation's own, so it is restated here; the
/// module's tests check the two agree.
pub fn levels(image: &ImageContext) -> Option<([f32; 4], f32)> {
    let camera = image.camera.as_ref()?;
    let pattern = &camera.black;
    let mut black = [0.0; 4];
    if pattern.width > 0 && pattern.height > 0 && !pattern.values.is_empty() {
        for (position, level) in black.iter_mut().enumerate() {
            let (column, row) = (position as u32 & 1, position as u32 >> 1);
            let at = (row % pattern.height) * pattern.width + column % pattern.width;
            *level = whole(pattern.values.get(at as usize).copied().unwrap_or(0.0));
        }
    }
    Some((black, whole(camera.white)))
}

/// A level as darktable's parameters hold it: the nearest whole number, within 16 bits.
/// The cast saturates, so a level below 0 is 0, one above 65535 is 65535 and NaN is 0, as
/// the operation's clamp and cast give.
fn whole(value: f32) -> f32 {
    f32::from(value.round() as u16)
}

#[cfg(test)]
mod tests {
    use apertrust_operations::OPERATIONS;
    use apertrust_operations::rawprepare::{BLACK as PARAMS, Rawprepare};
    use apertrust_plugin::{Kind, Operation, ParamValues, Pattern};

    use super::*;
    use crate::check;
    use crate::testing::DarktableSlider;

    /// A raw's context with this black pattern and white point.
    fn raw(black: Pattern<f32>, white: f32) -> ImageContext {
        let mut image = crate::testing::image();
        let camera = image.camera.as_mut().unwrap();
        (camera.black, camera.white) = (black, white);
        image
    }

    fn pattern(width: u32, height: u32, values: &[f32]) -> Pattern<f32> {
        Pattern {
            width,
            height,
            values: values.to_vec(),
        }
    }

    /// Contexts whose levels differ: the test raw's, a uniform black that rounds, the
    /// 40D's rows as darktable measures them, a 1×2 pattern of columns, levels beyond
    /// 16 bits either way, an empty pattern and an image that is not a raw.
    fn images() -> Vec<ImageContext> {
        vec![
            crate::testing::image(),
            raw(pattern(1, 1, &[1023.4]), 13600.6),
            raw(pattern(2, 2, &[1030.0, 1030.0, 1026.0, 1026.0]), 13600.0),
            raw(pattern(2, 1, &[500.0, 510.5]), 4095.0),
            raw(pattern(1, 1, &[-3.0]), 1e9),
            raw(pattern(0, 0, &[]), 16383.0),
            ImageContext::full(8, 8),
        ]
    }

    /// What the binding sets for these values, as a session binds them.
    fn bind(set: &[(&'static str, f32)], image: &ImageContext) -> Vec<Setting> {
        let mut values = ModuleValues::new();
        for &(control, value) in set {
            values.set(control, ControlValue::Number(value));
        }
        let values = values.off_defaults(&RawBlackWhitePoint.defaults(image));
        RawBlackWhitePoint.bind(&values, image, &Base::new(OPERATIONS, image))
    }

    #[test]
    fn the_definition_holds_and_its_binding_sweeps_inside_the_schema() {
        DEFINITION.check().unwrap();
        let (accepted, rejected) = check::accept(&[&RawBlackWhitePoint], OPERATIONS);
        assert!(rejected.is_empty(), "{rejected:?}");
        assert_eq!(accepted.len(), 1);
        for image in images() {
            let problems = check::sweep(&RawBlackWhitePoint, OPERATIONS, &image);
            assert!(problems.is_empty(), "{problems:?}");
        }
    }

    #[test]
    fn its_page_is_darktable_s_in_gui_init_s_order_on_one_untitled_tab() {
        assert_eq!(DEFINITION.tabs.len(), 1);
        assert_eq!(DEFINITION.tabs[0].id, None);
        let shown: Vec<&str> = DEFINITION
            .rows()
            .flat_map(|row| row.controls().iter().copied())
            .collect();
        assert_eq!(
            shown,
            [
                "raw_black_level_separate_0",
                "raw_black_level_separate_1",
                "raw_black_level_separate_2",
                "raw_black_level_separate_3",
                "raw_white_point",
                "flat_field",
            ]
        );
        let controls: Vec<&str> = CONTROLS.iter().map(|c| c.id).collect();
        assert_eq!(controls, shown);
        assert!(!DEFINITION.layers && !DEFINITION.auto);
    }

    /// The module owns every parameter of `rawprepare`, each level's control named as its
    /// parameter, with the parameter's range; the crop margins have no control.
    #[test]
    fn it_owns_every_parameter_of_the_operation_and_its_levels_ranges_are_the_schema_s() {
        let schema = Rawprepare.schema();
        let owned: Vec<&str> = OWNS.iter().map(|owned| owned.param).collect();
        let params: Vec<&str> = schema.params.iter().map(|param| param.name).collect();
        assert_eq!(owned, params);
        assert_eq!(BLACK, PARAMS);
        for id in BLACK.into_iter().chain([WHITE]) {
            let Some(ControlKind::Number { min, max, .. }) =
                DEFINITION.control(id).map(|control| control.kind)
            else {
                panic!("{id} is a number");
            };
            let Kind::Int {
                min: low,
                max: high,
                ..
            } = schema.param(id).unwrap().kind
            else {
                panic!("{id} is a whole number");
            };
            assert_eq!((min as i64, max as i64), (low, high), "{id}");
        }
        for margin in ["left", "top", "right", "bottom"] {
            assert!(DEFINITION.control(margin).is_none(), "{margin}");
        }
    }

    /// Every default is the operation's for the photo: the levels its `defaults`, the
    /// restatement agreeing with it on every context, and flat field off.
    #[test]
    fn its_defaults_are_the_operation_s_for_the_photo() {
        for image in images() {
            let ours = RawBlackWhitePoint.defaults(&image);
            let theirs = operation_defaults(&image);
            for id in BLACK.into_iter().chain([WHITE]) {
                assert_eq!(ours.number(id) as i64, theirs.int(id), "{id}");
            }
            assert_eq!(ours.choice(FLAT_FIELD), theirs.choice(FLAT_FIELD));
        }
        let defaults = |image: &ImageContext| {
            let defaults = RawBlackWhitePoint.defaults(image);
            (BLACK.map(|id| defaults.number(id)), defaults.number(WHITE))
        };
        assert_eq!(
            defaults(&images()[2]),
            ([1030.0, 1030.0, 1026.0, 1026.0], 13600.0)
        );
        assert_eq!(
            defaults(&images()[3]),
            ([500.0, 511.0, 500.0, 511.0], 4095.0)
        );
        assert_eq!(defaults(&images()[4]), ([0.0; 4], 65535.0));
        assert_eq!(defaults(&ImageContext::full(8, 8)), ([0.0; 4], 65535.0));
    }

    /// The operation's defaults for the photo, every parameter resolved over its schema.
    fn operation_defaults(image: &ImageContext) -> ParamValues {
        Rawprepare
            .schema()
            .resolve_over(&Rawprepare.defaults(image), &ParamValues::new())
            .unwrap()
    }

    #[test]
    fn at_its_defaults_it_sets_nothing() {
        for image in images() {
            let base = Base::new(OPERATIONS, &image);
            assert!(
                RawBlackWhitePoint
                    .bind(&ModuleValues::new(), &image, &base)
                    .is_empty()
            );
            // Every level written at the photo's default is still nothing to set.
            let defaults = RawBlackWhitePoint.defaults(&image);
            let written: Vec<(&str, f32)> = BLACK
                .into_iter()
                .chain([WHITE])
                .map(|id| (id, defaults.number(id)))
                .collect();
            assert!(bind(&written, &image).is_empty());
        }
    }

    #[test]
    fn each_level_off_its_default_sets_its_parameter_as_a_whole_number() {
        let image = images()[2].clone();
        assert_eq!(
            bind(
                &[
                    ("raw_black_level_separate_0", 1030.0),
                    ("raw_black_level_separate_2", 1023.0),
                    ("raw_white_point", 16383.4),
                ],
                &image
            ),
            [
                Setting::new(RAWPREPARE, BLACK[2], Value::Int(1023)),
                Setting::new(RAWPREPARE, WHITE, Value::Int(16383)),
            ]
        );
        // Every level set, each to its own value, in the parameters' order.
        let set: Vec<(&str, f32)> = BLACK
            .into_iter()
            .chain([WHITE])
            .zip([0.0, 1.0, 64.0, 65535.0, 12000.0])
            .collect();
        let settings = bind(&set, &image);
        let expected: Vec<Setting> = set
            .iter()
            .map(|&(id, level)| Setting::new(RAWPREPARE, id, Value::Int(level as i64)))
            .collect();
        assert_eq!(settings, expected);
    }

    #[test]
    fn flat_field_takes_only_off_and_is_never_available() {
        let flat_field = DEFINITION.control(FLAT_FIELD).unwrap();
        assert_eq!(
            flat_field.check(&ControlValue::Choice(1)),
            Err(crate::ControlError::Unported {
                control: FLAT_FIELD,
                option: "embedded"
            })
        );
        assert_eq!(flat_field.check(&ControlValue::Choice(0)), Ok(()));
        for image in images() {
            let defaults = RawBlackWhitePoint.defaults(&image);
            assert!(!RawBlackWhitePoint.available(FLAT_FIELD, &defaults, &image));
            // The levels act on a raw only, whatever the values.
            for id in BLACK.into_iter().chain([WHITE]) {
                assert_eq!(
                    RawBlackWhitePoint.available(id, &defaults, &image),
                    image.camera.is_some(),
                    "{id}"
                );
            }
        }
    }

    #[test]
    fn levels_are_whole_16_bit_numbers() {
        assert_eq!(whole(28.9), 29.0);
        assert_eq!(whole(28.5), 29.0);
        assert_eq!(whole(-3.0), 0.0);
        assert_eq!(whole(1e9), 65535.0);
        assert_eq!(whole(f32::NAN), 0.0);
    }

    /// darktable's raw black/white point sliders (`src/iop/rawprepare.c`, the params
    /// and `gui_init`), in its parameters' units: `docs/modules/raw-black-white-point.md`'s
    /// slider table. Each is `dt_bauhaus_slider_from_params` over a `uint16_t`, 0 to
    /// `UINT16_MAX`, no digits and a step of 1, with `dt_bauhaus_slider_set_soft_max(…,
    /// 16384)`. The crop's four, shown only under darktable's preference, are not shown.
    const DARKTABLE: [DarktableSlider; 5] = [
        DarktableSlider::whole("raw_black_level_separate_0", (0.0, 65535.0)).soft(0.0, 16384.0),
        DarktableSlider::whole("raw_black_level_separate_1", (0.0, 65535.0)).soft(0.0, 16384.0),
        DarktableSlider::whole("raw_black_level_separate_2", (0.0, 65535.0)).soft(0.0, 16384.0),
        DarktableSlider::whole("raw_black_level_separate_3", (0.0, 65535.0)).soft(0.0, 16384.0),
        DarktableSlider::whole("raw_white_point", (0.0, 65535.0)).soft(0.0, 16384.0),
    ];

    #[test]
    fn its_sliders_are_darktable_s() {
        let differences = crate::testing::slider_differences(
            &RawBlackWhitePoint,
            &DEFINITION.defaults(),
            &DARKTABLE,
        );
        assert!(differences.is_empty(), "{differences:#?}");
    }

    /// The restatement's position of each black in the camera's pattern is the operation's:
    /// a pattern of four distinct levels comes back in the same order from both.
    #[test]
    fn a_black_pattern_reads_at_the_same_positions_as_the_operation_s() {
        let image = raw(pattern(2, 2, &[1.0, 2.0, 3.0, 4.0]), 100.0);
        let ours = RawBlackWhitePoint.defaults(&image);
        let theirs = operation_defaults(&image);
        assert_eq!(BLACK.map(|id| ours.number(id)), [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(PARAMS.map(|id| theirs.int(id)), [1, 2, 3, 4]);
        // A pattern of rows repeats across the columns; a white halfway rounds up.
        let image = raw(pattern(1, 2, &[600.0, 620.0]), 4095.5);
        let ours = RawBlackWhitePoint.defaults(&image);
        assert_eq!(
            BLACK.map(|id| ours.number(id)),
            [600.0, 600.0, 620.0, 620.0]
        );
        assert_eq!(
            PARAMS.map(|id| operation_defaults(&image).int(id)),
            [600, 600, 620, 620]
        );
        assert_eq!(ours.number(WHITE), 4096.0);
        assert_eq!(operation_defaults(&image).int(WHITE), 4096);
    }
}
