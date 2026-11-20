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

//! The zoomed view (decision 99; `docs/engine.md`, The zoomed view): the zoom and centre a
//! client asks the session to show ([`View`]), where they place the photo in the bounds of
//! the last `Resize` ([`place`]), and where a frame lies in the photo ([`FramePlace`]).
//!
//! [`place`] is pure and the one placement the session and every client use, so they never
//! disagree on a scale, a rounding or a clamp. A client works out a resize at Fill, or the
//! centre a pan reaches, without a round trip.
//!
//! Built: the view, its placement and the frame's place; the session renders the whole
//! photo at Fit at every view. The detail pipeline, which renders the visible region at the
//! zoom's scale, is to come.

use crate::pipeline::{at_scale, fit_scale};

/// The most a view enlarges the photo: 1600%, sixteen physical pixels a photo pixel.
pub const MAX_FACTOR: f32 = 16.0;

/// How far a side of the visible rectangle may fall short of the image's and still count as
/// the whole of it: what rounding leaves where the photo fits the bounds exactly.
const WHOLE: f64 = 1e-6;

/// The zoom a client asks for (decision 99).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewScale {
    /// Half of Fit: the fitted frame, which the client draws at half, an exact 2:1 box; the
    /// engine renders nothing more for it.
    Small,
    /// The whole photo in the bounds, never above 100%: `pipeline::fit`'s scale.
    Fit,
    /// The bounds filled: the larger of the bounds' ratios to the photo's sides, so the
    /// photo's other side is cropped; never below Fit nor above [`MAX_FACTOR`].
    Fill,
    /// Physical pixels per photo pixel, 1.0 being 100%: one photo pixel per physical pixel.
    /// Clamped to Fit and [`MAX_FACTOR`]; one not finite or not above 0 is Fit.
    Factor(f32),
}

/// The zoom and its centre a client asks the session to show
/// ([`crate::session::Command::View`]): one view per session, shared by its clients.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub scale: ViewScale,
    /// The point shown at the bounds' centre, in the photo's coordinates: 0 to 1 across the
    /// photo as export renders it, after every geometry, (0, 0) its top left, x to the right
    /// and y down; the colour pickers' coordinates (decision 79).
    pub centre: [f32; 2],
}

impl Default for View {
    /// Fit, centred.
    fn default() -> View {
        View {
            scale: ViewScale::Fit,
            centre: [0.5, 0.5],
        }
    }
}

impl View {
    /// The view as the session holds it and [`place`] reads it: a factor not finite or not
    /// above 0 taken as Fit, one above [`MAX_FACTOR`] as that; each coordinate of the centre
    /// clamped to 0 to 1, one not finite taken as 0.5.
    pub fn normalized(self) -> View {
        let scale = match self.scale {
            ViewScale::Factor(factor) if !(factor.is_finite() && factor > 0.0) => ViewScale::Fit,
            ViewScale::Factor(factor) => ViewScale::Factor(factor.min(MAX_FACTOR)),
            scale => scale,
        };
        let centre = (self.centre).map(|at| {
            if at.is_finite() {
                at.clamp(0.0, 1.0)
            } else {
                0.5
            }
        });
        View { scale, centre }
    }
}

/// Where a view places the photo in the bounds: [`place`]'s answer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Physical pixels per photo pixel on screen: the fitted scale at Fit, half of it at
    /// Small, the larger ratio at Fill, else the factor; but for Small, clamped to the
    /// fitted scale and [`MAX_FACTOR`].
    pub shown: f64,
    /// The scale the engine renders at: `min(shown, 1)`, and the fitted scale at Fit and
    /// Small. Above 1 the client enlarges the frame.
    pub render: f64,
    /// The output image at `render`: the extent at that scale, each side rounded and at
    /// least 1, as `pipeline::fit` rounds; at Fit, the frame's size.
    pub size: [u32; 2],
    /// The visible rectangle in output pixels at `render`: x, y, width and height. It lies
    /// inside `size`, and is the whole image on an axis where the photo at `shown` fits the
    /// bounds, and at Fit and Small.
    pub visible: [f64; 4],
    /// The view's centre, clamped so `visible` never passes the photo's edges: 0.5 on an
    /// axis where the photo fits the bounds.
    pub centre: [f32; 2],
}

/// Where `view` places the photo whose output image is `extent` at full resolution
/// (`Values::extent`) in `bounds`, the last `Resize`'s physical pixels: the scale shown,
/// the scale rendered, the image at it, the visible rectangle and the centre clamped
/// (decision 99). At Fit it agrees with `Values::viewport_size`.
pub fn place(extent: [u32; 2], bounds: (u32, u32), view: &View) -> Placement {
    let view = view.normalized();
    let extent = extent.map(|side| side.max(1));
    let fitted = fit_scale((extent[0], extent[1]), bounds);
    let shown = match view.scale {
        ViewScale::Small => fitted / 2.0,
        ViewScale::Fit => fitted,
        ViewScale::Fill => {
            let ratio = |bound: u32, side: u32| f64::from(bound) / f64::from(side);
            let fill = ratio(bounds.0, extent[0]).max(ratio(bounds.1, extent[1]));
            fill.clamp(fitted, f64::from(MAX_FACTOR))
        }
        ViewScale::Factor(factor) => f64::from(factor).clamp(fitted, f64::from(MAX_FACTOR)),
    };
    let render = match view.scale {
        ViewScale::Small | ViewScale::Fit => fitted,
        ViewScale::Fill | ViewScale::Factor(_) => shown.min(1.0),
    };
    let size = at_scale(extent, render);
    let mut visible = [0.0, 0.0, f64::from(size[0]), f64::from(size[1])];
    let mut centre = [0.5; 2];
    if matches!(view.scale, ViewScale::Fill | ViewScale::Factor(_)) {
        for (axis, bound) in [bounds.0, bounds.1].into_iter().enumerate() {
            let side = f64::from(size[axis]);
            // The bounds' side in output pixels at `render`: the whole image where the photo
            // fits it, and where empty bounds at Fill leave it not a number.
            let seen = f64::from(bound) / shown * render;
            if seen.is_nan() || seen + WHOLE >= side {
                continue;
            }
            let half = seen / side / 2.0;
            let at = f64::from(view.centre[axis]).clamp(half, 1.0 - half);
            centre[axis] = at as f32;
            visible[axis] = (at * side - seen / 2.0).clamp(0.0, side - seen);
            visible[axis + 2] = seen;
        }
    }
    Placement {
        shown,
        render,
        size,
        visible,
        centre,
    }
}

/// Where a frame lies in the photo (decision 99), so a client draws it in place, pans it
/// and marks a pick on it: `Frame::place`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FramePlace {
    /// The scale the frame was rendered at, at most 1: the output image at it is `extent`
    /// at that scale, each side rounded, as [`Placement::size`] is. The fitted scale,
    /// [`place`]'s `render` at Fit, for every frame for now, whatever the view; kept in
    /// `f64`, as the pipe rounds with it, so a client rounds to the frame's own size.
    pub scale: f64,
    /// The frame's top-left corner in the output image at `scale`, in pixels: (0, 0) at Fit.
    pub origin: [u32; 2],
    /// The output image at full resolution, as `Values::extent`.
    pub extent: [u32; 2],
}

impl FramePlace {
    /// The place of a frame of the whole photo, its output image `extent` at full
    /// resolution, fitted in `bounds`: [`place`]'s at Fit.
    pub(crate) fn fitted(extent: [u32; 2], bounds: (u32, u32)) -> FramePlace {
        let fitted = place(extent, bounds, &View::default());
        FramePlace {
            scale: fitted.render,
            origin: [0, 0],
            extent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::fit;

    fn at(scale: ViewScale, centre: [f32; 2]) -> View {
        View { scale, centre }
    }

    /// A 3:2 photo of 6000 × 4000 in bounds of 2400 × 1600: fitted at 0.4.
    const EXTENT: [u32; 2] = [6000, 4000];
    const BOUNDS: (u32, u32) = (2400, 1600);

    #[test]
    fn fit_places_the_whole_photo_as_pipeline_fit_does() {
        for (extent, bounds) in [
            ([6000, 4000], (2400, 1600)),
            ([6000, 4000], (1000, 1000)),
            ([4000, 6000], (2400, 1600)),
            ([9504, 6336], (2560, 1440)),
            ([9504, 6336], (1311, 977)),
            ([8256, 5504], (3840, 2160)),
            ([7008, 4672], (1, 1)),
            ([84, 62], (40, 40)),
            ([84, 62], (300, 200)),
            ([62, 84], (40, 40)),
            ([1088, 62], (640, 480)),
            ([400, 300], (2400, 1600)),
            ([5, 7000], (1920, 1080)),
        ] {
            for centre in [[0.5, 0.5], [0.0, 1.0], [0.9, 0.1]] {
                let fitted = place(extent, bounds, &at(ViewScale::Fit, centre));
                let (width, height) = fit((extent[0], extent[1]), bounds);
                assert_eq!(fitted.size, [width, height], "{extent:?} in {bounds:?}");
                let scale = fit_scale((extent[0], extent[1]), bounds);
                assert_eq!((fitted.shown, fitted.render), (scale, scale));
                let whole = [0.0, 0.0, f64::from(width), f64::from(height)];
                assert_eq!(fitted.visible, whole, "{extent:?} in {bounds:?}");
                assert_eq!(fitted.centre, [0.5, 0.5]);
            }
        }
    }

    #[test]
    fn small_shows_the_fitted_frame_at_half() {
        let small = place(EXTENT, BOUNDS, &at(ViewScale::Small, [0.0, 0.0]));
        let fitted = place(EXTENT, BOUNDS, &View::default());
        assert_eq!(small.shown, fitted.shown / 2.0);
        assert_eq!(small.render, fitted.render);
        assert_eq!((small.size, small.visible), (fitted.size, fitted.visible));
        assert_eq!(small.centre, [0.5, 0.5]);
    }

    #[test]
    fn fill_takes_the_larger_ratio() {
        // A 3:2 photo in a square panel: its height fills it, the middle of its width shows.
        let filled = place(EXTENT, (1000, 1000), &at(ViewScale::Fill, [0.5, 0.5]));
        assert_eq!(filled.shown, 0.25);
        assert_eq!(filled.render, 0.25);
        assert_eq!(filled.size, [1500, 1000]);
        assert_eq!(filled.visible, [250.0, 0.0, 1000.0, 1000.0]);
        assert_eq!(filled.centre, [0.5, 0.5]);
        // Its centre moves along the width alone.
        let left = place(EXTENT, (1000, 1000), &at(ViewScale::Fill, [0.0, 0.0]));
        assert_eq!(left.visible, [0.0, 0.0, 1000.0, 1000.0]);
        assert!((left.centre[0] - 1.0 / 3.0).abs() < 1e-6);
        assert_eq!(left.centre[1], 0.5);
    }

    #[test]
    fn a_factor_is_clamped_to_fit_and_16() {
        let fitted = place(EXTENT, BOUNDS, &View::default());
        for factor in [0.01, 0.0, -1.0, f32::NAN, f32::INFINITY] {
            let view = at(ViewScale::Factor(factor), [0.5, 0.5]);
            assert_eq!(place(EXTENT, BOUNDS, &view), fitted, "{factor}");
        }
        // Above 16, 16.
        let most = place(EXTENT, BOUNDS, &at(ViewScale::Factor(40.0), [0.5, 0.5]));
        assert_eq!(most.shown, 16.0);
        assert_eq!(
            most,
            place(EXTENT, BOUNDS, &at(ViewScale::Factor(16.0), [0.5, 0.5]))
        );
        // As the session holds them.
        let held = |factor| at(ViewScale::Factor(factor), [0.5, 0.5]).normalized().scale;
        assert_eq!(held(f32::NAN), ViewScale::Fit);
        assert_eq!(held(-1.0), ViewScale::Fit);
        assert_eq!(held(40.0), ViewScale::Factor(16.0));
        assert_eq!(held(0.01), ViewScale::Factor(0.01));
    }

    #[test]
    fn the_centre_is_clamped_at_every_edge_and_corner() {
        let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
        for centre in [[0.0, 0.0], [1.0, 1.0], [0.0, 1.0], [1.0, 0.0]] {
            let placed = place(EXTENT, BOUNDS, &at(ViewScale::Factor(1.0), centre));
            let [x, y, width, height] = placed.visible;
            assert_eq!((width, height), (2400.0, 1600.0));
            // Touching the edges the centre asks for, and inside the photo.
            assert!(close(
                x,
                if centre[0] == 0.0 {
                    0.0
                } else {
                    6000.0 - width
                }
            ));
            assert!(close(
                y,
                if centre[1] == 0.0 {
                    0.0
                } else {
                    4000.0 - height
                }
            ));
            assert!(x >= 0.0 && y >= 0.0 && x + width <= 6000.0 && y + height <= 4000.0);
            // The centre as clamped is the visible rectangle's.
            let middle = [(x + width / 2.0) / 6000.0, (y + height / 2.0) / 4000.0];
            for (centre, middle) in placed.centre.into_iter().zip(middle) {
                assert!((f64::from(centre) - middle).abs() < 1e-6);
            }
        }
        // A centre outside the photo, or not a number, as the nearest inside.
        let outside = place(EXTENT, BOUNDS, &at(ViewScale::Factor(1.0), [-3.0, 7.0]));
        let corner = place(EXTENT, BOUNDS, &at(ViewScale::Factor(1.0), [0.0, 1.0]));
        assert_eq!(outside, corner);
        let nan = place(EXTENT, BOUNDS, &at(ViewScale::Factor(1.0), [f32::NAN, 0.5]));
        assert_eq!(nan.centre, [0.5, 0.5]);
    }

    #[test]
    fn above_100_the_render_scale_is_1() {
        for factor in [2.0, 16.0] {
            let placed = place(EXTENT, BOUNDS, &at(ViewScale::Factor(factor), [0.5, 0.5]));
            assert_eq!(placed.render, 1.0);
            assert_eq!(placed.shown, f64::from(factor));
            assert_eq!(placed.size, EXTENT);
            let seen = [2400.0 / f64::from(factor), 1600.0 / f64::from(factor)];
            assert_eq!(placed.visible[2..], seen);
            assert_eq!(
                placed.visible[..2],
                [3000.0 - seen[0] / 2.0, 2000.0 - seen[1] / 2.0]
            );
        }
        // At 100%, one photo pixel per physical pixel; at 50%, rendered at 0.5.
        let full = place(EXTENT, BOUNDS, &at(ViewScale::Factor(1.0), [0.5, 0.5]));
        assert_eq!(
            (full.render, full.visible[2..].to_vec()),
            (1.0, vec![2400.0, 1600.0])
        );
        let half = place(EXTENT, BOUNDS, &at(ViewScale::Factor(0.5), [0.5, 0.5]));
        assert_eq!((half.render, half.size), (0.5, [3000, 2000]));
        assert_eq!(half.visible, [300.0, 200.0, 2400.0, 1600.0]);
    }

    #[test]
    fn a_photo_smaller_than_the_panel_fits_at_1() {
        let extent = [400, 300];
        let fitted = place(extent, BOUNDS, &View::default());
        assert_eq!(
            (fitted.shown, fitted.render, fitted.size),
            (1.0, 1.0, extent)
        );
        assert_eq!(fitted.visible, [0.0, 0.0, 400.0, 300.0]);
        // 100% is Fit; Fill is above 1, its width whole and its height cropped.
        let full = place(extent, BOUNDS, &at(ViewScale::Factor(1.0), [0.2, 0.2]));
        assert_eq!(full, fitted);
        let filled = place(extent, BOUNDS, &at(ViewScale::Fill, [0.5, 0.0]));
        assert_eq!(
            (filled.shown, filled.render, filled.size),
            (6.0, 1.0, extent)
        );
        assert_eq!(filled.visible[..3], [0.0, 0.0, 400.0]);
        assert!((filled.visible[3] - 1600.0 / 6.0).abs() < 1e-9);
        assert_eq!(filled.centre[0], 0.5);
        assert!(filled.centre[1] > 0.0);
    }

    #[test]
    fn a_quarter_turned_extent_swaps_its_sides() {
        let swap = |pair: [f64; 2]| [pair[1], pair[0]];
        for scale in [
            ViewScale::Small,
            ViewScale::Fit,
            ViewScale::Fill,
            ViewScale::Factor(0.5),
            ViewScale::Factor(1.0),
            ViewScale::Factor(3.0),
        ] {
            let level = place([6000, 4000], (2400, 1500), &at(scale, [0.2, 0.9]));
            let turned = place([4000, 6000], (1500, 2400), &at(scale, [0.9, 0.2]));
            assert_eq!((turned.shown, turned.render), (level.shown, level.render));
            assert_eq!(turned.size, [level.size[1], level.size[0]]);
            let [x, y, width, height] = level.visible;
            assert_eq!(turned.visible[..2], swap([x, y]), "{scale:?}");
            assert_eq!(turned.visible[2..], swap([width, height]), "{scale:?}");
            assert_eq!(turned.centre, [level.centre[1], level.centre[0]]);
        }
    }

    #[test]
    fn a_fitted_frame_s_place_is_fit_s() {
        let framed = FramePlace::fitted(EXTENT, BOUNDS);
        assert_eq!(framed.scale, 0.4);
        assert_eq!((framed.origin, framed.extent), ([0, 0], EXTENT));
    }
}
