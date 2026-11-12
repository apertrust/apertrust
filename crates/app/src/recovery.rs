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

//! A start that did not finish, and `--safe-mode` (decision 81): the way back from a
//! graphics card or API that stops the app, since Settings, where they are chosen, is out
//! of reach once it cannot start.
//!
//! - **The marker** ([`Marker`]): before the app opens its graphics device it opens
//!   `starting.toml` in the user data folder and holds an exclusive lock on it for the
//!   whole start, naming in it the card and the API it tries ([`Tried`]): the settings'
//!   before anything loads a driver, then what the engine's settings resolve them to. Once
//!   its first frame has been drawn it empties and removes it ([`Pending`]). A run that
//!   opens no window (`--version`, `--help`, `--list-adapters`, a command line refused)
//!   writes none, and a normal quit leaves none. Where the user data folder cannot be
//!   written, there is no marker, logged under `settings`.
//! - **A marker another start holds locked** is a start under way at the same moment, as
//!   Open With on several raws starts one process for each: it is neither read nor written,
//!   and this start keeps none.
//! - **A marker unlocked with something in it** is a start that did not finish, having
//!   crashed, hung or been closed with a black window ([`decide`]): this run opens
//!   Automatic instead, the settings kept in the file, and says so in the window
//!   ([`Notice`]), with Keep Automatic, which resets the two settings, and Try again, which
//!   restarts with them. Where the marker names Automatic itself, it opens the CPU, with
//!   Keep CPU and Try again; where the API has no CPU adapter, wgpu's choice opens again,
//!   and the notice says so.
//! - **`--safe-mode`** opens Automatic for this run with nothing said, as darktable's
//!   `--disable-opencl` does for OpenCL, whatever a marker says.
//!
//! The notice is drawn in the window, over its top, under the top panel or the start
//! window's strip, on every platform: Windows shows no alert yet (`docs/gui.md`, The
//! alerts), and the notice asks for nothing, the app running meanwhile.

use std::fs::{File, OpenOptions, TryLockError};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::PathBuf;
use std::thread::JoinHandle;

use apertrust_base::settings::{Choice as _, Declared};
use apertrust_engine::gpu::{self, AdapterChoice, Backend};
use apertrust_engine::settings::{BACKEND_VARIABLE, Options, Source};
use apertrust_ui::components::Button;
use apertrust_ui::i18n::Args;
use apertrust_ui::{Translator, theme};
use eframe::egui::{self, os::OperatingSystem};
use toml::Value;

use crate::settings_dialog::Change;

/// Why a run opens Automatic or the CPU in place of the settings, as `-d settings` says.
pub const UNFINISHED: &str = "a start that did not finish";

/// The graphics card and API settings, which such a run and `--safe-mode` set aside.
pub fn graphics() -> [&'static dyn Declared; 2] {
    use apertrust_engine::settings::registry::{ENGINE_ADAPTER, ENGINE_BACKEND};
    [&ENGINE_ADAPTER, &ENGINE_BACKEND]
}

/// The graphics card and API a start tries: the card as `engine.adapter` writes it (empty
/// for Automatic, `cpu` for the CPU), the API as `engine.backend` chooses it, and
/// `WGPU_BACKEND`'s text where that chose the API in its place.
#[derive(Clone, Debug, PartialEq)]
pub struct Tried {
    pub adapter: String,
    pub backend: Backend,
    pub variable: Option<String>,
}

impl Tried {
    /// Automatic, both settings at their defaults.
    pub fn automatic() -> Tried {
        Tried::of(String::new(), Backend::Auto)
    }

    /// `adapter` with `backend`, as the settings name them.
    pub fn of(adapter: String, backend: Backend) -> Tried {
        Tried {
            adapter,
            backend,
            variable: None,
        }
    }

    /// What `options`, the engine's settings read, open: the card they name after their
    /// fallbacks, and the API, `WGPU_BACKEND`'s where it came first.
    pub fn opened(options: &Options) -> Tried {
        let adapter = match &options.gpu.adapter {
            AdapterChoice::Any => String::new(),
            AdapterChoice::Named(name) | AdapterChoice::Preferred(name) => name.clone(),
        };
        let variable = matches!(options.sources[2], Source::Variable(_))
            .then(|| std::env::var(BACKEND_VARIABLE).ok())
            .flatten();
        Tried {
            adapter,
            backend: options.gpu.backend,
            variable,
        }
    }

    fn is_automatic(&self) -> bool {
        *self == Tried::automatic()
    }

    /// The marker's text.
    fn text(&self) -> String {
        let mut table = toml::Table::new();
        table.insert("adapter".into(), Value::String(self.adapter.clone()));
        table.insert("backend".into(), Value::String(self.backend.name().into()));
        if let Some(variable) = &self.variable {
            table.insert("variable".into(), Value::String(variable.clone()));
        }
        format!(
            "# Apertrust is opening its graphics device with these; it removes this file once \
             its window has drawn.\n{table}"
        )
    }

    /// What a marker's `text` names; none for a text that is no marker.
    fn read(text: &str) -> Option<Tried> {
        let table: toml::Table = text.parse().ok()?;
        let adapter = table.get("adapter")?.as_str()?.to_owned();
        let backend = Backend::from_id(table.get("backend")?.as_str()?)?;
        let variable = table
            .get("variable")
            .and_then(Value::as_str)
            .map(str::to_owned);
        Some(Tried {
            adapter,
            backend,
            variable,
        })
    }
}

/// This start's marker, `starting.toml` in the user data folder (`Folders::starting`),
/// open and locked for it until it is removed or the process ends.
#[derive(Debug)]
pub struct Marker {
    path: PathBuf,
    file: File,
}

/// What opening the marker found.
#[derive(Debug)]
pub enum Opened {
    /// This start's, locked: with what the last start tried, where it did not finish.
    Mine(Marker, Option<Tried>),
    /// Another start holds it, under way at the same moment: neither read nor written.
    Busy,
    /// None can be kept: the user data folder cannot be written.
    Nothing,
}

impl Marker {
    /// Opens the marker at `path`, making it where there is none, and locks it for this
    /// start: what is in it is a start that did not finish; one another start has locked
    /// is that start's, under way.
    pub fn open(path: PathBuf) -> Opened {
        let name = apertrust_base::log::file_name(&path);
        let opened = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path);
        let mut file = match opened {
            Ok(file) => file,
            Err(error) => {
                tracing::debug!(target: "settings", "{name}: not opened, no start marker: {error}");
                return Opened::Nothing;
            }
        };
        match file.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                tracing::debug!(target: "settings", "{name}: another start holds it; none for this one");
                return Opened::Busy;
            }
            Err(TryLockError::Error(error)) => {
                tracing::debug!(target: "settings", "{name}: not locked, no start marker: {error}");
                return Opened::Nothing;
            }
        }
        let mut text = String::new();
        let _ = file.read_to_string(&mut text);
        let found = (!text.trim().is_empty())
            .then(|| Tried::read(&text))
            .flatten();
        if found.is_none() && !text.trim().is_empty() {
            tracing::warn!(target: "settings", "{name}: not a start marker; replaced");
        }
        if let Some(tried) = &found {
            let (card, api) = (&tried.adapter, tried.backend.name());
            tracing::debug!(target: "settings", "{name}: the last start did not finish: {card:?} on {api}");
        }
        Opened::Mine(Marker { path, file }, found)
    }

    /// Names `tried` in it, in place of what it named; returns whether it could.
    pub fn write(&mut self, tried: &Tried) -> bool {
        let file = &mut self.file;
        let written = file
            .set_len(0)
            .and_then(|()| file.seek(SeekFrom::Start(0)))
            .and_then(|_| file.write_all(tried.text().as_bytes()))
            .and_then(|()| file.sync_data());
        let name = apertrust_base::log::file_name(&self.path);
        match written {
            Ok(()) => {
                let (card, api) = (&tried.adapter, tried.backend.name());
                tracing::debug!(target: "settings", "{name}: tries {card:?} on {api}");
                true
            }
            Err(error) => {
                tracing::debug!(target: "settings", "{name}: not written: {error}");
                false
            }
        }
    }

    /// Empties it and removes it, its lock held until it has gone, so a start that opened
    /// it meanwhile finds nothing to take for a start that did not finish.
    pub fn remove(self) {
        let name = apertrust_base::log::file_name(&self.path);
        let _ = self.file.set_len(0);
        match std::fs::remove_file(&self.path) {
            Ok(()) => tracing::debug!(target: "settings", "{name}: removed, the window drew"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => tracing::debug!(target: "settings", "{name}: emptied, not removed: {e}"),
        }
        drop(self.file);
    }
}

/// What this run opens its graphics device with.
#[derive(Clone, Debug, PartialEq)]
pub enum Run {
    /// As the settings say.
    Settings,
    /// Automatic, by `--safe-mode`, saying nothing.
    SafeMode,
    /// Automatic, the start that tried this having not finished.
    Automatic(Tried),
    /// The CPU, the start that tried Automatic having not finished.
    Cpu,
}

/// As the app starts, before anything loads a graphics driver: what this run opens, by the
/// marker at `path` and `safe_mode` ([`decide`]), and this start's marker, locked and
/// naming what it tries, the settings' `graphics` where they decide; none where there is
/// no user data folder, it cannot be written or another start holds it.
pub fn begin(path: Option<PathBuf>, safe_mode: bool, graphics: Tried) -> (Run, Option<Marker>) {
    let (marker, found) = match path.map(Marker::open) {
        Some(Opened::Mine(marker, found)) => (Some(marker), found),
        _ => (None, None),
    };
    let run = decide(found, safe_mode);
    let tries = run.tries(graphics);
    let marker = marker.and_then(|mut marker| marker.write(&tries).then_some(marker));
    (run, marker)
}

/// What this run opens, the last start's marker `found` or none, `safe_mode` given or not:
/// `--safe-mode` first, as it means Automatic already; then a marker, as it says what
/// failed, Automatic failing too giving the CPU.
pub fn decide(found: Option<Tried>, safe_mode: bool) -> Run {
    match found {
        _ if safe_mode => Run::SafeMode,
        Some(tried) if tried.is_automatic() => Run::Cpu,
        Some(tried) => Run::Automatic(tried),
        None => Run::Settings,
    }
}

impl Run {
    /// The card it opens in place of the settings, with Automatic's API, and why: none
    /// where the settings decide.
    pub fn instead(&self) -> Option<(&'static str, &'static str)> {
        match self {
            Run::Settings => None,
            Run::SafeMode => Some(("", crate::restart::SAFE_MODE_FLAG)),
            Run::Automatic(_) => Some(("", UNFINISHED)),
            Run::Cpu => Some((gpu::CPU, UNFINISHED)),
        }
    }

    /// What it tries before the engine's settings are read: the settings', `graphics`,
    /// where they decide.
    pub fn tries(&self, graphics: Tried) -> Tried {
        match self.instead() {
            None => graphics,
            Some((adapter, _)) => Tried::of(adapter.to_owned(), Backend::Auto),
        }
    }

    /// What it says in the window, `software` whether what opened is the CPU: the CPU
    /// where it opened, else Automatic again, where the API has none.
    pub fn notice(&self, software: bool) -> Option<Notice> {
        match self {
            Run::Settings | Run::SafeMode => None,
            Run::Automatic(tried) => Some(Notice::Automatic(tried.clone())),
            Run::Cpu if software => Some(Notice::Cpu),
            Run::Cpu => Some(Notice::AutomaticAgain),
        }
    }
}

/// What the window says after a start that did not finish.
#[derive(Clone, Debug, PartialEq)]
pub enum Notice {
    /// This did not start: Automatic runs.
    Automatic(Tried),
    /// Automatic did not start: the CPU runs.
    Cpu,
    /// Automatic did not start, and the API has no CPU adapter: Automatic runs again.
    AutomaticAgain,
}

/// A notice's button pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pressed {
    /// Keep Automatic, or Keep CPU: the settings set to what runs.
    Keep,
    /// Try again: the app restarted with the settings.
    Retry,
    /// Close, where there is nothing to keep or try.
    Close,
}

impl Notice {
    /// Its words, the card and the API named as the Settings dialog names them on `os`.
    pub fn words(&self, words: &Translator, os: OperatingSystem) -> String {
        let tried = match self {
            Notice::Automatic(tried) => tried,
            Notice::Cpu => return words.text("start-failed-automatic"),
            Notice::AutomaticAgain => return words.text("start-failed-automatic-again"),
        };
        let mut args = Args::new();
        let card = match tried.adapter.as_str() {
            "" => None,
            cpu if cpu.eq_ignore_ascii_case(gpu::CPU) => Some(words.text("settings-gpu-cpu-short")),
            name => Some(name.to_owned()),
        };
        let api = match (&tried.variable, tried.backend) {
            (Some(variable), _) => Some(format!("{BACKEND_VARIABLE}={variable}")),
            (None, Backend::Auto) => None,
            (None, backend) => Some(crate::hardware::api_words(words, backend, os)),
        };
        let id = match (card, api) {
            (Some(card), Some(api)) => {
                args.set("card", card);
                args.set("api", api);
                "start-failed"
            }
            (Some(card), None) => {
                args.set("card", card);
                "start-failed-card"
            }
            (None, api) => {
                args.set("api", api.unwrap_or_default());
                "start-failed-api"
            }
        };
        words.text_with(id, Some(&args))
    }

    /// Its buttons' words, each with what pressing it asks.
    fn buttons(&self, words: &Translator) -> Vec<(String, Pressed)> {
        let retry = (words.text("start-failed-retry"), Pressed::Retry);
        match self {
            Notice::Automatic(_) => vec![
                (words.text("start-failed-keep-automatic"), Pressed::Keep),
                retry,
            ],
            Notice::Cpu => vec![(words.text("start-failed-keep-cpu"), Pressed::Keep), retry],
            Notice::AutomaticAgain => vec![(words.text("start-failed-close"), Pressed::Close)],
        }
    }

    /// What Keep sets: the card and the API to what runs, Automatic's or the CPU's.
    pub fn kept(&self) -> Vec<Change> {
        use apertrust_engine::settings::registry::{ENGINE_ADAPTER, ENGINE_BACKEND};
        let card = match self {
            Notice::Cpu => Change::Set(ENGINE_ADAPTER.id, Value::String(gpu::CPU.into())),
            Notice::Automatic(_) | Notice::AutomaticAgain => Change::Reset(ENGINE_ADAPTER.id),
        };
        vec![card, Change::Reset(ENGINE_BACKEND.id)]
    }

    /// Draws it over the window's top, `top` points down, under its top panel or strip, in
    /// `ctx`: its words, announced once as they appear (a polite live region), then its
    /// buttons. Returns the button pressed.
    pub fn show(&self, ctx: &egui::Context, top: f32) -> Option<Pressed> {
        let t = theme(ctx);
        let words = apertrust_ui::translator(ctx);
        let said = self.words(&words, ctx.os());
        let buttons = self.buttons(&words);
        let mut pressed = None;
        egui::Area::new(egui::Id::unique("start notice"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, top + t.spacing.l))
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(t.colors.raised)
                    .stroke(egui::Stroke::new(t.shape.separator_width, t.colors.divider))
                    .corner_radius(t.shape.radius)
                    .inner_margin(egui::Margin::same(t.spacing.l as i8))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(t.spacing.m, t.spacing.m);
                        let width = WIDTH.min(ctx.content_rect().width() - 4.0 * t.spacing.l);
                        ui.set_max_width(width.max(0.0));
                        let galley = ui.painter().layout(
                            said.clone(),
                            egui::FontId::proportional(t.text.body),
                            t.colors.text,
                            width.max(0.0),
                        );
                        let (rect, label) =
                            ui.allocate_exact_size(galley.size(), egui::Sense::hover());
                        ui.painter().galley(rect.min, galley, t.colors.text);
                        label.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::Role::Label, true, &said)
                        });
                        let polite = egui::accesskit::Live::Polite;
                        ctx.accesskit_node_builder(label.id, |node| node.set_live(polite));
                        ui.horizontal(|ui| {
                            for (text, asks) in &buttons {
                                if Button::new(text).show(ui).clicked() {
                                    pressed = Some(*asks);
                                }
                            }
                        });
                    });
            });
        pressed
    }
}

/// The notice's widest, in points.
const WIDTH: f32 = 480.0;

/// This run's marker until its first frame has been drawn, then its removal.
#[derive(Debug, Default)]
pub struct Pending {
    marker: Option<Marker>,
    removing: Option<JoinHandle<()>>,
}

impl Pending {
    /// `marker`, written as this run started, or none.
    pub fn new(marker: Option<Marker>) -> Pending {
        Pending {
            marker,
            removing: None,
        }
    }

    /// Before each frame in `ctx`: once one has been drawn, the marker is removed, on a
    /// thread of its own, as no file is written on the interface's thread; until then a
    /// frame is asked for, so one follows the first.
    pub fn frame(&mut self, ctx: &egui::Context) {
        if self.marker.is_none() {
            return;
        }
        if ctx.cumulative_frame_nr() == 0 {
            ctx.request_repaint();
            return;
        }
        let Some(marker) = self.marker.take() else {
            return;
        };
        let removing = std::thread::Builder::new()
            .name("start marker".into())
            .spawn(move || marker.remove());
        match removing {
            Ok(handle) => self.removing = Some(handle),
            Err(error) => tracing::warn!(target: "settings", "start marker not removed: {error}"),
        }
    }

    /// The start ends before its window could open for a reason that is not the graphics
    /// device's, as its event loop failing to start: the marker goes at once.
    pub fn abandon(&mut self) {
        if let Some(marker) = self.marker.take() {
            marker.remove();
        }
    }

    /// As the app ends: the removal under way finished, so a quit soon after the start
    /// never leaves the marker behind.
    pub fn finish(&mut self) {
        if let Some(handle) = self.removing.take() {
            let _ = handle.join();
        }
    }

    /// Whether the marker waits for the first frame.
    #[cfg(test)]
    pub fn waiting(&self) -> bool {
        self.marker.is_some()
    }
}

#[cfg(test)]
mod tests {
    use apertrust_ui::Locale;

    use super::*;
    use crate::settings::testing::Folder;

    const RADEON: &str = "AMD Radeon 780M Graphics";

    fn tried(adapter: &str, backend: Backend) -> Tried {
        Tried::of(adapter.to_owned(), backend)
    }

    /// The marker at `path`, as this start's, with what it found.
    fn mine(path: &std::path::Path) -> (Marker, Option<Tried>) {
        match Marker::open(path.to_owned()) {
            Opened::Mine(marker, found) => (marker, found),
            other => panic!("not this start's: {other:?}"),
        }
    }

    /// A marker written names what it tried; a start that ends without removing it, as a
    /// crash does, leaves it for the next to find; one removed, or emptied, is none; one
    /// that is not a marker is none, and replaced.
    #[test]
    fn a_marker_names_the_card_and_api_it_tried() {
        let folder = Folder::new("start-marker");
        let path = folder.0.join("starting.toml");
        let (mut marker, found) = mine(&path);
        assert_eq!(found, None);
        let radeon = tried(RADEON, Backend::Dx12);
        assert!(marker.write(&radeon));
        drop(marker);
        let text = std::fs::read_to_string(&path).expect("written");
        assert!(
            text.contains("adapter = \"AMD Radeon 780M Graphics\""),
            "{text}"
        );
        assert!(text.contains("backend = \"dx12\""), "{text}");
        let (marker, found) = mine(&path);
        assert_eq!(found, Some(radeon));
        marker.remove();
        assert!(!path.exists());
        assert_eq!(mine(&path).1, None);
        std::fs::write(&path, "not = [a marker").expect("written");
        assert_eq!(mine(&path).1, None);
        // `WGPU_BACKEND`'s text, where it chose the API, is named too.
        let mut variable = tried("", Backend::Auto);
        variable.variable = Some("gl".into());
        let (mut marker, _) = mine(&path);
        assert!(marker.write(&variable));
        drop(marker);
        assert_eq!(mine(&path).1, Some(variable));
    }

    /// Two starts at once, as Open With on several raws starts: the second finds the
    /// marker locked by the first, under way, and neither reads it as a start that did not
    /// finish nor writes its own; the first's lock goes with it, and only then is what it
    /// left read.
    #[test]
    fn a_marker_another_start_holds_is_that_start_s_under_way() {
        let folder = Folder::new("start-marker-two");
        let path = folder.0.join("starting.toml");
        let (mut first, _) = mine(&path);
        assert!(first.write(&tried(RADEON, Backend::Dx12)));
        assert!(matches!(Marker::open(path.clone()), Opened::Busy));
        let graphics = || tried(RADEON, Backend::Dx12);
        let (run, second) = begin(Some(path.clone()), false, graphics());
        assert_eq!(run, Run::Settings);
        assert!(second.is_none());
        // The first draws its window and removes it: the next start finds nothing.
        first.remove();
        let (run, third) = begin(Some(path.clone()), false, graphics());
        assert_eq!(run, Run::Settings);
        assert!(third.is_some());
    }

    /// A user data folder that cannot be written has no marker, and nothing stops: the
    /// start goes on as without one.
    #[test]
    fn a_folder_that_cannot_be_written_has_no_marker() {
        let folder = Folder::new("start-marker-read-only");
        // A file where the folder should be: nothing can be written in it, on every
        // platform, as in a read-only one.
        let blocked = folder.0.join("blocked");
        std::fs::write(&blocked, b"").expect("a file");
        let path = blocked.join("starting.toml");
        assert!(matches!(Marker::open(path.clone()), Opened::Nothing));
        let (run, marker) = begin(Some(path), false, Tried::automatic());
        assert_eq!(run, Run::Settings);
        assert!(marker.is_none());
        let mut pending = Pending::new(None);
        pending.frame(&egui::Context::default());
        pending.abandon();
        pending.finish();
    }

    /// `--safe-mode` first, saying nothing; else a marker found: Automatic in place of the
    /// card and API it names, the CPU in place of Automatic, Automatic again where the
    /// CPU did not open; else the settings decide.
    #[test]
    fn a_start_after_one_that_did_not_finish_takes_automatic_or_the_cpu() {
        let radeon = tried(RADEON, Backend::Vulkan);
        let graphics = || tried(RADEON, Backend::Dx12);
        let run = decide(Some(radeon.clone()), false);
        assert_eq!(run, Run::Automatic(radeon.clone()));
        assert_eq!(run.instead(), Some(("", UNFINISHED)));
        assert_eq!(run.tries(graphics()), Tried::automatic());
        assert_eq!(run.notice(false), Some(Notice::Automatic(radeon.clone())));
        let run = decide(Some(Tried::automatic()), false);
        assert_eq!(run, Run::Cpu);
        assert_eq!(run.instead(), Some((gpu::CPU, UNFINISHED)));
        assert_eq!(run.tries(graphics()), tried("cpu", Backend::Auto));
        assert_eq!(run.notice(true), Some(Notice::Cpu));
        assert_eq!(run.notice(false), Some(Notice::AutomaticAgain));
        // The API alone, or the card alone, is not Automatic.
        let api = tried("", Backend::Dx12);
        assert_eq!(decide(Some(api.clone()), false), Run::Automatic(api));
        for found in [None, Some(radeon), Some(Tried::automatic())] {
            let safe = decide(found, true);
            assert_eq!(safe, Run::SafeMode);
            assert_eq!(safe.instead(), Some(("", "--safe-mode")));
            assert_eq!(safe.notice(false), None);
            assert_eq!(safe.tries(graphics()), Tried::automatic());
        }
        let settings = decide(None, false);
        assert_eq!((settings.instead(), settings.notice(false)), (None, None));
        assert_eq!(settings.tries(graphics()), tried(RADEON, Backend::Dx12));
    }

    /// Start after start, as `main` begins each before the device opens: the settings'
    /// card written; that start not finishing, Automatic and its own marker; that not
    /// finishing either, the CPU; a start whose window drew leaves none, and the settings
    /// decide again.
    #[test]
    fn each_start_writes_what_it_tries_before_the_device_and_the_next_reads_it() {
        let folder = Folder::new("start-marker-starts");
        let path = folder.0.join("starting.toml");
        let graphics = || tried(RADEON, Backend::Dx12);
        let begin = |safe| begin(Some(path.clone()), safe, graphics());
        let (run, written) = begin(false);
        assert_eq!(run, Run::Settings);
        // It crashed: no frame drawn, the marker left, its lock gone with it.
        drop(written);
        let (run, written) = begin(false);
        assert_eq!(run, Run::Automatic(tried(RADEON, Backend::Dx12)));
        drop(written);
        let (run, written) = begin(false);
        assert_eq!(run, Run::Cpu);
        // That one drew its window.
        written.expect("written").remove();
        let (run, written) = begin(true);
        assert_eq!(run, Run::SafeMode);
        drop(written);
        // `--safe-mode` again over a marker left: no notice, and its own marker.
        let (run, _) = begin(true);
        assert_eq!(run, Run::SafeMode);
        // No user data folder: no marker, the settings decide.
        let (run, marker) = super::begin(None, false, graphics());
        assert_eq!((run, marker.is_none()), (Run::Settings, true));
    }

    /// The notice names the card and the API, CPU for `cpu`, and leaves out the one that
    /// was Automatic; Keep sets the settings to what runs.
    #[test]
    fn the_notice_says_what_did_not_start_and_keep_sets_what_runs() {
        use apertrust_engine::settings::registry::{ENGINE_ADAPTER, ENGINE_BACKEND};
        let words = Translator::new(Locale::EnGb);
        let os = OperatingSystem::Windows;
        let said = |tried: Tried| Notice::Automatic(tried).words(&words, os);
        assert_eq!(
            said(tried(RADEON, Backend::Dx12)),
            "Apertrust did not start last time with AMD Radeon 780M Graphics on Direct3D 12, \
             so it started with Automatic"
        );
        assert_eq!(
            said(tried(RADEON, Backend::Auto)),
            "Apertrust did not start last time with AMD Radeon 780M Graphics, so it started \
             with Automatic"
        );
        assert_eq!(
            said(tried("cpu", Backend::Vulkan)),
            "Apertrust did not start last time with CPU on Vulkan, so it started with \
             Automatic"
        );
        assert_eq!(
            said(tried("", Backend::Vulkan)),
            "Apertrust did not start last time on Vulkan, so it started with Automatic"
        );
        assert_eq!(
            Notice::Cpu.words(&words, os),
            "Apertrust did not start last time with Automatic, so it started on the CPU, \
             which is slower"
        );
        assert_eq!(
            Notice::AutomaticAgain.words(&words, os),
            "Apertrust did not start last time with Automatic, and this graphics API has no \
             CPU adapter, so it started with Automatic again"
        );
        let reset = [
            Change::Reset(ENGINE_ADAPTER.id),
            Change::Reset(ENGINE_BACKEND.id),
        ];
        assert_eq!(
            Notice::Automatic(tried(RADEON, Backend::Dx12)).kept(),
            reset
        );
        let cpu = Change::Set(ENGINE_ADAPTER.id, "cpu".into());
        assert_eq!(Notice::Cpu.kept(), [cpu, Change::Reset(ENGINE_BACKEND.id)]);
        // In every language.
        for locale in [Locale::EnGb, Locale::EnUs, Locale::Uk, Locale::Pl] {
            let words = Translator::new(locale);
            for id in [
                "start-failed",
                "start-failed-card",
                "start-failed-api",
                "start-failed-automatic",
                "start-failed-automatic-again",
                "start-failed-keep-automatic",
                "start-failed-keep-cpu",
                "start-failed-retry",
                "start-failed-close",
            ] {
                assert!(words.has(id), "{locale:?}: {id}");
            }
        }
    }

    /// The notice in a window, as the app draws it, and what was pressed in it.
    fn notice_in(notice: Notice) -> egui_kittest::Harness<'static, (Notice, Vec<Pressed>)> {
        egui_kittest::Harness::builder()
            .with_size(egui::vec2(820.0, 240.0))
            .with_os(OperatingSystem::Windows)
            .with_context_setup(|ctx| {
                let dark = apertrust_ui::Theme::builtin(apertrust_ui::Builtin::Dark);
                apertrust_ui::install(ctx, dark);
                apertrust_ui::set_locale(ctx, Locale::EnGb);
            })
            .build_ui_state(
                |ui, (notice, pressed): &mut (Notice, Vec<Pressed>)| {
                    // Over the photo's surround, as over the main window.
                    let t = theme(ui.ctx());
                    let all = ui.ctx().content_rect();
                    ui.painter().rect_filled(all, 0.0, t.colors.photo_surround);
                    pressed.extend(notice.show(ui.ctx(), 40.0));
                },
                (notice, Vec::new()),
            )
    }

    /// Screen readers hear the words once, as they appear (a polite live region), then
    /// Keep and Try again, each a button that says what it does; where the CPU did not
    /// open, Close alone, and no Keep CPU.
    #[test]
    fn the_notice_is_read_then_its_buttons() {
        use egui_kittest::kittest::{NodeT as _, Queryable as _};
        let mut harness = notice_in(Notice::Automatic(tried(RADEON, Backend::Dx12)));
        harness.run();
        let said = "Apertrust did not start last time with AMD Radeon 780M Graphics on \
                    Direct3D 12, so it started with Automatic";
        let label = harness.get_by_role_and_label(egui::Role::Label, said);
        let polite = egui::accesskit::Live::Polite;
        assert_eq!(label.accesskit_node().data().live(), Some(polite));
        harness
            .get_by_role_and_label(egui::Role::Button, "Keep Automatic")
            .click();
        harness.run();
        harness
            .get_by_role_and_label(egui::Role::Button, "Try again")
            .click();
        harness.run();
        assert_eq!(harness.state().1, [Pressed::Keep, Pressed::Retry]);
        let harness = notice_in(Notice::Cpu);
        harness.get_by_role_and_label(egui::Role::Button, "Keep CPU");
        let mut harness = notice_in(Notice::AutomaticAgain);
        harness.run();
        assert!(harness.query_by_label("Keep CPU").is_none());
        harness
            .get_by_role_and_label(egui::Role::Button, "Close")
            .click();
        harness.run();
        assert_eq!(harness.state().1, [Pressed::Close]);
    }

    /// Over the window's top, under its 40-point strip.
    #[test]
    fn the_notice_in_the_dark_theme() {
        let mut harness = notice_in(Notice::Automatic(tried(RADEON, Backend::Dx12)));
        harness.run();
        harness.snapshot("start-failed-notice-dark");
    }

    /// The marker is removed once the first frame has been drawn, not before: a frame is
    /// asked for after the first, and the removal is waited for as the app ends.
    #[test]
    fn the_marker_goes_once_the_first_frame_is_drawn() {
        let folder = Folder::new("start-marker-frame");
        let path = folder.0.join("starting.toml");
        let (mut marker, _) = mine(&path);
        assert!(marker.write(&tried(RADEON, Backend::Dx12)));
        let mut pending = Pending::new(Some(marker));
        let ctx = egui::Context::default();
        let mut frames = 0;
        let mut drawn = ctx.run_ui(egui::RawInput::default(), |ui| {
            pending.frame(ui.ctx());
            frames += 1;
        });
        drawn.textures_delta.clear();
        assert!(pending.waiting(), "not before the first frame is drawn");
        assert!(ctx.has_requested_repaint());
        assert!(matches!(Marker::open(path.clone()), Opened::Busy));
        let mut drawn = ctx.run_ui(egui::RawInput::default(), |ui| pending.frame(ui.ctx()));
        drawn.textures_delta.clear();
        pending.finish();
        assert!(!pending.waiting());
        assert!(!path.exists());
        assert_eq!(frames, 1);
    }
}
