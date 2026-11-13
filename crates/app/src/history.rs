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

//! The edit's history as the app words and draws it (`docs/history.md`, In the app;
//! decision 78): the History section, and what Undo and Redo would do, named as History
//! names a step, for Edit's items.
//!
//! A step with a note reads its note, Reset's among them, and one without its module's
//! title; a batch, the steps one change made on several modules, its note, else the name
//! of its first step the app knows and how many more it knows; a `GoTo` taken back or put back, Go to Step. darktable's
//! label after a • and a layer after a · join a module's title with layers (Phase 4).
//!
//! **The section** lists the mirror's `History` as darktable's history module lists its
//! stack (`src/libs/history.c`, `gui_update`), the wireframe's History frame:
//!
//! - a line per step, newest at the top, numbered from 1, its module's name, as
//!   darktable's history item reads, even with a note, Reset's reading Reset, and its
//!   source's tag at its end (You, Auto, Style, Assistant); the end's line chosen, on `list_chosen`,
//!   as darktable's active item, and the steps after it greyed, which darktable draws as
//!   any other (`docs/history.md`, Against darktable);
//! - then the Default lines, the modules every photo runs, unnumbered, tagged Default,
//!   and Original, 0, one block that always applies, never greyed; at end 0 the top
//!   Default line chosen, or Original with none;
//! - a step's tooltip: a small table, a row for each control it changed, its label, its
//!   value before, an arrow and its value after as its row writes them, in lined-up
//!   columns, then its note, who made it and when apart, on the local clock in the
//!   interface's language; a Default line's who and when alone;
//! - a click on a step goes to it ([`Action::GoTo`]), and on a Default line or Original
//!   to Original, but on where the end is, which changes nothing, as darktable's click on
//!   its active item; Shift-click on a step or a Default line opens its module's section
//!   ([`Action::Open`]), as darktable's Shift-click focuses it;
//! - the keys: ↑ and ↓ move between the lines, Enter does what a click does, Shift+Enter
//!   what Shift-click does; to screen readers a list of items, each named by its number
//!   and name, its tooltip its description, the end selected;
//! - Reset in the header, which resets the edit as one step, as Reset Edit does.
//!
//! The eye and Compress history come with later parts (9.7 and 9.8).

use std::sync::Arc;

use apertrust_engine::session::{
    ControlChange, ControlValue, History, HistoryStep, Mark, Move, Note, StepId, StepSource,
};
use apertrust_modules::{ControlKind, Definition, Module, Row, Slider, messages};
use apertrust_ui::components::{Section, SectionAction, focus_ring_centred};
use apertrust_ui::i18n::Args;
use apertrust_ui::{Translator, fonts, theme, translator};
use eframe::egui::text::LayoutJob;
use eframe::egui::{self, Align, FontId, Role, Sense, UiBuilder, WidgetInfo, pos2, vec2};

use crate::menus::Steps;

/// The section's id, by which settings keep whether it is open.
pub const ID: &str = "history";

/// Whether the section is open when settings have not kept it otherwise.
pub const OPEN: bool = true;

/// What separates the parts of a tooltip's line, as the wireframe writes it.
const SEPARATOR: &str = " · ";

/// What the user did in the section this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Open it, or close it.
    Toggle,
    /// Go to a step, or to Original with `None`: `Command::GoTo`.
    GoTo(Option<StepId>),
    /// Open this module's section in the side panel.
    Open(&'static str),
    /// Reset the edit, as Reset Edit does.
    Reset,
}

/// Where a line stands in the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// A step, numbered from 1: `applied` while it is at or under the end, and `end` the
    /// last applied.
    Step {
        id: StepId,
        number: usize,
        applied: bool,
        end: bool,
    },
    /// A Default line: a module every photo runs, never greyed. With Original it is one
    /// block under the steps, which a click goes to, end 0: `there` while the end is
    /// there, and `end` for the top Default line then, the boundary between what is undone
    /// and what always applies, which is chosen.
    Default { end: bool, there: bool },
    /// Original, 0: `there` at end 0, and `end` then where no Default line is over it.
    Original { end: bool, there: bool },
}

/// One line of the list, in the interface's words.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub place: Place,
    pub name: String,
    /// Its source's tag at its end; none on Original.
    pub tag: Option<String>,
    pub tip: Tip,
    /// The module Shift-click opens; none where the app
    /// does not know it, as an operation's unowned parameters or a newer build's module.
    pub module: Option<&'static str>,
}

/// A line's tooltip.
#[derive(Clone, Debug, PartialEq)]
pub enum Tip {
    /// Words alone: Original's.
    Text(String),
    /// A step's: each module's changes, then who made it and when, each part apart
    /// ("You", "21/10/2026, 17:52").
    Changes {
        modules: Vec<Changed>,
        by: Vec<String>,
    },
}

/// What a step changed of one module, for its tooltip.
#[derive(Clone, Debug, PartialEq)]
pub struct Changed {
    /// The module's title, over its rows where the step touched several modules.
    pub heading: Option<String>,
    pub rows: Vec<TipRow>,
}

/// One row of a step's tooltip.
#[derive(Clone, Debug, PartialEq)]
pub enum TipRow {
    /// A control's label, and its value before and after as its row writes them.
    Value {
        label: String,
        before: String,
        after: String,
    },
    /// Something said whole: a control kept, released or set by Auto.
    Said(String),
}

impl Tip {
    /// What screen readers hear of it, plain text in the order it is drawn: "Exposure,
    /// +0.700 EV to +1.200 EV; You, 21/10/2026, 17:52".
    pub fn said(&self, tr: &Translator) -> String {
        let (modules, by) = match self {
            Tip::Text(text) => return text.clone(),
            Tip::Changes { modules, by } => (modules, by),
        };
        let mut parts = Vec::new();
        for changed in modules {
            parts.extend(changed.heading.clone());
            for row in &changed.rows {
                parts.push(match row {
                    TipRow::Value {
                        label,
                        before,
                        after,
                    } => {
                        let mut args = Args::new();
                        args.set("control", label.as_str());
                        args.set("before", before.as_str());
                        args.set("after", after.as_str());
                        tr.text_with("history-said-change", Some(&args))
                    }
                    TipRow::Said(said) => said.clone(),
                });
            }
        }
        parts.push(by.join(", "));
        parts.join("; ")
    }
}

impl Line {
    /// Its number: a step's, and Original's 0; a Default line has none.
    fn number(&self) -> Option<usize> {
        match self.place {
            Place::Step { number, .. } => Some(number),
            Place::Original { .. } => Some(0),
            Place::Default { .. } => None,
        }
    }

    /// Whether it is the end's line, chosen: at end 0, the top Default line, else
    /// Original.
    fn end(&self) -> bool {
        matches!(
            self.place,
            Place::Step { end: true, .. }
                | Place::Default { end: true, .. }
                | Place::Original { end: true, .. }
        )
    }

    /// Whether it takes a click or a Shift-click, which its hover band says: a click going
    /// somewhere while the photo takes edits (`editable`), or a Shift-click opening its
    /// module, as on a Default line, whatever the photo. Original, the end already there,
    /// takes neither.
    fn takes(&self, editable: bool) -> bool {
        (editable && !self.there()) || self.module.is_some()
    }

    /// Whether a click goes nowhere: on the end's line, or on the block of Default lines
    /// and Original while the end is there.
    fn there(&self) -> bool {
        match self.place {
            Place::Step { end, .. } => end,
            Place::Default { there, .. } | Place::Original { there, .. } => there,
        }
    }

    /// Whether it is a step after the end, greyed.
    fn greyed(&self) -> bool {
        matches!(self.place, Place::Step { applied: false, .. })
    }
}

/// The list's lines from `history`, the open photo's, in `tr`'s words: its steps newest
/// first, the Default lines, also last added first, then Original; Original alone with no
/// history. `modules`
/// are the session's, whose titles, controls and rows word the lines.
pub fn lines(history: Option<&History>, modules: &[&dyn Module], tr: &Translator) -> Vec<Line> {
    let mut lines = Vec::new();
    let end = history.map_or(0, |h| h.end);
    if let Some(history) = history {
        for (index, step) in history.steps.iter().enumerate().rev() {
            let number = index + 1;
            lines.push(Line {
                place: Place::Step {
                    id: step.id,
                    number,
                    applied: number <= end,
                    end: number == end,
                },
                name: line_name(step, modules, tr),
                tag: Some(source(step.source, tr)),
                tip: tip(step, modules, tr),
                module: (step.changes.iter()).find_map(|change| known(modules, &change.module)),
            });
        }
        // Newest first, as the steps: the first darktable adds, Raw black/white point,
        // just over Original.
        for (index, &module) in history.defaults.iter().rev().enumerate() {
            lines.push(Line {
                place: Place::Default {
                    end: end == 0 && index == 0,
                    there: end == 0,
                },
                name: title(modules, module, tr),
                tag: Some(source(StepSource::Default, tr)),
                // Its source alone: the photo's own, with no change or time to show.
                tip: Tip::Changes {
                    modules: Vec::new(),
                    by: vec![source(StepSource::Default, tr)],
                },
                module: known(modules, module),
            });
        }
    }
    lines.push(Line {
        place: Place::Original {
            end: end == 0 && history.is_none_or(|h| h.defaults.is_empty()),
            there: end == 0,
        },
        name: tr.text("history-original"),
        tag: None,
        tip: Tip::Text(tr.text("history-original-tip")),
        module: None,
    });
    lines
}

/// Draws the section, open or closed as `open` says, its lines from `history`, the open
/// photo's, drawn disabled where there is none; it peeks while `peek` holds, as every
/// section does. A step is gone to only while `editable` (a photo live and not open for
/// viewing only), and Reset runs only while `resettable`. Returns what the user did.
pub fn section(
    ui: &mut egui::Ui,
    history: Option<&History>,
    modules: &[&dyn Module],
    open: bool,
    peek: bool,
    editable: bool,
    resettable: bool,
) -> Vec<Action> {
    let words = translator(ui.ctx());
    let title = words.text("section-history");
    let reset = words.text("history-reset-edit");
    let mut actions = Vec::new();
    // A utility: Reset before its options, which are drawn disabled while nothing is
    // behind them.
    let out = Section::new(&title, open)
        .utility_reset(&reset)
        .reset(resettable)
        .options(false)
        .peek(peek)
        .show(ui, |ui| {
            let lines = kept_lines(ui.ctx(), history, modules, &words);
            // Disabled through the list's own builder, so the lines are its children, as
            // screen readers count them.
            let list = UiBuilder::new()
                .id_salt("history lines")
                .accessibility_role(Role::List)
                .accessibility_label(&title);
            let list = if history.is_some() {
                list
            } else {
                list.disabled()
            };
            ui.scope_builder(list, |ui| {
                let column = number_column(ui, &lines);
                for line in lines.iter() {
                    actions.extend(line_row(ui, line, column, editable));
                }
            });
        });
    let mut done: Vec<Action> = (out.actions.into_iter())
        .filter_map(|action| match action {
            SectionAction::Toggle => Some(Action::Toggle),
            SectionAction::Reset => Some(Action::Reset),
            _ => None,
        })
        .collect();
    done.extend(actions);
    done
}

/// [`lines`], worded once for each history and language and kept in egui's memory: a
/// history of hundreds of steps would otherwise be worded again each frame. A history is
/// told from another by its `through`, its end, its Default lines and its steps, each
/// behind the `Arc` the session shares while the step stays as it is.
fn kept_lines(
    ctx: &egui::Context,
    history: Option<&History>,
    modules: &[&dyn Module],
    tr: &Translator,
) -> Arc<Vec<Line>> {
    let steps = history.map(|h| {
        let shared = h.steps.iter().map(|step| Arc::as_ptr(step) as usize);
        (h.through, h.end, &h.defaults, shared.collect::<Vec<_>>())
    });
    let key = {
        use std::hash::{BuildHasher as _, BuildHasherDefault, DefaultHasher};
        BuildHasherDefault::<DefaultHasher>::default().hash_one((tr.locale().id(), steps))
    };
    // One slot, the latest only.
    let slot = egui::Id::unique("history lines");
    let kept = ctx.data(|data| data.get_temp::<(u64, Arc<Vec<Line>>)>(slot));
    if let Some((kept, lines)) = kept
        && kept == key
    {
        return lines;
    }
    let worded = Arc::new(lines(history, modules, tr));
    ctx.data_mut(|data| data.insert_temp(slot, (key, Arc::clone(&worded))));
    worded
}

/// How wide the numbers' column is: the widest number's width, in the small text.
fn number_column(ui: &egui::Ui, lines: &[Line]) -> f32 {
    let t = theme(ui.ctx());
    let widest = lines.iter().filter_map(Line::number).max().unwrap_or(0);
    let text = translator(ui.ctx()).format_number(widest as f64, 0, false);
    let font = FontId::proportional(t.text.small);
    let galley = ui.painter().layout_no_wrap(text, font, t.colors.text);
    galley.size().x
}

/// One line, `sizes.panel_line` high: its number right-aligned in a column `column`
/// wide, its name cut at its end, and its tag at the row's end, as Browse's rows are
/// drawn; the end's on `list_chosen`, a later step's greyed. Returns what a click, or
/// Enter, asked for.
fn line_row(ui: &mut egui::Ui, line: &Line, column: f32, editable: bool) -> Option<Action> {
    let t = theme(ui.ctx());
    let words = translator(ui.ctx());
    let size = vec2(ui.available_width(), t.sizes.panel_line);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    // A line the keys reach is scrolled into sight.
    if response.gained_focus() {
        response.scroll_to_me(None);
    }
    let (chosen, greyed) = (line.end(), line.greyed());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let band = rect.expand2(vec2(t.spacing.xs, 0.0));
        if chosen {
            painter.rect_filled(band, t.shape.radius, t.colors.list_chosen);
        } else if response.hovered() && line.takes(editable) {
            painter.rect_filled(band, t.shape.radius, t.colors.hover);
        }
        let (ink, quiet) = if greyed || !ui.is_enabled() {
            (t.colors.text_disabled, t.colors.text_disabled)
        } else if chosen {
            (t.colors.text_strong, t.colors.text_secondary)
        } else {
            (t.colors.text, t.colors.text_secondary)
        };
        let small = FontId::proportional(t.text.small);
        let middle = rect.center().y;
        if let Some(number) = line.number() {
            let number = words.format_number(number as f64, 0, false);
            let galley = painter.layout_no_wrap(number, small.clone(), quiet);
            let at = pos2(rect.left() + column - galley.size().x, middle);
            painter.galley(at - vec2(0.0, galley.size().y / 2.0), galley, quiet);
        }
        let mut end = rect.right();
        if let Some(tag) = &line.tag {
            let caption = FontId::proportional(t.text.caption);
            let galley = painter.layout_no_wrap(tag.clone(), caption, quiet);
            end -= galley.size().x;
            painter.galley(pos2(end, middle - galley.size().y / 2.0), galley, quiet);
            end -= t.spacing.s;
        }
        let start = rect.left() + column + t.spacing.s;
        let font = FontId::proportional(t.text.body);
        let mut job = LayoutJob::simple_singleline(line.name.clone(), font, ink);
        job.wrap.max_width = (end - start).max(0.0);
        job.wrap.max_rows = 1;
        job.wrap.break_anywhere = true;
        job.halign = Align::LEFT;
        let galley = painter.layout_job(job);
        painter.galley(pos2(start, middle - galley.size().y / 2.0), galley, ink);
        if response.has_focus() {
            focus_ring_centred(painter, band, &t);
        }
    }
    // Worded only when asked, by screen readers: a long history has many lines.
    let enabled = ui.is_enabled();
    response.widget_info(|| {
        let name = match line.place {
            Place::Step { number, .. } => {
                let mut args = Args::new();
                args.set("number", number);
                args.set("name", line.name.as_str());
                words.text_with("history-step-name", Some(&args))
            }
            Place::Default { .. } | Place::Original { .. } => line.name.clone(),
        };
        WidgetInfo::labeled(Role::ListItem, enabled, name)
    });
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_selected(chosen);
        // A later step's description says first that it does not apply.
        node.set_description(if greyed {
            format!(
                "{}\n{}",
                words.text("section-not-applied"),
                line.tip.said(&words)
            )
        } else {
            line.tip.said(&words)
        });
    });
    let response = response.on_hover_ui(|ui| tip_ui(ui, &line.tip));
    if !response.clicked() {
        return None;
    }
    let shift = shift_clicked(ui);
    match line.place {
        Place::Step { .. } | Place::Default { .. } if shift => line.module.map(Action::Open),
        Place::Original { .. } if shift => None,
        // The end where it is changes nothing, as darktable's click on its active item.
        _ if line.there() || !editable => None,
        Place::Step { id, .. } => Some(Action::GoTo(Some(id))),
        // A Default line is Original: the modules every photo runs always run.
        Place::Default { .. } | Place::Original { .. } => Some(Action::GoTo(None)),
    }
}

/// Draws a line's tooltip. A step's is a small table: a row for each control, its label,
/// its value before, an arrow and its value after, the values right-aligned in the small
/// text a slider row writes its value in, so they line up down the rows; a module's
/// title over its rows where the step touched several; then who made it and when, apart,
/// in the secondary text.
fn tip_ui(ui: &mut egui::Ui, tip: &Tip) {
    let t = theme(ui.ctx());
    let (modules, by) = match tip {
        Tip::Text(text) => {
            ui.label(text);
            return;
        }
        Tip::Changes { modules, by } => (modules, by),
    };
    let font = FontId::proportional(t.text.small);
    let fonts = ui.ctx().clone();
    let layout = |text: &str, colour| {
        fonts.fonts_mut(|f| f.layout_no_wrap(text.to_owned(), font.clone(), colour))
    };
    // The columns, as wide as their widest cell across every module's rows: label, before,
    // the arrow, after.
    let arrow = layout("→", t.colors.text_secondary);
    let mut widths = [0.0_f32; 3];
    for row in modules.iter().flat_map(|changed| &changed.rows) {
        if let TipRow::Value {
            label,
            before,
            after,
        } = row
        {
            for (width, text) in widths.iter_mut().zip([label, before, after]) {
                *width = width.max(layout(text, t.colors.text).size().x);
            }
        }
    }
    let gap = t.spacing.s;
    let before_end = widths[0] + gap + widths[1];
    let arrow_start = before_end + gap;
    let after_end = arrow_start + arrow.size().x + gap + widths[2];
    let height = arrow.size().y;
    for changed in modules {
        if let Some(heading) = &changed.heading {
            let medium = FontId::new(t.text.small, egui::FontFamily::Name(fonts::MEDIUM.into()));
            let heading = egui::RichText::new(heading)
                .font(medium)
                .color(t.colors.text);
            ui.label(heading);
        }
        for row in &changed.rows {
            let TipRow::Value {
                label,
                before,
                after,
            } = row
            else {
                if let TipRow::Said(said) = row {
                    ui.label(
                        egui::RichText::new(said)
                            .font(font.clone())
                            .color(t.colors.text),
                    );
                }
                continue;
            };
            let (rect, _) = ui.allocate_exact_size(vec2(after_end, height), Sense::hover());
            let painter = ui.painter();
            let at = |x: f32| pos2(rect.left() + x, rect.top());
            let ink = t.colors.text;
            painter.galley(at(0.0), layout(label, ink), ink);
            let before = layout(before, ink);
            painter.galley(at(before_end - before.size().x), before, ink);
            painter.galley(at(arrow_start), arrow.clone(), t.colors.text_secondary);
            let after = layout(after, ink);
            painter.galley(at(after_end - after.size().x), after, ink);
        }
    }
    // Apart from the rows; a Default line's and a Default step's alone.
    if !modules.is_empty() {
        ui.add_space(t.spacing.xs);
    }
    let by = egui::RichText::new(by.join(SEPARATOR))
        .font(font)
        .color(t.colors.text_secondary);
    ui.label(by);
}

/// Whether this frame's click came with Shift held, as the click's own event says: the
/// primary button's release, or Enter's or Space's press. Not the input's modifiers, which
/// are only as fresh as the last change egui was told of: a change lost while a modal
/// dialog held the window's input would leave Shift down there, and every click a
/// Shift-click. A screen reader's click carries none, and is plain.
fn shift_clicked(ui: &egui::Ui) -> bool {
    ui.input(|i| {
        (i.events.iter().rev())
            .find_map(|event| match event {
                egui::Event::PointerButton {
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers,
                    ..
                }
                | egui::Event::Key {
                    key: egui::Key::Enter | egui::Key::Space,
                    pressed: true,
                    modifiers,
                    ..
                } => Some(modifiers.shift),
                _ => None,
            })
            .unwrap_or(false)
    })
}

/// `id` among `modules`, as the app knows it; none for a module or an operation it does
/// not.
fn known(modules: &[&dyn Module], id: &str) -> Option<&'static str> {
    (modules.iter())
        .map(|module| module.definition().id)
        .find(|known| *known == id)
}

/// The definition of the module `id`, where the app knows it.
fn definition(modules: &[&dyn Module], id: &str) -> Option<&'static Definition> {
    (modules.iter())
        .map(|module| module.definition())
        .find(|definition| definition.id == id)
}

/// A module's title, or the stored name of one the app does not know.
fn title(modules: &[&dyn Module], id: &str, tr: &Translator) -> String {
    match known(modules, id) {
        Some(id) => tr.text(&messages::title(id)),
        None => id.to_owned(),
    }
}

/// A source's tag: You, Auto, Style, Assistant or Default.
fn source(source: StepSource, tr: &Translator) -> String {
    tr.text(match source {
        StepSource::You => "history-source-you",
        StepSource::Auto => "history-source-auto",
        StepSource::Style => "history-source-style",
        StepSource::Assistant => "history-source-assistant",
        StepSource::Default => "history-source-default",
    })
}

/// Whether `step` is Reset's, the one step touching every module it reset.
fn is_reset(step: &HistoryStep) -> bool {
    matches!(&step.note, Some(Note::Message { id, .. }) if id == Note::RESET)
}

/// A step's line: its module's title, as darktable's history item reads, even with a note,
/// which its tooltip gives; the stored name of a module the app does not know, as a newer
/// build's or an operation's unowned parameters; Reset's, which touches every module it
/// reset, its note, Reset. A step that names nothing reads its source, so a line never
/// reads empty.
fn line_name(step: &HistoryStep, modules: &[&dyn Module], tr: &Translator) -> String {
    match (&step.note, step.changes.as_slice()) {
        (Some(note), _) if is_reset(step) => noted(note, tr),
        (_, [one]) => title(modules, &one.module, tr),
        (Some(note), _) => noted(note, tr),
        (None, _) => source(step.source, tr),
    }
}

/// A step's tooltip: its module's changes, then its note, who made it and when, on the
/// local clock. Reset's, the one step of several modules, has a heading over each
/// module's rows, and its note is its line's name. A step of source Default, the photo's
/// own as a Default line is, has who and when alone.
fn tip(step: &HistoryStep, modules: &[&dyn Module], tr: &Translator) -> Tip {
    let several = step.changes.len() > 1;
    let mut changed = Vec::new();
    let shown = (step.source != StepSource::Default).then_some(&step.changes);
    for change in shown.into_iter().flatten() {
        let definition = definition(modules, &change.module);
        // The controls alone, not a switched module's turning on or off, which they show:
        // a reset turning one off, its controls going back to their defaults.
        let rows: Vec<TipRow> = (change.controls.iter())
            .map(|control| control_change(definition, control, tr))
            .collect();
        // A module with nothing to describe, as a newer build's, shows by its heading
        // alone, and on a step of its own by the line's name.
        if several || !rows.is_empty() {
            changed.push(Changed {
                heading: several.then(|| title(modules, &change.module, tr)),
                rows,
            });
        }
    }
    // A style's or an assistant's note, beside who and when.
    let note = step.note.as_ref().filter(|_| !is_reset(step));
    let mut by: Vec<String> = note.map(|note| noted(note, tr)).into_iter().collect();
    by.push(source(step.source, tr));
    // A version 1 edit whose file's time was unknown has none.
    if step.clock.when > 0 {
        by.push(tr.format_date_time(step.clock.when));
    }
    Tip::Changes {
        modules: changed,
        by,
    }
}

/// One control's change, as darktable's history tooltip writes it: its label, its value
/// before and after, as its row writes them; or, with its value the same, that it was
/// kept, released or set by Auto. An operation's unowned parameter, with no definition,
/// by its name.
fn control_change(
    definition: Option<&'static Definition>,
    change: &ControlChange,
    tr: &Translator,
) -> TipRow {
    let label = match definition {
        Some(definition) => tr.text(&messages::label(definition.id, change.control)),
        None => change.control.to_owned(),
    };
    if change.before == change.after {
        let id = match change.mark {
            [_, Some(Mark::Kept)] => "history-kept",
            [_, Some(Mark::Auto)] => "history-auto",
            [_, None] => "history-released",
        };
        let mut args = Args::new();
        args.set("control", label);
        return TipRow::Said(tr.text_with(id, Some(&args)));
    }
    let value = |value| value_text(definition, change.control, value, tr);
    TipRow::Value {
        label,
        before: value(&change.before),
        after: value(&change.after),
    }
}

/// A control's value as its row writes it: a number with its row's decimals, signed
/// where its range spans zero, and its unit; a choice's option; on or off; a curve's
/// points. An unowned parameter's number with the decimals it needs, up to four.
fn value_text(
    definition: Option<&'static Definition>,
    control: &str,
    value: &ControlValue,
    tr: &Translator,
) -> String {
    let defined =
        definition.and_then(|definition| Some((definition, definition.control(control)?)));
    match value {
        ControlValue::Number(number) => {
            let number = f64::from(*number);
            let Some((definition, control)) = defined else {
                return tr.format_number(number, natural_decimals(number), false);
            };
            let signed = matches!(control.kind, ControlKind::Number { min, max, .. } if min < 0.0 && max > 0.0);
            let decimals = (definition.rows())
                .find_map(|row| match row {
                    Row::Slider(slider) if slider.control == control.id => Some(slider.decimals),
                    _ => None,
                })
                .unwrap_or(Slider::new(control.id).decimals);
            let text = tr.format_number(number, usize::from(decimals), signed);
            match control.unit {
                Some(unit) => {
                    let mut args = Args::new();
                    args.set("value", text);
                    tr.text_with(&messages::unit_value(unit), Some(&args))
                }
                None => text,
            }
        }
        ControlValue::Choice(index) => {
            let option = defined.and_then(|(definition, control)| match control.kind {
                ControlKind::Choice { options, .. } => {
                    let option = options.get(*index as usize)?;
                    Some(messages::option(definition.id, control.id, option))
                }
                _ => None,
            });
            match option {
                Some(option) => tr.text(&option),
                None => tr.format_number(f64::from(*index), 0, false),
            }
        }
        ControlValue::Bool(on) => tr.text(if *on { "history-on" } else { "history-off" }),
        ControlValue::Points(points) => {
            let mut args = Args::new();
            args.set("count", points.len());
            tr.text_with("history-points", Some(&args))
        }
    }
}

/// The decimals `value` needs, up to four: 0.5 needs one, 2 none.
fn natural_decimals(value: f64) -> usize {
    (0..4)
        .find(|&decimals| {
            let scale = 10f64.powi(decimals as i32);
            ((value * scale).round() - value * scale).abs() < 1e-6
        })
        .unwrap_or(4)
}

/// What Undo and Redo would do in `history`, the open photo's, in `tr`'s words: none where
/// there is nothing to do, or nothing to name. `modules` are the session's, whose titles
/// name a step.
pub fn steps(history: Option<&History>, modules: &[&dyn Module], tr: &Translator) -> Steps {
    let side = |wanted: fn(&History) -> Option<Move>| {
        let history = history?;
        named(history, wanted(history)?, modules, tr)
    };
    Steps {
        undo: side(|h| h.undo),
        redo: side(|h| h.redo),
    }
}

/// What `moved` does, as History names it; none for steps not in `history`, or whose first
/// names no module the app knows, as an operation's unowned parameters alone.
fn named(
    history: &History,
    moved: Move,
    modules: &[&dyn Module],
    tr: &Translator,
) -> Option<String> {
    match moved {
        Move::GoTo => Some(tr.text("history-go-to-step")),
        Move::Steps { first, count } => {
            let at = history.steps.iter().position(|s| s.id == first)?;
            let steps = history.steps.get(at..at + count)?;
            name(steps, modules, tr)
        }
    }
}

/// The name of one change's steps: its note, which a batch's steps share and its last
/// carries, a Reset's coming first; else the name of its first step the app knows, and how
/// many more of those it knows.
fn name(steps: &[Arc<HistoryStep>], modules: &[&dyn Module], tr: &Translator) -> Option<String> {
    if let Some(note) = &steps.last()?.note {
        return Some(noted(note, tr));
    }
    let mut known = steps.iter().filter_map(|step| step_name(step, modules, tr));
    let first = known.next()?;
    let more = known.count();
    if more == 0 {
        return Some(first);
    }
    let mut args = Args::new();
    args.set("module", first);
    args.set("count", more);
    Some(tr.text_with("history-modules", Some(&args)))
}

/// One step's name: its note, else its module's title.
fn step_name(step: &HistoryStep, modules: &[&dyn Module], tr: &Translator) -> Option<String> {
    if let Some(note) = &step.note {
        return Some(noted(note, tr));
    }
    let change = step.changes.first()?;
    (modules.iter())
        .any(|m| m.definition().id == change.module)
        .then(|| tr.text(&apertrust_modules::messages::title(&change.module)))
}

/// A note's words: an assistant's as written, Apertrust's own in `tr`'s.
fn noted(note: &Note, tr: &Translator) -> String {
    match note {
        Note::Text(text) => text.clone(),
        Note::Message { id, args } => {
            let mut fluent = Args::new();
            for (key, value) in args {
                fluent.set(key.as_str(), value.as_str());
            }
            tr.text_with(id, Some(&fluent))
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Arc;

    use apertrust_engine::history::{Clock, MachineId};
    use apertrust_engine::session::{BatchId, ModuleChange, StepId, StepSource};
    use apertrust_ui::components::PanelBody;
    use apertrust_ui::{Builtin, Locale, Theme};
    use egui::accesskit::Role as AccessRole;
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT as _, Queryable as _};

    use super::*;

    /// A step of source You touching `modules`, one, or a reset's several, with `note`.
    pub fn step(id: u64, modules: &[&'static str], note: Option<Note>) -> Arc<HistoryStep> {
        Arc::new(HistoryStep {
            id: StepId(id),
            clock: Clock::at(0, None, MachineId::UNKNOWN),
            source: StepSource::You,
            note,
            batch: None,
            changes: modules
                .iter()
                .map(|module| ModuleChange {
                    module: (*module).into(),
                    enabled: None,
                    controls: Vec::new(),
                })
                .collect(),
        })
    }

    /// A history including `through` commands, its `steps` all applied, with `undo` and
    /// `redo` as given.
    pub fn history(
        through: u64,
        steps: Vec<Arc<HistoryStep>>,
        undo: Option<Move>,
        redo: Option<Move>,
    ) -> History {
        History {
            through,
            end: steps.len(),
            steps,
            defaults: Vec::new(),
            undo,
            redo,
        }
    }

    /// `steps`, one change's of several modules, sharing the batch `batch`.
    pub fn batched(batch: u64, steps: Vec<Arc<HistoryStep>>) -> Vec<Arc<HistoryStep>> {
        (steps.into_iter())
            .map(|step| {
                Arc::new(HistoryStep {
                    batch: Some(BatchId(batch)),
                    ..(*step).clone()
                })
            })
            .collect()
    }

    /// Undo or Redo moving the `count` steps from the step `first` on.
    pub fn moving(first: u64, count: usize) -> Move {
        Move::Steps {
            first: StepId(first),
            count,
        }
    }

    fn words(history: Option<&History>, locale: Locale) -> Steps {
        let tr = Translator::new(locale);
        steps(history, apertrust_modules::MODULES, &tr)
    }

    #[test]
    fn undo_and_redo_name_the_steps_as_history_does() {
        let style = || Some(Note::Text("Style".to_owned()));
        let mut steps = vec![step(1, &["exposure"], None)];
        steps.extend(batched(
            9,
            vec![
                step(2, &["exposure"], None),
                step(3, &["colorbalancergb"], None),
                step(4, &["agx"], None),
            ],
        ));
        steps.push(step(5, &["exposure", "agx"], Some(Note::reset())));
        steps.extend(batched(
            8,
            vec![step(6, &["exposure"], style()), step(7, &["agx"], style())],
        ));
        steps.push(step(
            8,
            &["exposure"],
            Some(Note::Text("Brighter".to_owned())),
        ));
        // An operation's unowned parameters, which no module's title names.
        steps.push(step(9, &["colorin"], None));
        steps.push(step(10, &[], None));
        steps.extend(batched(
            7,
            vec![
                step(11, &["colorin"], None),
                step(12, &["exposure"], None),
                step(13, &["colorin"], None),
                step(14, &["agx"], None),
            ],
        ));
        let named = |undo, redo| {
            let history = history(15, steps.clone(), undo, redo);
            words(Some(&history), Locale::EnGb)
        };
        let undo = |first, count| named(Some(moving(first, count)), None).undo;
        assert_eq!(undo(1, 1).as_deref(), Some("Exposure"));
        // A batch: its first module and how many more; the part `GoTo` left of one alike.
        assert_eq!(undo(2, 3).as_deref(), Some("Exposure and 2 more"));
        assert_eq!(undo(3, 2).as_deref(), Some("Colour balance RGB and 1 more"));
        assert_eq!(undo(5, 1).as_deref(), Some("Reset"));
        // A batch with a note: its note.
        assert_eq!(undo(6, 2).as_deref(), Some("Style"));
        assert_eq!(undo(8, 1).as_deref(), Some("Brighter"));
        // A batch of steps the app does not all know: the first it knows, and how many more
        // it knows.
        assert_eq!(undo(11, 4).as_deref(), Some("Exposure and 1 more"));
        assert_eq!(undo(11, 2).as_deref(), Some("Exposure"));
        // Nothing the app can name, or steps not in the log: Undo reads Undo alone.
        assert_eq!(undo(9, 1), None);
        assert_eq!(undo(10, 1), None);
        assert_eq!(undo(13, 1), None);
        assert_eq!(undo(14, 2), None);
        assert_eq!(undo(15, 1), None);
        let go_to = named(Some(moving(1, 1)), Some(Move::GoTo));
        assert_eq!(go_to.redo.as_deref(), Some("Go to Step"));
        // No history, or nothing to do.
        assert_eq!(words(None, Locale::EnGb), Steps::default());
        assert_eq!(named(None, None), Steps::default());
    }

    #[test]
    fn a_step_is_named_in_the_interface_s_language() {
        let mut steps = vec![step(1, &["exposure"], None)];
        steps.extend(batched(
            9,
            vec![
                step(2, &["exposure"], None),
                step(3, &["colorbalancergb"], None),
                step(4, &["agx"], None),
            ],
        ));
        let undo = |first, count, locale| {
            let history = history(3, steps.clone(), Some(moving(first, count)), None);
            words(Some(&history), locale).undo
        };
        assert_eq!(undo(1, 1, Locale::Uk).as_deref(), Some("Експозиція"));
        assert_eq!(undo(1, 1, Locale::Pl).as_deref(), Some("Ekspozycja"));
        assert_eq!(
            undo(2, 3, Locale::Uk).as_deref(),
            Some("Експозиція та ще 2 модулі")
        );
        assert_eq!(
            undo(2, 3, Locale::Pl).as_deref(),
            Some("Ekspozycja i jeszcze 2 moduły")
        );
    }

    /// 21 October 2026, 17:52 on the local clock, the wireframe's, in milliseconds since
    /// 1970 in UTC: the tooltips read the same whatever the machine's zone.
    fn wireframe_s_time() -> u64 {
        use chrono::TimeZone as _;
        let local = (chrono::Local.with_ymd_and_hms(2026, 10, 21, 17, 52, 0))
            .single()
            .expect("a time the local clock has");
        u64::try_from(local.timestamp_millis()).expect("after 1970")
    }

    fn number(control: &'static str, before: f32, after: f32) -> ControlChange {
        ControlChange {
            control,
            before: ControlValue::Number(before),
            after: ControlValue::Number(after),
            mark: [None, Some(Mark::Kept)],
        }
    }

    /// A step of `source` with `note`, its `changes` each a module's.
    fn made(
        id: u64,
        source: StepSource,
        note: Option<Note>,
        changes: Vec<ModuleChange>,
    ) -> Arc<HistoryStep> {
        Arc::new(HistoryStep {
            id: StepId(id),
            clock: Clock::at(wireframe_s_time(), None, MachineId::UNKNOWN),
            source,
            note,
            batch: None,
            changes,
        })
    }

    fn change(
        module: &'static str,
        enabled: Option<[bool; 2]>,
        controls: Vec<ControlChange>,
    ) -> ModuleChange {
        ModuleChange {
            module: module.into(),
            enabled,
            controls,
        }
    }

    /// A photo's history of five steps, one module each, the end at the third, so the last
    /// two are greyed, with the Default lines the engine lists: Exposure, Grain turned on,
    /// an Auto step of Exposure, a reset of both, and an assistant's step of Exposure with
    /// its note.
    fn photo_s(end: usize) -> History {
        let exposure =
            |before, after| change("exposure", None, vec![number("exposure", before, after)]);
        let steps = vec![
            made(0xa1, StepSource::You, None, vec![exposure(0.7, 1.2)]),
            made(
                0xa2,
                StepSource::You,
                None,
                vec![change(
                    "grain",
                    Some([false, true]),
                    vec![number("strength", 25.0, 30.0)],
                )],
            ),
            made(0xa3, StepSource::Auto, None, vec![exposure(1.2, 0.95)]),
            made(
                0xa4,
                StepSource::You,
                Some(Note::reset()),
                vec![
                    exposure(0.95, 0.7),
                    change(
                        "grain",
                        Some([true, false]),
                        vec![number("strength", 30.0, 25.0)],
                    ),
                ],
            ),
            made(
                0xa5,
                StepSource::Assistant,
                Some(Note::Text("Brighter shadows".to_owned())),
                vec![change(
                    "exposure",
                    None,
                    vec![
                        number("exposure", 0.7, 1.5),
                        number("black_level", 0.0, 0.01),
                    ],
                )],
            ),
        ];
        History {
            through: 12,
            steps,
            end,
            defaults: apertrust_engine::history::default_lines(
                apertrust_modules::MODULES,
                apertrust_operations::OPERATIONS,
            ),
            undo: None,
            redo: None,
        }
    }

    fn lines_in(history: Option<&History>, locale: Locale) -> Vec<Line> {
        lines(
            history,
            apertrust_modules::MODULES,
            &Translator::new(locale),
        )
    }

    #[test]
    fn the_lines_are_the_steps_newest_first_the_default_lines_then_original() {
        let history = photo_s(3);
        let lines = lines_in(Some(&history), Locale::EnGb);
        let shown: Vec<(Option<usize>, &str, Option<&str>)> = (lines.iter())
            .map(|line| (line.number(), line.name.as_str(), line.tag.as_deref()))
            .collect();
        assert_eq!(
            shown,
            [
                // A note leaves the line its module's name, as darktable's.
                (Some(5), "Exposure", Some("Assistant")),
                (Some(4), "Reset", Some("You")),
                (Some(3), "Exposure", Some("Auto")),
                (Some(2), "Grain", Some("You")),
                (Some(1), "Exposure", Some("You")),
                // The Default lines, the first darktable adds just over Original.
                (None, "AgX", Some("Default")),
                (None, "Tone curve", Some("Default")),
                (None, "Colour calibration", Some("Default")),
                (None, "Exposure", Some("Default")),
                (None, "Orientation", Some("Default")),
                (None, "Highlight reconstruction", Some("Default")),
                (None, "White balance", Some("Default")),
                (None, "Output colour profile", Some("Default")),
                (None, "Demosaic", Some("Default")),
                (None, "Raw black/white point", Some("Default")),
                (Some(0), "Original", None),
            ]
        );
        // The end at the third step: it is chosen, the two after it greyed, the Default
        // lines never either.
        let state: Vec<(bool, bool)> = (lines.iter())
            .map(|line| (line.end(), line.greyed()))
            .collect();
        assert_eq!(
            state[..6],
            [
                (false, true),
                (false, true),
                (true, false),
                (false, false),
                (false, false),
                (false, false)
            ]
        );
        assert!(state[5..].iter().all(|s| *s == (false, false)));
        // Each opens its first module the app knows; Original none.
        assert_eq!(lines[2].module, Some("exposure"));
        assert_eq!(lines[3].module, Some("grain"));
        assert_eq!(lines[5].module, Some("agx"));
        assert_eq!(lines[15].module, None);
        // At end 0 every step is greyed, the Default lines and Original none, and the top
        // Default line alone is chosen: the boundary between what is undone and what
        // always applies.
        let lines = lines_in(Some(&photo_s(0)), Locale::EnGb);
        let chosen: Vec<&str> = (lines.iter())
            .filter(|line| line.end())
            .map(|line| line.name.as_str())
            .collect();
        assert_eq!(chosen, ["AgX"]);
        assert!(lines[..5].iter().all(Line::greyed));
        assert!(!lines[5..].iter().any(Line::greyed));
        // With no Default line, Original is.
        let bare = History {
            defaults: Vec::new(),
            ..photo_s(0)
        };
        let lines = lines_in(Some(&bare), Locale::EnGb);
        assert!(lines.last().is_some_and(Line::end));
        // No history: Original alone.
        let alone = lines_in(None, Locale::EnGb);
        assert_eq!(alone.len(), 1);
        assert_eq!(
            alone[0].place,
            Place::Original {
                end: true,
                there: true
            }
        );
    }

    fn value(label: &str, before: &str, after: &str) -> TipRow {
        TipRow::Value {
            label: label.to_owned(),
            before: before.to_owned(),
            after: after.to_owned(),
        }
    }

    #[test]
    fn a_step_s_tooltip_has_a_row_a_control_then_who_and_when() {
        let history = photo_s(3);
        let tip = |locale, number: usize| {
            let lines = lines_in(Some(&history), locale);
            let line = (lines.into_iter())
                .find(|line| line.number() == Some(number) && line.tag.is_some())
                .expect("the step's line");
            line.tip
        };
        let when = || "21/10/2026, 17:52".to_owned();
        // One module: its rows with no heading, the line naming it.
        assert_eq!(
            tip(Locale::EnGb, 2),
            Tip::Changes {
                modules: vec![Changed {
                    heading: None,
                    rows: vec![value("Strength", "25.00%", "30.00%")],
                }],
                by: vec!["You".to_owned(), when()],
            }
        );
        // A note beside who and when, the line keeping its module's name.
        assert_eq!(
            tip(Locale::EnGb, 5),
            Tip::Changes {
                modules: vec![Changed {
                    heading: None,
                    rows: vec![
                        value("Exposure", "+0.700 EV", "+1.500 EV"),
                        value("Black level correction", "+0.0000", "+0.0100"),
                    ],
                }],
                by: vec![
                    "Brighter shadows".to_owned(),
                    "Assistant".to_owned(),
                    when()
                ],
            }
        );
        // Reset, the one step of several modules: each module's rows under its title, its
        // note its line's name.
        assert_eq!(
            tip(Locale::EnGb, 4),
            Tip::Changes {
                modules: vec![
                    Changed {
                        heading: Some("Exposure".to_owned()),
                        rows: vec![value("Exposure", "+0.950 EV", "+0.700 EV")],
                    },
                    Changed {
                        heading: Some("Grain".to_owned()),
                        rows: vec![value("Strength", "30.00%", "25.00%")],
                    },
                ],
                by: vec!["You".to_owned(), when()],
            }
        );
        // A Default line, and a step of source Default: who and when alone, the line
        // having no time.
        let lines = lines_in(Some(&history), Locale::EnGb);
        let default = (lines.iter()).find(|line| matches!(line.place, Place::Default { .. }));
        let default_tip = Tip::Changes {
            modules: Vec::new(),
            by: vec!["Default".to_owned()],
        };
        assert_eq!(default.map(|line| &line.tip), Some(&default_tip));
        let applied = made(
            12,
            StepSource::Default,
            None,
            vec![change("exposure", None, vec![number("exposure", 0.0, 0.7)])],
        );
        let tr = Translator::new(Locale::EnGb);
        assert_eq!(
            super::tip(&applied, apertrust_modules::MODULES, &tr),
            Tip::Changes {
                modules: Vec::new(),
                by: vec!["Default".to_owned(), when()],
            }
        );
        assert_eq!(default_tip.said(&tr), "Default");
        // For screen readers, plain text in the same order, in each language.
        let said = |locale, number| tip(locale, number).said(&Translator::new(locale));
        assert_eq!(
            said(Locale::EnGb, 1),
            "Exposure, +0.700 EV to +1.200 EV; You, 21/10/2026, 17:52"
        );
        assert_eq!(
            said(Locale::EnGb, 4),
            "Exposure; Exposure, +0.950 EV to +0.700 EV; Grain; Strength, 30.00% to 25.00%; \
             You, 21/10/2026, 17:52"
        );
        assert_eq!(
            said(Locale::EnGb, 5),
            "Exposure, +0.700 EV to +1.500 EV; Black level correction, +0.0000 to +0.0100; \
             Brighter shadows, Assistant, 21/10/2026, 17:52"
        );
        assert_eq!(
            said(Locale::EnUs, 1),
            "Exposure, +0.700 EV to +1.200 EV; You, 10/21/2026, 5:52\u{202f}PM"
        );
        assert_eq!(
            said(Locale::Uk, 1),
            "Експозиція, з +0,700 EV на +1,200 EV; Ви, 21.10.2026, 17:52"
        );
        assert_eq!(
            said(Locale::Pl, 2),
            "Siła, z 25,00% na 30,00%; Ty, 21.10.2026, 17:52"
        );
    }

    #[test]
    fn a_value_reads_as_its_row_writes_it_and_a_module_this_build_lacks_by_its_name() {
        let tr = Translator::new(Locale::EnGb);
        let exposure = definition(apertrust_modules::MODULES, "exposure");
        let words = |change: &ControlChange| control_change(exposure, change, &tr);
        let choice = ControlChange {
            control: "mode",
            before: ControlValue::Choice(1),
            after: ControlValue::Choice(0),
            mark: [None, None],
        };
        assert_eq!(words(&choice), value("Mode", "Automatic", "Manual"));
        let flag = ControlChange {
            control: "camera_bias",
            before: ControlValue::Bool(true),
            after: ControlValue::Bool(false),
            mark: [None, None],
        };
        assert_eq!(
            words(&flag),
            value("Compensate camera exposure", "On", "Off")
        );
        let mut kept = number("exposure", 0.7, 0.7);
        assert_eq!(words(&kept), TipRow::Said("Exposure kept".to_owned()));
        kept.mark = [Some(Mark::Kept), None];
        assert_eq!(words(&kept), TipRow::Said("Exposure released".to_owned()));
        // A parameter no module owns, by its name, with the decimals it needs.
        let black = ControlChange {
            control: "black",
            before: ControlValue::Number(0.0),
            after: ControlValue::Number(0.0125),
            mark: [None, None],
        };
        assert_eq!(
            control_change(None, &black, &tr),
            value("black", "0", "0.0125")
        );
        // A module a newer build wrote: its stored name, nothing to describe.
        let newer = made(
            9,
            StepSource::You,
            None,
            vec![change("lens_2030", None, Vec::new())],
        );
        let history = History {
            end: 1,
            ..photo_s(0)
        };
        let history = History {
            steps: vec![newer],
            defaults: Vec::new(),
            ..history
        };
        let lines = lines_in(Some(&history), Locale::EnGb);
        assert_eq!(lines[0].name, "lens_2030");
        assert_eq!(lines[0].module, None);
        assert_eq!(lines[0].tip.said(&tr), "You, 21/10/2026, 17:52");
        // Beside a known module, its title alone, with nothing after it.
        let both = made(
            10,
            StepSource::You,
            None,
            vec![
                change("exposure", None, vec![number("exposure", 0.7, 1.2)]),
                change("lens_2030", None, Vec::new()),
            ],
        );
        assert_eq!(
            tip(&both, apertrust_modules::MODULES, &tr).said(&tr),
            "Exposure; Exposure, +0.700 EV to +1.200 EV; lens_2030; You, 21/10/2026, 17:52"
        );
        // A step that names nothing reads its source, never an empty line.
        let bare = made(11, StepSource::Auto, None, Vec::new());
        assert_eq!(line_name(&bare, apertrust_modules::MODULES, &tr), "Auto");
    }

    /// The History section drawn from a history the test holds, as the side panel draws
    /// it, and what it reported.
    struct Scene {
        history: Option<History>,
        editable: bool,
        resettable: bool,
        actions: Vec<Action>,
    }

    fn scene(look: Theme, size: egui::Vec2, history: Option<History>) -> Harness<'static, Scene> {
        Harness::builder()
            .with_size(size)
            .with_context_setup(move |ctx| {
                apertrust_ui::install(ctx, look.clone());
                apertrust_ui::set_locale(ctx, Locale::EnGb);
            })
            .build_ui_state(
                |ui, scene: &mut Scene| {
                    // The side panel at its usual width at the window's end, the photo's
                    // surround before it, where a tooltip too wide for the panel goes.
                    let t = theme(ui.ctx());
                    let window = ui.max_rect();
                    let side = egui::Rect::from_min_max(
                        pos2(window.right() - t.sizes.side_panel, window.top()),
                        window.max,
                    );
                    ui.painter()
                        .rect_filled(window, 0.0, t.colors.photo_surround);
                    ui.painter().rect_filled(side, 0.0, t.colors.panel);
                    let (history, editable, resettable) =
                        (scene.history.as_ref(), scene.editable, scene.resettable);
                    let modules = apertrust_modules::MODULES;
                    let mut actions = Vec::new();
                    ui.scope_builder(UiBuilder::new().max_rect(side), |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        PanelBody::new().show(ui, |ui| {
                            actions =
                                section(ui, history, modules, true, false, editable, resettable);
                        });
                    });
                    scene.actions.extend(actions);
                },
                Scene {
                    history,
                    editable: true,
                    resettable: true,
                    actions: Vec::new(),
                },
            )
    }

    fn photo_scene(end: usize) -> Harness<'static, Scene> {
        scene(
            Theme::default(),
            egui::vec2(320.0, 520.0),
            Some(photo_s(end)),
        )
    }

    fn item<'a>(harness: &'a Harness<'_, Scene>, name: &'a str) -> egui_kittest::Node<'a> {
        harness.get_by_role_and_label(AccessRole::ListItem, name)
    }

    #[test]
    fn a_click_goes_to_a_step_and_shift_click_or_a_default_line_opens_its_module() {
        let mut harness = photo_scene(3);
        harness.run();
        item(&harness, "Step 1: Exposure").click();
        harness.run();
        item(&harness, "Step 5: Exposure").click();
        harness.run();
        item(&harness, "Original").click();
        harness.run();
        // The end where it is: nothing, as darktable's click on its active item.
        item(&harness, "Step 3: Exposure").click();
        harness.run();
        assert_eq!(
            harness.state().actions,
            [
                Action::GoTo(Some(StepId(0xa1))),
                Action::GoTo(Some(StepId(0xa5))),
                Action::GoTo(None),
            ]
        );
        harness.state_mut().actions.clear();
        // Shift-click opens the module, of a step, the end's too, and of a Default line;
        // Original has none.
        let shift_click = |harness: &mut Harness<'_, Scene>, name: &str| {
            item(harness, name).click_modifiers(egui::Modifiers::SHIFT);
            harness.run();
        };
        shift_click(&mut harness, "Step 2: Grain");
        shift_click(&mut harness, "Step 3: Exposure");
        shift_click(&mut harness, "Original");
        shift_click(&mut harness, "Raw black/white point");
        assert_eq!(
            harness.state().actions,
            [
                Action::Open("grain"),
                Action::Open("exposure"),
                Action::Open("rawprepare"),
            ]
        );
        // A plain click on a Default line goes to Original, as darktable's click on one of
        // its first items ends the history there, and opens nothing.
        harness.state_mut().actions.clear();
        item(&harness, "Raw black/white point").click();
        harness.run();
        assert_eq!(harness.state().actions, [Action::GoTo(None)]);
        // On a photo that takes no edit, a click goes nowhere; Shift-click still opens.
        harness.state_mut().actions.clear();
        harness.state_mut().editable = false;
        harness.run();
        item(&harness, "Step 1: Exposure").click();
        harness.run();
        item(&harness, "Demosaic").click();
        harness.run();
        shift_click(&mut harness, "Demosaic");
        assert_eq!(harness.state().actions, [Action::Open("demosaic")]);
    }

    /// A line's hover band says it takes a click or a Shift-click: a Default line always,
    /// its Shift-click opening its module; Original only while a click on it goes
    /// somewhere, not at end 0 nor on a photo that takes no edit.
    #[test]
    fn the_hover_band_marks_a_line_that_takes_a_click_or_a_shift_click() {
        let lines = |end| lines_in(Some(&photo_s(end)), Locale::EnGb);
        let named = |lines: &[Line], name: &str| {
            (lines.iter().find(|line| line.name == name).cloned()).expect(name)
        };
        let at_0 = lines(0);
        assert!(!named(&at_0, "Original").takes(true));
        assert!(named(&at_0, "Demosaic").takes(true));
        assert!(named(&at_0, "Demosaic").takes(false));
        let at_3 = lines(3);
        assert!(named(&at_3, "Original").takes(true));
        assert!(!named(&at_3, "Original").takes(false));
        assert!(named(&at_3, "Demosaic").takes(false));
    }

    /// At end 0, a click on any of the Default lines or Original changes nothing: the end
    /// is there already, the top Default line chosen.
    #[test]
    fn at_end_0_a_click_below_the_steps_goes_nowhere() {
        let mut harness = photo_scene(0);
        harness.run();
        assert_eq!(
            item(&harness, "AgX").accesskit_node().is_selected(),
            Some(true)
        );
        assert_eq!(
            item(&harness, "Original").accesskit_node().is_selected(),
            Some(false)
        );
        for name in ["AgX", "Demosaic", "Original"] {
            item(&harness, name).click();
            harness.run();
        }
        assert!(harness.state().actions.is_empty());
        item(&harness, "Step 1: Exposure").click();
        harness.run();
        assert_eq!(harness.state().actions, [Action::GoTo(Some(StepId(0xa1)))]);
    }

    #[test]
    fn reset_in_the_header_runs_while_the_edit_holds_anything() {
        let mut harness = photo_scene(3);
        harness.run();
        harness
            .get_by_role_and_label(AccessRole::Button, "Reset edit")
            .click();
        harness.run();
        assert_eq!(harness.state().actions, [Action::Reset]);
        harness.state_mut().actions.clear();
        harness.state_mut().resettable = false;
        harness.run();
        let reset = harness.get_by_role_and_label(AccessRole::Button, "Reset edit");
        assert!(reset.accesskit_node().is_disabled());
        reset.click();
        harness.run();
        assert!(harness.state().actions.is_empty());
        // The header opens and closes the section, as every section's.
        harness
            .get_by_role_and_label(AccessRole::Button, "History")
            .click();
        harness.run();
        assert_eq!(harness.state().actions, [Action::Toggle]);
    }

    #[test]
    fn the_arrows_move_between_lines_and_enter_goes_to_the_step() {
        let mut harness = photo_scene(3);
        harness.run();
        item(&harness, "Step 3: Exposure").focus();
        harness.run();
        harness.key_press(egui::Key::ArrowDown);
        harness.run();
        assert!(
            item(&harness, "Step 2: Grain")
                .accesskit_node()
                .is_focused()
        );
        harness.key_press(egui::Key::ArrowDown);
        harness.run();
        harness.key_press(egui::Key::ArrowUp);
        harness.run();
        harness.key_press(egui::Key::ArrowUp);
        harness.run();
        assert!(
            item(&harness, "Step 3: Exposure")
                .accesskit_node()
                .is_focused()
        );
        harness.key_press(egui::Key::ArrowUp);
        harness.run();
        harness.key_press(egui::Key::Enter);
        harness.run();
        assert_eq!(harness.state().actions, [Action::GoTo(Some(StepId(0xa4)))]);
    }

    #[test]
    fn screen_readers_hear_each_line_s_name_tooltip_and_the_end() {
        let mut harness = photo_scene(3);
        harness.run();
        let list = harness.get_by_role_and_label(AccessRole::List, "History");
        assert_eq!(list.children().count(), 16);
        let end = item(&harness, "Step 3: Exposure");
        assert_eq!(end.accesskit_node().is_selected(), Some(true));
        let first = item(&harness, "Step 1: Exposure");
        assert_eq!(first.accesskit_node().is_selected(), Some(false));
        assert_eq!(
            first.accesskit_node().description().as_deref(),
            Some("Exposure, +0.700 EV to +1.200 EV; You, 21/10/2026, 17:52")
        );
        // A step after the end says first that it does not apply.
        let later = item(&harness, "Step 5: Exposure");
        let said = later.accesskit_node().description().unwrap_or_default();
        assert!(said.starts_with("Not applied\n"), "{said}");
        let default = item(&harness, "Exposure");
        assert_eq!(
            default.accesskit_node().description().as_deref(),
            Some("Default")
        );
    }

    #[test]
    fn with_no_history_original_alone_is_drawn_disabled() {
        let mut harness = scene(Theme::default(), egui::vec2(320.0, 120.0), None);
        harness.state_mut().resettable = false;
        harness.run();
        let original = item(&harness, "Original");
        assert!(original.accesskit_node().is_disabled());
        original.click();
        harness.run();
        assert!(harness.state().actions.is_empty());
        assert_eq!(
            harness
                .get_by_role_and_label(AccessRole::List, "History")
                .children()
                .count(),
            1
        );
    }

    /// The lines are worded once for a history and a language, and again when either
    /// changes: the end moved, a step described anew, another language.
    #[test]
    fn the_lines_are_worded_once_for_each_history_and_language() {
        let ctx = egui::Context::default();
        let modules = apertrust_modules::MODULES;
        let (en, uk) = (Translator::new(Locale::EnGb), Translator::new(Locale::Uk));
        let history = photo_s(3);
        let kept = kept_lines(&ctx, Some(&history), modules, &en);
        assert!(Arc::ptr_eq(
            &kept,
            &kept_lines(&ctx, Some(&history), modules, &en)
        ));
        let moved = History {
            end: 2,
            ..history.clone()
        };
        let again = kept_lines(&ctx, Some(&moved), modules, &en);
        assert!(!Arc::ptr_eq(&kept, &again));
        assert!(again[3].end());
        let mut anew = moved.clone();
        anew.steps[0] = Arc::new((*anew.steps[0]).clone());
        assert!(!Arc::ptr_eq(
            &again,
            &kept_lines(&ctx, Some(&anew), modules, &en)
        ));
        let ukrainian = kept_lines(&ctx, Some(&anew), modules, &uk);
        assert_eq!(ukrainian[4].name, "Експозиція");
    }

    /// The section as the wireframe's History frame draws it, the end mid-list and the
    /// fifth step's tooltip shown, its rows' columns lined up and its note beside who and when, in the dark and light themes.
    #[test]
    fn the_history_section_is_photographed() {
        let mut results = egui_kittest::SnapshotResults::new();
        for (which, name) in [
            (Builtin::Dark, "history-tab-dark"),
            (Builtin::Light, "history-tab-light"),
        ] {
            let mut harness = scene(
                Theme::builtin(which),
                egui::vec2(640.0, 460.0),
                Some(photo_s(3)),
            );
            harness.run();
            item(&harness, "Step 5: Exposure").hover();
            harness.run();
            harness.snapshot(name);
            results.extend(harness.take_snapshot_results());
        }
    }
}
