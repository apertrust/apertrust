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

//! The open photo's history in the session (decision 78, `docs/history.md`, Behaviour):
//! its log in memory, each completed change a step of each tool it touched, and the undo
//! records, which Undo and Redo take back and put back as darktable's undo does
//! (`src/libs/history.c:664-686`).
//!
//! A completed change that touched several tools is a step for each, in the order the
//! tools were written, sharing a batch id, and one undo record, as darktable's undo group
//! (`dt_undo_start_group`, `src/common/history.c:157`); Reset is one step of every tool
//! it reset. Two stacks, the done records and the undone ones: Undo takes the newest done
//! record back onto the undone stack, Redo the newest undone one back onto the done stack,
//! and a new step clears the undone stack. Only when the done stack is empty, as after
//! reopening a photo, does Undo move the end back over one change's steps, the step before
//! it and the steps of its batch before that, and only when the undone stack is empty does
//! Redo move it forward over the next; each such move is pushed onto the other stack, so
//! the next Redo or Undo reverses it. Both stacks end as the photo closes.
//!
//! `GoTo` moves the end, a record of its own, as darktable's click on a history item is
//! (`src/libs/history.c:1297-1318`), to any step, one inside a batch among them. `Step`
//! makes several commands one change of its source and note, all or none, taking in the
//! gesture under way on a tool it writes, of its source and note.
//!
//! The log is the sidecar's (version 2, [`crate::sidecar`]), read as the photo opens and
//! saved after each completed change and each move of the end, with the copy's clock
//! vector, which each step and each move advances. A newer file found before a write is
//! taken in ([`PhotoHistory::take_in`]): the steps not in it laid on its log after its end.
//!
//! A photo opened with no history is given its default steps, darktable's default modules,
//! as darktable's new raw is given its default modules and auto-applied presets
//! (`dt_dev_read_history_ext`, `src/develop/develop.c:2301-2352`); a log holding only them
//! is not written, so they go into the file with the first change. Undo's fallback stops
//! above them, as darktable's auto-applied items are no undo records; `GoTo` reaches each.
//!
//! `Compress` keeps the default steps and each tool's last step up to the end, `Truncate`
//! drops the person's steps after the end, neither changing what the photo shows but at
//! end 0, and `DiscardHistory` goes to Original and compresses, so the log holds no step:
//! each one undo record holding the log as it was, as darktable records its compress,
//! truncate and discard (`_lib_history_truncate`, `src/libs/history.c:1189-1235`). A log
//! they leave with no step is given the default steps again, as a new photo is, and the
//! file is written with none, the default steps going into it with the first change. On
//! an edit open for viewing only because of a `sidecar::Problem`, `DiscardHistory` starts
//! the log afresh from the edit as read ([`PhotoHistory::afresh`]).
//!
//! A style's change, `ApplyStyle` or `ClearStyle` (`session/styles.rs`, decision 106), leaves
//! the list of styles applied it makes to [`PhotoHistory::complete`], which puts it on the
//! change's last step, or on a step of its own where the change moves no tool; a Reset
//! empties it. A newer file taken in has the steps laid on it hold their lists rebuilt on
//! its own ([`crate::history::replay`]).
//!
//! A change merges into the step before it as darktable's undo merges a widget's quick
//! repeats (`_dev_undo_start_record_target`, `src/develop/develop.c:1351-1370`): the same
//! target, a control or a preset row, of the same source, You or Auto, with no note, made
//! right after that step with no other command on the edit between, its first write within
//! [`MERGE_WITHIN`] of the step's first and, once a frame including the step's last change
//! is published, within [`REVIEW`] of that frame (`develop.c:885`); a change of several
//! tools never merges, nor is a step of a batch merged into. It adds no undo record, so
//! one Undo takes the merged step back whole. The times are the commands' own, as the
//! client sent them, and the frames' publications, both handed in, so a test sets them.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::time::{Duration, Instant};

use apertrust_library::clock::{Clock, MachineId, wall_clock};
use apertrust_library::fields::FieldEntry;
use apertrust_plugin::{ImageContext, Operation};
use apertrust_tools::{Moment, ProcessingTool, ToolValues};

use super::fields::{self, FieldRecord};
use super::{Command, Effect, Engine, Event, Open, SessionError, on_edit};
use crate::edit::Edit;
use crate::history::{
    AppliedStyle, BatchId, Change, Described, Entry, History, Log, Move, Note, Step, StepId,
    StepSource, Wrote, advance, latest, replay,
};

/// The open photo's log, its undo records and what its next step is measured from.
pub(super) struct PhotoHistory {
    log: Log,
    /// The log's fold: the edit as the log's last step left it, or as it was read, what
    /// the next step's entries are measured from, so a change not yet a step, a `Set` or a
    /// gesture under way, joins the next.
    stepped: Edit,
    /// What the session makes of a fold as it reads one ([`Reader`]): the tools a change
    /// did not write are measured from the fold as read, so that a clamp or a redraw is
    /// saved only with its own tool's next step, never a step of its own. `None`, the fold
    /// as it is, for a history no photo's session reads.
    reader: Option<Reader>,
    /// [`Self::stepped`] as the reader reads it, kept as `stepped` changes
    /// ([`Self::set_stepped`]).
    read: Edit,
    done: Vec<Record>,
    undone: Vec<Record>,
    machine: MachineId,
    /// The latest clock the photo's history has made or read, which the next follows.
    clock: Option<Clock>,
    /// The copy's clock vector (decision 47): the latest clock of each machine that changed
    /// it, its steps and its moves of the end, which the sidecar saves.
    clocks: Vec<Clock>,
    described: Described,
    /// Whether the log, its end or the records changed since the last `History` sent.
    changed: bool,
    /// The changes not yet a step, from their first write; `None` while nothing is
    /// written.
    written: Option<Written>,
    /// The step the next change may merge into: the log's last, at the end, made with no
    /// command on the edit since but changes not yet a step, of one target, source You or
    /// Auto, no note, and of one tool, so in no batch.
    merge: Option<Merge>,
    /// Whether the log was read from a file holding steps, taken in from one, or handed to
    /// the writer: until then a log of default steps alone is not written
    /// ([`Self::unwritten`]).
    saved: bool,
    /// Whether the file may hold steps a Compress, a Truncate or a Discard History took
    /// out, leaving default steps alone not yet written: the file is written once with no
    /// step ([`Self::cleared`]).
    cleared: bool,
    /// The steps a Compress, a Truncate or a Discard History took out of the log and no Undo
    /// or Redo brought back, by id: a newer file taken in before the log without them is
    /// written keeps them, and its end stands ([`Self::take_in`]).
    dropped: HashSet<StepId>,
    /// The list of styles applied the change being completed leaves, an `ApplyStyle`'s or a
    /// `ClearStyle`'s, which its last step holds, or a step of its own where it moves no
    /// tool ([`Self::complete`]).
    styles: Option<Vec<AppliedStyle>>,
    /// Whether the change being completed reset the whole edit, which empties the list of
    /// styles applied though it may move no tool.
    reset: bool,
    /// Whether the change being completed is a Replace, which records its Reset step
    /// whatever it moves.
    replace: bool,
    /// When the field record on top of the done records began, while a quick repeat of its
    /// change may merge into it: no other change, of the edit or of a field, since.
    field_merge: Option<Instant>,
}

/// What the sidecar holds of the log as the session last read it or the writer last wrote
/// it: its steps, by id and clock, its copy's clock vector and its styles applied. A newer
/// file is compared
/// with it, not with the session's own vector, which this machine's other programs, as
/// `apertrust-cli edit`, share (`docs/history.md`, Behaviour).
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Base {
    pub(super) steps: HashSet<(StepId, Clock)>,
    pub(super) clocks: Vec<Clock>,
    /// The last step applied, `None` at Original.
    pub(super) end: Option<StepId>,
    /// The steps applied, up to the end.
    pub(super) applied: HashSet<(StepId, Clock)>,
    /// The styles applied at its end (decision 106), which a Compress not yet saved, whose
    /// log holds no list before its own list-only step, is replayed from.
    pub(super) styles: Vec<AppliedStyle>,
}

impl Base {
    /// What a file holding `log` with the clock vector `clocks` holds.
    pub(super) fn of(log: &Log, clocks: &[Clock]) -> Base {
        Base {
            steps: (log.steps().iter())
                .map(|step| (step.id, step.clock))
                .collect(),
            clocks: clocks.to_vec(),
            end: log.steps()[..log.end()].last().map(|step| step.id),
            applied: (log.steps()[..log.end()].iter())
                .map(|step| (step.id, step.clock))
                .collect(),
            styles: log.styles().to_vec(),
        }
    }

    /// Whether a file whose copy's clock vector is `clocks` holds nothing past this: every
    /// machine's clock at or below this one's.
    pub(super) fn covers(&self, clocks: &[Clock]) -> bool {
        clocks.iter().all(|theirs| {
            (self.clocks.iter()).any(|ours| ours.machine == theirs.machine && ours >= theirs)
        })
    }
}

/// darktable's `darkroom/undo/merge_same_secs`: a change merges into the step before it
/// only when its first write comes within this of the step's first.
pub(super) const MERGE_WITHIN: Duration = Duration::from_secs(10);

/// darktable's `darkroom/undo/review_secs`: nor once this has passed since the first frame
/// published that includes the step's last change, its `gui_previous_pipe_time`.
pub(super) const REVIEW: Duration = Duration::from_secs(2);

/// What a change wrote, darktable's widget: a change merges only into a step of the same
/// target (`docs/history.md`, Merging consecutive changes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Target {
    /// A control set or reset, a view's coordinate among them.
    Control { tool: String, control: String },
    /// A preset row an option was applied from.
    Preset { tool: String, preset: String },
}

/// The changes not yet a step.
#[derive(Debug)]
struct Written {
    /// When the first was sent.
    began: Instant,
    /// The one target they all wrote; `None` for several, or for a change of none.
    target: Option<Target>,
    /// What each of their commands wrote, in order, which orders their steps.
    wrote: Vec<Wrote>,
    /// When the first frame including the last of them was published; `None` until then.
    published: Option<Instant>,
}

/// The step a change may merge into, and its two times.
#[derive(Debug)]
struct Merge {
    target: Target,
    source: StepSource,
    /// When its first change's first write was sent: darktable's record start.
    began: Instant,
    /// When the first frame including its last change was published; `None` until then,
    /// while only [`MERGE_WITHIN`] counts.
    published: Option<Instant>,
}

/// A command that rewrites the log rather than moving its end: Compress and Truncate at the
/// end, or with a step just after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Rewrite {
    Compress(Option<StepId>),
    Truncate(Option<StepId>),
    Discard,
}

/// What one command on the history changed, which Undo takes back and Redo puts back.
#[derive(Debug)]
enum Record {
    /// One change's steps added at `at`, the end before them: one, or a batch's, after the
    /// `kept` default steps they moved below the end ([`Log::add`]). `held` is what the log
    /// does not hold: while the record is done, the steps that were after the end; while it
    /// is undone, the kept and the change's steps.
    Added {
        at: usize,
        kept: usize,
        held: Vec<Step>,
    },
    /// The end moved over the steps from `start` to `end`, one change's or the part of one
    /// that applied, by Undo's or Redo's fallback: Undo puts it at `start`, Redo at `end`.
    Moved { start: usize, end: usize },
    /// The end moved by a `GoTo` from `from` to `to`.
    GoTo { from: usize, to: usize },
    /// The log rewritten by `what`, a Compress, a Truncate or a Discard History: `log` is the
    /// log it is not, and `saved` whether that was in the file: while the record is done the
    /// log before, while it is undone the log after.
    Rewritten { what: Move, log: Log, saved: bool },
    /// The photo's rating, reject or labels changed (decision 110), no step of the log:
    /// Undo puts each field back, Redo sets it again, but where it changed since.
    Field(FieldRecord),
}

impl PhotoHistory {
    /// The history of a photo whose sidecar holds `log`, with its copy's clock vector
    /// `clocks`, both empty for a photo never edited: the next step's clock follows the
    /// latest of them ([`latest`]). A log with no step is given a default step of each of
    /// `defaults`, the end after them, none written until the first change; a log with steps
    /// is given none, so its steps keep their numbers and a default tool a later build
    /// adds runs at its defaults with no step.
    pub(super) fn read(log: Log, clocks: Vec<Clock>, defaults: &[&str]) -> Self {
        Self::read_at(log, clocks, defaults, wall_clock())
    }

    /// [`Self::read`], the default steps of a photo never edited made with the wall clock at
    /// `wall`, a command's own, so they come before its steps whenever it runs.
    pub(super) fn read_at(log: Log, clocks: Vec<Clock>, defaults: &[&str], wall: u64) -> Self {
        let machine = MachineId::current();
        let mut log = log;
        let read = (log.steps().iter().map(|step| &step.clock)).chain(&clocks);
        let mut clock = latest(read, machine);
        let mut clocks = clocks;
        let saved = !log.steps().is_empty();
        if !saved && !defaults.is_empty() {
            // Their clock is the vector's too, so the file they go into holds no step past
            // it.
            let made = Clock::at(wall, clock.as_ref(), machine);
            log.add_defaults(defaults, made);
            advance(&mut clocks, made);
            clock = Some(made);
        }
        PhotoHistory {
            clock,
            stepped: log.fold(),
            reader: None,
            read: log.fold(),
            log,
            done: Vec::new(),
            undone: Vec::new(),
            machine,
            clocks,
            described: Described::default(),
            changed: true,
            written: None,
            merge: None,
            saved,
            cleared: false,
            dropped: HashSet::new(),
            styles: None,
            reset: false,
            replace: false,
            field_merge: None,
        }
    }

    /// Whether the log holds default steps alone and was never in the file: written with
    /// the first change, not before, whatever its end, so opening a photo or moving among
    /// its default steps writes nothing.
    pub(super) fn unwritten(&self) -> bool {
        let steps = self.log.steps();
        !self.saved
            && !steps.is_empty()
            && steps.iter().all(|step| step.source == StepSource::Default)
    }

    /// Whether the file may hold steps the log no longer has, while the log holds default
    /// steps alone never written: written once with no step, not held.
    pub(super) fn cleared(&self) -> bool {
        self.cleared && self.unwritten()
    }

    /// The log was handed to the writer: a log of default steps alone that clears the file
    /// was handed with no step ([`Self::written`]), and is still unwritten.
    pub(super) fn handed(&mut self) {
        if self.unwritten() {
            self.cleared = false;
        } else {
            self.saved = true;
        }
    }

    /// The log as the file is to hold it: the log, but no step for a log of default steps
    /// alone never written, which go into the file with the first change.
    pub(super) fn written(&self) -> Cow<'_, Log> {
        if self.unwritten() {
            Cow::Owned(Log::default())
        } else {
            Cow::Borrowed(&self.log)
        }
    }

    /// Its steps described anew, as the decode's image context differs from the one they
    /// were described on.
    pub(super) fn describe_anew(&mut self) {
        self.described = Described::default();
        self.changed = true;
    }

    /// The log in memory, any default steps not yet written among it; the sidecar saves
    /// [`Self::written`].
    pub(super) fn log(&self) -> &Log {
        &self.log
    }

    /// The log and the copy's clock vector, to write, as a photo that is not open is
    /// changed (decision 109).
    pub(super) fn into_log(self) -> (Log, Vec<Clock>) {
        (self.log, self.clocks)
    }

    /// Its history is told again, though it has not changed: what Undo and Redo would do
    /// names another count of photos (decision 109).
    pub(super) fn tell_again(&mut self) {
        self.changed = true;
    }

    /// A batch id no step of the log has.
    pub(super) fn new_batch(&self) -> BatchId {
        self.log.new_batch()
    }

    /// The styles applied to the edit, newest first, as the log holds them at its end
    /// (decision 106).
    pub(super) fn styles(&self) -> &[AppliedStyle] {
        self.log.styles()
    }

    /// The change being completed leaves the list of styles applied `styles`: its last step
    /// holds it, moved or not, or, where it moves no tool, a step of its own, where it
    /// differs from the list the log holds.
    pub(super) fn leave_styles(&mut self, styles: Vec<AppliedStyle>) {
        self.styles = Some(styles);
    }

    /// The change being completed is a Replace: it records its Reset step, an empty one
    /// where it reset no tool, whatever else it moves.
    pub(super) fn replacing(&mut self) {
        self.replace = true;
    }

    /// The change being completed reset the whole edit: it empties the list of styles
    /// applied, a Reset's step of its own where it moves no tool.
    pub(super) fn reset_styles(&mut self) {
        self.reset = true;
    }

    /// The copy's clock vector, which the sidecar saves with the log.
    pub(super) fn clocks(&self) -> &[Clock] {
        &self.clocks
    }

    /// The edit as the log's last step left it, which the next step is measured from.
    pub(super) fn stepped(&self) -> &Edit {
        &self.stepped
    }

    /// Reads folds as the session does on the photo's `image` with `tools`, from now on.
    pub(super) fn set_reader(
        &mut self,
        tools: &[&'static dyn ProcessingTool],
        image: &ImageContext,
    ) {
        self.reader = Some(Reader {
            tools: tools.to_vec(),
            image: image.clone(),
        });
        self.set_stepped(self.stepped.clone());
    }

    /// `edit` as the session reads a fold ([`Reader`]), or as it is with no reader.
    pub(super) fn read_of(&self, edit: &Edit) -> Edit {
        self.reader
            .as_ref()
            .map_or_else(|| edit.clone(), |reader| reader.read(edit))
    }

    /// The log's fold becomes `stepped`, and its reading with it.
    fn set_stepped(&mut self, stepped: Edit) {
        self.read = match &self.reader {
            Some(reader) => reader.read(&stepped),
            None => stepped.clone(),
        };
        self.stepped = stepped;
    }

    /// What `edit`'s changes not yet a step are measured from: the log's fold, but for
    /// each tool `edit` holds as the fold as read, its clamp or its redraw, the fold as
    /// read. A tool's state counts as changed only where it differs from both, so neither
    /// a clamp nor a redraw is ever a change of its own.
    pub(super) fn baseline(&self, edit: &Edit) -> Edit {
        let mut from = self.stepped.clone();
        let state = |edit: &Edit, tool: &str| {
            (edit.tools.get(tool).cloned()).filter(|values| !values.is_empty())
        };
        let tools: BTreeSet<&String> = (self.read.tools.keys())
            .chain(self.stepped.tools.keys())
            .collect();
        for tool in tools {
            let as_read = state(&self.read, tool);
            if as_read != state(&self.stepped, tool) && as_read == state(edit, tool) {
                match as_read {
                    Some(values) => from.tools.insert(tool.clone(), values),
                    None => from.tools.remove(tool),
                };
            }
        }
        from
    }

    /// Whether `edit` holds no change not yet a step: each tool's state is the log's fold's
    /// or the fold's as read, its clamp and its redraw, which never become a step of their
    /// own ([`Self::baseline`]).
    pub(super) fn holds(&self, edit: &Edit) -> bool {
        Entry::between(&self.baseline(edit), edit).is_empty()
    }

    /// What the sidecar would hold of this history once written: its steps, none for default
    /// steps alone never written ([`Self::written`]), and its copy's clock vector.
    pub(super) fn base(&self) -> Base {
        Base::of(&self.written(), &self.clocks)
    }

    /// Takes in a newer file's `log`, with its copy's clock vector `clocks`
    /// (`docs/history.md`, Behaviour), `base` what the file held as the session last read it
    /// or the writer last wrote it: the steps up to this log's end that are in neither,
    /// by id and clock, the steps not yet saved, laid on it after its end, as an edit after
    /// Undo would, each with a new id where the newer log holds its id already; a step the
    /// other side dropped stays dropped. The vectors join, with a clock of this machine's
    /// where a step was laid or a move made again, and the undo records and the
    /// merge state are cleared, as they describe a log that is gone.
    ///
    /// A move of the end not yet saved, an Undo, a Redo or a `GoTo` with no step to lay, is
    /// made again on the newer log where the other side only added steps after the base's
    /// end: the end goes back to the step this log's end is on, the steps added after it.
    /// Where the other side moved its end or dropped steps of the base, or this log's end
    /// is on a step the newer one lacks, the newer log's end stands, as `false` in the
    /// second place of what this returns, unless it is on this log's end's step already.
    /// An Undo that took steps out of this log does not take them out of the newer one:
    /// with no step to lay, they come after the end, as Redo's fallback reaches them; with
    /// a step laid after them, as an edit after that Undo, they apply again beneath it,
    /// which is a move not kept too. So is a Compress, a Truncate or a Discard History not
    /// yet saved that took out steps the base holds: the newer log keeps them, and its end
    /// stands. Returns the edit the old log's last step left, which a change not yet a step
    /// is moved from onto the new [`Self::stepped`], and whether every move not yet saved
    /// was kept.
    pub(super) fn take_in(&mut self, log: Log, clocks: Vec<Clock>, base: &Base) -> (Edit, bool) {
        let theirs: HashSet<(StepId, Clock)> = (log.steps().iter())
            .map(|step| (step.id, step.clock))
            .collect();
        let mut log = log;
        let mut laid = false;
        // The styles applied, rebuilt from the newer log's end by the session's changes of
        // the list replayed in order (decision 106), and each tool's state in the joined log
        // just before the first step of the change being laid that set it.
        let mut rebuilt = log.styles().to_vec();
        let mut set: BTreeMap<String, ToolValues> = BTreeMap::new();
        // And each tool's state in the joined log just before the last step laid that set
        // it, which a Compress's list replays from, as its log keeps a tool's last step alone.
        let mut last: BTreeMap<String, ToolValues> = BTreeMap::new();
        let mut change = None;
        // Whether the change being laid has laid its Reset: the tools it sets after it are
        // at their reset state just before.
        let mut reset = false;
        for (at, step) in self.log.steps()[..self.log.end()].iter().enumerate() {
            let key = (step.id, step.clock);
            if theirs.contains(&key) || base.steps.contains(&key) {
                continue;
            }
            // Default steps never written are not laid on the newer log: no default step
            // is added to a log with steps.
            if step.source == StepSource::Default {
                continue;
            }
            // Compress's list-only step: the compressed log holds no list before it, so it
            // replays what it differs by from the list the base holds.
            let compressed = step.change == Change::None
                && step.source == StepSource::Style
                && step.note == Some(Note::styles_applied());
            let mut step = step.clone();
            if log.position(step.id).is_some() {
                step.id = log.new_id();
            }
            // A step of no batch, or of another batch, begins another change.
            if step.batch.is_none() || change != Some(step.batch) {
                set.clear();
                reset = false;
            }
            change = Some(step.batch);
            let joined = log.fold();
            for entry in step.change.entries() {
                if let Entry::Tool { tool, .. } = entry {
                    let state = || match reset {
                        true => ToolValues::new(),
                        false => joined.tools.get(tool).cloned().unwrap_or_default(),
                    };
                    set.entry(tool.clone()).or_insert_with(state);
                    last.insert(tool.clone(), state());
                }
            }
            // Every Reset laid empties the list, whether or not it stores one; a step that
            // stores one replays what it differs by from the list before it, and a style's
            // application, noted `history-style`, the style it applied though unchanged.
            let had = !rebuilt.is_empty();
            match (&step.change, &step.styles) {
                (Change::Reset(_), _) => {
                    rebuilt.clear();
                    step.styles = had.then(Vec::new);
                    reset = true;
                }
                (_, Some(after)) => {
                    let stepped = set.keys().cloned().collect();
                    let (before, laid) = match compressed {
                        true => (&base.styles[..], &last),
                        false => (self.log.styles_at(at), &set),
                    };
                    let joined = |tool: &str| {
                        (laid.get(tool).cloned())
                            .unwrap_or_else(|| joined.tools.get(tool).cloned().unwrap_or_default())
                    };
                    let applied = matches!(&step.note,
                        Some(Note::Message { id, .. }) if id == Note::STYLE);
                    replay(&mut rebuilt, (before, after), applied, &stepped, joined);
                    step.styles = Some(rebuilt.clone());
                }
                (_, None) => {}
            }
            // A Compress's list that leaves the list empty, as it was, holds nothing to lay.
            if compressed && !had && rebuilt.is_empty() {
                continue;
            }
            log.add(step);
            laid = true;
        }
        // The end of a log of default steps never written is no move of the end: the file
        // never held it.
        let ours = match self.unwritten() {
            true => base.end,
            false => (self.log.steps()[..self.log.end()].last()).map(|step| step.id),
        };
        // A rewrite not yet saved: a step the base and the newer log hold that a Compress, a
        // Truncate or a Discard History took out, and the log has not brought back.
        let held: HashSet<StepId> = self.log.steps().iter().map(|step| step.id).collect();
        let newer: HashSet<StepId> = theirs.iter().map(|(id, _)| *id).collect();
        let rewritten = (base.steps.iter())
            .any(|(id, _)| self.dropped.contains(id) && !held.contains(id) && newer.contains(id));
        let mut kept = !rewritten;
        let mut moved = false;
        if laid {
            // A step the base applied that this log took out, by its id, by an Undo before
            // the step laid, that applies again in the joined log. A step this log still
            // holds by id with a new clock, as a quick repeat merged into it, was not taken
            // out; nor was one the other side took out too, which applies nowhere; and one
            // the base had not applied, which the step laid dropped and the other side
            // redid, is the other side's move, no Undo of this session's.
            let applied: HashSet<(StepId, Clock)> = (log.steps()[..log.end()].iter())
                .map(|step| (step.id, step.clock))
                .collect();
            let again =
                (base.applied.iter()).any(|key| applied.contains(key) && !held.contains(&key.0));
            // And a step of the base this log applies that the base did not, as a Redo made
            // before the step laid, which the joined log does not apply: a step the base
            // applied already that the other side dropped is its change, no move of this
            // session's.
            let lost = (self.log.steps()[..self.log.end()].iter())
                .map(|step| (step.id, step.clock))
                .any(|key| {
                    base.steps.contains(&key)
                        && !base.applied.contains(&key)
                        && !applied.contains(&key)
                });
            kept &= !again && !lost;
        } else if rewritten {
            // Not made again on a log that holds what the rewrite took out: its end stands.
        } else if ours != base.end {
            // Where the base's end is in the newer log: the other side only added after it
            // where every step of the base is there, its end is no earlier, and no step it
            // applies past the base's end is the base's, as a Redo would apply.
            let at = |end: Option<StepId>, log: &Log| match end {
                None => Some(0),
                Some(id) => log.position(id).map(|at| at + 1),
            };
            let appended = base.steps.iter().all(|key| theirs.contains(key))
                && at(base.end, &log).is_some_and(|at| {
                    at <= log.end()
                        && (log.steps()[at..log.end()].iter())
                            .all(|step| !base.steps.contains(&(step.id, step.clock)))
                });
            match at(ours, &log) {
                // On this log's end's step already: nothing to make again.
                Some(end) if end == log.end() => {}
                Some(end) if appended => {
                    log.set_end(end);
                    moved = true;
                }
                _ => kept = false,
            }
        }
        let read = (log.steps().iter().map(|step| &step.clock)).chain(&clocks);
        if let Some(read) = latest(read, self.machine) {
            self.clock = self.clock.max(Some(read));
        }
        for clock in clocks {
            advance(&mut self.clocks, clock);
        }
        // Steps laid, or a move of the end made again, are this machine's change to the newer
        // file: a clock of its own, so the file it writes is newer than the one it read for
        // every reader, this machine's other programs among them, as a move of the end's
        // clock is.
        if laid || moved {
            let clock = Clock::now(self.clock.as_ref(), self.machine);
            self.clock = Some(clock);
            advance(&mut self.clocks, clock);
        }
        self.log = log;
        self.done.clear();
        self.undone.clear();
        self.merge = None;
        self.saved = true;
        self.cleared = false;
        self.dropped.clear();
        self.changed = true;
        let was = self.stepped.clone();
        self.set_stepped(self.log.fold());
        (was, kept)
    }

    /// Makes what changed since the last step a step of each tool it touched, of `source`
    /// with `note` ([`Change::between`]), and says whether it made any: a change that changes
    /// nothing makes none. Each is in `batch`, the command's, else several share a new
    /// batch id ([`Log::add_change`]); their clock is the command's wall time `wall`, after
    /// the latest clock the history has made or read. A Reset's step is noted
    /// `history-reset`. They drop the steps after the end, which Undo brings back, and clear
    /// the undone records. A change of one tool that merges into the step before it makes
    /// none of its own and adds no record: that step keeps its id, takes the change's entry
    /// and the latest clock. A tool the change did not write that the edit holds as the
    /// fold as read, its clamp or its redraw, makes no step ([`Self::measured_from`]), and
    /// the fold keeps it as it was, so it is saved with that tool's own next step.
    pub(super) fn complete(
        &mut self,
        edit: &Edit,
        (source, note): (StepSource, Option<Note>),
        batch: Option<BatchId>,
        wall: u64,
    ) -> bool {
        let written = self.written.take();
        let styles = self.styles.take();
        let reset = std::mem::take(&mut self.reset);
        let replace = std::mem::take(&mut self.replace);
        let wrote = written
            .as_ref()
            .map_or(&[][..], |written| &written.wrote[..]);
        let mut changes = Change::between(&self.measured_from(edit, wrote), edit, wrote);
        // A Reset or a tool's reset with the end among the default steps moves no value,
        // but applies the tools not applied: a step of what it reset.
        if changes.is_empty() && edit.unapplied.is_empty() && !self.stepped.unapplied.is_empty() {
            changes = applied_by(wrote, &self.stepped.unapplied);
        }
        // A change that resets the whole edit records its Reset step, empty where it reset no
        // tool, so the log says the edit was reset, as a Replace's style laid on a newer file
        // reads it (decision 106); and a Reset that moves no tool but empties the styles
        // applied is that step alone.
        let resets = |changes: &[Change]| changes.iter().any(|c| matches!(c, Change::Reset(_)));
        let any = !changes.is_empty() || !self.log.styles().is_empty();
        if ((reset && any) || replace) && !resets(&changes) {
            changes.insert(0, Change::Reset(Vec::new()));
        }
        // A change of the list alone: one step holding it, no tool's (decision 106).
        let list_only = changes.is_empty()
            && (styles.as_ref()).is_some_and(|styles| styles.as_slice() != self.log.styles());
        // So is a list a change ending on a Reset leaves, as a Replace that moved no tool,
        // which Reset's step, emptying the list, cannot hold.
        let after_reset = matches!(changes.last(), Some(Change::Reset(_)))
            && (styles.as_ref()).is_some_and(|styles| !styles.is_empty());
        if list_only || after_reset {
            changes.push(Change::None);
        }
        if changes.is_empty() {
            return false;
        }
        let touched: BTreeSet<String> = (changes.iter().flat_map(Change::entries))
            .filter_map(|entry| match entry {
                Entry::Tool { tool, .. } => Some(tool.clone()),
                Entry::Operation { .. } => None,
            })
            .collect();
        let clock = Clock::at(wall, self.clock.as_ref(), self.machine);
        // Of one target, source You or Auto, no note: what may merge, or be merged into.
        let target = written.as_ref().and_then(|written| {
            let merges = matches!(source, StepSource::You | StepSource::Auto) && note.is_none();
            (written.target.clone()).filter(|_| merges)
        });
        let merges = |merge: &Merge, written: &Written| {
            Some(&merge.target) == target.as_ref()
                && merge.source == source
                && written.began < merge.began + MERGE_WITHIN
                && (merge.published).is_none_or(|published| written.began < published + REVIEW)
        };
        if let (Some(merge), Some(written), [Change::Entry(entry)]) =
            (&mut self.merge, &written, &changes[..])
            && merges(merge, written)
            && self.log.merge(entry, clock)
        {
            merge.published = written.published;
        } else {
            let one = changes.len() == 1 && !list_only;
            let count = changes.len();
            let at = self.log.end();
            let dropped = self.log.add_change(changes, source, note, batch, clock);
            // The list it leaves, on its last step, where it moved it, and where a style
            // applied again left it as it was: so a newer file taken in replays that
            // application too ([`Self::take_in`]).
            if let Some(styles) = styles {
                self.log.hold_styles(styles);
            }
            let kept = self.log.end() - at - count;
            self.done.push(Record::Added {
                at,
                kept,
                held: dropped,
            });
            self.undone.clear();
            self.merge = (target.zip(written))
                .filter(|_| one)
                .map(|(target, written)| Merge {
                    target,
                    source,
                    began: written.began,
                    published: written.published,
                });
        }
        self.clock = Some(clock);
        advance(&mut self.clocks, clock);
        self.field_merge = None;
        // The fold the steps leave: a clamp or a redraw of a tool they did not touch is
        // still to come.
        let mut stepped = edit.clone();
        stepped.unapplied = self.log.unapplied(self.log.end());
        for tool in (self.stepped.tools.keys()).chain(edit.tools.keys()) {
            if !touched.contains(tool) {
                match self.stepped.tools.get(tool) {
                    Some(values) => stepped.tools.insert(tool.clone(), values.clone()),
                    None => stepped.tools.remove(tool),
                };
            }
        }
        self.set_stepped(stepped);
        self.changed = true;
        true
    }

    /// [`Self::complete`] of a change not yet a step, of source You with no note, as a move
    /// of the end makes one first: in no batch of its own and at the wall clock now.
    fn complete_pending(&mut self, edit: &Edit) -> bool {
        self.complete(edit, (StepSource::You, None), None, wall_clock())
    }

    /// [`Self::complete`] at the wall clock now, in no batch of the caller's, for a test.
    #[cfg(test)]
    pub(super) fn complete_now(
        &mut self,
        edit: &Edit,
        source: StepSource,
        note: Option<Note>,
    ) -> bool {
        self.complete(edit, (source, note), None, wall_clock())
    }

    /// What a change to `edit` that wrote `wrote` is measured from: the log's fold, but
    /// for a tool it did not write that `edit` holds as the fold as read, its clamp or its
    /// redraw, which is then no step of that tool's. After a Reset, the fold.
    fn measured_from(&self, edit: &Edit, wrote: &[Wrote]) -> Edit {
        if wrote.contains(&Wrote::All) {
            return self.stepped.clone();
        }
        let mut from = self.baseline(edit);
        for wrote in wrote {
            if let Wrote::Tool(tool) = wrote {
                match self.stepped.tools.get(tool) {
                    Some(values) => from.tools.insert(tool.clone(), values.clone()),
                    None => from.tools.remove(tool),
                };
            }
        }
        from
    }

    /// The tools not applied in `edit`, the edit as it stands: those the last step left
    /// unapplied while its values are the last step's, and none once they move away, as a
    /// change keeps the default steps after the end and applies them ([`Log::add`]).
    pub(super) fn unapplied(&self, edit: &Edit) -> BTreeSet<String> {
        if self.holds(edit) {
            self.stepped.unapplied.clone()
        } else {
            BTreeSet::new()
        }
    }

    /// A command on the edit, sent at `at`, wrote `target`, or, with `None`, wrote several
    /// or none, and wrote the tools and operations `wrote`, in order: the changes not yet
    /// a step go on, or begin with it.
    pub(super) fn wrote(&mut self, target: Option<Target>, wrote: Vec<Wrote>, at: Instant) {
        self.field_merge = None;
        self.written = Some(match self.written.take() {
            None => Written {
                began: at,
                target,
                wrote,
                published: None,
            },
            Some(mut written) => {
                written.wrote.extend(wrote);
                Written {
                    target: (written.target).filter(|one| target.as_ref() == Some(one)),
                    published: None,
                    ..written
                }
            }
        });
    }

    /// A command on the edit that wrote nothing, Undo, Redo, `GoTo`, a cancel or a refused
    /// one, `edit` the edit after it: nothing before it merges with what follows, and the
    /// changes not yet a step are forgotten where the edit is the last step's again, as
    /// after a cancelled gesture.
    pub(super) fn interrupted(&mut self, edit: &Edit) {
        self.merge = None;
        self.field_merge = None;
        if self.holds(edit) {
            self.written = None;
        }
    }

    /// A frame was published at `at`, including every change made so far: the first such
    /// frame for the last step's change, and for the changes not yet a step, starts their
    /// [`REVIEW`].
    pub(super) fn published(&mut self, at: Instant) {
        if let Some(written) = &mut self.written {
            written.published.get_or_insert(at);
        }
        if let Some(merge) = &mut self.merge {
            merge.published.get_or_insert(at);
        }
    }

    /// Undo, as [`Self::undo_in`] does, no field record touching the photo's fields.
    pub(super) fn undo(&mut self, edit: &Edit) -> Option<Edit> {
        self.undo_in(edit, &mut Vec::new())
    }

    /// Redo, as [`Self::redo_in`] does, no field record touching the photo's fields.
    pub(super) fn redo(&mut self, edit: &Edit) -> Option<Edit> {
        self.redo_in(edit, &mut Vec::new())
    }

    /// Undo, `edit` the edit as it stands, a change not yet a step made one first: the
    /// edit it leaves, or `None` where there is nothing to undo. A field record puts the
    /// photo's `fields` back, the edit as it was.
    pub(super) fn undo_in(&mut self, edit: &Edit, fields: &mut Vec<FieldEntry>) -> Option<Edit> {
        self.complete_pending(edit);
        let record = match self.done.pop() {
            Some(Record::Added { at, kept, held }) => {
                // The change's steps leave the log, and the steps they dropped come back.
                let steps = self.log.replace_after(at, held, at);
                Record::Added {
                    at,
                    kept,
                    held: steps,
                }
            }
            Some(Record::Moved { start, end }) => {
                self.log.set_end(start);
                Record::Moved { start, end }
            }
            Some(Record::GoTo { from, to }) => {
                self.log.set_end(from);
                Record::GoTo { from, to }
            }
            // The log as it was before, the log after held for Redo.
            Some(Record::Rewritten { what, log, saved }) => {
                let (log, saved) = self.swap(log, saved);
                Record::Rewritten { what, log, saved }
            }
            Some(Record::Field(record)) => return Some(self.put_fields(record, fields, true)),
            // The fallback: the end back over the step before it and its batch, which Redo
            // reverses; never over a default step, as darktable's auto-applied items are no
            // undo records.
            None => {
                let end = self.log.end();
                let start = self.log.undo_end()?;
                self.log.set_end(start);
                Record::Moved { start, end }
            }
        };
        self.undone.push(record);
        Some(self.moved())
    }

    /// Redo, `edit` the edit as it stands, a change not yet a step made one first: the
    /// edit it leaves, or `None` where there is nothing to redo. A field record sets the
    /// photo's `fields` again, the edit as it was.
    pub(super) fn redo_in(&mut self, edit: &Edit, fields: &mut Vec<FieldEntry>) -> Option<Edit> {
        self.complete_pending(edit);
        let record = match self.undone.pop() {
            Some(Record::Added { at, kept, held }) => {
                // The change's steps again, and the steps after the end dropped again.
                let end = at + held.len();
                let dropped = self.log.replace_after(at, held, end);
                Record::Added {
                    at,
                    kept,
                    held: dropped,
                }
            }
            Some(Record::Moved { start, end }) => {
                self.log.set_end(end);
                Record::Moved { start, end }
            }
            Some(Record::GoTo { from, to }) => {
                self.log.set_end(to);
                Record::GoTo { from, to }
            }
            // The log rewritten again, the log before held for Undo.
            Some(Record::Rewritten { what, log, saved }) => {
                let (log, saved) = self.swap(log, saved);
                Record::Rewritten { what, log, saved }
            }
            Some(Record::Field(record)) => return Some(self.put_fields(record, fields, false)),
            // The fallback: the end forward over the step after it and its batch, which Undo
            // reverses.
            None => {
                let start = self.log.end();
                let end = self.log.redo_end()?;
                self.log.set_end(end);
                Record::Moved { start, end }
            }
        };
        self.done.push(record);
        Some(self.moved())
    }

    /// `GoTo`: the end just after `step`, or at Original with `None`, `edit` the edit as it
    /// stands, a change not yet a step made one first, as for Undo. The edit it leaves, or
    /// `None` where the end is there already, as darktable's click on the item applied last
    /// does nothing (`src/libs/history.c:1265`); `Err` with a step not in the log, which
    /// changes nothing more. Undo takes it back, a record as darktable's click is
    /// (`:1297-1318`), and it clears the undone records. Any step, one inside a batch among
    /// them, as darktable's click on any item.
    pub(super) fn go_to(
        &mut self,
        edit: &Edit,
        step: Option<StepId>,
    ) -> Result<Option<Edit>, StepId> {
        self.complete_pending(edit);
        let to = match step {
            Some(id) => self.log.position(id).ok_or(id)? + 1,
            None => 0,
        };
        let from = self.log.end();
        if to == from {
            return Ok(None);
        }
        self.log.set_end(to);
        self.done.push(Record::GoTo { from, to });
        self.undone.clear();
        Ok(Some(self.moved()))
    }

    /// Compress, `edit` the edit as it stands, a change not yet a step made one first, as for
    /// Undo: the log keeps the default steps and each tool's last step up to the end
    /// ([`Log::compressed`]), a log left with no step given a default step of each of
    /// `defaults` again. With `up_to`, the end first just after that step, in the same
    /// record, so one Undo puts back the log and the end together. The edit it leaves, or
    /// `None` where it leaves the log as it is; `Err` with a step not in the log, as
    /// [`Self::go_to`] refuses it, which changes nothing more.
    pub(super) fn compress(
        &mut self,
        edit: &Edit,
        up_to: Option<StepId>,
        defaults: &[&str],
    ) -> Result<Option<Edit>, StepId> {
        self.complete_pending(edit);
        // Its clock, the list-only step's it writes the styles applied on too.
        let clock = Clock::now(self.clock.as_ref(), self.machine);
        let log = self.ending_at(up_to)?.compressed(clock);
        Ok(self.rewrite_at(log, Move::Compress, defaults, clock))
    }

    /// Truncate, as [`Self::compress`]: the person's steps after the end, or after `after`,
    /// dropped ([`Log::truncated`]).
    pub(super) fn truncate(
        &mut self,
        edit: &Edit,
        after: Option<StepId>,
        defaults: &[&str],
    ) -> Result<Option<Edit>, StepId> {
        self.complete_pending(edit);
        let log = self.ending_at(after)?.truncated();
        Ok(self.rewrite(log, Move::Truncate, defaults))
    }

    /// The log with its end just after `step`, as [`Self::go_to`] puts it, or as it is with
    /// `None`; `Err` with a step not in it.
    fn ending_at(&self, step: Option<StepId>) -> Result<Log, StepId> {
        let mut log = self.log.clone();
        if let Some(id) = step {
            log.set_end(log.position(id).ok_or(id)? + 1);
        }
        Ok(log)
    }

    /// Discard History, as [`Self::compress`]: going to Original and compressing, so the
    /// log holds no step and is given the default steps again, as a new photo is. `None` for
    /// a log of default steps alone never written with the end after them, which it would
    /// leave as it is.
    pub(super) fn discard(&mut self, edit: &Edit, defaults: &[&str]) -> Option<Edit> {
        self.complete_pending(edit);
        if self.unwritten() && self.log.end() == self.log.steps().len() {
            return None;
        }
        self.rewrite(Log::default(), Move::DiscardHistory, defaults)
    }

    /// The log becomes `log`, which `what` made, as one undo record holding the log before,
    /// which clears the undone records: a log with no step is given a default step of each
    /// of `defaults` at the rewrite's clock, the end after them, written with the first
    /// change, the file holding no step meanwhile. `None`, recording nothing, where `log` is
    /// the log already.
    fn rewrite(&mut self, log: Log, what: Move, defaults: &[&str]) -> Option<Edit> {
        let clock = Clock::now(self.clock.as_ref(), self.machine);
        self.rewrite_at(log, what, defaults, clock)
    }

    /// [`Self::rewrite`] at `clock`, the rewrite's own.
    fn rewrite_at(
        &mut self,
        mut log: Log,
        what: Move,
        defaults: &[&str],
        clock: Clock,
    ) -> Option<Edit> {
        if log == self.log {
            return None;
        }
        let saved = if log.steps().is_empty() {
            log.add_defaults(defaults, clock);
            false
        } else {
            self.saved
        };
        let (log, saved) = self.swap(log, saved);
        self.done.push(Record::Rewritten { what, log, saved });
        self.undone.clear();
        Some(self.moved_at(clock))
    }

    /// Puts `log` in the log's place, `saved` whether it was in the file, and returns the
    /// log it replaced with whether that was: the steps it takes out noted, those it brings
    /// back no longer, and the file cleared where it may hold steps and the log is now
    /// default steps alone never written.
    fn swap(&mut self, log: Log, saved: bool) -> (Log, bool) {
        let ids: HashSet<StepId> = log.steps().iter().map(|step| step.id).collect();
        let taken = (self.log.steps().iter()).filter(|step| !ids.contains(&step.id));
        self.dropped.extend(taken.map(|step| step.id));
        self.dropped.retain(|id| !ids.contains(id));
        // Whether the file may hold steps: the log was in it, or it is still to clear.
        self.cleared |= self.saved;
        (
            std::mem::replace(&mut self.log, log),
            std::mem::replace(&mut self.saved, saved),
        )
    }

    /// Starts the log afresh from `edit`, as Discard History does on an edit open for
    /// viewing only because of a [`crate::sidecar::Problem`]: a default step of each of
    /// `defaults`, as a new photo's log begins, then a step of each tool and operation it
    /// holds as far as it was read, sharing a batch, source You, noted `history-discard`,
    /// the end the last, the tools it holds not applied left so ([`Log::afresh`]). The
    /// file is written with it, or with no step for default steps alone with every one
    /// applied, which go into it with the first change; default steps alone with the end
    /// among them are written as they are, so the tools not applied stay so as the photo
    /// reopens. No undo record: the log it replaces could not be written back. The edit it
    /// leaves.
    pub(super) fn afresh(&mut self, edit: &Edit, defaults: &[&str]) -> Edit {
        let clock = Clock::now(self.clock.as_ref(), self.machine);
        let log = Log::afresh(edit, defaults, clock);
        let saved = log.end() < log.steps().len()
            || (log.steps().iter()).any(|step| step.source != StepSource::Default);
        (self.dropped).extend(self.log.steps().iter().map(|step| step.id));
        self.log = log;
        self.saved = saved;
        self.cleared = true;
        self.done.clear();
        self.undone.clear();
        self.written = None;
        self.moved_at(clock)
    }

    /// The field record `record` taken back onto the undone records, or with `back` false put
    /// back onto the done ones: each of `fields` it changed set as it was before, or after,
    /// at a clock of its own, which advances the copy's clock vector as a move of the end
    /// does, but one changed since. The edit as it stands.
    fn put_fields(
        &mut self,
        mut record: FieldRecord,
        fields: &mut Vec<FieldEntry>,
        back: bool,
    ) -> Edit {
        let clock = Clock::now(self.clock.as_ref(), self.machine);
        record.put(fields, back, clock);
        let other = if back {
            &mut self.undone
        } else {
            &mut self.done
        };
        other.push(Record::Field(record));
        self.moved_at(clock)
    }

    /// The clock the next change of a field takes, which [`Self::rated`] then makes the
    /// history's.
    pub(super) fn next_clock(&self) -> Clock {
        Clock::now(self.clock.as_ref(), self.machine)
    }

    /// A clock of its own, the vector advanced with it, for what a write adds that is no
    /// change of the person's: darktable's fields copied in at the first write.
    pub(super) fn stamp(&mut self) -> Clock {
        let clock = self.next_clock();
        self.clock = Some(clock);
        advance(&mut self.clocks, clock);
        clock
    }

    /// The photo's fields changed, as `record` holds, at its clock, sent at `at` (decision
    /// 110): an undo record of its own, or, a quick repeat of the change on top with nothing
    /// between, merged into it, so one Undo takes both back; no step. It clears the undone
    /// records, advances the copy's clock vector, and keeps the edit's next change from
    /// merging into the step before it.
    pub(super) fn rated(&mut self, record: FieldRecord, at: Instant) {
        let clock = record.clock();
        self.clock = Some(clock);
        advance(&mut self.clocks, clock);
        self.undone.clear();
        self.merge = None;
        self.changed = true;
        // A batch on several photos is its own record, so its Undo takes back no other.
        let merges = |top: &FieldRecord, began: Instant| {
            let alone = top.batch.is_none() && record.batch.is_none();
            alone && top.change.same_kind(&record.change) && at < began + MERGE_WITHIN
        };
        if let (Some(Record::Field(top)), Some(began)) = (self.done.last_mut(), self.field_merge)
            && merges(top, began)
        {
            top.merge(record);
            return;
        }
        self.done.push(Record::Field(record));
        self.field_merge = Some(at);
    }

    /// After Undo, Redo or `GoTo`: the edit the log folds to, which the next step is
    /// measured from; nothing before merges with what follows. The move advances the
    /// copy's clock vector with a clock of its own (decision 78).
    fn moved(&mut self) -> Edit {
        let clock = Clock::now(self.clock.as_ref(), self.machine);
        self.moved_at(clock)
    }

    /// [`Self::moved`] at `clock`, the move's own.
    fn moved_at(&mut self, clock: Clock) -> Edit {
        self.clock = Some(clock);
        advance(&mut self.clocks, clock);
        self.set_stepped(self.log.fold());
        self.merge = None;
        self.field_merge = None;
        self.changed = true;
        self.stepped.clone()
    }

    /// Whether the log, its end or the records changed since the last fact.
    pub(super) fn changed(&self) -> bool {
        self.changed
    }

    /// What Undo, or Redo where `undo` is false, would do now: the steps it would take back
    /// or put back, a record's or the fallback's.
    fn next(&self, undo: bool) -> Option<Move> {
        let records = if undo { &self.done } else { &self.undone };
        let (steps, end) = (self.log.steps(), self.log.end());
        let (start, end) = match records.last() {
            Some(Record::GoTo { .. }) => return Some(Move::GoTo),
            Some(Record::Field(record)) => {
                return Some(Move::Field {
                    change: record.change,
                    photos: 1,
                });
            }
            Some(Record::Rewritten { what, .. }) => return Some(*what),
            // The change's own steps, after the default steps it kept.
            Some(Record::Added { kept, held, .. }) if !undo => {
                let first = held.get(*kept)?;
                return Some(Move::Steps {
                    first: first.id,
                    count: held.len() - kept,
                    photos: 1,
                });
            }
            Some(Record::Added { at, kept, .. }) => (at + kept, steps.len()),
            Some(Record::Moved { start, end }) => (*start, *end),
            None if undo => (self.log.undo_end()?, end),
            None => (end, self.log.redo_end()?),
        };
        let first = steps.get(start)?;
        Some(Move::Steps {
            first: first.id,
            count: end - start,
            photos: 1,
        })
    }

    /// Whether Undo, or Redo where `redo`, would move over the steps of the batch `batch`
    /// now (decision 109): by its record, or by its fallback where it has none.
    pub(super) fn moves_batch(&self, batch: BatchId, redo: bool) -> bool {
        self.batch_moving(redo) == Some(batch)
    }

    /// The newest record of the batch `batch` setting the photo's fields, done or undone,
    /// as a batch on several photos reached it live and keeps it for the photo's other
    /// moves.
    pub(super) fn field_record(&self, batch: BatchId) -> Option<FieldRecord> {
        let mut records = self.done.iter().rev().chain(self.undone.iter().rev());
        records.find_map(|record| match record {
            Record::Field(record) if record.batch == Some(batch) => Some(record.clone()),
            _ => None,
        })
    }

    /// The batch whose steps Undo, or Redo where `redo`, would move over now, where they
    /// are in one: a rating's record's batch where that is the next record.
    pub(super) fn batch_moving(&self, redo: bool) -> Option<BatchId> {
        // A rating's record, by the batch it holds (decision 110).
        let records = if redo { &self.undone } else { &self.done };
        if let Some(Record::Field(record)) = records.last() {
            return record.batch;
        }
        let Some(Move::Steps { first, count, .. }) = self.next(!redo) else {
            return None;
        };
        let steps = || {
            let held = (self.undone.iter()).flat_map(|record| match record {
                Record::Added { held, .. } => held.as_slice(),
                _ => &[],
            });
            self.log.steps().iter().chain(held)
        };
        let batch = steps().find(|step| step.id == first)?.batch?;
        // Its whole batch, never the part of one `GoTo` left the end inside.
        let all = steps().filter(|step| step.batch == Some(batch)).count();
        (all == count).then_some(batch)
    }

    /// The `History` fact, through the first `through` commands, the photo's steps
    /// described on `image`; the history is then told.
    pub(super) fn fact(
        &mut self,
        through: u64,
        tools: &[&'static dyn ProcessingTool],
        operations: &'static [&'static dyn Operation],
        image: &ImageContext,
    ) -> History {
        self.changed = false;
        History {
            through,
            steps: (self.described).steps(&self.log, tools, operations, image),
            end: self.log.end(),
            undo: self.next(true),
            redo: self.next(false),
        }
    }
}

/// Whether `command`, applied, completes a change, which makes a step, and the step's
/// source and note (`docs/history.md`, What makes a step): a client's plain commands are
/// of source You, and a `Step` holding any command but `SetControl { done: false }` is of
/// its own.
fn completes(command: &Command) -> Option<(StepSource, Option<Note>)> {
    match command {
        Command::SetControl { done, .. } => done.then_some((StepSource::You, None)),
        Command::ResetControl { .. }
        | Command::ResetTool { .. }
        | Command::SetTool { .. }
        | Command::Keep { .. }
        | Command::ApplyPreset { .. } => Some((StepSource::You, None)),
        // Its step is noted `history-reset` as it is made.
        Command::Reset => Some((StepSource::You, None)),
        // `ApplyStyle` makes its own steps, in its batch, as on a photo that is not open
        // (`session/styles.rs`, decision 109).
        Command::ClearStyle { name, .. } => {
            Some((StepSource::Style, Some(Note::clear_style(name))))
        }
        Command::Step {
            source,
            note,
            commands,
        } if commands.iter().any(|command| !goes_on(command)) => Some((*source, note.clone())),
        _ => None,
    }
}

/// The step of a change that moved no value but applied the tools `unapplied`, a Reset or
/// a tool's reset with the end among the default steps, by what it wrote: a Reset holding
/// an empty entry for each, where it was a `Reset`, else the first tool it reset, empty.
fn applied_by(wrote: &[Wrote], unapplied: &BTreeSet<String>) -> Vec<Change> {
    if wrote.contains(&Wrote::All) {
        let entries = (unapplied.iter())
            .map(|tool| Entry::Tool {
                tool: tool.clone(),
                values: ToolValues::new(),
            })
            .collect();
        return vec![Change::Reset(entries)];
    }
    (wrote.iter())
        .find_map(|wrote| match wrote {
            Wrote::Tool(tool) => Some(Change::Entry(Entry::Tool {
                tool: tool.clone(),
                values: ToolValues::new(),
            })),
            _ => None,
        })
        .into_iter()
        .collect()
}

/// What `command` resets, the edit or a tool, or a `Step`'s commands in turn: with the end
/// among the default steps it applies the tools not applied (decision 78), and is a step
/// though it moves no value.
fn resets(command: &Command) -> Vec<Wrote> {
    match command {
        Command::Reset => vec![Wrote::All],
        Command::ResetTool { tool } => vec![Wrote::Tool(tool.clone())],
        Command::Step { commands, .. } => commands.iter().flat_map(resets).collect(),
        _ => Vec::new(),
    }
}

/// Whether `command` is a gesture's change, `SetControl { done: false }`.
fn goes_on(command: &Command) -> bool {
    matches!(command, Command::SetControl { done: false, .. })
}

/// The gesture `command` goes on with, or begins (decision 42): its tool, and the
/// source and note of its step. A `SetControl { done: false }`'s, of source You, or a
/// `Step`'s holding such changes alone, of its own, on the tool of the last of them.
pub(super) fn gesture_of(command: &Command) -> Option<(&str, StepSource, Option<&Note>)> {
    match command {
        Command::SetControl {
            tool, done: false, ..
        } => Some((tool, StepSource::You, None)),
        Command::Step {
            source,
            note,
            commands,
        } if commands.iter().all(goes_on) => match commands.last()? {
            Command::SetControl { tool, .. } => Some((tool, *source, note.as_ref())),
            _ => None,
        },
        _ => None,
    }
}

/// Whether `command` writes `tool`: names it, or, as `Reset`, writes every tool.
fn writes(command: &Command, tool: &str) -> bool {
    match command {
        Command::SetControl { tool: named, .. }
        | Command::ResetControl { tool: named, .. }
        | Command::ResetTool { tool: named }
        | Command::SetTool { tool: named, .. }
        | Command::Keep { tool: named, .. }
        | Command::ApplyPreset { tool: named, .. } => named == tool,
        Command::Reset => true,
        _ => false,
    }
}

/// What `command` writes: the tool it names, the operation a `Set` names, or every tool
/// and operation for `Reset`; nothing for any other, a `Step`'s left to
/// [`Engine::apply_step`], which knows which of its commands changed the edit.
fn wrote(command: &Command) -> Vec<Wrote> {
    match command {
        Command::SetControl { tool, .. }
        | Command::ResetControl { tool, .. }
        | Command::ResetTool { tool }
        | Command::SetTool { tool, .. }
        | Command::Keep { tool, .. }
        | Command::ApplyPreset { tool, .. } => vec![Wrote::Tool(tool.clone())],
        Command::Set { operation, .. } => vec![Wrote::Operation(operation.clone())],
        Command::Reset => vec![Wrote::All],
        _ => Vec::new(),
    }
}

/// The target `command` writes, which a change merges by: a control a `SetControl` or
/// `ResetControl` names, a preset row an `ApplyPreset` names, or a `Step`'s one command's;
/// `None` for a change of several, as a `Step` of several commands, or of none, as a tool
/// reset, a tool's values set whole, a keep, a `Set` or `Reset`.
fn target(command: &Command) -> Option<Target> {
    match command {
        Command::SetControl { tool, control, .. } | Command::ResetControl { tool, control } => {
            Some(Target::Control {
                tool: tool.clone(),
                control: control.clone(),
            })
        }
        Command::ApplyPreset { tool, preset, .. } => Some(Target::Preset {
            tool: tool.clone(),
            preset: preset.clone(),
        }),
        Command::Step { commands, .. } => match &commands[..] {
            [command] => target(command),
            _ => None,
        },
        _ => None,
    }
}

/// Whether a `Step` may hold `command`: the edit's own commands alone.
fn in_step(command: &Command) -> bool {
    matches!(
        command,
        Command::SetControl { .. }
            | Command::ResetControl { .. }
            | Command::ResetTool { .. }
            | Command::SetTool { .. }
            | Command::Keep { .. }
            | Command::ApplyPreset { .. }
            | Command::Set { .. }
            | Command::Reset
    )
}

impl Engine {
    /// Applies one command that changes the edit ([`Self::edit_command`]) and makes the
    /// steps it completes: a gesture under way that it ends, but for its own release or a
    /// `Step` that takes it in, is a step of its own first, ended as completed whether the
    /// command is taken or refused (decision 42); a completed change taken is one step,
    /// with its gesture; a release refused leaves its gesture a step of its own. A gesture
    /// has one source: only a change of its source and note goes on with it or takes it in.
    /// A `Step` of `Default`, or of nothing, is refused before the gesture is touched, and a
    /// `Step` refused begins no gesture.
    ///
    /// `at` is when the client sent it: a change taken that changes the edit writes its
    /// target then, which the next step may merge by, and one that changes nothing counts
    /// as none; a command on the edit that writes nothing, or is refused, keeps what came
    /// before it from merging with what follows, but a retried save.
    pub(super) fn apply_edit(
        &mut self,
        command: Command,
        at: Instant,
    ) -> Result<Effect, SessionError> {
        // A `Step` that cannot be one is refused before it touches the gesture, but, as
        // every refused command, it keeps what came before it from merging.
        if let Command::Step {
            source, commands, ..
        } = &command
        {
            let refused = if *source == StepSource::Default {
                Some(SessionError::DefaultSource)
            } else if commands.is_empty() {
                Some(SessionError::EmptyStep)
            } else {
                None
            };
            if let Some(refused) = refused {
                if let Some(open) = &mut self.open {
                    open.history.interrupted(&open.edit);
                }
                return Err(refused);
            }
        }
        // A one-shot pick's turning on, held for its pick, is a step of its own first.
        if on_edit(&command) {
            self.save_held_once();
        }
        let gesture = (self.open.as_ref())
            .and_then(|open| open.gesture.as_ref())
            .map(|gesture| (gesture.tool.clone(), gesture.source, gesture.note.clone()));
        // One source per gesture: only a change on its tool, of its source and note, goes
        // on with it or takes it in.
        let same = |tool: &str, source: StepSource, note: Option<&Note>| {
            (gesture.as_ref()).is_some_and(|(on, of, noted)| {
                on == tool && *of == source && noted.as_ref() == note
            })
        };
        let (releases, goes_on) = match (&command, gesture_of(&command)) {
            (_, Some((tool, source, note))) => (false, same(tool, source, note)),
            (Command::SetControl { tool, .. }, None) => (same(tool, StepSource::You, None), false),
            // A `Step` takes in the gesture whose tool its commands write, of its source
            // and note.
            (
                Command::Step {
                    source,
                    note,
                    commands,
                },
                None,
            ) => {
                let takes_in = |(tool, ..): &(String, _, _)| {
                    same(tool, *source, note.as_ref())
                        && commands.iter().any(|command| writes(command, tool))
                };
                (gesture.as_ref().is_some_and(takes_in), false)
            }
            (Command::CancelGesture { .. } | Command::RetrySave, None) => (false, true),
            _ => (false, false),
        };
        // The gesture's own step: of the source and note of the change that began it.
        let ended = gesture.map(|(_, source, note)| (source, note));
        let ends_gesture = ended.is_some() && on_edit(&command) && !goes_on;
        if let Some((source, note)) = &ended
            && ends_gesture
            && !releases
        {
            self.step(*source, note.clone());
        }
        let completes = completes(&command);
        // A `Step`, or a style's change, which reports what its commands wrote.
        let is_step = matches!(
            command,
            Command::Step { .. } | Command::ApplyStyle { .. } | Command::ClearStyle { .. }
        );
        // `ApplyStyle` tells its history what it wrote itself, as it makes its steps.
        let styled = matches!(command, Command::ApplyStyle { .. });
        let writes = (in_step(&command) || (is_step && !styled))
            .then(|| (target(&command), wrote(&command)));
        let retry = matches!(command, Command::RetrySave);
        let resets = resets(&command);
        let before = self.open.as_ref().map(|open| open.edit.clone());
        let mut applied = self.edit_command(command);
        // A Reset or a tool's reset applies the tools not applied, though it may move no
        // value: the edit at the photo's defaults with every tool applied, a step to save.
        // Inside a `Step` it is a write of its own, which its commands report only where a
        // value moved.
        if !resets.is_empty()
            && let (Ok(effect), Some(open)) = (&mut applied, &mut self.open)
            && !open.edit.unapplied.is_empty()
        {
            open.edit.unapplied.clear();
            open.unsaved = true;
            effect.render = true;
            if is_step {
                effect.wrote.extend(resets.iter().cloned());
            }
        }
        // A reset of the whole edit empties the styles applied, though it may move no tool.
        if resets.contains(&Wrote::All)
            && applied.is_ok()
            && let Some(open) = &mut self.open
        {
            open.history.reset_styles();
            // Emptying the list is a change to save, though the edit is as it was.
            open.unsaved |= !open.history.styles().is_empty();
        }
        if let Some(open) = &mut self.open {
            match (&applied, writes) {
                // A change that changes nothing counts as none.
                (Ok(effect), Some((target, mut wrote))) if before.as_ref() != Some(&open.edit) => {
                    // A `Step`'s, from its commands that changed the edit.
                    wrote.extend_from_slice(&effect.wrote);
                    open.history.wrote(target, wrote, at);
                }
                (Ok(_), Some(_)) => {}
                _ if !retry => open.history.interrupted(&open.edit),
                _ => {}
            }
        }
        let refused_ended = applied.is_err() && ends_gesture;
        match (&applied, completes, ended) {
            (Ok(_), Some((source, note)), _) => self.step(source, note),
            (Err(_), _, Some((source, note))) if releases => self.step(source, note),
            _ => {}
        }
        // A `Step` refused begins no gesture: one it would have begun is dropped, the
        // gesture before it having ended as a step of its own.
        if is_step
            && applied.is_err()
            && !goes_on
            && let Some(open) = &mut self.open
        {
            open.gesture = None;
        }
        // The tools not applied: the end's while the values are its, none once they move,
        // rendered where that changed them.
        if let Some(open) = &mut self.open {
            let unapplied = open.history.unapplied(&open.edit);
            if unapplied != open.edit.unapplied {
                open.edit.unapplied = unapplied;
                if let Ok(effect) = &mut applied {
                    effect.render = true;
                }
            }
        }
        if refused_ended && self.needs_save() {
            self.save();
        }
        applied
    }

    /// Makes what changed of the open photo's edit since its last step one step, of
    /// `source` with `note`; nothing on a photo open for viewing only.
    pub(super) fn step(&mut self, source: StepSource, note: Option<Note>) {
        if let Some(open) = &mut self.open
            && open.view_only.is_none()
        {
            open.history
                .complete(&open.edit, (source, note), None, wall_clock());
        }
    }

    /// Ends the gesture under way as a step of its own, of the source and note of the change
    /// that began it, as a command on the edit ends it (decision 42): before a change the
    /// session makes itself, as a pick, so the person's gesture is never a step of the
    /// session's source.
    pub(super) fn end_gesture(&mut self) {
        let gesture = (self.open.as_mut()).and_then(|open| open.gesture.take());
        if let Some(gesture) = gesture {
            self.step(gesture.source, gesture.note);
        }
    }

    /// Makes what changed of the open photo's edit since its last step one step: of the
    /// source and note of the change that began the gesture under way, else of source You
    /// with no note.
    pub(super) fn step_pending(&mut self) {
        if self.open.as_ref().is_some_and(as_read_only) {
            return;
        }
        let gesture = (self.open.as_ref()).and_then(|open| open.gesture.as_ref());
        let (source, note) = gesture.map_or((StepSource::You, None), |gesture| {
            (gesture.source, gesture.note.clone())
        });
        self.step(source, note);
    }

    /// [`Command::Step`]'s commands applied in order to the open photo's edit, in place, all
    /// or none: the edit, the held coordinates, whether the writer has the edit and the
    /// tools' memories are kept aside and put back should one be refused, which refuses the
    /// whole, naming its index.
    /// Its source, its steps and its gesture are left to [`Self::apply_edit`]. What it wrote
    /// is what its commands that changed the edit wrote, in order: one that changes nothing
    /// counts as none, as on its own.
    pub(super) fn apply_step(&mut self, commands: Vec<Command>) -> Result<Effect, SessionError> {
        let Some(open) = &self.open else {
            return Ok(Effect::default());
        };
        let save = commands.iter().any(|command| !goes_on(command));
        let kept = (
            open.edit.clone(),
            open.held.clone(),
            open.unsaved,
            open.memories.clone(),
        );
        let mut render = false;
        let mut written = Vec::new();
        for (index, command) in commands.into_iter().enumerate() {
            let writes = wrote(&command);
            let before = self.open.as_ref().map(|open| open.edit.clone());
            let applied = if in_step(&command) {
                self.apply_command(command)
            } else {
                Err(SessionError::NotInStep)
            };
            match applied {
                Ok(effect) => {
                    render |= effect.render;
                    if self.open.as_ref().map(|open| &open.edit) != before.as_ref() {
                        written.extend(writes);
                    }
                }
                Err(error) => {
                    if let Some(open) = &mut self.open {
                        (open.edit, open.held, open.unsaved, open.memories) = kept;
                    }
                    let error = Box::new(error);
                    return Err(SessionError::InStep { index, error });
                }
            }
        }
        Ok(Effect {
            render,
            save,
            cancelled: false,
            wrote: written,
        })
    }

    /// [`Command::Undo`], or [`Command::Redo`] where `redo`: the edit becomes the log's
    /// fold, a completed change, which saves it.
    pub(super) fn undo(&mut self, redo: bool) -> Result<Effect, SessionError> {
        let Some(open) = &mut self.open else {
            return Ok(Effect::default());
        };
        let edit = pending(open);
        let before = open.rating();
        let entries = fields::first_mut(&mut open.file);
        let moved = if redo {
            open.history.redo_in(&edit, entries)
        } else {
            open.history.undo_in(&edit, entries)
        };
        // A field put back is written as a change of its own, though the log stays as it is.
        open.fields_unsaved |= open.rating() != before;
        Ok(moved_to(open, moved, &self.tools))
    }

    /// [`Command::GoTo`]: the edit becomes the log's fold, a completed change, which saves
    /// it; a step not in the log refused.
    pub(super) fn go_to(&mut self, step: Option<StepId>) -> Result<Effect, SessionError> {
        let Some(open) = &mut self.open else {
            return Ok(Effect::default());
        };
        let edit = pending(open);
        let moved =
            (open.history.go_to(&edit, step)).map_err(|step| SessionError::UnknownStep { step })?;
        Ok(moved_to(open, moved, &self.tools))
    }

    /// [`Command::Compress`], [`Command::Truncate`] or [`Command::DiscardHistory`], as `what`
    /// names it: the log rewritten, the edit its fold, a completed change, which saves it;
    /// Compress's or Truncate's step not in the log refused, as [`Command::GoTo`]'s is.
    /// `DiscardHistory` on an edit open for viewing only, which [`Self::edit_command`] lets
    /// through only for a [`crate::sidecar::Problem`], any of them, never a newer build's
    /// edit, starts the log afresh, the file's unreadable parts going with the old log, and
    /// ends the view-only state.
    pub(super) fn rewrite(&mut self, what: Rewrite) -> Result<Effect, SessionError> {
        let defaults = crate::history::default_tools(&self.tools, self.operations);
        let Some(open) = &mut self.open else {
            return Ok(Effect::default());
        };
        if open.view_only.is_some() {
            let afresh = open.history.afresh(&open.edit, &defaults);
            open.view_only = None;
            open.photo.view_only = None;
            // What `Opened` says again: the image context the decode gave.
            *open.photo.image = open.image.clone();
            return Ok(moved_to(open, Some(afresh), &self.tools));
        }
        let edit = pending(open);
        let moved = match what {
            Rewrite::Compress(up_to) => open.history.compress(&edit, up_to, &defaults),
            Rewrite::Truncate(after) => open.history.truncate(&edit, after, &defaults),
            Rewrite::Discard => Ok(open.history.discard(&edit, &defaults)),
        };
        let moved = moved.map_err(|step| SessionError::UnknownStep { step })?;
        Ok(moved_to(open, moved, &self.tools))
    }

    /// The open photo's `History`, where it changed since it was last sent.
    pub(super) fn history_event(&mut self) -> Option<Event> {
        let open = self.open.as_mut().filter(|open| open.history.changed())?;
        // The batches Undo and Redo would move over, by their records, the steps Redo puts
        // back held outside the log.
        let moving = [false, true].map(|redo| open.history.batch_moving(redo));
        let mut fact = (open.history).fact(self.applied, &self.tools, self.operations, &open.image);
        // A remembered batch on several photos names how many (decision 109).
        let moves = [&mut fact.undo, &mut fact.redo].into_iter().zip(moving);
        for ((moved, batch), redo) in moves.zip([false, true]) {
            if let Some(Move::Steps { photos, .. } | Move::Field { photos, .. }) = moved {
                *photos = batch
                    .and_then(|batch| self.batch_photos(batch, redo))
                    .unwrap_or(1);
            }
        }
        Some(Event::History(fact))
    }
}

/// Whether `open`'s edit holds no change not yet a step but what a move of the end or the
/// photo's open made of the log's fold, the clamp and the tools redrawn ([`as_read`]),
/// which never becomes a step of its own: else an Undo would take back that step and land
/// where it began, never past it. Tool by tool: each tool's state is the fold's, as right
/// after a step, or the fold's as read ([`PhotoHistory::holds`]).
pub(super) fn as_read_only(open: &Open) -> bool {
    open.history.holds(&open.edit)
}

/// What the session makes of a log's fold as it reads one, as [`as_read`] does but telling
/// nothing: the photo's tools and image context, which [`PhotoHistory`] measures a change
/// of the tools it did not write from.
pub(super) struct Reader {
    tools: Vec<&'static dyn ProcessingTool>,
    image: ImageContext,
}

impl Reader {
    fn read(&self, edit: &Edit) -> Edit {
        let mut read = edit.clone();
        read.clamp_read(&self.tools, &self.image);
        read.redraw(&self.tools, &self.image);
        read
    }
}

/// `edit` as the session reads a log's fold, as a photo opens and after a move of the
/// history's end: held to the ranges its tools' values give on `image` (decisions 21 and
/// 30), each clamp warned of, then its tools redrawn as darktable's `gui_update` redraws
/// its modules ([`Edit::redraw`]); a change not yet a step, saved with its own tool's next
/// step ([`PhotoHistory::complete`]).
pub(super) fn as_read(edit: &mut Edit, tools: &[&dyn ProcessingTool], image: &ImageContext) {
    for warning in edit.clamp_read(tools, image) {
        tracing::warn!(target: "pipe", "{warning}");
    }
    edit.redraw(tools, image);
}

/// The edit a move of the end makes a step of first, as a change not yet a step: `open`'s,
/// or the log's fold where its only change is what reading it made ([`as_read_only`]).
fn pending(open: &Open) -> Edit {
    if as_read_only(open) {
        open.history.stepped().clone()
    } else {
        open.edit.clone()
    }
}

/// The open photo's edit after Undo, Redo, `GoTo`, Compress, Truncate or Discard History
/// moved the end or rewrote the log: `moved`, the log's fold,
/// rendered where it differs, and saved as a completed change, the end with it even where
/// the edit is as it was. With nothing moved, `None`, a change made a step first saves
/// alike.
fn moved_to(open: &mut Open, moved: Option<Edit>, tools: &[&dyn ProcessingTool]) -> Effect {
    let Some(mut edit) = moved else {
        return Effect {
            save: true,
            ..Effect::default()
        };
    };
    // Held to the ranges its tools' values give on the photo and redrawn, as an opened edit
    // is: a change not yet a step.
    as_read(&mut edit, tools, &open.image);
    let changed = edit != open.edit;
    open.edit = edit;
    open.remember(tools, Moment::Redrawn, None);
    open.held.prune(&open.edit);
    // The end is saved; a log of default steps never written is not ([`PhotoHistory::unwritten`]).
    open.unsaved = true;
    Effect {
        render: changed,
        save: true,
        cancelled: false,
        wrote: Vec::new(),
    }
}

#[cfg(test)]
mod tests;
