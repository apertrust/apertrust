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

use super::*;
use crate::library::Action;
use apertrust_ui::{Builtin, Locale, Theme};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

/// Why a recent of the test's cannot be opened.
#[derive(Clone, Copy)]
enum Mark {
    Gone,
    Failed(&'static str),
}

/// The start window as a test holds it: the recents, as paths with their marks, the list,
/// whether the close button is drawn, and what was asked.
struct Shown {
    recents: Vec<(PathBuf, Option<Mark>)>,
    list: List,
    close: bool,
    asked: Vec<Asked>,
}

const HOME: &str = "/Users/me";

/// Five recents, newest first, the fourth gone.
fn sample() -> Vec<(PathBuf, Option<Mark>)> {
    [
        ("/Users/me/Pictures/2026-10 Lisbon", None),
        ("/Volumes/Main-Storage/Photos/Family", None),
        ("/Users/me/Pictures/2026-09 Mountains", None),
        ("/Volumes/SD-128/DCIM", Some(Mark::Gone)),
        ("/Users/me/Pictures/2026-08 Wedding, Kraków", None),
    ]
    .into_iter()
    .map(|(path, mark)| (PathBuf::from(path), mark))
    .collect()
}

fn window(recents: Vec<(PathBuf, Option<Mark>)>) -> Harness<'static, Shown> {
    themed(Theme::default(), recents, false)
}

fn themed(
    theme: Theme,
    recents: Vec<(PathBuf, Option<Mark>)>,
    close: bool,
) -> Harness<'static, Shown> {
    // A frame's time as a display's, so two clicks a frame apart are a double-click.
    let mut harness = Harness::builder()
        .with_size(SIZE)
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, theme.clone());
            apertrust_ui::set_locale(ctx, Locale::EnGb);
            ctx.global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
        })
        .build_ui_state(
            |ui, shown: &mut Shown| {
                let recents: Vec<Recent<'_>> = shown
                    .recents
                    .iter()
                    .map(|(path, mark)| Recent {
                        kind: crate::recents::OpenKind::Folder,
                        path,
                        missing: mark.map(|mark| match mark {
                            Mark::Gone => Missing::Gone,
                            Mark::Failed(why) => Missing::Failed(why),
                        }),
                    })
                    .collect();
                let window = super::Shown {
                    home: Some(Path::new(HOME)),
                    close: shown.close,
                    keys: true,
                };
                let asked = show(ui, &recents, &mut shown.list, window);
                shown.asked.extend(asked);
            },
            Shown {
                recents,
                list: List::shown(),
                close,
                asked: Vec::new(),
            },
        );
    harness.run();
    harness
}

/// The recent whose row is chosen, by its name, as screen readers hear it.
fn chosen(harness: &Harness<'_, Shown>) -> String {
    let options = harness.get_all_by_role(Role::ListBoxOption);
    let chosen: Vec<String> = options
        .filter(|row| row.accesskit_node().data().is_selected() == Some(true))
        .filter_map(|row| row.accesskit_node().label())
        .collect();
    assert_eq!(chosen.len(), 1, "{chosen:?}");
    chosen[0].clone()
}

fn press(harness: &mut Harness<'_, Shown>, key: egui::Key) {
    harness.key_press(key);
    harness.run();
}

#[test]
fn the_start_window_in_the_dark_theme() {
    let mut harness = window(sample());
    harness.event(egui::Event::PointerGone);
    harness.run();
    harness.snapshot("start-window-dark");
}

#[test]
fn the_start_window_in_the_light_theme() {
    let mut harness = themed(Theme::builtin(Builtin::Light), sample(), false);
    harness.event(egui::Event::PointerGone);
    harness.run();
    harness.snapshot("start-window-light");
}

/// To screen readers a list box named Recents, of options each named by its name and
/// described by its kind and its path, written from `~`, and why it cannot be opened where
/// it cannot; the first chosen as the window shows.
#[test]
fn each_recent_is_its_kind_name_and_path_and_missing_where_it_is() {
    let mut recents = sample();
    recents[2].1 = Some(Mark::Failed("Permission denied"));
    let harness = window(recents);
    harness.get_by_role_and_label(Role::ListBox, "Recents");
    let about = |name: &str| {
        let row = harness.get_by_role_and_label(Role::ListBoxOption, name);
        let data = row.accesskit_node().data().clone();
        (data.is_selected(), data.description().map(str::to_owned))
    };
    let lisbon = (
        Some(true),
        Some("Folder, ~/Pictures/2026-10 Lisbon".to_owned()),
    );
    assert_eq!(about("2026-10 Lisbon"), lisbon);
    let family = (
        Some(false),
        Some("Folder, /Volumes/Main-Storage/Photos/Family".to_owned()),
    );
    assert_eq!(about("Family"), family);
    let gone = "Folder, /Volumes/SD-128/DCIM. This folder is no longer there.";
    assert_eq!(about("DCIM"), (Some(false), Some(gone.to_owned())));
    let failed = "Folder, ~/Pictures/2026-09 Mountains. This folder could not be opened. \
                  Permission denied";
    assert_eq!(
        about("2026-09 Mountains"),
        (Some(false), Some(failed.to_owned()))
    );
}

/// ↑, ↓, Home and End choose in the list, the focus staying on the chosen row; Enter opens
/// the recent chosen; Escape closes the window.
#[test]
fn the_list_s_keys_choose_and_open_and_escape_closes() {
    let mut harness = window(sample());
    assert_eq!(chosen(&harness), "2026-10 Lisbon");
    assert!(
        harness
            .get_by_role_and_label(Role::ListBoxOption, "2026-10 Lisbon")
            .is_focused()
    );
    press(&mut harness, egui::Key::ArrowDown);
    press(&mut harness, egui::Key::ArrowDown);
    assert_eq!(chosen(&harness), "2026-09 Mountains");
    assert!(
        harness
            .get_by_role_and_label(Role::ListBoxOption, "2026-09 Mountains")
            .is_focused()
    );
    press(&mut harness, egui::Key::End);
    assert_eq!(chosen(&harness), "2026-08 Wedding, Kraków");
    press(&mut harness, egui::Key::ArrowDown);
    assert_eq!(chosen(&harness), "2026-08 Wedding, Kraków");
    press(&mut harness, egui::Key::Home);
    press(&mut harness, egui::Key::ArrowUp);
    assert_eq!(chosen(&harness), "2026-10 Lisbon");
    press(&mut harness, egui::Key::ArrowDown);
    press(&mut harness, egui::Key::Enter);
    let family = PathBuf::from("/Volumes/Main-Storage/Photos/Family");
    assert_eq!(harness.state().asked, [Asked::Open(family)]);
    press(&mut harness, egui::Key::Escape);
    assert_eq!(harness.state().asked.last(), Some(&Asked::Close));
}

/// Tab goes from the list to the buttons, the first that can be pressed.
#[test]
fn tab_goes_from_the_list_to_the_buttons() {
    let mut recents = sample();
    recents.retain(|(_, mark)| mark.is_none());
    let mut harness = window(recents);
    press(&mut harness, egui::Key::Tab);
    assert!(
        harness
            .get_by_role_and_label(Role::Button, "Open Folder…")
            .is_focused()
    );
    // The list's keys are the list's alone: Enter presses the button with the focus.
    press(&mut harness, egui::Key::Enter);
    assert_eq!(harness.state().asked, [Asked::OpenFolder]);
}

/// A click chooses a recent, Open opens the one chosen, and a double-click opens one.
#[test]
fn open_opens_the_recent_chosen_and_a_double_click_opens_one() {
    let mut harness = window(sample());
    harness
        .get_by_role_and_label(Role::ListBoxOption, "2026-09 Mountains")
        .click();
    harness.run();
    assert_eq!(chosen(&harness), "2026-09 Mountains");
    harness.get_by_role_and_label(Role::Button, "Open").click();
    harness.run();
    let mountains = PathBuf::from("/Users/me/Pictures/2026-09 Mountains");
    assert_eq!(harness.state().asked, [Asked::Open(mountains)]);
    // Two clicks a frame apart, in a window of their own, so no click before them makes
    // them the end of a triple-click.
    let mut harness = window(sample());
    for _ in 0..2 {
        harness
            .get_by_role_and_label(Role::ListBoxOption, "Family")
            .click();
        harness.step();
    }
    harness.run();
    let family = PathBuf::from("/Volumes/Main-Storage/Photos/Family");
    assert_eq!(harness.state().asked, [Asked::Open(family)]);
}

/// A recent whose opening failed this run is marked so, with Locate… and Remove from the
/// List, but not greyed, as Open Recent's: Open stays enabled while it is chosen, and Enter
/// tries it again.
#[test]
fn a_recent_that_failed_to_open_tries_again() {
    let mut recents = sample();
    recents[2].1 = Some(Mark::Failed("Permission denied"));
    recents[3].1 = None;
    let mut harness = window(recents);
    press(&mut harness, egui::Key::ArrowDown);
    press(&mut harness, egui::Key::ArrowDown);
    assert_eq!(chosen(&harness), "2026-09 Mountains");
    let open = harness.get_by_role_and_label(Role::Button, "Open");
    assert!(!open.accesskit_node().is_disabled());
    press(&mut harness, egui::Key::Enter);
    let mountains = PathBuf::from("/Users/me/Pictures/2026-09 Mountains");
    assert_eq!(harness.state().asked, [Asked::Open(mountains)]);
    harness.get_by_role_and_label(Role::Button, "Locate…");
}

/// A recent found gone offers Locate… and Remove from the List, and cannot be opened: Open
/// is disabled while it is chosen, and Enter does nothing.
#[test]
fn a_missing_recent_offers_locate_and_remove_and_never_opens() {
    let mut harness = window(sample());
    let card = PathBuf::from("/Volumes/SD-128/DCIM");
    harness
        .get_by_role_and_label(Role::Button, "Locate…")
        .click();
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Remove from the List")
        .click();
    harness.run();
    assert_eq!(
        harness.state().asked,
        [Asked::Locate(card.clone()), Asked::Remove(card)]
    );
    harness.state_mut().asked.clear();
    harness
        .get_by_role_and_label(Role::ListBoxOption, "DCIM")
        .click();
    harness.run();
    assert_eq!(chosen(&harness), "DCIM");
    let open = harness.get_by_role_and_label(Role::Button, "Open");
    assert!(open.accesskit_node().is_disabled());
    press(&mut harness, egui::Key::Enter);
    assert!(harness.state().asked.is_empty());
}

/// Libraries, projects and Browse… come later: their buttons are there, disabled, saying
/// so; with nothing opened yet, the list says so and Open is disabled.
#[test]
fn what_comes_later_is_disabled_and_an_empty_list_says_nothing_opened_yet() {
    let harness = window(Vec::new());
    for later in ["New Library…", "New Project…", "Browse…"] {
        let button = harness.get_by_role_and_label(Role::Button, later);
        assert!(button.accesskit_node().is_disabled(), "{later}");
    }
    harness.get_by_label("Nothing opened yet");
    let open = harness.get_by_role_and_label(Role::Button, "Open");
    assert!(open.accesskit_node().is_disabled());
    let open_folder = harness.get_by_role_and_label(Role::Button, "Open Folder…");
    assert!(!open_folder.accesskit_node().is_disabled());
}

/// Where the app draws the window's frame, Windows' and Linux's, the strip ends in a close
/// button, which closes the window; a drag on the strip moves it.
#[test]
fn the_strip_s_close_button_closes_and_a_drag_moves_the_window() {
    let mut harness = themed(Theme::default(), sample(), true);
    harness.get_by_role_and_label(Role::Button, "Close").click();
    harness.run();
    assert_eq!(harness.state().asked, [Asked::Close]);
    harness.state_mut().asked.clear();
    let at = egui::pos2(SIZE.x / 2.0, STRIP / 2.0);
    harness.event(egui::Event::PointerMoved(at));
    harness.event(egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run();
    harness.event(egui::Event::PointerMoved(at + egui::vec2(40.0, 10.0)));
    harness.run();
    assert_eq!(harness.state().asked, [Asked::Move]);
    // The Mac's window buttons are AppKit's: the app draws no close of its own there.
    let mac = window(sample());
    assert!(mac.query_by_role_and_label(Role::Button, "Close").is_none());
}

/// What the window asks of the folders is Browse's to carry out; its own is not.
#[test]
fn what_it_asks_of_the_folders_is_browse_s() {
    let path = PathBuf::from("/photos/Wedding");
    assert_eq!(
        Asked::Open(path.clone()).library(),
        Some(Action::Open(path.clone()))
    );
    assert_eq!(Asked::OpenFolder.library(), Some(Action::OpenFolder));
    assert_eq!(
        Asked::Locate(path.clone()).library(),
        Some(Action::Locate(path.clone()))
    );
    assert_eq!(
        Asked::Remove(path.clone()).library(),
        Some(Action::Remove(path))
    );
    for own in [Asked::Close, Asked::Move, Asked::Menu] {
        assert_eq!(own.library(), None);
    }
}

/// The main window keeps its own size and place, never the start window's, nor the start
/// window's size it has for a moment after it.
#[test]
fn the_main_window_keeps_its_own_placement_never_the_start_window_s() {
    let mut window = Window::new(true, 1.0);
    assert!(!window.keeps_placement(SIZE));
    window.shown = false;
    window.settling = Some(SIZE * 1.5);
    assert!(!window.keeps_placement(SIZE * 1.5));
    assert!(window.settling.is_some());
    assert!(window.keeps_placement(egui::vec2(1400.0, 900.0)));
    assert_eq!(window.settling, None);
    assert!(window.keeps_placement(SIZE * 1.5));
}
