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

//! The actions registry and the keyboard map (`docs/gui.md`, Keyboard and actions).
//!
//! Every action is declared once, in [`ACTIONS`]: its id, its name (a message id), its
//! default keys, its scope, when it can run and what it emits, a command for the engine's
//! session or an intent for the interface. The top panel's buttons, the side panel's tabs,
//! the sections' peeking and the tooltips all read it, and [`read`] turns the frame's keys
//! into actions before any widget is drawn; no widget handles a shortcut.
//!
//! **Scopes.** The focused widget keeps its own keys first (a slider its arrows, a text
//! field everything it types or edits with), then the view's actions, then the app's: the
//! narrowest wins. A key whose action can't run does nothing, even where a wider scope
//! binds it too.
//!
//! **Keys** match egui's logical key, so a shortcut follows the character printed on the
//! key: Ctrl+Z is the key labelled Z on a German keyboard too. A letter egui has no name
//! for, as every Cyrillic and Greek one, arrives as its physical key (egui-winit puts it in
//! the logical key's place), so Ctrl+Я on a Ukrainian keyboard is Ctrl+Z. On the number
//! row the digit comes first, since layouts disagree about the symbols on it: French AZERTY
//! types `'` on the 4 key, so Ctrl+4 arrives as Ctrl+' and is taken as Ctrl+4. Letters
//! under symbols are not read by their physical key, since Dvorak puts `'` where QWERTY has
//! Q and Ctrl+' must not quit. Ctrl is Cmd on macOS ([`Modifiers::COMMAND`]). Alt and
//! Shift must match, so Option letters on a Mac never fire a shortcut.
//!
//! **AltGr** is no modifier egui sees: winit drops Ctrl and Alt while it is held on Windows,
//! and on Linux it is a modifier of its own. So AltGr+E, which types ę on a Polish
//! keyboard, arrives as a plain E, since egui-winit does not report AltGr. A letter's shortcut is
//! therefore skipped when the same press typed something else: a Latin letter not its
//! own, as ę or ł, or no letter at all, as @ or €. A letter of another alphabet, as у on a
//! Ukrainian keyboard, is that key's own letter and still runs it.
//!
//! **The user's keys** come from settings (`settings`): [`Keymap::load`] puts them in
//! place of the defaults, refusing any that would clash, and [`Shortcut::text`] writes
//! them the same way on every platform.

use std::collections::BTreeMap;

use apertrust_engine::session::Command;
use apertrust_ui::components::{GridBarAction, GridPosition, SortKey, ThumbnailSize};
use apertrust_ui::{Background, Translator};
use eframe::egui::{self, Key, Modifiers, os::OperatingSystem};

/// An action's id, stable so that settings can keep the keys a user gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ActionId(pub &'static str);

impl ActionId {
    pub const ABOUT: ActionId = ActionId("about");
    /// Help › Show Log: the log window.
    pub const SHOW_LOG: ActionId = ActionId("show-log");
    /// View › Jobs: the jobs window.
    pub const SHOW_JOBS: ActionId = ActionId("show-jobs");
    pub const SETTINGS: ActionId = ActionId("settings");
    pub const OPEN: ActionId = ActionId("open");
    pub const OPEN_FOLDER: ActionId = ActionId("open-folder");
    /// File › Open Recent's items by place, newest first, whatever they hold.
    pub const OPEN_RECENT: [ActionId; crate::recents::MOST] = [
        ActionId("open-recent-1"),
        ActionId("open-recent-2"),
        ActionId("open-recent-3"),
        ActionId("open-recent-4"),
        ActionId("open-recent-5"),
        ActionId("open-recent-6"),
        ActionId("open-recent-7"),
        ActionId("open-recent-8"),
        ActionId("open-recent-9"),
        ActionId("open-recent-10"),
    ];
    /// Open Recent's Clear Menu.
    pub const CLEAR_RECENT: ActionId = ActionId("clear-recent");
    pub const IMPORT: ActionId = ActionId("import");
    pub const EXPORT: ActionId = ActionId("export");
    pub const EXPORT_AGAIN: ActionId = ActionId("export-again");
    pub const RESET: ActionId = ActionId("reset");
    pub const UNDO: ActionId = ActionId("undo");
    pub const REDO: ActionId = ActionId("redo");
    pub const COPY_EDIT: ActionId = ActionId("copy-edit");
    pub const APPLY_EDIT: ActionId = ActionId("apply-edit");
    pub const COPY_APPLY_PARTS: ActionId = ActionId("copy-apply-parts");
    pub const EDIT_VIEW: ActionId = ActionId("view-edit");
    /// View › Browse: the first tab holding Browse.
    pub const SHOW_BROWSE: ActionId = ActionId("show-browse");
    /// The side panel's tabs by position, whatever the user puts in them.
    pub const PANEL_TABS: [ActionId; 5] = [
        ActionId("panel-tab-1"),
        ActionId("panel-tab-2"),
        ActionId("panel-tab-3"),
        ActionId("panel-tab-4"),
        ActionId("panel-tab-5"),
    ];
    pub const SCALE_UP: ActionId = ActionId("scale-up");
    pub const SCALE_DOWN: ActionId = ActionId("scale-down");
    pub const SCALE_RESET: ActionId = ActionId("scale-reset");
    pub const NEXT_REGION: ActionId = ActionId("next-region");
    pub const PREVIOUS_REGION: ActionId = ActionId("previous-region");
    /// The ☰'s menu, where the menus are drawn (decision 39).
    pub const MENU: ActionId = ActionId("menu");
    pub const FULL_SCREEN: ActionId = ActionId("full-screen");
    /// File › Close: the folder or photo open.
    pub const CLOSE: ActionId = ActionId("close");
    /// File › Close Window, the Mac's alone: the key window, a dialog's or the main one.
    pub const CLOSE_WINDOW: ActionId = ActionId("close-window");
    pub const QUIT: ActionId = ActionId("quit");
    /// Quits as Quit does and starts the app again with what is open (decision 81):
    /// Settings' Restart now, in no menu yet.
    pub const RESTART: ActionId = ActionId("restart");
    pub const PEEK: ActionId = ActionId("peek");
    pub const NEXT_PHOTO: ActionId = ActionId("next-photo");
    pub const PREVIOUS_PHOTO: ActionId = ActionId("previous-photo");
    /// The photo's zoom, View's items and the central bar's zoom menu's: in, out, Fit and
    /// 100%.
    pub const ZOOM_IN: ActionId = ActionId("zoom-in");
    pub const ZOOM_OUT: ActionId = ActionId("zoom-out");
    pub const ZOOM_FIT: ActionId = ActionId("zoom-fit");
    pub const ZOOM_100: ActionId = ActionId("zoom-100");
    /// The grid bar's Sort menu and View › Sort By: the key, then the direction.
    pub const SORT: [ActionId; 4] = [
        ActionId("sort-capture-time"),
        ActionId("sort-file-name"),
        ActionId("sort-edited"),
        ActionId("sort-rating"),
    ];
    pub const SORT_ASCENDING: ActionId = ActionId("sort-ascending");
    pub const SORT_DESCENDING: ActionId = ActionId("sort-descending");
    /// The grid bar's Zoom menu and View › Thumbnail Size, smallest first.
    pub const THUMBNAILS: [ActionId; 4] = [
        ActionId("thumbnails-small"),
        ActionId("thumbnails-medium"),
        ActionId("thumbnails-large"),
        ActionId("thumbnails-extra-large"),
    ];
    /// The grid bar's Labels and View › Show Labels: the thumbnails' captions.
    pub const SHOW_LABELS: ActionId = ActionId("show-labels");
    /// The grid bar's options menu and View › Grid Panel Position: left, right, bottom.
    pub const GRID_POSITION: [ActionId; 3] = [
        ActionId("grid-position-left"),
        ActionId("grid-position-right"),
        ActionId("grid-position-bottom"),
    ];
    /// View › Background and the central panel's right-click menu, black to white.
    pub const BACKGROUND: [ActionId; 6] = [
        ActionId("background-black"),
        ActionId("background-very-dark"),
        ActionId("background-dark"),
        ActionId("background-medium"),
        ActionId("background-light"),
        ActionId("background-white"),
    ];
}

/// The old views enum, from before the one view (decision 37), to become the central
/// panel's layouts: only Edit is left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Edit,
}

/// Where an action's keys work: everywhere, or in one view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    App,
    View(View),
}

/// How a key runs its action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trigger {
    /// Once per press.
    Press,
    /// On the press, and again while the key repeats.
    Repeat,
    /// For as long as the key is held.
    Hold,
}

/// What an action does.
// A command is far larger than an intent where a path is (a Windows `PathBuf`), but every
// action lives in the static [`ACTIONS`], where a `Box` cannot be built.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq)]
pub enum Emit {
    /// A command for the engine's session.
    Command(Command),
    /// A change to the interface's own state, applied after the frame.
    Intent(Intent),
    /// Nothing yet: the action is listed so that its keys are kept and shown, and it is
    /// never available until it is built.
    Later,
}

/// A change to the interface's own state (`docs/gui.md`, A frame).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    About,
    /// The log window, opened or brought to the front (`log_window`).
    ShowLog,
    /// The jobs window, opened or brought to the front as the spinner's click does, so it
    /// never closes itself (`jobs`).
    ShowJobs,
    Settings,
    /// The platform's open dialog, for one or more raws (`dialogs`).
    Open,
    /// The platform's folder dialog: the folder chosen is opened as the library's folder.
    OpenFolder,
    /// Open Recent's entry at this place, newest first, opened as Open Folder… opens one.
    OpenRecent(usize),
    /// Clear Menu: the recents emptied.
    ClearRecent,
    /// File › Close: what is open closed, its edit saved, and nothing open after it.
    Close,
    /// The Export dialog, a sheet on the Mac (`export`).
    Export,
    /// The export with the settings as last kept, with no dialog (`export_again`).
    ExportAgain,
    View(View),
    PanelTab(usize),
    /// View › Browse: the first tab holding Browse chosen, nothing where it is chosen.
    ShowBrowse,
    ScaleUp,
    ScaleDown,
    ScaleReset,
    NextRegion,
    PreviousRegion,
    /// The ☰'s menu, opened at its top, or closed where it is open.
    Menu,
    FullScreen,
    /// The main window, which ends the app as Quit does; a dialog's own window is the
    /// app's to close (`run_action`, in the app's `main`).
    CloseWindow,
    Quit,
    /// Quit, then the app started again (`restart`): the app turns it into Quit as it runs
    /// it, having noted the restart, so the quit path's own runs of Quit keep it.
    Restart,
    /// Held, never applied: the sections read it ([`Keys::held`]) and peek.
    Peek,
    /// The grid panel's sort, its direction, the thumbnails' size or where the panel is, as
    /// the grid bar's menus choose them, or its Labels turned on or off.
    Grid(GridBarAction),
    /// The background behind the photo (`docs/gui.md`, The background).
    Background(Background),
}

/// What the actions' availability depends on, as the app is this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub view: View,
    pub photo: bool,
    pub edited: bool,
    /// Whether the open photo's history has something for Undo, and for Redo, to do, and
    /// takes them: live and not open for viewing only (decision 78).
    pub undo: bool,
    pub redo: bool,
    /// Whether a text field of the main window has the focus: Undo and Redo act on it, as
    /// the Mac's Edit menu does (decision 39), whatever the photo's history holds.
    pub text: bool,
    pub photos: usize,
    /// Whether anything is open, a folder or a photo, opening, open or failed: File ›
    /// Close's to close.
    pub opened: bool,
    /// The recents Open Recent lists, newest first, by place: whether each can be opened,
    /// none where the list holds no entry there. One found missing cannot.
    pub recents: [Option<bool>; crate::recents::MOST],
    /// The side panel's tabs, by position: whether each can be chosen. One with nothing
    /// behind it yet cannot, so its key is unavailable (decision 38).
    pub tabs: &'static [bool],
    /// The interface's scale: egui's zoom factor.
    pub scale: f32,
    /// A modal dialog is open, or a dialog's window has the focus: the view's keys rest,
    /// and focus stays in the dialog.
    pub dialog: bool,
    /// A modal dialog is open over the main window, a sheet or a modal window of the dialog
    /// host's, the open dialog or About: every action that opens a dialog, a file
    /// dialog or About rests, so one dialog is open at a time, and so does Close Window, but
    /// in a dialog's own window with the focus (decision 41).
    pub modal: bool,
    /// A modeless dialog's own window has the focus, Settings' on the Mac: the items that act
    /// on the main window or its photo rest, as AppKit rests an item whose target is not in
    /// the key window's responder chain, and Close Window closes that window.
    pub dialog_window: bool,
    /// Whether the window is full screen, which Full Screen's item shows checked.
    pub full_screen: bool,
    /// The side panel's tab shown, by position, whose item is shown checked.
    pub panel_tab: usize,
    /// How the grid panel shows the folder, whose items are shown checked.
    pub grid: crate::settings::Grid,
    /// Whether the grid panel shows photos: its grid bar's items rest without any.
    pub shown: bool,
    /// The background behind the photo, whose item is shown checked; None for a theme's
    /// own surround that is none of the six.
    pub background: Option<Background>,
}

/// One key with its modifiers. Ctrl here is [`Modifiers::COMMAND`], Cmd on macOS. A Mac's
/// Control key is [`Modifiers::CTRL`], so Control+Cmd, as in Control+Cmd+F, is `CTRL` with
/// [`Modifiers::MAC_CMD`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Shortcut {
    pub modifiers: Modifiers,
    pub key: Key,
}

/// A default key, on every platform or on some.
#[derive(Clone, Copy, Debug)]
pub struct Binding {
    shortcut: Shortcut,
    on: Platforms,
}

/// The platforms a default key, or a menu's item, belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platforms {
    All,
    Mac,
    /// Windows and Linux.
    NotMac,
    Windows,
    /// Linux and every other platform but the Mac and Windows.
    Linux,
}

impl Platforms {
    pub fn includes(self, os: OperatingSystem) -> bool {
        let mac = os == OperatingSystem::Mac;
        let windows = os == OperatingSystem::Windows;
        match self {
            Platforms::All => true,
            Platforms::Mac => mac,
            Platforms::NotMac => !mac,
            Platforms::Windows => windows,
            Platforms::Linux => !mac && !windows,
        }
    }
}

const fn bind(modifiers: Modifiers, key: Key) -> Binding {
    Binding {
        shortcut: Shortcut { modifiers, key },
        on: Platforms::All,
    }
}

const fn plain(key: Key) -> Binding {
    bind(Modifiers::NONE, key)
}

const fn command(key: Key) -> Binding {
    bind(Modifiers::COMMAND, key)
}

const fn command_shift(key: Key) -> Binding {
    bind(Modifiers::COMMAND.plus(Modifiers::SHIFT), key)
}

impl Binding {
    const fn only(mut self, on: Platforms) -> Binding {
        self.on = on;
        self
    }

    fn applies(&self, os: OperatingSystem) -> bool {
        self.on.includes(os)
    }
}

/// An action, as the registry declares it.
pub struct Action {
    pub id: ActionId,
    /// Its name, a message id (`crates/ui/locales`), which the menus show.
    pub message: &'static str,
    /// Its default keys, the first one shown in tooltips.
    pub keys: &'static [Binding],
    pub scope: Scope,
    pub trigger: Trigger,
    /// Whether it can run in this state; one that can't is drawn disabled.
    pub available: fn(&State) -> bool,
    pub emits: Emit,
}

impl Action {
    /// Whether it can run now. An action not built yet never can.
    pub fn can_run(&self, state: &State) -> bool {
        !matches!(self.emits, Emit::Later) && (self.available)(state)
    }

    /// Whether it can run where it is: it can run now, and its scope is active, so a view's
    /// action rests while a dialog is open, as its keys do. A menu item is enabled exactly
    /// then, and a choice runs only then.
    pub fn enabled(&self, state: &State) -> bool {
        self.can_run(state) && scopes(state).any(|scope| scope == self.scope)
    }

    /// For an action that turns something on or chooses one of a set, whether it is on or
    /// the one chosen in `state`, which its menu item shows by a check mark: Full Screen,
    /// and the side panel's tabs. None for a command.
    pub fn checked(&self, state: &State) -> Option<bool> {
        match self.emits {
            Emit::Intent(Intent::FullScreen) => Some(state.full_screen),
            Emit::Intent(Intent::PanelTab(tab)) => Some(state.panel_tab == tab),
            Emit::Intent(Intent::Grid(chosen)) => Some(state.grid.holds(chosen)),
            Emit::Intent(Intent::Background(b)) => Some(state.background == Some(b)),
            _ => None,
        }
    }
}

/// The interface's scale, egui's zoom factor: from half to three times the system's, in
/// tenths, as the settings schema has it.
pub const SCALES: std::ops::RangeInclusive<f32> = crate::settings::registry::SCALE.range();

/// The next scale up, or down when `up` is false.
pub fn step_scale(scale: f32, up: bool) -> f32 {
    let tenths = (scale * 10.0).round() + if up { 1.0 } else { -1.0 };
    (tenths / 10.0).clamp(*SCALES.start(), *SCALES.end())
}

fn always(_: &State) -> bool {
    true
}

fn never(_: &State) -> bool {
    false
}

/// Whether no modal dialog is open, for an action that opens a dialog.
fn no_modal_dialog(state: &State) -> bool {
    !state.modal
}

/// Whether the main window is the one the keys and the menus act on, for an action that
/// acts on it or its photo: not while a dialog's own window has the focus.
fn on_main_window(state: &State) -> bool {
    !state.dialog_window
}

/// Whether the grid bar's menus can be used: the grid panel shows photos, on the main
/// window, with no modal dialog over it.
fn grid_shown(state: &State) -> bool {
    state.shown && on_main_window(state) && !state.modal
}

/// Whether the grid panel can be moved: on the main window, with no modal dialog over it,
/// whether it shows photos or not, as its options menu opens in every state.
fn grid_anywhere(state: &State) -> bool {
    on_main_window(state) && !state.modal
}

/// The grid bar's choice `chosen` as an action, which can run where `available` says: no
/// keys by default, its words the action's own message, `action-<id>`.
const fn grid(
    id: ActionId,
    message: &'static str,
    chosen: GridBarAction,
    available: fn(&State) -> bool,
) -> Action {
    Action {
        id,
        message,
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available,
        emits: Emit::Intent(Intent::Grid(chosen)),
    }
}

/// The background `chosen` as an action: no keys by default, on the main window with no
/// modal dialog over it, in every state, as the central panel's menu opens with no photo.
const fn background(id: ActionId, message: &'static str, chosen: Background) -> Action {
    Action {
        id,
        message,
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: grid_anywhere,
        emits: Emit::Intent(Intent::Background(chosen)),
    }
}

/// Whether Open Recent's `I`th entry, newest first, can be opened: it is there and not
/// found missing, and no modal dialog is open, as for Open Folder….
fn recent_enabled<const I: usize>(state: &State) -> bool {
    no_modal_dialog(state) && state.recents.get(I).copied().flatten() == Some(true)
}

/// Open Recent's entry at place `I`, newest first: no keys by default, its words for the
/// keyboard map its own message, the menu showing the folder's name.
const fn open_recent<const I: usize>(message: &'static str) -> Action {
    Action {
        id: ActionId::OPEN_RECENT[I],
        message,
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: recent_enabled::<I>,
        emits: Emit::Intent(Intent::OpenRecent(I)),
    }
}

/// Whether the side panel has an `I`th tab, by position, that can be chosen, the main
/// window the one acted on.
fn tab_enabled<const I: usize>(state: &State) -> bool {
    on_main_window(state) && state.tabs.get(I).copied().unwrap_or(false)
}

/// Every action, in the order a keyboard map lists them (`docs/gui.md`, The default
/// keyboard map). Slider keys are not here: they are the slider's own (`SliderRow`).
pub static ACTIONS: [Action; 71] = [
    // The top panel's mark, and About Apertrust in the menus once they come.
    Action {
        id: ActionId::ABOUT,
        message: "action-about",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: no_modal_dialog,
        emits: Emit::Intent(Intent::About),
    },
    // Help › Show Log, logging on or off: off, the window says where to turn it on. Never
    // disabled, as the log window opens beside any dialog.
    Action {
        id: ActionId::SHOW_LOG,
        message: "action-show-log",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: always,
        emits: Emit::Intent(Intent::ShowLog),
    },
    // View › Jobs, finished rows too: never disabled, as the jobs window opens beside any
    // dialog.
    Action {
        id: ActionId::SHOW_JOBS,
        message: "action-show-jobs",
        keys: &[command_shift(Key::J)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: always,
        emits: Emit::Intent(Intent::ShowJobs),
    },
    Action {
        id: ActionId::SETTINGS,
        message: "action-settings",
        keys: &[command(Key::Comma)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: no_modal_dialog,
        emits: Emit::Intent(Intent::Settings),
    },
    // Raws, opening their folder with the first active (`docs/gui.md`, The menu bar, File).
    Action {
        id: ActionId::OPEN,
        message: "action-open",
        keys: &[command(Key::O)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: no_modal_dialog,
        emits: Emit::Intent(Intent::Open),
    },
    // Folder mode: the folder's photos fill the grid panel.
    Action {
        id: ActionId::OPEN_FOLDER,
        message: "action-open-folder",
        keys: &[command_shift(Key::O)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: no_modal_dialog,
        emits: Emit::Intent(Intent::OpenFolder),
    },
    // File › Open Recent, the recents newest first, then Clear Menu, with nothing to
    // clear disabled (`docs/gui.md`, File).
    open_recent::<0>("action-open-recent-1"),
    open_recent::<1>("action-open-recent-2"),
    open_recent::<2>("action-open-recent-3"),
    open_recent::<3>("action-open-recent-4"),
    open_recent::<4>("action-open-recent-5"),
    open_recent::<5>("action-open-recent-6"),
    open_recent::<6>("action-open-recent-7"),
    open_recent::<7>("action-open-recent-8"),
    open_recent::<8>("action-open-recent-9"),
    open_recent::<9>("action-open-recent-10"),
    Action {
        id: ActionId::CLEAR_RECENT,
        message: "action-clear-recent",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| no_modal_dialog(s) && s.recents[0].is_some(),
        emits: Emit::Intent(Intent::ClearRecent),
    },
    Action {
        id: ActionId::IMPORT,
        message: "action-import",
        keys: &[command(Key::I)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: never,
        emits: Emit::Later,
    },
    Action {
        id: ActionId::EXPORT,
        message: "action-export",
        keys: &[command(Key::E)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| s.photo && no_modal_dialog(s),
        emits: Emit::Intent(Intent::Export),
    },
    // darktable's Ctrl+E: no key by default, so anyone used to it can bind one.
    Action {
        id: ActionId::EXPORT_AGAIN,
        message: "action-export-again",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| s.photo && no_modal_dialog(s),
        emits: Emit::Intent(Intent::ExportAgain),
    },
    // No key by default, as darktable's discard history has none: its button is registered
    // with no key (`src/libs/copy_history.c:328-330`), and the history module's reset asks
    // first, by default (`ask_before_discard`, `src/libs/history.c:1335-1354`), since it
    // throws the whole edit away at once. Undo takes it back, and a person may give it a
    // key.
    Action {
        id: ActionId::RESET,
        message: "action-reset",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| s.photo && s.edited && on_main_window(s),
        emits: Emit::Command(Command::Reset),
    },
    // darktable's Ctrl+Z and Ctrl+Y (`src/views/darkroom.c:3425-3428`), with the
    // platforms' Shift for Redo: Redo shows Ctrl+Y first on Windows, Ctrl+Shift+Z first on
    // Linux and Shift+Cmd+Z alone on the Mac (`docs/gui.md`, The menu bar). A text field
    // with the focus keeps these keys for its own undo ([`Focus::keeps`]); run from a menu
    // there, the Mac's by its key equivalent too, they are handed to it ([`field_keys`]).
    Action {
        id: ActionId::UNDO,
        message: "action-undo",
        keys: &[command(Key::Z)],
        scope: Scope::App,
        trigger: Trigger::Repeat,
        available: |s| (s.undo || s.text) && on_main_window(s) && !s.modal,
        emits: Emit::Command(Command::Undo),
    },
    Action {
        id: ActionId::REDO,
        message: "action-redo",
        keys: &[
            command(Key::Y).only(Platforms::Windows),
            command_shift(Key::Z),
            command(Key::Y).only(Platforms::Linux),
        ],
        scope: Scope::App,
        trigger: Trigger::Repeat,
        available: |s| (s.redo || s.text) && on_main_window(s) && !s.modal,
        emits: Emit::Command(Command::Redo),
    },
    Action {
        id: ActionId::COPY_EDIT,
        message: "action-copy-edit",
        keys: &[command_shift(Key::C)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: never,
        emits: Emit::Later,
    },
    Action {
        id: ActionId::APPLY_EDIT,
        message: "action-apply-edit",
        keys: &[command_shift(Key::V)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: never,
        emits: Emit::Later,
    },
    Action {
        id: ActionId::COPY_APPLY_PARTS,
        message: "action-copy-apply-parts",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: never,
        emits: Emit::Later,
    },
    Action {
        id: ActionId::EDIT_VIEW,
        message: "action-view-edit",
        keys: &[plain(Key::E)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| s.view != View::Edit,
        emits: Emit::Intent(Intent::View(View::Edit)),
    },
    Action {
        id: ActionId::SHOW_BROWSE,
        message: "action-show-browse",
        keys: &[plain(Key::L)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: on_main_window,
        emits: Emit::Intent(Intent::ShowBrowse),
    },
    Action {
        id: ActionId::PANEL_TABS[0],
        message: "action-panel-tab-1",
        keys: &[command(Key::Num1)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: tab_enabled::<0>,
        emits: Emit::Intent(Intent::PanelTab(0)),
    },
    Action {
        id: ActionId::PANEL_TABS[1],
        message: "action-panel-tab-2",
        keys: &[command(Key::Num2)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: tab_enabled::<1>,
        emits: Emit::Intent(Intent::PanelTab(1)),
    },
    Action {
        id: ActionId::PANEL_TABS[2],
        message: "action-panel-tab-3",
        keys: &[command(Key::Num3)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: tab_enabled::<2>,
        emits: Emit::Intent(Intent::PanelTab(2)),
    },
    Action {
        id: ActionId::PANEL_TABS[3],
        message: "action-panel-tab-4",
        keys: &[command(Key::Num4)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: tab_enabled::<3>,
        emits: Emit::Intent(Intent::PanelTab(3)),
    },
    Action {
        id: ActionId::PANEL_TABS[4],
        message: "action-panel-tab-5",
        keys: &[command(Key::Num5)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: tab_enabled::<4>,
        emits: Emit::Intent(Intent::PanelTab(4)),
    },
    // Ctrl+= first: on most keyboards + needs Shift, and Ctrl++ matches with it held.
    Action {
        id: ActionId::SCALE_UP,
        message: "action-scale-up",
        keys: &[command(Key::Equals), command(Key::Plus)],
        scope: Scope::App,
        trigger: Trigger::Repeat,
        available: |s| s.scale < *SCALES.end(),
        emits: Emit::Intent(Intent::ScaleUp),
    },
    Action {
        id: ActionId::SCALE_DOWN,
        message: "action-scale-down",
        keys: &[command(Key::Minus)],
        scope: Scope::App,
        trigger: Trigger::Repeat,
        available: |s| s.scale > *SCALES.start(),
        emits: Emit::Intent(Intent::ScaleDown),
    },
    Action {
        id: ActionId::SCALE_RESET,
        message: "action-scale-reset",
        keys: &[command(Key::Num0)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| s.scale != 1.0,
        emits: Emit::Intent(Intent::ScaleReset),
    },
    Action {
        id: ActionId::NEXT_REGION,
        message: "action-next-region",
        keys: &[plain(Key::F6)],
        scope: Scope::App,
        trigger: Trigger::Repeat,
        available: |s| !s.dialog,
        emits: Emit::Intent(Intent::NextRegion),
    },
    Action {
        id: ActionId::PREVIOUS_REGION,
        message: "action-previous-region",
        keys: &[bind(Modifiers::SHIFT, Key::F6)],
        scope: Scope::App,
        trigger: Trigger::Repeat,
        available: |s| !s.dialog,
        emits: Emit::Intent(Intent::PreviousRegion),
    },
    // F10 opens the ☰'s menu on Windows and Linux, as GNOME's apps open theirs (decision
    // 39). Its name is the ☰'s, and the menu it opens never holds it.
    Action {
        id: ActionId::MENU,
        message: "top-panel-menu",
        keys: &[plain(Key::F10).only(Platforms::NotMac)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| !s.dialog,
        emits: Emit::Intent(Intent::Menu),
    },
    // macOS keeps F11 for showing the desktop; its own full-screen keys come first there.
    Action {
        id: ActionId::FULL_SCREEN,
        message: "action-full-screen",
        keys: &[
            bind(Modifiers::CTRL.plus(Modifiers::MAC_CMD), Key::F).only(Platforms::Mac),
            plain(Key::F11),
        ],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: on_main_window,
        emits: Emit::Intent(Intent::FullScreen),
    },
    // What is open, the folder or the photo, on the main window with no modal dialog over
    // it. Ctrl+W on Windows and Linux; on the Mac Cmd+W is Close Window's, so Close has
    // none of its own (`docs/gui.md`, File).
    Action {
        id: ActionId::CLOSE,
        message: "action-close",
        keys: &[command(Key::W).only(Platforms::NotMac)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| s.opened && on_main_window(s) && !s.modal,
        emits: Emit::Intent(Intent::Close),
    },
    // The key window: a dialog's own, as Settings' on the Mac, or the main window, which
    // ends the app as Quit does while it is the one window, and rests under a modal dialog
    // there, as AppKit's does under a sheet. The Mac's: Windows and Linux close a window
    // with its caption's close button (`docs/gui.md`, The menu bar).
    Action {
        id: ActionId::CLOSE_WINDOW,
        message: "action-close-window",
        keys: &[command(Key::W).only(Platforms::Mac)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| s.dialog_window || !s.modal,
        emits: Emit::Intent(Intent::CloseWindow),
    },
    Action {
        id: ActionId::QUIT,
        message: "action-quit",
        keys: &[command(Key::Q)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: always,
        emits: Emit::Intent(Intent::Quit),
    },
    // Settings' Restart now, and Try again after a start that did not finish (decision 81):
    // Quit, then the app again with what is open. No key, and no menu yet.
    Action {
        id: ActionId::RESTART,
        message: "action-restart",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: always,
        emits: Emit::Intent(Intent::Restart),
    },
    Action {
        id: ActionId::PEEK,
        message: "action-peek",
        keys: &[plain(Key::H)],
        scope: Scope::View(View::Edit),
        trigger: Trigger::Hold,
        available: |s| s.photo,
        emits: Emit::Intent(Intent::Peek),
    },
    Action {
        id: ActionId::NEXT_PHOTO,
        message: "action-next-photo",
        keys: &[plain(Key::Space), plain(Key::ArrowRight)],
        scope: Scope::View(View::Edit),
        trigger: Trigger::Repeat,
        available: |s| s.photos > 1,
        emits: Emit::Later,
    },
    Action {
        id: ActionId::PREVIOUS_PHOTO,
        message: "action-previous-photo",
        keys: &[plain(Key::Backspace), plain(Key::ArrowLeft)],
        scope: Scope::View(View::Edit),
        trigger: Trigger::Repeat,
        available: |s| s.photos > 1,
        emits: Emit::Later,
    },
    // The photo's zoom, not built until the viewport can zoom. Then Fit, in and out take
    // Ctrl+0, Ctrl+= and Ctrl+− from the interface's size (`docs/gui.md`, The default
    // keyboard map); until then those keys stay the size's.
    Action {
        id: ActionId::ZOOM_IN,
        message: "action-zoom-in",
        keys: &[],
        scope: Scope::View(View::Edit),
        trigger: Trigger::Repeat,
        available: |s| s.photo,
        emits: Emit::Later,
    },
    Action {
        id: ActionId::ZOOM_OUT,
        message: "action-zoom-out",
        keys: &[],
        scope: Scope::View(View::Edit),
        trigger: Trigger::Repeat,
        available: |s| s.photo,
        emits: Emit::Later,
    },
    Action {
        id: ActionId::ZOOM_FIT,
        message: "action-zoom-fit",
        keys: &[],
        scope: Scope::View(View::Edit),
        trigger: Trigger::Press,
        available: |s| s.photo,
        emits: Emit::Later,
    },
    Action {
        id: ActionId::ZOOM_100,
        message: "action-zoom-100",
        keys: &[plain(Key::Z)],
        scope: Scope::View(View::Edit),
        trigger: Trigger::Press,
        available: |s| s.photo,
        emits: Emit::Later,
    },
    // The grid bar's menus and View's Sort By and Thumbnail Size (`docs/gui.md`, The grid
    // bar); no keys by default, so + and − stay free. Rating comes with the ratings.
    grid(
        ActionId::SORT[0],
        "action-sort-capture-time",
        GridBarAction::Sort(SortKey::CaptureTime),
        grid_shown,
    ),
    grid(
        ActionId::SORT[1],
        "action-sort-file-name",
        GridBarAction::Sort(SortKey::FileName),
        grid_shown,
    ),
    grid(
        ActionId::SORT[2],
        "action-sort-edited",
        GridBarAction::Sort(SortKey::Edited),
        grid_shown,
    ),
    Action {
        id: ActionId::SORT[3],
        message: "action-sort-rating",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: never,
        emits: Emit::Later,
    },
    grid(
        ActionId::SORT_ASCENDING,
        "action-sort-ascending",
        GridBarAction::Descending(false),
        grid_shown,
    ),
    grid(
        ActionId::SORT_DESCENDING,
        "action-sort-descending",
        GridBarAction::Descending(true),
        grid_shown,
    ),
    grid(
        ActionId::THUMBNAILS[0],
        "action-thumbnails-small",
        GridBarAction::Size(ThumbnailSize::Small),
        grid_shown,
    ),
    grid(
        ActionId::THUMBNAILS[1],
        "action-thumbnails-medium",
        GridBarAction::Size(ThumbnailSize::Medium),
        grid_shown,
    ),
    grid(
        ActionId::THUMBNAILS[2],
        "action-thumbnails-large",
        GridBarAction::Size(ThumbnailSize::Large),
        grid_shown,
    ),
    grid(
        ActionId::THUMBNAILS[3],
        "action-thumbnails-extra-large",
        GridBarAction::Size(ThumbnailSize::ExtraLarge),
        grid_shown,
    ),
    // The grid bar's Labels, a toggle shown checked while the captions show.
    grid(
        ActionId::SHOW_LABELS,
        "action-show-labels",
        GridBarAction::Labels,
        grid_shown,
    ),
    // Where the grid panel goes, in every state: the grid bar's options menu opens with
    // nothing shown too.
    grid(
        ActionId::GRID_POSITION[0],
        "action-grid-position-left",
        GridBarAction::Position(GridPosition::Left),
        grid_anywhere,
    ),
    grid(
        ActionId::GRID_POSITION[1],
        "action-grid-position-right",
        GridBarAction::Position(GridPosition::Right),
        grid_anywhere,
    ),
    grid(
        ActionId::GRID_POSITION[2],
        "action-grid-position-bottom",
        GridBarAction::Position(GridPosition::Bottom),
        grid_anywhere,
    ),
    background(
        ActionId::BACKGROUND[0],
        "action-background-black",
        Background::Black,
    ),
    background(
        ActionId::BACKGROUND[1],
        "action-background-very-dark",
        Background::VeryDark,
    ),
    background(
        ActionId::BACKGROUND[2],
        "action-background-dark",
        Background::Dark,
    ),
    background(
        ActionId::BACKGROUND[3],
        "action-background-medium",
        Background::Medium,
    ),
    background(
        ActionId::BACKGROUND[4],
        "action-background-light",
        Background::Light,
    ),
    background(
        ActionId::BACKGROUND[5],
        "action-background-white",
        Background::White,
    ),
];

/// The action `id`, if the registry has it.
pub fn action(id: ActionId) -> Option<&'static Action> {
    ACTIONS.iter().find(|a| a.id == id)
}

/// Each action's keys: the platform's defaults, with the user's own from settings in
/// their place.
#[derive(Clone, Debug)]
pub struct Keymap {
    os: OperatingSystem,
    keys: Vec<(ActionId, Vec<Shortcut>)>,
    /// What settings keep of the keys: each action's that differ from the defaults, in
    /// [`Shortcut::text`]'s form, and every override [`Keymap::load`] refused, as it was
    /// read.
    overrides: BTreeMap<String, Vec<String>>,
}

impl Keymap {
    /// The default keys on `os`.
    pub fn defaults(os: OperatingSystem) -> Keymap {
        Keymap {
            os,
            keys: ACTIONS
                .iter()
                .map(|a| {
                    let keys = a.keys.iter().filter(|b| b.applies(os));
                    (a.id, keys.map(|b| b.shortcut).collect())
                })
                .collect(),
            overrides: BTreeMap::new(),
        }
    }

    /// The defaults on `os` with the keys settings keep in their place, by action id, and
    /// why any was refused. An override is refused when it names no action, holds text
    /// that is no key, holds a key egui keeps for the clipboard ([`clipboard`]) or one that
    /// opens the ☰'s menu (F10, or Alt with a letter: [`opens_menu`]), or holds a key that one
    /// press can be with another action's where both work: in one scope, or in a view and
    /// the app ([`conflict`]). Two overrides that clash are both refused;
    /// a clash the defaults already have stays, as Ctrl++ with Ctrl+1 on a Czech keyboard.
    /// A refused override leaves its action its default keys. Every override stays in
    /// [`Keymap::overrides`] as it was read, so that a file another platform or version
    /// wrote keeps it until the user changes the action.
    pub fn load(
        os: OperatingSystem,
        overrides: &BTreeMap<String, Vec<String>>,
    ) -> (Keymap, Vec<String>) {
        let defaults = Keymap::defaults(os);
        let mut warnings = Vec::new();
        let mut refused = BTreeMap::new();
        let mut wanted: Vec<(ActionId, Vec<Shortcut>)> = Vec::new();
        for (name, texts) in overrides {
            let Some(action) = ACTIONS.iter().find(|a| a.id.0 == name) else {
                warnings.push(format!("keys.{name}: no action has this id; kept, unused"));
                refused.insert(name.clone(), texts.clone());
                continue;
            };
            match texts
                .iter()
                .map(|text| Shortcut::parse(text).ok_or(text))
                .collect::<Result<Vec<_>, _>>()
            {
                Ok(keys) => match keys
                    .iter()
                    .find_map(|k| kept(action.id, *k, os).map(|why| (k, why)))
                {
                    None => wanted.push((action.id, keys)),
                    Some((key, why)) => {
                        warnings.push(format!(
                            "keys.{name}: {} {why}; default keys kept",
                            key.text(os)
                        ));
                        refused.insert(name.clone(), texts.clone());
                    }
                },
                Err(text) => {
                    warnings.push(format!(
                        "keys.{name}: \"{text}\" is no key; default keys kept"
                    ));
                    refused.insert(name.clone(), texts.clone());
                }
            }
        }
        // Refusing one override gives its action back its defaults, which may clash with
        // another override: refuse until nothing clashes.
        loop {
            let mut keymap = defaults.clone();
            for (id, keys) in &wanted {
                keymap.set(*id, keys.clone());
            }
            let clashes: Vec<(usize, ActionId, Shortcut)> = wanted
                .iter()
                .enumerate()
                .filter_map(|(i, (id, keys))| {
                    clash(&keymap, &defaults, *id, keys).map(|(other, key)| (i, other, key))
                })
                .collect();
            if clashes.is_empty() {
                // As read, not in this platform's reading: a Mac's Ctrl+Meta+F read on
                // Linux stays so until the user changes it.
                for (id, _) in &wanted {
                    keymap
                        .overrides
                        .insert(id.0.to_owned(), overrides[id.0].clone());
                }
                keymap.overrides.extend(refused);
                return (keymap, warnings);
            }
            for (i, other, key) in clashes.into_iter().rev() {
                let (id, _) = wanted.remove(i);
                warnings.push(format!(
                    "keys.{}: {} clashes with {}; default keys kept",
                    id.0,
                    key.text(os),
                    other.0,
                ));
                refused.insert(id.0.to_owned(), overrides[id.0].clone());
            }
        }
    }

    pub fn keys(&self, id: ActionId) -> &[Shortcut] {
        self.keys
            .iter()
            .find(|(a, _)| *a == id)
            .map_or(&[], |(_, keys)| keys.as_slice())
    }

    /// Gives `id` `keys` in place of the ones it has; none leaves it without a key. It
    /// checks nothing: [`conflict`] says whether another action has one of them.
    pub fn set(&mut self, id: ActionId, keys: Vec<Shortcut>) {
        let os = self.os;
        let Some((_, own)) = self.keys.iter_mut().find(|(a, _)| *a == id) else {
            return;
        };
        let texts = |keys: &[Shortcut]| keys.iter().map(|k| k.text(os)).collect::<Vec<_>>();
        if texts(&keys) == texts(Keymap::defaults(os).keys(id)) {
            self.overrides.remove(id.0);
        } else {
            self.overrides.insert(id.0.to_owned(), texts(&keys));
        }
        *own = keys;
    }

    /// What settings keep of the keys: each action's that differ from the defaults, and
    /// every override [`Keymap::load`] refused, as it was read.
    pub fn overrides(&self) -> &BTreeMap<String, Vec<String>> {
        &self.overrides
    }

    /// The first of `id`'s keys, as `os` writes them, for its tooltip.
    pub fn tip(&self, id: ActionId, os: OperatingSystem, tr: &Translator) -> Option<String> {
        self.keys(id).first().map(|s| s.spell(os, tr))
    }
}

/// Another action whose keys in `keymap` one press can share with one of `keys`, `id`'s,
/// where both actions work at once, and that key. A clash the defaults already have does
/// not count.
fn clash(
    keymap: &Keymap,
    defaults: &Keymap,
    id: ActionId,
    keys: &[Shortcut],
) -> Option<(ActionId, Shortcut)> {
    let os = keymap.os;
    let scope = action(id)?.scope;
    let default = |a: ActionId, key: Shortcut| {
        let text = key.text(os);
        defaults.keys(a).iter().any(|d| d.text(os) == text)
    };
    ACTIONS
        .iter()
        .filter(|other| other.id != id && together(scope, other.scope))
        .find_map(|other| {
            keys.iter().find_map(|&key| {
                keymap
                    .keys(other.id)
                    .iter()
                    .any(|&theirs| {
                        conflict(key, theirs, os)
                            && !(default(id, key) && default(other.id, theirs))
                    })
                    .then_some((other.id, key))
            })
        })
}

/// Whether keys of the two scopes work at once: the same scope, or the app and a view,
/// where the view's key would hide the app's.
fn together(a: Scope, b: Scope) -> bool {
    a == b || a == Scope::App || b == Scope::App
}

/// Whether egui-winit turns a press of `key` into a clipboard event on `os`, so that no
/// action ever sees it: the command key alone with C, X or V, the Copy, Cut and Paste
/// keys, and on Windows Ctrl+Insert, Shift+Insert and Shift+Delete with any other keys.
pub fn clipboard(key: Shortcut, os: OperatingSystem) -> bool {
    let m = held(key.modifiers, os == OperatingSystem::Mac);
    let command_only = m.command && !m.alt && !m.shift;
    let windows = os == OperatingSystem::Windows;
    matches!(key.key, Key::Copy | Key::Cut | Key::Paste)
        || (command_only && matches!(key.key, Key::C | Key::X | Key::V))
        || (windows && m.ctrl && key.key == Key::Insert)
        || (windows && m.shift && matches!(key.key, Key::Insert | Key::Delete))
}

/// Whether a press of `key` opens the ☰'s menu at a top-level menu on `os`, and so is kept
/// from every action: on Windows and Linux, Alt alone with a letter, which is a top-level
/// menu's access key or may be one in another language (decision 39).
pub fn opens_menu(key: Shortcut, os: OperatingSystem) -> bool {
    os != OperatingSystem::Mac && alt_only(key.modifiers) && is_letter(key.key)
}

/// Why `id` may not have `key` on `os`, if it is kept from it: the clipboard's keys, and
/// the keys that open the ☰'s menu, Alt with a letter and the menu action's own, F10 on
/// Linux, whatever keys that action is given.
fn kept(id: ActionId, key: Shortcut, os: OperatingSystem) -> Option<&'static str> {
    let menu_key = || {
        let defaults = Keymap::defaults(os);
        id != ActionId::MENU
            && defaults
                .keys(ActionId::MENU)
                .iter()
                .any(|m| conflict(*m, key, os))
    };
    if clipboard(key, os) {
        Some("is the clipboard's")
    } else if opens_menu(key, os) || menu_key() {
        Some("opens the menus")
    } else {
        None
    }
}

fn alt_only(m: Modifiers) -> bool {
    m.alt && !m.shift && !m.ctrl && !m.command && !m.mac_cmd
}

impl Shortcut {
    /// The keys as settings keep them, the same on every platform: `Ctrl+Shift+Z`. Ctrl is
    /// the platform's command key, Cmd on macOS, and Meta a Mac's Control key, as in Qt's
    /// portable form; they come in the order Ctrl, Meta, Alt, Shift, then the key by
    /// egui's name (`Comma`, `Plus`, `F6`, `Up`), so a + never stands for a key. `os` says
    /// what the modifiers mean: off a Mac, egui reports Ctrl as both `ctrl` and `command`.
    pub fn text(self, os: OperatingSystem) -> String {
        let m = self.modifiers;
        let mac = os == OperatingSystem::Mac;
        let names = [
            (m.command || m.mac_cmd || (m.ctrl && !mac), "Ctrl"),
            (m.ctrl && mac, "Meta"),
            (m.alt, "Alt"),
            (m.shift, "Shift"),
        ];
        let mut parts: Vec<&str> = names
            .iter()
            .filter(|(on, _)| *on)
            .map(|(_, name)| *name)
            .collect();
        parts.push(self.key.name());
        parts.join("+")
    }

    /// Reads [`Shortcut::text`]'s form, forgiving what a person types: the modifiers in
    /// any order and case, Cmd for Ctrl and Option for Alt, spaces around the +, the key
    /// by its name in any case or by its symbol (`Ctrl+,`, `Ctrl++`). None when it is not
    /// one key with modifiers egui knows.
    pub fn parse(text: &str) -> Option<Shortcut> {
        let text = text.trim();
        // The key is what follows the last +, or + itself after another (`Ctrl + +`).
        let (names, key) = match text.strip_suffix('+').map(str::trim_end) {
            Some("") => ("", "+"),
            Some(names) if names.ends_with('+') => (&names[..names.len() - 1], "+"),
            _ => text.rsplit_once('+').unwrap_or(("", text)),
        };
        let mut modifiers = Modifiers::NONE;
        for name in names.split('+').map(str::trim).filter(|n| !n.is_empty()) {
            match name.to_ascii_lowercase().as_str() {
                "ctrl" | "cmd" | "command" => modifiers.command = true,
                "meta" => modifiers.ctrl = true,
                "alt" | "option" => modifiers.alt = true,
                "shift" => modifiers.shift = true,
                _ => return None,
            }
        }
        let key = key.trim();
        let key = Key::from_name(key).or_else(|| {
            Key::ALL
                .iter()
                .copied()
                .find(|k| k.name().eq_ignore_ascii_case(key))
        })?;
        Some(Shortcut { modifiers, key })
    }

    /// The keys as the platform writes them: `Ctrl+Shift+Z`, and on macOS `Shift+Cmd+Z`,
    /// with the modifiers in Apple's order. Modifiers are spelt out on macOS too, since
    /// IBM Plex Sans has no ⌘, ⇧, ⌥ or ⌃. Names printed on the keys stay as printed;
    /// Space, printed nowhere, is translated.
    pub fn spell(self, os: OperatingSystem, tr: &Translator) -> String {
        let m = self.modifiers;
        let mac = os == OperatingSystem::Mac;
        let mut parts: Vec<String> = Vec::new();
        let mut add = |on: bool, name: &str| {
            if on {
                parts.push(name.to_owned());
            }
        };
        if mac {
            add(m.ctrl, "Ctrl");
            add(m.alt, "Option");
            add(m.shift, "Shift");
            add(m.command || m.mac_cmd, "Cmd");
        } else {
            add(m.ctrl || m.command, "Ctrl");
            add(m.alt, "Alt");
            add(m.shift, "Shift");
        }
        parts.push(match self.key {
            Key::Space => tr.text("key-space"),
            Key::Backspace if mac => "Delete".to_owned(),
            Key::Delete if mac => "Forward Delete".to_owned(),
            Key::Enter if mac => "Return".to_owned(),
            // egui's own arrows are symbols IBM Plex Sans lacks.
            Key::ArrowLeft => "←".to_owned(),
            Key::ArrowRight => "→".to_owned(),
            Key::ArrowUp => "↑".to_owned(),
            Key::ArrowDown => "↓".to_owned(),
            key => key.symbol_or_name().to_owned(),
        });
        parts.join("+")
    }

    /// Whether a key pressed with `pressed` held is this shortcut. Alt and Shift must
    /// match, except that an extra Shift is allowed on keys whose symbol needs it on many
    /// layouts (+ is Shift and = on a US keyboard). On macOS Cmd and Control are told
    /// apart; elsewhere egui reports Ctrl as both `ctrl` and `command`.
    fn matches(self, key: Key, pressed: Modifiers, mac: bool) -> bool {
        self.key == key && same_modifiers(pressed, self.modifiers, key, mac)
    }
}

fn same_modifiers(pressed: Modifiers, wanted: Modifiers, key: Key, mac: bool) -> bool {
    const SHIFTED: [Key; 7] = [
        Key::Plus,
        Key::Colon,
        Key::Questionmark,
        Key::Exclamationmark,
        Key::Pipe,
        Key::OpenCurlyBracket,
        Key::CloseCurlyBracket,
    ];
    let shift = pressed.shift == wanted.shift || (pressed.shift && SHIFTED.contains(&key));
    // The platform's command key, and a Mac's Control key.
    let commands = |m: Modifiers| {
        if mac {
            (m.command || m.mac_cmd, m.ctrl)
        } else {
            (m.command || m.mac_cmd || m.ctrl, false)
        }
    };
    shift && pressed.alt == wanted.alt && commands(pressed) == commands(wanted)
}

/// What a focused widget is, for the keys it keeps: from the registry's actions, and from
/// the Mac's menus, which hold none of them while it has focus (`menus`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    None,
    /// A button, a slider, a tab: it keeps the keys that move and press.
    Control,
    /// A text field: it keeps every key that types or edits.
    Text,
}

impl Focus {
    pub fn of(ctx: &egui::Context) -> Focus {
        if ctx.text_edit_focused() {
            Focus::Text
        } else if ctx.memory(|m| m.focused().is_some()) {
            Focus::Control
        } else {
            Focus::None
        }
    }

    /// Whether the focused widget keeps `key` for itself: the widget scope, narrower than
    /// any action's.
    pub fn keeps(self, key: Key, modifiers: Modifiers) -> bool {
        // Insert and + add a point to a focused curve.
        const CONTROL: [Key; 16] = [
            Key::ArrowLeft,
            Key::ArrowRight,
            Key::ArrowUp,
            Key::ArrowDown,
            Key::Home,
            Key::End,
            Key::PageUp,
            Key::PageDown,
            Key::Space,
            Key::Enter,
            Key::Tab,
            Key::Escape,
            Key::Backspace,
            Key::Delete,
            Key::Insert,
            Key::Plus,
        ];
        // What a text field does with Ctrl: select all, the clipboard, undo, moving and
        // deleting by word.
        const TEXT: [Key; 14] = [
            Key::A,
            Key::C,
            Key::V,
            Key::X,
            Key::Z,
            Key::Y,
            Key::ArrowLeft,
            Key::ArrowRight,
            Key::ArrowUp,
            Key::ArrowDown,
            Key::Home,
            Key::End,
            Key::Backspace,
            Key::Delete,
        ];
        let command = modifiers.ctrl || modifiers.command || modifiers.mac_cmd;
        match self {
            Focus::None => false,
            Focus::Control => !command && CONTROL.contains(&key),
            Focus::Text if command => TEXT.contains(&key),
            // F6 and F11 still work while typing.
            Focus::Text => !is_function_key(key),
        }
    }
}

fn is_function_key(key: Key) -> bool {
    matches!(key.name().as_bytes(), [b'F', b'1'..=b'9', ..])
}

fn is_letter(key: Key) -> bool {
    matches!(key.name().as_bytes(), [b'A'..=b'Z'])
}

fn is_digit(key: Key) -> bool {
    matches!(key.name().as_bytes(), [b'0'..=b'9'])
}

fn is_symbol(key: Key) -> bool {
    matches!(
        key,
        Key::Colon
            | Key::Comma
            | Key::Minus
            | Key::Period
            | Key::Plus
            | Key::Equals
            | Key::Semicolon
            | Key::Backslash
            | Key::Slash
            | Key::Pipe
            | Key::Questionmark
            | Key::Exclamationmark
            | Key::OpenBracket
            | Key::CloseBracket
            | Key::OpenCurlyBracket
            | Key::CloseCurlyBracket
            | Key::Backtick
            | Key::Quote
    )
}

/// The digit under a symbol a press typed on the number row, as AZERTY types `'` on the 4
/// key.
fn digit_under(key: Key, physical: Option<Key>) -> Option<Key> {
    physical.filter(|p| is_digit(*p) && is_symbol(key))
}

/// Symbols that known layouts type on a digit key without Shift, with the digit: French
/// AZERTY's `'` on 4 and `-` on 6, Belgian AZERTY's `!` on 8, and Czech and Slovak `+` on 1.
/// A key bound to the symbol and one bound to the digit are then one press ([`conflict`]).
const ON_DIGITS: [(Key, Key); 4] = [
    (Key::Quote, Key::Num4),
    (Key::Minus, Key::Num6),
    (Key::Exclamationmark, Key::Num8),
    (Key::Plus, Key::Num1),
];

/// Whether `typed`, what a press of the letter `key` typed, is another key's: a Latin
/// letter not its own, as AltGr's ę, or no letter at all, as @ or €. A letter of another
/// alphabet is the layout's own letter on that key, as у on a Ukrainian keyboard.
fn typed_elsewhere(key: Key, typed: &str) -> bool {
    if typed.to_lowercase() == key.name().to_lowercase() {
        return false;
    }
    !typed.chars().all(|c| c.is_alphabetic() && !is_latin(c))
}

/// Latin letters, accented or not, and the letters Unicode puts before Greek (µ, ª, º).
fn is_latin(c: char) -> bool {
    c < '\u{0370}'
        || matches!(
            c,
            '\u{1E00}'..='\u{1EFF}'
                | '\u{2C60}'..='\u{2C7F}'
                | '\u{A720}'..='\u{A7FF}'
                | '\u{AB30}'..='\u{AB6F}'
                | '\u{FF21}'..='\u{FF3A}'
                | '\u{FF41}'..='\u{FF5A}'
        )
}

/// The modifiers egui reports while `m` is held: off a Mac, Ctrl is both `ctrl` and
/// `command`; on one, Cmd is both `mac_cmd` and `command`.
fn held(m: Modifiers, mac: bool) -> Modifiers {
    let command = m.command || m.mac_cmd || (!mac && m.ctrl);
    Modifiers {
        alt: m.alt,
        shift: m.shift,
        ctrl: if mac { m.ctrl } else { command },
        mac_cmd: mac && command,
        command,
    }
}

/// Whether one press can be both `a` and `b` on `os`, by the rules [`read`] matches with:
/// the same key with modifiers each accepts of the other (on Windows, Ctrl and Command of
/// one letter; Ctrl++ and Ctrl+Shift++), or a symbol a known layout types on the other's
/// digit ([`ON_DIGITS`]), where the digit wins. [`Keymap::load`] refuses an override with
/// it, and rebinding in Settings will warn with it.
pub fn conflict(a: Shortcut, b: Shortcut, os: OperatingSystem) -> bool {
    let mac = os == OperatingSystem::Mac;
    let accepts = |pressed: Shortcut, wanted: Shortcut| {
        same_modifiers(
            held(pressed.modifiers, mac),
            wanted.modifiers,
            wanted.key,
            mac,
        )
    };
    let keys = a.key == b.key
        || ON_DIGITS.contains(&(a.key, b.key))
        || ON_DIGITS.contains(&(b.key, a.key));
    keys && (accepts(a, b) || accepts(b, a))
}

/// What the keyboard asked for this frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Keys {
    /// The actions whose keys were pressed and that can run, in order.
    pub pressed: Vec<ActionId>,
    /// The hold actions whose keys are down and that can run.
    pub held: Vec<ActionId>,
}

/// The scopes active in `state`, narrowest first.
fn scopes(state: &State) -> impl Iterator<Item = Scope> {
    (!state.dialog)
        .then_some(Scope::View(state.view))
        .into_iter()
        .chain([Scope::App])
}

/// The narrowest action `key` belongs to, whether it can run or not.
fn bound(
    keymap: &Keymap,
    state: &State,
    key: Key,
    modifiers: Modifiers,
    mac: bool,
) -> Option<&'static Action> {
    scopes(state).find_map(|scope| {
        ACTIONS.iter().find(|a| {
            a.scope == scope
                && keymap
                    .keys(a.id)
                    .iter()
                    .any(|s| s.matches(key, modifiers, mac))
        })
    })
}

/// A key press as [`read`] meets it.
struct Press<'a> {
    key: Key,
    physical: Option<Key>,
    modifiers: Modifiers,
    /// What it typed: egui-winit sends that right after the press.
    typed: Option<&'a str>,
}

/// The action a press belongs to, whether it can run or not. None when the focused widget
/// keeps the key, or a letter's press typed another key's character (AltGr).
fn press_action(
    keymap: &Keymap,
    state: &State,
    focus: Focus,
    press: &Press<'_>,
    mac: bool,
) -> Option<&'static Action> {
    let foreign =
        is_letter(press.key) && press.typed.is_some_and(|t| typed_elsewhere(press.key, t));
    if focus.keeps(press.key, press.modifiers) || foreign {
        return None;
    }
    digit_under(press.key, press.physical)
        .and_then(|digit| bound(keymap, state, digit, press.modifiers, mac))
        .or_else(|| bound(keymap, state, press.key, press.modifiers, mac))
}

/// Takes out of `events` the presses that belong to an action, with `focus` the focused
/// widget's kind and `mac` the platform's, and returns the actions they asked for that can
/// run in `state`, in order. Keys the focused widget keeps stay, and so does every key no
/// action has.
fn take_presses(
    events: &mut Vec<egui::Event>,
    keymap: &Keymap,
    state: &State,
    focus: Focus,
    mac: bool,
) -> Vec<ActionId> {
    let mut pressed = Vec::new();
    let mut taken = Vec::with_capacity(events.len());
    for (i, event) in events.iter().enumerate() {
        let egui::Event::Key {
            key,
            physical_key,
            pressed: true,
            repeat,
            modifiers,
        } = *event
        else {
            taken.push(false);
            continue;
        };
        let typed = match events.get(i + 1) {
            Some(egui::Event::Text(text)) => Some(text.as_str()),
            _ => None,
        };
        let press = Press {
            key,
            physical: physical_key,
            modifiers,
            typed,
        };
        let found = press_action(keymap, state, focus, &press, mac);
        if let Some(action) = found {
            let fires = match action.trigger {
                Trigger::Press => !repeat,
                Trigger::Repeat => true,
                Trigger::Hold => false,
            };
            if fires && action.can_run(state) {
                pressed.push(action.id);
            }
        }
        taken.push(found.is_some());
    }
    let mut taken = taken.into_iter();
    events.retain(|_| !taken.next().unwrap_or(false));
    pressed
}

/// The actions the keys a dialog's own window left alone ask for (`docs/gui.md`, How a
/// dialog is built now): its window has no keymap, so what it did not take reaches the
/// app's, with `focus` the dialog's focused widget, which keeps its own keys, and `state`
/// the app's with the dialog open, which decides what rests. Hold actions never run from
/// there: they are the view's, which rest under a dialog.
pub fn read_from_dialog(
    events: &[egui::Event],
    focus: Focus,
    os: OperatingSystem,
    keymap: &Keymap,
    state: &State,
) -> Vec<ActionId> {
    let mut events = events.to_vec();
    take_presses(
        &mut events,
        keymap,
        state,
        focus,
        os == OperatingSystem::Mac,
    )
}

/// Reads the frame's keys before anything is drawn: takes out of egui's input the presses
/// that belong to an action, and reports the actions that can run. Keys the focused widget
/// keeps are left to it, and so is every key no action has.
pub fn read(ctx: &egui::Context, keymap: &Keymap, state: &State) -> Keys {
    let focus = Focus::of(ctx);
    let mac = ctx.os() == OperatingSystem::Mac;
    let mut keys = Keys::default();
    ctx.input_mut(|input| {
        keys.pressed = take_presses(&mut input.events, keymap, state, focus, mac);
        let holds = ACTIONS
            .iter()
            .filter(|a| a.trigger == Trigger::Hold && scopes(state).any(|scope| scope == a.scope));
        for action in holds {
            let down = keymap.keys(action.id).iter().any(|s| {
                input.key_down(s.key)
                    && same_modifiers(input.modifiers, s.modifiers, s.key, mac)
                    && !focus.keeps(s.key, input.modifiers)
            });
            if down && action.can_run(state) {
                keys.held.push(action.id);
            }
        }
    });
    keys
}

/// The keys a focused text field undoes or redoes with, egui's own whatever keys the
/// actions have, as a press and its release: what Undo and Redo hand the field when chosen
/// from a menu while it has the focus (decision 39), added to the next frame's input. None
/// for any other action.
pub fn field_keys(id: ActionId, os: OperatingSystem) -> Option<[egui::Event; 2]> {
    let shift = match id {
        ActionId::UNDO => Modifiers::NONE,
        ActionId::REDO => Modifiers::SHIFT,
        _ => return None,
    };
    let modifiers = held(Modifiers::COMMAND.plus(shift), os == OperatingSystem::Mac);
    let key = |pressed| egui::Event::Key {
        key: Key::Z,
        physical_key: Some(Key::Z),
        pressed,
        repeat: false,
        modifiers,
    };
    Some([key(true), key(false)])
}

/// Takes out of the frame's input the first press of Alt with a letter, and the text it
/// typed, if `menu` finds a menu for that letter, and returns what it found: on Linux, Alt
/// with a top-level menu's access key opens the ☰'s menu at that menu, from anywhere
/// (decision 39). The letter is the one the press typed on the current layout, as ф on a
/// Ukrainian keyboard, which egui reports as its physical key, A; where `menu` finds
/// nothing for it, or nothing was typed, the key's own Latin letter, so an English
/// interface's Alt+F opens File on a Cyrillic layout too. Only
/// Alt counts, so AltGr, which egui never sees as Alt, never opens a menu, and a key's
/// repeat opens none again. A press `menu` finds nothing for is left alone. `menu` runs
/// outside egui's lock, so it may read the context.
pub fn access_key<T>(ctx: &egui::Context, mut menu: impl FnMut(char) -> Option<T>) -> Option<T> {
    let (at, typed, own) = ctx.input(|input| {
        input.events.iter().enumerate().find_map(|(i, event)| {
            let egui::Event::Key {
                key,
                pressed: true,
                repeat: false,
                modifiers,
                ..
            } = *event
            else {
                return None;
            };
            if !(alt_only(modifiers) && is_letter(key)) {
                return None;
            }
            let typed = match input.events.get(i + 1) {
                Some(egui::Event::Text(text)) => text.chars().next(),
                _ => None,
            };
            let own = key.name().chars().next()?;
            Some((i, typed, own))
        })
    })?;
    let found = typed.and_then(&mut menu).or_else(|| menu(own))?;
    ctx.input_mut(|input| {
        let end = (at + 1 + usize::from(typed.is_some())).min(input.events.len());
        input.events.drain(at..end);
    });
    Some(found)
}

/// The widget F6 moves the focus to: the first that takes focus in the region after the
/// focused widget's, or before it going back, in `regions`' order (the top panel, the
/// central bar, the side panel, the grid panel). With nothing focused, F6 starts at the
/// first region and Shift+F6 at the last. `widgets` are the focusable widgets in the order
/// egui met them.
pub fn next_region(
    regions: &[egui::Rect],
    widgets: &[(egui::Id, egui::Rect)],
    focused: Option<egui::Id>,
    forward: bool,
) -> Option<egui::Id> {
    let n = regions.len();
    let region_of = |rect: &egui::Rect| regions.iter().position(|r| r.contains(rect.center()));
    let current = focused
        .and_then(|id| widgets.iter().find(|(w, _)| *w == id))
        .and_then(|(_, rect)| region_of(rect));
    (1..=n)
        .map(|k| match (current, forward) {
            (Some(c), true) => (c + k) % n,
            (Some(c), false) => (c + n - k % n) % n,
            (None, true) => k - 1,
            (None, false) => n - k,
        })
        .find_map(|region| {
            widgets
                .iter()
                .find(|(_, rect)| region_of(rect) == Some(region))
                .map(|(id, _)| *id)
        })
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use apertrust_ui::Locale;

    use super::*;

    const PLATFORMS: [OperatingSystem; 3] = [
        OperatingSystem::Mac,
        OperatingSystem::Windows,
        OperatingSystem::Nix,
    ];

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
            background: Some(Background::Dark),
        }
    }

    /// Every pair of default keys of different actions, in `scopes`, that one press can be.
    fn conflicts(os: OperatingSystem, scopes: [Scope; 2]) -> HashSet<(ActionId, ActionId)> {
        let keymap = Keymap::defaults(os);
        let keys: Vec<(ActionId, Scope, Shortcut)> = ACTIONS
            .iter()
            .flat_map(|a| keymap.keys(a.id).iter().map(|k| (a.id, a.scope, *k)))
            .filter(|(_, scope, _)| scopes.contains(scope))
            .collect();
        let mut found = HashSet::new();
        for (i, (a, scope_a, key_a)) in keys.iter().enumerate() {
            for (b, scope_b, key_b) in &keys[i + 1..] {
                let one_scope = scopes[0] == scopes[1] || scope_a != scope_b;
                if a != b && one_scope && conflict(*key_a, *key_b, os) {
                    found.insert((*a, *b));
                }
            }
        }
        found
    }

    #[test]
    fn no_two_default_keys_in_one_scope_are_one_press() {
        // On Czech and Slovak keyboards the 1 key types +: the number row's digit wins, so
        // Ctrl+1 chooses the first tab there, and Ctrl+= zooms in.
        let accepted = HashSet::from([(ActionId::PANEL_TABS[0], ActionId::SCALE_UP)]);
        for os in PLATFORMS {
            for scope in [Scope::App, Scope::View(View::Edit)] {
                let found = conflicts(os, [scope, scope]);
                let expected = if scope == Scope::App {
                    accepted.clone()
                } else {
                    HashSet::new()
                };
                assert_eq!(found, expected, "{os:?} in {scope:?}");
            }
        }
    }

    #[test]
    fn no_view_key_hides_an_app_key_by_default() {
        // The narrowest scope wins, so a view key that is one press with an app key hides it.
        for os in PLATFORMS {
            let found = conflicts(os, [Scope::App, Scope::View(View::Edit)]);
            assert!(found.is_empty(), "{os:?}: {found:?}");
        }
    }

    #[test]
    fn conflicts_follow_the_matching_rules() {
        let key = |modifiers, key| Shortcut { modifiers, key };
        let (windows, mac) = (OperatingSystem::Windows, OperatingSystem::Mac);
        let command = Modifiers::COMMAND;
        let command_shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);
        // Off a Mac, Ctrl and Command are one key.
        let ctrl_e = key(Modifiers::CTRL, Key::E);
        assert!(conflict(ctrl_e, key(command, Key::E), windows));
        assert!(!conflict(ctrl_e, key(command, Key::E), mac));
        // An extra Shift is allowed on +.
        assert!(conflict(
            key(command, Key::Plus),
            key(command_shift, Key::Plus),
            windows
        ));
        assert!(!conflict(
            key(command, Key::E),
            key(command_shift, Key::E),
            windows
        ));
        // AZERTY's ' is on the 4 key; Dvorak's is on Q, which is not read.
        assert!(conflict(
            key(command, Key::Quote),
            key(command, Key::Num4),
            windows
        ));
        assert!(!conflict(
            key(command, Key::Quote),
            key(command, Key::Q),
            windows
        ));
        assert!(!conflict(
            key(command, Key::Comma),
            key(command, Key::Num1),
            windows
        ));
        // Control+Cmd+F, Control+F and Cmd+F are three presses on a Mac.
        let control_command = Modifiers::CTRL.plus(Modifiers::MAC_CMD);
        assert!(!conflict(
            key(control_command, Key::F),
            key(command, Key::F),
            mac
        ));
        assert!(!conflict(
            key(Modifiers::CTRL, Key::F),
            key(command, Key::F),
            mac
        ));
        assert!(!conflict(
            key(Modifiers::CTRL, Key::F),
            key(control_command, Key::F),
            mac
        ));
    }

    #[test]
    fn the_keys_kept_for_later_features_are_free() {
        let kept = [
            Key::Backslash,
            Key::J,
            Key::O,
            Key::W,
            Key::B,
            Key::M,
            Key::S,
            Key::C,
        ];
        for os in PLATFORMS {
            let keymap = Keymap::defaults(os);
            for action in &ACTIONS {
                for key in keymap.keys(action.id) {
                    let plain =
                        key.modifiers == Modifiers::NONE || key.modifiers == Modifiers::SHIFT;
                    assert!(
                        !(plain && kept.contains(&key.key)),
                        "{} takes {:?}, kept for later",
                        action.id.0,
                        key.key
                    );
                }
            }
        }
    }

    #[test]
    fn every_action_s_name_exists_in_every_language() {
        for locale in [Locale::EnGb, Locale::EnUs, Locale::Uk, Locale::Pl] {
            let tr = Translator::new(locale);
            for action in &ACTIONS {
                assert!(
                    tr.has(action.message),
                    "{} lacks {} for {}",
                    locale.id(),
                    action.message,
                    action.id.0
                );
            }
            assert!(tr.has("key-space"));
        }
    }

    #[test]
    fn ids_are_unique() {
        let ids: HashSet<ActionId> = ACTIONS.iter().map(|a| a.id).collect();
        assert_eq!(ids.len(), ACTIONS.len());
    }

    #[test]
    fn actions_not_built_yet_never_run() {
        let state = everything();
        for action in ACTIONS.iter().filter(|a| matches!(a.emits, Emit::Later)) {
            assert!(!action.can_run(&state), "{}", action.id.0);
        }
        assert!(action(ActionId::RESET).is_some_and(|a| a.can_run(&state)));
    }

    #[test]
    fn undo_and_redo_run_with_a_step_to_take_or_a_text_field_and_on_the_main_window() {
        let undo = action(ActionId::UNDO).expect("declared");
        let redo = action(ActionId::REDO).expect("declared");
        let mut state = everything();
        assert_eq!(undo.emits, Emit::Command(Command::Undo));
        assert_eq!(redo.emits, Emit::Command(Command::Redo));
        assert!(undo.enabled(&state) && redo.enabled(&state));
        // Each from its own side of the history.
        state.undo = false;
        assert!(!undo.enabled(&state) && redo.enabled(&state));
        state.redo = false;
        assert!(!redo.enabled(&state));
        // A focused text field's own, whatever the history holds.
        state.text = true;
        assert!(undo.enabled(&state) && redo.enabled(&state));
        // Rest under a modal dialog, and while a dialog's own window has the focus, where
        // the Mac's items hold no key for them (`menus::shown`).
        for rest in [
            State {
                modal: true,
                ..everything()
            },
            State {
                dialog_window: true,
                ..everything()
            },
        ] {
            assert!(!undo.enabled(&rest) && !redo.enabled(&rest));
        }
    }

    #[test]
    fn a_text_field_is_handed_egui_s_own_undo_and_redo_keys() {
        let press = |id, os| field_keys(id, os).map(|[press, _]| press);
        let key = |modifiers| egui::Event::Key {
            key: Key::Z,
            physical_key: Some(Key::Z),
            pressed: true,
            repeat: false,
            modifiers,
        };
        let mac = Modifiers::MAC_CMD.plus(Modifiers::COMMAND);
        let ctrl = Modifiers::CTRL.plus(Modifiers::COMMAND);
        let shift = Modifiers::SHIFT;
        let os = OperatingSystem::Mac;
        assert_eq!(press(ActionId::UNDO, os), Some(key(mac)));
        assert_eq!(press(ActionId::REDO, os), Some(key(mac.plus(shift))));
        let os = OperatingSystem::Windows;
        assert_eq!(press(ActionId::UNDO, os), Some(key(ctrl)));
        assert_eq!(press(ActionId::REDO, os), Some(key(ctrl.plus(shift))));
        assert_eq!(field_keys(ActionId::RESET, os), None);
        // Whatever keys the user gave them: the field's are egui's.
        let [_, release] = field_keys(ActionId::UNDO, os).expect("Undo's");
        assert!(matches!(release, egui::Event::Key { pressed: false, .. }));
    }

    #[test]
    fn a_view_s_action_rests_while_a_dialog_is_open() {
        let mut state = everything();
        state.view = View::Edit;
        let peek = action(ActionId::PEEK).expect("declared");
        let settings = action(ActionId::SETTINGS).expect("declared");
        assert!(peek.enabled(&state) && settings.enabled(&state));
        state.dialog = true;
        assert!(peek.can_run(&state) && !peek.enabled(&state));
        assert!(settings.enabled(&state));
    }

    #[test]
    fn every_action_that_opens_a_dialog_rests_while_a_modal_one_is_open() {
        let mut state = everything();
        let opening = [
            ActionId::ABOUT,
            ActionId::SETTINGS,
            ActionId::OPEN,
            ActionId::OPEN_FOLDER,
            ActionId::EXPORT,
            // Its overwrite question is an alert, which one dialog at a time keeps out.
            ActionId::EXPORT_AGAIN,
        ];
        let enabled = |id, state: &State| action(id).expect("declared").enabled(state);
        assert!(opening.iter().all(|id| enabled(*id, &state)));
        // A sheet, as the dialog host reports it.
        (state.dialog, state.modal) = (true, true);
        for id in opening {
            assert!(!enabled(id, &state), "{id:?} rests under a modal dialog");
        }
        // Close Window too, as AppKit's does under a sheet; Quit still runs: it closes the
        // dialog first.
        assert!(!enabled(ActionId::CLOSE_WINDOW, &state));
        assert!(enabled(ActionId::QUIT, &state));
        // The open dialog and About are modal too, and the view's keys go on under the
        // open dialog, which takes them itself.
        (state.dialog, state.modal) = (false, true);
        for id in opening {
            assert!(!enabled(id, &state), "{id:?} rests under the open dialog");
        }
        // Settings' own window, with the focus beside a modal over the main window: Close
        // Window closes it, as AppKit's acts on the key window.
        (state.dialog, state.dialog_window) = (true, true);
        assert!(enabled(ActionId::CLOSE_WINDOW, &state));
        assert!(!enabled(ActionId::SETTINGS, &state));
    }

    #[test]
    fn while_a_dialog_s_own_window_has_the_focus_the_main_window_s_items_rest() {
        let mut state = everything();
        state.view = View::Edit;
        let enabled = |id, state: &State| action(id).expect("declared").enabled(state);
        let main_window = [
            ActionId::PANEL_TABS[0],
            ActionId::PANEL_TABS[4],
            ActionId::FULL_SCREEN,
            ActionId::RESET,
        ];
        let app_wide = [
            ActionId::ABOUT,
            ActionId::SETTINGS,
            ActionId::OPEN,
            ActionId::OPEN_FOLDER,
            ActionId::SCALE_UP,
            ActionId::SCALE_DOWN,
            ActionId::SCALE_RESET,
            ActionId::CLOSE_WINDOW,
            ActionId::QUIT,
        ];
        for id in main_window.iter().chain(&app_wide) {
            assert!(enabled(*id, &state), "{id:?}");
        }
        // Settings' window on the Mac, focused, as the app reports it: the view's keys
        // rest with it.
        (state.dialog, state.dialog_window) = (true, true);
        for id in main_window {
            assert!(!enabled(id, &state), "{id:?} acts on the main window");
        }
        assert!(!enabled(ActionId::PEEK, &state));
        // The interface's size scales every window; the rest are the app's.
        for id in app_wide {
            assert!(enabled(id, &state), "{id:?} acts on the app");
        }
    }

    #[test]
    fn close_window_takes_cmd_w_on_the_mac_alone() {
        let mac = Keymap::defaults(OperatingSystem::Mac);
        assert_eq!(
            mac.keys(ActionId::CLOSE_WINDOW),
            [Shortcut {
                modifiers: Modifiers::COMMAND,
                key: Key::W,
            }]
        );
        for os in [OperatingSystem::Windows, OperatingSystem::Nix] {
            assert!(Keymap::defaults(os).keys(ActionId::CLOSE_WINDOW).is_empty());
        }
        let close = action(ActionId::CLOSE_WINDOW).expect("declared");
        assert_eq!(close.emits, Emit::Intent(Intent::CloseWindow));
    }

    /// File › Close: Ctrl+W on Windows and Linux, no key of its own on the Mac, where Cmd+W
    /// is Close Window's; enabled with anything open, on the main window with no modal
    /// dialog over it.
    #[test]
    fn close_takes_ctrl_w_off_the_mac_and_needs_something_open() {
        let ctrl_w = Shortcut {
            modifiers: Modifiers::COMMAND,
            key: Key::W,
        };
        for os in [OperatingSystem::Windows, OperatingSystem::Nix] {
            assert_eq!(Keymap::defaults(os).keys(ActionId::CLOSE), [ctrl_w]);
        }
        assert!(
            Keymap::defaults(OperatingSystem::Mac)
                .keys(ActionId::CLOSE)
                .is_empty()
        );
        let close = action(ActionId::CLOSE).expect("declared");
        assert_eq!(close.emits, Emit::Intent(Intent::Close));
        let mut state = everything();
        assert!(close.enabled(&state));
        state.opened = false;
        assert!(!close.enabled(&state), "nothing open");
        state.opened = true;
        state.modal = true;
        assert!(!close.enabled(&state), "under a modal dialog");
        (state.modal, state.dialog, state.dialog_window) = (false, true, true);
        assert!(!close.enabled(&state), "Settings' window has the focus");
    }

    /// Open Recent's items: one per entry listed, each enabled only where its entry can be
    /// opened, a missing one not; Clear Menu with any entry, missing or not.
    #[test]
    fn open_recent_s_items_follow_the_recents() {
        let enabled = |id, state: &State| action(id).expect("declared").enabled(state);
        let mut state = everything();
        state.recents = [None; crate::recents::MOST];
        assert!(!enabled(ActionId::CLEAR_RECENT, &state), "nothing to clear");
        assert!(ActionId::OPEN_RECENT.iter().all(|id| !enabled(*id, &state)));
        state.recents[0] = Some(true);
        state.recents[1] = Some(false);
        assert!(enabled(ActionId::OPEN_RECENT[0], &state));
        assert!(!enabled(ActionId::OPEN_RECENT[1], &state), "missing");
        assert!(!enabled(ActionId::OPEN_RECENT[2], &state), "not listed");
        assert!(enabled(ActionId::CLEAR_RECENT, &state));
        state.modal = true;
        assert!(
            !enabled(ActionId::CLEAR_RECENT, &state),
            "under a modal dialog"
        );
        state.modal = false;
        for (i, id) in ActionId::OPEN_RECENT.iter().enumerate() {
            let recent = action(*id).expect("declared");
            assert_eq!(recent.emits, Emit::Intent(Intent::OpenRecent(i)));
            assert!(recent.keys.is_empty());
            assert_eq!(recent.message, format!("action-open-recent-{}", i + 1));
        }
        // Opening one opens a folder: not under a modal dialog, as Open Folder….
        state.modal = true;
        assert!(!enabled(ActionId::OPEN_RECENT[0], &state));
    }

    #[test]
    fn shortcuts_are_spelt_the_platform_s_way() {
        let en = Translator::new(Locale::EnGb);
        let spell = |id, os| Keymap::defaults(os).tip(id, os, &en);
        let windows = OperatingSystem::Windows;
        let mac = OperatingSystem::Mac;
        assert_eq!(
            spell(ActionId::SETTINGS, windows).as_deref(),
            Some("Ctrl+,")
        );
        assert_eq!(spell(ActionId::SETTINGS, mac).as_deref(), Some("Cmd+,"));
        // Redo as each platform shows it first.
        assert_eq!(spell(ActionId::REDO, windows).as_deref(), Some("Ctrl+Y"));
        assert_eq!(
            spell(ActionId::REDO, OperatingSystem::Nix).as_deref(),
            Some("Ctrl+Shift+Z")
        );
        assert_eq!(spell(ActionId::REDO, mac).as_deref(), Some("Shift+Cmd+Z"));
        assert_eq!(
            spell(ActionId::FULL_SCREEN, mac).as_deref(),
            Some("Ctrl+Cmd+F")
        );
        assert_eq!(
            spell(ActionId::FULL_SCREEN, windows).as_deref(),
            Some("F11")
        );
        assert_eq!(
            spell(ActionId::SCALE_DOWN, windows).as_deref(),
            Some("Ctrl+−")
        );
        assert_eq!(
            spell(ActionId::PREVIOUS_REGION, windows).as_deref(),
            Some("Shift+F6")
        );
        assert_eq!(
            spell(ActionId::PREVIOUS_PHOTO, mac).as_deref(),
            Some("Delete")
        );
        assert_eq!(spell(ActionId::RESET, windows), None);
        let uk = Translator::new(Locale::Uk);
        assert_eq!(
            Keymap::defaults(windows)
                .tip(ActionId::NEXT_PHOTO, windows, &uk)
                .as_deref(),
            Some("Пробіл")
        );
        // Ctrl+Y redoes only where Ctrl+Y is the custom.
        assert_eq!(Keymap::defaults(windows).keys(ActionId::REDO).len(), 2);
        assert_eq!(
            Keymap::defaults(OperatingSystem::Nix)
                .keys(ActionId::REDO)
                .len(),
            2
        );
        assert_eq!(Keymap::defaults(mac).keys(ActionId::REDO).len(), 1);
    }

    #[test]
    fn open_takes_the_command_key_with_o_and_runs_with_no_photo_open() {
        let en = Translator::new(Locale::EnGb);
        for (os, keys) in [
            (OperatingSystem::Windows, "Ctrl+O"),
            (OperatingSystem::Nix, "Ctrl+O"),
            (OperatingSystem::Mac, "Cmd+O"),
        ] {
            let keymap = Keymap::defaults(os);
            assert_eq!(keymap.tip(ActionId::OPEN, os, &en).as_deref(), Some(keys));
            assert_eq!(keymap.keys(ActionId::OPEN).len(), 1, "{os:?}");
        }
        let open = action(ActionId::OPEN).expect("declared");
        assert_eq!(en.text(open.message), "Open…");
        assert_eq!(open.emits, Emit::Intent(Intent::Open));
        let mut state = everything();
        (state.photo, state.edited, state.photos) = (false, false, 0);
        assert!(open.enabled(&state));
        // Over the Settings dialog too.
        state.dialog = true;
        assert!(open.enabled(&state));
    }

    #[test]
    fn export_again_has_no_key_by_default_and_needs_a_photo_and_the_dialog_s_platform() {
        let en = Translator::new(Locale::EnGb);
        for os in PLATFORMS {
            assert!(Keymap::defaults(os).keys(ActionId::EXPORT_AGAIN).is_empty());
        }
        let again = action(ActionId::EXPORT_AGAIN).expect("declared");
        assert_eq!(en.text(again.message), "Export Again");
        assert_eq!(again.emits, Emit::Intent(Intent::ExportAgain));
        let mut state = everything();
        assert!(again.enabled(&state));
        state.photo = false;
        assert!(!again.enabled(&state));
    }

    #[test]
    fn modifiers_match_exactly_but_for_the_shift_a_symbol_needs() {
        let ctrl_e = Shortcut {
            modifiers: Modifiers::COMMAND,
            key: Key::E,
        };
        let windows_ctrl = Modifiers::CTRL.plus(Modifiers::COMMAND);
        let mac_cmd = Modifiers::MAC_CMD.plus(Modifiers::COMMAND);
        assert!(ctrl_e.matches(Key::E, windows_ctrl, false));
        assert!(ctrl_e.matches(Key::E, mac_cmd, true));
        // A Mac's Control is not its Command, and Alt and Shift must match.
        assert!(!ctrl_e.matches(Key::E, Modifiers::CTRL, true));
        assert!(!ctrl_e.matches(Key::E, windows_ctrl.plus(Modifiers::ALT), false));
        assert!(!ctrl_e.matches(Key::E, windows_ctrl.plus(Modifiers::SHIFT), false));
        // Option+L types ¬ on a Mac: not L.
        let l = Shortcut {
            modifiers: Modifiers::NONE,
            key: Key::L,
        };
        assert!(l.matches(Key::L, Modifiers::NONE, true));
        assert!(!l.matches(Key::L, Modifiers::ALT, true));
        // Ctrl++ is Ctrl+Shift+= on a US keyboard.
        let plus = Shortcut {
            modifiers: Modifiers::COMMAND,
            key: Key::Plus,
        };
        assert!(plus.matches(Key::Plus, windows_ctrl.plus(Modifiers::SHIFT), false));
        // Control+Cmd+F on a Mac is not Cmd+F, and Cmd+F is not it.
        let full = Shortcut {
            modifiers: Modifiers::CTRL.plus(Modifiers::MAC_CMD),
            key: Key::F,
        };
        assert!(full.matches(Key::F, mac_cmd.plus(Modifiers::CTRL), true));
        assert!(!full.matches(Key::F, mac_cmd, true));
        let cmd_f = Shortcut {
            modifiers: Modifiers::COMMAND,
            key: Key::F,
        };
        assert!(!cmd_f.matches(Key::F, mac_cmd.plus(Modifiers::CTRL), true));
    }

    #[test]
    fn a_focused_widget_keeps_its_own_keys() {
        assert!(Focus::Control.keeps(Key::ArrowLeft, Modifiers::NONE));
        assert!(Focus::Control.keeps(Key::Space, Modifiers::NONE));
        assert!(!Focus::Control.keeps(Key::H, Modifiers::NONE));
        assert!(!Focus::Control.keeps(Key::Num1, Modifiers::COMMAND));
        assert!(Focus::Text.keeps(Key::H, Modifiers::NONE));
        assert!(Focus::Text.keeps(Key::Z, Modifiers::COMMAND));
        assert!(!Focus::Text.keeps(Key::Comma, Modifiers::COMMAND));
        assert!(!Focus::Text.keeps(Key::F6, Modifiers::NONE));
        assert!(!Focus::Text.keeps(Key::F11, Modifiers::NONE));
        assert!(!Focus::None.keeps(Key::ArrowLeft, Modifiers::NONE));
    }

    #[test]
    fn a_focused_control_keeps_insert_and_plus_for_the_curve() {
        assert!(Focus::Control.keeps(Key::Insert, Modifiers::NONE));
        assert!(Focus::Control.keeps(Key::Plus, Modifiers::NONE));
        // + needs Shift on a US keyboard.
        assert!(Focus::Control.keeps(Key::Plus, Modifiers::SHIFT));
        // Ctrl++ still scales the interface.
        assert!(!Focus::Control.keeps(Key::Plus, Modifiers::COMMAND));
        // Czech and Slovak keyboards type + on the 1 key: the curve still gets it, and Ctrl
        // with it is still the first tab.
        let state = everything();
        for os in PLATFORMS {
            let keymap = Keymap::defaults(os);
            let mac = os == OperatingSystem::Mac;
            let czech = |modifiers, typed| Press {
                key: Key::Plus,
                physical: Some(Key::Num1),
                modifiers,
                typed,
            };
            let plain = czech(Modifiers::NONE, Some("+"));
            assert!(press_action(&keymap, &state, Focus::Control, &plain, mac).is_none());
            let tab = press_action(
                &keymap,
                &state,
                Focus::Control,
                &czech(Modifiers::COMMAND, None),
                mac,
            );
            assert_eq!(tab.map(|a| a.id), Some(ActionId::PANEL_TABS[0]), "{os:?}");
        }
    }

    #[test]
    fn only_a_digit_under_a_symbol_is_read_from_its_physical_key() {
        // AZERTY: ' on the 4 key.
        assert_eq!(digit_under(Key::Quote, Some(Key::Num4)), Some(Key::Num4));
        // Dvorak: ' where QWERTY has Q.
        assert_eq!(digit_under(Key::Quote, Some(Key::Q)), None);
        // QWERTZ: Z where QWERTY has Y.
        assert_eq!(digit_under(Key::Z, Some(Key::Y)), None);
        assert_eq!(digit_under(Key::Num4, Some(Key::Num4)), None);
        assert_eq!(digit_under(Key::Quote, None), None);
        // The numeric keypad's 4 with Num Lock off is ←, not a symbol.
        assert_eq!(digit_under(Key::ArrowLeft, Some(Key::Num4)), None);
    }

    #[test]
    fn a_letter_typing_another_key_s_character_is_not_that_letter() {
        // AltGr on Polish and German keyboards.
        assert!(typed_elsewhere(Key::E, "ę"));
        assert!(typed_elsewhere(Key::L, "ł"));
        assert!(typed_elsewhere(Key::Q, "@"));
        assert!(typed_elsewhere(Key::E, "€"));
        assert!(typed_elsewhere(Key::M, "µ"));
        // The letter itself, with or without Shift.
        assert!(!typed_elsewhere(Key::E, "e"));
        assert!(!typed_elsewhere(Key::E, "E"));
        // Another alphabet's letter on that key: Ukrainian, Greek, Hebrew.
        assert!(!typed_elsewhere(Key::E, "у"));
        assert!(!typed_elsewhere(Key::E, "ε"));
        assert!(!typed_elsewhere(Key::E, "ק"));
    }

    #[test]
    fn the_scale_steps_in_tenths_within_its_range() {
        assert_eq!(step_scale(1.0, true), 1.1);
        assert_eq!(step_scale(1.1, false), 1.0);
        assert_eq!(step_scale(1.04, true), 1.1);
        assert_eq!(step_scale(3.0, true), 3.0);
        assert_eq!(step_scale(0.5, false), 0.5);
        let mut state = everything();
        state.scale = 3.0;
        assert!(!action(ActionId::SCALE_UP).is_some_and(|a| a.can_run(&state)));
        state.scale = 1.0;
        assert!(!action(ActionId::SCALE_RESET).is_some_and(|a| a.can_run(&state)));
    }

    #[test]
    fn f6_cycles_through_the_regions_that_have_something_to_focus() {
        let rect = |y: f32| egui::Rect::from_min_size(egui::pos2(0.0, y), egui::vec2(100.0, 10.0));
        let regions = [rect(0.0), rect(10.0), rect(20.0), rect(30.0)];
        let widget = |n: u64, y: f32| {
            (
                egui::Id::unique(n),
                egui::Rect::from_min_size(egui::pos2(5.0, y + 2.0), egui::vec2(5.0, 5.0)),
            )
        };
        // Nothing to focus in the central bar (region 1).
        let widgets = [
            widget(1, 0.0),
            widget(2, 0.0),
            widget(3, 20.0),
            widget(4, 30.0),
        ];
        let id = egui::Id::unique;
        assert_eq!(next_region(&regions, &widgets, None, true), Some(id(1)));
        assert_eq!(next_region(&regions, &widgets, None, false), Some(id(4)));
        assert_eq!(
            next_region(&regions, &widgets, Some(id(2)), true),
            Some(id(3))
        );
        assert_eq!(
            next_region(&regions, &widgets, Some(id(3)), true),
            Some(id(4))
        );
        assert_eq!(
            next_region(&regions, &widgets, Some(id(4)), true),
            Some(id(1))
        );
        assert_eq!(
            next_region(&regions, &widgets, Some(id(3)), false),
            Some(id(1))
        );
        assert_eq!(
            next_region(&regions, &widgets, Some(id(1)), false),
            Some(id(4))
        );
        assert_eq!(next_region(&regions, &[], None, true), None);
    }

    fn key(modifiers: Modifiers, key: Key) -> Shortcut {
        Shortcut { modifiers, key }
    }

    fn overrides(entries: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        entries
            .iter()
            .map(|(id, keys)| (id.to_string(), keys.iter().map(|k| k.to_string()).collect()))
            .collect()
    }

    #[test]
    fn every_key_round_trips_through_the_text_form_on_every_platform() {
        let command = Modifiers::COMMAND;
        let sets = [
            Modifiers::NONE,
            Modifiers::SHIFT,
            Modifiers::ALT,
            Modifiers::ALT.plus(Modifiers::SHIFT),
            command,
            command.plus(Modifiers::SHIFT),
            command.plus(Modifiers::ALT),
            command.plus(Modifiers::ALT).plus(Modifiers::SHIFT),
            // As egui reports Ctrl off a Mac, and Cmd on one.
            Modifiers::CTRL.plus(command),
            Modifiers::MAC_CMD.plus(command),
            // A Mac's Control key, alone and with Cmd: Control+Cmd+F.
            Modifiers::CTRL,
            Modifiers::CTRL.plus(Modifiers::MAC_CMD),
            Modifiers::CTRL.plus(Modifiers::SHIFT),
        ];
        for os in PLATFORMS {
            for &k in Key::ALL {
                for modifiers in sets {
                    let shortcut = key(modifiers, k);
                    let text = shortcut.text(os);
                    let back = Shortcut::parse(&text).unwrap_or_else(|| panic!("{text}"));
                    assert_eq!(back.text(os), text, "{os:?}");
                    // The same press, whatever egui's own form.
                    assert!(conflict(shortcut, back, os), "{os:?}: {text}");
                }
            }
        }
    }

    #[test]
    fn the_text_form_is_the_same_on_every_platform() {
        let redo = key(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::Z);
        for os in PLATFORMS {
            assert_eq!(redo.text(os), "Ctrl+Shift+Z");
            assert_eq!(key(Modifiers::COMMAND, Key::Comma).text(os), "Ctrl+Comma");
            assert_eq!(key(Modifiers::COMMAND, Key::Plus).text(os), "Ctrl+Plus");
            assert_eq!(key(Modifiers::NONE, Key::F6).text(os), "F6");
        }
        let full_screen = key(Modifiers::CTRL.plus(Modifiers::MAC_CMD), Key::F);
        assert_eq!(full_screen.text(OperatingSystem::Mac), "Ctrl+Meta+F");
        // Off a Mac, egui's Ctrl is Ctrl.
        let ctrl_e = key(Modifiers::CTRL.plus(Modifiers::COMMAND), Key::E);
        assert_eq!(ctrl_e.text(OperatingSystem::Windows), "Ctrl+E");
        assert_eq!(ctrl_e.text(OperatingSystem::Nix), "Ctrl+E");
    }

    #[test]
    fn the_text_form_forgives_what_a_person_types() {
        let command_shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);
        for (text, expected) in [
            ("Ctrl+Shift+Z", Some(key(command_shift, Key::Z))),
            ("shift + ctrl + z", Some(key(command_shift, Key::Z))),
            ("Cmd+Shift+Z", Some(key(command_shift, Key::Z))),
            ("Ctrl+,", Some(key(Modifiers::COMMAND, Key::Comma))),
            ("Ctrl++", Some(key(Modifiers::COMMAND, Key::Plus))),
            ("Ctrl + +", Some(key(Modifiers::COMMAND, Key::Plus))),
            ("Ctrl+Shift++", Some(key(command_shift, Key::Plus))),
            ("+", Some(key(Modifiers::NONE, Key::Plus))),
            ("Option+pageup", Some(key(Modifiers::ALT, Key::PageUp))),
            (
                "Meta+Ctrl+F",
                Some(key(Modifiers::CTRL.plus(Modifiers::COMMAND), Key::F)),
            ),
            ("f11", Some(key(Modifiers::NONE, Key::F11))),
            ("", None),
            ("Ctrl+", None),
            ("Hyper+Z", None),
            ("Ctrl+Shift+Zed", None),
        ] {
            assert_eq!(Shortcut::parse(text), expected, "{text:?}");
        }
    }

    #[test]
    fn overrides_round_trip_through_the_text_form_on_every_platform() {
        let shift_ctrl = Modifiers::COMMAND.plus(Modifiers::SHIFT);
        for os in PLATFORMS {
            let mut keymap = Keymap::defaults(os);
            keymap.set(ActionId::UNDO, vec![key(Modifiers::COMMAND, Key::K)]);
            keymap.set(
                ActionId::REDO,
                vec![key(shift_ctrl, Key::K), key(shift_ctrl, Key::Z)],
            );
            keymap.set(ActionId::QUIT, Vec::new());
            keymap.set(ActionId::FULL_SCREEN, vec![key(Modifiers::NONE, Key::F12)]);
            // Set back to its defaults, an action has no override.
            let settings = keymap.keys(ActionId::SETTINGS).to_vec();
            keymap.set(ActionId::SETTINGS, settings);
            assert_eq!(
                keymap.overrides().keys().collect::<Vec<_>>(),
                ["full-screen", "quit", "redo", "undo"],
                "{os:?}"
            );
            assert_eq!(keymap.overrides()["redo"], ["Ctrl+Shift+K", "Ctrl+Shift+Z"]);
            let (loaded, warnings) = Keymap::load(os, keymap.overrides());
            assert!(warnings.is_empty(), "{os:?}: {warnings:?}");
            assert_eq!(loaded.overrides(), keymap.overrides(), "{os:?}");
            for action in &ACTIONS {
                assert_eq!(
                    loaded.keys(action.id),
                    keymap.keys(action.id),
                    "{os:?}: {}",
                    action.id.0
                );
            }
        }
    }

    #[test]
    fn an_override_that_clashes_in_its_scope_is_refused() {
        let os = OperatingSystem::Windows;
        // Ctrl+Q quits; H peeks in Edit, where the app's keys work too.
        let wanted = overrides(&[("settings", &["Ctrl+Q"]), ("import", &["H"])]);
        let (keymap, warnings) = Keymap::load(os, &wanted);
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert!(warnings[0].contains("quit"), "{}", warnings[0]);
        assert_eq!(
            keymap.keys(ActionId::SETTINGS),
            [key(Modifiers::COMMAND, Key::Comma)]
        );
        assert_eq!(
            keymap.keys(ActionId::IMPORT),
            [key(Modifiers::COMMAND, Key::I)]
        );
        // Kept as written, for a file another platform shares.
        assert_eq!(keymap.overrides(), &wanted);

        // Two overrides on one press are both refused.
        let wanted = overrides(&[("undo", &["Ctrl+K"]), ("redo", &["Ctrl+K"])]);
        let (keymap, warnings) = Keymap::load(os, &wanted);
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert_eq!(
            keymap.keys(ActionId::UNDO),
            Keymap::defaults(os).keys(ActionId::UNDO)
        );

        // Taking a key another override frees is no clash.
        let wanted = overrides(&[("undo", &["Ctrl+Y"]), ("redo", &["Ctrl+Shift+Z"])]);
        let (keymap, warnings) = Keymap::load(os, &wanted);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(
            keymap.keys(ActionId::UNDO),
            [key(Modifiers::COMMAND, Key::Y)]
        );

        // Nor is a clash the defaults have: Ctrl++ with Ctrl+1 on Czech keyboards.
        let wanted = overrides(&[("scale-up", &["Ctrl+Plus", "Ctrl+K"])]);
        let (keymap, warnings) = Keymap::load(os, &wanted);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(keymap.keys(ActionId::SCALE_UP).len(), 2);

        // A refused override's action keeps its defaults, which another override then
        // clashes with: both are refused.
        let wanted = overrides(&[("settings", &["Ctrl+Q"]), ("export", &["Ctrl+Comma"])]);
        let (keymap, warnings) = Keymap::load(os, &wanted);
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert_eq!(
            keymap.keys(ActionId::EXPORT),
            [key(Modifiers::COMMAND, Key::E)]
        );

        // A view's keys work with the app's.
        let edit = Scope::View(View::Edit);
        assert!(together(edit, Scope::App) && together(Scope::App, edit));
    }

    #[test]
    fn an_override_is_kept_as_it_was_read() {
        // A Mac's Control+Cmd+F, read on Linux as Ctrl+F, and a person's own spelling.
        let wanted = overrides(&[("full-screen", &["Ctrl+Meta+F"]), ("undo", &["ctrl + k"])]);
        let (keymap, warnings) = Keymap::load(OperatingSystem::Nix, &wanted);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(keymap.overrides(), &wanted);
        let mut keymap = keymap;
        keymap.set(ActionId::UNDO, vec![key(Modifiers::COMMAND, Key::J)]);
        assert_eq!(keymap.overrides()["undo"], ["Ctrl+J"]);
        assert_eq!(keymap.overrides()["full-screen"], ["Ctrl+Meta+F"]);
    }

    #[test]
    fn a_clipboard_key_is_refused_since_no_action_ever_sees_it() {
        let windows = OperatingSystem::Windows;
        for text in ["Ctrl+C", "Ctrl+X", "Ctrl+V", "Copy"] {
            for os in PLATFORMS {
                let shortcut = Shortcut::parse(text).expect("a key");
                assert!(clipboard(shortcut, os), "{text} on {os:?}");
            }
        }
        for text in [
            "Ctrl+Insert",
            "Shift+Insert",
            "Shift+Delete",
            "Ctrl+Shift+Delete",
        ] {
            let shortcut = Shortcut::parse(text).expect("a key");
            assert!(clipboard(shortcut, windows), "{text}");
            assert!(!clipboard(shortcut, OperatingSystem::Nix), "{text}");
        }
        // Copy edit's Ctrl+Shift+C is no clipboard key, nor is any default.
        assert!(!clipboard(
            key(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::C),
            windows
        ));
        for os in PLATFORMS {
            let keymap = Keymap::defaults(os);
            for action in &ACTIONS {
                for &k in keymap.keys(action.id) {
                    assert!(!clipboard(k, os), "{}: {}", action.id.0, k.text(os));
                }
            }
        }
        let wanted = overrides(&[("export", &["Ctrl+X"])]);
        let (keymap, warnings) = Keymap::load(windows, &wanted);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert_eq!(
            keymap.keys(ActionId::EXPORT),
            [key(Modifiers::COMMAND, Key::E)]
        );
    }

    #[test]
    fn an_override_naming_no_action_or_no_key_is_kept_unused() {
        let os = OperatingSystem::Mac;
        let wanted = overrides(&[("teleport", &["Ctrl+T"]), ("undo", &["Ctrl+Nope"])]);
        let (keymap, warnings) = Keymap::load(os, &wanted);
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert_eq!(
            keymap.keys(ActionId::UNDO),
            Keymap::defaults(os).keys(ActionId::UNDO)
        );
        assert_eq!(keymap.overrides(), &wanted);
    }

    #[test]
    fn f10_opens_the_menu() {
        let f10 = Shortcut {
            modifiers: Modifiers::NONE,
            key: Key::F10,
        };
        for os in [OperatingSystem::Windows, OperatingSystem::Nix] {
            assert_eq!(Keymap::defaults(os).keys(ActionId::MENU), [f10], "{os:?}");
        }
        assert!(
            Keymap::defaults(OperatingSystem::Mac)
                .keys(ActionId::MENU)
                .is_empty()
        );
        // Not over a dialog.
        let menu = action(ActionId::MENU).expect("declared");
        let mut state = everything();
        assert!(menu.enabled(&state));
        state.dialog = true;
        assert!(!menu.enabled(&state));
    }

    #[test]
    fn alt_with_a_letter_is_kept_from_actions() {
        let alt = |key| Shortcut {
            modifiers: Modifiers::ALT,
            key,
        };
        let nix = OperatingSystem::Nix;
        assert!(opens_menu(alt(Key::F), nix));
        assert!(opens_menu(alt(Key::Q), nix));
        // Only Alt alone, and only a letter.
        assert!(!opens_menu(alt(Key::F4), nix));
        assert!(!opens_menu(alt(Key::ArrowLeft), nix));
        let alt_shift = Shortcut {
            modifiers: Modifiers::ALT | Modifiers::SHIFT,
            key: Key::F,
        };
        assert!(!opens_menu(alt_shift, nix));
        assert!(opens_menu(alt(Key::F), OperatingSystem::Windows));
        assert!(!opens_menu(alt(Key::F), OperatingSystem::Mac));
        // No default key takes one.
        let keymap = Keymap::defaults(nix);
        for a in &ACTIONS {
            assert!(
                !keymap.keys(a.id).iter().any(|k| opens_menu(*k, nix)),
                "{}",
                a.id.0
            );
        }
        // Nor may the user's.
        let overrides = BTreeMap::from([("settings".to_owned(), vec!["Alt+E".to_owned()])]);
        let (keymap, warnings) = Keymap::load(nix, &overrides);
        assert_eq!(
            warnings,
            ["keys.settings: Alt+E opens the menus; default keys kept"]
        );
        assert_eq!(
            keymap.keys(ActionId::SETTINGS),
            Keymap::defaults(nix).keys(ActionId::SETTINGS)
        );
        let (keymap, warnings) = Keymap::load(OperatingSystem::Mac, &overrides);
        assert!(warnings.is_empty());
        assert_eq!(keymap.keys(ActionId::SETTINGS), [alt(Key::E)]);
    }

    #[test]
    fn f10_is_kept_for_the_menu() {
        let f10 = Shortcut {
            modifiers: Modifiers::NONE,
            key: Key::F10,
        };
        let nix = OperatingSystem::Nix;
        let overrides = BTreeMap::from([("full-screen".to_owned(), vec!["F10".to_owned()])]);
        let (keymap, warnings) = Keymap::load(nix, &overrides);
        assert_eq!(
            warnings,
            ["keys.full-screen: F10 opens the menus; default keys kept"]
        );
        assert_eq!(
            keymap.keys(ActionId::FULL_SCREEN),
            Keymap::defaults(nix).keys(ActionId::FULL_SCREEN)
        );
        // Even once the menu has other keys.
        let overrides = BTreeMap::from([
            ("menu".to_owned(), vec!["F9".to_owned()]),
            ("full-screen".to_owned(), vec!["F10".to_owned()]),
        ]);
        let (keymap, warnings) = Keymap::load(nix, &overrides);
        assert_eq!(
            warnings,
            ["keys.full-screen: F10 opens the menus; default keys kept"]
        );
        assert_eq!(keymap.keys(ActionId::MENU)[0].key, Key::F9);
        // The menu's own may be F10 again; Windows keeps it too, and on the Mac it is free.
        let overrides = BTreeMap::from([("menu".to_owned(), vec!["F10".to_owned()])]);
        assert!(Keymap::load(nix, &overrides).1.is_empty());
        let overrides = BTreeMap::from([("full-screen".to_owned(), vec!["F10".to_owned()])]);
        let (_, warnings) = Keymap::load(OperatingSystem::Windows, &overrides);
        assert_eq!(
            warnings,
            ["keys.full-screen: F10 opens the menus; default keys kept"]
        );
        let (keymap, warnings) = Keymap::load(OperatingSystem::Mac, &overrides);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(keymap.keys(ActionId::FULL_SCREEN), [f10]);
    }

    /// What [`access_key`] finds for `events`, with a menu for each of `letters`, and the
    /// events it leaves.
    fn access(events: Vec<egui::Event>, letters: &str) -> (Option<char>, Vec<egui::Event>) {
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            events,
            ..Default::default()
        };
        let mut found = None;
        let mut left = Vec::new();
        let mut output = ctx.run_ui(input, |ui| {
            let ctx = ui.ctx();
            found = access_key(ctx, |letter| {
                let letter = letter.to_lowercase().next()?;
                letters.contains(letter).then_some(letter)
            });
            left = ctx.input(|i| i.events.clone());
        });
        // No painter takes the font atlas.
        output.textures_delta.clear();
        (found, left)
    }

    fn key_event(key: Key, modifiers: Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    #[test]
    fn alt_with_an_access_key_is_read_by_the_letter_it_typed() {
        let text = |t: &str| egui::Event::Text(t.to_owned());
        let alt_f = key_event(Key::F, Modifiers::ALT);
        // Taken, with what it typed, so a focused field types nothing.
        let (found, left) = access(vec![alt_f.clone(), text("f")], "fe");
        assert_eq!(found, Some('f'));
        assert!(left.is_empty(), "{left:?}");
        // Without its text, by the key's own letter.
        assert_eq!(access(vec![alt_f.clone()], "f").0, Some('f'));
        // A Ukrainian layout's ф, on the key egui reports as A.
        let alt_a = key_event(Key::A, Modifiers::ALT);
        assert_eq!(access(vec![alt_a.clone(), text("ф")], "фa").0, Some('ф'));
        // On a Cyrillic layout with an English interface, the key's own Latin letter.
        let (found, left) = access(vec![alt_f.clone(), text("а")], "f");
        assert_eq!(found, Some('f'));
        assert!(left.is_empty(), "{left:?}");
        assert_eq!(access(vec![alt_a, text("ф")], "e").0, None);
        // A letter no menu has is left alone.
        let (found, left) = access(vec![alt_f.clone(), text("f")], "e");
        assert_eq!(found, None);
        assert_eq!(left.len(), 2);
        // Only Alt alone: neither with Ctrl or Shift, nor AltGr, nor a plain letter.
        for modifiers in [
            Modifiers::ALT | Modifiers::COMMAND,
            Modifiers::ALT | Modifiers::SHIFT,
            Modifiers::NONE,
        ] {
            let found = access(vec![key_event(Key::F, modifiers), text("f")], "f").0;
            assert_eq!(found, None, "{modifiers:?}");
        }
        assert_eq!(
            access(vec![key_event(Key::F4, Modifiers::ALT)], "f").0,
            None
        );
    }
}
