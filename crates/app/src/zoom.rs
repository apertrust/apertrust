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

//! The Viewer's zoom (`docs/gui.md`, The zoom): a mode, Small, Fit or Fill, or a factor,
//! with the view's centre in the photo's coordinates. It is interface state, never the
//! edit's: no history step, nothing in settings, and undo leaves it.
//!
//! A mode is worked out again from the room each frame, so it follows the window; a
//! factor keeps its scale. The session is told the view (`Command::View`, decision 99),
//! and the app places the photo by the same pure function the session does
//! (`apertrust_engine::view::place`), so the two never disagree on a scale or a clamp.
//!
//! A zoom about the pointer, the keys', a wheel's notch, a pinch or middle-click's, keeps
//! the photo's point under it still ([`anchored`]), the centre then kept inside the edges by
//! the same placement. A notch and a gesture stop on Fit, 100% and 200% ([`notch`],
//! [`snapped`]), and never go below Fit.

use std::time::{Duration, Instant};

use apertrust_engine::session::{ControlValue, Values};
use apertrust_engine::view::{self, MAX_FACTOR, Placement, View, ViewScale};
use apertrust_plugin::ImageContext;
use apertrust_tools::orientation;
use apertrust_ui::components::{ViewerBarAction, Zoom};
use eframe::egui;

/// The Viewer's zoom: interface state, never the edit's (`docs/gui.md`, The zoom).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zoomed {
    pub scale: Scale,
    /// The view's centre, 0 to 1 across the photo as export renders it, (0, 0) its top
    /// left, as pickers take points; Small and Fit centre the photo.
    pub centre: [f32; 2],
}

/// The zoom's scale: a mode worked out from the room, or a factor that stays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Scale {
    /// Half of Fit.
    Small,
    /// The whole photo in the room, never above 1.
    Fit,
    /// The room covered, what overflows panned.
    Fill,
    /// Physical pixels per photo pixel, 1 being 100%.
    Factor(f32),
}

/// A move of the zoom, as an action or the zoom menu asks for it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ZoomStep {
    /// The next scale up, and down, in order of scale ([`step`]).
    In,
    Out,
    Fit,
    /// 100%, and from 100% back to Fit, as Z does.
    Hundred,
    /// That scale.
    To(Scale),
}

/// The photo and the room it is shown in, which a mode's scale and the edges follow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Room {
    /// The photo as the edit shows it, at full resolution (`Values::extent`).
    pub extent: [u32; 2],
    /// The room the photo frame leaves it, in physical pixels, as `Command::Resize` takes it.
    pub bounds: (u32, u32),
}

impl Room {
    /// Where `zoom` places the photo in the room, as the session places it.
    pub fn place(&self, zoom: &Zoomed) -> Placement {
        view::place(self.extent, self.bounds, &zoom.view())
    }

    /// Fit's scale and Fill's in this room, physical pixels per photo pixel.
    pub fn modes(&self) -> (f32, f32) {
        let at = |scale| {
            self.place(&Zoomed {
                scale,
                ..Zoomed::FIT
            })
            .shown as f32
        };
        (at(Scale::Fit), at(Scale::Fill))
    }
}

/// Whether the photo overflows the room on either axis where `placement` puts it, so it
/// can move: never at Fit or Small.
pub fn overflows(placement: &Placement) -> bool {
    let [_, _, width, height] = placement.visible;
    width < f64::from(placement.size[0]) || height < f64::from(placement.size[1])
}

impl Zoomed {
    /// The zoom a photo opens at: the whole photo, centred.
    pub const FIT: Zoomed = Zoomed {
        scale: Scale::Fit,
        centre: [0.5, 0.5],
    };

    /// The view the session is asked to render (decision 99).
    pub fn view(&self) -> View {
        let scale = match self.scale {
            Scale::Small => ViewScale::Small,
            Scale::Fit => ViewScale::Fit,
            Scale::Fill => ViewScale::Fill,
            Scale::Factor(factor) => ViewScale::Factor(factor),
        };
        View {
            scale,
            centre: self.centre,
        }
    }

    /// What the viewer bar's button shows in `room`: [`Self::label`], but Fit for a factor
    /// below Fit, which the room shows at Fit (decision 99).
    pub fn label_in(&self, room: Option<&Room>) -> Zoom {
        match (self.scale, room) {
            (Scale::Factor(factor), Some(room)) if factor < room.modes().0 => Zoom::Fit,
            _ => self.label(),
        }
    }

    /// What the viewer bar's button shows: the mode's word, or the factor as a percentage.
    pub fn label(&self) -> Zoom {
        match self.scale {
            Scale::Small => Zoom::Small,
            Scale::Fit => Zoom::Fit,
            Scale::Fill => Zoom::Fill,
            Scale::Factor(factor) => Zoom::Percent(f64::from(factor) * 100.0),
        }
    }

    /// The zoom after `step` in `room` about the view's centre, as the zoom menu's entries
    /// take it ([`Self::step_about`]).
    pub fn step(self, step: ZoomStep, room: &Room) -> Option<Zoomed> {
        self.step_about(step, egui::Vec2::ZERO, room)
    }

    /// The zoom after `step` in `room`, or None where it has nowhere to go: Zoom in at
    /// 1600% and Zoom out at Small. A scale keeps the screen point `about` still, in
    /// physical pixels from the photo area's centre, zero for the view's centre
    /// ([`Self::about`]); Fit and Small centre the photo. Z's step goes to 100%, and from
    /// 100% back to Fit.
    pub fn step_about(self, step: ZoomStep, about: egui::Vec2, room: &Room) -> Option<Zoomed> {
        let (fit, fill) = room.modes();
        let scale = match step {
            // From the scale shown, which a room grown past a factor clamps to Fit.
            ZoomStep::In | ZoomStep::Out => {
                let shown = room.place(&self).shown as f32;
                self::step(shown, step == ZoomStep::In, fit, fill)?
            }
            ZoomStep::Fit => Scale::Fit,
            ZoomStep::Hundred if self.scale == Scale::Factor(1.0) => Scale::Fit,
            ZoomStep::Hundred => Scale::Factor(1.0),
            // A factor below Fit shows at Fit (decision 99): it is Fit.
            ZoomStep::To(Scale::Factor(factor)) if factor < fit => Scale::Fit,
            ZoomStep::To(scale) => scale,
        };
        Some(self.about(scale, about, room))
    }

    /// The zoom at `scale` in `room`, keeping the screen point `about`, in physical pixels
    /// from the photo area's centre, on the same point of the photo ([`anchored`]), its
    /// centre then kept inside the edges as the session keeps it (decision 99). Fit and
    /// Small centre the photo.
    pub fn about(self, scale: Scale, about: egui::Vec2, room: &Room) -> Zoomed {
        let centre = match scale {
            Scale::Small | Scale::Fit => Zoomed::FIT.centre,
            Scale::Fill | Scale::Factor(_) => {
                let placed = room.place(&self);
                let to = room.place(&Zoomed { scale, ..self }).shown as f32;
                let from = placed.shown as f32;
                let centre = anchored(placed.centre, about, room.extent, from, to);
                room.place(&Zoomed { scale, centre }).centre
            }
        };
        Zoomed { scale, centre }
    }

    /// The zoom after the keys moved the view a `fraction` of the part shown in `direction`
    /// in `room` ([`panned`]), its centre kept inside the edges; the same where the photo
    /// does not overflow the room.
    pub fn stepped(self, direction: Direction, fraction: f32, room: &Room) -> Zoomed {
        let placed = room.place(&self);
        if !overflows(&placed) {
            return self;
        }
        self.centred(panned(&placed, direction, fraction), room)
    }

    /// The zoom moved just enough in `room` to show `mark`, in the photo's coordinates
    /// ([`revealed`]), its centre kept inside the edges.
    pub fn revealing(self, mark: egui::Rect, room: &Room) -> Zoomed {
        self.centred(revealed(&room.place(&self), mark), room)
    }

    /// The zoom with its centre kept inside the edges in `room`, as the session holds it once
    /// it places the view for that photo (decision 99).
    pub fn clamped(self, room: &Room) -> Zoomed {
        self.centred(self.centre, room)
    }

    /// The zoom with its centre at `centre`, kept inside the edges in `room` as the session
    /// keeps it.
    fn centred(self, centre: [f32; 2], room: &Room) -> Zoomed {
        Zoomed {
            centre: room.place(&Zoomed { centre, ..self }).centre,
            ..self
        }
    }

    /// The zoom after the photo moved by `delta` physical pixels in `room`, its centre kept
    /// inside the edges; the same where the photo does not overflow the room, as at Fit.
    pub fn panned(self, delta: egui::Vec2, room: &Room) -> Zoomed {
        let placed = room.place(&self);
        if !overflows(&placed) {
            return self;
        }
        let centre = moved(placed.centre, delta, room.extent, placed.shown as f32);
        self.centred(centre, room)
    }
}

/// A way the keys move a zoomed view (`docs/gui.md`, Panning).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// How far Alt with an arrow moves the view: a fifth of the part shown, darktable's
/// `move horizontal` and `move vertical` (`darkroom.c`, `_action_process_move`).
pub const PAN_STEP: f32 = 0.2;

/// The centre moved a `fraction` of the visible part in `direction` where `placement`
/// places the photo: the view, not the photo, moves that way. Not clamped:
/// [`Room::place`] keeps it inside the edges.
pub fn panned(placement: &Placement, direction: Direction, fraction: f32) -> [f32; 2] {
    let shown = |axis: usize| {
        let side = f64::from(placement.size[axis].max(1));
        (placement.visible[axis + 2] / side) as f32 * fraction
    };
    let [x, y] = placement.centre;
    match direction {
        Direction::Left => [x - shown(0), y],
        Direction::Right => [x + shown(0), y],
        Direction::Up => [x, y - shown(1)],
        Direction::Down => [x, y + shown(1)],
    }
}

/// The part of the photo `placement` shows on each axis, across then down, 0 to 1 across
/// the photo: the whole of it on an axis that fits.
pub fn visible(placement: &Placement) -> [egui::Rangef; 2] {
    [0, 1].map(|axis| {
        let side = f64::from(placement.size[axis].max(1));
        let start = placement.visible[axis];
        let end = start + placement.visible[axis + 2];
        egui::Rangef::new((start / side) as f32, (end / side) as f32)
    })
}

/// The centre that brings `mark`, in the photo's coordinates, inside the part of the photo
/// `placement` shows, moving it the least on each axis: a mark larger than the part shown
/// is centred on it. Not clamped: [`Room::place`] keeps it inside the edges.
pub fn revealed(placement: &Placement, mark: egui::Rect) -> [f32; 2] {
    let shown = visible(placement);
    let marks = [mark.x_range(), mark.y_range()];
    [0, 1].map(|axis| {
        let (part, mark) = (shown[axis], marks[axis]);
        let by = if mark.span() >= part.span() {
            mark.center() - part.center()
        } else if mark.min < part.min {
            mark.min - part.min
        } else if mark.max > part.max {
            mark.max - part.max
        } else {
            0.0
        };
        placement.centre[axis] + by
    })
}

/// The part of the photo `placement` shows on each axis where the photo overflows the room,
/// across then down, as the position marks span it: none on an axis that fits.
pub fn spans(placement: &Placement) -> [Option<egui::Rangef>; 2] {
    let parts = visible(placement);
    [0, 1].map(|axis| {
        let whole = placement.visible[axis + 2] >= f64::from(placement.size[axis]);
        (!whole).then_some(parts[axis])
    })
}

/// The id of the photo's focus stop while it is zoomed past Fit (`docs/gui.md`, The zoom
/// by keyboard and to screen readers), which keeps no keys of its own
/// ([`Focus::of`](crate::actions::Focus::of)).
pub fn photo_view_id() -> egui::Id {
    egui::Id::unique("photo-view")
}

/// When the position marks show (`docs/gui.md`, Where the view is): from a zoom or a move
/// of the view until [`Marks::LINGER`] after the last one.
#[derive(Clone, Copy, Debug, Default)]
pub struct Marks {
    /// The zoom at the last frame.
    seen: Option<Zoomed>,
    /// When they go, in egui's time, seconds.
    until: f64,
}

impl Marks {
    /// How long they stay after the view last changed, in seconds.
    pub const LINGER: f64 = 1.0;

    /// Takes the zoom of the frame at `now`, egui's time in seconds, and returns how long
    /// the marks still show, None once they have gone. A zoom or a move since the frame
    /// before shows them again for [`Self::LINGER`]; the first frame's view shows none.
    pub fn update(&mut self, zoom: Zoomed, now: f64) -> Option<f64> {
        if self.seen.is_some_and(|seen| seen != zoom) {
            self.until = now + Self::LINGER;
        }
        self.seen = Some(zoom);
        (now < self.until).then_some(self.until - now)
    }
}

/// What the zoom's retained centre was last confirmed in (`docs/gui.md`, The zoom's
/// centre): the photo last seen live, and the orientation the session last reported for
/// it, its frame's.
#[derive(Clone, Copy, Debug, Default)]
pub struct Confirmed {
    /// The open that asked for the photo last seen live, and that orientation.
    live: Option<(u64, Orientation)>,
}

impl Confirmed {
    /// The retained `zoom` once the photo the open numbered `open` is live in `room`, the
    /// room the session places it in, its extent and the size last sent, the session
    /// reporting it in `orientation`. As that photo first goes live its centre is
    /// committed kept inside the photo's edges, so the photo after carries the centre it
    /// showed. Then a turn or a flip reported maps the centre through the turns not yet
    /// applied ([`reoriented`]), so a pan between two reports stays on its point, and
    /// commits the clamp in the turned room, the centre the session renders, which the
    /// photo after then carries on both sides; a size sent (`resized`) commits the clamp in
    /// it; else the centre stays as it was.
    pub fn follow(
        &mut self,
        open: u64,
        orientation: Orientation,
        zoom: Zoomed,
        room: &Room,
        resized: bool,
    ) -> Zoomed {
        let Some((seen, from)) = self.live.replace((open, orientation)) else {
            return zoom.clamped(room);
        };
        if seen != open {
            return zoom.clamped(room);
        }
        if from != orientation {
            let centre = reoriented(zoom.centre, from, orientation);
            return Zoomed { centre, ..zoom }.clamped(room);
        }
        if resized { zoom.clamped(room) } else { zoom }
    }
}

/// The zoom menu's factors, after its modes.
const FACTORS: [f32; 6] = [0.5, 1.0, 2.0, 4.0, 8.0, 16.0];

/// The menu's entries in order of scale for this photo and room, where Fit is `fit` and
/// Fill is `fill`: Small, Fit, Fill and the factors, each with its scale, an entry within 1%
/// of the one before counting once, the one earlier in the menu kept. A factor below Fit
/// is left out, since it shows at Fit (decision 99) and a step to it would change nothing.
pub fn steps(fit: f32, fill: f32) -> Vec<(Scale, f32)> {
    let modes = [
        (Scale::Small, fit / 2.0),
        (Scale::Fit, fit),
        (Scale::Fill, fill),
    ];
    let factors = FACTORS.map(|factor| (Scale::Factor(factor), factor));
    let mut entries: Vec<(usize, Scale, f32)> = (modes.into_iter().chain(factors))
        .enumerate()
        .map(|(at, (scale, value))| (at, scale, value))
        .filter(|(_, scale, value)| !matches!(scale, Scale::Factor(_)) || *value >= fit)
        .collect();
    // Stable, so equal scales keep the menu's order.
    entries.sort_by(|a, b| a.2.total_cmp(&b.2));
    let mut kept: Vec<(usize, Scale, f32)> = Vec::new();
    for entry in entries {
        match kept.last_mut() {
            Some(last) if entry.2 <= last.2 * 1.01 => {
                if entry.0 < last.0 {
                    *last = entry;
                }
            }
            _ => kept.push(entry),
        }
    }
    kept.into_iter()
        .map(|(_, scale, value)| (scale, value))
        .collect()
}

/// The scale Zoom in (`up`) or Zoom out goes to from `shown`, if any: the first of the
/// menu's [`steps`] above it, or the last below it, by more than half a percent.
pub fn step(shown: f32, up: bool, fit: f32, fill: f32) -> Option<Scale> {
    let steps = steps(fit, fill);
    let found = if up {
        steps.iter().find(|(_, scale)| *scale > shown * 1.005)
    } else {
        steps.iter().rev().find(|(_, scale)| *scale < shown / 1.005)
    };
    found.map(|(scale, _)| *scale)
}

/// The centre after the photo moved by `delta` physical pixels, the photo `photo` pixels
/// at full resolution shown at `shown`: a move right or down brings the photo's left or top
/// into view. Not clamped: [`Room::place`] keeps it inside the edges.
pub fn moved(centre: [f32; 2], delta: egui::Vec2, photo: [u32; 2], shown: f32) -> [f32; 2] {
    let along = |at: f32, delta: f32, side: u32| at - delta / (side.max(1) as f32 * shown);
    [
        along(centre[0], delta.x, photo[0]),
        along(centre[1], delta.y, photo[1]),
    ]
}

/// How near a smooth zoom or a pinch comes to a snap point to rest on it: 4% of it.
pub const SNAP: f32 = 0.04;

/// How near two scales are to count as one: a scale on a snap point but for rounding is on
/// it.
const SAME: f32 = 1e-4;

/// The snap points where Fit is `fit`: Fit, 100% and 200% (`docs/gui.md`, Zooming).
fn snap_points(fit: f32) -> [f32; 3] {
    [fit, 1.0, 2.0]
}

/// One wheel notch from `scale`, up or down, where Fit is `fit`, as darktable's
/// (`_calculate_new_scroll_zoom_tscale`): by 1.1 under 200% and by 2 from 200%, stopping on
/// Fit, 100% or 200% where it would pass one, never below Fit nor above 1600%. Down from
/// Fit or below it, as from Small, the scale stays.
pub fn notch(scale: f32, up: bool, fit: f32) -> f32 {
    let points = snap_points(fit);
    let near = |p: f32| (scale - p).abs() <= p * SAME;
    let scale = points.into_iter().find(|&p| near(p)).unwrap_or(scale);
    if up {
        let next = if scale < 2.0 {
            scale * 1.1
        } else {
            scale * 2.0
        };
        let passed = points.into_iter().filter(|&p| p > scale && p <= next);
        passed.fold(next, f32::min).clamp(fit, MAX_FACTOR)
    } else if scale <= fit {
        scale
    } else {
        let next = if scale <= 2.0 {
            scale / 1.1
        } else {
            scale / 2.0
        };
        let passed = points.into_iter().filter(|&p| p < scale && p >= next);
        passed.fold(next, f32::max).clamp(fit, MAX_FACTOR)
    }
}

/// The scale a smooth zoom or a pinch shows at its unsnapped scale `raw`, where Fit is
/// `fit`: the snap point within [`SNAP`] of it, the nearest where two are, else `raw`;
/// never below Fit nor above 1600%.
pub fn snapped(raw: f32, fit: f32) -> f32 {
    let distance = |p: f32| (raw - p).abs();
    let near = (snap_points(fit).into_iter())
        .filter(|&p| distance(p) <= p * SNAP)
        .min_by(|&a, &b| distance(a).total_cmp(&distance(b)));
    near.unwrap_or(raw).clamp(fit, MAX_FACTOR)
}

/// The centre that keeps the screen point `d`, in physical pixels from the photo area's
/// centre, on the same point of the photo as the zoom goes from `from` to `to`, the photo
/// `photo` pixels at full resolution and the view centred on `centre`: darktable's anchor
/// (`dt_dev_zoom_move`). Not clamped: [`Room::place`] keeps it inside the edges.
pub fn anchored(centre: [f32; 2], d: egui::Vec2, photo: [u32; 2], from: f32, to: f32) -> [f32; 2] {
    let along = |at: f32, d: f32, side: u32| at + d / side.max(1) as f32 * (1.0 / from - 1.0 / to);
    [
        along(centre[0], d.x, photo[0]),
        along(centre[1], d.y, photo[1]),
    ]
}

/// Middle-click's next scale from the scale shown, darktable's cycle: under 100% to 100%,
/// from 100% to 200%, from 200% on to Fit.
pub fn middle_click(scale: f32) -> Scale {
    if scale < 1.0 - SAME {
        Scale::Factor(1.0)
    } else if scale < 2.0 - SAME {
        Scale::Factor(2.0)
    } else {
        Scale::Fit
    }
}

/// The scale a notch or a gesture that stops at `value` sets, where Fit is `fit`: Fit on
/// Fit, so it reads Fit and follows the room, else the factor, 1 reading "100%" and Z going
/// from it back to Fit.
pub fn scale_at(value: f32, fit: f32) -> Scale {
    if (value - fit).abs() <= fit * SAME {
        Scale::Fit
    } else {
        Scale::Factor(value)
    }
}

/// One smooth zoom's or pinch's unsnapped scale (`docs/gui.md`, Zooming): the gesture
/// zooms on from it, so it moves past a snap point it rested on, and it is dropped as the
/// gesture ends, after [`Gesture::IDLE`] with no zoom or at [`Gesture::end`], and where the
/// scale shown is no longer the one it showed, as after a resize changes Fit.
#[derive(Clone, Copy, Debug, Default)]
pub struct Gesture {
    raw: Option<f32>,
    last: Option<Instant>,
    /// The scale it showed last.
    shown: Option<f32>,
}

impl Gesture {
    /// How long a gesture goes without a zoom before it ends.
    pub const IDLE: Duration = Duration::from_millis(100);

    /// The scale shown after `factor` more of the gesture at `now`, where Fit is `fit`, a
    /// gesture starting from the scale `shown`: [`snapped`], within Fit and 1600%. One that
    /// starts below Fit, at Small, and zooms out leaves the scale where it is.
    pub fn zoom(&mut self, shown: f32, factor: f32, fit: f32, now: Instant) -> f32 {
        let idle = (self.last).is_none_or(|last| now.duration_since(last) > Self::IDLE);
        let moved = (self.shown).is_some_and(|showed| (showed - shown).abs() > showed * SAME);
        if idle || moved {
            self.raw = None;
        }
        self.last = Some(now);
        if self.raw.is_none() && shown < fit && factor < 1.0 {
            self.shown = Some(shown);
            return shown;
        }
        let raw = (self.raw.unwrap_or(shown) * factor).clamp(fit, MAX_FACTOR);
        self.raw = Some(raw);
        let showing = snapped(raw, fit);
        self.shown = Some(showing);
        showing
    }

    /// Ends the gesture: its unsnapped scale goes, and the scale shown stays.
    pub fn end(&mut self) {
        *self = Gesture::default();
    }
}

/// The move the viewer bar's zoom menu asks for with `action`, None for its other
/// actions. The 100% entry is Z's, `zoom-100`.
pub fn chosen(action: ViewerBarAction) -> Option<ZoomStep> {
    Some(match action {
        ViewerBarAction::Zoom(Zoom::Small) => ZoomStep::To(Scale::Small),
        ViewerBarAction::Zoom(Zoom::Fit) => ZoomStep::Fit,
        ViewerBarAction::Zoom(Zoom::Fill) => ZoomStep::To(Scale::Fill),
        ViewerBarAction::Zoom(Zoom::Percent(100.0)) => ZoomStep::Hundred,
        ViewerBarAction::Zoom(Zoom::Percent(p)) => ZoomStep::To(Scale::Factor((p / 100.0) as f32)),
        ViewerBarAction::ZoomIn => ZoomStep::In,
        ViewerBarAction::ZoomOut => ZoomStep::Out,
        _ => return None,
    })
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
        Orientation {
            flip_x: bits & orientation::FLIP_X != 0,
            flip_y: bits & orientation::FLIP_Y != 0,
            swap_xy: bits & orientation::SWAP_XY != 0,
        }
    }

    /// The orientation the live frame shows the photo of `image` in, from its `values`:
    /// Orientation's, the photo's own at auto and while the History tab's eye hides the
    /// tool, which then renders as if reset (decision 78).
    pub fn shown(values: &Values, image: &ImageContext) -> Orientation {
        let tool = (values.tools.iter())
            .find(|state| state.tool == orientation::FLIP)
            .filter(|state| !state.hidden);
        let control = tool.and_then(|state| {
            (state.controls.iter()).find(|control| control.control == orientation::ORIENTATION)
        });
        let choice = match control.map(|control| &control.value) {
            Some(ControlValue::Choice(choice)) => *choice,
            _ => 0,
        };
        Orientation::from_bits(orientation::shown_bits(choice, image))
    }

    /// Where this orientation shows the point `at` of the unturned image, 0 to 1 across and
    /// down.
    fn forward(self, [x, y]: [f32; 2]) -> [f32; 2] {
        let x = if self.flip_x { 1.0 - x } else { x };
        let y = if self.flip_y { 1.0 - y } else { y };
        if self.swap_xy { [y, x] } else { [x, y] }
    }

    /// The point of the unturned image this orientation shows at `at`: [`Self::forward`]
    /// undone, the axes swapped back, then the flips.
    fn backward(self, [x, y]: [f32; 2]) -> [f32; 2] {
        let [x, y] = if self.swap_xy { [y, x] } else { [x, y] };
        let x = if self.flip_x { 1.0 - x } else { x };
        let y = if self.flip_y { 1.0 - y } else { y };
        [x, y]
    }
}

/// The centre `c`, in the photo's coordinates as orientation `from` shows it, mapped to
/// orientation `to`, so it stays on the same point of the scene: back to the unturned
/// image through `from`, then forward through `to`. Not clamped: [`Room::place`] keeps it
/// inside the edges.
pub fn reoriented(c: [f32; 2], from: Orientation, to: Orientation) -> [f32; 2] {
    to.forward(from.backward(c))
}

#[cfg(test)]
mod tests {
    use super::*;
    use apertrust_ui::components::percent_label;

    fn at(scale: Scale) -> Zoomed {
        Zoomed {
            scale,
            ..Zoomed::FIT
        }
    }

    /// A photo of 6000 × 4000 in a room of 4800 × 2700, as `docs/gui.md` has it.
    const ROOM: Room = Room {
        extent: [6000, 4000],
        bounds: (4800, 2700),
    };

    #[test]
    fn small_is_half_of_fit() {
        // A landscape photo and a portrait one, each in a landscape room.
        for (extent, fit) in [([6000, 4000], 0.4), ([4000, 6000], 1600.0 / 6000.0)] {
            let room = Room {
                extent,
                bounds: (2400, 1600),
            };
            let (scale, _) = room.modes();
            assert_eq!(scale, fit, "{extent:?}");
            let small = room.place(&at(Scale::Small)).shown;
            assert_eq!(small as f32, scale / 2.0, "{extent:?}");
        }
    }

    #[test]
    fn fit_never_enlarges_a_small_photo() {
        let room = Room {
            extent: [800, 600],
            bounds: (2400, 1600),
        };
        assert_eq!(room.modes().0, 1.0);
    }

    #[test]
    fn the_label_names_the_mode() {
        assert_eq!(at(Scale::Small).label(), Zoom::Small);
        assert_eq!(Zoomed::FIT.label(), Zoom::Fit);
        assert_eq!(at(Scale::Fill).label(), Zoom::Fill);
        assert_eq!(at(Scale::Factor(2.0)).label(), Zoom::Percent(200.0));
    }

    #[test]
    fn a_factor_below_fit_reads_fit() {
        // 50% kept while the room grew until Fit is 67.5%: shown at Fit, and said so.
        let kept = at(Scale::Factor(0.5));
        assert_eq!(kept.label_in(Some(&ROOM)), Zoom::Fit);
        assert_eq!(kept.label_in(None), Zoom::Percent(50.0));
        let wide = Room {
            extent: [6000, 4000],
            bounds: (2400, 1600),
        };
        assert_eq!(kept.label_in(Some(&wide)), Zoom::Percent(50.0));
    }

    #[test]
    fn the_label_reads_100_only_at_one_to_one() {
        assert_eq!(percent_label(99.6), 99);
        assert_eq!(percent_label(100.4), 101);
        assert_eq!(percent_label(100.0), 100);
        assert_eq!(percent_label(137.4), 137);
        assert_eq!(percent_label(49.5), 50);
    }

    #[test]
    fn steps_go_in_order_of_scale_not_of_the_list() {
        let (fit, fill) = ROOM.modes();
        assert_eq!((fit, fill), (0.675, 0.8));
        // 50% falls below Fit, which it would show at: no step.
        let scales: Vec<Scale> = steps(fit, fill).into_iter().map(|(s, _)| s).collect();
        assert_eq!(
            scales,
            [
                Scale::Small,
                Scale::Fit,
                Scale::Fill,
                Scale::Factor(1.0),
                Scale::Factor(2.0),
                Scale::Factor(4.0),
                Scale::Factor(8.0),
                Scale::Factor(16.0),
            ]
        );
        // Zoom in from Small: Fit, then Fill, then 100%; each step changes the scale shown.
        let mut zoom = at(Scale::Small);
        let mut shown = ROOM.place(&zoom).shown;
        for next in [Scale::Fit, Scale::Fill, Scale::Factor(1.0)] {
            zoom = zoom.step(ZoomStep::In, &ROOM).expect("a step up");
            assert_eq!(zoom.scale, next);
            let now = ROOM.place(&zoom).shown;
            assert!(now > shown, "{next:?}");
            shown = now;
        }
        // And back down the same way, to Small, below which there is none.
        for next in [Scale::Fill, Scale::Fit, Scale::Small] {
            zoom = zoom.step(ZoomStep::Out, &ROOM).expect("a step down");
            assert_eq!(zoom.scale, next);
        }
        assert_eq!(zoom.step(ZoomStep::Out, &ROOM), None);
        let most = at(Scale::Factor(16.0));
        assert_eq!(most.step(ZoomStep::In, &ROOM), None);
        // Where Fit is under 50%, 50% is a step of its own.
        let wide = Room {
            extent: [6000, 4000],
            bounds: (2400, 1600),
        };
        let (fit, fill) = wide.modes();
        assert_eq!(step(fit, true, fit, fill), Some(Scale::Factor(0.5)));
        // 50% chosen where it falls below Fit is Fit, which it shows.
        let chosen = at(Scale::Small).step(ZoomStep::To(Scale::Factor(0.5)), &ROOM);
        assert_eq!(chosen, Some(Zoomed::FIT));
    }

    #[test]
    fn a_step_starts_from_the_scale_shown() {
        // 50% kept while the room grew until Fit is 67.5%: shown at Fit, so Zoom in goes
        // on to Fill, never to Fit again.
        let kept = at(Scale::Factor(0.5));
        assert_eq!(ROOM.place(&kept).shown, 0.675);
        let up = kept.step(ZoomStep::In, &ROOM).map(|z| z.scale);
        assert_eq!(up, Some(Scale::Fill));
        let down = kept.step(ZoomStep::Out, &ROOM).map(|z| z.scale);
        assert_eq!(down, Some(Scale::Small));
    }

    #[test]
    fn entries_within_one_percent_count_once() {
        // Fit at 50.2% and 50%: one entry, Fit, the earlier in the menu; Fill at 100.5% and
        // 100%: Fill.
        let scales: Vec<Scale> = steps(0.502, 1.005).into_iter().map(|(s, _)| s).collect();
        assert_eq!(
            scales,
            [
                Scale::Small,
                Scale::Fit,
                Scale::Fill,
                Scale::Factor(2.0),
                Scale::Factor(4.0),
                Scale::Factor(8.0),
                Scale::Factor(16.0),
            ]
        );
        // Fill at 100.5% is within half a percent of 100%, so no step from there.
        assert_eq!(step(1.0, true, 0.502, 1.005), Some(Scale::Factor(2.0)));
        assert_eq!(step(1.0, false, 0.502, 1.005), Some(Scale::Fit));
    }

    #[test]
    fn fill_stops_at_1600() {
        let room = Room {
            extent: [100, 100],
            bounds: (2000, 1000),
        };
        let (fit, fill) = room.modes();
        assert_eq!((fit, fill), (1.0, 16.0));
        // Fill and 1600% count once, Fill kept, with nothing above it.
        assert_eq!(steps(fit, fill).last(), Some(&(Scale::Fill, 16.0)));
        assert_eq!(at(Scale::Fill).step(ZoomStep::In, &room), None);
        // The photo centred across, the surround either side, its label still Fill.
        let placed = room.place(&at(Scale::Fill));
        assert_eq!(placed.shown, 16.0);
        assert_eq!(placed.visible[2], f64::from(placed.size[0]));
        assert_eq!(placed.centre[0], 0.5);
        assert_eq!(at(Scale::Fill).label(), Zoom::Fill);
    }

    #[test]
    fn z_toggles_100_and_fit() {
        let off_centre = Zoomed {
            scale: Scale::Fill,
            centre: [0.2, 0.5],
        };
        // 100% at the view's centre, as the room clamps it.
        let hundred = off_centre.step(ZoomStep::Hundred, &ROOM).expect("100%");
        let centre = ROOM.place(&off_centre).centre;
        let expected = Zoomed {
            scale: Scale::Factor(1.0),
            centre,
        };
        assert_eq!(hundred, expected);
        // At 100%, back to Fit, centred.
        assert_eq!(hundred.step(ZoomStep::Hundred, &ROOM), Some(Zoomed::FIT));
        // Even where Fit shows a small photo pixel for pixel, Z chooses 100%.
        let small = Room {
            extent: [800, 600],
            bounds: (2400, 1600),
        };
        let chosen = Zoomed::FIT.step(ZoomStep::Hundred, &small).expect("100%");
        assert_eq!(chosen.scale, Scale::Factor(1.0));
        assert_eq!(chosen.label(), Zoom::Percent(100.0));
    }

    #[test]
    fn fit_and_small_centre_the_photo_and_a_scale_keeps_the_centre() {
        let moved = Zoomed {
            scale: Scale::Factor(2.0),
            centre: [0.3, 0.6],
        };
        assert_eq!(moved.step(ZoomStep::Fit, &ROOM), Some(Zoomed::FIT));
        let small = moved.step(ZoomStep::To(Scale::Small), &ROOM);
        assert_eq!(small, Some(at(Scale::Small)));
        let four = moved.step(ZoomStep::To(Scale::Factor(4.0)), &ROOM);
        assert_eq!(four.map(|z| z.centre), Some([0.3, 0.6]));
    }

    #[test]
    fn a_move_at_fit_does_nothing() {
        let delta = egui::vec2(120.0, -80.0);
        assert_eq!(Zoomed::FIT.panned(delta, &ROOM), Zoomed::FIT);
        assert_eq!(at(Scale::Small).panned(delta, &ROOM), at(Scale::Small));
        // At 100% it moves against the delta, by the delta in photo pixels.
        let hundred = at(Scale::Factor(1.0)).panned(delta, &ROOM);
        let close = |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).abs() + (a[1] - b[1]).abs() < 1e-6;
        let expected = [0.5 - 120.0 / 6000.0, 0.5 + 80.0 / 4000.0];
        assert!(close(hundred.centre, expected), "{:?}", hundred.centre);
        // At 200% by half as much of the photo, and never past an edge.
        let two = at(Scale::Factor(2.0)).panned(delta, &ROOM);
        let expected = [0.5 - 60.0 / 6000.0, 0.5 + 40.0 / 4000.0];
        assert!(close(two.centre, expected), "{:?}", two.centre);
        let far = at(Scale::Factor(1.0)).panned(egui::vec2(1e6, 0.0), &ROOM);
        assert!((far.centre[0] - 2400.0 / 6000.0).abs() < 1e-6, "{far:?}");
    }

    #[test]
    fn a_key_moves_the_view_a_share_of_the_part_shown() {
        // At 100% the room shows 4800 of 6000 across and 2700 of 4000 down.
        let hundred = at(Scale::Factor(1.0));
        let placed = ROOM.place(&hundred);
        let [x, y] = panned(&placed, Direction::Left, PAN_STEP);
        assert!((x - (0.5 - 0.2 * 0.8)).abs() < 1e-6 && y == 0.5, "{x} {y}");
        let [x, y] = panned(&placed, Direction::Down, 1.0);
        assert!(x == 0.5 && (y - (0.5 + 0.675)).abs() < 1e-6, "{x} {y}");
        // Kept inside the edges, and at Fit nothing moves.
        let right = hundred.stepped(Direction::Right, 1.0, &ROOM);
        assert!((right.centre[0] - 0.6).abs() < 1e-6, "{right:?}");
        assert_eq!(Zoomed::FIT.stepped(Direction::Up, 1.0, &ROOM), Zoomed::FIT);
    }

    #[test]
    fn a_mark_out_of_view_moves_the_view_just_enough() {
        // At 200% the room shows 0.4 of the width and 0.3375 of the height about the centre.
        let two = at(Scale::Factor(2.0));
        let placed = ROOM.place(&two);
        let inside = egui::Rect::from_min_max(egui::pos2(0.4, 0.4), egui::pos2(0.6, 0.6));
        assert_eq!(revealed(&placed, inside), [0.5, 0.5]);
        // Past the right edge by 0.05, and above the top by 0.01.
        let mark = egui::Rect::from_min_max(egui::pos2(0.6, 0.32), egui::pos2(0.75, 0.4));
        let [x, y] = revealed(&placed, mark);
        let top = 0.5 - 0.3375 / 2.0;
        assert!(
            (x - 0.55).abs() < 1e-5 && (y - (0.5 - (top - 0.32))).abs() < 1e-5,
            "{x} {y}"
        );
        // Larger than the part shown: centred on it.
        let wide = egui::Rect::from_min_max(egui::pos2(0.1, 0.45), egui::pos2(0.7, 0.55));
        assert!((revealed(&placed, wide)[0] - 0.4).abs() < 1e-5);
        // And kept inside the photo's edges.
        let corner = egui::Rect::from_min_max(egui::pos2(0.97, 0.97), egui::pos2(1.02, 1.02));
        let moved = two.revealing(corner, &ROOM);
        let [x, y] = moved.centre;
        assert!(
            (x - 0.8).abs() < 1e-5 && (y - (1.0 - 0.3375 / 2.0)).abs() < 1e-5,
            "{x} {y}"
        );
    }

    #[test]
    fn the_zoom_menu_s_entries_are_steps() {
        let step = |zoom| chosen(ViewerBarAction::Zoom(zoom));
        assert_eq!(step(Zoom::Small), Some(ZoomStep::To(Scale::Small)));
        assert_eq!(step(Zoom::Fit), Some(ZoomStep::Fit));
        assert_eq!(step(Zoom::Fill), Some(ZoomStep::To(Scale::Fill)));
        assert_eq!(step(Zoom::Percent(100.0)), Some(ZoomStep::Hundred));
        assert_eq!(
            step(Zoom::Percent(400.0)),
            Some(ZoomStep::To(Scale::Factor(4.0)))
        );
        assert_eq!(chosen(ViewerBarAction::ZoomIn), Some(ZoomStep::In));
        assert_eq!(chosen(ViewerBarAction::Guides), None);
    }

    #[test]
    fn the_view_names_the_scale_and_the_centre() {
        let zoom = Zoomed {
            scale: Scale::Factor(4.0),
            centre: [0.25, 0.75],
        };
        let view = zoom.view();
        assert_eq!(view.scale, ViewScale::Factor(4.0));
        assert_eq!(view.centre, [0.25, 0.75]);
        assert_eq!(at(Scale::Small).view().scale, ViewScale::Small);
        assert_eq!(Zoomed::FIT.view(), View::default());
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn a_notch_steps_1_1_under_200_and_2_from_200() {
        let fit = 0.2;
        for (from, up, to) in [
            (0.5, true, 0.55),
            (1.2, true, 1.32),
            (0.5, false, 0.5 / 1.1),
            (2.0, false, 2.0 / 1.1),
            (2.0, true, 4.0),
            (4.0, true, 8.0),
            (8.0, false, 4.0),
            (4.0, false, 2.0),
        ] {
            let stepped = notch(from, up, fit);
            assert!(close(stepped, to), "{from} {up}: {stepped}");
        }
        // At most 1600%, and down from Fit, or from Small below it, nowhere.
        assert_eq!(notch(16.0, true, fit), 16.0);
        assert_eq!(notch(12.0, true, fit), 16.0);
        assert_eq!(notch(fit, false, fit), fit);
        assert_eq!(notch(fit / 2.0, false, fit), fit / 2.0);
        // Up from Small, Fit: gestures never stop below it.
        assert_eq!(notch(fit / 2.0, true, fit), fit);
    }

    #[test]
    fn a_notch_never_passes_fit_100_or_200() {
        let fit = 0.4;
        assert_eq!(notch(0.95, true, fit), 1.0);
        assert_eq!(notch(1.9, true, fit), 2.0);
        assert_eq!(notch(3.0, false, fit), 2.0);
        assert_eq!(notch(1.05, false, fit), 1.0);
        assert_eq!(notch(fit * 1.05, false, fit), fit);
        // From a snap point but for rounding, on past it.
        assert!(close(notch(1.0 - 1e-6, true, fit), 1.1));
    }

    #[test]
    fn a_smooth_zoom_rests_on_a_snap_point_within_4_percent() {
        let fit = 0.4;
        assert_eq!(snapped(1.03, fit), 1.0);
        assert_eq!(snapped(0.97, fit), 1.0);
        assert_eq!(snapped(1.05, fit), 1.05);
        assert_eq!(snapped(2.07, fit), 2.0);
        assert_eq!(snapped(0.41, fit), fit);
        // A gesture moves on past a snap point it rests on: its own scale is unsnapped.
        let now = Instant::now();
        let mut gesture = Gesture::default();
        assert_eq!(gesture.zoom(0.98, 1.0, fit, now), 1.0);
        assert_eq!(gesture.zoom(1.0, 1.04, fit, now), 1.0);
        let on = gesture.zoom(1.0, 1.04, fit, now);
        assert!(close(on, 0.98 * 1.04 * 1.04), "{on}");
        // It ends after a tenth of a second with no zoom, and the next starts where the
        // scale shown is.
        let later = now + Gesture::IDLE + Duration::from_millis(1);
        assert!(close(gesture.zoom(1.5, 1.1, fit, later), 1.65));
        gesture.end();
        assert!(close(gesture.zoom(1.0, 1.1, fit, later), 1.1));
        // A resize that raises Fit above the gesture's scale starts it again from the scale
        // shown: 60% shown at Fit's 80% zooms on from 80%.
        let mut gesture = Gesture::default();
        assert!(close(gesture.zoom(0.6, 1.0, 0.4, now), 0.6));
        assert!(close(gesture.zoom(0.8, 1.05, 0.8, now), 0.84));
    }

    #[test]
    fn gestures_stop_at_fit_and_16() {
        let (fit, now) = (0.4, Instant::now());
        assert_eq!(snapped(0.1, fit), fit);
        assert_eq!(snapped(20.0, fit), 16.0);
        let mut gesture = Gesture::default();
        assert_eq!(gesture.zoom(1.0, 0.1, fit, now), fit);
        // Its own scale stops there too, so zooming back in starts from Fit.
        assert!(close(gesture.zoom(fit, 1.2, fit, now), fit * 1.2));
        assert_eq!(gesture.zoom(fit, 100.0, fit, now), 16.0);
        assert!(close(gesture.zoom(16.0, 0.5, fit, now), 8.0));
        // From Small, out leaves it; in goes to Fit at least.
        let mut gesture = Gesture::default();
        assert_eq!(gesture.zoom(fit / 2.0, 0.9, fit, now), fit / 2.0);
        assert_eq!(gesture.zoom(fit / 2.0, 1.1, fit, now), fit);
    }

    #[test]
    fn the_point_under_the_pointer_stays_still() {
        // A small linear congruential generator: the same values every run.
        let mut seed = 0x2545_f491_u32;
        let mut random = move || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1u32 << 24) as f32
        };
        let (fit, _) = ROOM.modes();
        let mut kept = 0;
        for _ in 0..1000 {
            let from = fit + random() * (8.0 - fit);
            let to = fit + random() * (16.0 - fit);
            let centre = [random(), random()];
            let zoom = Zoomed {
                scale: Scale::Factor(from),
                centre,
            };
            let before = ROOM.place(&zoom);
            // A point of the photo area: the room, or the photo where it is smaller.
            let area = [0, 1].map(|axis| {
                let side = [ROOM.bounds.0, ROOM.bounds.1][axis] as f32;
                side.min(ROOM.extent[axis] as f32 * from)
            });
            let d = egui::vec2((random() - 0.5) * area[0], (random() - 0.5) * area[1]);
            let photo = ROOM.extent.map(|side| side as f32);
            let point = [0, 1].map(|axis| before.centre[axis] + d[axis] / (photo[axis] * from));
            let zoomed = zoom.about(Scale::Factor(to), d, &ROOM);
            let unclamped = anchored(before.centre, d, ROOM.extent, from, to);
            let after = ROOM.place(&zoomed);
            assert_eq!(after.centre, zoomed.centre);
            for axis in [0, 1] {
                if (after.centre[axis] - unclamped[axis]).abs() > 1e-6 {
                    continue;
                }
                let on_screen = (point[axis] - after.centre[axis]) * photo[axis] * to;
                let moved = (on_screen - d[axis]).abs();
                assert!(
                    moved < 1.0,
                    "{from} → {to} at {d:?}: {moved} on axis {axis}"
                );
                kept += 1;
            }
        }
        // Most anchors are not clamped.
        assert!(kept > 1000, "{kept}");
    }

    #[test]
    fn middle_click_cycles_100_200_fit() {
        assert_eq!(middle_click(0.5), Scale::Factor(1.0));
        assert_eq!(middle_click(1.0), Scale::Factor(2.0));
        assert_eq!(middle_click(1.5), Scale::Factor(2.0));
        assert_eq!(middle_click(2.0), Scale::Fit);
        assert_eq!(middle_click(3.0), Scale::Fit);
    }

    #[test]
    fn a_zoom_that_stops_on_fit_is_fit_and_on_1_is_100() {
        assert_eq!(scale_at(0.4, 0.4), Scale::Fit);
        assert_eq!(scale_at(1.0, 0.4), Scale::Factor(1.0));
        assert_eq!(
            Zoomed::FIT
                .about(Scale::Factor(1.0), egui::Vec2::ZERO, &ROOM)
                .label(),
            Zoom::Percent(100.0)
        );
    }

    #[test]
    fn z_at_a_point_keeps_it_still() {
        // At Fit, 40% of the room's 2400 × 1600, the pointer 600 pixels right of the
        // middle: the photo's point there, 0.5 + 600 / (6000 × 0.4) across, stays under it
        // at 100%.
        let room = Room {
            extent: [6000, 4000],
            bounds: (2400, 1600),
        };
        let d = egui::vec2(600.0, 0.0);
        let hundred = Zoomed::FIT
            .step_about(ZoomStep::Hundred, d, &room)
            .expect("100%");
        let point = 0.5 + 600.0 / (6000.0 * 0.4);
        assert!(
            close(hundred.centre[0] + 600.0 / 6000.0, point),
            "{hundred:?}"
        );
        assert_eq!(hundred.centre[1], 0.5);
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
        close(a[0], b[0]) && close(a[1], b[1])
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

    /// The retained centre follows what the session confirms: a photo going live commits
    /// its clamp; a turn reported maps it through the turns not yet applied and commits its
    /// clamp in the turned room, so two turns in flight end on the scene's point; a size
    /// sent commits its clamp; the first sight of another photo commits its own clamp, never
    /// mapping.
    #[test]
    fn the_confirmed_state_moves_the_retained_centre() {
        let (none, cw) = (Orientation::default(), Orientation::from_bits(turn_bits(0)));
        let half = Orientation::from_bits(turn_bits(turn_bits(0)));
        let landscape = Room {
            extent: [6000, 4000],
            bounds: (2400, 1600),
        };
        let portrait = Room {
            extent: [4000, 6000],
            ..landscape
        };
        let at = |centre| Zoomed {
            scale: Scale::Factor(1.0),
            centre,
        };
        let mut confirmed = Confirmed::default();
        // Live: the carried 0.9 commits to 0.8, as the landscape shows 0.2 to 0.8.
        let live = confirmed.follow(2, none, at([0.9, 0.5]), &landscape, false);
        assert!(close_at(live.centre, [0.8, 0.5]), "{live:?}");
        // A quarter turn clockwise: (1 − 0.5, 0.8), inside the portrait's edges.
        let turned = confirmed.follow(2, cw, live, &portrait, false);
        assert!(close_at(turned.centre, [0.5, 0.8]), "{turned:?}");
        // The same report again moves nothing; a pan in the portrait, 0.15 to the left, is
        // the person's, then the second turn maps it on: the scene's (0.8, 0.65), at (0.2,
        // 0.35) half round.
        assert_eq!(confirmed.follow(2, cw, turned, &portrait, false), turned);
        let panned = at([0.35, 0.8]);
        let round = confirmed.follow(2, half, panned, &landscape, false);
        assert!(close_at(round.centre, [0.2, 0.35]), "{round:?}");
        // With no pan, the two turns end on 0.2, the portrait between holding (0.5, 0.8).
        let mut confirmed = Confirmed::default();
        let live = confirmed.follow(3, none, at([0.8, 0.5]), &landscape, false);
        let between = confirmed.follow(3, cw, live, &portrait, false);
        let round = confirmed.follow(3, half, between, &landscape, false);
        assert!(close_at(round.centre, [0.2, 0.5]), "{round:?}");
        // A turn whose centre the turned room cannot show commits the one it shows: (0.5,
        // 0.2) turns to (0.8, 0.5), shown at 0.7 across, as the portrait shows 0.3 to 0.7.
        let mut clamping = Confirmed::default();
        let live = clamping.follow(4, none, at([0.5, 0.2]), &landscape, false);
        let turned = clamping.follow(4, cw, live, &portrait, false);
        assert!(close_at(turned.centre, [0.7, 0.5]), "{turned:?}");
        // A size sent commits its clamp: 3600 shows 0.3 to 0.7.
        let wide = Room {
            bounds: (3600, 1600),
            ..landscape
        };
        let sent = confirmed.follow(3, half, at([0.2, 0.5]), &wide, true);
        assert!(close_at(sent.centre, [0.3, 0.5]), "{sent:?}");
        // Another photo, live in another orientation: its clamp, no turn.
        let other = confirmed.follow(5, cw, at([0.9, 0.5]), &portrait, false);
        assert!(close_at(other.centre, [0.7, 0.5]), "{other:?}");
    }

    /// darktable's clockwise quarter turn from the orientation of `bits`.
    fn turn_bits(bits: u8) -> u8 {
        apertrust_tools::orientation::turn("rotate_cw", bits).expect("a button")
    }
}
