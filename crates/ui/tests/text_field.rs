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

//! The text field and the path field with its list of variables (`docs/gui.md`, The Export
//! dialog, States), driven by the keys and read through AccessKit.

use apertrust_ui::components::{PathField, TextField, Variable};
use apertrust_ui::{Theme, install};
use egui::accesskit::Role;
use egui::{Event, Id, Key, Modifiers, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

const VARIABLES: [Variable; 4] = [
    Variable {
        insert: "FILE.FOLDER",
        description: "folder containing the input image",
    },
    Variable {
        insert: "FILE.NAME",
        description: "basename of the input image",
    },
    Variable {
        insert: "SEQUENCE[4,1]",
        description: "sequence number",
    },
    Variable {
        insert: "YEAR",
        description: "year",
    },
];

/// The field as the caller applies it: the text it holds, and what the list was as the
/// last Escape came, which the caller keeps from closing its dialog.
struct State {
    text: String,
    list_open_at_escape: Option<bool>,
}

fn path(text: &str) -> Harness<'static, State> {
    let mut harness = Harness::builder()
        .with_size(vec2(420.0, 260.0))
        .with_context_setup(|ctx| install(ctx, Theme::default()))
        .build_ui_state(
            |ui, state: &mut State| {
                let escape = ui.input(|i| i.key_pressed(Key::Escape));
                let out =
                    PathField::new(Id::unique("path"), "Path", &state.text, &VARIABLES).show(ui);
                if escape {
                    state.list_open_at_escape = Some(out.list_open);
                }
                if let Some(text) = out.text {
                    state.text = text;
                }
            },
            State {
                text: text.to_owned(),
                list_open_at_escape: None,
            },
        );
    harness.run();
    harness
}

fn typed(harness: &mut Harness<'_, State>, text: &str) {
    harness
        .get_by_role_and_label(Role::TextInput, "Path")
        .focus();
    harness.run();
    harness.event(Event::Text(text.to_owned()));
    harness.run();
}

#[test]
fn a_text_field_reports_the_whole_text_at_every_edit_and_shows_its_hint_while_empty() {
    let mut harness = Harness::builder()
        .with_size(vec2(300.0, 60.0))
        .with_context_setup(|ctx| install(ctx, Theme::default()))
        .build_ui_state(
            |ui, text: &mut String| {
                if let (_, Some(new)) = TextField::new(Id::unique("scale"), "Scale", text)
                    .hint("1, 1/2 or 0.5")
                    .show(ui)
                {
                    *text = new;
                }
            },
            String::new(),
        );
    harness.run();
    let field = harness.get_by_role_and_label(Role::TextInput, "Scale");
    field.focus();
    harness.run();
    harness.event(Event::Text("1".to_owned()));
    harness.run();
    harness.event(Event::Text("/2".to_owned()));
    harness.run();
    assert_eq!(harness.state(), "1/2");
}

#[test]
fn typing_after_a_dollar_and_parenthesis_lists_the_variables_the_name_begins() {
    let mut harness = path("");
    assert!(harness.query_by_role(Role::ListBox).is_none());
    typed(&mut harness, "$(FILE");
    harness.get_by_role_and_label(Role::ListBox, "Variables");
    harness.get_by_role_and_label(Role::ListBoxOption, "FILE.FOLDER");
    harness.get_by_role_and_label(Role::ListBoxOption, "FILE.NAME");
    assert!(harness.query_by_label("YEAR").is_none());
    // A name with its brackets, found by the name alone.
    typed(&mut harness, "x");
    assert!(harness.query_by_role(Role::ListBox).is_none());
}

#[test]
fn a_click_inserts_the_variable_closed_and_keeps_the_rest_of_the_path() {
    let mut harness = path("a/$(SEQ/b");
    harness
        .get_by_role_and_label(Role::TextInput, "Path")
        .focus();
    harness.run();
    // The caret after "$(SEQ": the end of the text, then back over "/b".
    harness.key_press(Key::End);
    harness.key_press(Key::ArrowLeft);
    harness.key_press(Key::ArrowLeft);
    harness.run();
    harness
        .get_by_role_and_label(Role::ListBoxOption, "SEQUENCE[4,1]")
        .click();
    harness.run();
    assert_eq!(harness.state().text, "a/$(SEQUENCE[4,1])/b");
}

#[test]
fn the_arrows_and_enter_choose_from_the_list_with_the_focus_staying_in_the_field() {
    let mut harness = path("");
    typed(&mut harness, "$(FILE.");
    harness.key_press(Key::ArrowDown);
    harness.key_press(Key::ArrowDown);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().text, "$(FILE.NAME)");
    assert!(
        harness
            .get_by_role_and_label(Role::TextInput, "Path")
            .is_focused()
    );
    assert!(harness.query_by_role(Role::ListBox).is_none());
}

#[test]
fn escape_closes_the_list_alone_and_it_stays_closed_until_the_text_moves() {
    let mut harness = path("");
    typed(&mut harness, "$(FI");
    assert!(harness.query_by_role(Role::ListBox).is_some());
    harness.key_press_modifiers(Modifiers::NONE, Key::Escape);
    harness.run();
    assert!(harness.query_by_role(Role::ListBox).is_none());
    // Escape was the list's: the caller is told it was open as the key came, and the
    // field keeps the focus, for the next key.
    assert_eq!(harness.state().list_open_at_escape, Some(true));
    harness.run();
    assert!(harness.query_by_role(Role::ListBox).is_none());
    assert!(
        harness
            .get_by_role_and_label(Role::TextInput, "Path")
            .is_focused()
    );
    typed(&mut harness, "L");
    assert!(harness.query_by_role(Role::ListBox).is_some());
    assert_eq!(harness.state().text, "$(FIL");
}
