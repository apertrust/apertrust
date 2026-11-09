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

//! The shell's parts: the top panel, the central bar, the grid panel and the side panel's
//! edge, driven through AccessKit, and photographed at a full window's width, where nothing
//! is compact (`docs/gui.md`, Testing).

use apertrust_ui::components::{
    Available, CentralBar, CentralBarAction, GridPanel, GridPanelAction, Margin, PanelEdge,
    PanelEdgeAction, PhotoCaption, PhotoFrame, SaveState, Thumb, Toggle, TopPanel, TopPanelAction,
    Zoom,
};
use apertrust_ui::{Builtin, Locale, Theme, Translator};
use egui::accesskit::{Action, Role};
use egui::{Color32, CursorIcon, Key, Modifiers, PointerButton, Pos2, Sense, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

fn install(ctx: &egui::Context) {
    apertrust_ui::install(ctx, Theme::default());
}

fn top_panel(available: Available) -> Harness<'static, Vec<TopPanelAction>> {
    Harness::builder()
        .with_size(egui::vec2(1200.0, 60.0))
        .with_context_setup(install)
        .build_ui_state(
            move |ui, actions: &mut Vec<TopPanelAction>| {
                if let Some(action) = TopPanel::new()
                    .available(available)
                    .saved(SaveState::Saved)
                    .show(ui)
                {
                    actions.push(action);
                }
            },
            Vec::new(),
        )
}

/// The common actions' names, in the panel's order.
const ACTIONS: [&str; 7] = [
    "Import…",
    "Export…",
    "Reset",
    "Undo",
    "Redo",
    "Copy edit",
    "Apply edit",
];

#[test]
fn the_top_panel_reports_its_actions() {
    let mut harness = top_panel(Available::default());
    harness.run();
    // The mark first, a button named for what it opens.
    harness
        .get_by_role_and_label(Role::Button, "About Apertrust")
        .click();
    harness.run();
    for name in ACTIONS {
        harness.get_by_label(name).click();
        harness.run();
    }
    assert_eq!(
        *harness.state(),
        vec![
            TopPanelAction::About,
            TopPanelAction::Import,
            TopPanelAction::Export,
            TopPanelAction::Reset,
            TopPanelAction::Undo,
            TopPanelAction::Redo,
            TopPanelAction::CopyEdit,
            TopPanelAction::ApplyEdit,
        ]
    );
    // No Settings button: Settings is in the menus and on its keys.
    assert!(harness.query_by_label("Settings").is_none());
}

#[test]
fn the_actions_come_first_in_order_and_saved_last() {
    let mut harness = top_panel(Available::default());
    harness.run();
    let rect = |name: &str| harness.get_by_label(name).rect();
    for pair in ACTIONS.windows(2) {
        assert!(rect(pair[0]).right() < rect(pair[1]).left(), "{pair:?}");
    }
    // The mark at the start, after the panel's margin, a button as large as the icon
    // buttons, 8 points before Import.
    let bar = harness.get_by_role(Role::Toolbar).rect();
    let mark = rect("About Apertrust");
    assert_eq!(mark.left() - bar.left(), 8.0);
    assert_eq!(mark.size(), vec2(30.0, 30.0));
    assert_eq!(rect("Import…").left() - mark.right(), 8.0);
    // Copy edit and Apply edit are a group of their own, 16 points on, with no line.
    assert_eq!(rect("Copy edit").left() - rect("Redo").right(), 16.0);
    assert_eq!(rect("Export…").left() - rect("Import…").right(), 2.0);
    assert_eq!(rect("Reset").left() - rect("Export…").right(), 2.0);
    // The empty strip that moves the window, then the end.
    assert!(rect("Apply edit").right() < bar.center().x);
    let saved = harness.get_by_role(Role::Status).rect();
    assert_eq!(saved.right(), bar.right() - 8.0);
}

#[test]
fn unavailable_actions_do_nothing() {
    let mut harness = top_panel(Available {
        undo: false,
        export: false,
        ..Available::default()
    });
    harness.run();
    harness.get_by_label("Undo").click();
    harness.get_by_label("Export…").click();
    harness.run();
    assert!(harness.state().is_empty());
}

/// The top panel `width` points wide, with Saved, and as on the Mac, the window's buttons'
/// room at its start, or as on Windows and Linux, the ☰ at its start and the caption
/// buttons at its end; and Export's keys.
fn narrow(
    width: f32,
    saved: SaveState,
    inset: f32,
    caption: bool,
) -> Harness<'static, Vec<TopPanelAction>> {
    narrow_in(Locale::EnGb, width, saved, inset, caption)
}

/// [`narrow`] in `locale`.
fn narrow_in(
    locale: Locale,
    width: f32,
    saved: SaveState,
    inset: f32,
    caption: bool,
) -> Harness<'static, Vec<TopPanelAction>> {
    Harness::builder()
        .with_size(egui::vec2(width, 120.0))
        .with_context_setup(move |ctx| {
            install(ctx);
            apertrust_ui::set_locale(ctx, locale);
        })
        .build_ui_state(
            move |ui, actions: &mut Vec<TopPanelAction>| {
                if let Some(action) = TopPanel::new()
                    .available(Available {
                        redo: false,
                        apply: false,
                        ..Available::default()
                    })
                    .saved(saved)
                    .start_inset(inset)
                    .title_bar(true)
                    .menu_button(caption)
                    .caption_buttons(caption)
                    .shortcut(TopPanelAction::Export, "Ctrl+E")
                    .show(ui)
                {
                    actions.push(action);
                }
            },
            Vec::new(),
        )
}

/// The common actions the panel shows, by name, and the width of each.
fn shown(harness: &Harness<'_, Vec<TopPanelAction>>) -> Vec<(&'static str, f32)> {
    ACTIONS
        .iter()
        .filter_map(|name| Some((*name, harness.query_by_label(name)?.rect().width())))
        .collect()
}

#[test]
fn a_narrowing_top_panel_folds_its_words_then_its_actions_into_more() {
    let mut wide = narrow(1200.0, SaveState::Saved, 0.0, false);
    wide.run();
    // Import, Export and Reset with their words; no ⋯.
    let widths = shown(&wide);
    assert_eq!(widths.len(), 7);
    assert!(
        widths[..3].iter().all(|(_, width)| *width > 30.0),
        "{widths:?}"
    );
    assert!(wide.query_by_label("More actions").is_none());

    // Narrower, the actions are icons.
    let mut icons = narrow(480.0, SaveState::Saved, 0.0, false);
    icons.run();
    let widths = shown(&icons);
    assert_eq!(widths.len(), 7);
    assert!(widths.iter().all(|(_, width)| *width == 30.0), "{widths:?}");
    assert!(icons.query_by_label("More actions").is_none());

    // Narrower still, all but Undo and Redo go under ⋯, listed with their keys and
    // disabled where they can't run.
    let mut more = narrow(300.0, SaveState::Saved, 0.0, false);
    more.run();
    let names: Vec<_> = shown(&more).into_iter().map(|(name, _)| name).collect();
    assert_eq!(names, ["Undo", "Redo"]);
    more.get_by_label("More actions").click();
    more.run();
    assert!(
        more.get_by_label("Apply edit")
            .accesskit_node()
            .is_disabled()
    );
    // Screen readers hear the name alone, and the keys as its shortcut.
    let export = more.get_by_label("Export…");
    assert_eq!(
        export.accesskit_node().data().keyboard_shortcut(),
        Some("Ctrl+E")
    );
    export.click();
    more.run();
    assert_eq!(*more.state(), vec![TopPanelAction::Export]);

    // At the narrowest, Undo and Redo join them.
    let mut least = narrow(150.0, SaveState::Saved, 0.0, false);
    least.run();
    assert!(shown(&least).is_empty());
    least.get_by_label("More actions").click();
    least.run();
    for name in ACTIONS {
        assert!(least.query_by_label(name).is_some(), "{name}");
    }
}

/// ⋯ is a pull-down of the popup service: each folded action an item with its keys as the
/// shortcut, disabled where it cannot run, the focus back on the button after Esc.
#[test]
fn the_more_menu_is_a_popup_list_with_keys_and_enabled_states() {
    let mut harness = narrow(300.0, SaveState::Saved, 0.0, false);
    harness.run();
    // Focused as a key left it, so the list can give the focus back.
    harness.get_by_label("More actions").focus();
    harness.run();
    harness.get_by_label("More actions").click();
    harness.run();
    let menu = harness.get_by_role(Role::Menu);
    assert_eq!(
        menu.accesskit_node().label().as_deref(),
        Some("More actions")
    );
    let export = harness.get_by_role_and_label(Role::MenuItem, "Export…");
    assert_eq!(
        export.accesskit_node().data().keyboard_shortcut(),
        Some("Ctrl+E")
    );
    assert!(!export.accesskit_node().is_disabled());
    let apply = harness.get_by_role_and_label(Role::MenuItem, "Apply edit");
    assert!(apply.accesskit_node().is_disabled());
    apply.click();
    harness.run();
    assert!(harness.state().is_empty(), "a disabled item is not chosen");
    // Esc closes it, the focus back on the button.
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.query_by_role(Role::Menu).is_none());
    assert_eq!(
        harness
            .get_by(|node| node.is_focused())
            .accesskit_node()
            .label()
            .as_deref(),
        Some("More actions")
    );
    // ↓ on the button opens it, Enter chooses the line with the focus.
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert!(harness.query_by_role(Role::Menu).is_some());
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.query_by_role(Role::Menu).is_none());
    assert_eq!(*harness.state(), vec![TopPanelAction::Import]);
}

/// Opened with the pointer, ⋯ gives the focus to its button as it closes, chosen or not:
/// a press leaves no focus to give back, so the button is the menu's opener.
#[test]
fn the_more_menu_opened_with_the_pointer_gives_the_focus_to_its_button() {
    let focused = |harness: &Harness<'_, Vec<TopPanelAction>>| {
        harness
            .get_by(|node| node.is_focused())
            .accesskit_node()
            .label()
    };
    let mut harness = narrow(300.0, SaveState::Saved, 0.0, false);
    harness.run();
    harness.get_by_label("More actions").click();
    harness.run();
    harness
        .get_by_role_and_label(Role::MenuItem, "Export…")
        .click();
    harness.run();
    assert_eq!(*harness.state(), vec![TopPanelAction::Export]);
    assert_eq!(focused(&harness).as_deref(), Some("More actions"));
    let mut harness = narrow(300.0, SaveState::Saved, 0.0, false);
    harness.run();
    harness.get_by_label("More actions").click();
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.query_by_role(Role::Menu).is_none());
    assert_eq!(focused(&harness).as_deref(), Some("More actions"));
}

#[test]
fn a_narrowing_top_panel_always_leaves_room_to_move_the_window() {
    // In every language; as on the Mac, with the traffic lights' room, and as on Linux,
    // with the caption buttons; with and without Saved; down to the window's minimum width
    // (`docs/gui.md`, The window frame and menus).
    for locale in Locale::ALL {
        let words = Translator::new(locale);
        let parts = [
            "top-panel-more",
            "action-import",
            "action-export",
            "action-reset",
            "action-undo",
            "action-redo",
            "action-copy-edit",
            "action-apply-edit",
        ]
        .map(|id| words.text(id));
        let minimise = words.text("window-minimise");
        for (inset, caption) in [(78.0, false), (0.0, true)] {
            for saved in [SaveState::None, SaveState::Saved, SaveState::Unsaved] {
                let mut harness = narrow_in(locale, 1400.0, saved, inset, caption);
                for width in (500..=1400).rev().step_by(5) {
                    harness.set_size(egui::vec2(width as f32, 120.0));
                    harness.run();
                    let last = parts
                        .iter()
                        .filter_map(|name| harness.query_by_label(name))
                        .map(|node| node.rect().right())
                        .fold(0.0, f32::max);
                    // The status at the end, where Not saved yet folded takes no room;
                    // without one, the end itself, before the caption buttons' gap or the
                    // margin.
                    let end = harness.query_by_role(Role::Status).map_or_else(
                        || {
                            if caption {
                                harness.get_by_label(&minimise).rect().left() - 16.0
                            } else {
                                width as f32 - 8.0
                            }
                        },
                        |status| status.rect().left(),
                    );
                    assert!(
                        end - last >= 24.0,
                        "{locale:?}, {width} points, {saved:?}, inset {inset}: {last} to {end}"
                    );
                }
            }
        }
    }
}

#[test]
fn saving_never_moves_the_fold() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 120.0))
        .with_context_setup(install)
        .build_ui_state(
            |ui, saved: &mut SaveState| {
                TopPanel::new().saved(*saved).show(ui);
            },
            SaveState::Saved,
        );
    // Whether Import has its words, whether ⋯ shows, and whether Undo is out of it.
    let fold = |h: &Harness<'_, SaveState>| {
        (
            h.query_by_label("Import…")
                .is_some_and(|node| node.rect().width() > 30.0),
            h.query_by_label("More actions").is_some(),
            h.query_by_label("Undo").is_some(),
        )
    };
    // An edit, then its save, at every width as the window narrows.
    for width in (150..=900).rev() {
        harness.set_size(egui::vec2(width as f32, 120.0));
        *harness.state_mut() = SaveState::Saved;
        harness.run();
        let saved = fold(&harness);
        for state in [SaveState::Unsaved, SaveState::Saved] {
            *harness.state_mut() = state;
            harness.run();
            assert_eq!(fold(&harness), saved, "{width} points, {state:?}");
        }
    }
}

#[test]
fn screen_readers_hear_whether_the_edit_is_saved() {
    for locale in Locale::ALL {
        let words = Translator::new(locale);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1200.0, 120.0))
            .with_context_setup(move |ctx| {
                install(ctx);
                apertrust_ui::set_locale(ctx, locale);
            })
            .build_ui_state(
                |ui, saved: &mut SaveState| {
                    TopPanel::new().saved(*saved).show(ui);
                },
                SaveState::Saved,
            );
        harness.run();
        // With the words shown, and at the narrowest, folded to Saved's tick or to nothing.
        for (width, folded) in [(1200.0, false), (150.0, true)] {
            harness.set_size(egui::vec2(width, 120.0));
            for (state, id) in [
                (SaveState::Unsaved, "status-unsaved"),
                (SaveState::Saved, "status-saved"),
            ] {
                // Heard on the frame the status changes.
                *harness.state_mut() = state;
                harness.step();
                let status = harness.get_by_role(Role::Status);
                let node = status.accesskit_node();
                let name = words.text(id);
                assert_eq!(node.label(), Some(name), "{locale:?}, {width}");
                // The tooltip says only that, so it has no value to be read twice.
                assert_eq!(node.value(), None, "{locale:?}, {width}");
                // Where it is drawn: its word and Saved's tick, the tick alone, or nothing.
                let drawn = status.rect().width();
                match (state, folded) {
                    (SaveState::Saved, true) => assert_eq!(drawn, 12.0, "{locale:?}"),
                    (_, true) => assert_eq!(drawn, 0.0, "{locale:?}"),
                    _ => assert!(drawn > 12.0, "{locale:?}, {state:?}: {drawn}"),
                }
            }
        }
        // No photo, no status.
        *harness.state_mut() = SaveState::None;
        harness.step();
        assert!(harness.query_by_role(Role::Status).is_none());
    }
}

#[test]
fn a_widening_top_panel_unfolds_only_with_room_to_spare() {
    let mut width = 700.0;
    let mut harness = narrow(width, SaveState::Saved, 0.0, false);
    harness.run();
    let words =
        |h: &Harness<'_, Vec<TopPanelAction>>| h.get_by_label("Import…").rect().width() > 30.0;
    // Narrowed a point at a time until the words go.
    while words(&harness) {
        width -= 1.0;
        harness.set_size(egui::vec2(width, 120.0));
        harness.run();
    }
    // Widened by less than 16 points, the words stay folded; by 16 more, they come back.
    harness.set_size(egui::vec2(width + 10.0, 120.0));
    harness.run();
    assert!(!words(&harness));
    harness.set_size(egui::vec2(width + 18.0, 120.0));
    harness.run();
    assert!(words(&harness));
}

#[test]
fn tooltips_carry_the_keys_and_screen_readers_hear_them() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1200.0, 120.0))
        .with_context_setup(|ctx| {
            install(ctx);
            ctx.global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
        })
        .build_ui(|ui| {
            TopPanel::new()
                .shortcut(TopPanelAction::CopyEdit, "Ctrl+Shift+C")
                .shortcut(TopPanelAction::Export, "Ctrl+E")
                .show(ui);
        });
    harness.run();
    for (name, keys, tip) in [
        ("Copy edit", "Ctrl+Shift+C", "Copy edit (Ctrl+Shift+C)"),
        ("Export…", "Ctrl+E", "Export… (Ctrl+E)"),
    ] {
        let button = harness.get_by_role_and_label(Role::Button, name);
        assert_eq!(
            button.accesskit_node().data().keyboard_shortcut(),
            Some(keys)
        );
        button.hover();
        harness.run();
        assert!(harness.query_by_label(tip).is_some(), "{tip}");
    }
    // Without keys, a tooltip is the name alone.
    harness.get_by_label("Undo").hover();
    harness.run();
    assert!(harness.query_by_label_contains("Undo (").is_none());
    // The mark has no tooltip: hovered, only its own name says About Apertrust.
    harness
        .get_by_role_and_label(Role::Button, "About Apertrust")
        .hover();
    harness.run();
    assert_eq!(harness.query_all_by_label("About Apertrust").count(), 1);
}

/// The top panel as the window's title bar, or not, with room for the window's buttons at
/// its start. At 60 frames a second, since each event takes a frame of its own and a
/// double-click's second click must come within 0.3 s of its first.
fn title_bar(on: bool, start: f32) -> Harness<'static, Vec<TopPanelAction>> {
    title_bar_with(on, start, Available::default())
}

/// The title bar with only the actions `available` can run.
fn title_bar_with(
    on: bool,
    start: f32,
    available: Available,
) -> Harness<'static, Vec<TopPanelAction>> {
    Harness::builder()
        .with_size(egui::vec2(1200.0, 120.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(install)
        .build_ui_state(
            move |ui, actions: &mut Vec<TopPanelAction>| {
                if let Some(action) = TopPanel::new()
                    .available(available)
                    .saved(SaveState::Saved)
                    .start_inset(start)
                    .title_bar(on)
                    .show(ui)
                {
                    actions.push(action);
                }
            },
            Vec::new(),
        )
}

/// An empty point of the panel: after the last action at its start, before Saved or, with
/// no status, the panel's end.
fn empty_point<T>(harness: &Harness<'_, T>) -> egui::Pos2 {
    let apply = harness.get_by_label("Apply edit").rect();
    let end = harness.query_by_role(Role::Status).map_or_else(
        || harness.get_by_role(Role::Toolbar).rect().right(),
        |saved| saved.rect().left(),
    );
    egui::pos2((apply.right() + end) / 2.0, apply.center().y)
}

fn press(harness: &Harness<'_, Vec<TopPanelAction>>, pos: egui::Pos2, pressed: bool) {
    harness.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    });
}

/// Two clicks at `pos`, half a second after anything before, which would otherwise make
/// them a triple click.
fn double_click(harness: &mut Harness<'_, Vec<TopPanelAction>>, pos: egui::Pos2) {
    harness.run_steps(30);
    harness.hover_at(pos);
    for pressed in [true, false, true, false] {
        press(harness, pos, pressed);
    }
    harness.run();
}

/// Presses at `from`, moves 30 points across a frame at a time, and lets go.
fn drag(harness: &mut Harness<'_, Vec<TopPanelAction>>, from: egui::Pos2) {
    harness.hover_at(from);
    press(harness, from, true);
    harness.step();
    for step in 1..=3 {
        harness.hover_at(from + egui::vec2(10.0 * step as f32, 0.0));
        harness.step();
    }
    harness.drop_at(from + egui::vec2(30.0, 0.0));
    harness.run();
}

#[test]
fn the_top_panel_keeps_room_for_the_window_s_buttons() {
    let mut plain = title_bar(false, 0.0);
    plain.run();
    let mut inset = title_bar(true, 90.0);
    inset.run();
    let first = |h: &Harness<'_, Vec<TopPanelAction>>| h.get_by_label("Import…").rect().left();
    let last = |h: &Harness<'_, Vec<TopPanelAction>>| h.get_by_role(Role::Status).rect().right();
    // The room takes the place of the panel's margin at its start, 8 points; the end stays.
    assert_eq!(first(&inset) - first(&plain), 90.0 - 8.0);
    assert!(first(&inset) >= 90.0);
    assert_eq!(last(&inset), last(&plain));
}

#[test]
fn the_title_bar_moves_and_zooms_the_window_from_its_empty_areas_only() {
    let mut harness = title_bar(true, 90.0);
    harness.run();
    let empty = empty_point(&harness);
    double_click(&mut harness, empty);
    assert_eq!(*harness.state(), vec![TopPanelAction::TitleBarDoubleClick]);
    // A drag is reported once, however far it goes.
    harness.state_mut().clear();
    drag(&mut harness, empty);
    assert_eq!(*harness.state(), vec![TopPanelAction::MoveWindow]);

    // On a button, a double-click is two clicks, and a drag neither clicks nor moves. The
    // mark is a button too, after the window's buttons' room.
    harness.state_mut().clear();
    let mark = harness.get_by_label("About Apertrust").rect();
    let bar = harness.get_by_role(Role::Toolbar).rect();
    assert_eq!(mark.left() - bar.left(), 90.0);
    double_click(&mut harness, mark.center());
    assert_eq!(*harness.state(), vec![TopPanelAction::About; 2]);
    harness.state_mut().clear();
    let reset = harness.get_by_label("Reset").rect().center();
    double_click(&mut harness, reset);
    assert_eq!(*harness.state(), vec![TopPanelAction::Reset; 2]);
    harness.state_mut().clear();
    let undo = harness.get_by_label("Undo").rect().center();
    drag(&mut harness, undo);
    let apply = harness.get_by_label("Apply edit").rect();
    drag(&mut harness, apply.center());
    drag(&mut harness, mark.center());
    assert!(harness.state().is_empty());
}

#[test]
fn a_top_panel_that_is_not_the_title_bar_ignores_its_empty_areas() {
    let mut harness = title_bar(false, 0.0);
    harness.run();
    let empty = empty_point(&harness);
    double_click(&mut harness, empty);
    drag(&mut harness, empty);
    assert!(harness.state().is_empty());
}

#[test]
fn a_disabled_button_and_the_margin_round_a_button_are_no_empty_areas() {
    let available = Available {
        undo: false,
        ..Available::default()
    };
    let mut harness = title_bar_with(true, 90.0, available);
    harness.run();
    let undo = harness.get_by_label("Undo").rect().center();
    double_click(&mut harness, undo);
    drag(&mut harness, undo);
    assert!(harness.state().is_empty());
    // Just off a button, within egui's margin, a press is still the button's.
    let reset = harness.get_by_label("Reset").rect();
    let near = egui::pos2(reset.center().x, reset.bottom() + 3.0);
    double_click(&mut harness, near);
    assert_eq!(*harness.state(), vec![TopPanelAction::Reset; 2]);
    harness.state_mut().clear();
    drag(&mut harness, near);
    assert!(harness.state().is_empty());
}

#[test]
fn a_new_press_moves_the_window_again_when_the_last_release_never_came() {
    let mut harness = title_bar(true, 90.0);
    harness.run();
    let empty = empty_point(&harness);
    // The system's window drag may swallow the release.
    for _ in 0..2 {
        harness.hover_at(empty);
        press(&harness, empty, true);
        harness.step();
        harness.hover_at(empty + egui::vec2(0.0, 10.0));
        harness.step();
    }
    assert_eq!(*harness.state(), vec![TopPanelAction::MoveWindow; 2]);
}

/// A right-click at `pos`, as a press and a release.
fn right_click(harness: &mut Harness<'_, Vec<TopPanelAction>>, pos: egui::Pos2) {
    harness.run_steps(30);
    harness.hover_at(pos);
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos,
            button: PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    harness.run();
}

#[test]
fn a_right_click_on_the_empty_areas_asks_for_the_window_s_menu() {
    let mut harness = title_bar(true, 0.0);
    harness.run();
    let empty = empty_point(&harness);
    right_click(&mut harness, empty);
    assert_eq!(*harness.state(), vec![TopPanelAction::WindowMenu]);
    // On a control, enabled or not, it is not the title bar's, and it clicks nothing.
    harness.state_mut().clear();
    for name in ["About Apertrust", "Reset", "Undo", "Apply edit"] {
        let control = harness.get_by_label(name).rect().center();
        right_click(&mut harness, control);
    }
    assert!(harness.state().is_empty());
    // Nor where the panel is not the title bar.
    let mut plain = title_bar(false, 0.0);
    plain.run();
    let empty = empty_point(&plain);
    right_click(&mut plain, empty);
    assert!(plain.state().is_empty());
}

/// The title bar with the window's caption buttons, the window maximised or not.
fn caption_buttons(maximised: bool) -> Harness<'static, Vec<TopPanelAction>> {
    Harness::builder()
        .with_size(egui::vec2(1200.0, 120.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(install)
        .build_ui_state(
            move |ui, actions: &mut Vec<TopPanelAction>| {
                if let Some(action) = TopPanel::new()
                    .saved(SaveState::Saved)
                    .title_bar(true)
                    .caption_buttons(true)
                    .maximised(maximised)
                    .show(ui)
                {
                    actions.push(action);
                }
            },
            Vec::new(),
        )
}

#[test]
fn the_caption_buttons_report_their_clicks() {
    let mut harness = caption_buttons(false);
    harness.run();
    // Each as tall as the panel and 46 points wide, the three at its very end, close last.
    let bar = harness.get_by_role(Role::Toolbar).rect();
    let close = harness.get_by_label("Close").rect();
    let minimise = harness.get_by_label("Minimise").rect();
    assert_eq!(close.right(), bar.right());
    assert_eq!(close.right() - minimise.left(), 3.0 * 46.0);
    assert_eq!(close.height(), bar.height());
    // A group's gap before them.
    assert_eq!(
        minimise.left() - harness.get_by_role(Role::Status).rect().right(),
        16.0
    );
    for name in ["Minimise", "Maximise", "Close"] {
        harness.get_by_role_and_label(Role::Button, name).click();
        harness.run();
    }
    assert_eq!(
        *harness.state(),
        vec![
            TopPanelAction::Minimise,
            TopPanelAction::MaximiseOrRestore,
            TopPanelAction::Close,
        ]
    );
    // They are no empty areas: a double-click on one is two clicks, never a zoom.
    harness.state_mut().clear();
    double_click(&mut harness, minimise.center());
    assert_eq!(*harness.state(), vec![TopPanelAction::Minimise; 2]);
    harness.state_mut().clear();
    right_click(&mut harness, close.center());
    assert!(harness.state().is_empty());
}

#[test]
fn a_maximised_window_s_middle_button_restores() {
    let mut harness = caption_buttons(true);
    harness.run();
    assert!(harness.query_by_label("Maximise").is_none());
    harness
        .get_by_role_and_label(Role::Button, "Restore")
        .click();
    harness.run();
    assert_eq!(*harness.state(), vec![TopPanelAction::MaximiseOrRestore]);
    let mut normal = caption_buttons(false);
    normal.run();
    assert!(normal.query_by_label("Restore").is_none());
    assert!(normal.query_by_label("Maximise").is_some());
}

#[test]
fn without_the_option_there_are_no_caption_buttons() {
    let mut harness = title_bar(true, 0.0);
    harness.run();
    for name in ["Minimise", "Maximise", "Restore", "Close"] {
        assert!(harness.query_by_label(name).is_none(), "{name}");
    }
    // Saved ends the panel, before its margin, as before.
    let bar = harness.get_by_role(Role::Toolbar).rect();
    let saved = harness.get_by_role(Role::Status).rect();
    assert_eq!(saved.right(), bar.right() - 8.0);
}

/// The end of a top panel with caption buttons, in the dark theme, for its snapshots.
fn caption_snapshot() -> Harness<'static> {
    Harness::builder()
        .with_size(egui::vec2(700.0, 90.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark)))
        .build_ui(|ui| {
            let panel = apertrust_ui::theme(ui.ctx()).colors.panel;
            ui.painter()
                .rect_filled(ui.ctx().content_rect(), 0.0, panel);
            TopPanel::new()
                .title_bar(true)
                .caption_buttons(true)
                .show(ui);
        })
}

#[test]
fn the_close_button_under_the_pointer() {
    let mut harness = caption_snapshot();
    harness.run();
    // Off the icon, so the snapshot shows it.
    let close = harness.get_by_label("Close").rect();
    harness.hover_at(close.left_bottom() + egui::vec2(6.0, -6.0));
    harness.run();
    harness.snapshot("caption-buttons-close-hover");
}

#[test]
fn the_minimise_button_focused_by_the_keyboard() {
    let mut harness = caption_snapshot();
    harness.run();
    harness.get_by_label("Minimise").focus();
    harness.run();
    assert!(harness.get_by_label("Minimise").is_focused());
    harness.snapshot("caption-buttons-minimise-focused");
}

/// The title bar with, when `cover` is set, a popup drawn over that point of it.
struct Covered {
    actions: Vec<TopPanelAction>,
    cover: Option<egui::Pos2>,
}

#[test]
fn a_popup_over_the_bar_takes_its_presses() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1200.0, 120.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(install)
        .build_ui_state(
            |ui, state: &mut Covered| {
                if let Some(action) = TopPanel::new().title_bar(true).show(ui) {
                    state.actions.push(action);
                }
                if let Some(at) = state.cover {
                    egui::Area::new(egui::Id::unique("popup"))
                        .order(egui::Order::Foreground)
                        .fixed_pos(at - egui::vec2(30.0, 10.0))
                        .show(ui.ctx(), |ui| {
                            ui.allocate_exact_size(egui::vec2(60.0, 40.0), egui::Sense::hover());
                        });
                }
            },
            Covered {
                actions: Vec::new(),
                cover: None,
            },
        );
    harness.run();
    let empty = empty_point(&harness);
    harness.state_mut().cover = Some(empty);
    harness.run();
    harness.run_steps(30);
    harness.hover_at(empty);
    for pressed in [true, false, true, false] {
        harness.event(egui::Event::PointerButton {
            pos: empty,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    harness.run();
    assert!(harness.state().actions.is_empty());
    // Without it, the same double-click is the title bar's.
    harness.state_mut().cover = None;
    harness.run_steps(30);
    harness.hover_at(empty);
    for pressed in [true, false, true, false] {
        harness.event(egui::Event::PointerButton {
            pos: empty,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    harness.run();
    assert_eq!(
        harness.state().actions,
        vec![TopPanelAction::TitleBarDoubleClick]
    );
}

/// The central bar with its state applied as the app would.
struct Photo {
    guides: bool,
    zoom: Zoom,
    zoomable: bool,
    actions: Vec<CentralBarAction>,
}

fn central_bar() -> Harness<'static, Photo> {
    central_bar_in(Theme::default(), egui::vec2(1000.0, 420.0), true)
}

fn central_bar_in(theme: Theme, size: egui::Vec2, zoomable: bool) -> Harness<'static, Photo> {
    Harness::builder()
        .with_size(size)
        .with_context_setup(move |ctx| apertrust_ui::install(ctx, theme.clone()))
        .build_ui_state(
            |ui, state: &mut Photo| {
                let panel = apertrust_ui::theme(ui.ctx()).colors.panel;
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                if let Some(action) = CentralBar::new("Image", state.zoom)
                    .toggles(
                        Toggle::new(false, false),
                        Toggle::new(false, true),
                        Toggle::new(false, true),
                        Toggle::new(state.guides, true),
                    )
                    .values(Some([142, 128, 113]))
                    .zoom_enabled(true)
                    .zoomable(state.zoomable)
                    .shortcut(CentralBarAction::Zoom(Zoom::Fit), "Ctrl+0")
                    .shortcut(CentralBarAction::Zoom(Zoom::Percent(100.0)), "Z")
                    .shortcut(CentralBarAction::ZoomIn, "Ctrl+=")
                    .shortcut(CentralBarAction::ZoomOut, "Ctrl+−")
                    .show(ui)
                {
                    match action {
                        CentralBarAction::Guides => state.guides = !state.guides,
                        CentralBarAction::Zoom(zoom) => state.zoom = zoom,
                        _ => {}
                    }
                    state.actions.push(action);
                }
            },
            Photo {
                guides: false,
                zoom: Zoom::Fit,
                zoomable,
                actions: Vec::new(),
            },
        )
}

/// The zoom menu's line named `name`: whether it can be chosen, and whether it is checked,
/// None for a command.
fn zoom_line(harness: &Harness<'_, Photo>, name: &str) -> (bool, Option<bool>) {
    let node = harness
        .get_by(|node| {
            matches!(node.role(), Role::MenuItem | Role::MenuItemCheckBox)
                && node.label().as_deref() == Some(name)
        })
        .accesskit_node();
    let checked = node.toggled().map(|t| t == egui::accesskit::Toggled::True);
    (!node.is_disabled(), checked)
}

/// The zoom menu's lines in order.
fn zoom_lines(harness: &Harness<'_, Photo>) -> Vec<String> {
    harness
        .get_by_role_and_label(Role::Menu, "Zoom")
        .query_all_by(|node| matches!(node.role(), Role::MenuItem | Role::MenuItemCheckBox))
        .map(|node| node.accesskit_node().label().unwrap_or_default())
        .collect()
}

#[test]
fn the_central_bar_toggles_and_shows_the_values() {
    let mut harness = central_bar();
    harness.run();
    harness.get_by_label("Guides").click();
    harness.run();
    assert!(harness.state().guides);
    // Disabled: one photo, nothing to show side by side.
    harness.get_by_label("Show all selected photos").click();
    harness.run();
    assert_eq!(harness.state().actions, vec![CentralBarAction::Guides]);
    assert_eq!(
        harness.get_by_label("Values under the cursor").value(),
        Some("Red 142, green 128, blue 113, grey 130".to_owned())
    );
}

#[test]
fn the_zoom_is_one_button_whose_menu_is_darktable_s_list_then_the_steps() {
    let mut harness = central_bar();
    harness.run();
    let button = harness.get_by_role_and_label(Role::Button, "Zoom");
    let node = button.accesskit_node();
    assert_eq!(node.value().as_deref(), Some("Fit"));
    assert_eq!(
        node.data().has_popup(),
        Some(egui::accesskit::HasPopup::Menu)
    );
    // At the bar's end, as wide as the wireframe draws it, in the bar's height.
    let rect = button.rect();
    assert_eq!(rect.right(), 1000.0 - 16.0);
    assert!((64.0..=76.0).contains(&rect.width()), "{rect:?}");
    assert_eq!(rect.height(), 28.0);
    // The slider and its − and + are gone.
    assert!(harness.query_by_role(Role::Slider).is_none());
    button.click();
    harness.run();
    assert_eq!(
        zoom_lines(&harness),
        [
            "Small", "Fit", "Fill", "50%", "100%", "200%", "400%", "800%", "1600%", "Zoom in",
            "Zoom out"
        ]
    );
    assert_eq!(zoom_line(&harness, "Fit"), (true, Some(true)));
    assert_eq!(zoom_line(&harness, "100%"), (true, Some(false)));
    assert_eq!(zoom_line(&harness, "Zoom in"), (true, None));
    // Each entry with its keys where it has them.
    let keys = |name: &str| {
        harness
            .get_by(|node| {
                node.role() == Role::MenuItemCheckBox && node.label().as_deref() == Some(name)
            })
            .accesskit_node()
            .data()
            .keyboard_shortcut()
            .map(str::to_owned)
    };
    assert_eq!(keys("Fit").as_deref(), Some("Ctrl+0"));
    assert_eq!(keys("100%").as_deref(), Some("Z"));
    assert_eq!(keys("Small"), None);
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "100%")
        .click();
    harness.run();
    assert_eq!(harness.state().zoom, Zoom::Percent(100.0));
    let button = harness.get_by_role_and_label(Role::Button, "Zoom");
    assert_eq!(button.accesskit_node().value().as_deref(), Some("100%"));
    // Fill, and the steps, which the caller takes along the list.
    button.click();
    harness.run();
    assert_eq!(zoom_line(&harness, "100%"), (true, Some(true)));
    assert_eq!(zoom_line(&harness, "Fit"), (true, Some(false)));
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "Fill")
        .click();
    harness.run();
    assert_eq!(harness.state().zoom, Zoom::Fill);
    harness.get_by_role_and_label(Role::Button, "Zoom").click();
    harness.run();
    harness
        .get_by_role_and_label(Role::MenuItem, "Zoom in")
        .click();
    harness.run();
    assert_eq!(
        harness.state().actions,
        [
            CentralBarAction::Zoom(Zoom::Percent(100.0)),
            CentralBarAction::Zoom(Zoom::Fill),
            CentralBarAction::ZoomIn,
        ]
    );
}

#[test]
fn an_in_between_scale_shows_rounded_and_checks_nothing() {
    let mut harness = central_bar();
    harness.state_mut().zoom = Zoom::Percent(137.4);
    harness.run();
    let button = harness.get_by_role_and_label(Role::Button, "Zoom");
    assert_eq!(button.accesskit_node().value().as_deref(), Some("137%"));
    button.click();
    harness.run();
    for name in zoom_lines(&harness) {
        let (_, checked) = zoom_line(&harness, &name);
        assert_ne!(checked, Some(true), "{name}");
    }
    // At either end, the step that has nowhere to go is disabled.
    harness.key_press(Key::Escape);
    harness.state_mut().zoom = Zoom::Percent(1600.0);
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Zoom").click();
    harness.run();
    assert_eq!(zoom_line(&harness, "Zoom in"), (false, None));
    assert_eq!(zoom_line(&harness, "Zoom out"), (true, None));
}

#[test]
fn while_the_viewport_only_fits_every_entry_but_fit_is_disabled() {
    let mut harness = central_bar_in(Theme::default(), egui::vec2(1000.0, 420.0), false);
    harness.run();
    let button = harness.get_by_role_and_label(Role::Button, "Zoom");
    assert_eq!(button.accesskit_node().value().as_deref(), Some("Fit"));
    button.hover();
    harness.run();
    assert!(
        harness
            .query_by_label("Zoom comes with the 100% view")
            .is_some()
    );
    harness.get_by_role_and_label(Role::Button, "Zoom").click();
    harness.run();
    for name in zoom_lines(&harness) {
        let (enabled, _) = zoom_line(&harness, &name);
        assert_eq!(enabled, name == "Fit", "{name}");
    }
    // A disabled entry chooses nothing; Fit does, which it already is.
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "100%")
        .click();
    harness.run();
    assert!(harness.state().actions.is_empty());
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "Fit")
        .click();
    harness.run();
    assert_eq!(harness.state().actions, [CentralBarAction::Zoom(Zoom::Fit)]);
}

#[test]
fn the_zoom_s_tooltip_says_how_to_zoom_and_at_100_what_it_shows() {
    let mut harness = central_bar();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Zoom").hover();
    harness.run();
    let tip = "Zoom: pinch or Ctrl+scroll; Z for 100%. Scroll moves a zoomed photo, \
               Shift+scroll sideways";
    assert!(harness.query_by_label(tip).is_some());
    harness.state_mut().zoom = Zoom::Percent(100.0);
    harness.run();
    let exact = format!("At 100% the screen shows exactly what the export will be. {tip}");
    assert!(harness.query_by_label(&exact).is_some());
}

#[test]
fn the_zoom_menu_opens_and_closes_by_keys_and_gives_the_focus_back() {
    let mut harness = central_bar();
    harness.run();
    let menu_open = |harness: &Harness<'_, Photo>| {
        harness
            .query_by_role_and_label(Role::Menu, "Zoom")
            .is_some()
    };
    for opener in [Key::Enter, Key::Space, Key::ArrowDown] {
        harness.get_by_role_and_label(Role::Button, "Zoom").focus();
        harness.run();
        harness.key_press(opener);
        harness.run();
        assert!(menu_open(&harness), "{opener:?}");
        harness.key_press(Key::Escape);
        harness.run();
        assert!(!menu_open(&harness), "{opener:?}");
        assert!(
            harness
                .get_by_role_and_label(Role::Button, "Zoom")
                .is_focused(),
            "{opener:?}"
        );
    }
    // The arrows move through the lines, and Enter chooses one.
    harness.key_press(Key::ArrowDown);
    harness.run();
    let focused = |harness: &Harness<'_, Photo>| {
        harness
            .get_by(|node| node.is_focused())
            .accesskit_node()
            .label()
            .unwrap_or_default()
    };
    let first = focused(&harness);
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_ne!(focused(&harness), first);
    let chosen = focused(&harness);
    harness.key_press(Key::Enter);
    harness.run();
    assert!(!menu_open(&harness));
    let button = harness.get_by_role_and_label(Role::Button, "Zoom");
    assert_eq!(button.accesskit_node().value(), Some(chosen));
}

/// The zoom's button at rest and its menu open, at 100%, in `which` theme.
fn photograph_the_zoom(which: Builtin, name: &str) {
    let size = egui::vec2(420.0, 400.0);
    let mut harness = central_bar_in(Theme::builtin(which), size, true);
    harness.state_mut().zoom = Zoom::Percent(100.0);
    harness.run();
    harness.snapshot(format!("central-bar-zoom-{name}"));
    harness.get_by_role_and_label(Role::Button, "Zoom").click();
    harness.run();
    harness.snapshot(format!("central-bar-zoom-menu-{name}"));
}

#[test]
fn the_zoom_is_photographed_in_the_dark_theme() {
    photograph_the_zoom(Builtin::Dark, "dark");
}

#[test]
fn the_zoom_is_photographed_in_the_light_theme() {
    photograph_the_zoom(Builtin::Light, "light");
}

fn grid_panel(count: usize) -> Harness<'static, Vec<GridPanelAction>> {
    Harness::builder()
        .with_size(egui::vec2(800.0, 200.0))
        .with_context_setup(install)
        .build_ui_state(
            move |ui, actions: &mut Vec<GridPanelAction>| {
                let names: Vec<String> = (0..count).map(|i| format!("IMG_{i:04}.ARW")).collect();
                let thumb = |i: usize| Thumb {
                    name: &names[i],
                    edited: false,
                    status: None,
                    selected: false,
                    picture: None,
                    rating: None,
                };
                if let Some(action) = GridPanel::new(count, Some(0), thumb).show(ui).action {
                    actions.push(action);
                }
            },
            Vec::new(),
        )
}

#[test]
fn the_grid_panel_opens_and_selects_with_modifiers() {
    let mut harness = grid_panel(20);
    harness.run();
    harness.get_by_label("IMG_0003.ARW").click();
    harness.run();
    harness
        .get_by_label("IMG_0005.ARW")
        .click_modifiers(Modifiers::SHIFT);
    harness.run();
    harness
        .get_by_label("IMG_0002.ARW")
        .click_modifiers(Modifiers::COMMAND);
    harness.run();
    assert_eq!(
        *harness.state(),
        vec![
            GridPanelAction::Open(3),
            GridPanelAction::SelectRange(5),
            GridPanelAction::ToggleSelected(2),
        ]
    );
}

#[test]
fn the_grid_panel_draws_only_what_is_visible() {
    let mut harness = grid_panel(10_000);
    harness.run();
    assert!(harness.query_by_label("IMG_0001.ARW").is_some());
    assert!(harness.query_by_label("IMG_9000.ARW").is_none());
}

/// A panel beside the photo in a window 1000 points wide, with what its edge reported.
struct Panel {
    width: f32,
    at_start: bool,
    actions: Vec<PanelEdgeAction>,
}

/// The panel's own width, which a double-click on its edge goes back to.
const OWN: f32 = 400.0;

/// A panel `width` wide at the window's end, or its start, beside the photo, laid out as
/// the app lays out the side panel: at the width its edge gives, the edge shown last.
fn panel(width: f32, at_start: bool) -> Harness<'static, Panel> {
    Harness::builder()
        .with_size(vec2(1000.0, 300.0))
        // Frames a sixtieth of a second apart, so two clicks make a double-click (0.3 s).
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(install)
        .build_ui_state(
            |ui, state: &mut Panel| {
                let edge = PanelEdge::new("Side panel width", state.width, 320.0..=600.0)
                    .at_start(state.at_start);
                let side = if state.at_start {
                    egui::Panel::left("panel")
                } else {
                    egui::Panel::right("panel")
                };
                let panel = side
                    .exact_size(edge.width(ui.ctx()))
                    .resizable(false)
                    .show(ui, |_| {});
                egui::CentralPanel::default().show(ui, |_| {});
                if let Some(action) = edge.show(ui, panel.response.rect) {
                    state.width = match action {
                        PanelEdgeAction::Resize(width) => width,
                        PanelEdgeAction::Reset => OWN,
                    };
                    state.actions.push(action);
                }
            },
            Panel {
                width,
                at_start,
                actions: Vec::new(),
            },
        )
}

/// Where the edge is: the middle of the splitter screen readers find.
fn edge(harness: &Harness<'_, Panel>) -> Pos2 {
    harness
        .get_by_role_and_label(Role::Splitter, "Side panel width")
        .rect()
        .center()
}

/// Presses at `from`, drags to `to` and lets go, each in frames of its own.
fn drag_panel(harness: &mut Harness<'_, Panel>, from: Pos2, to: Pos2) {
    harness.hover_at(from);
    harness.run();
    harness.drag_at(from);
    harness.run();
    harness.hover_at(to);
    harness.run();
    harness.drop_at(to);
    harness.run();
}

/// Drags the edge `by` points along the window.
fn drag_edge(harness: &mut Harness<'_, Panel>, by: f32) {
    let from = edge(harness);
    drag_panel(harness, from, from + vec2(by, 0.0));
}

#[test]
fn dragging_the_panel_edge_resizes_the_panel_within_its_widths() {
    let mut harness = panel(OWN, false);
    harness.run();
    // At the window's end, the edge is the panel's start: the panel's fixed edge, less its
    // width.
    let end = edge(&harness).x + OWN;
    drag_edge(&mut harness, -100.0);
    assert_eq!(harness.state().width, 500.0);
    assert_eq!(edge(&harness).x, end - 500.0);
    // Beyond its widths it stops at their ends.
    drag_edge(&mut harness, -400.0);
    assert_eq!(harness.state().width, 600.0);
    drag_edge(&mut harness, 600.0);
    assert_eq!(harness.state().width, 320.0);
    assert_eq!(edge(&harness).x, end - 320.0);
    // Taken hold of a little off the line, it moves as far as the pointer, never jumping
    // to it.
    let from = edge(&harness) + vec2(2.0, 0.0);
    drag_panel(&mut harness, from, from - vec2(30.0, 0.0));
    assert_eq!(harness.state().width, 350.0);
    // The panel follows the pointer in the frame it moves, not a frame later.
    let from = edge(&harness);
    harness.hover_at(from);
    harness.run();
    harness.drag_at(from);
    harness.run();
    harness.hover_at(from - vec2(40.0, 0.0));
    harness.step();
    assert_eq!(edge(&harness).x, from.x - 40.0);
    // Screen readers hear the width as dragged, not as it was.
    assert_eq!(
        harness
            .get_by_role_and_label(Role::Splitter, "Side panel width")
            .accesskit_node()
            .numeric_value(),
        Some(390.0)
    );
    harness.drop_at(from - vec2(40.0, 0.0));
    harness.run();
    assert_eq!(harness.state().width, 390.0);
    // A drag leaves the focus where it was.
    assert!(
        !harness
            .get_by_role_and_label(Role::Splitter, "Side panel width")
            .is_focused()
    );

    // At the window's start, the edge is the panel's end, and widens it the other way.
    let mut harness = panel(OWN, true);
    harness.run();
    let start = edge(&harness).x - OWN;
    drag_edge(&mut harness, 100.0);
    assert_eq!(harness.state().width, 500.0);
    assert_eq!(edge(&harness).x, start + 500.0);
}

#[test]
fn a_double_click_on_the_panel_edge_resets_the_panel() {
    let mut harness = panel(520.0, false);
    harness.run();
    let end = edge(&harness).x + 520.0;
    for _ in 0..2 {
        harness
            .get_by_role_and_label(Role::Splitter, "Side panel width")
            .click();
        // Each queued event runs in a frame of its own; the last is the release.
        harness.step();
    }
    let double = harness
        .ctx
        .input(|i| i.pointer.button_double_clicked(PointerButton::Primary));
    harness.run();
    assert!(double);
    assert_eq!(harness.state().actions, vec![PanelEdgeAction::Reset]);
    assert_eq!(edge(&harness).x, end - OWN);
}

#[test]
fn the_panel_edge_shows_the_resize_cursor_and_steps_for_screen_readers() {
    let mut harness = panel(OWN, false);
    harness.run();
    harness.hover_at(edge(&harness));
    harness.run();
    assert_eq!(
        harness.output().platform_output.cursor_icon,
        CursorIcon::ResizeColumn
    );
    let splitter = harness.get_by_role_and_label(Role::Splitter, "Side panel width");
    assert_eq!(
        splitter.accesskit_node().numeric_value(),
        Some(f64::from(OWN))
    );
    splitter.increment_accesskit();
    harness.run();
    assert_eq!(harness.state().width, OWN + 10.0);
    // At its widest it can only be stepped narrower.
    harness.state_mut().width = 600.0;
    harness.run();
    let splitter = harness.get_by_role_and_label(Role::Splitter, "Side panel width");
    assert!(
        !splitter
            .accesskit_node()
            .data()
            .supports_action(Action::Increment)
    );
    assert!(
        splitter
            .accesskit_node()
            .data()
            .supports_action(Action::Decrement)
    );
}

#[test]
fn the_focused_panel_edge_moves_with_the_arrows_and_takes_the_ends_and_reset_keys() {
    for (at_start, wider, narrower) in [
        (false, Key::ArrowLeft, Key::ArrowRight),
        (true, Key::ArrowRight, Key::ArrowLeft),
    ] {
        let mut harness = panel(OWN, at_start);
        harness.run();
        harness
            .get_by_role_and_label(Role::Splitter, "Side panel width")
            .focus();
        harness.run();
        let width = |harness: &Harness<'_, Panel>| harness.state().width;
        harness.key_press(wider);
        harness.run();
        assert_eq!(width(&harness), OWN + 10.0, "at start: {at_start}");
        harness.key_press_modifiers(Modifiers::SHIFT, narrower);
        harness.run();
        assert_eq!(width(&harness), OWN - 40.0, "at start: {at_start}");
        harness.key_press(Key::End);
        harness.run();
        assert_eq!(width(&harness), 600.0);
        // Never past the ends.
        harness.key_press(wider);
        harness.run();
        assert_eq!(width(&harness), 600.0);
        harness.key_press(Key::Home);
        harness.run();
        assert_eq!(width(&harness), 320.0);
        harness.key_press(Key::Delete);
        harness.run();
        assert_eq!(width(&harness), OWN);
        assert_eq!(
            harness.state().actions.last(),
            Some(&PanelEdgeAction::Reset)
        );
        // The arrows stay with the edge rather than moving the focus.
        assert!(
            harness
                .get_by_role_and_label(Role::Splitter, "Side panel width")
                .is_focused()
        );
    }
}

#[test]
fn the_shell_at_full_width() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 300.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark)))
        .build_ui(|ui| {
            let panel = apertrust_ui::theme(ui.ctx()).colors.panel;
            ui.painter()
                .rect_filled(ui.ctx().content_rect(), 0.0, panel);
            ui.spacing_mut().item_spacing.y = 0.0;
            TopPanel::new()
                .available(Available {
                    redo: false,
                    ..Available::default()
                })
                .saved(SaveState::Saved)
                .show(ui);
            CentralBar::new("Image", Zoom::Fit)
                .toggles(
                    Toggle::new(false, true),
                    Toggle::new(true, true),
                    Toggle::new(false, true),
                    Toggle::new(false, true),
                )
                .layers(true)
                .values(Some([142, 128, 113]))
                .zoom_enabled(true)
                .show(ui);
            let (slot, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), 160.0), Sense::hover());
            let surround = apertrust_ui::theme(ui.ctx()).colors.photo_surround;
            ui.painter().rect_filled(slot, 0.0, surround);
            let frame = PhotoFrame::new(slot, Margin::Points(10.0));
            let room = frame.bounds();
            let width = room.x.min(room.y * 1.5);
            let placed = frame.place(vec2(width, width / 1.5), ui.pixels_per_point());
            ui.painter()
                .rect_filled(placed.photo, 0.0, Color32::from_gray(0x60));
            let exposure = ["1/250 s", "f/8", "35 mm", "ISO 100"].map(str::to_owned);
            PhotoCaption::new("IMG_2041.ARW")
                .exposure(&exposure)
                .show(ui, placed.caption);
        });
    harness.run();
    harness.snapshot("shell-full-width");
}
