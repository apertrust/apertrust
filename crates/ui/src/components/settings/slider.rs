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

//! A number with both its ends in a settings row, as Interface size: a scroll bar's look,
//! not a tool's slider, with no marker and no popup.
//!
//! - **The track**, the column wide less the value's place: 16 points high
//!   (`sizes.track_strip`), fully rounded, a step darker than `sunken`
//!   (`Colors::sunken_deep`) with no frame, and no fill.
//! - **The thumb**, a capsule 20 × 12 centred on the value, inside the track's ends, in
//!   `text_secondary`; `text` under the pointer and while dragged.
//! - **The value** at the end, in `text_strong`, with its unit, at the number's decimals.
//! - **Pointer:** a drag sets the value, ending where it is let go, and a click on the
//!   track jumps there; either end of the travel is the range's end exactly. It takes no
//!   wheel: the wheel scrolls the page, whatever is under the pointer.
//! - **Keys:** ← and → step by its step, Shift ten steps and the finer modifier (Ctrl,
//!   Option on the Mac) a tenth where its decimals show one; Home and End go to its ends.
//! - **Typing:** a click on the value makes it a field, the number selected and the unit
//!   after it, an accent hairline round it; Enter or leaving it sets it, within the range, and Esc
//!   puts the value back. It takes what a tool's value field takes ([`ValueField`]).
//! - **Screen readers:** a slider named by the row's label, with its value as it is written
//!   and its range, which their own keys and set value move; the value, a focus stop of its
//!   own, a button named by the label, its value as written.

use std::hash::Hash;
use std::ops::RangeInclusive;

use egui::accesskit::{Action, ActionData};
use egui::{Id, Rect, Role, Sense, Ui, UiBuilder, WidgetInfo, pos2, vec2};

use super::super::plate::undimmed_painter;
use super::super::steps::Press;
use super::super::value_field::ValueField;
use super::super::{FieldLook, field_inset};
use super::{body, dimmed, field_look};
use crate::{Translator, theme, translator};

/// The thumb, a capsule.
const THUMB: egui::Vec2 = vec2(20.0, 12.0);

pub struct Slider<'a> {
    id: Id,
    name: &'a str,
    value: f64,
    range: RangeInclusive<f64>,
    /// The values it is kept at, as shown.
    grid: Option<f64>,
    step: f64,
    decimals: usize,
    unit: Option<&'a str>,
}

/// Whether the value's field was clicked open, and opens next frame, or is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Clicked,
    Open,
}

impl<'a> Slider<'a> {
    /// `id` names it, stable across frames; `name` is the row's label, translated; `value`
    /// is as shown, within `range`.
    pub fn new(
        id: impl Hash + std::fmt::Debug,
        name: &'a str,
        value: f64,
        range: RangeInclusive<f64>,
    ) -> Self {
        let step = (range.end() - range.start()) / 100.0;
        Slider {
            id: Id::unique(("setting slider", id)),
            name,
            value,
            range,
            grid: None,
            step,
            decimals: 0,
            unit: None,
        }
    }

    /// The values it is kept at, as shown, as [`super::Number::grid`]: what is typed, keyed
    /// or dragged is rounded to the nearest, the range's ends kept exactly.
    pub fn grid(mut self, grid: f64) -> Self {
        self.grid = Some(grid);
        self
    }

    /// What a key moves it by.
    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// How many decimals it is shown and set with.
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// Its unit's words, translated, after the value ("%").
    pub fn unit(mut self, unit: &'a str) -> Self {
        self.unit = Some(unit);
        self
    }

    /// The value as it is written, with its unit.
    fn written(&self, words: &Translator, value: f64) -> String {
        let number = words.format_number(value, self.decimals, false);
        match self.unit {
            Some(unit) => {
                let mut args = crate::i18n::Args::new();
                args.set("value", number.as_str());
                args.set("unit", unit);
                words.text_with("settings-value-with-unit", Some(&args))
            }
            None => number,
        }
    }

    /// `value` within the range, on its grid at the decimals shown: as a key, a screen
    /// reader or typing leaves it.
    fn held(&self, value: f64) -> f64 {
        let (min, max) = (*self.range.start(), *self.range.end());
        // The ends lie on the grid (the schema's generator refuses one that does not).
        super::on_grid(value, self.grid, self.decimals).clamp(min, max) + 0.0
    }

    /// `value` on a step from the range's start, then [`held`](Self::held): where the
    /// pointer leaves it.
    fn settle(&self, value: f64) -> f64 {
        let (min, max) = (*self.range.start(), *self.range.end());
        // The ends stay the ends exactly, as Home and End leave them, though the range is
        // no whole number of steps.
        if value <= min {
            return min;
        }
        if value >= max {
            return max;
        }
        let stepped = if self.step > 0.0 {
            min + ((value - min) / self.step).round() * self.step
        } else {
            value
        };
        self.held(stepped)
    }

    /// One step as `scale` makes it: a tenth only where the decimals show it.
    fn scaled(&self, scale: f64) -> f64 {
        let finest = 10f64.powi(-(self.decimals as i32));
        let step = self.step * scale;
        if step + f64::EPSILON < finest {
            self.step
        } else {
            step
        }
    }

    /// Draws it across `ui`'s width; returns the value set this frame.
    pub fn show(self, ui: &mut Ui) -> Option<f64> {
        let ctx = ui.ctx().clone();
        let t = theme(&ctx);
        let words = translator(&ctx);
        let enabled = ui.is_enabled();
        let (min, max) = (*self.range.start(), *self.range.end());
        let inset = field_inset(&t);
        // The value's place: as wide as the range's widest value needs, so it never moves.
        let widest = [min, max]
            .iter()
            .map(|v| {
                let galley =
                    ui.painter()
                        .layout_no_wrap(self.written(&words, *v), body(&t), t.colors.text);
                galley.size().x
            })
            .fold(0.0, f32::max);
        let size = vec2(ui.available_width(), t.sizes.setting_control);
        let (strip, _) = ui.allocate_exact_size(size, Sense::hover());
        let place = (widest + 2.0 * inset).ceil().max(t.sizes.value_column);
        let value_rect = Rect::from_min_max(pos2(strip.right() - place, strip.top()), strip.max);
        let track = Rect::from_min_max(
            pos2(strip.left(), strip.center().y - t.sizes.track_strip / 2.0),
            pos2(
                value_rect.left() - t.spacing.m,
                strip.center().y + t.sizes.track_strip / 2.0,
            ),
        );
        let edge = (track.height() - THUMB.y) / 2.0;
        let travel = (
            track.left() + edge + THUMB.x / 2.0,
            track.right() - edge - THUMB.x / 2.0,
        );
        let span = (max - min).max(f64::EPSILON);
        let x_of = |v: f64| {
            let f = ((v - min) / span).clamp(0.0, 1.0) as f32;
            travel.0 + f * (travel.1 - travel.0)
        };
        let value_at = |x: f32| {
            let f = (x - travel.0) / (travel.1 - travel.0).max(1.0);
            // The ends exactly: `min + span` need not be `max` (-1 + 1.2 is 0.19999…).
            if f <= 0.0 {
                min
            } else if f >= 1.0 {
                max
            } else {
                min + f64::from(f) * span
            }
        };
        let id = self.id;
        // A track too short for the thumb to travel takes no pointer and is not drawn:
        // the value, typed, still sets it.
        let travels = travel.1 - travel.0 >= THUMB.x;
        let sense = if enabled && travels {
            Sense::click_and_drag()
        } else {
            Sense::hover()
        };
        let response = ui.interact(track, id, sense);
        let mut shown = self.value;
        if enabled {
            // A press jumps, a drag follows: held, where the pointer is; let go, where it was
            // let go, though it moved on in the same pass. The primary button's alone.
            let at = if super::pressed(&response) {
                response.interact_pointer_pos()
            } else if response.clicked() || response.drag_stopped_by(egui::PointerButton::Primary) {
                ui.input(super::super::steps::released_at)
            } else {
                None
            };
            if let Some(at) = at {
                shown = self.settle(value_at(at.x));
            }
            if response.has_focus() {
                // ← and → are its own, not the focus's to move.
                ui.memory_mut(|m| {
                    m.set_focus_lock_filter(
                        id,
                        egui::EventFilter {
                            horizontal_arrows: true,
                            ..Default::default()
                        },
                    );
                });
                // Each press in order, with its own modifiers, held in the range before the
                // next: at the end, → then ← leaves it a step under the end, and End then
                // Home at its start.
                let os = ctx.os();
                ui.input_mut(|i| {
                    super::super::steps::presses(i, os, |press| {
                        shown = match press {
                            Press::Step(scale) => {
                                self.held(shown + scale.signum() * self.scaled(scale.abs()))
                            }
                            Press::Start => min,
                            Press::End => max,
                        };
                    });
                });
            }
            let (more, less, set) = ui.input(|i| {
                (
                    i.num_accesskit_action_requests(id, Action::Increment),
                    i.num_accesskit_action_requests(id, Action::Decrement),
                    i.accesskit_action_requests(id, Action::SetValue)
                        .filter_map(|request| match request.data {
                            Some(ActionData::NumericValue(v)) => Some(v),
                            _ => None,
                        })
                        .last(),
                )
            });
            if let Some(v) = set {
                shown = self.held(v);
            } else if more != less {
                shown = self.held(shown + (more as f64 - less as f64) * self.step);
            }
        }
        // The value's field, open from a click on the value until it loses the focus. A
        // click opens it on the next frame: egui takes the focus from a widget a click
        // lands beside, and the field, new under the pointer, is beside it this frame.
        // A click is kept with the pass it was taken in, and opens the field only in the
        // pass right after: one the slider was not drawn in, its page left, is dropped.
        let open_id = id.with("open");
        let field_id = id.with("field");
        let pass = ctx.cumulative_pass_nr();
        let clicked = enabled
            && ctx
                .read_response(id.with("value"))
                .is_some_and(|value| value.clicked());
        let was = ui.data(|d| d.get_temp::<(Field, u64)>(open_id));
        if clicked {
            ui.data_mut(|d| d.insert_temp(open_id, (Field::Clicked, pass)));
            ctx.request_repaint();
        }
        let opening =
            enabled && !clicked && was.is_some_and(|w| w == (Field::Clicked, pass.wrapping_sub(1)));
        if opening {
            ui.data_mut(|d| d.insert_temp(open_id, (Field::Open, pass)));
            ValueField::forget(&ctx, field_id);
            ui.memory_mut(|m| m.request_focus(field_id));
        }
        let live = opening
            || ui.memory(|m| m.has_focus(field_id))
            || ctx
                .read_response(field_id)
                .is_some_and(|field| field.lost_focus());
        let state = ui.data(|d| d.get_temp::<(Field, u64)>(open_id));
        let open = enabled && live && state.is_some_and(|(field, _)| field == Field::Open);
        let stale = match state {
            Some((Field::Open, _)) => !open,
            Some((Field::Clicked, at)) => !enabled || at + 1 < pass,
            None => false,
        };
        if stale {
            ui.data_mut(|d| d.remove::<(Field, u64)>(open_id));
            ValueField::forget(&ctx, field_id);
        }
        let painter = undimmed_painter(ui);
        if open {
            let mut field = ValueField::new(field_id, self.name, self.value, self.range.clone())
                .decimals(self.decimals)
                .number_only()
                .look(FieldLook {
                    frame: None,
                    ..field_look(&t, false)
                });
            if let Some(unit) = self.unit {
                field = field.unit(unit).suffix(unit);
            }
            let mut place = ui.new_child(UiBuilder::new().max_rect(value_rect));
            let (_, typed) = field.show(&mut place);
            if let Some(typed) = typed {
                shown = self.held(typed);
            }
            // The field's accent hairline while it is typed in, whatever gave it the focus.
            painter.rect_stroke(
                value_rect,
                t.shape.radius,
                egui::Stroke::new(t.shape.hairline, t.colors.focus_ring),
                egui::StrokeKind::Inside,
            );
        } else {
            let value = ui.interact(value_rect, id.with("value"), Sense::click());
            let colour = dimmed(&t, enabled, t.colors.text_strong);
            let written = self.written(&words, shown);
            // To screen readers a button named by the row's label, its value as written:
            // "Interface size, 110 %".
            value.widget_info(|| {
                let mut info = WidgetInfo::labeled(Role::Button, enabled, self.name);
                info.current_text_value = Some(written.clone());
                info
            });
            let galley = ui
                .painter()
                .layout_no_wrap(written.clone(), body(&t), colour);
            painter.galley(
                pos2(
                    value_rect.right() - inset - galley.size().x,
                    value_rect.center().y - galley.size().y / 2.0,
                ),
                galley,
                colour,
            );
            if enabled && value.hovered() {
                ctx.set_cursor_icon(egui::CursorIcon::Text);
            }
            // Tab reaches the value too: it shows the ring, Space or Enter opening it.
            if value.has_focus() {
                super::ring(&painter, value_rect, &t);
            }
        }
        // The track and the thumb, where the thumb has room to travel.
        if travels {
            let deep = t.colors.sunken_deep();
            let radius = track.height() / 2.0;
            painter.rect(
                track,
                radius,
                deep,
                super::edge(&t),
                egui::StrokeKind::Inside,
            );
            let lit = enabled && (response.hovered() || response.dragged());
            let thumb = dimmed(
                &t,
                enabled,
                if lit {
                    t.colors.text
                } else {
                    t.colors.text_secondary
                },
            );
            let at = pos2(x_of(shown), track.center().y);
            painter.rect_filled(Rect::from_center_size(at, THUMB), THUMB.y / 2.0, thumb);
            if response.has_focus() {
                super::ring(&painter, track, &t);
            }
        }
        // Screen readers hear the value as it is written, with the range.
        response.widget_info(|| WidgetInfo::slider(enabled, shown, self.name));
        let heard = self.written(&words, shown);
        let step = self.step;
        ctx.accesskit_node_builder(id, |node| {
            node.set_value(heard);
            node.set_min_numeric_value(min);
            node.set_max_numeric_value(max);
            if step > 0.0 {
                node.set_numeric_value_step(step);
            }
            node.add_action(Action::SetValue);
            node.add_action(Action::Increment);
            node.add_action(Action::Decrement);
        });
        (shown != self.value).then_some(shown)
    }
}
