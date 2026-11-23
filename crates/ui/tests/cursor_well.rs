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

//! The cursors' well (`docs/cursors.md`, The well, Group settings, Keyboard and screen
//! readers): its faces and chevrons pressed by the pointer, the keys and a screen reader,
//! each reported as the app applies it; its lists' lines; the one Tab stop and the arrows;
//! the names screen readers hear; and the gallery's Cursor well section and the lists open,
//! photographed at twice the size in the dark and the light theme. Accepted with
//! `UPDATE_SNAPSHOTS=1 cargo test -p apertrust-ui --test cursor_well`, reviewed like a golden
//! image.

mod common;

use apertrust_ui::components::{CursorWell, CursorWellAction, WellCursor, WellGroup, WellSetting};
use apertrust_ui::gallery::Gallery;
use apertrust_ui::{Builtin, Icon, Theme};
use egui::Key;
use egui::accesskit::{HasPopup, Role, Toggled};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, SnapshotResults};

/// The well's state, as the app keeps it, and what it reported.
struct State {
    chosen: usize,
    active: [usize; 4],
    show_last: bool,
    applies: usize,
    disabled: bool,
    width: f32,
    reported: Vec<CursorWellAction>,
}

impl Default for State {
    fn default() -> Self {
        State {
            chosen: 0,
            active: [0; 4],
            show_last: true,
            applies: 0,
            disabled: false,
            width: 0.0,
            reported: Vec::new(),
        }
    }
}

/// Phase 2's Select, Pan and Pick, then, from later phases, Copy and apply, a group of one
/// with a setting, between a button before it and one after, as the Toolbar's actions and
/// status slots stand round it.
fn well(state: State, which: Builtin) -> Harness<'static, State> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(360.0, 220.0))
        .with_pixels_per_point(2.0)
        .with_context_setup(move |ctx| apertrust_ui::install(ctx, Theme::builtin(which)))
        .build_ui_state(
            |ui, state: &mut State| {
                let panel = apertrust_ui::theme(ui.ctx()).colors.panel;
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                let select = [WellCursor::new(Icon::Select, "Select")
                    .key("Esc")
                    .how("Click a mark to select it")];
                let pan = [WellCursor::new(Icon::Pan, "Pan").how("Drag to pan")];
                let picks = [
                    WellCursor::new(Icon::PickWhiteBalance, "White balance picker")
                        .key("K")
                        .how("Click a neutral area, or drag one"),
                    WellCursor::new(Icon::PickExposure, "Exposure picker"),
                    WellCursor::new(Icon::Pick, "Colour readout").disabled("Comes later"),
                ];
                let pick_settings = [WellSetting::Toggle {
                    name: "Show the last pick on the photo",
                    on: state.show_last,
                }];
                let apply = [WellCursor::new(Icon::Apply, "Copy and apply").amounts(true)];
                let what = ["The whole edit", "The parts chosen"];
                let apply_settings = [WellSetting::Choice {
                    name: "A click applies",
                    options: &what,
                    chosen: state.applies,
                }];
                let groups = [
                    WellGroup::new("Select", &select, state.active[0]),
                    WellGroup::new("Pan", &pan, state.active[1]),
                    WellGroup::new("Pick", &picks, state.active[2]).settings(&pick_settings),
                    WellGroup::new("Copy and apply", &apply, state.active[3])
                        .settings(&apply_settings),
                ];
                egui::Frame::new().inner_margin(12.0).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let _ = ui.button("Before");
                        let mut well = CursorWell::new(&groups).chosen(state.chosen);
                        if state.disabled {
                            well = well.disabled("No photo is shown");
                        }
                        state.width = well.width(ui);
                        if let Some(action) = well.show(ui) {
                            match action {
                                CursorWellAction::Cursor { group, cursor } => {
                                    state.chosen = group;
                                    state.active[group] = cursor;
                                }
                                CursorWellAction::Toggle { on, .. } => state.show_last = on,
                                CursorWellAction::Choice { option, .. } => state.applies = option,
                                CursorWellAction::Amounts { .. }
                                | CursorWellAction::Again { .. } => {}
                            }
                            state.reported.push(action);
                        }
                        let _ = ui.button("After");
                    });
                });
            },
            state,
        );
    harness.run();
    harness
}

fn reported(harness: &mut Harness<'_, State>) -> Vec<CursorWellAction> {
    harness.run();
    std::mem::take(&mut harness.state_mut().reported)
}

fn focused(harness: &Harness<'_, State>) -> String {
    harness
        .get_by(|node| node.is_focused())
        .accesskit_node()
        .label()
        .unwrap_or_default()
}

const PICK_LIST: &str = "Pick cursors and settings";
const APPLY_LIST: &str = "Copy and apply settings";

#[test]
fn phase_2_s_three_groups_are_101_points_and_a_fourth_adds_its_button() {
    let harness = well(State::default(), Builtin::Dark);
    // Select 24, Pan 24, Pick 22 and 11, Copy and apply 22 and 11, 6 apart, 4 at each end.
    assert_eq!(harness.state().width, 101.0 + 6.0 + 33.0);
    let face = harness
        .get_by_role_and_label(Role::RadioButton, "Pan")
        .rect();
    assert_eq!((face.width(), face.height()), (24.0, 26.0));
    let face = harness
        .get_by_role_and_label(Role::RadioButton, "White balance picker, K")
        .rect();
    let chevron = harness
        .get_by_role_and_label(Role::Button, PICK_LIST)
        .rect();
    assert_eq!((face.width(), chevron.width()), (22.0, 11.0));
    // Right against it.
    assert_eq!(face.right(), chevron.left());
}

#[test]
fn a_face_chooses_its_group_s_active_cursor_and_the_chosen_face_is_pressed_again() {
    let mut harness = well(State::default(), Builtin::Dark);
    harness
        .get_by_role_and_label(Role::RadioButton, "Pan")
        .click();
    assert_eq!(
        reported(&mut harness),
        vec![CursorWellAction::Cursor {
            group: 1,
            cursor: 0
        }]
    );
    // Pan is chosen now: a click on it again, with no amounts, is a press again, which a
    // picker cursor's face turns its picker off with.
    harness
        .get_by_role_and_label(Role::RadioButton, "Pan")
        .click();
    assert_eq!(
        reported(&mut harness),
        vec![CursorWellAction::Again { group: 1 }]
    );
    harness
        .get_by_role_and_label(Role::RadioButton, "White balance picker, K")
        .click();
    assert_eq!(
        reported(&mut harness),
        vec![CursorWellAction::Cursor {
            group: 2,
            cursor: 0
        }]
    );
}

#[test]
fn a_chosen_face_with_amounts_opens_them_by_a_click_enter_or_down() {
    let mut harness = well(
        State {
            chosen: 3,
            ..State::default()
        },
        Builtin::Dark,
    );
    harness
        .get_by_role_and_label(Role::RadioButton, "Copy and apply")
        .click();
    assert_eq!(
        reported(&mut harness),
        vec![CursorWellAction::Amounts { group: 3 }]
    );
    harness.key_press(Key::Tab);
    harness.key_press(Key::Tab);
    harness.run();
    assert_eq!(focused(&harness), "Copy and apply");
    harness.key_press(Key::ArrowDown);
    assert_eq!(
        reported(&mut harness),
        vec![CursorWellAction::Amounts { group: 3 }]
    );
    harness.key_press(Key::Enter);
    assert_eq!(
        reported(&mut harness),
        vec![CursorWellAction::Amounts { group: 3 }]
    );
}

#[test]
fn a_chevron_opens_its_list_of_cursors_then_settings() {
    let mut harness = well(State::default(), Builtin::Dark);
    let chevron = harness.get_by_role_and_label(Role::Button, PICK_LIST);
    assert_eq!(
        chevron.accesskit_node().data().has_popup(),
        Some(HasPopup::Menu)
    );
    assert_eq!(chevron.accesskit_node().data().is_expanded(), Some(false));
    chevron.click();
    harness.run();
    assert_eq!(
        harness
            .get_by_role_and_label(Role::Button, PICK_LIST)
            .accesskit_node()
            .data()
            .is_expanded(),
        Some(true)
    );
    let menu = harness.get_by_role_and_label(Role::Menu, PICK_LIST);
    let lines: Vec<(String, Option<Toggled>, bool)> = menu
        .query_all_by_role(Role::MenuItemCheckBox)
        .map(|n| {
            let n = n.accesskit_node();
            (n.label().unwrap_or_default(), n.toggled(), n.is_disabled())
        })
        .collect();
    assert_eq!(
        lines,
        vec![
            ("White balance picker".into(), Some(Toggled::True), false),
            ("Exposure picker".into(), Some(Toggled::False), false),
            ("Colour readout".into(), Some(Toggled::False), true),
            (
                "Show the last pick on the photo".into(),
                Some(Toggled::True),
                false
            ),
        ]
    );
    // Its key at the line's end, for screen readers too.
    let white_balance =
        harness.get_by_role_and_label(Role::MenuItemCheckBox, "White balance picker");
    assert_eq!(
        white_balance.accesskit_node().data().keyboard_shortcut(),
        Some("K")
    );
    // A press on the chevron again closes it, choosing nothing.
    harness
        .get_by_role_and_label(Role::Button, PICK_LIST)
        .click();
    assert_eq!(reported(&mut harness), vec![]);
    assert!(
        harness
            .query_by_role_and_label(Role::Menu, PICK_LIST)
            .is_none()
    );
}

#[test]
fn a_line_of_a_list_chooses_a_cursor_or_a_setting() {
    let mut harness = well(State::default(), Builtin::Dark);
    harness
        .get_by_role_and_label(Role::Button, PICK_LIST)
        .click();
    harness.run();
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "Exposure picker")
        .click();
    assert_eq!(
        reported(&mut harness),
        vec![CursorWellAction::Cursor {
            group: 2,
            cursor: 1
        }]
    );
    // The face shows its group's active cursor now, chosen.
    let face = harness.get_by_role_and_label(Role::RadioButton, "Exposure picker");
    assert_eq!(face.accesskit_node().toggled(), Some(Toggled::True));

    harness
        .get_by_role_and_label(Role::Button, PICK_LIST)
        .click();
    harness.run();
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "Show the last pick on the photo")
        .click();
    assert_eq!(
        reported(&mut harness),
        vec![CursorWellAction::Toggle {
            group: 2,
            setting: 0,
            on: false
        }]
    );

    // A group of one lists its settings alone: the choice's heading over its options.
    harness
        .get_by_role_and_label(Role::Button, APPLY_LIST)
        .click();
    harness.run();
    let menu = harness.get_by_role_and_label(Role::Menu, APPLY_LIST);
    assert!(
        menu.query_by_role_and_label(Role::Label, "A click applies")
            .is_some()
    );
    assert!(
        menu.query_by_role_and_label(Role::MenuItemCheckBox, "Copy and apply")
            .is_none()
    );
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "The parts chosen")
        .click();
    assert_eq!(
        reported(&mut harness),
        vec![CursorWellAction::Choice {
            group: 3,
            setting: 0,
            option: 1
        }]
    );
}

#[test]
fn a_change_of_cursor_is_said_politely_by_its_name() {
    let mut harness = well(State::default(), Builtin::Dark);
    let said = |harness: &Harness<'_, State>| {
        let node = harness.get_by_role(Role::Status);
        let node = node.accesskit_node();
        assert_eq!(node.data().live(), Some(egui::accesskit::Live::Polite));
        node.label().unwrap_or_default()
    };
    assert_eq!(said(&harness), "Select");
    harness
        .get_by_role_and_label(Role::Button, PICK_LIST)
        .click();
    harness.run();
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "Exposure picker")
        .click();
    reported(&mut harness);
    harness.run();
    assert_eq!(said(&harness), "Exposure picker");
}

#[test]
fn a_chosen_cursor_that_is_not_ready_says_why() {
    let mut harness = well(
        State {
            chosen: 2,
            active: [0, 0, 2, 0],
            ..State::default()
        },
        Builtin::Dark,
    );
    harness.run();
    let face = harness.get_by_role_and_label(Role::RadioButton, "Colour readout");
    let face = face.accesskit_node();
    assert!(face.is_disabled());
    assert_eq!(face.toggled(), Some(Toggled::True));
    assert_eq!(face.description().as_deref(), Some("Comes later"));
}

#[test]
fn a_disabled_cursor_cannot_be_chosen_from_its_list() {
    let mut harness = well(State::default(), Builtin::Dark);
    harness
        .get_by_role_and_label(Role::Button, PICK_LIST)
        .click();
    harness.run();
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "Colour readout")
        .click();
    assert_eq!(reported(&mut harness), vec![]);
}

#[test]
fn a_list_open_as_the_well_is_disabled_closes() {
    let mut harness = well(State::default(), Builtin::Dark);
    harness
        .get_by_role_and_label(Role::Button, PICK_LIST)
        .click();
    harness.run();
    harness.state_mut().disabled = true;
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(Role::Menu, PICK_LIST)
            .is_none()
    );
    assert_eq!(reported(&mut harness), vec![]);
}

#[test]
fn the_well_is_one_tab_stop_and_the_arrows_move_inside_it() {
    let mut harness = well(
        State {
            chosen: 1,
            ..State::default()
        },
        Builtin::Dark,
    );
    harness.key_press(Key::Tab);
    harness.run();
    assert_eq!(focused(&harness), "Before");
    // Tab lands on the chosen cursor's face.
    harness.key_press(Key::Tab);
    harness.run();
    assert_eq!(focused(&harness), "Pan");
    let order = [
        "White balance picker, K",
        PICK_LIST,
        "Copy and apply",
        APPLY_LIST,
        // At the end it stays.
        APPLY_LIST,
    ];
    for name in order {
        harness.key_press(Key::ArrowRight);
        harness.run();
        assert_eq!(focused(&harness), name);
    }
    for name in [
        "Copy and apply",
        PICK_LIST,
        "White balance picker, K",
        "Pan",
        "Select, Esc",
    ] {
        harness.key_press(Key::ArrowLeft);
        harness.run();
        assert_eq!(focused(&harness), name);
    }
    // One Tab leaves the well from wherever its focus is.
    harness.key_press(Key::ArrowRight);
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(focused(&harness), "White balance picker, K");
    harness.key_press(Key::Tab);
    harness.run();
    assert_eq!(focused(&harness), "After");
    // And Shift+Tab comes back to the chosen cursor's face.
    harness.key_press_modifiers(egui::Modifiers::SHIFT, Key::Tab);
    harness.run();
    assert_eq!(focused(&harness), "Pan");
}

#[test]
fn space_presses_a_face_and_down_or_space_on_a_chevron_opens_its_list() {
    let mut harness = well(State::default(), Builtin::Dark);
    harness.key_press(Key::Tab);
    harness.key_press(Key::Tab);
    harness.key_press(Key::ArrowRight);
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(focused(&harness), "White balance picker, K");
    harness.key_press(Key::Space);
    assert_eq!(
        reported(&mut harness),
        vec![CursorWellAction::Cursor {
            group: 2,
            cursor: 0
        }]
    );
    // ↓ on a face opens its group's list, its first line focused.
    harness.key_press(Key::ArrowDown);
    harness.run();
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(Role::Menu, PICK_LIST)
            .is_some()
    );
    assert_eq!(focused(&harness), "White balance picker");
    harness.key_press(Key::ArrowDown);
    harness.key_press(Key::Enter);
    assert_eq!(
        reported(&mut harness),
        vec![CursorWellAction::Cursor {
            group: 2,
            cursor: 1
        }]
    );
    harness.run();
    // The focus is back on the face, which shows Exposure's picker now.
    assert_eq!(focused(&harness), "Exposure picker");
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(focused(&harness), PICK_LIST);
    harness.key_press(Key::Space);
    harness.run();
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(Role::Menu, PICK_LIST)
            .is_some()
    );
}

#[test]
fn screen_readers_hear_a_radio_group_of_faces_named_with_their_keys() {
    let mut harness = well(State::default(), Builtin::Dark);
    let group = harness.get_by_role_and_label(Role::RadioGroup, "Cursors");
    let faces: Vec<(String, Option<Toggled>)> = group
        .query_all_by_role(Role::RadioButton)
        .map(|n| {
            let n = n.accesskit_node();
            (n.label().unwrap_or_default(), n.toggled())
        })
        .collect();
    assert_eq!(
        faces,
        vec![
            ("Select, Esc".into(), Some(Toggled::True)),
            ("Pan".into(), Some(Toggled::False)),
            ("White balance picker, K".into(), Some(Toggled::False)),
            ("Copy and apply".into(), Some(Toggled::False)),
        ]
    );
    let chevrons: Vec<String> = group
        .query_all_by_role(Role::Button)
        .map(|n| n.accesskit_node().label().unwrap_or_default())
        .collect();
    assert_eq!(chevrons, vec![PICK_LIST, APPLY_LIST]);
    // Its tooltip is its description: name, key and what it does.
    let white_balance = harness.get_by_role_and_label(Role::RadioButton, "White balance picker, K");
    assert_eq!(
        white_balance.accesskit_node().description().as_deref(),
        Some("White balance picker · K — Click a neutral area, or drag one")
    );
    assert_eq!(
        white_balance.accesskit_node().data().keyboard_shortcut(),
        Some("K")
    );
    assert_eq!(
        harness
            .get_by_role_and_label(Role::RadioButton, "Pan")
            .accesskit_node()
            .description()
            .as_deref(),
        Some("Pan — Drag to pan")
    );
    // Disabled, every target says why and none takes the focus.
    harness.state_mut().disabled = true;
    harness.run();
    let pan = harness.get_by_role_and_label(Role::RadioButton, "Pan");
    assert!(pan.accesskit_node().is_disabled());
    assert_eq!(
        pan.accesskit_node().description().as_deref(),
        Some("No photo is shown")
    );
    harness.key_press(Key::Tab);
    harness.key_press(Key::Tab);
    harness.run();
    assert_eq!(focused(&harness), "After");
}

fn photograph(which: Builtin, name: &str) {
    let size = egui::vec2(600.0, 230.0);
    common::photograph_section(which, size, name, Gallery::show_cursor_well).unwrap();
}

#[test]
fn the_cursor_well_in_the_dark_theme() {
    photograph(Builtin::Dark, "cursor-well-dark");
}

#[test]
fn the_cursor_well_in_the_light_theme() {
    photograph(Builtin::Light, "cursor-well-light");
}

/// Pick's list open, its cursors and its setting, and Copy and apply's, its choice under
/// its heading, each photographed in the theme `which`.
fn photograph_lists(which: Builtin, name: &str) {
    let mut results = SnapshotResults::new();
    for (list, what) in [(PICK_LIST, "pick"), (APPLY_LIST, "apply")] {
        let mut harness = well(State::default(), which);
        harness.get_by_role_and_label(Role::Button, list).click();
        harness.run();
        // The pointer away, so it hides nothing.
        harness.remove_cursor();
        harness.run();
        harness.snapshot(format!("{name}-{what}"));
        results.extend_harness(&mut harness);
    }
    results.unwrap();
}

#[test]
fn the_lists_in_the_dark_theme() {
    photograph_lists(Builtin::Dark, "cursor-well-list-dark");
}

#[test]
fn the_lists_in_the_light_theme() {
    photograph_lists(Builtin::Light, "cursor-well-list-light");
}
