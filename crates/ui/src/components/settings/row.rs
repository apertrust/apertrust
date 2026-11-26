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

//! A row of the Settings dialog (`docs/gui.md`, The Settings dialog): one line,
//! `sizes.setting_row` high (32), with no band under the pointer: the controls are the
//! targets.
//!
//! - **The label** at the start, in `text`, its help the row's tooltip. A label too long
//!   for its room wraps; a row that grows, as chips or a text area make it, keeps its label
//!   on its first line. A row that names a thing drawn by a mark elsewhere, a rating's star,
//!   the reject or a colour label, draws that mark before its label ([`RowGlyph`]), an
//!   icon `sizes.icon` square in the label's colour or the label's dot.
//! - **The control** in one column at the end, `sizes.setting_column` wide (260), a box
//!   centred on the first line: the caller draws it in the `Ui` it is handed. A read-only
//!   row's value may take the room its label leaves too ([`SettingRow::read_only`]).
//! - **The changed mark** after the label, where a tool's kept marker sits: ● in
//!   `kept_marker` while the value is off its default, nothing at it. It is a button whose
//!   click resets the value, its tooltip and its name the default's ("Reset to Dark").
//!   A double-click on the label resets too, as darktable's does, and so do Delete on the
//!   focused label and Reset in the label's right-click menu.
//! - **After a restart:** "· after a restart" follows the label, or its mark, in
//!   `text_secondary`.
//! - **Disabled** (set by `--conf`, or a `requires` that is off): the row dimmed, the
//!   tooltip over its label and over its control, and its label's description, saying why,
//!   and no mark.
//! - **A match** of Find a setting: the label underlined ([`SettingRow::matched`]).
//! - **A note** under the control, from the column's start to its end, in the caption size:
//!   what the row's owner says of it, in `warning` where it warns.
//!
//! A switch's row is its target whole ([`SettingRow::whole_row`]), as tall as its label
//! where it wraps: each click anywhere on it but its control is reported at once, for the
//! caller to toggle the switch, as darktable's label toggles on every press, and so is a
//! key's or a screen reader's; the second click of a double-click is not, resetting the
//! row instead, so a double-click leaves it at its default, as darktable's does, and a
//! third toggles again. Two primary clicks are a double-click on this label only where the
//! first was on it too, within egui's double-click delay of each other, another button's
//! click between them aside. To screen readers the label is a label, described as changed
//! from its default while it is, the mark a button.

use std::hash::Hash;

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Align, FontId, Id, Key, Layout, PointerButton, Rect, Role, Sense, Ui, UiBuilder, WidgetInfo,
    pos2, vec2,
};

use super::super::grid_cell::paint_labels;
use super::super::{ColourLabel, ColourLabels, context_menu, paint_keep_marker};
use crate::popup::{Entry, Item};
use crate::{Icon, theme, translator};

/// One setting's row.
pub struct SettingRow<'a> {
    id: Id,
    label: &'a str,
    help: Option<&'a str>,
    /// The default's words, while the value is off it.
    default: Option<&'a str>,
    after_restart: bool,
    disabled: Option<&'a str>,
    whole_row: bool,
    read_only: bool,
    matched: bool,
    note: Option<Note<'a>>,
    glyph: Option<RowGlyph>,
}

/// The mark before a row's label, the thing it notes drawn as everywhere else: a rating's
/// star or the reject, an icon, or a colour label's dot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowGlyph {
    Icon(Icon),
    Label(ColourLabel),
}

/// What a row's owner says under its control.
#[derive(Clone, Copy, Debug)]
pub struct Note<'a> {
    pub text: &'a str,
    /// In `warning` rather than `text_secondary`.
    pub warning: bool,
    /// Announced as it appears, a polite live region, where the person's change on another
    /// row made it.
    pub live: bool,
}

/// What a frame of the row did.
#[derive(Debug)]
pub struct RowOutput<R> {
    /// What the control drawn in it returned.
    pub inner: R,
    /// The value is to go back to its default: by the mark, a double-click on the label,
    /// Delete on it or its menu's Reset.
    pub reset: bool,
    /// A click on a whole row's target outside its control, which toggles its switch, at once:
    /// a pointer's, but the second of a double-click, which resets instead, a key's or a
    /// screen reader's.
    pub clicked: bool,
}

impl<'a> SettingRow<'a> {
    /// `id` names the row, stable across frames; `label` is its words, translated.
    pub fn new(id: impl Hash + std::fmt::Debug, label: &'a str) -> Self {
        SettingRow {
            id: Id::unique(("setting row", id)),
            label,
            help: None,
            default: None,
            after_restart: false,
            disabled: None,
            whole_row: false,
            read_only: false,
            matched: false,
            note: None,
            glyph: None,
        }
    }

    /// Its help, translated: the row's tooltip.
    pub fn help(mut self, text: &'a str) -> Self {
        self.help = Some(text);
        self
    }

    /// The value is off its default, whose words, translated, `default` is ("Dark",
    /// "100 %"): the changed mark shows, "Reset to" them.
    pub fn changed(mut self, default: &'a str) -> Self {
        self.default = Some(default);
        self
    }

    /// The setting applies after a restart, which the row says after its label.
    pub fn after_restart(mut self, after: bool) -> Self {
        self.after_restart = after;
        self
    }

    /// The row cannot be changed now, for the reason `why`, translated: dimmed, no mark.
    pub fn disabled(mut self, why: &'a str) -> Self {
        self.disabled = Some(why);
        self
    }

    /// The whole row is the target, as a switch's: a click outside its control is
    /// reported in [`RowOutput::clicked`] at once, the second of a double-click resetting.
    pub fn whole_row(mut self, whole: bool) -> Self {
        self.whole_row = whole;
        self
    }

    /// The row matches what Find a setting holds: its label is underlined.
    pub fn matched(mut self, matched: bool) -> Self {
        self.matched = matched;
        self
    }

    /// A row that only shows something, as Storage's folders: its label takes no focus and
    /// has no menu, there being nothing to reset.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// What is said under its control.
    pub fn note(mut self, note: Note<'a>) -> Self {
        self.note = Some(note);
        self
    }

    /// The mark drawn before its label.
    pub fn glyph(mut self, glyph: RowGlyph) -> Self {
        self.glyph = Some(glyph);
        self
    }

    /// Whether the right-click menu of the row `id` is open, which Escape closes first.
    pub fn menu_open(ctx: &egui::Context, id: impl Hash + std::fmt::Debug) -> bool {
        let target = Id::unique(("setting row", id)).with("label");
        crate::popup::is_open(ctx, target.with("context menu"))
    }

    /// Draws the row across the width `ui` has, the control drawn by `control` in the
    /// column; returns what it and the row did.
    pub fn show<R>(self, ui: &mut Ui, control: impl FnOnce(&mut Ui) -> R) -> RowOutput<R> {
        let ctx = ui.ctx().clone();
        let t = theme(&ctx);
        let words = translator(&ctx);
        let enabled = self.disabled.is_none() && ui.is_enabled();
        let width = ui.available_width();
        let top = ui.cursor().min;
        let line = t.sizes.setting_row;
        let pad = ((line - t.sizes.setting_control) / 2.0).max(0.0);
        let inset = t.spacing.s;
        let column_width = t.sizes.setting_column.min(width - 2.0 * inset).max(0.0);
        let column_end = top.x + width - inset;
        let column_left = column_end - column_width;
        let marked = enabled && !self.read_only && self.default.is_some();
        let body = FontId::proportional(t.text.body);
        let (text, secondary) = if enabled {
            (t.colors.text, t.colors.text_secondary)
        } else {
            (t.colors.text_disabled, t.colors.text_disabled)
        };
        // After the label: the mark's column and the restart note, the label wrapping in
        // the room they leave.
        let restart = self
            .after_restart
            .then(|| words.text("settings-restart-note"));
        let note_galley = restart.as_ref().map(|said| {
            let mut job = LayoutJob::default();
            let format = TextFormat::simple(FontId::proportional(t.text.caption), secondary);
            job.append("·", 0.0, format.clone());
            job.append(said, t.spacing.xs, format);
            ui.painter().layout_job(job)
        });
        // The label after the glyph, where it has one.
        let glyph_at = top.x + inset;
        let start = glyph_at + self.glyph.map_or(0.0, |_| t.sizes.icon + t.spacing.s);
        let room = column_left - t.spacing.l - start;
        let after = if marked { t.sizes.marker_column } else { 0.0 }
            + note_galley
                .as_ref()
                .map_or(0.0, |g| g.size().x + t.spacing.xs);
        // Where the label and what follows it have no room beside the column, as in a
        // window too narrow for it, the control goes under them, never over them.
        let beside = room - after - t.spacing.xs;
        let stacked = beside < t.sizes.value_column;
        let wrap = if stacked {
            column_end - start - after - t.spacing.xs
        } else {
            beside
        };
        let mut job = LayoutJob::simple(
            self.label.to_owned(),
            body,
            text,
            wrap.max(t.sizes.value_column.min(column_end - start)),
        );
        if self.matched {
            for section in &mut job.sections {
                section.format.underline = egui::Stroke::new(t.shape.hairline, text);
            }
        }
        let label = ui.painter().layout_job(job);
        let first = label
            .rows
            .first()
            .map_or(label.size().y, |row| row.height());
        let label_at = pos2(start, top.y + (line - first) / 2.0);
        let label_end = start + label.size().x;
        let label_bottom = label_at.y + label.size().y;
        let column_top = if stacked {
            label_bottom.max(top.y + line) + t.spacing.xs
        } else {
            top.y + pad
        };
        let column = Rect::from_min_size(
            pos2(column_left, column_top),
            vec2(column_width, f32::INFINITY),
        );
        // The target: the label's own, or the whole row, as tall as its label where it
        // wraps. It comes before the control, so the control keeps its own clicks.
        let target_rect = if self.whole_row {
            Rect::from_min_max(top, pos2(top.x + width, (top.y + line).max(label_bottom)))
        } else {
            Rect::from_min_max(top, pos2(label_end, (top.y + line).max(label_bottom)))
        };
        // A read-only or disabled row's label takes no focus and has no menu: there is
        // nothing it could do. Its tooltip still says why.
        let sense = if self.read_only || !enabled {
            Sense::hover()
        } else {
            Sense::click()
        };
        let target = ui.interact(target_rect, self.id.with("label"), sense);
        // Delete and the menu key act on the focused label, so it shows the ring.
        if target.has_focus() {
            let label_rect = Rect::from_min_size(label_at, label.size());
            super::ring(ui.painter(), label_rect.expand(t.spacing.xxs), &t);
        }
        ui.painter().galley(label_at, label, text);
        if let Some(glyph) = self.glyph {
            let side = t.sizes.icon;
            let centre = pos2(glyph_at + side / 2.0, top.y + line / 2.0);
            let rect = Rect::from_center_size(centre, vec2(side, side));
            match glyph {
                RowGlyph::Icon(icon) => icon.paint(ui.painter(), rect, text),
                RowGlyph::Label(label) => {
                    let dot = rect.shrink(t.spacing.xxs);
                    paint_labels(ui.painter(), &t, dot, ColourLabels::of([label]));
                }
            }
        }
        let mut x = label_end + t.spacing.xs;
        let mut reset = false;
        // The mark, a button.
        if let (true, Some(default)) = (marked, self.default) {
            let mut args = crate::i18n::Args::new();
            args.set("value", default);
            let reset_to = words.text_with("settings-reset-to", Some(&args));
            let rect = Rect::from_min_size(pos2(x, top.y), vec2(t.sizes.marker_column, line));
            let mark = ui.interact(rect, self.id.with("mark"), Sense::click());
            paint_keep_marker(ui.painter(), rect.center(), true, false, &t, |c| c);
            if mark.has_focus() {
                super::ring(ui.painter(), rect.shrink(t.spacing.xxs), &t);
            }
            mark.widget_info(|| WidgetInfo::labeled(Role::Button, true, &reset_to));
            reset |= mark.on_hover_text(&reset_to).clicked();
            x = rect.right();
        }
        if let (Some(galley), Some(said)) = (note_galley, restart.as_deref()) {
            let at = pos2(x, top.y + (line - galley.size().y) / 2.0);
            let rect = Rect::from_min_size(at, galley.size());
            ui.painter().galley(at, galley, secondary);
            let note = ui.interact(rect, self.id.with("restart"), Sense::hover());
            note.widget_info(|| WidgetInfo::labeled(Role::Label, enabled, said));
        }
        // The control, in its column; what a read-only row shows may take the room its
        // label leaves, so a long path is cut only where it must be.
        let shown = if self.read_only {
            let from = (x + t.spacing.l).min(column.left());
            Rect::from_min_max(pos2(from, column.top()), column.max)
        } else {
            column
        };
        let mut column_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(shown)
                .layout(Layout::top_down(Align::Min))
                .id_salt(self.id.with("control")),
        );
        if !enabled {
            column_ui.disable();
        }
        let inner = control(&mut column_ui);
        let used = column_ui.min_rect();
        let mut bottom = (top.y + line)
            .max(used.bottom() + pad)
            .max(label_bottom + pad);
        // What the row's owner says of it, under the control.
        if let Some(note) = self.note {
            let colour = if note.warning {
                t.colors.warning
            } else {
                t.colors.text_secondary
            };
            let galley = ui.painter().layout(
                note.text.to_owned(),
                FontId::proportional(t.text.caption),
                colour,
                column_width,
            );
            let at = pos2(column.left(), used.bottom() + t.spacing.xs);
            let rect = Rect::from_min_size(at, galley.size());
            ui.painter().galley(at, galley, colour);
            let said = ui.interact(rect, self.id.with("note"), Sense::hover());
            said.widget_info(|| WidgetInfo::labeled(Role::Label, true, note.text));
            if note.live {
                let polite = egui::accesskit::Live::Polite;
                ctx.accesskit_node_builder(said.id, |node| node.set_live(polite));
            }
            bottom = bottom.max(rect.bottom() + pad);
        }
        let row = Rect::from_min_max(top, pos2(top.x + width, bottom));
        // A dimmed row says why over its control too, which takes no pointer while disabled.
        if let Some(why) = self.disabled {
            let over = Rect::from_min_max(pos2(column.left(), top.y), pos2(column.right(), bottom));
            ui.interact(over, self.id.with("why"), Sense::hover())
                .on_hover_text(why);
        }
        // The target's words, help and resets.
        let resets = marked;
        target.widget_info(|| WidgetInfo::labeled(Role::Label, enabled, self.label));
        let described = match self.disabled {
            Some(why) => Some(why.to_owned()),
            None if self.default.is_some() => Some(words.text("settings-changed")),
            None => None,
        };
        if let Some(described) = described {
            ctx.accesskit_node_builder(target.id, |node| node.set_description(described));
        }
        let delete =
            target.has_focus() && ui.input_mut(|i| super::super::steps::plain(i, Key::Delete));
        // A double-click is two quick primary clicks on this label, the first not another
        // row's, as egui counts one wherever its first click landed within a few points: a
        // click anywhere else, the row's switch included, starts the count again. Its
        // interval is the primary clicks' own, as egui's runs from any button's: a middle
        // click between two slow ones makes no double.
        let last = last_click_id();
        let ran = ctx.data(|d| d.get_temp::<(Id, u32, f64)>(last));
        let mut double = false;
        if target.clicked_by(PointerButton::Primary) {
            let now = ctx.input(|i| i.time);
            let delay = ctx.options(|o| o.input_options.max_double_click_delay);
            let quick = target.double_clicked() || target.triple_clicked();
            let run = match ran {
                Some((id, run, at)) if quick && id == target.id && now - at < delay => run + 1,
                _ => 1,
            };
            ctx.data_mut(|d| d.insert_temp(last, (target.id, run, now)));
            double = run == 2;
        } else if ran.is_some_and(|(id, _, _)| id == target.id)
            && ctx.input(|i| i.pointer.button_clicked(PointerButton::Primary))
        {
            ctx.data_mut(|d| d.remove::<(Id, u32, f64)>(last));
        }
        // As darktable's: a switch's row resets on the second click of a double-click
        // though its first, toggling, left it at its default, so it ends at the default.
        reset |= resets && delete;
        reset |= (resets || (self.whole_row && enabled)) && double;
        let menu = if self.read_only || !enabled {
            None
        } else {
            context_menu(ui, &target, self.label, || {
                let item = Item::new("reset", words.text("settings-reset")).enabled(resets);
                vec![Entry::Item(item)]
            })
        };
        reset |= resets && menu.as_deref() == Some("reset");
        // Each click toggles a switch's row at once, as darktable's label does on every
        // press, but the second of a double-click, which resets it; a key's and a screen
        // reader's too.
        let clicked = self.whole_row && enabled && target.clicked() && !double;
        if let Some(tip) = self.disabled.or(self.help) {
            target.on_hover_text(tip);
        }
        ui.advance_cursor_after_rect(row);
        RowOutput {
            inner,
            reset,
            clicked,
        }
    }
}

/// Where the label last clicked, how many quick clicks it has had in a row and when the
/// last was, is kept.
fn last_click_id() -> Id {
    Id::unique("setting row last click")
}
