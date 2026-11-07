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

//! A job's progress, darktable's `src/control/progress.c` (decision 57): a fraction
//! clamped to 0 to 1, a [`Message`] and whether it can be cancelled, reported as
//! [`JobEvent`]s, darktable's proxy of five callbacks (`added`, `updated`,
//! `message_updated`, `cancellable`, `destroyed`) as plain events, at most every
//! [`INTERVAL`]. The throttle, [`Throttle`], is `_update_progress` in
//! `src/control/jobs/control_jobs.c`, with its `PROGRESS_UPDATE_INTERVAL`, which each of
//! darktable's jobs calls itself and the API keeps here for every caller (difference
//! J7).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::{JobId, Queue, State};

/// `PROGRESS_UPDATE_INTERVAL` of darktable's `src/control/jobs/control_jobs.c`, 0.5 s: a
/// fraction or a message goes out at most this often, the first and the last always.
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

/// A [`Message`]'s argument: a number, which the client formats in its language, text
/// as it is, or a path, which the client shows as its platform does.
#[derive(Clone, Debug, PartialEq)]
pub enum Argument {
    Number(f64),
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

/// What the control tells its observer of a job with a progress, darktable's proxy as
/// plain events (difference J3). A job without a progress, as darktable's, is not shown
/// and sends none.
#[derive(Clone, Debug, PartialEq)]
pub enum JobEvent {
    /// The job was added, darktable's `added`, so it shows while it waits: its kind, its
    /// queue (`None` for [`Control::run_now`](super::Control::run_now)), its message and
    /// whether it can be cancelled, darktable's `cancellable`.
    Added {
        job: JobId,
        kind: &'static str,
        queue: Option<Queue>,
        message: Message,
        cancellable: bool,
    },
    /// Its fraction, 0 to 1, darktable's `updated`.
    Progress { job: JobId, fraction: f64 },
    /// Its message, darktable's `message_updated`.
    Message { job: JobId, message: Message },
    /// Its state while it lives: [`State::Queued`], [`State::Running`], and
    /// [`State::Cancelled`] when a running job is asked to stop.
    State { job: JobId, state: State },
    /// It ended and is gone, darktable's `destroyed`: [`State::Finished`],
    /// [`State::Cancelled`] or [`State::Discarded`], with the failure that ended it, in
    /// plain words, if its body failed or panicked (difference J10).
    Ended {
        job: JobId,
        state: State,
        failure: Option<Message>,
    },
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The throttle on a clock of its own, worked through `_update_progress`:
    ///
    /// | at, s | offered | out  | why                                  |
    /// | ----- | ------- | ---- | ------------------------------------ |
    /// | 0.0   | 0.1     | 0.1  | the first always                     |
    /// | 0.1   | 0.2     |      | held                                 |
    /// | 0.4   | 0.3     |      | held, replacing 0.2                  |
    /// | 0.5   | 0.4     |      | 0.5 is not past 0.0 + 0.5: held      |
    /// | 0.6   | 0.5     | 0.5  | past: out, nothing held              |
    /// | 0.7   | 0.6     |      | held                                 |
    /// | 1.0   | (due)   |      | 1.0 is not past 0.6 + 0.5            |
    /// | 1.2   | (due)   | 0.6  | past: the held value goes out        |
    /// | 1.3   | 0.7     |      | held                                 |
    /// | end   | (flush) | 0.7  | the last always                      |
    /// | end   | (flush) |      | nothing more                         |
    #[test]
    fn values_go_out_at_most_every_half_second_the_first_and_last_always() {
        let start = Instant::now();
        let at = |seconds: f64| start + Duration::from_secs_f64(seconds);
        let mut throttle = Throttle::default();
        assert_eq!(throttle.offer(0.1, at(0.0)), Some(0.1));
        assert_eq!(throttle.offer(0.2, at(0.1)), None);
        assert_eq!(throttle.offer(0.3, at(0.4)), None);
        assert_eq!(throttle.offer(0.4, at(0.5)), None);
        assert_eq!(throttle.offer(0.5, at(0.6)), Some(0.5));
        assert_eq!(throttle.due(at(0.6)), None);
        assert_eq!(throttle.offer(0.6, at(0.7)), None);
        assert_eq!(throttle.due(at(1.0)), None);
        assert_eq!(throttle.due(at(1.2)), Some(0.6));
        assert_eq!(throttle.due(at(1.25)), None);
        assert_eq!(throttle.offer(0.7, at(1.3)), None);
        assert_eq!(throttle.flush(), Some(0.7));
        assert_eq!(throttle.flush(), None);
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
