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

//! A list of read-only facts (decision 110; `docs/gui.md`, The Metadata tool): rows of a
//! label and a value, as the Export tool's Summary lines are, the Metadata tool's groups
//! drawn with it.
//!
//! - **A row** is one line of 22 points: the label in `text_secondary`, right-aligned in a
//!   column as wide as the longest label of the lists drawn together, at most 40% of the
//!   row ([`FactList::column`]), cut at its end beyond that with the whole label in its
//!   tooltip; then, 8 points after it, the value in `text`, in the small text, cut with …
//!   in its middle (a path, a file name keeping its extension) or at its end, the whole value
//!   in its tooltip. No value reads "—" in `text_disabled`, its tooltip saying why, and
//!   screen readers hear "None". An end item, an icon button, may follow the value.
//! - **Under the pointer** the `hover` band, and the arrow, never an I-beam.
//! - **The keys:** the list is one Tab stop, the row last focused, the first at first; ↑ and
//!   ↓ move between its rows, Home and End to its ends, and Enter presses the focused row's
//!   end item. A click focuses a row and selects its value; Ctrl+C (Cmd+C) copies the value
//!   as shown.
//! - **To screen readers** it is a List named as the caller says, each row an item named
//!   "Label: value".

use egui::{
    Event, FontId, Key, Modifiers, Rect, Role, Sense, Ui, UiBuilder, WidgetInfo, pos2, vec2,
};

use super::grid_cell::paint_labels;
use super::{ColourLabels, IconButton, focus_ring_centred, middle_cut, truncated};
use crate::{Icon, theme, translator};

/// A row's height, in points: a fact's line, shorter than a tool's rows (`panel_line`),
/// which no token holds.
const ROW: f32 = 22.0;
/// The largest part of a row the labels' column takes.
const LABEL_SHARE: f32 = 0.4;

/// Where a value too long for its row is cut with ….
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FactCut {
    /// In its middle, as a path.
    Middle,
    /// In its middle, its extension kept, as a file name.
    File,
    /// At its end, as a name.
    End,
}

/// An icon button at a row's end, as the Metadata tool's Open This Folder.
#[derive(Clone, Copy, Debug)]
pub struct FactEnd<'a> {
    pub icon: Icon,
    /// Its name, translated.
    pub name: &'a str,
    pub enabled: bool,
    /// Its tooltip, translated: why it is disabled, where it is.
    pub tip: Option<&'a str>,
}

/// One row of a [`FactList`]: its words translated, its value as shown.
#[derive(Clone, Copy, Debug)]
pub struct Fact<'a> {
    pub label: &'a str,
    /// The value as shown, whole; `None` reads "—".
    pub value: Option<&'a str>,
    /// What its tooltip says: the whole value or why it has none, then its source.
    pub tip: &'a str,
    pub cut: FactCut,
    pub end: Option<FactEnd<'a>>,
    /// A disabled row is drawn in `text_disabled` and takes no focus.
    pub enabled: bool,
}

/// What the user did to a list this frame, each a row's index.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FactListOutput {
    /// The row the focus is on.
    pub focused: Option<usize>,
    /// The row whose end item was pressed, or Enter on it.
    pub end: Option<usize>,
    /// The row whose value was copied.
    pub copied: Option<usize>,
}

pub struct FactList<'a> {
    name: &'a str,
    facts: &'a [Fact<'a>],
    column: Option<f32>,
}

impl<'a> FactList<'a> {
    /// The list `facts`, named `name`, translated, for screen readers.
    pub fn new(name: &'a str, facts: &'a [Fact<'a>]) -> Self {
        FactList {
            name,
            facts,
            column: None,
        }
    }

    /// The labels' column `width`, as [`FactList::column`] measures it for every list drawn
    /// together; else this list's own.
    pub fn column_width(mut self, width: f32) -> Self {
        self.column = Some(width);
        self
    }

    /// How wide the labels' column is for `labels`, drawn in `ui`: the widest, at most 40%
    /// of the row.
    pub fn column<'l>(ui: &Ui, labels: impl IntoIterator<Item = &'l str>) -> f32 {
        let t = theme(ui.ctx());
        let font = FontId::proportional(t.text.small);
        let widest = (labels.into_iter())
            .map(|label| width_of(ui, &font, label))
            .fold(0.0, f32::max);
        widest.min(ui.available_width() * LABEL_SHARE).ceil()
    }

    pub fn show(self, ui: &mut Ui) -> FactListOutput {
        // A group may hold two lists apart, as Basic's round its Rating: each is told by its
        // first row too.
        let first = self.facts.first().map(|fact| fact.label);
        let base = ui.make_persistent_id(("fact list", self.name, first));
        let column = self
            .column
            .unwrap_or_else(|| FactList::column(ui, self.facts.iter().map(|fact| fact.label)));
        let ids: Vec<egui::Id> = (0..self.facts.len()).map(|i| base.with(i)).collect();
        let current_id = base.with("current");
        let mut current = ui
            .data(|d| d.get_temp::<usize>(current_id))
            .unwrap_or(0)
            .min(self.facts.len().saturating_sub(1));
        let mut output = FactListOutput::default();
        // The keys of the row the focus is on, before any row is drawn, so the one moved to
        // takes the focus as it is drawn.
        let focused = ids.iter().position(|id| ui.memory(|m| m.has_focus(*id)));
        if let Some(at) = focused {
            current = at;
            let filter = egui::EventFilter {
                vertical_arrows: true,
                ..Default::default()
            };
            ui.memory_mut(|m| m.set_focus_lock_filter(ids[at], filter));
            let to = keyed(ui, at, self.facts.len());
            if let Some(to) = to.filter(|to| *to != at) {
                current = to;
                ui.memory_mut(|m| m.request_focus(ids[to]));
            }
            let pressed = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
            if pressed && self.facts[at].end.is_some_and(|end| end.enabled) {
                output.end = Some(at);
            }
            let copy = ui.input_mut(|i| {
                let asked = i.events.iter().any(|e| matches!(e, Event::Copy));
                i.events.retain(|e| !matches!(e, Event::Copy));
                asked
            });
            if copy && let Some(value) = self.facts[at].value {
                ui.ctx().copy_text(value.to_owned());
                output.copied = Some(at);
            }
        }
        ui.scope_builder(
            UiBuilder::new()
                .id_salt(base)
                .accessibility_role(Role::List)
                .accessibility_label(self.name),
            |ui| {
                for (at, fact) in self.facts.iter().enumerate() {
                    let row = Row {
                        id: ids[at],
                        fact,
                        column,
                        tab_stop: at == current,
                    };
                    let drawn = row.show(ui);
                    if drawn.clicked {
                        current = at;
                        ui.memory_mut(|m| m.request_focus(ids[at]));
                    }
                    if drawn.end {
                        output.end = Some(at);
                    }
                }
            },
        );
        ui.data_mut(|d| d.insert_temp(current_id, current));
        output.focused = ids.iter().position(|id| ui.memory(|m| m.has_focus(*id)));
        output
    }
}

/// The row ↑, ↓, Home or End move the focus to from `at`, of `count` rows.
fn keyed(ui: &Ui, at: usize, count: usize) -> Option<usize> {
    let pressed = |key| ui.input_mut(|i| i.consume_key(Modifiers::NONE, key));
    if pressed(Key::ArrowUp) {
        return Some(at.saturating_sub(1));
    }
    if pressed(Key::ArrowDown) {
        return Some((at + 1).min(count - 1));
    }
    if pressed(Key::Home) {
        return Some(0);
    }
    if pressed(Key::End) {
        return Some(count - 1);
    }
    None
}

fn width_of(ui: &Ui, font: &FontId, text: &str) -> f32 {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE);
    galley.size().x
}

/// One row as drawn this frame.
struct Row<'f> {
    id: egui::Id,
    fact: &'f Fact<'f>,
    column: f32,
    /// Whether it is the list's one Tab stop.
    tab_stop: bool,
}

/// What a row reports.
struct Drawn {
    clicked: bool,
    end: bool,
}

impl Row<'_> {
    fn show(self, ui: &mut Ui) -> Drawn {
        let t = theme(ui.ctx());
        let tr = translator(ui.ctx());
        let fact = self.fact;
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::hover());
        let sense = match (fact.enabled, self.tab_stop) {
            (false, _) => Sense::hover(),
            (true, true) => Sense::click(),
            (true, false) => Sense::CLICK,
        };
        // The end item first, so a click on it is its own.
        let mut end_pressed = false;
        let mut value_right = rect.right();
        if let Some(end) = fact.end {
            let size = t.sizes.panel_line;
            let at = Rect::from_min_size(
                pos2(rect.right() - size, rect.center().y - size / 2.0),
                vec2(size, size),
            );
            value_right = at.left() - t.spacing.xs;
            let button = ui
                .scope_builder(UiBuilder::new().max_rect(at), |ui| {
                    let mut button = IconButton::new(end.icon, end.name)
                        .width(size)
                        .enabled(end.enabled && fact.enabled);
                    if let Some(tip) = end.tip {
                        button = button.tip(tip);
                    }
                    button.show(ui)
                })
                .inner;
            end_pressed = button.clicked();
        }
        let hit = Rect::from_min_max(rect.min, pos2(value_right, rect.max.y));
        let response = ui.interact(hit, self.id, sense);
        let none = tr.text("fact-none");
        let said = format!("{}: {}", fact.label, fact.value.unwrap_or(&none));
        response.widget_info(|| WidgetInfo::labeled(Role::ListItem, fact.enabled, &said));

        let painter = ui.painter_at(rect);
        let focused = response.has_focus();
        let band = rect.expand2(vec2(t.spacing.xs, 0.0));
        if response.hovered() && fact.enabled {
            painter.rect_filled(band, t.shape.radius, t.colors.hover);
        }
        let font = FontId::proportional(t.text.small);
        let middle = rect.center().y;
        let label_colour = if fact.enabled {
            t.colors.text_secondary
        } else {
            t.colors.text_disabled
        };
        let label = painter.layout_job(truncated(
            fact.label,
            font.clone(),
            self.column,
            label_colour,
        ));
        let label_cut = label.elided;
        painter.galley(
            pos2(
                rect.left() + self.column - label.size().x,
                middle - label.size().y / 2.0,
            ),
            label,
            label_colour,
        );
        let left = rect.left() + self.column + t.spacing.m;
        let room = (value_right - left).max(0.0);
        let (shown, colour) = match fact.value {
            Some(value) => {
                let shown = match fact.cut {
                    FactCut::End => None,
                    FactCut::Middle => Some(middle_cut(&painter, value, "", &font, room)),
                    FactCut::File => {
                        let (stem, tail) = match value.rfind('.') {
                            Some(dot) if dot > 0 => value.split_at(dot),
                            _ => (value, ""),
                        };
                        Some(middle_cut(&painter, stem, tail, &font, room))
                    }
                };
                let colour = if fact.enabled {
                    t.colors.text
                } else {
                    t.colors.text_disabled
                };
                (shown.unwrap_or_else(|| value.to_owned()), colour)
            }
            None => ("—".to_owned(), t.colors.text_disabled),
        };
        let value = painter.layout_job(truncated(&shown, font, room, colour));
        let value_rect =
            Rect::from_min_size(pos2(left, middle - value.size().y / 2.0), value.size());
        // A focused row's value is selected, as text is.
        if focused && fact.value.is_some() {
            let selected = value_rect.expand(t.shape.hairline);
            painter.rect_filled(selected, t.shape.radius_small, t.colors.selection);
        }
        painter.galley(value_rect.min, value, colour);
        if focused {
            focus_ring_centred(ui.painter(), band, &t);
        }
        let tip = if label_cut {
            format!("{}\n{}", fact.label, fact.tip)
        } else {
            fact.tip.to_owned()
        };
        let clicked = response.clicked();
        if !tip.is_empty() {
            response.on_hover_text(tip);
        }
        Drawn {
            clicked,
            end: end_pressed,
        }
    }
}

/// What a [`FactMarks`] row shows: Rating's five stars or Colour labels' five rings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Marks {
    Stars,
    Labels,
}

/// A row of five marks laid out as a [`FactList`]'s rows, its label in their column, drawn
/// disabled until what the marks call is built: a grid cell's empty stars and its ring of no
/// labels. Not a Tab stop; screen readers read "Label: tip".
pub struct FactMarks<'a> {
    label: &'a str,
    tip: &'a str,
    marks: Marks,
    column: f32,
}

impl<'a> FactMarks<'a> {
    /// The row `label`, translated, saying `tip` on hover, in the labels' `column`
    /// ([`FactList::column`]).
    pub fn new(label: &'a str, tip: &'a str, marks: Marks, column: f32) -> Self {
        FactMarks {
            label,
            tip,
            marks,
            column,
        }
    }

    pub fn show(self, ui: &mut Ui) {
        let t = theme(ui.ctx());
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::hover());
        let said = format!("{}: {}", self.label, self.tip);
        response.widget_info(|| WidgetInfo::labeled(Role::Label, false, &said));
        let painter = ui.painter_at(rect);
        let font = FontId::proportional(t.text.small);
        let colour = t.colors.text_disabled;
        let label = painter.layout_job(truncated(self.label, font, self.column, colour));
        let label_cut = label.elided;
        let middle = rect.center().y;
        painter.galley(
            pos2(
                rect.left() + self.column - label.size().x,
                middle - label.size().y / 2.0,
            ),
            label,
            colour,
        );
        let side = t.sizes.icon;
        let mut x = rect.left() + self.column + t.spacing.m;
        for _ in 0..5 {
            let mark = Rect::from_min_size(pos2(x, middle - side / 2.0), vec2(side, side));
            match self.marks {
                Marks::Stars => Icon::Star.paint(&painter, mark, colour),
                // A ring a star's inner size, as the grid cell's beside its stars.
                Marks::Labels => {
                    let ring = mark.shrink(t.spacing.xxs);
                    paint_labels(&painter, &t, ring, ColourLabels::default());
                }
            }
            x += side + t.spacing.xs;
        }
        let tip = if label_cut {
            format!("{}\n{}", self.label, self.tip)
        } else {
            self.tip.to_owned()
        };
        response.on_hover_text(tip);
    }
}
