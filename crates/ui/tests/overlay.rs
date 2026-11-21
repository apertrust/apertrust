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
