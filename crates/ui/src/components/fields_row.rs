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

//! The fields row (decision 107; `docs/gui.md`, Line rows and end items): two or three
//! compact value fields on one line, sharing its width, with no label of the line's own,
//! as Export's `[w] × [h] px`.
//!
//! - **One line,** `panel_line` high: each field a [`ValueField`] taking a number only, a
//!   separator before it and a unit after it in the small text in `text_secondary`,
//!   `spacing.s` between the parts, the last unit's ink ending 5 points from the row's end
//!   ([`field_inset`]). Fields side by side that share a unit draw it once, after the last
//!   of them. The fields share what the separators, units and gaps leave, in equal parts.
//! - **Folded,** below its least width ([`FieldsRow::least_width`]), the widest field's at
//!   either end of its range, with its decimals and sign, for each field: one labelled row
//!   per field, its name at the start, cut at its end where it is longer than the room,
//!   and its field at the end, `value_column` wide, its unit after it, and no separator.
//!   The fold follows the row's width alone, never the values, so typing never folds it.
//! - **A field's tooltip** is its own (`RowField::tip`). A disabled field, one whose
//!   setting `--conf` gives, takes no typing, and its reason is its tooltip and its
//!   description to screen readers.
//! - **To screen readers** the row is a group named by its words, folded or not, each
//!   field named by its own name with its unit, "Width, in px". Tab goes left to right,
//!   then top to bottom when folded.

use std::ops::RangeInclusive;

use egui::{Align, FontId, Id, Label, Layout, Response, RichText, Ui, vec2};

use super::{ValueField, field_inset};
use crate::{Theme, theme};

/// One field of a fields row: what it shows, the words round it, and whether it takes
/// typing.
pub struct RowField<'a> {
    name: &'a str,
    value: f64,
    range: RangeInclusive<f64>,
    decimals: usize,
    before: Option<&'a str>,
    unit: Option<&'a str>,
    disabled: Option<&'a str>,
    tip: Option<&'a str>,
}

impl<'a> RowField<'a> {
    /// `name` is the field's full name, translated, and its folded row's label; `value`
    /// is within `range`.
    pub fn new(name: &'a str, value: f64, range: RangeInclusive<f64>) -> Self {
        RowField {
            name,
            value,
            range,
            decimals: 0,
            before: None,
            unit: None,
            disabled: None,
            tip: None,
        }
    }

    /// The decimals it shows.
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// Drawn before it on one line, after the field before: "×", "@".
    pub fn before(mut self, text: &'a str) -> Self {
        self.before = Some(text);
        self
    }

    /// Its unit, translated: in its name to screen readers, and drawn after it.
    pub fn unit(mut self, unit: &'a str) -> Self {
        self.unit = Some(unit);
        self
    }

    /// Drawn disabled, taking no typing, `why` its tooltip and its description.
    pub fn disabled(mut self, why: &'a str) -> Self {
        self.disabled = Some(why);
        self
    }

    /// Its tooltip while it takes typing; a disabled field's is its reason.
    pub fn tip(mut self, text: &'a str) -> Self {
        self.tip = Some(text);
        self
    }

    /// Its value field, the id `id`'s.
    fn field(&self, id: Id) -> ValueField<'a> {
        let field = ValueField::new(id, self.name, self.value, self.range.clone())
            .decimals(self.decimals)
            .number_only();
        match self.unit {
            Some(unit) => field.unit(unit),
            None => field,
        }
    }

    /// Draws its field filling `ui`'s width, the id `id`'s; returns the value typed.
    fn show(&self, ui: &mut Ui, id: Id) -> Option<f64> {
        let Some(why) = self.disabled else {
            let (response, typed) = self.field(id).show(ui);
            if let Some(tip) = self.tip {
                drop(response.on_hover_text(tip));
            }
            return typed;
        };
        ui.add_enabled_ui(false, |ui| {
            let (response, _) = self.field(id).show(ui);
            described(response, why);
        });
        None
    }
}

/// A disabled field's reason, its tooltip and what screen readers hear of it.
fn described(response: Response, why: &str) {
    (response.ctx).accesskit_node_builder(response.id, |node| node.set_description(why));
    drop(response.on_disabled_hover_text(why));
}

pub struct FieldsRow<'a> {
    id: Id,
    name: &'a str,
    fields: &'a [RowField<'a>],
}

impl<'a> FieldsRow<'a> {
    /// `id` must be stable across frames; `name` is the row's words, translated, the
    /// group's name to screen readers.
    pub fn new(id: Id, name: &'a str, fields: &'a [RowField<'a>]) -> Self {
        FieldsRow { id, name, fields }
    }

    /// The width it needs on one line: as many times as it has fields, since they share
    /// the line alike, the widest a field needs at either end of its range with its decimals
    /// and sign, "100000" for Export's width ([`ValueField::width`]), with the separators,
    /// units and gaps and the inset after the last. Narrower, it folds.
    pub fn least_width(&self, ui: &Ui) -> f32 {
        let t = theme(ui.ctx());
        let widest = (self.fields.iter())
            .flat_map(|field| [*field.range.start(), *field.range.end()].map(|end| (field, end)))
            .map(|(field, end)| {
                ValueField::new(self.id, field.name, end, field.range.clone())
                    .decimals(field.decimals)
                    .width(ui)
            })
            .fold(0.0, f32::max);
        widest * self.fields.len() as f32 + self.fixed(ui, &t) + field_inset(&t)
    }

    /// Draws it, on one line or folded as its width decides; returns the field typed in,
    /// by its place, and the value set.
    pub fn show(self, ui: &mut Ui) -> Option<(usize, f64)> {
        let t = theme(ui.ctx());
        let folded = ui.available_width() < self.least_width(ui);
        let mut typed = None;
        let scope = ui.scope(|ui| {
            ui.spacing_mut().item_spacing.x = t.spacing.s;
            ui.set_min_height(t.sizes.panel_line);
            if folded {
                self.folded(ui, &t, &mut typed);
            } else {
                self.line(ui, &t, &mut typed);
            }
            // The row is a group named by its words, the fields inside it.
            let at = ui.min_rect();
            let name = self.name.to_owned();
            ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
                node.set_role(egui::accesskit::Role::Group);
                node.set_label(name);
                node.set_bounds(egui::accesskit::Rect {
                    x0: f64::from(at.left()),
                    y0: f64::from(at.top()),
                    x1: f64::from(at.right()),
                    y1: f64::from(at.bottom()),
                });
            });
        });
        drop(scope);
        typed
    }

    /// The fields on one line, sharing what the words and the gaps leave.
    fn line(&self, ui: &mut Ui, t: &Theme, typed: &mut Option<(usize, f64)>) {
        ui.horizontal(|ui| {
            ui.set_min_height(t.sizes.panel_line);
            let room = ui.available_width() - field_inset(t) - self.fixed(ui, t);
            let each = (room / self.fields.len() as f32).floor().max(8.0);
            for (at, field) in self.fields.iter().enumerate() {
                if let Some(before) = field.before {
                    small(ui, t, before);
                }
                ui.allocate_ui(vec2(each, t.sizes.field), |ui| {
                    if let Some(value) = field.show(ui, self.id.with(at)) {
                        *typed = Some((at, value));
                    }
                });
                if let Some(unit) = self.unit_after(at) {
                    small(ui, t, unit);
                }
            }
        });
    }

    /// One labelled row per field, its unit after it and no separator.
    fn folded(&self, ui: &mut Ui, t: &Theme, typed: &mut Option<(usize, f64)>) {
        for (at, field) in self.fields.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.set_min_height(t.sizes.panel_line);
                let colour = if field.disabled.is_some() {
                    t.colors.text_disabled
                } else {
                    t.colors.text
                };
                // The field's box ends with the row, or its unit's ink 5 points short of it;
                // the label takes the rest, cut at its end where it is longer.
                let unit = field
                    .unit
                    .map_or(0.0, |unit| wide(ui, t, unit) + t.spacing.s + field_inset(t));
                let width = t.sizes.value_column;
                let room = (ui.available_width() - width - unit - t.spacing.s).max(0.0);
                let layout = Layout::left_to_right(Align::Center);
                ui.allocate_ui_with_layout(vec2(room, t.sizes.panel_line), layout, |ui| {
                    ui.set_width(room);
                    let label = RichText::new(field.name).size(t.text.body).color(colour);
                    ui.add(Label::new(label).truncate());
                });
                ui.allocate_ui(vec2(width, t.sizes.field), |ui| {
                    if let Some(value) = field.show(ui, self.id.with(at)) {
                        *typed = Some((at, value));
                    }
                });
                if let Some(unit) = field.unit {
                    small(ui, t, unit);
                }
            });
        }
    }

    /// The unit drawn after the field at `at` on one line: its own, unless the next
    /// field shares it, which draws it after both.
    fn unit_after(&self, at: usize) -> Option<&'a str> {
        let unit = self.fields[at].unit?;
        let next = self.fields.get(at + 1).and_then(|next| next.unit);
        (next != Some(unit)).then_some(unit)
    }

    /// What the words and the gaps between the parts take on one line.
    fn fixed(&self, ui: &Ui, t: &Theme) -> f32 {
        let mut words: Vec<&str> = Vec::new();
        for (at, field) in self.fields.iter().enumerate() {
            words.extend(field.before);
            words.extend(self.unit_after(at));
        }
        let parts = self.fields.len() + words.len();
        let drawn: f32 = words.iter().map(|text| wide(ui, t, text)).sum();
        drawn + (parts.saturating_sub(1)) as f32 * t.spacing.s
    }
}

/// How wide `text` is in the small text.
fn wide(ui: &Ui, t: &Theme, text: &str) -> f32 {
    let font = FontId::proportional(t.text.small);
    (ui.painter())
        .layout_no_wrap(text.to_owned(), font, t.colors.text_secondary)
        .size()
        .x
}

/// `text` in the small text in `text_secondary`, a separator or a unit.
fn small(ui: &mut Ui, t: &Theme, text: &str) {
    ui.label(
        RichText::new(text)
            .size(t.text.small)
            .color(t.colors.text_secondary),
    );
}
