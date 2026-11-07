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

//! A few options side by side, one of them chosen: Linear | Log, a scope's colour space.
//! In a bar each option is as wide as the widest; in a module the row is two lines
//! (`docs/gui.md`, How the rows are laid out), the label on a line of its own and the
//! options sharing the full width under it.

use egui::{FontId, Rect, Role, Sense, Stroke, StrokeKind, Ui, WidgetInfo, pos2, vec2};

use super::{focus_ring, truncated};
use crate::theme;

pub struct Segmented<'a> {
    name: &'a str,
    options: &'a [&'a str],
    selected: usize,
    label: Option<&'a str>,
}

impl<'a> Segmented<'a> {
    /// `name` says what the options choose, for screen readers; the options are
    /// translated.
    pub fn new(name: &'a str, options: &'a [&'a str], selected: usize) -> Self {
        Segmented {
            name,
            options,
            selected,
            label: None,
        }
    }

    /// In a module: the control's short, translated label on a line of its own, and the
    /// options across the full width under it.
    pub fn label(mut self, short: &'a str) -> Self {
        self.label = Some(short);
        self
    }

    /// Returns the option the user chose this frame.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let t = theme(ui.ctx());
        let font = FontId::proportional(t.text.small);
        let pad = t.spacing.xxs;
        let galleys: Vec<_> = self
            .options
            .iter()
            .map(|o| {
                ui.painter()
                    .layout_no_wrap((*o).to_owned(), font.clone(), t.colors.text)
            })
            .collect();
        let count = self.options.len() as f32;
        let height = t.sizes.control - 2.0;
        let (outer, group) = match self.label {
            Some(label) => {
                let (row, group) = ui.allocate_exact_size(
                    vec2(ui.available_width(), 2.0 * t.sizes.row),
                    Sense::hover(),
                );
                let line = Rect::from_min_size(row.min, vec2(row.width(), t.sizes.row));
                let galley = ui.painter().layout_job(truncated(
                    label,
                    FontId::proportional(t.text.body),
                    line.width(),
                    t.colors.text,
                ));
                let at = pos2(line.left(), line.center().y - galley.size().y / 2.0);
                ui.painter().galley(at, galley, t.colors.text);
                let outer = Rect::from_center_size(
                    pos2(row.center().x, line.bottom() + t.sizes.row / 2.0),
                    vec2(row.width(), height + 2.0 * pad),
                );
                (outer, group)
            }
            None => {
                let segment =
                    galleys.iter().map(|g| g.size().x).fold(0.0, f32::max) + 2.0 * t.spacing.m;
                let size = vec2(segment * count + pad * (count + 1.0), height + 2.0 * pad);
                ui.allocate_exact_size(size, Sense::hover())
            }
        };
        let segment = (outer.width() - pad * (count + 1.0)) / count;
        group.widget_info(|| WidgetInfo::labeled(Role::RadioGroup, true, self.name));
        ui.painter()
            .rect_filled(outer, t.shape.radius_large, t.colors.sunken);
        let mut chosen = None;
        for (i, galley) in galleys.into_iter().enumerate() {
            let rect = Rect::from_min_size(
                pos2(
                    outer.left() + pad + i as f32 * (segment + pad),
                    outer.top() + pad,
                ),
                vec2(segment, height),
            );
            let response = ui.interact(rect, group.id.with(i), Sense::click());
            let on = i == self.selected;
            if on {
                ui.painter().rect(
                    rect,
                    t.shape.radius,
                    t.colors.panel,
                    Stroke::new(t.shape.hairline, t.colors.border),
                    StrokeKind::Inside,
                );
            } else if response.hovered() {
                ui.painter()
                    .rect_filled(rect, t.shape.radius, t.colors.hover);
            }
            let colour = if on {
                t.colors.text
            } else {
                t.colors.text_secondary
            };
            ui.painter()
                .galley(rect.center() - galley.size() / 2.0, galley, colour);
            if response.has_focus() {
                focus_ring(ui.painter(), rect, &t);
            }
            let option = self.options[i];
            response.widget_info(|| WidgetInfo::selected(Role::RadioButton, true, on, option));
            if response.clicked() && !on {
                chosen = Some(i);
            }
        }
        chosen
    }
}
