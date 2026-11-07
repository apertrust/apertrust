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

//! The photo frame's caption (`docs/gui.md`, The photo frame): the parts it drops as the
//! photo narrows, what screen readers hear whatever it drops, and the frame photographed
//! at three widths, in a wide slot and a narrow one.

use apertrust_ui::components::{
    CaptionFit, CaptionMark, CaptionName, Margin, PhotoCaption, PhotoFrame,
};
use apertrust_ui::{Builtin, Theme};
use egui::accesskit::Role;
use egui::{Color32, Rect, Sense, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

fn values() -> Vec<String> {
    ["1/250 s", "f/8", "35 mm", "ISO 200"]
        .map(str::to_owned)
        .to_vec()
}

/// What a caption of `width` points shows, measured by egui.
fn shown(width: f32, view_only: bool) -> CaptionFit {
    let mut harness = Harness::builder()
        .with_size(vec2(width.max(100.0), 40.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark)))
        .build_ui_state(
            move |ui, fit: &mut Option<CaptionFit>| {
                let exposure = values();
                let caption = PhotoCaption::new("IMG_2041.ARW")
                    .view_only(view_only)
                    .exposure(&exposure);
                *fit = Some(caption.fitted(ui, width));
            },
            None,
        );
    harness.run();
    harness.state().expect("drawn")
}

#[test]
fn a_caption_drops_its_parts_as_the_photo_narrows() {
    // Wide: everything, the exposure values to the last.
    let wide = shown(600.0, true);
    assert_eq!(
        wide,
        CaptionFit {
            exposure: 4,
            mark: CaptionMark::Word,
            name: CaptionName::Whole
        }
    );
    // Narrower, the exposure values go from the last, and the name and the mark stay.
    let mut last = 4;
    for width in (100..600).rev().step_by(5) {
        let fit = shown(width as f32, true);
        assert!(fit.exposure <= last, "{width}: the values only drop");
        last = fit.exposure;
        if fit.exposure > 0 {
            assert_eq!(
                (fit.mark, fit.name),
                (CaptionMark::Word, CaptionName::Whole),
                "{width}: the values go before the mark and the name"
            );
        }
    }
    // Then the mark's word becomes its icon, the name is cut, the name goes.
    let mut order = Vec::new();
    for width in (20..600).rev() {
        let fit = shown(width as f32, true);
        let step = (fit.exposure == 0, fit.mark, fit.name);
        if order.last() != Some(&step) {
            order.push(step);
        }
    }
    let bare = (true, CaptionMark::Word, CaptionName::Whole);
    let icon = (true, CaptionMark::Icon, CaptionName::Whole);
    let cut = (true, CaptionMark::Icon, CaptionName::Cut);
    let gone = (true, CaptionMark::Icon, CaptionName::Gone);
    let steps: Vec<_> = order.iter().filter(|step| step.0).copied().collect();
    assert_eq!(steps, [bare, icon, cut, gone], "{order:?}");
}

#[test]
fn a_photo_without_a_mark_drops_only_what_it_has() {
    // Nothing to fold into an icon: the values go, then the name is cut and goes.
    let mut seen = Vec::new();
    for width in (20..600).rev() {
        let fit = shown(width as f32, false);
        let step = (fit.exposure == 0, fit.name);
        if seen.last() != Some(&step) {
            seen.push(step);
        }
    }
    let names: Vec<_> = seen.iter().filter(|s| s.0).map(|s| s.1).collect();
    assert_eq!(
        names,
        [CaptionName::Whole, CaptionName::Cut, CaptionName::Gone]
    );
}

/// The caption in a slot `width` points wide, high enough for a 3:2 photo to fill its width.
fn framed(width: f32, margin: Margin) -> Harness<'static> {
    let height = ((width - 20.0) / 1.5 + 44.0).ceil();
    Harness::builder()
        .with_size(vec2(width, height))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark)))
        .build_ui(move |ui| {
            let surround = apertrust_ui::theme(ui.ctx()).colors.photo_surround;
            let slot = Rect::from_min_size(pos2(0.0, 0.0), vec2(width, height));
            ui.painter().rect_filled(slot, 0.0, surround);
            let frame = PhotoFrame::new(slot, margin);
            let room = frame.bounds();
            let photo = room.x.min(room.y * 1.5);
            let placed = frame.place(vec2(photo, photo / 1.5), ui.pixels_per_point());
            ui.painter()
                .rect_filled(placed.photo, 0.0, Color32::from_gray(0x60));
            let exposure = values();
            let response = PhotoCaption::new("R0001176.DNG")
                .copy(Some(2))
                .view_only(true)
                .exposure(&exposure)
                .details("Leica Q3 · 9 520 × 6 336")
                .show(ui, placed.caption);
            // Whatever the caption dropped, it keeps its width as the photo's.
            assert_eq!(response.rect, placed.caption);
            let _ = ui.allocate_rect(slot, Sense::hover());
        })
}

#[test]
fn screen_readers_hear_everything_whatever_the_width_dropped() {
    for width in [640.0, 300.0, 160.0] {
        let mut harness = framed(width, Margin::Points(10.0));
        harness.run();
        harness.get_by_role_and_label(
            Role::Label,
            "R0001176.DNG [2], View only, 1/250 s, f/8, 35 mm, ISO 200",
        );
    }
}

fn photographed(name: &str, width: f32, margin: Margin) {
    let mut harness = framed(width, margin);
    harness.run();
    harness.snapshot(name);
}

#[test]
fn the_frame_in_a_wide_slot() {
    photographed("photo-frame-wide", 640.0, Margin::Points(10.0));
}

#[test]
fn the_frame_in_a_slot_that_drops_the_values() {
    photographed("photo-frame-medium", 330.0, Margin::Points(10.0));
}

#[test]
fn the_frame_in_a_narrow_slot() {
    photographed("photo-frame-narrow", 190.0, Margin::Points(10.0));
}

/// Proof margin: the margin a fraction of the slot's smaller side, darktable's white
/// border round the photo inside it.
#[test]
fn the_frame_with_proof_margin() {
    photographed("photo-frame-proof-margin", 420.0, Margin::Proof(0.1));
}
