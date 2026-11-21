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

use apertrust_engine::view::{self, Placement, View, ViewScale};
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

    /// The zoom after `step` in `room`, or None where it has nowhere to go: Zoom in at
    /// 1600% and Zoom out at Small. A scale keeps the view's centre, as the room clamps it;
    /// Fit and Small centre the photo. Z's step goes to 100%, and from 100% back to Fit.
    pub fn step(self, step: ZoomStep, room: &Room) -> Option<Zoomed> {
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
        let centre = match scale {
            Scale::Small | Scale::Fit => Zoomed::FIT.centre,
            Scale::Fill | Scale::Factor(_) => room.place(&self).centre,
        };
        Some(Zoomed { scale, centre })
    }

    /// The zoom after the photo moved by `delta` physical pixels in `room`, its centre kept
    /// inside the edges; the same where the photo does not overflow the room, as at Fit.
    pub fn panned(self, delta: egui::Vec2, room: &Room) -> Zoomed {
        let placed = room.place(&self);
        if !overflows(&placed) {
            return self;
        }
        let centre = moved(placed.centre, delta, room.extent, placed.shown as f32);
        let moved = Zoomed { centre, ..self };
        Zoomed {
            centre: room.place(&moved).centre,
            ..self
        }
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
}
