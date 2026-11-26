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
    CaptionCentre, CaptionFit, CaptionMark, CaptionName, CaptionTips, CellAction, CellRating,
    ColourLabel, ColourLabels, Margin, PhotoCaption, PhotoFrame,
};
use apertrust_ui::{Builtin, Theme};
use egui::accesskit::{Role, Toggled};
use egui::{Color32, Rect, Sense, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

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
            centre: CaptionCentre::Whole,
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
                .show(ui, placed.caption)
                .response;
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
        let caption = harness.get_by_role_and_label(
            Role::Label,
            "R0001176.DNG [2], View only, 1/250 s, f/8, 35 mm, ISO 200",
        );
        assert_eq!(
            caption.accesskit_node().description().as_deref(),
            Some("Leica Q3 · 9 520 × 6 336")
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

/// Three stars, the red and blue labels.
fn three_stars() -> CellRating {
    CellRating {
        stars: 3,
        rejected: false,
        labels: ColourLabels::of([ColourLabel::Red, ColourLabel::Blue]),
    }
}

/// A caption line `width` points wide on the photo's surround, in `theme`, rated `rating`,
/// its centre's last action kept as the harness's state.
fn rated(theme: Builtin, width: f32, rating: CellRating) -> Harness<'static, Option<CellAction>> {
    Harness::builder()
        .with_size(vec2(width, 32.0))
        .with_context_setup(move |ctx| apertrust_ui::install(ctx, Theme::builtin(theme)))
        .build_ui_state(
            move |ui, action: &mut Option<CellAction>| {
                let surround = apertrust_ui::theme(ui.ctx()).colors.photo_surround;
                let whole = Rect::from_min_size(pos2(0.0, 0.0), vec2(width, 32.0));
                ui.painter().rect_filled(whole, 0.0, surround);
                let exposure = values();
                let tips = CaptionTips::plain(ui.ctx());
                let line = Rect::from_min_size(pos2(0.0, 6.0), vec2(width, 20.0));
                let shown = PhotoCaption::new("IMG_2041.ARW")
                    .exposure(&exposure)
                    .rating(Some(rating))
                    .tips(&tips)
                    .show(ui, line);
                if shown.action.is_some() {
                    *action = shown.action;
                }
            },
            None,
        )
}

/// A star, Reject and a label each report their action; a click changes nothing shown,
/// the caller sends the command.
#[test]
fn the_caption_s_controls_report_what_was_clicked() {
    let clicks = [
        (Role::RadioButton, "4 stars", CellAction::Rate(4)),
        (Role::RadioButton, "3 stars", CellAction::Rate(3)),
        (Role::Button, "Reject", CellAction::ToggleReject),
        (
            Role::Button,
            "Green label",
            CellAction::ToggleLabel(ColourLabel::Green),
        ),
    ];
    for (role, label, wanted) in clicks {
        let mut harness = rated(Builtin::Dark, 640.0, three_stars());
        harness.run();
        harness.get_by_role_and_label(role, label).click();
        harness.run();
        assert_eq!(*harness.state(), Some(wanted), "{label}");
    }
    // The rating's star is the checked one; a set label is pressed.
    let harness = rated(Builtin::Dark, 640.0, three_stars());
    let checked = |label: &str| {
        let star = harness.get_by_role_and_label(Role::RadioButton, label);
        star.accesskit_node().toggled()
    };
    assert_eq!(checked("3 stars"), Some(Toggled::True));
    assert_eq!(checked("4 stars"), Some(Toggled::False));
}

/// The stars are one Tab stop after Reject, a radio group: → rates one more, ← one fewer,
/// and Space on the checked one clears it, as its click does; at five stars → and at one
/// ← do nothing, never clearing the rating.
#[test]
fn the_stars_are_one_tab_stop_moved_by_the_arrows() {
    let stars = |stars| CellRating {
        stars,
        ..three_stars()
    };
    for (rating, key, wanted) in [
        (3, egui::Key::ArrowRight, Some(CellAction::Rate(4))),
        (3, egui::Key::ArrowLeft, Some(CellAction::Rate(2))),
        (3, egui::Key::Space, Some(CellAction::Rate(3))),
        (5, egui::Key::ArrowRight, None),
        (1, egui::Key::ArrowLeft, None),
    ] {
        let mut harness = rated(Builtin::Dark, 640.0, stars(rating));
        harness.run();
        harness.key_press(egui::Key::Tab);
        harness.key_press(egui::Key::Tab);
        harness.run();
        let group = harness.get_by_role_and_label(Role::RadioGroup, "Rating");
        assert!(group.accesskit_node().is_focused(), "{key:?}");
        harness.key_press(key);
        harness.run();
        assert_eq!(*harness.state(), wanted, "{rating} {key:?}");
    }
}

/// Narrow, the labels fold into dots and then the stars into one star and the number,
/// read only; screen readers still hear the rating.
#[test]
fn a_narrow_caption_folds_its_rating_and_still_says_it() {
    let mut harness = rated(Builtin::Dark, 150.0, three_stars());
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(Role::RadioGroup, "Rating")
            .is_none()
    );
    harness.get_by_role_and_label(
        Role::Label,
        "IMG_2041.ARW, 3 stars, Colour labels: Red, Blue, 1/250 s, f/8, 35 mm, ISO 200",
    );
}

/// The caption's centre photographed on `theme` and on the light theme: three stars and
/// two labels at rest, rejected, under the pointer, and folded.
#[test]
fn the_caption_s_centre_in_each_state() {
    let rejected = CellRating {
        rejected: true,
        ..three_stars()
    };
    let mut results = egui_kittest::SnapshotResults::new();
    for (theme, name) in [(Builtin::Dark, "dark"), (Builtin::Light, "light")] {
        let mut harness = rated(theme, 640.0, three_stars());
        harness.run();
        results.add(harness.try_snapshot(format!("photo-caption-rated-{name}")));
        let star = harness.get_by_role_and_label(Role::RadioButton, "5 stars");
        let at = star.rect().center();
        harness.hover_at(at);
        harness.run();
        results.add(harness.try_snapshot(format!("photo-caption-hover-{name}")));
        let mut harness = rated(theme, 640.0, rejected);
        harness.run();
        results.add(harness.try_snapshot(format!("photo-caption-rejected-{name}")));
        let mut harness = rated(theme, 200.0, rejected);
        harness.run();
        results.add(harness.try_snapshot(format!("photo-caption-folded-{name}")));
    }
}
