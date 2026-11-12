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

//! The on-photo overlay's primitives (`docs/gui.md`, On-photo overlays): the gallery's
//! section, each weight, handle and point mark, the label and the focus ring over a light,
//! a dark and a busy photo, photographed at twice the size, where a line's pixels show. The
//! overlay's colours are the same in every theme; the gallery's snapshots hold the section,
//! and the ring's colour, in each. Accepted with `UPDATE_SNAPSHOTS=1 cargo test -p
//! apertrust-ui --test overlay`, reviewed like a golden image.

use apertrust_ui::gallery::Gallery;
use apertrust_ui::{Builtin, Theme};
use egui_kittest::Harness;

fn photograph(which: Builtin, name: &str) {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(600.0, 900.0))
        .with_pixels_per_point(2.0)
        .with_context_setup(move |ctx| apertrust_ui::install(ctx, Theme::builtin(which)))
        .build_ui_state(
            |ui, gallery: &mut Gallery| {
                let panel = apertrust_ui::theme(ui.ctx()).colors.panel;
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                egui::Frame::new().inner_margin(12.0).show(ui, |ui| {
                    gallery.show_overlays(ui);
                });
            },
            Gallery::new(which),
        );
    harness.run();
    harness.snapshot(name);
}

#[test]
fn the_overlays_in_the_dark_theme() {
    photograph(Builtin::Dark, "overlay-dark");
}
