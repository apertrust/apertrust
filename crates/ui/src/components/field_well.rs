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

//! A field row (decision 110; `docs/gui.md`, The Metadata tool): an editable value of a
//! utility, the Metadata tool's Title, drawn as a label and a well, edited in place and
//! saved as it is committed, with no Apply.
//!
//! - **The row:** the label in `text_secondary`, right-aligned in the facts' column
//!   ([`super::FactList::column`]) and top-aligned with the well's first line, then, 8 points
//!   after it, the well to the row's end, `sunken` with the small radius and no border, its
//!   text 5 points in, in the small text, one line `sizes.field` high in a `panel_line` row.
//!   A note may follow the well in `text_secondary` ("2 of 3").
//! - **At rest** the well shows up to three lines, the rest cut with …, the whole text in
//!   its tooltip; empty, it is one line, with no hint. **Under the pointer** it is
//!   `sunken_hover` with the I-beam. **Editing**, it grows to six lines, then scrolls, with
//!   no border, as a value field; given the focus by Tab, its whole text is selected and
//!   the focus ring drawn round it.
//! - **Commit and cancel:** Return, Tab, Shift+Tab, a press elsewhere, the window losing the
//!   focus and closing commit ([`FieldOutput::committed`], the text whole, only where it
//!   changed); Esc puts the text back as it was when editing began and commits nothing
//!   ([`FieldOutput::cancelled`]). In a field that takes line breaks Shift+Return and
//!   Ctrl+Return break the line; elsewhere they commit as Return does. Return gives the focus
//!   up, so the keys of what is under it act again. Ctrl+Z and Ctrl+Y undo and redo typing.
//! - **A warning button** may sit at the well's end, in `warning`, its menu the caller's
//!   ([`FieldWell::warning`]); **a pull-down of fixed values** may stand in the text's place
//!   ([`FieldWell::choice`]).
//! - **Different,** for a field several photos hold apart ([`FieldWell::different`]): the
//!   word in `text_secondary` where a hint would sit, until the first character typed
//!   replaces it; nothing typed commits nothing. **A ⌄** at the well's end opens the
//!   caller's list of the values ([`FieldWell::values`]), as Alt+↓ does while the field has
//!   the focus. A pull-down of fixed values reads Different as its value, no option
//!   checked, where a choice is no "nothing typed".
//! - **Disabled,** the text in `text_disabled`, no Tab stop, its tooltip the caller's
//!   reason.
//! - **To screen readers** it is a text input named by its label, multi-line where it takes
//!   line breaks.

use egui::text::{CCursor, CCursorRange, CharIndex, LayoutJob, TextWrapping};
use egui::{
    CursorIcon, Event, FontId, Frame, Id, Key, Margin, Modifiers, Rect, Role, ScrollArea, Sense,
    Stroke, StrokeKind, TextBuffer, TextEdit, Ui, UiBuilder, WidgetInfo, pos2, vec2,
};

use super::{field_inset, focus_ring, pull_down, truncated};
use crate::popup::{self, Entry, Item, Request};
use crate::{Icon, theme};

/// The lines a well shows at rest, and the most it grows to while edited.
const LINES: (usize, usize) = (3, 6);

/// A field row: its label and its well.
pub struct FieldWell<'a> {
    id: Id,
    label: &'a str,
    text: &'a str,
    column: f32,
    lines: bool,
    enabled: bool,
    tip: Option<&'a str>,
    after: Option<&'a str>,
    warning: Option<(&'a str, Vec<Entry>)>,
    choice: Option<&'a [(&'a str, &'a str)]>,
    different: Option<(&'a str, &'a str)>,
    values: Option<(&'a str, Vec<Entry>)>,
}

/// What a frame of the field did.
#[derive(Debug, Default)]
pub struct FieldOutput {
    /// The whole text as committed, where it changed since editing began.
    pub committed: Option<String>,
    /// Esc put the text back.
    pub cancelled: bool,
    /// Whether it is being edited after this frame.
    pub editing: bool,
    /// Whether editing began this frame, which a caller marks the photo it edits at.
    pub began: bool,
    /// The item chosen this frame in its warning button's menu, its values list, or the
    /// value chosen in its pull-down.
    pub chosen: Option<String>,
}

/// What a field keeps while it is edited: the text as editing began, and what was typed.
#[derive(Clone, Debug)]
struct Editing {
    original: String,
    typed: String,
    /// Whether Tab gave it the focus, which draws the ring.
    by_key: bool,
}

impl<'a> FieldWell<'a> {
    /// `id` must be stable across frames; `label` is the row's label, translated, which
    /// names the field; `text` is its value as shown, `column` the labels' column's width.
    pub fn new(id: Id, label: &'a str, text: &'a str, column: f32) -> Self {
        FieldWell {
            id,
            label,
            text,
            column,
            lines: false,
            enabled: true,
            tip: None,
            after: None,
            warning: None,
            choice: None,
            different: None,
            values: None,
        }
    }

    /// Whether it takes line breaks, as Description and Notes do.
    pub fn lines(mut self, lines: bool) -> Self {
        self.lines = lines;
        self
    }

    /// A disabled field takes no input and no Tab stop.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Its tooltip, translated, after the whole text where the well cuts it: the field's
    /// standard and property, or why it is disabled.
    pub fn tip(mut self, tip: &'a str) -> Self {
        self.tip = Some(tip);
        self
    }

    /// A note after the well, in `text_secondary`: "2 of 3".
    pub fn after(mut self, note: &'a str) -> Self {
        self.after = Some(note);
        self
    }

    /// A warning button at the well's end, named `name`, opening `menu`, whose item chosen
    /// comes back as [`FieldOutput::chosen`].
    pub fn warning(mut self, name: &'a str, menu: Vec<Entry>) -> Self {
        self.warning = Some((name, menu));
        self
    }

    /// A pull-down of `options`, each its id and words, in the text's place, the one whose
    /// id is the field's text checked; the one chosen comes back as
    /// [`FieldOutput::chosen`].
    pub fn choice(mut self, options: &'a [(&'a str, &'a str)]) -> Self {
        self.choice = Some(options);
        self
    }

    /// The photos it describes hold it apart (decision 110): `word`, "Different", drawn in
    /// `text_secondary` while nothing is typed, its text empty; to screen readers its
    /// value, described as `description`, "12 photos, 3 different titles. Typing replaces
    /// them all."
    pub fn different(mut self, word: &'a str, description: &'a str) -> Self {
        self.different = Some((word, description));
        self
    }

    /// A ⌄ at the well's end, named `name`, opening `menu`, as Alt+↓ does while the field
    /// has the focus; the item chosen comes back as [`FieldOutput::chosen`].
    pub fn values(mut self, name: &'a str, menu: Vec<Entry>) -> Self {
        self.values = Some((name, menu));
        self
    }

    pub fn show(mut self, ui: &mut Ui) -> FieldOutput {
        let t = theme(ui.ctx());
        let font = FontId::proportional(t.text.small);
        let line = ui.fonts_mut(|f| f.row_height(&font));
        let one = t.sizes.field;
        let pad = (field_inset(&t), ((one - line) / 2.0).max(0.0));
        let row_pad = ((t.sizes.panel_line - one) / 2.0).max(0.0);
        let mut out = FieldOutput::default();
        let width = ui.available_width();
        let left = self.column + t.spacing.m;
        // The note after the well, and its room.
        let note = self.after.map(|note| {
            let galley =
                ui.painter()
                    .layout_no_wrap(note.to_owned(), font.clone(), t.colors.text_secondary);
            let room = galley.size().x + t.spacing.m;
            (galley, room)
        });
        let note_room = note.as_ref().map_or(0.0, |(_, room)| *room);
        let well_width = (width - left - note_room).max(t.sizes.control);
        let warning = self.warning.take();
        let values = self.values.take();
        // The buttons at the well's end: the warning, then ⌄.
        let buttons = usize::from(warning.is_some()) + usize::from(values.is_some());
        let warning_side = buttons as f32 * one;
        let wrap = (well_width - 2.0 * pad.0 - warning_side).max(1.0);

        let key = self.id.with("editing");
        let ctx = ui.ctx().clone();
        let mut editing: Option<Editing> = ctx.data(|d| d.get_temp(key));
        let focused = ctx.memory(|m| m.has_focus(self.id));
        // The keys that end it, taken before the text edit sees them.
        let mut commit = false;
        if let Some(editing) = &mut editing
            && focused
        {
            let breaks = |m: Modifiers| self.lines && (m.shift || m.ctrl);
            let pressed = ui.input(|i| {
                (i.events.iter()).find_map(|event| match event {
                    Event::Key {
                        key: Key::Enter,
                        pressed: true,
                        modifiers,
                        ..
                    } => Some(*modifiers),
                    _ => None,
                })
            });
            match pressed {
                // Shift+Return is the text edit's own return key; Ctrl+Return breaks the
                // line here.
                Some(m) if breaks(m) && m.ctrl => {
                    ui.input_mut(|i| i.consume_key(m, Key::Enter));
                    insert_break(&ctx, self.id, &mut editing.typed);
                }
                Some(m) if breaks(m) => {}
                Some(m) => {
                    ui.input_mut(|i| i.consume_key(m, Key::Enter));
                    commit = true;
                }
                None => {}
            }
        }
        // Alt+↓ opens the values list, as the ⌄ does.
        let list_asked = values.is_some()
            && focused
            && ui.input_mut(|i| i.consume_key(Modifiers::ALT, Key::ArrowDown));
        let lines_shown = |text: &str, most: usize| -> usize {
            let galley = ui
                .painter()
                .layout(text.to_owned(), font.clone(), t.colors.text, wrap);
            galley.rows.len().clamp(1, most)
        };
        let shown_text = editing.as_ref().map_or(self.text, |e| e.typed.as_str());
        let empty = shown_text.is_empty();
        let rows = match &editing {
            Some(_) => lines_shown(shown_text, LINES.1),
            None => lines_shown(shown_text, LINES.0),
        };
        let well_height = one + (rows as f32 - 1.0) * line;
        let height = well_height + 2.0 * row_pad;
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let well = Rect::from_min_size(
            pos2(rect.left() + left, rect.top() + row_pad),
            vec2(well_width, well_height),
        );
        // The label, top-aligned with the well's first line.
        let muted = if self.enabled {
            t.colors.text_secondary
        } else {
            t.colors.text_disabled
        };
        let label =
            (ui.painter()).layout_job(truncated(self.label, font.clone(), self.column, muted));
        ui.painter().galley(
            pos2(
                rect.left() + self.column - label.size().x,
                well.top() + (one - label.size().y) / 2.0,
            ),
            label,
            muted,
        );
        if let Some((galley, _)) = note {
            let at = pos2(
                well.right() + t.spacing.m,
                well.top() + (one - galley.size().y) / 2.0,
            );
            ui.painter().galley(at, galley, t.colors.text_secondary);
        }
        // A press elsewhere ends it in this frame, before what was pressed acts.
        let pressed_elsewhere = ui.input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .press_origin()
                    .is_some_and(|at| !well.contains(at))
        });
        let window_left = ui.input(|i| {
            i.viewport().close_requested()
                || (i.events.iter()).any(|event| matches!(event, Event::WindowFocused(false)))
        });
        if editing.is_some() && focused && (pressed_elsewhere || window_left) {
            commit = true;
        }

        let hovered = self.enabled && ui.rect_contains_pointer(well) && editing.is_none();
        let fill = if hovered {
            t.colors.sunken_hover
        } else {
            t.colors.sunken
        };
        let painter = ui.painter();
        painter.rect_filled(well, t.shape.radius_small, fill);
        // The high-contrast theme's edge, where every ground is black and a well has no tone.
        if t.colors.button_edge.a() > 0 {
            let edge = Stroke::new(t.shape.hairline, t.colors.button_edge);
            painter.rect_stroke(well, t.shape.radius_small, edge, StrokeKind::Inside);
        }

        if let Some(options) = self.choice {
            let field = Rect::from_min_max(well.min, well.max - vec2(warning_side, 0.0));
            out.chosen = self.pull_down(ui, field, options, &font);
            if let Some((name, menu)) = warning {
                let at = well.right() - warning_side;
                out.chosen = out.chosen.or(warning_button(
                    ui,
                    self.id,
                    (at, well.top()),
                    one,
                    name,
                    menu,
                ));
            }
            return out;
        }

        // The text: a text edit while edited; at rest the same, its text cut at three lines.
        let colour = match (self.enabled, editing.is_some()) {
            (false, _) => t.colors.text_disabled,
            (true, true) => t.colors.text_strong,
            (true, false) => t.colors.text,
        };
        let mut rest = self.text.to_owned();
        let at_rest = editing.is_none();
        let text: &mut String = match &mut editing {
            Some(editing) => &mut editing.typed,
            None => &mut rest,
        };
        let inner = Rect::from_min_max(well.min, well.max - vec2(warning_side, 0.0));
        let mut child = ui.new_child(UiBuilder::new().max_rect(inner));
        let visuals = child.visuals_mut();
        visuals.selection.bg_fill = t.colors.field_selection;
        visuals.text_cursor.stroke.color = t.colors.text;
        let cut_font = font.clone();
        let mut layouter = |ui: &Ui, text: &dyn TextBuffer, wrap_width: f32| {
            let mut job = LayoutJob::simple(
                text.as_str().to_owned(),
                cut_font.clone(),
                colour,
                wrap_width,
            );
            if at_rest {
                job.wrap = TextWrapping {
                    max_width: wrap_width,
                    max_rows: LINES.0,
                    overflow_character: Some('…'),
                    ..TextWrapping::default()
                };
            }
            ui.fonts_mut(|f| f.layout_job(job))
        };
        let return_key = self
            .lines
            .then(|| egui::KeyboardShortcut::new(Modifiers::SHIFT, Key::Enter));
        let edit = TextEdit::multiline(text)
            .id(self.id)
            .frame(Frame::NONE.inner_margin(Margin::symmetric(pad.0 as i8, pad.1 as i8)))
            .font(font.clone())
            .text_color(colour)
            .desired_width(inner.width())
            .desired_rows(1)
            .return_key(return_key)
            .layouter(&mut layouter);
        let response = child
            .add_enabled_ui(self.enabled, |ui| {
                ScrollArea::vertical()
                    .id_salt(self.id.with("scroll"))
                    .max_height(well.height())
                    .auto_shrink([false, false])
                    .show(ui, |ui| ui.add(edit))
                    .inner
            })
            .inner;
        if response.hovered() && self.enabled {
            ctx.set_cursor_icon(CursorIcon::Text);
        }
        // A click on the well's padding edits too.
        if self.enabled
            && !response.has_focus()
            && ui
                .interact(well, self.id.with("well"), Sense::CLICK)
                .clicked()
        {
            response.request_focus();
        }
        // Given the focus while not edited: editing begins, the caret where a click put it,
        // or, as Tab gives it the focus, the whole text selected.
        if editing.is_none() && response.has_focus() && self.enabled {
            let by_key = !ui.input(|i| i.pointer.any_pressed() || i.pointer.any_released());
            // What this frame put in it already, as a middle-click's paste, is the draft's;
            // Esc puts back the text as editing found it.
            editing = Some(Editing {
                original: self.text.to_owned(),
                typed: rest.clone(),
                by_key,
            });
            out.began = true;
            // Undo of typing reaches back to the draft's start alone, never an earlier one.
            let mut state = TextEdit::load_state(&ctx, self.id).unwrap_or_default();
            state.clear_undoer();
            state.store(&ctx, self.id);
            if by_key {
                select_all(&ctx, self.id, &rest);
            }
        }
        // Different, where a hint would sit, until the first character typed.
        if let Some((word, _)) = self.different.filter(|_| empty) {
            let colour = match self.enabled {
                true => t.colors.text_secondary,
                false => t.colors.text_disabled,
            };
            let galley = ui
                .painter()
                .layout_no_wrap(word.to_owned(), font.clone(), colour);
            let at = pos2(
                inner.left() + pad.0,
                inner.top() + (one - galley.size().y) / 2.0,
            );
            ui.painter().galley(at, galley, colour);
        }
        let name = self.label.to_owned();
        let lines = self.lines;
        let different = self.different.filter(|_| empty);
        ctx.accesskit_node_builder(self.id, |node| {
            node.set_label(name);
            if !lines {
                node.set_role(Role::TextInput);
            }
            if let Some((word, description)) = different {
                node.set_placeholder(word);
                node.set_description(description);
            }
        });
        let whole = (!self.text.is_empty()).then_some(self.text);
        let tip = [whole, self.tip]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("\n");
        if !tip.is_empty() && at_rest {
            let tip = tip.clone();
            response
                .clone()
                .on_hover_text(tip.clone())
                .on_disabled_hover_text(tip);
        }

        // No border while edited, as a value field: the ring for Tab's focus alone.
        if editing.as_ref().is_some_and(|editing| editing.by_key) {
            focus_ring(ui.painter(), well, &t);
        }
        if let Some((name, menu)) = warning {
            let at = (well.right() - warning_side, well.top());
            out.chosen = warning_button(ui, self.id, at, one, name, menu);
        }
        if let Some((name, menu)) = values {
            let at = (well.right() - one, well.top());
            let chosen = values_button(ui, self.id, at, one, (name, menu), list_asked);
            out.chosen = out.chosen.or(chosen);
        }

        // How editing ends.
        let lost = editing.is_some() && !focused && !out.began;
        let escaped = lost && ui.input(|i| i.key_pressed(Key::Escape));
        if let Some(ended) = editing.take_if(|_| commit || lost) {
            if escaped {
                out.cancelled = true;
            } else if ended.typed != ended.original {
                out.committed = Some(ended.typed);
            }
            if focused {
                ctx.memory_mut(|m| m.surrender_focus(self.id));
            }
        }
        out.editing = editing.is_some();
        ctx.data_mut(|d| match editing {
            Some(editing) => {
                d.insert_temp(key, editing);
            }
            None => d.remove::<Editing>(key),
        });
        let _ = response;
        out
    }

    /// The pull-down in the well, its value the option whose id is the field's text, else
    /// the first; the option chosen.
    fn pull_down(
        &self,
        ui: &mut Ui,
        well: Rect,
        options: &[(&str, &str)],
        font: &FontId,
    ) -> Option<String> {
        let t = theme(ui.ctx());
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let response = ui.interact(well, self.id, sense);
        // Different, where the photos hold it apart: no option is theirs.
        let different = self.different.filter(|_| self.text.is_empty());
        let current = match different {
            Some((word, _)) => word,
            None => (options.iter())
                .find(|(id, _)| *id == self.text)
                .or(options.first())
                .map_or("", |(_, words)| *words),
        };
        let colour = match (self.enabled, different.is_some()) {
            (false, _) => t.colors.text_disabled,
            (true, true) => t.colors.text_secondary,
            (true, false) => t.colors.text,
        };
        let painter = ui.painter();
        let chevron = t.sizes.icon;
        let room = well.width() - 2.0 * field_inset(&t) - chevron;
        let galley = painter.layout_job(truncated(current, font.clone(), room, colour));
        painter.galley(
            pos2(
                well.left() + field_inset(&t),
                well.center().y - galley.size().y / 2.0,
            ),
            galley,
            colour,
        );
        let at = Rect::from_center_size(
            pos2(
                well.right() - field_inset(&t) - chevron / 2.0,
                well.center().y,
            ),
            vec2(chevron, chevron),
        );
        Icon::ChevronDown.paint(painter, at, t.colors.text_secondary);
        if response.has_focus() {
            focus_ring(painter, well, &t);
        }
        let (label, enabled) = (self.label, self.enabled);
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(Role::ComboBox, enabled, label);
            info.current_text_value = Some(current.to_owned());
            info
        });
        if let Some((_, description)) = different {
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_description(description);
            });
        }
        if let Some(tip) = self.tip {
            response
                .clone()
                .on_hover_text(tip)
                .on_disabled_hover_text(tip);
        }
        let entries = || {
            (options.iter())
                .map(|(id, words)| {
                    let checked = different.is_none() && *id == self.text;
                    Entry::Item(Item::new(*id, *words).checked(checked))
                })
                .collect()
        };
        pull_down(ui, &response, self.label, entries)
    }
}

/// The warning button of the field `id`, its corner at `at` and its side `one`, named
/// `name`, opening `menu`; the item chosen in it.
fn warning_button(
    ui: &mut Ui,
    id: Id,
    at: (f32, f32),
    one: f32,
    name: &str,
    menu: Vec<Entry>,
) -> Option<String> {
    let t = theme(ui.ctx());
    let button = Rect::from_min_size(pos2(at.0, at.1), vec2(one, one));
    let response = ui.interact(button, id.with("warning"), Sense::click());
    let response = response.on_hover_text(name);
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, name));
    let side = vec2(t.sizes.icon, t.sizes.icon);
    let at = Rect::from_center_size(button.center(), side);
    Icon::Warning.paint(ui.painter(), at, t.colors.warning);
    pull_down(ui, &response, name, || menu)
}

/// The ⌄ of the field `id`, its corner at `at` and its side `one`, named `name`, opening
/// `menu`, the values list, as it does where `asked` by Alt+↓; the item chosen in it.
fn values_button(
    ui: &mut Ui,
    id: Id,
    at: (f32, f32),
    one: f32,
    (name, menu): (&str, Vec<Entry>),
    asked: bool,
) -> Option<String> {
    let t = theme(ui.ctx());
    let button = Rect::from_min_size(pos2(at.0, at.1), vec2(one, one));
    let response = ui.interact(button, id.with("values"), Sense::click());
    let hovered = response.hovered();
    let response = response.on_hover_text(name);
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, name));
    if hovered {
        let band = button.shrink(1.0);
        ui.painter()
            .rect_filled(band, t.shape.radius_small, t.colors.sunken_hover);
    }
    let side = vec2(t.sizes.icon, t.sizes.icon);
    let at = Rect::from_center_size(button.center(), side);
    Icon::ChevronDown.paint(ui.painter(), at, t.colors.text_secondary);
    let ctx = ui.ctx();
    let popup_id = popup::button_menu(response.id);
    let mut menu = Some(menu);
    if asked && !popup::is_open(ctx, popup_id) {
        let entries = menu.take().unwrap_or_default();
        let request = Request::pull_down(button, name, entries).opened_by(response.id);
        popup::open(ctx, popup_id, request);
    }
    pull_down(ui, &response, name, || menu.unwrap_or_default())
}

impl FieldWell<'_> {
    /// Ends the editing of the field `id` with no press to end it, as the photos it was
    /// opened on leave the tool, and takes its focus away: what was typed, where it changed
    /// since editing began, for the caller to commit to those photos.
    pub fn take(ctx: &egui::Context, id: Id) -> Option<String> {
        let key = id.with("editing");
        let editing: Option<Editing> = ctx.data_mut(|d| {
            let editing = d.get_temp(key);
            d.remove::<Editing>(key);
            editing
        });
        if ctx.memory(|m| m.has_focus(id)) {
            ctx.memory_mut(|m| m.surrender_focus(id));
        }
        editing
            .filter(|editing| editing.typed != editing.original)
            .map(|editing| editing.typed)
    }

    /// Forgets what the field `id` was editing and takes its focus away, committing
    /// nothing: the photo it was opened on has gone.
    pub fn forget(ctx: &egui::Context, id: Id) {
        ctx.data_mut(|d| d.remove::<Editing>(id.with("editing")));
        // Its caret and its undo of typing too, so nothing typed there comes back.
        ctx.data_mut(|d| d.remove::<egui::text_edit::TextEditState>(id));
        if ctx.memory(|m| m.has_focus(id)) {
            ctx.memory_mut(|m| m.surrender_focus(id));
        }
    }
}

/// The whole of `text` selected in the field `id`, as Tab's focus selects it.
fn select_all(ctx: &egui::Context, id: Id, text: &str) {
    let mut state = TextEdit::load_state(ctx, id).unwrap_or_default();
    let end = CCursor::new(CharIndex(text.chars().count()));
    state
        .cursor
        .set_char_range(Some(CCursorRange::two(CCursor::new(CharIndex(0)), end)));
    state.store(ctx, id);
}

/// A line break put in `text` where the field `id`'s caret is, over its selection.
fn insert_break(ctx: &egui::Context, id: Id, text: &mut String) {
    let mut state = TextEdit::load_state(ctx, id).unwrap_or_default();
    let count = text.chars().count();
    let (start, end) = state.cursor.char_range().map_or((count, count), |range| {
        let [a, b] = range.sorted_cursors();
        (a.index.0.min(count), b.index.0.min(count))
    });
    let byte = |at: usize| text.char_indices().nth(at).map_or(text.len(), |(i, _)| i);
    let (from, to) = (byte(start), byte(end));
    text.replace_range(from..to, "\n");
    let caret = CCursor::new(CharIndex(start + 1));
    state.cursor.set_char_range(Some(CCursorRange::one(caret)));
    state.store(ctx, id);
}
