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
//! [`CacheOptions`]; the golden harness hands none. The session keeps it within the limit
//! with two jobs on darktable's `SystemBackground` queue, neither with a progress, so
//! neither is ever shown:
//!
//! - **`cache-walk`,** added as the session starts: the tidy of old temporaries, then the
//!   walk, whose sizes the handle then keeps up to date with its own writes and removals,
//!   then an eviction asked for where the limited kinds are past the limit;
//! - **`cache-evict`,** one at a time, asked for after the start's walk and after a write
//!   that leave the limited kinds past the limit ([`Upkeep::check`]), and when the limit is
//!   lowered or turned on ([`Upkeep::set_limit`]); never while the cache is unlimited. A
//!   request while one is queued is that one's, which reads the limit as it runs; a
//!   request while one runs is folded into it, which walks and evicts once more before it
//!   ends.
//!
//! Only the engine thread adds jobs, since it owns the control: a worker that finds the
//! limit passed asks it through the session's inbox. As the session ends, an eviction
//! under way stops before its next removal.
//!
//! Under `-d perf`, the walk's sizes and time and each eviction's removals, failures and
//! time, with each kind's sizes under `-d verbose`; a cache that cannot be tidied, walked
//! or evicted is warned of under `settings`, as its folder is logged.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use apertrust_base::cache::{Cache, KINDS, Limit, Removed, Sizes};
use apertrust_base::jobs::{Context, Control, Job, Queue};

/// The disk cache a binary hands its session as it starts (decision 76).
#[derive(Clone, Debug)]
pub struct CacheOptions {
    /// The cache, on the folder the binary chose, created there
    /// ([`Cache::create`]).
    pub cache: Arc<Cache>,
    /// The limit as the settings give it as the session starts
    /// ([`crate::settings::cache_limit`]); [`Command::CacheLimit`](super::Command::CacheLimit)
    /// changes it.
    pub limit: Limit,
}

/// Where eviction stands: one job at a time.
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

struct State {
    limit: Limit,
    eviction: Eviction,
    /// The steps taken, in order, for a test.
    #[cfg(test)]
    steps: Vec<Step>,
    /// How many eviction jobs were added, for a test.
    #[cfg(test)]
    added: usize,
}

/// How a worker asks the engine thread for an eviction: the session's inbox.
pub(crate) type Ask = Arc<dyn Fn() + Send + Sync>;

/// The session's keeper of its cache, cloned into each of the cache's jobs.
#[derive(Clone)]
pub(crate) struct Upkeep {
    cache: Arc<Cache>,
    state: Arc<Mutex<State>>,
    ask: Ask,
    /// Set as the session ends: an eviction under way stops before its next removal.
    stop: Arc<AtomicBool>,
    /// Held by a test to keep an eviction running, each pass waiting for it.
    #[cfg(test)]
    hold: Arc<Mutex<()>>,
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
        let upkeep = Upkeep {
            cache: options.cache,
            state: Arc::new(Mutex::new(State {
                limit: options.limit,
                eviction: Eviction::Idle,
                #[cfg(test)]
                steps: Vec::new(),
                #[cfg(test)]
                added: 0,
            })),
            ask,
            stop,
            #[cfg(test)]
            hold: Arc::default(),
        };
        let start = upkeep.clone();
        let job = Job::new("cache-walk", "the cache's tidy and walk", move |_| {
            start.tidy_and_walk();
            Ok(())
        });
        control.add(Queue::SystemBackground, job);
        upkeep
    }

    /// The start's job: the tidy, then the walk, then an eviction where the limited kinds
    /// are past the limit.
    fn tidy_and_walk(&self) {
        let started = Instant::now();
        let folder = self.cache.folder().display();
        #[cfg(test)]
        self.took(Step::Tidy);
        let tidied = self.cache.tidy().unwrap_or_else(|error| {
            tracing::warn!(target: "settings", "cache not tidied: {folder}: {error}");
            0
        });
        #[cfg(test)]
        self.took(Step::Walk);
        match self.cache.walk() {
            Ok(sizes) => log_walk(&sizes, tidied, self.limit(), started.elapsed()),
            Err(error) => tracing::warn!(target: "settings", "cache not walked: {folder}: {error}"),
        }
        self.check();
    }

    /// Asks for an eviction where the limited kinds are past the limit as the handle counts
    /// them, from any thread: after the start's walk, and after each write to a limited
    /// kind, the thumbnails' with the library's step 6. Nothing while the cache is
    /// unlimited, or before its walk.
    pub(crate) fn check(&self) {
        let Limit::Bytes(limit) = self.limit() else {
            return;
        };
        let past = self
            .cache
            .sizes()
            .is_some_and(|sizes| sizes.limited().bytes > limit);
        if past {
            (self.ask)();
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
    /// the cache is unlimited; one running takes the request in and runs once more.
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

    /// The eviction's job: walks the limited kinds and removes the oldest past the limit,
    /// as it stands as each pass begins, then again while a request came meanwhile.
    fn run_eviction(&self, context: &Context<'_>) {
        let cancel = || context.cancelled() || self.stop.load(Ordering::Relaxed);
        loop {
            let limit = {
                let mut state = self.lock();
                state.eviction = Eviction::Running { again: false };
                state.limit
            };
            #[cfg(test)]
            drop(self.hold.lock().unwrap_or_else(PoisonError::into_inner));
            if let Limit::Bytes(_) = limit
                && !cancel()
            {
                #[cfg(test)]
                self.took(Step::Evict);
                let started = Instant::now();
                match self.cache.evict(limit, &cancel) {
                    Ok(removed) => log_eviction(&removed, limit, started.elapsed()),
                    Err(error) => {
                        let folder = self.cache.folder().display();
                        tracing::warn!(target: "settings", "cache not evicted: {folder}: {error}");
                    }
                }
            }
            let mut state = self.lock();
            if state.eviction == (Eviction::Running { again: true }) && !cancel() {
                continue;
            }
            state.eviction = Eviction::Idle;
            return;
        }
    }

    fn limit(&self) -> Limit {
        self.lock().limit
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
