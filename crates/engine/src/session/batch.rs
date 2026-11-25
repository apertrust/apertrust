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
//! and hands the engine thread the file's XML, which alone knows which photo is open:
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
use apertrust_library::listing;
use apertrust_plugin::Operation;
use apertrust_tools::{ProcessingTool, ToolValues};

use super::styles::{Batch, Style, style_log};
use super::{
    Command, Effect, Engine, Event, Inbox, Load, Outbox, Over, Reads, SessionError, StyleMode,
    library,
};
use crate::history::{BatchId, Log};
use crate::jobs::{self, Context, Failure, Interrupted, Job, Message};
use crate::sidecar::{self, Edits};

/// What a batch on several photos tells (decision 109): notices, apart from the session's
/// batches of commands, naming the style's batch by its id.
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
        style: String,
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

/// A batch on several photos while it lasts: its id and clock, its style, the photos it has
/// still to reach, and what it did to those it reached.
pub(super) struct Pending {
    pub(super) batch: Batch,
    pub(super) style: Owned,
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
    /// Its job, once the engine thread started it: one batch job runs at a time.
    job: Option<jobs::JobId>,
    /// Set as its job ends, cancelled or not, or as it ends with no job: the photos not
    /// reached are left.
    ended: bool,
    /// Set as its job is cancelled: no photo is reached from then on but those whose write
    /// was handed over already.
    cancelled: bool,
}

/// How a photo of a batch was reached.
enum Outcome {
    Changed,
    /// Left as it was: it held the batch already, or the style changes nothing.
    Unchanged,
    Skipped(Skip),
}

impl Pending {
    fn new(batch: Batch, style: Owned, command: u64, photos: Vec<PathBuf>) -> Pending {
        let mut to_come: Vec<PathBuf> = Vec::new();
        for photo in photos {
            if !to_come.contains(&photo) {
                to_come.push(photo);
            }
        }
        Pending {
            batch,
            style,
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
        Some(Event::Batch(BatchEvent::Ended {
            command: self.command,
            batch: self.batch.id,
            action: BatchAction::Applied,
            style: self.style.name.clone(),
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

/// How long a job waits before it looks again at a photo that is opening.
const OPENING_WAIT: Duration = Duration::from_millis(20);

/// How many times a photo's file is read before it is named as changing: once, and twice
/// more after a write found it changed (decision 109).
const TRIES: usize = 3;

impl Engine {
    /// [`Command::ApplyStyle`] naming `photos`, the command numbered [`Self::applied`]: one
    /// batch drawn, its clock the wall clock now; the active photo, where named, changed at
    /// once, a refusal skipping it rather than the command; the others by one
    /// `apply-style` job; the batch's `Ended` now where there is no job. Refused whole while
    /// the active photo opens, as any command on the edit; a gesture under way ends first.
    pub(super) fn apply_style_to(
        &mut self,
        style: Owned,
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
        let pending = Arc::new(Pending::new(batch, style, self.applied, photos));
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
                let reached = self.reach(&pending, &photo, events);
                effect.render |= reached.render;
                effect.save |= reached.save;
            }
        }
        effect
    }

    /// The batch `pending` applied to the open photo, `photo`, as the active photo's change
    /// is, and recorded: left as it is where its log holds the batch already, skipped and
    /// named where it is open for viewing only or refuses a value of the style.
    fn reach(&mut self, pending: &Arc<Pending>, photo: &Path, events: &mut Vec<Event>) -> Effect {
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
        let applied = self.apply_edit(pending.style.command(), Instant::now());
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
            for tool in Owned::changes(pending.style.mode, &pending.style.tools) {
                self.picker_tool_changed(tool.as_deref(), events);
            }
        }
        pending.reach(photo, outcome);
        effect
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
    /// where none runs the first waiting started, or ended at once where the active photo's
    /// commands reached all its photos. Its `Ended` where it ends so.
    pub(super) fn start_batches(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        self.batches.retain(|pending| !pending.has_ended());
        while let Some(first) = self.batches.first().cloned() {
            let job = first.state().job;
            if let Some(job) = job {
                // Gone without its end, cancelled while queued or its body panicking: ended
                // here, so the batches after it start.
                if self.jobs.control.state(job).is_none() {
                    events.extend(first.end(true));
                    self.batches.remove(0);
                    continue;
                }
                break;
            }
            if first.state().to_come.is_empty() {
                events.extend(first.end(false));
                self.batches.remove(0);
                continue;
            }
            self.batch_job(first);
            break;
        }
        events
    }

    /// The batch's job, `apply-style` on `UserForeground`, cancellable and quiet.
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

    /// The batches' photos the job workers handed the engine thread since, each routed:
    /// open and live, the batch applied there; opening, told so; any other, handed to the
    /// writer over the file as read. What the live photo's change says.
    pub(super) fn take_routes(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        let mut changed = false;
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
                let effect = self.reach(&route.pending, &route.photo, &mut events);
                changed |= effect.render || effect.save;
                Routed::Live
            } else {
                Routed::Handed(self.writer.write_over(sidecar, route.xml, route.read))
            };
            // A job that has gone hears nothing.
            let _ = route.answer.send(routed);
        }
        if changed {
            self.edited_events(&mut events, true, false);
        }
        events
    }
}

/// What an `apply-style` job runs with, owned, so its body borrows nothing of the engine
/// thread's.
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

/// The job applying a batch to its photos but the active one.
fn job(run: Run) -> Job {
    let pending = &run.pending;
    let total = pending.state().to_come.len() as u64;
    let title = Message::new("job-apply-style")
        .with("style", pending.style.name.as_str())
        .with("count", pending.count as u64);
    Job::new(
        "apply-style",
        format!("apply {}", pending.style.name),
        move |context| body(&run, context),
    )
    .progress(title, true)
    .total(total)
    .quiet()
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
        // The engine thread reached it first, applying the batch to it live.
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
        let outcome = match reach(run, context, photo) {
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
        }
    }
    let cancelled = context.cancelled();
    let ended = pending.end(cancelled);
    let changed = pending.state().changed.len() as u64;
    context.set_outcome(
        Message::new("job-apply-style-outcome")
            .with("style", pending.style.name.as_str())
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

/// One photo of the batch, not live as the job began it: on the disk, its metadata's image
/// context, then its sidecar read through the edit writer, the style applied to its log and
/// its XML handed to the engine thread to route, read again where the file changed since,
/// twice more. `None` where the engine thread reached it live.
fn reach(run: &Run, context: &Context<'_>, photo: &Path) -> Result<Option<Outcome>, Interrupted> {
    let skipped = |skip| Ok(Some(Outcome::Skipped(skip)));
    match std::fs::metadata(photo) {
        Ok(metadata) if !listing::is_local(&metadata) => return skipped(Skip::NotOnDisk),
        Ok(_) => {}
        Err(error) => return skipped(Skip::Unreadable(error.to_string())),
    }
    let image = match (run.load)(photo).and_then(|file| file.metadata()) {
        Ok(metadata) => metadata.image,
        Err(error) => return skipped(Skip::Unreadable(error.to_string())),
    };
    let path = sidecar::path_for(photo);
    let pending = &run.pending;
    let mut tries = 0;
    while tries < TRIES {
        let snapshot = context.wait(&run.reads.read(vec![path.clone()]))?.pop();
        let Some(snapshot) = snapshot else {
            return Err(Interrupted::Disconnected);
        };
        let (modified, read) = (snapshot.changed(), snapshot.file_stamp());
        let (mut file, log, clocks) = match snapshot.xml() {
            Err(error) => return skipped(Skip::Unreadable(error.to_string())),
            Ok(None) => (Edits::new(""), Log::default(), Vec::new()),
            Ok(Some(xml)) => {
                let (operations, tools) = (run.operations, &run.tools[..]);
                match sidecar::from_xml_at(&xml, modified, operations, tools, run.retired) {
                    Err(error) => return skipped(Skip::Unreadable(error.to_string())),
                    Ok(loaded) => {
                        if let Some(view_only) = loaded.view_only() {
                            return skipped(Skip::ViewOnly(view_only));
                        }
                        let clocks = loaded.clocks().to_vec();
                        (loaded.file, loaded.log, clocks)
                    }
                }
            }
        };
        let styled = style_log(
            (log, clocks),
            &run.defaults,
            pending.style.style(),
            (&run.tools, &image),
            pending.batch,
        );
        let (log, clocks) = match styled {
            Ok(Some(styled)) => styled,
            Ok(None) => return Ok(Some(Outcome::Unchanged)),
            Err(error) => return skipped(Skip::Refused(error.to_string())),
        };
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
                pending: Arc::clone(pending),
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
            match routed.recv().map_err(|_| Interrupted::Disconnected)? {
                Routed::Live => return Ok(None),
                // Read again once it is live, or its open has ended.
                Routed::Opening => {
                    std::thread::sleep(OPENING_WAIT);
                    if context.cancelled() {
                        return Err(Interrupted::Cancelled);
                    }
                    break;
                }
                Routed::Handed(over) => match over.recv().map_err(|_| Interrupted::Disconnected)? {
                    Over::Written => return Ok(Some(Outcome::Changed)),
                    Over::Changed => {
                        tries += 1;
                        break;
                    }
                    // Its photo opened meanwhile: routed again.
                    Over::Dropped => {}
                    Over::Failed(error) => return skipped(Skip::Failed(error.to_string())),
                },
            }
        }
    }
    skipped(Skip::KeptChanging)
}
