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
//! Built: the view, its placement and the frame's place; past Fit, the region the detail
//! pipeline renders at the scale rendered ([`rendered`]), the visible rectangle with its
//! margin, and whether a pan stays inside it ([`covers`]); a centre mapped through a turn
//! or a flip of the photo ([`reoriented`]).

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
    /// [`place`]'s `render` at Fit, for a frame of the whole photo; the detail's, the
    /// view's `render`, past Fit. Kept in `f64`, as the pipe rounds with it, so a client
    /// rounds to the frame's own size.
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

    /// The place of a detail frame of `region`, x, y, width and height of the output
    /// image `extent` at full resolution at `scale`, rendered at that scale.
    pub(crate) fn detail(extent: [u32; 2], region: [u32; 4], scale: f64) -> FramePlace {
        FramePlace {
            scale,
            origin: [region[0], region[1]],
            extent,
        }
    }
}

/// The placement of `view` of the photo whose output image is `extent` at full resolution
/// in `bounds` where the detail pipeline renders it (decision 99): past Fit, at the scale
/// [`Placement::render`] gives, Fill's, 50% or a pinch's below 100%, 1 from 100% on;
/// `None` where the view renders at Fit: at Fit and Small, at Fill or a factor whose image
/// is the fitted frame's size, and from 100% on where the photo fits the bounds at 100%,
/// whose fitted frame is it at 100% already. Above 100% the engine renders at 1, and the
/// client enlarges the frame.
pub fn zoomed(extent: [u32; 2], bounds: (u32, u32), view: &View) -> Option<Placement> {
    let placed = place(extent, bounds, view);
    let fitted = place(extent, bounds, &View::default());
    // A factor a rounding above Fit, as an `f32` of the fitted scale is, renders the
    // fitted frame's pixels.
    (placed.render > fitted.render && placed.size != fitted.size).then_some(placed)
}

/// darktable's `darkroom/ui/anticipate_move`: how much larger than the visible rectangle,
/// on each side, the detail renders, so a pan inside the margin renders nothing; the
/// default of the setting `viewer.anticipate_move`, which goes from 1.0 to 2.0.
pub const ANTICIPATE_MOVE: f64 = 1.2;

/// The grid, in output pixels at the rendered scale, the rendered region's sides and its
/// corner are snapped to (decision 99).
pub const GRID: u32 = 64;

/// The region the detail renders for `placement` (decision 99): x, y, width and height of
/// the output image at `placement.render`, holding the visible rectangle with a margin, so
/// a pan inside it renders nothing ([`covers`]).
///
/// - Its size on each axis is the visible side times `anticipate` (1.0 to 2.0, else
///   [`ANTICIPATE_MOVE`]), rounded up to a multiple of [`GRID`], but at least the visible
///   side rounded outward from any place, and at most the image's side. It depends on the
///   visible size alone, so every region of a view's size is the same size, and a pan
///   past it updates the detail's plan in place.
/// - It is centred on the visible rectangle, then moved, never shrunk, to lie inside the
///   image, its corner on the grid where the image and the visible rectangle allow.
/// - Where its pixels pass `limit_pixels`, the largest RGBA `f32` image the GPU binds, or
///   fewer where the detail's plan binds more a pixel, as demosaic's images over a mosaic
///   grown by its reach do, `anticipate` shrinks towards 1.0 until they fit; `None` where
///   they pass it even at 1.0, as the fitted viewport is refused above one binding.
pub fn rendered(placement: &Placement, anticipate: f64, limit_pixels: u64) -> Option<[u32; 4]> {
    let anticipate = if anticipate.is_finite() {
        anticipate.clamp(1.0, 2.0)
    } else {
        ANTICIPATE_MOVE
    };
    let grid = f64::from(GRID);
    let sides = |anticipate: f64| {
        [0, 1].map(|axis| {
            let (seen, whole) = (placement.visible[axis + 2], placement.size[axis]);
            let grown = ((seen * anticipate - WHOLE) / grid).ceil() * grid;
            // The visible side rounded outward, wherever its corner lies.
            let least = (seen - WHOLE).ceil() + 1.0;
            (grown.max(least) as u32).min(whole)
        })
    };
    // A hundredth of the margin at a time, down to none.
    let steps = ((anticipate - 1.0) * 100.0).ceil() as u32;
    let [width, height] = (0..=steps)
        .rev()
        .map(|step| sides(1.0 + (anticipate - 1.0) * f64::from(step) / f64::from(steps.max(1))))
        .find(|[width, height]| u64::from(*width) * u64::from(*height) <= limit_pixels)?;
    let corner = |axis: usize, side: u32| {
        let (start, seen) = (placement.visible[axis], placement.visible[axis + 2]);
        let room = f64::from(placement.size[axis] - side);
        let side = f64::from(side);
        // The corners that hold the visible rectangle rounded outward, inside the image.
        let low = ((start + seen - WHOLE).ceil() - side).max(0.0);
        let high = (start + WHOLE).floor().min(room).max(low);
        let centred = start + seen / 2.0 - side / 2.0;
        let [first, last] = [(low / grid).ceil() * grid, (high / grid).floor() * grid];
        let at = if first <= last {
            ((centred / grid).round() * grid).clamp(first, last)
        } else {
            centred.round().clamp(low, high)
        };
        at as u32
    };
    Some([corner(0, width), corner(1, height), width, height])
}

/// Whether `region`, x, y, width and height in output pixels, holds `visible`, a
/// [`Placement::visible`] at the same scale: a pan whose visible rectangle the detail's
/// region holds renders nothing (decision 99).
pub fn covers(region: [u32; 4], visible: [f64; 4]) -> bool {
    let [x, y, width, height] = region.map(f64::from);
    let [left, top, seen_width, seen_height] = visible;
    left + WHOLE >= x
        && top + WHOLE >= y
        && left + seen_width <= x + width + WHOLE
        && top + seen_height <= y + height + WHOLE
}

/// The photo's orientation, as Orientation's settings give it (`apertrust_tools::orientation`):
/// darktable's `dt_image_orientation_t`, flip x, flip y and swap xy, applied to the unturned
/// image in that order (`flip.c`, `distort_transform`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Orientation {
    pub flip_x: bool,
    pub flip_y: bool,
    pub swap_xy: bool,
}

impl Orientation {
    /// The orientation of darktable's flip `bits`.
    pub fn from_bits(bits: u8) -> Orientation {
        use apertrust_tools::orientation::{FLIP_X, FLIP_Y, SWAP_XY};
        Orientation {
            flip_x: bits & FLIP_X != 0,
            flip_y: bits & FLIP_Y != 0,
            swap_xy: bits & SWAP_XY != 0,
        }
    }

    /// The orientation Orientation's option `choice` shows the photo of `image` in, the
    /// photo's own at auto (`apertrust_tools::orientation::shown_bits`).
    pub fn shown(choice: u32, image: &apertrust_plugin::ImageContext) -> Orientation {
        Orientation::from_bits(apertrust_tools::orientation::shown_bits(choice, image))
    }

    /// Where this orientation shows the point `at` of the unturned image, 0 to 1 across and
    /// down.
    pub fn forward(self, [x, y]: [f32; 2]) -> [f32; 2] {
        let x = if self.flip_x { 1.0 - x } else { x };
        let y = if self.flip_y { 1.0 - y } else { y };
        if self.swap_xy { [y, x] } else { [x, y] }
    }

    /// The point of the unturned image this orientation shows at `at`: [`Self::forward`]
    /// undone, the axes swapped back, then the flips.
    pub fn backward(self, [x, y]: [f32; 2]) -> [f32; 2] {
        let [x, y] = if self.swap_xy { [y, x] } else { [x, y] };
        let x = if self.flip_x { 1.0 - x } else { x };
        let y = if self.flip_y { 1.0 - y } else { y };
        [x, y]
    }
}

/// The centre `c`, in the photo's coordinates as orientation `from` shows it, mapped to
/// orientation `to`, so it stays on the same point of the scene: back to the unturned
/// image through `from`, then forward through `to`. Not clamped: [`place`] keeps it inside
/// the edges. The session maps its view through a turn of the photo with it, and a client
/// its own centre, so the two never disagree (decision 99).
pub fn reoriented(c: [f32; 2], from: Orientation, to: Orientation) -> [f32; 2] {
    to.forward(from.backward(c))
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

    /// The region at 100% of `view` of `extent` in `BOUNDS`, with darktable's margin and no
    /// limit.
    fn region_of(extent: [u32; 2], view: &View) -> [u32; 4] {
        let placed = zoomed(extent, BOUNDS, view).expect("a detail");
        rendered(&placed, ANTICIPATE_MOVE, u64::MAX).expect("a region")
    }

    #[test]
    fn the_detail_renders_past_fit() {
        let zoom = |scale| zoomed(EXTENT, BOUNDS, &at(scale, [0.5, 0.5]));
        for (scale, render) in [
            (ViewScale::Factor(0.5), 0.5),
            (ViewScale::Factor(1.0), 1.0),
            (ViewScale::Factor(3.0), 1.0),
        ] {
            let placed = zoom(scale).expect("a detail");
            assert_eq!(placed, place(EXTENT, BOUNDS, &at(scale, [0.5, 0.5])));
            assert_eq!(placed.render, render);
        }
        // Fit, Small, and Fill and factors no larger than Fit, at Fit: a photo of the
        // bounds' shape fills them at Fit.
        for scale in [
            ViewScale::Fit,
            ViewScale::Small,
            ViewScale::Fill,
            ViewScale::Factor(0.4),
            ViewScale::Factor(0.3),
        ] {
            assert_eq!(zoom(scale), None, "{scale:?}");
        }
        // A photo the bounds hold at 100% is rendered at 100% by its fitted frame.
        let small = zoomed([400, 300], BOUNDS, &at(ViewScale::Factor(4.0), [0.5, 0.5]));
        assert_eq!(small, None);
        for scale in [1.0, 0.5] {
            let placed = FramePlace::detail(EXTENT, [1536, 1024, 2880, 1920], scale);
            assert_eq!((placed.scale, placed.origin), (scale, [1536, 1024]));
        }
    }

    #[test]
    fn fill_on_a_photo_of_another_shape_shows_its_shorter_side_whole() {
        // A landscape photo in a portrait panel, and a portrait one in a landscape panel:
        // Fill shows the photo's shorter side whole, at 0.6, and crops its longer side,
        // which the centre moves along; the detail renders it, its region holding it.
        for (extent, bounds, shorter) in [
            ([6000, 4000], (1600, 2400), 1),
            ([4000, 6000], (2400, 1600), 0),
        ] {
            let longer = 1 - shorter;
            let view = at(ViewScale::Fill, [0.9, 0.9]);
            let filled = place(extent, bounds, &view);
            assert_eq!((filled.shown, filled.render), (0.6, 0.6));
            assert_eq!(filled.size[shorter], 2400);
            assert_eq!(filled.size[longer], 3600);
            assert_eq!(filled.visible[shorter], 0.0);
            assert_eq!(filled.visible[shorter + 2], 2400.0);
            let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
            assert!(close(filled.visible[longer + 2], 1600.0), "{filled:?}");
            assert!(close(filled.visible[longer], 3600.0 - 1600.0), "{filled:?}");
            assert_eq!(filled.centre[shorter], 0.5);
            assert!(filled.centre[longer] > 0.5);
            assert_eq!(zoomed(extent, bounds, &view), Some(filled));
            let region = rendered(&filled, ANTICIPATE_MOVE, u64::MAX).expect("a region");
            assert!(covers(region, filled.visible), "{region:?}");
            assert_eq!(region[shorter + 2], 2400, "{region:?}");
        }
    }

    #[test]
    fn the_rendered_region_holds_the_visible_one_with_its_margin() {
        // 2400 × 1600 visible at 100%: 2880 × 1920, 1.2 times it, both multiples of 64,
        // its corner the grid's nearest to the centred one's, 1560 and 1040.
        let view = at(ViewScale::Factor(1.0), [0.5, 0.5]);
        let region = region_of(EXTENT, &view);
        assert_eq!(region, [1536, 1024, 2880, 1920]);
        assert!(covers(region, place(EXTENT, BOUNDS, &view).visible));
        // Any centre, and any zoom from 100% on, rendered at 1: 1.2 times the visible size
        // rounded up to the grid, but at least the visible size rounded outward from
        // anywhere, holding the visible rectangle; its corner on the grid where the margin
        // leaves room.
        for factor in [1.0, 1.5, 3.0, 16.0] {
            for centre in [[0.3, 0.6], [0.5137, 0.4821], [0.71, 0.29]] {
                let view = at(ViewScale::Factor(factor), centre);
                let placed = place(EXTENT, BOUNDS, &view);
                let region = region_of(EXTENT, &view);
                let [x, y, width, height] = region;
                let least = |seen: f64| seen.ceil() as u32 + 1;
                let want = |seen: f64| {
                    let grown = ((seen * 1.2 - 1e-6) / 64.0).ceil() as u32 * 64;
                    grown.max(least(seen))
                };
                let (seen_width, seen_height) = (placed.visible[2], placed.visible[3]);
                let sides = [want(seen_width), want(seen_height)];
                assert_eq!([width, height], sides, "{factor} at {centre:?}");
                assert!(covers(region, placed.visible), "{factor} at {centre:?}");
                if width - least(seen_width) >= 64 {
                    assert_eq!(x % 64, 0, "{factor} at {centre:?}");
                }
                if height - least(seen_height) >= 64 {
                    assert_eq!(y % 64, 0, "{factor} at {centre:?}");
                }
            }
        }
        // A covering region holds the visible rectangle whole; one a pixel short does not.
        let visible = [10.5, 20.0, 100.0, 50.0];
        assert!(covers([10, 20, 101, 50], visible));
        assert!(!covers([11, 20, 101, 50], visible));
        assert!(!covers([10, 20, 100, 50], visible));
        assert!(!covers([10, 21, 101, 50], visible));
    }

    #[test]
    fn the_rendered_region_moves_inside_the_photo_at_its_edges() {
        let middle = region_of(EXTENT, &at(ViewScale::Factor(1.0), [0.5, 0.5]));
        for centre in [
            [0.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.5],
            [1.0, 0.5],
            [0.5, 0.0],
            [0.5, 1.0],
            [0.25, 0.75],
        ] {
            let view = at(ViewScale::Factor(1.0), centre);
            let region = region_of(EXTENT, &view);
            let [x, y, width, height] = region;
            // The same size everywhere, inside the photo, holding the visible rectangle.
            assert_eq!([width, height], [middle[2], middle[3]], "{centre:?}");
            assert!(x + width <= 6000 && y + height <= 4000, "{centre:?}");
            assert!(covers(region, place(EXTENT, BOUNDS, &view).visible));
            // At an edge, against it.
            let edge = |at: f32, low: u32, high: u32| {
                if at == 0.0 {
                    assert_eq!(low, 0, "{centre:?}");
                } else if at == 1.0 {
                    assert_eq!(high, 0, "{centre:?}");
                }
            };
            edge(centre[0], x, 6000 - x - width);
            edge(centre[1], y, 4000 - y - height);
        }
        // The far edges are off the grid, where the photo's sides are.
        let corner = region_of(EXTENT, &at(ViewScale::Factor(1.0), [1.0, 1.0]));
        assert_eq!(corner, [3120, 2080, 2880, 1920]);
        // Across the photo, a fortieth of it at a time, every corner lies on the grid but
        // against the far edge.
        for step in 0..=40 {
            let view = at(ViewScale::Factor(1.0), [step as f32 / 40.0, 0.4]);
            let [x, y, ..] = region_of(EXTENT, &view);
            assert!(x % 64 == 0 || x == 3120, "{x}");
            assert_eq!(y % 64, 0);
        }
    }

    #[test]
    fn the_rendered_region_shrinks_its_margin_to_bind() {
        // 4K bounds on the 7R V's photo, on a GPU that binds 128 MiB: 8 Mi pixels of
        // RGBA `f32`. At 1.2 the region, 4608 × 2624, would take about 11.5 Mi.
        let (extent, bounds) = ([9504, 6336], (3840, 2160));
        let view = at(ViewScale::Factor(1.0), [0.5, 0.5]);
        let placed = zoomed(extent, bounds, &view).expect("a detail");
        let limit = (128 << 20) / 16;
        let margin = rendered(&placed, ANTICIPATE_MOVE, u64::MAX).expect("a region");
        assert_eq!(margin[2..], [4608, 2624]);
        let bound = rendered(&placed, ANTICIPATE_MOVE, limit).expect("a region");
        let [_, _, width, height] = bound;
        assert!(u64::from(width) * u64::from(height) <= limit);
        assert!(width < margin[2] && height < margin[3]);
        assert!(covers(bound, placed.visible));
        // Here, at 1.0: the visible rectangle rounded up to the grid, and wide enough to
        // hold it rounded outward wherever it lies.
        assert_eq!(rendered(&placed, 1.0, limit), Some(bound));
        assert_eq!(bound[2..], [3841, 2176]);
        // Fewer pixels than that, refused.
        assert_eq!(rendered(&placed, ANTICIPATE_MOVE, 3840 * 2160), None);
        // A setting out of its range is taken at the nearer end; one not a number, as the
        // default.
        let unbound = |anticipate| rendered(&placed, anticipate, u64::MAX);
        assert_eq!(unbound(0.5), unbound(1.0));
        assert_eq!(unbound(3.0), unbound(2.0));
        assert_eq!(unbound(f64::NAN), Some(margin));
    }

    #[test]
    fn a_photo_smaller_than_the_region_is_rendered_whole() {
        // 1.2 times 2400 × 1600 is more than the photo: the whole photo.
        let extent = [2600, 1700];
        for centre in [[0.5, 0.5], [0.0, 0.0], [1.0, 0.3]] {
            let view = at(ViewScale::Factor(1.0), centre);
            assert_eq!(region_of(extent, &view), [0, 0, 2600, 1700], "{centre:?}");
        }
        // One side whole, the other with its margin.
        let view = at(ViewScale::Factor(1.0), [0.25, 0.5]);
        assert_eq!(region_of([6000, 1000], &view), [64, 0, 2880, 1000]);
    }

    /// darktable's eight orientations, by their flip bits.
    fn orientations() -> impl Iterator<Item = Orientation> {
        (0..8).map(Orientation::from_bits)
    }

    /// Points across the photo: its corners, its middle and some between.
    const POINTS: [[f32; 2]; 7] = [
        [0.0, 0.0],
        [1.0, 0.0],
        [0.0, 1.0],
        [0.5, 0.5],
        [0.3, 0.7],
        [0.9, 0.1],
        [0.25, 0.6],
    ];

    fn close_at(a: [f32; 2], b: [f32; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-5 && (a[1] - b[1]).abs() < 1e-5
    }

    #[test]
    fn reoriented_round_trips_through_every_orientation() {
        for from in orientations() {
            for to in orientations() {
                for c in POINTS {
                    let there = reoriented(c, from, to);
                    let back = reoriented(there, to, from);
                    assert!(close_at(back, c), "{from:?} to {to:?}: {c:?} {back:?}");
                }
            }
            // Through itself, nothing moves.
            for c in POINTS {
                assert!(close_at(reoriented(c, from, from), c), "{from:?}");
            }
        }
    }

    /// A quarter turn clockwise maps (x, y) to (1 − y, x) and a horizontal flip to
    /// (1 − x, y); every one of darktable's four buttons, from every orientation, keeps the
    /// centre on the scene's point it showed: the unturned image's point under it is the
    /// same before and after.
    #[test]
    fn a_quarter_turn_maps_the_centre_onto_the_same_scene_point() {
        use apertrust_tools::orientation::{ACTIONS, turn};
        let none = Orientation::default();
        let bits = |action| Orientation::from_bits(turn(action, 0).expect("a button"));
        for c in POINTS {
            let [x, y] = c;
            assert!(close_at(
                reoriented(c, none, bits("rotate_cw")),
                [1.0 - y, x]
            ));
            assert!(close_at(
                reoriented(c, none, bits("rotate_ccw")),
                [y, 1.0 - x]
            ));
            let flipped = reoriented(c, none, bits("flip_horizontally"));
            assert!(close_at(flipped, [1.0 - x, y]));
            assert!(close_at(
                reoriented(c, none, bits("flip_vertically")),
                [x, 1.0 - y]
            ));
        }
        for start in 0..8 {
            let from = Orientation::from_bits(start);
            for action in ACTIONS {
                let to = Orientation::from_bits(turn(action, start).expect("a button"));
                for c in POINTS {
                    let scene = from.backward(c);
                    let mapped = reoriented(c, from, to);
                    assert!(close_at(to.backward(mapped), scene), "{from:?} {action}");
                }
            }
        }
    }
}
