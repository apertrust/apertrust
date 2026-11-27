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

//! The fact list (decision 110): its rows to screen readers, the focus and its keys, the
//! copy, the end item, the labels' column; and a utility's ＋, Rows Shown.

use apertrust_ui::components::{Fact, FactCut, FactEnd, FactList, FactListOutput, Section};
use apertrust_ui::popup::{Item, Submenu};
use apertrust_ui::{Icon, Theme};
use egui::Key;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

const fn fact<'a>(label: &'a str, value: Option<&'a str>) -> Fact<'a> {
    Fact {
        label,
        value,
        tip: "",
        cut: FactCut::End,
        end: None,
        enabled: true,
    }
}

const FACTS: [Fact<'static>; 4] = [
    Fact {
        end: Some(FactEnd {
            icon: Icon::ShowInFolder,
            name: "Show in Finder",
            enabled: true,
            tip: None,
        }),
        ..fact("Folder", Some("~/Pictures/Lisbon"))
    },
    fact("Aperture", Some("f/5.6")),
    fact("Subject distance", None),
    fact("ISO", Some("ISO 200")),
];

/// A list of [`FACTS`] above a button, at `width` points, keeping what each frame reported.
fn list(width: f32) -> Harness<'static, Vec<FactListOutput>> {
    Harness::builder()
        .with_size(egui::vec2(width, 200.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, outputs: &mut Vec<FactListOutput>| {
                outputs.push(FactList::new("Exposure", &FACTS).show(ui));
                let _ = ui.button("After");
            },
            Vec::new(),
        )
}

#[test]
fn each_row_is_a_list_item_named_by_its_label_and_value() {
    let mut harness = list(320.0);
    harness.run();
    let list = harness.get_by_role_and_label(Role::List, "Exposure");
    let items: Vec<String> = (list.children())
        .filter(|node| node.accesskit_node().role() == Role::ListItem)
        .filter_map(|node| node.accesskit_node().label())
        .collect();
    assert_eq!(
        items,
        [
            "Folder: ~/Pictures/Lisbon",
            "Aperture: f/5.6",
            "Subject distance: None",
            "ISO: ISO 200",
        ]
    );
}

/// A click focuses a row and Ctrl+C copies its value; ↑, ↓, Home and End move the focus
/// in the list, which is one Tab stop: Tab leaves it.
#[test]
fn a_click_focuses_a_row_its_keys_move_and_copy() {
    let mut harness = list(320.0);
    harness.run();
    harness.get_by_label("Aperture: f/5.6").click();
    harness.run();
    assert_eq!(harness.state().last().unwrap().focused, Some(1));
    harness.event(egui::Event::Copy);
    harness.step();
    let copied = &harness.output().platform_output.commands;
    let wanted = egui::OutputCommand::CopyText("f/5.6".to_owned());
    assert!(copied.contains(&wanted), "{copied:?}");
    harness.run();
    for (key, row) in [
        (Key::ArrowDown, 2),
        (Key::ArrowDown, 3),
        (Key::ArrowDown, 3),
        (Key::Home, 0),
        (Key::ArrowUp, 0),
        (Key::End, 3),
    ] {
        harness.key_press(key);
        harness.run();
        assert_eq!(
            harness.state().last().unwrap().focused,
            Some(row),
            "{key:?}"
        );
    }
    // A row with no value copies nothing.
    harness.key_press(Key::ArrowUp);
    harness.run();
    harness.event(egui::Event::Copy);
    harness.step();
    let copied = &harness.output().platform_output.commands;
    assert!(
        !copied
            .iter()
            .any(|c| matches!(c, egui::OutputCommand::CopyText(_)))
    );
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
    assert_eq!(harness.state().last().unwrap().focused, None);
    assert!(harness.get_by_label("After").accesskit_node().is_focused());
}

/// Enter on a row presses its end item, as a click on it does.
#[test]
fn enter_or_a_click_presses_the_end_item() {
    let mut harness = list(320.0);
    harness.run();
    harness.get_by_label("Show in Finder").click();
    harness.run();
    assert!(harness.state().iter().any(|out| out.end == Some(0)));
    harness.state_mut().clear();
    harness.get_by_label("Folder: ~/Pictures/Lisbon").click();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().iter().any(|out| out.end == Some(0)));
}

/// The labels' column is as wide as the widest label, at most 40% of the row.
#[test]
fn the_labels_column_is_the_widest_at_most_two_fifths() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(240.0, 60.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, widths: &mut (f32, f32, f32)| {
                let short = FactList::column(ui, ["ISO"]);
                let long = FactList::column(ui, ["Image description (camera’s), as written"]);
                *widths = (short, long, ui.available_width());
            },
            (0.0, 0.0, 0.0),
        );
    harness.run();
    let (short, long, row) = *harness.state();
    assert!(short > 0.0 && short < 40.0, "{short}");
    assert_eq!(long, (row * 0.4).ceil());
}

/// ＋ opens the rows its utility gives, reports the row chosen, and opens again at the
/// submenu it was chosen in, so several rows are shown or hidden in a row.
#[test]
fn rows_shown_reports_the_row_chosen_and_stays_open() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(400.0, 300.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, chosen: &mut Vec<String>| {
                let rows = vec![
                    Submenu::new(
                        "exposure",
                        "Exposure",
                        vec![
                            Item::new("aperture", "Aperture").checked(true).into(),
                            Item::new("flash", "Flash").checked(false).into(),
                        ],
                    )
                    .into(),
                ];
                let out = Section::new("Metadata", false)
                    .rows_shown("Rows Shown")
                    .rows_menu(rows)
                    .options(false)
                    .show(ui, |_| {});
                chosen.extend(out.rows);
            },
            Vec::new(),
        );
    harness.run();
    harness.get_by_label("Rows Shown").click();
    harness.run();
    harness.get_by_label("Exposure").click();
    harness.run();
    harness.get_by_label("Flash").click();
    harness.run();
    assert_eq!(*harness.state(), ["flash"]);
    // Open again at Exposure.
    harness.get_by_label("Aperture").click();
    harness.run();
    assert_eq!(*harness.state(), ["flash", "aperture"]);
}
