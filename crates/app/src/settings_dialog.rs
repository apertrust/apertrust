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
//! with a row per setting the schema puts there, in the schema's order. A row is its label
//! at the start and its control by its kind at the end: a checkbox, a choice, a number with
//! its unit or a text; a list of a choice is a checkbox per option, stacked in the control
//! column in its type's order, the row as tall as they are.
//!
//! - A row's help, where its words have one (`setting-<id>-help`), is its label's tooltip.
//! - A row off its default has darktable's dot before its control; a double-click on its
//!   label, or Reset in its right-click menu, puts the default back.
//! - A row `--conf` sets for this run is drawn disabled, saying so (decision 43), and so is
//!   one whose required boolean (the schema's `requires`) is off, as the cache's limit
//!   while it is unlimited.
//! - A row whose `platforms` leave out the platform the dialog runs on, as the context's
//!   operating system says, is not drawn: the graphics API's off Windows.
//! - "Applies after a restart" is said under the control of a setting that does.
//! - Processing's Hardware acceleration group draws the graphics card as a list of the
//!   adapters the chosen API finds, says what is wrong with a card on its row, and ends in
//!   the In use line ([`hardware`], decision 81).
//! - Every change applies at once and is kept, with no OK or Apply: it goes to the app as a
//!   [`Change`], which keeps it through the settings' keeper and shows it in the main
//!   window; the theme and the language are shown in this dialog's own window at once too.
//! - A page that holds nothing yet is listed dimmed and cannot be chosen. The page last
//!   shown is kept, `settings_window.page`.
//!
//! It is a [`Dialog`] (decision 41), which the dialog host draws in a window of its own:
//! modeless on the Mac and modal on Windows and Linux.

use apertrust_base::settings::{Applies, CONF_FLAG, Choice as _, Declared, Kind, Page, Platform};
use apertrust_ui::components::{Choice, ChoiceAction, TextField, ToggleRow, ValueField};
use apertrust_ui::popup::{self, Entry, Item, Request};
use apertrust_ui::{Builtin, Locale, Theme, fonts, theme, tr, translator};
use eframe::egui::os::OperatingSystem;
use eframe::egui::{
    self, Align, FontFamily, FontId, Key, Layout, Modifiers, Rect, Role, ScrollArea, Sense, Ui,
    UiBuilder, WidgetInfo, pos2, vec2,
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
/// A page's line in the list, for its name in the section titles' size.
const PAGE_LINE: f32 = 30.0;
/// A group title's line.
const GROUP_LINE: f32 = 28.0;
/// The label column's narrowest, darktable's grid.
const LABEL_MIN: f32 = 176.0;
/// The bar that marks the chosen page, at its line's start. A token to come.
const PAGE_MARK: f32 = 3.0;
/// The column of the dot before a row's control.
const DOT: f32 = 14.0;
/// The column of a row's control, and the widest a control in it grows to.
const CONTROL: f32 = 220.0;
/// Every choice box, number field and text field of a page is this wide, or as wide as
/// its widest choice needs, up to [`CONTROL`]: one width, so their ends line up.
const CONTROL_WIDTH: f32 = 180.0;
/// The dot's radius, as darktable's. A token to come.
const DOT_RADIUS: f32 = 3.0;

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
    pub overridden: bool,
    /// The boolean setting whose row must be on for this one to be enabled.
    pub requires: Option<&'static str>,
    /// The platforms it is drawn on; None for every one.
    pub platforms: Option<&'static [Platform]>,
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
            overridden: keeper.overridden(setting),
            requires: setting.requires(),
            platforms: setting.platforms(),
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

    /// Whether it is off its default, which the dot says.
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
        let open = self.menu_open(ui.ctx());
        let asked = self.frame(ui);
        let change = asked.changes.into_iter().map(Output::Settings);
        let storage = asked.storage.into_iter().map(Output::Storage);
        Asks {
            outputs: change.into_iter().chain(storage).collect(),
            close: asked.close,
            keeps_escape: open,
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

    /// Whether `row` can be changed: neither `--conf` sets it nor is the row it requires
    /// off.
    fn enabled(&self, row: &Row) -> bool {
        let on = |id| {
            let required = self.rows.iter().find(|other| other.id == id);
            required.and_then(|other| other.value.as_ref()?.as_bool()) == Some(true)
        };
        !row.overridden && row.requires.is_none_or(on)
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
            .filter(|backend| backend.built())
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
            .any(|row| popup::is_open(ctx, menu_id(row.id)))
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
        ui.set_width(WIDTH);
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        ui.horizontal_top(|ui| {
            if let Some(page) = self.pages(ui, &t) {
                self.page = page;
                self.hardware.left();
                asked.changes.push(Change::Set(
                    settings::registry::SETTINGS_WINDOW_PAGE.id,
                    Value::String(page.id().to_owned()),
                ));
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
                                let (change, storage) = self.page_rows(ui, &t, gpu.as_ref());
                                asked.changes.extend(change);
                                if let Some(storage::Asked::Clear(kind)) = &storage {
                                    self.pressed.push(*kind);
                                }
                                asked.storage.extend(storage);
                            });
                    },
                );
            });
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

    /// The page list, each page a line: the chosen one marked, those that hold nothing yet
    /// dimmed, saying they come later, and never chosen. A click, or ↑ and ↓ while a line
    /// has the focus, chooses one. Returns the page chosen.
    fn pages(&self, ui: &mut Ui, t: &Theme) -> Option<Page> {
        let ctx = ui.ctx().clone();
        let (column, _) = ui.allocate_exact_size(vec2(PAGES, BODY), Sense::hover());
        let painter = ui.painter_at(column);
        painter.rect_filled(column, 0.0, t.colors.sunken);
        let line = t.shape.separator_width;
        painter.rect_filled(
            Rect::from_x_y_ranges(column.right() - line..=column.right(), column.y_range()),
            0.0,
            t.colors.divider,
        );
        let later = tr(&ctx, "settings-page-later");
        let font = FontId::proportional(t.text.section_title);
        let mut chosen = None;
        let mut moved = None;
        // The first page's name level with the first group's title: both centred where
        // the page's top margin and half a title's line put it.
        let top = column.top() + t.spacing.l + GROUP_LINE / 2.0 - PAGE_LINE / 2.0;
        for (i, page) in Page::ALL.into_iter().enumerate() {
            let rect = Rect::from_min_size(
                pos2(column.left(), top + i as f32 * PAGE_LINE),
                vec2(PAGES - line, PAGE_LINE),
            );
            let enabled = self.holds(page, ctx.os());
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
            let colour = if shown {
                t.colors.text_strong
            } else if enabled {
                t.colors.text
            } else {
                t.colors.text_disabled
            };
            if shown {
                painter.rect_filled(rect, 0.0, t.colors.list_chosen);
                painter.rect_filled(
                    Rect::from_min_size(rect.min, vec2(PAGE_MARK, rect.height())),
                    0.0,
                    t.colors.frame_ring,
                );
            } else if enabled && response.hovered() {
                painter.rect_filled(rect, 0.0, t.colors.hover);
            }
            let text = painter.layout_no_wrap(name, font.clone(), colour);
            painter.galley(
                pos2(
                    rect.left() + t.spacing.xl + PAGE_MARK,
                    rect.center().y - text.size().y / 2.0,
                ),
                text,
                colour,
            );
            if response.has_focus() {
                painter.rect_stroke(
                    rect.shrink(1.0),
                    t.shape.radius_small,
                    egui::Stroke::new(1.5, t.colors.focus_ring),
                    egui::StrokeKind::Inside,
                );
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

    /// The shown page's groups, each under its title, in the order of their first rows,
    /// and their rows; on Storage, its Folders group first and its In the cache group last;
    /// on Processing, the Hardware acceleration group's parts as `gpu` has them, and its In
    /// use line. Returns what the user changed, and what Storage asked.
    fn page_rows(
        &self,
        ui: &mut Ui,
        t: &Theme,
        gpu: Option<&hardware::Frame>,
    ) -> (Option<Change>, Option<storage::Asked>) {
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
        let title = FontId::new(t.text.section_title, FontFamily::Name(fonts::MEDIUM.into()));
        let content = ui.available_width();
        let words = translator(&ctx);
        let storage = self.page == Page::Storage;
        let mut labels: Vec<String> = shown().map(|row| label_of(&words, row)).collect();
        if storage {
            labels.extend(storage::labels(&words));
        }
        let in_use = gpu.and(self.hardware.running().in_use.as_ref());
        if in_use.is_some() {
            labels.extend(hardware::labels(&words));
        }
        let label_width = label_column(ui, t, &labels, content);
        let control_width = control_column(ui, &self.rows, self.page, gpu);
        let lines = storage::Widths {
            label: label_width,
            content,
            value: label_width + t.spacing.xl + DOT + t.spacing.xs,
        };
        let mut change = None;
        let mut asked = None;
        if storage {
            group_title(ui, t, &tr(&ctx, "settings-group-folders"), &title, content);
            asked = storage::folders(ui, t, &self.storage, lines);
        }
        for (i, group) in groups.into_iter().enumerate() {
            if i > 0 || storage {
                ui.add_space(t.spacing.l);
            }
            let name = tr(&ctx, &format!("settings-group-{}", group.replace('_', "-")));
            group_title(ui, t, &name, &title, content);
            for row in shown().filter(|row| row.group == group) {
                let widths = (label_width, control_width, content);
                let extra = Extra::of(row, gpu);
                if let Some(changed) = draw_row(ui, t, row, self.enabled(row), widths, extra) {
                    change = Some(changed);
                }
            }
            // What runs now, under the rows that choose what runs after a restart.
            if let Some(in_use) = in_use.filter(|_| group == HARDWARE_GROUP) {
                ui.add_space(t.spacing.m);
                hardware::in_use(ui, t, in_use, lines);
            }
        }
        if storage {
            ui.add_space(t.spacing.l);
            let name = tr(&ctx, "settings-group-cache-contents");
            group_title(ui, t, &name, &title, content);
            let pressed = &self.pressed;
            if let Some(contents) = storage::contents(ui, t, &self.storage, pressed, lines) {
                asked = Some(contents);
            }
        }
        (change, asked)
    }
}

/// A group's title, `name`, on a line of its own across `content`, its text centred in
/// it.
fn group_title(ui: &mut Ui, t: &Theme, name: &str, font: &FontId, content: f32) {
    let (line, response) = ui.allocate_exact_size(vec2(content, GROUP_LINE), Sense::hover());
    let galley = ui
        .painter()
        .layout_no_wrap(name.to_owned(), font.clone(), t.colors.text_strong);
    let at = pos2(line.left(), line.center().y - galley.size().y / 2.0);
    ui.painter().galley(at, galley, t.colors.text_strong);
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, name));
}

/// The id of `page`'s line in the list.
fn page_id(page: Page) -> egui::Id {
    egui::Id::unique(("settings page", page.id()))
}

/// The id of the right-click menu of the row of the setting `id`.
fn menu_id(id: &str) -> egui::Id {
    egui::Id::unique(("settings row menu", id))
}

/// The label column's width on a page, darktable's grid: its widest label in `ui`'s
/// language at the body size, rounded up, at least [`LABEL_MIN`] and no wider than leaves
/// the dot's column and the control within `content`, the page's width inside its margins.
fn label_column(ui: &Ui, t: &Theme, labels: &[String], content: f32) -> f32 {
    let font = FontId::proportional(t.text.body);
    let widest = labels
        .iter()
        .map(|label| {
            let galley = ui
                .painter()
                .layout_no_wrap(label.clone(), font.clone(), t.colors.text);
            galley.size().x
        })
        .fold(0.0, f32::max)
        .ceil();
    let most = content - t.spacing.xl - DOT - t.spacing.xs - CONTROL;
    widest.max(LABEL_MIN).min(most)
}

/// The words of `row`'s label.
fn label_of(words: &apertrust_ui::Translator, row: &Row) -> String {
    words.text(&format!("setting-{}-label", row.words()))
}

/// The width of every control on `page` but its checkboxes: [`CONTROL_WIDTH`], or as wide
/// as the widest box a choice drawn there needs in `ui`'s language, the graphics card's
/// list and the API's Automatic as `gpu` has them among them, up to [`CONTROL`].
fn control_column(ui: &Ui, rows: &[Row], page: Page, gpu: Option<&hardware::Frame>) -> f32 {
    let os = ui.ctx().os();
    let width = |names: &[String]| {
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        Choice::new("", &names, None).box_width(ui)
    };
    let choices = rows
        .iter()
        .filter(|row| row.page == page && row.drawn_on(os))
        .filter(|row| matches!(row.kind, Kind::Choice(_)))
        .map(|row| width(&options(ui.ctx(), row).1));
    let gpu = gpu.into_iter().flat_map(|gpu| {
        let automatic = gpu.automatic.iter().cloned().collect::<Vec<String>>();
        [width(&gpu.card.words), width(&automatic)]
    });
    choices
        .chain(gpu)
        .fold(CONTROL_WIDTH, f32::max)
        .min(CONTROL)
}

/// The Hardware acceleration group, whose rows [`Extra`] adds to, by its id.
const HARDWARE_GROUP: &str = "hardware_acceleration";

/// What a row of the Hardware acceleration group draws in place of its kind's (decision
/// 81): the graphics card's list for its text, the API's Automatic with what it picks, and
/// what is said under either.
#[derive(Clone, Copy, Debug, Default)]
struct Extra<'a> {
    card: Option<&'a hardware::Card>,
    automatic: Option<&'a str>,
    note: Option<&'a hardware::Note>,
}

impl<'a> Extra<'a> {
    /// `row`'s, as `gpu` has the group this frame.
    fn of(row: &Row, gpu: Option<&'a hardware::Frame>) -> Extra<'a> {
        match gpu {
            Some(gpu) if row.id == ENGINE_ADAPTER.id => Extra {
                card: Some(&gpu.card),
                note: gpu.card.note.as_ref(),
                ..Extra::default()
            },
            Some(gpu) if row.id == ENGINE_BACKEND.id => Extra {
                automatic: gpu.automatic.as_deref(),
                note: gpu.api_note.as_ref(),
                ..Extra::default()
            },
            _ => Extra::default(),
        }
    }
}

/// Draws `row` across `content` points: its label in the first `label_width`, wrapped
/// where longer, the dot while it is off its default, its control, and what is said of it
/// under the row, wrapping at the content's end, one note at a time: `--conf`'s, then what
/// `extra` says, then "Applies after a restart"; disabled unless `enabled`. `widths` are
/// the label's, the control's and the content's. Returns what the user changed.
fn draw_row(
    ui: &mut Ui,
    t: &Theme,
    row: &Row,
    enabled: bool,
    widths: (f32, f32, f32),
    extra: Extra<'_>,
) -> Option<Change> {
    let (label_width, control_width, content) = widths;
    let ctx = ui.ctx().clone();
    let tr_ = translator(&ctx);
    let label = label_of(&tr_, row);
    let line = t.sizes.panel_line;
    let top = ui.cursor().min;
    let mut change = None;
    // The label: a double-click, Delete or the menu's Reset puts the default back. Its
    // target is its own column, never the gap before the dot.
    let colour = if enabled {
        t.colors.text
    } else {
        t.colors.text_disabled
    };
    let text = ui.painter().layout(
        label.clone(),
        FontId::proportional(t.text.body),
        colour,
        label_width,
    );
    let first = text.size().y / text.rows.len().max(1) as f32;
    let label_rect = Rect::from_min_size(top, vec2(label_width, text.size().y.max(line)));
    let response = ui.interact(
        label_rect,
        egui::Id::unique(("settings label", row.id)),
        Sense::click(),
    );
    ui.painter()
        .galley(pos2(top.x, top.y + (line - first) / 2.0), text, colour);
    let resets = enabled && row.changed();
    response.widget_info(|| WidgetInfo::labeled(Role::Label, enabled, &label));
    let help = format!("setting-{}-help", row.words());
    let response = if tr_.has(&help) {
        response.on_hover_text(tr_.text(&help))
    } else {
        response
    };
    if row.changed() {
        let said = tr_.text("settings-changed");
        ctx.accesskit_node_builder(response.id, |node| node.set_description(said));
    }
    // Delete on the label in focus resets it too, as on a module's row.
    let delete =
        response.has_focus() && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Delete));
    if resets && (response.double_clicked() || delete) {
        change = Some(Change::Reset(row.id));
    }
    let menu = menu_id(row.id);
    if response.secondary_clicked()
        && let Some(at) = response.interact_pointer_pos()
    {
        let reset = Item::new("reset", tr_.text("settings-reset")).enabled(resets);
        let anchor = Rect::from_min_size(at, egui::Vec2::ZERO);
        let request = Request::pull_down(anchor, label.clone(), vec![Entry::Item(reset)])
            .opened_by(response.id);
        popup::open(&ctx, menu, request);
        popup::show(&ctx);
    }
    if let Some(popup::Answer::Chosen(item)) = popup::answer(&ctx, menu)
        && item == "reset"
        && resets
    {
        change = Some(Change::Reset(row.id));
    }
    // The dot, darktable's, centred in its column before the control; dimmed on a row the
    // command line holds.
    let dot = Rect::from_min_size(
        pos2(top.x + label_width + t.spacing.xl, top.y),
        vec2(DOT, line),
    );
    if row.changed() {
        let colour = if enabled {
            t.colors.kept_marker
        } else {
            t.colors.text_disabled
        };
        ui.painter().circle_filled(dot.center(), DOT_RADIUS, colour);
    }
    // The control, its start edge on the column's; a list's checkboxes one under another.
    let start = dot.right() + t.spacing.xs;
    let (lines, layout) = match row.kind {
        Kind::List {
            of: Kind::Choice(_),
            ..
        } => (row.options.len().max(1), Layout::top_down(Align::Min)),
        _ => (1, Layout::left_to_right(Align::Center)),
    };
    let height = line * lines as f32;
    let control_rect = Rect::from_min_size(pos2(start, top.y), vec2(CONTROL, height));
    // What is said of it, under the row: `--conf`'s first, then the group's own, then
    // that it applies after a restart; in the warning colour where it is a warning, and
    // announced as it appears where the person's change on another row made it.
    let (note, warning, live) = if row.overridden {
        let mut args = apertrust_ui::i18n::Args::new();
        args.set("flag", CONF_FLAG);
        let text = tr_.text_with("settings-overridden", Some(&args));
        (Some(text), false, false)
    } else if let Some(note) = extra.note {
        (Some(note.text.clone()), note.warning, note.live)
    } else if row.applies == Applies::AfterRestart {
        (Some(tr_.text("settings-after-restart")), false, false)
    } else {
        (None, false, false)
    };
    let mut column = ui.new_child(UiBuilder::new().max_rect(control_rect).layout(layout));
    column.set_min_size(vec2(CONTROL, height));
    column.spacing_mut().item_spacing.x = t.spacing.m;
    let said = note.as_deref();
    if let Some(changed) = column
        .add_enabled_ui(enabled, |ui| {
            control(ui, t, row, &label, control_width, extra, said)
        })
        .inner
    {
        change = Some(changed);
    }
    let mut bottom = label_rect.bottom().max(control_rect.bottom());
    if let Some(note) = note {
        let width = (top.x + content - start).max(0.0);
        let colour = if warning {
            t.colors.warning
        } else {
            t.colors.text_secondary
        };
        let galley = ui.painter().layout(
            note.clone(),
            FontId::proportional(t.text.caption),
            colour,
            width,
        );
        let at = pos2(start, control_rect.bottom());
        let rect = Rect::from_min_size(at, galley.size());
        ui.painter().galley(at, galley, colour);
        let id = egui::Id::unique(("settings note", row.id));
        let said = ui.interact(rect, id, Sense::hover());
        said.widget_info(|| WidgetInfo::labeled(Role::Label, true, &note));
        if live {
            let polite = egui::accesskit::Live::Polite;
            ctx.accesskit_node_builder(said.id, |node| node.set_live(polite));
        }
        bottom = bottom.max(rect.bottom() + t.spacing.xs);
    }
    ui.allocate_rect(
        Rect::from_min_max(top, pos2(top.x + content, bottom)),
        Sense::hover(),
    );
    change
}

/// `row`'s control by its kind, `label` its full name: a checkbox, or `width` wide, a
/// choice, a number with its unit inside it, or a text; for a list of a choice, a checkbox
/// per option, each on a line of its own, named by its option. The graphics card's is its
/// list, as `extra` has it, described to screen readers by `said`, what the row says under
/// it; the API's Automatic says what it picks where `extra` knows. Returns the value the
/// user gave it.
fn control(
    ui: &mut Ui,
    t: &Theme,
    row: &Row,
    label: &str,
    width: f32,
    extra: Extra<'_>,
    said: Option<&str>,
) -> Option<Change> {
    let ctx = ui.ctx().clone();
    let set = |value| Some(Change::Set(row.id, value));
    if let Some(card) = extra.card {
        return card_control(ui, t, row, card, (label, said), width);
    }
    match row.kind {
        Kind::Boolean => {
            let on = row.value.as_ref().and_then(Value::as_bool).unwrap_or(false);
            // The box at the column's start, where the checkbox draws it.
            let mut toggle = ui.new_child(
                UiBuilder::new()
                    .max_rect(ui.max_rect())
                    .layout(Layout::left_to_right(Align::Center)),
            );
            ToggleRow::new("", on)
                .name(label)
                .show(&mut toggle)
                .and_then(|on| set(Value::Boolean(on)))
        }
        Kind::Choice(_) => {
            let (ids, mut names) = options(&ctx, row);
            let automatic = ids.iter().position(|id| *id == Backend::Auto.name());
            if let (Some(words), Some(at)) = (extra.automatic, automatic) {
                names[at] = words.to_owned();
            }
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            let chosen = row.value.as_ref().and_then(Value::as_str);
            let selected = ids.iter().position(|id| Some(*id) == chosen);
            let choice = Choice::new("", &names, selected)
                .name(label)
                .min_box_width(width);
            match boxed(ui, t, choice, width) {
                Some(ChoiceAction::Set(i)) => set(Value::String(ids[i].to_owned())),
                _ => None,
            }
        }
        Kind::Number(number) => {
            let factor = f64::from(number.factor.unwrap_or(1.0));
            let kept = row.value.as_ref().and_then(as_number).unwrap_or(0.0);
            let range = number.range();
            let shown = |n: f32| f64::from(n) * factor;
            let decimals = number.decimals.unwrap_or(2) as i32;
            let shown_decimals = (decimals - factor.log10().round() as i32).max(0) as usize;
            let unit = number.unit.map(|unit| tr(&ctx, &format!("unit-{unit}")));
            let mut field = ValueField::new(
                ui.make_persistent_id(("settings field", row.id)),
                label,
                kept * factor,
                shown(*range.start())..=shown(*range.end()),
            )
            .decimals(shown_decimals)
            .number_only();
            if let Some(unit) = &unit {
                field = field.unit(unit).suffix(unit);
            }
            let typed = ui
                .allocate_ui(vec2(width, t.sizes.field), |ui| {
                    ui.set_width(width);
                    field.show(ui).1
                })
                .inner;
            let typed = typed?;
            let value = typed / factor;
            set(if decimals == 0 {
                Value::Integer(value.round() as i64)
            } else {
                let scale = 10f64.powi(decimals);
                Value::Float((value * scale).round() / scale)
            })
        }
        Kind::Text => {
            let text = row.value.as_ref().and_then(Value::as_str).unwrap_or("");
            let id = ui.make_persistent_id(("settings text", row.id));
            // What it shows while empty, where its words say: `setting-<id>-hint`.
            let words = translator(&ctx);
            let hint = format!("setting-{}-hint", row.words());
            let hint = words.has(&hint).then(|| words.text(&hint));
            let mut field = TextField::new(id, label, text).width(width);
            if let Some(hint) = &hint {
                field = field.hint(hint);
            }
            let (_, typed) = field.show(ui);
            typed.and_then(|typed| set(Value::String(typed)))
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
            let line = t.sizes.panel_line;
            let mut toggled = None;
            for (id, name) in ids.iter().zip(&names) {
                // Each box drawn as a boolean's, with no words of its own, at the column's
                // start.
                let at = ui.cursor().min;
                let rect = Rect::from_min_size(at, vec2(ui.max_rect().width(), line));
                let mut toggle = ui.new_child(
                    UiBuilder::new()
                        .max_rect(rect)
                        .layout(Layout::left_to_right(Align::Center)),
                );
                let on = ticked.contains(id);
                if let Some(on) = ToggleRow::new("", on).name(name).show(&mut toggle) {
                    toggled = Some((*id, on));
                }
                // The option's words after it, which tick it too, as a checkbox's own do.
                let galley = toggle.painter().layout_no_wrap(
                    name.clone(),
                    FontId::proportional(t.text.body),
                    t.colors.text,
                );
                // As far after the box as a checkbox puts its own words.
                let spacing = toggle.spacing();
                let start = at.x + spacing.icon_width + spacing.icon_spacing;
                let words = Rect::from_min_size(
                    pos2(start, at.y + (line - galley.size().y) / 2.0),
                    galley.size(),
                );
                let colour = if toggle.is_enabled() {
                    t.colors.text
                } else {
                    t.colors.text_disabled
                };
                toggle.painter().galley(words.min, galley, colour);
                let id_of = egui::Id::unique(("settings option words", row.id, *id));
                let sense = if toggle.is_enabled() {
                    Sense::click()
                } else {
                    Sense::hover()
                };
                if toggle.interact(words, id_of, sense).clicked() {
                    toggled = Some((*id, !on));
                }
                ui.allocate_space(vec2(ui.max_rect().width(), line));
            }
            // The options ticked after the change, in the type's order.
            let (changed, on) = toggled?;
            let items = ids
                .iter()
                .filter(|id| {
                    if **id == changed {
                        on
                    } else {
                        ticked.contains(id)
                    }
                })
                .map(|id| Value::String((*id).to_owned()));
            set(Value::Array(items.collect()))
        }
        // No other list, and no table, is on a page yet.
        Kind::List { .. } | Kind::Table { .. } => None,
    }
}

/// Draws `choice` as a row's box, at the column's start, as wide as its widest option
/// needs, at least `width` and at most [`CONTROL`]: the row's own gap before it falls in
/// the dot's column, and a longer option is cut, whole in its tooltip.
fn boxed(ui: &mut Ui, t: &Theme, choice: Choice<'_>, width: f32) -> Option<ChoiceAction> {
    let width = choice.box_width(ui).max(width).min(CONTROL) + t.spacing.l;
    let start = ui.cursor().min.x - t.spacing.l;
    let rect = Rect::from_min_size(
        pos2(start, ui.max_rect().top()),
        vec2(width, ui.max_rect().height()),
    );
    let mut boxed = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    choice.show(&mut boxed)
}

/// The graphics card's list (decision 81), `named` its row's full name and what is said
/// under it, which screen readers hear on the box: Automatic, the cards, a card not found,
/// then CPU after a separator, as `card` has them; disabled while they are listed. Choosing
/// Automatic resets the setting, so it leaves the file.
fn card_control(
    ui: &mut Ui,
    t: &Theme,
    row: &Row,
    card: &hardware::Card,
    named: (&str, Option<&str>),
    width: f32,
) -> Option<Change> {
    let (label, said) = named;
    let names: Vec<&str> = card.words.iter().map(String::as_str).collect();
    let missing: Vec<usize> = card.missing.iter().map(|(at, _)| *at).collect();
    let missing_tip = card.missing.as_ref().map_or("", |(_, tip)| tip.as_str());
    let mut choice = Choice::new("", &names, card.selected)
        .name(label)
        .min_box_width(width)
        .enabled(ui.is_enabled() && !card.listing)
        .disabled_options(&missing, missing_tip);
    if let Some((at, tip)) = &card.unlisted {
        choice = choice.unlisted(*at, tip);
    }
    if let Some(at) = card.separator {
        choice = choice.separator_before(at);
    }
    if let Some(said) = said {
        choice = choice.description(said);
    }
    match boxed(ui, t, choice, width)? {
        ChoiceAction::Set(item) => Some(match card.stores(item) {
            Some(stored) => Change::Set(row.id, Value::String(stored.to_owned())),
            None => Change::Reset(row.id),
        }),
        _ => None,
    }
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

    /// The dialog as the host draws it in its window, and what each frame asked: every
    /// change, and those of each frame that asked any.
    struct Bench {
        content: Settings,
        changes: Vec<Change>,
        frames: Vec<Vec<Change>>,
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
            .with_size(vec2(WIDTH, BODY) + vec2(16.0, 16.0))
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
                    let mut frame = Vec::new();
                    for output in drawn.outputs {
                        if let Output::Settings(change) = output {
                            frame.push(change);
                        }
                    }
                    bench.changes.extend(frame.iter().cloned());
                    if !frame.is_empty() {
                        bench.frames.push(frame);
                    }
                },
                Bench {
                    content,
                    changes: Vec::new(),
                    frames: Vec::new(),
                },
            );
        harness.run();
        harness
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
        harness.get_by_role_and_label(Role::CheckBox, "Square thumbnails");
        harness.get_by_role_and_label(Role::Label, "Thumbnails");
        // ↓ moves to the next page that holds rows: Editing.
        harness.get_by_label("Photos").focus();
        harness.run();
        harness.key_press(Key::ArrowDown);
        harness.run();
        assert_eq!(harness.state().content.page, Page::Editing);
        harness.get_by_role_and_label(Role::CheckBox, "Show tab names");
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
        keeper.apply(
            &settings,
            "grid_panel.square_thumbnails",
            Some(&true.into()),
        );
        keeper.flush();
        assert!(
            folder.text().contains("page = \"library\""),
            "{}",
            folder.text()
        );
    }

    /// A change shows darktable's dot, and a double-click on the label, or Reset in its
    /// right-click menu, puts the default back.
    #[test]
    fn a_row_off_its_default_is_dotted_and_reset_from_its_label() {
        let mut harness = dialog();
        let label = "Ask before overwriting an exported file";
        assert!(!dotted(&harness, label));
        harness.get_by_role_and_label(Role::CheckBox, label).click();
        harness.run();
        assert_eq!(
            harness.state().changes,
            [Change::Set("export.ask_before_overwrite", false.into())]
        );
        assert!(dotted(&harness, label));
        let at = harness
            .get_by_role_and_label(Role::Label, label)
            .rect()
            .center();
        double_click(&mut harness, at);
        assert_eq!(
            harness.state().changes.last(),
            Some(&Change::Reset("export.ask_before_overwrite"))
        );
        assert!(!dotted(&harness, label));
        // The interface's size, typed in per cent, is kept as a factor; the menu resets it.
        let (size, field) = ("Interface size", "Interface size, in %");
        harness
            .get_by_role_and_label(Role::TextInput, field)
            .click();
        harness.run();
        harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
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
        // Delete on the label in focus resets it as well.
        harness.get_by_role_and_label(Role::CheckBox, label).click();
        harness.run();
        harness.get_by_role_and_label(Role::Label, label).focus();
        harness.run();
        harness.key_press(Key::Delete);
        harness.run();
        assert_eq!(
            harness.state().changes.last(),
            Some(&Change::Reset("export.ask_before_overwrite"))
        );
    }

    /// A list of a choice is a checkbox per option, one under another in its type's order:
    /// a change sets the whole list, one dot says the row is off its default, one Reset puts
    /// it all back, and the row's help is its label's tooltip.
    #[test]
    fn a_list_of_a_choice_is_a_checkbox_per_option_with_one_dot_and_one_reset() {
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
        let label = harness.get_by_role_and_label(Role::Label, "Log").rect();
        let mut above = label.top() - 1.0;
        // Every box is drawn as a boolean's, on the same page: the same widget, with no
        // words of its own, its left edge the boolean box's, where the choices' boxes start.
        let keep = harness
            .get_by_role_and_label(Role::CheckBox, "Keep a log of this session")
            .rect();
        let ask = harness
            .get_by_role_and_label(Role::CheckBox, "Ask before overwriting an exported file")
            .rect();
        assert_eq!(keep.left(), ask.left());
        for (name, ticked) in options {
            let node = harness.get_by_role_and_label(Role::CheckBox, name);
            let on = node.accesskit_node().toggled() == Some(Toggled::True);
            assert_eq!(on, ticked, "{name}");
            let rect = node.rect();
            assert!(rect.top() > above, "{name} is under the one before");
            assert_eq!(rect.left(), keep.left(), "{name}");
            assert_eq!(
                rect.width(),
                keep.width(),
                "{name}: a box alone, as a boolean's"
            );
            above = rect.top();
        }
        // The option's words, after its box, tick it too.
        let pipe = harness.get_by_role_and_label(Role::CheckBox, "Pipe").rect();
        let words = pos2(pipe.right() + 12.0, pipe.center().y);
        let pipe_on = |harness: &Harness<'_, Bench>| {
            let node = harness.get_by_role_and_label(Role::CheckBox, "Pipe");
            node.accesskit_node().toggled() == Some(Toggled::True)
        };
        for expected in [false, true] {
            harness.run_steps(30);
            harness.hover_at(words);
            press(&mut harness, words, PointerButton::Primary);
            harness.run();
            assert_eq!(pipe_on(&harness), expected, "the words tick their box");
        }
        harness.state_mut().changes.clear();
        assert!(!dotted(&harness, "Log"));
        harness
            .get_by_role_and_label(Role::CheckBox, "Memory")
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
        harness.get_by_role_and_label(Role::CheckBox, "GPU").click();
        harness.run();
        assert_eq!(
            harness.state().changes.last(),
            Some(&Change::Set("log.domains", ids(&with_memory[1..])))
        );
        // One Reset for the whole list.
        let at = harness
            .get_by_role_and_label(Role::Label, "Log")
            .rect()
            .center();
        right_click(&mut harness, at);
        harness.get_by_label("Reset").click();
        harness.run();
        assert_eq!(
            harness.state().changes.last(),
            Some(&Change::Reset("log.domains"))
        );
        assert!(!dotted(&harness, "Log"));
        let gpu = harness.get_by_role_and_label(Role::CheckBox, "GPU");
        assert_eq!(gpu.accesskit_node().toggled(), Some(Toggled::True));
        // The help, in the label's tooltip.
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

    /// A row `--conf` sets is disabled, saying so, and its label resets nothing; a setting
    /// that applies after a restart says so.
    #[test]
    fn a_row_set_by_conf_is_disabled_and_one_after_a_restart_says_so() {
        let conf = ["scale=1.5".to_owned()];
        let (keeper, _) = Keeper::open_in(None, &settings::overrides(&conf).expect("read"));
        let mut harness = dialog_of(&keeper, Builtin::Dark);
        let field = harness.get_by_role_and_label(Role::TextInput, "Interface size, in %");
        assert!(field.accesskit_node().is_disabled());
        harness.get_by_label("Set by --conf for this run");
        let at = harness
            .get_by_role_and_label(Role::Label, "Interface size")
            .rect()
            .center();
        double_click(&mut harness, at);
        assert!(harness.state().changes.is_empty());
        processing(&mut harness);
        harness.get_by_role_and_label(Role::ComboBox, "Graphics card");
        harness.get_by_role_and_label(Role::TextInput, "Memory budget, in MiB");
        let notes = harness
            .query_all_by_label("Applies after a restart")
            .count();
        assert_eq!(notes, 2);
    }

    /// Processing's Hardware acceleration group: Graphics card, Graphics API and Memory
    /// budget, the graphics API drawn on Windows alone, the only platform with a choice of
    /// one (`platforms`; decision 81).
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
            let notes = harness
                .query_all_by_label("Applies after a restart")
                .count();
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

    /// A row is disabled while the boolean it requires is off, and enabled again once it
    /// is on: the cache's limit while the cache is unlimited.
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
        harness
            .get_by_role_and_label(Role::CheckBox, limited)
            .click();
        harness.run();
        let set = Change::Set("cache.limited", Value::Boolean(false));
        assert_eq!(harness.state().changes.last(), Some(&set));
        assert!(disabled(&harness));
        harness
            .get_by_role_and_label(Role::CheckBox, limited)
            .click();
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
                if let Kind::Number(number) = row.kind {
                    ids.extend(number.unit.map(|unit| format!("unit-{unit}")));
                }
                for id in ids {
                    assert!(words.has(&id), "{locale:?}: {id}");
                }
            }
        }
    }

    /// darktable's grid: every label on a page as wide as its widest, at least 176 points,
    /// then 16, the dot's 14 and 4 before each control, whose start is the column's; the
    /// label's target ends with its column.
    #[test]
    fn the_labels_take_the_page_s_widest_and_the_controls_start_after_the_dot() {
        let keeper = Keeper::nowhere().0;
        for locale in [Locale::EnGb, Locale::Uk] {
            let mut harness = dialog_in(&keeper, Builtin::Dark, locale);
            let words = Translator::new(locale);
            for page in [Page::General, Page::Photos, Page::Editing, Page::Processing] {
                let name = words.text(&format!("settings-page-{}", page.id()));
                harness.get_by_role_and_label(Role::Tab, &name).click();
                harness.run();
                listed(&mut harness);
                let rows: Vec<Row> = harness.state().content.rows.clone();
                let (mut widths, mut ends) = (Vec::new(), Vec::new());
                let drawn = |row: &&Row| row.page == page && row.drawn_on(OperatingSystem::Mac);
                for row in rows.iter().filter(drawn) {
                    let label = label_of(&words, row);
                    let rect = harness.get_by_role_and_label(Role::Label, &label).rect();
                    widths.push(rect.width());
                    let start = rect.left() + rect.width() + 34.0;
                    // The graphics card's text is a list of the cards (decision 81).
                    let kind = if row.id == ENGINE_ADAPTER.id {
                        Kind::Choice("")
                    } else {
                        row.kind
                    };
                    match kind {
                        Kind::Number(_) | Kind::Text => {
                            let fields: Vec<egui::Rect> = harness
                                .get_all_by_role(Role::TextInput)
                                .map(|node| node.rect())
                                .collect();
                            let field = fields.iter().find(|field| field.left() == start);
                            let field = field.unwrap_or_else(|| panic!("{locale:?} {}", row.id));
                            ends.push(field.right() - start);
                        }
                        Kind::Choice(_) => {
                            // The row's node holds the gap before its box, which ends it.
                            let choice = harness.get_by_role_and_label(Role::ComboBox, &label);
                            ends.push(choice.rect().right() - start);
                        }
                        _ => {}
                    }
                }
                assert!(
                    widths.windows(2).all(|w| w[0] == w[1]),
                    "{locale:?} {page:?}"
                );
                assert!(widths[0] >= LABEL_MIN, "{locale:?} {page:?}");
                // One width for every box and field on the page, their ends together.
                assert!(
                    ends.windows(2).all(|w| w[0] == w[1]),
                    "{locale:?} {page:?}: {ends:?}"
                );
                assert!(
                    ends.iter().all(|w| (CONTROL_WIDTH..=CONTROL).contains(w)),
                    "{locale:?} {page:?}: {ends:?}"
                );
            }
        }
        // A right-click past a label's column, before the dot, opens nothing.
        let mut harness = dialog();
        let rect = harness.get_by_role_and_label(Role::Label, "Theme").rect();
        right_click(&mut harness, pos2(rect.right() + 8.0, rect.center().y));
        assert!(harness.query_by_label("Reset").is_none());
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
    /// software adapter's own name is never shown (decision 81).
    #[test]
    fn the_graphics_card_is_a_list_of_the_cards_the_api_finds() {
        let mut harness = windows(&Keeper::nowhere().0);
        let automatic = format!("Automatic ({RTX})");
        let after_restart = Some("Applies after a restart".to_owned());
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
        let automatic = format!("Automatic ({M4})");
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
        assert!(
            harness
                .query_all_by_label("Applies after a restart")
                .count()
                > 0
        );
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
        // Automatic picks what `cpu` opens: WARP, with Direct3D 12.
        let api = harness.get_by_role_and_label(Role::ComboBox, "Graphics API");
        let api = api.accesskit_node().value();
        assert_eq!(api.as_deref(), Some("Automatic (Direct3D 12)"));
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
        choose(&mut harness, "Graphics API", "Automatic (Vulkan)");
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
        let note = harness
            .query_all_by_label("Set by --conf for this run")
            .count();
        assert_eq!(note, 1);
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
        let after_restart = Some("Applies after a restart".to_owned());
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

    // How it looks.

    #[test]
    fn the_general_page_in_the_dark_theme() {
        let mut harness = dialog();
        let ask = "Ask before overwriting an exported file";
        harness.get_by_role_and_label(Role::CheckBox, ask).click();
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
