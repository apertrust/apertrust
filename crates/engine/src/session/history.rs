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
//! its log in memory, each completed change a step, and the undo records, which Undo and
//! Redo take back and put back as darktable's undo does (`src/libs/history.c:664-686`).
//!
//! Two stacks, the done records and the undone ones: Undo takes the newest done record
//! back onto the undone stack, Redo the newest undone one back onto the done stack, and a
//! new step clears the undone stack. Only when the done stack is empty, as after
//! reopening a photo, does Undo move the end back one step, and only when the undone stack
//! is empty does Redo move it forward one; each such move is pushed onto the other stack,
//! so the next Redo or Undo reverses it. Both stacks end as the photo closes.
//!
//! `GoTo` moves the end, a record of its own, as darktable's click on a history item is
//! (`src/libs/history.c:1297-1318`). `Step` makes several commands one step of its source
//! and note, all or none, taking in the gesture under way on a module it writes, of its
//! source and note.
//!
//! A change merges into the step before it as darktable's undo merges a widget's quick
//! repeats (`_dev_undo_start_record_target`, `src/develop/develop.c:1351-1370`): the same
//! target, a control or a preset row, of the same source, You or Auto, with no note, made
//! right after that step with no other command on the edit between, its first write within
//! [`MERGE_WITHIN`] of the step's first and, once a frame including the step's last change
//! is published, within [`REVIEW`] of that frame (`develop.c:885`). It adds no undo record,
//! so one Undo takes the merged step back whole. The times are the commands' own, as the
//! client sent them, and the frames' publications, both handed in, so a test sets them.

use std::time::{Duration, Instant, SystemTime};

use apertrust_library::clock::{Clock, MachineId};
use apertrust_modules::Module;
use apertrust_plugin::{ImageContext, Operation};

use super::{Command, Effect, Engine, Event, Open, SessionError, on_edit};
use crate::edit::Edit;
use crate::history::{
    Described, Entry, History, Log, Move, Note, Step, StepId, StepSource, default_lines,
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
    /// The file the edit was read from: its bytes' hash and when it was last modified.
    file: (Option<[u8; 32]>, Option<SystemTime>),
    described: Described,
    /// Whether the log, its end or the records changed since the last `History` sent.
    changed: bool,
    /// The changes not yet a step, from their first write; `None` while nothing is
    /// written.
    written: Option<Written>,
    /// The step the next change may merge into: the log's last, at the end, made with no
    /// command on the edit since but changes not yet a step, of one target, source You or
    /// Auto, no note and no batch.
    merge: Option<Merge>,
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
    /// A step added at `at`, the end before it. `held` is what the log does not hold:
    /// while the record is done, the steps after the end that the step dropped; while it
    /// is undone, the step itself.
    Added { at: usize, held: Vec<Step> },
    /// The end moved over the step at `at`, between `at` and `at + 1`, by Undo's or Redo's
    /// fallback.
    Moved { at: usize },
    /// The end moved by a `GoTo` from `from` to `to`.
    GoTo { from: usize, to: usize },
}

impl PhotoHistory {
    /// The history of a photo whose edit was read as `edit` from a file whose bytes hash
    /// to `hash` and which was last modified at `modified`, none for a photo never
    /// edited: the file is version 1, read as one step ([`Log::read`]).
    pub(super) fn read(edit: &Edit, hash: Option<[u8; 32]>, modified: Option<SystemTime>) -> Self {
        let log = Log::read(edit, hash, modified);
        let clock = log.steps().last().map(|step| step.clock);
        PhotoHistory {
            stepped: log.fold(),
            log,
            done: Vec::new(),
            undone: Vec::new(),
            machine: MachineId::current(),
            clock,
            file: (hash, modified),
            described: Described::default(),
            changed: true,
            written: None,
            merge: None,
        }
    }

    /// The same file read again as `edit`, as the decode's image context clamps it before
    /// any command can change it: its step holds that, and every step is described anew
    /// on the new context.
    pub(super) fn read_again(&mut self, edit: &Edit) {
        let (hash, modified) = self.file;
        *self = PhotoHistory::read(edit, hash, modified);
    }

    /// The log, for a test.
    #[cfg(test)]
    pub(super) fn log(&self) -> &Log {
        &self.log
    }

    /// Makes what changed since the last step one step, of `source` with `note`, and says
    /// whether it did: a change that changes nothing makes none. It drops the steps after
    /// the end, which Undo brings back, and clears the undone records. A change that merges
    /// into the step before it makes none of its own and adds no record: that step keeps
    /// its id, takes the change's entries and the latest clock.
    pub(super) fn complete(&mut self, edit: &Edit, source: StepSource, note: Option<Note>) -> bool {
        let written = self.written.take();
        let entries = Entry::between(&self.stepped, edit);
        if entries.is_empty() {
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
        match (&mut self.merge, written) {
            (Some(merge), Some(written)) if merges(merge, &written) => {
                self.log.merge(entries, clock);
                merge.published = written.published;
            }
            (merge, written) => {
                let step = Step {
                    id: self.log.new_id(),
                    clock,
                    source,
                    note,
                    batch: None,
                    entries,
                };
                let at = self.log.end();
                let dropped = self.log.add(step);
                self.done.push(Record::Added { at, held: dropped });
                self.undone.clear();
                *merge = target.zip(written).map(|(target, written)| Merge {
                    target,
                    source,
                    began: written.began,
                    published: written.published,
                });
            }
        }
        self.clock = Some(clock);
        self.stepped = edit.clone();
        self.changed = true;
        true
    }

    /// A command on the edit, sent at `at`, wrote `target`, or, with `None`, wrote several
    /// or none: the changes not yet a step go on, or begin with it.
    pub(super) fn wrote(&mut self, target: Option<Target>, at: Instant) {
        self.written = Some(match self.written.take() {
            None => Written {
                began: at,
                target,
                published: None,
            },
            Some(written) => Written {
                target: (written.target).filter(|one| target.as_ref() == Some(one)),
                published: None,
                ..written
            },
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
                // The step leaves the log, and the steps it dropped come back.
                let step = self.log.replace_after(at, held, at);
                Record::Added { at, held: step }
            }
            Some(Record::Moved { at }) => {
                self.log.set_end(at);
                Record::Moved { at }
            }
            Some(Record::GoTo { from, to }) => {
                self.log.set_end(from);
                Record::GoTo { from, to }
            }
            // The fallback: the end back one step, which Redo reverses.
            None => {
                let at = self.log.end().checked_sub(1)?;
                self.log.set_end(at);
                Record::Moved { at }
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
                // The step again, and the steps after the end dropped again.
                let dropped = self.log.replace_after(at, held, at + 1);
                Record::Added { at, held: dropped }
            }
            Some(Record::Moved { at }) => {
                self.log.set_end(at + 1);
                Record::Moved { at }
            }
            Some(Record::GoTo { from, to }) => {
                self.log.set_end(to);
                Record::GoTo { from, to }
            }
            // The fallback: the end forward one step, which Undo reverses.
            None if self.log.end() < self.log.steps().len() => {
                let at = self.log.end();
                self.log.set_end(at + 1);
                Record::Moved { at }
            }
            None => return None,
        };
        self.done.push(record);
        Some(self.moved())
    }

    /// `GoTo`: the end just after `step`, or at Original with `None`, `edit` the edit as it
    /// stands, a change not yet a step made one first, as for Undo. The edit it leaves, or
    /// `None` where the end is there already, as darktable's click on the item applied last
    /// does nothing (`src/libs/history.c:1265`); `Err` with a step not in the log, which
    /// changes nothing more. Undo takes it back, a record as darktable's click is
    /// (`:1297-1318`), and it clears the undone records.
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
    /// measured from; nothing before merges with what follows.
    fn moved(&mut self) -> Edit {
        self.stepped = self.log.fold();
        self.merge = None;
        self.changed = true;
        self.stepped.clone()
    }

    /// Whether the log, its end or the records changed since the last fact.
    pub(super) fn changed(&self) -> bool {
        self.changed
    }

    /// What Undo, or Redo where `undo` is false, would do now: the step it would take back
    /// or put back, a record's or the fallback's.
    fn next(&self, undo: bool) -> Option<Move> {
        let records = if undo { &self.done } else { &self.undone };
        let step = match records.last() {
            Some(Record::GoTo { .. }) => return Some(Move::GoTo),
            Some(Record::Added { held, .. }) if !undo => held.first(),
            Some(Record::Added { at, .. } | Record::Moved { at }) => self.log.steps().get(*at),
            None if undo => (self.log.end().checked_sub(1)).and_then(|at| self.log.steps().get(at)),
            None => self.log.steps().get(self.log.end()),
        };
        step.map(|step| Move::Step(step.id))
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
        Command::Reset => Some((StepSource::You, Some(Note::reset()))),
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
        let writes = (in_step(&command) || is_step).then(|| target(&command));
        let retry = matches!(command, Command::RetrySave);
        let before = self.open.as_ref().map(|open| open.edit.clone());
        let applied = self.edit_command(command);
        if let Some(open) = &mut self.open {
            match (&applied, writes) {
                // A change that changes nothing counts as none.
                (Ok(_), Some(target)) if before.as_ref() != Some(&open.edit) => {
                    open.history.wrote(target, at);
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

    /// Makes what changed of the open photo's edit since its last step one step: of the
    /// source and note of the change that began the gesture under way, else of source You
    /// with no note.
    pub(super) fn step_pending(&mut self) {
        let gesture = (self.open.as_ref()).and_then(|open| open.gesture.as_ref());
        let (source, note) = gesture.map_or((StepSource::You, None), |gesture| {
            (gesture.source, gesture.note.clone())
        });
        self.step(source, note);
    }

    /// [`Command::Step`]'s commands applied in order to the open photo's edit, in place, all
    /// or none: the edit, the held coordinates and whether the writer has the edit are kept
    /// aside and put back should one be refused, which refuses the whole, naming its index.
    /// Its source, its steps and its gesture are left to [`Self::apply_edit`].
    pub(super) fn apply_step(&mut self, commands: Vec<Command>) -> Result<Effect, SessionError> {
        let Some(open) = &self.open else {
            return Ok(Effect::default());
        };
        let save = commands.iter().any(|command| !goes_on(command));
        let kept = (open.edit.clone(), open.held.clone(), open.unsaved);
        let mut render = false;
        for (index, command) in commands.into_iter().enumerate() {
            let applied = if in_step(&command) {
                self.apply_command(command)
            } else {
                Err(SessionError::NotInStep)
            };
            match applied {
                Ok(effect) => render |= effect.render,
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
        })
    }

    /// [`Command::Undo`], or [`Command::Redo`] where `redo`: the edit becomes the log's
    /// fold, a completed change, which saves it.
    pub(super) fn undo(&mut self, redo: bool) -> Result<Effect, SessionError> {
        let Some(open) = &mut self.open else {
            return Ok(Effect::default());
        };
        let moved = if redo {
            open.history.redo(&open.edit)
        } else {
            open.history.undo(&open.edit)
        };
        Ok(moved_to(open, moved))
    }

    /// [`Command::GoTo`]: the edit becomes the log's fold, a completed change, which saves
    /// it; a step not in the log refused.
    pub(super) fn go_to(&mut self, step: Option<StepId>) -> Result<Effect, SessionError> {
        let Some(open) = &mut self.open else {
            return Ok(Effect::default());
        };
        let moved = (open.history.go_to(&open.edit, step))
            .map_err(|step| SessionError::UnknownStep { step })?;
        Ok(moved_to(open, moved))
    }

    /// The open photo's `History`, where it changed since it was last sent.
    pub(super) fn history_event(&mut self) -> Option<Event> {
        let open = self.open.as_mut().filter(|open| open.history.changed())?;
        let fact = (open.history).fact(self.applied, &self.modules, self.operations, &open.image);
        Some(Event::History(fact))
    }
}

/// The open photo's edit after Undo, Redo or `GoTo` moved the end: `moved`, the log's fold,
/// rendered where it differs, and saved as a completed change. With nothing moved, `None`,
/// a change made a step first saves alike.
fn moved_to(open: &mut Open, moved: Option<Edit>) -> Effect {
    let Some(edit) = moved else {
        return Effect {
            save: true,
            ..Effect::default()
        };
    };
    let changed = edit != open.edit;
    open.edit = edit;
    open.held.prune(&open.edit);
    open.unsaved |= changed;
    Effect {
        render: changed,
        save: true,
        cancelled: false,
    }
}

#[cfg(test)]
mod tests;
