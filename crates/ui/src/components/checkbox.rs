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

//! A checkbox (`docs/gui.md`, Checkboxes, text buttons and icon buttons): a 14-point box
//! at the start, its label after it, the whole of both the target. Unticked, the box is
//! empty, outlined in `control_outline`, so it takes any ground, and fills with
//! `button_hover` under the pointer and `button_pressed` pressed, its edge then
//! `text_secondary`. Ticked, it fills with `check`, neutral, never the accent, with a
//! 1.5-point `check_mark` tick. Disabled, the edge, the label and a ticked box's fill are
//! `text_disabled`. A value after the label, as Exposure's "−⅔ EV", is `text_secondary`,
//! `text_disabled` with the rest, and part of the target; it is never cut, the label
//! before it cut at its end with … where the row is too narrow.

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Rect, Response, Role, Shape, Stroke, StrokeKind, Ui};
use egui::{WidgetInfo, pos2, vec2};

use super::plate::{ButtonState, Touch, allocate, paint_focus, undimmed_painter};
use crate::{Theme, theme};

/// The box's side, in points.
const BOX: f32 = 14.0;

/// The tick's three points from the box's centre, and its width.
const TICK: [[f32; 2]; 3] = [[-3.2, 0.1], [-0.9, 2.4], [3.3, -2.3]];
const TICK_WIDTH: f32 = 1.5;

pub struct Checkbox<'a> {
    on: &'a mut bool,
    label: &'a str,
    value: Option<&'a str>,
    enabled: bool,
    state: Option<ButtonState>,
}

impl<'a> Checkbox<'a> {
    /// `label` is translated, and may be empty for a box alone; a click toggles `on`.
    pub fn new(on: &'a mut bool, label: &'a str) -> Self {
        Checkbox {
            on,
            label,
            value: None,
            enabled: true,
            state: None,
        }
    }

    /// A value after the label, a small gap (`spacing.xs`) apart, in `text_secondary`:
    /// translated, with no brackets, as Exposure's "−⅔ EV".
    pub fn value(mut self, value: &'a str) -> Self {
        self.value = Some(value);
        self
    }

    /// A disabled checkbox keeps its tick and ignores input.
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
        self.show_cut(ui).0
    }

    /// Draws it as [`Self::show`] does, saying too whether the label was cut short
    /// before its value, so the row can show it whole in its tooltip.
    pub fn show_cut(self, ui: &mut Ui) -> (Response, bool) {
        let t = theme(ui.ctx());
        let enabled = self.enabled && ui.is_enabled() && self.state != Some(ButtonState::Disabled);
        let gap = ui.spacing().icon_spacing;
        let font = FontId::proportional(t.text.body);
        // The value a small gap after the label, never cut.
        let value = self.value.map(|value| {
            let colour = if enabled {
                t.colors.text_secondary
            } else {
                t.colors.text_disabled
            };
            let mut job = LayoutJob::default();
            job.append(
                value,
                t.spacing.xs,
                TextFormat::simple(font.clone(), colour),
            );
            ui.painter().layout_job(job)
        });
        let galley = (!self.label.is_empty()).then(|| match &value {
            // A label with a value is cut with … before it where the row is too narrow.
            Some(value) => {
                let room = ui.available_width() - BOX - gap - value.size().x;
                let job = super::truncated(
                    self.label,
                    font.clone(),
                    room.max(0.0),
                    Color32::PLACEHOLDER,
                );
                ui.painter().layout_job(job)
            }
            None => ui.painter().layout_no_wrap(
                self.label.to_owned(),
                font.clone(),
                Color32::PLACEHOLDER,
            ),
        });
        let cut = galley.as_ref().is_some_and(|galley| galley.elided);
        // The value follows the label, or the box where there is no label.
        let words = match (&galley, &value) {
            (Some(g), v) => gap + g.size().x + v.as_ref().map_or(0.0, |v| v.size().x),
            (None, Some(v)) => gap + v.size().x,
            (None, None) => 0.0,
        };
        // As egui's own: at least a control square, the box at its start.
        let size = vec2(BOX + words, t.sizes.control).max(vec2(t.sizes.control, 0.0));
        let (rect, mut response) = allocate(ui, size, enabled);
        if response.clicked() {
            *self.on = !*self.on;
            response.mark_changed();
        }
        let on = *self.on;
        if ui.is_rect_visible(rect) {
            let painter = &undimmed_painter(ui);
            let touch = Touch::of(ui.ctx(), &response, enabled, self.state);
            let square = Rect::from_min_size(
                pos2(rect.left(), rect.center().y - BOX / 2.0),
                vec2(BOX, BOX),
            );
            paint_box(painter, square, on, touch, &t);
            paint_focus(painter, square, t.shape.radius_small, touch, &t);
            let colour = if touch.enabled {
                t.colors.text
            } else {
                t.colors.text_disabled
            };
            let mut end = square.right() + gap;
            if let Some(galley) = galley {
                let at = pos2(end, rect.center().y - galley.size().y / 2.0);
                end = at.x + galley.size().x;
                painter.galley(at, galley, colour);
            }
            if let Some(value) = value {
                let at = pos2(end, rect.center().y - value.size().y / 2.0);
                painter.galley(at, value, colour);
            }
        }
        // Heard whole, with its value.
        let label = match self.value {
            Some(value) if self.label.is_empty() => value.to_owned(),
            Some(value) => format!("{} {value}", self.label),
            None => self.label.to_owned(),
        };
        response.widget_info(|| WidgetInfo::selected(Role::CheckBox, enabled, on, &label));
        (response, cut)
    }
}

/// The box in `square`, ticked or not, as `touch` has it.
fn paint_box(painter: &egui::Painter, square: Rect, on: bool, touch: Touch, t: &Theme) {
    let c = &t.colors;
    let radius = t.shape.radius_small;
    if on {
        let fill = if !touch.enabled {
            c.text_disabled
        } else if touch.pressed {
            c.check_pressed
        } else if touch.hovered {
            c.check_hover
        } else {
            c.check
        };
        painter.rect_filled(square, radius, fill);
        let centre = square.center();
        let points = TICK
            .iter()
            .map(|[x, y]| centre + vec2(*x, *y))
            .collect::<Vec<_>>();
        painter.add(Shape::line(points, Stroke::new(TICK_WIDTH, c.check_mark)));
        return;
    }
    let (fill, edge) = if !touch.enabled {
        (Color32::TRANSPARENT, c.text_disabled)
    } else if touch.pressed {
        (c.button_pressed, c.text_secondary)
    } else if touch.hovered {
        (c.button_hover, c.text_secondary)
    } else {
        (Color32::TRANSPARENT, c.control_outline)
    };
    painter.rect(
        square,
        radius,
        fill,
        Stroke::new(t.shape.hairline, edge),
        StrokeKind::Inside,
    );
}
