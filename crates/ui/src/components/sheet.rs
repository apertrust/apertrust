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

//! The sheet (`docs/gui.md`, Sheets; decision 86): a full-window sheet inside the main
//! window, below the Toolbar, drawn on `panel` the same on every platform. Import is its
//! one user now; the frame is general, so Print may take it later. Four regions:
//!
//! - **The sheet bar,** across the sheet, as tall as the viewer bar with its 2-point
//!   separator under it: the heading in 16-point Medium, its counts in `text_secondary`,
//!   then the caller's Select buttons, quiet text buttons that fold into one Select ▾
//!   pull-down below about 520 points of grid width. To screen readers a toolbar named as
//!   the caller says.
//! - **The grid,** the rest of the width between the bar and the footer: the Browser's
//!   cells ([`GridCell`]), choosing by check boxes ([`Checks`](super::Checks)), only those
//!   in sight drawn. Its keys are the Browser's ← → ↑ ↓ Home End, moving the focus and the
//!   sheet's active photo, with Shift checking the cells passed; Space turns the focused
//!   cell's check and Shift+Space sets the range from the last touched; Ctrl+A (Cmd+A) and
//!   Ctrl+Shift+A check all and none while the grid has the focus. A list to screen
//!   readers, named as the caller says, the checked count said politely at most every half
//!   second. The cells shrink, never the kept Zoom, to the largest size two of which fit.
//! - **The column,** on the Tools panel's side, 440 points by default, resized at its edge
//!   ([`PanelEdge`]) from 296 to 600 and to no more than half the sheet where that leaves
//!   more than 296; 296 is the floor. It gives way only once the grid is down to two cells.
//!   It holds one tool, the caller's, scrolling with the quiet scroll bar.
//! - **The footer,** 52 points with a hairline above: Help at its start (drawn disabled
//!   until the Apertrust Guide is built), the summary on two lines, its path cut in its
//!   middle first, then Close and the primary button in the platform's order, never cut. A
//!   primary button that cannot press stays a focus stop, its reason its tooltip, its
//!   description and the summary's second line.
//!
//! **Keys,** the sheet's own, read before its widgets see them: Return presses the primary
//! button, unless a text field is typed in, a list is open, or a button, a choice or a
//! slider has the focus, which takes Return itself; a focused cell does not. A text field
//! row that reports Return presses it too ([`Returned`]). Ctrl+Return (Cmd+Return) presses
//! it anywhere but an open list. Esc, and Cmd+. on the Mac, close the innermost first, a
//! list, a menu, the variable list or a field being typed in, then the sheet. F6 and
//! Shift+F6 move among three regions, the bar and the grid (landing on the active cell),
//! the column and the footer; Tab and Shift+Tab stay inside the sheet. As it opens,
//! whatever had the focus behind it lets go of it, and the grid's active cell takes it, or,
//! with the grid still empty, the first active cell it has, unless something in the sheet
//! took the focus meanwhile.
//!
//! **To screen readers** a region named as the caller says ("Import, EOS_DIGITAL") inside a
//! modal dialog of the same name.
//!
//! **Nothing here acts:** the sheet reports gestures and presses ([`SheetOutput`]), and its
//! caller applies them.

use std::ops::Range;

use egui::accesskit::Live;
use egui::os::OperatingSystem;
use egui::scroll_area::ScrollBarVisibility;
use egui::{
    Align, FocusDirection, FontFamily, FontId, Id, Key, Label, Layout, Modifiers, Rect, RichText,
    Role, ScrollArea, Sense, Stroke, Ui, UiBuilder, WidgetInfo, pos2, vec2,
};

use super::browser::{Cells, SCROLL_BAR, TOP, keyed};
use super::grid_cell::CellMark;
use super::plate::{Plate, Touch, paint_focus, paint_plate};
use super::tools_panel::{restore, scroll_area};
use super::{
    Button, ButtonOrder, CellCheck, CellPicture, CheckGesture, GridCell, IconButton, PanelEdge,
    PanelEdgeAction, ThumbnailSize, ToolButton, ViewerBar, middle_cut, pull_down, truncated,
};
use crate::popup::{self, Item};
use crate::{Icon, Theme, fonts, theme, tr};

/// The column's width at first, and the least and the most it takes, in points.
pub const COLUMN_WIDTH: f32 = 440.0;
pub const COLUMN_LEAST: f32 = 296.0;
pub const COLUMN_MOST: f32 = 600.0;

/// The footer's height, in points.
pub const FOOTER: f32 = 52.0;

/// The grid's width below which the Select buttons fold into Select ▾, in points.
const SELECT_FOLD: f32 = 520.0;

/// How often, at most, the checked count is said, in seconds.
const SAID_EVERY: f64 = 0.5;

/// One photo of the sheet's grid.
#[derive(Clone, Copy, Debug)]
pub struct SheetCell<'a> {
    /// The file name, with its extension.
    pub name: &'a str,
    /// Its picture, once it has come.
    pub picture: Option<CellPicture>,
    /// Whether its file holds no picture to show: No preview yet.
    pub no_preview: bool,
    pub checked: bool,
    /// Its check box's name to screen readers, translated: "Import DSCF1033.RAF".
    pub check_name: &'a str,
    /// Its check box's description to screen readers, translated, as its capture time;
    /// empty for none.
    pub check_description: &'a str,
    /// Its picture at half strength, as a photo imported before while unchecked.
    pub dimmed: bool,
    /// The mark at its picture's bottom right, as the destination mark.
    pub mark: Option<CellMark<'a>>,
    /// Its tooltip, translated, in place of the cell's own of its name and mark.
    pub tip: Option<&'a str>,
}

/// A Select button of the sheet bar: what it answers with, and its words, translated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectButton<'a> {
    pub id: &'a str,
    pub text: &'a str,
}

/// The footer's summary, on two lines, translated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary<'a> {
    /// The first line's start, kept whole: the count and the size, "209 of 412 photos ·
    /// 6.2 GB of 412 GB free → ".
    pub lead: &'a str,
    /// The path after it, cut in its middle first where the line is too short.
    pub path: &'a str,
    /// After the path, kept whole: ", a new folder".
    pub tail: &'a str,
    /// The first line's tooltip.
    pub tip: Option<&'a str>,
    /// The second line, smaller, in `text_secondary`, unless it warns.
    pub note: &'a str,
    /// Whether the second line warns of what may surprise: `warning`, with the warning sign.
    pub warning: bool,
}

/// What a text field row in the column said this frame: that Return was pressed in it,
/// which presses the primary button where it can.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Returned(pub bool);

/// What the sheet did this frame.
#[derive(Clone, Debug, PartialEq)]
pub struct SheetOutput<R> {
    /// What the person did to the checks.
    pub gesture: Option<CheckGesture>,
    /// A Select button pressed, by its id.
    pub select: Option<String>,
    /// The primary button pressed, by the pointer, its keys or Return; never while it
    /// cannot press.
    pub primary: bool,
    /// Close pressed, by the pointer, Esc or Cmd+.
    pub close: bool,
    /// What was done to the column's edge.
    pub column: Option<PanelEdgeAction>,
    /// The photos whose cells were drawn.
    pub shown: Range<usize>,
    /// What the column's tool returned.
    pub inner: R,
}

/// A sheet of `count` photos; `cell(i)` gives the i-th, asked for only the visible ones.
pub struct Sheet<'a, F: Fn(usize) -> SheetCell<'a>> {
    heading: &'a str,
    name: &'a str,
    counts: &'a str,
    bar_name: &'a str,
    select: &'a [SelectButton<'a>],
    count: usize,
    cell: F,
    active: Option<usize>,
    grid_name: &'a str,
    checked_said: &'a str,
    size: ThumbnailSize,
    column_width: f32,
    column_at_start: bool,
    summary: Summary<'a>,
    primary: &'a str,
    why_not: Option<&'a str>,
    order: ButtonOrder,
    focus_given: bool,
}

impl<'a, F: Fn(usize) -> SheetCell<'a>> Sheet<'a, F> {
    /// `heading` is the bar's ("Import · EOS_DIGITAL") and `name` what screen readers call
    /// the sheet ("Import, EOS_DIGITAL"), both translated; `active` is the sheet's own active
    /// photo, never the window's.
    pub fn new(
        heading: &'a str,
        name: &'a str,
        count: usize,
        active: Option<usize>,
        cell: F,
    ) -> Self {
        Sheet {
            heading,
            name,
            counts: "",
            bar_name: "",
            select: &[],
            count,
            cell,
            active,
            grid_name: "",
            checked_said: "",
            size: ThumbnailSize::Medium,
            column_width: COLUMN_WIDTH,
            column_at_start: false,
            summary: Summary::default(),
            primary: "",
            why_not: None,
            order: ButtonOrder::platform(),
            focus_given: false,
        }
    }

    /// The counts after the heading, translated: "412 photos, 198 imported before".
    pub fn counts(mut self, counts: &'a str) -> Self {
        self.counts = counts;
        self
    }

    /// What screen readers call the bar, translated: "Import grid bar".
    pub fn bar_name(mut self, name: &'a str) -> Self {
        self.bar_name = name;
        self
    }

    /// The Select buttons, in their order.
    pub fn select(mut self, buttons: &'a [SelectButton<'a>]) -> Self {
        self.select = buttons;
        self
    }

    /// What screen readers call the grid, and the checked count they hear said as it
    /// changes, translated: "EOS_DIGITAL, 412 photos, 209 checked", "209 checked".
    pub fn grid_name(mut self, name: &'a str, checked_said: &'a str) -> Self {
        self.grid_name = name;
        self.checked_said = checked_said;
        self
    }

    /// The cells' Zoom, which the grid shows at, or smaller where two do not fit.
    pub fn size(mut self, size: ThumbnailSize) -> Self {
        self.size = size;
        self
    }

    /// The column's kept width, and whether it is at the window's start, as the Tools
    /// panel is.
    pub fn column(mut self, width: f32, at_start: bool) -> Self {
        self.column_width = width;
        self.column_at_start = at_start;
        self
    }

    pub fn summary(mut self, summary: Summary<'a>) -> Self {
        self.summary = summary;
        self
    }

    /// The primary button's words, translated, and why it cannot press, where it cannot.
    pub fn primary(mut self, words: &'a str, why_not: Option<&'a str>) -> Self {
        self.primary = words;
        self.why_not = why_not;
        self
    }

    /// Where the primary button sits; the platform's unless a test says otherwise.
    pub fn order(mut self, order: ButtonOrder) -> Self {
        self.order = order;
        self
    }

    /// The caller puts the focus in the column as the sheet opens, as Import name for a
    /// card: the grid's active cell does not take it.
    pub fn focus_given(mut self, given: bool) -> Self {
        self.focus_given = given;
        self
    }

    /// Draws the sheet over the whole of `ui`'s room, the column's tool drawn by `column`.
    pub fn show<R>(
        self,
        ui: &mut Ui,
        column: impl FnOnce(&mut Ui, &mut Returned) -> R,
    ) -> SheetOutput<R> {
        let t = theme(ui.ctx());
        let ctx = ui.ctx().clone();
        let id = sheet_id();
        let pass = ctx.cumulative_pass_nr();
        // What the last frame drew, while the sheet stayed open; none as it opens.
        let last = (ctx.data(|d| d.get_temp::<Last>(id))).filter(|last| last.pass + 1 >= pass);
        let (rect, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
        ui.painter().rect_filled(rect, 0.0, t.colors.panel);
        // As it opens, whatever had the focus behind it lets go of it, before anything in
        // it is drawn: a slider or a field under the sheet never keeps the keys, with no
        // photo in the grid to take them either. The caller may put it in the column.
        if last.is_none()
            && let Some(focused) = ctx.memory(|m| m.focused())
        {
            ctx.memory_mut(|m| m.surrender_focus(focused));
        }

        let enabled = self.why_not.is_none();
        let keys = Keys::read(ui, last.as_ref(), self.count);
        let mut primary = enabled && keys.primary;
        let mut close = keys.close;
        let mut gesture = keys.gesture;

        // The regions.
        let bar_height = ViewerBar::height(&ctx) + t.shape.separator_width;
        let bar = Rect::from_min_size(rect.min, vec2(rect.width(), bar_height));
        let footer = Rect::from_min_max(pos2(rect.left(), rect.bottom() - FOOTER), rect.max);
        let middle = Rect::from_min_max(
            pos2(rect.left(), bar.bottom()),
            pos2(rect.right(), footer.top()),
        );
        let most = (rect.width() / 2.0).clamp(COLUMN_LEAST, COLUMN_MOST);
        let edge_name = tr(&ctx, "sheet-column-width");
        let edge = PanelEdge::new(&edge_name, self.column_width, COLUMN_LEAST..=most)
            .at_start(self.column_at_start);
        // The grid keeps two cells at its Zoom before the column gives way, down to 296.
        let wanted = edge.width(&ctx);
        let two = grid_least(self.size.side());
        let width = wanted
            .min(rect.width() - two)
            .max(COLUMN_LEAST)
            .min(rect.width());
        let (grid, column_rect) = if self.column_at_start {
            let column = Rect::from_min_size(middle.min, vec2(width, middle.height()));
            (
                Rect::from_min_max(pos2(column.right(), middle.top()), middle.max),
                column,
            )
        } else {
            let column = Rect::from_min_max(pos2(middle.right() - width, middle.top()), middle.max);
            (
                Rect::from_min_max(middle.min, pos2(column.left(), middle.bottom())),
                column,
            )
        };

        let name = self.name;
        // A modal dialog, and inside it a region, both named as the caller says, the sheet's
        // parts inside the region.
        let dialog = ui.interact(rect, id.with("dialog"), Sense::hover());
        dialog.widget_info(|| WidgetInfo::labeled(Role::Dialog, true, name));
        ctx.accesskit_node_builder(dialog.id, |node| node.set_modal());
        let mut within = ui.new_child(
            UiBuilder::new()
                .max_rect(rect)
                .accessibility_parent(dialog.id),
        );
        let region = within.interact(rect, id.with("region"), Sense::hover());
        region.widget_info(|| WidgetInfo::labeled(Role::Region, true, name));
        let ui = &mut within.new_child(
            UiBuilder::new()
                .max_rect(rect)
                .accessibility_parent(region.id),
        );
        let select = self.bar(ui, &t, bar, grid.width());
        // The focus as the sheet opens is the grid's active cell's, unless the caller gave it;
        // and F6's back to the grid. A grid still empty as it opens, its photos on their way,
        // keeps it waiting until it first has an active cell, unless something in the sheet
        // has taken the focus meanwhile.
        let waiting = id.with("opening");
        if last.is_none() {
            ctx.data_mut(|d| d.insert_temp(waiting, !self.focus_given));
        }
        let taken = last.is_some() && ctx.memory(|m| m.focused()).is_some();
        let ready = self.count > 0 && (self.active.is_some() || last.is_none());
        let opening = ctx.data(|d| d.get_temp::<bool>(waiting)).unwrap_or(false) && !taken;
        if taken || (opening && ready) {
            ctx.data_mut(|d| d.insert_temp(waiting, false));
        }
        let opening = opening && ready;
        let drawn = self.grid(ui, &t, grid, last.as_ref(), keys.focus_active || opening);
        gesture = gesture.or(drawn.gesture);
        let shown = drawn.shown;
        let mut child = ui.new_child(UiBuilder::new().max_rect(column_rect));
        child.set_clip_rect(column_rect);
        let mut returned = Returned::default();
        let inner = column_area(&mut child, &t, &mut returned, column);
        let resized = edge.show(ui, column_rect);
        let pressed = self.footer(ui, &t, footer);
        primary |= pressed.primary;
        close |= pressed.close;
        primary |= enabled && returned.0;

        let kept = Last {
            pass,
            sheet: rect,
            regions: [
                [bar, grid],
                [column_rect, Rect::NOTHING],
                [footer, Rect::NOTHING],
            ],
            shown: shown.clone(),
            typing: ctx.text_edit_focused(),
        };
        ctx.data_mut(|d| d.insert_temp(id, kept));
        SheetOutput {
            gesture,
            select,
            primary,
            close,
            column: resized,
            shown,
            inner,
        }
    }

    /// The sheet bar in `bar`, its separator under it: the heading, the counts, then the
    /// Select buttons, folded into Select ▾ while the grid is `grid` wide and under
    /// [`SELECT_FOLD`]. Returns the Select button pressed.
    fn bar(&self, ui: &mut Ui, t: &Theme, bar: Rect, grid: f32) -> Option<String> {
        let line = Stroke::new(t.shape.separator_width, t.colors.separator);
        ui.painter().hline(
            bar.x_range(),
            bar.bottom() - t.shape.separator_width / 2.0,
            line,
        );
        let inner = Rect::from_min_max(
            pos2(bar.left() + t.spacing.l, bar.top()),
            pos2(
                bar.right() - t.spacing.l,
                bar.bottom() - t.shape.separator_width,
            ),
        );
        let toolbar = ui.interact(inner, sheet_id().with("bar"), Sense::hover());
        let bar_name = self.bar_name;
        toolbar.widget_info(|| WidgetInfo::labeled(Role::Toolbar, true, bar_name));
        let mut row = ui.new_child(
            UiBuilder::new()
                .max_rect(inner)
                .layout(Layout::left_to_right(Align::Center))
                .accessibility_parent(toolbar.id),
        );
        row.spacing_mut().item_spacing.x = t.spacing.m;
        let folded = grid < SELECT_FOLD;
        let words = tr(ui.ctx(), "sheet-select");
        let controls = if self.select.is_empty() {
            0.0
        } else if folded {
            select_menu_width(&row, t, &words)
        } else {
            (self.select.iter())
                .map(|b| ToolButton::new(b.text).width(&row) + t.spacing.xxs)
                .sum::<f32>()
        };
        let heading = FontId::new(t.text.heading, FontFamily::Name(fonts::MEDIUM.into()));
        let room = (inner.width() - controls - 2.0 * t.spacing.xl).max(0.0);
        let heading_width = row
            .painter()
            .layout_no_wrap(self.heading.to_owned(), heading.clone(), t.colors.text)
            .size()
            .x
            .min(room);
        row.allocate_ui(vec2(heading_width, inner.height()), |ui| {
            let text = RichText::new(self.heading)
                .font(heading)
                .color(t.colors.text);
            ui.add(Label::new(text).selectable(false).truncate());
        });
        let counts_room = (room - heading_width - t.spacing.m).max(0.0);
        if !self.counts.is_empty() && counts_room > 0.0 {
            row.allocate_ui(vec2(counts_room, inner.height()), |ui| {
                let text = RichText::new(self.counts)
                    .size(t.text.body)
                    .color(t.colors.text_secondary);
                ui.add(Label::new(text).selectable(false).truncate());
            });
        }
        if self.select.is_empty() {
            return None;
        }
        // The controls at the end of the room the words left them, a group's gap after.
        let at = inner.right() - controls;
        let mut end = ui.new_child(
            UiBuilder::new()
                .max_rect(Rect::from_min_max(pos2(at, inner.top()), inner.max))
                .layout(Layout::left_to_right(Align::Center))
                .accessibility_parent(toolbar.id),
        );
        end.spacing_mut().item_spacing.x = t.spacing.xxs;
        if folded {
            let response = select_menu(&mut end, t, &words);
            return pull_down(&end, &response, &words, || {
                (self.select.iter())
                    .map(|b| Item::new(b.id, b.text).into())
                    .collect()
            });
        }
        let mut pressed = None;
        for button in self.select {
            if ToolButton::new(button.text).show(&mut end).clicked() {
                pressed = Some(button.id.to_owned());
            }
        }
        pressed
    }

    /// The grid in `rect`: the cells in sight, their keys and gestures.
    fn grid(
        &self,
        ui: &mut Ui,
        t: &Theme,
        rect: Rect,
        last: Option<&Last>,
        focus_active: bool,
    ) -> Grid {
        ui.painter().rect_filled(rect, 0.0, t.colors.surface);
        let list_id = sheet_id().with("grid");
        let list = ui.interact(rect, list_id, Sense::hover());
        let grid_name = self.grid_name;
        list.widget_info(|| WidgetInfo::labeled(Role::List, true, grid_name));
        ui.ctx()
            .accesskit_node_builder(list.id, |node| node.set_multiselectable());
        said(ui, rect, list.id, self.checked_said);
        let mut out = Grid {
            gesture: None,
            shown: 0..0,
        };
        if self.count == 0 {
            return out;
        }
        let size = shown_size(self.size, rect.width());
        let side = size.side();
        let across = ((rect.width() - 2.0 * TOP - SCROLL_BAR + super::browser::GAP)
            / (side + super::browser::GAP))
            .floor()
            .max(1.0) as usize;
        let cells = Cells {
            count: self.count,
            lines: across,
            cell: size.cell(true),
            column: true,
        };
        let active = self.active.filter(|a| *a < self.count);
        let pending_id = list_id.with("goto");
        // The active cell asked for by F6 or the opening is revealed, then takes the focus
        // once drawn; a cell a key moved to likewise.
        let reveal = focus_active.then(|| active.unwrap_or(0));
        if let Some(target) = reveal {
            ui.data_mut(|d| d.insert_temp(pending_id, target));
        }
        let goto: Option<usize> = ui.data(|d| d.get_temp(pending_id));
        let seen_id = list_id.with("seen");
        let now = (active, size, across, last.is_some());
        let changed = ui
            .data(|d| d.get_temp::<(Option<usize>, ThumbnailSize, usize, bool)>(seen_id))
            != Some(now);
        ui.data_mut(|d| d.insert_temp(seen_id, now));
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(rect)
                .accessibility_parent(list.id),
        );
        child.set_clip_rect(rect);
        let bar = &mut child.spacing_mut().scroll;
        bar.bar_width = SCROLL_BAR;
        bar.bar_inner_margin = 0.0;
        bar.bar_outer_margin = 0.0;
        let mut moved = None;
        scroll_area(&mut child, t, |ui| {
            ScrollArea::vertical()
                .id_salt("sheet grid")
                .auto_shrink([false, false])
                .show_viewport(ui, |ui, viewport| {
                    restore(ui, |ui| {
                        let (all, _) = ui.allocate_exact_size(cells.size(), Sense::hover());
                        let origin = all.min;
                        let room = vec2(0.0, TOP);
                        if let Some(target) = reveal.or(active.filter(|_| changed)) {
                            ui.scroll_to_rect(cells.rect(origin, target).expand2(room), None);
                        }
                        let visible = cells.visible(viewport);
                        out.shown = visible.clone();
                        for i in visible.clone() {
                            let id = cell_id(i);
                            if goto == Some(i) {
                                ui.memory_mut(|m| m.request_focus(id));
                                ui.data_mut(|d| d.remove::<usize>(pending_id));
                            }
                            // The cell's keys, before it takes Space as a click.
                            if ui.memory(|m| m.has_focus(id)) {
                                let (keyed_gesture, target) = cell_keys(ui, id, i, &cells);
                                out.gesture = out.gesture.or(keyed_gesture);
                                moved = moved.or(target);
                            }
                            let cell = (self.cell)(i);
                            let drawn = GridCell::new(cell.name)
                                .side(side)
                                .picture(cell.picture)
                                .no_preview(cell.no_preview)
                                .active(active == Some(i))
                                .check(Some(CellCheck {
                                    on: cell.checked,
                                    name: cell.check_name,
                                    description: cell.check_description,
                                }))
                                .dimmed(cell.dimmed)
                                .mark(cell.mark)
                                .tip(cell.tip)
                                .show_at(ui, cells.rect(origin, i), id);
                            let modifiers = ui.input(|input| input.modifiers);
                            // A click on the cell, its box or its picture, gives it the focus,
                            // so the keys go on from it.
                            if drawn.check || drawn.clicked {
                                ui.memory_mut(|m| m.request_focus(id));
                            }
                            if drawn.check {
                                out.gesture = Some(if modifiers.shift {
                                    CheckGesture::SetRange(i)
                                } else {
                                    CheckGesture::Toggle(i)
                                });
                            } else if drawn.clicked {
                                out.gesture = Some(if modifiers.shift {
                                    CheckGesture::CheckRange(i)
                                } else if modifiers.command {
                                    CheckGesture::ToggleActivate(i)
                                } else {
                                    CheckGesture::Activate(i)
                                });
                            }
                        }
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
                    });
                });
        });
        out
    }

    /// The footer in `rect`: Help, the summary, Close and the primary button.
    fn footer(&self, ui: &mut Ui, t: &Theme, rect: Rect) -> Pressed {
        let ctx = ui.ctx().clone();
        ui.painter().hline(
            rect.x_range(),
            rect.top() + t.shape.hairline / 2.0,
            Stroke::new(t.shape.hairline, t.colors.divider),
        );
        let inner = rect.shrink2(vec2(t.spacing.l, 0.0));
        let mut row = ui.new_child(
            UiBuilder::new()
                .max_rect(inner)
                .layout(Layout::left_to_right(Align::Center)),
        );
        row.spacing_mut().item_spacing.x = t.spacing.m;
        // Help, until the Apertrust Guide is built: disabled, saying so.
        let later = tr(&ctx, "sheet-help-later");
        let help = IconButton::new(Icon::Help, &tr(&ctx, "sheet-help"))
            .tip(&later)
            .enabled(false)
            .show(&mut row);
        ctx.accesskit_node_builder(help.id, |node| node.set_description(later.clone()));
        // The buttons at the end, never cut, in the platform's order: the primary first in
        // the request, Close second.
        let close_words = tr(&ctx, "sheet-close");
        let words = [self.primary, close_words.as_str()];
        let buttons: f32 = (words.iter())
            .map(|w| Button::new(w).natural_width(&row))
            .sum::<f32>()
            + t.spacing.m;
        let room = (row.available_width() - buttons - t.spacing.m).max(0.0);
        let (lines, _) = row.allocate_exact_size(vec2(room, inner.height()), Sense::hover());
        self.summary_lines(&mut row, t, lines);
        let mut pressed = Pressed::default();
        for i in self.order.arrange(2, 0) {
            if i == 0 {
                let mut button = Button::new(self.primary)
                    .primary(true)
                    .held_off(self.why_not.is_some());
                if let Some(why) = self.why_not {
                    button = button.tip(why);
                }
                pressed.primary = button.show_pressed(&mut row);
            } else {
                pressed.close = Button::new(&close_words).show(&mut row).clicked();
            }
        }
        pressed
    }

    /// The summary's two lines in `rect`: the first in `text`, its path cut in its middle
    /// first, its tooltip the caller's; the second smaller, in `text_secondary`, or in
    /// `warning` with the warning sign, or the primary button's reason where it cannot
    /// press. Each is a label to screen readers, cut with … where it is still too long, whole
    /// in its tooltip.
    fn summary_lines(&self, ui: &mut Ui, t: &Theme, rect: Rect) {
        let summary = self.summary;
        let painter = ui.painter().clone();
        let font = FontId::proportional(t.text.body);
        let small = FontId::proportional(t.text.small);
        let width = rect.width();
        let measure = |text: &str, font: &FontId| {
            (painter.layout_no_wrap(text.to_owned(), font.clone(), t.colors.text))
                .size()
                .x
        };
        let kept = measure(summary.lead, &font) + measure(summary.tail, &font);
        let path = middle_cut(&painter, summary.path, "", &font, (width - kept).max(0.0));
        let first = format!("{}{path}{}", summary.lead, summary.tail);
        let first = painter.layout_job(truncated(&first, font, width, t.colors.text));
        let (note, warning) = match self.why_not {
            Some(why) => (why, false),
            None => (summary.note, summary.warning),
        };
        let colour = if warning {
            t.colors.warning
        } else {
            t.colors.text_secondary
        };
        let sign = if warning {
            Icon::Warning.size() + t.spacing.xs
        } else {
            0.0
        };
        let second = painter.layout_job(truncated(note, small, width - sign, colour));
        let gap = t.spacing.xxs;
        let top = rect.center().y - (first.size().y + gap + second.size().y) / 2.0;
        let first_rect = Rect::from_min_size(pos2(rect.left(), top), first.size());
        let second_at = pos2(rect.left() + sign, first_rect.bottom() + gap);
        let second_rect = Rect::from_min_size(second_at, second.size());
        if warning {
            let side = Icon::Warning.size();
            let at = pos2(rect.left(), second_rect.center().y - side / 2.0);
            Icon::Warning.paint(&painter, Rect::from_min_size(at, vec2(side, side)), colour);
        }
        let whole = format!("{}{}{}", summary.lead, summary.path, summary.tail);
        let second_cut = second.elided;
        painter.galley(first_rect.min, first, t.colors.text);
        painter.galley(second_at, second, colour);
        let id = sheet_id().with("summary");
        let response = ui.interact(first_rect, id, Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &whole));
        drop(response.on_hover_text(summary.tip.map_or(whole.clone(), str::to_owned)));
        let response = ui.interact(second_rect, id.with("note"), Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, note));
        if second_cut {
            drop(response.on_hover_text(note));
        }
    }
}

/// The sheet's id: one sheet is open at a time.
fn sheet_id() -> Id {
    Id::unique("apertrust-ui sheet")
}

/// What the last frame of the sheet drew, for this frame's keys.
#[derive(Clone, Debug)]
struct Last {
    /// The pass it was drawn in: a sheet not drawn on the pass before has just opened.
    pass: u64,
    sheet: Rect,
    /// F6's three regions: the bar and the grid, the column, the footer.
    regions: [[Rect; 2]; 3],
    /// The cells drawn, by their places: those the focus can be on.
    shown: Range<usize>,
    /// Whether a text field had the focus as the frame ended: an Esc since is its own.
    typing: bool,
}

impl Last {
    /// Whether `id` is one of the grid's cells drawn.
    fn is_cell(&self, id: Id) -> bool {
        self.shown.clone().any(|i| cell_id(i) == id)
    }
}

/// The grid's cell for photo `i`.
fn cell_id(i: usize) -> Id {
    sheet_id().with("grid").with(i)
}

/// What the sheet's own keys asked for, read before any widget sees them.
#[derive(Default)]
struct Keys {
    primary: bool,
    close: bool,
    gesture: Option<CheckGesture>,
    /// F6 lands on the grid's active cell.
    focus_active: bool,
}

impl Keys {
    fn read(ui: &mut Ui, last: Option<&Last>, count: usize) -> Keys {
        let ctx = ui.ctx().clone();
        let mut keys = Keys::default();
        let Some(last) = last else {
            return keys;
        };
        let focused = ctx.memory(|m| m.focused());
        let widgets = focusable(&ctx, ui.layer_id(), last.sheet);
        // A popup open, or the focused path field's list of variables, which the field draws
        // itself.
        let list_open = popup::any_open(&ctx)
            || focused.is_some_and(|id| super::text_field::variables_open(&ctx, id));
        let typing = ctx.text_edit_focused();
        // Ctrl+Return (Cmd+Return) anywhere but an open list, whose keys are its own.
        if !list_open && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter)) {
            keys.primary = true;
        }
        // Return, unless a text field is typed in or a control that presses on it has the
        // focus, a button, a choice or a slider; a focused cell does not, and nor does a
        // focus stop that takes no press.
        let on_cell = focused.is_some_and(|id| last.is_cell(id));
        let takes_return = focused.is_some_and(|id| {
            !last.is_cell(id) && widgets.iter().any(|(w, _, click)| *w == id && *click)
        });
        if !list_open
            && !typing
            && !takes_return
            && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter))
        {
            keys.primary = true;
        }
        // Esc, and Cmd+. on the Mac: the sheet, once nothing inside is the innermost. An
        // open list takes it before the frame; a field typed in, or its variables' list,
        // took it as the last frame ended.
        let escape = ui.input(|i| i.key_pressed(Key::Escape));
        let period = ctx.os() == OperatingSystem::Mac
            && ui.input_mut(|i| i.consume_key(Modifiers::MAC_CMD, Key::Period));
        if (escape && !last.typing && !list_open) || (period && !typing && !list_open) {
            keys.close = true;
        }
        // F6 and Shift+F6 among the three regions: Shift first, which a key asked for with
        // no modifier would take too.
        let f6 = ui.input_mut(|i| {
            if i.consume_key(Modifiers::SHIFT, Key::F6) {
                Some(false)
            } else if i.consume_key(Modifiers::NONE, Key::F6) {
                Some(true)
            } else {
                None
            }
        });
        if let Some(forward) = f6 {
            let region_of = |rect: Rect| {
                (last.regions.iter()).position(|r| r.iter().any(|r| r.contains(rect.center())))
            };
            let current = focused
                .and_then(|id| widgets.iter().find(|(w, _, _)| *w == id))
                .and_then(|(_, rect, _)| region_of(*rect));
            let n = last.regions.len();
            let next = match (current, forward) {
                (Some(c), true) => (c + 1) % n,
                (Some(c), false) => (c + n - 1) % n,
                (None, true) => 0,
                (None, false) => n - 1,
            };
            if next == 0 && count > 0 {
                keys.focus_active = true;
            } else if let Some((id, _, _)) =
                (widgets.iter()).find(|(_, rect, _)| region_of(*rect) == Some(next))
            {
                ctx.memory_mut(|m| m.request_focus(*id));
            }
        }
        // Tab and Shift+Tab stay inside the sheet: past its last stop to its first, and
        // back.
        let tab = ui.input(|i| {
            if i.key_pressed(Key::Tab) && i.modifiers.is_none() {
                Some(true)
            } else if i.key_pressed(Key::Tab) && i.modifiers.shift_only() {
                Some(false)
            } else {
                None
            }
        });
        if let Some(forward) = tab
            && let (Some(first), Some(end)) = (widgets.first(), widgets.last())
        {
            let at = focused.and_then(|id| widgets.iter().position(|(w, _, _)| *w == id));
            let wrap = match (at, forward) {
                (None, true) => Some(first.0),
                (None, false) => Some(end.0),
                (Some(at), true) if at + 1 == widgets.len() => Some(first.0),
                (Some(0), false) => Some(end.0),
                _ => None,
            };
            if let Some(target) = wrap {
                ctx.memory_mut(|m| {
                    m.move_focus(FocusDirection::None);
                    m.request_focus(target);
                });
            }
        }
        // Ctrl+A and Ctrl+Shift+A (Cmd on the Mac) while the grid has the focus.
        if on_cell && !typing {
            ui.input_mut(|i| {
                if i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::A) {
                    keys.gesture = Some(CheckGesture::None);
                } else if i.consume_key(Modifiers::COMMAND, Key::A) {
                    keys.gesture = Some(CheckGesture::All);
                }
            });
        }
        keys
    }
}

/// The widgets of `layer` inside `sheet` that the keyboard can focus, in the order the last pass met
/// them, with the parts of them in sight and whether they take a click.
fn focusable(ctx: &egui::Context, layer: egui::LayerId, sheet: Rect) -> Vec<(Id, Rect, bool)> {
    ctx.viewport(|viewport| {
        viewport
            .prev_pass
            .widgets
            .get_layer(layer)
            .filter(|w| w.enabled && w.sense.is_focusable() && w.interact_rect.is_positive())
            .filter(|w| sheet.contains(w.interact_rect.center()))
            .map(|w| (w.id, w.interact_rect, w.sense.senses_click()))
            .collect()
    })
}

/// The keys on the focused cell `i`, the widget `id`: Space turns its check and Shift+Space
/// sets the range; the arrows, Home and End move to a cell, with Shift checking the cells
/// passed. Returns the gesture and the cell the focus moves to.
fn cell_keys(ui: &Ui, id: Id, i: usize, cells: &Cells) -> (Option<CheckGesture>, Option<usize>) {
    // Shift first: a key asked for with no modifier would take it too.
    let space = ui.input_mut(|input| {
        if input.consume_key(Modifiers::SHIFT, Key::Space) {
            Some(CheckGesture::SetRange(i))
        } else if input.consume_key(Modifiers::NONE, Key::Space) {
            Some(CheckGesture::Toggle(i))
        } else {
            None
        }
    });
    if space.is_some() {
        return (space, None);
    }
    if let Some(target) = keyed(ui, id, i, cells, Modifiers::SHIFT) {
        return (Some(CheckGesture::MoveChecking(i, target)), Some(target));
    }
    if let Some(target) = keyed(ui, id, i, cells, Modifiers::NONE) {
        return (Some(CheckGesture::Activate(target)), Some(target));
    }
    (None, None)
}

/// What the grid drew and was asked.
struct Grid {
    gesture: Option<CheckGesture>,
    shown: Range<usize>,
}

/// What the footer's buttons were pressed for.
#[derive(Default)]
struct Pressed {
    primary: bool,
    close: bool,
}

/// The grid's least width for two cells of `side` points with their gaps, the room round
/// them and the scroll bar's lane.
fn grid_least(side: f32) -> f32 {
    2.0 * side + super::browser::GAP + 2.0 * TOP + SCROLL_BAR
}

/// The size the grid shows its cells at in `width`: the kept Zoom, or the largest smaller
/// one two cells of which fit, Small at least.
fn shown_size(size: ThumbnailSize, width: f32) -> ThumbnailSize {
    (ThumbnailSize::ALL.into_iter().rev())
        .filter(|s| s.side() <= size.side())
        .find(|s| grid_least(s.side()) <= width)
        .unwrap_or(ThumbnailSize::Small)
}

/// The column's tool in `ui`, scrolling with the quiet scroll bar, always there so the rows
/// never move as it comes.
fn column_area<R>(
    ui: &mut Ui,
    t: &Theme,
    returned: &mut Returned,
    column: impl FnOnce(&mut Ui, &mut Returned) -> R,
) -> R {
    let mut inner = None;
    scroll_area(ui, t, |ui| {
        ScrollArea::vertical()
            .id_salt("sheet column")
            .auto_shrink([false, false])
            .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
            .show(ui, |ui| {
                ui.scope(|ui| restore(ui, |ui| inner = Some(column(ui, returned))))
            });
    });
    inner.expect("the column's tool is drawn")
}

/// The checked count said politely in the grid `list`, at most every half second: the
/// words said last stay until then, a repaint asked for to say the new ones.
fn said(ui: &Ui, rect: Rect, list: Id, words: &str) {
    let id = list.with("said");
    let now = ui.input(|i| i.time);
    let (mut kept, at): (String, f64) = ui
        .data(|d| d.get_temp(id))
        .unwrap_or((words.to_owned(), f64::NEG_INFINITY));
    if kept != words {
        let wait = at + SAID_EVERY - now;
        if wait <= 0.0 {
            kept = words.to_owned();
            ui.data_mut(|d| d.insert_temp(id, (kept.clone(), now)));
        } else {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f64(wait));
        }
    } else if at == f64::NEG_INFINITY {
        ui.data_mut(|d| d.insert_temp(id, (kept.clone(), now)));
    }
    let status = ui.interact(
        Rect::from_min_size(rect.min, egui::Vec2::ZERO),
        id,
        Sense::hover(),
    );
    status.widget_info(|| WidgetInfo::labeled(Role::Status, true, &kept));
    (ui.ctx()).accesskit_node_builder(status.id, |node| node.set_live(Live::Polite));
}

/// Select ▾'s width: its words, a gap and the chevron, a spacing step at each end.
fn select_menu_width(ui: &Ui, t: &Theme, words: &str) -> f32 {
    let galley = ui.painter().layout_no_wrap(
        words.to_owned(),
        FontId::proportional(t.text.body),
        t.colors.text,
    );
    galley.size().x + t.spacing.xs + Icon::ChevronDown.size() + 2.0 * t.spacing.m
}

/// Select ▾: a quiet text button, its words then a chevron, as the bars' buttons are; its
/// pull-down is the caller's.
fn select_menu(ui: &mut Ui, t: &Theme, words: &str) -> egui::Response {
    let width = select_menu_width(ui, t, words);
    let (rect, response) =
        ui.allocate_exact_size(vec2(width, t.sizes.toolbar - 14.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let touch = Touch::of(ui.ctx(), &response, true, None);
        paint_plate(painter, rect, Plate::Quiet, touch, t);
        let galley = painter.layout_no_wrap(
            words.to_owned(),
            FontId::proportional(t.text.body),
            t.colors.text,
        );
        let x = rect.left() + t.spacing.m;
        let at = pos2(x, rect.center().y - galley.size().y / 2.0);
        let end = x + galley.size().x + t.spacing.xs;
        painter.galley(at, galley, t.colors.text);
        let side = Icon::ChevronDown.size();
        let chevron =
            Rect::from_min_size(pos2(end, rect.center().y - side / 2.0), vec2(side, side));
        Icon::ChevronDown.paint(painter, chevron, t.colors.text_secondary);
        paint_focus(painter, rect, t.shape.radius, touch, t);
    }
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, words));
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cells_shrink_to_the_largest_size_two_of_which_fit_never_past_the_zoom() {
        // 500 points less the 296-point column: two Small cells, the Zoom kept Medium.
        assert_eq!(
            shown_size(ThumbnailSize::Medium, 204.0),
            ThumbnailSize::Small
        );
        assert_eq!(grid_least(ThumbnailSize::Small.side()), 204.0);
        assert_eq!(
            shown_size(ThumbnailSize::Medium, 840.0),
            ThumbnailSize::Medium
        );
        assert_eq!(
            shown_size(ThumbnailSize::Large, 300.0),
            ThumbnailSize::Medium
        );
        assert_eq!(
            shown_size(ThumbnailSize::Small, 2000.0),
            ThumbnailSize::Small
        );
        assert_eq!(
            shown_size(ThumbnailSize::Medium, 100.0),
            ThumbnailSize::Small
        );
    }
}
