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

//! Settings › Processing's Hardware acceleration group, beyond what its rows' kinds draw
//! (decision 81; `docs/gui.md`, The Settings dialog):
//!
//! - **The graphics card's list** ([`Card`]): Automatic, saying in brackets what it picks,
//!   then the hardware adapters the chosen API finds, by name, in the order Automatic ranks
//!   them, each name once, then a separator and CPU, the software adapter, stored as the
//!   reserved `cpu` and never shown by its own name (WARP, llvmpipe). CPU is disabled where
//!   the API has none and not listed on the Mac. A stored card the API does not find is
//!   listed disabled and checked, the row saying Automatic is used.
//! - **Listing** ([`Hardware`]): each API's adapters, listed once while the dialog is open,
//!   by a short-lived helper process, the app's own executable with [`HELPER_FLAG`], so a
//!   driver that crashes as it loads ends the helper and not the app; the helper's lines
//!   are read on a thread of their own, never in a frame. Only after
//!   [`LOOKING_AFTER`] seconds does the row say it is looking, so nothing flashes.
//! - **A card the person's API change leaves out** goes back to Automatic, saying so on the
//!   row until the card or the API changes again or another page shows.
//! - **The In use line** ([`InUse`]): the adapter and the API the engine opened, with a
//!   warning where that is the software adapter.

use std::io::Read as _;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use apertrust_base::settings::Choice as _;
use apertrust_engine::gpu::{self, Backend};
use apertrust_ui::i18n::Args;
use apertrust_ui::{Theme, Translator};
use eframe::egui::os::OperatingSystem;
use eframe::egui::{self, Ui};
use eframe::egui_wgpu::wgpu;

use crate::storage;

/// The hidden argument that makes the app the helper: `--list-adapters [auto|dx12|vulkan]`
/// lists the adapters of that graphics API on stdout, as [`lines`] writes them, and exits,
/// never opening a window.
pub const HELPER_FLAG: &str = "--list-adapters";

/// The helper's first line, saying what follows and in which version.
const HEADER: &str = "apertrust adapters 1";

/// The helper's last line: a list cut short, as by a crash, has none.
const END: &str = "end";

/// How long the helper may take before it is ended and the cards said not listed: loading
/// a cold driver can take seconds, but not this many.
const TIMEOUT: Duration = Duration::from_secs(10);

/// How long a listing goes before the row says it is looking, so a quick one never
/// flashes the words.
pub const LOOKING_AFTER: f64 = 0.3;

/// The kinds of adapter, by the ids the helper writes them with.
const KINDS: [(wgpu::DeviceType, &str); 5] = [
    (wgpu::DeviceType::DiscreteGpu, "discrete"),
    (wgpu::DeviceType::IntegratedGpu, "integrated"),
    (wgpu::DeviceType::VirtualGpu, "virtual"),
    (wgpu::DeviceType::Cpu, "cpu"),
    (wgpu::DeviceType::Other, "other"),
];

/// The backends, by the ids the helper writes them with.
const BACKENDS: [(wgpu::Backend, &str); 6] = [
    (wgpu::Backend::Vulkan, "vulkan"),
    (wgpu::Backend::Metal, "metal"),
    (wgpu::Backend::Dx12, "dx12"),
    (wgpu::Backend::Gl, "gl"),
    (wgpu::Backend::BrowserWebGpu, "webgpu"),
    (wgpu::Backend::Noop, "noop"),
];

/// The names the software adapters report, WARP's and lavapipe's, whose version follows
/// lavapipe's: how a stored text that names one is told where no listing has it.
const SOFTWARE_NAMES: [&str; 2] = ["Microsoft Basic Render Driver", "llvmpipe"];

/// An adapter as the helper lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct Adapter {
    pub name: String,
    pub kind: wgpu::DeviceType,
    pub backend: wgpu::Backend,
}

/// Why the cards could not be listed.
#[derive(Clone, Debug, PartialEq)]
pub enum Failure {
    /// The helper could not be started, or there is no way to list here.
    Start(String),
    /// The helper ended with this status, as when a driver crashes in it.
    Crashed(String),
    /// The helper took longer than its limit, and was ended.
    TimedOut,
    /// The helper printed nothing.
    Nothing,
    /// The helper printed something other than a list, or one cut short.
    Garbled,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Failure::Start(error) => write!(f, "not started: {error}"),
            Failure::Crashed(status) => write!(f, "the helper ended: {status}"),
            Failure::TimedOut => write!(f, "the helper took over {} s", TIMEOUT.as_secs()),
            Failure::Nothing => f.write_str("the helper printed nothing"),
            Failure::Garbled => f.write_str("the helper printed no list"),
        }
    }
}

/// What a listing gave.
pub type Listed = Result<Vec<Adapter>, Failure>;

/// The graphics API the command line asks the helper to list, where it asks for the
/// helper: `args`, after the program's name, starting with [`HELPER_FLAG`]. One it does not
/// name, or names wrongly, is Automatic.
pub fn helper_asked(mut args: impl Iterator<Item = std::ffi::OsString>) -> Option<Backend> {
    if args.next()? != HELPER_FLAG {
        return None;
    }
    let backend = args.next();
    let backend = backend
        .as_ref()
        .and_then(|id| Backend::from_id(id.to_str()?));
    Some(backend.unwrap_or_default())
}

/// The helper's work: the adapters an instance loading `backend` lists, as [`lines`] writes
/// them, on stdout. `WGPU_BACKEND`, which it inherits, chooses in the backend's place.
pub fn print_listed(backend: Backend) {
    print!("{}", lines(&gpu::listed(backend)));
}

/// `adapters` as the helper writes them: [`HEADER`], a line each, its kind, its backend
/// and its name apart by tabs, then [`END`].
pub fn lines(adapters: &[wgpu::AdapterInfo]) -> String {
    let mut text = format!("{HEADER}\n");
    for info in adapters {
        let kind = KINDS.iter().find(|(kind, _)| *kind == info.device_type);
        let backend = BACKENDS
            .iter()
            .find(|(backend, _)| *backend == info.backend);
        let (Some((_, kind)), Some((_, backend))) = (kind, backend) else {
            continue;
        };
        // A name is one field of one line.
        let name: String = info
            .name
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect();
        text.push_str(&format!("{kind}\t{backend}\t{}\n", name.trim()));
    }
    text.push_str(END);
    text.push('\n');
    text
}

/// The adapters in the helper's `text`: the lines between [`HEADER`] and [`END`]. What
/// comes before the header, as a driver writing to stdout, and after the end is not the
/// helper's. Nothing printed, no header, a line that is not an adapter or no end is a
/// failure.
pub fn parse(text: &str) -> Listed {
    let mut lines = text
        .lines()
        .map(|line| line.strip_suffix('\r').unwrap_or(line));
    if !lines.any(|line| line == HEADER) {
        return Err(if text.trim().is_empty() {
            Failure::Nothing
        } else {
            Failure::Garbled
        });
    }
    let mut adapters = Vec::new();
    for line in lines {
        if line == END {
            return Ok(adapters);
        }
        adapters.push(adapter(line).ok_or(Failure::Garbled)?);
    }
    Err(Failure::Garbled)
}

/// One of the helper's lines as an adapter.
fn adapter(line: &str) -> Option<Adapter> {
    let mut fields = line.splitn(3, '\t');
    let (kind, backend, name) = (fields.next()?, fields.next()?, fields.next()?);
    let kind = KINDS.iter().find(|(_, id)| *id == kind)?.0;
    let backend = BACKENDS.iter().find(|(_, id)| *id == backend)?.0;
    let name = name.trim();
    (!name.is_empty()).then(|| Adapter {
        name: name.to_owned(),
        kind,
        backend,
    })
}

/// Runs `command`, the helper, and reads its list: ended after `timeout`, as one stuck in
/// a driver; one that prints nothing or no whole list is a [`Failure`], said as a crash
/// where it ended with a failure. A whole list counts whatever its status, as a driver
/// that crashes as the helper ends, which is logged. Its stdout is read on a thread of its
/// own, so a full pipe never stops it, and a helper ended is reaped on another, so one
/// that a driver keeps from ending never holds the listing.
pub fn run(mut command: Command, timeout: Duration) -> Listed {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command
        .spawn()
        .map_err(|error| Failure::Start(error.to_string()))?;
    let mut stdout = child.stdout.take().ok_or(Failure::Nothing)?;
    let (read, text) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        let _ = read.send(String::from_utf8_lossy(&bytes).into_owned());
    });
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(None) => {
                end(child);
                return Err(Failure::TimedOut);
            }
            Err(error) => {
                end(child);
                return Err(Failure::Start(error.to_string()));
            }
        }
    };
    // Its stdout closes as it ends, unless something it started still holds it.
    let text = text
        .recv_timeout(Duration::from_secs(1))
        .unwrap_or_default();
    match (parse(&text), status.success()) {
        (Ok(adapters), success) => {
            if !success {
                tracing::warn!(target: "gpu", "adapter helper listed, then ended: {status}");
            }
            Ok(adapters)
        }
        (Err(_), false) => Err(Failure::Crashed(status.to_string())),
        (Err(failure), true) => Err(failure),
    }
}

/// Ends `child` and reaps it on a thread of its own: a helper stuck in a driver may not end
/// at once, or ever, and the listing never waits for it.
fn end(mut child: std::process::Child) {
    let _ = child.kill();
    let _ = std::thread::Builder::new()
        .name("adapter helper".into())
        .spawn(move || child.wait());
}

/// The app's own executable, to run as the helper: on Linux through `/proc/self/exe`, the
/// file this process runs, so a package upgrade that replaced it since the start never
/// runs another build; elsewhere, or where that is missing, its path.
fn own_executable() -> std::io::Result<std::path::PathBuf> {
    let running = std::path::Path::new("/proc/self/exe");
    if cfg!(target_os = "linux") && running.exists() {
        return Ok(running.to_owned());
    }
    std::env::current_exe()
}

/// How the dialog lists a graphics API's adapters: run on a thread of its own, never in a
/// frame.
#[derive(Clone)]
pub struct Lister(Arc<dyn Fn(Backend) -> Listed + Send + Sync>);

impl Lister {
    /// A lister that lists with `list`.
    pub fn new(list: impl Fn(Backend) -> Listed + Send + Sync + 'static) -> Lister {
        Lister(Arc::new(list))
    }

    /// The app's own: its executable as the helper, with [`HELPER_FLAG`] and the API's id.
    pub fn helper() -> Lister {
        Lister::new(|backend| {
            let program = own_executable().map_err(|e| Failure::Start(e.to_string()))?;
            let mut command = Command::new(program);
            command.args([HELPER_FLAG, backend.name()]);
            run(command, TIMEOUT)
        })
    }
}

impl std::fmt::Debug for Lister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Lister")
    }
}

impl PartialEq for Lister {
    fn eq(&self, other: &Lister) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// The adapter the engine opened, as the app's window opened it, for the In use line.
#[derive(Clone, Debug, PartialEq)]
pub struct InUse {
    pub name: String,
    pub backend: wgpu::Backend,
    /// Whether it is a software adapter, which the line calls CPU.
    pub software: bool,
    /// Whether the person chose the software adapter, `cpu`, rather than Automatic falling
    /// to it, when the line adds the driver advice.
    pub chosen: bool,
}

impl InUse {
    /// The adapter `info` describes, opened for `choice`: chosen where the choice names it,
    /// by `cpu` or by its own name, as a text kept before may, rather than wgpu's choice
    /// falling to it.
    pub fn of(info: &wgpu::AdapterInfo, choice: &gpu::AdapterChoice) -> InUse {
        let chosen = match choice {
            gpu::AdapterChoice::Named(name) | gpu::AdapterChoice::Preferred(name) => {
                gpu::matching(&[(info.name.as_str(), info.device_type)], name).is_some()
            }
            gpu::AdapterChoice::Any => false,
        };
        InUse {
            name: info.name.clone(),
            backend: info.backend,
            software: info.device_type == wgpu::DeviceType::Cpu,
            chosen,
        }
    }
}

/// What the app hands the dialog: the adapter in use, and how to list the others.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Running {
    pub in_use: Option<InUse>,
    pub lister: Option<Lister>,
}

/// One API's listing while the dialog is open.
#[derive(Clone, Debug)]
enum Listing {
    /// Under way since this time, in the context's seconds, its answer to come in the slot.
    Pending {
        since: f64,
        slot: Arc<Mutex<Option<Listed>>>,
    },
    Done(Listed),
}

impl PartialEq for Listing {
    fn eq(&self, other: &Listing) -> bool {
        match (self, other) {
            (Listing::Pending { slot: a, .. }, Listing::Pending { slot: b, .. }) => {
                Arc::ptr_eq(a, b)
            }
            (Listing::Done(a), Listing::Done(b)) => a == b,
            _ => false,
        }
    }
}

/// The group's state while the dialog is open: each API's listing, and a card the
/// person's API change left out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Hardware {
    running: Running,
    listings: Vec<(Backend, Listing)>,
    /// The stored card and the API the row had before the person changed it, until both
    /// that API's listing and the new one's say whether it goes.
    dropping: Option<(String, Backend)>,
    /// What the card's row says of a card the person's API change left out.
    dropped: Option<String>,
}

/// What a listing shows as the row is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shown<'a> {
    /// Under way; `looking` once it has been for [`LOOKING_AFTER`] seconds.
    Listing {
        looking: bool,
    },
    Listed(&'a [Adapter]),
    Failed,
}

/// What is said under a row of the group, in place of "Applies after a restart".
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub text: String,
    /// In the warning colour, else the secondary text's.
    pub warning: bool,
    /// Announced once as it appears, as a polite live region: it follows the person's own
    /// change on another row.
    pub live: bool,
}

impl Note {
    fn warning(text: String) -> Note {
        Note {
            text,
            warning: true,
            live: false,
        }
    }
}

/// What the group's rows draw this frame, beyond their kinds'.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub card: Card,
    /// The API row's Automatic, with the API it picks in brackets, once known.
    pub automatic: Option<String>,
    /// What is said under the API row: that the API finds no adapter.
    pub api_note: Option<Note>,
}

impl Hardware {
    /// The group handed `running` by the app.
    pub fn new(running: Running) -> Hardware {
        Hardware {
            running,
            ..Hardware::default()
        }
    }

    /// What the app handed it.
    pub fn running(&self) -> &Running {
        &self.running
    }

    /// The group's rows' parts this frame, `stored` the card's value and `backend` the API
    /// listed, as the API row has it: listing that API where it is not listed yet.
    pub fn frame(
        &mut self,
        ctx: &egui::Context,
        words: &Translator,
        stored: &str,
        backend: Backend,
    ) -> Frame {
        let os = ctx.os();
        let api = self.api_name(words, backend, os);
        let software = self.software(stored);
        let shown = self.shown(ctx, backend);
        let stored = Stored {
            text: stored,
            software,
        };
        let mut card = card(
            words,
            stored,
            shown,
            api.as_deref(),
            os,
            gpu::power_preference(),
        );
        if let Some(dropped) = &self.dropped {
            card.note = Some(Note {
                text: dropped.clone(),
                warning: true,
                live: true,
            });
        }
        let api_note = match self.done(backend) {
            Some(Ok(adapters)) if adapters.is_empty() => {
                let text = with_api(words, "settings-gpu-no-adapter", api.as_deref(), None);
                Some(Note::warning(text))
            }
            _ => None,
        };
        let automatic = self.automatic_api(words, stored.text);
        Frame {
            card,
            automatic,
            api_note,
        }
    }

    /// The API Automatic stands for, as the notes name it: the one the platform has, Metal
    /// on the Mac and Vulkan on Linux; on Windows, where it loads both, the one its own
    /// choice runs on once Automatic is listed, else none.
    fn api_name(
        &self,
        words: &Translator,
        backend: Backend,
        os: OperatingSystem,
    ) -> Option<String> {
        match (backend, os) {
            (Backend::Auto, OperatingSystem::Mac) => Some(in_use_api(words, wgpu::Backend::Metal)),
            (Backend::Auto, OperatingSystem::Nix) => Some(in_use_api(words, wgpu::Backend::Vulkan)),
            (Backend::Auto, _) => {
                let Some(Ok(adapters)) = self.done(Backend::Auto) else {
                    return None;
                };
                let kinds: Vec<wgpu::DeviceType> = adapters.iter().map(|a| a.kind).collect();
                let at = gpu::automatic(&kinds, gpu::power_preference())?;
                Some(in_use_api(words, adapters[at].backend))
            }
            _ => Some(api_words(words, backend, os)),
        }
    }

    /// Whether `stored` names the software adapter: `cpu`, or the name of an adapter whose
    /// kind is software in any listing so far or in use, as a text the field kept before
    /// may; where none of those names it, a text holding WARP's or lavapipe's name.
    fn software(&self, stored: &str) -> bool {
        if stored.is_empty() {
            return false;
        }
        if stored.eq_ignore_ascii_case(gpu::CPU) {
            return true;
        }
        let in_use = self.running.in_use.iter().map(|in_use| {
            let kind = if in_use.software {
                wgpu::DeviceType::Cpu
            } else {
                wgpu::DeviceType::Other
            };
            (in_use.name.as_str(), kind)
        });
        let listed = self.listings.iter().flat_map(|(_, listing)| match listing {
            Listing::Done(Ok(adapters)) => adapters.as_slice(),
            _ => &[],
        });
        let known: Vec<(&str, wgpu::DeviceType)> = listed
            .map(|adapter| (adapter.name.as_str(), adapter.kind))
            .chain(in_use)
            .collect();
        if let Some(at) = gpu::matching(&known, stored) {
            return known[at].1 == wgpu::DeviceType::Cpu;
        }
        let stored = stored.to_lowercase();
        SOFTWARE_NAMES
            .iter()
            .any(|name| stored.contains(&name.to_lowercase()))
    }

    /// The API row's Automatic with what it picks in brackets: the backend of the adapter
    /// `stored` opens under Automatic, else of Automatic's own. Known only once Automatic
    /// was listed, as the row had it, since listing it otherwise would load a backend the
    /// setting leaves out.
    fn automatic_api(&self, words: &Translator, stored: &str) -> Option<String> {
        let Some(Ok(adapters)) = self.done(Backend::Auto) else {
            return None;
        };
        let opened = opened(adapters, stored);
        let at = opened.or_else(|| {
            let kinds: Vec<wgpu::DeviceType> = adapters.iter().map(|a| a.kind).collect();
            gpu::automatic(&kinds, gpu::power_preference())
        })?;
        let mut args = Args::new();
        args.set("api", in_use_api(words, adapters[at].backend));
        Some(words.text_with("settings-gpu-automatic-api", Some(&args)))
    }

    /// The listing of `backend` as it stands, started where there is none, picked up where
    /// its answer has come; a repaint asked for when it will say it is looking.
    fn shown(&mut self, ctx: &egui::Context, backend: Backend) -> Shown<'_> {
        let now = ctx.input(|i| i.time);
        self.start(ctx, backend, now);
        self.poll(backend);
        let Some((_, listing)) = self.listings.iter().find(|(b, _)| *b == backend) else {
            return Shown::Failed;
        };
        match listing {
            Listing::Pending { since, .. } => {
                let waited = now - *since;
                if waited < LOOKING_AFTER {
                    let left = Duration::from_secs_f64(LOOKING_AFTER - waited);
                    ctx.request_repaint_after(left);
                }
                Shown::Listing {
                    looking: waited >= LOOKING_AFTER,
                }
            }
            Listing::Done(Ok(adapters)) => Shown::Listed(adapters),
            Listing::Done(Err(_)) => Shown::Failed,
        }
    }

    /// Takes in `backend`'s answer, where its listing is under way and the answer has come.
    fn poll(&mut self, backend: Backend) {
        let listing = self.listings.iter_mut().find(|(b, _)| *b == backend);
        let Some((_, listing)) = listing else {
            return;
        };
        if let Listing::Pending { slot, .. } = listing {
            let answer = slot.lock().unwrap_or_else(|e| e.into_inner()).take();
            if let Some(listed) = answer {
                *listing = Listing::Done(listed);
            }
        }
    }

    /// Lists `backend` on a thread of its own where it is not listed or being listed;
    /// with no lister, as in a test that gives none, it is said not listed.
    fn start(&mut self, ctx: &egui::Context, backend: Backend, now: f64) {
        if self.listings.iter().any(|(b, _)| *b == backend) {
            return;
        }
        let Some(Lister(list)) = self.running.lister.clone() else {
            let none = Failure::Start("no lister".to_owned());
            self.listings.push((backend, Listing::Done(Err(none))));
            return;
        };
        let slot = Arc::new(Mutex::new(None));
        let (answer, ctx_) = (slot.clone(), ctx.clone());
        let started = std::thread::Builder::new()
            .name("adapters".into())
            .spawn(move || {
                let listed = list(backend);
                if let Err(failure) = &listed {
                    let api = backend.name();
                    tracing::warn!(target: "gpu", "graphics cards not listed with {api}: {failure}");
                }
                *answer.lock().unwrap_or_else(|e| e.into_inner()) = Some(listed);
                ctx_.request_repaint();
            });
        let listing = match started {
            Ok(_) => Listing::Pending { since: now, slot },
            Err(error) => Listing::Done(Err(Failure::Start(error.to_string()))),
        };
        self.listings.push((backend, listing));
    }

    /// `backend`'s listing, once done.
    fn done(&self, backend: Backend) -> Option<&Listed> {
        self.listings.iter().find_map(|(b, listing)| match listing {
            Listing::Done(listed) if *b == backend => Some(listed),
            _ => None,
        })
    }

    /// The person changed the API row from `from`, the card's value `stored`: once both
    /// `from`'s listing and the new API's are in ([`Hardware::settle`]), a card `from` has
    /// and the new API lacks goes back to Automatic; one found with neither stays, as any
    /// card not found.
    pub fn api_changed(&mut self, from: Backend, stored: &str) {
        self.dropped = None;
        self.dropping = (!stored.is_empty()).then(|| (stored.to_owned(), from));
    }

    /// The person changed the card's row: what an API change said of it goes.
    pub fn card_changed(&mut self) {
        self.dropped = None;
        self.dropping = None;
    }

    /// Another page is shown: what an API change said goes, and one still to say nothing.
    pub fn left(&mut self) {
        self.dropped = None;
        self.dropping = None;
    }

    /// Whether the card an API change may leave out goes back to Automatic, now the API
    /// the row had and `backend`'s listings are in: true where the first has it and the
    /// second does not, saying so on the row. Waits while either is under way; a listing
    /// that failed keeps the card.
    pub fn settle(&mut self, ctx: &egui::Context, words: &Translator, backend: Backend) -> bool {
        let Some((stored, from)) = self.dropping.clone() else {
            return false;
        };
        // An API never listed had nothing to leave out.
        if !self.listings.iter().any(|(listed, _)| *listed == from) {
            self.dropping = None;
            return false;
        }
        self.poll(from);
        if matches!(self.shown(ctx, backend), Shown::Listing { .. }) || self.done(from).is_none() {
            return false;
        }
        self.dropping = None;
        let had =
            matches!(self.done(from), Some(Ok(adapters)) if opened(adapters, &stored).is_some());
        let lacks =
            matches!(self.done(backend), Some(Ok(adapters)) if opened(adapters, &stored).is_none());
        if !(had && lacks) {
            return false;
        }
        let card = if self.software(&stored) {
            words.text("settings-gpu-cpu-short")
        } else {
            stored
        };
        let api = self.api_name(words, backend, ctx.os());
        let text = with_api(words, "settings-gpu-dropped", api.as_deref(), Some(&card));
        self.dropped = Some(text);
        true
    }

    /// Waits until every listing under way has its answer, so the next frame takes it.
    #[cfg(test)]
    pub fn wait(&self) {
        for (backend, _) in &self.listings {
            self.wait_for(*backend);
        }
    }

    /// Waits until `backend`'s listing, where under way, has its answer.
    #[cfg(test)]
    pub fn wait_for(&self, backend: Backend) {
        let deadline = Instant::now() + Duration::from_secs(60);
        for (_, listing) in self.listings.iter().filter(|(b, _)| *b == backend) {
            if let Listing::Pending { slot, .. } = listing {
                while slot.lock().unwrap_or_else(|e| e.into_inner()).is_none() {
                    assert!(Instant::now() < deadline, "a listing never answered");
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        }
    }
}

/// Which of `adapters` `stored` opens, by the engine's own rule; none for an empty one.
fn opened(adapters: &[Adapter], stored: &str) -> Option<usize> {
    if stored.is_empty() {
        return None;
    }
    let listed: Vec<(&str, wgpu::DeviceType)> = adapters
        .iter()
        .map(|adapter| (adapter.name.as_str(), adapter.kind))
        .collect();
    gpu::matching(&listed, stored)
}

/// The words of `backend`, as the API row and the notes name it: on Automatic, the API it
/// means where the platform has one, Metal on the Mac and Vulkan on Linux.
pub fn api_words(words: &Translator, backend: Backend, os: OperatingSystem) -> String {
    match (backend, os) {
        (Backend::Auto, OperatingSystem::Mac) => in_use_api(words, wgpu::Backend::Metal),
        (Backend::Auto, OperatingSystem::Nix) => in_use_api(words, wgpu::Backend::Vulkan),
        _ => words.text(&format!("setting-engine-backend-option-{}", backend.name())),
    }
}

/// The words of the API `backend` the engine runs on: Direct3D 12, Vulkan, Metal, OpenGL.
pub fn in_use_api(words: &Translator, backend: wgpu::Backend) -> String {
    match backend {
        wgpu::Backend::Dx12 => words.text("setting-engine-backend-option-dx12"),
        wgpu::Backend::Vulkan => words.text("setting-engine-backend-option-vulkan"),
        wgpu::Backend::Metal => words.text("settings-gpu-api-metal"),
        wgpu::Backend::Gl => words.text("settings-gpu-api-gl"),
        other => other.to_str().to_owned(),
    }
}

/// The graphics card's list, as its row draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct Card {
    /// Each item's words, in order.
    pub words: Vec<String>,
    /// What choosing each item stores: none for Automatic, which is a reset.
    stores: Vec<Option<String>>,
    /// The item the stored value opens, checked.
    pub selected: Option<usize>,
    /// The item the separator comes before: CPU.
    pub separator: Option<usize>,
    /// An item that cannot be chosen, and why: CPU where the API has none.
    pub missing: Option<(usize, String)>,
    /// The stored value's item where it opens nothing, chosen and disabled, and why.
    pub unlisted: Option<(usize, String)>,
    /// Whether the cards are being listed, the box disabled meanwhile.
    pub listing: bool,
    /// What is said under the row.
    pub note: Option<Note>,
}

impl Card {
    /// What choosing `item` stores: a reset for Automatic, else its value.
    pub fn stores(&self, item: usize) -> Option<&str> {
        self.stores.get(item)?.as_deref()
    }
}

/// The card's value, and whether it names the software adapter, by its kind where a
/// listing has it ([`Hardware::software`]), so a text kept before, WARP's or lavapipe's own
/// name, reads as CPU as `cpu` does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stored<'a> {
    pub text: &'a str,
    pub software: bool,
}

/// The words of the message `id` with `$api`, the API's name where it is known, else its
/// message without it, `<id>-unnamed`; and `$card` where given.
fn with_api(words: &Translator, id: &str, api: Option<&str>, card: Option<&str>) -> String {
    let mut args = Args::new();
    if let Some(card) = card {
        args.set("card", card);
    }
    match api {
        Some(api) => {
            args.set("api", api);
            words.text_with(id, Some(&args))
        }
        None => words.text_with(&format!("{id}-unnamed"), Some(&args)),
    }
}

/// The graphics card's list for the card's value `stored`, as `shown` lists the API whose
/// name is `api`, where known, on `os`, Automatic choosing for `preference`.
pub fn card(
    words: &Translator,
    stored: Stored<'_>,
    shown: Shown<'_>,
    api: Option<&str>,
    os: OperatingSystem,
    preference: wgpu::PowerPreference,
) -> Card {
    let Stored {
        text: stored,
        software: cpu_stored,
    } = stored;
    let automatic = words.text("settings-gpu-automatic");
    let cpu = words.text("settings-gpu-cpu");
    let adapters = match shown {
        Shown::Listing { looking } => {
            // The stored choice's words without brackets, the box disabled.
            let current = if stored.is_empty() {
                automatic
            } else if cpu_stored {
                cpu
            } else {
                stored.to_owned()
            };
            let note = looking.then(|| Note {
                text: words.text("settings-gpu-listing"),
                warning: false,
                live: false,
            });
            return Card {
                words: vec![current],
                stores: vec![None],
                selected: Some(0),
                separator: None,
                missing: None,
                unlisted: None,
                listing: true,
                note,
            };
        }
        Shown::Listed(adapters) => adapters,
        Shown::Failed => &[],
    };
    let kinds: Vec<wgpu::DeviceType> = adapters.iter().map(|a| a.kind).collect();
    let first = match gpu::automatic(&kinds, preference).map(|at| &adapters[at]) {
        Some(picked) if picked.kind == wgpu::DeviceType::Cpu => {
            words.text("settings-gpu-automatic-cpu")
        }
        Some(picked) => {
            let mut args = Args::new();
            args.set("card", picked.name.as_str());
            words.text_with("settings-gpu-automatic-card", Some(&args))
        }
        None => automatic,
    };
    // The hardware adapters in Automatic's order, ties as listed, each name once.
    let mut hardware: Vec<&Adapter> = adapters
        .iter()
        .filter(|a| a.kind != wgpu::DeviceType::Cpu)
        .collect();
    hardware.sort_by_key(|a| gpu::rank(a.kind, preference));
    let mut names: Vec<&str> = Vec::new();
    for adapter in hardware {
        if !names.contains(&adapter.name.as_str()) {
            names.push(&adapter.name);
        }
    }
    let mut card = Card {
        words: vec![first],
        stores: vec![None],
        selected: None,
        separator: None,
        missing: None,
        unlisted: None,
        listing: false,
        note: None,
    };
    for name in &names {
        card.words.push((*name).to_owned());
        card.stores.push(Some((*name).to_owned()));
    }
    let failed = shown == Shown::Failed;
    let not_listed = words.text("settings-gpu-not-listed");
    let missing_tip = with_api(words, "settings-gpu-cpu-missing", api, None);
    let opened = opened(adapters, stored).map(|at| &adapters[at]);
    let software = adapters.iter().any(|a| a.kind == wgpu::DeviceType::Cpu);
    // The stored value's item: Automatic, a card, CPU, or one it does not find.
    let mut cpu_chosen = false;
    if stored.is_empty() {
        card.selected = Some(0);
    } else if let Some(adapter) = opened.filter(|a| a.kind != wgpu::DeviceType::Cpu) {
        card.selected = names
            .iter()
            .position(|n| *n == adapter.name)
            .map(|at| at + 1);
    } else if opened.is_some() || cpu_stored {
        cpu_chosen = true;
    } else {
        // Not found, or the cards not listed: kept as it is, listed disabled and checked.
        let at = card.words.len();
        card.words.push(stored.to_owned());
        card.stores.push(Some(stored.to_owned()));
        card.selected = Some(at);
        let tip = if failed {
            not_listed.clone()
        } else {
            words.text("settings-gpu-not-found-item")
        };
        card.unlisted = Some((at, tip));
    }
    // CPU, after a separator; on the Mac, whose Metal has none, only where it is stored.
    if os != OperatingSystem::Mac || cpu_chosen {
        let at = card.words.len();
        card.separator = Some(at);
        card.words.push(cpu);
        card.stores.push(Some(gpu::CPU.to_owned()));
        if cpu_chosen {
            card.selected = Some(at);
        }
        if !software {
            let tip = if failed {
                not_listed.clone()
            } else {
                missing_tip
            };
            if cpu_chosen {
                card.unlisted = Some((at, tip));
            } else {
                card.missing = Some((at, tip));
            }
        }
    }
    card.note = if failed {
        Some(Note::warning(not_listed))
    } else if card.unlisted.is_some() {
        Some(Note::warning(words.text("settings-gpu-not-found")))
    } else {
        None
    };
    card
}

/// The labels the group's own lines take, for the label column's width.
pub fn labels(words: &Translator) -> Vec<String> {
    vec![words.text("settings-gpu-in-use")]
}

/// The In use line, as Storage's lines: the adapter and the API the engine opened, CPU for
/// a software adapter, with a warning under it where that is one, and the driver advice
/// where Automatic fell to it.
pub fn in_use(ui: &mut Ui, t: &Theme, in_use: &InUse, widths: storage::Widths) {
    let words = apertrust_ui::translator(ui.ctx());
    let label = words.text("settings-gpu-in-use");
    let card = if in_use.software {
        words.text("settings-gpu-cpu-short")
    } else {
        in_use.name.clone()
    };
    let mut args = Args::new();
    args.set("card", card.as_str());
    args.set("api", in_use_api(&words, in_use.backend));
    let value = words.text_with("settings-gpu-in-use-value", Some(&args));
    let note = in_use.software.then(|| {
        let warning = words.text("settings-gpu-software");
        if in_use.chosen {
            warning
        } else {
            format!("{warning}\n{}", words.text("settings-gpu-driver"))
        }
    });
    storage::Line {
        label: &label,
        value: &value,
        cut: storage::Cut::End,
        note: note.as_deref(),
        warning: true,
        button: None,
    }
    .draw(ui, t, widths);
}

#[cfg(test)]
pub(crate) mod tests;
