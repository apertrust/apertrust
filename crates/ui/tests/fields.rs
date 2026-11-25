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

//! The fields row (decision 107; `docs/gui.md`, Line rows and end items): its fields
//! sharing the line's width, its fold below its least width and only there, its labels
//! folded, a disabled field, the group screen readers hear, the Tab order and a typed
//! value reported once; and the gallery's Fields row, photographed at twice the size.
//! Accepted with `UPDATE_SNAPSHOTS=1 cargo test -p apertrust-ui --test fields`, reviewed
//! like a golden image.

mod common;

use apertrust_ui::components::{FieldsRow, RowField};
use apertrust_ui::gallery::Gallery;
use apertrust_ui::{Builtin, Theme};
use egui::accesskit::Role;
use egui::{Event, Key};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

/// The row's width, the width and height it shows, whether the height is given, and what
/// it reported.
struct State {
    width: f32,
    values: [f64; 2],
    given: bool,
    least: f32,
    reported: Vec<(usize, f64)>,
}

/// Export's size in pixels as a fields row `width` points wide.
fn row(width: f32, given: bool) -> Harness<'static, State> {
    let state = State {
        width,
        values: [2048.0, 1365.0],
        given,
        least: 0.0,
        reported: Vec::new(),
    };
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 120.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, state: &mut State| {
                let mut height = RowField::new("Height", state.values[1], 0.0..=100_000.0)
                    .before("×")
                    .unit("px");
                if state.given {
                    height = height.disabled("Set by --conf for this run");
                }
                let fields = [
                    RowField::new("Width", state.values[0], 0.0..=100_000.0).unit("px"),
                    height,
                ];
                ui.allocate_ui(egui::vec2(state.width, 100.0), |ui| {
                    ui.set_width(state.width);
                    let row = FieldsRow::new(egui::Id::unique("size"), "Size in pixels", &fields);
                    state.least = row.least_width(ui);
                    if let Some((at, value)) = row.show(ui) {
                        state.values[at] = value;
                        state.reported.push((at, value));
                    }
                });
                let _ = ui.button("After");
            },
            state,
        );
    harness.run();
    harness
}

const WIDTH: &str = "Width, in px";
const HEIGHT: &str = "Height, in px";

fn field(harness: &Harness<'_, State>, name: &str) -> egui::Rect {
    harness.get_by_role_and_label(Role::TextInput, name).rect()
}

/// Whether the fields are on one line.
fn one_line(harness: &Harness<'_, State>) -> bool {
    (field(harness, WIDTH).center().y - field(harness, HEIGHT).center().y).abs() < 0.5
}

fn least() -> f32 {
    row(296.0, false).state().least
}

#[test]
fn on_one_line_the_fields_share_the_width_equally() {
    let harness = row(296.0, false);
    assert!(one_line(&harness));
    let (w, h) = (field(&harness, WIDTH), field(&harness, HEIGHT));
    assert_eq!(w.width(), h.width());
    assert!(w.right() < h.left());
    // No label of its own, nor the fields' names.
    assert!(harness.query_by_label("Width").is_none());
    assert!(harness.query_by_label("Size in pixels").is_some());
    // Wider, they grow alike.
    let wide = row(400.0, false);
    assert_eq!(field(&wide, WIDTH).width(), field(&wide, HEIGHT).width());
    assert!(field(&wide, WIDTH).width() > w.width());
}

#[test]
fn it_folds_exactly_below_its_least_width() {
    let least = least();
    // Two fields as wide as "100000" needs, the separator, the unit, the gaps and the inset.
    assert!(least > 100.0 && least < 200.0, "{least}");
    assert!(one_line(&row(least.ceil(), false)), "at {least}");
    assert!(one_line(&row(least.ceil() + 1.0, false)));
    let folded = row(least.ceil() - 1.0, false);
    assert!(!one_line(&folded), "under {least}");
    // Width above Height, each its own row.
    let (w, h) = (field(&folded, WIDTH), field(&folded, HEIGHT));
    assert!(w.bottom() <= h.top(), "{w:?} {h:?}");
    // Typing never folds it: the fold follows the width, never the values.
    let mut harness = row(least.ceil(), false);
    harness.state_mut().values = [99_999.0, 99_999.0];
    harness.run();
    assert!(one_line(&harness));
}

#[test]
fn folded_each_row_carries_its_label() {
    let harness = row(140.0, false);
    assert!(!one_line(&harness));
    for (label, name) in [("Width", WIDTH), ("Height", HEIGHT)] {
        let label = harness.get_by_label(label).rect();
        let field = field(&harness, name);
        assert!((label.center().y - field.center().y).abs() < 1.0, "{name}");
        assert!(label.right() < field.left(), "{name}");
    }
    // No separator folded.
    assert!(harness.query_by_label("×").is_none());
}

#[test]
fn a_disabled_field_takes_no_typing_and_says_why() {
    let mut harness = row(296.0, true);
    let height = harness.get_by_role_and_label(Role::TextInput, HEIGHT);
    assert!(height.accesskit_node().is_disabled());
    assert_eq!(
        height.accesskit_node().description().as_deref(),
        Some("Set by --conf for this run")
    );
    height.focus();
    harness.run();
    harness.event(Event::Text("4".to_owned()));
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().reported.is_empty());
    assert_eq!(harness.state().values[1], 1365.0);
}

#[test]
fn screen_readers_hear_one_group_of_its_fields_folded_or_not() {
    for width in [296.0, 140.0] {
        let harness = row(width, false);
        let group = harness.get_by_role_and_label(Role::Group, "Size in pixels");
        let inside: Vec<String> = group
            .children_recursive()
            .filter(|node| node.accesskit_node().role() == Role::TextInput)
            .filter_map(|node| node.accesskit_node().label())
            .collect();
        assert_eq!(inside, [WIDTH, HEIGHT], "{width}");
        assert_eq!(harness.query_all_by_role(Role::Group).count(), 1, "{width}");
    }
}

fn focused(harness: &Harness<'_, State>) -> String {
    harness
        .get_by(|node| node.is_focused())
        .accesskit_node()
        .label()
        .unwrap_or_default()
}

#[test]
fn tab_goes_left_to_right_then_top_to_bottom() {
    for width in [296.0, 140.0] {
        let mut harness = row(width, false);
        harness
            .get_by_role_and_label(Role::TextInput, WIDTH)
            .focus();
        harness.run();
        assert_eq!(focused(&harness), WIDTH);
        harness.key_press(Key::Tab);
        harness.run();
        assert_eq!(focused(&harness), HEIGHT, "{width}");
        harness.key_press(Key::Tab);
        harness.run();
        assert_eq!(focused(&harness), "After", "{width}");
    }
}

#[test]
fn a_value_typed_is_reported_once_on_enter() {
    let mut harness = row(296.0, false);
    harness
        .get_by_role_and_label(Role::TextInput, WIDTH)
        .focus();
    harness.run();
    harness.event(Event::Text("4096".to_owned()));
    harness.run();
    assert!(harness.state().reported.is_empty(), "nothing while typing");
    harness.key_press(Key::Enter);
    harness.run();
    harness.run();
    assert_eq!(harness.state().reported, [(0, 4096.0)]);
}

fn photograph(which: Builtin, name: &str, show: fn(&mut Gallery, &mut egui::Ui)) {
    let size = egui::vec2(330.0, 90.0);
    common::photograph_section(which, size, name, show).unwrap();
}

#[test]
fn the_fields_row_in_the_dark_theme() {
    photograph(Builtin::Dark, "fields-row-dark", Gallery::show_fields_row);
}

#[test]
fn the_fields_row_in_the_light_theme() {
    photograph(Builtin::Light, "fields-row-light", Gallery::show_fields_row);
}

#[test]
fn the_fields_row_folded_in_the_dark_theme() {
    let show = Gallery::show_fields_row_folded;
    photograph(Builtin::Dark, "fields-row-folded-dark", show);
}

#[test]
fn the_fields_row_folded_in_the_light_theme() {
    let show = Gallery::show_fields_row_folded;
    photograph(Builtin::Light, "fields-row-folded-light", show);
}

#[test]
fn the_fields_row_with_a_field_given() {
    photograph(
        Builtin::Dark,
        "fields-row-given",
        Gallery::show_fields_row_given,
    );
}
