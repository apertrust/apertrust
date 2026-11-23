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

//! The shell's parts: the Toolbar, the viewer bar, the Browser and the Tools panel's
//! edge, driven through AccessKit, and photographed at a full window's width, where nothing
//! is compact (`docs/gui.md`, Testing).

use apertrust_ui::components::{
    Browser, BrowserAction, CursorWell, CursorWellAction, Job, JobMessage, JobPhase, Jobs, Margin,
    MenuButtonStyle, PanelEdge, PanelEdgeAction, PhotoCaption, PhotoFrame, SaveState, Thumb,
    Toggle, Toolbar, ToolbarAction, ToolbarButton, ToolbarEntry, ToolbarMenu, ViewerBar,
    ViewerBarAction, WellCursor, WellGroup, WellSetting, Zoom,
};
use apertrust_ui::gallery::toolbar_entries;
use apertrust_ui::{Builtin, Icon, Locale, Theme, Translator};
use egui::accesskit::{Action, Role};
use egui::{Color32, CursorIcon, Key, Modifiers, PointerButton, Pos2, Sense, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

fn install(ctx: &egui::Context) {
    apertrust_ui::install(ctx, Theme::default());
}

/// Phase 2's well, Select, Pan and Pick, as the app hands it to the Toolbar, `chosen` the
/// group whose cursor is chosen, drawn by `draw`.
fn with_well<R>(chosen: usize, draw: impl FnOnce(CursorWell<'_>) -> R) -> R {
    let select = [WellCursor::new(Icon::Select, "Select").key("Esc")];
    let pan = [WellCursor::new(Icon::Pan, "Pan").key("A")];
    let picks = [
        WellCursor::new(Icon::PickWhiteBalance, "White balance picker").key("K"),
        WellCursor::new(Icon::PickExposure, "Exposure picker"),
    ];
    let settings = [WellSetting::Toggle {
        name: "Show the last pick on the photo",
        on: true,
    }];
    let groups = [
        WellGroup::new("Select", &select, 0),
        WellGroup::new("Pan", &pan, 0),
        WellGroup::new("Pick", &picks, 0).settings(&settings),
    ];
    draw(CursorWell::new(&groups).chosen(chosen))
}

/// The default Toolbar at a full window's width, those `disabled` drawn disabled, with
/// its labels where `labels`.
fn toolbar(
    disabled: &'static [&'static str],
    labels: bool,
) -> Harness<'static, Vec<ToolbarAction>> {
    Harness::builder()
        .with_size(egui::vec2(1200.0, 60.0))
        .with_context_setup(install)
        .build_ui_state(
            move |ui, actions: &mut Vec<ToolbarAction>| {
                if let Some(action) = Toolbar::new()
                    .entries(toolbar_entries(ui.ctx(), disabled))
                    .labels(labels)
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
fn the_toolbar_reports_its_actions() {
    for labels in [true, false] {
        the_toolbar_reports_its_actions_with(labels);
    }
}

/// [`the_toolbar_reports_its_actions`], with labels or without.
fn the_toolbar_reports_its_actions_with(labels: bool) {
    let mut harness = toolbar(&[], labels);
    harness.run();
    for name in ACTIONS {
        harness.get_by_label(name).click();
        harness.run();
    }
    assert_eq!(
        *harness.state(),
        vec![
            ToolbarAction::Button("import"),
            ToolbarAction::Button("export"),
            ToolbarAction::Button("reset"),
            ToolbarAction::Button("undo"),
            ToolbarAction::Button("redo"),
            ToolbarAction::Button("copy-edit"),
            ToolbarAction::Button("apply-edit"),
        ]
    );
    // No Settings button: Settings is in the menus and on its keys. No mark either: About
    // is in the menus.
    assert!(harness.query_by_label("Settings").is_none());
    assert!(harness.query_by_label("About Apertrust").is_none());
}

#[test]
fn the_actions_come_first_in_order_and_saved_last() {
    let mut harness = toolbar(&[], false);
    harness.run();
    let rect = |name: &str| harness.get_by_label(name).rect();
    for pair in ACTIONS.windows(2) {
        assert!(rect(pair[0]).right() < rect(pair[1]).left(), "{pair:?}");
    }
    // Import at the start, after the panel's margin: no mark before it.
    let bar = harness.get_by_role(Role::Toolbar).rect();
    assert_eq!(rect("Import…").left() - bar.left(), 8.0);
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Menu")
            .is_none()
    );
    // Copy edit and Apply edit are a group of their own, 16 points on, with no line.
    assert_eq!(rect("Copy edit").left() - rect("Redo").right(), 16.0);
    assert_eq!(rect("Export…").left() - rect("Import…").right(), 2.0);
    assert_eq!(rect("Reset").left() - rect("Export…").right(), 2.0);
    // The empty strip that moves the window, then the end, where the save state shows
    // nothing while saving works: no word, no tick.
    assert!(rect("Apply edit").right() < bar.center().x);
    for word in ["Saved", "Saving…", "Not saved"] {
        assert!(harness.query_by_label(word).is_none(), "{word}");
    }
}

#[test]
fn unavailable_actions_do_nothing() {
    for labels in [true, false] {
        let mut harness = toolbar(&["undo", "export"], labels);
        harness.run();
        harness.get_by_label("Undo").click();
        harness.get_by_label("Export…").click();
        harness.run();
        assert!(harness.state().is_empty(), "labels {labels}");
        assert!(harness.get_by_label("Undo").accesskit_node().is_disabled());
    }
}

/// Every text the panel drew, with where it stands.
fn texts<T>(harness: &Harness<'_, T>) -> Vec<(String, egui::Rect)> {
    fn walk(shape: &egui::Shape, found: &mut Vec<(String, egui::Rect)>) {
        match shape {
            egui::Shape::Text(text) => {
                let rect = text.galley.rect.translate(text.pos.to_vec2());
                found.push((text.galley.text().to_owned(), rect));
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| walk(shape, found)),
            _ => {}
        }
    }
    let mut found = Vec::new();
    for clipped in &harness.output().shapes {
        walk(&clipped.shape, &mut found);
    }
    found
}

/// Every image the panel drew, the icons among them, by where it stands.
fn images<T>(harness: &Harness<'_, T>) -> Vec<egui::Rect> {
    fn walk(shape: &egui::Shape, found: &mut Vec<egui::Rect>) {
        match shape {
            egui::Shape::Mesh(mesh) if mesh.texture_id != egui::TextureId::default() => {
                found.push(mesh.calc_bounds());
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| walk(shape, found)),
            _ => {}
        }
    }
    let mut found = Vec::new();
    for clipped in &harness.output().shapes {
        walk(&clipped.shape, &mut found);
    }
    found
}

/// Toolbar Labels (decision 92): each button's `.toolbar` word under its icon, centred,
/// the icon in a 22-point square 8 points below the panel's top and the word 4 points
/// below the square, the button as wide as its word or its icon, the wider, with 8 points
/// at each side; the panel 56 points tall with labels, 44 without, its buttons a gap
/// apart, as without, each 32 points wide with its 22-point icon.
#[test]
fn labels_stand_under_their_icons_in_a_taller_toolbar() {
    let mut harness = toolbar(&[], true);
    harness.run();
    let bar = harness.get_by_role(Role::Toolbar).rect();
    assert_eq!(bar.height(), 56.0);
    assert_eq!(Toolbar::height(&Theme::default(), true), 56.0);
    let words = Translator::new(Locale::EnGb);
    let drawn = texts(&harness);
    let icons = images(&harness);
    let caption = Theme::default().text.caption;
    for (name, id) in ACTIONS.iter().zip([
        "action-import",
        "action-export",
        "action-reset",
        "action-undo",
        "action-redo",
        "action-copy-edit",
        "action-apply-edit",
    ]) {
        let button = harness.get_by_label(name).rect();
        let word = words.attribute(id, "toolbar").expect("a .toolbar word");
        let (_, text) = (drawn.iter())
            .find(|(text, _)| *text == word)
            .unwrap_or_else(|| panic!("{word}: not drawn"));
        let icon = (icons.iter())
            .find(|icon| button.contains_rect(**icon))
            .unwrap_or_else(|| panic!("{name}: no icon"));
        // In a 22-point square 8 points below the panel's top, the word 4 points below it,
        // and 8 points of the panel left under the word.
        assert_eq!(icon.width(), 22.0, "{name}: {icon:?}");
        assert!((icon.center().y - bar.top() - 19.0).abs() < 0.51, "{name}");
        assert_eq!(text.top() - bar.top(), 8.0 + 22.0 + 4.0, "{name}");
        assert_eq!(bar.bottom() - text.bottom(), 8.0, "{name}: under the word");
        assert!((text.center().x - button.center().x).abs() < 0.01, "{name}");
        assert!((icon.center().x - button.center().x).abs() < 0.51, "{name}");
        assert!(text.height() < caption * 1.5, "{name}: one line");
        assert_eq!(button.width(), text.width().max(22.0) + 16.0, "{name}");
    }
    let rect = |name: &str| harness.get_by_label(name).rect();
    assert_eq!(rect("Export…").left() - rect("Import…").right(), 2.0);
    assert_eq!(rect("Copy edit").left() - rect("Redo").right(), 16.0);
    // Off, each button is its icon alone, Import…, Export… and Reset too, and no word
    // shows; the panel is 44 points.
    let mut off = toolbar(&[], false);
    off.run();
    assert_eq!(off.get_by_role(Role::Toolbar).rect().height(), 44.0);
    assert_eq!(Toolbar::height(&Theme::default(), false), 44.0);
    let icons = images(&off);
    for name in ACTIONS {
        let button = off.get_by_label(name).rect();
        assert_eq!(button.size(), egui::vec2(32.0, 32.0), "{name}");
        let icon = (icons.iter())
            .find(|icon| button.contains_rect(**icon))
            .unwrap_or_else(|| panic!("{name}: no icon"));
        assert_eq!(icon.width(), 22.0, "{name}: {icon:?}");
        assert!((icon.center() - button.center()).length() < 0.71, "{name}");
    }
    assert!(texts(&off).is_empty(), "{:?}", texts(&off));
}

/// With labels on, what has no label stays centred in the taller panel, as on Windows and
/// Linux: the menu button, the cursors' well and the caption buttons, which run its whole
/// height; and with actions under » as it narrows, the panel keeps its 56 points, » centred
/// in it too.
#[test]
fn what_has_no_label_stays_centred_in_the_taller_toolbar() {
    for width in [1400.0, 600.0] {
        let mut harness = narrow(width, SaveState::Saved, 0.0, true);
        harness.run();
        let bar = harness.get_by_role(Role::Toolbar).rect();
        assert_eq!(bar.height(), 56.0, "at {width}");
        let centred = |name: &str, rect: egui::Rect| {
            assert!(
                (rect.center().y - bar.center().y).abs() < 0.51,
                "{name} at {width}: {rect:?} in {bar:?}"
            );
            rect
        };
        let named = |name: &str| centred(name, harness.get_by_label(name).rect());
        named("Menu");
        centred("the well", harness.get_by_role(Role::RadioGroup).rect());
        for name in ["Minimise", "Close"] {
            assert_eq!(named(name).height(), bar.height(), "{name} at {width}");
        }
        if width < 1000.0 {
            named("More actions");
            let undo = harness.get_by_label("Undo").rect();
            assert!(undo.width() > 32.0, "its word kept: {undo:?}");
        }
    }
}

/// The Toolbar `width` points wide, with its labels, with the save state `saved`, and as
/// on the Mac, the window's buttons' room at its start, or as on Windows and Linux, the
/// menu button at its start and the caption buttons at its end; and Export's keys.
fn narrow(
    width: f32,
    saved: SaveState<'static>,
    inset: f32,
    caption: bool,
) -> Harness<'static, Vec<ToolbarAction>> {
    narrow_in(Locale::EnGb, true, width, saved, inset, caption)
}

/// [`narrow`] in `locale`, with labels where `labels`.
fn narrow_in(
    locale: Locale,
    labels: bool,
    width: f32,
    saved: SaveState<'static>,
    inset: f32,
    caption: bool,
) -> Harness<'static, Vec<ToolbarAction>> {
    Harness::builder()
        .with_size(egui::vec2(width, 120.0))
        .with_context_setup(move |ctx| {
            install(ctx);
            apertrust_ui::set_locale(ctx, locale);
        })
        .build_ui_state(
            move |ui, actions: &mut Vec<ToolbarAction>| {
                let bar = Toolbar::new()
                    .entries(toolbar_entries(ui.ctx(), &["redo", "apply-edit"]))
                    .labels(labels)
                    .saved(saved)
                    .start_inset(inset)
                    .title_bar(true)
                    .menu_button(caption)
                    .caption_buttons(caption)
                    .shortcut(ToolbarAction::Button("export"), "Ctrl+E");
                if let Some(action) = with_well(0, |well| bar.cursors(well).show(ui)) {
                    actions.push(action);
                }
            },
            Vec::new(),
        )
}

/// The common actions the panel shows, by name, and the width of each.
fn shown(harness: &Harness<'_, Vec<ToolbarAction>>) -> Vec<(&'static str, f32)> {
    ACTIONS
        .iter()
        .filter_map(|name| Some((*name, harness.query_by_label(name)?.rect().width())))
        .collect()
}

/// The same with labels or without (decision 92): every action shows, with its word where
/// labels are on, until it does not fit; then the actions go under » one at a time, words
/// and all, the shown ones keeping their words.
#[test]
fn a_narrowing_toolbar_folds_its_actions_into_more() {
    for labels in [true, false] {
        a_narrowing_toolbar_folds_its_actions_into_more_with(labels);
    }
    a_narrowing_toolbar_s_more_lists_what_it_holds();
}

/// [`a_narrowing_toolbar_folds_its_actions_into_more`], with labels or without.
fn a_narrowing_toolbar_folds_its_actions_into_more_with(labels: bool) {
    let saved = SaveState::Saved;
    let mut wide = narrow_in(Locale::EnGb, labels, 1400.0, saved, 0.0, false);
    wide.run();
    // Every action, with its word where labels are on; no ».
    let full = shown(&wide);
    assert_eq!(full.len(), 7);
    let labelled = |width: f32| if labels { width > 32.0 } else { width == 32.0 };
    assert!(full.iter().all(|(_, w)| labelled(*w)), "{labels}: {full:?}");
    assert!(wide.query_by_label("More actions").is_none());

    // Narrower, the actions go under » one at a time, a tool's items first from the end,
    // then Import…, the app's own, » fitting in the first's room; Undo and Redo last. The
    // ones still shown keep their widths, words and all.
    let mut narrowing = narrow_in(Locale::EnGb, labels, 1400.0, saved, 0.0, false);
    let mut gone: Vec<Vec<&str>> = Vec::new();
    let mut last = ACTIONS.to_vec();
    for width in (150..=1400).rev() {
        narrowing.set_size(egui::vec2(width as f32, 120.0));
        narrowing.run();
        let now = shown(&narrowing);
        for (name, w) in &now {
            let was = full.iter().find(|(n, _)| n == name).map(|(_, w)| *w);
            assert_eq!(Some(*w), was, "{labels}, {width}: {name}");
        }
        let now: Vec<&str> = now.into_iter().map(|(name, _)| name).collect();
        let went: Vec<&str> = last.iter().copied().filter(|n| !now.contains(n)).collect();
        if !went.is_empty() {
            gone.push(went);
        }
        last = now;
    }
    assert_eq!(
        gone,
        [
            vec!["Apply edit"],
            vec!["Copy edit"],
            vec!["Reset"],
            vec!["Export…"],
            vec!["Import…"],
            vec!["Undo", "Redo"],
        ],
        "{labels}"
    );
}

/// » lists what it holds with their keys, disabled where they can't run, and at the
/// narrowest Undo and Redo too.
fn a_narrowing_toolbar_s_more_lists_what_it_holds() {
    // With all but Undo and Redo under », » lists them with their keys and disabled where
    // they can't run.
    let mut more = narrow(370.0, SaveState::Saved, 0.0, false);
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
    assert_eq!(*more.state(), vec![ToolbarAction::Button("export")]);

    // At the narrowest, Undo and Redo go under » too.
    let mut least = narrow(150.0, SaveState::Saved, 0.0, false);
    least.run();
    assert!(shown(&least).is_empty());
    least.get_by_label("More actions").click();
    least.run();
    for name in ACTIONS {
        assert!(least.query_by_label(name).is_some(), "{name}");
    }
}

/// » is a pull-down of the popup service: each folded action an item with its keys as the
/// shortcut, disabled where it cannot run, the focus back on the button after Esc.
#[test]
fn the_more_menu_is_a_popup_list_with_keys_and_enabled_states() {
    let mut harness = narrow(370.0, SaveState::Saved, 0.0, false);
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
    assert_eq!(*harness.state(), vec![ToolbarAction::Button("import")]);
}

/// Opened with the pointer, » gives the focus to its button as it closes, chosen or not:
/// a press leaves no focus to give back, so the button is the menu's opener.
#[test]
fn the_more_menu_opened_with_the_pointer_gives_the_focus_to_its_button() {
    let focused = |harness: &Harness<'_, Vec<ToolbarAction>>| {
        harness
            .get_by(|node| node.is_focused())
            .accesskit_node()
            .label()
    };
    let mut harness = narrow(370.0, SaveState::Saved, 0.0, false);
    harness.run();
    harness.get_by_label("More actions").click();
    harness.run();
    harness
        .get_by_role_and_label(Role::MenuItem, "Export…")
        .click();
    harness.run();
    assert_eq!(*harness.state(), vec![ToolbarAction::Button("export")]);
    assert_eq!(focused(&harness).as_deref(), Some("More actions"));
    let mut harness = narrow(370.0, SaveState::Saved, 0.0, false);
    harness.run();
    harness.get_by_label("More actions").click();
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.query_by_role(Role::Menu).is_none());
    assert_eq!(focused(&harness).as_deref(), Some("More actions"));
}

#[test]
fn a_narrowing_toolbar_always_leaves_room_to_move_the_window() {
    // In every language; as on the Mac, with the traffic lights' room, and as on Linux,
    // with the caption buttons; with Saving… and the save warning, the save state's two
    // forms, whose slot is the same when it shows nothing; down to the window's minimum
    // width (`docs/gui.md`, The window frame and menus).
    for locale in Locale::ALL {
        let words = Translator::new(locale);
        let parts = [
            "toolbar-more",
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
        for (inset, caption, labels) in [
            (78.0, false, true),
            (0.0, true, true),
            (78.0, false, false),
            (0.0, true, false),
        ] {
            for saved in [SaveState::Saving, SaveState::Failed("disk full")] {
                let mut harness = narrow_in(locale, labels, 1400.0, saved, inset, caption);
                for width in (500..=1400).rev().step_by(5) {
                    harness.set_size(egui::vec2(width as f32, 120.0));
                    harness.run();
                    let last = parts
                        .iter()
                        .filter_map(|name| harness.query_by_label(name))
                        .map(|node| node.rect().right())
                        .fold(0.0, f32::max);
                    // The save state at the end, before the caption buttons' gap or the
                    // margin.
                    let status = save_state_in(&harness, &words);
                    let edge = if caption {
                        harness.get_by_label(&minimise).rect().left() - 16.0
                    } else {
                        harness.get_by_role(Role::Toolbar).rect().right() - 8.0
                    };
                    assert_eq!(status.right(), edge, "{locale:?}, {width}, {saved:?}");
                    let end = status.left();
                    // The cursors' well between them, the empty panel counted on both its
                    // sides together.
                    let well = harness.get_by_role(Role::RadioGroup).rect();
                    let (before, after) = (well.left() - last, end - well.right());
                    assert!(
                        before >= 0.0 && after >= 0.0 && before + after >= 24.0,
                        "{locale:?}, {width} points, {saved:?}, inset {inset}: {last}, {well:?}, {end}"
                    );
                }
            }
        }
    }
}

/// Where each of the overflow's steps starts, in points, in `locale`, with labels where
/// `labels`: as on the Mac, with the traffic lights' room, where `menu` is None, else as
/// on Windows and Linux, the menu button drawn as `menu` says and the caption buttons at
/// the end; with the jobs spinner's slot and Phase 2's cursors' well, as the app draws
/// them, and a failed write, whose words show the status step. Each is the widest Toolbar
/// that has taken the step, plus one, 1601 for one taken at the widest: the fixed space
/// narrowed, the fixed space gone, the first action under », every action but Undo and
/// Redo under », the status words folded, and Undo and Redo under ».
fn fold_widths(locale: Locale, menu: Option<MenuButtonStyle>, labels: bool) -> [u32; 6] {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1600.0, 120.0))
        .with_context_setup(move |ctx| {
            install(ctx);
            apertrust_ui::set_locale(ctx, locale);
        })
        .build_ui(move |ui| {
            let inset = if menu.is_some() { 0.0 } else { 78.0 };
            let bar = Toolbar::new()
                .entries(toolbar_entries(ui.ctx(), &[]))
                .labels(labels)
                .saved(SaveState::Failed("disk full"))
                .jobs(Jobs::new(&[]))
                .start_inset(inset)
                .title_bar(true)
                .menu_button(menu.is_some())
                .menu_button_style(menu.unwrap_or_default())
                .caption_buttons(menu.is_some());
            with_well(0, |well| bar.cursors(well).show(ui));
        });
    let words = Translator::new(locale);
    let [import, redo, copy, more, undo, failed] = [
        "action-import",
        "action-redo",
        "action-copy-edit",
        "toolbar-more",
        "action-undo",
        "status-not-saved",
    ]
    .map(|id| words.text(id));
    // The room between Redo and Copy edit, the fixed space's, while both show.
    let space = |harness: &Harness<'_>| {
        let redo = harness.query_by_label(&redo)?.rect();
        Some(harness.query_by_label(&copy)?.rect().left() - redo.right())
    };
    let sign = 2.0 * Theme::default().spacing.s + Icon::Warning.size();
    let mut starts = [0; 6];
    for width in (150..=1600).rev() {
        harness.set_size(egui::vec2(width as f32, 120.0));
        harness.run();
        let taken = [
            space(&harness).is_none_or(|room| room < 16.0),
            space(&harness).is_none_or(|room| room < 8.0),
            harness.query_by_label(&more).is_some(),
            harness.query_by_label(&import).is_none(),
            harness.get_by_label(&failed).rect().width() == sign,
            harness.query_by_label(&undo).is_none(),
        ];
        for (start, taken) in starts.iter_mut().zip(taken) {
            if taken && *start == 0 {
                *start = width + 1;
            }
        }
    }
    starts
}

/// The default list with every part and many more actions placed (decision 92): a space
/// and twelve actions more before the first flexible space, Export Again and Orientation's
/// two a tool's items, as the app marks them, the rest the app's own.
fn every_part(ctx: &egui::Context) -> Vec<ToolbarEntry> {
    let mut entries = toolbar_entries(ctx, &[]);
    let at = (entries.iter())
        .position(|entry| *entry == ToolbarEntry::FlexibleSpace)
        .expect("a flexible space");
    let more = [
        "open",
        "open-folder",
        "settings",
        "full-screen",
        "zoom-100",
        "zoom-fit",
        "show-jobs",
        "next-photo",
        "previous-photo",
        "export-again",
        "tool-flip-rotate_ccw",
        "tool-flip-rotate_cw",
    ]
    .map(|id| {
        let button = ToolbarButton::new(id, Icon::Settings, id);
        let tool = id == "export-again" || id.starts_with("tool-");
        ToolbarEntry::Button(button.folds_first(tool))
    });
    entries.splice(at..at, [ToolbarEntry::Space].into_iter().chain(more));
    entries
}

/// At 500 points, the planned minimum (decision 39), in `locale`, the list `entries` gives, as on
/// the Mac where `menu` is None, else as on Windows and Linux with the menu button drawn
/// as `menu` says: every part shows, the menu button, the cursors' well, the jobs spinner
/// and the save warning, with », and 24 points of the panel are empty, counted on both
/// sides of the well together.
fn at_500(
    locale: Locale,
    menu: Option<MenuButtonStyle>,
    entries: fn(&egui::Context) -> Vec<ToolbarEntry>,
) {
    let mut harness = every_part_bar(locale, menu, entries, 1.0, 500.0);
    harness.run();
    keeps_every_part(&harness, locale, menu);
}

/// The Toolbar `width` points wide at the interface size `zoom`, in `locale`, the list
/// `entries` gives, as on the Mac where `menu` is None, else as on Windows and Linux with
/// the menu button drawn as `menu` says, with labels, a job running and a failed write; the
/// traffic lights' 78 points at the interface size, as the app measures them.
fn every_part_bar(
    locale: Locale,
    menu: Option<MenuButtonStyle>,
    entries: fn(&egui::Context) -> Vec<ToolbarEntry>,
    zoom: f32,
    width: f32,
) -> Harness<'static> {
    Harness::builder()
        .with_size(egui::vec2(width, 120.0))
        .with_context_setup(move |ctx| {
            install(ctx);
            apertrust_ui::set_locale(ctx, locale);
            ctx.options_mut(|o| o.zoom_factor = zoom);
        })
        .build_ui(move |ui| {
            let jobs = [Job::new(
                1,
                JobPhase::Running,
                JobMessage::new("job-thumbnails"),
            )];
            let bar = Toolbar::new()
                .entries(entries(ui.ctx()))
                .labels(true)
                .saved(SaveState::Failed("disk full"))
                .jobs(Jobs::new(&jobs))
                .start_inset(if menu.is_some() { 0.0 } else { 78.0 / zoom })
                .title_bar(true)
                .menu_button(menu.is_some())
                .menu_button_style(menu.unwrap_or_default())
                .caption_buttons(menu.is_some());
            with_well(0, |well| bar.cursors(well).show(ui));
        })
}

/// Every part shows, the menu button where `menu` says, the cursors' well, the jobs
/// spinner and the save warning, with », no button but the well's own runs over another,
/// the well or past the panel's end, and 24 points of the panel are empty, counted on both
/// sides of the well together.
fn keeps_every_part<T>(harness: &Harness<'_, T>, locale: Locale, menu: Option<MenuButtonStyle>) {
    let words = Translator::new(locale);
    let width = harness.ctx.content_rect().width();
    let at = format!("{locale:?}, {menu:?}, {width} points");
    for id in ["toolbar-more", "jobs-name", "status-not-saved"] {
        let name = words.text(id);
        assert!(harness.query_by_label(&name).is_some(), "{at}: {name}");
    }
    let menu_name = words.text("toolbar-menu");
    let menu_button = harness.query_by_role_and_label(Role::Button, &menu_name);
    assert_eq!(menu_button.is_some(), menu.is_some(), "{at}");
    let well = harness.get_by_role(Role::RadioGroup).rect();
    let mut buttons: Vec<egui::Rect> = (harness.query_all_by_role(Role::Button))
        .map(|node| node.rect())
        .filter(|rect| !well.contains_rect(*rect))
        .collect();
    buttons.push(well);
    buttons.sort_by(|a, b| a.left().total_cmp(&b.left()));
    for pair in buttons.windows(2) {
        assert!(pair[0].right() <= pair[1].left(), "{at}: {pair:?} overlap");
    }
    let end = buttons.last().map_or(0.0, |rect| rect.right());
    assert!(end <= width, "{at}: a button ends at {end}");
    let before = (buttons.iter())
        .filter(|rect| rect.right() <= well.left())
        .map(|rect| rect.right())
        .fold(f32::MIN, f32::max);
    let after = (buttons.iter())
        .filter(|rect| rect.left() >= well.right())
        .map(|rect| rect.left())
        .fold(f32::MAX, f32::min);
    let empty = (well.left() - before) + (after - well.right());
    assert!(empty >= 24.0, "{at}: {empty} points empty");
}

/// The widths `docs/gui.md`'s Narrowing table gives, each step's start as the window
/// narrows: on the Mac in each language, and on Linux with either menu button, with labels
/// and without, the same steps either way. And at 500 points, in every language, both platforms' defaults and a list with
/// every part placed fold no part and keep 24 points empty.
#[test]
fn the_toolbar_folds_where_gui_md_says() {
    type Case = (Locale, Option<MenuButtonStyle>, [u32; 6], [u32; 6]);
    let cases: [Case; 5] = [
        (
            Locale::EnGb,
            None,
            [744, 736, 730, 533, 481, 423],
            [613, 605, 599, 495, 461, 403],
        ),
        (
            Locale::EnGb,
            Some(MenuButtonStyle::Icon),
            [869, 861, 855, 658, 606, 548],
            [738, 730, 724, 620, 586, 528],
        ),
        (
            Locale::EnGb,
            Some(MenuButtonStyle::Menu),
            [858, 850, 844, 647, 595, 537],
            [727, 719, 713, 609, 575, 517],
        ),
        (
            Locale::Uk,
            None,
            [875, 867, 861, 618, 564, 480],
            [640, 632, 626, 522, 488, 403],
        ),
        (
            Locale::Pl,
            None,
            [848, 840, 834, 567, 506, 433],
            [628, 620, 614, 510, 476, 403],
        ),
    ];
    for (locale, menu, on, off) in cases {
        let measured = fold_widths(locale, menu, true);
        assert_eq!(measured, on, "{locale:?}, {menu:?}, labels on");
        let measured = fold_widths(locale, menu, false);
        assert_eq!(measured, off, "{locale:?}, {menu:?}, labels off");
    }
    let defaults: fn(&egui::Context) -> Vec<ToolbarEntry> = |ctx| toolbar_entries(ctx, &[]);
    for locale in Locale::ALL {
        for menu in [
            None,
            Some(MenuButtonStyle::Icon),
            Some(MenuButtonStyle::Menu),
        ] {
            at_500(locale, menu, defaults);
            at_500(locale, menu, every_part);
        }
    }
}

/// The interface size setting's largest (`scale` in `data/settings.schema.toml`).
const LARGEST_SCALE: f32 = 3.0;

/// At the largest interface size, with every part placed, in every language, as on the
/// Mac and on Windows and Linux with either menu button, the least width the Toolbar
/// reports (decision 92), the window's least where the app's own minimum is narrower: it
/// is `docs/gui.md`'s (Narrowing), whatever the language, and at it the Toolbar folds no
/// part, overlaps nothing and keeps 24 points empty.
#[test]
fn the_least_width_keeps_what_never_folds() {
    for locale in Locale::ALL {
        for (menu, expected) in [
            (None, 267.0),
            (Some(MenuButtonStyle::Icon), 444.0),
            (Some(MenuButtonStyle::Menu), 433.0),
        ] {
            let mut harness = every_part_bar(locale, menu, every_part, LARGEST_SCALE, 1400.0);
            harness.run();
            let least = Toolbar::least_width(&harness.ctx).expect("a least width");
            assert_eq!(least, expected, "{locale:?}, {menu:?}");
            harness.set_size(vec2(least, 120.0));
            harness.run();
            keeps_every_part(&harness, locale, menu);
        }
    }
}

/// With the save state and the jobs left out of the list, as on Windows and Linux, a failed
/// write and a failed job, whose guards then show, leave the least width as it is, and at it
/// the Toolbar still folds no part, overlaps nothing and keeps 24 points empty.
#[test]
fn a_guard_never_changes_the_least_width() {
    let hidden: fn(&egui::Context) -> Vec<ToolbarEntry> = |ctx| {
        let mut entries = every_part(ctx);
        entries.retain(|entry| !matches!(entry, ToolbarEntry::SaveState | ToolbarEntry::Jobs));
        entries
    };
    let menu = Some(MenuButtonStyle::Icon);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 120.0))
        .with_context_setup(install)
        .build_ui_state(
            move |ui, failed: &mut bool| {
                let (saved, phase) = if *failed {
                    (SaveState::Failed("disk full"), JobPhase::Failed)
                } else {
                    (SaveState::Saved, JobPhase::Running)
                };
                let jobs = [Job::new(1, phase, JobMessage::new("job-thumbnails"))];
                let bar = Toolbar::new()
                    .entries(hidden(ui.ctx()))
                    .labels(true)
                    .saved(saved)
                    .jobs(Jobs::new(&jobs))
                    .title_bar(true)
                    .menu_button(true)
                    .menu_button_style(MenuButtonStyle::Icon)
                    .caption_buttons(true);
                with_well(0, |well| bar.cursors(well).show(ui));
            },
            false,
        );
    harness.run();
    let quiet = Toolbar::least_width(&harness.ctx).expect("a least width");
    *harness.state_mut() = true;
    harness.run();
    assert_eq!(Toolbar::least_width(&harness.ctx), Some(quiet));
    harness.set_size(vec2(quiet, 120.0));
    harness.run();
    keeps_every_part(&harness, Locale::EnGb, menu);
}

/// Where Saving…, or the save warning, whichever shows, is drawn.
fn save_state<T>(harness: &Harness<'_, T>) -> egui::Rect {
    save_state_in(harness, &Translator::new(Locale::EnGb))
}

/// [`save_state`] in the language of `words`.
fn save_state_in<T>(harness: &Harness<'_, T>, words: &Translator) -> egui::Rect {
    let (saving, failed) = (words.text("status-saving"), words.text("status-not-saved"));
    (harness.query_by_role_and_label(Role::Label, &saving))
        .or_else(|| harness.query_by_role_and_label(Role::Button, &failed))
        .expect("no save state shown")
        .rect()
}

/// The save states, from no photo to a failed write.
const SAVE_STATES: [SaveState<'static>; 4] = [
    SaveState::None,
    SaveState::Saved,
    SaveState::Saving,
    SaveState::Failed("disk full"),
];

#[test]
fn saving_never_moves_the_fold() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 120.0))
        .with_context_setup(install)
        .build_ui_state(
            |ui, saved: &mut SaveState<'static>| {
                Toolbar::new()
                    .entries(toolbar_entries(ui.ctx(), &[]))
                    .saved(*saved)
                    .show(ui);
            },
            SaveState::Saved,
        );
    // Whether Import has its words, whether » shows, and whether Undo is out of it.
    let fold = |h: &Harness<'_, SaveState<'static>>| {
        (
            h.query_by_label("Import…")
                .is_some_and(|node| node.rect().width() > 32.0),
            h.query_by_label("More actions").is_some(),
            h.query_by_label("Undo").is_some(),
        )
    };
    // An edit, its slow save, its failure, at every width as the window narrows.
    for width in (150..=900).rev() {
        harness.set_size(egui::vec2(width as f32, 120.0));
        *harness.state_mut() = SaveState::Saved;
        harness.run();
        let saved = fold(&harness);
        for state in SAVE_STATES {
            *harness.state_mut() = state;
            harness.run();
            assert_eq!(fold(&harness), saved, "{width} points, {state:?}");
        }
    }
}

/// What the save state test harness holds: the state drawn and what the panel reported.
type Saving = (SaveState<'static>, Vec<ToolbarAction>);

/// The save state as the app draws it, in a panel `width` points wide that is the title bar.
fn saving(width: f32) -> Harness<'static, Saving> {
    Harness::builder()
        .with_size(egui::vec2(width, 120.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(install)
        .build_ui_state(
            |ui, (saved, actions): &mut Saving| {
                let action = Toolbar::new()
                    .entries(toolbar_entries(ui.ctx(), &[]))
                    .saved(*saved)
                    .title_bar(true)
                    .show(ui);
                actions.extend(action);
            },
            (SaveState::Saved, Vec::new()),
        )
}

#[test]
fn nothing_shows_at_rest_and_the_slot_moves_the_window() {
    let mut harness = saving(1200.0);
    harness.run();
    for word in ["Saved", "Saving…", "Not saved"] {
        assert!(harness.query_by_label(word).is_none(), "{word}");
    }
    // Where the warning would be, the strip moves the window.
    let bar = harness.get_by_role(Role::Toolbar).rect();
    let slot = egui::pos2(bar.right() - 20.0, bar.center().y);
    harness.hover_at(slot);
    harness.event(egui::Event::PointerButton {
        pos: slot,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    for step in 1..=3 {
        harness.hover_at(slot - egui::vec2(10.0 * step as f32, 0.0));
        harness.step();
    }
    harness.drop_at(slot - egui::vec2(30.0, 0.0));
    harness.run();
    assert_eq!(harness.state().1, vec![ToolbarAction::MoveWindow]);
}

#[test]
fn saving_shows_in_words_that_fold_to_an_ellipsis() {
    let mut harness = saving(1200.0);
    harness.state_mut().0 = SaveState::Saving;
    harness.run();
    let words = save_state(&harness);
    // Not a control.
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Saving…")
            .is_none()
    );
    harness.set_size(egui::vec2(150.0, 120.0));
    harness.run();
    let folded = save_state(&harness);
    assert!(folded.width() < words.width() / 3.0, "{folded:?} {words:?}");
}

#[test]
fn a_failed_write_shows_a_warning_button_that_asks_for_the_reason() {
    for width in [1200.0, 150.0] {
        let mut harness = saving(width);
        harness.state_mut().0 = SaveState::Failed("Permission denied");
        harness.run();
        let warning = harness.get_by_role_and_label(Role::Button, "Not saved");
        // The reason is its description, and its tooltip.
        assert_eq!(
            warning.accesskit_node().description().as_deref(),
            Some("Permission denied"),
        );
        // Folded, the sign alone, which never folds away.
        let rect = warning.rect();
        if width < 200.0 {
            assert_eq!(rect.width(), 24.0, "{rect:?}");
        } else {
            assert!(rect.width() > 40.0, "{rect:?}");
        }
        // Focusable from the keyboard; pressed, it asks for the reason.
        warning.focus();
        harness.run();
        harness.key_press(Key::Enter);
        harness.run();
        harness
            .get_by_role_and_label(Role::Button, "Not saved")
            .click();
        harness.run();
        assert_eq!(harness.state().1, vec![ToolbarAction::NotSaved; 2]);
    }
}

/// What the save state's live region, an assertive status, says.
fn said<T>(harness: &Harness<'_, T>) -> String {
    let live = harness
        .query_all_by_role(Role::Status)
        .find(|n| n.accesskit_node().data().live() == Some(egui::accesskit::Live::Assertive))
        .expect("no live region");
    live.accesskit_node()
        .label()
        .unwrap_or_default()
        .to_string()
}

#[test]
fn screen_readers_hear_a_failed_write_and_saving_working_again_and_nothing_else() {
    for locale in Locale::ALL {
        let words = Translator::new(locale);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1200.0, 120.0))
            .with_context_setup(move |ctx| {
                install(ctx);
                apertrust_ui::set_locale(ctx, locale);
            })
            .build_ui_state(
                |ui, saved: &mut SaveState<'static>| {
                    Toolbar::new()
                        .entries(toolbar_entries(ui.ctx(), &[]))
                        .saved(*saved)
                        .show(ui);
                },
                SaveState::Saved,
            );
        let mut step = |state| {
            *harness.state_mut() = state;
            harness.step();
            said(&harness)
        };
        // Saves, slow or not, say nothing.
        assert_eq!(step(SaveState::Saved), "", "{locale:?}");
        assert_eq!(step(SaveState::Saving), "", "{locale:?}");
        assert_eq!(step(SaveState::Saved), "", "{locale:?}");
        // A failure says why, on the frame it comes.
        let mut args = apertrust_ui::i18n::Args::new();
        args.set("error", "disk full");
        let failed = words.text_with("status-not-saved-reason", Some(&args));
        assert_eq!(step(SaveState::Failed("disk full")), failed, "{locale:?}");
        assert_eq!(step(SaveState::Failed("disk full")), failed, "{locale:?}");
        // A write that succeeds says Saved, kept as later saves come.
        let saved = words.text("status-saved");
        assert_ne!(
            step(SaveState::Saving),
            saved,
            "a retry has not succeeded: {locale:?}"
        );
        assert_eq!(step(SaveState::Saved), saved, "{locale:?}");
        assert_eq!(step(SaveState::Saving), saved, "{locale:?}");
        // A photo left with its write failed is the Viewer's to tell: no Saved.
        step(SaveState::Failed("gone"));
        assert_ne!(step(SaveState::None), saved, "{locale:?}");
        assert_ne!(step(SaveState::Saved), saved, "{locale:?}");
    }
}

/// The save warning, then Saving…, at the end of a Toolbar in the dark theme.
#[test]
fn the_save_warning_and_saving() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(700.0, 124.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark)))
        .build_ui(|ui| {
            let panel = apertrust_ui::theme(ui.ctx()).colors.panel;
            ui.painter()
                .rect_filled(ui.ctx().content_rect(), 0.0, panel);
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.push_id("failed", |ui| {
                Toolbar::new()
                    .entries(toolbar_entries(ui.ctx(), &[]))
                    .labels(true)
                    .saved(SaveState::Failed("Permission denied"))
                    .show(ui);
            });
            ui.push_id("saving", |ui| {
                Toolbar::new()
                    .entries(toolbar_entries(ui.ctx(), &[]))
                    .labels(true)
                    .saved(SaveState::Saving)
                    .show(ui);
            });
        });
    harness.run();
    harness.snapshot("toolbar-save-state");
}

#[test]
fn a_widening_toolbar_unfolds_only_with_room_to_spare() {
    let mut width = 1400.0;
    let mut harness = narrow(width, SaveState::Saved, 0.0, false);
    harness.run();
    let all = |h: &Harness<'_, Vec<ToolbarAction>>| h.query_by_label("More actions").is_none();
    // Narrowed a point at a time until » shows, the first action under it.
    while all(&harness) {
        width -= 1.0;
        harness.set_size(egui::vec2(width, 120.0));
        harness.run();
    }
    // Widened by less than 16 points, » stays; by 16 more, the action comes back.
    harness.set_size(egui::vec2(width + 10.0, 120.0));
    harness.run();
    assert!(!all(&harness));
    harness.set_size(egui::vec2(width + 18.0, 120.0));
    harness.run();
    assert!(all(&harness));
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
            Toolbar::new()
                .entries(toolbar_entries(ui.ctx(), &[]))
                .shortcut(ToolbarAction::Button("copy-edit"), "Ctrl+Shift+C")
                .shortcut(ToolbarAction::Button("export"), "Ctrl+E")
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
}

/// The Toolbar as the window's title bar, or not, with room for the window's buttons at
/// its start. At 60 frames a second, since each event takes a frame of its own and a
/// double-click's second click must come within 0.3 s of its first.
fn title_bar(on: bool, start: f32) -> Harness<'static, Vec<ToolbarAction>> {
    title_bar_with(on, start, &[])
}

/// The title bar with the actions `disabled` names unable to run.
fn title_bar_with(
    on: bool,
    start: f32,
    disabled: &'static [&'static str],
) -> Harness<'static, Vec<ToolbarAction>> {
    Harness::builder()
        .with_size(egui::vec2(1200.0, 120.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(install)
        .build_ui_state(
            move |ui, actions: &mut Vec<ToolbarAction>| {
                if let Some(action) = Toolbar::new()
                    .entries(toolbar_entries(ui.ctx(), disabled))
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

/// An empty point of the panel: after the last action at its start, well before its end,
/// where the save state's slot shows nothing at rest.
fn empty_point<T>(harness: &Harness<'_, T>) -> egui::Pos2 {
    let apply = harness.get_by_label("Apply edit").rect();
    let end = harness.get_by_role(Role::Toolbar).rect().right() - 100.0;
    egui::pos2((apply.right() + end) / 2.0, apply.center().y)
}

fn press(harness: &Harness<'_, Vec<ToolbarAction>>, pos: egui::Pos2, pressed: bool) {
    harness.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    });
}

/// Two clicks at `pos`, half a second after anything before, which would otherwise make
/// them a triple click.
fn double_click(harness: &mut Harness<'_, Vec<ToolbarAction>>, pos: egui::Pos2) {
    harness.run_steps(30);
    harness.hover_at(pos);
    for pressed in [true, false, true, false] {
        press(harness, pos, pressed);
    }
    harness.run();
}

/// Presses at `from`, moves 30 points across a frame at a time, and lets go.
fn drag(harness: &mut Harness<'_, Vec<ToolbarAction>>, from: egui::Pos2) {
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
fn the_toolbar_keeps_room_for_the_window_s_buttons() {
    let mut plain = title_bar(false, 0.0);
    plain.run();
    let mut inset = title_bar(true, 90.0);
    inset.run();
    let first = |h: &Harness<'_, Vec<ToolbarAction>>| h.get_by_label("Import…").rect().left();
    let last = |h: &Harness<'_, Vec<ToolbarAction>>| h.get_by_role(Role::Status).rect().right();
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
    assert_eq!(*harness.state(), vec![ToolbarAction::TitleBarDoubleClick]);
    // A drag is reported once, however far it goes.
    harness.state_mut().clear();
    drag(&mut harness, empty);
    assert_eq!(*harness.state(), vec![ToolbarAction::MoveWindow]);

    // On a button, a double-click is two clicks, and a drag neither clicks nor moves.
    // Import, the first, starts after the window's buttons' room.
    harness.state_mut().clear();
    let import = harness.get_by_label("Import…").rect();
    let bar = harness.get_by_role(Role::Toolbar).rect();
    assert_eq!(import.left() - bar.left(), 90.0);
    double_click(&mut harness, import.center());
    assert_eq!(*harness.state(), vec![ToolbarAction::Button("import"); 2]);
    harness.state_mut().clear();
    let reset = harness.get_by_label("Reset").rect().center();
    double_click(&mut harness, reset);
    assert_eq!(*harness.state(), vec![ToolbarAction::Button("reset"); 2]);
    harness.state_mut().clear();
    let undo = harness.get_by_label("Undo").rect().center();
    drag(&mut harness, undo);
    let apply = harness.get_by_label("Apply edit").rect();
    drag(&mut harness, apply.center());
    drag(&mut harness, import.center());
    assert!(harness.state().is_empty());
}

#[test]
fn a_toolbar_that_is_not_the_title_bar_ignores_its_empty_areas() {
    let mut harness = title_bar(false, 0.0);
    harness.run();
    let empty = empty_point(&harness);
    double_click(&mut harness, empty);
    drag(&mut harness, empty);
    assert!(harness.state().is_empty());
}

#[test]
fn a_disabled_button_and_the_margin_round_a_button_are_no_empty_areas() {
    let mut harness = title_bar_with(true, 90.0, &["undo"]);
    harness.run();
    let undo = harness.get_by_label("Undo").rect().center();
    double_click(&mut harness, undo);
    drag(&mut harness, undo);
    assert!(harness.state().is_empty());
    // Just off a button, within egui's margin, a press is still the button's.
    let reset = harness.get_by_label("Reset").rect();
    let near = egui::pos2(reset.center().x, reset.bottom() + 3.0);
    double_click(&mut harness, near);
    assert_eq!(*harness.state(), vec![ToolbarAction::Button("reset"); 2]);
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
    assert_eq!(*harness.state(), vec![ToolbarAction::MoveWindow; 2]);
}

/// The title bar with Phase 2's cursors' well, Select chosen, `width` points wide.
fn title_bar_with_well(width: f32) -> Harness<'static, Vec<ToolbarAction>> {
    Harness::builder()
        .with_size(egui::vec2(width, 120.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(install)
        .build_ui_state(
            |ui, actions: &mut Vec<ToolbarAction>| {
                let bar = Toolbar::new()
                    .entries(toolbar_entries(ui.ctx(), &[]))
                    .saved(SaveState::Saved)
                    .title_bar(true);
                actions.extend(with_well(0, |well| bar.cursors(well).show(ui)));
            },
            Vec::new(),
        )
}

/// The well sits after Apply edit, a group's gap before it at least, its presses reported
/// as the well's; it is no empty area, so neither a drag nor a double-click on it, between
/// its buttons too, moves or zooms the window, while the strips either side still do.
#[test]
fn the_cursors_well_is_the_toolbar_s_and_never_moves_the_window() {
    let mut harness = title_bar_with_well(1200.0);
    harness.run();
    let well = harness.get_by_role(Role::RadioGroup).rect();
    let apply = harness.get_by_label("Apply edit").rect();
    assert!(well.left() - apply.right() >= 16.0, "{apply:?}, {well:?}");
    assert_eq!(well.size(), egui::vec2(101.0, 30.0));
    harness.get_by_label("Pan, A").click();
    harness.run();
    assert_eq!(
        *harness.state(),
        vec![ToolbarAction::Cursors(CursorWellAction::Cursor {
            group: 1,
            cursor: 0
        })]
    );
    harness.state_mut().clear();
    // Between Select's face and Pan's, the track showing through.
    let select = harness.get_by_label("Select, Esc").rect();
    let between = egui::pos2(select.right() + 3.0, select.center().y);
    for at in [between, egui::pos2(well.left() + 1.0, well.center().y)] {
        drag(&mut harness, at);
        harness.run_steps(30);
        harness.hover_at(at);
        for pressed in [true, false, true, false] {
            press(&harness, at, pressed);
        }
        harness.run();
    }
    assert!(
        (harness.state().iter()).all(|a| !matches!(
            a,
            ToolbarAction::MoveWindow | ToolbarAction::TitleBarDoubleClick
        )),
        "{:?}",
        harness.state()
    );
    // The strip after it still moves the window.
    harness.state_mut().clear();
    drag(
        &mut harness,
        egui::pos2(well.right() + 40.0, well.center().y),
    );
    assert_eq!(*harness.state(), vec![ToolbarAction::MoveWindow]);
    // A right-click on it, between its faces too, asks for the toolbar menu on the well,
    // the list's eleventh entry (decision 92).
    for at in [between, select.center()] {
        harness.state_mut().clear();
        right_click(&mut harness, at);
        let menu = match harness.state()[..] {
            [ToolbarAction::ContextMenu(menu)] => menu,
            ref other => panic!("{other:?}"),
        };
        assert_eq!(menu.entry, Some(10));
    }
}

/// The Toolbar with Phase 2's well, Select chosen, at a full window's width, in the
/// theme `which`, with labels where `labels` (`docs/gui.md`, The Toolbar; `docs/cursors.md`,
/// The well). The picture is the panel and 8 points round it.
fn photograph_the_well(which: Builtin, labels: bool, name: &'static str) {
    let height = Toolbar::height(&Theme::builtin(which), labels) + 16.0;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1000.0, height))
        .with_context_setup(move |ctx| apertrust_ui::install(ctx, Theme::builtin(which)))
        .build_ui(move |ui| {
            let panel = apertrust_ui::theme(ui.ctx()).colors.panel;
            ui.painter()
                .rect_filled(ui.ctx().content_rect(), 0.0, panel);
            let bar = Toolbar::new()
                .entries(toolbar_entries(ui.ctx(), &[]))
                .labels(labels)
                .saved(SaveState::Saved)
                .jobs(Jobs::new(&[]))
                .title_bar(true);
            with_well(0, |well| bar.cursors(well).show(ui));
        });
    harness.run();
    harness.snapshot(name);
}

#[test]
fn the_toolbar_with_the_cursors_well_is_photographed_in_the_dark_theme() {
    photograph_the_well(Builtin::Dark, true, "toolbar-cursors-dark");
}

#[test]
fn the_toolbar_with_the_cursors_well_is_photographed_in_the_light_theme() {
    photograph_the_well(Builtin::Light, true, "toolbar-cursors-light");
}

/// Toolbar Labels off: every action its icon alone (decision 92).
#[test]
fn the_toolbar_without_labels_is_photographed_in_the_dark_theme() {
    photograph_the_well(Builtin::Dark, false, "toolbar-labels-off-dark");
}

/// A right-click at `pos`, as a press and a release.
fn right_click(harness: &mut Harness<'_, Vec<ToolbarAction>>, pos: egui::Pos2) {
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

/// The toolbar menu each action of `harness` asked for: the entry it was asked on, where it
/// hangs from, and the control the focus goes back to.
fn menus_asked(
    harness: &Harness<'_, Vec<ToolbarAction>>,
) -> Vec<(Option<usize>, Pos2, Option<egui::Id>)> {
    (harness.state().iter())
        .map(|action| match action {
            ToolbarAction::ContextMenu(menu) => (menu.entry, menu.at, menu.opener),
            other => panic!("{other:?} is no toolbar menu"),
        })
        .collect()
}

/// Decision 92: a right-click anywhere on the Toolbar asks for the toolbar menu, title bar
/// or not: on an empty area with no entry, on a button, enabled or not, with its place in
/// the list, at the pointer. It presses nothing, and never asks for the window's menu.
#[test]
fn a_right_click_anywhere_asks_for_the_toolbar_menu_with_what_is_under_it() {
    for on in [true, false] {
        let mut harness = title_bar(on, 0.0);
        harness.run();
        let empty = empty_point(&harness);
        right_click(&mut harness, empty);
        assert_eq!(menus_asked(&harness), vec![(None, empty, None)], "{on}");
        harness.state_mut().clear();
        // The list's places, the menu button first.
        let buttons = [("Import…", 1), ("Reset", 3), ("Undo", 4), ("Apply edit", 8)];
        for (name, _) in buttons {
            let control = harness.get_by_label(name).rect().center();
            right_click(&mut harness, control);
        }
        let entries: Vec<Option<usize>> = (menus_asked(&harness).iter())
            .map(|(entry, _, _)| *entry)
            .collect();
        let expected: Vec<Option<usize>> = buttons.iter().map(|(_, at)| Some(*at)).collect();
        assert_eq!(entries, expected, "{on}");
    }
}

/// Decision 92: the menu key or Shift+F10 on a focused button asks for the toolbar menu on
/// it, under it, the focus going back to it as it closes, as does a screen reader's
/// ShowContextMenu, which each button offers.
#[test]
fn the_menu_keys_on_a_focused_button_ask_for_the_toolbar_menu_on_it() {
    let mut harness = title_bar(true, 0.0);
    harness.run();
    harness.get_by_label("Undo").focus();
    harness.run();
    let undo = harness.get_by_label("Undo");
    let under = undo.rect().left_bottom();
    let (undo_id, _) = undo.accesskit_node().locate();
    for (modifiers, key) in [(Modifiers::NONE, Key::Menu), (Modifiers::SHIFT, Key::F10)] {
        harness.state_mut().clear();
        harness.key_press_modifiers(modifiers, key);
        harness.run();
        let asked = menus_asked(&harness);
        assert_eq!(asked.len(), 1, "{key:?}");
        let (entry, at, opener) = asked[0];
        assert_eq!((entry, at), (Some(4), under), "{key:?}");
        let opener = opener.map(|id| id.accesskit_id());
        assert_eq!(
            opener,
            Some(undo_id),
            "{key:?}: the focus goes back to Undo"
        );
    }
    let node = harness.get_by_label("Reset").accesskit_node();
    let show = egui::accesskit::Action::ShowContextMenu;
    assert!(node.data().supports_action(show), "Reset offers its menu");
}

/// Decision 91: on the Mac a Control-click on a button asks for the toolbar menu, as a
/// right-click does, and presses nothing.
#[test]
fn a_control_click_on_the_mac_asks_for_the_toolbar_menu() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1200.0, 120.0))
        .with_os(egui::os::OperatingSystem::Mac)
        .with_context_setup(install)
        .build_ui_state(
            |ui, actions: &mut Vec<ToolbarAction>| {
                let bar = Toolbar::new().entries(toolbar_entries(ui.ctx(), &[]));
                actions.extend(bar.title_bar(true).show(ui));
            },
            Vec::new(),
        );
    harness.run();
    let reset = harness.get_by_label("Reset").rect().center();
    harness.run_steps(30);
    harness.hover_at(reset);
    let control = Modifiers {
        ctrl: true,
        ..Modifiers::NONE
    };
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos: reset,
            button: PointerButton::Primary,
            pressed,
            modifiers: control,
        });
    }
    harness.run();
    assert_eq!(menus_asked(&harness), vec![(Some(3), reset, None)]);
}

/// The title bar with the window's caption buttons, the window maximised or not.
fn caption_buttons(maximised: bool) -> Harness<'static, Vec<ToolbarAction>> {
    Harness::builder()
        .with_size(egui::vec2(1200.0, 120.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(install)
        .build_ui_state(
            move |ui, actions: &mut Vec<ToolbarAction>| {
                if let Some(action) = Toolbar::new()
                    .entries(toolbar_entries(ui.ctx(), &[]))
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
            ToolbarAction::Minimise,
            ToolbarAction::MaximiseOrRestore,
            ToolbarAction::Close,
        ]
    );
    // They are no empty areas: a double-click on one is two clicks, never a zoom.
    harness.state_mut().clear();
    double_click(&mut harness, minimise.center());
    assert_eq!(*harness.state(), vec![ToolbarAction::Minimise; 2]);
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
    assert_eq!(*harness.state(), vec![ToolbarAction::MaximiseOrRestore]);
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
}

/// The end of a Toolbar with caption buttons, in the dark theme, for its snapshots, with
/// room under it for a tooltip.
fn caption_snapshot() -> Harness<'static> {
    Harness::builder()
        .with_size(egui::vec2(700.0, 102.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark)))
        .build_ui(|ui| {
            let panel = apertrust_ui::theme(ui.ctx()).colors.panel;
            ui.painter()
                .rect_filled(ui.ctx().content_rect(), 0.0, panel);
            Toolbar::new()
                .entries(toolbar_entries(ui.ctx(), &[]))
                .labels(true)
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
    actions: Vec<ToolbarAction>,
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
                if let Some(action) = Toolbar::new()
                    .entries(toolbar_entries(ui.ctx(), &[]))
                    .title_bar(true)
                    .show(ui)
                {
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
        vec![ToolbarAction::TitleBarDoubleClick]
    );
}

/// What a Toolbar drawn from a list of entries (decision 92) reports, with the jobs and
/// the save state it is given.
struct Listed {
    entries: Vec<ToolbarEntry>,
    jobs: Vec<Job>,
    saved: SaveState<'static>,
    actions: Vec<ToolbarAction>,
}

/// The Toolbar 1000 points wide drawn from `entries`, as the title bar and with the menus
/// drawn, as on Windows and Linux, and Phase 2's cursors' well.
fn listed(entries: Vec<ToolbarEntry>) -> Harness<'static, Listed> {
    listed_on(entries, 1000.0, egui::os::OperatingSystem::Nix)
}

/// [`listed`]'s Toolbar `width` points wide, on `os`.
fn listed_on(
    entries: Vec<ToolbarEntry>,
    width: f32,
    os: egui::os::OperatingSystem,
) -> Harness<'static, Listed> {
    Harness::builder()
        .with_size(egui::vec2(width, 120.0))
        .with_os(os)
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(install)
        .build_ui_state(
            |ui, listed: &mut Listed| {
                let jobs = listed.jobs.clone();
                let bar = Toolbar::new()
                    .entries(listed.entries.clone())
                    .jobs(Jobs::new(&jobs))
                    .saved(listed.saved)
                    .menu_button(true)
                    .title_bar(true);
                if let Some(action) = with_well(0, |well| bar.cursors(well).show(ui)) {
                    listed.actions.push(action);
                }
            },
            Listed {
                entries,
                jobs: Vec::new(),
                saved: SaveState::Saved,
                actions: Vec::new(),
            },
        )
}

/// Decision 92: a right-click on the menu button asks for the window's menu, never the
/// toolbar menu; a right-click beside it, and Shift+F10 or the Menu key on it, still ask for
/// the toolbar menu.
#[test]
fn a_right_click_on_the_menu_button_asks_for_the_window_s_menu() {
    let mut harness = listed(vec![ToolbarEntry::MenuButton, settings()]);
    harness.run();
    let menu = harness.get_by_role_and_label(Role::Button, "Menu").rect();
    right_click_listed(&mut harness, menu.center());
    assert_eq!(harness.state().actions, [ToolbarAction::WindowMenu]);
    harness.state_mut().actions.clear();
    right_click_listed(&mut harness, menu.right_center() + egui::vec2(60.0, 0.0));
    assert!(matches!(
        harness.state().actions[..],
        [ToolbarAction::ContextMenu(ToolbarMenu { entry: None, .. })]
    ));
    for (modifiers, key) in [(Modifiers::NONE, Key::Menu), (Modifiers::SHIFT, Key::F10)] {
        harness.get_by_role_and_label(Role::Button, "Menu").focus();
        harness.run();
        harness.state_mut().actions.clear();
        harness.key_press_modifiers(modifiers, key);
        harness.run();
        assert!(
            matches!(
                harness.state().actions[..],
                [ToolbarAction::ContextMenu(ToolbarMenu {
                    entry: Some(0),
                    ..
                })]
            ),
            "{key:?}: {:?}",
            harness.state().actions
        );
    }
}

/// A right-click at `pos` on [`listed`]'s Toolbar, as a press and a release.
fn right_click_listed(harness: &mut Harness<'_, Listed>, pos: egui::Pos2) {
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

/// A button of the app's own that is in no default, Settings….
fn settings() -> ToolbarEntry {
    ToolbarEntry::Button(ToolbarButton::new("settings", Icon::Settings, "Settings…"))
}

#[test]
fn the_toolbar_draws_its_entries_in_their_order() {
    let mut harness = listed(vec![
        ToolbarEntry::SaveState,
        settings(),
        ToolbarEntry::Cursors,
        ToolbarEntry::FlexibleSpace,
        ToolbarEntry::MenuButton,
    ]);
    harness.state_mut().saved = SaveState::Failed("disk full");
    harness.run();
    let bar = harness.get_by_role(Role::Toolbar).rect();
    let rect = |name: &str| harness.get_by_label(name).rect();
    let well = harness.get_by_role(Role::RadioGroup).rect();
    // The save warning first, Settings… after it, the well a group's gap, 16 points, on;
    // the menu button alone at the end.
    assert!(rect("Not saved").right() < rect("Settings…").left());
    assert_eq!(well.left() - rect("Settings…").right(), 16.0);
    let menu = harness.get_by_role_and_label(Role::Button, "Menu").rect();
    assert_eq!(bar.right() - menu.right(), 8.0);
    harness.get_by_label("Settings…").click();
    harness.run();
    assert_eq!(harness.state().actions, [ToolbarAction::Button("settings")]);
}

/// A tool's items go under » before the app's own, wherever they stand (decision 92): with
/// Open, the app's, after Reset, a tool's, Reset still goes first after Copy edit, » taking
/// the first one's room, then Open, then Import….
#[test]
fn a_tool_s_items_fold_before_the_app_s_own() {
    let button = |id: &'static str, tool: bool| {
        ToolbarEntry::Button(ToolbarButton::new(id, Icon::Settings, id).folds_first(tool))
    };
    let mut harness = listed(vec![
        button("import", false),
        button("reset", true),
        button("open", false),
        button("copy-edit", true),
        ToolbarEntry::FlexibleSpace,
        ToolbarEntry::Cursors,
        ToolbarEntry::FlexibleSpace,
        ToolbarEntry::SaveState,
    ]);
    let ids = ["import", "reset", "open", "copy-edit"];
    let mut gone: Vec<Vec<&str>> = Vec::new();
    let mut last = ids.to_vec();
    for width in (100..=1000).rev() {
        harness.set_size(egui::vec2(width as f32, 120.0));
        harness.run();
        let now: Vec<&str> = (ids.iter().copied())
            .filter(|id| harness.query_by_label(id).is_some())
            .collect();
        let went: Vec<&str> = last
            .iter()
            .copied()
            .filter(|id| !now.contains(id))
            .collect();
        if !went.is_empty() {
            gone.push(went);
        }
        last = now;
    }
    assert_eq!(
        gone,
        [
            vec!["copy-edit"],
            vec!["reset"],
            vec!["open"],
            vec!["import"]
        ]
    );
}

/// With no menu button listed, the menus hang from the panel's start, where it would
/// stand, so F10 and Alt still open them (decision 92).
#[test]
fn with_no_menu_button_the_menus_hang_from_the_toolbar_s_start() {
    let mut harness = listed(vec![settings()]);
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Menu")
            .is_none()
    );
    let bar = harness.get_by_role(Role::Toolbar).rect();
    let anchor = Toolbar::menu_anchor(&harness.ctx).expect("an anchor");
    assert_eq!(anchor.left(), bar.left() + 8.0);
    assert_eq!(anchor.center().y, bar.center().y);
    // Settings… stands where the menu button would have.
    assert_eq!(
        harness.get_by_label("Settings…").rect().left(),
        anchor.left()
    );
}

/// With no save state listed, a failed write still shows its warning at the panel's end,
/// and the live region stays; Saving… does not show (decision 92).
#[test]
fn with_no_save_state_a_failed_write_still_shows_at_the_end() {
    let mut harness = listed(vec![
        settings(),
        ToolbarEntry::FlexibleSpace,
        ToolbarEntry::Cursors,
    ]);
    harness.state_mut().saved = SaveState::Saving;
    harness.run();
    let words = Translator::new(Locale::EnGb);
    assert!(
        harness
            .query_by_label(&words.text("status-saving"))
            .is_none()
    );
    harness.state_mut().saved = SaveState::Failed("disk full");
    harness.run();
    let bar = harness.get_by_role(Role::Toolbar).rect();
    assert_eq!(save_state(&harness).right(), bar.right() - 8.0);
    // The cursors' well, last in the list, stands a group's gap before its slot at least.
    let well = harness.get_by_role(Role::RadioGroup).rect();
    assert!(save_state(&harness).left() - well.right() >= 8.0);
    assert!(said(&harness).contains("disk full"));
    harness.get_by_label("Not saved").click();
    harness.run();
    assert_eq!(harness.state().actions, [ToolbarAction::NotSaved]);
    // Saved again, it goes, and screen readers hear so.
    harness.state_mut().saved = SaveState::Saved;
    harness.run();
    assert!(harness.query_by_label("Not saved").is_none());
    assert_eq!(said(&harness), words.text("status-saved"));
}

/// With no jobs listed, the spinner shows at the end only while a failed job is left
/// (decision 92).
#[test]
fn with_no_jobs_the_spinner_shows_while_a_failed_job_is_left() {
    let mut harness = listed(vec![settings(), ToolbarEntry::SaveState]);
    let job = |phase| Job::new(1, phase, JobMessage::new("job-thumbnails"));
    harness.state_mut().jobs = vec![job(JobPhase::Running)];
    harness.run();
    let words = Translator::new(Locale::EnGb);
    let name = words.text("jobs-name");
    assert!(
        harness
            .query_by_role_and_label(Role::Button, &name)
            .is_none()
    );
    harness.state_mut().jobs = vec![job(JobPhase::Failed)];
    harness.run();
    let spinner = harness.get_by_role_and_label(Role::Button, &name).rect();
    let bar = harness.get_by_role(Role::Toolbar).rect();
    assert_eq!(spinner.right(), bar.right() - 8.0);
    harness.get_by_role_and_label(Role::Button, &name).click();
    harness.run();
    assert_eq!(harness.state().actions, [ToolbarAction::Jobs]);
}

/// Decision 92: the menu keys on the focused spinner, or on a focused face or chevron of the
/// cursors' well, ask for the toolbar menu on that entry, so Hide reaches them from the keys.
#[test]
fn the_menu_keys_on_the_spinner_and_the_well_ask_for_the_toolbar_menu_on_them() {
    let entries = vec![
        settings(),
        ToolbarEntry::Cursors,
        ToolbarEntry::FlexibleSpace,
        ToolbarEntry::Jobs,
    ];
    let mut harness = listed(entries);
    let job = Job::new(1, JobPhase::Running, JobMessage::new("job-thumbnails"));
    harness.state_mut().jobs = vec![job];
    harness.run();
    let jobs = Translator::new(Locale::EnGb).text("jobs-name");
    // Each reached as the keys reach it: the spinner by itself, the well's face by Tab from
    // Settings…, its chevron by → from the face.
    let reach = |harness: &mut Harness<'_, Listed>, name: &str| {
        if name == jobs {
            harness.get_by_label(name).focus();
            harness.run();
            return;
        }
        harness.get_by_label("Settings…").focus();
        harness.run();
        harness.key_press(Key::Tab);
        harness.run();
        for _ in 0..6 {
            if harness.get_by_label(name).accesskit_node().is_focused() {
                return;
            }
            harness.key_press(Key::ArrowRight);
            harness.run();
        }
        panic!("{name} never takes the focus");
    };
    for (name, entry) in [
        (jobs.as_str(), 3),
        ("Select, Esc", 1),
        ("Pick cursors and settings", 1),
    ] {
        for (modifiers, key) in [(Modifiers::NONE, Key::Menu), (Modifiers::SHIFT, Key::F10)] {
            reach(&mut harness, name);
            harness.state_mut().actions.clear();
            harness.key_press_modifiers(modifiers, key);
            harness.run();
            let menus: Vec<Option<usize>> = (harness.state().actions.iter())
                .filter_map(|action| match action {
                    ToolbarAction::ContextMenu(menu) => Some(menu.entry),
                    _ => None,
                })
                .collect();
            assert_eq!(menus, [Some(entry)], "{key:?} on {name}");
        }
    }
}

/// Decision 91: on the Mac a Control-click on » or on a chevron of the well opens its own
/// menu alone, never the toolbar menu too.
#[test]
fn a_control_click_on_more_or_a_chevron_opens_its_own_menu_alone() {
    let mut harness = listed_on(
        toolbar_entries(&egui::Context::default(), &[]),
        500.0,
        egui::os::OperatingSystem::Mac,
    );
    harness.run();
    let control = Modifiers {
        ctrl: true,
        ..Modifiers::NONE
    };
    for name in ["More actions", "Pick cursors and settings"] {
        let at = harness.get_by_label(name).rect().center();
        harness.run_steps(30);
        harness.hover_at(at);
        for pressed in [true, false] {
            harness.event(egui::Event::PointerButton {
                pos: at,
                button: PointerButton::Primary,
                pressed,
                modifiers: control,
            });
        }
        harness.run();
        let toolbar_menu = (harness.state().actions.iter())
            .any(|action| matches!(action, ToolbarAction::ContextMenu(_)));
        assert!(!toolbar_menu, "{name}: {:?}", harness.state().actions);
        harness.key_press(Key::Escape);
        harness.run();
    }
}

/// With nothing listed, the whole panel is empty area: it takes a double-click and a drag
/// moves the window (decision 92).
#[test]
fn an_empty_toolbar_is_all_empty_area() {
    let mut harness = listed(Vec::new());
    harness.run();
    let bar = harness.get_by_role(Role::Toolbar).rect();
    for at in [bar.left() + 12.0, bar.center().x, bar.right() - 12.0] {
        let point = egui::pos2(at, bar.center().y);
        harness.state_mut().actions.clear();
        harness.run_steps(30);
        harness.hover_at(point);
        for pressed in [true, false, true, false] {
            harness.event(egui::Event::PointerButton {
                pos: point,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            });
        }
        harness.run();
        assert_eq!(
            harness.state().actions,
            [ToolbarAction::TitleBarDoubleClick],
            "{at}"
        );
        // A drag from there moves the window.
        harness.state_mut().actions.clear();
        harness.run_steps(30);
        let press = |harness: &Harness<'_, Listed>, pressed| {
            harness.event(egui::Event::PointerButton {
                pos: point,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            });
        };
        harness.hover_at(point);
        press(&harness, true);
        harness.step();
        for step in 1..=3 {
            harness.hover_at(point - egui::vec2(10.0 * step as f32, 0.0));
            harness.step();
        }
        harness.drop_at(point - egui::vec2(30.0, 0.0));
        harness.run();
        assert_eq!(harness.state().actions, [ToolbarAction::MoveWindow], "{at}");
    }
}

/// The viewer bar with its state applied as the app would.
struct Photo {
    guides: bool,
    zoom: Zoom,
    /// Whether Zoom in and Zoom out have somewhere to go, as the app works them out.
    steps: (bool, bool),
    actions: Vec<ViewerBarAction>,
}

fn viewer_bar() -> Harness<'static, Photo> {
    viewer_bar_in(Theme::default(), egui::vec2(1000.0, 420.0))
}

fn viewer_bar_in(theme: Theme, size: egui::Vec2) -> Harness<'static, Photo> {
    Harness::builder()
        .with_size(size)
        .with_context_setup(move |ctx| apertrust_ui::install(ctx, theme.clone()))
        .build_ui_state(
            |ui, state: &mut Photo| {
                let panel = apertrust_ui::theme(ui.ctx()).colors.panel;
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                if let Some(action) = ViewerBar::new("Image", state.zoom)
                    .toggles(
                        Toggle::new(false, false),
                        Toggle::new(false, true),
                        Toggle::new(false, true),
                        Toggle::new(state.guides, true),
                    )
                    .values(Some([142, 128, 113]))
                    .zoom_enabled(true)
                    .steps(state.steps.0, state.steps.1)
                    .shortcut(ViewerBarAction::Zoom(Zoom::Fit), "Ctrl+0")
                    .shortcut(ViewerBarAction::Zoom(Zoom::Percent(100.0)), "Z")
                    .shortcut(ViewerBarAction::ZoomIn, "Ctrl+=")
                    .shortcut(ViewerBarAction::ZoomOut, "Ctrl+−")
                    .show(ui)
                {
                    match action {
                        ViewerBarAction::Guides => state.guides = !state.guides,
                        ViewerBarAction::Zoom(zoom) => state.zoom = zoom,
                        _ => {}
                    }
                    state.actions.push(action);
                }
            },
            Photo {
                guides: false,
                zoom: Zoom::Fit,
                steps: (true, true),
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
fn the_viewer_bar_toggles_and_shows_the_values() {
    let mut harness = viewer_bar();
    harness.run();
    harness.get_by_label("Guides").click();
    harness.run();
    assert!(harness.state().guides);
    // Disabled: one photo, nothing to show side by side.
    harness.get_by_label("Show all selected photos").click();
    harness.run();
    assert_eq!(harness.state().actions, vec![ViewerBarAction::Guides]);
    assert_eq!(
        harness.get_by_label("Values under the cursor").value(),
        Some("Red 142, green 128, blue 113, grey 130".to_owned())
    );
}

#[test]
fn the_zoom_is_one_button_whose_menu_is_darktable_s_list_then_the_steps() {
    let mut harness = viewer_bar();
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
            ViewerBarAction::Zoom(Zoom::Percent(100.0)),
            ViewerBarAction::Zoom(Zoom::Fill),
            ViewerBarAction::ZoomIn,
        ]
    );
}

#[test]
fn an_in_between_scale_shows_rounded_and_checks_nothing() {
    let mut harness = viewer_bar();
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
fn every_zoom_entry_is_enabled_once_the_view_zooms() {
    let mut harness = viewer_bar();
    for zoom in [Zoom::Small, Zoom::Fit, Zoom::Fill, Zoom::Percent(400.0)] {
        harness.state_mut().zoom = zoom;
        harness.run();
        harness.get_by_role_and_label(Role::Button, "Zoom").click();
        harness.run();
        for name in zoom_lines(&harness) {
            let (enabled, _) = zoom_line(&harness, &name);
            // Zoom out has nowhere to go at Small.
            let expected = !(zoom == Zoom::Small && name == "Zoom out");
            assert_eq!(enabled, expected, "{zoom:?}: {name}");
            // No entry says zoom is still to come.
            let tip = harness
                .get_by(|node| {
                    matches!(node.role(), Role::MenuItem | Role::MenuItemCheckBox)
                        && node.label().as_deref() == Some(name.as_str())
                })
                .accesskit_node()
                .description();
            assert_eq!(tip, None, "{zoom:?}: {name}");
        }
        harness.key_press(Key::Escape);
        harness.run();
    }
    // The app says where the steps have nowhere to go, in order of scale: here at Fill
    // with Fill at 1600%, Zoom in.
    harness.state_mut().zoom = Zoom::Fill;
    harness.state_mut().steps = (false, true);
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Zoom").click();
    harness.run();
    assert_eq!(zoom_line(&harness, "Zoom in"), (false, None));
    assert_eq!(zoom_line(&harness, "Zoom out"), (true, None));
    // A scale chosen goes to the app.
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "200%")
        .click();
    harness.run();
    assert_eq!(
        harness.state().actions,
        [ViewerBarAction::Zoom(Zoom::Percent(200.0))]
    );
}

#[test]
fn rendering_shows_beside_a_picker_s_words() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1000.0, 40.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui(|ui| {
            ViewerBar::new("Image", Zoom::Percent(100.0))
                .zoom_enabled(true)
                .status(Some("Rendering…"))
                .picker("Exposure · Exposure", "Drag an area on the photo.")
                .show(ui);
        });
    harness.run();
    assert!(harness.query_by_label("Rendering…").is_some());
    assert!(harness.query_by_label("Exposure · Exposure").is_some());
}

#[test]
fn a_scale_next_to_100_never_reads_100() {
    let mut harness = viewer_bar();
    for (percent, label) in [(99.6, "99%"), (100.4, "101%"), (100.0, "100%")] {
        harness.state_mut().zoom = Zoom::Percent(percent);
        harness.run();
        let button = harness.get_by_role_and_label(Role::Button, "Zoom");
        assert_eq!(button.accesskit_node().value().as_deref(), Some(label));
    }
}

#[test]
fn the_zoom_s_tooltip_says_how_to_zoom_and_at_100_what_it_shows() {
    let mut harness = viewer_bar();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Zoom").hover();
    harness.run();
    let tip = "Zoom: pinch or Ctrl+scroll; Z for 100%; middle-click cycles 100%, 200% and Fit. \
               Scroll moves a zoomed photo, Shift+scroll sideways";
    assert!(harness.query_by_label(tip).is_some());
    harness.state_mut().zoom = Zoom::Percent(100.0);
    harness.run();
    let exact = format!("At 100% the screen shows exactly what a full-size export will be. {tip}");
    assert!(harness.query_by_label(&exact).is_some());
}

#[test]
fn the_zoom_menu_opens_and_closes_by_keys_and_gives_the_focus_back() {
    let mut harness = viewer_bar();
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
    let mut harness = viewer_bar_in(Theme::builtin(which), size);
    harness.state_mut().zoom = Zoom::Percent(100.0);
    harness.run();
    harness.snapshot(format!("viewer-bar-zoom-{name}"));
    harness.get_by_role_and_label(Role::Button, "Zoom").click();
    harness.run();
    harness.snapshot(format!("viewer-bar-zoom-menu-{name}"));
}

#[test]
fn the_zoom_is_photographed_in_the_dark_theme() {
    photograph_the_zoom(Builtin::Dark, "dark");
}

#[test]
fn the_zoom_is_photographed_in_the_grey_theme() {
    photograph_the_zoom(Builtin::Grey, "grey");
}

#[test]
fn the_zoom_is_photographed_in_the_light_theme() {
    photograph_the_zoom(Builtin::Light, "light");
}

#[test]
fn the_zoom_is_photographed_in_high_contrast() {
    photograph_the_zoom(Builtin::HighContrast, "high-contrast");
}

/// The zoom's button at Small, the fitted frame shown at half, in `which` theme.
fn photograph_the_zoom_at_small(which: Builtin, name: &str) {
    let size = egui::vec2(420.0, 40.0);
    let mut harness = viewer_bar_in(Theme::builtin(which), size);
    harness.state_mut().zoom = Zoom::Small;
    harness.run();
    harness.snapshot(format!("viewer-bar-zoom-small-{name}"));
}

#[test]
fn the_zoom_at_small_is_photographed_in_the_dark_theme() {
    photograph_the_zoom_at_small(Builtin::Dark, "dark");
}

#[test]
fn the_zoom_at_small_is_photographed_in_the_grey_theme() {
    photograph_the_zoom_at_small(Builtin::Grey, "grey");
}

#[test]
fn the_zoom_at_small_is_photographed_in_the_light_theme() {
    photograph_the_zoom_at_small(Builtin::Light, "light");
}

#[test]
fn the_zoom_at_small_is_photographed_in_high_contrast() {
    photograph_the_zoom_at_small(Builtin::HighContrast, "high-contrast");
}

/// The Browser with `count` photos, wide enough to show the first six cells whole, and what
/// was done to them.
fn browser(count: usize) -> Harness<'static, Vec<BrowserAction>> {
    Harness::builder()
        .with_size(egui::vec2(900.0, 200.0))
        .with_context_setup(install)
        .build_ui_state(
            move |ui, actions: &mut Vec<BrowserAction>| {
                let names: Vec<String> = (0..count).map(|i| format!("IMG_{i:04}.ARW")).collect();
                let thumb = |i: usize| Thumb {
                    name: &names[i],
                    edited: false,
                    status: None,
                    selected: false,
                    picture: None,
                    no_preview: false,
                    rating: None,
                };
                if let Some(action) = Browser::new(count, Some(0), thumb).show(ui).action {
                    actions.push(action);
                }
            },
            Vec::new(),
        )
}

#[test]
fn the_browser_opens_and_selects_with_modifiers() {
    let mut harness = browser(20);
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
            BrowserAction::Open(3),
            BrowserAction::SelectRange(5),
            BrowserAction::ToggleSelected(2),
        ]
    );
}

#[test]
fn the_browser_draws_only_what_is_visible() {
    let mut harness = browser(10_000);
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
/// the app lays out the Tools panel: at the width its edge gives, the edge shown last.
fn panel(width: f32, at_start: bool) -> Harness<'static, Panel> {
    Harness::builder()
        .with_size(vec2(1000.0, 300.0))
        // Frames a sixtieth of a second apart, so two clicks make a double-click (0.3 s).
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(install)
        .build_ui_state(
            |ui, state: &mut Panel| {
                let edge = PanelEdge::new("Tools panel width", state.width, 320.0..=600.0)
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
        .get_by_role_and_label(Role::Splitter, "Tools panel width")
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
            .get_by_role_and_label(Role::Splitter, "Tools panel width")
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
            .get_by_role_and_label(Role::Splitter, "Tools panel width")
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
            .get_by_role_and_label(Role::Splitter, "Tools panel width")
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
    let splitter = harness.get_by_role_and_label(Role::Splitter, "Tools panel width");
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
    let splitter = harness.get_by_role_and_label(Role::Splitter, "Tools panel width");
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
            .get_by_role_and_label(Role::Splitter, "Tools panel width")
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
                .get_by_role_and_label(Role::Splitter, "Tools panel width")
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
            Toolbar::new()
                .entries(toolbar_entries(ui.ctx(), &["redo"]))
                .labels(true)
                .saved(SaveState::Saved)
                .show(ui);
            ViewerBar::new("Image", Zoom::Fit)
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

/// While a colour picker is on, the viewer bar says which it is and how to use it, after
/// the layer's buttons, in place of the values under the cursor (`docs/pickers.md`, On the
/// photo); where how does not fit, the name alone.
#[test]
fn a_picker_s_words_take_the_values_place() {
    const NAME: &str = "Exposure · Exposure";
    const HOW: &str = "Drag an area on the photo. Ctrl-click switches to a point. Esc ends.";
    for (width, how) in [(1000.0, true), (520.0, false)] {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(width, 60.0))
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
            .build_ui(|ui| {
                ViewerBar::new("Image", Zoom::Fit)
                    .values(Some([142, 128, 113]))
                    .picker(NAME, HOW)
                    .show(ui);
            });
        harness.run();
        let name = harness.get_by_role_and_label(Role::Label, NAME);
        assert!(name.rect().left() > harness.get_by_label("New layer").rect().right());
        assert_eq!(
            harness.query_by_role_and_label(Role::Label, HOW).is_some(),
            how,
            "{width}"
        );
        assert!(harness.query_by_label("Values under the cursor").is_none());
    }
}
