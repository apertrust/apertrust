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

//! The Settings dialog's chips (`docs/gui.md`, The Settings dialog): a list of a choice as
//! toggle chips wrapping in a well, as the log's categories, driven by the pointer and
//! the keys and read through AccessKit.

use apertrust_ui::components::settings::Chips;
use apertrust_ui::{Theme, install};
use egui::accesskit::{Role, Toggled};
use egui::{Key, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

const OPTIONS: [&str; 9] = [
    "GPU",
    "Raw",
    "Pipe",
    "Performance",
    "Memory",
    "Settings",
    "Control",
    "Events",
    "Verbose",
];

/// The chips in a column `width` points wide, over `on`.
fn chips(width: f32, on: [bool; 9]) -> Harness<'static, [bool; 9]> {
    let mut harness = Harness::builder()
        .with_size(vec2(width + 20.0, 160.0))
        .with_context_setup(|ctx| install(ctx, Theme::default()))
        .build_ui_state(
            move |ui, on: &mut [bool; 9]| {
                ui.set_width(width);
                if let Some((i, set)) = Chips::new("Log categories", &OPTIONS, &on[..]).show(ui) {
                    on[i] = set;
                }
            },
            on,
        );
    harness.run();
    harness
}

fn toggled(harness: &Harness<'_, [bool; 9]>, name: &str) -> Option<Toggled> {
    harness
        .get_by_role_and_label(Role::Button, name)
        .accesskit_node()
        .toggled()
}

#[test]
fn each_chip_is_a_toggle_button_a_click_or_space_turns_on_or_off() {
    let mut on = [true; 9];
    on[4] = false;
    let mut harness = chips(260.0, on);
    assert_eq!(toggled(&harness, "GPU"), Some(Toggled::True));
    assert_eq!(toggled(&harness, "Memory"), Some(Toggled::False));
    // Each chip is the group's, for screen readers.
    let group = harness.get_by_role_and_label(Role::Group, "Log categories");
    assert_eq!(group.query_all_by_role(Role::Button).count(), OPTIONS.len());
    harness
        .get_by_role_and_label(Role::Button, "Memory")
        .click();
    harness.run();
    assert!(harness.state()[4]);
    assert_eq!(toggled(&harness, "Memory"), Some(Toggled::True));
    harness.get_by_role_and_label(Role::Button, "GPU").focus();
    harness.run();
    harness.key_press(Key::Space);
    harness.run();
    assert!(!harness.state()[0]);
    assert_eq!(toggled(&harness, "GPU"), Some(Toggled::False));
}

#[test]
fn the_chips_wrap_at_the_column_s_width_in_a_well_as_wide() {
    for width in [260.0, 160.0] {
        let harness = chips(width, [true; 9]);
        let well = harness
            .get_by_role_and_label(Role::Group, "Log categories")
            .rect();
        assert_eq!(well.width(), width);
        let rects: Vec<egui::Rect> = OPTIONS
            .iter()
            .map(|name| harness.get_by_role_and_label(Role::Button, name).rect())
            .collect();
        for rect in &rects {
            assert_eq!(rect.height(), 22.0);
            assert!(well.contains_rect(*rect), "{rect:?} in {well:?}");
        }
        // More than one line, each chip after the last on its line or at the next's start.
        let lines = rects
            .windows(2)
            .filter(|pair| pair[1].top() > pair[0].top())
            .count();
        assert!(lines >= 2, "{width}: {lines}");
        for pair in rects.windows(2) {
            if pair[1].top() == pair[0].top() {
                assert_eq!(pair[1].left() - pair[0].right(), 4.0);
            } else {
                assert_eq!(pair[1].left(), well.left() + 3.0);
                assert_eq!(pair[1].top() - pair[0].bottom(), 4.0);
            }
        }
    }
}

/// An on chip's words are `text_strong` on `raised`, an off chip's `text_secondary`.
#[test]
fn each_chip_s_words_take_their_state_s_colour() {
    let mut on = [true; 9];
    on[4] = false;
    let harness = chips(260.0, on);
    let t = Theme::default();
    let colour_of = |word: &str| {
        harness
            .output()
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.galley.job.text == word => {
                    Some(text.fallback_color)
                }
                _ => None,
            })
            .expect("its words")
    };
    assert_eq!(colour_of("GPU"), t.colors.text_strong);
    assert_eq!(colour_of("Memory"), t.colors.text_secondary);
}
