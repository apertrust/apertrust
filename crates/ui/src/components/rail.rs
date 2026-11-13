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

//! A timeline's rail, as History draws its steps (`docs/gui.md`, The History tab; the
//! wireframe's "Photo · History tab: one line says what changed" board): a line down a
//! list's rows with a node on each, painted into a row the caller lays out.
//!
//! - **A step** is a small dot in `text_secondary`.
//! - **The end,** the step the list stands at, is a larger filled dot in `text_strong`,
//!   ringed with the row's ground, so it is told apart by its shape as well as its colour.
//! - **An undone step** is a hollow dot, and the rail under it dashed in `text_disabled`.
//! - **Reset** is its glyph, the Reset icon, on the row's ground; at the end in `text_strong`
//!   and ringed, a shape cue as well as a colour.
//! - The rail itself is a hairline in `border`; no accent anywhere.

use egui::{Color32, Painter, Rect, Shape, Stroke, pos2, vec2};

use crate::{Icon, Theme};

/// What a row's node is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RailNode {
    /// A step that applies.
    Step,
    /// The step the list stands at.
    End,
    /// A step after the end, which does not apply.
    Undone,
    /// A reset of the whole edit; `end` where the list stands at it.
    Reset { end: bool },
    /// No node: the rail runs through, as past a note between steps.
    None,
}

/// The rail above or below a row's node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RailLine {
    /// No rail: the list's first or last row.
    None,
    /// Between steps that apply.
    Solid,
    /// Under an undone step.
    Dashed,
}

/// One row's piece of the rail: its node, and the rail above and below it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rail {
    pub node: RailNode,
    pub above: RailLine,
    pub below: RailLine,
}

impl Rail {
    /// Paints it into `row`, the rail at `x`, its node at the row's middle on `ground`, the
    /// colour behind the row there (the panel, a hover band or the chosen band), which
    /// rings the end and fills a hollow dot. Drawn in `text_disabled` unless `enabled`, as
    /// the list's words are. The rail and the node's centre fall on pixel centres, and the
    /// dashes keep one phase from the row's top, so rows whose heights are a whole number
    /// of dashes join seamlessly.
    pub fn paint(
        &self,
        painter: &Painter,
        row: Rect,
        x: f32,
        ground: Color32,
        enabled: bool,
        t: &Theme,
    ) {
        let x = painter.round_to_pixel_center(x);
        let middle = painter.round_to_pixel_center(row.center().y);
        let off = |colour: Color32| {
            if enabled {
                colour
            } else {
                t.colors.text_disabled
            }
        };
        let line = |from: f32, to: f32, style: RailLine| match style {
            RailLine::None => {}
            RailLine::Solid => {
                let stroke = Stroke::new(t.shape.hairline, off(t.colors.border));
                painter.line_segment([pos2(x, from), pos2(x, to)], stroke);
            }
            RailLine::Dashed => {
                // From the start of the dash it falls in, counted from the row's top.
                let start = from - (from - row.top()).rem_euclid(2.0 * DASH);
                let stroke = Stroke::new(t.shape.hairline, t.colors.text_disabled);
                painter.extend(Shape::dashed_line_with_offset(
                    &[pos2(x, start), pos2(x, to)],
                    stroke,
                    &[DASH],
                    &[DASH],
                    0.0,
                ));
            }
        };
        line(row.top(), middle, self.above);
        line(middle, row.bottom(), self.below);
        let centre = pos2(x, middle);
        match self.node {
            RailNode::Step => {
                painter.circle_filled(centre, STEP / 2.0, off(t.colors.text_secondary));
            }
            RailNode::End => {
                painter.circle_filled(centre, END / 2.0 + RING, ground);
                painter.circle_filled(centre, END / 2.0, off(t.colors.text_strong));
            }
            RailNode::Undone => {
                let stroke = Stroke::new(t.shape.hairline, off(t.colors.text_secondary));
                painter.circle(centre, UNDONE / 2.0, ground, stroke);
            }
            RailNode::Reset { end } => {
                let glyph = Rect::from_center_size(centre, vec2(GLYPH, GLYPH));
                painter.circle_filled(centre, GLYPH / 2.0 + RING, ground);
                let colour = if end {
                    // Told from the others by a ring as well as its colour.
                    let ring = Stroke::new(t.shape.hairline, off(t.colors.text_strong));
                    painter.circle_stroke(centre, GLYPH / 2.0 + RING, ring);
                    t.colors.text_strong
                } else {
                    t.colors.text_secondary
                };
                Icon::Reset.paint(painter, glyph, off(colour));
            }
            RailNode::None => {}
        }
    }
}

/// A step's dot, across, in points.
const STEP: f32 = 5.0;
/// The end's dot, across, and the ring of the row's ground round it.
const END: f32 = 9.0;
const RING: f32 = 2.0;
/// An undone step's hollow dot, across its outline.
const UNDONE: f32 = 5.0;
/// Reset's glyph, the Reset icon's box.
const GLYPH: f32 = 11.0;
/// A dash of the undone rail, and the gap after it.
const DASH: f32 = 2.0;
