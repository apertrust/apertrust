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
//! session or an intent for the interface. The Toolbar's buttons, the Tools panel's tabs,
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
//! Shift must match, so Option letters on a Mac never fire a shortcut, but for an extra
//! Shift on a symbol that needs it on many layouts; a shortcut naming Shift exactly wins
//! over that allowance, so Ctrl++ on a US keyboard, which is Ctrl+Shift+=, zooms every
//! photo, and Ctrl+= one.
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

use apertrust_engine::session::{Command, SelectionCommand};
use apertrust_tools::{ProcessingDefinition, Row};
use apertrust_ui::components::{
    BrowserBarAction, BrowserPosition, SortKey, ThumbnailSize, ToolsPanelPosition,
};
use apertrust_ui::guides::{Guide, GuideFlip};
use apertrust_ui::{Background, Icon, Translator};
use eframe::egui::{self, Key, Modifiers, os::OperatingSystem};

use crate::zoom::{Direction, Scale, ZoomStep};

/// An action's id, stable so that settings can keep the keys a user gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ActionId(pub &'static str);

impl ActionId {
    pub const ABOUT: ActionId = ActionId("about");
    /// Help › Show Log: the log window.
    pub const SHOW_LOG: ActionId = ActionId("show-log");
    /// View › Jobs: the jobs window.
    pub const SHOW_JOBS: ActionId = ActionId("show-jobs");
    /// A job's row's menu in the jobs window (decision 91): Cancel, `CancelJob` as the row's
    /// ✕ sends; Show in Folder, its first file written shown in the platform's file manager
    /// as the row's Show in Finder does; Copy Error, its failure copied as the row says it.
    /// Each acts on the job whose menu chose it ([`State::job`]).
    pub const JOB_CANCEL: ActionId = ActionId("job-cancel");
    pub const JOB_SHOW_IN_FOLDER: ActionId = ActionId("job-show-in-folder");
    pub const JOB_COPY_ERROR: ActionId = ActionId("job-copy-error");
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
    /// Edit › Discard History… and the History section's ⋯ item: asks, then
    /// `Command::DiscardHistory` (decision 78).
    pub const DISCARD_HISTORY: ActionId = ActionId("discard-history");
    /// The History section's ⋯ items (decision 93): `Command::Compress`, `Command::Truncate`,
    /// and Create a Style…, which waits for styles (Phase 3).
    pub const COMPRESS_HISTORY: ActionId = ActionId("compress-history");
    pub const TRUNCATE_HISTORY: ActionId = ActionId("truncate-history");
    pub const CREATE_STYLE: ActionId = ActionId("create-style");
    /// A History step's menu (decision 91): Go to Step, `Command::GoTo` of the step; Show
    /// Tool, its tool's section revealed, as Shift-click on it does; Compress up to Here and
    /// Truncate after Here, `Command::Compress` and `Command::Truncate` with the step. Each
    /// acts on the step whose menu chose it ([`State::step`]).
    pub const STEP_GO_TO: ActionId = ActionId("history-step-go-to");
    pub const STEP_SHOW_TOOL: ActionId = ActionId("history-step-show-tool");
    pub const STEP_COMPRESS: ActionId = ActionId("history-step-compress");
    pub const STEP_TRUNCATE: ActionId = ActionId("history-step-truncate");
    pub const UNDO: ActionId = ActionId("undo");
    pub const REDO: ActionId = ActionId("redo");
    pub const COPY_EDIT: ActionId = ActionId("copy-edit");
    pub const APPLY_EDIT: ActionId = ActionId("apply-edit");
    pub const COPY_APPLY_PARTS: ActionId = ActionId("copy-apply-parts");
    /// The Select menu's items, darktable's select all, select none, invert selection and
    /// select untouched: the session's `SelectionCommand`s `All`, `Clear`, `Invert` and
    /// `Unedited` (decision 37; `selection::edit`).
    pub const SELECT_ALL: ActionId = ActionId("select-all");
    pub const SELECT_NONE: ActionId = ActionId("select-none");
    pub const INVERT_SELECTION: ActionId = ActionId("invert-selection");
    pub const SELECT_UNEDITED: ActionId = ActionId("select-unedited");
    /// Select › Previous Selection: the selection as it was before its last change, one
    /// level back (decision 90); drawn disabled until Phase 3.
    pub const PREVIOUS_SELECTION: ActionId = ActionId("previous-selection");
    pub const EDIT_VIEW: ActionId = ActionId("view-edit");
    /// View › Collections: the first tab holding Collections.
    pub const SHOW_COLLECTIONS: ActionId = ActionId("show-collections");
    /// The Tools panel's tabs by position, whatever the workspace puts in them: one for
    /// each tab a workspace shows, which a compile-time assert beside the app's
    /// `layout::MOST_TABS` holds them to (decision 96).
    pub const PANEL_TABS: [ActionId; 12] = [
        ActionId("panel-tab-1"),
        ActionId("panel-tab-2"),
        ActionId("panel-tab-3"),
        ActionId("panel-tab-4"),
        ActionId("panel-tab-5"),
        ActionId("panel-tab-6"),
        ActionId("panel-tab-7"),
        ActionId("panel-tab-8"),
        ActionId("panel-tab-9"),
        ActionId("panel-tab-10"),
        ActionId("panel-tab-11"),
        ActionId("panel-tab-12"),
    ];
    /// The Tools panel's Hidden tab, chosen while it shows, after the twelve (decision 96).
    pub const PANEL_TAB_HIDDEN: ActionId = ActionId("panel-tab-hidden");
    /// Find Tool…, in the Tools panel's ⋮: the tab bar becomes a search field and the tools
    /// found take the tab's place, or, open, the search closes (decision 96).
    pub const FIND_TOOL: ActionId = ActionId("find-tool");
    /// Show Hidden Tools, in the Tools panel's ⋮: the Hidden tab shown last and chosen, or
    /// taken away (decision 96).
    pub const SHOW_HIDDEN_TOOLS: ActionId = ActionId("tools-panel-show-hidden");
    /// A section's layout (decisions 83 and 96), each acting on the section whose header has
    /// the focus, or in ⋯ on its own: Pin Tool or Unpin Tool, in ⋯.
    pub const SECTION_PIN: ActionId = ActionId("section-pin");
    /// Move Tool Up and Move Tool Down, one place in its list, as Alt+↑ and Alt+↓ on its
    /// header do; in no menu.
    pub const SECTION_MOVE_UP: ActionId = ActionId("section-move-up");
    pub const SECTION_MOVE_DOWN: ActionId = ActionId("section-move-down");
    /// Move Tool to Tab ▸'s items by the tab's position, or Show Tool in Tab ▸'s for a
    /// hidden tool, in ⋯.
    pub const SECTION_MOVE_TO_TAB: [ActionId; 12] = [
        ActionId("section-move-to-tab-1"),
        ActionId("section-move-to-tab-2"),
        ActionId("section-move-to-tab-3"),
        ActionId("section-move-to-tab-4"),
        ActionId("section-move-to-tab-5"),
        ActionId("section-move-to-tab-6"),
        ActionId("section-move-to-tab-7"),
        ActionId("section-move-to-tab-8"),
        ActionId("section-move-to-tab-9"),
        ActionId("section-move-to-tab-10"),
        ActionId("section-move-to-tab-11"),
        ActionId("section-move-to-tab-12"),
    ];
    /// Remove Tool, in ⋯: out of every tab into `hidden`, its edit staying.
    pub const SECTION_HIDE: ActionId = ActionId("section-hide");
    /// Show Tool in Tab, for keys and screen readers: a hidden tool to its first default tab
    /// shown.
    pub const SECTION_SHOW: ActionId = ActionId("section-show");
    /// A tool's ⋯'s Reset (decision 91): its header's ↺, run there by the header
    /// (`tools::pressed`) and enabled as ↺ is, never by the registry, so no key runs it; a
    /// tool's reset from the keys is decision 98's.
    pub const SECTION_RESET: ActionId = ActionId("section-reset");
    /// A processing tool's ⋯'s Copy Settings and Paste Settings (decision 91), drawn
    /// disabled until they are built.
    pub const SECTION_COPY_SETTINGS: ActionId = ActionId("section-copy-settings");
    pub const SECTION_PASTE_SETTINGS: ActionId = ActionId("section-paste-settings");
    pub const SCALE_UP: ActionId = ActionId("scale-up");
    pub const SCALE_DOWN: ActionId = ActionId("scale-down");
    pub const SCALE_RESET: ActionId = ActionId("scale-reset");
    pub const NEXT_REGION: ActionId = ActionId("next-region");
    pub const PREVIOUS_REGION: ActionId = ActionId("previous-region");
    /// The menu button's menu, where the menus are drawn (decision 39).
    pub const MENU: ActionId = ActionId("menu");
    /// The Toolbar's words under its icons, on or off (decision 92), in View › Panels and
    /// the toolbar menu.
    pub const TOOLBAR_LABELS: ActionId = ActionId("toolbar-labels");
    /// The Customise Toolbar window, drawn disabled until it is built (decision 92).
    pub const CUSTOMISE_TOOLBAR: ActionId = ActionId("customise-toolbar");
    /// The toolbar menu's Hide "X", taking the item its menu opened on out of the
    /// workspace's list (decision 92).
    pub const TOOLBAR_HIDE: ActionId = ActionId("toolbar-hide");
    /// The menu button put back at the Toolbar's start while it is hidden, on Windows and
    /// Linux (decision 92).
    pub const SHOW_MENU_BUTTON: ActionId = ActionId("show-menu-button");
    pub const FULL_SCREEN: ActionId = ActionId("full-screen");
    /// File › Close: the folder or photo open.
    pub const CLOSE: ActionId = ActionId("close");
    /// File › Close Window, the Mac's alone: the key window, a dialog's or the main one.
    pub const CLOSE_WINDOW: ActionId = ActionId("close-window");
    pub const QUIT: ActionId = ActionId("quit");
    /// Quits as Quit does and starts the app again with what is open (decision 82):
    /// Settings' Restart now, in no menu yet.
    pub const RESTART: ActionId = ActionId("restart");
    pub const PEEK: ActionId = ActionId("peek");
    /// The cursors (decision 104): Select, chosen by Esc where nothing focused keeps it,
    /// and Pan, borrowed while A is held; a tool's are made as the app starts
    /// ([`cursor_actions`]).
    pub const CURSOR_SELECT: ActionId = ActionId("cursor-select");
    pub const CURSOR_PAN: ActionId = ActionId("cursor-pan");
    pub const NEXT_PHOTO: ActionId = ActionId("next-photo");
    pub const PREVIOUS_PHOTO: ActionId = ActionId("previous-photo");
    /// The photo's zoom, View's items and the viewer bar's zoom menu's: in, out, Fit and
    /// 100%.
    pub const ZOOM_IN: ActionId = ActionId("zoom-in");
    pub const ZOOM_OUT: ActionId = ActionId("zoom-out");
    pub const ZOOM_FIT: ActionId = ActionId("zoom-fit");
    pub const ZOOM_100: ActionId = ActionId("zoom-100");
    /// The zoom menu's other scales, with no default keys.
    pub const ZOOM_SMALL: ActionId = ActionId("zoom-small");
    pub const ZOOM_FILL: ActionId = ActionId("zoom-fill");
    pub const ZOOM_50: ActionId = ActionId("zoom-50");
    pub const ZOOM_200: ActionId = ActionId("zoom-200");
    pub const ZOOM_400: ActionId = ActionId("zoom-400");
    pub const ZOOM_800: ActionId = ActionId("zoom-800");
    pub const ZOOM_1600: ActionId = ActionId("zoom-1600");
    /// The Shift forms, every photo shown together (Phase 3's cells); with one photo they
    /// do what the plain ones do.
    pub const ZOOM_IN_ALL: ActionId = ActionId("zoom-in-all");
    pub const ZOOM_OUT_ALL: ActionId = ActionId("zoom-out-all");
    pub const ZOOM_FIT_ALL: ActionId = ActionId("zoom-fit-all");
    pub const ZOOM_100_ALL: ActionId = ActionId("zoom-100-all");
    /// Alt with the arrows, Option on the Mac: a zoomed photo's view moved a fifth of the
    /// part shown, and with Shift a whole one (`docs/gui.md`, Panning), in no menu.
    pub const PAN_LEFT: ActionId = ActionId("pan-left");
    pub const PAN_RIGHT: ActionId = ActionId("pan-right");
    pub const PAN_UP: ActionId = ActionId("pan-up");
    pub const PAN_DOWN: ActionId = ActionId("pan-down");
    pub const PAN_PAGE_LEFT: ActionId = ActionId("pan-page-left");
    pub const PAN_PAGE_RIGHT: ActionId = ActionId("pan-page-right");
    pub const PAN_PAGE_UP: ActionId = ActionId("pan-page-up");
    pub const PAN_PAGE_DOWN: ActionId = ActionId("pan-page-down");
    /// The browser bar's Sort menu and View › Sort By: the key, then the direction.
    pub const SORT: [ActionId; 4] = [
        ActionId("sort-capture-time"),
        ActionId("sort-file-name"),
        ActionId("sort-edited"),
        ActionId("sort-rating"),
    ];
    pub const SORT_ASCENDING: ActionId = ActionId("sort-ascending");
    pub const SORT_DESCENDING: ActionId = ActionId("sort-descending");
    /// The browser bar's Zoom menu and View › Thumbnail Size, smallest first.
    pub const THUMBNAILS: [ActionId; 4] = [
        ActionId("thumbnails-small"),
        ActionId("thumbnails-medium"),
        ActionId("thumbnails-large"),
        ActionId("thumbnails-extra-large"),
    ];
    /// The browser bar's Labels and View › Show Labels: the thumbnails' captions.
    pub const SHOW_LABELS: ActionId = ActionId("show-labels");
    /// The browser bar's options menu and View › Browser Position: left, right, bottom.
    pub const BROWSER_POSITION: [ActionId; 3] = [
        ActionId("browser-position-left"),
        ActionId("browser-position-right"),
        ActionId("browser-position-bottom"),
    ];
    /// The Tools panel's ⋮ and View › Tools Panel Position: left, right.
    pub const TOOLS_PANEL_POSITION: [ActionId; 2] = [
        ActionId("tools-panel-position-left"),
        ActionId("tools-panel-position-right"),
    ];
    /// View › Background and the Viewer's right-click menu, black to white.
    pub const BACKGROUND: [ActionId; 6] = [
        ActionId("background-black"),
        ActionId("background-very-dark"),
        ActionId("background-dark"),
        ActionId("background-medium"),
        ActionId("background-light"),
        ActionId("background-white"),
    ];
    /// Guides, on or off: darktable's guide lines over the photo, the type and flip
    /// `guides.type` and `guides.flip` say (decision 92).
    pub const GUIDES: ActionId = ActionId("guides");
    /// View › Guides' types, darktable's ten in its order, each chosen and the guides
    /// turned on.
    pub const GUIDE_TYPES: [ActionId; 10] = [
        ActionId("guides-grid"),
        ActionId("guides-rules-of-thirds"),
        ActionId("guides-metering"),
        ActionId("guides-perspective"),
        ActionId("guides-diagonal-method"),
        ActionId("guides-harmonious-triangles"),
        ActionId("guides-golden-sections"),
        ActionId("guides-golden-spiral"),
        ActionId("guides-golden-spiral-sections"),
        ActionId("guides-golden-mean"),
    ];
    /// View › Guides › Flip's choices, darktable's four.
    pub const GUIDE_FLIPS: [ActionId; 4] = [
        ActionId("guides-flip-none"),
        ActionId("guides-flip-horizontally"),
        ActionId("guides-flip-vertically"),
        ActionId("guides-flip-both"),
    ];
}

/// The old views enum, from before the one view (decision 37), to become the
/// Viewer's layouts: only Edit is left.
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
#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// Export…: reveals the Export tool in the Photos tab (`export`).
    Export,
    /// The export with the settings as last kept, with no dialog (`export_again`).
    ExportAgain,
    /// Next Photo, or Previous Photo where false: the active photo moves through the
    /// selection, or the folder or the photos opened alone (`selection::step`).
    Step {
        next: bool,
    },
    /// Discard History, asked first unless the setting says not to (`alerts`).
    DiscardHistory,
    View(View),
    PanelTab(usize),
    /// The Hidden tab chosen, while it shows (decision 96).
    HiddenTab,
    /// Show Hidden Tools turned on, the Hidden tab shown last and chosen, or off, it gone
    /// and the tab shown before it shown again (decision 96).
    ShowHiddenTools,
    /// Find Tool… opened, the search field in the tab bar's place, or closed, the tab shown
    /// before it as it was (decision 96).
    FindTool,
    /// View › Collections: the first tab holding Collections chosen, nothing where it is
    /// chosen.
    ShowCollections,
    ScaleUp,
    ScaleDown,
    ScaleReset,
    NextRegion,
    PreviousRegion,
    /// The menu button's menu, opened at its top, or closed where it is open.
    Menu,
    /// Toolbar Labels turned on or off, `toolbar.labels`, at once (decision 92).
    ToolbarLabels,
    /// The item the toolbar menu opened on taken out of the workspace's Toolbar, run by
    /// that menu alone (decision 92).
    ToolbarHide,
    /// The menu button put back at the start of the workspace's Toolbar (decision 92).
    ShowMenuButton,
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
    /// The cursor chosen, which decides what a press on the photo does (decision 104):
    /// its key, its face in the well or its line in its group's list. Pan's key is held
    /// alone, read as [`Keys::held`], borrowing Pan while it is down.
    Cursor(crate::cursors::Cursor),
    /// The Browser's sort, its direction, the thumbnails' size or where the panel is, as
    /// the browser bar's menus choose them, or its Labels turned on or off.
    BrowserBar(BrowserBarAction),
    /// Where the Tools panel is, as its ⋮ and View › Tools Panel Position choose it.
    ToolsPanelPosition(ToolsPanelPosition),
    /// The background behind the photo (`docs/gui.md`, The background).
    Background(Background),
    /// The guides over the photo turned on or off (decision 92).
    Guides,
    /// The guide type chosen, `guides.type`, and the guides turned on.
    GuideType(Guide),
    /// The guides' flip chosen, `guides.flip`.
    GuideFlip(GuideFlip),
    /// A move of the photo's zoom (`docs/gui.md`, The zoom).
    Zoom(ZoomStep),
    /// A zoomed photo's view moved in `direction`, a fifth of the part shown, or the whole
    /// of it where `page` (`docs/gui.md`, Panning).
    Pan {
        direction: Direction,
        page: bool,
    },
    /// A tool's own action, run as pressing its button does ([`tool_actions`]).
    ToolAction {
        tool: &'static str,
        action: &'static str,
    },
    /// View › Workspace's item for the workspace of this id: it shown in the Tools panel,
    /// on the tab last shown in it, and kept in settings ([`workspace`]; decision 96).
    Workspace(&'static str),
    /// A move of the section whose header has the focus, or whose ⋯ chose it (decisions
    /// 83 and 96): display order only, never the edit.
    Section(SectionMove),
    /// One of a History step's menu's items, run on the step whose menu chose it (decision
    /// 91; `Shell::step_menu`).
    HistoryStep(StepItem),
    /// One of a job's row's menu's items, run on the job whose menu chose it by the jobs
    /// window (decision 91; `jobs::Window`).
    Job(JobItem),
}

/// What an item of a History step's menu does to its step ([`Intent::HistoryStep`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepItem {
    GoTo,
    ShowTool,
    Compress,
    Truncate,
}

/// The History step a step's menu acts on ([`State::step`]; `history::stepping`): what each
/// of its items can do there.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stepping {
    /// Whether it is the end, where Go to Step changes nothing.
    pub end: bool,
    /// Whether it has a tool the app knows, which Show Tool opens.
    pub tool: bool,
    /// Whether a step after it is the person's own, which Truncate after Here drops.
    pub truncate: bool,
}

/// What an item of a job's row's menu does to its job ([`Intent::Job`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobItem {
    Cancel,
    ShowInFolder,
    CopyError,
}

/// What a job's row's menu can do for its job ([`State::job`]; `jobs::Mirror::items`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JobItems {
    /// Whether it is queued or running and can be stopped, as its ✕ shows.
    pub cancel: bool,
    /// Whether it has written a file, which Show in Folder shows in its folder.
    pub show: bool,
    /// Whether it failed, saying why, which Copy Error copies.
    pub copy: bool,
}

/// What a layout action does to its section ([`Intent::Section`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionMove {
    /// Pins it in its tab, or unpins it.
    Pin,
    Up,
    Down,
    /// Moves it to the tab shown at this position, out of `hidden` for a hidden tool.
    ToTab(usize),
    /// Takes it out of every tab into `hidden`.
    Hide,
    /// Moves a hidden tool to its first default tab shown.
    Show,
}

/// Where the section a layout action acts on is ([`State::section`]): what each of the
/// layout's actions can do to it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Placing {
    /// Whether it is hidden, in no tab shown: only Show Tool in Tab can run.
    pub hidden: bool,
    /// Whether it is pinned in its tab, which names Pin Tool's item Unpin Tool.
    pub pinned: bool,
    /// Whether a tool of its list is above it, and below it.
    pub up: bool,
    pub down: bool,
    /// Whether it can be hidden: every tool but Export (decision 90).
    pub removable: bool,
    /// Whether it can move to each tab shown, by position: one that can be chosen and does
    /// not hold it.
    pub tabs: [bool; 12],
    /// Whether Show Tool in Tab has a tab to move it to.
    pub show: bool,
}

/// What the actions' availability depends on, as the app is this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub view: View,
    pub photo: bool,
    /// Whether the photo asked for, opening or open, is not yet live, its values taking
    /// changes and its frames coming (decision 77), as a photo just made active is: the
    /// photo's and a thumbnail's menus then show an item that cannot run yet disabled, not
    /// hidden (decision 91; `menus::context`).
    pub waiting: bool,
    /// Whether the photo open takes edits: live and not open for viewing only, as a tool's
    /// buttons need (decisions 77 and 91).
    pub editable: bool,
    /// Whether the photo open is open for viewing only (decision 43): a newer Apertrust
    /// wrote its edit, or part of its history cannot be read. Reset rests, as every change
    /// of its edit does.
    pub view_only: bool,
    pub edited: bool,
    /// Whether the open photo's history has something for Undo, and for Redo, to do, and
    /// takes them: live and not open for viewing only (decision 78).
    pub undo: bool,
    pub redo: bool,
    /// Whether Compress History, and Truncate History, have something to do and are taken:
    /// the photo live and not open for viewing only (decision 78; `history::options`).
    pub compress: bool,
    pub truncate: bool,
    /// Whether Discard History has something to do and is taken: the photo live and not
    /// open for viewing only, or open for viewing only because part of its history cannot
    /// be read, which it starts afresh (decision 78; `history::options`).
    pub discard: bool,
    /// Whether the photo open is open for viewing only because part of its history cannot
    /// be read, which Discard History's tip says it starts afresh.
    pub unreadable: bool,
    /// Why Discard History… cannot run where only the platform's missing alert stops it,
    /// Windows' until its task dialog, as its items say ([`why_unavailable`]): a message's
    /// id, `history::discard_why`'s.
    pub discard_why: Option<&'static str>,
    /// Whether the photo open can be exported: one open for viewing only is left out, as the
    /// Export tool leaves it, so File › Export Again rests.
    pub exportable: bool,
    /// Whether a text field of the main window has the focus: Undo and Redo act on it, as
    /// the Mac's Edit menu does (decision 39), whatever the photo's history holds.
    pub text: bool,
    /// What the Select menu's items would change, from the mirror (`selection::selectable`).
    pub select: Selectable,
    /// Whether Next Photo, and Previous Photo, have a photo to go to (`selection::stepped`).
    pub next: bool,
    pub previous: bool,
    /// The photos selected: the active one alone until several can be (decision 91 hides
    /// what acts on one photo alone from a thumbnail's menu while several are).
    pub photos: usize,
    /// How many photos an export takes, which Export… and Export Again name while more
    /// than one (`selection::exported_count`).
    pub exported: usize,
    /// Whether anything is open, a folder or a photo, opening, open or failed: File ›
    /// Close's to close.
    pub opened: bool,
    /// The recents Open Recent lists, newest first, by place: whether each can be opened,
    /// none where the list holds no entry there. One found missing cannot.
    pub recents: [Option<bool>; crate::recents::MOST],
    /// The Tools panel's tabs, by position: whether each can be chosen. One with nothing
    /// behind it yet cannot, so its key is unavailable (decision 38), and nor can one the
    /// workspace does not show.
    pub tabs: [bool; 12],
    /// Whether the Hidden tab shows, and whether it is the tab chosen: None while it does
    /// not show (decision 96).
    pub hidden_tab: Option<bool>,
    /// Whether Show Hidden Tools is on, which its item shows checked.
    pub show_hidden: bool,
    /// The interface's scale: egui's zoom factor.
    pub scale: f32,
    /// Whether the photo's zoom has a scale to go to up, and down, in order of scale
    /// ([`Zoomed::step`](crate::zoom::Zoomed::step)), and whether it is at Fit and at
    /// 100%, which Zoom to Fit's and Zoom to 100%'s items show checked.
    pub zoom_in: bool,
    pub zoom_out: bool,
    pub fitted: bool,
    pub at_100: bool,
    /// Whether the photo is zoomed past Fit, so it overflows the room and its view can move:
    /// the pan actions' state (`docs/gui.md`, Panning).
    pub zoomed: bool,
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
    /// Whether the Toolbar shows its words under its icons, which Toolbar Labels' item shows
    /// checked (decision 92).
    pub toolbar_labels: bool,
    /// The Toolbar's item whose menu is open, by its id in the workspace's list, which Hide
    /// takes out; none elsewhere, and it cannot run (decision 92).
    pub toolbar_item: Option<&'static str>,
    /// Whether the app draws the menus here and the workspace's Toolbar leaves the menu
    /// button out, so Show Menu Button can put it back: on Windows and Linux alone.
    pub menu_button_hidden: bool,
    /// The Tools panel's tab shown, by position, whose item is shown checked.
    pub panel_tab: usize,
    /// The workspace shown, by its action ([`workspace`]), whose item is shown checked.
    pub workspace: ActionId,
    /// Whether `--conf` sets `workspace` for this run: no workspace can be chosen then, as
    /// the settings store refuses the setting's change (decision 43).
    pub workspace_given: bool,
    /// Whether `--conf` sets `tools_panel.show_hidden` for this run: Show Hidden Tools
    /// cannot be turned on or off then, as the settings store refuses the change.
    pub show_hidden_given: bool,
    /// Where the Tools panel is, whose item is shown checked.
    pub panel_position: ToolsPanelPosition,
    /// Whether `--conf` sets `tools_panel.position` for this run: the Tools panel cannot
    /// move then, as the settings store refuses the change.
    pub panel_position_given: bool,
    /// Where the section the layout's actions act on is: the one whose header has the
    /// focus, or in ⋯ its own; none where none has, and they cannot run.
    pub section: Option<Placing>,
    /// The History step whose menu is open, which its items act on; none elsewhere, and
    /// they cannot run (decision 91).
    pub step: Option<Stepping>,
    /// The job whose row's menu is being built, which its items act on; none elsewhere, and
    /// they cannot run (decision 91).
    pub job: Option<JobItems>,
    /// How the Browser shows the folder, whose items are shown checked.
    pub browser: crate::settings::BrowserSettings,
    /// Whether the Browser shows photos: its browser bar's items rest without any.
    pub shown: bool,
    /// The background behind the photo, whose item is shown checked; None for a theme's
    /// own surround that is none of the six.
    pub background: Option<Background>,
    /// Whether the guides are on, which Guides shows checked, and the type and flip in use,
    /// whose items are shown checked (decision 92).
    pub guides: bool,
    pub guide: Guide,
    pub guide_flip: GuideFlip,
    /// Whether a widget had the keys as the last frame ended, a focused control, a field,
    /// an open list or Find Tool…'s search: Escape is then theirs, left in the input for
    /// them, egui having taken the focus from them as the frame began (decision 104).
    pub escape_kept: bool,
}

/// Whether each of the Select menu's items would change the selection
/// ([`State::select`]); one that would not is drawn disabled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Selectable {
    /// Select All: a photo of the folder open is not selected.
    pub all: bool,
    /// Select None: a photo other than the active one is selected.
    pub none: bool,
    /// Invert Selection: a folder is open with photos, the session's collection.
    pub invert: bool,
    /// Select Unedited: a photo of the folder open has no edit, as its cell's pencil says.
    pub unedited: bool,
    /// Previous Selection: the session keeps an earlier selection, a folder or photos
    /// opened alone are open, and no Previous Selection sent waits to be told.
    pub previous: bool,
}

/// Whether each of the Tools panel's tabs can be chosen, by position, where the first
/// `shown` can, as tests give [`State::tabs`].
#[cfg(test)]
pub fn first_tabs(shown: usize) -> [bool; 12] {
    std::array::from_fn(|i| i < shown)
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
    /// and the Tools panel's tabs. None for a command.
    pub fn checked(&self, state: &State) -> Option<bool> {
        if self.id == ActionId::ZOOM_FIT {
            return Some(state.fitted);
        }
        if self.id == ActionId::ZOOM_100 {
            return Some(state.at_100);
        }
        match self.emits {
            Emit::Intent(Intent::FullScreen) => Some(state.full_screen),
            Emit::Intent(Intent::ToolbarLabels) => Some(state.toolbar_labels),
            Emit::Intent(Intent::PanelTab(tab)) => {
                Some(state.panel_tab == tab && state.hidden_tab != Some(true))
            }
            Emit::Intent(Intent::HiddenTab) => Some(state.hidden_tab == Some(true)),
            Emit::Intent(Intent::ShowHiddenTools) => Some(state.show_hidden),
            Emit::Intent(Intent::Workspace(_)) => Some(state.workspace == self.id),
            Emit::Intent(Intent::BrowserBar(chosen)) => Some(state.browser.holds(chosen)),
            Emit::Intent(Intent::ToolsPanelPosition(p)) => Some(state.panel_position == p),
            Emit::Intent(Intent::Background(b)) => Some(state.background == Some(b)),
            Emit::Intent(Intent::Guides) => Some(state.guides),
            Emit::Intent(Intent::GuideType(guide)) => Some(state.guide == guide),
            Emit::Intent(Intent::GuideFlip(flip)) => Some(state.guide_flip == flip),
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

/// Whether the photo's zoom can move: a photo open and live (decision 77).
fn zooms(state: &State) -> bool {
    state.photo && !state.waiting
}

/// One of the photo's zoom actions, in Photo.
const fn zoom(
    id: ActionId,
    message: &'static str,
    keys: &'static [Binding],
    trigger: Trigger,
    available: fn(&State) -> bool,
    emits: Emit,
) -> Action {
    Action {
        id,
        message,
        keys,
        scope: Scope::View(View::Edit),
        trigger,
        available,
        emits,
    }
}

/// Whether a zoomed photo's view can move: a photo live and zoomed past Fit.
fn pans(state: &State) -> bool {
    zooms(state) && state.zoomed
}

/// Alt with Shift: a pan action's whole view.
const PAN_PAGE: Modifiers = Modifiers::ALT.plus(Modifiers::SHIFT);

/// One of the pan actions, in Photo, repeating while held.
const fn pan(
    id: ActionId,
    message: &'static str,
    keys: &'static [Binding],
    direction: Direction,
    page: bool,
) -> Action {
    zoom(
        id,
        message,
        keys,
        Trigger::Repeat,
        pans,
        Emit::Intent(Intent::Pan { direction, page }),
    )
}

fn always(_: &State) -> bool {
    true
}

fn never(_: &State) -> bool {
    false
}

/// Whether no modal dialog is open, for an action that opens a dialog.
/// Why the action `id` cannot run in `state`, as its menu item's tooltip says, where the
/// reason is one the person can act on: Discard History… where it would ask and the
/// platform shows no alert; the Tools panel's positions where `--conf` sets it for this
/// run, [`SET_BY_CONF`]; Customise Toolbar… until its window is built; Guides' flips while
/// the guide type is one darktable never flips. The message's id, or none.
pub fn why_unavailable(id: ActionId, state: &State) -> Option<&'static str> {
    if state.panel_position_given && ActionId::TOOLS_PANEL_POSITION.contains(&id) {
        return Some(SET_BY_CONF);
    }
    if id == ActionId::CUSTOMISE_TOOLBAR {
        return Some("toolbar-customise-later");
    }
    if ActionId::GUIDE_FLIPS.contains(&id) && !state.guide.flips() {
        return Some("guides-flip-unavailable");
    }
    state
        .discard_why
        .filter(|_| id == ActionId::DISCARD_HISTORY)
}

/// The message that says `--conf` sets a setting for this run, as a Settings row says it,
/// which names the flag (`utility::overridden`).
pub const SET_BY_CONF: &str = "settings-overridden";

/// What the action `id`'s item says in its tip in a tool's ⋯ (decision 93) and a History
/// step's menu (decision 91), as the pull-down's items do: why it cannot run where
/// something says ([`why_unavailable`]), else what it does, for History's items, or that
/// it comes later, for Copy Settings and Paste Settings. The message's id, or none.
pub fn options_tip(id: ActionId, state: &State) -> Option<&'static str> {
    why_unavailable(id, state).or(match id {
        ActionId::COMPRESS_HISTORY => Some("history-compress-tip"),
        ActionId::TRUNCATE_HISTORY => Some("history-truncate-tip"),
        ActionId::CREATE_STYLE => Some("history-options-style-later"),
        ActionId::SECTION_COPY_SETTINGS | ActionId::SECTION_PASTE_SETTINGS => {
            Some("section-options-later")
        }
        ActionId::STEP_GO_TO => Some("history-step-go-to-tip"),
        ActionId::STEP_SHOW_TOOL => Some("history-step-show-tool-tip"),
        ActionId::STEP_COMPRESS => Some("history-step-compress-tip"),
        ActionId::STEP_TRUNCATE => Some("history-step-truncate-tip"),
        ActionId::DISCARD_HISTORY if state.unreadable => Some("history-discard-tip-unreadable"),
        ActionId::DISCARD_HISTORY => Some("history-discard-tip"),
        _ => None,
    })
}

/// Whether `id` is one of the layout's actions, which move a section (decisions 83 and
/// 96): run on the section whose ⋯ chose it, never through the edit.
pub fn moves_section(id: ActionId) -> bool {
    action(id).is_some_and(|a| matches!(a.emits, Emit::Intent(Intent::Section(_))))
}

/// The words of the layout's action `id` for a section where `placing` says it is: Pin
/// Tool's are Unpin Tool's while it is pinned; the others' are their own.
pub fn section_words(id: ActionId, placing: Placing) -> &'static str {
    match action(id) {
        Some(_) if id == ActionId::SECTION_PIN && placing.pinned => "action-section-unpin",
        Some(action) => action.message,
        None => "",
    }
}

fn no_modal_dialog(state: &State) -> bool {
    !state.modal
}

/// Whether the main window is the one the keys and the menus act on, for an action that
/// acts on it or its photo: not while a dialog's own window has the focus.
fn on_main_window(state: &State) -> bool {
    !state.dialog_window
}

/// Whether the browser bar's menus can be used: the Browser shows photos, on the main
/// window, with no modal dialog over it.
fn browser_shown(state: &State) -> bool {
    state.shown && on_main_window(state) && !state.modal
}

/// Whether the Browser can be moved: on the main window, with no modal dialog over it,
/// whether it shows photos or not, as its options menu opens in every state.
fn browser_anywhere(state: &State) -> bool {
    on_main_window(state) && !state.modal
}

/// The browser bar's choice `chosen` as an action, which can run where `available` says: no
/// keys by default, its words the action's own message, `action-<id>`.
const fn browser_bar(
    id: ActionId,
    message: &'static str,
    chosen: BrowserBarAction,
    available: fn(&State) -> bool,
) -> Action {
    Action {
        id,
        message,
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available,
        emits: Emit::Intent(Intent::BrowserBar(chosen)),
    }
}

/// Whether the Tools panel can be moved: as the Browser can, unless `--conf` sets where it
/// is for this run.
fn tools_panel_movable(state: &State) -> bool {
    !state.panel_position_given && browser_anywhere(state)
}

/// The Tools panel's place `chosen` as an action, no keys by default, its words
/// `action-<id>`.
const fn tools_panel_position(
    id: ActionId,
    message: &'static str,
    chosen: ToolsPanelPosition,
) -> Action {
    Action {
        id,
        message,
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: tools_panel_movable,
        emits: Emit::Intent(Intent::ToolsPanelPosition(chosen)),
    }
}

/// The background `chosen` as an action: no keys by default, on the main window with no
/// modal dialog over it, in every state, as the Viewer's menu opens with no photo.
const fn background(id: ActionId, message: &'static str, chosen: Background) -> Action {
    Action {
        id,
        message,
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: browser_anywhere,
        emits: Emit::Intent(Intent::Background(chosen)),
    }
}

/// Whether the guides can be turned on, or their type changed: a photo open and live, as
/// the photo's zoom (decision 77).
fn guided(state: &State) -> bool {
    zooms(state)
}

/// Whether the guides' flip can change: as [`guided`], and the type in use takes a flip.
fn flipped(state: &State) -> bool {
    guided(state) && state.guide.flips()
}

/// The guide type `chosen` as an action of the view: no keys by default, its words
/// `action-guides-<id>`.
const fn guide_type(id: ActionId, message: &'static str, chosen: Guide) -> Action {
    Action {
        id,
        message,
        keys: &[],
        scope: Scope::View(View::Edit),
        trigger: Trigger::Press,
        available: guided,
        emits: Emit::Intent(Intent::GuideType(chosen)),
    }
}

/// The guides' flip `chosen` as an action of the view: no keys by default, its words
/// `action-guides-flip-<id>`.
const fn guide_flip(id: ActionId, message: &'static str, chosen: GuideFlip) -> Action {
    Action {
        id,
        message,
        keys: &[],
        scope: Scope::View(View::Edit),
        trigger: Trigger::Press,
        available: flipped,
        emits: Emit::Intent(Intent::GuideFlip(chosen)),
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

/// Whether the Tools panel has an `I`th tab, by position, that can be chosen, the main
/// window the one acted on.
fn tab_enabled<const I: usize>(state: &State) -> bool {
    on_main_window(state) && state.tabs.get(I).copied().unwrap_or(false)
}

/// Whether a layout action can run on the section it acts on, `can` saying what it needs
/// of where that is: on the main window, with no modal dialog over it.
fn section_can(state: &State, can: impl Fn(&Placing) -> bool) -> bool {
    on_main_window(state) && no_modal_dialog(state) && state.section.as_ref().is_some_and(can)
}

/// Whether the section can move to the tab shown at position `I`.
fn to_tab_enabled<const I: usize>(state: &State) -> bool {
    section_can(state, |s| s.tabs[I])
}

/// A layout action moving its section to the tab shown at position `I`, Move Tool to Tab
/// `I + 1` in the keymap: no keys by default.
const fn to_tab<const I: usize>(message: &'static str) -> Action {
    Action {
        id: ActionId::SECTION_MOVE_TO_TAB[I],
        message,
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: to_tab_enabled::<I>,
        emits: Emit::Intent(Intent::Section(SectionMove::ToTab(I))),
    }
}

/// Every action, in the order a keyboard map lists them (`docs/gui.md`, The default
/// keyboard map). Slider keys are not here: they are the slider's own (`SliderRow`).
pub static ACTIONS: [Action; 159] = [
    // About Apertrust, in the app menu on the Mac and in Help on Windows and Linux.
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
    // A job's row's menu (decision 91), with no key: each acts on the job whose menu chose
    // it, and none can run elsewhere. The jobs window is modeless beside any dialog, so they
    // ask nothing of the main window.
    Action {
        id: ActionId::JOB_CANCEL,
        message: "jobs-cancel",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| s.job.is_some_and(|job| job.cancel),
        emits: Emit::Intent(Intent::Job(JobItem::Cancel)),
    },
    // Named as the platform's file manager is, as the row's own button: Show in Finder on
    // the Mac.
    Action {
        id: ActionId::JOB_SHOW_IN_FOLDER,
        message: crate::jobs::FILE_MANAGER.show_message(),
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| s.job.is_some_and(|job| job.show),
        emits: Emit::Intent(Intent::Job(JobItem::ShowInFolder)),
    },
    Action {
        id: ActionId::JOB_COPY_ERROR,
        message: "jobs-copy-error",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| s.job.is_some_and(|job| job.copy),
        emits: Emit::Intent(Intent::Job(JobItem::CopyError)),
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
    // Folder mode: the folder's photos fill the Browser.
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
        available: |s| (s.photo || s.exportable) && no_modal_dialog(s),
        emits: Emit::Intent(Intent::Export),
    },
    // darktable's Ctrl+E, on Ctrl+Shift+E (Cmd+Shift+E), Ctrl+E revealing the tool (decision 90).
    Action {
        id: ActionId::EXPORT_AGAIN,
        message: "action-export-again",
        keys: &[command_shift(Key::E)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| s.exportable && no_modal_dialog(s),
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
        available: |s| s.photo && s.edited && !s.view_only && on_main_window(s),
        emits: Emit::Command(Command::Reset),
    },
    // No key by default either, as darktable's discard history has none. It asks first
    // (`ask_before_discard`), an alert, so it rests under a modal dialog as the actions that
    // open one do.
    Action {
        id: ActionId::DISCARD_HISTORY,
        message: "action-discard-history",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| s.photo && s.discard && on_main_window(s) && no_modal_dialog(s),
        emits: Emit::Intent(Intent::DiscardHistory),
    },
    // darktable's compress history stack and its Ctrl-click, which truncates
    // (`src/libs/history.c:1239-1250`): History's ⋯ items, with no key, as darktable's
    // button has none. They rewrite the log, so they rest as Undo does.
    Action {
        id: ActionId::COMPRESS_HISTORY,
        message: "history-options-compress",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| s.compress && on_main_window(s) && !s.modal,
        emits: Emit::Command(Command::Compress { up_to: None }),
    },
    Action {
        id: ActionId::TRUNCATE_HISTORY,
        message: "history-options-truncate",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| s.truncate && on_main_window(s) && !s.modal,
        emits: Emit::Command(Command::Truncate { after: None }),
    },
    // A History step's menu (decision 91), with no key: each acts on the step whose menu
    // chose it, and none can run elsewhere. Go to Step and the rewrites take edits, as a
    // click on a step and History's ⋯ items do; Show Tool opens a tool whatever the photo,
    // as Shift-click on the step does.
    Action {
        id: ActionId::STEP_GO_TO,
        message: "history-step-go-to",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| {
            s.editable && s.step.is_some_and(|step| !step.end) && on_main_window(s) && !s.modal
        },
        emits: Emit::Intent(Intent::HistoryStep(StepItem::GoTo)),
    },
    Action {
        id: ActionId::STEP_SHOW_TOOL,
        message: "history-step-show-tool",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| s.step.is_some_and(|step| step.tool) && on_main_window(s) && !s.modal,
        emits: Emit::Intent(Intent::HistoryStep(StepItem::ShowTool)),
    },
    Action {
        id: ActionId::STEP_COMPRESS,
        message: "history-step-compress",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| s.compress && s.step.is_some() && on_main_window(s) && !s.modal,
        emits: Emit::Intent(Intent::HistoryStep(StepItem::Compress)),
    },
    Action {
        id: ActionId::STEP_TRUNCATE,
        message: "history-step-truncate",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| {
            s.editable && s.step.is_some_and(|step| step.truncate) && on_main_window(s) && !s.modal
        },
        emits: Emit::Intent(Intent::HistoryStep(StepItem::Truncate)),
    },
    // History's ⋯ item, drawn disabled until styles come (Phase 3).
    Action {
        id: ActionId::CREATE_STYLE,
        message: "history-options-style",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: never,
        emits: Emit::Later,
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
    // darktable's Ctrl+A and Ctrl+Shift+A (`src/libs/select.c:142-147`). Select All is an editing
    // item, as Undo is: a text field with the focus keeps Ctrl+A, and run from a menu there
    // it hands the field its own ([`field_keys`]). Select None leaves no active photo, so
    // with the active photo alone selected it would only close it, and rests.
    Action {
        id: ActionId::SELECT_ALL,
        message: "action-select-all",
        keys: &[command(Key::A)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| (s.select.all || s.text) && on_main_window(s) && !s.modal,
        emits: Emit::Command(Command::Selection(SelectionCommand::All)),
    },
    Action {
        id: ActionId::SELECT_NONE,
        message: "action-select-none",
        keys: &[command_shift(Key::A)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| s.select.none && on_main_window(s) && !s.modal,
        emits: Emit::Command(Command::Selection(SelectionCommand::Clear)),
    },
    // Ctrl+Shift+I, since darktable's Ctrl+I (`src/libs/select.c:150-151`) is Import's here
    // (`docs/gui.md`, File), as is its copy & import's Ctrl+Shift+I.
    Action {
        id: ActionId::INVERT_SELECTION,
        message: "action-invert-selection",
        keys: &[command_shift(Key::I)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| s.select.invert && on_main_window(s) && !s.modal,
        emits: Emit::Command(Command::Selection(SelectionCommand::Invert)),
    },
    // No key, as darktable's select untouched has none (`src/libs/select.c:158-159`).
    Action {
        id: ActionId::SELECT_UNEDITED,
        message: "action-select-unedited",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| s.select.unedited && on_main_window(s) && !s.modal,
        emits: Emit::Command(Command::Selection(SelectionCommand::Unedited)),
    },
    // Ctrl+Shift+D (decision 90), darktable's duplicate virgin's key there: the selection
    // as it was before its last change, while the session keeps one.
    Action {
        id: ActionId::PREVIOUS_SELECTION,
        message: "action-previous-selection",
        keys: &[command_shift(Key::D)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| s.select.previous && on_main_window(s) && !s.modal,
        emits: Emit::Command(Command::Selection(SelectionCommand::Previous)),
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
        id: ActionId::SHOW_COLLECTIONS,
        message: "action-show-collections",
        keys: &[plain(Key::L)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: on_main_window,
        emits: Emit::Intent(Intent::ShowCollections),
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
    // No default key for the sixth and seventh: Ctrl+6 is Ctrl+− on French AZERTY
    // keyboards, which would zoom out (`docs/gui.md`, The Tools panel). The settings file's
    // `keys` table can give each one.
    Action {
        id: ActionId::PANEL_TABS[5],
        message: "action-panel-tab-6",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: tab_enabled::<5>,
        emits: Emit::Intent(Intent::PanelTab(5)),
    },
    Action {
        id: ActionId::PANEL_TABS[6],
        message: "action-panel-tab-7",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: tab_enabled::<6>,
        emits: Emit::Intent(Intent::PanelTab(6)),
    },
    // The eighth to the twelfth, for a workspace of more tabs than the Default's seven,
    // none with a default key.
    Action {
        id: ActionId::PANEL_TABS[7],
        message: "action-panel-tab-8",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: tab_enabled::<7>,
        emits: Emit::Intent(Intent::PanelTab(7)),
    },
    Action {
        id: ActionId::PANEL_TABS[8],
        message: "action-panel-tab-9",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: tab_enabled::<8>,
        emits: Emit::Intent(Intent::PanelTab(8)),
    },
    Action {
        id: ActionId::PANEL_TABS[9],
        message: "action-panel-tab-10",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: tab_enabled::<9>,
        emits: Emit::Intent(Intent::PanelTab(9)),
    },
    Action {
        id: ActionId::PANEL_TABS[10],
        message: "action-panel-tab-11",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: tab_enabled::<10>,
        emits: Emit::Intent(Intent::PanelTab(10)),
    },
    Action {
        id: ActionId::PANEL_TABS[11],
        message: "action-panel-tab-12",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: tab_enabled::<11>,
        emits: Emit::Intent(Intent::PanelTab(11)),
    },
    // The Hidden tab, after the twelve, while it shows (decision 96).
    Action {
        id: ActionId::PANEL_TAB_HIDDEN,
        message: "action-panel-tab-hidden",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| on_main_window(s) && s.hidden_tab.is_some(),
        emits: Emit::Intent(Intent::HiddenTab),
    },
    // The Tools panel's ⋮ (decision 96); the tab bar's menu joins it. Ctrl+Shift+F is Find
    // Tool…'s, from a text field too, Ctrl+F being kept for Search Photos….
    Action {
        id: ActionId::FIND_TOOL,
        message: "action-find-tool",
        keys: &[command_shift(Key::F)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: on_main_window,
        emits: Emit::Intent(Intent::FindTool),
    },
    Action {
        id: ActionId::SHOW_HIDDEN_TOOLS,
        message: "action-tools-panel-show-hidden",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| on_main_window(s) && !s.show_hidden_given,
        emits: Emit::Intent(Intent::ShowHiddenTools),
    },
    // A section's layout (decisions 83 and 96), on the section whose header has the focus,
    // none with a default key: Alt+↑ and Alt+↓ are the header's own.
    Action {
        id: ActionId::SECTION_PIN,
        message: "action-section-pin",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| section_can(s, |s| !s.hidden),
        emits: Emit::Intent(Intent::Section(SectionMove::Pin)),
    },
    Action {
        id: ActionId::SECTION_MOVE_UP,
        message: "action-section-move-up",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Repeat,
        available: |s| section_can(s, |s| !s.hidden && s.up),
        emits: Emit::Intent(Intent::Section(SectionMove::Up)),
    },
    Action {
        id: ActionId::SECTION_MOVE_DOWN,
        message: "action-section-move-down",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Repeat,
        available: |s| section_can(s, |s| !s.hidden && s.down),
        emits: Emit::Intent(Intent::Section(SectionMove::Down)),
    },
    to_tab::<0>("action-section-move-to-tab-1"),
    to_tab::<1>("action-section-move-to-tab-2"),
    to_tab::<2>("action-section-move-to-tab-3"),
    to_tab::<3>("action-section-move-to-tab-4"),
    to_tab::<4>("action-section-move-to-tab-5"),
    to_tab::<5>("action-section-move-to-tab-6"),
    to_tab::<6>("action-section-move-to-tab-7"),
    to_tab::<7>("action-section-move-to-tab-8"),
    to_tab::<8>("action-section-move-to-tab-9"),
    to_tab::<9>("action-section-move-to-tab-10"),
    to_tab::<10>("action-section-move-to-tab-11"),
    to_tab::<11>("action-section-move-to-tab-12"),
    Action {
        id: ActionId::SECTION_HIDE,
        message: "action-section-hide",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| section_can(s, |s| !s.hidden && s.removable),
        emits: Emit::Intent(Intent::Section(SectionMove::Hide)),
    },
    Action {
        id: ActionId::SECTION_SHOW,
        message: "action-section-show",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| section_can(s, |s| s.hidden && s.show),
        emits: Emit::Intent(Intent::Section(SectionMove::Show)),
    },
    // A tool's ⋯'s editing items (decision 91): Reset runs in ⋯ alone, as its header's ↺
    // ([`ActionId::SECTION_RESET`]); Copy Settings and Paste Settings wait to be built.
    Action {
        id: ActionId::SECTION_RESET,
        message: "action-section-reset",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: never,
        emits: Emit::Later,
    },
    Action {
        id: ActionId::SECTION_COPY_SETTINGS,
        message: "action-section-copy-settings",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: never,
        emits: Emit::Later,
    },
    Action {
        id: ActionId::SECTION_PASTE_SETTINGS,
        message: "action-section-paste-settings",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: never,
        emits: Emit::Later,
    },
    // No keys: the photo's zoom has them (`docs/gui.md`, The default keyboard map), and the
    // interface's size, set once, keeps View › Interface Size and Settings.
    Action {
        id: ActionId::SCALE_UP,
        message: "action-scale-up",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Repeat,
        available: |s| s.scale < *SCALES.end(),
        emits: Emit::Intent(Intent::ScaleUp),
    },
    Action {
        id: ActionId::SCALE_DOWN,
        message: "action-scale-down",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Repeat,
        available: |s| s.scale > *SCALES.start(),
        emits: Emit::Intent(Intent::ScaleDown),
    },
    Action {
        id: ActionId::SCALE_RESET,
        message: "action-scale-reset",
        keys: &[],
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
    // F10 opens the menu button's menu on Windows and Linux, as GNOME's apps open theirs
    // (decision 39). Its name is the menu button's, and the menu it opens never holds it.
    Action {
        id: ActionId::MENU,
        message: "toolbar-menu",
        keys: &[plain(Key::F10).only(Platforms::NotMac)],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| !s.dialog,
        emits: Emit::Intent(Intent::Menu),
    },
    // The Toolbar's own (decision 92), on the main window with no modal dialog over it.
    Action {
        id: ActionId::TOOLBAR_LABELS,
        message: "action-toolbar-labels",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| on_main_window(s) && !s.modal,
        emits: Emit::Intent(Intent::ToolbarLabels),
    },
    Action {
        id: ActionId::TOOLBAR_HIDE,
        message: "action-toolbar-hide",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| s.toolbar_item.is_some() && on_main_window(s) && !s.modal,
        emits: Emit::Intent(Intent::ToolbarHide),
    },
    Action {
        id: ActionId::SHOW_MENU_BUTTON,
        message: "action-show-menu-button",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: |s| s.menu_button_hidden && on_main_window(s) && !s.modal,
        emits: Emit::Intent(Intent::ShowMenuButton),
    },
    Action {
        id: ActionId::CUSTOMISE_TOOLBAR,
        message: "action-customise-toolbar",
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: never,
        emits: Emit::Later,
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
    // Settings' Restart now, and Try again after a start that did not finish (decision 82):
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
    // The cursors (decision 104; `docs/cursors.md`, Keys): Select on Esc, which a widget
    // that had the keys keeps ([`State::escape_kept`]); and Pan while A is held, as H's
    // peek is. Each while a photo is shown.
    Action {
        id: ActionId::CURSOR_SELECT,
        message: "cursor-select",
        keys: &[plain(Key::Escape)],
        scope: Scope::View(View::Edit),
        trigger: Trigger::Press,
        available: cursor_available,
        emits: Emit::Intent(Intent::Cursor(crate::cursors::Cursor::SELECT)),
    },
    Action {
        id: ActionId::CURSOR_PAN,
        message: "cursor-pan",
        keys: &[plain(Key::A)],
        scope: Scope::View(View::Edit),
        trigger: Trigger::Hold,
        available: cursor_available,
        emits: Emit::Intent(Intent::Cursor(crate::cursors::Cursor::PAN)),
    },
    Action {
        id: ActionId::NEXT_PHOTO,
        message: "action-next-photo",
        keys: &[plain(Key::Space), plain(Key::ArrowRight)],
        scope: Scope::View(View::Edit),
        trigger: Trigger::Repeat,
        available: |s| s.next && on_main_window(s) && !s.modal,
        emits: Emit::Intent(Intent::Step { next: true }),
    },
    Action {
        id: ActionId::PREVIOUS_PHOTO,
        message: "action-previous-photo",
        keys: &[plain(Key::Backspace), plain(Key::ArrowLeft)],
        scope: Scope::View(View::Edit),
        trigger: Trigger::Repeat,
        available: |s| s.previous && on_main_window(s) && !s.modal,
        emits: Emit::Intent(Intent::Step { next: false }),
    },
    // The photo's zoom (`docs/gui.md`, The zoom), on a live photo: every scale of the zoom
    // menu, and the steps in order of scale. Ctrl+= first: on most keyboards + needs
    // Shift, and Ctrl++ matches with it held, unless Ctrl+Shift+= names it exactly.
    zoom(
        ActionId::ZOOM_IN,
        "action-zoom-in",
        &[command(Key::Equals), command(Key::Plus)],
        Trigger::Repeat,
        |s| zooms(s) && s.zoom_in,
        Emit::Intent(Intent::Zoom(ZoomStep::In)),
    ),
    zoom(
        ActionId::ZOOM_OUT,
        "action-zoom-out",
        &[command(Key::Minus)],
        Trigger::Repeat,
        |s| zooms(s) && s.zoom_out,
        Emit::Intent(Intent::Zoom(ZoomStep::Out)),
    ),
    zoom(
        ActionId::ZOOM_FIT,
        "action-zoom-fit",
        &[command(Key::Num0)],
        Trigger::Press,
        zooms,
        Emit::Intent(Intent::Zoom(ZoomStep::Fit)),
    ),
    zoom(
        ActionId::ZOOM_100,
        "action-zoom-100",
        &[plain(Key::Z)],
        Trigger::Press,
        zooms,
        Emit::Intent(Intent::Zoom(ZoomStep::Hundred)),
    ),
    zoom(
        ActionId::ZOOM_SMALL,
        "action-zoom-small",
        &[],
        Trigger::Press,
        zooms,
        Emit::Intent(Intent::Zoom(ZoomStep::To(Scale::Small))),
    ),
    zoom(
        ActionId::ZOOM_FILL,
        "action-zoom-fill",
        &[],
        Trigger::Press,
        zooms,
        Emit::Intent(Intent::Zoom(ZoomStep::To(Scale::Fill))),
    ),
    zoom(
        ActionId::ZOOM_50,
        "action-zoom-50",
        &[],
        Trigger::Press,
        zooms,
        Emit::Intent(Intent::Zoom(ZoomStep::To(Scale::Factor(0.5)))),
    ),
    zoom(
        ActionId::ZOOM_200,
        "action-zoom-200",
        &[],
        Trigger::Press,
        zooms,
        Emit::Intent(Intent::Zoom(ZoomStep::To(Scale::Factor(2.0)))),
    ),
    zoom(
        ActionId::ZOOM_400,
        "action-zoom-400",
        &[],
        Trigger::Press,
        zooms,
        Emit::Intent(Intent::Zoom(ZoomStep::To(Scale::Factor(4.0)))),
    ),
    zoom(
        ActionId::ZOOM_800,
        "action-zoom-800",
        &[],
        Trigger::Press,
        zooms,
        Emit::Intent(Intent::Zoom(ZoomStep::To(Scale::Factor(8.0)))),
    ),
    zoom(
        ActionId::ZOOM_1600,
        "action-zoom-1600",
        &[],
        Trigger::Press,
        zooms,
        Emit::Intent(Intent::Zoom(ZoomStep::To(Scale::Factor(16.0)))),
    ),
    // With Shift, every photo shown together; with one, as the plain ones.
    zoom(
        ActionId::ZOOM_IN_ALL,
        "action-zoom-in-all",
        &[command_shift(Key::Equals)],
        Trigger::Repeat,
        |s| zooms(s) && s.zoom_in,
        Emit::Intent(Intent::Zoom(ZoomStep::In)),
    ),
    zoom(
        ActionId::ZOOM_OUT_ALL,
        "action-zoom-out-all",
        &[command_shift(Key::Minus)],
        Trigger::Repeat,
        |s| zooms(s) && s.zoom_out,
        Emit::Intent(Intent::Zoom(ZoomStep::Out)),
    ),
    zoom(
        ActionId::ZOOM_FIT_ALL,
        "action-zoom-fit-all",
        &[command_shift(Key::Num0)],
        Trigger::Press,
        zooms,
        Emit::Intent(Intent::Zoom(ZoomStep::Fit)),
    ),
    zoom(
        ActionId::ZOOM_100_ALL,
        "action-zoom-100-all",
        &[bind(Modifiers::SHIFT, Key::Z)],
        Trigger::Press,
        zooms,
        Emit::Intent(Intent::Zoom(ZoomStep::Hundred)),
    ),
    // Alt with the arrows move a zoomed photo's view, Shift a whole view at a time
    // (`docs/gui.md`, Panning); the arrows alone stay Next and Previous Photo.
    pan(
        ActionId::PAN_LEFT,
        "action-pan-left",
        &[bind(Modifiers::ALT, Key::ArrowLeft)],
        Direction::Left,
        false,
    ),
    pan(
        ActionId::PAN_RIGHT,
        "action-pan-right",
        &[bind(Modifiers::ALT, Key::ArrowRight)],
        Direction::Right,
        false,
    ),
    pan(
        ActionId::PAN_UP,
        "action-pan-up",
        &[bind(Modifiers::ALT, Key::ArrowUp)],
        Direction::Up,
        false,
    ),
    pan(
        ActionId::PAN_DOWN,
        "action-pan-down",
        &[bind(Modifiers::ALT, Key::ArrowDown)],
        Direction::Down,
        false,
    ),
    pan(
        ActionId::PAN_PAGE_LEFT,
        "action-pan-page-left",
        &[bind(PAN_PAGE, Key::ArrowLeft)],
        Direction::Left,
        true,
    ),
    pan(
        ActionId::PAN_PAGE_RIGHT,
        "action-pan-page-right",
        &[bind(PAN_PAGE, Key::ArrowRight)],
        Direction::Right,
        true,
    ),
    pan(
        ActionId::PAN_PAGE_UP,
        "action-pan-page-up",
        &[bind(PAN_PAGE, Key::ArrowUp)],
        Direction::Up,
        true,
    ),
    pan(
        ActionId::PAN_PAGE_DOWN,
        "action-pan-page-down",
        &[bind(PAN_PAGE, Key::ArrowDown)],
        Direction::Down,
        true,
    ),
    // The browser bar's menus and View's Sort By and Thumbnail Size (`docs/gui.md`, The
    // browser bar); no keys by default, so + and − stay free. Rating comes with the
    // ratings.
    browser_bar(
        ActionId::SORT[0],
        "action-sort-capture-time",
        BrowserBarAction::Sort(SortKey::CaptureTime),
        browser_shown,
    ),
    browser_bar(
        ActionId::SORT[1],
        "action-sort-file-name",
        BrowserBarAction::Sort(SortKey::FileName),
        browser_shown,
    ),
    browser_bar(
        ActionId::SORT[2],
        "action-sort-edited",
        BrowserBarAction::Sort(SortKey::Edited),
        browser_shown,
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
    browser_bar(
        ActionId::SORT_ASCENDING,
        "action-sort-ascending",
        BrowserBarAction::Descending(false),
        browser_shown,
    ),
    browser_bar(
        ActionId::SORT_DESCENDING,
        "action-sort-descending",
        BrowserBarAction::Descending(true),
        browser_shown,
    ),
    browser_bar(
        ActionId::THUMBNAILS[0],
        "action-thumbnails-small",
        BrowserBarAction::Size(ThumbnailSize::Small),
        browser_shown,
    ),
    browser_bar(
        ActionId::THUMBNAILS[1],
        "action-thumbnails-medium",
        BrowserBarAction::Size(ThumbnailSize::Medium),
        browser_shown,
    ),
    browser_bar(
        ActionId::THUMBNAILS[2],
        "action-thumbnails-large",
        BrowserBarAction::Size(ThumbnailSize::Large),
        browser_shown,
    ),
    browser_bar(
        ActionId::THUMBNAILS[3],
        "action-thumbnails-extra-large",
        BrowserBarAction::Size(ThumbnailSize::ExtraLarge),
        browser_shown,
    ),
    // The browser bar's Labels, a toggle shown checked while the captions show.
    browser_bar(
        ActionId::SHOW_LABELS,
        "action-show-labels",
        BrowserBarAction::Labels,
        browser_shown,
    ),
    // Where the Browser goes, in every state: the browser bar's options menu opens with
    // nothing shown too.
    browser_bar(
        ActionId::BROWSER_POSITION[0],
        "action-browser-position-left",
        BrowserBarAction::Position(BrowserPosition::Left),
        browser_anywhere,
    ),
    browser_bar(
        ActionId::BROWSER_POSITION[1],
        "action-browser-position-right",
        BrowserBarAction::Position(BrowserPosition::Right),
        browser_anywhere,
    ),
    browser_bar(
        ActionId::BROWSER_POSITION[2],
        "action-browser-position-bottom",
        BrowserBarAction::Position(BrowserPosition::Bottom),
        browser_anywhere,
    ),
    // Where the Tools panel goes, in every state, as the Browser.
    tools_panel_position(
        ActionId::TOOLS_PANEL_POSITION[0],
        "action-tools-panel-position-left",
        ToolsPanelPosition::Left,
    ),
    tools_panel_position(
        ActionId::TOOLS_PANEL_POSITION[1],
        "action-tools-panel-position-right",
        ToolsPanelPosition::Right,
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
    // View › Guides: Guides on G, darktable's key, then the types and the flips.
    Action {
        id: ActionId::GUIDES,
        message: "action-guides",
        keys: &[plain(Key::G)],
        scope: Scope::View(View::Edit),
        trigger: Trigger::Press,
        available: guided,
        emits: Emit::Intent(Intent::Guides),
    },
    guide_type(ActionId::GUIDE_TYPES[0], "action-guides-grid", Guide::Grid),
    guide_type(
        ActionId::GUIDE_TYPES[1],
        "action-guides-rules-of-thirds",
        Guide::RulesOfThirds,
    ),
    guide_type(
        ActionId::GUIDE_TYPES[2],
        "action-guides-metering",
        Guide::Metering,
    ),
    guide_type(
        ActionId::GUIDE_TYPES[3],
        "action-guides-perspective",
        Guide::Perspective,
    ),
    guide_type(
        ActionId::GUIDE_TYPES[4],
        "action-guides-diagonal-method",
        Guide::DiagonalMethod,
    ),
    guide_type(
        ActionId::GUIDE_TYPES[5],
        "action-guides-harmonious-triangles",
        Guide::HarmoniousTriangles,
    ),
    guide_type(
        ActionId::GUIDE_TYPES[6],
        "action-guides-golden-sections",
        Guide::GoldenSections,
    ),
    guide_type(
        ActionId::GUIDE_TYPES[7],
        "action-guides-golden-spiral",
        Guide::GoldenSpiral,
    ),
    guide_type(
        ActionId::GUIDE_TYPES[8],
        "action-guides-golden-spiral-sections",
        Guide::GoldenSpiralSections,
    ),
    guide_type(
        ActionId::GUIDE_TYPES[9],
        "action-guides-golden-mean",
        Guide::GoldenMean,
    ),
    guide_flip(
        ActionId::GUIDE_FLIPS[0],
        "action-guides-flip-none",
        GuideFlip::None,
    ),
    guide_flip(
        ActionId::GUIDE_FLIPS[1],
        "action-guides-flip-horizontally",
        GuideFlip::Horizontally,
    ),
    guide_flip(
        ActionId::GUIDE_FLIPS[2],
        "action-guides-flip-vertically",
        GuideFlip::Vertically,
    ),
    guide_flip(
        ActionId::GUIDE_FLIPS[3],
        "action-guides-flip-both",
        GuideFlip::Both,
    ),
];

/// The icons the registry gives the app's own actions, those that belong to no tool, each
/// of which it makes a candidate for the Toolbar (decision 92; `toolbar::placeable`), in
/// the order the Toolbar's set lists them. An action that belongs to a tool takes the icon
/// its tool offers it with (`ToolDefinition::toolbar`); Import… is here until the Import
/// tool offers it.
pub const ICONS: [(ActionId, Icon); 11] = [
    (ActionId::IMPORT, Icon::Import),
    (ActionId::GUIDES, Icon::Guides),
    (ActionId::OPEN, Icon::Photo),
    (ActionId::OPEN_FOLDER, Icon::ChooseFolder),
    (ActionId::SETTINGS, Icon::Settings),
    (ActionId::FULL_SCREEN, Icon::Maximise),
    (ActionId::ZOOM_100, Icon::ZoomIn),
    (ActionId::ZOOM_FIT, Icon::ZoomOut),
    (ActionId::SHOW_JOBS, Icon::Process),
    (ActionId::NEXT_PHOTO, Icon::Chevron),
    (ActionId::PREVIOUS_PHOTO, Icon::Back),
];

/// The action `id`, if the registry has it: one of [`ACTIONS`], a tool's
/// ([`tool_actions`]) or a workspace's ([`workspace_actions`]).
pub fn action(id: ActionId) -> Option<&'static Action> {
    all().find(|a| a.id == id)
}

/// The action whose id reads `id`, if the registry has it, as settings and menus name one.
pub fn named(id: &str) -> Option<&'static Action> {
    all().find(|a| a.id.0 == id)
}

/// Every action: [`ACTIONS`], then the tools' own ([`tool_actions`]), then the tools'
/// cursors' ([`cursor_actions`]), then the workspaces' ([`workspace_actions`]).
pub fn all() -> impl Iterator<Item = &'static Action> {
    ACTIONS
        .iter()
        .chain(tool_actions())
        .chain(cursor_actions())
        .chain(workspace_actions())
}

/// The built-in tools' own actions as registry actions (decision 91), made once as the
/// app starts from their definitions: one for each [`Row::Action`] and each button of a
/// [`Row::Buttons`], `tool-<tool>-<action>`, its words the action's full name
/// (`tool-<tool>-action-<action>-full`), no default keys, in the view's scope. A button's
/// runs as pressing it does ([`Intent::ToolAction`]), while the photo takes edits and no
/// modal dialog is open; a [`Row::Action`]'s, which the panel draws disabled until what it
/// runs is built, is listed and never runs.
pub fn tool_actions() -> &'static [Action] {
    static MADE: std::sync::OnceLock<Vec<Action>> = std::sync::OnceLock::new();
    MADE.get_or_init(|| {
        let leak = |text: String| -> &'static str { Box::leak(text.into_boxed_str()) };
        let mut made = Vec::new();
        for tool in apertrust_tools::TOOLS {
            let definition = tool.definition();
            for (action, button) in own_actions(definition) {
                let [_, full, _] = apertrust_tools::messages::action(definition.tool.id, action);
                let emits = match button {
                    true => Emit::Intent(Intent::ToolAction {
                        tool: definition.tool.id,
                        action,
                    }),
                    false => Emit::Later,
                };
                made.push(Action {
                    id: ActionId(leak(format!("tool-{}-{action}", definition.tool.id))),
                    message: leak(full),
                    keys: &[],
                    scope: Scope::View(View::Edit),
                    trigger: Trigger::Press,
                    available: tool_action_available,
                    emits,
                });
            }
        }
        made
    })
}

/// A tool's own actions, in its panel's order, each with whether it is a button's.
fn own_actions(definition: &'static ProcessingDefinition) -> Vec<(&'static str, bool)> {
    let mut actions = Vec::new();
    for row in definition.rows() {
        match row {
            Row::Action { id } => actions.push((*id, false)),
            Row::Buttons { buttons, .. } => actions.extend(buttons.iter().map(|b| (b.id, true))),
            _ => {}
        }
    }
    actions
}

/// Whether a tool's button can run: the photo takes edits, live and not open for viewing
/// only, on the main window, with no modal dialog over it, as its button is enabled.
fn tool_action_available(state: &State) -> bool {
    state.editable && on_main_window(state) && no_modal_dialog(state)
}

/// The tools' picker cursors' actions (decision 104; `docs/cursors.md`, Actions and words),
/// made once as the app starts from their declarations: `tool-<tool>-cursor-<cursor>`, its
/// words the cursor's name, its key the cursor's own where it declares one, as White
/// balance's K, in the view's scope. Each chooses its cursor while a photo is shown, as
/// its face in the well does.
pub fn cursor_actions() -> &'static [Action] {
    static MADE: std::sync::OnceLock<Vec<Action>> = std::sync::OnceLock::new();
    MADE.get_or_init(|| {
        let leak = |text: String| -> &'static str { Box::leak(text.into_boxed_str()) };
        let mut made = Vec::new();
        for entry in crate::cursors::registry().entries() {
            let crate::cursors::Owner::Tool(tool) = entry.cursor.owner else {
                continue;
            };
            let id = apertrust_tools::messages::cursor(tool, entry.cursor.id);
            let keys: Vec<Binding> = (entry.def.key.and_then(Shortcut::parse))
                .map(|shortcut| Binding {
                    shortcut,
                    on: Platforms::All,
                })
                .into_iter()
                .collect();
            let id = leak(id);
            made.push(Action {
                id: ActionId(id),
                message: id,
                keys: Box::leak(keys.into_boxed_slice()),
                scope: Scope::View(View::Edit),
                trigger: Trigger::Press,
                available: cursor_available,
                emits: Emit::Intent(Intent::Cursor(entry.cursor)),
            });
        }
        made
    })
}

/// A tool's picker cursor's action: `tool-<tool>-cursor-<cursor>`, as made by
/// [`cursor_actions`].
pub fn cursor_action(cursor: crate::cursors::Cursor) -> ActionId {
    let crate::cursors::Owner::Tool(tool) = cursor.owner else {
        return ActionId::CURSOR_SELECT;
    };
    let id = apertrust_tools::messages::cursor(tool, cursor.id);
    (cursor_actions().iter())
        .find(|action| action.id.0 == id)
        .map_or(ActionId::CURSOR_SELECT, |action| action.id)
}

/// Whether a cursor can be chosen: a photo shown, on the main window, with no modal dialog
/// over it, as the well is enabled (`docs/cursors.md`, The well). One that cannot act yet
/// is chosen all the same, and says why.
fn cursor_available(state: &State) -> bool {
    state.photo && on_main_window(state) && no_modal_dialog(state)
}

/// The workspaces' actions made so far ([`workspace`]), the built-in ones first.
static WORKSPACES: std::sync::Mutex<Vec<&'static Action>> = std::sync::Mutex::new(Vec::new());

/// [`WORKSPACES`], the built-in workspaces' actions made where none are yet.
fn workspaces_made() -> std::sync::MutexGuard<'static, Vec<&'static Action>> {
    let mut made = WORKSPACES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if made.is_empty() {
        made.extend(crate::layout::builtin_ids().map(made_for));
    }
    made
}

/// The workspaces' actions (decision 96), one for each workspace, `workspace-<id>`: the
/// built-in ones, Default then Essentials, always, and a person's own once [`workspace`]
/// has made theirs, as the app does as it starts from the workspaces it reads, before
/// the keys settings keep are loaded.
pub fn workspace_actions() -> Vec<&'static Action> {
    workspaces_made().clone()
}

/// The action of the workspace `id`, made the first time it is asked for, as the app asks
/// for each workspace's as it starts, as tools' actions are made (decision 96): the
/// registry's `workspace-<id>`, no default keys, its words a built-in's message,
/// `workspace-<id>`, and for a person's own, which View › Workspace names by its file, the
/// submenu's, `menu-workspace`. It runs on the main window with no modal dialog over it,
/// unless `--conf` sets the workspace for this run.
pub fn workspace(id: &str) -> ActionId {
    let wanted = format!("workspace-{id}");
    let mut made = workspaces_made();
    if let Some(action) = made.iter().find(|a| a.id.0 == wanted) {
        return action.id;
    }
    let action = made_for(id);
    made.push(action);
    action.id
}

/// Whether a workspace can be chosen: on the main window, with no modal dialog over it,
/// unless `--conf` sets the workspace for this run.
fn workspace_available(state: &State) -> bool {
    !state.workspace_given && on_main_window(state) && no_modal_dialog(state)
}

/// The workspace `id`'s action, made for the run.
fn made_for(id: &str) -> &'static Action {
    let leak = |text: String| -> &'static str { Box::leak(text.into_boxed_str()) };
    let action_id = leak(format!("workspace-{id}"));
    let builtin = crate::layout::builtin_ids().any(|built| built == id);
    Box::leak(Box::new(Action {
        id: ActionId(action_id),
        message: if builtin { action_id } else { "menu-workspace" },
        keys: &[],
        scope: Scope::App,
        trigger: Trigger::Press,
        available: workspace_available,
        emits: Emit::Intent(Intent::Workspace(leak(id.to_owned()))),
    }))
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
            keys: all()
                .map(|a| {
                    let keys = a.keys.iter().filter(|b| b.applies(os));
                    (a.id, keys.map(|b| b.shortcut).collect())
                })
                .collect(),
            overrides: BTreeMap::new(),
        }
    }

    /// The defaults on `os` with the keys settings keep in their place, by action id, and
    /// why any was refused. An override is refused when it names no action, holds text that
    /// is no key, holds a key egui keeps for the clipboard ([`clipboard`]) or one that
    /// opens the menu button's menu (F10, or Alt with a letter: [`opens_menu`]), or holds a
    /// key that one press can be with another action's where both work: in one scope, or in
    /// a view and the app ([`conflict`]). Two overrides that clash are both refused; a
    /// clash the defaults already have stays, as Ctrl++ with Ctrl+1 on a Czech keyboard. A
    /// refused override leaves its action its default keys. Every override stays in
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
            let Some(action) = named(name) else {
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
    all()
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

/// Whether a press of `key` opens the menu button's menu at a top-level menu on `os`, and
/// so is kept from every action: on Windows and Linux, Alt alone with a letter, which is a
/// top-level menu's access key or may be one in another language (decision 39).
pub fn opens_menu(key: Shortcut, os: OperatingSystem) -> bool {
    os != OperatingSystem::Mac && alt_only(key.modifiers) && is_letter(key.key)
}

/// Why `id` may not have `key` on `os`, if it is kept from it: the clipboard's keys, and
/// the keys that open the menu button's menu, Alt with a letter and the menu action's own,
/// F10 on Linux, whatever keys that action is given.
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
            // As darktable and the keys print it.
            Key::Escape => "Esc".to_owned(),
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

    /// Whether a key pressed on `physical` with `pressed` held is this shortcut with Shift
    /// exactly as it names it, + typed with Shift being the = key with Shift, as on a US
    /// keyboard, where Ctrl++ is Ctrl+Shift+=. Such a shortcut wins over one the extra
    /// Shift allows.
    fn matches_exactly(
        self,
        key: Key,
        physical: Option<Key>,
        pressed: Modifiers,
        mac: bool,
    ) -> bool {
        let keys = [key, under_plus(key, physical, pressed)];
        keys.iter().any(|&key| self.matches(key, pressed, mac))
            && pressed.shift == self.modifiers.shift
    }
}

/// The key a press of `key` on `physical` with `pressed` held was made on, for a shortcut
/// naming Shift exactly: the = key, where `key` is a + typed with Shift, as on the layouts
/// where it needs Shift, the US, Dvorak and AZERTY among them; else `key` itself. The
/// keypad's +, a key of its own that egui reports as the physical +, stays +.
fn under_plus(key: Key, physical: Option<Key>, pressed: Modifiers) -> Key {
    if key == Key::Plus && pressed.shift && physical != Some(Key::Plus) {
        Key::Equals
    } else {
        key
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
    /// The focused widget's kind in `ctx`. The zoomed photo's focus stop keeps no keys, so
    /// Alt with the arrows move it and the arrows change the photo as anywhere.
    pub fn of(ctx: &egui::Context) -> Focus {
        let focused = ctx.memory(|m| m.focused());
        if ctx.text_edit_focused() {
            Focus::Text
        } else if focused.is_some_and(|id| id != crate::zoom::photo_view_id()) {
            Focus::Control
        } else {
            Focus::None
        }
    }

    /// Whether the focused widget keeps `key` for itself: the widget scope, narrower than
    /// any action's. A control keeps the arrows with Ctrl on Windows and Linux too, a
    /// slider's, the curve's and the equalizer's tenth of a step, never with the Mac's Cmd;
    /// on the Mac that tenth is Option's, which no shortcut of a control's keys takes.
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
        let arrow = matches!(
            key,
            Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown
        );
        // Ctrl on Windows and Linux is egui's command without the Mac's Cmd.
        let finer = modifiers.command && !modifiers.mac_cmd;
        match self {
            Focus::None => false,
            Focus::Control if arrow && finer => true,
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

/// Symbols that known layouts type on a digit key with Shift, with the digit: German's `=`
/// on 0. A key bound to the symbol with Shift and one bound to the digit with Shift are then
/// one press ([`conflict`]), the digit winning.
const SHIFTED_ON_DIGITS: [(Key, Key); 1] = [(Key::Equals, Key::Num0)];

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
/// one letter; Ctrl++ and Ctrl+Shift++), + with Shift as the = key with Shift (Ctrl++ and
/// Ctrl+Shift+= on a US keyboard, where the one naming Shift wins), or a symbol a known
/// layout types on the other's digit ([`ON_DIGITS`], and with Shift [`SHIFTED_ON_DIGITS`]),
/// where the digit wins. [`Keymap::load`] refuses an override with it, and rebinding in
/// Settings will warn with it.
pub fn conflict(a: Shortcut, b: Shortcut, os: OperatingSystem) -> bool {
    // The = key with Shift types + on the layouts where + needs Shift.
    let typed = |s: Shortcut| match s.key {
        Key::Equals if s.modifiers.shift => Shortcut {
            key: Key::Plus,
            ..s
        },
        _ => s,
    };
    one_press(a, b, os) || one_press(typed(a), b, os) || one_press(a, typed(b), os)
}

/// Whether one press can be both `a` and `b` on `os`, the keys as typed ([`conflict`]).
fn one_press(a: Shortcut, b: Shortcut, os: OperatingSystem) -> bool {
    let mac = os == OperatingSystem::Mac;
    let accepts = |pressed: Shortcut, wanted: Shortcut| {
        same_modifiers(
            held(pressed.modifiers, mac),
            wanted.modifiers,
            wanted.key,
            mac,
        )
    };
    let pair =
        |pairs: &[(Key, Key)]| pairs.contains(&(a.key, b.key)) || pairs.contains(&(b.key, a.key));
    let shifted = a.modifiers.shift && b.modifiers.shift;
    let keys = a.key == b.key || pair(&ON_DIGITS) || (shifted && pair(&SHIFTED_ON_DIGITS));
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

/// The narrowest action `key` belongs to, whether it can run or not. In a scope, a
/// shortcut naming Shift exactly wins over one the extra Shift a symbol needs allows
/// ([`Shortcut::matches_exactly`]), so Ctrl++ on a US keyboard is Ctrl+Shift+=.
fn bound(
    keymap: &Keymap,
    state: &State,
    (key, physical): (Key, Option<Key>),
    modifiers: Modifiers,
    mac: bool,
) -> Option<&'static Action> {
    let exactly = |s: &Shortcut| s.matches_exactly(key, physical, modifiers, mac);
    let allowed = |s: &Shortcut| s.matches(key, modifiers, mac);
    let has =
        |a: &Action, matches: &dyn Fn(&Shortcut) -> bool| keymap.keys(a.id).iter().any(matches);
    scopes(state).find_map(|scope| {
        let ours = || all().filter(move |a| a.scope == scope);
        (ours().find(|a| has(a, &exactly))).or_else(|| ours().find(|a| has(a, &allowed)))
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
        .and_then(|digit| bound(keymap, state, (digit, press.physical), press.modifiers, mac))
        .or_else(|| {
            bound(
                keymap,
                state,
                (press.key, press.physical),
                press.modifiers,
                mac,
            )
        })
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
        // A plain Escape stays with the widget that had the keys as the last frame ended,
        // which egui has taken the focus from already: a field's cancel, a list's close.
        if key == Key::Escape && modifiers.is_none() && state.escape_kept {
            taken.push(false);
            continue;
        }
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
        let holds = all()
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

/// The keys a focused text field undoes, redoes or selects all of its text with, egui's
/// own whatever keys the actions have, as a press and its release: what Undo, Redo and
/// Select All hand the field when chosen from a menu while it has the focus (decision 39),
/// added to the next frame's input. None for any other action.
pub fn field_keys(id: ActionId, os: OperatingSystem) -> Option<[egui::Event; 2]> {
    let (key, shift) = match id {
        ActionId::UNDO => (Key::Z, Modifiers::NONE),
        ActionId::REDO => (Key::Z, Modifiers::SHIFT),
        ActionId::SELECT_ALL => (Key::A, Modifiers::NONE),
        _ => return None,
    };
    let modifiers = held(Modifiers::COMMAND.plus(shift), os == OperatingSystem::Mac);
    let key = |pressed| egui::Event::Key {
        key,
        physical_key: Some(key),
        pressed,
        repeat: false,
        modifiers,
    };
    Some([key(true), key(false)])
}

/// Takes out of the frame's input the first press of Alt with a letter, and the text it
/// typed, if `menu` finds a menu for that letter, and returns what it found: on Linux, Alt
/// with a top-level menu's access key opens the menu button's menu at that menu, from
/// anywhere (decision 39). The letter is the one the press typed on the current layout, as
/// ф on a Ukrainian keyboard, which egui reports as its physical key, A; where `menu` finds
/// nothing for it, or nothing was typed, the key's own Latin letter, so an English
/// interface's Alt+F opens File on a Cyrillic layout too. Only Alt counts, so AltGr, which
/// egui never sees as Alt, never opens a menu, and a key's repeat opens none again. A press
/// `menu` finds nothing for is left alone. `menu` runs outside egui's lock, so it may read
/// the context.
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
/// focused widget's, or before it going back, in `regions`' order (the Toolbar, the
/// viewer bar, the Tools panel, the Browser). With nothing focused, F6 starts at the
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
pub(crate) mod tests {
    use std::collections::HashSet;

    use apertrust_ui::Locale;

    use super::*;

    const PLATFORMS: [OperatingSystem; 3] = [
        OperatingSystem::Mac,
        OperatingSystem::Windows,
        OperatingSystem::Nix,
    ];

    /// A state in which everything built can run.
    pub(crate) fn everything() -> State {
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
            select: Selectable {
                all: true,
                none: true,
                invert: true,
                unedited: true,
                previous: true,
            },
            next: true,
            previous: true,
            photos: 2,
            exported: 2,
            opened: true,
            recents: [Some(true); crate::recents::MOST],
            tabs: first_tabs(7),
            hidden_tab: Some(false),
            show_hidden: false,
            scale: 1.5,
            zoom_in: true,
            zoom_out: true,
            fitted: true,
            at_100: false,
            zoomed: true,
            dialog: false,
            modal: false,
            dialog_window: false,
            full_screen: false,
            toolbar_labels: true,
            toolbar_item: None,
            menu_button_hidden: false,
            panel_tab: 0,
            workspace: workspace(crate::layout::DEFAULT),
            workspace_given: false,
            show_hidden_given: false,
            panel_position: ToolsPanelPosition::Right,
            panel_position_given: false,
            section: None,
            step: None,
            job: None,
            browser: crate::settings::BrowserSettings::default(),
            shown: true,
            background: Some(Background::Dark),
            // A type that takes a flip, so every flip can run.
            guides: true,
            guide: apertrust_ui::guides::Guide::GoldenMean,
            guide_flip: apertrust_ui::guides::GuideFlip::None,
            escape_kept: false,
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

    /// The default keys that are one press on some keyboard, each settled by the matching
    /// rules (`docs/gui.md`, The default keyboard map): on Czech and Slovak keyboards the 1
    /// key types +, and the number row's digit wins, so Ctrl+1 chooses the first tab there,
    /// and Ctrl+= zooms in; on a US keyboard Ctrl++ is Ctrl+Shift+=, and the key naming
    /// Shift exactly wins, zooming every photo; on German keyboards Shift+0 types =, and the
    /// digit wins, fitting every photo.
    const ACCEPTED: [(ActionId, ActionId); 3] = [
        (ActionId::PANEL_TABS[0], ActionId::ZOOM_IN),
        (ActionId::ZOOM_IN, ActionId::ZOOM_IN_ALL),
        (ActionId::ZOOM_IN_ALL, ActionId::ZOOM_FIT_ALL),
    ];

    /// The accepted overlaps between actions of `scopes`, either way round.
    fn accepted(scopes: [Scope; 2]) -> HashSet<(ActionId, ActionId)> {
        let scope = |id| action(id).expect("declared").scope;
        ACCEPTED
            .into_iter()
            .filter(|&(a, b)| {
                let theirs = [scope(a), scope(b)];
                theirs == scopes || theirs == [scopes[1], scopes[0]]
            })
            .collect()
    }

    #[test]
    fn no_two_default_keys_in_one_scope_are_one_press() {
        for os in PLATFORMS {
            for scope in [Scope::App, Scope::View(View::Edit)] {
                let found = conflicts(os, [scope, scope]);
                assert_eq!(found, accepted([scope, scope]), "{os:?} in {scope:?}");
            }
        }
    }

    #[test]
    fn no_view_key_hides_an_app_key_by_default() {
        // The narrowest scope wins, so a view key that is one press with an app key hides
        // it; but Ctrl+1 on a Czech keyboard, where the digit wins.
        let scopes = [Scope::App, Scope::View(View::Edit)];
        for os in PLATFORMS {
            let found = conflicts(os, scopes);
            assert_eq!(found, accepted(scopes), "{os:?}");
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
        // + with Shift is the = key with Shift, where + needs Shift, but = alone is not +.
        assert!(conflict(
            key(command, Key::Plus),
            key(command_shift, Key::Equals),
            windows
        ));
        assert!(conflict(
            key(command_shift, Key::Equals),
            key(command, Key::Plus),
            mac
        ));
        assert!(!conflict(
            key(command, Key::Plus),
            key(command, Key::Equals),
            windows
        ));
        assert!(!conflict(
            key(command, Key::Equals),
            key(command_shift, Key::Equals),
            windows
        ));
        // German types = on Shift+0, but not on 0 alone.
        assert!(conflict(
            key(command_shift, Key::Equals),
            key(command_shift, Key::Num0),
            windows
        ));
        assert!(!conflict(
            key(command, Key::Equals),
            key(command, Key::Num0),
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

    /// Each workspace has its action, `workspace-<id>` (decision 96): the built-in ones
    /// always, Default then darktable, named by their messages; a person's own made once,
    /// the first time it is asked for, as the app asks as it starts. None has a default key;
    /// each chooses its workspace, checked while it is the one shown, on the main window
    /// with no modal dialog over it.
    #[test]
    fn each_workspace_has_its_action_made_once() {
        let made: Vec<&str> = workspace_actions().iter().map(|a| a.id.0).collect();
        assert_eq!(made[..2], ["workspace-default", "workspace-essentials"]);
        let own = workspace("street");
        assert_eq!(own, workspace("street"));
        assert_eq!(own.0, "workspace-street");
        let mut state = everything();
        for (id, message) in [
            ("default", "workspace-default"),
            ("essentials", "workspace-essentials"),
            ("street", "menu-workspace"),
        ] {
            let action = action(workspace(id)).expect("registered");
            assert_eq!(action.message, message);
            assert!(action.keys.is_empty(), "{id}");
            assert_eq!(action.emits, Emit::Intent(Intent::Workspace(id)));
            assert_eq!(named(action.id.0).map(|a| a.id), Some(action.id));
            state.workspace = action.id;
            assert_eq!(action.checked(&state), Some(true));
            state.workspace = workspace("elsewhere");
            assert_eq!(action.checked(&state), Some(false));
            assert!(action.enabled(&state));
            state.modal = true;
            assert!(!action.enabled(&state));
            state.modal = false;
            // `--conf` sets the workspace for the run: none can be chosen.
            state.workspace_given = true;
            assert!(!action.enabled(&state));
            state.workspace_given = false;
        }
        let ids: Vec<&str> = workspace_actions().iter().map(|a| a.id.0).collect();
        assert_eq!(
            ids.iter().filter(|id| **id == "workspace-street").count(),
            1
        );
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

    /// Reset rests on a photo open for viewing only, whose edit takes no change, even
    /// where the edit holds something.
    #[test]
    fn reset_rests_on_a_photo_open_for_viewing_only() {
        let reset = action(ActionId::RESET).unwrap();
        assert!(reset.enabled(&everything()));
        let viewing = State {
            view_only: true,
            ..everything()
        };
        assert!(!reset.enabled(&viewing));
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

    /// The Select menu's items send the session's selection commands, each disabled where
    /// it would change nothing; Select All also runs for a focused text field, as Undo does.
    #[test]
    fn select_s_items_send_their_commands_and_rest_where_they_change_nothing() {
        let all = everything().select;
        // Each item, its command, and the state in which it alone would change nothing.
        let items = [
            (
                ActionId::SELECT_ALL,
                SelectionCommand::All,
                Selectable { all: false, ..all },
            ),
            (
                ActionId::SELECT_NONE,
                SelectionCommand::Clear,
                Selectable { none: false, ..all },
            ),
            (
                ActionId::INVERT_SELECTION,
                SelectionCommand::Invert,
                Selectable {
                    invert: false,
                    ..all
                },
            ),
            (
                ActionId::SELECT_UNEDITED,
                SelectionCommand::Unedited,
                Selectable {
                    unedited: false,
                    ..all
                },
            ),
            (
                ActionId::PREVIOUS_SELECTION,
                SelectionCommand::Previous,
                Selectable {
                    previous: false,
                    ..all
                },
            ),
        ];
        for (id, command, rests) in items {
            let item = action(id).expect("declared");
            assert_eq!(item.emits, Emit::Command(Command::Selection(command)));
            let mut state = everything();
            assert!(item.enabled(&state), "{}", id.0);
            state.select = rests;
            assert!(!item.enabled(&state), "{}", id.0);
            state.text = true;
            assert_eq!(item.enabled(&state), id == ActionId::SELECT_ALL, "{}", id.0);
            let modal = State {
                modal: true,
                ..everything()
            };
            assert!(!item.enabled(&modal), "{}", id.0);
        }
        // Darktable's keys, but Invert Selection's Ctrl+I, which is Import's: Ctrl+Shift+I.
        let keymap = Keymap::defaults(OperatingSystem::Nix);
        let keys = |id| keymap.keys(id).to_vec();
        let ctrl = |modifiers, key| Shortcut { modifiers, key };
        assert_eq!(
            keys(ActionId::SELECT_ALL),
            [ctrl(Modifiers::COMMAND, Key::A)]
        );
        let shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);
        assert_eq!(keys(ActionId::SELECT_NONE), [ctrl(shift, Key::A)]);
        assert_eq!(keys(ActionId::INVERT_SELECTION), [ctrl(shift, Key::I)]);
        assert!(keys(ActionId::SELECT_UNEDITED).is_empty());
        // Previous Selection on Ctrl+Shift+D, darktable's duplicate virgin's key (decision 90).
        assert_eq!(keys(ActionId::PREVIOUS_SELECTION), [ctrl(shift, Key::D)]);
    }

    /// Next Photo and Previous Photo move the active photo, each disabled with no photo to
    /// go to, under a modal dialog, and while a dialog's own window has the focus.
    #[test]
    fn next_and_previous_photo_run_with_a_photo_to_go_to() {
        let next = action(ActionId::NEXT_PHOTO).expect("declared");
        let previous = action(ActionId::PREVIOUS_PHOTO).expect("declared");
        assert_eq!(next.emits, Emit::Intent(Intent::Step { next: true }));
        assert_eq!(previous.emits, Emit::Intent(Intent::Step { next: false }));
        let mut state = everything();
        assert!(next.enabled(&state) && previous.enabled(&state));
        state.next = false;
        assert!(!next.enabled(&state) && previous.enabled(&state));
        state.previous = false;
        assert!(!previous.enabled(&state));
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
            assert!(!next.enabled(&rest) && !previous.enabled(&rest));
        }
    }

    /// Ctrl+A in a focused text field is the field's, selecting its text, never the
    /// photos; chosen from a menu there, Select All hands the field egui's own Ctrl+A.
    #[test]
    fn ctrl_a_in_a_text_field_selects_its_text_not_the_photos() {
        for os in PLATFORMS {
            let mac = os == OperatingSystem::Mac;
            let keymap = Keymap::defaults(os);
            let state = everything();
            let ctrl_a = || vec![key_event(Key::A, held(Modifiers::COMMAND, mac))];
            let mut events = ctrl_a();
            let pressed = take_presses(&mut events, &keymap, &state, Focus::Text, mac);
            assert!(pressed.is_empty(), "{os:?}: {pressed:?}");
            assert_eq!(events, ctrl_a(), "{os:?}: left to the field");
            let mut events = ctrl_a();
            let pressed = take_presses(&mut events, &keymap, &state, Focus::None, mac);
            assert_eq!(pressed, [ActionId::SELECT_ALL], "{os:?}");
            let [press, _] = field_keys(ActionId::SELECT_ALL, os).expect("Select All's");
            assert_eq!(press, ctrl_a()[0], "{os:?}");
        }
    }

    /// A plain Escape after a focused field stays the field's, choosing no cursor; an
    /// Escape with a modifier the user bound still runs its action.
    #[test]
    fn a_kept_escape_holds_back_only_the_plain_key() {
        for os in PLATFORMS {
            let mac = os == OperatingSystem::Mac;
            let (keymap, warnings) = Keymap::load(os, &overrides(&[("show-log", &["Ctrl+Esc"])]));
            assert!(warnings.is_empty(), "{os:?}: {warnings:?}");
            let state = State {
                escape_kept: true,
                ..everything()
            };
            let plain = || vec![key_event(Key::Escape, Modifiers::NONE)];
            let mut events = plain();
            let pressed = take_presses(&mut events, &keymap, &state, Focus::None, mac);
            assert!(pressed.is_empty(), "{os:?}: {pressed:?}");
            assert_eq!(events, plain(), "{os:?}: left to the field");
            let mut events = vec![key_event(Key::Escape, held(Modifiers::COMMAND, mac))];
            let pressed = take_presses(&mut events, &keymap, &state, Focus::None, mac);
            assert_eq!(pressed, [ActionId::SHOW_LOG], "{os:?}");
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
            spell(ActionId::ZOOM_OUT, windows).as_deref(),
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

    /// Export… reveals the tool with a photo open, view-only included, so the tool can say
    /// why, or with a selection to export and none open.
    #[test]
    fn export_reveals_the_tool_with_a_photo_open_or_a_selection_to_export() {
        let export = action(ActionId::EXPORT).expect("declared");
        let mut state = everything();
        (state.photo, state.exportable) = (true, false);
        assert!(export.enabled(&state));
        (state.photo, state.exportable) = (false, true);
        assert!(export.enabled(&state));
        state.exportable = false;
        assert!(!export.enabled(&state));
    }

    #[test]
    fn export_again_is_ctrl_shift_e_and_needs_a_photo_to_export() {
        let en = Translator::new(Locale::EnGb);
        for os in PLATFORMS {
            let keys = Keymap::defaults(os).keys(ActionId::EXPORT_AGAIN).to_vec();
            let ctrl_shift_e = Shortcut {
                modifiers: Modifiers::COMMAND | Modifiers::SHIFT,
                key: Key::E,
            };
            assert_eq!(keys, [ctrl_shift_e], "{os:?}");
        }
        let again = action(ActionId::EXPORT_AGAIN).expect("declared");
        assert_eq!(en.text(again.message), "Export Again");
        assert_eq!(again.emits, Emit::Intent(Intent::ExportAgain));
        let mut state = everything();
        assert!(again.enabled(&state));
        state.exportable = false;
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

    /// A focused control keeps the arrows with Ctrl on Windows and Linux, its tenth of a
    /// step, Shift with it too, but no other key with Ctrl and no arrow with the Mac's Cmd
    /// or Control.
    #[test]
    fn a_focused_control_keeps_the_arrows_with_ctrl_for_their_tenth() {
        let ctrl = Modifiers::CTRL | Modifiers::COMMAND;
        for key in [
            Key::ArrowLeft,
            Key::ArrowRight,
            Key::ArrowUp,
            Key::ArrowDown,
        ] {
            assert!(Focus::Control.keeps(key, ctrl));
            assert!(Focus::Control.keeps(key, ctrl | Modifiers::SHIFT));
            assert!(Focus::Control.keeps(key, Modifiers::ALT));
            assert!(!Focus::Control.keeps(key, Modifiers::MAC_CMD | Modifiers::COMMAND));
            assert!(!Focus::Control.keeps(key, Modifiers::CTRL));
            assert!(!Focus::None.keeps(key, ctrl));
        }
        assert!(!Focus::Control.keeps(Key::Home, ctrl));
        assert!(!Focus::Control.keeps(Key::PageUp, ctrl));
    }

    #[test]
    fn a_focused_control_keeps_insert_and_plus_for_the_curve() {
        assert!(Focus::Control.keeps(Key::Insert, Modifiers::NONE));
        assert!(Focus::Control.keeps(Key::Plus, Modifiers::NONE));
        // + needs Shift on a US keyboard.
        assert!(Focus::Control.keeps(Key::Plus, Modifiers::SHIFT));
        // Ctrl++ still zooms the photo.
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

    /// The action a press of `key` with `modifiers` held, on the `physical` key, runs with a
    /// photo open, as `os` matches it.
    fn pressed(
        os: OperatingSystem,
        key: Key,
        physical: Key,
        modifiers: Modifiers,
    ) -> Option<ActionId> {
        pressed_on(Focus::None, os, key, physical, modifiers)
    }

    /// As [`pressed`], with `focus` the focused widget's kind.
    fn pressed_on(
        focus: Focus,
        os: OperatingSystem,
        key: Key,
        physical: Key,
        modifiers: Modifiers,
    ) -> Option<ActionId> {
        let keymap = Keymap::defaults(os);
        let mac = os == OperatingSystem::Mac;
        let press = Press {
            key,
            physical: Some(physical),
            modifiers: held(modifiers, mac),
            typed: None,
        };
        press_action(&keymap, &everything(), focus, &press, mac).map(|a| a.id)
    }

    /// Each arrow with the pan action Alt with it runs, and the one Shift+Alt with it runs.
    const PANS: [(Key, ActionId, ActionId); 4] = [
        (Key::ArrowLeft, ActionId::PAN_LEFT, ActionId::PAN_PAGE_LEFT),
        (
            Key::ArrowRight,
            ActionId::PAN_RIGHT,
            ActionId::PAN_PAGE_RIGHT,
        ),
        (Key::ArrowUp, ActionId::PAN_UP, ActionId::PAN_PAGE_UP),
        (Key::ArrowDown, ActionId::PAN_DOWN, ActionId::PAN_PAGE_DOWN),
    ];

    /// Alt with the arrows, Option on the Mac, move a zoomed photo's view, Shift+Alt a whole
    /// view, each repeating while held and opening no menu on Windows and Linux; the
    /// arrows alone stay Next Photo and Previous Photo.
    #[test]
    fn alt_arrows_pan_and_plain_arrows_change_the_photo() {
        for os in PLATFORMS {
            for (key, step, page) in PANS {
                assert_eq!(pressed(os, key, key, Modifiers::ALT), Some(step), "{os:?}");
                assert_eq!(pressed(os, key, key, PAN_PAGE), Some(page), "{os:?}");
                let alt = Shortcut {
                    modifiers: Modifiers::ALT,
                    key,
                };
                assert!(!opens_menu(alt, os), "{os:?}");
                for id in [step, page] {
                    assert_eq!(action(id).expect("declared").trigger, Trigger::Repeat);
                }
            }
            let right = pressed(os, Key::ArrowRight, Key::ArrowRight, Modifiers::NONE);
            assert_eq!(right, Some(ActionId::NEXT_PHOTO), "{os:?}");
            let left = pressed(os, Key::ArrowLeft, Key::ArrowLeft, Modifiers::NONE);
            assert_eq!(left, Some(ActionId::PREVIOUS_PHOTO), "{os:?}");
        }
    }

    /// A focused section header, a control, keeps Alt+↑ and Alt+↓ to move itself (decision
    /// 83): they never pan the photo while it has the focus.
    #[test]
    fn a_focused_header_keeps_alt_up_and_down() {
        for os in PLATFORMS {
            for key in [Key::ArrowUp, Key::ArrowDown] {
                let kept = pressed_on(Focus::Control, os, key, key, Modifiers::ALT);
                assert_eq!(kept, None, "{os:?}");
            }
        }
    }

    /// A focused text field keeps every Alt with an arrow, Shift's too: they move and select
    /// by word there.
    #[test]
    fn a_text_field_keeps_alt_arrows() {
        for os in PLATFORMS {
            for (key, ..) in PANS {
                for modifiers in [Modifiers::ALT, PAN_PAGE] {
                    let kept = pressed_on(Focus::Text, os, key, key, modifiers);
                    assert_eq!(kept, None, "{os:?}");
                }
            }
        }
    }

    #[test]
    fn ctrl_equals_minus_and_zero_zoom_the_photo_not_the_interface() {
        let command = Modifiers::COMMAND;
        for os in PLATFORMS {
            let keymap = Keymap::defaults(os);
            for id in [
                ActionId::SCALE_UP,
                ActionId::SCALE_DOWN,
                ActionId::SCALE_RESET,
            ] {
                assert!(keymap.keys(id).is_empty(), "{os:?}: {}", id.0);
            }
            let zooms = [
                (Key::Equals, command, ActionId::ZOOM_IN),
                (Key::Minus, command, ActionId::ZOOM_OUT),
                (Key::Num0, command, ActionId::ZOOM_FIT),
                // The Shift forms, every photo; with one, as the plain ones.
                (
                    Key::Minus,
                    command.plus(Modifiers::SHIFT),
                    ActionId::ZOOM_OUT_ALL,
                ),
                (
                    Key::Num0,
                    command.plus(Modifiers::SHIFT),
                    ActionId::ZOOM_FIT_ALL,
                ),
                (Key::Z, Modifiers::SHIFT, ActionId::ZOOM_100_ALL),
                (Key::Z, Modifiers::NONE, ActionId::ZOOM_100),
            ];
            for (key, modifiers, id) in zooms {
                assert_eq!(
                    pressed(os, key, key, modifiers),
                    Some(id),
                    "{os:?}: {}",
                    id.0
                );
            }
        }
        // Each Shift form does what its plain action does.
        for (all, one) in [
            (ActionId::ZOOM_IN_ALL, ActionId::ZOOM_IN),
            (ActionId::ZOOM_OUT_ALL, ActionId::ZOOM_OUT),
            (ActionId::ZOOM_FIT_ALL, ActionId::ZOOM_FIT),
            (ActionId::ZOOM_100_ALL, ActionId::ZOOM_100),
        ] {
            let emits = |id| &action(id).expect("declared").emits;
            assert_eq!(emits(all), emits(one), "{}", all.0);
        }
    }

    #[test]
    fn ctrl_plus_on_a_us_keyboard_runs_zoom_in_all() {
        // Shift and the = key type +: the shortcut naming Shift exactly wins over Ctrl++'s
        // extra Shift.
        let command_shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);
        for os in PLATFORMS {
            let us = pressed(os, Key::Plus, Key::Equals, command_shift);
            assert_eq!(us, Some(ActionId::ZOOM_IN_ALL), "{os:?}");
            // As some platforms report it, the key under the +.
            let equals = pressed(os, Key::Equals, Key::Equals, command_shift);
            assert_eq!(equals, Some(ActionId::ZOOM_IN_ALL), "{os:?}");
            // Ctrl+= is one photo's.
            let one = pressed(os, Key::Equals, Key::Equals, Modifiers::COMMAND);
            assert_eq!(one, Some(ActionId::ZOOM_IN), "{os:?}");
            // German types = with Shift on the 0 key: the digit wins.
            let german = pressed(os, Key::Equals, Key::Num0, command_shift);
            assert_eq!(german, Some(ActionId::ZOOM_FIT_ALL), "{os:?}");
        }
    }

    #[test]
    fn ctrl_plus_where_plus_has_its_own_key_runs_zoom_in() {
        // German, Spanish and Nordic keyboards type + on a key of its own, without Shift.
        for os in PLATFORMS {
            let own = pressed(os, Key::Plus, Key::CloseBracket, Modifiers::COMMAND);
            assert_eq!(own, Some(ActionId::ZOOM_IN), "{os:?}");
            // The numeric keypad's +, with Shift too: a key of its own, never the = key.
            let keypad = pressed(os, Key::Plus, Key::Plus, Modifiers::COMMAND);
            assert_eq!(keypad, Some(ActionId::ZOOM_IN), "{os:?}");
            let shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);
            let keypad = pressed(os, Key::Plus, Key::Plus, shift);
            assert_eq!(keypad, Some(ActionId::ZOOM_IN), "{os:?}");
        }
    }

    #[test]
    fn the_zoom_steps_only_where_it_has_somewhere_to_go() {
        let state = State {
            zoom_in: false,
            zoom_out: true,
            fitted: true,
            at_100: false,
            ..everything()
        };
        let runs = |id, state: &State| action(id).expect("declared").can_run(state);
        assert!(!runs(ActionId::ZOOM_IN, &state) && !runs(ActionId::ZOOM_IN_ALL, &state));
        assert!(runs(ActionId::ZOOM_OUT, &state) && runs(ActionId::ZOOM_OUT_ALL, &state));
        assert!(runs(ActionId::ZOOM_FIT, &state) && runs(ActionId::ZOOM_SMALL, &state));
        let fit = action(ActionId::ZOOM_FIT).expect("declared");
        assert_eq!(fit.checked(&state), Some(true));
        assert_eq!(
            fit.checked(&State {
                fitted: false,
                ..state
            }),
            Some(false)
        );
        // Nothing moves on a photo not yet live, nor with none.
        for blocked in [
            State {
                waiting: true,
                ..state
            },
            State {
                photo: false,
                ..state
            },
        ] {
            for id in [ActionId::ZOOM_OUT, ActionId::ZOOM_FIT, ActionId::ZOOM_SMALL] {
                assert!(!runs(id, &blocked), "{}", id.0);
            }
        }
        // Every scale of the menu runs on a live photo, and Zoom to 100% is checked at it.
        for id in [ActionId::ZOOM_100, ActionId::ZOOM_FILL, ActionId::ZOOM_1600] {
            assert!(runs(id, &state), "{}", id.0);
        }
        let hundred = action(ActionId::ZOOM_100).expect("declared");
        assert_eq!(hundred.checked(&state), Some(false));
        let at_100 = State {
            at_100: true,
            ..state
        };
        assert_eq!(hundred.checked(&at_100), Some(true));
        assert_eq!(hundred.emits, Emit::Intent(Intent::Zoom(ZoomStep::Hundred)));
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
        // Nothing to focus in the viewer bar (region 1).
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
        let wanted = overrides(&[("zoom-in", &["Ctrl+Plus", "Ctrl+K"])]);
        let (keymap, warnings) = Keymap::load(os, &wanted);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(keymap.keys(ActionId::ZOOM_IN).len(), 2);

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

    /// Decision 91: every built tool's own actions are registry actions, one for each
    /// action row and each button, `tool-<tool>-<action>`, with no default keys, found by
    /// id beside the static ones; a button's runs as pressing it does, an action row's,
    /// drawn disabled until built, never.
    #[test]
    fn every_tool_s_own_actions_are_registry_actions_found_by_id() {
        let ids: Vec<&str> = tool_actions().iter().map(|a| a.id.0).collect();
        for id in [
            "tool-flip-rotate_ccw",
            "tool-flip-rotate_cw",
            "tool-flip-flip_horizontally",
            "tool-flip-flip_vertically",
            "tool-agx-auto_tune_levels",
        ] {
            assert!(ids.contains(&id), "{id} in {ids:?}");
        }
        let every: Vec<&str> = all().map(|a| a.id.0).collect();
        let unique: HashSet<&str> = every.iter().copied().collect();
        assert_eq!(unique.len(), every.len());
        assert!(every.len() >= ACTIONS.len() + tool_actions().len());
        let rotate = named("tool-flip-rotate_cw").expect("Rotate 90° clockwise");
        assert_eq!(action(rotate.id).map(|a| a.id), Some(rotate.id));
        assert_eq!(rotate.message, "tool-flip-action-rotate_cw-full");
        assert!(rotate.keys.is_empty());
        assert_eq!(rotate.scope, Scope::View(View::Edit));
        assert_eq!(
            rotate.emits,
            Emit::Intent(Intent::ToolAction {
                tool: "flip",
                action: "rotate_cw"
            })
        );
        // Only while the photo takes edits, with no modal dialog.
        assert!(rotate.enabled(&everything()));
        for rest in [
            State {
                editable: false,
                ..everything()
            },
            State {
                modal: true,
                ..everything()
            },
            State {
                dialog_window: true,
                ..everything()
            },
        ] {
            assert!(!rotate.enabled(&rest), "{rest:?}");
        }
        let auto = named("tool-agx-auto_tune_levels").expect("Auto tune levels");
        assert_eq!(auto.emits, Emit::Later);
        assert!(!auto.enabled(&everything()));
    }

    #[test]
    fn every_tool_action_has_its_words_in_every_language() {
        for locale in Locale::ALL {
            let tr = Translator::new(locale);
            for action in tool_actions() {
                assert!(tr.has(action.message), "{locale:?}: {}", action.message);
            }
        }
    }

    /// A tool's action takes keys from the settings file as any action does, and only one
    /// that clashes with none.
    #[test]
    fn a_tool_s_action_takes_keys_from_settings() {
        let os = OperatingSystem::Nix;
        let rotate = named("tool-flip-rotate_cw").unwrap().id;
        assert!(Keymap::defaults(os).keys(rotate).is_empty());
        let wanted = overrides(&[("tool-flip-rotate_cw", &["Ctrl+Alt+R"])]);
        let (keymap, warnings) = Keymap::load(os, &wanted);
        assert!(warnings.is_empty(), "{warnings:?}");
        let key = Shortcut::parse("Ctrl+Alt+R").unwrap();
        assert_eq!(keymap.keys(rotate), [key]);
        assert_eq!(keymap.overrides(), &wanted);
        // Undo's own keys clash with Undo.
        let (keymap, warnings) =
            Keymap::load(os, &overrides(&[("tool-flip-rotate_cw", &["Ctrl+Z"])]));
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(keymap.keys(rotate).is_empty());
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
