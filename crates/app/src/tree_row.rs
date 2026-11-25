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

//! One row of a tree on the Tools panel, as Collections' Folders and the Styles tool draw
//! theirs (`docs/gui.md`, Names): a `panel_line` high, set in by its level, a chevron where
//! it opens and closes, or a mark in the chevron's column, then an optional icon, its name
//! cut in its middle, and an optional count at its end; a band while hovered or chosen, a
//! focus ring while focused. To screen readers an element of the role given, at its level,
//! open or closed where it has a chevron.

use apertrust_ui::components::{focus_ring_centred, middle_cut};
use apertrust_ui::{Icon, theme, translator};
use eframe::egui::{self, Color32, Rect, Sense, pos2, vec2};

/// How far each level of a tree is set in.
pub const LEVEL_INDENT: f32 = 12.0;

/// The side of a tree row's icon.
pub const ROW_ICON: f32 = 16.0;

/// A row as its tree gives it.
pub struct TreeRow<'a> {
    /// Its id, stable across frames, where one is asked for, as the row a key moves the
    /// focus to; else the one its place gives it.
    pub id: Option<egui::Id>,
    /// 1 for the tree's top.
    pub level: usize,
    pub name: &'a str,
    /// Some for a row that opens (true) or closes (false) what is under it.
    pub expanded: Option<bool>,
    /// What stands in the chevron's column of a row with no chevron, in its colour.
    pub mark: Option<(Icon, Color32)>,
    /// An icon after the chevron's column, as a folder's.
    pub icon: Option<Icon>,
    /// A count at its end, in `text_secondary`, already written.
    pub count: Option<String>,
    pub chosen: bool,
    /// Drawn disabled where false; it still takes the focus and clicks, for its menu.
    pub enabled: bool,
    pub role: egui::Role,
    /// Its name to screen readers, where it differs from what it shows.
    pub label: Option<String>,
}

impl TreeRow<'_> {
    /// Draws it; returns its response and its chevron's column.
    pub fn show(self, ui: &mut egui::Ui) -> (egui::Response, Rect) {
        let t = theme(ui.ctx());
        let size = vec2(ui.available_width(), t.sizes.panel_line);
        let (rect, response) = match self.id {
            Some(id) => {
                let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                (rect, ui.interact(rect, id, Sense::click()))
            }
            None => ui.allocate_exact_size(size, Sense::click()),
        };
        // The list is short: a row the keys reach is scrolled into sight.
        if response.gained_focus() {
            response.scroll_to_me(None);
        }
        let indent = (self.level.saturating_sub(1)) as f32 * LEVEL_INDENT;
        let chevron_side = Icon::ChevronEnd.size();
        let chevron = Rect::from_min_size(
            pos2(rect.left() + indent, rect.center().y - chevron_side / 2.0),
            vec2(chevron_side, chevron_side),
        );
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let band = rect.expand2(vec2(t.spacing.xs, 0.0));
            if self.chosen {
                painter.rect_filled(band, t.shape.radius, t.colors.list_chosen);
            } else if response.hovered() && self.enabled {
                painter.rect_filled(band, t.shape.radius, t.colors.hover);
            }
            let ink = match (self.enabled, self.chosen) {
                (false, _) => t.colors.text_disabled,
                (true, true) => t.colors.text_strong,
                (true, false) => t.colors.text,
            };
            match (self.expanded, self.mark) {
                (Some(expanded), _) => {
                    let icon = if expanded {
                        Icon::ChevronDown
                    } else {
                        Icon::ChevronEnd
                    };
                    icon.paint(painter, chevron, t.colors.text_secondary);
                }
                (None, Some((mark, colour))) => {
                    let side = mark.size();
                    let at = Rect::from_center_size(chevron.center(), vec2(side, side));
                    mark.paint(painter, at, colour);
                }
                (None, None) => {}
            }
            let mut x = chevron.right() + t.spacing.xs;
            if let Some(icon) = self.icon {
                let at = Rect::from_min_size(
                    pos2(x, rect.center().y - ROW_ICON / 2.0),
                    vec2(ROW_ICON, ROW_ICON),
                );
                icon.paint(painter, at, t.colors.text_secondary);
                x = at.right() + t.spacing.s;
            }
            let font = egui::FontId::proportional(t.text.body);
            let mut end = rect.right();
            if let Some(count) = self.count {
                let small = egui::FontId::proportional(t.text.caption);
                let galley = painter.layout_no_wrap(count, small, t.colors.text_secondary);
                end -= galley.size().x;
                let at = pos2(end, rect.center().y - galley.size().y / 2.0);
                painter.galley(at, galley, t.colors.text_secondary);
                end -= t.spacing.s;
            }
            let shown = middle_cut(painter, self.name, "", &font, (end - x).max(0.0));
            let galley = painter.layout_no_wrap(shown, font, ink);
            painter.galley(
                pos2(x, rect.center().y - galley.size().y / 2.0),
                galley,
                ink,
            );
            if response.has_focus() {
                focus_ring_centred(painter, band, &t);
            }
        }
        let label = self.label.unwrap_or_else(|| self.name.to_owned());
        let (role, enabled) = (self.role, self.enabled);
        response.widget_info(|| egui::WidgetInfo::labeled(role, enabled, &label));
        let (level, expanded) = (self.level, self.expanded);
        let tree = role == egui::Role::TreeItem;
        ui.ctx().accesskit_node_builder(response.id, |node| {
            if tree {
                node.set_level(level.saturating_sub(1));
            }
            if let Some(expanded) = expanded {
                node.set_expanded(expanded);
            }
        });
        (response, chevron)
    }
}

/// The words of a count, `count`, as a tree row ends with it.
pub fn count(ctx: &egui::Context, count: usize) -> String {
    translator(ctx).format_number(count as f64, 0, false)
}
