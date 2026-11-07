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

//! The grid panel's edge towards the central panel, which resizes it in whole lines of
//! cells (`docs/gui.md`, The grid panel, Its size): the side panel's `PanelEdge` laid along
//! the grid panel's top, or its side when it is a column, stepping by rows or columns
//! rather than points.

use egui::accesskit::{Action, Orientation};
use egui::{
    Context, CursorIcon, EventFilter, Id, Key, Rect, Response, Role, Sense, Stroke, Ui, WidgetInfo,
    vec2,
};

use super::focus_ring;
use super::grid_panel::{GAP, GridPosition, ThumbnailSize};
use crate::i18n::Args;
use crate::{theme, tr, translator};

/// What the user did to the grid panel's edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridEdgeAction {
    /// The panel's new number of lines, within those it may show.
    Resize(usize),
    /// Back to one line: a double-click on the edge, or Delete while it has focus.
    Reset,
}

/// The edge between the grid panel and the central panel, the separator on the panel's
/// side towards it: its top at the bottom of the window, which resizes it in whole rows,
/// its right at the left and its left at the right, in whole columns. It brightens from
/// `separator` to `border` while the pointer is over it, drags it or it has focus.
///
/// - **The pointer**: dragged, the panel gains or loses a line as the pointer passes the
///   middle of the next one, within the lines it may show; double-clicked, it goes back to
///   one. The row-resize or column-resize cursor shows over it, and it takes the pointer a
///   few points either side of the line (egui's `resize_grab_radius_side`). A drag leaves
///   the focus where it was, so the arrow keys still go from photo to photo.
/// - **The keyboard**: it takes focus with Tab, with a focus ring. Then the arrow towards
///   the central panel adds a line and the other takes one away (↑ and ↓ at the bottom),
///   Home and End take the panel to its fewest and most, and Delete (or Backspace, the
///   Mac's Delete) goes back to one.
/// - **Screen readers** find a splitter, "Grid panel height" at the bottom and "Grid panel
///   width" at a side, whose value is the number of lines ("2 rows", "3 columns"), as
///   dragged this frame, which they step by one.
///
/// It is made before the panel is laid out, so that [`GridEdge::lines`] gives the lines a
/// drag asks for this frame and the panel follows the pointer without a frame's delay,
/// and shown after the panel and what lies beside it, so its grab zone is on top of both.
pub struct GridEdge {
    id: Id,
    /// The lines kept, which a smaller window may show fewer of.
    lines: usize,
    /// The most lines the panel may show in the window now.
    most: usize,
    size: ThumbnailSize,
    labels: bool,
    position: GridPosition,
}

impl GridEdge {
    /// The edge of a grid panel along the bottom keeping `lines` rows of thumbnails of
    /// `size`, with their captions, showing at most `most`.
    pub fn new(lines: usize, most: usize, size: ThumbnailSize) -> Self {
        GridEdge {
            id: Id::unique("apertrust-ui grid edge"),
            lines,
            most: most.max(1),
            size,
            labels: true,
            position: GridPosition::Bottom,
        }
    }

    /// Whether the cells show their captions, which a row's height counts; on by default.
    pub fn labels(mut self, labels: bool) -> Self {
        self.labels = labels;
        self
    }

    /// For the panel at `position` instead: at a side its lines are columns.
    pub fn position(mut self, position: GridPosition) -> Self {
        self.position = position;
        self
    }

    /// The lines to lay the panel out with this frame: those kept, as many as fit, or where
    /// a drag of the edge has taken them.
    pub fn lines(&self, ctx: &Context) -> usize {
        ctx.read_response(self.id)
            .and_then(|response| self.dragged(ctx, &response))
            .unwrap_or_else(|| self.shown())
    }

    /// Draws the edge of `panel`, laid out with [`GridEdge::lines`], and reports what the
    /// user did. Shown after the panel and what lies beside it.
    pub fn show(self, ui: &mut Ui, panel: Rect) -> Option<GridEdgeAction> {
        let t = theme(ui.ctx());
        let grab = ui.style().interaction.resize_grab_radius_side;
        let half = t.shape.separator_width / 2.0;
        // Where the line is, and the zone that takes the pointer round it.
        let (line, rect) = match self.position {
            GridPosition::Bottom => {
                let y = panel.top();
                (
                    y + half,
                    Rect::from_x_y_ranges(panel.x_range(), y - grab..=y + grab),
                )
            }
            GridPosition::Left => {
                let x = panel.right();
                (
                    x - half,
                    Rect::from_x_y_ranges(x - grab..=x + grab, panel.y_range()),
                )
            }
            GridPosition::Right => {
                let x = panel.left();
                (
                    x + half,
                    Rect::from_x_y_ranges(x - grab..=x + grab, panel.y_range()),
                )
            }
        };
        let response = ui.interact(rect, self.id, Sense::click_and_drag());
        let shown = self.shown();
        let column = self.position.is_column();

        // What the pointer, the keys and screen readers ask for, in that order.
        let mut reset = response.double_clicked();
        let mut lines = self.dragged(ui.ctx(), &response).unwrap_or(shown);
        if response.has_focus() {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    self.id,
                    EventFilter {
                        horizontal_arrows: column,
                        vertical_arrows: !column,
                        ..Default::default()
                    },
                );
            });
            // The arrow towards the central panel adds a line.
            let (more, fewer) = match self.position {
                GridPosition::Bottom => (Key::ArrowUp, Key::ArrowDown),
                GridPosition::Left => (Key::ArrowRight, Key::ArrowLeft),
                GridPosition::Right => (Key::ArrowLeft, Key::ArrowRight),
            };
            ui.input(|i| {
                if i.key_pressed(more) {
                    lines += 1;
                }
                if i.key_pressed(fewer) {
                    lines = lines.saturating_sub(1);
                }
                if i.key_pressed(Key::Home) {
                    lines = 1;
                }
                if i.key_pressed(Key::End) {
                    lines = self.most;
                }
                reset |= i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace);
            });
        }
        let steps = ui.input(|i| {
            i.num_accesskit_action_requests(self.id, Action::Increment) as i64
                - i.num_accesskit_action_requests(self.id, Action::Decrement) as i64
        });
        let lines = self.within(lines as i64 + steps);

        let active = response.hovered() || response.dragged();
        if active {
            let cursor = if column {
                CursorIcon::ResizeColumn
            } else {
                CursorIcon::ResizeRow
            };
            ui.ctx().set_cursor_icon(cursor);
        }
        let colour = if active || response.has_focus() {
            t.colors.border
        } else {
            t.colors.separator
        };
        let stroke = Stroke::new(t.shape.separator_width, colour);
        if column {
            ui.painter().vline(line, panel.y_range(), stroke);
        } else {
            ui.painter().hline(panel.x_range(), line, stroke);
        }
        if response.has_focus() {
            // Inside the panel's length, so the ring never runs past its ends.
            let inset = if column {
                vec2(0.0, 2.0)
            } else {
                vec2(2.0, 0.0)
            };
            focus_ring(ui.painter(), rect.shrink2(inset), &t);
        }

        // For screen readers: a splitter between the photo and the panel, its value the
        // lines as this frame leaves them.
        let (name, unit) = if column {
            ("grid-panel-width", "grid-columns")
        } else {
            ("grid-panel-height", "grid-rows")
        };
        let name = tr(ui.ctx(), name);
        let mut args = Args::new();
        args.set("count", lines);
        let value = translator(ui.ctx()).text_with(unit, Some(&args));
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(Role::Splitter, true, &name);
            info.value = Some(lines as f64);
            info
        });
        let most = self.most;
        ui.ctx().accesskit_node_builder(self.id, |node| {
            // A splitter's orientation is its line's.
            node.set_orientation(if column {
                Orientation::Vertical
            } else {
                Orientation::Horizontal
            });
            node.set_value(value);
            node.set_min_numeric_value(1.0);
            node.set_max_numeric_value(most as f64);
            node.set_numeric_value_step(1.0);
            if lines < most {
                node.add_action(Action::Increment);
            }
            if lines > 1 {
                node.add_action(Action::Decrement);
            }
        });

        let action = if reset {
            Some(GridEdgeAction::Reset)
        } else {
            (lines != shown).then_some(GridEdgeAction::Resize(lines))
        };
        if action.is_some() {
            ui.ctx().request_repaint();
        }
        action
    }

    /// From one line's start to the next's, in points: a row's cells with their captions,
    /// or a column's.
    fn pitch(&self) -> f32 {
        let cell = self.size.cell(self.labels);
        if self.position.is_column() {
            cell.x + GAP
        } else {
            cell.y + GAP
        }
    }

    /// The lines shown: those kept, as many as fit.
    fn shown(&self) -> usize {
        self.within(self.lines as i64)
    }

    /// The lines a drag of the edge asks for: those it started with, and one more for every
    /// line's thickness the pointer has moved towards the central panel since the press, a
    /// line joining as the pointer passes its middle.
    fn dragged(&self, ctx: &Context, response: &Response) -> Option<usize> {
        if !(response.dragged() || response.drag_stopped()) {
            return None;
        }
        let along = |p: egui::Pos2| match self.position {
            GridPosition::Bottom => -p.y,
            GridPosition::Left => p.x,
            GridPosition::Right => -p.x,
        };
        let pointer = along(response.interact_pointer_pos()?);
        // The lines and the press's place, kept from the drag's start, since the press
        // origin goes with the release.
        let (start, origin) = if response.drag_started() {
            let origin = along(ctx.input(|i| i.pointer.press_origin())?);
            let start = (self.shown(), origin);
            ctx.data_mut(|d| d.insert_temp(self.id, start));
            start
        } else {
            ctx.data(|d| d.get_temp::<(usize, f32)>(self.id))?
        };
        let moved = ((pointer - origin) / self.pitch()).round() as i64;
        Some(self.within(start as i64 + moved))
    }

    /// `lines` within one and the most the panel may show.
    fn within(&self, lines: i64) -> usize {
        lines.clamp(1, self.most as i64) as usize
    }
}
