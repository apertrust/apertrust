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
//! decision 78): the History tab, and what Undo and Redo would do, named as History
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
//!   darktable's history item reads, even with a note, Reset's reading Reset, then what
//!   changed in the small text, as the board "Photo · History tab: one line says what
//!   changed" draws it, and its source's tag at its end only where it is not the
//!   person's own (Auto, Style, Assistant); the end's line chosen, on `list_chosen`, as
//!   darktable's active item, and the steps after it in the secondary text, which
//!   darktable draws as any other (`docs/history.md`, Against darktable), the undone mark
//!   under them;
//! - a batch, one change's steps on several modules, one closed line, a group: its note
//!   or how many modules, its steps' numbers as a range, its modules' titles, its source;
//!   the chevron, or →, opens it, its steps indented under it, and it opens itself when
//!   the end lands inside it; a click on it goes to its last step, as Undo moves it whole.
//!   Closed with the end inside it, it is the end's line, the undone mark under it.
//!   Which groups are open the app keeps while it runs, never saved;
//! - a heading over each day's steps, from their clocks: Today, Yesterday with its date,
//!   then the date, in the interface's language; No date over steps of unknown time under
//!   a day's;
//! - the rail down the lines' start (`Rail`): a dot a step, a square a group, Reset's
//!   glyph, the end a larger filled dot, or square, the undone steps hollow on a dashed
//!   rail;
//! - then a new photo's default steps, 1 to n, one closed group, Defaults, which opens as
//!   a batch does, its steps numbered and with no tag, and Original, 0, chosen at end 0;
//! - a step's tooltip: a small table, a row for each control it changed, its label, its
//!   value before, an arrow and its value after as its row writes them, in lined-up
//!   columns, then its note, who made it and when apart, on the local clock in the
//!   interface's language; a group's, its steps' rows under each module's title; a
//!   default step's who and when alone;
//! - a click on a step goes to it ([`Action::GoTo`]), a default step among them, on a
//!   group's line, the Defaults' among them, to its last step, and on Original to
//!   Original, but on where the end is, which changes nothing, as darktable's click on its
//!   active item; Shift-click on a step opens its module's section ([`Action::Open`]), as
//!   darktable's Shift-click focuses it;
//! - the keys, a tree's: ↑ and ↓ move between the lines, → opens a group, or goes into
//!   one open, ← closes it, or goes from a line inside it to it, Home to the newest line
//!   and End to Original, a key with nothing to do keeping the focus; Enter does what a
//!   click does, Shift+Enter what Shift-click does. To screen readers a tree of items,
//!   each named by its number and name, a group by its numbers, name and modules, its
//!   tooltip its description, the end selected, each saying where it is among its level's,
//!   a group expanded or not, offering Expand or Collapse, its Click going to its last
//!   step as a click does, the lines inside one a level down in a group of their own; the
//!   day headings named, never focused;
//! - Reset in the header, which resets the edit as one step, as Reset Edit does; the
//!   header stays at the top of the tab while the list scrolls under it, and the end is
//!   scrolled into sight whenever it moves;
//! - its options, ⋯ in the header, through the popup service: Compress History, darktable's
//!   compress history stack, which Option (Alt on Windows and Linux) held while the menu is
//!   open turns into Truncate History, darktable's Ctrl-click on it, where a step lies
//!   after the end for it to drop; Create a Style…, drawn disabled until styles come; and
//!   Discard History…, which asks first, as Edit's does ([`options`]);
//! - the eye at the end of a step's line of one module, under the pointer or with the
//!   focus: held, by the pointer or Space or Enter, a peek at the photo without the module
//!   ([`Action::Peek`], `Command::Hide` while held), never saved, the module's name struck
//!   through while the session reports it hidden; nothing stays once it is let go.
//!

use std::collections::HashSet;
use std::sync::Arc;

use apertrust_engine::session::{
    BatchId, ControlChange, ControlValue, History, HistoryStep, Mark, Move, Note, StepId,
    StepSource,
};
use apertrust_modules::{ControlKind, Definition, Module, Row, Slider, messages};
use apertrust_ui::components::{
    IconButton, PanelBody, Rail, RailLine, RailNode, Section, SectionAction, focus_ring_centred,
    section_body,
};
use apertrust_ui::i18n::{Args, local_day};
use apertrust_ui::popup::{Entry as MenuEntry, Item};
use apertrust_ui::{Icon, Translator, fonts, theme, translator};
use eframe::egui::{self, FontId, Key, Modifiers, Role, Sense, UiBuilder, WidgetInfo, pos2, vec2};

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
    /// The eye held this frame on this module's line: the photo shown without the module
    /// while it is held, `Command::Hide`.
    Peek(&'static str),
    /// Compress History, from its options: `Command::Compress`.
    Compress,
    /// Truncate History, Compress History's alternate: `Command::Truncate`.
    Truncate,
    /// Discard History…, from its options: run as Edit's, which asks first.
    Discard,
}

/// What the section's options can do now ([`options`]), each item drawn disabled where it
/// cannot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub compress: bool,
    /// Whether Compress History has Truncate History as its alternate.
    pub truncate: bool,
    pub discard: bool,
    /// Whether the edit is open for viewing only because part of its history cannot be
    /// read, which Discard History starts afresh, its tip saying so.
    pub unreadable: bool,
    /// Whether Discard History cannot run only because it would ask and the platform shows
    /// no alert, its tip saying so; the app's to set.
    pub no_alert: bool,
}

/// What the section can do for the photo: go to a step while `editable`, live and not
/// open for viewing only; Reset while `resettable`; and its `options`. `photo` is the
/// number of the photo's open, so a menu opened for one photo chooses nothing for the
/// next.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub editable: bool,
    pub resettable: bool,
    pub options: Options,
    pub photo: Option<u64>,
}

/// Why Discard History… cannot run, where only the platform's missing alert stops it
/// (`options.no_alert`): with the question on, turning it off lets it run; on an
/// unreadable edit, which always asks, nothing does. None where it can run, or cannot for
/// another reason.
pub fn discard_why(options: Options) -> Option<&'static str> {
    match (options.no_alert, options.unreadable) {
        (false, _) => None,
        (true, false) => Some("history-discard-no-alert"),
        (true, true) => Some("history-discard-no-alert-unreadable"),
    }
}

/// What Compress History, Truncate History and Discard History can do on `history`, the
/// open photo's, where it `takes` edits, live and not open for viewing only, or is open
/// for viewing only because part of its history cannot be read, `unreadable`, which
/// Discard History alone is taken on, starting it afresh (decision 78). Each is offered
/// only where the history shows it has something to do, from what `History` tells:
///
/// - Compress History where a step is not a default one, since Compress keeps every default
///   step, and at end 0, where it drops every step, the photo is given them again: a
///   history of default steps alone is compressed already. It may still find nothing to
///   do, each module's step up to the end its last and none empty, which `History` does
///   not tell; the session then records nothing.
/// - Truncate History, its alternate, where a step after the end is not a default one,
///   which it would drop; otherwise Option shows Compress History still.
/// - Discard History where the end lies before a step, or a step is not a default one: a
///   history of default steps alone with the end after them is a new photo's, which it
///   leaves as it is. Always on an `unreadable` edit.
pub fn options(history: Option<&History>, takes: bool, unreadable: bool) -> Options {
    let Some(history) = history else {
        return Options {
            discard: unreadable,
            unreadable,
            ..Options::default()
        };
    };
    let own = |step: &Arc<HistoryStep>| step.source != StepSource::Default;
    let end = history.end.min(history.steps.len());
    let compress = takes && history.steps.iter().any(own);
    Options {
        compress,
        truncate: takes && history.steps[end..].iter().any(own),
        discard: unreadable || (takes && (compress || end < history.steps.len())),
        unreadable,
        no_alert: false,
    }
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
    /// A batch's line, its group's, or the Defaults', the default steps': its steps
    /// numbered `first` to `last`, the last's id `last_id`, which a click goes to; `applied`
    /// while its first is at or under the end, and `end` where the end is its last, which
    /// it shows while closed.
    Batch {
        last_id: StepId,
        first: usize,
        last: usize,
        applied: bool,
        end: bool,
    },
    /// Original, 0: `there` and `end` at end 0.
    Original { end: bool, there: bool },
}

/// A group of lines shown as one that opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GroupKey {
    /// A batch's steps.
    Batch(BatchId),
    /// A new photo's default steps.
    Defaults,
}

/// What a line is, which keeps its widget's id, and so its focus, whatever opens or
/// closes over it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LineKey {
    Step(StepId),
    Group(GroupKey),
    Original,
}

/// One line of the list, in the interface's words.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub place: Place,
    pub key: LineKey,
    pub name: String,
    /// What changed, after the name in the small text, cut at its end where the line has no
    /// room; none where there is nothing to say, as on a default step. A group's, its
    /// modules or how many.
    pub change: Option<String>,
    /// What changed as screen readers hear it in the step's name, its values before and
    /// after, since they have no tooltip to rest on.
    pub heard: Option<String>,
    /// Its source's tag at its end, where it is not the person's own: Auto, Style or
    /// Assistant.
    pub tag: Option<String>,
    pub tip: Tip,
    /// Its node on the rail; a group's is drawn the end's only while it is closed.
    pub node: RailNode,
    /// The module Shift-click opens; none where the app
    /// does not know it, as an operation's unowned parameters or a newer build's module.
    pub module: Option<&'static str>,
    /// The module the eye at its end hides from the view: a step's one module the app
    /// knows; none on Reset's, a group's line or Original, nor where the app does not know
    /// it.
    pub hides: Option<&'static str>,
}

/// What the list holds, newest first, worded once for a history ([`kept`]).
#[derive(Clone, Debug, PartialEq)]
pub enum Entry {
    /// A day's heading, over its steps.
    Day(String),
    Line(Line),
    /// A group: its line, and the lines it holds, newest first. `holds_end` where the end
    /// is inside it, under its last step, where it opens itself.
    Group {
        key: GroupKey,
        line: Line,
        lines: Vec<Line>,
        holds_end: bool,
    },
}

/// A history's list as the section draws it.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Listed {
    pub entries: Vec<Entry>,
    /// How many steps are undone.
    pub undone: usize,
}

impl Listed {
    /// Every line, a group's and those it holds after it, in the list's order.
    pub fn lines(&self) -> impl Iterator<Item = &Line> {
        self.entries.iter().flat_map(|entry| {
            let (line, inside): (Option<&Line>, &[Line]) = match entry {
                Entry::Day(_) => (None, &[]),
                Entry::Line(line) => (Some(line), &[]),
                Entry::Group { line, lines, .. } => (Some(line), lines),
            };
            line.into_iter().chain(inside)
        })
    }
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
    /// Its number: a step's, and Original's 0; a group's line has none.
    fn number(&self) -> Option<usize> {
        match self.place {
            Place::Step { number, .. } => Some(number),
            Place::Original { .. } => Some(0),
            Place::Batch { .. } => None,
        }
    }

    /// Its numbers as its line writes them: a step's, Original's 0, a batch's first and
    /// last with an en dash between, "12–16".
    fn numbers(&self, tr: &Translator) -> Option<String> {
        let number = |n: usize| tr.format_number(n as f64, 0, false);
        match self.place {
            Place::Batch { first, last, .. } => Some(range(&number(first), &number(last))),
            _ => self.number().map(number),
        }
    }

    /// Whether it is the end's line: Original at end 0; a group's line where the end is its
    /// last. A group's line is drawn chosen only while it is closed.
    fn end(&self) -> bool {
        matches!(
            self.place,
            Place::Step { end: true, .. }
                | Place::Batch { end: true, .. }
                | Place::Original { end: true, .. }
        )
    }

    /// Whether it is a group's line, which opens.
    fn group(&self) -> bool {
        matches!(self.place, Place::Batch { .. })
    }

    /// Whether it takes a click or a Shift-click, which its hover band says: a click going
    /// somewhere while the photo takes edits (`editable`), or a Shift-click opening its
    /// module, whatever the photo. A group's line always takes one,
    /// on its chevron. Original, the end already there, takes neither.
    fn takes(&self, editable: bool) -> bool {
        (editable && !self.there()) || self.module.is_some() || self.group()
    }

    /// Whether a click goes nowhere: on the end's line, or on Original while the end is
    /// there.
    fn there(&self) -> bool {
        match self.place {
            Place::Step { end, .. } | Place::Batch { end, .. } => end,
            Place::Original { there, .. } => there,
        }
    }

    /// Whether it is a step after the end, greyed; a batch whose every step is.
    fn greyed(&self) -> bool {
        matches!(
            self.place,
            Place::Step { applied: false, .. } | Place::Batch { applied: false, .. }
        )
    }

    /// What a click on it asks for, Shift held or not, where the photo takes edits
    /// (`editable`).
    fn clicked(&self, shift: bool, editable: bool) -> Option<Action> {
        match self.place {
            Place::Step { .. } if shift => self.module.map(Action::Open),
            _ if shift => None,
            // The end where it is changes nothing, as darktable's click on its active item.
            _ if self.there() || !editable => None,
            Place::Step { id, .. } => Some(Action::GoTo(Some(id))),
            // A group's line goes to its last step, as Undo and Redo move a batch whole.
            Place::Batch { last_id, .. } => Some(Action::GoTo(Some(last_id))),
            Place::Original { .. } => Some(Action::GoTo(None)),
        }
    }
}

/// A range of numbers, its first and last with an en dash between.
fn range(first: &str, last: &str) -> String {
    format!("{first}\u{2013}{last}")
}

/// The list from `history`, the open photo's, in `tr`'s words: its steps newest first, a
/// batch's as a group, with a heading over each day's, then the Defaults group, a new
/// photo's default steps, also newest first, then Original; Original alone with no
/// history. `today` is the
/// local calendar's day now ([`local_day`]), which the headings count back from. `modules`
/// are the session's, whose titles, controls and rows word the lines.
pub fn listed(
    history: Option<&History>,
    modules: &[&dyn Module],
    tr: &Translator,
    today: i32,
) -> Listed {
    let mut entries = Vec::new();
    let end = history.map_or(0, |h| h.end);
    let mut day = None;
    if let Some(history) = history {
        let steps = &history.steps;
        // The default steps, which the log begins with, are the Defaults group.
        let defaults = (steps.iter())
            .take_while(|step| step.source == StepSource::Default)
            .count();
        // Newest first: the steps from `first` to `last`, one change's, each pass.
        let mut next = steps.len();
        while next > defaults {
            let last = next - 1;
            let mut first = last;
            if let Some(batch) = steps[last].batch {
                while first > defaults && steps[first - 1].batch == Some(batch) {
                    first -= 1;
                }
            }
            next = first;
            let newest = &steps[last];
            let on = (newest.clock.when > 0)
                .then(|| local_day(newest.clock.when))
                .flatten();
            // A step of unknown time, under a day's steps, has a heading of its own, No
            // date, never the newer day's; at the top it has none.
            if on != day {
                match on {
                    Some(on) => entries.push(Entry::Day(heading(on, today, tr))),
                    None if !entries.is_empty() => {
                        entries.push(Entry::Day(tr.text("history-day-unknown")));
                    }
                    None => {}
                }
                day = on;
            }
            let lines = (first..=last)
                .rev()
                .map(|index| step_line(&steps[index], index + 1, end, modules, tr));
            match steps[last].batch {
                Some(batch) if first < last => entries.push(Entry::Group {
                    key: GroupKey::Batch(batch),
                    line: batch_line(&steps[first..=last], first + 1, end, modules, tr),
                    // The source is the group's, on its line alone.
                    lines: lines.map(|line| Line { tag: None, ..line }).collect(),
                    holds_end: (first + 1..last + 1).contains(&end),
                }),
                _ => entries.extend(lines.map(Entry::Line)),
            }
        }
        if defaults > 0 {
            entries.push(defaults_group(&steps[..defaults], end, modules, tr));
        }
    }
    let original = end == 0;
    entries.push(Entry::Line(Line {
        place: Place::Original {
            end: original,
            there: end == 0,
        },
        key: LineKey::Original,
        name: tr.text("history-original"),
        change: None,
        heard: None,
        tag: None,
        tip: Tip::Text(tr.text("history-original-tip")),
        node: if original {
            RailNode::End
        } else {
            RailNode::Step
        },
        module: None,
        hides: None,
    }));
    Listed {
        entries,
        undone: history.map_or(0, |h| h.steps.len().saturating_sub(h.end)),
    }
}

/// A step's line, numbered `number`, with the end at `end`.
fn step_line(
    step: &HistoryStep,
    number: usize,
    end: usize,
    modules: &[&dyn Module],
    tr: &Translator,
) -> Line {
    Line {
        place: Place::Step {
            id: step.id,
            number,
            applied: number <= end,
            end: number == end,
        },
        key: LineKey::Step(step.id),
        name: line_name(step, modules, tr),
        change: summary(step, modules, tr),
        heard: heard(step, modules, tr),
        tag: tag(step.source, tr),
        tip: tip(step, modules, tr),
        node: match () {
            () if is_reset(step) => RailNode::Reset { end: number == end },
            () if number == end => RailNode::End,
            () if number > end => RailNode::Undone,
            () => RailNode::Step,
        },
        module: (step.changes.iter()).find_map(|change| known(modules, &change.module)),
        hides: match &step.changes[..] {
            [change] if !is_reset(step) => known(modules, &change.module),
            _ => None,
        },
    }
}

/// A batch's line, its `steps` numbered from `first`, with the end at `end`: its note,
/// which its last step carries, else how many modules; its modules' titles in its steps'
/// order; the last step's source; a tooltip with each step's rows under its module's
/// title, then its note, who and when.
fn batch_line(
    steps: &[Arc<HistoryStep>],
    first: usize,
    end: usize,
    modules: &[&dyn Module],
    tr: &Translator,
) -> Line {
    let newest = &steps[steps.len() - 1];
    let last = first + steps.len() - 1;
    let name = match &newest.note {
        Some(note) => noted(note, tr),
        None => {
            let mut args = Args::new();
            args.set("count", steps.len());
            tr.text_with("history-group-modules", Some(&args))
        }
    };
    let titles: Vec<String> = (steps.iter())
        .filter_map(|step| step.changes.first())
        .map(|change| title(modules, &change.module, tr))
        .collect();
    let mut changed = Vec::new();
    for step in steps {
        for change in &step.changes {
            let definition = definition(modules, &change.module);
            changed.push(Changed {
                heading: Some(title(modules, &change.module, tr)),
                rows: (change.controls.iter())
                    .map(|control| control_change(definition, control, tr))
                    .collect(),
            });
        }
    }
    let Tip::Changes { by, .. } = tip(newest, modules, tr) else {
        unreachable!("a step's tooltip has its changes")
    };
    let applied = first <= end;
    Line {
        place: Place::Batch {
            last_id: newest.id,
            first,
            last,
            applied,
            end: last == end,
        },
        key: LineKey::Group(GroupKey::Batch(newest.batch.unwrap_or(BatchId(0)))),
        name,
        change: Some(titles.join(", ")),
        heard: None,
        tag: tag(newest.source, tr),
        tip: Tip::Changes {
            modules: changed,
            by,
        },
        node: RailNode::Group {
            end: last == end,
            undone: !applied,
        },
        module: None,
        hides: None,
    }
}

/// The Defaults group, a new photo's default `steps`, numbered from 1, with the end at
/// `end`: its line, "Defaults", how many modules, its numbers' range, which a click goes to
/// the last of as a batch's does, and the default steps' lines, newest first, as the steps:
/// the first darktable adds, Raw black/white point, just over Original.
fn defaults_group(
    steps: &[Arc<HistoryStep>],
    end: usize,
    modules: &[&dyn Module],
    tr: &Translator,
) -> Entry {
    let lines = (1..=steps.len())
        .rev()
        .map(|number| step_line(&steps[number - 1], number, end, modules, tr))
        .collect();
    let newest = &steps[steps.len() - 1];
    let last = steps.len();
    let Tip::Changes { by, .. } = tip(newest, modules, tr) else {
        unreachable!("a step's tooltip has its changes")
    };
    let mut args = Args::new();
    args.set("count", steps.len());
    let applied = end >= 1;
    Entry::Group {
        key: GroupKey::Defaults,
        line: Line {
            place: Place::Batch {
                last_id: newest.id,
                first: 1,
                last,
                applied,
                end: last == end,
            },
            key: LineKey::Group(GroupKey::Defaults),
            name: tr.text("history-defaults"),
            change: Some(tr.text_with("history-defaults-modules", Some(&args))),
            heard: None,
            tag: None,
            tip: Tip::Changes {
                modules: Vec::new(),
                by,
            },
            node: RailNode::Group {
                end: last == end,
                undone: !applied,
            },
            module: None,
            hides: None,
        },
        lines,
        holds_end: (1..last).contains(&end),
    }
}

/// A day's heading, `day` counted as [`local_day`] counts it: Today, Yesterday with its
/// date, else its date alone.
fn heading(day: i32, today: i32, tr: &Translator) -> String {
    match today - day {
        0 => tr.text("history-day-today"),
        1 => {
            let mut args = Args::new();
            args.set("date", tr.format_day(day, today));
            tr.text_with("history-day-yesterday", Some(&args))
        }
        _ => tr.format_day(day, today),
    }
}

/// The local calendar's day now, as [`local_day`] counts it, which the day headings count
/// back from.
pub fn today() -> i32 {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
    let now = now.map_or(0, |since| since.as_millis() as u64);
    local_day(now).unwrap_or_default()
}

/// A row of the list as drawn this frame, groups open or closed.
#[derive(Clone, Copy, Debug)]
enum Drawn<'a> {
    Day(&'a str),
    /// The space over the Defaults' line.
    Gap,
    Line(Shown<'a>),
    /// The undone mark, under the last undone line shown.
    Undone,
}

/// A line as drawn this frame.
#[derive(Clone, Copy, Debug)]
struct Shown<'a> {
    line: &'a Line,
    /// 0 at the top, 1 inside a group, as AccessKit counts, from 0.
    level: usize,
    /// A group's line: whether it is open.
    open: Option<bool>,
    /// A line inside a group: the group's.
    within: Option<GroupKey>,
    /// Drawn the end's, on `list_chosen`: the end's line, or a closed group's holding
    /// the end.
    chosen: bool,
    /// A closed group's line holding the end, some of its steps undone and out of sight.
    holds: bool,
    /// Where it is among the lines of its level, its group's inside one, from 0, and how
    /// many they are, as screen readers say "3 of 9".
    position: usize,
    set: usize,
    rail: Rail,
}

/// The rows `listed` draws with the groups in `open` open: a group's lines under its own
/// while open, a gap over the Defaults' line, the undone mark under the last undone line,
/// or under a closed group's holding the end, whose undone steps are out of sight, and the
/// rail through them all, dashed under an undone line and through the rows between.
fn drawn<'a>(listed: &'a Listed, open: &HashSet<GroupKey>) -> Vec<Drawn<'a>> {
    let mut rows = Vec::new();
    let shown = |line: &'a Line, level, open: Option<bool>, within, holds_end: bool| {
        let holds = holds_end && open == Some(false);
        Shown {
            line,
            level,
            open,
            within,
            chosen: (line.end() && open != Some(true)) || holds,
            holds,
            position: 0,
            set: 0,
            rail: Rail {
                node: match line.node {
                    RailNode::Group { undone, .. } => RailNode::Group {
                        end: holds || (line.end() && open != Some(true)),
                        undone,
                    },
                    node => node,
                },
                above: RailLine::None,
                below: RailLine::None,
            },
        }
    };
    for entry in &listed.entries {
        match entry {
            Entry::Day(day) => rows.push(Drawn::Day(day)),
            Entry::Line(line) => rows.push(Drawn::Line(shown(line, 0, None, None, false))),
            Entry::Group {
                key,
                line,
                lines,
                holds_end,
            } => {
                let is_open = open.contains(key);
                if *key == GroupKey::Defaults && !rows.is_empty() {
                    rows.push(Drawn::Gap);
                }
                rows.push(Drawn::Line(shown(line, 0, Some(is_open), None, *holds_end)));
                if is_open {
                    let inside = (lines.iter()).map(|line| shown(line, 1, None, Some(*key), false));
                    rows.extend(inside.map(Drawn::Line));
                }
            }
        }
    }
    // Each line's place among its level's, a group's lines among their group's.
    let mut sets: Vec<(usize, Option<GroupKey>, Vec<usize>)> = Vec::new();
    for (at, row) in rows.iter().enumerate() {
        if let Drawn::Line(shown) = row {
            let set = (sets.iter_mut())
                .find(|(level, within, _)| *level == shown.level && *within == shown.within);
            match set {
                Some((_, _, members)) => members.push(at),
                None => sets.push((shown.level, shown.within, vec![at])),
            }
        }
    }
    for (_, _, members) in &sets {
        for (position, &at) in members.iter().enumerate() {
            if let Drawn::Line(shown) = &mut rows[at] {
                shown.position = position;
                shown.set = members.len();
            }
        }
    }
    let last_undone = rows
        .iter()
        .rposition(|row| matches!(row, Drawn::Line(shown) if shown.line.greyed() || shown.holds));
    if let Some(at) = last_undone.filter(|_| listed.undone > 0) {
        rows.insert(at + 1, Drawn::Undone);
    }
    // The rail between the lines: dashed under an undone line, solid else, and through
    // the rows between two lines as under the one above.
    let under = |line: &Line| {
        if line.greyed() {
            RailLine::Dashed
        } else {
            RailLine::Solid
        }
    };
    let lines: Vec<usize> = (rows.iter().enumerate())
        .filter_map(|(at, row)| matches!(row, Drawn::Line(_)).then_some(at))
        .collect();
    let mut above = RailLine::None;
    for (at, row) in rows.iter_mut().enumerate() {
        if let Drawn::Line(shown) = row {
            shown.rail.above = above;
            if lines.last().is_some_and(|&last| last > at) {
                shown.rail.below = under(shown.line);
            }
            above = shown.rail.below;
        }
    }
    rows
}

/// The rail through a row between lines, `rows[at]`: as under the line above it, none
/// where no line is above or below it.
fn through(rows: &[Drawn<'_>], at: usize) -> RailLine {
    let rail = |row: &Drawn<'_>| match row {
        Drawn::Line(shown) => Some(shown.rail.below),
        _ => None,
    };
    let below = rows[at + 1..].iter().any(|row| rail(row).is_some());
    match rows[..at].iter().rev().find_map(rail) {
        Some(above) if below => above,
        _ => RailLine::None,
    }
}

/// Draws the History tab: the History section, open or closed as `open` says, its header
/// staying at the top of the tab while its list scrolls under it, the list from
/// `history`, the open photo's, drawn disabled where there is none; it peeks while `peek`
/// holds, as every section does. What it can do is `state`'s: a step is gone to only
/// while it is editable (a photo live and not open for viewing only), Reset runs only
/// while it is resettable, and each of the options only where it can; ⋯ opens them while
/// there is a history. The eye, view state alone, is held on any photo, `hidden` the
/// modules the session reports hidden, as the mirror's values have them. `today` is the
/// local calendar's day ([`today`]). Returns what the user did.
#[expect(
    clippy::too_many_arguments,
    reason = "the section's state from the app, each part apart"
)]
pub fn tab(
    ui: &mut egui::Ui,
    history: Option<&History>,
    modules: &[&dyn Module],
    hidden: &[&str],
    open: bool,
    peek: bool,
    state: State,
    today: i32,
) -> Vec<Action> {
    let words = translator(ui.ctx());
    let title = words.text("section-history");
    let reset = words.text("history-reset-edit");
    let mut header = None;
    let mut actions = Vec::new();
    let editable = state.editable;
    // A utility: Reset before its options.
    PanelBody::new()
        .header(|ui| {
            let os = ui.ctx().os();
            let section = Section::new(&title, open)
                .utility_reset(&reset)
                .reset(state.resettable)
                .options(history.is_some())
                .options_menu(menu(&words, state.options, os))
                .peek(peek);
            header = Some(section.show_header(ui));
        })
        .show(ui, |ui| {
            if open {
                let lines = |ui: &mut egui::Ui| list(ui, history, modules, hidden, editable, today);
                actions = section_body(ui, lines);
            }
        });
    // Another photo since the last frame: its options menu, opened for the photo before,
    // closes, and a choice from it is dropped, never run on this one.
    let photo_id = ui.make_persistent_id("history options photo");
    let before = ui.data(|d| d.get_temp::<Option<u64>>(photo_id));
    ui.data_mut(|d| d.insert_temp(photo_id, state.photo));
    let replaced = before.is_some_and(|before| before != state.photo);
    let mut done = Vec::new();
    if let Some(out) = header {
        done.extend(out.actions.iter().filter_map(|action| match action {
            SectionAction::Toggle => Some(Action::Toggle),
            SectionAction::Reset => Some(Action::Reset),
            _ => None,
        }));
        if replaced {
            if let Some(button) = out.options {
                apertrust_ui::popup::close(ui.ctx(), apertrust_ui::popup::button_menu(button));
            }
        } else {
            done.extend(out.chosen.as_deref().and_then(chosen));
        }
    }
    done.extend(actions);
    done
}

/// The options' ids, as their items answer.
const COMPRESS: &str = "compress";
const TRUNCATE: &str = "truncate";
const STYLE: &str = "style";
const DISCARD: &str = "discard";

/// The section's options as the popup service lists them (`docs/history.md`, In the app):
/// Compress History, Truncate History in its place while Option (Alt) is held where it can
/// run, each with its tip naming the other's key; Create a Style…, disabled until styles
/// come (Phase 3); and Discard History…, apart, as the destructive one. `os` names the key.
fn menu(words: &Translator, options: Options, os: egui::os::OperatingSystem) -> Vec<MenuEntry> {
    let mut args = Args::new();
    let key = if os == egui::os::OperatingSystem::Mac {
        "Option"
    } else {
        "Alt"
    };
    args.set("key", key);
    // The tip names Option only where it shows Truncate History.
    let tip = if options.truncate {
        words.text_with("history-compress-tip-truncate", Some(&args))
    } else {
        words.text("history-compress-tip")
    };
    let mut compress = Item::new(COMPRESS, words.text("history-options-compress"))
        .enabled(options.compress)
        .tip(tip);
    if options.truncate {
        let truncate = Item::new(TRUNCATE, words.text("history-options-truncate"))
            .tip(words.text("history-truncate-tip"));
        compress = compress.alternate(truncate);
    }
    let style = Item::new(STYLE, words.text("history-options-style"))
        .enabled(false)
        .tip(words.text("history-options-style-later"));
    let tip = discard_why(options).unwrap_or(if options.unreadable {
        "history-discard-tip-unreadable"
    } else {
        "history-discard-tip"
    });
    let discard = Item::new(DISCARD, words.text("action-discard-history"))
        .enabled(options.discard)
        .tip(words.text(tip));
    vec![
        compress.into(),
        style.into(),
        MenuEntry::Separator,
        discard.into(),
    ]
}

/// The action an option's id asks for; none for one that runs nothing yet.
fn chosen(id: &str) -> Option<Action> {
    match id {
        COMPRESS => Some(Action::Compress),
        TRUNCATE => Some(Action::Truncate),
        DISCARD => Some(Action::Discard),
        _ => None,
    }
}

/// Draws the section's list, a tree, and returns what a click or a key asked for.
fn list(
    ui: &mut egui::Ui,
    history: Option<&History>,
    modules: &[&dyn Module],
    hidden: &[&str],
    editable: bool,
    today: i32,
) -> Vec<Action> {
    let words = translator(ui.ctx());
    let listed = kept(ui.ctx(), history, modules, &words, today);
    let base = ui.make_persistent_id("history lines");
    let open_id = base.with("open");
    let mut open: HashSet<GroupKey> = ui.data(|d| d.get_temp(open_id)).unwrap_or_default();
    // The end moved, or another photo's history came: the group it lands inside opens,
    // and it is scrolled into sight.
    let seen_id = base.with("end");
    let seen = history.map(|h| (h.end, h.steps.len(), h.steps.first().map(|s| s.id)));
    let moved =
        ui.data(|d| d.get_temp::<Option<(usize, usize, Option<StepId>)>>(seen_id)) != Some(seen);
    if moved {
        ui.data_mut(|d| d.insert_temp(seen_id, seen));
        for entry in &listed.entries {
            if let Entry::Group {
                key,
                holds_end: true,
                ..
            } = entry
            {
                open.insert(*key);
            }
        }
    }
    let rows = drawn(&listed, &open);
    // Disabled through the list's own builder, so the lines are its children, as screen
    // readers count them.
    let title = words.text("section-history");
    let tree = UiBuilder::new()
        .id_salt("history lines")
        .accessibility_role(Role::Tree)
        .accessibility_label(&title);
    let tree = if history.is_some() {
        tree
    } else {
        tree.disabled()
    };
    let mut pass = Pass {
        order: (rows.iter())
            .filter_map(|row| match row {
                Drawn::Line(shown) => Some(*shown),
                _ => None,
            })
            .collect(),
        // One numbers' column for the whole section, so the names stay put as it scrolls.
        columns: Columns::of(ui.ctx(), &listed, &words),
        rows: &rows,
        undone: listed.undone,
        base,
        editable,
        hidden,
        moved,
        index: 0,
        actions: Vec::new(),
        toggled: Vec::new(),
    };
    ui.scope_builder(tree, |ui| {
        // Rows stack with no space between, so the rail runs through.
        ui.spacing_mut().item_spacing.y = 0.0;
        let top = pass.order.iter().filter(|shown| shown.level == 0).count();
        ui.ctx()
            .accesskit_node_builder(ui.unique_id(), |node| node.set_size_of_set(top));
        let mut at = 0;
        while at < rows.len() {
            let Some(group) = inside(&rows[at]) else {
                pass.row(ui, at);
                at += 1;
                continue;
            };
            // An open group's lines, and the undone mark among them, in a group of their
            // own, so screen readers count them apart from the top's.
            let run = (rows[at..].iter())
                .take_while(|row| matches!(row, Drawn::Undone) || inside(row).is_some())
                .count();
            let count = (rows[at..at + run].iter())
                .filter(|row| inside(row).is_some())
                .count();
            let lines = UiBuilder::new()
                .id_salt(("history group", group))
                .accessibility_role(Role::Group);
            ui.scope_builder(lines, |ui| {
                ui.ctx()
                    .accesskit_node_builder(ui.unique_id(), |node| node.set_size_of_set(count));
                for row in at..at + run {
                    pass.row(ui, row);
                }
            });
            at += run;
        }
    });
    let Pass {
        actions, toggled, ..
    } = pass;
    if !toggled.is_empty() {
        for key in toggled {
            let LineKey::Group(group) = key else { continue };
            if !open.remove(&group) {
                open.insert(group);
            }
        }
        ui.ctx().request_repaint();
    }
    ui.data_mut(|d| d.insert_temp(open_id, open));
    actions
}

/// The group of a line inside one.
fn inside(row: &Drawn<'_>) -> Option<GroupKey> {
    match row {
        Drawn::Line(shown) => shown.within,
        _ => None,
    }
}

/// One frame's drawing of the list's rows, and what they asked for.
struct Pass<'a> {
    rows: &'a [Drawn<'a>],
    /// The lines in order, for the keys, and the index of the next one drawn.
    order: Vec<Shown<'a>>,
    index: usize,
    columns: Columns,
    undone: usize,
    /// The id the lines' ids are made from.
    base: egui::Id,
    editable: bool,
    /// The modules the session reports hidden, as the eye holds them.
    hidden: &'a [&'a str],
    /// Whether the end moved, to be scrolled into sight.
    moved: bool,
    actions: Vec<Action>,
    /// The groups to open or close.
    toggled: Vec<LineKey>,
}

impl Pass<'_> {
    /// Draws the row `rows[at]`.
    fn row(&mut self, ui: &mut egui::Ui, at: usize) {
        let rows = self.rows;
        let shown = match &rows[at] {
            Drawn::Day(day) => return day_row(ui, day, through(rows, at), self.columns),
            Drawn::Gap => return gap_row(ui, through(rows, at)),
            Drawn::Undone => return undone_mark(ui, self.undone, self.columns),
            Drawn::Line(shown) => shown,
        };
        let id_of = |key: LineKey| self.base.with(key);
        let hidden = (shown.line.hides).is_some_and(|module| self.hidden.contains(&module));
        let out = line_row(
            ui,
            id_of(shown.line.key),
            shown,
            self.columns,
            self.editable,
            hidden,
        );
        self.actions.extend(out.action);
        if out.toggle {
            self.toggled.push(shown.line.key);
        }
        if self.moved && shown.chosen {
            out.response.scroll_to_me(None);
        }
        if out.response.has_focus() {
            match keyed(ui, &out.response, &self.order, self.index) {
                Some(Keyed::Focus(to)) => {
                    let to = id_of(self.order[to].line.key);
                    ui.memory_mut(|m| m.request_focus(to));
                }
                Some(Keyed::Toggle) => self.toggled.push(shown.line.key),
                None => {}
            }
        }
        self.index += 1;
    }
}

/// What a key did on a focused line.
enum Keyed {
    /// Move the focus to the line at this index.
    Focus(usize),
    /// Open or close the focused group.
    Toggle,
}

/// The keys of a tree on the focused line, `order[index]`, its `response`: ↑ and ↓ the
/// line above and below, Home the first and End the last; → opens a closed group and
/// goes into an open one, ← closes an open group and goes from a line inside one to its
/// group's. The arrows kept from egui's own moves of the focus, which might leave the list.
fn keyed(
    ui: &egui::Ui,
    response: &egui::Response,
    order: &[Shown<'_>],
    index: usize,
) -> Option<Keyed> {
    let filter = egui::EventFilter {
        horizontal_arrows: true,
        vertical_arrows: true,
        ..Default::default()
    };
    ui.memory_mut(|m| m.set_focus_lock_filter(response.id, filter));
    let pressed = |key| ui.input_mut(|i| i.consume_key(Modifiers::NONE, key));
    let shown = order[index];
    if pressed(Key::ArrowUp) {
        return index.checked_sub(1).map(Keyed::Focus);
    }
    if pressed(Key::ArrowDown) {
        return (index + 1 < order.len()).then_some(Keyed::Focus(index + 1));
    }
    if pressed(Key::Home) {
        return Some(Keyed::Focus(0));
    }
    if pressed(Key::End) {
        return Some(Keyed::Focus(order.len() - 1));
    }
    if pressed(Key::ArrowRight) {
        return match shown.open {
            Some(false) => Some(Keyed::Toggle),
            Some(true) => (index + 1 < order.len()).then_some(Keyed::Focus(index + 1)),
            None => None,
        };
    }
    if pressed(Key::ArrowLeft) {
        if shown.open == Some(true) {
            return Some(Keyed::Toggle);
        }
        let within = shown.within?;
        let group = order[..index]
            .iter()
            .rposition(|line| line.line.key == LineKey::Group(within))?;
        return Some(Keyed::Focus(group));
    }
    None
}

/// [`listed`], worded once for each history, language and day and kept in egui's memory:
/// a history of hundreds of steps would otherwise be worded again each frame. A history is
/// told from another by its `through`, its end and its steps, each
/// behind the `Arc` the session shares while the step stays as it is.
fn kept(
    ctx: &egui::Context,
    history: Option<&History>,
    modules: &[&dyn Module],
    tr: &Translator,
    today: i32,
) -> Arc<Listed> {
    let steps = history.map(|h| {
        let shared = h.steps.iter().map(|step| Arc::as_ptr(step) as usize);
        (h.through, h.end, shared.collect::<Vec<_>>())
    });
    let key = {
        use std::hash::{BuildHasher as _, BuildHasherDefault, DefaultHasher};
        BuildHasherDefault::<DefaultHasher>::default().hash_one((tr.locale().id(), today, steps))
    };
    // One slot, the latest only.
    let slot = egui::Id::unique("history lines");
    let kept = ctx.data(|data| data.get_temp::<(u64, Arc<Listed>)>(slot));
    if let Some((kept, listed)) = kept
        && kept == key
    {
        return listed;
    }
    let worded = Arc::new(listed(history, modules, tr, today));
    ctx.data_mut(|data| data.insert_temp(slot, (key, Arc::clone(&worded))));
    worded
}

/// The rail, from a line's start, as the board draws it.
const RAIL: f32 = 4.0;

/// Where the numbers' column starts, from a line's start, clear of the rail's nodes, and
/// the gap between the column and the names.
const NUMBERS: f32 = 14.0;
const NAME_GAP: f32 = 8.0;

/// How far a group's name stands after its chevron, and a line inside a group after the
/// group's: the chevron's width and its gap.
const CHEVRON: f32 = 13.0;

/// The range the numbers' column is wide enough for by default: two-digit steps, as a
/// batch's line writes them, so the names stay put as a history grows its first groups.
const DEFAULT_RANGE: (usize, usize) = (12, 16);

/// The numbers' column of a section: as wide as its widest number, the highest step's or
/// a batch's range, or a two-digit range, whichever is widest, in the caption size; each
/// number centred in it, and the names after it. One for the whole section, so nothing
/// moves as it scrolls or a group opens.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Columns {
    width: f32,
}

impl Columns {
    /// The column for `listed`, measured in `ctx`'s caption font.
    fn of(ctx: &egui::Context, listed: &Listed, tr: &Translator) -> Columns {
        let t = theme(ctx);
        let font = FontId::proportional(t.text.caption);
        let width_of = |text: String| {
            ctx.fonts_mut(|fonts| fonts.layout_no_wrap(text, font.clone(), t.colors.text))
                .size()
                .x
        };
        let number = |n: usize| tr.format_number(n as f64, 0, false);
        let (first, last) = DEFAULT_RANGE;
        let highest = listed.lines().filter_map(Line::number).max().unwrap_or(0);
        let ranges = (listed.entries.iter()).filter_map(|entry| match entry {
            Entry::Group { line, .. } => line.numbers(tr),
            _ => None,
        });
        let widest = [range(&number(first), &number(last)), number(highest)]
            .into_iter()
            .chain(ranges)
            .map(width_of)
            .fold(0.0, f32::max);
        Columns { width: widest }
    }

    /// Where a number `width` wide starts, centred in the column.
    fn number_at(self, width: f32) -> f32 {
        NUMBERS + (self.width - width) / 2.0
    }

    /// Where the names start, after the column.
    fn name(self) -> f32 {
        NUMBERS + self.width + NAME_GAP
    }
}

/// A day's heading, 18 points high: its words in the caption's medium weight and the
/// secondary text, where the names start, the rail through it as `rail` says. To screen
/// readers a heading, never focused.
fn day_row(ui: &mut egui::Ui, day: &str, rail: RailLine, columns: Columns) {
    let t = theme(ui.ctx());
    let size = vec2(ui.available_width(), DAY);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let through = Rail {
            node: RailNode::None,
            above: rail,
            below: rail,
        };
        let enabled = ui.is_enabled();
        through.paint(
            painter,
            rect,
            rect.left() + RAIL,
            t.colors.panel,
            enabled,
            &t,
        );
        let font = FontId::new(t.text.caption, egui::FontFamily::Name(fonts::MEDIUM.into()));
        let colour = t.colors.text_secondary;
        let room = rect.right() - rect.left() - columns.name();
        if let Some(galley) = cut_at(painter, day, font, colour, room) {
            // A point under the middle, nearer the steps it heads, as the board sets it.
            let at = pos2(
                rect.left() + columns.name(),
                rect.center().y + 1.0 - galley.size().y / 2.0,
            );
            painter.galley(at, galley, colour);
        }
    }
    response.widget_info(|| WidgetInfo::labeled(Role::Heading, ui.is_enabled(), day));
}

/// A day heading's row, its height.
const DAY: f32 = 18.0;

/// The space over the Defaults' line, the rail through it as `rail` says.
fn gap_row(ui: &mut egui::Ui, rail: RailLine) {
    let t = theme(ui.ctx());
    // Space alone, no widget, which screen readers would count among the list's.
    let rect = egui::Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), GAP));
    ui.add_space(GAP);
    if ui.is_rect_visible(rect) {
        let through = Rail {
            node: RailNode::None,
            above: rail,
            below: rail,
        };
        let enabled = ui.is_enabled();
        through.paint(
            ui.painter(),
            rect,
            rect.left() + RAIL,
            t.colors.panel,
            enabled,
            &t,
        );
    }
}

/// The gap's height.
const GAP: f32 = 4.0;

/// The mark under the undone steps, `count` of them: "↑ 2 undone · your next edit
/// replaces them" in the caption's secondary text, cut at the row's end with its whole
/// words in its tooltip, then a dashed line to the row's end, the rail dashed through it,
/// as the board draws it. To screen readers a note, not one of the list's items, in words
/// with no arrow or dot: "2 undone: your next edit replaces them".
fn undone_mark(ui: &mut egui::Ui, count: usize, columns: Columns) {
    let t = theme(ui.ctx());
    let tr = translator(ui.ctx());
    let mut args = Args::new();
    args.set("count", count);
    let words = tr.text_with("history-undone", Some(&args));
    let said = tr.text_with("history-undone-said", Some(&args));
    let size = vec2(ui.available_width(), UNDONE_MARK);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    let mut cut = false;
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let rail = Rail {
            node: RailNode::None,
            above: RailLine::Dashed,
            below: RailLine::Dashed,
        };
        rail.paint(painter, rect, rect.left() + RAIL, t.colors.panel, true, &t);
        let font = FontId::proportional(t.text.caption);
        let middle = rect.center().y;
        let start = rect.left() + columns.name();
        let shown = cut_at(
            painter,
            &words,
            font,
            t.colors.text_secondary,
            rect.right() - start,
        );
        let width = shown.as_ref().map_or(0.0, |galley| galley.size().x);
        cut = shown.as_ref().is_none_or(|galley| galley.text() != words);
        if let Some(galley) = shown {
            let at = pos2(start, middle - galley.size().y / 2.0);
            painter.galley(at, galley, t.colors.text_secondary);
        }
        let after = start + width + t.spacing.s;
        if after < rect.right() {
            let stroke = egui::Stroke::new(t.shape.hairline, t.colors.text_disabled);
            painter.extend(egui::Shape::dashed_line(
                &[pos2(after, middle), pos2(rect.right(), middle)],
                stroke,
                2.0,
                2.0,
            ));
        }
    }
    response.widget_info(|| WidgetInfo::labeled(Role::Note, true, &said));
    if cut {
        response.on_hover_text(words);
    }
}

/// The undone mark's row, its height.
const UNDONE_MARK: f32 = 20.0;

/// What a line's click asked for.
struct Clicked {
    response: egui::Response,
    action: Option<Action>,
    /// A group's chevron was clicked: open or close it.
    toggle: bool,
}

/// One line, `sizes.panel_line` high, as the board draws it, its widget `id`: its piece
/// of the rail, its number, or a batch's range, centred in the section's numbers' column,
/// a group's chevron, the name, a group's in the medium weight and a line inside a group
/// indented under it, then what changed in the small text, cut at its end where there is
/// no room, and the tag before the eye's room at the row's end; the end's on
/// `list_chosen`, an undone step's in `text_secondary`; and the eye on a step of one
/// module ([`eye_button`]), its module's name struck through in `text_secondary` while the
/// session reports it `hidden`. Returns what a click, or Enter, asked for, and the eye held.
fn line_row(
    ui: &mut egui::Ui,
    id: egui::Id,
    shown: &Shown<'_>,
    columns: Columns,
    editable: bool,
    hidden: bool,
) -> Clicked {
    let t = theme(ui.ctx());
    let words = translator(ui.ctx());
    let line = shown.line;
    let rect = egui::Rect::from_min_size(
        ui.cursor().min,
        vec2(ui.available_width(), t.sizes.panel_line),
    );
    // Asked before the line takes its interest in the focus, which Tab passes on there.
    let focused = ui.memory(|m| m.has_focus(id));
    let response = ui.interact(rect, id, Sense::click());
    ui.advance_cursor_after_rect(rect);
    // A line the keys reach is scrolled into sight.
    if response.gained_focus() {
        response.scroll_to_me(None);
    }
    // The eye, under the pointer and with the focus, the line's or its own last frame; the
    // pointer over it is still over the line. Drawn as the line passes the focus on with
    // Tab, it takes it next.
    let eye_id = id.with("eye");
    let eye_focused = ui.data(|d| d.get_temp::<bool>(eye_id)).unwrap_or(false);
    let over = ui.rect_contains_pointer(rect);
    let focused = focused || response.has_focus() || eye_focused;
    let eye = (line.hides).filter(|_| over || focused);
    let (chosen, greyed) = (shown.chosen, line.greyed());
    let start = rect.left()
        + columns.name()
        + if shown.open.is_some() || shown.level > 0 {
            CHEVRON
        } else {
            0.0
        };
    let chevron = shown.open.map(|_| {
        egui::Rect::from_min_max(
            pos2(rect.left() + columns.name() - 2.0, rect.top()),
            pos2(start, rect.bottom()),
        )
    });
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let band = rect.expand2(vec2(t.spacing.xs, 0.0));
        let hovered = (response.hovered() || eye.is_some() && over) && line.takes(editable);
        let ground = if chosen {
            t.colors.list_chosen
        } else if hovered {
            t.colors.hover
        } else {
            t.colors.panel
        };
        if ground != t.colors.panel {
            painter.rect_filled(band, t.shape.radius, ground);
        }
        let enabled = ui.is_enabled();
        shown
            .rail
            .paint(painter, rect, rect.left() + RAIL, ground, enabled, &t);
        // The end in the strong text, its change and number in the text; an undone step
        // all in the secondary text, which still reads as a line that takes a click.
        let (ink, quiet, number) = if !enabled {
            let off = t.colors.text_disabled;
            (off, off, off)
        } else if chosen {
            (t.colors.text_strong, t.colors.text, t.colors.text)
        } else if greyed {
            let secondary = t.colors.text_secondary;
            (secondary, secondary, secondary)
        } else {
            (
                t.colors.text,
                t.colors.text_secondary,
                t.colors.text_secondary,
            )
        };
        // A module hidden while the eye is held: its name in the secondary text, struck
        // through below.
        let ink = if hidden && enabled {
            t.colors.text_secondary
        } else {
            ink
        };
        let caption = FontId::proportional(t.text.caption);
        let middle = rect.center().y;
        let centred =
            |galley: &std::sync::Arc<egui::Galley>, x: f32| pos2(x, middle - galley.size().y / 2.0);
        if let Some(n) = line.numbers(&words) {
            let galley = painter.layout_no_wrap(n, caption.clone(), number);
            let at = centred(&galley, rect.left() + columns.number_at(galley.size().x));
            painter.galley(at, galley, number);
        }
        // A group's chevron where the names start, in the line's ink while it is chosen
        // or under the pointer, else the secondary text.
        if let Some(open) = shown.open {
            let icon = if open {
                Icon::ChevronDown
            } else {
                Icon::Chevron
            };
            let colour = if !enabled {
                t.colors.text_disabled
            } else if chosen || hovered {
                ink
            } else {
                t.colors.text_secondary
            };
            let at = pos2(rect.left() + columns.name() - 1.0, middle - 5.0);
            icon.paint(
                painter,
                egui::Rect::from_min_size(at, vec2(10.0, 10.0)),
                colour,
            );
        }
        // Every line leaves the eye's room at its end, so the tags line up.
        let mut end = rect.right() - EYE - t.spacing.xxs;
        if let Some(tag) = &line.tag {
            let tagged = if chosen {
                t.colors.text_secondary
            } else {
                quiet
            };
            let galley = painter.layout_no_wrap(tag.clone(), caption.clone(), tagged);
            end -= galley.size().x;
            painter.galley(centred(&galley, end), galley, tagged);
            end -= t.spacing.s;
        }
        let body = if line.group() {
            FontId::new(t.text.body, egui::FontFamily::Name(fonts::MEDIUM.into()))
        } else {
            FontId::proportional(t.text.body)
        };
        let small = FontId::proportional(t.text.small);
        // The name first, but a name cut leaves part of what changed in sight.
        let width_of = |text: &str, font: &FontId| {
            painter
                .layout_no_wrap(text.to_owned(), font.clone(), ink)
                .size()
                .x
        };
        let changed = (line.change.as_ref()).map(|change| width_of(change, &small) + t.spacing.s);
        let room = name_room(end - start, width_of(&line.name, &body), changed);
        let name = cut_at(painter, &line.name, body, ink, room);
        let after = start + name.as_ref().map_or(0.0, |name| name.size().x) + t.spacing.s;
        if let Some(name) = name {
            let (at, size) = (centred(&name, start), name.size());
            painter.galley(at, name, ink);
            if hidden {
                let stroke = egui::Stroke::new(t.shape.hairline, ink);
                painter.hline(at.x..=at.x + size.x, at.y + size.y / 2.0, stroke);
            }
        }
        // What changed, cut at its end where the line has no room.
        let change = (line.change.as_ref())
            .and_then(|change| cut_at(painter, change, small, quiet, end - after));
        if let Some(galley) = change {
            painter.galley(centred(&galley, after), galley, quiet);
        }
        if response.has_focus() {
            focus_ring_centred(painter, band, &t);
        }
    }
    // Worded only when asked, by screen readers: a long history has many lines.
    let enabled = ui.is_enabled();
    response.widget_info(|| WidgetInfo::labeled(Role::TreeItem, enabled, said(line, &words)));
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_selected(chosen);
        node.set_level(shown.level);
        node.set_position_in_set(shown.position);
        // A group's line opens and closes by its own actions; its click goes to its last
        // step, as a pointer's click on the line does.
        if let Some(open) = shown.open {
            node.set_expanded(open);
            node.add_action(if open {
                egui::accesskit::Action::Collapse
            } else {
                egui::accesskit::Action::Expand
            });
        }
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
    let at = pos2(rect.right() - EYE, rect.center().y - EYE / 2.0);
    let area = egui::Rect::from_min_size(at, vec2(EYE, EYE));
    // Whether the primary button's press under way began on the eye, kept from its frame
    // and read from the press's own event: egui forgets where a press began as it is let
    // go, before the frame's widgets are drawn where both come in one frame, as a tap's do.
    let eye_press = id.with("eye press");
    let began = ui.input(|i| {
        (i.events.iter().rev()).find_map(|event| match event {
            egui::Event::PointerButton {
                button: egui::PointerButton::Primary,
                pressed: true,
                pos,
                ..
            } => Some(area.contains(*pos)),
            _ => None,
        })
    });
    if let Some(on_eye) = began {
        ui.data_mut(|d| d.insert_temp(eye_press, on_eye));
    }
    // The pointer on the eye shows the eye's tooltip alone: the line's, open, would stay
    // while the pointer is inside the line.
    let on_eye = eye.is_some()
        && ui
            .ctx()
            .pointer_hover_pos()
            .is_some_and(|p| area.contains(p));
    let response = if on_eye {
        response
    } else {
        response.on_hover_ui(|ui| tip_ui(ui, &line.tip))
    };
    let eyed = eye.and_then(|module| {
        // Over the line, and inside its item for screen readers.
        let builder = UiBuilder::new()
            .id_salt(eye_id)
            .max_rect(area)
            .accessibility_parent(response.id);
        let pressed = eye_button(&mut ui.new_child(builder), &line.name);
        let focused = pressed.has_focus();
        ui.data_mut(|d| d.insert_temp(eye_id, focused));
        held(ui, &pressed, area).then_some(Action::Peek(module))
    });
    if eye.is_none() && eye_focused {
        ui.data_mut(|d| d.remove::<bool>(eye_id));
    }
    // A screen reader's Expand on a closed group, or Collapse on an open one.
    let disclosed = shown.open.is_some_and(|open| {
        let asked = if open {
            egui::accesskit::Action::Collapse
        } else {
            egui::accesskit::Action::Expand
        };
        ui.input(|i| i.has_accesskit_action_request(response.id, asked))
    });
    let mut clicked = Clicked {
        action: eyed,
        toggle: disclosed,
        response: response.clone(),
    };
    if !response.clicked() {
        return clicked;
    }
    // A pointer's press that began on the eye is the eye's, a peek, though the line took it
    // where the eye was not drawn yet, the pointer arriving and pressing in one frame, as a
    // tap does: it goes to no step. A key's click is the line's.
    if line.hides.is_some() && pressed_on(ui, eye_press) {
        return clicked;
    }
    // The chevron, under the pointer, opens or closes the group; a key's click goes to it.
    let on_chevron = (response.interact_pointer_pos())
        .is_some_and(|at| chevron.is_some_and(|chevron| chevron.contains(at)));
    if on_chevron {
        clicked.toggle = true;
    } else {
        clicked.action = line.clicked(shift_clicked(ui), editable);
    }
    clicked
}

/// Whether this frame's click is the primary button's release of a press that began on the
/// eye, as its frame kept it under `eye_press`; a key's click is not.
fn pressed_on(ui: &egui::Ui, eye_press: egui::Id) -> bool {
    let released = ui.input(|i| {
        i.events.iter().any(|event| {
            matches!(
                event,
                egui::Event::PointerButton {
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    ..
                }
            )
        })
    });
    released && ui.data(|d| d.get_temp::<bool>(eye_press)) == Some(true)
}

/// The eye's side, at a line's end, the icon's and a point round it, as the board draws it.
const EYE: f32 = 16.0;

/// The eye on a step's line, `module` its module's title: an icon button, held to see the
/// photo without the module, never saved. To screen readers a plain button, "Hold to see
/// the photo without Exposure", its tooltip saying nothing is saved.
fn eye_button(ui: &mut egui::Ui, module: &str) -> egui::Response {
    let words = translator(ui.ctx());
    let mut args = Args::new();
    args.set("module", module);
    let name = words.text_with("history-peek", Some(&args));
    let tip = words.text_with("history-peek-tip", Some(&args));
    IconButton::new(Icon::Visible, &name)
        .tip(&tip)
        .size(EYE)
        .show(ui)
}

/// Whether the eye, its `response` over `area`, is held this frame: the primary button
/// pressed on this eye and down, the pointer still over it, or Space or Enter down while it
/// has the focus; never while the window has lost the focus, so a release the window never
/// saw leaves nothing hidden, and not again after that until a new press of the button or
/// the key. Read from the pointer's own state, as egui stops counting a press longer than a
/// click's as one on the widget (`is_pointer_button_down_on`); the press is bound to the eye
/// it began on by its start time, so a list scrolled under a held button never hands the
/// hold to another line's eye.
fn held(ui: &egui::Ui, response: &egui::Response, area: egui::Rect) -> bool {
    let inside = |at: Option<egui::Pos2>| at.is_some_and(|at| area.contains(at));
    let (down, began, start, over, keys, focused, pressed) = ui.input(|i| {
        let p = &i.pointer;
        let keys = i.key_down(Key::Space) || i.key_down(Key::Enter);
        let pressed = p.primary_pressed() || i.key_pressed(Key::Space) || i.key_pressed(Key::Enter);
        (
            p.primary_down(),
            p.primary_pressed() && inside(p.press_origin()),
            p.press_start_time(),
            inside(p.hover_pos()),
            keys,
            i.focused,
            pressed,
        )
    });
    // The press this eye took: its start time, which no later press shares.
    let press = egui::Id::unique("history eye press");
    if began {
        ui.data_mut(|d| d.insert_temp(press, (response.id, start)));
    }
    let ours =
        ui.data(|d| d.get_temp::<(egui::Id, Option<f64>)>(press)) == Some((response.id, start));
    let pointer = down && over && ours;
    // Set as the window loses the focus mid-hold, cleared by the next press.
    let lost = response.id.with("lost");
    let mut stopped = ui.data(|d| d.get_temp::<bool>(lost)).unwrap_or(false);
    let holding = pointer || keys && response.has_focus();
    // Only a new press clears it: the pointer leaving the eye and coming back with the
    // button still down is the same press.
    if !focused && holding {
        stopped = true;
    } else if pressed {
        stopped = false;
    }
    ui.data_mut(|d| d.insert_temp(lost, stopped));
    focused && holding && !stopped
}

/// A line's name as screen readers hear it: a step's number, name and what changed, its
/// values before and after; a group's numbers, name and modules, the Defaults' how many;
/// Original's name.
fn said(line: &Line, words: &Translator) -> String {
    let mut args = Args::new();
    args.set("name", line.name.as_str());
    match line.place {
        Place::Step { number, .. } => {
            args.set("number", number);
            let id = match &line.heard {
                Some(change) => {
                    args.set("change", change.as_str());
                    "history-step-name-change"
                }
                None => "history-step-name",
            };
            words.text_with(id, Some(&args))
        }
        Place::Batch { first, last, .. } => {
            args.set("first", first);
            args.set("last", last);
            args.set("modules", line.change.clone().unwrap_or_default());
            words.text_with("history-group-name", Some(&args))
        }
        Place::Original { .. } => line.name.clone(),
    }
}

/// The room a line's name gets of `room`, the name `name` points wide and what changed
/// after it `changed`, its gap included: all it needs where it fits beside what changed
/// whole, or beside what changed's share, its width up to a third of the room; else the
/// room less that share, so a name cut leaves part of what changed in sight.
fn name_room(room: f32, name: f32, changed: Option<f32>) -> f32 {
    let share = changed.map_or(0.0, |changed| changed.min(room / 3.0));
    if name <= room - share {
        room
    } else {
        room - share
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
    // Apart from the rows; a default step's alone.
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

/// What of `text` a line shows in `room` points, `width_of` measuring a string: the whole
/// where it fits, else its longest start that fits with … after it; nothing where not
/// even one character and … fit, so a lone … never shows, as epaint would paint one at no
/// width, over what comes after.
fn fit(text: &str, room: f32, width_of: impl Fn(&str) -> f32) -> Option<String> {
    if width_of(text) <= room {
        return Some(text.to_owned());
    }
    let chars: Vec<char> = text.chars().collect();
    let cut = |count: usize| {
        let start: String = chars[..count].iter().collect();
        format!("{}{ELLIPSIS}", start.trim_end())
    };
    let fits = |count: usize| width_of(&cut(count)) <= room;
    if chars.len() < 2 || !fits(1) {
        return None;
    }
    // The longest start that fits: `fitting` does, `over` does not.
    let (mut fitting, mut over) = (1, chars.len());
    while over - fitting > 1 {
        let middle = (fitting + over) / 2;
        if fits(middle) {
            fitting = middle;
        } else {
            over = middle;
        }
    }
    Some(cut(fitting))
}

/// What a cut line ends in.
const ELLIPSIS: char = '…';

/// `text` on one line in `font`, cut at its end with … where it is wider than `width`
/// ([`fit`]); none where nothing of it fits.
fn cut_at(
    painter: &egui::Painter,
    text: &str,
    font: FontId,
    colour: egui::Color32,
    width: f32,
) -> Option<std::sync::Arc<egui::Galley>> {
    let width_of = |text: &str| {
        let galley = painter.layout_no_wrap(text.to_owned(), font.clone(), colour);
        galley.size().x
    };
    let shown = fit(text, width, width_of)?;
    Some(painter.layout_no_wrap(shown, font, colour))
}

/// A step's tag: its source where it is not the person's own, Auto, Style or Assistant;
/// You only in the tooltip.
fn tag(source_of: StepSource, tr: &Translator) -> Option<String> {
    match source_of {
        StepSource::You | StepSource::Default => None,
        other => Some(source(other, tr)),
    }
}

/// What a step changed, as its line says it after the module's name: one number its
/// control's label and which way it went, "Shadows ↑" or "Strength ↓", the arrow alone
/// where the control is the module's own, as Exposure's slider, and no arrow where it did
/// not move; any other control, a curve, a choice, a toggle, or one kept, released or set
/// by Auto, its label alone, none where it is the module's own; several controls how
/// many. Nothing for Reset, a step of source Default or a module this build does not know,
/// which has no control; an operation's unowned parameter, with no definition, by its
/// name, as the tooltip and screen readers have it. The before and after are the
/// tooltip's.
fn summary(step: &HistoryStep, modules: &[&dyn Module], tr: &Translator) -> Option<String> {
    if step.source == StepSource::Default || is_reset(step) {
        return None;
    }
    let changed = step.changes.first()?;
    let definition = definition(modules, &changed.module);
    match changed.controls.as_slice() {
        [] => None,
        [one] => {
            let label = Some(label(definition, one.control, tr))
                .filter(|label| *label != title(modules, &changed.module, tr));
            let arrow = match (&one.before, &one.after) {
                (ControlValue::Number(before), ControlValue::Number(after)) if after > before => {
                    Some(UP)
                }
                (ControlValue::Number(before), ControlValue::Number(after)) if after < before => {
                    Some(DOWN)
                }
                _ => None,
            };
            match (label, arrow) {
                (Some(label), Some(arrow)) => Some(format!("{label} {arrow}")),
                (Some(label), None) => Some(label),
                (None, arrow) => arrow.map(str::to_owned),
            }
        }
        several => {
            let mut args = Args::new();
            args.set("count", several.len());
            Some(tr.text_with("history-line-controls", Some(&args)))
        }
    }
}

/// The arrows of a number that went up or down: glyphs, the same in every language.
const UP: &str = "↑";
const DOWN: &str = "↓";

/// What a step changed as screen readers hear it in its name: each control's label and its
/// value before and after, as the tooltip has them, the label left out where it is the
/// module's own; nothing where the line says nothing.
fn heard(step: &HistoryStep, modules: &[&dyn Module], tr: &Translator) -> Option<String> {
    if step.source == StepSource::Default || is_reset(step) {
        return None;
    }
    let changed = step.changes.first()?;
    let definition = definition(modules, &changed.module);
    let own = title(modules, &changed.module, tr);
    let parts: Vec<String> = (changed.controls.iter())
        .map(|control| match control_change(definition, control, tr) {
            TipRow::Value {
                label,
                before,
                after,
            } => {
                let mut args = Args::new();
                args.set("before", before);
                args.set("after", after);
                if label == own {
                    return tr.text_with("history-said-change-alone", Some(&args));
                }
                args.set("control", label);
                tr.text_with("history-said-change", Some(&args))
            }
            TipRow::Said(said) => said,
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join("; "))
}

/// A control's label, or an unowned parameter's name, with no definition.
fn label(definition: Option<&'static Definition>, control: &str, tr: &Translator) -> String {
    match definition {
        Some(definition) => tr.text(&messages::label(definition.id, control)),
        None => control.to_owned(),
    }
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
/// module's rows, and its note is its line's name. A default step, the photo's own, has
/// who and when alone.
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
        Move::Compress => Some(tr.text("history-compress")),
        Move::Truncate => Some(tr.text("history-truncate")),
        Move::DiscardHistory => Some(tr.text("history-discard")),
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
    use apertrust_ui::{Builtin, Locale, Theme};
    use egui::accesskit::{Role as AccessRole, Toggled};
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

    fn change_of(module: &'static str, controls: Vec<ControlChange>) -> ModuleChange {
        ModuleChange {
            module: module.into(),
            enabled: None,
            controls,
        }
    }

    /// A photo's history of five steps, one module each, the end at the third, so the last
    /// two are greyed: Exposure, Grain turned on, an Auto step of Exposure, a reset of both,
    /// and an assistant's step of Exposure with its note. [`with_defaults`] gives it a new
    /// photo's default steps under them.
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
            undo: None,
            redo: None,
        }
    }

    /// How many default steps a new photo is given, of the modules the app runs.
    const DEFAULT_STEPS: usize = 9;

    /// `history`'s steps after a new photo's default steps, one of each of darktable's
    /// default modules, numbered 1 to [`DEFAULT_STEPS`], the end at `end`.
    fn with_defaults(history: History, end: usize) -> History {
        let defaults = apertrust_engine::history::default_modules(
            apertrust_modules::MODULES,
            apertrust_operations::OPERATIONS,
        );
        assert_eq!(defaults.len(), DEFAULT_STEPS);
        let mut steps: Vec<_> = (defaults.iter().enumerate())
            .map(|(index, module)| {
                made(
                    0xd0 + index as u64,
                    StepSource::Default,
                    None,
                    vec![change(module, None, Vec::new())],
                )
            })
            .collect();
        steps.extend(history.steps.iter().cloned());
        History {
            steps,
            end,
            ..history
        }
    }

    /// The local calendar's day of [`wireframe_s_time`], today as the tests have it.
    fn today() -> i32 {
        local_day(wireframe_s_time()).expect("a day the local clock has")
    }

    fn listed_in(history: Option<&History>, locale: Locale) -> Listed {
        listed(
            history,
            apertrust_modules::MODULES,
            &Translator::new(locale),
            today(),
        )
    }

    /// Every line, a group's and those it holds, open or closed.
    fn lines_in(history: Option<&History>, locale: Locale) -> Vec<Line> {
        listed_in(history, locale).lines().cloned().collect()
    }

    #[test]
    fn the_lines_are_the_steps_newest_first_then_original() {
        let history = photo_s(3);
        let lines = lines_in(Some(&history), Locale::EnGb);
        let shown: Vec<(Option<usize>, &str, Option<&str>)> = (lines.iter())
            .map(|line| (line.number(), line.name.as_str(), line.tag.as_deref()))
            .collect();
        assert_eq!(
            shown,
            [
                // A note leaves the line its module's name, as darktable's; the source
                // only where it is not the person's own.
                (Some(5), "Exposure", Some("Assistant")),
                (Some(4), "Reset", None),
                (Some(3), "Exposure", Some("Auto")),
                (Some(2), "Grain", None),
                (Some(1), "Exposure", None),
                (Some(0), "Original", None),
            ]
        );
        // The end at the third step: it is chosen, the two after it greyed.
        let state: Vec<(bool, bool)> = (lines.iter())
            .map(|line| (line.end(), line.greyed()))
            .collect();
        assert_eq!(
            state,
            [
                (false, true),
                (false, true),
                (true, false),
                (false, false),
                (false, false),
                (false, false)
            ]
        );
        // Each opens its first module the app knows; Original none.
        assert_eq!(lines[2].module, Some("exposure"));
        assert_eq!(lines[3].module, Some("grain"));
        assert_eq!(lines[5].module, None);
        // At end 0 every step is greyed, and Original is chosen.
        let lines = lines_in(Some(&photo_s(0)), Locale::EnGb);
        let chosen: Vec<&str> = (lines.iter())
            .filter(|line| line.end())
            .map(|line| line.name.as_str())
            .collect();
        assert_eq!(chosen, ["Original"]);
        assert!(lines[..5].iter().all(Line::greyed));
        // No history: Original alone.
        let alone = lines_in(None, Locale::EnGb);
        assert_eq!(alone.len(), 1);
        assert_eq!(listed_in(None, Locale::EnGb).entries.len(), 1);
        assert_eq!(
            alone[0].place,
            Place::Original {
                end: true,
                there: true
            }
        );
    }

    /// A new photo's default steps are one closed group, Defaults, over Original: its
    /// numbers' range 1 to n, how many modules, and inside it the default steps numbered,
    /// with no tag, each opening its module; the photo's own steps numbered after them.
    #[test]
    fn the_default_steps_are_the_defaults_group_numbered_from_1() {
        let n = DEFAULT_STEPS;
        let history = with_defaults(photo_s(5), n + 3);
        let tr = Translator::new(Locale::EnGb);
        let listed = listed_in(Some(&history), Locale::EnGb);
        let (line, lines) = (listed.entries.iter())
            .find_map(|entry| match entry {
                Entry::Group {
                    key: GroupKey::Defaults,
                    line,
                    lines,
                    ..
                } => Some((line, lines)),
                _ => None,
            })
            .expect("the Defaults group");
        assert_eq!(line.name, "Defaults");
        assert_eq!(line.numbers(&tr).as_deref(), Some("1\u{2013}9"));
        assert_eq!(line.change.as_deref(), Some("9 modules every photo runs"));
        assert!(line.group() && !line.greyed());
        let shown: Vec<(Option<usize>, &str, Option<&str>)> = (lines.iter())
            .map(|line| (line.number(), line.name.as_str(), line.tag.as_deref()))
            .collect();
        assert_eq!(
            shown,
            [
                (Some(9), "AgX", None),
                (Some(8), "Colour calibration", None),
                (Some(7), "Exposure", None),
                (Some(6), "Orientation", None),
                (Some(5), "Highlight reconstruction", None),
                (Some(4), "White balance", None),
                (Some(3), "Output colour profile", None),
                (Some(2), "Demosaic", None),
                (Some(1), "Raw black/white point", None),
            ]
        );
        assert_eq!(lines[0].module, Some("agx"));
        let all = lines_in(Some(&history), Locale::EnGb);
        assert_eq!(all[0].number(), Some(n + 5));
        // At end 0 the group is undone, greyed, and Original chosen; with the end inside it
        // the group holds the end.
        let at_0 = lines_in(Some(&with_defaults(photo_s(5), 0)), Locale::EnGb);
        let defaults = (at_0.iter()).find(|line| line.name == "Defaults").unwrap();
        assert!(defaults.greyed() && !defaults.end());
        assert!(at_0.last().is_some_and(Line::end));
        let inside = listed_in(Some(&with_defaults(photo_s(5), 4)), Locale::EnGb);
        assert!(inside.entries.iter().any(|entry| matches!(
            entry,
            Entry::Group {
                key: GroupKey::Defaults,
                holds_end: true,
                ..
            }
        )));
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
                .find(|line| matches!(line.place, Place::Step { number: n, .. } if n == number))
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
        // A default step: who and when alone.
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
        /// What the options can do.
        options: Options,
        /// The number of the photo's open.
        photo: Option<u64>,
        /// The modules the eye hides, as the mirror's values report them.
        hidden: Vec<&'static str>,
        actions: Vec<Action>,
        /// The side panel's width; the theme's where none.
        panel: Option<f32>,
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
                        pos2(
                            window.right() - scene.panel.unwrap_or(t.sizes.side_panel),
                            window.top(),
                        ),
                        window.max,
                    );
                    ui.painter()
                        .rect_filled(window, 0.0, t.colors.photo_surround);
                    ui.painter().rect_filled(side, 0.0, t.colors.panel);
                    let history = scene.history.as_ref();
                    let state = State {
                        editable: scene.editable,
                        resettable: scene.resettable,
                        options: scene.options,
                        photo: scene.photo,
                    };
                    let modules = apertrust_modules::MODULES;
                    let mut actions = Vec::new();
                    ui.scope_builder(UiBuilder::new().max_rect(side), |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        actions = tab(
                            ui,
                            history,
                            modules,
                            &scene.hidden,
                            true,
                            false,
                            state,
                            today(),
                        );
                    });
                    scene.actions.extend(actions);
                },
                Scene {
                    options: options(history.as_ref(), true, false),
                    photo: None,
                    history,
                    editable: true,
                    resettable: true,
                    hidden: Vec::new(),
                    actions: Vec::new(),
                    panel: None,
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

    /// The list's line named `name`, or "`name`, what changed".
    fn item<'a>(harness: &'a Harness<'_, Scene>, name: &'a str) -> egui_kittest::Node<'a> {
        let named = |node: &egui_kittest::Node<'_>| {
            let label = node.accesskit_node().label().unwrap_or_default();
            label == name || label.starts_with(&format!("{name}, "))
        };
        (harness.query_all_by_role(AccessRole::TreeItem))
            .find(named)
            .unwrap_or_else(|| panic!("no line {name}"))
    }

    /// Whether a line named `name` is shown.
    fn shows(harness: &Harness<'_, Scene>, name: &str) -> bool {
        (harness.query_all_by_role(AccessRole::TreeItem)).any(|node| {
            let label = node.accesskit_node().label().unwrap_or_default();
            label == name || label.starts_with(&format!("{name}, "))
        })
    }

    /// Opens or closes the group named `name` with a click on its chevron.
    fn chevron(harness: &mut Harness<'_, Scene>, name: &str) {
        let line = item(harness, name).rect();
        let at = pos2(line.left() + 56.0, line.center().y);
        harness.drag_at(at);
        harness.drop_at(at);
        harness.run();
    }

    #[test]
    fn a_click_goes_to_a_step_and_shift_click_opens_its_module() {
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
        // Shift-click opens the module, of a step, the end's too; Original has none.
        let shift_click = |harness: &mut Harness<'_, Scene>, name: &str| {
            item(harness, name).click_modifiers(egui::Modifiers::SHIFT);
            harness.run();
        };
        shift_click(&mut harness, "Step 2: Grain");
        shift_click(&mut harness, "Step 3: Exposure");
        shift_click(&mut harness, "Original");
        assert_eq!(
            harness.state().actions,
            [Action::Open("grain"), Action::Open("exposure")]
        );
        // On a photo that takes no edit, a click goes nowhere; Shift-click still opens.
        harness.state_mut().actions.clear();
        harness.state_mut().editable = false;
        harness.run();
        item(&harness, "Step 1: Exposure").click();
        harness.run();
        item(&harness, "Step 2: Grain").click();
        harness.run();
        shift_click(&mut harness, "Step 2: Grain");
        assert_eq!(harness.state().actions, [Action::Open("grain")]);
    }

    /// The eye named `name`, if drawn: whether it is a toggle.
    fn eye(harness: &Harness<'_, Scene>, name: &str) -> Option<Option<Toggled>> {
        (harness.query_by_role_and_label(AccessRole::Button, name))
            .map(|node| node.accesskit_node().toggled())
    }

    /// Whether any eye is drawn.
    fn any_eye(harness: &Harness<'_, Scene>) -> bool {
        (harness.query_all_by_role(AccessRole::Button)).any(|node| {
            let label = node.accesskit_node().label().unwrap_or_default();
            label.starts_with("Hold to see the photo without ")
        })
    }

    /// The modules the eye holds in the frames a run draws, each once, once the events
    /// sent before it are taken in: the harness may take a button's in the frame after.
    fn peeks(harness: &mut Harness<'_, Scene>) -> Vec<&'static str> {
        harness.run();
        harness.state_mut().actions.clear();
        harness.run();
        let actions = std::mem::take(&mut harness.state_mut().actions);
        let mut held: Vec<&'static str> = (actions.into_iter())
            .filter_map(|action| match action {
                Action::Peek(module) => Some(module),
                _ => None,
            })
            .collect();
        held.dedup();
        held
    }

    /// The primary button pressed or let go at `at`.
    fn button(harness: &mut Harness<'_, Scene>, at: egui::Pos2, pressed: bool) {
        harness.event(egui::Event::PointerMoved(at));
        harness.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }

    /// The eye at the end of a step's line: none at rest; under the pointer, a plain button
    /// of one name, not a toggle, its tooltip saying nothing is saved, on a photo that takes
    /// no edit too. Held by the pointer, it asks to see the photo without the module each
    /// frame, and let go, nothing; a press that leaves the eye, or a window that loses the
    /// focus, lets it go. With the line's focus, and with its own after Tab, where Space
    /// held holds it.
    #[test]
    fn the_eye_held_shows_the_photo_without_a_step_s_module() {
        const GRAIN: &str = "Hold to see the photo without Grain";
        const EXPOSURE: &str = "Hold to see the photo without Exposure";
        let mut harness = photo_scene(3);
        harness.state_mut().editable = false;
        harness.run();
        assert!(!any_eye(&harness));
        item(&harness, "Step 2: Grain").hover();
        harness.run();
        assert_eq!(eye(&harness, GRAIN), Some(None));
        let node = harness.get_by_role_and_label(AccessRole::Button, GRAIN);
        let tip = node.accesskit_node().description();
        assert_eq!(
            tip.as_deref(),
            Some("Hold to see the photo without Grain; nothing is saved")
        );
        let at = node.rect().center();
        let line = item(&harness, "Step 2: Grain").rect().center();
        assert!(peeks(&mut harness).is_empty());
        // Held, then let go.
        button(&mut harness, at, true);
        assert_eq!(peeks(&mut harness), ["grain"]);
        assert_eq!(peeks(&mut harness), ["grain"]);
        // Its name stays the same while the session reports the module hidden.
        harness.state_mut().hidden = vec!["grain"];
        harness.run();
        assert_eq!(eye(&harness, GRAIN), Some(None));
        button(&mut harness, at, false);
        harness.state_mut().hidden.clear();
        assert!(peeks(&mut harness).is_empty());
        // Held, the pointer leaving the eye with the button down lets it go.
        button(&mut harness, at, true);
        assert_eq!(peeks(&mut harness), ["grain"]);
        harness.event(egui::Event::PointerMoved(line));
        assert!(peeks(&mut harness).is_empty());
        button(&mut harness, line, false);
        harness.run();
        // Held, the window losing the focus lets it go.
        button(&mut harness, at, true);
        assert_eq!(peeks(&mut harness), ["grain"]);
        harness.event(egui::Event::WindowFocused(false));
        harness.input_mut().focused = false;
        assert!(peeks(&mut harness).is_empty());
        // Back with the button still down, as when another program took its release: no
        // hold until a new press, leaving the eye and coming back over it included.
        harness.event(egui::Event::WindowFocused(true));
        harness.input_mut().focused = true;
        assert!(peeks(&mut harness).is_empty());
        harness.event(egui::Event::PointerMoved(line));
        assert!(peeks(&mut harness).is_empty());
        harness.event(egui::Event::PointerMoved(at));
        assert!(peeks(&mut harness).is_empty());
        button(&mut harness, at, false);
        harness.run();
        button(&mut harness, at, true);
        assert_eq!(peeks(&mut harness), ["grain"]);
        button(&mut harness, at, false);
        harness.run();
        // The keys: the line's focus shows it, Tab moves the focus onto it, and Space held
        // there holds it.
        harness.remove_cursor();
        harness.run();
        assert!(!any_eye(&harness));
        item(&harness, "Step 3: Exposure").focus();
        harness.run();
        assert!(eye(&harness, EXPOSURE).is_some());
        harness.key_press(Key::Tab);
        harness.run();
        harness.run();
        let focused = harness.get_by_role_and_label(AccessRole::Button, EXPOSURE);
        assert!(focused.is_focused());
        harness.key_down(Key::Space);
        assert_eq!(peeks(&mut harness), ["exposure"]);
        harness.key_up(Key::Space);
        assert!(peeks(&mut harness).is_empty());
        let focused = harness.get_by_role_and_label(AccessRole::Button, EXPOSURE);
        assert!(focused.is_focused());
    }

    /// A press on a line's eye before the eye is drawn, the pointer arriving and pressing in
    /// one frame, as a tap does, is the eye's: it peeks, and its release goes to no step.
    #[test]
    fn a_press_on_an_eye_not_yet_drawn_goes_to_no_step() {
        let mut harness = photo_scene(3);
        harness.remove_cursor();
        harness.run();
        assert!(!any_eye(&harness));
        let line = item(&harness, "Step 2: Grain").rect();
        let at = egui::pos2(line.right() - EYE / 2.0, line.center().y);
        harness.state_mut().actions.clear();
        // Both in one frame: the harness gives each event it queues a frame of its own.
        let events = &mut harness.input_mut().events;
        events.push(egui::Event::PointerMoved(at));
        events.push(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        });
        harness.step();
        let held = std::mem::take(&mut harness.state_mut().actions);
        assert_eq!(held, [Action::Peek("grain")]);
        button(&mut harness, at, false);
        harness.run();
        let actions = &harness.state().actions;
        assert!(
            !actions
                .iter()
                .any(|action| matches!(action, Action::GoTo(_))),
            "{actions:?}"
        );
    }

    /// A tap on a line's eye, the pointer arriving, pressing and letting go in one frame,
    /// goes to no step.
    #[test]
    fn a_tap_on_an_eye_goes_to_no_step() {
        let mut harness = photo_scene(3);
        harness.remove_cursor();
        harness.run();
        let line = item(&harness, "Step 2: Grain").rect();
        let at = egui::pos2(line.right() - EYE / 2.0, line.center().y);
        harness.state_mut().actions.clear();
        // All in one frame: the harness gives each event it queues a frame of its own.
        let events = &mut harness.input_mut().events;
        events.push(egui::Event::PointerMoved(at));
        for pressed in [true, false] {
            events.push(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        harness.run();
        let actions = &harness.state().actions;
        assert!(
            !actions
                .iter()
                .any(|action| matches!(action, Action::GoTo(_))),
            "{actions:?}"
        );
    }

    /// A held eye's press is its own: the list scrolled under the button held puts another
    /// line's eye under the press, which is not held, and nothing is.
    #[test]
    fn a_list_scrolled_under_a_held_eye_holds_no_other() {
        let steps = (1..=30)
            .map(|n| step(n, &[if n % 2 == 0 { "grain" } else { "exposure" }], None))
            .collect();
        let mut harness = scene(
            Theme::default(),
            egui::vec2(640.0, 480.0),
            Some(history(40, steps, None, None)),
        );
        harness.run();
        let line = item(&harness, "Step 20: Grain").rect();
        item(&harness, "Step 20: Grain").hover();
        harness.run();
        let at = (harness
            .get_by_role_and_label(AccessRole::Button, "Hold to see the photo without Grain"))
        .rect()
        .center();
        button(&mut harness, at, true);
        assert_eq!(peeks(&mut harness), ["grain"]);
        // A line's height up, Step 19's eye under the press.
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -line.height()),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers: egui::Modifiers::NONE,
        });
        assert!(peeks(&mut harness).is_empty());
        let under = item(&harness, "Step 19: Exposure").rect();
        assert!(under.contains(at), "{under:?} under {at:?}");
        assert!(eye(&harness, "Hold to see the photo without Exposure").is_some());
        button(&mut harness, at, false);
        harness.run();
    }

    /// The eye as the board draws it: the pointer on the first step's eye, its tooltip
    /// shown; then held, Exposure's name struck through on each of its lines while the
    /// session reports it hidden.
    #[test]
    fn the_eye_is_photographed() {
        let mut results = egui_kittest::SnapshotResults::new();
        let mut harness = scene(
            Theme::builtin(Builtin::Dark),
            egui::vec2(640.0, 480.0),
            Some(photo_s(3)),
        );
        harness.run();
        item(&harness, "Step 1: Exposure").hover();
        harness.run();
        let at = harness
            .get_by_role_and_label(AccessRole::Button, "Hold to see the photo without Exposure")
            .rect()
            .center();
        harness.event(egui::Event::PointerMoved(at));
        harness.run();
        harness.snapshot("history-tab-eye");
        results.extend(harness.take_snapshot_results());
        button(&mut harness, at, true);
        harness.state_mut().hidden = vec!["exposure"];
        harness.run();
        harness.snapshot("history-tab-eye-held");
        results.extend(harness.take_snapshot_results());
    }

    /// No eye on Reset's line, a group's or the Defaults', nor on Original; a default step
    /// inside the Defaults has one.
    #[test]
    fn the_eye_is_on_a_step_of_one_module_alone() {
        let mut harness = photo_scene(3);
        harness.run();
        for name in ["Step 4: Reset", "Original"] {
            item(&harness, name).hover();
            harness.run();
            assert!(!any_eye(&harness), "{name}");
        }
        let mut harness = scene(
            Theme::default(),
            egui::vec2(320.0, 900.0),
            Some(with_defaults(styled(6), 4)),
        );
        harness.run();
        for name in ["Steps 12 to 14: Warm film", "Steps 1 to 9: Defaults"] {
            item(&harness, name).hover();
            harness.run();
            assert!(!any_eye(&harness), "{name}");
        }
        item(&harness, "Step 1: Raw black/white point").hover();
        harness.run();
        let name = "Hold to see the photo without Raw black/white point";
        assert!(eye(&harness, name).is_some());
    }

    /// The default steps are inside the Defaults' group, closed at first: a click on the
    /// group goes to its last step, as a batch's does, a click on one inside it to it, and
    /// Shift-click opens its module.
    #[test]
    fn a_click_on_the_defaults_goes_to_their_last_step() {
        let mut harness = scene(
            Theme::default(),
            egui::vec2(320.0, 640.0),
            Some(with_defaults(photo_s(5), DEFAULT_STEPS + 3)),
        );
        harness.run();
        let group = "Steps 1 to 9: Defaults";
        assert!(!shows(&harness, "Step 1: Raw black/white point"));
        item(&harness, group).click();
        harness.run();
        chevron(&mut harness, group);
        item(&harness, "Step 1: Raw black/white point").click();
        harness.run();
        item(&harness, "Step 2: Demosaic").click_modifiers(egui::Modifiers::SHIFT);
        harness.run();
        assert_eq!(
            harness.state().actions,
            [
                Action::GoTo(Some(StepId(0xd8))),
                Action::GoTo(Some(StepId(0xd0))),
                Action::Open("demosaic"),
            ]
        );
    }

    /// A line's hover band says it takes a click or a Shift-click: a step always, its
    /// Shift-click opening its module; Original only while a click on it goes somewhere,
    /// not at end 0 nor on a photo that takes no edit.
    #[test]
    fn the_hover_band_marks_a_line_that_takes_a_click_or_a_shift_click() {
        let lines = |end| lines_in(Some(&photo_s(end)), Locale::EnGb);
        let named = |lines: &[Line], name: &str| {
            (lines.iter().find(|line| line.name == name).cloned()).expect(name)
        };
        let at_0 = lines(0);
        assert!(!named(&at_0, "Original").takes(true));
        assert!(named(&at_0, "Grain").takes(true));
        assert!(named(&at_0, "Grain").takes(false));
        let at_3 = lines(3);
        assert!(named(&at_3, "Original").takes(true));
        assert!(!named(&at_3, "Original").takes(false));
        assert!(named(&at_3, "Grain").takes(false));
    }

    /// At end 0, a click on Original changes nothing: the end is there already, Original
    /// chosen, the default steps' group undone with the photo's steps.
    #[test]
    fn at_end_0_a_click_on_original_goes_nowhere() {
        let mut harness = scene(
            Theme::default(),
            egui::vec2(320.0, 640.0),
            Some(with_defaults(photo_s(5), 0)),
        );
        harness.run();
        let selected =
            |harness: &Harness<'_, Scene>, name| item(harness, name).accesskit_node().is_selected();
        assert_eq!(selected(&harness, "Original"), Some(true));
        assert_eq!(selected(&harness, "Steps 1 to 9: Defaults"), Some(false));
        item(&harness, "Original").click();
        harness.run();
        assert!(harness.state().actions.is_empty());
        item(&harness, "Step 10: Exposure").click();
        harness.run();
        assert_eq!(harness.state().actions, [Action::GoTo(Some(StepId(0xa1)))]);
    }

    /// What the options offer, from what `History` tells: Compress where a step is not a
    /// default one, Truncate where one lies after the end, Discard where the history is not
    /// a new photo's; none where the photo takes no edits, Discard alone where its history
    /// cannot be read.
    #[test]
    fn the_options_offer_only_what_the_history_shows_they_can_do() {
        let options = |history: &History, takes, unreadable| {
            let o = options(Some(history), takes, unreadable);
            (o.compress, o.truncate, o.discard)
        };
        let steps = photo_s(3);
        let last = steps.steps.len();
        assert_eq!(options(&steps, true, false), (true, true, true));
        // At the last step, nothing after the end to drop.
        let at_last = History {
            end: last,
            ..steps.clone()
        };
        assert_eq!(options(&at_last, true, false), (true, false, true));
        // At Original every step is after the end.
        assert_eq!(
            options(&with_defaults(photo_s(0), 0), true, false),
            (true, true, true)
        );
        // A new photo's default steps alone, the end after them: nothing to do. The end
        // among them: Discard applies them all again.
        let defaults = |end| History {
            steps: with_defaults(photo_s(0), 0).steps[..DEFAULT_STEPS].to_vec(),
            end,
            ..photo_s(0)
        };
        assert_eq!(
            options(&defaults(DEFAULT_STEPS), true, false),
            (false, false, false)
        );
        assert_eq!(options(&defaults(4), true, false), (false, false, true));
        // Not live, or open for viewing only: nothing, but Discard on an unreadable one.
        assert_eq!(options(&steps, false, false), (false, false, false));
        assert_eq!(options(&steps, false, true), (false, false, true));
        let none = super::options(None, false, true);
        assert_eq!(
            (none.compress, none.truncate, none.discard),
            (false, false, true)
        );
        assert_eq!(super::options(None, true, false), Options::default());
    }

    /// Opens the header's options, ⋯.
    fn open_options(harness: &mut Harness<'_, Scene>) {
        harness
            .get_by_role_and_label(AccessRole::Button, "History options")
            .click();
        harness.run();
    }

    /// Holds Option (Alt), or lets it go.
    fn hold_option(harness: &mut Harness<'_, Scene>, held: bool) {
        let modifiers = if held {
            egui::Modifiers::ALT
        } else {
            egui::Modifiers::NONE
        };
        harness.event(egui::Event::ModifiersChanged(modifiers));
        harness.run();
    }

    /// The header's options: Compress History, Create a Style… disabled and Discard
    /// History…; Option held shows Truncate History in Compress's place, where a step lies
    /// after the end, and each sends its action.
    #[test]
    fn the_options_compress_truncate_with_option_held_and_discard() {
        let mut harness = photo_scene(3);
        harness.run();
        open_options(&mut harness);
        let compress = harness.get_by_role_and_label(AccessRole::MenuItem, "Compress History");
        assert!(!compress.accesskit_node().is_disabled());
        let style = harness.get_by_role_and_label(AccessRole::MenuItem, "Create a Style…");
        assert!(style.accesskit_node().is_disabled());
        harness
            .get_by_role_and_label(AccessRole::MenuItem, "Compress History")
            .click();
        harness.run();
        assert_eq!(harness.state().actions, [Action::Compress]);
        harness.state_mut().actions.clear();
        open_options(&mut harness);
        hold_option(&mut harness, true);
        assert!(harness.query_by_label("Compress History").is_none());
        harness
            .get_by_role_and_label(AccessRole::MenuItem, "Truncate History")
            .click_modifiers(egui::Modifiers::ALT);
        harness.run();
        assert_eq!(harness.state().actions, [Action::Truncate]);
        hold_option(&mut harness, false);
        harness.state_mut().actions.clear();
        open_options(&mut harness);
        harness
            .get_by_role_and_label(AccessRole::MenuItem, "Discard History…")
            .click();
        harness.run();
        assert_eq!(harness.state().actions, [Action::Discard]);
        // With nothing after the end, Option shows Compress History still.
        harness.state_mut().actions.clear();
        let last = photo_s(3).steps.len();
        harness.state_mut().options = options(Some(&photo_s(last)), true, false);
        open_options(&mut harness);
        hold_option(&mut harness, true);
        harness
            .get_by_role_and_label(AccessRole::MenuItem, "Compress History")
            .click();
        harness.run();
        assert_eq!(harness.state().actions, [Action::Compress]);
        hold_option(&mut harness, false);
        // Where nothing can run, each is disabled and a click sends nothing.
        harness.state_mut().actions.clear();
        harness.state_mut().options = Options::default();
        open_options(&mut harness);
        for name in ["Compress History", "Discard History…"] {
            let item = harness.get_by_role_and_label(AccessRole::MenuItem, name);
            assert!(item.accesskit_node().is_disabled(), "{name}");
            item.click();
            harness.run();
        }
        assert!(harness.state().actions.is_empty());
    }

    /// A menu opened for one photo closes as another opens, and chooses nothing for it.
    #[test]
    fn the_options_menu_closes_as_another_photo_opens() {
        let mut harness = photo_scene(3);
        harness.state_mut().photo = Some(1);
        harness.run();
        open_options(&mut harness);
        assert!(harness.query_by_label("Compress History").is_some());
        harness.state_mut().photo = Some(2);
        harness.run();
        harness.run();
        assert!(harness.query_by_label("Compress History").is_none());
        assert!(harness.state().actions.is_empty());
        // Opened for this photo, it chooses for it.
        open_options(&mut harness);
        harness
            .get_by_role_and_label(AccessRole::MenuItem, "Compress History")
            .click();
        harness.run();
        assert_eq!(harness.state().actions, [Action::Compress]);
    }

    /// Discard History…'s tip where no alert can be shown: the setting's way out where the
    /// edit reads whole, none where it cannot be read, which always asks.
    #[test]
    fn where_no_alert_can_be_shown_discard_says_why() {
        let blocked = |unreadable| Options {
            no_alert: true,
            unreadable,
            ..Options::default()
        };
        assert_eq!(discard_why(Options::default()), None);
        assert_eq!(
            discard_why(blocked(false)),
            Some("history-discard-no-alert")
        );
        assert_eq!(
            discard_why(blocked(true)),
            Some("history-discard-no-alert-unreadable")
        );
    }

    /// The options as the board draws them, and with Option held.
    #[test]
    fn the_options_are_photographed() {
        let mut results = egui_kittest::SnapshotResults::new();
        let mut harness = scene(
            Theme::builtin(Builtin::Dark),
            egui::vec2(480.0, 360.0),
            Some(photo_s(3)),
        );
        harness.run();
        open_options(&mut harness);
        harness.snapshot("history-tab-options");
        results.extend(harness.take_snapshot_results());
        hold_option(&mut harness, true);
        harness.snapshot("history-tab-options-truncate");
        results.extend(harness.take_snapshot_results());
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
        // Home the newest line, End Original, and neither leaves the list.
        let focused = |harness: &Harness<'_, Scene>, name| item(harness, name).is_focused();
        harness.key_press(egui::Key::End);
        harness.run();
        assert!(focused(&harness, "Original"));
        harness.key_press(egui::Key::ArrowDown);
        harness.run();
        assert!(focused(&harness, "Original"));
        harness.key_press(egui::Key::Home);
        harness.run();
        assert!(focused(&harness, "Step 5: Exposure"));
        harness.key_press(egui::Key::ArrowUp);
        harness.run();
        assert!(focused(&harness, "Step 5: Exposure"));
        // → opens the Defaults, the default steps' group, and goes into them open; ← goes
        // back to their line and closes them.
        let mut harness = scene(
            Theme::default(),
            egui::vec2(320.0, 640.0),
            Some(with_defaults(photo_s(5), DEFAULT_STEPS + 5)),
        );
        harness.run();
        let group = "Steps 1 to 9: Defaults";
        item(&harness, "Step 14: Exposure").focus();
        harness.run();
        harness.key_press(egui::Key::End);
        harness.run();
        harness.key_press(egui::Key::ArrowUp);
        harness.run();
        assert!(focused(&harness, group));
        harness.key_press(egui::Key::ArrowRight);
        harness.run();
        assert!(shows(&harness, "Step 9: AgX"));
        assert!(focused(&harness, group));
        harness.key_press(egui::Key::ArrowRight);
        harness.run();
        assert!(focused(&harness, "Step 9: AgX"));
        harness.key_press(egui::Key::ArrowDown);
        harness.run();
        assert!(focused(&harness, "Step 8: Colour calibration"));
        // Shift+Enter opens the line's module.
        harness.state_mut().actions.clear();
        harness.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Enter);
        harness.run();
        assert_eq!(harness.state().actions, [Action::Open("channelmixerrgb")]);
        harness.key_press(egui::Key::ArrowLeft);
        harness.run();
        assert!(focused(&harness, group));
        harness.key_press(egui::Key::ArrowLeft);
        harness.run();
        assert!(!shows(&harness, "Step 9: AgX"));
        assert!(focused(&harness, group));
    }

    #[test]
    fn screen_readers_hear_each_line_s_name_tooltip_and_the_end() {
        let mut harness = photo_scene(3);
        harness.run();
        let list = harness.get_by_role_and_label(AccessRole::Tree, "History");
        // Today's heading, the five steps, the undone mark under the two undone, and
        // Original.
        assert_eq!(list.children().count(), 8);
        harness.get_by_role_and_label(AccessRole::Heading, "Today");
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
        // The Defaults' line, the default steps' group: a top item, level 0, expanded or
        // not, its numbers, name and how many, its default steps a level down.
        let mut harness = scene(
            Theme::default(),
            egui::vec2(320.0, 640.0),
            Some(with_defaults(photo_s(5), DEFAULT_STEPS + 3)),
        );
        harness.run();
        let group = "Steps 1 to 9: Defaults";
        let defaults = item(
            &harness,
            "Steps 1 to 9: Defaults, 9 modules every photo runs",
        );
        assert_eq!(defaults.accesskit_node().level(), Some(0));
        assert_eq!(defaults.accesskit_node().data().is_expanded(), Some(false));
        assert_eq!(
            defaults.accesskit_node().description().as_deref(),
            Some("Default, 21/10/2026, 17:52")
        );
        chevron(&mut harness, group);
        let defaults = item(&harness, group);
        assert_eq!(defaults.accesskit_node().data().is_expanded(), Some(true));
        let default = item(&harness, "Step 7: Exposure");
        assert_eq!(default.accesskit_node().level(), Some(1));
        assert_eq!(
            default.accesskit_node().description().as_deref(),
            Some("Default, 21/10/2026, 17:52")
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
                .get_by_role_and_label(AccessRole::Tree, "History")
                .children()
                .count(),
            1
        );
    }

    /// The lines are worded once for a history, a language and a day, and again when any
    /// changes: the end moved, a step described anew, another language, another day.
    #[test]
    fn the_lines_are_worded_once_for_each_history_language_and_day() {
        let ctx = egui::Context::default();
        let modules = apertrust_modules::MODULES;
        let (en, uk) = (Translator::new(Locale::EnGb), Translator::new(Locale::Uk));
        let history = photo_s(3);
        let kept_now =
            |history: &History, tr: &Translator| kept(&ctx, Some(history), modules, tr, today());
        let first = kept_now(&history, &en);
        assert!(Arc::ptr_eq(&first, &kept_now(&history, &en)));
        let moved = History {
            end: 2,
            ..history.clone()
        };
        let again = kept_now(&moved, &en);
        assert!(!Arc::ptr_eq(&first, &again));
        assert!(again.lines().nth(3).is_some_and(Line::end));
        let mut anew = moved.clone();
        anew.steps[0] = Arc::new((*anew.steps[0]).clone());
        let described = kept_now(&anew, &en);
        assert!(!Arc::ptr_eq(&again, &described));
        let ukrainian = kept_now(&anew, &uk);
        assert_eq!(
            ukrainian.lines().nth(4).map(|line| line.name.as_str()),
            Some("Експозиція")
        );
        // The next day, its heading says Yesterday.
        let tomorrow = kept(&ctx, Some(&anew), modules, &uk, today() + 1);
        assert!(!Arc::ptr_eq(&ukrainian, &tomorrow));
        assert_eq!(
            tomorrow.entries.first(),
            Some(&Entry::Day("Учора, 21 жовтня".to_owned()))
        );
    }

    /// The section as the board draws it: each line saying what changed, the rail, the
    /// end mid-list with the undone mark under the two undone steps and the second step's
    /// tooltip shown under it, in the dark and light themes; and at an older step, the
    /// first, four undone over it.
    #[test]
    fn the_history_section_is_photographed() {
        let mut results = egui_kittest::SnapshotResults::new();
        for (which, name, end, hovered) in [
            (Builtin::Dark, "history-tab-dark", 3, true),
            (Builtin::Light, "history-tab-light", 3, true),
            (Builtin::Dark, "history-tab-older", 1, false),
        ] {
            let mut harness = scene(
                Theme::builtin(which),
                egui::vec2(640.0, 480.0),
                Some(photo_s(end)),
            );
            harness.run();
            if hovered {
                item(&harness, "Step 2: Grain").hover();
                harness.run();
            }
            harness.snapshot(name);
            results.extend(harness.take_snapshot_results());
        }
    }

    /// The numbers' column: wide enough for a two-digit range by default, wider once the
    /// highest step's number needs it; each number centred in it, and the names starting
    /// after the widest.
    #[test]
    fn the_numbers_are_centred_in_a_column_as_wide_as_the_widest() {
        let base = photo_s(0);
        let with = |count: usize| {
            let steps = (0..count)
                .map(|i| Arc::clone(&base.steps[i % base.steps.len()]))
                .collect();
            let history = History {
                steps,
                end: count,
                ..base.clone()
            };
            listed_in(Some(&history), Locale::EnGb)
        };
        // 104 steps, the last four a batch: its range, 101–104, is the widest.
        let batched_at = |count: usize| {
            // The steps cycled, as `with` does, with ids of their own.
            let steps: Vec<Arc<HistoryStep>> = (0..count)
                .map(|i| {
                    let step = &base.steps[i % base.steps.len()];
                    Arc::new(HistoryStep {
                        id: StepId(0x100 + i as u64),
                        batch: (i + 4 >= count).then_some(BatchId(0xb9)),
                        ..(**step).clone()
                    })
                })
                .collect();
            let history = History {
                steps,
                end: count,
                ..base.clone()
            };
            listed_in(Some(&history), Locale::EnGb)
        };
        let lines = [with(9), with(99), with(150), with(12_345), batched_at(104)];
        let mut harness = Harness::builder()
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
            .build_ui_state(
                move |ui, measured: &mut Vec<(Columns, Vec<f32>)>| {
                    if !measured.is_empty() {
                        return;
                    }
                    let t = theme(ui.ctx());
                    let font = FontId::proportional(t.text.caption);
                    let width_of = |text: &str| {
                        let galley = ui.painter().layout_no_wrap(
                            text.to_owned(),
                            font.clone(),
                            t.colors.text,
                        );
                        galley.size().x
                    };
                    let tr = Translator::new(Locale::EnGb);
                    for listed in &lines {
                        let columns = Columns::of(ui.ctx(), listed, &tr);
                        let widths = ["9", "99", "150", "12345", "12\u{2013}16", "101\u{2013}104"]
                            .map(width_of)
                            .to_vec();
                        measured.push((columns, widths));
                    }
                },
                Vec::new(),
            );
        harness.run();
        let measured = &harness.state()[..];
        let (default, widths) = (measured[0].0, &measured[0].1);
        // Up to 99, and here past it, the two-digit range sets the width.
        assert_eq!(default.width, widths[4]);
        assert_eq!(measured[1].0, default);
        assert_eq!(measured[2].0, default);
        // A number wider than it widens the column, and the names move by as much.
        let wide = measured[3].0;
        assert_eq!(wide.width, widths[3]);
        assert!(wide.name() - default.name() == wide.width - default.width);
        // A batch's range wider than the default widens it too.
        assert!(widths[5] > widths[4]);
        assert_eq!(measured[4].0.width, widths[5]);
        for (columns, widths) in [(default, &widths[..3]), (wide, &widths[..4])] {
            let centre = NUMBERS + columns.width / 2.0;
            for &width in widths {
                let start = columns.number_at(width);
                assert!(
                    (start + width / 2.0 - centre).abs() < 1e-3,
                    "{width} off centre"
                );
                assert!(start >= NUMBERS - 1e-3, "{width} starts before the column");
                assert!(start + width + NAME_GAP <= columns.name() + 1e-3);
            }
        }
    }

    /// What a line shows in a given room: the whole where it fits, else its longest start
    /// with … after it, the space before the … dropped, and nothing where not even one
    /// character and … fit, so a lone … never paints over the tag.
    /// A name gets the room it needs where what changed fits beside it, or a third of the
    /// room is left; else it is cut, leaving what changed up to a third of the room.
    #[test]
    fn a_name_cut_leaves_part_of_what_changed_in_sight() {
        // Both fit.
        assert_eq!(name_room(120.0, 60.0, Some(40.0)), 120.0);
        // What changed is cut, the name whole, a third of the room left to it.
        assert_eq!(name_room(120.0, 80.0, Some(60.0)), 120.0);
        // The name too long: cut to leave a third, or what changed whole where less.
        assert_eq!(name_room(120.0, 110.0, Some(60.0)), 80.0);
        assert_eq!(name_room(120.0, 110.0, Some(20.0)), 100.0);
        // Nothing changed to say: the name has the room.
        assert_eq!(name_room(120.0, 200.0, None), 120.0);
    }

    #[test]
    fn a_line_is_cut_at_its_end_and_never_to_a_lone_ellipsis() {
        // A character a point.
        let width_of = |text: &str| text.chars().count() as f32;
        let shown = |text: &str, room: f32| fit(text, room, width_of);
        assert_eq!(shown("Strength ↑", 10.0).as_deref(), Some("Strength ↑"));
        assert_eq!(shown("Strength ↑", 9.0).as_deref(), Some("Strength…"));
        assert_eq!(shown("Strength ↑", 5.0).as_deref(), Some("Stre…"));
        assert_eq!(shown("Strength ↑", 2.0).as_deref(), Some("S…"));
        assert_eq!(shown("Strength ↑", 1.0), None);
        assert_eq!(shown("Strength ↑", 0.0), None);
        assert_eq!(shown("Strength ↑", -3.0), None);
        assert_eq!(shown("↑", 1.0).as_deref(), Some("↑"));
        assert_eq!(shown("↑", 0.5), None);
        // A space before the cut is dropped.
        assert_eq!(shown("Two controls", 5.0).as_deref(), Some("Two…"));
    }

    /// The section at the side panel's narrowest, 240 points: a line's change cut, the
    /// undone mark cut with its whole words in its tooltip, and a tag kept whole.
    #[test]
    fn at_the_narrowest_side_panel_the_lines_and_the_mark_are_cut() {
        let mut harness = scene(
            Theme::builtin(Builtin::Dark),
            egui::vec2(480.0, 480.0),
            Some(photo_s(3)),
        );
        harness.state_mut().panel = Some(240.0);
        harness.run();
        harness.snapshot("history-tab-narrow");
        let note = (harness.query_all_by_role(AccessRole::Note).next()).expect("the mark");
        assert_eq!(
            note.accesskit_node().label().as_deref(),
            Some("2 undone: your next edit replaces them")
        );
        note.hover();
        harness.run();
        harness.get_by_label("↑ 2 undone · your next edit replaces them");
    }

    /// Each line says what changed after its module's name, the control with a direction:
    /// a number's label and ↑ or ↓, the arrow alone where it is the module's own control,
    /// no arrow where it did not move; any other control its label; several how many;
    /// Reset and a default step nothing. Screen readers hear the values
    /// before and after in the step's name.
    #[test]
    fn a_line_says_what_changed_and_which_way() {
        let tr = Translator::new(Locale::EnGb);
        let said = |module, controls| {
            let step = made(1, StepSource::You, None, vec![change_of(module, controls)]);
            summary(&step, apertrust_modules::MODULES, &tr)
        };
        let shown = |text: &str| Some(text.to_owned());
        // The module's own control: the arrow alone.
        assert_eq!(
            said("exposure", vec![number("exposure", 0.7, 1.2)]),
            shown("↑")
        );
        assert_eq!(
            said("exposure", vec![number("exposure", 1.2, 0.7)]),
            shown("↓")
        );
        assert_eq!(
            said("grain", vec![number("strength", 25.0, 12.0)]),
            shown("Strength ↓")
        );
        // Kept at its value: the label, no arrow; the module's own, nothing.
        assert_eq!(
            said("grain", vec![number("strength", 25.0, 25.0)]),
            shown("Strength")
        );
        assert_eq!(said("exposure", vec![number("exposure", 0.7, 0.7)]), None);
        // A choice and a toggle: the label alone.
        let choice = ControlChange {
            control: "mode",
            before: ControlValue::Choice(1),
            after: ControlValue::Choice(0),
            mark: [None, None],
        };
        assert_eq!(said("exposure", vec![choice]), shown("Mode"));
        let curve = ControlChange {
            control: "luma_points",
            before: ControlValue::Points(Vec::new()),
            after: ControlValue::Points(vec![[0.5, 0.6]]),
            mark: [None, Some(Mark::Kept)],
        };
        let curved = said("tone_curve", vec![curve]).expect("its label");
        assert!(!curved.contains(['↑', '↓']), "{curved}");
        // Several: how many, in each language.
        let two = vec![
            number("exposure", 0.7, 1.2),
            number("black_level", 0.0, 0.01),
        ];
        assert_eq!(said("exposure", two.clone()), shown("2 controls"));
        let step = made(1, StepSource::You, None, vec![change_of("exposure", two)]);
        let pl = Translator::new(Locale::Pl);
        let uk = Translator::new(Locale::Uk);
        let modules = apertrust_modules::MODULES;
        assert_eq!(summary(&step, modules, &pl).as_deref(), Some("2 parametry"));
        assert_eq!(summary(&step, modules, &uk).as_deref(), Some("2 параметри"));
        // Reset, a Default step and a newer build's module: nothing.
        let reset = made(
            2,
            StepSource::You,
            Some(Note::reset()),
            vec![
                change_of("exposure", Vec::new()),
                change_of("grain", Vec::new()),
            ],
        );
        assert_eq!(summary(&reset, modules, &tr), None);
        let applied = made(
            3,
            StepSource::Default,
            None,
            vec![change_of("exposure", vec![number("exposure", 0.0, 0.7)])],
        );
        assert_eq!(summary(&applied, modules, &tr), None);
        assert_eq!(said("lens_2030", Vec::new()), None);
        // An operation's unowned parameter, with no definition: its name, as the tooltip's.
        assert_eq!(
            said("colorin", vec![number("gamma", 1.0, 1.2)]),
            shown("gamma ↑")
        );
        let lines = lines_in(Some(&photo_s(3)), Locale::EnGb);
        assert!(
            (lines.iter())
                .all(|line| line.number().is_some() || line.group() || line.change.is_none())
        );
        // Screen readers: the values before and after in the step's name.
        let mut harness = photo_scene(3);
        harness.run();
        harness.get_by_role_and_label(
            AccessRole::TreeItem,
            "Step 1: Exposure, +0.700 EV to +1.200 EV",
        );
        harness.get_by_role_and_label(
            AccessRole::TreeItem,
            "Step 2: Grain, Strength, 25.00% to 30.00%",
        );
        harness.get_by_role_and_label(AccessRole::TreeItem, "Step 4: Reset");
    }

    /// The rail as the board draws it: a dot a step, the end a larger dot, Reset its glyph,
    /// undone steps hollow on a dashed rail; and the undone mark, its words in each
    /// language, under the last undone step.
    #[test]
    fn the_rail_and_the_undone_mark_show_where_the_edit_stands() {
        let n = DEFAULT_STEPS;
        let listed = listed_in(Some(&with_defaults(photo_s(5), n + 3)), Locale::EnGb);
        let closed = HashSet::new();
        let rows = drawn(&listed, &closed);
        let rails: Vec<Rail> = (rows.iter())
            .filter_map(|row| match row {
                Drawn::Line(shown) => Some(shown.rail),
                _ => None,
            })
            .collect();
        let nodes: Vec<RailNode> = rails.iter().map(|rail| rail.node).collect();
        let group = |end, undone| RailNode::Group { end, undone };
        assert_eq!(
            nodes,
            [
                RailNode::Undone,
                RailNode::Reset { end: false },
                RailNode::End,
                RailNode::Step,
                RailNode::Step,
                group(false, false),
                RailNode::Step,
            ]
        );
        assert_eq!(
            (rails[0].above, rails[0].below),
            (RailLine::None, RailLine::Dashed)
        );
        assert_eq!(rails[2].above, RailLine::Dashed);
        assert_eq!(rails[2].below, RailLine::Solid);
        assert_eq!(rails.last().map(|rail| rail.below), Some(RailLine::None));
        // Today's heading first, no rail over it; the mark under the second line, the
        // last undone; the gap over the Defaults, the rail solid through it.
        assert!(matches!(rows[0], Drawn::Day("Today")));
        assert_eq!(through(&rows, 0), RailLine::None);
        assert!(matches!(rows[3], Drawn::Undone));
        assert_eq!(listed.undone, 2);
        let gap = (rows.iter().position(|row| matches!(row, Drawn::Gap))).expect("the gap");
        assert_eq!(through(&rows, gap), RailLine::Solid);
        // At end 0 every step is undone, the default steps among them, and Original is
        // the end.
        let at_0 = listed_in(Some(&with_defaults(photo_s(5), 0)), Locale::EnGb);
        let node_of = |open: &HashSet<GroupKey>, name: &str| {
            (drawn(&at_0, open).into_iter())
                .find_map(|row| match row {
                    Drawn::Line(shown) if shown.line.name == name => Some(shown.rail.node),
                    _ => None,
                })
                .expect(name)
        };
        assert_eq!(node_of(&closed, "Defaults"), group(false, true));
        let defaults = HashSet::from([GroupKey::Defaults]);
        assert_eq!(node_of(&defaults, "AgX"), RailNode::Undone);
        assert_eq!(node_of(&defaults, "Original"), RailNode::End);
        assert_eq!(at_0.undone, n + 5);
        let mut harness = photo_scene(3);
        harness.run();
        // To screen readers a note, its words with no arrow or dot.
        harness.get_by_role_and_label(AccessRole::Note, "2 undone: your next edit replaces them");
        let words = |locale, count: usize| {
            let mut args = Args::new();
            args.set("count", count);
            Translator::new(locale).text_with("history-undone", Some(&args))
        };
        assert_eq!(
            words(Locale::EnGb, 1),
            "↑ 1 undone · your next edit replaces it"
        );
        // Ukrainian's pronoun follows the number itself: "його" for 1 alone, not 21.
        assert!(words(Locale::Uk, 1).ends_with("його замінить"));
        assert!(words(Locale::Uk, 21).ends_with("їх замінить"));
        assert_eq!(
            words(Locale::Uk, 3),
            "↑ 3 скасовані · наступна зміна їх замінить"
        );
        assert_eq!(
            words(Locale::Pl, 5),
            "↑ 5 cofniętych · następna edycja je zastąpi"
        );
    }

    /// A day in milliseconds.
    const DAY_MS: u64 = 24 * 60 * 60 * 1000;

    /// A step of `source` with `note` made `days` before [`wireframe_s_time`], its
    /// `changes` each a module's, in `batch` where there is one.
    fn dated(
        id: u64,
        days: u64,
        batch: Option<u64>,
        source: StepSource,
        note: Option<Note>,
        changes: Vec<ModuleChange>,
    ) -> Arc<HistoryStep> {
        Arc::new(HistoryStep {
            id: StepId(id),
            clock: Clock::at(wireframe_s_time() - days * DAY_MS, None, MachineId::UNKNOWN),
            source,
            note,
            batch: batch.map(BatchId),
            changes,
        })
    }

    /// A photo's history over two days, the end at `end`: yesterday Exposure and Grain;
    /// today a style, Warm film, on Colour balance RGB, Grain and Vignetting, one batch,
    /// steps 3 to 5, then Exposure again.
    fn styled(end: usize) -> History {
        let one = |module, control, before, after| {
            vec![change_of(module, vec![number(control, before, after)])]
        };
        let warm = || Some(Note::Text("Warm film".to_owned()));
        let steps = vec![
            dated(
                0xc1,
                1,
                None,
                StepSource::You,
                None,
                one("exposure", "exposure", 0.0, 0.7),
            ),
            dated(
                0xc2,
                1,
                None,
                StepSource::You,
                None,
                one("grain", "strength", 25.0, 12.0),
            ),
            dated(
                0xc3,
                0,
                Some(0xb1),
                StepSource::Style,
                warm(),
                vec![change_of(
                    "colorbalancergb",
                    vec![number("vibrance", 0.0, 0.2), number("contrast", 0.0, 0.1)],
                )],
            ),
            dated(
                0xc4,
                0,
                Some(0xb1),
                StepSource::Style,
                warm(),
                one("grain", "strength", 12.0, 30.0),
            ),
            dated(
                0xc5,
                0,
                Some(0xb1),
                StepSource::Style,
                warm(),
                one("vignette", "brightness", 0.0, -0.2),
            ),
            dated(
                0xc6,
                0,
                None,
                StepSource::You,
                None,
                one("exposure", "exposure", 0.7, 0.9),
            ),
        ];
        History {
            through: 9,
            steps,
            end,
            undo: None,
            redo: None,
        }
    }

    fn styled_scene(end: usize) -> Harness<'static, Scene> {
        scene(
            Theme::builtin(Builtin::Dark),
            egui::vec2(640.0, 480.0),
            Some(styled(end)),
        )
    }

    /// A batch is one closed line: its note, its numbers as a range, its modules' titles,
    /// its source, a square on the rail, and its steps inside it, a level down.
    #[test]
    fn a_batch_is_one_closed_group_of_its_steps() {
        let listed = listed_in(Some(&styled(6)), Locale::EnGb);
        let tr = Translator::new(Locale::EnGb);
        let group = (listed.entries.iter())
            .find_map(|entry| match entry {
                Entry::Group {
                    key: GroupKey::Batch(_),
                    line,
                    lines,
                    holds_end,
                    ..
                } => Some((line, lines, *holds_end)),
                _ => None,
            })
            .expect("the style's group");
        let (line, lines, holds_end) = group;
        assert_eq!(line.name, "Warm film");
        assert_eq!(line.numbers(&tr).as_deref(), Some("3\u{2013}5"));
        assert_eq!(
            line.change.as_deref(),
            Some("Colour balance RGB, Grain, Vignetting")
        );
        assert_eq!(line.tag.as_deref(), Some("Style"));
        assert_eq!(
            line.node,
            RailNode::Group {
                end: false,
                undone: false
            }
        );
        assert!(!holds_end);
        let numbers: Vec<Option<usize>> = lines.iter().map(Line::number).collect();
        assert_eq!(numbers, [Some(5), Some(4), Some(3)]);
        // Its tooltip: each step's rows under its module's title, then the note, who and
        // when.
        let Tip::Changes { modules, by } = &line.tip else {
            panic!("a group's tooltip has its changes")
        };
        let headings: Vec<Option<&str>> = (modules.iter())
            .map(|changed| changed.heading.as_deref())
            .collect();
        assert_eq!(
            headings,
            [
                Some("Colour balance RGB"),
                Some("Grain"),
                Some("Vignetting")
            ]
        );
        assert_eq!(by[..2], ["Warm film".to_owned(), "Style".to_owned()]);
        // A batch without a note: how many modules, in each language.
        let plain = History {
            steps: (styled(6).steps.into_iter())
                .map(|step| {
                    Arc::new(HistoryStep {
                        note: None,
                        ..(*step).clone()
                    })
                })
                .collect(),
            ..styled(6)
        };
        let named = |locale| {
            let listed = listed_in(Some(&plain), locale);
            (listed.lines())
                .find(|line| matches!(line.place, Place::Batch { .. }))
                .map(|line| line.name.clone())
        };
        assert_eq!(named(Locale::EnGb).as_deref(), Some("3 modules"));
        assert_eq!(named(Locale::Uk).as_deref(), Some("3 модулі"));
        assert_eq!(named(Locale::Pl).as_deref(), Some("3 moduły"));
        let mut harness = styled_scene(6);
        harness.run();
        let header = item(&harness, "Steps 3 to 5: Warm film");
        assert_eq!(header.accesskit_node().level(), Some(0));
        assert!(!shows(&harness, "Step 4: Grain"));
    }

    /// A click on a batch's line goes to its last step, as Undo moves it whole; its
    /// chevron, or →, opens it, its steps under it, and the end inside it shows there.
    #[test]
    fn a_group_opens_on_its_chevron_and_a_click_goes_to_its_last_step() {
        let mut harness = styled_scene(2);
        harness.run();
        item(&harness, "Steps 3 to 5: Warm film").click();
        harness.run();
        assert_eq!(harness.state().actions, [Action::GoTo(Some(StepId(0xc5)))]);
        harness.state_mut().actions.clear();
        chevron(&mut harness, "Steps 3 to 5: Warm film");
        assert!(harness.state().actions.is_empty());
        let inside = item(&harness, "Step 4: Grain");
        assert_eq!(inside.accesskit_node().level(), Some(1));
        let header = item(&harness, "Steps 3 to 5: Warm film");
        assert_eq!(header.accesskit_node().data().is_expanded(), Some(true));
        // A step inside goes to itself; ← on it goes to its group's line, ← there closes it.
        item(&harness, "Step 4: Grain").click();
        harness.run();
        assert_eq!(harness.state().actions, [Action::GoTo(Some(StepId(0xc4)))]);
        item(&harness, "Step 4: Grain").focus();
        harness.run();
        harness.key_press(egui::Key::ArrowLeft);
        harness.run();
        assert!(item(&harness, "Steps 3 to 5: Warm film").is_focused());
        harness.key_press(egui::Key::ArrowLeft);
        harness.run();
        assert!(!shows(&harness, "Step 4: Grain"));
        // At its last step, the closed line is the end, chosen.
        let mut harness = styled_scene(5);
        harness.run();
        let header = item(&harness, "Steps 3 to 5: Warm film");
        assert_eq!(header.accesskit_node().is_selected(), Some(true));
        assert!(!shows(&harness, "Step 5: Vignetting"));
    }

    /// The end landing inside a group opens it, and the step there is chosen; the group
    /// stays as the person leaves it until the end moves again.
    #[test]
    fn a_group_opens_itself_when_the_end_lands_inside_it() {
        let mut harness = styled_scene(6);
        harness.run();
        assert!(!shows(&harness, "Step 4: Grain"));
        harness.state_mut().history = Some(styled(4));
        harness.run();
        let end = item(&harness, "Step 4: Grain");
        assert_eq!(end.accesskit_node().is_selected(), Some(true));
        assert_eq!(
            item(&harness, "Steps 3 to 5: Warm film")
                .accesskit_node()
                .is_selected(),
            Some(false)
        );
        chevron(&mut harness, "Steps 3 to 5: Warm film");
        assert!(!shows(&harness, "Step 4: Grain"));
        harness.run();
        assert!(!shows(&harness, "Step 4: Grain"));
        // Closed with the end inside, its line is the end's, chosen, its square the end's,
        // and the undone mark under it counts the steps out of sight with those over it.
        let header = item(&harness, "Steps 3 to 5: Warm film");
        assert_eq!(header.accesskit_node().is_selected(), Some(true));
        let line = header.rect();
        let note = harness
            .get_by_role_and_label(AccessRole::Note, "2 undone: your next edit replaces them");
        assert!(note.rect().top() >= line.bottom() - 0.5);
        let listed = listed_in(Some(&styled(4)), Locale::EnGb);
        let closed = HashSet::new();
        let node = (drawn(&listed, &closed).into_iter())
            .find_map(|row| match row {
                Drawn::Line(shown) if shown.line.name == "Warm film" => Some(shown.rail.node),
                _ => None,
            })
            .expect("the group's line");
        assert_eq!(
            node,
            RailNode::Group {
                end: true,
                undone: false
            }
        );
        // A click on it still goes to its last step.
        item(&harness, "Steps 3 to 5: Warm film").click();
        harness.run();
        assert_eq!(harness.state().actions, [Action::GoTo(Some(StepId(0xc5)))]);
    }

    /// Sends a screen reader's `action` to the line named `name`.
    fn act(harness: &Harness<'_, Scene>, name: &str, action: egui::accesskit::Action) {
        let line = item(harness, name);
        let (target_node, target_tree) = line.accesskit_node().locate();
        harness.event(egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                target_node,
                target_tree,
                action,
                data: None,
            },
        ));
    }

    /// Screen readers open a group with Expand and close it with Collapse, which it offers
    /// by its state; their Click goes to its last step, as a click on the line does, and
    /// opens nothing.
    #[test]
    fn screen_readers_open_and_close_a_group_by_its_own_actions() {
        use egui::accesskit::Action as Ask;
        let mut harness = styled_scene(6);
        harness.run();
        let group = "Steps 3 to 5: Warm film";
        let offers = |harness: &Harness<'_, Scene>, ask| {
            item(harness, group)
                .accesskit_node()
                .data()
                .supports_action(ask)
        };
        assert!(offers(&harness, Ask::Expand));
        assert!(!offers(&harness, Ask::Collapse));
        act(&harness, group, Ask::Expand);
        harness.run();
        assert!(shows(&harness, "Step 4: Grain"));
        assert!(offers(&harness, Ask::Collapse));
        assert!(harness.state().actions.is_empty());
        item(&harness, group).click_accesskit();
        harness.run();
        assert_eq!(harness.state().actions, [Action::GoTo(Some(StepId(0xc5)))]);
        assert!(shows(&harness, "Step 4: Grain"));
        act(&harness, group, Ask::Collapse);
        harness.run();
        assert!(!shows(&harness, "Step 4: Grain"));
        // A step's line offers neither.
        let step = item(&harness, "Step 6: Exposure");
        assert!(!step.accesskit_node().data().supports_action(Ask::Expand));
    }

    /// The keys that have nothing to do keep the focus where it is: → on a line with
    /// nothing inside, ← on a top line, ← on a closed group.
    #[test]
    fn the_arrows_with_nothing_to_do_keep_the_focus() {
        let mut harness = styled_scene(6);
        harness.run();
        let focused = |harness: &Harness<'_, Scene>, name| item(harness, name).is_focused();
        item(&harness, "Step 6: Exposure").focus();
        harness.run();
        for key in [egui::Key::ArrowRight, egui::Key::ArrowLeft] {
            harness.key_press(key);
            harness.run();
            assert!(focused(&harness, "Step 6: Exposure"), "{key:?}");
        }
        let group = "Steps 3 to 5: Warm film";
        item(&harness, group).focus();
        harness.run();
        harness.key_press(egui::Key::ArrowLeft);
        harness.run();
        assert!(focused(&harness, group));
        assert!(!shows(&harness, "Step 4: Grain"));
        assert!(harness.state().actions.is_empty());
    }

    /// Screen readers say where a line is among its level's, "2 of 6", a group's lines
    /// among their group's, the headings, the gap and the mark not counted.
    #[test]
    fn each_line_says_where_it_is_among_its_level_s() {
        let mut harness = styled_scene(6);
        harness.run();
        chevron(&mut harness, "Steps 3 to 5: Warm film");
        let tree = harness.get_by_role_and_label(AccessRole::Tree, "History");
        // Step 6, the batch, steps 2 and 1, and Original.
        assert_eq!(tree.accesskit_node().data().size_of_set(), Some(5));
        let position = |name| {
            item(&harness, name)
                .accesskit_node()
                .data()
                .position_in_set()
        };
        assert_eq!(position("Step 6: Exposure"), Some(0));
        assert_eq!(position("Steps 3 to 5: Warm film"), Some(1));
        assert_eq!(position("Step 2: Grain"), Some(2));
        assert_eq!(position("Original"), Some(4));
        // The batch's three lines, in a group of their own.
        let group = harness.get_by_role(AccessRole::Group);
        assert_eq!(group.accesskit_node().data().size_of_set(), Some(3));
        assert_eq!(group.children().count(), 3);
        assert_eq!(position("Step 5: Vignetting"), Some(0));
        assert_eq!(position("Step 3: Colour balance RGB"), Some(2));
    }

    /// A heading over each day's steps: Today, Yesterday with its date, then the date
    /// alone, with its year where it is another year's; named for screen readers, never
    /// focused.
    #[test]
    fn a_heading_names_each_day_the_edit_was_worked_on() {
        let listed = listed_in(Some(&styled(6)), Locale::EnGb);
        let days: Vec<&str> = (listed.entries.iter())
            .filter_map(|entry| match entry {
                Entry::Day(day) => Some(day.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(days, ["Today", "Yesterday, 20 October"]);
        let tr = Translator::new(Locale::EnGb);
        let day = today();
        assert_eq!(heading(day - 6, day, &tr), "15 October");
        assert_eq!(heading(day - 300, day, &tr), "25 December 2025");
        let uk = Translator::new(Locale::Uk);
        assert_eq!(heading(day, day, &uk), "Сьогодні");
        assert_eq!(heading(day - 1, day, &uk), "Учора, 20 жовтня");
        let pl = Translator::new(Locale::Pl);
        assert_eq!(heading(day - 1, day, &pl), "Wczoraj, 20 października");
        // A step whose time is unknown, a version 1 edit's, has none over it.
        let unknown = listed_in(
            Some(&history(1, vec![step(1, &["exposure"], None)], None, None)),
            Locale::EnGb,
        );
        assert!(!unknown.entries.iter().any(|e| matches!(e, Entry::Day(_))));
        // Under a day's steps, steps of unknown time have a heading of their own, never
        // the newer day's.
        let mixed = History {
            steps: vec![
                dated(0xd1, 1, None, StepSource::You, None, Vec::new()),
                step(0xd2, &["exposure"], None),
                dated(0xd3, 0, None, StepSource::You, None, Vec::new()),
            ],
            end: 3,
            ..styled(6)
        };
        let days: Vec<String> = (listed_in(Some(&mixed), Locale::EnGb).entries.into_iter())
            .filter_map(|entry| match entry {
                Entry::Day(day) => Some(day),
                _ => None,
            })
            .collect();
        assert_eq!(days, ["Today", "No date", "Yesterday, 20 October"]);
        let mut harness = styled_scene(6);
        harness.run();
        harness.get_by_role_and_label(AccessRole::Heading, "Yesterday, 20 October");
    }

    /// A batch, closed and open, under today's heading and over yesterday's, and the
    /// Defaults open with the end on the fourth default step, as the board draws them.
    #[test]
    fn the_groups_and_the_days_are_photographed() {
        let mut results = egui_kittest::SnapshotResults::new();
        let mut harness = styled_scene(6);
        harness.run();
        harness.snapshot("history-tab-group");
        results.extend(harness.take_snapshot_results());
        chevron(&mut harness, "Steps 3 to 5: Warm film");
        harness.remove_cursor();
        harness.run();
        harness.snapshot("history-tab-group-open");
        results.extend(harness.take_snapshot_results());
        let mut harness = scene(
            Theme::builtin(Builtin::Dark),
            egui::vec2(640.0, 480.0),
            Some(with_defaults(styled(6), 4)),
        );
        harness.run();
        // Open of itself, the end inside it.
        harness.remove_cursor();
        harness.run();
        harness.snapshot("history-tab-defaults");
        results.extend(harness.take_snapshot_results());
    }

    /// A history longer than the panel scrolled to its end: History's header stays at the
    /// top, the list under it, with a hairline and a shadow under the header.
    #[test]
    fn the_header_stays_at_the_top_as_a_long_history_scrolls() {
        let mut long = styled(6);
        let base = long.steps.clone();
        long.steps = (0..30)
            .map(|i| {
                let step = &base[i % base.len()];
                Arc::new(HistoryStep {
                    id: StepId(0x1000 + i as u64),
                    batch: step
                        .batch
                        .map(|batch| BatchId(batch.0 + (i / base.len()) as u64)),
                    clock: Clock::at(wireframe_s_time(), None, MachineId::UNKNOWN),
                    ..(**step).clone()
                })
            })
            .collect();
        long.end = 30;
        let mut harness = scene(
            Theme::builtin(Builtin::Dark),
            egui::vec2(640.0, 480.0),
            Some(long),
        );
        harness.run();
        let top = |harness: &Harness<'_, Scene>| {
            (harness.get_by_role_and_label(AccessRole::Button, "History"))
                .rect()
                .top()
        };
        let before = top(&harness);
        // The end, at the top, is in sight; End goes to Original, scrolled into sight.
        item(&harness, "Step 30: Exposure").focus();
        harness.run();
        harness.key_press(egui::Key::End);
        harness.run();
        assert!(item(&harness, "Original").is_focused());
        harness.remove_cursor();
        harness.run();
        harness.snapshot("history-tab-scrolled");
        assert_eq!(top(&harness), before);
        let first = item(&harness, "Step 30: Exposure").rect();
        assert!(first.top() < before, "{first:?} scrolled under the header");
    }
}
