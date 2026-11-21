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

//! The Settings dialog's slider (`docs/gui.md`, The Settings dialog), driven by the pointer,
//! the keys and typing, and read through AccessKit: Interface size, 50 to 300 % in steps of
//! 10.

use apertrust_ui::components::settings::Slider;
use apertrust_ui::{Theme, install};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

const NAME: &str = "Interface size";

/// The slider in a column 260 points wide, at `value` per cent.
fn slider(value: f64) -> Harness<'static, f64> {
    let mut harness = Harness::builder()
        .with_size(vec2(280.0, 60.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx| install(ctx, Theme::default()))
        .build_ui_state(
            |ui, value: &mut f64| {
                ui.set_width(260.0);
                let set = Slider::new("scale", NAME, *value, 50.0..=300.0)
                    .step(10.0)
                    .unit("%")
                    .show(ui);
                if let Some(set) = set {
                    *value = set;
                }
            },
            value,
        );
    harness.run();
    harness
}

fn track(harness: &Harness<'_, f64>) -> egui::Rect {
    harness.get_by_role_and_label(Role::Slider, NAME).rect()
}

fn button(harness: &mut Harness<'_, f64>, pos: Pos2, pressed: bool) {
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    });
    harness.run();
}

#[test]
fn a_click_on_the_track_jumps_and_a_drag_sets_the_value() {
    let mut harness = slider(110.0);
    let rect = track(&harness);
    // Near the track's end: the range's end.
    let end = pos2(rect.right() - 3.0, rect.center().y);
    harness.hover_at(end);
    button(&mut harness, end, true);
    button(&mut harness, end, false);
    assert_eq!(*harness.state(), 300.0);
    // Pressed near the start and dragged to the middle, on a step.
    let start = pos2(rect.left() + 3.0, rect.center().y);
    harness.hover_at(start);
    button(&mut harness, start, true);
    assert_eq!(*harness.state(), 50.0);
    let middle = pos2(rect.center().x, rect.center().y);
    harness.event(Event::PointerMoved(middle));
    harness.run();
    button(&mut harness, middle, false);
    let value = *harness.state();
    assert!((160.0..=190.0).contains(&value), "{value}");
    assert_eq!(value % 10.0, 0.0, "{value}");
}

#[test]
fn the_arrows_step_shift_ten_home_and_end_go_to_its_ends() {
    let mut harness = slider(110.0);
    harness.get_by_role_and_label(Role::Slider, NAME).focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(*harness.state(), 120.0);
    harness.key_press(Key::ArrowLeft);
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert_eq!(*harness.state(), 100.0);
    harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowRight);
    harness.run();
    assert_eq!(*harness.state(), 200.0);
    // Held to the range.
    harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowRight);
    harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowRight);
    harness.run();
    assert_eq!(*harness.state(), 300.0);
    harness.key_press(Key::Home);
    harness.run();
    assert_eq!(*harness.state(), 50.0);
    harness.key_press(Key::End);
    harness.run();
    assert_eq!(*harness.state(), 300.0);
}

/// Opens the value's field with a click on the value at the track's end.
fn open_field(harness: &mut Harness<'_, f64>) {
    let rect = track(harness);
    let value = pos2(rect.right() + 30.0, rect.center().y);
    harness.hover_at(value);
    button(harness, value, true);
    button(harness, value, false);
    harness.run();
}

#[test]
fn a_click_on_the_value_opens_its_field_the_number_selected() {
    let mut harness = slider(110.0);
    open_field(&mut harness);
    let field = harness.get_by_role_and_label(Role::TextInput, "Interface size, in %");
    assert!(field.accesskit_node().is_focused());
    // What is typed replaces the number: it was selected.
    field.type_text("150");
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(*harness.state(), 150.0);
    assert!(
        harness
            .query_by_role_and_label(Role::TextInput, "Interface size, in %")
            .is_none(),
        "the field closes"
    );
    // Esc puts the value back.
    open_field(&mut harness);
    harness
        .get_by_role_and_label(Role::TextInput, "Interface size, in %")
        .type_text("200");
    harness.key_press(Key::Escape);
    harness.run();
    harness.run();
    assert_eq!(*harness.state(), 150.0);
    // A value past the range is held to it.
    open_field(&mut harness);
    harness
        .get_by_role_and_label(Role::TextInput, "Interface size, in %")
        .type_text("999");
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(*harness.state(), 300.0);
}

#[test]
fn screen_readers_hear_a_slider_with_its_value_and_range() {
    let harness = slider(110.0);
    let node = harness.get_by_role_and_label(Role::Slider, NAME);
    let node = node.accesskit_node();
    assert_eq!(node.value().as_deref(), Some("110 %"));
    assert_eq!(node.numeric_value(), Some(110.0));
    assert_eq!(node.min_numeric_value(), Some(50.0));
    assert_eq!(node.max_numeric_value(), Some(300.0));
    assert_eq!(node.numeric_value_step(), Some(10.0));
}

/// A range that is no whole number of steps: the pointer still reaches its end, as End
/// does.
#[test]
fn the_pointer_reaches_the_end_of_a_range_of_no_whole_steps() {
    let mut harness = Harness::builder()
        .with_size(vec2(280.0, 60.0))
        .with_context_setup(|ctx| install(ctx, Theme::default()))
        .build_ui_state(
            |ui, value: &mut f64| {
                ui.set_width(260.0);
                if let Some(set) = Slider::new("ten", "Ten", *value, 0.0..=10.0)
                    .step(3.0)
                    .show(ui)
                {
                    *value = set;
                }
            },
            3.0,
        );
    harness.run();
    let rect = harness.get_by_role_and_label(Role::Slider, "Ten").rect();
    let end = pos2(rect.right() - 1.0, rect.center().y);
    harness.hover_at(end);
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: end,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    harness.run();
    assert_eq!(*harness.state(), 10.0);
}

/// The pointer at either end sets the range's end exactly, though its decimals would round
/// it or its span, added to its start, falls short of it (-1 + 1.2 is 0.19999…), as Home
/// and End do.
#[test]
fn the_pointer_at_either_end_sets_it_exactly() {
    for (range, step, start) in [(0.26..=1.24, 0.1, 0.6), (-1.0..=0.2, 0.5, -0.5)] {
        let (min, max) = (*range.start(), *range.end());
        let mut harness = Harness::builder()
            .with_size(vec2(280.0, 60.0))
            .with_context_setup(|ctx| install(ctx, Theme::default()))
            .build_ui_state(
                move |ui, value: &mut f64| {
                    ui.set_width(260.0);
                    if let Some(set) = Slider::new("odd", "Odd", *value, range.clone())
                        .step(step)
                        .decimals(1)
                        .show(ui)
                    {
                        *value = set;
                    }
                },
                start,
            );
        harness.run();
        let rect = harness.get_by_role_and_label(Role::Slider, "Odd").rect();
        for (x, end) in [(rect.left() + 1.0, min), (rect.right() - 1.0, max)] {
            let at = pos2(x, rect.center().y);
            harness.hover_at(at);
            for pressed in [true, false] {
                harness.event(Event::PointerButton {
                    pos: at,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                });
            }
            harness.run();
            assert_eq!(*harness.state(), end, "{min} to {max}");
        }
    }
}

/// A drag let go at the track's start, the pointer moving on in the same pass, sets the
/// value where it was let go, as the grip of a text area does.
#[test]
fn a_drag_ends_where_it_is_let_go() {
    let mut harness = slider(200.0);
    let rect = track(&harness);
    let middle = rect.center();
    let start = pos2(rect.left() + 1.0, middle.y);
    harness.hover_at(middle);
    button(&mut harness, middle, true);
    harness.hover_at(pos2(middle.x - 20.0, middle.y));
    harness.run();
    assert_ne!(*harness.state(), 200.0, "dragged");
    for event in [
        Event::PointerMoved(start),
        Event::PointerButton {
            pos: start,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
        Event::PointerMoved(pos2(rect.right() - 1.0, middle.y)),
    ] {
        harness.input_mut().events.push(event);
    }
    harness.run();
    assert_eq!(*harness.state(), 50.0);
}

/// The slider in a page that scrolls, 60 points of it shown, at 110 %.
fn in_a_page() -> Harness<'static, f64> {
    let mut harness = Harness::builder()
        .with_size(vec2(280.0, 80.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx| install(ctx, Theme::default()))
        .build_ui_state(
            |ui, value: &mut f64| {
                egui::ScrollArea::vertical()
                    .max_height(60.0)
                    .show(ui, |ui| {
                        ui.set_width(260.0);
                        let set = Slider::new("scale", NAME, *value, 50.0..=300.0)
                            .step(10.0)
                            .unit("%")
                            .show(ui);
                        if let Some(set) = set {
                            *value = set;
                        }
                        ui.add_space(400.0);
                    });
            },
            110.0,
        );
    harness.run();
    harness
}

/// The wheel over the slider scrolls the page and never moves the slider, focused or not:
/// a page scrolled past it must not change the interface's size.
#[test]
fn the_wheel_over_the_slider_scrolls_the_page_and_changes_nothing() {
    for focused in [false, true] {
        let mut harness = in_a_page();
        if focused {
            harness.get_by_role_and_label(Role::Slider, NAME).focus();
            harness.run();
        }
        let before = track(&harness);
        harness.hover_at(before.center());
        harness.run();
        for _ in 0..3 {
            harness.event(Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: vec2(0.0, -1.0),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Wheel,
                modifiers: Modifiers::NONE,
            });
            harness.run_steps(10);
        }
        assert_eq!(*harness.state(), 110.0, "focused {focused}");
        assert!(track(&harness).top() < before.top(), "the page scrolled");
    }
}

/// The track takes the primary button alone: a secondary or middle press, or a drag with
/// one, changes nothing.
#[test]
fn only_the_primary_button_moves_it() {
    for other in [PointerButton::Secondary, PointerButton::Middle] {
        let mut harness = slider(110.0);
        let rect = track(&harness);
        let end = pos2(rect.right() - 3.0, rect.center().y);
        let start = pos2(rect.left() + 3.0, rect.center().y);
        harness.hover_at(end);
        harness.run();
        harness.event(Event::PointerButton {
            pos: end,
            button: other,
            pressed: true,
            modifiers: Modifiers::NONE,
        });
        harness.run();
        harness.hover_at(start);
        harness.run();
        harness.event(Event::PointerButton {
            pos: start,
            button: other,
            pressed: false,
            modifiers: Modifiers::NONE,
        });
        harness.run();
        assert_eq!(*harness.state(), 110.0, "{other:?}");
        // A click where it stands, near an end: neither the press nor the release.
        harness.hover_at(end);
        harness.run();
        for pressed in [true, false] {
            harness.event(Event::PointerButton {
                pos: end,
                button: other,
                pressed,
                modifiers: Modifiers::NONE,
            });
            harness.run();
            assert_eq!(*harness.state(), 110.0, "{other:?}, pressed {pressed}");
        }
    }
}

/// A press held still past egui's click time, then dragged, still drags.
#[test]
fn a_press_held_still_then_dragged_still_drags() {
    let mut harness = slider(110.0);
    let rect = track(&harness);
    let from = pos2(rect.left() + rect.width() / 4.0, rect.center().y);
    harness.hover_at(from);
    button(&mut harness, from, true);
    let pressed = *harness.state();
    harness.run_steps(90);
    let end = pos2(rect.right() - 3.0, rect.center().y);
    harness.hover_at(end);
    harness.run();
    assert_eq!(*harness.state(), 300.0, "from {pressed}");
    button(&mut harness, end, false);
}

/// A value typed in a slider with a grid, its kept values as shown, is rounded to the
/// nearest of them, half way rounding up as the store rounds: 0.4 steps from 0 to 4, so
/// 0.6 is 0.8, 0.5 is 0.4 and 0.4 stays.
#[test]
fn a_value_typed_is_rounded_to_its_grid() {
    for (typed, kept) in [("0.6", 0.8), ("0.5", 0.4), ("0.4", 0.4), ("1.0", 1.2)] {
        let mut harness = Harness::builder()
            .with_size(vec2(280.0, 60.0))
            .with_step_dt(1.0 / 60.0)
            .with_context_setup(|ctx| install(ctx, Theme::default()))
            .build_ui_state(
                |ui, value: &mut f64| {
                    ui.set_width(260.0);
                    let set = Slider::new("limit", NAME, *value, 0.0..=4.0)
                        .step(0.4)
                        .grid(0.4)
                        .decimals(1)
                        .unit("%")
                        .show(ui);
                    if let Some(set) = set {
                        *value = set;
                    }
                },
                2.0,
            );
        harness.run();
        open_field(&mut harness);
        harness
            .get_by_role_and_label(Role::TextInput, "Interface size, in %")
            .type_text(typed);
        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(*harness.state(), kept, "{typed}");
    }
}

/// A click on the value opens its field in the pass after it only: a slider not drawn
/// then, its page left, drops the click, and comes back with its field closed and the
/// focus where it was.
#[test]
fn a_click_on_the_value_left_undrawn_opens_nothing_later() {
    let mut harness = Harness::builder()
        .with_size(vec2(280.0, 60.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx| install(ctx, Theme::default()))
        .build_ui_state(
            |ui, (value, shown): &mut (f64, bool)| {
                ui.set_width(260.0);
                if !*shown {
                    return;
                }
                let set = Slider::new("scale", NAME, *value, 50.0..=300.0)
                    .step(10.0)
                    .unit("%")
                    .show(ui);
                if let Some(set) = set {
                    *value = set;
                }
            },
            (110.0, true),
        );
    harness.run();
    let rect = harness.get_by_role_and_label(Role::Slider, NAME).rect();
    let value = pos2(rect.right() + 30.0, rect.center().y);
    harness.hover_at(value);
    harness.step();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: value,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    // The press and the release with its click, a pass each, the click kept in the
    // second: the slider is left before the pass that would open its field.
    harness.step();
    harness.state_mut().1 = false;
    harness.run_steps(5);
    harness.state_mut().1 = true;
    for _ in 0..5 {
        harness.step();
        let opened = harness.query_by_role_and_label(Role::TextInput, "Interface size, in %");
        assert!(opened.is_none(), "the field opened by itself");
    }
}

/// Tab from the track reaches the value, which shows the focus ring; the focus alone opens
/// nothing, and Space opens its field.
#[test]
fn the_value_shows_the_focus_and_opens_on_a_key() {
    let mut harness = slider(110.0);
    harness.get_by_role_and_label(Role::Slider, NAME).focus();
    harness.run();
    let before = rings(&harness);
    harness.key_press(Key::Tab);
    harness.run();
    let slider = harness.get_by_role_and_label(Role::Slider, NAME);
    assert!(!slider.accesskit_node().is_focused());
    let after = rings(&harness);
    assert_eq!(before, 1, "the track's ring");
    assert_eq!(after, 1, "the value's ring");
    assert!(
        harness
            .query_by_role_and_label(Role::TextInput, "Interface size, in %")
            .is_none(),
        "the focus alone opens nothing"
    );
    harness.key_press(Key::Space);
    harness.run();
    let field = harness.get_by_role_and_label(Role::TextInput, "Interface size, in %");
    assert!(field.accesskit_node().is_focused());
}

/// How many focus rings are painted: strokes in the theme's `focus_ring`.
fn rings(harness: &Harness<'_, f64>) -> usize {
    let ring = Theme::default().colors.focus_ring;
    harness
        .output()
        .shapes
        .iter()
        .filter(|clipped| match &clipped.shape {
            egui::Shape::Rect(r) => r.stroke.color == ring && r.stroke.width > 1.0,
            _ => false,
        })
        .count()
}

/// To screen readers the value, a focus stop of its own, is a button named by the row's
/// label, its value as written with its unit, and it follows the value.
#[test]
fn screen_readers_hear_the_value_named_with_its_unit() {
    let mut harness = slider(110.0);
    let value = harness.get_by_role_and_label(Role::Button, NAME);
    assert_eq!(value.accesskit_node().value().as_deref(), Some("110 %"));
    harness.get_by_role_and_label(Role::Slider, NAME).focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    let value = harness.get_by_role_and_label(Role::Button, NAME);
    assert_eq!(value.accesskit_node().value().as_deref(), Some("120 %"));
}

/// The arrows in one pass step in the order they came, each with the modifiers it was
/// pressed with and held in the range before the next. Each case: the value, the presses
/// with their modifiers, those held as the pass ends, and the value after.
#[test]
fn the_arrows_step_in_order_with_their_own_modifiers() {
    type Presses = &'static [(Key, Modifiers)];
    let cases: [(f64, Presses, Modifiers, f64); 6] = [
        // Home and End in the order they came, among the arrows.
        (
            110.0,
            &[(Key::End, Modifiers::NONE), (Key::Home, Modifiers::NONE)],
            Modifiers::NONE,
            50.0,
        ),
        (
            110.0,
            &[(Key::Home, Modifiers::NONE), (Key::End, Modifiers::NONE)],
            Modifiers::NONE,
            300.0,
        ),
        (
            110.0,
            &[
                (Key::End, Modifiers::NONE),
                (Key::ArrowLeft, Modifiers::NONE),
            ],
            Modifiers::NONE,
            290.0,
        ),
        // Shift let go before the pass: the press still had it.
        (
            110.0,
            &[(Key::ArrowRight, Modifiers::SHIFT)],
            Modifiers::NONE,
            210.0,
        ),
        // At the end, → stays, ← steps down.
        (
            300.0,
            &[
                (Key::ArrowRight, Modifiers::NONE),
                (Key::ArrowLeft, Modifiers::NONE),
            ],
            Modifiers::NONE,
            290.0,
        ),
        (
            110.0,
            &[
                (Key::ArrowRight, Modifiers::NONE),
                (Key::ArrowRight, Modifiers::NONE),
            ],
            Modifiers::NONE,
            130.0,
        ),
    ];
    for (from, presses, held, to) in cases {
        let mut harness = slider(from);
        harness.get_by_role_and_label(Role::Slider, NAME).focus();
        harness.run();
        let input = harness.input_mut();
        for &(key, modifiers) in presses {
            for pressed in [true, false] {
                input.events.push(Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers,
                });
            }
        }
        input.events.push(Event::ModifiersChanged(held));
        harness.step();
        harness.run();
        assert_eq!(*harness.state(), to, "{from}, {presses:?}");
    }
}

/// In a column too narrow for the thumb to travel beside the value, the track takes no
/// pointer, a click on it changing nothing, and the value, typed, still sets it. In the
/// column's own width the track takes the pointer.
#[test]
fn a_track_with_no_room_to_travel_takes_no_pointer() {
    for (width, moves) in [(260.0, true), (84.0, false)] {
        let mut harness = Harness::builder()
            .with_size(vec2(width + 20.0, 60.0))
            .with_step_dt(1.0 / 60.0)
            .with_context_setup(|ctx| install(ctx, Theme::default()))
            .build_ui_state(
                move |ui, value: &mut f64| {
                    ui.set_width(width);
                    let set = Slider::new("scale", NAME, *value, 50.0..=300.0)
                        .step(10.0)
                        .unit("%")
                        .show(ui);
                    if let Some(set) = set {
                        *value = set;
                    }
                },
                110.0,
            );
        harness.run();
        let rect = track(&harness);
        let near_end = pos2(rect.right() - 1.0, rect.center().y);
        harness.hover_at(near_end);
        button(&mut harness, near_end, true);
        button(&mut harness, near_end, false);
        assert_eq!(
            *harness.state() != 110.0,
            moves,
            "{width}: {}",
            harness.state()
        );
        if !moves {
            open_field(&mut harness);
            harness
                .get_by_role_and_label(Role::TextInput, "Interface size, in %")
                .type_text("150");
            harness.key_press(egui::Key::Enter);
            harness.run();
            assert_eq!(*harness.state(), 150.0, "{width}: the value still sets it");
        }
    }
}
