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
//!   (`[pinned]`, `[order]`), the tools it hides (`hidden`) and every tool the build that
//!   wrote it knew (`seen`). A person's own is `<user data>/workspaces/<id>.toml`.
//! - **Built in:** the Default workspace, `data/workspaces/default.toml`, and Essentials,
//!   `data/workspaces/essentials.toml`, embedded and read as a person's own is
//!   ([`Layout::defaults`]); a person's changed copy of one, in the user data folder, takes
//!   its place.
//! - **The workspaces there are,** [`Workspaces`], read once as the app starts: the
//!   built-in ones, then a person's own, each `<id>.toml` that reads, by name, which View ›
//!   Workspace lists; switching works out the layout shown from them ([`Workspaces::open`]).
//! - **The layout shown,** [`Shown`], worked out from the file and the defaults when the
//!   workspace is read ([`resolve`]), never per frame: each tab's lists, the tools hidden,
//!   a new build's new tools placed by `seen`, the first twelve tabs, and Export always in
//!   one of them.
//! - **The Hidden tab,** [`Workspace::hidden_tab`], worked out with the layout shown: the
//!   tools in no tab shown, processing tools in pipeline order, then utilities, never
//!   pinned or reordered there.
//! - **Reading,** [`Workspaces::read`]: a file that is not TOML is set aside as
//!   `<id>.toml.bak`, one listing no tab refused, both with a warning, the built-in's
//!   defaults then applying, or a person's own not listed; a newer version's is read for
//!   what this build knows and never overwritten.
//! - **Writing** a workspace's file is not built: it comes with decision 96's step 4, the
//!   first step that changes a layout; until then nothing writes one, and a person changes
//!   a workspace only by editing its file, read at the next start.

use std::collections::{BTreeMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

use apertrust_ui::Icon;
use toml::{Table, Value};

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
        // What the error says, never `Display`'s quote of the line.
        let table: Table = text
            .parse()
            .map_err(|e: toml::de::Error| Refused::Damaged(e.message().replace('\n', " ")))?;
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
        let layout = Layout {
            version,
            name,
            tabs,
            pinned,
            order,
            hidden,
            seen,
        };
        Ok((layout, warnings))
    }

    /// The built-in workspace `id`'s defaults, its bundled file, or None where no
    /// workspace of that id is built in.
    pub fn defaults(id: &str) -> Option<Layout> {
        let (_, text) = BUILT_IN.iter().find(|(built, _)| *built == id)?;
        let (layout, _) = Layout::parse(text).expect("a bundled workspace reads");
        Some(layout)
    }
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
fn hidden_order(tool: &str) -> (bool, usize) {
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
    let shown = Shown {
        tabs,
        hidden,
        beyond,
    };
    (shown, warnings)
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
        let id = if self.builtin { &self.id } else { DEFAULT };
        Layout::defaults(id).expect("built in")
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
                listed.layout = layout;
            }
        }
        let mut own: Vec<Listed> = Vec::new();
        for id in folder.map_or_else(Vec::new, |f| own_ids(f, &mut warnings)) {
            if let Some(layout) = read_in(folder, &id, &mut warnings) {
                own.push(Listed {
                    id,
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
        let hidden_tab = Tab {
            id: HIDDEN.to_owned(),
            title: Title::Message(format!("tab-{HIDDEN}")),
            icon: HIDDEN_ICON,
            pinned: Vec::new(),
            order: shown.hidden.clone(),
        };
        let workspace = Workspace {
            id: listed.id.clone(),
            layout: listed.layout.clone(),
            newer: listed.layout.version > VERSION,
            shown,
            hidden_tab,
        };
        (workspace, warnings)
    }
}

/// The workspace the Tools panel shows: its id, the layout as read, and the layout shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    /// Its id: the one asked for, or Default's where that has no workspace.
    pub id: String,
    /// Its file as read, or the built-in's defaults where it has none.
    pub layout: Layout,
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
