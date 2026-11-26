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

//! A tag: a few letters marking a fact, as the Info tool's "AWB" after the camera and "DNG"
//! after the size. Light with dark letters, as a checked box
//! is: `check` behind `check_mark`, the letters 10 points SemiBold, no border, its corners
//! `radius_small`. It takes no focus; its tooltip says what it stands for, which screen
//! readers hear as its description.

use egui::accesskit::Role;
use egui::{FontFamily, FontId, Response, Sense, Ui, WidgetInfo, vec2};

use crate::fonts::SEMIBOLD;
use crate::theme;

/// The letters' size, in points: smaller than any text token, as a tag sits inside a line.
const LETTERS: f32 = 10.0;
/// Its height, in points: the letters' line with a point above and below.
const HEIGHT: f32 = 14.0;

pub struct Tag<'a> {
    text: &'a str,
    tip: Option<&'a str>,
}

impl<'a> Tag<'a> {
    /// `text` is its letters, translated.
    pub fn new(text: &'a str) -> Self {
        Tag { text, tip: None }
    }

    /// What it stands for, translated.
    pub fn tip(mut self, tip: &'a str) -> Self {
        self.tip = Some(tip);
        self
    }

    /// Its width for `text`, as [`Tag::show`] draws it, for a caller placing it at a line's
    /// end.
    pub fn width(ui: &Ui, text: &str) -> f32 {
        let t = theme(ui.ctx());
        let letters = ui
            .painter()
            .layout_no_wrap(text.to_owned(), font(), t.colors.check_mark);
        (letters.size().x + 2.0 * t.spacing.xs).ceil()
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let t = theme(ui.ctx());
        let size = vec2(Tag::width(ui, self.text), HEIGHT);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            painter.rect_filled(rect, t.shape.radius_small, t.colors.check);
            let letters = painter.layout_no_wrap(self.text.to_owned(), font(), t.colors.check_mark);
            let at = rect.center() - letters.size() / 2.0;
            painter.galley(at, letters, t.colors.check_mark);
        }
        let text = self.text;
        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, text));
        match self.tip {
            Some(tip) => {
                ui.ctx()
                    .accesskit_node_builder(response.id, |node| node.set_description(tip));
                response.on_hover_text(tip)
            }
            None => response,
        }
    }
}

fn font() -> FontId {
    FontId::new(LETTERS, FontFamily::Name(SEMIBOLD.into()))
}
