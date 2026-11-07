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

//! The control on real workers, its test caller a counting job. Nothing here sleeps or
//! races: a job that must hold a worker waits for the test's go through [`Context::wait`],
//! and the test waits for what a job or the observer tells it, with a generous deadline
//! that only a failure reaches. The order of picks is [`schedule`]'s tables', so these hold
//! the control to them.

use std::fmt::Debug;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{OnceLock, Weak};
use std::time::Duration;

use super::*;
use Queue::{SystemBackground, SystemForeground, UserBackground, UserExport, UserForeground};

/// How long a test waits for what must come; only a failure waits this long.
const PATIENCE: Duration = Duration::from_secs(20);

/// What a counting job tells the test.
#[derive(Clone, Debug, PartialEq)]
enum Told {
    Started(&'static str),
    Counted(&'static str, usize),
    /// It returned: the count it reached, or why its wait ended without a go.
    Stopped(&'static str, Result<usize, Interrupted>),
}

/// What a channel has brought so far, waited for by what the test needs.
struct Heard<T> {
    receiver: Receiver<T>,
    seen: Vec<T>,
}

impl<T: Debug> Heard<T> {
    fn new(receiver: Receiver<T>) -> Self {
        Self {
            receiver,
            seen: Vec::new(),
        }
    }

    /// Waits until something heard matches.
    fn until(&mut self, mut found: impl FnMut(&T) -> bool) {
        while !self.seen.iter().any(&mut found) {
            let next = self
                .receiver
                .recv_timeout(PATIENCE)
                .unwrap_or_else(|_| panic!("still waiting, heard: {:?}", self.seen));
            self.seen.push(next);
        }
    }
}

impl Heard<Told> {
    fn started(&self) -> Vec<&'static str> {
        self.seen
            .iter()
            .filter_map(|told| match told {
                Told::Started(label) => Some(*label),
                _ => None,
            })
            .collect()
    }
}

impl Heard<JobEvent> {
    /// One job's events, in order.
    fn of(&self, job: JobId) -> Vec<JobEvent> {
        self.seen
            .iter()
            .filter(|event| id(event) == job)
            .cloned()
            .collect()
    }

    fn ended(&mut self, job: JobId) {
        self.until(|event| matches!(event, JobEvent::Ended { job: ended, .. } if *ended == job));
    }
}

fn id(event: &JobEvent) -> JobId {
    match event {
        JobEvent::Added { job, .. }
        | JobEvent::Progress { job, .. }
        | JobEvent::Message { job, .. }
        | JobEvent::State { job, .. }
        | JobEvent::Ended { job, .. } => *job,
    }
}

/// A control of `workers` workers, its events heard by the test.
fn control(workers: usize) -> (Control, Heard<JobEvent>) {
    let (sender, receiver) = mpsc::channel();
    let control = Control::with_workers(workers, move |event| {
        let _ = sender.send(event);
    });
    (control, Heard::new(receiver))
}

/// The test caller: a job that counts to `steps`, its fraction the count over `steps`,
/// telling the test as it starts, at each step and as it returns, and stopping at
/// cancel. With a `go`, it waits for one before each step.
fn counting(
    label: &'static str,
    steps: usize,
    go: Option<Receiver<()>>,
    tell: &Sender<Told>,
) -> Job {
    let tell = tell.clone();
    Job::new("count", label, move |context| {
        let _ = tell.send(Told::Started(label));
        for step in 1..=steps {
            if let Some(go) = &go {
                if let Err(interrupted) = context.wait(go) {
                    let _ = tell.send(Told::Stopped(label, Err(interrupted)));
                    return Ok(());
                }
            } else if context.cancelled() {
                let _ = tell.send(Told::Stopped(label, Err(Interrupted::Cancelled)));
                return Ok(());
            }
            context.set_fraction(step as f64 / steps as f64);
            let _ = tell.send(Told::Counted(label, step));
        }
        let _ = tell.send(Told::Stopped(label, Ok(steps)));
        Ok(())
    })
}

/// A counting job of one step that holds its worker until the test sends its go.
fn gate(label: &'static str, tell: &Sender<Told>) -> (Job, Sender<()>) {
    let (go, wait) = mpsc::channel();
    (counting(label, 1, Some(wait), tell), go)
}

fn told() -> (Sender<Told>, Heard<Told>) {
    let (sender, receiver) = mpsc::channel();
    (sender, Heard::new(receiver))
}

fn cancellable(job: Job, label: &'static str) -> Job {
    job.progress(Message::new("job-test").with("label", label), true)
}

#[test]
fn workers_are_darktables_without_its_memory_term() {
    for (threads, workers_) in [(1, 4), (2, 4), (3, 4), (4, 7), (8, 7), (64, 7)] {
        assert_eq!(workers(threads), workers_, "{threads} threads");
    }
}

/// One worker, held by a gate while the jobs are added, then released: the picks are
/// [`schedule`]'s ageing table, the user background job sixth.
#[test]
fn a_waiting_job_ages_until_it_runs() {
    let (control, _) = control(1);
    let (tell, mut heard) = told();
    let (job, go) = gate("gate", &tell);
    control.add(UserForeground, job);
    heard.until(|told| *told == Told::Started("gate"));
    for label in ["u1", "u2", "u3", "u4", "u5", "u6"] {
        control.add(UserForeground, counting(label, 1, None, &tell));
    }
    control.add(UserBackground, counting("b1", 1, None, &tell));
    go.send(()).unwrap();
    heard.until(|told| *told == Told::Stopped("u6", Ok(1)));
    heard.until(|told| *told == Told::Stopped("b1", Ok(1)));
    assert_eq!(
        heard.started(),
        ["gate", "u1", "u2", "u3", "u4", "u5", "b1", "u6"]
    );
}

/// One worker, every queue: the order of [`schedule`]'s table of five queues. With one
/// worker the first export has ended before the next pick, so g1, which has aged to 4,
/// still beats e2.
#[test]
fn jobs_run_in_darktables_order_across_queues() {
    let (control, _) = control(1);
    let (tell, mut heard) = told();
    let (job, go) = gate("gate", &tell);
    control.add(UserForeground, job);
    heard.until(|told| *told == Told::Started("gate"));
    for (queue, label) in [
        (SystemBackground, "g1"),
        (UserExport, "e1"),
        (UserExport, "e2"),
        (UserBackground, "b1"),
        (SystemForeground, "s1"),
        (UserForeground, "u1"),
    ] {
        control.add(queue, counting(label, 1, None, &tell));
    }
    assert_eq!(control.pending(), 7);
    go.send(()).unwrap();
    heard.until(|told| *told == Told::Stopped("e2", Ok(1)));
    assert_eq!(
        heard.started(),
        ["gate", "u1", "s1", "b1", "e1", "g1", "e2"]
    );
}

/// `SystemForeground` deduplicates: a job equal to the running one and one equal to a
/// queued one are discarded, shown as added and ended discarded, as darktable's
/// progress is made and destroyed; the queued one moves up.
#[test]
fn system_foreground_jobs_are_deduplicated() {
    let (control, mut events) = control(1);
    let (tell, mut heard) = told();
    let (job, go) = gate("gate", &tell);
    control.add(SystemForeground, cancellable(job.dedup(*b"gate"), "gate"));
    heard.until(|told| *told == Told::Started("gate"));

    let again = control.add(
        SystemForeground,
        cancellable(counting("again", 1, None, &tell).dedup(*b"gate"), "again"),
    );
    assert_eq!(control.state(again), None);
    events.ended(again);
    assert_eq!(
        events.of(again),
        [
            JobEvent::Added {
                job: again,
                kind: "count",
                queue: Some(SystemForeground),
                message: Message::new("job-test").with("label", "again"),
                cancellable: true,
            },
            JobEvent::Ended {
                job: again,
                state: State::Discarded,
                failure: None,
            },
        ]
    );

    control.add(
        SystemForeground,
        counting("s1", 1, None, &tell).dedup(*b"s1"),
    );
    control.add(
        SystemForeground,
        counting("s2", 1, None, &tell).dedup(*b"s2"),
    );
    let s1_again = control.add(
        SystemForeground,
        counting("s1 again", 1, None, &tell).dedup(*b"s1"),
    );
    assert_eq!(control.state(s1_again), None);
    assert_eq!(control.pending(), 3);
    go.send(()).unwrap();
    heard.until(|told| *told == Told::Stopped("s2", Ok(1)));
    assert_eq!(heard.started(), ["gate", "s1", "s2"]);
}

/// Two workers, two exports: while the first runs, the second waits though a worker is
/// free, as a user background job added after it shows by running on that worker.
#[test]
fn one_export_runs_at_a_time() {
    let (control, _) = control(2);
    let (tell, mut heard) = told();
    let (job, go) = gate("e1", &tell);
    let e1 = control.add(UserExport, job);
    heard.until(|told| *told == Told::Started("e1"));
    let e2 = control.add(UserExport, counting("e2", 1, None, &tell));
    control.add(UserBackground, counting("b1", 1, None, &tell));
    heard.until(|told| *told == Told::Stopped("b1", Ok(1)));
    assert_eq!(control.state(e1), Some(State::Running));
    assert_eq!(control.state(e2), Some(State::Queued));
    assert_eq!(heard.started(), ["e1", "b1"]);
    go.send(()).unwrap();
    heard.until(|told| *told == Told::Stopped("e2", Ok(1)));
    assert_eq!(heard.started(), ["e1", "b1", "e2"]);
}

/// A job cancelled while queued never runs: it leaves its queue and ends at once
/// (difference J14).
#[test]
fn a_job_cancelled_before_it_starts_never_runs() {
    let (control, mut events) = control(1);
    let (tell, mut heard) = told();
    let (job, go) = gate("gate", &tell);
    control.add(UserForeground, job);
    heard.until(|told| *told == Told::Started("gate"));
    let x = control.add(
        UserBackground,
        cancellable(counting("x", 3, None, &tell), "x"),
    );
    let y = control.add(UserBackground, counting("y", 1, None, &tell));
    assert_eq!(control.pending(), 3);

    assert!(control.cancel(x));
    assert_eq!(control.state(x), None, "it left its queue at once");
    assert!(!control.cancel(x), "a cancelled job is not cancelled twice");
    events.ended(x);
    assert_eq!(
        events.of(x)[1..],
        [
            JobEvent::State {
                job: x,
                state: State::Queued,
            },
            JobEvent::Ended {
                job: x,
                state: State::Cancelled,
                failure: None,
            },
        ]
    );
    assert_eq!(control.pending(), 2);

    go.send(()).unwrap();
    heard.until(|told| *told == Told::Stopped("y", Ok(1)));
    assert_eq!(heard.started(), ["gate", "y"]);
    // The job says it stopped before its worker ends it, and it sends no events: wait
    // until the control has ended it.
    let deadline = std::time::Instant::now() + PATIENCE;
    while control.state(y).is_some() {
        assert!(std::time::Instant::now() < deadline, "y never ended");
        std::thread::yield_now();
    }
    assert!(!control.cancel(y), "a job that has ended is gone");
}

/// A running job stops at its next check, here its wait for the next go, and ends
/// cancelled; its fraction so far went out first.
#[test]
fn a_job_cancelled_while_running_stops_at_its_next_check() {
    let (control, mut events) = control(1);
    let (tell, mut heard) = told();
    let (go, wait) = mpsc::channel();
    let x = control.add(
        UserBackground,
        cancellable(counting("x", 4, Some(wait), &tell), "x"),
    );
    heard.until(|told| *told == Told::Started("x"));
    go.send(()).unwrap();
    heard.until(|told| *told == Told::Counted("x", 1));
    assert!(control.cancel(x));
    heard.until(|told| matches!(told, Told::Stopped("x", _)));
    assert!(
        heard
            .seen
            .contains(&Told::Stopped("x", Err(Interrupted::Cancelled)))
    );
    events.ended(x);
    let states: Vec<_> = events
        .of(x)
        .into_iter()
        .filter(|event| !matches!(event, JobEvent::Added { .. }))
        .collect();
    assert_eq!(
        states,
        [
            JobEvent::State {
                job: x,
                state: State::Queued,
            },
            JobEvent::State {
                job: x,
                state: State::Running,
            },
            JobEvent::Progress {
                job: x,
                fraction: 0.25,
            },
            JobEvent::State {
                job: x,
                state: State::Cancelled,
            },
            JobEvent::Ended {
                job: x,
                state: State::Cancelled,
                failure: None,
            },
        ]
    );
}

/// `wait` returns at cancel though no answer ever comes, and when the other side hangs
/// up.
#[test]
fn wait_returns_at_cancel_and_when_the_other_side_hangs_up() {
    let (control, _) = control(1);
    let (tell, mut heard) = told();

    let (answer, wait) = mpsc::channel::<()>();
    let x = control.add(UserBackground, counting("x", 1, Some(wait), &tell));
    heard.until(|told| *told == Told::Started("x"));
    assert!(control.cancel(x));
    heard.until(|told| matches!(told, Told::Stopped("x", _)));
    assert!(
        heard
            .seen
            .contains(&Told::Stopped("x", Err(Interrupted::Cancelled)))
    );
    drop(answer);

    let (answer, wait) = mpsc::channel::<()>();
    control.add(UserBackground, counting("y", 1, Some(wait), &tell));
    heard.until(|told| *told == Told::Started("y"));
    drop(answer);
    heard.until(|told| matches!(told, Told::Stopped("y", _)));
    assert!(
        heard
            .seen
            .contains(&Told::Stopped("y", Err(Interrupted::Disconnected)))
    );
}

/// Fractions and messages set at once go out throttled: the first at once and the last
/// as the job ends, whatever came between; a fraction clamped to 0 to 1.
#[test]
fn progress_is_throttled_the_first_and_last_always_sent() {
    let (control, mut events) = control(1);
    let job = Job::new("count", "throttled", |context| {
        context.set_fraction(-0.5);
        context.set_message(Message::new("job-first"));
        for step in 1..=99 {
            context.set_fraction(f64::from(step) / 100.0);
        }
        context.set_message(Message::new("job-between"));
        context.set_fraction(1.5);
        context.set_message(Message::new("job-last"));
        Ok(())
    })
    .progress(Message::new("job-test"), false);
    let x = control.add(UserBackground, job);
    events.ended(x);
    let of = events.of(x);
    let fractions: Vec<f64> = of
        .iter()
        .filter_map(|event| match event {
            JobEvent::Progress { fraction, .. } => Some(*fraction),
            _ => None,
        })
        .collect();
    let messages: Vec<&str> = of
        .iter()
        .filter_map(|event| match event {
            JobEvent::Message { message, .. } => Some(message.id.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(fractions.first(), Some(&0.0));
    assert_eq!(fractions.last(), Some(&1.0));
    assert!(fractions.len() <= 5, "{fractions:?}");
    assert_eq!(messages.first(), Some(&"job-first"));
    assert_eq!(messages.last(), Some(&"job-last"));
    assert!(messages.len() <= 3, "{messages:?}");
    assert!(matches!(
        of.last(),
        Some(JobEvent::Ended {
            state: State::Finished,
            failure: None,
            ..
        })
    ));
}

/// Shutting down: a running job that can be cancelled is, one that cannot finishes,
/// one queued is discarded unrun, and a job added afterwards is discarded too, its queue
/// reported (difference J16).
#[test]
fn shutdown_cancels_what_can_be_cancelled_and_finishes_the_rest() {
    let (control, mut events) = control(2);
    let (tell, mut heard) = told();
    let (_never, wait) = mpsc::channel();
    let a = control.add(
        UserBackground,
        cancellable(counting("a", 1, Some(wait), &tell), "a"),
    );
    let (go, wait) = mpsc::channel();
    let b = control.add(
        UserForeground,
        counting("b", 1, Some(wait), &tell).progress(Message::new("job-test"), false),
    );
    heard.until(|told| *told == Told::Started("a"));
    heard.until(|told| *told == Told::Started("b"));
    let c = control.add(
        UserBackground,
        cancellable(counting("c", 1, None, &tell), "c"),
    );

    thread::scope(|scope| {
        let stopping = scope.spawn(|| control.shutdown());
        events.ended(a);
        events.ended(c);
        assert_eq!(control.state(b), Some(State::Running));
        go.send(()).unwrap();
        stopping.join().unwrap();
    });
    events.ended(b);
    let end = |job| events.of(job).last().cloned();
    assert_eq!(
        end(a),
        Some(JobEvent::Ended {
            job: a,
            state: State::Cancelled,
            failure: None,
        })
    );
    assert_eq!(
        end(b),
        Some(JobEvent::Ended {
            job: b,
            state: State::Finished,
            failure: None,
        })
    );
    assert_eq!(
        end(c),
        Some(JobEvent::Ended {
            job: c,
            state: State::Discarded,
            failure: None,
        })
    );
    heard.until(|told| *told == Told::Stopped("b", Ok(1)));
    let mut started = heard.started();
    started.sort_unstable();
    assert_eq!(started, ["a", "b"], "c never ran");
    assert_eq!(control.pending(), 0);

    let late = control.add(
        UserBackground,
        cancellable(counting("late", 1, None, &tell), "late"),
    );
    assert_eq!(control.state(late), None);
    events.ended(late);
    assert_eq!(
        events.of(late),
        [
            JobEvent::Added {
                job: late,
                kind: "count",
                queue: Some(UserBackground),
                message: Message::new("job-test").with("label", "late"),
                cancellable: true,
            },
            JobEvent::Ended {
                job: late,
                state: State::Discarded,
                failure: None,
            },
        ]
    );
    assert_eq!(heard.receiver.try_iter().count(), 0, "late never ran");
}

/// A body's failure and a panic both end the job finished with the failure, never
/// cancelled; the worker carries on.
#[test]
fn a_failure_ends_a_job_failed_not_cancelled() {
    let (control, mut events) = control(1);
    let failing = control.add(
        UserBackground,
        Job::new("count", "failing", |_| {
            Err(Message::new("job-test-failed").with("file", "IMG_0001.ARW"))
        })
        .progress(Message::new("job-test"), true),
    );
    let panicking = control.add(
        UserBackground,
        Job::new("count", "panicking", |_| panic!("counted too far"))
            .progress(Message::new("job-test"), true),
    );
    let (tell, mut heard) = told();
    control.add(UserBackground, counting("after", 1, None, &tell));
    events.ended(failing);
    events.ended(panicking);
    heard.until(|told| *told == Told::Stopped("after", Ok(1)));

    assert_eq!(
        events.of(failing).last(),
        Some(&JobEvent::Ended {
            job: failing,
            state: State::Finished,
            failure: Some(Message::new("job-test-failed").with("file", "IMG_0001.ARW")),
        })
    );
    assert_eq!(
        events.of(panicking).last(),
        Some(&JobEvent::Ended {
            job: panicking,
            state: State::Finished,
            failure: Some(Message::new("job-panicked").with("reason", "counted too far")),
        })
    );
}

/// `run_now` runs the job on the caller's thread, shown with no queue, and hands back
/// how it ended.
#[test]
fn run_now_runs_on_the_callers_thread() {
    let (control, mut events) = control(1);
    let caller = thread::current().id();
    let (sender, ran_on) = mpsc::channel();
    let ended = control.run_now(
        Job::new("count", "now", move |context| {
            let _ = sender.send(thread::current().id());
            context.set_fraction(1.0);
            Ok(())
        })
        .progress(Message::new("job-test"), false),
    );
    assert_eq!(ended, Ok(State::Finished));
    assert_eq!(ran_on.try_recv(), Ok(caller));
    events.until(|event| matches!(event, JobEvent::Ended { .. }));
    let job = id(&events.seen[0]);
    assert_eq!(
        events.of(job),
        [
            JobEvent::Added {
                job,
                kind: "count",
                queue: None,
                message: Message::new("job-test"),
                cancellable: false,
            },
            JobEvent::State {
                job,
                state: State::Running,
            },
            JobEvent::Progress { job, fraction: 1.0 },
            JobEvent::Ended {
                job,
                state: State::Finished,
                failure: None,
            },
        ]
    );
    assert_eq!(
        control.run_now(Job::new("count", "failing now", |_| Err(Message::new(
            "job-test-failed"
        )))),
        Err(Message::new("job-test-failed"))
    );
    assert_eq!(control.pending(), 0);
}

/// A `SystemForeground` job cancelled while queued leaves the stack, so an equal one
/// added after it is queued and runs, where darktable would move the cancelled one up
/// and discard the new one (difference J14).
#[test]
fn an_equal_job_added_after_a_cancel_runs() {
    let (control, _) = control(1);
    let (tell, mut heard) = told();
    let (job, go) = gate("gate", &tell);
    control.add(UserForeground, job);
    heard.until(|told| *told == Told::Started("gate"));
    let first = control.add(
        SystemForeground,
        counting("first", 1, None, &tell).dedup(*b"photo"),
    );
    assert!(control.cancel(first));
    let second = control.add(
        SystemForeground,
        counting("second", 1, None, &tell).dedup(*b"photo"),
    );
    assert_eq!(control.state(second), Some(State::Queued));
    go.send(()).unwrap();
    heard.until(|told| *told == Told::Stopped("second", Ok(1)));
    assert_eq!(heard.started(), ["gate", "second"]);
}

/// An observer that panics loses that event only: the panic is logged, never blamed on
/// the job that sent it, and jobs, exports among them, still run on every worker.
#[test]
fn a_panicking_observer_loses_one_event_and_the_control_runs_on() {
    let (sender, receiver) = mpsc::channel();
    let control = Control::with_workers(1, move |event| {
        if matches!(event, JobEvent::Progress { .. }) {
            panic!("the observer fell over");
        }
        let _ = sender.send(event);
    });
    let mut events = Heard::new(receiver);
    let (tell, mut heard) = told();
    let x = control.add(UserExport, cancellable(counting("x", 2, None, &tell), "x"));
    events.ended(x);
    assert!(
        events
            .of(x)
            .iter()
            .all(|event| !matches!(event, JobEvent::Progress { .. })),
        "{:?}",
        events.of(x)
    );
    assert_eq!(
        events.of(x).last(),
        Some(&JobEvent::Ended {
            job: x,
            state: State::Finished,
            failure: None,
        })
    );
    let y = control.add(UserExport, cancellable(counting("y", 1, None, &tell), "y"));
    events.ended(y);
    heard.until(|told| *told == Told::Stopped("y", Ok(1)));
    assert_eq!(heard.started(), ["x", "y"]);
    assert_eq!(control.pending(), 0);
}

/// An observer that calls its control would wait for its own lock forever; the call
/// panics instead, and the control runs on.
#[test]
fn an_observer_calling_its_control_panics_rather_than_hangs() {
    let shared: Arc<OnceLock<Weak<Control>>> = Arc::new(OnceLock::new());
    let (sender, receiver) = mpsc::channel();
    let cell = shared.clone();
    let control = Arc::new(Control::with_workers(1, move |event| {
        if let Some(control) = cell.get().and_then(Weak::upgrade) {
            let called = panic::catch_unwind(AssertUnwindSafe(|| control.pending()));
            let _ = sender.send((event, called.is_err()));
        }
    }));
    shared.set(Arc::downgrade(&control)).unwrap();
    let (tell, mut heard) = told();
    let x = control.add(
        UserBackground,
        cancellable(counting("x", 1, None, &tell), "x"),
    );
    heard.until(|told| *told == Told::Stopped("x", Ok(1)));
    let mut calls = Heard::new(receiver);
    calls.until(|(event, _)| matches!(event, JobEvent::Ended { job, .. } if *job == x));
    assert!(
        calls.seen.iter().all(|(_, panicked)| *panicked),
        "{:?}",
        calls.seen
    );
    assert_eq!(control.pending(), 0);
}
