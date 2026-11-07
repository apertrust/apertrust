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

//! The button row, one line (`docs/gui.md`, How the rows are laid out): a control's label
//! with darktable's icon buttons inline at the row's end, as orientation's transform row
//! has its rotate and flip buttons. The label is darktable's reset label: a double-click,
//! or Delete while it has the focus, resets the control. Each button is an `IconButton`, named for screen readers and in
//! its tooltip by its full name.

use egui::{Key, Label, Sense, Ui};

use super::IconButton;
use crate::{Icon, theme};

/// What the user did on a button row this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonsAction {
    /// The label double-clicked: the control back to its default.
    Reset,
    /// The button at this index pressed.
    Pressed(usize),
}

pub struct ButtonsRow<'a> {
    label: &'a str,
    name: &'a str,
    promise: Option<&'a str>,
    buttons: &'a [(Icon, &'a str)],
    enabled: bool,
}

impl<'a> ButtonsRow<'a> {
    /// `label` is the control's short, translated label; `buttons` each button's icon and
    /// its translated full name.
    pub fn new(label: &'a str, buttons: &'a [(Icon, &'a str)]) -> Self {
        ButtonsRow {
            label,
            name: label,
            promise: None,
            buttons,
            enabled: true,
        }
    }

    /// The control's full name, what screen readers say of the label.
    pub fn name(mut self, full: &'a str) -> Self {
        self.name = full;
        self
    }

    /// The control's promise, the label's tooltip.
    pub fn promise(mut self, text: &'a str) -> Self {
        self.promise = Some(text);
        self
    }

    /// A disabled row is dimmed and ignores input, as on a photo open for viewing only.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Option<ButtonsAction> {
        let t = theme(ui.ctx());
        let mut action = None;
        ui.horizontal(|ui| {
            ui.set_min_height(t.sizes.panel_line);
            let label = ui.add_enabled(
                self.enabled,
                Label::new(self.label)
                    .sense(Sense::click())
                    .selectable(false),
            );
            let name = self.name;
            ui.ctx()
                .accesskit_node_builder(label.id, |node| node.set_label(name));
            let label = match self.promise {
                Some(promise) => label.on_hover_text(promise).on_disabled_hover_text(promise),
                None => label,
            };
            // A double-click, or Delete or Backspace while it has the focus, as a slider's
            // label resets its control.
            let keyed = label.has_focus()
                && ui.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace));
            if self.enabled && (label.double_clicked() || keyed) {
                action = Some(ButtonsAction::Reset);
            }
            // At the row's end, left to right, so readers and Tab meet them in order.
            let side = t.sizes.control;
            let count = self.buttons.len() as f32;
            let width = count * side + (count - 1.0).max(0.0) * ui.spacing().item_spacing.x;
            ui.add_space((ui.available_width() - width).max(0.0));
            for (index, &(icon, name)) in self.buttons.iter().enumerate() {
                let pressed = IconButton::new(icon, name)
                    .enabled(self.enabled)
                    .size(side)
                    .show(ui)
                    .clicked();
                if pressed && self.enabled {
                    action = Some(ButtonsAction::Pressed(index));
                }
            }
        });
        action
    }
}
