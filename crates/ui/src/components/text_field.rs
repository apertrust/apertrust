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

//! A field for typing text, and the Export dialog's path field built on it (`docs/gui.md`,
//! The Export dialog, States).
//!
//! - **`TextField`**: one line in a sunken well, a field high, in the small text, filling
//!   the row or `width` points, as a value field is drawn but left-aligned. Every edit is
//!   reported with the whole text, so the caller's preview follows each letter. A hint
//!   shows while it is empty and an error outlines it in `error`. Screen readers hear it
//!   as a text input named by the control's full name.
//! - **`PathField`**: a text field that offers the variables a path may hold. While the
//!   caret stands just after `$(` and the letters of a name, a list under the field shows
//!   the variables that name begins, each with its description. A click inserts one, with
//!   its closing `)`; ↓ and ↑ move a highlight in the list and Enter inserts the
//!   highlighted one; Escape closes the list and nothing else, so the dialog stays
//!   ([`PathOutput::list_open`] says it was open as Escape came). The list is part of the
//!   row, not a popup, so the field keeps the keyboard focus while it is typed to.

use egui::text::{CCursor, CCursorRange, CharIndex};
use egui::{
    FontId, Frame, Id, Key, Margin, Modifiers, Rect, Response, Role, Sense, Stroke, StrokeKind,
    TextEdit, Ui, UiBuilder, WidgetInfo, vec2,
};

use super::{field_inset, focus_ring, truncated};
use crate::{Theme, theme, translator};

/// The most variables the path field's list shows at once.
const LISTED: usize = 7;

/// One line of text.
pub struct TextField<'a> {
    id: Id,
    name: &'a str,
    text: &'a str,
    width: Option<f32>,
    hint: Option<&'a str>,
    error: bool,
    enabled: bool,
}

/// What a frame of the field did.
pub(crate) struct Drawn {
    pub response: Response,
    /// The whole text, once the user changed it.
    pub text: Option<String>,
    /// The caret, as a character index into the text as it stands now, while the field has
    /// the focus.
    pub caret: Option<usize>,
}

impl<'a> TextField<'a> {
    /// `id` must be stable across frames; `name` is the control's full name, translated.
    pub fn new(id: Id, name: &'a str, text: &'a str) -> Self {
        TextField {
            id,
            name,
            text,
            width: None,
            hint: None,
            error: false,
            enabled: true,
        }
    }

    /// The field's width in points; it fills the row unless given.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// What it shows, translated, while it is empty.
    pub fn hint(mut self, text: &'a str) -> Self {
        self.hint = Some(text);
        self
    }

    /// Outlines the field in the error colour.
    pub fn error(mut self, on: bool) -> Self {
        self.error = on;
        self
    }

    /// A disabled field is dimmed and ignores input.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws the field; returns its response and, when the user changed the text, all of it.
    pub fn show(self, ui: &mut Ui) -> (Response, Option<String>) {
        let drawn = self.draw(ui, None);
        (drawn.response, drawn.text)
    }

    /// Draws the field, with the caret `put` where the caller moved it this frame.
    pub(crate) fn draw(self, ui: &mut Ui, put: Option<usize>) -> Drawn {
        let t = theme(ui.ctx());
        // A caret the list put on the last frame, after the variable it inserted.
        let put = put.or_else(|| ui.data_mut(|d| d.remove_temp::<usize>(self.id.with("put"))));
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let (rect, _) = ui.allocate_exact_size(vec2(width, t.sizes.field), Sense::hover());
        let colour = if self.enabled {
            t.colors.text
        } else {
            t.colors.text_disabled
        };
        ui.painter()
            .rect_filled(rect, t.shape.radius_small, t.colors.sunken);
        let mut text = self.text.to_owned();
        let padding = Margin {
            left: field_inset(&t) as i8,
            right: field_inset(&t) as i8,
            top: 0,
            bottom: 0,
        };
        let mut edit = TextEdit::singleline(&mut text)
            .id(self.id)
            .frame(Frame::NONE.inner_margin(padding))
            .font(FontId::proportional(t.text.small))
            .text_color(colour)
            .desired_width(rect.width());
        if let Some(hint) = self.hint {
            edit = edit.hint_text(
                egui::RichText::new(hint)
                    .size(t.text.small)
                    .color(t.colors.text_secondary),
            );
        }
        // egui makes a text field a control high at least; the cursor and selection are
        // neutral, as a value field's.
        let mut field = ui.new_child(UiBuilder::new().max_rect(rect));
        field.spacing_mut().interact_size.y = t.sizes.field;
        let visuals = field.visuals_mut();
        visuals.selection.bg_fill = t.colors.field_selection;
        visuals.text_cursor.stroke.color = t.colors.text;
        field.add_enabled_ui(self.enabled, |ui| {
            ui.put(rect, |ui: &mut Ui| {
                if let Some(at) = put {
                    let mut state = TextEdit::load_state(ui.ctx(), self.id).unwrap_or_default();
                    state
                        .cursor
                        .set_char_range(Some(CCursorRange::one(CCursor::new(CharIndex(at)))));
                    state.store(ui.ctx(), self.id);
                }
                let output = edit.show(ui);
                ui.data_mut(|d| d.insert_temp(self.id.with("caret"), output.cursor_range));
                output.response.response
            })
        });
        let response = ui
            .ctx()
            .read_response(self.id)
            .unwrap_or_else(|| ui.interact(rect, self.id, Sense::click()));
        let caret = ui
            .data(|d| d.get_temp::<Option<CCursorRange>>(self.id.with("caret")))
            .flatten()
            .filter(|_| response.has_focus())
            .map(|range| range.primary.index.0);
        if self.error {
            ui.painter().rect_stroke(
                rect,
                t.shape.radius_small,
                Stroke::new(1.5, t.colors.error),
                StrokeKind::Inside,
            );
        }
        if response.has_focus() {
            focus_ring(ui.painter(), rect, &t);
        }
        let name = self.name.to_owned();
        ui.ctx().accesskit_node_builder(self.id, |node| {
            node.set_label(name);
        });
        let changed = (text != self.text).then_some(text);
        Drawn {
            response,
            text: changed,
            caret,
        }
    }
}

/// A variable the path field offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Variable<'a> {
    /// What the list inserts after `$(`, before the closing `)`: the name with its
    /// parameters' brackets, such as `SEQUENCE[4,1]`.
    pub insert: &'a str,
    /// Its description, translated or as the source gives it.
    pub description: &'a str,
}

impl Variable<'_> {
    /// The name without its brackets, which the list is filtered by.
    fn name(&self) -> &str {
        self.insert.split_once('[').map_or(self.insert, |(n, _)| n)
    }
}

/// What a frame of the path field did.
pub struct PathOutput {
    pub response: Response,
    /// The whole path, once the user changed it, by typing or by choosing a variable.
    pub text: Option<String>,
    /// Whether the list was open as this frame began, for the caller to know that Escape
    /// closed it and nothing else.
    pub list_open: bool,
}

/// A text field for a path, offering the variables it may hold.
pub struct PathField<'a> {
    field: TextField<'a>,
    variables: &'a [Variable<'a>],
}

impl<'a> PathField<'a> {
    pub fn new(id: Id, name: &'a str, text: &'a str, variables: &'a [Variable<'a>]) -> Self {
        PathField {
            field: TextField::new(id, name, text),
            variables,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        self.field = self.field.width(width);
        self
    }

    pub fn hint(mut self, text: &'a str) -> Self {
        self.field = self.field.hint(text);
        self
    }

    pub fn error(mut self, on: bool) -> Self {
        self.field = self.field.error(on);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.field = self.field.enabled(enabled);
        self
    }

    pub fn show(self, ui: &mut Ui) -> PathOutput {
        let t = theme(ui.ctx());
        let id = self.field.id;
        let text = self.field.text.to_owned();
        let list_key = id.with("list");
        let last: Last = ui.data(|d| d.get_temp(list_key)).unwrap_or_default();
        let had_focus = ui.memory(|m| m.has_focus(id) || m.had_focus_last_frame(id));
        let list_open = last.open && had_focus;
        let mut highlight = last.highlight;
        let mut chosen = None;
        // The list's keys are the list's, before the field sees them.
        if list_open && !last.matches.is_empty() {
            ui.input_mut(|i| {
                if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                    highlight = Some(highlight.map_or(0, |h| (h + 1).min(last.matches.len() - 1)));
                }
                if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                    highlight = highlight.and_then(|h| h.checked_sub(1));
                }
                if let Some(h) = highlight
                    && i.consume_key(Modifiers::NONE, Key::Enter)
                {
                    chosen = last.matches.get(h).copied();
                }
            });
        }
        let mut hidden = last.hidden;
        if list_open && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            // egui let go of the focus as the frame began; the field keeps it, and the list
            // stays closed until the text or the caret moves.
            ui.memory_mut(|m| m.request_focus(id));
            hidden = Some((text.clone(), last.caret));
        }
        let mut put = None;
        let mut edited = None;
        if let Some(variable) = chosen {
            let (new, caret) = insert(
                &text,
                last.caret,
                last.partial.clone(),
                variable,
                self.variables,
            );
            edited = Some(new);
            put = Some(caret);
            highlight = None;
        }
        let shown = edited.clone().unwrap_or_else(|| text.clone());
        let field = TextField {
            text: &shown,
            ..self.field
        };
        let drawn = field.draw(ui, put);
        let mut typed = drawn.text;
        if typed.is_some() {
            highlight = None;
        }
        let current = typed.clone().unwrap_or_else(|| shown.clone());
        // What is being typed after "$(", at the caret.
        let caret = drawn
            .caret
            .or(put)
            .or_else(|| had_focus.then_some(last.caret).flatten());
        let partial = caret.and_then(|c| partial_name(&current, c));
        let focused = drawn.response.has_focus();
        let pointer_in_list = last.rect.is_some_and(|rect| {
            ui.input(|i| {
                (i.pointer.any_down() || i.pointer.any_released())
                    && i.pointer.interact_pos().is_some_and(|p| rect.contains(p))
            })
        });
        let matches: Vec<usize> = partial
            .as_deref()
            .filter(|_| focused || pointer_in_list || had_focus)
            .filter(|_| hidden.as_ref() != Some(&(current.clone(), caret)))
            .map(|p| {
                let wanted = p.to_uppercase();
                self.variables
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| v.name().starts_with(&wanted))
                    .map(|(i, _)| i)
                    .take(LISTED)
                    .collect()
            })
            .unwrap_or_default();
        let mut rect = None;
        if !matches.is_empty() {
            let (r, pick) = list(ui, &t, self.variables, &matches, highlight);
            rect = Some(r);
            if let (Some(variable), None) = (pick, &edited) {
                let (new, at) = insert(
                    &current,
                    caret,
                    partial.clone().unwrap_or_default(),
                    variable,
                    self.variables,
                );
                typed = Some(new);
                ui.memory_mut(|m| m.request_focus(id));
                // The caret of the next frame, after the closing parenthesis.
                ui.data_mut(|d| d.insert_temp(id.with("put"), at));
                highlight = None;
            }
        }
        let open = !matches.is_empty();
        // Moving on shows the list again.
        let hidden = hidden.filter(|h| *h == (current.clone(), caret));
        let highlight = highlight.filter(|h| *h < matches.len());
        ui.data_mut(|d| {
            d.insert_temp(
                list_key,
                Last {
                    open,
                    highlight,
                    matches: matches.clone(),
                    caret,
                    partial: partial.unwrap_or_default(),
                    rect,
                    hidden,
                },
            );
        });
        PathOutput {
            response: drawn.response,
            text: typed.or(edited),
            list_open,
        }
    }
}

/// What the list was like at the end of the last frame.
#[derive(Clone, Default)]
struct Last {
    open: bool,
    highlight: Option<usize>,
    /// The variables listed, by index among those offered.
    matches: Vec<usize>,
    caret: Option<usize>,
    /// The letters typed after `$(`.
    partial: String,
    rect: Option<Rect>,
    /// The text and caret the list was closed at by Escape.
    hidden: Option<(String, Option<usize>)>,
}

/// The letters of a variable's name just before character `caret`, if they follow `$(`.
fn partial_name(text: &str, caret: usize) -> Option<String> {
    let before: String = text.chars().take(caret).collect();
    let start = before.rfind("$(")?;
    let name = &before[start + 2..];
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        .then(|| name.to_owned())
}

/// `text` with the name being typed before character `caret`, `partial`, replaced by the
/// variable `which` and its closing parenthesis (not doubled where one follows), and the
/// caret after it.
fn insert(
    text: &str,
    caret: Option<usize>,
    partial: String,
    which: usize,
    variables: &[Variable],
) -> (String, usize) {
    let chars: Vec<char> = text.chars().collect();
    let caret = caret.unwrap_or(chars.len()).min(chars.len());
    let start = caret - partial.chars().count().min(caret);
    let insert = variables.get(which).map_or("", |v| v.insert);
    let mut new: String = chars[..start].iter().collect();
    new.push_str(insert);
    let mut at = new.chars().count();
    // A name already closed keeps its parenthesis.
    let rest = &chars[caret..];
    if rest.first() == Some(&')') {
        at += 1;
    } else {
        new.push(')');
        at += 1;
    }
    new.extend(rest);
    (new, at)
}

/// Draws the list under the field: a row each, the name and its description. Returns its
/// rectangle and the variable the user clicked, by index among those offered.
fn list(
    ui: &mut Ui,
    t: &Theme,
    variables: &[Variable],
    matches: &[usize],
    highlight: Option<usize>,
) -> (Rect, Option<usize>) {
    let name = translator(ui.ctx()).text("path-variables");
    let mut pick = None;
    let group = Frame::NONE
        .fill(t.colors.raised)
        .stroke(Stroke::new(t.shape.hairline, t.colors.border))
        .corner_radius(t.shape.radius_small)
        .inner_margin(Margin::same(t.spacing.xxs as i8))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (row_index, &which) in matches.iter().enumerate() {
                let variable = variables[which];
                let (row, _) =
                    ui.allocate_exact_size(vec2(ui.available_width(), t.sizes.row), Sense::hover());
                // Clicks only: the list is chosen from with the keys the field takes.
                let id = ui.scope_id().with(("variable", which));
                let response = ui.interact(row, id, Sense::CLICK);
                if highlight == Some(row_index) {
                    ui.painter()
                        .rect_filled(row, t.shape.radius_small, t.colors.accent_subtle);
                } else if response.hovered() {
                    ui.painter()
                        .rect_filled(row, t.shape.radius_small, t.colors.hover);
                }
                let font = FontId::proportional(t.text.small);
                let name_width = (row.width() * 0.4).max(80.0);
                let inset = field_inset(t);
                let name_galley = ui.painter().layout_job(truncated(
                    variable.insert,
                    font.clone(),
                    name_width - inset,
                    t.colors.text,
                ));
                let at = row.left_center() + vec2(inset, -name_galley.size().y / 2.0);
                ui.painter().galley(at, name_galley, t.colors.text);
                let description = ui.painter().layout_job(truncated(
                    variable.description,
                    font,
                    row.width() - name_width - 2.0 * inset,
                    t.colors.text_secondary,
                ));
                let at = egui::pos2(
                    row.left() + name_width,
                    row.center().y - description.size().y / 2.0,
                );
                ui.painter()
                    .galley(at, description, t.colors.text_secondary);
                response.widget_info(|| {
                    WidgetInfo::labeled(Role::ListBoxOption, true, variable.insert)
                });
                if response.clicked() {
                    pick = Some(which);
                }
            }
        });
    let rect = group.response.rect;
    group
        .response
        .widget_info(|| WidgetInfo::labeled(Role::ListBox, true, &name));
    (rect, pick)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VARIABLES: [Variable; 3] = [
        Variable {
            insert: "FILE.FOLDER",
            description: "folder containing the input image",
        },
        Variable {
            insert: "FILE.NAME",
            description: "basename of the input image",
        },
        Variable {
            insert: "SEQUENCE[4,1]",
            description: "sequence number",
        },
    ];

    #[test]
    fn the_letters_typed_after_a_dollar_and_parenthesis_are_the_name_being_typed() {
        assert_eq!(partial_name("$(", 2).as_deref(), Some(""));
        assert_eq!(partial_name("a/$(FILE.", 9).as_deref(), Some("FILE."));
        // The caret decides, not the end of the text.
        assert_eq!(partial_name("$(FI)/x", 4).as_deref(), Some("FI"));
        // Not after a closed variable, nor with no "$(".
        assert_eq!(partial_name("$(FILE.NAME)/", 13), None);
        assert_eq!(partial_name("folder/FI", 9), None);
        // Characters count, not bytes.
        assert_eq!(partial_name("ф/$(FIL", 7).as_deref(), Some("FIL"));
    }

    #[test]
    fn choosing_a_variable_replaces_the_name_typed_and_closes_it() {
        let (new, at) = insert("a/$(FI", Some(6), "FI".to_owned(), 1, &VARIABLES);
        assert_eq!(new, "a/$(FILE.NAME)");
        assert_eq!(at, new.chars().count());
        // Its brackets are kept, for the person to fill in.
        let (new, _) = insert("$(SE", Some(4), "SE".to_owned(), 2, &VARIABLES);
        assert_eq!(new, "$(SEQUENCE[4,1])");
        // The rest of the path stays, and a parenthesis that is there is not doubled.
        let (new, at) = insert("$(FI)/x", Some(4), "FI".to_owned(), 0, &VARIABLES);
        assert_eq!(new, "$(FILE.FOLDER)/x");
        assert_eq!(at, 14);
        let (new, at) = insert("$(F/x", Some(3), "F".to_owned(), 0, &VARIABLES);
        assert_eq!(new, "$(FILE.FOLDER)/x");
        assert_eq!(at, 14);
    }

    #[test]
    fn a_variable_is_filtered_by_its_name_without_brackets() {
        assert_eq!(VARIABLES[2].name(), "SEQUENCE");
        assert_eq!(VARIABLES[0].name(), "FILE.FOLDER");
    }
}
