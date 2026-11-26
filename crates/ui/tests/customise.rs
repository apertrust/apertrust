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

//! The Toolbar's edit state, as its customise mode draws it (decision 92; `docs/gui.md`,
//! The customise mode): driven through AccessKit, the pointer and the keys, and photographed
//! at a window's widths.

use apertrust_ui::components::{
    CaptionStyle, CursorWell, EditMenu, Jobs, SaveState, Toolbar, ToolbarAction, ToolbarEdit,
    ToolbarEntry, ToolbarSpot, WellCursor, WellGroup,
};
use apertrust_ui::gallery::{customising, toolbar_entries};
use apertrust_ui::{Builtin, Icon, Theme};
use egui::accesskit::{Action, ActionData, ActionRequest, Role};
use egui::{Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

/// What a harness draws and what it was told.
struct Bench {
    labels: bool,
    selected: Option<usize>,
    /// Seconds since the mode started, and whether motion is reduced.
    since: f64,
    still: bool,
    empty: bool,
    menu: bool,
    caption: Option<CaptionStyle>,
    customised: bool,
    actions: Vec<ToolbarAction>,
}

impl Default for Bench {
    fn default() -> Self {
        Bench {
            labels: true,
            selected: None,
            since: 1.0,
            still: false,
            empty: false,
            menu: false,
            caption: None,
            customised: true,
            actions: Vec::new(),
        }
    }
}

impl Bench {
    fn edits(&self) -> Vec<ToolbarEdit> {
        (self.actions.iter())
            .filter_map(|action| match action {
                ToolbarAction::Edit(edit) => Some(*edit),
                _ => None,
            })
            .collect()
    }
}

/// Phase 2's well, Select chosen, drawn by `draw`.
fn with_well<R>(draw: impl FnOnce(CursorWell<'_>) -> R) -> R {
    let select = [WellCursor::new(Icon::Select, "Select").key("Esc")];
    let pan = [WellCursor::new(Icon::Pan, "Pan").key("A")];
    let groups = [
        WellGroup::new("Select", &select, 0),
        WellGroup::new("Pan", &pan, 0),
    ];
    draw(CursorWell::new(&groups))
}

/// The default list in its edit state, `width` points wide, in theme `which`.
fn edit_in(which: Builtin, width: f32, bench: Bench) -> Harness<'static, Bench> {
    let height = Toolbar::height(&Theme::builtin(which), true) + 40.0;
    Harness::builder()
        .with_size(egui::vec2(width, height))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(move |ctx| apertrust_ui::install(ctx, Theme::builtin(which)))
        .build_ui_state(
            |ui, bench: &mut Bench| {
                let panel = apertrust_ui::theme(ui.ctx()).colors.panel;
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                let entries = if bench.empty {
                    Vec::new()
                } else {
                    toolbar_entries(ui.ctx(), &[])
                };
                let edit = customising(ui.ctx(), &entries, bench.selected)
                    .since(bench.since, bench.still)
                    .keep_focus(true);
                let mut bar = Toolbar::new()
                    .entries(entries)
                    .labels(bench.labels)
                    .saved(SaveState::Saved)
                    .jobs(Jobs::new(&[]))
                    .title_bar(true)
                    .menu_button(bench.menu)
                    .caption_buttons(bench.caption);
                if bench.customised {
                    bar = bar.customising(edit);
                }
                if let Some(action) = with_well(|well| bar.cursors(well).show(ui)) {
                    bench.actions.push(action);
                }
            },
            bench,
        )
}

fn edit(bench: Bench) -> Harness<'static, Bench> {
    edit_in(Builtin::Dark, 1440.0, bench)
}

/// The entries' options as screen readers find them, in the tree's order.
fn options(harness: &Harness<'_, Bench>) -> Vec<(String, egui::Rect)> {
    let list = harness.get_by_role(Role::ListBox);
    let found: Vec<(String, egui::Rect)> = (list.query_all_by_role(Role::ListBoxOption))
        .map(|node| {
            (
                node.accesskit_node().label().unwrap_or_default(),
                node.rect(),
            )
        })
        .collect();
    found
}

fn option<'a>(harness: &'a Harness<'_, Bench>, name: &'a str) -> egui_kittest::Node<'a> {
    harness.get_by_role_and_label(Role::ListBoxOption, name)
}

fn button(harness: &Harness<'_, Bench>, pos: Pos2, which: PointerButton, pressed: bool) {
    harness.event(egui::Event::PointerButton {
        pos,
        button: which,
        pressed,
        modifiers: Modifiers::NONE,
    });
}

fn right_click(harness: &mut Harness<'_, Bench>, pos: Pos2) {
    harness.run_steps(30);
    harness.hover_at(pos);
    button(harness, pos, PointerButton::Secondary, true);
    button(harness, pos, PointerButton::Secondary, false);
    harness.run();
}

/// Presses at `from`, moves to `to` a frame at a time, and lets go there.
fn drag(harness: &mut Harness<'_, Bench>, from: Pos2, to: Pos2) {
    harness.hover_at(from);
    button(harness, from, PointerButton::Primary, true);
    harness.step();
    for step in 1..=10 {
        harness.hover_at(from + (to - from) * (step as f32 / 10.0));
        harness.step();
    }
    harness.drop_at(to);
    harness.run();
}

fn key(harness: &mut Harness<'_, Bench>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

/// Decision 92: the edit state is a horizontal list box named Toolbar, each entry an option
/// named by its words, with its place and count and its selection; Done a default button.
#[test]
fn screen_readers_meet_a_list_box_of_options_and_done() {
    let mut harness = edit(Bench {
        selected: Some(3),
        ..Bench::default()
    });
    harness.run();
    let list = harness.get_by_role(Role::ListBox);
    assert_eq!(list.accesskit_node().label().as_deref(), Some("Toolbar"));
    let names: Vec<String> = options(&harness)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(
        names,
        [
            "Import…",
            "Export…",
            "Reset",
            "Undo",
            "Redo",
            "Flexible space",
            "Cursors",
            "Flexible space",
            "Guides",
            "Space",
            "Copy edit",
            "Apply edit",
            "Jobs",
            "Save state",
        ]
    );
    let undo = option(&harness, "Undo");
    // AccessKit's are zero-based, the size on the list box.
    assert_eq!(undo.accesskit_node().position_in_set(), Some(3));
    assert_eq!(list.accesskit_node().size_of_set(), Some(14));
    assert_eq!(undo.accesskit_node().is_selected(), Some(true));
    assert_eq!(
        option(&harness, "Redo").accesskit_node().is_selected(),
        Some(false)
    );
    harness.get_by_role_and_label(Role::DefaultButton, "Done");
    // The Toolbar at rest keeps its own role.
    harness.state_mut().customised = false;
    harness.run();
    assert!(harness.query_by_role(Role::ListBox).is_none());
    harness.get_by_role(Role::Toolbar);
}

/// An empty Toolbar is the bare bar with Done, named "Toolbar, empty".
#[test]
fn an_empty_toolbar_is_named_so() {
    let mut harness = edit(Bench {
        empty: true,
        ..Bench::default()
    });
    harness.run();
    let list = harness.get_by_role(Role::ListBox);
    assert_eq!(
        list.accesskit_node().label().as_deref(),
        Some("Toolbar, empty")
    );
    assert!(options(&harness).is_empty());
    harness.get_by_role_and_label(Role::DefaultButton, "Done");
}

/// Each key reports its edit on the focused line: ← → Home End select, Alt with the arrows
/// moves, Delete and Backspace remove, Shift+F10 and the Menu key ask for the menu; Enter is
/// Done and Esc Cancel.
#[test]
fn each_key_reports_its_edit() {
    let cases: [(Option<usize>, Modifiers, Key, ToolbarEdit); 12] = [
        (
            None,
            Modifiers::NONE,
            Key::ArrowRight,
            ToolbarEdit::Select(0),
        ),
        (
            None,
            Modifiers::NONE,
            Key::ArrowLeft,
            ToolbarEdit::Select(13),
        ),
        (
            Some(3),
            Modifiers::NONE,
            Key::ArrowLeft,
            ToolbarEdit::Select(2),
        ),
        (
            Some(3),
            Modifiers::NONE,
            Key::ArrowRight,
            ToolbarEdit::Select(4),
        ),
        (Some(3), Modifiers::NONE, Key::Home, ToolbarEdit::Select(0)),
        (Some(3), Modifiers::NONE, Key::End, ToolbarEdit::Select(13)),
        (
            Some(3),
            Modifiers::ALT,
            Key::ArrowLeft,
            ToolbarEdit::Move { from: 3, to: 2 },
        ),
        (
            Some(3),
            Modifiers::ALT,
            Key::ArrowRight,
            ToolbarEdit::Move { from: 3, to: 4 },
        ),
        (
            Some(3),
            Modifiers::NONE,
            Key::Delete,
            ToolbarEdit::Remove(3),
        ),
        (
            Some(3),
            Modifiers::NONE,
            Key::Backspace,
            ToolbarEdit::Remove(3),
        ),
        (None, Modifiers::NONE, Key::Enter, ToolbarEdit::Done),
        (None, Modifiers::NONE, Key::Escape, ToolbarEdit::Cancel),
    ];
    for (selected, modifiers, pressed, expected) in cases {
        let mut harness = edit(Bench {
            selected,
            ..Bench::default()
        });
        harness.run();
        key(&mut harness, modifiers, pressed);
        assert_eq!(
            harness.state().edits(),
            [expected],
            "{modifiers:?} {pressed:?}"
        );
    }
    for (modifiers, pressed) in [(Modifiers::SHIFT, Key::F10), (Modifiers::NONE, Key::Menu)] {
        let mut harness = edit(Bench {
            selected: Some(3),
            ..Bench::default()
        });
        harness.run();
        key(&mut harness, modifiers, pressed);
        let at: Vec<ToolbarSpot> = (harness.state().edits().into_iter())
            .filter_map(|edit| match edit {
                ToolbarEdit::Menu(EditMenu { at, .. }) => Some(at),
                _ => None,
            })
            .collect();
        assert_eq!(at, [ToolbarSpot::Entry(3)], "{pressed:?}");
    }
}

/// A click selects; a right-click on an item asks for the menu on it, on the empty line in
/// the gap under the pointer, on the cursors' well the customise menu, never the well's own.
#[test]
fn clicks_select_and_right_clicks_ask_for_the_menu() {
    let mut harness = edit(Bench::default());
    harness.run();
    option(&harness, "Undo").click();
    harness.run();
    assert_eq!(harness.state().edits(), [ToolbarEdit::Select(3)]);
    harness.state_mut().actions.clear();
    let undo = option(&harness, "Undo").rect();
    right_click(&mut harness, undo.center());
    let spots: Vec<ToolbarSpot> = (harness.state().edits().into_iter())
        .filter_map(|edit| match edit {
            ToolbarEdit::Menu(menu) => Some(menu.at),
            _ => None,
        })
        .collect();
    assert_eq!(spots, [ToolbarSpot::Entry(3)]);
    // The 4 points between Undo's and Redo's plates: the gap before Redo.
    let redo = option(&harness, "Redo").rect();
    harness.state_mut().actions.clear();
    right_click(
        &mut harness,
        egui::pos2((undo.right() + redo.left()) / 2.0, undo.center().y),
    );
    assert!(
        matches!(
            harness.state().edits()[..],
            [ToolbarEdit::Menu(EditMenu {
                at: ToolbarSpot::Gap(4),
                ..
            })]
        ),
        "{:?}",
        harness.state().actions
    );
    let well = option(&harness, "Cursors").rect();
    harness.state_mut().actions.clear();
    right_click(&mut harness, well.center());
    assert!(
        matches!(
            harness.state().actions[..],
            [ToolbarAction::Edit(ToolbarEdit::Menu(EditMenu {
                at: ToolbarSpot::Entry(6),
                ..
            }))]
        ),
        "{:?}",
        harness.state().actions
    );
}

/// The menu button stands at the start, outside the list: no option, and a right-click on
/// it asks for the menu at the list's start, never the window's.
#[test]
fn the_menu_button_is_no_option_and_opens_the_menu_at_the_start() {
    let mut harness = edit(Bench {
        menu: true,
        ..Bench::default()
    });
    harness.run();
    assert!(harness.query_by_label("Menu").is_none());
    let import = option(&harness, "Import…").rect();
    right_click(
        &mut harness,
        egui::pos2(import.left() - 20.0, import.center().y),
    );
    assert!(
        matches!(
            harness.state().actions[..],
            [ToolbarAction::Edit(ToolbarEdit::Menu(EditMenu {
                at: ToolbarSpot::Gap(0),
                ..
            }))]
        ),
        "{:?}",
        harness.state().actions
    );
}

/// A drag along the bar reports a move to the gap under the release; one let go off the bar
/// reports nothing; Esc mid-drag cancels the drag alone.
#[test]
fn a_drag_moves_an_item_to_the_gap_under_its_release() {
    let mut harness = edit(Bench::default());
    harness.run();
    let undo = option(&harness, "Undo").rect();
    let guides = option(&harness, "Guides").rect();
    // Let go just before Guides' centre: the gap before Guides, place 8.
    drag(
        &mut harness,
        undo.center(),
        egui::pos2(guides.center().x - 4.0, guides.center().y),
    );
    assert_eq!(
        harness.state().edits(),
        [ToolbarEdit::Move { from: 3, to: 7 }]
    );
    harness.state_mut().actions.clear();
    drag(
        &mut harness,
        undo.center(),
        egui::pos2(guides.center().x, guides.bottom() + 30.0),
    );
    assert!(
        harness.state().edits().is_empty(),
        "{:?}",
        harness.state().actions
    );
    // Esc during a drag: no Cancel, and the release does nothing.
    let from = undo.center();
    harness.hover_at(from);
    button(&harness, from, PointerButton::Primary, true);
    harness.step();
    for step in 1..=5 {
        harness.hover_at(from + egui::vec2(20.0 * step as f32, 0.0));
        harness.step();
    }
    harness.key_press(Key::Escape);
    harness.step();
    harness.drop_at(from + egui::vec2(100.0, 0.0));
    harness.run();
    assert!(
        harness.state().edits().is_empty(),
        "{:?}",
        harness.state().actions
    );
}

/// No part of the edit state moves the window or takes a double-click; the caption buttons
/// still report.
#[test]
fn no_part_moves_the_window_and_the_caption_buttons_still_report() {
    let mut harness = edit(Bench {
        caption: Some(CaptionStyle::Linux),
        ..Bench::default()
    });
    harness.run();
    let flexible = harness
        .get_all_by_role_and_label(Role::ListBoxOption, "Flexible space")
        .next()
        .unwrap()
        .rect();
    drag(
        &mut harness,
        flexible.center(),
        flexible.center() + egui::vec2(0.0, 200.0),
    );
    harness.run_steps(30);
    for pressed in [true, false, true, false] {
        button(&harness, flexible.center(), PointerButton::Primary, pressed);
    }
    harness.run();
    let window = harness.state().actions.iter().any(|action| {
        matches!(
            action,
            ToolbarAction::MoveWindow
                | ToolbarAction::TitleBarDoubleClick
                | ToolbarAction::WindowMenu
        )
    });
    assert!(!window, "{:?}", harness.state().actions);
    harness.state_mut().actions.clear();
    harness
        .get_by_role_and_label(Role::Button, "Minimise")
        .click();
    harness.run();
    assert_eq!(harness.state().actions, [ToolbarAction::Minimise]);
}

/// A screen reader's custom actions do what the keys do, and ShowContextMenu asks for the
/// menu on the option.
#[test]
fn custom_actions_move_and_remove() {
    let mut harness = edit(Bench::default());
    harness.run();
    for (custom, expected) in [
        (0, ToolbarEdit::Move { from: 3, to: 2 }),
        (1, ToolbarEdit::Move { from: 3, to: 4 }),
        (2, ToolbarEdit::Remove(3)),
    ] {
        harness.state_mut().actions.clear();
        let id = option(&harness, "Undo").accesskit_node().locate().0;
        harness.event(egui::Event::AccessKitActionRequest(ActionRequest {
            action: Action::CustomAction,
            target_tree: egui::accesskit::TreeId::ROOT,
            target_node: id,
            data: Some(ActionData::CustomAction(custom)),
        }));
        harness.run();
        assert_eq!(harness.state().edits(), [expected]);
    }
    harness.state_mut().actions.clear();
    let id = option(&harness, "Redo").accesskit_node().locate().0;
    harness.event(egui::Event::AccessKitActionRequest(ActionRequest {
        action: Action::ShowContextMenu,
        target_tree: egui::accesskit::TreeId::ROOT,
        target_node: id,
        data: None,
    }));
    harness.run();
    assert!(matches!(
        harness.state().edits()[..],
        [ToolbarEdit::Menu(EditMenu {
            at: ToolbarSpot::Entry(4),
            ..
        })]
    ));
}

/// The wiggle: entries offset mid-wiggle, none once `wiggle_ms` has passed, none with
/// reduced motion; Done never moves.
#[test]
fn the_wiggle_moves_entries_once_and_never_done() {
    let place = |since: f64, still: bool| {
        let mut harness = edit(Bench {
            since,
            still,
            ..Bench::default()
        });
        harness.run_steps(2);
        let undo = option(&harness, "Undo").rect().left();
        let redo = option(&harness, "Redo").rect().left();
        let done = harness.get_by_label("Done").rect().left();
        (undo, redo, done)
    };
    let rest = place(1.0, false);
    let mid = place(0.04, false);
    assert_ne!(mid.0, rest.0, "Undo still mid-wiggle");
    // Neighbours in opposite phase.
    assert!(
        (mid.0 - rest.0) * (mid.1 - rest.1) < 0.0,
        "{mid:?} {rest:?}"
    );
    assert!((mid.0 - rest.0).abs() <= 1.5);
    assert_eq!(mid.2, rest.2);
    assert_eq!(place(0.04, true), rest);
}

/// At 760 points the edit state never folds: every entry is an option, the marked ones
/// exactly those the Toolbar at rest puts under ».
#[test]
fn a_narrow_toolbar_marks_what_more_takes_at_rest() {
    let mut harness = edit_in(
        Builtin::Dark,
        760.0,
        Bench {
            customised: false,
            caption: Some(CaptionStyle::Linux),
            menu: true,
            ..Bench::default()
        },
    );
    harness.run();
    let shown: Vec<String> = (["Import…", "Export…", "Reset", "Undo", "Redo", "Guides"].iter())
        .chain(&["Copy edit", "Apply edit"])
        .filter(|name| {
            harness
                .query_by_role_and_label(Role::Button, name)
                .is_some()
        })
        .map(|name| (*name).to_owned())
        .collect();
    harness.state_mut().customised = true;
    harness.run();
    assert_eq!(options(&harness).len(), 14);
    let marked: Vec<String> = (harness.query_all_by_role(Role::ListBoxOption))
        .filter(|node| {
            let description = node.accesskit_node().description();
            description.is_some_and(|d| d.contains("Under » at this width"))
        })
        .map(|node| node.accesskit_node().label().unwrap_or_default())
        .collect();
    assert!(!marked.is_empty(), "nothing folds at 960 points");
    for name in &marked {
        assert!(!shown.contains(name), "{name} is shown at rest");
    }
    for name in [
        "Import…",
        "Export…",
        "Reset",
        "Undo",
        "Redo",
        "Guides",
        "Copy edit",
        "Apply edit",
    ] {
        assert!(
            shown.iter().any(|s| s == name) || marked.iter().any(|m| m == name),
            "{name} neither shown at rest nor marked"
        );
    }
}

/// The edit state photographed: the default list at 1440 points in the dark and light
/// themes, labels on and off, Undo selected; at 960, scrolled; and empty.
#[test]
fn the_edit_state_is_photographed() {
    let shots: [(&str, Builtin, f32, Bench); 5] = [
        (
            "toolbar-customising-dark",
            Builtin::Dark,
            1440.0,
            Bench {
                selected: Some(3),
                ..Bench::default()
            },
        ),
        (
            "toolbar-customising-light",
            Builtin::Light,
            1440.0,
            Bench {
                selected: Some(3),
                ..Bench::default()
            },
        ),
        (
            "toolbar-customising-labels-off-dark",
            Builtin::Dark,
            1440.0,
            Bench {
                labels: false,
                caption: Some(CaptionStyle::Windows),
                menu: true,
                ..Bench::default()
            },
        ),
        (
            "toolbar-customising-960-dark",
            Builtin::Dark,
            960.0,
            Bench {
                caption: Some(CaptionStyle::Linux),
                menu: true,
                ..Bench::default()
            },
        ),
        (
            "toolbar-customising-empty-dark",
            Builtin::Dark,
            960.0,
            Bench {
                empty: true,
                ..Bench::default()
            },
        ),
    ];
    let mut results = egui_kittest::SnapshotResults::new();
    for (name, which, width, bench) in shots {
        let mut harness = edit_in(which, width, bench);
        harness.run();
        harness.snapshot(name);
        results.extend_harness(&mut harness);
    }
    results.unwrap();
}

/// A drag under way photographed: Undo lifted over the gap before Guides, its place dimmed,
/// the insertion line and what a release does.
#[test]
fn a_drag_is_photographed() {
    let mut harness = edit(Bench::default());
    harness.run();
    let undo = option(&harness, "Undo").rect();
    let guides = option(&harness, "Guides").rect();
    let to = egui::pos2(guides.center().x - 4.0, guides.center().y);
    harness.hover_at(undo.center());
    button(&harness, undo.center(), PointerButton::Primary, true);
    harness.step();
    for step in 1..=10 {
        harness.hover_at(undo.center() + (to - undo.center()) * (step as f32 / 10.0));
        harness.step();
    }
    harness.run_steps(4);
    harness.snapshot("toolbar-customising-dragging-dark");
}

/// The one-off pieces the entries draw from are what the app gives: an entry's tooltip
/// is its words then how to move it.
#[test]
fn an_entry_s_tooltip_says_how_to_move_it() {
    let mut harness = edit(Bench::default());
    harness.run();
    option(&harness, "Undo").hover();
    harness.run_steps(60);
    assert!(
        harness
            .query_by_label_contains("Drag to move. Right-click for more")
            .is_some()
    );
}

/// A fixed space the Toolbar at rest skips, at the list's start with no menu button, carries
/// the fold mark however wide the bar; one between two entries drawn does not.
#[test]
fn a_space_the_toolbar_at_rest_skips_is_marked() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1440.0, 100.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui(|ui| {
            let entries = vec![
                ToolbarEntry::Space,
                toolbar_entries(ui.ctx(), &[]).remove(3),
                ToolbarEntry::Space,
                toolbar_entries(ui.ctx(), &[]).remove(4),
            ];
            let edit = customising(ui.ctx(), &entries, None);
            Toolbar::new().entries(entries).customising(edit).show(ui);
        });
    harness.run();
    let marks: Vec<bool> = (harness.query_all_by_role(Role::ListBoxOption))
        .map(|node| {
            let description = node.accesskit_node().description();
            description.is_some_and(|d| d.contains("Under » at this width"))
        })
        .collect();
    assert_eq!(marks, [true, false, false, false]);
}

/// Where the line does not fit, the wheel scrolls it: the entries move, Done stays.
#[test]
fn the_wheel_scrolls_a_narrow_line_and_done_stays() {
    let mut harness = edit_in(Builtin::Dark, 760.0, Bench::default());
    harness.run();
    let place = |harness: &Harness<'_, Bench>| {
        let undo = option(harness, "Undo").rect().left();
        let done = harness.get_by_label("Done").rect().left();
        (undo, done)
    };
    let before = place(&harness);
    let over = option(&harness, "Undo").rect().center();
    harness.hover_at(over);
    harness.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(-120.0, 0.0),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
    });
    harness.run_steps(30);
    let after = place(&harness);
    assert!(after.0 < before.0, "{before:?} {after:?}");
    assert_eq!(after.1, before.1);
}
