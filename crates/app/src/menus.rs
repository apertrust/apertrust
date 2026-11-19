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

//! The application menus (`docs/gui.md`, The menus and The menu bar; decision 39).
//!
//! One model, [`MENUS`]: menus of groups that lines separate, each entry an action of the
//! registry by id, a submenu, one of the Mac's own items, Open Recent's list, whose items
//! are the recents the app hands in ([`Recent`]), or View › Workspace's, the workspaces it
//! hands in ([`WorkspaceItem`]; [`Lists`]). It names actions as the
//! Toolbar's `TOOLBAR` does, so the registry still declares each action once: an item's
//! words are its action's message, Undo's and Redo's naming the step they would take back
//! or put back ([`words`]), its key the keymap's first for the platform, and it is enabled
//! exactly when its action can run where it is ([`shown`]). Only actions the
//! registry declares are placed, drawn disabled until they are built; a later one joins the
//! tree when the registry declares it, and a menu with nothing to hold is left out, but for
//! the Mac's Window and Help menus, which AppKit fills (`docs/gui.md`, The menu bar).
//!
//! On the Mac the menus are the system's menu bar, built with muda once the window exists
//! ([`NativeMenus`]). Each frame the app hands it the keymap and the state, and it calls
//! muda only for what changed; a choice comes back through muda's event handler, down a
//! channel, and runs in `App::logic`, before the next frame or, while the window is
//! minimised or hidden, without one ([`runs`] says when). On Windows and Linux the same
//! model is the one menu of the Toolbar's ☰ (decision 39), drawn by the popup service
//! ([`request`]); the id chosen in it runs as the Mac's choices do.

// Off the Mac the Mac's own parts are read only by the tests.
#![cfg_attr(
    not(target_os = "macos"),
    allow(
        dead_code,
        reason = "the Mac's own items and key repeats are the Mac's alone"
    )
)]

use apertrust_ui::i18n::Args;
use apertrust_ui::{Translator, popup, translator};
use eframe::egui::{self, Rect, os::OperatingSystem};

use apertrust_tools::{MenuEntry, MenuItem, Section};

use crate::actions::{self, Action, ActionId, Focus, Keymap, Platforms, Shortcut, State, Trigger};

/// What the system does with a menu on the Mac.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// A menu of the app's own.
    Plain,
    /// The app menu, which macOS titles with the app's name, whatever its title says.
    App,
    /// The Window menu, where AppKit adds its tiling items and lists the windows.
    Window,
    /// The Help menu, where AppKit adds its search.
    Help,
}

/// A menu or a submenu.
pub struct Menu {
    /// Its title, a message id.
    pub title: &'static str,
    pub role: Role,
    /// The platforms that have it.
    pub on: Platforms,
    /// Its entries in groups, which lines separate.
    pub groups: &'static [&'static [Entry]],
}

/// One entry of a menu.
pub enum Entry {
    /// A registry action, on the platforms given.
    Action(ActionId, Platforms),
    Submenu(&'static Menu),
    /// One of the Mac's own items, which the system performs.
    System(System),
    /// Open Recent's list: an item for each recent the app hands in, newest first, each
    /// the registry's action for its place ([`ActionId::OPEN_RECENT`]), its words the
    /// recent's own. A menu holding it is built afresh on the Mac whenever what it shows
    /// changes.
    Recents,
    /// View › Workspace's list: an item for each workspace the app hands in, the built-in
    /// ones then a person's own, each its registry action, `workspace-<id>`, the one shown
    /// checked, its words the workspace's name. Built afresh on the Mac as Open Recent is.
    Workspaces,
}

/// One of Open Recent's entries as its item shows it: its words, the folder's name, and
/// its tip, the path and why it cannot be opened where it cannot, which the drawn menu
/// shows and the Mac's menus, which show no path, leave out; and its path, which a choice
/// of its item opens. Whether it can be chosen is its action's ([`State::recents`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recent {
    pub words: String,
    pub tip: String,
    pub path: std::path::PathBuf,
}

/// One of View › Workspace's items as the app hands it in: the workspace's action,
/// `workspace-<id>` (`actions::workspace`), and its name, a built-in's message in the
/// interface's language or a person's own workspace's name as its file writes it (decision
/// 96), with its tip, why it cannot be chosen where something says, which the drawn menu
/// shows. Whether it is checked is its action's ([`State::workspace`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceItem {
    pub id: ActionId,
    pub words: String,
    pub tip: Option<String>,
}

/// What the app hands in for the menus whose items it lists: Open Recent's recents and View
/// › Workspace's workspaces.
#[derive(Clone, Copy, Debug, Default)]
pub struct Lists<'a> {
    pub recents: &'a [Recent],
    pub workspaces: &'a [WorkspaceItem],
}

/// What Undo and Redo would do, as History names the step, in the interface's language:
/// "Exposure", "Go to Step"; none where there is nothing to do or to name
/// (`history::steps`, from the session's `History`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Steps {
    pub undo: Option<String>,
    pub redo: Option<String>,
}

/// `action`'s words as its item shows them: its message, but Undo and Redo name what they
/// would do, `steps` ("Undo Exposure", `menu-undo-step`; "Redo Go to Step",
/// `menu-redo-step`). With nothing to name, and while a text field has the focus, whose own
/// undo they then are, they read Undo and Redo alone (`docs/gui.md`, The editing items
/// follow focus).
pub fn words(action: &Action, steps: &Steps, state: &State, tr: &Translator) -> String {
    let (step, message) = match action.id {
        ActionId::UNDO => (&steps.undo, "menu-undo-step"),
        ActionId::REDO => (&steps.redo, "menu-redo-step"),
        _ => return tr.text(action.message),
    };
    match step {
        Some(step) if !state.text => {
            let mut args = Args::new();
            args.set("step", step.as_str());
            tr.text_with(message, Some(&args))
        }
        _ => tr.text(action.message),
    }
}

/// Whether `id`'s words follow the photo's history, so the Mac's menus word its item again
/// each frame rather than only when the language changes.
fn names_a_step(id: ActionId) -> bool {
    id == ActionId::UNDO || id == ActionId::REDO
}

/// The Mac's own items, which muda predefines: macOS performs them, so they are not the
/// registry's actions, and they keep the system's keys (Cmd+H, Option+Cmd+H, Cmd+M).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum System {
    Services,
    Hide,
    HideOthers,
    ShowAll,
    Minimise,
    Zoom,
    BringAllToFront,
}

impl System {
    /// Its words, a message id.
    pub fn message(self) -> &'static str {
        match self {
            System::Services => "menu-services",
            System::Hide => "menu-hide-app",
            System::HideOthers => "menu-hide-others",
            System::ShowAll => "menu-show-all",
            System::Minimise => "menu-minimise",
            System::Zoom => "menu-zoom-window",
            System::BringAllToFront => "menu-bring-all-to-front",
        }
    }
}

const fn item(id: ActionId) -> Entry {
    Entry::Action(id, Platforms::All)
}

static PANEL_TAB: Menu = Menu {
    title: "menu-tools-panel-tab",
    role: Role::Plain,
    on: Platforms::All,
    groups: &[&[
        item(ActionId::PANEL_TABS[0]),
        item(ActionId::PANEL_TABS[1]),
        item(ActionId::PANEL_TABS[2]),
        item(ActionId::PANEL_TABS[3]),
        item(ActionId::PANEL_TABS[4]),
        item(ActionId::PANEL_TABS[5]),
        item(ActionId::PANEL_TABS[6]),
        // After the twelve a workspace may show, chosen while it shows (decision 96).
        item(ActionId::PANEL_TAB_HIDDEN),
    ]],
};

/// View › Workspace: every workspace, Default, Essentials, then a person's own by name, the
/// one shown checked (decision 96); Reset Workspace joins it with the tab bar's menu.
static WORKSPACE: Menu = Menu {
    title: "menu-workspace",
    role: Role::Plain,
    on: Platforms::All,
    groups: &[&[Entry::Workspaces]],
};

/// File › Open Recent: the recents, newest first, then Clear Menu, which alone is there
/// with none (`docs/gui.md`, File).
static OPEN_RECENT: Menu = Menu {
    title: "menu-open-recent",
    role: Role::Plain,
    on: Platforms::All,
    groups: &[&[Entry::Recents], &[item(ActionId::CLEAR_RECENT)]],
};

static INTERFACE_SIZE: Menu = Menu {
    title: "menu-interface-size",
    role: Role::Plain,
    on: Platforms::All,
    groups: &[&[
        item(ActionId::SCALE_UP),
        item(ActionId::SCALE_DOWN),
        item(ActionId::SCALE_RESET),
    ]],
};

static SORT_BY: Menu = Menu {
    title: "menu-sort-by",
    role: Role::Plain,
    on: Platforms::All,
    groups: &[
        &[
            item(ActionId::SORT[0]),
            item(ActionId::SORT[1]),
            item(ActionId::SORT[2]),
            item(ActionId::SORT[3]),
        ],
        &[
            item(ActionId::SORT_ASCENDING),
            item(ActionId::SORT_DESCENDING),
        ],
    ],
};

static THUMBNAIL_SIZE: Menu = Menu {
    title: "menu-thumbnail-size",
    role: Role::Plain,
    on: Platforms::All,
    groups: &[&[
        item(ActionId::THUMBNAILS[0]),
        item(ActionId::THUMBNAILS[1]),
        item(ActionId::THUMBNAILS[2]),
        item(ActionId::THUMBNAILS[3]),
    ]],
};

/// View › Background, which the Viewer's right-click menu lists too.
pub static BACKGROUND: Menu = Menu {
    title: "menu-background",
    role: Role::Plain,
    on: Platforms::All,
    groups: &[&[
        item(ActionId::BACKGROUND[0]),
        item(ActionId::BACKGROUND[1]),
        item(ActionId::BACKGROUND[2]),
        item(ActionId::BACKGROUND[3]),
        item(ActionId::BACKGROUND[4]),
        item(ActionId::BACKGROUND[5]),
    ]],
};

static BROWSER_POSITION: Menu = Menu {
    title: "menu-browser-position",
    role: Role::Plain,
    on: Platforms::All,
    groups: &[&[
        item(ActionId::BROWSER_POSITION[0]),
        item(ActionId::BROWSER_POSITION[1]),
        item(ActionId::BROWSER_POSITION[2]),
    ]],
};

/// The menus as `docs/gui.md` (The tree) places them, macOS first, with only the actions
/// the registry declares; the few it declares that no menu holds are the tests' `LEFT_OUT`.
pub static MENUS: [Menu; 7] = [
    // The app menu, macOS only.
    Menu {
        title: "app-name",
        role: Role::App,
        on: Platforms::Mac,
        groups: &[
            // Apertrust's own About, never muda's predefined one, which shows macOS's panel.
            &[item(ActionId::ABOUT)],
            &[item(ActionId::SETTINGS)],
            &[Entry::System(System::Services)],
            &[
                Entry::System(System::Hide),
                Entry::System(System::HideOthers),
                Entry::System(System::ShowAll),
            ],
            // Never muda's predefined Quit, which would leave without the app's exit path.
            &[item(ActionId::QUIT)],
        ],
    },
    Menu {
        title: "menu-file",
        role: Role::Plain,
        on: Platforms::All,
        groups: &[
            &[
                item(ActionId::OPEN),
                item(ActionId::OPEN_FOLDER),
                Entry::Submenu(&OPEN_RECENT),
            ],
            &[
                item(ActionId::IMPORT),
                item(ActionId::EXPORT),
                item(ActionId::EXPORT_AGAIN),
            ],
            // Close; then Close Window on the Mac; Exit on Windows, Quit on Linux.
            &[
                item(ActionId::CLOSE),
                Entry::Action(ActionId::CLOSE_WINDOW, Platforms::Mac),
                Entry::Action(ActionId::QUIT, Platforms::NotMac),
            ],
        ],
    },
    Menu {
        title: "menu-edit",
        role: Role::Plain,
        on: Platforms::All,
        groups: &[
            &[item(ActionId::UNDO), item(ActionId::REDO)],
            &[
                item(ActionId::COPY_EDIT),
                item(ActionId::APPLY_EDIT),
                item(ActionId::COPY_APPLY_PARTS),
            ],
            &[item(ActionId::RESET), item(ActionId::DISCARD_HISTORY)],
            &[Entry::Action(ActionId::SETTINGS, Platforms::NotMac)],
        ],
    },
    Menu {
        title: "menu-photo",
        role: Role::Plain,
        on: Platforms::All,
        groups: &[&[item(ActionId::NEXT_PHOTO), item(ActionId::PREVIOUS_PHOTO)]],
    },
    Menu {
        title: "menu-view",
        role: Role::Plain,
        on: Platforms::All,
        groups: &[
            // The photo's zoom, which the viewer bar's zoom menu offers too.
            &[
                item(ActionId::ZOOM_IN),
                item(ActionId::ZOOM_OUT),
                item(ActionId::ZOOM_FIT),
                item(ActionId::ZOOM_100),
            ],
            // The browser bar's menus (`docs/gui.md`, The browser bar).
            &[
                Entry::Submenu(&SORT_BY),
                Entry::Submenu(&THUMBNAIL_SIZE),
                item(ActionId::SHOW_LABELS),
                Entry::Submenu(&BROWSER_POSITION),
            ],
            &[
                item(ActionId::SHOW_COLLECTIONS),
                Entry::Submenu(&PANEL_TAB),
                Entry::Submenu(&WORKSPACE),
                Entry::Submenu(&INTERFACE_SIZE),
                Entry::Submenu(&BACKGROUND),
            ],
            // The jobs window, here on every platform: the Mac's Window menu keeps the
            // system's items alone.
            &[item(ActionId::SHOW_JOBS)],
            &[item(ActionId::FULL_SCREEN)],
        ],
    },
    // AppKit adds its tiling items and the windows' list.
    Menu {
        title: "menu-window",
        role: Role::Window,
        on: Platforms::Mac,
        groups: &[
            &[Entry::System(System::Minimise), Entry::System(System::Zoom)],
            &[Entry::System(System::BringAllToFront)],
        ],
    },
    // AppKit adds its search on the Mac; Keyboard Shortcuts joins it once the registry
    // declares it. About ends it on Windows and Linux.
    Menu {
        title: "menu-help",
        role: Role::Help,
        on: Platforms::All,
        groups: &[
            &[item(ActionId::SHOW_LOG)],
            &[Entry::Action(ActionId::ABOUT, Platforms::NotMac)],
        ],
    },
];

/// Whether `entry` has something to show on `os`.
fn holds(entry: &Entry, os: OperatingSystem) -> bool {
    match entry {
        Entry::Action(_, on) => on.includes(os),
        Entry::Submenu(menu) => present(menu, os),
        Entry::System(_) => os == OperatingSystem::Mac,
        Entry::Recents | Entry::Workspaces => true,
    }
}

/// Whether `menu` holds a list the app hands in, Open Recent's or View › Workspace's, so
/// its items follow it ([`Lists`]).
fn lists(menu: &Menu) -> bool {
    menu.groups
        .iter()
        .flat_map(|g| g.iter())
        .any(|e| matches!(e, Entry::Recents | Entry::Workspaces))
}

/// One item of a menu whose items follow a list the app hands in, or a line between two of
/// its groups.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Listed {
    Item {
        id: ActionId,
        words: String,
        tip: Option<String>,
        shown: Shown,
        /// Its check mark, where its action chooses one of a set ([`Action::checked`]).
        checked: Option<bool>,
    },
    Separator,
}

/// What `menu`, one that lists what the app hands in ([`lists`]), holds on `os` with
/// `lists`: its actions' items, each recent's, newest first, at most
/// [`ActionId::OPEN_RECENT`]'s, and each workspace's ([`shown`]), in groups that lines
/// separate, a group with nothing to hold left out with its line, so with no recents Open
/// Recent holds Clear Menu alone. Its submenus and the Mac's own items are never in it.
pub fn listed(
    menu: &'static Menu,
    os: OperatingSystem,
    keymap: &Keymap,
    state: &State,
    lists: Lists<'_>,
    tr: &Translator,
    focus: Focus,
) -> Vec<Listed> {
    let item = |id: ActionId, words: String, tip: Option<String>| Listed::Item {
        id,
        words,
        tip,
        shown: shown(id, keymap, state, focus),
        checked: actions::action(id).and_then(|action| action.checked(state)),
    };
    let mut listed = Vec::new();
    for group in groups(menu, os) {
        let mut items = Vec::new();
        for entry in group {
            match entry {
                Entry::Action(id, _) => {
                    let Some(action) = actions::action(*id) else {
                        continue;
                    };
                    items.push(item(*id, tr.text(action.message), None));
                }
                Entry::Recents => {
                    for (id, recent) in ActionId::OPEN_RECENT.iter().zip(lists.recents) {
                        let tip = Some(recent.tip.clone());
                        items.push(item(*id, recent.words.clone(), tip));
                    }
                }
                Entry::Workspaces => {
                    for workspace in lists.workspaces {
                        let tip = workspace.tip.clone();
                        items.push(item(workspace.id, workspace.words.clone(), tip));
                    }
                }
                Entry::Submenu(_) | Entry::System(_) => {}
            }
        }
        if items.is_empty() {
            continue;
        }
        if !listed.is_empty() {
            listed.push(Listed::Separator);
        }
        listed.extend(items);
    }
    listed
}

/// Whether `menu` is there on `os`: when it holds anything there, and on the Mac the
/// Window and Help menus always, since AppKit fills them.
pub fn present(menu: &Menu, os: OperatingSystem) -> bool {
    let filled = os == OperatingSystem::Mac && matches!(menu.role, Role::Window | Role::Help);
    menu.on.includes(os)
        && (filled
            || menu
                .groups
                .iter()
                .flat_map(|g| g.iter())
                .any(|e| holds(e, os)))
}

/// The menus there on `os`, in order.
pub fn menus(os: OperatingSystem) -> impl Iterator<Item = &'static Menu> {
    MENUS.iter().filter(move |menu| present(menu, os))
}

/// `menu`'s groups on `os`, each with only the entries there, and none left empty.
pub fn groups(menu: &'static Menu, os: OperatingSystem) -> Vec<Vec<&'static Entry>> {
    menu.groups
        .iter()
        .map(|group| group.iter().filter(|e| holds(e, os)).collect::<Vec<_>>())
        .filter(|group| !group.is_empty())
        .collect()
}

/// What an action's item shows, but for its words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shown {
    /// Whether it can be chosen: exactly when its action can run where it is.
    pub enabled: bool,
    /// Its key: the first the keymap gives its action on the platform, unless the focused
    /// widget keeps it, so a native menu never takes what is typed into a field
    /// (`docs/gui.md`, Who owns the keys). The editing items keep theirs whatever has the
    /// focus, and act on a focused text field ([`edits_the_field`]), but hold none while a
    /// dialog's own window has the focus, so its own field takes them.
    pub key: Option<Shortcut>,
}

/// Whether `id`'s item is one of the editing items (decision 39), which keep their keys
/// while a text field has the focus and act on the field: Undo and Redo, which Cut, Copy,
/// Paste and Select All are to join.
pub fn edits_the_field(id: ActionId) -> bool {
    id == ActionId::UNDO || id == ActionId::REDO
}

/// What `id`'s item shows with `keymap`, in `state`, while `focus` has the focus.
pub fn shown(id: ActionId, keymap: &Keymap, state: &State, focus: Focus) -> Shown {
    let edits = edits_the_field(id);
    Shown {
        enabled: actions::action(id).is_some_and(|a| a.enabled(state)),
        key: keymap.keys(id).first().copied().filter(|k| {
            if edits {
                !state.dialog_window
            } else {
                !focus.keeps(k.key, k.modifiers)
            }
        }),
    }
}

/// Whether choosing `id`'s item runs it: always when clicked or pressed, and when AppKit
/// performs its key equivalent again for a key held down (`repeat`), only for an action
/// that repeats, since the registry runs any other once a press.
pub fn runs(id: ActionId, repeat: bool) -> bool {
    actions::action(id).is_some_and(|a| !repeat || a.trigger == Trigger::Repeat)
}

/// The ☰'s menu, hung from `anchor` (decision 39): the menus [`MENUS`] has on the
/// context's platform, each a submenu whose words carry its access key, and their items as
/// the Mac's show them ([`shown`]): the action's words, enabled exactly when it can run
/// where it is, and the keymap's first key as the platform spells it, whatever has the
/// focus, since the registry reads every key there and the menu only shows them; and a
/// check mark where the action turns something on or chooses one of a set. An item's id is
/// its action's ([`chosen`]). Open Recent lists `lists`' recents, each with its path in its
/// tip, and View › Workspace its workspaces; Undo and Redo name `steps` ([`words`]).
pub fn request(
    ctx: &egui::Context,
    keymap: &Keymap,
    state: &State,
    lists: Lists<'_>,
    steps: &Steps,
    anchor: Rect,
) -> popup::Request {
    let tr = translator(ctx);
    let os = ctx.os();
    let entries = menus(os)
        .map(|menu| drawn(menu, os, keymap, state, lists, steps, &tr).into())
        .collect();
    popup::Request::pull_down(anchor, tr.text("toolbar-menu"), entries)
}

/// Where a context menu opens (decision 91; `docs/gui.md`, Context menus): a place tools
/// add items to, or one of the app's alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// The photo, a thumbnail or a tool's ⋯: the app's items and the tools', by section and
    /// order.
    Open(apertrust_tools::Place),
    /// The Viewer's surround, round the photo: View › Background's items alone
    /// (`docs/gui.md`, The background).
    Surround,
}

/// What a context menu's items act on (decision 91).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    /// The active photo, with the point under the pointer in the photo's coordinates as
    /// pickers take them (`apertrust_tools::pick::Region::Point`, 0 to 1 across the photo
    /// as export renders it), none when the menu opens from the keys.
    Photo { point: Option<[f32; 2]> },
    /// The selection, a thumbnail's menu's: its items act on the selection, the active
    /// photo alone until several can be selected, and opening the menu changes neither
    /// the selection nor the photo in the Viewer.
    Selection,
    /// The tool or utility whose ⋯ it is, by its id: only its own items join the menu.
    Tool(&'static str),
    /// The app itself, as the surround's items set its background.
    App,
}

/// The context menu at `place`, acting on `target`, hung from `anchor`, the point
/// right-clicked for a right-click (decision 91): the app's items there ([`APP_ITEMS`]) and
/// those tools and utilities add ([`added`]) together, by section and order, a line
/// wherever the section changes ([`built`]). The surround, the app's alone, holds View ›
/// Background's items. Each item is its action's, as the ☰'s ([`shown`]); in the photo's
/// and a thumbnail's menus an item that cannot run is hidden, but shown disabled while only
/// the photo's not yet being live stops it, and no item shows its key ([`pared`]). In ⋯
/// an item that cannot run is drawn disabled with its tip, as in a pull-down. Its items'
/// ids are their actions' ([`chosen`]).
pub fn context(
    ctx: &egui::Context,
    place: Place,
    target: &Target,
    keymap: &Keymap,
    state: &State,
    anchor: Rect,
) -> popup::Request {
    let tools: Vec<_> = apertrust_tools::TOOLS
        .iter()
        .map(|tool| tool.definition())
        .collect();
    let added = match place {
        Place::Open(open) => added(open, target, &tools, apertrust_tools::utility::UTILITIES),
        Place::Surround => Vec::new(),
    };
    let (os, tr) = (ctx.os(), translator(ctx));
    built(place, target, &added, keymap, state, os, &tr, anchor)
}

/// The entries of `tool`'s ⋯, its header's options (decisions 91 and 93): the items it
/// declares at `Place::Options`, each its registry action's, built as [`context`] builds
/// every menu, enabled from `state`, an item that cannot run drawn disabled with its tip,
/// each saying in its tip what it does where the registry words it
/// ([`actions::options_tip`]). None for a tool that declares none; the shell adds the
/// layout's items after them ([`layout_items`]).
pub fn options(
    ctx: &egui::Context,
    tool: &'static str,
    keymap: &Keymap,
    state: &State,
) -> Vec<popup::Entry> {
    let place = Place::Open(apertrust_tools::Place::Options);
    context(
        ctx,
        place,
        &Target::Tool(tool),
        keymap,
        state,
        Rect::NOTHING,
    )
    .entries
}

/// A tab shown, as ⋯'s Move Tool to Tab ▸ and Show Tool in Tab ▸ list it: its name in the
/// interface's language, and whether it takes a tool, every tab but one still to come
/// (`layout::Workspace::receiving`).
pub struct TabItem {
    pub name: String,
    pub enabled: bool,
}

/// The layout's items in `tool`'s ⋯ (decisions 83 and 96), last, after its own: where
/// `state.section` says it is in a tab, Pin Tool, or Unpin Tool where it is pinned, Move
/// Tool Up and Move Tool Down, Move Tool to Tab ▸ with each of `tabs` by name and Remove
/// Tool; where it is hidden, Show Tool in Tab ▸ alone. Each item is its registry action's,
/// enabled as it can run: Move Tool Up is disabled first in its list, the pinned one or
/// the scrolling one, and Move Tool Down last, each saying so; a tab is disabled where it
/// holds the tool or takes none, the latter saying it comes in a later version; Remove
/// Tool says what it does, and for Export, which every workspace keeps, why it cannot.
/// None where the tool has no place in the workspace.
pub fn layout_items(
    ctx: &egui::Context,
    tool: &str,
    keymap: &Keymap,
    state: &State,
    tabs: &[TabItem],
) -> Vec<popup::Entry> {
    let Some(placing) = state.section else {
        return Vec::new();
    };
    let (os, tr) = (ctx.os(), translator(ctx));
    let steps = Steps::default();
    let item = |id: ActionId| action_item(id, keymap, state, &steps, os, &tr);
    let to_tab = |(i, tab): (usize, &TabItem)| {
        let mut item = item(*ActionId::SECTION_MOVE_TO_TAB.get(i)?)?;
        item.text = tab.name.clone();
        if !tab.enabled {
            let mut args = Args::new();
            args.set("tab", tab.name.as_str());
            item = item.tip(tr.text_with("panel-tab-later", Some(&args)));
        }
        Some(popup::Entry::from(item))
    };
    let tabs: Vec<popup::Entry> = tabs.iter().enumerate().filter_map(to_tab).collect();
    let title = if placing.hidden {
        "menu-section-show-in-tab"
    } else {
        "menu-section-move-to-tab"
    };
    let submenu = popup::Submenu::new(title, tr.text(title), tabs).into();
    if placing.hidden {
        return vec![submenu];
    }
    let mut entries = Vec::new();
    if let Some(mut pin) = item(ActionId::SECTION_PIN) {
        pin.text = tr.text(actions::section_words(ActionId::SECTION_PIN, placing));
        entries.push(pin.into());
    }
    // Move Tool Up and Move Tool Down, each disabled at its end of the tool's list, saying so.
    let list = if placing.pinned { "pinned" } else { "other" };
    let ends = [
        (
            ActionId::SECTION_MOVE_UP,
            placing.up,
            "section-move-up-first",
        ),
        (
            ActionId::SECTION_MOVE_DOWN,
            placing.down,
            "section-move-down-last",
        ),
    ];
    for (id, can, why) in ends {
        let Some(mut moves) = item(id) else {
            continue;
        };
        if !can {
            let mut args = Args::new();
            args.set("tool", crate::tools::title(&tr, tool));
            args.set("list", list);
            moves = moves.tip(tr.text_with(why, Some(&args)));
        }
        entries.push(moves.into());
    }
    entries.push(submenu);
    if let Some(hide) = item(ActionId::SECTION_HIDE) {
        let tip = if placing.removable {
            let mut args = Args::new();
            args.set("tool", crate::tools::title(&tr, tool));
            tr.text_with("section-hide-tip", Some(&args))
        } else {
            tr.text("section-hide-export")
        };
        entries.push(hide.tip(tip).into());
    }
    entries
}

/// The app's items first in the Tools panel's ⋮ (decision 96), before the tab bar's own:
/// Find Tool…, with its keys, and Show Hidden Tools, as the ☰'s menu draws an item
/// ([`shown`]), Show Hidden Tools' tip `tip` where the
/// dot on ⋮ or `--conf` says why ("Hidden tools change this photo: Exposure"); then
/// Workspace ▸, View › Workspace's submenu with `workspaces`, the one shown checked, each
/// item its `workspace-<id>`, so a choice runs as the menu's does ([`chosen`]). The tab
/// bar's menu's other items join them with that menu.
pub fn tools_panel(
    ctx: &egui::Context,
    keymap: &Keymap,
    state: &State,
    tip: Option<&str>,
    workspaces: &[WorkspaceItem],
) -> Vec<popup::Entry> {
    let (os, tr) = (ctx.os(), translator(ctx));
    let steps = Steps::default();
    let find = action_item(ActionId::FIND_TOOL, keymap, state, &steps, os, &tr);
    let item = action_item(ActionId::SHOW_HIDDEN_TOOLS, keymap, state, &steps, os, &tr);
    let item = item.map(|item| match tip {
        Some(tip) => item.tip(tip),
        None => item,
    });
    let lists = Lists {
        recents: &[],
        workspaces,
    };
    let workspace = drawn(&WORKSPACE, os, keymap, state, lists, &steps, &tr);
    let entries = find.into_iter().chain(item).map(Into::into);
    entries.chain([workspace.into()]).collect()
}

/// The app's items in the places open to tools' (decision 91), declared as the tools'
/// are: the photo's Export… and Background ▸, View › Background's items as a submenu, and
/// a thumbnail's Export…. Copy Edit and Apply Edit join Edit's section once built, Show in
/// Finder and Show in Folder once built; Reset is History's. The app's are held to no
/// section's rule.
pub static APP_ITEMS: [MenuItem; 3] = [
    MenuItem {
        place: apertrust_tools::Place::Photo,
        section: Section::EXPORT,
        order: 100,
        entry: MenuEntry::Action(ActionId::EXPORT.0),
    },
    MenuItem {
        place: apertrust_tools::Place::Photo,
        section: Section::VIEW,
        order: 100,
        entry: MenuEntry::Submenu {
            title: BACKGROUND.title,
            actions: &[
                ActionId::BACKGROUND[0].0,
                ActionId::BACKGROUND[1].0,
                ActionId::BACKGROUND[2].0,
                ActionId::BACKGROUND[3].0,
                ActionId::BACKGROUND[4].0,
                ActionId::BACKGROUND[5].0,
            ],
        },
    },
    MenuItem {
        place: apertrust_tools::Place::Thumbnail,
        section: Section::EXPORT,
        order: 100,
        entry: MenuEntry::Action(ActionId::EXPORT.0),
    },
];

/// One item of a context menu with its actions by their registry ids, at its section and
/// order: an action alone, or a submenu's, under its title.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Added {
    section: Section,
    order: u32,
    /// A submenu's title, a message id; none for an action alone.
    title: Option<&'static str>,
    actions: Vec<String>,
}

impl Added {
    /// `item`, each of its actions' ids as `id` gives it.
    fn of(item: &MenuItem, id: &dyn Fn(&str) -> String) -> Added {
        let title = match item.entry {
            MenuEntry::Action(_) => None,
            MenuEntry::Submenu { title, .. } => Some(title),
        };
        Added {
            section: item.section,
            order: item.order,
            title,
            actions: item
                .entry
                .actions()
                .iter()
                .map(|action| id(action))
                .collect(),
        }
    }
}

/// The app's items at `place` ([`APP_ITEMS`]).
fn app_items(place: apertrust_tools::Place) -> Vec<Added> {
    let items = APP_ITEMS.iter().filter(|item| item.place == place);
    items
        .map(|item| Added::of(item, &|id| id.to_string()))
        .collect()
}

/// The items `tools` and `utilities` add to `place` for `target`, by their registry actions'
/// ids: a tool's action is `tool-<tool>-<action>`, a utility's its own. In ⋯ only its own
/// tool's join.
fn added(
    place: apertrust_tools::Place,
    target: &Target,
    tools: &[&apertrust_tools::ProcessingDefinition],
    utilities: &[apertrust_tools::utility::UtilityDefinition],
) -> Vec<Added> {
    let joins = |id: &str| {
        place != apertrust_tools::Place::Options || matches!(target, Target::Tool(t) if *t == id)
    };
    let at = |items: &'static [MenuItem], id: &dyn Fn(&str) -> String| {
        let items = items.iter().filter(move |item| item.place == place);
        items.map(|item| Added::of(item, id)).collect::<Vec<_>>()
    };
    let tools = tools
        .iter()
        .filter(|tool| joins(tool.tool.id))
        .flat_map(|tool| {
            at(tool.tool.menus, &|action| {
                format!("tool-{}-{action}", tool.tool.id)
            })
        });
    let utilities = (utilities.iter())
        .filter(|utility| joins(utility.tool.id))
        .flat_map(|utility| at(utility.tool.menus, &|action| action.to_string()));
    tools.chain(utilities).collect()
}

/// [`context`]'s menu with the app's items at `place` and `added`, sorted by section and
/// order, a line wherever the section changes. An action the registry does not declare is
/// left out, and so is a submenu left with none.
#[expect(
    clippy::too_many_arguments,
    reason = "the menu's place, target and state"
)]
fn built(
    place: Place,
    target: &Target,
    added: &[Added],
    keymap: &Keymap,
    state: &State,
    os: OperatingSystem,
    tr: &Translator,
    anchor: Rect,
) -> popup::Request {
    let steps = Steps::default();
    let Place::Open(open) = place else {
        let lists = Lists::default();
        let entries = drawn(&BACKGROUND, os, keymap, state, lists, &steps, tr).entries;
        return popup::Request::pull_down(anchor, name(place, target, tr), entries);
    };
    let options = open == apertrust_tools::Place::Options;
    let item = |id: &String| {
        let id = chosen(id)?;
        let item = action_item(id, keymap, state, &steps, os, tr)?;
        // In ⋯ an item says what it does, as a pull-down's items do (decision 93).
        Some(match actions::options_tip(id, state).filter(|_| options) {
            Some(tip) => item.tip(tr.text(tip)),
            None => item,
        })
    };
    let all = app_items(open);
    let all = all.iter().chain(added).filter_map(|added| {
        let entry = match added.title {
            None => item(added.actions.first()?)?.into(),
            Some(title) => {
                let items = added.actions.iter().filter_map(item).map(Into::into);
                let items: Vec<popup::Entry> = items.collect();
                if items.is_empty() {
                    return None;
                }
                popup::Submenu::new(title, tr.text(title), items).into()
            }
        };
        Some((added.section, added.order, entry))
    });
    let mut items: Vec<(Section, u32, popup::Entry)> = all.collect();
    items.sort_by_key(|(section, order, _)| (*section, *order));
    let mut entries = Vec::new();
    let mut last = None;
    for (section, _, entry) in items {
        if last.is_some_and(|last| last != section) {
            entries.push(popup::Entry::Separator);
        }
        last = Some(section);
        entries.push(entry);
    }
    if let apertrust_tools::Place::Photo | apertrust_tools::Place::Thumbnail = open {
        // A photo open for viewing only never takes what waits for it to go live.
        entries = pared(entries, state.waiting && !state.view_only);
    }
    popup::Request::pull_down(anchor, name(place, target, tr), entries)
}

/// What screen readers call the menu at `place`: the surround's is Background, as View ›
/// Background's submenu is; ⋯'s its tool's or utility's options, as its button is.
fn name(place: Place, target: &Target, tr: &Translator) -> String {
    match (place, target) {
        (Place::Surround, _) => tr.text(BACKGROUND.title),
        (Place::Open(apertrust_tools::Place::Photo), _) => tr.text("context-menu-photo"),
        (Place::Open(apertrust_tools::Place::Thumbnail), _) => tr.text("context-menu-thumbnail"),
        (Place::Open(apertrust_tools::Place::Options), target) => {
            let tool = match target {
                Target::Tool(id) if apertrust_tools::utility::utility(id).is_some() => {
                    tr.text(&format!("section-{id}"))
                }
                Target::Tool(tool) => tr.text(&apertrust_tools::messages::title(tool)),
                _ => String::new(),
            };
            let mut args = Args::new();
            args.set("section", tool);
            tr.text_with("section-options", Some(&args))
        }
    }
}

/// `entries` as the photo's and a thumbnail's menus show them (decision 91), as macOS's
/// context menus do: with no keys, and an item that cannot run hidden, unless the photo
/// `waits` to become live and nothing else stops it, no tip saying why
/// ([`actions::why_unavailable`]), when it is shown disabled, since it runs in a moment. A
/// submenu left with nothing is hidden too, and lines left at an end or beside another go.
fn pared(entries: Vec<popup::Entry>, waits: bool) -> Vec<popup::Entry> {
    let mut kept = Vec::new();
    for entry in entries {
        let entry = match entry {
            popup::Entry::Item(item) if !item.enabled && (!waits || item.tip.is_some()) => {
                continue;
            }
            popup::Entry::Item(item) => popup::Entry::Item(popup::Item {
                shortcut: None,
                ..item
            }),
            popup::Entry::Submenu(submenu) => {
                let entries = pared(submenu.entries, waits);
                if entries.iter().all(|e| matches!(e, popup::Entry::Separator)) {
                    continue;
                }
                popup::Entry::Submenu(popup::Submenu { entries, ..submenu })
            }
            popup::Entry::Separator => {
                if matches!(kept.last(), None | Some(popup::Entry::Separator)) {
                    continue;
                }
                popup::Entry::Separator
            }
        };
        kept.push(entry);
    }
    if matches!(kept.last(), Some(popup::Entry::Separator)) {
        kept.pop();
    }
    kept
}

/// `id`'s item, as a menu shows it ([`shown`]): its words, enabled exactly when it can run
/// where it is, the keymap's first key, a check mark where it turns something on or
/// chooses one of a set, and why it cannot run where something says; none for an action
/// the registry does not declare.
fn action_item(
    id: ActionId,
    keymap: &Keymap,
    state: &State,
    steps: &Steps,
    os: OperatingSystem,
    tr: &Translator,
) -> Option<popup::Item> {
    let action = actions::action(id)?;
    let shown = shown(id, keymap, state, Focus::None);
    let mut item = popup::Item::new(id.0, words(action, steps, state, tr)).enabled(shown.enabled);
    if let Some(key) = shown.key {
        item = item.shortcut(key.spell(os, tr));
    }
    if let Some(on) = action.checked(state) {
        item = item.checked(on);
    }
    if let Some(why) = actions::why_unavailable(id, state) {
        item = item.tip(tr.text(why));
    }
    Some(item)
}

/// `menu` as a submenu of the ☰'s menu, its id its title's message id; one that lists what
/// the app hands in holds `lists` ([`listed`]), and Undo and Redo name `steps`.
fn drawn(
    menu: &'static Menu,
    os: OperatingSystem,
    keymap: &Keymap,
    state: &State,
    lists: Lists<'_>,
    steps: &Steps,
    tr: &Translator,
) -> popup::Submenu {
    if self::lists(menu) {
        let listed = listed(menu, os, keymap, state, lists, tr, Focus::None);
        let entries = listed.into_iter().map(|listed| match listed {
            Listed::Item {
                id,
                words,
                tip,
                shown,
                checked,
            } => {
                let mut item = popup::Item::new(id.0, words).enabled(shown.enabled);
                if let Some(key) = shown.key {
                    item = item.shortcut(key.spell(os, tr));
                }
                if let Some(tip) = tip {
                    item = item.tip(tip);
                }
                if let Some(on) = checked {
                    item = item.checked(on);
                }
                item.into()
            }
            Listed::Separator => popup::Entry::Separator,
        });
        return popup::Submenu::new(menu.title, tr.text(menu.title), entries.collect());
    }
    let mut entries = Vec::new();
    for (i, group) in groups(menu, os).into_iter().enumerate() {
        if i > 0 {
            entries.push(popup::Entry::Separator);
        }
        for entry in group {
            match entry {
                Entry::Action(id, _) => {
                    entries.extend(action_item(*id, keymap, state, steps, os, tr).map(Into::into));
                }
                Entry::Submenu(inner) => {
                    entries.push(drawn(inner, os, keymap, state, lists, steps, tr).into());
                }
                // The Mac's alone, which `groups` leaves out elsewhere; the lists are in
                // menus of their own ([`listed`]).
                Entry::System(_) | Entry::Recents | Entry::Workspaces => {}
            }
        }
    }
    popup::Submenu::new(menu.title, tr.text(menu.title), entries)
}

/// The action of the item chosen in the ☰'s menu, by its id.
pub fn chosen(id: &str) -> Option<ActionId> {
    actions::named(id).map(|a| a.id)
}

#[cfg(target_os = "macos")]
pub use mac::NativeMenus;

/// No native menus off the Mac: Windows and Linux will draw [`MENUS`] in the Toolbar.
#[cfg(not(target_os = "macos"))]
pub enum NativeMenus {}

#[cfg(not(target_os = "macos"))]
impl NativeMenus {
    pub fn install(_: &eframe::egui::Context) -> Option<NativeMenus> {
        None
    }

    pub fn chosen(&self) -> Vec<ActionId> {
        match *self {}
    }

    pub fn update(
        &mut self,
        _: &eframe::egui::Context,
        _: &Keymap,
        _: &State,
        _: Lists<'_>,
        _: &Steps,
    ) {
        match *self {}
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::str::FromStr;
    use std::sync::mpsc;

    use apertrust_ui::{Locale, Translator, translator};
    use eframe::egui::{self, Key, os::OperatingSystem};
    use muda::accelerator::{Key as Logical, KeyAccelerator, Modifiers as Held, NamedKey};
    use muda::{
        CheckMenuItem, IsMenuItem, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu,
    };

    use super::{
        Entry, Listed, Lists, Role, Shown, Steps, System, groups, listed, lists, menus,
        names_a_step, runs, shown, words,
    };
    use crate::actions::{self, ActionId, Focus, Keymap, Shortcut, State};

    /// The Mac's menu bar, built from [`super::MENUS`] once the window exists, on the main
    /// thread, and handed to AppKit in place of winit's default menu.
    pub struct NativeMenus {
        /// Kept: dropping it would take the menu bar from AppKit.
        bar: muda::Menu,
        /// Each action's item, with what it shows now.
        items: Vec<Placed>,
        /// The menus whose items follow what the app hands in, Open Recent and View ›
        /// Workspace, each built afresh as what it shows changes.
        listing: Vec<Listing>,
        /// The menus' titles and the system's items, by their message ids.
        titles: Vec<(&'static str, Titled)>,
        /// The language the words are in.
        locale: Locale,
        /// The ids of the items chosen, from muda's event handler, each with whether a
        /// key's repeat chose it.
        chosen: mpsc::Receiver<(MenuId, bool)>,
        /// Whether a key muda refused has been reported.
        refused: bool,
    }

    /// An action's item, where it sits and what it shows.
    struct Placed {
        id: ActionId,
        item: ActionItem,
        menu: Submenu,
        position: usize,
        shown: Shown,
        words: String,
    }

    enum Titled {
        Menu(Submenu),
        System(PredefinedMenuItem),
    }

    enum ActionItem {
        Plain(MenuItem),
        Checked(CheckMenuItem),
    }

    impl ActionItem {
        fn as_item(&self) -> &dyn IsMenuItem {
            match self {
                Self::Plain(item) => item,
                Self::Checked(item) => item,
            }
        }

        fn set_text(&self, text: &str) {
            match self {
                Self::Plain(item) => item.set_text(text),
                Self::Checked(item) => item.set_text(text),
            }
        }

        fn set_enabled(&self, enabled: bool) {
            match self {
                Self::Plain(item) => item.set_enabled(enabled),
                Self::Checked(item) => item.set_enabled(enabled),
            }
        }

        fn set_key_accelerator(&self, key: Option<KeyAccelerator>) -> muda::Result<()> {
            match self {
                Self::Plain(item) => item.set_key_accelerator(key),
                Self::Checked(item) => item.set_key_accelerator(key),
            }
        }
    }

    /// A menu whose items follow what the app hands in, with what it shows now, none until
    /// it is first filled.
    struct Listing {
        menu: &'static super::Menu,
        submenu: Submenu,
        shown: Option<Vec<Listed>>,
        /// Its check items, each with whether it is checked now: muda toggles one as it is
        /// chosen, before the app hears of it, so each update puts every check back, as
        /// [`Placed`]'s are, a choice of the workspace shown changing nothing else.
        checks: Vec<(CheckMenuItem, bool)>,
    }

    impl Listing {
        /// Puts `listed` in place of its items: the actions', the recents' and the
        /// workspaces' items, by their actions' ids, one with a check mark as a check
        /// item, and the lines. A key muda cannot set is left out.
        fn fill(&mut self, listed: &[Listed]) -> muda::Result<()> {
            while self.submenu.remove_at(0).is_some() {}
            self.checks.clear();
            for listed in listed {
                match listed {
                    Listed::Item {
                        id,
                        words,
                        shown,
                        checked,
                        ..
                    } => {
                        let key = shown.key.and_then(key_equivalent);
                        let (enabled, id) = (shown.enabled, id.0);
                        if let Some(checked) = checked {
                            let item = CheckMenuItem::with_id(id, words, enabled, *checked, None);
                            if key.is_some() {
                                item.set_key_accelerator(key)?;
                            }
                            self.submenu.append(&item)?;
                            self.checks.push((item, *checked));
                        } else {
                            let item = MenuItem::with_id(id, words, enabled, None);
                            if key.is_some() {
                                item.set_key_accelerator(key)?;
                            }
                            self.submenu.append(&item)?;
                        }
                    }
                    Listed::Separator => {
                        self.submenu.append(&PredefinedMenuItem::separator())?;
                    }
                }
            }
            Ok(())
        }

        /// Puts each check item's mark back as last filled, whatever a choice of it did.
        fn recheck(&self) {
            for (item, checked) in &self.checks {
                item.set_checked(*checked);
            }
        }
    }

    impl NativeMenus {
        /// Builds the menu bar in the interface's language and hands it to AppKit, the
        /// Window and Help menus as such; None, with a warning, if muda fails. Its items
        /// rest, disabled and without keys, until the first [`NativeMenus::update`].
        pub fn install(ctx: &egui::Context) -> Option<NativeMenus> {
            let built = NativeMenus::build(ctx);
            if let Err(e) = &built {
                tracing::warn!(target: "gpu", "menu bar not built: {e}");
            }
            built.ok()
        }

        fn build(ctx: &egui::Context) -> muda::Result<NativeMenus> {
            let tr = translator(ctx);
            let (sender, chosen) = mpsc::channel();
            let mut built = NativeMenus {
                bar: muda::Menu::new(),
                items: Vec::new(),
                listing: Vec::new(),
                titles: Vec::new(),
                locale: tr.locale(),
                chosen,
                refused: false,
            };
            let mut roles = Vec::new();
            for menu in menus(OperatingSystem::Mac) {
                let submenu = built.submenu(menu, &tr)?;
                built.bar.append(&submenu)?;
                roles.push((menu.role, submenu));
            }
            built.bar.init_for_nsapp();
            // Once the bar is the app's: muda finds them in it.
            for (role, submenu) in &roles {
                match role {
                    Role::Window => submenu.set_as_windows_menu_for_nsapp(),
                    Role::Help => submenu.set_as_help_menu_for_nsapp(),
                    Role::App | Role::Plain => {}
                }
            }
            // A choice wakes egui as an engine event does, and runs after the next frame.
            let ctx = ctx.clone();
            MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                // Now, while AppKit handles the key event the item answers, if any.
                let repeat = apertrust_display::handling_key_repeat();
                let _ = sender.send((event.id, repeat));
                ctx.request_repaint();
            }));
            Ok(built)
        }

        /// `menu` as a muda submenu, its entries in place, recorded for updates.
        fn submenu(
            &mut self,
            menu: &'static super::Menu,
            tr: &Translator,
        ) -> muda::Result<Submenu> {
            // muda leaves out the `&` of a top-level menu's access key on the Mac.
            let submenu = Submenu::new(tr.text(menu.title), true);
            // Filled by each update, as the lists change.
            if lists(menu) {
                self.listing.push(Listing {
                    menu,
                    submenu: submenu.clone(),
                    shown: None,
                    checks: Vec::new(),
                });
                self.titles
                    .push((menu.title, Titled::Menu(submenu.clone())));
                return Ok(submenu);
            }
            for (i, group) in groups(menu, OperatingSystem::Mac).into_iter().enumerate() {
                if i > 0 {
                    submenu.append(&PredefinedMenuItem::separator())?;
                }
                for entry in group {
                    match entry {
                        Entry::Action(id, _) => {
                            let words =
                                actions::action(*id).map_or(String::new(), |a| tr.text(a.message));
                            let item = MenuItem::with_id(id.0, &words, false, None);
                            let position = submenu.items().len();
                            submenu.append(&item)?;
                            self.items.push(Placed {
                                id: *id,
                                item: ActionItem::Plain(item),
                                menu: submenu.clone(),
                                position,
                                shown: Shown {
                                    enabled: false,
                                    key: None,
                                },
                                words,
                            });
                        }
                        Entry::Submenu(inner) => {
                            let inner = self.submenu(inner, tr)?;
                            submenu.append(&inner)?;
                        }
                        Entry::System(system) => {
                            let item = predefined(*system, &tr.text(system.message()));
                            submenu.append(&item)?;
                            self.titles.push((system.message(), Titled::System(item)));
                        }
                        // In menus of their own, filled by each update.
                        Entry::Recents | Entry::Workspaces => {}
                    }
                }
            }
            self.titles
                .push((menu.title, Titled::Menu(submenu.clone())));
            Ok(submenu)
        }

        /// The actions chosen from the menus since the last call, in order, but for a
        /// press action chosen by a key's repeat ([`runs`]). `App::logic` runs them, each
        /// only if it can still run where it is (`Shell::run`).
        pub fn chosen(&self) -> Vec<ActionId> {
            self.chosen
                .try_iter()
                .filter_map(|(id, repeat)| {
                    let action = actions::named(&id.0)?;
                    runs(action.id, repeat).then_some(action.id)
                })
                .collect()
        }

        /// Brings every item up to date with `keymap`, `state`, the focus and the
        /// language, calling muda only for what changed: a language chosen in Settings
        /// renames every menu, and while a widget has focus no item holds a key it keeps.
        /// Open Recent and View › Workspace list `lists`, each built afresh only when what
        /// it shows changes; Undo and Redo name `steps`.
        pub fn update(
            &mut self,
            ctx: &egui::Context,
            keymap: &Keymap,
            state: &State,
            lists: Lists<'_>,
            steps: &Steps,
        ) {
            let tr = translator(ctx);
            let renamed = tr.locale() != self.locale;
            for placed in &mut self.items {
                if !(renamed || names_a_step(placed.id)) {
                    continue;
                }
                let Some(action) = actions::action(placed.id) else {
                    continue;
                };
                let now = words(action, steps, state, &tr);
                if now != placed.words {
                    placed.item.set_text(&now);
                    placed.words = now;
                }
            }
            if renamed {
                self.locale = tr.locale();
                for (message, titled) in &self.titles {
                    match titled {
                        Titled::Menu(menu) => menu.set_text(tr.text(message)),
                        Titled::System(item) => item.set_text(tr.text(message)),
                    }
                }
            }
            let focus = Focus::of(ctx);
            for placed in &mut self.items {
                let now = shown(placed.id, keymap, state, focus);
                if let Some(checked) =
                    actions::action(placed.id).and_then(|action| action.checked(state))
                    && let Err(error) = placed.set_checked(checked)
                {
                    tracing::warn!(target: "gpu", "menu check for {} not set: {error}", placed.id.0);
                }
                if now.enabled != placed.shown.enabled {
                    placed.item.set_enabled(now.enabled);
                }
                if now.key != placed.shown.key
                    && let Err(e) = placed.set_key(now.key.and_then(key_equivalent))
                    && !self.refused
                {
                    tracing::warn!(target: "gpu", "menu key for {} not set: {e}", placed.id.0);
                    self.refused = true;
                }
                placed.shown = now;
            }
            let mac = OperatingSystem::Mac;
            for listing in &mut self.listing {
                let now = listed(listing.menu, mac, keymap, state, lists, &tr, focus);
                if listing.shown.as_ref() == Some(&now) {
                    listing.recheck();
                    continue;
                }
                if let Err(e) = listing.fill(&now)
                    && !self.refused
                {
                    tracing::warn!(target: "gpu", "{} not filled: {e}", listing.menu.title);
                    self.refused = true;
                }
                listing.shown = Some(now);
            }
        }
    }

    impl Placed {
        fn set_checked(&mut self, checked: bool) -> muda::Result<()> {
            if let ActionItem::Checked(item) = &self.item {
                item.set_checked(checked);
                return Ok(());
            }
            let item =
                CheckMenuItem::with_id(self.id.0, &self.words, self.shown.enabled, checked, None);
            item.set_key_accelerator(self.shown.key.and_then(key_equivalent))?;
            self.menu.remove(self.item.as_item())?;
            if let Err(error) = self.menu.insert(&item, self.position) {
                let _ = self.menu.insert(self.item.as_item(), self.position);
                return Err(error);
            }
            self.item = ActionItem::Checked(item);
            Ok(())
        }

        /// Gives the item `key` as its key equivalent, or none. Given none, muda 0.21 keeps
        /// the key equivalent it had (muda 0.21 leaves it in place), so the item is then put
        /// back in its place, which builds it afresh without one. That relies on AppKit
        /// only appending its own items to a menu (Edit's dictation, Window's tiling), so
        /// muda's index of the item among its children is still its index in the NSMenu.
        fn set_key(&self, key: Option<KeyAccelerator>) -> muda::Result<()> {
            let none = key.is_none();
            self.item.set_key_accelerator(key)?;
            if none {
                self.menu.remove(self.item.as_item())?;
                self.menu.insert(self.item.as_item(), self.position)?;
            }
            Ok(())
        }
    }

    fn predefined(system: System, words: &str) -> PredefinedMenuItem {
        let words = Some(words);
        match system {
            System::Services => PredefinedMenuItem::services(words),
            System::Hide => PredefinedMenuItem::hide(words),
            System::HideOthers => PredefinedMenuItem::hide_others(words),
            System::ShowAll => PredefinedMenuItem::show_all(words),
            System::Minimise => PredefinedMenuItem::minimize(words),
            System::Zoom => PredefinedMenuItem::zoom(words),
            System::BringAllToFront => PredefinedMenuItem::bring_all_to_front(words),
        }
    }

    /// `key` as an AppKit key equivalent: the logical key, which AppKit matches as the
    /// registry does, with Cmd for the command key and Control for a Mac's Control key.
    /// None for a key muda cannot express, or gets wrong, which then stays with the
    /// registry alone and its item shows none (`docs/gui.md`, Who owns the keys): the
    /// clipboard's and the browser's keys, and Forward Delete, which muda gives the Delete
    /// key's character.
    pub fn key_equivalent(key: Shortcut) -> Option<KeyAccelerator> {
        let m = key.modifiers;
        let mut held = Held::empty();
        for (on, modifier) in [
            (m.command || m.mac_cmd, Held::META),
            (m.ctrl, Held::CONTROL),
            (m.alt, Held::ALT),
            (m.shift, Held::SHIFT),
        ] {
            if on {
                held |= modifier;
            }
        }
        let character = |c: &str| Some(Logical::Character(c.to_owned()));
        let named = |n| Some(Logical::Named(n));
        let logical = match key.key {
            Key::ArrowDown => named(NamedKey::ArrowDown),
            Key::ArrowLeft => named(NamedKey::ArrowLeft),
            Key::ArrowRight => named(NamedKey::ArrowRight),
            Key::ArrowUp => named(NamedKey::ArrowUp),
            Key::Escape => named(NamedKey::Escape),
            Key::Tab => named(NamedKey::Tab),
            // The Mac's Delete key, which AppKit's menus write ⌫.
            Key::Backspace => named(NamedKey::Backspace),
            Key::Enter => named(NamedKey::Enter),
            Key::Insert => named(NamedKey::Insert),
            Key::Home => named(NamedKey::Home),
            Key::End => named(NamedKey::End),
            Key::PageUp => named(NamedKey::PageUp),
            Key::PageDown => named(NamedKey::PageDown),
            Key::Space => character(" "),
            Key::Colon => character(":"),
            Key::Comma => character(","),
            Key::Backslash => character("\\"),
            Key::Slash => character("/"),
            Key::Pipe => character("|"),
            Key::Questionmark => character("?"),
            Key::Exclamationmark => character("!"),
            Key::OpenBracket => character("["),
            Key::CloseBracket => character("]"),
            Key::OpenCurlyBracket => character("{"),
            Key::CloseCurlyBracket => character("}"),
            Key::Backtick => character("`"),
            Key::Minus => character("-"),
            Key::Period => character("."),
            Key::Plus => character("+"),
            Key::Equals => character("="),
            Key::Semicolon => character(";"),
            Key::Quote => character("'"),
            key => {
                let name = key.name();
                let first = name.chars().next()?;
                if name.len() == 1 && (first.is_ascii_uppercase() || first.is_ascii_digit()) {
                    character(&name.to_ascii_lowercase())
                } else if first == 'F' && name[1..].bytes().all(|b| b.is_ascii_digit()) {
                    NamedKey::from_str(name).ok().map(Logical::Named)
                } else {
                    None
                }
            }
        }?;
        Some(KeyAccelerator::new(held, logical))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use apertrust_ui::{Locale, Translator};
    use eframe::egui::{Key, Modifiers};

    use super::*;
    use crate::actions::{ACTIONS, Trigger, View};

    const PLATFORMS: [OperatingSystem; 3] = [
        OperatingSystem::Mac,
        OperatingSystem::Windows,
        OperatingSystem::Nix,
    ];

    /// The actions the registry declares that no menu holds, on purpose (`docs/gui.md`, The
    /// menu bar, Rules): the Edit view's, since the layouts' items join only once there are
    /// two layouts; F6's, which move the focus as Tab does; and H's peek, held, while a menu
    /// item is a press; the ☰'s menu's own, which opens the menu it would be in; and
    /// Restart, Settings' Restart now's for now (decision 82); and the eighth to the
    /// twelfth tab's, which only a workspace of more tabs than the Default's has, until Tools
    /// Panel Tab lists the tabs a workspace shows (decision 96).
    const LEFT_OUT: [ActionId; 33] = [
        ActionId::EDIT_VIEW,
        ActionId::NEXT_REGION,
        ActionId::PREVIOUS_REGION,
        ActionId::PEEK,
        ActionId::MENU,
        ActionId::RESTART,
        // History's ⋯ items, which its ⋯ holds alone (decision 93).
        ActionId::COMPRESS_HISTORY,
        ActionId::TRUNCATE_HISTORY,
        ActionId::CREATE_STYLE,
        ActionId::PANEL_TABS[7],
        ActionId::PANEL_TABS[8],
        ActionId::PANEL_TABS[9],
        ActionId::PANEL_TABS[10],
        ActionId::PANEL_TABS[11],
        // The Tools panel's ⋮ holds them, and the tab bar's menu will (decision 96).
        ActionId::FIND_TOOL,
        ActionId::SHOW_HIDDEN_TOOLS,
        // A section's layout, in its ⋯ and on its header's keys and screen readers'
        // actions, acting on that section (decisions 83 and 96).
        ActionId::SECTION_PIN,
        ActionId::SECTION_MOVE_UP,
        ActionId::SECTION_MOVE_DOWN,
        ActionId::SECTION_MOVE_TO_TAB[0],
        ActionId::SECTION_MOVE_TO_TAB[1],
        ActionId::SECTION_MOVE_TO_TAB[2],
        ActionId::SECTION_MOVE_TO_TAB[3],
        ActionId::SECTION_MOVE_TO_TAB[4],
        ActionId::SECTION_MOVE_TO_TAB[5],
        ActionId::SECTION_MOVE_TO_TAB[6],
        ActionId::SECTION_MOVE_TO_TAB[7],
        ActionId::SECTION_MOVE_TO_TAB[8],
        ActionId::SECTION_MOVE_TO_TAB[9],
        ActionId::SECTION_MOVE_TO_TAB[10],
        ActionId::SECTION_MOVE_TO_TAB[11],
        ActionId::SECTION_HIDE,
        ActionId::SECTION_SHOW,
    ];

    /// The actions only the Mac's menus hold: Close Window, since Windows and Linux close
    /// the window with its caption's close button (`docs/gui.md`, Windows and Linux).
    const MAC_ONLY: [ActionId; 1] = [ActionId::CLOSE_WINDOW];

    /// The actions `menu` holds on `os`, its submenus' included, in order.
    fn placed_in(menu: &'static Menu, os: OperatingSystem, ids: &mut Vec<ActionId>) {
        for entry in groups(menu, os).into_iter().flatten() {
            match entry {
                Entry::Action(id, _) => ids.push(*id),
                Entry::Submenu(inner) => placed_in(inner, os, ids),
                Entry::System(_) => {}
                // Every place, whatever the recents hold.
                Entry::Recents => ids.extend(ActionId::OPEN_RECENT),
                // The built-in workspaces', there whatever files a person writes.
                Entry::Workspaces => {
                    ids.extend(crate::layout::builtin_ids().map(actions::workspace));
                }
            }
        }
    }

    fn placed(os: OperatingSystem) -> Vec<ActionId> {
        let mut ids = Vec::new();
        for menu in menus(os) {
            placed_in(menu, os, &mut ids);
        }
        ids
    }

    /// Every menu and submenu, wherever it is there.
    fn every_menu() -> Vec<&'static Menu> {
        fn walk(menu: &'static Menu, found: &mut Vec<&'static Menu>) {
            found.push(menu);
            for entry in menu.groups.iter().flat_map(|g| g.iter()) {
                if let Entry::Submenu(inner) = entry {
                    walk(inner, found);
                }
            }
        }
        let mut found = Vec::new();
        for menu in &MENUS {
            walk(menu, &mut found);
        }
        found
    }

    /// A state in which everything built can run.
    fn everything() -> State {
        State {
            view: View::Edit,
            photo: true,
            waiting: false,
            editable: true,
            view_only: false,
            edited: true,
            undo: true,
            redo: true,
            compress: true,
            truncate: true,
            discard: true,
            unreadable: false,
            discard_why: None,
            exportable: true,
            text: false,
            photos: 2,
            opened: true,
            recents: [Some(true); crate::recents::MOST],
            tabs: actions::first_tabs(5),
            hidden_tab: Some(false),
            show_hidden: false,
            scale: 1.5,
            dialog: false,
            modal: false,
            dialog_window: false,
            full_screen: false,
            panel_tab: 0,
            workspace: actions::workspace(crate::layout::DEFAULT),
            workspace_given: false,
            show_hidden_given: false,
            section: None,
            browser: crate::settings::BrowserSettings::default(),
            shown: true,
            background: Some(apertrust_ui::Background::Dark),
        }
    }

    #[test]
    fn every_item_is_a_declared_action_placed_once_and_never_held() {
        for os in PLATFORMS {
            let ids = placed(os);
            let unique: HashSet<ActionId> = ids.iter().copied().collect();
            assert_eq!(unique.len(), ids.len(), "{os:?}: placed twice in {ids:?}");
            for id in ids {
                let action = actions::action(id).unwrap_or_else(|| panic!("{} undeclared", id.0));
                assert_ne!(action.trigger, Trigger::Hold, "{os:?}: {} is held", id.0);
            }
        }
    }

    #[test]
    fn every_action_is_placed_or_left_out_on_purpose() {
        for os in PLATFORMS {
            let ids = placed(os);
            for action in &ACTIONS {
                let placed = ids.contains(&action.id);
                let off_mac = os != OperatingSystem::Mac && MAC_ONLY.contains(&action.id);
                let left_out = LEFT_OUT.contains(&action.id) || off_mac;
                assert!(placed != left_out, "{os:?}: {}", action.id.0);
            }
        }
        for id in LEFT_OUT {
            assert!(actions::action(id).is_some(), "{} undeclared", id.0);
        }
    }

    #[test]
    fn settings_and_quit_sit_where_each_platform_puts_them() {
        let menu_of = |os, id| {
            menus(os)
                .find(|menu| {
                    let mut ids = Vec::new();
                    placed_in(menu, os, &mut ids);
                    ids.contains(&id)
                })
                .map(|menu| menu.title)
        };
        let mac = OperatingSystem::Mac;
        assert_eq!(menu_of(mac, ActionId::ABOUT), Some("app-name"));
        assert_eq!(menu_of(mac, ActionId::SETTINGS), Some("app-name"));
        assert_eq!(menu_of(mac, ActionId::QUIT), Some("app-name"));
        assert_eq!(menu_of(mac, ActionId::CLOSE_WINDOW), Some("menu-file"));
        for os in [OperatingSystem::Windows, OperatingSystem::Nix] {
            assert_eq!(menu_of(os, ActionId::ABOUT), Some("menu-help"));
            assert_eq!(menu_of(os, ActionId::SETTINGS), Some("menu-edit"));
            assert_eq!(menu_of(os, ActionId::QUIT), Some("menu-file"));
        }
    }

    /// Help › Show Log on every platform, first, and enabled whether or not a log is kept:
    /// the window says why it is empty.
    #[test]
    fn show_log_opens_help_on_every_platform() {
        for os in PLATFORMS {
            let help = menus(os)
                .find(|menu| menu.title == "menu-help")
                .expect("a Help menu");
            let mut ids = Vec::new();
            placed_in(help, os, &mut ids);
            assert_eq!(ids.first(), Some(&ActionId::SHOW_LOG), "{os:?}");
            let keymap = Keymap::defaults(os);
            let item = shown(ActionId::SHOW_LOG, &keymap, &everything(), Focus::None);
            assert!(item.enabled, "{os:?}");
        }
        let action = actions::action(ActionId::SHOW_LOG).expect("declared");
        assert_eq!(
            action.emits,
            actions::Emit::Intent(actions::Intent::ShowLog)
        );
        let words = apertrust_ui::Translator::new(apertrust_ui::Locale::EnGb);
        assert_eq!(words.text(action.message), "Show Log");
    }

    /// Help › Show Log stays enabled under a modal dialog on every platform, as the log
    /// window opens beside any dialog: under a sheet or an alert, the log is in reach.
    #[test]
    fn show_log_stays_enabled_under_a_modal_dialog() {
        for os in PLATFORMS {
            let keymap = Keymap::defaults(os);
            let mut state = everything();
            state.dialog = true;
            state.modal = true;
            assert!(
                actions::action(ActionId::SHOW_LOG)
                    .expect("declared")
                    .enabled(&state),
                "{os:?}"
            );
            let holding: Vec<_> = menus(os)
                .filter(|menu| {
                    let mut ids = Vec::new();
                    placed_in(menu, os, &mut ids);
                    ids.contains(&ActionId::SHOW_LOG)
                })
                .map(|menu| menu.title)
                .collect();
            assert_eq!(holding, ["menu-help"], "{os:?}");
            let item = shown(ActionId::SHOW_LOG, &keymap, &state, Focus::None);
            assert!(item.enabled, "{os:?}");
        }
    }

    /// View › Jobs on every platform, never in the Mac's Window menu, with Ctrl+Shift+J as
    /// each platform spells it, and enabled whatever is open, as the jobs window opens
    /// beside any dialog.
    #[test]
    fn view_jobs_is_in_view_on_every_platform_with_its_key_and_always_enabled() {
        for os in PLATFORMS {
            let holding: Vec<_> = menus(os)
                .filter(|menu| {
                    let mut ids = Vec::new();
                    placed_in(menu, os, &mut ids);
                    ids.contains(&ActionId::SHOW_JOBS)
                })
                .map(|m| m.title)
                .collect();
            assert_eq!(holding, ["menu-view"], "{os:?}");
            let keymap = Keymap::defaults(os);
            let mut state = everything();
            state.dialog = true;
            state.modal = true;
            state.dialog_window = true;
            let item = shown(ActionId::SHOW_JOBS, &keymap, &state, Focus::None);
            assert!(item.enabled, "{os:?}");
            let words = Translator::new(Locale::EnGb);
            let key = item.key.map(|k| k.spell(os, &words));
            let spelt = if os == OperatingSystem::Mac {
                "Shift+Cmd+J"
            } else {
                "Ctrl+Shift+J"
            };
            assert_eq!(key.as_deref(), Some(spelt), "{os:?}");
        }
        let action = actions::action(ActionId::SHOW_JOBS).expect("declared");
        assert_eq!(
            action.emits,
            actions::Emit::Intent(actions::Intent::ShowJobs)
        );
        // In the ☰'s menu, under View, with its words in every language.
        for locale in Locale::ALL {
            let menu = drawn_menu(locale, &everything());
            let view = tops(&menu)
                .into_iter()
                .find(|s| s.id == "menu-view")
                .expect("View");
            let item = items(&view.entries)
                .into_iter()
                .find(|item| item.id == ActionId::SHOW_JOBS.0)
                .unwrap_or_else(|| panic!("{}: not under View", locale.id()));
            let words = Translator::new(locale);
            assert_eq!(item.text, words.text("action-show-jobs"));
            assert!(item.enabled);
            assert_eq!(item.shortcut.as_deref(), Some("Ctrl+Shift+J"));
        }
        let en = Translator::new(Locale::EnGb);
        assert_eq!(en.text(action.message), "Jobs");
    }

    #[test]
    fn export_again_follows_export_in_file_on_every_platform() {
        for os in PLATFORMS {
            let file = menus(os)
                .find(|menu| menu.title == "menu-file")
                .expect("File is there");
            let mut ids = Vec::new();
            placed_in(file, os, &mut ids);
            let export = ids.iter().position(|id| *id == ActionId::EXPORT);
            let again = ids.iter().position(|id| *id == ActionId::EXPORT_AGAIN);
            assert_eq!(again, export.map(|at| at + 1), "{os:?}");
            // In Export's group, with no line between them.
            let group: Vec<ActionId> = groups(file, os)
                .into_iter()
                .map(|group| {
                    group
                        .into_iter()
                        .filter_map(|entry| match entry {
                            Entry::Action(id, _) => Some(*id),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                })
                .find(|group| group.contains(&ActionId::EXPORT))
                .expect("Export's group");
            assert!(group.contains(&ActionId::EXPORT_AGAIN), "{os:?}");
        }
    }

    #[test]
    fn open_comes_first_in_file_on_every_platform() {
        for os in PLATFORMS {
            let file = menus(os)
                .find(|menu| menu.title == "menu-file")
                .expect("File is there");
            let mut ids = Vec::new();
            placed_in(file, os, &mut ids);
            assert_eq!(ids.first(), Some(&ActionId::OPEN), "{os:?}");
            // A group of its own, Open Folder… and Open Recent after it.
            let first: Vec<_> = groups(file, os)[0]
                .iter()
                .map(|entry| match entry {
                    Entry::Action(id, _) => id.0,
                    Entry::Submenu(menu) => menu.title,
                    _ => panic!("an action or a submenu"),
                })
                .collect();
            assert_eq!(first, ["open", "open-folder", "menu-open-recent"], "{os:?}");
            // Close starts File's last group, Close Window or Quit after it.
            let last = groups(file, os).pop().expect("groups");
            assert!(
                matches!(last[0], Entry::Action(ActionId::CLOSE, _)),
                "{os:?}"
            );
            assert_eq!(last.len(), 2, "{os:?}");
        }
    }

    fn recent(words: &str, tip: &str) -> Recent {
        Recent {
            words: words.to_owned(),
            tip: tip.to_owned(),
            path: std::path::PathBuf::from(tip.lines().last().unwrap_or_default()),
        }
    }

    /// Open Recent's items as `os` lists them, with `recents` and `state`.
    fn open_recent(os: OperatingSystem, state: &State, recents: &[Recent]) -> Vec<Listed> {
        let tr = Translator::new(Locale::EnGb);
        let keymap = Keymap::defaults(os);
        let lists = Lists {
            recents,
            workspaces: &[],
        };
        listed(&OPEN_RECENT, os, &keymap, state, lists, &tr, Focus::None)
    }

    /// Open Recent the same on every platform: each recent's words, newest first, a missing
    /// one disabled, then a line and Clear Menu; with none, Clear Menu alone and disabled.
    #[test]
    fn open_recent_lists_the_recents_then_clear_menu_on_every_platform() {
        let mut state = everything();
        state.recents = [None; crate::recents::MOST];
        state.recents[0] = Some(true);
        state.recents[1] = Some(false);
        let recents = [
            recent("Wedding", "/photos/Wedding"),
            recent("Gone", "This folder is no longer there.\n/photos/Gone"),
        ];
        for os in PLATFORMS {
            let shown = |enabled| Shown { enabled, key: None };
            let item = |id: ActionId, words: &str, tip: Option<&str>, enabled| Listed::Item {
                id,
                words: words.to_owned(),
                tip: tip.map(str::to_owned),
                shown: shown(enabled),
                checked: None,
            };
            assert_eq!(
                open_recent(os, &state, &recents),
                [
                    item(
                        ActionId::OPEN_RECENT[0],
                        "Wedding",
                        Some("/photos/Wedding"),
                        true
                    ),
                    item(
                        ActionId::OPEN_RECENT[1],
                        "Gone",
                        Some(&recents[1].tip),
                        false
                    ),
                    Listed::Separator,
                    item(ActionId::CLEAR_RECENT, "Clear menu", None, true),
                ],
                "{os:?}"
            );
            state.recents = [None; crate::recents::MOST];
            assert_eq!(
                open_recent(os, &state, &[]),
                [item(ActionId::CLEAR_RECENT, "Clear menu", None, false)],
                "{os:?}"
            );
            state.recents[0] = Some(true);
            state.recents[1] = Some(false);
        }
    }

    /// View › Workspace the same on every platform, after Tools Panel Tab: the workspaces the
    /// app hands in, each its action, `workspace-<id>`, with its words, the one shown
    /// checked, as the Mac's bar fills it and the ☰ draws it.
    #[test]
    fn view_workspace_lists_the_workspaces_the_one_shown_checked() {
        let travel = actions::workspace("travel");
        let mut workspaces = builtin_workspaces();
        workspaces.push(WorkspaceItem {
            id: travel,
            words: "Travel".to_owned(),
            tip: None,
        });
        let mut state = everything();
        state.workspace = travel;
        let lists = Lists {
            recents: &[],
            workspaces: &workspaces,
        };
        let tr = Translator::new(Locale::EnGb);
        for os in PLATFORMS {
            let view = menus(os).find(|m| m.title == "menu-view").expect("View");
            let submenus: Vec<&str> = groups(view, os)
                .into_iter()
                .flatten()
                .filter_map(|entry| match entry {
                    Entry::Submenu(menu) => Some(menu.title),
                    _ => None,
                })
                .collect();
            let at = submenus.iter().position(|t| *t == "menu-tools-panel-tab");
            assert_eq!(submenus.get(at.unwrap() + 1), Some(&"menu-workspace"));
            let keymap = Keymap::defaults(os);
            let shown = listed(&WORKSPACE, os, &keymap, &state, lists, &tr, Focus::None);
            let items: Vec<(&str, &str, bool, Option<bool>)> = shown
                .iter()
                .map(|listed| match listed {
                    Listed::Item {
                        id,
                        words,
                        shown,
                        checked,
                        ..
                    } => (id.0, words.as_str(), shown.enabled, *checked),
                    Listed::Separator => panic!("a line"),
                })
                .collect();
            assert_eq!(
                items,
                [
                    ("workspace-default", "Default", true, Some(false)),
                    ("workspace-essentials", "Essentials", true, Some(false)),
                    ("workspace-travel", "Travel", true, Some(true)),
                ],
                "{os:?}"
            );
            // Under a modal dialog, none can be chosen.
            state.modal = true;
            let shown = listed(&WORKSPACE, os, &keymap, &state, lists, &tr, Focus::None);
            assert!(shown.iter().all(|listed| matches!(
                listed,
                Listed::Item { shown, .. } if !shown.enabled
            )));
            state.modal = false;
        }
        let ctx = egui::Context::default();
        ctx.set_os(OperatingSystem::Nix);
        apertrust_ui::set_locale(&ctx, Locale::EnGb);
        let keymap = Keymap::defaults(OperatingSystem::Nix);
        let menu = request(
            &ctx,
            &keymap,
            &state,
            lists,
            &Steps::default(),
            Rect::NOTHING,
        );
        let drawn = items(&menu.entries);
        let travel = drawn
            .iter()
            .find(|i| i.id == "workspace-travel")
            .expect("Travel");
        assert_eq!(
            (travel.text.as_str(), travel.checked),
            ("Travel", Some(true))
        );
        assert_eq!(
            chosen("workspace-travel"),
            Some(actions::workspace("travel"))
        );
    }

    /// The ☰'s Open Recent: the recents' words with their paths in the tips, a missing one
    /// disabled, each chosen by its place's action.
    #[test]
    fn the_drawn_open_recent_holds_the_recents_with_their_paths() {
        let mut state = everything();
        state.recents = [None; crate::recents::MOST];
        state.recents[0] = Some(true);
        state.recents[1] = Some(false);
        let recents = [
            recent("Wedding", "/photos/Wedding"),
            recent("Selects — Lisbon", "/photos/Lisbon/Selects"),
        ];
        let ctx = egui::Context::default();
        ctx.set_os(OperatingSystem::Nix);
        apertrust_ui::set_locale(&ctx, Locale::EnGb);
        let keymap = Keymap::defaults(OperatingSystem::Nix);
        let steps = Steps::default();
        let lists = Lists {
            recents: &recents,
            workspaces: &[],
        };
        let menu = request(&ctx, &keymap, &state, lists, &steps, Rect::NOTHING);
        let file = tops(&menu)[0];
        let submenu = file
            .entries
            .iter()
            .find_map(|e| match e {
                popup::Entry::Submenu(s) if s.id == "menu-open-recent" => Some(s),
                _ => None,
            })
            .expect("Open Recent in File");
        assert_eq!(submenu.text, "Open Recent");
        assert_eq!(
            submenu.entries,
            [
                popup::Item::new("open-recent-1", "Wedding")
                    .tip("/photos/Wedding")
                    .into(),
                popup::Item::new("open-recent-2", "Selects — Lisbon")
                    .tip("/photos/Lisbon/Selects")
                    .enabled(false)
                    .into(),
                popup::Entry::Separator,
                popup::Item::new("clear-recent", "Clear menu").into(),
            ]
        );
        assert_eq!(chosen("open-recent-2"), Some(ActionId::OPEN_RECENT[1]));
        assert_eq!(chosen("clear-recent"), Some(ActionId::CLEAR_RECENT));
        // Close, with Linux's Ctrl+W, before Quit.
        let close = file
            .entries
            .iter()
            .find_map(|e| match e {
                popup::Entry::Item(item) if item.id == "close" => Some(item),
                _ => None,
            })
            .expect("Close in File");
        assert_eq!(close.text, "Close");
        assert_eq!(close.shortcut.as_deref(), Some("Ctrl+W"));
        assert!(close.enabled);
    }

    #[test]
    fn a_menu_with_nothing_to_hold_is_left_out_but_the_mac_s_window_and_help() {
        let titles = |os| menus(os).map(|m| m.title).collect::<Vec<_>>();
        assert_eq!(
            titles(OperatingSystem::Mac),
            [
                "app-name",
                "menu-file",
                "menu-edit",
                "menu-photo",
                "menu-view",
                "menu-window",
                "menu-help"
            ]
        );
        for os in [OperatingSystem::Windows, OperatingSystem::Nix] {
            assert_eq!(
                titles(os),
                [
                    "menu-file",
                    "menu-edit",
                    "menu-photo",
                    "menu-view",
                    "menu-help"
                ]
            );
        }
        // No group is left empty, nor two lines in a row.
        for os in PLATFORMS {
            for menu in every_menu() {
                assert!(groups(menu, os).iter().all(|g| !g.is_empty()));
            }
        }
    }

    #[test]
    fn every_title_and_system_item_has_words_in_every_language() {
        let mut ids: Vec<&str> = every_menu().iter().map(|m| m.title).collect();
        for menu in every_menu() {
            for entry in menu.groups.iter().flat_map(|g| g.iter()) {
                if let Entry::System(system) = entry {
                    ids.push(system.message());
                }
            }
        }
        for locale in [Locale::EnGb, Locale::EnUs, Locale::Uk, Locale::Pl] {
            let tr = Translator::new(locale);
            for id in &ids {
                assert!(tr.has(id), "{} lacks {id}", locale.id());
            }
        }
        // The app's name is never translated.
        assert_eq!(
            Translator::new(Locale::Uk).text("menu-hide-app"),
            "Приховати Apertrust"
        );
    }

    #[test]
    fn an_item_holds_no_key_the_focused_widget_keeps() {
        let state = everything();
        let mac = OperatingSystem::Mac;
        let keymap = Keymap::defaults(mac);
        let key = |id, focus| shown(id, &keymap, &state, focus).key;
        let command = |k| Shortcut {
            modifiers: Modifiers::COMMAND,
            key: k,
        };
        // A value field keeps every plain key, but not Cmd+, nor Cmd+Q; Undo and Redo keep
        // Cmd+Z and Shift+Cmd+Z, which act on the field (decision 39).
        assert_eq!(key(ActionId::UNDO, Focus::None), Some(command(Key::Z)));
        assert_eq!(key(ActionId::UNDO, Focus::Text), Some(command(Key::Z)));
        let shift = Shortcut {
            modifiers: Modifiers::COMMAND.plus(Modifiers::SHIFT),
            key: Key::Z,
        };
        assert_eq!(key(ActionId::REDO, Focus::Text), Some(shift));
        assert_eq!(key(ActionId::SHOW_COLLECTIONS, Focus::Text), None);
        assert_eq!(key(ActionId::NEXT_PHOTO, Focus::Text), None);
        assert_eq!(
            key(ActionId::SETTINGS, Focus::Text),
            Some(command(Key::Comma))
        );
        assert_eq!(key(ActionId::QUIT, Focus::Text), Some(command(Key::Q)));
        // A slider keeps Space, the next photo's, but not Cmd+Z.
        assert_eq!(key(ActionId::NEXT_PHOTO, Focus::Control), None);
        assert_eq!(key(ActionId::UNDO, Focus::Control), Some(command(Key::Z)));
        // A dialog's own window with the focus: Undo and Redo hold none, so its own field
        // takes Cmd+Z, and the rest are as they were.
        let in_dialog = State {
            dialog_window: true,
            ..everything()
        };
        assert_eq!(
            shown(ActionId::UNDO, &keymap, &in_dialog, Focus::Text).key,
            None
        );
        assert_eq!(
            shown(ActionId::REDO, &keymap, &in_dialog, Focus::None).key,
            None
        );
        assert_eq!(
            shown(ActionId::QUIT, &keymap, &in_dialog, Focus::None).key,
            Some(command(Key::Q))
        );
        // Whatever the focus, an item holds exactly the action's first key that it does
        // not keep, but the editing items, which keep theirs.
        for os in PLATFORMS {
            let keymap = Keymap::defaults(os);
            for id in placed(os) {
                for focus in [Focus::None, Focus::Control, Focus::Text] {
                    let first = keymap.keys(id).first().copied();
                    let expected = if edits_the_field(id) {
                        first
                    } else {
                        first.filter(|k| !focus.keeps(k.key, k.modifiers))
                    };
                    assert_eq!(shown(id, &keymap, &state, focus).key, expected, "{}", id.0);
                }
            }
        }
    }

    #[test]
    fn an_item_is_enabled_exactly_when_its_action_can_run_where_it_is() {
        let keymap = Keymap::defaults(OperatingSystem::Mac);
        let mut state = everything();
        let enabled = |id, state: &State| shown(id, &keymap, state, Focus::None).enabled;
        assert!(enabled(ActionId::RESET, &state));
        assert!(enabled(ActionId::SETTINGS, &state));
        // Declared, not built: drawn disabled.
        assert!(!enabled(ActionId::COPY_EDIT, &state));
        assert!(!enabled(ActionId::NEXT_PHOTO, &state));
        state.edited = false;
        assert!(!enabled(ActionId::RESET, &state));
        // Undo and Redo, from the photo's history.
        assert!(enabled(ActionId::UNDO, &state) && enabled(ActionId::REDO, &state));
        state.undo = false;
        assert!(!enabled(ActionId::UNDO, &state) && enabled(ActionId::REDO, &state));
        // Discard History…, from the photo's history, resting under a modal dialog.
        assert!(enabled(ActionId::DISCARD_HISTORY, &state));
        state.modal = true;
        assert!(!enabled(ActionId::DISCARD_HISTORY, &state));
        state.modal = false;
        state.discard = false;
        assert!(!enabled(ActionId::DISCARD_HISTORY, &state));
        // Where it would ask and the platform shows no alert, the drawn item says why.
        state.discard_why = Some("history-discard-no-alert");
        let tip = |state: &State| {
            let menu = drawn_menu_naming(Locale::EnGb, state, &Steps::default());
            let edit = tops(&menu)[1];
            (edit.entries.iter())
                .find_map(|e| match e {
                    popup::Entry::Item(item) if item.id == ActionId::DISCARD_HISTORY.0 => {
                        Some((item.enabled, item.tip.clone()))
                    }
                    _ => None,
                })
                .expect("in Edit")
        };
        let (enabled_now, why) = tip(&state);
        assert!(!enabled_now);
        assert!(why.is_some_and(|why| why.contains("can’t be shown here")));
        state.discard_why = None;
        assert_eq!(tip(&state).1, None);
        for os in PLATFORMS {
            for id in placed(os) {
                let action = actions::action(id).expect("declared");
                assert_eq!(enabled(id, &state), action.enabled(&state), "{}", id.0);
            }
        }
    }

    /// The words of `id`'s item in `menu`'s Edit.
    fn edit_item(menu: &popup::Request, id: ActionId) -> (String, bool) {
        let edit = tops(menu)[1];
        assert_eq!(edit.id, "menu-edit");
        edit.entries
            .iter()
            .find_map(|e| match e {
                popup::Entry::Item(item) if item.id == id.0 => {
                    Some((item.text.clone(), item.enabled))
                }
                _ => None,
            })
            .expect("in Edit")
    }

    #[test]
    fn undo_and_redo_name_the_step_unless_a_text_field_has_the_focus() {
        let mut state = everything();
        let steps = Steps {
            undo: Some("Exposure".to_owned()),
            redo: Some("Go to Step".to_owned()),
        };
        let item = |locale, state: &State, steps: &Steps, id| {
            edit_item(&drawn_menu_naming(locale, state, steps), id)
        };
        let named = |locale, state: &State, id| item(locale, state, &steps, id).0;
        assert_eq!(named(Locale::EnGb, &state, ActionId::UNDO), "Undo Exposure");
        assert_eq!(
            named(Locale::EnGb, &state, ActionId::REDO),
            "Redo Go to Step"
        );
        assert_eq!(
            named(Locale::Uk, &state, ActionId::UNDO),
            "Скасувати: Exposure"
        );
        assert_eq!(
            named(Locale::Pl, &state, ActionId::REDO),
            "Ponów: Go to Step"
        );
        // Nothing to name: Undo alone, and disabled with nothing to undo.
        state.undo = false;
        let nothing = Steps::default();
        let undo = item(Locale::EnGb, &state, &nothing, ActionId::UNDO);
        assert_eq!(undo, ("Undo".to_owned(), false));
        // A text field with the focus: its own undo, whatever the photo's history holds.
        state.text = true;
        assert_eq!(
            item(Locale::EnGb, &state, &steps, ActionId::UNDO),
            ("Undo".to_owned(), true)
        );
        assert_eq!(named(Locale::EnGb, &state, ActionId::REDO), "Redo");
        // The Mac's items take the same words.
        let tr = Translator::new(Locale::EnGb);
        let undo = actions::action(ActionId::UNDO).expect("declared");
        assert_eq!(words(undo, &steps, &state, &tr), "Undo");
        state.text = false;
        assert_eq!(words(undo, &steps, &state, &tr), "Undo Exposure");
        let reset = actions::action(ActionId::RESET).expect("declared");
        assert_eq!(words(reset, &steps, &state, &tr), "Reset");
        assert!(names_a_step(ActionId::REDO) && !names_a_step(ActionId::RESET));
    }

    #[test]
    fn a_key_held_down_runs_again_only_an_action_that_repeats() {
        // Control+Cmd+F held: full screen once, not on every repeat; Cmd+= held: larger
        // each time.
        assert!(runs(ActionId::FULL_SCREEN, false));
        assert!(!runs(ActionId::FULL_SCREEN, true));
        assert!(!runs(ActionId::QUIT, true));
        assert!(runs(ActionId::SCALE_UP, true));
        assert!(!runs(ActionId("no-such-action"), false));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn every_default_key_of_a_placed_item_is_a_key_equivalent() {
        // None is left without one on purpose yet.
        let os = OperatingSystem::Mac;
        let keymap = Keymap::defaults(os);
        for id in placed(os) {
            if let Some(key) = keymap.keys(id).first() {
                assert!(
                    mac::key_equivalent(*key).is_some(),
                    "{}: {}",
                    id.0,
                    key.text(os)
                );
            }
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn key_equivalents_carry_the_mac_s_modifiers_and_the_logical_key() {
        use muda::accelerator::{Key as Logical, Modifiers as Held};
        let equivalent = |modifiers, key| {
            mac::key_equivalent(Shortcut { modifiers, key })
                .map(|k| (k.modifiers(), k.key().clone()))
        };
        let character = |c: &str| Logical::Character(c.to_owned());
        assert_eq!(
            equivalent(Modifiers::COMMAND, Key::Comma),
            Some((Held::META, character(",")))
        );
        assert_eq!(
            equivalent(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::Z),
            Some((Held::META | Held::SHIFT, character("z")))
        );
        // Control+Cmd+F, full screen.
        assert_eq!(
            equivalent(Modifiers::CTRL.plus(Modifiers::MAC_CMD), Key::F),
            Some((Held::CONTROL | Held::META, character("f")))
        );
        assert_eq!(
            equivalent(Modifiers::COMMAND, Key::Num1),
            Some((Held::META, character("1")))
        );
        assert_eq!(
            equivalent(Modifiers::NONE, Key::Space),
            Some((Held::empty(), character(" ")))
        );
        assert!(equivalent(Modifiers::NONE, Key::F11).is_some());
        // What muda cannot express, or gets wrong, stays with the registry.
        assert_eq!(equivalent(Modifiers::NONE, Key::Delete), None);
        assert_eq!(equivalent(Modifiers::NONE, Key::Copy), None);
        assert_eq!(equivalent(Modifiers::NONE, Key::BrowserBack), None);
    }

    /// The ☰'s menu as Linux draws it, in `locale`, with `state`.
    fn drawn_menu(locale: Locale, state: &State) -> popup::Request {
        drawn_menu_naming(locale, state, &Steps::default())
    }

    /// The ☰'s menu as Linux draws it, in `locale`, with `state`, Undo and Redo naming
    /// `steps`.
    fn drawn_menu_naming(locale: Locale, state: &State, steps: &Steps) -> popup::Request {
        let ctx = egui::Context::default();
        ctx.set_os(OperatingSystem::Nix);
        apertrust_ui::set_locale(&ctx, locale);
        request(
            &ctx,
            &Keymap::defaults(OperatingSystem::Nix),
            state,
            Lists {
                recents: &[],
                workspaces: &builtin_workspaces(),
            },
            steps,
            Rect::NOTHING,
        )
    }

    /// View › Workspace's built-in items as the app hands them in, in British English.
    fn builtin_workspaces() -> Vec<WorkspaceItem> {
        let en = Translator::new(Locale::EnGb);
        crate::layout::builtin_ids()
            .map(|id| WorkspaceItem {
                id: actions::workspace(id),
                words: en.text(&format!("workspace-{id}")),
                tip: None,
            })
            .collect()
    }

    /// The submenus at the top of `menu`.
    fn tops(menu: &popup::Request) -> Vec<&popup::Submenu> {
        menu.entries
            .iter()
            .map(|entry| match entry {
                popup::Entry::Submenu(submenu) => submenu,
                other => panic!("not a menu at the top: {other:?}"),
            })
            .collect()
    }

    /// Every item in `entries`, its submenus' included.
    fn items(entries: &[popup::Entry]) -> Vec<&popup::Item> {
        entries
            .iter()
            .flat_map(|entry| match entry {
                popup::Entry::Item(item) => vec![item],
                popup::Entry::Submenu(submenu) => items(&submenu.entries),
                popup::Entry::Separator => Vec::new(),
            })
            .collect()
    }

    #[test]
    fn the_menu_on_linux_holds_the_menus_with_items_as_the_registry_has_them() {
        let mut state = everything();
        state.edited = false;
        state.full_screen = true;
        state.panel_tab = 2;
        let menu = drawn_menu(Locale::EnGb, &state);
        assert_eq!(menu.name, "Menu");
        let titles: Vec<(&str, &str)> = tops(&menu)
            .iter()
            .map(|s| (s.id.as_str(), s.text.as_str()))
            .collect();
        assert_eq!(
            titles,
            [
                ("menu-file", "&File"),
                ("menu-edit", "&Edit"),
                ("menu-photo", "&Photo"),
                ("menu-view", "&View"),
                ("menu-help", "&Help"),
            ]
        );
        // File: Open…, Import and Export, Quit, in groups that lines separate.
        let file = tops(&menu)[0];
        let separators = |s: &popup::Submenu| {
            s.entries
                .iter()
                .filter(|e| **e == popup::Entry::Separator)
                .count()
        };
        assert_eq!(separators(file), 2);
        assert_eq!(
            file.entries[0],
            popup::Item::new("open", "Open…").shortcut("Ctrl+O").into()
        );
        let all = items(&menu.entries);
        let find = |id: ActionId| {
            *all.iter()
                .find(|item| item.id == id.0)
                .unwrap_or_else(|| panic!("{} not in the menu", id.0))
        };
        // Built and able to run: enabled. Declared but not built, or unable to run: there,
        // disabled.
        assert!(find(ActionId::EXPORT).enabled);
        assert!(!find(ActionId::RESET).enabled);
        assert!(find(ActionId::SETTINGS).enabled);
        // The keys as Linux spells them, the first of each, whatever has the focus.
        assert_eq!(
            find(ActionId::NEXT_PHOTO).shortcut.as_deref(),
            Some("Space")
        );
        assert_eq!(
            find(ActionId::REDO).shortcut.as_deref(),
            Some("Ctrl+Shift+Z")
        );
        assert_eq!(find(ActionId::RESET).shortcut, None);
        // Check marks for what is on and the chosen one of a set; none for a command.
        assert_eq!(find(ActionId::FULL_SCREEN).checked, Some(true));
        assert_eq!(find(ActionId::PANEL_TABS[2]).checked, Some(true));
        assert_eq!(find(ActionId::PANEL_TABS[0]).checked, Some(false));
        assert_eq!(find(ActionId::QUIT).checked, None);
        // Every item is its action's, as the registry has it, and the menus' order; Open
        // Recent, given no recents, holds Clear Menu alone.
        let keymap = Keymap::defaults(OperatingSystem::Nix);
        let en = Translator::new(Locale::EnGb);
        let ids: Vec<&str> = all.iter().map(|item| item.id.as_str()).collect();
        let placed: Vec<&str> = placed(OperatingSystem::Nix)
            .iter()
            .filter(|id| !ActionId::OPEN_RECENT.contains(id))
            .map(|id| id.0)
            .collect();
        assert_eq!(ids, placed);
        for item in all {
            let id = chosen(&item.id).expect("an action's id");
            let action = actions::action(id).expect("declared");
            assert_eq!(item.text, en.text(action.message), "{}", id.0);
            assert_eq!(item.enabled, action.enabled(&state), "{}", id.0);
            let key = keymap
                .keys(id)
                .first()
                .map(|k| k.spell(OperatingSystem::Nix, &en));
            assert_eq!(item.shortcut, key, "{}", id.0);
            assert_eq!(item.checked, action.checked(&state), "{}", id.0);
        }
        // With a dialog open, the view's items rest.
        state.dialog = true;
        let menu = drawn_menu(Locale::EnGb, &state);
        let next = items(&menu.entries)
            .into_iter()
            .find(|item| item.id == ActionId::NEXT_PHOTO.0)
            .expect("there");
        assert!(!next.enabled);
    }

    #[test]
    fn each_top_level_menu_has_an_access_key_of_its_own_in_every_language() {
        let state = everything();
        // AltGr types these on a Polish keyboard, so Alt cannot.
        const ALTGR: &str = "ąćęłńóśźż";
        for locale in Locale::ALL {
            let menu = drawn_menu(locale, &state);
            let mut keys = HashSet::new();
            for submenu in tops(&menu) {
                let key = submenu
                    .access_key()
                    .unwrap_or_else(|| panic!("{}: {} has none", locale.id(), submenu.id));
                let key = key.to_lowercase().next().unwrap_or(key);
                assert!(keys.insert(key), "{}: {key} twice", locale.id());
                assert!(!ALTGR.contains(key), "{}: {key}", locale.id());
                assert_eq!(menu.access_key(key), Some(submenu.id.as_str()));
            }
            // The Mac's Window menu too, which Windows and Linux do not have.
            let tr = Translator::new(locale);
            let window = popup::Submenu::new("menu-window", tr.text("menu-window"), vec![]);
            let key = window.access_key().expect("Window has one");
            assert!(menu.access_key(key).is_none(), "{}: {key}", locale.id());
        }
        assert_eq!(
            drawn_menu(Locale::Uk, &state).access_key('ф'),
            Some("menu-file")
        );
        assert_eq!(
            drawn_menu(Locale::Pl, &state).access_key('M'),
            Some("menu-help")
        );
        // The file's word as a term has no `&`: only the menu's title carries it.
        let en = Translator::new(Locale::EnGb);
        assert_eq!(en.text("menu-file"), "&File");
    }

    #[test]
    fn a_choice_names_its_action_by_id() {
        assert_eq!(chosen("settings"), Some(ActionId::SETTINGS));
        assert_eq!(
            chosen(ActionId::PANEL_TABS[3].0),
            Some(ActionId::PANEL_TABS[3])
        );
        assert_eq!(chosen("no-such-action"), None);
    }

    use apertrust_tools::{
        Place as Open, ProcessingDefinition, Reset, ToolDefinition, utility::UtilityDefinition,
    };

    /// The item naming `action` at `place`, in the section `section` at `order`.
    const fn at(place: Open, section: u32, order: u32, action: &'static str) -> MenuItem {
        MenuItem {
            place,
            section: Section(section),
            order,
            entry: MenuEntry::Action(action),
        }
    }

    /// Every built-in tool's definition.
    fn definitions() -> Vec<&'static ProcessingDefinition> {
        let tools = apertrust_tools::TOOLS.iter();
        tools.map(|tool| tool.definition()).collect()
    }

    fn leak(items: &[MenuItem]) -> &'static [MenuItem] {
        Box::leak(items.to_vec().into_boxed_slice())
    }

    /// A tool with nothing but `id` and the items it adds to context menus.
    fn tool(id: &'static str, menus: &[MenuItem]) -> &'static ProcessingDefinition {
        let menus = leak(menus);
        Box::leak(Box::new(ProcessingDefinition {
            tool: ToolDefinition {
                id,
                tabs: &[],
                before: &[],
                after: &[],
                reset: Reset::Tool,
                presets: true,
                open_at_first: false,
                menus,
            },
            version: 1,
            layers: false,
            auto: false,
            switched: false,
            controls: &[],
            preset_rows: &[],
            views: &[],
            pickers: &[],
            facts: None,
        }))
    }

    /// Decision 91: every utility's items are within a tool's limits, each a registry
    /// action, and those of the photo's and a thumbnail's menus in the menu bar on every
    /// platform, so none is reached by a right-click alone; ⋯'s are reached in ⋯
    /// (decision 93). Every tool's names its action in the registry.
    #[test]
    fn every_utility_s_item_is_within_the_limits_and_in_the_menu_bar() {
        let in_menus = |action: &str| {
            chosen(action).is_some_and(|id| PLATFORMS.iter().all(|os| placed(*os).contains(&id)))
        };
        // What the check refuses: an action in no menu, one the registry lacks.
        assert!(in_menus(ActionId::RESET.0));
        assert!(!in_menus(ActionId::RESTART.0));
        assert!(!in_menus("no-such-action"));
        for utility in apertrust_tools::utility::UTILITIES {
            assert_eq!(
                apertrust_tools::check_menus(utility.tool.menus),
                Ok(()),
                "{}",
                utility.tool.id
            );
            for item in utility.tool.menus {
                for action in item.entry.actions() {
                    let id = utility.tool.id;
                    assert!(
                        chosen(action).is_some(),
                        "{id}: {action} is no registry action"
                    );
                    let reached = item.place == Open::Options || in_menus(action);
                    assert!(reached, "{id}: {action}");
                }
                // A tool's sections, History's Reset alone first in Edit's.
                let reset = (utility.tool.id, item.section, item.order, item.entry)
                    == ("history", Section::EDIT, 100, MenuEntry::Action("reset"));
                if !reset {
                    let placed = apertrust_tools::check_section(item);
                    assert_eq!(placed, Ok(()), "{}: {item:?}", utility.tool.id);
                }
            }
        }
        // History adds Reset alone to the photo's menu, nothing to a thumbnail's, and its
        // ⋯ holds its four.
        let history = apertrust_tools::utility::utility("history").unwrap();
        let at = |place| {
            (history.tool.menus.iter())
                .filter(|item| item.place == place)
                .map(|item| item.entry)
                .collect::<Vec<_>>()
        };
        assert_eq!(at(Open::Photo), [MenuEntry::Action(ActionId::RESET.0)]);
        assert_eq!(at(Open::Thumbnail), []);
        let options = [
            ActionId::COMPRESS_HISTORY,
            ActionId::TRUNCATE_HISTORY,
            ActionId::CREATE_STYLE,
            ActionId::DISCARD_HISTORY,
        ];
        assert_eq!(at(Open::Options), options.map(|id| MenuEntry::Action(id.0)));
        assert_eq!(history.tool.menus.len(), 5);
        for tool in definitions() {
            for action in tool.tool.menus.iter().flat_map(|item| item.entry.actions()) {
                let id = format!("tool-{}-{action}", tool.tool.id);
                assert!(chosen(&id).is_some(), "{id} is no registry action");
            }
        }
    }

    /// What tools and utilities add to a place: each item there with its section and
    /// order, the tools' as their registry actions; in ⋯ only its own tool's.
    #[test]
    fn tools_and_utilities_add_their_items_at_their_places() {
        let first = tool(
            "first",
            &[
                at(Open::Photo, 1000, 200, "auto"),
                at(Open::Options, 1000, 100, "auto"),
                at(Open::Photo, 5000, 100, "pick"),
            ],
        );
        let second = tool("second", &[at(Open::Thumbnail, 2000, 200, "level")]);
        let utilities = [UtilityDefinition {
            tool: ToolDefinition {
                id: "history",
                tabs: &[],
                before: &[],
                after: &[],
                reset: Reset::None,
                presets: false,
                open_at_first: false,
                menus: leak(&[
                    at(Open::Photo, 1000, 100, "reset"),
                    at(Open::Options, 4000, 100, "reset"),
                ]),
            },
            measures: None,
            titles: &[],
            open: &[],
        }];
        let tools = [first, tool("none", &[]), second];
        let ids = |place, target: Target| -> Vec<(u32, u32, String)> {
            let added = added(place, &target, &tools, &utilities).into_iter();
            added
                .map(|item| (item.section.0, item.order, item.actions.join(",")))
                .collect()
        };
        let id = |section, order, action: &str| (section, order, action.to_string());
        assert_eq!(
            ids(Open::Photo, Target::Photo { point: None }),
            [
                id(1000, 200, "tool-first-auto"),
                id(5000, 100, "tool-first-pick"),
                id(1000, 100, "reset"),
            ]
        );
        assert_eq!(
            ids(Open::Thumbnail, Target::Selection),
            [id(2000, 200, "tool-second-level")]
        );
        assert_eq!(
            ids(Open::Options, Target::Tool("first")),
            [id(1000, 100, "tool-first-auto")]
        );
        assert_eq!(
            ids(Open::Options, Target::Tool("history")),
            [id(4000, 100, "reset")]
        );
        assert!(ids(Open::Options, Target::Tool("second")).is_empty());
    }

    /// The menu lists the app's items and the added ones together by section, then order,
    /// a line wherever the section changes: a tool's item may sit between two of the
    /// app's sections, or before them all.
    #[test]
    fn a_menu_sorts_its_items_by_section_and_order_with_a_line_between_sections() {
        let ctx = egui::Context::default();
        ctx.set_os(OperatingSystem::Nix);
        apertrust_ui::set_locale(&ctx, Locale::EnGb);
        let added = |section, order, action: &str| Added {
            section: Section(section),
            order,
            title: None,
            actions: vec![action.into()],
        };
        let menu = built(
            Place::Open(Open::Photo),
            &Target::Photo { point: None },
            &[
                added(5000, 100, "undo"),
                added(1000, 200, "redo"),
                added(1000, 100, "reset"),
                added(2500, 100, "show-log"),
            ],
            &Keymap::defaults(OperatingSystem::Nix),
            &State {
                photos: 1,
                ..everything()
            },
            OperatingSystem::Nix,
            &translator(&ctx),
            Rect::NOTHING,
        );
        let ids: Vec<String> = lines(&menu).into_iter().map(|(id, ..)| id).collect();
        assert_eq!(
            ids,
            [
                "reset",
                "redo",
                "-",
                "export",
                "-",
                "show-log",
                "-",
                "menu-background",
                "-",
                "undo"
            ]
        );
    }

    /// Every item's section and order at a place, the app's and those `tools` and
    /// `utilities` add, that another item there has too.
    fn clashes(
        place: Open,
        target: Target,
        tools: &[&ProcessingDefinition],
        utilities: &[UtilityDefinition],
    ) -> Vec<(Section, u32)> {
        let app = (APP_ITEMS.iter())
            .filter(|item| item.place == place)
            .map(|item| (item.section, item.order));
        let added = added(place, &target, tools, utilities).into_iter();
        let mut keys: Vec<_> = app
            .chain(added.map(|item| (item.section, item.order)))
            .collect();
        keys.sort();
        let twice = keys.windows(2).filter(|pair| pair[0] == pair[1]);
        twice.map(|pair| pair[0]).collect()
    }

    /// Decision 91: no two items in one place share a section and an order, the app's and
    /// every tool's and utility's together, so their order is never left to chance; in ⋯
    /// each tool's own with the app's.
    #[test]
    fn no_two_items_in_a_place_share_a_section_and_an_order() {
        let tools = definitions();
        let utilities = apertrust_tools::utility::UTILITIES;
        let mut targets = vec![
            (Open::Photo, Target::Photo { point: None }),
            (Open::Thumbnail, Target::Selection),
        ];
        let ids = (tools.iter().map(|definition| definition.tool.id))
            .chain(utilities.iter().map(|u| u.tool.id));
        targets.extend(ids.map(|id| (Open::Options, Target::Tool(id))));
        for (place, target) in targets {
            let clashes = clashes(place, target, &tools, utilities);
            assert!(clashes.is_empty(), "{place:?} {target:?}: {clashes:?}");
        }
        // What the check refuses: a tool's item where the app's Export… is.
        let export = tool("export", &[at(Open::Photo, 2000, 100, "send")]);
        assert_eq!(
            clashes(Open::Photo, Target::Photo { point: None }, &[export], &[]),
            [(Section::EXPORT, 100)]
        );
    }

    /// `groups`, each a group of actions' ids, as the context menu at `place` on `target`
    /// shows them with `state`, on Linux in British English: each a section of its own,
    /// after the app's.
    fn context_menu(
        place: Open,
        target: Target,
        groups: &[&[&str]],
        state: &State,
    ) -> popup::Request {
        let ctx = egui::Context::default();
        ctx.set_os(OperatingSystem::Nix);
        apertrust_ui::set_locale(&ctx, Locale::EnGb);
        let mut added = Vec::new();
        for (section, group) in (5000..).step_by(1000).zip(groups) {
            for (order, id) in (100..).step_by(100).zip(group.iter()) {
                added.push(Added {
                    section: Section(section),
                    order,
                    title: None,
                    actions: vec![id.to_string()],
                });
            }
        }
        built(
            Place::Open(place),
            &target,
            &added,
            &Keymap::defaults(OperatingSystem::Nix),
            state,
            OperatingSystem::Nix,
            &translator(&ctx),
            Rect::NOTHING,
        )
    }

    /// Each entry of `menu` as (id, enabled, key shown), a line as `-`.
    fn lines(menu: &popup::Request) -> Vec<(String, bool, bool)> {
        menu.entries
            .iter()
            .map(|entry| match entry {
                popup::Entry::Item(item) => {
                    (item.id.clone(), item.enabled, item.shortcut.is_some())
                }
                popup::Entry::Separator => ("-".into(), true, false),
                popup::Entry::Submenu(submenu) => (submenu.id.clone(), true, false),
            })
            .collect()
    }

    /// Decision 91: in the photo's and a thumbnail's menus, the app's items with them, an item
    /// that cannot run is hidden, and its group with it, unless the photo is not yet live,
    /// when it is shown disabled, but for one something else stops, its tip saying why
    /// (`actions::why_unavailable`; Discard History…'s missing alert stands in, though no
    /// tool adds it here); no item shows its key. ⋯ draws one disabled, with its tip, and
    /// keys.
    #[test]
    fn the_photo_s_menu_hides_what_cannot_run_shows_what_waits_disabled_and_no_keys() {
        let groups: &[&[&str]] = &[&["undo", "redo"], &["discard-history"]];
        let line = |id: &str, enabled| (id.to_string(), enabled, false);
        let sep = || line("-", true);
        let bg = || line("menu-background", true);
        let one = State {
            photos: 1,
            ..everything()
        };
        let cannot = State {
            edited: false,
            undo: false,
            redo: false,
            discard: false,
            discard_why: Some("history-discard-no-alert"),
            ..one
        };
        // While the photo just made active opens.
        let waiting = State {
            photo: false,
            waiting: true,
            ..cannot
        };
        let none = State {
            waiting: false,
            ..waiting
        };
        let photo = Target::Photo { point: None };
        let cases = [
            (
                Open::Photo,
                photo,
                &one,
                vec![
                    line("export", true),
                    sep(),
                    bg(),
                    sep(),
                    line("undo", true),
                    line("redo", true),
                    sep(),
                    line("discard-history", true),
                ],
            ),
            (
                Open::Photo,
                photo,
                &cannot,
                vec![line("export", true), sep(), bg()],
            ),
            (
                Open::Photo,
                photo,
                &waiting,
                vec![
                    line("export", false),
                    sep(),
                    bg(),
                    sep(),
                    line("undo", false),
                    line("redo", false),
                ],
            ),
            (Open::Photo, photo, &none, vec![bg()]),
            (
                Open::Thumbnail,
                Target::Selection,
                &one,
                vec![
                    line("export", true),
                    sep(),
                    line("undo", true),
                    line("redo", true),
                    sep(),
                    line("discard-history", true),
                ],
            ),
            (
                Open::Thumbnail,
                Target::Selection,
                &cannot,
                vec![line("export", true)],
            ),
            (
                Open::Thumbnail,
                Target::Selection,
                &waiting,
                vec![
                    line("export", false),
                    sep(),
                    line("undo", false),
                    line("redo", false),
                ],
            ),
            (Open::Thumbnail, Target::Selection, &none, vec![]),
        ];
        for (place, target, state, expected) in cases {
            let menu = context_menu(place, target, groups, state);
            assert_eq!(lines(&menu), expected, "{place:?} {state:?}");
        }
        let groups: &[&[&str]] = &[&["undo", "redo"], &["reset"], &["discard-history"]];
        let options = State {
            redo: false,
            discard: false,
            discard_why: Some("history-discard-no-alert"),
            ..everything()
        };
        let menu = context_menu(Open::Options, Target::Tool("history"), groups, &options);
        let keyed = |id: &str, enabled, key| (id.to_string(), enabled, key);
        assert_eq!(
            lines(&menu),
            [
                keyed("undo", true, true),
                keyed("redo", false, true),
                sep(),
                keyed("reset", true, false),
                sep(),
                keyed("discard-history", false, false),
            ]
        );
        let discard = items(&menu.entries)
            .into_iter()
            .find(|item| item.id == "discard-history")
            .unwrap();
        assert!(discard.tip.is_some());
        assert_eq!(menu.name, "History options");
    }

    /// The photo's and a thumbnail's menus as the app opens them (decision 91), by section:
    /// on the photo History's Reset in Edit's, the app's Export… in Export's and
    /// Background ▸ in View's; on a thumbnail Export… alone, Reset left out since its photo
    /// may not be the one open.
    #[test]
    fn the_photo_s_and_a_thumbnail_s_menus_hold_their_items_in_order() {
        let ctx = egui::Context::default();
        ctx.set_os(OperatingSystem::Nix);
        apertrust_ui::set_locale(&ctx, Locale::EnGb);
        let keymap = Keymap::defaults(OperatingSystem::Nix);
        let open = |place, target: Target, state: &State| {
            let place = Place::Open(place);
            context(&ctx, place, &target, &keymap, state, Rect::NOTHING)
        };
        let words = |menu: &popup::Request| -> Vec<String> {
            let line = |entry: &popup::Entry| match entry {
                popup::Entry::Item(item) => item.text.clone(),
                popup::Entry::Submenu(submenu) => format!("{} ▸", submenu.text),
                popup::Entry::Separator => "-".into(),
            };
            menu.entries.iter().map(line).collect()
        };
        let one = State {
            photos: 1,
            ..everything()
        };
        let photo = Target::Photo {
            point: Some([0.5, 0.5]),
        };
        let menu = open(Open::Photo, photo, &one);
        assert_eq!(menu.name, "Photo");
        assert_eq!(
            words(&menu),
            [
                "Reset",
                "-",
                "Export…",
                "-",
                "Background ▸",
                "Orientation ▸"
            ]
        );
        // Orientation's buttons, in their order, as its submenu.
        let orientation = match menu.entries.last() {
            Some(popup::Entry::Submenu(submenu)) => submenu,
            other => panic!("not Orientation ▸: {other:?}"),
        };
        let buttons: Vec<&str> = (orientation.entries.iter())
            .map(|entry| match entry {
                popup::Entry::Item(item) => item.id.as_str(),
                other => panic!("not a button's: {other:?}"),
            })
            .collect();
        assert_eq!(
            buttons,
            [
                "tool-flip-rotate_ccw",
                "tool-flip-rotate_cw",
                "tool-flip-flip_horizontally",
                "tool-flip-flip_vertically"
            ]
        );
        // On a photo open for viewing only, neither Reset nor Orientation's buttons can
        // run, and they go, its submenu with them.
        let viewing = State {
            editable: false,
            view_only: true,
            ..one
        };
        let menu = open(Open::Photo, photo, &viewing);
        assert_eq!(words(&menu), ["Export…", "-", "Background ▸"]);
        // Nor while it loads: what waits for it to go live would never run there.
        let loading = State {
            waiting: true,
            ..viewing
        };
        let menu = open(Open::Photo, photo, &loading);
        assert_eq!(words(&menu), ["Export…", "-", "Background ▸"]);
        let menu = open(Open::Thumbnail, Target::Selection, &one);
        assert_eq!(menu.name, "Thumbnail");
        assert_eq!(words(&menu), ["Export…"]);
        // Several selected: a thumbnail's acts on them; the photo's on the active photo, as
        // Edit › Reset does.
        let several = State { photos: 2, ..one };
        let menu = open(Open::Thumbnail, Target::Selection, &several);
        assert_eq!(words(&menu), ["Export…"]);
        let menu = open(Open::Photo, photo, &several);
        assert_eq!(words(&menu)[0], "Reset");
        // Nothing to reset: hidden.
        let unedited = State {
            edited: false,
            ..one
        };
        let menu = open(Open::Photo, photo, &unedited);
        assert_eq!(
            words(&menu),
            ["Export…", "-", "Background ▸", "Orientation ▸"]
        );
    }

    /// The photo's Background ▸ is View › Background's items, in its order, so the two
    /// never differ.
    #[test]
    fn the_photo_s_background_submenu_is_view_background_s_items() {
        let background = APP_ITEMS.iter().find_map(|item| match item.entry {
            MenuEntry::Submenu { title, actions } if title == BACKGROUND.title => Some(actions),
            _ => None,
        });
        let mut menu_bar = Vec::new();
        placed_in(&BACKGROUND, OperatingSystem::Nix, &mut menu_bar);
        let menu_bar: Vec<&str> = menu_bar.iter().map(|id| id.0).collect();
        assert_eq!(background, Some(&menu_bar[..]));
    }

    /// A line or a submenu left with nothing by hidden items goes with them, and every
    /// key with them.
    #[test]
    fn a_context_menu_drops_the_lines_and_submenus_its_hidden_items_leave() {
        let item = |id: &str, enabled| {
            popup::Entry::from(popup::Item::new(id, id).enabled(enabled).shortcut("Ctrl+K"))
        };
        let submenu = |entries| popup::Entry::from(popup::Submenu::new("sub", "Sub", entries));
        let entries = vec![
            popup::Entry::Separator,
            item("gone", false),
            popup::Entry::Separator,
            submenu(vec![item("gone-too", false), popup::Entry::Separator]),
            popup::Entry::Separator,
            item("kept", true),
            popup::Entry::Separator,
            submenu(vec![item("inner", true)]),
            popup::Entry::Separator,
        ];
        let kept = pared(entries.clone(), false);
        assert_eq!(
            kept,
            [
                popup::Item::new("kept", "kept").into(),
                popup::Entry::Separator,
                submenu(vec![popup::Item::new("inner", "inner").into()]),
            ]
        );
        // Waiting, the disabled items stay, still without keys.
        let waiting = pared(entries, true);
        assert_eq!(items(&waiting).len(), 4);
        assert!(items(&waiting).iter().all(|item| item.shortcut.is_none()));
    }

    /// Decision 91's cap: the photo's and a thumbnail's menus, with every item every tool
    /// and utility adds there shown, hold at most 16 items, a submenu's each counted. An
    /// item past it needs a decision.
    #[test]
    fn the_photo_s_and_a_thumbnail_s_menus_hold_at_most_16_items() {
        for (place, target) in [
            (
                Open::Photo,
                Target::Photo {
                    point: Some([0.5, 0.5]),
                },
            ),
            (Open::Thumbnail, Target::Selection),
        ] {
            let tools = definitions();
            let added = added(place, &target, &tools, apertrust_tools::utility::UTILITIES);
            let all = app_items(place).into_iter().chain(added);
            let count: usize = all.map(|item| item.actions.len()).sum();
            assert!(count <= 16, "{place:?} holds {count} items");
        }
    }

    /// The surround's menu is View › Background's items alone, as it was before the
    /// places: each background checked as chosen, enabled, with no line, named Background.
    #[test]
    fn the_surround_s_menu_is_view_background_s_items() {
        let ctx = egui::Context::default();
        ctx.set_os(OperatingSystem::Nix);
        apertrust_ui::set_locale(&ctx, Locale::EnGb);
        let keymap = Keymap::defaults(OperatingSystem::Nix);
        let state = everything();
        let anchor = Rect::from_min_size(egui::pos2(30.0, 40.0), egui::Vec2::ZERO);
        let menu = context(&ctx, Place::Surround, &Target::App, &keymap, &state, anchor);
        let tr = translator(&ctx);
        let background = drawn(
            &BACKGROUND,
            OperatingSystem::Nix,
            &keymap,
            &state,
            Lists::default(),
            &Steps::default(),
            &tr,
        );
        assert_eq!(
            menu,
            popup::Request::pull_down(anchor, "Background", background.entries)
        );
        let words: Vec<(&str, Option<bool>)> = menu
            .entries
            .iter()
            .map(|entry| match entry {
                popup::Entry::Item(item) if item.enabled && item.shortcut.is_none() => {
                    (item.text.as_str(), item.checked)
                }
                other => panic!("not a background's item: {other:?}"),
            })
            .collect();
        assert_eq!(
            words,
            [
                ("Black", Some(false)),
                ("Very dark", Some(false)),
                ("Dark", Some(true)),
                ("Medium", Some(false)),
                ("Light", Some(false)),
                ("White", Some(false)),
            ]
        );
    }
}
