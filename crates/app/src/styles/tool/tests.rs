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

use apertrust_library::sidecar::Entry;
use apertrust_tools::TOOLS;
use apertrust_ui::{Builtin, Locale, Theme};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::*;
use crate::settings::Settings;
use crate::tools::Head;
use apertrust_engine::sidecar::Parked;
use apertrust_library::presets::{self as files, Kind};
use apertrust_ui::translator;

/// A style named `name` setting Grain, its file named after it.
fn style(name: &str) -> Listed {
    let mut values = ToolValues::new();
    values.set("strength", apertrust_tools::ControlValue::Number(40.0));
    values.set_enabled(true);
    Listed {
        name: name.to_owned(),
        path: PathBuf::from(format!("/styles/{name}.apstyle")),
        tools: Some(vec![("grain".to_owned(), values)]),
        left_out: Vec::new(),
        newer: false,
        builtin: None,
    }
}

/// The styles built in, in English, as the tree reads them.
fn builtins() -> Vec<Listed> {
    let words = Translator::new(Locale::EnGb);
    read(Path::new("/no styles here"), TOOLS, &words)
}

/// The styles of the tests: two in Film, one alone, and one a newer Apertrust made.
fn listed() -> Vec<Listed> {
    let mut newer = style("Future");
    newer.tools = None;
    newer.newer = true;
    vec![
        style("Film|Warm film"),
        style("Film|Cool film"),
        style("Bright"),
        newer,
    ]
}

fn applied(names: &[&str]) -> Vec<StyleState> {
    (names.iter())
        .map(|name| StyleState {
            name: (*name).to_owned(),
            file: format!("{name}.apstyle"),
            tools: vec!["grain".to_owned()],
        })
        .collect()
}

#[test]
fn the_tree_makes_folders_of_the_bar_in_names_each_level_in_order() {
    let listed = listed();
    let shown: Vec<&Listed> = listed.iter().collect();
    let names = |nodes: &[Node<'_>]| -> Vec<String> {
        nodes.iter().map(|node| node.name().to_owned()).collect()
    };
    let tree = tree_of(&shown, CUSTOM);
    assert_eq!(names(&tree), ["Film", "Bright", "Future"]);
    let Node::Folder { path, children, .. } = &tree[0] else {
        panic!("Film is a folder: {tree:?}");
    };
    assert_eq!(path, "custom|Film");
    assert_eq!(names(children), ["Cool film", "Warm film"]);
}

#[test]
fn find_style_keeps_the_styles_whose_names_hold_its_text() {
    let listed = listed();
    let names = |find: &str| -> Vec<String> {
        shown(&listed, find)
            .iter()
            .map(|style| style.name.clone())
            .collect()
    };
    assert_eq!(names("FILM"), ["Film|Warm film", "Film|Cool film"]);
    assert_eq!(names(" warm "), ["Film|Warm film"]);
    assert_eq!(names("").len(), 4);
}

#[test]
fn a_click_applies_the_style_as_the_mode_says_and_parked_entries_are_said() {
    let mut warm = style("Film|Warm film");
    let Some(Command::ApplyStyle {
        name,
        file,
        mode,
        tools,
    }) = apply(&warm, StyleMode::Replace, &[])
    else {
        panic!("a style applied");
    };
    assert_eq!(
        (name.as_str(), file.as_str(), mode),
        (
            "Film|Warm film",
            "Film|Warm film.apstyle",
            StyleMode::Replace
        )
    );
    assert_eq!(tools.len(), 1);
    let words = Translator::new(Locale::EnGb);
    assert_eq!(left_out(&warm, &words), None);
    warm.left_out = vec![Parked::NewerTool {
        version: "9".to_owned(),
    }];
    assert_eq!(
        left_out(&warm, &words).as_deref(),
        Some("1 tool of “Film|Warm film” was left out: made by a newer Apertrust")
    );
    let mut future = warm;
    future.tools = None;
    assert_eq!(apply(&future, StyleMode::Append, &[]), None);
    assert_eq!(
        clear("Warm", "Warm.apstyle"),
        Command::ClearStyle {
            name: "Warm".to_owned(),
            file: "Warm.apstyle".to_owned(),
        }
    );
}

#[test]
fn a_file_s_entries_are_read_as_tools_and_a_newer_one_left_out() {
    let folder = std::env::temp_dir().join(format!("apertrust-styles-tool-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    let entry = |tool: &str, version: &str| Entry::Tool {
        tool: tool.to_owned(),
        version: Some(version.to_owned()),
        kept: Vec::new(),
        auto: Vec::new(),
        enabled: Some("true".to_owned()),
        controls: vec![("strength".to_owned(), "40".to_owned())],
    };
    let mut stored = files::Stored::new(Kind::Style, "Warm film");
    stored.entries = vec![entry("grain", "2"), entry("grain-of-the-future", "1")];
    files::write(&folder, &stored, "0.1.0").unwrap();
    let words = Translator::new(Locale::EnGb);
    let read = read(&folder, TOOLS, &words);
    let own: Vec<&Listed> = read.iter().filter(|s| s.builtin.is_none()).collect();
    let [warm] = own.as_slice() else {
        panic!("one style: {read:?}");
    };
    assert_eq!(warm.name, "Warm film");
    assert_eq!(warm.file(), "Warm film.apstyle");
    let tools: Vec<&str> = (warm.tools.iter().flatten())
        .map(|(tool, _)| tool.as_str())
        .collect();
    assert_eq!((tools, warm.left_out.len()), (vec!["grain"], 1));
    // None of its entries applies: parked, saying why.
    let mut future = files::Stored::new(Kind::Style, "Future");
    future.entries = vec![entry("grain-of-the-future", "1")];
    files::write(&folder, &future, "0.1.0").unwrap();
    let read = super::read(&folder, TOOLS, &words);
    let future = read.iter().find(|style| style.name == "Future").unwrap();
    assert_eq!((future.tools.as_ref(), future.left_out.len()), (None, 1));
    assert_eq!(parked(future, &words), "Made by a newer Apertrust");
    let retired = Listed {
        left_out: vec![Parked::Retired],
        ..future.clone()
    };
    assert_eq!(
        parked(&retired, &words),
        "Holds only tools Apertrust has retired"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// A photo still opening takes no edit yet: the tree's styles say so as with no photo.
#[test]
fn a_photo_still_opening_applies_no_style() {
    let words = Translator::new(Locale::EnGb);
    let no_photo = Some("Open a photo to apply a style");
    assert_eq!(unavailable(true, false, false, &words).as_deref(), no_photo);
    assert_eq!(
        unavailable(false, false, false, &words).as_deref(),
        no_photo
    );
    assert_eq!(unavailable(true, false, true, &words), None);
}

#[test]
fn the_mode_is_chosen_from_options_and_import_and_export_wait() {
    let words = Translator::new(Locale::EnGb);
    let entries = options(&words, StyleMode::Replace);
    let Entry_::Submenu(mode) = &entries[0] else {
        panic!("Mode ▸ first: {entries:?}");
    };
    assert_eq!(mode.text, "Mode");
    let checked: Vec<(String, bool)> = (mode.entries.iter())
        .filter_map(|entry| match entry {
            Entry_::Item(item) => Some((item.text.clone(), item.checked == Some(true))),
            _ => None,
        })
        .collect();
    assert_eq!(
        checked,
        [("Append".to_owned(), false), ("Replace".to_owned(), true)]
    );
    for (at, words) in [(1, "Import Style…"), (2, "Export Style…")] {
        let Entry_::Item(item) = &entries[at] else {
            panic!("{entries:?}");
        };
        assert_eq!((item.text.as_str(), item.enabled), (words, false));
    }
    assert_eq!(
        pressed(&Pressed::Own(MODE_APPEND.to_owned())),
        Some(Action::Mode(StyleMode::Append))
    );
    assert_eq!(pressed(&Pressed::Toggle), None);
}

use apertrust_ui::popup::Entry as Entry_;

/// What the harness holds: the styles listed, the styles applied (none with no photo),
/// what is kept, why styles cannot be applied, and what the tool asked.
struct Drawn {
    listed: Vec<Listed>,
    applied: Option<Vec<StyleState>>,
    kept: Kept,
    unavailable: Option<String>,
    actions: Vec<Action>,
}

/// The tool, open, in the theme `which`, 320 points wide, on a platform that asks first.
fn drawn(which: Builtin, state: Drawn) -> Harness<'static, Drawn> {
    drawn_with(which, state, true)
}

/// [`drawn`], `alerts` whether the platform shows the alert Delete Style… asks first with.
fn drawn_with(which: Builtin, state: Drawn, alerts: bool) -> Harness<'static, Drawn> {
    Harness::builder()
        .with_size(egui::vec2(320.0, 540.0))
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::builtin(which));
            apertrust_ui::set_locale(ctx, Locale::EnGb);
        })
        .build_ui_state(
            move |ui, state: &mut Drawn| {
                let (mirror, settings) = (crate::mirror::tests::mirror(), Settings::default());
                let (words, mut commands) = (translator(ui.ctx()), Vec::new());
                let mut cx = Context {
                    mirror: &mirror,
                    settings: &settings,
                    words: &words,
                    commands: &mut commands,
                };
                let titles = |id: &str| title(&words, id);
                let mut tool = StylesTool {
                    listed: &state.listed,
                    applied: state.applied.as_deref(),
                    kept: &state.kept,
                    unavailable: state.unavailable.clone(),
                    titles: &titles,
                    alerts,
                };
                let head = Head {
                    open: true,
                    options: options(&words, StyleMode::Append),
                    ..Head::default()
                };
                let mut actions = Vec::new();
                apertrust_ui::components::PanelBody::new().show(ui, |ui| {
                    let section = crate::utility::section(ui, &mut tool, &mut cx, head, false);
                    actions = section.actions;
                });
                state.actions.extend(actions);
            },
            state,
        )
}

fn with_photo() -> Drawn {
    Drawn {
        listed: listed(),
        applied: Some(applied(&["Film|Warm film", "Bright"])),
        kept: Kept::default(),
        unavailable: None,
        actions: Vec::new(),
    }
}

#[test]
fn the_styles_applied_are_listed_newest_first_and_the_tree_marks_them() {
    let mut harness = drawn(Builtin::Dark, with_photo());
    harness.run();
    let list = harness.get_by_role_and_label(Role::List, "Styles applied");
    let items: Vec<String> = (list.children())
        .filter(|node| node.accesskit_node().role() == Role::ListItem)
        .filter_map(|node| node.accesskit_node().label())
        .collect();
    assert_eq!(items, ["Warm film, applied", "Bright, applied"]);
    harness.get_by_role_and_label(Role::Tree, "Styles");
    let bright = harness.get_by_role_and_label(Role::TreeItem, "Bright");
    // Applied, in its description, never selected: nothing in the tool is chosen.
    let said = bright
        .accesskit_node()
        .data()
        .description()
        .map(str::to_owned);
    assert_eq!(said.as_deref(), Some("applied, Grain"));
    assert_ne!(bright.accesskit_node().data().is_selected(), Some(true));
    let future = harness.get_by_role_and_label(Role::TreeItem, "Future");
    assert!(future.accesskit_node().is_disabled());
    // Film is closed: its styles are not drawn.
    let film = harness.get_by_role_and_label(Role::TreeItem, "Film");
    assert_eq!(film.accesskit_node().data().is_expanded(), Some(false));
    assert!(harness.query_by_label("Warm film").is_none());
}

#[test]
fn a_click_applies_a_style_and_a_folder_opens() {
    let mut harness = drawn(Builtin::Dark, with_photo());
    harness.run();
    harness
        .get_by_role_and_label(Role::TreeItem, "Bright")
        .click();
    harness.run();
    harness
        .get_by_role_and_label(Role::TreeItem, "Film")
        .click();
    harness.run();
    assert_eq!(
        harness.state().actions,
        [
            Action::Apply("Bright.apstyle".to_owned()),
            Action::Toggle("custom|Film".to_owned())
        ]
    );
}

#[test]
fn with_no_photo_the_tree_is_disabled_and_nothing_is_listed_as_applied() {
    let words = Translator::new(Locale::EnGb);
    let state = Drawn {
        applied: None,
        unavailable: unavailable(false, false, false, &words),
        ..with_photo()
    };
    let mut harness = drawn(Builtin::Dark, state);
    harness.run();
    assert!(harness.query_by_label("Styles applied").is_none());
    let bright = harness.get_by_role_and_label(Role::TreeItem, "Bright");
    assert!(bright.accesskit_node().is_disabled());
    bright.click();
    harness.run();
    assert!(harness.state().actions.is_empty());
    assert_eq!(
        unavailable(true, true, false, &words).as_deref(),
        Some("The photo is open for viewing only")
    );
}

#[test]
fn find_style_opens_the_folders_of_the_styles_it_finds() {
    let state = Drawn {
        kept: Kept {
            find: "warm".to_owned(),
            ..Kept::default()
        },
        ..with_photo()
    };
    let mut harness = drawn(Builtin::Dark, state);
    harness.run();
    let warm = harness.get_by_role_and_label(Role::TreeItem, "Warm film");
    let said = warm
        .accesskit_node()
        .data()
        .description()
        .map(str::to_owned);
    assert_eq!(said.as_deref(), Some("applied, Grain"));
    assert!(
        harness.query_by_label("Bright").is_none() || {
            // Bright is still listed as applied, never in the tree.
            harness
                .query_all_by_role(Role::TreeItem)
                .all(|node| node.accesskit_node().label().as_deref() != Some("Bright"))
        }
    );
}

/// The two root folders are always drawn, open at first, Custom Styles first; an empty
/// Custom Styles says so.
#[test]
fn the_two_root_folders_are_always_drawn_and_an_empty_one_says_so() {
    let state = Drawn {
        listed: builtins(),
        ..with_photo()
    };
    let mut harness = drawn(Builtin::Dark, state);
    harness.run();
    let roots: Vec<String> = (harness.query_all_by_role(Role::TreeItem))
        .filter(|node| node.accesskit_node().data().level() == Some(0))
        .filter_map(|node| node.accesskit_node().label())
        .collect();
    assert_eq!(roots, ["Custom Styles", "Built-in Styles"]);
    for root in &roots {
        let node = harness.get_by_role_and_label(Role::TreeItem, root);
        assert_eq!(node.accesskit_node().data().is_expanded(), Some(true));
    }
    harness.get_by_label("No styles yet.");
    for name in [
        "Black and white",
        "Natural colour",
        "Vibrant colour",
        "Punchy",
        "Lomo",
    ] {
        harness.get_by_role_and_label(Role::TreeItem, name);
    }
    harness.get_by_role_and_label(Role::TreeItem, "Open shadows and highlights");
    // Closed, Custom Styles shows nothing of its own.
    harness
        .get_by_role_and_label(Role::TreeItem, "Custom Styles")
        .click();
    harness.run();
    assert_eq!(harness.state().actions, [Action::Toggle(CUSTOM.to_owned())]);
}

/// Find style searches both roots, hiding one with no style it finds.
#[test]
fn find_style_searches_both_roots_and_hides_one_with_no_match() {
    let mut styles = builtins();
    styles.extend(listed());
    let tool = |find: &str, listed: &[Listed]| -> Vec<String> {
        let kept = Kept {
            find: find.to_owned(),
            ..Kept::default()
        };
        let titles = |id: &str| id.to_owned();
        let tool = StylesTool {
            listed,
            applied: None,
            kept: &kept,
            unavailable: None,
            titles: &titles,
            alerts: true,
        };
        let words = Translator::new(Locale::EnGb);
        let (nodes, _) = tool.nodes(&words);
        nodes.iter().map(|node| node.name().to_owned()).collect()
    };
    assert_eq!(tool("", &styles), ["Custom Styles", "Built-in Styles"]);
    assert_eq!(tool("o", &styles), ["Custom Styles", "Built-in Styles"]);
    assert_eq!(tool("lomo", &styles), ["Built-in Styles"]);
    assert_eq!(tool("warm", &styles), ["Custom Styles"]);
    assert!(tool("nothing like it", &styles).is_empty());
}

fn photograph(which: Builtin, name: &str) {
    let mut kept = Kept::default();
    kept.open.insert("custom|Film".to_owned());
    let mut styles = builtins();
    styles.extend(listed());
    // Warm film and the built-in Lomo applied.
    let mut applied = applied(&["Film|Warm film", "Lomo"]);
    applied[1].file = "builtin:lomo".to_owned();
    let state = Drawn {
        listed: styles,
        applied: Some(applied),
        kept,
        ..with_photo()
    };
    let mut harness = drawn(which, state);
    harness.run();
    harness.snapshot(name);
}

#[test]
fn the_styles_tool_in_the_dark_theme() {
    photograph(Builtin::Dark, "styles-tool-dark");
}

#[test]
fn the_styles_tool_in_the_light_theme() {
    photograph(Builtin::Light, "styles-tool-light");
}

/// Return in Find style applies the first style the tree shows, a person's before a
/// built-in, a closed folder's hidden, a finding's folders open.
#[test]
fn return_applies_the_first_style_the_tree_shows() {
    let mut listed = builtins();
    listed.extend(self::listed());
    let first = |kept: &Kept| {
        let titles = |id: &str| id.to_owned();
        let tool = StylesTool {
            listed: &listed,
            applied: None,
            kept,
            unavailable: None,
            titles: &titles,
            alerts: true,
        };
        let words = Translator::new(Locale::EnGb);
        tool.first_shown(&words).map(|style| style.name.clone())
    };
    assert_eq!(first(&Kept::default()).as_deref(), Some("Bright"));
    let mut open = Kept::default();
    open.open.insert("custom|Film".to_owned());
    assert_eq!(first(&open).as_deref(), Some("Film|Cool film"));
    let finding = Kept {
        find: "warm".to_owned(),
        ..Kept::default()
    };
    assert_eq!(first(&finding).as_deref(), Some("Film|Warm film"));
}

/// Return in Find style applies the first style the tree shows, the field keeping the
/// focus; ↓ moves into the tree, to its first row, a folder where it comes first.
#[test]
fn return_applies_the_first_style_shown_and_down_moves_into_the_tree() {
    let state = Drawn {
        kept: Kept {
            find: "film".to_owned(),
            ..Kept::default()
        },
        ..with_photo()
    };
    let mut harness = drawn(Builtin::Dark, state);
    harness.run();
    harness
        .get_by_role_and_label(Role::TextInput, "Find style")
        .focus();
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(
        harness.state().actions,
        [Action::Apply("Film|Cool film.apstyle".to_owned())]
    );
    assert!(
        harness
            .get_by_role_and_label(Role::TextInput, "Find style")
            .is_focused()
    );
    // The first row the tree draws is its first root, Custom Styles.
    let mut harness = drawn(Builtin::Dark, with_photo());
    harness.run();
    harness
        .get_by_role_and_label(Role::TextInput, "Find style")
        .focus();
    harness.run();
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    assert!(
        harness
            .get_by_role_and_label(Role::TreeItem, "Custom Styles")
            .is_focused()
    );
}

/// F2 renames the focused style and Delete asks to delete it; Shift+F10 opens its menu:
/// Apply for one not applied, Clear for one that is, never both, then Rename Style… and
/// Delete Style….
#[test]
fn a_style_s_keys_and_menu() {
    let mut harness = drawn(Builtin::Dark, with_photo());
    harness.run();
    harness
        .get_by_role_and_label(Role::TreeItem, "Future")
        .focus();
    harness.run();
    harness.key_press(egui::Key::F2);
    harness.run();
    harness.key_press(egui::Key::Delete);
    harness.run();
    // A newer Apertrust's file is never rewritten: F2 renames nothing, Delete asks.
    assert_eq!(
        harness.state().actions,
        [Action::Delete("Future".to_owned())]
    );
    // Its menu: Apply and Rename Style… disabled, Delete Style… offered.
    harness.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F10);
    harness.run();
    let apply = harness.get_by_role_and_label(Role::MenuItem, "Apply");
    assert!(apply.accesskit_node().is_disabled());
    assert!(
        harness
            .query_by_role_and_label(Role::MenuItem, "Clear")
            .is_none()
    );
    let rename = harness.get_by_role_and_label(Role::MenuItem, "Rename Style…");
    assert!(rename.accesskit_node().is_disabled());
    let delete = harness.get_by_role_and_label(Role::MenuItem, "Delete Style…");
    assert!(!delete.accesskit_node().is_disabled());
    // A file whose tools are all retired can still be renamed: F2 renames it.
    let mut retired = style("Old look");
    retired.tools = None;
    retired.left_out = vec![Parked::Retired];
    let mut state = with_photo();
    state.listed.push(retired);
    let mut harness = drawn(Builtin::Dark, state);
    harness.run();
    harness
        .get_by_role_and_label(Role::TreeItem, "Old look")
        .focus();
    harness.run();
    harness.key_press(egui::Key::F2);
    harness.run();
    assert_eq!(
        harness.state().actions,
        [Action::Rename("Old look".to_owned())]
    );
    // An applied style's: Clear in Apply's place.
    let mut harness = drawn(Builtin::Dark, with_photo());
    harness.run();
    harness
        .get_by_role_and_label(Role::TreeItem, "Bright")
        .focus();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F10);
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(Role::MenuItem, "Apply")
            .is_none()
    );
    harness
        .get_by_role_and_label(Role::MenuItem, "Clear")
        .click();
    harness.run();
    assert_eq!(
        harness.state().actions,
        [Action::Clear {
            name: "Bright".to_owned(),
            file: "Bright.apstyle".to_owned(),
        }]
    );
}

/// A click on a style applied opens its one command, Clear.
#[test]
fn a_click_on_an_applied_style_offers_clear() {
    let mut harness = drawn(Builtin::Dark, with_photo());
    harness.run();
    harness
        .get_by_role_and_label(Role::ListItem, "Warm film, applied")
        .click();
    harness.run();
    harness
        .get_by_role_and_label(Role::MenuItem, "Clear")
        .click();
    harness.run();
    assert_eq!(
        harness.state().actions,
        [Action::Clear {
            name: "Film|Warm film".to_owned(),
            file: "Film|Warm film.apstyle".to_owned(),
        }]
    );
}

/// Where the platform cannot ask first, Delete Style… is disabled and Delete does nothing.
#[test]
fn delete_waits_for_an_alert_the_platform_shows() {
    let mut harness = drawn_with(Builtin::Dark, with_photo(), false);
    harness.run();
    harness
        .get_by_role_and_label(Role::TreeItem, "Bright")
        .focus();
    harness.run();
    harness.key_press(egui::Key::Delete);
    harness.run();
    assert!(harness.state().actions.is_empty());
    harness.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F10);
    harness.run();
    let delete = harness.get_by_role_and_label(Role::MenuItem, "Delete Style…");
    assert!(delete.accesskit_node().is_disabled());
}

/// Delete Style… removes the style's file, and Rename Style… names it anew.
#[test]
fn delete_removes_the_file_and_rename_names_it_anew() {
    let folder =
        std::env::temp_dir().join(format!("apertrust-styles-delete-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    for name in ["Warm film", "Bright"] {
        let stored =
            apertrust_library::presets::Stored::new(apertrust_library::presets::Kind::Style, name);
        apertrust_library::presets::write(&folder, &stored, "0.1.0").unwrap();
    }
    super::super::delete(&folder, "Warm film").unwrap();
    assert!(!folder.join("Warm film.apstyle").exists());
    super::super::rename(&folder, "Bright", "Brighter").unwrap();
    assert_eq!(super::super::taken(&folder), ["Brighter"]);
    assert!(super::super::delete(&folder, "Warm film").is_err());
    let _ = std::fs::remove_dir_all(&folder);
}

mod darktable;

/// Each built-in style sets exactly what its darktable preset sets: one entry of the
/// preset's tool, on where the tool is switched, each parameter the preset sets off the
/// module's defaults read back to darktable's value as the tool's control shows it, and no
/// other control, so every other is at its default, darktable's. Its British English name
/// is its file's.
#[test]
fn each_built_in_style_reads_back_to_its_darktable_preset_control_by_control() {
    use apertrust_tools::ControlValue::Number;
    let words = Translator::new(Locale::EnGb);
    let stored = files::builtin_styles();
    let ids: Vec<&str> = stored.iter().map(|(id, _)| *id).collect();
    let presets = darktable::presets();
    let expected: Vec<&str> = presets.iter().map(|preset| preset.id).collect();
    assert_eq!(ids, expected);
    for (preset, (id, file)) in presets.iter().zip(&stored) {
        assert_eq!(builtin_name(id, &words), file.name, "{id}");
        let [entry] = file.entries.as_slice() else {
            panic!("{id}: one entry");
        };
        let (tool, values) = tool_values(entry, TOOLS, apertrust_tools::RETIRED).unwrap();
        assert_eq!(tool, preset.tool, "{id}");
        let definition = darktable::tool(preset.tool).definition();
        assert!(definition.in_styles, "{id}");
        assert_eq!(values.enabled(), definition.switched, "{id}");
        let wanted: Vec<(&str, apertrust_tools::ControlValue)> = (preset.params.iter())
            .map(|(param, value)| darktable::control(preset.tool, param, *value))
            .collect();
        for control in definition.controls {
            let read = values.get(control.id);
            let want = wanted
                .iter()
                .find(|(c, _)| *c == control.id)
                .map(|(_, v)| v);
            match (read, want) {
                (Some(Number(read)), Some(Number(want))) => {
                    // One unit in the last place: AgX's contrast goes through `powf`.
                    let near = (read - want).abs() <= 2.0 * f32::EPSILON * want.abs().max(1.0);
                    assert!(near, "{id}: {} is {read}, darktable's {want}", control.id);
                }
                (read, want) => assert_eq!(read, want, "{id}: {}", control.id),
            }
        }
    }
}

/// A built-in style applies with its file recorded as built in, and clears by its name;
/// its menu holds Apply or Clear, then Copy to Custom Styles…, never Rename Style… or
/// Delete Style…, and F2 and Delete do nothing on it.
#[test]
fn a_built_in_style_applies_clears_and_copies_but_is_never_renamed_or_deleted() {
    let styles = builtins();
    let lomo = styles.iter().find(|style| style.name == "Lomo").unwrap();
    let Some(Command::ApplyStyle {
        name, file, tools, ..
    }) = apply(lomo, StyleMode::Append, &[])
    else {
        panic!("a style applied");
    };
    assert_eq!((name.as_str(), file.as_str()), ("Lomo", "builtin:lomo"));
    assert_eq!(tools.len(), 1);
    let state = Drawn {
        listed: styles.clone(),
        applied: Some(Vec::new()),
        ..with_photo()
    };
    let mut harness = drawn(Builtin::Dark, state);
    harness.run();
    harness
        .get_by_role_and_label(Role::TreeItem, "Lomo")
        .click();
    harness.run();
    harness
        .get_by_role_and_label(Role::TreeItem, "Lomo")
        .focus();
    harness.run();
    harness.key_press(egui::Key::F2);
    harness.run();
    harness.key_press(egui::Key::Delete);
    harness.run();
    assert_eq!(
        harness.state().actions,
        [Action::Apply("builtin:lomo".to_owned())]
    );
    harness.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F10);
    harness.run();
    for gone in ["Rename Style…", "Delete Style…", "Clear"] {
        let item = harness.query_by_role_and_label(Role::MenuItem, gone);
        assert!(item.is_none(), "{gone}");
    }
    harness.get_by_role_and_label(Role::MenuItem, "Apply");
    harness
        .get_by_role_and_label(Role::MenuItem, "Copy to Custom Styles…")
        .click();
    harness.run();
    assert_eq!(
        harness.state().actions[1..],
        [Action::Copy("Lomo".to_owned())]
    );
    // Applied, in another language, its file still says which: its menu offers Clear, of
    // the name it was applied under.
    let in_ukrainian = StyleState {
        name: "Ломо".to_owned(),
        file: "builtin:lomo".to_owned(),
        tools: vec!["vignette".to_owned()],
    };
    let state = Drawn {
        listed: styles,
        applied: Some(vec![in_ukrainian]),
        ..with_photo()
    };
    let mut harness = drawn(Builtin::Dark, state);
    harness.run();
    harness
        .get_by_role_and_label(Role::TreeItem, "Lomo")
        .focus();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F10);
    harness.run();
    harness
        .get_by_role_and_label(Role::MenuItem, "Clear")
        .click();
    harness.run();
    assert_eq!(
        harness.state().actions,
        [Action::Clear {
            name: "Ломо".to_owned(),
            file: "builtin:lomo".to_owned(),
        }]
    );
}

/// Copy to Custom Styles… writes the built-in's entries into the styles folder under the
/// name given, refusing one a style there holds; a person's style takes no built-in
/// style's name as shown.
#[test]
fn a_copy_lands_in_custom_styles_and_no_style_takes_a_built_in_s_name() {
    use crate::presets::SheetContent as _;
    let folder = std::env::temp_dir().join(format!("apertrust-styles-copy-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    super::super::copy(&folder, "lomo", " My lomo ", "0.1.0").unwrap();
    assert_eq!(super::super::taken(&folder), ["My lomo"]);
    let copied = super::super::stored(&folder);
    let (_, builtin) = (files::builtin_styles().into_iter())
        .find(|(id, _)| *id == "lomo")
        .unwrap();
    assert_eq!(copied[0].entries, builtin.entries);
    let again = super::super::copy(&folder, "lomo", "My lomo", "0.1.0");
    assert!(matches!(again, Err(files::PresetError::NameTaken(_))));
    let words = Translator::new(Locale::EnGb);
    let refused = super::super::refused(&folder, &words);
    for name in ["My lomo", "Lomo", "Black and white|HP5+", "Natural colour"] {
        assert!(refused.iter().any(|r| r == name), "{name}");
    }
    let rename = super::super::RenameDialog::new("My lomo".to_owned(), refused);
    assert_eq!(rename.button(&words), ("Rename".to_owned(), true));
    let _ = std::fs::remove_dir_all(&folder);
}

/// Copy to Custom Styles… opens on the built-in's name and "copy", numbered from 2 where a
/// style has that, and Copy takes it at once.
#[test]
fn a_copy_s_dialog_opens_on_a_free_name_copy_takes() {
    use crate::presets::SheetContent as _;
    let words = Translator::new(Locale::EnGb);
    let builtins = || vec!["Lomo".to_owned(), "Natural colour".to_owned()];
    for (mine, name) in [
        (vec![], "Lomo copy"),
        (vec!["Lomo copy"], "Lomo copy 2"),
        (vec!["Lomo copy", "Lomo copy 2"], "Lomo copy 3"),
    ] {
        let mine = mine.into_iter().map(str::to_owned);
        let refused: Vec<String> = mine.chain(builtins()).collect();
        assert_eq!(super::super::copy_name("Lomo", &refused, &words), name);
        let copy = super::super::RenameDialog::copy("Lomo".to_owned(), refused, &words);
        assert_eq!(copy.button(&words), ("Copy".to_owned(), true));
        assert_eq!(
            copy.output(),
            crate::dialog_host::Output::StyleRename {
                from: "Lomo".to_owned(),
                to: name.to_owned(),
                copy: true,
            }
        );
    }
}

/// A person's style named as a built-in is, an older file's, applies itself, by its file,
/// and only the row of the style applied, the person's or the built-in, shows it applied.
#[test]
fn a_person_s_style_named_as_a_built_in_is_never_taken_for_it() {
    let mut styles = builtins();
    styles.push(style("Lomo"));
    // Which "Lomo" row is which: the person's, first, sets Grain, the built-in Vignetting.
    let rows = |harness: &Harness<'static, Drawn>| -> Vec<String> {
        (harness.query_all_by_role_and_label(Role::TreeItem, "Lomo"))
            .filter_map(|node| {
                node.accesskit_node()
                    .data()
                    .description()
                    .map(str::to_owned)
            })
            .collect()
    };
    let person_s = StyleState {
        name: "Lomo".to_owned(),
        file: "Lomo.apstyle".to_owned(),
        tools: vec!["grain".to_owned()],
    };
    let built_in = StyleState {
        file: "builtin:lomo".to_owned(),
        tools: vec!["vignette".to_owned()],
        ..person_s.clone()
    };
    for (applied, said) in [
        (person_s, ["applied, Grain", "Vignetting"]),
        (built_in, ["Grain", "applied, Vignetting"]),
    ] {
        let state = Drawn {
            listed: styles.clone(),
            applied: Some(vec![applied]),
            ..with_photo()
        };
        let mut harness = drawn(Builtin::Dark, state);
        harness.run();
        assert_eq!(rows(&harness), said);
    }
    let state = Drawn {
        listed: styles,
        applied: Some(Vec::new()),
        ..with_photo()
    };
    let mut harness = drawn(Builtin::Dark, state);
    harness.run();
    (harness.query_all_by_role_and_label(Role::TreeItem, "Lomo"))
        .find(|node| node.accesskit_node().data().description() == Some("Grain"))
        .unwrap()
        .click();
    harness.run();
    assert_eq!(
        harness.state().actions,
        [Action::Apply("Lomo.apstyle".to_owned())]
    );
}

/// A built-in applied in another language is applied again under the name it was applied
/// under, never twice; a person's style of that name applies under its own.
#[test]
fn a_built_in_applied_again_after_a_language_change_keeps_its_name() {
    let styles = builtins();
    let lomo = styles.iter().find(|style| style.name == "Lomo").unwrap();
    let in_ukrainian = StyleState {
        name: "Ломо".to_owned(),
        file: "builtin:lomo".to_owned(),
        tools: vec!["vignette".to_owned()],
    };
    let name =
        |style: &Listed, applied: &[StyleState]| match apply(style, StyleMode::Append, applied) {
            Some(Command::ApplyStyle { name, file, .. }) => (name, file),
            other => panic!("a style applied: {other:?}"),
        };
    let applied = std::slice::from_ref(&in_ukrainian);
    assert_eq!(
        name(lomo, applied),
        ("Ломо".to_owned(), "builtin:lomo".to_owned())
    );
    assert_eq!(
        name(lomo, &[]),
        ("Lomo".to_owned(), "builtin:lomo".to_owned())
    );
    let mut own = style("Ломо");
    own.path = PathBuf::from("/styles/Ломо.apstyle");
    assert_eq!(
        name(&own, applied),
        ("Ломо".to_owned(), "Ломо.apstyle".to_owned())
    );
}
