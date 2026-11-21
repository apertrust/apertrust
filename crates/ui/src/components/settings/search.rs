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

//! Find a setting, at the top of the Settings dialog's page list: a field
//! `sizes.setting_control` high, `sunken` with no frame, a magnifier at its
//! start in `text_secondary` and its name as its placeholder. Every edit is reported with
//! the whole text, so the list and the rows follow each letter. Escape in it clears it while
//! it holds anything, keeping the focus; once it is empty, Escape in it is the dialog's. To
//! screen readers a text input named by its name.

use std::hash::Hash;

use egui::{Id, Key, Rect, Ui, pos2, vec2};

use super::super::{FieldLook, TextField, field_inset};
use super::field_look;
use crate::{Icon, theme};

pub struct Search<'a> {
    id: Id,
    name: &'a str,
    text: &'a str,
}

impl<'a> Search<'a> {
    /// `id` names it, stable across frames; `name` is its name and placeholder,
    /// translated; `text` what it holds.
    pub fn new(id: impl Hash + std::fmt::Debug, name: &'a str, text: &'a str) -> Self {
        Search {
            id: Search::id_of(id),
            name,
            text,
        }
    }

    /// The id of the field the search `id` draws, to give it the focus, as Ctrl+F does.
    pub fn id_of(id: impl Hash + std::fmt::Debug) -> Id {
        Id::unique(("setting search", id))
    }

    /// Draws it across `ui`'s width; returns the whole text once the user changed it, and
    /// an empty one as Escape clears it.
    pub fn show(self, ui: &mut Ui) -> Option<String> {
        let t = theme(ui.ctx());
        let side = Icon::Search.size().min(t.sizes.icon);
        // egui lets go of the focus on Escape as the frame begins: ask about the last one.
        let cleared = !self.text.is_empty()
            && ui.memory(|m| m.had_focus_last_frame(self.id))
            && ui.input(|i| i.key_pressed(Key::Escape));
        let look = FieldLook {
            lead: side + t.spacing.xs,
            ..field_look(&t, false)
        };
        let shown = if cleared { "" } else { self.text };
        let (response, typed) = TextField::new(self.id, self.name, shown)
            .width(ui.available_width())
            .hint(self.name)
            .look(look)
            .show(ui);
        let at = pos2(
            response.rect.left() + field_inset(&t),
            response.rect.center().y - side / 2.0,
        );
        let colour = if ui.is_enabled() {
            t.colors.text_secondary
        } else {
            t.colors.text_disabled
        };
        Icon::Search.paint(
            ui.painter(),
            Rect::from_min_size(at, vec2(side, side)),
            colour,
        );
        if cleared {
            ui.memory_mut(|m| m.request_focus(self.id));
            return Some(String::new());
        }
        typed
    }
}
