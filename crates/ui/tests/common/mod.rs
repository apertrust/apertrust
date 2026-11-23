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

//! What several of the design system's tests share: a section of the gallery photographed
//! alone. Each test binary uses part of it.

#![allow(dead_code)]

use apertrust_ui::gallery::Gallery;
use apertrust_ui::{Builtin, Theme};
use egui_kittest::{Harness, SnapshotResults};

/// Photographs as `name` the gallery's section `show` draws, in the theme `which`, at twice
/// the size in a window `size` points large, on the panel's colour inside a 12-point margin.
/// Returns the photograph's result, which fails the test as it is dropped where it differs.
pub fn photograph_section(
    which: Builtin,
    size: egui::Vec2,
    name: &str,
    show: fn(&mut Gallery, &mut egui::Ui),
) -> SnapshotResults {
    photograph_section_at(which, size, 2.0, 1.0, name, show)
}

/// Photographs it as [`photograph_section`] does, on a screen of `scale` physical pixels a
/// point, at the interface size `zoom`, egui's zoom factor as Settings' Interface size sets
/// it, in a window `size` points large.
pub fn photograph_section_at(
    which: Builtin,
    size: egui::Vec2,
    scale: f32,
    zoom: f32,
    name: &str,
    show: fn(&mut Gallery, &mut egui::Ui),
) -> SnapshotResults {
    let mut harness = Harness::builder()
        .with_size(size)
        .with_pixels_per_point(scale)
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::builtin(which));
            ctx.set_zoom_factor(zoom);
        })
        .build_ui_state(
            move |ui, gallery: &mut Gallery| {
                let panel = apertrust_ui::theme(ui.ctx()).colors.panel;
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                egui::Frame::new()
                    .inner_margin(12.0)
                    .show(ui, |ui| show(gallery, ui));
            },
            Gallery::new(which),
        );
    harness.run();
    harness.snapshot(name);
    harness.take_snapshot_results()
}
