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

//! Apertrust's diagnostics, the way darktable has them (architecture decision 44):
//! silent but for warnings and errors, unless a domain is switched on with `-d`, as in
//! darktable's `-d perf`. Every line is the seconds since start, the domain in
//! brackets and a short phrase, the fact first and its details as `key: value`, within
//! [`WIDTH`] columns in the common case (decision 44):
//!
//! ```text
//!     0.1301 [perf] decoded DSC0042.ARW in 93 ms
//!     0.4430 [pipe] plan: export, 8 ops, 22 passes, 9600×6376 RGBA
//!     0.6200 [gpu] warning: software adapter: llvmpipe, rendering is slow
//! ```
//!
//! Library crates log with `tracing`, naming the domain as the target: `debug!` for a
//! domain's lines, `trace!` for the detail `-d verbose` adds, and `warn!` and `error!`,
//! which always show. Other crates' lines, rawler's through the `log` crate, are
//! condensed to the width unless `verbose` is on ([`condense`]), an error kept whole on
//! [`one_line`], and a line repeated is written once with how often it repeated.
//! rawler's lens lookups are the raw domain's: Apertrust does not use the lens yet.
//! Binaries call [`init`] with their `-d` switches, keeping the [`Logging`] it returns,
//! then [`starting`] with the [`Program`] that [`program!`](crate::program) names, which
//! prints the [`banner()`] when a domain is on.

mod banner;

pub use banner::{
    Item, Program, REPOSITORY, about, accesskit, banner, egui, full_version, kernels, languages,
    onnx_runtime, print_version, rawler, starting, version_asked, wgpu,
};

use std::borrow::Cow;
use std::fmt;
use std::io::Write as _;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use tracing::level_filters::LevelFilter;
use tracing::{Event, Level, Metadata, Subscriber};
use tracing_log::NormalizeEvent;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::layer::{Context, Filter, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

/// A kind of diagnostics that `-d` switches on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Domain {
    pub name: &'static str,
    pub about: &'static str,
}

pub const DOMAINS: &[Domain] = &[
    Domain {
        name: "gpu",
        about: "the adapter in use, its limits and wgpu's own messages",
    },
    Domain {
        name: "raw",
        about: "decoding: the camera, sensor, levels and white balance",
    },
    Domain {
        name: "pipe",
        about: "the plan: operations in order, their images and passes",
    },
    Domain {
        name: "perf",
        about: "timings: decoding, uploading, compiling, each pass, writing",
    },
    Domain {
        name: "memory",
        about: "GPU buffers: what is allocated, and how much",
    },
    Domain {
        name: "golden",
        about: "the harness: darktable-cli runs and cached references",
    },
    Domain {
        name: "settings",
        about: "the user data folder, how it was chosen, and each read and write of settings",
    },
    Domain {
        name: "control",
        about: "background jobs: each added, run, discarded or cancelled, its queue and priority",
    },
    Domain {
        name: "event",
        about: "the session's events: each sent, with its numbers, and each drain; signal too",
    },
];

/// Other names of domains, as darktable's `-d signal` for its signals: each turns on the
/// domain it names (decision 59).
pub const ALIASES: &[(&str, &str)] = &[("signal", "event")];

/// Help text for a binary's `-d` option.
pub fn help() -> String {
    let mut text = String::from(
        "Switch on a debug domain, as darktable's -d; repeat for more. `all` switches on \
         every domain, `verbose` adds detail to those on. Domains:",
    );
    for domain in DOMAINS {
        text.push_str(&format!(" {} ({});", domain.name, domain.about));
    }
    text.pop();
    text.push('.');
    text
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no debug domain {0:?}; the domains are {list}, all and verbose", list = names())]
    UnknownDomain(String),
    #[error("logging is already set up")]
    AlreadySet,
}

fn names() -> String {
    DOMAINS
        .iter()
        .map(|domain| domain.name)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Which domains are on, from `-d` switches.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Switches {
    pub domains: Vec<&'static str>,
    pub verbose: bool,
}

impl Switches {
    pub fn parse<S: AsRef<str>>(switches: &[S]) -> Result<Self, Error> {
        let mut parsed = Self::default();
        for switch in switches {
            for name in switch.as_ref().split(',').map(str::trim) {
                match name {
                    "all" => parsed.domains = DOMAINS.iter().map(|d| d.name).collect(),
                    "verbose" => parsed.verbose = true,
                    _ => {
                        let name = ALIASES
                            .iter()
                            .find(|(alias, _)| *alias == name)
                            .map_or(name, |(_, domain)| domain);
                        let domain = DOMAINS
                            .iter()
                            .find(|d| d.name == name)
                            .ok_or_else(|| Error::UnknownDomain(name.to_owned()))?;
                        if !parsed.domains.contains(&domain.name) {
                            parsed.domains.push(domain.name);
                        }
                    }
                }
            }
        }
        Ok(parsed)
    }

    /// Whether a line shows: a warning or an error from anywhere; a domain's own line
    /// when the domain is on, its detail too when verbose; wgpu's messages only with
    /// `gpu` and `verbose`, since they are many. Targets match exactly, so the domain
    /// `raw` never lets rawler's own debug lines through.
    fn admits(&self, target: &str, level: Level) -> bool {
        if level <= Level::WARN {
            return true;
        }
        if self.domains.contains(&target) {
            return level <= Level::DEBUG || self.verbose;
        }
        let wgpu = target.starts_with("wgpu_core") || target.starts_with("wgpu_hal");
        wgpu && self.verbose && self.domains.contains(&"gpu") && level <= Level::INFO
    }
}

impl<S> Filter<S> for Switches {
    fn enabled(&self, metadata: &Metadata<'_>, _: &Context<'_, S>) -> bool {
        self.admits(metadata.target(), *metadata.level())
    }

    /// rawler's lens lookups are the raw domain's lines, not warnings: Apertrust does not
    /// use the lens yet, so they show only with `raw` on.
    fn event_enabled(&self, event: &Event<'_>, _: &Context<'_, S>) -> bool {
        let normalized = event.normalized_metadata();
        let metadata = normalized.as_ref().unwrap_or_else(|| event.metadata());
        !metadata.target().starts_with("rawler")
            || self.domains.contains(&"raw")
            || !is_lens(&message_of(event))
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        Some(match (self.domains.is_empty(), self.verbose) {
            (true, _) => LevelFilter::WARN,
            (false, false) => LevelFilter::DEBUG,
            (false, true) => LevelFilter::TRACE,
        })
    }
}

/// An event's message, as a crate wrote it.
fn message_of(event: &Event<'_>) -> String {
    struct Message(String);
    impl tracing::field::Visit for Message {
        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            if field.name() == "message" {
                self.0 = value.to_owned();
            }
        }
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
            if field.name() == "message" {
                self.0 = format!("{value:?}");
            }
        }
    }
    let mut message = Message(String::new());
    event.record(&mut message);
    message.0
}

/// Sends every log line to stderr from now on, wgpu's and rawler's `log` messages
/// included. `RUST_LOG`, when set, replaces the switches with its own filter. With a
/// domain on, or `RUST_LOG`, [`starting`] then prints the banner. Keep what it returns
/// until the program ends: dropped, it writes how often the last line repeated.
pub fn init<S: AsRef<str>>(switches: &[S]) -> Result<Logging, Error> {
    let switches = Switches::parse(switches)?;
    let developer_filter = std::env::var("RUST_LOG")
        .ok()
        .and_then(|value| EnvFilter::try_new(value).ok());
    let shown = !switches.domains.is_empty() || developer_filter.is_some();
    let whole = switches.verbose || developer_filter.is_some();
    let (lines, shared) = layer(whole, std::io::stderr);
    let registry = tracing_subscriber::registry();
    match developer_filter {
        Some(filter) => registry.with(lines.with_filter(filter)).try_init(),
        None => registry.with(lines.with_filter(switches)).try_init(),
    }
    .map_err(|_| Error::AlreadySet)?;
    banner::SHOWN.store(shown, Ordering::Relaxed);
    Ok(Logging(shared))
}

/// The layer that writes the lines to `writer`, other crates' whole when `whole`, and the
/// state they share.
fn layer<S, W>(whole: bool, writer: W) -> (impl Layer<S>, Arc<Shared>)
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    W: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let shared = Arc::new(Shared {
        start: Instant::now(),
        whole,
        last: Mutex::new(Last::default()),
    });
    let lines = tracing_subscriber::fmt::layer()
        .with_writer(writer)
        .event_format(Lines(shared.clone()));
    (lines, shared)
}

/// Logging, set up by [`init`]: dropped, it writes how often the last line repeated.
#[must_use = "dropped, it ends the count of a repeated line"]
pub struct Logging(Arc<Shared>);

impl Drop for Logging {
    fn drop(&mut self) {
        if let Some(line) = self.0.repeats() {
            let _ = std::io::stderr().lock().write_all(line.as_bytes());
        }
    }
}

/// The columns a line fits in the common case, its time and domain included; other
/// crates' lines are cut to it unless `verbose` is on.
pub const WIDTH: usize = 100;

/// What every line shares.
struct Shared {
    start: Instant,
    /// Whether other crates' lines show whole: under `verbose`, or `RUST_LOG`.
    whole: bool,
    last: Mutex<Last>,
}

/// The last line another crate wrote, to count its repeats rather than write them.
#[derive(Default)]
struct Last {
    /// Its target, level and message as the crate wrote it; `None` after a line of ours.
    key: Option<(String, Level, String)>,
    /// What was written after the time, for the line that says how often it repeated.
    written: String,
    repeats: usize,
    /// The time of the last repeat.
    at: f64,
}

impl Shared {
    fn last(&self) -> std::sync::MutexGuard<'_, Last> {
        self.last
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The line saying how often the last one repeated, if it did, and the count ended.
    fn repeats(&self) -> Option<String> {
        let mut last = self.last();
        let line = (last.repeats > 0).then(|| {
            format!(
                "{:10.4} {} (repeated {}×)\n",
                last.at, last.written, last.repeats
            )
        });
        *last = Last::default();
        line
    }
}

/// darktable's line: seconds since start, the domain, the message.
struct Lines(Arc<Shared>);

impl<S, N> FormatEvent<S, N> for Lines
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let normalized = event.normalized_metadata();
        let metadata = normalized.as_ref().unwrap_or_else(|| event.metadata());
        let seconds = self.0.start.elapsed().as_secs_f64();
        let (target, level) = (metadata.target(), *metadata.level());
        let label = match level {
            Level::ERROR => "error: ",
            Level::WARN => "warning: ",
            _ => "",
        };
        let domain = domain_of(target);
        if DOMAINS.iter().any(|domain| domain.name == target) {
            if let Some(repeats) = self.0.repeats() {
                writer.write_str(&repeats)?;
            }
            write!(writer, "{seconds:10.4} [{domain}] {label}")?;
            ctx.field_format().format_fields(writer.by_ref(), event)?;
            return writeln!(writer);
        }
        let mut message = String::new();
        ctx.field_format()
            .format_fields(Writer::new(&mut message), event)?;
        // rawler's lens lookups, one line however often it says so for one lookup.
        // Its warnings come from more than one of rawler's modules.
        let lens = !self.0.whole && is_lens(&message);
        if lens {
            message = LENS.to_owned();
        }
        let from = if lens { domain } else { target };
        let key = Some((from.to_owned(), level, message.clone()));
        let mut last = self.0.last();
        if last.key == key {
            if !lens {
                last.repeats += 1;
                last.at = seconds;
            }
            // An empty write writes no line.
            return Ok(());
        }
        drop(last);
        if let Some(repeats) = self.0.repeats() {
            writer.write_str(&repeats)?;
        }
        let label = if lens { "" } else { label };
        let prefix = format!("{seconds:10.4} [{domain}] {label}");
        let text = if self.0.whole || lens {
            message
        } else if level == Level::ERROR {
            // An error whole, its cause last.
            one_line(&message)
        } else {
            condense(&message, WIDTH.saturating_sub(prefix.chars().count()))
        };
        let mut last = self.0.last();
        last.key = key;
        last.written = format!("[{domain}] {label}{text}");
        writeln!(writer, "{prefix}{text}")
    }
}

/// rawler's lens lookups, as one line: it cannot tell the lens, either because its
/// database lacks it, which it says up to three times for one lookup, each asking for an
/// issue at dnglab's, or because the raw carries no lens data.
const LENS: &str = "lens: not identified by rawler";

fn is_lens(message: &str) -> bool {
    [
        "Found multiple (",
        "No lens definition found in database",
        "Lens resolver found no matching lens",
        // rawler 0.8's decoders, when a raw carries no lens to look up: Panasonic's
        // (rw2.rs), Olympus's (orf.rs), Canon's and Olympus's makernote (cr2.rs, orf.rs)
        // and Nikon's (nef.rs).
        "No lens data available",
        "No lens data found",
        "Camera settings in makernote not found, no lens data available",
        "Failed to read lens information",
    ]
    .iter()
    .any(|start| message.starts_with(start))
}

/// Text over several lines on one, as wgpu's errors are written: "Validation Error",
/// "Caused by:", then each cause, the innermost last.
pub fn one_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Another crate's line as a person reads it at a glance, within `room` characters:
/// rawler's request to report an issue left out, on [`one_line`], cut with "…" where it
/// is longer, its innermost cause kept. `-d verbose` shows them whole.
pub fn condense(message: &str, room: usize) -> String {
    const ISSUE: &str = "Please open an issue at https://github.com/dnglab/dnglab/issues";
    let message = message.split(ISSUE).next().unwrap_or(message);
    let line = one_line(message);
    let room = room.max(20);
    let width = |text: &str| text.chars().count();
    if width(&line) <= room {
        return line;
    }
    let lines: Vec<&str> = message
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    match lines.split_last() {
        Some((cause, rest)) if !rest.is_empty() && width(cause) + 20 <= room => {
            let head = cut(&rest.join(" "), room - width(cause) - 1);
            format!("{head} {cause}")
        }
        _ => cut(&line, room),
    }
}

/// `text` within `room` characters, ending in "…" when it is cut.
fn cut(text: &str, room: usize) -> String {
    if text.chars().count() <= room {
        return text.to_owned();
    }
    let mut cut: String = text.chars().take(room.saturating_sub(1)).collect();
    cut.truncate(cut.trim_end().len());
    cut.push('…');
    cut
}

/// A file's name, as a debug line names it, or the whole path where it has none.
pub fn file_name(path: &Path) -> Cow<'_, str> {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
}

/// The domain a target belongs to: a domain's own name, or the crate that logged.
fn domain_of(target: &str) -> &str {
    let krate = target.split("::").next().unwrap_or(target);
    match krate {
        "wgpu" | "wgpu_core" | "wgpu_hal" | "wgpu_types" | "naga" => "gpu",
        "rawler" => "raw",
        _ => krate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switches_parse_domains_all_and_verbose() {
        assert_eq!(
            Switches::parse(&["perf", "pipe,perf"]).unwrap(),
            Switches {
                domains: vec!["perf", "pipe"],
                verbose: false
            }
        );
        let all = Switches::parse(&["all", "verbose"]).unwrap();
        assert_eq!(all.domains.len(), DOMAINS.len());
        assert!(all.verbose);
        // darktable's name for its signals turns on the events.
        assert_eq!(
            Switches::parse(&["signal", "event"]).unwrap().domains,
            ["event"]
        );
        assert!(matches!(
            Switches::parse(&["opencl"]),
            Err(Error::UnknownDomain(name)) if name == "opencl"
        ));
    }

    #[test]
    fn warnings_always_show_and_domains_only_when_on() {
        let none = Switches::default();
        assert!(none.admits("rawler::decoders", Level::WARN));
        assert!(none.admits("perf", Level::ERROR));
        assert!(!none.admits("perf", Level::DEBUG));

        let perf = Switches::parse(&["perf"]).unwrap();
        assert!(perf.admits("perf", Level::DEBUG));
        assert!(!perf.admits("perf", Level::TRACE));
        assert!(!perf.admits("pipe", Level::DEBUG));
    }

    #[test]
    fn domains_match_targets_exactly() {
        let raw = Switches::parse(&["raw", "gpu"]).unwrap();
        assert!(raw.admits("raw", Level::DEBUG));
        assert!(!raw.admits("rawler::decoders", Level::DEBUG));
        assert!(!raw.admits("gpu_alloc", Level::DEBUG));
        assert!(!raw.admits("wgpu_core::device", Level::INFO));
    }

    #[test]
    fn verbose_adds_detail_and_wgpus_own_lines() {
        let gpu = Switches::parse(&["gpu", "verbose"]).unwrap();
        assert!(gpu.admits("gpu", Level::TRACE));
        assert!(gpu.admits("wgpu_hal::vulkan", Level::INFO));
        assert!(!gpu.admits("wgpu_hal::vulkan", Level::DEBUG));
    }

    /// The lines `log` writes with `switches` on, each without its time.
    fn written(switches: &[&str], log: impl FnOnce()) -> (Vec<String>, Option<String>) {
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
        let switches = Switches::parse(switches).unwrap();
        let (lines, shared) = layer(switches.verbose, move || writer.clone());
        let subscriber = tracing_subscriber::registry().with(lines.with_filter(switches));
        tracing::subscriber::with_default(subscriber, log);
        let text = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
        let untimed = |line: &str| line.get(11..).unwrap_or_default().to_owned();
        let lines = text.lines().map(untimed).collect();
        (lines, shared.repeats().as_deref().map(untimed))
    }

    /// rawler's warnings for one Canon EOS 5D Mark IV raw (B13A0729.CR2), as it logs them.
    fn lens_lookup() {
        tracing::warn!(
            target: "rawler::lens",
            "Found multiple (10) lens definitions, unable to determine which lens to use. \
             Please open an issue at https://github.com/dnglab/dnglab/issues and provide this \
             message (optionally the RAW file, if you can license it under CC0-license)."
        );
        tracing::warn!(
            target: "rawler::lens",
            "No lens definition found in database, search parameters: Mounts: [\"cn-mount\", \
             \"ef-mount\"], ID: '137:0', Keyname: '24-70mm', Focal len: '40/1'. Please open an \
             issue at https://github.com/dnglab/dnglab/issues and provide this message \
             (optionally the RAW file, if you can license it under CC0-license)."
        );
        tracing::warn!(
            target: "rawler::decoders",
            "Lens resolver found no matching lens, falling back to EXIF lens data: make: None, \
             model: Some(\"24-70mm\"), spec: Some([Rational {{ n: 24, d: 1 }}])"
        );
    }

    #[test]
    fn rawlers_lens_lookups_are_one_line_of_the_raw_domain() {
        let (lines, _) = written(&[], || {
            lens_lookup();
            tracing::warn!(target: "rawler::decoders", "ARW makernote not found");
        });
        assert_eq!(lines, ["[raw] warning: ARW makernote not found"]);

        let (lines, _) = written(&["raw"], || {
            lens_lookup();
            tracing::debug!(target: "raw", "levels: black 511, white 14008");
            lens_lookup();
        });
        assert_eq!(
            lines,
            [
                "[raw] lens: not identified by rawler",
                "[raw] levels: black 511, white 14008",
                "[raw] lens: not identified by rawler",
            ]
        );
    }

    /// rawler's decoders' warnings when a raw carries no lens, as the Panasonic DC-S5M2's,
    /// are the raw domain's too, so none is written without `-d`.
    #[test]
    fn rawlers_missing_lens_data_is_the_raw_domains_too() {
        let missing = || {
            tracing::warn!(target: "rawler::decoders::rw2", "No lens data available");
            tracing::warn!(target: "rawler::decoders::orf", "No lens data found");
            tracing::warn!(
                target: "rawler::decoders::cr2",
                "Camera settings in makernote not found, no lens data available"
            );
            tracing::warn!(
                target: "rawler::decoders::nef",
                "Failed to read lens information: UnexpectedEof"
            );
        };
        let (lines, _) = written(&[], missing);
        assert!(lines.is_empty(), "{lines:?}");
        // With `raw` on, each is the one lens line, which the repeats then count.
        let (lines, _) = written(&["raw"], missing);
        assert_eq!(lines, ["[raw] lens: not identified by rawler"]);
    }

    #[test]
    fn a_repeated_line_is_written_once_with_its_count() {
        let (lines, pending) = written(&[], || {
            for _ in 0..3 {
                tracing::warn!(target: "wgpu_core::device", "a buffer is too large");
            }
            tracing::warn!(target: "perf", "--scopes with --changes 0: no changes to count");
            // The same text, but an error now: both show.
            tracing::warn!(target: "wgpu_core::device", "a buffer is too large");
            tracing::error!(target: "wgpu_core::device", "a buffer is too large");
            tracing::error!(target: "wgpu_core::device", "a buffer is too large");
        });
        assert_eq!(
            lines,
            [
                "[gpu] warning: a buffer is too large",
                "[gpu] warning: a buffer is too large (repeated 2×)",
                "[perf] warning: --scopes with --changes 0: no changes to count",
                "[gpu] warning: a buffer is too large",
                "[gpu] error: a buffer is too large",
            ]
        );
        // What `Logging` writes as it is dropped.
        assert_eq!(
            pending.as_deref(),
            Some("[gpu] error: a buffer is too large (repeated 1×)\n")
        );

        // Two long messages that differ only past the width are two lines.
        let long = "x".repeat(120);
        let (lines, _) = written(&[], || {
            tracing::warn!(target: "rawler::tiff", "{long} a");
            tracing::warn!(target: "rawler::tiff", "{long} b");
        });
        assert_eq!(lines.len(), 2, "{lines:?}");
    }

    #[test]
    fn errors_of_other_crates_are_whole_on_one_line() {
        let error = "Validation Error\n\nCaused by:\n  In a CommandEncoder::copy_buffer_to_buffer, \
                     label = 'viewport frame'\n    Copy error\n      Source buffer \
                     (0, 1, Metal) with 'display 0' label has been destroyed";
        let (lines, _) = written(&[], || {
            tracing::error!(target: "wgpu_core::device", "{error}");
        });
        assert_eq!(lines, [format!("[gpu] error: {}", one_line(error))]);
        assert!(lines[0].ends_with("label has been destroyed"));
    }

    #[test]
    fn other_crates_lines_fit_the_width() {
        assert_eq!(
            condense("ARW makernote not found", 80),
            "ARW makernote not found"
        );
        assert_eq!(
            condense(
                "Unsupported bps: 9. Please open an issue at https://github.com/dnglab/dnglab/issues and provide this message",
                80
            ),
            "Unsupported bps: 9."
        );
        let long = "Validation Error: in a set_bind_group command, the buffer binding is larger than the device allows for storage buffers";
        let cut = condense(long, 60);
        assert_eq!(cut.chars().count(), 60, "{cut}");
        assert!(cut.ends_with('…') && long.starts_with(cut.trim_end_matches('…')));
        assert_eq!(
            condense("Validation Error\n\nCaused by:\n  a buffer", 60),
            "Validation Error Caused by: a buffer"
        );
        // Cut, the innermost cause stays.
        let error = "Validation Error\n\nCaused by:\n  In a CommandEncoder::copy_buffer_to_buffer, \
                     label = 'viewport frame'\n    Source buffer has been destroyed";
        let cut = condense(error, 70);
        assert_eq!(cut.chars().count(), 70, "{cut}");
        assert!(cut.starts_with("Validation Error Caused by: In a"), "{cut}");
        assert!(cut.ends_with("… Source buffer has been destroyed"), "{cut}");
    }

    #[test]
    fn library_targets_map_to_domains() {
        assert_eq!(domain_of("perf"), "perf");
        assert_eq!(domain_of("wgpu_hal::vulkan::instance"), "gpu");
        assert_eq!(domain_of("rawler::decoders::arw"), "raw");
        assert_eq!(domain_of("apertrust_engine::pipeline"), "apertrust_engine");
    }
}
