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
//! - "Applies after a restart" is said under the control of a setting that does.
//! - Every change applies at once and is kept, with no OK or Apply: it goes to the app as a
//!   [`Change`], which keeps it through the settings' keeper and shows it in the main
//!   window; the theme and the language are shown in this dialog's own window at once too.
//! - A page that holds nothing yet is listed dimmed and cannot be chosen. The page last
//!   shown is kept, `settings_window.page`.
//!
//! It is a [`Dialog`] (decision 41), which the dialog host draws in a window of its own:
//! modeless on the Mac and modal on Windows and Linux.

use apertrust_base::settings::{Applies, CONF_FLAG, Choice as _, Declared, Kind, Page};
use apertrust_ui::components::{Choice, ChoiceAction, TextField, ToggleRow, ValueField};
use apertrust_ui::popup::{self, Entry, Item, Request};
use apertrust_ui::{Builtin, Locale, Theme, fonts, theme, tr, translator};
use eframe::egui::os::OperatingSystem;
use eframe::egui::{
    self, Align, FontFamily, FontId, Key, Layout, Modifiers, Rect, Role, ScrollArea, Sense, Ui,
    UiBuilder, WidgetInfo, pos2, vec2,
};
use toml::Value;

use crate::dialog_host::{Asks, Dialog, Form, Keys, Look, Output};
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
        })
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
    pub change: Option<Change>,
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
        let change = asked.change.map(Output::Settings);
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
        };
        let kept = keeper.page();
        if dialog.holds(kept) {
            dialog.page = kept;
        }
        dialog
    }

    /// The dialog with Storage's hand-built part showing `view`.
    pub fn with_storage(mut self, view: storage::View) -> Settings {
        self.set_storage(view);
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

    /// Whether `page` holds a row, so it can be chosen.
    pub fn holds(&self, page: Page) -> bool {
        self.rows.iter().any(|row| row.page == page)
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
                asked.change = Some(Change::Set(
                    settings::registry::SETTINGS_WINDOW_PAGE.id,
                    Value::String(page.id().to_owned()),
                ));
            }
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
                                let (change, storage) = self.page_rows(ui, &t);
                                if let Some(change) = change {
                                    asked.change = Some(change);
                                }
                                if let Some(storage::Asked::Clear(kind)) = &storage {
                                    self.pressed.push(*kind);
                                }
                                asked.storage.extend(storage);
                            });
                    },
                );
            });
        });
        if let Some(change) = &asked.change {
            self.take(change, &ctx);
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
            let enabled = self.holds(page);
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
                    moved = self.next(page, 1);
                } else if step(Key::ArrowUp) {
                    moved = self.next(page, -1);
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

    /// The page that can be chosen next to `from`, `by` 1 after it or −1 before it.
    fn next(&self, from: Page, by: isize) -> Option<Page> {
        let at = Page::ALL.iter().position(|page| *page == from)? as isize;
        let mut i = at + by;
        while (0..Page::ALL.len() as isize).contains(&i) {
            let page = Page::ALL[i as usize];
            if self.holds(page) {
                return Some(page);
            }
            i += by;
        }
        None
    }

    /// The shown page's groups, each under its title, in the order of their first rows,
    /// and their rows; on Storage, its Folders group first and its In the cache group last.
    /// Returns what the user changed, and what Storage asked.
    fn page_rows(&self, ui: &mut Ui, t: &Theme) -> (Option<Change>, Option<storage::Asked>) {
        let ctx = ui.ctx().clone();
        let mut groups: Vec<&str> = Vec::new();
        for row in self.rows.iter().filter(|row| row.page == self.page) {
            if !groups.contains(&row.group) {
                groups.push(row.group);
            }
        }
        let title = FontId::new(t.text.section_title, FontFamily::Name(fonts::MEDIUM.into()));
        let content = ui.available_width();
        let words = translator(&ctx);
        let storage = self.page == Page::Storage;
        let mut labels: Vec<String> = self
            .rows
            .iter()
            .filter(|row| row.page == self.page)
            .map(|row| label_of(&words, row))
            .collect();
        if storage {
            labels.extend(storage::labels(&words));
        }
        let label_width = label_column(ui, t, &labels, content);
        let control_width = control_column(ui, &self.rows, self.page);
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
            let rows = self.rows.iter().filter(|row| row.group == group);
            for row in rows.filter(|row| row.page == self.page) {
                let widths = (label_width, control_width, content);
                if let Some(changed) = draw_row(ui, t, row, self.enabled(row), widths) {
                    change = Some(changed);
                }
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
/// as the widest box a choice there needs in `ui`'s language, up to [`CONTROL`].
fn control_column(ui: &Ui, rows: &[Row], page: Page) -> f32 {
    rows.iter()
        .filter(|row| row.page == page && matches!(row.kind, Kind::Choice(_)))
        .map(|row| {
            let (_, names) = options(ui.ctx(), row);
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            Choice::new("", &names, None).box_width(ui)
        })
        .fold(CONTROL_WIDTH, f32::max)
        .min(CONTROL)
}

/// Draws `row` across `content` points: its label in the first `label_width`, wrapped
/// where longer, the dot while it is off its default, its control, and what is said of it
/// under the row, wrapping at the content's end; disabled unless `enabled`. `widths` are the
/// label's, the control's and the content's. Returns what the user changed.
fn draw_row(
    ui: &mut Ui,
    t: &Theme,
    row: &Row,
    enabled: bool,
    widths: (f32, f32, f32),
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
    let mut column = ui.new_child(UiBuilder::new().max_rect(control_rect).layout(layout));
    column.set_min_size(vec2(CONTROL, height));
    column.spacing_mut().item_spacing.x = t.spacing.m;
    if let Some(changed) = column
        .add_enabled_ui(enabled, |ui| control(ui, t, row, &label, control_width))
        .inner
    {
        change = Some(changed);
    }
    let mut bottom = label_rect.bottom().max(control_rect.bottom());
    // What is said of it, under the row, wrapping at the content's end.
    let note = if row.overridden {
        let mut args = apertrust_ui::i18n::Args::new();
        args.set("flag", CONF_FLAG);
        Some(tr_.text_with("settings-overridden", Some(&args)))
    } else if row.applies == Applies::AfterRestart {
        Some(tr_.text("settings-after-restart"))
    } else {
        None
    };
    if let Some(note) = note {
        let width = (top.x + content - start).max(0.0);
        let galley = ui.painter().layout(
            note.clone(),
            FontId::proportional(t.text.caption),
            t.colors.text_secondary,
            width,
        );
        let at = pos2(start, control_rect.bottom());
        let rect = Rect::from_min_size(at, galley.size());
        ui.painter().galley(at, galley, t.colors.text_secondary);
        let id = egui::Id::unique(("settings note", row.id));
        let said = ui.interact(rect, id, Sense::hover());
        said.widget_info(|| WidgetInfo::labeled(Role::Label, true, &note));
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
/// per option, each on a line of its own, named by its option. Returns the value the user
/// gave it.
fn control(ui: &mut Ui, t: &Theme, row: &Row, label: &str, width: f32) -> Option<Change> {
    let ctx = ui.ctx().clone();
    let set = |value| Some(Change::Set(row.id, value));
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
            let (ids, names) = options(&ctx, row);
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            let chosen = row.value.as_ref().and_then(Value::as_str);
            let selected = ids.iter().position(|id| Some(*id) == chosen);
            let choice = Choice::new("", &names, selected)
                .name(label)
                .min_box_width(width);
            // The box at the column's start: the row's own gap before it falls in the
            // dot's column.
            let width = choice.box_width(ui).max(width) + t.spacing.l;
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
            match choice.show(&mut boxed) {
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

    /// The dialog as the host draws it in its window, and what each frame asked.
    struct Bench {
        content: Settings,
        changes: Vec<Change>,
    }

    /// The dialog over `keeper`'s settings, without Close as on the Mac, in `which` theme;
    /// the language's row reads British English, whatever the system's.
    fn dialog_of(keeper: &Keeper, which: Builtin) -> Harness<'static, Bench> {
        dialog_in(keeper, which, Locale::EnGb)
    }

    /// The dialog as [`dialog_of`] draws it, in `locale`.
    fn dialog_in(keeper: &Keeper, which: Builtin, locale: Locale) -> Harness<'static, Bench> {
        let mut content = Settings::new(keeper, false);
        for row in content.rows.iter_mut().filter(|row| row.id == "locale") {
            row.default = Some("en-GB".into());
            if !row.overridden {
                row.value = Some("en-GB".into());
            }
        }
        let mut harness = Harness::builder()
            .with_size(vec2(WIDTH, BODY) + vec2(16.0, 16.0))
            .with_os(OperatingSystem::Mac)
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
                    for output in drawn.outputs {
                        if let Output::Settings(change) = output {
                            bench.changes.push(change);
                        }
                    }
                },
                Bench {
                    content,
                    changes: Vec::new(),
                },
            );
        harness.run();
        harness
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
            ("Library", false, false),
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
        harness.get_by_label("Library").click();
        harness.run();
        assert_eq!(
            harness.state().changes,
            [Change::Set("settings_window.page", "library".into())]
        );
        harness.get_by_role_and_label(Role::CheckBox, "Square thumbnails");
        harness.get_by_role_and_label(Role::Label, "Thumbnails");
        // ↓ moves to the next page that holds rows: Editing.
        harness.get_by_label("Library").focus();
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
        harness.get_by_label("Processing").click();
        harness.run();
        harness.get_by_role_and_label(Role::TextInput, "Graphics card");
        harness.get_by_role_and_label(Role::TextInput, "Memory budget, in MiB");
        let notes = harness
            .query_all_by_label("Applies after a restart")
            .count();
        assert_eq!(notes, 2);
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
            for page in [
                Page::General,
                Page::Library,
                Page::Editing,
                Page::Processing,
            ] {
                let name = words.text(&format!("settings-page-{}", page.id()));
                harness.get_by_role_and_label(Role::Tab, &name).click();
                harness.run();
                let rows: Vec<Row> = harness.state().content.rows.clone();
                let (mut widths, mut ends) = (Vec::new(), Vec::new());
                for row in rows.iter().filter(|row| row.page == page) {
                    let label = label_of(&words, row);
                    let rect = harness.get_by_role_and_label(Role::Label, &label).rect();
                    widths.push(rect.width());
                    let start = rect.left() + rect.width() + 34.0;
                    match row.kind {
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
        harness.get_by_label("Processing").click();
        harness.event(Event::PointerGone);
        harness.run();
        harness.snapshot("settings-processing-dark");
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
