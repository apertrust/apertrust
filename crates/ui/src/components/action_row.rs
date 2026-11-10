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

//! The action row, one line (`docs/gui.md`, How the rows are laid out): one of darktable's
//! buttons in a module, an action that writes the module's controls and is never stored,
//! as AgX's Auto tune levels and Set from above. The design system's [`Button`] at the
//! start, holding the short label; screen readers hear its full name, and its promise is
//! the tooltip. Until its tool comes a module's action is drawn disabled.

use egui::Ui;

use super::Button;
use crate::theme;

pub struct ActionRow<'a> {
    label: &'a str,
    name: &'a str,
    promise: Option<&'a str>,
    enabled: bool,
}

impl<'a> ActionRow<'a> {
    /// `label` is the short, translated label; the full name defaults to it.
    pub fn new(label: &'a str) -> Self {
        ActionRow {
            label,
            name: label,
            promise: None,
            enabled: true,
        }
    }

    /// The action's full name, what screen readers say.
    pub fn name(mut self, full: &'a str) -> Self {
        self.name = full;
        self
    }

    /// The action's promise, in the tooltip.
    pub fn promise(mut self, text: &'a str) -> Self {
        self.promise = Some(text);
        self
    }

    /// A disabled action keeps its plate, its words in `text_disabled`, and ignores input:
    /// one whose tool has not come, or that cannot run now.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Returns true when the user runs the action this frame.
    pub fn show(self, ui: &mut Ui) -> bool {
        let t = theme(ui.ctx());
        let response = ui
            .horizontal(|ui| {
                ui.set_min_height(t.sizes.panel_line);
                Button::new(self.label).enabled(self.enabled).show(ui)
            })
            .inner;
        let name = self.name;
        ui.ctx()
            .accesskit_node_builder(response.id, |node| node.set_label(name));
        let response = match self.promise {
            Some(promise) => response
                .on_hover_text(promise)
                .on_disabled_hover_text(promise),
            None => response,
        };
        self.enabled && response.clicked()
    }
}
