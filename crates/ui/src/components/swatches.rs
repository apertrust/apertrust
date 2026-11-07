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

//! A row of colour buttons, one of them chosen: Colour editor's colour ranges.

use egui::epaint::{Hsva, Mesh};
use egui::{Color32, Rect, Role, Sense, Stroke, StrokeKind, Ui, WidgetInfo, pos2, vec2};

use super::focus_ring;
use crate::theme;

/// What a swatch shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SwatchFill {
    /// The colour it stands for. Data, not a theme colour.
    Color(Color32),
    /// Every hue: a range picked on the photo.
    Spectrum,
}

#[derive(Clone, Copy, Debug)]
pub struct Swatch<'a> {
    /// Translated: the tooltip and what screen readers say.
    pub name: &'a str,
    pub fill: SwatchFill,
}

pub struct Swatches<'a> {
    name: &'a str,
    items: &'a [Swatch<'a>],
    selected: usize,
}

const SIDE: f32 = 24.0;

impl<'a> Swatches<'a> {
    /// `name` says what the swatches choose, for screen readers.
    pub fn new(name: &'a str, items: &'a [Swatch<'a>], selected: usize) -> Self {
        Swatches {
            name,
            items,
            selected,
        }
    }

    /// Returns the swatch the user chose this frame.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let t = theme(ui.ctx());
        let gap = t.spacing.m + t.spacing.xxs;
        let count = self.items.len() as f32;
        let width = ui.available_width();
        let (row, group) =
            ui.allocate_exact_size(vec2(width, SIDE + 2.0 * t.spacing.s), Sense::hover());
        group.widget_info(|| WidgetInfo::labeled(Role::RadioGroup, true, self.name));
        let start = row.center().x - (count * SIDE + (count - 1.0) * gap) / 2.0;
        let mut chosen = None;
        for (i, swatch) in self.items.iter().enumerate() {
            let rect = Rect::from_min_size(
                pos2(start + i as f32 * (SIDE + gap), row.center().y - SIDE / 2.0),
                vec2(SIDE, SIDE),
            );
            let response = ui.interact(rect, group.id.with(i), Sense::click());
            let painter = ui.painter();
            match swatch.fill {
                SwatchFill::Color(colour) => {
                    painter.rect_filled(rect, t.shape.radius_large, colour);
                }
                SwatchFill::Spectrum => {
                    painter.add(spectrum(rect));
                }
            }
            painter.rect_stroke(
                rect,
                t.shape.radius_large,
                Stroke::new(t.shape.hairline, t.colors.border),
                StrokeKind::Inside,
            );
            let on = i == self.selected;
            if on {
                // A gap in the panel's colour, then the accent: the ring reads on any swatch.
                painter.rect_stroke(
                    rect.expand(2.0),
                    t.shape.radius_large + 2.0,
                    Stroke::new(2.0, t.colors.panel),
                    StrokeKind::Inside,
                );
                painter.rect_stroke(
                    rect.expand(4.0),
                    t.shape.radius_large + 4.0,
                    Stroke::new(2.0, t.colors.text),
                    StrokeKind::Inside,
                );
            } else if response.has_focus() {
                focus_ring(painter, rect.expand(2.0), &t);
            }
            let name = swatch.name;
            response.widget_info(|| WidgetInfo::selected(Role::RadioButton, true, on, name));
            if response.on_hover_text(name).clicked() && !on {
                chosen = Some(i);
            }
        }
        chosen
    }
}

/// Every hue across the square, left to right.
fn spectrum(rect: Rect) -> Mesh {
    let mut mesh = Mesh::default();
    let steps = 6;
    for i in 0..=steps {
        let f = i as f32 / steps as f32;
        let colour = Color32::from(Hsva::new(f, 0.75, 0.9, 1.0));
        let x = rect.left() + f * rect.width();
        mesh.colored_vertex(pos2(x, rect.top()), colour);
        mesh.colored_vertex(pos2(x, rect.bottom()), colour);
        if i > 0 {
            let n = mesh.vertices.len() as u32;
            mesh.add_triangle(n - 4, n - 3, n - 2);
            mesh.add_triangle(n - 3, n - 1, n - 2);
        }
    }
    mesh
}
