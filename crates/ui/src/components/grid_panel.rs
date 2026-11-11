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

//! The grid panel: the open folder's photos as `GridCell`s along the bottom of the window,
//! or in a column at its left or right, with the grid bar (`docs/gui.md`, The grid panel,
//! The grid bar): at the bottom a narrow column at the panel's left end, its buttons
//! stacked, and at a side a row across the panel's top. In the order the caller gives;
//! only the visible cells are drawn, so a folder of thousands costs what a screenful does.
//!
//! Each cell draws its thumbnail where the caller gives one, and the panel reports which
//! photos it drew ([`GridPanelOutput::shown`]), so the caller asks for the thumbnails of
//! those on screen as it scrolls.
//!
//! A click opens a photo; Shift-click selects a range, Ctrl or Cmd-click adds or removes
//! one. With a cell focused, ← and → move the focus to the next or previous photo, Home and
//! End to the first and last, and Enter or Space opens the one focused; the panel scrolls
//! to keep it in sight. The active photo's cell is scrolled into view when it changes.
//! With no photos it is drawn empty and, where the app gives one, says why: a struck-through
//! photo over the message, and a hint under it where the app gives one too.
//!
//! The grid bar holds Sort and Labels at its start and Zoom at its end: Sort and Zoom each
//! open their menu through the popup service, the one in use checked, and Labels shows or
//! hides the cells' captions. The panel reports what was chosen ([`GridBarAction`]) and
//! knows no photos, so the caller sorts them and sizes the cells. A cell's height counts
//! its caption while labels show, and so do the panel's rows; what the user did on a
//! caption's first line is reported with its photo ([`GridPanelOutput::cell`]).

use std::ops::Range;

use apertrust_base::settings::Choice;
use egui::accesskit::HasPopup;
use egui::{
    Align, EventFilter, Id, Key, Layout, Modifiers, Rect, Response, Role, ScrollArea, Sense,
    Stroke, Ui, UiBuilder, WidgetInfo, pos2, vec2,
};

use super::side_panel::{restore, scroll_area};
use super::{
    CellAction, CellPicture, CellRating, CellStatus, CentralBar, GridCell, IconButton, pull_down,
};
use crate::i18n::Args;
use crate::popup::{self, Answer, Entry, Item, Request, Submenu};
use crate::{Icon, theme, tr, translator};

/// One photo in the panel.
#[derive(Clone, Copy, Debug)]
pub struct Thumb<'a> {
    /// The file name, with its extension.
    pub name: &'a str,
    pub edited: bool,
    /// What its status mark says, if it has one.
    pub status: Option<CellStatus<'a>>,
    pub selected: bool,
    /// Its thumbnail, once it has come.
    pub picture: Option<CellPicture>,
    /// Its rating, reject and colour labels, where the library has them.
    pub rating: Option<CellRating>,
}

/// What the grid panel's photos are sorted by (`docs/gui.md`, The grid bar).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SortKey {
    /// The camera's time of the shot.
    CaptureTime,
    /// The file name, as the file browser orders it.
    FileName,
    /// Photos without an edit first, then those with one.
    Edited,
    /// Rejected photos first, then no stars, then 1 to 5; not built until the ratings.
    Rating,
}

impl SortKey {
    pub const ALL: [SortKey; 4] = [
        SortKey::CaptureTime,
        SortKey::FileName,
        SortKey::Edited,
        SortKey::Rating,
    ];
}

/// The sort as settings keep it (decision 43).
impl Choice for SortKey {
    const OPTIONS: &'static [(SortKey, &'static str)] = &[
        (SortKey::CaptureTime, "capture-time"),
        (SortKey::FileName, "file-name"),
        (SortKey::Edited, "edited"),
        (SortKey::Rating, "rating"),
    ];
}

/// The grid panel's sort: its key and its direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Sort {
    pub key: SortKey,
    /// Whether the key runs from its end; ties keep their order either way.
    pub descending: bool,
}

/// The thumbnails' size: the side of a cell's square.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThumbnailSize {
    Small,
    Medium,
    Large,
    ExtraLarge,
}

impl ThumbnailSize {
    pub const ALL: [ThumbnailSize; 4] = [
        ThumbnailSize::Small,
        ThumbnailSize::Medium,
        ThumbnailSize::Large,
        ThumbnailSize::ExtraLarge,
    ];

    /// The side of a cell's square, in points.
    pub fn side(self) -> f32 {
        match self {
            ThumbnailSize::Small => 84.0,
            ThumbnailSize::Medium => GridCell::MEDIUM,
            ThumbnailSize::Large => 176.0,
            ThumbnailSize::ExtraLarge => 256.0,
        }
    }

    /// A cell's size: its square, and its caption under it while `labels` show.
    pub fn cell(self, labels: bool) -> egui::Vec2 {
        let side = self.side();
        vec2(side, side + GridCell::caption_height(side, labels))
    }
}

/// The size as settings keep it, by name, so its points can be retuned.
impl Choice for ThumbnailSize {
    const OPTIONS: &'static [(ThumbnailSize, &'static str)] = &[
        (ThumbnailSize::Small, "small"),
        (ThumbnailSize::Medium, "medium"),
        (ThumbnailSize::Large, "large"),
        (ThumbnailSize::ExtraLarge, "extra-large"),
    ];
}

/// Where the grid panel is in the window (`docs/gui.md`, Its position).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GridPosition {
    /// Along the bottom, under the central panel: rows of cells, scrolling sideways.
    Bottom,
    /// A column at the window's left: columns of cells, scrolling up and down.
    Left,
    /// A column between the central panel and the side panel.
    Right,
}

impl GridPosition {
    pub const ALL: [GridPosition; 3] = [
        GridPosition::Left,
        GridPosition::Right,
        GridPosition::Bottom,
    ];

    /// Whether the panel is a column, its strip running up and down.
    pub fn is_column(self) -> bool {
        self != GridPosition::Bottom
    }
}

/// The position as settings keep it.
impl Choice for GridPosition {
    const OPTIONS: &'static [(GridPosition, &'static str)] = &[
        (GridPosition::Bottom, "bottom"),
        (GridPosition::Left, "left"),
        (GridPosition::Right, "right"),
    ];
}

/// What the panel did this frame.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct GridPanelOutput {
    /// What the user did to the photos, if anything.
    pub action: Option<GridPanelAction>,
    /// What was chosen in the grid bar's menus, if anything.
    pub bar: Option<GridBarAction>,
    /// The photos whose cells were drawn, in sight or partly: those whose thumbnails are
    /// wanted now. Empty with no photos.
    pub shown: Range<usize>,
    /// What the user did on a photo's caption, and the photo.
    pub cell: Option<(usize, CellAction)>,
}

/// What the user did to the grid panel this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridPanelAction {
    /// Open the photo and select only it.
    Open(usize),
    /// Select the range from the active photo to this one.
    SelectRange(usize),
    /// Add the photo to the selection, or take it out.
    ToggleSelected(usize),
}

/// What was chosen in the grid bar's menus. Each is an action of the app's registry, which
/// the View menu's items run too: its id is the action's ([`GridBarAction::id`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridBarAction {
    Sort(SortKey),
    /// Descending, or ascending.
    Descending(bool),
    Size(ThumbnailSize),
    /// From the options menu, a right-click on the grid bar.
    Position(GridPosition),
    /// Labels: show the cells' captions, or hide them.
    Labels,
}

impl GridBarAction {
    /// Every choice, in the menus' order.
    pub const ALL: [GridBarAction; 14] = [
        GridBarAction::Sort(SortKey::CaptureTime),
        GridBarAction::Sort(SortKey::FileName),
        GridBarAction::Sort(SortKey::Edited),
        GridBarAction::Sort(SortKey::Rating),
        GridBarAction::Descending(false),
        GridBarAction::Descending(true),
        GridBarAction::Size(ThumbnailSize::Small),
        GridBarAction::Size(ThumbnailSize::Medium),
        GridBarAction::Size(ThumbnailSize::Large),
        GridBarAction::Size(ThumbnailSize::ExtraLarge),
        GridBarAction::Position(GridPosition::Left),
        GridBarAction::Position(GridPosition::Right),
        GridBarAction::Position(GridPosition::Bottom),
        GridBarAction::Labels,
    ];

    /// The id of the app's action that does the same, which is its menu item's id, and
    /// `action-<id>` its words.
    pub fn id(self) -> &'static str {
        match self {
            GridBarAction::Sort(SortKey::CaptureTime) => "sort-capture-time",
            GridBarAction::Sort(SortKey::FileName) => "sort-file-name",
            GridBarAction::Sort(SortKey::Edited) => "sort-edited",
            GridBarAction::Sort(SortKey::Rating) => "sort-rating",
            GridBarAction::Descending(false) => "sort-ascending",
            GridBarAction::Descending(true) => "sort-descending",
            GridBarAction::Size(ThumbnailSize::Small) => "thumbnails-small",
            GridBarAction::Size(ThumbnailSize::Medium) => "thumbnails-medium",
            GridBarAction::Size(ThumbnailSize::Large) => "thumbnails-large",
            GridBarAction::Size(ThumbnailSize::ExtraLarge) => "thumbnails-extra-large",
            GridBarAction::Position(GridPosition::Left) => "grid-position-left",
            GridBarAction::Position(GridPosition::Right) => "grid-position-right",
            GridBarAction::Position(GridPosition::Bottom) => "grid-position-bottom",
            GridBarAction::Labels => "show-labels",
        }
    }

    /// The choice whose id is `id`.
    pub fn from_id(id: &str) -> Option<GridBarAction> {
        GridBarAction::ALL
            .into_iter()
            .find(|action| action.id() == id)
    }

    /// Its menu item: its action's words, checked while it is what the panel holds. Rating
    /// is drawn disabled until the ratings come.
    fn item<'a, F: Fn(usize) -> Thumb<'a>>(self, ui: &Ui, panel: &GridPanel<'a, F>) -> Entry {
        let on = match self {
            GridBarAction::Sort(key) => panel.sort.key == key,
            GridBarAction::Descending(descending) => panel.sort.descending == descending,
            GridBarAction::Size(chosen) => panel.size == chosen,
            GridBarAction::Position(chosen) => panel.position == chosen,
            GridBarAction::Labels => panel.labels,
        };
        let words = tr(ui.ctx(), &format!("action-{}", self.id()));
        Item::new(self.id(), words)
            .checked(on)
            .enabled(self != GridBarAction::Sort(SortKey::Rating))
            .into()
    }
}

/// The panel along the bottom, or a column at a side, for `count` photos; `thumb(i)` gives
/// the i-th, asked for only the visible ones.
pub struct GridPanel<'a, F: Fn(usize) -> Thumb<'a>> {
    count: usize,
    thumb: F,
    active: Option<usize>,
    empty: Option<&'a str>,
    hint: Option<&'a str>,
    sort: Sort,
    size: ThumbnailSize,
    lines: usize,
    position: GridPosition,
    labels: bool,
    square: bool,
}

/// The space before the first line of cells and after the last, inside the panel, for
/// their outlines and focus rings.
const TOP: f32 = 8.0;
/// Between cells: room for the active one's ring and the focus ring.
pub(super) const GAP: f32 = 8.0;
/// After the cells: the scroll bar and the room round it.
const SCROLL_BAR: f32 = 8.0 + 6.0;

impl<'a, F: Fn(usize) -> Thumb<'a>> GridPanel<'a, F> {
    /// `active` is the photo being edited, if it is in the panel.
    pub fn new(count: usize, active: Option<usize>, thumb: F) -> Self {
        GridPanel {
            count,
            thumb,
            active,
            empty: None,
            hint: None,
            sort: Sort {
                key: SortKey::CaptureTime,
                descending: false,
            },
            size: ThumbnailSize::Medium,
            lines: 1,
            position: GridPosition::Bottom,
            labels: true,
            square: false,
        }
    }

    /// What the panel says while it has no photos, translated; nothing by default.
    pub fn empty(mut self, message: &'a str) -> Self {
        self.empty = Some(message);
        self
    }

    /// What to do about it, translated, under the message while it has no photos; nothing
    /// by default.
    pub fn empty_hint(mut self, hint: &'a str) -> Self {
        self.hint = Some(hint);
        self
    }

    /// The sort the grid bar's Sort shows checked; the caller gives the photos in it.
    pub fn sort(mut self, sort: Sort) -> Self {
        self.sort = sort;
        self
    }

    /// The thumbnails' size, which Zoom shows checked; Medium by default.
    pub fn size(mut self, size: ThumbnailSize) -> Self {
        self.size = size;
        self
    }

    /// How many lines of cells it shows, at least one; one by default: rows at the bottom,
    /// each column filled from the top before the next, and columns at a side, each row
    /// filled from the left before the next, so the photos read along the strip.
    pub fn lines(mut self, lines: usize) -> Self {
        self.lines = lines.max(1);
        self
    }

    /// Where it is in the window, which the options menu shows checked; the bottom by
    /// default. At a side it fills the room it is given, as a column.
    pub fn position(mut self, position: GridPosition) -> Self {
        self.position = position;
        self
    }

    /// Whether the cells show their captions, which Labels shows on; on by default.
    pub fn labels(mut self, labels: bool) -> Self {
        self.labels = labels;
        self
    }

    /// Whether each picture fills its square, cut at its longer sides; off by default.
    pub fn square(mut self, square: bool) -> Self {
        self.square = square;
        self
    }

    pub fn show(self, ui: &mut Ui) -> GridPanelOutput {
        let t = theme(ui.ctx());
        let column = self.position.is_column();
        let height = if column {
            ui.available_height()
        } else {
            grid_panel_height(ui.ctx(), self.size, self.labels, self.lines)
        };
        let (strip, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        ui.painter().rect_filled(strip, 0.0, t.colors.surface);
        // The separator on the side towards the central panel, and what lies inside it.
        let line = Stroke::new(t.shape.separator_width, t.colors.separator);
        let half = t.shape.separator_width / 2.0;
        let mut inner = strip;
        match self.position {
            GridPosition::Bottom => {
                ui.painter()
                    .hline(strip.x_range(), strip.top() + half, line);
                inner.min.y += t.shape.separator_width;
            }
            GridPosition::Left => {
                ui.painter()
                    .vline(strip.right() - half, strip.y_range(), line);
                inner.max.x -= t.shape.separator_width;
            }
            GridPosition::Right => {
                ui.painter()
                    .vline(strip.left() + half, strip.y_range(), line);
                inner.min.x += t.shape.separator_width;
            }
        }
        // The grid bar: a row across a column's top, a column at a row's left end.
        let header = if column {
            Rect::from_min_size(inner.min, vec2(inner.width(), CentralBar::height(ui.ctx())))
        } else {
            Rect::from_min_size(inner.min, vec2(BAR_COLUMN, inner.height()))
        };
        let bar_id = response.id.with("bar");

        let name = tr(ui.ctx(), "grid-panel");
        response.widget_info(|| WidgetInfo::labeled(Role::List, true, &name));
        // The node exists before its cells, so they hang under it (egui parents a node to the
        // nearest ancestor that has one when it is made).
        ui.ctx().accesskit_node_builder(response.id, |_| {});
        let list = response.id;
        // A row runs from the grid bar to the panel's end, inset at both; a column from the
        // grid bar to the bottom.
        let area = if column {
            Rect::from_min_max(pos2(inner.left(), header.bottom()), inner.max)
        } else {
            Rect::from_min_max(
                pos2(header.right() + t.spacing.l, inner.top()),
                pos2(inner.right() - t.spacing.l, inner.bottom()),
            )
        };
        if self.count == 0 {
            if let Some(message) = self.empty {
                empty(ui, area, message, self.hint);
            }
            return GridPanelOutput {
                bar: self.bar(ui, header, bar_id),
                ..GridPanelOutput::default()
            };
        }

        let cells = Cells {
            count: self.count,
            lines: self.lines,
            cell: self.size.cell(self.labels),
            column,
        };
        let cell_id = |i: usize| list.with(i);
        // The focus a key asked for is given when the cell is drawn; the active photo is
        // scrolled to once it changes, or once the cells' size or lines do.
        let pending_id = list.with("goto");
        let seen_id = list.with("active");
        let goto: Option<usize> = ui.data(|d| d.get_temp(pending_id));
        let now = (
            self.active,
            self.size,
            self.lines,
            self.position,
            self.labels,
        );
        let seen: Option<(Option<usize>, ThumbnailSize, usize, GridPosition, bool)> =
            ui.data(|d| d.get_temp(seen_id));
        let changed = seen != Some(now);
        ui.data_mut(|d| d.insert_temp(seen_id, now));

        let mut action = None;
        let mut cell_action = None;
        let mut moved = None;
        let mut shown = 0..0;
        // The cells are the list's children for screen readers, not the window's.
        let build = egui::UiBuilder::new()
            .max_rect(area)
            .accessibility_parent(list);
        let mut child = ui.new_child(build);
        // A plain wheel scrolls a row of cells sideways, as it does a column up and down.
        child.style_mut().always_scroll_the_only_direction = true;
        // The side panel's quiet scroll bar, along the strip's far edge while the cells
        // overflow the panel.
        scroll_area(&mut child, &t, |ui| {
            ScrollArea::new([!column, column])
                .id_salt(("grid panel", column))
                .auto_shrink([false, !column])
                .show_viewport(ui, |ui, viewport| {
                    restore(ui, |ui| {
                        let (all, _) = ui.allocate_exact_size(cells.size(), Sense::hover());
                        let origin = all.min;
                        // Room round a cell scrolled to, for its rings.
                        let room = if column {
                            vec2(0.0, GAP)
                        } else {
                            vec2(GAP, 0.0)
                        };
                        if changed && let Some(active) = self.active.filter(|a| *a < self.count) {
                            ui.scroll_to_rect(cells.rect(origin, active).expand2(room), None);
                        }
                        let visible = cells.visible(viewport);
                        shown = visible.clone();
                        for i in visible.clone() {
                            let thumb = (self.thumb)(i);
                            let rect = cells.rect(origin, i);
                            let id = cell_id(i);
                            if goto == Some(i) {
                                ui.memory_mut(|m| m.request_focus(id));
                                ui.data_mut(|d| d.remove::<usize>(pending_id));
                            }
                            let cell = GridCell::new(thumb.name)
                                .side(self.size.side())
                                .labels(self.labels)
                                .square(self.square)
                                .rating(thumb.rating)
                                .picture(thumb.picture)
                                .edited(thumb.edited)
                                .status(thumb.status)
                                .selected(thumb.selected)
                                .active(self.active == Some(i))
                                .show_at(ui, rect, id);
                            if let Some(done) = cell.action {
                                cell_action = Some((i, done));
                            }
                            let response = cell.response;
                            if response.has_focus() {
                                moved = moved.or(keyed(ui, id, i, &cells));
                            }
                            if response.clicked() {
                                let modifiers = ui.input(|input| input.modifiers);
                                action = Some(if modifiers.shift {
                                    GridPanelAction::SelectRange(i)
                                } else if modifiers.command {
                                    GridPanelAction::ToggleSelected(i)
                                } else {
                                    GridPanelAction::Open(i)
                                });
                            }
                        }
                        // A key moved the focus: to a cell in sight now, else after scrolling to it.
                        if let Some(target) = moved {
                            let rect = cells.rect(origin, target);
                            if visible.contains(&target) {
                                ui.memory_mut(|m| m.request_focus(cell_id(target)));
                            } else {
                                ui.data_mut(|d| d.insert_temp(pending_id, target));
                            }
                            ui.scroll_to_rect(rect.expand2(room), Some(Align::Center));
                            ui.ctx().request_repaint();
                        }
                    })
                });
        });
        // After the cells, so Tab reaches them first, then the grid bar's buttons.
        let bar = self.bar(ui, header, bar_id);
        GridPanelOutput {
            action,
            bar,
            shown,
            cell: cell_action,
        }
    }

    /// The grid bar in `header`, its toolbar's id `id`: Sort, Labels and Zoom, drawn
    /// disabled with no photos, and its options menu, Position, on a right-click anywhere
    /// on it. At a side it is a row across the panel's top, the central bar's separator
    /// under it, so the two bars' lines meet, Sort and Labels at its left end and Zoom at
    /// its right; at the bottom a column at the panel's left end, the same separator
    /// between it and the cells, Sort and Labels at its top and Zoom at its bottom.
    /// Returns what was chosen in their menus, or Labels pressed.
    fn bar(&self, ui: &mut Ui, header: Rect, id: Id) -> Option<GridBarAction> {
        let t = theme(ui.ctx());
        let ctx = ui.ctx().clone();
        let stacked = !self.position.is_column();
        let line = Stroke::new(t.shape.separator_width, t.colors.separator);
        let half = t.shape.separator_width / 2.0;
        if stacked {
            ui.painter()
                .vline(header.right() - half, header.y_range(), line);
        } else {
            ui.painter()
                .hline(header.x_range(), header.bottom() - half, line);
        }
        // Clicks, for the options menu, but no Tab stop of its own.
        let toolbar = ui.interact(header, id, Sense::CLICK);
        let name = tr(&ctx, "grid-bar");
        toolbar.widget_info(|| WidgetInfo::labeled(Role::Toolbar, true, &name));
        ctx.accesskit_node_builder(toolbar.id, |_| {});
        let options = self.options(ui, &toolbar);
        let enabled = self.count > 0;
        let (sort, size) = (self.sort, self.size);
        let words = translator(&ctx);
        let mut args = Args::new();
        args.set("key", id_of(sort.key));
        args.set("descending", if sort.descending { "yes" } else { "no" });
        args.set("size", id_of(size));
        let text = |id: &str| words.text_with(id, Some(&args));
        let (inner, layout) = if stacked {
            let inner = header.shrink2(vec2(0.0, t.spacing.xs));
            (inner, Layout::top_down(Align::Center))
        } else {
            let inner = header.shrink2(vec2(t.spacing.m, 0.0));
            (inner, Layout::left_to_right(Align::Center))
        };
        // A child that allocates nothing in the panel, which has its size already.
        let build = UiBuilder::new()
            .max_rect(inner)
            .layout(layout)
            .accessibility_parent(toolbar.id);
        let ui = &mut ui.new_child(build);
        {
            ui.spacing_mut().item_spacing = vec2(t.spacing.xxs, t.spacing.xs);
            let sort_icon = if sort.descending {
                Icon::SortDescending
            } else {
                Icon::SortAscending
            };
            // A button that opens its menu of `actions`, reporting the one chosen.
            let menu = |ui: &mut Ui, icon, name: &str, tip: &str, value: &str, actions: &[_]| {
                let button = IconButton::new(icon, name)
                    .tip(tip)
                    .enabled(enabled)
                    .size(BUTTON)
                    .show(ui);
                ctx.accesskit_node_builder(button.id, |node| {
                    node.set_value(value);
                    node.set_has_popup(HasPopup::Menu);
                });
                let picked = pull_down(ui, &button, name, || {
                    let mut entries = Vec::new();
                    for (i, action) in actions.iter().enumerate() {
                        // Sort's direction after a line.
                        if i == 4 {
                            entries.push(Entry::Separator);
                        }
                        entries.push(GridBarAction::item(*action, ui, self));
                    }
                    entries
                });
                picked.as_deref().and_then(GridBarAction::from_id)
            };
            let sort_name = tr(&ctx, "grid-sort");
            let sorted = menu(
                ui,
                sort_icon,
                &sort_name,
                &text("grid-sort-tip"),
                &text("grid-sort-value"),
                &GridBarAction::ALL[..6],
            );
            // Labels after Sort: a toggle, its eye filled while the captions show.
            let labels = IconButton::new(Icon::Labels, &tr(&ctx, "grid-labels"))
                .toggled(self.labels)
                .enabled(enabled)
                .size(BUTTON)
                .show(ui)
                .clicked()
                .then_some(GridBarAction::Labels);
            // Zoom at the bar's far end: the column's bottom, the row's right.
            let room = if stacked {
                ui.available_height()
            } else {
                ui.available_width()
            };
            ui.add_space((room - BUTTON).max(0.0));
            let zoom_name = tr(&ctx, "grid-thumbnail-size");
            let zoomed = menu(
                ui,
                Icon::ZoomIn,
                &zoom_name,
                &text("grid-thumbnail-size-tip"),
                &text("grid-thumbnail-size-value"),
                &GridBarAction::ALL[6..10],
            );
            let chosen = sorted.or(labels).or(zoomed);
            chosen.or(options)
        }
    }

    /// The grid bar's options menu, hung from the pointer where `toolbar` was right-clicked:
    /// Position › Left, Right, Bottom, the one in use checked. Returns what was chosen.
    fn options(&self, ui: &Ui, toolbar: &Response) -> Option<GridBarAction> {
        let ctx = ui.ctx();
        let id = toolbar.id.with("options");
        if toolbar.secondary_clicked()
            && let Some(at) = toolbar.interact_pointer_pos()
        {
            let positions = GridBarAction::ALL[10..13]
                .iter()
                .map(|action| action.item(ui, self))
                .collect();
            let entries =
                vec![Submenu::new("position", tr(ctx, "grid-position"), positions).into()];
            let anchor = Rect::from_min_size(at, vec2(0.0, 0.0));
            let request = Request::pull_down(anchor, tr(ctx, "grid-bar-options"), entries);
            popup::open(ctx, id, request);
        }
        if popup::is_open(ctx, id) {
            popup::show(ctx);
        }
        match popup::answer(ctx, id) {
            Some(Answer::Chosen(chosen)) => GridBarAction::from_id(&chosen),
            _ => None,
        }
    }
}

/// The panel's empty state, centred in `area`: a struck-through photo, `message` under it
/// and `hint`, smaller, under that, all muted, each line wrapped to the area's width so a
/// column at a side holds them too.
fn empty(ui: &Ui, area: Rect, message: &str, hint: Option<&str>) {
    let t = theme(ui.ctx());
    let painter = ui.painter_at(area);
    let colour = t.colors.text_secondary;
    let width = (area.width() - 2.0 * t.spacing.m).max(0.0);
    let centred = |text: &str, size: f32| {
        let mut job = egui::text::LayoutJob::simple(
            text.to_owned(),
            egui::FontId::proportional(size),
            colour,
            width,
        );
        job.halign = Align::Center;
        painter.layout_job(job)
    };
    let message = centred(message, t.text.body);
    let hint = hint.map(|hint| centred(hint, t.text.small));
    let icon = Icon::PhotoOff.size();
    let mut height = icon + t.spacing.s + message.size().y;
    if let Some(hint) = &hint {
        height += t.spacing.xxs + hint.size().y;
    }
    let mut y = area.center().y - height / 2.0;
    let x = area.center().x;
    Icon::PhotoOff.paint(
        &painter,
        Rect::from_min_size(pos2(x - icon / 2.0, y), vec2(icon, icon)),
        colour,
    );
    y += icon + t.spacing.s;
    let below = message.size().y;
    painter.galley(pos2(x, y), message, colour);
    if let Some(hint) = hint {
        painter.galley(pos2(x, y + below + t.spacing.xxs), hint, colour);
    }
}

/// The grid panel's height with `lines` rows of thumbnails of `size`, their captions
/// counted while `labels` show: the separator, the cells with room for their rings above
/// and below, and the scroll bar. The grid bar, a column at its left end, fits beside the
/// cells of one Small row.
pub fn grid_panel_height(
    ctx: &egui::Context,
    size: ThumbnailSize,
    labels: bool,
    lines: usize,
) -> f32 {
    let t = theme(ctx);
    t.shape.separator_width + lines_across(size.cell(labels).y, lines.max(1)) + SCROLL_BAR
}

/// The grid panel's width at a side with `lines` columns of thumbnails of `size`: the
/// separator, the cells with room for their rings either side, and the scroll bar.
pub fn grid_panel_width(ctx: &egui::Context, size: ThumbnailSize, lines: usize) -> f32 {
    theme(ctx).shape.separator_width + lines_across(size.side(), lines.max(1)) + SCROLL_BAR
}

/// How thick the grid panel is at `position` with `lines` lines of thumbnails of `size`,
/// their captions counted while `labels` show: its height at the bottom, its width at a
/// side.
pub fn grid_panel_thickness(
    ctx: &egui::Context,
    position: GridPosition,
    size: ThumbnailSize,
    labels: bool,
    lines: usize,
) -> f32 {
    if position.is_column() {
        grid_panel_width(ctx, size, lines)
    } else {
        grid_panel_height(ctx, size, labels, lines)
    }
}

/// The most lines of thumbnails of `size`, their captions counted while `labels` show, the
/// grid panel at `position` can show in `room` points of its thickness, at least one
/// (`docs/gui.md`, Its size: at most half the window).
pub fn grid_panel_lines(
    ctx: &egui::Context,
    position: GridPosition,
    size: ThumbnailSize,
    labels: bool,
    room: f32,
) -> usize {
    let mut lines = 1;
    while grid_panel_thickness(ctx, position, size, labels, lines + 1) <= room {
        lines += 1;
    }
    lines
}

/// The grid bar's buttons: the central bar's.
const BUTTON: f32 = 28.0;

/// The grid bar's width at the bottom, a column of its buttons with a little room either
/// side.
const BAR_COLUMN: f32 = BUTTON + 8.0;

/// `value`'s id as settings keep it, which the grid bar's messages select on.
fn id_of<T: Choice + PartialEq>(value: T) -> &'static str {
    T::OPTIONS
        .iter()
        .find(|(option, _)| *option == value)
        .map_or("", |(_, id)| id)
}

/// The photo a key sends the focus to from cell `i`, taking the keys it uses: along the
/// strip (← and → in a row, ↑ and ↓ in a column), to the same place in the next or previous
/// line across, the last photo where the last line is short; across it, to the cell beside
/// it in its own line, and nothing at the line's end; Home and End to the first and last.
fn keyed(ui: &Ui, id: Id, i: usize, cells: &Cells) -> Option<usize> {
    ui.memory_mut(|m| {
        m.set_focus_lock_filter(
            id,
            EventFilter {
                horizontal_arrows: true,
                vertical_arrows: true,
                ..Default::default()
            },
        );
    });
    let (count, lines) = (cells.count, cells.lines);
    let last = count - 1;
    let line = i % lines;
    let (next, previous, beside, before) = if cells.column {
        (
            Key::ArrowDown,
            Key::ArrowUp,
            Key::ArrowRight,
            Key::ArrowLeft,
        )
    } else {
        (
            Key::ArrowRight,
            Key::ArrowLeft,
            Key::ArrowDown,
            Key::ArrowUp,
        )
    };
    ui.input_mut(|input| {
        let mut key = |key| input.consume_key(Modifiers::NONE, key);
        if key(next) {
            // The next line across, where there is one.
            (i / lines < last / lines).then(|| (i + lines).min(last))
        } else if key(previous) {
            i.checked_sub(lines)
        } else if key(beside) {
            (line + 1 < lines && i < last).then_some(i + 1)
        } else if key(before) {
            (line > 0).then(|| i - 1)
        } else if key(Key::Home) {
            Some(0)
        } else if key(Key::End) {
            Some(last)
        } else {
            None
        }
    })
    .filter(|target| *target != i)
}

/// Where the cells lie in the scrolling strip: in a row, columns of `lines` cells, each
/// filled from the top before the next; in a column, rows of `lines`, each filled from the
/// left before the next.
struct Cells {
    count: usize,
    lines: usize,
    /// A cell's size, its caption counted, in points.
    cell: egui::Vec2,
    /// Whether the strip runs up and down.
    column: bool,
}

impl Cells {
    /// A cell's extent along the strip and across it.
    fn extents(&self) -> (f32, f32) {
        if self.column {
            (self.cell.y, self.cell.x)
        } else {
            (self.cell.x, self.cell.y)
        }
    }

    /// From one cell's start to the next's, along the strip and across it.
    fn pitch(&self) -> (f32, f32) {
        let (along, across) = self.extents();
        (along + GAP, across + GAP)
    }

    /// Before the first line along the strip: a row's area is inset at its ends already,
    /// a column's top under the grid bar is not.
    fn lead(&self) -> f32 {
        if self.column { TOP } else { 0.0 }
    }

    /// The strip's content: every line along it, and the lines across with room before
    /// and after.
    fn size(&self) -> egui::Vec2 {
        let along = self.lead() * 2.0 + self.count.div_ceil(self.lines) as f32 * self.pitch().0;
        let across = lines_across(self.extents().1, self.lines);
        if self.column {
            vec2(across, along)
        } else {
            vec2(along, across)
        }
    }

    /// Cell `i`'s rectangle, its square and caption, the content starting at `origin`.
    fn rect(&self, origin: egui::Pos2, i: usize) -> Rect {
        let (major, line) = (i / self.lines, i % self.lines);
        let (pitch_along, pitch_across) = self.pitch();
        let along = self.lead() + major as f32 * pitch_along;
        let across = TOP + line as f32 * pitch_across;
        let at = if self.column {
            vec2(across, along)
        } else {
            vec2(along, across)
        };
        Rect::from_min_size(origin + at, self.cell)
    }

    /// The cells in sight, or partly, in `viewport`, the content's own coordinates.
    fn visible(&self, viewport: Rect) -> Range<usize> {
        let range = if self.column {
            viewport.y_range()
        } else {
            viewport.x_range()
        };
        let pitch = self.pitch().0;
        let (start, end) = (range.min - self.lead(), range.max - self.lead());
        let first = (start / pitch).floor().max(0.0) as usize;
        let last = (end / pitch).ceil().max(0.0) as usize;
        let end = (last * self.lines).min(self.count);
        (first * self.lines).min(end)..end
    }
}

/// How far `lines` lines of cells `extent` points across reach across the strip, with the
/// room before the first and after the last.
fn lines_across(extent: f32, lines: usize) -> f32 {
    TOP + lines as f32 * (extent + GAP) - GAP + TOP
}
