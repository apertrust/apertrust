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

//! The zoomed view's position marks (`docs/gui.md`, Where the view is): a 2-point bar in
//! `text_secondary` in the margin along the photo area's bottom, and another along its end,
//! each the visible part of that axis against the whole photo, only on an axis where the
//! photo overflows the area. They lie on the surround, centred in the margin's width, so
//! they never touch the photo or the slot's edge; the bottom one goes under the caption.
//! They are painted only: nothing to click, and nothing in the AccessKit tree, since the
//! photo's own node says the part shown.

use egui::{CornerRadius, Painter, Rangef, Rect};

use crate::theme::Theme;

/// A bar's thickness, in points.
const THICKNESS: f32 = 2.0;

/// The least margin the marks show in, in points: under it they would touch the photo or
/// the slot's edge.
const LEAST_MARGIN: f32 = 4.0;

/// The zoomed view's position marks in the margin (`docs/gui.md`, Where the view is).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewMarks {
    area: Rect,
    margin: f32,
    caption: f32,
    across: Option<Rangef>,
    down: Option<Rangef>,
    opacity: f32,
}

impl ViewMarks {
    /// The marks round the photo area `area`, the part of the photo shown, in a margin of
    /// `margin` points.
    pub fn new(area: Rect, margin: f32) -> Self {
        ViewMarks {
            area,
            margin,
            caption: 0.0,
            across: None,
            down: None,
            opacity: 1.0,
        }
    }

    /// The caption's line and its gap under the area, in points, which the bottom bar goes
    /// below: it lies in the margin, never on the caption.
    pub fn caption(mut self, height: f32) -> Self {
        self.caption = height.max(0.0);
        self
    }

    /// The visible part of the photo's width, 0 to 1; none where it does not overflow.
    pub fn across(mut self, span: Option<Rangef>) -> Self {
        self.across = span;
        self
    }

    /// The visible part of the photo's height, 0 to 1; none where it does not overflow.
    pub fn down(mut self, span: Option<Rangef>) -> Self {
        self.down = span;
        self
    }

    /// How strongly they show, 0 to 1, as they fade.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// The bars' rectangles: along the bottom, then along the end, the right in a
    /// left-to-right language and the left where `rtl`. None for an axis that fits, and
    /// none at all under a [`LEAST_MARGIN`].
    fn bars(&self, rtl: bool) -> [Option<Rect>; 2] {
        if self.margin < LEAST_MARGIN {
            return [None, None];
        }
        let area = self.area;
        let along = |span: Rangef, range: Rangef| {
            let at = |share: f32| range.min + range.span() * share.clamp(0.0, 1.0);
            Rangef::new(at(span.min), at(span.max))
        };
        let half = THICKNESS / 2.0;
        let bottom = self.across.map(|span| {
            let y = area.bottom() + self.caption + self.margin / 2.0;
            Rect::from_x_y_ranges(along(span, area.x_range()), y - half..=y + half)
        });
        let end = self.down.map(|span| {
            let x = if rtl {
                area.left() - self.margin / 2.0
            } else {
                area.right() + self.margin / 2.0
            };
            Rect::from_x_y_ranges(x - half..=x + half, along(span, area.y_range()))
        });
        [bottom, end]
    }

    /// Paints the bars with `painter`, in `theme`'s colours; nothing under a 4-point margin.
    /// The end bar is on the left where `rtl`, a right-to-left language.
    pub fn paint(&self, painter: &Painter, theme: &Theme, rtl: bool) {
        if self.opacity <= 0.0 {
            return;
        }
        let colour = theme.colors.text_secondary.gamma_multiply(self.opacity);
        // Rounded ends: half the thickness.
        let round = CornerRadius::same((THICKNESS / 2.0) as u8);
        for bar in self.bars(rtl).into_iter().flatten() {
            painter.rect_filled(bar, round, colour);
        }
    }
}
