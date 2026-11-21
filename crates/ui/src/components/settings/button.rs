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

//! An action in a settings row, as Storage's Clear: a button `sizes.setting_control` high
//! at the column's end, `raised` with no frame, its words in `text` at the body
//! size, the same under the pointer and `button_pressed` while pressed. Its fact, as a
//! size, comes before it in `text_secondary`, cut at its end where it does not fit, whole
//! in its tooltip. To screen readers a button named by its full name, its fact a label.

use egui::{Color32, Rect, Response, Role, Sense, StrokeKind, Ui, WidgetInfo, pos2, vec2};

use super::super::plate::{allocate, undimmed_painter};
use super::super::truncated;
use super::{body, dimmed, ring};
use crate::theme;

pub struct Button<'a> {
    text: &'a str,
    name: Option<&'a str>,
    fact: Option<&'a str>,
    tip: Option<&'a str>,
    enabled: bool,
}

impl<'a> Button<'a> {
    /// `text` is its words, translated.
    pub fn new(text: &'a str) -> Self {
        Button {
            text,
            name: None,
            fact: None,
            tip: None,
            enabled: true,
        }
    }

    /// Its full name for screen readers, translated, where its words are short ("Clear
    /// thumbnails").
    pub fn name(mut self, name: &'a str) -> Self {
        self.name = Some(name);
        self
    }

    /// What it acts on, translated, before it: "2,345 thumbnails, 120 MB".
    pub fn fact(mut self, fact: &'a str) -> Self {
        self.fact = Some(fact);
        self
    }

    /// Its tooltip, translated, shown whether it is enabled or not.
    pub fn tip(mut self, tip: &'a str) -> Self {
        self.tip = Some(tip);
        self
    }

    /// A disabled button keeps its box, its words in `text_disabled`, and ignores input.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws it, with its fact, at the end of `ui`'s width.
    pub fn show(self, ui: &mut Ui) -> Response {
        let t = theme(ui.ctx());
        let enabled = self.enabled && ui.is_enabled();
        let height = t.sizes.setting_control;
        let painter = undimmed_painter(ui);
        let words = painter.layout_no_wrap(self.text.to_owned(), body(&t), Color32::PLACEHOLDER);
        let width = words.size().x + 2.0 * t.spacing.m;
        let (strip, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        let place = Rect::from_min_size(
            pos2(strip.right() - width, strip.top()),
            vec2(width, height),
        );
        let mut end = ui.new_child(egui::UiBuilder::new().max_rect(place));
        let (rect, response) = allocate(&mut end, vec2(width, height), enabled);
        if let Some(fact) = self.fact {
            let room = (rect.left() - t.spacing.m - strip.left()).max(0.0);
            let colour = dimmed(&t, enabled, t.colors.text_secondary);
            let galley = painter.layout_job(truncated(fact, body(&t), room, colour));
            let at = pos2(
                rect.left() - t.spacing.m - galley.size().x,
                strip.center().y - galley.size().y / 2.0,
            );
            let cut = galley.elided;
            let said = Rect::from_min_size(at, galley.size());
            painter.galley(at, galley, colour);
            let label = ui.interact(said, response.id.with("fact"), Sense::hover());
            label.widget_info(|| WidgetInfo::labeled(Role::Label, true, fact));
            if cut {
                label.on_hover_text(fact);
            }
        }
        let fill = if enabled && super::pressed(&response) {
            t.colors.button_pressed
        } else {
            t.colors.raised
        };
        painter.rect(
            rect,
            t.shape.radius,
            fill,
            super::edge(&t),
            StrokeKind::Inside,
        );
        let colour = dimmed(&t, enabled, t.colors.text);
        painter.galley(rect.center() - words.size() / 2.0, words, colour);
        if response.has_focus() {
            ring(&painter, rect, &t);
        }
        let name = self.name.unwrap_or(self.text);
        response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, name));
        match self.tip {
            Some(tip) => response.on_hover_text(tip).on_disabled_hover_text(tip),
            None => response,
        }
    }
}
