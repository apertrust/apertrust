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

//! The top panel's ☰ and the drawn menu it opens through the popup service (`docs/gui.md`,
//! The menu bar, The drawn menus; decision 39), driven by the pointer and the keys and
//! read through AccessKit, as a screen reader reads them.

use apertrust_ui::components::{TopPanel, TopPanelAction};
use apertrust_ui::popup::{self, Answer, Entry, Item, Request, Submenu};
use apertrust_ui::{Locale, Theme};
use egui::accesskit::{HasPopup, Role, Toggled};
use egui::{Event, Key, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

/// The top panel with or without the ☰, opening the menu as the app would, and what came
/// back.
struct App {
    menu_button: bool,
    actions: Vec<TopPanelAction>,
    answers: Vec<Answer>,
    /// Open the menu on the next frame, at its top or with this menu expanded, as F10 and
    /// Alt with an access key will.
    open_at: Option<Option<&'static str>>,
    /// Whether the frame draws the popups, as the app must.
    show: bool,
    /// The key presses and text that reached the app's own code.
    seen: Vec<Event>,
    /// Answers read a second time in a frame, which must never come.
    twice: Vec<Answer>,
}

fn entries() -> Vec<Entry> {
    vec![
        Submenu::new(
            "file",
            "&File",
            vec![
                Item::new("open", "Open…").shortcut("Ctrl+O").into(),
                Submenu::new(
                    "recent",
                    "Open Recent",
                    vec![
                        Item::new("recent-1", "Holidays").into(),
                        Item::new("recent-2", "Studio").into(),
                    ],
                )
                .into(),
                Entry::Separator,
                Item::new("export", "Export…")
                    .shortcut("Ctrl+E")
                    .enabled(false)
                    .into(),
                Entry::Separator,
                Item::new("quit", "Quit").shortcut("Ctrl+Q").into(),
            ],
        )
        .into(),
        Submenu::new(
            "edit",
            "&Edit",
            vec![
                Item::new("undo", "Undo").into(),
                Item::new("redo", "Redo").enabled(false).into(),
                Item::new("rock", "Rock & Roll").into(),
                Submenu::new(
                    "tips",
                    "Tips & Tricks",
                    vec![Item::new("tip", "Tip").into()],
                )
                .into(),
            ],
        )
        .into(),
        Entry::Separator,
        Submenu::new(
            "view",
            "&View",
            vec![
                Item::new("guides", "Guides").checked(true).into(),
                Item::new("proof", "Proof Margin").checked(false).into(),
            ],
        )
        .into(),
        Submenu::new(
            "help",
            "&Help",
            vec![
                Item::new("about", "About").into(),
                // Option (Alt) shows Truncate in its place, as macOS's alternate items do.
                Item::new("compress", "Compress")
                    .alternate(Item::new("truncate", "Truncate the Whole Log").tip("Drops"))
                    .into(),
            ],
        )
        .into(),
    ]
}

fn app(menu_button: bool) -> Harness<'static, App> {
    app_in(Locale::EnGb, menu_button)
}

fn app_in(locale: Locale, menu_button: bool) -> Harness<'static, App> {
    Harness::builder()
        .with_size(vec2(1000.0, 400.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::default());
            apertrust_ui::set_locale(ctx, locale);
            ctx.global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
        })
        .build_ui_state(
            |ui, app: &mut App| {
                app.seen.extend(ui.input(|i| {
                    i.events
                        .iter()
                        .filter(|e| matches!(e, Event::Key { pressed: true, .. } | Event::Text(_)))
                        .cloned()
                        .collect::<Vec<_>>()
                }));
                let action = TopPanel::new()
                    .menu_button(app.menu_button)
                    .title_bar(true)
                    .caption_buttons(true)
                    .shortcut(TopPanelAction::Menu, "F10")
                    .show(ui);
                let ctx = ui.ctx().clone();
                if let Some(action) = action {
                    app.actions.push(action);
                    if action == TopPanelAction::Menu {
                        app.open_at = Some(None);
                    }
                }
                if let Some(at) = app.open_at.take()
                    && let Some(anchor) = TopPanel::menu_anchor(&ctx)
                {
                    let mut request = Request::pull_down(anchor, "Menu", entries());
                    if let Some(id) = at {
                        request = request.expanded(id);
                    }
                    popup::open(&ctx, TopPanel::menu_popup(), request);
                }
                if app.show {
                    popup::show(&ctx);
                }
                if let Some(answer) = popup::answer(&ctx, TopPanel::menu_popup()) {
                    app.answers.push(answer);
                }
                if let Some(answer) = popup::answer(&ctx, TopPanel::menu_popup()) {
                    app.twice.push(answer);
                }
            },
            App {
                menu_button,
                actions: Vec::new(),
                answers: Vec::new(),
                open_at: None,
                show: true,
                seen: Vec::new(),
                twice: Vec::new(),
            },
        )
}

/// The menu named `name` is open.
fn menu_open(harness: &Harness<'_, App>, name: &str) -> bool {
    harness.query_by_role_and_label(Role::Menu, name).is_some()
}

/// The name of the line with the focus.
fn focused(harness: &Harness<'_, App>) -> String {
    harness
        .get_by(|node| node.is_focused())
        .accesskit_node()
        .label()
        .unwrap_or_default()
}

fn open(harness: &mut Harness<'_, App>, at: Option<&'static str>) {
    harness.state_mut().open_at = Some(at);
    harness.run();
}

fn key(harness: &mut Harness<'_, App>, key: Key) {
    harness.key_press(key);
    harness.run();
}

#[test]
fn the_menu_button_comes_first_and_reports_menu() {
    let mut harness = app(true);
    harness.run();
    let bar = harness.get_by_role(Role::Toolbar).rect();
    let button = harness.get_by_role_and_label(Role::Button, "Menu");
    let rect = button.rect();
    let mark = harness
        .get_by_role_and_label(Role::Button, "About Apertrust")
        .rect();
    // At the panel's start, a button as large as the others, as far from the mark as the
    // mark is from Import.
    assert_eq!(rect.left() - bar.left(), 8.0);
    assert_eq!(rect.size(), vec2(30.0, 30.0));
    let import = harness.get_by_label("Import…").rect();
    assert_eq!(mark.left() - rect.right(), 8.0);
    assert_eq!(import.left() - mark.right(), 8.0);
    // A button that opens a menu, with its keys, and its name in its tooltip with them.
    let node = button.accesskit_node();
    assert_eq!(node.data().has_popup(), Some(HasPopup::Menu));
    assert_eq!(node.data().is_expanded(), Some(false));
    assert_eq!(node.toggled(), None);
    assert_eq!(node.data().keyboard_shortcut(), Some("F10"));
    button.hover();
    harness.run();
    assert!(harness.query_by_label("Menu (F10)").is_some());
    harness.get_by_role_and_label(Role::Button, "Menu").click();
    harness.run();
    assert_eq!(harness.state().actions, vec![TopPanelAction::Menu]);
}

#[test]
fn without_the_option_there_is_no_menu_button() {
    let mut harness = app(false);
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Menu")
            .is_none()
    );
    let bar = harness.get_by_role(Role::Toolbar).rect();
    let mark = harness
        .get_by_role_and_label(Role::Button, "About Apertrust")
        .rect();
    assert_eq!(mark.left() - bar.left(), 8.0);
}

#[test]
fn the_menu_button_is_named_in_every_language() {
    for (locale, name) in [(Locale::Uk, "Меню"), (Locale::Pl, "Menu")] {
        let mut harness = app_in(locale, true);
        harness.run();
        assert!(
            harness
                .query_by_role_and_label(Role::Button, name)
                .is_some(),
            "{locale:?}"
        );
    }
}

#[test]
fn the_menu_opens_at_its_top_with_the_focus_on_its_first_line() {
    let mut harness = app(true);
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Menu").click();
    harness.run();
    assert!(menu_open(&harness, "Menu"));
    assert_eq!(focused(&harness), "File");
    assert!(!menu_open(&harness, "File"));
    // Under the button, which shows it open.
    let button = harness.get_by_role_and_label(Role::Button, "Menu");
    let file = harness.get_by_role_and_label(Role::MenuItem, "File").rect();
    assert!(file.top() > button.rect().bottom());
    assert!((file.left() - button.rect().left()).abs() <= 6.0);
    assert_eq!(button.accesskit_node().data().is_expanded(), Some(true));
    assert_eq!(button.accesskit_node().toggled(), None);
}

#[test]
fn the_menu_button_takes_down_to_open_it() {
    let mut harness = app(true);
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Menu").focus();
    harness.run();
    key(&mut harness, Key::ArrowDown);
    assert_eq!(harness.state().actions, vec![TopPanelAction::Menu]);
    assert_eq!(focused(&harness), "File");
}

#[test]
fn the_menu_opens_with_a_menu_expanded() {
    let mut harness = app(true);
    harness.run();
    open(&mut harness, Some("view"));
    assert!(menu_open(&harness, "Menu"));
    assert!(menu_open(&harness, "View"));
    assert_eq!(focused(&harness), "Guides");
    // The menu sits beside its line, and its line says it is open.
    let line = harness.get_by_role_and_label(Role::MenuItem, "View");
    assert_eq!(line.accesskit_node().data().is_expanded(), Some(true));
    let guides = harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "Guides")
        .rect();
    assert!(guides.left() >= line.rect().right());
    assert_eq!(guides.top(), line.rect().top());
}

#[test]
fn the_arrows_move_through_the_lines_and_into_and_out_of_submenus() {
    let mut harness = app(true);
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Menu").focus();
    harness.run();
    open(&mut harness, None);
    // Round from the top to the last line, and back.
    key(&mut harness, Key::ArrowUp);
    assert_eq!(focused(&harness), "Help");
    key(&mut harness, Key::ArrowDown);
    assert_eq!(focused(&harness), "File");
    // Past the separator between Edit and View.
    key(&mut harness, Key::ArrowDown);
    key(&mut harness, Key::ArrowDown);
    assert_eq!(focused(&harness), "View");
    key(&mut harness, Key::ArrowUp);
    key(&mut harness, Key::ArrowUp);
    // → opens File at its first line, and → again its submenu.
    key(&mut harness, Key::ArrowRight);
    assert!(menu_open(&harness, "File"));
    assert_eq!(focused(&harness), "Open…");
    key(&mut harness, Key::ArrowDown);
    key(&mut harness, Key::ArrowRight);
    assert!(menu_open(&harness, "Open Recent"));
    assert_eq!(focused(&harness), "Holidays");
    // ← closes it, back on its line.
    key(&mut harness, Key::ArrowLeft);
    assert!(!menu_open(&harness, "Open Recent"));
    assert_eq!(focused(&harness), "Open Recent");
    // A disabled line is reached, and read so, but Enter does nothing on it.
    key(&mut harness, Key::ArrowDown);
    assert_eq!(focused(&harness), "Export…");
    assert!(
        harness
            .get_by_role_and_label(Role::MenuItem, "Export…")
            .accesskit_node()
            .is_disabled()
    );
    key(&mut harness, Key::Enter);
    assert!(menu_open(&harness, "File"));
    assert!(harness.state().answers.is_empty());
    // Enter chooses Quit; the menus close and the focus goes back to the ☰.
    key(&mut harness, Key::ArrowDown);
    key(&mut harness, Key::Enter);
    assert_eq!(
        harness.state().answers,
        vec![Answer::Chosen("quit".to_owned())]
    );
    assert!(!menu_open(&harness, "Menu"));
    assert!(!menu_open(&harness, "File"));
    assert!(
        harness
            .get_by_role_and_label(Role::Button, "Menu")
            .is_focused()
    );
}

#[test]
fn escape_closes_a_level_and_the_last_gives_the_focus_back() {
    let mut harness = app(true);
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Menu").focus();
    harness.run();
    open(&mut harness, Some("file"));
    key(&mut harness, Key::Escape);
    assert!(!menu_open(&harness, "File"));
    assert!(menu_open(&harness, "Menu"));
    assert_eq!(focused(&harness), "File");
    key(&mut harness, Key::Escape);
    assert!(!menu_open(&harness, "Menu"));
    assert_eq!(harness.state().answers, vec![Answer::Dismissed]);
    assert!(
        harness
            .get_by_role_and_label(Role::Button, "Menu")
            .is_focused()
    );
}

#[test]
fn tab_closes_every_level() {
    let mut harness = app(true);
    harness.run();
    open(&mut harness, Some("file"));
    key(&mut harness, Key::Tab);
    assert!(!menu_open(&harness, "Menu"));
    assert!(!menu_open(&harness, "File"));
    assert_eq!(harness.state().answers, vec![Answer::Dismissed]);
}

#[test]
fn a_letter_moves_to_the_next_line_starting_with_it() {
    let mut harness = app(true);
    harness.run();
    open(&mut harness, None);
    let typed = |harness: &mut Harness<'_, App>, text: &str| {
        harness.event(Event::Text(text.to_owned()));
        harness.run();
    };
    typed(&mut harness, "v");
    assert_eq!(focused(&harness), "View");
    typed(&mut harness, "H");
    assert_eq!(focused(&harness), "Help");
    // A letter no line starts with leaves the choice.
    typed(&mut harness, "z");
    assert_eq!(focused(&harness), "Help");
    // In a submenu, from the line chosen on, round to the first.
    open(&mut harness, Some("file"));
    typed(&mut harness, "o");
    assert_eq!(focused(&harness), "Open Recent");
    typed(&mut harness, "o");
    assert_eq!(focused(&harness), "Open…");
    typed(&mut harness, "q");
    assert_eq!(focused(&harness), "Quit");
    // Nothing is chosen by a letter.
    assert!(harness.state().answers.is_empty());
}

#[test]
fn a_click_chooses_an_item_but_never_a_disabled_one() {
    let mut harness = app(true);
    harness.run();
    open(&mut harness, Some("file"));
    harness
        .get_by_role_and_label(Role::MenuItem, "Export…")
        .click();
    harness.run();
    assert!(harness.state().answers.is_empty());
    assert!(menu_open(&harness, "File"));
    harness
        .get_by_role_and_label(Role::MenuItem, "Open…")
        .click();
    harness.run();
    assert_eq!(
        harness.state().answers,
        vec![Answer::Chosen("open".to_owned())]
    );
    assert!(!menu_open(&harness, "Menu"));
}

/// Holds Option (Alt), or lets it go, from the next frame on.
fn hold_alt(harness: &mut Harness<'_, App>, held: bool) {
    let modifiers = if held {
        egui::Modifiers::ALT
    } else {
        egui::Modifiers::NONE
    };
    harness.event(Event::ModifiersChanged(modifiers));
    harness.run();
}

#[test]
fn option_shows_an_item_s_alternate_in_its_place_and_chooses_it() {
    let mut harness = app(true);
    harness.run();
    open(&mut harness, Some("help"));
    assert_eq!(focused(&harness), "About");
    key(&mut harness, Key::ArrowDown);
    assert_eq!(focused(&harness), "Compress");
    // The level's width, as its first line's.
    let help = |harness: &Harness<'_, App>| {
        let about = harness.get_by_role_and_label(Role::MenuItem, "About");
        about.rect().width()
    };
    let width = help(&harness);
    // Held, the alternate is the line, the focus staying on it; the level is as wide as
    // either, so nothing moves.
    hold_alt(&mut harness, true);
    assert!(harness.query_by_label("Compress").is_none());
    assert_eq!(focused(&harness), "Truncate the Whole Log");
    assert_eq!(help(&harness), width);
    // Let go, the item is back.
    hold_alt(&mut harness, false);
    assert_eq!(focused(&harness), "Compress");
    // Enter with it held chooses the alternate.
    hold_alt(&mut harness, true);
    harness.key_press_modifiers(egui::Modifiers::ALT, Key::Enter);
    harness.run();
    assert_eq!(
        harness.state().answers,
        vec![Answer::Chosen("truncate".to_owned())]
    );
    assert!(!menu_open(&harness, "Menu"));
    // A click with it held does too; without, the item.
    hold_alt(&mut harness, false);
    for (alt, chosen) in [(true, "truncate"), (false, "compress")] {
        harness.state_mut().answers.clear();
        open(&mut harness, Some("help"));
        hold_alt(&mut harness, alt);
        let name = if alt {
            "Truncate the Whole Log"
        } else {
            "Compress"
        };
        // A click's release carries the keys held, as the window system's does.
        let held = if alt {
            egui::Modifiers::ALT
        } else {
            egui::Modifiers::NONE
        };
        let item = harness.get_by_role_and_label(Role::MenuItem, name);
        item.click_modifiers(held);
        harness.run();
        assert_eq!(
            harness.state().answers,
            vec![Answer::Chosen(chosen.to_owned())]
        );
    }
}

/// A click chooses by Alt as the button was let go, not as Alt is by the frame's end: a
/// release without it then Alt pressed in the same frame chooses the item, and the
/// reverse its alternate.
#[test]
fn a_click_chooses_by_alt_as_the_button_was_let_go() {
    for (at_release, after, chosen) in [
        (egui::Modifiers::NONE, egui::Modifiers::ALT, "compress"),
        (egui::Modifiers::ALT, egui::Modifiers::NONE, "truncate"),
    ] {
        let mut harness = app(true);
        harness.run();
        open(&mut harness, Some("help"));
        hold_alt(&mut harness, at_release.alt);
        let name = if at_release.alt {
            "Truncate the Whole Log"
        } else {
            "Compress"
        };
        let at = harness
            .get_by_role_and_label(Role::MenuItem, name)
            .rect()
            .center();
        let button = |pressed, modifiers| Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers,
        };
        harness.event(Event::PointerMoved(at));
        harness.event(button(true, at_release));
        harness.run();
        let events = &mut harness.input_mut().events;
        events.push(button(false, at_release));
        events.push(Event::ModifiersChanged(after));
        harness.run();
        assert_eq!(
            harness.state().answers,
            vec![Answer::Chosen(chosen.to_owned())],
            "{at_release:?}"
        );
    }
}

#[test]
fn a_submenu_opens_once_the_pointer_rests_on_its_line() {
    let mut harness = app(true);
    harness.run();
    open(&mut harness, None);
    harness
        .get_by_role_and_label(Role::MenuItem, "Edit")
        .hover();
    harness.run_steps(2);
    // Chosen at once, opened only after a moment.
    assert_eq!(focused(&harness), "Edit");
    assert!(!menu_open(&harness, "Edit"));
    harness.run_steps(20);
    harness.run();
    assert!(menu_open(&harness, "Edit"));
    // A click on another line opens its submenu at once.
    harness
        .get_by_role_and_label(Role::MenuItem, "View")
        .click();
    harness.run();
    assert!(menu_open(&harness, "View"));
    assert!(!menu_open(&harness, "Edit"));
}

#[test]
fn a_press_outside_closes_the_menus_and_one_on_the_button_too() {
    let mut harness = app(true);
    harness.run();
    open(&mut harness, Some("view"));
    harness.get_by_role(Role::Toolbar);
    let outside = egui::pos2(900.0, 300.0);
    harness.hover_at(outside);
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: outside,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    harness.run();
    assert!(!menu_open(&harness, "Menu"));
    assert_eq!(harness.state().answers, vec![Answer::Dismissed]);

    // The ☰ opens it and, pressed again, closes it, never opening it twice.
    harness.get_by_role_and_label(Role::Button, "Menu").click();
    harness.run();
    assert!(menu_open(&harness, "Menu"));
    harness.get_by_role_and_label(Role::Button, "Menu").click();
    harness.run();
    assert!(!menu_open(&harness, "Menu"));
    assert_eq!(harness.state().actions, vec![TopPanelAction::Menu]);
    assert_eq!(
        harness.state().answers,
        vec![Answer::Dismissed, Answer::Dismissed]
    );
}

#[test]
fn screen_readers_read_it_as_a_menu() {
    let mut harness = app(true);
    harness.run();
    open(&mut harness, Some("view"));
    // The top level is a menu named as the request names it, its submenus' lines menu items
    // that open menus, with their access keys.
    let file = harness.get_by_role_and_label(Role::MenuItem, "File");
    let data = file.accesskit_node();
    assert_eq!(data.data().has_popup(), Some(HasPopup::Menu));
    assert_eq!(data.data().is_expanded(), Some(false));
    assert_eq!(data.data().access_key(), Some("F"));
    assert!(harness.query_by_label("&File").is_none());
    // Items that can be checked are check boxes, checked or not.
    let guides = harness.get_by_role_and_label(Role::MenuItemCheckBox, "Guides");
    assert_eq!(guides.accesskit_node().toggled(), Some(Toggled::True));
    let proof = harness.get_by_role_and_label(Role::MenuItemCheckBox, "Proof Margin");
    assert_eq!(proof.accesskit_node().toggled(), Some(Toggled::False));
    // An item's keys are its shortcut; a disabled one is read so.
    open(&mut harness, Some("file"));
    let item = harness.get_by_role_and_label(Role::MenuItem, "Open…");
    assert_eq!(
        item.accesskit_node().data().keyboard_shortcut(),
        Some("Ctrl+O")
    );
    assert!(!item.accesskit_node().is_disabled());
    assert!(
        harness
            .get_by_role_and_label(Role::MenuItem, "Export…")
            .accesskit_node()
            .is_disabled()
    );
    // The submenu is a menu of its own, under its line.
    let menu = harness.get_by_role_and_label(Role::Menu, "File");
    assert!(
        menu.accesskit_node()
            .parent()
            .is_some_and(|parent| parent.label().as_deref() == Some("File"))
    );
    // Separators are read as separators, each in its menu, and never take the focus.
    let mut menus = Vec::new();
    for separator in harness.get_all_by_role(Role::Splitter) {
        let node = separator.accesskit_node();
        assert!(!node.data().supports_action(egui::accesskit::Action::Focus));
        let mut menu = node.parent();
        while let Some(above) = menu.as_ref().filter(|above| above.role() != Role::Menu) {
            menu = above.parent();
        }
        menus.push(menu.and_then(|menu| menu.label()).unwrap_or_default());
    }
    menus.sort();
    assert_eq!(menus, ["File", "File", "Menu"]);
}

/// Presses `key`, as a keyboard sends it with the text it types, if any.
fn typed_key(harness: &mut Harness<'_, App>, key: Key, text: Option<&str>) {
    harness.key_press(key);
    if let Some(text) = text {
        harness.event(Event::Text(text.to_owned()));
    }
    harness.run();
}

fn pressed(seen: &[Event], wanted: Key) -> bool {
    seen.iter()
        .any(|e| matches!(e, Event::Key { key, .. } if *key == wanted))
}

#[test]
fn a_key_typed_into_the_open_menu_reaches_nothing_else() {
    let mut harness = app(true);
    harness.run();
    open(&mut harness, None);
    harness.state_mut().seen.clear();
    // A letter for type-ahead, which would also be G's shortcut, and the other plain keys.
    typed_key(&mut harness, Key::G, Some("g"));
    typed_key(&mut harness, Key::Num1, Some("1"));
    for key in [
        Key::Home,
        Key::End,
        Key::PageUp,
        Key::PageDown,
        Key::Backspace,
        Key::Delete,
        Key::Insert,
        Key::Minus,
    ] {
        typed_key(&mut harness, key, None);
    }
    assert!(menu_open(&harness, "Menu"));
    assert_eq!(harness.state().seen, vec![]);
    // The F-keys and keys with a modifier stay the app's.
    typed_key(&mut harness, Key::F10, None);
    harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::G);
    harness.run();
    let seen = std::mem::take(&mut harness.state_mut().seen);
    assert!(pressed(&seen, Key::F10), "{seen:?}");
    assert!(pressed(&seen, Key::G), "{seen:?}");
    // Alt with a letter, and the letter it typed, as ф on a Ukrainian keyboard's A, so the
    // app can open another menu by its access key.
    let alt = |pressed| Event::Key {
        key: Key::A,
        physical_key: Some(Key::A),
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::ALT,
    };
    let events = &mut harness.input_mut().events;
    events.extend([alt(true), Event::Text("ф".to_owned()), alt(false)]);
    harness.run();
    let seen = std::mem::take(&mut harness.state_mut().seen);
    assert!(pressed(&seen, Key::A), "{seen:?}");
    assert!(seen.contains(&Event::Text("ф".to_owned())), "{seen:?}");
    // Text that follows no Alt is still the menu's.
    typed_key(&mut harness, Key::G, Some("g"));
    assert_eq!(harness.state().seen, vec![]);

    // With no popup open, nothing is taken.
    key(&mut harness, Key::Escape);
    harness.state_mut().seen.clear();
    typed_key(&mut harness, Key::G, Some("g"));
    typed_key(&mut harness, Key::ArrowDown, None);
    let seen = &harness.state().seen;
    assert!(pressed(seen, Key::G), "{seen:?}");
    assert!(pressed(seen, Key::ArrowDown), "{seen:?}");
    assert!(seen.contains(&Event::Text("g".to_owned())), "{seen:?}");
}

#[test]
fn an_ampersand_below_the_top_is_shown_as_given() {
    let mut harness = app(true);
    harness.run();
    open(&mut harness, Some("edit"));
    for name in ["Rock & Roll", "Tips & Tricks"] {
        let line = harness.get_by_role_and_label(Role::MenuItem, name);
        assert_eq!(line.accesskit_node().data().access_key(), None, "{name}");
    }
    assert!(harness.query_by_label("Rock  Roll").is_none());
    // Type-ahead reads the words as given; the submenu is named by them.
    harness.event(Event::Text("t".to_owned()));
    harness.run();
    assert_eq!(focused(&harness), "Tips & Tricks");
    key(&mut harness, Key::ArrowRight);
    assert!(menu_open(&harness, "Tips & Tricks"));
    // At the top, the & still marks the access key and is not shown.
    let edit = harness.get_by_role_and_label(Role::MenuItem, "Edit");
    assert_eq!(edit.accesskit_node().data().access_key(), Some("E"));
}

#[test]
fn opened_again_it_gives_the_focus_back_to_what_had_it_first() {
    let mut harness = app(true);
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Menu").focus();
    harness.run();
    open(&mut harness, None);
    assert_eq!(focused(&harness), "File");
    // Opened again at View while its own line has the focus.
    open(&mut harness, Some("view"));
    assert_eq!(focused(&harness), "Guides");
    key(&mut harness, Key::Escape);
    key(&mut harness, Key::Escape);
    assert!(!menu_open(&harness, "Menu"));
    assert!(
        harness
            .get_by_role_and_label(Role::Button, "Menu")
            .is_focused()
    );
}

#[test]
fn a_submenu_opens_while_the_pointer_moves_along_its_line() {
    let mut harness = app(true);
    harness.run();
    open(&mut harness, None);
    let line = harness.get_by_role_and_label(Role::MenuItem, "Edit").rect();
    // A point further along the line every frame, for half a second.
    for step in 0..30 {
        harness.hover_at(egui::pos2(line.left() + 4.0 + step as f32, line.center().y));
        harness.run_steps(1);
        if step == 5 {
            assert!(!menu_open(&harness, "Edit"), "open before its pause");
        }
    }
    // No `run()` here: it would let the pointer rest until the pause ran out.
    assert!(menu_open(&harness, "Edit"));
}

#[test]
fn a_popup_no_frame_draws_takes_no_keys_and_closes() {
    let mut harness = app(true);
    harness.run();
    open(&mut harness, None);
    harness.state_mut().show = false;
    harness.run();
    harness.state_mut().seen.clear();
    key(&mut harness, Key::ArrowDown);
    assert!(pressed(&harness.state().seen, Key::ArrowDown));
    assert_eq!(harness.state().answers, vec![Answer::Dismissed]);
    assert!(!popup::is_open(&harness.ctx, TopPanel::menu_popup()));
}

#[test]
fn an_answer_is_read_once() {
    let mut harness = app(true);
    harness.run();
    // Chosen by a click, by the keys, and dismissed.
    open(&mut harness, Some("file"));
    harness
        .get_by_role_and_label(Role::MenuItem, "Open…")
        .click();
    harness.run();
    open(&mut harness, Some("file"));
    key(&mut harness, Key::Enter);
    open(&mut harness, None);
    key(&mut harness, Key::Escape);
    let open_image = Answer::Chosen("open".to_owned());
    assert_eq!(
        harness.state().answers,
        vec![open_image.clone(), open_image, Answer::Dismissed]
    );
    assert_eq!(harness.state().twice, vec![]);
}
