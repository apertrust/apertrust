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

//! The Settings dialog's boxes and wells (`docs/gui.md`, The Settings dialog): the combo,
//! the segmented choice, the text field, the text area and the folder, each 24 points high
//! and the column wide, in their states.

use apertrust_ui::components::settings::{Combo, Folder, Segmented, Text, TextArea};
use apertrust_ui::{Theme, install};
use egui::accesskit::{Role, Toggled};
use egui::{Rect, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

/// The column's width, as a settings row gives its control.
const COLUMN: f32 = 260.0;

/// What the boxes hold.
#[derive(Default)]
struct State {
    theme: usize,
    marker: usize,
    pattern: String,
    note: String,
    folder: String,
    chose_folder: bool,
    enabled: bool,
}

const THEMES: [&str; 4] = ["Dark", "Grey", "Light", "High contrast"];
const SHAPES: [&str; 3] = ["Triangle", "Circle", "Square"];

fn boxes(note: &str, enabled: bool) -> Harness<'static, State> {
    let mut harness = Harness::builder()
        .with_size(vec2(COLUMN + 20.0, 400.0))
        .with_context_setup(|ctx| install(ctx, Theme::default()))
        .build_ui_state(
            |ui, state: &mut State| {
                ui.set_width(COLUMN);
                ui.spacing_mut().item_spacing.y = 8.0;
                if !state.enabled {
                    ui.disable();
                }
                if let Some(i) = Combo::new("Theme", &THEMES, Some(state.theme)).show(ui) {
                    state.theme = i;
                }
                if let Some(i) =
                    Segmented::new("Slider marker", &SHAPES, Some(state.marker)).show(ui)
                {
                    state.marker = i;
                }
                if let Some(text) = Text::new("pattern", "File name", &state.pattern).show(ui) {
                    state.pattern = text;
                }
                if let Some(text) = TextArea::new("note", "Note in each log", &state.note).show(ui)
                {
                    state.note = text;
                }
                let out =
                    Folder::new("folder", "Export to", &state.folder, "Choose folder").show(ui);
                if let Some(text) = out.text {
                    state.folder = text;
                }
                state.chose_folder |= out.choose;
            },
            State {
                note: note.to_owned(),
                enabled,
                ..State::default()
            },
        );
    harness.run();
    harness
}

fn rect(harness: &Harness<'_, State>, role: Role, name: &str) -> Rect {
    harness.get_by_role_and_label(role, name).rect()
}

#[test]
fn each_box_is_24_high_and_the_column_wide() {
    let harness = boxes("", true);
    let combo = rect(&harness, Role::ComboBox, "Theme");
    let segmented = rect(&harness, Role::RadioGroup, "Slider marker");
    let text = rect(&harness, Role::TextInput, "File name");
    for (name, r) in [("combo", combo), ("segmented", segmented), ("text", text)] {
        assert_eq!((r.width(), r.height()), (COLUMN, 24.0), "{name}");
    }
    // The folder's field and its button, a box high, end at the column's end.
    let field = rect(&harness, Role::TextInput, "Export to");
    let button = rect(&harness, Role::Button, "Choose folder");
    assert_eq!(field.height(), 24.0);
    assert_eq!(button.size(), vec2(24.0, 24.0));
    assert_eq!(button.right(), combo.right());
    assert_eq!(field.left(), combo.left());
    // The segments are its group's, for screen readers, and share its width.
    let group = harness.get_by_role_and_label(Role::RadioGroup, "Slider marker");
    assert_eq!(
        group.query_all_by_role(Role::RadioButton).count(),
        SHAPES.len()
    );
    let segments: Vec<Rect> = SHAPES
        .iter()
        .map(|name| rect(&harness, Role::RadioButton, name))
        .collect();
    for pair in segments.windows(2) {
        assert!((pair[0].width() - pair[1].width()).abs() < 0.01);
        assert!((pair[1].left() - pair[0].right()).abs() < 0.01);
    }
}

#[test]
fn a_text_area_is_three_lines_at_rest_growing_to_six() {
    // The area's height: from the field before it to the folder after it, less their gaps.
    let height = |note: &str| {
        let harness = boxes(note, true);
        let above = rect(&harness, Role::TextInput, "File name");
        let below = rect(&harness, Role::TextInput, "Export to");
        below.top() - above.bottom() - 2.0 * 8.0
    };
    // The field inside the well's hairline.
    let harness = boxes("One line", true);
    let node = harness.get_by_role_and_label(Role::MultilineTextInput, "Note in each log");
    assert_eq!(node.rect().width(), COLUMN - 2.0);
    let rest = height("One line");
    assert_eq!(rest, height("One\ntwo\nthree"));
    assert!((3.0 * 15.0..4.0 * 18.0).contains(&rest), "{rest}");
    let six = height(&"A line\n".repeat(5));
    let line = (six - rest) / 3.0;
    assert!((15.0..20.0).contains(&line), "{line}");
    // Past six lines it scrolls, no taller.
    assert_eq!(height(&"A line\n".repeat(12)), six);
}

/// The grip at the text area's bottom end follows the pointer where it is, the movement and
/// the release coming in one pass included, never under three lines.
#[test]
fn the_text_area_s_grip_follows_the_pointer() {
    use egui::{Event, Modifiers, PointerButton, pos2};
    let area = |harness: &Harness<'_, State>| {
        let above = rect(harness, Role::TextInput, "File name");
        let below = rect(harness, Role::TextInput, "Export to");
        below.top() - above.bottom() - 2.0 * 8.0
    };
    for (by, grown) in [(40.0, true), (-200.0, false)] {
        let mut harness = boxes("One line", true);
        let rest = area(&harness);
        let well = rect(&harness, Role::MultilineTextInput, "Note in each log");
        let grip = pos2(well.right() - 4.0, well.bottom() - 4.0);
        harness.hover_at(grip);
        harness.run();
        harness.event(Event::PointerButton {
            pos: grip,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        });
        harness.run();
        // The movement and the release in one batch of input, one pass.
        let to = grip + vec2(0.0, by);
        let events = &mut harness.input_mut().events;
        events.push(Event::PointerMoved(to));
        events.push(Event::PointerButton {
            pos: to,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        });
        harness.step();
        harness.run();
        let after = area(&harness);
        if grown {
            assert!((after - (rest + by)).abs() < 1.5, "{rest} + {by}: {after}");
        } else {
            assert_eq!(after, rest, "never under three lines");
        }
    }
}

/// The grip resizes for the primary press that began on it alone, and ends where that
/// press was let go: a secondary drag from it, the primary pressed elsewhere meanwhile,
/// resizes nothing; a release and a move after it in one pass end at the release.
#[test]
fn the_text_area_s_grip_is_the_primary_press_s_own() {
    use egui::{Event, Modifiers, PointerButton, pos2};
    let area = |harness: &Harness<'_, State>| {
        let above = rect(harness, Role::TextInput, "File name");
        let below = rect(harness, Role::TextInput, "Export to");
        below.top() - above.bottom() - 2.0 * 8.0
    };
    let button = |pos, button, pressed| Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: Modifiers::NONE,
    };
    let start = |harness: &mut Harness<'_, State>| {
        let well = rect(harness, Role::MultilineTextInput, "Note in each log");
        let grip = pos2(well.right() - 4.0, well.bottom() - 4.0);
        harness.hover_at(grip);
        harness.run();
        grip
    };
    // A secondary drag from the grip, the primary pressed elsewhere, then moved.
    let mut harness = boxes("One line", true);
    let rest = area(&harness);
    let grip = start(&mut harness);
    harness.event(button(grip, PointerButton::Secondary, true));
    harness.run();
    let elsewhere = grip - vec2(200.0, 0.0);
    harness.event(button(elsewhere, PointerButton::Primary, true));
    harness.run();
    harness.hover_at(grip + vec2(0.0, 40.0));
    harness.run();
    assert_eq!(area(&harness), rest, "a secondary drag resizes nothing");
    // A primary drag let go 40 points down, the pointer then moving on, in one pass.
    let mut harness = boxes("One line", true);
    let grip = start(&mut harness);
    harness.event(button(grip, PointerButton::Primary, true));
    harness.run();
    let down = grip + vec2(0.0, 40.0);
    let events = &mut harness.input_mut().events;
    events.push(Event::PointerMoved(down));
    events.push(button(down, PointerButton::Primary, false));
    events.push(Event::PointerMoved(down + vec2(0.0, 60.0)));
    harness.step();
    harness.run();
    let after = area(&harness);
    assert!((after - (rest + 40.0)).abs() < 1.5, "{rest} + 40: {after}");
}

#[test]
fn each_box_takes_its_input_and_none_while_disabled() {
    let mut harness = boxes("", true);
    harness
        .get_by_role_and_label(Role::ComboBox, "Theme")
        .click();
    harness.run();
    harness.get_by_label("Light").click();
    harness.run();
    assert_eq!(harness.state().theme, 2);
    let node = harness.get_by_role_and_label(Role::ComboBox, "Theme");
    assert_eq!(node.accesskit_node().value().as_deref(), Some("Light"));
    harness
        .get_by_role_and_label(Role::RadioButton, "Circle")
        .click();
    harness.run();
    assert_eq!(harness.state().marker, 1);
    let circle = harness.get_by_role_and_label(Role::RadioButton, "Circle");
    assert_eq!(circle.accesskit_node().toggled(), Some(Toggled::True));
    harness
        .get_by_role_and_label(Role::TextInput, "File name")
        .click();
    harness.run();
    harness
        .get_by_role_and_label(Role::TextInput, "File name")
        .type_text("$(FILE.NAME)");
    harness.run();
    assert_eq!(harness.state().pattern, "$(FILE.NAME)");
    harness
        .get_by_role_and_label(Role::Button, "Choose folder")
        .click();
    harness.run();
    assert!(harness.state().chose_folder);
    // Disabled, each is said so and takes nothing.
    let mut harness = boxes("", false);
    for (role, name) in [
        (Role::ComboBox, "Theme"),
        (Role::RadioButton, "Circle"),
        (Role::TextInput, "File name"),
        (Role::Button, "Choose folder"),
    ] {
        let node = harness.get_by_role_and_label(role, name);
        assert!(node.accesskit_node().is_disabled(), "{name}");
    }
    harness
        .get_by_role_and_label(Role::ComboBox, "Theme")
        .click();
    harness.run();
    assert!(harness.query_by_label("Light").is_none());
    assert_eq!(harness.state().theme, 0);
}

/// Find a setting: a field 24 high reporting each letter, Escape clearing what it holds.
#[test]
fn find_a_setting_reports_its_text_and_escape_clears_it() {
    use apertrust_ui::components::settings::Search;
    let mut harness = Harness::builder()
        .with_size(vec2(COLUMN + 20.0, 80.0))
        .with_context_setup(|ctx| install(ctx, Theme::default()))
        .build_ui_state(
            |ui, text: &mut String| {
                ui.set_width(COLUMN);
                if let Some(typed) = Search::new("find", "Find a setting", text).show(ui) {
                    *text = typed;
                }
            },
            String::new(),
        );
    harness.run();
    let field = harness.get_by_role_and_label(Role::TextInput, "Find a setting");
    assert_eq!(
        (field.rect().width(), field.rect().height()),
        (COLUMN, 24.0)
    );
    field.click();
    harness.run();
    harness
        .get_by_role_and_label(Role::TextInput, "Find a setting")
        .type_text("Log");
    harness.run();
    assert_eq!(harness.state(), "Log");
    harness.key_press(egui::Key::Escape);
    harness.run();
    harness.run();
    assert_eq!(harness.state(), "");
}

/// The wheel over a text area scrolls its text while it can, then the page: a text that
/// fits gives every notch to the page, and one that overflows gives the page what it can
/// no longer take.
#[test]
fn the_wheel_over_a_text_area_reaches_the_page_at_its_end() {
    use egui::{Event, Modifiers, MouseWheelSource, MouseWheelUnit, TouchPhase};
    for (note, notches) in [("One line", 1), (&*"A line\n".repeat(12), 30)] {
        let mut harness = Harness::builder()
            .with_size(vec2(COLUMN + 20.0, 160.0))
            .with_step_dt(1.0 / 60.0)
            .with_context_setup(|ctx| install(ctx, Theme::default()))
            .build_ui_state(
                |ui, note: &mut String| {
                    egui::ScrollArea::vertical()
                        .id_salt("page")
                        .max_height(140.0)
                        .show(ui, |ui| {
                            ui.set_width(COLUMN);
                            if let Some(text) =
                                TextArea::new("note", "Note in each log", note).show(ui)
                            {
                                *note = text;
                            }
                            ui.add_space(600.0);
                        });
                },
                note.to_owned(),
            );
        harness.run();
        let area = harness
            .get_by_role_and_label(Role::MultilineTextInput, "Note in each log")
            .rect();
        let before = area.top();
        harness.hover_at(area.center());
        harness.run();
        for _ in 0..notches {
            harness.event(Event::MouseWheel {
                unit: MouseWheelUnit::Point,
                delta: vec2(0.0, -6.0),
                phase: TouchPhase::Move,
                source: MouseWheelSource::Wheel,
                modifiers: Modifiers::NONE,
            });
            harness.run_steps(4);
        }
        let after = harness
            .get_by_role_and_label(Role::MultilineTextInput, "Note in each log")
            .rect()
            .top();
        assert!(
            after < before,
            "{notches}: the page moved, {before} to {after}"
        );
    }
}

/// A combo disabled by itself, in an enabled column, as the graphics card's while the cards
/// are listed, is said disabled and takes no focus from Tab.
#[test]
fn a_disabled_combo_takes_no_focus() {
    let mut harness = Harness::builder()
        .with_size(vec2(COLUMN + 20.0, 120.0))
        .with_context_setup(|ctx| install(ctx, Theme::default()))
        .build_ui(|ui| {
            ui.set_width(COLUMN);
            Text::new("before", "Before", "").show(ui);
            Combo::new("Graphics card", &THEMES, Some(0))
                .enabled(false)
                .show(ui);
            Text::new("after", "After", "").show(ui);
        });
    harness.run();
    let combo = harness.get_by_role_and_label(Role::ComboBox, "Graphics card");
    assert!(combo.accesskit_node().is_disabled());
    harness
        .get_by_role_and_label(Role::TextInput, "Before")
        .focus();
    harness.run();
    harness.key_press(egui::Key::Tab);
    harness.run();
    let after = harness.get_by_role_and_label(Role::TextInput, "After");
    assert!(after.accesskit_node().is_focused());
}
