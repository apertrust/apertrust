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

//! The Settings dialog's switch (`docs/gui.md`, The Settings dialog): its size and place,
//! each state's colours, and what screen readers hear.

use apertrust_ui::components::ButtonState;
use apertrust_ui::components::settings::Switch;
use apertrust_ui::{Builtin, Theme};
use egui::accesskit::{Role, Toggled};
use egui::{Color32, Key, Rect, Shape};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

/// The column's width, as a settings row gives its control.
const COLUMN: f32 = 260.0;

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

/// The switch alone in a column, `on` and pinned in `state`.
fn alone(which: Builtin, on: bool, state: Option<ButtonState>) -> Harness<'static, bool> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(COLUMN + 20.0, 40.0))
        .with_context_setup(move |ctx| apertrust_ui::install(ctx, Theme::builtin(which)))
        .build_ui_state(
            move |ui, on: &mut bool| {
                ui.set_width(COLUMN);
                let mut switch = Switch::new(on, "Reopen what was open last");
                if let Some(state) = state {
                    switch = switch.state(state);
                }
                switch.show(ui);
            },
            on,
        );
    harness.run();
    harness
}

/// The track and its fill, and the knob's centre, radius and fill.
fn painted(harness: &Harness<'_, bool>, t: &Theme) -> (Rect, Color32, egui::Pos2, f32, Color32) {
    let mut track = None;
    let mut knob = None;
    for shape in shapes(harness) {
        match shape {
            Shape::Rect(r)
                if (r.rect.width() - t.sizes.setting_switch_width).abs() < 0.01
                    && r.fill != Color32::TRANSPARENT =>
            {
                track = Some((r.rect, r.fill));
            }
            Shape::Circle(c) => knob = Some((c.center, c.radius, c.fill)),
            _ => {}
        }
    }
    let (rect, fill) = track.expect("a track");
    let (centre, radius, colour) = knob.expect("a knob");
    (rect, fill, centre, radius, colour)
}

/// Off, on, under the pointer and disabled, each in its table's colours, in every theme:
/// a tool's switch's colours, a size up, at the column's end.
#[test]
fn each_state_of_a_settings_switch_paints_its_tokens() {
    for which in Builtin::ALL {
        let t = Theme::builtin(which);
        let c = &t.colors;
        let edged = c.button_edge.a() > 0;
        let (on_rest, on_hover, on_disabled) = if edged {
            (
                (c.check, c.check_mark),
                (c.check_hover, c.check_mark),
                (c.text_disabled, c.sunken),
            )
        } else {
            (
                (c.button, c.text_strong),
                (c.button_hover, c.text_strong),
                (c.button, c.text_disabled),
            )
        };
        let table = [
            (ButtonState::Rest, false, (c.sunken, c.control_outline)),
            (ButtonState::Rest, true, on_rest),
            (ButtonState::Hover, false, (c.sunken, c.text_secondary)),
            (ButtonState::Hover, true, on_hover),
            (ButtonState::Disabled, false, (c.sunken, c.text_disabled)),
            (ButtonState::Disabled, true, on_disabled),
        ];
        for (state, on, (track, knob)) in table {
            let harness = alone(which, on, Some(state));
            let (rect, fill, centre, radius, colour) = painted(&harness, &t);
            let id = format!("{which:?} {state:?} on {on}");
            assert_eq!(fill, track, "{id}: track");
            assert_eq!(colour, knob, "{id}: knob");
            // 32 by 18, the knob 12 across and 3 points in, at the end its side says.
            assert_eq!((rect.width(), rect.height()), (32.0, 18.0), "{id}");
            assert_eq!(radius * 2.0, 12.0, "{id}");
            let expected = if on {
                rect.right() - 9.0
            } else {
                rect.left() + 9.0
            };
            assert!((centre.x - expected).abs() < 0.01, "{id}: {centre:?}");
            // At the column's end.
            assert!((rect.right() - COLUMN - 8.0).abs() < 1.0, "{id}: {rect:?}");
        }
    }
}

#[test]
fn a_settings_switch_is_a_switch_and_space_toggles_it() {
    let mut harness = alone(Builtin::Dark, false, None);
    let node = harness.get_by_role_and_label(Role::Switch, "Reopen what was open last");
    assert_eq!(node.accesskit_node().toggled(), Some(Toggled::False));
    node.focus();
    harness.run();
    harness.key_press(Key::Space);
    harness.run();
    assert!(*harness.state());
    let node = harness.get_by_role_and_label(Role::Switch, "Reopen what was open last");
    assert_eq!(node.accesskit_node().toggled(), Some(Toggled::True));
    node.click();
    harness.run();
    assert!(!*harness.state());
}
