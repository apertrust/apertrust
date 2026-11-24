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

mod common;

use apertrust_ui::Builtin;
use apertrust_ui::gallery::Gallery;

fn photograph(which: Builtin, name: &str) {
    let size = egui::vec2(600.0, 900.0);
    common::photograph_section(which, size, name, Gallery::show_overlays);
}

#[test]
fn the_overlays_in_the_dark_theme() {
    photograph(Builtin::Dark, "overlay-dark");
}

/// darktable's ten guide types (decision 92), each over a mid-grey photo 280 by 180 points,
/// unflipped but for harmonious triangles and the golden spiral, flipped both ways beside
/// their unflipped forms: the overlay's secondary line, the grid's at its lower opacities.
#[test]
fn the_guides_are_photographed() {
    use apertrust_ui::guides::{self, Guide, GuideFlip};
    use egui_kittest::Harness;
    let photo = egui::vec2(280.0, 180.0);
    let mut cells: Vec<(Guide, GuideFlip)> = Guide::ALL
        .into_iter()
        .map(|guide| (guide, GuideFlip::None))
        .collect();
    cells.push((Guide::HarmoniousTriangles, GuideFlip::Both));
    cells.push((Guide::GoldenSpiral, GuideFlip::Both));
    let rows = cells.len().div_ceil(2) as f32;
    let size = egui::vec2(2.0 * photo.x + 36.0, rows * (photo.y + 12.0) + 12.0);
    let mut harness = Harness::builder()
        .with_size(size)
        .with_pixels_per_point(2.0)
        .with_context_setup(|ctx| apertrust_ui::install(ctx, apertrust_ui::Theme::default()))
        .build_ui(move |ui| {
            let painter = ui.painter();
            painter.rect_filled(ui.ctx().content_rect(), 0.0, egui::Color32::from_gray(40));
            for (i, (guide, flip)) in cells.iter().enumerate() {
                let at = egui::pos2(
                    12.0 + (i % 2) as f32 * (photo.x + 12.0),
                    12.0 + (i / 2) as f32 * (photo.y + 12.0),
                );
                let rect = egui::Rect::from_min_size(at, photo);
                painter.rect_filled(rect, 0.0, egui::Color32::from_gray(119));
                guides::draw(&painter.with_clip_rect(rect), rect, *guide, *flip);
            }
        });
    harness.run();
    harness.snapshot("guides");
}
