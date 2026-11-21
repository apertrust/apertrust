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

//! An open-ended number in a settings row, or one its schema asks a box of
//! (`control = "number"`): one box `sizes.setting_control` high at the column's end,
//! `sunken` with no frame, − at its start and + at its end, each a box high and wide on
//! `surface`. The value sits
//! right-aligned in `text_strong`, its unit after it in `text_secondary`.
//!
//! - **The buttons** step by its step, Shift ten steps; held with the primary button, each
//!   repeats after 400 ms, then every 60 ms, until it is let go; other buttons do nothing.
//!   A button is drawn disabled at its end of the range.
//! - **Typing:** the value is typed in place, as a tool's value field takes it
//!   ([`ValueField`]); Enter or leaving the field sets it, Esc puts it back. What is typed
//!   or stepped is rounded as it is kept ([`Number::grid`]) before the range is checked.
//! - **Out of range:** while what is typed is out of the range, or after it was set so, the
//!   box is outlined in `error` and the range is said under it in the caption size and
//!   `error` ("256 to 2048 MiB"); the value keeps its last good one. A number that must be
//!   more than a value ([`Number::above`]) refuses that value too ("more than 0 GB"), and
//!   − never steps to it.
//! - **Screen readers:** the field is a text input named "Label, in Unit", the buttons
//!   "Decrease Label" and "Increase Label", the range a label as it shows.

use std::hash::Hash;
use std::ops::RangeInclusive;

use egui::{
    CornerRadius, FontId, Id, Rect, Role, Sense, Stroke, StrokeKind, Ui, UiBuilder, WidgetInfo,
    pos2, vec2,
};

use super::super::plate::undimmed_painter;
use super::super::value_field::ValueField;
use super::super::{FieldLook, field_inset};
use super::{at_end, body, dimmed, field_look, paint_well};
use crate::i18n::Args;
use crate::{Translator, theme, translator};

/// How long a held button waits before it repeats, and then between repeats, in seconds.
const REPEAT_AFTER: f64 = 0.4;
const REPEAT_EVERY: f64 = 0.06;

pub struct Number<'a> {
    id: Id,
    name: &'a str,
    value: f64,
    range: RangeInclusive<f64>,
    /// What it must be more than, an exclusive lower end.
    above: Option<f64>,
    /// The values it is kept at, as shown.
    grid: Option<f64>,
    step: f64,
    decimals: usize,
    unit: Option<&'a str>,
}

/// A button held by the primary button: since when, and when it last stepped.
#[derive(Clone, Copy, Debug)]
struct Held {
    since: f64,
    last: f64,
}

impl<'a> Number<'a> {
    /// `id` names it, stable across frames; `name` is the row's label, translated; `value`
    /// is as shown, and `range` what it may be, an open end the largest number there.
    pub fn new(
        id: impl Hash + std::fmt::Debug,
        name: &'a str,
        value: f64,
        range: RangeInclusive<f64>,
    ) -> Self {
        Number {
            id: Id::unique(("setting number", id)),
            name,
            value,
            range,
            above: None,
            grid: None,
            step: 1.0,
            decimals: 0,
            unit: None,
        }
    }

    /// The values it is kept at, as shown: the store's smallest step times the factor it is
    /// shown with ("0.4" for a tenth shown four times over). What is typed or stepped is
    /// rounded to the nearest, as the store keeps it, before the range is checked.
    pub fn grid(mut self, grid: f64) -> Self {
        self.grid = Some(grid);
        self
    }

    /// What it must be more than, as shown, the schema's `above`: that value and any under
    /// it are refused as out of range.
    pub fn above(mut self, above: f64) -> Self {
        self.above = Some(above);
        self
    }

    /// What − and + step it by.
    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// How many decimals it is shown and set with.
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// Its unit's words, translated, after the value ("MiB").
    pub fn unit(mut self, unit: &'a str) -> Self {
        self.unit = Some(unit);
        self
    }

    /// Whether `value` is within the range, and more than what it must be more than.
    fn holds(&self, value: f64) -> bool {
        self.range.contains(&value) && self.above.is_none_or(|above| value > above)
    }

    /// `value` as it is kept: on its grid, at the decimals shown. The ends lie on the grid
    /// (the schema's generator refuses one that does not).
    fn rounded(&self, value: f64) -> f64 {
        super::on_grid(value, self.grid, self.decimals)
    }

    /// The range in words, "256 to 2048 MiB", "more than 0 GB", an open end left out.
    fn range_words(&self, words: &Translator) -> String {
        let number = |v: f64| words.format_number(v, self.decimals, false);
        let (min, max) = (*self.range.start(), *self.range.end());
        let bound = |v: f64| v.abs() < f64::from(f32::MAX);
        // An exclusive lower end above the minimum is the one said.
        let above = self.above.filter(|above| *above >= min);
        let (from, to) = (number(above.unwrap_or(min)), number(max));
        let mut args = Args::new();
        let id = match (above.is_some(), bound(min), bound(max)) {
            (true, _, true) => {
                args.set("min", from.as_str());
                args.set("max", to.as_str());
                "settings-number-range-above-to"
            }
            (true, _, false) => {
                args.set("min", from.as_str());
                "settings-number-range-above"
            }
            (false, true, true) => {
                args.set("min", from.as_str());
                args.set("max", to.as_str());
                "settings-number-range"
            }
            (false, true, false) => {
                args.set("min", from.as_str());
                "settings-number-range-from"
            }
            _ => {
                args.set("max", to.as_str());
                "settings-number-range-up-to"
            }
        };
        let range = words.text_with(id, Some(&args));
        match self.unit {
            Some(unit) => {
                let mut args = Args::new();
                args.set("value", range.as_str());
                args.set("unit", unit);
                words.text_with("settings-value-with-unit", Some(&args))
            }
            None => range,
        }
    }

    /// Draws it at the end of `ui`'s width; returns the value set this frame.
    pub fn show(self, ui: &mut Ui) -> Option<f64> {
        let ctx = ui.ctx().clone();
        let t = theme(&ctx);
        let words = translator(&ctx);
        let enabled = ui.is_enabled();
        let side = t.sizes.setting_control;
        let inset = field_inset(&t);
        let font = body(&t);
        // The value's room: the range's widest value with its unit, at least two value
        // columns, so it never moves.
        let measure = |text: String| {
            ui.painter()
                .layout_no_wrap(text, font.clone(), t.colors.text)
                .size()
                .x
        };
        let ends = [*self.range.start(), *self.range.end()];
        let widest = ends
            .iter()
            .filter(|v| v.abs() < f64::from(f32::MAX))
            .map(|v| measure(words.format_number(*v, self.decimals, false)))
            .fold(0.0, f32::max);
        let unit = self
            .unit
            .map_or(0.0, |unit| measure(unit.to_owned()) + t.spacing.xs);
        let room = (widest + unit + 2.0 * inset)
            .ceil()
            .max(2.0 * t.sizes.value_column);
        let width = (room + 2.0 * side).min(ui.available_width());
        let mut end = at_end(ui, width, side);
        let (whole, _) = end.allocate_exact_size(vec2(width, side), Sense::hover());
        let painter = undimmed_painter(ui);
        paint_well(&painter, whole, &t);
        let less_rect = Rect::from_min_size(whole.min, vec2(side, side));
        let more_rect =
            Rect::from_min_size(pos2(whole.right() - side, whole.top()), vec2(side, side));
        let field_rect = Rect::from_min_max(
            pos2(less_rect.right() + t.shape.hairline, whole.top()),
            pos2(more_rect.left() - t.shape.hairline, whole.bottom()),
        );
        let id = self.id;
        let field_id = id.with("field");
        let refused_id = id.with("refused");
        // A value changed from outside, as a reset, is a good one: a refusal said of the last
        // goes.
        let last_id = id.with("last");
        if ui.data(|d| d.get_temp::<f64>(last_id)) != Some(self.value) {
            ui.data_mut(|d| {
                d.insert_temp(last_id, self.value);
                d.remove::<bool>(refused_id);
            });
        }
        // So does a box not drawn in the pass before, its page left: it comes back as it is.
        let drawn_id = id.with("drawn");
        let pass = ctx.cumulative_pass_nr();
        let drawn = ui.data(|d| d.get_temp::<u64>(drawn_id));
        if drawn.is_none_or(|drawn| drawn + 1 < pass) {
            ui.data_mut(|d| d.remove::<bool>(refused_id));
        }
        ui.data_mut(|d| d.insert_temp(drawn_id, pass));
        let mut set = None;
        // The buttons, each a step, held repeating.
        let now = ui.input(|i| i.time);
        let (min, max) = (*self.range.start(), *self.range.end());
        // Where a step `by` from `from` lands: an end reached is the end exactly; a step
        // inside the range is rounded to the decimals shown.
        let toward = |from: f64, by: f64| {
            let to = from + by;
            if to <= min {
                min
            } else if to >= max {
                max
            } else {
                self.rounded(to).clamp(min, max)
            }
        };
        // Whether `value` reaches what the number must be more than.
        let below = |value: f64| self.above.is_some_and(|above| value <= above);
        for (rect, sign, glyph, name_id) in [
            (less_rect, -1.0, "−", "settings-number-decrease"),
            (more_rect, 1.0, "+", "settings-number-increase"),
        ] {
            // − stops where a step would reach what the number must be more than.
            let at_its_end = if sign < 0.0 {
                self.value <= min || below(toward(self.value, -self.step))
            } else {
                self.value >= max
            };
            let live = enabled && !at_its_end;
            // Sensing drags too, the button stays egui's from the primary press to its
            // release, however long, past the time a click could come of it.
            let sense = if live {
                Sense::click_and_drag()
            } else {
                Sense::hover()
            };
            let button = ui.interact(rect, id.with(name_id), sense);
            let held_id = button.id.with("held");
            // A release this pass ends a hold, though the button is pressed again by the
            // pass's end: that press starts a new one.
            let released = ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary));
            let was_held = ui.data(|d| d.get_temp::<Held>(held_id));
            let held = was_held.filter(|_| !released);
            let mut step = false;
            // A hold is the primary press's that began on the button, read off where it was
            // pressed: egui gives the button no press that follows a release in one pass,
            // and one pressed elsewhere and moved on is not its own.
            let (begun_here, pressed_now) = ui.input(|i| {
                let here = i.pointer.press_origin().is_some_and(|at| rect.contains(at));
                (
                    here && i.pointer.primary_down(),
                    i.pointer.primary_pressed(),
                )
            });
            let again = released && pressed_now;
            let holding = live && begun_here && (super::pressed(&button) || again);
            // Shift as the event that activated this button came, its press or its key,
            // not as the pass ends, its keyboard as the pass began; held, as it is held now.
            let focused = button.has_focus() || ui.memory(|m| m.had_focus_last_frame(button.id));
            let shift =
                ui.input(|i| super::super::steps::activation_modifiers(i, rect, focused).shift);
            let by = if shift { 10.0 } else { 1.0 } * self.step;
            if holding {
                match held {
                    // A press while the field is typed in commits what is typed, in this
                    // pass, as the field loses the focus, and steps nothing.
                    None if ui.memory(|m| m.has_focus(field_id)) => {
                        ui.memory_mut(|m| m.surrender_focus(field_id));
                        let h = Held {
                            since: now,
                            last: now,
                        };
                        ui.data_mut(|d| d.insert_temp(held_id, h));
                    }
                    None => {
                        step = true;
                        let h = Held {
                            since: now,
                            last: now,
                        };
                        ui.data_mut(|d| d.insert_temp(held_id, h));
                    }
                    Some(h) if now - h.since >= REPEAT_AFTER && now - h.last >= REPEAT_EVERY => {
                        step = true;
                        ui.data_mut(|d| d.insert_temp(held_id, Held { last: now, ..h }));
                    }
                    Some(_) => {}
                }
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(REPEAT_EVERY));
            } else {
                // A click with no hold, from a key, steps once; the release of a hold does
                // not step again.
                step = live && button.clicked() && was_held.is_none();
                ui.data_mut(|d| d.remove::<Held>(held_id));
            }
            if step {
                let from = set.unwrap_or(self.value);
                let to = toward(from, sign * by);
                // Ten steps that would reach what it must be more than take one, if one
                // does not.
                let one = toward(from, sign * self.step);
                set = Some(match (below(to), below(one)) {
                    (true, false) => one,
                    (true, true) => from,
                    _ => to,
                });
                ui.data_mut(|d| d.remove::<bool>(refused_id));
            }
            // Its plate and its glyph.
            let corners = if sign < 0.0 {
                CornerRadius {
                    nw: t.shape.radius as u8,
                    sw: t.shape.radius as u8,
                    ..CornerRadius::ZERO
                }
            } else {
                CornerRadius {
                    ne: t.shape.radius as u8,
                    se: t.shape.radius as u8,
                    ..CornerRadius::ZERO
                }
            };
            let fill = if holding {
                t.colors.button_pressed
            } else {
                t.colors.surface
            };
            painter.rect_filled(rect, corners, fill);
            let colour = dimmed(&t, live, t.colors.text_secondary);
            let galley = painter.layout_no_wrap(glyph.to_owned(), font.clone(), colour);
            painter.galley(rect.center() - galley.size() / 2.0, galley, colour);
            if button.has_focus() {
                super::ring(&painter, rect, &t);
            }
            let mut args = Args::new();
            args.set("control", self.name);
            let name = words.text_with(name_id, Some(&args));
            button.widget_info(|| WidgetInfo::labeled(Role::Button, live, &name));
        }
        // The value, typed in place; out of the range it is refused, not clamped.
        let mut field = ValueField::new(field_id, self.name, self.value, f64::MIN..=f64::MAX)
            .decimals(self.decimals)
            .number_only()
            .look(FieldLook {
                fill: None,
                frame: None,
                ..field_look(&t, false)
            });
        if let Some(unit) = self.unit {
            field = field.unit(unit).suffix(unit);
        }
        let mut place = ui.new_child(UiBuilder::new().max_rect(field_rect));
        let (response, typed) = field.show(&mut place);
        if response.has_focus() {
            ui.data_mut(|d| d.remove::<bool>(refused_id));
        }
        if let Some(typed) = typed {
            let typed = self.rounded(typed);
            if self.holds(typed) {
                set = Some(typed);
            } else {
                // Refused, the value stays as it was: a step in the same pass goes too.
                set = None;
                ui.data_mut(|d| d.insert_temp(refused_id, true));
            }
        }
        let draft = ui
            .data(|d| d.get_temp::<String>(field_id.with("draft")))
            .and_then(|draft| words.parse_number(&draft));
        let refused = ui.data(|d| d.get_temp::<bool>(refused_id)).unwrap_or(false);
        let out = enabled && (refused || draft.is_some_and(|v| !self.holds(self.rounded(v))));
        if out {
            painter.rect_stroke(
                whole,
                t.shape.radius,
                Stroke::new(t.shape.focus_width, t.colors.error),
                StrokeKind::Inside,
            );
            let said = self.range_words(&words);
            let galley = painter.layout_no_wrap(
                said.clone(),
                FontId::proportional(t.text.caption),
                t.colors.error,
            );
            let (line, _) = ui.allocate_exact_size(
                vec2(ui.available_width(), galley.size().y + t.spacing.xxs),
                Sense::hover(),
            );
            let at = pos2(
                whole.right() - galley.size().x,
                line.bottom() - galley.size().y,
            );
            let rect = Rect::from_min_size(at, galley.size());
            painter.galley(at, galley, t.colors.error);
            let note = ui.interact(rect, id.with("range"), Sense::hover());
            note.widget_info(|| WidgetInfo::labeled(Role::Label, true, &said));
            let polite = egui::accesskit::Live::Polite;
            ctx.accesskit_node_builder(note.id, |node| node.set_live(polite));
        } else if response.has_focus() {
            super::ring(&painter, whole, &t);
        }
        set.filter(|value| *value != self.value)
    }
}
