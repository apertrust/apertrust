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

//! The Settings dialog's number box (`docs/gui.md`, The Settings dialog), driven by its
//! buttons and by typing, and read through AccessKit and what it paints: Memory budget,
//! 256 to 2048 MiB in steps of 256.

use apertrust_ui::components::settings::Number;
use apertrust_ui::{Theme, install};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Shape, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

const NAME: &str = "Memory budget";
const FIELD: &str = "Memory budget, in MiB";

/// The box in a column 260 points wide, at `value`.
fn number(value: f64) -> Harness<'static, f64> {
    let mut harness = Harness::builder()
        .with_size(vec2(280.0, 70.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx| install(ctx, Theme::default()))
        .build_ui_state(
            |ui, value: &mut f64| {
                ui.set_width(260.0);
                let set = Number::new("budget", NAME, *value, 256.0..=2048.0)
                    .step(256.0)
                    .unit("MiB")
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

fn button(harness: &mut Harness<'_, f64>, pos: Pos2, pressed: bool, modifiers: Modifiers) {
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers,
    });
    harness.run();
}

fn centre(harness: &Harness<'_, f64>, label: &str) -> Pos2 {
    harness
        .get_by_role_and_label(Role::Button, label)
        .rect()
        .center()
}

#[test]
fn the_buttons_step_and_repeat_while_held() {
    let mut harness = number(1024.0);
    let more = centre(&harness, "Increase Memory budget");
    let less = centre(&harness, "Decrease Memory budget");
    harness.hover_at(more);
    button(&mut harness, more, true, Modifiers::NONE);
    button(&mut harness, more, false, Modifiers::NONE);
    assert_eq!(*harness.state(), 1280.0, "a click is one step");
    harness.hover_at(less);
    button(&mut harness, less, true, Modifiers::NONE);
    // Held a third of a second: no repeat yet.
    harness.run_steps(18);
    assert_eq!(*harness.state(), 1024.0);
    // Then one every 60 ms, down to the range's start, where it stops.
    harness.run_steps(12);
    let held = *harness.state();
    assert!(held < 1024.0, "{held}");
    harness.run_steps(60);
    button(&mut harness, less, false, Modifiers::NONE);
    assert_eq!(*harness.state(), 256.0);
    // Shift steps ten, held to the range.
    harness.hover_at(more);
    harness.event(Event::ModifiersChanged(Modifiers::SHIFT));
    harness.run();
    button(&mut harness, more, true, Modifiers::SHIFT);
    button(&mut harness, more, false, Modifiers::SHIFT);
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.run();
    assert_eq!(*harness.state(), 2048.0);
}

#[test]
fn a_button_is_disabled_at_its_end_of_the_range() {
    let harness = number(256.0);
    let less = harness.get_by_role_and_label(Role::Button, "Decrease Memory budget");
    assert!(less.accesskit_node().is_disabled());
    let more = harness.get_by_role_and_label(Role::Button, "Increase Memory budget");
    assert!(!more.accesskit_node().is_disabled());
    let harness = number(2048.0);
    let more = harness.get_by_role_and_label(Role::Button, "Increase Memory budget");
    assert!(more.accesskit_node().is_disabled());
}

/// Whether the box is outlined in the error colour.
fn outlined(harness: &Harness<'_, f64>) -> bool {
    let error = Theme::default().colors.error;
    harness
        .output()
        .shapes
        .iter()
        .any(|clipped| match &clipped.shape {
            Shape::Rect(r) => r.stroke.color == error,
            _ => false,
        })
}

#[test]
fn typing_sets_it_and_a_number_out_of_range_is_said_and_refused() {
    let mut harness = number(1024.0);
    harness
        .get_by_role_and_label(Role::TextInput, FIELD)
        .click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness
        .get_by_role_and_label(Role::TextInput, FIELD)
        .type_text("1536");
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(*harness.state(), 1536.0);
    assert!(!outlined(&harness));
    // Out of range while typed: outlined, the range said under it.
    harness
        .get_by_role_and_label(Role::TextInput, FIELD)
        .click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness
        .get_by_role_and_label(Role::TextInput, FIELD)
        .type_text("4096");
    harness.run();
    assert!(outlined(&harness));
    harness.get_by_role_and_label(Role::Label, "256 to 2048 MiB");
    // Set, it is refused: the last good value stays, still outlined and said.
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(*harness.state(), 1536.0);
    assert!(outlined(&harness));
    harness.get_by_role_and_label(Role::Label, "256 to 2048 MiB");
    // A step clears it.
    let more = centre(&harness, "Increase Memory budget");
    harness.hover_at(more);
    button(&mut harness, more, true, Modifiers::NONE);
    button(&mut harness, more, false, Modifiers::NONE);
    assert_eq!(*harness.state(), 1792.0);
    assert!(!outlined(&harness));
    assert!(harness.query_by_label("256 to 2048 MiB").is_none());
    // So does a value set from outside, as a reset.
    harness
        .get_by_role_and_label(Role::TextInput, FIELD)
        .click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness
        .get_by_role_and_label(Role::TextInput, FIELD)
        .type_text("9999");
    harness.key_press(Key::Enter);
    harness.run();
    assert!(outlined(&harness));
    *harness.state_mut() = 2048.0;
    harness.run();
    assert!(!outlined(&harness));
}

/// The wheel over the box scrolls the page and never changes the number: over its field
/// and over its + button, each in a fresh box, each small wheel step moving the page and
/// the box staying under the pointer.
#[test]
fn the_wheel_over_the_box_scrolls_the_page_and_changes_nothing() {
    for target in [FIELD, "Increase Memory budget"] {
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
                            let set = Number::new("budget", NAME, *value, 256.0..=2048.0)
                                .step(256.0)
                                .unit("MiB")
                                .show(ui);
                            if let Some(set) = set {
                                *value = set;
                            }
                            ui.add_space(400.0);
                        });
                },
                1024.0,
            );
        harness.run();
        let top = |harness: &Harness<'_, f64>| {
            harness
                .get_by_role_and_label(Role::TextInput, FIELD)
                .rect()
                .top()
        };
        for step in 0..2 {
            let before = top(&harness);
            let at = harness.get_by_label(target).rect().center();
            harness.hover_at(at);
            harness.run();
            harness.event(Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0.0, -8.0),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Wheel,
                modifiers: Modifiers::NONE,
            });
            harness.run_steps(10);
            assert!(top(&harness) < before, "{target}, {step}: the page moved");
            assert_eq!(*harness.state(), 1024.0, "{target}, {step}");
        }
    }
}

/// A box for a number that must be more than `above`, within `min` and `max` where given,
/// at `value`, kept with one decimal and stepped by 1; and how many values it set.
fn above(
    min: Option<f64>,
    max: Option<f64>,
    above: f64,
    value: f64,
) -> Harness<'static, (f64, usize)> {
    let range = min.unwrap_or(f64::from(f32::MIN))..=max.unwrap_or(f64::from(f32::MAX));
    let mut harness = Harness::builder()
        .with_size(vec2(280.0, 70.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx| install(ctx, Theme::default()))
        .build_ui_state(
            move |ui, (value, sets): &mut (f64, usize)| {
                ui.set_width(260.0);
                let set = Number::new("limit", "Limit", *value, range.clone())
                    .above(above)
                    .decimals(1)
                    .unit("GB")
                    .show(ui);
                if let Some(set) = set {
                    *value = set;
                    *sets += 1;
                }
            },
            (value, 0),
        );
    harness.run();
    harness
}

/// A number that must be more than its `above` refuses that value and any under it as out
/// of range, saying what it may be, and takes any over it within the rest of its range; −
/// never steps to it. Each case: the minimum, maximum and `above`, the value typed, whether
/// it is kept, and the range said when it is not.
#[test]
fn a_number_more_than_its_above_refuses_it() {
    let cases = [
        (None, None, 0.0, "0", false, "more than 0.0 GB"),
        (None, None, 0.0, "-1", false, "more than 0.0 GB"),
        (None, None, 0.0, "0.1", true, ""),
        (None, None, 0.0, "5", true, ""),
        (
            None,
            Some(10.0),
            0.0,
            "0",
            false,
            "more than 0.0, up to 10.0 GB",
        ),
        (None, Some(10.0), 0.0, "10", true, ""),
        (
            None,
            Some(10.0),
            0.0,
            "11",
            false,
            "more than 0.0, up to 10.0 GB",
        ),
        (
            Some(-5.0),
            Some(10.0),
            0.0,
            "0",
            false,
            "more than 0.0, up to 10.0 GB",
        ),
        (Some(-5.0), Some(10.0), 0.0, "0.1", true, ""),
        // A minimum over `above` is the end said, and the end held.
        (Some(1.0), None, 0.0, "0.5", false, "1.0 or more GB"),
        (Some(1.0), None, 0.0, "1", true, ""),
    ];
    let field = "Limit, in GB";
    for (min, max, bound, typed, kept, said) in cases {
        let case = format!("{min:?} {max:?} above {bound}: {typed}");
        let mut harness = above(min, max, bound, 3.0);
        harness
            .get_by_role_and_label(Role::TextInput, field)
            .click();
        harness.run();
        harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
        harness
            .get_by_role_and_label(Role::TextInput, field)
            .type_text(typed);
        harness.key_press(Key::Enter);
        harness.run();
        let (value, sets) = *harness.state();
        if kept {
            assert_eq!(
                (value, sets),
                (typed.parse().expect("a number"), 1),
                "{case}"
            );
            assert!(
                harness.query_by_label_contains("more than").is_none(),
                "{case}"
            );
        } else {
            assert_eq!((value, sets), (3.0, 0), "{case}");
            harness.get_by_role_and_label(Role::Label, said);
        }
    }
    // − stops a step before it: at 1, a step down would be 0.
    let harness = above(None, None, 0.0, 1.0);
    let less = harness.get_by_role_and_label(Role::Button, "Decrease Limit");
    assert!(less.accesskit_node().is_disabled());
    let harness = above(None, None, 0.0, 1.5);
    let less = harness.get_by_role_and_label(Role::Button, "Decrease Limit");
    assert!(!less.accesskit_node().is_disabled());
    // Shift's ten steps from 3 would reach it: one step is taken.
    let mut harness = above(None, None, 0.0, 3.0);
    let less = centre_of(&harness, "Decrease Limit");
    harness.hover_at(less);
    harness.event(Event::ModifiersChanged(Modifiers::SHIFT));
    harness.run();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: less,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::SHIFT,
        });
        harness.run();
    }
    assert_eq!(harness.state().0, 2.0);
    // A minimum over it is reached exactly: from 1, a step down lands on 0.5, not 0.
    let mut harness = above(Some(0.5), None, 0.0, 1.0);
    let less = centre_of(&harness, "Decrease Limit");
    harness.hover_at(less);
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: less,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.run();
    }
    assert_eq!(harness.state().0, 0.5);
}

fn centre_of(harness: &Harness<'_, (f64, usize)>, label: &str) -> Pos2 {
    harness
        .get_by_role_and_label(Role::Button, label)
        .rect()
        .center()
}

/// A box over a wide range, 0 to 100000 in steps of 1, at `value`.
fn wide(value: f64) -> Harness<'static, f64> {
    let mut harness = Harness::builder()
        .with_size(vec2(280.0, 70.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx| install(ctx, Theme::default()))
        .build_ui_state(
            |ui, value: &mut f64| {
                ui.set_width(260.0);
                let set = Number::new("wide", NAME, *value, 0.0..=100_000.0).show(ui);
                if let Some(set) = set {
                    *value = set;
                }
            },
            value,
        );
    harness.run();
    harness
}

/// A held button keeps stepping for as long as the primary button is down, past egui's
/// click time, and stops as it is let go.
#[test]
fn a_held_button_steps_until_it_is_let_go() {
    let mut harness = wide(1000.0);
    let more = centre(&harness, "Increase Memory budget");
    harness.hover_at(more);
    button(&mut harness, more, true, Modifiers::NONE);
    harness.run_steps(60);
    let at_one_second = *harness.state();
    harness.run_steps(60);
    let at_two_seconds = *harness.state();
    assert!(
        at_two_seconds > at_one_second + 10.0,
        "{at_one_second} then {at_two_seconds}"
    );
    button(&mut harness, more, false, Modifiers::NONE);
    let released = *harness.state();
    harness.run_steps(30);
    assert_eq!(*harness.state(), released);
}

/// The buttons take the primary button alone: a secondary or middle press, held or not,
/// changes nothing. Space on a focused button steps once.
#[test]
fn only_the_primary_button_or_a_key_steps_it() {
    for other in [PointerButton::Secondary, PointerButton::Middle] {
        let mut harness = number(1024.0);
        let more = centre(&harness, "Increase Memory budget");
        harness.hover_at(more);
        harness.run();
        harness.event(Event::PointerButton {
            pos: more,
            button: other,
            pressed: true,
            modifiers: Modifiers::NONE,
        });
        harness.run_steps(60);
        harness.event(Event::PointerButton {
            pos: more,
            button: other,
            pressed: false,
            modifiers: Modifiers::NONE,
        });
        harness.run();
        assert_eq!(*harness.state(), 1024.0, "{other:?}");
    }
    let mut harness = number(1024.0);
    harness
        .get_by_role_and_label(Role::Button, "Increase Memory budget")
        .focus();
    harness.run();
    harness.key_press(Key::Space);
    harness.run_steps(60);
    assert_eq!(*harness.state(), 1280.0);
}

/// A release ends a hold, though the button is down again by the pass's end: pressed
/// again elsewhere in the same pass, the box steps no more; pressed again on the same
/// button, a new hold starts, with its one step.
#[test]
fn a_release_ends_a_hold_whatever_comes_in_its_pass() {
    for again_on_it in [false, true] {
        let mut harness = wide(1000.0);
        let more = centre(&harness, "Increase Memory budget");
        harness.hover_at(more);
        button(&mut harness, more, true, Modifiers::NONE);
        harness.run_steps(40);
        let held = *harness.state();
        assert!(held > 1001.0, "it repeats: {held}");
        let again = if again_on_it {
            more
        } else {
            more + vec2(0.0, 40.0)
        };
        let events = &mut harness.input_mut().events;
        for (pos, pressed) in [(more, false), (again, true)] {
            events.push(Event::PointerMoved(pos));
            events.push(Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            });
        }
        harness.step();
        let after = *harness.state();
        if again_on_it {
            assert_eq!(after, held + 1.0, "a new hold's first step");
        } else {
            assert_eq!(after, held, "nothing in the release's pass");
        }
        // Before a new hold would repeat: nothing more.
        harness.run_steps(15);
        assert_eq!(*harness.state(), after, "{again_on_it}");
    }
}

/// Shift is read off the press that steps, a click's or a key's: let go before the pass
/// ends, the press still steps ten.
#[test]
fn shift_is_the_press_s_own() {
    use egui::Event::{Key as KeyEvent, ModifiersChanged, PointerButton as Press};
    for by_key in [false, true] {
        let mut harness = wide(1000.0);
        let more = centre(&harness, "Increase Memory budget");
        if by_key {
            harness
                .get_by_role_and_label(Role::Button, "Increase Memory budget")
                .focus();
        } else {
            harness.hover_at(more);
        }
        harness.run();
        let input = harness.input_mut();
        for pressed in [true, false] {
            input.events.push(if by_key {
                KeyEvent {
                    key: Key::Space,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: Modifiers::SHIFT,
                }
            } else {
                Press {
                    pos: more,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::SHIFT,
                }
            });
        }
        input.events.push(ModifiersChanged(Modifiers::NONE));
        harness.step();
        harness.run();
        assert_eq!(*harness.state(), 1010.0, "by key: {by_key}");
    }
}

/// A number typed and left by a press on + is kept, the press stepping nothing: the field
/// gives up the focus as the press comes, committing in that pass, and the release adds
/// no step either.
#[test]
fn a_press_on_a_button_while_typing_keeps_what_is_typed() {
    let mut harness = number(1024.0);
    harness
        .get_by_role_and_label(Role::TextInput, FIELD)
        .click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness
        .get_by_role_and_label(Role::TextInput, FIELD)
        .type_text("1536");
    harness.run();
    assert_eq!(*harness.state(), 1024.0, "nothing set while typing");
    let more = centre(&harness, "Increase Memory budget");
    harness.hover_at(more);
    harness.run();
    button(&mut harness, more, true, Modifiers::NONE);
    assert_eq!(*harness.state(), 1536.0, "after the press");
    button(&mut harness, more, false, Modifiers::NONE);
    assert_eq!(*harness.state(), 1536.0, "after the release");
}

fn pointer(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers,
    }
}

/// What one pass of input does to a button, read as it came. Each case, on a box at 1000
/// stepping by 1: the events of one pass, given + where it is, and the value after it and
/// once a hold could have repeated.
#[test]
fn a_button_takes_one_pass_of_input_as_it_came() {
    type Case = (&'static str, fn(Pos2) -> Vec<Event>, f64);
    let cases: [Case; 2] = [
        // Shift+Space activates the focused +, a later press elsewhere none of its own.
        (
            "Shift+Space, then a press elsewhere",
            |more| {
                vec![
                    Event::Key {
                        key: Key::Space,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::SHIFT,
                    },
                    pointer(more + vec2(0.0, 40.0), true, Modifiers::NONE),
                    pointer(more + vec2(0.0, 40.0), false, Modifiers::NONE),
                ]
            },
            1010.0,
        ),
        // A press on + with Shift, Shift let go before the pass ends: ten steps.
        (
            "a Shift press on +",
            |more| {
                vec![
                    pointer(more, true, Modifiers::SHIFT),
                    pointer(more, false, Modifiers::SHIFT),
                    Event::ModifiersChanged(Modifiers::NONE),
                ]
            },
            1010.0,
        ),
    ];
    for (said, events, after) in cases {
        let mut harness = wide(1000.0);
        let more = centre(&harness, "Increase Memory budget");
        harness
            .get_by_role_and_label(Role::Button, "Increase Memory budget")
            .focus();
        harness.run();
        harness.input_mut().events.extend(events(more));
        harness.step();
        harness.run_steps(40);
        assert_eq!(*harness.state(), after, "{said}");
    }
    // Held, then let go and pressed again elsewhere and moved onto + in one pass: the new
    // press holds nothing.
    let mut harness = wide(1000.0);
    let more = centre(&harness, "Increase Memory budget");
    harness.hover_at(more);
    button(&mut harness, more, true, Modifiers::NONE);
    let held = *harness.state();
    let elsewhere = more + vec2(0.0, 40.0);
    harness.input_mut().events.extend([
        pointer(more, false, Modifiers::NONE),
        Event::PointerMoved(elsewhere),
        pointer(elsewhere, true, Modifiers::NONE),
        Event::PointerMoved(more),
    ]);
    harness.step();
    harness.run_steps(40);
    assert_eq!(*harness.state(), held, "a press that began elsewhere");
}

/// A typed value refused, out of the range, takes a step of the same pass with it: the
/// value stays, and the range is said.
#[test]
fn a_refused_value_takes_a_step_of_its_pass_with_it() {
    let mut harness = number(1024.0);
    harness
        .get_by_role_and_label(Role::TextInput, FIELD)
        .click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness
        .get_by_role_and_label(Role::TextInput, FIELD)
        .type_text("999999");
    harness.run();
    let more = centre(&harness, "Increase Memory budget");
    harness.hover_at(more);
    harness.run();
    harness.input_mut().events.extend([
        pointer(more, true, Modifiers::NONE),
        pointer(more, false, Modifiers::NONE),
    ]);
    harness.step();
    harness.run();
    assert_eq!(*harness.state(), 1024.0);
    harness.get_by_role_and_label(Role::Label, "256 to 2048 MiB");
}
