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

//! The sheet frame (`docs/gui.md`, Sheets; decision 86), on the gallery's sample sheet with
//! the Import tool's rows in its column: its keys (Return in each place, Ctrl+Return, Esc's
//! order, F6's three stops, Tab kept inside), choosing by check boxes with the pointer and
//! the keys, the Help button, what screen readers hear, the column's width and the least
//! window; and the sheet photographed at 1280 points and at the least window, its column at
//! either side, in the dark and light themes, Ukrainian, Polish and the pseudo-locale.
//! Accepted with `UPDATE_SNAPSHOTS=1 cargo test -p apertrust-ui --test sheet`, reviewed like
//! a golden image.

use apertrust_ui::components::{
    ButtonOrder, COLUMN_LEAST, Checks, PanelEdgeAction, Sheet, SheetCell, ThumbnailSize,
};
use apertrust_ui::gallery::Gallery;
use apertrust_ui::{Builtin, Locale, Theme};
use egui::accesskit::Role;
use egui::{Key, Modifiers, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, SnapshotResults};

/// The Toolbar's height over the sheet, in the window.
const TOOLBAR: f32 = 44.0;

/// The gallery's sheet in a window, a stand-in for the Toolbar's button over it, and what
/// the sheet reported.
struct State {
    gallery: Gallery,
    at_start: bool,
    primary: usize,
    close: usize,
    widths: Vec<f32>,
}

fn sheet(
    size: egui::Vec2,
    which: Builtin,
    locale: Locale,
    at_start: bool,
) -> Harness<'static, State> {
    let state = State {
        gallery: Gallery::new(which),
        at_start,
        primary: 0,
        close: 0,
        widths: Vec::new(),
    };
    let mut harness = Harness::builder()
        .with_size(size)
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::builtin(which));
            apertrust_ui::set_locale(ctx, locale);
        })
        .build_ui_state(
            |ui, state: &mut State| {
                // The window from edge to edge: a stand-in for the Toolbar, the sheet under it.
                let whole = ui.ctx().content_rect();
                let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(whole));
                let ui = &mut ui;
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                let surface = apertrust_ui::theme(ui.ctx()).colors.surface;
                let (bar, _) = ui
                    .allocate_exact_size(vec2(ui.available_width(), TOOLBAR), egui::Sense::hover());
                ui.painter().rect_filled(bar, 0.0, surface);
                let mut toolbar = ui.new_child(egui::UiBuilder::new().max_rect(bar));
                let _ = toolbar.button("Outside");
                let out = state.gallery.show_sheet(ui, state.at_start);
                state.primary += usize::from(out.primary);
                state.close += usize::from(out.close);
                if let Some(apertrust_ui::components::PanelEdgeAction::Resize(width)) = out.column {
                    state.widths.push(width);
                }
            },
            state,
        );
    harness.run();
    harness.run();
    harness
}

fn wide() -> Harness<'static, State> {
    sheet(vec2(1280.0, 720.0), Builtin::Dark, Locale::EnGb, false)
}

fn checks(harness: &Harness<'_, State>) -> Checks {
    harness.state().gallery.sheet_checks().clone()
}

fn focused<S>(harness: &Harness<'_, S>) -> Option<String> {
    harness
        .query_by(|node| node.is_focused())
        .and_then(|node| node.accesskit_node().label())
}

#[test]
fn it_opens_with_the_focus_on_the_active_cell_and_return_there_presses_the_primary_button() {
    let mut harness = wide();
    assert_eq!(focused(&harness).as_deref(), Some("DSCF1009.RAF"));
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().primary, 1);
    // The active photo stays as it was: the cell took no click.
    assert_eq!(checks(&harness).active(), Some(8));
}

/// A sheet whose grid is empty as it opens, its photos on their way, gives the opening
/// focus to its active cell once it has one: Space, with no click, turns its check.
#[test]
fn a_grid_empty_as_the_sheet_opens_takes_the_focus_once_its_photos_come() {
    use apertrust_ui::components::CheckGesture;
    struct Filling {
        count: usize,
        checks: Checks,
    }
    let mut harness = Harness::builder()
        .with_size(vec2(1280.0, 720.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark)))
        .build_ui_state(
            |ui, state: &mut Filling| {
                let names = ["DSCF1001.RAF", "DSCF1002.RAF", "DSCF1003.RAF"];
                let checks = &state.checks;
                let out = Sheet::new(
                    "Import · CARD",
                    "Import, CARD",
                    state.count,
                    checks.active().filter(|_| state.count > 0),
                    |i| SheetCell {
                        name: names[i],
                        picture: None,
                        no_preview: true,
                        checked: checks.is_checked(i),
                        check_name: names[i],
                        check_description: "",
                        dimmed: false,
                        mark: None,
                        tip: None,
                    },
                )
                .primary("Import", None)
                .show(ui, |_, _| {});
                if let Some(gesture) = out.gesture {
                    state.checks.apply(gesture);
                }
            },
            Filling {
                count: 0,
                checks: Checks::new(3),
            },
        );
    harness.run();
    harness.run();
    // The listing arrives: three photos, the first active.
    harness.state_mut().count = 3;
    harness.state_mut().checks.apply(CheckGesture::Activate(0));
    harness.run();
    harness.run();
    harness.key_press(Key::Space);
    harness.run();
    assert!(harness.state().checks.is_checked(0));
}

#[test]
fn return_goes_to_a_focused_button_and_a_text_field_and_presses_nothing_from_a_path_field() {
    let mut harness = wide();
    // Close focused presses itself.
    harness.get_by_role_and_label(Role::Button, "Close").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!((harness.state().primary, harness.state().close), (0, 1));
    // A path field typed in ends its typing and presses nothing.
    harness
        .get_by_role_and_label(Role::TextInput, "Folder")
        .focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().primary, 0);
    // Import name reports its Return, which presses the primary button.
    harness
        .get_by_role_and_label(Role::TextInput, "Import name")
        .focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().primary, 1);
    // A ⌄ focused opens its list; Return there chooses in it, never the sheet's.
    harness
        .get_by_role_and_label(Role::Button, "Subfolders")
        .focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(
        harness
            .query_all_by_role(Role::MenuItemRadio)
            .next()
            .is_some()
    );
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().primary, 1);
}

#[test]
fn ctrl_return_presses_the_primary_button_from_a_text_field_too() {
    let mut harness = wide();
    harness
        .get_by_role_and_label(Role::TextInput, "Folder")
        .focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
    harness.run();
    assert_eq!(harness.state().primary, 1);
}

/// Ctrl+Return with a path field's list of variables open is the list's, never the
/// primary button's; with the list closed it presses the primary button from the field.
#[test]
fn ctrl_return_with_the_variables_listed_presses_nothing() {
    let mut harness = wide();
    harness
        .get_by_role_and_label(Role::TextInput, "Folder")
        .focus();
    harness.run();
    harness.key_press(Key::End);
    harness.run();
    harness.event(egui::Event::Text("/$(".to_owned()));
    harness.run();
    assert!(harness.query_by_role(Role::ListBox).is_some());
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
    harness.run();
    assert_eq!(harness.state().primary, 0);
    // The list closed, the field still typed in: Ctrl+Return presses it.
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.query_by_role(Role::ListBox).is_none());
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
    harness.run();
    assert_eq!(harness.state().primary, 1);
}

#[test]
fn a_primary_button_that_cannot_press_stays_a_stop_and_says_why() {
    let mut harness = wide();
    harness.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::A);
    harness.run();
    harness.run();
    assert_eq!(checks(&harness).checked_count(), 0);
    let button = harness.get_by_role_and_label(Role::Button, "Import 0 photos");
    assert!(button.accesskit_node().is_disabled());
    assert_eq!(
        button.accesskit_node().description().as_deref(),
        Some("No photos checked")
    );
    // The footer's second line says it too, and nothing presses it.
    assert!(harness.query_by_label("No photos checked").is_some());
    harness.key_press(Key::Enter);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
    harness.run();
    assert_eq!(harness.state().primary, 0);
}

#[test]
fn escape_closes_the_innermost_first_then_the_sheet() {
    let mut harness = wide();
    // The variable list, then the field typed in, then the sheet.
    harness
        .get_by_role_and_label(Role::TextInput, "Folder")
        .focus();
    harness.run();
    harness.key_press(Key::End);
    harness.run();
    harness.event(egui::Event::Text("/$(".to_owned()));
    harness.run();
    assert!(harness.query_by_role(Role::ListBox).is_some());
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.query_by_role(Role::ListBox).is_none());
    assert_eq!(harness.state().close, 0);
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().close, 0);
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().close, 1);
    // A menu open takes it first.
    harness
        .get_by_role_and_label(Role::Button, "Subfolders")
        .click();
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert!(
        harness
            .query_all_by_role(Role::MenuItemRadio)
            .next()
            .is_none()
    );
    assert_eq!(harness.state().close, 1);
}

#[test]
fn f6_moves_among_three_regions_landing_on_the_active_cell() {
    let mut harness = wide();
    let mut stops = Vec::new();
    for _ in 0..4 {
        harness.key_press(Key::F6);
        harness.run();
        harness.run();
        stops.push(focused(&harness).unwrap_or_default());
    }
    // The column's first stop, the footer's first, then the grid's active cell again. The
    // footer's first is the platform's: the primary button first on Windows, Close first
    // elsewhere.
    let first = match ButtonOrder::platform().arrange(2, 0)[0] {
        0 => "Import 32 photos",
        _ => "Close",
    };
    assert_eq!(stops[0], "Import");
    assert_eq!(stops[1], first);
    assert_eq!(stops[2], "DSCF1009.RAF");
    assert_eq!(stops[3], "Import");
    harness.key_press_modifiers(Modifiers::SHIFT, Key::F6);
    harness.run();
    harness.run();
    assert_eq!(focused(&harness).as_deref(), Some("DSCF1009.RAF"));
}

/// F6 back to the grid with its active cell scrolled out of sight scrolls it into sight
/// and gives it the focus.
#[test]
fn f6_back_to_the_grid_reveals_the_active_cell_scrolled_away() {
    let mut harness = sheet(vec2(500.0, 600.0), Builtin::Dark, Locale::EnGb, false);
    harness.key_press(Key::F6);
    harness.run();
    harness.run();
    assert_eq!(focused(&harness).as_deref(), Some("Import"));
    let grid = harness
        .get_by_role_and_label(Role::List, "EOS_DIGITAL, 40 photos, 32 checked")
        .rect();
    harness.hover_at(grid.center());
    harness.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: vec2(0.0, -3000.0),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
    });
    harness.run_steps(30);
    assert!(
        harness
            .query_by_role_and_label(Role::ListItem, "DSCF1009.RAF")
            .is_none_or(|cell| !grid.intersects(cell.rect())),
        "scrolled away"
    );
    for _ in 0..2 {
        harness.key_press(Key::F6);
        harness.run();
        harness.run();
    }
    harness.run();
    assert_eq!(focused(&harness).as_deref(), Some("DSCF1009.RAF"));
    let cell = harness
        .get_by_role_and_label(Role::ListItem, "DSCF1009.RAF")
        .rect();
    assert!(grid.contains_rect(cell), "{cell:?} in {grid:?}");
}

/// Shift with an arrow checks the cells passed from the cell focused, not from the active
/// photo: a check box's click focuses its cell and leaves the active photo where it was.
#[test]
fn shift_with_an_arrow_after_a_check_box_s_click_checks_from_that_cell() {
    let mut harness = wide();
    assert_eq!(checks(&harness).active(), Some(8));
    harness
        .get_by_role_and_label(Role::CheckBox, "Import DSCF1002.RAF")
        .click();
    harness.run();
    assert_eq!(checks(&harness).active(), Some(8));
    assert_eq!(focused(&harness).as_deref(), Some("DSCF1002.RAF"));
    harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowRight);
    harness.run();
    let after = checks(&harness);
    assert!(after.is_checked(1) && after.is_checked(2));
    // Nothing between it and the active photo, none of which was passed.
    assert!((3..8).all(|photo| !after.is_checked(photo)));
    assert_eq!(after.active(), Some(2));
}

/// → pressed in two passes one after the other keeps the focus and the active photo on
/// the same cell, so Space then turns the active photo's check: egui's own move of the
/// focus by the arrows never runs on past the cell the grid moved it to.
#[test]
fn arrows_in_consecutive_passes_keep_the_focus_on_the_active_photo() {
    let mut harness = wide();
    assert_eq!(checks(&harness).active(), Some(8));
    for _ in 0..2 {
        harness.key_press(Key::ArrowRight);
        harness.step();
    }
    harness.run();
    let active = checks(&harness).active().expect("an active photo");
    assert_eq!(active, 10);
    let name = format!("DSCF{}.RAF", 1001 + active);
    assert_eq!(focused(&harness).as_deref(), Some(name.as_str()));
    let before = checks(&harness).is_checked(active);
    harness.key_press(Key::Space);
    harness.run();
    let after = checks(&harness);
    assert_eq!(after.is_checked(active), !before);
    assert!(
        after.is_checked(active + 1),
        "the next photo left as it was"
    );
    // ↓ likewise, a line at a time.
    for _ in 0..2 {
        harness.key_press(Key::ArrowDown);
        harness.step();
    }
    harness.run();
    let down = checks(&harness).active().expect("an active photo");
    assert!(
        down > active + 1,
        "moved down two lines from {active} to {down}"
    );
    let name = format!("DSCF{}.RAF", 1001 + down);
    assert_eq!(focused(&harness).as_deref(), Some(name.as_str()));
}

#[test]
fn tab_stays_inside_the_sheet() {
    let mut harness = wide();
    // The last stop, the primary button where the platform puts it last, or Close.
    let last = harness
        .get_all_by_role(Role::Button)
        .filter(|b| {
            matches!(
                b.accesskit_node().label().as_deref(),
                Some("Close" | "Import 32 photos")
            )
        })
        .max_by(|a, b| a.rect().left().total_cmp(&b.rect().left()))
        .expect("the footer's buttons")
        .accesskit_node()
        .label()
        .unwrap_or_default();
    harness.get_by_role_and_label(Role::Button, &last).focus();
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
    harness.run();
    let after = focused(&harness).unwrap_or_default();
    assert_ne!(after, "Outside");
    assert!(!after.is_empty());
    // And back.
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
    harness.run();
    harness.run();
    assert_eq!(focused(&harness).as_deref(), Some(last.as_str()));
}

#[test]
fn the_pointer_and_the_keys_choose_by_check_boxes() {
    let mut harness = wide();
    // A click on a check box turns it, its cell taking the focus.
    harness
        .get_by_role_and_label(Role::CheckBox, "Import DSCF1002.RAF")
        .click();
    harness.run();
    assert!(checks(&harness).is_checked(1));
    assert_eq!(checks(&harness).last_touched(), Some(1));
    assert_eq!(focused(&harness).as_deref(), Some("DSCF1002.RAF"));
    // Space on the focused cell turns its check.
    harness.key_press(Key::Space);
    harness.run();
    assert!(!checks(&harness).is_checked(1));
    // Shift+Space sets the range from the last touched as it was set: unchecked.
    harness
        .get_by_role_and_label(Role::ListItem, "DSCF1010.RAF")
        .focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Space);
    harness.run();
    assert_eq!(checks(&harness).checked().next(), Some(10));
    // → moves the focus and the active photo; Shift+← checks the cells passed.
    harness.key_press(Key::ArrowRight);
    harness.run();
    harness.run();
    assert_eq!(checks(&harness).active(), Some(10));
    assert_eq!(focused(&harness).as_deref(), Some("DSCF1011.RAF"));
    harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowLeft);
    harness.run();
    assert_eq!(checks(&harness).active(), Some(9));
    assert!(checks(&harness).is_checked(9));
    // A click on a picture makes it the active photo, the checks unchanged.
    let before = checks(&harness).checked_count();
    harness
        .get_by_role_and_label(Role::ListItem, "DSCF1013.RAF")
        .click();
    harness.run();
    assert_eq!(checks(&harness).active(), Some(12));
    assert_eq!(checks(&harness).checked_count(), before);
    // Ctrl+A checks every photo.
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.run();
    assert_eq!(checks(&harness).checked_count(), 40);
}

#[test]
fn screen_readers_hear_a_region_in_a_modal_dialog_a_list_and_named_check_boxes() {
    let harness = wide();
    let dialog = harness.get_by_role_and_label(Role::Dialog, "Import, EOS_DIGITAL");
    assert!(dialog.accesskit_node().is_modal());
    // The region inside the dialog, the sheet's parts inside the region.
    let region = dialog.get_by_role_and_label(Role::Region, "Import, EOS_DIGITAL");
    region.get_by_role_and_label(Role::Toolbar, "Import grid bar");
    harness.get_by_role_and_label(Role::List, "EOS_DIGITAL, 40 photos, 32 checked");
    let cell = harness.get_by_role_and_label(Role::ListItem, "DSCF1001.RAF");
    assert_eq!(
        cell.accesskit_node().description().as_deref(),
        Some("Imported before")
    );
    cell.get_by_role_and_label(Role::CheckBox, "Import DSCF1001.RAF");
    let said = harness.get_by_role_and_label(Role::Status, "32 checked");
    assert_eq!(
        said.accesskit_node().data().live(),
        Some(egui::accesskit::Live::Polite)
    );
}

#[test]
fn help_is_drawn_disabled_saying_it_comes_later() {
    let mut harness = wide();
    let help = harness.get_by_role_and_label(Role::Button, "Help");
    assert!(help.accesskit_node().is_disabled());
    assert_eq!(
        help.accesskit_node().description().as_deref(),
        Some("Comes in a later version")
    );
    help.hover();
    harness.run();
    assert!(harness.query_by_label("Comes in a later version").is_some());
}

#[test]
fn at_the_least_window_the_column_is_296_and_the_buttons_are_whole() {
    let harness = sheet(vec2(500.0, 600.0), Builtin::Dark, Locale::EnGb, false);
    let edge = harness.get_by_role_and_label(Role::Splitter, "Tool column width");
    assert_eq!(
        edge.accesskit_node().numeric_value(),
        Some(f64::from(COLUMN_LEAST))
    );
    for name in ["Close", "Import 32 photos"] {
        let button = harness.get_by_role_and_label(Role::Button, name).rect();
        assert!(
            button.right() <= 500.0 && button.left() >= 0.0,
            "{name} {button:?}"
        );
    }
    // The Select buttons folded into Select ▾.
    harness.get_by_role_and_label(Role::Button, "Select");
    assert!(harness.query_by_label("Select new").is_none());
    // Two Small cells across, the active photo's row in sight.
    let active = harness
        .get_by_role_and_label(Role::ListItem, "DSCF1009.RAF")
        .rect();
    let beside = harness
        .get_by_role_and_label(Role::ListItem, "DSCF1010.RAF")
        .rect();
    assert_eq!(active.width(), 84.0);
    assert_eq!(active.top(), beside.top());
    assert!(beside.right() <= 500.0 - COLUMN_LEAST);
}

#[test]
fn the_column_follows_its_edge_within_its_widths() {
    let mut harness = wide();
    let edge = harness.get_by_role_and_label(Role::Splitter, "Tool column width");
    assert_eq!(edge.accesskit_node().numeric_value(), Some(440.0));
    edge.focus();
    harness.run();
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().widths, [450.0]);
    harness.key_press(Key::End);
    harness.run();
    assert_eq!(harness.state().widths.last(), Some(&600.0));
}

/// A sheet of one photo, `size` points large edge to edge, at `zoom`, its column empty,
/// and the widths its edge reported.
fn one_photo(size: egui::Vec2, zoom: ThumbnailSize) -> Harness<'static, Vec<f32>> {
    let mut harness = Harness::builder()
        .with_size(size)
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark)))
        .build_ui_state(
            move |ui, widths: &mut Vec<f32>| {
                let whole = ui.ctx().content_rect();
                let ui = &mut ui.new_child(egui::UiBuilder::new().max_rect(whole));
                let out = Sheet::new("Import · CARD", "Import, CARD", 1, Some(0), |_| {
                    SheetCell {
                        name: "DSCF1001.RAF",
                        picture: None,
                        no_preview: true,
                        checked: false,
                        check_name: "Import DSCF1001.RAF",
                        check_description: "",
                        dimmed: false,
                        mark: None,
                        tip: None,
                    }
                })
                .size(zoom)
                .column(440.0, false)
                .primary("Import", None)
                .show(ui, |_, _| {});
                if let Some(PanelEdgeAction::Resize(width)) = out.column {
                    widths.push(width);
                }
            },
            Vec::new(),
        );
    harness.run();
    harness.run();
    harness
}

/// F6 passes over a column with nothing to focus, either way.
#[test]
fn f6_passes_over_a_column_with_nothing_to_focus() {
    let mut harness = one_photo(vec2(1280.0, 720.0), ThumbnailSize::Medium);
    assert_eq!(focused(&harness).as_deref(), Some("DSCF1001.RAF"));
    let first = match ButtonOrder::platform().arrange(2, 0)[0] {
        0 => "Import",
        _ => "Close",
    };
    harness.key_press(Key::F6);
    harness.run();
    harness.run();
    assert_eq!(focused(&harness).as_deref(), Some(first));
    harness.key_press_modifiers(Modifiers::SHIFT, Key::F6);
    harness.run();
    harness.run();
    assert_eq!(focused(&harness).as_deref(), Some("DSCF1001.RAF"));
}

/// A sheet whose grid keeps two Large cells draws its column narrower than the kept 440:
/// its edge reports the width drawn, and an arrow key steps from it.
#[test]
fn a_column_giving_way_reports_the_width_drawn_and_steps_from_it() {
    let mut harness = one_photo(vec2(700.0, 600.0), ThumbnailSize::Large);
    // 700 points less two Large cells with their gaps, the room round them and the scroll
    // bar's lane: 312.
    let grid = harness.get_by_role(Role::List).rect();
    let drawn = 700.0 - grid.width();
    assert_eq!(drawn, 312.0);
    let edge = harness.get_by_role_and_label(Role::Splitter, "Tool column width");
    assert_eq!(
        edge.accesskit_node().numeric_value(),
        Some(f64::from(drawn))
    );
    edge.focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(harness.state().as_slice(), [302.0]);
}

/// The sheet in a window `size` points large, less the Toolbar, in `which` and `locale`, its
/// column at the start where `at_start` says.
fn photograph(
    name: &str,
    size: egui::Vec2,
    which: Builtin,
    locale: Locale,
    at_start: bool,
) -> SnapshotResults {
    let mut harness = sheet(size, which, locale, at_start);
    harness.snapshot(name);
    harness.take_snapshot_results()
}

#[test]
fn the_sheet_at_1280_and_at_the_least_window() {
    let mut results = SnapshotResults::new();
    let wide = vec2(1280.0, 720.0);
    let least = vec2(500.0, 600.0);
    results.extend(photograph(
        "sheet-1280-right-dark",
        wide,
        Builtin::Dark,
        Locale::EnGb,
        false,
    ));
    results.extend(photograph(
        "sheet-1280-left-light",
        wide,
        Builtin::Light,
        Locale::EnGb,
        true,
    ));
    results.extend(photograph(
        "sheet-least-dark",
        least,
        Builtin::Dark,
        Locale::EnGb,
        false,
    ));
    results.extend(photograph(
        "sheet-least-left-light",
        least,
        Builtin::Light,
        Locale::EnGb,
        true,
    ));
    results.extend(photograph(
        "sheet-1280-uk",
        wide,
        Builtin::Dark,
        Locale::Uk,
        false,
    ));
    results.extend(photograph(
        "sheet-1280-pl",
        wide,
        Builtin::Dark,
        Locale::Pl,
        false,
    ));
    results.extend(photograph(
        "sheet-least-pseudo",
        least,
        Builtin::Dark,
        Locale::Pseudo,
        false,
    ));
    results.unwrap();
}
