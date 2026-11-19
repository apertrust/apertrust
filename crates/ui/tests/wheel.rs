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

//! The scroll wheel over a slider and a choice row in a scrolling panel, with Mouse wheel
//! scrolls the Tools panel on and off, on the Mac and elsewhere (`docs/gui.md`, The scroll
//! wheel over a row).

use apertrust_ui::Theme;
use apertrust_ui::components::{
    Choice, ChoiceAction, ChoiceStyle, SliderAction, SliderRow, Wheel, set_wheel_scrolls_panel,
};
use egui::accesskit::Role;
use egui::os::OperatingSystem;
use egui::{Event, Key, Modifiers, MouseWheelSource, MouseWheelUnit, Pos2, TouchPhase, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

const OPTIONS: [&str; 4] = ["Alpha", "Beta", "Gamma", "Delta"];

/// The rows' values and what they reported.
#[derive(Default)]
struct Rows {
    exposure: f64,
    mode: usize,
    locked: f64,
    quality: f64,
    plain: f64,
    reported: Vec<SliderAction>,
}

/// A panel scrolling over a gap, then Exposure (−4 to 4 by 0.1, two decimals), Mode (its
/// third option disabled), Locked (disabled), Quality (the wheel only while focused, as
/// Export's) and Plain (no wheel, as the Settings dialog's), then a long gap.
fn panel(os: OperatingSystem, scrolls: bool) -> Harness<'static, Rows> {
    Harness::builder()
        .with_size(vec2(400.0, 300.0))
        .with_os(os)
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::default());
            set_wheel_scrolls_panel(ctx, scrolls);
        })
        .build_ui_state(
            |ui, rows: &mut Rows| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(80.0);
                    let slider = |name, value: f64| {
                        SliderRow::new(name, value, -4.0..=4.0)
                            .step(0.1)
                            .decimals(2)
                    };
                    let mut set = |action: Option<SliderAction>, value: &mut f64| {
                        if let Some(action) = action {
                            if let SliderAction::Set { value: to, .. } = action {
                                *value = to;
                            }
                            rows.reported.push(action);
                        }
                    };
                    let action = slider("Exposure", rows.exposure)
                        .wheel(Wheel::Pointer)
                        .show(ui);
                    set(action, &mut rows.exposure);
                    let chosen = Choice::new("Mode", &OPTIONS, rows.mode)
                        .style(ChoiceStyle::Blended)
                        .disabled_options(&[2], "Later")
                        .wheel(Wheel::Pointer)
                        .show(ui);
                    if let Some(ChoiceAction::Set(i)) = chosen {
                        rows.mode = i;
                    }
                    let action = slider("Locked", rows.locked)
                        .enabled(false)
                        .wheel(Wheel::Pointer)
                        .show(ui);
                    set(action, &mut rows.locked);
                    let action = slider("Quality", rows.quality)
                        .wheel(Wheel::Focused)
                        .show(ui);
                    set(action, &mut rows.quality);
                    let action = slider("Plain", rows.plain).show(ui);
                    set(action, &mut rows.plain);
                    ui.add_space(1000.0);
                });
            },
            Rows::default(),
        )
}

/// The modifier that makes a step a tenth, and that lets the wheel change a row while it
/// scrolls the panel: Option on the Mac, Ctrl elsewhere.
fn finer(os: OperatingSystem) -> Modifiers {
    if os == OperatingSystem::Mac {
        Modifiers::ALT
    } else {
        Modifiers::CTRL | Modifiers::COMMAND
    }
}

const PLATFORMS: [OperatingSystem; 2] = [OperatingSystem::Mac, OperatingSystem::Nix];

/// The top of the row `name` with `role`: it moves up as the panel scrolls.
fn top(harness: &Harness<'_, Rows>, role: Role, name: &str) -> f32 {
    harness.get_by_role_and_label(role, name).rect().top()
}

/// One event of the wheel at the pointer, `delta` in `unit`s, up positive, with
/// `modifiers` held, and the frames egui spreads its scroll over.
fn turn(
    harness: &mut Harness<'_, Rows>,
    unit: MouseWheelUnit,
    delta: f32,
    modifiers: Modifiers,
    source: MouseWheelSource,
) {
    let event = Event::MouseWheel {
        unit,
        delta: vec2(0.0, delta),
        phase: TouchPhase::Move,
        source,
        modifiers,
    };
    harness.event_modifiers(event, modifiers);
    harness.run_steps(12);
}

/// `notches` of a mouse wheel over the row `name`, up positive.
fn wheel(
    harness: &mut Harness<'_, Rows>,
    role: Role,
    name: &str,
    notches: f32,
    modifiers: Modifiers,
) {
    harness.get_by_role_and_label(role, name).hover();
    harness.run();
    let (line, wheel) = (MouseWheelUnit::Line, MouseWheelSource::Wheel);
    turn(harness, line, notches, modifiers, wheel);
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn on_a_plain_scroll_scrolls_the_panel_and_the_finer_modifier_changes_the_row() {
    for os in PLATFORMS {
        let mut harness = panel(os, true);
        harness.run();
        let at = top(&harness, Role::Slider, "Exposure");
        // A plain scroll down over the slider scrolls the panel and leaves the slider.
        wheel(
            &mut harness,
            Role::Slider,
            "Exposure",
            -1.0,
            Modifiers::NONE,
        );
        assert_eq!(harness.state().exposure, 0.0, "{os:?}");
        let scrolled = top(&harness, Role::Slider, "Exposure");
        assert!(scrolled < at, "{os:?}: not scrolled, {scrolled} from {at}");
        // With the finer modifier a notch is a step, Shift with it ten, each done as a
        // key's; the panel stays.
        let fine = finer(os);
        wheel(&mut harness, Role::Slider, "Exposure", 1.0, fine);
        assert!(near(harness.state().exposure, 0.1), "{os:?}");
        wheel(
            &mut harness,
            Role::Slider,
            "Exposure",
            1.0,
            fine | Modifiers::SHIFT,
        );
        assert!(near(harness.state().exposure, 1.1), "{os:?}");
        wheel(&mut harness, Role::Slider, "Exposure", -2.0, fine);
        assert!(near(harness.state().exposure, 0.9), "{os:?}");
        assert!(matches!(
            harness.state().reported.last(),
            Some(SliderAction::Set { done: true, .. })
        ));
        assert_eq!(top(&harness, Role::Slider, "Exposure"), scrolled, "{os:?}");
        // Over the choice, down steps to the next option, past the disabled one and no
        // further than the last, whatever else is held.
        wheel(&mut harness, Role::ComboBox, "Mode", -1.0, Modifiers::NONE);
        assert_eq!(harness.state().mode, 0, "{os:?}");
        wheel(&mut harness, Role::ComboBox, "Mode", -1.0, fine);
        assert_eq!(harness.state().mode, 1, "{os:?}");
        wheel(
            &mut harness,
            Role::ComboBox,
            "Mode",
            -1.0,
            fine | Modifiers::SHIFT,
        );
        assert_eq!(harness.state().mode, 3, "{os:?}");
        wheel(&mut harness, Role::ComboBox, "Mode", -1.0, fine);
        assert_eq!(harness.state().mode, 3, "{os:?}");
        wheel(&mut harness, Role::ComboBox, "Mode", 3.0, fine);
        assert_eq!(harness.state().mode, 0, "{os:?}");
    }
}

#[test]
fn off_the_wheel_changes_the_row_under_the_pointer_as_darktable_s() {
    for os in PLATFORMS {
        let mut harness = panel(os, false);
        harness.run();
        let at = top(&harness, Role::Slider, "Exposure");
        // A notch a step, Shift ten, the finer modifier a tenth; the panel stays.
        wheel(&mut harness, Role::Slider, "Exposure", 1.0, Modifiers::NONE);
        assert!(near(harness.state().exposure, 0.1), "{os:?}");
        wheel(
            &mut harness,
            Role::Slider,
            "Exposure",
            1.0,
            Modifiers::SHIFT,
        );
        assert!(near(harness.state().exposure, 1.1), "{os:?}");
        wheel(&mut harness, Role::Slider, "Exposure", -1.0, finer(os));
        assert!(near(harness.state().exposure, 1.09), "{os:?}");
        assert_eq!(top(&harness, Role::Slider, "Exposure"), at, "{os:?}");
        // The choice steps its options plain.
        wheel(&mut harness, Role::ComboBox, "Mode", -1.0, Modifiers::NONE);
        assert_eq!(harness.state().mode, 1, "{os:?}");
        // Anywhere else in the panel the wheel scrolls it.
        harness.hover_at(Pos2::new(200.0, 40.0));
        harness.run();
        let (line, source) = (MouseWheelUnit::Line, MouseWheelSource::Wheel);
        turn(&mut harness, line, -1.0, Modifiers::NONE, source);
        assert!(top(&harness, Role::Slider, "Exposure") < at, "{os:?}");
        assert!(near(harness.state().exposure, 1.09), "{os:?}");
    }
}

#[test]
fn a_disabled_row_and_one_given_no_wheel_take_none_and_the_panel_scrolls() {
    for name in ["Locked", "Plain"] {
        let mut harness = panel(OperatingSystem::Nix, false);
        harness.run();
        let at = top(&harness, Role::Slider, name);
        wheel(&mut harness, Role::Slider, name, -1.0, Modifiers::NONE);
        assert!(harness.state().reported.is_empty(), "{name}");
        assert!(
            top(&harness, Role::Slider, name) < at,
            "{name}: not scrolled"
        );
    }
}

#[test]
fn a_row_that_takes_the_wheel_only_focused_ignores_it_until_focused() {
    for (scrolls, held) in [(false, Modifiers::NONE), (true, Modifiers::ALT)] {
        let os = OperatingSystem::Mac;
        let mut harness = panel(os, scrolls);
        harness.run();
        wheel(&mut harness, Role::Slider, "Quality", 1.0, held);
        assert_eq!(harness.state().quality, 0.0, "{scrolls}");
        harness
            .get_by_role_and_label(Role::Slider, "Quality")
            .focus();
        harness.run();
        wheel(&mut harness, Role::Slider, "Quality", 1.0, held);
        assert!(near(harness.state().quality, 0.1), "{scrolls}");
    }
}

#[test]
fn a_trackpad_s_points_add_up_to_notches_and_its_momentum_changes_nothing() {
    let mut harness = panel(OperatingSystem::Mac, false);
    harness.run();
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .hover();
    harness.run();
    // Two points an event: darktable's 0.06 × 2^0.7, about a tenth of a notch each.
    let (points, trackpad) = (MouseWheelUnit::Point, MouseWheelSource::Trackpad);
    for _ in 0..10 {
        turn(&mut harness, points, 2.0, Modifiers::NONE, trackpad);
    }
    assert_eq!(harness.state().exposure, 0.0);
    turn(&mut harness, points, 2.0, Modifiers::NONE, trackpad);
    assert!(near(harness.state().exposure, 0.1));
    // The system's momentum after the fingers lift is not the person's.
    let momentum = MouseWheelSource::Momentum;
    turn(&mut harness, points, 200.0, Modifiers::NONE, momentum);
    assert!(near(harness.state().exposure, 0.1));
}

/// Two trackpad gestures, each less than a notch, change nothing: what is left of the
/// first is not carried into the second.
#[test]
fn a_part_of_a_notch_is_not_carried_into_the_next_gesture() {
    let mut harness = panel(OperatingSystem::Mac, false);
    harness.run();
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .hover();
    harness.run();
    // 25 points, about 0.57 of a notch each by darktable's conversion.
    for _ in 0..2 {
        let event = Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta: vec2(0.0, 25.0),
            phase: TouchPhase::Start,
            source: MouseWheelSource::Trackpad,
            modifiers: Modifiers::NONE,
        };
        harness.event(event);
        harness.run_steps(12);
    }
    assert_eq!(harness.state().exposure, 0.0);
}

/// A plain scroll right after a notch taken with the modifier, while egui still spreads
/// it, scrolls the panel: the row holds the panel still only while the modifier is held.
#[test]
fn a_plain_scroll_after_a_taken_notch_scrolls_the_panel() {
    let os = OperatingSystem::Nix;
    let mut harness = panel(os, true);
    harness.run();
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .hover();
    harness.run();
    let notch = |modifiers| Event::MouseWheel {
        unit: MouseWheelUnit::Line,
        delta: vec2(0.0, -1.0),
        phase: TouchPhase::Move,
        source: MouseWheelSource::Wheel,
        modifiers,
    };
    harness.event_modifiers(notch(finer(os)), finer(os));
    harness.step();
    assert!(near(harness.state().exposure, -0.1));
    let at = top(&harness, Role::Slider, "Exposure");
    harness.event(notch(Modifiers::NONE));
    harness.run_steps(12);
    assert!(near(harness.state().exposure, -0.1));
    let now = top(&harness, Role::Slider, "Exposure");
    assert!(now < at, "the panel did not scroll: {now} from {at}");
}

/// After a notch taken with the modifier, a new scroll begun away from the row, while egui
/// still spreads the notch, is the panel's.
#[test]
fn a_new_scroll_away_from_the_row_scrolls_the_panel() {
    let os = OperatingSystem::Nix;
    let mut harness = panel(os, true);
    harness.run();
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .hover();
    harness.run();
    harness.input_mut().events.push(notch(-1.0, finer(os)));
    harness.step();
    assert!(near(harness.state().exposure, -0.1));
    let at = top(&harness, Role::Slider, "Exposure");
    harness.hover_at(Pos2::new(200.0, 40.0));
    harness.step();
    harness.event(notch(-1.0, Modifiers::NONE));
    harness.run_steps(12);
    let now = top(&harness, Role::Slider, "Exposure");
    assert!(now < at, "the panel did not scroll: {now} from {at}");
}

/// A trackpad gesture that took a notch keeps the panel still to its end, though the
/// pointer leaves the row while the fingers still move.
#[test]
fn a_trackpad_gesture_keeps_the_panel_still_after_the_pointer_leaves_the_row() {
    let os = OperatingSystem::Mac;
    let mut harness = panel(os, true);
    harness.run();
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .hover();
    harness.run();
    let swipe = |phase, modifiers| Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: vec2(0.0, -60.0),
        phase,
        source: MouseWheelSource::Trackpad,
        modifiers,
    };
    harness.event_modifiers(swipe(TouchPhase::Start, finer(os)), finer(os));
    harness.step();
    let value = harness.state().exposure;
    assert!(value < 0.0, "{value}");
    let at = top(&harness, Role::Slider, "Exposure");
    harness.hover_at(Pos2::new(200.0, 40.0));
    harness.step();
    harness.event(swipe(TouchPhase::Move, Modifiers::NONE));
    harness.run_steps(12);
    assert_eq!(top(&harness, Role::Slider, "Exposure"), at);
}

/// What was left of a notch goes at a new gesture's start, though that gesture starts
/// without the modifier and the modifier comes later in it.
#[test]
fn a_gesture_started_plain_starts_from_nothing() {
    let os = OperatingSystem::Mac;
    let mut harness = panel(os, true);
    harness.run();
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .hover();
    harness.run();
    let swipe = |phase, modifiers| Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: vec2(0.0, 25.0),
        phase,
        source: MouseWheelSource::Trackpad,
        modifiers,
    };
    // About 0.57 of a notch with Option, then a gesture begun plain and continued with it.
    harness.event_modifiers(swipe(TouchPhase::Start, finer(os)), finer(os));
    harness.run_steps(12);
    harness.event(swipe(TouchPhase::Start, Modifiers::NONE));
    harness.step();
    harness.event_modifiers(swipe(TouchPhase::Move, finer(os)), finer(os));
    harness.run_steps(12);
    assert_eq!(harness.state().exposure, 0.0);
}

/// A notch with the modifier and a plain one in one frame: the first steps the row, the
/// second still scrolls the panel.
#[test]
fn a_frame_with_a_taken_notch_and_a_plain_one_does_both() {
    let os = OperatingSystem::Nix;
    let mut harness = panel(os, true);
    harness.run();
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .hover();
    harness.run();
    let at = top(&harness, Role::Slider, "Exposure");
    let events = [notch(-1.0, finer(os)), notch(-1.0, Modifiers::NONE)];
    harness.input_mut().events.extend(events);
    harness.run_steps(12);
    assert!(near(harness.state().exposure, -0.1));
    let now = top(&harness, Role::Slider, "Exposure");
    assert!(now < at, "the panel did not scroll: {now} from {at}");
}

/// Without its code, each row would see a notch as nothing and the panel would scroll.
#[test]
fn a_notch_taken_leaves_the_panel_still_for_the_rest_of_its_scroll() {
    let os = OperatingSystem::Mac;
    let mut harness = panel(os, true);
    harness.run();
    let at = top(&harness, Role::ComboBox, "Mode");
    // Option let go at once: the frames egui spreads the notch over scroll nothing.
    wheel(&mut harness, Role::ComboBox, "Mode", -3.0, finer(os));
    assert_eq!(harness.state().mode, 3);
    assert_eq!(top(&harness, Role::ComboBox, "Mode"), at);
}

/// What is under the pointer decides: Ctrl+scroll over the photo, Cmd+scroll on the Mac,
/// whichever way the setting is, reaches what is drawn there, here a stand-in reading
/// egui's zoom, and changes no row. The photo's own zoom is planned.
#[test]
fn ctrl_scroll_over_the_photo_reaches_it_and_changes_no_row() {
    #[derive(Default)]
    struct Window {
        zoom: f32,
        exposure: f64,
    }
    for (os, scrolls) in [
        (OperatingSystem::Nix, true),
        (OperatingSystem::Nix, false),
        (OperatingSystem::Mac, true),
        (OperatingSystem::Mac, false),
    ] {
        let mut harness = Harness::builder()
            .with_size(vec2(600.0, 200.0))
            .with_os(os)
            .with_context_setup(move |ctx| {
                apertrust_ui::install(ctx, Theme::default());
                set_wheel_scrolls_panel(ctx, scrolls);
            })
            .build_ui_state(
                |ui, window: &mut Window| {
                    ui.horizontal_top(|ui| {
                        let size = vec2(300.0, 200.0);
                        let (_, photo) = ui.allocate_exact_size(size, egui::Sense::hover());
                        if photo.hovered() {
                            window.zoom += ui.input(|i| i.zoom_delta()).ln();
                        }
                        ui.vertical(|ui| {
                            let row = SliderRow::new("Exposure", window.exposure, -4.0..=4.0)
                                .step(0.1)
                                .decimals(2)
                                .wheel(Wheel::Pointer);
                            if let Some(SliderAction::Set { value, .. }) = row.show(ui) {
                                window.exposure = value;
                            }
                        });
                    });
                },
                Window::default(),
            );
        harness.hover_at(Pos2::new(150.0, 100.0));
        harness.run();
        let event = Event::MouseWheel {
            unit: MouseWheelUnit::Line,
            delta: vec2(0.0, 1.0),
            phase: TouchPhase::Move,
            source: MouseWheelSource::Wheel,
            modifiers: Modifiers::COMMAND,
        };
        harness.event_modifiers(event, Modifiers::COMMAND);
        harness.run_steps(12);
        let window = harness.state();
        assert!(window.zoom > 0.0, "{os:?} {scrolls}: no zoom");
        assert_eq!(window.exposure, 0.0, "{os:?} {scrolls}");
    }
}

/// A wheel event with `modifiers`, `notches` lines up, from a mouse.
fn notch(notches: f32, modifiers: Modifiers) -> Event {
    Event::MouseWheel {
        unit: MouseWheelUnit::Line,
        delta: vec2(0.0, notches),
        phase: TouchPhase::Move,
        source: MouseWheelSource::Wheel,
        modifiers,
    }
}

/// Two notches in one frame, a plain one and one with Shift, are a step and ten, each by
/// the modifiers held as it came, whichever comes first.
#[test]
fn each_notch_in_a_frame_steps_by_its_own_modifiers() {
    for shift_first in [false, true] {
        let mut harness = panel(OperatingSystem::Nix, false);
        harness.run();
        harness
            .get_by_role_and_label(Role::Slider, "Exposure")
            .hover();
        harness.run();
        let mut events = vec![notch(1.0, Modifiers::NONE), notch(1.0, Modifiers::SHIFT)];
        if shift_first {
            events.reverse();
        }
        harness.input_mut().events.extend(events);
        harness.run_steps(12);
        let value = harness.state().exposure;
        assert!(near(value, 1.1), "{shift_first}: {value}");
    }
}

/// A slider by 0.01 with two decimals, the wheel over it changing it plain, its value.
fn fine_slider() -> Harness<'static, f64> {
    Harness::builder()
        .with_size(vec2(400.0, 120.0))
        .with_os(OperatingSystem::Nix)
        .with_context_setup(|ctx| {
            apertrust_ui::install(ctx, Theme::default());
            set_wheel_scrolls_panel(ctx, false);
        })
        .build_ui_state(
            |ui, value: &mut f64| {
                let action = SliderRow::new("Fine", *value, -1.0..=1.0)
                    .step(0.01)
                    .decimals(2)
                    .wheel(Wheel::Pointer)
                    .show(ui);
                if let Some(SliderAction::Set { value: to, .. }) = action {
                    *value = to;
                }
            },
            0.0,
        )
}

/// Fine notches whose tenth of a step is below the last decimal move the value by that
/// decimal each, as many in one frame as in frames of their own.
#[test]
fn fine_notches_in_one_frame_move_as_far_as_in_frames_of_their_own() {
    let fine = |harness: &mut Harness<'_, f64>, together: bool| {
        harness.get_by_label("Fine").hover();
        harness.run();
        let notches = [0; 3].map(|_| notch(1.0, Modifiers::CTRL));
        if together {
            harness.input_mut().events.extend(notches);
            harness.run_steps(4);
        } else {
            for event in notches {
                harness.input_mut().events.push(event);
                harness.run_steps(4);
            }
        }
        *harness.state()
    };
    let (mut together, mut apart) = (fine_slider(), fine_slider());
    together.run();
    apart.run();
    let (together, apart) = (fine(&mut together, true), fine(&mut apart, false));
    assert!(near(together, 0.03), "{together}");
    assert!(near(apart, 0.03), "{apart}");
}

/// At the range's end a notch past it stops there, and the notch back after it in the
/// same frame comes back one step, as it would in a frame of its own.
#[test]
fn notches_at_the_range_s_end_move_as_in_frames_of_their_own() {
    let turn = |together: bool| {
        let mut harness = fine_slider();
        *harness.state_mut() = 1.0;
        harness.run();
        harness.get_by_label("Fine").hover();
        harness.run();
        let notches = [notch(1.0, Modifiers::NONE), notch(-1.0, Modifiers::NONE)];
        if together {
            harness.input_mut().events.extend(notches);
            harness.run_steps(4);
        } else {
            for event in notches {
                harness.input_mut().events.push(event);
                harness.run_steps(4);
            }
        }
        *harness.state()
    };
    let (together, apart) = (turn(true), turn(false));
    assert!(near(apart, 0.99), "{apart}");
    assert!(near(together, 0.99), "{together}");
}

/// Notches that cancel in their count still move a slider by their steps, a plain one up
/// and a Shift one down nine steps down; on a choice at its first option, one up stops
/// there and one down then reaches the second, as in frames of their own.
#[test]
fn notches_that_cancel_in_count_still_step_a_slider_and_a_choice() {
    let mut harness = panel(OperatingSystem::Nix, false);
    harness.run();
    harness
        .get_by_role_and_label(Role::Slider, "Exposure")
        .hover();
    harness.run();
    let events = [notch(1.0, Modifiers::NONE), notch(-1.0, Modifiers::SHIFT)];
    harness.input_mut().events.extend(events);
    harness.run_steps(12);
    let value = harness.state().exposure;
    assert!(near(value, -0.9), "{value}");
    assert_eq!(harness.state().mode, 0, "Alpha first");
    harness.get_by_label("Mode").hover();
    harness.run();
    let events = [notch(1.0, Modifiers::NONE), notch(-1.0, Modifiers::NONE)];
    harness.input_mut().events.extend(events);
    harness.run_steps(12);
    assert_eq!(harness.state().mode, 1, "Beta");
}

/// An arrow and a notch in one frame both count: the value sent is the frame's last.
#[test]
fn an_arrow_and_a_notch_in_one_frame_are_one_value() {
    let mut harness = panel(OperatingSystem::Nix, false);
    harness.run();
    let slider = harness.get_by_role_and_label(Role::Slider, "Exposure");
    slider.focus();
    slider.hover();
    harness.run();
    let key = |pressed| Event::Key {
        key: Key::ArrowRight,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    let events = [key(true), notch(1.0, Modifiers::NONE), key(false)];
    harness.input_mut().events.extend(events);
    harness.run_steps(12);
    let value = harness.state().exposure;
    assert!(near(value, 0.2), "{value}");
}

/// The panel stays still for the rest of a scroll whose notch a row took, though the
/// pointer leave the row for the panel's empty part while egui still spreads it.
#[test]
fn a_notch_taken_keeps_the_panel_still_after_the_pointer_leaves_the_row() {
    let os = OperatingSystem::Mac;
    let mut harness = panel(os, true);
    harness.run();
    let at = top(&harness, Role::ComboBox, "Mode");
    harness
        .get_by_role_and_label(Role::ComboBox, "Mode")
        .hover();
    harness.run();
    harness.input_mut().events.push(notch(-3.0, finer(os)));
    harness.step();
    assert_eq!(harness.state().mode, 3);
    harness.hover_at(Pos2::new(200.0, 40.0));
    harness.run_steps(12);
    assert_eq!(top(&harness, Role::ComboBox, "Mode"), at);
}
