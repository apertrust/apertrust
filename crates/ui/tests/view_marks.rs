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

//! The zoomed view's position marks (`docs/gui.md`, Where the view is): where the bars lie
//! and what they span, and the gallery's View marks section photographed at twice the size
//! in each theme. Accepted with `UPDATE_SNAPSHOTS=1 cargo test -p apertrust-ui --test
//! view_marks`, reviewed like a golden image.

mod common;

use apertrust_ui::components::ViewMarks;
use apertrust_ui::gallery::Gallery;
use apertrust_ui::{Builtin, Theme};
use egui::{Rangef, Rect, pos2};

/// The photo area the marks go round, in points.
const AREA: Rect = Rect {
    min: pos2(40.0, 30.0),
    max: pos2(440.0, 330.0),
};

/// The bars `marks` paints where `rtl` says which way the language goes: the rectangles
/// filled in the secondary text's colour.
fn painted(marks: ViewMarks, rtl: bool) -> Vec<Rect> {
    let ctx = egui::Context::default();
    apertrust_ui::install(&ctx, Theme::builtin(Builtin::Dark));
    let colour = apertrust_ui::theme(&ctx).colors.text_secondary;
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        marks.paint(ui.painter(), &apertrust_ui::theme(ui.ctx()), rtl);
    });
    // The fonts' atlas, which nothing here draws.
    output.textures_delta.clear();
    (output.shapes.iter())
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect) if rect.fill == colour => Some(rect.rect),
            _ => None,
        })
        .collect()
}

/// The span from `a` to `b`.
const fn span(a: f32, b: f32) -> Option<Rangef> {
    Some(Rangef { min: a, max: b })
}

/// Whether `marks` painted where `rtl` says gives the bars `expected`, as left, top, right
/// and bottom, each side within a point.
fn paints(marks: ViewMarks, rtl: bool, expected: &[[f32; 4]]) {
    let bars = painted(marks, rtl);
    let sides = |r: &Rect| [r.left(), r.top(), r.right(), r.bottom()];
    let near = |a: [f32; 4], b: [f32; 4]| a.iter().zip(b).all(|(a, b)| (a - b).abs() <= 1.0);
    assert_eq!(bars.len(), expected.len(), "{bars:?}");
    for (bar, want) in bars.iter().zip(expected) {
        assert!(near(sides(bar), *want), "{bar:?}, not {want:?}");
    }
}

#[test]
fn the_marks_show_the_visible_span_on_an_overflowing_axis_only() {
    // 2 points thick, centred in the 12-point margin under the area and at its right; the
    // bottom one under the caption's line where there is one; nothing faded out.
    let marks = ViewMarks::new(AREA, 12.0);
    let cases: [(ViewMarks, &[[f32; 4]]); 4] = [
        (
            marks.across(span(0.25, 0.5)),
            &[[140.0, 335.0, 240.0, 337.0]],
        ),
        (marks.down(span(0.0, 0.3)), &[[445.0, 30.0, 447.0, 120.0]]),
        (
            marks.caption(24.0).across(span(0.0, 1.0)),
            &[[40.0, 359.0, 440.0, 361.0]],
        ),
        (marks.across(span(0.0, 0.5)).opacity(0.0), &[]),
    ];
    for (marks, expected) in cases {
        paints(marks, false, expected);
    }
}

#[test]
fn the_marks_hide_under_a_4_point_margin() {
    for (margin, shown) in [(0.0, 0), (3.5, 0), (4.0, 2), (40.0, 2)] {
        let marks = ViewMarks::new(AREA, margin).across(span(0.2, 0.6));
        let count = painted(marks.down(span(0.2, 0.6)), false).len();
        assert_eq!(count, shown, "{margin}");
    }
}

#[test]
fn the_end_bar_is_on_the_left_in_a_right_to_left_language() {
    let marks = ViewMarks::new(AREA, 12.0).down(span(0.5, 1.0));
    paints(marks, true, &[[33.0, 180.0, 35.0, 330.0]]);
}

/// The gallery's View marks at twice the size, in each theme: `view-marks-dark` and the
/// rest.
#[test]
fn the_view_marks_are_photographed_in_every_theme() {
    let mut results = egui_kittest::SnapshotResults::new();
    for which in Builtin::ALL {
        let name = format!("view-marks-{}", which.id());
        let size = egui::vec2(600.0, 180.0);
        results.extend(common::photograph_section(
            which,
            size,
            &name,
            Gallery::show_view_marks,
        ));
    }
}
