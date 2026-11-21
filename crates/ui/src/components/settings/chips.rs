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

//! A list of a choice in a settings row, as the log's categories: a toggle chip per option,
//! `sizes.control` high (22), wrapping in a well the column wide, `sunken` with no frame,
//! 3 points in from its edge and `spacing.xs` between chips. An on chip is
//! `raised` in `text_strong`; an off one has no fill and `text_secondary`, under the
//! pointer too. A chip's words are cut at their end where the well cannot hold them, whole
//! in its tooltip. A click or Space turns one on or off. To screen readers the well is a
//! group named by the row's label, each chip a toggle button with its state.

use egui::{Color32, Rect, Role, Sense, Ui, UiBuilder, WidgetInfo, pos2, vec2};

use super::super::plate::undimmed_painter;
use super::super::truncated;
use super::{body, dimmed, paint_well, ring};
use crate::theme;

/// How far the chips sit in from the well's edge.
const WELL_INSET: f32 = 3.0;

pub struct Chips<'a> {
    name: &'a str,
    options: &'a [&'a str],
    on: &'a [bool],
}

impl<'a> Chips<'a> {
    /// `name` is the row's label and `options` the options' words, translated, in their
    /// type's order; `on` says which are on, option by option.
    pub fn new(name: &'a str, options: &'a [&'a str], on: &'a [bool]) -> Self {
        Chips { name, options, on }
    }

    /// Draws them across `ui`'s width; returns the option turned on or off this frame, and
    /// whether it is now on.
    pub fn show(self, ui: &mut Ui) -> Option<(usize, bool)> {
        let t = theme(ui.ctx());
        let enabled = ui.is_enabled();
        let width = ui.available_width();
        let high = t.sizes.control;
        let gap = t.spacing.xs;
        let pad = t.spacing.m;
        let painter = undimmed_painter(ui);
        // Each chip's words, in the placeholder colour so each is painted in its state's, cut
        // at their end where a chip as wide as the well cannot hold them.
        let room = width - 2.0 * WELL_INSET;
        let words: Vec<_> = self
            .options
            .iter()
            .map(|option| {
                let job = truncated(option, body(&t), room - 2.0 * pad, Color32::PLACEHOLDER);
                painter.layout_job(job)
            })
            .collect();
        // Where each chip goes, a line at a time, wrapping where the next does not fit.
        let (mut x, mut y) = (0.0, 0.0);
        let mut places = Vec::with_capacity(words.len());
        for galley in &words {
            let chip = (galley.size().x + 2.0 * pad).min(room);
            if x > 0.0 && x + chip > room {
                x = 0.0;
                y += high + gap;
            }
            places.push((x, y, chip));
            x += chip + gap;
        }
        let height = y + high + 2.0 * WELL_INSET;
        let (well, group) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        group.widget_info(|| WidgetInfo::labeled(Role::Group, enabled, self.name));
        paint_well(&painter, well, &t);
        // The chips are the group's, for screen readers.
        let inside = ui.new_child(
            UiBuilder::new()
                .max_rect(well)
                .accessibility_parent(group.id),
        );
        let origin = well.min + vec2(WELL_INSET, WELL_INSET);
        let mut toggled = None;
        for (i, ((x, y, chip), galley)) in places.into_iter().zip(words).enumerate() {
            let rect = Rect::from_min_size(origin + vec2(x, y), vec2(chip, high));
            let sense = if enabled {
                Sense::click()
            } else {
                Sense::hover()
            };
            let response = inside.interact(rect, group.id.with(i), sense);
            let on = self.on.get(i).copied().unwrap_or(false);
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
            let at = pos2(
                rect.center().x - galley.size().x / 2.0,
                rect.center().y - galley.size().y / 2.0,
            );
            let cut = galley.elided;
            painter.galley(at, galley, colour);
            if response.has_focus() {
                ring(&painter, rect, &t);
            }
            let option = self.options[i];
            response.widget_info(|| WidgetInfo::selected(Role::Button, enabled, on, option));
            let clicked = response.clicked();
            if cut {
                response.on_hover_text(option);
            }
            if enabled && clicked {
                toggled = Some((i, !on));
            }
        }
        toggled
    }
}
