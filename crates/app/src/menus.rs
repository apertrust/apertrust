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
//! registry by id, a submenu, one of the Mac's own items, or Open Recent's list, whose
//! items are the recents the app hands in ([`Recent`]). It names actions as the top
//! panel's `TOP_PANEL` does, so the registry still declares each action once: an item's
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
//! model is the one menu of the top panel's ☰ (decision 39), drawn by the popup service
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
use eframe::egui::{self, Pos2, Rect, os::OperatingSystem};

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
    title: "menu-panel-tab",
    role: Role::Plain,
    on: Platforms::All,
    groups: &[&[
        item(ActionId::PANEL_TABS[0]),
        item(ActionId::PANEL_TABS[1]),
        item(ActionId::PANEL_TABS[2]),
        item(ActionId::PANEL_TABS[3]),
        item(ActionId::PANEL_TABS[4]),
    ]],
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

/// View › Background, which the central panel's right-click menu lists too.
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

static GRID_POSITION: Menu = Menu {
    title: "menu-grid-position",
    role: Role::Plain,
    on: Platforms::All,
    groups: &[&[
        item(ActionId::GRID_POSITION[0]),
        item(ActionId::GRID_POSITION[1]),
        item(ActionId::GRID_POSITION[2]),
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
            &[item(ActionId::RESET)],
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
            // The photo's zoom, which the central bar's zoom menu offers too.
            &[
                item(ActionId::ZOOM_IN),
                item(ActionId::ZOOM_OUT),
                item(ActionId::ZOOM_FIT),
                item(ActionId::ZOOM_100),
            ],
            // The grid bar's menus (`docs/gui.md`, The grid bar).
            &[
                Entry::Submenu(&SORT_BY),
                Entry::Submenu(&THUMBNAIL_SIZE),
                item(ActionId::SHOW_LABELS),
                Entry::Submenu(&GRID_POSITION),
            ],
            &[
                item(ActionId::SHOW_BROWSE),
                Entry::Submenu(&PANEL_TAB),
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
        Entry::Recents => true,
    }
}

/// Whether `menu` holds Open Recent's list, so its items follow the recents.
fn lists_recents(menu: &Menu) -> bool {
    menu.groups
        .iter()
        .flat_map(|g| g.iter())
        .any(|e| matches!(e, Entry::Recents))
}

/// One item of a menu whose items follow the recents, or a line between two of its groups.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Listed {
    Item {
        id: ActionId,
        words: String,
        tip: Option<String>,
        shown: Shown,
    },
    Separator,
}

/// What `menu`, one that lists the recents, holds on `os` with `recents`, newest first, at
/// most [`ActionId::OPEN_RECENT`]'s: its actions' items and each recent's ([`shown`]), in
/// groups that lines separate, a group with nothing to hold left out with its line, so
/// with no recents Open Recent holds Clear Menu alone. Its submenus and the Mac's own items
/// are never in it.
pub fn listed(
    menu: &'static Menu,
    os: OperatingSystem,
    keymap: &Keymap,
    state: &State,
    recents: &[Recent],
    tr: &Translator,
    focus: Focus,
) -> Vec<Listed> {
    let mut listed = Vec::new();
    for group in groups(menu, os) {
        let mut items = Vec::new();
        for entry in group {
            match entry {
                Entry::Action(id, _) => {
                    let Some(action) = actions::action(*id) else {
                        continue;
                    };
                    items.push(Listed::Item {
                        id: *id,
                        words: tr.text(action.message),
                        tip: None,
                        shown: shown(*id, keymap, state, focus),
                    });
                }
                Entry::Recents => {
                    for (id, recent) in ActionId::OPEN_RECENT.iter().zip(recents) {
                        items.push(Listed::Item {
                            id: *id,
                            words: recent.words.clone(),
                            tip: Some(recent.tip.clone()),
                            shown: shown(*id, keymap, state, focus),
                        });
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
/// its action's ([`chosen`]). Open Recent lists `recents`, each with its path in its tip;
/// Undo and Redo name `steps` ([`words`]).
pub fn request(
    ctx: &egui::Context,
    keymap: &Keymap,
    state: &State,
    recents: &[Recent],
    steps: &Steps,
    anchor: Rect,
) -> popup::Request {
    let tr = translator(ctx);
    let os = ctx.os();
    let entries = menus(os)
        .map(|menu| drawn(menu, os, keymap, state, recents, steps, &tr).into())
        .collect();
    popup::Request::pull_down(anchor, tr.text("top-panel-menu"), entries)
}

/// `menu`'s items alone, hung from the pointer at `at`, as a right-click menu: the central
/// panel's, which is View › Background's. Its items' ids are their actions' ([`chosen`]).
pub fn context(
    ctx: &egui::Context,
    keymap: &Keymap,
    state: &State,
    menu: &'static Menu,
    at: Pos2,
) -> popup::Request {
    let tr = translator(ctx);
    let steps = Steps::default();
    let entries = drawn(menu, ctx.os(), keymap, state, &[], &steps, &tr).entries;
    let anchor = Rect::from_min_size(at, egui::Vec2::ZERO);
    popup::Request::pull_down(anchor, tr.text(menu.title), entries)
}

/// `menu` as a submenu of the ☰'s menu, its id its title's message id; one that lists the
/// recents holds `recents` ([`listed`]), and Undo and Redo name `steps`.
fn drawn(
    menu: &'static Menu,
    os: OperatingSystem,
    keymap: &Keymap,
    state: &State,
    recents: &[Recent],
    steps: &Steps,
    tr: &Translator,
) -> popup::Submenu {
    if lists_recents(menu) {
        let listed = listed(menu, os, keymap, state, recents, tr, Focus::None);
        let entries = listed.into_iter().map(|listed| match listed {
            Listed::Item {
                id,
                words,
                tip,
                shown,
            } => {
                let mut item = popup::Item::new(id.0, words).enabled(shown.enabled);
                if let Some(key) = shown.key {
                    item = item.shortcut(key.spell(os, tr));
                }
                if let Some(tip) = tip {
                    item = item.tip(tip);
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
                    let Some(action) = actions::action(*id) else {
                        continue;
                    };
                    let shown = shown(*id, keymap, state, Focus::None);
                    let words = words(action, steps, state, tr);
                    let mut item = popup::Item::new(id.0, words).enabled(shown.enabled);
                    if let Some(key) = shown.key {
                        item = item.shortcut(key.spell(os, tr));
                    }
                    if let Some(on) = action.checked(state) {
                        item = item.checked(on);
                    }
                    entries.push(item.into());
                }
                Entry::Submenu(inner) => {
                    entries.push(drawn(inner, os, keymap, state, recents, steps, tr).into());
                }
                // The Mac's alone, which `groups` leaves out elsewhere; the recents are in a
                // menu of their own ([`listed`]).
                Entry::System(_) | Entry::Recents => {}
            }
        }
    }
    popup::Submenu::new(menu.title, tr.text(menu.title), entries)
}

/// The action of the item chosen in the ☰'s menu, by its id.
pub fn chosen(id: &str) -> Option<ActionId> {
    actions::ACTIONS.iter().find(|a| a.id.0 == id).map(|a| a.id)
}

#[cfg(target_os = "macos")]
pub use mac::NativeMenus;

/// No native menus off the Mac: Windows and Linux will draw [`MENUS`] in the top panel.
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
        _: &[Recent],
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
    use muda::{MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};

    use super::{
        Entry, Listed, Recent, Role, Shown, Steps, System, groups, listed, lists_recents, menus,
        names_a_step, runs, shown, words,
    };
    use crate::actions::{self, ACTIONS, ActionId, Focus, Keymap, Shortcut, State};

    /// The Mac's menu bar, built from [`super::MENUS`] once the window exists, on the main
    /// thread, and handed to AppKit in place of winit's default menu.
    pub struct NativeMenus {
        /// Kept: dropping it would take the menu bar from AppKit.
        bar: muda::Menu,
        /// Each action's item, with what it shows now.
        items: Vec<Placed>,
        /// The menus whose items follow the recents, Open Recent, each built afresh as what
        /// it shows changes.
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
        item: MenuItem,
        menu: Submenu,
        position: usize,
        shown: Shown,
        words: String,
    }

    enum Titled {
        Menu(Submenu),
        System(PredefinedMenuItem),
    }

    /// A menu whose items follow the recents, with what it shows now, none until it is
    /// first filled.
    struct Listing {
        menu: &'static super::Menu,
        submenu: Submenu,
        shown: Option<Vec<Listed>>,
    }

    impl Listing {
        /// Puts `listed` in place of its items: the actions' and the recents' items, by
        /// their actions' ids, and the lines. A key muda cannot set is left out.
        fn fill(&self, listed: &[Listed]) -> muda::Result<()> {
            while self.submenu.remove_at(0).is_some() {}
            for listed in listed {
                match listed {
                    Listed::Item {
                        id, words, shown, ..
                    } => {
                        let item = MenuItem::with_id(id.0, words, shown.enabled, None);
                        if let Some(key) = shown.key.and_then(key_equivalent) {
                            item.set_key_accelerator(Some(key))?;
                        }
                        self.submenu.append(&item)?;
                    }
                    Listed::Separator => {
                        self.submenu.append(&PredefinedMenuItem::separator())?;
                    }
                }
            }
            Ok(())
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
            // Filled by each update, as the recents change.
            if lists_recents(menu) {
                self.listing.push(Listing {
                    menu,
                    submenu: submenu.clone(),
                    shown: None,
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
                                item,
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
                        // In a menu of their own, filled by each update.
                        Entry::Recents => {}
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
                    let action = ACTIONS.iter().find(|a| a.id.0 == id.0)?;
                    runs(action.id, repeat).then_some(action.id)
                })
                .collect()
        }

        /// Brings every item up to date with `keymap`, `state`, the focus and the
        /// language, calling muda only for what changed: a language chosen in Settings
        /// renames every menu, and while a widget has focus no item holds a key it keeps.
        /// Open Recent lists `recents`, built afresh only when what it shows changes; Undo
        /// and Redo name `steps`.
        pub fn update(
            &mut self,
            ctx: &egui::Context,
            keymap: &Keymap,
            state: &State,
            recents: &[Recent],
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
                let now = listed(listing.menu, mac, keymap, state, recents, &tr, focus);
                if listing.shown.as_ref() == Some(&now) {
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
        /// Gives the item `key` as its key equivalent, or none. Given none, muda 0.21 keeps
        /// the key equivalent it had (muda 0.21 leaves it in place), so the item is then put
        /// back in its place, which builds it afresh without one. That relies on AppKit
        /// only appending its own items to a menu (Edit's dictation, Window's tiling), so
        /// muda's index of the item among its children is still its index in the NSMenu.
        fn set_key(&self, key: Option<KeyAccelerator>) -> muda::Result<()> {
            let none = key.is_none();
            self.item.set_key_accelerator(key)?;
            if none {
                self.menu.remove(&self.item)?;
                self.menu.insert(&self.item, self.position)?;
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
    /// item is a press; and the ☰'s menu's own, which opens the menu it would be in.
    const LEFT_OUT: [ActionId; 5] = [
        ActionId::EDIT_VIEW,
        ActionId::NEXT_REGION,
        ActionId::PREVIOUS_REGION,
        ActionId::PEEK,
        ActionId::MENU,
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
            edited: true,
            undo: true,
            redo: true,
            text: false,
            photos: 2,
            opened: true,
            recents: [Some(true); crate::recents::MOST],
            tabs: &[true; 5],
            scale: 1.5,
            dialog: false,
            modal: false,
            dialog_window: false,
            full_screen: false,
            panel_tab: 0,
            grid: crate::settings::Grid::default(),
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
        listed(&OPEN_RECENT, os, &keymap, state, recents, &tr, Focus::None)
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
        let menu = request(&ctx, &keymap, &state, &recents, &steps, Rect::NOTHING);
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
        assert_eq!(key(ActionId::SHOW_BROWSE, Focus::Text), None);
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
            &[],
            steps,
            Rect::NOTHING,
        )
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
}
