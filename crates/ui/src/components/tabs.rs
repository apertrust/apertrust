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

//! A section's tabs: plain text over a thin line, each tab as wide as its name with a little
//! room at each end, one after another from the row's start, the chosen one SemiBold and
//! underlined (a module's content tabs, a scope's views). When the row is too narrow for
//! them, that room shrinks first, then the names in proportion to their widths, each cut
//! at its end with an ellipsis, on one line. And the small all-caps label that groups rows
//! inside a tab.

use egui::text::LayoutJob;
use egui::{
    Color32, FontFamily, FontId, Rect, Role, Sense, TextFormat, Ui, WidgetInfo, pos2, vec2,
};

use super::{focus_ring, truncated};
use crate::{Theme, fonts, theme};

pub struct Tabs<'a> {
    name: &'a str,
    tabs: &'a [&'a str],
    selected: usize,
    /// The tabs that cannot be chosen, by index, and their tooltip.
    disabled: &'a [usize],
    disabled_tip: &'a str,
    /// How far the row reaches out before the content's edge (see [`Tabs::hang`]).
    hang: f32,
}

/// The line under the chosen tab's name: thin, so the accent stays quiet.
const UNDERLINE: f32 = 1.0;

impl<'a> Tabs<'a> {
    /// `name` is the tab list's name for screen readers; the tabs are translated.
    pub fn new(name: &'a str, tabs: &'a [&'a str], selected: usize) -> Self {
        Tabs {
            name,
            tabs,
            selected,
            disabled: &[],
            disabled_tip: "",
            hang: 0.0,
        }
    }

    /// The row starts `hang` points before the content's edge, where the caller put it, so
    /// that a first tab with that much padding has its name on the edge; the line under
    /// the tabs starts at the edge. A section hangs its tabs by their padding.
    pub fn hang(mut self, hang: f32) -> Self {
        self.hang = hang;
        self
    }

    /// The tabs that cannot be chosen yet, by index, drawn disabled as an action that
    /// cannot run is, with `tip`, translated, as their tooltip.
    pub fn disabled(mut self, tabs: &'a [usize], tip: &'a str) -> Self {
        self.disabled = tabs;
        self.disabled_tip = tip;
        self
    }

    /// Returns the tab the user chose this frame.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let t = theme(ui.ctx());
        // The tab strip is a row high, 26 points.
        let height = t.sizes.row;
        let (row, list) =
            ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        list.widget_info(|| WidgetInfo::labeled(Role::TabList, true, self.name));
        let painter = ui.painter();
        painter.hline(
            row.left() + self.hang..=row.right(),
            row.bottom() - t.shape.hairline / 2.0,
            egui::Stroke::new(t.shape.hairline, t.colors.divider),
        );
        let font = |on: bool| {
            let family = if on {
                FontFamily::Name(fonts::SEMIBOLD.into())
            } else {
                FontFamily::Proportional
            };
            FontId::new(t.text.body, family)
        };
        // Each name is measured as the chosen tab draws it, SemiBold, so choosing another
        // tab moves none of them.
        let name_widths: Vec<f32> = self
            .tabs
            .iter()
            .map(|&name| {
                ui.painter()
                    .layout_no_wrap(name.to_owned(), font(true), Color32::PLACEHOLDER)
                    .size()
                    .x
            })
            .collect();
        // Each tab has its padding at both ends: 8 points while the row has room, the
        // leftover room staying empty after the last tab; less, all tabs alike, down to 4,
        // when it hasn't. Only then do the names shrink, in proportion to their widths, as
        // a dropdown row's label and box share theirs (darktable's `label / (label +
        // value)`).
        let names_width: f32 = name_widths.iter().sum();
        let ends = 2.0 * self.tabs.len() as f32;
        let (padding, scale) = if names_width + ends * t.spacing.xs <= row.width() {
            (((row.width() - names_width) / ends).min(t.spacing.m), 1.0)
        } else {
            (
                t.spacing.xs,
                ((row.width() - ends * t.spacing.xs) / names_width).max(0.0),
            )
        };
        let mut start = row.left();
        let mut chosen = None;
        for (i, (&name, &name_width)) in self.tabs.iter().zip(&name_widths).enumerate() {
            let width = name_width * scale + 2.0 * padding;
            let rect = Rect::from_min_size(pos2(start, row.top()), vec2(width, height));
            start += width;
            let enabled = !self.disabled.contains(&i);
            let sense = if enabled {
                Sense::click()
            } else {
                Sense::hover()
            };
            let response = ui.interact(rect, list.id.with(i), sense);
            let on = i == self.selected;
            let colour = if !enabled {
                t.colors.text_disabled
            } else if on || response.hovered() {
                t.colors.text
            } else {
                t.colors.text_secondary
            };
            // One line: a shrunk tab's name is cut at its end with an ellipsis, and its
            // tooltip says it whole, as screen readers do. A single letter stays whole.
            let room = if scale < 1.0 {
                name_width * scale
            } else {
                f32::INFINITY
            };
            let galley = ui
                .painter()
                .layout_job(truncated(name, font(on), room, colour));
            let elided = galley.elided;
            let label = galley.size();
            ui.painter_at(rect)
                .galley(rect.center() - label / 2.0, galley, colour);
            if on {
                let x = rect.center().x;
                let half = (label.x / 2.0 + t.spacing.xs).min(width / 2.0);
                ui.painter().rect_filled(
                    Rect::from_min_max(
                        pos2(x - half, rect.bottom() - UNDERLINE),
                        pos2(x + half, rect.bottom()),
                    ),
                    0.0,
                    t.colors.accent,
                );
            }
            if response.has_focus() {
                focus_ring(ui.painter(), rect.shrink(2.0), &t);
            }
            response.widget_info(|| WidgetInfo::selected(Role::Tab, enabled, on, name));
            let response = match (enabled, elided) {
                (true, false) => response,
                (true, true) => response.on_hover_text(name),
                (false, false) => response.on_hover_text(self.disabled_tip),
                (false, true) => response.on_hover_text(format!("{name}\n{}", self.disabled_tip)),
            };
            if enabled && response.clicked() && !on {
                chosen = Some(i);
            }
        }
        chosen
    }
}

/// The small all-caps label over a group of rows inside a tab: ILLUMINANT, SHOULDER.
pub fn group_label(ui: &mut Ui, text: &str) {
    let t = theme(ui.ctx());
    ui.add_space(t.spacing.m);
    let colour = t.colors.text_secondary;
    let galley = ui
        .painter()
        .layout_job(group_label_text(&t, text, f32::INFINITY, colour));
    let (rect, response) = ui.allocate_exact_size(
        vec2(ui.available_width(), galley.size().y + t.spacing.xxs),
        Sense::hover(),
    );
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, text));
    ui.painter().galley(rect.min, galley, colour);
}

/// A group label's text, as [`group_label`] and a collapsible group's row draw it: caption
/// sized Medium capitals, a little apart, on one line at most `width` wide, cut at its
/// end with … when longer.
pub(super) fn group_label_text(t: &Theme, text: &str, width: f32, colour: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.append(
        &text.to_uppercase(),
        0.0,
        TextFormat {
            font_id: FontId::new(t.text.caption, FontFamily::Name(fonts::MEDIUM.into())),
            color: colour,
            extra_letter_spacing: 0.55,
            ..Default::default()
        },
    );
    job.wrap.max_width = width;
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job
}
