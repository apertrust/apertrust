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

//! The field row (decision 110): how editing begins and ends, what it reports, and how it
//! looks in each state.

mod common;

use apertrust_ui::components::FieldWell;
use apertrust_ui::gallery::Gallery;
use apertrust_ui::{Builtin, Theme};
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

/// What the fields reported: each commit by field, and each cancel.
#[derive(Default)]
struct Reported {
    title: String,
    notes: String,
    committed: Vec<(&'static str, String)>,
    cancelled: usize,
}

/// Title, one line, and Notes, which takes line breaks, above a button, each showing what
/// its last commit left.
fn fields() -> Harness<'static, Reported> {
    Harness::builder()
        .with_size(egui::vec2(320.0, 240.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, state: &mut Reported| {
                for (name, lines) in [("Title", false), ("Notes", true)] {
                    let text = if lines { &state.notes } else { &state.title };
                    let id = egui::Id::unique(name);
                    let out = FieldWell::new(id, name, &text.clone(), 80.0)
                        .lines(lines)
                        .show(ui);
                    state.cancelled += usize::from(out.cancelled);
                    if let Some(text) = out.committed {
                        state.committed.push((name, text.clone()));
                        match lines {
                            true => state.notes = text,
                            false => state.title = text,
                        }
                    }
                }
                let _ = ui.button("After");
            },
            Reported {
                title: "Morning market".to_owned(),
                ..Reported::default()
            },
        )
}

/// Clicks `name`'s well and types `text` at the end of what it holds.
fn type_into(harness: &mut Harness<'_, Reported>, name: &str, text: &str) {
    harness.get_by_label(name).click();
    harness.run();
    harness.key_press(Key::End);
    harness.run();
    harness.get_by_label(name).type_text(text);
    harness.run();
}

#[test]
fn return_commits_once_and_gives_the_focus_up() {
    let mut harness = fields();
    harness.run();
    type_into(&mut harness, "Title", ", Alfama");
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(
        harness.state().committed,
        [("Title", "Morning market, Alfama".to_owned())]
    );
    assert!(!harness.get_by_label("Title").is_focused());
    // A value unchanged commits nothing.
    harness.get_by_label("Title").click();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().committed.len(), 1);
}

#[test]
fn esc_puts_the_field_back_and_commits_nothing() {
    let mut harness = fields();
    harness.run();
    type_into(&mut harness, "Title", " gone");
    harness.key_press(Key::Escape);
    harness.run();
    harness.run();
    assert_eq!(harness.state().committed, []);
    assert_eq!(harness.state().cancelled, 1);
    let value = harness.get_by_label("Title").accesskit_node().value();
    assert_eq!(value.as_deref(), Some("Morning market"));
}

#[test]
fn shift_return_breaks_a_line_where_lines_are_taken_and_commits_elsewhere() {
    let mut harness = fields();
    harness.run();
    type_into(&mut harness, "Notes", "Warmer");
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Enter);
    harness.run();
    harness.get_by_label("Notes").type_text("crop");
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(
        harness.state().committed,
        [("Notes", "Warmer\ncrop".to_owned())]
    );
    type_into(&mut harness, "Title", "!");
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Enter);
    harness.run();
    assert_eq!(
        harness.state().committed.last(),
        Some(&("Title", "Morning market!".to_owned()))
    );
}

#[test]
fn tab_and_a_click_elsewhere_commit() {
    let mut harness = fields();
    harness.run();
    type_into(&mut harness, "Title", " at dawn");
    harness.key_press(Key::Tab);
    harness.run();
    harness.run();
    assert_eq!(
        harness.state().committed,
        [("Title", "Morning market at dawn".to_owned())]
    );
    // Tab gave Notes the focus, editing with its text selected.
    harness.get_by_label("Notes").type_text("Try a warmer crop");
    harness.run();
    harness.get_by_label("After").click();
    harness.run();
    harness.run();
    assert_eq!(
        harness.state().committed.last(),
        Some(&("Notes", "Try a warmer crop".to_owned()))
    );
}

/// The gallery's field rows at `width`, in the dark and light themes.
fn photographed(width: f32) {
    let size = egui::vec2(width + 24.0, 300.0);
    let show: fn(&mut Gallery, &mut egui::Ui) = match width as u32 {
        240 => |gallery, ui| gallery.show_field_wells(ui, 240.0),
        320 => |gallery, ui| gallery.show_field_wells(ui, 320.0),
        _ => |gallery, ui| gallery.show_field_wells(ui, 600.0),
    };
    let mut results = egui_kittest::SnapshotResults::new();
    for which in [Builtin::Dark, Builtin::Light] {
        let name = format!("field-well-{}-{}", width as u32, which.id());
        results.extend(common::photograph_section(which, size, &name, show));
    }
}

#[test]
fn field_rows_are_photographed_at_240() {
    photographed(240.0);
}

#[test]
fn field_rows_are_photographed_at_320() {
    photographed(320.0);
}

#[test]
fn field_rows_are_photographed_at_600() {
    photographed(600.0);
}

/// A pull-down's well keeps its warning button at its end, as a text's does.
#[test]
fn a_pull_down_shows_its_warning_button() {
    use apertrust_ui::popup::{Entry, Item};
    const OPTIONS: [(&str, &str); 2] = [("", "Unknown"), ("True", "Copyrighted")];
    let mut harness = Harness::builder()
        .with_size(egui::vec2(320.0, 80.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui(|ui| {
            let menu = vec![Entry::Item(Item::new("try-again", "Try Again"))];
            FieldWell::new(
                egui::Id::unique("status"),
                "Copyright status",
                "True",
                100.0,
            )
            .choice(&OPTIONS)
            .warning("Copyright status was not saved", menu)
            .show(ui);
        });
    harness.run();
    let warning = harness.query_by_label("Copyright status was not saved");
    assert!(warning.is_some());
}

/// Ctrl+Return breaks the line in a field that takes line breaks, by the Control key on
/// every system; Cmd+Return on the Mac commits, as Return does.
#[test]
fn ctrl_return_breaks_the_line_and_cmd_return_commits() {
    let mut harness = fields();
    harness.run();
    type_into(&mut harness, "Notes", "Warmer");
    harness.key_press_modifiers(Modifiers::CTRL, Key::Enter);
    harness.run();
    harness.get_by_label("Notes").type_text("crop");
    harness.run();
    harness.key_press_modifiers(Modifiers::MAC_CMD, Key::Enter);
    harness.run();
    assert_eq!(
        harness.state().committed,
        [("Notes", "Warmer\ncrop".to_owned())]
    );
}

/// A draft forgotten, as the Metadata tool forgets one as the photo changes, leaves no undo
/// of its typing: Undo in the next draft never brings it back.
#[test]
fn a_forgotten_draft_leaves_nothing_to_undo() {
    let mut harness = fields();
    harness.run();
    type_into(&mut harness, "Title", " on photo A");
    FieldWell::forget(&harness.ctx, egui::Id::unique("Title"));
    harness.state_mut().title = "Harbour".to_owned();
    harness.run();
    harness.get_by_label("Title").click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().committed, []);
    let value = harness.get_by_label("Title").accesskit_node().value();
    assert_eq!(value.as_deref(), Some("Harbour"));
}
