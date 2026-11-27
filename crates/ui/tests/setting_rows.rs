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

//! The setting rows the Tools panel lacks (`docs/gui.md`, The rows the tool panel lacks;
//! decision 86): the warning button's menu by the pointer and the keys, the path row's ⌄
//! on Alt+↓, Return in a path field and in a text field row, a long pattern on one line,
//! what screen readers hear and a disabled row; and the gallery's Setting rows,
//! photographed at 296 and 440 points. Accepted with `UPDATE_SNAPSHOTS=1 cargo test -p
//! apertrust-ui --test setting_rows`, reviewed like a golden image.

use apertrust_ui::components::{
    Fix, PathChoice, PathRow, Problem, TEXT_FIELD_WIDTH, TextFieldRow, Variable,
};
use apertrust_ui::gallery::Gallery;
use apertrust_ui::{Builtin, Theme};
use egui::accesskit::{HasPopup, Role, Toggled};
use egui::{Event, Id, Key, Modifiers, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, SnapshotResults};

const VARIABLES: [Variable; 2] = [
    Variable {
        insert: "MONTH",
        description: "month",
    },
    Variable {
        insert: "YEAR",
        description: "year",
    },
];

const FIXES: [Fix; 2] = [
    Fix {
        id: "use-month",
        text: "Use $(MONTH)",
    },
    Fix {
        id: "choose-folder",
        text: "Choose a folder…",
    },
];

const CHOICES: [PathChoice; 2] = [
    PathChoice {
        id: "by-day",
        name: "By day",
        pattern: Some("$(YEAR)$(MONTH)$(DAY)"),
        chosen: true,
        disabled: None,
    },
    PathChoice {
        id: "by-camera",
        name: "By camera",
        pattern: None,
        chosen: false,
        disabled: Some("Comes in a later version"),
    },
];

/// A path row and a text field row as their caller keeps them: the texts, and what each
/// reported.
#[derive(Default)]
struct State {
    path: String,
    name: String,
    width: f32,
    stops: bool,
    disabled: bool,
    fixes: Vec<String>,
    choices: Vec<String>,
    returned: usize,
    row_height: f32,
}

fn rows(path: &str) -> Harness<'static, State> {
    let state = State {
        path: path.to_owned(),
        name: "Wedding".to_owned(),
        width: 296.0,
        ..State::default()
    };
    let mut harness = Harness::builder()
        .with_size(vec2(480.0, 320.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, state: &mut State| {
                ui.allocate_ui(vec2(state.width, 300.0), |ui| {
                    ui.set_width(state.width);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let top = ui.cursor().top();
                    let problem = Problem::new("Unknown variable $(MONT)")
                        .stops(state.stops)
                        .fixes(&FIXES);
                    let mut row = PathRow::new(
                        Id::unique("subfolder"),
                        "Subfolder",
                        &state.path,
                        &VARIABLES,
                    )
                    .problem(Some(problem))
                    .choices("Subfolders", &CHOICES)
                    .folder("Choose a folder");
                    if state.disabled {
                        row = row.disabled("Comes in a later version");
                    }
                    let out = row.show(ui);
                    state.row_height = ui.cursor().top() - top;
                    if let Some(text) = out.text {
                        state.path = text;
                    }
                    state.fixes.extend(out.fix);
                    state.choices.extend(out.choice);
                    let out =
                        TextFieldRow::new(Id::unique("name"), "Import name", &state.name).show(ui);
                    if let Some(text) = out.text {
                        state.name = text;
                    }
                    state.returned += usize::from(out.returned);
                });
            },
            state,
        );
    harness.run();
    harness
}

const PROBLEM: &str = "Unknown variable $(MONT)";

#[test]
fn the_warning_button_s_menu_opens_with_a_click_and_with_the_keys_and_reports_its_fix() {
    let mut harness = rows("$(YEAR)$(MONT)");
    harness.get_by_role_and_label(Role::Button, PROBLEM).click();
    harness.run();
    harness
        .get_by_role_and_label(Role::MenuItem, "Use $(MONTH)")
        .click();
    harness.run();
    assert_eq!(harness.state().fixes, ["use-month"]);
    // Return and Space on the focused button open it too.
    for key in [Key::Enter, Key::Space] {
        harness.get_by_role_and_label(Role::Button, PROBLEM).focus();
        harness.run();
        harness.key_press(key);
        harness.run();
        harness
            .get_by_role_and_label(Role::MenuItem, "Choose a folder…")
            .click();
        harness.run();
    }
    assert_eq!(
        harness.state().fixes,
        ["use-month", "choose-folder", "choose-folder"]
    );
}

#[test]
fn the_warning_button_is_a_menu_button_named_by_its_problem_and_says_it_assertively() {
    let harness = rows("$(YEAR)$(MONT)");
    let button = harness.get_by_role_and_label(Role::Button, PROBLEM);
    assert_eq!(
        button.accesskit_node().data().has_popup(),
        Some(HasPopup::Menu)
    );
    let said = harness.get_by_role_and_label(Role::Status, PROBLEM);
    assert_eq!(
        said.accesskit_node().data().live(),
        Some(egui::accesskit::Live::Assertive)
    );
    // The field's problem is its description, after nothing the session said.
    let field = harness.get_by_role_and_label(Role::TextInput, "Subfolder");
    assert_eq!(
        field.accesskit_node().description().as_deref(),
        Some(PROBLEM)
    );
}

#[test]
fn alt_down_on_the_focused_field_opens_its_choices_as_radio_items() {
    let mut harness = rows("$(YEAR)");
    assert!(harness.query_by_role(Role::MenuItemRadio).is_none());
    harness
        .get_by_role_and_label(Role::TextInput, "Subfolder")
        .focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowDown);
    harness.run();
    let by_day = harness.get_by_role_and_label(Role::MenuItemRadio, "By day");
    assert_eq!(by_day.accesskit_node().toggled(), Some(Toggled::True));
    // Its pattern, drawn under its name, is its description.
    assert_eq!(
        by_day.accesskit_node().description().as_deref(),
        Some("$(YEAR)$(MONTH)$(DAY)")
    );
    let camera = harness.get_by_role_and_label(Role::MenuItemRadio, "By camera");
    assert!(camera.accesskit_node().is_disabled());
    // The pattern under the name is drawn, and the item it stands for answers.
    harness
        .get_by_role_and_label(Role::MenuItemRadio, "By day")
        .click();
    harness.run();
    assert_eq!(harness.state().choices, ["by-day"]);
}

#[test]
fn return_in_the_path_field_takes_the_variable_highlighted_and_reports_no_press() {
    let mut harness = rows("");
    harness
        .get_by_role_and_label(Role::TextInput, "Subfolder")
        .focus();
    harness.run();
    harness.event(Event::Text("$(MO".to_owned()));
    harness.run();
    harness.key_press(Key::ArrowDown);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().path, "$(MONTH)");
    assert_eq!(harness.state().returned, 0);
    assert!(
        harness
            .get_by_role_and_label(Role::TextInput, "Subfolder")
            .is_focused()
    );
}

#[test]
fn return_in_the_text_field_row_ends_the_typing_and_is_reported() {
    let mut harness = rows("$(YEAR)");
    harness
        .get_by_role_and_label(Role::TextInput, "Import name")
        .focus();
    harness.run();
    harness.event(Event::Text("s".to_owned()));
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().name, "Weddings");
    assert_eq!(harness.state().returned, 1);
    assert!(
        !harness
            .get_by_role_and_label(Role::TextInput, "Import name")
            .is_focused()
    );
}

#[test]
fn the_text_field_row_s_field_keeps_its_width_and_the_label_gives_way() {
    let harness = rows("$(YEAR)");
    let field = harness.get_by_role_and_label(Role::TextInput, "Import name");
    assert_eq!(field.rect().width(), TEXT_FIELD_WIDTH);
    // Its end on the row's.
    assert_eq!(field.rect().right(), 8.0 + 296.0);
}

#[test]
fn a_long_pattern_scrolls_inside_the_field_and_the_row_keeps_its_two_lines() {
    let long = "/Volumes/Photos NAS/Clients/2026/Weddings/$(YEAR)$(MONTH)$(DAY)_$(IMPORT_NAME)";
    let mut harness = rows(long);
    let field = harness.get_by_role_and_label(Role::TextInput, "Subfolder");
    let rect = field.rect();
    assert_eq!(rect.height(), 18.0);
    // The field, its warning button, ⌄ and folder button, 4 points apart, fill the width.
    assert_eq!(rect.width(), 296.0 - 3.0 * (18.0 + 4.0));
    // The label's line and the field's: never wrapped.
    assert_eq!(harness.state().row_height, 48.0);
    field.focus();
    harness.run();
    harness.key_press(Key::End);
    harness.run();
    let after = harness.get_by_role_and_label(Role::TextInput, "Subfolder");
    assert_eq!(after.rect(), rect);
    assert_eq!(after.accesskit_node().value().as_deref(), Some(long));
}

#[test]
fn a_disabled_row_draws_no_warning_button_and_says_why() {
    let mut harness = rows("$(YEAR)$(MONT)");
    harness.state_mut().disabled = true;
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(Role::Button, PROBLEM)
            .is_none()
    );
    let field = harness.get_by_role_and_label(Role::TextInput, "Subfolder");
    assert!(field.accesskit_node().is_disabled());
    assert_eq!(
        field.accesskit_node().description().as_deref(),
        Some("Comes in a later version")
    );
    // Its ⌄ and folder button are drawn disabled, never hidden.
    let folder = harness.get_by_role_and_label(Role::Button, "Choose a folder");
    assert!(folder.accesskit_node().is_disabled());
}

/// The gallery's Setting rows, `width` points wide, in `which`, at twice the
/// size; `then` puts them in the state photographed. The rows have no words of their own,
/// their labels and problems being their caller's, so they are photographed in English
/// alone; the sheet's snapshots show them in the other languages beside its own words.
fn photograph(
    name: &str,
    which: Builtin,
    width: f32,
    then: fn(&mut Harness<'_, Gallery>),
) -> SnapshotResults {
    let mut harness = Harness::builder()
        .with_size(vec2(width + 24.0, 400.0))
        .with_pixels_per_point(2.0)
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::builtin(which));
        })
        .build_ui_state(
            move |ui, gallery: &mut Gallery| {
                let panel = apertrust_ui::theme(ui.ctx()).colors.panel;
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                egui::Frame::new()
                    .inner_margin(12.0)
                    .show(ui, |ui| gallery.show_setting_rows(ui, width));
            },
            Gallery::new(which),
        );
    harness.run();
    then(&mut harness);
    harness.run();
    harness.snapshot(name);
    harness.take_snapshot_results()
}

fn at_rest(_: &mut Harness<'_, Gallery>) {}

#[test]
fn the_setting_rows_at_296_and_440_points() {
    let mut results = SnapshotResults::new();
    for (which, theme) in [(Builtin::Dark, "dark"), (Builtin::Light, "light")] {
        for width in [296.0, 440.0] {
            let name = format!("setting-rows-{width}-{theme}");
            results.extend(photograph(&name, which, width, at_rest));
        }
    }
    results.unwrap();
}

#[test]
fn the_setting_rows_focused_and_with_the_choices_open() {
    let mut results = SnapshotResults::new();
    results.extend(photograph(
        "setting-rows-focused",
        Builtin::Dark,
        296.0,
        |harness| {
            harness
                .get_all_by_role_and_label(Role::TextInput, "Import name")
                .next()
                .expect("the first Import name")
                .focus();
            harness.run();
            harness.key_press(Key::End);
        },
    ));
    for (which, name) in [
        (Builtin::Dark, "setting-rows-choices-dark"),
        (Builtin::Light, "setting-rows-choices-light"),
    ] {
        results.extend(photograph(name, which, 296.0, |harness| {
            harness
                .get_all_by_role_and_label(Role::Button, "Subfolders")
                .next()
                .expect("Subfolder's ⌄")
                .click();
        }));
    }
    results.unwrap();
}
