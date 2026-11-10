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

//! The session's one way out to its client (decision 59, rule 3): every thread of the
//! session, the engine thread, the edit writer and the job control's observer and its
//! callers, sends through an [`Outbox`], and the client drains what arrived with
//! [`Drain::events`].
//!
//! The outbox calls the client's wake only when the client may not have looked since the
//! last call: a flag set by the send that wakes and cleared as the client drains, so a
//! burst of events from any number of threads wakes it once, and the first send after a
//! drain wakes it again. No send blocks: the channel is unbounded, its volume bounded by
//! the work. Once the client has dropped its handle, nothing more is sent.
//!
//! Under `-d event`, each event sent, with its kind, numbers and topic, and each drain;
//! under `-d event -d verbose`, besides, how many of each kind were sent each second, to
//! catch a storm.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant};

use super::{Event, LibraryEvent, SessionError, Topic, Wake};
use crate::export_job::ExportStatus;
use crate::jobs::JobEvent;

/// How often `-d event -d verbose` counts the events sent by kind.
const STORM_INTERVAL: Duration = Duration::from_secs(1);

/// The sending side, cloned for every thread of the session.
#[derive(Clone)]
pub(crate) struct Outbox {
    sender: mpsc::Sender<Event>,
    shared: Arc<Shared>,
}

/// The client's side, which the session's handle holds.
pub(crate) struct Drain {
    receiver: mpsc::Receiver<Event>,
    shared: Arc<Shared>,
    /// Whether [`SessionError::Stopped`] has been reported.
    stopped: AtomicBool,
}

struct Shared {
    wake: Wake,
    /// Set by the send that wakes the client, cleared as it drains: while set, the client
    /// has been woken and has not looked since, so a send wakes nobody.
    woken: AtomicBool,
    /// Set as the client drops its handle: nothing more is sent.
    closed: AtomicBool,
    /// The events sent by kind since the last count, under `-d event -d verbose`.
    storm: Mutex<Storm>,
}

struct Storm {
    since: Instant,
    sent: BTreeMap<&'static str, u64>,
}

/// An outbox and its drain, the outbox waking the client with `wake`.
pub(crate) fn channel(wake: Wake) -> (Outbox, Drain) {
    let (sender, receiver) = mpsc::channel();
    let shared = Arc::new(Shared {
        wake,
        woken: AtomicBool::new(false),
        closed: AtomicBool::new(false),
        storm: Mutex::new(Storm {
            since: Instant::now(),
            sent: BTreeMap::new(),
        }),
    });
    let drain = Drain {
        receiver,
        shared: Arc::clone(&shared),
        stopped: AtomicBool::new(false),
    };
    (Outbox { sender, shared }, drain)
}

impl Outbox {
    /// An outbox nobody hears, for a test that runs a job's body without a session.
    #[cfg(test)]
    pub(crate) fn unheard() -> Outbox {
        channel(Arc::new(|| {})).0
    }

    /// Sends `event`, waking the client unless it has been woken and has not looked since.
    pub(crate) fn send(&self, event: Event) {
        self.send_all([event]);
    }

    /// Sends `events` in order, then wakes the client once, unless it has been woken and
    /// has not looked since: a batch's events are one burst.
    pub(crate) fn send_all(&self, events: impl IntoIterator<Item = Event>) {
        if self.shared.closed.load(Ordering::Acquire) {
            return;
        }
        let mut sent = false;
        for event in events {
            tracing::debug!(target: "event", "sent {}", Line(&event));
            self.shared.count(&event);
            // Only once the drain has dropped, after the session's end.
            if self.sender.send(event).is_err() {
                return;
            }
            sent = true;
        }
        // After the sends: a drain that clears the flag after this sees them, and one that
        // cleared it before has this wake it again.
        if sent && !self.shared.woken.swap(true, Ordering::AcqRel) {
            (self.shared.wake)();
        }
    }
}

impl Shared {
    /// Counts `event` under `-d event -d verbose`, writing the counts once a second has
    /// passed since the last.
    fn count(&self, event: &Event) {
        if !tracing::enabled!(target: "event", tracing::Level::TRACE) {
            return;
        }
        let mut storm = self.storm.lock().unwrap_or_else(PoisonError::into_inner);
        *storm.sent.entry(kind(event)).or_default() += 1;
        let elapsed = storm.since.elapsed();
        if elapsed >= STORM_INTERVAL {
            let counts: Vec<String> = storm
                .sent
                .iter()
                .map(|(kind, count)| format!("{kind} {count}"))
                .collect();
            let (seconds, counts) = (elapsed.as_secs_f64(), counts.join(", "));
            tracing::trace!(target: "event", "sent in {seconds:.1} s: {counts}");
            storm.sent.clear();
            storm.since = Instant::now();
        }
    }
}

impl Drain {
    /// The events that have arrived, oldest first; never waits. Received one at a time
    /// until there are none, so an event sent between a look and the next is taken by the
    /// next drain, never lost; once every outbox has gone, one [`SessionError::Stopped`].
    pub(crate) fn events(&self) -> Vec<Event> {
        // Cleared first: a send from now on wakes the client again, so none goes unseen.
        self.shared.woken.swap(false, Ordering::AcqRel);
        let mut events = Vec::new();
        loop {
            match self.receiver.try_recv() {
                Ok(event) => events.push(event),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    if !self.stopped.swap(true, Ordering::Relaxed) {
                        events.push(Event::Failed(SessionError::Stopped));
                    }
                    break;
                }
            }
        }
        if !events.is_empty() {
            tracing::debug!(target: "event", "drained {}: {}", events.len(), Kinds(&events));
        }
        events
    }

    /// Sends nothing more: the client is dropping its handle.
    pub(crate) fn close(&self) {
        self.shared.closed.store(true, Ordering::Release);
    }
}

/// An event's kind, as `-d event` counts it.
fn kind(event: &Event) -> &'static str {
    match event {
        Event::OpenPreview(_) => "OpenPreview",
        Event::Opened(_) => "Opened",
        Event::Values(_) => "Values",
        Event::Frame(_) => "Frame",
        Event::Scopes(_) => "Scopes",
        Event::Saved { .. } => "Saved",
        Event::Job { event, .. } => match event {
            JobEvent::Added { .. } => "Job Added",
            JobEvent::Progress { .. } => "Job Progress",
            JobEvent::Title { .. } => "Job Title",
            JobEvent::Detail { .. } => "Job Detail",
            JobEvent::Destination { .. } => "Job Destination",
            JobEvent::State { .. } => "Job State",
            JobEvent::Ended { .. } => "Job Ended",
        },
        Event::Export { status, .. } => match status {
            ExportStatus::Started { .. } => "Export Started",
            ExportStatus::Done { .. } => "Export Done",
            ExportStatus::Finished(_) => "Export Finished",
        },
        Event::ExportPreview { .. } => "ExportPreview",
        Event::Library(event) => match event {
            LibraryEvent::Opened { .. } => "Library Opened",
            LibraryEvent::Subfolders { .. } => "Library Subfolders",
            LibraryEvent::Listed { .. } => "Library Listed",
            LibraryEvent::Failed { .. } => "Library Failed",
            LibraryEvent::Taken { .. } => "Library Taken",
            LibraryEvent::Closed { .. } => "Library Closed",
            LibraryEvent::Thumbnail { .. } => "Library Thumbnail",
        },
        Event::Failed(_) => "Failed",
    }
}

/// An event as `-d event` writes it, for the session's tests.
#[cfg(test)]
pub(super) fn line(event: &Event) -> String {
    Line(event).to_string()
}

/// An event as `-d event` writes it: its kind, its numbers and its topic.
struct Line<'a>(&'a Event);

impl fmt::Display for Line<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let event = self.0;
        f.write_str(kind(event))?;
        match event {
            Event::OpenPreview(preview) => write!(f, ", open {}", preview.open)?,
            Event::Opened(photo) => write!(f, ", open {}", photo.open)?,
            Event::Values(values) if !values.live => {
                write!(f, ", through {}, not live", values.through)?
            }
            Event::Values(values) => write!(f, ", through {}", values.through)?,
            Event::Frame(frame) => write!(f, ", through {}", frame.through)?,
            Event::Scopes(reading) => write!(f, ", through {}", reading.through)?,
            Event::Saved { through, .. } | Event::ExportPreview { through, .. } => {
                write!(f, ", through {through}")?
            }
            Event::Job { event, command } => {
                write!(f, ", job {}", event.job())?;
                if let Some(command) = command {
                    write!(f, ", command {command}")?;
                }
            }
            Event::Export { job, .. } => write!(f, ", job {job}")?,
            Event::Library(event) => write!(f, ", open {}", event.open())?,
            Event::Failed(SessionError::Open { open, .. }) => write!(f, ", open {open}")?,
            Event::Failed(SessionError::Refused { command, .. }) => {
                write!(f, ", refused command {command}")?
            }
            Event::Failed(_) => {}
        }
        match event.topic() {
            Topic::Fact(_) => f.write_str(", a fact"),
            Topic::Notice => f.write_str(", a notice"),
        }
    }
}

/// The kinds of a drain's events, in order, a run of one kind counted.
struct Kinds<'a>(&'a [Event]);

impl fmt::Display for Kinds<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut kinds = self.0.iter().map(kind).peekable();
        let mut first = true;
        while let Some(kind) = kinds.next() {
            let mut count = 1;
            while kinds.next_if_eq(&kind).is_some() {
                count += 1;
            }
            if !first {
                f.write_str(", ")?;
            }
            first = false;
            f.write_str(kind)?;
            if count > 1 {
                write!(f, " ×{count}")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::AtomicUsize;

    use super::*;

    /// An outbox whose wakes are counted.
    fn counted() -> (Outbox, Drain, Arc<AtomicUsize>) {
        let wakes = Arc::new(AtomicUsize::new(0));
        let counting = Arc::clone(&wakes);
        let (outbox, drain) = channel(Arc::new(move || {
            counting.fetch_add(1, Ordering::SeqCst);
        }));
        (outbox, drain, wakes)
    }

    fn saved(producer: usize, through: u64) -> Event {
        Event::Saved {
            path: PathBuf::from(format!("{producer}.apxmp")),
            through,
        }
    }

    #[test]
    fn a_burst_wakes_the_client_once_and_a_send_after_a_drain_again() {
        let (outbox, drain, wakes) = counted();
        let writer = outbox.clone();
        outbox.send_all((1..=3).map(|n| saved(0, n)));
        writer.send(saved(1, 1));
        outbox.send(saved(0, 4));
        assert_eq!(wakes.load(Ordering::SeqCst), 1);
        assert_eq!(drain.events().len(), 5);
        // Nothing sent wakes nobody.
        outbox.send_all([]);
        assert_eq!(wakes.load(Ordering::SeqCst), 1);
        writer.send(saved(1, 2));
        outbox.send(saved(0, 5));
        assert_eq!(wakes.load(Ordering::SeqCst), 2);
        // A drain that finds nothing still lets the next send wake.
        assert_eq!(drain.events().len(), 2);
        assert!(drain.events().is_empty());
        outbox.send(saved(0, 6));
        assert_eq!(wakes.load(Ordering::SeqCst), 3);
    }

    /// Three producers send while a fourth thread drains whenever it is woken: every
    /// event arrives once, each producer's in order, and no burst goes without a wake.
    #[test]
    fn no_event_is_lost_while_three_threads_send_and_a_fourth_drains() {
        const EACH: u64 = 20_000;
        let (woken, wakes) = mpsc::channel();
        let (outbox, drain) = channel(Arc::new(move || {
            let _ = woken.send(());
        }));
        let producers: Vec<_> = (0..3)
            .map(|producer| {
                let outbox = outbox.clone();
                std::thread::spawn(move || {
                    // Bursts of one to three, as the engine's batches and the writer's.
                    let mut through = 0;
                    while through < EACH {
                        let end = (through + 1 + through % 3).min(EACH);
                        outbox.send_all((through + 1..=end).map(|n| saved(producer, n)));
                        through = end;
                    }
                })
            })
            .collect();
        drop(outbox);
        let drainer = std::thread::spawn(move || {
            let mut last = [0u64; 3];
            let mut heard = 0;
            while heard < 3 * EACH {
                // A wake lost would leave events unheard here.
                wakes
                    .recv_timeout(Duration::from_secs(60))
                    .expect("every burst wakes the client");
                for event in drain.events() {
                    match event {
                        Event::Saved { path, through } => {
                            let producer: usize =
                                path.file_stem().unwrap().to_str().unwrap().parse().unwrap();
                            assert_eq!(through, last[producer] + 1, "producer {producer}");
                            last[producer] = through;
                            heard += 1;
                        }
                        // The producers may have gone by the last drain.
                        Event::Failed(SessionError::Stopped) => {}
                        other => panic!("{other:?}"),
                    }
                }
            }
            (heard, last)
        });
        for producer in producers {
            producer.join().unwrap();
        }
        let (heard, last) = drainer.join().unwrap();
        assert_eq!(heard, 3 * EACH);
        assert_eq!(last, [EACH; 3]);
    }

    #[test]
    fn nothing_is_sent_once_the_client_drops_its_handle() {
        let (outbox, drain, wakes) = counted();
        drain.close();
        outbox.send(saved(0, 1));
        assert_eq!(wakes.load(Ordering::SeqCst), 0);
        drop(outbox);
        assert!(matches!(
            &drain.events()[..],
            [Event::Failed(SessionError::Stopped)]
        ));
    }

    #[test]
    fn a_drain_s_kinds_count_runs() {
        let events = [
            saved(0, 1),
            saved(0, 2),
            Event::Failed(SessionError::Stopped),
        ];
        assert_eq!(Kinds(&events).to_string(), "Saved ×2, Failed");
        assert_eq!(Line(&saved(0, 3)).to_string(), "Saved, through 3, a notice");
    }
}
