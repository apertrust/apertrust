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

//! The active picker's label and the pointer's badge (`docs/pickers.md`, On the photo):
//! the gallery's rows of them over a light, a dark and a busy photo, in place, focused,
//! near each edge and in the corner, at interface sizes 100 % and 200 % (egui's zoom factor,
//! as Settings' Interface size sets it). The overlay's colours are the same in every theme.
//! Accepted with `UPDATE_SNAPSHOTS=1 cargo test -p
//! apertrust-ui --test picker_label`, reviewed like a golden image.

mod common;

use apertrust_ui::Builtin;
use apertrust_ui::gallery::Gallery;

/// Photographs the rows on a screen of a pixel a point at the interface size `zoom`.
fn photograph(zoom: f32, name: &str) {
    let size = egui::vec2(600.0, 780.0);
    let show = Gallery::show_picker_labels;
    common::photograph_section_at(Builtin::Dark, size, 1.0, zoom, name, show);
}

#[test]
fn the_picker_s_labels_and_badges_at_100_percent() {
    photograph(1.0, "picker-label-100");
}

#[test]
fn the_picker_s_labels_and_badges_at_200_percent() {
    photograph(2.0, "picker-label-200");
}
