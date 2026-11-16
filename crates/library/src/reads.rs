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

//! Bounded file reads whose caller can stop waiting for a blocked filesystem call.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, LazyLock, Mutex, PoisonError, mpsc};
use std::time::Duration;

/// Shared by sidecar and EXIF reads, including outstanding reads of cancelled opens.
static ADMISSION: LazyLock<Arc<Admission>> =
    LazyLock::new(|| Arc::new(Admission::new(crate::index::READERS)));

struct Admission {
    used: Mutex<usize>,
    changed: Condvar,
    limit: usize,
}

impl Admission {
    fn new(limit: usize) -> Self {
        Self {
            used: Mutex::new(0),
            changed: Condvar::new(),
            limit,
        }
    }

    fn acquire(
        self: &Arc<Self>,
        requested: usize,
        cancel: &(dyn Fn() -> bool + Sync),
    ) -> Option<Vec<Permit>> {
        loop {
            if cancel() {
                return None;
            }
            let mut used = self.used.lock().unwrap_or_else(PoisonError::into_inner);
            let count = requested.min(self.limit - *used);
            if count > 0 {
                *used += count;
                return Some((0..count).map(|_| Permit(Arc::clone(self))).collect());
            }
            let _waited = self
                .changed
                .wait_timeout(used, Duration::from_millis(20))
                .unwrap_or_else(PoisonError::into_inner);
        }
    }
}

struct Permit(Arc<Admission>);

impl Drop for Permit {
    fn drop(&mut self) {
        *self.0.used.lock().unwrap_or_else(PoisonError::into_inner) -= 1;
        self.0.changed.notify_all();
    }
}

struct Stop(Arc<AtomicBool>);

impl Drop for Stop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

/// Reads owned paths on at most `workers` threads. Cancelling returns completed results
/// without joining an outstanding filesystem call; workers stop before their next read.
pub(crate) fn read<T: Send + 'static>(
    paths: Vec<PathBuf>,
    workers: usize,
    reader: impl Fn(&Path) -> T + Send + Sync + 'static,
    cancel: &(dyn Fn() -> bool + Sync),
) -> (Vec<(usize, T)>, bool) {
    read_with_admission(paths, workers, reader, cancel, Arc::clone(&ADMISSION))
}

fn read_with_admission<T: Send + 'static>(
    paths: Vec<PathBuf>,
    workers: usize,
    reader: impl Fn(&Path) -> T + Send + Sync + 'static,
    cancel: &(dyn Fn() -> bool + Sync),
    admission: Arc<Admission>,
) -> (Vec<(usize, T)>, bool) {
    let workers = workers.max(1);
    let count = paths.len();
    if count == 0 {
        return (Vec::new(), cancel());
    }
    let Some(permits) = admission.acquire(workers.min(count), cancel) else {
        return (Vec::new(), true);
    };
    let paths = Arc::new(paths);
    let reader = Arc::new(reader);
    let next = Arc::new(AtomicUsize::new(0));
    let stop = Stop(Arc::new(AtomicBool::new(false)));
    let (send, receive) = mpsc::sync_channel(workers);
    for permit in permits {
        let (paths, reader, next, stopped, send) = (
            Arc::clone(&paths),
            Arc::clone(&reader),
            Arc::clone(&next),
            Arc::clone(&stop.0),
            send.clone(),
        );
        std::thread::spawn(move || {
            let _permit = permit;
            while !stopped.load(Ordering::Acquire) {
                let at = next.fetch_add(1, Ordering::Relaxed);
                let Some(path) = paths.get(at) else { break };
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| reader(path)));
                if send.send((at, result)).is_err() {
                    break;
                }
            }
        });
    }
    drop(send);
    let mut results = Vec::new();
    loop {
        if cancel() {
            return (results, true);
        }
        match receive.recv_timeout(Duration::from_millis(20)) {
            Ok((at, Ok(value))) => results.push((at, value)),
            Ok((_, Err(panic))) => std::panic::resume_unwind(panic),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return (results, false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waiting_for_reader_admission_is_cancellable() {
        let admission = Arc::new(Admission::new(2));
        let permits = admission.acquire(2, &|| false).unwrap();
        let polls = AtomicUsize::new(0);
        let (results, cancelled) = read_with_admission(
            vec![PathBuf::from("queued")],
            1,
            |_| -> () { panic!("no reader may start while capacity is held") },
            &|| polls.fetch_add(1, Ordering::Relaxed) > 0,
            Arc::clone(&admission),
        );
        assert!(cancelled);
        assert!(results.is_empty());
        assert_eq!(*admission.used.lock().unwrap(), 2);
        drop(permits);
        assert_eq!(admission.acquire(2, &|| false).unwrap().len(), 2);
    }

    #[test]
    fn cancellation_returns_while_a_reader_is_blocked() {
        let (started, reading) = mpsc::channel();
        let (release, gate) = mpsc::channel();
        let gate = std::sync::Mutex::new(gate);
        let admission = Arc::new(Admission::new(1));
        let worker_admission = Arc::clone(&admission);
        let mut cancelled = false;
        let stop = AtomicBool::new(false);
        std::thread::scope(|scope| {
            let (done, completed) = mpsc::channel();
            let stop_ref = &stop;
            scope.spawn(move || {
                let result = read_with_admission(
                    vec![PathBuf::from("held")],
                    1,
                    move |_| {
                        started.send(()).unwrap();
                        let _ = gate.lock().unwrap().recv();
                    },
                    &|| stop_ref.load(Ordering::Acquire),
                    worker_admission,
                );
                done.send(result).unwrap();
            });
            reading.recv_timeout(Duration::from_secs(5)).unwrap();
            stop.store(true, Ordering::Release);
            let result = completed.recv_timeout(Duration::from_secs(2));
            let held = *admission.used.lock().unwrap();
            release.send(()).unwrap();
            assert_eq!(
                held, 1,
                "cancelled reads retain their admission until they return"
            );
            if let Ok((results, stopped)) = result {
                assert!(results.is_empty());
                cancelled = stopped;
            }
        });
        assert!(
            cancelled,
            "the caller must return before the read is released"
        );
    }
}
