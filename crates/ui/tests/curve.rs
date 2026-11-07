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

//! Interaction tests for the curve, driven as a person or a screen reader would and found
//! through AccessKit (`docs/gui.md`, Testing), and photographs of it in use.

use apertrust_ui::components::{Axis, Curve, CurveAction};
use apertrust_ui::{Builtin, Locale, Theme};
use egui::accesskit::Role;
use egui::{Key, Modifiers, PointerButton, Pos2, Rect, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

/// How close in x points, and handles that are ends, may come.
const GAP: f32 = 0.0025;

/// A curve, its handles, and what it reported, applied as the app would.
struct Tone {
    points: Vec<(f32, f32)>,
    black: f32,
    white: f32,
    /// The handles are the curve's ends, rather than levels.
    ends: bool,
    log: bool,
    enabled: bool,
    histogram: Option<Vec<f32>>,
    actions: Vec<CurveAction>,
}

impl Tone {
    fn apply(&mut self, action: CurveAction) {
        match action {
            CurveAction::Move { index, x, y, .. } => self.points[index] = (x, y),
            CurveAction::Add { x, y } => {
                let at = self.points.partition_point(|p| p.0 < x);
                self.points.insert(at, (x, y));
            }
            CurveAction::Remove(index) => {
                self.points.remove(index);
            }
            CurveAction::BlackPoint { value, .. } => self.black = value,
            CurveAction::WhitePoint { value, .. } => self.white = value,
            CurveAction::Reset => {
                self.points = vec![(0.0, 0.0), (1.0, 1.0)];
                (self.black, self.white) = (0.0, 1.0);
            }
        }
        self.actions.push(action);
    }

    fn last(&self) -> Option<CurveAction> {
        self.actions.last().copied()
    }
}

/// A curve in `locale` and `theme`, with `setup` applied to its state before the first
/// frame.
fn tone_in(
    points: &[(f32, f32)],
    locale: Locale,
    theme: Builtin,
    setup: impl FnOnce(&mut Tone),
) -> Harness<'static, Tone> {
    let mut state = Tone {
        points: points.to_vec(),
        black: 0.0,
        white: 1.0,
        ends: false,
        log: false,
        enabled: true,
        histogram: None,
        actions: Vec::new(),
    };
    setup(&mut state);
    let mut harness = Harness::builder()
        .with_size(vec2(320.0, 360.0))
        // Frames a sixtieth of a second apart, so two clicks make a double-click (0.3 s).
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::builtin(theme));
            apertrust_ui::set_locale(ctx, locale);
        })
        .build_ui_state(
            |ui, state: &mut Tone| {
                let mut curve = Curve::new("Tone curve", &state.points)
                    .max_points(5)
                    .enabled(state.enabled);
                curve = if state.ends {
                    curve.ends(state.black, state.white)
                } else {
                    curve.levels(state.black, state.white)
                };
                if state.log {
                    curve = curve.axis(Axis::Log(32.0));
                }
                if let Some(bins) = &state.histogram {
                    curve = curve.histogram(bins);
                }
                if let Some(action) = curve.show(ui) {
                    state.apply(action);
                }
            },
            state,
        );
    harness.run();
    harness
}

fn tone(points: &[(f32, f32)]) -> Harness<'static, Tone> {
    tone_in(points, Locale::EnGb, Builtin::Dark, |_| {})
}

fn tone_with(points: &[(f32, f32)], setup: impl FnOnce(&mut Tone)) -> Harness<'static, Tone> {
    tone_in(points, Locale::EnGb, Builtin::Dark, setup)
}

/// The plot's square: its frame, less the inset that keeps the points inside it.
fn area(harness: &Harness<'_, Tone>) -> Rect {
    harness
        .get_by_role_and_label(Role::ListBox, "Tone curve")
        .rect()
        .shrink(Theme::builtin(Builtin::Dark).spacing.s)
}

fn at(area: Rect, x: f32, y: f32) -> Pos2 {
    pos2(
        area.left() + x * area.width(),
        area.bottom() - y * area.height(),
    )
}

/// Point `n`, counted from 1 as screen readers count it; its name also says where it is.
fn point<'a>(harness: &'a Harness<'_, Tone>, n: usize) -> Node<'a> {
    let prefix = format!("Point {n}:");
    harness.get_by(move |node| {
        node.role() == Role::ListBoxOption
            && node.label().is_some_and(|label| label.starts_with(&prefix))
    })
}

/// Within a point of the plot, about 1/300 of it.
fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.005
}

/// `b` is at least the gap above `a` in f32, and no more than a hair beyond it.
fn just_apart(a: f32, b: f32) -> bool {
    b - a >= GAP && b - a < GAP + 1e-6
}

fn click_at(harness: &mut Harness<'_, Tone>, p: Pos2) {
    harness.hover_at(p);
    harness.run();
    harness.drag_at(p);
    harness.run();
    harness.drop_at(p);
    harness.run();
}

fn drag(harness: &mut Harness<'_, Tone>, from: Pos2, to: Pos2) {
    harness.hover_at(from);
    harness.run();
    harness.drag_at(from);
    harness.run();
    harness.hover_at(to);
    harness.run();
    harness.drop_at(to);
    harness.run();
}

/// Lets half a second pass, so the next click starts afresh rather than making a double or
/// triple click with the last.
fn pause(harness: &mut Harness<'_, Tone>) {
    for _ in 0..30 {
        harness.step();
    }
}

/// Double-clicks the node `find` gives, and says whether egui took it as a double-click.
fn double_click(
    harness: &mut Harness<'static, Tone>,
    find: impl for<'h> Fn(&'h Harness<'static, Tone>) -> Node<'h>,
) -> bool {
    pause(harness);
    for _ in 0..2 {
        find(harness).click();
        // Each queued event runs in a frame of its own; the last is the release.
        harness.step();
    }
    let double = harness
        .ctx
        .input(|i| i.pointer.button_double_clicked(PointerButton::Primary));
    harness.run();
    double
}

fn focus(harness: &mut Harness<'_, Tone>, role: Role, name: &str) {
    harness.get_by_role_and_label(role, name).focus();
    harness.run();
}

fn press(harness: &mut Harness<'_, Tone>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

/// A key held down for `presses` presses: egui counts all but the first as repeats.
fn hold(harness: &mut Harness<'_, Tone>, key: Key, presses: usize) {
    for _ in 0..presses {
        harness.key_down(key);
        harness.run();
    }
    harness.key_up(key);
    harness.run();
}

const IDENTITY: [(f32, f32); 2] = [(0.0, 0.0), (1.0, 1.0)];
const THREE: [(f32, f32); 3] = [(0.0, 0.0), (0.5, 0.5), (1.0, 1.0)];

#[test]
fn clicking_empty_space_adds_a_point_there() {
    let mut harness = tone(&IDENTITY);
    let area = area(&harness);
    click_at(&mut harness, at(area, 0.5, 0.8));
    let Some(CurveAction::Add { x, y }) = harness.state().actions.first().copied() else {
        panic!("{:?}", harness.state().actions);
    };
    assert!(near(x, 0.5) && near(y, 0.8), "{x} {y}");
    assert_eq!(harness.state().points.len(), 3);
    assert_eq!(harness.state().points[1], (x, y));
}

#[test]
fn clicking_near_the_curve_adds_a_point_on_it() {
    let mut harness = tone(&IDENTITY);
    let area = area(&harness);
    click_at(&mut harness, at(area, 0.3, 0.3) + vec2(0.0, -4.0));
    let Some(CurveAction::Add { x, y }) = harness.state().actions.first().copied() else {
        panic!("{:?}", harness.state().actions);
    };
    // On the curve exactly: with straight lines, adding it changes nothing.
    assert!(near(x, 0.3), "{x}");
    assert!((y - x).abs() < 1e-6, "{x} {y}");
}

#[test]
fn dragging_a_point_moves_it_and_marks_the_release_done() {
    let mut harness = tone(&THREE);
    let area = area(&harness);
    let from = at(area, 0.5, 0.5);
    let to = at(area, 0.6, 0.7);
    harness.hover_at(from);
    harness.run();
    harness.drag_at(from);
    harness.run();
    harness.hover_at(to);
    harness.run();
    let Some(CurveAction::Move { index, x, y, done }) = harness.state().last() else {
        panic!("{:?}", harness.state().actions);
    };
    assert_eq!((index, done), (1, false));
    assert!(near(x, 0.6) && near(y, 0.7), "{x} {y}");
    harness.drop_at(to);
    harness.run();
    assert!(matches!(
        harness.state().last(),
        Some(CurveAction::Move {
            index: 1,
            done: true,
            ..
        })
    ));
    assert!(
        !harness
            .state()
            .actions
            .iter()
            .any(|a| matches!(a, CurveAction::Add { .. })),
        "{:?}",
        harness.state().actions
    );
}

#[test]
fn a_dragged_point_keeps_the_gap_from_its_neighbours_and_stays_in_the_plot() {
    let mut harness = tone(&[(0.0, 0.0), (0.4, 0.4), (0.6, 0.6), (1.0, 1.0)]);
    let area = area(&harness);
    drag(
        &mut harness,
        at(area, 0.4, 0.4),
        at(area, 0.9, 1.2) + vec2(0.0, -20.0),
    );
    let (x, y) = harness.state().points[1];
    assert!(just_apart(x, 0.6), "{x}");
    assert_eq!(y, 1.0);
    assert_eq!(harness.state().points.len(), 4);
}

#[test]
fn the_gap_holds_in_f32_wherever_the_neighbour_sits() {
    // `x + 0.0025` falls short of the gap in f32 for most x: push a point into its
    // neighbour, for neighbours all along the plot.
    for k in 0..24 {
        let next = 0.31 + k as f32 * 0.0271;
        let mut harness = tone(&[(0.0, 0.0), (0.3, 0.3), (next, 0.6), (1.0, 1.0)]);
        focus(&mut harness, Role::ListBox, "Tone curve");
        press(&mut harness, Modifiers::NONE, Key::PageDown);
        for _ in 0..8 {
            press(&mut harness, Modifiers::SHIFT, Key::ArrowRight);
        }
        let points = &harness.state().points;
        assert!(points[2].0 - points[1].0 >= GAP, "{next}: {points:?}");
    }
}

#[test]
fn pressing_empty_space_and_dragging_places_the_new_point() {
    let mut harness = tone(&IDENTITY);
    let area = area(&harness);
    drag(&mut harness, at(area, 0.3, 0.7), at(area, 0.35, 0.8));
    let actions = &harness.state().actions;
    assert!(matches!(actions[0], CurveAction::Add { .. }), "{actions:?}");
    assert!(
        matches!(
            actions.last(),
            Some(CurveAction::Move {
                index: 1,
                done: true,
                ..
            })
        ),
        "{actions:?}"
    );
    let (x, y) = harness.state().points[1];
    assert!(near(x, 0.35) && near(y, 0.8), "{x} {y}");
}

#[test]
fn a_double_click_removes_a_point_but_never_the_first_or_last() {
    let mut harness = tone(&THREE);
    assert!(double_click(&mut harness, |h| point(h, 2)));
    assert_eq!(harness.state().last(), Some(CurveAction::Remove(1)));
    assert_eq!(harness.state().points, IDENTITY.to_vec());
    for n in [1, 2] {
        assert!(double_click(&mut harness, |h| point(h, n)), "point {n}");
    }
    assert_eq!(harness.state().points, IDENTITY.to_vec());
    assert_eq!(harness.state().actions.len(), 1);
}

#[test]
fn keys_choose_a_point_and_move_it() {
    let mut harness = tone(&THREE);
    focus(&mut harness, Role::ListBox, "Tone curve");
    press(&mut harness, Modifiers::NONE, Key::PageDown);
    assert_eq!(
        point(&harness, 2).accesskit_node().is_selected(),
        Some(true)
    );
    press(&mut harness, Modifiers::NONE, Key::ArrowUp);
    let Some(CurveAction::Move {
        index: 1,
        x,
        y,
        done: true,
    }) = harness.state().last()
    else {
        panic!("{:?}", harness.state().actions);
    };
    assert!(x == 0.5 && (y - 0.51).abs() < 1e-6, "{x} {y}");
    press(&mut harness, Modifiers::SHIFT, Key::ArrowUp);
    assert!((harness.state().points[1].1 - 0.61).abs() < 1e-5);
    press(&mut harness, Modifiers::ALT, Key::ArrowRight);
    assert!((harness.state().points[1].0 - 0.501).abs() < 1e-5);
    press(&mut harness, Modifiers::NONE, Key::End);
    press(&mut harness, Modifiers::NONE, Key::ArrowDown);
    assert!(matches!(
        harness.state().last(),
        Some(CurveAction::Move { index: 2, .. })
    ));
    assert!((harness.state().points[2].1 - 0.99).abs() < 1e-5);
    press(&mut harness, Modifiers::NONE, Key::Home);
    press(&mut harness, Modifiers::NONE, Key::PageUp);
    press(&mut harness, Modifiers::NONE, Key::ArrowRight);
    assert!(matches!(
        harness.state().last(),
        Some(CurveAction::Move { index: 0, .. })
    ));
    assert!((harness.state().points[0].0 - 0.01).abs() < 1e-5);
}

#[test]
fn on_a_logarithmic_axis_an_arrow_moves_one_coordinate() {
    let mut harness = tone_with(&THREE, |tone| tone.log = true);
    focus(&mut harness, Role::ListBox, "Tone curve");
    press(&mut harness, Modifiers::NONE, Key::PageDown);
    press(&mut harness, Modifiers::NONE, Key::ArrowUp);
    let (x, y) = harness.state().points[1];
    assert_eq!(x, 0.5);
    assert!(y > 0.5, "{y}");
    press(&mut harness, Modifiers::NONE, Key::ArrowRight);
    assert_eq!(harness.state().points[1].1, y);
}

#[test]
fn a_held_key_moves_again_but_adds_and_removes_once() {
    let mut harness = tone(&IDENTITY);
    focus(&mut harness, Role::ListBox, "Tone curve");
    hold(&mut harness, Key::Insert, 4);
    assert_eq!(harness.state().points.len(), 3);
    // The new point is chosen, and moves with every repeat.
    hold(&mut harness, Key::ArrowUp, 4);
    assert!((harness.state().points[1].1 - 0.54).abs() < 1e-5);
    hold(&mut harness, Key::Delete, 4);
    assert_eq!(harness.state().points, IDENTITY.to_vec());
    hold(&mut harness, Key::Plus, 4);
    assert_eq!(harness.state().points.len(), 3);
}

#[test]
fn insert_and_plus_add_points_midway_up_to_the_most() {
    let mut harness = tone(&IDENTITY);
    focus(&mut harness, Role::ListBox, "Tone curve");
    press(&mut harness, Modifiers::NONE, Key::Insert);
    assert_eq!(
        harness.state().last(),
        Some(CurveAction::Add { x: 0.5, y: 0.5 })
    );
    // The new point is chosen: + adds between it and the last.
    press(&mut harness, Modifiers::NONE, Key::Plus);
    assert_eq!(
        harness.state().last(),
        Some(CurveAction::Add { x: 0.75, y: 0.75 })
    );
    press(&mut harness, Modifiers::NONE, Key::Plus);
    assert_eq!(harness.state().points.len(), 5);
    // Five is the most this curve takes.
    press(&mut harness, Modifiers::NONE, Key::Plus);
    assert_eq!(harness.state().points.len(), 5);
    assert_eq!(harness.state().actions.len(), 3);
}

#[test]
fn delete_removes_the_chosen_point_and_shift_delete_all_of_them() {
    let mut harness = tone(&THREE);
    focus(&mut harness, Role::ListBox, "Tone curve");
    press(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(harness.state().last(), None, "the first point stays");
    press(&mut harness, Modifiers::NONE, Key::PageDown);
    press(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(harness.state().last(), Some(CurveAction::Remove(1)));
    press(&mut harness, Modifiers::NONE, Key::Backspace);
    assert_eq!(harness.state().actions.len(), 1, "the last point stays");
    // Keys with Ctrl are the app's.
    press(&mut harness, Modifiers::COMMAND, Key::Delete);
    assert_eq!(harness.state().actions.len(), 1);
    press(&mut harness, Modifiers::SHIFT, Key::Delete);
    assert_eq!(harness.state().last(), Some(CurveAction::Reset));
    // A Mac's Delete is egui's Backspace.
    press(&mut harness, Modifiers::SHIFT, Key::Backspace);
    assert_eq!(harness.state().actions.len(), 3);
}

#[test]
fn the_handles_drag_the_black_and_white_points() {
    let mut harness = tone(&IDENTITY);
    let area = area(&harness);
    let black = harness
        .get_by_role_and_label(Role::Slider, "Tone curve: black point")
        .rect()
        .center();
    harness.hover_at(black);
    harness.run();
    harness.drag_at(black);
    harness.run();
    let to = pos2(at(area, 0.2, 0.0).x, black.y);
    harness.hover_at(to);
    harness.run();
    let Some(CurveAction::BlackPoint { value, done }) = harness.state().last() else {
        panic!("{:?}", harness.state().actions);
    };
    assert!(near(value, 0.2) && !done, "{value}");
    harness.drop_at(to);
    harness.run();
    assert!(matches!(
        harness.state().last(),
        Some(CurveAction::BlackPoint { done: true, .. })
    ));
    // Levels keep 0.05 apart.
    let black = harness
        .get_by_role_and_label(Role::Slider, "Tone curve: black point")
        .rect()
        .center();
    drag(&mut harness, black, pos2(area.right() + 30.0, black.y));
    let (black, white) = (harness.state().black, harness.state().white);
    assert!(
        white - black >= 0.05 && white - black < 0.05 + 1e-6,
        "{black}"
    );
    assert_eq!(harness.state().points, IDENTITY.to_vec());
}

#[test]
fn the_handles_take_the_slider_keys_and_a_double_click_resets_them() {
    let mut harness = tone(&IDENTITY);
    focus(&mut harness, Role::Slider, "Tone curve: white point");
    press(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    let Some(CurveAction::WhitePoint { value, done: true }) = harness.state().last() else {
        panic!("{:?}", harness.state().actions);
    };
    assert!((value - 0.99).abs() < 1e-6, "{value}");
    press(&mut harness, Modifiers::SHIFT, Key::ArrowDown);
    assert!((harness.state().white - 0.89).abs() < 1e-5);
    press(&mut harness, Modifiers::NONE, Key::PageDown);
    assert!((harness.state().white - 0.79).abs() < 1e-5);
    press(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(harness.state().white, 1.0);
    press(&mut harness, Modifiers::NONE, Key::Home);
    assert!(harness.state().white - harness.state().black >= 0.05);
    assert!(harness.state().white < 0.05 + 1e-6);
    assert!(double_click(&mut harness, |h| {
        h.get_by_role_and_label(Role::Slider, "Tone curve: white point")
    }));
    assert_eq!(harness.state().white, 1.0);
}

#[test]
fn a_screen_reader_steps_sets_and_hears_a_handle() {
    let mut harness = tone(&IDENTITY);
    let black = || "Tone curve: black point";
    harness
        .get_by_role_and_label(Role::Slider, black())
        .increment_accesskit();
    harness.run();
    assert!((harness.state().black - 0.01).abs() < 1e-6);
    harness
        .get_by_role_and_label(Role::Slider, black())
        .set_value_accesskit(4.0);
    harness.run();
    assert!((harness.state().black - 0.04).abs() < 1e-6);
    // It hears 4, not 3.9999998, and the range the handle can really reach.
    let node = harness.get_by_role_and_label(Role::Slider, black());
    let node = node.accesskit_node();
    assert_eq!(node.numeric_value(), Some(4.0));
    assert_eq!(node.min_numeric_value(), Some(0.0));
    assert_eq!(node.max_numeric_value(), Some(95.0));
    assert_eq!(node.numeric_value_step(), Some(1.0));
}

#[test]
fn points_tell_screen_readers_where_they_are() {
    // VoiceOver reads a list item's name and nothing else, so the name says where it is.
    let points = [(0.0, 0.0), (0.425, 0.51), (1.0, 1.0)];
    let harness = tone(&points);
    harness.get_by_role_and_label(Role::ListBoxOption, "Point 2: input 42.5%, output 51%");
    let harness = tone_in(&points, Locale::Uk, Builtin::Dark, |_| {});
    harness.get_by_role_and_label(Role::ListBoxOption, "Точка 2: вхід 42,5%, вихід 51%");
    harness.get_by_role_and_label(Role::Slider, "Tone curve: точка чорного");
}

/// A curve whose handles are its own ends, (black, 0) and (white, 1), with `points`
/// between them.
fn ends(points: &[(f32, f32)], black: f32, white: f32) -> Harness<'static, Tone> {
    tone_with(points, |tone| {
        tone.ends = true;
        (tone.black, tone.white) = (black, white);
    })
}

#[test]
fn when_the_handles_are_the_ends_every_point_can_go() {
    let mut harness = ends(&[(0.3, 0.2), (0.7, 0.8)], 0.1, 0.9);
    assert!(double_click(&mut harness, |h| point(h, 1)));
    assert_eq!(harness.state().last(), Some(CurveAction::Remove(0)));
    focus(&mut harness, Role::ListBox, "Tone curve");
    press(&mut harness, Modifiers::NONE, Key::Home);
    press(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(harness.state().last(), Some(CurveAction::Remove(0)));
    assert!(harness.state().points.is_empty());
    // With no points, Insert adds one midway between the handles.
    press(&mut harness, Modifiers::NONE, Key::Insert);
    let Some(CurveAction::Add { x, .. }) = harness.state().last() else {
        panic!("{:?}", harness.state().actions);
    };
    assert!((x - 0.5).abs() < 1e-6, "{x}");
}

#[test]
fn when_the_handles_are_the_ends_points_and_handles_keep_their_order() {
    let mut harness = ends(&[(0.3, 0.2), (0.7, 0.8)], 0.1, 0.9);
    let area = area(&harness);
    // No point outside the ends.
    click_at(&mut harness, at(area, 0.05, 0.5));
    click_at(&mut harness, at(area, 0.95, 0.5));
    assert_eq!(harness.state().actions, []);
    // A point stops the gap short of the black handle.
    drag(&mut harness, at(area, 0.3, 0.2), at(area, 0.0, 0.2));
    assert!(just_apart(0.1, harness.state().points[0].0));
    // With the point moved right, the black handle follows it as far as the gap.
    let first = harness.state().points[0];
    drag(&mut harness, at(area, first.0, first.1), at(area, 0.4, 0.2));
    let first = harness.state().points[0].0;
    assert!(near(first, 0.4), "{first}");
    let black = harness
        .get_by_role_and_label(Role::Slider, "Tone curve: black point")
        .rect()
        .center();
    drag(&mut harness, black, pos2(at(area, 0.6, 0.0).x, black.y));
    let black = harness.state().black;
    assert!(black > 0.3, "{black}");
    assert!(just_apart(black, first), "{black} {first}");
    // The white handle stops the gap short of the last point.
    focus(&mut harness, Role::Slider, "Tone curve: white point");
    press(&mut harness, Modifiers::NONE, Key::Home);
    assert!(just_apart(0.7, harness.state().white));
}

#[test]
fn a_disabled_curve_ignores_input() {
    let mut harness = tone_with(&THREE, |tone| tone.enabled = false);
    let area = area(&harness);
    click_at(&mut harness, at(area, 0.3, 0.8));
    drag(&mut harness, at(area, 0.5, 0.5), at(area, 0.6, 0.9));
    let black = harness
        .get_by_role_and_label(Role::Slider, "Tone curve: black point")
        .rect()
        .center();
    drag(&mut harness, black, black + vec2(60.0, 0.0));
    focus(&mut harness, Role::ListBox, "Tone curve");
    press(&mut harness, Modifiers::NONE, Key::Insert);
    press(&mut harness, Modifiers::NONE, Key::ArrowUp);
    assert_eq!(harness.state().actions, []);
    assert!(
        harness
            .get_by_role_and_label(Role::ListBox, "Tone curve")
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn values_that_are_not_numbers_do_not_break_the_curve() {
    let mut harness = tone_with(
        &[(0.0, 0.0), (f32::NAN, 0.5), (1.0, f32::INFINITY)],
        |tone| {
            tone.black = f32::NAN;
        },
    );
    focus(&mut harness, Role::ListBox, "Tone curve");
    press(&mut harness, Modifiers::NONE, Key::PageDown);
    press(&mut harness, Modifiers::NONE, Key::ArrowRight);
    focus(&mut harness, Role::Slider, "Tone curve: white point");
    press(&mut harness, Modifiers::NONE, Key::Home);
    for action in &harness.state().actions {
        if let CurveAction::Move { x, y, .. } = action {
            assert!(x.is_finite() && y.is_finite(), "{action:?}");
        }
    }
}

/// The curve in use: the second point chosen with the keys, so it is drawn in the accent
/// with the focus ring, and its values in the corner, in the pseudo-locale.
#[test]
fn the_curve_in_use_in_the_pseudo_locale() {
    let mut harness = tone_in(
        &[(0.0, 0.0), (0.3, 0.22), (0.7, 0.8), (1.0, 1.0)],
        Locale::Pseudo,
        Builtin::Dark,
        |tone| {
            (tone.black, tone.white) = (0.04, 0.96);
            tone.histogram = Some(
                (0..64)
                    .map(|i| {
                        let x = (i as f32 + 0.5) / 64.0;
                        (-((x - 0.35) / 0.15).powi(2)).exp() + 0.05
                    })
                    .collect(),
            );
        },
    );
    focus(&mut harness, Role::ListBox, "Tone curve");
    press(&mut harness, Modifiers::NONE, Key::PageDown);
    harness.remove_cursor();
    harness.run();
    harness.snapshot("curve-in-use-pseudo");
}

/// A curve whose handles are its ends and that has no points yet, as Tone curve starts,
/// focused: with no point to choose, the plot shows the focus.
#[test]
fn an_empty_curve_shows_its_focus() {
    let mut harness = ends(&[], 0.0, 1.0);
    focus(&mut harness, Role::ListBox, "Tone curve");
    harness.remove_cursor();
    harness.run();
    harness.snapshot("curve-empty-focused");
}
