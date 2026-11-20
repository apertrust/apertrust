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
//! factor keeps its scale. Small is the fitted frame drawn at half (decision 99), so it
//! and Fit are what the Viewer shows today; Fill and the factors come with the engine's
//! detail view, the zoom's second step.

use apertrust_ui::components::{ViewerBarAction, Zoom};

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
    /// The next scale up, and down, in order of scale.
    In,
    Out,
    Fit,
    /// 100%, and from 100% back to Fit, as Z does.
    Hundred,
    /// That scale.
    To(Scale),
}

impl Zoomed {
    /// The zoom a photo opens at: the whole photo, centred.
    pub const FIT: Zoomed = Zoomed {
        scale: Scale::Fit,
        centre: [0.5, 0.5],
    };

    /// The scale shown, physical pixels per photo pixel, where Fit is `fit` and Fill is
    /// `fill`.
    #[cfg_attr(
        not(test),
        allow(dead_code, reason = "the percentages and panning read it (#357)")
    )]
    pub fn shown(&self, fit: f32, fill: f32) -> f32 {
        match self.scale {
            Scale::Small => fit / 2.0,
            Scale::Fit => fit,
            Scale::Fill => fill,
            Scale::Factor(factor) => factor,
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

    /// The zoom after `step`, or None where it has nowhere to go: Zoom in at Fit and Zoom
    /// out at Small, between the two scales the fitted frame shows. The others, 100%
    /// among them, wait for the detail view.
    pub fn step(self, step: ZoomStep) -> Option<Zoomed> {
        let scale = match (step, self.scale) {
            (ZoomStep::In, Scale::Small) => Scale::Fit,
            (ZoomStep::Out, Scale::Fit) => Scale::Small,
            (ZoomStep::Fit, _) => Scale::Fit,
            (ZoomStep::To(to @ (Scale::Small | Scale::Fit)), _) => to,
            _ => return None,
        };
        // Both show the whole photo, centred.
        Some(Zoomed {
            scale,
            ..Zoomed::FIT
        })
    }

    /// Whether the scale shown is Small, which draws the fitted frame at half.
    pub fn small(&self) -> bool {
        self.scale == Scale::Small
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

/// Fit's scale for a photo of `photo` pixels in a room of `room` physical pixels, at most 1:
/// the engine never renders larger than the photo.
#[cfg_attr(
    not(test),
    allow(dead_code, reason = "the percentages and panning read it (#357)")
)]
pub fn fit_scale(photo: (u32, u32), room: (u32, u32)) -> f32 {
    if photo.0 == 0 || photo.1 == 0 {
        return 1.0;
    }
    let across = room.0 as f32 / photo.0 as f32;
    let down = room.1 as f32 / photo.1 as f32;
    across.min(down).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(scale: Scale) -> Zoomed {
        Zoomed {
            scale,
            ..Zoomed::FIT
        }
    }

    #[test]
    fn small_is_half_of_fit() {
        // A landscape photo and a portrait one, each in a landscape room.
        for (photo, fit) in [((6000, 4000), 0.4), ((4000, 6000), 1600.0 / 6000.0)] {
            let scale = fit_scale(photo, (2400, 1600));
            assert_eq!(scale, fit, "{photo:?}");
            assert_eq!(at(Scale::Small).shown(scale, 0.5), scale / 2.0, "{photo:?}");
            assert_eq!(at(Scale::Fit).shown(scale, 0.5), scale, "{photo:?}");
        }
    }

    #[test]
    fn fit_never_enlarges_a_small_photo() {
        assert_eq!(fit_scale((800, 600), (2400, 1600)), 1.0);
    }

    #[test]
    fn the_label_names_the_mode() {
        assert_eq!(at(Scale::Small).label(), Zoom::Small);
        assert_eq!(Zoomed::FIT.label(), Zoom::Fit);
        assert_eq!(at(Scale::Factor(2.0)).label(), Zoom::Percent(200.0));
    }

    #[test]
    fn zoom_in_and_out_step_between_small_and_fit() {
        let small = at(Scale::Small);
        assert_eq!(Zoomed::FIT.step(ZoomStep::Out), Some(small));
        assert_eq!(small.step(ZoomStep::In), Some(Zoomed::FIT));
        // Nowhere to go: the other scales wait for the detail view.
        assert_eq!(Zoomed::FIT.step(ZoomStep::In), None);
        assert_eq!(small.step(ZoomStep::Out), None);
        assert_eq!(small.step(ZoomStep::Hundred), None);
        assert_eq!(small.step(ZoomStep::To(Scale::Fill)), None);
        // Fit and Small from either, Fit again at Fit.
        assert_eq!(small.step(ZoomStep::Fit), Some(Zoomed::FIT));
        assert_eq!(Zoomed::FIT.step(ZoomStep::Fit), Some(Zoomed::FIT));
        assert_eq!(Zoomed::FIT.step(ZoomStep::To(Scale::Small)), Some(small));
    }

    #[test]
    fn the_zoom_menu_s_entries_are_steps() {
        let step = |zoom| chosen(ViewerBarAction::Zoom(zoom));
        assert_eq!(step(Zoom::Small), Some(ZoomStep::To(Scale::Small)));
        assert_eq!(step(Zoom::Fit), Some(ZoomStep::Fit));
        assert_eq!(step(Zoom::Percent(100.0)), Some(ZoomStep::Hundred));
        assert_eq!(
            step(Zoom::Percent(400.0)),
            Some(ZoomStep::To(Scale::Factor(4.0)))
        );
        assert_eq!(chosen(ViewerBarAction::ZoomIn), Some(ZoomStep::In));
        assert_eq!(chosen(ViewerBarAction::Guides), None);
    }
}
