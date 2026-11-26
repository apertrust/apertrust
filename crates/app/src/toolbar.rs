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

//! What can be placed on the Toolbar (decision 92; `docs/gui.md`, Customising the Toolbar):
//! the parts, the app's own actions, every tool's items and the two spacings, each once,
//! in one stable order ([`placeable`]). An action that belongs to a tool is offered by that
//! tool (`ToolDefinition::toolbar`); only actions that belong to no tool are the app's,
//! those the registry gives an icon ([`actions::ICONS`]). The Toolbar draws a list from it
//! ([`listed`]), each action with its word under its icon ([`word`]), and the customise
//! mode's Add ▸ lists what a draft does not hold of it (`customise`), its Done writing the
//! draft back with the ids this build does not know in their places ([`merged`]).

use std::sync::OnceLock;

use apertrust_tools::utility::{UTILITIES, UtilityDefinition};
use apertrust_tools::{ActionEnd, End, Face, ProcessingDefinition, ToolbarItem};
use apertrust_ui::{Icon, Translator};

use crate::actions;

/// What kind of item a placeable one is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// One of the Toolbar's own parts, its id no registry action's.
    Part,
    /// An action of the app's own, which belongs to no tool.
    App,
    /// An item the tool of this id offers.
    Tool(&'static str),
    /// `space` or `flexible-space`, placed any number of times.
    Spacing,
}

/// One item a person may place on the Toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placeable {
    /// Its id in a workspace's `toolbar` list: a part's or a spacing's own, an action's its
    /// registry id, as `undo` and `tool-flip-rotate_ccw`.
    pub id: &'static str,
    pub kind: Kind,
    /// None for a part drawn otherwise, as the jobs' spinner, and for the spacings.
    pub icon: Option<Icon>,
    /// Its words' message id, an action's its registry message; none for the save state
    /// and the spacings, which have none yet.
    pub words: Option<&'static str>,
}

/// The parts (decision 92), by id, each with its icon and words. The menu button is none:
/// on Windows and Linux it stands first whatever the list holds, so an old file's
/// `menu-button` is an id the set lacks. The assistant joins them in Phase 6.
const PARTS: [(&str, Option<Icon>, Option<&str>); 3] = [
    ("jobs", None, Some("jobs-name")),
    ("save-state", None, None),
    ("cursors", None, Some("cursor-well")),
];

/// The spacings: 16 points, and a share of what is left.
const SPACINGS: [&str; 2] = ["space", "flexible-space"];

/// Everything that can be placed on the Toolbar, each once: the parts, the app's own
/// actions the registry holds with their icons, the items `tools` then `utilities` offer,
/// in their order, then the spacings. A tool's item takes its registry action's id and
/// words, its icon by name or, with none, its button's; one the registry lacks is left out.
pub fn placeable(
    tools: &[&'static ProcessingDefinition],
    utilities: &'static [UtilityDefinition],
) -> Vec<Placeable> {
    let parts = PARTS.map(|(id, icon, words)| Placeable {
        id,
        kind: Kind::Part,
        icon,
        words,
    });
    let app = actions::ICONS.iter().filter_map(|&(id, icon)| {
        let action = actions::action(id)?;
        Some(Placeable {
            id: action.id.0,
            kind: Kind::App,
            icon: Some(icon),
            words: Some(action.message),
        })
    });
    let processing = tools.iter().flat_map(|tool| {
        let id = tool.tool.id;
        let items = tool.tool.toolbar.iter();
        items.filter_map(move |item| {
            let button = button_icon(tool, item.action);
            offered(id, &format!("tool-{id}-{}", item.action), item, button)
        })
    });
    let utilities = utilities.iter().flat_map(|utility| {
        let id = utility.tool.id;
        let items = utility.tool.toolbar.iter();
        items.filter_map(move |item| offered(id, item.action, item, None))
    });
    let spacings = SPACINGS.map(|id| Placeable {
        id,
        kind: Kind::Spacing,
        icon: None,
        words: None,
    });
    parts
        .into_iter()
        .chain(app)
        .chain(processing)
        .chain(utilities)
        .chain(spacings)
        .collect()
}

/// The item `tool` offers, `action` its registry id, with its icon by name or else
/// `button`, its button's; none where the registry lacks the action.
fn offered(
    tool: &'static str,
    action: &str,
    item: &ToolbarItem,
    button: Option<Icon>,
) -> Option<Placeable> {
    let action = actions::named(action)?;
    Some(Placeable {
        id: action.id.0,
        kind: Kind::Tool(tool),
        icon: item.icon.and_then(named).or(button),
        words: Some(action.message),
    })
}

/// Everything that can be placed on the Toolbar, as [`placeable`] lists it for the tools
/// this build has, worked out once.
pub fn set() -> &'static [Placeable] {
    static SET: OnceLock<Vec<Placeable>> = OnceLock::new();
    SET.get_or_init(|| {
        let tools = apertrust_tools::TOOLS.iter();
        let tools: Vec<&'static ProcessingDefinition> =
            tools.map(|tool| tool.definition()).collect();
        placeable(&tools, UTILITIES)
    })
}

/// What a list of item ids draws, in its order (decision 92): each id [`set`] holds, a part
/// or an action once, at its first place, the spacings as often as listed. An id the set
/// lacks, a removed tool's or a newer build's, is skipped, never taken out of the list.
pub fn listed<'a>(list: impl IntoIterator<Item = &'a str>) -> Vec<&'static Placeable> {
    let mut drawn: Vec<&'static Placeable> = Vec::new();
    for id in list {
        let Some(placed) = set().iter().find(|placed| placed.id == id) else {
            continue;
        };
        if placed.kind == Kind::Spacing || !drawn.iter().any(|d| d.id == id) {
            drawn.push(placed);
        }
    }
    drawn
}

/// The list Done writes for `draft`, the customise mode's list, which started as `start`,
/// the workspace's (decision 92): `draft` with each id of `start` this build does not know,
/// a removed tool's or a newer build's, written back right after the known id it followed in
/// `start`, or, where that is no longer in `draft`, the nearest known id before it still
/// there, else at the start. An old list's `menu-button` is the one unknown id dropped.
pub fn merged(start: &[String], draft: &[&str]) -> Vec<String> {
    let known = |id: &str| set().iter().any(|placed| placed.id == id);
    // Each unknown id with the place in `draft` it follows, none for the start.
    let mut unknown: Vec<(Option<usize>, &str)> = Vec::new();
    for (at, id) in start.iter().enumerate() {
        if known(id) || id == "menu-button" {
            continue;
        }
        let anchor = (start[..at].iter().rev())
            .filter(|before| known(before))
            .find_map(|before| draft.iter().position(|id| id == before));
        unknown.push((anchor, id));
    }
    let after = |place: Option<usize>| {
        (unknown.iter())
            .filter(move |(anchor, _)| *anchor == place)
            .map(|(_, id)| (*id).to_owned())
    };
    let mut written: Vec<String> = after(None).collect();
    for (at, id) in draft.iter().enumerate() {
        written.push((*id).to_owned());
        written.extend(after(Some(at)));
    }
    written
}

/// The words' message of the item `id` can place, as [`set`] has it: an action's its
/// message, a part's its own; none for an id the set lacks, the save state and the
/// spacings.
pub fn words(id: &str) -> Option<&'static str> {
    set().iter().find(|placed| placed.id == id)?.words
}

impl Placeable {
    /// Whether it goes under » before the app's own actions as the Toolbar narrows: a
    /// tool's item does (decision 92).
    pub fn folds_first(&self) -> bool {
        matches!(self.kind, Kind::Tool(_))
    }
}

/// An item's word under its icon (decision 92, Labels), its words being the message
/// `message`: the message's `.toolbar` attribute, short and on one line, else the message.
pub fn word(words: &Translator, message: &str) -> String {
    (words.attribute(message, "toolbar")).unwrap_or_else(|| words.text(message))
}

/// `apertrust-ui`'s icon called `name`, if it has one.
fn named(name: &str) -> Option<Icon> {
    Icon::ALL.into_iter().find(|icon| icon.name() == name)
}

/// The icon of `tool`'s button `action`, if a row's end holds it as an icon action.
fn button_icon(tool: &ProcessingDefinition, action: &str) -> Option<Icon> {
    tool.rows().find_map(|row| {
        row.end().iter().find_map(|item| match item {
            End::Action(ActionEnd {
                id,
                face: Face::Icon(icon),
                ..
            }) if *id == action => Some(crate::tools::icon(*icon)),
            _ => None,
        })
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use apertrust_ui::{Locale, Theme, Translator};
    use eframe::egui;
    use egui_kittest::Harness;

    use super::*;
    use crate::actions::Trigger;

    fn tools() -> Vec<&'static ProcessingDefinition> {
        let tools = apertrust_tools::TOOLS.iter();
        tools.map(|tool| tool.definition()).collect()
    }

    /// Every item a tool offers, by its tool and its action's registry id.
    fn declared() -> Vec<(&'static str, String, &'static ToolbarItem)> {
        let processing = tools().into_iter().flat_map(|tool| {
            let id = tool.tool.id;
            let items = tool.tool.toolbar.iter();
            items.map(move |item| (id, format!("tool-{id}-{}", item.action), item))
        });
        let utilities = UTILITIES.iter().flat_map(|utility| {
            let id = utility.tool.id;
            let items = utility.tool.toolbar.iter();
            items.map(move |item| (id, item.action.to_owned(), item))
        });
        processing.chain(utilities).collect()
    }

    /// Every action a tool offers is one the registry holds, so none is left out.
    #[test]
    fn every_declared_action_is_a_registry_action() {
        for (tool, action, _) in declared() {
            assert!(
                actions::named(&action).is_some(),
                "{tool}: no action {action}"
            );
        }
    }

    /// The app's own actions belong to no tool: none is a processing tool's action,
    /// offered or not, nor an item a utility offers, and each is pressed, never held.
    #[test]
    fn no_app_action_is_a_tool_s() {
        let tools = actions::tool_actions()
            .iter()
            .map(|action| action.id.0.to_owned());
        let offered: BTreeSet<String> = (declared().into_iter().map(|(_, id, _)| id))
            .chain(tools)
            .collect();
        for (id, _) in actions::ICONS {
            assert!(!offered.contains(id.0), "{} is a tool's too", id.0);
            let action = actions::action(id).expect("in the registry");
            assert_ne!(action.trigger, Trigger::Hold, "{} is held", id.0);
        }
    }

    /// Every icon a tool names is one `apertrust-ui` has, and every item of an action has
    /// an icon, its button's where it names none.
    #[test]
    fn every_icon_name_is_one_apertrust_ui_has() {
        for (tool, action, item) in declared() {
            if let Some(name) = item.icon {
                assert!(named(name).is_some(), "{tool}'s {action}: no icon {name}");
            }
        }
        for placed in placeable(&tools(), UTILITIES) {
            if matches!(placed.kind, Kind::App | Kind::Tool(_)) {
                assert!(placed.icon.is_some(), "{} has no icon", placed.id);
            }
        }
    }

    /// Decision 92: the set is exactly the parts, the app's own actions the registry holds,
    /// the items the tools offer and the two spacings, in that order, each id once; a
    /// button's item takes its button's icon.
    #[test]
    fn the_set_is_the_parts_the_app_s_actions_the_tools_items_and_the_spacings() {
        let set = placeable(&tools(), UTILITIES);
        let ids: Vec<&str> = set.iter().map(|placed| placed.id).collect();
        let app = actions::ICONS.iter().map(|(id, _)| id.0);
        let app = app.filter(|id| actions::named(id).is_some());
        let declared: Vec<String> = declared().into_iter().map(|(_, id, _)| id).collect();
        let expected: Vec<&str> = (PARTS.iter().map(|(id, _, _)| *id))
            .chain(app)
            .chain(declared.iter().map(String::as_str))
            .chain(SPACINGS)
            .collect();
        assert_eq!(ids, expected);
        let once: BTreeSet<&str> = ids.iter().copied().collect();
        assert_eq!(once.len(), ids.len(), "an id twice: {ids:?}");
        let kinds: Vec<(&str, Kind)> = set.iter().map(|placed| (placed.id, placed.kind)).collect();
        for (id, kind) in [
            ("cursors", Kind::Part),
            ("import", Kind::App),
            ("guides", Kind::App),
            ("previous-photo", Kind::App),
            ("tool-flip-rotate_ccw", Kind::Tool("flip")),
            ("undo", Kind::Tool("history")),
            ("show-collections", Kind::Tool("collections")),
            ("export-again", Kind::Tool("export")),
            ("flexible-space", Kind::Spacing),
        ] {
            assert!(kinds.contains(&(id, kind)), "{id}: not {kind:?}");
        }
        let icon = |id: &str| set.iter().find(|placed| placed.id == id).unwrap().icon;
        assert_eq!(icon("tool-flip-rotate_ccw"), Some(Icon::RotateLeft));
        assert_eq!(icon("tool-flip-rotate_cw"), Some(Icon::RotateRight));
        assert_eq!(icon("show-collections"), Some(Icon::Library));
        assert_eq!(icon("guides"), Some(Icon::Guides));
    }

    /// A tool's item folds under » before the app's own actions; nothing else is marked.
    #[test]
    fn a_tool_s_items_fold_first() {
        let set = placeable(&tools(), UTILITIES);
        let first = |id: &str| {
            set.iter()
                .find(|placed| placed.id == id)
                .unwrap()
                .folds_first()
        };
        for id in ["reset", "export", "tool-flip-rotate_cw", "show-collections"] {
            assert!(first(id), "{id}");
        }
        for id in ["import", "open", "settings", "cursors", "space"] {
            assert!(!first(id), "{id}");
        }
    }

    /// Parts' and spacings' ids are their own: none is a registry action's.
    #[test]
    fn no_part_s_id_is_a_registry_id() {
        for id in PARTS.iter().map(|(id, _, _)| *id).chain(SPACINGS) {
            assert!(actions::named(id).is_none(), "{id} is a registry action's");
        }
    }

    fn strings(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| (*id).to_owned()).collect()
    }

    /// Decision 92: Done writes an unknown id back after the known id it followed, wherever
    /// that now stands; after the nearest earlier one still there where it was removed;
    /// at the start where none is. An old `menu-button` is dropped.
    #[test]
    fn unknown_ids_are_kept_at_their_anchors() {
        let start = strings(&[
            "gone-first",
            "import",
            "gone-a",
            "undo",
            "gone-b",
            "menu-button",
            "guides",
        ]);
        // Guides moved first, Undo removed: gone-b follows Import, Undo's nearest earlier.
        let written = merged(&start, &["guides", "import", "space"]);
        assert_eq!(
            written,
            [
                "gone-first",
                "guides",
                "import",
                "gone-a",
                "gone-b",
                "space"
            ]
        );
        // Nothing known left before them: at the start.
        let written = merged(&start, &["guides"]);
        assert_eq!(written, ["gone-first", "gone-a", "gone-b", "guides"]);
        assert_eq!(merged(&[], &["undo"]), ["undo"]);
    }

    /// Decision 92: the menu button is no item of the list, so the set holds no
    /// `menu-button`, and an old file's is skipped where it stands.
    #[test]
    fn the_set_holds_no_menu_button() {
        assert!(set().iter().all(|placed| placed.id != "menu-button"));
        let drawn = listed(["import", "menu-button", "undo"]);
        let ids: Vec<&str> = drawn.iter().map(|placed| placed.id).collect();
        assert_eq!(ids, ["import", "undo"]);
    }

    /// Every item's words exist in every language.
    #[test]
    fn every_item_s_words_exist_in_every_language() {
        let set = placeable(&tools(), UTILITIES);
        for locale in [Locale::EnGb, Locale::EnUs, Locale::Uk, Locale::Pl] {
            let tr = Translator::new(locale);
            for placed in &set {
                if let Some(words) = placed.words {
                    assert!(
                        tr.has(words),
                        "{}: no {words} for {}",
                        locale.id(),
                        placed.id
                    );
                }
            }
        }
    }

    /// Every action's word under its icon (decision 92, Labels) is its language's own
    /// `.toolbar` attribute, never British English's in its place, and at most 96 points at
    /// the caption size, Regular, in every language.
    #[test]
    fn every_action_s_word_is_its_own_and_at_most_96_points() {
        let set = placeable(&tools(), UTILITIES);
        let messages: Vec<&str> = (set.iter())
            .filter(|placed| matches!(placed.kind, Kind::App | Kind::Tool(_)))
            .filter_map(|placed| placed.words)
            .collect();
        // Every language; the pseudo-locale lengthens every word on purpose.
        let locales = Locale::ALL.into_iter().filter(|l| *l != Locale::Pseudo);
        let words: Vec<(Locale, &str, String)> = locales
            .flat_map(|locale| {
                let tr = Translator::new(locale);
                (messages.iter()).map(move |&message| {
                    assert!(
                        tr.has_attribute(message, "toolbar"),
                        "{}: {message} has no .toolbar word",
                        locale.id()
                    );
                    (locale, message, word(&tr, message))
                })
            })
            .collect();
        let mut harness = Harness::builder()
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
            .build_ui_state(
                |ui, widths: &mut Vec<f32>| {
                    let caption = apertrust_ui::theme(ui.ctx()).text.caption;
                    let font = egui::FontId::proportional(caption);
                    let measure = |(_, _, word): &(Locale, &str, String)| {
                        let galley = ui.painter().layout_no_wrap(
                            word.clone(),
                            font.clone(),
                            egui::Color32::WHITE,
                        );
                        galley.size().x
                    };
                    *widths = words.iter().map(measure).collect();
                },
                Vec::new(),
            );
        harness.run();
        for ((locale, message, word), width) in words.iter().zip(harness.state()) {
            assert!(
                *width <= 96.0,
                "{}: {message}'s word {word:?} is {width} points",
                locale.id()
            );
        }
    }
}
