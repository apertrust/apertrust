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

//! On-photo overlays (`docs/gui.md`, On-photo overlays): the one line every mark on the
//! photo is drawn with, darktable's picker line (`src/views/darkroom.c:331-470` at the
//! reference release), and the handles, the point picker, the label and the focus ring
//! drawn with it.
//!
//! An [`Overlay`] is made for a frame from the photo's rectangle on screen and the zoom,
//! the points one frame pixel spans. Marks are given in the photo's coordinates, 0 to 1
//! across the photo as it is shown, (0, 0) its top left; their sizes are points, the
//! theme's `overlay` tokens, the same at every zoom. It only paints, with no clip of its
//! own: what a mark means, and the pointer and keys over it, are the caller's.

use std::sync::Arc;

use egui::emath::GuiRounding as _;
use egui::{
    Align2, Color32, FontId, Galley, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2, pos2,
    vec2,
};

use crate::components::focus_shows;
use crate::{Icon, Theme};

/// How a mark is drawn (`docs/gui.md`, the three weights).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Weight {
    /// What is active: the picker's area or point, the shape being edited.
    #[default]
    Normal,
    /// What the pointer or the list points at: the line and its halo twice as thick
    /// (`darkroom.c:436`), and a point's crosshair twice as large (`:420`).
    Selected,
    /// What is shown but not edited, as stored samples: the line's white dashed over its
    /// solid black, 4 points on and 4 off, each dash with square ends (`:345`, `:442-446`).
    Secondary,
}

impl Weight {
    pub const ALL: [Weight; 3] = [Weight::Normal, Weight::Selected, Weight::Secondary];

    /// How many times the line's width this weight draws.
    fn scale(self) -> f32 {
        match self {
            Weight::Selected => 2.0,
            Weight::Normal | Weight::Secondary => 1.0,
        }
    }
}

/// A point mark, as darktable draws a picker's point (`darkroom.c:400-431`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Point {
    /// The active picker's point: a circle with a crosshair through it, both reaching 4
    /// half-pixels from the point (`:419`, `:426`).
    Active,
    /// A stored sample's point: the crosshair alone, reaching 5 half-pixels (`:419`).
    Stored,
}

impl Point {
    /// How far its marks reach from the point, in half-pixels.
    fn reach(self) -> f32 {
        match self {
            Point::Active => 4.0,
            Point::Stored => 5.0,
        }
    }
}

/// One mark's lines and handles in the photo's coordinates, stroked together by
/// [`Overlay::stroke`]: every part's black first, then every part's white, so where its
/// parts cross the white runs unbroken, as darktable strokes one path (`darkroom.c:437-448`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mark {
    weight: Weight,
    parts: Vec<Part>,
}

#[derive(Clone, Debug, PartialEq)]
enum Part {
    /// A line through the points, closed back to the first or open with square ends.
    Line { points: Vec<Pos2>, closed: bool },
    /// A square handle, `overlay.handle` points across, centred on the point.
    Square(Pos2),
    /// A circle of a radius in points.
    Circle(Pos2, f32),
    /// A crosshair whose arms reach a length in points from the point.
    Crosshair(Pos2, f32),
}

impl Mark {
    /// A mark drawn at `weight`, with no parts yet.
    pub fn new(weight: Weight) -> Self {
        Mark {
            weight,
            parts: Vec::new(),
        }
    }

    /// A line through `points`, its ends square.
    pub fn line(mut self, points: impl IntoIterator<Item = Pos2>) -> Self {
        self.parts.push(Part::Line {
            points: points.into_iter().collect(),
            closed: false,
        });
        self
    }

    /// An area's outline, its corners on the screen's pixels.
    pub fn rect(mut self, area: Rect) -> Self {
        self.parts.push(Part::Line {
            points: vec![
                area.left_top(),
                area.right_top(),
                area.right_bottom(),
                area.left_bottom(),
            ],
            closed: true,
        });
        self
    }

    /// A square handle at `at`, for what bounds or resizes: an area's corner.
    pub fn square(mut self, at: Pos2) -> Self {
        self.parts.push(Part::Square(at));
        self
    }

    /// A square handle on each of `area`'s corners, as an area picker's
    /// (`darkroom.c:389-397`).
    pub fn corners(self, area: Rect) -> Self {
        [
            area.left_top(),
            area.right_top(),
            area.left_bottom(),
            area.right_bottom(),
        ]
        .into_iter()
        .fold(self, Mark::square)
    }

    /// A circle handle at `at`, `radius` points round, for a point and for what turns: a
    /// gradient's pivot, a rotation.
    pub fn circle(mut self, at: Pos2, radius: f32) -> Self {
        self.parts.push(Part::Circle(at, radius));
        self
    }

    /// A crosshair at `at`, its arms reaching `reach` points from it.
    pub fn crosshair(mut self, at: Pos2, reach: f32) -> Self {
        self.parts.push(Part::Crosshair(at, reach));
        self
    }
}

/// A mark a label names ([`Overlay::mark_label`]), in the photo's coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Labelled {
    /// An area drawn with its corner handles; `focused` while it has the keyboard's focus,
    /// its ring then round it.
    Area { area: Rect, focused: bool },
    /// The active picker's point.
    Point(Pos2),
}

/// A labelled mark on screen, in points ([`label_place`]).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Near {
    /// An area's rectangle, its corner handles reaching `handle` past it on each side, and
    /// what is drawn round it, the handles or the focus ring, `clear` past it.
    Area { rect: Rect, handle: f32, clear: f32 },
    /// A point whose mark reaches `reach` from it.
    Point { at: Pos2, reach: f32 },
}

/// A label's gaps, in points: `apart` from its mark, `inside` past the handle where it goes
/// inside an area.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Gaps {
    apart: f32,
    inside: f32,
}

/// Where a label `size` points large goes by `near`, inside `shown` (`docs/pickers.md`, On
/// the photo). An area's: above its top-left handle, left-aligned with the handle's outer
/// edge, its bottom `apart` over what is drawn round the area; below the bottom-left
/// handle where there is no room above; right-aligned to the right-hand handles' outer
/// edge where left-aligned it would leave `shown`; and where neither way fits on an axis,
/// inside the top-left corner, `inside` past the handle. A point's: `apart` past its reach
/// to its right, centred on it, or to its left where the right has no room. Either is then
/// moved inside `shown` where it fits there.
fn label_place(near: Near, size: Vec2, shown: Rect, gaps: Gaps) -> Rect {
    let fits_across = |left: f32| left >= shown.left() && left + size.x <= shown.right();
    let min = match near {
        Near::Area {
            rect,
            handle,
            clear,
        } => {
            let start = rect.left() - handle;
            let end = rect.right() + handle - size.x;
            let left = [start, end].into_iter().find(|&left| fits_across(left));
            let above = rect.top() - clear - gaps.apart - size.y;
            let below = rect.bottom() + clear + gaps.apart;
            let top = [above, below]
                .into_iter()
                .find(|&top| top >= shown.top() && top + size.y <= shown.bottom());
            match (left, top) {
                (Some(left), Some(top)) => pos2(left, top),
                _ => rect.left_top() + Vec2::splat(handle + gaps.inside),
            }
        }
        Near::Point { at, reach } => {
            let right = at.x + reach + gaps.apart;
            let left = if right + size.x <= shown.right() {
                right
            } else {
                at.x - reach - gaps.apart - size.x
            };
            pos2(left, at.y - size.y / 2.0)
        }
    };
    // Kept inside the part shown where it fits there: a zoomed area's corner, or a point,
    // past its edge.
    let inside = |at: f32, from: f32, to: f32| if from <= to { at.clamp(from, to) } else { at };
    let max = shown.max - size;
    let min = pos2(
        inside(min.x, shown.left(), max.x),
        inside(min.y, shown.top(), max.y),
    );
    Rect::from_min_size(min, size)
}

/// Where the pointer's badge, `size` points square, goes by `pointer`: `offset` below and
/// right of it, or on its other side across or down where that would leave `shown`
/// ([`Overlay::badge`]).
fn badge_place(pointer: Pos2, size: f32, offset: f32, shown: Rect) -> Rect {
    let one = |at: f32, end: f32| {
        if at + offset + size <= end {
            at + offset
        } else {
            at - offset - size
        }
    };
    let min = pos2(
        one(pointer.x, shown.right()),
        one(pointer.y, shown.bottom()),
    );
    Rect::from_min_size(min, Vec2::splat(size))
}

/// The photo's overlay for one frame: where the photo is on screen, and the painter and
/// theme every mark is drawn with.
pub struct Overlay<'a> {
    painter: &'a Painter,
    photo: Rect,
    zoom: f32,
    t: Arc<Theme>,
}

/// A line on screen, in points, ready to stroke.
enum Path {
    Line { points: Vec<Pos2>, closed: bool },
    Circle(Pos2, f32),
}

impl<'a> Overlay<'a> {
    /// The overlay of the photo shown in `photo`, a rectangle on screen in points, at
    /// `zoom`, the points one frame pixel spans, as `docs/pickers.md` names it; marks are
    /// painted with `painter`.
    pub fn new(painter: &'a Painter, photo: Rect, zoom: f32) -> Self {
        Overlay {
            painter,
            photo,
            zoom,
            t: crate::theme(painter.ctx()),
        }
    }

    /// Where `at`, in the photo's coordinates, is on screen.
    pub fn to_screen(&self, at: Pos2) -> Pos2 {
        self.photo.min + vec2(at.x * self.photo.width(), at.y * self.photo.height())
    }

    /// Half a frame pixel in points, rounded, never less than `overlay.half_pixel`
    /// (`darkroom.c:363-417`): the point marks' unit, so they keep their size at Fit and
    /// grow once a frame pixel spans 9 points or more.
    pub fn half_pixel(&self) -> f32 {
        self.frame_half_pixel().max(self.t.overlay.half_pixel)
    }

    /// How far a point mark's crosshair and circle reach from its point, in points: 16 for
    /// the active picker's at Fit, 20 for a stored one's, twice that selected.
    pub fn reach(&self, point: Point, weight: Weight) -> f32 {
        point.reach() * self.half_pixel() * weight.scale()
    }

    /// Strokes `mark`: its black, then its white.
    pub fn stroke(&self, mark: &Mark) {
        let (line, halo) = self.widths(mark.weight);
        let paths: Vec<Path> = mark
            .parts
            .iter()
            .flat_map(|part| self.paths(part, line))
            .collect();
        for path in &paths {
            self.paint(path, Stroke::new(halo, self.t.colors.overlay_halo), None);
        }
        let dash = (mark.weight == Weight::Secondary).then_some(self.t.overlay.dash);
        for path in &paths {
            self.paint(path, Stroke::new(line, self.t.colors.overlay_line), dash);
        }
    }

    /// A point picker's mark at `at`, with `colour`, the colour it sampled, at its centre
    /// (`darkroom.c:400-431`, `:457-468`). The centre is a disc of a half-pixel's radius;
    /// once a frame pixel spans 7 points or more, a square a frame pixel wide whose top edge
    /// runs through the point, the very pixel sampled; selected, a disc of twice the
    /// half-pixel. Its crosshair is never dashed, as darktable dashes only areas, so the
    /// secondary weight draws it as the normal one.
    pub fn point(&self, at: Pos2, point: Point, colour: Color32, weight: Weight) {
        let lines = match weight {
            Weight::Secondary => Weight::Normal,
            other => other,
        };
        let reach = self.reach(point, weight);
        let mut mark = Mark::new(lines);
        if point == Point::Active {
            mark = mark.circle(at, reach);
        }
        self.stroke(&mark.crosshair(at, reach));

        let centre = self.snap(self.to_screen(at), self.widths(lines).0);
        let half = self.half_pixel();
        if weight == Weight::Selected {
            self.painter.circle_filled(centre, 2.0 * half, colour);
        } else if self.shows_pixel() {
            let square = Rect::from_min_size(centre - vec2(half, 0.0), Vec2::splat(2.0 * half));
            self.painter.rect_filled(square, 0.0, colour);
        } else {
            self.painter.circle_filled(centre, half, colour);
        }
    }

    /// `text` on the photo, its `anchor` at `at` moved by `offset` points, in the small text
    /// in the line's white over its black, with no box (`docs/gui.md`, On-photo overlays).
    /// The black is `overlay_halo` drawn at eight offsets of a point round the text, so
    /// where they overlap, next to the letters, it stacks to near black (98 % where all
    /// eight do) and is the token's 40 % only at its outer edge: darker than the line's,
    /// which keeps small text readable on a light photo. Returns where it was drawn.
    pub fn label(&self, at: Pos2, offset: Vec2, anchor: Align2, text: &str) -> Rect {
        let galley = self.label_galley(text);
        let rect = anchor.anchor_size(self.to_screen(at) + offset, galley.size());
        self.paint_label(rect.min, galley)
    }

    /// The label naming what `mark` is, its tool's name on a picker's area or point
    /// (`docs/pickers.md`, On the photo), drawn as [`Overlay::label`] is, by the mark and
    /// inside `shown`, the part of the photo on screen: for an area above its top-left
    /// corner handle, left-aligned with the handle's outer edge, 4 points over it, or over
    /// the focus ring while it shows; for a point 4 points past its reach, centred on it.
    /// Where it would leave `shown`, an area's goes below its bottom-left handle or
    /// right-aligns to its right-hand handles, a point's goes to its left, and where
    /// nothing else fits an area's goes inside its top-left corner, 6 points past the
    /// handle (`label_place`). Nothing is drawn while the mark lies wholly outside
    /// `shown`. Returns where it was drawn.
    pub fn mark_label(&self, mark: Labelled, text: &str, shown: Rect) -> Option<Rect> {
        let t = &self.t;
        let (line, halo) = self.widths(Weight::Normal);
        let (near, bounds) = match mark {
            Labelled::Area { area, focused } => {
                let rect = Rect::from_min_max(
                    self.snap(self.to_screen(area.min), line),
                    self.snap(self.to_screen(area.max), line),
                );
                let handle = t.overlay.handle / 2.0;
                // The focus ring's outer edge past the corners, as `focus_rect` draws it.
                let ring =
                    (t.overlay.handle + halo) / 2.0 + t.shape.focus_gap + t.shape.focus_width;
                let clear = if focused && focus_shows(self.painter.ctx()) {
                    ring
                } else {
                    handle
                };
                let near = Near::Area {
                    rect,
                    handle,
                    clear,
                };
                (near, rect.expand(clear))
            }
            Labelled::Point(at) => {
                let at = self.snap(self.to_screen(at), line);
                let reach = self.reach(Point::Active, Weight::Normal);
                let near = Near::Point { at, reach };
                (near, Rect::from_center_size(at, Vec2::splat(2.0 * reach)))
            }
        };
        if !bounds.intersects(shown) {
            return None;
        }
        let galley = self.label_galley(text);
        let gaps = Gaps {
            apart: t.spacing.xs,
            inside: t.spacing.s,
        };
        let rect = label_place(near, galley.size(), shown, gaps);
        Some(self.paint_label(rect.min, galley))
    }

    /// `icon` in `rect`, as large as fits, stroked as the line is: its lines in the line's
    /// white over the line's black, each as wide as the line's (`docs/gui.md`, On-photo
    /// overlays).
    pub fn icon(&self, icon: Icon, rect: Rect) {
        let (line, halo) = self.widths(Weight::Normal);
        let c = &self.t.colors;
        icon.paint_stroked(self.painter, rect, c.overlay_halo, halo);
        icon.paint_stroked(self.painter, rect, c.overlay_line, line);
    }

    /// The pointer's badge while a picker is on (`docs/pickers.md`, On the photo): `icon`,
    /// the picker's cursor's, at the small text's size, a spacing's `l` (12 points) below
    /// and right of `pointer`, the pointer's hotspot on screen, flipped to its other side
    /// where it would leave `shown`, the part of the photo on screen, at the right or the
    /// bottom (`badge_place`). Returns where it was drawn.
    pub fn badge(&self, icon: Icon, pointer: Pos2, shown: Rect) -> Rect {
        let rect = badge_place(pointer, self.t.text.small, self.t.spacing.l, shown);
        let rect = Rect::from_min_size(
            rect.min.round_to_pixels(self.painter.pixels_per_point()),
            rect.size(),
        );
        self.icon(icon, rect);
        rect
    }

    /// `text` laid out in the small text in the line's white.
    fn label_galley(&self, text: &str) -> Arc<Galley> {
        let font = FontId::proportional(self.t.text.small);
        (self.painter).layout_no_wrap(text.to_owned(), font, self.t.colors.overlay_line)
    }

    /// Paints `galley` from `min`, on the screen's pixels, over its black: `overlay_halo` at
    /// eight offsets of a point round it ([`Overlay::label`]). Returns where it was drawn.
    fn paint_label(&self, min: Pos2, galley: Arc<Galley>) -> Rect {
        let c = &self.t.colors;
        let size = galley.size();
        let min = min.round_to_pixels(self.painter.pixels_per_point());
        // The black reaches as far past the text as past the line: a point.
        let reach = (self.t.overlay.halo - self.t.overlay.line) / 2.0;
        for (x, y) in [
            (-1.0, -1.0),
            (0.0, -1.0),
            (1.0, -1.0),
            (-1.0, 0.0),
            (1.0, 0.0),
            (-1.0, 1.0),
            (0.0, 1.0),
            (1.0, 1.0),
        ] {
            self.painter.galley_with_override_text_color(
                min + reach * vec2(x, y),
                galley.clone(),
                c.overlay_halo,
            );
        }
        self.painter.galley(min, galley, c.overlay_line);
        Rect::from_min_size(min, size)
    }

    /// The focus ring round an area drawn at `weight` with its corner handles: the theme's
    /// ring, outside the handles' black with `shape.focus_gap` of photo between. The only
    /// interface colour on the photo; drawn after the mark while it has the keyboard's
    /// focus, and only while focus rings show, after a key rather than a click
    /// ([`crate::components`]' `FocusVisible`).
    pub fn focus_rect(&self, area: Rect, weight: Weight) {
        if !focus_shows(self.painter.ctx()) {
            return;
        }
        let (line, halo) = self.widths(weight);
        let corners = Rect::from_min_max(
            self.snap(self.to_screen(area.min), line),
            self.snap(self.to_screen(area.max), line),
        );
        let out = (self.t.overlay.handle + halo) / 2.0 + self.t.shape.focus_gap;
        self.painter
            .rect_stroke(corners.expand(out), 0.0, self.ring(), StrokeKind::Outside);
    }

    /// The focus ring round a circle of `radius` points at `at`, a point's reach or a
    /// circle handle, as [`Overlay::focus_rect`] rings an area.
    pub fn focus_circle(&self, at: Pos2, radius: f32, weight: Weight) {
        if !focus_shows(self.painter.ctx()) {
            return;
        }
        let (line, halo) = self.widths(weight);
        let centre = self.snap(self.to_screen(at), line);
        let ring = self.ring();
        let radius = radius + halo / 2.0 + self.t.shape.focus_gap + ring.width / 2.0;
        self.painter.circle_stroke(centre, radius, ring);
    }

    fn ring(&self) -> Stroke {
        Stroke::new(self.t.shape.focus_width, self.t.colors.focus_ring)
    }

    /// The line's and its halo's widths at `weight`.
    fn widths(&self, weight: Weight) -> (f32, f32) {
        let o = &self.t.overlay;
        (o.line * weight.scale(), o.halo * weight.scale())
    }

    /// Half a frame pixel in points, rounded, before its least: darktable's `half_px`.
    fn frame_half_pixel(&self) -> f32 {
        if self.zoom > 0.0 {
            (0.5 * self.zoom).round()
        } else {
            0.0
        }
    }

    /// Whether a point's centre shows the frame pixel sampled: darktable's
    /// `show_preview_pixel_scale` (`darkroom.c:413-417`).
    fn shows_pixel(&self) -> bool {
        self.frame_half_pixel() >= self.t.overlay.half_pixel
    }

    /// `at` on the screen's pixels for a line `width` points wide.
    fn snap(&self, at: Pos2, width: f32) -> Pos2 {
        snap(at, width, self.painter.pixels_per_point())
    }

    /// `part` on screen, its points on the screen's pixels for a line `line` points wide.
    fn paths(&self, part: &Part, line: f32) -> Vec<Path> {
        let at = |p: Pos2| self.snap(self.to_screen(p), line);
        match part {
            Part::Line { points, closed } => vec![Path::Line {
                points: points.iter().map(|&p| at(p)).collect(),
                closed: *closed,
            }],
            Part::Square(p) => {
                let r = Rect::from_center_size(at(*p), Vec2::splat(self.t.overlay.handle));
                vec![Path::Line {
                    points: vec![
                        r.left_top(),
                        r.right_top(),
                        r.right_bottom(),
                        r.left_bottom(),
                    ],
                    closed: true,
                }]
            }
            Part::Circle(p, radius) => vec![Path::Circle(at(*p), *radius)],
            Part::Crosshair(p, reach) => {
                let c = at(*p);
                let (across, down) = (vec2(*reach, 0.0), vec2(0.0, *reach));
                vec![
                    Path::Line {
                        points: vec![c - across, c + across],
                        closed: false,
                    },
                    Path::Line {
                        points: vec![c - down, c + down],
                        closed: false,
                    },
                ]
            }
        }
    }

    /// Strokes `path`, dashed `dash` points on and off when given.
    fn paint(&self, path: &Path, stroke: Stroke, dash: Option<f32>) {
        match (path, dash) {
            (Path::Circle(centre, radius), None) => {
                self.painter.circle_stroke(*centre, *radius, stroke);
            }
            (Path::Circle(centre, radius), Some(dash)) => {
                self.paint_dashed(&circle_points(*centre, *radius), true, stroke, dash);
            }
            (
                Path::Line {
                    points,
                    closed: true,
                },
                None,
            ) => {
                self.painter.add(Shape::closed_line(points.clone(), stroke));
            }
            (
                Path::Line {
                    points,
                    closed: false,
                },
                None,
            ) => {
                self.painter.add(Shape::line(
                    square_ends(points, stroke.width, self.painter.pixels_per_point()),
                    stroke,
                ));
            }
            (Path::Line { points, closed }, Some(dash)) => {
                self.paint_dashed(points, *closed, stroke, dash);
            }
        }
    }

    fn paint_dashed(&self, points: &[Pos2], closed: bool, stroke: Stroke, dash: f32) {
        for piece in dashes(points, closed, dash, dash) {
            self.painter.add(Shape::line(
                square_ends(&piece, stroke.width, self.painter.pixels_per_point()),
                stroke,
            ));
        }
    }
}

/// `at` on the pixels of a screen of `ppp` pixels a point, for a line `width` points wide:
/// a pixel's centre for a line an odd number of pixels wide, its edge for an even one, so
/// the line is sharp, as darktable puts its one-pixel line on a pixel's centre
/// (`darkroom.c:382`, `:409`).
fn snap(at: Pos2, width: f32, ppp: f32) -> Pos2 {
    let odd = (width * ppp).round() as i64 % 2 == 1;
    let one = |v: f32| {
        let device = v * ppp;
        let snapped = if odd {
            (device - 0.5).round() + 0.5
        } else {
            device.round()
        };
        snapped / ppp
    };
    pos2(one(at.x), one(at.y))
}

/// `points` with each end moved out along the line by half `width`: cairo's square cap,
/// darktable's (`darkroom.c:348`), which egui's strokes lack. Less half a pixel of a screen
/// of `ppp` pixels a point, since egui feathers a line's end outwards over a whole pixel:
/// so the end's edge falls where the cap's does, on a pixel's edge.
fn square_ends(points: &[Pos2], width: f32, ppp: f32) -> Vec<Pos2> {
    let mut out = points.to_vec();
    let n = out.len();
    if n < 2 {
        return out;
    }
    let half = width / 2.0 - 0.5 / ppp;
    let start = (out[0] - out[1]).normalized();
    let end = (out[n - 1] - out[n - 2]).normalized();
    out[0] += start * half;
    out[n - 1] += end * half;
    out
}

/// The pieces of the line through `points`, closed back to the first when `closed`, drawn
/// `on` long with `off` between, from its first point, as cairo dashes a path. A dash that
/// turns a corner keeps it.
fn dashes(points: &[Pos2], closed: bool, on: f32, off: f32) -> Vec<Vec<Pos2>> {
    let mut path = points.to_vec();
    if closed && let Some(&first) = points.first() {
        path.push(first);
    }
    let push = |piece: &mut Vec<Pos2>, p: Pos2| {
        if piece.last() != Some(&p) {
            piece.push(p);
        }
    };
    let mut pieces = Vec::new();
    let mut piece: Vec<Pos2> = path.first().copied().into_iter().collect();
    let mut drawing = true;
    let mut left = on;
    for pair in path.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let length = (b - a).length();
        if length <= 0.0 {
            continue;
        }
        let mut done = 0.0;
        while length - done > left {
            done += left;
            let p = a + (b - a) * (done / length);
            if drawing {
                push(&mut piece, p);
                pieces.push(std::mem::take(&mut piece));
                left = off;
            } else {
                piece = vec![p];
                left = on;
            }
            drawing = !drawing;
        }
        left -= length - done;
        if drawing {
            push(&mut piece, b);
        }
    }
    if drawing && piece.len() > 1 {
        pieces.push(piece);
    }
    pieces
}

/// A circle as a closed line, to dash.
fn circle_points(centre: Pos2, radius: f32) -> Vec<Pos2> {
    const STEPS: usize = 64;
    (0..STEPS)
        .map(|i| {
            let angle = i as f32 / STEPS as f32 * std::f32::consts::TAU;
            centre + radius * vec2(angle.cos(), angle.sin())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use egui::{Color32, Event, Key, Modifiers, PointerButton, Rect, Shape, pos2, vec2};

    use super::{
        Gaps, Labelled, Near, Overlay, Weight, badge_place, dashes, label_place, snap, square_ends,
    };
    use crate::theme::contrast;
    use crate::{Builtin, Theme};

    /// The focus rings on the photo follow the components' (`FocusVisible`): shown at first
    /// and after a key, hidden after a press of the pointer.
    #[test]
    fn a_press_hides_the_rings_on_the_photo_and_a_key_brings_them_back() {
        let ctx = egui::Context::default();
        crate::install(&ctx, Theme::builtin(Builtin::Dark));
        let ring = crate::theme(&ctx).colors.focus_ring;
        let rings = |events: Vec<Event>| {
            let input = egui::RawInput {
                events,
                ..Default::default()
            };
            let output = ctx.run_ui(input, |ui| {
                let photo = Rect::from_min_size(pos2(0.0, 0.0), vec2(180.0, 110.0));
                let overlay = Overlay::new(ui.painter(), photo, 1.0);
                let area = Rect::from_min_max(pos2(0.1, 0.2), pos2(0.5, 0.8));
                overlay.focus_rect(area, Weight::Normal);
                overlay.focus_circle(pos2(0.75, 0.5), 16.0, Weight::Normal);
            });
            let count = output
                .shapes
                .iter()
                .filter(|clipped| match &clipped.shape {
                    Shape::Rect(rect) => rect.stroke.color == ring,
                    Shape::Circle(circle) => circle.stroke.color == ring,
                    _ => false,
                })
                .count();
            output.drop_without_applying_deltas();
            count
        };
        assert_eq!(rings(Vec::new()), 2);
        let press = Event::PointerButton {
            pos: pos2(40.0, 40.0),
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        };
        assert_eq!(rings(vec![press]), 0);
        let tab = Event::Key {
            key: Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        assert_eq!(rings(vec![tab]), 2);
    }

    /// darktable's line reads on any ground (`docs/gui.md`, On-photo overlays): over every
    /// grey from black to white, its white stands at least 2:1 from its own black, and the
    /// white or the black at least 2:1 from the ground. Their least, 2.4:1 and 2.3:1, are
    /// over white and over a grey of 146.
    #[test]
    fn the_line_reads_over_white_black_and_every_grey_between() {
        for which in Builtin::ALL {
            let c = Theme::builtin(which).colors;
            for v in 0..=255u8 {
                let ground = Color32::from_gray(v);
                let halo = ground.blend(c.overlay_halo);
                let line = halo.blend(c.overlay_line);
                let own = contrast(line, halo);
                let against = contrast(line, ground).max(contrast(halo, ground));
                assert!(own >= 2.0, "{which:?} over {v}: {own:.2}");
                assert!(against >= 2.0, "{which:?} over {v}: {against:.2}");
            }
        }
    }

    /// The part of the photo shown in the placement tests, and a label 60 by 16 points.
    const SHOWN: Rect = Rect {
        min: pos2(0.0, 0.0),
        max: pos2(400.0, 300.0),
    };
    const LABEL: egui::Vec2 = vec2(60.0, 16.0);
    const GAPS: Gaps = Gaps {
        apart: 4.0,
        inside: 6.0,
    };

    /// An area's label by the area from `min` to `max`, its handles 5 points past it and
    /// nothing else round it.
    fn by_area(min: egui::Pos2, max: egui::Pos2) -> Rect {
        let near = Near::Area {
            rect: Rect::from_min_max(min, max),
            handle: 5.0,
            clear: 5.0,
        };
        label_place(near, LABEL, SHOWN, GAPS)
    }

    #[test]
    fn an_area_s_label_sits_above_its_top_left_handle() {
        let label = by_area(pos2(100.0, 100.0), pos2(200.0, 200.0));
        // Left with the handle's outer edge; its bottom 4 over the handle's top.
        assert_eq!(label.min, pos2(95.0, 100.0 - 5.0 - 4.0 - 16.0));
        // Focused, 4 over the ring instead.
        let near = Near::Area {
            rect: Rect::from_min_max(pos2(100.0, 100.0), pos2(200.0, 200.0)),
            handle: 5.0,
            clear: 9.0,
        };
        let focused = label_place(near, LABEL, SHOWN, GAPS);
        assert_eq!(focused.max.y, 100.0 - 9.0 - 4.0);
    }

    #[test]
    fn an_area_s_label_goes_below_near_the_top_and_right_aligns_near_the_right() {
        let below = by_area(pos2(100.0, 10.0), pos2(200.0, 100.0));
        assert_eq!(below.min, pos2(95.0, 100.0 + 5.0 + 4.0));
        let right = by_area(pos2(350.0, 100.0), pos2(390.0, 200.0));
        assert_eq!(right.max.x, 395.0);
        assert_eq!(right.max.y, 100.0 - 5.0 - 4.0);
        // Both at the top right corner.
        let both = by_area(pos2(350.0, 10.0), pos2(390.0, 100.0));
        assert_eq!((both.max.x, both.min.y), (395.0, 109.0));
    }

    #[test]
    fn an_area_s_label_goes_inside_its_corner_where_nothing_else_fits() {
        // As tall as the part shown: no room above or below.
        let tall = by_area(pos2(100.0, 10.0), pos2(200.0, 290.0));
        assert_eq!(tall.min, pos2(100.0 + 5.0 + 6.0, 10.0 + 5.0 + 6.0));
        // As wide: no room left-aligned or right-aligned.
        let wide = by_area(pos2(2.0, 100.0), pos2(398.0, 200.0));
        assert_eq!(wide.min, pos2(2.0 + 11.0, 100.0 + 11.0));
        // A zoomed area's corner past the part shown: the label inside its edge.
        let zoomed = by_area(pos2(-100.0, -100.0), pos2(300.0, 290.0));
        assert_eq!(zoomed.min, pos2(0.0, 0.0));
    }

    #[test]
    fn a_point_s_label_sits_past_its_reach_then_flips_left_near_the_edge() {
        let near = |x: f32| Near::Point {
            at: pos2(x, 150.0),
            reach: 16.0,
        };
        let right = label_place(near(100.0), LABEL, SHOWN, GAPS);
        assert_eq!(right.min, pos2(100.0 + 16.0 + 4.0, 150.0 - 8.0));
        let left = label_place(near(350.0), LABEL, SHOWN, GAPS);
        assert_eq!(left.max.x, 350.0 - 16.0 - 4.0);
        assert_eq!(left.center().y, 150.0);
        // On the top edge, moved down inside it.
        let top = Near::Point {
            at: pos2(100.0, 0.0),
            reach: 16.0,
        };
        assert_eq!(label_place(top, LABEL, SHOWN, GAPS).min, pos2(120.0, 0.0));
    }

    #[test]
    fn the_badge_sits_below_right_of_the_pointer_and_flips_near_the_edges() {
        let badge = |x, y| badge_place(pos2(x, y), 12.0, 12.0, SHOWN);
        assert_eq!(badge(100.0, 100.0).min, pos2(112.0, 112.0));
        assert_eq!(badge(390.0, 100.0).min, pos2(366.0, 112.0));
        assert_eq!(badge(100.0, 290.0).min, pos2(112.0, 266.0));
        assert_eq!(badge(390.0, 290.0).min, pos2(366.0, 266.0));
    }

    /// A label is drawn by its mark only while the mark shows in the part of the photo shown.
    #[test]
    fn a_mark_s_label_is_not_drawn_while_the_mark_is_out_of_view() {
        let ctx = egui::Context::default();
        crate::install(&ctx, Theme::builtin(Builtin::Dark));
        let mut drawn = Vec::new();
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let photo = Rect::from_min_size(pos2(-400.0, 0.0), vec2(800.0, 300.0));
            let overlay = Overlay::new(ui.painter(), photo, 1.0);
            let shown = Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0));
            for area in [
                Rect::from_min_max(pos2(0.1, 0.3), pos2(0.3, 0.6)),
                Rect::from_min_max(pos2(0.6, 0.3), pos2(0.8, 0.6)),
            ] {
                let mark = Labelled::Area {
                    area,
                    focused: false,
                };
                drawn.push(overlay.mark_label(mark, "Exposure", shown).is_some());
            }
            let point = Labelled::Point(pos2(0.2, 0.5));
            drawn.push(overlay.mark_label(point, "Exposure", shown).is_some());
        });
        output.drop_without_applying_deltas();
        assert_eq!(drawn, [false, true, false]);
    }

    #[test]
    fn a_line_falls_on_pixel_centres_or_edges_by_its_width() {
        let at = pos2(10.2, 7.9);
        // One pixel wide at 1×: the pixel's centre, as darktable's round(x + 0.5) - 0.5.
        assert_eq!(snap(at, 1.0, 1.0), pos2(10.5, 7.5));
        // Two pixels wide, selected at 1× or normal at 2×: the pixel's edge.
        assert_eq!(snap(at, 2.0, 1.0), pos2(10.0, 8.0));
        assert_eq!(snap(at, 1.0, 2.0), pos2(10.0, 8.0));
    }

    #[test]
    fn an_end_is_square_half_the_width_past_the_point_less_egui_s_feather() {
        // At 1×, egui's feather takes half a pixel of the cap's 1.5 points.
        let ends = square_ends(&[pos2(0.0, 0.0), pos2(10.0, 0.0)], 3.0, 1.0);
        assert_eq!(ends, vec![pos2(-1.0, 0.0), pos2(11.0, 0.0)]);
        let ends = square_ends(&[pos2(0.0, 0.0), pos2(10.0, 0.0)], 3.0, 2.0);
        assert_eq!(ends, vec![pos2(-1.25, 0.0), pos2(11.25, 0.0)]);
    }

    #[test]
    fn a_dash_runs_on_round_a_corner() {
        // A 6-point square dashed 4 on 4: the third dash turns a corner.
        let square = [
            pos2(0.0, 0.0),
            pos2(6.0, 0.0),
            pos2(6.0, 6.0),
            pos2(0.0, 6.0),
        ];
        assert_eq!(
            dashes(&square, true, 4.0, 4.0),
            vec![
                vec![pos2(0.0, 0.0), pos2(4.0, 0.0)],
                vec![pos2(6.0, 2.0), pos2(6.0, 6.0)],
                vec![pos2(2.0, 6.0), pos2(0.0, 6.0), pos2(0.0, 4.0)],
            ]
        );
    }
}
