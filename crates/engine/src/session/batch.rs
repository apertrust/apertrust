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

//! A style on several photos (decision 109, `docs/history.md`, A batch on several photos):
//! one [`Command::ApplyStyle`] naming them, one batch id on every step of every photo it
//! changes. The active photo changes at once in the command's turn; the others by one
//! `apply-style` job, which reads each one's metadata and sidecar on its worker, applies the
//! style to its log by [`style_log`], the function the active photo's change goes through,
//! and hands the engine thread the file's XML, which alone knows which photo is open.
//!
//! A rating on several photos (decision 110), one [`Command::Rate`] naming them, is a batch
//! the same way, by a `rate` job reading no raw: each photo's fields set with a clock of the
//! batch's, darktable's copied in first where it has no sidecar, its log left as it is. It
//! is no step, so each photo's [`FieldRecord`] is kept with the batch, which takes it back,
//! but where the field changed since. Every photo is routed alike:
//!
//! - **open and live,** the result dropped and the style applied there as to the active
//!   photo ([`Engine::reach`]);
//! - **opening,** the job waiting, then reading the file again;
//! - **any other,** written over the file as read ([`super::Writer::write_over`]), read
//!   again where it changed since, twice more, then named as changing.
//!
//! A command on the edit while an earlier batch still has the active photo to come has that
//! batch applied to it first ([`Engine::catch_up`]), so each photo's batches land in the
//! order sent. One batch job runs at a time, the next waiting for the one before.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant};

use apertrust_library::clock::wall_clock;
use apertrust_library::fields::FieldEntry;
use apertrust_library::listing;
use apertrust_plugin::Operation;
use apertrust_tools::{ProcessingTool, ToolValues};

use super::fields::{self, FieldRecord};
use super::history::PhotoHistory;
use super::styles::{Batch, Style, style_log};
use super::{
    Command, Effect, Engine, Event, Inbox, Load, Open, Outbox, Over, Rating, RatingChange, Reads,
    SessionError, StyleMode, library,
};
use crate::history::{BatchId, Clock, Log};
use crate::jobs::{self, Context, Failure, Interrupted, Job, Message};
use crate::sidecar::{self, Edits};

/// What a batch on several photos does to each, as its `Ended` names it: a style applied,
/// by its name (decision 109), or a rating set (decision 110).
#[derive(Debug, Clone, PartialEq)]
pub enum Batched {
    Style(String),
    Rating(RatingChange),
}

/// What a batch on several photos tells (decision 109): notices, apart from the session's
/// batches of commands, naming the batch by its id.
#[derive(Debug, Clone, PartialEq)]
pub enum BatchEvent {
    /// A photo's change landed: the active photo's in the command's turn, another's as its
    /// write lands. `edit` is its edit's state after it, so a client marks the Browser's
    /// pencil and asks for its thumbnail again.
    Changed {
        batch: BatchId,
        photo: PathBuf,
        edit: library::EditState,
    },
    /// The batch ended, in the turn of the command numbered `command` where there was no
    /// job: how many photos it named, those it changed, those it skipped with why, and
    /// whether it was cancelled, which left the rest.
    Ended {
        command: u64,
        batch: BatchId,
        action: BatchAction,
        what: Batched,
        named: usize,
        changed: Vec<PathBuf>,
        skipped: Vec<(PathBuf, Skip)>,
        cancelled: bool,
    },
}

/// What a batch did to its photos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchAction {
    /// The style applied.
    Applied,
    /// The batch taken back, by Undo.
    TakenBack,
    /// The batch put back, by Redo.
    PutBack,
}

/// Why a batch left a photo as it was, which a client names (decision 109).
#[derive(Debug, Clone, PartialEq)]
pub enum Skip {
    /// Its raw is not on the disk, a cloud placeholder: no batch downloads it.
    NotOnDisk,
    /// Its raw, or its sidecar, cannot be read, or the raw is no 16-bit mosaic.
    Unreadable(String),
    /// Its edit opens for viewing only: a newer Apertrust's, or one part of whose history
    /// cannot be read.
    ViewOnly(sidecar::ViewOnly),
    /// It refuses a value of the style, as an option it does not list.
    Refused(String),
    /// Its file kept changing between its read and its write.
    KeptChanging,
    /// Its write failed.
    Failed(String),
    /// Its history moved since the batch: its end is no longer on the batch's last step, for
    /// Undo, or its next step is not the batch's first, for Redo; the batch stays under its
    /// own steps (decision 109).
    ChangedSince,
}

/// A style as a batch keeps it while its job runs: what [`Command::ApplyStyle`] named.
#[derive(Debug, Clone)]
pub(super) struct Owned {
    pub(super) name: String,
    pub(super) file: String,
    pub(super) mode: StyleMode,
    pub(super) tools: Vec<(String, ToolValues)>,
}

impl Owned {
    fn style(&self) -> Style<'_> {
        Style {
            name: &self.name,
            file: &self.file,
            mode: self.mode,
            tools: &self.tools,
        }
    }

    /// The command that applies it to the open photo alone, as the batch's.
    fn command(&self) -> Command {
        Command::ApplyStyle {
            name: self.name.clone(),
            file: self.file.clone(),
            mode: self.mode,
            tools: self.tools.clone(),
            photos: Vec::new(),
        }
    }

    /// The tools it may change, `None` for every tool: a tool's picker on turns off as its
    /// tool changes.
    pub(super) fn changes(mode: StyleMode, tools: &[(String, ToolValues)]) -> Vec<Option<String>> {
        match mode {
            StyleMode::Replace => vec![None],
            StyleMode::Append => (tools.iter()).map(|(tool, _)| Some(tool.clone())).collect(),
        }
    }
}

/// What a batch does to each of its photos: a style applied, or a rating set.
#[derive(Debug, Clone)]
pub(super) enum Change {
    Style(Owned),
    Rate(RatingChange),
}

impl Change {
    fn batched(&self) -> Batched {
        match self {
            Change::Style(style) => Batched::Style(style.name.clone()),
            Change::Rate(change) => Batched::Rating(*change),
        }
    }
}

/// A batch on several photos while it lasts: its id and clock, what it does, the photos it
/// has still to reach, and what it did to those it reached.
pub(super) struct Pending {
    pub(super) batch: Batch,
    pub(super) change: Change,
    /// What it does to its photos: applies the style, or takes it back or puts it back.
    action: BatchAction,
    /// The number of the command that started it.
    command: u64,
    /// How many photos it names.
    count: usize,
    state: Mutex<Reached>,
}

/// What a batch has reached of its photos.
#[derive(Default)]
struct Reached {
    /// Not yet reached, in the order named.
    to_come: Vec<PathBuf>,
    changed: Vec<PathBuf>,
    skipped: Vec<(PathBuf, Skip)>,
    /// Its job, once the engine thread started it, which a take-back cancels while it runs:
    /// one batch job runs at a time.
    job: Option<jobs::JobId>,
    /// For the batch applying a style, its latest take-back or put-back, whose photos the
    /// next one moves.
    latest: Option<Arc<Pending>>,
    /// For a take-back or a put-back, the move before it, whose photos changed it moves,
    /// read again as it starts, after that one's job; let go as it starts or ends, so it
    /// holds no cycle with the batch's `latest`.
    from: Option<Arc<Pending>>,
    /// Set as its job ends, cancelled or not, or as it ends with no job: the photos not
    /// reached are left.
    ended: bool,
    /// Set as its job is cancelled: no photo is reached from then on but those whose write
    /// was handed over already.
    cancelled: bool,
    /// For a rating's batch, each photo it set fields on, with the record that takes them
    /// back, or puts them back, and the photo's rating it left.
    rated: Vec<(PathBuf, FieldRecord, Rating)>,
}

/// How a photo of a batch was reached.
enum Outcome {
    Changed,
    /// Left as it was: it held the batch already, or the style changes nothing.
    Unchanged,
    Skipped(Skip),
}

impl Pending {
    fn new(
        (batch, change, action): (Batch, Change, BatchAction),
        command: u64,
        photos: Vec<PathBuf>,
    ) -> Pending {
        let mut to_come: Vec<PathBuf> = Vec::new();
        for photo in photos {
            if !to_come.contains(&photo) {
                to_come.push(photo);
            }
        }
        Pending {
            batch,
            change,
            action,
            command,
            count: to_come.len(),
            state: Mutex::new(Reached {
                to_come,
                ..Reached::default()
            }),
        }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, Reached> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Whether it has `sidecar`'s photo still to reach, its job neither ended nor cancelled.
    fn to_come(&self, sidecar: &Path) -> Option<PathBuf> {
        let state = self.state();
        if state.ended || state.cancelled {
            return None;
        }
        (state.to_come.iter())
            .find(|photo| sidecar::path_for(photo) == sidecar)
            .cloned()
    }

    /// Whether it has ended.
    fn has_ended(&self) -> bool {
        self.state().ended
    }

    /// Its job is cancelled: it reaches no more photos, live or caught up.
    fn cancel(&self) {
        self.state().cancelled = true;
    }

    /// Its job, once started.
    #[cfg(test)]
    pub(super) fn job(&self) -> Option<jobs::JobId> {
        self.state().job
    }

    /// A take-back's or a put-back's photos, those its batch changed, as it starts, that
    /// batch's job having ended.
    fn take_from(&self) {
        let Some(from) = self.state().from.take() else {
            return;
        };
        let (changed, rated) = {
            let from = from.state();
            (from.changed.clone(), from.rated.clone())
        };
        let mut state = self.state();
        for kept in rated {
            if !state.rated.iter().any(|(photo, ..)| *photo == kept.0) {
                state.rated.push(kept);
            }
        }
        for photo in changed {
            let reached = state.changed.contains(&photo)
                || state.skipped.iter().any(|(skipped, _)| *skipped == photo);
            if !reached && !state.to_come.contains(&photo) {
                state.to_come.push(photo);
            }
        }
    }

    /// Keeps `record`, a rating's on `photo`, and the rating it left, `rating`, for the
    /// batch's moves on it, in place of one kept before.
    fn keep_rated(&self, photo: &Path, record: FieldRecord, rating: Rating) {
        let mut state = self.state();
        state.rated.retain(|(kept, ..)| kept != photo);
        state.rated.push((photo.to_owned(), record, rating));
    }

    /// The record this rating's batch keeps for `photo` put on its fields, `entries`: each
    /// field as it was before the batch, or with `redo` as the batch left it, at `clock`,
    /// where it still holds the value the batch left (decision 110). The record and the
    /// rating as it left them, which the caller keeps once they are the photo's; none where
    /// the batch keeps no record of the photo or every field changed since.
    fn put_rated(
        &self,
        photo: &Path,
        entries: &mut Vec<FieldEntry>,
        redo: bool,
        clock: Clock,
    ) -> Option<(FieldRecord, Rating)> {
        let mut record = self.rated(photo)?;
        (record.put(entries, !redo, clock)).then(|| (record, Rating::of(entries)))
    }

    /// The record a rating's batch keeps for `photo`, if any.
    fn rated(&self, photo: &Path) -> Option<FieldRecord> {
        let state = self.state();
        (state.rated.iter()).find_map(|(kept, record, _)| (kept == photo).then(|| record.clone()))
    }

    /// The rating a rating's batch left on `photo`, if any.
    fn rating_of(&self, photo: &Path) -> Option<Rating> {
        let state = self.state();
        (state.rated.iter()).find_map(|(kept, _, rating)| (kept == photo).then_some(*rating))
    }

    /// Records `photo` reached, `outcome` how, once: whether it was still to come.
    fn reach(&self, photo: &Path, outcome: Outcome) -> bool {
        let mut state = self.state();
        let Some(at) = state.to_come.iter().position(|kept| kept == photo) else {
            return false;
        };
        state.to_come.remove(at);
        match outcome {
            Outcome::Changed => state.changed.push(photo.to_owned()),
            Outcome::Unchanged => {}
            Outcome::Skipped(skip) => state.skipped.push((photo.to_owned(), skip)),
        }
        true
    }

    /// It ends, cancelled or not, the photos not reached left: its `Ended`, once, `None`
    /// where it has ended already.
    fn end(&self, cancelled: bool) -> Option<Event> {
        let mut state = self.state();
        if state.ended {
            return None;
        }
        state.ended = true;
        state.from = None;
        Some(Event::Batch(BatchEvent::Ended {
            command: self.command,
            batch: self.batch.id,
            action: self.action,
            what: self.change.batched(),
            named: self.count,
            changed: state.changed.clone(),
            skipped: state.skipped.clone(),
            cancelled,
        }))
    }
}

/// What a job's worker hands the engine thread for a photo of its batch: the file's XML it
/// built from the file it read, and that file's stamp.
pub(crate) struct Route {
    pending: Arc<Pending>,
    photo: PathBuf,
    xml: String,
    read: Option<sidecar::FileStamp>,
    answer: mpsc::Sender<Routed>,
}

#[cfg(test)]
impl Route {
    /// The photo it hands over.
    pub(super) fn photo(&self) -> &Path {
        &self.photo
    }
}

/// How the engine thread routed a photo of a batch.
enum Routed {
    /// It is open and live: the batch applied there, or not, and recorded.
    Live,
    /// It is opening: the job reads it again once it is live or its open has ended.
    Opening,
    /// Handed to the edit writer, over the file as read.
    Handed(mpsc::Receiver<Over>),
}

/// How many batches on several photos the session remembers for their Undo across photos,
/// the latest (decision 109).
const REMEMBERED: usize = 64;

/// How long a job waits before it looks again at a photo that is opening.
const OPENING_WAIT: Duration = Duration::from_millis(20);

/// How many times a photo's file is read before it is named as changing: once, and twice
/// more after a write found it changed (decision 109).
const TRIES: usize = 3;

impl Engine {
    /// [`Command::ApplyStyle`] or [`Command::Rate`] naming `photos`, `change` what it does,
    /// the command numbered [`Self::applied`]: one batch drawn, its clock the wall clock now;
    /// the active photo, where named, changed at once, a refusal skipping it rather than the
    /// command; the others by one `apply-style` or `rate` job; the batch's `Ended` now where
    /// there is no job. Refused whole while the active photo opens, as any command on the
    /// edit; a gesture under way ends first.
    pub(super) fn batch_to(
        &mut self,
        change: Change,
        photos: Vec<PathBuf>,
        events: &mut Vec<Event>,
    ) -> Result<Effect, SessionError> {
        if self.opening.is_some() {
            return Err(SessionError::NotLive);
        }
        let id = (self.open.as_ref()).map_or_else(
            || Log::default().new_batch(),
            |open| open.history.new_batch(),
        );
        let batch = Batch {
            id,
            wall: wall_clock(),
        };
        let what = (batch, change, BatchAction::Applied);
        let pending = Arc::new(Pending::new(what, self.applied, photos));
        // Remembered for its Undo across photos, the latest kept.
        if self.remembered.len() == REMEMBERED {
            self.remembered.pop_front();
        }
        self.remembered.push_back(Arc::clone(&pending));
        let active = self.open.as_ref().map(|open| open.sidecar.clone());
        let mut effect = Effect::default();
        match active.as_deref().and_then(|active| pending.to_come(active)) {
            Some(photo) => effect = self.reach(&pending, &photo, events),
            None => self.end_gesture(),
        }
        if pending.state().to_come.is_empty() {
            events.extend(pending.end(false));
        } else {
            self.batches.push(pending);
            events.extend(self.start_batches());
        }
        Ok(effect)
    }

    /// Every earlier batch that still has the active photo to come applied to it, in the
    /// order sent, before a command on its edit (decision 109); what they did to it.
    pub(super) fn catch_up(&mut self, events: &mut Vec<Event>) -> Effect {
        let mut effect = Effect::default();
        if self.opening.is_some() {
            return effect;
        }
        let Some(active) = self.open.as_ref().map(|open| open.sidecar.clone()) else {
            return effect;
        };
        for pending in self.batches.clone() {
            if let Some(photo) = pending.to_come(&active) {
                let reached = self.reach_live(&pending, &photo, events);
                effect.render |= reached.render;
                effect.save |= reached.save;
            }
        }
        effect
    }

    /// The batch `pending` reached on the open photo, `photo`, as its action says: applied,
    /// or taken back or put back.
    fn reach_live(
        &mut self,
        pending: &Arc<Pending>,
        photo: &Path,
        events: &mut Vec<Event>,
    ) -> Effect {
        match pending.action {
            BatchAction::Applied => self.reach(pending, photo, events),
            BatchAction::TakenBack => self.move_live(pending, photo, false, events),
            BatchAction::PutBack => self.move_live(pending, photo, true, events),
        }
    }

    /// The batch `pending` applied to the open photo, `photo`, as the active photo's change
    /// is, and recorded.
    fn reach(&mut self, pending: &Arc<Pending>, photo: &Path, events: &mut Vec<Event>) -> Effect {
        match &pending.change {
            Change::Style(style) => self.reach_style(pending, style, photo, events),
            Change::Rate(change) => self.reach_rating(pending, *change, photo, events),
        }
    }

    /// The style `style` of the batch `pending` applied to the open photo, `photo`: left as
    /// it is where its log holds the batch already, skipped and named where it is open for
    /// viewing only or refuses a value of the style.
    fn reach_style(
        &mut self,
        pending: &Arc<Pending>,
        style: &Owned,
        photo: &Path,
        events: &mut Vec<Event>,
    ) -> Effect {
        let holds = (self.open.as_ref()).is_some_and(|open| {
            let steps = open.history.log().steps();
            steps
                .iter()
                .any(|step| step.batch == Some(pending.batch.id))
        });
        // Only this batch writes its id: it changed the photo already.
        if holds {
            if pending.reach(photo, Outcome::Changed) {
                events.push(Event::Batch(BatchEvent::Changed {
                    batch: pending.batch.id,
                    photo: photo.to_owned(),
                    edit: library::EditState::Edited,
                }));
            }
            return Effect::default();
        }
        self.batching = Some(pending.batch);
        let applied = self.apply_edit(style.command(), Instant::now());
        self.batching = None;
        let (outcome, effect) = match applied {
            Ok(effect) => {
                let changed = (self.open.as_ref()).is_some_and(|open| {
                    let steps = open.history.log().steps();
                    steps
                        .iter()
                        .any(|step| step.batch == Some(pending.batch.id))
                });
                match changed {
                    true => (Outcome::Changed, effect),
                    false => (Outcome::Unchanged, effect),
                }
            }
            Err(SessionError::ViewOnly(view_only)) => (
                Outcome::Skipped(Skip::ViewOnly(view_only)),
                Effect::default(),
            ),
            Err(error) => {
                let refused = Skip::Refused(error.to_string());
                (Outcome::Skipped(refused), Effect::default())
            }
        };
        if matches!(outcome, Outcome::Changed) {
            events.push(Event::Batch(BatchEvent::Changed {
                batch: pending.batch.id,
                photo: photo.to_owned(),
                edit: library::EditState::Edited,
            }));
            for tool in Owned::changes(style.mode, &style.tools) {
                self.picker_tool_changed(tool.as_deref(), events);
            }
        }
        pending.reach(photo, outcome);
        effect
    }

    /// The rating `change` of the batch `pending` set on the open photo, `photo`, as
    /// [`Command::Rate`] sets it, its record in the batch, which keeps a copy for the
    /// photo's moves once it has closed: left as it is where it holds the batch's record
    /// already, skipped and named where it is open for viewing only.
    fn reach_rating(
        &mut self,
        pending: &Arc<Pending>,
        change: RatingChange,
        photo: &Path,
        events: &mut Vec<Event>,
    ) -> Effect {
        let id = pending.batch.id;
        let holds =
            (self.open.as_ref()).is_some_and(|open| open.history.field_record(id).is_some());
        if !holds {
            self.batching = Some(pending.batch);
            let rated = self.rate(change, Instant::now());
            self.batching = None;
            let skipped = match rated {
                Ok(true) => None,
                Ok(false) => Some(Outcome::Unchanged),
                Err(SessionError::ViewOnly(view_only)) => {
                    Some(Outcome::Skipped(Skip::ViewOnly(view_only)))
                }
                Err(error) => Some(Outcome::Skipped(Skip::Refused(error.to_string()))),
            };
            if let Some(outcome) = skipped {
                pending.reach(photo, outcome);
                return Effect::default();
            }
        }
        if !self.keep_open_record(pending, photo) {
            return Effect::default();
        }
        if pending.reach(photo, Outcome::Changed) {
            events.push(self.moved_event(pending, photo));
        }
        Effect {
            save: true,
            ..Effect::default()
        }
    }

    /// Keeps the open photo's record of `pending`'s batch, as its reach or its own Undo or
    /// Redo left it, and its rating, for the batch's moves on it, `photo`, once it has
    /// closed: whether it holds one.
    fn keep_open_record(&self, pending: &Pending, photo: &Path) -> bool {
        let open = self.open.as_ref();
        let held = open.and_then(|open| {
            let record = open.history.field_record(pending.batch.id)?;
            Some((record, open.rating()))
        });
        let Some((record, rating)) = held else {
            return false;
        };
        pending.keep_rated(photo, record, rating);
        true
    }

    /// The job `job` cancelled: the batch it runs, if any, reaches no more photos.
    pub(super) fn batch_cancelled(&mut self, job: jobs::JobId) {
        for pending in &self.batches {
            if pending.state().job == Some(job) {
                pending.cancel();
            }
        }
    }

    /// The batches waiting, started one at a time in the order sent, by the engine thread so
    /// none holds a job's worker while it waits (decision 109): the ended ones let go, and
    /// where none runs the first waiting started, a take-back's photos read as it starts, or
    /// ended at once where none is left to reach. Its `Ended` where it ends so, and the
    /// history told again where a batch ended, its Undo naming how many photos it changed.
    pub(super) fn start_batches(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        // The batches that ended, by id, as they are let go.
        let mut ended: Vec<BatchId> = Vec::new();
        self.batches.retain(|pending| {
            let gone = pending.has_ended();
            if gone {
                ended.push(pending.batch.id);
            }
            !gone
        });
        while let Some(first) = self.batches.first().cloned() {
            let job = first.state().job;
            if let Some(job) = job {
                // Gone without its end, cancelled while queued or its body panicking: ended
                // here, so the batches after it start.
                if self.jobs.control.state(job).is_none() {
                    events.extend(first.end(true));
                    ended.push(first.batch.id);
                    self.batches.remove(0);
                    continue;
                }
                break;
            }
            first.take_from();
            if first.state().to_come.is_empty() {
                events.extend(first.end(false));
                ended.push(first.batch.id);
                self.batches.remove(0);
                continue;
            }
            self.batch_job(first);
            break;
        }
        // Undo or Redo over one that ended names another count of photos.
        let moving = |open: &Open| {
            let over = [
                open.history.batch_moving(false),
                open.history.batch_moving(true),
            ];
            over.into_iter()
                .flatten()
                .any(|batch| ended.contains(&batch))
        };
        if let Some(open) = &mut self.open
            && moving(open)
        {
            open.history.tell_again();
            events.extend(self.history_event());
        }
        events
    }

    /// The batch's job, `apply-style` or `undo-batch` on `UserForeground`, cancellable and
    /// quiet.
    fn batch_job(&mut self, pending: Arc<Pending>) {
        let run = Run {
            pending: Arc::clone(&pending),
            load: Arc::clone(&self.load),
            operations: self.operations,
            tools: self.tools.clone(),
            retired: self.retired,
            defaults: crate::history::default_tools(&self.tools, self.operations),
            build: self.build.clone(),
            reads: self.writer.reads(),
            inbox: self.jobs.inbox.clone(),
            outbox: self.jobs.outbox.clone(),
        };
        let job = (self.jobs).add(run.pending.command, jobs::Queue::UserForeground, |_| {
            job(run)
        });
        pending.state().job = Some(job);
    }

    /// How many photos Undo, or Redo where `redo`, would move the remembered batch `batch`
    /// on, for the `History` fact's `Move::Steps::photos`: those its latest move left it on,
    /// a put-back's for Undo or a take-back's for Redo, else those applying it changed;
    /// while that runs, those it changed and those still to come, once ended those it
    /// changed. `None` for one the session does not remember.
    pub(super) fn batch_photos(&self, batch: BatchId, redo: bool) -> Option<usize> {
        let pending = (self.remembered.iter()).find(|pending| pending.batch.id == batch)?;
        let wanted = match redo {
            true => BatchAction::TakenBack,
            false => BatchAction::PutBack,
        };
        let latest = (pending.state().latest.clone()).filter(|latest| latest.action == wanted);
        let source = latest.unwrap_or_else(|| Arc::clone(pending));
        let state = source.state();
        Some(match state.ended {
            true => state.changed.len(),
            false => state.changed.len() + state.to_come.len(),
        })
    }

    /// After the open photo's Undo, or Redo where `redo`, moved over the batch `batch`: one
    /// the session remembers moved on its other photos too (decision 109).
    pub(super) fn batch_crossed(&mut self, batch: BatchId, redo: bool, events: &mut Vec<Event>) {
        let Some(pending) = (self.remembered.iter())
            .find(|pending| pending.batch.id == batch)
            .cloned()
        else {
            return;
        };
        let active = self.open.as_ref().map(|open| open.sidecar.clone());
        let active = active.as_deref().map(|active| (active, true));
        self.move_others(&pending, redo, active, events);
    }

    /// [`Command::UndoBatch`]: the remembered batch `batch` taken back on each of its
    /// photos still on it, the active one by its own Undo where that would take it back and
    /// it is not open for viewing only, the others by an `undo-batch` job; one the session
    /// does not remember refused.
    pub(super) fn undo_batch(
        &mut self,
        batch: BatchId,
        events: &mut Vec<Event>,
    ) -> Result<Effect, SessionError> {
        let Some(pending) = (self.remembered.iter())
            .find(|pending| pending.batch.id == batch)
            .cloned()
        else {
            return Err(SessionError::UnknownBatch { batch });
        };
        let active = self.open.as_ref().map(|open| open.sidecar.clone());
        let on_active = (active.as_deref()).is_some_and(|active| {
            let state = pending.state();
            (state.changed.iter()).any(|photo| sidecar::path_for(photo) == active)
        });
        if on_active {
            self.step_pending();
        }
        // A view-only photo refuses its Undo: skipped and named, as the job names others.
        let moves = on_active
            && (self.open.as_ref())
                .is_some_and(|o| o.view_only.is_none() && o.history.moves_batch(batch, false));
        let effect = match moves {
            true => self.apply_edit(Command::Undo, Instant::now())?,
            false => Effect::default(),
        };
        let done = (active.as_deref())
            .filter(|_| on_active)
            .map(|active| (active, moves));
        self.move_others(&pending, false, done, events);
        Ok(effect)
    }

    /// The remembered batch `pending` taken back, or put back where `redo`, on its photos
    /// but `active`, the active photo's sidecar where it is among them and whether its own
    /// move moved over the batch, one that did not skipped as open for viewing only or as
    /// changed since: its `apply-style` job cancelled first where it still runs, then an
    /// `undo-batch` job, after it, over the photos it changed, the batch's `Ended` naming
    /// the active photo among them. With no other photo, the `Ended` comes now.
    pub(super) fn move_others(
        &mut self,
        pending: &Arc<Pending>,
        redo: bool,
        active: Option<(&Path, bool)>,
        events: &mut Vec<Event>,
    ) {
        // The batch's latest move, its applying or a take-back or put-back since, whose
        // changed photos this one moves: a photo an Undo skipped is none Redo puts back.
        let source = (pending.state().latest.clone()).unwrap_or_else(|| Arc::clone(pending));
        let (job, ended) = {
            let state = source.state();
            (state.job, state.ended)
        };
        match job {
            Some(job) if !ended => {
                self.jobs.control.cancel(job);
                source.cancel();
            }
            // Still waiting: it never starts.
            None if !ended => events.extend(source.end(true)),
            _ => {}
        }
        let action = match redo {
            true => BatchAction::PutBack,
            false => BatchAction::TakenBack,
        };
        // The photos it reached so far; a running job's are read again as this one's job
        // starts, after it.
        let photos = source.state().changed.clone();
        let what = (pending.batch, pending.change.clone(), action);
        let mut moving = Pending::new(what, self.applied, photos);
        moving.count = pending.count;
        // A rating's records, which it moves, those a running job keeps adding read again
        // as it starts.
        moving.state().rated = source.state().rated.clone();
        moving.state().from = Some(source);
        let moving = Arc::new(moving);
        pending.state().latest = Some(Arc::clone(&moving));
        // A rating's photo its own move did not move, opened since in a session of its own
        // or changed since in its edit: left to the job, whose route reaches it live from the
        // record the batch kept.
        let rating = matches!(pending.change, Change::Rate(_));
        if let Some((active, moved)) = active
            && let Some(photo) = moving.to_come(active)
        {
            let view_only = self.open.as_ref().and_then(|open| open.view_only.clone());
            let outcome = match (moved, view_only) {
                (true, _) => Some(Outcome::Changed),
                (false, Some(view_only)) => Some(Outcome::Skipped(Skip::ViewOnly(view_only))),
                (false, None) if rating => None,
                (false, None) => Some(Outcome::Skipped(Skip::ChangedSince)),
            };
            if let Some(outcome) = outcome {
                moving.reach(&photo, outcome);
            }
            if moved {
                // A rating's record as the photo's own Undo or Redo left it.
                self.keep_open_record(&moving, &photo);
                events.push(self.moved_event(&moving, &photo));
            }
        }
        // Started by the engine thread once the batches before it end, its batch's job among
        // them.
        self.batches.push(moving);
        events.extend(self.start_batches());
    }

    /// The `Changed` of the open photo, `photo`, a move of the batch `pending` reached: its
    /// edit's state as the move left it.
    fn moved_event(&self, pending: &Pending, photo: &Path) -> Event {
        let edited = (self.open.as_ref()).is_some_and(|open| !open.edit.is_empty());
        Event::Batch(BatchEvent::Changed {
            batch: pending.batch.id,
            photo: photo.to_owned(),
            edit: match edited {
                true => library::EditState::Edited,
                false => library::EditState::None,
            },
        })
    }

    /// The batch `pending` taken back, or put back where `redo`, on the open photo, `photo`,
    /// by its own Undo or Redo where that would move over it, and recorded; skipped and
    /// named as changed since otherwise.
    fn move_live(
        &mut self,
        pending: &Arc<Pending>,
        photo: &Path,
        redo: bool,
        events: &mut Vec<Event>,
    ) -> Effect {
        let batch = pending.batch.id;
        // A change not yet a step, a gesture's among them, is a later change: made one first.
        self.step_pending();
        let moves = (self.open.as_ref()).is_some_and(|open| open.history.moves_batch(batch, redo));
        if !moves && matches!(pending.change, Change::Rate(_)) {
            return self.put_rated_live(pending, photo, redo, events);
        }
        if !moves {
            pending.reach(photo, Outcome::Skipped(Skip::ChangedSince));
            return Effect::default();
        }
        let command = match redo {
            true => Command::Redo,
            false => Command::Undo,
        };
        match self.apply_edit(command, Instant::now()) {
            Ok(effect) => {
                // A rating's record as the move left it, for the batch's next move.
                self.keep_open_record(pending, photo);
                pending.reach(photo, Outcome::Changed);
                events.push(self.moved_event(pending, photo));
                effect
            }
            Err(error) => {
                pending.reach(photo, Outcome::Skipped(Skip::Refused(error.to_string())));
                Effect::default()
            }
        }
    }

    /// A rating's batch `pending` taken back, or put back where `redo`, on the open photo,
    /// `photo`, whose own Undo or Redo would not move it, opened since in a session of its
    /// own or changed since in its edit: from the record the batch kept, as its job does for
    /// a photo not open ([`Pending::put_rated`]); skipped and named as changed since, or as
    /// open for viewing only.
    fn put_rated_live(
        &mut self,
        pending: &Arc<Pending>,
        photo: &Path,
        redo: bool,
        events: &mut Vec<Event>,
    ) -> Effect {
        let Some(open) = &mut self.open else {
            return Effect::default();
        };
        if let Some(view_only) = &open.view_only {
            pending.reach(photo, Outcome::Skipped(Skip::ViewOnly(view_only.clone())));
            return Effect::default();
        }
        let clock = open.history.stamp();
        let entries = fields::first_mut(&mut open.file);
        let Some((record, rating)) = pending.put_rated(photo, entries, redo, clock) else {
            pending.reach(photo, Outcome::Skipped(Skip::ChangedSince));
            return Effect::default();
        };
        pending.keep_rated(photo, record, rating);
        open.fields_unsaved = true;
        open.unsaved = true;
        pending.reach(photo, Outcome::Changed);
        events.push(self.moved_event(pending, photo));
        Effect {
            save: true,
            ..Effect::default()
        }
    }

    /// The batches' photos the job workers handed the engine thread since, each routed:
    /// open and live, the batch applied there; opening, told so; any other, handed to the
    /// writer over the file as read. What the live photo's change says.
    pub(super) fn take_routes(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        let (mut changed, mut render) = (false, false);
        // The open photo's rating before, which a rating's batch reaching it live changes.
        let rating = self.open_rating();
        for route in std::mem::take(&mut self.routes) {
            let sidecar = sidecar::path_for(&route.photo);
            let opening =
                (self.opening.as_ref()).is_some_and(|o| sidecar::path_for(&o.path) == sidecar);
            let live = (self.open.as_ref()).is_some_and(|open| open.sidecar == sidecar);
            let routed = if opening {
                Routed::Opening
            } else if route.pending.to_come(&sidecar).is_none() {
                // Reached already, live, as a command caught it up: left as it is.
                Routed::Live
            } else if live {
                let effect = self.reach_live(&route.pending, &route.photo, &mut events);
                changed |= effect.render || effect.save;
                render |= effect.render;
                Routed::Live
            } else {
                Routed::Handed(self.writer.write_over(sidecar, route.xml, route.read))
            };
            // A job that has gone hears nothing.
            let _ = route.answer.send(routed);
        }
        // A rating's route saves and tells; only a style's renders.
        if changed {
            self.edited_events(&mut events, render, false);
        }
        events.extend(self.rating_events(rating));
        events
    }
}

/// What a batch job runs with, owned, so its body borrows nothing of the engine thread's.
struct Run {
    pending: Arc<Pending>,
    load: Load,
    operations: &'static [&'static dyn Operation],
    tools: Vec<&'static dyn ProcessingTool>,
    retired: &'static [&'static str],
    defaults: Vec<&'static str>,
    build: String,
    reads: Reads,
    inbox: mpsc::Sender<Inbox>,
    outbox: Outbox,
}

/// The job of a batch on its photos but the active one: `apply-style`, applying it, or
/// `undo-batch`, taking it back or putting it back.
fn job(run: Run) -> Job {
    let pending = &run.pending;
    let total = pending.state().to_come.len() as u64;
    let (kind, message, _) = words(pending);
    let title = named(Message::new(message), pending).with("count", pending.count as u64);
    let description = match &pending.change {
        Change::Style(style) => format!("{kind} {}", style.name),
        Change::Rate(change) => format!("{kind} {change:?}"),
    };
    Job::new(kind, description, move |context| body(&run, context))
        .progress(title, true)
        .total(total)
        .quiet()
}

/// A batch job's kind and the messages of its title and its outcome, by what its batch
/// does and how: `apply-style` or `rate`, or `undo-batch` taking either back or putting it
/// back.
fn words(pending: &Pending) -> (&'static str, &'static str, &'static str) {
    match (&pending.change, pending.action) {
        (Change::Style(_), BatchAction::Applied) => {
            ("apply-style", "job-apply-style", "job-apply-style-outcome")
        }
        (Change::Style(_), BatchAction::TakenBack) => {
            ("undo-batch", "job-undo-batch", "job-undo-batch-outcome")
        }
        (Change::Style(_), BatchAction::PutBack) => {
            ("undo-batch", "job-redo-batch", "job-redo-batch-outcome")
        }
        (Change::Rate(_), BatchAction::Applied) => ("rate", "job-rate", "job-rate-outcome"),
        (Change::Rate(_), BatchAction::TakenBack) => {
            ("undo-batch", "job-undo-rate", "job-undo-rate-outcome")
        }
        (Change::Rate(_), BatchAction::PutBack) => {
            ("undo-batch", "job-redo-rate", "job-redo-rate-outcome")
        }
    }
}

/// `message` naming the style a batch applies; a rating's words name none.
fn named(message: Message, pending: &Pending) -> Message {
    match &pending.change {
        Change::Style(style) => message.with("style", style.name.as_str()),
        Change::Rate(_) => message,
    }
}

/// The job's body: each photo still to come reached in order until it is cancelled, then
/// the batch's `Ended`, and the engine thread woken to start the next; it ends failed with
/// the first failed write.
fn body(run: &Run, context: &Context<'_>) -> Result<(), Failure> {
    let pending = &run.pending;
    let photos = pending.state().to_come.clone();
    let total = photos.len() as u64;
    let mut failure = None;
    for (done, photo) in photos.iter().enumerate() {
        context.set_progress(done as u64, Some(total));
        if context.cancelled() {
            break;
        }
        // The engine thread reached it first, live.
        if pending.to_come(&sidecar::path_for(photo)).is_none() {
            continue;
        }
        let name = sidecar::file_name(photo).into_owned();
        context.set_detail(Some(
            Message::new("job-apply-style-now")
                .with("done", done as u64)
                .with("total", total)
                .with("photo", name.as_str()),
        ));
        let reached = match pending.action {
            BatchAction::Applied => reach(run, context, photo),
            BatchAction::TakenBack => moved(run, context, photo, false),
            BatchAction::PutBack => moved(run, context, photo, true),
        };
        let outcome = match reached {
            Ok(Some(outcome)) => outcome,
            // Live, the engine thread recorded it.
            Ok(None) => continue,
            Err(Interrupted::Cancelled) => break,
            Err(Interrupted::Disconnected) => {
                Outcome::Skipped(Skip::Failed("the engine stopped".to_owned()))
            }
        };
        if let Outcome::Skipped(Skip::Failed(reason)) = &outcome {
            failure.get_or_insert_with(|| Failure::new("write", reason.clone()).item(photo));
        }
        let changed = matches!(outcome, Outcome::Changed);
        if pending.reach(photo, outcome) && changed {
            run.outbox.send(Event::Batch(BatchEvent::Changed {
                batch: pending.batch.id,
                photo: photo.clone(),
                edit: library::sidecar_state(&sidecar::path_for(photo)),
            }));
            // A rating's photo, whose cell follows it (decision 110).
            if let Some(rating) = pending.rating_of(photo) {
                run.outbox.send(Event::Rated {
                    photo: photo.clone(),
                    rating,
                });
            }
        }
    }
    let cancelled = context.cancelled();
    let ended = pending.end(cancelled);
    let changed = pending.state().changed.len() as u64;
    let (.., outcome) = words(pending);
    context.set_outcome(
        named(Message::new(outcome), pending)
            .with("changed", changed)
            .with("count", pending.count as u64),
    );
    if let Some(ended) = ended {
        run.outbox.send(ended);
    }
    // The next batch job, which the engine thread starts as it wakes.
    let _ = run.inbox.send(Inbox::BatchEnded);
    failure.map_or(Ok(()), Err)
}

/// A photo's sidecar as read: the file as text, its log, its copy's clock vector, and the
/// stamp read.
type Read = (Edits, Log, Vec<Clock>, Option<sidecar::FileStamp>);

/// A photo's sidecar read through the edit writer after the writes handed over before, or
/// why it is skipped: unreadable, or open for viewing only. A photo with none has the
/// fields of darktable's `.xmp` beside it, if any, which its first write copies in
/// (decision 51).
fn read(run: &Run, context: &Context<'_>, photo: &Path) -> Result<Result<Read, Skip>, Interrupted> {
    let path = sidecar::path_for(photo);
    let snapshot = context.wait(&run.reads.read(vec![path]))?.pop();
    let Some(snapshot) = snapshot else {
        return Err(Interrupted::Disconnected);
    };
    let (modified, stamp) = (snapshot.changed(), snapshot.file_stamp());
    Ok(match snapshot.xml() {
        Err(error) => Err(Skip::Unreadable(error.to_string())),
        Ok(None) => {
            let mut file = Edits::new("");
            if let Ok(Some(darktable)) = apertrust_library::darktable::read_fields(photo) {
                *fields::first_mut(&mut file) = darktable.fields;
            }
            Ok((file, Log::default(), Vec::new(), stamp))
        }
        Ok(Some(xml)) => {
            let (operations, tools) = (run.operations, &run.tools[..]);
            match sidecar::from_xml_at(&xml, modified, operations, tools, run.retired) {
                Err(error) => Err(Skip::Unreadable(error.to_string())),
                Ok(loaded) => match loaded.view_only() {
                    Some(view_only) => Err(Skip::ViewOnly(view_only)),
                    None => {
                        let clocks = loaded.clocks().to_vec();
                        Ok((loaded.file, loaded.log, clocks, stamp))
                    }
                },
            }
        }
    })
}

/// How a photo's file handed to the engine thread ended.
enum Handed {
    /// The engine thread reached it live and recorded it.
    Live,
    /// It is opening, or its write found the file changed: read again.
    Again,
    Written,
    Changed,
    Failed(String),
}

/// `log` with its vector `clocks`, laid on `file`, handed to the engine thread to route,
/// over the file as read, `read` its stamp, again where its photo opened since.
fn hand_over(
    run: &Run,
    context: &Context<'_>,
    photo: &Path,
    (mut file, log, clocks): (Edits, Log, Vec<Clock>),
    read: Option<sidecar::FileStamp>,
) -> Result<Handed, Interrupted> {
    // What the file names the photo by, as `apertrust-cli edit` writes it.
    file.photo = Some(sidecar::file_name(photo).into_owned());
    if let Ok(fingerprint) = sidecar::read_fingerprint(photo) {
        file.fingerprint = Some(fingerprint);
    }
    let (operations, tools) = (run.operations, &run.tools[..]);
    let xml = sidecar::log_xml(&file, &log, &clocks, (operations, tools), &run.build);
    loop {
        let (answer, routed) = mpsc::channel();
        let route = Route {
            pending: Arc::clone(&run.pending),
            photo: photo.to_owned(),
            xml: xml.clone(),
            read,
            answer,
        };
        if run.inbox.send(Inbox::Route(Box::new(route))).is_err() {
            return Err(Interrupted::Disconnected);
        }
        // Handed over, it completes, cancelled or not (decision 109): the engine thread
        // answers at once, and the writer once the writes before it are done.
        let gone = |_| Interrupted::Disconnected;
        return match routed.recv().map_err(gone)? {
            Routed::Live => Ok(Handed::Live),
            // Read again once it is live, or its open has ended.
            Routed::Opening => {
                std::thread::sleep(OPENING_WAIT);
                if context.cancelled() {
                    return Err(Interrupted::Cancelled);
                }
                Ok(Handed::Again)
            }
            Routed::Handed(over) => match over.recv().map_err(gone)? {
                Over::Written => Ok(Handed::Written),
                Over::Changed => Ok(Handed::Changed),
                // Its photo opened meanwhile: routed again.
                Over::Dropped => continue,
                Over::Failed(error) => Ok(Handed::Failed(error.to_string())),
            },
        };
    }
}

/// One photo `change` makes a change of, read and handed over, read again where its file
/// changed since, twice more, then named as changing; `None` where the engine thread
/// reached it live. `change` gives the log and vector to write, the file's fields set where
/// it sets any, or why it is left: as it is, or skipped.
fn change_photo(
    run: &Run,
    context: &Context<'_>,
    photo: &Path,
    change: impl Fn(&mut Edits, Log, Vec<Clock>) -> Result<(Log, Vec<Clock>), Outcome>,
) -> Result<Option<Outcome>, Interrupted> {
    let mut tries = 0;
    while tries < TRIES {
        let (mut file, log, clocks, stamp) = match read(run, context, photo)? {
            Ok(read) => read,
            Err(skip) => return Ok(Some(Outcome::Skipped(skip))),
        };
        let (log, clocks) = match change(&mut file, log, clocks) {
            Ok(changed) => changed,
            Err(outcome) => return Ok(Some(outcome)),
        };
        let (log, clocks) = stamp_copied(&mut file, (log, clocks));
        match hand_over(run, context, photo, (file, log, clocks), stamp)? {
            Handed::Live => return Ok(None),
            Handed::Again => {}
            Handed::Written => return Ok(Some(Outcome::Changed)),
            Handed::Changed => tries += 1,
            Handed::Failed(reason) => return Ok(Some(Outcome::Skipped(Skip::Failed(reason)))),
        }
    }
    Ok(Some(Outcome::Skipped(Skip::KeptChanging)))
}

/// One photo of the batch to apply, not live as the job began it: on the disk, then its
/// metadata's image context and the style applied to its log, or a rating set with no raw
/// read ([`rate`]) ([`change_photo`]).
fn reach(run: &Run, context: &Context<'_>, photo: &Path) -> Result<Option<Outcome>, Interrupted> {
    let skipped = |skip| Ok(Some(Outcome::Skipped(skip)));
    match std::fs::metadata(photo) {
        Ok(metadata) if !listing::is_local(&metadata) => return skipped(Skip::NotOnDisk),
        Ok(_) => {}
        Err(error) => return skipped(Skip::Unreadable(error.to_string())),
    }
    let pending = &run.pending;
    let style = match &pending.change {
        Change::Style(style) => style,
        Change::Rate(change) => return rate(run, context, photo, *change),
    };
    let image = match (run.load)(photo).and_then(|file| file.metadata()) {
        Ok(metadata) => metadata.image,
        Err(error) => return skipped(Skip::Unreadable(error.to_string())),
    };
    change_photo(run, context, photo, |_, log, clocks| {
        let style = style.style();
        match style_log(
            (log, clocks),
            &run.defaults,
            style,
            (&run.tools, &image),
            pending.batch,
        ) {
            Ok(Some(styled)) => Ok(styled),
            Ok(None) => Err(Outcome::Unchanged),
            Err(error) => Err(Outcome::Skipped(Skip::Refused(error.to_string()))),
        }
    })
}

/// The fields `file` was read with from darktable's `.xmp`, which have no clock yet, given
/// one of their own as they are first written, the copy's vector advanced with it
/// (decision 51): `log` and its vector as they are to be written.
fn stamp_copied(file: &mut Edits, (log, clocks): (Log, Vec<Clock>)) -> (Log, Vec<Clock>) {
    let copied = (file.copies.first()).is_some_and(|copy| {
        let unstamped = |entry: &apertrust_library::fields::FieldEntry| entry.clock.is_none();
        copy.fields.iter().any(unstamped)
    });
    if !copied {
        return (log, clocks);
    }
    let mut history = PhotoHistory::read_at(log, clocks, &[], wall_clock());
    fields::stamp_darktable(file, || history.stamp());
    history.into_log()
}

/// The rating `change` set on one photo of its batch, its log left as it is (decision 110):
/// each field the change sets taking a clock of the batch's after the photo's latest, and
/// its record kept with the batch ([`change_photo`]).
fn rate(
    run: &Run,
    context: &Context<'_>,
    photo: &Path,
    change: RatingChange,
) -> Result<Option<Outcome>, Interrupted> {
    let pending = &run.pending;
    // The record kept once its write lands: where the photo is reached live instead, the
    // engine thread keeps the live photo's.
    let rated = std::cell::RefCell::new(None);
    let reached = change_photo(run, context, photo, |file, log, clocks| {
        let mut history = PhotoHistory::read_at(log, clocks, &[], pending.batch.wall);
        let clock = history.stamp();
        let changes = change.apply(fields::first_mut(file), clock);
        if changes.is_empty() {
            return Err(Outcome::Unchanged);
        }
        let record = FieldRecord::new(change, changes, clock, Some(pending.batch.id));
        *rated.borrow_mut() = Some((record, Rating::of(fields::first_mut(file))));
        Ok(history.into_log())
    })?;
    if let (Some(Outcome::Changed), Some((record, rating))) = (&reached, rated.into_inner()) {
        pending.keep_rated(photo, record, rating);
    }
    Ok(reached)
}

/// One photo of the batch to take back, or put back where `redo`, by its end's move
/// ([`moved_log`]), or a rating's by its record ([`moved_rating`]): no raw read.
fn moved(
    run: &Run,
    context: &Context<'_>,
    photo: &Path,
    redo: bool,
) -> Result<Option<Outcome>, Interrupted> {
    if let Change::Rate(_) = run.pending.change {
        return moved_rating(run, context, photo, redo);
    }
    let batch = run.pending.batch.id;
    change_photo(run, context, photo, |_, log, clocks| {
        moved_log((log, clocks), batch, redo).map_err(Outcome::Skipped)
    })
}

/// One photo of a rating's batch to take back, or put back where `redo`: each field the
/// batch's record set put as it was before the batch, or as the batch left it, at a clock of
/// its own, but where it changed since (decision 110); [`Skip::ChangedSince`] where every
/// one did, or where the batch keeps no record of the photo.
fn moved_rating(
    run: &Run,
    context: &Context<'_>,
    photo: &Path,
    redo: bool,
) -> Result<Option<Outcome>, Interrupted> {
    let pending = &run.pending;
    // The record as this put leaves it, kept once its write lands: where the photo is open
    // the engine thread puts it there instead, from the record as it was.
    let put = std::cell::RefCell::new(None);
    let reached = change_photo(run, context, photo, |file, log, clocks| {
        let mut history = PhotoHistory::read_at(log, clocks, &[], wall_clock());
        let clock = history.stamp();
        let Some(kept) = pending.put_rated(photo, fields::first_mut(file), redo, clock) else {
            return Err(Outcome::Skipped(Skip::ChangedSince));
        };
        *put.borrow_mut() = Some(kept);
        Ok(history.into_log())
    })?;
    if let (Some(Outcome::Changed), Some((record, rating))) = (&reached, put.into_inner()) {
        pending.keep_rated(photo, record, rating);
    }
    Ok(reached)
}

/// A photo's log, as its sidecar holds it, with its end moved back over the batch `batch`,
/// or forward over it where `redo`, as Undo's and Redo's fallback move it with no undo
/// record, the copy's clock vector advanced (decision 109): the log and vector to write;
/// [`Skip::ChangedSince`] where its end is no longer on the batch's last step, or its next
/// step after the end is not the batch's first.
pub(super) fn moved_log(
    (log, clocks): (Log, Vec<Clock>),
    batch: BatchId,
    redo: bool,
) -> Result<(Log, Vec<Clock>), Skip> {
    let mut history = PhotoHistory::read(log, clocks, &[]);
    if !history.moves_batch(batch, redo) {
        return Err(Skip::ChangedSince);
    }
    let edit = history.stepped().clone();
    match redo {
        true => history.redo(&edit),
        false => history.undo(&edit),
    };
    Ok(history.into_log())
}
