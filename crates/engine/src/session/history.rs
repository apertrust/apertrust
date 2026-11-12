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

use std::time::SystemTime;

use apertrust_library::clock::{Clock, MachineId};
use apertrust_modules::Module;
use apertrust_plugin::{ImageContext, Operation};

use super::{Command, Effect, Engine, Event, SessionError, on_edit};
use crate::edit::Edit;
use crate::history::{Described, Entry, History, Log, Move, Note, Step, StepSource, default_lines};

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
    /// the end, which Undo brings back, and clears the undone records.
    pub(super) fn complete(&mut self, edit: &Edit, source: StepSource, note: Option<Note>) -> bool {
        let entries = Entry::between(&self.stepped, edit);
        if entries.is_empty() {
            return false;
        }
        let clock = Clock::now(self.clock.as_ref(), self.machine);
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
        self.clock = Some(clock);
        self.stepped = edit.clone();
        self.changed = true;
        true
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

    /// After Undo or Redo: the edit the log folds to, which the next step is measured
    /// from.
    fn moved(&mut self) -> Edit {
        self.stepped = self.log.fold();
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
/// note (`docs/history.md`, What makes a step).
fn completes(command: &Command) -> Option<Option<Note>> {
    match command {
        Command::SetControl { done, .. } => done.then_some(None),
        Command::ResetControl { .. }
        | Command::ResetModule { .. }
        | Command::Keep { .. }
        | Command::ApplyPreset { .. } => Some(None),
        Command::Reset => Some(Some(Note::reset())),
        _ => None,
    }
}

impl Engine {
    /// Applies one command that changes the edit ([`Self::edit_command`]) and makes the
    /// steps it completes: a gesture under way that it ends, but for its own release, is a
    /// step of its own first, ended as completed whether the command is taken or refused
    /// (decision 42); a completed change taken is one step, with its gesture; a release
    /// refused leaves its gesture a step of its own.
    pub(super) fn apply_edit(&mut self, command: Command) -> Result<Effect, SessionError> {
        let gesture = (self.open.as_ref())
            .and_then(|open| open.gesture.as_ref())
            .map(|gesture| gesture.module.clone());
        let on_gesture = |module: &String| gesture.as_ref() == Some(module);
        let (releases, goes_on) = match &command {
            Command::SetControl { module, done, .. } => {
                (*done && on_gesture(module), !*done && on_gesture(module))
            }
            Command::CancelGesture { .. } | Command::RetrySave => (false, true),
            _ => (false, false),
        };
        if gesture.is_some() && on_edit(&command) && !releases && !goes_on {
            self.step(None);
        }
        let completes = completes(&command);
        let applied = self.edit_command(command);
        match (&applied, completes) {
            (Ok(_), Some(note)) => self.step(note),
            (Err(_), _) if releases => self.step(None),
            _ => {}
        }
        applied
    }

    /// Makes what changed of the open photo's edit since its last step one step, source
    /// You, with `note`; nothing on a photo open for viewing only.
    pub(super) fn step(&mut self, note: Option<Note>) {
        if let Some(open) = &mut self.open
            && open.view_only.is_none()
        {
            open.history.complete(&open.edit, StepSource::You, note);
        }
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
        // Nothing to move, though a Redo may have made a change step first, which saves as
        // a completed change does.
        let Some(edit) = moved else {
            return Ok(Effect {
                save: true,
                ..Effect::default()
            });
        };
        let changed = edit != open.edit;
        open.edit = edit;
        open.held.prune(&open.edit);
        open.unsaved |= changed;
        Ok(Effect {
            render: changed,
            save: true,
            cancelled: false,
        })
    }

    /// The open photo's `History`, where it changed since it was last sent.
    pub(super) fn history_event(&mut self) -> Option<Event> {
        let open = self.open.as_mut().filter(|open| open.history.changed())?;
        let fact = (open.history).fact(self.applied, &self.modules, self.operations, &open.image);
        Some(Event::History(fact))
    }
}

#[cfg(test)]
mod tests;
