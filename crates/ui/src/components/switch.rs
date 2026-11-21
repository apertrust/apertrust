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

//! A switch (`docs/gui.md`, Switches and boolean rows): a quiet track, fully rounded with
//! no border, and a round knob inside it, `sizes.switch_width` × `sizes.switch_height`
//! with a `sizes.switch_knob` knob a few points in. Never the accent: on and off differ in
//! lightness as well as in the knob's side.
//!
//! | State | Track | Knob |
//! | --- | --- | --- |
//! | Off | `sunken` | `control_outline`, left |
//! | On | `button` | `text_strong`, right |
//! | Under the pointer, off | `sunken` | `text_secondary` |
//! | Under the pointer, on | `button_hover` | `text_strong` |
//! | Pressed, off | `button_pressed` | `text_secondary` |
//! | Pressed, on | `button_pressed` | `text` |
//! | Disabled, off | `sunken` | `text_disabled`, left |
//! | Disabled, on | `button` | `text_disabled`, right |
//!
//! The high-contrast theme, whose plates have an edge (`button_edge`) and whose `sunken`
//! and `button` are one black, as `control_outline` and `text_strong` are one white,
//! gives the track that edge, and draws on as a ticked checkbox is: a `check` track
//! (`check_hover`, `check_pressed`) with a `check_mark` knob; disabled on is a
//! `text_disabled` track with a `sunken` knob. So on and off differ in lightness in every
//! theme, at rest and disabled.
//!
//! Focused by a key a bare switch keeps its state's colours and gains the shared ring
//! round its track, with its gap. The switch alone is the target here;
//! [`super::ToggleRow`] makes the whole row one, rings the row as a choice row does, and
//! draws the switch with [`paint`]. To screen readers it is a `Switch` with its checked
//! state.

use egui::{Rect, Response, Role, Stroke, StrokeKind, Ui, WidgetInfo, pos2, vec2};

use super::plate::{ButtonState, Touch, allocate, paint_focus, undimmed_painter};
use crate::{Theme, theme};

pub struct Switch<'a> {
    on: &'a mut bool,
    name: &'a str,
    enabled: bool,
    state: Option<ButtonState>,
}

impl<'a> Switch<'a> {
    /// `name` is what screen readers say, translated; a click or Space toggles `on`.
    pub fn new(on: &'a mut bool, name: &'a str) -> Self {
        Switch {
            on,
            name,
            enabled: true,
            state: None,
        }
    }

    /// A disabled switch keeps its position and ignores input.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws it in `state` whatever the pointer and the keys do, as the gallery shows each
    /// state.
    pub fn state(mut self, state: ButtonState) -> Self {
        self.state = Some(state);
        self
    }

    /// Draws it; the response is marked changed when a click or a key toggled it.
    pub fn show(self, ui: &mut Ui) -> Response {
        let t = theme(ui.ctx());
        let enabled = self.enabled && ui.is_enabled() && self.state != Some(ButtonState::Disabled);
        // The target is a control high, the track centred in it.
        let size = vec2(
            t.sizes.switch_width,
            t.sizes.switch_height.max(t.sizes.control),
        );
        let (rect, mut response) = allocate(ui, size, enabled);
        if response.clicked() {
            *self.on = !*self.on;
            response.mark_changed();
        }
        let on = *self.on;
        if ui.is_rect_visible(rect) {
            let touch = Touch::of(ui.ctx(), &response, enabled, self.state);
            let track = Rect::from_center_size(
                rect.center(),
                vec2(t.sizes.switch_width, t.sizes.switch_height),
            );
            let knob = t.sizes.switch_knob;
            paint(&undimmed_painter(ui), track, knob, on, touch, &t);
        }
        response.widget_info(|| WidgetInfo::selected(Role::Switch, enabled, on, self.name));
        response
    }
}

/// The switch in `track`, `on` or off, as `touch` has it: its track, its knob `knob`
/// across and, focused, the ring. A tool's switch and a Settings switch, a size up, share
/// its colours.
pub(crate) fn paint(
    painter: &egui::Painter,
    track: Rect,
    knob_side: f32,
    on: bool,
    touch: Touch,
    t: &Theme,
) {
    let c = &t.colors;
    // Where `sunken` and `button` are one black, a plate has an edge: the high-contrast
    // theme, which draws on as its ticked checkbox.
    let edged = c.button_edge.a() > 0;
    let (fill, knob) = match (touch.enabled, on, touch.pressed, touch.hovered) {
        (false, false, _, _) => (c.sunken, c.text_disabled),
        (false, true, _, _) if edged => (c.text_disabled, c.sunken),
        (false, true, _, _) => (c.button, c.text_disabled),
        (true, false, true, _) => (c.button_pressed, c.text_secondary),
        (true, true, true, _) if edged => (c.check_pressed, c.check_mark),
        (true, true, true, _) => (c.button_pressed, c.text),
        (true, false, false, true) => (c.sunken, c.text_secondary),
        (true, true, false, true) if edged => (c.check_hover, c.check_mark),
        (true, true, false, true) => (c.button_hover, c.text_strong),
        (true, false, false, false) => (c.sunken, c.control_outline),
        (true, true, false, false) if edged => (c.check, c.check_mark),
        (true, true, false, false) => (c.button, c.text_strong),
    };
    let radius = track.height() / 2.0;
    painter.rect_filled(track, radius, fill);
    // Only the high-contrast theme draws an edge on a plate, and so on a track.
    if edged {
        let edge = if touch.enabled {
            c.button_edge
        } else {
            c.text_disabled
        };
        painter.rect_stroke(
            track,
            radius,
            Stroke::new(t.shape.hairline, edge),
            StrokeKind::Inside,
        );
    }
    // The knob a few points in, at the end its side says.
    let side = knob_side;
    let inset = (track.height() - side) / 2.0;
    let x = if on {
        track.right() - inset - side / 2.0
    } else {
        track.left() + inset + side / 2.0
    };
    painter.circle_filled(pos2(x, track.center().y), side / 2.0, knob);
    paint_focus(painter, track, radius, touch, t);
}
