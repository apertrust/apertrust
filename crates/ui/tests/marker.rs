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

//! The slider's marker (decision 75): each shape photographed from the gallery in the dark
//! and the light theme, at rest at the track's ends and its middle and while dragged, and
//! a press at the track's very ends giving its ends. Accepted with `UPDATE_SNAPSHOTS=1
//! cargo test -p apertrust-ui --test marker`, reviewed like a golden image.

use apertrust_ui::components::{Marker, set_marker};
use apertrust_ui::gallery::Gallery;
use apertrust_ui::{Builtin, Theme};
use egui::accesskit::Role;
use egui::pos2;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

/// A 320-point side panel's content, less its margins.
const WIDTH: f32 = 296.0;

fn markers(which: Builtin, marker: Marker) -> Harness<'static, Gallery> {
    Harness::builder()
        .with_size(egui::vec2(WIDTH + 24.0, 200.0))
        // Twice the size, so a review sees each shape's outline and border.
        .with_pixels_per_point(2.0)
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::builtin(which));
            set_marker(ctx, marker);
        })
        .build_ui_state(
            |ui, gallery: &mut Gallery| {
                let panel = apertrust_ui::theme(ui.ctx()).colors.panel;
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                egui::Frame::new().inner_margin(12.0).show(ui, |ui| {
                    ui.set_width(WIDTH);
                    gallery.show_markers(ui);
                });
            },
            Gallery::new(which),
        )
}

/// The value the row named `name` holds, as screen readers are told it.
fn value(harness: &Harness<'_, Gallery>, name: &str) -> f64 {
    harness
        .get_by_role_and_label(Role::Slider, name)
        .accesskit_node()
        .numeric_value()
        .expect("a slider's value")
}

fn photograph(which: Builtin, marker: Marker, name: &str) {
    let mut harness = markers(which, marker);
    harness.run();
    // Pressed where its marker sits, so it is dragged without moving, at the strip's
    // bottom, so the pointer drawn hides none of it.
    let track = harness
        .get_by_role_and_label(Role::Slider, "Exposure, dragged")
        .rect();
    let travel = marker.travel(track.x_range());
    let at = pos2(
        travel.min + (1.28 + 4.0) / 8.0 * travel.span(),
        track.bottom() - 1.0,
    );
    harness.drag_at(at);
    harness.run();
    assert!((value(&harness, "Exposure, dragged") - 1.28).abs() < 0.015);
    harness.snapshot(name);
}

#[test]
fn the_triangle_in_the_dark_theme() {
    photograph(
        Builtin::Dark,
        Marker::Triangle,
        "slider-marker-triangle-dark",
    );
}

#[test]
fn the_triangle_in_the_light_theme() {
    photograph(
        Builtin::Light,
        Marker::Triangle,
        "slider-marker-triangle-light",
    );
}

#[test]
fn the_circle_in_the_dark_theme() {
    photograph(Builtin::Dark, Marker::Circle, "slider-marker-circle-dark");
}

#[test]
fn the_circle_in_the_light_theme() {
    photograph(Builtin::Light, Marker::Circle, "slider-marker-circle-light");
}

#[test]
fn the_square_in_the_dark_theme() {
    photograph(Builtin::Dark, Marker::Square, "slider-marker-square-dark");
}

#[test]
fn the_square_in_the_light_theme() {
    photograph(Builtin::Light, Marker::Square, "slider-marker-square-light");
}

/// A press at the strip's very first or last point gives the track's end, though the
/// marker's travel stops short of it.
#[test]
fn a_press_at_the_track_s_very_ends_gives_its_ends() {
    for marker in Marker::ALL {
        let mut harness = markers(Builtin::Dark, marker);
        harness.run();
        let name = "Exposure, at its default";
        let track = harness.get_by_role_and_label(Role::Slider, name).rect();
        for (x, end) in [(track.left(), -4.0), (track.right() - 0.5, 4.0)] {
            let at = pos2(x, track.center().y);
            harness.drag_at(at);
            harness.run();
            harness.drop_at(at);
            harness.run();
            assert_eq!(value(&harness, name), end, "{marker:?}");
        }
    }
}
