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

//! Interaction tests: each component driven as a person or a screen reader would, found
//! through AccessKit by its role and name, and checked for what it reported
//! (`docs/gui.md`, Testing).

use apertrust_ui::components::{
    ActionRow, Choice, ChoiceAction, ChoiceStyle, CollapsibleGroup, IconButton, Mark, Quad,
    Segmented, SliderAction, SliderRow, Swatch, SwatchFill, Swatches, ToggleRow,
};
use apertrust_ui::{Builtin, Icon, Locale, Theme, Translator};
use egui::accesskit::Role;
use egui::os::OperatingSystem;
use egui::{Color32, ImeEvent, Key, Modifiers, pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

/// The side panel's content at its narrowest: 240 points, less its margins.
fn narrowest_content(t: &Theme) -> f32 {
    240.0 - 2.0 * t.spacing.l
}

/// A slider and what it last reported, applied as the app would.
struct Exposure {
    value: f64,
    mark: Mark,
    last: Option<SliderAction>,
}

fn exposure(locale: Locale) -> Harness<'static, Exposure> {
    slider(locale, None)
}

/// Exposure from −4 to 4, its track over `soft` when given.
fn slider(
    locale: Locale,
    soft: Option<std::ops::RangeInclusive<f64>>,
) -> Harness<'static, Exposure> {
    Harness::builder()
        .with_size(egui::vec2(560.0, 60.0))
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::default());
            apertrust_ui::set_locale(ctx, locale);
        })
        .build_ui_state(
            move |ui, state: &mut Exposure| {
                let mut row = SliderRow::new("Exposure", state.value, -4.0..=4.0)
                    .step(0.01)
                    .decimals(2)
                    .mark(state.mark);
                if let Some(soft) = soft.clone() {
                    row = row.soft_range(soft);
                }
                let action = row.show(ui);
                if let Some(action) = action {
                    match action {
                        SliderAction::Set { value, .. } => state.value = value,
                        SliderAction::Reset => state.value = 0.0,
                        SliderAction::ToggleKeep => {
                            state.mark = if state.mark == Mark::Kept {
                                Mark::None
                            } else {
                                Mark::Kept
                            };
                        }
                        SliderAction::Quad => {}
                    }
                    state.last = Some(action);
                }
            },
            Exposure {
                value: 0.0,
                mark: Mark::None,
                last: None,
            },
        )
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn dragging_the_track_sets_the_value_and_marks_the_release_done() {
    let mut harness = exposure(Locale::EnGb);
    harness.run();
    let track = harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .rect();
    let three_quarters = pos2(track.left() + track.width() * 0.75, track.center().y);
    harness.drag_at(three_quarters);
    // One frame: the press reports its value in the frame it lands, not the next.
    harness.step();
    assert!(
        near(harness.state().value, 2.0),
        "{}",
        harness.state().value
    );
    assert_eq!(
        harness.state().last,
        Some(SliderAction::Set {
            value: 2.0,
            done: false
        })
    );
    harness.drop_at(three_quarters);
    harness.run();
    assert!(matches!(
        harness.state().last,
        Some(SliderAction::Set { done: true, .. })
    ));
}

#[test]
fn keys_step_the_value_and_reach_the_ends() {
    let mut harness = exposure(Locale::EnGb);
    harness.run();
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert!(
        near(harness.state().value, 0.01),
        "{}",
        harness.state().value
    );
    harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowRight);
    harness.run();
    assert!(
        near(harness.state().value, 0.11),
        "{}",
        harness.state().value
    );
    harness.key_press(Key::End);
    harness.run();
    assert!(near(harness.state().value, 4.0));
    harness.key_press(Key::Home);
    harness.run();
    assert!(near(harness.state().value, -4.0));
    harness.key_press(Key::Delete);
    harness.run();
    assert_eq!(harness.state().last, Some(SliderAction::Reset));
    assert!(near(harness.state().value, 0.0));
}

/// Drags the track to `at`, a fraction of its width, and lets go.
fn drag_to(harness: &mut Harness<'_, Exposure>, at: f32) {
    let track = harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .rect();
    let at = pos2(track.left() + track.width() * at, track.center().y);
    harness.drag_at(at);
    harness.run();
    harness.drop_at(at);
    harness.run();
}

/// A track widened past its soft range starts again from it once the soft range itself
/// moves, as a range that follows other controls does (decision 30), as darktable's
/// `dt_bauhaus_slider_set_soft_min` resets its visible range.
#[test]
fn a_moved_soft_range_starts_the_track_s_span_again() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(560.0, 60.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, state: &mut (f64, (f64, f64))| {
                let (low, high) = state.1;
                let action = SliderRow::new("Exposure", state.0, -4.0..=4.0)
                    .step(0.01)
                    .decimals(2)
                    .soft_range(low..=high)
                    .show(ui);
                if let Some(SliderAction::Set { value, .. }) = action {
                    state.0 = value;
                }
            },
            (3.0, (-1.0, 1.0)),
        );
    let to_end = |harness: &mut Harness<'_, (f64, (f64, f64))>| {
        let track = harness
            .get_by_role_and_label(Role::Slider, "Exposure")
            .rect();
        let at = pos2(track.right() - 0.5, track.center().y);
        harness.drag_at(at);
        harness.run();
        harness.drop_at(at);
        harness.run();
        harness.state().0
    };
    harness.run();
    // A value of 3 widens the track to span −1 to 3.
    assert!(near(to_end(&mut harness), 3.0));
    // The soft range moves to −2 to 2 with a value inside it: the span starts again there.
    harness.state_mut().1 = (-2.0, 2.0);
    harness.state_mut().0 = 1.5;
    harness.run();
    assert!(near(to_end(&mut harness), 2.0), "{}", harness.state().0);
}

#[test]
fn a_soft_range_is_the_track_s_span_until_a_value_beyond_it_widens_it() {
    // Exposure holds −4 to 4, its track over −1 to 1, as darktable's soft range.
    let mut harness = slider(Locale::EnGb, Some(-1.0..=1.0));
    harness.run();
    let value = |harness: &Harness<'_, Exposure>| harness.state().value;
    drag_to(&mut harness, 1.0);
    assert!(near(value(&harness), 1.0), "{}", value(&harness));
    drag_to(&mut harness, 0.75);
    assert!(near(value(&harness), 0.5), "{}", value(&harness));
    // Keys stay inside the span, Home and End at its ends.
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .focus();
    harness.run();
    harness.key_press(Key::End);
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert!(near(value(&harness), 1.0), "{}", value(&harness));
    harness.key_press(Key::Home);
    harness.run();
    assert!(near(value(&harness), -1.0), "{}", value(&harness));
    // A typed value reaches the range, and the track widens to show it, from −1 to 3.
    type_value(&mut harness, "Exposure", "3", Key::Enter);
    assert!(near(value(&harness), 3.0), "{}", value(&harness));
    drag_to(&mut harness, 0.75);
    assert!(near(value(&harness), 2.0), "{}", value(&harness));
    // The span stays until the row is reset, as darktable's does.
    drag_to(&mut harness, 1.0);
    assert!(near(value(&harness), 3.0), "{}", value(&harness));
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .focus();
    harness.run();
    harness.key_press(Key::Delete);
    harness.run();
    assert_eq!(harness.state().last, Some(SliderAction::Reset));
    drag_to(&mut harness, 1.0);
    assert!(near(value(&harness), 1.0), "{}", value(&harness));
    // A screen reader is told the range, and may set a value anywhere in it.
    let node = harness.get_by_role_and_label(Role::Slider, "Exposure");
    let node = node.accesskit_node();
    assert_eq!(
        (node.min_numeric_value(), node.max_numeric_value()),
        (Some(-4.0), Some(4.0))
    );
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .set_value_accesskit(-3.456);
    harness.run();
    assert!(near(value(&harness), -3.46), "{}", value(&harness));
    drag_to(&mut harness, 0.0);
    assert!(near(value(&harness), -3.46), "{}", value(&harness));
}

/// Presses `key` with `modifiers` on the focused track and returns the value it left.
fn press(harness: &mut Harness<'_, Exposure>, modifiers: Modifiers, key: Key) -> f64 {
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .focus();
    harness.run();
    harness.key_press_modifiers(modifiers, key);
    harness.run();
    harness.state().value
}

#[test]
fn a_key_s_step_grows_with_the_track_s_span_as_darktable_s_zoom_step() {
    // −4 to 4, the track over −1 to 1, stepping by 0.01 there: darktable's step for a top
    // of 1.
    let mut harness = slider(Locale::EnGb, Some(-1.0..=1.0));
    harness.run();
    let value = press(&mut harness, Modifiers::NONE, Key::ArrowRight);
    assert!(near(value, 0.01), "{value}");
    // Typed to 4, the track spans −1 to 4, a top of 4: darktable steps by 0.05 there.
    type_value(&mut harness, "Exposure", "4", Key::Enter);
    let value = press(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    assert!(near(value, 3.95), "{value}");
    // Reset, the track spans its rest again, and the step with it.
    press(&mut harness, Modifiers::NONE, Key::Delete);
    let value = press(&mut harness, Modifiers::NONE, Key::ArrowRight);
    assert!(near(value, 0.01), "{value}");
}

#[test]
fn a_key_s_move_is_never_less_than_the_last_decimal_shown() {
    // A tenth of 0.01 is 0.001, under the two decimals shown: darktable's `min_visible`
    // makes it 0.01, so Alt still moves, as on Global offset chroma.
    let mut harness = exposure(Locale::EnGb);
    harness.run();
    let value = press(&mut harness, Modifiers::ALT, Key::ArrowRight);
    assert!(near(value, 0.01), "{value}");
    let value = press(&mut harness, Modifiers::ALT, Key::ArrowLeft);
    assert!(near(value, 0.0), "{value}");
    // A screen reader's increment too.
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .increment_accesskit();
    harness.run();
    assert!(
        near(harness.state().value, 0.01),
        "{}",
        harness.state().value
    );
}

#[test]
fn a_value_set_from_outside_beyond_the_soft_range_widens_the_track_and_it_stays() {
    // A script, a preset or an edit sets 3, as the app's mirror then shows it.
    let mut harness = slider(Locale::EnGb, Some(-1.0..=1.0));
    harness.run();
    harness.state_mut().value = 3.0;
    harness.run();
    drag_to(&mut harness, 0.75);
    assert!(
        near(harness.state().value, 2.0),
        "{}",
        harness.state().value
    );
    // Back inside the soft range, set from outside again: the span stays −1 to 3.
    harness.state_mut().value = 0.5;
    harness.run();
    drag_to(&mut harness, 1.0);
    assert!(
        near(harness.state().value, 3.0),
        "{}",
        harness.state().value
    );
}

#[test]
fn a_reset_from_outside_keeps_the_widened_span_as_darktable_s_gui_update_does() {
    // Widened to 3, then the module's or the header's reset sets the default from
    // outside, not the row's own reset: darktable's `gui_update` only widens, so the
    // track keeps −1 to 3.
    let mut harness = slider(Locale::EnGb, Some(-1.0..=1.0));
    harness.run();
    type_value(&mut harness, "Exposure", "3", Key::Enter);
    harness.state_mut().value = 0.0;
    harness.run();
    drag_to(&mut harness, 1.0);
    assert!(
        near(harness.state().value, 3.0),
        "{}",
        harness.state().value
    );
    // The row's own reset takes it back to the soft range.
    press(&mut harness, Modifiers::NONE, Key::Delete);
    drag_to(&mut harness, 1.0);
    assert!(
        near(harness.state().value, 1.0),
        "{}",
        harness.state().value
    );
}

#[test]
fn a_dragged_or_typed_value_is_rounded_to_the_decimals_shown_not_to_the_step() {
    // darktable rounds a dragged or typed value to its digits; the step is a key's.
    let mut harness = Harness::builder()
        .with_size(egui::vec2(560.0, 60.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, state: &mut Exposure| {
                let action = SliderRow::new("Exposure", state.value, -4.0..=4.0)
                    .step(0.5)
                    .decimals(2)
                    .show(ui);
                if let Some(SliderAction::Set { value, .. }) = action {
                    state.value = value;
                }
            },
            Exposure {
                value: 0.0,
                mark: Mark::None,
                last: None,
            },
        );
    harness.run();
    drag_to(&mut harness, 0.6);
    let value = harness.state().value;
    assert!(near(value, (value * 100.0).round() / 100.0), "{value}");
    assert!((value - 0.8).abs() < 0.02, "{value}");
    type_value(&mut harness, "Exposure", "1.234", Key::Enter);
    assert!(
        near(harness.state().value, 1.23),
        "{}",
        harness.state().value
    );
    // A key moves by the step.
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert!(
        near(harness.state().value, 1.73),
        "{}",
        harness.state().value
    );
}

#[test]
fn a_screen_reader_steps_and_sets_the_value() {
    let mut harness = exposure(Locale::EnGb);
    harness.run();
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .increment_accesskit();
    harness.run();
    assert!(
        near(harness.state().value, 0.01),
        "{}",
        harness.state().value
    );
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .set_value_accesskit(1.2345);
    harness.run();
    assert!(
        near(harness.state().value, 1.23),
        "{}",
        harness.state().value
    );
}

/// One slider row at rest, `width` wide: what it draws as its value, the text furthest
/// towards the row's end, and what screen readers hear, the track's value, numeric value
/// and keyboard shortcut.
struct AtRest {
    drawn: String,
    heard: Option<String>,
    numeric: Option<f64>,
    shortcut: Option<String>,
}

fn at_rest(
    locale: Locale,
    value: f64,
    range: std::ops::RangeInclusive<f64>,
    decimals: usize,
    unit: Option<&'static str>,
) -> AtRest {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(320.0, 60.0))
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::default());
            apertrust_ui::set_locale(ctx, locale);
        })
        .build_ui(move |ui| {
            let mut row = SliderRow::new("Exposure", value, range.clone()).decimals(decimals);
            if let Some(unit) = unit {
                row = row.unit(unit);
            }
            row.show(ui);
        });
    harness.run();
    let drawn = texts(&harness)
        .into_iter()
        .max_by(|a, b| a.1.right().total_cmp(&b.1.right()))
        .map(|(text, _)| text)
        .expect("the value is drawn");
    let node = harness.get_by_role_and_label(Role::Slider, "Exposure");
    let node = node.accesskit_node();
    AtRest {
        drawn,
        heard: node.value(),
        numeric: node.numeric_value(),
        shortcut: node.data().keyboard_shortcut().map(str::to_owned),
    }
}

/// Every text drawn, with where.
fn texts<State>(harness: &Harness<'_, State>) -> Vec<(String, egui::Rect)> {
    harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) => Some((
                shape.galley.text().to_owned(),
                shape.galley.rect.translate(shape.pos.to_vec2()),
            )),
            _ => None,
        })
        .collect()
}

/// Whether the text drawn as `text` is cut short, ending in …: a galley's text is the
/// whole of it either way.
fn elided<State>(harness: &Harness<'_, State>, text: &str) -> bool {
    harness
        .output()
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.text() == text => Some(shape.galley.elided),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{text} is drawn"))
}

/// darktable's `dt_bauhaus_slider_get_text`: the row's decimals in the language's format,
/// then the unit as each language places it, signed whenever the range spans zero, zero
/// as "+0.00" (decision 42). Ukrainian writes darktable's uk.po's "25%" and "px".
#[test]
fn a_slider_writes_its_value_at_rest_as_darktable_does_in_every_language() {
    type Case = (
        f64,
        std::ops::RangeInclusive<f64>,
        usize,
        Option<&'static str>,
        [&'static str; 3],
    );
    let cases: [Case; 12] = [
        (
            6.5,
            1.0..=10.0,
            2,
            Some("unit-ev-value"),
            ["6.50 EV", "6,50 EV", "6,50 EV"],
        ),
        (
            0.0,
            -4.0..=4.0,
            2,
            Some("unit-ev-value"),
            ["+0.00 EV", "+0,00 EV", "+0,00 EV"],
        ),
        (
            0.42,
            -4.0..=4.0,
            2,
            Some("unit-ev-value"),
            ["+0.42 EV", "+0,42 EV", "+0,42 EV"],
        ),
        // A range that does not span zero signs nothing.
        (
            -10.0,
            -20.0..=-1.0,
            2,
            Some("unit-ev-value"),
            ["−10.00 EV", "−10,00 EV", "−10,00 EV"],
        ),
        (
            25.0,
            0.0..=100.0,
            2,
            Some("unit-percent-value"),
            ["25.00%", "25,00%", "25,00%"],
        ),
        (
            -40.0,
            -100.0..=100.0,
            0,
            Some("unit-percent-value"),
            ["−40%", "−40%", "−40%"],
        ),
        (
            1600.0,
            20.0..=6400.0,
            0,
            Some("unit-iso-value"),
            ["1600 ISO", "1600 ISO", "1600 ISO"],
        ),
        (
            45.0,
            0.0..=360.0,
            1,
            Some("unit-degrees-value"),
            ["45.0°", "45,0°", "45,0°"],
        ),
        (
            5000.0,
            1667.0..=25000.0,
            0,
            Some("unit-kelvin-value"),
            ["5000 K", "5000 K", "5000 K"],
        ),
        (
            12.0,
            0.0..=100.0,
            0,
            Some("unit-pixels-value"),
            ["12 px", "12 px", "12 px"],
        ),
        // A stored value that rounds to zero shows "+0.00", where darktable prints
        // "−0.00" (deviation 13); with no unit, the number alone.
        (-0.0004, -1.0..=1.0, 2, None, ["+0.00", "+0,00", "+0,00"]),
        (0.0, 0.0..=1.0, 2, None, ["0.00", "0,00", "0,00"]),
    ];
    for (value, range, decimals, unit, expected) in cases {
        for (locale, expected) in [Locale::EnGb, Locale::Uk, Locale::Pl]
            .into_iter()
            .zip(expected)
        {
            let row = at_rest(locale, value, range.clone(), decimals, unit);
            assert_eq!(row.drawn, expected, "{locale:?}");
            // Screen readers hear it as it is drawn, with the number, and Enter, which
            // opens its field.
            assert_eq!(row.heard.as_deref(), Some(expected), "{locale:?}");
            assert_eq!(row.numeric, Some(value));
            assert_eq!(row.shortcut.as_deref(), Some("Enter"));
        }
        // The pseudo-locale writes the unit's message lengthened, the number in it whole.
        let row = at_rest(Locale::Pseudo, value, range, decimals, unit);
        let number = expected[0]
            .split([' ', '%', '°'])
            .next()
            .unwrap_or_default();
        assert!(
            row.drawn.contains(number) && row.heard.as_deref() == Some(row.drawn.as_str()),
            "{row:?}",
            row = (&row.drawn, &row.heard)
        );
        if unit.is_some() {
            assert!(
                row.drawn.starts_with('[') && row.drawn.ends_with(']'),
                "{}",
                row.drawn
            );
        }
    }
}

#[test]
fn a_toggle_row_reports_its_full_name_and_ignores_input_while_disabled() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(560.0, 60.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, state: &mut (bool, bool, Vec<bool>)| {
                if let Some(on) = ToggleRow::new("Use camera bias", state.0)
                    .name("Use the camera's exposure bias")
                    .enabled(state.1)
                    .show(ui)
                {
                    state.0 = on;
                    state.2.push(on);
                }
            },
            (true, true, Vec::new()),
        );
    harness.run();
    let name = "Use the camera's exposure bias";
    harness.get_by_role_and_label(Role::CheckBox, name).click();
    harness.run();
    assert_eq!(harness.state().2, [false]);
    harness.state_mut().1 = false;
    harness.run();
    let toggle = harness.get_by_role_and_label(Role::CheckBox, name);
    assert!(toggle.accesskit_node().is_disabled());
    toggle.click();
    harness.run();
    assert_eq!(harness.state().2, [false]);
}

#[test]
fn an_action_row_reports_its_full_name_and_ignores_input_while_disabled() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(320.0, 60.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, state: &mut (bool, usize)| {
                if ActionRow::new("Set from above")
                    .name("Set the primaries after tone mapping from above")
                    .promise("Set parameters to completely reverse primaries modifications.")
                    .enabled(state.0)
                    .show(ui)
                {
                    state.1 += 1;
                }
            },
            (true, 0),
        );
    harness.run();
    let name = "Set the primaries after tone mapping from above";
    harness.get_by_role_and_label(Role::Button, name).click();
    harness.run();
    assert_eq!(harness.state().1, 1);
    harness.state_mut().0 = false;
    harness.run();
    let action = harness.get_by_role_and_label(Role::Button, name);
    assert!(action.accesskit_node().is_disabled());
    action.click();
    harness.run();
    assert_eq!(harness.state().1, 1);
}

#[test]
fn clicking_a_toggle_row_s_label_toggles_it() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(320.0, 60.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, on: &mut bool| {
                if let Some(now) = ToggleRow::new("Use camera bias", *on)
                    .name("Use the camera's exposure bias")
                    .show(ui)
                {
                    *on = now;
                }
            },
            true,
        );
    harness.run();
    let row = harness
        .get_by_role_and_label(Role::CheckBox, "Use the camera's exposure bias")
        .rect();
    // The checkbox at the start, its label after it: click the label's end.
    assert!(row.width() > 60.0, "{row:?}");
    click_at(&mut harness, pos2(row.right() - 4.0, row.center().y));
    assert!(!*harness.state());
}

/// Types a value as the keyboard does: Enter on the focused track opens its field, the
/// number selected, `text` replaces it, then `end`.
fn type_value(harness: &mut Harness<'_, Exposure>, slider: &str, text: &str, end: Key) {
    harness.get_by_role_and_label(Role::Slider, slider).focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    harness.event(egui::Event::Text(text.to_owned()));
    harness.run();
    harness.key_press(end);
    harness.run();
}

#[test]
fn a_typed_value_commits_on_enter_and_snaps_to_the_step() {
    let mut harness = exposure(Locale::EnGb);
    harness.run();
    type_value(&mut harness, "Exposure", "1.505 EV", Key::Enter);
    assert!(
        near(harness.state().value, 1.51),
        "{}",
        harness.state().value
    );
    // Out of range: clamped.
    type_value(&mut harness, "Exposure", "−9", Key::Enter);
    assert!(
        near(harness.state().value, -4.0),
        "{}",
        harness.state().value
    );
}

#[test]
fn a_typed_value_takes_the_language_s_decimal_comma() {
    let mut harness = exposure(Locale::Uk);
    harness.run();
    type_value(&mut harness, "Exposure", "1,5", Key::Enter);
    assert!(
        near(harness.state().value, 1.5),
        "{}",
        harness.state().value
    );
}

#[test]
fn escape_cancels_a_typed_value() {
    let mut harness = exposure(Locale::EnGb);
    harness.run();
    type_value(&mut harness, "Exposure", "3", Key::Escape);
    assert!(
        near(harness.state().value, 0.0),
        "{}",
        harness.state().value
    );
    assert_eq!(harness.state().last, None);
    // The field is closed again, the value text once more.
    assert!(harness.query_by_role(Role::TextInput).is_none());
}

/// Opens Exposure's field from the keyboard, its number selected.
fn open_by_keyboard(harness: &mut Harness<'_, Exposure>) {
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
}

/// Types `text` into the open field as a key does, and returns what the field then holds.
fn typed(harness: &mut Harness<'_, Exposure>, text: &str) -> String {
    harness.event(egui::Event::Text(text.to_owned()));
    harness.run();
    open_field(harness).1
}

/// Pastes `text` into the open field, and returns what the field then holds.
fn pasted(harness: &mut Harness<'_, Exposure>, text: &str) -> String {
    harness.event(egui::Event::Paste(text.to_owned()));
    harness.run();
    open_field(harness).1
}

/// A slider's field takes a number only, for now (decision 42): digits, a minus and the
/// decimal separator. Any other character is ignored as it is typed, the number staying
/// selected, and a paste holding one is refused whole.
#[test]
fn a_slider_s_field_takes_digits_a_minus_and_the_separator_typed_or_pasted() {
    const REFUSED: [&str; 13] = [
        "a", "E", "e", "x", "+", "*", "/", "%", "^", "(", " ", "\u{a0}", "٣",
    ];
    let mut harness = exposure(Locale::EnGb);
    harness.state_mut().value = 0.42;
    harness.run();
    open_by_keyboard(&mut harness);
    for refused in REFUSED {
        assert_eq!(typed(&mut harness, refused), "0.42", "typed {refused:?}");
        assert_eq!(pasted(&mut harness, refused), "0.42", "pasted {refused:?}");
    }
    // Still selected: what is typed replaces it, a character at a time.
    for (key, holds) in [("−", "−"), ("1", "−1"), (".", "−1."), ("5", "−1.5")] {
        assert_eq!(typed(&mut harness, key), holds, "typed {key:?}");
    }
    harness.key_press(Key::Enter);
    harness.run();
    assert!(
        near(harness.state().value, -1.5),
        "{}",
        harness.state().value
    );
    // The hyphen-minus too.
    open_by_keyboard(&mut harness);
    assert_eq!(typed(&mut harness, "-"), "-");
    assert_eq!(typed(&mut harness, "2"), "-2");
}

/// A paste is a number taken whole, less the spaces round it, or refused whole, the number
/// still selected: filtered, "1e-5" would be 15 and, in English, "−0,25" −25.
#[test]
fn a_paste_is_a_number_taken_whole_or_refused_whole() {
    let mut harness = exposure(Locale::EnGb);
    harness.state_mut().value = 0.42;
    harness.run();
    open_by_keyboard(&mut harness);
    for refused in ["EV −2.5x", "1e-5", "−0,25", "−1.2.3", "1-2", "2.5 EV", "+2"] {
        assert_eq!(pasted(&mut harness, refused), "0.42", "pasted {refused:?}");
    }
    assert_eq!(typed(&mut harness, "3"), "3");
    harness.key_press(Key::Escape);
    harness.run();
    open_by_keyboard(&mut harness);
    assert_eq!(pasted(&mut harness, "  −2.5\n"), "−2.5");
    harness.key_press(Key::Enter);
    harness.run();
    assert!(
        near(harness.state().value, -2.5),
        "{}",
        harness.state().value
    );
}

/// The separator is the point or the language's own: in English a comma is none, and is
/// ignored as it is typed, in Ukrainian and Polish either is, but only one, typed or
/// pasted.
#[test]
fn a_slider_s_field_takes_the_point_or_the_language_s_separator() {
    for (locale, keys, holds, value) in [
        (Locale::EnGb, &["1", ".", "5"][..], "1.5", Some(1.5)),
        (Locale::EnGb, &["1", ",", "5"][..], "15", None),
        (Locale::Uk, &["1", ",", "5"][..], "1,5", Some(1.5)),
        (Locale::Uk, &["1", ".", "5"][..], "1.5", Some(1.5)),
        (Locale::Uk, &["1", ",", ".", "5"][..], "1,5", Some(1.5)),
        (Locale::Pl, &["1", ",", "5"][..], "1,5", Some(1.5)),
        (Locale::Pl, &["1", ".", ",", "5"][..], "1.5", Some(1.5)),
    ] {
        let mut harness = exposure(locale);
        harness.run();
        open_by_keyboard(&mut harness);
        for key in keys {
            typed(&mut harness, key);
        }
        assert_eq!(open_field(&harness).1, holds, "{locale:?} {keys:?}");
        if let Some(value) = value {
            harness.key_press(Key::Enter);
            harness.run();
            assert!(
                near(harness.state().value, value),
                "{locale:?} {keys:?}: {}",
                harness.state().value
            );
        }
    }
    for (locale, clipboard, holds) in [
        (Locale::EnGb, "−0.25", "−0.25"),
        (Locale::EnGb, "−0,25", "0.00"),
        (Locale::EnGb, "−0.2.5", "0.00"),
        (Locale::Uk, "−0,25", "−0,25"),
        (Locale::Uk, "−0.25", "−0.25"),
        (Locale::Uk, "−0,2.5", "0,00"),
        (Locale::Pl, "−0,25", "−0,25"),
        (Locale::Pl, "−0.2,5", "0,00"),
    ] {
        let mut harness = exposure(locale);
        harness.run();
        open_by_keyboard(&mut harness);
        assert_eq!(pasted(&mut harness, clipboard), holds, "{locale:?}");
    }
}

/// A second separator, and a minus anywhere but at the number's start, are refused, typed
/// or pasted, and nothing goes before a minus. Refused over a selection, a key or a paste
/// leaves the text as it was, the caret at the selection's start.
#[test]
fn a_second_separator_or_a_minus_past_the_start_is_refused() {
    let mut harness = exposure(Locale::EnGb);
    harness.run();
    open_by_keyboard(&mut harness);
    assert_eq!(typed(&mut harness, "1.5"), "1.5");
    assert_eq!(typed(&mut harness, "."), "1.5");
    assert_eq!(typed(&mut harness, "−"), "1.5");
    assert_eq!(pasted(&mut harness, "2.5"), "1.5");
    assert_eq!(pasted(&mut harness, "-5"), "1.5");
    assert_eq!(pasted(&mut harness, "25"), "1.525");
    harness.key_press(Key::Home);
    harness.run();
    assert_eq!(typed(&mut harness, "."), "1.525");
    // At the start a minus is taken, once.
    assert_eq!(typed(&mut harness, "-"), "-1.525");
    assert_eq!(typed(&mut harness, "-"), "-1.525");
    harness.key_press(Key::Home);
    harness.run();
    for refused in ["−", "3", "."] {
        assert_eq!(typed(&mut harness, refused), "-1.525", "typed {refused:?}");
        assert_eq!(
            pasted(&mut harness, refused),
            "-1.525",
            "pasted {refused:?}"
        );
    }
    // The last digit selected and a separator typed over it: the text stays, the caret at
    // the selection's start, where a digit then goes.
    let select_last = |harness: &mut Harness<'_, Exposure>| {
        harness.key_press(Key::End);
        harness.run();
        harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowLeft);
        harness.run();
    };
    select_last(&mut harness);
    assert_eq!(typed(&mut harness, "."), "-1.525");
    assert_eq!(typed(&mut harness, "9"), "-1.5295");
    // Pasted over the last digit: a minus or a separator refused whole, the caret again at
    // the selection's start, and a digit taken in its place.
    select_last(&mut harness);
    assert_eq!(pasted(&mut harness, "−"), "-1.5295");
    assert_eq!(typed(&mut harness, "8"), "-1.52985");
    select_last(&mut harness);
    assert_eq!(pasted(&mut harness, ".7"), "-1.52985");
    select_last(&mut harness);
    assert_eq!(pasted(&mut harness, "7"), "-1.52987");
    // Pasted over the whole number, as typing would.
    harness.key_press(Key::Escape);
    harness.run();
    open_by_keyboard(&mut harness);
    assert_eq!(pasted(&mut harness, "−1.234"), "−1.234");
    harness.key_press(Key::Enter);
    harness.run();
    assert!(
        near(harness.state().value, -1.23),
        "{}",
        harness.state().value
    );
}

/// A screen reader's set value and replaced selection are pastes: a number taken whole,
/// less the spaces round it, or nothing at all, the text as it was; an empty value clears
/// the field.
#[test]
fn a_screen_reader_s_text_is_a_number_taken_whole_or_refused_whole() {
    let mut harness = exposure(Locale::EnGb);
    harness.state_mut().value = 0.42;
    harness.run();
    open_by_keyboard(&mut harness);
    let set = |harness: &mut Harness<'_, Exposure>, text: &str| {
        harness
            .get_by_role(Role::TextInput)
            .set_text_value_accesskit(text);
        harness.run();
        open_field(harness).1
    };
    let replace = |harness: &mut Harness<'_, Exposure>, text: &str| {
        harness
            .get_by_role(Role::TextInput)
            .replace_selected_text_accesskit(text);
        harness.run();
        open_field(harness).1
    };
    for refused in ["abc", "1a2", "1e-5", "−0,25", "1-2"] {
        assert_eq!(set(&mut harness, refused), "0.42", "set {refused:?}");
        assert_eq!(
            replace(&mut harness, refused),
            "0.42",
            "replaced {refused:?}"
        );
    }
    assert_eq!(set(&mut harness, " 2.5 "), "2.5");
    assert_eq!(set(&mut harness, ""), "");
    // Over the selected number, a number replaces it, and nothing clears it.
    harness.key_press(Key::Escape);
    harness.run();
    open_by_keyboard(&mut harness);
    assert_eq!(replace(&mut harness, "7"), "7");
    harness.key_press(Key::Escape);
    harness.run();
    open_by_keyboard(&mut harness);
    assert_eq!(replace(&mut harness, ""), "");
}

/// What an input method composes is typed: a character no number has is ignored, while it
/// is composed and as it is committed, the number staying selected.
#[test]
fn a_composed_character_no_number_has_leaves_the_selection_as_it_was() {
    let mut harness = exposure(Locale::EnGb);
    harness.state_mut().value = 0.42;
    harness.run();
    let ime = |harness: &mut Harness<'_, Exposure>, event: ImeEvent| {
        harness.event(egui::Event::Ime(event));
        harness.run();
        open_field(harness).1
    };
    open_by_keyboard(&mut harness);
    assert_eq!(ime(&mut harness, ImeEvent::Commit("a".to_owned())), "0.42");
    assert_eq!(typed(&mut harness, "5"), "5");
    harness.key_press(Key::Escape);
    harness.run();
    open_by_keyboard(&mut harness);
    let preedit = ImeEvent::Preedit {
        text: "a".to_owned(),
        active_range_chars: Some(0..1),
    };
    assert_eq!(ime(&mut harness, preedit), "0.42");
    assert_eq!(ime(&mut harness, ImeEvent::Commit("a".to_owned())), "0.42");
    assert_eq!(typed(&mut harness, "5"), "5");
    // A digit composed is taken, in place of the selection.
    harness.key_press(Key::Escape);
    harness.run();
    open_by_keyboard(&mut harness);
    assert_eq!(ime(&mut harness, ImeEvent::Commit("7".to_owned())), "7");
}

/// "-0", or a small negative that snaps to zero, sets 0, never −0.
#[test]
fn a_typed_negative_zero_sets_zero() {
    for text in ["-0", "−0", "−0.004"] {
        let mut harness = exposure(Locale::EnGb);
        harness.state_mut().value = 0.42;
        harness.run();
        type_value(&mut harness, "Exposure", text, Key::Enter);
        let Some(SliderAction::Set { value, done: true }) = harness.state().last else {
            panic!("{text:?}: {:?}", harness.state().last);
        };
        assert!(
            value == 0.0 && value.is_sign_positive(),
            "{text:?}: {value}"
        );
    }
}

/// The field holds the number alone, so screen readers hear the slider's unit in its
/// name, each language placing it; a slider with no unit keeps "exact value".
#[test]
fn the_open_field_is_named_with_the_slider_s_unit() {
    for (locale, unit, name) in [
        (Locale::EnGb, Some("unit-ev-value"), "Exposure, in EV"),
        (Locale::EnGb, Some("unit-percent-value"), "Exposure, in %"),
        (Locale::Uk, Some("unit-ev-value"), "Exposure, в EV"),
        (Locale::Pl, Some("unit-ev-value"), "Exposure, w EV"),
        (Locale::EnGb, None, "Exposure: exact value"),
    ] {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(560.0, 60.0))
            .with_context_setup(move |ctx| {
                apertrust_ui::install(ctx, Theme::default());
                apertrust_ui::set_locale(ctx, locale);
            })
            .build_ui(move |ui| {
                let mut row = SliderRow::new("Exposure", 0.0, -4.0..=4.0).decimals(2);
                if let Some(unit) = unit {
                    row = row.unit(unit);
                }
                row.show(ui);
            });
        harness.run();
        harness
            .get_by_role_and_label(Role::Slider, "Exposure")
            .focus();
        harness.run();
        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(
            focused(&harness),
            (Role::TextInput, name.to_owned()),
            "{locale:?}"
        );
    }
}

/// A typed number is clamped to the control's range at both ends; one past the soft range
/// reaches it, and the track widens to show it.
#[test]
fn a_typed_number_is_clamped_to_the_range_and_widens_the_track_past_the_soft_range() {
    // Exposure holds −4 to 4, its track over −1 to 1.
    let mut harness = slider(Locale::EnGb, Some(-1.0..=1.0));
    harness.run();
    let value = |harness: &Harness<'_, Exposure>| harness.state().value;
    type_value(&mut harness, "Exposure", "2.5", Key::Enter);
    assert!(near(value(&harness), 2.5), "{}", value(&harness));
    // The track spans −1 to 2.5.
    drag_to(&mut harness, 0.0);
    assert!(near(value(&harness), -1.0), "{}", value(&harness));
    drag_to(&mut harness, 1.0);
    assert!(near(value(&harness), 2.5), "{}", value(&harness));
    type_value(&mut harness, "Exposure", "12", Key::Enter);
    assert!(near(value(&harness), 4.0), "{}", value(&harness));
    assert_eq!(resting_value(&harness).0, "+4.00");
    type_value(&mut harness, "Exposure", "−12", Key::Enter);
    assert!(near(value(&harness), -4.0), "{}", value(&harness));
    assert_eq!(resting_value(&harness).0, "−4.00");
}

/// Emptied, or holding a sign or a separator alone, the field holds no number: Enter, Tab
/// or a click elsewhere puts the value back and sets nothing, with no error, as Esc does.
#[test]
fn an_empty_field_or_a_sign_alone_puts_the_value_back() {
    type End = fn(&mut Harness<'static, Exposure>);
    let ends: [(&str, End); 4] = [
        ("Enter", |harness| harness.key_press(Key::Enter)),
        ("Tab", |harness| harness.key_press(Key::Tab)),
        ("a click elsewhere", |harness| {
            click_at(harness, pos2(4.0, 56.0));
        }),
        ("Esc", |harness| harness.key_press(Key::Escape)),
    ];
    for (locale, text, resting) in [
        (Locale::EnGb, "", "+0.42"),
        (Locale::EnGb, "-", "+0.42"),
        (Locale::EnGb, "−", "+0.42"),
        (Locale::EnGb, ".", "+0.42"),
        (Locale::EnGb, "−.", "+0.42"),
        (Locale::Uk, ",", "+0,42"),
        (Locale::Pl, "-,", "+0,42"),
    ] {
        for (name, end) in ends {
            let mut harness = exposure(locale);
            harness.state_mut().value = 0.42;
            harness.run();
            open_by_keyboard(&mut harness);
            // The number is selected: Backspace empties the field.
            harness.key_press(Key::Backspace);
            harness.run();
            if !text.is_empty() {
                typed(&mut harness, text);
            }
            assert_eq!(open_field(&harness).1, text, "{locale:?} {text:?}");
            end(&mut harness);
            harness.run();
            let case = format!("{locale:?} {text:?} then {name}");
            assert_eq!(harness.state().last, None, "{case}");
            assert!(near(harness.state().value, 0.42), "{case}");
            assert!(harness.query_by_role(Role::TextInput).is_none(), "{case}");
            assert_eq!(resting_value(&harness).0, resting, "{case}");
        }
    }
}

#[test]
fn the_marker_keeps_and_releases_the_slider() {
    let mut harness = exposure(Locale::EnGb);
    harness.run();
    harness.get_by_label("Keep Exposure").click();
    harness.run();
    assert_eq!(harness.state().mark, Mark::Kept);
    harness.get_by_label("Keep Exposure").click();
    harness.run();
    assert_eq!(harness.state().mark, Mark::None);
}

/// What has the keyboard focus: its role and its accessible name.
fn focused<State>(harness: &Harness<'_, State>) -> (Role, String) {
    let node = harness.get_by(|node| node.is_focused());
    let node = node.accesskit_node();
    (node.role(), node.label().unwrap_or_default())
}

/// The value at rest, Exposure's in the harness, as drawn: its text and where.
fn resting_value<State>(harness: &Harness<'_, State>) -> (String, egui::Rect) {
    texts(harness)
        .into_iter()
        .max_by(|a, b| a.1.right().total_cmp(&b.1.right()))
        .expect("the value is drawn")
}

/// The field, open: its rectangle and what it holds, as screen readers have it.
fn open_field<State>(harness: &Harness<'_, State>) -> (egui::Rect, String) {
    let node = harness.get_by_role(Role::TextInput);
    let rect = node.rect();
    (rect, node.accesskit_node().value().unwrap_or_default())
}

/// At rest the value is text, no field and no focus stop, until something opens it.
#[test]
fn a_slider_s_value_at_rest_is_text_with_no_field() {
    let mut harness = exposure(Locale::EnGb);
    harness.state_mut().value = 0.42;
    harness.run();
    assert!(harness.query_by_role(Role::TextInput).is_none());
    let (text, at) = resting_value(&harness);
    assert_eq!(text, "+0.42");
    // No node of its own, under the pointer too, its tooltip showing: the track tells
    // screen readers the value, and the row has no other.
    harness.hover_at(at.center());
    harness.run();
    let nodes: Vec<(Role, Option<String>)> = harness
        .query_all_by(|_| true)
        .map(|node| (node.accesskit_node().role(), node.accesskit_node().label()))
        .filter(|(role, _)| ![Role::GenericContainer, Role::Tooltip, Role::Label].contains(role))
        .collect();
    assert_eq!(
        nodes,
        [
            (Role::Button, Some("Keep Exposure".to_owned())),
            (Role::Slider, Some("Exposure".to_owned())),
        ]
    );
}

/// A click on the value opens the field in its place, the number selected, and moves
/// nothing: only the track's strip does.
#[test]
fn a_click_on_the_value_opens_its_field_and_moves_nothing() {
    let field = (Role::TextInput, "Exposure: exact value".to_owned());
    let mut harness = exposure(Locale::EnGb);
    harness.state_mut().value = 0.42;
    harness.run();
    let (_, text) = resting_value(&harness);
    click_at(&mut harness, text.center());
    harness.run();
    assert_eq!(harness.state().last, None);
    assert!(near(harness.state().value, 0.42));
    assert_eq!(focused(&harness), field);
    // The number alone, selected: what is typed replaces it.
    harness.event(egui::Event::Text("1.5".to_owned()));
    harness.run();
    assert_eq!(open_field(&harness).1, "1.5");
    harness.key_press(Key::Enter);
    harness.run();
    assert!(
        near(harness.state().value, 1.5),
        "{}",
        harness.state().value
    );
    assert!(harness.query_by_role(Role::TextInput).is_none());
    // Leaving it as it was sets nothing. It holds no `+`, which it does not take.
    harness.state_mut().last = None;
    let (_, text) = resting_value(&harness);
    click_at(&mut harness, text.center());
    harness.run();
    assert_eq!(open_field(&harness).1, "1.50");
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().last, None);
}

/// A digit, a minus sign or the decimal separator typed on the focused track opens the
/// field starting with it, the caret after it, so a slider takes "50" and never lets a
/// digit through to the photo; the language's separator included.
#[test]
fn a_digit_a_minus_or_the_separator_typed_on_the_track_opens_the_field_with_it() {
    for (locale, keys, typed, value) in [
        (Locale::EnGb, &["2"][..], "2", 2.0),
        (Locale::EnGb, &["−", "1"][..], "−1", -1.0),
        (Locale::EnGb, &["-", "3"][..], "-3", -3.0),
        (Locale::EnGb, &[".", "5"][..], ".5", 0.5),
        (Locale::Uk, &[",", "2", "5"][..], ",25", 0.25),
        (Locale::Pl, &["1", ",", "5"][..], "1,5", 1.5),
    ] {
        let mut harness = exposure(locale);
        harness.run();
        harness
            .get_by_role_and_label(Role::Slider, "Exposure")
            .focus();
        harness.run();
        for key in keys {
            harness.event(egui::Event::Text((*key).to_owned()));
            harness.run();
        }
        let field = translator_field(locale);
        assert_eq!(focused(&harness), (Role::TextInput, field), "{locale:?}");
        assert_eq!(open_field(&harness).1, typed, "{locale:?}");
        assert_eq!(harness.state().last, None, "{locale:?}");
        harness.key_press(Key::Enter);
        harness.run();
        assert!(
            near(harness.state().value, value),
            "{locale:?} {keys:?}: {}",
            harness.state().value
        );
    }
    // Any other character leaves the track alone.
    let mut harness = exposure(Locale::EnGb);
    harness.run();
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .focus();
    harness.run();
    harness.event(egui::Event::Text("h".to_owned()));
    harness.run();
    assert!(harness.query_by_role(Role::TextInput).is_none());
}

/// The field's name in `locale`, for Exposure.
fn translator_field(locale: Locale) -> String {
    let mut args = apertrust_ui::i18n::Args::new();
    args.set("control", "Exposure");
    Translator::new(locale).text_with("field-value", Some(&args))
}

#[test]
fn enter_on_the_track_opens_the_value_field_ready_to_type() {
    let mut harness = exposure(Locale::EnGb);
    harness.run();
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    let field = (Role::TextInput, "Exposure: exact value".to_owned());
    assert_eq!(focused(&harness), field);
    // The number selected: typed without selecting it, the value replaces it and commits
    // on the next Enter.
    harness.event(egui::Event::Text("1.5".to_owned()));
    harness.run();
    assert_eq!(focused(&harness), field);
    harness.key_press(Key::Enter);
    harness.run();
    assert!(
        near(harness.state().value, 1.5),
        "{}",
        harness.state().value
    );
}

/// Two slider rows as a section stacks them, with no spacing between them, at the side
/// panel's narrowest content, the first with a picker.
fn two_rows() -> Harness<'static> {
    let content = narrowest_content(&Theme::default());
    Harness::builder()
        .with_size(egui::vec2(360.0, 120.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui(move |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.allocate_ui(egui::vec2(content, 100.0), |ui| {
                SliderRow::new("Temperature", 5000.0, 2000.0..=10000.0)
                    .default_value(5000.0)
                    .unit("unit-kelvin-value")
                    .quad(Quad {
                        icon: Icon::Pick,
                        name: "Pick a neutral",
                        active: false,
                    })
                    .show(ui);
                SliderRow::new("Exposure", 0.5, -4.0..=4.0)
                    .decimals(2)
                    .show(ui);
            });
        })
}

#[test]
fn a_slider_row_s_parts_sit_where_the_design_puts_them() {
    let t = Theme::default();
    let mut harness = two_rows();
    harness.run();
    let strip = harness
        .get_by_role_and_label(Role::Slider, "Temperature")
        .rect();
    // 37 points a row: a point over the top line, the line a field high, the strip and 2
    // points under it.
    assert_eq!(t.sizes.slider_row, 37.0);
    let next = harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .rect();
    assert_eq!(next.top() - strip.top(), 37.0);
    // The pointer drags anywhere on the strip, the row's full width.
    assert_eq!((strip.width(), strip.height()), (216.0, 16.0));
    // The value at rest in the small text, its end 5 points from the row's, on the top
    // line, centred in it.
    let pad = t.spacing.xs + t.shape.hairline;
    let (value, text) = texts(&harness)
        .into_iter()
        .find(|(text, _)| text == "5000 K")
        .expect("the value is drawn with its unit");
    assert_eq!(value, "5000 K");
    assert!(
        (text.right() - (strip.right() - pad)).abs() < 0.5,
        "{text:?} by {strip:?}"
    );
    let top =
        egui::Rect::from_x_y_ranges(strip.x_range(), strip.top() - t.sizes.field..=strip.top());
    assert!(
        (text.center().y - top.center().y).abs() < 1.0,
        "{text:?} in {top:?}"
    );
    // The picker just before the value's hover band, which starts 4 points before it.
    let picker = harness.get_by_label("Pick a neutral").rect();
    assert_eq!((picker.width(), picker.height()), (18.0, 18.0));
    assert!(
        (picker.right() - (text.left() - 2.0 * t.spacing.xs)).abs() < 0.5
            && picker.top() == top.top(),
        "{picker:?} before {text:?}"
    );
    let marker = harness.get_by_label("Keep Temperature").rect();
    assert_eq!((marker.width(), marker.height()), (16.0, 18.0));
    assert_eq!(marker.top(), top.top());
    // The track, 4 thick, 6 under the strip's top.
    let shapes = &harness.output().shapes;
    let line = shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape)
                if shape.fill == t.colors.slider_track && strip.contains_rect(shape.rect) =>
            {
                Some(shape.rect)
            }
            _ => None,
        })
        .expect("the track is drawn");
    assert_eq!((line.top() - strip.top(), line.height()), (6.0, 4.0));
    // The handle, tip and base 5.5 each way of the track's middle, outline included,
    // inside the strip.
    let handle = shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Path(path)
                if path.fill == t.colors.slider_handle
                    && strip.intersects(path.visual_bounding_rect()) =>
            {
                Some(path)
            }
            _ => None,
        })
        .expect("the handle is drawn");
    let ys = handle.points.iter().map(|p| p.y - line.center().y);
    let (top, bottom) = ys.fold((0.0_f32, 0.0_f32), |(a, b), y| (a.min(y), b.max(y)));
    assert_eq!((top, bottom), (-5.5, 5.5));
    assert!(
        strip.contains_rect(handle.visual_bounding_rect()),
        "{handle:?} in {strip:?}"
    );
}

/// The value's text in the value field at `field`, as drawn: its rectangle and its clip.
fn value_text<State>(harness: &Harness<'_, State>, field: egui::Rect) -> (egui::Rect, egui::Rect) {
    harness
        .output()
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) => {
                let rect = shape.galley.rect.translate(shape.pos.to_vec2());
                field
                    .contains(rect.center())
                    .then_some((rect, clipped.clip_rect))
            }
            _ => None,
        })
        .expect("the value is drawn")
}

/// The tone equalizer's Edges refinement/feathering at its top, darktable's widest value,
/// at the side panel's narrowest, its label long: `value`, with two decimals.
fn feathering(value: f64) -> Harness<'static, f64> {
    let content = narrowest_content(&Theme::default());
    Harness::builder()
        .with_size(egui::vec2(260.0, 60.0))
        .with_context_setup(|ctx| {
            apertrust_ui::install(ctx, Theme::default());
            ctx.global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
        })
        .build_ui_state(
            move |ui, value: &mut f64| {
                ui.allocate_ui(egui::vec2(content, 40.0), |ui| {
                    let action =
                        SliderRow::new("Edges refinement/feathering", *value, 0.01..=10000.0)
                            .promise("How the mask follows the photo's edges.")
                            .decimals(2)
                            .show(ui);
                    if let Some(SliderAction::Set { value: set, .. }) = action {
                        *value = set;
                    }
                });
            },
            value,
        )
}

/// The value is never cut, darktable's widest, "10000.00", included: the label is, at its
/// end with …, whole in its tooltip and for screen readers. Opened, the field grows
/// towards the label for a number wider than a value field, holding it whole.
#[test]
fn at_the_narrowest_the_label_is_cut_and_the_value_stays_whole() {
    let t = Theme::default();
    let mut harness = feathering(10000.0);
    harness.run();
    let drawn = texts(&harness);
    let (value, value_at) = drawn
        .iter()
        .max_by(|a, b| a.1.right().total_cmp(&b.1.right()))
        .cloned()
        .expect("the value is drawn");
    assert_eq!(value, "10000.00");
    let (label, label_at) = drawn
        .iter()
        .min_by(|a, b| a.1.left().total_cmp(&b.1.left()))
        .cloned()
        .expect("the label is drawn");
    // The label cut, drawn narrower than it is whole; the value not.
    assert_eq!(
        (elided(&harness, &label), elided(&harness, &value)),
        (true, false)
    );
    let whole = harness.ctx.fonts_mut(|fonts| {
        let font = egui::FontId::proportional(t.text.body);
        fonts
            .layout_no_wrap(label.clone(), font, Color32::WHITE)
            .size()
            .x
    });
    assert!(label_at.width() < whole - 1.0, "{label}: {label_at:?}");
    assert!(
        label_at.right() < value_at.left(),
        "{label_at:?} runs into {value_at:?}"
    );
    let track = harness.get_by_role(Role::Slider);
    assert_eq!(
        track.accesskit_node().label().as_deref(),
        Some("Edges refinement/feathering")
    );
    assert_eq!(track.accesskit_node().value().as_deref(), Some("10000.00"));
    // The tooltip has the label whole, then the promise.
    harness.hover_at(label_at.center());
    harness.run();
    let tip = "Edges refinement/feathering\nHow the mask follows the photo's edges.";
    assert!(harness.query_by_label(tip).is_some(), "{tip}");
    // Opened, the field is as wide as its number with its padding, more than 54, and
    // holds it whole, 5 points from its end.
    harness.get_by_role(Role::Slider).focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    let (field, held) = open_field(&harness);
    assert_eq!(held, "10000.00");
    assert!(field.width() > t.sizes.value_column, "{field:?}");
    let (text, _) = value_text(&harness, field);
    let pad = t.spacing.xs + t.shape.hairline;
    assert!(
        (text.right() - (field.right() - pad)).abs() < 0.5
            && text.left() >= field.left() + pad - 0.5,
        "{text:?} in {field:?}"
    );
    // Left as it was, it sets nothing.
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(*harness.state(), 10000.0);
    // A value that fits a value field keeps its 54 points.
    let mut harness = feathering(50.0);
    harness.run();
    harness.get_by_role(Role::Slider).focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(open_field(&harness).0.width(), t.sizes.value_column);
}

/// A double-click on the value selects its whole number in the field the first click
/// opens, and resets nothing: a double-click resets on the label and the track only.
#[test]
fn a_double_click_on_the_value_selects_its_number() {
    let mut harness = exposure(Locale::EnGb);
    harness.state_mut().value = -1.25;
    harness.run();
    let (_, text) = resting_value(&harness);
    // Half a second after anything before, which would otherwise make a triple click.
    harness.run_steps(30);
    harness.hover_at(text.center());
    // Each queued event runs in a frame of its own.
    for pressed in [true, false, true, false] {
        harness.event(egui::Event::PointerButton {
            pos: text.center(),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    harness.run();
    assert_eq!(harness.state().last, None);
    harness.event(egui::Event::Text("2".to_owned()));
    harness.run();
    assert_eq!(open_field(&harness).1, "2");
}

/// The wireframe's padding as the browser draws it, 4 points inside a 1-point border: the
/// value sits 5 points from the field's end, as it does at rest, so nothing moves as the
/// field opens; typing included.
#[test]
fn a_value_field_s_text_keeps_its_padding_typing_included() {
    let t = Theme::default();
    let pad = t.spacing.xs + t.shape.hairline;
    assert_eq!(pad, 5.0);
    let mut harness = exposure(Locale::EnGb);
    harness.run();
    let (_, resting) = resting_value(&harness);
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    let (field, _) = open_field(&harness);
    let (text, _) = value_text(&harness, field);
    assert!(
        (text.right() - (field.right() - pad)).abs() < 0.5,
        "{text:?} in {field:?}"
    );
    assert!(
        (text.right() - resting.right()).abs() < 0.5,
        "{text:?} moved from {resting:?}"
    );
    harness.event(egui::Event::Text("−1.5".to_owned()));
    harness.run();
    let (text, clip) = value_text(&harness, field);
    assert!(
        (text.right() - (field.right() - pad)).abs() < 0.5,
        "{text:?} in {field:?}"
    );
    // A value too long to show whole scrolls inside the padding, less the point egui
    // leaves the cursor.
    assert!(
        clip.left() >= field.left() + t.spacing.xxs - 1.0
            && clip.right() <= field.right() - pad + 1.0,
        "{clip:?} in {field:?}"
    );
    // A value with room to spare keeps the start's 5 points too.
    assert!(text.left() >= field.left() + pad, "{text:?} in {field:?}");
}

/// Under the pointer the value takes the neutral hover band and an I-beam, and its tooltip
/// says how to type one; a disabled row's value is dimmed, with neither.
#[test]
fn the_value_under_the_pointer_takes_the_hover_band_and_an_i_beam() {
    let t = Theme::default();
    let mut harness = exposure(Locale::EnGb);
    harness
        .ctx
        .global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
    harness.run();
    let (_, text) = resting_value(&harness);
    let band = |harness: &Harness<'_, Exposure>| {
        harness
            .output()
            .shapes
            .iter()
            .any(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape) => {
                    shape.fill == t.colors.hover && shape.rect.contains_rect(text)
                }
                _ => false,
            })
    };
    assert!(!band(&harness));
    harness.hover_at(text.center());
    harness.run();
    assert!(band(&harness));
    assert_eq!(
        harness.output().platform_output.cursor_icon,
        egui::CursorIcon::Text
    );
    harness.run();
    let tip = Translator::new(Locale::EnGb).text("slider-value-tip");
    assert!(harness.query_by_label(&tip).is_some(), "{tip}");
}

/// Two slider rows as a panel draws them, shown or hidden as a tab switch hides them, and
/// enabled or not, and what each row set, in order.
struct Rows {
    values: [f64; 2],
    shown: bool,
    enabled: bool,
    sets: Vec<(usize, f64)>,
}

const ROWS: [&str; 2] = ["Exposure", "Contrast"];

fn rows(os: OperatingSystem) -> Harness<'static, Rows> {
    Harness::builder()
        .with_size(egui::vec2(320.0, 100.0))
        .with_os(os)
        .with_context_setup(|ctx| {
            apertrust_ui::install(ctx, Theme::default());
            ctx.global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
        })
        .build_ui_state(
            |ui, rows: &mut Rows| {
                if !rows.shown {
                    return;
                }
                for (i, name) in ROWS.iter().enumerate() {
                    let action = SliderRow::new(name, rows.values[i], -4.0..=4.0)
                        .decimals(2)
                        .enabled(rows.enabled)
                        .show(ui);
                    if let Some(SliderAction::Set { value, .. }) = action {
                        rows.values[i] = value;
                        rows.sets.push((i, value));
                    }
                }
            },
            Rows {
                values: [0.0; 2],
                shown: true,
                enabled: true,
                sets: Vec::new(),
            },
        )
}

/// Where row `row`'s value is drawn at rest: the texts furthest towards the rows' end, from
/// the top.
fn value_at<State>(harness: &Harness<'_, State>, row: usize) -> egui::Rect {
    let drawn = texts(harness);
    let end = drawn
        .iter()
        .map(|(_, rect)| rect.right())
        .fold(f32::MIN, f32::max);
    let mut values: Vec<egui::Rect> = drawn
        .into_iter()
        .filter(|(_, rect)| (rect.right() - end).abs() < 0.5)
        .map(|(_, rect)| rect)
        .collect();
    values.sort_by(|a, b| a.top().total_cmp(&b.top()));
    values[row]
}

/// Opens Exposure's field from the keyboard and types `text` into it, setting nothing.
fn start_typing(harness: &mut Harness<'_, Rows>, text: &str) {
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    harness.event(egui::Event::Text(text.to_owned()));
    harness.run();
    assert_eq!(open_field(harness).1, text);
}

/// What is typed belongs to the focus that typed it. A row hidden before Enter, as a tab
/// switched with Ctrl and a digit hides it, never sees its field lose the focus: shown
/// again, the value is at rest, with nothing typed flashing in a field, and a click opens
/// the field holding the value, selected.
#[test]
fn a_value_typed_in_a_row_hidden_before_enter_never_comes_back() {
    let mut harness = rows(OperatingSystem::Nix);
    harness.run();
    start_typing(&mut harness, "1.5");
    harness.state_mut().shown = false;
    harness.run();
    harness.state_mut().shown = true;
    // The first frame back, and every one after it.
    for _ in 0..3 {
        harness.step();
        assert!(harness.query_by_role(Role::TextInput).is_none());
        assert!(
            !texts(&harness).iter().any(|(text, _)| text == "1.5"),
            "{:?}",
            texts(&harness)
        );
    }
    harness.run();
    assert!(harness.state().sets.is_empty());
    let at = value_at(&harness, 0).center();
    click_at(&mut harness, at);
    assert_eq!(open_field(&harness).1, "0.00");
    harness.event(egui::Event::Text("2".to_owned()));
    harness.run();
    assert_eq!(open_field(&harness).1, "2");
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().sets, [(0, 2.0)]);
}

/// A press on another row's track takes the focus after the field has been drawn that
/// frame: the field, losing it a frame late, still sets what was typed, as a click
/// elsewhere does.
#[test]
fn a_press_on_another_row_s_track_sets_the_typed_value() {
    let mut harness = rows(OperatingSystem::Nix);
    harness.run();
    start_typing(&mut harness, "1.5");
    let track = harness
        .get_by_role_and_label(Role::Slider, "Contrast")
        .rect();
    harness.drag_at(track.center());
    harness.run();
    harness.drop_at(track.center());
    harness.run();
    assert_eq!(harness.state().sets.first(), Some(&(0, 1.5)));
    assert!(harness.query_by_role(Role::TextInput).is_none());
}

/// Enter and Escape in the field give the focus back to the track, so a keyboard user
/// goes on from where they were; the arrows step the value again.
#[test]
fn enter_and_escape_in_the_field_give_the_focus_back_to_the_track() {
    let mut harness = exposure(Locale::EnGb);
    harness.run();
    let track = (Role::Slider, "Exposure".to_owned());
    type_value(&mut harness, "Exposure", "1.5", Key::Enter);
    assert_eq!(focused(&harness), track);
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert!(
        near(harness.state().value, 1.51),
        "{}",
        harness.state().value
    );
    type_value(&mut harness, "Exposure", "3", Key::Escape);
    assert_eq!(focused(&harness), track);
    assert!(near(harness.state().value, 1.51));
}

/// A click on the value of a row disabled before its field could open opens nothing, then
/// or once the row is enabled again.
#[test]
fn a_click_on_a_row_disabled_before_its_field_opens_opens_nothing() {
    let mut harness = rows(OperatingSystem::Nix);
    harness.run();
    let at = value_at(&harness, 0).center();
    harness.hover_at(at);
    harness.run();
    // Each queued event runs in a frame of its own: the release is the click's.
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    harness.step();
    harness.step();
    for enabled in [false, false, true, true] {
        harness.state_mut().enabled = enabled;
        harness.step();
        assert!(
            harness.query_by_role(Role::TextInput).is_none(),
            "enabled: {enabled}"
        );
    }
}

/// On the Mac the key is Return, as its keyboard and the app's actions name it: to screen
/// readers and in the value's tooltip.
#[test]
fn on_the_mac_the_value_s_key_is_return() {
    for (os, key, tip) in [
        (OperatingSystem::Mac, "Return", "slider-value-tip-mac"),
        (OperatingSystem::Nix, "Enter", "slider-value-tip"),
        (OperatingSystem::Windows, "Enter", "slider-value-tip"),
    ] {
        let mut harness = rows(os);
        harness.run();
        let track = harness.get_by_role_and_label(Role::Slider, "Exposure");
        assert_eq!(
            track.accesskit_node().data().keyboard_shortcut(),
            Some(key),
            "{os:?}"
        );
        harness.hover_at(value_at(&harness, 0).center());
        harness.run();
        let tip = Translator::new(Locale::EnGb).text(tip);
        assert!(harness.query_by_label(&tip).is_some(), "{os:?}: {tip}");
    }
}

/// Every colour drawn over `area`: rectangles' fills and strokes, lines, and text, its
/// selection's fill included.
fn colours_in<State>(harness: &Harness<'_, State>, area: egui::Rect) -> Vec<Color32> {
    let mut colours = Vec::new();
    for clipped in &harness.output().shapes {
        match &clipped.shape {
            egui::Shape::Rect(shape) if area.intersects(shape.rect) => {
                colours.extend([shape.fill, shape.stroke.color]);
            }
            egui::Shape::LineSegment { points, stroke }
                if points.iter().any(|point| area.contains(*point)) =>
            {
                colours.push(stroke.color);
            }
            egui::Shape::Text(shape) if area.intersects(shape.visual_bounding_rect()) => {
                colours.extend(shape.override_text_color);
                for row in &shape.galley.rows {
                    colours.extend(row.visuals.mesh.vertices.iter().map(|v| v.color));
                }
            }
            _ => {}
        }
    }
    colours
}

/// The value is never the accent while it is edited, its slider dragged or a value typed.
#[test]
fn nothing_in_a_value_field_turns_the_accent_while_its_value_is_edited() {
    let c = Theme::default().colors;
    let accent = [
        c.accent,
        c.accent_text,
        c.accent_subtle,
        c.focus_ring,
        c.selection,
        c.slider_handle_active,
    ];
    let mut harness = exposure(Locale::EnGb);
    harness.run();
    let (_, value) = resting_value(&harness);
    let track = harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .rect();
    let neutral = |drawn: &[Color32]| !drawn.iter().any(|colour| accent.contains(colour));
    // Dragged: the value follows in the text colour.
    let at = pos2(track.left() + track.width() * 0.75, track.center().y);
    harness.drag_at(at);
    harness.step();
    let drawn = colours_in(&harness, value);
    assert!(drawn.contains(&c.text) && neutral(&drawn), "{drawn:?}");
    harness.drop_at(at);
    harness.run();
    // Typed in: the selection neutral, the text and the cursor in the text colour.
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    let (field, _) = open_field(&harness);
    let drawn = colours_in(&harness, field);
    assert!(drawn.contains(&c.field_selection), "the selection shows");
    assert!(neutral(&drawn), "{drawn:?}");
    harness.event(egui::Event::Text("1".to_owned()));
    harness.run();
    let drawn = colours_in(&harness, field);
    assert!(drawn.contains(&c.text) && neutral(&drawn), "{drawn:?}");
}

/// Tab goes through a row as it reads: the marker, the quad, the track; the value field is
/// a stop only while it is open.
#[test]
fn tab_follows_the_slider_rows_reading_order() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(560.0, 180.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui(|ui| {
            for (label, quad, enabled) in [
                ("Exposure", Some("Pick exposure"), true),
                ("Black level", Some("Pick black level"), false),
                ("Contrast", None, true),
            ] {
                let mut row = SliderRow::new(label, 0.0, -4.0..=4.0).enabled(enabled);
                if let Some(name) = quad {
                    row = row.quad(Quad {
                        icon: Icon::Pick,
                        name,
                        active: false,
                    });
                }
                row.show(ui);
            }
        });
    harness.run();
    let order = |row: &str, quad: Option<&str>| {
        let mut stops = vec![(Role::Button, format!("Keep {row}"))];
        stops.extend(quad.map(|quad| (Role::Button, quad.to_owned())));
        stops.push((Role::Slider, row.to_owned()));
        stops
    };
    // Black level is disabled: Tab passes over the whole row.
    let rows = [
        order("Exposure", Some("Pick exposure")),
        order("Contrast", None),
    ]
    .concat();
    let mut seen = Vec::new();
    for _ in &rows {
        harness.key_press(Key::Tab);
        harness.run();
        seen.push(focused(&harness));
    }
    assert_eq!(seen, rows);
    // Shift+Tab goes back the same way.
    let mut back = Vec::new();
    for _ in 1..rows.len() {
        harness.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
        harness.run();
        back.push(focused(&harness));
    }
    back.reverse();
    assert_eq!(back, rows[..rows.len() - 1]);
    // Open, the field is a stop before the track; leaving it closes it again.
    harness
        .get_by_role_and_label(Role::Slider, "Contrast")
        .focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(
        focused(&harness),
        (Role::TextInput, "Contrast: exact value".to_owned())
    );
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
    harness.run();
    assert_eq!(
        focused(&harness),
        (Role::Button, "Keep Contrast".to_owned())
    );
    assert!(harness.query_by_role(Role::TextInput).is_none());
}

#[test]
fn a_segmented_control_reports_the_chosen_option() {
    let mut chosen = None;
    let mut harness = Harness::builder()
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui(|ui| {
            if let Some(i) = Segmented::new("Scale", &["Linear", "Log"], 0).show(ui) {
                chosen = Some(i);
            }
        });
    harness.run();
    harness.get_by_label("Log").click();
    harness.run();
    drop(harness);
    assert_eq!(chosen, Some(1));
}

/// Colour calibration's illuminants, the third too long to share a line at the side
/// panel's narrowest.
const ILLUMINANTS: [&str; 4] = [
    "Daylight",
    "Black body",
    "Fluorescent F11 (narrow tri-band)",
    "Custom",
];

/// A dropdown row and what it last reported, applied as the app would, with the options
/// it lists disabled, what its box shows while none is chosen, an option the photo does
/// not list, its style and its kept marker, on a row that has one.
struct Dropdown {
    selected: Option<usize>,
    picking: bool,
    last: Option<ChoiceAction>,
    disabled: Vec<usize>,
    none_chosen: Option<&'static str>,
    unlisted: Option<usize>,
    style: ChoiceStyle,
    mark: Option<Mark>,
}

/// A dropdown row of the illuminants in a window `width` points wide, Boxed, with a picker
/// before its box when `quad`.
fn dropdown(width: f32, quad: bool) -> Harness<'static, Dropdown> {
    Harness::builder()
        .with_size(egui::vec2(width, 240.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            move |ui, state: &mut Dropdown| {
                let mut row = Choice::new("Illuminant", &ILLUMINANTS, state.selected)
                    .name("Adaptation illuminant")
                    .disabled_options(&state.disabled, "Coming in a later version")
                    .style(state.style);
                if let Some(text) = state.none_chosen {
                    row = row.none_chosen(text);
                }
                if let Some(option) = state.unlisted {
                    row = row.unlisted(option, UNLISTED);
                }
                if let Some(mark) = state.mark {
                    row = row.mark(mark);
                }
                if quad {
                    row = row.quad(Quad {
                        icon: Icon::Pick,
                        name: "Pick an illuminant on the photo",
                        active: state.picking,
                    });
                }
                let action = row.show(ui);
                match action {
                    Some(ChoiceAction::Set(i)) => state.selected = Some(i),
                    Some(ChoiceAction::Quad) => state.picking = !state.picking,
                    Some(ChoiceAction::ToggleKeep) => {
                        state.mark = match state.mark {
                            Some(Mark::Kept) => Some(Mark::None),
                            _ => Some(Mark::Kept),
                        };
                    }
                    None => {}
                }
                if action.is_some() {
                    state.last = action;
                }
            },
            Dropdown {
                selected: Some(0),
                picking: false,
                last: None,
                disabled: Vec::new(),
                none_chosen: None,
                unlisted: None,
                style: ChoiceStyle::Boxed,
                mark: None,
            },
        )
}

/// The same row Blended, as a module panel draws it, with its kept marker.
fn blended(width: f32, quad: bool) -> Harness<'static, Dropdown> {
    let mut harness = dropdown(width, quad);
    harness.state_mut().style = ChoiceStyle::Blended;
    harness.state_mut().mark = Some(Mark::None);
    harness
}

const ILLUMINANT: &str = "Adaptation illuminant";

/// What a dropdown row says of an option the photo does not list (decision 30).
const UNLISTED: &str = "Not available for this photo";

/// Presses and releases the primary button at `at`, as a click where no node is.
fn click_at<State>(harness: &mut Harness<'_, State>, at: egui::Pos2) {
    harness.hover_at(at);
    harness.run();
    harness.drag_at(at);
    harness.drop_at(at);
    harness.run();
}

/// A dropdown row's height: its box, a field high, with a point over and under it.
fn one_line() -> f32 {
    let t = Theme::default();
    t.sizes.field + t.spacing.xxs
}

/// A Blended row's height: a module panel's one-line row.
fn panel_line() -> f32 {
    Theme::default().sizes.panel_line
}

#[test]
fn a_dropdown_row_opens_from_its_label_and_reports_the_chosen_option() {
    let mut harness = dropdown(400.0, false);
    harness.run();
    let row = harness
        .get_by_role_and_label(Role::ComboBox, ILLUMINANT)
        .rect();
    assert_eq!(row.height(), one_line());
    assert_eq!(row.height(), 20.0);
    // The label, at the row's start.
    click_at(&mut harness, pos2(row.left() + 4.0, row.center().y));
    // The list opens under the box, its end edge on the box's, the row's end.
    let item = harness.get_by_label("Black body").rect();
    assert!(item.top() > row.center().y, "{item:?} under {row:?}");
    assert!(
        item.right() <= row.right() && item.right() > row.right() - 12.0,
        "{item:?} ends with {row:?}"
    );
    harness.get_by_label("Black body").click();
    harness.run();
    assert_eq!(harness.state().last, Some(ChoiceAction::Set(1)));
    let node = harness.get_by_role_and_label(Role::ComboBox, ILLUMINANT);
    assert_eq!(node.accesskit_node().value().as_deref(), Some("Black body"));
}

#[test]
fn a_dropdown_row_s_quad_does_not_open_it_and_its_box_takes_the_keys() {
    let mut harness = dropdown(400.0, true);
    harness.run();
    harness
        .get_by_label("Pick an illuminant on the photo")
        .click();
    harness.run();
    assert_eq!(harness.state().last, Some(ChoiceAction::Quad));
    assert!(harness.query_by_label("Black body").is_none());
    // One focus stop, the box: Enter opens the list.
    harness
        .get_by_role_and_label(Role::ComboBox, ILLUMINANT)
        .focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.query_by_label("Black body").is_some());
}

#[test]
fn down_on_a_focused_dropdown_row_opens_its_list() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(400.0, 240.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui(|ui| {
            Choice::new("Illuminant", &ILLUMINANTS, 0)
                .name(ILLUMINANT)
                .show(ui);
            // Under the row, where egui's own ↓ would move the focus.
            IconButton::new(Icon::Undo, "Undo").show(ui);
        });
    harness.run();
    harness
        .get_by_role_and_label(Role::ComboBox, ILLUMINANT)
        .focus();
    harness.run();
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert!(harness.query_by_label("Black body").is_some());
    // Consumed, so egui's own ↓ did not move the focus to the button under the row: it
    // is on the chosen option in the list, and Escape gives it back to the box.
    assert_eq!(focused(&harness).1, "Daylight");
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.query_by_label("Black body").is_none());
    assert_eq!(focused(&harness), (Role::ComboBox, ILLUMINANT.to_owned()));
}

/// The text drawn for `text`, as laid out: whether it was cut, and its rectangle.
fn drawn_text<State>(harness: &Harness<'_, State>, text: &str) -> Option<(bool, egui::Rect)> {
    harness
        .output()
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.text() == text => Some((
                shape.galley.elided,
                shape.galley.rect.translate(shape.pos.to_vec2()),
            )),
            _ => None,
        })
}

#[test]
fn a_dropdown_row_too_narrow_for_its_widest_option_shares_its_one_line() {
    let height = |harness: &Harness<'_, Dropdown>| {
        harness
            .get_by_role_and_label(Role::ComboBox, ILLUMINANT)
            .rect()
            .height()
    };
    // Room for the label, the gap and the box at its widest option: nothing cut.
    let mut wide = dropdown(400.0, false);
    wide.run();
    assert_eq!(height(&wide), one_line());
    assert_eq!(
        drawn_text(&wide, "Illuminant").map(|(cut, _)| cut),
        Some(false)
    );
    // Not, though the chosen Daylight would fit: the label and the box share the line,
    // whatever is chosen, and the label is cut.
    let mut narrow = dropdown(240.0, false);
    narrow.run();
    assert_eq!(height(&narrow), one_line());
    let box_daylight = drawn_box(&narrow);
    assert_eq!(
        drawn_text(&narrow, "Illuminant").map(|(cut, _)| cut),
        Some(true)
    );
    narrow.state_mut().selected = Some(2);
    narrow.run();
    assert_eq!(height(&narrow), one_line());
    assert_eq!(
        drawn_box(&narrow),
        box_daylight,
        "a choice keeps the layout"
    );
    // Narrower still: both are cut, while screen readers hear the whole option.
    let mut cut = dropdown(160.0, false);
    cut.state_mut().selected = Some(2);
    cut.run();
    let node = cut.get_by_role_and_label(Role::ComboBox, ILLUMINANT);
    assert_eq!(node.rect().height(), one_line());
    assert_eq!(
        node.accesskit_node().value().as_deref(),
        Some("Fluorescent F11 (narrow tri-band)")
    );
    let option = drawn_text(&cut, ILLUMINANTS[2]).expect("the option is drawn");
    assert!(option.0, "the option is cut");
    assert!(option.1.right() <= drawn_box(&cut).right(), "{option:?}");
}

/// darktable's rule for a combo box too narrow for its label and its value, at the side
/// panel's narrowest content, 216 points: one line, the label and the box sharing what the
/// gap leaves in proportion to their widths, each cut at its end.
#[test]
fn a_long_label_and_long_options_share_one_line_at_the_narrowest_side_panel() {
    const LABEL: &str = "Chromatic adaptation transform for the illuminant";
    const OPTIONS: [&str; 2] = [
        "Linear Bradford (ICC v4), as darktable writes it",
        "CAT16 (CIECAM16), the default for new edits",
    ];
    let t = Theme::default();
    let content = narrowest_content(&t);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(280.0, 80.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui(move |ui| {
            ui.allocate_ui(egui::vec2(content, 40.0), |ui| {
                Choice::new(LABEL, &OPTIONS, 1).name("Adaptation").show(ui);
            });
        });
    harness.run();
    let row = harness.get_by_role_and_label(Role::ComboBox, "Adaptation");
    assert_eq!(row.rect().width(), 216.0);
    assert_eq!(
        row.rect().height(),
        one_line(),
        "one line, as with short words"
    );
    assert_eq!(row.accesskit_node().value().as_deref(), Some(OPTIONS[1]));
    let row = row.rect();
    let (label_cut, label) = drawn_text(&harness, LABEL).expect("the label is drawn");
    let (option_cut, option) = drawn_text(&harness, OPTIONS[1]).expect("the option is drawn");
    assert!(label_cut && option_cut, "both cut");
    // Side by side on the row's line, the gap between them.
    assert!(
        label.right() <= option.left() - t.spacing.l,
        "{label:?}, {option:?}"
    );
    for text in [label, option] {
        assert!(
            text.top() >= row.top() && text.bottom() <= row.bottom() + 1.0,
            "{text:?} in {row:?}"
        );
    }
}

/// The dropdown row's box, as drawn: the one well-coloured fill in the row.
fn drawn_box(harness: &Harness<'_, Dropdown>) -> egui::Rect {
    let row = harness
        .get_by_role_and_label(Role::ComboBox, ILLUMINANT)
        .rect();
    let sunken = Theme::default().colors.sunken;
    let mut boxes = harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape) if shape.fill == sunken && row.contains_rect(shape.rect) => {
                Some(shape.rect)
            }
            _ => None,
        });
    let found = boxes.next().expect("the box is drawn");
    assert_eq!(boxes.next(), None, "one box");
    found
}

/// Opens the dropdown row and chooses `option` in its list.
fn choose(harness: &mut Harness<'_, Dropdown>, option: &str) {
    harness
        .get_by_role_and_label(Role::ComboBox, ILLUMINANT)
        .click();
    harness.run();
    harness.get_by_label(option).click();
    harness.run();
    let node = harness.get_by_role_and_label(Role::ComboBox, ILLUMINANT);
    assert_eq!(node.accesskit_node().value().as_deref(), Some(option));
}

#[test]
fn a_disabled_option_is_listed_but_never_chosen() {
    let mut harness = dropdown(400.0, false);
    // Fluorescent F11, as an option darktable lists that is not ported.
    harness.state_mut().disabled = vec![2];
    harness.run();
    harness
        .get_by_role_and_label(Role::ComboBox, ILLUMINANT)
        .click();
    harness.run();
    let fluorescent = harness.get_by_label(ILLUMINANTS[2]);
    assert!(fluorescent.accesskit_node().is_disabled());
    fluorescent.click();
    harness.run();
    // Nothing chosen, and the list stays open with the option greyed among the others.
    assert_eq!(harness.state().last, None);
    harness.snapshot("dropdown-disabled-option");
    // The arrow keys pass over it, from the chosen Daylight to Black body, Custom and back.
    assert_eq!(focused(&harness).1, ILLUMINANTS[0]);
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(focused(&harness).1, ILLUMINANTS[1]);
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(focused(&harness).1, ILLUMINANTS[3]);
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert_eq!(focused(&harness).1, ILLUMINANTS[1]);
    // The others are chosen as ever.
    harness.get_by_label(ILLUMINANTS[3]).click();
    harness.run();
    assert_eq!(harness.state().last, Some(ChoiceAction::Set(3)));
}

#[test]
fn a_dropdown_row_s_box_keeps_its_width_when_the_choice_changes() {
    let mut harness = dropdown(400.0, false);
    harness.run();
    choose(&mut harness, "Custom");
    let short = drawn_box(&harness);
    choose(&mut harness, "Fluorescent F11 (narrow tri-band)");
    let long = drawn_box(&harness);
    assert_eq!(short.width(), long.width(), "{short:?} then {long:?}");
}

#[test]
fn a_dropdown_row_with_no_option_chosen_shows_its_own_text_which_the_list_never_offers() {
    // Wider than every illuminant, as a preset row's Custom may be in some language.
    const NONE: &str = "Custom, matching none of the illuminants";
    let mut plain = dropdown(400.0, false);
    plain.run();
    let widest = drawn_box(&plain);
    let mut harness = dropdown(400.0, false);
    harness.state_mut().selected = None;
    harness.state_mut().none_chosen = Some(NONE);
    harness.run();
    let node = harness.get_by_role_and_label(Role::ComboBox, ILLUMINANT);
    assert_eq!(node.accesskit_node().value().as_deref(), Some(NONE));
    let unmatched = drawn_box(&harness);
    assert!(
        unmatched.width() > widest.width(),
        "{unmatched:?}, {widest:?}"
    );
    // The list offers every option, and never the box's own text.
    harness
        .get_by_role_and_label(Role::ComboBox, ILLUMINANT)
        .click();
    harness.run();
    for option in ILLUMINANTS {
        harness.get_by_label(option);
    }
    assert!(harness.query_by_label(NONE).is_none());
    harness.get_by_label(ILLUMINANTS[1]).click();
    harness.run();
    assert_eq!(harness.state().last, Some(ChoiceAction::Set(1)));
    // Its text counts towards the box's width, so the box keeps it.
    let chosen = drawn_box(&harness);
    assert_eq!(
        chosen.width(),
        unmatched.width(),
        "{chosen:?}, {unmatched:?}"
    );
}

/// An option the photo does not list, as the app puts it last in the list, from another
/// photo's edit (decision 30): chosen, its box says so in its tooltip and to screen
/// readers, and the list draws it disabled with that tooltip, so choosing it reports
/// nothing; another option chosen, the row says nothing of it.
#[test]
fn a_dropdown_row_s_unlisted_option_is_chosen_disabled_and_described() {
    let mut harness = dropdown(400.0, false);
    harness.state_mut().selected = Some(3);
    harness.state_mut().unlisted = Some(3);
    harness.run();
    let node = harness.get_by_role_and_label(Role::ComboBox, ILLUMINANT);
    assert_eq!(node.accesskit_node().value().as_deref(), Some("Custom"));
    assert_eq!(
        node.accesskit_node().description().as_deref(),
        Some(UNLISTED)
    );
    assert!(!node.accesskit_node().is_disabled());
    node.hover();
    harness.run();
    assert!(harness.query_by_label(UNLISTED).is_some());
    harness
        .get_by_role_and_label(Role::ComboBox, ILLUMINANT)
        .click();
    harness.run();
    let custom = harness.get_by_label("Custom");
    assert!(custom.accesskit_node().is_disabled());
    custom.click();
    harness.run();
    assert_eq!(harness.state().last, None);
    // Another option chosen, the row no longer describes itself so.
    if harness.query_by_label(ILLUMINANTS[1]).is_none() {
        harness
            .get_by_role_and_label(Role::ComboBox, ILLUMINANT)
            .click();
        harness.run();
    }
    harness.get_by_label(ILLUMINANTS[1]).click();
    harness.run();
    assert_eq!(harness.state().last, Some(ChoiceAction::Set(1)));
    let node = harness.get_by_role_and_label(Role::ComboBox, ILLUMINANT);
    let said = node.accesskit_node().description().unwrap_or_default();
    assert!(!said.contains(UNLISTED), "{said}");
}

/// The Blended row's kept marker, as screen readers name it.
const KEEP: &str = "Keep Adaptation illuminant";

/// The chevron drawn in `within`, the last icon there, and where its stroke ends:
/// `chevron-down.svg`'s path ends at 8 of its 10 points, with a round cap half its
/// 1.4-point stroke wide.
fn chevron<State>(harness: &Harness<'_, State>, within: egui::Rect) -> (egui::Rect, f32) {
    let icon = harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Mesh(mesh) if mesh.texture_id != egui::TextureId::default() => {
                Some(mesh.calc_bounds())
            }
            _ => None,
        })
        .filter(|rect| within.contains_rect(*rect))
        .max_by(|a, b| a.right().total_cmp(&b.right()))
        .expect("the chevron is drawn");
    (icon, icon.left() + icon.width() * 0.87)
}

/// A slider row over a Blended dropdown row, as a module panel stacks them, `width` points
/// wide.
fn slider_over_choice(width: f32) -> Harness<'static> {
    Harness::builder()
        .with_size(egui::vec2(width + 40.0, 120.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui(move |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.allocate_ui(egui::vec2(width, 100.0), |ui| {
                SliderRow::new("Exposure", 0.42, -4.0..=4.0)
                    .decimals(2)
                    .unit("unit-ev-value")
                    .show(ui);
                Choice::new("Illuminant", &ILLUMINANTS, 1)
                    .name(ILLUMINANT)
                    .style(ChoiceStyle::Blended)
                    .mark(Mark::Kept)
                    .show(ui);
            });
        })
}

/// darktable's combobox puts its value where its sliders put theirs (decision 42): the
/// chevron's stroke ends where the value at rest does, 5 points from the row's end, at the
/// side panel's 320 points and at its narrowest, where the option is cut.
#[test]
fn a_blended_chevron_ends_where_a_slider_s_value_at_rest_ends() {
    let t = Theme::default();
    let pad = t.spacing.xs + t.shape.hairline;
    for width in [296.0, narrowest_content(&t)] {
        let mut harness = slider_over_choice(width);
        harness.run();
        let (_, value) = texts(&harness)
            .into_iter()
            .find(|(text, _)| text == "+0.42 EV")
            .expect("the value is drawn");
        let row = harness
            .get_by_role_and_label(Role::ComboBox, ILLUMINANT)
            .rect();
        assert_eq!(row.width(), width);
        let (_, stroke) = chevron(&harness, row);
        assert!(
            (stroke - value.right()).abs() <= 0.5,
            "at {width}: the stroke ends at {stroke}, the value at {}",
            value.right()
        );
        assert!((stroke - (row.right() - pad)).abs() <= 0.5, "at {width}");
    }
}

#[test]
fn a_blended_row_has_no_box_and_no_minimum_width() {
    let t = Theme::default();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(300.0, 80.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui(|ui| {
            Choice::new("Shape", &["On", "Off"], 0)
                .style(ChoiceStyle::Blended)
                .show(ui);
        });
    harness.run();
    let row = harness
        .get_by_role_and_label(Role::ComboBox, "Shape")
        .rect();
    // Nothing filled in the row away from the pointer but the harness's frame round it, in
    // the panel's colour: no box.
    let filled: Vec<_> = harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape)
                if row.expand(t.spacing.xs).contains_rect(shape.rect)
                    && ![Color32::TRANSPARENT, t.colors.panel].contains(&shape.fill) =>
            {
                Some(shape)
            }
            _ => None,
        })
        .collect();
    assert!(filled.is_empty(), "{filled:?} in {row:?}");
    // The option right-aligned a gap before ⌄, the two narrower than a value field.
    let (_, option) = drawn_text(&harness, "On").expect("the option is drawn");
    let (icon, _) = chevron(&harness, row);
    assert!(
        (icon.left() - t.spacing.xs - option.right()).abs() <= 0.5,
        "{option:?} before {icon:?}"
    );
    assert!(icon.right() - option.left() < t.sizes.value_column);
}

/// Neither a choice nor keeping it moves the row's parts: the widest option decides the
/// layout, and the marker's place is the row's, kept or not.
#[test]
fn a_choice_or_keeping_it_never_moves_a_blended_row() {
    let mut harness = blended(240.0, true);
    harness.run();
    let layout = |harness: &Harness<'_, Dropdown>| {
        let row = harness
            .get_by_role_and_label(Role::ComboBox, ILLUMINANT)
            .rect();
        let (cut, label) = drawn_text(harness, "Illuminant").expect("the label is drawn");
        (
            row,
            cut,
            label,
            harness.get_by_label(KEEP).rect(),
            harness
                .get_by_label("Pick an illuminant on the photo")
                .rect(),
            chevron(harness, row).0,
        )
    };
    let first = layout(&harness);
    assert_eq!(first.0.height(), panel_line());
    assert!(
        first.1,
        "the label shares the line with the widest option, and is cut"
    );
    for (selected, mark) in [(2, Mark::None), (3, Mark::Kept), (2, Mark::Kept)] {
        harness.state_mut().selected = Some(selected);
        harness.state_mut().mark = Some(mark);
        harness.run();
        assert_eq!(layout(&harness), first, "{selected}, {mark:?}");
    }
}

#[test]
fn the_kept_marker_of_a_blended_row_keeps_and_releases_it() {
    use egui::accesskit::Toggled;
    let kept_marker = Theme::default().colors.kept_marker;
    let mut harness = blended(400.0, false);
    harness.run();
    let dot = |harness: &Harness<'_, Dropdown>| {
        let marker = harness.get_by_label(KEEP).rect();
        harness.output().shapes.iter().any(|clipped| {
            matches!(&clipped.shape, egui::Shape::Circle(circle)
                if circle.fill == kept_marker && marker.contains(circle.center))
        })
    };
    let toggled = |harness: &Harness<'_, Dropdown>| {
        let marker = harness.get_by_role_and_label(Role::Button, KEEP);
        marker.accesskit_node().toggled()
    };
    assert!(!dot(&harness));
    assert_eq!(toggled(&harness), Some(Toggled::False));
    harness.get_by_label(KEEP).click();
    harness.run();
    assert_eq!(harness.state().last, Some(ChoiceAction::ToggleKeep));
    assert_eq!(harness.state().mark, Some(Mark::Kept));
    assert!(dot(&harness));
    assert_eq!(toggled(&harness), Some(Toggled::True));
    assert!(
        harness.query_by_label("Black body").is_none(),
        "the list stays closed"
    );
    harness.get_by_label(KEEP).click();
    harness.run();
    assert_eq!(harness.state().mark, Some(Mark::None));
    assert!(!dot(&harness));
}

/// A click anywhere on the row opens its list, the label, the gap, the option and ⌄, but
/// the marker and the quad, which do what they do.
#[test]
fn a_blended_row_opens_from_anywhere_but_its_marker_and_its_quad() {
    let mut harness = blended(400.0, true);
    harness.run();
    let row = harness
        .get_by_role_and_label(Role::ComboBox, ILLUMINANT)
        .rect();
    let y = row.center().y;
    let marker = harness.get_by_label(KEEP).rect();
    let quad = harness
        .get_by_label("Pick an illuminant on the photo")
        .rect();
    let (_, option) = drawn_text(&harness, ILLUMINANTS[0]).expect("the option is drawn");
    let (icon, _) = chevron(&harness, row);
    let opening = [
        pos2(row.left() + 4.0, y),
        pos2((marker.right() + quad.left()) / 2.0, y),
        option.center(),
        icon.center(),
        pos2(row.right() - 1.0, y),
    ];
    for at in opening {
        click_at(&mut harness, at);
        assert!(
            harness.query_by_label("Black body").is_some(),
            "a click at {at:?} opens the list"
        );
        harness.key_press(Key::Escape);
        harness.run();
        assert!(harness.query_by_label("Black body").is_none());
    }
    assert_eq!(harness.state().last, None);
    for (at, action) in [
        (marker.center(), ChoiceAction::ToggleKeep),
        (quad.center(), ChoiceAction::Quad),
    ] {
        click_at(&mut harness, at);
        assert_eq!(harness.state().last, Some(action));
        assert!(
            harness.query_by_label("Black body").is_none(),
            "{action:?} opens nothing"
        );
    }
}

/// Screen readers hear a Blended row as a Boxed one: a combo box of the row's size, named
/// by the control's full name, the option its value, and an unlisted option described.
#[test]
fn a_blended_row_says_what_a_boxed_one_says() {
    let heard = |style: ChoiceStyle, unlisted: Option<usize>| {
        let mut harness = dropdown(400.0, false);
        harness.state_mut().style = style;
        harness.state_mut().selected = Some(3);
        harness.state_mut().unlisted = unlisted;
        harness.run();
        let node = harness.get_by_role_and_label(Role::ComboBox, ILLUMINANT);
        let rect = node.rect();
        let node = node.accesskit_node();
        (rect, node.value(), node.description(), node.is_disabled())
    };
    for unlisted in [None, Some(3)] {
        let boxed = heard(ChoiceStyle::Boxed, unlisted);
        assert_eq!(boxed.1.as_deref(), Some(ILLUMINANTS[3]));
        let blended = heard(ChoiceStyle::Blended, unlisted);
        // The same row but for its height: a panel line against a box with a point round it.
        assert_eq!(blended.0.width(), boxed.0.width());
        assert_eq!(blended.0.height(), panel_line());
        assert_eq!(
            (blended.1, blended.2, blended.3),
            (boxed.1, boxed.2, boxed.3)
        );
    }
}

/// The hover band runs across the whole row, under its marker too, and a key's focus ring
/// goes round the row as a slider row's does, as far out as the band.
#[test]
fn a_blended_row_s_band_and_focus_ring_are_the_row_s() {
    let t = Theme::default();
    let mut harness = blended(400.0, false);
    harness.run();
    let row = harness
        .get_by_role_and_label(Role::ComboBox, ILLUMINANT)
        .rect();
    let around = row.expand2(egui::vec2(t.spacing.xs, 0.0));
    let banded = |harness: &Harness<'_, Dropdown>| {
        harness.output().shapes.iter().any(|clipped| {
            matches!(&clipped.shape, egui::Shape::Rect(shape)
                if shape.fill == t.colors.hover && shape.rect == around)
        })
    };
    let ringed = |harness: &Harness<'_, Dropdown>| {
        harness.output().shapes.iter().any(|clipped| {
            matches!(&clipped.shape, egui::Shape::Rect(shape)
                if shape.stroke.color == t.colors.focus_ring && shape.rect == around)
        })
    };
    assert!(!banded(&harness));
    harness.hover_at(harness.get_by_label(KEEP).rect().center());
    harness.run();
    assert!(banded(&harness), "the band under the marker");
    harness.hover_at(pos2(row.center().x, row.bottom() + 100.0));
    harness
        .get_by_role_and_label(Role::ComboBox, ILLUMINANT)
        .focus();
    harness.run();
    assert!(!banded(&harness));
    assert!(ringed(&harness), "the ring round the row");
}

#[test]
fn swatches_report_the_chosen_colour() {
    let mut chosen = None;
    let items = [
        Swatch {
            name: "Red",
            fill: SwatchFill::Color(Color32::RED),
        },
        Swatch {
            name: "Blue",
            fill: SwatchFill::Color(Color32::BLUE),
        },
        Swatch {
            name: "Custom",
            fill: SwatchFill::Spectrum,
        },
    ];
    let mut harness = Harness::builder()
        .with_size(egui::vec2(300.0, 80.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui(|ui| {
            if let Some(i) = Swatches::new("Colour range", &items, 0).show(ui) {
                chosen = Some(i);
            }
        });
    harness.run();
    harness.get_by_label("Custom").click();
    harness.run();
    drop(harness);
    assert_eq!(chosen, Some(2));
}

#[test]
fn a_disabled_icon_button_does_not_click() {
    let (mut undo, mut redo) = (0, 0);
    let mut harness = Harness::builder()
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui(|ui| {
            if IconButton::new(Icon::Undo, "Undo").show(ui).clicked() {
                undo += 1;
            }
            if IconButton::new(Icon::Redo, "Redo")
                .enabled(false)
                .show(ui)
                .clicked()
            {
                redo += 1;
            }
        });
    harness.run();
    harness.get_by_label("Undo").click();
    harness.run();
    harness.get_by_label("Redo").click();
    harness.run();
    drop(harness);
    assert_eq!((undo, redo), (1, 0));
}

/// Two collapsible groups as a module shows them, each holding a slider, and a checkbox
/// row after them; what is open, how often each was toggled, and how many controls in
/// each are set, applied as the app would.
#[derive(Default)]
struct Groups {
    open: [bool; 2],
    toggles: [usize; 2],
    set: [usize; 2],
}

fn groups(locale: Locale) -> Harness<'static, Groups> {
    Harness::builder()
        .with_size(egui::vec2(344.0, 220.0))
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark));
            apertrust_ui::set_locale(ctx, locale);
        })
        .build_ui_state(
            |ui, state: &mut Groups| {
                let t = apertrust_ui::theme(ui.ctx());
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, t.colors.panel);
                ui.spacing_mut().item_spacing.y = 0.0;
                egui::Frame::new().inner_margin(t.spacing.l).show(ui, |ui| {
                    let rows = [("Look", "Lift"), ("Margins", "Left margin")];
                    for (i, (title, control)) in rows.into_iter().enumerate() {
                        let toggled = CollapsibleGroup::new(title, state.open[i])
                            .set(state.set[i])
                            .show(ui, |ui| {
                                SliderRow::new(control, 0.0, -1.0..=1.0).show(ui);
                            });
                        if toggled {
                            state.open[i] = !state.open[i];
                            state.toggles[i] += 1;
                        }
                    }
                    ToggleRow::new("Use camera bias", true).show(ui);
                });
            },
            Groups::default(),
        )
}

#[test]
fn a_collapsible_group_opens_and_closes_anywhere_on_its_row() {
    let mut harness = groups(Locale::EnGb);
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(Role::Slider, "Lift")
            .is_none()
    );
    let row = harness.get_by_role_and_label(Role::Button, "Look").rect();
    // On its title, on its hairline and on its chevron.
    let places = [row.left() + 4.0, row.center().x, row.right() - 4.0];
    for (n, x) in places.into_iter().enumerate() {
        click_at(&mut harness, pos2(x, row.center().y));
        assert_eq!(harness.state().toggles, [n + 1, 0]);
        let open = harness
            .query_by_role_and_label(Role::Slider, "Lift")
            .is_some();
        assert_eq!(open, n % 2 == 0, "after {} clicks", n + 1);
    }
}

/// A collapsible group's row is a panel line (24) with 4 points above it, and 4 follow its
/// last row while it is open; a toggle row after it is a panel line too.
#[test]
fn a_collapsible_group_is_four_above_a_panel_line_and_four_after_its_rows() {
    let t = Theme::default();
    let mut harness = groups(Locale::EnGb);
    harness.state_mut().open = [true, false];
    harness.run();
    let look = harness.get_by_role_and_label(Role::Button, "Look").rect();
    let margins = harness
        .get_by_role_and_label(Role::Button, "Margins")
        .rect();
    assert_eq!(look.height(), t.sizes.panel_line);
    assert_eq!(margins.height(), t.sizes.panel_line);
    // The open group's slider row, then 4 after it and 4 above the next group.
    let between = margins.top() - look.bottom();
    assert_eq!(between, t.sizes.slider_row + 2.0 * t.spacing.xs);
    // A closed group has no end gap: the toggle row, 24 high, follows at once.
    let toggle = harness
        .get_by_role_and_label(Role::CheckBox, "Use camera bias")
        .rect();
    assert!(
        (toggle.center().y - (margins.bottom() + t.sizes.panel_line / 2.0)).abs() < 0.01,
        "{margins:?} then {toggle:?}"
    );
}

/// A group with nothing behind it yet, as AgX's Show curve before its graph: disabled to
/// screen readers, closed whatever the caller keeps, its rows never drawn and no click
/// reported.
#[test]
fn a_disabled_collapsible_group_stays_closed_and_reports_nothing() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(344.0, 120.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, toggles: &mut usize| {
                let toggled = CollapsibleGroup::new("Show curve", true)
                    .disabled("Coming in a later version")
                    .show(ui, |ui| {
                        SliderRow::new("Contrast", 0.0, -1.0..=1.0).show(ui);
                    });
                *toggles += usize::from(toggled);
            },
            0,
        );
    harness.run();
    let group = harness.get_by_role_and_label(Role::Button, "Show curve");
    assert!(group.accesskit_node().is_disabled());
    assert_eq!(group.accesskit_node().data().is_expanded(), Some(false));
    group.click();
    harness.run();
    assert_eq!(*harness.state(), 0);
    assert!(
        harness
            .query_by_role_and_label(Role::Slider, "Contrast")
            .is_none()
    );
}

#[test]
fn enter_or_space_toggles_a_focused_collapsible_group() {
    let mut harness = groups(Locale::EnGb);
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Look").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().open[0]);
    harness.key_press(Key::Space);
    harness.run();
    assert!(!harness.state().open[0]);
    assert_eq!(harness.state().toggles, [2, 0]);
}

#[test]
fn tab_skips_a_closed_group_s_rows() {
    let mut harness = groups(Locale::EnGb);
    harness.run();
    let tab_from_look = |harness: &mut Harness<'_, Groups>| {
        harness.get_by_role_and_label(Role::Button, "Look").focus();
        harness.run();
        let mut seen = Vec::new();
        for _ in 0..2 {
            harness.key_press(Key::Tab);
            harness.run();
            seen.push(focused(harness));
        }
        seen
    };
    // Closed, one stop each: Look, Margins, then the checkbox after them.
    assert_eq!(
        tab_from_look(&mut harness),
        [
            (Role::Button, "Margins".to_owned()),
            (Role::CheckBox, "Use camera bias".to_owned()),
        ]
    );
    // Open, Look's slider comes next, starting with its marker.
    harness.state_mut().open[0] = true;
    harness.run();
    assert_eq!(
        tab_from_look(&mut harness),
        [
            (Role::Button, "Keep Lift".to_owned()),
            (Role::Slider, "Lift".to_owned()),
        ]
    );
}

#[test]
fn a_collapsible_group_tells_screen_readers_its_state_and_how_many_are_set() {
    let mut harness = groups(Locale::EnGb);
    harness.state_mut().set = [2, 0];
    harness.run();
    let look = harness.get_by_role_and_label(Role::Button, "Look");
    assert_eq!(look.accesskit_node().data().is_expanded(), Some(false));
    assert_eq!(
        look.accesskit_node().description().as_deref(),
        Some("2 set")
    );
    let margins = harness.get_by_role_and_label(Role::Button, "Margins");
    assert_eq!(margins.accesskit_node().description(), None);
    assert!(harness.query_by_role(Role::Group).is_none());

    // Open, its rows are the children of a group named by its title.
    harness.state_mut().open[0] = true;
    harness.run();
    let look = harness.get_by_role_and_label(Role::Button, "Look");
    assert_eq!(look.accesskit_node().data().is_expanded(), Some(true));
    assert_eq!(
        look.accesskit_node().description().as_deref(),
        Some("2 set")
    );
    let group = harness.get_by_role_and_label(Role::Group, "Look");
    assert!(
        group
            .query_by_role_and_label(Role::Slider, "Lift")
            .is_some()
    );
    assert!(
        harness
            .query_by_role_and_label(Role::Slider, "Left margin")
            .is_none()
    );
}

#[test]
fn a_collapsible_group_s_count_is_said_in_every_language() {
    for locale in Locale::ALL {
        let mut harness = groups(locale);
        harness.state_mut().set = [3, 0];
        harness.run();
        let mut args = apertrust_ui::i18n::Args::new();
        args.set("count", 3);
        let said = Translator::new(locale).text_with("collapsible-group-set", Some(&args));
        assert!(
            said.contains('3') && said != "collapsible-group-set",
            "{said}"
        );
        let look = harness.get_by_role_and_label(Role::Button, "Look");
        assert_eq!(
            look.accesskit_node().description(),
            Some(said),
            "{}",
            locale.id()
        );
    }
}

/// The dots drawn in the row of the group called `title`: filled circles in the text
/// colour.
fn dots(harness: &Harness<'_, Groups>, title: &str) -> usize {
    let row = harness.get_by_role_and_label(Role::Button, title).rect();
    let text = Theme::builtin(Builtin::Dark).colors.text;
    harness
        .output()
        .shapes
        .iter()
        .filter(|clipped| match &clipped.shape {
            egui::Shape::Circle(dot) => dot.fill == text && row.contains(dot.center),
            _ => false,
        })
        .count()
}

#[test]
fn a_dot_shows_while_a_group_with_controls_set_is_closed() {
    let mut harness = groups(Locale::EnGb);
    harness.state_mut().set = [1, 0];
    harness.run();
    assert_eq!((dots(&harness, "Look"), dots(&harness, "Margins")), (1, 0));
    harness.state_mut().open[0] = true;
    harness.run();
    assert_eq!(dots(&harness, "Look"), 0);
}

/// "Manual vignette correction" as the pseudo-locale writes it, at the side panel's
/// narrowest content width, 216 points: closed, with the dot and without, the title is cut
/// with … where the shortest hairline starts, and screen readers still hear it whole.
#[test]
fn a_long_title_is_cut_before_the_shortest_hairline_at_the_narrowest_side_panel() {
    const TITLE: &str = "[Máñüáĺ víğñéťťé çöŕŕéçťíöñ~~~~~~~~~~~]";
    let t = Theme::builtin(Builtin::Dark);
    let content = narrowest_content(&t);
    for set in [0, 1] {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(360.0, 80.0))
            .with_context_setup(|ctx| {
                apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark));
                apertrust_ui::set_locale(ctx, Locale::Pseudo);
            })
            .build_ui(move |ui| {
                ui.allocate_ui(egui::vec2(content, 40.0), |ui| {
                    CollapsibleGroup::new(TITLE, false)
                        .set(set)
                        .show(ui, |_| {});
                });
            });
        harness.run();
        let row = harness.get_by_role_and_label(Role::Button, TITLE).rect();
        assert_eq!(row.width(), 216.0);
        let shapes = &harness.output().shapes;
        let title = shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.galley.text() == TITLE.to_uppercase() => {
                    Some(text.clone())
                }
                _ => None,
            })
            .expect("the title is drawn");
        assert!(title.galley.elided, "with {set} set");
        let end = title.visual_bounding_rect().right();
        let chevron = row.right() - Icon::ChevronEnd.size();
        let (gap, shortest) = (t.spacing.s, t.spacing.l);
        assert!(end <= chevron - gap - shortest, "{end} with {set} set");
        // The hairline starts after the title, and after the dot when there is one, and
        // keeps its shortest length before the chevron.
        let line = shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::LineSegment { points, stroke } if stroke.color == t.colors.divider => {
                    Some(*points)
                }
                _ => None,
            })
            .expect("the hairline is drawn");
        assert!(line[0].x > end + gap - 0.5, "{line:?} after {end}");
        assert!(line[1].x - line[0].x >= shortest - 0.5, "{line:?}");
        assert!(line[1].x <= chevron - gap + 0.5, "{line:?}");
    }
}

/// A closed group with a control set, focused through AccessKit's focus request, which
/// `FocusVisible` treats as a key, so its ring shows, and an open one under the pointer:
/// the dot, the focus ring and the hover band.
#[test]
fn collapsible_groups_focused_and_under_the_pointer() {
    let mut harness = groups(Locale::EnGb);
    harness.state_mut().set = [1, 0];
    harness.state_mut().open[1] = true;
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Look").focus();
    harness.run();
    let margins = harness
        .get_by_role_and_label(Role::Button, "Margins")
        .rect();
    harness.hover_at(margins.center());
    harness.run();
    let hover = Theme::builtin(Builtin::Dark).colors.hover;
    let band = harness
        .output()
        .shapes
        .iter()
        .any(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape) => shape.fill == hover && shape.rect.contains_rect(margins),
            _ => false,
        });
    assert!(band, "no hover band across Margins");
    harness.snapshot("collapsible-groups-focused-hovered");
}
