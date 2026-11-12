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

//! Orientation (`docs/modules/orientation.md`): darktable's orientation as the user edits
//! with it, in the first pass (decision 2). It drives `flip`, which every photo runs, and
//! owns its one parameter, darktable's stored orientation, auto by default, the photo's
//! own orientation from its EXIF data, so an untouched photo shows the right way up and
//! the module sets nothing. It has no tabs or groups, and works on the background only.
//!
//! darktable's page, as the module has it: its transform label, a reset label of the
//! stored orientation, with darktable's four buttons inline, rotate 90° anticlockwise
//! and clockwise, flip horizontally and vertically, each composing the orientation shown
//! with its turn or mirror as darktable's `do_rotate`, `_flip_h` and `_flip_v` do
//! (`Module::action`), a `SetControl` of the orientation; then darktable's guides widget,
//! show guides with the global guide settings button at its end, no edit's, drawn
//! disabled until the module's guides come. darktable's presets wait for the
//! header's presets menu.

use apertrust_plugin::{ImageContext, Value};

use crate::definition::{Button, ButtonIcon, Control, ControlKind, Definition, Row, Tab};
use crate::{Base, ControlValue, Controls, Module, ModuleValues, Owned, Setting};

pub struct Orientation;

/// The operation it drives, and the module's id.
const FLIP: &str = "flip";

/// The control, the operation's parameter, darktable's `orientation`.
const ORIENTATION: &str = "orientation";

/// darktable's `dt_image_orientation_t` values, from -1, as the operation's choices name
/// them (`apertrust_operations::flip::ORIENTATIONS`): autodetect, no rotation, flip
/// vertically, flip horizontally, rotate 180°, transpose, rotate -90°, rotate 90° and
/// transverse. The option at index `i` is darktable's value `i - 1`.
pub const ORIENTATIONS: [&str; 9] = [
    "auto",
    "none",
    "flip_vertically",
    "flip_horizontally",
    "rotate_180",
    "transpose",
    "rotate_cw_90",
    "rotate_ccw_90",
    "transverse",
];

/// darktable's buttons, in `gui_init`'s order.
pub const ACTIONS: [&str; 4] = [
    "rotate_ccw",
    "rotate_cw",
    "flip_horizontally",
    "flip_vertically",
];

/// The stored orientation, auto by default, darktable's `reload_defaults`.
static CONTROLS: [Control; 1] = [Control {
    id: ORIENTATION,
    kind: ControlKind::Choice {
        options: &ORIENTATIONS,
        default: 0,
        unported: &[],
    },
    unit: None,
    unported: false,
}];

static DEFINITION: Definition = Definition {
    id: FLIP,
    version: 1,
    layers: false,
    auto: false,
    switched: false,
    controls: &CONTROLS,
    tabs: &[Tab {
        id: None,
        rows: &[
            Row::Buttons {
                control: ORIENTATION,
                buttons: &[
                    Button {
                        id: ACTIONS[0],
                        icon: ButtonIcon::RotateLeft,
                    },
                    Button {
                        id: ACTIONS[1],
                        icon: ButtonIcon::RotateRight,
                    },
                    Button {
                        id: ACTIONS[2],
                        icon: ButtonIcon::FlipHorizontal,
                    },
                    Button {
                        id: ACTIONS[3],
                        icon: ButtonIcon::FlipVertical,
                    },
                ],
            },
            // darktable's guides widget (`IOP_FLAGS_GUIDES_WIDGET`, `flags()`).
            Row::Guides,
        ],
    }],
    before: &[],
    after: &[],
    presets: &[],
    views: &[],
};

static OWNS: [Owned; 1] = [Owned::new(FLIP, ORIENTATION)];

impl Module for Orientation {
    fn definition(&self) -> &'static Definition {
        &DEFINITION
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    /// The orientation, written as the operation's choice of the same index, once off
    /// auto.
    fn bind(&self, values: &ModuleValues, _image: &ImageContext, _base: &Base) -> Vec<Setting> {
        values
            .choice(ORIENTATION)
            .filter(|&choice| choice != 0)
            .map(|choice| Setting::new(FLIP, ORIENTATION, Value::Choice(choice)))
            .into_iter()
            .collect()
    }

    fn available(&self, _control: &str, _: &Controls, _image: &ImageContext) -> bool {
        true
    }

    /// darktable's buttons on the orientation shown, auto the photo's own: the new
    /// orientation, written by its choice.
    fn action(&self, action: &str, values: &Controls, image: &ImageContext) -> Option<Controls> {
        let shown = match values.get(ORIENTATION) {
            Some(ControlValue::Choice(choice)) => *choice,
            _ => 0,
        };
        let orientation = match shown {
            0 => photo_orientation(image),
            choice => (choice - 1) as u8,
        };
        let turned = turn(action, orientation)?;
        let mut writes = Controls::new();
        writes.set(ORIENTATION, ControlValue::Choice(u32::from(turned) + 1));
        Some(writes)
    }
}

/// darktable's flip bits.
const FLIP_Y: u8 = 1;
const FLIP_X: u8 = 2;
const SWAP_XY: u8 = 4;

/// The photo's own orientation, darktable's `dt_image_orientation`: none for an image
/// that records none or is not a raw.
fn photo_orientation(image: &ImageContext) -> u8 {
    image
        .camera
        .as_ref()
        .map_or(0, |camera| camera.orientation & 7)
}

/// `orientation` after darktable's button `action`: `do_rotate` (`flip.c:525`), the flip
/// along the axis shown then the swap, anticlockwise flipping the columns shown and
/// clockwise the rows; `_flip_h` and `_flip_v` (563 and 579), the columns or the rows
/// shown, the stored axes exchanged where the axes swap. `None` for another action.
pub fn turn(action: &str, orientation: u8) -> Option<u8> {
    let swapped = orientation & SWAP_XY != 0;
    let (columns, rows) = if swapped {
        (FLIP_Y, FLIP_X)
    } else {
        (FLIP_X, FLIP_Y)
    };
    Some(match action {
        "rotate_ccw" => (orientation ^ columns) ^ SWAP_XY,
        "rotate_cw" => (orientation ^ rows) ^ SWAP_XY,
        "flip_horizontally" => orientation ^ columns,
        "flip_vertically" => orientation ^ rows,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use apertrust_operations::OPERATIONS;
    use apertrust_operations::flip as operation;
    use apertrust_plugin::{Kind, Operation};

    use super::*;
    use crate::{ControlValue, check};

    fn images() -> [ImageContext; 2] {
        [crate::testing::image(), ImageContext::full(8, 8)]
    }

    #[test]
    fn the_definition_holds_and_its_binding_sweeps_inside_the_schema() {
        DEFINITION.check().unwrap();
        let (accepted, rejected) = check::accept(&[&Orientation], OPERATIONS);
        assert!(rejected.is_empty(), "{rejected:?}");
        assert_eq!(accepted.len(), 1);
        for image in images() {
            let problems = check::sweep(&Orientation, OPERATIONS, &image);
            assert!(problems.is_empty(), "{problems:?}");
        }
    }

    /// The options are the operation's choices, darktable's values from -1 in order, and
    /// the module owns its one parameter.
    #[test]
    fn its_options_are_the_operation_s_and_it_owns_its_parameter() {
        let schema = operation::Flip.schema();
        let Kind::Choice { choices, default } = schema.param(ORIENTATION).unwrap().kind else {
            panic!("the orientation is a choice");
        };
        assert_eq!(choices, ORIENTATIONS);
        assert_eq!(choices, operation::ORIENTATIONS);
        assert_eq!(default, 0);
        let params: Vec<&str> = schema.params.iter().map(|p| p.name).collect();
        assert_eq!(params, [ORIENTATION]);
    }

    /// Row F3: auto writes nothing; every other option writes the operation's choice of
    /// the same index.
    #[test]
    fn f3_each_option_writes_the_operation_s_choice() {
        for image in images() {
            let base = Base::new(OPERATIONS, &image);
            for (index, _) in ORIENTATIONS.iter().enumerate() {
                let mut values = ModuleValues::new();
                values.set(ORIENTATION, ControlValue::Choice(index as u32));
                let values = values.off_defaults(&Orientation.defaults(&image));
                let written = Orientation.bind(&values, &image, &base);
                let expected = (index != 0)
                    .then(|| Setting::new(FLIP, ORIENTATION, Value::Choice(index as u32)));
                assert_eq!(written, expected.into_iter().collect::<Vec<_>>());
            }
        }
    }

    /// darktable's page: the transform row with its four buttons inline, then show
    /// guides.
    #[test]
    fn its_page_is_darktable_s() {
        let rows: Vec<&Row> = DEFINITION.rows().collect();
        let [Row::Buttons { control, buttons }, Row::Guides] = rows[..] else {
            panic!("{rows:?}");
        };
        assert_eq!(*control, ORIENTATION);
        let ids: Vec<&str> = buttons.iter().map(|button| button.id).collect();
        assert_eq!(ids, ACTIONS);
    }

    /// darktable's `do_rotate`, `_flip_h` and `_flip_v`, restated line for line from
    /// `flip.c` with its `dt_image_orientation_t` values.
    fn darktable(action: &str, mut orientation: u8) -> u8 {
        const ORIENTATION_FLIP_Y: u8 = 1;
        const ORIENTATION_FLIP_X: u8 = 2;
        const ORIENTATION_SWAP_XY: u8 = 4;
        const ORIENTATION_FLIP_VERTICALLY: u8 = ORIENTATION_FLIP_Y;
        const ORIENTATION_FLIP_HORIZONTALLY: u8 = ORIENTATION_FLIP_X;
        let do_rotate = |mut orientation: u8, cw: u32| {
            if cw == 0 {
                if orientation & ORIENTATION_SWAP_XY != 0 {
                    orientation ^= ORIENTATION_FLIP_Y;
                } else {
                    orientation ^= ORIENTATION_FLIP_X;
                }
            } else if orientation & ORIENTATION_SWAP_XY != 0 {
                orientation ^= ORIENTATION_FLIP_X;
            } else {
                orientation ^= ORIENTATION_FLIP_Y;
            }
            orientation ^ ORIENTATION_SWAP_XY
        };
        match action {
            "rotate_ccw" => do_rotate(orientation, 0),
            "rotate_cw" => do_rotate(orientation, 1),
            "flip_horizontally" => {
                if orientation & ORIENTATION_SWAP_XY != 0 {
                    orientation ^= ORIENTATION_FLIP_VERTICALLY;
                } else {
                    orientation ^= ORIENTATION_FLIP_HORIZONTALLY;
                }
                orientation
            }
            "flip_vertically" => {
                if orientation & ORIENTATION_SWAP_XY != 0 {
                    orientation ^= ORIENTATION_FLIP_HORIZONTALLY;
                } else {
                    orientation ^= ORIENTATION_FLIP_VERTICALLY;
                }
                orientation
            }
            other => panic!("{other}"),
        }
    }

    /// Row F6: each button on each of the eight orientations writes darktable's, and on
    /// auto composes with the photo's own, as darktable reads `ORIENTATION_NULL`.
    #[test]
    fn f6_each_button_composes_as_darktable_s() {
        let photo = |bits: u8| {
            let mut image = crate::testing::image();
            image.camera.as_mut().unwrap().orientation = bits;
            image
        };
        for action in ACTIONS {
            for bits in 0..8u8 {
                assert_eq!(
                    turn(action, bits),
                    Some(darktable(action, bits)),
                    "{action} {bits}"
                );
                // Shown as itself, or as auto on a photo whose EXIF gives it.
                for (choice, image) in [(u32::from(bits) + 1, photo(0)), (0, photo(bits))] {
                    let mut values = Orientation.defaults(&image);
                    values.set(ORIENTATION, ControlValue::Choice(choice));
                    let writes = Orientation.action(action, &values, &image).unwrap();
                    let expected = u32::from(darktable(action, bits)) + 1;
                    assert_eq!(
                        writes.get(ORIENTATION),
                        Some(&ControlValue::Choice(expected)),
                        "{action} on {choice}"
                    );
                    assert_eq!(writes.iter().count(), 1);
                }
            }
        }
        assert_eq!(turn("auto_tune", 0), None);
        // A quarter turn four times, or a mirror twice, comes back.
        for bits in 0..8u8 {
            let four = (0..4).fold(bits, |o, _| turn("rotate_cw", o).unwrap());
            assert_eq!(four, bits);
            let back = turn("rotate_ccw", turn("rotate_cw", bits).unwrap()).unwrap();
            assert_eq!(back, bits);
        }
    }
}
