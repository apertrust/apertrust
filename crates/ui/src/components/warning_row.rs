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

//! The warning row, one line (decision 88): an important warning shown inline in a tool's
//! panel, the one text a panel shows besides its rows' words, as Exposure's "The area has
//! clipped pixels" above Last pick. The warning sign at the start, then the words, both
//! in the theme's `warning` colour, cut with … where too long; the words whole and the
//! explanation in the tooltip. It takes no focus; screen readers hear the words as a label,
//! the explanation as its description.

use egui::accesskit::Role;
use egui::{FontId, Rect, Response, Sense, Ui, WidgetInfo, pos2, vec2};

use super::truncated;
use crate::{Icon, theme};

pub struct WarningRow<'a> {
    text: &'a str,
    tip: Option<&'a str>,
}

impl<'a> WarningRow<'a> {
    /// `text` is the warning, translated.
    pub fn new(text: &'a str) -> Self {
        WarningRow { text, tip: None }
    }

    /// What it means, translated: its tooltip, after the words where they are cut.
    pub fn tip(mut self, tip: &'a str) -> Self {
        self.tip = Some(tip);
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let t = theme(ui.ctx());
        let (rect, response) = ui.allocate_exact_size(
            vec2(ui.available_width(), t.sizes.panel_line),
            Sense::hover(),
        );
        let colour = t.colors.warning;
        let side = Icon::Warning.size();
        let font = FontId::proportional(t.text.small);
        let start = rect.left() + side + t.spacing.xs;
        let room = (rect.right() - start).max(0.0);
        let mut cut = false;
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let sign = Rect::from_min_size(
                pos2(rect.left(), rect.center().y - side / 2.0),
                vec2(side, side),
            );
            Icon::Warning.paint(painter, sign, colour);
            let whole = painter.layout_no_wrap(self.text.to_owned(), font.clone(), colour);
            cut = whole.size().x > room;
            let words = painter.layout_job(truncated(self.text, font, room, colour));
            let at = pos2(start, rect.center().y - words.size().y / 2.0);
            painter.galley(at, words, colour);
        }
        let text = self.text;
        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, text));
        if let Some(tip) = self.tip {
            ui.ctx()
                .accesskit_node_builder(response.id, |node| node.set_description(tip));
        }
        let lines: Vec<&str> = [cut.then_some(self.text), self.tip]
            .into_iter()
            .flatten()
            .collect();
        if lines.is_empty() {
            response
        } else {
            response.on_hover_text(lines.join("\n"))
        }
    }
}
