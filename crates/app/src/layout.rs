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

//! The Tools panel's layout: workspaces (decisions 83 and 96; `docs/gui.md`,
//! Workspaces), plain data with no egui.
//!
//! - **A workspace's file,** [`Layout`]: its `version`, its `name`, its tabs in order,
//!   each an id with a title and an icon, each tab's pinned tools and the rest by tab id
//!   (`[pinned]`, `[order]`), the tools it hides (`hidden`), every tool the build that
//!   wrote it knew (`seen`), and the Toolbar's item ids in order (`toolbar`, decision 92).
//!   A person's own is `<user data>/workspaces/<id>.toml`.
//! - **Built in:** the Default workspace, `data/workspaces/default.toml`, and Essentials,
//!   `data/workspaces/essentials.toml`, embedded and read as a person's own is
//!   ([`Layout::defaults`]); a person's changed copy of one, in the user data folder, takes
//!   its place.
//! - **The workspaces there are,** [`Workspaces`], read once as the app starts: the
//!   built-in ones, then a person's own, each `<id>.toml` that reads, by name, which View ›
//!   Workspace lists; switching works out the layout shown from them ([`Workspaces::open`]).
//! - **The layout shown,** [`Shown`], worked out from the file and the defaults when the
//!   workspace is read ([`resolve`]), never per frame: each tab's lists, the tools hidden,
//!   a new build's new tools placed by `seen`, the first twelve tabs, Export always in
//!   one of them, and the Toolbar's list, the defaults' where the file has none.
//! - **The Hidden tab,** [`Workspace::hidden_tab`], worked out with the layout shown: the
//!   tools in no tab shown, processing tools in pipeline order, then utilities, never
//!   pinned or reordered there.
//! - **Reading,** [`Workspaces::read`]: a file that is not TOML is set aside as
//!   `<id>.toml.bak`, one listing no tab refused, both with a warning, the built-in's
//!   defaults then applying, or a person's own not listed; a newer version's is read for
//!   what this build knows and never overwritten.
//! - **The moves,** [`Move`] (decision 96, step 4): a tool one place up or down in its
//!   list, pinned or unpinned, moved to another tab, hidden or shown in a tab, display
//!   order only, and the Toolbar's list as it now is (decision 92), applied to the layout
//!   as its file will be written ([`Layout::settled`], [`Layout::apply`],
//!   [`Workspace::apply`]), never per frame.
//! - **Writing** a workspace's file, [`files`]: the moves written whole through the safe
//!   write once the layout has stood still for a second, at once on a switch and on exit,
//!   each write applying them to the file as it is on disk then.

use std::collections::{BTreeMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

use apertrust_ui::Icon;
use toml::{Table, Value};

pub mod files;

pub use files::Files;

/// The file format's version: what this build reads, and the newest it reads.
pub const VERSION: i64 = 1;

/// How many of a file's tabs show, chosen by `panel-tab-1` to `panel-tab-12`; a tab past
/// them is not shown and counts as absent.
pub const MOST_TABS: usize = 12;

/// The Hidden tab's id, reserved: no file's tab takes it.
pub const HIDDEN: &str = "hidden";

/// The Hidden tab's icon, Lucide's `eye-off`, its filled form on the chosen tab.
pub const HIDDEN_ICON: Icon = Icon::EyeOff;

/// The Default workspace's id, built in and opened where settings name none or one with no
/// workspace.
pub const DEFAULT: &str = "default";

/// The Essentials workspace's id, built in: the tools most edits need, every other hidden
/// (decision 96).
pub const ESSENTIALS: &str = "essentials";

/// The built-in workspaces, each by its id with its file's text, embedded from the
/// repository's `data/workspaces/` (decision 45), in the order View › Workspace lists them.
const BUILT_IN: &[(&str, &str)] = &[
    (
        DEFAULT,
        include_str!("../../../data/workspaces/default.toml"),
    ),
    (
        ESSENTIALS,
        include_str!("../../../data/workspaces/essentials.toml"),
    ),
];

/// The built-in workspaces' ids, in the order View › Workspace lists them.
pub fn builtin_ids() -> impl Iterator<Item = &'static str> {
    BUILT_IN.iter().map(|(id, _)| *id)
}

/// The tools that stay in every workspace: Export is never hidden (decision 90).
const EXPORT: &str = "export";

/// Scopes, which measures the photo for the tools beside it: a tab holding it alone has
/// nothing to work with yet, as Detail until its first tool (decision 38).
const SCOPES: &str = "scopes";

/// The icon of a tab of a person's own that names none, or one this build lacks.
pub const OWN_TAB_ICON: Icon = Icon::Grid;

/// One tab as a workspace's file lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabEntry {
    pub id: String,
    /// Its title as written, the same in every language; none for a tab a build knows,
    /// whose title is its message.
    pub title: Option<String>,
    /// One of `apertrust-ui`'s icons by name, as written.
    pub icon: Option<String>,
}

/// A workspace as its file holds it, what this build reads of it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Layout {
    pub version: i64,
    /// A person's own workspace's name; a built-in's is its message, `workspace-<id>`.
    pub name: Option<String>,
    /// Its tabs, in order.
    pub tabs: Vec<TabEntry>,
    /// Each tab's pinned tools and the rest, by tab id, top first.
    pub pinned: BTreeMap<String, Vec<String>>,
    pub order: BTreeMap<String, Vec<String>>,
    /// The tools it hides.
    pub hidden: Vec<String>,
    /// Every tool the build that wrote it knew; empty in a bundled file, whose lists are
    /// its defaults.
    pub seen: Vec<String>,
    /// The Toolbar's item ids in order (decision 92), those this build does not know
    /// included; none where the file has no `toolbar` list, the defaults' then drawn.
    pub toolbar: Option<Vec<String>>,
}

/// Why a file cannot be a workspace.
#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    /// It is not UTF-8, or not TOML: set aside.
    Damaged(String),
    /// It lists no tab: left as it is.
    NoTab,
}

impl Layout {
    /// The workspace `text` holds, with what could not be read of it, each part skipped:
    /// a newer version's is read for what this build knows. Refused where it is not TOML or
    /// lists no tab.
    pub fn parse(text: &str) -> Result<(Layout, Vec<String>), Refused> {
        Layout::from_table(&table_of(text).map_err(Refused::Damaged)?)
    }

    /// The workspace `table` holds, as [`Layout::parse`] reads it; refused where it lists
    /// no tab.
    pub fn from_table(table: &Table) -> Result<(Layout, Vec<String>), Refused> {
        let mut warnings = Vec::new();
        let version = match table.get("version") {
            None => VERSION,
            Some(value) => value.as_integer().unwrap_or_else(|| {
                warnings.push("version is no number; read as 1".to_owned());
                VERSION
            }),
        };
        let name = table.get("name").and_then(|name| {
            let text = name.as_str().map(str::to_owned);
            if text.is_none() {
                warnings.push("name is no text; its id shown".to_owned());
            }
            text
        });
        let mut tabs = Vec::new();
        let listed = table.get("tabs").map(|tabs| match tabs.as_array() {
            Some(tabs) => tabs.as_slice(),
            None => {
                warnings.push("tabs is no list of tables".to_owned());
                &[]
            }
        });
        for (i, tab) in listed.into_iter().flatten().enumerate() {
            let Some(id) = tab.get("id").and_then(Value::as_str) else {
                warnings.push(format!("tabs[{i}] has no id; not shown"));
                continue;
            };
            let text = |key: &str, warnings: &mut Vec<String>| {
                let value = tab.get(key)?;
                let text = value.as_str().map(str::to_owned);
                if text.is_none() {
                    warnings.push(format!("tabs[{i}].{key} is no text; not read"));
                }
                text
            };
            let title = text("title", &mut warnings);
            let icon = text("icon", &mut warnings);
            tabs.push(TabEntry {
                id: id.to_owned(),
                title,
                icon,
            });
        }
        if !tabs.iter().any(|tab| tab.id != HIDDEN) {
            return Err(Refused::NoTab);
        }
        let mut lists = |key: &str| {
            let mut lists = BTreeMap::new();
            let Some(value) = table.get(key) else {
                return lists;
            };
            let Some(by_tab) = value.as_table() else {
                warnings.push(format!("{key} is no table; not read"));
                return lists;
            };
            for (tab, list) in by_tab {
                lists.insert(
                    tab.clone(),
                    ids(list, &format!("{key}.{tab}"), &mut warnings),
                );
            }
            lists
        };
        let (pinned, order) = (lists("pinned"), lists("order"));
        let mut list = |key: &str| match table.get(key) {
            Some(value) => ids(value, key, &mut warnings),
            None => Vec::new(),
        };
        let (hidden, seen) = (list("hidden"), list("seen"));
        let toolbar = table.get("toolbar").and_then(|value| {
            let listed = value.as_array().is_some();
            if !listed {
                warnings.push("toolbar is no list; the default drawn".to_owned());
            }
            listed.then(|| ids(value, "toolbar", &mut warnings))
        });
        let layout = Layout {
            version,
            name,
            tabs,
            pinned,
            order,
            hidden,
            seen,
            toolbar,
        };
        Ok((layout, warnings))
    }

    /// The built-in workspace `id`'s defaults, its bundled file, or None where no
    /// workspace of that id is built in. Each file is read once, the first time it is asked
    /// for, as a move asks for it again.
    pub fn defaults(id: &str) -> Option<Layout> {
        static READ: std::sync::OnceLock<Vec<(&str, Layout)>> = std::sync::OnceLock::new();
        let read = READ.get_or_init(|| {
            let read = BUILT_IN.iter().map(|(built, text)| {
                let (layout, _) = Layout::parse(text).expect("a bundled workspace reads");
                (*built, layout)
            });
            read.collect()
        });
        let (_, layout) = read.iter().find(|(built, _)| *built == id)?;
        Some(layout.clone())
    }
}

/// The table `text` holds, or why it is not TOML.
fn table_of(text: &str) -> Result<Table, String> {
    // What the error says, never `Display`'s quote of the line.
    text.parse()
        .map_err(|e: toml::de::Error| e.message().replace('\n', " "))
}

/// The tool ids `value` lists, in order, each a text; `key` names it in a warning.
fn ids(value: &Value, key: &str, warnings: &mut Vec<String>) -> Vec<String> {
    let Some(items) = value.as_array() else {
        warnings.push(format!("{key} is no list; not read"));
        return Vec::new();
    };
    let ids: Vec<String> = items
        .iter()
        .filter_map(|item| item.as_str().map(str::to_owned))
        .collect();
    if ids.len() < items.len() {
        warnings.push(format!("{key}: an item that is no text skipped"));
    }
    ids
}

/// The words a tab, or a workspace's name, shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Title {
    /// A tab a build knows, or a built-in workspace: its message, `tab-<id>` or
    /// `workspace-<id>`, in the interface's language.
    Message(String),
    /// Written in the file, or its id where nothing is: the same in every language.
    Own(String),
}

/// One tab shown, with its tools.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tab {
    pub id: String,
    pub title: Title,
    pub icon: Icon,
    /// Its pinned tools, top first, then the rest, top first: display order only.
    pub pinned: Vec<&'static str>,
    pub order: Vec<&'static str>,
}

impl Tab {
    /// Its tools, pinned first.
    pub fn tools(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.pinned.iter().chain(&self.order).copied()
    }

    /// Whether it holds `tool`.
    pub fn holds(&self, tool: &str) -> bool {
        self.tools().any(|t| t == tool)
    }

    /// Whether it can be chosen: it holds a tool to work with, not Scopes alone, as
    /// decision 38's Detail is drawn disabled until its first tool.
    pub fn enabled(&self) -> bool {
        self.tools().any(|tool| tool != SCOPES)
    }
}

/// The layout shown (decision 96, The layout shown): the tabs shown with their tools, and
/// the tools in none of them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Shown {
    /// At most [`MOST_TABS`].
    pub tabs: Vec<Tab>,
    /// Every tool in no tab shown, the Hidden tab's: processing tools in pipeline order,
    /// first at the top, as the Default workspace's Raw lists them, then utilities in the
    /// order the build lists them ([`hidden_order`]).
    pub hidden: Vec<&'static str>,
    /// Those of `hidden` that only a tab past the twelfth holds, never written into the
    /// file's `hidden`.
    pub beyond: Vec<&'static str>,
    /// The Toolbar's item ids in order (decision 92): the file's, else its defaults',
    /// those this build does not know kept, for the app to skip as it draws them.
    pub toolbar: Vec<String>,
}

/// The tools of the Tools panel this build has, processing tools then utilities: every
/// tool a workspace may place. No import tool is one (decision 86), so a file naming one
/// skips it, as any id this build does not know.
pub fn known() -> Vec<&'static str> {
    let tools = apertrust_tools::TOOLS.iter();
    let processing = tools.map(|tool| tool.definition().tool.id);
    let utilities = apertrust_tools::utility::UTILITIES.iter();
    processing.chain(utilities.map(|u| u.tool.id)).collect()
}

/// Whether `tool` is a processing tool, which edits the photo.
pub fn processing(tool: &str) -> bool {
    let mut tools = apertrust_tools::TOOLS.iter();
    tools.any(|t| t.definition().tool.id == tool)
}

/// Where `tool` sorts in the Hidden tab: a processing tool by the place in the pipeline of
/// the first of its operations to run, among [`apertrust_operations::OPERATIONS`], before
/// every utility, which keeps the order the build lists it in (the sort being stable).
pub fn hidden_order(tool: &str) -> (bool, usize) {
    let Some(processing) = apertrust_tools::TOOLS
        .iter()
        .find(|t| t.definition().tool.id == tool)
    else {
        return (true, 0);
    };
    let operations = apertrust_operations::OPERATIONS.iter();
    let places = operations.map(|op| op.manifest().id).enumerate();
    let place = places
        .filter(|(_, id)| processing.owns().iter().any(|owned| owned.operation == *id))
        .map(|(place, _)| place)
        .min();
    (false, place.unwrap_or(usize::MAX))
}

/// The tab ids a build knows, each with its icon by name: the built-in workspaces' tabs.
fn known_tabs() -> Vec<(String, Option<String>)> {
    let builtins = BUILT_IN.iter().filter_map(|(id, _)| Layout::defaults(id));
    let mut tabs: Vec<(String, Option<String>)> = Vec::new();
    for tab in builtins.flat_map(|layout| layout.tabs) {
        if !tabs.iter().any(|(id, _)| *id == tab.id) {
            tabs.push((tab.id, tab.icon));
        }
    }
    tabs
}

/// The icon `name` names, if `apertrust-ui` has one by that name.
pub fn icon(name: &str) -> Option<Icon> {
    Icon::ALL.into_iter().find(|icon| icon.name() == name)
}

/// The layout `layout` shows, the tools `known` lists the only ones placed, with what could
/// not be shown (decision 96's steps):
///
/// 1. each of the first twelve tabs takes its `pinned` and `order` lists, an id in both
///    pinned, one twice in a list at its first place, an id not `known` skipped;
/// 2. a tool `hidden` names is in no tab, but Export;
/// 3. a new tool, one neither `seen`, `hidden` nor any tab's lists name, goes into each tab
///    shown whose id is one of its default tabs, after its nearest preceding neighbour in
///    that tab's default list, pinned where the default pins it; its default tabs are those
///    of the first of `defaults` with a tab of that id; with none, it is hidden;
/// 4. a tool in no tab is hidden, `seen` naming it or not, the Hidden tab listing it in
///    pipeline order ([`Shown::hidden`]);
/// 5. Export in no tab shown goes to the end of the Photos tab where it is shown, else of
///    the first tab shown that can be chosen, else of the first tab.
///
/// The Toolbar's list is the file's, else that of the first of `defaults` with one.
pub fn resolve(
    layout: &Layout,
    defaults: &[&Layout],
    known: &[&'static str],
) -> (Shown, Vec<String>) {
    let mut warnings = Vec::new();
    let known_id = |id: &str| known.iter().copied().find(|k| *k == id);
    // The file's tabs, each once, but the Hidden tab's reserved id.
    let mut listed: Vec<&TabEntry> = Vec::new();
    for tab in &layout.tabs {
        if tab.id == HIDDEN {
            warnings.push(format!("tab {HIDDEN}: the Hidden tab's id; not shown"));
        } else if listed.iter().any(|t| t.id == tab.id) {
            warnings.push(format!("tab {} listed twice; the first shown", tab.id));
        } else {
            listed.push(tab);
        }
    }
    for tab in listed.iter().skip(MOST_TABS) {
        warnings.push(format!(
            "tab {}: past the {MOST_TABS}th; not shown, kept",
            tab.id
        ));
    }
    let shown_tabs = &listed[..listed.len().min(MOST_TABS)];
    let list = |lists: &BTreeMap<String, Vec<String>>, tab: &str| {
        let mut ids: Vec<&'static str> = Vec::new();
        for id in lists.get(tab).into_iter().flatten() {
            if let Some(id) = known_id(id)
                && !ids.contains(&id)
            {
                ids.push(id);
            }
        }
        ids
    };
    // 1. Each tab's lists; 2. without the tools hidden.
    let hidden: Vec<&'static str> = layout
        .hidden
        .iter()
        .filter_map(|id| known_id(id))
        .filter(|id| *id != EXPORT)
        .collect();
    let (known_tabs, mut tabs) = (known_tabs(), Vec::new());
    for entry in shown_tabs {
        let mut pinned = list(&layout.pinned, &entry.id);
        let mut order = list(&layout.order, &entry.id);
        order.retain(|id| !pinned.contains(id));
        pinned.retain(|id| !hidden.contains(id));
        order.retain(|id| !hidden.contains(id));
        let built = known_tabs.iter().find(|(id, _)| *id == entry.id);
        let title = match (&entry.title, built) {
            (Some(title), _) => Title::Own(title.clone()),
            (None, Some((id, _))) => Title::Message(format!("tab-{id}")),
            (None, None) => Title::Own(entry.id.clone()),
        };
        let written = entry.icon.as_deref().and_then(icon);
        if let Some(name) = &entry.icon
            && written.is_none()
        {
            warnings.push(format!("tab {}: no icon {name}", entry.id));
        }
        let default_icon = built.and_then(|(_, name)| name.as_deref().and_then(icon));
        tabs.push(Tab {
            id: entry.id.clone(),
            title,
            icon: written.or(default_icon).unwrap_or(OWN_TAB_ICON),
            pinned,
            order,
        });
    }
    // 3. The new tools, placed by their default tabs.
    let mut named: HashSet<&str> = layout.seen.iter().map(String::as_str).collect();
    named.extend(layout.hidden.iter().map(String::as_str));
    for tab in &listed {
        for lists in [&layout.pinned, &layout.order] {
            named.extend(lists.get(&tab.id).into_iter().flatten().map(String::as_str));
        }
    }
    let new: Vec<&'static str> = known
        .iter()
        .copied()
        .filter(|id| !named.contains(id))
        .collect();
    for tab in &mut tabs {
        let id = tab.id.as_str();
        let Some(default) = defaults.iter().find(|d| d.tabs.iter().any(|t| t.id == id)) else {
            continue;
        };
        for pin in [true, false] {
            let defaults = if pin { &default.pinned } else { &default.order };
            let Some(wanted) = defaults.get(id) else {
                continue;
            };
            for (at, tool) in wanted.iter().enumerate() {
                let Some(tool) = new.iter().copied().find(|t| t == tool) else {
                    continue;
                };
                if tab.holds(tool) {
                    continue;
                }
                let list = if pin { &mut tab.pinned } else { &mut tab.order };
                let after = wanted[..at]
                    .iter()
                    .rev()
                    .find_map(|neighbour| list.iter().position(|t| t == neighbour));
                list.insert(after.map_or(0, |i| i + 1), tool);
            }
        }
    }
    // 5. Export in a tab shown.
    if let Some(export) = known_id(EXPORT)
        && !tabs.iter().any(|tab| tab.holds(export))
        && !tabs.is_empty()
    {
        let at = (tabs.iter().position(|tab| tab.id == "photos"))
            .or_else(|| tabs.iter().position(Tab::enabled))
            .unwrap_or(0);
        tabs[at].order.push(export);
    }
    // 4. Every tool in no tab shown, hidden.
    let unshown = |id: &&'static str| !tabs.iter().any(|tab| tab.holds(id));
    let mut hidden: Vec<&'static str> = known.iter().copied().filter(unshown).collect();
    hidden.sort_by_key(|tool| hidden_order(tool));
    let past: Vec<&TabEntry> = listed.iter().skip(MOST_TABS).copied().collect();
    let beyond = hidden
        .iter()
        .copied()
        .filter(|tool| {
            let held = |lists: &BTreeMap<String, Vec<String>>, tab: &TabEntry| {
                lists
                    .get(&tab.id)
                    .is_some_and(|l| l.iter().any(|t| t == tool))
            };
            let only_past = past
                .iter()
                .any(|tab| held(&layout.pinned, tab) || held(&layout.order, tab));
            only_past && !layout.hidden.iter().any(|t| t == tool)
        })
        .collect();
    let toolbar = (layout.toolbar.iter())
        .chain(defaults.iter().filter_map(|d| d.toolbar.as_ref()))
        .next()
        .cloned()
        .unwrap_or_default();
    let shown = Shown {
        tabs,
        hidden,
        beyond,
        toolbar,
    };
    (shown, warnings)
}

/// A change a person makes to a workspace's layout (decisions 83 and 96): display order
/// only, never the edit, and never a tab added or taken away. It names its tool and tabs by
/// id, so a write applies it to the file as it is on disk then, a hand edit made meanwhile
/// kept ([`files`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Move {
    /// One place up in its list in the tab `tab`, pinned or not: nothing at the top.
    Up { tool: &'static str, tab: String },
    /// One place down in its list: nothing at the end.
    Down { tool: &'static str, tab: String },
    /// To the end of the tab's pinned list.
    Pin { tool: &'static str, tab: String },
    /// To the top of the tab's scrolling list, after the ids this build does not know that
    /// start it ([`top`]).
    Unpin { tool: &'static str, tab: String },
    /// Out of the tab `from`, or out of `hidden` where none, into the tab `to`, unpinned, as
    /// a drop on its icon puts it ([`landing`]); nothing where `to` holds it already.
    ToTab {
        tool: &'static str,
        from: Option<String>,
        to: String,
    },
    /// Out of every tab into `hidden`, its edit staying and still processed; never Export.
    Hide { tool: &'static str },
    /// The Toolbar's list as it now is (decision 92), the whole list, ids this build does
    /// not know in their places, written over the file's `toolbar`; its tabs and every
    /// other key stay as the file has them.
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "the toolbar menu's Hide and Show Menu Button make it once built"
        )
    )]
    Toolbar(Vec<String>),
}

impl Move {
    /// The tool it moves; none for the Toolbar's list.
    pub fn tool(&self) -> Option<&'static str> {
        match self {
            Move::Up { tool, .. }
            | Move::Down { tool, .. }
            | Move::Pin { tool, .. }
            | Move::Unpin { tool, .. }
            | Move::ToTab { tool, .. }
            | Move::Hide { tool } => Some(tool),
            Move::Toolbar(_) => None,
        }
    }
}

/// `now`'s tools as a list of the file, the ids of `file` not `known` each kept after the
/// tool it followed there, or first where it followed none that `now` holds.
fn keeping(file: &[String], now: &[&'static str], known: &[&'static str]) -> Vec<String> {
    let mut after: Vec<(Option<&str>, &str)> = Vec::new();
    let mut last = None;
    for id in file {
        if !known.contains(&id.as_str()) {
            after.push((last, id));
        } else if now.contains(&id.as_str()) {
            last = Some(id.as_str());
        }
    }
    let kept = |anchor: Option<&str>| -> Vec<String> {
        let ids = after.iter().filter(|(a, _)| *a == anchor);
        ids.map(|(_, id)| (*id).to_owned()).collect()
    };
    let mut list = kept(None);
    for tool in now {
        list.push((*tool).to_owned());
        list.extend(kept(Some(tool)));
    }
    list
}

/// The top of `list` for a tool put there: after the ids not `known` it starts with, which
/// stay first ([`keeping`]), before its first tool `known`.
fn top(list: &[String], known: &[&'static str]) -> usize {
    let unknown = list.iter().take_while(|id| !known.contains(&id.as_str()));
    unknown.count()
}

/// Where `tool` lands in the scrolling list `list` of a tab, as a drop on the tab's icon
/// puts it (decision 83): a processing tool after the processing tool there nearest before
/// it in the pipeline and the ids not `known` that follow that one, else before the one
/// nearest after it, else last; a utility at the top ([`top`]). Never after Export, the
/// Photos tab's last (decision 90). Only the tools `known` count.
fn landing(list: &[String], tool: &str, known: &[&'static str]) -> usize {
    let (utility, place) = hidden_order(tool);
    if utility {
        return top(list, known);
    }
    let places = list.iter().enumerate().filter_map(|(i, id)| {
        let (utility, place) = hidden_order(id);
        (known.contains(&id.as_str()) && !utility).then_some((i, place))
    });
    let before = (places.clone().filter(|(_, p)| *p < place)).max_by_key(|(_, p)| *p);
    let after = places.filter(|(_, p)| *p > place).min_by_key(|(_, p)| *p);
    // After the tool before it and the ids this build does not know that follow it, which
    // stay with it ([`keeping`]).
    let past_unknown = |i: usize| {
        let unknown = list[i + 1..]
            .iter()
            .take_while(|id| !known.contains(&id.as_str()));
        i + 1 + unknown.count()
    };
    let at = match (before, after) {
        (Some((i, _)), _) => past_unknown(i),
        (None, Some((i, _))) => i,
        (None, None) => list.len(),
    };
    let export = list.iter().position(|id| id == EXPORT);
    export.map_or(at, |export| at.min(export))
}

impl Layout {
    /// This layout with `shown`, the layout it shows, written into it, as a write keeps it
    /// (decision 96): each tab shown's lists as `shown` has them, an id not `known` kept
    /// after the tool it followed ([`keeping`]); the tools hidden in `hidden`, but those
    /// only a tab past the twelfth holds, an id not `known` kept there; and every tool
    /// `known` in `seen`, so none is new to it after. Tabs past the twelfth, lists for a tab
    /// id it lacks and its `version` stay as they are, so a newer build's file is still
    /// known as one ([`Workspace::newer`]); a write sets the version ([`Layout::write_into`]).
    pub fn settled(&self, shown: &Shown, known: &[&'static str]) -> Layout {
        let mut layout = self.clone();
        let settle = |lists: &mut BTreeMap<String, Vec<String>>, tab: &Tab, pinned: bool| {
            let file = lists.get(&tab.id).map_or(&[][..], Vec::as_slice);
            let now = if pinned { &tab.pinned } else { &tab.order };
            let list = keeping(file, now, known);
            if list.is_empty() {
                lists.remove(&tab.id);
            } else {
                lists.insert(tab.id.clone(), list);
            }
        };
        for tab in &shown.tabs {
            settle(&mut layout.pinned, tab, true);
            settle(&mut layout.order, tab, false);
        }
        let hides = |id: &str| shown.hidden.contains(&id) && !shown.beyond.contains(&id);
        let mut hidden: Vec<String> = Vec::new();
        let kept = self.hidden.iter().map(String::as_str);
        let now = shown.hidden.iter().copied().filter(|id| hides(id));
        for id in kept
            .filter(|id| !known.contains(id) || hides(id))
            .chain(now)
        {
            if !hidden.iter().any(|h| h == id) {
                hidden.push(id.to_owned());
            }
        }
        layout.hidden = hidden;
        for tool in known {
            if !layout.seen.iter().any(|seen| seen == tool) {
                layout.seen.push((*tool).to_owned());
            }
        }
        layout
    }

    /// The ids of the tabs it shows, as [`resolve`] works them out: each once, the first
    /// place kept, never the Hidden tab's, the first twelve.
    fn shown_tab_ids(&self) -> Vec<&str> {
        let mut ids: Vec<&str> = Vec::new();
        for tab in self.tabs.iter().filter(|tab| tab.id != HIDDEN) {
            if !ids.contains(&tab.id.as_str()) {
                ids.push(&tab.id);
            }
        }
        ids.truncate(MOST_TABS);
        ids
    }

    /// The tab `tab`'s list holding `tool`, pinned (true) or not, and its place there.
    fn find(&self, tab: &str, tool: &str) -> Option<(bool, usize)> {
        let at = |lists: &BTreeMap<String, Vec<String>>| {
            lists.get(tab)?.iter().position(|id| id == tool)
        };
        at(&self.pinned)
            .map(|i| (true, i))
            .or_else(|| at(&self.order).map(|i| (false, i)))
    }

    /// The tab `tab`'s pinned list, or its scrolling list, made where it has none.
    fn list(&mut self, pinned: bool, tab: &str) -> &mut Vec<String> {
        let lists = if pinned {
            &mut self.pinned
        } else {
            &mut self.order
        };
        lists.entry(tab.to_owned()).or_default()
    }

    /// Takes `tool` out of the tab `tab`'s lists, or where none out of every tab's the
    /// workspace lists, those past the twelfth included; a list for a tab id it lacks stays
    /// as it is (decision 96).
    fn take_out(&mut self, tab: Option<&str>, tool: &str) -> bool {
        let declared: Vec<String> = self.tabs.iter().map(|tab| tab.id.clone()).collect();
        let touches = |id: &str| match tab {
            Some(tab) => tab == id,
            None => declared.iter().any(|declared| declared == id),
        };
        let mut taken = false;
        for lists in [&mut self.pinned, &mut self.order] {
            // The lists this emptied go; one empty before stays as the file has it.
            let mut emptied = Vec::new();
            for (id, list) in lists.iter_mut().filter(|(id, _)| touches(id)) {
                let before = list.len();
                list.retain(|id| id != tool);
                if list.len() != before {
                    taken = true;
                    if list.is_empty() {
                        emptied.push(id.clone());
                    }
                }
            }
            for id in emptied {
                lists.remove(&id);
            }
        }
        taken
    }

    /// Applies `change` to this layout, settled ([`Layout::settled`]), the tools `known`
    /// those whose places count: whether anything moved. A move at an end, a pin of a
    /// pinned tool, a move to a tab that holds the tool already and Export hidden change
    /// nothing.
    pub fn apply(&mut self, change: &Move, known: &[&'static str]) -> bool {
        // A tab the file no longer shows, a hand edit having taken it away since, is no
        // move's source or destination: its lists stay as they are.
        let shown_ids: Vec<String> = (self.shown_tab_ids().into_iter())
            .map(str::to_owned)
            .collect();
        let shown = |tab: &str| shown_ids.iter().any(|id| id == tab);
        match change {
            Move::Up { tab, .. }
            | Move::Down { tab, .. }
            | Move::Pin { tab, .. }
            | Move::Unpin { tab, .. }
                if !shown(tab) =>
            {
                false
            }
            Move::Up { tool, tab } | Move::Down { tool, tab } => {
                let Some((pinned, _)) = self.find(tab, tool) else {
                    return false;
                };
                let list = self.list(pinned, tab);
                // The known tools swap; an id this build does not know stays after the
                // tool it followed ([`keeping`]).
                let tools: Vec<&'static str> = (list.iter())
                    .filter_map(|id| known.iter().copied().find(|k| *k == id))
                    .collect();
                let Some(at) = tools.iter().position(|t| t == tool) else {
                    return false;
                };
                let other = match change {
                    Move::Up { .. } => at.checked_sub(1),
                    _ => Some(at + 1).filter(|n| *n < tools.len()),
                };
                let Some(other) = other else {
                    return false;
                };
                let mut moved = tools;
                moved.swap(at, other);
                *list = keeping(list, &moved, known);
                true
            }
            Move::Pin { tool, tab } | Move::Unpin { tool, tab } => {
                let pin = matches!(change, Move::Pin { .. });
                if self.find(tab, tool).is_none_or(|(pinned, _)| pinned == pin) {
                    return false;
                }
                self.list(!pin, tab).retain(|id| id != tool);
                let list = self.list(pin, tab);
                let at = if pin { list.len() } else { top(list, known) };
                list.insert(at, (*tool).to_owned());
                // The list it left, where the move emptied it; every other stays.
                let left = if pin {
                    &mut self.order
                } else {
                    &mut self.pinned
                };
                if left.get(tab).is_some_and(Vec::is_empty) {
                    left.remove(tab);
                }
                true
            }
            Move::ToTab { tool, from, to } => {
                // A tab a hand edit has taken away since, or one past the twelfth, takes
                // nothing, and the tool stays where it is.
                if !shown(to) || self.find(to, tool).is_some() {
                    return false;
                }
                // Replayed on a file a hand edit changed: where the tool is no longer in the
                // tab it moved from but in another tab shown, the move is a conflict and is
                // skipped, or it would sit in two tabs. One the edit hid, or left in no tab
                // shown, is moved, placed once; a list for a tab the file no longer shows is
                // never looked at or changed.
                let from = from.as_deref().filter(|from| shown(from));
                let at_source = from.is_some_and(|from| self.find(from, tool).is_some());
                let elsewhere = shown_ids.iter().any(|id| self.find(id, tool).is_some());
                if !at_source && elsewhere {
                    return false;
                }
                if let Some(from) = from {
                    _ = self.take_out(Some(from), tool);
                }
                // Out of `hidden` either way: a hand edit may have hidden it since.
                self.hidden.retain(|id| id != tool);
                let list = self.list(false, to);
                let at = landing(list, tool, known);
                list.insert(at, (*tool).to_owned());
                true
            }
            Move::Hide { tool } => {
                if *tool == EXPORT {
                    return false;
                }
                let mut moved = self.take_out(None, tool);
                if !self.hidden.iter().any(|id| id == tool) {
                    self.hidden.push((*tool).to_owned());
                    moved = true;
                }
                moved
            }
            Move::Toolbar(list) => {
                let moved = self.toolbar.as_ref() != Some(list);
                self.toolbar = Some(list.clone());
                moved
            }
        }
    }

    /// Writes what a move changes into `table`, a workspace's file as read, the rest of it
    /// kept as it is: `version`, `hidden`, `seen`, `[pinned]` and `[order]` (decision 96),
    /// and `toolbar` where the layout has a list (decision 92).
    /// A list for a tab id it lacks, which no move changes, stays as the file has it, an
    /// item this build skips included.
    pub fn write_into(&self, table: &mut Table) {
        let ids = |ids: &[String]| Value::Array(ids.iter().map(|id| id.as_str().into()).collect());
        let declared = |id: &str| self.tabs.iter().any(|tab| tab.id == id);
        let lists = |key: &str, lists: &BTreeMap<String, Vec<String>>| {
            let raw = table.get(key).and_then(Value::as_table);
            let lists = lists.iter().map(|(tab, list)| {
                let kept = raw.filter(|_| !declared(tab)).and_then(|raw| raw.get(tab));
                (tab.clone(), kept.cloned().unwrap_or_else(|| ids(list)))
            });
            Value::Table(lists.collect())
        };
        let (pinned, order) = (lists("pinned", &self.pinned), lists("order", &self.order));
        table.insert("version".into(), VERSION.into());
        table.insert("hidden".into(), ids(&self.hidden));
        table.insert("seen".into(), ids(&self.seen));
        if let Some(toolbar) = &self.toolbar {
            table.insert("toolbar".into(), ids(toolbar));
        }
        if self.pinned.is_empty() {
            table.remove("pinned");
        } else {
            table.insert("pinned".into(), pinned);
        }
        table.insert("order".into(), order);
    }
}

/// Whether `id` can name a workspace, and its file: lower-case letters, digits and
/// hyphens.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// `path` with `suffix` after its whole name.
fn beside(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Where a damaged `file` is set aside: `<id>.toml.bak`, or where that is there already,
/// an earlier one kept, the first of `<id>.toml.1.bak`, `<id>.toml.2.bak` and on that is not.
fn aside(file: &Path) -> PathBuf {
    let first = beside(file, ".bak");
    let mut numbered = (1..).map(|n| beside(file, &format!(".{n}.bak")));
    let free = |path: &PathBuf| !path.exists();
    std::iter::once(first)
        .chain(&mut numbered)
        .find(free)
        .expect("an unused name")
}

/// The workspace `file` holds, with the warnings its reading gave: None where there is no
/// file, it cannot be read, it is damaged, set aside as `<id>.toml.bak` or, keeping an
/// earlier one, a numbered name ([`aside`]), or it lists no tab, left as it is.
pub fn read(file: &Path) -> (Option<Layout>, Vec<String>) {
    let text = match std::fs::read(file) {
        Ok(bytes) => String::from_utf8(bytes).map_err(|_| Refused::Damaged("not UTF-8".into())),
        Err(e) if e.kind() == io::ErrorKind::NotFound => return (None, Vec::new()),
        Err(e) => return (None, vec![format!("not read: {e}")]),
    };
    match text.and_then(|text| Layout::parse(&text)) {
        Ok((layout, mut warnings)) => {
            if layout.version > VERSION {
                let version = layout.version;
                warnings.push(format!(
                    "version {version} is newer than this build's; read for what it knows, never written"
                ));
            }
            (Some(layout), warnings)
        }
        Err(Refused::Damaged(why)) => {
            let aside = aside(file);
            let moved = match std::fs::rename(file, &aside) {
                Ok(()) => format!("set aside as {}", aside.display()),
                Err(e) => format!("could not be set aside: {}", e.kind()),
            };
            (None, vec![format!("damaged, {moved}: {why}")])
        }
        Err(Refused::NoTab) => (None, vec!["lists no tab; not used".to_owned()]),
    }
}

/// The workspace `id`'s file in `folder`, if any, its warnings added to `warnings`, each
/// naming the file.
fn read_in(folder: Option<&Path>, id: &str, warnings: &mut Vec<String>) -> Option<Layout> {
    let file = folder?.join(format!("{id}.toml"));
    let (layout, read) = read(&file);
    let shown = file.display();
    warnings.extend(read.into_iter().map(|w| format!("{shown}: {w}")));
    layout
}

/// The ids of a person's own workspaces whose files `folder` holds: each `<id>.toml` whose
/// name is a workspace id and not a built-in's, in no order; none where the folder is not
/// there, and none with a warning where it cannot be listed.
fn own_ids(folder: &Path, warnings: &mut Vec<String>) -> Vec<String> {
    let entries = match std::fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Vec::new(),
        Err(e) => {
            warnings.push(format!("{}: not listed: {e}", folder.display()));
            return Vec::new();
        }
    };
    let names = entries.filter_map(|entry| entry.ok()?.file_name().into_string().ok());
    names
        .filter_map(|name| name.strip_suffix(".toml").map(str::to_owned))
        .filter(|id| valid_id(id) && !builtin_ids().any(|built| built == id))
        .collect()
}

/// One workspace a person can choose (decision 96, Switching).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    pub id: String,
    /// Its file as read, or a built-in's defaults where it has none.
    pub layout: Layout,
    pub builtin: bool,
    /// Whether its file is a newer build's, as read or as a write found it since
    /// ([`Workspaces::mark_newer`]).
    pub newer: bool,
}

impl Listed {
    /// Its name as View › Workspace shows it: a built-in's message, `workspace-<id>`; a
    /// person's own its file's `name`, else its id, the same in every language.
    pub fn name(&self) -> Title {
        match (&self.layout.name, self.builtin) {
            (_, true) => Title::Message(format!("workspace-{}", self.id)),
            (Some(name), false) => Title::Own(name.clone()),
            (None, false) => Title::Own(self.id.clone()),
        }
    }

    /// The defaults that place its new tools: a built-in's own, and for a person's own
    /// Default's, so a tool new in Adjust joins any workspace with a tab `adjust` (decision
    /// 96, The layout shown, step 3).
    fn defaults(&self) -> Layout {
        defaults_of(&self.id, self.builtin)
    }
}

/// The workspaces there are, read once as the app starts (decision 96): the built-in ones,
/// Default then Essentials, each from its file in the user data folder where it has one,
/// else its defaults; then a person's own, those whose file reads, by name. A file added
/// while the app runs is read at the next start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspaces {
    listed: Vec<Listed>,
}

impl Workspaces {
    /// The built-in workspaces as built, with no file.
    pub fn builtin() -> Workspaces {
        let listed = builtin_ids().map(|id| Listed {
            id: id.to_owned(),
            layout: Layout::defaults(id).expect("built in"),
            builtin: true,
            newer: false,
        });
        Workspaces {
            listed: listed.collect(),
        }
    }

    /// The workspaces whose files `folder`, the user data folder's `workspaces`, holds, with
    /// the built-in ones, and the warnings their reading gave, each naming its file: a
    /// damaged file set aside and one listing no tab refused, a built-in's defaults then
    /// applying and a person's own not listed ([`read`]). With no folder, the built-in
    /// ones as built.
    pub fn read(folder: Option<&Path>) -> (Workspaces, Vec<String>) {
        let mut warnings = Vec::new();
        let mut workspaces = Workspaces::builtin();
        for listed in &mut workspaces.listed {
            if let Some(layout) = read_in(folder, &listed.id, &mut warnings) {
                listed.newer = layout.version > VERSION;
                listed.layout = layout;
            }
        }
        let mut own: Vec<Listed> = Vec::new();
        for id in folder.map_or_else(Vec::new, |f| own_ids(f, &mut warnings)) {
            if let Some(layout) = read_in(folder, &id, &mut warnings) {
                own.push(Listed {
                    id,
                    newer: layout.version > VERSION,
                    layout,
                    builtin: false,
                });
            }
        }
        let shown = |listed: &Listed| match listed.name() {
            Title::Message(name) | Title::Own(name) => name.to_lowercase(),
        };
        own.sort_by(|a, b| shown(a).cmp(&shown(b)).then_with(|| a.id.cmp(&b.id)));
        workspaces.listed.extend(own);
        (workspaces, warnings)
    }

    /// Every workspace, in View › Workspace's order.
    pub fn listed(&self) -> &[Listed] {
        &self.listed
    }

    /// The workspace `id`, its layout worked out from what was read, with the warnings that
    /// gave: Default where there is no such workspace, with a warning.
    pub fn open(&self, id: &str) -> (Workspace, Vec<String>) {
        let mut warnings = Vec::new();
        let listed = self.listed.iter().find(|listed| listed.id == id);
        let listed = listed.unwrap_or_else(|| {
            warnings.push(format!("workspace {id}: none, {DEFAULT} shown"));
            let default = self.listed.iter().find(|listed| listed.id == DEFAULT);
            default.expect("Default is built in")
        });
        let defaults = listed.defaults();
        let (shown, resolved) = resolve(&listed.layout, &[&defaults], &known());
        let where_ = format!("workspace {}", listed.id);
        warnings.extend(resolved.into_iter().map(|w| format!("{where_}: {w}")));
        let workspace = Workspace {
            id: listed.id.clone(),
            layout: listed.layout.clone(),
            builtin: listed.builtin,
            newer: listed.newer,
            hidden_tab: hidden_tab(&shown),
            shown,
        };
        (workspace, warnings)
    }

    /// Keeps `workspace`'s layout as a move left it, so switching back to it shows it so.
    pub fn keep(&mut self, workspace: &Workspace) {
        let listed = self.listed.iter_mut().find(|l| l.id == workspace.id);
        if let Some(listed) = listed {
            listed.layout = workspace.layout.clone();
        }
    }

    /// Marks the workspace `id`'s file a newer build's, as a write found it after it was
    /// read, so it is never written and its first move says so.
    pub fn mark_newer(&mut self, id: &str) {
        let listed = self.listed.iter_mut().find(|l| l.id == id);
        if let Some(listed) = listed {
            listed.newer = true;
        }
    }
}

/// The Hidden tab of the layout `shown`: its hidden tools, as [`Shown::hidden`] lists them,
/// none pinned.
fn hidden_tab(shown: &Shown) -> Tab {
    Tab {
        id: HIDDEN.to_owned(),
        title: Title::Message(format!("tab-{HIDDEN}")),
        icon: HIDDEN_ICON,
        pinned: Vec::new(),
        order: shown.hidden.clone(),
    }
}

/// The defaults that place a workspace's new tools: a built-in's own, and for a person's
/// own Default's (decision 96, The layout shown, step 3).
fn defaults_of(id: &str, builtin: bool) -> Layout {
    let id = if builtin { id } else { DEFAULT };
    Layout::defaults(id).expect("built in")
}

/// Where a tool is in a tab shown ([`Workspace::place`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Place {
    /// The tab's place among those shown.
    pub tab: usize,
    /// Whether it is pinned there.
    pub pinned: bool,
    /// Whether a tool of its list is above it, and below it: Move Tool Up and Move Tool
    /// Down can run.
    pub up: bool,
    pub down: bool,
    /// Its place among the tab's tools, pinned first, from 1, and how many the tab holds,
    /// as screen readers hear where it went ("3 of 12").
    pub at: usize,
    pub of: usize,
}

/// The workspace the Tools panel shows: its id, the layout as read, and the layout shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    /// Its id: the one asked for, or Default's where that has no workspace.
    pub id: String,
    /// Its file as read, or the built-in's defaults where it has none, and as each move has
    /// left it since ([`Workspace::apply`]).
    pub layout: Layout,
    /// Whether it is built in, Default or Essentials.
    pub builtin: bool,
    /// Whether its file is a newer build's, which this build never overwrites.
    pub newer: bool,
    pub shown: Shown,
    /// The Hidden tab (decision 96): the tools in no tab shown, as [`Shown::hidden`] lists
    /// them, none pinned, shown last while Show Hidden Tools is on or a tool is revealed
    /// there.
    hidden_tab: Tab,
}

impl Workspace {
    /// The Default workspace as built, with no file.
    pub fn builtin_default() -> Workspace {
        Workspaces::builtin().open(DEFAULT).0
    }

    /// The workspace `id` as the app opens it with the user data folder's `workspaces`,
    /// `folder`: the workspaces read ([`Workspaces::read`]), then the one asked for opened
    /// ([`Workspaces::open`]), with the warnings of both.
    #[cfg(test)]
    pub fn open(folder: Option<&Path>, id: &str) -> (Workspace, Vec<String>) {
        let (workspaces, mut warnings) = Workspaces::read(folder);
        let (workspace, opened) = workspaces.open(id);
        warnings.extend(opened);
        (workspace, warnings)
    }

    /// The Hidden tab, computed with the layout shown, never kept: its tools are
    /// [`Shown::hidden`], none pinned.
    pub fn hidden_tab(&self) -> &Tab {
        &self.hidden_tab
    }

    /// Whether `tool` is hidden: in no tab shown, so only the Hidden tab holds it.
    pub fn hides(&self, tool: &str) -> bool {
        self.shown.hidden.contains(&tool)
    }

    /// The Toolbar's item ids in order (decision 92), as [`Shown::toolbar`] has them.
    pub fn toolbar(&self) -> &[String] {
        &self.shown.toolbar
    }

    /// The tabs shown.
    pub fn tabs(&self) -> &[Tab] {
        &self.shown.tabs
    }

    /// The tab shown at `index`.
    pub fn tab(&self, index: usize) -> Option<&Tab> {
        self.shown.tabs.get(index)
    }

    /// The place of the tab `id` among those shown.
    pub fn position(&self, id: &str) -> Option<usize> {
        self.shown.tabs.iter().position(|tab| tab.id == id)
    }

    /// The first tab shown that holds `tool`.
    pub fn holding(&self, tool: &str) -> Option<usize> {
        self.shown.tabs.iter().position(|tab| tab.holds(tool))
    }

    /// Whether each tab, by position, can be chosen.
    pub fn enabled(&self) -> [bool; MOST_TABS] {
        let mut enabled = [false; MOST_TABS];
        for (enabled, tab) in enabled.iter_mut().zip(&self.shown.tabs) {
            *enabled = tab.enabled();
        }
        enabled
    }

    /// Whether each tab shown, by position, takes a tool moved there: every tab but one
    /// still to come, a built-in tab whose defaults hold no tool this build has but Scopes,
    /// as decision 38's Detail. A tab the person emptied, which cannot be chosen until it
    /// holds a tool again ([`Tab::enabled`]), takes one.
    pub fn receiving(&self) -> [bool; MOST_TABS] {
        let defaults = defaults_of(&self.id, self.builtin);
        let known = known();
        let to_come = |tab: &Tab| {
            let lists = [&defaults.pinned, &defaults.order];
            let listed: Vec<&Vec<String>> = lists.iter().filter_map(|l| l.get(&tab.id)).collect();
            let built = |t: &&String| *t != SCOPES && known.contains(&t.as_str());
            !listed.is_empty() && !listed.into_iter().flatten().any(|t| built(&t))
        };
        let mut receiving = [false; MOST_TABS];
        for (receives, tab) in receiving.iter_mut().zip(&self.shown.tabs) {
            *receives = tab.enabled() || !to_come(tab);
        }
        receiving
    }

    /// Where `tool` is in the tab shown at `tab`, if that tab holds it.
    pub fn place(&self, tool: &str, tab: usize) -> Option<Place> {
        let shown = self.shown.tabs.get(tab)?;
        let (pinned, at, list, before) = match shown.pinned.iter().position(|t| *t == tool) {
            Some(at) => (true, at, &shown.pinned, 0),
            None => {
                let at = shown.order.iter().position(|t| *t == tool)?;
                (false, at, &shown.order, shown.pinned.len())
            }
        };
        Some(Place {
            tab,
            pinned,
            up: at > 0,
            down: at + 1 < list.len(),
            at: before + at + 1,
            of: shown.pinned.len() + shown.order.len(),
        })
    }

    /// The tab `section-show` moves the hidden `tool` to (decision 96): the first of its
    /// default tabs among the tabs shown that takes a tool ([`Workspace::receiving`]), else
    /// the first tab shown that can be chosen, else the first that takes one; none where
    /// no tab takes one.
    pub fn show_in(&self, tool: &str) -> Option<usize> {
        let defaults = defaults_of(&self.id, self.builtin);
        let receiving = self.receiving();
        let holds = |lists: &BTreeMap<String, Vec<String>>, tab: &str| {
            lists.get(tab).is_some_and(|l| l.iter().any(|t| t == tool))
        };
        let default = defaults.tabs.iter().find_map(|tab| {
            let held = holds(&defaults.pinned, &tab.id) || holds(&defaults.order, &tab.id);
            let at = self.position(&tab.id).filter(|_| held)?;
            receiving[at].then_some(at)
        });
        let tabs = &self.shown.tabs;
        default
            .or_else(|| tabs.iter().position(Tab::enabled))
            .or_else(|| (0..tabs.len()).find(|at| receiving[*at]))
    }

    /// Applies `change` to the layout, as its file will be written ([`Layout::settled`],
    /// [`Layout::apply`]), and works out the layout shown again: whether anything moved.
    pub fn apply(&mut self, change: &Move) -> bool {
        let known = known();
        let mut layout = self.layout.settled(&self.shown, &known);
        if !layout.apply(change, &known) {
            return false;
        }
        let defaults = defaults_of(&self.id, self.builtin);
        let (shown, _) = resolve(&layout, &[&defaults], &known);
        self.hidden_tab = hidden_tab(&shown);
        self.shown = shown;
        self.layout = layout;
        true
    }

    /// The tab it opens on where none was last shown: the first holding a processing tool,
    /// those every edit starts with (Default's Adjust), else the first that can be chosen.
    pub fn opening(&self) -> usize {
        let tabs = &self.shown.tabs;
        (tabs.iter().position(|tab| tab.tools().any(processing)))
            .or_else(|| tabs.iter().position(Tab::enabled))
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests;
