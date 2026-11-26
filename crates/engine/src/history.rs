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

//! The edit's history (decision 78, `docs/history.md`): a [`Log`] of [`Step`]s and its
//! end, the edit folded from it, and what a client is told of it, [`History`].
//!
//! A step is one tool's change, or one operation's unowned parameters', as darktable's
//! history item is one tool's: its whole state after it ([`Entry`]); an empty entry is
//! the reset state, the tool's defaults, or off for a switched one (decision 32). Reset
//! is the one step of several, an empty entry for each tool it reset ([`Change`]). A
//! completed change that touches several tools makes a step for each, sharing a
//! [`BatchId`], which Undo and Redo move whole, as darktable's undo group. The edit is the
//! fold of the log: each tool's last entry among the steps up to the end, else its reset
//! state, so nothing else stores it. The before of a step is derived, never stored: the
//! tool's entry at the previous step that touched it, else its reset state. A control's
//! quick repeats merge into the step before them ([`Log::merge`]), as darktable's undo
//! merges them; the session decides when.
//!
//! A photo opened with no history is given its default steps ([`Log::add_defaults`]): a
//! step of source Default of each of darktable's default modules, at its reset state, as
//! darktable adds its default modules and auto-applied presets to a new raw's history.
//! They are steps 1 to n; Undo's fallback stops at them ([`Log::defaults`]). Below its
//! default step a tool darktable leaves off until its workflow applies it is not applied
//! ([`Log::unapplied`]), so the end within them shows the photo with the first k applied.
//!
//! Compress keeps the default steps and each tool's last step up to the end
//! ([`Log::compressed`]), as darktable's compress history stack; Truncate drops the person's
//! steps after the end ([`Log::truncated`]), as its Ctrl-click on it. Neither changes what
//! the photo shows at the end or at Original but at end 0, where both drop every step. An
//! edit whose history cannot be read is started afresh as a new photo's default steps and
//! a step of each tool it holds ([`Log::afresh`]), as Discard History does there.
//!
//! The styles applied to the edit (decision 106) are one more part of the log: a step that
//! changed them holds the whole list after it ([`Step::styles`]), a change of the list
//! alone a step of no tool ([`Change::None`]), and the edit's list is the one at the last
//! step up to the end holding one ([`Log::styles`]), so every move of the end moves it.
//!
//! Plain data, which the session keeps in memory for the photo open and the engine's
//! `sidecar` reads and writes in version 2, with the copy's clock vector ([`advance`]).

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub use apertrust_library::clock::{Clock, MachineId};
pub use apertrust_library::fields::RatingChange;
use apertrust_plugin::{ImageContext, Operation, ParamValues, Value};
use apertrust_tools::{Base, ControlValue, Controls, Mark, ProcessingTool, ToolValues};

use crate::edit::Edit;

/// A step's id: 64 random bits, so two logs joined step by step do not collide; drawn
/// again on a clash within its log. Written as 16 hex digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StepId(pub u64);

impl fmt::Display for StepId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

/// The session's id for the steps one completed change made, one per tool it touched,
/// and later on several photos (decision 37); never a client's. Drawn at random, again on
/// a clash within its log. Written as 16 hex digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BatchId(pub u64);

impl fmt::Display for BatchId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

/// Who made a step: the History tab's tag and the tooltip's who.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StepSource {
    /// The person, through any client's plain commands.
    You,
    /// Auto, or a colour picker's pick.
    Auto,
    /// A style applied.
    Style,
    /// An assistant, through MCP.
    Assistant,
    /// What the session applies to a new photo itself; no client sends it.
    Default,
}

impl StepSource {
    /// Its name, as a sidecar and the protocol write it.
    pub fn name(self) -> &'static str {
        match self {
            StepSource::You => "you",
            StepSource::Auto => "auto",
            StepSource::Style => "style",
            StepSource::Assistant => "assistant",
            StepSource::Default => "default",
        }
    }

    /// The source a sidecar names, `None` for a name this build does not know.
    pub fn from_name(name: &str) -> Option<StepSource> {
        [
            StepSource::You,
            StepSource::Auto,
            StepSource::Style,
            StepSource::Assistant,
            StepSource::Default,
        ]
        .into_iter()
        .find(|source| source.name() == name)
    }
}

/// What a step says of itself in place of the tools it touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    /// Apertrust's own words, as a message id with its arguments, worded by the client
    /// with Fluent.
    Message {
        id: String,
        args: Vec<(String, String)>,
    },
    /// An assistant's or a script's own words, stored as written, never translated.
    Text(String),
}

impl Note {
    /// The message id of [`Note::reset`].
    pub const RESET: &str = "history-reset";

    /// The message id of [`Note::discard`].
    pub const DISCARD: &str = "history-discard";

    /// The message id of [`Note::style`].
    pub const STYLE: &str = "history-style";

    /// The message id of [`Note::clear_style`].
    pub const CLEAR_STYLE: &str = "history-clear-style";

    /// The message id of [`Note::styles_applied`].
    pub const STYLES_APPLIED: &str = "history-styles-applied";

    /// The note of a style's steps (decision 106), with its name as the argument `style`.
    pub fn style(name: &str) -> Note {
        Note::Message {
            id: Note::STYLE.to_owned(),
            args: vec![("style".to_owned(), name.to_owned())],
        }
    }

    /// The note of a style's Clear, with its name as the argument `style`.
    pub fn clear_style(name: &str) -> Note {
        Note::Message {
            id: Note::CLEAR_STYLE.to_owned(),
            args: vec![("style".to_owned(), name.to_owned())],
        }
    }

    /// The note of the step Compress keeps the styles applied on ([`Log::compressed`]).
    pub fn styles_applied() -> Note {
        Note::Message {
            id: Note::STYLES_APPLIED.to_owned(),
            args: Vec::new(),
        }
    }

    /// The note of a reset of the whole edit, Reset's step.
    pub fn reset() -> Note {
        Note::Message {
            id: Note::RESET.to_owned(),
            args: Vec::new(),
        }
    }

    /// The note of the steps Discard History starts an edit whose history cannot be read
    /// afresh with ([`Log::afresh`]).
    pub fn discard() -> Note {
        Note::Message {
            id: Note::DISCARD.to_owned(),
            args: Vec::new(),
        }
    }
}

/// One tool, or the unowned parameters of one operation, with its whole state after the
/// step that holds it; empty, the reset state.
#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    Tool {
        tool: String,
        values: ToolValues,
    },
    /// Parameters no tool owns, while `Set` exists.
    Operation {
        operation: String,
        params: ParamValues,
    },
}

/// What an entry is of: its tool's or its operation's id. Layers add a layer and an
/// instance to it (decision 34).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Key<'a> {
    Tool(&'a str),
    Operation(&'a str),
}

impl Entry {
    /// The entries a change from `before` to `after` makes: each tool and operation whose
    /// state differs, with its whole state in `after`, in id order, tools first. An
    /// entry holding nothing and none at all are the same reset state.
    pub fn between(before: &Edit, after: &Edit) -> Vec<Entry> {
        let mut entries = Vec::new();
        let tools: BTreeSet<&str> = (before.tools.keys())
            .chain(after.tools.keys())
            .map(String::as_str)
            .collect();
        for tool in tools {
            let state = tool_state(after, tool);
            if tool_state(before, tool) != state {
                entries.push(Entry::Tool {
                    tool: tool.to_owned(),
                    values: state.cloned().unwrap_or_default(),
                });
            }
        }
        let operations: BTreeSet<&str> = (before.params.keys())
            .chain(after.params.keys())
            .map(String::as_str)
            .collect();
        for operation in operations {
            let state = params_state(after, operation);
            if params_state(before, operation) != state {
                entries.push(Entry::Operation {
                    operation: operation.to_owned(),
                    params: state.cloned().unwrap_or_default(),
                });
            }
        }
        entries
    }

    /// Whether it holds nothing: the reset state.
    pub fn is_empty(&self) -> bool {
        match self {
            Entry::Tool { values, .. } => values.is_empty(),
            Entry::Operation { params, .. } => params.iter().next().is_none(),
        }
    }

    pub(crate) fn key(&self) -> Key<'_> {
        match self {
            Entry::Tool { tool, .. } => Key::Tool(tool),
            Entry::Operation { operation, .. } => Key::Operation(operation),
        }
    }

    /// Lays it on `edit`: its state, or none for an empty entry.
    fn apply(&self, edit: &mut Edit) {
        match self {
            Entry::Tool { tool, values } if values.is_empty() => {
                edit.tools.remove(tool);
            }
            Entry::Tool { tool, values } => {
                edit.tools.insert(tool.clone(), values.clone());
            }
            Entry::Operation { operation, .. } if self.is_empty() => {
                edit.params.remove(operation);
            }
            Entry::Operation { operation, params } => {
                edit.params.insert(operation.clone(), params.clone());
            }
        }
    }
}

/// The tool's state in `edit`, none where it holds nothing.
fn tool_state<'e>(edit: &'e Edit, tool: &str) -> Option<&'e ToolValues> {
    edit.tools.get(tool).filter(|values| !values.is_empty())
}

/// The operation's unowned parameters in `edit`, none where it sets none.
fn params_state<'e>(edit: &'e Edit, operation: &str) -> Option<&'e ParamValues> {
    (edit.params.get(operation)).filter(|params| params.iter().next().is_some())
}

/// What one command on the edit wrote, in order, which orders the steps of the change it
/// is part of ([`Change::between`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wrote {
    /// A tool, by a control set or reset, a keep, a preset row or its reset.
    Tool(String),
    /// An operation's unowned parameters, by `Set`.
    Operation(String),
    /// Every tool and operation: `Reset`.
    All,
}

impl Wrote {
    fn key(&self) -> Option<Key<'_>> {
        match self {
            Wrote::Tool(tool) => Some(Key::Tool(tool)),
            Wrote::Operation(operation) => Some(Key::Operation(operation)),
            Wrote::All => None,
        }
    }
}

/// What a step holds: one tool's or one operation's entry, as darktable's history item
/// holds one tool, or Reset's, or none, for a step holding the styles applied alone.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    Entry(Entry),
    /// Reset, noted `history-reset`: an empty entry for each tool and operation it reset,
    /// in [`Entry::between`]'s order.
    Reset(Vec<Entry>),
    /// No tool: a change of the styles applied alone (decision 106), a Clear whose tools
    /// have all changed since, a style applied again whose tools hold its values already,
    /// or the step Compress keeps the list on; its [`Step::styles`] holds the list.
    None,
}

impl Change {
    /// The changes, one step each, that a completed change from `before` to `after` makes,
    /// `wrote` what its commands wrote in order: one for each tool and operation whose
    /// state differs, in the order they were first written after the last `Reset`; one
    /// Reset, in that `Reset`'s place, holding each empty entry no command wrote after it;
    /// then each entry no command wrote, as a tool's change of another's, in
    /// [`Entry::between`]'s order. None where nothing differs.
    pub fn between(before: &Edit, after: &Edit, wrote: &[Wrote]) -> Vec<Change> {
        let reset = wrote.iter().rposition(|wrote| *wrote == Wrote::All);
        let since = reset.map_or(0, |reset| reset + 1);
        let mut placed: Vec<(usize, Change)> = Vec::new();
        let mut resets = Vec::new();
        for entry in Entry::between(before, after) {
            let first = (wrote[since..].iter()).position(|wrote| wrote.key() == Some(entry.key()));
            match first {
                Some(first) => placed.push((since + first, Change::Entry(entry))),
                None if reset.is_some() && entry.is_empty() => resets.push(entry),
                None => placed.push((usize::MAX, Change::Entry(entry))),
            }
        }
        if let Some(reset) = reset
            && !resets.is_empty()
        {
            placed.push((reset, Change::Reset(resets)));
        }
        // Stable: those placed alike keep `Entry::between`'s order.
        placed.sort_by_key(|(at, _)| *at);
        placed.into_iter().map(|(_, change)| change).collect()
    }

    /// Its entries: one, Reset's, or none.
    pub fn entries(&self) -> &[Entry] {
        match self {
            Change::Entry(entry) => std::slice::from_ref(entry),
            Change::Reset(entries) => entries,
            Change::None => &[],
        }
    }
}

/// A style applied to the photo (decision 106, `docs/history.md`, The styles applied): its
/// name as applied, its file name in the styles folder then, and each tool it set with that
/// tool's whole state before it and after it, so that Clear can put back the tools that
/// still hold the style's values without its file.
#[derive(Debug, Clone, PartialEq)]
pub struct AppliedStyle {
    pub name: String,
    /// A file name, never a path.
    pub file: String,
    /// Each tool it set, in the order it set them.
    pub tools: Vec<StyleTool>,
}

/// A tool a style set: its whole state before the style and after it, either empty for its
/// reset state.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleTool {
    pub tool: String,
    pub before: ToolValues,
    pub after: ToolValues,
}

pub use apertrust_library::sidecar::BUILT_IN_FILE;

impl AppliedStyle {
    /// Whether it is the style `name` from `file` (decisions 106 and 109): by file where
    /// either its file or `file` is a built-in's, `builtin:<id>`, the same style only where
    /// both files are the same, so a built-in applied in another language is found and a
    /// person's style named as a built-in is never taken for it; by name where neither is.
    /// The sidecar's reading holds a list to the same rule, so a list the session leaves
    /// always reads ([`apertrust_library::sidecar::same_style`]).
    pub fn is(&self, name: &str, file: &str) -> bool {
        apertrust_library::sidecar::same_style((&self.name, &self.file), (name, file))
    }

    /// The style `name` from `file` applied over `earlier`, its item already on the list,
    /// `set` each tool this application set with its state just before it and after it
    /// (decision 106): a tool that still held what `earlier` set keeps `earlier`'s state
    /// before it, so Clear goes back to before the style first set it; every other tool
    /// takes its state just before; and `earlier`'s tools this application did not set come
    /// after, as they were.
    pub fn applied(
        name: &str,
        file: &str,
        earlier: Option<&AppliedStyle>,
        set: Vec<StyleTool>,
    ) -> AppliedStyle {
        let earlier_tool = |tool: &str| {
            (earlier.into_iter())
                .flat_map(|style| &style.tools)
                .find(|kept| kept.tool == tool)
        };
        let mut tools: Vec<StyleTool> = (set.into_iter())
            .map(|mut now| {
                if let Some(kept) = earlier_tool(&now.tool)
                    && kept.after == now.before
                {
                    now.before = kept.before.clone();
                }
                now
            })
            .collect();
        let others = (earlier.into_iter())
            .flat_map(|style| &style.tools)
            .filter(|kept| tools.iter().all(|now| now.tool != kept.tool))
            .cloned()
            .collect::<Vec<_>>();
        tools.extend(others);
        AppliedStyle {
            name: name.to_owned(),
            file: file.to_owned(),
            tools,
        }
    }
}

/// `list` with `style` at the top, any item that is the same style taken out first
/// ([`AppliedStyle::is`]).
pub fn on_top(list: &[AppliedStyle], style: AppliedStyle) -> Vec<AppliedStyle> {
    let mut list: Vec<AppliedStyle> = (list.iter())
        .filter(|kept| !kept.is(&style.name, &style.file))
        .cloned()
        .collect();
    list.insert(0, style);
    list
}

/// A newer log's list of styles applied, `rebuilt`, after one of the session's steps laid
/// on it that changed the session's list from `before` to `after` (decision 106, A newer
/// file taken in): each style cleared taken out where `rebuilt` holds it, and each style
/// added or moved to the top put at the top, in `after`'s order, any item of the same style
/// taken out first ([`AppliedStyle::is`]), with the session's after states. It is applied
/// again over the item `rebuilt` holds of the same style, by [`AppliedStyle::applied`]'s
/// rule: a tool it `set`, as the steps of
/// its change show or as its item differs from the one before, takes its state in the
/// joined log just before the step that set it, `joined`, or the earlier item's state
/// before it where it still held that item's after; a tool an earlier application set keeps
/// the earlier item's state before it, else the session's; and the earlier item's tools it
/// does not set stay, as they were.
pub(crate) fn replay(
    rebuilt: &mut Vec<AppliedStyle>,
    (before, after): (&[AppliedStyle], &[AppliedStyle]),
    stepped: &BTreeSet<String>,
    joined: impl Fn(&str) -> ToolValues,
) {
    let was = |item: &AppliedStyle| before.iter().find(|style| style.is(&item.name, &item.file));
    let cleared = |item: &AppliedStyle| {
        was(item).is_some() && after.iter().all(|now| !now.is(&item.name, &item.file))
    };
    rebuilt.retain(|kept| !cleared(kept));
    // The top before, of the styles still listed: one there only as those above it were
    // cleared was not moved.
    let top = before.iter().find(|style| !cleared(style));
    let moved: Vec<&AppliedStyle> = (after.iter().enumerate())
        .filter(|(at, style)| {
            let topped = *at == 0 && !top.is_some_and(|top| top.is(&style.name, &style.file));
            was(style) != Some(*style) || topped
        })
        .map(|(_, style)| style)
        .collect();
    for style in moved.into_iter().rev() {
        let earlier = (rebuilt.iter())
            .find(|kept| kept.is(&style.name, &style.file))
            .cloned();
        let previous = was(style);
        let tools = (style.tools.iter())
            .map(|tool| {
                let then = (previous.into_iter())
                    .flat_map(|previous| &previous.tools)
                    .find(|kept| kept.tool == tool.tool);
                let set = stepped.contains(&tool.tool) || then != Some(tool);
                let kept = (earlier.iter())
                    .flat_map(|earlier| &earlier.tools)
                    .find(|kept| kept.tool == tool.tool);
                let before = match (set, kept) {
                    (true, _) => joined(&tool.tool),
                    (false, Some(kept)) => kept.before.clone(),
                    (false, None) => tool.before.clone(),
                };
                StyleTool {
                    tool: tool.tool.clone(),
                    before,
                    after: tool.after.clone(),
                }
            })
            .collect();
        // Applied again over the item `rebuilt` holds, as the session applies one.
        let replayed = AppliedStyle::applied(&style.name, &style.file, earlier.as_ref(), tools);
        *rebuilt = on_top(rebuilt, replayed);
    }
}

/// One tool's change, or Reset (decision 78).
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub id: StepId,
    /// The hybrid logical clock of its last change (decision 47).
    pub clock: Clock,
    pub source: StepSource,
    pub note: Option<Note>,
    /// Shared by the steps of one completed change that touched several tools, which
    /// Undo and Redo move whole, and a style's on each of its steps, one step's too, as on
    /// every photo it changes (decision 109); `None` for a change of one tool but a style's.
    pub batch: Option<BatchId>,
    /// The tool it touched, with its whole state after it, or Reset's, or none.
    pub change: Change,
    /// The whole list of styles applied after it, newest first, where it changed the list,
    /// empty where it emptied it; `None` where it left the list as it was (decision 106).
    pub styles: Option<Vec<AppliedStyle>>,
}

/// One virtual copy's history: its steps in order and its end, how many of them apply,
/// darktable's `history_end`. End 0 is Original.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Log {
    steps: Vec<Step>,
    end: usize,
}

impl Log {
    /// The log of `steps`, oldest first, the first `end` of them applied, at most all.
    pub fn from_steps(steps: Vec<Step>, end: usize) -> Log {
        let end = end.min(steps.len());
        Log { steps, end }
    }

    /// Every step, oldest first, those after the end among them.
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// The styles applied to the edit, newest first (decision 106): the list at the last
    /// step up to the end that holds one, else none.
    pub fn styles(&self) -> &[AppliedStyle] {
        self.styles_at(self.end)
    }

    /// The styles applied with the first `end` steps applied, as [`Log::styles`].
    pub fn styles_at(&self, end: usize) -> &[AppliedStyle] {
        (self.steps[..end.min(self.steps.len())].iter().rev())
            .find_map(|step| step.styles.as_deref())
            .unwrap_or_default()
    }

    /// Puts `styles` on the last step applied, the list the change it ends leaves.
    pub fn hold_styles(&mut self, styles: Vec<AppliedStyle>) {
        if let Some(step) = self.steps[..self.end].last_mut() {
            step.styles = Some(styles);
        }
    }

    /// How many steps apply: 0 is Original, and the step numbered `end`, counting from 1,
    /// is the last applied.
    pub fn end(&self) -> usize {
        self.end
    }

    /// The edit: each tool's last entry among the steps up to the end, else its reset
    /// state.
    pub fn fold(&self) -> Edit {
        self.fold_to(self.end)
    }

    /// The edit with the first `end` steps applied, and the tools whose default step lies
    /// after them not applied where darktable leaves them off ([`Log::unapplied`]).
    pub(crate) fn fold_to(&self, end: usize) -> Edit {
        let mut edit = Edit::default();
        for step in &self.steps[..end] {
            for entry in step.change.entries() {
                entry.apply(&mut edit);
            }
        }
        edit.unapplied = self.unapplied(end);
        edit
    }

    /// The tools whose default step lies after the first `end` steps, with no entry among
    /// them, of those darktable leaves off until its workflow's auto-applied preset turns
    /// them on (`_dev_auto_apply_presets`, `src/develop/develop.c:1856`): Exposure, Colour
    /// calibration and the tone mapper. darktable's modules on by default run at
    /// their defaults below their item, as its `dt_dev_pop_history_items_ext` resets each
    /// module to `default_enabled` (`:1614-1636`), so they are applied.
    pub fn unapplied(&self, end: usize) -> BTreeSet<String> {
        let end = end.min(self.steps.len());
        let before: BTreeSet<Key<'_>> = (self.steps[..end].iter())
            .flat_map(|step| step.change.entries())
            .map(Entry::key)
            .collect();
        (self.steps[end..].iter())
            .filter(|step| step.source == StepSource::Default)
            .flat_map(|step| step.change.entries())
            .filter_map(|entry| match entry {
                Entry::Tool { tool, .. }
                    if !on_by_default(tool) && !before.contains(&entry.key()) =>
                {
                    Some(tool.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// Gives a log with no step its default steps: one of each of `tools`, in order,
    /// source Default with no note and no batch, its entry empty, the tool's reset state,
    /// at `clock`; the end after the last. A log that has steps is left as it is, so a
    /// tool a later build adds to `tools` runs at its defaults with no step, and the
    /// steps keep their numbers.
    pub fn add_defaults(&mut self, tools: &[&str], clock: Clock) {
        if !self.steps.is_empty() {
            return;
        }
        for tool in tools {
            let id = self.new_id();
            self.add(Step {
                id,
                clock,
                source: StepSource::Default,
                note: None,
                batch: None,
                change: Change::Entry(Entry::Tool {
                    tool: (*tool).to_owned(),
                    values: ToolValues::new(),
                }),
                styles: None,
            });
        }
    }

    /// How many default steps the log begins with: those of source Default before any
    /// other, n of them, 1 to n, the History tab's Defaults group. Undo's fallback stops
    /// above them, and above any default step after them ([`Log::undo_end`]).
    pub fn defaults(&self) -> usize {
        (self.steps.iter())
            .take_while(|step| step.source == StepSource::Default)
            .count()
    }

    /// Adds `step` at the end, the steps after the end dropped first, as darktable's
    /// `_dev_add_history_item_ext` removes the items above `history_end`, but for a
    /// default step of a tool with no entry up to the end, which is kept and moved below
    /// it, before `step`, as darktable keeps an always-on module's item
    /// (`src/develop/develop.c:1191-1229`). darktable drops its auto-applied items there;
    /// Apertrust keeps every default step on purpose, the tools every photo runs never
    /// being off (decision 32). Returns the steps that were after the end, the kept among them, which
    /// Undo puts back in their place.
    pub fn add(&mut self, step: Step) -> Vec<Step> {
        let after = self.steps.split_off(self.end);
        if !after.iter().any(|step| step.source == StepSource::Default) {
            self.steps.push(step);
            self.end = self.steps.len();
            return after;
        }
        let before: BTreeSet<Key<'_>> = (self.steps.iter())
            .flat_map(|step| step.change.entries())
            .map(Entry::key)
            .collect();
        let kept: Vec<Step> = (after.iter())
            .filter(|step| {
                step.source == StepSource::Default
                    && (step.change.entries().iter()).all(|entry| !before.contains(&entry.key()))
            })
            .cloned()
            .collect();
        self.steps.extend(kept);
        self.steps.push(step);
        self.end = self.steps.len();
        after
    }

    /// Adds the steps of one completed change, `changes` ([`Change::between`]), of `source`
    /// with `note`, at `clock`: a step of each, each in `batch`, the caller's, as a command
    /// on several photos gives every step of every photo its one batch (decision 109), else
    /// sharing a new batch where there are several; a Reset's noted `history-reset`,
    /// holding the empty list of styles applied where the list was not empty (decision 106).
    /// The first drops the steps after the end, which it returns, as [`Log::add`] does.
    pub fn add_change(
        &mut self,
        changes: Vec<Change>,
        source: StepSource,
        note: Option<Note>,
        batch: Option<BatchId>,
        clock: Clock,
    ) -> Vec<Step> {
        let batch = batch.or_else(|| (changes.len() > 1).then(|| self.new_batch()));
        let mut dropped = Vec::new();
        for change in changes {
            let (note, styles) = match change {
                Change::Reset(_) => {
                    let emptied = !self.styles().is_empty();
                    (Some(Note::reset()), emptied.then(Vec::new))
                }
                Change::Entry(_) | Change::None => (note.clone(), None),
            };
            let step = Step {
                id: self.new_id(),
                clock,
                source,
                note,
                batch,
                change,
                styles,
            };
            // Only the first drops any: the rest go on at the end.
            dropped.extend(self.add(step));
        }
        dropped
    }

    /// Merges a change of one tool into the last step applied, as darktable's undo merges
    /// a widget's quick repeats (`docs/history.md`, Merging consecutive changes), where that
    /// step holds one entry of the same tool or operation, is in no batch and changed no
    /// list of styles applied: the step
    /// keeps its id, `entry`, the change's, takes the place of its entry, and its clock
    /// becomes `clock`. Whether it merged; with no such step, nothing.
    pub fn merge(&mut self, entry: &Entry, clock: Clock) -> bool {
        let Some(step) = self.steps[..self.end].last_mut() else {
            return false;
        };
        match &mut step.change {
            Change::Entry(kept)
                if step.batch.is_none() && step.styles.is_none() && kept.key() == entry.key() =>
            {
                *kept = entry.clone();
                step.clock = clock;
                true
            }
            _ => false,
        }
    }

    /// The log Compress leaves, darktable's compress history stack
    /// (`dt_history_compress_on_image`, `src/common/history.c:1193`): every default step,
    /// wherever the end is, so the photo at each end and at Original shows what it showed;
    /// and among the person's steps up to the end, each tool's and operation's last,
    /// with its id, clock, source, note and batch, one whose entries are all empty dropped,
    /// Reset's among them, as an empty entry and none are the same reset state; their steps
    /// after the end dropped, as darktable deletes the items above `history_end`
    /// (`:1244-1255`). The end is the last step kept up to it. At end 0 nothing is kept, as
    /// darktable deletes the whole history (`:1212-1216`).
    ///
    /// The list of styles applied (decision 106) is taken off every step kept and, where the
    /// edit's is not empty, written on a step of its own after the last kept up to the end,
    /// the end then on it: no tool, source Style, noted `history-styles-applied`, with a new
    /// id and `clock`, Compress's. Keeping the step that held the list instead would keep its
    /// tool's entry over a later one Compress drops. A log whose end is on such a step
    /// already, holding the edit's list, keeps it as it is, so Compress again changes
    /// nothing.
    pub fn compressed(&self, clock: Clock) -> Log {
        if self.end == 0 {
            return Log::default();
        }
        let applied = &self.steps[..self.end];
        let styles = self.styles();
        let listed = (applied.last()).filter(|step| {
            step.change == Change::None
                && step.source == StepSource::Style
                && step.note == Some(Note::styles_applied())
                && step.styles.as_deref() == Some(styles)
        });
        let mut last: BTreeMap<Key<'_>, usize> = BTreeMap::new();
        for (at, step) in applied.iter().enumerate() {
            for entry in step.change.entries() {
                last.insert(entry.key(), at);
            }
        }
        let mut kept: Vec<Step> = (applied.iter().enumerate())
            .filter(|(at, step)| {
                step.source == StepSource::Default
                    || (step.change.entries().iter())
                        .any(|entry| last.get(&entry.key()) == Some(at) && !entry.is_empty())
            })
            .map(|(_, step)| Step {
                styles: None,
                ..step.clone()
            })
            .collect();
        match listed {
            Some(listed) if !styles.is_empty() => kept.push(listed.clone()),
            _ if !styles.is_empty() => kept.push(Step {
                id: self.new_id(),
                clock,
                source: StepSource::Style,
                note: Some(Note::styles_applied()),
                batch: None,
                change: Change::None,
                styles: Some(styles.to_vec()),
            }),
            _ => {}
        }
        let end = kept.len();
        Log::from_steps(kept.into_iter().chain(self.defaults_after()).collect(), end)
    }

    /// The log Truncate leaves, darktable's Ctrl-click on compress
    /// (`dt_history_truncate_on_image`, `src/common/history.c:1348-1399`): the person's
    /// steps after the end dropped (`:1366-1376`), every step up to it kept as it is, and
    /// the default steps after it, so the photo shows what it showed, the end where it
    /// is. At end 0 nothing is kept, as darktable deletes every item (`:1357-1362`).
    pub fn truncated(&self) -> Log {
        if self.end == 0 {
            return Log::default();
        }
        let steps = (self.steps[..self.end].iter().cloned()).chain(self.defaults_after());
        Log::from_steps(steps.collect(), self.end)
    }

    /// The default steps after the end, which leave their tools not applied there
    /// ([`Log::unapplied`]).
    fn defaults_after(&self) -> impl Iterator<Item = Step> + '_ {
        (self.steps[self.end..].iter())
            .filter(|step| step.source == StepSource::Default)
            .cloned()
    }

    /// A fresh log of `edit`, as Discard History starts an edit whose history cannot be
    /// read: a default step of each of `defaults` at `clock`, as a new photo's log begins
    /// ([`Log::add_defaults`]), then a step of each tool and operation the edit holds,
    /// in [`Entry::between`]'s order, source You, noted `history-discard`
    /// ([`Note::discard`]), sharing a batch where there are several, the end after the
    /// last. The default steps of the tools the edit holds not applied come after the
    /// end, so they stay not applied, and Original is the untouched photo.
    pub fn afresh(edit: &Edit, defaults: &[&str], clock: Clock) -> Log {
        let (after, before): (Vec<&str>, Vec<&str>) =
            (defaults.iter()).partition(|tool| edit.unapplied.contains(**tool));
        let mut log = Log::default();
        log.add_defaults(&before, clock);
        let changes = Change::between(&Edit::default(), edit, &[]);
        log.add_change(changes, StepSource::You, Some(Note::discard()), None, clock);
        let end = log.end;
        for tool in after {
            let id = log.new_id();
            log.steps.push(Step {
                id,
                clock,
                source: StepSource::Default,
                note: None,
                batch: None,
                change: Change::Entry(Entry::Tool {
                    tool: tool.to_owned(),
                    values: ToolValues::new(),
                }),
                styles: None,
            });
        }
        log.end = end;
        log
    }

    /// Keeps the first `at` steps, puts `after` after them, and returns the steps it took
    /// out in their place; the end goes to `end`, at most the steps' count.
    pub fn replace_after(&mut self, at: usize, after: Vec<Step>, end: usize) -> Vec<Step> {
        let taken = self.steps.split_off(at.min(self.steps.len()));
        self.steps.extend(after);
        self.end = end.min(self.steps.len());
        taken
    }

    /// Where the step `id` is among the steps, the first counting 0; `None` for one not in
    /// the log.
    pub fn position(&self, id: StepId) -> Option<usize> {
        self.steps.iter().position(|step| step.id == id)
    }

    /// Moves the end to `end`, at most the steps' count.
    pub fn set_end(&mut self, end: usize) {
        self.end = end.min(self.steps.len());
    }

    /// Where the steps Undo would take back from `end` begin: the step before it, with the
    /// steps of its batch before it; `None` at Original.
    pub fn batch_start(&self, end: usize) -> Option<usize> {
        let last = end.checked_sub(1)?;
        let batch = self.steps.get(last)?.batch;
        let other =
            (self.steps[..last].iter()).rposition(|step| batch.is_none() || step.batch != batch);
        Some(other.map_or(0, |other| other + 1))
    }

    /// Where the steps Redo would put back from `end` end: just after the step at it, and
    /// the steps of its batch after it; `None` past the last step.
    pub fn batch_end(&self, end: usize) -> Option<usize> {
        let batch = self.steps.get(end)?.batch;
        let other =
            (self.steps[end + 1..].iter()).position(|step| batch.is_none() || step.batch != batch);
        Some(other.map_or(self.steps.len(), |other| end + 1 + other))
    }

    /// Where Undo's fallback, with no undo record, as after reopening a photo, moves the
    /// end: back over the step before it and the steps of its batch before that
    /// ([`Log::batch_start`]), never over a default step, as darktable's auto-applied items
    /// are no undo records: it stops above the last default step before the end, wherever
    /// it lies, one [`Log::afresh`] left after the end and a Redo applied among them;
    /// `None` where it would not move.
    pub fn undo_end(&self) -> Option<usize> {
        let floor = (self.steps[..self.end].iter())
            .rposition(|step| step.source == StepSource::Default)
            .map_or(0, |last| last + 1);
        if self.end <= floor {
            return None;
        }
        // A batch holds no default step, so it never reaches below the floor.
        self.batch_start(self.end)
    }

    /// Where Redo's fallback moves the end: forward over the step after it and the steps of
    /// its batch after that ([`Log::batch_end`]); `None` past the last step.
    pub fn redo_end(&self) -> Option<usize> {
        self.batch_end(self.end)
    }

    /// A random id no step of the log has.
    pub fn new_id(&self) -> StepId {
        loop {
            let id = StepId(random());
            if self.steps.iter().all(|step| step.id != id) {
                return id;
            }
        }
    }

    /// A random batch id no step of the log has.
    pub fn new_batch(&self) -> BatchId {
        loop {
            let batch = BatchId(random());
            if self.steps.iter().all(|step| step.batch != Some(batch)) {
                return batch;
            }
        }
    }
}

/// Advances a copy's clock vector, the latest clock of each machine that changed it
/// (decision 47), with `clock`: its machine's clock where it is later, or a clock of a
/// machine it did not hold.
pub fn advance(vector: &mut Vec<Clock>, clock: Clock) {
    match vector.iter_mut().find(|kept| kept.machine == clock.machine) {
        Some(kept) => *kept = (*kept).max(clock),
        None => vector.push(clock),
    }
}

/// The latest clock the machine `me` keeps after reading `clocks`, a log's and its copy's
/// vector, with its wall clock now: each observed in turn ([`Clock::observe`]), so another
/// machine's clock far past the wall clock is ignored. What the next step's clock follows.
pub fn latest<'c>(clocks: impl IntoIterator<Item = &'c Clock>, me: MachineId) -> Option<Clock> {
    let wall = apertrust_library::clock::wall_clock();
    (clocks.into_iter()).fold(None, |last, read| {
        Clock::observe(last.as_ref(), read, wall, me)
    })
}

/// `edit` moved from `from` onto `onto`: `onto` with each tool and operation whose state
/// differs between `from` and `edit` at its state in `edit`, as a change not yet a step is
/// laid on a newer log.
pub(crate) fn rebase(edit: &Edit, from: &Edit, onto: &Edit) -> Edit {
    let mut rebased = onto.clone();
    for entry in Entry::between(from, edit) {
        entry.apply(&mut rebased);
    }
    rebased
}

/// 64 random bits from the system, else from std's randomly keyed hasher.
fn random() -> u64 {
    getrandom::u64().unwrap_or_else(|_| {
        use std::hash::{BuildHasher, Hasher};
        let mut hasher = std::hash::RandomState::new().build_hasher();
        hasher.write_u128(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |since| since.as_nanos()),
        );
        hasher.finish()
    })
}

/// The active photo's history, as a client lists it and names what Undo and Redo would
/// do (decision 78): a fact, `Fact::History`, its whole topic each time.
#[derive(Debug, Clone, PartialEq)]
pub struct History {
    /// How many commands it includes, as `Values::through` counts them.
    pub through: u64,
    /// Oldest first; a client lists them newest first. Each behind an `Arc`, so facts
    /// sent one after another share the steps they have in common.
    pub steps: Vec<Arc<HistoryStep>>,
    /// How many steps apply, darktable's `history_end`: 0 is Original, and the step
    /// numbered `end`, counting from 1, is the last applied.
    pub end: usize,
    /// What Undo and Redo would do now, so a menu names it; `None` when nothing.
    pub undo: Option<Move>,
    pub redo: Option<Move>,
}

/// A step as a client shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryStep {
    pub id: StepId,
    pub clock: Clock,
    pub source: StepSource,
    pub note: Option<Note>,
    /// Shared by the steps of one change that touched several tools, which Undo and Redo
    /// move whole, as [`Move::Steps`] names them.
    pub batch: Option<BatchId>,
    /// The tool the step touched, with what changed: one, or for a reset each tool it
    /// reset, in the step's order; an operation's unowned parameters under the operation's
    /// id, its controls the parameters. A tool or an operation this build does not know,
    /// as a newer build may have written, is listed by its stored name with no control.
    pub changes: Vec<ToolChange>,
}

/// What a step changed of one tool.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolChange {
    /// The tool's id, or the operation's for its unowned parameters; borrowed for one
    /// this build knows, owned for one it does not.
    pub tool: Cow<'static, str>,
    /// Before and after, where the step turned a switched tool on or off.
    pub enabled: Option<[bool; 2]>,
    /// Each control whose value or mark changed, in the definition's order.
    pub controls: Vec<ControlChange>,
}

/// One control's change, its values each the control's default for the photo where none
/// is set; an unowned parameter's integer as a number.
#[derive(Debug, Clone, PartialEq)]
pub struct ControlChange {
    pub control: &'static str,
    pub before: ControlValue,
    pub after: ControlValue,
    pub mark: [Option<Mark>; 2],
}

/// What Undo or Redo would do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    /// Take back or put back the `count` steps from `first` on, one change's: its step, or
    /// its batch's steps, added, or the end moved over them; a batch's part where `GoTo`
    /// left the end inside it.
    /// `photos` is how many photos a batch on several photos named that these steps are
    /// part of, which Undo and Redo take back or put back on each of them still on it
    /// (decision 109), else 1.
    Steps {
        first: StepId,
        count: usize,
        photos: usize,
    },
    /// Take back or put back a `GoTo`: the end back where it was, or where it put it.
    GoTo,
    /// Take back or put back a Compress: the log as it was before it, or compressed again.
    Compress,
    /// Take back or put back a Truncate: the log as it was before it, or truncated again.
    Truncate,
    /// Take back or put back a Discard History: the log as it was before it, or discarded
    /// again, the photo given its default steps.
    DiscardHistory,
    /// Take back or put back a change of the photo's rating, its reject or a label
    /// (decision 110), no step of the log: `change` the last change the undo record holds,
    /// quick repeats of one key merged, on `photos` photos, a batch's count where it is one on
    /// several photos (decision 109), else 1. The Metadata tool's fields join it (#643).
    Field { change: RatingChange, photos: usize },
}

/// The modules darktable has on by default with no on/off button (`hide_enable_button`),
/// which its `_dev_add_default_modules` adds to a new photo's history first
/// (`src/develop/develop.c:2142-2171`, called at `:2339`): Raw black/white point, Demosaic,
/// Input colour profile and Output colour profile.
const NO_BUTTON: &[&str] = &["rawprepare", "demosaic", "colorin", "colorout"];

/// The modules darktable has on by default with an on/off button, which it adds next:
/// White balance, Highlight reconstruction and Orientation. The workflow's auto-applied
/// presets, Exposure, Colour calibration and the tone mapper among them, come after
/// (`_dev_auto_apply_presets`, `:2343`).
const WITH_BUTTON: &[&str] = &["temperature", "highlights", "flip"];

/// The modules darktable's default workflow, scene-referred, applies to a new raw as
/// auto-applied presets after those (`_dev_auto_apply_presets`, `:1856`): Exposure, Colour
/// calibration and the tone mapper, Apertrust's AgX. No other tool has a default step:
/// Tone curve, which no preset applies, runs at its defaults with none.
const AUTO_APPLIED: &[&str] = &["exposure", "channelmixerrgb", "agx"];

/// Whether darktable has the module on by default (`default_enabled`), with an on/off
/// button or none, so it runs at its defaults below its default step.
fn on_by_default(tool: &str) -> bool {
    NO_BUTTON.contains(&tool) || WITH_BUTTON.contains(&tool)
}

/// The tools a new photo's default steps hold (`docs/history.md`, Original and the default
/// steps): darktable's for a new raw, among `tools`, in its history order, its three
/// groups: the tools on by default with no on/off button, those with one, and those its
/// workflow auto-applies, each in the pipe's order, by the first operation each tool
/// owns.
pub fn default_tools(
    tools: &[&'static dyn ProcessingTool],
    operations: &[&'static dyn Operation],
) -> Vec<&'static str> {
    let position = |tool: &dyn ProcessingTool| {
        (tool.owns().iter())
            .filter_map(|owned| {
                (operations.iter()).position(|operation| operation.manifest().id == owned.operation)
            })
            .min()
            .unwrap_or(usize::MAX)
    };
    let group = |id: &str| {
        if NO_BUTTON.contains(&id) {
            0
        } else if WITH_BUTTON.contains(&id) {
            1
        } else if AUTO_APPLIED.contains(&id) {
            2
        } else {
            3
        }
    };
    let mut lines: Vec<(u8, usize, &'static str)> = (tools.iter())
        .map(|tool| {
            let id = tool.definition().tool.id;
            (group(id), position(*tool), id)
        })
        .filter(|&(group, _, _)| group < 3)
        .collect();
    lines.sort_by_key(|&(group, position, _)| (group, position));
    lines.into_iter().map(|(_, _, id)| id).collect()
}

/// The steps of one photo's log as a client shows them, each described once and kept
/// while the steps before it stay as they are, and the photo's defaults they read.
#[derive(Default)]
pub struct Described {
    steps: Vec<Arc<HistoryStep>>,
    /// Content as described, since an external caller may reuse a clock while merging.
    source: Vec<Step>,
    /// Each tool's defaults for the photo, asked once.
    defaults: BTreeMap<&'static str, Controls>,
    /// The operations' defaults for the photo, for unowned parameters, made once asked.
    base: Option<Base>,
}

impl Described {
    /// Every step of `log`, described on the photo `image`: those described already kept
    /// up to the first whose identity, clock or content differs, the rest described anew.
    /// A merged change may reuse a clock, so the content is compared too.
    pub fn steps(
        &mut self,
        log: &Log,
        tools: &[&'static dyn ProcessingTool],
        operations: &[&'static dyn Operation],
        image: &ImageContext,
    ) -> Vec<Arc<HistoryStep>> {
        let kept = self
            .source
            .iter()
            .zip(log.steps())
            .take_while(|(described, step)| described == step)
            .count();
        self.steps.truncate(kept);
        self.source.truncate(kept);
        if kept < log.steps().len() {
            // Each key's last entry before the first step described anew: its before.
            let mut last: BTreeMap<Key<'_>, &Entry> = BTreeMap::new();
            for entry in (log.steps()[..kept].iter()).flat_map(|step| step.change.entries()) {
                last.insert(entry.key(), entry);
            }
            for step in &log.steps()[kept..] {
                let changes = (step.change.entries().iter())
                    .map(|entry| {
                        let before = last.get(&entry.key()).copied();
                        self.change(before, entry, tools, operations, image)
                    })
                    .collect();
                self.steps.push(Arc::new(HistoryStep {
                    id: step.id,
                    clock: step.clock,
                    source: step.source,
                    note: step.note.clone(),
                    batch: step.batch,
                    changes,
                }));
                self.source.push(step.clone());
                for entry in step.change.entries() {
                    last.insert(entry.key(), entry);
                }
            }
        }
        self.steps.clone()
    }

    /// What `entry` changed over `before`, the key's previous entry, none being its
    /// reset state; a tool or an operation this build does not know, by its stored
    /// name with no control, as nothing can describe its values.
    fn change(
        &mut self,
        before: Option<&Entry>,
        entry: &Entry,
        tools: &[&'static dyn ProcessingTool],
        operations: &[&'static dyn Operation],
        image: &ImageContext,
    ) -> ToolChange {
        self.known(before, entry, tools, operations, image)
            .unwrap_or_else(|| {
                let (Entry::Tool { tool: name, .. }
                | Entry::Operation {
                    operation: name, ..
                }) = entry;
                ToolChange {
                    tool: Cow::Owned(name.clone()),
                    enabled: None,
                    controls: Vec::new(),
                }
            })
    }

    /// What `entry` changed over `before`, as [`Self::change`]; `None` for a tool or an
    /// operation this build does not know.
    fn known(
        &mut self,
        before: Option<&Entry>,
        entry: &Entry,
        tools: &[&'static dyn ProcessingTool],
        operations: &[&'static dyn Operation],
        image: &ImageContext,
    ) -> Option<ToolChange> {
        match entry {
            Entry::Tool { tool, values } => {
                let tool = *tools.iter().find(|m| m.definition().tool.id == tool)?;
                let definition = tool.definition();
                // A tool's code may panic, as its binding is guarded against: its
                // controls then read the definition's defaults, as the values do.
                let defaults = self.defaults.entry(definition.tool.id).or_insert_with(|| {
                    catch_unwind(AssertUnwindSafe(|| tool.defaults(image)))
                        .unwrap_or_else(|_| definition.defaults())
                });
                let empty = ToolValues::new();
                let before = match before {
                    Some(Entry::Tool { values, .. }) => values,
                    _ => &empty,
                };
                let enabled = (definition.switched && before.enabled() != values.enabled())
                    .then_some([before.enabled(), values.enabled()]);
                let controls = (definition.controls.iter())
                    .filter_map(|control| {
                        let value = |values: &ToolValues| {
                            (values.get(control.id).or_else(|| defaults.get(control.id)))
                                .cloned()
                                .unwrap_or_else(|| control.default())
                        };
                        let (was, is) = (value(before), value(values));
                        let mark = [before.mark(control.id), values.mark(control.id)];
                        (was != is || mark[0] != mark[1]).then_some(ControlChange {
                            control: control.id,
                            before: was,
                            after: is,
                            mark,
                        })
                    })
                    .collect();
                Some(ToolChange {
                    tool: Cow::Borrowed(definition.tool.id),
                    enabled,
                    controls,
                })
            }
            Entry::Operation { operation, params } => {
                let operation = *operations.iter().find(|o| o.manifest().id == operation)?;
                let id = operation.manifest().id;
                let base = self
                    .base
                    .get_or_insert_with(|| Base::new(operations, image));
                let empty = ParamValues::new();
                let before = match before {
                    Some(Entry::Operation { params, .. }) => params,
                    _ => &empty,
                };
                let controls = (operation.schema().params.iter())
                    .filter_map(|param| {
                        let value = |params: &ParamValues| {
                            (params.get(param.name)).or_else(|| base.get(id, param.name))
                        };
                        let (was, is) = (value(before)?, value(params)?);
                        (was != is).then_some(ControlChange {
                            control: param.name,
                            before: control_value(was),
                            after: control_value(is),
                            mark: [None, None],
                        })
                    })
                    .collect();
                Some(ToolChange {
                    tool: Cow::Borrowed(id),
                    enabled: None,
                    controls,
                })
            }
        }
    }
}

/// An unowned parameter's value as a control's.
fn control_value(value: Value) -> ControlValue {
    match value {
        Value::Float(number) => ControlValue::Number(number),
        Value::Int(number) => ControlValue::Number(number as f32),
        Value::Choice(index) => ControlValue::Choice(index),
        Value::Bool(flag) => ControlValue::Bool(flag),
    }
}

#[cfg(test)]
mod tests;
