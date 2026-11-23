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

use std::time::{Duration, Instant};

use apertrust_ui::Translator;
use apertrust_ui::i18n::Locale;

use crate::settings::testing::Folder;

/// The Default workspace's tabs and their tools as decision 38 records them, top to
/// bottom, by tool id, those still to build included: Adjust in editing order, with the
/// edit's tone mapper, one of AgX, Sigmoid and Filmic RGB, at its place, and the Tone curve
/// at RGB curve's until RGB curve replaces it; Raw in processing order. The Photos tab
/// holds Collections, then the Export tool, its last (decisions 73 and 90); History its
/// section alone.
const DECISION_38: [(&str, &[&str]); 7] = [
    ("photos", &["collections", "export"]),
    (
        "adjust",
        &[
            "temperature",
            "channelmixerrgb",
            "exposure",
            "agx",
            "sigmoid",
            "filmicrgb",
            "toneequal",
            "bilat",
            "hazeremoval",
            "tone_curve",
            "rgblevels",
            "colorbalancergb",
            "colorequal",
            "primaries",
        ],
    ),
    (
        "detail",
        &["denoiseprofile", "sharpen", "diffuse", "retouch"],
    ),
    (
        "geometry",
        &["crop", "ashift", "flip", "lens", "cacorrectrgb"],
    ),
    ("effects", &["vignette", "grain", "soften", "bloom"]),
    (
        "raw",
        &[
            "rawprepare",
            "highlights",
            "demosaic",
            "colorin",
            "colorout",
        ],
    ),
    ("history", &["history"]),
];

/// The tabs that pin Scopes over their tools: the five editing tabs, as darktable's
/// histogram sits over its groups.
const SCOPES_PINNED: [&str; 5] = ["adjust", "detail", "geometry", "effects", "raw"];

fn default_layout() -> Layout {
    Layout::defaults(DEFAULT).expect("built in")
}

/// Holds the built-in workspace `layout` to `recorded`, its decision's tabs and their
/// tools: its tabs in order, each listing the built tools recorded for it in their order
/// and no other, Scopes pinned in `pinned` alone, each tool recorded in one tab only; a
/// whole file in the model's format, with no `seen`, nothing hidden, and no name nor any
/// tab's title, which the interface takes from Fluent.
fn holds_to(layout: &Layout, recorded: &[(&str, &[&str])], pinned: &[&str]) {
    assert_eq!(layout.version, VERSION);
    assert_eq!(layout.name, None, "a built-in's name is Fluent's");
    assert!(layout.seen.is_empty());
    assert!(layout.hidden.is_empty());
    let ids: Vec<&str> = layout.tabs.iter().map(|tab| tab.id.as_str()).collect();
    let recorded_ids: Vec<&str> = recorded.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, recorded_ids);
    let known = known();
    for (id, tools) in recorded {
        let expected: Vec<&str> = tools
            .iter()
            .copied()
            .filter(|t| known.contains(t))
            .collect();
        let order: Vec<&str> = layout.order[*id].iter().map(String::as_str).collect();
        assert_eq!(order, expected, "{id}");
        let kept = layout.pinned.get(*id).cloned().unwrap_or_default();
        let scopes = pinned.contains(id).then_some(vec![SCOPES.to_owned()]);
        assert_eq!(kept, scopes.unwrap_or_default(), "{id}");
    }
    let mut placed: Vec<&str> = recorded
        .iter()
        .flat_map(|(_, t)| t.iter().copied())
        .collect();
    let count = placed.len();
    placed.sort_unstable();
    placed.dedup();
    assert_eq!(placed.len(), count, "a tool placed twice");
    for tab in &layout.tabs {
        assert_eq!(
            tab.title, None,
            "{}: a built-in tab's title is Fluent's",
            tab.id
        );
    }
}

/// Holds the built-in workspace `layout` to the build: every tool and icon it names exists,
/// each tab's title is a message in every language, and the layout it shows holds the
/// invariants, with no warning. The layout shown.
fn names_only_what_exists(layout: &Layout) -> Shown {
    let known = known();
    for tab in &layout.tabs {
        let lists = [&layout.pinned, &layout.order];
        for tool in lists
            .iter()
            .flat_map(|l| l.get(&tab.id).into_iter().flatten())
        {
            assert!(known.contains(&tool.as_str()), "{}: no tool {tool}", tab.id);
        }
        let name = tab
            .icon
            .as_deref()
            .unwrap_or_else(|| panic!("{}: no icon", tab.id));
        assert!(icon(name).is_some(), "{}: no icon {name}", tab.id);
        for locale in Locale::ALL {
            let words = Translator::new(locale);
            assert!(
                words.has(&format!("tab-{}", tab.id)),
                "{locale:?}: {}",
                tab.id
            );
        }
    }
    let (shown, warnings) = resolve(layout, &[layout], &known);
    assert!(warnings.is_empty(), "{warnings:?}");
    holds_the_invariants(&shown, &known);
    shown
}

/// Holds `shown` to placing every tool of the Tools panel, none hidden: Scopes in each tab
/// of `pinned`, every other tool in one tab.
fn places_every_tool(shown: &Shown, pinned: &[&str]) {
    assert!(shown.hidden.is_empty(), "{:?}", shown.hidden);
    for tool in known() {
        let tabs = shown.tabs.iter().filter(|tab| tab.holds(tool)).count();
        let expected = if tool == SCOPES { pinned.len() } else { 1 };
        assert_eq!(tabs, expected, "{tool} in {tabs} tabs");
    }
}

/// Holds `shown` to decision 96's invariants for the tools `known` lists: each tab's tools
/// once, `pinned` and `order` sharing none; every tool in a tab shown or hidden, never
/// both; Export in a tab; nothing but `known`'s tools anywhere; at most twelve tabs.
fn holds_the_invariants(shown: &Shown, known: &[&str]) {
    assert!(shown.tabs.len() <= MOST_TABS);
    for tab in &shown.tabs {
        let tools: Vec<&str> = tab.tools().collect();
        let mut once = tools.clone();
        once.sort_unstable();
        once.dedup();
        assert_eq!(
            once.len(),
            tools.len(),
            "{}: a tool twice in {tools:?}",
            tab.id
        );
        for tool in tools {
            assert!(known.contains(&tool), "{}: {tool} is no tool", tab.id);
        }
    }
    for tool in known {
        let in_tab = shown.tabs.iter().any(|tab| tab.holds(tool));
        let hidden = shown.hidden.contains(tool);
        assert!(
            in_tab != hidden,
            "{tool}: in a tab {in_tab}, hidden {hidden}"
        );
    }
    for tool in &shown.hidden {
        assert!(known.contains(tool), "{tool} hidden but no tool");
    }
    for tool in &shown.beyond {
        assert!(shown.hidden.contains(tool), "{tool} beyond but not hidden");
    }
    assert!(shown.tabs.iter().any(|tab| tab.holds(EXPORT)), "no Export");
}

/// `text`'s layout, which must read.
fn parsed(text: &str) -> Layout {
    Layout::parse(text).expect("a workspace").0
}

/// The layout `text` shows over the Default workspace's defaults, with the build's tools.
fn layout_of(text: &str) -> (Shown, Vec<String>) {
    let defaults = default_layout();
    let (shown, warnings) = resolve(&parsed(text), &[&defaults], &known());
    holds_the_invariants(&shown, &known());
    (shown, warnings)
}

fn tab<'a>(shown: &'a Shown, id: &str) -> &'a Tab {
    let tab = shown.tabs.iter().find(|tab| tab.id == id);
    tab.unwrap_or_else(|| panic!("no tab {id}"))
}

/// The bundled Default workspace is decision 38's ([`holds_to`]).
#[test]
fn the_default_workspace_is_decision_38_s() {
    let layout = default_layout();
    holds_to(&layout, &DECISION_38, &SCOPES_PINNED);
}

/// The bundled Default workspace names only what the build has ([`names_only_what_exists`]).
/// Detail, with nothing built, is drawn disabled; the others can be chosen.
#[test]
fn the_default_workspace_names_only_tools_and_icons_that_exist() {
    let shown = names_only_what_exists(&default_layout());
    places_every_tool(&shown, &SCOPES_PINNED);
    let enabled: Vec<bool> = shown.tabs.iter().map(Tab::enabled).collect();
    assert_eq!(enabled, [true, true, false, true, true, true, true]);
    assert_eq!(tab(&shown, "photos").order.last(), Some(&EXPORT));
    let titles: Vec<&Title> = shown.tabs.iter().map(|tab| &tab.title).collect();
    assert_eq!(titles[1], &Title::Message("tab-adjust".to_owned()));
    assert_eq!(tab(&shown, "geometry").icon, Icon::Correct);
}

/// The bundled Essentials workspace (decision 96): Photos, Adjust with Scopes pinned over
/// the tools most edits need, and History, as a whole file with no name, nothing in `seen`
/// or `hidden` and no tab's title; it names only what the build has
/// ([`names_only_what_exists`]), every other tool is in no tab, so hidden, and it opens on
/// Adjust.
#[test]
fn the_essentials_workspace_shows_the_tools_most_edits_need() {
    const RECORDED: [(&str, &[&str]); 3] = [
        ("photos", &["collections", "export"]),
        (
            "adjust",
            &[
                "temperature",
                "exposure",
                "agx",
                "toneequal",
                "colorbalancergb",
                "flip",
            ],
        ),
        ("history", &["history"]),
    ];
    let layout = Layout::defaults(ESSENTIALS).expect("built in");
    holds_to(&layout, &RECORDED, &["adjust"]);
    let shown = names_only_what_exists(&layout);
    for (id, tools) in RECORDED {
        let pinned: &[&str] = if id == "adjust" { &[SCOPES] } else { &[] };
        let tab = tab(&shown, id);
        assert_eq!(
            (tab.pinned.as_slice(), tab.order.as_slice()),
            (pinned, tools)
        );
    }
    let listed: Vec<&str> = RECORDED.iter().flat_map(|(_, t)| t.to_vec()).collect();
    let mut hidden: Vec<&str> = (known().into_iter())
        .filter(|tool| *tool != SCOPES && !listed.contains(tool))
        .collect();
    let mut shown_hidden = shown.hidden.clone();
    hidden.sort_unstable();
    shown_hidden.sort_unstable();
    assert_eq!(shown_hidden, hidden);
    assert!(shown.hidden.contains(&"highlights"));
    assert!(shown.tabs.iter().all(Tab::enabled));
    let (workspace, warnings) = Workspaces::builtin().open(ESSENTIALS);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(workspace.shown, shown);
    assert_eq!(workspace.opening(), workspace.position("adjust").unwrap());
}

/// The Hidden tab (decision 96): id `hidden`, its title `tab-hidden`, its icon Lucide's
/// `eye-off`, nothing pinned; the workspace's hidden tools, processing tools in pipeline
/// order, first at the top, as Default's Raw lists them, then utilities. Default hides
/// none; Essentials every tool its three tabs leave out; Export is never in it.
#[test]
fn the_hidden_tab_lists_the_hidden_tools_in_processing_order() {
    let workspaces = Workspaces::builtin();
    let (default, _) = workspaces.open(DEFAULT);
    let tab = default.hidden_tab();
    assert_eq!(tab.id, HIDDEN);
    assert_eq!(tab.title, Title::Message("tab-hidden".to_owned()));
    assert_eq!(tab.icon, Icon::EyeOff);
    assert!(tab.pinned.is_empty() && tab.order.is_empty());
    let (essentials, _) = workspaces.open(ESSENTIALS);
    let order = &essentials.hidden_tab().order;
    let built: Vec<&str> = [
        "rawprepare",
        "highlights",
        "demosaic",
        "hazeremoval",
        "colorin",
        "channelmixerrgb",
        "primaries",
        "tone_curve",
        "grain",
        "soften",
        "vignette",
        "colorout",
    ]
    .into_iter()
    .filter(|tool| known().contains(tool))
    .collect();
    assert_eq!(order, &built);
    assert!(essentials.hidden_tab().pinned.is_empty());
    for tool in order {
        assert!(essentials.hides(tool), "{tool}");
        assert_eq!(essentials.holding(tool), None, "{tool}");
    }
    // Default's Raw lists its tools in the order the Hidden tab would.
    let raw = default.tab(default.position("raw").unwrap()).unwrap();
    let mut sorted = raw.order.clone();
    sorted.sort_by_key(|tool| hidden_order(tool));
    assert_eq!(sorted, raw.order);
    // Utilities after the processing tools, in the build's order; Export never, though the
    // file hides it.
    let text = "hidden = [\"export\", \"history\", \"exposure\", \"scopes\", \"rawprepare\"]
                [[tabs]]\nid = \"photos\"\n";
    let (shown, _) = layout_of(text);
    assert!(!shown.hidden.contains(&EXPORT));
    let at = |tool: &str| shown.hidden.iter().position(|t| *t == tool).unwrap();
    assert!(at("rawprepare") < at("exposure"));
    let utilities: Vec<&str> = (shown.hidden.iter().copied())
        .filter(|tool| !processing(tool))
        .collect();
    let first_utility = at(utilities[0]);
    assert!(shown.hidden[first_utility..].iter().all(|t| !processing(t)));
    assert!(at("scopes") < at("history"));
}

/// The workspaces are the built-in ones, Default then Essentials, each named by its message
/// in every language, then a person's own, each `<id>.toml` that reads, by name, its file's
/// `name` or else its id: a built-in's changed copy takes its place, never listed twice; a
/// file whose name is no workspace id, or not TOML's, is not listed, a damaged one set
/// aside and one listing no tab refused, each with a warning.
#[test]
fn the_workspaces_are_the_built_in_ones_then_a_person_s_own_by_name() {
    let (none, warnings) = Workspaces::read(None);
    assert!(warnings.is_empty());
    assert_eq!(none, Workspaces::builtin());
    let folder = Folder::new("workspace-listed");
    let write = |name: &str, text: &str| std::fs::write(folder.0.join(name), text).unwrap();
    let tabs = "[[tabs]]\nid = \"photos\"\n";
    write("portraits.toml", &format!("name = \"Portraits\"\n{tabs}"));
    write("film.toml", tabs);
    write(
        "essentials.toml",
        &format!("{tabs}[order]\nphotos = [\"export\"]\n"),
    );
    write("Bad Name.toml", tabs);
    write("notes.txt", tabs);
    write("broken.toml", "[[tabs]\n");
    write("empty.toml", "name = \"Empty\"\n");
    let (workspaces, warnings) = Workspaces::read(Some(&folder.0));
    assert_eq!(warnings.len(), 2, "{warnings:?}");
    assert!(folder.0.join("broken.toml.bak").exists());
    let ids: Vec<&str> = workspaces.listed().iter().map(|l| l.id.as_str()).collect();
    assert_eq!(ids, [DEFAULT, ESSENTIALS, "film", "portraits"]);
    let names: Vec<Title> = workspaces.listed().iter().map(Listed::name).collect();
    assert_eq!(
        names,
        [
            Title::Message("workspace-default".to_owned()),
            Title::Message("workspace-essentials".to_owned()),
            Title::Own("film".to_owned()),
            Title::Own("Portraits".to_owned()),
        ]
    );
    for locale in Locale::ALL {
        let words = Translator::new(locale);
        for id in builtin_ids() {
            assert!(words.has(&format!("workspace-{id}")), "{locale:?}: {id}");
        }
    }
    let essentials = &workspaces.listed()[1];
    assert!(essentials.builtin);
    assert_ne!(
        Some(&essentials.layout),
        Layout::defaults(ESSENTIALS).as_ref()
    );
    // Each opens as listed; an id of none, Default, with a warning.
    let (portraits, warnings) = workspaces.open("portraits");
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(portraits.id, "portraits");
    let (shown, warnings) = workspaces.open("empty");
    assert_eq!(shown.id, DEFAULT);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
}

/// A person's own workspace's new tools go by the Default workspace's tabs alone: a tab
/// Default lacks, as darktable's `base`, takes none, its own title its id.
#[test]
fn a_person_s_own_new_tools_go_by_default_s_tabs_alone() {
    let folder = Folder::new("workspace-own-base");
    let text = "[[tabs]]\nid = \"photos\"\n[[tabs]]\nid = \"base\"\n\
                [order]\nphotos = [\"collections\", \"export\"]\nbase = [\"exposure\"]\n";
    std::fs::write(folder.0.join("mine.toml"), text).unwrap();
    let (workspace, warnings) = Workspace::open(Some(&folder.0), "mine");
    assert!(warnings.is_empty(), "{warnings:?}");
    holds_the_invariants(&workspace.shown, &known());
    let base = tab(&workspace.shown, "base");
    assert!(base.pinned.is_empty());
    assert_eq!(base.order, ["exposure"]);
    assert_eq!(base.title, Title::Own("base".to_owned()));
    for tool in ["temperature", "agx", "demosaic", "scopes"] {
        assert!(workspace.shown.hidden.contains(&tool), "{tool}");
    }
}

/// With no file, the Default workspace is its defaults, and opens on Adjust, which holds
/// the tools every edit starts with.
#[test]
fn the_default_workspace_with_no_file_is_its_defaults() {
    let folder = Folder::new("workspace-none");
    let (workspace, warnings) = Workspace::open(Some(&folder.0), DEFAULT);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(workspace, Workspace::builtin_default());
    assert_eq!(workspace.layout, default_layout());
    assert_eq!(workspace.opening(), workspace.position("adjust").unwrap());
    assert!(!workspace.newer);
    let (none, warnings) = Workspace::open(None, DEFAULT);
    assert!(warnings.is_empty());
    assert_eq!(none, workspace);
}

/// Step 1: an id in both lists stays pinned, one twice in a list keeps its first place, and
/// an id this build does not know, as an import tool's, is skipped.
#[test]
fn each_tab_takes_its_lists_once_each() {
    let (shown, _) = layout_of(
        "[[tabs]]\nid = \"photos\"\n[[tabs]]\nid = \"skin\"\n\
         [pinned]\nskin = [\"scopes\", \"exposure\"]\n\
         [order]\nphotos = [\"collections\", \"export\"]\n\
         skin = [\"exposure\", \"temperature\", \"vignette\", \"temperature\", \"ingest\"]\n\
         seen = []\n",
    );
    let skin = tab(&shown, "skin");
    assert_eq!(skin.pinned, ["scopes", "exposure"]);
    assert_eq!(skin.order, ["temperature", "vignette"]);
    assert_eq!(skin.title, Title::Own("skin".to_owned()));
    assert_eq!(skin.icon, OWN_TAB_ICON);
}

/// Step 2: a tool `hidden` names is in no tab, whatever the lists say, but Export, which
/// is never hidden.
#[test]
fn hidden_wins_over_the_lists_but_for_export() {
    let (shown, _) = layout_of(
        "hidden = [\"vignette\", \"export\"]\n\
         [[tabs]]\nid = \"photos\"\n[[tabs]]\nid = \"effects\"\n\
         [order]\nphotos = [\"collections\", \"export\"]\neffects = [\"vignette\", \"grain\"]\n",
    );
    assert_eq!(tab(&shown, "effects").order, ["grain", "soften"]);
    assert!(shown.hidden.contains(&"vignette"));
    assert_eq!(tab(&shown, "photos").order, ["collections", "export"]);
    assert!(!shown.hidden.contains(&EXPORT));
}

/// Step 3: a new tool, one the file names nowhere, goes into each tab shown whose id is one
/// of its default tabs, after its nearest preceding neighbour there that the tab holds,
/// pinned where its default is pinned, Scopes into each; with none of its default tabs, it
/// is hidden.
#[test]
fn a_new_tool_joins_its_default_tabs_after_its_neighbour() {
    let known = known();
    let seen: Vec<&str> = known
        .iter()
        .copied()
        .filter(|id| !["exposure", "scopes", "grain"].contains(id))
        .collect();
    let seen = seen
        .iter()
        .map(|id| format!("\"{id}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let (shown, _) = layout_of(&format!(
        "seen = [{seen}]\n\
         [[tabs]]\nid = \"adjust\"\n[[tabs]]\nid = \"raw\"\n[[tabs]]\nid = \"skin\"\n\
         [order]\nadjust = [\"agx\", \"channelmixerrgb\"]\nraw = [\"demosaic\"]\n\
         skin = [\"collections\", \"export\"]\n"
    ));
    // After its nearest preceding neighbour in Adjust's defaults that the tab holds,
    // Colour calibration, though the tab lists that after AgX.
    assert_eq!(
        tab(&shown, "adjust").order,
        ["agx", "channelmixerrgb", "exposure"]
    );
    assert_eq!(tab(&shown, "adjust").pinned, ["scopes"]);
    assert_eq!(tab(&shown, "raw").pinned, ["scopes"]);
    // A tab of a person's own is no tool's default tab.
    assert!(tab(&shown, "skin").pinned.is_empty());
    // Grain's only default tab, Effects, is not in the workspace.
    assert!(shown.hidden.contains(&"grain"));
}

/// A new tool with no default neighbour the tab holds goes to its top; one the defaults
/// list first, too.
#[test]
fn a_new_tool_with_no_neighbour_goes_to_the_top() {
    let (shown, _) = layout_of(
        "seen = [\"vignette\", \"soften\"]\n\
         [[tabs]]\nid = \"effects\"\n[order]\neffects = [\"soften\", \"vignette\"]\n",
    );
    // Grain follows Vignetting in Effects' defaults.
    assert_eq!(
        tab(&shown, "effects").order,
        ["soften", "vignette", "grain", EXPORT]
    );
    let (shown, _) = shown_with_seen_but(&["temperature"], "adjust", &["exposure"]);
    assert_eq!(tab(&shown, "adjust").order[0], "temperature");
}

/// The layout of a workspace of one tab, `tab`, listing `order`, whose `seen` names every
/// tool but `new`.
fn shown_with_seen_but(new: &[&str], tab: &str, order: &[&str]) -> (Shown, Vec<String>) {
    let quoted = |ids: &mut dyn Iterator<Item = &str>| {
        ids.map(|id| format!("\"{id}\""))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let known = known();
    let seen = quoted(&mut known.iter().copied().filter(|id| !new.contains(id)));
    let order = quoted(&mut order.iter().copied());
    layout_of(&format!(
        "seen = [{seen}]\n[[tabs]]\nid = \"{tab}\"\n[order]\n{tab} = [{order}]\n"
    ))
}

/// Step 4: a tool `seen` names that is in no tab is hidden, as a hand edit that drops a
/// tool from every list hides it.
#[test]
fn a_seen_tool_in_no_tab_is_hidden() {
    let (shown, _) = shown_with_seen_but(&[], "adjust", &["exposure"]);
    assert_eq!(tab(&shown, "adjust").order, ["exposure", EXPORT]);
    assert!(shown.hidden.contains(&"temperature"));
    assert!(shown.hidden.contains(&"collections"));
    assert!(shown.beyond.is_empty());
}

/// Step 5: Export in no tab shown goes to the end of the Photos tab where it is shown,
/// else of the first tab that can be chosen, else of the first tab.
#[test]
fn export_goes_to_the_photos_tab_else_the_first_holding_tools() {
    let (shown, _) = layout_of(
        "[[tabs]]\nid = \"skin\"\n[[tabs]]\nid = \"photos\"\n\
         [order]\nskin = [\"exposure\"]\nphotos = [\"collections\"]\n",
    );
    assert_eq!(tab(&shown, "photos").order, ["collections", EXPORT]);
    let (shown, _) = shown_with_seen_but(&[], "skin", &[]);
    assert_eq!(tab(&shown, "skin").order, [EXPORT]);
    let (shown, _) = layout_of(
        "seen = [\"exposure\", \"scopes\"]\n\
         [[tabs]]\nid = \"look\"\n[[tabs]]\nid = \"skin\"\n\
         [pinned]\nlook = [\"scopes\"]\n[order]\nskin = [\"exposure\"]\n",
    );
    // Scopes alone leaves a tab with nothing to work with.
    assert_eq!(tab(&shown, "skin").order, ["exposure", EXPORT]);
}

/// At most twelve tabs show; one past them is not, with a warning, its only tools hidden
/// but never written into `hidden` (`beyond`), and no new tool placed there.
#[test]
fn a_tab_past_the_twelfth_is_not_shown() {
    let mut text = String::new();
    for i in 1..=12 {
        text.push_str(&format!("[[tabs]]\nid = \"tab-{i}\"\n"));
    }
    text.push_str("[[tabs]]\nid = \"effects\"\n[order]\neffects = [\"vignette\"]\n");
    text.push_str("tab-1 = [\"collections\", \"export\"]\n");
    let (shown, warnings) = layout_of(&text);
    assert_eq!(shown.tabs.len(), MOST_TABS);
    assert!(!shown.tabs.iter().any(|tab| tab.id == "effects"));
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("effects"));
    assert_eq!(shown.beyond, ["vignette"]);
    // Grain and Soften, new, have no default tab shown.
    assert!(shown.hidden.contains(&"grain"));
    assert!(!shown.beyond.contains(&"grain"));
}

/// A tab listed twice shows once, the first; the Hidden tab's id is no file's tab; a
/// built-in tab's title is its message, and its icon the built-in's where the file names
/// none; a written title and icon win; an icon this build lacks is the own tabs' icon,
/// with a warning.
#[test]
fn tabs_take_their_titles_and_icons() {
    let (shown, warnings) = layout_of(
        "[[tabs]]\nid = \"adjust\"\n[[tabs]]\nid = \"adjust\"\n[[tabs]]\nid = \"hidden\"\n\
         [[tabs]]\nid = \"raw\"\ntitle = \"Sensor\"\nicon = \"detail\"\n\
         [[tabs]]\nid = \"skin\"\ntitle = \"Skin\"\nicon = \"no-such-icon\"\n\
         [order]\nadjust = [\"exposure\"]\n",
    );
    let ids: Vec<&str> = shown.tabs.iter().map(|tab| tab.id.as_str()).collect();
    assert_eq!(ids, ["adjust", "raw", "skin"]);
    assert_eq!(warnings.len(), 3, "{warnings:?}");
    let adjust = tab(&shown, "adjust");
    assert_eq!(
        (&adjust.title, adjust.icon),
        (&Title::Message("tab-adjust".into()), Icon::Adjust)
    );
    let raw = tab(&shown, "raw");
    assert_eq!(
        (&raw.title, raw.icon),
        (&Title::Own("Sensor".into()), Icon::Detail)
    );
    let skin = tab(&shown, "skin");
    assert_eq!(
        (&skin.title, skin.icon),
        (&Title::Own("Skin".into()), OWN_TAB_ICON)
    );
}

/// A file that is no TOML, or no UTF-8, is set aside as `<id>.toml.bak`, with a warning, and
/// the built-in's defaults apply.
#[test]
fn a_damaged_file_is_set_aside_and_the_defaults_apply() {
    for (name, bytes) in [
        ("toml", &b"[[tabs]\nid = "[..]),
        ("utf8", &b"name = \"\xff\"\n"[..]),
    ] {
        let folder = Folder::new(&format!("workspace-damaged-{name}"));
        let file = folder.0.join("default.toml");
        std::fs::write(&file, bytes).unwrap();
        let (workspace, warnings) = Workspace::open(Some(&folder.0), DEFAULT);
        assert_eq!(workspace, Workspace::builtin_default());
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("set aside"), "{warnings:?}");
        assert!(!file.exists());
        assert_eq!(
            std::fs::read(folder.0.join("default.toml.bak")).unwrap(),
            bytes
        );
    }
}

/// A damaged file never replaces an earlier one set aside: the next free numbered name
/// takes it.
#[test]
fn an_earlier_backup_is_kept() {
    let folder = Folder::new("workspace-backups");
    let file = folder.0.join("default.toml");
    for (i, text) in ["first [", "second [", "third ["].into_iter().enumerate() {
        std::fs::write(&file, text).unwrap();
        let (_, warnings) = Workspace::open(Some(&folder.0), DEFAULT);
        let name = [
            "default.toml.bak",
            "default.toml.1.bak",
            "default.toml.2.bak",
        ][i];
        assert!(warnings[0].contains(name), "{warnings:?}");
    }
    let kept = |name: &str| std::fs::read_to_string(folder.0.join(name)).unwrap();
    assert_eq!(kept("default.toml.bak"), "first [");
    assert_eq!(kept("default.toml.1.bak"), "second [");
    assert_eq!(kept("default.toml.2.bak"), "third [");
    assert!(!file.exists());
}

/// A file listing no tab is refused, with a warning, and left as it is: the built-in's
/// defaults apply, and a person's own does not open, the Default workspace shown.
#[test]
fn a_file_listing_no_tab_is_refused() {
    let folder = Folder::new("workspace-no-tab");
    for id in [DEFAULT, "portraits"] {
        let text = "version = 1\nhidden = [\"grain\"]\n[[tabs]]\nid = \"hidden\"\n";
        std::fs::write(folder.0.join(format!("{id}.toml")), text).unwrap();
        let (workspace, warnings) = Workspace::open(Some(&folder.0), id);
        assert_eq!(workspace, Workspace::builtin_default(), "{id}");
        assert!(
            warnings.iter().any(|w| w.contains("lists no tab")),
            "{warnings:?}"
        );
        let kept = std::fs::read_to_string(folder.0.join(format!("{id}.toml"))).unwrap();
        assert_eq!(kept, text);
    }
}

/// A newer version's file is read for what this build knows, and marked never to be
/// written, with a warning.
#[test]
fn a_newer_version_s_file_is_read_and_never_written() {
    let folder = Folder::new("workspace-newer");
    let text = "version = 2\nmood = \"calm\"\n[[tabs]]\nid = \"photos\"\nshape = \"round\"\n\
                [order]\nphotos = [\"collections\", \"export\", \"styles\"]\n";
    std::fs::write(folder.0.join("default.toml"), text).unwrap();
    let (workspace, warnings) = Workspace::open(Some(&folder.0), DEFAULT);
    assert!(workspace.newer);
    assert!(warnings.iter().any(|w| w.contains("newer")), "{warnings:?}");
    assert_eq!(
        tab(&workspace.shown, "photos").order,
        ["collections", EXPORT]
    );
    holds_the_invariants(&workspace.shown, &known());
}

/// A person's own workspace is used by naming it in settings and writing its file; its
/// new tools go by the Default workspace's tabs. One settings name without a file, or by an
/// id no file can have, opens the Default workspace, with a warning, a changed copy of it
/// where the folder holds one.
#[test]
fn a_person_s_own_workspace_opens_from_its_file() {
    let folder = Folder::new("workspace-own");
    let text = "name = \"Portraits\"\n\
                [[tabs]]\nid = \"photos\"\n[[tabs]]\nid = \"skin\"\ntitle = \"Skin\"\nicon = \"adjust\"\n\
                [[tabs]]\nid = \"adjust\"\n\
                [order]\nphotos = [\"collections\"]\nskin = [\"exposure\"]\nadjust = [\"agx\"]\n";
    std::fs::write(folder.0.join("portraits.toml"), text).unwrap();
    let (workspace, warnings) = Workspace::open(Some(&folder.0), "portraits");
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(workspace.id, "portraits");
    assert_eq!(workspace.layout.name.as_deref(), Some("Portraits"));
    let shown = &workspace.shown;
    holds_the_invariants(shown, &known());
    assert_eq!(tab(shown, "photos").order, ["collections", EXPORT]);
    assert_eq!(tab(shown, "skin").order, ["exposure"]);
    // Every other tool of Adjust's defaults joins it, in their order around AgX.
    let adjust = tab(shown, "adjust");
    assert_eq!(adjust.pinned, ["scopes"]);
    assert_eq!(
        adjust.order[..4],
        ["temperature", "channelmixerrgb", "agx", "toneequal"]
    );
    // Raw's, Effects' and Geometry's tools have no tab here; History's neither.
    for tool in ["demosaic", "vignette", "flip", "history"] {
        assert!(shown.hidden.contains(&tool), "{tool}");
    }
    assert_eq!(workspace.opening(), workspace.position("skin").unwrap());
    // Changed: the Default workspace's copy in the folder.
    let changed = "[[tabs]]\nid = \"photos\"\n[order]\nphotos = [\"export\"]\n";
    std::fs::write(folder.0.join("default.toml"), changed).unwrap();
    for id in ["landscapes", "Bad id", ""] {
        let (workspace, warnings) = Workspace::open(Some(&folder.0), id);
        assert_eq!(workspace.id, DEFAULT, "{id}");
        assert_eq!(warnings.len(), 1, "{id}: {warnings:?}");
        assert_eq!(workspace.layout, parsed(changed));
    }
}

/// A workspace id is lower-case letters, digits and hyphens.
#[test]
fn a_workspace_id_is_its_file_s_name() {
    for id in ["default", "portraits", "film-2", "a"] {
        assert!(valid_id(id), "{id}");
    }
    for id in ["", "Portraits", "film_2", "../x", "a.b", "é"] {
        assert!(!valid_id(id), "{id}");
    }
}

/// What cannot be read of a file is skipped with a warning, and the rest read: a tab
/// with no id, a title that is no text, lists that are no lists.
#[test]
fn what_cannot_be_read_is_skipped() {
    let (layout, warnings) = Layout::parse(
        "version = \"one\"\nname = 3\nhidden = \"grain\"\nseen = [\"exposure\", 4]\n\
         [[tabs]]\ntitle = \"No id\"\n[[tabs]]\nid = \"skin\"\ntitle = 5\n\
         [pinned]\nskin = \"scopes\"\n[order]\nskin = [\"exposure\"]\n",
    )
    .expect("a workspace");
    assert_eq!(warnings.len(), 7, "{warnings:?}");
    assert_eq!(layout.version, VERSION);
    assert_eq!(layout.name, None);
    assert_eq!(layout.tabs.len(), 1);
    assert_eq!(layout.tabs[0].title, None);
    assert!(layout.hidden.is_empty());
    assert_eq!(layout.seen, ["exposure"]);
    assert_eq!(layout.pinned["skin"], Vec::<String>::new());
    assert_eq!(layout.order["skin"], ["exposure"]);
    assert_eq!(Layout::parse("name = \"x\"\n"), Err(Refused::NoTab));
    assert!(matches!(Layout::parse("[["), Err(Refused::Damaged(_))));
}

/// The Default workspace as built, with no file.
fn default_workspace() -> Workspace {
    Workspaces::builtin().open(DEFAULT).0
}

/// The tab `id` of `workspace`'s layout shown.
fn shown_tab<'a>(workspace: &'a Workspace, id: &str) -> &'a Tab {
    tab(&workspace.shown, id)
}

/// `change` applied to `workspace`, which must move something; the layout shown holds the
/// invariants after it.
fn moved(workspace: &mut Workspace, change: Move) {
    assert!(workspace.apply(&change), "{change:?} moved nothing");
    holds_the_invariants(&workspace.shown, &known());
}

/// Settled, a layout shows what it showed: written whole, a built-in workspace reads back
/// as the same layout, every tool in `seen` and none hidden by it.
#[test]
fn a_settled_layout_shows_what_it_showed() {
    for id in [DEFAULT, ESSENTIALS] {
        let workspace = Workspaces::builtin().open(id).0;
        let known = known();
        let settled = workspace.layout.settled(&workspace.shown, &known);
        let defaults = Layout::defaults(id).unwrap();
        let (shown, warnings) = resolve(&settled, &[&defaults], &known);
        assert!(warnings.is_empty(), "{id}: {warnings:?}");
        assert_eq!(shown, workspace.shown, "{id}");
        for tool in &known {
            assert!(settled.seen.iter().any(|seen| seen == tool), "{id}: {tool}");
        }
        let hidden: Vec<&str> = settled.hidden.iter().map(String::as_str).collect();
        assert_eq!(hidden, workspace.shown.hidden, "{id}");
    }
}

/// Settling keeps what this build does not know at its place: an id after the tool it
/// followed, one first where it followed none, a tab's lists the file has no tab for, and
/// a hidden id.
#[test]
fn settling_keeps_unknown_ids_in_their_places() {
    let layout = parsed(
        "hidden = [\"future-hidden\"]\n\
         [[tabs]]\nid = \"photos\"\n[[tabs]]\nid = \"adjust\"\n\
         [order]\nphotos = [\"collections\", \"export\"]\n\
         adjust = [\"future-first\", \"exposure\", \"future-after\", \"temperature\"]\n\
         elsewhere = [\"grain\"]\n",
    );
    let defaults = default_layout();
    let known = known();
    let (shown, _) = resolve(&layout, &[&defaults], &known);
    let settled = layout.settled(&shown, &known);
    // Adjust's tools the file names nowhere join it as new, after their neighbours.
    let adjust = &settled.order["adjust"];
    assert_eq!(adjust[..3], ["future-first", "exposure", "future-after"]);
    assert!(adjust[3..].iter().any(|t| t == "temperature"));
    assert_eq!(settled.order["elsewhere"], ["grain"]);
    assert_eq!(
        settled.hidden.first().map(String::as_str),
        Some("future-hidden")
    );
}

/// Pin Tool puts a tool at the end of its tab's pinned list; Unpin Tool at the top of the
/// scrolling list. Neither touches another tab, and pinning a pinned tool moves nothing.
#[test]
fn pin_and_unpin() {
    let mut workspace = default_workspace();
    let pin = |tool| Move::Pin {
        tool,
        tab: "adjust".to_owned(),
    };
    let unpin = |tool| Move::Unpin {
        tool,
        tab: "adjust".to_owned(),
    };
    moved(&mut workspace, pin("exposure"));
    let tab = shown_tab(&workspace, "adjust");
    assert_eq!(tab.pinned, ["scopes", "exposure"]);
    assert!(!tab.order.contains(&"exposure"));
    assert!(!workspace.apply(&pin("exposure")));
    moved(&mut workspace, unpin("exposure"));
    let tab = shown_tab(&workspace, "adjust");
    assert_eq!(tab.pinned, ["scopes"]);
    assert_eq!(tab.order.first(), Some(&"exposure"));
    assert!(!workspace.apply(&unpin("exposure")));
    // Scopes unpinned in Adjust stays pinned in Detail.
    moved(&mut workspace, unpin(SCOPES));
    assert_eq!(shown_tab(&workspace, "detail").pinned, [SCOPES]);
    assert_eq!(shown_tab(&workspace, "adjust").order.first(), Some(&SCOPES));
}

/// Move Tool Up and Move Tool Down move a tool one place in its own list, pinned or not,
/// and nothing at either end; the place screen readers hear counts the pinned first.
#[test]
fn up_and_down_stop_at_the_ends() {
    let mut workspace = default_workspace();
    let up = |tool| Move::Up {
        tool,
        tab: "effects".to_owned(),
    };
    let down = |tool| Move::Down {
        tool,
        tab: "effects".to_owned(),
    };
    let order = |w: &Workspace| shown_tab(w, "effects").order.clone();
    let before = order(&workspace);
    let (first, last) = (before[0], before[before.len() - 1]);
    assert!(!workspace.apply(&up(first)));
    assert!(!workspace.apply(&down(last)));
    moved(&mut workspace, down(first));
    assert_eq!(order(&workspace)[..2], [before[1], before[0]]);
    moved(&mut workspace, up(first));
    assert_eq!(order(&workspace), before);
    // The place: Scopes pinned first, so the first of the scrolling list is second.
    let at = workspace.position("effects").unwrap();
    let place = workspace.place(first, at).unwrap();
    assert_eq!((place.at, place.of), (2, before.len() + 1));
    assert!(!place.up && place.down && !place.pinned);
    // Alone in the pinned list, Scopes moves neither way.
    let scopes = workspace.place(SCOPES, at).unwrap();
    assert!(scopes.pinned && !scopes.up && !scopes.down);
    assert!(!workspace.apply(&down(SCOPES)));
}

/// A move of `tool` from the tab `from` to the tab `to`.
fn to_tab(tool: &'static str, from: &str, to: &str) -> Move {
    Move::ToTab {
        tool,
        from: Some(from.to_owned()),
        to: to.to_owned(),
    }
}

/// Move Tool to Tab takes a tool out of its tab into another, unpinned, as a drop on the
/// tab's icon: a processing tool at its place in the pipeline among the tab's processing
/// tools, a utility at the top. A tool in several tabs leaves only the one it moves from;
/// a tab holding it already takes nothing.
#[test]
fn move_to_tab_places_a_tool_as_a_drop_on_its_icon() {
    let mut workspace = default_workspace();
    // Vignetting, after the processing tool of Adjust nearest before it in the pipeline,
    // whatever order Adjust lists them in.
    moved(&mut workspace, to_tab("vignette", "effects", "adjust"));
    assert!(!shown_tab(&workspace, "effects").holds("vignette"));
    let adjust = &shown_tab(&workspace, "adjust").order;
    let at = adjust.iter().position(|t| *t == "vignette").unwrap();
    let (_, place) = hidden_order("vignette");
    let nearest = (adjust.iter())
        .filter(|tool| **tool != "vignette" && !hidden_order(tool).0)
        .filter(|tool| hidden_order(tool).1 < place)
        .max_by_key(|tool| hidden_order(tool).1)
        .expect("a processing tool before it");
    assert_eq!(adjust[at - 1], *nearest);
    // Raw's Demosaic, early, lands before the first that runs after it.
    moved(&mut workspace, to_tab("demosaic", "raw", "effects"));
    let effects = &shown_tab(&workspace, "effects").order;
    assert_eq!(effects.first(), Some(&"demosaic"));
    // A utility goes to the top.
    moved(&mut workspace, to_tab("collections", "photos", "adjust"));
    let adjust = &shown_tab(&workspace, "adjust").order;
    assert_eq!(adjust.first(), Some(&"collections"));
    // Scopes leaves Adjust alone; Effects keeps it; a tab holding it takes nothing.
    moved(&mut workspace, to_tab(SCOPES, "adjust", "photos"));
    assert!(!shown_tab(&workspace, "adjust").holds(SCOPES));
    assert!(shown_tab(&workspace, "effects").holds(SCOPES));
    assert!(!workspace.apply(&to_tab(SCOPES, "effects", "photos")));
}

/// A move to a tab takes the tool out of `hidden` too, where a hand edit hid it after the
/// move was made and before it was written, so the tool shows in the tab it was moved to.
#[test]
fn a_move_to_a_tab_unhides_a_tool_a_hand_edit_hid() {
    // As the file's writer replays a move on the file it read again: the layout as read,
    // the move applied to it directly.
    let mut layout = default_workspace().layout;
    layout.hidden.push("exposure".to_owned());
    assert!(layout.apply(&to_tab("exposure", "adjust", "raw"), &known()));
    assert!(!layout.hidden.iter().any(|t| t == "exposure"));
    assert!(
        layout
            .order
            .get("raw")
            .is_some_and(|raw| raw.iter().any(|t| t == "exposure"))
    );
}

/// Remove Tool takes a tool out of every tab into `hidden`, never Export; Show Tool in Tab
/// brings a hidden one back to a tab, out of `hidden`, and `section-show` to its first
/// default tab shown, else the first tab shown that can be chosen.
#[test]
fn remove_and_show() {
    let mut workspace = default_workspace();
    moved(&mut workspace, Move::Hide { tool: "vignette" });
    assert!(workspace.hides("vignette"));
    assert!(workspace.layout.hidden.iter().any(|t| t == "vignette"));
    assert!(workspace.hidden_tab().holds("vignette"));
    assert!(!workspace.apply(&Move::Hide { tool: "vignette" }));
    // Scopes leaves every tab.
    moved(&mut workspace, Move::Hide { tool: SCOPES });
    assert!(workspace.tabs().iter().all(|tab| !tab.holds(SCOPES)));
    assert!(!workspace.apply(&Move::Hide { tool: EXPORT }));
    assert!(workspace.holding(EXPORT).is_some());
    // Back to its default tab.
    assert_eq!(workspace.show_in("vignette"), workspace.position("effects"));
    let show = Move::ToTab {
        tool: "vignette",
        from: None,
        to: "effects".to_owned(),
    };
    moved(&mut workspace, show);
    assert!(!workspace.hides("vignette"));
    assert!(!workspace.layout.hidden.iter().any(|t| t == "vignette"));
    // Essentials has no Effects: the first tab that can be chosen.
    let essentials = Workspaces::builtin().open(ESSENTIALS).0;
    assert!(essentials.hides("vignette"));
    assert_eq!(
        essentials.show_in("vignette"),
        essentials.position("photos")
    );
}

/// A move made in a workspace is kept for it, so switching back shows it.
#[test]
fn a_move_is_kept_for_the_switch_back() {
    let mut workspaces = Workspaces::builtin();
    let (mut workspace, _) = workspaces.open(DEFAULT);
    moved(&mut workspace, Move::Hide { tool: "grain" });
    workspaces.keep(&workspace);
    let (again, _) = workspaces.open(DEFAULT);
    assert!(again.hides("grain"));
    assert_eq!(again.shown, workspace.shown);
}

/// The workspace `id`'s file in `folder`, as written.
fn written(folder: &Folder, id: &str) -> Layout {
    let file = folder.0.join(format!("{id}.toml"));
    parsed(&std::fs::read_to_string(file).expect("written"))
}

/// `change`, made in `workspace`, handed to `files` at `now`.
fn move_in(files: &mut Files, workspace: &mut Workspace, change: Move, now: Instant) {
    moved(workspace, change.clone());
    files.moved(workspace, change, now);
}

/// The moves are written once the layout has stood still for a second, not before; a
/// built-in workspace's first write is its file whole, read back as the layout shown.
#[test]
fn moves_are_written_once_the_layout_has_stood_still() {
    let folder = Folder::new("workspace-still");
    let mut files = Files::new(folder.0.clone());
    let mut workspace = default_workspace();
    let start = Instant::now();
    let pin = Move::Pin {
        tool: "exposure",
        tab: "adjust".to_owned(),
    };
    move_in(&mut files, &mut workspace, pin, start);
    let soon = files.poll(start + Duration::from_millis(400));
    assert_eq!(soon, Some(Duration::from_millis(600)));
    files.wait();
    assert!(!folder.0.join("default.toml").exists(), "written too soon");
    // Another move starts the second again.
    let later = start + Duration::from_millis(900);
    let hide = Move::Hide { tool: "grain" };
    move_in(&mut files, &mut workspace, hide, later);
    assert!(files.poll(start + files::DELAY).is_some());
    files.wait();
    assert!(!folder.0.join("default.toml").exists(), "written too soon");
    assert_eq!(files.poll(later + files::DELAY), None);
    files.wait();
    let layout = written(&folder, DEFAULT);
    assert_eq!(layout.pinned["adjust"], ["scopes", "exposure"]);
    assert!(layout.hidden.iter().any(|t| t == "grain"));
    assert!(known().iter().all(|t| layout.seen.iter().any(|s| s == t)));
    assert_eq!(layout.tabs, default_layout().tabs);
    let (read, warnings) = Workspace::open(Some(&folder.0), DEFAULT);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(read.shown, workspace.shown);
    // Nothing more moved, nothing more to write.
    assert_eq!(files.poll(later + Duration::from_secs(5)), None);
}

/// A switch writes at once, and so does exit; with nothing moved, nothing is written.
#[test]
fn a_switch_and_exit_write_at_once() {
    let folder = Folder::new("workspace-at-once");
    let mut files = Files::new(folder.0.clone());
    files.write_now();
    files.wait();
    assert!(
        folder.files().is_empty(),
        "nothing moved: {:?}",
        folder.files()
    );
    let now = Instant::now();
    let mut workspace = default_workspace();
    move_in(
        &mut files,
        &mut workspace,
        Move::Hide { tool: "grain" },
        now,
    );
    files.write_now();
    files.wait();
    assert!(
        written(&folder, DEFAULT)
            .hidden
            .iter()
            .any(|t| t == "grain")
    );
    let mut essentials = Workspaces::builtin().open(ESSENTIALS).0;
    let show = Move::ToTab {
        tool: "grain",
        from: None,
        to: "adjust".to_owned(),
    };
    move_in(&mut files, &mut essentials, show, now);
    files.flush();
    let adjust = &written(&folder, ESSENTIALS).order["adjust"];
    assert!(adjust.iter().any(|t| t == "grain"));
}

/// A write reads the file again and applies the moves to it as it is, so a hand edit made
/// meanwhile stays, with what this build does not know: a key, a tool's id.
#[test]
fn a_write_keeps_a_hand_edit() {
    let folder = Folder::new("workspace-hand-edit");
    let mut files = Files::new(folder.0.clone());
    let mut workspace = default_workspace();
    let edited = "version = 1\ntoolbar = [\"mark\", \"undo\"]\nhidden = [\"bloom\"]\n\
                  [[tabs]]\nid = \"photos\"\nicon = \"folder\"\n\
                  [[tabs]]\nid = \"adjust\"\nicon = \"adjust\"\n\
                  [order]\nphotos = [\"export\", \"collections\"]\n\
                  adjust = [\"future-tool\", \"exposure\", \"temperature\"]\n";
    std::fs::write(folder.0.join("default.toml"), edited).unwrap();
    let pin = Move::Pin {
        tool: "exposure",
        tab: "adjust".to_owned(),
    };
    move_in(&mut files, &mut workspace, pin, Instant::now());
    files.flush();
    let text = std::fs::read_to_string(folder.0.join("default.toml")).unwrap();
    let layout = parsed(&text);
    assert_eq!(layout.tabs.len(), 2, "the hand edit's tabs");
    assert_eq!(layout.order["photos"], ["export", "collections"]);
    // Scopes, which the file names nowhere, is new to it, and pinned as Default pins it.
    assert_eq!(layout.pinned["adjust"], ["scopes", "exposure"]);
    assert_eq!(layout.order["adjust"][0], "future-tool");
    assert!(layout.hidden.iter().any(|t| t == "bloom"));
    let table: Table = text.parse().unwrap();
    assert_eq!(table["toolbar"].as_array().map(Vec::len), Some(2));
}

/// A newer build's file is never written: a workspace read from one takes no move, and one
/// that became a newer build's since it was read is left as it is.
#[test]
fn a_newer_build_s_file_is_never_overwritten() {
    let folder = Folder::new("workspace-newer-write");
    let newer = "version = 2\n[[tabs]]\nid = \"photos\"\n[order]\nphotos = [\"export\"]\n";
    let file = folder.0.join("default.toml");
    std::fs::write(&file, newer).unwrap();
    let (mut workspace, _) = Workspace::open(Some(&folder.0), DEFAULT);
    assert!(workspace.newer);
    let mut files = Files::new(folder.0.clone());
    let hide = Move::Hide {
        tool: "collections",
    };
    move_in(&mut files, &mut workspace, hide.clone(), Instant::now());
    files.flush();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), newer);
    // Read as this build's, then replaced by a newer build's.
    let mut files = Files::new(folder.0.clone());
    let mut workspace = default_workspace();
    move_in(&mut files, &mut workspace, hide, Instant::now());
    files.flush();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), newer);
}

/// A person's own workspace whose file is gone since read is not written again; a
/// built-in's damaged since is set aside and written from its bundled file, and one deleted
/// by hand is written from its bundled file too, the moves not yet written applied, as if
/// it had never been written.
#[test]
fn a_file_gone_or_damaged_since_read() {
    let folder = Folder::new("workspace-gone");
    let hide = [Move::Hide { tool: "grain" }];
    let own = folder.0.join("portraits.toml");
    let mut kept = files::Kept::default();
    let wrote = files::write(&own, "portraits", false, &hide, &mut kept).unwrap();
    assert_eq!(wrote, files::Wrote::Left);
    assert!(!own.exists());
    let default = folder.0.join("default.toml");
    std::fs::write(&default, "[[").unwrap();
    let mut kept = files::Kept::default();
    assert_eq!(
        files::write(&default, DEFAULT, true, &hide, &mut kept).unwrap(),
        files::Wrote::Written
    );
    assert!(folder.0.join("default.toml.bak").exists());
    let layout = written(&folder, DEFAULT);
    assert!(layout.hidden.iter().any(|t| t == "grain"));
    assert_eq!(layout.tabs, default_layout().tabs);
    // Deleted by hand: the bundled file, with the next move alone.
    std::fs::remove_file(&default).unwrap();
    let pin = [Move::Pin {
        tool: "exposure",
        tab: "adjust".to_owned(),
    }];
    assert_eq!(
        files::write(&default, DEFAULT, true, &pin, &mut kept).unwrap(),
        files::Wrote::Written
    );
    let layout = written(&folder, DEFAULT);
    assert!(!layout.hidden.iter().any(|t| t == "grain"));
    assert_eq!(layout.pinned["adjust"], ["scopes", "exposure"]);
}

/// A damaged file set aside, then a write that fails, keeps the moves: the retry finds no
/// file and writes it from what was set in its place, a person's own included.
#[test]
fn a_failed_write_after_a_set_aside_is_tried_again() {
    let folder = Folder::new("workspace-retry");
    let file = folder.0.join("portraits.toml");
    let own = "name = \"Portraits\"\n[[tabs]]\nid = \"skin\"\n[order]\nskin = [\"exposure\"]\n";
    std::fs::write(&file, own).unwrap();
    let mut kept = files::Kept::default();
    let pin = [Move::Pin {
        tool: "exposure",
        tab: "skin".to_owned(),
    }];
    assert_eq!(
        files::write(&file, "portraits", false, &pin, &mut kept).unwrap(),
        files::Wrote::Written
    );
    std::fs::write(&file, "[[").unwrap();
    let hide = [Move::Hide { tool: "grain" }];
    files::fail_next_write(&file);
    assert!(files::write(&file, "portraits", false, &hide, &mut kept).is_err());
    assert!(!file.exists() && kept.set_aside);
    assert_eq!(
        files::write(&file, "portraits", false, &hide, &mut kept).unwrap(),
        files::Wrote::Written
    );
    let layout = written(&folder, "portraits");
    assert_eq!(layout.name.as_deref(), Some("Portraits"));
    assert_eq!(layout.pinned["skin"], ["exposure"]);
    assert!(layout.hidden.iter().any(|t| t == "grain"));
    assert!(!kept.set_aside);
}

/// A file that lists no tab when it is written, perhaps for a moment as a person edits
/// it, is left as it is, and its moves are kept to try again: with its tabs back, a retry
/// with no other move writes them.
#[test]
fn a_file_listing_no_tab_at_the_write_is_tried_again() {
    let folder = Folder::new("workspace-no-tab-write");
    let file = folder.0.join("default.toml");
    let edited = "name = \"x\"\n";
    std::fs::write(&file, edited).unwrap();
    let mut kept = files::Kept::default();
    let hide = [Move::Hide { tool: "grain" }];
    assert!(files::write(&file, DEFAULT, true, &hide, &mut kept).is_err());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), edited);
    // Through the writer: kept, then written once the tabs are back.
    let mut files = Files::new(folder.0.clone());
    let mut workspace = default_workspace();
    move_in(
        &mut files,
        &mut workspace,
        Move::Hide { tool: "grain" },
        Instant::now(),
    );
    files.write_now();
    files.wait();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), edited);
    std::fs::write(
        &file,
        "[[tabs]]\nid = \"effects\"\n[order]\neffects = [\"grain\"]\n",
    )
    .unwrap();
    files.write_now();
    files.wait();
    assert!(
        written(&folder, DEFAULT)
            .hidden
            .iter()
            .any(|t| t == "grain")
    );
}

/// Written, each list's items one a line, so a person edits them by hand.
#[test]
fn a_written_file_lists_one_item_a_line() {
    let folder = Folder::new("workspace-pretty");
    let file = folder.0.join("default.toml");
    let hide = [Move::Hide { tool: "grain" }, Move::Hide { tool: "bloom" }];
    assert_eq!(
        files::write(&file, DEFAULT, true, &hide, &mut files::Kept::default()).unwrap(),
        files::Wrote::Written
    );
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(
        text.contains("hidden = [\n    \"grain\",\n    \"bloom\",\n]"),
        "{text}"
    );
    assert!(text.contains("seen = [\n    \""), "{text}");
}

/// A move to a tab a hand edit took away since moves nothing, the tool staying in its
/// tab; nor does one to a tab past the twelfth.
#[test]
fn a_move_to_a_tab_gone_since_moves_nothing() {
    let workspace = default_workspace();
    let known = known();
    let mut layout = workspace.layout.settled(&workspace.shown, &known);
    layout.tabs.retain(|tab| tab.id != "raw");
    let change = to_tab("vignette", "effects", "raw");
    assert!(!layout.apply(&change, &known));
    assert!(layout.order["effects"].iter().any(|t| t == "vignette"));
}

/// A processing tool moved to a tab with no processing tool lands before Export, the
/// Photos tab's last (decision 90), never after it.
#[test]
fn a_processing_tool_lands_before_export() {
    let mut workspace = default_workspace();
    moved(&mut workspace, to_tab("vignette", "effects", "photos"));
    let photos = &shown_tab(&workspace, "photos").order;
    assert_eq!(photos.last(), Some(&EXPORT));
    let at = |id| photos.iter().position(|t| *t == id).unwrap();
    assert!(at("vignette") < at(EXPORT));
}

/// A tab the person emptied of every tool but Scopes cannot be chosen until it holds one
/// again, but takes a tool moved or shown there; one still to come, as Detail, takes none.
#[test]
fn an_emptied_tab_takes_a_tool_back() {
    let mut workspace = default_workspace();
    for tool in shown_tab(&workspace, "effects").order.clone() {
        moved(&mut workspace, Move::Hide { tool });
    }
    let effects = workspace.position("effects").unwrap();
    let detail = workspace.position("detail").unwrap();
    assert!(!workspace.tabs()[effects].enabled());
    let receiving = workspace.receiving();
    assert!(receiving[effects] && !receiving[detail]);
    assert_eq!(workspace.show_in("grain"), Some(effects));
    let show = Move::ToTab {
        tool: "grain",
        from: None,
        to: "effects".to_owned(),
    };
    moved(&mut workspace, show);
    assert!(workspace.tabs()[effects].enabled());
}

/// A workspace whose file a newer build wrote stays known as one through a move and a
/// switch back, so its file is never written.
#[test]
fn a_newer_build_s_workspace_stays_newer_through_a_move() {
    let folder = Folder::new("workspace-newer-move");
    let newer = "version = 2\n[[tabs]]\nid = \"photos\"\n[order]\nphotos = [\"export\"]\n";
    std::fs::write(folder.0.join("default.toml"), newer).unwrap();
    let (mut workspaces, _) = Workspaces::read(Some(&folder.0));
    let (mut workspace, _) = workspaces.open(DEFAULT);
    assert!(workspace.newer);
    moved(
        &mut workspace,
        Move::Hide {
            tool: "collections",
        },
    );
    assert!(workspace.newer);
    workspaces.keep(&workspace);
    let (again, _) = workspaces.open(DEFAULT);
    assert!(again.newer);
    assert!(again.hides("collections"));
}

/// Moving a tool up or down swaps it with its known neighbour; an id this build does not
/// know stays after the tool it followed, so a write keeps it there.
#[test]
fn up_and_down_keep_unknown_ids_by_their_neighbours() {
    let mut layout = parsed(
        "[[tabs]]\nid = \"adjust\"\n\
         [order]\nadjust = [\"exposure\", \"future-tool\", \"temperature\"]\n",
    );
    let up = Move::Up {
        tool: "temperature",
        tab: "adjust".to_owned(),
    };
    assert!(layout.apply(&up, &known()));
    assert_eq!(
        layout.order["adjust"],
        ["temperature", "exposure", "future-tool"]
    );
    let down = Move::Down {
        tool: "temperature",
        tab: "adjust".to_owned(),
    };
    assert!(layout.apply(&down, &known()));
    assert_eq!(
        layout.order["adjust"],
        ["exposure", "future-tool", "temperature"]
    );
}

/// A utility moved into a tab, or a tool unpinned, goes to the top of the scrolling list
/// after the ids this build does not know that start it, so those stay first.
#[test]
fn the_top_is_after_leading_unknown_ids() {
    let text = "[[tabs]]\nid = \"adjust\"\n[[tabs]]\nid = \"photos\"\n\
                [pinned]\nadjust = [\"scopes\"]\n\
                [order]\nadjust = [\"future-first\", \"exposure\"]\n\
                photos = [\"collections\", \"export\"]\n";
    let mut layout = parsed(text);
    let unpin = Move::Unpin {
        tool: SCOPES,
        tab: "adjust".to_owned(),
    };
    assert!(layout.apply(&unpin, &known()));
    assert_eq!(layout.order["adjust"], ["future-first", SCOPES, "exposure"]);
    let mut layout = parsed(text);
    assert!(layout.apply(&to_tab("collections", "photos", "adjust"), &known()));
    assert_eq!(
        layout.order["adjust"],
        ["future-first", "collections", "exposure"]
    );
}

/// The tabs a move may go to are those shown as `resolve` works them out: a tab listed
/// twice counts once, so a tab after twelve copies of another is still shown.
#[test]
fn a_tab_listed_twice_counts_once_for_a_move() {
    let mut text: String = "[[tabs]]\nid = \"photos\"\n".repeat(12);
    text.push_str(
        "[[tabs]]\nid = \"adjust\"\n\
         [order]\nphotos = [\"collections\", \"export\", \"grain\"]\nadjust = [\"exposure\"]\n",
    );
    let mut layout = parsed(&text);
    let (shown, _) = resolve(&layout, &[&default_layout()], &known());
    assert!(shown.tabs.iter().any(|tab| tab.id == "adjust"));
    assert!(layout.apply(&to_tab("grain", "photos", "adjust"), &known()));
    assert!(layout.order["adjust"].iter().any(|t| t == "grain"));
}

/// A move to a tab replayed on a file a hand edit changed: where the edit put the tool in
/// another tab, the move is skipped, so it never sits in two; where the edit hid it, the
/// move shows it, placed once.
#[test]
fn a_replayed_move_whose_tool_moved_by_hand_is_skipped() {
    let workspace = default_workspace();
    let known = known();
    let settled = workspace.layout.settled(&workspace.shown, &known);
    let change = to_tab("grain", "effects", "photos");
    // Moved by hand to Adjust.
    let mut moved = settled.clone();
    moved
        .order
        .get_mut("effects")
        .unwrap()
        .retain(|t| t != "grain");
    moved
        .order
        .get_mut("adjust")
        .unwrap()
        .push("grain".to_owned());
    assert!(!moved.apply(&change, &known));
    let holding: Vec<&String> = (moved.order.iter())
        .filter(|(_, list)| list.iter().any(|t| t == "grain"))
        .map(|(tab, _)| tab)
        .collect();
    assert_eq!(holding, ["adjust"]);
    // Hidden by hand.
    let mut hidden = settled;
    hidden
        .order
        .get_mut("effects")
        .unwrap()
        .retain(|t| t != "grain");
    hidden.hidden.push("grain".to_owned());
    assert!(hidden.apply(&change, &known));
    assert!(hidden.order["photos"].iter().any(|t| t == "grain"));
    assert!(!hidden.hidden.iter().any(|t| t == "grain"));
}

/// Remove Tool takes a tool out of every tab the workspace lists, one past the twelfth
/// included, and leaves a list for a tab id it lacks as it is (decision 96).
#[test]
fn hide_leaves_the_lists_of_tabs_the_workspace_lacks() {
    let mut text: String = (1..=13)
        .map(|n| format!("[[tabs]]\nid = \"tab-{n}\"\n"))
        .collect();
    text.push_str(
        "[order]\ntab-1 = [\"grain\", \"exposure\"]\ntab-13 = [\"grain\"]\n\
         elsewhere = [\"grain\"]\n",
    );
    let mut layout = parsed(&text);
    assert!(layout.apply(&Move::Hide { tool: "grain" }, &known()));
    assert_eq!(layout.order["tab-1"], ["exposure"]);
    assert!(!layout.order.contains_key("tab-13"));
    assert_eq!(layout.order["elsewhere"], ["grain"]);
    assert!(layout.hidden.iter().any(|t| t == "grain"));
}

/// A write leaves the lists for a tab id the file lacks as they are, in `[pinned]` and
/// `[order]`, an item this build skips included.
#[test]
fn a_write_keeps_the_lists_of_tabs_the_file_lacks_as_written() {
    let folder = Folder::new("workspace-undeclared-lists");
    let file = folder.0.join("portraits.toml");
    let own = "[[tabs]]\nid = \"skin\"\n\
               [pinned]\nskin = [\"scopes\"]\nelsewhere = [\"grain\", 42]\n\
               [order]\nskin = [\"exposure\", \"grain\"]\nelsewhere = [\"bloom\", 7]\n";
    std::fs::write(&file, own).unwrap();
    let hide = [Move::Hide { tool: "grain" }];
    let wrote = files::write(
        &file,
        "portraits",
        false,
        &hide,
        &mut files::Kept::default(),
    );
    assert_eq!(wrote.unwrap(), files::Wrote::Written);
    let table: Table = std::fs::read_to_string(&file).unwrap().parse().unwrap();
    let list = |key: &str, tab: &str| table[key][tab].as_array().cloned().unwrap();
    assert_eq!(
        list("pinned", "elsewhere"),
        [Value::from("grain"), 42.into()]
    );
    assert_eq!(list("order", "elsewhere"), [Value::from("bloom"), 7.into()]);
    assert!(!list("order", "skin").contains(&Value::from("grain")));
}

/// A write that failed is tried again on a switch with no new move, at once, not only at
/// the next move or on exit.
#[test]
fn a_switch_tries_a_failed_write_again() {
    let folder = Folder::new("workspace-switch-retry");
    let file = folder.0.join("default.toml");
    let mut files = Files::new(folder.0.clone());
    let mut workspace = default_workspace();
    files::fail_next_write(&file);
    let hide = Move::Hide { tool: "grain" };
    move_in(&mut files, &mut workspace, hide, Instant::now());
    files.write_now();
    // The failed write is tried, and leaves no file.
    let deadline = Instant::now() + Duration::from_secs(10);
    while files::FAIL.lock().unwrap().contains(&file) {
        assert!(Instant::now() < deadline, "the write was never tried");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!file.exists());
    // A switch, nothing moved: written.
    files.write_now();
    while !file.exists() {
        assert!(Instant::now() < deadline, "the write was never tried again");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        written(&folder, DEFAULT)
            .hidden
            .iter()
            .any(|t| t == "grain")
    );
}

/// A file a person puts in the place of one set aside, even one this build never writes,
/// ends the set-aside: deleted after, a person's own workspace is gone, never written again.
#[test]
fn a_file_put_in_the_place_of_one_set_aside_ends_it() {
    let folder = Folder::new("workspace-set-aside-replaced");
    let file = folder.0.join("portraits.toml");
    let own = "name = \"Portraits\"\n[[tabs]]\nid = \"skin\"\n[order]\nskin = [\"exposure\"]\n";
    std::fs::write(&file, own).unwrap();
    let mut kept = files::Kept::default();
    let hide = [Move::Hide { tool: "grain" }];
    let wrote = files::write(&file, "portraits", false, &hide, &mut kept);
    assert_eq!(wrote.unwrap(), files::Wrote::Written);
    std::fs::write(&file, "[[").unwrap();
    files::fail_next_write(&file);
    assert!(files::write(&file, "portraits", false, &hide, &mut kept).is_err());
    assert!(kept.set_aside);
    let newer = "version = 2\n[[tabs]]\nid = \"skin\"\n";
    std::fs::write(&file, newer).unwrap();
    let wrote = files::write(&file, "portraits", false, &hide, &mut kept);
    assert_eq!(wrote.unwrap(), files::Wrote::Newer);
    assert!(!kept.set_aside);
    std::fs::remove_file(&file).unwrap();
    let wrote = files::write(&file, "portraits", false, &hide, &mut kept);
    assert_eq!(wrote.unwrap(), files::Wrote::Left);
    assert!(!file.exists());
}

/// Replayed on a file whose Effects tab a hand edit took away, keeping its lists: a move
/// within Effects or pinning there does nothing, and those lists are never changed; a move
/// of a tool only that list holds still reaches a tab shown, the list left as it is.
#[test]
fn a_tab_the_file_no_longer_lists_is_no_move_s_source() {
    let workspace = default_workspace();
    let known = known();
    let mut layout = workspace.layout.settled(&workspace.shown, &known);
    layout.tabs.retain(|tab| tab.id != "effects");
    let effects = layout.order["effects"].clone();
    let tab = || "effects".to_owned();
    for change in [
        Move::Up {
            tool: "grain",
            tab: tab(),
        },
        Move::Down {
            tool: "vignette",
            tab: tab(),
        },
        Move::Pin {
            tool: "grain",
            tab: tab(),
        },
        Move::Unpin {
            tool: SCOPES,
            tab: tab(),
        },
    ] {
        assert!(!layout.apply(&change, &known), "{change:?}");
    }
    assert!(layout.apply(&to_tab("grain", "effects", "photos"), &known));
    assert_eq!(layout.order["effects"], effects);
    assert!(layout.order["photos"].iter().any(|t| t == "grain"));
}

/// Pin Tool and Unpin Tool take away the list the tool left only where the move emptied
/// it: an empty list of a tab the workspace lacks stays.
#[test]
fn pinning_leaves_other_empty_lists() {
    let mut layout = parsed(
        "[[tabs]]\nid = \"adjust\"\n\
         [order]\nadjust = [\"exposure\"]\nelsewhere = []\n",
    );
    let pin = Move::Pin {
        tool: "exposure",
        tab: "adjust".to_owned(),
    };
    assert!(layout.apply(&pin, &known()));
    assert!(!layout.order.contains_key("adjust"));
    assert_eq!(layout.order["elsewhere"], Vec::<String>::new());
    let unpin = Move::Unpin {
        tool: "exposure",
        tab: "adjust".to_owned(),
    };
    assert!(layout.apply(&unpin, &known()));
    assert!(!layout.pinned.contains_key("adjust"));
    assert_eq!(layout.order["elsewhere"], Vec::<String>::new());
}

/// The writer tells the app of a newer build's file and wakes it, so a window at rest
/// takes it in at once: the delivery, with no wait.
#[test]
fn the_writer_wakes_the_app_with_a_newer_file() {
    let folder = Folder::new("workspace-wake");
    let (woken, wake) = std::sync::mpsc::channel();
    let mut files = Files::waking(folder.0.clone(), move || _ = woken.send(()));
    let newer = "version = 2\n[[tabs]]\nid = \"photos\"\n";
    std::fs::write(folder.0.join("default.toml"), newer).unwrap();
    let mut workspace = default_workspace();
    move_in(
        &mut files,
        &mut workspace,
        Move::Hide { tool: "grain" },
        Instant::now(),
    );
    files.write_now();
    wake.recv_timeout(Duration::from_secs(10)).expect("woken");
    assert_eq!(files.newer(), [DEFAULT]);
    assert!(files.newer().is_empty());
}

/// A processing tool lands after the one before it in the pipeline and the ids this build
/// does not know that follow that one, which stay with it; still never after Export.
#[test]
fn a_tool_lands_after_its_neighbour_s_unknown_ids() {
    let ids = |ids: &[&str]| ids.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>();
    let known = known();
    let list = ids(&["demosaic", "future-tool"]);
    assert_eq!(landing(&list, "vignette", &known), 2);
    let list = ids(&["demosaic", "future-tool", EXPORT]);
    assert_eq!(landing(&list, "vignette", &known), 2);
    let list = ids(&["demosaic", EXPORT, "future-tool"]);
    assert_eq!(landing(&list, "vignette", &known), 1);
}

/// Once a write finds a workspace's file a newer build's, the writer writes it no more
/// this run, a save asked before the app knew included, even where a file this build
/// writes is put back meanwhile; another workspace's still saves.
#[test]
fn a_newer_file_is_never_written_again_this_run() {
    let folder = Folder::new("workspace-newer-dropped");
    let file = folder.0.join("default.toml");
    let newer = "version = 2\n[[tabs]]\nid = \"photos\"\n";
    std::fs::write(&file, newer).unwrap();
    let mut files = Files::new(folder.0.clone());
    let mut workspace = default_workspace();
    move_in(
        &mut files,
        &mut workspace,
        Move::Hide { tool: "grain" },
        Instant::now(),
    );
    files.write_now();
    files.wait();
    assert_eq!(files.newer(), [DEFAULT]);
    let restored = "version = 1\n[[tabs]]\nid = \"effects\"\n[order]\neffects = [\"grain\"]\n";
    std::fs::write(&file, restored).unwrap();
    move_in(
        &mut files,
        &mut workspace,
        Move::Hide { tool: "bloom" },
        Instant::now(),
    );
    let mut essentials = Workspaces::builtin().open(ESSENTIALS).0;
    let show = Move::ToTab {
        tool: "grain",
        from: None,
        to: "adjust".to_owned(),
    };
    move_in(&mut files, &mut essentials, show, Instant::now());
    files.write_now();
    files.wait();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), restored);
    let adjust = &written(&folder, ESSENTIALS).order["adjust"];
    assert!(adjust.iter().any(|t| t == "grain"));
}

/// Taking a tool out removes only the lists it empties: a list already empty in the file
/// stays as it is.
#[test]
fn taking_a_tool_out_leaves_lists_empty_before() {
    let mut text: String = (1..=13)
        .map(|n| format!("[[tabs]]\nid = \"tab-{n}\"\n"))
        .collect();
    text.push_str("[order]\ntab-1 = [\"grain\"]\ntab-13 = []\n");
    let mut layout = parsed(&text);
    assert!(layout.apply(&Move::Hide { tool: "grain" }, &known()));
    assert!(!layout.order.contains_key("tab-1"));
    assert_eq!(layout.order["tab-13"], Vec::<String>::new());
}

/// A person's own workspace set aside as damaged, with nothing written of it this run and
/// no defaults to write instead, keeps its moves: a valid file put back takes them.
#[test]
fn a_damaged_own_workspace_keeps_its_moves_for_a_file_put_back() {
    let folder = Folder::new("workspace-damaged-own");
    let file = folder.0.join("portraits.toml");
    std::fs::write(&file, "[[").unwrap();
    let mut kept = files::Kept::default();
    let hide = [Move::Hide { tool: "grain" }];
    assert!(files::write(&file, "portraits", false, &hide, &mut kept).is_err());
    assert!(kept.set_aside && !file.exists());
    // Still nothing to write: kept again.
    assert!(files::write(&file, "portraits", false, &hide, &mut kept).is_err());
    let own = "name = \"Portraits\"\n[[tabs]]\nid = \"skin\"\n[order]\nskin = [\"exposure\"]\n";
    std::fs::write(&file, own).unwrap();
    let wrote = files::write(&file, "portraits", false, &hide, &mut kept);
    assert_eq!(wrote.unwrap(), files::Wrote::Written);
    let layout = written(&folder, "portraits");
    assert!(layout.hidden.iter().any(|t| t == "grain"));
    assert_eq!(layout.name.as_deref(), Some("Portraits"));
}
