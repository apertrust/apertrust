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

//! The app's settings (`docs/gui.md`, Settings and workspace): how the app looks and is laid
//! out, kept between runs in one versioned TOML file, `settings.toml` in the user data
//! folder, which `apertrust_base::folders` chooses: `--user-data-dir`'s, then
//! `APERTRUST_USER_DATA_DIR`'s, then the platform's (decision 43). Never edits, which live
//! beside their photos (decision 48), and never the last photo.
//!
//! ```toml
//! version = 2
//! theme = "light"
//! locale = "uk"
//! scale = 1.2
//! workspace = "portraits"
//!
//! [window]
//! size = [1600, 1000]
//! position = [160, 120]
//! scale_factor = 2.0
//! maximized = false
//!
//! [settings_window]
//! position = [960, 520]
//! scale_factor = 2.0
//!
//! [jobs_window]
//! position = [1200, 160]
//! scale_factor = 2.0
//!
//! [log]
//! keep = true
//! domains = ["gpu", "perf"]
//!
//! [log_window]
//! position = [400, 300]
//! scale_factor = 2.0
//! size = [960, 466]
//! folder = "/Users/you/Downloads"
//!
//! [sliders]
//! marker = "triangle"
//!
//! [toolbar]
//! menu_button = "menu"
//! labels = false
//!
//! [tools_panel]
//! tab_names = false
//! wheel_scrolls_panel = false
//! show_hidden = true
//! position = "left"
//! width = 480
//!
//! [viewer]
//! margin = 16
//! proof_margin = 0.25
//!
//! [edit.sections]
//! exposure = false
//!
//! [edit.groups.agx]
//! advanced_curve_parameters = true
//!
//! [scopes]
//! graph = "waveform"
//! channels = ["red", "green", "blue", "luminance"]
//!
//! [browser]
//! position = "right"
//! columns = 2
//! rows = 2
//! sort = "file-name"
//! descending = true
//! thumbnails = "large"
//! labels = false
//! square_thumbnails = true
//!
//! [keys]
//! undo = ["Ctrl+K"]
//! quit = []
//! ```
//!
//! A setting the user never changed is not written, so a missing one is its default.
//! The language is kept only once the user chooses one; until then it follows the
//! system's. The Tools panel's side, right or left, is kept once chosen, app-wide. Its
//! width is in points, as the user dragged it, within the widths it can be dragged to;
//! until then it is the theme's. A tool's collapsible
//! groups are kept by the tool's id and the group's, only those the user opened or
//! closed, the others starting closed (decision 29). The scopes' view is kept by part:
//! the graph, the scale (`linear` or `logarithmic`), the channels shown and whether each
//! clipping overlay is on, each once the user changes it. So is the Browser's: the
//! sort's key and direction, the thumbnails' size, where it is, the rows it shows at the
//! bottom and the columns at a side, each a whole number, whether the thumbnails show
//! their captions and whether each fills its square (`docs/gui.md`, The Browser and The
//! browser bar). Keys are kept only where they
//! differ from the defaults, in the form `Shortcut::text` writes (`actions`), the same on
//! every platform. The Viewer's two margins have no page in the Settings dialog:
//! the margin round the photo and its caption, in points, and the one Proof margin takes,
//! a fraction of the slot's smaller side (`docs/gui.md`, The photo frame). The window's
//! position is in physical pixels, with the display's scale
//! factor then, as egui-winit keeps it, and so is the place Settings' own window was last
//! left in, on the Mac.
//!
//! **Declared** in the settings schema, `data/settings.schema.toml` (decision 43):
//! [`registry`] holds the app's settings, generated from it by `build.rs`. **Read and
//! written** by `apertrust-base`'s store (`apertrust_base::settings::Store`), which has the
//! file's rules: a value it cannot read gets its default with a warning and stays in the
//! file until the user changes it, unknown keys stay, a damaged file is set aside as
//! `settings.toml.bak`, a newer version's is never overwritten, and what changes is merged
//! into the file as it is, on a thread of its own, once it has stood still for a second
//! and on exit. [`Settings`] is the app's own view of them, which [`Keeper`] reads from the
//! store before the window opens and hands back each frame: only what differs from the
//! app's reading of the store is set, so a value the app reads otherwise than as it is
//! kept, as a position without its scale factor or channels named twice, stays until the
//! user changes it.
//!
//! The same store reads the engine's settings, `[engine]`, declared in the schema with
//! `owner = "engine"` and generated in `apertrust_engine::settings`: [`Keeper::engine`]
//! turns them into the plain options the engine takes, so the engine never opens the file
//! (decision 43). The app writes them only as the Settings dialog changes them
//! ([`Keeper::apply`]), which also keeps what Settings changes of the app's own and the
//! page it last showed.
//!
//! `--conf KEY=VALUE` sets one of either for this run ([`overrides`]): the store holds it
//! over the file and never writes it, so a change the user makes to it meanwhile, as the
//! scale by its keys, leaves the file's value as it is.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use apertrust_base::folders::Folders;
use apertrust_base::settings::{
    CONF_FLAG, Declared, FileStorage, OverrideError, Overrides, Setting, Storage, Store, Stored,
};
use apertrust_engine::session::ControlValue;
use apertrust_engine::settings as engine;
use apertrust_tools::{ControlKind, Row, SettingRow, TOOLS};
use apertrust_ui::components::{
    BrowserBarAction, BrowserPosition, Channel, Marker, MenuButtonStyle, ScopeView, Sort,
    ThumbnailSize, ToolsPanelPosition,
};
use apertrust_ui::{Builtin, Locale};

use apertrust_export::settings::registry::EXPORT_ASK_BEFORE_OVERWRITE;

use crate::export::Values;
use registry::*;

/// The app's settings as the settings schema declares them, generated from it by
/// `build.rs` as typed statics, each named after its id, and `ALL`, every one of them, for
/// the store (decision 43).
pub mod registry {
    include!(concat!(env!("OUT_DIR"), "/settings.rs"));

    /// The defaults the schema marks computed, each under its setting's name.
    mod computed {
        use apertrust_ui::{Background, Locale, Theme};

        /// The system's language, the interface's until the user chooses one.
        pub fn locale() -> Locale {
            crate::system_locale()
        }

        /// The theme's width, the default theme's here; the shell takes the width of the
        /// theme in use (`Shell::panel_edge`).
        pub fn tools_panel_width() -> f32 {
            Theme::builtin(super::THEME.default()).sizes.tools_panel
        }

        /// The default theme's own surround; the shell keeps the theme in use's while
        /// none is chosen.
        pub fn viewer_background() -> Background {
            let theme = Theme::builtin(super::THEME.default());
            Background::of(theme.colors.photo_surround).unwrap_or(Background::Dark)
        }
    }
}

/// What the app keeps between runs.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub theme: Builtin,
    /// The language the user chose; None follows the system's.
    pub locale: Option<Locale>,
    /// The interface's scale, egui's zoom factor on top of the system's.
    pub scale: f32,
    /// The marker on the sliders' tracks and the curve's handles (decision 75).
    pub marker: Marker,
    /// How the Toolbar's menu button is drawn on Windows and Linux, `toolbar.menu_button`.
    pub menu_button: MenuButtonStyle,
    /// Whether the Toolbar's actions show their words under their icons, `toolbar.labels`
    /// (decision 92).
    pub toolbar_labels: bool,
    pub window: Window,
    /// Where Settings' own window was last left, on the Mac; None opens it centred over
    /// the main window.
    pub settings_window: Option<Position>,
    /// Where the jobs window was last left; None opens it centred over the main window.
    pub jobs_window: Option<Position>,
    /// The log kept in memory, and its window's place and folder.
    pub log: Log,
    /// Whether the Tools panel's tabs show their names.
    pub tab_names: bool,
    /// Whether the mouse wheel scrolls the Tools panel, a row taking it only with Ctrl
    /// (Option on the Mac); off, darktable's default, a row under the pointer takes it.
    pub wheel_scrolls_panel: bool,
    /// Whether the Tools panel shows the Hidden tab, Show Hidden Tools (decision 96).
    pub show_hidden: bool,
    /// Where the Tools panel is, at the window's right or left, `tools_panel.position`.
    pub panel_position: ToolsPanelPosition,
    /// The Tools panel's width in points, within `PANEL_WIDTHS`, as the user dragged it;
    /// None for the theme's.
    pub panel_width: Option<f32>,
    /// The margins round the Viewer's photo and caption.
    pub viewer: Viewer,
    /// The workspace the Tools panel shows, by id, as kept: one with no workspace stays
    /// until another is chosen (decision 96).
    pub workspace: String,
    /// Edit's own state.
    pub edit: ViewState,
    /// What the Scopes section shows, and which clipping overlays the photo shows.
    pub scopes: ScopeView,
    /// How the Browser shows the folder's photos.
    pub browser: BrowserSettings,
    /// The keys that differ from the defaults, by action id, in `Shortcut::text`'s form,
    /// with what the keymap could not use as it was read; an empty list leaves an action
    /// without a key.
    pub keys: BTreeMap<String, Vec<String>>,
    /// The Export tool's last settings, `export.*`, kept as they are left.
    pub export: Values,
    /// The tools' settings, `tools.<tool>.*` (decision 29).
    pub tool_settings: ToolSettings,
    /// Whether Export and Export Again ask before replacing files, `export.ask_before_overwrite`:
    /// kept apart from the tool's settings, since the tool does not edit it and its
    /// Default settings do not reset it.
    pub ask_before_overwrite: bool,
    /// Whether Discard History asks first, `history.ask_before_discard`.
    pub ask_before_discard: bool,
    /// Whether the area a picker cursor last read stays drawn on the photo,
    /// `cursors.pick.show_last` (decision 104).
    pub show_last_pick: bool,
}

impl Default for Settings {
    /// The schema's defaults. What it computes, the language and the Tools panel's width,
    /// and what is kept only once the window or the user sets it, the window's size and
    /// place and Edit's tab, start unkept, as do the tables of sections, groups and keys.
    fn default() -> Self {
        let channels = SCOPES_CHANNELS.default();
        Settings {
            theme: THEME.default(),
            locale: None,
            scale: SCALE.default(),
            marker: SLIDERS_MARKER.default(),
            menu_button: TOOLBAR_MENU_BUTTON.default(),
            toolbar_labels: TOOLBAR_LABELS.default(),
            window: Window {
                size: None,
                position: None,
                maximized: WINDOW_MAXIMIZED.default(),
            },
            settings_window: None,
            jobs_window: None,
            log: Log::default(),
            tab_names: TOOLS_PANEL_TAB_NAMES.default(),
            wheel_scrolls_panel: TOOLS_PANEL_WHEEL_SCROLLS_PANEL.default(),
            show_hidden: TOOLS_PANEL_SHOW_HIDDEN.default(),
            panel_position: TOOLS_PANEL_POSITION.default(),
            panel_width: None,
            viewer: Viewer::default(),
            workspace: WORKSPACE.default().to_owned(),
            edit: ViewState::default(),
            scopes: ScopeView {
                scope: SCOPES_GRAPH.default(),
                scale: SCOPES_SCALE.default(),
                channels: Channel::ALL.map(|channel| channels.contains(&channel)),
                shadow_clipping: SCOPES_SHADOW_CLIPPING.default(),
                highlight_clipping: SCOPES_HIGHLIGHT_CLIPPING.default(),
            },
            browser: BrowserSettings::default(),
            keys: BTreeMap::new(),
            export: Values::default(),
            tool_settings: ToolSettings::default(),
            ask_before_overwrite: EXPORT_ASK_BEFORE_OVERWRITE.default(),
            ask_before_discard: HISTORY_ASK_BEFORE_DISCARD.default(),
            show_last_pick: CURSORS_PICK_SHOW_LAST.default(),
        }
    }
}

impl Settings {
    /// The settings as `store` holds them: what is kept only once set (the language, the
    /// window's size and place, the Tools panel's width, Edit's tab) as kept, the rest as
    /// their values, kept or their defaults.
    fn read(store: &Store) -> Settings {
        let position = |pixels: Option<[f32; 2]>, scale_factor| {
            Some(Position {
                pixels: pixels?,
                scale_factor: scale_factor?,
            })
        };
        let window = position(store.get(&WINDOW_POSITION), store.get(&WINDOW_SCALE_FACTOR));
        let settings_window = position(
            store.get(&SETTINGS_WINDOW_POSITION),
            store.get(&SETTINGS_WINDOW_SCALE_FACTOR),
        );
        let jobs_window = position(
            store.get(&JOBS_WINDOW_POSITION),
            store.get(&JOBS_WINDOW_SCALE_FACTOR),
        );
        let log_window = position(
            store.get(&LOG_WINDOW_POSITION),
            store.get(&LOG_WINDOW_SCALE_FACTOR),
        );
        let shown = store.get(&SCOPES_CHANNELS);
        Settings {
            theme: store.get(&THEME),
            locale: store.kept(&LOCALE),
            scale: store.get(&SCALE),
            marker: store.get(&SLIDERS_MARKER),
            menu_button: store.get(&TOOLBAR_MENU_BUTTON),
            toolbar_labels: store.get(&TOOLBAR_LABELS),
            window: Window {
                size: store.kept(&WINDOW_SIZE),
                position: window,
                maximized: store.get(&WINDOW_MAXIMIZED),
            },
            settings_window,
            jobs_window,
            log: Log {
                keep: store.get(&LOG_KEEP),
                domains: store.get(&LOG_DOMAINS),
                window: log_window,
                size: store.get(&LOG_WINDOW_SIZE),
                folder: store.get(&LOG_WINDOW_FOLDER).map(PathBuf::from),
            },
            tab_names: store.get(&TOOLS_PANEL_TAB_NAMES),
            wheel_scrolls_panel: store.get(&TOOLS_PANEL_WHEEL_SCROLLS_PANEL),
            show_hidden: store.get(&TOOLS_PANEL_SHOW_HIDDEN),
            panel_position: store.get(&TOOLS_PANEL_POSITION),
            panel_width: store.kept(&TOOLS_PANEL_WIDTH),
            viewer: Viewer {
                margin: store.get(&VIEWER_MARGIN),
                proof_margin: store.get(&VIEWER_PROOF_MARGIN),
                background: store.kept(&VIEWER_BACKGROUND),
            },
            workspace: store.get(&WORKSPACE),
            edit: ViewState {
                sections: store.get(&EDIT_SECTIONS),
                groups: store.get(&EDIT_GROUPS),
            },
            scopes: ScopeView {
                scope: store.get(&SCOPES_GRAPH),
                scale: store.get(&SCOPES_SCALE),
                // Each named once or more; the others hidden.
                channels: Channel::ALL.map(|channel| shown.contains(&channel)),
                shadow_clipping: store.get(&SCOPES_SHADOW_CLIPPING),
                highlight_clipping: store.get(&SCOPES_HIGHLIGHT_CLIPPING),
            },
            browser: BrowserSettings {
                sort: Sort {
                    key: store.get(&BROWSER_SORT),
                    descending: store.get(&BROWSER_DESCENDING),
                },
                thumbnails: store.get(&BROWSER_THUMBNAILS),
                rows: store.get(&BROWSER_ROWS) as usize,
                position: store.get(&BROWSER_POSITION),
                columns: store.get(&BROWSER_COLUMNS) as usize,
                labels: store.get(&BROWSER_LABELS),
                square_thumbnails: store.get(&BROWSER_SQUARE_THUMBNAILS),
            },
            keys: store.get(&KEYS),
            export: Values::read(store),
            tool_settings: ToolSettings::read(store),
            ask_before_overwrite: store.get(&EXPORT_ASK_BEFORE_OVERWRITE),
            ask_before_discard: store.get(&HISTORY_ASK_BEFORE_DISCARD),
            show_last_pick: store.get(&CURSORS_PICK_SHOW_LAST),
        }
    }

    /// Sets in `store` what differs from the app's reading of it ([`Settings::read`]). A
    /// setting `--conf` gives that the user changed, as the scale by its keys, is offered
    /// each frame and refused by the store, so the file keeps its own value.
    fn write(&self, store: &mut Store) {
        let (s, was) = (self, &Settings::read(store));
        if s.theme != was.theme {
            store.set(&THEME, s.theme);
        }
        if s.locale != was.locale {
            keep(store, &LOCALE, s.locale);
        }
        if s.scale != was.scale {
            store.set(&SCALE, s.scale);
        }
        if s.marker != was.marker {
            store.set(&SLIDERS_MARKER, s.marker);
        }
        if s.menu_button != was.menu_button {
            store.set(&TOOLBAR_MENU_BUTTON, s.menu_button);
        }
        if s.toolbar_labels != was.toolbar_labels {
            store.set(&TOOLBAR_LABELS, s.toolbar_labels);
        }
        if s.window.size != was.window.size {
            keep(store, &WINDOW_SIZE, s.window.size);
        }
        // Each position and its scale factor, kept and removed together: neither while
        // `--conf` gives one, so the file never holds one without the other.
        if s.window.position != was.window.position {
            keep_position(
                store,
                &WINDOW_POSITION,
                &WINDOW_SCALE_FACTOR,
                s.window.position,
            );
        }
        if s.settings_window != was.settings_window {
            let (pixels, scale_factor) = (&SETTINGS_WINDOW_POSITION, &SETTINGS_WINDOW_SCALE_FACTOR);
            keep_position(store, pixels, scale_factor, s.settings_window);
        }
        if s.jobs_window != was.jobs_window {
            let (pixels, scale_factor) = (&JOBS_WINDOW_POSITION, &JOBS_WINDOW_SCALE_FACTOR);
            keep_position(store, pixels, scale_factor, s.jobs_window);
        }
        let (log, before) = (&s.log, &was.log);
        if log.keep != before.keep {
            store.set(&LOG_KEEP, log.keep);
        }
        if log.domains != before.domains {
            store.set(&LOG_DOMAINS, log.domains.clone());
        }
        if log.window != before.window {
            let (pixels, scale_factor) = (&LOG_WINDOW_POSITION, &LOG_WINDOW_SCALE_FACTOR);
            keep_position(store, pixels, scale_factor, log.window);
        }
        if log.size != before.size {
            store.set(&LOG_WINDOW_SIZE, log.size);
        }
        if log.folder != before.folder {
            let folder = log.folder.as_ref();
            store.set(
                &LOG_WINDOW_FOLDER,
                folder.map(|f| f.to_string_lossy().into_owned()),
            );
        }
        if s.window.maximized != was.window.maximized {
            store.set(&WINDOW_MAXIMIZED, s.window.maximized);
        }
        if s.tab_names != was.tab_names {
            store.set(&TOOLS_PANEL_TAB_NAMES, s.tab_names);
        }
        if s.wheel_scrolls_panel != was.wheel_scrolls_panel {
            store.set(&TOOLS_PANEL_WHEEL_SCROLLS_PANEL, s.wheel_scrolls_panel);
        }
        if s.show_hidden != was.show_hidden {
            store.set(&TOOLS_PANEL_SHOW_HIDDEN, s.show_hidden);
        }
        if s.panel_position != was.panel_position {
            store.set(&TOOLS_PANEL_POSITION, s.panel_position);
        }
        if s.panel_width != was.panel_width {
            keep(store, &TOOLS_PANEL_WIDTH, s.panel_width);
        }
        if s.viewer.margin != was.viewer.margin {
            store.set(&VIEWER_MARGIN, s.viewer.margin);
        }
        if s.viewer.proof_margin != was.viewer.proof_margin {
            store.set(&VIEWER_PROOF_MARGIN, s.viewer.proof_margin);
        }
        if s.viewer.background != was.viewer.background {
            keep(store, &VIEWER_BACKGROUND, s.viewer.background);
        }
        if s.workspace != was.workspace {
            store.set(&WORKSPACE, s.workspace.clone());
        }
        if s.edit.sections != was.edit.sections {
            store.set(&EDIT_SECTIONS, s.edit.sections.clone());
        }
        if s.edit.groups != was.edit.groups {
            store.set(&EDIT_GROUPS, s.edit.groups.clone());
        }
        let (view, before) = (s.scopes, was.scopes);
        if view.scope != before.scope {
            store.set(&SCOPES_GRAPH, view.scope);
        }
        if view.scale != before.scale {
            store.set(&SCOPES_SCALE, view.scale);
        }
        if view.channels != before.channels {
            let shown = Channel::ALL.into_iter().filter(|c| view.shows(*c));
            store.set(&SCOPES_CHANNELS, shown.collect());
        }
        if view.shadow_clipping != before.shadow_clipping {
            store.set(&SCOPES_SHADOW_CLIPPING, view.shadow_clipping);
        }
        if view.highlight_clipping != before.highlight_clipping {
            store.set(&SCOPES_HIGHLIGHT_CLIPPING, view.highlight_clipping);
        }
        let (browser, before) = (s.browser, was.browser);
        if browser.sort.key != before.sort.key {
            store.set(&BROWSER_SORT, browser.sort.key);
        }
        if browser.sort.descending != before.sort.descending {
            store.set(&BROWSER_DESCENDING, browser.sort.descending);
        }
        if browser.thumbnails != before.thumbnails {
            store.set(&BROWSER_THUMBNAILS, browser.thumbnails);
        }
        if browser.rows != before.rows {
            store.set(&BROWSER_ROWS, browser.rows as f32);
        }
        if browser.position != before.position {
            store.set(&BROWSER_POSITION, browser.position);
        }
        if browser.columns != before.columns {
            store.set(&BROWSER_COLUMNS, browser.columns as f32);
        }
        if browser.labels != before.labels {
            store.set(&BROWSER_LABELS, browser.labels);
        }
        if browser.square_thumbnails != before.square_thumbnails {
            store.set(&BROWSER_SQUARE_THUMBNAILS, browser.square_thumbnails);
        }
        if s.keys != was.keys {
            store.set(&KEYS, s.keys.clone());
        }
        if s.export != was.export {
            s.export.write(store);
        }
        if s.tool_settings != was.tool_settings {
            s.tool_settings.write(store, &was.tool_settings);
        }
        if s.ask_before_overwrite != was.ask_before_overwrite {
            store.set(&EXPORT_ASK_BEFORE_OVERWRITE, s.ask_before_overwrite);
        }
        if s.ask_before_discard != was.ask_before_discard {
            store.set(&HISTORY_ASK_BEFORE_DISCARD, s.ask_before_discard);
        }
        if s.show_last_pick != was.show_last_pick {
            store.set(&CURSORS_PICK_SHOW_LAST, s.show_last_pick);
        }
    }
}

/// The tools' settings (decision 29; `docs/pickers.md`, A tool's settings): each kept
/// setting of [`apertrust_tools::settings::registry`], by its tool and its id there,
/// as the value its row (`Row::Setting`) shows and the picker's commands carry, and a
/// reference row's two (`Row::Reference`, decision 88), its L and its photo's path, each
/// optional, none until a pick measures one and none again once cleared. Kept by the app
/// as darktable keeps its darktablerc, never in an edit, and sent with
/// `PickerCommand::On` and `Settings`, a photo's path never. One `--conf` gives keeps its
/// value for the run, as the store keeps it (decision 43): a set of it is refused, a
/// measured value's too, and its row is drawn disabled.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolSettings {
    values: BTreeMap<(&'static str, &'static str), ControlValue>,
    /// A reference's photo's path, by its tool and its setting.
    texts: BTreeMap<(&'static str, &'static str), String>,
    /// Those `--conf` gives for this run.
    overridden: BTreeSet<(&'static str, &'static str)>,
}

/// Each kept setting at its default, darktable's; a reference none.
impl Default for ToolSettings {
    fn default() -> ToolSettings {
        ToolSettings {
            values: kept()
                .filter_map(|(tool, id, kind, _)| match kind {
                    Kept::Row(row) => Some(((tool, id), row.default())),
                    Kept::Optional | Kept::Text => None,
                })
                .collect(),
            texts: BTreeMap::new(),
            overridden: BTreeSet::new(),
        }
    }
}

impl ToolSettings {
    /// None kept, nothing overridden.
    pub const fn new() -> ToolSettings {
        ToolSettings {
            values: BTreeMap::new(),
            texts: BTreeMap::new(),
            overridden: BTreeSet::new(),
        }
    }

    /// Every kept setting's value in `store`, read by its row's kind: a number, the
    /// option a choice's id names, or a boolean; one that cannot be read is its default,
    /// a reference's none. Those `--conf` gives are marked so.
    pub fn read(store: &Store) -> ToolSettings {
        let mut settings = ToolSettings::new();
        for (tool, id, kind, declared) in kept() {
            let value = store.value(declared);
            match kind {
                Kept::Row(row) => {
                    let value = value.as_ref().and_then(|value| control_value(row, value));
                    let value = value.unwrap_or_else(|| row.default());
                    settings.values.insert((tool, id), value);
                }
                Kept::Optional => {
                    let number = value.as_ref().and_then(toml::Value::as_float).or_else(|| {
                        value
                            .as_ref()
                            .and_then(toml::Value::as_integer)
                            .map(|n| n as f64)
                    });
                    if let Some(number) = number {
                        let number = ControlValue::Number(number as f32);
                        settings.values.insert((tool, id), number);
                    }
                }
                Kept::Text => {
                    if let Some(text) = value.as_ref().and_then(toml::Value::as_str) {
                        settings.texts.insert((tool, id), text.to_owned());
                    }
                }
            }
            if store.overridden(declared) {
                settings.overridden.insert((tool, id));
            }
        }
        // A reference's L `--conf` gives was measured on no photo the file names: the
        // photo left out for the run, the file keeping it (decision 88).
        for (tool, lightness, photo) in references() {
            if settings.overridden(tool, lightness) {
                settings.texts.remove(&(tool, photo));
            }
        }
        settings
    }

    /// Whether `--conf` gives `tool`'s setting `setting` for this run.
    pub fn overridden(&self, tool: &str, setting: &str) -> bool {
        (self.overridden.iter()).any(|&(m, s)| m == tool && s == setting)
    }

    /// Sets in `store` each value that differs from `was`, and removes a reference's that
    /// was cleared.
    fn write(&self, store: &mut Store, was: &ToolSettings) {
        for (tool, id, kind, declared) in kept() {
            let value = match kind {
                Kept::Row(row) => {
                    let value = self.get(tool, id);
                    if value == was.get(tool, id) {
                        continue;
                    }
                    value.and_then(|value| toml_value(row, value))
                }
                Kept::Optional => {
                    let value = self.get(tool, id);
                    if value == was.get(tool, id) {
                        continue;
                    }
                    match value {
                        Some(ControlValue::Number(n)) => Some(toml::Value::Float(f64::from(*n))),
                        _ => None,
                    }
                }
                Kept::Text => {
                    let text = self.text(tool, id);
                    if text == was.text(tool, id) {
                        continue;
                    }
                    text.map(|text| toml::Value::String(text.to_owned()))
                }
            };
            match value {
                Some(value) => {
                    store.set_value(declared, &value);
                }
                None if !matches!(kind, Kept::Row(_)) => store.reset_value(declared),
                None => {}
            }
        }
    }

    /// `tool`'s setting `setting`, if it is kept and has a value.
    pub fn get(&self, tool: &str, setting: &str) -> Option<&ControlValue> {
        self.values
            .iter()
            .find(|((m, s), _)| *m == tool && *s == setting)
            .map(|(_, value)| value)
    }

    /// `tool`'s text setting `setting`, a reference's photo's path, if it has one.
    pub fn text(&self, tool: &str, setting: &str) -> Option<&str> {
        self.texts
            .iter()
            .find(|((m, s), _)| *m == tool && *s == setting)
            .map(|(_, text)| text.as_str())
    }

    /// Sets `tool`'s setting `setting`, if it is kept, a number or a choice, and `--conf`
    /// does not give it.
    pub fn set(&mut self, tool: &str, setting: &str, value: ControlValue) {
        if self.overridden(tool, setting) {
            return;
        }
        let found = kept()
            .find(|(m, s, kind, _)| *m == tool && *s == setting && !matches!(kind, Kept::Text));
        if let Some((tool, setting, _, _)) = found {
            self.values.insert((tool, setting), value);
        }
    }

    /// Sets `tool`'s text setting `setting`, if it is kept and `--conf` does not give it.
    pub fn set_text(&mut self, tool: &str, setting: &str, text: String) {
        if self.overridden(tool, setting) {
            return;
        }
        let found = kept()
            .find(|(m, s, kind, _)| *m == tool && *s == setting && matches!(kind, Kept::Text));
        if let Some((tool, setting, _, _)) = found {
            self.texts.insert((tool, setting), text);
        }
    }

    /// Sets `tool`'s reference whole (decision 88): its L in `lightness` and the path of
    /// the photo it was measured on in `photo`, or no path where none is known, so an L
    /// never stays beside another photo's path. Both or neither: nothing where `--conf`
    /// gives either.
    pub fn set_reference(
        &mut self,
        tool: &str,
        (lightness, photo): (&str, &str),
        value: ControlValue,
        path: Option<String>,
    ) {
        if self.overridden(tool, lightness) || self.overridden(tool, photo) {
            return;
        }
        self.set(tool, lightness, value);
        match path {
            Some(path) => self.set_text(tool, photo, path),
            None => self.clear(tool, photo),
        }
    }

    /// Clears `tool`'s reference whole, its L and its photo's path: both or neither,
    /// nothing where `--conf` gives either.
    pub fn clear_reference(&mut self, tool: &str, (lightness, photo): (&str, &str)) {
        if self.overridden(tool, lightness) || self.overridden(tool, photo) {
            return;
        }
        self.clear(tool, lightness);
        self.clear(tool, photo);
    }

    /// Clears `tool`'s optional setting `setting`, a reference's L or its photo, unless
    /// `--conf` gives it.
    pub fn clear(&mut self, tool: &str, setting: &str) {
        if self.overridden(tool, setting) {
            return;
        }
        let optional = kept()
            .any(|(m, s, kind, _)| m == tool && s == setting && !matches!(kind, Kept::Row(_)));
        if optional {
            self.values
                .retain(|(m, s), _| !(*m == tool && *s == setting));
            self.texts
                .retain(|(m, s), _| !(*m == tool && *s == setting));
        }
    }

    /// Every text setting's value, the references' photos' paths.
    pub fn texts(&self) -> impl Iterator<Item = &str> {
        self.texts.values().map(String::as_str)
    }

    /// `tool`'s kept settings that have a value, as its picker's commands carry them: no
    /// text, which no pick reads.
    pub fn of(&self, tool: &str) -> Vec<(&'static str, ControlValue)> {
        (self.values.iter())
            .filter(|((m, _), _)| *m == tool)
            .map(|((_, setting), value)| (*setting, value.clone()))
            .collect()
    }
}

/// How a kept tool setting is kept: as its row's kind, as an optional number, a
/// reference's L, or as an optional text, a reference's photo's path.
#[derive(Clone, Copy)]
enum Kept {
    Row(&'static SettingRow),
    Optional,
    Text,
}

/// Each tool's reference (`Row::Reference`): its tool's id, its L's setting and its photo's.
fn references() -> impl Iterator<Item = (&'static str, &'static str, &'static str)> {
    TOOLS.iter().flat_map(|tool| {
        let id = tool.definition().tool.id;
        tool.definition().rows().filter_map(move |row| match row {
            Row::Reference {
                lightness, photo, ..
            } => Some((id, *lightness, *photo)),
            _ => None,
        })
    })
}

/// Each kept tool setting: its tool's id, its id, how it is kept and its declaration.
fn kept() -> impl Iterator<Item = (&'static str, &'static str, Kept, &'static dyn Declared)> {
    TOOLS.iter().flat_map(|tool| {
        let id = tool.definition().tool.id;
        let declared = move |setting: &'static str, kind: Kept| {
            apertrust_tools::settings::setting(id, setting).map(|d| (id, setting, kind, d))
        };
        tool.definition()
            .rows()
            .flat_map(move |row| match row {
                Row::Setting(setting) => vec![declared(setting.id, Kept::Row(setting))],
                Row::Reference {
                    lightness, photo, ..
                } => vec![
                    declared(lightness, Kept::Optional),
                    declared(photo, Kept::Text),
                ],
                _ => Vec::new(),
            })
            .flatten()
    })
}

/// A tool setting's value as `settings.toml` keeps it, read as its row's kind.
fn control_value(row: &SettingRow, value: &toml::Value) -> Option<ControlValue> {
    match row.kind {
        ControlKind::Number { .. } => match value {
            toml::Value::Float(n) => Some(ControlValue::Number(*n as f32)),
            toml::Value::Integer(n) => Some(ControlValue::Number(*n as f32)),
            _ => None,
        },
        ControlKind::Choice { options, .. } => {
            let id = value.as_str()?;
            let index = options.iter().position(|option| *option == id)?;
            Some(ControlValue::Choice(index as u32))
        }
        ControlKind::Bool { .. } => value.as_bool().map(ControlValue::Bool),
        _ => None,
    }
}

/// A tool setting's value as `settings.toml` keeps it.
fn toml_value(row: &SettingRow, value: &ControlValue) -> Option<toml::Value> {
    match (row.kind, value) {
        (ControlKind::Number { .. }, ControlValue::Number(n)) => {
            Some(toml::Value::Float(f64::from(*n)))
        }
        (ControlKind::Choice { options, .. }, ControlValue::Choice(index)) => options
            .get(*index as usize)
            .map(|option| toml::Value::String((*option).to_owned())),
        (ControlKind::Bool { .. }, ControlValue::Bool(on)) => Some(toml::Value::Boolean(*on)),
        _ => None,
    }
}

/// Keeps `position` in `pixels` and `scale_factor`, or removes both for None, unless
/// `--conf` gives either, so the file never holds one without the other.
fn keep_position(
    store: &mut Store,
    pixels: &Setting<Option<[f32; 2]>>,
    scale_factor: &Setting<Option<f32>>,
    position: Option<Position>,
) {
    if store.overridden(pixels) || store.overridden(scale_factor) {
        return;
    }
    store.set(pixels, position.map(|p| p.pixels));
    store.set(scale_factor, position.map(|p| p.scale_factor));
}

/// Keeps `value` for `setting`, or nothing for None, so that it follows its default.
fn keep<T: Stored>(store: &mut Store, setting: &Setting<T>, value: Option<T::Owned>) {
    match value {
        Some(value) => store.set(setting, value),
        None => store.reset(setting),
    }
}

/// What the app cannot use of what `store` read, beyond what the store itself warned of,
/// each after where it came from, the file or `--conf`: a position without its scale
/// factor, the main window's, Settings', the jobs window's or the log window's.
fn warnings(store: &Store) -> Vec<(&str, String)> {
    [
        (&WINDOW_POSITION, &WINDOW_SCALE_FACTOR),
        (&SETTINGS_WINDOW_POSITION, &SETTINGS_WINDOW_SCALE_FACTOR),
        (&JOBS_WINDOW_POSITION, &JOBS_WINDOW_SCALE_FACTOR),
        (&LOG_WINDOW_POSITION, &LOG_WINDOW_SCALE_FACTOR),
    ]
    .into_iter()
    .filter(|(pixels, scale_factor)| {
        store.get(*pixels).is_some() && store.get(*scale_factor).is_none()
    })
    .map(|(pixels, _)| {
        let from = if store.overridden(pixels) {
            CONF_FLAG
        } else {
            store.location().unwrap_or_default()
        };
        (from, format!("{} has no scale factor; not used", pixels.id))
    })
    .collect()
}

/// The margins round the Viewer's photo and its caption, and the background behind
/// them (`[viewer]`, which no page of the Settings dialog shows; `docs/gui.md`, The
/// photo frame).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewer {
    /// The default margin, in points.
    pub margin: f32,
    /// The margin while Proof margin is on, as a fraction of the slot's smaller side.
    pub proof_margin: f32,
    /// The background chosen behind the photo; None for the theme's own.
    pub background: Option<apertrust_ui::Background>,
}

/// The log the app keeps in memory, `[log]`, the Settings dialog's Diagnostics, and what
/// its window keeps, `[log_window]` (`docs/gui.md`, The log window).
#[derive(Clone, Debug, PartialEq)]
pub struct Log {
    /// Whether the session's log is kept, from the moment it is on.
    pub keep: bool,
    /// The domains it keeps, and `verbose`'s detail where it holds it, in `-d`'s order.
    pub domains: Vec<apertrust_base::log::Domain>,
    /// Where the log window was last left; None opens it centred over the main window.
    pub window: Option<Position>,
    /// The size it was last left at, in points; None opens it at its design's.
    pub size: Option<[f32; 2]>,
    /// The folder Save to file… last wrote to; None opens its dialog in Downloads.
    pub folder: Option<PathBuf>,
}

impl Default for Log {
    fn default() -> Self {
        Log {
            keep: LOG_KEEP.default(),
            domains: LOG_DOMAINS.default().to_vec(),
            window: None,
            size: None,
            folder: None,
        }
    }
}

impl Default for Viewer {
    fn default() -> Self {
        Viewer {
            margin: VIEWER_MARGIN.default(),
            proof_margin: VIEWER_PROOF_MARGIN.default(),
            background: None,
        }
    }
}

/// How the Browser shows the folder's photos (`[browser]`, `docs/gui.md`, The
/// Browser and The browser bar).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrowserSettings {
    /// The order the photos are shown in.
    pub sort: Sort,
    /// The thumbnails' size.
    pub thumbnails: ThumbnailSize,
    /// The rows it shows at the bottom, as its edge was dragged.
    pub rows: usize,
    /// Where it is in the window.
    pub position: BrowserPosition,
    /// The columns it shows at the left or right, as its edge was dragged.
    pub columns: usize,
    /// Whether the thumbnails show their captions: the browser bar's Labels.
    pub labels: bool,
    /// Whether each thumbnail fills its square, cut at its longer sides.
    pub square_thumbnails: bool,
}

impl BrowserSettings {
    /// Whether `chosen` is what it holds, which the menus show checked.
    pub fn holds(&self, chosen: BrowserBarAction) -> bool {
        match chosen {
            BrowserBarAction::Sort(key) => self.sort.key == key,
            BrowserBarAction::Descending(descending) => self.sort.descending == descending,
            BrowserBarAction::Size(size) => self.thumbnails == size,
            BrowserBarAction::Position(position) => self.position == position,
            BrowserBarAction::Labels => self.labels,
        }
    }

    /// How many lines of cells it keeps where it is: rows at the bottom, columns at a side.
    pub fn lines(&self) -> usize {
        if self.position.is_column() {
            self.columns
        } else {
            self.rows
        }
    }

    /// Keeps `lines` lines of cells where it is.
    pub fn keep_lines(&mut self, lines: usize) {
        if self.position.is_column() {
            self.columns = lines;
        } else {
            self.rows = lines;
        }
    }

    /// Takes `chosen` in: Labels turns the captions on or off.
    pub fn apply(&mut self, chosen: BrowserBarAction) {
        match chosen {
            BrowserBarAction::Sort(key) => self.sort.key = key,
            BrowserBarAction::Descending(descending) => self.sort.descending = descending,
            BrowserBarAction::Size(size) => self.thumbnails = size,
            BrowserBarAction::Position(position) => self.position = position,
            BrowserBarAction::Labels => self.labels = !self.labels,
        }
    }
}

impl Default for BrowserSettings {
    fn default() -> Self {
        BrowserSettings {
            sort: Sort {
                key: BROWSER_SORT.default(),
                descending: BROWSER_DESCENDING.default(),
            },
            thumbnails: BROWSER_THUMBNAILS.default(),
            rows: BROWSER_ROWS.default() as usize,
            position: BROWSER_POSITION.default(),
            columns: BROWSER_COLUMNS.default() as usize,
            labels: BROWSER_LABELS.default(),
            square_thumbnails: BROWSER_SQUARE_THUMBNAILS.default(),
        }
    }
}

/// The window's placement, as it last stood while neither maximised nor full screen.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Window {
    /// The inner size, in logical pixels.
    pub size: Option<[f32; 2]>,
    /// The outer position, where the platform reports one (Wayland never does).
    pub position: Option<Position>,
    pub maximized: bool,
}

/// Where the window's corner was: in physical pixels, which Windows and Linux place windows
/// in and which are the same on every monitor there, with the display's scale factor, which
/// turns them into the points macOS places windows in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub pixels: [f32; 2],
    pub scale_factor: f32,
}

/// A view's own state: the tab last shown is `state.toml`'s, by workspace (decision 96).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ViewState {
    /// The sections the user opened or closed, by id; the others start as their section
    /// does.
    pub sections: BTreeMap<String, bool>,
    /// The collapsible groups the user opened or closed, by tool and group, the same
    /// for every photo; the others start closed.
    pub groups: BTreeMap<String, BTreeMap<String, bool>>,
}

/// The settings the app's store reads: its own, then the engine's, which the app hands
/// the engine as options (decision 43), then the export settings the Export tool edits
/// (decision 56).
fn registries() -> Vec<&'static dyn Declared> {
    let engine = engine::registry::ALL.iter();
    let export = apertrust_export::settings::registry::ALL.iter();
    let tools = apertrust_tools::settings::registry::ALL.iter();
    registry::ALL
        .iter()
        .chain(engine)
        .chain(export)
        .chain(tools)
        .copied()
        .collect()
}

/// The settings the Settings dialog shows as rows, every owner's the schema puts on a page,
/// in the schema's order (decision 43).
pub fn shown() -> Vec<&'static dyn Declared> {
    let mut shown: Vec<&'static dyn Declared> = registries()
        .into_iter()
        .filter(|setting| setting.page().is_some())
        .collect();
    shown.sort_by_key(|setting| setting.place());
    shown
}

/// `--conf KEY=VALUE`'s, `given` in order, read against the settings the app's store reads,
/// its own and the engine's, before anything else is (decision 43): a key that is none
/// of them, or a value that cannot be read for its kind, is an error, and the app does not
/// start.
pub fn overrides(given: &[String]) -> Result<Overrides, OverrideError> {
    Overrides::read(given, &registries())
}

/// Keeps the app's settings in the store: reads them once, then hands each frame's to it,
/// which writes them on a thread of its own once they have stood still, and on exit.
pub struct Keeper {
    store: Store,
}

impl Keeper {
    /// Reads the settings from `storage`, `overrides` over them for this run, logging what
    /// it could not use, and starts the store's writer unless the storage must not be
    /// written.
    pub fn open_with(storage: Box<dyn Storage>, overrides: &Overrides) -> (Keeper, Settings) {
        Keeper::keep(Store::open(storage, &registries()), overrides)
    }

    /// Reads the settings from `settings.toml` in the user data folder of `folders`, as
    /// [`open_with`](Keeper::open_with) does, or keeps nothing where there is none
    /// ([`Folders::find`]), `overrides` over them either way.
    pub fn open_in(folders: Option<&Folders>, overrides: &Overrides) -> (Keeper, Settings) {
        match folders {
            Some(folders) => {
                let storage = Box::new(FileStorage::new(folders.settings()));
                Keeper::open_with(storage, overrides)
            }
            None => Keeper::keep(Store::nowhere(&registries()), overrides),
        }
    }

    /// [`open_with`](Keeper::open_with) with nothing set for the run, as tests open them.
    #[cfg(test)]
    pub fn open(storage: Box<dyn Storage>) -> (Keeper, Settings) {
        Keeper::open_with(storage, &Overrides::default())
    }

    /// A keeper with nowhere to keep anything, for a system that gives no folder for
    /// settings: the defaults, never written.
    #[cfg(test)]
    pub fn nowhere() -> (Keeper, Settings) {
        Keeper::open_in(None, &Overrides::default())
    }

    /// Keeps `store`'s settings with `overrides` over them, logging what it could not use
    /// and each of the app's own settings `overrides` give, under `-d settings`; the
    /// engine's are logged as [`engine`](Keeper::engine) reads them.
    fn keep(store: Store, overrides: &Overrides) -> (Keeper, Settings) {
        let store = store.with_overrides(overrides);
        for (from, warning) in warnings(&store) {
            tracing::warn!(target: "settings", "{from}: {warning}");
        }
        for &setting in registry::ALL {
            if let Some(value) = overrides.value(setting) {
                let id = setting.id();
                tracing::debug!(target: "settings", "{id}: {value}, from: {CONF_FLAG}");
            }
        }
        let settings = Settings::read(&store);
        (Keeper { store }, settings)
    }

    /// Whether `--conf` sets `setting` for this run, so the Settings dialog draws its row
    /// disabled, saying why (decision 43).
    pub fn overridden(&self, setting: &dyn Declared) -> bool {
        self.store.overridden(setting)
    }

    /// `setting`'s value, as `settings.toml` keeps it, for the Settings dialog's row.
    pub fn value(&self, setting: &dyn Declared) -> Option<toml::Value> {
        self.store.value(setting)
    }

    /// The recent folders an earlier build kept in settings, `library.recent_folders`,
    /// newest first: read only for their move into `state.toml`, where it has no list yet
    /// ([`crate::recents`]), and never written.
    pub fn old_recent_folders(&self) -> Vec<PathBuf> {
        let folders = self.store.get(&LIBRARY_RECENT_FOLDERS).into_iter();
        folders.map(PathBuf::from).collect()
    }

    /// The Tools panel's tab an earlier build kept in settings, `edit.tab`: read only for
    /// its move into `state.toml`, where it holds no tab for the Default workspace
    /// (`Recents::move_tab`), and never written: the file's own, never a `--conf` value for
    /// one run.
    pub fn old_tab(&self) -> Option<String> {
        self.store.kept_in_file(&EDIT_TAB)
    }

    /// Whether the start reopens what was open last, `library.reopen_last`, read once as
    /// the app starts (`docs/gui.md`, The start window).
    pub fn reopen_last(&self) -> bool {
        self.store.get(&LIBRARY_REOPEN_LAST)
    }

    /// The Settings dialog's page last shown.
    pub fn page(&self) -> apertrust_base::settings::Page {
        self.store.get(&SETTINGS_WINDOW_PAGE)
    }

    /// Takes in what the user changed in the Settings dialog: the setting `id` set to
    /// `value`, as `settings.toml` keeps it, or reset for none, after a frame's `settings`,
    /// so nothing the shell changed meanwhile is lost. Returns the settings as they are
    /// then, for the shell to show. A setting no registry of the store's has, or a value its
    /// kind cannot read, changes nothing, with a warning; one `--conf` gives is refused, as
    /// the store refuses every set of it.
    pub fn apply(
        &mut self,
        settings: &Settings,
        id: &str,
        value: Option<&toml::Value>,
    ) -> Settings {
        settings.write(&mut self.store);
        match (registries().into_iter().find(|s| s.id() == id), value) {
            (Some(setting), Some(value)) => {
                if !self.store.set_value(setting, value) {
                    tracing::warn!(target: "settings", "{id} cannot be {value}; not kept");
                }
            }
            (Some(setting), None) => self.store.reset_value(setting),
            (None, _) => tracing::warn!(target: "settings", "no setting {id}; not kept"),
        }
        let mut updated = Settings::read(&self.store);
        // Zoom keys and the panel edge remain usable when --conf chose their initial
        // geometry. An unrelated dialog row must not restore that initial value.
        if self.store.overridden(&SCALE) {
            updated.scale = settings.scale;
        }
        if self.store.overridden(&TOOLS_PANEL_WIDTH) {
            updated.panel_width = settings.panel_width;
        }
        updated
    }

    /// The engine's settings as `--conf` and the file give them, as the plain options the
    /// engine takes, each logged under `-d settings` with where it came from: `--conf`,
    /// the file or the default, since the app has no flag over them; wgpu's `WGPU_BACKEND`
    /// before `engine.backend`.
    pub fn engine(&self) -> engine::Options {
        engine::Options::read(&self.store, &engine::Flags::default())
    }

    /// The engine's settings as [`Keeper::engine`] gives them, but for the graphics card,
    /// `adapter`, and the graphics API, `api`, for this run, the file and `--conf` keeping
    /// theirs: `--safe-mode`, or a start after one that did not finish, `by` saying which
    /// (decision 82; `recovery`).
    pub fn engine_instead(
        &self,
        adapter: &str,
        api: apertrust_engine::gpu::Backend,
        by: &'static str,
    ) -> engine::Options {
        engine::Options::instead(&self.store, adapter, api, by)
    }

    /// The graphics card and graphics API settings as `--conf` and the file give them,
    /// unchecked: what a start tries to open its device with (`recovery`).
    pub fn graphics(&self) -> (String, apertrust_engine::gpu::Backend) {
        let adapter = self.store.get(&engine::registry::ENGINE_ADAPTER);
        let backend = self.store.get(&engine::registry::ENGINE_BACKEND);
        (adapter, backend)
    }

    /// The disk cache's limit as `cache.limited` and `cache.limit` give it now, which the
    /// session takes as it starts and again each time it changes (decision 76).
    pub fn cache_limit(&self) -> apertrust_base::cache::Limit {
        engine::cache_limit(&self.store)
    }

    /// From which size a photo with no edit shows a render of its raw, as
    /// `library.raw_thumbnails_from` gives it now, which the session is given as it starts
    /// and again each time it changes.
    pub fn raw_thumbnails(&self) -> apertrust_engine::session::library::RawThumbnails {
        self.store
            .get(&engine::registry::LIBRARY_RAW_THUMBNAILS_FROM)
    }

    /// Takes in a frame's settings at `now`, which the store writes once they have stood
    /// still for a second. Returns how long until they will have, for a frame then.
    pub fn frame(&mut self, now: Instant, settings: &Settings) -> Option<Duration> {
        settings.write(&mut self.store);
        self.store.poll(now)
    }

    /// On exit: has what changed since the last write written, and waits for it.
    pub fn flush(&mut self) {
        self.store.flush();
    }
}

/// What tests of settings share.
#[cfg(test)]
pub mod testing {
    use std::path::PathBuf;

    use apertrust_base::settings::{FileStorage, Storage};

    /// A folder of a test's own, empty, removed when the test ends.
    pub struct Folder(pub PathBuf);

    impl Folder {
        pub fn new(name: &str) -> Folder {
            let path = std::env::temp_dir()
                .join(format!("apertrust-settings-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("a temporary folder");
            Folder(path)
        }

        pub fn settings(&self) -> PathBuf {
            self.0.join("settings.toml")
        }

        pub fn storage(&self) -> Box<dyn Storage> {
            Box::new(FileStorage::new(self.settings()))
        }

        /// The names of the files in it, sorted.
        pub fn files(&self) -> Vec<String> {
            let mut names: Vec<String> = std::fs::read_dir(&self.0)
                .expect("the folder")
                .map(|e| {
                    e.expect("an entry")
                        .file_name()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect();
            names.sort();
            names
        }

        pub fn text(&self) -> String {
            std::fs::read_to_string(self.settings()).expect("the settings file")
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use apertrust_base::settings::DELAY;
    use apertrust_ui::components::{Scale, Scope, SortKey};
    use eframe::egui::os::OperatingSystem;
    use toml::{Table, Value};

    use super::testing::Folder;
    use super::*;
    use crate::PANEL_WIDTHS;
    use crate::actions::{ActionId, Keymap, Shortcut};

    /// Settings with every field off its default.
    fn everything() -> Settings {
        Settings {
            theme: Builtin::HighContrast,
            locale: Some(Locale::Pl),
            scale: 1.3,
            marker: Marker::Triangle,
            menu_button: MenuButtonStyle::Menu,
            toolbar_labels: false,
            window: Window {
                size: Some([1280.0, 800.0]),
                position: Some(Position {
                    pixels: [-80.0, 50.0],
                    scale_factor: 2.0,
                }),
                maximized: true,
            },
            settings_window: Some(Position {
                pixels: [960.0, 520.0],
                scale_factor: 2.0,
            }),
            jobs_window: Some(Position {
                pixels: [1200.0, 160.0],
                scale_factor: 2.0,
            }),
            log: Log {
                keep: true,
                domains: vec![
                    apertrust_base::log::Domain::GPU,
                    apertrust_base::log::Domain::MEMORY,
                ],
                window: Some(Position {
                    pixels: [400.0, 300.0],
                    scale_factor: 2.0,
                }),
                size: Some([720.0, 400.0]),
                folder: Some(PathBuf::from("/photos/logs")),
            },
            tab_names: false,
            wheel_scrolls_panel: false,
            show_hidden: true,
            panel_position: ToolsPanelPosition::Left,
            panel_width: Some(480.0),
            viewer: Viewer {
                margin: 16.0,
                proof_margin: 0.25,
                background: Some(apertrust_ui::Background::Medium),
            },
            workspace: "portraits".to_owned(),
            edit: ViewState {
                sections: BTreeMap::from([
                    ("exposure".to_owned(), false),
                    ("tone_curve".to_owned(), true),
                ]),
                groups: BTreeMap::from([(
                    "exposure".to_owned(),
                    BTreeMap::from([
                        ("area_exposure_mapping".to_owned(), true),
                        ("deflicker".to_owned(), false),
                    ]),
                )]),
            },
            scopes: ScopeView {
                scope: Scope::Parade,
                scale: Scale::Logarithmic,
                channels: [true, false, true, true],
                shadow_clipping: true,
                highlight_clipping: true,
            },
            browser: BrowserSettings {
                sort: Sort {
                    key: SortKey::FileName,
                    descending: true,
                },
                thumbnails: ThumbnailSize::Large,
                rows: 3,
                position: BrowserPosition::Right,
                columns: 2,
                labels: false,
                square_thumbnails: true,
            },
            keys: BTreeMap::from([
                ("undo".to_owned(), vec!["Ctrl+K".to_owned()]),
                ("quit".to_owned(), Vec::new()),
            ]),
            export: Values {
                path: "$(HOME)/proofs/$(FILE_NAME)".to_owned(),
                on_conflict: apertrust_export::settings::OnConflict::Skip,
                format: apertrust_export::settings::FileFormat::Png,
                jpeg_quality: 80,
                jpeg_subsample: apertrust_export::jpeg::Subsample::S420,
                png_bit_depth: apertrust_export::png::BitDepth::Sixteen,
                png_compression: 9,
                size_mode: apertrust_export::settings::SizeMode::Cm,
                width: 2048,
                height: 1365,
                dpi: 600,
                scale: "1/2".to_owned(),
                upscale: true,
                high_quality: true,
                profile: apertrust_export::settings::Profile::Srgb,
            },
            tool_settings: {
                let mut settings = ToolSettings::default();
                settings.set("exposure", "target", ControlValue::Choice(1));
                settings.set("exposure", "lightness", ControlValue::Number(42.5));
                let number = ControlValue::Number(62.5);
                settings.set("exposure", "reference_lightness", number);
                let photo = "/photos/IMG_2041.CR3".to_owned();
                settings.set_text("exposure", "reference_photo", photo);
                settings
            },
            ask_before_overwrite: false,
            ask_before_discard: false,
            show_last_pick: false,
        }
    }

    /// Opens `folder`'s settings, changes them to `changed` as a frame would, and ends as
    /// the app does on exit.
    fn change(folder: &Folder, changed: &Settings) {
        let (mut keeper, _) = Keeper::open(folder.storage());
        keeper.frame(Instant::now(), changed);
        keeper.flush();
    }

    /// The recent folders and the tab an earlier build kept are read for their move, and
    /// left in the file as they are when the settings are written again.
    #[test]
    fn the_old_recent_folders_are_read_for_the_move_and_left_alone() {
        let folder = Folder::new("old-recents");
        let text = "version = 1\n\n[edit]\ntab = \"raw\"\n\n[library]\nrecent_folders = [\"/photos/b\", \"/photos/a\"]\n";
        std::fs::write(folder.settings(), text).expect("written");
        let (mut keeper, settings) = Keeper::open(folder.storage());
        let old = [PathBuf::from("/photos/b"), PathBuf::from("/photos/a")];
        assert_eq!(keeper.old_recent_folders(), old);
        assert_eq!(keeper.old_tab().as_deref(), Some("raw"));
        let changed = Settings {
            tab_names: !settings.tab_names,
            ..settings
        };
        keeper.frame(Instant::now(), &changed);
        keeper.flush();
        assert!(
            folder
                .text()
                .contains("recent_folders = [\"/photos/b\", \"/photos/a\"]")
        );
        assert!(folder.text().contains("tab = \"raw\""));
        // A `--conf` value for one run never moves.
        let overrides = overrides(&["edit.tab=geometry".to_owned()]).expect("read");
        let (keeper, _) = Keeper::open_with(folder.storage(), &overrides);
        assert_eq!(keeper.old_tab().as_deref(), Some("raw"));
        let empty = Folder::new("old-tab-conf");
        let (keeper, _) = Keeper::open_with(empty.storage(), &overrides);
        assert_eq!(keeper.old_tab(), None);
    }

    /// The settings `text` holds, with what the store and the app warned of, each by the
    /// setting's id, in the order they were read.
    fn read(text: &str) -> (Settings, Vec<String>) {
        let folder = Folder::new(&format!("read-{:x}", hash(text)));
        std::fs::write(folder.settings(), text).expect("written");
        let (keeper, settings) = Keeper::open(folder.storage());
        let store = &keeper.store;
        let apps = warnings(store);
        let all = store.warnings().iter().chain(apps.iter().map(|(_, w)| w));
        let names = all.map(|w| w.split(' ').next().unwrap_or_default().to_owned());
        (settings, names.collect())
    }

    /// A name for a folder of `text`'s own, so tests that read at once never share one.
    fn hash(text: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn the_schema_s_defaults_are_those_the_app_had() {
        use apertrust_ui::Theme;
        assert_eq!(
            Settings::default(),
            Settings {
                theme: Builtin::Dark,
                locale: None,
                scale: 1.0,
                marker: Marker::Square,
                menu_button: MenuButtonStyle::Icon,
                // A new user's Toolbar shows its labels (decision 92).
                toolbar_labels: true,
                window: Window {
                    size: None,
                    position: None,
                    maximized: false,
                },
                settings_window: None,
                jobs_window: None,
                log: Log {
                    keep: false,
                    domains: ["gpu", "raw", "pipe", "perf", "settings", "control", "event"]
                        .map(|id| apertrust_base::settings::option(id, id))
                        .to_vec(),
                    window: None,
                    size: None,
                    folder: None,
                },
                tab_names: true,
                wheel_scrolls_panel: true,
                show_hidden: false,
                panel_position: ToolsPanelPosition::Right,
                panel_width: None,
                viewer: Viewer {
                    margin: 10.0,
                    proof_margin: 0.12,
                    background: None,
                },
                workspace: "default".to_owned(),
                edit: ViewState {
                    sections: BTreeMap::new(),
                    groups: BTreeMap::new(),
                },
                scopes: ScopeView::default(),
                browser: BrowserSettings {
                    sort: Sort {
                        key: SortKey::CaptureTime,
                        descending: false,
                    },
                    thumbnails: ThumbnailSize::Medium,
                    rows: 1,
                    position: BrowserPosition::Left,
                    columns: 1,
                    labels: true,
                    square_thumbnails: false,
                },
                keys: BTreeMap::new(),
                export: Values::default(),
                // Exposure's Picker target at Middle grey, Lightness at darktable's 50 %, and no
                // reference.
                tool_settings: ToolSettings {
                    values: BTreeMap::from([
                        (("exposure", "target"), ControlValue::Choice(0)),
                        (("exposure", "lightness"), ControlValue::Number(50.0)),
                    ]),
                    texts: BTreeMap::new(),
                    overridden: BTreeSet::new(),
                },
                ask_before_overwrite: true,
                ask_before_discard: true,
                show_last_pick: true,
            }
        );
        // What the app takes where nothing is kept.
        assert_eq!(LOCALE.default(), crate::system_locale());
        assert_eq!(TOOLS_PANEL_WIDTH.default(), 320.0);
        assert_eq!(
            TOOLS_PANEL_WIDTH.default(),
            Theme::builtin(Builtin::Dark).sizes.tools_panel
        );
        // The workspace shown where none is kept, or the one kept is none.
        assert_eq!(WORKSPACE.default(), crate::layout::DEFAULT);
        assert!(EDIT_SECTIONS.default().is_empty());
        assert!(EDIT_GROUPS.default().is_empty());
        assert!(KEYS.default().is_empty());
        // So the store reads what nothing keeps as they are.
        assert_eq!(Keeper::nowhere().1, Settings::default());
    }

    /// The settings the app's own settings write: the app's and the export settings the
    /// Export tool keeps. The engine's, and the page Settings last showed, the Settings
    /// dialog keeps through the keeper (`Keeper::apply`), and so is reopening what was open
    /// last, read once at the start (`Keeper::reopen_last`); the old recent folders and tab
    /// are only read, for their move into `state.toml` (`Keeper::old_recent_folders`,
    /// `Keeper::old_tab`).
    fn declared() -> impl Iterator<Item = &'static str> {
        let export = apertrust_export::settings::registry::ALL.iter();
        let tools = apertrust_tools::settings::registry::ALL.iter();
        let (page, old) = (SETTINGS_WINDOW_PAGE.id, LIBRARY_RECENT_FOLDERS.id);
        let (reopen, old_tab) = (LIBRARY_REOPEN_LAST.id, EDIT_TAB.id);
        registry::ALL
            .iter()
            .chain(export)
            .chain(tools)
            .map(|s| s.id())
            .filter(move |id| ![page, old, reopen, old_tab].contains(id))
    }

    /// Every setting of the schema is one of the app's or the export's, written once it
    /// changes, and the file holds no key the schema lacks.
    #[test]
    fn the_schema_declares_every_key_the_app_reads_and_no_other() {
        let folder = Folder::new("declared");
        change(&folder, &everything());
        let table: Table = folder.text().parse().expect("TOML");
        let mut written = Vec::new();
        let mut tables = vec![(String::new(), &table)];
        while let Some((prefix, table)) = tables.pop() {
            for (key, value) in table {
                let id = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                if declared().any(|s| s == id) {
                    written.push(id);
                } else if let Some(inner) = value.as_table() {
                    tables.push((id, inner));
                } else {
                    assert_eq!(id, "version", "a key the schema lacks");
                }
            }
        }
        written.sort();
        let mut declared: Vec<&str> = declared().collect();
        declared.sort();
        assert_eq!(written, declared);
    }

    /// Every key written as this version writes it, byte for byte: in the order of `toml`'s
    /// tables, never the schema's. A file of every key read back writes the same bytes, and
    /// a change to one leaves every other line as it was.
    #[test]
    fn every_key_is_written_byte_for_byte_as_before() {
        let folder = Folder::new("every-key");
        change(&folder, &everything());
        let expected = "\
# Apertrust's settings. Apertrust writes here only what changes, without comments;
# keys it does not know stay.

version = 2
locale = \"pl\"
scale = 1.3
theme = \"high-contrast\"
workspace = \"portraits\"

[browser]
columns = 2
descending = true
labels = false
position = \"right\"
rows = 3
sort = \"file-name\"
square_thumbnails = true
thumbnails = \"large\"

[cursors.pick]
show_last = false

[edit.groups.exposure]
area_exposure_mapping = true
deflicker = false

[edit.sections]
exposure = false
tone_curve = true

[export]
ask_before_overwrite = false
format = \"png\"
profile = \"srgb\"

[export.disk]
on_conflict = \"skip\"
path = \"$(HOME)/proofs/$(FILE_NAME)\"

[export.jpeg]
quality = 80
subsample = \"420\"

[export.png]
bit_depth = \"16\"
compression = 9

[export.size]
dpi = 600
height = 1365
high_quality = true
mode = \"cm\"
scale = \"1/2\"
upscale = true
width = 2048

[history]
ask_before_discard = false

[jobs_window]
position = [1200, 160]
scale_factor = 2.0

[keys]
quit = []
undo = [\"Ctrl+K\"]

[log]
domains = [\"gpu\", \"memory\"]
keep = true

[log_window]
folder = \"/photos/logs\"
position = [400, 300]
scale_factor = 2.0
size = [720, 400]

[scopes]
channels = [\"red\", \"blue\", \"luminance\"]
graph = \"parade\"
highlight_clipping = true
scale = \"logarithmic\"
shadow_clipping = true

[settings_window]
position = [960, 520]
scale_factor = 2.0

[sliders]
marker = \"triangle\"

[toolbar]
labels = false
menu_button = \"menu\"

[tools.exposure]
lightness = 42.5
reference_lightness = 62.5
reference_photo = \"/photos/IMG_2041.CR3\"
target = \"lightness\"

[tools_panel]
position = \"left\"
show_hidden = true
tab_names = false
wheel_scrolls_panel = false
width = 480

[viewer]
background = \"medium\"
margin = 16
proof_margin = 0.25

[window]
maximized = true
position = [-80, 50]
scale_factor = 2.0
size = [1280, 800]
";
        assert_eq!(folder.text(), expected);
        // Read back, and written afresh: the same bytes.
        let (read, warnings) = read(expected);
        assert_eq!((&read, warnings.as_slice()), (&everything(), &[][..]));
        let again = Folder::new("every-key-again");
        change(&again, &read);
        assert_eq!(again.text(), expected);
        // One change: one line.
        change(&folder, &Settings { scale: 1.5, ..read });
        assert_eq!(
            folder.text(),
            expected.replace("scale = 1.3", "scale = 1.5")
        );
    }

    /// A version-1 file, from before the window's parts were named by their role (decision
    /// 95): every renamed key, action id and Browse entry, under its old name.
    const VERSION_1: &str = "\
version = 1
scale = 1.3

[central_panel]
background = \"white\"
margin = 24
proof_margin = 0.3

[edit.groups.browse]
folders = false

[edit.sections]
browse = false
exposure = true

[grid_panel]
columns = 2
descending = true
labels = false
position = \"bottom\"
rows = 3
sort = \"file-name\"
square_thumbnails = true
thumbnails = \"large\"

[keys]
grid-position-bottom = [\"Ctrl+3\"]
grid-position-left = [\"Ctrl+1\"]
grid-position-right = [\"Ctrl+2\"]
show-browse = [\"Shift+L\"]
undo = [\"Ctrl+K\"]

[side_panel]
tab_names = false
width = 480
";

    /// A version-1 file keeps every value under its new name, and is written as version 2
    /// with the new names alone at its next write.
    #[test]
    fn a_version_1_file_keeps_every_value_under_the_new_names() {
        let (read, warnings) = read(VERSION_1);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(!read.tab_names);
        assert_eq!(read.panel_width, Some(480.0));
        assert_eq!(
            read.viewer,
            Viewer {
                margin: 24.0,
                proof_margin: 0.3,
                background: Some(apertrust_ui::Background::White),
            }
        );
        assert_eq!(
            read.browser,
            BrowserSettings {
                sort: Sort {
                    key: SortKey::FileName,
                    descending: true,
                },
                thumbnails: ThumbnailSize::Large,
                rows: 3,
                position: BrowserPosition::Bottom,
                columns: 2,
                labels: false,
                square_thumbnails: true,
            }
        );
        let keys = |keys: &[(&str, &str)]| {
            keys.iter()
                .map(|(id, key)| ((*id).to_owned(), vec![(*key).to_owned()]))
                .collect::<BTreeMap<_, _>>()
        };
        assert_eq!(
            read.keys,
            keys(&[
                ("browser-position-bottom", "Ctrl+3"),
                ("browser-position-left", "Ctrl+1"),
                ("browser-position-right", "Ctrl+2"),
                ("show-collections", "Shift+L"),
                ("undo", "Ctrl+K"),
            ])
        );
        assert_eq!(
            read.edit.sections,
            BTreeMap::from([
                ("collections".to_owned(), false),
                ("exposure".to_owned(), true)
            ])
        );
        assert_eq!(
            read.edit.groups,
            BTreeMap::from([(
                "collections".to_owned(),
                BTreeMap::from([("folders".to_owned(), false)])
            )])
        );
        // Opened and closed, the file stays version 1; one change writes it as version 2.
        let folder = Folder::new("version-1");
        std::fs::write(folder.settings(), VERSION_1).expect("written");
        let (keeper, _) = Keeper::open(folder.storage());
        drop(keeper);
        assert_eq!(folder.text(), VERSION_1);
        let changed = Settings { scale: 1.5, ..read };
        change(&folder, &changed);
        let expected = "\
# Apertrust's settings. Apertrust writes here only what changes, without comments;
# keys it does not know stay.

version = 2
scale = 1.5

[browser]
columns = 2
descending = true
labels = false
position = \"bottom\"
rows = 3
sort = \"file-name\"
square_thumbnails = true
thumbnails = \"large\"

[edit.groups.collections]
folders = false

[edit.sections]
collections = false
exposure = true

[keys]
browser-position-bottom = [\"Ctrl+3\"]
browser-position-left = [\"Ctrl+1\"]
browser-position-right = [\"Ctrl+2\"]
show-collections = [\"Shift+L\"]
undo = [\"Ctrl+K\"]

[tools_panel]
tab_names = false
width = 480

[viewer]
background = \"white\"
margin = 24
proof_margin = 0.3
";
        assert_eq!(folder.text(), expected);
        assert_eq!(Keeper::open(folder.storage()).1, changed);
    }

    /// A version-1 file holding an old key and its new one, as only a hand edit makes,
    /// keeps the new one's value.
    #[test]
    fn of_an_old_key_and_its_new_one_the_new_one_wins() {
        let text = "version = 1\n\
            [central_panel]\nmargin = 24\n[viewer]\nmargin = 16\n\
            [keys]\nshow-browse = [\"Shift+L\"]\nshow-collections = [\"Shift+C\"]\n\
            [edit.sections]\nbrowse = false\ncollections = true\n";
        let (read, warnings) = read(text);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(read.viewer.margin, 16.0);
        assert_eq!(read.keys["show-collections"], ["Shift+C"]);
        assert!(!read.keys.contains_key("show-browse"));
        assert_eq!(
            read.edit.sections,
            BTreeMap::from([("collections".to_owned(), true)])
        );
    }

    /// A file with no version, read as version 1, is migrated: its old keys keep their
    /// values through a write of another setting and a reading again.
    #[test]
    fn a_file_with_no_version_is_migrated_as_version_1() {
        let text = "[side_panel]\nwidth = 480\n\
            [keys]\ngrid-position-right = [\"Ctrl+Alt+Shift+F8\"]\n";
        let (read, warnings) = read(text);
        assert_eq!(warnings, ["version"]);
        assert_eq!(read.panel_width, Some(480.0));
        let wanted = Shortcut::parse("Ctrl+Alt+Shift+F8").expect("a key");
        let (keymap, refused) = Keymap::load(OperatingSystem::Mac, &read.keys);
        assert!(refused.is_empty(), "{refused:?}");
        assert_eq!(keymap.keys(ActionId::BROWSER_POSITION[1]), [wanted]);

        let folder = Folder::new("no-version");
        std::fs::write(folder.settings(), text).expect("written");
        let (_, settings) = Keeper::open(folder.storage());
        let changed = Settings {
            scale: 1.5,
            ..settings
        };
        change(&folder, &changed);
        assert!(folder.text().contains("version = 2"), "{}", folder.text());
        let (_, again) = Keeper::open(folder.storage());
        assert_eq!(again, changed);
        assert_eq!(again.panel_width, Some(480.0));
        assert_eq!(again.keys["browser-position-right"], ["Ctrl+Alt+Shift+F8"]);
    }

    /// Browse's groups and Collections' both set, as only a hand edit makes, join: each of
    /// Browse's kept where Collections' lacks it.
    #[test]
    fn browse_s_groups_join_collections_s() {
        let text = "version = 1\n\
            [edit.groups.browse]\nfolders = false\n\
            [edit.groups.collections]\nalbums = true\n";
        let (read, warnings) = read(text);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(
            read.edit.groups,
            BTreeMap::from([(
                "collections".to_owned(),
                BTreeMap::from([("albums".to_owned(), true), ("folders".to_owned(), false)])
            )])
        );
    }

    /// `--conf` takes the new keys alone: an old one names no setting.
    #[test]
    fn conf_takes_the_new_keys_alone() {
        for old in [
            "side_panel.width=300",
            "central_panel.margin=16",
            "grid_panel.rows=2",
        ] {
            assert!(conf(&[old]).is_err(), "{old}");
        }
        assert!(
            conf(&[
                "tools_panel.width=300",
                "viewer.margin=16",
                "browser.rows=2"
            ])
            .is_ok()
        );
    }

    #[test]
    fn settings_round_trip_through_a_file() {
        let folder = Folder::new("round-trip");
        change(&folder, &everything());
        let (_, settings) = Keeper::open(folder.storage());
        assert_eq!(settings, everything());
        assert_eq!(
            folder.files(),
            ["settings.toml"],
            "no temporary file is left"
        );
        // Back to the defaults: what may be missing goes, and so do the tables emptied.
        change(&folder, &Settings::default());
        let text = folder.text();
        // The windows' positions go; the Browser's, a choice, stays at its default.
        for gone in ["locale", "position = [", "width = 480", "[edit", "[keys"] {
            assert!(!text.contains(gone), "{gone}: {text}");
        }
        assert_eq!(Keeper::open(folder.storage()).1, Settings::default());
    }

    #[test]
    fn a_missing_file_gives_the_defaults_and_nothing_is_written_unchanged() {
        let folder = Folder::new("missing");
        let (keeper, settings) = Keeper::open(folder.storage());
        assert_eq!(settings, Settings::default());
        assert!(keeper.store.warnings().is_empty());
        drop(keeper);
        change(&folder, &Settings::default());
        assert!(folder.files().is_empty(), "{:?}", folder.files());
    }

    #[test]
    fn opening_and_closing_leaves_the_file_as_it_was() {
        let folder = Folder::new("unchanged");
        // As a person wrote it: comments, an order, an integer scale, a key not in a list,
        // channels in an order of their own.
        let text = "# mine\nversion = 1\nscale = 2\ntheme = \"light\"\n\
                    [scopes]\nchannels = [\"luminance\", \"red\", \"red\"]\n\
                    [keys]\nredo = \"Ctrl+R\"\n";
        std::fs::write(folder.settings(), text).expect("written");
        let (mut keeper, settings) = Keeper::open(folder.storage());
        // The first frames show a window that has not settled, then the settings as read.
        let start = Instant::now();
        let unsettled = Settings {
            window: Window {
                size: Some([800.0, 600.0]),
                ..Window::default()
            },
            ..settings.clone()
        };
        keeper.frame(start, &unsettled);
        keeper.frame(start + Duration::from_millis(16), &settings);
        keeper.frame(start + DELAY * 2, &settings);
        keeper.flush();
        assert_eq!(folder.text(), text);
    }

    #[test]
    fn a_value_that_cannot_be_read_is_left_at_its_default_with_a_warning() {
        let (s, names) = read(
            "version = 1\ntheme = \"solarized\"\nlocale = 3\nscale = 9.0\n\
             [window]\nsize = [1200]\nposition = [10, 20]\nmaximized = true\n\
             [tools_panel]\nwidth = 2000\n\
             [edit.sections]\nexposure = \"open\"\ncurve = true\n\
             [edit.groups]\nsample = true\n[edit.groups.exposure]\nmapping = 1\nshape = true\n\
             [keys]\nundo = 4\nredo = \"Ctrl+R\"\n",
        );
        assert_eq!(s.theme, Builtin::Dark);
        assert_eq!(s.locale, None);
        assert_eq!(s.scale, 3.0);
        assert_eq!(s.window.size, None);
        assert_eq!(s.window.position, None);
        assert!(s.window.maximized);
        // Out of range, the nearest width it can be dragged to.
        assert_eq!(s.panel_width, Some(*PANEL_WIDTHS.end()));
        assert_eq!(
            s.edit.sections,
            BTreeMap::from([("curve".to_owned(), true)])
        );
        assert_eq!(
            s.edit.groups,
            BTreeMap::from([(
                "exposure".to_owned(),
                BTreeMap::from([("shape".to_owned(), true)])
            )])
        );
        assert_eq!(
            s.keys,
            BTreeMap::from([("redo".to_owned(), vec!["Ctrl+R".to_owned()])])
        );
        // The store's, in the schema's order, then the app's own.
        assert_eq!(
            names,
            [
                "theme",
                "locale",
                "scale",
                "window.size",
                "tools_panel.width",
                "edit.sections.exposure",
                "edit.groups.exposure.mapping",
                "edit.groups.sample",
                "keys.undo",
                "window.position",
            ]
        );
        // A size of no area, or a scale factor of none, cannot be either.
        let (s, names) = read(
            "version = 1\n[window]\nsize = [1200, 0]\nposition = [10, 20]\nscale_factor = 0\n",
        );
        assert_eq!((s.window.size, s.window.position), (None, None));
        assert_eq!(
            names,
            ["window.size", "window.scale_factor", "window.position"]
        );
    }

    #[test]
    fn the_scopes_view_is_kept_by_its_names_and_what_cannot_be_read_is_left_at_its_default() {
        let (read_, names) = read(
            "version = 1\n[scopes]\ngraph = \"waveform\"\nscale = \"logarithmic\"\n\
             channels = [\"luminance\", \"red\"]\nhighlight_clipping = true\n",
        );
        let shown = ScopeView {
            scope: Scope::Waveform,
            scale: Scale::Logarithmic,
            channels: [true, false, false, true],
            shadow_clipping: false,
            highlight_clipping: true,
        };
        assert_eq!(read_.scopes, shown);
        assert!(names.is_empty(), "{names:?}");
        // Every channel hidden is a view too.
        let (hidden, _) = read("version = 1\n[scopes]\nchannels = []\n");
        assert_eq!(hidden.scopes.channels, [false; 4]);

        let (read_, names) = read(
            "version = 1\n[scopes]\ngraph = \"bars\"\nscale = 2\n\
             channels = [\"red\", \"ultraviolet\"]\nshadow_clipping = \"on\"\n",
        );
        assert_eq!(read_.scopes, ScopeView::default());
        assert_eq!(
            names,
            [
                "scopes.graph",
                "scopes.scale",
                "scopes.channels",
                "scopes.shadow_clipping"
            ]
        );
        // Only what changed is written, by name.
        let folder = Folder::new("scopes-written");
        let changed = Settings {
            scopes: ScopeView {
                scope: Scope::Parade,
                ..ScopeView::default()
            },
            ..Settings::default()
        };
        change(&folder, &changed);
        let text = folder.text();
        assert!(text.ends_with("[scopes]\ngraph = \"parade\"\n"), "{text}");
    }

    #[test]
    fn a_value_that_cannot_be_read_stays_until_the_user_changes_it() {
        let folder = Folder::new("unreadable");
        std::fs::write(
            folder.settings(),
            "version = 1\ntheme = \"solarized\"\nlocale = \"fr\"\n\
             [window]\nposition = [10, 20]\n\
             [tools_panel]\nwidth = \"wide\"\n\
             [edit]\ntab = 7\n[edit.sections]\nexposure = \"open\"\n[keys]\nundo = 4\n",
        )
        .expect("written");
        let (_, settings) = Keeper::open(folder.storage());
        assert_eq!(settings.panel_width, None);
        change(
            &folder,
            &Settings {
                scale: 1.5,
                ..settings.clone()
            },
        );
        let table: Table = folder.text().parse().expect("TOML");
        assert_eq!(table["scale"].as_float(), Some(1.5));
        assert_eq!(table["theme"].as_str(), Some("solarized"));
        assert_eq!(table["locale"].as_str(), Some("fr"));
        assert_eq!(
            table["window"]["position"].as_array().map(Vec::len),
            Some(2)
        );
        assert_eq!(table["tools_panel"]["width"].as_str(), Some("wide"));
        assert_eq!(table["edit"]["tab"].as_integer(), Some(7));
        assert_eq!(table["edit"]["sections"]["exposure"].as_str(), Some("open"));
        assert_eq!(table["keys"]["undo"].as_integer(), Some(4));
        // Until the user changes it.
        change(
            &folder,
            &Settings {
                theme: Builtin::Light,
                window: Window {
                    position: Some(Position {
                        pixels: [30.0, 40.0],
                        scale_factor: 1.0,
                    }),
                    ..settings.window
                },
                ..settings
            },
        );
        let table: Table = folder.text().parse().expect("TOML");
        assert_eq!(table["theme"].as_str(), Some("light"));
        let position = Value::Array(vec![30.into(), 40.into()]);
        assert_eq!(table["window"]["position"], position);
        assert_eq!(table["window"]["scale_factor"].as_float(), Some(1.0));
    }

    /// A language's tag in any case, as BCP 47 has them, and as the app read it before the
    /// store; a theme's id exactly.
    #[test]
    fn a_language_is_read_whatever_its_case() {
        let (settings, names) = read("version = 1\nlocale = \"UK\"\ntheme = \"Light\"\n");
        assert_eq!(
            (settings.locale, settings.theme),
            (Some(Locale::Uk), Builtin::Dark)
        );
        assert_eq!(names, ["theme"]);
    }

    /// The language is kept once the user chooses one, even the system's, and the Tools
    /// panel's width once dragged, even to the theme's: computed defaults are no values.
    #[test]
    fn a_choice_equal_to_a_computed_default_is_kept() {
        let folder = Folder::new("computed");
        change(
            &folder,
            &Settings {
                locale: Some(crate::system_locale()),
                panel_width: Some(TOOLS_PANEL_WIDTH.default()),
                ..Settings::default()
            },
        );
        let (_, settings) = Keeper::open(folder.storage());
        assert_eq!(settings.locale, Some(crate::system_locale()));
        assert_eq!(settings.panel_width, Some(320.0));
    }

    /// The engine's settings share the app's file (decision 43): read by the app's store
    /// with no warning, handed on as the engine's options, and kept as they are when the
    /// app writes its own.
    #[test]
    fn the_engine_s_settings_are_read_from_the_app_s_file_and_kept() {
        use apertrust_engine::gpu::{AdapterChoice, Backend};
        let folder = Folder::new("engine");
        let text = "version = 1\nscale = 2\n[engine]\nadapter = \"Radeon\"\nbackend = \"vulkan\"\n\
                    memory_budget = 4096\n";
        std::fs::write(folder.settings(), text).expect("written");
        let (mut keeper, settings) = Keeper::open(folder.storage());
        assert!(
            keeper.store.warnings().is_empty(),
            "{:?}",
            keeper.store.warnings()
        );
        let options = keeper.engine();
        let radeon = AdapterChoice::Preferred("Radeon".to_owned());
        assert_eq!((options.gpu.adapter, options.budget), (radeon, 4096 << 20));
        // Vulkan where this build has it and it finds an adapter, as on most Windows and
        // Linux machines; else Automatic.
        if std::env::var_os("WGPU_BACKEND").is_none() {
            let vulkan = if Backend::Vulkan.built()
                && apertrust_engine::gpu::finds_adapter(Backend::Vulkan)
            {
                Backend::Vulkan
            } else {
                Backend::Auto
            };
            assert_eq!(options.gpu.backend, vulkan);
        }
        keeper.frame(
            Instant::now(),
            &Settings {
                scale: 1.5,
                ..settings
            },
        );
        keeper.flush();
        let table: Table = folder.text().parse().expect("TOML");
        assert_eq!(table["scale"].as_float(), Some(1.5));
        assert_eq!(table["engine"]["adapter"].as_str(), Some("Radeon"));
        assert_eq!(table["engine"]["backend"].as_str(), Some("vulkan"));
        assert_eq!(table["engine"]["memory_budget"].as_integer(), Some(4096));
        // Nothing kept: the defaults.
        let options = Keeper::nowhere().0.engine();
        let defaults = (AdapterChoice::Any, Backend::Auto, 2048 << 20);
        let read = (options.gpu.adapter, options.gpu.backend, options.budget);
        assert_eq!(read, defaults);
    }

    fn conf(given: &[&str]) -> Result<Overrides, OverrideError> {
        let given: Vec<String> = given.iter().map(|g| (*g).to_owned()).collect();
        overrides(&given)
    }

    /// The Viewer's margins are 10 points and 0.2 of the slot's smaller side, as
    /// darktable's darkroom has them (Proof margin's 40% less); the file or `--conf` sets
    /// either, a number outside its range read as its nearer end, and neither is on a page
    /// of the Settings dialog.
    #[test]
    fn the_viewer_s_margins_have_their_defaults_and_follow_the_file_and_conf() {
        let (_, settings) = Keeper::nowhere();
        assert_eq!(settings.viewer, Viewer::default());
        assert_eq!(
            (settings.viewer.margin, settings.viewer.proof_margin),
            (10.0, 0.12)
        );

        let folder = Folder::new("margins-file");
        let text = "version = 1\n[viewer]\nmargin = 24\nproof_margin = 0.3\n";
        std::fs::write(folder.settings(), text).expect("written");
        let (_, settings) = Keeper::open(folder.storage());
        assert_eq!(
            settings.viewer,
            Viewer {
                margin: 24.0,
                proof_margin: 0.3,
                background: None
            }
        );

        // `--conf` over the file, and a value out of range at its nearer end.
        let overrides = conf(&["viewer.margin=16", "viewer.proof_margin=0.9"]).expect("read");
        let (keeper, settings) = Keeper::open_with(folder.storage(), &overrides);
        assert_eq!(
            settings.viewer,
            Viewer {
                margin: 16.0,
                proof_margin: 0.4,
                background: None
            }
        );
        assert!(keeper.store.overridden(&VIEWER_MARGIN));
        assert!(!keeper.store.overridden(&TOOLS_PANEL_TAB_NAMES));
        assert_eq!(folder.text(), text, "never written");
        // Without a file's value and without `--conf`, the default.
        let none = Folder::new("margins-conf");
        let overrides = conf(&["viewer.margin=0"]).expect("read");
        let (_, settings) = Keeper::open_with(none.storage(), &overrides);
        assert_eq!(settings.viewer.margin, 0.0);
        assert_eq!(settings.viewer.proof_margin, 0.12);
        assert!(conf(&["viewer.margin=wide"]).is_err());
    }

    /// Mouse wheel scrolls the Tools panel is on at first, kept off once turned off and read
    /// so at the next start; `--conf` gives it for the run, its row's change refused and the
    /// file keeping its own value.
    #[test]
    fn the_wheel_s_setting_is_kept_and_conf_gives_it_for_the_run() {
        let folder = Folder::new("wheel-scrolls-panel");
        let (_, settings) = Keeper::open(folder.storage());
        assert!(settings.wheel_scrolls_panel);
        let off = Settings {
            wheel_scrolls_panel: false,
            ..settings
        };
        change(&folder, &off);
        assert!(folder.text().contains("wheel_scrolls_panel = false"));
        let (_, settings) = Keeper::open(folder.storage());
        assert!(!settings.wheel_scrolls_panel);

        let overrides = conf(&["tools_panel.wheel_scrolls_panel=true"]).expect("read");
        let (mut keeper, settings) = Keeper::open_with(folder.storage(), &overrides);
        assert!(settings.wheel_scrolls_panel);
        assert!(keeper.store.overridden(&TOOLS_PANEL_WHEEL_SCROLLS_PANEL));
        let id = TOOLS_PANEL_WHEEL_SCROLLS_PANEL.id();
        let updated = keeper.apply(&settings, id, Some(&Value::Boolean(false)));
        assert!(updated.wheel_scrolls_panel);
        keeper.flush();
        assert!(folder.text().contains("wheel_scrolls_panel = false"));
    }

    /// A tool setting `--conf` gives keeps its value for the run: a change of its row and a
    /// value a pick measured are refused, so the picker's commands carry `--conf`'s, and the
    /// file keeps its own; the others change as ever.
    #[test]
    fn a_tool_setting_conf_gives_keeps_its_value_for_the_run() {
        let folder = Folder::new("tool-settings-conf");
        let text = "version = 1\n[tools.exposure]\nlightness = 30.0\n";
        std::fs::write(folder.settings(), text).expect("written");
        let overrides = conf(&["tools.exposure.target=lightness"]).expect("read");
        let (mut keeper, mut settings) = Keeper::open_with(folder.storage(), &overrides);
        let kept = &mut settings.tool_settings;
        assert!(kept.overridden("exposure", "target"));
        assert!(!kept.overridden("exposure", "lightness"));
        // Middle grey asked for from the row, and a lightness set.
        kept.set("exposure", "target", ControlValue::Choice(0));
        kept.set("exposure", "lightness", ControlValue::Number(42.0));
        assert_eq!(
            kept.of("exposure"),
            [
                ("lightness", ControlValue::Number(42.0)),
                ("target", ControlValue::Choice(1)),
            ]
        );
        keeper.frame(Instant::now(), &settings);
        keeper.flush();
        let written = folder.text();
        assert!(written.contains("lightness = 42.0"), "{written}");
        assert!(!written.contains("target"), "{written}");
    }

    #[test]
    fn unrelated_dialog_changes_preserve_runtime_geometry_over_conf() {
        let folder = Folder::new("runtime-geometry-conf");
        let overrides = conf(&["scale=1.5", "tools_panel.width=300"]).unwrap();
        let (mut keeper, mut settings) = Keeper::open_with(folder.storage(), &overrides);
        settings.scale = 1.7;
        settings.panel_width = Some(350.0);
        let updated = keeper.apply(
            &settings,
            TOOLS_PANEL_TAB_NAMES.id(),
            Some(&toml::Value::Boolean(false)),
        );
        assert_eq!(updated.scale, 1.7);
        assert_eq!(updated.panel_width, Some(350.0));
        assert_eq!(keeper.store.get(&SCALE), 1.5);
        assert_eq!(keeper.store.get(&TOOLS_PANEL_WIDTH), 300.0);
        keeper.flush();
        assert!(!folder.text().contains("1.7"));
        assert!(!folder.text().contains("350"));
    }

    /// Picker target and Lightness survive a restart; an old `area_mode` key, which went
    /// with darktable's measure mode (decision 88), is an unknown key: read as nothing,
    /// Picker target at its default, and left in the file as it was.
    #[test]
    fn picker_target_is_kept_and_an_old_area_mode_key_left_alone() {
        let folder = Folder::new("picker-target");
        let text = "version = 1\n[tools.exposure]\narea_mode = \"measure\"\nlightness = 30.0\n";
        std::fs::write(folder.settings(), text).expect("written");
        let (mut keeper, mut settings) = Keeper::open(folder.storage());
        let kept = &settings.tool_settings;
        assert_eq!(
            kept.of("exposure"),
            [
                ("lightness", ControlValue::Number(30.0)),
                ("target", ControlValue::Choice(0)),
            ]
        );
        settings
            .tool_settings
            .set("exposure", "target", ControlValue::Choice(1));
        keeper.frame(Instant::now(), &settings);
        keeper.flush();
        let written = folder.text();
        assert!(written.contains("area_mode = \"measure\""), "{written}");
        assert!(written.contains("target = \"lightness\""), "{written}");
        drop(keeper);
        let (_, again) = Keeper::open(folder.storage());
        assert_eq!(again.tool_settings, settings.tool_settings);
    }

    /// A reference's L `--conf` gives comes with no photo, though the file names one: the
    /// file keeps it, untouched (decision 88).
    #[test]
    fn a_reference_conf_gives_carries_no_photo_from_the_file() {
        let folder = Folder::new("reference-conf-photo");
        let text = "version = 1\n[tools.exposure]\nreference_lightness = 62.0\n\
                    reference_photo = \"/photos/IMG_2041.CR3\"\n";
        std::fs::write(folder.settings(), text).expect("written");
        let overrides = conf(&["tools.exposure.reference_lightness=60"]).expect("read");
        let (mut keeper, settings) = Keeper::open_with(folder.storage(), &overrides);
        let kept = &settings.tool_settings;
        let given = Some(ControlValue::Number(60.0));
        assert_eq!(kept.get("exposure", "reference_lightness").cloned(), given);
        assert_eq!(kept.text("exposure", "reference_photo"), None);
        keeper.frame(Instant::now(), &settings);
        keeper.flush();
        assert_eq!(folder.text(), text);
        // With no `--conf`, the file's reference whole.
        let (_, settings) = Keeper::open(folder.storage());
        let photo = settings.tool_settings.text("exposure", "reference_photo");
        assert_eq!(photo, Some("/photos/IMG_2041.CR3"));
    }

    /// A reference is set and cleared whole (decision 88): measured with no photo known,
    /// its L comes with no path, never beside the last photo's; and where `--conf` gives
    /// either of its two settings, neither changes.
    #[test]
    fn a_reference_is_set_and_cleared_whole() {
        let keys = ("reference_lightness", "reference_photo");
        let mut kept = ToolSettings::default();
        let photo = "/photos/IMG_2041.CR3".to_owned();
        kept.set_reference("exposure", keys, ControlValue::Number(62.0), Some(photo));
        kept.set_reference("exposure", keys, ControlValue::Number(40.0), None);
        let lightness = kept.get("exposure", keys.0).cloned();
        assert_eq!(lightness, Some(ControlValue::Number(40.0)));
        assert_eq!(kept.text("exposure", keys.1), None);
        kept.clear_reference("exposure", keys);
        assert_eq!(kept, ToolSettings::default());

        let folder = Folder::new("reference-conf");
        let overrides = conf(&["tools.exposure.reference_lightness=60"]).expect("read");
        let (_, settings) = Keeper::open_with(folder.storage(), &overrides);
        let mut kept = settings.tool_settings;
        let given = Some(ControlValue::Number(60.0));
        assert_eq!(kept.get("exposure", keys.0).cloned(), given);
        let photo = "/photos/IMG_2041.CR3".to_owned();
        kept.set_reference("exposure", keys, ControlValue::Number(62.0), Some(photo));
        kept.clear_reference("exposure", keys);
        assert_eq!(kept.get("exposure", keys.0).cloned(), given);
        assert_eq!(kept.text("exposure", keys.1), None);
    }

    /// A reference (decision 88): none until one is set, its L sent with the picker's
    /// commands and its photo's path never; both survive a restart, and clearing removes
    /// both keys from the file.
    #[test]
    fn a_reference_is_kept_across_a_restart_and_cleared_whole() {
        let folder = Folder::new("reference");
        let (mut keeper, mut settings) = Keeper::open(folder.storage());
        let kept = &mut settings.tool_settings;
        assert_eq!(kept.get("exposure", "reference_lightness"), None);
        assert_eq!(kept.text("exposure", "reference_photo"), None);
        kept.set(
            "exposure",
            "reference_lightness",
            ControlValue::Number(62.5),
        );
        let photo = "/photos/IMG_2041.CR3";
        kept.set_text("exposure", "reference_photo", photo.to_owned());
        // A text is no setting a pick reads.
        assert!(
            kept.of("exposure")
                .iter()
                .all(|(s, _)| *s != "reference_photo")
        );
        assert!(
            kept.of("exposure")
                .contains(&("reference_lightness", ControlValue::Number(62.5)))
        );
        keeper.frame(Instant::now(), &settings);
        keeper.flush();
        drop(keeper);
        let (mut keeper, mut again) = Keeper::open(folder.storage());
        assert_eq!(again.tool_settings, settings.tool_settings);
        assert_eq!(again.tool_settings.texts().collect::<Vec<_>>(), [photo]);
        again.tool_settings.clear("exposure", "reference_lightness");
        again.tool_settings.clear("exposure", "reference_photo");
        keeper.frame(Instant::now(), &again);
        keeper.flush();
        let written = folder.text();
        assert!(!written.contains("reference"), "{written}");
        drop(keeper);
        let (_, cleared) = Keeper::open(folder.storage());
        assert_eq!(cleared.tool_settings, ToolSettings::default());
    }

    #[test]
    fn the_background_follows_the_theme_until_one_is_chosen() {
        use apertrust_ui::Background;
        assert_eq!(VIEWER_BACKGROUND.default(), Background::Dark);
        let (_, settings) = Keeper::nowhere();
        assert_eq!(settings.viewer.background, None);

        let folder = Folder::new("background-file");
        let text = "version = 1\n[viewer]\nbackground = \"very-dark\"\n";
        std::fs::write(folder.settings(), text).expect("written");
        let (_, settings) = Keeper::open(folder.storage());
        assert_eq!(settings.viewer.background, Some(Background::VeryDark));

        let overrides = conf(&["viewer.background=white"]).expect("read");
        let (_, settings) = Keeper::open_with(folder.storage(), &overrides);
        assert_eq!(settings.viewer.background, Some(Background::White));
        assert!(conf(&["viewer.background=grey"]).is_err());
    }

    /// `--conf` sets the app's settings and the engine's for the run, over the file: what
    /// the app shows and the engine takes, reported as overridden, and never written, so a
    /// change the user makes to one leaves the file's value as it is, byte for byte, while
    /// the others are written (decision 43).
    #[test]
    fn a_conf_wins_over_the_file_and_is_never_written() {
        use apertrust_engine::settings::Source;
        let folder = Folder::new("conf");
        let text = "version = 1\nscale = 2\n[engine]\nmemory_budget = 4096\n";
        std::fs::write(folder.settings(), text).expect("written");
        let given = ["scale=1.5", "theme=light", "engine.memory_budget=1024"];
        let overrides = conf(&given).expect("read");
        let (mut keeper, settings) = Keeper::open_with(folder.storage(), &overrides);
        assert_eq!((settings.scale, settings.theme), (1.5, Builtin::Light));
        assert!(keeper.store.overridden(&SCALE) && keeper.store.overridden(&THEME));
        assert!(!keeper.store.overridden(&TOOLS_PANEL_TAB_NAMES));
        let options = keeper.engine();
        assert_eq!(options.budget, 1024 << 20);
        assert_eq!(options.sources[1], Source::Conf);
        // The user zooms and changes the theme back, and hides the tabs' names.
        let changed = Settings {
            scale: 1.7,
            theme: Builtin::Dark,
            tab_names: false,
            ..settings
        };
        keeper.frame(Instant::now(), &changed);
        keeper.flush();
        let expected = "\
# Apertrust's settings. Apertrust writes here only what changes, without comments;
# keys it does not know stay.

version = 2
scale = 2

[engine]
memory_budget = 4096

[tools_panel]
tab_names = false
";
        assert_eq!(folder.text(), expected);
        // Read without them, the file's own.
        let (keeper, settings) = Keeper::open(folder.storage());
        assert_eq!((settings.scale, settings.theme), (2.0, Builtin::Dark));
        assert_eq!(keeper.engine().budget, 4096 << 20);

        // A file without the setting never gains it.
        let folder = Folder::new("conf-none");
        let (mut keeper, settings) = Keeper::open_with(folder.storage(), &overrides);
        let resized = Settings {
            scale: 1.6,
            window: Window {
                size: Some([1280.0, 800.0]),
                ..settings.window
            },
            ..settings
        };
        keeper.frame(Instant::now(), &resized);
        keeper.flush();
        let table: Table = folder.text().parse().expect("TOML");
        assert!(!table.contains_key("scale") && !table.contains_key("theme"));
        assert!(!table.contains_key("engine"));
        assert_eq!(table["window"]["size"][0].as_integer(), Some(1280));
    }

    /// Don't ask again turns `export.ask_before_overwrite` off in the file, where the next
    /// run reads it; with `--conf` giving the setting, the run keeps asking and the file is
    /// not written (`alerts::Question`).
    #[test]
    fn a_yes_that_stops_the_overwrite_question_is_kept_unless_conf_gives_the_setting() {
        use crate::alerts::{Purpose, Question};
        use crate::dialog_host::Answer;
        let table = |folder: &Folder| -> Table { folder.text().parse().expect("TOML") };

        let folder = Folder::new("ask-overwrite");
        let (mut keeper, settings) = Keeper::open(folder.storage());
        assert!(settings.ask_before_overwrite);
        let mut overwrite = Question::overwrite(settings.ask_before_overwrite, false);
        overwrite.answered(Purpose::Overwrite, Answer::Ticked(0));
        let stopped = Settings {
            ask_before_overwrite: overwrite.asks(),
            ..settings
        };
        keeper.frame(Instant::now(), &stopped);
        keeper.flush();
        assert_eq!(
            table(&folder)["export"]["ask_before_overwrite"].as_bool(),
            Some(false)
        );
        // The next run reads it.
        let (_, next) = Keeper::open(folder.storage());
        assert!(!next.ask_before_overwrite);

        // Given by `--conf`, true over a file that says false: the run asks, and the file
        // keeps its own.
        let text = "version = 1\n[export]\nask_before_overwrite = false\n";
        let folder = Folder::new("ask-overwrite-conf");
        std::fs::write(folder.settings(), text).expect("written");
        let overrides = conf(&["export.ask_before_overwrite=true"]).expect("read");
        let (mut keeper, settings) = Keeper::open_with(folder.storage(), &overrides);
        assert!(settings.ask_before_overwrite);
        assert!(keeper.overridden(&EXPORT_ASK_BEFORE_OVERWRITE));
        let mut overwrite = Question::overwrite(settings.ask_before_overwrite, true);
        overwrite.answered(Purpose::Overwrite, Answer::Ticked(0));
        assert!(overwrite.asks());
        let same = Settings {
            ask_before_overwrite: overwrite.asks(),
            tab_names: false,
            ..settings
        };
        keeper.frame(Instant::now(), &same);
        keeper.flush();
        assert_eq!(
            table(&folder)["export"]["ask_before_overwrite"].as_bool(),
            Some(false)
        );
    }

    /// The window's position and its scale factor are kept together, so while `--conf`
    /// gives either, the window's moves write neither: the file never holds one without
    /// the other.
    #[test]
    fn a_conf_of_half_the_window_s_position_keeps_the_pair_as_the_file_has_it() {
        let moved = |settings: &Settings| Settings {
            window: Window {
                position: Some(Position {
                    pixels: [50.0, 60.0],
                    scale_factor: 2.0,
                }),
                ..settings.window
            },
            // Something else changes, so the file is written.
            tab_names: false,
            ..settings.clone()
        };
        // The pair in the file, the position given.
        let folder = Folder::new("conf-position");
        let text = "version = 1\n[window]\nposition = [10, 20]\nscale_factor = 1.0\n";
        std::fs::write(folder.settings(), text).expect("written");
        let overrides = conf(&["window.position=[30, 40]"]).expect("read");
        let (mut keeper, settings) = Keeper::open_with(folder.storage(), &overrides);
        let given = Position {
            pixels: [30.0, 40.0],
            scale_factor: 1.0,
        };
        assert_eq!(settings.window.position, Some(given));
        keeper.frame(Instant::now(), &moved(&settings));
        keeper.flush();
        let table: Table = folder.text().parse().expect("TOML");
        let pair = (
            &table["window"]["position"],
            &table["window"]["scale_factor"],
        );
        let file = (
            &Value::Array(vec![10.into(), 20.into()]),
            &Value::Float(1.0),
        );
        assert_eq!(pair, file);
        assert_eq!(table["tools_panel"]["tab_names"].as_bool(), Some(false));

        // An empty file, the scale factor given.
        let folder = Folder::new("conf-scale-factor");
        let overrides = conf(&["window.scale_factor=2"]).expect("read");
        let (mut keeper, settings) = Keeper::open_with(folder.storage(), &overrides);
        assert_eq!(settings.window.position, None);
        keeper.frame(Instant::now(), &moved(&settings));
        keeper.flush();
        let table: Table = folder.text().parse().expect("TOML");
        assert!(!table.contains_key("window"), "{table}");

        // A position given without a scale factor is warned of as `--conf`'s.
        let overrides = conf(&["window.position=[30, 40]"]).expect("read");
        let (keeper, settings) = Keeper::open_in(None, &overrides);
        assert_eq!(settings.window.position, None);
        let warned = warnings(&keeper.store);
        let why = "window.position has no scale factor; not used".to_owned();
        assert_eq!(warned, [("--conf", why)]);
    }

    /// Every setting the app reads, its own and the engine's, by its kind, as the file is
    /// read; a key that is none of them, or a value that cannot be read, is refused,
    /// naming it.
    #[test]
    fn a_conf_is_checked_against_the_app_s_settings_and_the_engine_s() {
        let overrides = conf(&["locale=UK", "engine.adapter=M4", "tools_panel.width=2000"]);
        let (keeper, settings) = Keeper::open_in(None, &overrides.expect("read"));
        // A language's tag in any case, as the file's; out of range, the nearer end.
        assert_eq!(settings.locale, Some(Locale::Uk));
        assert_eq!(settings.panel_width, Some(*crate::PANEL_WIDTHS.end()));
        let named = apertrust_engine::gpu::AdapterChoice::Named("M4".to_owned());
        assert_eq!(keeper.engine().gpu.adapter, named);
        // A backend typed for the run is kept, even one this build lacks, which opening
        // the device then refuses; Metal is no option of the setting's, Automatic on macOS.
        let overrides = conf(&["engine.backend=dx12"]).expect("read");
        let (keeper, _) = Keeper::open_in(None, &overrides);
        let options = keeper.engine();
        assert_eq!(options.gpu.backend, apertrust_engine::gpu::Backend::Dx12);
        if std::env::var_os("WGPU_BACKEND").is_none() {
            assert_eq!(options.sources[2], apertrust_engine::settings::Source::Conf);
        }
        let refused = conf(&["engine.backend=metal"]).expect_err("refused");
        assert_eq!(
            refused.to_string(),
            "--conf engine.backend: cannot be \"metal\""
        );
        assert_eq!(
            conf(&["density=compact"]).map(drop),
            Err(OverrideError::Unknown("density".to_owned()))
        );
        let refused = conf(&["scale=1.2", "theme=solarized"]).expect_err("refused");
        assert_eq!(refused.to_string(), "--conf theme: cannot be \"solarized\"");
    }
}
