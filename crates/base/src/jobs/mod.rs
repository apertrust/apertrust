// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2025  darktable developers (ported from src/control/jobs.c)
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

//! Background jobs, darktable's control jobs (`src/control/jobs.c`, `progress.c`,
//! `jobs/control_jobs.c`) restated as closely as Apertrust's architecture allows
//! (decision 57; `docs/engine.md`, Jobs), behind the `jobs` feature, which the engine and
//! the library turn on. Standard threads, a condvar and `tracing`; nothing about images,
//! colour, the GPU or the UI.
//!
//! A [`Control`] runs [`Job`]s on its workers, darktable's `worker 0` to `worker N`
//! ([`workers`]), from five [`Queue`]s in darktable's order with its priorities and
//! ageing ([`schedule`]): the user's actions, thumbnails in a deduplicated stack of 30, a
//! folder's reading, exports one at a time, and crawlers. A job's body is handed a
//! [`Context`] to check for cancel, report everything its row shows (its progress,
//! title, detail, destination, outcome and offers; decision 72), and wait for another
//! thread's answer; it returns a [`Failure`], or panics, which ends it failed too, cause
//! `panicked`. A job with a progress shows from the moment it is added: the control tells
//! its observer as [`JobEvent`]s, each of the four values at most every 0.5 s. `-d
//! control` logs each job added, run and discarded, as darktable's `_control_job_print`.
//!
//! Differences from darktable, each forced (decision 57's J1 to J13): no reserved
//! workers (J2); plain events, not a proxy into the toolkit (J3); message ids, not
//! translated text (J4); the workers from the threads alone (J5); no kicker thread
//! (J6); the throttle in the API (J7); every progress a job's (J8); [`Control::shutdown`]
//! cancelling what can be cancelled and finishing the rest (J9); a failure a failure, not
//! a cancel (J10); a job cancelled while queued leaving its queue at once (J14); a
//! priority that saturates at 255 (J15); a job added after shutdown discarded (J16);
//! done and a total in place of a fraction (J18); and the rest of its row in its events
//! (J19). A job's GPU work, stepped by the engine thread (J1), is the engine's.

pub mod progress;
pub mod schedule;

#[cfg(test)]
mod tests;

use std::any::Any;
use std::cell::Cell;
use std::collections::HashMap;
use std::fmt;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime};

pub use progress::{Argument, Destination, Failure, JobEvent, Message, Offer, Progress};

use progress::Throttles;
use schedule::{Added, Identity, Queues, Scheduled};

/// darktable's five job queues, `dt_job_queue_t`, in its order, which breaks ties
/// between heads of equal priority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Queue {
    /// `DT_JOB_QUEUE_USER_FG`: the user's actions (import, move, copy, rename, delete,
    /// Apply edit), priority 4, first in first out.
    UserForeground,
    /// `DT_JOB_QUEUE_SYSTEM_FG`: thumbnails and stills of what is on screen, priority 4,
    /// a deduplicated stack of 30, the newest first.
    SystemForeground,
    /// `DT_JOB_QUEUE_USER_BG`: a folder's listing and index, Rebuild Cache, priority 0.
    UserBackground,
    /// `DT_JOB_QUEUE_USER_EXPORT`: export, later print, priority 0, one running at a time.
    UserExport,
    /// `DT_JOB_QUEUE_SYSTEM_BG`: the thumbnail crawler, rescans, priority 0.
    SystemBackground,
}

impl Queue {
    /// The five in darktable's order.
    pub const ALL: [Queue; 5] = [
        Queue::UserForeground,
        Queue::SystemForeground,
        Queue::UserBackground,
        Queue::UserExport,
        Queue::SystemBackground,
    ];

    /// The priority a job starts with in this queue, as `dt_control_add_job` sets it.
    pub fn priority(self) -> u8 {
        match self {
            Queue::UserForeground | Queue::SystemForeground => schedule::FG_PRIORITY,
            Queue::UserBackground | Queue::UserExport | Queue::SystemBackground => 0,
        }
    }

    /// darktable's short name, `DT_JOB_QUEUE_` left out, as `-d control` writes it.
    pub fn name(self) -> &'static str {
        match self {
            Queue::UserForeground => "user_fg",
            Queue::SystemForeground => "system_fg",
            Queue::UserBackground => "user_bg",
            Queue::UserExport => "user_export",
            Queue::SystemBackground => "system_bg",
        }
    }
}

impl fmt::Display for Queue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A job's state, darktable's `dt_job_state_t`; its `DT_JOB_STATE_DISPOSED` is the job
/// gone, [`Control::state`]'s `None`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum State {
    /// Made, not added yet.
    Initialized,
    /// Waiting in its queue.
    Queued,
    /// Its body runs.
    Running,
    /// Its body returned, done or failed.
    Finished,
    /// Cancelled: a queued job leaves its queue and never runs; a running one stops at
    /// its next check.
    Cancelled,
    /// Dropped from its queue unrun: an equal job ran or waited, the stack was full, or
    /// the control shut down.
    Discarded,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            State::Initialized => "initialized",
            State::Queued => "queued",
            State::Running => "running",
            State::Finished => "finished",
            State::Cancelled => "cancelled",
            State::Discarded => "discarded",
        })
    }
}

/// A job's number within its control, from 1, never reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JobId(u64);

impl JobId {
    pub fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// What a job's body is: it returns `Ok(())`, cancelled or not, or the failure that ended
/// it.
type Body = Box<dyn FnOnce(&Context<'_>) -> Result<(), Failure> + Send>;

/// A job, darktable's `dt_job_t`: its kind, a description for `-d control` and its
/// deduplication, an optional key and progress, what its `Added` says beyond them, and
/// its body.
pub struct Job {
    identity: Identity,
    progress: Option<(Message, bool)>,
    destination: Option<Destination>,
    detail: Option<Message>,
    total: Option<u64>,
    retries: Option<JobId>,
    body: Body,
}

impl Job {
    /// darktable's `dt_control_job_create`: `kind` stands for its execute function and
    /// names what the job does (`"export"`, `"thumbnail"`), `description` its text for the
    /// log, compared where the job has no key.
    pub fn new(
        kind: &'static str,
        description: impl Into<String>,
        body: impl FnOnce(&Context<'_>) -> Result<(), Failure> + Send + 'static,
    ) -> Self {
        Self {
            identity: Identity {
                kind,
                key: None,
                description: description.into(),
            },
            progress: None,
            destination: None,
            detail: None,
            total: None,
            retries: None,
            body: Box::new(body),
        }
    }

    /// The bytes `SystemForeground` compares the job by, darktable's params given with
    /// `dt_control_job_set_params_with_size`: a thumbnail's photo, size and edit.
    pub fn dedup(mut self, key: impl Into<Vec<u8>>) -> Self {
        self.identity.key = Some(key.into());
        self
    }

    /// darktable's `dt_control_job_add_progress`: the job shows, with its title,
    /// darktable's message, from the moment it is added, queued, and can be cancelled from
    /// it when `cancellable`.
    pub fn progress(mut self, title: Message, cancellable: bool) -> Self {
        self.progress = Some((title, cancellable));
        self
    }

    /// Where its results go, as its `Added` says.
    pub fn destination(mut self, destination: Destination) -> Self {
        self.destination = Some(destination);
        self
    }

    /// The line under its bar while it waits, as its `Added` says.
    pub fn detail(mut self, detail: Message) -> Self {
        self.detail = Some(detail);
        self
    }

    /// How many items it will do, its `Added`'s progress the total, where it knows it.
    pub fn total(mut self, total: u64) -> Self {
        self.total = Some(total);
        self
    }

    /// The ended job it tries again, as its `Added` says.
    pub fn retries(mut self, job: JobId) -> Self {
        self.retries = Some(job);
        self
    }
}

impl fmt::Debug for Job {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Job")
            .field("identity", &self.identity)
            .field("progress", &self.progress)
            .field("destination", &self.destination)
            .field("detail", &self.detail)
            .field("total", &self.total)
            .field("retries", &self.retries)
            .finish_non_exhaustive()
    }
}

/// Why [`Context::wait`] returned without its answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interrupted {
    /// The job was cancelled: its body stops.
    Cancelled,
    /// The other side hung up without answering.
    Disconnected,
}

/// How often [`Context::wait`] looks for a cancel while it waits, and sends a held
/// value that has come due.
const WAIT_POLL: Duration = Duration::from_millis(5);

/// darktable's `dt_worker_threads` without its memory term (difference J5): 7 workers
/// where the machine has at least 4 threads, else 4.
pub fn workers(threads: usize) -> usize {
    if threads >= 4 { 7 } else { 4 }
}

/// What a body is handed, darktable's `dt_job_t` as a job's execute function sees it.
pub struct Context<'a> {
    shared: &'a Shared,
    record: &'a Arc<Record>,
}

impl Context<'_> {
    /// The job's number.
    pub fn job(&self) -> JobId {
        self.record.id
    }

    /// Whether the job was cancelled, darktable's `_job_cancelled`: a body checks
    /// between items and returns, its job ending cancelled.
    pub fn cancelled(&self) -> bool {
        self.record.cancel.load(Ordering::Acquire)
    }

    /// darktable's `dt_control_job_set_progress`, as `done` items of `total`, where it
    /// knows it (difference J18), through the throttle; nothing without a progress.
    pub fn set_progress(&self, done: u64, total: Option<u64>) {
        let progress = Progress { done, total };
        self.shared.with_shown(self.record.id, |shown, now| {
            shown.throttles.progress(progress, now)
        });
    }

    /// darktable's `dt_control_job_set_progress_message`, the row's title, through the
    /// throttle; nothing without a progress.
    pub fn set_title(&self, title: Message) {
        self.shared.with_shown(self.record.id, |shown, now| {
            shown.throttles.title(title, now)
        });
    }

    /// The one line under its bar, `None` clearing it, through the throttle; nothing
    /// without a progress.
    pub fn set_detail(&self, detail: Option<Message>) {
        self.shared.with_shown(self.record.id, |shown, now| {
            shown.throttles.detail(detail, now)
        });
    }

    /// Where its results go now, through the throttle; nothing without a progress.
    pub fn set_destination(&self, destination: Destination) {
        self.shared.with_shown(self.record.id, |shown, now| {
            shown.throttles.destination(destination, now)
        });
    }

    /// The ended row's title, held until its `Ended`, the last set kept; nothing without a
    /// progress.
    pub fn set_outcome(&self, outcome: Message) {
        self.shared.with_shown(self.record.id, |shown, _| {
            shown.outcome = Some(outcome);
            None
        });
    }

    /// What it can do once ended, held until its `Ended` with the others it offered;
    /// nothing without a progress.
    pub fn offer(&self, offer: Offer) {
        self.shared.with_shown(self.record.id, |shown, _| {
            shown.offers.push(offer);
            None
        });
    }

    /// Waits for another thread's answer, the engine's GPU step or a root's queue
    /// (decision 53), and returns at cancel, so a body never waits past it. Meanwhile a
    /// held progress, title, detail or destination goes out once it is due.
    pub fn wait<T>(&self, answer: &Receiver<T>) -> Result<T, Interrupted> {
        loop {
            if self.cancelled() {
                return Err(Interrupted::Cancelled);
            }
            match answer.recv_timeout(WAIT_POLL) {
                Ok(value) => return Ok(value),
                Err(RecvTimeoutError::Disconnected) => return Err(Interrupted::Disconnected),
                Err(RecvTimeoutError::Timeout) => {
                    if self.record.progress {
                        self.shared.send_due(self.record.id);
                    }
                }
            }
        }
    }
}

/// What every holder of a job shares: what the queues and the running list hold.
struct Record {
    id: JobId,
    identity: Identity,
    /// Whether it has a progress, and so sends events.
    progress: bool,
    cancel: AtomicBool,
}

impl Scheduled for Arc<Record> {
    fn identity(&self) -> &Identity {
        &self.identity
    }
}

/// What the control keeps of a job with a progress, which is shown: whether it can be
/// cancelled, its values through the throttle, and what its body set for its `Ended`.
struct Shown {
    cancellable: bool,
    throttles: Throttles,
    outcome: Option<Message>,
    offers: Vec<Offer>,
}

/// A job the control knows, from its add to its end.
struct Live {
    record: Arc<Record>,
    /// `None` for a job [`Control::run_now`] runs.
    queue: Option<Queue>,
    state: State,
    shown: Option<Shown>,
    body: Option<Body>,
}

/// What the lock guards, darktable's `queue_mutex`.
struct Inner {
    /// darktable's `dt_control_running`: false once the control shuts down.
    accepting: bool,
    queues: Queues<Arc<Record>>,
    /// darktable's `control->job`, the jobs the workers run, with their queues.
    running: Vec<(Queue, Arc<Record>)>,
    /// darktable's `export_scheduled`.
    export_running: bool,
    jobs: HashMap<JobId, Live>,
}

type Observer = Box<dyn Fn(JobEvent) + Send + Sync>;

struct Shared {
    inner: Mutex<Inner>,
    /// darktable's `control->cond`: a job added, or the control shutting down.
    wake: Condvar,
    observer: Observer,
    next: AtomicU64,
}

thread_local! {
    /// Whether this thread is in the observer, which runs under the control's lock.
    static IN_OBSERVER: Cell<bool> = const { Cell::new(false) };
}

impl Shared {
    /// The control's lock. The observer runs under it, so an observer that calls the
    /// control would wait for itself forever: that call panics instead, which
    /// [`Shared::notify`] catches.
    fn lock(&self) -> MutexGuard<'_, Inner> {
        assert!(
            !IN_OBSERVER.get(),
            "a jobs observer called its control: it runs under the control's lock, so it \
             must only pass the event on, a channel's send, and never call the control"
        );
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Hands the observer one event, the only call of it. A panic in it, its own or a
    /// call of the control it must not make, is logged and the event dropped, so no
    /// worker dies under the lock and no body is blamed for it.
    fn notify(&self, event: JobEvent) {
        let outer = IN_OBSERVER.replace(true);
        let outcome = panic::catch_unwind(AssertUnwindSafe(|| (self.observer)(event)));
        IN_OBSERVER.set(outer);
        if let Err(payload) = outcome {
            tracing::error!(
                target: "control",
                "the observer panicked, event dropped: {}",
                reason(payload.as_ref())
            );
        }
    }

    fn emit(&self, record: &Record, event: JobEvent) {
        if record.progress {
            self.notify(event);
        }
    }

    /// Runs `update` on the job's shown part under the lock, and sends what it returns.
    fn with_shown(&self, job: JobId, update: impl FnOnce(&mut Shown, Instant) -> Option<JobEvent>) {
        let mut inner = self.lock();
        let Some(shown) = inner.jobs.get_mut(&job).and_then(|l| l.shown.as_mut()) else {
            return;
        };
        if let Some(event) = update(shown, Instant::now()) {
            self.notify(event);
        }
    }

    /// Sends a job's held values once they are due.
    fn send_due(&self, job: JobId) {
        let mut inner = self.lock();
        let Some(shown) = inner.jobs.get_mut(&job).and_then(|l| l.shown.as_mut()) else {
            return;
        };
        for event in shown.throttles.held(Some(Instant::now())) {
            self.notify(event);
        }
    }

    /// Ends a job the control knows: its held values, then `Ended` with what its body set
    /// for it, `failure` and the time now.
    fn end(&self, inner: &mut Inner, job: JobId, state: State, failure: Option<Failure>) {
        let Some(live) = inner.jobs.remove(&job) else {
            return;
        };
        let Some(mut shown) = live.shown else {
            return;
        };
        for event in shown.throttles.held(None) {
            self.notify(event);
        }
        self.notify(JobEvent::Ended {
            job,
            state,
            outcome: shown.outcome,
            failure,
            at: SystemTime::now(),
            offers: shown.offers,
        });
    }

    /// Registers a job and sends `Added`.
    fn register(&self, inner: &mut Inner, job: Job, queue: Option<Queue>) -> Arc<Record> {
        let id = JobId(self.next.fetch_add(1, Ordering::Relaxed));
        let record = Arc::new(Record {
            id,
            identity: job.identity,
            progress: job.progress.is_some(),
            cancel: AtomicBool::new(false),
        });
        let shown = job.progress.map(|(title, cancellable)| {
            self.notify(JobEvent::Added {
                job: id,
                kind: record.identity.kind,
                queue,
                title,
                cancellable,
                destination: job.destination,
                detail: job.detail,
                progress: Progress {
                    done: 0,
                    total: job.total,
                },
                retries: job.retries,
            });
            Shown {
                cancellable,
                throttles: Throttles::new(id),
                outcome: None,
                offers: Vec::new(),
            }
        });
        inner.jobs.insert(
            id,
            Live {
                record: record.clone(),
                queue,
                state: State::Initialized,
                shown,
                body: Some(job.body),
            },
        );
        record
    }

    /// Sets a job's state and tells the observer.
    fn set_state(&self, inner: &mut Inner, job: JobId, state: State) {
        if let Some(live) = inner.jobs.get_mut(&job) {
            live.state = state;
            self.emit(&live.record, JobEvent::State { job, state });
        }
    }

    /// Runs a job's body on this thread, outside the lock: its failure, a panic caught as
    /// one, cause [`Failure::PANICKED`] with its text the reason.
    fn call(&self, record: &Arc<Record>, body: Body) -> Option<Failure> {
        let context = Context {
            shared: self,
            record,
        };
        match panic::catch_unwind(AssertUnwindSafe(|| body(&context))) {
            Ok(Ok(())) => None,
            Ok(Err(failure)) => Some(failure),
            Err(payload) => {
                let reason = reason(payload.as_ref());
                tracing::error!(
                    target: "control",
                    "job {} panicked: {reason}",
                    record.identity.description
                );
                Some(Failure::new(Failure::PANICKED, reason))
            }
        }
    }

    /// Ends a job whose body returned: cancelled if a cancel came before, else finished,
    /// with its failure either way (difference J10).
    fn finish(
        &self,
        inner: &mut Inner,
        record: &Record,
        worker: &str,
        failure: Option<Failure>,
    ) -> State {
        let state = if record.cancel.load(Ordering::Acquire) {
            State::Cancelled
        } else {
            State::Finished
        };
        let queue = inner.jobs.get(&record.id).and_then(|live| live.queue);
        log_run(
            "run_job-",
            worker,
            &record.identity,
            queue,
            None,
            Some(state),
        );
        self.end(inner, record.id, state, failure);
        state
    }

    /// Takes a job's body from its entry, `None` if it has none left.
    fn take_body(inner: &mut Inner, job: JobId) -> Option<Body> {
        inner.jobs.get_mut(&job).and_then(|live| live.body.take())
    }
}

/// darktable's `dt_control_t` job part: the queues, the workers and the jobs it knows.
/// Dropped, it shuts down ([`Control::shutdown`]).
pub struct Control {
    shared: Arc<Shared>,
    workers: Mutex<Vec<JoinHandle<()>>>,
}

impl Control {
    /// darktable's `dt_control_jobs_init`: starts [`workers`]`(threads)` workers, named
    /// `worker 0` onwards. `observer` hears every [`JobEvent`] of the jobs with a
    /// progress, in order; it runs under the control's lock, so it must be quick, a
    /// channel's send, and must never call the control.
    pub fn start(observer: impl Fn(JobEvent) + Send + Sync + 'static) -> Self {
        let threads = thread::available_parallelism().map_or(1, |threads| threads.get());
        Self::with_workers(workers(threads), observer)
    }

    fn with_workers(count: usize, observer: impl Fn(JobEvent) + Send + Sync + 'static) -> Self {
        let shared = Arc::new(Shared {
            inner: Mutex::new(Inner {
                accepting: true,
                queues: Queues::default(),
                running: Vec::new(),
                export_running: false,
                jobs: HashMap::new(),
            }),
            wake: Condvar::new(),
            observer: Box::new(observer),
            next: AtomicU64::new(1),
        });
        let workers = (0..count)
            .map(|worker| {
                let shared = shared.clone();
                thread::Builder::new()
                    .name(format!("worker {worker}"))
                    .spawn(move || work(&shared, worker))
                    .expect("the system starts a worker thread")
            })
            .collect();
        tracing::debug!(target: "control", "started {count} workers");
        Self {
            shared,
            workers: Mutex::new(workers),
        }
    }

    /// darktable's `dt_control_add_job`: queues the job and returns its number. Into
    /// `SystemForeground` a job equal to a running or queued one is discarded, the queued
    /// one moving up, and past 30 the stack's bottom is ([`schedule::add`]). Once the
    /// control has shut down the job is discarded, where darktable runs it on the
    /// caller's thread (difference J16); [`Control::run_now`] runs one there.
    pub fn add(&self, queue: Queue, job: Job) -> JobId {
        let shared = &*self.shared;
        let mut inner = shared.lock();
        let record = shared.register(&mut inner, job, Some(queue));
        let id = record.id;
        if !inner.accepting {
            log_discard(&record.identity, "shut down");
            discard(shared, &mut inner, id);
            return id;
        }
        let length = inner.queues.list(queue).len();
        log_add(&record.identity, queue, length);
        let inner = &mut *inner;
        match schedule::add(&mut inner.queues, &inner.running, queue, record) {
            Added::Queued { dropped } => {
                shared.set_state(inner, id, State::Queued);
                if let Some(dropped) = dropped {
                    log_discard(&dropped.identity, "past 30 in system_fg");
                    discard(shared, inner, dropped.id);
                }
                shared.wake.notify_all();
            }
            Added::Moved { new } => {
                log_discard(&new.identity, "an equal job is queued, moved up");
                discard(shared, inner, new.id);
            }
            Added::Running { new } => {
                log_discard(&new.identity, "an equal job is running");
                discard(shared, inner, new.id);
            }
        }
        id
    }

    /// darktable's `DT_JOB_QUEUE_SYNCHRONOUS`: runs the job on this thread, now, and
    /// returns the state it ended in, finished or cancelled, or its failure.
    pub fn run_now(&self, job: Job) -> Result<State, Failure> {
        let (_, state, failure) = self.run(job);
        match failure {
            Some(failure) => Err(failure),
            None => Ok(state),
        }
    }

    /// Runs a job on this thread: its number, the state it ended in and its failure.
    fn run(&self, job: Job) -> (JobId, State, Option<Failure>) {
        let shared = &*self.shared;
        let (record, body) = {
            let mut inner = shared.lock();
            let record = shared.register(&mut inner, job, None);
            let body = Shared::take_body(&mut inner, record.id).expect("a new job has its body");
            shared.set_state(&mut inner, record.id, State::Running);
            log_run("run_job+", "now", &record.identity, None, None, None);
            (record, body)
        };
        let failure = shared.call(&record, body);
        let mut inner = shared.lock();
        let state = shared.finish(&mut inner, &record, "now", failure.clone());
        (record.id, state, failure)
    }

    /// darktable's `dt_control_job_cancel`, for any job, cancellable or not, as
    /// darktable's: a queued job leaves its queue and ends at once, never running; a
    /// running one stops at its body's next check. Returns whether the job was queued or running.
    pub fn cancel(&self, job: JobId) -> bool {
        let shared = &*self.shared;
        let mut inner = shared.lock();
        let Some(live) = inner.jobs.get_mut(&job) else {
            return false;
        };
        match live.state {
            State::Queued => {
                live.record.cancel.store(true, Ordering::Release);
                tracing::debug!(
                    target: "control",
                    "cancelled: {}, queued",
                    live.record.identity.description
                );
                // Taken from its queue at once, where darktable leaves it until a worker
                // drops it, so an equal job added later is not discarded for it
                // (difference J14).
                if let Some(queue) = live.queue {
                    inner.queues.remove(queue, |record| record.id == job);
                }
                shared.end(&mut inner, job, State::Cancelled, None);
                true
            }
            State::Running => {
                live.record.cancel.store(true, Ordering::Release);
                tracing::debug!(
                    target: "control",
                    "cancelled: {}, running",
                    live.record.identity.description
                );
                shared.set_state(&mut inner, job, State::Cancelled);
                true
            }
            State::Initialized | State::Finished | State::Cancelled | State::Discarded => false,
        }
    }

    /// A job's state, darktable's `dt_control_job_get_state`; `None` once it has ended
    /// and gone, darktable's `DT_JOB_STATE_DISPOSED`.
    pub fn state(&self, job: JobId) -> Option<State> {
        self.shared.lock().jobs.get(&job).map(|live| live.state)
    }

    /// darktable's `dt_control_jobs_pending`: the jobs queued or running, those
    /// [`Control::run_now`] runs not counted.
    pub fn pending(&self) -> usize {
        self.shared
            .lock()
            .jobs
            .values()
            .filter(|live| live.queue.is_some())
            .count()
    }

    /// Stops the control (difference J9): every queued job discarded, every running job
    /// that can be cancelled from its progress cancelled, the rest finished, then the
    /// workers joined. Never from a job's body, whose worker it would wait for.
    pub fn shutdown(&self) {
        let shared = &*self.shared;
        {
            let mut inner = shared.lock();
            if inner.accepting {
                inner.accepting = false;
                tracing::debug!(target: "control", "shutting down");
                let inner = &mut *inner;
                for (_, entry) in inner.queues.drain() {
                    log_discard(&entry.job.identity, "shutting down");
                    discard(shared, inner, entry.job.id);
                }
                let cancellable: Vec<JobId> = inner
                    .jobs
                    .values()
                    .filter(|live| {
                        live.state == State::Running
                            && live.shown.as_ref().is_some_and(|p| p.cancellable)
                    })
                    .map(|live| live.record.id)
                    .collect();
                for job in cancellable {
                    if let Some(live) = inner.jobs.get(&job) {
                        live.record.cancel.store(true, Ordering::Release);
                    }
                    shared.set_state(inner, job, State::Cancelled);
                }
                shared.wake.notify_all();
            }
        }
        let workers =
            std::mem::take(&mut *self.workers.lock().unwrap_or_else(PoisonError::into_inner));
        for worker in workers {
            let _ = worker.join();
        }
    }
}

impl Drop for Control {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl fmt::Debug for Control {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Control")
            .field("pending", &self.pending())
            .finish_non_exhaustive()
    }
}

/// A panic's text, as `panic!` gave it.
fn reason(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|reason| (*reason).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_default()
}

/// A job taken from its queue unrun: discarded, unless it was cancelled there, which
/// ended it already.
fn discard(shared: &Shared, inner: &mut Inner, job: JobId) {
    match inner.jobs.get(&job).map(|live| live.state) {
        Some(State::Queued) | Some(State::Initialized) => {
            shared.end(inner, job, State::Discarded, None);
        }
        _ => {
            inner.jobs.remove(&job);
        }
    }
}

/// darktable's `_control_work`: takes the next job by [`schedule::pick`], runs it, and
/// waits on the condvar while none may run, until the control shuts down.
fn work(shared: &Shared, worker: usize) {
    let name = format!("worker {worker}");
    loop {
        let (record, body) = {
            let mut inner = shared.lock();
            loop {
                if !inner.accepting {
                    return;
                }
                let export_running = inner.export_running;
                let Some((queue, entry)) = schedule::pick(&mut inner.queues, export_running) else {
                    inner = shared
                        .wake
                        .wait(inner)
                        .unwrap_or_else(PoisonError::into_inner);
                    continue;
                };
                let record = entry.job;
                let state = inner.jobs.get(&record.id).map(|live| live.state);
                if state != Some(State::Queued) {
                    // A guard: a job cancelled while queued has left its queue already.
                    inner.jobs.remove(&record.id);
                    continue;
                }
                if queue == Queue::UserExport {
                    inner.export_running = true;
                }
                inner.running.push((queue, record.clone()));
                let body =
                    Shared::take_body(&mut inner, record.id).expect("a queued job has its body");
                shared.set_state(&mut inner, record.id, State::Running);
                log_run(
                    "run_job+",
                    &name,
                    &record.identity,
                    Some(queue),
                    Some(entry.priority),
                    None,
                );
                break (record, body);
            }
        };
        let failure = shared.call(&record, body);
        let mut inner = shared.lock();
        if let Some(at) = inner
            .running
            .iter()
            .position(|(_, running)| running.id == record.id)
        {
            let (queue, _) = inner.running.swap_remove(at);
            if queue == Queue::UserExport {
                inner.export_running = false;
            }
        }
        shared.finish(&mut inner, &record, &name, failure);
    }
}

/// `-d control`'s `add_job` line, as darktable's `_control_job_print` writes it: the
/// description, the queue, the priority it starts with and the queue's length before.
fn log_add(identity: &Identity, queue: Queue, length: usize) {
    tracing::debug!(
        target: "control",
        "add_job: {}, queue: {queue}, priority: {}, length: {length}",
        identity.description,
        queue.priority()
    );
}

/// `-d control`'s line for a job discarded, with why.
fn log_discard(identity: &Identity, why: &str) {
    tracing::debug!(target: "control", "discarded: {}, {why}", identity.description);
}

/// `-d control`'s `run_job+` and `run_job-` lines: the worker, the description, the
/// queue, the priority it won with and, at its end, its state.
fn log_run(
    info: &str,
    worker: &str,
    identity: &Identity,
    queue: Option<Queue>,
    priority: Option<u8>,
    state: Option<State>,
) {
    let queue = queue.map_or("synchronous", Queue::name);
    let priority = priority
        .map(|p| format!(", priority: {p}"))
        .unwrap_or_default();
    let state = state.map(|s| format!(", {s}")).unwrap_or_default();
    tracing::debug!(
        target: "control",
        "{info} {worker}: {}, queue: {queue}{priority}{state}",
        identity.description
    );
}
