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

//! Several photos selected in the Browser (`docs/gui.md`, Selection and the active photo):
//! the keys that select, what screen readers hear of the selection, and the marks and the
//! browser bar's count, photographed in every theme.

use apertrust_ui::components::{Browser, BrowserAction, CellRating, Thumb, ViewerBar, Zoom};
use apertrust_ui::{Builtin, Theme};
use egui::accesskit::{AriaCurrent, Role};
use egui::os::OperatingSystem;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

/// The Browser with eight photos, IMG_0001 to IMG_0003 selected and IMG_0002 active, on
/// `os`, and what the user did to them.
fn selecting(os: OperatingSystem) -> Harness<'static, Vec<BrowserAction>> {
    Harness::builder()
        .with_size(egui::vec2(1000.0, 220.0))
        .with_os(os)
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, actions: &mut Vec<BrowserAction>| {
                let names: Vec<String> = (0..8).map(|i| format!("IMG_{i:04}.ARW")).collect();
                let thumb = |i: usize| Thumb {
                    name: &names[i],
                    edited: false,
                    status: None,
                    selected: (1..=3).contains(&i),
                    picture: None,
                    no_preview: false,
                    rating: None,
                };
                actions.extend(Browser::new(8, Some(2), thumb).show(ui).action);
            },
            Vec::new(),
        )
}

/// Ctrl+Space toggles the focused cell's photo, Cmd+Space or Control+Space on the Mac,
/// where Spotlight holds Cmd+Space; Space alone still opens it.
#[test]
fn ctrl_space_toggles_the_focused_cell_s_photo_and_space_still_opens_it() {
    for (os, modifiers) in [
        // egui-winit sets both for Ctrl on Linux and Windows.
        (OperatingSystem::Nix, Modifiers::CTRL | Modifiers::COMMAND),
        (
            OperatingSystem::Windows,
            Modifiers::CTRL | Modifiers::COMMAND,
        ),
        (
            OperatingSystem::Mac,
            Modifiers::MAC_CMD | Modifiers::COMMAND,
        ),
        (OperatingSystem::Mac, Modifiers::CTRL),
    ] {
        let mut harness = selecting(os);
        harness.run();
        harness.get_by_label("IMG_0005.ARW").focus();
        harness.run();
        harness.key_press_modifiers(modifiers, Key::Space);
        harness.run();
        assert_eq!(
            *harness.state(),
            [BrowserAction::ToggleSelected(5)],
            "{os:?} {modifiers:?}"
        );
        // The focus stays, and Space alone opens the photo.
        harness.key_press(Key::Space);
        harness.run();
        assert_eq!(
            harness.state().last(),
            Some(&BrowserAction::Open(5)),
            "{os:?}"
        );
    }
}

/// The Browser is a list that allows several to be selected, and each cell says whether it
/// is: the selected ones and the active one, which is always selected.
#[test]
fn the_list_is_multi_selectable_and_each_cell_says_whether_it_is_selected() {
    let mut harness = selecting(OperatingSystem::Nix);
    harness.run();
    let list = harness.get_by_role_and_label(Role::List, "Browser");
    assert!(list.accesskit_node().is_multiselectable());
    // Where several cannot be, it says nothing of it.
    let mut one = Harness::new_ui(|ui| {
        apertrust_ui::install(ui.ctx(), Theme::default());
        let thumb = |_: usize| Thumb {
            name: "IMG_0000.ARW",
            edited: false,
            status: None,
            selected: false,
            picture: None,
            no_preview: false,
            rating: None,
        };
        Browser::new(1, Some(0), thumb).several(false).show(ui);
    });
    one.run();
    let list = one.get_by_role_and_label(Role::List, "Browser");
    assert!(!list.accesskit_node().is_multiselectable());
    let selected = |name: &str| harness.get_by_label(name).accesskit_node().is_selected();
    for (name, is) in [
        ("IMG_0000.ARW", false),
        ("IMG_0001.ARW", true),
        ("IMG_0002.ARW", true),
        ("IMG_0003.ARW", true),
        ("IMG_0004.ARW", false),
    ] {
        assert_eq!(selected(name), Some(is), "{name}");
    }
    // The active photo, of those selected, is the list's current item; no other is.
    let current = |name: &str| harness.get_by_label(name).accesskit_node().aria_current();
    assert_eq!(current("IMG_0002.ARW"), Some(AriaCurrent::True));
    for name in ["IMG_0001.ARW", "IMG_0003.ARW", "IMG_0004.ARW"] {
        assert_eq!(current(name), None, "{name}");
    }
}

/// A disabled Browser takes no Ctrl+Space, though a cell kept the focus from before.
#[test]
fn a_disabled_browser_takes_no_ctrl_space() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1000.0, 220.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, (disabled, actions): &mut (bool, Vec<BrowserAction>)| {
                if *disabled {
                    ui.disable();
                }
                let names: Vec<String> = (0..8).map(|i| format!("IMG_{i:04}.ARW")).collect();
                let thumb = |i: usize| Thumb {
                    name: &names[i],
                    edited: false,
                    status: None,
                    selected: false,
                    picture: None,
                    no_preview: false,
                    rating: None,
                };
                actions.extend(Browser::new(8, Some(2), thumb).show(ui).action);
            },
            (false, Vec::new()),
        );
    harness.run();
    harness.get_by_label("IMG_0005.ARW").focus();
    harness.run();
    harness.state_mut().0 = true;
    harness.key_press_modifiers(Modifiers::CTRL | Modifiers::COMMAND, Key::Space);
    harness.run();
    assert_eq!(harness.state().1, []);
}

/// The viewer bar counts no photos: the browser bar does (`docs/gui.md`, The browser bar).
#[test]
fn the_viewer_bar_shows_no_count() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(760.0, 40.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui(|ui| {
            ViewerBar::new("Image", Zoom::Fit)
                .zoom_enabled(true)
                .show(ui);
        });
    harness.run();
    let toolbar = harness.get_by_role_and_label(Role::Toolbar, "Viewer bar");
    assert!(toolbar.query_by_role(Role::Label).is_none());
    let drawn: Vec<String> = (harness.output().shapes.iter())
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect();
    assert_eq!(drawn, ["Image", "Fit"]);
}

/// The viewer bar over the Browser, whose bar says "3/5", the three photos' cells ringed,
/// the middle one, active, thicker, in each theme.
#[test]
fn a_selection_is_photographed_in_every_theme() {
    let mut results = egui_kittest::SnapshotResults::new();
    for which in Builtin::ALL {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(760.0, 250.0))
            .with_context_setup(move |ctx| apertrust_ui::install(ctx, Theme::builtin(which)))
            .build_ui(|ui| {
                let panel = apertrust_ui::theme(ui.ctx()).colors.panel;
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                ViewerBar::new("Image", Zoom::Fit)
                    .zoom_enabled(true)
                    .show(ui);
                let names: Vec<String> =
                    (0..5).map(|i| format!("IMG_{:04}.ARW", 2041 + i)).collect();
                let thumb = |i: usize| Thumb {
                    name: &names[i],
                    edited: i == 2,
                    status: None,
                    selected: (1..=3).contains(&i),
                    picture: None,
                    no_preview: false,
                    rating: Some(CellRating::default()),
                };
                Browser::new(5, Some(2), thumb).count(3, 5).show(ui);
            });
        harness.run();
        harness.snapshot(format!("selection-{}", which.id()));
        results.extend(harness.take_snapshot_results());
    }
}
