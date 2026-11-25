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

//! The line row and a toggle's end items (decision 107; `docs/gui.md`, Line rows and end
//! items): what each end item reports, the Tab order, a menu button's menu, a toggle's
//! end item that never toggles it, and the gallery's Line rows section photographed at
//! twice the size in the dark and the light theme, and with its menu open. Accepted with
//! `UPDATE_SNAPSHOTS=1 cargo test -p apertrust-ui --test line_row`, reviewed like a golden
//! image.

mod common;

use apertrust_ui::components::{
    EndAction, EndItem, LineAction, LineRow, LineStart, PickerAction, PickerButton, SliderRow,
    ToggleRow,
};
use apertrust_ui::gallery::Gallery;
use apertrust_ui::popup::{Entry, Item};
use apertrust_ui::{Builtin, Icon, Theme};
use egui::Key;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

/// A line row of a label, a picker, an icon button, a text button and a menu button, and a
/// toggle with a text button, with what each reported.
#[derive(Default)]
struct Lines {
    label: &'static str,
    on: bool,
    line: Vec<LineAction>,
    toggled: Vec<bool>,
    toggle_end: Vec<(usize, EndAction)>,
}

fn lines(label: &'static str, width: f32) -> Harness<'static, Lines> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(width, 120.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark)))
        .build_ui_state(
            |ui, lines: &mut Lines| {
                let line = LineRow::new(
                    LineStart::Label(lines.label),
                    vec![
                        EndItem::Picker(PickerButton::new("Range: pick from the photo", false)),
                        EndItem::Icon {
                            icon: Icon::Raw,
                            name: "Read exposure",
                            promise: Some("From the Exposure tool."),
                            enabled: true,
                        },
                        EndItem::Text {
                            label: "Set",
                            name: "Set from above",
                            promise: None,
                            enabled: true,
                        },
                        EndItem::Menu {
                            icon: Icon::Presets,
                            name: "Reset primaries",
                            promise: None,
                            enabled: true,
                            entries: vec![
                                Entry::Item(Item::new("smooth", "Smooth")),
                                Entry::Item(Item::new("unmodified", "Unmodified")),
                            ],
                        },
                    ],
                )
                .show(ui);
                lines.line.extend(line);
                let (toggled, end) = ToggleRow::new("Reverse all", lines.on)
                    .end(vec![EndItem::Text {
                        label: "Set from above",
                        name: "Set from above, again",
                        promise: None,
                        enabled: !lines.on,
                    }])
                    .show_with_end(ui);
                if let Some(on) = toggled {
                    lines.on = on;
                    lines.toggled.push(on);
                }
                lines.toggle_end.extend(end);
            },
            Lines {
                label,
                ..Lines::default()
            },
        );
    harness.run();
    harness
}

/// Each end item reports its own press, by its index, and the menu button its menu's
/// choice; the menu button says it has a menu and whether it is open.
#[test]
fn each_end_item_reports_its_press_and_a_menu_its_choice() {
    let mut harness = lines("Auto tune levels", 320.0);
    harness
        .get_by_role_and_label(Role::Button, "Range: pick from the photo")
        .click();
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Read exposure")
        .click();
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Set from above")
        .click();
    harness.run();
    let menu = harness.get_by_role_and_label(Role::Button, "Reset primaries");
    assert_eq!(menu.accesskit_node().data().is_expanded(), Some(false));
    menu.click();
    harness.run();
    let menu = harness.get_by_role_and_label(Role::Button, "Reset primaries");
    assert_eq!(menu.accesskit_node().data().is_expanded(), Some(true));
    harness
        .get_by_role_and_label(Role::MenuItem, "Smooth")
        .click();
    harness.run();
    assert_eq!(
        harness.state().line,
        [
            LineAction::End(0, EndAction::Picker(PickerAction::On { area: false })),
            LineAction::End(1, EndAction::Pressed),
            LineAction::End(2, EndAction::Pressed),
            LineAction::End(3, EndAction::Chosen("smooth".to_owned())),
        ]
    );
}

/// The Tab order is the reading order: the end items left to right, then the toggle and
/// its end item.
#[test]
fn tab_meets_the_end_items_in_reading_order() {
    let mut harness = lines("Auto tune levels", 320.0);
    let mut met = Vec::new();
    for _ in 0..6 {
        harness.key_press(Key::Tab);
        harness.run();
        let focused = harness.get_by(|node| node.is_focused());
        met.push(focused.accesskit_node().label().unwrap_or_default());
    }
    assert_eq!(
        met,
        [
            "Range: pick from the photo",
            "Read exposure",
            "Set from above",
            "Reset primaries",
            "Reverse all",
            "Set from above, again",
        ]
    );
}

/// A press on a toggle's end item is the item's, never the toggle's; a press elsewhere on
/// the row toggles it, and the item, disabled while the toggle is on, takes none.
#[test]
fn a_toggle_s_end_item_never_toggles_it() {
    let mut harness = lines("Auto tune levels", 320.0);
    harness
        .get_by_role_and_label(Role::Button, "Set from above, again")
        .click();
    harness.run();
    assert_eq!(harness.state().toggle_end, [(0, EndAction::Pressed)]);
    assert!(harness.state().toggled.is_empty());
    harness
        .get_by_role_and_label(Role::Switch, "Reverse all")
        .click();
    harness.run();
    assert_eq!(harness.state().toggled, [true]);
    let item = harness.get_by_role_and_label(Role::Button, "Set from above, again");
    assert!(item.accesskit_node().is_disabled());
    // A press on it, disabled, neither toggles the row nor runs it.
    item.click();
    harness.run();
    assert_eq!(harness.state().toggled, [true]);
    assert_eq!(harness.state().toggle_end, [(0, EndAction::Pressed)]);
}

/// The label is cut first, with …, never an end item: at a narrow width every end item is
/// still drawn whole.
#[test]
fn the_label_is_cut_before_any_end_item() {
    let harness = lines("A label far too long to fit beside its end items", 220.0);
    for name in [
        "Range: pick from the photo",
        "Read exposure",
        "Set from above",
        "Reset primaries",
    ] {
        let node = harness.get_by_role_and_label(Role::Button, name);
        let rect = node.rect();
        assert!(
            rect.right() <= 220.0 && rect.width() >= 18.0,
            "{name}: {rect:?}"
        );
    }
}

/// The gallery's Line rows section in the dark and the light theme.
#[test]
fn the_line_rows_in_the_dark_and_the_light_theme() {
    let size = egui::vec2(330.0, 230.0);
    let mut results = egui_kittest::SnapshotResults::new();
    for (which, name) in [
        (Builtin::Dark, "line-row-dark"),
        (Builtin::Light, "line-row-light"),
    ] {
        results.extend(common::photograph_section(
            which,
            size,
            name,
            Gallery::show_line_rows,
        ));
    }
}

/// The gallery's slider warnings, one on and one off, in the dark and the light theme.
#[test]
fn the_slider_warnings_in_the_dark_and_the_light_theme() {
    let size = egui::vec2(330.0, 150.0);
    let mut results = egui_kittest::SnapshotResults::new();
    for (which, name) in [
        (Builtin::Dark, "slider-warning-dark"),
        (Builtin::Light, "slider-warning-light"),
    ] {
        results.extend(common::photograph_section(
            which,
            size,
            name,
            Gallery::show_slider_warnings,
        ));
    }
}

/// A slider's warning is heard as the slider's description while it shows, and is no
/// focus stop: Tab goes from the marker to the track, as on a row without one.
#[test]
fn a_slider_s_warning_is_its_description_and_no_focus_stop() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(320.0, 100.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark)))
        .build_ui(|ui| {
            for (label, warning) in [
                ("Shoulder power", Some("Lost its S shape.")),
                ("Toe power", None),
            ] {
                SliderRow::new(label, 3.3, 0.0..=10.0)
                    .decimals(2)
                    .warning(warning)
                    .show(ui);
            }
        });
    harness.run();
    let shoulder = harness.get_by_role_and_label(Role::Slider, "Shoulder power");
    assert_eq!(
        shoulder.accesskit_node().description().as_deref(),
        Some("Lost its S shape.")
    );
    let toe = harness.get_by_role_and_label(Role::Slider, "Toe power");
    assert_eq!(toe.accesskit_node().description(), None);
    // The two values end at the same x, the sign's place kept on the row without it.
    assert_eq!(shoulder.rect().right(), toe.rect().right());
    let mut met = Vec::new();
    for _ in 0..4 {
        harness.key_press(Key::Tab);
        harness.run();
        let focused = harness.get_by(|node| node.is_focused());
        met.push(focused.accesskit_node().role());
    }
    assert!(!met.contains(&Role::Image), "{met:?}");
    // Heard first, before the promise.
    let mut harness = Harness::builder()
        .with_size(egui::vec2(320.0, 60.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark)))
        .build_ui(|ui| {
            SliderRow::new("Toe power", 1.5, 0.0..=10.0)
                .promise("Contrast in shadows.")
                .warning(Some("Lost its S shape."))
                .show(ui);
        });
    harness.run();
    let toe = harness.get_by_role_and_label(Role::Slider, "Toe power");
    assert_eq!(
        toe.accesskit_node().description().as_deref(),
        Some("Lost its S shape.\nContrast in shadows.")
    );
}

/// The gallery's menu button with its menu open, in the dark theme.
#[test]
fn the_line_row_s_menu_open() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(330.0, 300.0))
        .with_pixels_per_point(2.0)
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark)))
        .build_ui_state(
            |ui, gallery: &mut Gallery| {
                let panel = apertrust_ui::theme(ui.ctx()).colors.panel;
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                egui::Frame::new()
                    .inner_margin(12.0)
                    .show(ui, |ui| gallery.show_line_rows(ui));
            },
            Gallery::new(Builtin::Dark),
        );
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Reset primaries")
        .click();
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(Role::Menu, "Reset primaries")
            .is_some()
    );
    harness.snapshot("line-row-menu-dark");
}
