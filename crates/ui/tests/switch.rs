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

//! The switch and a boolean's row (`docs/gui.md`, Switches and boolean rows), checked by
//! what they paint and by what screen readers hear.

use apertrust_ui::components::{ButtonState, Switch, ToggleRow};
use apertrust_ui::{Builtin, Locale, Theme};
use egui::accesskit::{Role, Toggled};
use egui::{Color32, Key, Rect, Shape, pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

/// The shapes a frame painted, flattened.
fn shapes(harness: &Harness<'_, impl Sized>) -> Vec<Shape> {
    fn flat(shape: &Shape, out: &mut Vec<Shape>) {
        match shape {
            Shape::Vec(shapes) => shapes.iter().for_each(|s| flat(s, out)),
            other => out.push(other.clone()),
        }
    }
    let mut out = Vec::new();
    for clipped in &harness.output().shapes {
        flat(&clipped.shape, &mut out);
    }
    out
}

/// The track and the knob a frame painted: the one filled rectangle as wide as a switch
/// and the one filled circle.
fn track_and_knob(harness: &Harness<'_, impl Sized>, t: &Theme) -> (Rect, Color32, Color32) {
    let mut track = None;
    let mut knob = None;
    for shape in shapes(harness) {
        match shape {
            Shape::Rect(r)
                if (r.rect.width() - t.sizes.switch_width).abs() < 0.01
                    && r.fill != Color32::TRANSPARENT =>
            {
                track = Some((r.rect, r.fill));
            }
            Shape::Circle(c) => knob = Some(c.fill),
            _ => {}
        }
    }
    let (rect, fill) = track.expect("a track was painted");
    (rect, fill, knob.expect("a knob was painted"))
}

fn alone(which: Builtin, on: bool, state: ButtonState) -> Harness<'static, bool> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(120.0, 40.0))
        .with_context_setup(move |ctx| apertrust_ui::install(ctx, Theme::builtin(which)))
        .build_ui_state(
            move |ui, on: &mut bool| {
                Switch::new(on, "Use camera bias").state(state).show(ui);
            },
            on,
        );
    harness.run();
    harness
}

#[test]
fn every_state_of_a_switch_paints_its_tokens() {
    for which in [
        Builtin::Dark,
        Builtin::Grey,
        Builtin::Light,
        Builtin::HighContrast,
    ] {
        let t = Theme::builtin(which);
        let c = &t.colors;
        // (state, on, the track, the knob), as the design's table has them; in the
        // high-contrast theme, whose plates have an edge, on is drawn as its ticked
        // checkbox is.
        let edged = c.button_edge.a() > 0;
        let (on_rest, on_hover, on_pressed, on_disabled) = if edged {
            (
                (c.check, c.check_mark),
                (c.check_hover, c.check_mark),
                (c.check_pressed, c.check_mark),
                (c.text_disabled, c.sunken),
            )
        } else {
            (
                (c.button, c.text_strong),
                (c.button_hover, c.text_strong),
                (c.button_pressed, c.text),
                (c.button, c.text_disabled),
            )
        };
        let table = [
            (ButtonState::Rest, false, c.sunken, c.control_outline),
            (ButtonState::Rest, true, on_rest.0, on_rest.1),
            (ButtonState::Hover, false, c.sunken, c.text_secondary),
            (ButtonState::Hover, true, on_hover.0, on_hover.1),
            (
                ButtonState::Pressed,
                false,
                c.button_pressed,
                c.text_secondary,
            ),
            (ButtonState::Pressed, true, on_pressed.0, on_pressed.1),
            (ButtonState::Disabled, false, c.sunken, c.text_disabled),
            (ButtonState::Disabled, true, on_disabled.0, on_disabled.1),
            (ButtonState::Focus, false, c.sunken, c.control_outline),
            (ButtonState::Focus, true, on_rest.0, on_rest.1),
        ];
        for (state, on, track, knob) in table {
            let harness = alone(which, on, state);
            let (rect, fill, circle) = track_and_knob(&harness, &t);
            assert_eq!(fill, track, "{which:?} {state:?} on {on}: track");
            assert_eq!(circle, knob, "{which:?} {state:?} on {on}: knob");
            // 26 by 14, the knob 8 across and 3 points in, at the end its side says.
            assert_eq!((rect.width(), rect.height()), (26.0, 14.0));
            let at = shapes(&harness)
                .into_iter()
                .find_map(|s| match s {
                    Shape::Circle(c) => Some((c.center, c.radius)),
                    _ => None,
                })
                .expect("a knob");
            assert_eq!(at.1 * 2.0, 8.0);
            let expected = if on {
                rect.right() - 7.0
            } else {
                rect.left() + 7.0
            };
            assert!((at.0.x - expected).abs() < 0.01, "{state:?} {on}: {at:?}");
            assert!((at.0.y - rect.center().y).abs() < 0.01);
            // Never the accent, whatever the state.
            for accent in [c.accent, c.accent_hover, c.accent_pressed] {
                assert!(fill != accent && circle != accent, "{which:?} {state:?}");
            }
        }
        // On and off differ in lightness, in the track or the knob, in every theme, at rest
        // and disabled: the knob's side is not all that tells them apart.
        let luma = |c: Color32| {
            0.2126 * f32::from(c.r()) + 0.7152 * f32::from(c.g()) + 0.0722 * f32::from(c.b())
        };
        for (off, on) in [(0, 1), (6, 7)] {
            let (off, on) = (&table[off], &table[on]);
            let by = (luma(off.2) - luma(on.2))
                .abs()
                .max((luma(off.3) - luma(on.3)).abs());
            assert!(
                by >= 16.0,
                "{which:?} {:?}: on and off look alike ({by})",
                off.0
            );
        }
    }
}

#[test]
fn a_switch_is_a_switch_with_its_checked_state_and_space_toggles_it() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(120.0, 40.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, on: &mut bool| {
                Switch::new(on, "Use camera bias").show(ui);
            },
            false,
        );
    harness.run();
    let node = harness.get_by_role_and_label(Role::Switch, "Use camera bias");
    assert_eq!(node.accesskit_node().toggled(), Some(Toggled::False));
    node.focus();
    harness.run();
    harness.key_press(Key::Space);
    harness.run();
    assert!(*harness.state());
    let node = harness.get_by_role_and_label(Role::Switch, "Use camera bias");
    assert_eq!(node.accesskit_node().toggled(), Some(Toggled::True));
}

/// A row with a value, in `locale` at `width` points, over `on`.
fn row(locale: Locale, width: f32, label: &'static str, on: bool) -> Harness<'static, bool> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(width, 40.0))
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::default());
            apertrust_ui::set_locale(ctx, locale);
        })
        .build_ui_state(
            move |ui, on: &mut bool| {
                if let Some(now) = ToggleRow::new(label, *on).value("−⅔ EV").show(ui) {
                    *on = now;
                }
            },
            on,
        );
    harness.run();
    harness
}

#[test]
fn a_row_is_label_value_and_a_switch_at_its_end_the_whole_row_the_target() {
    let t = Theme::default();
    let mut harness = row(Locale::EnGb, 296.0, "Compensate camera exposure", true);
    let node = harness.get_by_role_and_label(Role::Switch, "Compensate camera exposure −⅔ EV");
    assert_eq!(node.accesskit_node().toggled(), Some(Toggled::True));
    let row = node.rect();
    assert_eq!(row.height(), t.sizes.panel_line);
    // The switch ends where a slider's value and a choice's chevron do, a field's inset
    // from the row's end, and the words end a group's gap before it.
    let (track, _, _) = track_and_knob(&harness, &t);
    let inset = t.spacing.xs + t.shape.hairline;
    assert!(
        (track.right() - (row.right() - inset)).abs() < 0.01,
        "{track:?} {row:?}"
    );
    assert!((track.center().y - row.center().y).abs() < 0.01);
    // The label, then its value 4 points on, in text_secondary, then the gap, then the switch.
    let texts: Vec<(String, Rect, Color32)> = shapes(&harness)
        .into_iter()
        .filter_map(|s| match s {
            Shape::Text(text) => Some((
                text.galley.text().to_owned(),
                text.galley.rect.translate(text.pos.to_vec2()),
                text.fallback_color,
            )),
            _ => None,
        })
        .collect();
    let label = texts
        .iter()
        .find(|(s, ..)| s == "Compensate camera exposure")
        .expect("label");
    let value = texts
        .iter()
        .find(|(s, ..)| s.trim() == "−⅔ EV")
        .expect("value");
    assert!((label.1.left() - row.left()).abs() < 0.5);
    assert!(value.1.right() <= track.left() - t.spacing.m + 0.5);
    // Anywhere on the row toggles it, the label's start and the gap before the switch.
    for at in [
        pos2(row.left() + 4.0, row.center().y),
        pos2(track.left() - 3.0, row.center().y),
    ] {
        let before = *harness.state();
        harness.hover_at(at);
        harness.run();
        // Under the pointer the row has the hover band a choice row has.
        let band = shapes(&harness).into_iter().any(|s| {
            matches!(s, Shape::Rect(r) if r.fill == t.colors.hover && r.rect.width() > row.width())
        });
        assert!(band, "the row's hover band");
        harness.drag_at(at);
        harness.drop_at(at);
        harness.run();
        assert_eq!(*harness.state(), !before, "{at:?}");
    }
}

#[test]
fn a_long_label_is_cut_before_its_value_which_is_never_cut_in_uk_and_pl_at_240() {
    let t = Theme::default();
    let labels = [
        (
            Locale::Uk,
            "Компенсувати експозицію камери, записану камерою в метаданих",
        ),
        (
            Locale::Pl,
            "Skompensuj ekspozycję aparatu zapisaną przez aparat w metadanych",
        ),
    ];
    for (locale, label) in labels {
        let harness = row(locale, 240.0, label, false);
        let (track, _, _) = track_and_knob(&harness, &t);
        let texts: Vec<(String, Rect, bool)> = shapes(&harness)
            .into_iter()
            .filter_map(|s| match s {
                Shape::Text(text) => Some((
                    text.galley.text().to_owned(),
                    text.galley.rect.translate(text.pos.to_vec2()),
                    text.galley.elided,
                )),
                _ => None,
            })
            .collect();
        let cut = texts
            .iter()
            .find(|(_, _, elided)| *elided)
            .unwrap_or_else(|| panic!("{locale:?}: the label is cut"));
        let value = texts
            .iter()
            .find(|(s, _, _)| s.trim() == "−⅔ EV")
            .expect("the value, whole");
        assert!(
            cut.1.right() <= value.1.left() + 0.5,
            "{locale:?}: the label before the value"
        );
        assert!(
            value.1.right() <= track.left() - t.spacing.m + 0.5,
            "{locale:?}"
        );
        assert!(track.right() <= 240.0);
        // Heard whole.
        harness.get_by_role_and_label(Role::Switch, &format!("{label} −⅔ EV"));
    }
}

/// Whether a focus ring, `focus_ring` stroked round a rectangle, was painted, and its size.
fn rings(harness: &Harness<'_, impl Sized>, t: &Theme) -> Vec<Rect> {
    shapes(harness)
        .into_iter()
        .filter_map(|s| match s {
            Shape::Rect(r) if r.stroke.color == t.colors.focus_ring && r.stroke.width > 1.0 => {
                Some(r.rect)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn a_bare_switch_rings_its_track_and_a_row_rings_the_whole_row() {
    let t = Theme::default();
    // A bare switch: the ring is round the track, a gap off it.
    let mut harness = Harness::builder()
        .with_size(egui::vec2(200.0, 40.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, on: &mut bool| {
                Switch::new(on, "Use camera bias").show(ui);
            },
            false,
        );
    harness.run();
    assert!(rings(&harness, &t).is_empty(), "no ring until focused");
    harness
        .get_by_role_and_label(Role::Switch, "Use camera bias")
        .focus();
    harness.run();
    let (track, _, _) = track_and_knob(&harness, &t);
    let ring = rings(&harness, &t);
    assert_eq!(ring.len(), 1, "{ring:?}");
    assert!(ring[0].contains_rect(track) && ring[0].width() < track.width() + 8.0);
    // A row: the ring is round the whole row, as a choice row's, the switch alone ringless.
    let mut harness = row(Locale::EnGb, 296.0, "Compensate camera exposure", false);
    harness.run();
    assert!(rings(&harness, &t).is_empty(), "no ring until focused");
    let node = harness.get_by_role_and_label(Role::Switch, "Compensate camera exposure −⅔ EV");
    let area = node.rect();
    node.focus();
    harness.run();
    let ring = rings(&harness, &t);
    assert_eq!(ring.len(), 1, "{ring:?}");
    assert!(
        ring[0].width() > 200.0 && ring[0].contains_rect(area),
        "{ring:?} {area:?}"
    );
}

#[test]
fn space_toggles_a_focused_row() {
    let mut harness = row(Locale::EnGb, 296.0, "Compensate camera exposure", false);
    let node = harness.get_by_role_and_label(Role::Switch, "Compensate camera exposure −⅔ EV");
    node.focus();
    harness.run();
    harness.key_press(Key::Space);
    harness.run();
    assert!(*harness.state());
    harness.key_press(Key::Space);
    harness.run();
    assert!(!*harness.state());
}
