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

//! A field for typing an exact value. It shows the value in the language's number format
//! and no unit; typing accepts the language's decimal separator or the point, and a unit.
//! Enter or leaving the field commits; Escape cancels. Focus arriving from the keyboard, or
//! from a slider's value, selects the number, so what is typed replaces it.
//!
//! A slider's field takes a number only (`number_only`), for now (decision 42): digits, a
//! minus at its start, `-` or `−`, and one decimal separator, the point or the language's
//! own. Any other character is ignored as it is typed, and a character the number refuses
//! where it lands, a second separator or a minus past the start, is too. A paste, or a
//! value a screen reader sets, is taken whole or not at all: its text, less the spaces
//! round it, only where the field takes every character of it, so "1e-5" is never 15. The
//! field never holds text that is no number; with no digit in it, or a number past an f64's
//! range, it sets nothing.
//!
//! It is a field high, in the small text, right-aligned 5 points from its end, typing
//! included, as the wireframe draws it. It is `sizes.value_column` wide, or as wide as its
//! number with 5 points at each end needs (`width`), so a number is never cut (decision
//! 42). Nothing in it turns the accent: the text stays in the text colour, the selection is
//! `field_selection` and the cursor the text colour.

use std::ops::{Range, RangeInclusive};

use egui::text::{CCursor, CCursorRange, CharIndex};
use egui::widgets::text_edit::TextEditState;
use egui::{
    Event, FontId, Frame, Id, ImeEvent, Key, Margin, Response, TextBuffer, TextEdit, Ui, UiBuilder,
    vec2,
};

use super::field_inset;
use crate::{Translator, theme, translator};

pub struct ValueField<'a> {
    id: Id,
    name: &'a str,
    unit: Option<&'a str>,
    /// Words drawn after the number at the field's end, its unit where a row shows none.
    suffix: Option<&'a str>,
    value: f64,
    decimals: usize,
    number_only: bool,
    range: RangeInclusive<f64>,
    step: f64,
}

impl<'a> ValueField<'a> {
    /// `id` must be stable across frames; `name` is the control's full name.
    pub fn new(id: Id, name: &'a str, value: f64, range: RangeInclusive<f64>) -> Self {
        ValueField {
            id,
            name,
            unit: None,
            suffix: None,
            value,
            decimals: 0,
            number_only: false,
            range,
            step: 0.0,
        }
    }

    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// The unit the number is in, translated, as `unit-ev` writes it: the field holds the
    /// number alone, so screen readers hear the unit in its name, "Exposure, in EV".
    pub fn unit(mut self, unit: &'a str) -> Self {
        self.unit = Some(unit);
        self
    }

    /// Draws `text`, translated, after the number at the field's end in `text_secondary`,
    /// as the Settings dialog draws a number's unit ("100 %"), so fields of one width end
    /// together. It is not part of what is typed. A field without one is drawn as before.
    pub fn suffix(mut self, text: &'a str) -> Self {
        self.suffix = Some(text);
        self
    }

    /// Takes a number only, as a slider's field does: digits, a minus at its start and one
    /// decimal separator, the point or the language's own; a paste taken whole or refused.
    pub fn number_only(mut self) -> Self {
        self.number_only = true;
        self
    }

    /// A typed value snaps to multiples of `step`.
    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// How wide the field is: `sizes.value_column`, or as wide as its number with 5
    /// points at each end needs, so the whole number is in view as typing starts.
    pub fn width(&self, ui: &Ui) -> f32 {
        let t = theme(ui.ctx());
        let number = self.written(&translator(ui.ctx()));
        let galley =
            ui.painter()
                .layout_no_wrap(number, FontId::proportional(t.text.small), t.colors.text);
        (galley.size().x + 2.0 * field_inset(&t) + self.suffix_room(ui))
            .ceil()
            .max(t.sizes.value_column)
    }

    /// The suffix's room at the field's end, with the gap before it; none without one.
    fn suffix_room(&self, ui: &Ui) -> f32 {
        let t = theme(ui.ctx());
        self.suffix.map_or(0.0, |suffix| {
            let font = FontId::proportional(t.text.small);
            let galley =
                ui.painter()
                    .layout_no_wrap(suffix.to_owned(), font, t.colors.text_secondary);
            galley.size().x + t.spacing.xs
        })
    }

    /// Draws the field filling `ui`'s width; returns its response and, when the user
    /// commits a new number, the value, snapped and within the range.
    pub fn show(self, ui: &mut Ui) -> (Response, Option<f64>) {
        let t = theme(ui.ctx());
        let tr = translator(ui.ctx());
        let draft_id = self.id.with("draft");
        let cancel_id = self.id.with("cancelled");
        let focused = ui.memory(|m| m.has_focus(self.id));
        // What was typed belongs to the focus that typed it. A field that lost the focus
        // while it was not drawn, its row hidden as a tab switched, never saw it go: its
        // draft goes now, so it never comes back. One losing it this frame or the last
        // still sets it.
        let losing = ui
            .ctx()
            .read_response(self.id)
            .is_some_and(|field| field.lost_focus());
        if !focused && !losing {
            ValueField::forget(ui.ctx(), self.id);
        }
        // Escape while typing cancels: the draft goes, and leaving the field commits
        // nothing, whichever frame egui reports the lost focus in.
        // egui drops the focus on Escape as the frame begins, so ask about the last frame.
        if ui.memory(|m| m.had_focus_last_frame(self.id))
            && ui.input(|i| i.key_pressed(Key::Escape))
        {
            ui.data_mut(|d| {
                d.remove::<String>(draft_id);
                d.insert_temp(cancel_id, true);
            });
        }
        let whole = self.written(&tr);
        let separator = tr.decimal_separator();
        // A character no number has is dropped from what is typed or composed before egui's
        // field sees it, and a paste that is no number goes whole, so a key or a paste it
        // ignores leaves a selection as it was; where the rest may go, the text decides
        // (`Number`). A paste no empty field takes, none takes.
        if self.number_only && focused {
            ui.input_mut(|i| {
                i.events.retain_mut(|event| match event {
                    Event::Text(text) => {
                        text.retain(|c| number_char(c, separator));
                        !text.is_empty()
                    }
                    Event::Paste(text) => {
                        *text = text.trim().to_owned();
                        takes("", text, "", separator)
                    }
                    // Left empty, egui ignores them while nothing is being composed.
                    Event::Ime(ImeEvent::Commit(text)) => {
                        text.retain(|c| number_char(c, separator));
                        true
                    }
                    Event::Ime(ImeEvent::Preedit {
                        text,
                        active_range_chars,
                    }) => {
                        let whole = text.chars().count();
                        text.retain(|c| number_char(c, separator));
                        if text.chars().count() != whole {
                            *active_range_chars = None;
                        }
                        true
                    }
                    _ => true,
                });
            });
        }
        let draft = ui.data(|d| d.get_temp::<String>(draft_id));
        // Focus arriving with nothing typed yet, by Tab or from a slider's value, selects
        // the number, so a character typed in the same frame replaces it; a click places
        // the caret instead, since egui's field takes the click after this.
        if focused && draft.is_none() {
            select_all(ui, self.id, &whole);
        }
        let editing = draft.is_some() || focused;
        let (rect, _) = ui.allocate_exact_size(
            vec2(ui.available_width(), t.sizes.field),
            egui::Sense::hover(),
        );
        let font = FontId::proportional(t.text.small);
        let mut text = draft.unwrap_or_else(|| whole.clone());
        ui.painter()
            .rect_filled(rect, t.shape.radius_small, t.colors.sunken);
        // The padding is the frame's: egui drops `TextEdit::margin` once the caller gives a
        // frame, and its own frame would draw a stroke while focused.
        let suffix = self.suffix_room(ui);
        let padding = Margin {
            left: t.spacing.xxs as i8,
            right: (field_inset(&t) + suffix).round() as i8,
            top: 0,
            bottom: 0,
        };
        if let Some(words) = self.suffix {
            let galley = ui.painter().layout_no_wrap(
                words.to_owned(),
                FontId::proportional(t.text.small),
                t.colors.text_secondary,
            );
            let at = egui::pos2(
                rect.right() - field_inset(&t) - galley.size().x,
                rect.center().y - galley.size().y / 2.0,
            );
            ui.painter().galley(at, galley, t.colors.text_secondary);
        }
        let mut number;
        let buffer: &mut dyn TextBuffer = if self.number_only {
            number = Number {
                text: &mut text,
                separator,
                deleted: None,
            };
            &mut number
        } else {
            &mut text
        };
        let edit = TextEdit::singleline(buffer)
            .id(self.id)
            .frame(Frame::NONE.inner_margin(padding))
            .align(egui::Align2::RIGHT_CENTER)
            .font(font)
            .text_color(t.colors.text)
            .desired_width(rect.width());
        // egui makes a text field a control high at least, which would take the slider's
        // strip under it from the pointer; and its selection and cursor are the accent's
        // elsewhere, while here they stay neutral.
        let mut field = ui.new_child(UiBuilder::new().max_rect(rect));
        field.spacing_mut().interact_size.y = t.sizes.field;
        let visuals = field.visuals_mut();
        visuals.selection.bg_fill = t.colors.field_selection;
        visuals.text_cursor.stroke.color = t.colors.text;
        let response = field.put(rect, edit);
        // A double-click selects the whole number, not the part of it between its signs.
        if response.double_clicked() {
            select_all(ui, self.id, &text);
            ui.ctx().request_repaint();
        }
        let mut committed = None;
        if response.lost_focus() {
            let cancelled = ui.data(|d| d.get_temp::<bool>(cancel_id)).unwrap_or(false);
            // Leaving the number as it was shown commits nothing, and nor does text that is
            // no number: empty, or a sign or a separator alone, which puts the value back.
            let unchanged = text.trim() == whole;
            if !cancelled
                && !unchanged
                && let Some(typed) = tr.parse_number(&text)
            {
                let value = self.settle(typed);
                // Nor does a number that settles where the value is.
                if value != self.value {
                    committed = Some(value);
                }
            }
            ui.data_mut(|d| {
                d.remove::<String>(draft_id);
                d.remove::<bool>(cancel_id);
            });
        } else if response.has_focus() {
            ui.data_mut(|d| d.insert_temp(draft_id, text));
        }
        let mut args = crate::i18n::Args::new();
        args.set("control", self.name);
        let label = match self.unit {
            Some(unit) => {
                args.set("unit", unit);
                tr.text_with("field-value-in", Some(&args))
            }
            None => tr.text_with("field-value", Some(&args)),
        };
        let value = if editing { None } else { Some(whole) };
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_label(label);
            if let Some(value) = value {
                node.set_value(value);
            }
        });
        (response, committed)
    }

    /// Forgets what was typed in the field `id`, and whether it was put back: a field
    /// opening afresh, or closing unseen, as a slider's does.
    pub(crate) fn forget(ctx: &egui::Context, id: Id) {
        ctx.data_mut(|d| {
            d.remove::<String>(id.with("draft"));
            d.remove::<bool>(id.with("cancelled"));
        });
    }

    /// The value as the field holds it before anything is typed: the number alone, with no
    /// `+`, which a field that takes a number only does not take.
    fn written(&self, tr: &Translator) -> String {
        tr.format_number(self.value, self.decimals, false)
    }

    /// Snapped, within the range, and never −0, which "-0" and a small negative snapped to
    /// zero would otherwise send.
    fn settle(&self, value: f64) -> f64 {
        let snapped = if self.step > 0.0 {
            (value / self.step).round() * self.step
        } else {
            value
        };
        snapped.clamp(*self.range.start(), *self.range.end()) + 0.0
    }
}

/// Whether `c` can be part of a number a field that takes a number only holds: a digit, a
/// minus, the point or the language's `separator`. Where it may go is `takes`'s.
pub(super) fn number_char(c: char, separator: char) -> bool {
    c.is_ascii_digit() || is_minus(c) || c == '.' || c == separator
}

fn is_minus(c: char) -> bool {
    matches!(c, '-' | '−')
}

/// Whether a number takes `typed`, whole, where it lands between `before` and `after`:
/// digits, a minus only at the number's start, and one decimal separator, the point or the
/// language's `separator`; nothing before a minus. A key is one character, so this is also
/// what typing takes.
fn takes(before: &str, typed: &str, after: &str, separator: char) -> bool {
    let is_separator = |c: char| c == '.' || c == separator;
    if typed.is_empty() || after.starts_with(is_minus) {
        return false;
    }
    let mut point = before.contains(is_separator) || after.contains(is_separator);
    typed.chars().enumerate().all(|(i, c)| {
        if is_minus(c) {
            before.is_empty() && i == 0
        } else if is_separator(c) {
            let first = !point;
            point = true;
            first
        } else {
            c.is_ascii_digit()
        }
    })
}

/// A field's text while it takes a number only: whatever egui's field inserts, typed,
/// pasted, composed or set by a screen reader, goes in whole, less the spaces round it,
/// where `takes` takes it, or not at all, so the text is always a number or on its way to
/// one.
struct Number<'a> {
    text: &'a mut String,
    separator: char,
    /// What the last deletion took, and from where. egui deletes a selection before it
    /// inserts what replaces it, so an insertion refused puts the selection's text back,
    /// the caret at its start: a key or a paste refused leaves the text as it was.
    deleted: Option<(CharIndex, String)>,
}

impl Number<'_> {
    fn byte(&self, at: CharIndex) -> usize {
        self.byte_index_from_char_index(at).0
    }
}

impl TextBuffer for Number<'_> {
    fn is_mutable(&self) -> bool {
        true
    }

    fn as_str(&self) -> &str {
        self.text.as_str()
    }

    fn insert_text(&mut self, text: &str, at: CharIndex) -> usize {
        let byte = self.byte(at);
        let (before, after) = self.text.split_at(byte);
        let number = text.trim();
        let taken = takes(before, number, after, self.separator);
        let deleted = self.deleted.take();
        if !taken {
            // Nothing given is no refusal: a screen reader setting the value to nothing
            // clears the field.
            if let Some((from, deleted)) = deleted
                && from == at
                && !text.is_empty()
            {
                self.text.insert_str(byte, &deleted);
            }
            return 0;
        }
        self.text.insert_str(byte, number);
        number.chars().count()
    }

    fn delete_char_range(&mut self, range: Range<CharIndex>) {
        let bytes = self.byte(range.start)..self.byte(range.end);
        self.deleted = Some((range.start, self.text.drain(bytes).collect()));
    }

    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Number<'static>>()
    }
}

/// Selects all of `text` in the field `id`, from the next time egui's field reads its
/// state.
fn select_all(ui: &Ui, id: Id, text: &str) {
    let mut state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
    let end = CCursor::new(CharIndex(text.chars().count()));
    state
        .cursor
        .set_char_range(Some(CCursorRange::two(CCursor::default(), end)));
    state.store(ui.ctx(), id);
}
