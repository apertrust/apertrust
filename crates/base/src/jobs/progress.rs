// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2014-2025  darktable developers (ported from src/control/progress.c)
// Copyright (C) 2010-2026  darktable developers (ported from src/control/jobs/control_jobs.c)
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

//! A job's progress, darktable's `src/control/progress.c` (decision 57), and everything
//! its row in a client shows (decision 72): its title, a [`Message`], darktable's message;
//! its [`Progress`], items done of an optional total, in place of darktable's fraction;
//! a detail line and its [`Destination`]; and as it ends, its outcome, its [`Failure`]
//! and its [`Offer`]s. They go to the observer as [`JobEvent`]s, darktable's proxy of
//! five callbacks (`added`, `updated`, `message_updated`, `cancellable`, `destroyed`) as
//! plain events. The progress, title, detail and destination each go out at most every
//! [`INTERVAL`] ([`Throttle`]): `_update_progress` in `src/control/jobs/control_jobs.c`,
//! with its `PROGRESS_UPDATE_INTERVAL`, which each of darktable's jobs calls itself and
//! the API keeps here for every caller (difference J7).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use super::{JobId, Queue, State};

/// `PROGRESS_UPDATE_INTERVAL` of darktable's `src/control/jobs/control_jobs.c`, 0.5 s: a
/// job's progress, title, detail or destination goes out at most this often, each on its
/// own, the first and the last always.
pub const INTERVAL: Duration = Duration::from_millis(500);

/// A message for the person, as an id and named arguments that the client translates
/// with Fluent (`job-…` ids, decision 36), never English from base (difference J4).
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub id: String,
    pub arguments: Vec<(String, Argument)>,
}

impl Message {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            arguments: Vec::new(),
        }
    }

    /// The message with an argument more, `{ $name }` in its Fluent text.
    pub fn with(mut self, name: impl Into<String>, value: impl Into<Argument>) -> Self {
        self.arguments.push((name.into(), value.into()));
        self
    }

    /// An argument by its name.
    pub fn argument(&self, name: &str) -> Option<&Argument> {
        self.arguments
            .iter()
            .find(|(own, _)| own == name)
            .map(|(_, value)| value)
    }
}

/// A [`Message`]'s argument: a number, which the client formats in its language, with at
/// least so many decimals where it says, text as it is, or a path, which the client shows
/// as its platform does.
#[derive(Clone, Debug, PartialEq)]
pub enum Argument {
    Number(f64),
    /// A number written with at least `at_least` decimals, as a size's "1.0 MB".
    Decimals {
        value: f64,
        at_least: u8,
    },
    Text(String),
    Path(PathBuf),
}

macro_rules! numbers {
    ($($number:ty),*) => {$(
        impl From<$number> for Argument {
            fn from(value: $number) -> Self {
                Argument::Number(value as f64)
            }
        }
    )*};
}

numbers!(f64, f32, i32, i64, u32, u64, usize);

impl From<&str> for Argument {
    fn from(value: &str) -> Self {
        Argument::Text(value.to_owned())
    }
}

impl From<String> for Argument {
    fn from(value: String) -> Self {
        Argument::Text(value)
    }
}

impl From<&Path> for Argument {
    fn from(value: &Path) -> Self {
        Argument::Path(value.to_owned())
    }
}

impl From<PathBuf> for Argument {
    fn from(value: PathBuf) -> Self {
        Argument::Path(value)
    }
}

/// A job's progress: `done` items of `total`, where the job knows it, in place of
/// darktable's fraction (difference J18). A bar fills at `done / total`, `done` clamped to
/// `total`, and is indeterminate with no total.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    pub done: u64,
    pub total: Option<u64>,
}

impl Progress {
    /// How full its bar is, 0 to 1, `done` clamped to `total`; `None`, indeterminate, with
    /// no total. A total of 0 is an empty bar.
    pub fn fraction(self) -> Option<f64> {
        let total = self.total?;
        Some(self.done.min(total) as f64 / total.max(1) as f64)
    }
}

/// Where a job's results go: the row's "To …" line, a file's folder, and what Show in
/// Finder reveals, a file selected in its folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Destination {
    Folder(PathBuf),
    File(PathBuf),
}

/// Why a job failed (difference J10): `cause`, a code a client words with Fluent as
/// `job-cause-<cause>`; the `item` it failed on, such as a photo; the `file` it could not
/// write; and `reason`, the error in plain words, for the log and a client with no words
/// for the code. A panic in a job's body is cause [`Failure::PANICKED`], its text the
/// reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub cause: String,
    pub item: Option<PathBuf>,
    pub file: Option<PathBuf>,
    pub reason: String,
}

impl Failure {
    /// The cause of a panic in a job's body.
    pub const PANICKED: &str = "panicked";

    /// A failure of `cause`, with `reason` in plain words, naming no item or file.
    pub fn new(cause: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            cause: cause.into(),
            item: None,
            file: None,
            reason: reason.into(),
        }
    }

    /// The failure, naming the item it failed on.
    pub fn item(mut self, item: impl Into<PathBuf>) -> Self {
        self.item = Some(item.into());
        self
    }

    /// The failure, naming the file it could not write.
    pub fn file(mut self, file: impl Into<PathBuf>) -> Self {
        self.file = Some(file.into());
        self
    }
}

/// What a job can do once it has ended, which its `Ended` carries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Offer {
    /// Try it again, on `items` items: for an export, the photos that failed and those not
    /// reached.
    Retry { items: u64 },
}

/// What the control tells its observer of a job with a progress, darktable's proxy as
/// plain events (difference J3): everything its row shows (decision 72). A job without a
/// progress, as darktable's, is not shown and sends none.
#[derive(Clone, Debug, PartialEq)]
pub enum JobEvent {
    /// The job was added, darktable's `added`, so it shows while it waits: its kind, its
    /// queue (`None` for [`Control::run_now`](super::Control::run_now)), its title,
    /// darktable's message, and whether it can be cancelled, darktable's `cancellable`;
    /// its destination and detail where it gives them; its progress as it starts, `done`
    /// 0; and the ended job it tries again.
    Added {
        job: JobId,
        kind: &'static str,
        queue: Option<Queue>,
        title: Message,
        cancellable: bool,
        destination: Option<Destination>,
        detail: Option<Message>,
        progress: Progress,
        retries: Option<JobId>,
    },
    /// Its progress, darktable's `updated`.
    Progress {
        job: JobId,
        done: u64,
        total: Option<u64>,
    },
    /// Its title, darktable's `message_updated`.
    Title { job: JobId, title: Message },
    /// The one line under its bar, `None` clearing it.
    Detail { job: JobId, detail: Option<Message> },
    /// Where its results go now.
    Destination {
        job: JobId,
        destination: Destination,
    },
    /// Its state while it lives: [`State::Queued`], [`State::Running`], and
    /// [`State::Cancelled`] when a running job is asked to stop.
    State { job: JobId, state: State },
    /// It ended and is gone, darktable's `destroyed`: [`State::Finished`],
    /// [`State::Cancelled`] or [`State::Discarded`]; the outcome its body set, the ended
    /// row's title; the failure that ended it, set exactly when its body failed or
    /// panicked (difference J10); the wall clock as the control ended it; and what it
    /// offers once ended.
    Ended {
        job: JobId,
        state: State,
        outcome: Option<Message>,
        failure: Option<Failure>,
        at: SystemTime,
        offers: Vec<Offer>,
    },
}

impl JobEvent {
    /// The job it is about.
    pub fn job(&self) -> JobId {
        match *self {
            JobEvent::Added { job, .. }
            | JobEvent::Progress { job, .. }
            | JobEvent::Title { job, .. }
            | JobEvent::Detail { job, .. }
            | JobEvent::Destination { job, .. }
            | JobEvent::State { job, .. }
            | JobEvent::Ended { job, .. } => job,
        }
    }
}

/// darktable's `_update_progress` throttle (`control_jobs.c`) for one value: the first offered goes out,
/// then one only once [`INTERVAL`] has passed since the last that went out, strictly,
/// as darktable's `curr_time > *prev_time + PROGRESS_UPDATE_INTERVAL`; what is held
/// meanwhile goes out once it is due, or as the job ends ([`Throttle::flush`]).
#[derive(Debug)]
pub struct Throttle<T> {
    last: Option<Instant>,
    held: Option<T>,
}

impl<T> Default for Throttle<T> {
    fn default() -> Self {
        Self {
            last: None,
            held: None,
        }
    }
}

impl<T> Throttle<T> {
    /// A new value at `now`: returned to go out now, or held, replacing what was held.
    pub fn offer(&mut self, value: T, now: Instant) -> Option<T> {
        if self.is_due(now) {
            self.last = Some(now);
            self.held = None;
            Some(value)
        } else {
            self.held = Some(value);
            None
        }
    }

    /// What is held, once it may go out at `now`.
    pub fn due(&mut self, now: Instant) -> Option<T> {
        if self.held.is_some() && self.is_due(now) {
            self.last = Some(now);
            self.held.take()
        } else {
            None
        }
    }

    /// What is held, whenever: the last value as the job ends.
    pub fn flush(&mut self) -> Option<T> {
        self.held.take()
    }

    fn is_due(&self, now: Instant) -> bool {
        self.last.is_none_or(|last| now > last + INTERVAL)
    }
}

/// The four values of a job's row that go through the throttle, each with a [`Throttle`]
/// of its own: its progress, title, detail and destination. Each call takes the time, so
/// the tests run its table on a clock of their own.
#[derive(Debug)]
pub(super) struct Throttles {
    job: JobId,
    progress: Throttle<Progress>,
    title: Throttle<Message>,
    detail: Throttle<Option<Message>>,
    destination: Throttle<Destination>,
}

impl Throttles {
    pub(super) fn new(job: JobId) -> Self {
        Self {
            job,
            progress: Throttle::default(),
            title: Throttle::default(),
            detail: Throttle::default(),
            destination: Throttle::default(),
        }
    }

    /// A new progress at `now`: its event, to go out now, or `None`, held.
    pub(super) fn progress(&mut self, progress: Progress, now: Instant) -> Option<JobEvent> {
        let progress = self.progress.offer(progress, now)?;
        Some(self.progress_event(progress))
    }

    /// A new title at `now`: its event, to go out now, or `None`, held.
    pub(super) fn title(&mut self, title: Message, now: Instant) -> Option<JobEvent> {
        let title = self.title.offer(title, now)?;
        Some(JobEvent::Title {
            job: self.job,
            title,
        })
    }

    /// A new detail at `now`: its event, to go out now, or `None`, held.
    pub(super) fn detail(&mut self, detail: Option<Message>, now: Instant) -> Option<JobEvent> {
        let detail = self.detail.offer(detail, now)?;
        Some(JobEvent::Detail {
            job: self.job,
            detail,
        })
    }

    /// A new destination at `now`: its event, to go out now, or `None`, held.
    pub(super) fn destination(
        &mut self,
        destination: Destination,
        now: Instant,
    ) -> Option<JobEvent> {
        let destination = self.destination.offer(destination, now)?;
        Some(JobEvent::Destination {
            job: self.job,
            destination,
        })
    }

    /// What is held and may go out at `now`, or with no time, whatever is held, as the job
    /// ends: progress, title, detail and destination, in that order.
    pub(super) fn held(&mut self, now: Option<Instant>) -> Vec<JobEvent> {
        fn take<T>(throttle: &mut Throttle<T>, now: Option<Instant>) -> Option<T> {
            match now {
                Some(now) => throttle.due(now),
                None => throttle.flush(),
            }
        }
        let job = self.job;
        let progress = take(&mut self.progress, now).map(|p| self.progress_event(p));
        let title = take(&mut self.title, now).map(|title| JobEvent::Title { job, title });
        let detail = take(&mut self.detail, now).map(|detail| JobEvent::Detail { job, detail });
        let destination = take(&mut self.destination, now)
            .map(|destination| JobEvent::Destination { job, destination });
        [progress, title, detail, destination]
            .into_iter()
            .flatten()
            .collect()
    }

    fn progress_event(&self, progress: Progress) -> JobEvent {
        JobEvent::Progress {
            job: self.job,
            done: progress.done,
            total: progress.total,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JOB: JobId = JobId(7);

    /// How [`Throttles`] takes one of its four values at a time.
    type Set<T> = fn(&mut Throttles, T, Instant) -> Option<JobEvent>;

    /// The throttle's table for one value, `values` its seven, `v0` to `v6`, on a clock of
    /// its own, worked through `_update_progress`:
    ///
    /// | at, s | offered | out | why                                  |
    /// | ----- | ------- | --- | ------------------------------------ |
    /// | 0.0   | v0      | v0  | the first always                     |
    /// | 0.1   | v1      |     | held                                 |
    /// | 0.4   | v2      |     | held, replacing v1                   |
    /// | 0.5   | v3      |     | 0.5 is not past 0.0 + 0.5: held      |
    /// | 0.6   | v4      | v4  | past: out, nothing held              |
    /// | 0.6   | (due)   |     | nothing held                         |
    /// | 0.7   | v5      |     | held                                 |
    /// | 1.0   | (due)   |     | 1.0 is not past 0.6 + 0.5            |
    /// | 1.2   | (due)   | v5  | past: the held value goes out        |
    /// | 1.25  | (due)   |     | nothing held                         |
    /// | 1.3   | v6      |     | held                                 |
    /// | end   | (flush) | v6  | the last always                      |
    /// | end   | (flush) |     | nothing more                         |
    fn table<T: Clone>(values: [T; 7], set: Set<T>, event: fn(JobId, T) -> JobEvent) {
        let start = Instant::now();
        let at = |seconds: f64| start + Duration::from_secs_f64(seconds);
        let out = |value: &T| Some(event(JOB, value.clone()));
        let [v0, v1, v2, v3, v4, v5, v6] = values;
        let mut throttles = Throttles::new(JOB);
        assert_eq!(set(&mut throttles, v0.clone(), at(0.0)), out(&v0));
        assert_eq!(set(&mut throttles, v1, at(0.1)), None);
        assert_eq!(set(&mut throttles, v2, at(0.4)), None);
        assert_eq!(set(&mut throttles, v3, at(0.5)), None);
        assert_eq!(set(&mut throttles, v4.clone(), at(0.6)), out(&v4));
        assert_eq!(throttles.held(Some(at(0.6))), []);
        assert_eq!(set(&mut throttles, v5.clone(), at(0.7)), None);
        assert_eq!(throttles.held(Some(at(1.0))), []);
        assert_eq!(throttles.held(Some(at(1.2))), [event(JOB, v5)]);
        assert_eq!(throttles.held(Some(at(1.25))), []);
        assert_eq!(set(&mut throttles, v6.clone(), at(1.3)), None);
        assert_eq!(throttles.held(None), [event(JOB, v6)]);
        assert_eq!(throttles.held(None), []);
    }

    #[test]
    fn the_progress_goes_out_at_most_every_half_second_the_first_and_last_always() {
        let values = [0, 1, 2, 3, 4, 5, 6].map(|done| Progress {
            done,
            total: (done < 6).then_some(12),
        });
        table(values, Throttles::progress, |job, progress| {
            JobEvent::Progress {
                job,
                done: progress.done,
                total: progress.total,
            }
        });
    }

    #[test]
    fn the_title_goes_out_at_most_every_half_second_the_first_and_last_always() {
        let values = [0, 1, 2, 3, 4, 5, 6].map(|n| Message::new("job-test").with("n", n));
        table(values, Throttles::title, |job, title| JobEvent::Title {
            job,
            title,
        });
    }

    /// The last is `None`, which clears the line, and goes out as the last always does.
    #[test]
    fn the_detail_goes_out_at_most_every_half_second_the_first_and_last_always() {
        let values =
            [0, 1, 2, 3, 4, 5, 6].map(|n| (n < 6).then(|| Message::new("job-test").with("n", n)));
        table(values, Throttles::detail, |job, detail| JobEvent::Detail {
            job,
            detail,
        });
    }

    #[test]
    fn the_destination_goes_out_at_most_every_half_second_the_first_and_last_always() {
        let values = [0, 1, 2, 3, 4, 5, 6].map(|n| match n {
            0 => Destination::Folder(PathBuf::from("/out")),
            n => Destination::File(PathBuf::from(format!("/out/{n}.jpg"))),
        });
        table(values, Throttles::destination, |job, destination| {
            JobEvent::Destination { job, destination }
        });
    }

    /// The four on one clock, each its own: one value going out holds back none of the
    /// others, and what is held goes out in the order progress, title, detail,
    /// destination.
    ///
    /// | at, s | offered               | out                    |
    /// | ----- | --------------------- | ---------------------- |
    /// | 0.0   | progress 1            | progress 1             |
    /// | 0.2   | title a               | title a, its first     |
    /// | 0.3   | progress 2            |                        |
    /// | 0.4   | detail x, destination | both, each its first   |
    /// | 0.6   | title b               |                        |
    /// | 0.6   | (due)                 | progress 2             |
    /// | 0.65  | detail y              |                        |
    /// | 0.8   | (due)                 | title b                |
    /// | end   | (flush)               | detail y               |
    #[test]
    fn each_value_has_its_own_throttle() {
        let start = Instant::now();
        let at = |seconds: f64| start + Duration::from_secs_f64(seconds);
        let progress = |done| Progress {
            done,
            total: Some(2),
        };
        let progress_event = |done| JobEvent::Progress {
            job: JOB,
            done,
            total: Some(2),
        };
        let title = |id| JobEvent::Title {
            job: JOB,
            title: Message::new(id),
        };
        let detail = |id| JobEvent::Detail {
            job: JOB,
            detail: Some(Message::new(id)),
        };
        let folder = Destination::Folder(PathBuf::from("/out"));
        let mut throttles = Throttles::new(JOB);
        assert_eq!(
            throttles.progress(progress(1), at(0.0)),
            Some(progress_event(1))
        );
        assert_eq!(
            throttles.title(Message::new("job-a"), at(0.2)),
            Some(title("job-a"))
        );
        assert_eq!(throttles.progress(progress(2), at(0.3)), None);
        assert_eq!(
            throttles.detail(Some(Message::new("job-x")), at(0.4)),
            Some(detail("job-x"))
        );
        assert_eq!(
            throttles.destination(folder.clone(), at(0.4)),
            Some(JobEvent::Destination {
                job: JOB,
                destination: folder,
            })
        );
        assert_eq!(throttles.title(Message::new("job-b"), at(0.6)), None);
        assert_eq!(throttles.held(Some(at(0.6))), [progress_event(2)]);
        assert_eq!(
            throttles.detail(Some(Message::new("job-y")), at(0.65)),
            None
        );
        assert_eq!(throttles.held(Some(at(0.8))), [title("job-b")]);
        assert_eq!(throttles.held(None), [detail("job-y")]);
    }

    #[test]
    fn a_progress_fills_its_bar_at_done_over_total() {
        let fraction = |done, total| Progress { done, total }.fraction();
        assert_eq!(fraction(0, Some(4)), Some(0.0));
        assert_eq!(fraction(1, Some(4)), Some(0.25));
        assert_eq!(fraction(4, Some(4)), Some(1.0));
        assert_eq!(fraction(5, Some(4)), Some(1.0), "done clamped to total");
        assert_eq!(fraction(0, Some(0)), Some(0.0), "nothing to do: empty");
        assert_eq!(fraction(3, None), None, "no total: indeterminate");
    }

    #[test]
    fn a_message_holds_its_arguments_by_name() {
        let message = Message::new("job-export-progress")
            .with("done", 3usize)
            .with("count", 12u32)
            .with("storage", "file on disk")
            .with("folder", Path::new("/photos"));
        assert_eq!(message.argument("done"), Some(&Argument::Number(3.0)));
        assert_eq!(message.argument("count"), Some(&Argument::Number(12.0)));
        assert_eq!(
            message.argument("storage"),
            Some(&Argument::Text("file on disk".to_owned()))
        );
        assert_eq!(
            message.argument("folder"),
            Some(&Argument::Path(PathBuf::from("/photos")))
        );
        assert_eq!(message.argument("missing"), None);
    }
}
