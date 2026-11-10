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
//! version = 1
//! theme = "light"
//! locale = "uk"
//! scale = 1.2
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
//! [side_panel]
//! tab_names = false
//! width = 480
//!
//! [central_panel]
//! margin = 16
//! proof_margin = 0.25
//!
//! [edit]
//! tab = "correct"
//!
//! [edit.sections]
//! exposure = false
//!
//! [edit.groups.exposure]
//! area_exposure_mapping = true
//!
//! [scopes]
//! graph = "waveform"
//! channels = ["red", "green", "blue", "luminance"]
//!
//! [grid_panel]
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
//! system's. The side panel's width is in points, as the user dragged it, within the
//! widths it can be dragged to; until then it is the theme's. A module's collapsible
//! groups are kept by the module's id and the group's, only those the user opened or
//! closed, the others starting closed (decision 29). The scopes' view is kept by part:
//! the graph, the scale (`linear` or `logarithmic`), the channels shown and whether each
//! clipping overlay is on, each once the user changes it. So is the grid panel's: the
//! sort's key and direction, the thumbnails' size, where it is, the rows it shows at the
//! bottom and the columns at a side, each a whole number, whether the thumbnails show
//! their captions and whether each fills its square (`docs/gui.md`, The grid panel and The
//! grid bar). Keys are kept only where they
//! differ from the defaults, in the form `Shortcut::text` writes (`actions`), the same on
//! every platform. The central panel's two margins have no page in the Settings dialog:
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

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use apertrust_base::folders::Folders;
use apertrust_base::settings::{
    CONF_FLAG, Declared, FileStorage, OverrideError, Overrides, Setting, Storage, Store, Stored,
};
use apertrust_engine::settings as engine;
use apertrust_ui::components::{
    Channel, GridBarAction, GridPosition, Marker, ScopeView, Sort, ThumbnailSize,
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
        pub fn side_panel_width() -> f32 {
            Theme::builtin(super::THEME.default()).sizes.side_panel
        }

        /// The default theme's own surround; the shell keeps the theme in use's while
        /// none is chosen.
        pub fn central_panel_background() -> Background {
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
    pub window: Window,
    /// Where Settings' own window was last left, on the Mac; None opens it centred over
    /// the main window.
    pub settings_window: Option<Position>,
    /// Where the jobs window was last left; None opens it centred over the main window.
    pub jobs_window: Option<Position>,
    /// The log kept in memory, and its window's place and folder.
    pub log: Log,
    /// Whether the side panel's tabs show their names.
    pub tab_names: bool,
    /// The side panel's width in points, within `PANEL_WIDTHS`, as the user dragged it;
    /// None for the theme's.
    pub panel_width: Option<f32>,
    /// The margins round the central panel's photo and caption.
    pub central_panel: CentralPanel,
    /// Edit's own state.
    pub edit: ViewState,
    /// What the Scopes section shows, and which clipping overlays the photo shows.
    pub scopes: ScopeView,
    /// How the grid panel shows the folder's photos.
    pub grid: Grid,
    /// The keys that differ from the defaults, by action id, in `Shortcut::text`'s form,
    /// with what the keymap could not use as it was read; an empty list leaves an action
    /// without a key.
    pub keys: BTreeMap<String, Vec<String>>,
    /// The Export dialog's last settings, `export.*`, kept as they are left.
    pub export: Values,
    /// Whether Export and Export Again ask before replacing files, `export.ask_before_overwrite`:
    /// kept apart from the dialog's settings, since the dialog does not edit it and its
    /// Default settings do not reset it.
    pub ask_before_overwrite: bool,
}

impl Default for Settings {
    /// The schema's defaults. What it computes, the language and the side panel's width,
    /// and what is kept only once the window or the user sets it, the window's size and
    /// place and Edit's tab, start unkept, as do the tables of sections, groups and keys.
    fn default() -> Self {
        let channels = SCOPES_CHANNELS.default();
        Settings {
            theme: THEME.default(),
            locale: None,
            scale: SCALE.default(),
            marker: SLIDERS_MARKER.default(),
            window: Window {
                size: None,
                position: None,
                maximized: WINDOW_MAXIMIZED.default(),
            },
            settings_window: None,
            jobs_window: None,
            log: Log::default(),
            tab_names: SIDE_PANEL_TAB_NAMES.default(),
            panel_width: None,
            central_panel: CentralPanel::default(),
            edit: ViewState::default(),
            scopes: ScopeView {
                scope: SCOPES_GRAPH.default(),
                scale: SCOPES_SCALE.default(),
                channels: Channel::ALL.map(|channel| channels.contains(&channel)),
                shadow_clipping: SCOPES_SHADOW_CLIPPING.default(),
                highlight_clipping: SCOPES_HIGHLIGHT_CLIPPING.default(),
            },
            grid: Grid::default(),
            keys: BTreeMap::new(),
            export: Values::default(),
            ask_before_overwrite: EXPORT_ASK_BEFORE_OVERWRITE.default(),
        }
    }
}

impl Settings {
    /// The settings as `store` holds them: what is kept only once set (the language, the
    /// window's size and place, the side panel's width, Edit's tab) as kept, the rest as
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
            tab_names: store.get(&SIDE_PANEL_TAB_NAMES),
            panel_width: store.kept(&SIDE_PANEL_WIDTH),
            central_panel: CentralPanel {
                margin: store.get(&CENTRAL_PANEL_MARGIN),
                proof_margin: store.get(&CENTRAL_PANEL_PROOF_MARGIN),
                background: store.kept(&CENTRAL_PANEL_BACKGROUND),
            },
            edit: ViewState {
                tab: store.kept(&EDIT_TAB),
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
            grid: Grid {
                sort: Sort {
                    key: store.get(&GRID_PANEL_SORT),
                    descending: store.get(&GRID_PANEL_DESCENDING),
                },
                thumbnails: store.get(&GRID_PANEL_THUMBNAILS),
                rows: store.get(&GRID_PANEL_ROWS) as usize,
                position: store.get(&GRID_PANEL_POSITION),
                columns: store.get(&GRID_PANEL_COLUMNS) as usize,
                labels: store.get(&GRID_PANEL_LABELS),
                square_thumbnails: store.get(&GRID_PANEL_SQUARE_THUMBNAILS),
            },
            keys: store.get(&KEYS),
            export: Values::read(store),
            ask_before_overwrite: store.get(&EXPORT_ASK_BEFORE_OVERWRITE),
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
            store.set(&SIDE_PANEL_TAB_NAMES, s.tab_names);
        }
        if s.panel_width != was.panel_width {
            keep(store, &SIDE_PANEL_WIDTH, s.panel_width);
        }
        if s.central_panel.margin != was.central_panel.margin {
            store.set(&CENTRAL_PANEL_MARGIN, s.central_panel.margin);
        }
        if s.central_panel.proof_margin != was.central_panel.proof_margin {
            store.set(&CENTRAL_PANEL_PROOF_MARGIN, s.central_panel.proof_margin);
        }
        if s.central_panel.background != was.central_panel.background {
            keep(store, &CENTRAL_PANEL_BACKGROUND, s.central_panel.background);
        }
        if s.edit.tab != was.edit.tab {
            keep(store, &EDIT_TAB, s.edit.tab.clone());
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
        let (grid, before) = (s.grid, was.grid);
        if grid.sort.key != before.sort.key {
            store.set(&GRID_PANEL_SORT, grid.sort.key);
        }
        if grid.sort.descending != before.sort.descending {
            store.set(&GRID_PANEL_DESCENDING, grid.sort.descending);
        }
        if grid.thumbnails != before.thumbnails {
            store.set(&GRID_PANEL_THUMBNAILS, grid.thumbnails);
        }
        if grid.rows != before.rows {
            store.set(&GRID_PANEL_ROWS, grid.rows as f32);
        }
        if grid.position != before.position {
            store.set(&GRID_PANEL_POSITION, grid.position);
        }
        if grid.columns != before.columns {
            store.set(&GRID_PANEL_COLUMNS, grid.columns as f32);
        }
        if grid.labels != before.labels {
            store.set(&GRID_PANEL_LABELS, grid.labels);
        }
        if grid.square_thumbnails != before.square_thumbnails {
            store.set(&GRID_PANEL_SQUARE_THUMBNAILS, grid.square_thumbnails);
        }
        if s.keys != was.keys {
            store.set(&KEYS, s.keys.clone());
        }
        if s.export != was.export {
            s.export.write(store);
        }
        if s.ask_before_overwrite != was.ask_before_overwrite {
            store.set(&EXPORT_ASK_BEFORE_OVERWRITE, s.ask_before_overwrite);
        }
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

/// The margins round the central panel's photo and its caption, and the background behind
/// them (`[central_panel]`, which no page of the Settings dialog shows; `docs/gui.md`, The
/// photo frame).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CentralPanel {
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

impl Default for CentralPanel {
    fn default() -> Self {
        CentralPanel {
            margin: CENTRAL_PANEL_MARGIN.default(),
            proof_margin: CENTRAL_PANEL_PROOF_MARGIN.default(),
            background: None,
        }
    }
}

/// How the grid panel shows the folder's photos (`[grid_panel]`, `docs/gui.md`, The grid
/// panel and The grid bar).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    /// The order the photos are shown in.
    pub sort: Sort,
    /// The thumbnails' size.
    pub thumbnails: ThumbnailSize,
    /// The rows it shows at the bottom, as its edge was dragged.
    pub rows: usize,
    /// Where it is in the window.
    pub position: GridPosition,
    /// The columns it shows at the left or right, as its edge was dragged.
    pub columns: usize,
    /// Whether the thumbnails show their captions: the grid bar's Labels.
    pub labels: bool,
    /// Whether each thumbnail fills its square, cut at its longer sides.
    pub square_thumbnails: bool,
}

impl Grid {
    /// Whether `chosen` is what it holds, which the menus show checked.
    pub fn holds(&self, chosen: GridBarAction) -> bool {
        match chosen {
            GridBarAction::Sort(key) => self.sort.key == key,
            GridBarAction::Descending(descending) => self.sort.descending == descending,
            GridBarAction::Size(size) => self.thumbnails == size,
            GridBarAction::Position(position) => self.position == position,
            GridBarAction::Labels => self.labels,
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
    pub fn apply(&mut self, chosen: GridBarAction) {
        match chosen {
            GridBarAction::Sort(key) => self.sort.key = key,
            GridBarAction::Descending(descending) => self.sort.descending = descending,
            GridBarAction::Size(size) => self.thumbnails = size,
            GridBarAction::Position(position) => self.position = position,
            GridBarAction::Labels => self.labels = !self.labels,
        }
    }
}

impl Default for Grid {
    fn default() -> Self {
        Grid {
            sort: Sort {
                key: GRID_PANEL_SORT.default(),
                descending: GRID_PANEL_DESCENDING.default(),
            },
            thumbnails: GRID_PANEL_THUMBNAILS.default(),
            rows: GRID_PANEL_ROWS.default() as usize,
            position: GRID_PANEL_POSITION.default(),
            columns: GRID_PANEL_COLUMNS.default() as usize,
            labels: GRID_PANEL_LABELS.default(),
            square_thumbnails: GRID_PANEL_SQUARE_THUMBNAILS.default(),
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

/// A view's own state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ViewState {
    /// The side panel's tab, by id.
    pub tab: Option<String>,
    /// The sections the user opened or closed, by id; the others start as their section
    /// does.
    pub sections: BTreeMap<String, bool>,
    /// The collapsible groups the user opened or closed, by module and group, the same
    /// for every photo; the others start closed.
    pub groups: BTreeMap<String, BTreeMap<String, bool>>,
}

/// The settings the app's store reads: its own, then the engine's, which the app hands
/// the engine as options (decision 43), then the export settings the Export dialog edits
/// (decision 56).
fn registries() -> Vec<&'static dyn Declared> {
    let engine = engine::registry::ALL.iter();
    let export = apertrust_export::settings::registry::ALL.iter();
    registry::ALL
        .iter()
        .chain(engine)
        .chain(export)
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
        Settings::read(&self.store)
    }

    /// The engine's settings as `--conf` and the file give them, as the plain options the
    /// engine takes, each logged under `-d settings` with where it came from: `--conf`,
    /// the file or the default, since the app has no flag over them.
    pub fn engine(&self) -> engine::Options {
        engine::Options::read(&self.store, &engine::Flags::default())
    }

    /// The disk cache's limit as `cache.limited` and `cache.limit` give it now, which the
    /// session takes as it starts and again each time it changes (decision 76).
    pub fn cache_limit(&self) -> apertrust_base::cache::Limit {
        engine::cache_limit(&self.store)
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
    use toml::{Table, Value};

    use super::testing::Folder;
    use super::*;
    use crate::PANEL_WIDTHS;

    /// Settings with every field off its default.
    fn everything() -> Settings {
        Settings {
            theme: Builtin::HighContrast,
            locale: Some(Locale::Pl),
            scale: 1.3,
            marker: Marker::Triangle,
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
            panel_width: Some(480.0),
            central_panel: CentralPanel {
                margin: 16.0,
                proof_margin: 0.25,
                background: Some(apertrust_ui::Background::Medium),
            },
            edit: ViewState {
                tab: Some("correct".to_owned()),
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
            grid: Grid {
                sort: Sort {
                    key: SortKey::FileName,
                    descending: true,
                },
                thumbnails: ThumbnailSize::Large,
                rows: 3,
                position: GridPosition::Right,
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
            ask_before_overwrite: false,
        }
    }

    /// Opens `folder`'s settings, changes them to `changed` as a frame would, and ends as
    /// the app does on exit.
    fn change(folder: &Folder, changed: &Settings) {
        let (mut keeper, _) = Keeper::open(folder.storage());
        keeper.frame(Instant::now(), changed);
        keeper.flush();
    }

    /// The recent folders an earlier build kept are read for their move, and left in the
    /// file as they are when the settings are written again.
    #[test]
    fn the_old_recent_folders_are_read_for_the_move_and_left_alone() {
        let folder = Folder::new("old-recents");
        let text = "version = 1\n\n[library]\nrecent_folders = [\"/photos/b\", \"/photos/a\"]\n";
        std::fs::write(folder.settings(), text).expect("written");
        let (mut keeper, settings) = Keeper::open(folder.storage());
        let old = [PathBuf::from("/photos/b"), PathBuf::from("/photos/a")];
        assert_eq!(keeper.old_recent_folders(), old);
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
                panel_width: None,
                central_panel: CentralPanel {
                    margin: 10.0,
                    proof_margin: 0.12,
                    background: None,
                },
                edit: ViewState {
                    tab: None,
                    sections: BTreeMap::new(),
                    groups: BTreeMap::new(),
                },
                scopes: ScopeView::default(),
                grid: Grid {
                    sort: Sort {
                        key: SortKey::CaptureTime,
                        descending: false,
                    },
                    thumbnails: ThumbnailSize::Medium,
                    rows: 1,
                    position: GridPosition::Bottom,
                    columns: 1,
                    labels: true,
                    square_thumbnails: false,
                },
                keys: BTreeMap::new(),
                export: Values::default(),
                ask_before_overwrite: true,
            }
        );
        // What the app takes where nothing is kept.
        assert_eq!(LOCALE.default(), crate::system_locale());
        assert_eq!(SIDE_PANEL_WIDTH.default(), 320.0);
        assert_eq!(
            SIDE_PANEL_WIDTH.default(),
            Theme::builtin(Builtin::Dark).sizes.side_panel
        );
        // The tab the shell shows when none is kept, or the one kept is no tab.
        assert_eq!(Some(EDIT_TAB.default()), crate::tab_id(crate::BASE));
        assert!(EDIT_SECTIONS.default().is_empty());
        assert!(EDIT_GROUPS.default().is_empty());
        assert!(KEYS.default().is_empty());
        // So the store reads what nothing keeps as they are.
        assert_eq!(Keeper::nowhere().1, Settings::default());
    }

    /// The settings the app's own settings write: the app's and the export settings the
    /// Export dialog keeps. The engine's, and the page Settings last showed, the Settings
    /// dialog keeps through the keeper (`Keeper::apply`); the old recent folders are only
    /// read, for their move into `state.toml` (`Keeper::old_recent_folders`).
    fn declared() -> impl Iterator<Item = &'static str> {
        let export = apertrust_export::settings::registry::ALL.iter();
        let (page, old) = (SETTINGS_WINDOW_PAGE.id, LIBRARY_RECENT_FOLDERS.id);
        registry::ALL
            .iter()
            .chain(export)
            .map(|s| s.id())
            .filter(move |id| *id != page && *id != old)
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

version = 1
locale = \"pl\"
scale = 1.3
theme = \"high-contrast\"

[central_panel]
background = \"medium\"
margin = 16
proof_margin = 0.25

[edit]
tab = \"correct\"

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

[grid_panel]
columns = 2
descending = true
labels = false
position = \"right\"
rows = 3
sort = \"file-name\"
square_thumbnails = true
thumbnails = \"large\"

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

[side_panel]
tab_names = false
width = 480

[sliders]
marker = \"triangle\"

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
        // The windows' positions go; the grid panel's, a choice, stays at its default.
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
             [side_panel]\nwidth = 2000\n\
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
                "side_panel.width",
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
             [side_panel]\nwidth = \"wide\"\n\
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
        assert_eq!(table["side_panel"]["width"].as_str(), Some("wide"));
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

    /// The language is kept once the user chooses one, even the system's, and the side
    /// panel's width once dragged, even to the theme's: computed defaults are no values.
    #[test]
    fn a_choice_equal_to_a_computed_default_is_kept() {
        let folder = Folder::new("computed");
        change(
            &folder,
            &Settings {
                locale: Some(crate::system_locale()),
                panel_width: Some(SIDE_PANEL_WIDTH.default()),
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
        use apertrust_engine::gpu::AdapterChoice;
        let folder = Folder::new("engine");
        let text = "version = 1\nscale = 2\n[engine]\nadapter = \"Radeon\"\nmemory_budget = 4096\n";
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
        assert_eq!(table["engine"]["memory_budget"].as_integer(), Some(4096));
        // Nothing kept: the defaults.
        let options = Keeper::nowhere().0.engine();
        let defaults = (AdapterChoice::Any, 2048 << 20);
        assert_eq!((options.gpu.adapter, options.budget), defaults);
    }

    fn conf(given: &[&str]) -> Result<Overrides, OverrideError> {
        let given: Vec<String> = given.iter().map(|g| (*g).to_owned()).collect();
        overrides(&given)
    }

    /// The central panel's margins are 10 points and 0.2 of the slot's smaller side, as
    /// darktable's darkroom has them (Proof margin's 40% less); the file or `--conf` sets
    /// either, a number outside its range read as its nearer end, and neither is on a page
    /// of the Settings dialog.
    #[test]
    fn the_central_panel_s_margins_have_their_defaults_and_follow_the_file_and_conf() {
        let (_, settings) = Keeper::nowhere();
        assert_eq!(settings.central_panel, CentralPanel::default());
        assert_eq!(
            (
                settings.central_panel.margin,
                settings.central_panel.proof_margin
            ),
            (10.0, 0.12)
        );

        let folder = Folder::new("margins-file");
        let text = "version = 1\n[central_panel]\nmargin = 24\nproof_margin = 0.3\n";
        std::fs::write(folder.settings(), text).expect("written");
        let (_, settings) = Keeper::open(folder.storage());
        assert_eq!(
            settings.central_panel,
            CentralPanel {
                margin: 24.0,
                proof_margin: 0.3,
                background: None
            }
        );

        // `--conf` over the file, and a value out of range at its nearer end.
        let overrides =
            conf(&["central_panel.margin=16", "central_panel.proof_margin=0.9"]).expect("read");
        let (keeper, settings) = Keeper::open_with(folder.storage(), &overrides);
        assert_eq!(
            settings.central_panel,
            CentralPanel {
                margin: 16.0,
                proof_margin: 0.4,
                background: None
            }
        );
        assert!(keeper.store.overridden(&CENTRAL_PANEL_MARGIN));
        assert!(!keeper.store.overridden(&SIDE_PANEL_TAB_NAMES));
        assert_eq!(folder.text(), text, "never written");
        // Without a file's value and without `--conf`, the default.
        let none = Folder::new("margins-conf");
        let overrides = conf(&["central_panel.margin=0"]).expect("read");
        let (_, settings) = Keeper::open_with(none.storage(), &overrides);
        assert_eq!(settings.central_panel.margin, 0.0);
        assert_eq!(settings.central_panel.proof_margin, 0.12);
        assert!(conf(&["central_panel.margin=wide"]).is_err());
    }

    #[test]
    fn the_background_follows_the_theme_until_one_is_chosen() {
        use apertrust_ui::Background;
        assert_eq!(CENTRAL_PANEL_BACKGROUND.default(), Background::Dark);
        let (_, settings) = Keeper::nowhere();
        assert_eq!(settings.central_panel.background, None);

        let folder = Folder::new("background-file");
        let text = "version = 1\n[central_panel]\nbackground = \"very-dark\"\n";
        std::fs::write(folder.settings(), text).expect("written");
        let (_, settings) = Keeper::open(folder.storage());
        assert_eq!(
            settings.central_panel.background,
            Some(Background::VeryDark)
        );

        let overrides = conf(&["central_panel.background=white"]).expect("read");
        let (_, settings) = Keeper::open_with(folder.storage(), &overrides);
        assert_eq!(settings.central_panel.background, Some(Background::White));
        assert!(conf(&["central_panel.background=grey"]).is_err());
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
        assert!(!keeper.store.overridden(&SIDE_PANEL_TAB_NAMES));
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

version = 1
scale = 2

[engine]
memory_budget = 4096

[side_panel]
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
    /// not written (`alerts::Overwrite`).
    #[test]
    fn a_yes_that_stops_the_overwrite_question_is_kept_unless_conf_gives_the_setting() {
        use crate::alerts::{Overwrite, Purpose};
        use crate::dialog_host::Answer;
        let table = |folder: &Folder| -> Table { folder.text().parse().expect("TOML") };

        let folder = Folder::new("ask-overwrite");
        let (mut keeper, settings) = Keeper::open(folder.storage());
        assert!(settings.ask_before_overwrite);
        let mut overwrite = Overwrite::new(settings.ask_before_overwrite, false);
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
        let mut overwrite = Overwrite::new(settings.ask_before_overwrite, true);
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
        assert_eq!(table["side_panel"]["tab_names"].as_bool(), Some(false));

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
        let overrides = conf(&["locale=UK", "engine.adapter=M4", "side_panel.width=2000"]);
        let (keeper, settings) = Keeper::open_in(None, &overrides.expect("read"));
        // A language's tag in any case, as the file's; out of range, the nearer end.
        assert_eq!(settings.locale, Some(Locale::Uk));
        assert_eq!(settings.panel_width, Some(*crate::PANEL_WIDTHS.end()));
        let named = apertrust_engine::gpu::AdapterChoice::Named("M4".to_owned());
        assert_eq!(keeper.engine().gpu.adapter, named);
        assert_eq!(
            conf(&["density=compact"]).map(drop),
            Err(OverrideError::Unknown("density".to_owned()))
        );
        let refused = conf(&["scale=1.2", "theme=solarized"]).expect_err("refused");
        assert_eq!(refused.to_string(), "--conf theme: cannot be \"solarized\"");
    }
}
