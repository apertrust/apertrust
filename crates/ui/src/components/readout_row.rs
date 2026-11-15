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

//! The readout row, one line (`docs/gui.md`, How the rows are laid out): a number the
//! photo gives and nobody edits, as darktable's "computed EC" under Exposure's Target
//! level. Its label at the start in `text_secondary`, its value at the end in `text`,
//! where a slider's value ends, written as a slider writes its value at rest, in the
//! language's format with its unit; with no value, nothing at the end. It takes no focus;
//! screen readers hear the label and the value as one label, and its promise is the
//! tooltip.

use egui::accesskit::Role;
use egui::{FontId, Response, Sense, Ui, WidgetInfo, pos2, vec2};

use crate::{theme, translator};

pub struct ReadoutRow<'a> {
    label: &'a str,
    promise: Option<&'a str>,
    value: Option<f64>,
    decimals: usize,
    signed: bool,
    unit: Option<&'a str>,
}

impl<'a> ReadoutRow<'a> {
    /// `label` is the short, translated label; `value` the number, `None` while there is
    /// none to show.
    pub fn new(label: &'a str, value: Option<f64>) -> Self {
        ReadoutRow {
            label,
            promise: None,
            value,
            decimals: 2,
            signed: false,
            unit: None,
        }
    }

    /// The readout's promise, in the tooltip.
    pub fn promise(mut self, text: &'a str) -> Self {
        self.promise = Some(text);
        self
    }

    /// The decimals the value shows.
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// Whether the value shows `+` before zero and above, as a slider's over a range that
    /// spans zero.
    pub fn signed(mut self, signed: bool) -> Self {
        self.signed = signed;
        self
    }

    /// The value's unit, as the id of the message that writes a value with it, such as
    /// `unit-ev-value`: "+0.83 EV".
    pub fn unit(mut self, message: &'a str) -> Self {
        self.unit = Some(message);
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let t = theme(ui.ctx());
        let tr = translator(ui.ctx());
        let (rect, response) = ui.allocate_exact_size(
            vec2(ui.available_width(), t.sizes.panel_line),
            Sense::hover(),
        );
        let value = self.value.map(|value| {
            let number = tr.format_number(value, self.decimals, self.signed);
            match self.unit {
                Some(unit) => {
                    let mut args = crate::i18n::Args::new();
                    args.set("value", number);
                    tr.text_with(unit, Some(&args))
                }
                None => number,
            }
        });
        let font = FontId::proportional(t.text.small);
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            // The value first, never cut; the label cut with … to what it leaves, a
            // choice row's 12-point gap between them. Screen readers hear both whole.
            let end = rect.right() - super::field_inset(&t);
            let value = value
                .as_ref()
                .map(|value| painter.layout_no_wrap(value.clone(), font.clone(), t.colors.text));
            let room = match &value {
                Some(value) => end - value.size().x - t.spacing.l - rect.left(),
                None => rect.width(),
            };
            let label = painter.layout_job(super::truncated(
                self.label,
                font,
                room.max(0.0),
                t.colors.text_secondary,
            ));
            let centred = |height: f32| rect.center().y - height / 2.0;
            painter.galley(
                pos2(rect.left(), centred(label.size().y)),
                label,
                t.colors.text_secondary,
            );
            if let Some(value) = value {
                // Where a slider's value ends at rest, inside its field's inset.
                let at = pos2(end - value.size().x, centred(value.size().y));
                painter.galley(at, value, t.colors.text);
            }
        }
        let heard = match &value {
            Some(value) => format!("{} {value}", self.label),
            None => self.label.to_owned(),
        };
        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &heard));
        match self.promise {
            Some(promise) => response.on_hover_text(promise),
            None => response,
        }
    }
}
