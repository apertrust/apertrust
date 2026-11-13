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
//! its log in memory, each completed change a step of each module it touched, and the undo
//! records, which Undo and Redo take back and put back as darktable's undo does
//! (`src/libs/history.c:664-686`).
//!
//! A completed change that touched several modules is a step for each, in the order the
//! modules were written, sharing a batch id, and one undo record, as darktable's undo group
//! (`dt_undo_start_group`, `src/common/history.c:157`); Reset is one step of every module
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
//! gesture under way on a module it writes, of its source and note.
//!
//! The log is the sidecar's (version 2, [`crate::sidecar`]), read as the photo opens and
//! saved after each completed change and each move of the end, with the copy's clock
//! vector, which each step and each move advances. A newer file found before a write is
//! taken in ([`PhotoHistory::take_in`]): the steps not in it laid on its log after its end.
//!
//! A change merges into the step before it as darktable's undo merges a widget's quick
//! repeats (`_dev_undo_start_record_target`, `src/develop/develop.c:1351-1370`): the same
//! target, a control or a preset row, of the same source, You or Auto, with no note, made
//! right after that step with no other command on the edit between, its first write within
//! [`MERGE_WITHIN`] of the step's first and, once a frame including the step's last change
//! is published, within [`REVIEW`] of that frame (`develop.c:885`); a change of several
//! modules never merges, nor is a step of a batch merged into. It adds no undo record, so
//! one Undo takes the merged step back whole. The times are the commands' own, as the
//! client sent them, and the frames' publications, both handed in, so a test sets them.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use apertrust_library::clock::{Clock, MachineId};
use apertrust_modules::Module;
use apertrust_plugin::{ImageContext, Operation};

use super::{Command, Effect, Engine, Event, Open, SessionError, on_edit};
use crate::edit::Edit;
use crate::history::{
    Change, Described, Entry, History, Log, Move, Note, Step, StepId, StepSource, Wrote, advance,
    default_lines, latest,
};

/// The open photo's log, its undo records and what its next step is measured from.
pub(super) struct PhotoHistory {
    log: Log,
    /// The edit as the log's last step left it, or as it was read: what the next step's
    /// entries are measured from, so a change not yet a step, a `Set` or a gesture under
    /// way, joins the next.
    stepped: Edit,
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
    /// Auto, no note, and of one module, so in no batch.
    merge: Option<Merge>,
}

/// What the sidecar holds of the log as the session last read it or the writer last wrote
/// it: its steps, by id and clock, and its copy's clock vector. A newer file is compared
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
    Control { module: String, control: String },
    /// A preset row an option was applied from.
    Preset { module: String, preset: String },
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

/// What one command on the history changed, which Undo takes back and Redo puts back.
#[derive(Debug)]
enum Record {
    /// One change's steps added at `at`, the end before them: one, or a batch's. `held` is
    /// what the log does not hold: while the record is done, the steps after the end that
    /// they dropped; while it is undone, the steps themselves.
    Added { at: usize, held: Vec<Step> },
    /// The end moved over the steps from `start` to `end`, one change's or the part of one
    /// that applied, by Undo's or Redo's fallback: Undo puts it at `start`, Redo at `end`.
    Moved { start: usize, end: usize },
    /// The end moved by a `GoTo` from `from` to `to`.
    GoTo { from: usize, to: usize },
}

impl PhotoHistory {
    /// The history of a photo whose sidecar holds `log`, with its copy's clock vector
    /// `clocks`, both empty for a photo never edited: the next step's clock follows the
    /// latest of them ([`latest`]).
    pub(super) fn read(log: Log, clocks: Vec<Clock>) -> Self {
        let machine = MachineId::current();
        let read = (log.steps().iter().map(|step| &step.clock)).chain(&clocks);
        PhotoHistory {
            clock: latest(read, machine),
            stepped: log.fold(),
            log,
            done: Vec::new(),
            undone: Vec::new(),
            machine,
            clocks,
            described: Described::default(),
            changed: true,
            written: None,
            merge: None,
        }
    }

    /// Its steps described anew, as the decode's image context differs from the one they
    /// were described on.
    pub(super) fn describe_anew(&mut self) {
        self.described = Described::default();
        self.changed = true;
    }

    /// The log, which the sidecar saves.
    pub(super) fn log(&self) -> &Log {
        &self.log
    }

    /// The copy's clock vector, which the sidecar saves with the log.
    pub(super) fn clocks(&self) -> &[Clock] {
        &self.clocks
    }

    /// The edit as the log's last step left it, which the next step is measured from.
    pub(super) fn stepped(&self) -> &Edit {
        &self.stepped
    }

    /// What the sidecar would hold of this history once written: its steps and its copy's
    /// clock vector.
    pub(super) fn base(&self) -> Base {
        Base::of(&self.log, &self.clocks)
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
    /// which is a move not kept too. Returns the edit the old log's last step left, which a change not yet a step
    /// is moved from onto the new [`Self::stepped`], and whether every move not yet saved
    /// was kept.
    pub(super) fn take_in(&mut self, log: Log, clocks: Vec<Clock>, base: &Base) -> (Edit, bool) {
        let theirs: HashSet<(StepId, Clock)> = (log.steps().iter())
            .map(|step| (step.id, step.clock))
            .collect();
        let mut log = log;
        let mut laid = false;
        for step in &self.log.steps()[..self.log.end()] {
            let key = (step.id, step.clock);
            if theirs.contains(&key) || base.steps.contains(&key) {
                continue;
            }
            let mut step = step.clone();
            if log.position(step.id).is_some() {
                step.id = log.new_id();
            }
            log.add(step);
            laid = true;
        }
        let ours = self.log.steps()[..self.log.end()]
            .last()
            .map(|step| step.id);
        let mut kept = true;
        let mut moved = false;
        if laid {
            // A step the base applied that this log took out, by its id, by an Undo before
            // the step laid, that applies again in the joined log. A step this log still
            // holds by id with a new clock, as a quick repeat merged into it, was not taken
            // out; nor was one the other side took out too, which applies nowhere; and one
            // the base had not applied, which the step laid dropped and the other side
            // redid, is the other side's move, no Undo of this session's.
            let held: HashSet<StepId> = self.log.steps().iter().map(|step| step.id).collect();
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
            kept = !again && !lost;
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
        self.changed = true;
        (std::mem::replace(&mut self.stepped, self.log.fold()), kept)
    }

    /// Makes what changed since the last step a step of each module it touched, of `source`
    /// with `note` ([`Change::between`]), and says whether it made any: a change that changes
    /// nothing makes none. Several share a batch id; a Reset's step is noted
    /// `history-reset`. They drop the steps after the end, which Undo brings back, and clear
    /// the undone records. A change of one module that merges into the step before it makes
    /// none of its own and adds no record: that step keeps its id, takes the change's entry
    /// and the latest clock.
    pub(super) fn complete(&mut self, edit: &Edit, source: StepSource, note: Option<Note>) -> bool {
        let written = self.written.take();
        let wrote = written
            .as_ref()
            .map_or(&[][..], |written| &written.wrote[..]);
        let changes = Change::between(&self.stepped, edit, wrote);
        if changes.is_empty() {
            return false;
        }
        let clock = Clock::now(self.clock.as_ref(), self.machine);
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
            let one = changes.len() == 1;
            let at = self.log.end();
            let dropped = self.log.add_change(changes, source, note, clock);
            self.done.push(Record::Added { at, held: dropped });
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
        self.stepped = edit.clone();
        self.changed = true;
        true
    }

    /// A command on the edit, sent at `at`, wrote `target`, or, with `None`, wrote several
    /// or none, and wrote the modules and operations `wrote`, in order: the changes not yet
    /// a step go on, or begin with it.
    pub(super) fn wrote(&mut self, target: Option<Target>, wrote: Vec<Wrote>, at: Instant) {
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
        if Entry::between(&self.stepped, edit).is_empty() {
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

    /// Undo, `edit` the edit as it stands, a change not yet a step made one first: the
    /// edit it leaves, or `None` where there is nothing to undo.
    pub(super) fn undo(&mut self, edit: &Edit) -> Option<Edit> {
        self.complete(edit, StepSource::You, None);
        let record = match self.done.pop() {
            Some(Record::Added { at, held }) => {
                // The change's steps leave the log, and the steps they dropped come back.
                let steps = self.log.replace_after(at, held, at);
                Record::Added { at, held: steps }
            }
            Some(Record::Moved { start, end }) => {
                self.log.set_end(start);
                Record::Moved { start, end }
            }
            Some(Record::GoTo { from, to }) => {
                self.log.set_end(from);
                Record::GoTo { from, to }
            }
            // The fallback: the end back over the step before it and its batch, which Redo
            // reverses.
            None => {
                let end = self.log.end();
                let start = self.log.batch_start(end)?;
                self.log.set_end(start);
                Record::Moved { start, end }
            }
        };
        self.undone.push(record);
        Some(self.moved())
    }

    /// Redo, `edit` the edit as it stands, a change not yet a step made one first: the
    /// edit it leaves, or `None` where there is nothing to redo.
    pub(super) fn redo(&mut self, edit: &Edit) -> Option<Edit> {
        self.complete(edit, StepSource::You, None);
        let record = match self.undone.pop() {
            Some(Record::Added { at, held }) => {
                // The change's steps again, and the steps after the end dropped again.
                let end = at + held.len();
                let dropped = self.log.replace_after(at, held, end);
                Record::Added { at, held: dropped }
            }
            Some(Record::Moved { start, end }) => {
                self.log.set_end(end);
                Record::Moved { start, end }
            }
            Some(Record::GoTo { from, to }) => {
                self.log.set_end(to);
                Record::GoTo { from, to }
            }
            // The fallback: the end forward over the step after it and its batch, which Undo
            // reverses.
            None => {
                let start = self.log.end();
                let end = self.log.batch_end(start)?;
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
        self.complete(edit, StepSource::You, None);
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

    /// After Undo, Redo or `GoTo`: the edit the log folds to, which the next step is
    /// measured from; nothing before merges with what follows. The move advances the
    /// copy's clock vector with a clock of its own (decision 78).
    fn moved(&mut self) -> Edit {
        let clock = Clock::now(self.clock.as_ref(), self.machine);
        self.clock = Some(clock);
        advance(&mut self.clocks, clock);
        self.stepped = self.log.fold();
        self.merge = None;
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
            Some(Record::Added { held, .. }) if !undo => {
                let first = held.first()?;
                return Some(Move::Steps {
                    first: first.id,
                    count: held.len(),
                });
            }
            Some(Record::Added { at, .. }) => (*at, steps.len()),
            Some(Record::Moved { start, end }) => (*start, *end),
            None if undo => (self.log.batch_start(end)?, end),
            None => (end, self.log.batch_end(end)?),
        };
        let first = steps.get(start)?;
        Some(Move::Steps {
            first: first.id,
            count: end - start,
        })
    }

    /// The `History` fact, through the first `through` commands, the photo's steps
    /// described on `image`; the history is then told.
    pub(super) fn fact(
        &mut self,
        through: u64,
        modules: &[&'static dyn Module],
        operations: &'static [&'static dyn Operation],
        image: &ImageContext,
    ) -> History {
        self.changed = false;
        History {
            through,
            steps: (self.described).steps(&self.log, modules, operations, image),
            end: self.log.end(),
            defaults: default_lines(modules, operations),
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
        | Command::ResetModule { .. }
        | Command::Keep { .. }
        | Command::ApplyPreset { .. } => Some((StepSource::You, None)),
        // Its step is noted `history-reset` as it is made.
        Command::Reset => Some((StepSource::You, None)),
        Command::Step {
            source,
            note,
            commands,
        } if commands.iter().any(|command| !goes_on(command)) => Some((*source, note.clone())),
        _ => None,
    }
}

/// Whether `command` is a gesture's change, `SetControl { done: false }`.
fn goes_on(command: &Command) -> bool {
    matches!(command, Command::SetControl { done: false, .. })
}

/// The gesture `command` goes on with, or begins (decision 42): its module, and the
/// source and note of its step. A `SetControl { done: false }`'s, of source You, or a
/// `Step`'s holding such changes alone, of its own, on the module of the last of them.
pub(super) fn gesture_of(command: &Command) -> Option<(&str, StepSource, Option<&Note>)> {
    match command {
        Command::SetControl {
            module,
            done: false,
            ..
        } => Some((module, StepSource::You, None)),
        Command::Step {
            source,
            note,
            commands,
        } if commands.iter().all(goes_on) => match commands.last()? {
            Command::SetControl { module, .. } => Some((module, *source, note.as_ref())),
            _ => None,
        },
        _ => None,
    }
}

/// Whether `command` writes `module`: names it, or, as `Reset`, writes every module.
fn writes(command: &Command, module: &str) -> bool {
    match command {
        Command::SetControl { module: named, .. }
        | Command::ResetControl { module: named, .. }
        | Command::ResetModule { module: named }
        | Command::Keep { module: named, .. }
        | Command::ApplyPreset { module: named, .. } => named == module,
        Command::Reset => true,
        _ => false,
    }
}

/// What `command` writes: the module it names, the operation a `Set` names, or every module
/// and operation for `Reset`; nothing for any other, a `Step`'s left to
/// [`Engine::apply_step`], which knows which of its commands changed the edit.
fn wrote(command: &Command) -> Vec<Wrote> {
    match command {
        Command::SetControl { module, .. }
        | Command::ResetControl { module, .. }
        | Command::ResetModule { module }
        | Command::Keep { module, .. }
        | Command::ApplyPreset { module, .. } => vec![Wrote::Module(module.clone())],
        Command::Set { operation, .. } => vec![Wrote::Operation(operation.clone())],
        Command::Reset => vec![Wrote::All],
        _ => Vec::new(),
    }
}

/// The target `command` writes, which a change merges by: a control a `SetControl` or
/// `ResetControl` names, a preset row an `ApplyPreset` names, or a `Step`'s one command's;
/// `None` for a change of several, as a `Step` of several commands, or of none, as a module
/// reset, a keep, a `Set` or `Reset`.
fn target(command: &Command) -> Option<Target> {
    match command {
        Command::SetControl {
            module, control, ..
        }
        | Command::ResetControl { module, control } => Some(Target::Control {
            module: module.clone(),
            control: control.clone(),
        }),
        Command::ApplyPreset { module, preset, .. } => Some(Target::Preset {
            module: module.clone(),
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
            | Command::ResetModule { .. }
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
        let gesture = (self.open.as_ref())
            .and_then(|open| open.gesture.as_ref())
            .map(|gesture| (gesture.module.clone(), gesture.source, gesture.note.clone()));
        // One source per gesture: only a change on its module, of its source and note, goes
        // on with it or takes it in.
        let same = |module: &str, source: StepSource, note: Option<&Note>| {
            (gesture.as_ref()).is_some_and(|(on, of, noted)| {
                on == module && *of == source && noted.as_ref() == note
            })
        };
        let (releases, goes_on) = match (&command, gesture_of(&command)) {
            (_, Some((module, source, note))) => (false, same(module, source, note)),
            (Command::SetControl { module, .. }, None) => {
                (same(module, StepSource::You, None), false)
            }
            // A `Step` takes in the gesture whose module its commands write, of its source
            // and note.
            (
                Command::Step {
                    source,
                    note,
                    commands,
                },
                None,
            ) => {
                let takes_in = |(module, ..): &(String, _, _)| {
                    same(module, *source, note.as_ref())
                        && commands.iter().any(|command| writes(command, module))
                };
                (gesture.as_ref().is_some_and(takes_in), false)
            }
            (Command::CancelGesture { .. } | Command::RetrySave, None) => (false, true),
            _ => (false, false),
        };
        // The gesture's own step: of the source and note of the change that began it.
        let ended = gesture.map(|(_, source, note)| (source, note));
        if let Some((source, note)) = &ended
            && on_edit(&command)
            && !releases
            && !goes_on
        {
            self.step(*source, note.clone());
        }
        let completes = completes(&command);
        let is_step = matches!(command, Command::Step { .. });
        let writes = (in_step(&command) || is_step).then(|| (target(&command), wrote(&command)));
        let retry = matches!(command, Command::RetrySave);
        let before = self.open.as_ref().map(|open| open.edit.clone());
        let applied = self.edit_command(command);
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
        applied
    }

    /// Makes what changed of the open photo's edit since its last step one step, of
    /// `source` with `note`; nothing on a photo open for viewing only.
    pub(super) fn step(&mut self, source: StepSource, note: Option<Note>) {
        if let Some(open) = &mut self.open
            && open.view_only.is_none()
        {
            open.history.complete(&open.edit, source, note);
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
        if self
            .open
            .as_ref()
            .is_some_and(|open| clamp_only(open, &self.modules))
        {
            return;
        }
        let gesture = (self.open.as_ref()).and_then(|open| open.gesture.as_ref());
        let (source, note) = gesture.map_or((StepSource::You, None), |gesture| {
            (gesture.source, gesture.note.clone())
        });
        self.step(source, note);
    }

    /// [`Command::Step`]'s commands applied in order to the open photo's edit, in place, all
    /// or none: the edit, the held coordinates and whether the writer has the edit are kept
    /// aside and put back should one be refused, which refuses the whole, naming its index.
    /// Its source, its steps and its gesture are left to [`Self::apply_edit`]. What it wrote
    /// is what its commands that changed the edit wrote, in order: one that changes nothing
    /// counts as none, as on its own.
    pub(super) fn apply_step(&mut self, commands: Vec<Command>) -> Result<Effect, SessionError> {
        let Some(open) = &self.open else {
            return Ok(Effect::default());
        };
        let save = commands.iter().any(|command| !goes_on(command));
        let kept = (open.edit.clone(), open.held.clone(), open.unsaved);
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
                        (open.edit, open.held, open.unsaved) = kept;
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
        let edit = pending(open, &self.modules);
        let moved = if redo {
            open.history.redo(&edit)
        } else {
            open.history.undo(&edit)
        };
        Ok(moved_to(open, moved, &self.modules))
    }

    /// [`Command::GoTo`]: the edit becomes the log's fold, a completed change, which saves
    /// it; a step not in the log refused.
    pub(super) fn go_to(&mut self, step: Option<StepId>) -> Result<Effect, SessionError> {
        let Some(open) = &mut self.open else {
            return Ok(Effect::default());
        };
        let edit = pending(open, &self.modules);
        let moved =
            (open.history.go_to(&edit, step)).map_err(|step| SessionError::UnknownStep { step })?;
        Ok(moved_to(open, moved, &self.modules))
    }

    /// The open photo's `History`, where it changed since it was last sent.
    pub(super) fn history_event(&mut self) -> Option<Event> {
        let open = self.open.as_mut().filter(|open| open.history.changed())?;
        let fact = (open.history).fact(self.applied, &self.modules, self.operations, &open.image);
        Some(Event::History(fact))
    }
}

/// Whether the only change of `open`'s edit not yet a step is the clamp a move of the end or
/// the photo's open made of the log's fold, which never becomes a step of its own: else an
/// Undo would take back that clamp's step and land where it began, never past it.
pub(super) fn clamp_only(open: &Open, modules: &[&dyn Module]) -> bool {
    let mut clamped = open.history.stepped().clone();
    clamped.clamp_read(modules, &open.image);
    clamped == open.edit
}

/// The edit a move of the end makes a step of first, as a change not yet a step: `open`'s,
/// or the log's fold where its only change is a clamp ([`clamp_only`]).
fn pending(open: &Open, modules: &[&dyn Module]) -> Edit {
    if clamp_only(open, modules) {
        open.history.stepped().clone()
    } else {
        open.edit.clone()
    }
}

/// The open photo's edit after Undo, Redo or `GoTo` moved the end: `moved`, the log's fold,
/// rendered where it differs, and saved as a completed change, the end with it even where
/// the edit is as it was. With nothing moved, `None`, a change made a step first saves
/// alike.
fn moved_to(open: &mut Open, moved: Option<Edit>, modules: &[&dyn Module]) -> Effect {
    let Some(mut edit) = moved else {
        return Effect {
            save: true,
            ..Effect::default()
        };
    };
    // Held to the ranges its modules' values give on the photo, as an opened edit is: the
    // clamp is a change not yet a step.
    for warning in edit.clamp_read(modules, &open.image) {
        tracing::warn!(target: "pipe", "{warning}");
    }
    let changed = edit != open.edit;
    open.edit = edit;
    open.held.prune(&open.edit);
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
