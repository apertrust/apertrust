// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2012-2026  darktable developers (ported from src/gui/guides.c)
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

//! The composition guides over the photo (`docs/gui.md`, The Toolbar's Guides; decision
//! 92): darktable's ten guide types and their flip, from `src/gui/guides.c` at the
//! reference release, drawn in the overlay's one language (`docs/gui.md`, On-photo
//! overlays).
//!
//! [`lines`] works each guide out as darktable's `dt_guides_draw` does: about the photo's
//! centre, in points, the y axis down, flipped where the guide takes a flip, and the grid
//! at darktable's defaults, three lines each way with three subdivisions. [`draw`] strokes
//! them over the photo as secondary marks, darktable's solid dark line under its dashed
//! light one, the grid's lines at darktable's lower opacities.

use std::f32::consts::{FRAC_PI_2, PI};

use apertrust_base::settings::Choice;
use egui::{Painter, Pos2, Rect, Vec2, pos2, vec2};

use crate::overlay::{Mark, Overlay, Weight};

/// One of darktable's guide types, in its order (`_guide_names`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Guide {
    Grid,
    /// darktable's default.
    #[default]
    RulesOfThirds,
    Metering,
    Perspective,
    DiagonalMethod,
    HarmoniousTriangles,
    GoldenSections,
    GoldenSpiral,
    GoldenSpiralSections,
    /// Golden mean (all guides): the sections, the spiral and its sections together.
    GoldenMean,
}

impl Guide {
    pub const ALL: [Guide; 10] = [
        Guide::Grid,
        Guide::RulesOfThirds,
        Guide::Metering,
        Guide::Perspective,
        Guide::DiagonalMethod,
        Guide::HarmoniousTriangles,
        Guide::GoldenSections,
        Guide::GoldenSpiral,
        Guide::GoldenSpiralSections,
        Guide::GoldenMean,
    ];

    /// Whether it takes a flip: harmonious triangles and the golden guides, as darktable's
    /// `support_flip` says. The others are drawn unflipped whatever the flip.
    pub fn flips(self) -> bool {
        matches!(
            self,
            Guide::HarmoniousTriangles
                | Guide::GoldenSections
                | Guide::GoldenSpiral
                | Guide::GoldenSpiralSections
                | Guide::GoldenMean
        )
    }
}

/// The guide type setting's options (`guides.type`), darktable's names.
impl Choice for Guide {
    const OPTIONS: &'static [(Guide, &'static str)] = &[
        (Guide::Grid, "grid"),
        (Guide::RulesOfThirds, "rules-of-thirds"),
        (Guide::Metering, "metering"),
        (Guide::Perspective, "perspective"),
        (Guide::DiagonalMethod, "diagonal-method"),
        (Guide::HarmoniousTriangles, "harmonious-triangles"),
        (Guide::GoldenSections, "golden-sections"),
        (Guide::GoldenSpiral, "golden-spiral"),
        (Guide::GoldenSpiralSections, "golden-spiral-sections"),
        (Guide::GoldenMean, "golden-mean"),
    ];
}

/// How a guide that takes a flip is mirrored about the photo's centre, darktable's flip.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GuideFlip {
    #[default]
    None,
    Horizontally,
    Vertically,
    Both,
}

impl GuideFlip {
    pub const ALL: [GuideFlip; 4] = [
        GuideFlip::None,
        GuideFlip::Horizontally,
        GuideFlip::Vertically,
        GuideFlip::Both,
    ];
}

/// The guide flip setting's options (`guides.flip`), darktable's names.
impl Choice for GuideFlip {
    const OPTIONS: &'static [(GuideFlip, &'static str)] = &[
        (GuideFlip::None, "none"),
        (GuideFlip::Horizontally, "horizontally"),
        (GuideFlip::Vertically, "vertically"),
        (GuideFlip::Both, "both"),
    ];
}

/// Lines a guide draws at one opacity, each through its points, about the photo's centre
/// in points.
#[derive(Clone, Debug, PartialEq)]
pub struct Lines {
    /// darktable's alpha for them: 1 for every guide's but the grid's.
    pub opacity: f32,
    pub lines: Vec<Vec<Pos2>>,
}

/// darktable's 1/φ (`INVPHI`).
const INVPHI: f32 = 0.618_034;

/// The grid's lines each way and their subdivisions, darktable's defaults (`grid_nbh`,
/// `grid_nbv`, `grid_subdiv`), which no setting changes yet.
const GRID_LINES: i32 = 3;
const GRID_SUBDIVISIONS: i32 = 3;

/// The lines `guide` draws over a photo `size` points large, flipped as `flip` says where
/// it takes a flip, about the photo's centre, as darktable's `dt_guides_draw` works them
/// out: the grid's at two opacities, every other guide's at one.
pub fn lines(guide: Guide, flip: GuideFlip, size: Vec2) -> Vec<Lines> {
    let (w, h) = (size.x, size.y);
    if w <= 0.0 || h <= 0.0 {
        return Vec::new();
    }
    let one = |lines| {
        vec![Lines {
            opacity: 1.0,
            lines,
        }]
    };
    let mut drawn = match guide {
        Guide::Grid => grid(w, h),
        Guide::RulesOfThirds => one(rules_of_thirds(w, h)),
        Guide::Metering => one(metering(w, h)),
        Guide::Perspective => one(perspective(w, h)),
        Guide::DiagonalMethod => one(diagonal_method(w, h)),
        Guide::HarmoniousTriangles => one(harmonious_triangles(w, h)),
        Guide::GoldenSections => one(golden_mean(w, h, Golden::Sections)),
        Guide::GoldenSpiral => one(golden_mean(w, h, Golden::Spiral)),
        Guide::GoldenSpiralSections => one(golden_mean(w, h, Golden::SpiralSections)),
        Guide::GoldenMean => one(golden_mean(w, h, Golden::All)),
    };
    // Mirrored about the centre: darktable's `cairo_scale(cr, -1, 1)` and `(1, -1)`.
    let (x, y) = match flip {
        _ if !guide.flips() => (1.0, 1.0),
        GuideFlip::None => (1.0, 1.0),
        GuideFlip::Horizontally => (-1.0, 1.0),
        GuideFlip::Vertically => (1.0, -1.0),
        GuideFlip::Both => (-1.0, -1.0),
    };
    for line in drawn.iter_mut().flat_map(|group| group.lines.iter_mut()) {
        for point in line.iter_mut() {
            *point = pos2(point.x * x, point.y * y);
        }
    }
    drawn
}

/// Strokes `guide`, flipped as `flip` says, over the whole photo `photo`, a rectangle on
/// screen in points, with `painter`, which the caller clips to the part shown: each line a
/// secondary mark of the overlay, the grid's at its opacities.
pub fn draw(painter: &Painter, photo: Rect, guide: Guide, flip: GuideFlip) {
    let size = photo.size();
    // About the centre in points, to the photo's coordinates, 0 to 1 across it.
    let to_photo = |p: Pos2| pos2(p.x / size.x + 0.5, p.y / size.y + 0.5);
    for group in lines(guide, flip, size) {
        let mut faded = painter.clone();
        faded.multiply_opacity(group.opacity);
        let overlay = Overlay::new(&faded, photo, 1.0);
        let mark = (group.lines.iter()).fold(Mark::new(Weight::Secondary), |mark, line| {
            mark.line(line.iter().map(|&p| to_photo(p)))
        });
        overlay.stroke(&mark);
    }
}

/// A line from `(x0, y0)` to `(x1, y1)`: darktable's `dt_draw_line`.
fn line(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<Pos2> {
    vec![pos2(x0, y0), pos2(x1, y1)]
}

/// `dt_guides_draw_rules_of_thirds`: two lines each way at the thirds.
fn rules_of_thirds(w: f32, h: f32) -> Vec<Vec<Pos2>> {
    let (left, top) = (-w / 2.0, -h / 2.0);
    let (right, bottom) = (left + w, top + h);
    let (x_3, y_3) = (w / 3.0, h / 3.0);
    vec![
        line(left + x_3, top, left + x_3, bottom),
        line(left + 2.0 * x_3, top, left + 2.0 * x_3, bottom),
        line(left, top + y_3, right, top + y_3),
        line(left, top + 2.0 * y_3, right, top + 2.0 * y_3),
    ]
}

/// `dt_guides_draw_grid` at darktable's defaults: the subdivisions' lines at 0.3, then the
/// cells' at 0.5, each set of `dt_draw_horizontal_lines` and `dt_draw_vertical_lines`,
/// its bounds truncated to whole points, where darktable's C truncates them in image
/// pixels.
fn grid(w: f32, h: f32) -> Vec<Lines> {
    let (left, top) = ((-w / 2.0).trunc(), (-h / 2.0).trunc());
    let (right, bottom) = ((-w / 2.0 + w).trunc(), (-h / 2.0 + h).trunc());
    let set = |num: i32| {
        let across = (1..num).map(|k| {
            let y = top + k as f32 / num as f32 * (bottom - top);
            line(left, y, right, y)
        });
        let down = (1..num).map(|k| {
            let x = left + k as f32 / num as f32 * (right - left);
            line(x, top, x, bottom)
        });
        across.chain(down).collect()
    };
    vec![
        Lines {
            opacity: 0.3,
            lines: set((1 + GRID_LINES) * (1 + GRID_SUBDIVISIONS)),
        },
        Lines {
            opacity: 0.5,
            lines: set(1 + GRID_LINES),
        },
    ]
}

/// `dt_guides_draw_metering`: a scale along the middle each way, its marks short, middle
/// and long, the middle lines across the whole photo, and small crosses between.
fn metering(w: f32, h: f32) -> Vec<Vec<Pos2>> {
    const X_LINES: i32 = 49;
    const Y_LINES: i32 = 33;
    const CROSSES: i32 = 6;
    let (x, y) = (-w / 2.0, -h / 2.0);
    let x_step = w / (X_LINES - 1) as f32;
    let y_step = h / (Y_LINES - 1) as f32;
    let short = w.min(h) * 0.02;
    let middle = short * 1.5;
    let long = middle * 1.5;
    let mut lines = Vec::new();
    // Along the x axis, about the middle row.
    for i in 0..X_LINES {
        let at = x + i as f32 * x_step;
        let reach = if i % 4 != 0 {
            short
        } else if i % 12 != 0 {
            middle
        } else if i != X_LINES / 2 {
            long
        } else {
            h * 0.5
        };
        lines.push(line(at, -reach, at, reach));
    }
    // Along the y axis, about the middle column; C's remainder, negative below 4.
    for i in 0..Y_LINES {
        let at = y + i as f32 * y_step;
        let reach = if (i - 4) % 4 != 0 {
            short
        } else if i == Y_LINES / 2 {
            w * 0.5
        } else if (i - 4) % 12 != 0 {
            middle
        } else {
            long
        };
        lines.push(line(-reach, at, reach, at));
    }
    // The small crosses, off the middle lines.
    let cross = short * 0.5;
    let (cross_x, cross_y) = (w / CROSSES as f32, h / CROSSES as f32);
    for cx in 1..CROSSES {
        for cy in 1..CROSSES {
            if cx != CROSSES / 2 && cy != CROSSES / 2 {
                let (at_x, at_y) = (x + cx as f32 * cross_x, y + cy as f32 * cross_y);
                lines.push(line(at_x - cross, at_y, at_x + cross, at_y));
                lines.push(line(at_x, at_y - cross, at_x, at_y + cross));
            }
        }
    }
    lines
}

/// `dt_guides_draw_perspective`: sixteen rays from the centre, a sixteenth of a turn
/// apart, anticlockwise on screen from the right. darktable draws each far past the photo
/// and clips it; each here ends where it leaves the photo, which draws the same.
fn perspective(w: f32, h: f32) -> Vec<Vec<Pos2>> {
    const LINES: usize = 16;
    let step = 2.0 / LINES as f32;
    (0..LINES)
        .map(|i| {
            let angle = -PI * step * i as f32;
            let (sin, cos) = angle.sin_cos();
            // Where the ray leaves the rectangle, the nearer of its two edges.
            let reach = |half: f32, along: f32| {
                if along.abs() > f32::EPSILON {
                    half / along.abs()
                } else {
                    f32::INFINITY
                }
            };
            let t = reach(w / 2.0, cos).min(reach(h / 2.0, sin));
            line(0.0, 0.0, t * cos, t * sin)
        })
        .collect()
}

/// `dt_guides_draw_diagonal_method`: the diagonals of a square at each end of the photo's
/// longer side.
fn diagonal_method(w: f32, h: f32) -> Vec<Vec<Pos2>> {
    let (x, y) = (-w / 2.0, -h / 2.0);
    if w > h {
        vec![
            line(x, y, x + h, y + h),
            line(x, y + h, x + h, y),
            line(x + w - h, y, x + w, y + h),
            line(x + w - h, y + h, x + w, y),
        ]
    } else {
        vec![
            line(x, y, x + w, y + w),
            line(x, y + w, x + w, y),
            line(x, y + h - w, x + w, y + h),
            line(x, y + h, x + w, y + h - w),
        ]
    }
}

/// `dt_guides_draw_harmonious_triangles`: the diagonal, and a line from each of the other
/// corners meeting it at a right angle, `dst` truncated to whole points, where darktable
/// truncates it in image pixels.
fn harmonious_triangles(w: f32, h: f32) -> Vec<Vec<Pos2>> {
    let dst = (h * (w / h).atan().cos() / (h / w).atan().cos()).trunc();
    vec![
        line(-w / 2.0, -h / 2.0, w / 2.0, h / 2.0),
        line(-w / 2.0 + dst, -h / 2.0, -w / 2.0, h / 2.0),
        line(w / 2.0, -h / 2.0, w / 2.0 - dst, h / 2.0),
    ]
}

/// Which of the golden mean's parts a golden guide draws (`dt_golden_type_t`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Golden {
    Sections,
    Spiral,
    SpiralSections,
    All,
}

/// darktable's `dt_QRect_t`: a rectangle by its corner and size.
#[derive(Clone, Copy, Debug, PartialEq)]
struct QRect {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    width: f32,
    height: f32,
}

impl QRect {
    /// `dt_guides_q_rect`.
    fn new(left: f32, top: f32, width: f32, height: f32) -> QRect {
        QRect {
            left,
            top,
            right: left + width,
            bottom: top + height,
            width,
            height,
        }
    }
}

/// The golden mean's seven rectangles, R1 to R7, about the centre of a photo `w` by `h`
/// (`_guides_draw_golden_mean`). R2's width keeps darktable's extra point, its
/// `w - 2 * w_2` being none in floats.
fn golden_rects(w: f32, h: f32) -> [QRect; 7] {
    let (w_g, h_g) = (w * INVPHI, h * INVPHI);
    let (w_2, h_2) = (w / 2.0, h / 2.0);
    let r1 = QRect::new(-w_2, -h_2, w_g, h);
    let r2 = QRect::new(w_g - w_2, h_2 - h_g, w - w_g + 1.0 - (w - 2.0 * w_2), h_g);
    let r3 = QRect::new(
        w_2 - r2.width * INVPHI,
        -h_2,
        r2.width * INVPHI,
        h - r2.height,
    );
    let r4 = QRect::new(r2.left, r1.top, r3.left - r2.left, r3.height * INVPHI);
    let r5 = QRect::new(r4.left, r4.bottom, r4.width * INVPHI, r3.height - r4.height);
    let r6 = QRect::new(
        r5.left + r5.width,
        r5.bottom - r5.height * INVPHI,
        r3.left - r5.right,
        r5.height * INVPHI,
    );
    let r7 = QRect::new(
        r6.right - r6.width * INVPHI,
        r4.bottom,
        r6.width * INVPHI,
        r5.height - r6.height,
    );
    [r1, r2, r3, r4, r5, r6, r7]
}

/// `dt_guides_draw_golden_mean`'s parts for `which`: the golden sections, the spiral's
/// sections and the golden spiral, its arcs as cairo draws them under a scale.
fn golden_mean(w: f32, h: f32, which: Golden) -> Vec<Vec<Pos2>> {
    let [r1, r2, r3, r4, r5, r6, r7] = golden_rects(w, h);
    let mut lines = Vec::new();
    if matches!(which, Golden::Sections | Golden::All) {
        lines.push(line(r1.left, r2.top, r2.right, r2.top));
        let across = r1.top + r2.height;
        lines.push(line(r1.left, across, r2.right, across));
        lines.push(line(r1.right, r1.top, r1.right, r1.bottom));
        let down = r1.left + r2.width;
        lines.push(line(down, r1.top, down, r1.bottom));
    }
    if matches!(which, Golden::SpiralSections | Golden::All) {
        lines.push(line(r1.right, r1.top, r1.right, r1.bottom));
        lines.push(line(r2.left, r2.top, r2.right, r2.top));
        lines.push(line(r3.left, r3.top, r3.left, r3.bottom));
        lines.push(line(r4.left, r4.bottom, r4.right, r4.bottom));
        lines.push(line(r5.right, r5.top, r5.right, r5.bottom));
        lines.push(line(r6.left, r6.top, r6.right, r6.top));
        lines.push(line(r7.left, r7.top, r7.left, r7.bottom));
    }
    if matches!(which, Golden::Spiral | Golden::All) {
        lines.extend(spiral_arcs(w, h).iter().map(arc));
    }
    lines
}

/// One of the golden spiral's arcs as cairo draws it: `cairo_arc(centre, radius, from, to)`
/// under `cairo_scale(scale.x, scale.y)`, from angle `from` to `to`, increasing.
#[derive(Clone, Copy, Debug, PartialEq)]
struct EllipseArc {
    scale: Vec2,
    centre: Pos2,
    radius: f32,
    from: f32,
    to: f32,
}

impl EllipseArc {
    /// Its point at angle `angle`, on screen.
    fn at(&self, angle: f32) -> Pos2 {
        let (sin, cos) = angle.sin_cos();
        pos2(
            self.scale.x * (self.centre.x + self.radius * cos),
            self.scale.y * (self.centre.y + self.radius * sin),
        )
    }
}

/// The golden spiral's eight arcs about the centre of a photo `w` by `h`
/// (`dt_guides_draw_golden_mean`).
fn spiral_arcs(w: f32, h: f32) -> [EllipseArc; 8] {
    let [r1, r2, r3, r4, r5, r6, r7] = golden_rects(w, h);
    let wide = |r: QRect| r.width / r.height;
    let tall = |r: QRect| r.height / r.width;
    [
        EllipseArc {
            scale: vec2(wide(r1), 1.0),
            centre: pos2(r1.right / r1.width * r1.height, r1.top),
            radius: r1.height,
            from: FRAC_PI_2,
            to: PI,
        },
        EllipseArc {
            scale: vec2(wide(r2), 1.0),
            centre: pos2(r2.left / r2.width * r2.height, r2.top),
            radius: r2.height,
            from: 0.0,
            to: FRAC_PI_2,
        },
        EllipseArc {
            scale: vec2(wide(r3), 1.0),
            centre: pos2(r3.left / r3.width * r3.height, r3.bottom),
            radius: r3.height,
            from: 1.5 * PI,
            to: 2.0 * PI,
        },
        EllipseArc {
            scale: vec2(1.0, tall(r4)),
            centre: pos2(r4.right, r4.bottom / r4.height * r4.width),
            radius: r4.width,
            from: PI,
            to: 1.5 * PI,
        },
        EllipseArc {
            scale: vec2(1.0, tall(r5)),
            centre: pos2(r5.right, r5.top / r5.height * r5.width),
            radius: r5.width,
            from: FRAC_PI_2,
            to: PI,
        },
        EllipseArc {
            scale: vec2(1.0, tall(r6)),
            centre: pos2(r6.left, r6.top / r6.height * r6.width),
            radius: r6.width,
            from: 0.0,
            to: FRAC_PI_2,
        },
        EllipseArc {
            scale: vec2(wide(r7), 1.0),
            centre: pos2(r7.left / r7.width * r7.height, r7.bottom),
            radius: r7.height,
            from: 1.5 * PI,
            to: 2.0 * PI,
        },
        EllipseArc {
            scale: vec2((r6.width - r7.width) / r7.height, 1.0),
            centre: pos2(r7.left / (r6.width - r7.width) * r7.height, r7.bottom),
            radius: r7.height,
            from: 210.0_f32.to_radians(),
            to: 1.5 * PI,
        },
    ]
}

/// The points of a quarter turn's arc, at least: enough that its segments never show at
/// the sizes a photo is fitted at.
const ARC_STEPS: usize = 32;

/// The most a golden arc's segments may stray from cairo's curve, in points, at any zoom.
const ARC_ERROR: f32 = 0.25;

/// The most segments one arc is drawn with, however large the photo is on screen.
const ARC_MOST: usize = 4096;

/// `arc` as a line through points on screen: as many segments as keep each within
/// [`ARC_ERROR`] of the curve, at least [`ARC_STEPS`] a quarter turn and at most
/// [`ARC_MOST`]. A chord one step long strays at most its length squared times the curve's
/// curvature over 8; with the ellipse's semi-axes `a` ≥ `b`, a step of angle `Δ` is at most
/// `a·Δ` long and its curvature at most `a / b²`, so `Δ = √(8·e·b² / a³)` keeps it within
/// `e`.
fn arc(arc: &EllipseArc) -> Vec<Pos2> {
    let turn = arc.to - arc.from;
    let least = ((turn / FRAC_PI_2 * ARC_STEPS as f32).ceil() as usize).max(1);
    let (x, y) = (arc.scale.x.abs(), arc.scale.y.abs());
    let (a, b) = (arc.radius.abs() * x.max(y), arc.radius.abs() * x.min(y));
    let needed = (turn / (8.0 * ARC_ERROR * b * b / (a * a * a)).sqrt()).ceil();
    let steps = if needed.is_finite() && needed > least as f32 {
        (needed as usize).min(ARC_MOST)
    } else {
        least
    };
    (0..=steps)
        .map(|i| arc.at(arc.from + turn * i as f32 / steps as f32))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The photo the tests draw over: 300 by 200 points, wider than tall.
    const SIZE: Vec2 = vec2(300.0, 200.0);

    /// Whether `a` and `b` are the same lines, each point within a thousandth of a point.
    fn same(a: &[Vec<Pos2>], b: &[Vec<Pos2>]) -> bool {
        a.len() == b.len()
            && a.iter().zip(b).all(|(a, b)| {
                a.len() == b.len() && (a.iter().zip(b)).all(|(a, b)| (*a - *b).length() < 1e-3)
            })
    }

    fn one(guide: Guide, flip: GuideFlip, size: Vec2) -> Vec<Vec<Pos2>> {
        let drawn = lines(guide, flip, size);
        assert_eq!(drawn.len(), 1, "{guide:?}");
        assert_eq!(drawn[0].opacity, 1.0, "{guide:?}");
        drawn.into_iter().next().unwrap().lines
    }

    fn segments(lines: &[[f32; 4]]) -> Vec<Vec<Pos2>> {
        (lines.iter())
            .map(|&[x0, y0, x1, y1]| line(x0, y0, x1, y1))
            .collect()
    }

    /// The setting's ids are darktable's names, rules of thirds and no flip its defaults.
    #[test]
    fn the_options_are_darktable_s_in_its_order_with_its_defaults() {
        let ids: Vec<&str> = Guide::OPTIONS.iter().map(|(_, id)| *id).collect();
        assert_eq!(
            ids,
            [
                "grid",
                "rules-of-thirds",
                "metering",
                "perspective",
                "diagonal-method",
                "harmonious-triangles",
                "golden-sections",
                "golden-spiral",
                "golden-spiral-sections",
                "golden-mean",
            ]
        );
        let order: Vec<Guide> = Guide::OPTIONS.iter().map(|(guide, _)| *guide).collect();
        assert_eq!(order, Guide::ALL);
        assert_eq!(Guide::default(), Guide::RulesOfThirds);
        let flips: Vec<&str> = GuideFlip::OPTIONS.iter().map(|(_, id)| *id).collect();
        assert_eq!(flips, ["none", "horizontally", "vertically", "both"]);
        assert_eq!(GuideFlip::default(), GuideFlip::None);
        let flipped: Vec<Guide> = Guide::ALL.into_iter().filter(|g| g.flips()).collect();
        assert_eq!(
            flipped,
            [
                Guide::HarmoniousTriangles,
                Guide::GoldenSections,
                Guide::GoldenSpiral,
                Guide::GoldenSpiralSections,
                Guide::GoldenMean,
            ]
        );
    }

    /// Two lines each way at the thirds, never flipped.
    #[test]
    fn the_rules_of_thirds_are_two_lines_each_way_at_the_thirds() {
        let expected = segments(&[
            [-50.0, -100.0, -50.0, 100.0],
            [50.0, -100.0, 50.0, 100.0],
            [-150.0, -100.0 / 3.0, 150.0, -100.0 / 3.0],
            [-150.0, 100.0 / 3.0, 150.0, 100.0 / 3.0],
        ]);
        for flip in GuideFlip::ALL {
            let drawn = one(Guide::RulesOfThirds, flip, SIZE);
            assert!(same(&drawn, &expected), "{flip:?}: {drawn:?}");
        }
    }

    /// Sixteen subdivisions each way at 0.3, then four cells each way at 0.5, between the
    /// bounds truncated to whole points: a 301 by 201 photo's are ±150 and ±100.
    #[test]
    fn the_grid_is_its_subdivisions_then_its_cells_within_whole_bounds() {
        let drawn = lines(Guide::Grid, GuideFlip::Both, vec2(301.0, 201.0));
        assert_eq!(drawn.len(), 2);
        assert_eq!((drawn[0].opacity, drawn[1].opacity), (0.3, 0.5));
        assert_eq!(drawn[0].lines.len(), 30);
        let cells = segments(&[
            [-150.0, -50.0, 150.0, -50.0],
            [-150.0, 0.0, 150.0, 0.0],
            [-150.0, 50.0, 150.0, 50.0],
            [-75.0, -100.0, -75.0, 100.0],
            [0.0, -100.0, 0.0, 100.0],
            [75.0, -100.0, 75.0, 100.0],
        ]);
        assert!(same(&drawn[1].lines, &cells), "{:?}", drawn[1].lines);
        // The first subdivision each way, a sixteenth across.
        assert!(same(
            &drawn[0].lines[..1],
            &segments(&[[-150.0, -87.5, 150.0, -87.5]])
        ));
        assert!(same(
            &drawn[0].lines[15..16],
            &segments(&[[-131.25, -100.0, -131.25, 100.0]])
        ));
    }

    /// The scale's marks: 49 along x a 48th apart, 33 along y a 32nd apart, short (2 % of
    /// the shorter side each way), middle, long and the middle lines across, then 16
    /// crosses' 32 strokes off the middle lines.
    #[test]
    fn metering_is_two_scales_and_sixteen_crosses() {
        let drawn = one(Guide::Metering, GuideFlip::None, SIZE);
        assert_eq!(drawn.len(), 49 + 33 + 32);
        let (short, middle, long) = (4.0, 6.0, 9.0);
        // Along x: i = 0 long, 1 short, 4 middle, 12 long, 24 the middle line.
        let x = |i: f32| -150.0 + i * 6.25;
        let along = segments(&[
            [x(0.0), -long, x(0.0), long],
            [x(1.0), -short, x(1.0), short],
            [x(4.0), -middle, x(4.0), middle],
            [x(12.0), -long, x(12.0), long],
            [x(24.0), -100.0, x(24.0), 100.0],
        ]);
        let picked: Vec<Vec<Pos2>> = [0, 1, 4, 12, 24].map(|i| drawn[i].clone()).to_vec();
        assert!(same(&picked, &along), "{picked:?}");
        // Along y: i = 0 middle, C's -4 % 12 not being 0; 1 short; 4 long; 8 middle; 16
        // the middle line.
        let y = |i: f32| -100.0 + i * 6.25;
        let across = segments(&[
            [-middle, y(0.0), middle, y(0.0)],
            [-short, y(1.0), short, y(1.0)],
            [-long, y(4.0), long, y(4.0)],
            [-middle, y(8.0), middle, y(8.0)],
            [-150.0, y(16.0), 150.0, y(16.0)],
        ]);
        let picked: Vec<Vec<Pos2>> = [0, 1, 4, 8, 16].map(|i| drawn[49 + i].clone()).to_vec();
        assert!(same(&picked, &across), "{picked:?}");
        // The first cross, a sixth in each way, its arms half a short mark.
        let cross = segments(&[[-102.0, -200.0 / 3.0, -98.0, -200.0 / 3.0]]);
        assert!(same(&drawn[82..83], &cross), "{:?}", &drawn[82..83]);
    }

    /// Sixteen rays from the centre, a sixteenth of a turn apart, anticlockwise on screen,
    /// each ending at the photo's edge.
    #[test]
    fn perspective_is_sixteen_rays_to_the_edges() {
        let drawn = one(Guide::Perspective, GuideFlip::Both, SIZE);
        assert_eq!(drawn.len(), 16);
        let ends = [
            (0, [150.0, 0.0]),
            (1, [150.0, -62.132]),
            (2, [100.0, -100.0]),
            (4, [0.0, -100.0]),
            (6, [-100.0, -100.0]),
            (8, [-150.0, 0.0]),
            (12, [0.0, 100.0]),
        ];
        for (i, [x, y]) in ends {
            let expected = line(0.0, 0.0, x, y);
            assert!(same(&drawn[i..=i], &[expected]), "{i}: {:?}", drawn[i]);
        }
    }

    /// The diagonals of a square at each end of the longer side, wide or tall.
    #[test]
    fn the_diagonal_method_crosses_a_square_at_each_end() {
        let wide = segments(&[
            [-150.0, -100.0, 50.0, 100.0],
            [-150.0, 100.0, 50.0, -100.0],
            [-50.0, -100.0, 150.0, 100.0],
            [-50.0, 100.0, 150.0, -100.0],
        ]);
        let drawn = one(Guide::DiagonalMethod, GuideFlip::Both, SIZE);
        assert!(same(&drawn, &wide), "{drawn:?}");
        let tall = segments(&[
            [-100.0, -150.0, 100.0, 50.0],
            [-100.0, 50.0, 100.0, -150.0],
            [-100.0, -50.0, 100.0, 150.0],
            [-100.0, 150.0, 100.0, -50.0],
        ]);
        let drawn = one(Guide::DiagonalMethod, GuideFlip::None, vec2(200.0, 300.0));
        assert!(same(&drawn, &tall), "{drawn:?}");
    }

    /// The diagonal and the two lines meeting it at a right angle, `dst` 133, the whole
    /// part of h·cos(atan(w/h)) / cos(atan(h/w)); each flip mirrors them.
    #[test]
    fn harmonious_triangles_flip_as_asked() {
        let drawn = one(Guide::HarmoniousTriangles, GuideFlip::None, SIZE);
        let expected = segments(&[
            [-150.0, -100.0, 150.0, 100.0],
            [-17.0, -100.0, -150.0, 100.0],
            [150.0, -100.0, 17.0, 100.0],
        ]);
        assert!(same(&drawn, &expected), "{drawn:?}");
        for (flip, (sx, sy)) in [
            (GuideFlip::Horizontally, (-1.0, 1.0)),
            (GuideFlip::Vertically, (1.0, -1.0)),
            (GuideFlip::Both, (-1.0, -1.0)),
        ] {
            let mirrored: Vec<Vec<Pos2>> = (expected.iter())
                .map(|l| l.iter().map(|p| pos2(p.x * sx, p.y * sy)).collect())
                .collect();
            let drawn = one(Guide::HarmoniousTriangles, flip, SIZE);
            assert!(same(&drawn, &mirrored), "{flip:?}: {drawn:?}");
        }
    }

    /// The golden mean's rectangles as darktable works them out for 300 by 200.
    #[test]
    fn the_golden_rectangles_are_darktable_s() {
        let expected = [
            [-150.0, -100.0, 185.4102, 200.0],
            [35.4102, -23.6068, 115.5898, 123.6068],
            [78.5616, -100.0, 71.4384, 76.3932],
            [35.4102, -100.0, 43.1514, 47.2136],
            [35.4102, -52.7864, 26.669, 29.1796],
            [62.0792, -41.6408, 16.4824, 18.034],
            [68.3749, -52.7864, 10.1867, 11.1456],
        ];
        for (i, (r, [left, top, width, height])) in golden_rects(SIZE.x, SIZE.y)
            .iter()
            .zip(expected)
            .enumerate()
        {
            let got = [r.left, r.top, r.width, r.height];
            let close =
                (got.iter().zip([left, top, width, height])).all(|(a, b)| (a - b).abs() < 1e-3);
            assert!(close, "R{}: {got:?}", i + 1);
            assert_eq!((r.right, r.bottom), (r.left + r.width, r.top + r.height));
        }
    }

    /// Each golden guide's parts: four sections, seven spiral sections, eight arcs, and all
    /// of them together; the sections where darktable draws them, the spiral's first arc,
    /// from R1's bottom right to the photo's top left, and a flip mirroring them.
    #[test]
    fn the_golden_guides_draw_their_parts() {
        let count = |guide| one(guide, GuideFlip::None, SIZE).len();
        assert_eq!(count(Guide::GoldenSections), 4);
        assert_eq!(count(Guide::GoldenSpiralSections), 7);
        assert_eq!(count(Guide::GoldenSpiral), 8);
        assert_eq!(count(Guide::GoldenMean), 4 + 7 + 8);
        let sections = one(Guide::GoldenSections, GuideFlip::None, SIZE);
        let expected = segments(&[
            [-150.0, -23.6068, 151.0, -23.6068],
            [-150.0, 23.6068, 151.0, 23.6068],
            [35.4102, -100.0, 35.4102, 100.0],
            [-34.4102, -100.0, -34.4102, 100.0],
        ]);
        assert!(same(&sections, &expected), "{sections:?}");
        let spiral = one(Guide::GoldenSpiral, GuideFlip::None, SIZE);
        let first = &spiral[0];
        assert!(
            (first[0] - pos2(35.4102, 100.0)).length() < 1e-3,
            "{first:?}"
        );
        let last = first[first.len() - 1];
        assert!((last - pos2(-150.0, -100.0)).length() < 1e-3, "{last:?}");
        let flipped = one(Guide::GoldenSections, GuideFlip::Horizontally, SIZE);
        let mirrored: Vec<Vec<Pos2>> = (expected.iter())
            .map(|l| l.iter().map(|p| pos2(-p.x, p.y)).collect())
            .collect();
        assert!(same(&flipped, &mirrored), "{flipped:?}");
    }

    /// Each golden arc's segments stay within a quarter point of cairo's curve, on a photo
    /// fitted at 300 by 200 points and on one 96,000 by 64,000, a 6000 by 4000 photo at
    /// 1600%: each segment's middle, the curve's point halfway along its angle, is that
    /// close to the chord. A quarter turn takes 32 at least.
    #[test]
    fn the_golden_arcs_stay_within_a_quarter_point_at_any_size() {
        for size in [SIZE, vec2(96_000.0, 64_000.0)] {
            for (i, ellipse) in spiral_arcs(size.x, size.y).iter().enumerate() {
                let points = arc(ellipse);
                let steps = points.len() - 1;
                let turn = ellipse.to - ellipse.from;
                let least = (turn / FRAC_PI_2 * ARC_STEPS as f32).ceil() as usize;
                assert!(
                    steps >= least && steps <= ARC_MOST,
                    "{size:?}, {i}: {steps}"
                );
                let error = (0..steps)
                    .map(|k| {
                        let mid = ellipse.at(ellipse.from + turn * (k as f32 + 0.5) / steps as f32);
                        let (p, q) = (points[k], points[k + 1]);
                        let along = (q - p).normalized();
                        let off = mid - p;
                        (off - along * off.dot(along)).length()
                    })
                    .fold(0.0, f32::max);
                // f32's own rounding at 96,000 points is about a hundredth of a point.
                assert!(error <= ARC_ERROR + 0.02, "{size:?}, arc {i}: {error}");
            }
        }
    }

    /// Nothing over an empty photo.
    #[test]
    fn an_empty_photo_has_no_guides() {
        for guide in Guide::ALL {
            assert!(lines(guide, GuideFlip::None, Vec2::ZERO).is_empty());
        }
    }
}
