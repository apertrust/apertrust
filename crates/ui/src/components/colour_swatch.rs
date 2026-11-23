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

//! A colour swatch: a plain filled rectangle of a colour a tool computes, with its
//! tooltip, as darktable paints color calibration's illuminant beside its CCT
//! (`_illuminant_color_draw`, `channelmixerrgb.c:3533-3574` at the reference release). The
//! colour is data the renderer hands it, never a theme's. It is at least two quads wide and
//! one high, a field's side, filling what width it is given; it takes no focus, and screen
//! readers hear it as an image named by its tooltip.

use egui::{Color32, Response, Role, Sense, Ui, WidgetInfo, vec2};

use crate::theme;

pub struct ColourSwatch<'a> {
    colour: Color32,
    tip: &'a str,
    width: Option<f32>,
}

impl<'a> ColourSwatch<'a> {
    /// `colour` as the screen shows it; `tip`, translated, says what it is.
    pub fn new(colour: Color32, tip: &'a str) -> Self {
        ColourSwatch {
            colour,
            tip,
            width: None,
        }
    }

    /// Its width, at least two quads; the width available where none is given.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let t = theme(ui.ctx());
        let side = t.sizes.field;
        let width = self.width.unwrap_or(ui.available_width()).max(2.0 * side);
        let (rect, response) = ui.allocate_exact_size(vec2(width, side), Sense::hover());
        // darktable's margins inside its drawing area, a little above and below.
        let fill = rect.shrink2(vec2(0.0, t.spacing.xxs));
        ui.painter()
            .rect_filled(fill, t.shape.radius_small, self.colour);
        let enabled = ui.is_enabled();
        response.widget_info(|| WidgetInfo::labeled(Role::Image, enabled, self.tip));
        response
            .on_hover_text(self.tip)
            .on_disabled_hover_text(self.tip)
    }
}
