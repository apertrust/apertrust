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

//! A choice of two to four short options in a settings row: a well the column wide and
//! `sizes.setting_control` high, `sunken` with no frame, the options sharing its
//! width. The chosen one is filled `raised` in `text_strong`, the others in
//! `text_secondary`, with no plate under the pointer. To screen readers a radio group named by
//! the row's label, each option a radio button.

use egui::{Color32, Rect, Role, Sense, Ui, UiBuilder, WidgetInfo, pos2, vec2};

use super::super::plate::undimmed_painter;
use super::{body, dimmed, paint_well, ring};
use crate::theme;

pub struct Segmented<'a> {
    name: &'a str,
    options: &'a [&'a str],
    selected: Option<usize>,
}

impl<'a> Segmented<'a> {
    /// `name` is the row's label and `options` the options' words, translated.
    pub fn new(name: &'a str, options: &'a [&'a str], selected: Option<usize>) -> Self {
        Segmented {
            name,
            options,
            selected,
        }
    }

    /// Draws it across `ui`'s width; returns the option chosen this frame.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let t = theme(ui.ctx());
        let enabled = ui.is_enabled();
        let size = vec2(ui.available_width(), t.sizes.setting_control);
        let (well, group) = ui.allocate_exact_size(size, Sense::hover());
        group.widget_info(|| WidgetInfo::labeled(Role::RadioGroup, enabled, self.name));
        let painter = undimmed_painter(ui);
        paint_well(&painter, well, &t);
        let pad = t.spacing.xxs;
        let count = self.options.len().max(1) as f32;
        let inner = well.shrink(pad);
        let segment = inner.width() / count;
        // The options are the group's, for screen readers.
        let inside = ui.new_child(
            UiBuilder::new()
                .max_rect(well)
                .accessibility_parent(group.id),
        );
        let mut chosen = None;
        for (i, option) in self.options.iter().enumerate() {
            let rect = Rect::from_min_size(
                pos2(inner.left() + i as f32 * segment, inner.top()),
                vec2(segment, inner.height()),
            );
            let sense = if enabled {
                Sense::click()
            } else {
                Sense::hover()
            };
            let response = inside.interact(rect, group.id.with(i), sense);
            let on = self.selected == Some(i);
            if on {
                painter.rect_filled(rect, t.shape.radius_small, t.colors.raised);
            }
            let colour = dimmed(
                &t,
                enabled,
                if on {
                    t.colors.text_strong
                } else {
                    t.colors.text_secondary
                },
            );
            let room = rect.width() - 2.0 * t.spacing.xs;
            let words = painter.layout_job(super::super::truncated(
                option,
                body(&t),
                room.max(0.0),
                Color32::PLACEHOLDER,
            ));
            let at = rect.center() - words.size() / 2.0;
            painter.galley(at, words, colour);
            if response.has_focus() {
                ring(&painter, rect, &t);
            }
            response.widget_info(|| WidgetInfo::selected(Role::RadioButton, enabled, on, option));
            if response.clicked() && enabled && !on {
                chosen = Some(i);
            }
        }
        chosen
    }
}
