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

//! The session's log, kept in memory for the app's log window (`docs/gui.md`, The log
//! window) while the `log.keep` setting is on.
//!
//! It is a second layer beside stderr's, which [`init`](super::init) sets up with a filter
//! that is off: off, the filter's level hint is `OFF` and every callsite's interest is
//! `never`, so tracing's level check still stops each `debug!` and `trace!` before it is
//! built, as with stderr's layer alone, and the layer is handed nothing; nothing is kept.
//! [`Kept::keep`] switches the filter through `tracing_subscriber`'s `reload`, which
//! rebuilds tracing's interest cache, so the change applies from the next line.
//!
//! On, it keeps the lines exactly as the terminal writes them ([`Shared::write`]: the
//! seconds, the domain, the line, other crates' condensed and a repeated line written once
//! with how often), for the domains `log.domains` names and those `-d` turned on, warnings
//! and errors always, the detail `verbose` adds only where it is chosen. At most
//! [`CAPACITY`] lines: past that the oldest go, and are counted.

use std::collections::VecDeque;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::Waker;
use std::time::Instant;

use tracing::level_filters::LevelFilter;
use tracing::subscriber::Interest;
use tracing::{Dispatch, Event, Level, Metadata, Subscriber};
use tracing_log::NormalizeEvent;
use tracing_subscriber::fmt::FormatFields;
use tracing_subscriber::fmt::format::{DefaultFields, Writer};
use tracing_subscriber::layer::{Context, Filter, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{Layer, reload};

use super::{Domain, Error, Last, Shared, Switches};

/// The most lines kept: past them the oldest go.
pub const CAPACITY: usize = 20_000;

/// What a line says of how it was written: its level, as the log window colours and
/// filters it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    /// A domain's line, `debug!`'s, or a line of the banner.
    Debug,
    /// The detail `verbose` adds: `trace!`'s lines, and wgpu's own.
    Detail,
}

impl Severity {
    fn of(level: Level) -> Severity {
        match level {
            Level::ERROR => Severity::Error,
            Level::WARN => Severity::Warning,
            Level::DEBUG => Severity::Debug,
            _ => Severity::Detail,
        }
    }
}

/// One line kept, exactly as the terminal writes it, without its newline.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    text: String,
    /// Where the seconds end and the message starts in `text`; both 0 for a line with
    /// neither, the banner's or a message's own second line.
    seconds: usize,
    message: usize,
    /// The domain it belongs to: a `-d` domain's name, or the crate that wrote it; that of
    /// the line it continues for a message's second line; empty for the banner's.
    domain: String,
    severity: Severity,
}

impl Line {
    /// `raw` as a line kept: one with its seconds and domain, its severity by its level,
    /// `own`, where it is the event's own, else by its label or as `before`, the line it
    /// follows; or a second line of the message of `before`.
    fn read(raw: &str, before: Option<&Line>, own: Option<Severity>) -> Line {
        let inherited = before.map_or(Severity::Debug, |line| line.severity);
        match prefix(raw) {
            Some((seconds, message, domain)) => {
                let said = &raw[message..];
                let labelled = if said.starts_with("error: ") {
                    Some(Severity::Error)
                } else if said.starts_with("warning: ") {
                    Some(Severity::Warning)
                } else {
                    None
                };
                Line {
                    text: raw.to_owned(),
                    seconds,
                    message,
                    domain: domain.to_owned(),
                    severity: own.or(labelled).unwrap_or(inherited),
                }
            }
            None => Line {
                text: raw.to_owned(),
                seconds: 0,
                message: 0,
                domain: before.map_or_else(String::new, |line| line.domain.clone()),
                severity: inherited,
            },
        }
    }

    /// A line of the banner: no seconds, no domain.
    fn plain(raw: &str) -> Line {
        Line {
            text: raw.to_owned(),
            seconds: 0,
            message: 0,
            domain: String::new(),
            severity: Severity::Debug,
        }
    }

    /// The whole line, as the terminal writes it.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The seconds since start, `0.4433`; empty for a line without.
    pub fn seconds(&self) -> &str {
        self.text[..self.seconds].trim_start()
    }

    /// The domain as the line writes it, in its brackets, `[raw]`; empty for a line
    /// without.
    pub fn bracket(&self) -> &str {
        if self.message == 0 {
            return "";
        }
        self.text[self.seconds..self.message].trim()
    }

    /// What the line says after its domain, its label included (`warning: …`).
    pub fn message(&self) -> &str {
        &self.text[self.message..]
    }

    /// The domain it belongs to; empty for the banner's lines.
    pub fn domain(&self) -> &str {
        &self.domain
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }
}

/// Where a line's seconds end, where its message starts, and its domain, for a line
/// written with them: `    0.4433 [raw] decoded …`.
fn prefix(raw: &str) -> Option<(usize, usize, &str)> {
    let open = raw.find(" [")?;
    let seconds = raw[..open].trim_start();
    if !seconds.contains('.') || seconds.parse::<f64>().is_err() {
        return None;
    }
    let close = open + 2 + raw[open + 2..].find(']')?;
    let domain = &raw[open + 2..close];
    let rest = &raw[close + 1..];
    if domain.is_empty() || domain.contains(' ') || !(rest.is_empty() || rest.starts_with(' ')) {
        return None;
    }
    Some((open, (close + 2).min(raw.len()), domain))
}

/// The log kept in memory: a handle on it, cheap to clone, which every thread may use.
#[derive(Clone)]
pub struct Kept(Arc<Inner>);

struct Inner {
    /// The `-d` switches of this run: their domains are kept whatever `log.domains` says.
    run: Switches,
    /// The kept lines' own repeats and width, their seconds counted as stderr's are.
    shared: Shared,
    /// Switches the layer's filter, which rebuilds tracing's interest cache.
    filter: Box<dyn Fn(Option<Switches>) + Send + Sync>,
    state: Mutex<State>,
    /// What asks the log window for a frame when a line comes, while it is open.
    waker: Mutex<Option<Waker>>,
    /// Whether the waker was woken since the last read, so a burst of lines wakes it once.
    woken: AtomicBool,
}

#[derive(Default)]
struct State {
    /// What is kept, None while nothing is.
    switches: Option<Switches>,
    lines: VecDeque<Line>,
    /// How many of this session's oldest lines went past the capacity.
    dropped: u64,
    /// The number of the next line kept in this session, from 0.
    next: u64,
    /// The session: one more each time keeping starts.
    session: u64,
}

impl State {
    /// Keeps the lines one event wrote, `text`, `severity` its own level: a line saying
    /// how often the one before repeated, then its own, each perhaps over several lines.
    fn push(&mut self, text: &str, severity: Severity) {
        let own = text
            .lines()
            .enumerate()
            .filter(|(_, line)| prefix(line).is_some())
            .last()
            .map(|(i, _)| i);
        for (i, raw) in text.lines().enumerate() {
            let line = Line::read(raw, self.lines.back(), (Some(i) == own).then_some(severity));
            self.keep(line);
        }
    }

    fn keep(&mut self, line: Line) {
        if self.lines.len() == CAPACITY {
            self.lines.pop_front();
            self.dropped += 1;
        }
        self.lines.push_back(line);
        self.next += 1;
    }
}

/// Where a reader of the log is: the session it read and the next line it has not.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cursor {
    session: u64,
    next: u64,
}

/// What [`Kept::read`] gives.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Read {
    /// The lines kept since the cursor's last read, oldest first, or since this session's
    /// start where it `restarted`.
    pub lines: Vec<Line>,
    /// Keeping started again since the cursor's last read: the lines read before are no
    /// longer kept.
    pub restarted: bool,
    /// How many of this session's oldest lines are no longer kept.
    pub dropped: u64,
    /// The line saying how often the last line has repeated so far, which the terminal
    /// writes once another comes.
    pub pending: Option<Line>,
}

impl Kept {
    /// A kept log on its own subscriber, outside the program's logging, with `run` as its
    /// `-d` switches: for tests and tools, which log to the dispatch it returns.
    pub fn scoped<S: AsRef<str>>(run: &[S]) -> Result<(Kept, Dispatch), Error> {
        let run = Switches::parse(run)?;
        let (lines, kept) = layer(Instant::now(), run);
        let dispatch = Dispatch::new(tracing_subscriber::registry().with(lines));
        Ok((kept, dispatch))
    }

    /// Keeps the lines of `chosen`'s domains from now on, with the run's `-d` domains,
    /// warnings and errors, and the detail `verbose` adds where `chosen` holds
    /// [`Domain::VERBOSE`] or `-d` turned it on; None keeps nothing, and lets go of what was
    /// kept. As keeping starts, the banner's lines, `banner`'s, are kept first. A change of
    /// the domains applies from the next line; the same domains again change nothing.
    pub fn keep(&self, chosen: Option<&[Domain]>, banner: impl FnOnce() -> String) {
        let switches = chosen.map(|chosen| with(&self.0.run, chosen));
        let mut state = self.0.state();
        if state.switches == switches {
            return;
        }
        match &switches {
            None => {
                state.lines = VecDeque::new();
                state.dropped = 0;
            }
            Some(on) => {
                if state.switches.is_none() {
                    state.session += 1;
                    state.next = 0;
                    *self.0.shared.last() = Last::default();
                    for line in banner().lines() {
                        state.keep(Line::plain(line));
                    }
                }
                self.0.shared.whole.store(on.verbose, Ordering::Relaxed);
            }
        }
        state.switches = switches.clone();
        drop(state);
        (self.0.filter)(switches);
        self.0.wake();
    }

    /// Keeps `text` as the lines an event of `severity` wrote, while keeping is on: lines
    /// of known seconds, for tests and tools that draw them.
    #[doc(hidden)]
    pub fn keep_written(&self, text: &str, severity: Severity) {
        let mut state = self.0.state();
        if state.switches.is_some() {
            state.push(text, severity);
        }
        drop(state);
        self.0.wake();
    }

    /// What is kept: the domains, and whether `verbose`'s detail is; None while nothing is.
    pub fn switches(&self) -> Option<Switches> {
        self.0.state().switches.clone()
    }

    /// The lines kept since `cursor` last read, which it moves past them, and how many of
    /// the oldest went. Never held while the caller draws, so a line logged meanwhile waits
    /// for nothing.
    pub fn read(&self, cursor: &mut Cursor) -> Read {
        self.0.woken.store(false, Ordering::Release);
        let state = self.0.state();
        let restarted = cursor.session != state.session;
        if restarted {
            *cursor = Cursor {
                session: state.session,
                next: 0,
            };
        }
        let first = state.next - state.lines.len() as u64;
        let from = cursor.next.max(first);
        let lines: Vec<Line> = state
            .lines
            .iter()
            .skip((from - first) as usize)
            .cloned()
            .collect();
        cursor.next = state.next;
        let dropped = state.dropped;
        let on = state.switches.is_some();
        let pending = self.0.shared.last().repeated().filter(|_| on).map(|text| {
            let own = state.lines.back();
            Line::read(text.trim_end(), own, None)
        });
        Read {
            lines,
            restarted,
            dropped,
            pending,
        }
    }

    /// Asks `waker` for a frame when a line comes, at most once until the next read; None
    /// asks nothing.
    pub fn wake_with(&self, waker: Option<Waker>) {
        *self.0.waker() = waker;
        self.0.woken.store(false, Ordering::Release);
    }
}

impl fmt::Debug for Kept {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Kept")
            .field("switches", &self.switches())
            .finish_non_exhaustive()
    }
}

impl Inner {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn waker(&self) -> MutexGuard<'_, Option<Waker>> {
        self.waker.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Wakes the waker, unless it was since the last read.
    fn wake(&self) {
        if self.woken.swap(true, Ordering::AcqRel) {
            return;
        }
        let waker = self.waker().clone();
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

/// `run`'s switches with `chosen`'s domains on too, and its detail where `chosen` holds
/// [`Domain::VERBOSE`].
fn with(run: &Switches, chosen: &[Domain]) -> Switches {
    let mut switches = run.clone();
    for domain in chosen {
        if *domain == Domain::VERBOSE {
            switches.verbose = true;
        } else if !switches.domains.contains(&domain.name) {
            switches.domains.push(domain.name);
        }
    }
    switches
}

/// The kept log's layer, with its filter off, and its handle; its seconds count from
/// `start`, `run` the `-d` switches it keeps.
pub(super) fn layer<S>(start: Instant, run: Switches) -> (impl Layer<S>, Kept)
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    let (filter, handle) = reload::Layer::new(Keeping(None));
    let inner = Arc::new(Inner {
        run,
        shared: Shared::new(start, false),
        filter: Box::new(move |switches| {
            // Only fails when the subscriber is gone, when there is nothing to filter.
            let _ = handle.modify(|keeping| keeping.0 = switches);
        }),
        state: Mutex::new(State::default()),
        waker: Mutex::new(None),
        woken: AtomicBool::new(false),
    });
    (KeptLines(inner.clone()).with_filter(filter), Kept(inner))
}

/// The kept log's filter: what is kept, None while nothing is, when it lets nothing through
/// and tells tracing so, each callsite's interest `never` and its level hint `OFF`.
struct Keeping(Option<Switches>);

impl<S> Filter<S> for Keeping {
    fn enabled(&self, metadata: &Metadata<'_>, _: &Context<'_, S>) -> bool {
        self.0
            .as_ref()
            .is_some_and(|on| on.admits(metadata.target(), *metadata.level()))
    }

    /// On, every callsite is asked per event, as stderr's filter has it: a `log` record's
    /// callsite is tracing-log's, of every target.
    fn callsite_enabled(&self, _: &'static Metadata<'static>) -> Interest {
        match &self.0 {
            None => Interest::never(),
            Some(_) => Interest::sometimes(),
        }
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        match &self.0 {
            None => Some(LevelFilter::OFF),
            Some(on) => Filter::<S>::max_level_hint(on),
        }
    }
}

/// The kept log's layer: each line its filter lets through, as the terminal writes it.
struct KeptLines(Arc<Inner>);

impl<S> Layer<S> for KeptLines
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        let normalized = event.normalized_metadata();
        let metadata = normalized.as_ref().unwrap_or_else(|| event.metadata());
        {
            let mut state = self.0.state();
            // rawler's lens lookups, as stderr's filter has them: reload's filter hands
            // on no event's own check.
            let Some(on) = &state.switches else {
                return;
            };
            if !on.admits_event(event) {
                return;
            }
            let mut text = String::new();
            let mut fields = |writer: Writer<'_>| DefaultFields::new().format_fields(writer, event);
            let written = self
                .0
                .shared
                .write(metadata, &mut fields, Writer::new(&mut text));
            if written.is_err() || text.is_empty() {
                return;
            }
            state.push(&text, Severity::of(*metadata.level()));
        }
        self.0.wake();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    /// The lines `kept` holds now, each without its seconds.
    fn untimed(kept: &Kept) -> Vec<String> {
        let read = kept.read(&mut Cursor::default());
        read.lines.iter().map(without_seconds).collect()
    }

    fn without_seconds(line: &Line) -> String {
        match line.bracket() {
            "" => line.text().to_owned(),
            bracket => format!("{bracket} {}", line.message()),
        }
    }

    fn banner() -> String {
        "Apertrust 0.1.0\n[starting] as: apertrust\n".to_owned()
    }

    fn lines(kept: &Kept, dispatch: &Dispatch, log: impl FnOnce()) -> Vec<String> {
        tracing::dispatcher::with_default(dispatch, log);
        untimed(kept)
    }

    #[test]
    fn off_it_keeps_nothing_and_tells_tracing_so() {
        let _serial = super::super::tests::serial();
        let (layer, kept) = layer(Instant::now(), Switches::default());
        let subscriber = Arc::new(tracing_subscriber::registry().with(layer));
        let dispatch = Dispatch::new(subscriber.clone());
        let kept_lines = lines(&kept, &dispatch, || {
            tracing::debug!(target: "perf", "rendered 22 passes in 682.5 ms");
            tracing::warn!(target: "raw", "no lens data");
            tracing::error!(target: "gpu", "device lost");
        });
        assert!(kept_lines.is_empty(), "{kept_lines:?}");
        assert_eq!(kept.switches(), None);
        assert_eq!(subscriber.max_level_hint(), Some(LevelFilter::OFF));
        // On, its hint is the switches': warnings with no domain chosen.
        kept.keep(Some(&[]), String::new);
        assert_eq!(subscriber.max_level_hint(), Some(LevelFilter::WARN));
        // Off again, it lets go of what it kept.
        tracing::dispatcher::with_default(&dispatch, || tracing::warn!(target: "raw", "x"));
        kept.keep(None, String::new);
        assert!(untimed(&kept).is_empty());
        assert_eq!(subscriber.max_level_hint(), Some(LevelFilter::OFF));
    }

    #[test]
    fn it_keeps_the_domains_chosen_with_the_banner_first() {
        let _serial = super::super::tests::serial();
        let (kept, dispatch) = Kept::scoped::<&str>(&[]).unwrap();
        kept.keep(Some(&[Domain::PERF, Domain::RAW]), banner);
        let kept_lines = lines(&kept, &dispatch, || {
            tracing::debug!(target: "perf", "decoded B13A0729.CR2 in 277 ms");
            tracing::debug!(target: "pipe", "plan: export, 8 ops");
            tracing::trace!(target: "perf", "pass rawprepare: 3.0 ms");
            tracing::debug!(target: "raw", "levels: black 511, white 14008");
        });
        assert_eq!(
            kept_lines,
            [
                "Apertrust 0.1.0",
                "[starting] as: apertrust",
                "[perf] decoded B13A0729.CR2 in 277 ms",
                "[raw] levels: black 511, white 14008",
            ]
        );
        let read = kept.read(&mut Cursor::default());
        assert_eq!(read.lines[0].domain(), "");
        assert_eq!(read.lines[2].domain(), "perf");
        assert_eq!(read.lines[2].severity(), Severity::Debug);
        // A change of the domains applies from the next line.
        kept.keep(Some(&[Domain::PIPE]), banner);
        let kept_lines = lines(&kept, &dispatch, || {
            tracing::debug!(target: "perf", "decoded again");
            tracing::debug!(target: "pipe", "plan: viewport");
        });
        assert_eq!(kept_lines.last().unwrap(), "[pipe] plan: viewport");
        assert_eq!(kept_lines.len(), 5);
    }

    #[test]
    fn warnings_and_errors_are_always_kept_and_verbose_only_when_chosen() {
        let _serial = super::super::tests::serial();
        let (kept, dispatch) = Kept::scoped::<&str>(&[]).unwrap();
        kept.keep(Some(&[]), String::new);
        let kept_lines = lines(&kept, &dispatch, || {
            tracing::warn!(target: "raw", "R0001176.DNG: no lens data");
            tracing::error!(target: "egui_wgpu", "surface lost");
            tracing::trace!(target: "perf", "pass demosaic: 9.1 ms");
        });
        assert_eq!(
            kept_lines,
            [
                "[raw] warning: R0001176.DNG: no lens data",
                "[egui_wgpu] error: surface lost",
            ]
        );
        let read = kept.read(&mut Cursor::default());
        let severities: Vec<Severity> = read.lines.iter().map(Line::severity).collect();
        assert_eq!(severities, [Severity::Warning, Severity::Error]);

        kept.keep(Some(&[Domain::PERF, Domain::VERBOSE]), String::new);
        let kept_lines = lines(&kept, &dispatch, || {
            tracing::trace!(target: "perf", "pass demosaic: 9.1 ms");
        });
        assert_eq!(kept_lines.last().unwrap(), "[perf] pass demosaic: 9.1 ms");
        let read = kept.read(&mut Cursor::default());
        assert_eq!(read.lines.last().unwrap().severity(), Severity::Detail);
    }

    #[test]
    fn the_domains_d_turned_on_are_kept_too() {
        let _serial = super::super::tests::serial();
        let (kept, dispatch) = Kept::scoped(&["pipe"]).unwrap();
        kept.keep(Some(&[Domain::PERF]), String::new);
        let kept_lines = lines(&kept, &dispatch, || {
            tracing::debug!(target: "pipe", "plan: export");
            tracing::debug!(target: "gpu", "adapter: Apple M4");
        });
        assert_eq!(kept_lines, ["[pipe] plan: export"]);
        let switches = kept.switches().unwrap();
        assert_eq!(switches.domains, ["pipe", "perf"]);
        assert!(!switches.verbose);
    }

    #[test]
    fn past_the_capacity_the_oldest_go_and_are_counted() {
        let _serial = super::super::tests::serial();
        let (kept, dispatch) = Kept::scoped::<&str>(&[]).unwrap();
        kept.keep(Some(&[Domain::PERF]), String::new);
        let mut cursor = Cursor::default();
        tracing::dispatcher::with_default(&dispatch, || {
            for i in 0..10 {
                tracing::debug!(target: "perf", "line {i}");
            }
        });
        assert_eq!(kept.read(&mut cursor).lines.len(), 10);
        tracing::dispatcher::with_default(&dispatch, || {
            for i in 10..CAPACITY + 5 {
                tracing::debug!(target: "perf", "line {i}");
            }
        });
        // The reader is handed what it has not read and is still kept.
        let read = kept.read(&mut cursor);
        assert_eq!(read.dropped, 5);
        assert_eq!(read.lines.len(), CAPACITY + 5 - 10);
        assert_eq!(read.lines.last().unwrap().message(), "line 20004");
        let all = kept.read(&mut Cursor::default());
        assert_eq!(all.lines.len(), CAPACITY);
        assert_eq!(all.lines[0].message(), "line 5");
        assert!(all.restarted);
        assert!(kept.read(&mut cursor).lines.is_empty());
    }

    #[test]
    fn a_repeated_line_is_kept_once_with_its_count_as_on_the_terminal() {
        let _serial = super::super::tests::serial();
        let (kept, dispatch) = Kept::scoped::<&str>(&[]).unwrap();
        kept.keep(Some(&[Domain::PERF]), String::new);
        let mut cursor = Cursor::default();
        tracing::dispatcher::with_default(&dispatch, || {
            for _ in 0..3 {
                tracing::warn!(target: "wgpu_core::device", "a buffer is too large");
            }
        });
        // The count so far, as the terminal will write it once another line comes.
        let read = kept.read(&mut cursor);
        assert_eq!(read.lines.len(), 1);
        let pending = read.pending.expect("the count so far");
        assert_eq!(
            without_seconds(&pending),
            "[gpu] warning: a buffer is too large (repeated 2×)"
        );
        assert_eq!(pending.severity(), Severity::Warning);
        tracing::dispatcher::with_default(&dispatch, || {
            tracing::debug!(target: "perf", "render in 21 ms");
        });
        assert_eq!(
            untimed(&kept),
            [
                "[gpu] warning: a buffer is too large",
                "[gpu] warning: a buffer is too large (repeated 2×)",
                "[perf] render in 21 ms",
            ]
        );
        assert_eq!(kept.read(&mut cursor).pending, None);
    }

    /// The kept lines are stderr's, byte for byte, for the same switches.
    #[test]
    fn the_lines_are_the_terminal_s() {
        let _serial = super::super::tests::serial();
        #[derive(Clone, Default)]
        struct Captured(Arc<Mutex<Vec<u8>>>);
        impl std::io::Write for Captured {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let captured = Captured::default();
        let writer = captured.clone();
        let switches = Switches::parse(&["perf", "raw"]).unwrap();
        let start = Instant::now();
        let (stderr, _) = super::super::layer(start, false, move || writer.clone());
        let (kept_lines, kept) = layer(start, Switches::default());
        let subscriber = tracing_subscriber::registry()
            .with(kept_lines)
            .with(stderr.with_filter(switches));
        kept.keep(Some(&[Domain::PERF, Domain::RAW]), String::new);
        tracing::subscriber::with_default(subscriber, || {
            tracing::debug!(target: "perf", "decoded {} in {} ms", "B13A0729.CR2", 277);
            tracing::warn!(target: "rawler::lens", "No lens definition found in database");
            tracing::warn!(target: "rawler::tiff", "{}", "x".repeat(140));
            tracing::warn!(target: "rawler::tiff", "{}", "x".repeat(140));
            tracing::error!(target: "wgpu_core::device", "Validation Error\n\nCaused by:\n  a");
            tracing::debug!(target: "raw", "levels: black 511\nwhite 14008");
        });
        let terminal = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
        let read = kept.read(&mut Cursor::default());
        // Each layer reads the clock for itself, so their seconds may differ in the last
        // place.
        let untimed = |text: &str| text.get(10..).unwrap_or(text).to_owned();
        let texts: Vec<String> = read.lines.iter().map(|l| untimed(l.text())).collect();
        let written: Vec<String> = terminal.lines().map(untimed).collect();
        assert_eq!(texts, written);
        for (kept, written) in read.lines.iter().zip(terminal.lines()) {
            let seconds = |text: &str| text.get(..10).and_then(|s| s.trim().parse::<f64>().ok());
            if let (Some(a), Some(b)) = (seconds(kept.text()), seconds(written)) {
                assert!((a - b).abs() < 0.01, "{a} {b}");
            }
        }
        // A message's second line belongs to its domain.
        assert_eq!(read.lines.last().unwrap().domain(), "raw");
        assert_eq!(read.lines.last().unwrap().bracket(), "");
    }

    #[test]
    fn a_line_comes_apart_into_seconds_domain_and_message() {
        let line = Line::read("    0.6200 [raw] warning: no lens data", None, None);
        assert_eq!(line.seconds(), "0.6200");
        assert_eq!(line.bracket(), "[raw]");
        assert_eq!(line.message(), "warning: no lens data");
        assert_eq!(line.severity(), Severity::Warning);
        let banner = Line::read("Build:", None, None);
        assert_eq!((banner.seconds(), banner.bracket()), ("", ""));
        assert_eq!(banner.message(), "Build:");
        assert!(prefix("[starting] as: apertrust").is_none());
        assert!(prefix("  wgpu 30.0.1 [Metal]").is_none());
    }

    /// The cost of a kept line, logged through tracing into the kept log with the filter
    /// on, past the capacity so each keeps one and lets one go: run with
    /// `cargo test --release -p apertrust-base --features log -- --ignored --nocapture
    /// a_kept_line_s_cost`.
    #[test]
    #[ignore = "a timing, for the release build"]
    fn a_kept_line_s_cost() {
        let _serial = super::super::tests::serial();
        let (kept, dispatch) = Kept::scoped::<&str>(&[]).unwrap();
        let lines = 200_000;
        let time = |kept: &Kept| {
            tracing::dispatcher::with_default(&dispatch, || {
                let at = Instant::now();
                for i in 0..lines {
                    tracing::debug!(
                        target: "perf",
                        "rendered {} passes from rawprepare in {:.1} ms, GPU: {:.1} ms",
                        22,
                        682.5 + f64::from(i % 7),
                        185.7
                    );
                }
                let ns = at.elapsed().as_nanos() as f64 / f64::from(lines);
                let _ = kept;
                ns
            })
        };
        let off = time(&kept);
        kept.keep(Some(&[Domain::PERF]), String::new);
        let on = time(&kept);
        println!("a debug line: {off:.1} ns with the log off, {on:.0} ns kept");
    }
}
