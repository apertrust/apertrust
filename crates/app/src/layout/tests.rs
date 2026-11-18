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

/// The bundled Default workspace is decision 38's: its tabs in order, each listing the
/// built tools decision 38 gives it in its order and no other, Scopes pinned over the five
/// editing tabs, each tool placed in one tab only; a whole file in the model's format,
/// with no `seen`, nothing hidden, and its name, which the interface takes from Fluent.
#[test]
fn the_default_workspace_is_decision_38_s() {
    let layout = default_layout();
    assert_eq!(layout.version, VERSION);
    assert_eq!(layout.name.as_deref(), Some("Default"));
    assert!(layout.seen.is_empty());
    assert!(layout.hidden.is_empty());
    let ids: Vec<&str> = layout.tabs.iter().map(|tab| tab.id.as_str()).collect();
    assert_eq!(ids, DECISION_38.map(|(id, _)| id));
    let known = known();
    for (id, recorded) in DECISION_38 {
        let expected: Vec<&str> = recorded
            .iter()
            .copied()
            .filter(|t| known.contains(t))
            .collect();
        let order: Vec<&str> = layout.order[id].iter().map(String::as_str).collect();
        assert_eq!(order, expected, "{id}");
        let pinned = layout.pinned.get(id).cloned().unwrap_or_default();
        let scopes = SCOPES_PINNED
            .contains(&id)
            .then_some(vec![SCOPES.to_owned()]);
        assert_eq!(pinned, scopes.unwrap_or_default(), "{id}");
    }
    let mut placed: Vec<&str> = DECISION_38
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

/// Every tool and icon the bundled file names exists, every tool of the Tools panel is in
/// one of its tabs, Scopes in each it pins it in and every other once, each tab's title is
/// a message in every language, and the layout it shows holds the invariants, with no
/// warning. Detail, with nothing built, is drawn disabled; the others can be chosen.
#[test]
fn the_default_workspace_names_only_tools_and_icons_that_exist() {
    let layout = default_layout();
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
    let (shown, warnings) = resolve(&layout, &[&layout], &known);
    assert!(warnings.is_empty(), "{warnings:?}");
    holds_the_invariants(&shown, &known);
    assert!(shown.hidden.is_empty(), "{:?}", shown.hidden);
    for tool in &known {
        let tabs = shown.tabs.iter().filter(|tab| tab.holds(tool)).count();
        let expected = if *tool == SCOPES {
            SCOPES_PINNED.len()
        } else {
            1
        };
        assert_eq!(tabs, expected, "{tool} in {tabs} tabs");
    }
    let enabled: Vec<bool> = shown.tabs.iter().map(Tab::enabled).collect();
    assert_eq!(enabled, [true, true, false, true, true, true, true]);
    assert_eq!(tab(&shown, "photos").order.last(), Some(&EXPORT));
    let titles: Vec<&Title> = shown.tabs.iter().map(|tab| &tab.title).collect();
    assert_eq!(titles[1], &Title::Message("tab-adjust".to_owned()));
    assert_eq!(tab(&shown, "geometry").icon, Icon::Correct);
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
         [[tabs]]\nid = \"skin\"\ntitle = \"Skin\"\nicon = \"sparkles\"\n\
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
