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

//! A picker's patches (`docs/pickers.md`, Declaring pickers): darktable's input and target
//! patches under an area mapping, as Exposure's area exposure mapping shows them
//! (`exposure.c:1288-1326` at the reference release), side by side, each under its heading:
//! the input's colour, the area its picker last read, with its readout under it, and the
//! target's. The colours are data the renderer hands it, never a theme's.

use egui::{Color32, Rect, Role, Sense, Stroke, StrokeKind, Ui, WidgetInfo, vec2};

use super::tabs::group_label_text;
use crate::theme;

/// One of the two patches.
#[derive(Clone, Copy, Debug)]
pub struct Patch<'a> {
    /// Its heading, translated: darktable's "input" or "target".
    pub heading: &'a str,
    /// Its colour, sRGB as the screen shows it; `None` while it has none, the input before
    /// a pick, drawn as an empty outline.
    pub colour: Option<Color32>,
    /// Its tooltip, translated.
    pub tip: &'a str,
    /// The line under it, translated, with its own tooltip: the input's L readout.
    pub readout: Option<(&'a str, &'a str)>,
}

/// darktable's input and target patches: two columns of half the row's width, each its
/// heading as a group label, then its patch, a field high, then the input's readout.
/// Screen readers hear each patch as an image named by its heading and its readout.
pub struct Patches<'a> {
    input: Patch<'a>,
    target: Patch<'a>,
    enabled: bool,
}

impl<'a> Patches<'a> {
    pub fn new(input: Patch<'a>, target: Patch<'a>) -> Self {
        Patches {
            input,
            target,
            enabled: true,
        }
    }

    /// Disabled, while its picker is not drawn: the patches outlined, their words in
    /// `text_disabled`.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let t = theme(ui.ctx());
        let gap = t.spacing.m;
        let width = ((ui.available_width() - gap) / 2.0).max(0.0);
        ui.add_space(t.spacing.m);
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for patch in [self.input, self.target] {
                ui.allocate_ui(vec2(width, 0.0), |ui| {
                    ui.set_width(width);
                    ui.vertical(|ui| self.side(ui, &patch, width));
                });
            }
        });
    }

    fn side(&self, ui: &mut Ui, patch: &Patch, width: f32) {
        let t = theme(ui.ctx());
        let enabled = self.enabled;
        let words = if enabled {
            t.colors.text_secondary
        } else {
            t.colors.text_disabled
        };
        let galley = ui
            .painter()
            .layout_job(group_label_text(&t, patch.heading, width, words));
        let (rect, _) =
            ui.allocate_exact_size(vec2(width, galley.size().y + t.spacing.xxs), Sense::hover());
        ui.painter().galley(rect.min, galley, words);
        let (rect, response) = ui.allocate_exact_size(vec2(width, t.sizes.field), Sense::hover());
        let name = match patch.readout {
            Some((readout, _)) => format!("{}, {readout}", patch.heading),
            None => patch.heading.to_owned(),
        };
        response.widget_info(|| WidgetInfo::labeled(Role::Image, enabled, &name));
        paint(ui, rect, if enabled { patch.colour } else { None });
        response
            .on_hover_text(patch.tip)
            .on_disabled_hover_text(patch.tip);
        if let Some((readout, tip)) = patch.readout {
            let colour = if enabled {
                t.colors.text
            } else {
                t.colors.text_disabled
            };
            ui.add_space(t.spacing.xxs);
            ui.add(
                egui::Label::new(egui::RichText::new(readout).color(colour)).sense(Sense::hover()),
            )
            .on_hover_text(tip);
        }
    }
}

/// A patch filled with its colour, its corners the theme's, or with none an outline.
fn paint(ui: &Ui, rect: Rect, colour: Option<Color32>) {
    let t = theme(ui.ctx());
    let radius = t.shape.radius_small;
    match colour {
        Some(colour) => {
            ui.painter().rect_filled(rect, radius, colour);
        }
        None => {
            ui.painter().rect_stroke(
                rect,
                radius,
                Stroke::new(1.0, t.colors.control_outline),
                StrokeKind::Inside,
            );
        }
    }
}
