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

//! The Settings dialog's content (`docs/gui.md`, The Settings dialog), drawn from the
//! settings registry (decision 43), as darktable's preferences are: the pages listed at
//! the start, the chosen one marked, and the chosen page's groups, each under its title,
//! with a row per setting the schema puts there, in the schema's order. A row is the
//! design system's [`SettingRow`]: its label at the start and its control at the end, in
//! the column, by its kind and the schema's `control`:
//!
//! - a choice a [`Combo`], or [`Segmented`] where the schema says so;
//! - a boolean a [`Switch`], the whole row its target;
//! - a number a [`Slider`] where it has both ends, else a [`Number`] box, as it is where
//!   the schema asks one;
//! - a text a [`Text`] field, or a [`TextArea`] or a [`Folder`] where the schema says so;
//! - a list of a choice [`Chips`], in its type's order.
//!
//! - A row's help, where its words have one (`setting-<id>-help`), is its tooltip.
//! - A row off its default has the changed mark after its label, a button that resets it,
//!   its tooltip naming the default; a double-click on its label, Delete on it, or Reset in
//!   its right-click menu, puts the default back too.
//! - A row `--conf` sets for this run is dimmed, its tooltip saying so (decision 43), and
//!   so is one whose required boolean (the schema's `requires`) is off, as the cache's
//!   limit while it is unlimited; neither has a mark.
//! - A row whose `platforms` leave out the platform the dialog runs on, as the context's
//!   operating system says, is not drawn: the graphics API's off Windows.
//! - "· after a restart" follows the label of a setting that applies then.
//! - Processing's Hardware acceleration group draws the graphics card as a list of the
//!   adapters the chosen API finds, says what is wrong with a card on its row, and ends in
//!   the In use line, then, while a choice waits for a restart, the waiting line with
//!   Restart now ([`hardware`], decision 82).
//! - Every change is kept as it is committed, with no OK or Apply, and takes effect as its
//!   setting's `applies` says: it goes to the app as a
//!   [`Change`], which keeps it through the settings' keeper and shows it in the main
//!   window; the theme and the language are shown in this dialog's own window at once too.
//! - A page that holds nothing yet is listed dimmed and cannot be chosen. The page last
//!   shown is kept, `settings_window.page`.
//! - Find a setting, at the page list's top, finds the rows whose label or help holds what
//!   it holds, case and accents aside: the pages with none are dimmed, still chosen, and
//!   the shown page's rows that match have their labels underlined. Ctrl+F (Cmd+F) gives it
//!   the focus, and Escape clears it while it holds anything: in it, which keeps the
//!   focus, or from a control that is not a field, a field being typed in keeping its own.
//! - The footer has Close at its end where the window has one.
//!
//! It is a [`Dialog`] (decision 41), which the dialog host draws in a window of its own:
//! modeless on the Mac and modal on Windows and Linux.

use apertrust_base::settings::{
    Applies, CONF_FLAG, Choice as _, Control, Declared, Kind, Page, Platform,
};
use apertrust_ui::components::group_label_text;
use apertrust_ui::components::settings::{
    Chips, Combo, Folder, Note, Number, Search, Segmented, SettingRow, Slider, Switch, Text,
    TextArea,
};
use apertrust_ui::popup;
use apertrust_ui::{Builtin, Icon, Locale, Theme, Translator, fonts, theme, tr, translator};
use eframe::egui::os::OperatingSystem;
use eframe::egui::{
    self, Align, FontFamily, FontId, Key, Layout, Modifiers, Rect, Role, ScrollArea, Sense, Stroke,
    Ui, UiBuilder, WidgetInfo, pos2, vec2,
};
use toml::Value;

use apertrust_engine::gpu::Backend;
use apertrust_engine::settings::registry::{ENGINE_ADAPTER, ENGINE_BACKEND};

use crate::dialog_host::{Asks, Dialog, Form, Keys, Look, Output};
use crate::hardware::{self, Hardware};
use crate::settings::{self, Keeper, Position};
use crate::storage;

/// The dialog's width, in points, as its design has it.
pub const WIDTH: f32 = 960.0;
/// The page list's width.
const PAGES: f32 = 200.0;
/// The height of the page list and of the page beside it.
const BODY: f32 = 640.0;
/// A group title's line, the artboard's.
const GROUP_LINE: f32 = 26.0;
/// The room between one group's last row and the next group's title, the artboard's.
const GROUP_GAP: f32 = 14.0;
/// The page list's padding above Find a setting and between it and the pages, the
/// artboard's.
const LIST_PADDING: f32 = 10.0;
/// A page's line in the list: a row's 26 points (`sizes.row`) and 5 more above and below,
/// so the icons and names breathe.
const PAGE_LINE: f32 = 36.0;
/// How far a page's icon sits in from its line's start, the artboard's.
const PAGE_INSET: f32 = 10.0;
/// A page's icon, before its name.
const PAGE_ICON: f32 = 16.0;
/// Between a page's icon and its name.
const PAGE_ICON_GAP: f32 = 8.0;
/// Find a setting's id, as Ctrl+F (Cmd+F) gives it the focus.
const FIND: &str = "settings find";

/// One setting's row, as the dialog shows it: what the registry declares of it, its value
/// and its default as `settings.toml` keeps them, and whether `--conf` sets it.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub id: &'static str,
    pub page: Page,
    pub group: &'static str,
    pub kind: Kind,
    pub applies: Applies,
    /// A choice's options' ids, in its type's order.
    pub options: Vec<&'static str>,
    pub value: Option<Value>,
    pub default: Option<Value>,
    /// Whether its default is one the app works out, as the language's from the system's.
    pub computed: bool,
    pub overridden: bool,
    /// The boolean setting whose row must be on for this one to be enabled.
    pub requires: Option<&'static str>,
    /// The platforms it is drawn on; None for every one.
    pub platforms: Option<&'static [Platform]>,
    /// The control the schema names in place of its kind's own.
    pub control: Option<Control>,
}

impl Row {
    /// `setting`'s row as `keeper` holds it, or none for a setting on no page.
    fn of(setting: &'static dyn Declared, keeper: &Keeper) -> Option<Row> {
        let (page, group) = setting.page()?;
        Some(Row {
            id: setting.id(),
            page,
            group,
            kind: *setting.kind(),
            applies: setting.applies(),
            options: setting.options(),
            value: keeper.value(setting),
            default: setting.default_value(),
            computed: setting.default_is_computed(),
            overridden: keeper.overridden(setting),
            requires: setting.requires(),
            platforms: setting.platforms(),
            control: setting.control(),
        })
    }

    /// Whether the dialog draws it on `os`: on every platform, or on one its `platforms`
    /// name. None of them is any other system's.
    pub fn drawn_on(&self, os: OperatingSystem) -> bool {
        let platform = match os {
            OperatingSystem::Windows => Some(Platform::Windows),
            OperatingSystem::Mac => Some(Platform::Mac),
            OperatingSystem::Nix => Some(Platform::Linux),
            _ => None,
        };
        self.platforms
            .is_none_or(|platforms| platform.is_some_and(|p| platforms.contains(&p)))
    }

    /// Whether it is off its default, which the changed mark says.
    pub fn changed(&self) -> bool {
        self.value != self.default
    }

    /// The id as its words' ids write it: dots and underscores as hyphens.
    fn words(&self) -> String {
        self.id.replace(['.', '_'], "-")
    }
}

/// What the dialog shows: every row of every page, the page shown, and whether it ends in
/// a Close button.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub rows: Vec<Row>,
    pub page: Page,
    /// Whether it ends in a Close button: in the modal window of Windows and Linux, not in
    /// the Mac's window, which its own close button and Cmd+W close, as Apple's Settings
    /// windows have it.
    pub close_button: bool,
    /// Storage's hand-built part, as the app last handed it.
    storage: storage::View,
    /// The kinds whose Clear was pressed since the app last handed the view, disabled
    /// meanwhile, so a second press never asks again.
    pressed: Vec<apertrust_base::cache::Kind>,
    /// Whether Storage asked for the sizes since it was last shown.
    sizes_asked: bool,
    /// Whether its window had the focus at the last frame, so Storage asks for the sizes
    /// again as it comes back.
    focused: bool,
    /// Processing's Hardware acceleration group: the adapters listed, and the one in use.
    hardware: Hardware,
    /// What Find a setting holds.
    query: String,
}

/// What the user changed in the dialog: a setting set to a value, as `settings.toml` keeps
/// it, or reset to its default, by its id. The app keeps it with the rest of settings and
/// shows it in the main window; the dialog has applied the theme and the language to its
/// own window already.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    Set(&'static str, Value),
    Reset(&'static str),
}

impl Change {
    /// The setting's id, and the value it is set to; none for a reset.
    pub fn parts(&self) -> (&'static str, Option<&Value>) {
        match self {
            Change::Set(id, value) => (id, Some(value)),
            Change::Reset(id) => (id, None),
        }
    }
}

/// What one frame of the dialog asked for.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Asked {
    /// What the user changed, in order: a row's change, and a graphics card an API change
    /// left out going back to Automatic with it.
    pub changes: Vec<Change>,
    /// What Storage asked, after the change.
    pub storage: Vec<storage::Asked>,
    /// Restart now was pressed, for the app to run its `restart` action.
    pub restart: bool,
    /// Its Close button was pressed.
    pub close: bool,
}

/// Settings in the dialog host: Apple's Settings window on the Mac, modeless, kept where it
/// was left, with no Close button and Escape doing nothing, its own close button and Close
/// Window (Cmd+W, the menu's) closing it; elsewhere a modal window with a Close button,
/// which Escape closes too (decision 41).
impl Dialog for Settings {
    fn id(&self) -> &'static str {
        "settings"
    }

    fn title(&self, ctx: &egui::Context) -> String {
        tr(ctx, "action-settings")
    }

    fn form(&self, os: OperatingSystem) -> Form {
        if os == OperatingSystem::Mac {
            Form::Modeless
        } else {
            Form::Modal
        }
    }

    /// The panel's colour to its edges: the page list and the page draw their own margins.
    fn look(&self, t: &Theme) -> Look {
        Look {
            background: t.colors.panel,
            margin: 0.0,
        }
    }

    fn keys(&self, os: OperatingSystem) -> Keys {
        Keys {
            escape: os != OperatingSystem::Mac,
            ..Keys::default()
        }
    }

    fn show(&mut self, ui: &mut egui::Ui) -> Asks {
        // Escape clears Find a setting first, while it holds anything; empty, Find a
        // setting keeps none, though it had the keyboard, so the host's Escape closes.
        let open = self.menu_open(ui.ctx()) || !self.query.is_empty();
        let finding = ui.memory(|m| m.had_focus_last_frame(Search::id_of(FIND)));
        // Escape held to clear what is found repeats, and a repeat is no press of its own:
        // it closes nothing, though Find a setting is empty by then, or egui has taken the
        // focus from it. A fresh press alone closes the dialog.
        let repeat = ui.input(|i| {
            let escape = |repeated| {
                i.events.iter().any(|event| {
                    matches!(
                        event,
                        egui::Event::Key { key: Key::Escape, pressed: true, repeat, .. }
                            if *repeat == repeated
                    )
                })
            };
            escape(true) && !escape(false)
        });
        let gives_escape = finding && self.query.is_empty();
        let asked = self.frame(ui);
        let change = asked.changes.into_iter().map(Output::Settings);
        let storage = asked.storage.into_iter().map(Output::Storage);
        let restart = asked.restart.then_some(Output::Restart);
        Asks {
            outputs: change.chain(storage).chain(restart).collect(),
            close: asked.close,
            keeps_escape: open || repeat,
            gives_escape,
        }
    }

    fn placed(&self, at: Position) -> Option<Output> {
        Some(Output::Placed(at))
    }
}

impl Settings {
    /// The dialog over the settings `keeper` holds, on the page it last showed, or General
    /// where that page holds nothing yet.
    pub fn new(keeper: &Keeper, close_button: bool) -> Settings {
        let rows: Vec<Row> = settings::shown()
            .into_iter()
            .filter_map(|setting| Row::of(setting, keeper))
            .collect();
        let mut dialog = Settings {
            rows,
            page: Page::General,
            close_button,
            storage: storage::View::default(),
            pressed: Vec::new(),
            sizes_asked: false,
            focused: true,
            hardware: Hardware::default(),
            query: String::new(),
        };
        let kept = keeper.page();
        if dialog.holds(kept, OperatingSystem::from_target_os()) {
            dialog.page = kept;
        }
        dialog
    }

    /// The dialog with Storage's hand-built part showing `view`.
    pub fn with_storage(mut self, view: storage::View) -> Settings {
        self.set_storage(view);
        self
    }

    /// The dialog with the Hardware acceleration group handed `running`: the adapter in
    /// use, and how to list the others.
    pub fn with_hardware(mut self, running: hardware::Running) -> Settings {
        self.hardware = Hardware::new(running);
        self
    }

    /// Hands the Hardware acceleration group `running` again, as the app changed it, and
    /// takes in `changes` the app made to the settings meanwhile, as Keep's, but for a row
    /// `--conf` sets, whose value the store refuses to change: whether it changed anything,
    /// so its window is drawn again.
    pub fn set_hardware(&mut self, running: hardware::Running, changes: &[Change]) -> bool {
        let mut changed = false;
        for change in changes {
            let (id, value) = change.parts();
            let row = self.rows.iter_mut().find(|row| row.id == id);
            if let Some(row) = row.filter(|row| !row.overridden) {
                let value = value.cloned().or_else(|| row.default.clone());
                changed |= row.value != value;
                row.value = value;
            }
        }
        if self.hardware.running() != &running {
            self.hardware.set_running(running);
            changed = true;
        }
        changed
    }

    /// What Storage's hand-built part shows.
    pub fn storage(&self) -> &storage::View {
        &self.storage
    }

    /// Storage's hand-built part showing `view`, as the app hands it after a press: a
    /// Clear pressed since is the view's to disable now.
    pub fn set_storage(&mut self, view: storage::View) {
        self.storage = view;
        self.pressed.clear();
    }

    /// Whether `page` holds a row drawn on `os`, so it can be chosen.
    pub fn holds(&self, page: Page, os: OperatingSystem) -> bool {
        self.rows
            .iter()
            .any(|row| row.page == page && row.drawn_on(os))
    }

    /// The row of the setting `id`.
    #[cfg(test)]
    pub fn row(&self, id: &str) -> Option<&Row> {
        self.rows.iter().find(|row| row.id == id)
    }

    /// Why `row` cannot be changed now, in `words`, or none where it can: `--conf` sets
    /// it, or the row it requires is off.
    fn disabled(&self, row: &Row, words: &Translator) -> Option<String> {
        let mut args = apertrust_ui::i18n::Args::new();
        if row.overridden {
            args.set("flag", CONF_FLAG);
            return Some(words.text_with("settings-overridden", Some(&args)));
        }
        let required = row.requires?;
        let other = self.rows.iter().find(|other| other.id == required);
        if other.and_then(|other| other.value.as_ref()?.as_bool()) == Some(true) {
            return None;
        }
        let label = other
            .map(|other| label_of(words, other))
            .unwrap_or_default();
        args.set("setting", label.as_str());
        Some(words.text_with("settings-requires", Some(&args)))
    }

    /// The text the row of the setting `id` holds, empty where it holds none.
    fn text(&self, id: &str) -> &str {
        let row = self.rows.iter().find(|row| row.id == id);
        row.and_then(|row| row.value.as_ref()?.as_str())
            .unwrap_or("")
    }

    /// The graphics API the card's list is for: the API row's, where this build has it,
    /// else Automatic, as the engine takes it.
    fn backend(&self) -> Backend {
        let backend = Backend::from_id(self.text(ENGINE_BACKEND.id));
        backend
            .filter(|backend| self.hardware.built(*backend))
            .unwrap_or_default()
    }

    /// The Hardware acceleration group's parts this frame, on Processing.
    fn hardware_frame(&mut self, ctx: &egui::Context) -> Option<hardware::Frame> {
        if self.page != Page::Processing {
            return None;
        }
        let (stored, backend) = (self.text(ENGINE_ADAPTER.id).to_owned(), self.backend());
        let words = translator(ctx);
        Some(self.hardware.frame(ctx, &words, &stored, backend))
    }

    /// Whether a row's right-click menu is open, which Escape closes first.
    fn menu_open(&self, ctx: &egui::Context) -> bool {
        self.rows
            .iter()
            .any(|row| SettingRow::menu_open(ctx, row.id))
    }

    /// Shows `page`, chosen in the list, keeping it in `changes`.
    fn show_page(&mut self, page: Page, changes: &mut Vec<Change>) {
        self.page = page;
        self.hardware.left();
        changes.push(Change::Set(
            settings::registry::SETTINGS_WINDOW_PAGE.id,
            Value::String(page.id().to_owned()),
        ));
    }

    /// Draws the dialog in `ui`: the page list, the page, and Close where it has one.
    fn frame(&mut self, ui: &mut Ui) -> Asked {
        let ctx = ui.ctx().clone();
        let t = theme(&ctx);
        let mut asked = Asked::default();
        // A row's open menu is drawn, and takes the pointer, before its answer is read.
        if self.menu_open(&ctx) {
            popup::show(&ctx);
        }
        // Ctrl+F (Cmd+F) finds a setting; Escape clears what is being found from a control
        // that is no field. Find a setting clears itself and keeps the focus, and a field
        // being typed in keeps its own Escape (`dialog_host::typing`). Find a setting empty
        // keeps none: the host closes by its own binding (`Asks::gives_escape`).
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::F)) {
            // A list or menu left open would take the letters typed next, so it closes,
            // giving no focus back: Find a setting takes it.
            popup::dismiss_all(&ctx);
            ctx.memory_mut(|m| m.request_focus(Search::id_of(FIND)));
        }
        let typing = crate::dialog_host::typing(&ctx);
        let escape = ctx.input(|i| i.key_pressed(Key::Escape));
        if escape && !self.query.is_empty() && !typing {
            self.query.clear();
        }
        ui.set_width(WIDTH);
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        ui.horizontal_top(|ui| {
            // A page chosen shows at once, but while a field is typed in: the page shown is
            // drawn once more then, so the field the choice took the focus from commits what
            // it holds, and the new page shows from the next frame. Every route takes the
            // focus from the field here, a screen reader's click too, which moves none.
            let chosen = self.pages(ui, &t);
            let (now, after) = match chosen {
                Some(page) if typing => {
                    ctx.memory_mut(|m| {
                        let tab = |id| Page::ALL.into_iter().any(|page| page_id(page) == id);
                        if let Some(field) = m.focused().filter(|id| !tab(*id)) {
                            m.surrender_focus(field);
                        }
                    });
                    (None, Some(page))
                }
                chosen => (chosen, None),
            };
            if let Some(page) = now {
                self.show_page(page, &mut asked.changes);
            }
            // The page's own parts, once the page shown this frame is known.
            let gpu = self.hardware_frame(&ctx);
            let width = WIDTH - PAGES;
            ui.allocate_ui_with_layout(vec2(width, BODY), Layout::top_down(Align::Min), |ui| {
                ui.set_min_size(vec2(width, BODY));
                apertrust_ui::components::quiet_scroll(
                    ui,
                    ScrollArea::vertical()
                        .id_salt("settings page")
                        .max_height(BODY)
                        .auto_shrink([false, false]),
                    |ui| {
                        egui::Frame::NONE
                            .inner_margin(egui::Margin::symmetric(
                                t.spacing.xxl as i8,
                                t.spacing.l as i8,
                            ))
                            .show(ui, |ui| {
                                let (changes, storage, restart) =
                                    self.page_rows(ui, &t, gpu.as_ref());
                                asked.restart = restart;
                                asked.changes.extend(changes);
                                if let Some(storage::Asked::Clear(kind)) = &storage {
                                    self.pressed.push(*kind);
                                }
                                asked.storage.extend(storage);
                            });
                    },
                );
            });
            if let Some(page) = after {
                self.show_page(page, &mut asked.changes);
                ctx.request_repaint();
            }
        });
        // A card `--conf` holds for this run is never the API change's to reset.
        let held = self
            .rows
            .iter()
            .any(|row| row.id == ENGINE_ADAPTER.id && row.overridden);
        for change in &asked.changes {
            match change.parts().0 {
                id if id == ENGINE_BACKEND.id && !held => {
                    let stored = self.text(ENGINE_ADAPTER.id).to_owned();
                    self.hardware.api_changed(self.backend(), &stored);
                }
                id if id == ENGINE_ADAPTER.id => self.hardware.card_changed(),
                _ => {}
            }
            self.take(change, &ctx);
        }
        // A card the person's API change left out goes back to Automatic once the new
        // API's listing is in: in this frame too, beside the API's change, where it is.
        if !held
            && self
                .hardware
                .settle(&ctx, &translator(&ctx), self.backend())
        {
            let reset = Change::Reset(ENGINE_ADAPTER.id);
            self.take(&reset, &ctx);
            asked.changes.push(reset);
        }
        // Storage asks for the sizes each time it is shown, and as its window comes back
        // to the front.
        let focused = ctx.input(|i| i.focused);
        let was = std::mem::replace(&mut self.focused, focused);
        let back = focused && !was;
        if self.page != Page::Storage {
            self.sizes_asked = false;
        } else if !std::mem::replace(&mut self.sizes_asked, true) || back {
            asked.storage.push(storage::Asked::Sizes);
        }
        // The footer, where the window has Close.
        if self.close_button {
            let (line, _) = ui.allocate_exact_size(vec2(WIDTH, 1.0), Sense::hover());
            ui.painter().rect_filled(line, 0.0, t.colors.separator);
            egui::Frame::NONE
                .inner_margin(egui::Margin::same(t.spacing.l as i8))
                .show(ui, |ui| {
                    ui.set_width(WIDTH - 2.0 * t.spacing.l);
                    let close = tr(&ctx, "settings-close");
                    asked.close = crate::dialog_host::buttons(ui, &[&close]).is_some();
                });
        }
        asked
    }

    /// Takes in `change`: its row shows it, and a theme or a language shows in this
    /// dialog's own window at once.
    fn take(&mut self, change: &Change, ctx: &egui::Context) {
        let (id, value) = change.parts();
        let Some(row) = self.rows.iter_mut().find(|row| row.id == id) else {
            return;
        };
        row.value = value.cloned().or_else(|| row.default.clone());
        let chosen = row.value.as_ref().and_then(Value::as_str);
        if id == settings::registry::THEME.id
            && let Some(theme) = chosen.and_then(Builtin::from_id)
        {
            apertrust_ui::set_theme(ctx, Theme::builtin(theme));
        }
        if id == settings::registry::LOCALE.id
            && let Some(locale) = chosen.and_then(Locale::from_id)
        {
            apertrust_ui::set_locale(ctx, locale);
        }
    }

    /// The page list: Find a setting at its top, then each page a line, its icon and its
    /// name in a tool title's type, the shown one on a `raised` plate with its icon filled,
    /// those that hold nothing yet dimmed, saying they come later, and never chosen; while
    /// Find a setting holds something, those with no row that matches it dimmed too, but
    /// chosen still. A click, or ↑ and ↓ while a line has the focus, chooses one. Returns
    /// the page chosen.
    fn pages(&mut self, ui: &mut Ui, t: &Theme) -> Option<Page> {
        let ctx = ui.ctx().clone();
        let (column, _) = ui.allocate_exact_size(vec2(PAGES, BODY), Sense::hover());
        let painter = ui.painter_at(column);
        painter.rect_filled(column, 0.0, t.colors.sunken);
        // A hairline in `separator` at its end, as the footer's above it.
        let line = t.shape.hairline;
        painter.rect_filled(
            Rect::from_x_y_ranges(column.right() - line..=column.right(), column.y_range()),
            0.0,
            t.colors.separator,
        );
        // The list's own padding, the artboard's.
        let inner = Rect::from_min_max(
            column.min + vec2(t.spacing.m, LIST_PADDING),
            pos2(column.right() - line - t.spacing.m, column.bottom()),
        );
        let find = tr(&ctx, "settings-find");
        let search = Rect::from_min_size(inner.min, vec2(inner.width(), t.sizes.setting_control));
        let mut field = ui.new_child(UiBuilder::new().max_rect(search));
        if let Some(typed) = Search::new(FIND, &find, &self.query).show(&mut field) {
            self.query = typed;
        }
        let words = translator(&ctx);
        let later = tr(&ctx, "settings-page-later");
        // A tool title's type: its size in the Medium face.
        let font = FontId::new(t.text.section_title, FontFamily::Name(fonts::MEDIUM.into()));
        let mut chosen = None;
        let mut moved = None;
        let top = search.bottom() + LIST_PADDING;
        for (i, page) in Page::ALL.into_iter().enumerate() {
            let rect = Rect::from_min_size(
                pos2(inner.left(), top + i as f32 * PAGE_LINE),
                vec2(inner.width(), PAGE_LINE),
            );
            let enabled = self.holds(page, ctx.os());
            let found = self.query.trim().is_empty() || self.finds(page, &words, ctx.os());
            let shown = page == self.page;
            let id = page_id(page);
            let sense = if enabled {
                Sense::click()
            } else {
                Sense::hover()
            };
            let response = ui.interact(rect, id, sense);
            let name = tr(&ctx, &format!("settings-page-{}", page.id()));
            response.widget_info(|| WidgetInfo::selected(Role::Tab, enabled, shown, &name));
            let colour = if !enabled {
                t.colors.text_disabled
            } else if !found {
                t.colors.text_secondary
            } else if shown {
                t.colors.text_strong
            } else {
                t.colors.text
            };
            if shown {
                painter.rect_filled(rect, t.shape.radius_small, t.colors.raised);
            }
            let icon = page_icon(page);
            let icon = if shown { icon.filled() } else { icon };
            let at = pos2(rect.left() + PAGE_INSET, rect.center().y - PAGE_ICON / 2.0);
            icon.paint(
                &painter,
                Rect::from_min_size(at, vec2(PAGE_ICON, PAGE_ICON)),
                colour,
            );
            let text = painter.layout_no_wrap(name, font.clone(), colour);
            painter.galley(
                pos2(
                    rect.left() + PAGE_INSET + PAGE_ICON + PAGE_ICON_GAP,
                    rect.center().y - text.size().y / 2.0,
                ),
                text,
                colour,
            );
            if response.has_focus() {
                apertrust_ui::components::focus_ring_centred(&painter, rect, t);
                let step = |key| ctx.input_mut(|i| i.consume_key(Modifiers::NONE, key));
                if step(Key::ArrowDown) {
                    moved = self.next(page, 1, ctx.os());
                } else if step(Key::ArrowUp) {
                    moved = self.next(page, -1, ctx.os());
                }
            }
            let response = if enabled {
                response
            } else {
                response.on_hover_text(&later)
            };
            if response.clicked() && !shown {
                chosen = Some(page);
            }
        }
        if let Some(page) = moved {
            ctx.memory_mut(|m| m.request_focus(page_id(page)));
            chosen = Some(page).filter(|page| *page != self.page);
        }
        chosen
    }

    /// Whether `row` matches what Find a setting holds: its label or its help, in `words`,
    /// holds it, case and accents aside.
    fn matches(&self, row: &Row, words: &Translator) -> bool {
        let wanted = apertrust_ui::i18n::fold(self.query.trim());
        if wanted.is_empty() {
            return false;
        }
        let help = format!("setting-{}-help", row.words());
        let help = words.has(&help).then(|| words.text(&help));
        std::iter::once(label_of(words, row))
            .chain(help)
            .any(|text| apertrust_ui::i18n::fold(&text).contains(&wanted))
    }

    /// Whether `page` holds a row drawn on `os` that matches what Find a setting holds.
    fn finds(&self, page: Page, words: &Translator, os: OperatingSystem) -> bool {
        self.rows
            .iter()
            .filter(|row| row.page == page && row.drawn_on(os))
            .any(|row| self.matches(row, words))
    }

    /// The page that can be chosen on `os` next to `from`, `by` 1 after it or −1 before it.
    fn next(&self, from: Page, by: isize, os: OperatingSystem) -> Option<Page> {
        let at = Page::ALL.iter().position(|page| *page == from)? as isize;
        let mut i = at + by;
        while (0..Page::ALL.len() as isize).contains(&i) {
            let page = Page::ALL[i as usize];
            if self.holds(page, os) {
                return Some(page);
            }
            i += by;
        }
        None
    }

    /// Whether a setting that applies after a restart has a value other than the one this
    /// run started with, which the waiting line says (decision 82).
    fn waiting(&self) -> bool {
        let started = &self.hardware.running().started;
        self.rows.iter().any(|row| {
            row.applies == Applies::AfterRestart && started.differs(row.id, row.value.as_ref())
        })
    }

    /// The shown page's groups, each under its title, in the order of their first rows,
    /// and their rows; on Storage, its Folders group first and its In the cache group last;
    /// on Processing, the Hardware acceleration group's parts as `gpu` has them, its In use
    /// line, and the waiting line while a change waits for a restart. Returns what the user
    /// changed, every row's change in the rows' order, as a switch's click and a field's
    /// commit as it loses the focus can come in one frame; what Storage asked; and whether
    /// Restart now was pressed.
    fn page_rows(
        &self,
        ui: &mut Ui,
        t: &Theme,
        gpu: Option<&hardware::Frame>,
    ) -> (Vec<Change>, Option<storage::Asked>, bool) {
        let ctx = ui.ctx().clone();
        // The page's rows drawn on this platform.
        let os = ctx.os();
        let shown = || {
            self.rows
                .iter()
                .filter(move |row| row.page == self.page && row.drawn_on(os))
        };
        let mut groups: Vec<&str> = Vec::new();
        for row in shown() {
            if !groups.contains(&row.group) {
                groups.push(row.group);
            }
        }
        let content = ui.available_width();
        let words = translator(&ctx);
        let storage = self.page == Page::Storage;
        let in_use = gpu.and(self.hardware.running().in_use.as_ref());
        let mut changes = Vec::new();
        let mut asked = None;
        let mut restart = false;
        if storage {
            group_title(ui, t, &tr(&ctx, "settings-group-folders"), content);
            asked = storage::folders(ui, t, &self.storage);
        }
        for (i, group) in groups.into_iter().enumerate() {
            if i > 0 || storage {
                ui.add_space(GROUP_GAP);
            }
            let name = tr(&ctx, &format!("settings-group-{}", group.replace('_', "-")));
            group_title(ui, t, &name, content);
            for row in shown().filter(|row| row.group == group) {
                let extra = Extra::of(row, gpu);
                let disabled = self.disabled(row, &words);
                let matched = self.matches(row, &words);
                if let Some(changed) = draw_row(ui, row, disabled.as_deref(), matched, extra) {
                    changes.push(changed);
                }
            }
            // What runs now, under the rows that choose what runs after a restart.
            if let Some(in_use) = in_use.filter(|_| group == HARDWARE_GROUP) {
                ui.add_space(t.spacing.m);
                hardware::in_use(ui, t, in_use);
            }
            // And while what is chosen differs from what this run started with, that it
            // waits for a restart, with Restart now.
            let failed = self.hardware.running().restart_failed.as_deref();
            if gpu.is_some() && group == HARDWARE_GROUP && (self.waiting() || failed.is_some()) {
                ui.add_space(t.spacing.m);
                restart = hardware::waiting(ui, t, content, failed);
            }
        }
        if storage {
            ui.add_space(GROUP_GAP);
            let name = tr(&ctx, "settings-group-cache-contents");
            group_title(ui, t, &name, content);
            let pressed = &self.pressed;
            if let Some(contents) = storage::contents(ui, t, &self.storage, pressed) {
                asked = Some(contents);
            }
        }
        (changes, asked, restart)
    }
}

/// A group's title, `name`, on a line of its own across `content`, drawn as a tool's group
/// titles are (`group_label_text`: 11-point Medium capitals, a little apart, in
/// `text_secondary`), centred on the line, then a `divider` hairline to the line's end. No
/// chevron: a page's groups are always open.
fn group_title(ui: &mut Ui, t: &Theme, name: &str, content: f32) {
    let (line, response) = ui.allocate_exact_size(vec2(content, GROUP_LINE), Sense::hover());
    let colour = t.colors.text_secondary;
    let galley = ui
        .painter()
        .layout_job(group_label_text(t, name, content, colour));
    let at = pos2(line.left(), line.center().y - galley.size().y / 2.0);
    let from = at.x + galley.size().x + t.spacing.m;
    ui.painter().galley(at, galley, colour);
    if from < line.right() {
        let stroke = Stroke::new(t.shape.hairline, t.colors.divider);
        ui.painter()
            .hline(from..=line.right(), line.center().y, stroke);
    }
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, name));
}

/// The id of `page`'s line in the list.
fn page_id(page: Page) -> egui::Id {
    egui::Id::unique(("settings page", page.id()))
}

/// The icon before a page's name, drawn filled on the page shown (`Icon::filled`), as the
/// Tools panel's tabs draw theirs.
fn page_icon(page: Page) -> Icon {
    match page {
        Page::General => Icon::Settings,
        Page::Import => Icon::Import,
        Page::Photos => Icon::Photo,
        Page::Editing => Icon::Adjust,
        Page::Processing => Icon::Cpu,
        Page::Storage => Icon::HardDrive,
        Page::Shortcuts => Icon::Keyboard,
        Page::Presets => Icon::Presets,
        Page::Ai => Icon::Sparkles,
    }
}

/// The words of `row`'s label.
fn label_of(words: &Translator, row: &Row) -> String {
    words.text(&format!("setting-{}-label", row.words()))
}

/// The Hardware acceleration group, whose rows [`Extra`] adds to, by its id.
const HARDWARE_GROUP: &str = "hardware_acceleration";

/// What a row of the Hardware acceleration group draws in place of its kind's (decision
/// 82): the graphics card's list for its text, and what is said under it or the API's.
#[derive(Clone, Copy, Debug, Default)]
struct Extra<'a> {
    card: Option<&'a hardware::Card>,
    note: Option<&'a hardware::Note>,
}

impl<'a> Extra<'a> {
    /// `row`'s, as `gpu` has the group this frame.
    fn of(row: &Row, gpu: Option<&'a hardware::Frame>) -> Extra<'a> {
        match gpu {
            Some(gpu) if row.id == ENGINE_ADAPTER.id => Extra {
                card: Some(&gpu.card),
                note: gpu.card.note.as_ref(),
            },
            Some(gpu) if row.id == ENGINE_BACKEND.id => Extra {
                note: gpu.api_note.as_ref(),
                ..Extra::default()
            },
            _ => Extra::default(),
        }
    }
}

/// Draws `row` as a [`SettingRow`]: its label, its help as its tooltip, the changed mark
/// while it is off its default, "after a restart" where it applies then, its control, and
/// what `extra` says under it; dimmed for the reason `disabled` gives, its label underlined
/// where it is `matched` by Find a setting. A boolean's whole row toggles it. Returns what
/// the user changed.
fn draw_row(
    ui: &mut Ui,
    row: &Row,
    disabled: Option<&str>,
    matched: bool,
    extra: Extra<'_>,
) -> Option<Change> {
    let ctx = ui.ctx().clone();
    let words = translator(&ctx);
    let label = label_of(&words, row);
    let help = format!("setting-{}-help", row.words());
    let help = words.has(&help).then(|| words.text(&help));
    let default = row.changed().then(|| default_words(&ctx, row, extra));
    let switch = matches!(row.kind, Kind::Boolean) && extra.card.is_none();
    let mut setting = SettingRow::new(row.id, &label)
        .after_restart(row.applies == Applies::AfterRestart)
        .whole_row(switch)
        .matched(matched);
    if let Some(help) = &help {
        setting = setting.help(help);
    }
    if let Some(default) = &default {
        setting = setting.changed(default);
    }
    if let Some(why) = disabled {
        setting = setting.disabled(why);
    }
    if let Some(note) = extra.note {
        setting = setting.note(Note {
            text: &note.text,
            warning: note.warning,
            live: note.live,
        });
    }
    // What screen readers hear on a combo after its value: what is said under it, or that
    // it applies after a restart, which its label's note says only to those who read it.
    let restart =
        (row.applies == Applies::AfterRestart).then(|| words.text("settings-restart-note"));
    let said = extra
        .note
        .map(|note| note.text.as_str())
        .or(restart.as_deref());
    let out = setting.show(ui, |ui| control(ui, row, &label, extra, said));
    let on = row.value.as_ref().and_then(Value::as_bool).unwrap_or(false);
    if out.reset {
        Some(Change::Reset(row.id))
    } else if switch && out.clicked {
        // The row's toggle after the switch's own, where both came in this frame: two
        // toggles leave the value as it was.
        let switched = match &out.inner {
            Some(Change::Set(_, Value::Boolean(switched))) => *switched,
            _ => on,
        };
        (switched == on).then_some(Change::Set(row.id, Value::Boolean(!on)))
    } else {
        out.inner
    }
}

/// `row`'s default as its mark's tooltip names it, in the dialog's language: an option's
/// words, a number with its unit, On or Off, a text, and a list's options; the graphics
/// card's Automatic.
fn default_words(ctx: &egui::Context, row: &Row, extra: Extra<'_>) -> String {
    let words = translator(ctx);
    if extra.card.is_some() {
        return words.text("settings-gpu-automatic");
    }
    let default = row.default.as_ref();
    match row.kind {
        Kind::Boolean => match default.and_then(Value::as_bool) {
            Some(true) => words.text("settings-default-on"),
            _ => words.text("settings-default-off"),
        },
        // A default the app works out, as the language's from the system's, is named by the
        // setting's own words, "the system's language", never by the option it gives today.
        Kind::Choice(_) if row.computed => words.text(&format!("setting-{}-default", row.words())),
        Kind::Choice(_) => {
            let (ids, names) = options(ctx, row);
            let chosen = default.and_then(Value::as_str);
            let at = ids.iter().position(|id| Some(*id) == chosen);
            at.map(|at| names[at].clone()).unwrap_or_default()
        }
        Kind::Number(number) => {
            let kept = default.and_then(as_number).unwrap_or(0.0);
            let shown =
                words.format_number(as_shown(&number, kept), shown_decimals(&number), false);
            match number.unit {
                Some(unit) => {
                    let mut args = apertrust_ui::i18n::Args::new();
                    args.set("value", shown.as_str());
                    args.set("unit", words.text(&format!("unit-{unit}")));
                    words.text_with("settings-value-with-unit", Some(&args))
                }
                None => shown,
            }
        }
        Kind::Text => match default.and_then(Value::as_str) {
            Some(text) if !text.is_empty() => text.to_owned(),
            _ => words.text("settings-default-empty"),
        },
        Kind::List {
            of: Kind::Choice(_),
            ..
        } => {
            let (ids, names) = options(ctx, row);
            let items: Vec<&str> = default
                .and_then(Value::as_array)
                .map(|items| items.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            let named: Vec<&str> = ids
                .iter()
                .zip(&names)
                .filter(|(id, _)| items.contains(id))
                .map(|(_, name)| name.as_str())
                .collect();
            named.join(", ")
        }
        Kind::List { .. } | Kind::Table { .. } => String::new(),
    }
}

/// How many decimals `number` is shown with, as the schema's generator counts them
/// (`Number::shown_places`): the fewest that hold its kept step times its factor.
fn shown_decimals(number: &apertrust_base::settings::Number) -> usize {
    number.shown_places() as usize
}

/// `n`, an `f32` of the schema, as the decimal number it was written as
/// (`settings::written`): 0.7, not the `f32`'s 0.699999988.
fn decimal(n: f32) -> f64 {
    apertrust_base::settings::written(n)
}

/// `value`, kept, as `number` shows it: times its factor, at the decimals it is shown
/// with, which keep every value it keeps apart.
fn as_shown(number: &apertrust_base::settings::Number, value: f64) -> f64 {
    let factor = decimal(number.factor.unwrap_or(1.0));
    let scale = 10f64.powi(shown_decimals(number) as i32);
    (value * factor * scale).round() / scale + 0.0
}

/// `row`'s control by its kind and the schema's `control`, `label` its full name, in the
/// row's column; the graphics card's is its list, as `extra` has it, described to screen
/// readers by `said`, what the row says under it. Returns the value the user gave it.
fn control(
    ui: &mut Ui,
    row: &Row,
    label: &str,
    extra: Extra<'_>,
    said: Option<&str>,
) -> Option<Change> {
    let ctx = ui.ctx().clone();
    let set = |value| Some(Change::Set(row.id, value));
    if let Some(card) = extra.card {
        return card_control(ui, row, card, (label, said));
    }
    match row.kind {
        Kind::Boolean => {
            let mut on = row.value.as_ref().and_then(Value::as_bool).unwrap_or(false);
            Switch::new(&mut on, label)
                .show(ui)
                .changed()
                .then_some(Value::Boolean(on))
                .and_then(set)
        }
        Kind::Choice(_) => {
            let (ids, names) = options(&ctx, row);
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            let chosen = row.value.as_ref().and_then(Value::as_str);
            let selected = ids.iter().position(|id| Some(*id) == chosen);
            let picked = if row.control == Some(Control::Segmented) {
                Segmented::new(label, &names, selected).show(ui)
            } else {
                let mut combo = Combo::new(label, &names, selected);
                if let Some(said) = said {
                    combo = combo.description(said);
                }
                combo.show(ui)
            };
            picked.and_then(|i| set(Value::String(ids[i].to_owned())))
        }
        Kind::Number(number) => {
            // One representation, the schema's decimal numbers, shown through the factor:
            // its bounds, step, `above` and the value, so 0.7 typed meets a maximum of 0.7;
            // and one rounding, the store's, as the controls' grid.
            let shown = |n: f64| as_shown(&number, n);
            let kept = row.value.as_ref().and_then(as_number).unwrap_or(0.0);
            // An open end stays the largest number there, as the controls take it.
            let open = f64::from(f32::MAX);
            let start = number.min.map_or(-open, |min| shown(decimal(min)));
            let range = start..=number.max.map_or(open, |max| shown(decimal(max)));
            let decimals = shown_decimals(&number);
            let kept_decimals = number.decimals.unwrap_or(2) as i32;
            let grid = shown(10f64.powi(-kept_decimals));
            // With no step of its own, one kept step: the next value the store keeps.
            let step = number.step.map_or(grid, |step| shown(decimal(step)));
            let unit = number.unit.map(|unit| tr(&ctx, &format!("unit-{unit}")));
            let slider = number.min.is_some()
                && number.max.is_some()
                && row.control != Some(Control::Number);
            let typed = if slider {
                let mut slider = Slider::new(row.id, label, shown(kept), range)
                    .step(step)
                    .grid(grid)
                    .decimals(decimals);
                if let Some(unit) = &unit {
                    slider = slider.unit(unit);
                }
                slider.show(ui)
            } else {
                let mut field = Number::new(row.id, label, shown(kept), range)
                    .step(step)
                    .grid(grid)
                    .decimals(decimals);
                if let Some(unit) = &unit {
                    field = field.unit(unit);
                }
                // What it must be more than, which no range holds.
                if let Some(above) = number.above {
                    field = field.above(shown(decimal(above)));
                }
                field.show(ui)
            };
            // On the grid, its ends too (the schema's generator refuses an end that is not),
            // so this is exact but for the float's last digits.
            let factor = decimal(number.factor.unwrap_or(1.0));
            let scale = 10f64.powi(kept_decimals);
            let kept = (typed? / factor * scale).round() / scale;
            set(if kept_decimals == 0 {
                Value::Integer(kept as i64)
            } else {
                Value::Float(kept)
            })
        }
        Kind::Text => {
            let text = row.value.as_ref().and_then(Value::as_str).unwrap_or("");
            match row.control {
                Some(Control::Area) => TextArea::new(row.id, label, text).show(ui),
                // No setting is a folder yet: the app's folder dialog comes with the first,
                // and with it what the button asks of it.
                Some(Control::Folder) => {
                    let choose = tr(&ctx, "export-choose-folder");
                    Folder::new(row.id, label, text, &choose).show(ui).text
                }
                _ => {
                    // What it shows while empty, where its words say: `setting-<id>-hint`.
                    let words = translator(&ctx);
                    let hint = format!("setting-{}-hint", row.words());
                    let hint = words.has(&hint).then(|| words.text(&hint));
                    let mut field = Text::new(row.id, label, text);
                    if let Some(hint) = &hint {
                        field = field.hint(hint);
                    }
                    field.show(ui)
                }
            }
            .and_then(|typed| set(Value::String(typed)))
        }
        Kind::List {
            of: Kind::Choice(_),
            ..
        } => {
            let ticked: Vec<&str> = row
                .value
                .as_ref()
                .and_then(Value::as_array)
                .map(|items| items.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            let (ids, names) = options(&ctx, row);
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            let on: Vec<bool> = ids.iter().map(|id| ticked.contains(id)).collect();
            let (changed, now) = Chips::new(label, &names, &on).show(ui)?;
            // The options on after the change, in the type's order.
            let items = ids
                .iter()
                .enumerate()
                .filter(|(i, _)| if *i == changed { now } else { on[*i] })
                .map(|(_, id)| Value::String((*id).to_owned()));
            set(Value::Array(items.collect()))
        }
        // No other list, and no table, is on a page yet.
        Kind::List { .. } | Kind::Table { .. } => None,
    }
}

/// The graphics card's list (decision 82), `named` its row's full name and what is said
/// under it, which screen readers hear on the box: Automatic, the cards, a card not found,
/// then CPU after a separator, as `card` has them; disabled while they are listed. Choosing
/// Automatic resets the setting, so it leaves the file.
fn card_control(
    ui: &mut Ui,
    row: &Row,
    card: &hardware::Card,
    named: (&str, Option<&str>),
) -> Option<Change> {
    let (label, said) = named;
    let names: Vec<&str> = card.words.iter().map(String::as_str).collect();
    let missing: Vec<usize> = card.missing.iter().map(|(at, _)| *at).collect();
    let missing_tip = card.missing.as_ref().map_or("", |(_, tip)| tip.as_str());
    let mut combo = Combo::new(label, &names, card.selected)
        .enabled(!card.listing)
        .disabled_options(&missing, missing_tip);
    if let Some((at, tip)) = &card.unlisted {
        combo = combo.unlisted(*at, tip);
    }
    if let Some(at) = card.separator {
        combo = combo.separator_before(at);
    }
    if let Some(said) = said {
        combo = combo.description(said);
    }
    let item = combo.show(ui)?;
    Some(match card.stores(item) {
        Some(stored) => Change::Set(row.id, Value::String(stored.to_owned())),
        None => Change::Reset(row.id),
    })
}

/// A number as `settings.toml` keeps it, whole or not.
fn as_number(value: &Value) -> Option<f64> {
    value
        .as_float()
        .or_else(|| value.as_integer().map(|i| i as f64))
}

/// A choice's options, the ids and their words: the language's each in its own language,
/// those this build offers; any other's its messages, `setting-<id>-option-<option>`.
fn options(ctx: &egui::Context, row: &Row) -> (Vec<&'static str>, Vec<String>) {
    if row.id == settings::registry::LOCALE.id {
        let offered = row
            .options
            .iter()
            .filter_map(|id| Locale::from_id(id).map(|locale| (*id, locale)))
            .filter(|(_, locale)| crate::offered(*locale, cfg!(debug_assertions)));
        return offered
            .map(|(id, locale)| (id, locale.native_name().to_owned()))
            .unzip();
    }
    let names = row
        .options
        .iter()
        .map(|option| tr(ctx, &format!("setting-{}-option-{option}", row.words())))
        .collect();
    (row.options.clone(), names)
}

#[cfg(test)]
mod tests {
    use apertrust_ui::Translator;
    use eframe::egui::accesskit::Toggled;
    use eframe::egui::{Event, PointerButton};
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT as _, Queryable as _};

    use super::*;
    use crate::dialog_host;

    /// Each page has an icon of its own, drawn filled on the page shown as the Tools panel's
    /// tabs are: every page but Presets, whose lines have no filled form.
    #[test]
    fn each_page_has_its_own_icon_and_its_filled_form() {
        let icons: Vec<Icon> = Page::ALL.into_iter().map(page_icon).collect();
        let distinct: std::collections::BTreeSet<&str> = icons.iter().map(|i| i.name()).collect();
        assert_eq!(distinct.len(), Page::ALL.len());
        for (page, icon) in Page::ALL.into_iter().zip(icons) {
            let filled = icon.filled() != icon;
            assert_eq!(filled, page != Page::Presets, "{page:?}");
        }
    }

    /// The dialog as the host draws it in its window, and what each frame asked: every
    /// change, and those of each frame that asked any.
    struct Bench {
        content: Settings,
        changes: Vec<Change>,
        frames: Vec<Vec<Change>>,
        /// How many times Restart now was pressed.
        restarts: usize,
        /// How many frames asked the host to close the dialog.
        closes: usize,
        /// Every change asked for by the frame that first asked to close, that one's included:
        /// what the app takes before the window goes.
        closed_with: Option<Vec<Change>>,
    }

    /// The dialog over `keeper`'s settings, without Close as on the Mac, in `which` theme;
    /// the language's row reads British English, whatever the system's.
    fn dialog_of(keeper: &Keeper, which: Builtin) -> Harness<'static, Bench> {
        dialog_in(keeper, which, Locale::EnGb)
    }

    /// The dialog as [`dialog_of`] draws it, in `locale`.
    fn dialog_in(keeper: &Keeper, which: Builtin, locale: Locale) -> Harness<'static, Bench> {
        dialog_on(keeper, which, locale, OperatingSystem::Mac)
    }

    /// The dialog as [`dialog_in`] draws it, on `os`, its graphics cards those of a
    /// computer like the test laptop (`hardware::tests::running`).
    fn dialog_on(
        keeper: &Keeper,
        which: Builtin,
        locale: Locale,
        os: OperatingSystem,
    ) -> Harness<'static, Bench> {
        let running = crate::hardware::tests::running(os);
        dialog_with(keeper, which, locale, os, running)
    }

    /// The dialog as [`dialog_on`] draws it, handed `running`.
    fn dialog_with(
        keeper: &Keeper,
        which: Builtin,
        locale: Locale,
        os: OperatingSystem,
        running: hardware::Running,
    ) -> Harness<'static, Bench> {
        let mut content = Settings::new(keeper, false).with_hardware(running);
        for row in content.rows.iter_mut().filter(|row| row.id == "locale") {
            row.default = Some("en-GB".into());
            if !row.overridden {
                row.value = Some("en-GB".into());
            }
        }
        let mut harness = Harness::builder()
            .with_size(vec2(WIDTH, BODY) + vec2(16.0, 104.0))
            .with_os(os)
            // Each event is a frame of its own: a 60th of a second apart, two clicks are one
            // double-click.
            .with_step_dt(1.0 / 60.0)
            .with_context_setup(move |ctx| {
                apertrust_ui::install(ctx, Theme::builtin(which));
                apertrust_ui::set_locale(ctx, locale);
                ctx.global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
            })
            .build_ui_state(
                |ui, bench: &mut Bench| {
                    let drawn = dialog_host::draw(ui, &mut bench.content);
                    bench.closes += usize::from(drawn.close);
                    let mut frame = Vec::new();
                    for output in drawn.outputs {
                        match output {
                            Output::Settings(change) => frame.push(change),
                            Output::Restart => bench.restarts += 1,
                            _ => {}
                        }
                    }
                    bench.changes.extend(frame.iter().cloned());
                    if !frame.is_empty() {
                        bench.frames.push(frame);
                    }
                    if drawn.close && bench.closed_with.is_none() {
                        bench.closed_with = Some(bench.changes.clone());
                    }
                },
                Bench {
                    content,
                    changes: Vec::new(),
                    frames: Vec::new(),
                    restarts: 0,
                    closes: 0,
                    closed_with: None,
                },
            );
        harness.run();
        harness
    }

    /// The app's changes handed to the dialog, as the start notice's Keep's, change the rows
    /// they name, but for one `--conf` sets, which keeps its value, as the store does.
    #[test]
    fn the_apps_changes_leave_a_row_set_by_conf() {
        use apertrust_engine::settings::registry::{ENGINE_ADAPTER, ENGINE_BACKEND};
        let conf = ["engine.adapter=missing".to_owned()];
        let (keeper, _) = Keeper::open_in(None, &settings::overrides(&conf).expect("read"));
        let mut dialog = Settings::new(&keeper, false);
        let running = crate::hardware::tests::running(OperatingSystem::Windows);
        dialog.set_hardware(running.clone(), &[]);
        let value = |dialog: &Settings, id: &str| {
            let row = dialog.rows.iter().find(|row| row.id == id).expect("a row");
            row.value.clone()
        };
        let missing = Some(Value::String("missing".into()));
        assert_eq!(value(&dialog, ENGINE_ADAPTER.id), missing);
        let keep = [
            Change::Reset(ENGINE_ADAPTER.id),
            Change::Set(ENGINE_BACKEND.id, Value::String("dx12".into())),
        ];
        assert!(dialog.set_hardware(running, &keep));
        assert_eq!(value(&dialog, ENGINE_ADAPTER.id), missing);
        let dx12 = Some(Value::String("dx12".into()));
        assert_eq!(value(&dialog, ENGINE_BACKEND.id), dx12);
    }

    /// Shows Processing, its graphics cards listed, and forgets the page's change.
    fn processing(harness: &mut Harness<'_, Bench>) {
        harness.get_by_label("Processing").click();
        harness.run();
        listed(harness);
        harness.state_mut().changes.clear();
        harness.state_mut().frames.clear();
    }

    /// Lets the listings under way answer, and the dialog take them.
    fn listed(harness: &mut Harness<'_, Bench>) {
        harness.state().content.hardware.wait();
        harness.run();
    }

    /// The keeper's settings with `id` set to `value`.
    fn keeper_with(id: &str, value: &str) -> Keeper {
        let (mut keeper, settings) = Keeper::nowhere();
        keeper.apply(&settings, id, Some(&value.into()));
        keeper
    }

    fn dialog() -> Harness<'static, Bench> {
        dialog_of(&Keeper::nowhere().0, Builtin::Dark)
    }

    fn press(harness: &mut Harness<'_, Bench>, pos: egui::Pos2, button: PointerButton) {
        for pressed in [true, false] {
            harness.event(Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers: Modifiers::NONE,
            });
        }
    }

    /// Two clicks at `pos`, after a pause that keeps them from an earlier click.
    fn double_click(harness: &mut Harness<'_, Bench>, pos: egui::Pos2) {
        harness.run_steps(30);
        harness.hover_at(pos);
        press(harness, pos, PointerButton::Primary);
        press(harness, pos, PointerButton::Primary);
        harness.run();
    }

    fn right_click(harness: &mut Harness<'_, Bench>, pos: egui::Pos2) {
        harness.run_steps(30);
        harness.hover_at(pos);
        press(harness, pos, PointerButton::Secondary);
        harness.run();
    }

    /// Whether the row labelled `label` says it is off its default.
    fn dotted(harness: &Harness<'_, Bench>, label: &str) -> bool {
        let node = harness.get_by_role_and_label(Role::Label, label);
        node.accesskit_node().description().as_deref() == Some("Changed from its default")
    }

    #[test]
    fn every_page_is_listed_and_those_with_nothing_yet_are_dimmed() {
        let mut harness = dialog();
        let pages: Vec<(String, bool, bool)> = harness
            .get_all_by_role(Role::Tab)
            .map(|node| {
                let node = node.accesskit_node();
                let name = node.label().unwrap_or_default();
                let shown = node.toggled() == Some(Toggled::True);
                (name, node.is_disabled(), shown)
            })
            .collect();
        let expected = [
            ("General", false, true),
            ("Import", true, false),
            ("Photos", false, false),
            ("Editing", false, false),
            ("Processing", false, false),
            ("Storage", false, false),
            ("Shortcuts", true, false),
            ("Presets", true, false),
            ("AI", true, false),
        ]
        .map(|(name, dimmed, shown)| (name.to_owned(), dimmed, shown));
        assert_eq!(pages, expected);
        // A dimmed page says it comes later, and cannot be chosen.
        harness.get_by_label("Import").hover();
        harness.run();
        harness.get_by_label("Coming in a later version");
        harness.get_by_label("Import").click();
        harness.run();
        assert_eq!(harness.state().content.page, Page::General);
        assert!(harness.state().changes.is_empty());
    }

    /// General's rows in the schema's order, under their groups, and a chosen page kept.
    #[test]
    fn a_page_shows_its_groups_and_rows_and_the_page_chosen_is_kept() {
        let mut harness = dialog();
        let general = [
            "Interface",
            "Theme",
            "Language",
            "Interface size",
            "Confirmations",
            "Ask before overwriting an exported file",
        ];
        let names: Vec<String> = harness
            .get_all_by_role(Role::Label)
            .filter_map(|node| node.accesskit_node().value())
            .filter(|name| general.contains(&name.as_str()))
            .collect();
        assert_eq!(names, general);
        assert!(harness.query_by_label("Square thumbnails").is_none());
        harness.get_by_label("Photos").click();
        harness.run();
        assert_eq!(
            harness.state().changes,
            [Change::Set("settings_window.page", "photos".into())]
        );
        harness.get_by_role_and_label(Role::Switch, "Square thumbnails");
        harness.get_by_role_and_label(Role::Label, "Thumbnails");
        // ↓ moves to the next page that holds rows: Editing.
        harness.get_by_label("Photos").focus();
        harness.run();
        harness.key_press(Key::ArrowDown);
        harness.run();
        assert_eq!(harness.state().content.page, Page::Editing);
        harness.get_by_role_and_label(Role::Switch, "Show tab names");
        // And the dialog opens on the page kept, or General where it holds nothing.
        let (mut keeper, settings) = Keeper::nowhere();
        keeper.apply(
            &settings,
            "settings_window.page",
            Some(&"processing".into()),
        );
        assert_eq!(Settings::new(&keeper, false).page, Page::Processing);
        keeper.apply(&settings, "settings_window.page", Some(&"ai".into()));
        assert_eq!(Settings::new(&keeper, false).page, Page::General);
    }

    /// The Photos page's old id, `library`, which earlier builds kept, is a value this build
    /// cannot read: the dialog opens on General, and the value stays in the file until a
    /// page is chosen.
    #[test]
    fn a_page_kept_as_library_opens_general_and_stays_kept() {
        let folder = settings::testing::Folder::new("library-page");
        let text = "version = 1\n\n[settings_window]\npage = \"library\"\n";
        std::fs::write(folder.settings(), text).expect("written");
        let (mut keeper, settings) = Keeper::open(folder.storage());
        assert_eq!(Settings::new(&keeper, false).page, Page::General);
        // Another setting written leaves it as it was.
        keeper.apply(&settings, "browser.square_thumbnails", Some(&true.into()));
        keeper.flush();
        assert!(
            folder.text().contains("page = \"library\""),
            "{}",
            folder.text()
        );
        assert!(
            folder.text().contains("square_thumbnails = true"),
            "the other setting reached the file: {}",
            folder.text()
        );
    }

    /// A language chosen is reset to the system's, whichever that is: the mark names that,
    /// never the language the system gives now; an explicit default keeps its option's name.
    #[test]
    fn a_computed_default_is_named_by_its_own_words() {
        let mut harness = dialog();
        let content = &mut harness.state_mut().content;
        let row = content.rows.iter_mut().find(|row| row.id == "locale");
        row.expect("the language").value = Some("uk".into());
        harness.run();
        assert!(dotted(&harness, "Language"));
        harness.get_by_role_and_label(Role::Button, "Reset to the system's language");
        assert!(
            harness
                .query_by_role_and_label(Role::Button, "Reset to English (UK)")
                .is_none()
        );
        let mut harness = dialog_of(&keeper_with("theme", "light"), Builtin::Dark);
        harness.run();
        harness.get_by_role_and_label(Role::Button, "Reset to Dark");
    }

    /// The changed mark after the label of a row off its default, a button named and tipped
    /// by the default, which resets it; a double-click on the label, Reset in its
    /// right-click menu and Delete on it reset too.
    #[test]
    fn a_changed_setting_shows_its_mark_and_resets_from_it() {
        let mut harness = dialog();
        let ask = "Ask before overwriting an exported file";
        let reset_to_on = "Reset to On";
        assert!(!dotted(&harness, ask));
        assert!(
            harness
                .query_by_role_and_label(Role::Button, reset_to_on)
                .is_none()
        );
        harness.get_by_role_and_label(Role::Switch, ask).click();
        harness.run();
        assert_eq!(
            harness.state().changes,
            [Change::Set("export.ask_before_overwrite", false.into())]
        );
        assert!(dotted(&harness, ask));
        // The mark after the label, on its line; its tooltip names the default.
        let label = harness.get_by_role_and_label(Role::Label, ask).rect();
        let mark = harness
            .get_by_role_and_label(Role::Button, reset_to_on)
            .rect();
        // The switch's label is its whole row's target, so the row's rectangle.
        assert!(mark.left() > label.left() + 100.0);
        assert!((mark.center().y - label.center().y).abs() < 1.0);
        harness.run_steps(30);
        harness
            .get_by_role_and_label(Role::Button, reset_to_on)
            .hover();
        harness.run_steps(10);
        assert!(harness.query_all_by_label(reset_to_on).count() >= 2);
        press(&mut harness, mark.center(), PointerButton::Primary);
        harness.run();
        assert_eq!(
            harness.state().changes.last(),
            Some(&Change::Reset("export.ask_before_overwrite"))
        );
        assert!(!dotted(&harness, ask));
        assert!(
            harness
                .query_by_role_and_label(Role::Button, reset_to_on)
                .is_none()
        );
        // The interface's size, typed in per cent, is kept as a factor; its mark names the
        // default with its unit, and the label's menu resets it.
        let (size, field) = ("Interface size", "Interface size, in %");
        let track = harness.get_by_role_and_label(Role::Slider, size).rect();
        let value = pos2(track.right() + 30.0, track.center().y);
        harness.run_steps(30);
        press(&mut harness, value, PointerButton::Primary);
        harness.run();
        harness
            .get_by_role_and_label(Role::TextInput, field)
            .type_text("125");
        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(
            harness.state().changes.last(),
            Some(&Change::Set("scale", 1.25.into()))
        );
        assert!(dotted(&harness, size));
        harness.get_by_role_and_label(Role::Button, "Reset to 100 %");
        let at = harness
            .get_by_role_and_label(Role::Label, size)
            .rect()
            .center();
        right_click(&mut harness, at);
        harness.get_by_label("Reset").click();
        harness.run();
        assert_eq!(
            harness.state().changes.last(),
            Some(&Change::Reset("scale"))
        );
        assert!(!dotted(&harness, size));
        // A double-click on the label resets a choice, and so does Delete on it. The theme
        // shown changes with it, the window's areas fading to it.
        let theme = |harness: &mut Harness<'_, Bench>, item: &str| {
            harness
                .get_by_role_and_label(Role::ComboBox, "Theme")
                .click();
            harness.run_steps(12);
            harness.get_by_label(item).click();
            harness.run_steps(12);
        };
        theme(&mut harness, "Light");
        assert!(dotted(&harness, "Theme"));
        harness.get_by_role_and_label(Role::Button, "Reset to Dark");
        let at = harness
            .get_by_role_and_label(Role::Label, "Theme")
            .rect()
            .center();
        harness.run_steps(30);
        harness.hover_at(at);
        press(&mut harness, at, PointerButton::Primary);
        press(&mut harness, at, PointerButton::Primary);
        harness.run_steps(12);
        assert_eq!(
            harness.state().changes.last(),
            Some(&Change::Reset("theme"))
        );
        theme(&mut harness, "Grey");
        assert_eq!(
            harness.state().changes.last(),
            Some(&Change::Set("theme", "grey".into()))
        );
        let before = harness.state().changes.len();
        harness.get_by_role_and_label(Role::Label, "Theme").focus();
        harness.run();
        harness.key_press(Key::Delete);
        harness.run_steps(12);
        assert_eq!(harness.state().changes[before..], [Change::Reset("theme")]);
    }

    /// A value typed and left by a click on a switch: the field's commit, as it loses the
    /// focus, and the switch's toggle come in one frame, and both are kept.
    #[test]
    fn a_commit_and_a_switch_in_one_frame_are_both_kept() {
        let mut harness = dialog();
        let track = harness
            .get_by_role_and_label(Role::Slider, "Interface size")
            .rect();
        let value = pos2(track.right() + 30.0, track.center().y);
        harness.run_steps(30);
        press(&mut harness, value, PointerButton::Primary);
        harness.run();
        harness
            .get_by_role_and_label(Role::TextInput, "Interface size, in %")
            .type_text("125");
        harness.run();
        harness.state_mut().frames.clear();
        let switch = harness
            .get_by_role_and_label(Role::Switch, "Keep a log of this session")
            .rect()
            .center();
        harness.hover_at(switch);
        press(&mut harness, switch, PointerButton::Primary);
        harness.run();
        let both = [
            Change::Set("scale", 1.25.into()),
            Change::Set("log.keep", true.into()),
        ];
        let frame = harness
            .state()
            .frames
            .iter()
            .find(|frame| frame.len() > 1)
            .cloned();
        assert_eq!(
            frame.as_deref(),
            Some(&both[..]),
            "{:?}",
            harness.state().frames
        );
    }

    /// The point of `name`'s row's words at `y` of its height, and of its switch.
    fn words_and_switch(
        harness: &Harness<'_, Bench>,
        name: &str,
        y: f32,
    ) -> (egui::Pos2, egui::Pos2) {
        let words = harness.get_by_role_and_label(Role::Label, name).rect();
        let switch = harness.get_by_role_and_label(Role::Switch, name).rect();
        let at = pos2(words.left() + 20.0, words.top() + y * words.height());
        (at, switch.center())
    }

    /// A switch's row takes each click at once, as darktable's label does: one on its
    /// words or its switch toggles it; a double-click on its words resets it, a stored value
    /// on its default's other side going back with no stray value kept, and at its default
    /// it ends there too; quick clicks on two rows toggle each, resetting neither. Each case:
    /// where the clicks land, the setting before, and after with whether it was reset.
    #[test]
    fn a_switch_s_row_takes_each_click() {
        const KEEP: &str = "Keep a log of this session";
        const ASK: [&str; 2] = [
            "Ask before overwriting an exported file",
            "Ask before discarding a photo’s history",
        ];
        let keep = |on: bool| {
            let (mut keeper, settings) = Keeper::nowhere();
            keeper.apply(&settings, "log.keep", Some(&on.into()));
            keeper
        };
        // Where each click lands: Keep a log's words or switch, or the confirmations' words
        // at the first row's bottom and the second's top, near enough for a double-click.
        #[derive(Clone, Copy, Debug)]
        enum At {
            Words,
            Switch,
            Low,
            High,
        }
        let cases = [
            (&[At::Words][..], false, vec![("log.keep", true, false)]),
            (&[At::Switch][..], false, vec![("log.keep", true, false)]),
            (
                &[At::Words, At::Words][..],
                true,
                vec![("log.keep", false, true)],
            ),
            (
                &[At::Words, At::Words][..],
                false,
                vec![("log.keep", false, true)],
            ),
            (
                &[At::Low, At::High][..],
                false,
                vec![
                    ("export.ask_before_overwrite", false, false),
                    ("history.ask_before_discard", false, false),
                ],
            ),
        ];
        for (clicks, before, after) in cases {
            let mut harness = dialog_of(&keep(before), Builtin::Dark);
            harness.run_steps(30);
            let (words, switch) = words_and_switch(&harness, KEEP, 0.5);
            let (low, _) = words_and_switch(&harness, ASK[0], 1.0);
            let (high, _) = words_and_switch(&harness, ASK[1], 0.0);
            for at in clicks {
                let pos = match at {
                    At::Words => words,
                    At::Switch => switch,
                    At::Low => low - vec2(0.0, 0.5),
                    At::High => high + vec2(0.0, 0.5),
                };
                harness.hover_at(pos);
                press(&mut harness, pos, PointerButton::Primary);
                harness.step();
            }
            harness.run();
            let state = harness.state();
            for (id, on, reset) in &after {
                let row = state.content.row(id).expect("the row");
                let stored = row.value.as_ref().and_then(Value::as_bool);
                let was_reset = state.changes.contains(&Change::Reset(id));
                let said = format!("{clicks:?} from {before}: {:?}", state.changes);
                assert_eq!((stored, was_reset), (Some(*on), *reset), "{id}, {said}");
            }
            // A double-click ends with the key removed, its last change the reset.
            if clicks.len() == 2 && matches!(clicks[0], At::Words) {
                let last = state.changes.last();
                assert_eq!(last, Some(&Change::Reset("log.keep")), "{before}");
            }
        }
    }

    /// The log's categories are chips, a toggle button each in the type's order, keeping the
    /// setting's values: a change sets the whole list, one mark says the row is off its
    /// default, one Reset puts it all back, and the row's help is its tooltip.
    #[test]
    fn the_log_categories_are_chips() {
        let mut harness = dialog();
        let options = [
            ("GPU", true),
            ("Raw", true),
            ("Pipe", true),
            ("Performance", true),
            ("Memory", false),
            ("Settings", true),
            ("Control", true),
            ("Events", true),
            ("Verbose", false),
        ];
        harness.get_by_role_and_label(Role::Group, "Log");
        let label = harness.get_by_role_and_label(Role::Label, "Log").rect();
        let mut before: Option<egui::Rect> = None;
        for (name, ticked) in options {
            let node = harness.get_by_role_and_label(Role::Button, name);
            let on = node.accesskit_node().toggled() == Some(Toggled::True);
            assert_eq!(on, ticked, "{name}");
            let rect = node.rect();
            assert!(rect.left() > label.right(), "{name} is in the column");
            if let Some(before) = before {
                let after = rect.top() > before.top() || rect.left() > before.right();
                assert!(after, "{name} comes after the one before");
            }
            before = Some(rect);
        }
        // The label stays on the row's first line, level with the first chips.
        let first = harness.get_by_role_and_label(Role::Button, "GPU").rect();
        assert!((first.center().y - label.center().y).abs() < 3.0);
        assert!(!dotted(&harness, "Log"));
        harness
            .get_by_role_and_label(Role::Button, "Memory")
            .click();
        harness.run();
        let ids = |ids: &[&str]| {
            let items = ids.iter().map(|id| Value::String((*id).to_owned()));
            Value::Array(items.collect())
        };
        let with_memory = [
            "gpu", "raw", "pipe", "perf", "memory", "settings", "control", "event",
        ];
        assert_eq!(
            harness.state().changes,
            [Change::Set("log.domains", ids(&with_memory))]
        );
        assert!(dotted(&harness, "Log"));
        harness.get_by_role_and_label(Role::Button, "GPU").click();
        harness.run();
        assert_eq!(
            harness.state().changes.last(),
            Some(&Change::Set("log.domains", ids(&with_memory[1..])))
        );
        // One Reset for the whole list.
        let at = label.center();
        right_click(&mut harness, at);
        harness.get_by_label("Reset").click();
        harness.run();
        assert_eq!(
            harness.state().changes.last(),
            Some(&Change::Reset("log.domains"))
        );
        assert!(!dotted(&harness, "Log"));
        let gpu = harness.get_by_role_and_label(Role::Button, "GPU");
        assert_eq!(gpu.accesskit_node().toggled(), Some(Toggled::True));
        // The help, in the row's tooltip.
        harness.run_steps(30);
        harness.get_by_role_and_label(Role::Label, "Log").hover();
        harness.run_steps(10);
        let help = "What the log keeps besides warnings and errors, which it always keeps. \
                    Verbose adds much more, for a developer.";
        let shown: Vec<String> = harness
            .get_all_by_role(Role::Label)
            .filter_map(|node| {
                node.accesskit_node()
                    .label()
                    .or(node.accesskit_node().value())
            })
            .collect();
        assert!(shown.iter().any(|text| text == help), "{shown:?}");
    }

    /// A row `--conf` sets is dimmed, its control disabled, with no mark though it is off
    /// its default; its label says why and resets nothing. A setting that applies after a
    /// restart says so after its label.
    #[test]
    fn a_conf_set_row_is_dimmed_with_no_mark() {
        let conf = ["scale=1.5".to_owned()];
        let (keeper, _) = Keeper::open_in(None, &settings::overrides(&conf).expect("read"));
        let mut harness = dialog_of(&keeper, Builtin::Dark);
        let slider = harness.get_by_role_and_label(Role::Slider, "Interface size");
        assert!(slider.accesskit_node().is_disabled());
        assert_eq!(slider.accesskit_node().numeric_value(), Some(150.0));
        assert!(harness.query_by_label("Reset to 100 %").is_none());
        let label = harness.get_by_role_and_label(Role::Label, "Interface size");
        assert!(label.accesskit_node().is_disabled());
        let said = label.accesskit_node().description();
        assert_eq!(said.as_deref(), Some("Set by --conf for this run"));
        let at = label.rect().center();
        double_click(&mut harness, at);
        assert!(harness.state().changes.is_empty());
        processing(&mut harness);
        harness.get_by_role_and_label(Role::ComboBox, "Graphics card");
        harness.get_by_role_and_label(Role::TextInput, "Memory budget, in MiB");
        let notes = harness.query_all_by_label("after a restart").count();
        assert_eq!(notes, 2);
    }

    /// The graphics API is drawn on Windows alone, the only platform with a choice of one
    /// (`platforms`; decision 82): Processing's Hardware acceleration group holds Graphics
    /// card, Graphics API and Memory budget there, and the API's row elsewhere is absent.
    #[test]
    fn the_graphics_api_is_drawn_on_windows_alone() {
        let keeper = Keeper::nowhere().0;
        for os in [
            OperatingSystem::Windows,
            OperatingSystem::Mac,
            OperatingSystem::Nix,
        ] {
            let mut harness = dialog_on(&keeper, Builtin::Dark, Locale::EnGb, os);
            processing(&mut harness);
            harness.get_by_role_and_label(Role::Label, "Hardware acceleration");
            assert!(harness.query_by_label("GPU").is_none(), "{os:?}");
            let windows = os == OperatingSystem::Windows;
            let mut rows = vec!["Graphics card", "Graphics API", "Memory budget"];
            if !windows {
                rows.remove(1);
            }
            let labels: Vec<String> = harness
                .get_all_by_role(Role::Label)
                .filter_map(|node| node.accesskit_node().value())
                .filter(|name| name.starts_with("Graphics") || name == "Memory budget")
                .collect();
            assert_eq!(labels, rows, "{os:?}");
            let api = harness.query_by_role_and_label(Role::ComboBox, "Graphics API");
            assert_eq!(api.is_some(), windows, "{os:?}");
            // Each row says it applies after a restart.
            let notes = harness.query_all_by_label("after a restart").count();
            assert_eq!(notes, rows.len(), "{os:?}");
        }
        // Its options, in the setting's order, and Automatic chosen.
        let row = Settings::new(&keeper, false);
        let row = row.row("engine.backend").expect("a row");
        assert_eq!(row.options, ["auto", "dx12", "vulkan"]);
        assert_eq!(row.value, Some("auto".into()));
        let words = Translator::new(Locale::EnGb);
        let names: Vec<String> = row
            .options
            .iter()
            .map(|option| words.text(&format!("setting-engine-backend-option-{option}")))
            .collect();
        assert_eq!(names, ["Automatic", "Direct3D 12", "Vulkan"]);
        // A page whose only rows are another platform's holds nothing there.
        let mut alone = Settings::new(&keeper, false);
        alone.rows.retain(|row| row.id == "engine.backend");
        assert!(alone.holds(Page::Processing, OperatingSystem::Windows));
        assert!(!alone.holds(Page::Processing, OperatingSystem::Mac));
        assert!(!alone.holds(Page::Processing, OperatingSystem::Nix));
    }

    /// A row is dimmed while the boolean it requires is off, saying so, and enabled again
    /// once it is on: the cache's limit while the cache is unlimited.
    #[test]
    fn a_row_is_disabled_while_the_boolean_it_requires_is_off() {
        let mut harness = dialog();
        harness.get_by_label("Storage").click();
        harness.run();
        let disabled = |harness: &Harness<'_, Bench>| {
            let field = harness.get_by_role_and_label(Role::TextInput, "Cache limit, in GB");
            field.accesskit_node().is_disabled()
        };
        assert!(!disabled(&harness));
        let limited = "Limit the cache's size";
        harness.get_by_role_and_label(Role::Switch, limited).click();
        harness.run();
        let set = Change::Set("cache.limited", Value::Boolean(false));
        assert_eq!(harness.state().changes.last(), Some(&set));
        assert!(disabled(&harness));
        let label = harness.get_by_role_and_label(Role::Label, "Cache limit");
        let said = label.accesskit_node().description();
        assert_eq!(
            said.as_deref(),
            Some("Off while “Limit the cache's size” is off")
        );
        harness.get_by_role_and_label(Role::Switch, limited).click();
        harness.run();
        assert!(!disabled(&harness));
    }

    /// Every row's words, its page's and its group's, and a choice's options but the
    /// language's, are in each language.
    #[test]
    fn every_row_s_words_are_in_every_language() {
        let rows = Settings::new(&Keeper::nowhere().0, false).rows;
        for locale in [Locale::EnGb, Locale::EnUs, Locale::Uk, Locale::Pl] {
            let words = Translator::new(locale);
            for page in Page::ALL {
                let id = format!("settings-page-{}", page.id());
                assert!(words.has(&id), "{locale:?}: {id}");
            }
            for row in &rows {
                let mut ids = vec![
                    format!("setting-{}-label", row.words()),
                    format!("settings-group-{}", row.group.replace('_', "-")),
                ];
                if row.id != "locale" {
                    let options = row.options.iter();
                    ids.extend(options.map(|o| format!("setting-{}-option-{o}", row.words())));
                }
                // A choice whose default the app works out names it in words of its own.
                if row.computed && matches!(row.kind, Kind::Choice(_)) {
                    ids.push(format!("setting-{}-default", row.words()));
                }
                if let Kind::Number(number) = row.kind {
                    ids.extend(number.unit.map(|unit| format!("unit-{unit}")));
                }
                for id in ids {
                    assert!(words.has(&id), "{locale:?}: {id}");
                }
            }
        }
    }

    /// General draws each kind with its control, where the schema says: Theme and Language
    /// combos, Interface size a slider, Slider marker and Menu button segmented, the
    /// confirmations switches, and the log's categories chips; on Linux, where the menu
    /// button is drawn. Every box ends at one edge, the column's, and the boxes and wells
    /// fill its 260 points; the labels start together at the rows' start.
    #[test]
    fn the_general_page_draws_each_kind_with_its_control() {
        let keeper = Keeper::nowhere().0;
        let harness = dialog_on(&keeper, Builtin::Dark, Locale::EnGb, OperatingSystem::Nix);
        let rect = |role, name| harness.get_by_role_and_label(role, name).rect();
        let theme = rect(Role::ComboBox, "Theme");
        let wide = [
            theme,
            rect(Role::ComboBox, "Language"),
            rect(Role::RadioGroup, "Slider marker"),
            rect(Role::RadioGroup, "Menu button"),
            rect(Role::Group, "Log"),
        ];
        for box_ in wide {
            assert_eq!(box_.width(), 260.0, "{box_:?}");
            assert_eq!(box_.right(), theme.right(), "{box_:?}");
        }
        for name in ["Theme", "Language"] {
            assert_eq!(rect(Role::ComboBox, name).height(), 24.0);
        }
        // Slider marker's three words, Menu button's two options.
        for name in ["Triangle", "Circle", "Square", "Apertrust icon", "Menu"] {
            harness.get_by_role_and_label(Role::RadioButton, name);
        }
        let slider = rect(Role::Slider, "Interface size");
        assert!(slider.right() < theme.right() && slider.left() >= theme.left());
        for name in [
            "Reopen what was open last",
            "Ask before overwriting an exported file",
            "Ask before discarding a photo’s history",
            "Keep a log of this session",
        ] {
            let switch = rect(Role::Switch, name);
            assert_eq!(switch.right(), theme.right(), "{name}");
        }
        // The labels start together, before the column.
        let starts: Vec<f32> = ["Theme", "Interface size", "Slider marker", "Log"]
            .iter()
            .map(|name| rect(Role::Label, name).left())
            .collect();
        assert!(starts.windows(2).all(|w| w[0] == w[1]), "{starts:?}");
        assert!(starts[0] < theme.left() - 200.0);
        // A right-click past a label, before its control, opens nothing.
        let mut harness = dialog();
        let label = harness.get_by_role_and_label(Role::Label, "Theme").rect();
        right_click(&mut harness, pos2(label.right() + 40.0, label.center().y));
        assert!(harness.query_by_label("Reset").is_none());
    }

    /// Whether the label `label` is drawn underlined, as a row Find a setting matches.
    fn underlined(harness: &Harness<'_, Bench>, label: &str) -> bool {
        harness
            .output()
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.galley.job.text == label => Some(
                    text.galley
                        .job
                        .sections
                        .iter()
                        .any(|section| section.format.underline.width > 0.0),
                ),
                _ => None,
            })
            .next()
            .expect("the label drawn")
    }

    /// Find a setting, which Ctrl+F (Cmd+F) focuses, finds rows by their labels and their
    /// help, case and accents aside: the shown page's matching labels are underlined, and a
    /// page with no match is dimmed but chosen still.
    #[test]
    fn find_a_setting_finds_labels_and_help() {
        let mut harness = dialog();
        harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
        harness.run();
        let field = harness.get_by_role_and_label(Role::TextInput, "Find a setting");
        assert!(field.accesskit_node().is_focused());
        field.type_text("KÉEP A LOG");
        harness.run();
        assert_eq!(harness.state().content.query, "KÉEP A LOG");
        assert!(underlined(&harness, "Keep a log of this session"));
        assert!(!underlined(&harness, "Theme"));
        let words = Translator::new(Locale::EnGb);
        let os = OperatingSystem::Mac;
        let content = &harness.state().content;
        assert!(content.finds(Page::General, &words, os));
        assert!(!content.finds(Page::Photos, &words, os));
        // The help is searched too: Log's says what it keeps besides warnings.
        harness.state_mut().content.query = "warnings".to_owned();
        harness.run();
        assert!(underlined(&harness, "Log"));
        assert!(!underlined(&harness, "Keep a log of this session"));
        // A page with no match is chosen still.
        harness.get_by_label("Photos").click();
        harness.run();
        assert_eq!(harness.state().content.page, Page::Photos);
    }

    /// Ctrl+F (Cmd+F) with Theme's list open closes the list, and the letters typed next
    /// go to Find a setting, not to the list's filter.
    #[test]
    fn find_a_setting_closes_an_open_list() {
        let mut harness = dialog();
        harness
            .get_by_role_and_label(Role::ComboBox, "Theme")
            .click();
        harness.run_steps(12);
        assert!(harness.query_by_label("Light").is_some(), "the list open");
        harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
        harness.run();
        harness.event(Event::Text("l".to_owned()));
        harness.run_steps(12);
        assert_eq!(harness.state().content.query, "l");
        assert!(harness.query_by_label("Light").is_none(), "the list closed");
        assert!(harness.state().changes.is_empty());
    }

    /// Find a setting, empty, gives Escape up, and the host closes by its own binding:
    /// plain Escape closes Settings on Windows and Linux, Ctrl+Escape does not.
    #[test]
    fn an_empty_find_a_setting_leaves_escape_to_the_host_s_binding() {
        for os in [OperatingSystem::Windows, OperatingSystem::Nix] {
            let keeper = Keeper::nowhere().0;
            let mut harness = dialog_on(&keeper, Builtin::Dark, Locale::EnGb, os);
            for (modifiers, closes) in [(Modifiers::CTRL, 0), (Modifiers::NONE, 1)] {
                harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
                harness.run();
                let field = harness.get_by_role_and_label(Role::TextInput, "Find a setting");
                assert!(field.accesskit_node().is_focused(), "{os:?}");
                harness.key_press_modifiers(modifiers, Key::Escape);
                harness.run();
                assert_eq!(harness.state().closes, closes, "{os:?}, {modifiers:?}");
            }
        }
    }

    /// Escape held in Find a setting clears it and leaves Settings open: its repeats find
    /// Find a setting empty but are no fresh press, which alone closes, on Windows and
    /// Linux.
    #[test]
    fn a_held_escape_clears_find_a_setting_and_closes_nothing() {
        let key = |pressed, repeat| Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed,
            repeat,
            modifiers: Modifiers::NONE,
        };
        for os in [OperatingSystem::Windows, OperatingSystem::Nix] {
            let keeper = Keeper::nowhere().0;
            let mut harness = dialog_on(&keeper, Builtin::Dark, Locale::EnGb, os);
            harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
            harness.run();
            harness
                .get_by_role_and_label(Role::TextInput, "Find a setting")
                .type_text("Log");
            harness.run();
            harness.event(key(true, false));
            harness.run();
            assert_eq!(harness.state().content.query, "", "{os:?}");
            for _ in 0..2 {
                harness.event(key(true, true));
                harness.run();
            }
            assert_eq!(harness.state().closes, 0, "{os:?}");
            harness.event(key(false, false));
            harness.run();
            harness.event(key(true, false));
            harness.run();
            assert_eq!(harness.state().closes, 1, "{os:?}");
        }
    }

    /// What has the focus as Escape comes, in [`escape_goes_by_what_has_the_focus`].
    #[derive(Clone, Copy, Debug)]
    enum Focus {
        /// Find a setting.
        Find,
        /// The interface size's field, a value typed in it.
        SliderField,
        /// The cache limit's number box, on Storage, a value typed in it.
        NumberBox,
        /// The interface size's slider, nothing typed.
        Slider,
        /// The Photos page's line.
        Page,
    }

    /// Escape goes by what has the focus, on the Mac, where it never closes Settings, and
    /// on Windows and Linux, where it does: Find a setting holding something clears it,
    /// keeping the focus; a field typed in puts its value back, keeping Escape; from
    /// anything else Escape clears Find a setting while it holds anything, the dialog
    /// staying, and closes it once it is empty, Find a setting's own empty field included.
    /// Each case: what has the focus and what Find a setting holds, then after one Escape
    /// and after a second what it holds and how many times the dialog asked to close.
    #[test]
    fn escape_goes_by_what_has_the_focus() {
        const FOUND: &str = "warnings";
        let cases = [
            (Focus::Find, FOUND, ("", 0), ("", 1)),
            (Focus::Find, "", ("", 1), ("", 2)),
            (Focus::SliderField, FOUND, (FOUND, 0), ("", 0)),
            (Focus::NumberBox, FOUND, (FOUND, 0), ("", 0)),
            (Focus::NumberBox, "", ("", 0), ("", 1)),
            (Focus::Slider, FOUND, ("", 0), ("", 1)),
            (Focus::Page, FOUND, ("", 0), ("", 1)),
            (Focus::Page, "", ("", 1), ("", 2)),
        ];
        let oses = [
            OperatingSystem::Mac,
            OperatingSystem::Windows,
            OperatingSystem::Nix,
        ];
        for os in oses {
            for (focus, query, first, second) in cases {
                let keeper = Keeper::nowhere().0;
                let mut harness = dialog_on(&keeper, Builtin::Dark, Locale::EnGb, os);
                let said = format!("{focus:?} holding {query:?} on {os:?}");
                match focus {
                    Focus::Find => {
                        harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
                        harness.run();
                        if !query.is_empty() {
                            harness
                                .get_by_role_and_label(Role::TextInput, "Find a setting")
                                .type_text(query);
                            harness.run();
                        }
                    }
                    Focus::SliderField => {
                        harness.state_mut().content.query = query.to_owned();
                        let track = harness
                            .get_by_role_and_label(Role::Slider, "Interface size")
                            .rect();
                        let value = pos2(track.right() + 30.0, track.center().y);
                        harness.run_steps(30);
                        harness.hover_at(value);
                        press(&mut harness, value, PointerButton::Primary);
                        harness.run();
                        harness
                            .get_by_role_and_label(Role::TextInput, "Interface size, in %")
                            .type_text("125");
                        harness.run();
                    }
                    Focus::NumberBox => {
                        harness.state_mut().content.query = query.to_owned();
                        harness.state_mut().content.page = Page::Storage;
                        harness.run();
                        let field = "Cache limit, in GB";
                        harness
                            .get_by_role_and_label(Role::TextInput, field)
                            .focus();
                        harness.run();
                        harness
                            .get_by_role_and_label(Role::TextInput, field)
                            .type_text("8");
                        harness.run();
                    }
                    Focus::Slider | Focus::Page => {
                        harness.state_mut().content.query = query.to_owned();
                        harness.run();
                        let (role, name) = match focus {
                            Focus::Slider => (Role::Slider, "Interface size"),
                            _ => (Role::Tab, "Photos"),
                        };
                        harness.get_by_role_and_label(role, name).focus();
                        harness.run();
                        let node = harness.get_by_role_and_label(role, name);
                        assert!(node.accesskit_node().is_focused(), "{said}");
                    }
                }
                assert_eq!(harness.state().content.query, query, "{said}");
                let closes = |n: usize| if os == OperatingSystem::Mac { 0 } else { n };
                for (i, (holds, closed)) in [first, second].into_iter().enumerate() {
                    harness.key_press(Key::Escape);
                    harness.run();
                    let state = harness.state();
                    let after = format!("{said}, Escape {}", i + 1);
                    assert_eq!(state.content.query, holds, "{after}");
                    assert_eq!(state.closes, closes(closed), "{after}");
                    // A value typed is put back, never kept.
                    assert!(state.changes.is_empty(), "{after}: {:?}", state.changes);
                }
            }
        }
    }

    /// A number is shown with the fewest decimals that hold its kept step times its
    /// factor: none where the factor moves them all into the whole part, as many as it
    /// keeps where it has none, small steps included.
    #[test]
    fn a_number_shows_the_decimals_its_factor_leaves() {
        let cases = [
            (1.0, 0, 0),
            (1.0, 2, 2),
            (1.0, 7, 7),
            (4.0, 1, 1),
            (4.0, 2, 2),
            (5.0, 1, 1),
            (100.0, 0, 0),
            (100.0, 1, 0),
            (100.0, 2, 0),
            (100.0, 3, 1),
        ];
        for (factor, decimals, shown) in cases {
            let number = apertrust_base::settings::Number {
                min: None,
                max: None,
                above: None,
                step: None,
                decimals: Some(decimals),
                unit: None,
                factor: Some(factor),
            };
            assert_eq!(shown_decimals(&number), shown, "{factor} with {decimals}");
        }
    }

    /// A number typed as its row shows it is kept as its factor and decimals have it, for
    /// each factor the schema may give: 1, 4, which no power of ten divides, and 100.
    #[test]
    fn a_number_typed_is_kept_through_its_factor() {
        let cases = [
            (1.0, 1, "2.5", 2.5),
            (1.0, 0, "7", 7.0),
            (4.0, 1, "0.8", 0.2),
            // Half way between 0.4 and 0.8, rounded up, as the store rounds 0.15.
            (4.0, 1, "0.6", 0.2),
            // A factor of 0.00000001 shows eight decimals, each kept value apart.
            (0.00000001, 0, "0.00000002", 2.0),
            (4.0, 2, "0.04", 0.01),
            (100.0, 2, "80", 0.8),
        ];
        for (factor, decimals, typed, kept) in cases {
            let mut harness = dialog();
            let content = &mut harness.state_mut().content;
            content.page = Page::Storage;
            let row = content.rows.iter_mut().find(|row| row.id == "cache.limit");
            let row = row.expect("the cache's limit");
            row.kind = Kind::Number(apertrust_base::settings::Number {
                min: Some(0.0),
                max: Some(1000.0),
                above: None,
                step: Some(10f32.powi(-decimals)),
                decimals: Some(decimals as u32),
                unit: Some("gb"),
                factor: Some(factor),
            });
            harness.run();
            let field = "Cache limit, in GB";
            harness
                .get_by_role_and_label(Role::TextInput, field)
                .focus();
            harness.run();
            harness
                .get_by_role_and_label(Role::TextInput, field)
                .type_text(typed);
            harness.key_press(Key::Enter);
            harness.run();
            let value = if decimals == 0 {
                Value::Integer(kept as i64)
            } else {
                Value::Float(kept)
            };
            let said = format!("{factor} with {decimals}: {typed}");
            let set = Change::Set("cache.limit", value);
            assert_eq!(harness.state().changes, [set], "{said}");
        }
    }

    /// A number with no step of its own steps by one kept step, shown through its factor: a
    /// whole factor of 1 to 3 shown in per cent steps by 100, never by a 1 % the store drops.
    #[test]
    fn a_number_with_no_step_steps_by_its_shown_grid() {
        let mut harness = dialog();
        let content = &mut harness.state_mut().content;
        let row = content.rows.iter_mut().find(|row| row.id == "scale");
        let row = row.expect("the interface's size");
        row.kind = Kind::Number(apertrust_base::settings::Number {
            min: Some(1.0),
            max: Some(3.0),
            above: None,
            step: None,
            decimals: Some(0),
            unit: Some("percent"),
            factor: Some(100.0),
        });
        row.value = Some(Value::Integer(1));
        harness.run();
        harness
            .get_by_role_and_label(Role::Slider, "Interface size")
            .focus();
        harness.run();
        harness.key_press(Key::ArrowRight);
        harness.run();
        assert_eq!(
            harness.state().changes,
            [Change::Set("scale", Value::Integer(2))]
        );
    }

    /// The dialog on Storage, the cache's limit declared as `number`, `typed` in its box and
    /// Enter pressed.
    fn typed_into_the_limit(
        number: apertrust_base::settings::Number,
        typed: &str,
    ) -> Harness<'static, Bench> {
        let mut harness = dialog();
        let content = &mut harness.state_mut().content;
        content.page = Page::Storage;
        let row = content.rows.iter_mut().find(|row| row.id == "cache.limit");
        row.expect("the cache's limit").kind = Kind::Number(number);
        harness.run();
        let field = "Cache limit, in GB";
        harness
            .get_by_role_and_label(Role::TextInput, field)
            .focus();
        harness.run();
        harness
            .get_by_role_and_label(Role::TextInput, field)
            .type_text(typed);
        harness.key_press(Key::Enter);
        harness.run();
        harness
    }

    /// Whether the box says its range under it, as it does for a value it refuses.
    fn says_its_range(harness: &Harness<'_, Bench>) -> bool {
        [" to ", "more than", "or more", "up to"]
            .iter()
            .any(|words| {
                harness.query_all_by_label_contains(words).any(|node| {
                    let node = node.accesskit_node();
                    let said = node.label().or_else(|| node.value()).unwrap_or_default();
                    said.ends_with(" GB") && !said.starts_with("Reset")
                })
            })
    }

    /// A number box compares what is typed with its ends, and keeps it, in one
    /// representation, the schema's decimals shown through the factor, and one rounding,
    /// the store's: for factors 1, 4 and 100 and 0 to 2 decimals kept, a value at the
    /// maximum (0.7, which no `f32` holds) and the minimum is kept, one a kept step past
    /// either is refused, saying the range; with an `above` of 0 in place of the minimum,
    /// one step over it is kept, and 0, or a value that rounds to it, is refused.
    #[test]
    fn a_number_box_takes_its_ends_in_one_representation() {
        let words = Translator::new(Locale::EnGb);
        for factor in [1.0f32, 4.0, 100.0] {
            for decimals in 0..=2u32 {
                let unit = 10f64.powi(-(decimals as i32));
                let (min, max) = (3.0 * unit, if decimals == 0 { 7.0 } else { 0.7 });
                let number =
                    |min: Option<f64>, above: Option<f32>| apertrust_base::settings::Number {
                        min: min.map(|min| min as f32),
                        max: Some(max as f32),
                        above,
                        step: Some(unit as f32),
                        decimals: Some(decimals),
                        unit: Some("gb"),
                        factor: Some(factor),
                    };
                let bounded = number(Some(min), None);
                let above = number(None, Some(0.0));
                // Each case: the number, the value kept that is typed as it shows, how
                // many decimals more it is typed with, and whether it is kept.
                let cases = [
                    (bounded, max, 0, true),
                    (bounded, max + unit, 0, false),
                    (bounded, min, 0, true),
                    (bounded, min - unit, 0, false),
                    (above, unit, 0, true),
                    (above, 0.0, 0, false),
                    (above, 0.4 * unit, 2, false),
                ];
                for (number, value, more, kept) in cases {
                    let shown_with = shown_decimals(&number) + more;
                    let typed = words.format_number(value * f64::from(factor), shown_with, false);
                    let harness = typed_into_the_limit(number, &typed);
                    let said = format!("{factor} with {decimals}: {typed}");
                    let changes = &harness.state().changes;
                    if kept {
                        let value = if decimals == 0 {
                            Value::Integer(value.round() as i64)
                        } else {
                            Value::Float((value / unit).round() * unit)
                        };
                        let value = match value {
                            Value::Float(v) => Value::Float((v * 1e6).round() / 1e6),
                            other => other,
                        };
                        assert_eq!(changes, &[Change::Set("cache.limit", value)], "{said}");
                        assert!(!says_its_range(&harness), "{said}");
                    } else {
                        assert!(changes.is_empty(), "{said}: {changes:?}");
                        assert!(says_its_range(&harness), "{said}");
                    }
                }
            }
        }
    }

    /// Escape in Find a setting clears it and leaves it the focus, so what is typed next
    /// finds again; Escape in a field being typed in is the field's, putting its value back
    /// and leaving Find a setting as it was, until Escape comes from no field.
    #[test]
    fn escape_in_a_field_is_the_field_s_own() {
        for os in [OperatingSystem::Mac, OperatingSystem::Windows] {
            let keeper = Keeper::nowhere().0;
            let mut harness = dialog_on(&keeper, Builtin::Dark, Locale::EnGb, os);
            harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
            harness.run();
            harness
                .get_by_role_and_label(Role::TextInput, "Find a setting")
                .type_text("Theme");
            harness.run();
            assert_eq!(harness.state().content.query, "Theme", "{os:?}");
            harness.key_press(Key::Escape);
            harness.run();
            assert_eq!(harness.state().content.query, "", "{os:?}");
            let field = harness.get_by_role_and_label(Role::TextInput, "Find a setting");
            assert!(field.accesskit_node().is_focused(), "{os:?}");
            field.type_text("Log");
            harness.run();
            assert_eq!(harness.state().content.query, "Log", "{os:?}");
            // The interface's size typed in, then put back by Escape.
            let track = harness
                .get_by_role_and_label(Role::Slider, "Interface size")
                .rect();
            harness.run_steps(30);
            press(
                &mut harness,
                pos2(track.right() + 30.0, track.center().y),
                PointerButton::Primary,
            );
            harness.run();
            harness
                .get_by_role_and_label(Role::TextInput, "Interface size, in %")
                .type_text("125");
            harness.run();
            harness.key_press(Key::Escape);
            harness.run();
            assert_eq!(harness.state().content.query, "Log", "{os:?}");
            assert!(harness.state().changes.is_empty(), "{os:?}");
            assert_eq!(harness.state().closes, 0, "{os:?}");
            harness.key_press(Key::Escape);
            harness.run();
            assert_eq!(harness.state().content.query, "", "{os:?}");
            assert_eq!(harness.state().closes, 0, "{os:?}");
        }
    }

    /// A value typed and left by choosing another page is kept, as leaving it on its own
    /// page keeps it: a slider's field and a number box, the page chosen by the pointer or
    /// by a screen reader's click, which moves no focus.
    #[test]
    fn a_value_typed_is_kept_as_another_page_is_chosen() {
        let cases = [
            (
                Page::General,
                "Interface size, in %",
                "125",
                Change::Set("scale", 1.25.into()),
            ),
            (
                Page::Storage,
                "Cache limit, in GB",
                "8",
                Change::Set("cache.limit", 8.0.into()),
            ),
        ];
        for ((page, field, typed, kept), screen_reader) in cases
            .into_iter()
            .flat_map(|case| [(case.clone(), false), (case, true)])
        {
            let mut harness = dialog();
            harness.state_mut().content.page = page;
            harness.run();
            if page == Page::General {
                let track = harness
                    .get_by_role_and_label(Role::Slider, "Interface size")
                    .rect();
                harness.run_steps(30);
                press(
                    &mut harness,
                    pos2(track.right() + 30.0, track.center().y),
                    PointerButton::Primary,
                );
                harness.run();
            } else {
                harness
                    .get_by_role_and_label(Role::TextInput, field)
                    .focus();
                harness.run();
            }
            harness
                .get_by_role_and_label(Role::TextInput, field)
                .type_text(typed);
            harness.run();
            assert!(harness.state().changes.is_empty(), "{field}");
            let tab = harness.get_by_role_and_label(Role::Tab, "Photos");
            if screen_reader {
                let (target_node, target_tree) = tab.accesskit_node().locate();
                harness.event(Event::AccessKitActionRequest(
                    egui::accesskit::ActionRequest {
                        target_node,
                        target_tree,
                        action: egui::accesskit::Action::Click,
                        data: None,
                    },
                ));
            } else {
                let photos = tab.rect().center();
                harness.hover_at(photos);
                press(&mut harness, photos, PointerButton::Primary);
            }
            harness.run();
            let said = format!("{field}, a screen reader's: {screen_reader}");
            let page = Change::Set("settings_window.page", "photos".into());
            assert_eq!(harness.state().changes, [kept, page], "{said}");
            assert_eq!(harness.state().content.page, Page::Photos, "{said}");
        }
    }

    /// The dialog on Windows over `keeper`'s settings, Processing shown with its graphics
    /// cards listed.
    fn windows(keeper: &Keeper) -> Harness<'static, Bench> {
        let mut harness = dialog_on(
            keeper,
            Builtin::Dark,
            Locale::EnGb,
            OperatingSystem::Windows,
        );
        processing(&mut harness);
        harness
    }

    /// The software adapter in use, `chosen` or fallen back to.
    fn software_in_use(chosen: bool) -> hardware::InUse {
        hardware::InUse {
            name: WARP.to_owned(),
            backend: eframe::egui_wgpu::wgpu::Backend::Dx12,
            software: true,
            chosen,
        }
    }

    /// The graphics card box's value and description, as screen readers hear them.
    fn card_box(harness: &Harness<'_, Bench>) -> (String, Option<String>, bool) {
        let node = harness.get_by_role_and_label(Role::ComboBox, "Graphics card");
        let node = node.accesskit_node();
        let value = node.value().unwrap_or_default();
        (value, node.description(), node.is_disabled())
    }

    /// The open list's items: each one's words, whether it is disabled and whether checked.
    fn items(harness: &Harness<'_, Bench>) -> Vec<(String, bool, bool)> {
        harness
            .get_all_by_role(Role::MenuItemCheckBox)
            .map(|node| {
                let node = node.accesskit_node();
                let checked = node.toggled() == Some(Toggled::True);
                (
                    node.label().unwrap_or_default(),
                    node.is_disabled(),
                    checked,
                )
            })
            .collect()
    }

    /// Opens the list of the choice `name` and chooses `item`.
    fn choose(harness: &mut Harness<'_, Bench>, name: &str, item: &str) {
        harness.get_by_role_and_label(Role::ComboBox, name).click();
        harness.run();
        harness.get_by_label(item).click();
        harness.run();
    }

    use crate::hardware::tests::{M4, RADEON, RTX, WARP};

    const CPU_ITEM: &str = "CPU (no graphics card), slow";

    /// Automatic first, saying what it picks, the cards in Automatic's order, a separator
    /// and CPU; choosing a card stores its name, CPU `cpu`, and Automatic resets the
    /// setting. The box's value is its whole text, its description the row's note, and the
    /// software adapter's own name is never shown (decision 82).
    #[test]
    fn the_graphics_card_is_a_list_of_the_cards_the_api_finds() {
        let mut harness = windows(&Keeper::nowhere().0);
        let automatic = "Automatic".to_owned();
        let after_restart = Some("after a restart".to_owned());
        assert_eq!(
            card_box(&harness),
            (automatic.clone(), after_restart, false)
        );
        harness
            .get_by_role_and_label(Role::ComboBox, "Graphics card")
            .click();
        harness.run();
        let listed = [
            (automatic.as_str(), false, true),
            (RTX, false, false),
            (RADEON, false, false),
            (CPU_ITEM, false, false),
        ]
        .map(|(words, disabled, checked)| (words.to_owned(), disabled, checked));
        assert_eq!(items(&harness), listed);
        assert_eq!(harness.get_all_by_role(Role::Splitter).count(), 1);
        assert!(harness.query_by_label(WARP).is_none());
        harness.get_by_label(RADEON).click();
        harness.run();
        let set = |value: &str| Change::Set("engine.adapter", value.into());
        assert_eq!(harness.state().changes, [set(RADEON)]);
        assert!(dotted(&harness, "Graphics card"));
        assert_eq!(card_box(&harness).0, RADEON);
        choose(&mut harness, "Graphics card", CPU_ITEM);
        assert_eq!(harness.state().changes.last(), Some(&set("cpu")));
        assert_eq!(card_box(&harness).0, CPU_ITEM);
        choose(&mut harness, "Graphics card", &automatic);
        let reset = Change::Reset("engine.adapter");
        assert_eq!(harness.state().changes.last(), Some(&reset));
        assert!(!dotted(&harness, "Graphics card"));
    }

    /// Where the API has no software adapter, CPU is disabled, saying so: Vulkan on Linux
    /// without lavapipe; the Mac never lists it.
    #[test]
    fn cpu_is_disabled_where_the_api_has_none_and_the_mac_lists_no_cpu() {
        let keeper = Keeper::nowhere().0;
        let mut harness = dialog_on(&keeper, Builtin::Dark, Locale::EnGb, OperatingSystem::Nix);
        processing(&mut harness);
        harness
            .get_by_role_and_label(Role::ComboBox, "Graphics card")
            .click();
        harness.run();
        let cpu = items(&harness).pop().expect("an item");
        assert_eq!(cpu, (CPU_ITEM.to_owned(), true, false));
        harness.get_by_label(CPU_ITEM).hover();
        harness.run_steps(10);
        harness.get_by_label("Not found with Vulkan");

        let mut harness = dialog();
        processing(&mut harness);
        harness
            .get_by_role_and_label(Role::ComboBox, "Graphics card")
            .click();
        harness.run();
        let automatic = "Automatic".to_owned();
        let listed = [(automatic, false, true), (M4.to_owned(), false, false)];
        assert_eq!(items(&harness), listed);
        assert_eq!(harness.query_all_by_role(Role::Splitter).count(), 0);
    }

    /// A stored card this computer does not have is kept, opening Settings writes nothing,
    /// and the row says Automatic is used, in the box's description too; the list holds it,
    /// disabled and checked. A reset clears it.
    #[test]
    fn a_card_not_found_is_kept_and_said_so() {
        let gone = "NVIDIA GeForce RTX 4090";
        let mut harness = windows(&keeper_with("engine.adapter", gone));
        let said = "Graphics card not found: Automatic used";
        harness.get_by_label(said);
        assert_eq!(card_box(&harness), (gone.into(), Some(said.into()), false));
        assert!(harness.query_all_by_label("after a restart").count() > 0);
        assert!(dotted(&harness, "Graphics card"));
        assert!(harness.state().changes.is_empty());
        harness
            .get_by_role_and_label(Role::ComboBox, "Graphics card")
            .click();
        harness.run();
        assert!(items(&harness).contains(&(gone.to_owned(), true, true)));
        harness.key_press(Key::Escape);
        harness.run();
        let at = harness
            .get_by_role_and_label(Role::Label, "Graphics card")
            .rect()
            .center();
        double_click(&mut harness, at);
        let reset = Change::Reset("engine.adapter");
        assert_eq!(harness.state().changes, [reset]);
        assert!(harness.query_by_label(said).is_none());
    }

    /// A card found with the API the row had that the new API lacks goes back to
    /// Automatic, saying so once on the card's row: both changes kept, in one frame where
    /// the new API was listed before. A card found with neither API stays.
    #[test]
    fn an_api_change_drops_a_card_the_new_api_lacks() {
        let mut harness = windows(&keeper_with("engine.adapter", "cpu"));
        // The API row's Automatic is plain: the In use line says what runs.
        let api = harness.get_by_role_and_label(Role::ComboBox, "Graphics API");
        let api = api.accesskit_node().value();
        assert_eq!(api.as_deref(), Some("Automatic"));
        choose(&mut harness, "Graphics API", "Vulkan");
        listed(&mut harness);
        let backend = |value: &str| Change::Set("engine.backend", value.into());
        let reset = Change::Reset("engine.adapter");
        assert_eq!(harness.state().changes, [backend("vulkan"), reset.clone()]);
        let said = "CPU isn’t available with Vulkan: set to Automatic";
        let note = harness.get_by_label(said);
        let polite = egui::accesskit::Live::Polite;
        assert_eq!(note.accesskit_node().data().live(), Some(polite));
        assert_eq!(card_box(&harness).1.as_deref(), Some(said));
        // Back on Automatic it goes; CPU again, then Vulkan, listed already: one frame.
        choose(&mut harness, "Graphics API", "Automatic");
        assert!(harness.query_by_label(said).is_none());
        choose(&mut harness, "Graphics card", CPU_ITEM);
        choose(&mut harness, "Graphics API", "Vulkan");
        let last = harness.state().frames.last().cloned();
        assert_eq!(last, Some(vec![backend("vulkan"), reset]));
        harness.get_by_label(said);
        // Another page shown, and it goes.
        harness.get_by_label("Storage").click();
        harness.run();
        processing(&mut harness);
        assert!(harness.query_by_label(said).is_none());

        // A card found with neither API is not this rule's.
        let gone = "NVIDIA GeForce RTX 4090";
        let mut harness = windows(&keeper_with("engine.adapter", gone));
        choose(&mut harness, "Graphics API", "Vulkan");
        listed(&mut harness);
        assert_eq!(harness.state().changes, [backend("vulkan")]);
        harness.get_by_label("Graphics card not found: Automatic used");
    }

    /// A card `--conf` holds for this run stays as it is through an API change, its row
    /// saying so and nothing else.
    #[test]
    fn an_api_change_leaves_a_card_held_by_conf() {
        let conf = ["engine.adapter=cpu".to_owned()];
        let (keeper, _) = Keeper::open_in(None, &settings::overrides(&conf).expect("read"));
        let mut harness = windows(&keeper);
        choose(&mut harness, "Graphics API", "Vulkan");
        listed(&mut harness);
        let backend = Change::Set("engine.backend", "vulkan".into());
        assert_eq!(harness.state().changes, [backend]);
        let label = harness.get_by_role_and_label(Role::Label, "Graphics card");
        let said = label.accesskit_node().description();
        assert_eq!(said.as_deref(), Some("Set by --conf for this run"));
        assert!(harness.query_by_label_contains("isn’t available").is_none());
        assert_eq!(card_box(&harness).0, CPU_ITEM);
    }

    /// A gate on one API's listing, which waits until released, the other APIs listed at
    /// once as the test laptop lists them.
    fn gated(api: Backend) -> (hardware::Running, std::sync::mpsc::Sender<()>) {
        let os = OperatingSystem::Windows;
        let (release, gate) = std::sync::mpsc::channel::<()>();
        let gate = std::sync::Mutex::new(gate);
        let mut running = crate::hardware::tests::running(os);
        running.lister = Some(hardware::Lister::new(move |listed| {
            if listed == api {
                let _ = gate.lock().expect("one listing at a time").recv();
            }
            Ok(crate::hardware::tests::listed(os, listed))
        }));
        (running, release)
    }

    /// An API changed before the old one's listing is in: the card is decided once both are
    /// in, gone where the new API lacks it; and another page shown meanwhile says nothing
    /// as the person comes back.
    #[test]
    fn an_api_change_before_the_old_listing_is_in_waits_for_it() {
        let os = OperatingSystem::Windows;
        let keeper = keeper_with("engine.adapter", "cpu");
        let (running, release) = gated(Backend::Auto);
        let mut harness = dialog_with(&keeper, Builtin::Dark, Locale::EnGb, os, running);
        harness.get_by_label("Processing").click();
        harness.run();
        choose(&mut harness, "Graphics API", "Vulkan");
        listed_for(&mut harness, Backend::Vulkan);
        let backend = |value: &str| Change::Set("engine.backend", value.into());
        assert!(
            !harness
                .state()
                .changes
                .contains(&Change::Reset("engine.adapter"))
        );
        release.send(()).expect("the listing waits");
        listed(&mut harness);
        let reset = Change::Reset("engine.adapter");
        assert_eq!(harness.state().changes.last(), Some(&reset));
        assert!(harness.state().changes.contains(&backend("vulkan")));
        harness.get_by_label("CPU isn’t available with Vulkan: set to Automatic");

        // Left for another page before the new API's listing is in: nothing is said after.
        let (running, release) = gated(Backend::Vulkan);
        let mut harness = dialog_with(&keeper, Builtin::Dark, Locale::EnGb, os, running);
        processing(&mut harness);
        choose(&mut harness, "Graphics API", "Vulkan");
        harness.get_by_label("Storage").click();
        harness.run();
        release.send(()).expect("the listing waits");
        harness.get_by_label("Processing").click();
        harness.run();
        listed(&mut harness);
        assert!(harness.query_by_label_contains("isn’t available").is_none());
        let reset = Change::Reset("engine.adapter");
        assert!(!harness.state().changes.contains(&reset));
    }

    /// Lets `api`'s listing answer, and the dialog take it, others still under way.
    fn listed_for(harness: &mut Harness<'_, Bench>, api: Backend) {
        harness.state().content.hardware.wait_for(api);
        harness.run();
    }

    /// While the cards are listed the box is disabled, showing the stored choice; it says
    /// it is looking only after 0.3 seconds, so a quick listing flashes nothing.
    #[test]
    fn listing_says_so_only_after_a_moment() {
        let os = OperatingSystem::Windows;
        let (release, gate) = std::sync::mpsc::channel::<()>();
        let gate = std::sync::Mutex::new(gate);
        let mut running = crate::hardware::tests::running(os);
        running.lister = Some(hardware::Lister::new(move |api| {
            let _ = gate.lock().expect("one listing at a time").recv();
            Ok(crate::hardware::tests::listed(os, api))
        }));
        let keeper = Keeper::nowhere().0;
        let mut harness = dialog_with(&keeper, Builtin::Dark, Locale::EnGb, os, running);
        harness.get_by_label("Processing").click();
        harness.run();
        let looking = "Looking for graphics cards…";
        let after_restart = Some("after a restart".to_owned());
        assert_eq!(
            card_box(&harness),
            ("Automatic".into(), after_restart, true)
        );
        assert!(harness.query_by_label(looking).is_none());
        harness.run_steps(10);
        assert!(harness.query_by_label(looking).is_none());
        harness.run_steps(20);
        harness.get_by_label(looking);
        assert_eq!(card_box(&harness).1.as_deref(), Some(looking));
        release.send(()).expect("the listing waits");
        listed(&mut harness);
        assert!(harness.query_by_label(looking).is_none());
        assert!(!card_box(&harness).2);
    }

    /// A helper that fails says the cards could not be listed and keeps the stored card; an
    /// API that finds none says so under the API's row.
    #[test]
    fn a_listing_that_fails_or_finds_nothing_says_so() {
        let os = OperatingSystem::Windows;
        let mut running = crate::hardware::tests::running(os);
        running.lister = Some(hardware::Lister::new(move |api| match api {
            Backend::Auto => Err(hardware::Failure::Crashed("exit code: 3".into())),
            _ => Ok(Vec::new()),
        }));
        let keeper = keeper_with("engine.adapter", RADEON);
        let mut harness = dialog_with(&keeper, Builtin::Dark, Locale::EnGb, os, running);
        processing(&mut harness);
        harness.get_by_label("The graphics cards could not be listed");
        assert_eq!(card_box(&harness).0, RADEON);
        choose(&mut harness, "Graphics API", "Vulkan");
        listed(&mut harness);
        harness.get_by_label("No graphics card found with Vulkan: Automatic used");
        let set = Change::Set("engine.backend", "vulkan".into());
        assert_eq!(harness.state().changes, [set]);
    }

    /// The In use line says what the engine opened, CPU for the software adapter, never
    /// its own name, warning that the processor renders, with the driver advice where
    /// Automatic fell to it.
    #[test]
    fn the_in_use_line_says_what_runs_and_warns_of_the_processor() {
        let harness = windows(&Keeper::nowhere().0);
        harness.get_by_role_and_label(Role::Label, "In use");
        harness.get_by_role_and_label(Role::Label, &format!("{RTX}, Vulkan"));
        let software = "No graphics card in use: Apertrust renders on the processor, which \
                        is slower";
        assert!(harness.query_by_label(software).is_none());
        let advice = "Updating the graphics driver may help";
        for chosen in [false, true] {
            let os = OperatingSystem::Windows;
            let mut running = crate::hardware::tests::running(os);
            running.in_use = Some(software_in_use(chosen));
            let keeper = Keeper::nowhere().0;
            let mut harness = dialog_with(&keeper, Builtin::Dark, Locale::EnGb, os, running);
            processing(&mut harness);
            let value = harness.get_by_role_and_label(Role::Label, "CPU, Direct3D 12");
            let note = if chosen {
                software.to_owned()
            } else {
                format!("{software}\n{advice}")
            };
            let described = value.accesskit_node().description();
            assert_eq!(described.as_deref(), Some(note.as_str()), "{chosen}");
            harness.get_by_label(&note);
            assert!(harness.query_by_label(WARP).is_none());
        }
    }

    /// The dialog on Windows over `keeper`'s settings, as a run that started with
    /// `started`'s, Processing shown with its graphics cards listed.
    fn started_with(keeper: &Keeper, started: &Keeper) -> Harness<'static, Bench> {
        let os = OperatingSystem::Windows;
        let mut running = crate::hardware::tests::running(os);
        running.started = hardware::Started::of(settings::shown(), |s| started.value(s));
        let mut harness = dialog_with(keeper, Builtin::Dark, Locale::EnGb, os, running);
        processing(&mut harness);
        harness
    }

    const WAITING: &str = "Your choice applies when Apertrust restarts";

    /// The waiting line shows while a setting that applies after a restart has a value
    /// other than this run's, compared by value, and goes as it comes back; Restart now
    /// asks the app for the restart. The words are a label in a polite live region, the
    /// button named Restart now and described by them, after the last row in Tab's order.
    #[test]
    fn the_waiting_line_shows_while_a_choice_waits_for_a_restart() {
        let keeper = Keeper::nowhere().0;
        let mut harness = started_with(&keeper, &keeper);
        assert!(harness.query_by_label(WAITING).is_none());
        choose(&mut harness, "Graphics card", RADEON);
        harness.run();
        let said = harness.get_by_role_and_label(Role::Label, WAITING);
        let polite = egui::accesskit::Live::Polite;
        assert_eq!(said.accesskit_node().data().live(), Some(polite));
        let button = harness.get_by_role_and_label(Role::Button, "Restart now");
        let described = button.accesskit_node().description();
        assert_eq!(described.as_deref(), Some(WAITING));
        // Below the In use line, the button after the words.
        let in_use = harness.get_by_role_and_label(Role::Label, "In use").rect();
        assert!(said.rect().top() > in_use.bottom());
        assert!(button.rect().left() > said.rect().right());
        // Back to the value it started with, Automatic, and it goes.
        choose(&mut harness, "Graphics card", "Automatic");
        harness.run();
        assert!(harness.query_by_label(WAITING).is_none());
        // The memory budget waits too; Tab reaches Restart now after its field.
        let field = "Memory budget, in MiB";
        harness
            .get_by_role_and_label(Role::TextInput, field)
            .click();
        harness.run();
        harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
        harness
            .get_by_role_and_label(Role::TextInput, field)
            .type_text("4096");
        harness.key_press(Key::Enter);
        harness.run();
        harness.get_by_label(WAITING);
        harness
            .get_by_role_and_label(Role::TextInput, field)
            .focus();
        harness.run();
        harness.key_press(Key::Tab);
        harness.run();
        let button = harness.get_by_role_and_label(Role::Button, "Restart now");
        assert!(button.accesskit_node().is_focused());
        assert_eq!(harness.state().restarts, 0);
        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(harness.state().restarts, 1);
        // Settings started with the card it holds now wait for nothing.
        let radeon = keeper_with("engine.adapter", RADEON);
        let harness = started_with(&radeon, &radeon);
        assert!(harness.query_by_label(WAITING).is_none());
        // A run that hands no values waits for nothing either.
        let harness = windows(&radeon);
        assert!(harness.query_by_label(WAITING).is_none());
    }

    // How it looks.

    /// A card chosen and waiting for a restart, on Windows: the waiting line and Restart now
    /// under the In use line.
    #[test]
    fn the_waiting_line_in_the_dark_theme() {
        let started = Keeper::nowhere().0;
        let mut harness = started_with(&keeper_with("engine.adapter", RADEON), &started);
        harness.event(Event::PointerGone);
        harness.run();
        harness.snapshot("settings-restart-waiting-dark");
    }

    #[test]
    fn the_general_page_in_the_dark_theme() {
        let mut harness = dialog();
        let ask = "Ask before overwriting an exported file";
        harness.get_by_role_and_label(Role::Switch, ask).click();
        harness.event(Event::PointerGone);
        harness.run();
        harness.snapshot("settings-general-dark");
    }

    #[test]
    fn the_processing_page_in_the_dark_theme() {
        let mut harness = dialog();
        processing(&mut harness);
        harness.event(Event::PointerGone);
        harness.run();
        harness.snapshot("settings-processing-dark");
    }

    /// The graphics card's list open on Windows: Automatic with what it picks, the cards,
    /// a separator and CPU.
    #[test]
    fn the_graphics_card_list_open_on_windows_in_the_dark_theme() {
        let mut harness = windows(&Keeper::nowhere().0);
        harness
            .get_by_role_and_label(Role::ComboBox, "Graphics card")
            .click();
        harness.run();
        harness.event(Event::PointerGone);
        harness.run();
        harness.snapshot("settings-graphics-card-list-dark");
    }

    /// A card the computer does not have, and the processor in use, each in the warning
    /// colour.
    #[test]
    fn a_card_not_found_and_the_processor_in_use_in_the_dark_theme() {
        let keeper = keeper_with("engine.adapter", "NVIDIA GeForce RTX 4090");
        let os = OperatingSystem::Windows;
        let mut running = crate::hardware::tests::running(os);
        running.in_use = Some(software_in_use(false));
        let mut harness = dialog_with(&keeper, Builtin::Dark, Locale::EnGb, os, running);
        processing(&mut harness);
        harness.event(Event::PointerGone);
        harness.run();
        harness.snapshot("settings-graphics-card-warnings-dark");
    }

    /// Storage: its folders, the cache's three rows from the schema at their defaults, and
    /// what the cache holds, as the session answered.
    #[test]
    fn the_storage_page_in_the_dark_theme() {
        let mut harness = dialog();
        harness.state_mut().content.storage = crate::storage::tests::sample();
        harness.get_by_label("Storage").click();
        harness.event(Event::PointerGone);
        harness.run();
        harness.snapshot("settings-storage-dark");
    }
}
