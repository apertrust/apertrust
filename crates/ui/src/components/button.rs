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

//! A text button (`docs/gui.md`, Checkboxes, text buttons and icon buttons): its words on
//! a plate, `button`, a control high with a spacing step at each end. A dialog's one
//! primary button is amber. A quiet one, a plate only while touched, is the bars'
//! `ToolButton`. Its words are its name for screen readers.

use egui::{Color32, FontId, Response, Role, Ui, WidgetInfo, pos2, vec2};

use super::plate::{
    ButtonState, Plate, Touch, allocate, paint_focus, paint_plate, undimmed_painter, words_colour,
};
use super::report_shortcut;
use crate::theme;

pub struct Button<'a> {
    text: &'a str,
    primary: bool,
    enabled: bool,
    tip: Option<&'a str>,
    keys: Option<&'a str>,
    width: f32,
    state: Option<ButtonState>,
}

impl<'a> Button<'a> {
    /// `text` is translated.
    pub fn new(text: &'a str) -> Self {
        Button {
            text,
            primary: false,
            enabled: true,
            tip: None,
            keys: None,
            width: 0.0,
            state: None,
        }
    }

    /// The primary button, one in a dialog: amber, `accent_hover` under the pointer and
    /// `accent_pressed` pressed. While disabled it is drawn as any disabled button.
    pub fn primary(mut self, primary: bool) -> Self {
        self.primary = primary;
        self
    }

    /// A disabled button keeps its plate, its words in `text_disabled`, and ignores input.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The tooltip, translated, shown whether it is enabled or not: a disabled button's
    /// says why.
    pub fn tip(mut self, tip: &'a str) -> Self {
        self.tip = Some(tip);
        self
    }

    /// The keys that do the same, in the platform's spelling (`Ctrl+O`), shown at its end in
    /// `text_secondary` and told to screen readers as its shortcut.
    pub fn keys(mut self, keys: &'a str) -> Self {
        self.keys = Some(keys);
        self
    }

    /// At least `width` points wide, its words at its start and its keys at its end, as the
    /// central panel's card's buttons fill its width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Draws it in `state` whatever the pointer and the keys do, as the gallery shows each
    /// state.
    pub fn state(mut self, state: ButtonState) -> Self {
        self.state = Some(state);
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let t = theme(ui.ctx());
        let enabled = self.enabled && ui.is_enabled() && self.state != Some(ButtonState::Disabled);
        let font = FontId::proportional(t.text.body);
        let galley =
            ui.painter()
                .layout_no_wrap(self.text.to_owned(), font.clone(), Color32::PLACEHOLDER);
        let keys = self.keys.map(|keys| {
            ui.painter()
                .layout_no_wrap(keys.to_owned(), font, Color32::PLACEHOLDER)
        });
        let keys_width = keys.as_ref().map_or(0.0, |k| t.spacing.l + k.size().x);
        let width = galley.size().x + keys_width + 2.0 * t.spacing.m;
        let size = vec2(width.max(self.width), t.sizes.control);
        let (rect, response) = allocate(ui, size, enabled);
        if ui.is_rect_visible(rect) {
            let painter = &undimmed_painter(ui);
            let touch = Touch::of(ui.ctx(), &response, enabled, self.state);
            let plate = if self.primary {
                Plate::Primary
            } else {
                Plate::Default
            };
            paint_plate(painter, rect, plate, touch, &t);
            let at = pos2(
                rect.left() + t.spacing.m,
                rect.center().y - galley.size().y / 2.0,
            );
            painter.galley(at, galley, words_colour(&t, plate, touch));
            if let Some(keys) = keys {
                let colour = if touch.enabled {
                    t.colors.text_secondary
                } else {
                    t.colors.text_disabled
                };
                let at = pos2(
                    rect.right() - t.spacing.m - keys.size().x,
                    rect.center().y - keys.size().y / 2.0,
                );
                painter.galley(at, keys, colour);
            }
            paint_focus(painter, rect, t.shape.radius, touch, &t);
        }
        let text = self.text;
        response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, text));
        if let Some(keys) = self.keys {
            report_shortcut(ui, response.id, keys);
        }
        match self.tip {
            Some(tip) => response.on_hover_text(tip).on_disabled_hover_text(tip),
            None => response,
        }
    }
}
