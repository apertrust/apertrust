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

//! The checkbox row, one line (`docs/gui.md`, How the rows are laid out): the design
//! system's [`Checkbox`] at the start, its short label after it, which toggles it too.
//! Screen readers hear its full name; its promise is the tooltip. A value of the photo's
//! may follow the label, in `text_secondary`: "Compensate camera exposure (−⅔ EV)".

use egui::Ui;

use super::Checkbox;
use crate::theme;

pub struct ToggleRow<'a> {
    label: &'a str,
    name: &'a str,
    value: Option<&'a str>,
    promise: Option<&'a str>,
    on: bool,
    enabled: bool,
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
        }
    }

    /// The control's full name, what screen readers say.
    pub fn name(mut self, full: &'a str) -> Self {
        self.name = full;
        self
    }

    /// A value after the label, in `text_secondary`, translated with its brackets, as
    /// Exposure's "(−⅔ EV)"; screen readers hear it after the full name.
    pub fn value(mut self, value: &'a str) -> Self {
        self.value = Some(value);
        self
    }

    /// The control's promise, in the tooltip.
    pub fn promise(mut self, text: &'a str) -> Self {
        self.promise = Some(text);
        self
    }

    /// A disabled toggle keeps its tick, drawn in `text_disabled` with its label, and
    /// ignores input: a control that can change nothing on the photo.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Returns the new value when the user changes it.
    pub fn show(self, ui: &mut Ui) -> Option<bool> {
        let t = theme(ui.ctx());
        let mut on = self.on;
        let (response, cut) = ui
            .horizontal(|ui| {
                ui.set_min_height(t.sizes.panel_line);
                let mut checkbox = Checkbox::new(&mut on, self.label).enabled(self.enabled);
                if let Some(value) = self.value {
                    checkbox = checkbox.value(value);
                }
                checkbox.show_cut(ui)
            })
            .inner;
        let name = match self.value {
            Some(value) => format!("{} {value}", self.name),
            None => self.name.to_owned(),
        };
        ui.ctx()
            .accesskit_node_builder(response.id, |node| node.set_label(name));
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
        (response.changed() && on != self.on).then_some(on)
    }
}
