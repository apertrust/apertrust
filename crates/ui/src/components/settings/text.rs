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

//! A text in a settings row, in a field the column wide and `sizes.setting_control` high,
//! `sunken` with no frame, its text in `text_strong`:
//!
//! - [`Text`], one line, a tool's text field in the settings' look ([`TextField`]); a
//!   pattern in the mono face, offering the variables a path pattern may hold after `$(`
//!   as Export's path does ([`PathField`]).
//! - [`TextArea`], several lines, darktable's `longstring`: three lines high at rest,
//!   growing with its text to six, then scrolling; its grip at the end of its bottom edge
//!   makes it taller or shorter, never under three lines.
//! - [`Folder`], the field with a folder button a box high at its end
//!   (`Icon::ChooseFolder`), as Export's path row; the caller opens the folder dialog.
//!
//! To screen readers each is a text input named by the row's label, the button by its
//! words.

use std::hash::Hash;

use egui::{
    CursorIcon, Frame, Id, Margin, Rect, ScrollArea, Sense, Stroke, TextEdit, Ui, UiBuilder, pos2,
    vec2,
};

use super::super::plate::undimmed_painter;
use super::super::{IconButton, PathField, TextField, Variable, field_inset};
use super::{body, field_look, paint_well};
use crate::{Icon, theme};

/// The lines a text area shows at rest, and the most it grows to by itself.
const AREA_LINES: (f32, f32) = (3.0, 6.0);

/// One line of text.
pub struct Text<'a> {
    id: Id,
    name: &'a str,
    text: &'a str,
    hint: Option<&'a str>,
    mono: bool,
    variables: Option<&'a [Variable<'a>]>,
}

impl<'a> Text<'a> {
    /// `id` names it, stable across frames; `name` is the row's label, translated.
    pub fn new(id: impl Hash + std::fmt::Debug, name: &'a str, text: &'a str) -> Self {
        Text {
            id: Id::unique(("setting text", id)),
            name,
            text,
            hint: None,
            mono: false,
            variables: None,
        }
    }

    /// What it shows while empty, translated.
    pub fn hint(mut self, text: &'a str) -> Self {
        self.hint = Some(text);
        self
    }

    /// A pattern, in the mono face.
    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    /// A path pattern: in the mono face, offering `variables` after `$(`.
    pub fn variables(mut self, variables: &'a [Variable<'a>]) -> Self {
        self.variables = Some(variables);
        self.mono = true;
        self
    }

    /// Draws it across `ui`'s width; returns the whole text once the user changed it.
    pub fn show(self, ui: &mut Ui) -> Option<String> {
        let t = theme(ui.ctx());
        let width = ui.available_width();
        let look = field_look(&t, self.mono);
        match self.variables {
            Some(variables) => {
                let mut field = PathField::new(self.id, self.name, self.text, variables)
                    .width(width)
                    .look(look);
                if let Some(hint) = self.hint {
                    field = field.hint(hint);
                }
                field.show(ui).text
            }
            None => {
                let mut field = TextField::new(self.id, self.name, self.text)
                    .width(width)
                    .look(look);
                if let Some(hint) = self.hint {
                    field = field.hint(hint);
                }
                field.show(ui).1
            }
        }
    }
}

/// Several lines of text.
pub struct TextArea<'a> {
    id: Id,
    name: &'a str,
    text: &'a str,
}

impl<'a> TextArea<'a> {
    /// `id` names it, stable across frames; `name` is the row's label, translated.
    pub fn new(id: impl Hash + std::fmt::Debug, name: &'a str, text: &'a str) -> Self {
        TextArea {
            id: Id::unique(("setting text area", id)),
            name,
            text,
        }
    }

    /// Draws it across `ui`'s width; returns the whole text once the user changed it.
    pub fn show(self, ui: &mut Ui) -> Option<String> {
        let t = theme(ui.ctx());
        let enabled = ui.is_enabled();
        let width = ui.available_width();
        let font = body(&t);
        let pad = field_inset(&t);
        let line = ui.fonts_mut(|f| f.row_height(&font));
        // As tall as its text, from three lines to six, or as the grip left it.
        let inner = width - 2.0 * pad;
        let lines = ui
            .painter()
            .layout(self.text.to_owned(), font.clone(), t.colors.text, inner)
            .rows
            .len() as f32;
        let height_id = self.id.with("height");
        let least = AREA_LINES.0 * line + 2.0 * pad;
        let grown = lines.clamp(AREA_LINES.0, AREA_LINES.1) * line + 2.0 * pad;
        let height = ui
            .data(|d| d.get_temp::<f32>(height_id))
            .unwrap_or(grown)
            .max(least);
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let painter = undimmed_painter(ui);
        paint_well(&painter, rect, &t);
        let colour = super::dimmed(&t, enabled, t.colors.text_strong);
        let mut text = self.text.to_owned();
        let mut area = ui.new_child(UiBuilder::new().max_rect(rect.shrink(t.shape.hairline)));
        let visuals = area.visuals_mut();
        visuals.selection.bg_fill = t.colors.field_selection;
        visuals.text_cursor.stroke.color = t.colors.text;
        let response = ScrollArea::vertical()
            .id_salt(self.id.with("scroll"))
            .max_height(rect.height())
            .auto_shrink([false, false])
            .show(&mut area, |ui| {
                ui.add(
                    TextEdit::multiline(&mut text)
                        .id(self.id)
                        .frame(Frame::NONE.inner_margin(Margin::same(pad as i8)))
                        .font(font)
                        .text_color(colour)
                        .desired_width(f32::INFINITY)
                        .desired_rows(AREA_LINES.0 as usize),
                )
            })
            .inner;
        if response.has_focus() {
            super::ring(&painter, rect, &t);
        }
        let name = self.name.to_owned();
        ui.ctx()
            .accesskit_node_builder(self.id, |node| node.set_label(name));
        // The grip, at the end of its bottom edge.
        let side = t.sizes.icon;
        let grip = Rect::from_min_size(rect.max - vec2(side, side), vec2(side, side));
        if enabled {
            // A pointer's handle alone, no focus stop: the keys have nothing to do with it.
            let drag = ui.interact(grip, self.id.with("grip"), Sense::DRAG);
            // The edge follows the pointer, where it is, not by the movement each pass
            // reports, which a release in the same pass leaves out. The gesture is the
            // primary press's that landed on the grip: kept from it is how far below the
            // edge it took the grip, and a drag another button started resizes nothing.
            let offset_id = self.id.with("grip offset");
            let primary = egui::PointerButton::Primary;
            let pressed_here = ui.input(|i| {
                i.pointer
                    .press_origin()
                    .filter(|at| i.pointer.button_pressed(primary) && grip.contains(*at))
            });
            if let Some(at) = pressed_here {
                ui.data_mut(|d| d.insert_temp(offset_id, at.y - rect.bottom()));
            }
            let offset = ui.data(|d| d.get_temp::<f32>(offset_id));
            if drag.hovered() || offset.is_some() {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
            }
            // Held, where the pointer is; let go, where it was let go, though it moved on
            // in the same pass.
            let at = if drag.drag_stopped_by(primary) {
                ui.input(super::super::steps::released_at)
            } else if drag.dragged_by(primary) {
                drag.interact_pointer_pos()
            } else {
                None
            };
            if let (Some(at), Some(offset)) = (at, offset) {
                let dragged = (at.y - rect.top() - offset).max(least);
                ui.data_mut(|d| d.insert_temp(height_id, dragged));
            }
            if !ui.input(|i| i.pointer.primary_down()) {
                ui.data_mut(|d| d.remove::<f32>(offset_id));
            }
        }
        let stroke = Stroke::new(t.shape.hairline, t.colors.text_secondary);
        let corner = t.spacing.xxs;
        for at in [t.spacing.xs, t.spacing.m] {
            painter.line_segment(
                [
                    pos2(grip.right() - at, grip.bottom() - corner),
                    pos2(grip.right() - corner, grip.bottom() - at),
                ],
                stroke,
            );
        }
        (text != self.text).then_some(text)
    }
}

/// A folder's path, with the button that chooses one.
pub struct Folder<'a> {
    id: Id,
    name: &'a str,
    text: &'a str,
    choose: &'a str,
}

/// What a frame of a folder's row did.
#[derive(Debug, Default)]
pub struct FolderOutput {
    /// The whole path, once the user typed in it.
    pub text: Option<String>,
    /// The folder button was pressed: the caller opens the folder dialog.
    pub choose: bool,
}

impl<'a> Folder<'a> {
    /// `id` names it, stable across frames; `name` is the row's label and `choose` the
    /// button's name and tooltip, translated.
    pub fn new(
        id: impl Hash + std::fmt::Debug,
        name: &'a str,
        text: &'a str,
        choose: &'a str,
    ) -> Self {
        Folder {
            id: Id::unique(("setting folder", id)),
            name,
            text,
            choose,
        }
    }

    /// Draws it across `ui`'s width.
    pub fn show(self, ui: &mut Ui) -> FolderOutput {
        let t = theme(ui.ctx());
        let side = t.sizes.setting_control;
        let field = (ui.available_width() - side - t.spacing.s).max(0.0);
        let mut output = FolderOutput::default();
        let (strip, _) = ui.allocate_exact_size(vec2(ui.available_width(), side), Sense::hover());
        let mut start = ui.new_child(
            UiBuilder::new().max_rect(Rect::from_min_size(strip.min, vec2(field, side))),
        );
        output.text = TextField::new(self.id, self.name, self.text)
            .width(field)
            .look(field_look(&t, false))
            .show(&mut start)
            .1;
        let button = Rect::from_min_size(pos2(strip.right() - side, strip.top()), vec2(side, side));
        let mut end = ui.new_child(UiBuilder::new().max_rect(button));
        output.choose = IconButton::new(Icon::ChooseFolder, self.choose)
            .size(side)
            .show(&mut end)
            .clicked();
        output
    }
}
