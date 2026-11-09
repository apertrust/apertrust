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

//! The marker on a slider's track, the handle that points at its value (decision 75,
//! `docs/gui.md`, How the rows are laid out): the shape the user chooses in Settings, the
//! same on the slider rows and the curve's handles.
//!
//! Everything that differs between shapes is answered here, so a component asks the
//! marker and never which shape it is: how it is drawn at rest, under the pointer and
//! while dragged ([`Marker::paint`]), and where its travel stops short of a track's ends
//! so it stays whole ([`Marker::travel`]). Each takes the pointer where its component's
//! strip does, so none has a hit area of its own. The app sets the shape once, as it sets
//! the theme ([`set_marker`]); a new shape is a variant here and nothing else.

use apertrust_base::settings::Choice;
use egui::{
    Color32, Id, Painter, Pos2, Rangef, Rect, Response, Shape, Stroke, StrokeKind, pos2, vec2,
};

use crate::Theme;

/// The marker's shapes, each centred on the track's middle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Marker {
    /// darktable's, 13 × 11 points, pointing at the value from below.
    Triangle,
    /// 11 points across.
    Circle,
    /// 10 × 11 points, its corners rounded 2.5: the default.
    #[default]
    Square,
}

/// The slider marker setting's options (decision 43).
impl Choice for Marker {
    const OPTIONS: &'static [(Marker, &'static str)] = &[
        (Marker::Triangle, "triangle"),
        (Marker::Circle, "circle"),
        (Marker::Square, "square"),
    ];
}

/// The width of a handle's outline, and of its border while dragged.
const OUTLINE: f32 = 1.5;
/// How far the triangle's tip and base sit each way of its centre, and its base's half
/// width, as the wireframe draws it.
const TRIANGLE_HALF_HEIGHT: f32 = 5.5;
const TRIANGLE_HALF_WIDTH: f32 = 6.5;
const CIRCLE_RADIUS: f32 = 5.5;
const SQUARE: egui::Vec2 = vec2(10.0, 11.0);
const SQUARE_RADIUS: f32 = 2.5;
/// The border's corners inside the square's, as the wireframe rounds them.
const SQUARE_BORDER_RADIUS: f32 = 1.25;
/// The centre line the circle and the square show while dragged.
const CENTRE_LINE: f32 = 3.5;
const CENTRE_LINE_WIDTH: f32 = 1.0;

impl Marker {
    pub const ALL: [Marker; 3] = [Marker::Triangle, Marker::Circle, Marker::Square];

    /// How far its travel stops short of each end of a track: half its width where it
    /// crosses the track plus half its outline, so it sits flush with the end.
    pub const fn stop(self) -> f32 {
        match self {
            Marker::Triangle => 5.0,
            Marker::Circle => 6.25,
            Marker::Square => 5.75,
        }
    }

    /// Where its centre runs along `track`, a track's span in x: short of each end by its
    /// stop, or the track's middle alone on a track too short for that.
    pub fn travel(self, track: Rangef) -> Rangef {
        let stop = self.stop().min(track.span() / 2.0);
        Rangef::new(track.min + stop, track.max - stop)
    }

    /// Paints it centred on `centre` for `response`, the control that moves it: brighter
    /// under the pointer, and while dragged with an accent border just inside its edge, so
    /// it never grows, the circle and the square adding a short centre line in the accent.
    /// `fade` dims it on a disabled row. A marker that can't move looks as it always does.
    pub(crate) fn paint(
        self,
        painter: &Painter,
        centre: Pos2,
        response: &Response,
        t: &Theme,
        fade: impl Fn(Color32) -> Color32,
    ) {
        let movable = response.sense.senses_drag();
        let dragged = movable && response.is_pointer_button_down_on();
        let hovered = movable && (dragged || response.hovered());
        let fill = fade(if hovered {
            t.colors.slider_handle_hover
        } else {
            t.colors.slider_handle
        });
        let outline = Stroke::new(OUTLINE, t.colors.slider_handle_outline);
        // The outline's inner half covers the fill's edge; the border starts where it ends.
        let border = Stroke::new(OUTLINE, fade(t.colors.slider_handle_active));
        match self {
            Marker::Triangle => {
                let points = [
                    pos2(centre.x, centre.y - TRIANGLE_HALF_HEIGHT),
                    pos2(
                        centre.x + TRIANGLE_HALF_WIDTH,
                        centre.y + TRIANGLE_HALF_HEIGHT,
                    ),
                    pos2(
                        centre.x - TRIANGLE_HALF_WIDTH,
                        centre.y + TRIANGLE_HALF_HEIGHT,
                    ),
                ];
                painter.add(Shape::convex_polygon(points.to_vec(), fill, outline));
                if dragged {
                    painter.add(Shape::closed_line(inset(points, OUTLINE).to_vec(), border));
                }
            }
            Marker::Circle => {
                painter.circle(centre, CIRCLE_RADIUS, fill, outline);
                if dragged {
                    painter.circle_stroke(centre, CIRCLE_RADIUS - OUTLINE, border);
                }
            }
            Marker::Square => {
                let rect = Rect::from_center_size(centre, SQUARE);
                painter.rect(rect, SQUARE_RADIUS, fill, outline, StrokeKind::Middle);
                if dragged {
                    let inner = rect.shrink(OUTLINE);
                    painter.rect_stroke(inner, SQUARE_BORDER_RADIUS, border, StrokeKind::Middle);
                }
            }
        }
        if dragged && self != Marker::Triangle {
            let half = CENTRE_LINE / 2.0;
            painter.line_segment(
                [
                    pos2(centre.x, centre.y - half),
                    pos2(centre.x, centre.y + half),
                ],
                Stroke::new(CENTRE_LINE_WIDTH, border.color),
            );
        }
    }
}

/// Makes `marker` the one every slider and curve draws, from the next frame.
pub fn set_marker(ctx: &egui::Context, marker: Marker) {
    ctx.data_mut(|data| data.insert_temp(marker_id(), marker));
}

/// The marker in use; the default until [`set_marker`] is called.
pub fn marker(ctx: &egui::Context) -> Marker {
    ctx.data(|data| data.get_temp::<Marker>(marker_id()))
        .unwrap_or_default()
}

fn marker_id() -> Id {
    Id::unique("apertrust-ui marker")
}

/// The triangle `points` with every edge moved `by` inwards: scaled about its incentre.
fn inset(points: [Pos2; 3], by: f32) -> [Pos2; 3] {
    let [a, b, c] = points;
    let sides = [(b - c).length(), (c - a).length(), (a - b).length()];
    let perimeter: f32 = sides.iter().sum();
    let centre =
        (a.to_vec2() * sides[0] + b.to_vec2() * sides[1] + c.to_vec2() * sides[2]) / perimeter;
    let area = (b - a).x * (c - a).y - (b - a).y * (c - a).x;
    let radius = area.abs() / perimeter;
    let scale = ((radius - by) / radius).max(0.0);
    points.map(|p| (centre + (p.to_vec2() - centre) * scale).to_pos2())
}

#[cfg(test)]
mod tests {
    use egui::{Rangef, pos2};

    use super::{Marker, inset};

    #[test]
    fn an_inset_handle_keeps_its_edges_parallel_at_the_distance() {
        let triangle = [pos2(0.0, -6.5), pos2(6.5, 4.5), pos2(-6.5, 4.5)];
        let inner = inset(triangle, 1.5);
        // Each inner corner lies 1.5 from both edges it sits between.
        let distance = |p: egui::Pos2, from: egui::Pos2, to: egui::Pos2| {
            let edge = to - from;
            ((p - from).x * edge.y - (p - from).y * edge.x).abs() / edge.length()
        };
        for i in 0..3 {
            let (prev, next) = (triangle[(i + 2) % 3], triangle[(i + 1) % 3]);
            assert!((distance(inner[i], triangle[i], prev) - 1.5).abs() < 1e-4);
            assert!((distance(inner[i], triangle[i], next) - 1.5).abs() < 1e-4);
        }
    }

    #[test]
    fn each_marker_stops_flush_with_the_track_s_ends() {
        let track = Rangef::new(10.0, 210.0);
        for (marker, stop) in [
            (Marker::Triangle, 5.0),
            (Marker::Circle, 6.25),
            (Marker::Square, 5.75),
        ] {
            assert_eq!(marker.travel(track), Rangef::new(10.0 + stop, 210.0 - stop));
        }
        // On a track too short for its stops, the middle.
        assert_eq!(
            Marker::Circle.travel(Rangef::new(0.0, 8.0)),
            Rangef::point(4.0)
        );
    }
}
