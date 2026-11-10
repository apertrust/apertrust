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

//! The session's part of the disk cache (decision 76; `docs/cache.md`, Through the
//! session). The engine opens no folder (decision 43): a binary opens the cache on the
//! folder it chose and hands it to the session with the limit its settings give, as
//! [`CacheOptions`]; the CLI hands none, and the golden harness starts no session. The
//! session keeps it within the limit with two jobs on darktable's `SystemBackground`
//! queue, neither with a progress, so neither is ever shown:
//!
//! - **`cache-walk`,** added as the session starts: the tidy of old temporaries, then the
//!   walk, whose sizes the handle then keeps up to date with its own writes and removals
//!   (walked again once should the handle's own Clear overtake it, [`Cache::walk`]),
//!   then, where the limited kinds are past the limit, an eviction in the same job. It
//!   counts as an eviction running, so a request meanwhile is folded into it;
//! - **`cache-evict`,** one at a time, asked for after a write that leaves the limited
//!   kinds past the limit ([`Upkeep::check`]), and when the limit is lowered or turned on
//!   ([`CacheCommand::Limit`](apertrust_base::cache::CacheCommand::Limit), [`Upkeep::set_limit`]); never while the cache is unlimited.
//!   A request while one is queued is that one's, which reads the limit as it runs; a
//!   request while one runs is folded into it, which walks and evicts once more before it
//!   ends. One that panics leaves eviction free for the next request.
//!
//! The client's own commands add two more:
//!
//! - **a `cache-walk` for the sizes,** on the same queue, one at a time, for
//!   [`CacheCommand::Sizes`](apertrust_base::cache::CacheCommand::Sizes): the engine
//!   thread answers at once with the handle's count where a walk has run, and the walk
//!   answers again with what other processes did meanwhile; a `Sizes` while it runs is
//!   folded into a pass more;
//! - **`cache-clear`,** on darktable's `UserBackground` queue with a progress, cancellable,
//!   as darktable's Rebuild Cache, for
//!   [`CacheCommand::Clear`](apertrust_base::cache::CacheCommand::Clear): its row's title,
//!   detail and outcome are `job-cache-…` messages, and it answers the sizes as it ends.
//!   One kind is cleared by one job at a time.
//!
//! Only the engine thread adds jobs, since it owns the control: a worker that finds the
//! limit passed asks it through the session's inbox. As the session ends, the tidy, the
//! walk and an eviction's listing stop before their next subfolder, and an eviction's
//! removals before the next, so quitting never waits for a cold walk; a Clear, cancellable,
//! stops before its next removal.
//!
//! Under `-d perf`, the walk's sizes and time and each eviction's and Clear's removals,
//! failures and time, with each kind's sizes under `-d verbose`; a cache that cannot be
//! tidied, walked, evicted or cleared is warned of under `settings`, as its folder is
//! logged.

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use apertrust_base::cache::{Cache, CacheEvent, KINDS, Kind, Limit, Removed, Sizes};
use apertrust_base::jobs::{Argument, Context, Control, Failure, Job, JobId, Message, Queue};

use super::{Event, SessionError};

/// The disk cache a binary hands its session as it starts (decision 76).
#[derive(Clone, Debug)]
pub struct CacheOptions {
    /// The cache, on the folder the binary chose, created there
    /// ([`Cache::create`]).
    pub cache: Arc<Cache>,
    /// The limit as the settings give it as the session starts
    /// ([`crate::settings::cache_limit`]);
    /// [`CacheCommand::Limit`](apertrust_base::cache::CacheCommand::Limit) changes it.
    pub limit: Limit,
}

/// Where eviction stands: one job at a time, the start's walk counting as one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Eviction {
    Idle,
    /// Added, not yet running: a request now is its own, since it reads the limit as it
    /// runs.
    Queued,
    /// Running, and whether a request came meanwhile, so that it runs once more.
    Running {
        again: bool,
    },
}

/// A step of the cache's jobs, as a test sees them in order.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    Tidy,
    Walk,
    Evict,
}

/// The cause of a Clear that could not remove some of its entries, which a client words
/// as `job-cause-cache-remove`.
pub const CACHE_REMOVE: &str = "cache-remove";

/// The cause of a Clear whose kind's folder cannot be listed, `job-cause-cache-read`.
pub const CACHE_READ: &str = "cache-read";

/// Where the walk for the sizes stands.
#[derive(Debug, Default)]
struct Sizing {
    /// Whether its job is queued or running.
    job: bool,
    /// The `Sizes` commands no pass begun after them has answered yet: one queued takes
    /// them in its first pass, one running in a pass more.
    asked: Vec<u64>,
}

struct State {
    limit: Limit,
    eviction: Eviction,
    sizing: Sizing,
    /// Each Clear with its kind, while it may not have ended.
    clears: Vec<(Kind, JobId)>,
    /// The steps taken, in order, for a test.
    #[cfg(test)]
    steps: Vec<Step>,
    /// How many eviction jobs were added, for a test.
    #[cfg(test)]
    added: usize,
    /// Makes the next eviction's pass panic, for a test.
    #[cfg(test)]
    panic: bool,
}

/// How a worker asks the engine thread for an eviction: the session's inbox.
pub(crate) type Ask = Arc<dyn Fn() + Send + Sync>;

/// How the cache's jobs tell the client: the session's outbox.
pub(crate) type Tell = Arc<dyn Fn(Event) + Send + Sync>;

/// The session's keeper of its cache, cloned into each of the cache's jobs.
#[derive(Clone)]
pub(crate) struct Upkeep {
    cache: Arc<Cache>,
    state: Arc<Mutex<State>>,
    ask: Ask,
    /// Set as the session ends: the tidy, the walk and an eviction stop.
    stop: Arc<AtomicBool>,
    /// Held by a test to keep the start's job or an eviction's pass waiting for it.
    #[cfg(test)]
    hold: Arc<Mutex<()>>,
    /// Set by a test to hold the walk for the sizes before each subfolder.
    #[cfg(test)]
    walk_hold: Arc<Mutex<Option<crate::export_job::testing::Hold>>>,
}

/// Puts eviction back to idle should a job's body panic, so the next request adds one.
struct Unwinding<'a>(&'a Upkeep);

impl Drop for Unwinding<'_> {
    fn drop(&mut self) {
        // On a normal end the job has set it, under the lock that folds a request in.
        if std::thread::panicking() {
            self.0.lock().eviction = Eviction::Idle;
        }
    }
}

/// Leaves the walk for the sizes free should its body panic, so the next `Sizes` adds one.
struct SizingUnwinding<'a>(&'a Upkeep);

impl Drop for SizingUnwinding<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.0.lock().sizing = Sizing::default();
        }
    }
}

impl Upkeep {
    /// Keeps `options`' cache for the session whose jobs `control` runs, from the engine
    /// thread: adds the start's `cache-walk`. A worker asks for an eviction through `ask`,
    /// which the engine thread answers with [`Upkeep::evict`]; `stop` is the session's
    /// end.
    pub(crate) fn start(
        options: CacheOptions,
        control: &Control,
        ask: Ask,
        stop: Arc<AtomicBool>,
    ) -> Upkeep {
        let upkeep = Upkeep::new(options, ask, stop);
        upkeep.start_walk(control);
        upkeep
    }

    /// The keeper, its walk not yet added: counted as an eviction running from now, so a
    /// request before the walk ends is folded into it.
    fn new(options: CacheOptions, ask: Ask, stop: Arc<AtomicBool>) -> Upkeep {
        Upkeep {
            cache: options.cache,
            state: Arc::new(Mutex::new(State {
                limit: options.limit,
                eviction: Eviction::Running { again: false },
                sizing: Sizing::default(),
                clears: Vec::new(),
                #[cfg(test)]
                steps: Vec::new(),
                #[cfg(test)]
                added: 0,
                #[cfg(test)]
                panic: false,
            })),
            ask,
            stop,
            #[cfg(test)]
            hold: Arc::default(),
            #[cfg(test)]
            walk_hold: Arc::default(),
        }
    }

    /// Adds the start's `cache-walk`.
    fn start_walk(&self, control: &Control) {
        let start = self.clone();
        let job = Job::new("cache-walk", "the cache's tidy and walk", move |context| {
            start.tidy_and_walk(context);
            Ok(())
        });
        control.add(Queue::SystemBackground, job);
    }

    /// The start's job: the tidy, then the walk, then eviction where the limited kinds are
    /// past the limit or a request came meanwhile.
    fn tidy_and_walk(&self, context: &Context<'_>) {
        let _unwinding = Unwinding(self);
        let cancel = || context.cancelled() || self.stop.load(Ordering::Relaxed);
        #[cfg(test)]
        drop(self.hold.lock().unwrap_or_else(PoisonError::into_inner));
        let started = Instant::now();
        let folder = self.cache.folder().display();
        #[cfg(test)]
        self.took(Step::Tidy);
        let tidied = match self.cache.tidy(&cancel) {
            Ok(tidied) => tidied,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {
                return self.idle();
            }
            Err(error) => {
                tracing::warn!(target: "settings", "cache not tidied: {folder}: {error}");
                0
            }
        };
        #[cfg(test)]
        self.took(Step::Walk);
        match self.walk(&cancel) {
            Ok(Some(sizes)) => log_walk(&sizes, tidied, self.limit(), started.elapsed()),
            Ok(None) => log_overtaken(),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {
                return self.idle();
            }
            Err(error) => tracing::warn!(target: "settings", "cache not walked: {folder}: {error}"),
        }
        // Running still, so the check folds its own request in.
        self.check();
        self.evict_while_asked(&cancel);
    }

    /// Asks for an eviction where the limited kinds are past the limit as the handle counts
    /// them, from any thread: after the start's walk, and after each write to a limited
    /// kind, the thumbnails' writes. One running takes it in; one
    /// queued is it; otherwise the engine thread is asked to add one. Nothing while the
    /// cache is unlimited, or before its walk.
    pub(crate) fn check(&self) {
        let Limit::Bytes(limit) = self.limit() else {
            return;
        };
        let past = self
            .cache
            .sizes()
            .is_some_and(|sizes| sizes.limited().bytes > limit);
        if !past {
            return;
        }
        let mut state = self.lock();
        match state.eviction {
            Eviction::Queued | Eviction::Running { again: true } => {}
            Eviction::Running { again: false } => {
                state.eviction = Eviction::Running { again: true };
            }
            Eviction::Idle => {
                drop(state);
                (self.ask)();
            }
        }
    }

    /// The limit `cache.limited` and `cache.limit` now give, from the engine thread: an
    /// eviction where it is lowered or turned on, none where it is raised, the same or
    /// turned off.
    pub(crate) fn set_limit(&self, limit: Limit, control: &Control) {
        let before = std::mem::replace(&mut self.lock().limit, limit);
        let lowered = match (before, limit) {
            (Limit::Unlimited, Limit::Bytes(_)) => true,
            (Limit::Bytes(before), Limit::Bytes(now)) => now < before,
            (_, Limit::Unlimited) => false,
        };
        if lowered {
            self.evict(control);
        }
    }

    /// Adds a `cache-evict` job, from the engine thread, unless one is queued already or
    /// the cache is unlimited; one running, the start's walk among them, takes the request
    /// in and runs once more.
    pub(crate) fn evict(&self, control: &Control) {
        let mut state = self.lock();
        if state.limit == Limit::Unlimited {
            return;
        }
        match state.eviction {
            Eviction::Idle => {}
            Eviction::Queued | Eviction::Running { again: true } => return,
            Eviction::Running { again: false } => {
                state.eviction = Eviction::Running { again: true };
                return;
            }
        }
        state.eviction = Eviction::Queued;
        #[cfg(test)]
        {
            state.added += 1;
        }
        drop(state);
        let upkeep = self.clone();
        let job = Job::new("cache-evict", "the cache's eviction", move |context| {
            upkeep.run_eviction(context);
            Ok(())
        });
        control.add(Queue::SystemBackground, job);
    }

    /// The eviction's job: its own request, then any that came meanwhile.
    fn run_eviction(&self, context: &Context<'_>) {
        let _unwinding = Unwinding(self);
        let cancel = || context.cancelled() || self.stop.load(Ordering::Relaxed);
        self.lock().eviction = Eviction::Running { again: true };
        self.evict_while_asked(&cancel);
    }

    /// Walks the limited kinds and removes the oldest past the limit, as it stands as each
    /// pass begins, while a request came since the last; then idle, under the lock a
    /// request takes, so none is lost.
    fn evict_while_asked(&self, cancel: &(dyn Fn() -> bool + Sync)) {
        loop {
            let limit = {
                let mut state = self.lock();
                if state.eviction != (Eviction::Running { again: true }) || cancel() {
                    state.eviction = Eviction::Idle;
                    return;
                }
                state.eviction = Eviction::Running { again: false };
                state.limit
            };
            #[cfg(test)]
            drop(self.hold.lock().unwrap_or_else(PoisonError::into_inner));
            #[cfg(test)]
            {
                let panic = std::mem::take(&mut self.lock().panic);
                assert!(!panic, "an eviction panicked, as a test asked");
            }
            if let Limit::Bytes(_) = limit
                && !cancel()
            {
                #[cfg(test)]
                self.took(Step::Evict);
                let started = Instant::now();
                match self.cache.evict(limit, cancel) {
                    Ok(removed) => log_eviction(&removed, limit, started.elapsed()),
                    Err(error) => {
                        let folder = self.cache.folder().display();
                        tracing::warn!(target: "settings", "cache not evicted: {folder}: {error}");
                    }
                }
            }
        }
    }

    /// Eviction idle, the job stopped.
    fn idle(&self) {
        self.lock().eviction = Eviction::Idle;
    }

    fn limit(&self) -> Limit {
        self.lock().limit
    }

    /// The cache it keeps, which the thumbnail jobs read and write.
    pub(crate) fn cache(&self) -> &Cache {
        &self.cache
    }

    /// The sizes as the handle counts them, within the limit, once a walk has run.
    pub(crate) fn counted(&self) -> Option<CacheEvent> {
        let sizes = self.cache.sizes()?;
        Some(CacheEvent::sizes(sizes, self.limit()))
    }

    /// [`CacheCommand::Sizes`](apertrust_base::cache::CacheCommand::Sizes), the command
    /// numbered `command`, from the engine thread: a `cache-walk` that tells the sizes it
    /// finds through `tell`. One queued already takes the command in; one running walks
    /// once more for it, as eviction folds a request in, so every `Sizes` is answered by a
    /// walk begun after it. A walk that fails is told as [`SessionError::CacheWalk`] for
    /// each command its pass answers.
    pub(crate) fn walk_for_sizes(&self, command: u64, control: &Control, tell: Tell) {
        {
            let mut state = self.lock();
            state.sizing.asked.push(command);
            if state.sizing.job {
                return;
            }
            state.sizing.job = true;
        }
        let upkeep = self.clone();
        let job = Job::new(
            "cache-walk",
            "the cache's walk for its sizes",
            move |context| {
                upkeep.walk_while_asked(context, &tell);
                Ok(())
            },
        );
        control.add(Queue::SystemBackground, job);
    }

    /// The walk for the sizes' job: a pass for the commands asked before it began, then
    /// one more while any came meanwhile; then free, under the lock a `Sizes` takes, so
    /// none is left unanswered.
    fn walk_while_asked(&self, context: &Context<'_>, tell: &Tell) {
        let _unwinding = SizingUnwinding(self);
        let stopped = || context.cancelled() || self.stop.load(Ordering::Relaxed);
        let cancel = || {
            #[cfg(test)]
            if let Some(hold) = &*self
                .walk_hold
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
            {
                hold.wait();
            }
            stopped()
        };
        loop {
            let commands = {
                let mut state = self.lock();
                let commands = std::mem::take(&mut state.sizing.asked);
                if commands.is_empty() || stopped() {
                    state.sizing.job = false;
                    return;
                }
                commands
            };
            let started = Instant::now();
            match self.walk(&cancel) {
                Ok(Some(sizes)) => {
                    log_sizes(&sizes, self.limit(), started.elapsed());
                    tell(Event::Cache(CacheEvent::sizes(sizes, self.limit())));
                }
                // The handle's own removals overtook both walks: its count holds them.
                Ok(None) => {
                    log_overtaken();
                    if let Some(counted) = self.counted() {
                        tell(Event::Cache(counted));
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => {
                    let folder = self.cache.folder().display();
                    tracing::warn!(target: "settings", "cache not walked: {folder}: {error}");
                    for command in commands {
                        let source = io::Error::new(error.kind(), error.to_string());
                        tell(Event::Failed(SessionError::CacheWalk { command, source }));
                    }
                }
            }
        }
    }

    /// Walks the cache, and once more should the handle's own eviction or clear overtake
    /// the walk ([`Cache::walk`]); `None` where it overtook both, the handle's count then
    /// holding what they removed.
    fn walk(&self, cancel: &(dyn Fn() -> bool + Sync)) -> io::Result<Option<Sizes>> {
        match self.cache.walk(cancel)? {
            Some(sizes) => Ok(Some(sizes)),
            None => self.cache.walk(cancel),
        }
    }

    /// Whether a Clear of `kind` is queued or running, from the engine thread: one whose
    /// job has ended is forgotten.
    pub(crate) fn clearing(&self, kind: Kind, control: &Control) -> bool {
        let mut state = self.lock();
        state
            .clears
            .retain(|&(_, job)| control.state(job).is_some());
        state.clears.iter().any(|&(cleared, _)| cleared == kind)
    }

    /// Notes that `job` clears `kind`, from the engine thread, as it adds it.
    pub(crate) fn clears(&self, kind: Kind, job: JobId) {
        self.lock().clears.push((kind, job));
    }

    /// The `cache-clear` job of [`CacheCommand::Clear`] of the limited `kind`, for the
    /// engine thread to add on `UserBackground`: base's [`Cache::clear`], its progress the
    /// row's bar and detail, its cancel the row's ✕ and the session's end. It tells the
    /// sizes through `tell` as it ends, cancelled or not. Entries it could not remove fail
    /// the job, `cache-remove`, as does a kind's folder that cannot be listed,
    /// `cache-read`.
    ///
    /// [`CacheCommand::Clear`]: apertrust_base::cache::CacheCommand::Clear
    pub(crate) fn clear_job(
        &self,
        kind: Kind,
        tell: Tell,
        #[cfg(test)] hold: Option<crate::export_job::testing::Hold>,
    ) -> Job {
        let upkeep = self.clone();
        let title = Message::new("job-cache-clear").with("kind", kind.id());
        let description = format!("clearing the cache's {}", kind.id());
        Job::new("cache-clear", description, move |context| {
            let cancel = || context.cancelled() || upkeep.stop.load(Ordering::Relaxed);
            let progress = |done: u64, total: u64| {
                context.set_progress(done, Some(total));
                let detail = Message::new("job-cache-clear-now")
                    .with("done", done)
                    .with("total", total);
                context.set_detail(Some(detail));
                #[cfg(test)]
                if let Some(hold) = &hold {
                    hold.wait();
                }
            };
            let started = Instant::now();
            let cleared = upkeep.cache.clear(kind, &progress, &cancel);
            if let Ok(removed) = &cleared {
                log_clear(kind, removed, started.elapsed());
                let id = if removed.cancelled {
                    "job-cache-clear-cancelled"
                } else {
                    "job-cache-cleared"
                };
                let outcome = Message::new(id)
                    .with("kind", kind.id())
                    .with("entries", removed.entries);
                context.set_outcome(with_size(outcome, removed.bytes));
            }
            upkeep.tell_sizes(&tell);
            match cleared {
                Ok(removed) if removed.failed > 0 && !removed.cancelled => {
                    let reason = format!(
                        "{} of the cache's {} could not be removed",
                        removed.failed,
                        kind.id()
                    );
                    Err(Failure::new(CACHE_REMOVE, reason))
                }
                Ok(_) => Ok(()),
                Err(error) => {
                    let folder = upkeep.cache.kind_folder(kind);
                    let folder = folder.display();
                    tracing::warn!(target: "settings", "cache not cleared: {folder}: {error}");
                    let reason = format!("the cache's {} cannot be listed: {error}", kind.id());
                    Err(Failure::new(CACHE_READ, reason))
                }
            }
        })
        .progress(title, true)
    }

    /// Tells the sizes as the handle counts them, walking first where no walk has run;
    /// nothing should that walk fail, or the session end meanwhile.
    fn tell_sizes(&self, tell: &Tell) {
        let sizes = match self.cache.sizes() {
            Some(sizes) => Some(sizes),
            None => self
                .walk(&|| self.stop.load(Ordering::Relaxed))
                .ok()
                .flatten(),
        };
        if let Some(sizes) = sizes {
            tell(Event::Cache(CacheEvent::sizes(sizes, self.limit())));
        }
    }

    /// Notes a step, for the tests.
    #[cfg(test)]
    fn took(&self, step: Step) {
        self.lock().steps.push(step);
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        // The state stays whole whatever panicked while it was held.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// `bytes` in megabytes of 10⁶ bytes, as disks count them and `cache.limit` does.
fn megabytes(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1e6)
}

/// The limit as a log line shows it.
fn shown(limit: Limit) -> String {
    match limit {
        Limit::Unlimited => "unlimited".to_owned(),
        Limit::Bytes(bytes) => format!("{:.1} GB", bytes as f64 / 1e9),
    }
}

fn milliseconds(took: Duration) -> f64 {
    took.as_secs_f64() * 1e3
}

/// The start's walk under `-d perf`: the limited kinds' total, the temporaries tidied and
/// the time, then each kind's under `-d verbose`.
fn log_walk(sizes: &Sizes, tidied: u64, limit: Limit, took: Duration) {
    let limited = sizes.limited();
    let (entries, bytes) = (limited.entries, megabytes(limited.bytes));
    let (limit, ms) = (shown(limit), milliseconds(took));
    tracing::debug!(
        target: "perf",
        "cache walked: {entries} entries, {bytes}, limit: {limit}, tidied: {tidied}, {ms:.1} ms"
    );
    for kind in KINDS {
        let usage = sizes.of(kind);
        let (entries, bytes) = (usage.entries, megabytes(usage.bytes));
        tracing::trace!(target: "perf", "cache {}: {entries} entries, {bytes}", kind.id());
    }
}

/// A walk for the sizes under `-d perf`: the limited kinds' total and the time.
fn log_sizes(sizes: &Sizes, limit: Limit, took: Duration) {
    let limited = sizes.limited();
    let (entries, bytes) = (limited.entries, megabytes(limited.bytes));
    let (limit, ms) = (shown(limit), milliseconds(took));
    tracing::debug!(
        target: "perf",
        "cache walked for its sizes: {entries} entries, {bytes}, limit: {limit}, {ms:.1} ms"
    );
}

/// A Clear under `-d perf`: its kind, what it removed, what it could not, and the time.
fn log_clear(kind: Kind, removed: &Removed, took: Duration) {
    let (entries, bytes, failed) = (removed.entries, megabytes(removed.bytes), removed.failed);
    let ms = milliseconds(took);
    let stopped = if removed.cancelled { ", stopped" } else { "" };
    tracing::debug!(
        target: "perf",
        "cache cleared: {}, {entries} entries, {bytes}, failed: {failed}, {ms:.1} ms{stopped}",
        kind.id()
    );
}

/// `message` with `bytes` as a person reads a size, [`size_in_unit`]'s `$size` in `$unit`;
/// the client words the unit.
fn with_size(message: Message, bytes: u64) -> Message {
    let (value, at_least, unit) = size_in_unit(bytes);
    let at_least = at_least as u8;
    let size = Argument::Decimals { value, at_least };
    message.with("size", size).with("unit", unit)
}

/// `bytes` as a person reads a size: a number, the decimals it is written with, and its
/// unit, `kb`, `mb` or `gb`, of 10³, 10⁶ and 10⁹ bytes as disks count them; one decimal
/// below 10, "1.0", and none from there, as a Clear's messages and Settings › Storage give
/// it.
pub fn size_in_unit(bytes: u64) -> (f64, usize, &'static str) {
    let rounded = |value: f64| {
        if value < 10.0 {
            (value * 10.0).round() / 10.0
        } else {
            value.round()
        }
    };
    // The unit chosen after rounding, so 999,800 bytes are 1.0 MB, never 1000 kB.
    let units = [("kb", 1e3), ("mb", 1e6), ("gb", 1e9)];
    let (unit, size) = units
        .iter()
        .map(|&(unit, scale)| (unit, rounded(bytes as f64 / scale)))
        .find(|&(unit, size)| size < 1000.0 || unit == "gb")
        .expect("gb takes any size");
    (size, usize::from(size < 10.0), unit)
}

/// A walk the handle's own eviction or clear overtook twice, under `-d perf`: the count
/// stays as it was.
fn log_overtaken() {
    tracing::debug!(target: "perf", "cache walked while it removed, twice: its count kept");
}

/// An eviction under `-d perf`: what it removed, what it could not, and the time.
fn log_eviction(removed: &Removed, limit: Limit, took: Duration) {
    let (entries, bytes, failed) = (removed.entries, megabytes(removed.bytes), removed.failed);
    let (limit, ms) = (shown(limit), milliseconds(took));
    let stopped = if removed.cancelled { ", stopped" } else { "" };
    tracing::debug!(
        target: "perf",
        "cache evicted: {entries} entries, {bytes}, failed: {failed}, limit: {limit}, {ms:.1} ms{stopped}"
    );
}

#[cfg(test)]
mod tests;
