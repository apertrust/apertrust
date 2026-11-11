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

//! The fade from an opening photo's first picture to its first live frame (decision 77;
//! `docs/gui.md`, The central panel's states).
//!
//! The live frame is drawn at full strength from its first frame, with the scopes, the
//! values and the tools on at once; the picture is drawn over it, in the place it was drawn
//! in, its alpha falling from 1 to 0 over [`FADE`], eased out. Only that swap fades
//! ([`Reaction::Swapped`](crate::mirror::Reaction::Swapped)): an edit, a resize, a
//! re-render, a failed render and one photo replaced by another are cuts. The picture's own
//! texture is kept until the fade ends, then dropped, and the app repaints only while it
//! fades: one textured quad for about seven frames. With the system's reduce-motion
//! preference on ([`apertrust_display::reduce_motion`]), or the theme's motion tokens at
//! zero, for which the spinner holds still too (egui's `animation_time`), the swap is
//! instant.

use std::time::Duration;

use apertrust_ui::components::Placed;
use eframe::egui;

/// How long the picture takes to fade out over the live frame.
pub const FADE: Duration = Duration::from_millis(120);

/// A picture fading out over the live frame of the photo the open numbered `open`.
pub struct Fade {
    open: u64,
    texture: egui::TextureHandle,
    /// When it began, in egui's time ([`egui::InputState::time`]), seconds.
    from: f64,
}

impl Fade {
    /// The fade of the picture `kept` holds where the swap of `open` replaced the picture of
    /// `generation`, in `ctx`, from its time now: none with reduce motion on, the theme's
    /// motion at zero, or where that picture was never drawn, as when the live frame came
    /// within a frame of the click.
    pub fn begin(
        ctx: &egui::Context,
        kept: &mut Option<(u64, egui::TextureHandle)>,
        open: u64,
        generation: u64,
        reduce_motion: bool,
    ) -> Option<Fade> {
        let (made, texture) = kept.take()?;
        let still = reduce_motion || ctx.global_style().animation_time <= 0.0;
        let now = ctx.input(|input| input.time);
        (made == generation && !still).then_some(Fade {
            open,
            texture,
            from: now,
        })
    }

    /// The picture's alpha `elapsed` seconds into the fade, eased out: `None` once it ends.
    pub fn alpha(elapsed: f64) -> Option<f32> {
        let t = elapsed / FADE.as_secs_f64();
        (t < 1.0).then(|| (1.0 - t.max(0.0)).powi(2) as f32)
    }

    /// Draws the picture over the live frame placed in `placed`, while the open numbered
    /// `open` is the one shown, and asks for the next frame; ends the fade, dropping its
    /// texture, once it is over by egui's clock or another photo is asked for.
    pub fn paint(fade: &mut Option<Fade>, painter: &egui::Painter, placed: &Placed, open: u64) {
        let Some(shown) = fade.as_ref() else {
            return;
        };
        let now = painter.ctx().input(|input| input.time);
        let alpha = (shown.open == open)
            .then(|| Self::alpha(now - shown.from))
            .flatten();
        let Some(alpha) = alpha else {
            *fade = None;
            return;
        };
        let [width, height] = shown.texture.size();
        let ppp = painter.pixels_per_point();
        let rect = crate::picture_rect(placed, (width as u32, height as u32), ppp);
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        let tint = egui::Color32::WHITE.gamma_multiply(alpha);
        painter.image(shown.texture.id(), rect, uv, tint);
        painter.ctx().request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;

    use super::*;
    use apertrust_ui::components::{Margin, PhotoFrame};

    #[test]
    fn the_picture_s_alpha_falls_from_one_to_none_over_the_fade_eased_out() {
        assert_eq!(Fade::alpha(0.0), Some(1.0));
        let half = Fade::alpha(FADE.as_secs_f64() / 2.0).unwrap();
        // Eased out: past halfway down at half time.
        assert!(half < 0.5 && half > 0.0, "{half}");
        let mut last = 1.0;
        for ms in (0..120).step_by(10) {
            let alpha = Fade::alpha(f64::from(ms) / 1e3).unwrap();
            assert!(alpha <= last, "{ms} ms");
            last = alpha;
        }
        assert_eq!(Fade::alpha(FADE.as_secs_f64()), None);
    }

    fn texture(ctx: &egui::Context) -> egui::TextureHandle {
        let image = egui::ColorImage::filled([4, 3], egui::Color32::GRAY);
        ctx.load_texture("opening picture", image, egui::TextureOptions::LINEAR)
    }

    #[test]
    fn a_fade_takes_the_drawn_picture_unless_motion_is_off_or_another_was_drawn() {
        let ctx = egui::Context::default();
        ctx.global_style_mut(|style| style.animation_time = 0.1);
        let mut kept = Some((3, texture(&ctx)));
        assert!(Fade::begin(&ctx, &mut kept, 2, 3, true).is_none());
        assert!(
            kept.is_none(),
            "with reduce motion on, the texture goes at once"
        );
        let mut kept = Some((3, texture(&ctx)));
        assert!(Fade::begin(&ctx, &mut kept, 2, 4, false).is_none());
        assert!(kept.is_none(), "another picture's texture is never faded");
        assert!(Fade::begin(&ctx, &mut None, 2, 3, false).is_none());
        let mut kept = Some((3, texture(&ctx)));
        assert!(Fade::begin(&ctx, &mut kept, 2, 3, false).is_some());
        assert!(kept.is_none());
        // The theme's motion at zero: a cut, as the spinner holds still.
        ctx.global_style_mut(|style| style.animation_time = 0.0);
        let mut kept = Some((3, texture(&ctx)));
        assert!(Fade::begin(&ctx, &mut kept, 2, 3, false).is_none());
        assert!(kept.is_none());
    }

    /// A slot with a fade under way, as the central panel draws it over the live frame.
    struct Scene {
        fade: Option<Fade>,
        open: u64,
    }

    fn scene(step: f32) -> Harness<'static, Scene> {
        Harness::builder()
            .with_size(egui::vec2(400.0, 300.0))
            .with_step_dt(step)
            .build_ui_state(
                |ui, scene: &mut Scene| {
                    let slot = ui.max_rect();
                    let block = PhotoFrame::new(slot, Margin::Points(24.0));
                    let placed = block.place(egui::vec2(300.0, 200.0), ui.pixels_per_point());
                    if scene.fade.is_none() {
                        return;
                    }
                    Fade::paint(&mut scene.fade, &ui.painter_at(slot), &placed, scene.open);
                },
                Scene {
                    fade: None,
                    open: 2,
                },
            )
    }

    /// The picture fades over 120 ms by egui's clock, asking for frames only meanwhile,
    /// and its texture is dropped as it ends.
    #[test]
    fn a_fade_ends_after_its_time_and_drops_its_texture() {
        let mut harness = scene(0.04);
        let ctx = harness.ctx.clone();
        // The harness holds egui's animations still; the theme's motion would not.
        ctx.global_style_mut(|style| style.animation_time = 0.1);
        let mut kept = Some((3, texture(&ctx)));
        harness.state_mut().fade = Fade::begin(&ctx, &mut kept, 2, 3, false);
        let textures = || ctx.tex_manager().read().num_allocated();
        let with_fade = textures();
        let mut steps = 0;
        while harness.state().fade.is_some() {
            harness.step();
            steps += 1;
            assert!(steps <= 5, "a fade lasts 120 ms");
        }
        assert_eq!(steps, 4, "three frames of fading at 40 ms, then its end");
        assert!(textures() < with_fade, "its texture is dropped");
        // Nothing more is asked for: the app goes idle.
        harness.step();
        assert!(!ctx.has_requested_repaint());
    }

    /// Another photo asked for during the fade ends it at once.
    #[test]
    fn another_photo_ends_a_fade_at_once() {
        let mut harness = scene(0.01);
        let ctx = harness.ctx.clone();
        // The harness holds egui's animations still; the theme's motion would not.
        ctx.global_style_mut(|style| style.animation_time = 0.1);
        let mut kept = Some((3, texture(&ctx)));
        harness.state_mut().fade = Fade::begin(&ctx, &mut kept, 2, 3, false);
        harness.step();
        assert!(harness.state().fade.is_some());
        harness.state_mut().open = 4;
        harness.step();
        assert!(harness.state().fade.is_none());
    }
}
