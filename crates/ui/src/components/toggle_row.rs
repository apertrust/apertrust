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

//! A boolean's row, one line (`docs/gui.md`, Switches and boolean rows): the short label at
//! the content's start, a photo's value after it in `text_secondary`, and a quiet
//! [`Switch`](super::Switch) at the row's end, its right edge where a slider's value and a
//! choice's chevron end. The whole row is the target, with the `hover` band a choice row
//! has, and the focus ring goes round the whole row, as a choice row's does. Screen readers hear a `Switch` with its checked state, named by the full name and
//! the value; its promise is the tooltip.
//!
//! The value follows the label a small gap (`spacing.xs`) apart, no brackets, never cut:
//! "Compensate camera exposure −⅔ EV". The label is cut with … before it, and the words
//! end a group's gap (`spacing.m`) before the switch.
//!
//! End items (decision 107), as a line row's, sit before the switch, `spacing.xs` from it
//! and from each other, in reading and Tab order after the row: a press on one is its own,
//! never the row's toggle, and the words end a group's gap before the first.

use egui::text::{LayoutJob, TextFormat};
use egui::{Align, Color32, FontId, Layout, Rect, Role, Ui, UiBuilder, WidgetInfo, pos2, vec2};

use super::focus_ring_centred;
use super::line_row::{EndAction, EndItem, end_width, show_end};
use super::plate::{Touch, allocate, undimmed_painter};
use super::switch;
use crate::theme;

pub struct ToggleRow<'a> {
    label: &'a str,
    name: &'a str,
    value: Option<&'a str>,
    promise: Option<&'a str>,
    on: bool,
    enabled: bool,
    end: Vec<EndItem<'a>>,
}

impl<'a> ToggleRow<'a> {
    /// `label` is the short, translated label; the full name defaults to it.
    pub fn new(label: &'a str, on: bool) -> Self {
        ToggleRow {
            label,
            name: label,
            value: None,
            promise: None,
            on,
            enabled: true,
            end: Vec::new(),
        }
    }

    /// The control's full name, what screen readers say.
    pub fn name(mut self, full: &'a str) -> Self {
        self.name = full;
        self
    }

    /// A value after the label, in `text_secondary`, translated, with no brackets, as
    /// Exposure's "−⅔ EV"; screen readers hear it after the full name.
    pub fn value(mut self, value: &'a str) -> Self {
        self.value = Some(value);
        self
    }

    /// The control's promise, in the tooltip.
    pub fn promise(mut self, text: &'a str) -> Self {
        self.promise = Some(text);
        self
    }

    /// A disabled row keeps its switch in place, drawn in `text_disabled` with its label,
    /// and ignores input: a control that can change nothing on the photo.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// End items before the switch, left to right (decision 107): each enabled or not as
    /// it says, whatever the row's own state.
    pub fn end(mut self, items: Vec<EndItem<'a>>) -> Self {
        self.end = items;
        self
    }

    /// Returns the new value when the user changes it.
    pub fn show(self, ui: &mut Ui) -> Option<bool> {
        self.show_with_end(ui).0
    }

    /// Returns the new value when the user changes it, and the end item pressed this frame,
    /// by its index, with how.
    pub fn show_with_end(mut self, ui: &mut Ui) -> (Option<bool>, Option<(usize, EndAction)>) {
        let t = theme(ui.ctx());
        let enabled = self.enabled && ui.is_enabled();
        let size = vec2(ui.available_width(), t.sizes.panel_line);
        let (row, mut response) = allocate(ui, size, enabled);
        // Where the switch sits: its right edge a field's inset from the row's end.
        let track = egui::Rect::from_center_size(
            pos2(
                row.right() - super::field_inset(&t) - t.sizes.switch_width / 2.0,
                row.center().y,
            ),
            vec2(t.sizes.switch_width, t.sizes.switch_height),
        );
        // The end items before the switch, `spacing.xs` from it.
        let items = std::mem::take(&mut self.end);
        let ends = (!items.is_empty()).then(|| {
            let width = end_width(ui, &items);
            let right = track.left() - t.spacing.xs;
            Rect::from_min_max(pos2(right - width, row.top()), pos2(right, row.bottom()))
        });
        // A press on the end items is theirs, even on one disabled, which takes none.
        let on_ends = (response.interact_pointer_pos())
            .is_some_and(|at| ends.is_some_and(|ends| ends.contains(at)));
        let mut on = self.on;
        if response.clicked() && !on_ends {
            on = !on;
            response.mark_changed();
        }
        let font = FontId::proportional(t.text.body);
        let (text, secondary) = if enabled {
            (t.colors.text, t.colors.text_secondary)
        } else {
            (t.colors.text_disabled, t.colors.text_disabled)
        };
        let value = self.value.map(|value| {
            let mut job = LayoutJob::default();
            job.append(
                value,
                t.spacing.xs,
                TextFormat::simple(font.clone(), secondary),
            );
            ui.painter().layout_job(job)
        });
        let words_end = ends.map_or(track.left(), |ends| ends.left());
        // The words end a group's gap before the switch, or the first end item; the value
        // is never cut, the label before it is, with …
        let room = (words_end - t.spacing.m - row.left()).max(0.0);
        let room = room - value.as_ref().map_or(0.0, |v| v.size().x);
        let label = ui.painter().layout_job(super::truncated(
            self.label,
            font,
            room.max(0.0),
            Color32::PLACEHOLDER,
        ));
        let cut = label.elided;
        if ui.is_rect_visible(row) {
            let painter = &undimmed_painter(ui);
            // The band a choice row has, under the whole row.
            if enabled && response.hovered() {
                painter.rect_filled(
                    row.expand2(vec2(t.spacing.xs, 0.0)),
                    t.shape.radius,
                    t.colors.hover,
                );
            }
            let touch = Touch::of(ui.ctx(), &response, enabled, None);
            let mut x = row.left();
            let middle = row.center().y;
            let end = x + label.size().x;
            painter.galley(pos2(x, middle - label.size().y / 2.0), label, text);
            x = end;
            if let Some(value) = value {
                painter.galley(pos2(x, middle - value.size().y / 2.0), value, secondary);
            }
            // The ring goes round the whole row, as a choice row's does, not the track.
            switch::paint(
                painter,
                track,
                t.sizes.switch_knob,
                on,
                Touch {
                    focused: false,
                    ..touch
                },
                &t,
            );
            if touch.focused {
                focus_ring_centred(painter, row.expand2(vec2(t.spacing.xs, 0.0)), &t);
            }
        }
        // Heard whole, with its value, as a switch with its state.
        let name = match self.value {
            Some(value) => format!("{} {value}", self.name),
            None => self.name.to_owned(),
        };
        response.widget_info(|| WidgetInfo::selected(Role::Switch, enabled, on, &name));
        // A label cut short is whole in its tooltip, with its value, before the promise.
        let whole =
            (self.value).map_or_else(|| self.label.to_owned(), |v| format!("{} {v}", self.label));
        let tip: Vec<&str> = [cut.then_some(whole.as_str()), self.promise]
            .into_iter()
            .flatten()
            .collect();
        let response = if tip.is_empty() {
            response
        } else {
            let tip = tip.join("\n");
            response.on_hover_text(&tip).on_disabled_hover_text(&tip)
        };
        let toggled = (response.changed() && on != self.on).then_some(on);
        // Drawn after the row, so a press on one is its own.
        let pressed = ends.and_then(|ends| {
            let mut inside = ui.new_child(
                UiBuilder::new()
                    .max_rect(ends)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            show_end(&mut inside, items)
        });
        (toggled, pressed)
    }
}
