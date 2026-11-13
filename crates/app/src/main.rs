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

//! `apertrust`: the desktop editor. For now it is the shell (`docs/gui.md`, The shell)
//! with one raw in Edit: the top panel, the side panel, the central bar and the grid panel,
//! and in the side panel's tabs, darktable's module groups, the modules' sections, each
//! drawn from its definition (`modules`), under the Scopes section every editing tab pins
//! (`scopes`). Its buttons and keys run the actions declared in `actions`. File › Open
//! Image… opens another raw in its place, chosen in the platform's open dialog
//! (`dialogs`).
//!
//! The engine runs on its own thread, on the window's own wgpu device (decision 17):
//! the app sends it [`Command`]s, mirrors what its events report, and draws its latest
//! frame with a paint callback (`viewport`). The window is sRGB, tagged as such every
//! frame by `apertrust-display`, since wgpu leaves it untagged and macOS then shows it
//! without colour management. On macOS the top panel is the window's title bar (decision
//! 39): `apertrust-display` places the traffic lights in it every frame, and a drag or a
//! double-click on its empty areas moves the window or does what the system's setting
//! says, run after the frame. The menu bar there is the system's, built from the registry
//! (`menus`), and what is chosen in it runs through the same path as keys and clicks, in
//! `App::logic`, so it runs while the window is minimised too.
//!
//! What the shell keeps between runs (the theme, the language, the scale, the keys, the
//! side panel's tab and width, the scopes' view, the window's placement) is read from
//! `settings` before the window opens, in the user data folder `apertrust_base::folders`
//! chooses, and written back once it has stood still for a second.
//!
//! On Windows it is a windowed program, so no console opens beside it, in a release
//! build or a debug one; started from a terminal, it attaches to it first thing, so
//! `--version`, `-d`'s lines and its errors print there (`apertrust-display`). Its unit
//! tests stay a console program, which cargo's test runner reads.

#![cfg_attr(not(test), windows_subsystem = "windows")]

mod about;
mod actions;
mod alerts;
mod dialog_host;
mod dialogs;
mod dropped;
mod export;
mod export_again;
mod exposure;
mod fade;
mod folder;
mod hardware;
mod history;
mod jobs;
mod library;
mod log_window;
mod menus;
mod mirror;
mod modules;
mod paths;
mod pickers;
mod popups;
mod recents;
mod recovery;
mod restart;
mod scopes;
mod settings;
mod settings_dialog;
mod start;
mod storage;
#[cfg(test)]
mod testing;
mod thumbnails;
mod viewport;

use std::collections::{BTreeMap, HashMap};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use actions::{ActionId, Emit, Focus, Intent, Keymap, State, View};
use apertrust_base::cache::{Cache, Limit};
use apertrust_base::folders::{self, Folders, Given};
use apertrust_base::settings::CONF_FLAG;
use apertrust_display::{ColorSpace, DoubleClick, WindowButtons};
use apertrust_engine::gpu::{self, GpuContext, SubmitGate};
use apertrust_engine::session::library::{EditState, RawThumbnails, Source};
use apertrust_engine::session::{
    CacheCommand, CacheOptions, Command, ControlValue, Frame, Graphs, LibraryCommand,
    PickerCommand, PickerRef, Places, Session, StepSource, Values, ViewOnly,
};
use apertrust_modules::Module;
use apertrust_ui::components::{
    ActivityAction, Available, CellRating, CentralBar, CentralBarAction, GridEdge, GridEdgeAction,
    GridPanel, GridPanelAction, GridPosition, Jobs, Margin, Marker, PanelBody, PanelEdge,
    PanelEdgeAction, PanelTab, PhotoCaption, PhotoFrame, PickerAction, Placed, SaveState,
    ScopeView, TabBar, TabBarAction, Thumb, Toggle, TopPanel, TopPanelAction, Zoom,
    grid_panel_lines, grid_panel_thickness,
};
use apertrust_ui::{
    Background, Builtin, Icon, Locale, Theme, Translator, popup, theme, tr, translator,
};
use eframe::egui::{self, os::OperatingSystem};
use eframe::egui_wgpu::{self, WgpuSetup, WgpuSetupCreateNew};
use egui_winit::winit::application::ApplicationHandler;
use egui_winit::winit::event::{DeviceEvent, DeviceId, StartCause, WindowEvent};
use egui_winit::winit::event_loop::ActiveEventLoop;
use egui_winit::winit::window::WindowId;
use mirror::{Mirror, PhotoState, Picture, Reaction, Shown};
use modules::{Change, Panels, Pending};
use recents::Recents;
use settings::{Keeper, Settings};

/// What the command line asks of the app: `apertrust [-d DOMAIN]... [--user-data-dir DIR]
/// [--cache-dir DIR] [--conf KEY=VALUE]... [--safe-mode] [FILE | FOLDER | --in-folder
/// FILE...]` ([`USAGE`]).
#[derive(Debug, Default, PartialEq)]
struct Arguments {
    /// What to open: the first path given, from a terminal.
    open: Option<Start>,
    /// `--in-folder FILE...`, Open With's and a double-click's on Windows and Linux, the
    /// registrations passing it before the files (`docs/packaging.md`, Opening a raw from
    /// the system): every path given, absolute, to open as a drop of them would, the
    /// photo's folder with it active, looked at off the interface's thread once the window
    /// is up and refused there, since a launcher has no terminal. None given, as a desktop
    /// entry's `%F` gives none when the app is started from a menu, opens nothing.
    in_folder: Vec<PathBuf>,
    /// darktable-style debug domains for stderr (`apertrust_base::log::DOMAINS`).
    domains: Vec<String>,
    /// The user data folder, where the settings are kept, in place of the platform's:
    /// `--user-data-dir DIR` or `--user-data-dir=DIR`, or `--config-dir` or darktable's
    /// `--configdir` the same way (`folders::FLAGS`); the last given; a leading `~` the home
    /// folder, which `apertrust_base::folders` expands. Until edits are kept
    /// in it, it separates settings only: edits are still written beside the photos.
    user_data: Option<Given>,
    /// The cache folder, in place of the variable's, the user data folder's or the
    /// platform's: `--cache-dir DIR` or `--cache-dir=DIR`, or darktable's `--cachedir` the
    /// same way (`folders::CACHE_FLAGS`), the last given.
    cache: Option<Given>,
    /// `--conf KEY=VALUE` or `--conf=KEY=VALUE`, darktable's, each a setting's value for
    /// this run, in the order given, which `settings::overrides` reads (decision 43).
    conf: Vec<String>,
    /// `--safe-mode`: the graphics card and API at Automatic for this run, the settings
    /// kept (decision 82, `recovery`).
    safe_mode: bool,
    /// The hidden `--reopen-last` a restart passes, true: what was open last reopens,
    /// whatever `library.reopen_last` says; or `--no-reopen-last`, false: the start window
    /// comes back (`restart`).
    reopen: Option<bool>,
    /// The flags as given, but `--safe-mode`, the two hidden ones, `--in-folder` and the paths,
    /// each with its value, a folder's made absolute: what a restart starts with again.
    flags: Vec<OsString>,
}

/// What the command line's path opens (`docs/gui.md`, The start window).
#[derive(Clone, Debug, PartialEq)]
enum Start {
    /// A raw, alone (`docs/library.md`, One photo).
    Photo(PathBuf),
    /// A folder, in folder mode.
    Folder(PathBuf),
}

/// `--in-folder`, which Open With's registrations pass before the files.
const IN_FOLDER_FLAG: &str = "--in-folder";

/// What `--help` prints: the app's command line. `--list-adapters`, `--reopen-last` and
/// `--no-reopen-last` are the app's own, for Settings' helper and a restart, and left out.
const USAGE: &str = "\
Usage: apertrust [OPTIONS] [FILE | FOLDER | --in-folder FILE...]

Opens a raw photo alone, or a folder in folder mode; with neither, what was open last.

Options:
  -d, --debug DOMAIN      Log a domain on stderr: gpu, raw, pipe, perf, memory, settings,
                          control, event, all or verbose; again for another
      --user-data-dir DIR The folder of settings and personal resources (also --config-dir,
                          --configdir; APERTRUST_USER_DATA_DIR). Edits stay beside the photos
      --cache-dir DIR     The cache folder (also --cachedir; APERTRUST_CACHE_DIR)
      --conf KEY=VALUE    A setting's value for this run only
      --safe-mode         Start with the graphics card and API at Automatic for this run,
                          the settings kept
      --in-folder FILE... Open the files' folder with the first file active, as Open With does
  -h, --help              Print this help
  -V, --version           Print the version, libraries and GPU backends
";

/// Whether `args`, the command line after the program's name, asks for [`USAGE`].
fn help_asked(mut args: impl Iterator<Item = OsString>) -> bool {
    args.any(|arg| arg == "--help" || arg == "-h")
}

/// Reads `args`, the command line after the program's name, `look` saying what its path
/// is on the disk ([`start()`]), unless `--in-folder` comes anywhere in it, when the disk is
/// not asked here and every path is its. A flag of the user data folder or the cache
/// folder without one is an error, so that another is never used in its place, as is
/// `--conf` without its setting.
fn arguments(
    args: impl IntoIterator<Item = OsString>,
    look: impl FnOnce(&Path) -> std::io::Result<dropped::Kind>,
) -> Result<Arguments, String> {
    let mut args = args.into_iter();
    let mut arguments = Arguments::default();
    let (mut paths, mut in_folder) = (Vec::new(), false);
    while let Some(arg) = args.next() {
        if arg == IN_FOLDER_FLAG {
            in_folder = true;
        } else if arg == restart::SAFE_MODE_FLAG {
            arguments.safe_mode = true;
        } else if arg == restart::REOPEN_FLAG {
            arguments.reopen = Some(true);
        } else if arg == restart::NO_REOPEN_FLAG {
            arguments.reopen = Some(false);
        } else if arg == "-d" || arg == "--debug" {
            let domain = args.next();
            let named = domain.as_ref().map(|d| d.to_string_lossy().into_owned());
            arguments.domains.extend(named);
            arguments.flags.push(arg);
            arguments.flags.extend(domain);
        } else if arg == CONF_FLAG {
            let given = args
                .next()
                .ok_or_else(|| format!("{CONF_FLAG} needs KEY=VALUE"))?;
            arguments.conf.push(utf8(given.clone())?);
            arguments.flags.extend([arg, given]);
        } else if let Some(given) = conf_joined(&arg) {
            arguments.conf.push(given?);
            arguments.flags.push(arg);
        } else if let Some(flag) = FOLDER_FLAGS.into_iter().find(|flag| arg == *flag) {
            let folder = args
                .next()
                .ok_or_else(|| format!("{flag} needs a folder"))?;
            arguments.named(flag, PathBuf::from(folder));
        } else if let Some(given) = joined(&arg) {
            let (flag, folder) = given?;
            arguments.named(flag, folder);
        } else {
            paths.push(PathBuf::from(arg));
        }
    }
    if in_folder {
        let absolute = |path: PathBuf| std::path::absolute(&path).unwrap_or(path);
        arguments.in_folder = paths.into_iter().map(absolute).collect();
    } else {
        let first = paths.into_iter().next();
        arguments.open = first.map(|path| start(path, look)).transpose()?;
    }
    Ok(arguments)
}

/// What `path`, the command line's, opens, as `look` finds it on the disk: a folder in
/// folder mode; a raw, told by its name, alone; either [`resolved`], so the photo's folder
/// is known (Open This Folder). A library's or a
/// project's file, until Phase 5, another file, or nothing there, is an error saying why.
fn start(
    path: PathBuf,
    look: impl FnOnce(&Path) -> std::io::Result<dropped::Kind>,
) -> Result<Start, String> {
    let given = path.display();
    let kind = look(&path).map_err(|error| format!("{given}: {error}"))?;
    match kind {
        dropped::Kind::Folder => Ok(Start::Folder(resolved(&path, true))),
        dropped::Kind::Raw => Ok(Start::Photo(resolved(&path, false))),
        dropped::Kind::Library => Err(format!(
            "{given}: libraries and projects come in a later version"
        )),
        dropped::Kind::Other => Err(format!("{given}: only folders and raw photos open")),
    }
}

/// `path`, a `folder` or a photo, as the app keeps what the command line names: absolute,
/// from the folder the app starts in, its `.` and `..` and links resolved, a photo's in its
/// folder alone, so a photo that is a link keeps its own name, and its folder is where the
/// link lies. On Windows only made absolute, since a resolved path there is a `\\?\` one
/// the file manager and the dialogs show as it is. One that cannot be resolved is kept
/// absolute.
fn resolved(path: &Path, folder: bool) -> PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_owned());
    if cfg!(windows) {
        return absolute;
    }
    let canonical = |path: &Path| std::fs::canonicalize(path).ok();
    let resolved = if folder {
        canonical(&absolute)
    } else {
        let (parent, name) = (absolute.parent(), absolute.file_name());
        parent
            .zip(name)
            .and_then(|(parent, name)| Some(canonical(parent)?.join(name)))
    };
    resolved.unwrap_or(absolute)
}

/// The flags that name a folder: the user data folder's (`folders::FLAGS`), then the cache
/// folder's (`folders::CACHE_FLAGS`).
const FOLDER_FLAGS: [&str; 5] = [
    folders::FLAGS[0],
    folders::FLAGS[1],
    folders::FLAGS[2],
    folders::CACHE_FLAGS[0],
    folders::CACHE_FLAGS[1],
];

impl Arguments {
    /// Takes the folder `flag`, one of [`FOLDER_FLAGS`], names: the last given counts. A
    /// restart is given it again, absolute but for a leading `~`, which `folders` expands,
    /// since the Mac's `open` starts the app in another folder.
    fn named(&mut self, flag: &'static str, folder: PathBuf) {
        let home = folder.starts_with("~");
        let kept = match std::path::absolute(&folder) {
            Ok(absolute) if !home => absolute,
            _ => folder.clone(),
        };
        self.flags.extend([flag.into(), kept.into_os_string()]);
        let given = Some(Given { flag, folder });
        if folders::CACHE_FLAGS.contains(&flag) {
            self.cache = given;
        } else {
            self.user_data = given;
        }
    }
}

/// `arg` as `FLAG=DIR`, for a flag of [`FOLDER_FLAGS`]: the flag and the folder given, or
/// an error for one whose name is not UTF-8, which goes after a space instead; None for any
/// other argument.
fn joined(arg: &OsStr) -> Option<Result<(&'static str, PathBuf), String>> {
    let bytes = arg.as_encoded_bytes();
    let flag = FOLDER_FLAGS.into_iter().find(|flag| {
        let rest = bytes.strip_prefix(flag.as_bytes());
        rest.is_some_and(|rest| rest.first() == Some(&b'='))
    })?;
    Some(match arg.to_str() {
        Some(arg) => Ok((flag, PathBuf::from(&arg[flag.len() + 1..]))),
        None => Err(format!(
            "{flag}=: a folder whose name is not UTF-8 goes after a space, as {flag} DIR"
        )),
    })
}

/// `arg` as `--conf=KEY=VALUE`: the setting given, or an error for one that is not UTF-8;
/// None for any other argument.
fn conf_joined(arg: &OsStr) -> Option<Result<String, String>> {
    let bytes = arg.as_encoded_bytes();
    let given = bytes
        .strip_prefix(CONF_FLAG.as_bytes())?
        .strip_prefix(b"=")?;
    Some(match std::str::from_utf8(given) {
        Ok(given) => Ok(given.to_owned()),
        Err(_) => Err(not_utf8(&String::from_utf8_lossy(given))),
    })
}

/// `--conf`'s `KEY=VALUE`, which settings hold as text.
fn utf8(given: OsString) -> Result<String, String> {
    given
        .into_string()
        .map_err(|given| not_utf8(&given.to_string_lossy()))
}

fn not_utf8(given: &str) -> String {
    format!("{CONF_FLAG} {given}: not UTF-8")
}

/// What the banner and `--version` list the app as built with.
fn build(program: &apertrust_base::log::Program) -> Vec<apertrust_base::log::Item> {
    let languages: Vec<&str> = Locale::ALL
        .into_iter()
        .filter(|l| offered(*l, cfg!(debug_assertions)))
        .map(Locale::id)
        .collect();
    vec![
        apertrust_base::log::wgpu(program, gpu::backends()),
        apertrust_base::log::egui(program),
        apertrust_base::log::rawler(program),
        apertrust_base::log::kernels(program),
        apertrust_base::log::accesskit(program),
        apertrust_base::log::languages(&languages),
        apertrust_base::log::onnx_runtime(),
    ]
}

fn main() -> eframe::Result {
    // Before anything is printed: a windowed program's stdout and stderr go nowhere until
    // it attaches to the terminal that started it, where one did.
    apertrust_display::attach_to_terminal();
    if apertrust_base::log::version_asked() {
        let program = apertrust_base::program!();
        apertrust_base::log::print_version(program, &build(&program));
        return Ok(());
    }
    if help_asked(std::env::args_os().skip(1)) {
        print!("{USAGE}");
        return Ok(());
    }
    // The helper Settings runs to list a graphics API's adapters (decision 82): it prints
    // them and ends, never opening a window, so a driver that crashes as it loads ends it
    // and not the app.
    if let Some(backend) = hardware::helper_asked(std::env::args_os().skip(1)) {
        hardware::print_listed(backend);
        return Ok(());
    }
    let Arguments {
        open,
        in_folder,
        domains,
        user_data,
        cache,
        conf,
        safe_mode,
        reopen,
        flags,
    } = match arguments(std::env::args_os().skip(1), dropped::found) {
        Ok(arguments) => arguments,
        Err(error) => {
            eprintln!("apertrust: {error}");
            std::process::exit(2);
        }
    };
    let logging = match apertrust_base::log::init(&domains) {
        Ok(logging) => logging,
        Err(error) => {
            eprintln!("apertrust: {error}");
            std::process::exit(2);
        }
    };
    // Only with a domain on, as the CLI: the app is as silent as decision 44 has it,
    // since its stderr goes nowhere when it is opened from the desktop.
    apertrust_base::log::starting(apertrust_base::program!(), build);
    // The log kept in memory for the log window, off until settings turn it on.
    let kept = logging.kept();
    // `--conf`'s values for this run, checked against the settings before anything is
    // read or created: one the app cannot take stops the start (decision 43).
    let overrides = match settings::overrides(&conf) {
        Ok(overrides) => overrides,
        Err(error) => {
            eprintln!("apertrust: {error}");
            std::process::exit(2);
        }
    };
    // The user data folder, before its settings are read, and the cache folder. One given
    // that cannot be used stops the start, never falling back to the platform's
    // (decision 43).
    let folders = match Folders::find(user_data, cache, apertrust_base::program!().dev) {
        Ok(folders) => folders,
        Err(error) => {
            eprintln!("apertrust: {error}");
            std::process::exit(1);
        }
    };
    // A folder an environment variable chose is a flag for a restart, which the Mac's
    // `open` starts without this process's environment (`restart`).
    let mut flags = flags;
    flags.extend(carried(folders.as_ref()));
    // Before the window opens, since it opens where it was.
    let (keeper, settings) = Keeper::open_in(folders.as_ref(), &overrides);
    // The recents, from `state.toml` beside the settings, the old setting's moved into it
    // where it has none yet; kept nowhere without a user data folder.
    let recents = match folders.as_ref() {
        Some(folders) => {
            let file = folders.state();
            let (recents, warnings) = Recents::open(file.clone(), || keeper.old_recent_folders());
            for warning in warnings {
                tracing::warn!(target: "settings", "{}: {warning}", file.display());
            }
            recents
        }
        None => Recents::nowhere(),
    };
    // Whether the window opens as the start window, nothing to open given or reopened:
    // Finder's files at the launch come only once the loop runs, and `App::new` gives the
    // window the main window's size and place back where they come.
    let reopen_last = reopen.unwrap_or_else(|| keeper.reopen_last());
    let reopens = reopen_last && recents.last().is_some();
    let starts = open.is_none() && in_folder.is_empty() && !reopens;
    let placement = if starts {
        start_placement(&settings.window, settings.scale)
    } else {
        settings.window
    };
    // A start that did not finish, or `--safe-mode`: Automatic, or the CPU, in place of the
    // graphics settings for this run, without the API that did not start where Automatic
    // loads another (decision 82). The marker, locked for this start, is
    // written naming what it tries before anything loads a graphics driver, the engine's
    // settings included, then what they resolve to, and removed once the window has drawn
    // (`recovery`). One another start holds is that start's, under way at once.
    let path = folders.as_ref().map(Folders::starting);
    let (graphics, backend) = keeper.graphics();
    let tried = recovery::Tried::of(graphics, backend);
    let (run, mut marker) = recovery::begin(path, safe_mode, tried);
    // The engine's settings, from the same file, as the engine's options (decision 43).
    // The session takes the memory budget as it starts, so the settings read once here
    // give every photo of this run its budget: a change applies at the next start.
    let engine = match run.instead() {
        Some((adapter, api, by)) => keeper.engine_instead(adapter, api, by),
        None => keeper.engine(),
    };
    if let Some(marker) = marker.as_mut() {
        marker.write(&run.opened(&engine));
    }
    tracing::debug!(target: "settings", "engine.memory_budget: for this run, a change at the next start");
    // The disk cache, on the folder chosen with the user data folder (decision 76).
    let cache = folders
        .as_ref()
        .and_then(|folders| open_cache(folders.cache()));
    // The folders as Settings › Storage shows them.
    let storage_folders = storage::Folders::of(folders.as_ref(), export::places().home);
    // The window's device is the engine's, so it is created from the engine's instance,
    // built from the same options, so the two load the same backends (decision 82), its
    // Direct3D 12 shader compiler included, and with the engine's needs. A display handle
    // only matters for OpenGL on Wayland; the engine uses Vulkan. A backend `--conf` names
    // that this build lacks would find no adapter: it stops the start, as the CLI's does.
    if let Err(error) = gpu::check_backend(&engine.gpu) {
        // The command line's error, not the device's: no marker is left for it.
        if let Some(marker) = marker {
            marker.remove();
        }
        eprintln!("apertrust: {error}");
        std::process::exit(1);
    }
    // Where a restart's command line goes once the app has ended, for the last step here.
    let relaunch = restart::Slot::default();
    let mut launch = Launch {
        reopen_last,
        restart: restart::Restart::new(flags, safe_mode, relaunch.clone()),
        run,
        marker: recovery::Pending::new(marker),
    };
    let mut setup = WgpuSetupCreateNew::without_display_handle();
    setup.instance_descriptor = gpu::instance_descriptor(&engine.gpu);
    setup.device_descriptor = Arc::new(gpu::device_descriptor);
    // On the adapter `engine.adapter` names, chosen as the engine chooses it, or wgpu's
    // choice where it names none; once, so a change applies after a restart.
    if engine.gpu.adapter != gpu::AdapterChoice::Any {
        let choice = engine.gpu.adapter.clone();
        setup.native_adapter_selector = Some(Arc::new(
            move |adapters: &[egui_wgpu::wgpu::Adapter],
                  surface: Option<&egui_wgpu::wgpu::Surface<'_>>| {
                gpu::choose_adapter(adapters, surface, &choice).map_err(|e| e.to_string())
            },
        ));
    }
    let options = eframe::NativeOptions {
        viewport: window(&placement, starts),
        // The start window with no place kept, as on the first run, in the screen's middle.
        centered: starts && placement.position.is_none(),
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: egui_wgpu::WgpuConfiguration {
            wgpu_setup: WgpuSetup::CreateNew(setup),
            ..Default::default()
        },
        ..Default::default()
    };
    // eframe runs inside a handler of the app's own, which holds the engine's submit gate
    // while eframe works, as eframe reconfigures the window's surface (`Gated`). On its
    // own event loop, so eframe ignores `NativeOptions::event_loop_builder`: what the loop
    // is built with goes here.
    let gate = SubmitGate::default();
    let engine_gate = gate.clone();
    // The app's menus have their own full-screen item: AppKit's stays out of them.
    apertrust_display::no_system_full_screen_item();
    let mut builder =
        egui_winit::winit::event_loop::EventLoop::<eframe::UserEvent>::with_user_event();
    // The Mac's menu bar is the app's own, built from the registry once the window exists
    // (`menus`), so winit's default menu, whose Quit would skip the app's exit, never comes.
    #[cfg(target_os = "macos")]
    egui_winit::winit::platform::macos::EventLoopBuilderExtMacOS::with_default_menu(
        &mut builder,
        false,
    );
    // An event loop that cannot be made is no graphics device's failure: no marker is left
    // for it. The returns before this one have none yet, or remove it themselves.
    let event_loop = match builder.build() {
        Ok(event_loop) => event_loop,
        Err(error) => {
            launch.marker.abandon();
            return Err(error.into());
        }
    };
    // Finder's files, at the launch too, once winit's delegate is set and before its loop
    // runs.
    if let Err(e) = apertrust_display::receive_opened_files() {
        tracing::warn!(target: "control", "files Finder opens with the app not received: {e}");
    }
    // A click on the Dock icon, which shows the start window again once it was closed.
    if let Err(e) = apertrust_display::receive_reopen() {
        tracing::warn!(target: "control", "the Dock icon's click not received: {e}");
    }
    // The dialogs' windows, beside eframe's, run by `Gated` (decision 41).
    let host = dialog_host::Host::new(event_loop.create_proxy());
    let app_host = host.clone();
    // eframe ends the loop, logging why, when the window or its device cannot be made, and
    // returns the error only from its own `run_native`: this says whether the app began.
    let started = std::cell::Cell::new(false);
    let app = {
        let started = &started;
        eframe::create_native(
            APP_NAME,
            options,
            Box::new(move |cc| {
                started.set(true);
                Ok(Box::new(App::new(
                    cc,
                    open,
                    in_folder,
                    (placement, starts),
                    keeper,
                    settings,
                    engine_gate,
                    engine,
                    cache,
                    recents,
                    storage_folders,
                    app_host,
                    kept,
                    launch,
                )))
            }),
            &event_loop,
        )
    };
    event_loop.run_app(&mut Gated { app, gate, host })?;
    if !started.get() {
        return Err(eframe::Error::AppCreation(
            "the window could not open: eframe's error is in the log above".into(),
        ));
    }
    // Restart now's new start, the very last step: the app has ended, its window gone,
    // its settings and recents written and its session dropped, its edit saved, so the
    // new one never races it for a file (decision 82).
    let relaunch = relaunch.lock().map(|mut slot| slot.take());
    if let Ok(Some(relaunch)) = relaunch {
        restart::start(relaunch);
    }
    Ok(())
}

/// The folders `folders` holds that an environment variable chose, `APERTRUST_USER_DATA_DIR`
/// or `APERTRUST_CACHE_DIR`, as their flags with the folders found, absolute, for a
/// restart: one a flag chose is among the flags already, and the platform's is found again.
fn carried(folders: Option<&Folders>) -> Vec<OsString> {
    let Some(folders) = folders else {
        return Vec::new();
    };
    let mut flags = Vec::new();
    if folders.chosen() == folders::Chosen::Variable {
        flags.extend([folders::FLAGS[0].into(), folders.user_data().into()]);
    }
    if folders.cache_chosen() == folders::CacheChosen::Variable {
        flags.extend([folders::CACHE_FLAGS[0].into(), folders.cache().into()]);
    }
    flags
}

/// What `main` hands the app of how it starts: whether it reopens what was open last, its
/// restart, and what it opens its graphics device with, which decides what the window
/// says (decision 82).
struct Launch {
    reopen_last: bool,
    restart: restart::Restart,
    run: recovery::Run,
    marker: recovery::Pending,
}

/// eframe's winit application, run with the engine's [`SubmitGate`] held while any of its
/// callbacks runs. eframe reconfigures the window's surface on the UI thread, as the
/// window resizes, as a resize ends and when a frame finds the surface out of date, and
/// wgpu-core 30.0.1 fails a reconfigure during which another thread submits, a failure
/// that aborted the app inside winit's resize callback. The
/// engine thread holds the gate round each submission, so the two never overlap. They
/// take turns: a submission waits for the callback under way, however long it runs, so a
/// callback that runs a loop of its own, a modal one included, keeps the engine from
/// submitting until it returns; a callback waits for a submission asked for before it,
/// and a reconfigure inside it then waits for the GPU to finish everything submitted, a
/// render's work. On every platform, since every reconfigure is inside one of these.
///
/// The dialogs' windows are the host's, beside eframe's (decision 41): their events,
/// AccessKit requests and repaints go to the host, never to eframe, and after each of
/// eframe's callbacks the host makes or closes a dialog asked for, all with the gate held.
struct Gated<'a> {
    app: eframe::EframeWinitApplication<'a>,
    gate: SubmitGate,
    host: dialog_host::Host,
}

impl ApplicationHandler<eframe::UserEvent> for Gated<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let _held = self.gate.hold();
        self.app.resumed(event_loop);
        self.host.run(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window: WindowId, event: WindowEvent) {
        let _held = self.gate.hold();
        if self.host.owns(window) {
            self.host.window_event(event_loop, window, &event);
        } else {
            self.app.window_event(event_loop, window, event);
        }
        self.host.run(event_loop);
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        let _held = self.gate.hold();
        self.app.new_events(event_loop, cause);
        self.host.run(event_loop);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: eframe::UserEvent) {
        let _held = self.gate.hold();
        if let Some(event) = self.host.user_event(event) {
            self.app.user_event(event_loop, event);
        }
        self.host.run(event_loop);
    }

    fn device_event(&mut self, event_loop: &ActiveEventLoop, device: DeviceId, event: DeviceEvent) {
        let _held = self.gate.hold();
        self.app.device_event(event_loop, device, event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let _held = self.gate.hold();
        self.app.about_to_wait(event_loop);
        self.host.run(event_loop);
        // eframe has set when the loop wakes for its windows; the dialog's may come first.
        self.host.wait(event_loop);
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        let _held = self.gate.hold();
        self.app.suspended(event_loop);
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        let _held = self.gate.hold();
        self.host.exiting();
        self.app.exiting(event_loop);
    }

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
        let _held = self.gate.hold();
        self.app.memory_warning(event_loop);
    }
}

/// The app's name, never translated: the window's title, Apertrust Dev for a development
/// build. On macOS the app menu's comes from `Info.plist`, which the build embeds in the
/// binary (`build.rs`).
const APP_NAME: &str = apertrust_base::program!().app_name();
/// The app's id, its desktop file's name on Linux, where a Wayland compositor finds the
/// window's icon and name through it (packaging/linux); a development build's ends in
/// `.Dev`.
const APP_ID: &str = apertrust_base::program!().app_id();

/// The smallest size the shell takes, in logical pixels; its size at first start is
/// `window.size`'s default.
const WINDOW_MIN: [f32; 2] = [960.0, 600.0];

/// On macOS and Windows the window is moved where it was once it exists ([`place`]), and
/// only then maximised, since moving resets it, as egui-winit's `WindowSettings` does for
/// eframe's own persistence; Linux's window managers place it from the builder.
const PLACE_AFTER_CREATION: bool = cfg!(any(target_os = "macos", target_os = "windows"));

/// macOS's zoom is not maximising: egui reads it only when the window opens, so settings
/// neither follow nor restore it there, and the window opens at its last size.
const MAXIMISES: bool = !cfg!(target_os = "macos");

/// The top panel is the window's title bar (`docs/gui.md`, The window frame and menus;
/// decision 39). On macOS, built: the content runs to the window's top edge under a title
/// bar that is neither drawn nor titled, and the traffic lights sit in the top panel. On
/// Windows and Linux, see [`CAPTION_BUTTONS`].
const OWN_TITLE_BAR: bool = cfg!(target_os = "macos");

/// On Windows and on Linux, X11 and Wayland, the top panel is the title bar too, built as
/// decision 39 says: decorations off, so no system draws a title bar; the top panel draws
/// minimise, maximise or restore and close at its end, and the window resizes from bands
/// inside its edges ([`RESIZE_BAND`]). Moves, resizes and the window menu are the
/// system's: on Windows winit keeps the styles Aero Snap and Win+arrows need, and the
/// window keeps its shadow ([`frame_shadow`]).
const CAPTION_BUTTONS: bool = cfg!(any(target_os = "linux", target_os = "windows"));

/// The width of the bands inside the window's edges that resize it where the app draws
/// its own frame ([`CAPTION_BUTTONS`]), in points; a corner where two meet.
const RESIZE_BAND: f32 = 6.0;

/// The room the Mac's window buttons take where AppKit puts them in its 28-point title
/// bar, in the window system's points, for when they cannot be measured: the zoom
/// button ends about 70 points from the window's edge.
const BUTTONS_ROOM: f32 = 78.0;

/// The window as it last stood: its size, its position where the platform lets an app
/// place it (Wayland does not), and maximised or not. Full screen is not restored. The
/// window is created at the interface's scale of 1, so a position in logical pixels is
/// its pixels over its scale factor. Its title is kept where it is not drawn, for
/// Mission Control, the Window menu and screen readers. As the start window (`start`), the
/// size `placement` holds, `start_placement`'s, and neither resized nor maximised.
fn window(placement: &settings::Window, start: bool) -> egui::ViewportBuilder {
    let size = inner_size(placement, start);
    let viewport = egui::ViewportBuilder::default()
        .with_title(APP_NAME)
        .with_app_id(APP_ID)
        .with_icon(icon());
    let viewport = if start {
        viewport
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_max_inner_size(size)
            .with_resizable(false)
            .with_maximize_button(false)
    } else {
        viewport
            .with_inner_size(size)
            .with_min_inner_size(WINDOW_MIN)
            .with_maximized(MAXIMISES && !PLACE_AFTER_CREATION && placement.maximized)
    };
    let viewport = if OWN_TITLE_BAR {
        viewport
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false)
            .with_titlebar_buttons_shown(true)
    } else if CAPTION_BUTTONS {
        viewport.with_decorations(false)
    } else {
        viewport
    };
    match placement.position {
        Some(at) => viewport.with_position(egui::Pos2::from(at.pixels) / at.scale_factor),
        None => viewport,
    }
}

/// The inner size, in logical pixels, the window is made at from `placement`: the start
/// window's as it is (`start`), the main window's never under [`WINDOW_MIN`].
fn inner_size(placement: &settings::Window, start: bool) -> [f32; 2] {
    let size = placement
        .size
        .unwrap_or(settings::registry::WINDOW_SIZE.default());
    if start {
        size
    } else {
        [size[0].max(WINDOW_MIN[0]), size[1].max(WINDOW_MIN[1])]
    }
}

/// The inner size, in logical pixels, the window made at `size` is given once it exists on
/// macOS (`mac`): at most the largest monitor's size in points, the largest width and the
/// largest height taken apart, as eframe caps it as it makes the window; the main window's
/// never under [`WINDOW_MIN`], the start window's as it is (`start`). On macOS a window
/// created at about the screen's visible size came back 20 points smaller (1450 by 848 of
/// 1470 by 868, on a screen of 1470 by 956); setting its size again once it exists keeps
/// it. None elsewhere, where the window keeps the size it is made at, and where no monitor
/// is known.
fn size_on_screen(
    size: [f32; 2],
    start: bool,
    monitors: &[Monitor],
    mac: bool,
) -> Option<[f32; 2]> {
    if !mac || monitors.is_empty() {
        return None;
    }
    let largest = monitors
        .iter()
        .map(|m| m.points().size())
        .fold(egui::Vec2::ZERO, egui::Vec2::max);
    let size = egui::Vec2::from(size).min(largest);
    let size = if start {
        size
    } else {
        size.max(egui::Vec2::from(WINDOW_MIN))
    };
    Some([size.x, size.y])
}

/// Gives the frameless window on Windows the system's drop shadow and border, which winit
/// leaves off without decorations; Windows 11 rounds its corners with them. Elsewhere the
/// system or the compositor draws them.
fn frame_shadow(window: &egui_winit::winit::window::Window) {
    #[cfg(target_os = "windows")]
    {
        use egui_winit::winit::platform::windows::WindowExtWindows;
        window.set_undecorated_shadow(true);
    }
    #[cfg(not(target_os = "windows"))]
    let _ = window;
}

/// Moves the new window, not yet shown, where `placement` says, on a monitor that is still
/// there ([`on_screen`]), then maximises it if it was, on macOS and Windows: there winit
/// would place it on the monitor it is created on, with that monitor's scale factor. On
/// macOS it is given its size again first ([`size_on_screen`]), the start window's as
/// `start` says, since one created at about the screen's visible size came back 20 points
/// smaller.
fn place(window: &egui_winit::winit::window::Window, placement: &settings::Window, start: bool) {
    use egui_winit::winit::dpi::LogicalSize;
    if !PLACE_AFTER_CREATION {
        return;
    }
    if let Some(position) = placement.position {
        let size = placement
            .size
            .unwrap_or(settings::registry::WINDOW_SIZE.default());
        let monitors = monitors(window);
        if let Some(spot) = on_screen(&position, size, &monitors, cfg!(target_os = "macos")) {
            let inner = inner_size(placement, start);
            let mac = cfg!(target_os = "macos");
            if let Some([width, height]) = size_on_screen(inner, start, &monitors, mac) {
                let size = LogicalSize::new(f64::from(width), f64::from(height));
                let _ = window.request_inner_size(size);
            }
            move_to(window, spot);
        }
    }
    if MAXIMISES && placement.maximized {
        egui_winit::set_maximized(window, true);
    }
}

/// Where the start window stands at the interface's `scale` (`docs/gui.md`, The start
/// window): its size, in logical pixels, centred on `main`, the main window's last normal
/// placement, where that has a position; never maximised.
fn start_placement(main: &settings::Window, scale: f32) -> settings::Window {
    let size = (start::SIZE * scale).round();
    let kept = main
        .size
        .unwrap_or(settings::registry::WINDOW_SIZE.default());
    let kept = egui::Vec2::from(kept).max(egui::Vec2::from(WINDOW_MIN));
    let position = main.position.map(|at| {
        let offset = (kept - size) / 2.0 * at.scale_factor;
        settings::Position {
            pixels: [
                (at.pixels[0] + offset.x).round(),
                (at.pixels[1] + offset.y).round(),
            ],
            scale_factor: at.scale_factor,
        }
    });
    settings::Window {
        size: Some([size.x, size.y]),
        position,
        maximized: false,
    }
}

/// Gives the window it shows the start window's placement or the main window's, as `start`
/// says: its size, its least size and whether it resizes and maximises; then its place,
/// where `placement` has one on a monitor still there (Wayland places none); and, for the
/// main window, maximised if it was.
fn reshape(window: &egui_winit::winit::window::Window, placement: &settings::Window, start: bool) {
    use egui_winit::winit::dpi::LogicalSize;
    use egui_winit::winit::window::WindowButtons;
    let [width, height] = placement
        .size
        .unwrap_or(settings::registry::WINDOW_SIZE.default());
    let logical = |[width, height]: [f32; 2]| LogicalSize::new(f64::from(width), f64::from(height));
    window.set_min_inner_size(None::<LogicalSize<f64>>);
    window.set_max_inner_size(None::<LogicalSize<f64>>);
    if start {
        let size = logical([width, height]);
        let _ = window.request_inner_size(size);
        window.set_min_inner_size(Some(size));
        window.set_max_inner_size(Some(size));
        window.set_resizable(false);
        window.set_enabled_buttons(WindowButtons::CLOSE | WindowButtons::MINIMIZE);
    } else {
        // On macOS at most the largest monitor's, so a first run's default size, larger
        // than a laptop's screen, never runs past its edge and is kept.
        let size = inner_size(placement, false);
        let mac = cfg!(target_os = "macos");
        let size = size_on_screen(size, false, &monitors(window), mac).unwrap_or(size);
        let _ = window.request_inner_size(logical(size));
        window.set_min_inner_size(Some(logical(WINDOW_MIN)));
        window.set_resizable(true);
        window.set_enabled_buttons(WindowButtons::all());
    }
    if let Some(position) = placement.position {
        let monitors = monitors(window);
        let mac = cfg!(target_os = "macos");
        if let Some(spot) = on_screen(&position, [width, height], &monitors, mac) {
            move_to(window, spot);
        }
    }
    if !start && MAXIMISES && placement.maximized {
        egui_winit::set_maximized(window, true);
    }
}

/// `placement` with a place where it has none, as on the first run: centred on the monitor
/// `window` is on ([`centred`]), so the main window, larger than the start window it
/// follows, never reaches past the screen's edge. Unchanged where it has a place, or no
/// monitor is known.
fn placed_on_monitor(
    window: &egui_winit::winit::window::Window,
    placement: settings::Window,
) -> settings::Window {
    if placement.position.is_some() {
        return placement;
    }
    let Some(handle) = window.current_monitor() else {
        return placement;
    };
    let monitor = Monitor {
        rect: egui::Rect::from_min_size(
            egui::pos2(handle.position().x as f32, handle.position().y as f32),
            egui::vec2(handle.size().width as f32, handle.size().height as f32),
        ),
        scale_factor: handle.scale_factor() as f32,
        primary: false,
    };
    let size = placement
        .size
        .unwrap_or(settings::registry::WINDOW_SIZE.default());
    settings::Window {
        position: Some(centred(&monitor, size)),
        ..placement
    }
}

/// Where a window of `size` logical pixels stands centred on `monitor`, its corner never
/// above or left of the monitor's.
fn centred(monitor: &Monitor, size: [f32; 2]) -> settings::Position {
    let size = egui::Vec2::from(size).max(egui::Vec2::from(WINDOW_MIN)) * monitor.scale_factor;
    let room = (monitor.rect.size() - size).max(egui::Vec2::ZERO);
    let at = monitor.rect.min + room / 2.0;
    settings::Position {
        pixels: [at.x.round(), at.y.round()],
        scale_factor: monitor.scale_factor,
    }
}

/// The monitors there are now, as `window`'s platform reports them.
fn monitors(window: &egui_winit::winit::window::Window) -> Vec<Monitor> {
    let primary = window.primary_monitor();
    window
        .available_monitors()
        .map(|m| Monitor {
            rect: egui::Rect::from_min_size(
                egui::pos2(m.position().x as f32, m.position().y as f32),
                egui::vec2(m.size().width as f32, m.size().height as f32),
            ),
            scale_factor: m.scale_factor() as f32,
            primary: primary.as_ref() == Some(&m),
        })
        .collect()
}

/// Puts `window`'s corner at `spot`, the main window's or a dialog's.
fn move_to(window: &egui_winit::winit::window::Window, spot: Spot) {
    use egui_winit::winit::dpi::{LogicalPosition, PhysicalPosition};
    match spot {
        Spot::Points(at) => window.set_outer_position(LogicalPosition::new(at.x, at.y)),
        Spot::Pixels(at) => window.set_outer_position(PhysicalPosition::new(at.x, at.y)),
    }
}

/// A monitor, as winit reports it: its rectangle in physical pixels and its scale factor.
struct Monitor {
    rect: egui::Rect,
    scale_factor: f32,
    primary: bool,
}

impl Monitor {
    /// Its rectangle in points, as macOS places windows.
    fn points(&self) -> egui::Rect {
        self.rect / self.scale_factor
    }
}

/// Where to put a window's corner, in the unit its platform places windows in.
#[derive(Debug, PartialEq)]
enum Spot {
    Points(egui::Pos2),
    Pixels(egui::Pos2),
}

/// Where a window of `size` logical pixels saved at `position` goes on the `monitors` there
/// are now, as egui-winit's `WindowSettings` does it. On macOS, in points, or nowhere when
/// no monitor holds it, as when the one it was on is unplugged. On Windows, in pixels, kept
/// on the monitor it overlaps most (or the primary one), its title bar included, so it never
/// opens out of sight.
fn on_screen(
    position: &settings::Position,
    size: [f32; 2],
    monitors: &[Monitor],
    mac: bool,
) -> Option<Spot> {
    let pixels = egui::Pos2::from(position.pixels);
    if mac {
        let points = pixels / position.scale_factor;
        return monitors
            .iter()
            .any(|m| m.points().contains(points))
            .then_some(Spot::Points(points));
    }
    let size = egui::Vec2::from(size);
    // Apart in both directions, two rectangles meet in one of positive area.
    let overlap = |m: &Monitor| {
        let window = egui::Rect::from_min_size(pixels, size * m.scale_factor);
        let both = window.intersect(m.rect);
        if both.is_positive() { both.area() } else { 0.0 }
    };
    let monitor = monitors
        .iter()
        .filter(|m| overlap(m) > 0.0)
        .max_by(|a, b| overlap(a).total_cmp(&overlap(b)))
        .or_else(|| monitors.iter().find(|m| m.primary))
        .or(monitors.first())?;
    // Windows 10 and 11 draw a title bar of 32 points.
    let window = (size + egui::vec2(0.0, 32.0)) * monitor.scale_factor;
    let room = (monitor.rect.size() - window).max(egui::Vec2::ZERO);
    let rect = monitor.rect;
    Some(Spot::Pixels(pixels.clamp(rect.min, rect.min + room)))
}

/// The window's icon, in place of egui's (`assets/icons`): on a Mac eframe makes it the
/// Dock icon, so it is the macOS plate there and the free-standing A elsewhere.
fn icon() -> egui::IconData {
    #[cfg(target_os = "macos")]
    let png = include_bytes!("../assets/icons/png/macos/AppIcon.iconset/icon_512x512.png");
    #[cfg(not(target_os = "macos"))]
    let png = include_bytes!(
        "../assets/icons/png/linux/hicolor/256x256/apps/org.apertrust.Apertrust.png"
    );
    eframe::icon_data::from_png_bytes(png).expect("the bundled icon is a valid PNG")
}

/// The language the environment asks for, as Unix sets it (`uk_UA.UTF-8`), or British
/// English: the interface's until the user chooses one in Settings, which keeps it.
fn system_locale() -> Locale {
    let wanted: Vec<String> = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .filter(|value| !value.is_empty())
        .collect();
    Locale::negotiate(
        wanted
            .iter()
            .map(|value| value.split('.').next().unwrap_or_default()),
    )
}

/// The side panel's tabs in Edit (decision 38): darktable's scene-referred module
/// groups, base, color, correct and effect, between Apertrust's own Photos and History.
/// Ctrl+1 to Ctrl+5 choose the first five by position ([`ActionId::PANEL_TABS`]); History,
/// the sixth, has no key.
const TABS: [(Icon, &str); 6] = [
    (Icon::Folder, "tab-photos"),
    (Icon::Base, "tab-base"),
    (Icon::Color, "tab-color"),
    (Icon::Correct, "tab-correct"),
    (Icon::Effect, "tab-effect"),
    (Icon::History, "tab-history"),
];
/// The tab Edit opens on, and the one shown where settings keep no tab, or one that is not
/// there or cannot be chosen.
const BASE: usize = 1;

/// The tabs' ids, as settings keep the last one, so a tab the user moves keeps its place:
/// darktable's names for its groups, as a module's id is its operation's.
const TAB_IDS: [&str; TABS.len()] = ["photos", "base", "color", "correct", "effect", "history"];

/// The Photos tab, first: Browse so far.
const PHOTOS: usize = 0;

/// The first tab holding Browse, which View › Browse chooses (decision 73): the Photos tab,
/// until workspaces let the tabs be changed (`docs/gui.md`, Workspaces).
const BROWSE: usize = PHOTOS;

/// Whether each tab can be chosen: History has nothing behind it until it is built, so it
/// is drawn disabled, its key unavailable.
const TABS_ENABLED: [bool; TABS.len()] = [true, true, true, true, true, false];

fn tab_id(tab: usize) -> Option<&'static str> {
    TAB_IDS.get(tab).copied()
}

/// The widths the side panel can be dragged to, in points, as the settings schema has them:
/// down to 240 and up to 600; it opens at the theme's 320.
const PANEL_WIDTHS: std::ops::RangeInclusive<f32> = settings::registry::SIDE_PANEL_WIDTH.range();

/// The widths the side panel may take in a window `window` points wide: [`PANEL_WIDTHS`],
/// but never more than half the window, so the photo keeps the other half, and never less
/// than the narrowest, where half is less: how the shell gives way below 960 points is
/// still to be designed (decision 39).
fn panel_widths(window: f32) -> std::ops::RangeInclusive<f32> {
    let (least, most) = (*PANEL_WIDTHS.start(), *PANEL_WIDTHS.end());
    least..=most.min((window / 2.0).floor()).max(least)
}

/// Whether the Settings dialog offers `locale`: the pseudo-locale is for checking layouts,
/// so only `debug` builds have it, and a release build that reads it uses the system's
/// language.
fn offered(locale: Locale, debug: bool) -> bool {
    locale != Locale::Pseudo || debug
}

/// The modules each tab shows, by id: each in darktable's group, those darktable puts in
/// none in the group its other layouts give them, and listed as darktable's right panel
/// lists them, in pipeline order with the last module at the top (decision 38,
/// `docs/modules/README.md`). Customise tabs edits these lists once settings keep them
/// (`docs/gui.md`, The shell).
const TAB_MODULES: [&[&str]; TABS.len()] = [
    &[],
    &[
        "agx",
        "tone_curve",
        "toneequal",
        "exposure",
        "flip",
        "demosaic",
        "highlights",
        "temperature",
        "rawprepare",
    ],
    &[
        "colorout",
        "colorbalancergb",
        "primaries",
        "channelmixerrgb",
    ],
    &["hazeremoval"],
    &["vignette", "soften", "grain"],
    &[],
];

/// Whether each tab pins Scopes over its modules: every tab but the Photos tab, as
/// darktable's histogram sits over its groups (`docs/gui.md`, The tabs: darktable's
/// groups).
const SCOPES_PINNED: [bool; TABS.len()] = [false, true, true, true, true, true];

struct App {
    session: Session,
    /// What the session's events have reported (`mirror.rs`, `docs/gui.md`, A frame).
    mirror: Mirror,
    /// The photo area's size for the engine, in physical pixels.
    bounds: Bounds,
    /// The window's size at the last frame, in points, to tell when it is resizing.
    window_size: egui::Vec2,
    /// The side panel's width at the last frame, in points, to tell when its edge moves.
    panel_width: f32,
    /// The grid panel's place and lines at the last frame, to tell when it moves.
    grid_lines: (GridPosition, usize),
    shell: Shell,
    /// Each module section's open state and tab.
    panels: Panels,
    /// Whether tagging the window's colour space failed, reported once.
    tag_failed: bool,
    /// Whether placing the window's buttons in the top panel failed, reported once.
    buttons_failed: bool,
    /// Whether the last frame handed a window move to the system, whose release egui
    /// may then never see ([`release_after_drag`]).
    handed_off: bool,
    /// Which way a press in the bands inside the window's edges resizes it, taken from
    /// egui's input before the frame ([`resize_press`]) and sent in it.
    resize: Option<egui::ResizeDirection>,
    /// Writes the shell's settings once they stand still, and on exit.
    keeper: Keeper,
    /// The Mac's menu bar (decision 39); none elsewhere, nor where muda failed.
    menus: Option<menus::NativeMenus>,
    /// The open dialog's answer on its way, while it asks.
    asking: Option<dialogs::Asking>,
    /// What was dropped onto the window, while it is looked at (`dropped`).
    drops: dropped::Drops,
    /// The files the system asked the app to open, Open With's, while they are looked at.
    opened: dropped::Drops,
    /// The Pictures folder, where the open dialog starts with no photo open; read once.
    pictures: Option<PathBuf>,
    /// The dialogs' windows (decision 41).
    dialogs: dialog_host::AppDialogs,
    /// The jobs as the spinner and the jobs window draw them, rebuilt each frame.
    jobs_view: Vec<apertrust_ui::components::Job>,
    /// The person's folders and name, which the export's variables give; read once.
    places: Places,
    /// Export Again, between its preview and the answer to its question.
    again: export_again::Again,
    /// Whether Quit waits for the open dialog's answer ([`hold_for_panel`]).
    quit_held: bool,
    /// The alert asked and not yet answered (`docs/gui.md`, Dialogs and alerts).
    alerts: alerts::Alerts,
    /// Whether Quit or Close Window asks about the jobs running, and what a yes does.
    quit: alerts::Guard,
    /// Quit is to run again before the next frame: the quit alert was answered yes, or
    /// could not be shown.
    quit_again: bool,
    /// The texture of the mirror's picture of a photo opening, with its generation
    /// (decision 77).
    picture: Option<(u64, egui::TextureHandle)>,
    /// That picture fading out over the photo's first live frame, its texture taken from
    /// `picture` (decision 77).
    fade: Option<fade::Fade>,
    /// What the colour picker's live region reads out (decision 79).
    said: pickers::Said,
    /// The grid panel's thumbnails as textures, made of what the mirror holds.
    thumbnails: thumbnails::Textures,
    /// The thumbnails asked for of the photos in the grid panel's sight.
    thumbnails_asked: thumbnails::Asking,
    /// The session's log kept in memory, while Settings says so, for the log window.
    kept: apertrust_base::log::Kept,
    /// The log window's save dialog, while it asks.
    saving: Option<dialogs::Saving>,
    /// When the session started, to the minute, which names a saved log.
    started: apertrust_export::variables::DateTime,
    /// The Downloads folder, where the log's save dialog first opens; read once.
    downloads: Option<PathBuf>,
    /// The disk cache's limit the session was last given, sent again as its settings
    /// change it (decision 76).
    cache_limit: Limit,
    /// From which size a photo with no edit shows a render of its raw, as the session was
    /// last given it.
    raw_thumbnails: RawThumbnails,
    /// Whether the main window had the focus at the last frame.
    focused: bool,
    /// The folders the app started with, which Settings › Storage shows.
    storage_folders: storage::Folders,
    /// What Settings › Storage was last handed, so it is handed again only when it changes.
    storage_shown: storage::View,
    /// What Settings › Processing's Hardware acceleration group is handed: the adapter in
    /// use, how to list the others, and the values this run started with.
    hardware: hardware::Running,
    /// Restart now, once asked: the app quits, and `main` starts it again (decision 82).
    restart: restart::Restart,
    /// What the window says after a start that did not finish, until a button is pressed.
    notice: Option<recovery::Notice>,
    /// Whether Keep was pressed while `--conf` sets the graphics card or API, which then
    /// cannot change: the notice stays, saying why.
    notice_held: bool,
    /// This run's start marker, removed once a frame has been drawn into the window.
    marker: recovery::Pending,
    /// What was open last, while it reopens at the start ([`reopened`]).
    reopening: Option<Reopening>,
    /// The photo last active in each folder this run, and the one the folder changed to
    /// waits to open ([`Remembered`]).
    remembered: Remembered,
    /// The main window as the start window, with nothing open (`docs/gui.md`, The start
    /// window).
    start: start::Window,
}

/// What the top panel asked for this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FromTopPanel {
    /// A registry action, from its button.
    Action(ActionId),
    /// Something of the window's, from the panel's empty areas where it is the title bar.
    Window(TitleBar),
    /// The jobs spinner was pressed: the jobs window opens, or comes to the front.
    Jobs,
    /// The save warning was pressed: why the edit is not saved shows, with Try Again.
    NotSaved,
}

/// What the top panel's empty areas, and its caption buttons where it draws them, ask of
/// the window where the panel is its title bar. Close is the Quit action's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TitleBar {
    Move,
    DoubleClick,
    /// A right-click: the window's menu, where the system has one.
    Menu,
    Minimise,
    MaximiseOrRestore,
}

/// What the window's platform does for the top panel's gestures, through
/// `apertrust-display`; the tests stand in for it.
trait WindowControl {
    /// What a double-click on a title bar does, by the system's setting.
    fn double_click(&self) -> DoubleClick;
    /// Zooms the window, or returns it to its size before, where the platform does it
    /// itself; false where egui's `Maximized` does it.
    fn zoom(&mut self) -> bool;
    /// Starts moving the window with the pointer where the platform does it itself;
    /// false where egui's `StartDrag` does it.
    fn drag(&mut self) -> bool;
    /// Opens the window's menu at `at`, in physical pixels from the window's top left,
    /// where the system has one: the compositor's on Wayland, none on X11 or macOS.
    fn menu(&mut self, at: egui::Pos2);
}

/// The window's platform, as the app runs on it.
struct System<'a> {
    frame: &'a eframe::Frame,
    /// Whether a move was handed to the system, which may keep the press's release
    /// from egui ([`release_after_drag`]).
    handed_off: bool,
}

impl WindowControl for System<'_> {
    fn double_click(&self) -> DoubleClick {
        apertrust_display::title_bar_double_click()
    }

    fn zoom(&mut self) -> bool {
        match apertrust_display::zoom_window(self.frame) {
            Ok(zooms) => zooms,
            Err(e) => {
                tracing::warn!(target: "gpu", "window zoom left to egui: {e}");
                false
            }
        }
    }

    fn drag(&mut self) -> bool {
        match apertrust_display::start_window_drag(self.frame) {
            Ok(Some(start)) => {
                let (x, y) = start.at;
                tracing::debug!(target: "gpu", "window move: {} at {x:.0},{y:.0}, AppKit in: {}", start.handed, start.current);
                self.handed_off = true;
                true
            }
            Ok(None) => false,
            Err(e) => {
                tracing::warn!(target: "gpu", "window move left to egui: {e}");
                false
            }
        }
    }

    fn menu(&mut self, at: egui::Pos2) {
        use egui_winit::winit::dpi::PhysicalPosition;
        if let Some(window) = self.frame.winit_window() {
            let (x, y) = (f64::from(at.x), f64::from(at.y));
            window.show_window_menu(PhysicalPosition::new(x, y));
        }
    }
}

/// The room the top panel keeps at its start for the window's buttons, in egui's points at
/// the interface's scale `zoom`, from where `apertrust-display` left them (`buttons`, in
/// the window system's points, `None` where it could not reach the window): none where
/// the top panel is not the title bar; the room measured, whether it placed them or not;
/// and where the window could not be reached, the room AppKit's own title bar gives them
/// where the app draws its own (`own_title_bar`), unless in full screen, so the panel stays
/// the title bar and the window can still be moved.
fn button_room(
    buttons: Option<WindowButtons>,
    zoom: f32,
    full_screen: bool,
    own_title_bar: bool,
) -> Option<f32> {
    match buttons {
        Some(WindowButtons::None) => None,
        Some(WindowButtons::Placed(room) | WindowButtons::Unplaced(room)) => Some(room / zoom),
        None => (own_title_bar && !full_screen).then_some(BUTTONS_ROOM / zoom),
    }
}

/// Gives egui the release of the press a window move was handed off from, if this
/// frame's input has none and egui still holds the button down: the system moves the
/// window and may keep the release from the app, and while egui thinks the button is
/// down it shows no tooltip until the next click.
fn release_after_drag(ctx: &egui::Context, input: &mut egui::RawInput) {
    let primary = egui::PointerButton::Primary;
    let released = input.events.iter().any(|event| {
        matches!(event, egui::Event::PointerButton { button, pressed: false, .. } if *button == primary)
    });
    let (down, at) = ctx.input(|i| (i.pointer.button_down(primary), i.pointer.latest_pos()));
    if let (false, true, Some(pos)) = (released, down, at) {
        let release = egui::Event::PointerButton {
            pos,
            button: primary,
            pressed: false,
            modifiers: ctx.input(|i| i.modifiers),
        };
        input.events.insert(0, release);
    }
}

/// Takes the main window's input away under a modal dialog, so nothing in it reacts: no
/// key, click, scroll or text reaches it, and the pointer is gone, so nothing shows
/// hovered. What the window itself reports, its focus, stays. The pointer goes only with
/// what was taken, since an event each frame would ask for the next one, and the window
/// would draw for ever under the dialog.
fn under_modal(input: &mut egui::RawInput) {
    let events = input.events.len();
    input
        .events
        .retain(|event| matches!(event, egui::Event::WindowFocused(_)));
    if input.events.len() < events {
        input.events.push(egui::Event::PointerGone);
    }
    input.hovered_files.clear();
    input.dropped_files.clear();
}

/// Whether a press on the main window, or its taking the focus, brings the window over it
/// forward on `os` ([`dialog_host::Dialogs::raise`]): under a modal window (`modal`), but
/// not while the system's alert asks (`alert`), which would lose the front to the dialog;
/// an alert the host draws ([`dialog_host::platform::draws_alerts`]) is itself the window
/// brought forward.
fn raises_under_modal(modal: bool, alert: bool, os: OperatingSystem) -> bool {
    modal && (!alert || dialog_host::platform::draws_alerts(os))
}

/// Whether `input` reaches for the main window under a modal dialog: a press of a pointer's
/// button in it, or its taking the focus.
fn reaches_under_modal(input: &egui::RawInput) -> bool {
    input.events.iter().any(|event| {
        matches!(
            event,
            egui::Event::PointerButton { pressed: true, .. } | egui::Event::WindowFocused(true)
        )
    })
}

/// Does what the top panel's empty areas or its caption buttons asked of the window, as its
/// title bar would, after the frame: a drag moves it, a double-click does what the
/// system's setting says, a right-click opens the window's menu where the pointer is, and
/// the buttons minimise it, or maximise or restore it. The platform zooms the window from
/// its run loop, not within the frame, so each step of the animation is drawn
/// (`apertrust_display::zoom_window`); elsewhere egui's `Maximized` does.
fn run_title_bar(gesture: TitleBar, ctx: &egui::Context, window: &mut impl WindowControl) {
    let maximise_or_restore = || {
        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    };
    match gesture {
        TitleBar::Move => {
            if !window.drag() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
        }
        TitleBar::DoubleClick => match window.double_click() {
            DoubleClick::Zoom => {
                if !window.zoom() {
                    maximise_or_restore();
                }
            }
            DoubleClick::Minimise => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
            DoubleClick::Nothing => {}
        },
        TitleBar::Menu => {
            if let Some(at) = ctx.input(|i| i.pointer.interact_pos()) {
                window.menu(at * ctx.pixels_per_point());
            }
        }
        TitleBar::Minimise => ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
        TitleBar::MaximiseOrRestore => maximise_or_restore(),
    }
}

/// Which way the window resizes from `at`, in the bands `band` wide inside the edges of
/// `window`, its rectangle in points: [`RESIZE_BAND`] for the main window, wider for a
/// dialog's whose card sits inside a margin (`dialog_host::resize_band`); a corner where two
/// bands meet. None elsewhere.
fn resize_direction(
    window: egui::Rect,
    at: egui::Pos2,
    band: f32,
) -> Option<egui::ResizeDirection> {
    use egui::ResizeDirection as Way;
    if !window.contains(at) {
        return None;
    }
    let west = at.x < window.left() + band;
    let east = at.x >= window.right() - band;
    let north = at.y < window.top() + band;
    let south = at.y >= window.bottom() - band;
    match (north, south, west, east) {
        (true, _, true, _) => Some(Way::NorthWest),
        (true, _, _, true) => Some(Way::NorthEast),
        (_, true, true, _) => Some(Way::SouthWest),
        (_, true, _, true) => Some(Way::SouthEast),
        (true, ..) => Some(Way::North),
        (_, true, ..) => Some(Way::South),
        (_, _, true, _) => Some(Way::West),
        (_, _, _, true) => Some(Way::East),
        _ => None,
    }
}

/// The cursor that says which way the window resizes.
fn resize_cursor(way: egui::ResizeDirection) -> egui::CursorIcon {
    use egui::{CursorIcon, ResizeDirection as Way};
    match way {
        Way::North => CursorIcon::ResizeNorth,
        Way::South => CursorIcon::ResizeSouth,
        Way::East => CursorIcon::ResizeEast,
        Way::West => CursorIcon::ResizeWest,
        Way::NorthEast => CursorIcon::ResizeNorthEast,
        Way::SouthEast => CursorIcon::ResizeSouthEast,
        Way::NorthWest => CursorIcon::ResizeNorthWest,
        Way::SouthWest => CursorIcon::ResizeSouthWest,
    }
}

/// Whether the window, as `viewport` describes it, can be resized from its edges: not
/// while it is maximised or in full screen.
fn resizable(viewport: &egui::ViewportInfo) -> bool {
    !viewport.maximized.unwrap_or(false) && !viewport.fullscreen.unwrap_or(false)
}

/// Takes from `input` a press of the primary button in the bands inside the window's
/// edges, where it can be resized, so no widget under a band sees it, and returns which
/// way the window is to resize. The compositor or window manager then resizes it and may
/// keep the release, which egui, never having seen the press, ignores.
fn resize_press(
    ctx: &egui::Context,
    input: &mut egui::RawInput,
    band: f32,
) -> Option<egui::ResizeDirection> {
    if !resizable(input.viewport()) {
        return None;
    }
    let window = input.screen_rect.unwrap_or_else(|| ctx.viewport_rect());
    let mut way = None;
    input.events.retain(|event| match event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            ..
        } => match resize_direction(window, *pos, band) {
            Some(direction) => {
                way = Some(direction);
                false
            }
            None => true,
        },
        _ => true,
    });
    way
}

/// Shows the resize cursor over the bands inside the window's edges where it can be
/// resized, in place of any a widget under them asked for, while no button is down.
fn resize_hover(ctx: &egui::Context, band: f32) {
    let (can, at, down, window) = ctx.input(|i| {
        let pointer = &i.pointer;
        let viewport = resizable(i.viewport());
        (
            viewport,
            pointer.latest_pos(),
            pointer.any_down(),
            i.viewport_rect(),
        )
    });
    if let (true, false, Some(at)) = (can, down, at)
        && let Some(way) = resize_direction(window, at, band)
    {
        ctx.set_cursor_icon(resize_cursor(way));
    }
}

/// The top panel's buttons and the actions they run.
const TOP_PANEL: [(TopPanelAction, ActionId); 9] = [
    (TopPanelAction::Menu, ActionId::MENU),
    (TopPanelAction::About, ActionId::ABOUT),
    (TopPanelAction::Import, ActionId::IMPORT),
    (TopPanelAction::Export, ActionId::EXPORT),
    (TopPanelAction::Reset, ActionId::RESET),
    (TopPanelAction::Undo, ActionId::UNDO),
    (TopPanelAction::Redo, ActionId::REDO),
    (TopPanelAction::CopyEdit, ActionId::COPY_EDIT),
    (TopPanelAction::ApplyEdit, ActionId::APPLY_EDIT),
];

/// The central bar's zoom menu entries that are actions too, whose keys they show.
const ZOOM_MENU: [(CentralBarAction, ActionId); 4] = [
    (CentralBarAction::Zoom(Zoom::Fit), ActionId::ZOOM_FIT),
    (
        CentralBarAction::Zoom(Zoom::Percent(100.0)),
        ActionId::ZOOM_100,
    ),
    (CentralBarAction::ZoomIn, ActionId::ZOOM_IN),
    (CentralBarAction::ZoomOut, ActionId::ZOOM_OUT),
];

/// The shell's own state (`docs/gui.md`, A frame), part of it kept in settings.
struct Shell {
    view: View,
    panel_tab: usize,
    tab_names: bool,
    /// The side panel's width as the user dragged it, in points; None for the theme's.
    /// Kept while a narrower window shows the panel narrower ([`panel_widths`]).
    panel_width: Option<f32>,
    guides: bool,
    proof_margin: bool,
    /// The central panel's margins, from the settings (`[central_panel]`).
    central_panel: settings::CentralPanel,
    /// Whether Settings… ran, for the app to open it in the dialog host after the action.
    settings: bool,
    /// Whether Show Log ran, for the app to open the log window after the action.
    show_log: bool,
    /// Whether View › Jobs ran, for the app to open the jobs window after the action.
    show_jobs: bool,
    about: about::About,
    /// Whether Open… ran, for the app to show the open dialog after the frame.
    open: bool,
    /// Whether Open Folder… ran, for the app to show the folder dialog after the frame.
    open_folder: bool,
    /// The recent folder Open Recent chose, for the app to open after the action.
    open_recent: Option<PathBuf>,
    /// The recents' paths as Open Recent last showed them, newest first: what a choice of
    /// its item at a place opens, though the list has changed since ([`Shell::show_recents`]).
    menu_recents: Vec<PathBuf>,
    /// Whether File › Close ran, for the app to close what is open after the action.
    close: bool,
    /// Whether Quit, or Close Window on the main window, asked the window to close, which
    /// ends the app: the start window's own close on the Mac does not ([`App`]'s
    /// `raw_input_hook`).
    leaving: bool,
    /// What was opened, newest first, kept in `state.toml`: Browse's recent folders and
    /// Open Recent's list, with what was open last beside them.
    recents: Recents,
    /// Whether Browse was shown at the last frame, so the recents are checked again as it
    /// comes into sight.
    browse_shown: bool,
    /// The folders the user opened (true) or closed (false) in Browse's tree, for this run:
    /// neither a folder chosen in the tree nor a way in changes them.
    tree: BTreeMap<PathBuf, bool>,
    /// The missing recent Locate… asked about, for the folder dialog after the frame.
    locate: Option<PathBuf>,
    /// The recent folders that could not be opened when tried this run, and why.
    missing: BTreeMap<PathBuf, String>,
    /// Whether Export… ran, for the app to open the Export dialog's sheet.
    export_asked: bool,
    /// Whether Export Again ran, for the app to send the preview that decides its question.
    export_again: bool,
    /// The Export dialog's settings, as it left them (`export.*`).
    export: export::Values,
    /// Whether Export and Export Again ask before replacing files
    /// (`export.ask_before_overwrite`).
    overwrite: alerts::Overwrite,
    /// Whether the menus are the ☰'s, drawn: on Windows and Linux (decision 39), where the
    /// frameless window has no menu bar. The Mac has its own.
    drawn_menus: bool,
    /// Whether the ☰ or F10 asked for its menu, opened after the frame, or closed there
    /// where it is open ([`Shell::open_menu`]).
    menu: bool,
    /// Whether a key ran an action while the menu was open, so it closes after the frame
    /// ([`Shell::close_menu`]).
    close_menu: bool,
    /// What Undo and Redo would do, as the menus name it, from the session's history
    /// (`history::steps`), kept up to date by the app before each frame.
    steps: menus::Steps,
    /// The keys Undo or Redo hand a focused text field when chosen from a menu, for the
    /// next frame's input ([`actions::field_keys`], the app's `raw_input_hook`).
    field_keys: Vec<egui::Event>,
    theme: Builtin,
    /// The marker on the sliders' tracks and the curve's handles (decision 75).
    marker: Marker,
    /// The interface's language: the user's choice, or the system's.
    locale: Locale,
    /// The language settings keep: none until the user chooses one, so the interface
    /// follows the system's. Kept as read even when this build does not offer it.
    kept_locale: Option<Locale>,
    /// The system's language, the interface's while settings keep none.
    system_locale: Locale,
    /// Each action's keys: the platform's defaults, with the user's from settings.
    keymap: Keymap,
    /// The sections the user opened or closed in Edit, by id.
    sections: BTreeMap<String, bool>,
    /// The collapsible groups the user opened or closed, by module and group.
    groups: BTreeMap<String, BTreeMap<String, bool>>,
    /// What the Scopes section shows, as the user left it.
    scopes: ScopeView,
    /// How the grid panel shows the folder: its sort and its thumbnails' size.
    grid: settings::Grid,
    /// The window's placement as it last stood while neither maximised nor full screen.
    window: settings::Window,
    /// Where Settings' own window was last left, on the Mac.
    settings_window: Option<settings::Position>,
    /// Where the jobs window was last left.
    jobs_window: Option<settings::Position>,
    /// The log kept in memory, as Settings' Diagnostics has it, and its window's place and
    /// folder.
    log: settings::Log,
    /// The tab the shell opened on, and Edit's tab as settings had it, which stays until
    /// the user chooses another, even one this build does not have or cannot choose yet;
    /// None once the user has chosen a tab, so the id read is never written back.
    opening_tab: Option<(usize, Option<String>)>,
    /// Whether the window is still to be maximised once it shows ([`Shell::maximise`]).
    maximise: bool,
}

impl Shell {
    /// The shell as `settings` left it, in `system`'s language unless they keep another.
    /// A key override that would clash with another action's key is refused, with a
    /// warning, and so is a tab that is not there, as an older build's (`adjust`), which
    /// opens Base; so does a tab that cannot be chosen yet, without a warning. Either id
    /// stays in settings until the user chooses a tab.
    fn new(settings: &Settings, system: Locale, os: OperatingSystem) -> Shell {
        let (keymap, warnings) = Keymap::load(os, &settings.keys);
        for warning in warnings {
            tracing::warn!(target: "settings", "{warning}");
        }
        let tab = settings.edit.tab.as_deref();
        let panel_tab = tab.and_then(|id| (0..TABS.len()).find(|i| tab_id(*i) == Some(id)));
        if let (Some(id), None) = (tab, panel_tab) {
            tracing::warn!(target: "settings", "edit.tab: no tab {id}, Base shown");
        }
        let panel_tab = panel_tab.filter(|tab| TABS_ENABLED[*tab]).unwrap_or(BASE);
        let locale = settings
            .locale
            .filter(|l| offered(*l, cfg!(debug_assertions)))
            .unwrap_or(system);
        Shell {
            view: View::Edit,
            panel_tab,
            tab_names: settings.tab_names,
            panel_width: settings.panel_width,
            guides: false,
            proof_margin: false,
            central_panel: settings.central_panel,
            settings: false,
            show_log: false,
            show_jobs: false,
            about: about::About::default(),
            open: false,
            open_folder: false,
            open_recent: None,
            menu_recents: Vec::new(),
            close: false,
            leaving: false,
            recents: Recents::nowhere(),
            browse_shown: false,
            tree: BTreeMap::new(),
            locate: None,
            missing: BTreeMap::new(),
            export_asked: false,
            export_again: false,
            export: settings.export.clone(),
            overwrite: alerts::Overwrite::new(settings.ask_before_overwrite, false),
            drawn_menus: os != OperatingSystem::Mac,
            menu: false,
            close_menu: false,
            steps: menus::Steps::default(),
            field_keys: Vec::new(),
            theme: settings.theme,
            marker: settings.marker,
            locale,
            kept_locale: settings.locale,
            system_locale: system,
            keymap,
            sections: settings.edit.sections.clone(),
            groups: settings.edit.groups.clone(),
            scopes: settings.scopes,
            grid: settings.grid,
            window: settings.window,
            settings_window: settings.settings_window,
            jobs_window: settings.jobs_window,
            log: settings.log.clone(),
            opening_tab: Some((panel_tab, settings.edit.tab.clone())),
            maximise: MAXIMISES && settings.window.maximized,
        }
    }

    /// What settings keep of the shell, and of the interface's scale, which egui holds.
    fn to_settings(&self, ctx: &egui::Context) -> Settings {
        let tab = match &self.opening_tab {
            Some((opening, read)) if self.panel_tab == *opening => read.clone(),
            _ => tab_id(self.panel_tab).map(str::to_owned),
        };
        Settings {
            theme: self.theme,
            locale: self.kept_locale,
            scale: ctx.zoom_factor(),
            marker: self.marker,
            window: self.window,
            settings_window: self.settings_window,
            jobs_window: self.jobs_window,
            log: self.log.clone(),
            tab_names: self.tab_names,
            panel_width: self.panel_width,
            central_panel: self.central_panel,
            edit: settings::ViewState {
                tab,
                sections: self.sections.clone(),
                groups: self.groups.clone(),
            },
            scopes: self.scopes,
            grid: self.grid,
            keys: self.keymap.overrides().clone(),
            export: self.export.clone(),
            ask_before_overwrite: self.overwrite.asks(),
        }
    }

    /// Follows the window's placement as the next start will place it: its inner size in
    /// logical pixels, its outer position in physical ones with the display's scale factor,
    /// and whether it is maximised. While it is maximised, full screen or minimised, its
    /// last normal size and position stay; on macOS, where egui does not follow the zoom,
    /// whether it is maximised stays as read.
    fn track_window(&mut self, ctx: &egui::Context) {
        let zoom = ctx.zoom_factor();
        let maximise = self.maximise;
        let window = &mut self.window;
        ctx.input(|input| {
            let viewport = input.viewport();
            let full_screen = viewport.fullscreen.unwrap_or(false);
            if full_screen || viewport.minimized.unwrap_or(false) {
                return;
            }
            let maximized = MAXIMISES && viewport.maximized.unwrap_or(false);
            // Until it shows, a window to be maximised may not be yet.
            if MAXIMISES && !maximise {
                window.maximized = maximized;
            }
            if maximized {
                return;
            }
            let size = input.viewport_rect().size() * zoom;
            window.size = Some([size.x.round(), size.y.round()]);
            if let (Some(outer), Some(native)) =
                (viewport.outer_rect, viewport.native_pixels_per_point)
            {
                let pixels = outer.min * (zoom * native);
                window.position = Some(settings::Position {
                    pixels: [pixels.x.round(), pixels.y.round()],
                    scale_factor: native,
                });
            }
        });
    }

    /// Maximises the window once it shows, if settings say it was: X11 window managers
    /// ignore maximising a window before it shows, and eframe shows it after its first
    /// frame.
    fn maximise(&mut self, ctx: &egui::Context) {
        if !self.maximise || ctx.cumulative_frame_nr() < 1 {
            return;
        }
        self.maximise = false;
        if ctx.input(|i| i.viewport().maximized) == Some(false) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
        }
    }

    /// A folder opened as a way in: first among the recents, once, at most ten, and no
    /// longer missing, since it opened, and what was open last, with no folder chosen or
    /// photo active yet, unless it is that already, whose folder and photo stay, so a reopen
    /// under way keeps them; written on the recents' own thread. A photo opened alone is
    /// never a folder opened.
    fn remember_folder(&mut self, path: PathBuf) {
        self.missing.remove(&path);
        if self.recents.last().is_none_or(|last| last.path != path) {
            self.recents.set_last(Some(recents::Last {
                kind: recents::OpenKind::Folder,
                path: path.clone(),
                folder: None,
                photo: None,
            }));
        }
        self.recents.opened(recents::OpenKind::Folder, path);
        self.recents.save();
    }

    /// Opens or closes `folder` in Browse's tree, for the run; `root` says it is the tree's
    /// root, which starts open, the others closed. Returns whether it is open now.
    fn toggle_tree(&mut self, folder: &Path, root: bool) -> bool {
        let now = !self.tree.get(folder).copied().unwrap_or(root);
        self.tree.insert(folder.to_owned(), now);
        now
    }

    /// Remove from the List: `path` taken out of the recents, and its failure this run
    /// forgotten.
    fn forget_recent(&mut self, path: &Path) {
        self.missing.remove(path);
        self.recents.remove(recents::OpenKind::Folder, path);
        self.recents.save();
    }

    /// Locate…'s answer: the missing recent `from` points to `to`, keeping its place, and
    /// is asked about again. Returns `to`, the folder to open.
    fn located(&mut self, from: &Path, to: PathBuf) -> PathBuf {
        self.missing.remove(from);
        self.recents
            .repoint(recents::OpenKind::Folder, from, to.clone());
        self.recents.save();
        self.recents.check();
        to
    }

    /// The recent folders, each with why it cannot be opened, if it cannot: its opening
    /// failed this run, or the recents' check found it gone. Nothing is asked of the disk as
    /// the list is drawn: a share can be slow, and the interface never waits for one.
    fn recent_folders(&self) -> Vec<library::Recent<'_>> {
        let folders = self.recents.entries();
        folders
            .filter(|entry| entry.kind == recents::OpenKind::Folder)
            .map(|entry| {
                let path = entry.path.as_path();
                let failed = self
                    .missing
                    .get(path)
                    .map(|why| library::Missing::Failed(why));
                let gone = self.recents.missing(entry.kind, path);
                library::Recent {
                    kind: entry.kind,
                    path,
                    missing: failed.or(gone.then_some(library::Missing::Gone)),
                }
            })
            .collect()
    }

    /// Open Recent's items, newest first, from the recents as they are: each folder's
    /// label ([`recents::OpenKind::label`]), its name with the folders it is in, the nearest
    /// first, as far up as it takes to tell it from another recent ([`told_apart`]), and its
    /// path in the tip, after why it cannot be opened where it cannot. Nothing is asked of
    /// the disk.
    fn recent_items(&self, ctx: &egui::Context) -> Vec<menus::Recent> {
        let tr = translator(ctx);
        let recents = self.recent_folders();
        let paths: Vec<&Path> = recents.iter().map(|recent| recent.path).collect();
        recents
            .iter()
            .zip(told_apart(&paths))
            .map(|(recent, names)| {
                let mut names = names.into_iter();
                let first = names.next().unwrap_or_default();
                let name = names.fold(first, |words, parent| {
                    let mut args = apertrust_ui::i18n::Args::new();
                    args.set("name", words);
                    args.set("parent", parent);
                    tr.text_with("menu-recent-in", Some(&args))
                });
                let words = recent.kind.label(&tr, &name);
                let path = recent.path.display().to_string();
                let tip = match recent.missing {
                    Some(missing) => format!("{}\n{path}", library::missing_words(ctx, missing)),
                    None => path,
                };
                let path = recent.path.to_owned();
                menus::Recent { words, tip, path }
            })
            .collect()
    }

    /// Open Recent's items as a menu shows them from now ([`Shell::recent_items`]), their
    /// paths kept for its choices: as the ☰'s menu is built, and as each frame fills the
    /// Mac's bar.
    fn show_recents(&mut self, ctx: &egui::Context) -> Vec<menus::Recent> {
        let items = self.recent_items(ctx);
        self.menu_recents = items.iter().map(|item| item.path.clone()).collect();
        items
    }

    /// Whether each of Open Recent's items as last shown can be opened ([`recent_states`]).
    fn recent_states(&self) -> [Option<bool>; recents::MOST] {
        recent_states(&self.recents, &self.menu_recents)
    }

    /// The keys that open a folder and a raw, as the platform writes them.
    fn open_keys(&self, ctx: &egui::Context) -> library::Keys {
        let words = translator(ctx);
        let os = ctx.os();
        library::Keys {
            folder: self.keymap.tip(ActionId::OPEN_FOLDER, os, &words),
            files: self.keymap.tip(ActionId::OPEN, os, &words),
        }
    }

    /// Whether the section `id` is open: as the user left it, or `default`.
    fn section_open(&self, id: &str, default: bool) -> bool {
        self.sections.get(id).copied().unwrap_or(default)
    }

    /// Keeps the section `id` open or closed, for this run and the next.
    fn set_section_open(&mut self, id: &str, open: bool) {
        if self.sections.get(id) != Some(&open) {
            self.sections.insert(id.to_owned(), open);
        }
    }

    /// Whether `module`'s collapsible group `group` is open: as the user left it, or
    /// closed (decision 29).
    fn group_open(&self, module: &str, group: &str) -> bool {
        self.group_open_or(module, group, modules::GROUP_OPEN)
    }

    /// Whether `module`'s collapsible group `group` is open: as the user left it, or
    /// `default`, as Browse's Folders, open in folder mode, where it is Browse alone.
    fn group_open_or(&self, module: &str, group: &str, default: bool) -> bool {
        let kept = self.groups.get(module).and_then(|groups| groups.get(group));
        kept.copied().unwrap_or(default)
    }

    /// Opens or closes `module`'s collapsible group `group`, for this run and the next.
    /// Resetting the module leaves it as it is.
    fn toggle_group(&mut self, module: &str, group: &str) {
        let open = !self.group_open(module, group);
        self.set_group_open(module, group, open);
    }

    /// Keeps `module`'s collapsible group `group` open or closed, for this run and the next.
    fn set_group_open(&mut self, module: &str, group: &str, open: bool) {
        let groups = self.groups.entry(module.to_owned()).or_default();
        groups.insert(group.to_owned(), open);
    }

    /// Whether Browse and its Folders are open, as the user left them.
    fn browse_open(&self) -> library::Open {
        library::Open {
            browse: self.section_open(library::ID, library::OPEN),
            folders: self.group_open_or(library::ID, library::FOLDERS, library::FOLDERS_OPEN),
        }
    }

    /// Opens or closes Browse, or its Folders, as Browse's header or the Folders' row
    /// asked (`library::Action::Toggle`, `ToggleFolders`), for this run and the next.
    fn toggle_browse(&mut self, folders: bool) {
        let open = self.browse_open();
        if folders {
            self.set_group_open(library::ID, library::FOLDERS, !open.folders);
        } else {
            self.set_section_open(library::ID, !open.browse);
        }
    }

    /// Whether the side panel shows the Scopes section: the tab shown pins it.
    fn scopes_pinned(&self) -> bool {
        SCOPES_PINNED.get(self.panel_tab).copied().unwrap_or(false)
    }

    /// Applies what the Scopes section asked for, for this run and the next.
    fn scopes_change(&mut self, change: scopes::Change) {
        match change {
            scopes::Change::Toggle => {
                let open = self.section_open(scopes::ID, scopes::OPEN);
                self.set_section_open(scopes::ID, !open);
            }
            scopes::Change::View(action) => self.scopes.apply(action),
        }
    }

    /// What the session is to count of each frame for the Scopes section as it stands
    /// ([`scopes::wanted`]); asked whether a photo is open or not, so a photo's first frame
    /// is counted as it renders.
    fn scopes_wanted(&self) -> Option<Graphs> {
        let open = self.section_open(scopes::ID, scopes::OPEN);
        scopes::wanted(self.scopes_pinned(), open, self.scopes)
    }

    /// The side panel's edge, for the panel at the end of a window `window` points wide: at
    /// the width the user dragged it to, or the theme's, within [`panel_widths`]; `name`
    /// is `panel-width`'s message, for screen readers.
    fn panel_edge<'a>(&self, ctx: &egui::Context, window: f32, name: &'a str) -> PanelEdge<'a> {
        let width = self.panel_width.unwrap_or(theme(ctx).sizes.side_panel);
        PanelEdge::new(name, width, panel_widths(window))
    }

    /// The grid panel's edge, for a window `window` points in size: at the rows or columns
    /// the user dragged it to where it is, as many as fit in half the window's height at
    /// the bottom or half its width at a side (`docs/gui.md`, Its size).
    fn grid_edge(&self, ctx: &egui::Context, window: egui::Vec2) -> GridEdge {
        let (size, position) = (self.grid.thumbnails, self.grid.position);
        let room = if position.is_column() {
            window.x
        } else {
            window.y
        };
        let labels = self.grid.labels;
        let most = grid_panel_lines(ctx, position, size, labels, room / 2.0);
        GridEdge::new(self.grid.lines(), most, size)
            .labels(labels)
            .position(position)
    }

    /// The grid panel's egui panel where the shell keeps it, laid out after the side panel
    /// so that at the right it lies between the photo and the side panel: as thick as the
    /// lines its edge gives this frame, a drag in progress included. With it, the edge, to
    /// show after the central panel, and those lines.
    fn grid_frame(&self, ctx: &egui::Context) -> (egui::Panel, GridEdge, usize) {
        let grid = self.grid;
        let edge = self.grid_edge(ctx, ctx.content_rect().size());
        let lines = edge.lines(ctx);
        let panel = match grid.position {
            GridPosition::Bottom => egui::Panel::bottom("grid panel"),
            GridPosition::Left => egui::Panel::left("grid panel"),
            GridPosition::Right => egui::Panel::right("grid panel"),
        }
        .exact_size(grid_panel_thickness(
            ctx,
            grid.position,
            grid.thumbnails,
            grid.labels,
            lines,
        ))
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE);
        (panel, edge, lines)
    }

    /// Keeps the rows or columns the grid panel's edge was dragged to, or goes back to one.
    fn resize_grid(&mut self, action: GridEdgeAction) {
        self.grid.keep_lines(match action {
            GridEdgeAction::Resize(lines) => lines,
            GridEdgeAction::Reset => 1,
        });
    }

    /// Keeps the width the side panel's edge was dragged to, for this run and the next, or
    /// goes back to the theme's on a double-click.
    fn resize_panel(&mut self, action: PanelEdgeAction) {
        self.panel_width = match action {
            PanelEdgeAction::Resize(width) => Some(width),
            PanelEdgeAction::Reset => None,
        };
    }

    /// What the actions' availability depends on this frame; `photo` and `edited` come
    /// from the session mirror.
    fn state(&self, ctx: &egui::Context, photo: bool, edited: bool) -> State {
        let recents = self.recent_states();
        State {
            view: self.view,
            photo,
            edited,
            // The app's, from the photo's history ([`App::shell_state`]).
            undo: false,
            redo: false,
            text: Focus::of(ctx) == Focus::Text,
            // One photo until the catalogue.
            photos: usize::from(photo),
            // The app's, from the folder and the photo it holds ([`App::shell_state`]).
            opened: false,
            recents,
            tabs: &TABS_ENABLED,
            scale: ctx.zoom_factor(),
            dialog: self.about.is_open(),
            // About is modal over the window; the open dialog and the host's dialogs are the
            // app's ([`with_dialogs`]).
            modal: self.about.is_open(),
            dialog_window: false,
            full_screen: ctx.input(|i| i.viewport().fullscreen).unwrap_or(false),
            panel_tab: self.panel_tab,
            grid: self.grid,
            // The app's, from the folder it shows ([`App::shell_state`]).
            shown: false,
            background: Background::of(theme(ctx).colors.photo_surround),
        }
    }

    /// Alt with a top-level menu's access key, pressed this frame where the menus are
    /// drawn: opens the ☰'s menu with that menu expanded, before any widget sees the press
    /// (decision 39), unless a dialog is open.
    fn access_key(&mut self, ctx: &egui::Context, state: &State) {
        if !self.drawn_menus || state.dialog {
            return;
        }
        let Some(anchor) = TopPanel::menu_anchor(ctx) else {
            return;
        };
        let mut request = None;
        let mut shown = None;
        let opened = actions::access_key(ctx, |letter| {
            let request = request.get_or_insert_with(|| {
                let recents = self.recent_items(ctx);
                let paths: Vec<PathBuf> = recents.iter().map(|item| item.path.clone()).collect();
                let state = State {
                    recents: recent_states(&self.recents, &paths),
                    ..*state
                };
                shown = Some(paths);
                menus::request(ctx, &self.keymap, &state, &recents, &self.steps, anchor)
            });
            let menu = request.access_key(letter)?.to_owned();
            Some(request.clone().expanded(menu))
        });
        if let Some(request) = opened {
            self.menu_recents = shown.unwrap_or_default();
            popup::open(ctx, TopPanel::menu_popup(), request);
        }
    }

    /// Draws the open popups, once a frame on every platform, as the popup service needs,
    /// and returns the action chosen in the ☰'s menu, if one was this frame.
    fn popups(&self, ctx: &egui::Context) -> Option<ActionId> {
        popup::show(ctx);
        match popup::answer(ctx, TopPanel::menu_popup())? {
            popup::Answer::Chosen(id) => menus::chosen(&id),
            popup::Answer::Dismissed => None,
        }
    }

    /// Asks for the ☰'s menu to close after the frame ([`Shell::open_menu`]) when the
    /// frame's keys ran an action, `pressed`, but its own: the menu, built as it opened,
    /// would show what that action changed as it was before.
    fn close_menu(&mut self, pressed: &[ActionId]) {
        if pressed.iter().any(|id| *id != ActionId::MENU) {
            self.close_menu = true;
        }
    }

    /// After the frame's actions, with `state` as they left it: closes the ☰'s menu where
    /// a key ran an action, giving the focus back to what had it, and whenever a dialog is
    /// open, leaving the dialog the focus. Then opens it at its top, as the ☰ or F10 asked
    /// during the frame, under the ☰ as last drawn and with what can run in `state`; F10
    /// closes it where it is open.
    fn open_menu(&mut self, ctx: &egui::Context, state: &State) {
        let id = TopPanel::menu_popup();
        let close = std::mem::take(&mut self.close_menu);
        if state.dialog {
            self.menu = false;
            popup::dismiss(ctx, id);
            return;
        }
        if close {
            popup::close(ctx, id);
        }
        // Open Recent's choices go by the list it shows: the recents as they are while the
        // menu is closed, as it was built while open.
        if self.drawn_menus && !popup::is_open(ctx, id) {
            self.menu_recents = self
                .recent_folders()
                .iter()
                .map(|r| r.path.to_owned())
                .collect();
        }
        if !std::mem::take(&mut self.menu) || !self.drawn_menus {
            return;
        }
        if popup::is_open(ctx, id) {
            popup::close(ctx, id);
        } else if let Some(anchor) = TopPanel::menu_anchor(ctx) {
            let recents = self.show_recents(ctx);
            let state = State {
                recents: self.recent_states(),
                ..*state
            };
            let steps = &self.steps;
            let request = menus::request(ctx, &self.keymap, &state, &recents, steps, anchor);
            popup::open(ctx, id, request);
        }
    }

    /// The top panel, from the registry: what can't run is drawn disabled, and each tooltip
    /// carries its keys. With `title_bar`, the room the window's buttons take at its start,
    /// it is the window's title bar; with `caption`, it draws the caption buttons at its
    /// end, Restore in place of Maximise where `caption` says the window is maximised.
    /// Returns what was asked for, to run after the frame.
    fn top_panel(
        &self,
        ui: &mut egui::Ui,
        state: &State,
        saved: SaveState<'_>,
        title_bar: Option<f32>,
        caption: Option<bool>,
        jobs: &[apertrust_ui::components::Job],
    ) -> Option<FromTopPanel> {
        let ctx = ui.ctx().clone();
        let words = translator(&ctx);
        let can = |id| actions::action(id).is_some_and(|a| a.enabled(state));
        let available = Available {
            import: can(ActionId::IMPORT),
            export: can(ActionId::EXPORT),
            reset: can(ActionId::RESET),
            // The photo's history alone: a click takes the focus from a text field first.
            undo: state.undo && can(ActionId::UNDO),
            redo: state.redo && can(ActionId::REDO),
            copy: can(ActionId::COPY_EDIT),
            apply: can(ActionId::APPLY_EDIT),
        };
        let tips: Vec<(TopPanelAction, String)> = TOP_PANEL
            .iter()
            .filter_map(|(button, id)| Some((*button, self.keymap.tip(*id, ctx.os(), &words)?)))
            .collect();
        let mut bar = TopPanel::new()
            .menu_button(self.drawn_menus)
            .available(available)
            .jobs(Jobs::new(jobs).file_manager(jobs::FILE_MANAGER))
            .saved(saved);
        if let Some(inset) = title_bar {
            bar = bar.title_bar(true).start_inset(inset);
        }
        if let Some(maximised) = caption {
            bar = bar.caption_buttons(true).maximised(maximised);
        }
        for (button, keys) in &tips {
            bar = bar.shortcut(*button, keys);
        }
        let window = |gesture| Some(FromTopPanel::Window(gesture));
        match bar.show(ui)? {
            TopPanelAction::MoveWindow => window(TitleBar::Move),
            TopPanelAction::TitleBarDoubleClick => window(TitleBar::DoubleClick),
            TopPanelAction::WindowMenu => window(TitleBar::Menu),
            TopPanelAction::Minimise => window(TitleBar::Minimise),
            TopPanelAction::MaximiseOrRestore => window(TitleBar::MaximiseOrRestore),
            // Closing the window quits, through the app's way out.
            TopPanelAction::Close => Some(FromTopPanel::Action(ActionId::QUIT)),
            TopPanelAction::Jobs => Some(FromTopPanel::Jobs),
            TopPanelAction::NotSaved => Some(FromTopPanel::NotSaved),
            clicked => TOP_PANEL
                .iter()
                .find(|(button, _)| *button == clicked)
                .map(|(_, id)| FromTopPanel::Action(*id)),
        }
    }

    /// The side panel's tab bar, each tab's tooltip with its keys, the tabs that cannot be
    /// chosen yet drawn disabled. Returns the action of the tab clicked.
    fn tab_bar(&mut self, ui: &mut egui::Ui) -> Option<ActionId> {
        let ctx = ui.ctx().clone();
        let words = translator(&ctx);
        let names = TABS.map(|(_, id)| tr(&ctx, id));
        let tabs: Vec<PanelTab> = TABS
            .iter()
            .zip(&names)
            .zip(TABS_ENABLED)
            .enumerate()
            .map(|(i, (((icon, _), name), enabled))| PanelTab {
                icon: *icon,
                name,
                shortcut: ActionId::PANEL_TABS
                    .get(i)
                    .and_then(|id| self.keymap.tip(*id, ctx.os(), &words)),
                enabled,
            })
            .collect();
        match TabBar::new(&tabs, self.panel_tab, self.tab_names)
            .customise(false)
            .show(ui)
        {
            Some(TabBarAction::Select(i)) => ActionId::PANEL_TABS.get(i).copied(),
            Some(TabBarAction::ShowNames(on)) => {
                self.tab_names = on;
                None
            }
            Some(TabBarAction::Customise) | None => None,
        }
    }

    /// Runs `id` if it can run where it is in `state`: applies its intent, or returns its
    /// command for the session. Keys, clicks and the ☰'s menu's choices arrive here after
    /// the frame, the Mac's menus' choices before it (`App::logic`); `regions` are the
    /// frame's top panel, central bar, side panel and grid panel, for F6. Undo and Redo
    /// act on a text field with the focus, handing it its own keys for the next frame
    /// (decision 39).
    fn run(
        &mut self,
        id: ActionId,
        state: &State,
        ctx: &egui::Context,
        regions: &[egui::Rect],
    ) -> Option<Command> {
        let action = actions::action(id).filter(|a| a.enabled(state))?;
        if state.text
            && let Some(keys) = actions::field_keys(id, ctx.os())
        {
            self.field_keys.extend(keys);
            ctx.request_repaint();
            return None;
        }
        match &action.emits {
            Emit::Command(command) => Some(command.clone()),
            Emit::Intent(intent) => {
                self.apply(*intent, ctx, regions);
                None
            }
            Emit::Later => None,
        }
    }

    fn apply(&mut self, intent: Intent, ctx: &egui::Context, regions: &[egui::Rect]) {
        match intent {
            Intent::About => self.about.open(ctx),
            Intent::ShowLog => self.show_log = true,
            Intent::ShowJobs => self.show_jobs = true,
            Intent::Settings => self.settings = true,
            Intent::Open => self.open = true,
            Intent::OpenFolder => self.open_folder = true,
            // The folder its item showed, though the list has changed since.
            Intent::OpenRecent(place) => self.open_recent = self.menu_recents.get(place).cloned(),
            Intent::ClearRecent => {
                self.recents.clear();
                self.recents.save();
            }
            Intent::Close => self.close = true,
            Intent::Export => self.export_asked = true,
            Intent::ExportAgain => self.export_again = true,
            Intent::Menu => self.menu = true,
            Intent::View(view) => self.view = view,
            Intent::PanelTab(tab) => self.choose_tab(tab),
            // Pressed again, it is chosen already, and nothing changes.
            Intent::ShowBrowse => self.choose_tab(BROWSE),
            Intent::ScaleUp | Intent::ScaleDown => {
                let up = intent == Intent::ScaleUp;
                ctx.set_zoom_factor(actions::step_scale(ctx.zoom_factor(), up));
            }
            Intent::ScaleReset => ctx.set_zoom_factor(1.0),
            Intent::NextRegion | Intent::PreviousRegion => {
                let forward = intent == Intent::NextRegion;
                let focused = ctx.memory(|m| m.focused());
                if let Some(id) = actions::next_region(regions, &focusable(ctx), focused, forward) {
                    ctx.memory_mut(|m| m.request_focus(id));
                }
            }
            Intent::FullScreen => {
                let on = ctx.input(|i| i.viewport().fullscreen).unwrap_or(false);
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!on));
            }
            // The main window, the app's one: a dialog's own is the app's to close. A close
            // the app leaves by, never the start window's own. Restart is Quit by now
            // (`App::run_actions`).
            Intent::CloseWindow | Intent::Quit | Intent::Restart => {
                self.leaving = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            // Read while held, by the sections.
            Intent::Peek => {}
            Intent::Grid(chosen) => self.grid.apply(chosen),
            Intent::Background(background) => {
                self.central_panel.background = Some(background);
                apertrust_ui::set_theme(ctx, self.look());
            }
        }
        ctx.request_repaint();
    }

    /// Shows the side panel's tab `tab`, chosen by the user: the one settings had is
    /// forgotten, even if they come back to the tab the shell opened on.
    fn choose_tab(&mut self, tab: usize) {
        if tab != self.panel_tab {
            self.opening_tab = None;
        }
        self.panel_tab = tab;
    }

    /// The theme in use, with the background chosen behind the photo, if any.
    fn look(&self) -> Theme {
        Theme::builtin(self.theme).with_background(self.central_panel.background)
    }

    /// Takes in what the Settings dialog's rows show of `settings` and shows it in the main
    /// window, `ctx`'s: the theme, the language, the interface's size, the sliders' marker,
    /// the tab names, square thumbnails and whether Export asks before overwriting. The
    /// engine's settings are the session's at its next start, and nothing of the shell's.
    fn follow(&mut self, settings: &Settings, ctx: &egui::Context) {
        if settings.theme != self.theme {
            self.theme = settings.theme;
            apertrust_ui::set_theme(ctx, self.look());
        }
        if settings.locale != self.kept_locale {
            self.kept_locale = settings.locale;
            let locale = settings
                .locale
                .filter(|l| offered(*l, cfg!(debug_assertions)));
            self.locale = locale.unwrap_or(self.system_locale);
            apertrust_ui::set_locale(ctx, self.locale);
        }
        if settings.scale != ctx.zoom_factor() {
            ctx.set_zoom_factor(settings.scale);
        }
        self.marker = settings.marker;
        apertrust_ui::components::set_marker(ctx, self.marker);
        self.tab_names = settings.tab_names;
        self.grid.square_thumbnails = settings.grid.square_thumbnails;
        self.log.keep = settings.log.keep;
        self.log.domains.clone_from(&settings.log.domains);
        self.overwrite =
            alerts::Overwrite::new(settings.ask_before_overwrite, self.overwrite.held());
        ctx.request_repaint();
    }
}

/// Keeps the session's log in memory as `log` says, at once: while `log.keep` is on, the
/// lines of its domains from now on, the banner first as keeping starts; nothing while it
/// is off (`docs/gui.md`, The log window). The same settings again change nothing.
fn keep_log(kept: &apertrust_base::log::Kept, log: &settings::Log) {
    kept.keep(log.keep.then_some(log.domains.as_slice()), || {
        let program = apertrust_base::program!();
        let command = apertrust_base::log::command_line();
        apertrust_base::log::banner(program, &build(&program), &command)
    });
}

/// The disk cache in `folder`, its folder created as the cache's first write (decision 76).
/// One that cannot be created or written is warned of, once, as the app starts, and the
/// app runs with no disk cache: never refused, and never warned of again as it would have
/// been written.
fn open_cache(folder: &Path) -> Option<Arc<Cache>> {
    match Cache::create(folder) {
        Ok(cache) => Some(Arc::new(cache)),
        Err(error) => {
            let folder = folder.display();
            tracing::warn!(target: "settings", "cannot write the cache folder {folder}: {error}; no disk cache");
            None
        }
    }
}

/// The command a module section's change of the edit's values sends, and what the app
/// shows of it until a report includes it: `SetControl`, `ResetControl`, `Keep` or
/// `ResetModule`, or several of them as one `Step { source: You }` (decision 78), each shown
/// as alone; none for any other change.
fn sent(change: Change) -> Option<(Command, Vec<Pending>)> {
    Some(match change {
        Change::Set {
            module,
            control,
            value,
            done,
        } => (
            Command::SetControl {
                module: module.to_owned(),
                control: control.to_owned(),
                value: value.clone(),
                done,
            },
            vec![Pending::Control {
                module,
                control,
                value,
            }],
        ),
        Change::Reset { module, control } => (
            Command::ResetControl {
                module: module.to_owned(),
                control: control.to_owned(),
            },
            vec![Pending::Reset { module, control }],
        ),
        Change::Keep {
            module,
            control,
            kept,
        } => (
            Command::Keep {
                module: module.to_owned(),
                control: control.to_owned(),
                kept,
            },
            vec![Pending::Keep {
                module,
                control,
                kept,
            }],
        ),
        Change::ResetModule { module } => (
            Command::ResetModule {
                module: module.to_owned(),
            },
            vec![Pending::ResetModule { module }],
        ),
        Change::Step(changes) => {
            let (commands, shown): (Vec<_>, Vec<_>) = changes.into_iter().filter_map(sent).unzip();
            let step = Command::Step {
                source: StepSource::You,
                note: None,
                commands,
            };
            (step, shown.concat())
        }
        _ => return None,
    })
}

/// Whether `command` completes a change, which saves the edit, as the engine's history
/// says: a gesture's change is saved with the one that ends it and `Set` with the next, and
/// a `Step` completes one when it holds any command but a gesture's change.
fn completes(command: &Command) -> bool {
    let goes_on = |command: &Command| matches!(command, Command::SetControl { done: false, .. });
    match command {
        Command::SetControl { done: false, .. } | Command::Set { .. } => false,
        Command::Step { commands, .. } => !commands.iter().all(goes_on),
        _ => true,
    }
}

/// A setting `now` where it differs from the `last` the session was given, which then
/// becomes it; none where it is the same.
fn changed<T: PartialEq + Copy>(last: &mut T, now: T) -> Option<T> {
    (now != *last).then(|| {
        *last = now;
        now
    })
}

/// What the session is sent where Settings changed the disk cache's limit from `last` to
/// `now`: the limit, then the sizes, so Settings › Storage's total and limit agree; nothing
/// where it is the same.
fn cache_commands(last: &mut Limit, now: Limit) -> Vec<Command> {
    let Some(limit) = changed(last, now) else {
        return Vec::new();
    };
    vec![
        Command::Cache(CacheCommand::Limit(limit)),
        Command::Cache(CacheCommand::Sizes),
    ]
}

/// What the user changed in Settings, kept by `keeper` with the rest of settings and shown
/// by the shell in the main window, `ctx`'s, in the order changed.
fn apply_settings(
    changes: &[settings_dialog::Change],
    shell: &mut Shell,
    keeper: &mut Keeper,
    ctx: &egui::Context,
) {
    for change in changes {
        let (id, value) = change.parts();
        let settings = keeper.apply(&shell.to_settings(ctx), id, value);
        shell.follow(&settings, ctx);
    }
}

impl App {
    /// The app as `settings` left it, before its first frame, opening what the command line
    /// named, `open`, or the files Open With gave it, `in_folder`, once they have been
    /// looked at; `keeper` writes them back. Its window was made at `built`'s placement, as
    /// the start window where it says so, and is given the other where the start turns out
    /// otherwise, before it first shows.
    /// Its session plans within `engine`'s budget, `engine.memory_budget`, and keeps
    /// `cache`, the disk cache, if the app could open one; Settings says whether the
    /// adapter in use is the one `engine` chose. Its dialogs are the host's. It keeps the
    /// session's log in `kept` while settings say so. `launch` says whether it reopens what
    /// was open last, how it restarts, and what a start that did not finish has it say.
    #[allow(clippy::too_many_arguments, reason = "what the app is made of, once")]
    fn new(
        cc: &eframe::CreationContext<'_>,
        open: Option<Start>,
        in_folder: Vec<PathBuf>,
        built: (settings::Window, bool),
        keeper: Keeper,
        settings: Settings,
        gate: SubmitGate,
        engine: apertrust_engine::settings::Options,
        cache: Option<Arc<Cache>>,
        recents: Recents,
        storage_folders: storage::Folders,
        dialogs: dialog_host::Host,
        kept: apertrust_base::log::Kept,
        launch: Launch,
    ) -> Self {
        keep_log(&kept, &settings.log);
        let ask = &apertrust_export::settings::registry::EXPORT_ASK_BEFORE_OVERWRITE;
        // Whether each recent entry is there, asked as the window shows, off this thread,
        // so a slow share never holds a frame; again as it gains the focus and as Browse
        // comes into sight.
        let mut recents = recents;
        recents.check();
        let shell = Shell {
            overwrite: alerts::Overwrite::new(
                settings.ask_before_overwrite,
                keeper.overridden(ask),
            ),
            recents,
            ..Shell::new(&settings, system_locale(), cc.egui_ctx.os())
        };
        prepare(&cc.egui_ctx, &shell, settings.scale);
        let (placement, built_start) = built;
        if let Some(window) = cc.winit_window() {
            frame_shadow(window);
            place(window, &placement, built_start);
        }
        // Only the top panel's empty areas move the window, never its buttons.
        if OWN_TITLE_BAR && let Err(e) = apertrust_display::content_never_moves_window(cc) {
            tracing::warn!(target: "gpu", "window moves not kept to the top panel: {e}");
        }
        // Now the window exists, in the language `prepare` set.
        let menus = menus::NativeMenus::install(&cc.egui_ctx);
        popups::install(&cc.egui_ctx);
        let state = cc
            .wgpu_render_state
            .as_ref()
            .expect("the app runs on the wgpu renderer");
        gpu::log_adapters(&state.available_adapters, &state.adapter);
        // An error no scope of the engine's takes, as the window's own drawing could make,
        // is logged rather than ending the app, as wgpu's default handler would: once,
        // since a draw that fails fails again every frame, and up to a few kinds.
        const LOGGED_KINDS: usize = 16;
        let logged = std::sync::Mutex::new(std::collections::HashSet::new());
        state.device.on_uncaptured_error(Arc::new(move |error| {
            let mut logged = logged
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if logged.len() < LOGGED_KINDS && logged.insert(error.to_string()) {
                // Whole, on one line: its cause comes last.
                let error = apertrust_base::log::one_line(&error.to_string());
                tracing::error!(target: "gpu", "window draw refused: {error}");
                if logged.len() == LOGGED_KINDS {
                    tracing::error!(target: "gpu", "window draw errors: no more logged");
                }
            }
        }));
        viewport::install(state);
        dialogs.attach(state.clone(), cc.winit_window().cloned());
        let dialogs = dialog_host::AppDialogs::new(dialogs);
        let gpu = GpuContext {
            adapter: state.adapter.clone(),
            device: state.device.clone(),
            queue: state.queue.clone(),
            gate,
        };
        // What Settings' In use line says: the adapter the window opened, the engine's
        // (decision 82), whether the setting chose it, the graphics APIs this build has,
        // and how its list of graphics cards is made; and the values this run started
        // with, for the waiting line.
        let info = state.adapter.get_info();
        let hardware = hardware::Running {
            in_use: Some(hardware::InUse::of(&info, &engine.gpu.adapter)),
            built: egui_wgpu::wgpu::Instance::enabled_backend_features(),
            lister: Some(hardware::Lister::helper()),
            started: hardware::Started::of(settings::shown(), |s| keeper.value(s)),
            restart_failed: None,
        };
        let Launch {
            reopen_last,
            restart,
            run,
            marker,
        } = launch;
        // What a start that did not finish has the window say, by what opened: where the
        // CPU was asked for and the API has none, wgpu's choice opened again.
        let notice = run.notice(info.device_type == egui_wgpu::wgpu::DeviceType::Cpu);
        let places = export::places();
        let ctx = cc.egui_ctx.clone();
        let modules = apertrust_modules::MODULES;
        let cache_limit = keeper.cache_limit();
        let cache = cache.map(|cache| CacheOptions {
            cache,
            limit: cache_limit,
        });
        let session = Session::start(
            gpu,
            apertrust_operations::OPERATIONS,
            modules,
            apertrust_base::program!().version,
            engine.budget,
            cache,
            move || ctx.request_repaint(),
        );
        // The setting of the thumbnails of photos with no edit, before any is asked for.
        let raw_thumbnails = keeper.raw_thumbnails();
        session.send(Command::Library(LibraryCommand::RawThumbnails(
            raw_thumbnails,
        )));
        let mut app = Self {
            session,
            mirror: Mirror::new(modules, jobs::Mirror::new(places.home.clone())),
            bounds: Bounds::default(),
            window_size: egui::Vec2::ZERO,
            panel_width: 0.0,
            grid_lines: (GridPosition::Bottom, 0),
            shell,
            panels: Panels::default(),
            tag_failed: false,
            buttons_failed: false,
            handed_off: false,
            resize: None,
            keeper,
            menus,
            asking: None,
            drops: dropped::Drops::default(),
            opened: dropped::Drops::default(),
            pictures: paths::pictures(),
            dialogs,
            jobs_view: Vec::new(),
            places,
            again: export_again::Again::default(),
            quit_held: false,
            alerts: alerts::Alerts::default(),
            quit: alerts::Guard::default(),
            quit_again: false,
            picture: None,
            fade: None,
            said: pickers::Said::default(),
            thumbnails: thumbnails::Textures::default(),
            thumbnails_asked: thumbnails::Asking::default(),
            kept,
            saving: None,
            started: apertrust_engine::names::now(),
            downloads: paths::downloads(),
            cache_limit,
            raw_thumbnails,
            focused: true,
            storage_folders,
            storage_shown: storage::View::default(),
            hardware,
            restart,
            notice,
            notice_held: false,
            marker,
            reopening: None,
            remembered: Remembered::default(),
            start: start::Window::new(false, settings.scale),
        };
        // Finder's files at the launch, which macOS hands over as it finishes launching,
        // before the window, win over what was open last, as `--in-folder`'s do; any that
        // come later open over it.
        let mut in_folder = in_folder;
        in_folder.extend(apertrust_display::take_opened_files().unwrap_or_default());
        let session = &app.session;
        let send = &mut |command| session.send(command);
        if let Some(open) = open {
            started(open, &mut app.mirror, send);
        } else if in_folder.is_empty() {
            let last = app.shell.recents.last();
            app.reopening = reopen(last, reopen_last, &mut app.mirror, send);
        }
        if !in_folder.is_empty() {
            app.opened.look(in_folder, &cc.egui_ctx);
        }
        // With nothing to open, the start window in place of the main window; the window
        // given the other's size and place where the start turned out otherwise.
        let looking = app.opened.looking();
        let starts = shows_start(opened(&app.mirror), looking, false, false);
        app.start.shown = starts;
        if starts != built_start
            && let Some(window) = cc.winit_window()
        {
            let placement = if starts {
                start_placement(&settings.window, settings.scale)
            } else {
                settings.window
            };
            reshape(window, &placement, starts);
        }
        // Finder's files, at the launch and later, and a click on the Dock icon wake the app
        // as they come.
        let ctx = cc.egui_ctx.clone();
        apertrust_display::on_opened_files(move || ctx.request_repaint());
        let ctx = cc.egui_ctx.clone();
        apertrust_display::on_reopen(move || ctx.request_repaint());
        app
    }

    /// Gives the session the disk cache's limit where Settings changed it, applied at once:
    /// the session evicts where it is lowered or turned on (decision 76).
    fn follow_cache_limit(&mut self) {
        for command in cache_commands(&mut self.cache_limit, self.keeper.cache_limit()) {
            self.session.send(command);
        }
    }

    /// Gives the session the thumbnails' setting where Settings changed it, applied at
    /// once to the thumbnails asked for from then on and the photo open.
    fn follow_raw_thumbnails(&mut self) {
        if let Some(raw) = changed(&mut self.raw_thumbnails, self.keeper.raw_thumbnails()) {
            let command = LibraryCommand::RawThumbnails(raw);
            self.session.send(Command::Library(command));
        }
    }

    /// Does what Settings › Storage asked: the sizes, a Clear, whose button stays disabled
    /// until its job ends, or a folder shown in the file manager.
    fn storage_asked(&mut self, asked: storage::Asked) {
        let session = &self.session;
        storage::act(
            asked,
            &mut |command| session.send(command),
            &mut self.mirror.cache,
        );
    }

    /// Hands Settings › Storage what it shows, where that has changed since last handed.
    fn show_storage(&mut self) {
        let view = self.mirror.cache.view(&self.storage_folders);
        if view != self.storage_shown {
            self.dialogs.storage(view.clone());
            self.storage_shown = view;
        }
    }

    /// Takes what the Export dialog asked for the photo open: a new preview of the
    /// settings it changed, and the export itself when Export was pressed, of the settings
    /// the shell holds, which are the dialog's. The photo's edit is the session's to take
    /// as the command is handled (D12). Export pressed with On conflict replacing files
    /// asks first ([`export::Asked::confirm`]): the export goes on a yes
    /// ([`App::alert_answered`]).
    fn export(&mut self, asked: export::Asked, ctx: &egui::Context) {
        self.send_export(&asked);
        if asked.confirm {
            self.ask_overwrite(ctx);
        }
    }

    /// Asks the overwrite question, answered on a later frame ([`App::alert_answered`]),
    /// unless the setting says not to, when the export goes ahead as a yes would send it
    /// ([`App::overwrite_confirmed`]). Returns whether the question was settled, asked or
    /// skipped; if not, no export starts.
    fn ask_overwrite(&mut self, ctx: &egui::Context) -> bool {
        if !self.shell.overwrite.asks() {
            self.overwrite_confirmed();
            return true;
        }
        let request = alerts::overwrite_request(ctx, self.shell.overwrite.held());
        let purpose = alerts::Purpose::Overwrite;
        let shown = self.alerts.ask(&mut self.dialogs, purpose, &request, ctx);
        let why = match shown {
            dialog_host::Ask::Shown => return true,
            dialog_host::Ask::Busy => "another alert is up",
            dialog_host::Ask::Failed => "the alert could not be shown",
            dialog_host::Ask::NoAlerts => "this platform shows no alerts",
        };
        tracing::warn!(target: "gpu", "export not started: {why}");
        false
    }

    /// File › Export Again: darktable's export with the settings as last kept, for the
    /// photo open, with no dialog. With On conflict Overwrite it asks the overwrite
    /// question first, unless the setting says not to ([`export::asks_before`],
    /// [`App::ask_overwrite`]); any other On conflict exports at once. Not while the app
    /// is quitting: a job started now would be uncounted by the quit alert's yes.
    fn export_again(&mut self, ctx: &egui::Context) {
        let PhotoState::Open(photo) = &self.mirror.photo else {
            return;
        };
        if self.quit.quitting() || self.quit_held {
            tracing::warn!(target: "gpu", "export again dropped: the app is quitting");
            return;
        }
        let command = export_again::export(&photo.path, &self.shell.export, &self.places);
        if !export::asks_before(self.shell.export.on_conflict) {
            self.send_job(command);
            return;
        }
        self.again.hold(command);
        if !self.ask_overwrite(ctx) {
            self.again.declined();
        }
    }

    /// Sends an export job, which the jobs window follows by its events, and quitting
    /// waits for until it is added.
    fn send_job(&mut self, command: Command) {
        let number = self.session.send(command);
        self.mirror.jobs.starting(number);
    }

    /// Sends the session what `asked` holds for the photo open.
    fn send_export(&mut self, asked: &export::Asked) {
        let PhotoState::Open(photo) = &self.mirror.photo else {
            return;
        };
        let commands = export::commands(&photo.path, asked, &self.shell.export, &self.places);
        for command in commands {
            let preview = matches!(command, Command::PreviewExport { .. });
            let exporting = matches!(command, Command::Export { .. });
            let number = self.session.send(command);
            if preview && asked.opened {
                self.mirror.previews_from(number);
            }
            if exporting {
                self.mirror.jobs.starting(number);
            }
        }
    }

    /// Does what the answer to the alert asked for `purpose` says. Overwrite: a yes sends
    /// the export, Export Again's if it asked, else the Export dialog's, which closes
    /// with it; anything else leaves the dialog as it was and drops Export Again. Quit: a yes stops the jobs that can be stopped and goes on
    /// quitting, which waits for the others; an alert that could not be shown after all
    /// leaves; anything else stays, and the next Quit asks again. Save: Try Again writes
    /// the edit again.
    fn alert_answered(&mut self, purpose: alerts::Purpose, answer: dialog_host::Answer) {
        self.shell.overwrite.answered(purpose, answer);
        match alerts::then(purpose, answer, &mut self.quit, &self.mirror.jobs) {
            alerts::Then::Nothing => {
                self.again.declined();
                // The app stays: a restart asked with Quit is asked no more.
                if purpose == alerts::Purpose::Quit {
                    self.restart.cancel();
                }
            }
            alerts::Then::Export => self.overwrite_confirmed(),
            alerts::Then::Stop(jobs) => {
                for job in jobs {
                    self.session.send(Command::CancelJob { job });
                }
                self.quit_again = true;
            }
            alerts::Then::RetrySave => {
                self.send_edit(Command::RetrySave);
            }
            alerts::Then::Leave => self.quit_again = true,
            alerts::Then::Told => {}
        }
    }

    /// The overwrite question is settled in favour of exporting, by a yes or by the setting:
    /// the question of Export Again, if it was waiting for it, goes to the session, and no
    /// dialog is open to close; otherwise the Export dialog's export is sent, and the dialog
    /// closes with it.
    fn overwrite_confirmed(&mut self) {
        match self.again.confirmed() {
            Some(command) => self.send_job(command),
            None => {
                self.send_export(&export::Asked {
                    start: true,
                    ..export::Asked::default()
                });
                self.dialogs.close();
            }
        }
    }

    /// Whether an alert or the open dialog keeps the main window's actions that open
    /// dialogs at rest, as a sheet does.
    fn asking_modal(&self) -> bool {
        self.asking.is_some() || self.alerts.pending().is_some()
    }

    /// Does what the log window asked: Open Diagnostics settings… opens Settings on
    /// General, as Settings… does; Save to file… asks where, offering the session's name
    /// in the folder the last save went to, or Downloads, unless it is asking already.
    fn log_asked(&mut self, asked: log_window::Asked, ctx: &egui::Context) {
        match asked {
            log_window::Asked::Settings => {
                let page = settings::registry::SETTINGS_WINDOW_PAGE.id;
                let general = apertrust_base::settings::Page::General.id();
                let general = settings_dialog::Change::Set(page, general.into());
                apply_settings(&[general], &mut self.shell, &mut self.keeper, ctx);
                self.run_actions([ActionId::SETTINGS], ctx, &[]);
            }
            log_window::Asked::Save(text) => {
                if self.saving.is_some() {
                    return;
                }
                let name = log_window::file_name(&self.started);
                let last = self.shell.log.folder.as_deref();
                let folder = log_window::save_folder(last, self.downloads.as_deref());
                let saving = dialogs::Saving::ask(ctx, text, &name, folder.as_deref());
                self.saving = Some(saving);
            }
        }
    }

    /// Once the save dialog has its answer: writes the log's text to the file chosen, on a
    /// thread of its own, so a slow disk never holds a frame, and keeps its folder for the
    /// next save. A cancel writes nothing.
    fn log_saved(&mut self, ctx: &egui::Context) {
        let Some(answer) = self.saving.as_mut().and_then(|saving| saving.chosen(ctx)) else {
            return;
        };
        self.saving = None;
        let Some((path, text)) = answer else {
            return;
        };
        if let Some(folder) = path.parent() {
            self.shell.log.folder = Some(folder.to_owned());
        }
        std::thread::spawn(move || {
            if let Err(e) = std::fs::write(&path, text) {
                let path = path.display();
                tracing::warn!(target: "control", "log not saved to {path}: {e}");
            }
        });
    }

    /// Does what a button of the jobs window asked ([`jobs::Mirror::act`]). Closing the
    /// window stops nothing.
    fn jobs_action(&mut self, action: ActivityAction, ctx: &egui::Context) {
        let changed = match self.mirror.jobs.act(action) {
            jobs::Do::Nothing => false,
            jobs::Do::Send(command) => {
                self.session.send(command);
                false
            }
            jobs::Do::Start(command) => {
                self.send_job(command);
                true
            }
            jobs::Do::Reveal(what) => {
                jobs::reveal(&what);
                false
            }
            jobs::Do::Changed => true,
        };
        if changed {
            self.jobs_view = self.mirror.jobs.view(&translator(ctx));
            let place = self.shell.jobs_window;
            let view = &self.jobs_view;
            jobs::refresh(
                view,
                Vec::new(),
                &mut self.dialogs,
                jobs::Show::Update,
                place,
                ctx,
            );
        }
    }

    /// Opens the raw at `path` in the session, in place of the photo open, as File › Open…
    /// and a drop do. The session saves the open photo's edit first, if
    /// anything in it is not on disk yet, and reads the new photo's from beside it.
    fn open(&mut self, path: PathBuf) {
        let session = &self.session;
        open_photo(
            path,
            &mut self.remembered,
            &mut self.mirror,
            &mut |command| session.send(command),
        );
    }

    /// Opens what Open… or a drop asked for, or asks the alert that says why nothing opens,
    /// naming the path it refused.
    fn carry_out_opening(&mut self, opening: Opening, ctx: &egui::Context) {
        match opening {
            Ok(Opens::Photo(path)) => self.open(path),
            Ok(Opens::Folder(path)) => self.open_folder(path),
            Err((path, why)) => {
                let request = alerts::refused_request(ctx, &file_name(&path), why);
                let purpose = alerts::Purpose::Refused;
                self.alerts.ask(&mut self.dialogs, purpose, &request, ctx);
            }
        }
    }

    /// Opens the folder at `path` in the library, its photos going into the grid panel, a
    /// way in: it becomes Browse's root and, once open, the first of the recents; Open
    /// Folder…, Open Recent, Browse's recents and Open This Folder, Locate… and a dropped
    /// folder. The photo shown follows it ([`change_folder`]).
    fn open_folder(&mut self, path: PathBuf) {
        self.change_folder(path, FolderChange::WayIn);
    }

    /// Shows the folder at `path`, chosen in Browse's tree, in the grid panel. The photo
    /// shown follows it ([`change_folder`]).
    fn choose_folder(&mut self, path: PathBuf) {
        self.change_folder(path, FolderChange::Choose);
    }

    /// The active folder changed to `path` by `change` ([`change_folder`]).
    fn change_folder(&mut self, path: PathBuf, change: FolderChange) {
        let session = &self.session;
        change_folder(
            path,
            change,
            self.shell.recents.last(),
            &mut self.remembered,
            &mut self.mirror,
            &mut |command| session.send(command),
        );
    }

    /// File › Close ([`close`]): what was open last is forgotten where it is the folder
    /// closed, and no longer reopens; a photo opened alone leaves it. The photos remembered
    /// this run are forgotten too.
    fn close(&mut self) {
        let session = &self.session;
        let closed = self.mirror.folder.root().map(Path::to_owned);
        close(&mut self.remembered, &mut self.mirror, &mut |command| {
            session.send(command)
        });
        forget_last(&mut self.shell.recents, closed.as_deref());
        self.reopening = None;
    }

    /// Takes in what the engine has reported since the last frame through the mirror
    /// (`mirror.rs`), the one place events enter, and does what it asks in reply. Run first
    /// in `logic`, which eframe calls before each frame and alone while the window is
    /// minimised.
    fn drain(&mut self, ctx: &egui::Context) {
        let mut show = jobs::Show::Nothing;
        for event in self.session.events() {
            for reaction in self.mirror.apply(event) {
                match reaction {
                    Reaction::Jobs(asked) => show = show.max(asked),
                    // The Export dialog shows the preview, the newest it holds.
                    Reaction::Preview { through, preview } => {
                        self.dialogs.preview(through, *preview);
                    }
                    Reaction::FolderOpened(path) => self.shell.remember_folder(path),
                    Reaction::FolderFailed { path, reason } => {
                        self.shell.missing.insert(path, reason);
                    }
                    Reaction::Swapped { open, generation } => {
                        // The system's reduce-motion preference, read as each fade begins.
                        let reduce = apertrust_display::reduce_motion();
                        let kept = &mut self.picture;
                        self.fade = fade::Fade::begin(ctx, kept, open, generation, reduce);
                    }
                }
            }
        }
        let session = &self.session;
        reopened(&mut self.reopening, &mut self.mirror, &mut |command| {
            session.send(command)
        });
        self.remembered.remember(&self.mirror);
        self.remembered
            .restore(&mut self.mirror, &mut |command| session.send(command));
        // The spinner and the window show the jobs as they stand, in the language now.
        self.jobs_view = self.mirror.jobs.view(&translator(ctx));
        let place = self.shell.jobs_window;
        // The jobs that opened the window, for it to close itself once they end well.
        let opened = self.mirror.jobs.opened();
        let view = &self.jobs_view;
        jobs::refresh(view, opened, &mut self.dialogs, show, place, ctx);
    }

    /// Sends a module control's new value and shows it at once, kept, until the engine
    /// reports it; `done` on the last change of a gesture, which saves the edit.
    fn set_control(
        &mut self,
        module: &'static str,
        control: &'static str,
        value: ControlValue,
        done: bool,
    ) {
        self.send_change(Change::Set {
            module,
            control,
            value,
            done,
        });
    }

    /// Sends a change of the edit's values ([`sent`]) and shows it at once until a report
    /// includes it.
    fn send_change(&mut self, change: Change) {
        let Some((command, shown)) = sent(change) else {
            return;
        };
        let number = self.send_edit(command);
        for pending in shown {
            self.pend(number, pending);
        }
    }

    /// Shows `pending`, sent as command `number`, until a report includes it
    /// ([`Mirror::pend`]).
    fn pend(&mut self, number: u64, pending: Pending) {
        self.mirror.pend(number, pending);
    }

    /// Sends what a module section asks of the session, shown at once until a report
    /// includes it, or applies what it asks of the section's own state. A photo open for
    /// viewing only takes no change of its edit, whose controls and resets are drawn
    /// disabled, nor a module's picker turned on.
    fn change(&mut self, change: Change, ctx: &egui::Context) {
        let edits = matches!(
            change,
            Change::Set { .. }
                | Change::Reset { .. }
                | Change::Keep { .. }
                | Change::ResetModule { .. }
                | Change::ApplyPreset { .. }
                | Change::Action { .. }
                | Change::Step(_)
                | Change::Picker {
                    action: PickerAction::On { .. },
                    ..
                }
        );
        // Only the open photo's values, once they have arrived, can be changed: not the
        // defaults drawn with no photo or while one opens, nor a photo open for viewing
        // only.
        if edits && !self.mirror.takes_edits() {
            return;
        }
        match change {
            change @ (Change::Set { .. }
            | Change::Reset { .. }
            | Change::Keep { .. }
            | Change::ResetModule { .. }
            | Change::Step(_)) => self.send_change(change),
            // Nothing to show before the session reports it: the option's values are the
            // photo's, which only the session knows.
            Change::ApplyPreset {
                module,
                preset,
                option,
            } => {
                self.send_edit(Command::ApplyPreset {
                    module: module.to_owned(),
                    preset: preset.to_owned(),
                    option: option.to_owned(),
                });
            }
            // darktable's button, run on the values shown: each control it writes, one
            // change, shown at once as a slider's is.
            Change::Action {
                module,
                action,
                values,
            } => {
                let PhotoState::Open(photo) = &self.mirror.photo else {
                    return;
                };
                let writes = modules::action_writes(
                    self.mirror.modules(),
                    module,
                    action,
                    &values,
                    &photo.image,
                    self.view_only(),
                );
                for (control, value, done) in writes {
                    self.set_control(module, control, value, done);
                }
            }
            Change::Toggle { module } => {
                let open = self.shell.section_open(module, modules::OPEN);
                self.shell.set_section_open(module, !open);
            }
            Change::Group { module, group } => self.shell.toggle_group(module, group),
            Change::Tab { .. } => self.panels.apply(&change),
            Change::Picker {
                module,
                picker,
                action,
                keys,
            } => {
                let command = pickers::pressed(PickerRef::Module { module, picker }, action, keys);
                self.send_picker(command, ctx);
            }
        }
    }

    /// Sends a colour picker's command (decision 79) and shows what it asks at once: a
    /// picker turned on, with the focus moved to the photo, or off, and a region moved; the
    /// live region waits for the reading of a command that applies one, and the save
    /// state, for a command that completes a change, for the values its pick reports.
    fn send_picker(&mut self, command: PickerCommand, ctx: &egui::Context) {
        let showing = pickers::Showing::of(&command, self.mirror.modules());
        // A module's pick is a change on the edit, which the session answers with a
        // write, or the edit reported held where it wrote nothing; the Colour picker's
        // writes nothing.
        let picker = match &command {
            PickerCommand::On { picker, .. } => Some(*picker),
            _ => self.mirror.pickers.active(),
        };
        let module = matches!(picker, Some(PickerRef::Module { .. }));
        let completes = module && pickers::completes(&command);
        let number = self.session.send(Command::Picker(command));
        let mirror = &mut self.mirror;
        if completes {
            mirror.saving.sent(number, true, Instant::now());
        }
        showing.sent(
            number,
            &mut mirror.pickers,
            &mut self.said,
            &mirror.values,
            ctx,
        );
    }

    /// Sends a command that changes the edit: the top panel's save state waits for the
    /// session to report a write that includes it, one that completes a change at once.
    fn send_edit(&mut self, command: Command) -> u64 {
        let completes = completes(&command);
        let number = self.session.send(command);
        self.mirror.saving.sent(number, completes, Instant::now());
        number
    }

    /// The save warning pressed: why the edit's last write failed, with Try Again, which
    /// writes it again. Nothing while another alert is up.
    fn ask_why_not_saved(&mut self, ctx: &egui::Context) {
        let (Some(why), PhotoState::Open(photo)) =
            (&self.mirror.saving.failure, &self.mirror.photo)
        else {
            return;
        };
        let request = alerts::save_request(ctx, &file_name(&photo.path), why);
        self.alerts
            .ask(&mut self.dialogs, alerts::Purpose::Save, &request, ctx);
    }

    /// What the actions' availability depends on, for the keys and the menus: the shell's,
    /// with the open dialog and the host's dialog ([`with_dialogs`]).
    fn state(&self, ctx: &egui::Context) -> State {
        let state = with_dialogs(
            self.shell_state(ctx),
            self.asking_modal(),
            self.dialogs.shown(),
        );
        if self.start.shown {
            in_start(state)
        } else {
            state
        }
    }

    /// What the main window's own controls are drawn from: [`App::state`], but as though no
    /// dialog's window had the focus, since a window's controls act on it whichever window
    /// has the focus, as AppKit checks a toolbar's items against their own window.
    fn state_in_window(&self, ctx: &egui::Context) -> State {
        let shown = self.dialogs.shown().map(|shown| dialog_host::Shown {
            focused: false,
            ..shown
        });
        with_dialogs(self.shell_state(ctx), self.asking_modal(), shown)
    }

    /// The shell's part of the actions' availability, with the photo's.
    fn shell_state(&self, ctx: &egui::Context) -> State {
        let open = matches!(self.mirror.photo, PhotoState::Open(_));
        // A photo open for viewing only has nothing Reset could take back, nor one still
        // opening, read-only until it is live (decision 77); nor anything Undo and Redo
        // could, which the session refuses there.
        let takes = self.mirror.takes_edits();
        let resettable = self.mirror.values.edited && takes;
        let state = self.shell.state(ctx, open, resettable);
        let history = self.mirror.history.as_ref().filter(|_| takes);
        State {
            shown: !self.mirror.folder.order().is_empty(),
            opened: opened(&self.mirror),
            undo: history.is_some_and(|h| h.undo.is_some()),
            redo: history.is_some_and(|h| h.redo.is_some()),
            ..state
        }
    }

    /// Whether the photo open is open for viewing only: a newer Apertrust wrote its edit,
    /// which the session never changes (decision 43).
    fn view_only(&self) -> bool {
        matches!(&self.mirror.photo, PhotoState::Open(photo) if photo.view_only.is_some())
    }

    /// Runs `ids`, from the keys, the clicks or the menus, in order, each only if it can
    /// run where it is then (`Shell::run`), sending the session the commands they make;
    /// `regions` are the frame's, for F6.
    fn run_actions(
        &mut self,
        ids: impl IntoIterator<Item = ActionId>,
        ctx: &egui::Context,
        regions: &[egui::Rect],
    ) {
        for id in ids {
            let id = match quit_to_restart(id, &mut self.restart) {
                Ok(id) => id,
                Err(error) => {
                    self.restart_failed(&error, ctx);
                    continue;
                }
            };
            // The open dialog and the overwrite question are answered before Quit runs; the
            // quit alert is Quit's own, and decides below.
            let overwrite = self.alerts.pending() == Some(alerts::Purpose::Overwrite);
            if !hold_for_panel(id, self.asking.is_some() || overwrite, &mut self.quit_held) {
                continue;
            }
            let state = self.state(ctx);
            // Close Window on the start window closes it, as its close button does; on a
            // dialog's own window with the focus, that window.
            let dialog_focused = self.dialogs.shown().is_some_and(|shown| shown.focused);
            if id == ActionId::CLOSE_WINDOW && self.start.shown && !dialog_focused {
                let enabled = actions::action(id).is_some_and(|a| a.enabled(&state));
                self.start.close |= enabled;
                continue;
            }
            // On the Mac's main window, what Close does.
            let id = if id == ActionId::CLOSE_WINDOW && !self.start.shown {
                close_window_runs(ctx.os(), &state)
            } else {
                id
            };
            let running = self.mirror.jobs.running();
            let edit = quit_edit(&self.mirror.saving);
            let mut leave = alerts::Leave::new(&mut self.quit, running, edit);
            let (shell, dialogs, keeper) = (&mut self.shell, &mut self.dialogs, &self.keeper);
            let command = run_action(
                shell,
                dialogs,
                keeper,
                &self.jobs_view,
                (&self.storage_shown, &self.hardware),
                id,
                &state,
                ctx,
                regions,
                &mut || leave.may(),
            );
            if leave.ask {
                // The alert is a sheet on the window, which the start window closed on the
                // Mac has hidden: shown again for it ([`App::reveal`]).
                if std::mem::take(&mut self.start.hidden) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                let (alerts, quit) = (&mut self.alerts, &mut self.quit);
                let dialogs = &mut self.dialogs;
                self.quit_again |= alerts::ask_quit(alerts, dialogs, quit, running, edit, ctx);
            }
            if std::mem::take(&mut self.shell.export_again) {
                self.export_again(ctx);
            }
            if let Some(path) = self.shell.open_recent.take() {
                self.open_folder(path);
            }
            if std::mem::take(&mut self.shell.close) {
                self.close();
            }
            if std::mem::take(&mut self.shell.show_log) {
                let size = self.shell.log.size.map(egui::Vec2::from);
                let window = log_window::Window::new(self.kept.clone(), size);
                self.dialogs.open_log(window, self.shell.log.window, ctx);
            }
            if let Some(command) = command {
                // Everything goes back to its default, what is still pending included.
                if command == Command::Reset {
                    self.mirror.pending.clear();
                }
                self.send_edit(command);
            }
        }
    }

    /// Places the window's buttons in the top panel, `height` points tall, where it is the
    /// title bar, and returns the room they take at its start in egui's points: none in
    /// full screen, where the system hides them, nor where it draws the title bar itself.
    /// Every frame, since the system lays them out again as the window changes. Buttons
    /// that cannot be placed keep the top panel the title bar, with room for them where
    /// they are, so the window can still be moved. Where the app draws the caption
    /// buttons itself ([`CAPTION_BUTTONS`]), at the panel's end, none at its start.
    fn window_buttons(
        &mut self,
        frame: &eframe::Frame,
        ctx: &egui::Context,
        height: f32,
    ) -> Option<f32> {
        let full_screen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
        if CAPTION_BUTTONS {
            return (!full_screen).then_some(0.0);
        }
        // The window system's points are egui's times the interface's scale.
        let zoom = ctx.zoom_factor();
        let mut failed = |why: &dyn std::fmt::Display| {
            if !self.buttons_failed {
                tracing::warn!(target: "gpu", "window buttons not placed: {why}");
                self.buttons_failed = true;
            }
        };
        let placed = apertrust_display::place_window_buttons(frame, height * zoom);
        match &placed {
            Ok(WindowButtons::Unplaced(_)) => failed(&"unexpected views"),
            Err(e) => failed(e),
            Ok(_) => {}
        }
        button_room(placed.ok(), zoom, full_screen, OWN_TITLE_BAR)
    }

    /// The tab bar and the tab's sections, as the mirror's photo has them, which peek
    /// while `peek` holds ([`edit_tab`]). What the sections ask for is sent or applied once
    /// they are drawn. Returns the action of the tab clicked.
    fn side_panel(&mut self, ui: &mut egui::Ui, peek: bool) -> Option<ActionId> {
        let tab_clicked = self.shell.tab_bar(ui);
        ui.spacing_mut().item_spacing.y = 0.0;
        let browse = self.shell.panel_tab == PHOTOS;
        if browse && !self.shell.browse_shown {
            self.shell.recents.check();
        }
        self.shell.browse_shown = browse;
        if browse {
            let actions = self.photos_tab(ui);
            self.carry_out(actions, ui.ctx());
            return tab_clicked;
        }
        let (scopes, changes) = edit_tab(ui, &self.shell, &self.mirror, &self.panels, peek);
        for change in scopes {
            self.shell.scopes_change(change);
        }
        for change in changes {
            self.change(change, ui.ctx());
        }
        tab_clicked
    }

    /// The Photos tab's body: its Browse section, as the mirror's folder and the recent
    /// folders have it, under the tab bar.
    fn photos_tab(&self, ui: &mut egui::Ui) -> Vec<library::Action> {
        let recent = self.shell.recent_folders();
        let folder = &self.mirror.folder.folder;
        let keys = self.shell.open_keys(ui.ctx());
        let browse = library::Browse {
            folder,
            root: self.mirror.folder.root(),
            counts: self.mirror.folder.counts(),
            recent: &recent,
            open: self.shell.browse_open(),
            tree: &self.shell.tree,
            inside: self.mirror.folder.inside(),
            keys: &keys,
            home: self.places.home.as_deref(),
        };
        let mut actions = Vec::new();
        PanelBody::new().show(ui, |ui| {
            actions = library::browse(ui, &browse);
        });
        actions
    }

    /// What Browse and the empty states asked for: the dialogs are asked after the frame, a
    /// recent opened or a folder in the tree chosen through the session at once, Browse's
    /// header, its Folders' row and the tree's folders open or close them, the file manager is started, and a
    /// missing recent is taken out of the recents or located.
    fn carry_out(&mut self, actions: Vec<library::Action>, ctx: &egui::Context) {
        let mut repaint = false;
        for action in actions {
            // An ask comes each frame a folder waits for its listing, and needs no frame.
            repaint |= !matches!(action, library::Action::Ask(_));
            match action {
                library::Action::OpenFolder => self.shell.open_folder = true,
                library::Action::OpenFiles => self.shell.open = true,
                library::Action::Open(path) => self.open_folder(path),
                library::Action::Choose(path) => self.choose_folder(path),
                library::Action::Toggle => self.shell.toggle_browse(false),
                library::Action::ToggleFolders => self.shell.toggle_browse(true),
                library::Action::ToggleTree(path) => {
                    let root = self.mirror.folder.root() == Some(path.as_path());
                    self.shell.toggle_tree(&path, root);
                }
                // Asked while the tree is drawn, sent once an open.
                library::Action::Ask(folder) => {
                    if self.mirror.folder.ask_inside(&folder) {
                        let command = LibraryCommand::Subfolders { folder };
                        self.session.send(Command::Library(command));
                    }
                }
                library::Action::Retry(folder) => self.mirror.folder.forget_inside(&folder),
                library::Action::Reveal(path) => jobs::reveal(&jobs::Reveal::Folder(path)),
                library::Action::ShowInFolder(path) => jobs::reveal(&jobs::Reveal::File(path)),
                library::Action::Remove(path) => self.shell.forget_recent(&path),
                library::Action::Locate(path) => self.shell.locate = Some(path),
                // Open Recent's Clear Menu, run as its item runs, on the list as it stands.
                library::Action::ClearRecents => {
                    self.shell.show_recents(ctx);
                    let state = self.state(ctx);
                    self.shell.run(ActionId::CLEAR_RECENT, &state, ctx, &[]);
                }
            }
        }
        if repaint {
            ctx.request_repaint();
        }
    }

    /// The grid panel: the open folder's photos, in the grid bar's sort, the photo open
    /// the active cell; a click or Enter opens one. Always there, empty with no folder
    /// (`docs/gui.md`, The grid panel), and holding a photo open alone by itself, with no
    /// thumbnail, since nothing of its folder is read. Each cell draws its thumbnail once it has come, and
    /// those in sight without one are asked for (`thumbnails.rs`). Returns the action a
    /// choice in the grid bar's menus, or its Labels, runs.
    fn grid_panel(&mut self, ui: &mut egui::Ui, lines: usize) -> Option<ActionId> {
        self.thumbnails.sync(ui.ctx(), &self.mirror.folder);
        let grid = self.shell.grid;
        self.mirror.folder.sort(grid.sort);
        // A thumbnail filling its square is cut at its longer side, so a 3:2 photo's
        // shorter side must cover the square.
        let side = grid.thumbnails.side() * if grid.square_thumbnails { 1.5 } else { 1.0 };
        let size = thumbnails::size_for(side, ui.pixels_per_point());
        let (clicked, wanted, bar) = {
            let mirror = &self.mirror;
            let textures = &self.thumbnails;
            let listing = match &mirror.folder.folder {
                folder::Folder::Open(listing) => Some(listing),
                _ => None,
            };
            let active = mirror.folder.active_shown();
            let empty = library::Empty::of(&mirror.folder.folder);
            let (says, hint) = (empty.grid_says(ui.ctx()), empty.grid_hint(ui.ctx()));
            let photos = mirror.folder.photos();
            let order = mirror.folder.order();
            let thumb = |i: usize| {
                let photo = &photos[order[i]];
                Thumb {
                    name: &photo.name,
                    edited: matches!(photo.edit, EditState::Edited),
                    status: folder::status(&photo.edit),
                    selected: false,
                    picture: textures.picture(&photo.name, size),
                    // The library keeps no ratings or labels yet (Phase 3): every photo
                    // shows as unrated and unlabelled.
                    rating: Some(CellRating::default()),
                }
            };
            // What a caption's first line reports waits for the ratings, as they do.
            let mut panel = GridPanel::new(order.len(), active, thumb)
                .sort(grid.sort)
                .size(grid.thumbnails)
                .lines(lines)
                .position(grid.position)
                .labels(grid.labels)
                .square(grid.square_thumbnails);
            if let Some(says) = &says {
                panel = panel.empty(says);
            }
            if let Some(hint) = &hint {
                panel = panel.empty_hint(hint);
            }
            let output = panel.show(ui);
            let clicked = match (output.action, listing) {
                (Some(GridPanelAction::Open(i)), Some(listing)) => {
                    // The photo open stays as it is when its own cell is pressed again.
                    (active != Some(i)).then(|| listing.path.join(&photos[order[i]].name))
                }
                _ => None,
            };
            // Only an open folder's are asked for.
            let shown = order
                .get(output.shown)
                .filter(|_| listing.is_some())
                .unwrap_or_default()
                .iter()
                .map(|&i| photos[i].name.as_str());
            let folder = &mirror.folder;
            let has = |photo: &str| folder.thumbnail(photo, size).is_some();
            let wanted = self.thumbnails_asked.ask(folder.latest(), shown, size, has);
            (clicked, wanted, output.bar)
        };
        if !wanted.is_empty() {
            self.session
                .send(Command::Library(LibraryCommand::Thumbnails {
                    photos: wanted,
                    size,
                }));
        }
        if let Some(path) = clicked {
            self.open(path);
        }
        bar.map(|chosen| ActionId(chosen.id()))
    }

    /// The central bar, the photo and its caption. Returns the central bar's rectangle, a
    /// region for F6, and the action a choice in its right-click menu runs.
    /// `panel_resized`: the side panel's width or the grid panel's rows changed this frame.
    fn central_panel(
        &mut self,
        ui: &mut egui::Ui,
        panel_resized: bool,
    ) -> (egui::Rect, Option<ActionId>) {
        let t = theme(ui.ctx());
        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
        // The photo's slot, everything under the central bar, first. The photo and its
        // caption are one block in it, inside the margin (`PhotoFrame`, `docs/gui.md`, The
        // photo frame).
        let top = ui.cursor().min;
        let rest = ui.available_rect_before_wrap();
        let slot = egui::Rect::from_min_max(
            egui::pos2(rest.min.x, rest.min.y + CentralBar::height(ui.ctx())),
            rest.max,
        );
        // A right-click round the photo chooses the background behind it; the photo itself
        // keeps its right-click for a menu of its own, and what is drawn over the slot later
        // keeps its own clicks.
        let id = ui.scope_id().with("background");
        let background = ui.interact(slot, id, egui::Sense::click());
        let margins = self.shell.central_panel;
        let margin = if self.shell.proof_margin {
            Margin::Proof(margins.proof_margin)
        } else {
            Margin::Points(margins.margin)
        };
        let block = PhotoFrame::new(slot, margin);
        // The engine renders the photo to fit the room the block leaves it, in physical
        // pixels, whatever the photo's shape.
        let ppp = ui.pixels_per_point();
        let room = block.bounds();
        let bounds = ((room.x * ppp).floor() as u32, (room.y * ppp).floor() as u32);
        // A drag of the side panel's edge resizes the photo as a window resize does, a frame
        // at a time, so it is held the same way.
        let window = ui.ctx().input(|i| i.viewport_rect().size());
        let resized = std::mem::replace(&mut self.window_size, window) != window || panel_resized;
        let now = Instant::now();
        if let Some((width, height)) = self.bounds.update(bounds, resized, now) {
            self.session.send(Command::Resize { width, height });
        }
        if let Some(wait) = self.bounds.wait(now) {
            ui.ctx().request_repaint_after(wait);
        }

        let photo = match &self.mirror.photo {
            PhotoState::Open(photo) => Some(photo),
            _ => None,
        };
        let layer = tr(ui.ctx(), "layer-image");
        // Until the photo is live, nothing acts on it: no guides, proof margin or zoom
        // (decision 77).
        let live = self.mirror.live();
        let open = photo.is_some() && live;
        // Rendering…, quietly, only once the photo is still not live a second after it was
        // asked for, from a repaint asked for then.
        if let Some(from) = self.mirror.rendering_from()
            && from > now
        {
            ui.ctx().request_repaint_after(from - now);
        }
        let rendering = self
            .mirror
            .rendering(now)
            .then(|| tr(ui.ctx(), "photo-rendering"));
        // The zoom menu's entries with the keys of the actions that do the same.
        let words = translator(ui.ctx());
        let keys: Vec<(CentralBarAction, String)> = ZOOM_MENU
            .iter()
            .filter_map(|(entry, id)| {
                Some((*entry, self.shell.keymap.tip(*id, ui.ctx().os(), &words)?))
            })
            .collect();
        // While a picker is on, which it is and how to use it; what the live region said
        // goes with the picker it was about.
        self.said.follow(self.mirror.pickers.active());
        let mac = ui.ctx().os() == OperatingSystem::Mac;
        let picker_words = (self.mirror.pickers.active())
            .zip(self.mirror.pickers.area())
            .filter(|_| open)
            .and_then(|(picker, area)| {
                pickers::bar_words(&words, self.mirror.modules(), picker, area, mac)
            });
        // The viewport only fits the photo for now: the zoom shows Fit, and its menu
        // offers Fit alone, which it already is.
        let mut bar = CentralBar::new(&layer, Zoom::Fit)
            .toggles(
                Toggle::new(false, false),
                Toggle::new(self.shell.proof_margin, open),
                Toggle::new(false, false),
                Toggle::new(self.shell.guides, open),
            )
            .zoom_enabled(open)
            .status(rendering.as_deref());
        if let Some((name, how)) = &picker_words {
            bar = bar.picker(name, how);
        }
        for (entry, keys) in &keys {
            bar = bar.shortcut(*entry, keys);
        }
        match bar.show(ui) {
            // The area was laid out before the bar, so the margin shows from the next frame.
            Some(CentralBarAction::ProofMargin) => {
                self.shell.proof_margin = !self.shell.proof_margin;
                ui.ctx().request_repaint();
            }
            Some(CentralBarAction::Guides) => self.shell.guides = !self.shell.guides,
            _ => {}
        }
        let central_bar =
            egui::Rect::from_min_max(top, egui::pos2(ui.max_rect().right(), ui.cursor().top()));

        // Until its first frame, the photo's picture where one is in hand, else its name.
        let picture = self
            .mirror
            .picture()
            .filter(|_| self.mirror.frame.is_none());
        if self.mirror.picture().is_none() {
            self.picture = None;
        }
        let opening = |path: &Path| {
            let mut args = apertrust_ui::i18n::Args::new();
            args.set("name", file_name(path));
            translator(ui.ctx()).text_with("photo-opening", Some(&args))
        };
        let message = match &self.mirror.photo {
            // The folder's own words, under the photo's: the empty state below.
            PhotoState::None => None,
            PhotoState::Opening { path, .. } => picture.is_none().then(|| opening(path)),
            PhotoState::Failed { message, .. } => Some(message.clone()),
            PhotoState::Open(photo) => (!live && picture.is_none()).then(|| opening(&photo.path)),
        };
        let values = photo.map(|_| &self.mirror.values);
        let placed = place_photo(&block, bounds, ppp, values, picture.map(|(p, _)| p));
        // A failed save says so until a write succeeds; a failed render until the next
        // frame. The edit of a photo left, which could not be written, is said over
        // whatever follows, until another photo is asked for.
        let failure = self
            .mirror
            .failure
            .as_ref()
            .or(self.mirror.saving.failure.as_ref());
        // A failed render keeps the last frame, if any, and says what failed over it.
        let mut picking = None;
        if message.is_none()
            && let (Some(frame), Some(placed)) = (&self.mirror.frame, &placed)
        {
            self.photo_at(ui, slot, placed, frame);
            // The first picture fading out over the first live frame, until it ends.
            let open = self.mirror.photo.open().unwrap_or_default();
            fade::Fade::paint(&mut self.fade, &ui.painter_at(slot), placed, open);
            // The active picker over the live photo, once the rest is drawn.
            if live {
                picking = Some((placed.photo, [frame.width, frame.height]));
            }
        } else if message.is_none()
            && failure.is_none()
            && let (Some((shown, generation)), Some(placed)) = (picture, &placed)
        {
            let painter = ui.painter_at(slot);
            paint_picture(&painter, placed, shown, generation, &mut self.picture);
        }
        let left = self.mirror.saving.left.as_ref().map(|(edit, why)| {
            let mut args = apertrust_ui::i18n::Args::new();
            // The edit's file is the photo's name with `.apxmp` after it.
            args.set("name", file_name(&edit.with_extension("")));
            args.set("error", why.clone());
            translator(ui.ctx()).text_with("edit-not-saved", Some(&args))
        });
        let said: Vec<&str> = [
            left.as_deref(),
            message.as_ref().or(failure).map(String::as_str),
        ]
        .into_iter()
        .flatten()
        .collect();
        if !said.is_empty() {
            let painter = ui.painter_at(slot);
            let galley = painter.layout(
                said.join("\n\n"),
                egui::FontId::proportional(t.text.body),
                t.colors.text,
                (slot.width() - 2.0 * t.spacing.xl).max(t.spacing.xl),
            );
            let text = egui::Rect::from_center_size(slot.center(), galley.size());
            painter.rect_filled(text.expand(t.spacing.m), t.shape.radius, t.colors.raised);
            painter.galley(text.min, galley, t.colors.text);
        }
        ui.allocate_rect(slot, egui::Sense::hover());

        // With no photo open the folder says what to do: where to start, what is being
        // read, why it could not be, or to choose a photo.
        let mut asked = Vec::new();
        if matches!(self.mirror.photo, PhotoState::None) {
            let keys = self.shell.open_keys(ui.ctx());
            let what = library::Empty::of(&self.mirror.folder.folder);
            asked = library::empty_state(ui, slot, what, &keys);
        }

        let photo_rect = placed.as_ref().map(|placed| placed.photo);
        if let (Some(photo), Some(placed)) = (photo, placed) {
            let words = translator(ui.ctx());
            // The shooting data the session read from the raw as it decoded it.
            let values = exposure::Exposure::of(photo).parts(&words);
            // What the tooltip and screen readers say besides the caption's line.
            let mut details = format!(
                "{} · {} × {}",
                photo.camera,
                words.format_number(f64::from(self.mirror.values.extent[0]), 0, false),
                words.format_number(f64::from(self.mirror.values.extent[1]), 0, false),
            );
            if let Some(view_only) = &photo.view_only {
                details.push('\n');
                details.push_str(&view_only_notice(&words, view_only));
            }
            // The active photo's, with a single photo shown: no ring, its name SemiBold.
            // The centre (reject, rating and labels) waits for the library's ratings.
            PhotoCaption::new(&file_name(&photo.path))
                .view_only(photo.view_only.is_some())
                .exposure(&values)
                .details(&details)
                .show(ui, placed.caption);
        }
        if let Some((photo, pixels)) = picking {
            self.picker_on_photo(ui, photo, pixels);
        }
        self.carry_out(asked, ui.ctx());
        (
            central_bar,
            self.background_menu(ui.ctx(), &background, photo_rect),
        )
    }

    /// The active colour picker over the photo drawn at `photo` from a frame of `pixels`
    /// (`docs/pickers.md`, On the photo): its marks, what the pointer and the keys send,
    /// and the live region reading a pick out once it has rested.
    fn picker_on_photo(&mut self, ui: &mut egui::Ui, photo: egui::Rect, pixels: [u32; 2]) {
        let mirror = &self.mirror.pickers;
        let (Some(picker), Some(region)) = (mirror.active(), mirror.region()) else {
            return;
        };
        let words = translator(ui.ctx());
        let name = match picker {
            PickerRef::Module { module, picker } => (self.mirror.modules().iter())
                .find(|m| m.definition().id == module)
                .map(|m| pickers::name(&words, m.definition(), picker))
                .unwrap_or_default(),
            PickerRef::ColourPicker => String::new(),
        };
        let on = pickers::OnPhoto {
            picker,
            photo,
            pixels,
            region,
            switched: mirror.switched(region),
            colour: pickers::sampled(mirror.reading()),
            name: &name,
        };
        for command in pickers::on_photo(ui, &on) {
            self.send_picker(command, ui.ctx());
        }
        let wait = self.said.update(
            Instant::now(),
            self.mirror.pickers.reading(),
            &self.mirror.values,
            self.mirror.modules(),
            &words,
        );
        if let Some(wait) = wait {
            ui.ctx().request_repaint_after(wait);
        }
        pickers::live_region(ui, photo.left_bottom(), self.said.text());
    }

    /// The surround's right-click menu, View › Background's items, hung from the pointer
    /// where `slot` was right-clicked outside `photo`, the photo's rectangle while one is
    /// open (`docs/gui.md`, The background). Returns the action of the item chosen.
    fn background_menu(
        &self,
        ctx: &egui::Context,
        slot: &egui::Response,
        photo: Option<egui::Rect>,
    ) -> Option<ActionId> {
        surround_menu(ctx, slot, photo, |at| {
            let state = self.state(ctx);
            menus::context(ctx, &self.shell.keymap, &state, &menus::BACKGROUND, at)
        })
    }

    /// Whether the main window is the start window this frame ([`shows_start`]): it becomes
    /// it as nothing is open, the first recent chosen and the recents checked again, and the
    /// main window again as something opens, shown if it was hidden. The window takes the
    /// start window's size and place, centred where the main window was, once it is neither
    /// full screen nor maximised, and again as the interface's scale changes; and the main
    /// window's back, whose size it does not keep until it has it.
    fn follow_start(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        let wanted = self.shows_start(ctx);
        let window = frame.winit_window();
        if wanted != self.start.shown {
            self.start.shown = wanted;
            if wanted {
                self.start.list = start::List::shown();
                self.start.reshape = true;
                self.shell.recents.check();
            } else {
                self.reveal(ctx);
                if let Some(window) = window {
                    let placement = placed_on_monitor(window, self.shell.window);
                    reshape(window, &placement, false);
                }
                self.start.settling = Some(start::SIZE * self.start.zoom);
            }
        }
        let zoom = ctx.zoom_factor();
        let viewport = ctx.input(|i| i.viewport().clone());
        let full_screen = viewport.fullscreen.unwrap_or(false);
        let maximised = MAXIMISES && viewport.maximized.unwrap_or(false);
        let changed = self.start.zoom != zoom;
        match start_shape(wanted, self.start.reshape, changed, full_screen, maximised) {
            StartShape::Keep => {}
            StartShape::LeaveFullScreen => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                ctx.request_repaint();
            }
            StartShape::Unmaximise => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                ctx.request_repaint();
            }
            StartShape::Resize { centred } => {
                let placement = start_placement(&self.shell.window, zoom);
                let placement = settings::Window {
                    position: placement.position.filter(|_| centred),
                    ..placement
                };
                if let Some(window) = window {
                    reshape(window, &placement, true);
                }
                self.start.reshape = false;
                self.start.zoom = zoom;
            }
        }
    }

    /// Whether the main window is to be the start window now ([`shows_start`]).
    fn shows_start(&self, ctx: &egui::Context) -> bool {
        let looking = self.opened.looking() || self.drops.looking();
        let modal = self.state(ctx).modal;
        shows_start(opened(&self.mirror), looking, modal, self.start.shown)
    }

    /// Shows the main window again where the start window was closed on the Mac, with the
    /// focus.
    fn reveal(&mut self, ctx: &egui::Context) {
        if std::mem::take(&mut self.start.hidden) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
    }

    /// The start window's close, asked by its close button, Escape or Close Window: on the
    /// Mac the window is hidden and the app runs on, its menu bar there; elsewhere the app
    /// leaves, as Quit does.
    fn close_start(&mut self, ctx: &egui::Context) {
        if !std::mem::take(&mut self.start.close) || !self.start.shown {
            return;
        }
        match start_closes(ctx.os()) {
            StartCloses::Hide => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                self.start.hidden = true;
            }
            StartCloses::Quit => self.run_actions([ActionId::QUIT], ctx, &[]),
        }
    }

    /// One frame of the main window as the start window (`start.rs`): its window buttons in
    /// its strip on the Mac, the app's keys, the recents and the buttons, what they ask run
    /// as Browse's would, and the rest of the frame as the main window's.
    fn start_frame(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let room = self.window_buttons(frame, &ctx, start::STRIP);
        let state = self.state(&ctx);
        let keys = actions::read(&ctx, &self.shell.keymap, &state);
        let mut run = keys.pressed;
        let recents = self.shell.recent_folders();
        let shown = start::Shown {
            home: self.places.home.as_deref(),
            // Where the app draws the window's frame, but in full screen.
            close: CAPTION_BUTTONS && room.is_some(),
            keys: !state.modal,
            version: apertrust_base::program!().version,
        };
        let asked = start::show(ui, &recents, &mut self.start.list, shown);
        let mut gesture = None;
        let mut library = Vec::new();
        for asked in asked {
            match asked {
                start::Asked::Close => {
                    self.start.close = true;
                    ctx.request_repaint();
                }
                start::Asked::Move => gesture = Some(TitleBar::Move),
                start::Asked::Menu => gesture = Some(TitleBar::Menu),
                asked => library.extend(asked.library()),
            }
        }
        self.carry_out(library, &ctx);
        self.shell.about.show(&ctx);
        run.extend(self.shell.popups(&ctx));
        self.run_actions(run, &ctx, &[]);
        self.finish_frame(&ctx, frame, ui.max_rect(), gesture);
    }

    /// The end of each frame, the main window's or the start window's: the ☰'s menu, the
    /// open dialog and what it chose, drops and what the system opened, over `target`, the
    /// title bar's `gesture`, the bands that resize the window, the menus, the Mac's popup
    /// menus and the settings kept. Where what ran changed whether the start window shows,
    /// the frame is drawn again before it is shown.
    fn finish_frame(
        &mut self,
        ctx: &egui::Context,
        frame: &mut eframe::Frame,
        target: egui::Rect,
        gesture: Option<TitleBar>,
    ) {
        // What a start that did not finish says, over the window's top (decision 82).
        self.show_notice(ctx);
        let state = self.state(ctx);
        self.shell.open_menu(ctx, &state);
        // Open…'s and Open Folder…'s dialog, over the window; what is chosen opens a later
        // frame.
        let asked = if std::mem::take(&mut self.shell.open) {
            Some(dialogs::Pick::Files)
        } else if std::mem::take(&mut self.shell.open_folder) {
            Some(dialogs::Pick::Folder)
        } else {
            self.shell.locate.take().map(dialogs::Pick::Locate)
        };
        let (photo, pictures) = (&self.mirror.photo, self.pictures.as_deref());
        let shown = self.mirror.folder.shown();
        let mut dialog = dialogs::Native {
            window: frame,
            ctx,
            asking: &mut self.asking,
        };
        let start = dialogs::locate_start(asked.as_ref());
        let chosen = dialogs::open(&mut dialog, asked, || match start {
            Some(parent) => parent,
            None => dialogs::folder(shown, photo.path(), pictures),
        });
        // Quit, held while the dialog asked, runs once it has answered, opening nothing.
        let overwrite = self.alerts.pending() == Some(alerts::Purpose::Overwrite);
        if self.quit_held && self.asking.is_none() && !overwrite {
            self.quit_held = false;
            self.run_actions([ActionId::QUIT], ctx, &[]);
        } else if let Some(opening) = chosen.and_then(|c| answered(&mut self.shell, c)) {
            self.carry_out_opening(opening, ctx);
        }
        // What was dropped onto the window opens as Open… or Open Folder… would, once it has
        // been looked at, a later frame; nothing while a modal dialog is up, as one may have
        // opened meanwhile. An earlier drop's answer is taken before this frame's drop is,
        // so a drop never opens the frame it came in, however soon its thread answers.
        let modal = self.state(ctx).modal;
        if let Some(dropped) = self.drops.looked().filter(|_| !modal) {
            self.carry_out_opening(dropped_opens(dropped), ctx);
        }
        self.drops.take(ctx, modal);
        // What the system asked the app to open, Open With's, taken in `logic`, opens as a
        // drop of it would, its folder with the photo active, once looked at; it waits for a
        // modal dialog to go rather than being set aside, since nothing on the screen says it
        // came. A drop's answer and Open With's in one frame both open, the later winning,
        // and the second's refusal, if any, finds the alert busy and says nothing.
        if !modal && let Some(opened) = self.opened.looked() {
            self.carry_out_opening(dropped_opens(opened), ctx);
        }
        if dropped::hovering(ctx, modal) {
            dropped::show_target(ctx, target);
        }
        if let Some(gesture) = gesture {
            let mut system = System {
                frame,
                handed_off: false,
            };
            run_title_bar(gesture, ctx, &mut system);
            // The compositor or window manager moving the window keeps the release too.
            let moved = CAPTION_BUTTONS && gesture == TitleBar::Move;
            self.handed_off |= system.handed_off || moved;
        }
        // The start window is never resized.
        if CAPTION_BUTTONS && !self.start.shown {
            resize_hover(ctx, RESIZE_BAND);
        }
        // The menus follow what ran: the keys, what can run now, the focus, the language;
        // Open Recent the recents as they are, kept for its choices, which `logic` runs
        // after the next drain.
        let recents = self.menus.is_some().then(|| self.shell.show_recents(ctx));
        let state = self.state(ctx);
        if let (Some(menus), Some(recents)) = (&mut self.menus, recents) {
            menus.update(ctx, &self.shell.keymap, &state, &recents, &self.shell.steps);
        }
        // The Mac's popup menus the frame asked for, shown from the run loop.
        popups::show(ctx, frame);
        let size = ctx.input(|i| i.viewport_rect().size()) * ctx.zoom_factor();
        let tracks = self.start.keeps_placement(size);
        keep(&mut self.shell, &mut self.keeper, ctx, tracks);
        // Something opened from the start window, or all closed: the other is drawn at once,
        // never a frame of the main window with nothing open.
        if self.shows_start(ctx) != self.start.shown {
            ctx.request_discard("the start window shows or goes");
        }
    }

    /// Restart could not find the app to start: nothing quits, the error is logged and
    /// Settings' waiting line says it, in the warning colour (decision 82).
    fn restart_failed(&mut self, error: &std::io::Error, ctx: &egui::Context) {
        tracing::error!(target: "control", "not restarted: {error}");
        let mut args = apertrust_ui::i18n::Args::new();
        args.set("error", error.to_string());
        let said = translator(ctx).text_with("settings-restart-failed", Some(&args));
        self.hardware.restart_failed = Some(said);
        self.dialogs.hardware(self.hardware.clone(), Vec::new());
    }

    /// The notice of a start that did not finish, under the top panel or the start window's
    /// strip, until one of its buttons is pressed: Keep sets the graphics settings to what
    /// runs, so nothing waits for a restart, but where `--conf` sets either, which then
    /// cannot change, it changes nothing and the notice stays, saying so; Try again
    /// restarts with them, `--safe-mode` left out (`recovery`).
    fn show_notice(&mut self, ctx: &egui::Context) {
        let Some(notice) = self.notice.clone() else {
            return;
        };
        let top = if self.start.shown {
            start::STRIP
        } else {
            theme(ctx).sizes.top_bar
        };
        match notice.show(ctx, top, self.notice_held) {
            Some(recovery::Pressed::Keep) if recovery::held(&self.keeper) => {
                self.notice_held = true;
            }
            Some(recovery::Pressed::Keep) => {
                apply_settings(&notice.kept(), &mut self.shell, &mut self.keeper, ctx);
                for setting in recovery::graphics() {
                    let kept = self.keeper.value(setting);
                    self.hardware.started.keep(setting.id(), kept);
                }
                // Settings open beside it, as the Mac's modeless window, takes them too.
                self.dialogs.hardware(self.hardware.clone(), notice.kept());
                self.notice = None;
            }
            Some(recovery::Pressed::Retry) => match self.restart.ask(false) {
                Ok(()) => self.run_actions([ActionId::RESTART], ctx, &[]),
                Err(error) => self.restart_failed(&error, ctx),
            },
            Some(recovery::Pressed::Close) => self.notice = None,
            None => {}
        }
    }

    /// Draws `frame` where the block `placed` puts the photo, with Proof margin's white
    /// border under it and the guides over it. A frame rendered for another size, as while
    /// the window resizes, is shown at the size the next will have: `placed`'s.
    fn photo_at(&self, ui: &egui::Ui, slot: egui::Rect, placed: &Placed, frame: &Frame) {
        let ppp = ui.pixels_per_point();
        let rect = placed.photo;
        let snap = |v: f32| (v * ppp).round() / ppp;
        // Cut to the slot, should a frame still be larger than it.
        let painter = ui.painter_at(slot);
        viewport::paint(&painter, rect, frame);
        if self.shell.guides {
            // Thirds, one physical pixel wide.
            let stroke = egui::Stroke::new(1.0 / ppp, theme(ui.ctx()).colors.guides);
            for k in [1.0, 2.0] {
                let x = snap(rect.left() + rect.width() * k / 3.0);
                let y = snap(rect.top() + rect.height() * k / 3.0);
                painter.vline(x, rect.y_range(), stroke);
                painter.hline(rect.x_range(), y, stroke);
            }
        }
    }
}

/// The largest size with `size`'s shape that fits `bounds`, larger than `size` too, each
/// side rounded and at least a pixel, as the engine fits the viewport (`pipeline::fit`) but
/// for its cap: an opening photo's picture takes the place its live frame will.
fn fit_scaled((width, height): (u32, u32), (room_width, room_height): (u32, u32)) -> (u32, u32) {
    if width == 0 || height == 0 {
        return (0, 0);
    }
    let scale =
        (f64::from(room_width) / f64::from(width)).min(f64::from(room_height) / f64::from(height));
    let side = |n: u32| ((f64::from(n) * scale).round() as u32).max(1);
    (side(width), side(height))
}

/// Whether `size` has `place`'s shape but for each side's rounding to whole pixels, as a
/// picture smaller or larger than the photo has the shape of the place fitted to the photo:
/// sides within half a pixel of the four make one ratio, the cross terms' extra half pixel
/// included.
fn same_shape((width, height): (u32, u32), (place_width, place_height): (u32, u32)) -> bool {
    let [width, height, place_width, place_height] =
        [width, height, place_width, place_height].map(u64::from);
    let apart = (width * place_height).abs_diff(height * place_width);
    2 * apart <= width + height + place_width + place_height + 1
}

/// Where the photo goes in `block`, for a room of `bounds` physical pixels at `ppp` (the
/// photo frame's layout, which its caption follows): once the photo's `values` have come,
/// at the size the engine renders the edit at, so a frame rendered for it is shown pixel
/// for pixel and one rendered for another size, as while the window resizes, scaled to the
/// size its successor will have; before them, an opening photo's `picture` in its own
/// shape, scaled to fit `bounds` as the live frame will be (decision 77). `None` with
/// neither.
fn place_photo(
    block: &PhotoFrame,
    bounds: (u32, u32),
    ppp: f32,
    values: Option<&Values>,
    picture: Option<&Picture>,
) -> Option<Placed> {
    let (width, height) = match (values, picture) {
        _ if bounds.0 == 0 || bounds.1 == 0 => (0, 0),
        (Some(values), _) => values.viewport_size(bounds),
        (None, Some(picture)) => fit_scaled((picture.image.width, picture.image.height), bounds),
        (None, None) => return None,
    };
    Some(block.place(egui::vec2(width as f32, height as f32) / ppp, ppp))
}

/// Draws an opening photo's `picture` of `generation` unmarked, scaled to the place the
/// photo takes, `placed`: from the photo's values on, the place its live frame covers, so
/// the live frame swaps in with nothing moving (decision 77); the whole place where it has
/// the picture's shape, else letterboxed in it. Its texture, made once for each picture and
/// kept in `kept`, filters linearly. Returns where it was drawn.
fn paint_picture(
    painter: &egui::Painter,
    placed: &Placed,
    picture: &Picture,
    generation: u64,
    kept: &mut Option<(u64, egui::TextureHandle)>,
) -> egui::Rect {
    let image = (picture.image.width, picture.image.height);
    let rect = picture_rect(placed, image, painter.pixels_per_point());
    let texture = picture_texture(painter.ctx(), kept, picture, generation);
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    painter.image(texture.id(), rect, uv, egui::Color32::WHITE);
    rect
}

/// Where a picture of `image`'s size is drawn in the place the photo takes, `placed`, at
/// `ppp`: the whole place where it has the picture's shape, else letterboxed in it.
fn picture_rect(placed: &Placed, image: (u32, u32), ppp: f32) -> egui::Rect {
    let room = placed.photo.size() * ppp;
    let room = (room.x.round() as u32, room.y.round() as u32);
    if same_shape(image, room) {
        return placed.photo;
    }
    let (width, height) = fit_scaled(image, room);
    let size = egui::vec2(width as f32, height as f32) / ppp;
    let corner = egui::Rect::from_center_size(placed.photo.center(), size).min;
    egui::Rect::from_min_size(egui::emath::GuiRounding::round_to_pixels(corner, ppp), size)
}

/// The texture of the mirror's picture of a photo opening, `picture` of `generation`, made
/// once for each and kept in `kept` (decision 77).
fn picture_texture(
    ctx: &egui::Context,
    kept: &mut Option<(u64, egui::TextureHandle)>,
    picture: &Picture,
    generation: u64,
) -> egui::TextureHandle {
    if let Some((made, texture)) = kept
        && *made == generation
    {
        return texture.clone();
    }
    let image = &picture.image;
    let size = [image.width as usize, image.height as usize];
    let pixels = egui::ColorImage::from_rgba_unmultiplied(size, &image.pixels);
    let texture = ctx.load_texture("opening picture", pixels, egui::TextureOptions::LINEAR);
    *kept = Some((generation, texture.clone()));
    texture
}

/// Opens the menu `request` gives at the pointer when `slot` is right-clicked outside
/// `photo`, and returns the action of the item chosen in it. The menu is drawn here while
/// it is open, before its answer is read: a drawn menu (Windows, Linux) answers as it is
/// drawn, and its answer lasts only that frame, so a read before the frame's later
/// `popup::show` would miss it. The Mac's native menu answers between frames.
fn surround_menu(
    ctx: &egui::Context,
    slot: &egui::Response,
    photo: Option<egui::Rect>,
    request: impl FnOnce(egui::Pos2) -> popup::Request,
) -> Option<ActionId> {
    let id = slot.id.with("menu");
    if slot.secondary_clicked()
        && let Some(at) = slot.interact_pointer_pos()
        && !photo.is_some_and(|photo| photo.contains(at))
    {
        popup::open(ctx, id, request(at));
    }
    if popup::is_open(ctx, id) {
        popup::show(ctx);
    }
    match popup::answer(ctx, id)? {
        popup::Answer::Chosen(chosen) => menus::chosen(&chosen),
        popup::Answer::Dismissed => None,
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if let Err(e) = apertrust_display::tag_window(frame, ColorSpace::Srgb)
            && !self.tag_failed
        {
            tracing::warn!(target: "gpu", "window colour space untagged: {e}");
            self.tag_failed = true;
        }
        let ctx = ui.ctx().clone();
        // What says when this frame reaches the window, while the start marker waits.
        self.marker.mark(&ctx);
        // With nothing open the main window is the start window, and draws nothing else.
        self.follow_start(&ctx, frame);
        if self.start.shown {
            self.start_frame(ui, frame);
            return;
        }
        let t = theme(&ctx);
        let title_bar = self.window_buttons(frame, &ctx, t.sizes.top_bar);
        let maximised = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        let caption = (CAPTION_BUTTONS && title_bar.is_some()).then_some(maximised);
        // A press in the bands inside the window's edges, taken from egui's input.
        if let Some(way) = self.resize.take() {
            ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(way));
        }
        // The keys first, before any widget sees them. What they and the clicks ask for runs
        // after the frame, so the frame never draws half-updated state.
        let state = self.state(&ctx);
        self.shell.access_key(&ctx, &state);
        let keys = actions::read(&ctx, &self.shell.keymap, &state);
        self.shell.close_menu(&keys.pressed);
        // Its own controls as its own: AppKit checks a window's controls against that window,
        // its menus against the key window.
        let drawn = self.state_in_window(&ctx);
        let peek = keys.held.contains(&ActionId::PEEK);
        let mut run = keys.pressed;
        // The side panel runs the window's height under the top panel; the grid panel runs
        // under the photo only.
        let top_panel = egui::Panel::top("top panel")
            .exact_size(t.sizes.top_bar)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                // Saving… shows once a write is unconfirmed long enough, from a repaint asked
                // for then.
                let now = Instant::now();
                if let Some(from) = self.mirror.saving.saving_from()
                    && from > now
                {
                    ui.ctx().request_repaint_after(from - now);
                }
                let saved = self.mirror.saving.state(now);
                self.shell
                    .top_panel(ui, &drawn, saved, title_bar, caption, &self.jobs_view)
            });
        let mut window = None;
        match top_panel.inner {
            Some(FromTopPanel::Action(id)) => run.push(id),
            Some(FromTopPanel::Window(gesture)) => window = Some(gesture),
            Some(FromTopPanel::Jobs) => {
                let place = self.shell.jobs_window;
                let view = &self.jobs_view;
                jobs::refresh(
                    view,
                    Vec::new(),
                    &mut self.dialogs,
                    jobs::Show::Front,
                    place,
                    &ctx,
                );
            }
            Some(FromTopPanel::NotSaved) => self.ask_why_not_saved(&ctx),
            None => {}
        }
        // The side panel is as wide as its edge was dragged, a drag in progress included.
        // While the edge moves, the photo's size is held as it is while the window resizes.
        let edge_name = tr(&ctx, "panel-width");
        let edge = self
            .shell
            .panel_edge(&ctx, ui.available_width(), &edge_name);
        let panel_width = edge.width(&ctx);
        let panel_resized = std::mem::replace(&mut self.panel_width, panel_width) != panel_width;
        let side_panel = egui::Panel::right("side panel")
            .exact_size(panel_width)
            .resizable(false)
            .frame(egui::Frame::NONE.fill(t.colors.panel))
            .show(ui, |ui| self.side_panel(ui, peek));
        run.extend(side_panel.inner);
        // Never hidden: empty with no folder open (`docs/gui.md`, The grid panel). Along the
        // bottom, or a column at the window's left, or between the photo and the side
        // panel; as many rows or columns as its edge was dragged to, a drag in progress
        // included, the photo held while it moves as for the side panel's edge.
        let (grid_frame, grid_edge, lines) = self.shell.grid_frame(&ctx);
        let placed = (self.shell.grid.position, lines);
        let grid_resized = std::mem::replace(&mut self.grid_lines, placed) != placed;
        let grid_panel = grid_frame.show(ui, |ui| self.grid_panel(ui, lines));
        run.extend(grid_panel.inner);
        let grid_panel = grid_panel.response.rect;
        let central = egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(t.colors.photo_surround))
            .show(ui, |ui| {
                self.central_panel(ui, panel_resized || grid_resized)
            });
        let (central_bar, background) = central.inner;
        let central = central.response.rect;
        run.extend(background);
        // The side panel parts from the photo by a separator, as the bars do: its edge,
        // which resizes it, over the photo and the panel both; the grid panel's likewise.
        if let Some(action) = edge.show(ui, side_panel.response.rect) {
            self.shell.resize_panel(action);
        }
        if let Some(action) = grid_edge.show(ui, grid_panel) {
            self.shell.resize_grid(action);
        }
        self.shell.about.show(&ctx);
        run.extend(self.shell.popups(&ctx));

        // F6 goes through the regions in reading order.
        let regions: Vec<egui::Rect> = [
            Some(top_panel.response.rect),
            Some(central_bar),
            Some(side_panel.response.rect),
            Some(grid_panel),
        ]
        .into_iter()
        .flatten()
        .collect();
        self.run_actions(run, &ctx, &regions);
        // The scopes count what the section now shows: its tab, whether it is open, the
        // graph.
        if let Some(command) = self.mirror.scopes.ask(self.shell.scopes_wanted()) {
            self.session.send(command);
        }
        self.finish_frame(&ctx, frame, central.union(grid_panel), window);
    }

    /// What was chosen in the menus runs here, at once: eframe calls this before each frame,
    /// and alone while the window is minimised or hidden, when it runs no frame, so a choice
    /// such as Quit never waits for the window to show.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // The start marker goes once a frame has been drawn into the window (decision 82).
        self.marker.frame(ctx);
        // What the engine reported first, so the actions and the frame see it.
        self.drain(ctx);
        // The recents checked again as the window gains the focus, a folder that came back
        // no longer shown missing; what their checks have found, without waiting.
        // Finder's files, taken here since `logic` runs while the window is minimised too,
        // which then comes back and to the front to show them.
        if let Some(files) = apertrust_display::take_opened_files() {
            self.opened.look(files, ctx);
            self.reveal(ctx);
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        // The start window closed on the Mac comes back with a click on the Dock icon.
        if apertrust_display::take_reopened() {
            self.reveal(ctx);
        }
        let focused = ctx.input(|i| i.focused);
        if focused && !self.focused {
            self.shell.recents.check();
        }
        self.focused = focused;
        if self.shell.recents.take() {
            ctx.request_repaint();
        }
        let chosen = self.menus.as_ref().map(menus::NativeMenus::chosen);
        let chosen = chosen.unwrap_or_default();
        // A choice in the menus with the start window closed shows it again for what it
        // opens there, but for those that open no dialog over it.
        if reveals_for(&chosen) {
            self.reveal(ctx);
        }
        // No F6 among them: the menus hold no action that moves the focus.
        self.run_actions(chosen, ctx, &[]);
        // An alert answered, before anything else it may have held.
        if let Some((purpose, answer)) = self.alerts.answered(&mut self.dialogs) {
            self.alert_answered(purpose, answer);
        }
        // The main window's close button, held while a job the user started runs or the edit
        // is not on disk, for Quit to wait or ask.
        let edit = quit_edit(&self.mirror.saving);
        let holds = self.quit.holds_close(self.mirror.jobs.running(), edit);
        let jobs_hold = hold_close_for_jobs(ctx, holds);
        // Before the frame, so it shows what Settings' window changed.
        let state = self.state(ctx);
        let taken = take_dialogs(&mut self.shell, &mut self.dialogs, &state, ctx);
        apply_settings(&taken.settings, &mut self.shell, &mut self.keeper, ctx);
        if !taken.settings.is_empty() {
            keep_log(&self.kept, &self.shell.log);
            self.follow_cache_limit();
            self.follow_raw_thumbnails();
        }
        for asked in taken.storage {
            self.storage_asked(asked);
        }
        self.show_storage();
        for asked in taken.export {
            self.export(asked, ctx);
        }
        for action in taken.jobs {
            self.jobs_action(action, ctx);
        }
        for asked in taken.log {
            self.log_asked(asked, ctx);
        }
        self.log_saved(ctx);
        // The app's keys pressed in a dialog's own window, Quit among them.
        self.run_actions(taken.actions, ctx, &[]);
        // The start window's close, asked by its button, Escape, Close Window or, on the
        // Mac, its red button; the main window's red button, which closes what is open.
        self.close_start(ctx);
        if std::mem::take(&mut self.start.close_main) {
            self.run_actions([ActionId::CLOSE], ctx, &[]);
        }
        let size = ctx.input(|i| i.viewport_rect().size()) * ctx.zoom_factor();
        let placed = self.start.keeps_placement(size);
        settle(taken.quit, placed, &mut self.shell, &mut self.keeper, ctx);
        // Quit again: a dialog it closed has gone, the person said yes to the quit alert, the
        // jobs that were left to finish have, or the write it waited for is confirmed or
        // has failed.
        let finished = self.quit.confirmed() && self.mirror.jobs.running().none();
        let written = self.quit.waits_for_write() && !self.mirror.saving.writing();
        if taken.quit || jobs_hold || std::mem::take(&mut self.quit_again) || finished || written {
            self.run_actions([ActionId::QUIT], ctx, &[]);
        }
        // What Undo and Redo would do, in the language Settings may just have changed, for
        // the menus this frame builds or brings up to date.
        // Named only while the photo takes edits, as Undo and Redo run only then.
        let history = (self.mirror.history.as_ref()).filter(|_| self.mirror.takes_edits());
        let modules = self.mirror.modules();
        let steps = history::steps(history, modules, &translator(ctx));
        self.shell.steps = steps;
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        // A modal dialog keeps the main window's input from it, as X11 and Wayland leave
        // to the client, and as macOS does for a sheet's clicks but not its pointer.
        // An alert is a sheet on the window too.
        let modal = self.dialogs.shown().is_some_and(|shown| shown.modal());
        // A click on the main window, or its taking the focus, under a modal window brings
        // the dialog forward, as a system's modal window does; where the platform keeps it
        // in front already, a sheet or a window Windows' owner holds, nothing moves.
        let alert = self.alerts.pending().is_some();
        if raises_under_modal(modal, alert, ctx.os()) && reaches_under_modal(raw_input) {
            self.dialogs.raise();
        }
        if modal || self.alerts.pending().is_some() {
            under_modal(raw_input);
        }
        if std::mem::take(&mut self.handed_off) {
            release_after_drag(ctx, raw_input);
        }
        // What Undo or Redo, chosen from a menu, hand the focused text field.
        raw_input.events.append(&mut self.shell.field_keys);
        // The start window is never resized.
        if CAPTION_BUTTONS && !self.start.shown {
            self.resize = resize_press(ctx, raw_input, RESIZE_BAND);
        }
        // On the Mac the close button closes the start window, the app running on, and on
        // the main window with something open does what Close does, so eframe never sees
        // that close; the close Quit asks for still ends the app.
        if raw_input.viewport().close_requested() {
            let quitting = std::mem::take(&mut self.shell.leaving);
            // The main window's own state, whichever window has the focus, as AppKit checks
            // a window's buttons against that window.
            let state = self.state_in_window(ctx);
            let closes = close_window_runs(ctx.os(), &state) == ActionId::CLOSE;
            let button = close_button(ctx.os(), quitting, self.start.shown, state.modal, closes);
            if button != CloseButton::Quit
                && let Some(viewport) = raw_input.viewports.get_mut(&raw_input.viewport_id)
            {
                viewport.events.retain(|e| *e != egui::ViewportEvent::Close);
            }
            match button {
                CloseButton::Rest | CloseButton::Quit => {}
                CloseButton::HideStart => self.start.close = true,
                CloseButton::Close => self.start.close_main = true,
            }
        }
    }

    fn on_exit(&mut self) {
        self.keeper.flush();
        // What was open last, with the folder and photo active in it, for the next start;
        // as it was while it still reopens.
        keep_at_exit(
            &mut self.shell.recents,
            self.reopening.as_ref(),
            self.remembered.waiting.as_ref(),
            &self.mirror,
        );
        self.shell.recents.flush();
        // Restart now's: what is open, for the new start to open again, and whether the
        // graphics card or API changed this run, which `--safe-mode` gives way to.
        let alone = match &self.mirror.folder.folder {
            folder::Folder::Alone { path, .. } => Some(path.as_path()),
            _ => None,
        };
        let last = self.shell.recents.last().map(|last| last.path.as_path());
        let open = restart::Open::of(self.mirror.folder.root(), alone, last);
        let started = &self.hardware.started;
        let changed = recovery::graphics()
            .into_iter()
            .any(|setting| started.differs(setting.id(), self.keeper.value(setting).as_ref()));
        self.restart.ended(open, changed);
        self.marker.finish();
    }
}

/// What an open dialog's answer, or a drop, opens.
#[derive(Debug, PartialEq)]
enum Opens {
    Photo(PathBuf),
    Folder(PathBuf),
}

/// Sends a command to the session, returning its number: the app's session, or a test's
/// list.
type SendCommand<'a> = dyn FnMut(Command) -> u64 + 'a;

/// What the command line named, `open`, opened at the start: a folder in folder mode, or a
/// raw alone (`docs/library.md`, One photo), where no folder is listed, nothing enters the
/// recents, and the grid panel holds the photo by itself; nothing is written but its edit,
/// once something in it changes.
fn started(open: Start, mirror: &mut Mirror, send: &mut SendCommand<'_>) {
    match open {
        Start::Folder(path) => open_folder(path, mirror, send),
        Start::Photo(path) => {
            mirror.folder.alone(path.clone());
            let open = send(Command::Open(path.clone()));
            mirror.opening(path, open, Instant::now());
        }
    }
}

/// Opens the raw at `path` in place of the photo open, and its folder behind it
/// (`docs/gui.md`, File): where that folder is the root or inside its tree, chosen there
/// as a click in the tree chooses it, through [`change_folder`], the root and the recents
/// staying, unless it is active already; elsewhere opened as a way in that makes it the
/// tree's root and a recent. From a photo alone it opens folder mode.
fn open_photo(
    path: PathBuf,
    remembered: &mut Remembered,
    mirror: &mut Mirror,
    send: &mut SendCommand<'_>,
) {
    let folder = mirror.folder.to_show(&path);
    let open = send(Command::Open(path.clone()));
    mirror.opening(path, open, Instant::now());
    match folder {
        Some(folder::Show::Open(folder)) => open_folder(folder, mirror, send),
        // The photo is that folder's, so it stays, and none is waited for.
        Some(folder::Show::Choose(folder)) => {
            change_folder(folder, FolderChange::Choose, None, remembered, mirror, send);
        }
        None => {}
    }
}

/// Opens the folder at `path` in the library, its photos going into the grid panel, a way
/// in: it becomes Browse's root and, once open, the first of the recents. The photo open
/// stays open (`docs/events.md`, The library's events).
fn open_folder(path: PathBuf, mirror: &mut Mirror, send: &mut SendCommand<'_>) {
    // Absolute, from the folder the app started in, so the tree's folders are inside it for
    // the library, which lists no relative path; no disk is asked.
    let path = std::path::absolute(&path).unwrap_or(path);
    let source = Source::Folder(path.clone());
    let open = send(Command::Library(LibraryCommand::Open(source)));
    mirror.folder.opening(path, open);
}

/// Shows the folder at `path`, chosen in Browse's tree, in the grid panel: the library opens
/// it, and the tree's root, what it has listed and the recents stay as they were
/// (`docs/gui.md`, Browse).
fn choose_folder(path: PathBuf, mirror: &mut Mirror, send: &mut SendCommand<'_>) {
    let source = Source::Folder(path.clone());
    let open = send(Command::Library(LibraryCommand::Open(source)));
    mirror.folder.choosing(path, open);
}

/// File › Close: the photo open closed, its edit saved as the session saves any photo it
/// leaves, and the folder open closed, so the central panel shows where to start; the
/// photos `remembered` this run are forgotten, as what was open last is. Nothing that is
/// not open is closed.
fn close(remembered: &mut Remembered, mirror: &mut Mirror, send: &mut SendCommand<'_>) {
    *remembered = Remembered::default();
    close_photo(mirror, send);
    if !matches!(mirror.folder.folder, folder::Folder::None) {
        let close = send(Command::Library(LibraryCommand::Close));
        mirror.folder.closing(close);
    }
}

/// The photo open, or opening, closed with the folder left open: the session saves its
/// edit as it saves any photo it leaves and frees it, and the side panel shows no photo.
/// With none, nothing.
fn close_photo(mirror: &mut Mirror, send: &mut SendCommand<'_>) {
    if !matches!(mirror.photo, PhotoState::None) {
        send(Command::Close);
        mirror.closing();
    }
}

/// The photo last active in each folder this run, by the folder's path, kept in memory
/// alone, and the one the folder changed to waits to open once its listing holds it
/// ([`change_folder`]).
#[derive(Debug, Default)]
struct Remembered {
    photos: HashMap<PathBuf, PathBuf>,
    waiting: Option<Waiting>,
}

/// A remembered photo to open once the folder the library command numbered `open` lists
/// it, and how many of that folder's photos have been looked through for it.
#[derive(Debug, PartialEq)]
struct Waiting {
    open: u64,
    photo: PathBuf,
    looked: usize,
}

impl Remembered {
    /// Takes in the photo active now, opening or open, as the last active in the folder
    /// shown where it is one of that folder's.
    fn remember(&mut self, mirror: &Mirror) {
        let (Some(photo), Some(folder)) = (mirror.photo.asked(), mirror.folder.shown()) else {
            return;
        };
        if photo.parent() == Some(folder)
            && self.photos.get(folder).map(PathBuf::as_path) != Some(photo)
        {
            self.photos.insert(folder.to_owned(), photo.to_owned());
        }
    }

    /// Opens the photo waited for once the folder chosen lists it, as a click in the grid
    /// panel opens one, unless a photo was asked for meanwhile or another folder is; a
    /// listing that ends without it, or a folder that could not be read, ends the wait.
    fn restore(&mut self, mirror: &mut Mirror, send: &mut SendCommand<'_>) {
        let Some(waiting) = &mut self.waiting else {
            return;
        };
        let asked = !matches!(mirror.photo, PhotoState::None);
        if mirror.folder.latest() != waiting.open || asked {
            self.waiting = None;
            return;
        }
        match &mirror.folder.folder {
            folder::Folder::Opening { .. } => {}
            folder::Folder::Open(listing) => {
                let name = waiting.photo.file_name();
                if listed_since(listing, &mut waiting.looked, name) {
                    let photo = waiting.photo.clone();
                    let open = send(Command::Open(photo.clone()));
                    mirror.opening(photo, open, Instant::now());
                    self.waiting = None;
                } else if listing.done {
                    self.waiting = None;
                }
            }
            _ => self.waiting = None,
        }
    }
}

/// Whether the photos `listing` has listed since the first `looked` hold the one named
/// `name`, `looked` moved past them: a photo waited for is looked for once in each chunk.
fn listed_since(listing: &folder::Listing, looked: &mut usize, name: Option<&OsStr>) -> bool {
    let new = listing.photos.get(*looked..).unwrap_or_default();
    *looked = listing.photos.len();
    new.iter()
        .any(|photo| Some(OsStr::new(&photo.name)) == name)
}

/// How the active folder changes ([`change_folder`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FolderChange {
    /// A way in ([`open_folder`]): the folder becomes Browse's root.
    WayIn,
    /// A folder chosen in Browse's tree ([`choose_folder`]): the root stays.
    Choose,
}

/// The active folder changes to `path` by `change`, as the person asked: the one place the
/// rule that the photo shown is always one of the active folder's is kept (`docs/gui.md`,
/// Browse). The photo open is closed unless it is one of that folder's; then the photo
/// last active there this run, `remembered`, or else the one what was open last, `last`,
/// records for that folder, opens once the folder lists it. With none, or one gone from
/// it, none is shown. File › Open… of a raw and the start's reopening open a photo of
/// their own, and go round it.
fn change_folder(
    path: PathBuf,
    change: FolderChange,
    last: Option<&recents::Last>,
    remembered: &mut Remembered,
    mirror: &mut Mirror,
    send: &mut SendCommand<'_>,
) {
    // As the library is asked for it, so it compares with the photo's folder.
    let path = std::path::absolute(&path).unwrap_or(path);
    remembered.remember(mirror);
    let photo = mirror.photo.asked();
    let stays = photo.is_some_and(|photo| photo.parent() == Some(path.as_path()));
    if !stays {
        close_photo(mirror, send);
    }
    let kept = last
        .filter(|last| last.folder.as_deref().unwrap_or(&last.path) == path)
        .and_then(|last| last.photo.as_deref())
        .map(|name| path.join(name));
    let photo = remembered.photos.get(&path).cloned().or(kept);
    match change {
        FolderChange::WayIn => open_folder(path, mirror, send),
        FolderChange::Choose => choose_folder(path, mirror, send),
    }
    remembered.waiting = photo.filter(|_| !stays).map(|photo| Waiting {
        open: mirror.folder.latest(),
        photo,
        looked: 0,
    });
}

/// Whether each of `paths`, Open Recent's as a menu shows them, can be opened, by place:
/// all but one a check of `recents` found gone. One whose opening failed this run can,
/// which tries again.
fn recent_states(recents: &Recents, paths: &[PathBuf]) -> [Option<bool>; recents::MOST] {
    let mut states = [None; recents::MOST];
    for (state, path) in states.iter_mut().zip(paths) {
        *state = Some(!recents.missing(recents::OpenKind::Folder, path));
    }
    states
}

/// Forgets what was open last where `closed`, the root of the folder File › Close closed,
/// is it: nothing else, so closing a photo opened alone, which has no root, leaves it.
fn forget_last(recents: &mut Recents, closed: Option<&Path>) {
    if closed.is_some() && recents.last().map(|last| last.path.as_path()) == closed {
        recents.set_last(None);
        recents.save();
    }
}

/// The open folder's names, by recent, to tell each from the others: its own name, then
/// the folders it is in, nearest first, as many as it takes for no other recent to have
/// the same, or all it has. A path with no name, a root, is named whole.
fn told_apart(paths: &[&Path]) -> Vec<Vec<String>> {
    let names: Vec<Vec<String>> = paths
        .iter()
        .map(|path| {
            let names: Vec<String> = path
                .components()
                .rev()
                .filter_map(|part| match part {
                    std::path::Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
                    _ => None,
                })
                .collect();
            if names.is_empty() {
                vec![path.display().to_string()]
            } else {
                names
            }
        })
        .collect();
    let first = |names: &[String], n: usize| names[..n.min(names.len())].to_vec();
    names
        .iter()
        .enumerate()
        .map(|(i, own)| {
            let shared = |n: usize| {
                names
                    .iter()
                    .enumerate()
                    .any(|(j, other)| j != i && first(other, n) == first(own, n))
            };
            let n = (1..=own.len()).find(|n| !shared(*n)).unwrap_or(own.len());
            first(own, n)
        })
        .collect()
}

/// What was open last, reopening at the start: the library command followed, its root, the
/// folder of its tree to choose once the root has opened, the photo to open once its folder
/// lists it, and how many of that folder's photos have been looked through for it.
#[derive(Debug, PartialEq)]
struct Reopening {
    open: u64,
    root: PathBuf,
    folder: Option<PathBuf>,
    photo: Option<PathBuf>,
    looked: usize,
}

/// What the start opens with no path on the command line and no file from the system:
/// where `on` (`library.reopen_last`), what was open last, `last`: its root at once in
/// folder mode, a way in, the folder of its tree once the root has opened, chosen there as
/// a click in the tree chooses it, and its photo once that folder lists it ([`reopened`]).
/// Nothing is asked of the disk here: a folder that is gone fails as its reading does,
/// and one on a slow share says it is being read.
fn reopen(
    last: Option<&recents::Last>,
    on: bool,
    mirror: &mut Mirror,
    send: &mut SendCommand<'_>,
) -> Option<Reopening> {
    let last = last.filter(|_| on)?;
    match last.kind {
        recents::OpenKind::Folder => open_folder(last.path.clone(), mirror, send),
    }
    let folder = last.folder.clone().filter(|folder| *folder != last.path);
    let active = folder.as_deref().unwrap_or(&last.path);
    Some(Reopening {
        open: mirror.folder.latest(),
        root: last.path.clone(),
        photo: last.photo.as_ref().map(|name| active.join(name)),
        folder,
        looked: 0,
    })
}

/// Follows what was open last as it reopens, after the events of a frame. Once the root
/// has opened, the folder of its tree is chosen; once the folder followed lists the photo,
/// it opens, if no photo has been asked for meanwhile. A root that could not be read, or
/// whose reading was cancelled, is closed, so the central panel says nothing is open, its
/// recent marked; a folder of its tree that could not be is left for the root, chosen
/// again. It ends there, and as another folder is asked for, or the listing ends without
/// the photo.
fn reopened(reopening: &mut Option<Reopening>, mirror: &mut Mirror, send: &mut SendCommand<'_>) {
    let Some(asked) = reopening else {
        return;
    };
    if mirror.folder.latest() != asked.open {
        *reopening = None;
        return;
    }
    match &mirror.folder.folder {
        folder::Folder::Opening { .. } => {}
        folder::Folder::Open(_) if asked.folder.is_some() => {
            let folder = asked.folder.take().unwrap_or_default();
            choose_folder(folder, mirror, send);
            asked.open = mirror.folder.latest();
            asked.looked = 0;
        }
        folder::Folder::Open(listing) => {
            let none_asked = matches!(mirror.photo, PhotoState::None);
            let name = asked.photo.as_deref().and_then(Path::file_name);
            let listed = listed_since(listing, &mut asked.looked, name);
            let waits = name.is_some() && !listing.done;
            match asked.photo.take() {
                // Opened as it is, in the folder shown: no way in, so the tree keeps its root.
                Some(photo) if listed && none_asked => {
                    let open = send(Command::Open(photo.clone()));
                    mirror.opening(photo, open, Instant::now());
                    *reopening = None;
                }
                photo if none_asked && waits => asked.photo = photo,
                _ => *reopening = None,
            }
        }
        folder::Folder::Failed { path, .. } if *path != asked.root => {
            let root = asked.root.clone();
            choose_folder(root, mirror, send);
            *reopening = None;
        }
        folder::Folder::Failed { .. } => {
            let close = send(Command::Library(LibraryCommand::Close));
            mirror.folder.closing(close);
            *reopening = None;
        }
        folder::Folder::None | folder::Folder::Alone { .. } => *reopening = None,
    }
}

/// What was open last, as the app ends with `mirror` and the photo `waiting` for its
/// folder's listing ([`at_exit`]); while it still reopens, `reopening`, as it was, so
/// quitting early drops none of it.
fn keep_at_exit(
    recents: &mut Recents,
    reopening: Option<&Reopening>,
    waiting: Option<&Waiting>,
    mirror: &Mirror,
) {
    if reopening.is_none()
        && let Some(last) = recents.last().cloned()
    {
        recents.set_last(Some(at_exit(last, waiting, mirror)));
    }
}

/// What was open last, `last`, as the app ends with `mirror`: where its root is still the
/// tree's, the folder active in it where that is another, and the photo active there,
/// opening or open but never one whose open failed, or else the one `waiting` for that
/// folder's listing; else `last` as it was, so a photo opened alone, or another folder,
/// leaves it.
fn at_exit(last: recents::Last, waiting: Option<&Waiting>, mirror: &Mirror) -> recents::Last {
    if mirror.folder.root() != Some(last.path.as_path()) {
        return last;
    }
    let active = mirror.folder.shown().map(Path::to_owned);
    let waited = waiting
        .filter(|waiting| waiting.open == mirror.folder.latest())
        .map(|waiting| waiting.photo.as_path());
    let photo = mirror.photo.asked().or(waited);
    let photo = photo.filter(|photo| photo.parent() == active.as_deref());
    let name = photo.and_then(Path::file_name);
    recents::Last {
        folder: active.filter(|active| *active != last.path),
        photo: name.map(|name| name.to_string_lossy().into_owned()),
        ..last
    }
}

/// What an open dialog's answer or a drop opens, or the path it refuses and why.
type Opening = Result<Opens, (PathBuf, dropped::Refusal)>;

/// What `chosen`, an open dialog's answer, opens: for Open…, what a drop of the files
/// chosen would, told by their names (`dropped::chosen`), so a library's file or another
/// typed into the dialog is refused as a drop of it is; the folder chosen; for Locate…,
/// the folder chosen, which the missing recent now points to. None for an answer with no
/// path.
fn answered(shell: &mut Shell, chosen: dialogs::Chosen) -> Option<Opening> {
    let dialogs::Chosen { pick, paths } = chosen;
    match pick {
        dialogs::Pick::Files => dropped::chosen(&paths).map(dropped_opens),
        dialogs::Pick::Folder => paths.into_iter().next().map(|p| Ok(Opens::Folder(p))),
        dialogs::Pick::Locate(from) => {
            let to = paths.into_iter().next()?;
            Some(Ok(Opens::Folder(shell.located(&from, to))))
        }
    }
}

/// What a drop opens: the folder, or the first raw, with its folder, the others to be
/// selected with it as Open…'s are; or the first path, with why nothing opens.
fn dropped_opens(dropped: dropped::Dropped) -> Opening {
    match dropped {
        dropped::Dropped::Folder(path) => Ok(Opens::Folder(path)),
        dropped::Dropped::Raws { first, .. } => Ok(Opens::Photo(first)),
        dropped::Dropped::Refused { path, why } => Err((path, why)),
    }
}

/// Runs `id` if it can run where it is in `state`, with the dialogs as the app has them,
/// and returns its command for the session. Close Window closes the key window: a dialog's
/// own, Settings' on the Mac, else the main window. Quit, and Close Window on the main
/// window, close the dialog open first, as its Cancel, and leave once it has gone
/// ([`take_dialogs`]). Settings and Export asked for go to the dialog host, which opens
/// them in their platform's form, or brings Settings' modeless window to the front. View ›
/// Jobs opens the jobs window with `jobs`, the rows as they stand, or brings it to the
/// front, as the spinner's click does.
#[allow(clippy::too_many_arguments)]
fn run_action(
    shell: &mut Shell,
    dialogs: &mut dialog_host::AppDialogs,
    keeper: &Keeper,
    jobs: &[apertrust_ui::components::Job],
    shown: (&storage::View, &hardware::Running),
    id: ActionId,
    state: &State,
    ctx: &egui::Context,
    regions: &[egui::Rect],
    may_leave: &mut dyn FnMut() -> bool,
) -> Option<Command> {
    let enabled = actions::action(id).is_some_and(|a| a.enabled(state));
    if id == ActionId::CLOSE_WINDOW && enabled && state.dialog_window {
        dialogs.close();
        return None;
    }
    let leaves = id == ActionId::QUIT || id == ActionId::CLOSE_WINDOW;
    if leaves && enabled && !dialogs.quit() {
        return None;
    }
    // With a job the user started running, leaving asks first (`alerts::Guard`).
    if leaves && enabled && !may_leave() {
        return None;
    }
    let command = shell.run(id, state, ctx, regions);
    if std::mem::take(&mut shell.settings) {
        let close = ctx.os() != OperatingSystem::Mac;
        let (storage, hardware) = shown;
        let settings = settings_dialog::Settings::new(keeper, close)
            .with_storage(storage.clone())
            .with_hardware(hardware.clone());
        dialogs.open(settings, shell.settings_window, ctx);
    }
    if std::mem::take(&mut shell.export_asked) {
        let export = export::Export::new(shell.export.clone())
            .alerts(dialog_host::platform::shows_alerts(ctx.os()));
        dialogs.open(export, None, ctx);
    }
    if std::mem::take(&mut shell.show_jobs) {
        let show = jobs::Show::Front;
        jobs::refresh(jobs, Vec::new(), dialogs, show, shell.jobs_window, ctx);
    }
    command
}

/// What the dialogs bring before each of the main window's frames, `ctx`'s, as
/// `App::logic` takes it: what the user changed in Settings, for the keeper to keep and
/// the shell to show (`Taken::settings`), and where Settings' window was left, kept for
/// next time;
/// the Export dialog's settings, kept by the shell as they change, and what it asks of the
/// session (`Taken::export`); the app's keys pressed in a dialog's own window, the actions
/// they ask for that can run in `state`, the app's with the dialog open (`Taken::actions`);
/// the main window's close, from its close button, held for the dialog open
/// ([`hold_close`]); and the focus back from a dialog gone.
fn take_dialogs(
    shell: &mut Shell,
    dialogs: &mut dialog_host::AppDialogs,
    state: &State,
    ctx: &egui::Context,
) -> Taken {
    let mut export = Vec::new();
    let mut jobs = Vec::new();
    let mut log = Vec::new();
    let mut actions = Vec::new();
    let mut settings = Vec::new();
    let mut storage = Vec::new();
    for output in dialogs.outputs() {
        match output {
            dialog_host::Output::Keys(keys) => actions.extend(actions::read_from_dialog(
                &keys.events,
                keys.focus,
                keys.os,
                &shell.keymap,
                state,
            )),
            dialog_host::Output::Settings(change) => settings.push(change),
            dialog_host::Output::Storage(asked) => storage.push(asked),
            // Settings' Restart now: the registry's action, run as the keys' are.
            dialog_host::Output::Restart => actions.push(ActionId::RESTART),
            dialog_host::Output::Placed(place) => shell.settings_window = Some(place),
            dialog_host::Output::JobsPlaced(place) => shell.jobs_window = Some(place),
            dialog_host::Output::Jobs(action) => jobs.push(action),
            dialog_host::Output::LogPlaced(place) => shell.log.window = Some(place),
            // Kept in whole points, as the schema has it.
            dialog_host::Output::LogSized(size) => {
                shell.log.size = Some([size.x.round(), size.y.round()]);
            }
            dialog_host::Output::Log(asked) => log.push(asked),
            dialog_host::Output::Export(asked) => {
                if let Some(values) = &asked.values {
                    shell.export = values.clone();
                }
                export.push(asked);
            }
        }
    }
    let held = hold_close(ctx, dialogs);
    let gone = dialogs.frame(ctx);
    Taken {
        quit: held || gone,
        settings,
        storage,
        export,
        jobs,
        log,
        actions,
    }
}

/// Before Quit runs: hands the shell's settings to `keeper`, which `on_exit` flushes. The
/// windows closed for Quit report where they were left only as they go, so their places
/// (Settings', the jobs window's) are in the shell by now and in the file only if the
/// keeper has them before the app leaves, whether or not a frame still follows.
/// The window's size and place are kept only where they are the main window's own
/// (`placed`), as [`keep`] keeps them.
fn settle(quit: bool, placed: bool, shell: &mut Shell, keeper: &mut Keeper, ctx: &egui::Context) {
    if quit {
        keep(shell, keeper, ctx, placed);
    }
}

/// What the dialogs brought before a frame ([`take_dialogs`]).
struct Taken {
    /// Whether Quit is to run now: to close the dialog before the main window, or because
    /// the dialog Quit was held for has gone.
    quit: bool,
    /// What the user changed in Settings, in order, for the keeper to keep and the shell
    /// to show ([`apply_settings`]).
    settings: Vec<settings_dialog::Change>,
    /// What Settings › Storage asked, in order, after the settings changed with it.
    storage: Vec<storage::Asked>,
    /// What the Export dialog asked of the session, in order.
    export: Vec<export::Asked>,
    /// What was pressed in the jobs window, in order.
    jobs: Vec<apertrust_ui::components::ActivityAction>,
    /// What the log window asked, in order.
    log: Vec<log_window::Asked>,
    /// The actions the app's keys asked for in a dialog's own window, in order.
    actions: Vec<ActionId>,
}

/// The action that runs for `id`: for Restart (decision 82), Quit, the restart noted in
/// `restart`, so it goes through the app's own quit path, the dialog open closed as its
/// Cancel and the quit alert asked where a job runs, and Quit's own runs of itself later on
/// that path, once the dialog has gone or the jobs have ended, keep it; `main` starts the
/// app again once it has ended. Where the app to start cannot be found, nothing runs, and
/// the error says why. Any other as it is.
fn quit_to_restart(id: ActionId, restart: &mut restart::Restart) -> std::io::Result<ActionId> {
    if id != ActionId::RESTART {
        return Ok(id);
    }
    restart.ask(true)?;
    Ok(ActionId::QUIT)
}

/// Whether `id` runs now, while the open dialog asks (`asking`) or not: every action but
/// Quit, which waits for its answer (`held`). AppKit's open panel hangs on the main window as
/// a sheet does, and a Quit under it, checked by hand on the M4, took the panel away and
/// left the window open, the app running on; the panel's answer is the person's to give.
fn hold_for_panel(id: ActionId, asking: bool, held: &mut bool) -> bool {
    if id == ActionId::QUIT && asking {
        *held = true;
        return false;
    }
    true
}

/// The actions' availability in `state`, the shell's, with the dialogs beside it: the open
/// dialog while it asks (`asking`), modal to the main window as AppKit's open panel is, a
/// sheet on it; and the host's dialog, `shown`. A modal one rests every action that opens
/// a dialog and the view's keys; a modeless one's window with the focus rests the view's
/// keys and the items that act on the main window.
fn with_dialogs(mut state: State, asking: bool, shown: Option<dialog_host::Shown>) -> State {
    state.modal |= asking;
    if let Some(shown) = shown {
        state.modal |= shown.modal();
        state.dialog |= shown.modal() || shown.focused;
        state.dialog_window |= shown.focused && !shown.modal();
    }
    state
}

/// What Close Window runs on the main window, where it has the focus, in `state`
/// (`docs/gui.md`, File): on the Mac with something open, Close, the start window taking the
/// main window's place, as the Mac's apps close a document's window and stay; else itself,
/// which ends the app, as Quit does. A dialog's own window with the focus leaves Close
/// disabled, so Close Window closes that window.
fn close_window_runs(os: OperatingSystem, state: &State) -> ActionId {
    let close = actions::action(ActionId::CLOSE).is_some_and(|a| a.enabled(state));
    if os == OperatingSystem::Mac && close {
        ActionId::CLOSE
    } else {
        ActionId::CLOSE_WINDOW
    }
}

/// What the main window's close button does (`docs/gui.md`, File): on the Mac, nothing under
/// a modal dialog or alert (`modal`), as Close Window rests there; closes the start window
/// where it is `shown`; does what Close does where Close Window would (`closes`); else, and
/// on Windows and Linux, and for the close Quit asked for (`quitting`), it ends the app.
fn close_button(
    os: OperatingSystem,
    quitting: bool,
    shown: bool,
    modal: bool,
    closes: bool,
) -> CloseButton {
    if os != OperatingSystem::Mac || quitting {
        CloseButton::Quit
    } else if modal {
        CloseButton::Rest
    } else if shown {
        CloseButton::HideStart
    } else if closes {
        CloseButton::Close
    } else {
        CloseButton::Quit
    }
}

/// What the main window's close button does ([`close_button`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CloseButton {
    Rest,
    HideStart,
    Close,
    Quit,
}

/// What closing the start window does on `os`: on the Mac it hides, the app running on with
/// its menu bar; elsewhere the app leaves, as Quit does.
fn start_closes(os: OperatingSystem) -> StartCloses {
    if os == OperatingSystem::Mac {
        StartCloses::Hide
    } else {
        StartCloses::Quit
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StartCloses {
    Hide,
    Quit,
}

/// Whether `chosen`, from the Mac's menus with the start window hidden, shows it again: any
/// but those in [`OVER_NO_WINDOW`]. Quit's alert shows it itself, should Quit ask.
fn reveals_for(chosen: &[ActionId]) -> bool {
    chosen.iter().any(|id| !OVER_NO_WINDOW.contains(id))
}

/// What the window does this frame as the start window `shown` (`wanted`), as it is to take
/// the start window's place (`reshape`) or the scale has `changed`: leave full screen, then
/// its maximised state, then take the start window's size, centred where the main window
/// was only as it becomes the start window; nothing otherwise.
fn start_shape(
    wanted: bool,
    reshape: bool,
    changed: bool,
    full_screen: bool,
    maximised: bool,
) -> StartShape {
    if !wanted || !(reshape || changed) {
        StartShape::Keep
    } else if full_screen {
        StartShape::LeaveFullScreen
    } else if maximised {
        StartShape::Unmaximise
    } else {
        StartShape::Resize { centred: reshape }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StartShape {
    Keep,
    LeaveFullScreen,
    Unmaximise,
    Resize { centred: bool },
}

/// What a choice in the Mac's menus runs with the start window closed, its window hidden,
/// without showing it again: what opens a window of its own, or leaves, or changes only the
/// recents; and Close Window, the start window being closed already.
const OVER_NO_WINDOW: [ActionId; 6] = [
    ActionId::QUIT,
    ActionId::CLOSE_WINDOW,
    ActionId::SETTINGS,
    ActionId::SHOW_LOG,
    ActionId::SHOW_JOBS,
    ActionId::CLEAR_RECENT,
];

/// The actions' availability, `state`, with the main window as the start window
/// (`docs/gui.md`, The start window): as with a modeless dialog's window with the focus, the
/// view's keys rest and so do the items that act on the main window or its photo, and Close
/// Window closes it, but under a modal dialog or alert over it.
fn in_start(mut state: State) -> State {
    state.dialog = true;
    state.dialog_window |= !state.modal;
    state
}

/// Whether anything is open in `mirror`, a folder or a photo, opening, open or failed.
fn opened(mirror: &Mirror) -> bool {
    !matches!(mirror.folder.folder, folder::Folder::None)
        || !matches!(mirror.photo, PhotoState::None)
}

/// Whether the main window is the start window (`docs/gui.md`, The start window): never
/// while anything is `opened`; with nothing, it stays so once `shown`, and becomes so but
/// while what the system or a drop handed the app is still `looking`, or a modal dialog or
/// alert is over the main window (`modal`), as the refusal of what was handed over is. The
/// main window then shows with nothing open until it is answered: the one time it does.
fn shows_start(opened: bool, looking: bool, modal: bool, shown: bool) -> bool {
    !opened && (shown || !(looking || modal))
}

/// The open photo's edit as Quit finds it: a write under way, which it waits for, or the last
/// write failed, the open photo's or the photo left's, which it asks about.
fn quit_edit(saving: &mirror::Saving) -> alerts::Edit<'_> {
    match saving.failed() {
        _ if saving.writing() => alerts::Edit::Writing,
        Some(why) => alerts::Edit::Failed(why),
        None => alerts::Edit::Saved,
    }
}

/// Whether the main window's close, asked by its close button this frame, is held because a
/// job the user started runs or the edit is not on disk (`holds`): then it is cancelled,
/// for Quit to wait or to ask.
fn hold_close_for_jobs(ctx: &egui::Context, holds: bool) -> bool {
    let held = holds && ctx.input(|i| i.viewport().close_requested());
    if held {
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
    }
    held
}

/// Whether the main window's close, asked by its close button this frame, is held for the
/// dialog open: then it is cancelled, for Quit to close the dialog first and the window
/// once it has gone, as decision 41 has it.
fn hold_close(ctx: &egui::Context, dialogs: &dialog_host::AppDialogs) -> bool {
    let asked = ctx.input(|i| i.viewport().close_requested());
    let held = asked && dialogs.shown().is_some();
    if held {
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
    }
    held
}

/// An editing tab's sections as the mirror's photo has them (`docs/gui.md`, The side
/// panel): the open photo's values and Scopes; with no photo, and from the moment another
/// is asked for until its values arrive, every module at its defaults and Scopes with no
/// graph, all drawn disabled and each open or closed as it was, never a blank panel. Only the open photo's sections send the session
/// anything ([`Mirror::takes_edits`], which [`App::change`] asks); every section opens,
/// closes and changes tab whatever it shows.
fn edit_tab(
    ui: &mut egui::Ui,
    shell: &Shell,
    mirror: &Mirror,
    panels: &Panels,
    peek: bool,
) -> (Vec<scopes::Change>, Vec<Change>) {
    let shown = mirror.shown();
    // A photo opening shows Scopes as with no photo until it is live (decision 77).
    let photo = matches!(shown, Shown::Photo { .. });
    let scopes = Some((photo.then(|| mirror.scopes.measured()).flatten(), photo));
    let (modules, values) = (mirror.modules(), &mirror.values);
    // Every control of a photo opening says why it is disabled.
    let words = translator(ui.ctx());
    let opening = mirror.opening_name().map(|name| {
        let mut args = apertrust_ui::i18n::Args::new();
        args.set(
            "name",
            Path::new(&name)
                .file_stem()
                .map_or(name.clone(), |stem| stem.to_string_lossy().into_owned()),
        );
        words.text_with("controls-opening", Some(&args))
    });
    // A module's picker is pressed only on the open photo once live, and never on one
    // open for viewing only, whose notice says why.
    let view_only = match &mirror.photo {
        PhotoState::Open(photo) => photo.view_only.as_ref(),
        _ => None,
    };
    let view_only = view_only.map(|view_only| view_only_notice(&words, view_only));
    let pickers = modules::Pickers {
        drawn: modules::PICKING,
        on: mirror.pickers.active(),
        enabled: shown.editable(),
        why: opening.as_deref().or(view_only.as_deref()),
    };
    panel_body(
        ui,
        shell,
        scopes,
        modules,
        values,
        panels,
        peek,
        shown.editable(),
        opening.as_deref(),
        &pickers,
    )
}

/// The side panel's sections under its tab bar: Scopes pinned over the band where the
/// tab pins it, unless `scopes` is `None`, its graph from what it holds and drawn disabled
/// unless its flag says a photo is shown, then the tab's
/// modules among `modules`, drawn from their definitions with the mirror's `values`; each
/// section open as the shell keeps it, peeking while `peek` holds, its reset disabled
/// unless `editable`, and its colour pickers as `pickers` says. Returns what Scopes and the
/// modules asked for.
#[expect(
    clippy::too_many_arguments,
    reason = "the app's own state for the side panel, each part apart"
)]
fn panel_body(
    ui: &mut egui::Ui,
    shell: &Shell,
    scopes: Option<(Option<&scopes::Measured>, bool)>,
    modules: &[&dyn Module],
    values: &Values,
    panels: &Panels,
    peek: bool,
    editable: bool,
    opening: Option<&str>,
    pickers: &modules::Pickers,
) -> (Vec<scopes::Change>, Vec<Change>) {
    let shown = TAB_MODULES
        .get(shell.panel_tab)
        .copied()
        .unwrap_or_default();
    let (mut pinned, mut changes) = (Vec::new(), Vec::new());
    let open = shell.section_open(scopes::ID, scopes::OPEN);
    let mut body = PanelBody::new();
    let scopes = scopes.filter(|_| shell.scopes_pinned());
    let (measured, enabled) = scopes.unwrap_or((None, false));
    if scopes.is_some() {
        body = body
            .pinned(|ui| pinned = scopes::section(ui, shell.scopes, open, measured, peek, enabled));
    }
    body.show(ui, |ui| {
        for id in shown {
            let module = modules.iter().find(|module| module.definition().id == *id);
            let state = values.modules.iter().find(|state| state.module == *id);
            if let (Some(module), Some(state)) = (module, state) {
                let open = shell.section_open(id, modules::OPEN);
                let groups = |group: &str| shell.group_open(id, group);
                // Each section's widgets in an id of their own, the module's: two modules'
                // sliders of one name, as Vignetting's and Soften's Brightness, never share
                // one.
                let drawn = ui.push_id(id, |ui| {
                    modules::section(
                        ui, *module, state, open, panels, &groups, peek, editable, opening, pickers,
                    )
                });
                changes.extend(drawn.inner);
            }
        }
    });
    (pinned, changes)
}

/// Prepares `ctx` for the shell before its first frame: its theme, its language and
/// `scale`, with the registry owning every key. The scale is set in egui's options, not
/// with `set_zoom_factor`, which would only apply from the second frame, so the first
/// would be laid out at the wrong scale.
fn prepare(ctx: &egui::Context, shell: &Shell, scale: f32) {
    apertrust_ui::install(ctx, shell.look());
    apertrust_ui::components::set_marker(ctx, shell.marker);
    registry_keys_only(ctx);
    apertrust_ui::set_locale(ctx, shell.locale);
    ctx.options_mut(|o| o.zoom_factor = scale);
}

/// Hands the frame's settings to `keeper`, which writes them once they have stood still,
/// and asks for a frame then, should nothing else bring one; keeps the window's size and
/// place, and maximises it once it shows, if it was, only where they are the main window's
/// own (`placed`), never the start window's.
fn keep(shell: &mut Shell, keeper: &mut Keeper, ctx: &egui::Context, placed: bool) {
    if placed {
        shell.maximise(ctx);
        shell.track_window(ctx);
    }
    if let Some(wait) = keeper.frame(Instant::now(), &shell.to_settings(ctx)) {
        ctx.request_repaint_after(wait);
    }
}

/// How long the window must keep its size before the photo is rendered for it again.
const RESIZE_SETTLED: std::time::Duration = std::time::Duration::from_millis(150);

/// The photo area's size for the engine, held while the window resizes. AppKit resizes it
/// step by step through a zoom's animation or a live resize, and eframe draws each step;
/// the engine could not render each step in time, and nobody would see most of them, so
/// the last frame is shown scaled to the coming size until the window has kept its size
/// for [`RESIZE_SETTLED`]. The side panel's edge, dragged or held under a key, resizes the
/// photo the same way and is held the same way.
#[derive(Debug, Default)]
struct Bounds {
    /// The size last sent, in physical pixels.
    sent: (u32, u32),
    /// A size held while the window resizes.
    held: Option<(u32, u32)>,
    /// When the window last changed size.
    resized: Option<Instant>,
}

impl Bounds {
    /// The size to send the engine now, if any: `bounds` at once, unless the window has
    /// changed size (`resized`, this frame) within [`RESIZE_SETTLED`] and a size has been
    /// sent before, when it waits.
    fn update(&mut self, bounds: (u32, u32), resized: bool, now: Instant) -> Option<(u32, u32)> {
        if resized {
            self.resized = Some(now);
        }
        if bounds.0 == 0 || bounds.1 == 0 || bounds == self.sent {
            self.held = None;
            return None;
        }
        let settled = self
            .resized
            .is_none_or(|at| now.duration_since(at) >= RESIZE_SETTLED);
        if !settled && self.sent != (0, 0) {
            self.held = Some(bounds);
            return None;
        }
        self.held = None;
        self.sent = bounds;
        Some(bounds)
    }

    /// How long until a held size is due, for a frame then.
    fn wait(&self, now: Instant) -> Option<std::time::Duration> {
        let at = self.resized?;
        self.held
            .map(|_| RESIZE_SETTLED.saturating_sub(now.duration_since(at)))
    }
}

/// Turns off the keys egui handles itself, at the end of each frame, for whatever the
/// registry left: Ctrl+=, Ctrl+− and Ctrl+0, and Ctrl+Q, which it matches with any Shift
/// or Alt, so Ctrl+Shift+Q would close the window. The registry owns every key.
fn registry_keys_only(ctx: &egui::Context) {
    ctx.options_mut(|o| {
        o.zoom_with_keyboard = false;
        o.quit_shortcuts.clear();
    });
}

/// The widgets drawn this frame that can take focus, in the order egui met them.
fn focusable(ctx: &egui::Context) -> Vec<(egui::Id, egui::Rect)> {
    ctx.viewport(|viewport| {
        viewport
            .this_pass
            .widgets
            .get_layer(egui::LayerId::background())
            .filter(|w| w.enabled && w.visible && w.sense.is_focusable())
            .map(|w| (w.id, w.rect))
            .collect()
    })
}

/// What the line under the photo says of a photo whose edit a newer Apertrust wrote: the
/// version that wrote it, where the edit records one, and that it is open for viewing only.
fn view_only_notice(words: &Translator, view_only: &ViewOnly) -> String {
    match &view_only.build {
        Some(build) => {
            let mut args = apertrust_ui::i18n::Args::new();
            args.set("version", build.as_str());
            words.text_with("photo-view-only", Some(&args))
        }
        None => words.text("photo-view-only-unknown"),
    }
}

fn file_name(path: &std::path::Path) -> String {
    path.file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mirror::{reported, show};
    use apertrust_engine::session::{Event, LibraryEvent, Mark, Photo};
    use apertrust_plugin::ImageContext;
    use apertrust_ui::components::{Section, SliderRow};
    use egui::accesskit::{Role, Toggled};
    use egui::{Key, Modifiers};
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT as _, Queryable as _};
    use std::path::Path;

    /// A cache folder that cannot be created or written is warned of once, as the app
    /// starts, and the app runs with no disk cache; one that can is created (decision 76).
    #[test]
    fn a_cache_folder_that_cannot_be_written_is_warned_of_once_and_left_out() {
        let folder =
            std::env::temp_dir().join(format!("apertrust-app-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        // A file where the cache folder should be, on every platform.
        let file = folder.join("cache");
        std::fs::write(&file, b"").unwrap();
        let (kept, dispatch, _serial) = crate::testing::kept(Some(&[]));
        let cache = tracing::dispatcher::with_default(&dispatch, || open_cache(&file));
        assert!(cache.is_none());
        let read = kept.read(&mut Default::default());
        let warnings: Vec<&str> = read
            .lines
            .iter()
            .map(|line| line.text())
            .filter(|text| text.contains("cache folder"))
            .collect();
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("warning:"), "{warnings:?}");
        assert!(warnings[0].contains(&file.display().to_string()));

        let made = folder.join("made");
        let cache = open_cache(&made).expect("a cache folder that can be written");
        assert_eq!(cache.folder(), made);
        assert!(made.is_dir());
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// The session is given the cache's limit again only where Settings changed it: lowered,
    /// turned off, or reset to its default.
    #[test]
    fn the_cache_s_limit_is_sent_again_only_when_it_changes() {
        let folder = settings::testing::Folder::new("cache-limit");
        let (mut keeper, settings) = Keeper::open(folder.storage());
        let mut last = keeper.cache_limit();
        assert_eq!(last, Limit::Bytes(4_000_000_000));
        let mut set = |id, value: Option<toml::Value>| {
            keeper.apply(&settings, id, value.as_ref());
            keeper.cache_limit()
        };
        let lowered = set("cache.limit", Some(2.0.into()));
        assert_eq!(
            changed(&mut last, lowered),
            Some(Limit::Bytes(2_000_000_000))
        );
        let same = set("cache.limit", Some(2.0.into()));
        assert_eq!(changed(&mut last, same), None);
        let off = set("cache.limited", Some(false.into()));
        assert_eq!(changed(&mut last, off), Some(Limit::Unlimited));
        let on = set("cache.limited", None);
        assert_eq!(changed(&mut last, on), Some(Limit::Bytes(2_000_000_000)));
        let reset = set("cache.limit", None);
        assert_eq!(changed(&mut last, reset), Some(Limit::Bytes(4_000_000_000)));
        assert_eq!(last, Limit::Bytes(4_000_000_000));
    }

    /// The thumbnails' setting is read from the engine's settings, darktable's never by
    /// default, and sent again only when Settings changes it.
    #[test]
    fn the_raw_thumbnails_setting_is_sent_again_only_when_it_changes() {
        let folder = settings::testing::Folder::new("raw-thumbnails");
        let (mut keeper, settings) = Keeper::open(folder.storage());
        let mut last = keeper.raw_thumbnails();
        assert_eq!(last, RawThumbnails::Never);
        let mut set = |value: Option<toml::Value>| {
            keeper.apply(&settings, "library.raw_thumbnails_from", value.as_ref());
            keeper.raw_thumbnails()
        };
        let p720 = set(Some("720p".into()));
        assert_eq!(changed(&mut last, p720), Some(RawThumbnails::P720));
        let same = set(Some("720p".into()));
        assert_eq!(changed(&mut last, same), None);
        let reset = set(None);
        assert_eq!(changed(&mut last, reset), Some(RawThumbnails::Never));
    }

    /// A limit Settings changed is sent, then the sizes asked again, so Storage's total and
    /// limit agree; the same limit sends nothing.
    #[test]
    fn a_changed_limit_asks_for_the_sizes_again() {
        let mut last = Limit::Bytes(4_000_000_000);
        let lowered = Limit::Bytes(2_000_000_000);
        assert_eq!(
            cache_commands(&mut last, lowered),
            [
                Command::Cache(CacheCommand::Limit(lowered)),
                Command::Cache(CacheCommand::Sizes),
            ]
        );
        assert_eq!(cache_commands(&mut last, lowered), []);
        assert_eq!(
            cache_commands(&mut last, Limit::Unlimited),
            [
                Command::Cache(CacheCommand::Limit(Limit::Unlimited)),
                Command::Cache(CacheCommand::Sizes),
            ]
        );
    }

    #[test]
    fn a_newer_apertrust_s_edit_says_which_version_wrote_it() {
        use apertrust_engine::sidecar::Newer;
        let words = Translator::new(Locale::EnGb);
        let view_only = |build: Option<&str>| ViewOnly {
            build: build.map(str::to_owned),
            newer: Newer::Module("lens".to_owned()),
        };
        assert_eq!(
            view_only_notice(&words, &view_only(Some("0.9.0+3~gabcdef0"))),
            "Edited in a newer Apertrust, 0.9.0+3~gabcdef0: open for viewing only"
        );
        assert_eq!(
            view_only_notice(&words, &view_only(None)),
            "Edited in a newer Apertrust: open for viewing only"
        );
    }

    /// A section's changes of several commands, the tone equalizer's release of its bands and
    /// its reset of them, go as one `Step { source: You }`, one history step, each change
    /// shown as it would be alone.
    #[test]
    fn changes_of_several_commands_are_sent_as_one_step() {
        let set = |control, value| Change::Set {
            module: "toneequal",
            control,
            value: ControlValue::Number(value),
            done: true,
        };
        let reset = |control| Change::Reset {
            module: "toneequal",
            control,
        };
        let changes = vec![set("shadows", 0.5), set("blacks", 0.25), reset("noise")];
        let alone: Vec<_> = changes.iter().cloned().map(|c| sent(c).unwrap()).collect();
        let (command, shown) = sent(Change::Step(changes)).unwrap();
        let (commands, each): (Vec<_>, Vec<_>) = alone.into_iter().unzip();
        assert!(matches!(commands[2], Command::ResetControl { .. }));
        assert_eq!(
            command,
            Command::Step {
                source: StepSource::You,
                note: None,
                commands,
            }
        );
        assert_eq!(shown, each.concat());
        assert_eq!(shown.len(), 3);
        // It completes a change, which the save state waits for; a `Step` of a gesture's
        // changes alone does not, as the engine says.
        assert!(completes(&command));
        let dragged = Change::Set {
            module: "toneequal",
            control: "shadows",
            value: ControlValue::Number(0.5),
            done: false,
        };
        let (gesture, _) = sent(Change::Step(vec![dragged])).unwrap();
        assert!(!completes(&gesture));
    }

    #[test]
    fn changes_shown_as_they_are_sent_show_what_the_session_s_edit_will_report() {
        use apertrust_engine::edit::{self, Edit, Held};
        use apertrust_engine::session::{ControlState, CoordinateState, ModuleState};
        use apertrust_modules::testing::{self, Twin};

        /// A change to Twin, through a control's row or a coordinate's.
        #[derive(Debug, Clone, Copy, PartialEq)]
        enum Step {
            Set(&'static str, f32),
            Reset(&'static str),
            Keep(&'static str, bool),
            ResetModule,
        }

        let image = testing::image();
        let modules: &[&dyn Module] = &[&Twin];
        let definition = Twin.definition();
        let coordinate = |id| definition.coordinate(id).is_some();
        // What the session reports of Twin with `edit` and `held`: each control's value
        // over its default and its mark, each view's coordinates, held or the module's
        // view of the values, with the mark its controls share.
        let report = |edit: &Edit, held: &Held| {
            let set = edit.module("twin");
            let defaults = Twin.defaults(&image);
            let controls: Vec<ControlState> = definition
                .controls
                .iter()
                .map(|control| {
                    let default = defaults.get(control.id).unwrap().clone();
                    let value = set.and_then(|v| v.get(control.id)).cloned();
                    ControlState {
                        control: control.id,
                        value: value.unwrap_or_else(|| default.clone()),
                        default,
                        mark: set.and_then(|v| v.mark(control.id)),
                        available: true,
                        options: Vec::new(),
                        range: None,
                    }
                })
                .collect();
            let mut views = Vec::new();
            for view in definition.views {
                let values = edit.coordinates(held, &Twin, view, &image).unwrap();
                let defaults = edit::default_coordinates(&Twin, view, &image).unwrap();
                let marks: Vec<_> = view
                    .controls
                    .iter()
                    .map(|c| set.and_then(|v| v.mark(c)))
                    .collect();
                let mark = marks[0].filter(|_| marks.iter().all(|m| *m == marks[0]));
                let each = view.coordinates.iter().zip(values).zip(defaults);
                views.extend(each.map(|((c, value), default)| CoordinateState {
                    view: view.id,
                    coordinate: c.id,
                    value,
                    default,
                    mark,
                    available: true,
                }));
            }
            let edited = controls.iter().any(|c| c.value != c.default);
            Values {
                modules: vec![ModuleState {
                    module: "twin",
                    enabled: true,
                    edited,
                    controls,
                    presets: Vec::new(),
                    views,
                }],
                settings: Vec::new(),
                edited,
                through: 0,
                extent: [84, 62],
                live: true,
            }
        };
        // The step on the session's edit, its holds then pruned as after every change.
        let apply = |edit: &mut Edit, held: &mut Held, step: Step| {
            let number = ControlValue::Number;
            let done = match step {
                Step::Set(id, v) if coordinate(id) => {
                    edit.set_coordinate(held, &Twin, id, number(v), &image)
                }
                Step::Set(id, v) => edit.set_control(&Twin, id, number(v), &image),
                Step::Reset(id) if coordinate(id) => edit.reset_coordinate(held, &Twin, id, &image),
                Step::Reset(id) => edit.reset_control(&Twin, id),
                Step::Keep(id, kept) if coordinate(id) => edit.keep_coordinate(&Twin, id, kept),
                Step::Keep(id, kept) => edit.keep(&Twin, id, kept),
                Step::ResetModule => {
                    edit.reset_module(&Twin);
                    Ok(())
                }
            };
            done.unwrap();
            held.prune(edit);
        };
        // The step as the app sends it.
        let pending = |step: Step| {
            let module = "twin";
            match step {
                Step::Set(control, v) => Pending::Control {
                    module,
                    control,
                    value: ControlValue::Number(v),
                },
                Step::Reset(control) => Pending::Reset { module, control },
                Step::Keep(control, kept) => Pending::Keep {
                    module,
                    control,
                    kept,
                },
                Step::ResetModule => Pending::ResetModule { module },
            }
        };
        // x through its own row, then the pair, then the mix, which moves the pair, and
        // back: a coordinate reset with the other held, released and kept, x reset, both
        // of the pair's back, which resets x and y and moves the mix, then the module.
        let steps = [
            Step::Set("x", 30.0),
            Step::Set("sum", 110.0),
            Step::Set("sum", 111.0),
            Step::Set("u", 200.0),
            Step::Set("v", 12.0),
            Step::Reset("difference"),
            Step::Keep("sum", false),
            Step::Keep("u", true),
            Step::Reset("x"),
            Step::Reset("sum"),
            Step::Reset("difference"),
            Step::Set("u", 150.0),
            Step::ResetModule,
        ];
        let (mut edit, mut held) = (Edit::default(), Held::default());
        // As `App::pend` shows each change once over the values as they are; and as it
        // did before, showing every pending change again over values that held them.
        let mut sent = report(&edit, &held);
        let (mut again, mut every) = (sent.clone(), Vec::new());
        let mut wrong = None;
        for (number, step) in (1..).zip(steps) {
            apply(&mut edit, &mut held, step);
            let expected = report(&edit, &held);
            let change = pending(step);
            show(&mut sent, &change, modules, &image);
            assert_eq!(sent.modules, expected.modules, "after {step:?}");
            every.push((number, change));
            again = reported(again, &mut every, modules, &image);
            if wrong.is_none() && again.modules != expected.modules {
                wrong = Some(step);
            }
        }
        // Showing them again starts the second step of the sum from where showing x
        // again moved the pair.
        assert_eq!(wrong, Some(Step::Set("sum", 111.0)));
    }

    /// White balance's temperature and tint read the camera's matrix (decision 27): a
    /// drag shows the coefficients the session will write, through the image context the
    /// session's `Photo` carries, not the sRGB matrix a photo's size alone would give.
    #[test]
    fn a_temperature_dragged_shows_the_coefficients_the_camera_s_matrix_gives() {
        use apertrust_engine::session::{ControlState, CoordinateState, ModuleState};
        use apertrust_modules::testing;
        use apertrust_modules::white_balance::{COEFFICIENTS, WhiteBalance};

        // The synthetic raw with a Canon EOS 40D's matrix, far from sRGB's.
        let mut image = testing::image();
        let matrix = [
            [6071.0, -747.0, -856.0],
            [-7653.0, 15365.0, 2441.0],
            [-2025.0, 2553.0, 7315.0],
        ];
        image.camera.as_mut().unwrap().xyz_to_camera =
            Some(matrix.map(|row: [f32; 3]| row.map(|v| v / 10000.0)));
        let photo = PhotoState::Open(Photo {
            path: PathBuf::from("40D.CR2"),
            open: 1,
            camera: String::new(),
            width: 84,
            height: 62,
            view_only: None,
            exposure_time: None,
            aperture: None,
            focal_length: None,
            image: Box::new(image.clone()),
        });
        assert_eq!(photo.image(), image);

        // White balance at its defaults, as the session reports it.
        let definition = WhiteBalance.definition();
        let defaults = WhiteBalance.defaults(&image);
        let view = &definition.views[0];
        let viewed = WhiteBalance.view(view.id, &defaults, &image).unwrap();
        let report = Values {
            modules: vec![ModuleState {
                module: definition.id,
                enabled: true,
                edited: false,
                controls: definition
                    .controls
                    .iter()
                    .map(|control| {
                        let value = defaults.get(control.id).unwrap().clone();
                        ControlState {
                            control: control.id,
                            value: value.clone(),
                            default: value,
                            mark: None,
                            available: true,
                            options: Vec::new(),
                            range: None,
                        }
                    })
                    .collect(),
                presets: Vec::new(),
                views: view
                    .coordinates
                    .iter()
                    .zip(&viewed)
                    .map(|(coordinate, &value)| CoordinateState {
                        view: view.id,
                        coordinate: coordinate.id,
                        value,
                        default: value,
                        mark: None,
                        available: true,
                    })
                    .collect(),
            }],
            settings: Vec::new(),
            edited: false,
            through: 0,
            extent: [84, 62],
            live: true,
        };
        let modules: &[&dyn Module] = &[&WhiteBalance];
        let drag = Pending::Control {
            module: definition.id,
            control: "temperature",
            value: ControlValue::Number(5000.0),
        };
        let shown = |image: &ImageContext| {
            let mut values = report.clone();
            show(&mut values, &drag, modules, image);
            let controls = &values.modules[0].controls;
            COEFFICIENTS.map(|id| {
                let shown = controls.iter().find(|c| c.control == id).unwrap();
                (shown.value.clone(), shown.mark)
            })
        };
        let expected = WhiteBalance
            .unview(view.id, &[5000.0, viewed[1]], &image)
            .unwrap();
        let through_the_camera = shown(&photo.image());
        for (id, (value, mark)) in COEFFICIENTS.iter().zip(&through_the_camera) {
            assert_eq!((Some(value), *mark), (expected.get(id), Some(Mark::Kept)));
        }
        // The photo's size alone reads the coefficients through sRGB's matrix instead.
        assert_ne!(through_the_camera, shown(&ImageContext::full(84, 62)));
    }

    /// What Settings' window hands the app, the setting `id` set to `value`, kept by
    /// `keeper` and shown by `shell` in `ctx`'s window, as `App::logic` takes it.
    fn set_in_settings(
        shell: &mut Shell,
        keeper: &mut Keeper,
        id: &'static str,
        value: impl Into<toml::Value>,
        ctx: &egui::Context,
    ) {
        let change = settings_dialog::Change::Set(id, value.into());
        apply_settings(&[change], shell, keeper, ctx);
    }

    /// What Settings changes reaches the shell, which keeps it, and the main window, which
    /// shows it at once; the dialog's own drawing is the dialog's (`settings_dialog::tests`)
    /// and the host's (`dialog_host::tests`), on every platform.
    #[test]
    fn settings_change_the_theme_and_the_language_at_once() {
        let (mut keeper, settings) = Keeper::nowhere();
        let mut shell = Shell::new(&settings, Locale::EnGb, OperatingSystem::Windows);
        let ctx = egui::Context::default();
        apertrust_ui::install(&ctx, Theme::default());
        set_in_settings(&mut shell, &mut keeper, "theme", "light", &ctx);
        assert_eq!(shell.theme, Builtin::Light);
        assert_eq!(theme(&ctx).colors, Theme::builtin(Builtin::Light).colors);
        set_in_settings(&mut shell, &mut keeper, "locale", "uk", &ctx);
        assert_eq!(
            (shell.locale, shell.kept_locale),
            (Locale::Uk, Some(Locale::Uk))
        );
        assert_eq!(apertrust_ui::translator(&ctx).locale(), Locale::Uk);
        // Reset, the language follows the system's again.
        let reset = settings_dialog::Change::Reset("locale");
        apply_settings(&[reset], &mut shell, &mut keeper, &ctx);
        assert_eq!((shell.locale, shell.kept_locale), (Locale::EnGb, None));
        // The dialog it opens shows what is kept.
        let view = settings_dialog::Settings::new(&keeper, true);
        let theme = view.row("theme").expect("the theme's row");
        assert_eq!(theme.value, Some("light".into()));
        assert!(theme.changed());
    }

    /// A row `--conf` sets for this run is handed to the dialog as overridden, which draws
    /// it disabled, saying why (decision 43; drawn in `settings_dialog::tests`), and the
    /// run keeps the value it gives.
    #[test]
    fn settings_set_by_conf_are_drawn_disabled_saying_why() {
        let overrides = settings::overrides(&["theme=light".to_owned(), "locale=pl".to_owned()]);
        let (keeper, settings) = Keeper::open_in(None, &overrides.expect("read"));
        let shell = Shell::new(&settings, Locale::EnGb, OperatingSystem::Windows);
        assert_eq!((shell.theme, shell.locale), (Builtin::Light, Locale::Pl));
        let view = settings_dialog::Settings::new(&keeper, true);
        let overridden: Vec<&str> = view
            .rows
            .iter()
            .filter(|row| row.overridden)
            .map(|row| row.id)
            .collect();
        assert_eq!(overridden, ["theme", "locale"]);
        // In each language, with the flag as it is typed.
        for (locale, words) in [
            (Locale::Uk, "Задано через --conf для цього запуску"),
            (Locale::Pl, "Ustawione przez --conf na to uruchomienie"),
        ] {
            let mut args = apertrust_ui::i18n::Args::new();
            args.set("flag", CONF_FLAG);
            let said = Translator::new(locale).text_with("settings-overridden", Some(&args));
            assert_eq!(said, words);
        }
    }

    fn read(line: &[&str]) -> Result<Arguments, String> {
        arguments(line.iter().map(OsString::from), on_disk)
    }

    /// What the disk would say of a path, by its name, as `dropped`'s tests have it: a
    /// folder where it has no dot; nothing there where it names `missing`.
    fn on_disk(path: &Path) -> std::io::Result<dropped::Kind> {
        if path.to_string_lossy().contains("missing") {
            return Err(std::io::ErrorKind::NotFound.into());
        }
        Ok(dropped::tests::by_name(path))
    }

    /// `photo.ARW`, as the command line opens it alone: from the folder the app starts in.
    fn photo_arw() -> Option<Start> {
        let path = std::path::absolute("photo.ARW").expect("absolute");
        Some(Start::Photo(path))
    }

    /// The path, a raw opening alone and a folder in folder mode, both absolute; a
    /// library's or a project's file, another file and nothing there stop the start, saying
    /// why; the other flags read as ever around it, and only the first path counts.
    #[test]
    fn the_command_line_s_path_opens_a_photo_alone_or_a_folder() {
        let absolute = |path: &str| std::path::absolute(path).expect("absolute");
        assert_eq!(read(&["photo.ARW"]).expect("read").open, photo_arw());
        let folder = read(&["--conf", "scale=2", "Trips/Lisbon", "-d", "settings"]);
        let folder = folder.expect("read");
        assert_eq!(folder.open, Some(Start::Folder(absolute("Trips/Lisbon"))));
        assert_eq!(folder.conf, ["scale=2"]);
        assert_eq!(folder.domains, ["settings"]);
        let given = read(&["/photos/IMG_1.CR3", "--cache-dir=one", "/photos/IMG_2.CR3"]);
        let given = given.expect("read");
        let first = Some(Start::Photo(PathBuf::from("/photos/IMG_1.CR3")));
        assert_eq!(given.open, first);
        assert!(given.cache.is_some());
        assert_eq!(read(&["-d", "gpu"]).expect("read").open, None);
        for (line, error) in [
            (
                &["Home.aplibrary"][..],
                "Home.aplibrary: libraries and projects come in a later version",
            ),
            (
                &["--configdir", "dev", "Trip.approject"],
                "Trip.approject: libraries and projects come in a later version",
            ),
            (
                &["notes.txt"],
                "notes.txt: only folders and raw photos open",
            ),
        ] {
            assert_eq!(read(line), Err(error.to_owned()), "{line:?}");
        }
        let missing = read(&["missing.CR3"]).expect_err("nothing there");
        assert!(missing.starts_with("missing.CR3: "), "{missing}");
        // On the disk: a folder, a raw, and nothing there.
        let scratch = settings::testing::Folder::new("command-line");
        let raw = scratch.0.join("IMG_1.CR3");
        std::fs::write(&raw, b"").expect("a raw");
        let line = |path: &Path| [OsString::from(path)];
        let found = |path: &Path| arguments(line(path), dropped::found).map(|a| a.open);
        // Resolved where the system resolves them; on Windows only made absolute.
        let resolved = |path: &Path| {
            if cfg!(windows) {
                path.to_owned()
            } else {
                std::fs::canonicalize(path).expect("there")
            }
        };
        let folder = resolved(&scratch.0);
        assert_eq!(found(&scratch.0), Ok(Some(Start::Folder(folder.clone()))));
        let photo = Some(Start::Photo(folder.join("IMG_1.CR3")));
        assert_eq!(found(&raw), Ok(photo.clone()));
        assert!(found(&scratch.0.join("IMG_2.CR3")).is_err());
        // `.` and `..` lead to the folder they name, the photo in it; on Windows making
        // the path absolute resolves them.
        let days = scratch.0.join("Days");
        std::fs::create_dir(&days).expect("a folder");
        for dots in [scratch.0.join("."), days.join("..")] {
            let given = Ok(Some(Start::Folder(folder.clone())));
            assert_eq!(found(&dots), given, "{}", dots.display());
        }
        assert_eq!(found(&days.join("..").join("IMG_1.CR3")), Ok(photo));
    }

    /// `--in-folder FILE...`: every path given, absolute, wherever the flag comes, the other
    /// flags read as ever and the disk never asked here, so a library's file or a missing
    /// one is refused later, in the window; with none, as a desktop entry's `%F` started
    /// from a menu gives, nothing opens.
    #[test]
    fn in_folder_takes_every_file_given_and_asks_nothing_of_the_disk() {
        let unasked = |path: &Path| -> std::io::Result<dropped::Kind> {
            panic!("the disk asked of {}", path.display())
        };
        let read = |line: &[&str]| arguments(line.iter().map(OsString::from), unasked);
        let absolute = |path: &str| std::path::absolute(path).expect("absolute");
        let line = [
            "IMG_1.CR3",
            "--conf",
            "scale=2",
            "--in-folder",
            "/a/IMG_2.CR3",
        ];
        let given = read(&line).expect("read");
        assert_eq!(
            given.in_folder,
            [absolute("IMG_1.CR3"), absolute("/a/IMG_2.CR3")]
        );
        assert_eq!((given.open, given.conf), (None, vec!["scale=2".to_owned()]));
        let refused_later = read(&["--in-folder", "/a/Home.aplibrary", "/nowhere/x.CR3"]);
        assert_eq!(refused_later.expect("read").in_folder.len(), 2);
        assert_eq!(read(&["--in-folder"]), Ok(Arguments::default()));
    }

    /// `--safe-mode` and the hidden `--reopen-last` are read anywhere; the flags a restart
    /// starts with again are the rest but what was opened, each with its value as given, a
    /// folder's made absolute but for a leading `~`. `--help` asks for the usage, which
    /// names `--safe-mode`.
    #[test]
    fn a_restart_keeps_the_flags_but_what_was_opened() {
        let absolute = |path: &str| OsString::from(std::path::absolute(path).expect("absolute"));
        let line = [
            "-d",
            "gpu",
            "--safe-mode",
            "--conf",
            "scale=2",
            "--conf=log.keep=true",
            "--user-data-dir",
            "dev",
            "--cache-dir=~/cache",
            "photo.ARW",
            "--reopen-last",
        ];
        let given = read(&line).expect("read");
        assert!(given.safe_mode);
        assert_eq!(given.reopen, Some(true));
        assert_eq!(given.open, photo_arw());
        let kept: Vec<OsString> = vec![
            "-d".into(),
            "gpu".into(),
            "--conf".into(),
            "scale=2".into(),
            "--conf=log.keep=true".into(),
            "--user-data-dir".into(),
            absolute("dev"),
            "--cache-dir".into(),
            "~/cache".into(),
        ];
        assert_eq!(given.flags, kept);
        let unasked = |path: &Path| -> std::io::Result<dropped::Kind> {
            panic!("the disk asked of {}", path.display())
        };
        let line = ["--in-folder", "/a/IMG_2.CR3", "--debug", "settings"];
        let given = arguments(line.map(OsString::from), unasked).expect("read");
        let debug: Vec<OsString> = vec!["--debug".into(), "settings".into()];
        assert_eq!(
            (given.flags, given.safe_mode, given.reopen),
            (debug, false, None)
        );
        let asks = |line: &[&str]| help_asked(line.iter().map(OsString::from));
        assert!(asks(&["-d", "gpu", "--help"]) && asks(&["-h"]));
        assert!(!asks(&["-d", "gpu", "photo.ARW"]));
        assert!(USAGE.contains(restart::SAFE_MODE_FLAG));
        assert!(!USAGE.contains(restart::REOPEN_FLAG));
        assert_eq!(
            read(&["--no-reopen-last"]).expect("read").reopen,
            Some(false)
        );
    }

    /// What Open With gave the app opens as a drop of it would, once looked at: a raw as
    /// Open… opens it, a library's file refused with the alert in the window. Unlike a
    /// drop, it waits for a modal dialog to go, and then opens.
    #[test]
    fn open_with_s_files_open_as_a_drop_and_wait_for_a_modal_dialog() {
        let folder = settings::testing::Folder::new("open-with");
        let (raw, library) = (folder.0.join("IMG_1.CR3"), folder.0.join("Home.aplibrary"));
        for file in [&raw, &library] {
            std::fs::write(file, b"").expect("a file");
        }
        let mut harness = bench(OperatingSystem::Windows);
        harness.run();
        let opened = |harness: &mut Harness<'_, Bench>, paths: &[&PathBuf]| {
            let ctx = harness.ctx.clone();
            let paths = paths.iter().map(|p| p.to_path_buf()).collect();
            harness.state_mut().opened.look(paths, &ctx);
            harness.state_mut().opened.wait();
            harness.run();
        };
        opened(&mut harness, &[&library]);
        let why = dropped::Refusal::Library;
        assert_eq!(harness.state().refused, [(library.clone(), why)]);
        harness.state_mut().alerts = alerts::Alerts::default();
        harness.run();
        // About up: the raw waits, then opens once About has gone.
        let ctx = harness.ctx.clone();
        harness.state_mut().shell.about.open(&ctx);
        harness.run();
        opened(&mut harness, &[&raw]);
        assert!(harness.state().commands.is_empty());
        harness.state_mut().shell.about = about::About::default();
        harness.run();
        assert_eq!(harness.state().commands, [Command::Open(raw)]);
    }

    /// A photo that is a link opens under its own name, in the folder where the link lies,
    /// never as the file it points to.
    #[cfg(unix)]
    #[test]
    fn a_photo_that_is_a_link_keeps_its_name_and_its_folder() {
        let scratch = settings::testing::Folder::new("command-line-link");
        let (photos, elsewhere) = (scratch.0.join("Photos"), scratch.0.join("Elsewhere"));
        for folder in [&photos, &elsewhere] {
            std::fs::create_dir(folder).expect("a folder");
        }
        let raw = elsewhere.join("IMG_1.CR3");
        std::fs::write(&raw, b"").expect("a raw");
        let link = photos.join("Best.CR3");
        std::os::unix::fs::symlink(&raw, &link).expect("a link");
        let open = arguments([OsString::from(&link)], dropped::found).expect("read");
        let folder = std::fs::canonicalize(&photos).expect("there");
        assert_eq!(open.open, Some(Start::Photo(folder.join("Best.CR3"))));
    }

    /// What `open`, the command line's, sends as the app starts, `App::new` needing a
    /// session and a GPU: the commands, numbered from 1.
    fn started_with(open: Start, mirror: &mut Mirror) -> Vec<Command> {
        let mut sent = Vec::new();
        started(open, mirror, &mut |command| {
            sent.push(command);
            sent.len() as u64
        });
        sent
    }

    /// `apertrust FILE`: the photo alone is the one command, no library command is sent,
    /// so nothing is listed, and nothing of the photo's own events reaches the recents,
    /// which the folder's opening alone adds to (`Reaction::FolderOpened`); the recents'
    /// file stays as it was.
    #[test]
    fn a_photo_alone_sends_no_library_command_and_leaves_the_recents_untouched() {
        let scratch = settings::testing::Folder::new("alone-recents");
        let file = scratch.0.join("state.toml");
        let (mut recents, _) = Recents::open(file.clone(), Vec::new);
        recents.flush();
        let before = std::fs::read(&file).ok();
        let mut m = mirror::tests::mirror();
        let path = PathBuf::from("/photos/a.ARW");
        let sent = started_with(Start::Photo(path.clone()), &mut m);
        assert_eq!(sent, [Command::Open(path)]);
        let mut reactions = m.apply(Event::Opened(mirror::tests::photo(1)));
        reactions.extend(m.apply(Event::Values(mirror::tests::report(1.0, 1))));
        for reaction in reactions {
            if let Reaction::FolderOpened(path) = reaction {
                recents.opened(recents::OpenKind::Folder, path);
                recents.save();
            }
        }
        recents.flush();
        assert_eq!(recents.entries().count(), 0);
        assert_eq!(std::fs::read(&file).ok(), before);
        // `apertrust FOLDER` lists the folder, and opens no photo.
        let folder = PathBuf::from("/photos");
        let sent = started_with(Start::Folder(folder.clone()), &mut mirror::tests::mirror());
        let source = Source::Folder(folder);
        assert_eq!(sent, [Command::Library(LibraryCommand::Open(source))]);
    }

    /// A list of the commands sent, numbered from 1, and what sends to it.
    fn sender(sent: &mut Vec<Command>) -> impl FnMut(Command) -> u64 + '_ {
        |command| {
            sent.push(command);
            sent.len() as u64
        }
    }

    fn listed(open: u64, names: &[&str], done: bool) -> Event {
        let cell = |name: &&str| apertrust_engine::session::library::Photo {
            name: (*name).to_owned(),
            size: 1,
            modified: None,
            edit: EditState::None,
        };
        let photos = names.iter().map(cell).collect();
        Event::Library(LibraryEvent::Listed { open, photos, done })
    }

    fn folder_opened(open: u64, folder: &Path) -> Event {
        Event::Library(LibraryEvent::Opened {
            open,
            name: folder.file_name().unwrap().to_string_lossy().into_owned(),
            folder: folder.to_owned(),
        })
    }

    /// File › Close: the photo closed through the session, which saves its edit, and the
    /// folder closed, so nothing is open and the start window shows in place of the main
    /// window; what the photo closed still sends is dropped. With nothing open, nothing is
    /// sent.
    #[test]
    fn close_closes_the_photo_and_the_folder_and_shows_the_start_window() {
        let mut m = mirror::tests::mirror();
        let mut sent = Vec::new();
        let folder = PathBuf::from("/photos");
        open_folder(folder.clone(), &mut m, &mut sender(&mut sent));
        open_photo(
            folder.join("a.ARW"),
            &mut Remembered::default(),
            &mut m,
            &mut sender(&mut sent),
        );
        m.apply(Event::Opened(mirror::tests::photo(2)));
        assert!(!shows_start(opened(&m), false, false, false));
        sent.clear();
        close(&mut Remembered::default(), &mut m, &mut sender(&mut sent));
        assert_eq!(
            sent,
            [Command::Close, Command::Library(LibraryCommand::Close)]
        );
        assert!(shows_start(opened(&m), false, false, false));
        assert!(matches!(m.photo, PhotoState::None));
        assert_eq!(m.folder.folder, folder::Folder::None);
        // Browse's tree has no root after it.
        assert_eq!(m.folder.root(), None);
        assert_eq!(library::Empty::of(&m.folder.folder), library::Empty::Start);
        m.apply(Event::Values(mirror::tests::report(1.0, 3)));
        assert_eq!(m.shown(), Shown::Nothing);
        assert!(matches!(m.photo, PhotoState::None));
        // Another photo opened takes its events again.
        let mut more = Vec::new();
        open_photo(
            folder.join("a.ARW"),
            &mut Remembered::default(),
            &mut m,
            &mut sender(&mut more),
        );
        m.apply(Event::Opened(mirror::tests::photo(1)));
        assert!(matches!(m.photo, PhotoState::Open(_)));
        // Nothing open: nothing to close.
        let mut nothing = mirror::tests::mirror();
        let mut none = Vec::new();
        close(
            &mut Remembered::default(),
            &mut nothing,
            &mut sender(&mut none),
        );
        assert!(none.is_empty());
    }

    /// The folder `AT` open with `R0001176.DNG` active, opening as a click in the grid
    /// panel opens it, its subfolders `New 1`, empty, and `Day 2`.
    fn at_with_its_photo() -> (Mirror, Vec<Command>, Remembered, PathBuf) {
        let (mut m, mut sent) = (mirror::tests::mirror(), Vec::new());
        let at = PathBuf::from("/photos/AT");
        open_folder(at.clone(), &mut m, &mut sender(&mut sent));
        m.apply(folder_opened(1, &at));
        m.apply(listed(1, &["R0001175.DNG", "R0001176.DNG"], true));
        open_photo(
            at.join("R0001176.DNG"),
            &mut Remembered::default(),
            &mut m,
            &mut sender(&mut sent),
        );
        let mut remembered = Remembered::default();
        remembered.remember(&m);
        (m, sent, remembered, at)
    }

    /// The photo the central panel shows, open or opening.
    fn photo_shown(m: &Mirror) -> Option<&Path> {
        m.photo.asked()
    }

    /// Choosing another folder in Browse's tree closes the photo of the folder left,
    /// through the session's `Close` alone, the folder staying: an empty subfolder, or one
    /// with photos and none remembered, shows no photo, the editing tabs as with none
    /// active, and what was open last keeps the folder with no photo.
    #[test]
    fn a_folder_chosen_in_the_tree_shows_no_photo_of_the_folder_left() {
        let (mut m, mut sent, mut remembered, at) = at_with_its_photo();
        let new = at.join("New 1");
        change_folder(
            new.clone(),
            FolderChange::Choose,
            None,
            &mut remembered,
            &mut m,
            &mut sender(&mut sent),
        );
        let chosen = Command::Library(LibraryCommand::Open(Source::Folder(new.clone())));
        assert_eq!(sent[2..], [Command::Close, chosen]);
        assert_eq!(photo_shown(&m), None);
        assert_eq!(m.shown(), Shown::Nothing);
        m.apply(folder_opened(4, &new));
        m.apply(listed(4, &[], true));
        remembered.restore(&mut m, &mut sender(&mut sent));
        assert_eq!(sent.len(), 4);
        assert_eq!(
            library::Empty::of(&m.folder.folder),
            library::Empty::NoRaws { subfolders: false }
        );
        assert_eq!(m.folder.root(), Some(at.as_path()));
        let kept = recents::Last {
            kind: recents::OpenKind::Folder,
            path: at.clone(),
            folder: None,
            photo: Some("R0001176.DNG".to_owned()),
        };
        let left = at_exit(kept.clone(), None, &m);
        assert_eq!((left.folder, left.photo), (Some(new.clone()), None));

        // One with photos and none remembered: none shown, and none opened once listed.
        let day = at.join("Day 2");
        change_folder(
            day.clone(),
            FolderChange::Choose,
            None,
            &mut remembered,
            &mut m,
            &mut sender(&mut sent),
        );
        assert_eq!(sent.len(), 5, "nothing to close: {sent:?}");
        m.apply(folder_opened(5, &day));
        m.apply(listed(5, &["a.DNG", "b.DNG"], true));
        remembered.restore(&mut m, &mut sender(&mut sent));
        assert_eq!(sent.len(), 5);
        assert_eq!(photo_shown(&m), None);
        assert_eq!(
            library::Empty::of(&m.folder.folder),
            library::Empty::NoneChosen
        );
    }

    /// Going back to a folder opens the photo last active in it this run, as a click in the
    /// grid panel does, once its listing holds it and not before; one gone from it shows
    /// none. What was open last then agrees.
    #[test]
    fn a_folder_chosen_again_reopens_its_photo_once_listed() {
        let (mut m, mut sent, mut remembered, at) = at_with_its_photo();
        let new = at.join("New 1");
        change_folder(
            new.clone(),
            FolderChange::Choose,
            None,
            &mut remembered,
            &mut m,
            &mut sender(&mut sent),
        );
        m.apply(folder_opened(4, &new));
        m.apply(listed(4, &[], true));
        remembered.restore(&mut m, &mut sender(&mut sent));

        // Back to AT: nothing opens while it is read, nor before its listing holds the
        // photo; then it opens.
        change_folder(
            at.clone(),
            FolderChange::Choose,
            None,
            &mut remembered,
            &mut m,
            &mut sender(&mut sent),
        );
        assert_eq!(sent.len(), 5);
        // Quitting meanwhile keeps the photo waited for, not the folder without it.
        let new_kept = recents::Last {
            kind: recents::OpenKind::Folder,
            path: at.clone(),
            folder: Some(new.clone()),
            photo: None,
        };
        let waits = at_exit(new_kept, remembered.waiting.as_ref(), &m);
        assert_eq!(
            (waits.folder, waits.photo.as_deref()),
            (None, Some("R0001176.DNG"))
        );
        remembered.restore(&mut m, &mut sender(&mut sent));
        m.apply(folder_opened(5, &at));
        m.apply(listed(5, &["R0001175.DNG"], false));
        remembered.restore(&mut m, &mut sender(&mut sent));
        assert_eq!(sent.len(), 5);
        assert_eq!(photo_shown(&m), None);
        m.apply(listed(5, &["R0001176.DNG"], true));
        remembered.restore(&mut m, &mut sender(&mut sent));
        let photo = at.join("R0001176.DNG");
        assert_eq!(sent[5..], [Command::Open(photo.clone())]);
        assert_eq!(photo_shown(&m), Some(photo.as_path()));
        let kept = recents::Last {
            kind: recents::OpenKind::Folder,
            path: at.clone(),
            folder: Some(new.clone()),
            photo: None,
        };
        let back = at_exit(kept, None, &m);
        assert_eq!(
            (back.folder, back.photo.as_deref()),
            (None, Some("R0001176.DNG"))
        );

        // The photo gone from AT by the next time: none shown.
        change_folder(
            new.clone(),
            FolderChange::Choose,
            None,
            &mut remembered,
            &mut m,
            &mut sender(&mut sent),
        );
        m.apply(folder_opened(8, &new));
        m.apply(listed(8, &[], true));
        change_folder(
            at.clone(),
            FolderChange::Choose,
            None,
            &mut remembered,
            &mut m,
            &mut sender(&mut sent),
        );
        m.apply(folder_opened(9, &at));
        m.apply(listed(9, &["R0001175.DNG"], true));
        remembered.restore(&mut m, &mut sender(&mut sent));
        assert_eq!(sent.len(), 9, "{sent:?}");
        assert_eq!(photo_shown(&m), None);
        assert_eq!(remembered.waiting, None);
    }

    /// [`at_with_its_photo`], then `New 1` chosen and listed, then AT chosen again, its
    /// photo waited for, AT's open numbered 5.
    fn waiting_for_at() -> (Mirror, Vec<Command>, Remembered, PathBuf) {
        let (mut m, mut sent, mut remembered, at) = at_with_its_photo();
        for folder in [at.join("New 1"), at.clone()] {
            change_folder(
                folder,
                FolderChange::Choose,
                None,
                &mut remembered,
                &mut m,
                &mut sender(&mut sent),
            );
            if sent.len() == 4 {
                m.apply(folder_opened(4, &at.join("New 1")));
                m.apply(listed(4, &[], true));
                remembered.restore(&mut m, &mut sender(&mut sent));
            }
        }
        assert_eq!(sent.len(), 5);
        assert!(remembered.waiting.is_some());
        (m, sent, remembered, at)
    }

    /// The photo waited for gives way: to a photo asked for meanwhile, to another folder
    /// chosen before its listing, whose stale listing then opens nothing, and to a folder
    /// that could not be read.
    #[test]
    fn a_photo_waited_for_gives_way_to_what_the_person_asks_and_to_a_failure() {
        // A cell clicked before the listing holds it.
        let (mut m, mut sent, mut remembered, at) = waiting_for_at();
        let clicked = at.join("R0001175.DNG");
        open_photo(
            clicked.clone(),
            &mut remembered,
            &mut m,
            &mut sender(&mut sent),
        );
        assert_eq!(sent[5..], [Command::Open(clicked.clone())]);
        m.apply(folder_opened(5, &at));
        m.apply(listed(5, &["R0001175.DNG", "R0001176.DNG"], true));
        remembered.restore(&mut m, &mut sender(&mut sent));
        assert_eq!(sent.len(), 6, "{sent:?}");
        assert_eq!(photo_shown(&m), Some(clicked.as_path()));

        // Another folder chosen before AT's listing comes: its listing then is stale.
        let (mut m, mut sent, mut remembered, at) = waiting_for_at();
        let new = at.join("New 1");
        let choose = FolderChange::Choose;
        change_folder(
            new.clone(),
            choose,
            None,
            &mut remembered,
            &mut m,
            &mut sender(&mut sent),
        );
        assert_eq!(remembered.waiting, None);
        m.apply(folder_opened(5, &at));
        m.apply(listed(5, &["R0001176.DNG"], true));
        remembered.restore(&mut m, &mut sender(&mut sent));
        m.apply(folder_opened(6, &new));
        m.apply(listed(6, &[], true));
        remembered.restore(&mut m, &mut sender(&mut sent));
        assert_eq!(sent.len(), 6, "{sent:?}");
        assert_eq!(photo_shown(&m), None);

        // AT could not be read.
        let (mut m, mut sent, mut remembered, _) = waiting_for_at();
        m.apply(Event::Library(LibraryEvent::Failed {
            open: 5,
            reason: "No such folder".to_owned(),
        }));
        remembered.restore(&mut m, &mut sender(&mut sent));
        assert_eq!((sent.len(), remembered.waiting), (5, None));
    }

    /// What the session still sends of the photo closed as the folder changed is dropped:
    /// the photo stays closed, and the wait for the next still opens its photo.
    #[test]
    fn the_closed_photo_s_late_events_change_nothing() {
        let (mut m, mut sent, mut remembered, at) = at_with_its_photo();
        let new = at.join("New 1");
        let choose = FolderChange::Choose;
        change_folder(
            new.clone(),
            choose,
            None,
            &mut remembered,
            &mut m,
            &mut sender(&mut sent),
        );
        // The photo was opened by command 2.
        m.apply(Event::Opened(mirror::tests::photo(2)));
        m.apply(Event::Values(mirror::tests::report(1.0, 2)));
        assert_eq!(photo_shown(&m), None);
        assert_eq!(m.shown(), Shown::Nothing);
        m.apply(folder_opened(4, &new));
        m.apply(listed(4, &[], true));
        change_folder(
            at.clone(),
            choose,
            None,
            &mut remembered,
            &mut m,
            &mut sender(&mut sent),
        );
        m.apply(folder_opened(5, &at));
        m.apply(listed(5, &["R0001176.DNG"], true));
        remembered.restore(&mut m, &mut sender(&mut sent));
        let photo = at.join("R0001176.DNG");
        assert_eq!(sent[5..], [Command::Open(photo.clone())]);
        assert_eq!(photo_shown(&m), Some(photo.as_path()));
    }

    /// Open This Folder from a photo alone keeps the photo, which is that folder's; and a
    /// folder named by a relative path is the photo's absolute folder.
    #[test]
    fn the_photo_s_own_folder_keeps_it_whatever_its_path_is_written_as() {
        let mut m = mirror::tests::mirror();
        started_with(Start::Photo(PathBuf::from("/photos/a.ARW")), &mut m);
        let (mut sent, mut remembered) = (Vec::new(), Remembered::default());
        let folder = PathBuf::from("/photos");
        let way_in = FolderChange::WayIn;
        change_folder(
            folder.clone(),
            way_in,
            None,
            &mut remembered,
            &mut m,
            &mut sender(&mut sent),
        );
        assert_eq!(
            sent,
            [Command::Library(LibraryCommand::Open(Source::Folder(
                folder
            )))]
        );
        assert_eq!(photo_shown(&m), Some(Path::new("/photos/a.ARW")));
        assert_eq!(remembered.waiting, None);

        let relative = PathBuf::from("relative-photos");
        let absolute = std::path::absolute(&relative).unwrap();
        let (mut m, mut sent) = (mirror::tests::mirror(), Vec::new());
        open_folder(relative.clone(), &mut m, &mut sender(&mut sent));
        m.apply(folder_opened(1, &absolute));
        m.apply(listed(1, &["x.ARW"], true));
        let photo = absolute.join("x.ARW");
        open_photo(
            photo.clone(),
            &mut remembered,
            &mut m,
            &mut sender(&mut sent),
        );
        let choose = FolderChange::Choose;
        change_folder(
            relative,
            choose,
            None,
            &mut remembered,
            &mut m,
            &mut sender(&mut sent),
        );
        assert!(!sent.contains(&Command::Close), "{sent:?}");
        assert_eq!(photo_shown(&m), Some(photo.as_path()));
    }

    /// File › Close forgets the photos remembered this run, as it forgets what was open
    /// last: the folder opened again waits for none.
    #[test]
    fn close_forgets_the_photos_remembered_this_run() {
        let (mut m, mut sent, mut remembered, at) = waiting_for_at();
        close(&mut remembered, &mut m, &mut sender(&mut sent));
        assert!(remembered.photos.is_empty());
        assert_eq!(remembered.waiting, None);
        let way_in = FolderChange::WayIn;
        change_folder(
            at,
            way_in,
            None,
            &mut remembered,
            &mut m,
            &mut sender(&mut sent),
        );
        assert_eq!(remembered.waiting, None);
    }

    /// The folder's reading cancelled from the jobs window while its photo waits ends the
    /// wait: nothing opens.
    #[test]
    fn a_reading_cancelled_while_its_photo_waits_ends_the_wait() {
        use apertrust_engine::jobs::{Message, Progress, Queue, State};
        let (mut m, mut sent, mut remembered, _) = waiting_for_at();
        let job = crate::jobs::tests::ids(1)[0];
        let added = apertrust_engine::session::JobEvent::Added {
            job,
            kind: "folder",
            queue: Some(Queue::UserBackground),
            title: Message::new("job-folder"),
            cancellable: true,
            destination: None,
            detail: None,
            progress: Progress::default(),
            retries: None,
        };
        let cancelled = apertrust_engine::session::JobEvent::Ended {
            job,
            state: State::Cancelled,
            outcome: None,
            failure: None,
            at: std::time::SystemTime::UNIX_EPOCH,
            offers: Vec::new(),
        };
        m.apply(Event::Job {
            event: added,
            command: Some(5),
        });
        m.apply(Event::Job {
            event: cancelled,
            command: None,
        });
        remembered.restore(&mut m, &mut sender(&mut sent));
        assert_eq!((sent.len(), remembered.waiting), (5, None));
        assert_eq!(photo_shown(&m), None);
    }

    /// A way in, as Open Folder…, Open Recent, Browse's recents or a dropped folder open
    /// one, follows the same rule as the tree: the photo of the folder left closes, and the
    /// photo last active in the folder opened this run opens once it is listed.
    #[test]
    fn a_folder_opened_as_a_way_in_shows_its_own_photo_or_none() {
        let (mut m, mut sent, mut remembered, at) = at_with_its_photo();
        // Open Folder… of another folder.
        let other = PathBuf::from("/photos/Other");
        let way_in = FolderChange::WayIn;
        change_folder(
            other.clone(),
            way_in,
            None,
            &mut remembered,
            &mut m,
            &mut sender(&mut sent),
        );
        let opened = Command::Library(LibraryCommand::Open(Source::Folder(other.clone())));
        assert_eq!(sent[2..], [Command::Close, opened]);
        assert_eq!(photo_shown(&m), None);
        assert_eq!(m.folder.root(), Some(other.as_path()));
        m.apply(folder_opened(4, &other));
        m.apply(listed(4, &["x.DNG"], true));
        remembered.restore(&mut m, &mut sender(&mut sent));
        assert_eq!((sent.len(), photo_shown(&m)), (4, None));
        // AT again from a recent: its photo opens once listed, the folder a way in.
        change_folder(
            at.clone(),
            way_in,
            None,
            &mut remembered,
            &mut m,
            &mut sender(&mut sent),
        );
        assert_eq!(sent.len(), 5, "nothing to close: {sent:?}");
        m.apply(folder_opened(5, &at));
        m.apply(listed(5, &["R0001175.DNG", "R0001176.DNG"], true));
        remembered.restore(&mut m, &mut sender(&mut sent));
        let photo = at.join("R0001176.DNG");
        assert_eq!(sent[5..], [Command::Open(photo.clone())]);
        assert_eq!(photo_shown(&m), Some(photo.as_path()));
        assert_eq!(m.folder.root(), Some(at.as_path()));
        // Open This Folder, or the folder opened again: its own photo stays as it is.
        change_folder(
            at.clone(),
            way_in,
            None,
            &mut remembered,
            &mut m,
            &mut sender(&mut sent),
        );
        assert_eq!(sent.len(), 7, "nothing closed: {sent:?}");
        assert_eq!(photo_shown(&m), Some(photo.as_path()));
        assert_eq!(remembered.waiting, None);
    }

    /// A recent not opened this run waits for the photo what was open last records for it,
    /// where that is the folder opened; a photo it records in a subfolder is not that
    /// folder's, and none is shown.
    #[test]
    fn a_recent_not_opened_this_run_shows_the_photo_kept_for_it() {
        let (mut m, mut sent, mut remembered, _) = at_with_its_photo();
        let kept = last(Some("b.ARW"));
        let way_in = FolderChange::WayIn;
        change_folder(
            kept.path.clone(),
            way_in,
            Some(&kept),
            &mut remembered,
            &mut m,
            &mut sender(&mut sent),
        );
        assert_eq!(sent[2], Command::Close);
        m.apply(folder_opened(4, &kept.path));
        m.apply(listed(4, &["a.ARW", "b.ARW"], true));
        remembered.restore(&mut m, &mut sender(&mut sent));
        assert_eq!(sent[4..], [Command::Open(kept.path.join("b.ARW"))]);

        let (mut m, mut sent, mut remembered, _) = at_with_its_photo();
        let in_day = kept_last(Some("Day 1"), Some("b.ARW"));
        let path = in_day.path.clone();
        change_folder(
            path,
            way_in,
            Some(&in_day),
            &mut remembered,
            &mut m,
            &mut sender(&mut sent),
        );
        assert_eq!(remembered.waiting, None);
        assert_eq!(photo_shown(&m), None);
    }

    /// File › Open… of a raw in the open root's tree keeps the root and the recents: its
    /// folder is chosen there, as a click in the tree chooses it, and the raw opens; the
    /// root's own raw chooses the root. A raw elsewhere opens its folder as a way in, the
    /// tree's root and a recent.
    #[test]
    fn a_raw_opened_inside_the_root_s_tree_chooses_its_folder_there() {
        let (mut m, mut sent, mut remembered, at) = at_with_its_photo();
        let joins = |reactions: Vec<Reaction>| {
            reactions
                .into_iter()
                .filter_map(|reaction| match reaction {
                    Reaction::FolderOpened(path) => Some(path),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        // In a subfolder of the root.
        let other = at.join("Other");
        let raw = other.join("x.dng");
        open_photo(raw.clone(), &mut remembered, &mut m, &mut sender(&mut sent));
        let chosen = Command::Library(LibraryCommand::Open(Source::Folder(other.clone())));
        assert_eq!(sent[2..], [Command::Open(raw.clone()), chosen]);
        assert_eq!(m.folder.root(), Some(at.as_path()));
        assert_eq!(m.folder.shown(), Some(other.as_path()));
        assert!(joins(m.apply(folder_opened(4, &other))).is_empty());
        assert_eq!(photo_shown(&m), Some(raw.as_path()));
        let kept = recents::Last {
            kind: recents::OpenKind::Folder,
            path: at.clone(),
            folder: None,
            photo: None,
        };
        let left = at_exit(kept, None, &m);
        assert_eq!((left.path, left.folder), (at.clone(), Some(other.clone())));
        assert_eq!(left.photo.as_deref(), Some("x.dng"));

        // The root's own raw: the root chosen again, still no recent.
        let own = at.join("R0001175.DNG");
        open_photo(own.clone(), &mut remembered, &mut m, &mut sender(&mut sent));
        let root = Command::Library(LibraryCommand::Open(Source::Folder(at.clone())));
        assert_eq!(sent[4..], [Command::Open(own.clone()), root]);
        assert!(joins(m.apply(folder_opened(6, &at))).is_empty());
        assert_eq!(m.folder.root(), Some(at.as_path()));
        assert_eq!(photo_shown(&m), Some(own.as_path()));

        // Outside the tree: a way in, the new root and a recent.
        let elsewhere = PathBuf::from("/photos/Else");
        let raw = elsewhere.join("y.dng");
        open_photo(raw.clone(), &mut remembered, &mut m, &mut sender(&mut sent));
        let opened = Command::Library(LibraryCommand::Open(Source::Folder(elsewhere.clone())));
        assert_eq!(sent[6..], [Command::Open(raw), opened]);
        assert_eq!(m.folder.root(), Some(elsewhere.as_path()));
        assert_eq!(joins(m.apply(folder_opened(8, &elsewhere))), [elsewhere]);
    }

    /// What was open last: the root `/photos/Wedding`, with the folder of its tree and the
    /// photo given.
    fn kept_last(folder: Option<&str>, photo: Option<&str>) -> recents::Last {
        let root = PathBuf::from("/photos/Wedding");
        recents::Last {
            kind: recents::OpenKind::Folder,
            folder: folder.map(|folder| root.join(folder)),
            path: root,
            photo: photo.map(str::to_owned),
        }
    }

    fn last(photo: Option<&str>) -> recents::Last {
        kept_last(None, photo)
    }

    /// At the start with nothing named, what was open last reopens while the setting is
    /// on: its folder at once, its photo once the folder lists it. Off, or with nothing
    /// kept, nothing opens.
    #[test]
    fn what_was_open_last_reopens_with_its_photo_once_listed() {
        let kept = last(Some("b.ARW"));
        let (mut m, mut sent) = (mirror::tests::mirror(), Vec::new());
        assert_eq!(
            reopen(Some(&kept), false, &mut m, &mut sender(&mut sent)),
            None
        );
        assert_eq!(reopen(None, true, &mut m, &mut sender(&mut sent)), None);
        assert!(sent.is_empty());
        let mut reopening = reopen(Some(&kept), true, &mut m, &mut sender(&mut sent));
        let source = Source::Folder(kept.path.clone());
        assert_eq!(sent, [Command::Library(LibraryCommand::Open(source))]);
        // A way in: the tree's root.
        assert_eq!(m.folder.root(), Some(kept.path.as_path()));
        // Reading: nothing more yet; listed in chunks, the photo opens once it comes.
        reopened(&mut reopening, &mut m, &mut sender(&mut sent));
        m.apply(folder_opened(1, &kept.path));
        m.apply(listed(1, &["a.ARW"], false));
        reopened(&mut reopening, &mut m, &mut sender(&mut sent));
        assert_eq!(sent.len(), 1);
        assert!(reopening.is_some());
        m.apply(listed(1, &["b.ARW", "c.ARW"], false));
        reopened(&mut reopening, &mut m, &mut sender(&mut sent));
        assert_eq!(sent[1..], [Command::Open(kept.path.join("b.ARW"))]);
        assert_eq!(reopening, None);
    }

    /// The round trip: a subfolder chosen in the tree and a photo active in it, kept as the
    /// app ends, reopen as they were, the root opened as a way in, the subfolder chosen as
    /// a click in the tree chooses it, and the photo opened once listed; ending then keeps
    /// the same. A subfolder that is gone leaves the root, chosen again.
    #[test]
    fn a_subfolder_chosen_and_its_photo_come_back_with_the_root() {
        let kept = kept_last(Some("Day 1"), Some("b.ARW"));
        let day = kept.path.join("Day 1");
        let (mut m, mut sent) = (mirror::tests::mirror(), Vec::new());
        let mut reopening = reopen(Some(&kept), true, &mut m, &mut sender(&mut sent));
        m.apply(folder_opened(1, &kept.path));
        reopened(&mut reopening, &mut m, &mut sender(&mut sent));
        let chosen = Command::Library(LibraryCommand::Open(Source::Folder(day.clone())));
        assert_eq!(sent[1..], [chosen]);
        assert_eq!(m.folder.root(), Some(kept.path.as_path()));
        m.apply(folder_opened(2, &day));
        m.apply(listed(2, &["a.ARW", "b.ARW"], true));
        reopened(&mut reopening, &mut m, &mut sender(&mut sent));
        assert_eq!(sent[2..], [Command::Open(day.join("b.ARW"))]);
        assert_eq!(reopening, None);
        assert_eq!(m.folder.root(), Some(kept.path.as_path()));
        assert_eq!(at_exit(last(None), None, &m), kept);
        // The subfolder gone: the root is chosen again, and no photo opens.
        let (mut m, mut sent) = (mirror::tests::mirror(), Vec::new());
        let mut reopening = reopen(Some(&kept), true, &mut m, &mut sender(&mut sent));
        m.apply(folder_opened(1, &kept.path));
        reopened(&mut reopening, &mut m, &mut sender(&mut sent));
        m.apply(Event::Library(LibraryEvent::Failed {
            open: 2,
            reason: "No such folder".to_owned(),
        }));
        reopened(&mut reopening, &mut m, &mut sender(&mut sent));
        let root = Command::Library(LibraryCommand::Open(Source::Folder(kept.path.clone())));
        assert_eq!(sent[2..], [root]);
        assert_eq!(reopening, None);
    }

    /// Quitting while what was open last still reopens keeps it whole: the root's opening
    /// leaves its folder and photo, and the end writes nothing over them.
    #[test]
    fn a_quit_during_a_reopen_keeps_its_folder_and_photo() {
        let kept = kept_last(Some("Day 1"), Some("b.ARW"));
        let mut shell = opened_on("photos");
        shell.recents.set_last(Some(kept.clone()));
        let (mut m, mut sent) = (mirror::tests::mirror(), Vec::new());
        let reopening = reopen(Some(&kept), true, &mut m, &mut sender(&mut sent));
        for reaction in m.apply(folder_opened(1, &kept.path)) {
            if let Reaction::FolderOpened(path) = reaction {
                shell.remember_folder(path);
            }
        }
        assert_eq!(shell.recents.last(), Some(&kept));
        keep_at_exit(&mut shell.recents, reopening.as_ref(), None, &m);
        assert_eq!(shell.recents.last(), Some(&kept));
        // Once it has ended, the end keeps what is open then.
        keep_at_exit(&mut shell.recents, None, None, &m);
        assert_eq!(shell.recents.last(), Some(&last(None)));
    }

    /// A photo opened alone never touches what was open last: not as it opens, not on
    /// File › Close, which closes it, and not on Quit.
    #[test]
    fn a_photo_opened_alone_leaves_what_was_open_last_through_close_and_quit() {
        let kept = kept_last(Some("Day 1"), Some("b.ARW"));
        let mut shell = opened_on("photos");
        shell.recents.set_last(Some(kept.clone()));
        let mut m = mirror::tests::mirror();
        let alone = PathBuf::from("/photos/Other/x.ARW");
        let mut sent = Vec::new();
        started(Start::Photo(alone), &mut m, &mut sender(&mut sent));
        for reaction in m.apply(Event::Opened(mirror::tests::photo(1))) {
            if let Reaction::FolderOpened(path) = reaction {
                shell.remember_folder(path);
            }
        }
        assert_eq!(shell.recents.last(), Some(&kept));
        keep_at_exit(&mut shell.recents, None, None, &m);
        assert_eq!(shell.recents.last(), Some(&kept), "Quit");
        let root = m.folder.root().map(Path::to_owned);
        close(&mut Remembered::default(), &mut m, &mut sender(&mut sent));
        forget_last(&mut shell.recents, root.as_deref());
        assert_eq!(shell.recents.last(), Some(&kept), "Close");
        keep_at_exit(&mut shell.recents, None, None, &m);
        assert_eq!(shell.recents.last(), Some(&kept), "Quit after Close");
        // Close of another root leaves it too; of its own root forgets it.
        forget_last(&mut shell.recents, Some(Path::new("/photos/Other")));
        assert_eq!(shell.recents.last(), Some(&kept));
        forget_last(&mut shell.recents, Some(kept.path.as_path()));
        assert_eq!(shell.recents.last(), None);
    }

    /// What the end keeps: the root's folder and photo while the root is the tree's, the
    /// photo opening or open but never one whose open failed, and nothing of another root.
    #[test]
    fn the_end_keeps_the_active_folder_and_photo_never_a_failed_one() {
        let root = PathBuf::from("/photos/Wedding");
        let mut m = mirror::tests::mirror();
        let mut sent = Vec::new();
        open_folder(root.clone(), &mut m, &mut sender(&mut sent));
        m.apply(folder_opened(1, &root));
        assert_eq!(at_exit(last(Some("a.ARW")), None, &m), last(None));
        m.opening(root.join("b.ARW"), 2, Instant::now());
        assert_eq!(at_exit(last(None), None, &m), last(Some("b.ARW")));
        m.apply(Event::Failed(
            apertrust_engine::session::SessionError::Open {
                open: 2,
                error: Box::new(apertrust_engine::session::SessionError::Stopped),
            },
        ));
        assert!(matches!(m.photo, PhotoState::Failed { .. }));
        assert_eq!(at_exit(last(Some("b.ARW")), None, &m), last(None));
        // Another root: what was kept stays.
        let other = recents::Last {
            path: PathBuf::from("/photos/Other"),
            ..last(Some("c.ARW"))
        };
        assert_eq!(at_exit(other.clone(), None, &m), other);
    }

    /// The reopened folder's photo waits for nothing: a photo asked for meanwhile wins, a
    /// listing that ends without it ends the wait, and a folder with no photo kept opens
    /// none.
    #[test]
    fn a_photo_asked_for_meanwhile_or_gone_from_the_folder_is_not_reopened() {
        let kept = last(Some("b.ARW"));
        // Finder's file, or a click, before the listing reaches the photo.
        let (mut m, mut sent) = (mirror::tests::mirror(), Vec::new());
        let mut reopening = reopen(Some(&kept), true, &mut m, &mut sender(&mut sent));
        m.apply(folder_opened(1, &kept.path));
        open_photo(
            kept.path.join("a.ARW"),
            &mut Remembered::default(),
            &mut m,
            &mut sender(&mut sent),
        );
        m.apply(listed(1, &["a.ARW", "b.ARW"], true));
        reopened(&mut reopening, &mut m, &mut sender(&mut sent));
        assert_eq!(sent.len(), 2, "{sent:?}");
        assert_eq!(reopening, None);
        // Gone from the folder.
        let (mut m, mut sent) = (mirror::tests::mirror(), Vec::new());
        let mut reopening = reopen(Some(&kept), true, &mut m, &mut sender(&mut sent));
        m.apply(folder_opened(1, &kept.path));
        m.apply(listed(1, &["a.ARW"], true));
        reopened(&mut reopening, &mut m, &mut sender(&mut sent));
        assert_eq!(sent.len(), 1);
        assert_eq!(reopening, None);
        // No photo kept.
        let (mut m, mut sent) = (mirror::tests::mirror(), Vec::new());
        let mut reopening = reopen(Some(&last(None)), true, &mut m, &mut sender(&mut sent));
        m.apply(folder_opened(1, &kept.path));
        reopened(&mut reopening, &mut m, &mut sender(&mut sent));
        assert_eq!((sent.len(), reopening), (1, None));
        // Another folder opened over it, as Finder's file's.
        let (mut m, mut sent) = (mirror::tests::mirror(), Vec::new());
        let mut reopening = reopen(Some(&kept), true, &mut m, &mut sender(&mut sent));
        open_folder(
            PathBuf::from("/photos/Other"),
            &mut m,
            &mut sender(&mut sent),
        );
        m.apply(folder_opened(2, Path::new("/photos/Other")));
        m.apply(listed(2, &["b.ARW"], true));
        reopened(&mut reopening, &mut m, &mut sender(&mut sent));
        assert_eq!((sent.len(), reopening), (2, None));
    }

    /// Browse's Clear Menu runs Open Recent's, as `carry_out` runs it: the one list of
    /// recents empties, Browse's and Open Recent's both, and Open Recent's Clear Menu is
    /// disabled after it.
    #[test]
    fn browse_s_clear_menu_clears_the_recents_open_recent_shows() {
        let mut shell = opened_on("photos");
        shell.remember_folder(PathBuf::from("/photos/A"));
        shell.remember_folder(PathBuf::from("/photos/B"));
        let ctx = egui::Context::default();
        apertrust_ui::install(&ctx, Theme::default());
        assert_eq!(shell.recent_folders().len(), 2);
        assert_eq!(shell.show_recents(&ctx).len(), 2);
        let clear = actions::action(ActionId::CLEAR_RECENT).expect("declared");
        assert!(clear.enabled(&shell.state(&ctx, false, false)));
        // `library::Action::ClearRecents`, carried out.
        shell.show_recents(&ctx);
        let state = shell.state(&ctx, false, false);
        assert_eq!(shell.run(ActionId::CLEAR_RECENT, &state, &ctx, &[]), None);
        assert!(shell.recent_folders().is_empty());
        assert!(shell.show_recents(&ctx).is_empty());
        assert!(!clear.enabled(&shell.state(&ctx, false, false)));
    }

    /// A kept folder that is gone fails as its reading does, never waited for at the start,
    /// and is closed: the start window shows, its recent marked with why, and still enabled
    /// in Open Recent, so choosing it tries again; one a check found gone is disabled.
    #[test]
    fn a_kept_folder_that_is_gone_shows_the_start_window_its_recent_marked() {
        let kept = last(Some("b.ARW"));
        let (mut m, mut sent) = (mirror::tests::mirror(), Vec::new());
        let mut shell = opened_on("photos");
        shell.remember_folder(PathBuf::from("/photos/Gone"));
        shell.remember_folder(kept.path.clone());
        let mut reopening = reopen(Some(&kept), true, &mut m, &mut sender(&mut sent));
        // The main window while it is read.
        assert!(!shows_start(opened(&m), false, false, false));
        let failed = Event::Library(LibraryEvent::Failed {
            open: 1,
            reason: "No such folder".to_owned(),
        });
        for reaction in m.apply(failed) {
            if let Reaction::FolderFailed { path, reason } = reaction {
                shell.missing.insert(path, reason);
            }
        }
        reopened(&mut reopening, &mut m, &mut sender(&mut sent));
        assert_eq!(sent[1..], [Command::Library(LibraryCommand::Close)]);
        assert_eq!(reopening, None);
        assert!(shows_start(opened(&m), false, false, false));
        let rows = shell.recent_folders();
        assert_eq!(
            rows[0].missing,
            Some(library::Missing::Failed("No such folder"))
        );
        let gone = Path::new("/photos/Gone");
        shell.recents.mark(recents::OpenKind::Folder, gone, false);
        let ctx = egui::Context::default();
        apertrust_ui::install(&ctx, Theme::default());
        let items = shell.show_recents(&ctx);
        let state = shell.state(&ctx, false, false);
        assert_eq!(state.recents[..3], [Some(true), Some(false), None]);
        assert_eq!(items[0].words, "Folder: Wedding");
        assert!(
            items[0].tip.ends_with("\n/photos/Wedding"),
            "{}",
            items[0].tip
        );
    }

    /// What shows at the start (`docs/gui.md`, The start window): with nothing kept, or
    /// Reopen what was open last off, the start window in place of the main window; with
    /// what was kept reopening, a path from the command line, or Open With's files still
    /// being looked at, the main window. A refusal of what was handed over is asked over the
    /// main window, which becomes the start window once it is answered.
    #[test]
    fn the_start_window_shows_at_the_start_with_nothing_to_open() {
        let kept = last(None);
        let starts = |last: Option<&recents::Last>, on: bool| {
            let (mut m, mut sent) = (mirror::tests::mirror(), Vec::new());
            reopen(last, on, &mut m, &mut sender(&mut sent));
            shows_start(opened(&m), false, false, false)
        };
        assert!(starts(None, true), "nothing kept");
        assert!(starts(Some(&kept), false), "reopen off");
        assert!(!starts(Some(&kept), true), "what was open last reopening");
        for given in [
            Start::Photo(PathBuf::from("/photos/a.ARW")),
            Start::Folder(PathBuf::from("/photos")),
        ] {
            let mut m = mirror::tests::mirror();
            started_with(given, &mut m);
            assert!(!shows_start(opened(&m), false, false, false));
        }
        // `--in-folder`'s or Finder's files, looked at off this thread, then refused.
        let nothing = opened(&mirror::tests::mirror());
        assert!(!shows_start(nothing, true, false, false));
        assert!(!shows_start(nothing, false, true, false));
        assert!(shows_start(nothing, false, false, false));
        // Shown, it stays while a dialog or a drop asks over it.
        assert!(shows_start(nothing, true, true, true));
    }

    /// The start window's place: its size at the interface's scale, centred on the main
    /// window's last normal placement, the main window's own kept as it was; with no place
    /// kept, none, for the system to choose.
    #[test]
    fn the_start_window_stands_centred_where_the_main_window_was() {
        let main = settings::Window {
            size: Some([1400.0, 900.0]),
            position: Some(settings::Position {
                pixels: [200.0, 100.0],
                scale_factor: 2.0,
            }),
            maximized: true,
        };
        let start = start_placement(&main, 1.0);
        assert_eq!(start.size, Some([820.0, 520.0]));
        // (1400 - 820) / 2 = 290 points, 580 pixels at 2; (900 - 520) / 2 = 190, 380.
        let at = start.position.expect("a place");
        assert_eq!(at.pixels, [780.0, 480.0]);
        assert_eq!(at.scale_factor, 2.0);
        assert!(!start.maximized);
        let larger = start_placement(&main, 1.5);
        assert_eq!(larger.size, Some([1230.0, 780.0]));
        let first = settings::Window {
            size: None,
            position: None,
            maximized: false,
        };
        assert_eq!(start_placement(&first, 1.0).position, None);
    }

    /// The start window's menus and keys: as with a modeless dialog's window with the focus,
    /// what needs something open or acts on the main window rests; the ways in, Settings and
    /// Quit stay, and Close Window closes it, but under a modal dialog or alert.
    #[test]
    fn the_start_window_rests_what_acts_on_the_main_window() {
        let ctx = egui::Context::default();
        apertrust_ui::install(&ctx, Theme::default());
        let shell = Shell::new(&Settings::default(), Locale::EnGb, OperatingSystem::Mac);
        let state = in_start(shell.state(&ctx, false, false));
        let enabled = |id, state: &State| actions::action(id).is_some_and(|a| a.enabled(state));
        for id in [
            ActionId::OPEN,
            ActionId::OPEN_FOLDER,
            ActionId::SETTINGS,
            ActionId::QUIT,
            ActionId::CLOSE_WINDOW,
        ] {
            assert!(enabled(id, &state), "{id:?}");
        }
        for id in [
            ActionId::CLOSE,
            ActionId::EXPORT,
            ActionId::FULL_SCREEN,
            ActionId::SHOW_BROWSE,
            ActionId::PANEL_TABS[1],
        ] {
            assert!(!enabled(id, &state), "{id:?}");
        }
        let modal = State {
            modal: true,
            ..shell.state(&ctx, false, false)
        };
        let modal = in_start(modal);
        assert!(!enabled(ActionId::CLOSE_WINDOW, &modal));
        assert!(!enabled(ActionId::OPEN_FOLDER, &modal));
        assert!(enabled(ActionId::QUIT, &modal));
    }

    /// Close Window on the main window, and the Mac's close button there: on the Mac with
    /// something open, what Close does, the start window taking its place; with nothing
    /// open, under a modal dialog, with a dialog's own window focused, and on Windows and
    /// Linux, itself, which ends the app.
    #[test]
    fn the_close_button_hides_the_start_window_on_the_mac_and_rests_under_a_modal() {
        let mac = OperatingSystem::Mac;
        // quitting, shown, modal, closes
        assert_eq!(
            close_button(mac, false, true, false, false),
            CloseButton::HideStart
        );
        assert_eq!(
            close_button(mac, false, false, false, true),
            CloseButton::Close
        );
        assert_eq!(
            close_button(mac, false, true, true, false),
            CloseButton::Rest
        );
        assert_eq!(
            close_button(mac, false, false, true, false),
            CloseButton::Rest
        );
        // Nothing open on the main window, and Quit's own close, end the app.
        assert_eq!(
            close_button(mac, false, false, false, false),
            CloseButton::Quit
        );
        assert_eq!(
            close_button(mac, true, true, false, false),
            CloseButton::Quit
        );
        for os in [OperatingSystem::Windows, OperatingSystem::Nix] {
            assert_eq!(close_button(os, false, true, true, true), CloseButton::Quit);
        }
        assert_eq!(start_closes(mac), StartCloses::Hide);
        assert_eq!(start_closes(OperatingSystem::Nix), StartCloses::Quit);
        assert_eq!(start_closes(OperatingSystem::Windows), StartCloses::Quit);
    }

    /// With the start window hidden on the Mac, a menu's choice shows it again but for what
    /// needs no window: Quit (whose alert shows it itself), Settings, the log and jobs
    /// windows, Clear Menu and Close Window.
    #[test]
    fn a_menu_choice_shows_the_hidden_start_window_but_for_what_needs_none() {
        for id in [
            ActionId::OPEN,
            ActionId::OPEN_FOLDER,
            ActionId::OPEN_RECENT[0],
            ActionId::ABOUT,
        ] {
            assert!(reveals_for(&[id]), "{id:?}");
        }
        for id in OVER_NO_WINDOW {
            assert!(!reveals_for(&[id]), "{id:?}");
        }
        assert!(reveals_for(&[ActionId::SETTINGS, ActionId::OPEN]));
        assert!(!reveals_for(&[]));
    }

    /// As it becomes the start window, the window leaves full screen, then its maximised
    /// state, then takes the start window's size centred where it was; as only the scale
    /// changes, its size where it stands; nothing as the main window.
    #[test]
    fn the_window_leaves_full_screen_and_maximised_before_it_takes_the_start_size() {
        // wanted, reshape, changed, full screen, maximised
        let shape = start_shape;
        assert_eq!(shape(false, true, true, true, true), StartShape::Keep);
        assert_eq!(shape(true, false, false, true, true), StartShape::Keep);
        assert_eq!(
            shape(true, true, false, true, true),
            StartShape::LeaveFullScreen
        );
        assert_eq!(
            shape(true, true, false, false, true),
            StartShape::Unmaximise
        );
        let centred = StartShape::Resize { centred: true };
        assert_eq!(shape(true, true, false, false, false), centred);
        let in_place = StartShape::Resize { centred: false };
        assert_eq!(shape(true, false, true, false, false), in_place);
    }

    /// With no place kept, as on the first run, the main window is centred on its monitor,
    /// never past its top or left edge.
    #[test]
    fn with_no_place_kept_the_main_window_is_centred_on_its_monitor() {
        let monitor = Monitor {
            rect: egui::Rect::from_min_size(egui::pos2(1920.0, 0.0), egui::vec2(3024.0, 1964.0)),
            scale_factor: 2.0,
            primary: false,
        };
        let at = centred(&monitor, [1400.0, 900.0]);
        assert_eq!(at.pixels, [1920.0 + 112.0, 82.0]);
        assert_eq!(at.scale_factor, 2.0);
        // Larger than the monitor: at its corner.
        let at = centred(&monitor, [2000.0, 1200.0]);
        assert_eq!(at.pixels, [1920.0, 0.0]);
        // Never smaller than the main window's least.
        let small = centred(&monitor, [400.0, 300.0]);
        let least = centred(&monitor, WINDOW_MIN);
        assert_eq!(small, least);
    }

    /// The start window's keys after the app's: the keymap takes Cmd+O and leaves ↓, Enter
    /// and Escape to the list, as the app reads its keys before the window is drawn.
    #[test]
    fn the_start_window_s_keys_reach_the_list_after_the_app_s() {
        struct Both {
            shell: Shell,
            list: start::List,
            asked: Vec<start::Asked>,
            pressed: Vec<ActionId>,
        }
        let mut harness = Harness::builder()
            .with_size(start::SIZE)
            .with_os(OperatingSystem::Mac)
            .with_context_setup(|ctx| {
                apertrust_ui::install(ctx, Theme::default());
                registry_keys_only(ctx);
            })
            .build_ui_state(
                |ui, both: &mut Both| {
                    let ctx = ui.ctx().clone();
                    let state = in_start(both.shell.state(&ctx, false, false));
                    let keys = actions::read(&ctx, &both.shell.keymap, &state);
                    both.pressed.extend(keys.pressed);
                    let paths = [
                        PathBuf::from("/photos/Lisbon"),
                        PathBuf::from("/photos/Kraków"),
                    ];
                    let recents: Vec<library::Recent<'_>> = paths
                        .iter()
                        .map(|path| library::Recent {
                            kind: recents::OpenKind::Folder,
                            path,
                            missing: None,
                        })
                        .collect();
                    let shown = start::Shown {
                        home: None,
                        close: false,
                        keys: !state.modal,
                        version: "0.1.0+197~g9201df0",
                    };
                    both.asked
                        .extend(start::show(ui, &recents, &mut both.list, shown));
                },
                Both {
                    shell: Shell::new(&Settings::default(), Locale::EnGb, OperatingSystem::Mac),
                    list: start::List::shown(),
                    asked: Vec::new(),
                    pressed: Vec::new(),
                },
            );
        harness.run();
        for key in [Key::ArrowDown, Key::Enter, Key::Escape] {
            harness.key_press(key);
            harness.run();
        }
        harness.key_press_modifiers(Modifiers::COMMAND, Key::O);
        harness.run();
        let both = harness.state();
        assert_eq!(
            both.asked,
            [
                start::Asked::Open(PathBuf::from("/photos/Kraków")),
                start::Asked::Close
            ]
        );
        assert_eq!(both.pressed, [ActionId::OPEN]);
    }

    #[test]
    fn close_window_on_the_mac_s_main_window_does_what_close_does() {
        let ctx = egui::Context::default();
        apertrust_ui::install(&ctx, Theme::default());
        let shell = Shell::new(&Settings::default(), Locale::EnGb, OperatingSystem::Mac);
        let open = State {
            opened: true,
            ..shell.state(&ctx, true, false)
        };
        let mac = OperatingSystem::Mac;
        assert_eq!(close_window_runs(mac, &open), ActionId::CLOSE);
        for os in [OperatingSystem::Windows, OperatingSystem::Nix] {
            assert_eq!(close_window_runs(os, &open), ActionId::CLOSE_WINDOW);
        }
        let nothing = shell.state(&ctx, false, false);
        assert_eq!(close_window_runs(mac, &nothing), ActionId::CLOSE_WINDOW);
        let modal = State {
            modal: true,
            ..open
        };
        assert_eq!(close_window_runs(mac, &modal), ActionId::CLOSE_WINDOW);
        let settings = State {
            dialog: true,
            dialog_window: true,
            ..open
        };
        assert_eq!(close_window_runs(mac, &settings), ActionId::CLOSE_WINDOW);
    }

    /// Open Recent's items label each folder, its name with as many of the folders it is in
    /// as tell it from another of the same name, and their paths in the tips.
    #[test]
    fn open_recent_s_items_name_the_folders_and_tell_two_of_one_name_apart() {
        let mut shell = opened_on("photos");
        for path in [
            "/photos/a/Shoot/Selects",
            "/photos/b/Shoot/Selects",
            "/photos/Lisbon/Selects",
            "/photos/Wedding",
        ] {
            shell.remember_folder(PathBuf::from(path));
        }
        let ctx = egui::Context::default();
        apertrust_ui::install(&ctx, Theme::default());
        let items = shell.show_recents(&ctx);
        let words: Vec<&str> = items.iter().map(|item| item.words.as_str()).collect();
        assert_eq!(
            words,
            [
                "Folder: Wedding",
                "Folder: Selects — Lisbon",
                "Folder: Selects — Shoot — b",
                "Folder: Selects — Shoot — a"
            ]
        );
        assert_eq!(items[1].tip, "/photos/Lisbon/Selects");
        let state = shell.state(&ctx, false, false);
        let listed = [Some(true), Some(true), Some(true), Some(true), None];
        assert_eq!(state.recents[..5], listed);
        assert_eq!(told_apart(&[Path::new("/")]), [vec!["/".to_owned()]]);
    }

    /// On the Mac, the bar is filled from the recents each frame; a drain before the next
    /// frame that reorders them, as a slow open landing, never changes what a choice of
    /// an item it shows opens.
    #[test]
    fn a_choice_on_the_mac_s_bar_opens_what_it_showed_though_the_list_changed() {
        let mut shell = opened_on("photos");
        let (older, newer) = (PathBuf::from("/photos/a"), PathBuf::from("/photos/b"));
        shell.remember_folder(older.clone());
        shell.remember_folder(newer.clone());
        let ctx = egui::Context::default();
        apertrust_ui::install(&ctx, Theme::default());
        let items = shell.show_recents(&ctx);
        assert_eq!(items[0].path, newer);
        // The drain: the older one opens late and goes to the top.
        shell.remember_folder(older.clone());
        let state = shell.state(&ctx, false, false);
        shell.run(ActionId::OPEN_RECENT[0], &state, &ctx, &[]);
        assert_eq!(shell.open_recent.take(), Some(newer));
    }

    /// Open… from a photo alone opens folder mode: the photo chosen and its folder behind
    /// it, whether it is another of the folder's photos or the one alone again.
    #[test]
    fn open_from_a_photo_alone_opens_its_folder() {
        let folder = PathBuf::from("/photos");
        let listed = || Command::Library(LibraryCommand::Open(Source::Folder(folder.clone())));
        for chosen in ["/photos/b.ARW", "/photos/a.ARW"] {
            let mut m = mirror::tests::mirror();
            started_with(Start::Photo(PathBuf::from("/photos/a.ARW")), &mut m);
            let mut sent = Vec::new();
            let chosen = PathBuf::from(chosen);
            open_photo(
                chosen.clone(),
                &mut Remembered::default(),
                &mut m,
                &mut |command| {
                    sent.push(command);
                    sent.len() as u64 + 1
                },
            );
            assert_eq!(sent, [Command::Open(chosen), listed()]);
            assert_eq!(m.folder.shown(), Some(folder.as_path()));
        }
    }

    #[test]
    fn the_command_line_names_the_user_data_folder_by_any_of_its_flags() {
        let given = |flag, folder: &str| {
            let folder = PathBuf::from(folder);
            Some(Given { flag, folder })
        };
        assert_eq!(
            read(&["-d", "settings", "--user-data-dir", "dev", "photo.ARW"]),
            Ok(Arguments {
                open: photo_arw(),
                in_folder: Vec::new(),
                domains: vec!["settings".to_owned()],
                user_data: given("--user-data-dir", "dev"),
                cache: None,
                conf: Vec::new(),
                safe_mode: false,
                reopen: None,
                flags: ["-d", "settings", "--user-data-dir"]
                    .map(OsString::from)
                    .into_iter()
                    .chain([std::path::absolute("dev").expect("absolute").into()])
                    .collect(),
            })
        );
        for flag in folders::FLAGS {
            let apart = read(&[flag, "dev"]).expect("read");
            assert_eq!(apart.user_data, given(flag, "dev"));
            assert_eq!(read(&[&format!("{flag}=dev")]), Ok(apart));
        }
        // A leading `~`, which the shell leaves after `=`, reaches the folders as given in
        // either form, and they take it for the home folder.
        let tilde = given("--user-data-dir", "~/dev");
        for line in [
            &["--user-data-dir=~/dev"][..],
            &["--user-data-dir", "~/dev"],
        ] {
            assert_eq!(read(line).expect("read").user_data, tilde);
        }
        // The last given, as darktable's --configdir.
        let last = read(&["--configdir", "one", "photo.ARW", "--user-data-dir=two"]);
        let last = last.expect("read");
        assert_eq!(last.user_data, given("--user-data-dir", "two"));
        assert_eq!(last.open, photo_arw());
        // A flag without its folder stops the start, so the platform's is never used for it.
        assert_eq!(
            read(&["photo.ARW", "--config-dir"]),
            Err("--config-dir needs a folder".to_owned())
        );
        // None given: the variable's or the platform's.
        assert_eq!(read(&["photo.ARW"]).expect("read").user_data, None);
    }

    /// `--cache-dir` or darktable's `--cachedir`, in either form, the last given, beside the
    /// user data folder's flags; one without its folder stops the start.
    #[test]
    fn the_command_line_names_the_cache_folder() {
        let given = |flag, folder: &str| {
            let folder = PathBuf::from(folder);
            Some(Given { flag, folder })
        };
        let line = ["--cache-dir", "one", "--configdir=dev", "photo.ARW"];
        let read_ = read(&line).expect("read");
        assert_eq!(read_.cache, given("--cache-dir", "one"));
        assert_eq!(read_.user_data.map(|given| given.flag), Some("--configdir"));
        assert_eq!(read_.open, photo_arw());
        for flag in folders::CACHE_FLAGS {
            let apart = read(&[flag, "dev"]).expect("read");
            assert_eq!(apart.cache, given(flag, "dev"));
            assert_eq!(read(&[&format!("{flag}=dev")]), Ok(apart));
        }
        let last = read(&["--cache-dir", "one", "photo.ARW", "--cachedir=~/two"]);
        assert_eq!(last.expect("read").cache, given("--cachedir", "~/two"));
        assert_eq!(
            read(&["photo.ARW", "--cachedir"]),
            Err("--cachedir needs a folder".to_owned())
        );
        assert_eq!(read(&["photo.ARW"]).expect("read").cache, None);
    }

    /// `--conf KEY=VALUE` in either form, each kept in order for the settings to read, the
    /// last for one key counting there; one without its setting stops the start.
    #[test]
    fn the_command_line_sets_settings_for_the_run_with_conf() {
        let line = [
            "--conf",
            "scale=1.5",
            "photo.ARW",
            "--conf=theme=light",
            "--conf",
            "scale=2",
        ];
        let read_ = read(&line).expect("read");
        assert_eq!(read_.conf, ["scale=1.5", "theme=light", "scale=2"]);
        assert_eq!(read_.open, photo_arw());
        // A value with `=` in it, as a table's, keeps it.
        let table = read(&["--conf=keys={ undo = [\"Ctrl+K\"] }"]).expect("read");
        assert_eq!(table.conf, ["keys={ undo = [\"Ctrl+K\"] }"]);
        assert_eq!(
            read(&["photo.ARW", "--conf"]),
            Err("--conf needs KEY=VALUE".to_owned())
        );
        // `--configdir` is a folder's flag, not `--conf`'s.
        let folder = read(&["--configdir=dev"]).expect("read");
        assert!(folder.conf.is_empty() && folder.user_data.is_some());
        // Not UTF-8, in either form: the flag named once.
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            let not_utf8 = |arg: &[u8]| OsString::from_vec(arg.to_vec());
            let error = Err("--conf edit.tab=\u{FFFD}: not UTF-8".to_owned());
            assert_eq!(
                arguments([not_utf8(b"--conf=edit.tab=\xff")], on_disk),
                error
            );
            let apart = ["--conf".into(), not_utf8(b"edit.tab=\xff")];
            assert_eq!(arguments(apart, on_disk), error);
        }
    }

    /// From the command line to the file: the settings are read from, and written to, the
    /// user data folder it names, created as the app starts.
    #[test]
    fn the_settings_are_kept_in_the_user_data_folder_the_command_line_names() {
        let scratch = settings::testing::Folder::new("user-data");
        let folder = scratch.0.join("dev");
        let line = [
            "--user-data-dir".into(),
            folder.clone().into(),
            "photo.ARW".into(),
        ];
        let given = arguments(line, on_disk).expect("read").user_data;
        let folders = Folders::find(given, None, false)
            .expect("usable")
            .expect("a folder");
        assert_eq!(folders.user_data(), folder);
        assert!(folder.is_dir());
        // Its cache with it, unless the variable names another, chosen and not yet made.
        if std::env::var_os(folders::CACHE_VARIABLE).is_none() {
            assert_eq!(folders.cache(), folder.join("cache"));
            assert!(!folders.cache().exists());
        }
        let none = settings::overrides(&[]).expect("none");
        let (mut keeper, settings) = Keeper::open_in(Some(&folders), &none);
        assert_eq!(settings, Settings::default());
        let light = Settings {
            theme: Builtin::Light,
            ..settings
        };
        keeper.frame(Instant::now(), &light);
        keeper.flush();
        let text = std::fs::read_to_string(folder.join("settings.toml")).expect("written there");
        assert!(text.contains("theme = \"light\""), "{text}");
        // And read from there on the next start.
        let (_, settings) = Keeper::open_in(Some(&folders), &none);
        assert_eq!(settings.theme, Builtin::Light);
    }

    /// The shell and its settings' keeper, as the app's frame runs them.
    struct Kept {
        shell: Shell,
        keeper: Keeper,
        changes: Vec<settings_dialog::Change>,
    }

    #[test]
    fn the_theme_and_the_language_survive_a_new_shell() {
        let folder = settings::testing::Folder::new("survive");
        let windows = OperatingSystem::Windows;
        let (keeper, settings) = Keeper::open(folder.storage());
        let shell = Shell::new(&settings, Locale::EnGb, windows);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(800.0, 600.0))
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
            .build_ui_state(
                |ui, kept: &mut Kept| {
                    let ctx = ui.ctx().clone();
                    // What Settings' window changed, as `App::logic` hands it on.
                    let changes = std::mem::take(&mut kept.changes);
                    apply_settings(&changes, &mut kept.shell, &mut kept.keeper, &ctx);
                    keep(&mut kept.shell, &mut kept.keeper, &ctx, true);
                },
                Kept {
                    shell,
                    keeper,
                    changes: Vec::new(),
                },
            );
        harness.run();
        harness.state_mut().changes = vec![
            settings_dialog::Change::Set("theme", "light".into()),
            settings_dialog::Change::Set("locale", "uk".into()),
        ];
        harness.run();
        harness.state_mut().shell.panel_tab = 3;
        harness.run();
        // The app ends, and starts again.
        harness.state_mut().keeper.flush();
        drop(harness);
        let (_, settings) = Keeper::open(folder.storage());
        let shell = Shell::new(&settings, Locale::EnGb, windows);
        assert_eq!(shell.theme, Builtin::Light);
        assert_eq!(
            (shell.locale, shell.kept_locale),
            (Locale::Uk, Some(Locale::Uk))
        );
        assert_eq!(shell.panel_tab, 3);
        // The next start's first frame is in them.
        let ctx = egui::Context::default();
        prepare(&ctx, &shell, settings.scale);
        assert_eq!(theme(&ctx).colors, Theme::builtin(Builtin::Light).colors);
        assert_eq!(
            tr(&ctx, "settings-close"),
            Translator::new(Locale::Uk).text("settings-close")
        );
    }

    #[test]
    fn a_collapsible_group_opened_stays_open_in_the_next_run() {
        use apertrust_engine::session::{ControlState, ModuleState};
        use apertrust_modules::Module as _;
        use apertrust_modules::testing::Sample;

        let folder = settings::testing::Folder::new("groups");
        let windows = OperatingSystem::Windows;
        let (keeper, settings) = Keeper::open(folder.storage());
        let shell = Shell::new(&settings, Locale::EnGb, windows);
        let definition = Sample.definition();
        let state = ModuleState {
            module: definition.id,
            enabled: true,
            edited: false,
            controls: definition
                .controls
                .iter()
                .map(|control| ControlState {
                    control: control.id,
                    value: control.default(),
                    default: control.default(),
                    mark: None,
                    available: true,
                    options: Vec::new(),
                    range: None,
                })
                .collect(),
            presets: Vec::new(),
            views: Vec::new(),
        };
        // Its Shape tab, where the Midtones group is.
        let mut panels = modules::Panels::default();
        panels.apply(&Change::Tab {
            module: "sample",
            tab: 1,
        });
        let mut harness = Harness::builder()
            .with_size(egui::vec2(400.0, 300.0))
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
            .build_ui_state(
                move |ui, kept: &mut Kept| {
                    let ctx = ui.ctx().clone();
                    let shell = &kept.shell;
                    let groups = |group: &str| shell.group_open("sample", group);
                    let changes = modules::section(
                        ui,
                        &Sample,
                        &state,
                        true,
                        &panels,
                        &groups,
                        false,
                        true,
                        None,
                        &modules::Pickers::NONE,
                    );
                    for change in changes {
                        if let Change::Group { module, group } = change {
                            kept.shell.toggle_group(module, group);
                        }
                    }
                    keep(&mut kept.shell, &mut kept.keeper, &ctx, true);
                },
                Kept {
                    shell,
                    keeper,
                    changes: Vec::new(),
                },
            );
        harness.run();
        let midtones = "module-sample-collapsible-midtones";
        let group = harness.get_by_role_and_label(Role::Button, midtones);
        assert_eq!(group.accesskit_node().data().is_expanded(), Some(false));
        group.click();
        harness.run();
        let group = harness.get_by_role_and_label(Role::Button, midtones);
        assert_eq!(group.accesskit_node().data().is_expanded(), Some(true));
        // The app ends, and starts again: open, the others still closed.
        harness.state_mut().keeper.flush();
        drop(harness);
        let (_, settings) = Keeper::open(folder.storage());
        let open = BTreeMap::from([("midtones".to_owned(), true)]);
        assert_eq!(
            settings.edit.groups,
            BTreeMap::from([("sample".to_owned(), open)])
        );
        let shell = Shell::new(&settings, Locale::EnGb, windows);
        assert!(shell.group_open("sample", "midtones"));
        assert!(!shell.group_open("sample", "grain"));
        assert!(!shell.group_open("exposure", "midtones"));
    }

    /// The side panel as the app draws it with a photo open, Exposure untouched in the
    /// mirror: what Scopes asks for applied to the shell, what the session is asked to
    /// count sent as `App::ui` sends it, and the settings kept as the app's frame keeps
    /// them.
    struct Side {
        shell: Shell,
        keeper: Keeper,
        scopes: scopes::Scopes,
        values: Values,
        sent: Vec<Command>,
    }

    /// The side panel with the settings in `folder`.
    fn side(folder: &settings::testing::Folder) -> Harness<'static, Side> {
        use apertrust_engine::session::{ControlState, ModuleState};

        let (keeper, settings) = Keeper::open(folder.storage());
        let exposure = apertrust_modules::MODULES
            .iter()
            .find(|module| module.definition().id == "exposure")
            .expect("Exposure is built in")
            .definition();
        let untouched = ModuleState {
            module: exposure.id,
            enabled: true,
            edited: false,
            controls: exposure
                .controls
                .iter()
                .map(|control| ControlState {
                    control: control.id,
                    value: control.default(),
                    default: control.default(),
                    mark: None,
                    available: true,
                    options: Vec::new(),
                    range: None,
                })
                .collect(),
            presets: Vec::new(),
            views: Vec::new(),
        };
        Harness::builder()
            .with_size(egui::vec2(360.0, 700.0))
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
            .build_ui_state(
                |ui, side: &mut Side| {
                    let ctx = ui.ctx().clone();
                    let state = side.shell.state(&ctx, true, false);
                    if let Some(id) = side.shell.tab_bar(ui) {
                        side.shell.run(id, &state, &ctx, &[]);
                    }
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let (shell, measured) = (&side.shell, side.scopes.measured());
                    let panels = modules::Panels::default();
                    let modules = apertrust_modules::MODULES;
                    let values = &side.values;
                    let (scopes, _) = panel_body(
                        ui,
                        shell,
                        Some((measured, true)),
                        modules,
                        values,
                        &panels,
                        false,
                        true,
                        None,
                        &modules::Pickers::NONE,
                    );
                    for change in scopes {
                        side.shell.scopes_change(change);
                    }
                    side.sent
                        .extend(side.scopes.ask(side.shell.scopes_wanted()));
                    keep(&mut side.shell, &mut side.keeper, &ctx, true);
                },
                Side {
                    shell: Shell::new(&settings, Locale::EnGb, OperatingSystem::Mac),
                    keeper,
                    scopes: scopes::Scopes::default(),
                    values: Values {
                        modules: vec![untouched],
                        settings: Vec::new(),
                        edited: false,
                        through: 1,
                        extent: [84, 62],
                        live: true,
                    },
                    sent: Vec::new(),
                },
            )
    }

    /// What the scopes' graph, `name`, says to screen readers.
    fn graph_says(harness: &Harness<'_, Side>, name: &str) -> String {
        let graph = harness.get_by_role_and_label(Role::Image, name);
        let said = graph.accesskit_node().description();
        said.unwrap_or_default().to_owned()
    }

    fn counted(histogram: bool, waveform: bool, vectorscope: bool) -> Command {
        Command::Scopes(Some(Graphs {
            histogram,
            waveform,
            vectorscope,
        }))
    }

    /// The side panel's editing tab as the app draws it from its mirror ([`edit_tab`]),
    /// the mirror fed events by the test. With nothing to edit, a frame may change only the
    /// sections' own state.
    struct Editing {
        shell: Shell,
        mirror: Mirror,
    }

    fn editing(folder: &settings::testing::Folder) -> Harness<'static, Editing> {
        let (_, settings) = Keeper::open(folder.storage());
        Harness::builder()
            // Tall enough for every section of a tab to be laid out.
            .with_size(egui::vec2(360.0, 6000.0))
            // egui's warning of two widgets with one id, drawn in a glyph the theme's fonts
            // lack, fails these tests: every module's section keeps ids of its own.
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
            .build_ui_state(
                |ui, editing: &mut Editing| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let panels = modules::Panels::default();
                    let (shell, mirror) = (&editing.shell, &editing.mirror);
                    let (_, changes) = edit_tab(ui, shell, mirror, &panels, false);
                    let own = |change: &Change| {
                        matches!(
                            change,
                            Change::Toggle { .. } | Change::Tab { .. } | Change::Group { .. }
                        )
                    };
                    assert!(
                        mirror.takes_edits() || changes.iter().all(own),
                        "{changes:?}"
                    );
                },
                Editing {
                    shell: Shell::new(&settings, Locale::EnGb, OperatingSystem::Mac),
                    mirror: crate::mirror::tests::mirror(),
                },
            )
    }

    /// The module titles `tab` lists, in English.
    fn titles(tab: usize) -> Vec<String> {
        let words = Translator::new(Locale::EnGb);
        TAB_MODULES[tab]
            .iter()
            .map(|id| words.text(&apertrust_modules::messages::title(id)))
            .collect()
    }

    /// Whether `tab` draws the no-photo state: each of its modules' sections, their resets
    /// and every control disabled, and Scopes with no graph. Panics saying what differs.
    fn draws_no_photo(harness: &Harness<'_, Editing>, tab: usize) {
        // Scopes keeps its section, open as it was, with no graph.
        harness.get_by_role_and_label(Role::Button, "Scopes");
        assert_eq!(
            graph_says_in(harness, "Histogram"),
            "Nothing measured yet",
            "tab {tab}"
        );
        for title in titles(tab) {
            harness.get_by_role_and_label(Role::Button, &title);
            let reset = format!("Reset {title}");
            let reset = harness.get_by_role_and_label(Role::Button, &reset);
            assert!(reset.accesskit_node().is_disabled(), "{title}'s reset");
        }
        let mut drawn = 0;
        for role in [
            Role::Slider,
            Role::ComboBox,
            Role::CheckBox,
            Role::SpinButton,
        ] {
            for node in harness.query_all_by_role(role) {
                let node = node.accesskit_node();
                let name = node.label().unwrap_or_default();
                assert!(node.is_disabled(), "tab {tab}: {role:?} {name}");
                drawn += 1;
            }
        }
        assert!(drawn > 0, "tab {tab} drew no control");
    }

    /// Exposure's slider: its value as screen readers hear it, and whether it is disabled.
    fn exposure_slider<S>(harness: &Harness<'_, S>) -> (String, bool) {
        let slider = harness.get_by_role_and_label(Role::Slider, "Exposure");
        let node = slider.accesskit_node();
        (
            node.value().unwrap_or_default().to_owned(),
            node.is_disabled(),
        )
    }

    #[test]
    fn with_no_photo_every_edit_tab_draws_its_modules_and_scopes_disabled_at_defaults() {
        let folder = settings::testing::Folder::new("panel-no-photo");
        let mut harness = editing(&folder);
        for tab in [BASE, 2, 3, 4] {
            harness.state_mut().shell.panel_tab = tab;
            harness.run();
            draws_no_photo(&harness, tab);
        }
        // Exposure at its definition's default.
        harness.state_mut().shell.panel_tab = BASE;
        harness.run();
        assert_eq!(exposure_slider(&harness), (String::from("+0.000 EV"), true));
    }

    /// Every module's values as the session reports them for a photo, every control live,
    /// Exposure's at `exposure`, after `through` commands.
    fn photo_values(exposure: f32, through: u64) -> Values {
        let mut values = crate::mirror::tests::mirror().values;
        for state in &mut values.modules {
            for control in &mut state.controls {
                control.available = true;
                if (state.module, control.control) == ("exposure", "exposure") {
                    control.value = ControlValue::Number(exposure);
                }
            }
        }
        values.through = through;
        values.live = true;
        values
    }

    /// A photo opening shows its values read-only until it is live (decision 77): every
    /// control disabled, its tooltip saying why, Scopes with no graph; live, enabled in
    /// the next frame.
    #[test]
    fn a_photo_opening_shows_its_values_disabled_until_it_is_live() {
        use crate::mirror::tests::photo;
        let folder = settings::testing::Folder::new("panel-opening");
        let mut harness = editing(&folder);
        let mirror = &mut harness.state_mut().mirror;
        mirror.opening(PathBuf::from("IMG_2041.ARW"), 2, Instant::now());
        mirror.apply(Event::Opened(Photo {
            path: PathBuf::from("IMG_2041.ARW"),
            ..photo(2)
        }));
        mirror.apply(Event::Values(Values {
            live: false,
            ..photo_values(1.5, 3)
        }));
        harness.run();
        assert_eq!(exposure_slider(&harness), (String::from("+1.500 EV"), true));
        assert_eq!(graph_says_in(&harness, "Histogram"), "Nothing measured yet");
        // Its tooltip says why.
        harness
            .get_by_role_and_label(Role::Slider, "Exposure")
            .hover();
        // A second's frames, past egui's tooltip delay.
        for _ in 0..60 {
            harness.step();
        }
        let tip = "Opening IMG_2041…";
        assert!(harness.query_by_role_and_label(Role::Label, tip).is_some());
        // Screen readers hear it on each row of controls.
        let described = harness
            .query_all_by_role(Role::Group)
            .filter(|row| row.accesskit_node().description().as_deref() == Some(tip))
            .count();
        assert!(described > 1, "{described}");
        // Live: enabled, and nothing said of opening.
        harness
            .state_mut()
            .mirror
            .apply(Event::Values(photo_values(1.5, 3)));
        harness.run();
        assert_eq!(
            exposure_slider(&harness),
            (String::from("+1.500 EV"), false)
        );
        let described = harness
            .query_all_by_role(Role::Group)
            .filter(|row| row.accesskit_node().description().as_deref() == Some(tip))
            .count();
        assert_eq!(described, 0);
    }

    /// A photo opening as the app draws it, for the snapshots: the central bar, Rendering…
    /// on it from `rendering`, the picture fitted in the photo's slot, and the side panel's
    /// Base tab with the photo's values read-only.
    struct Opening {
        shell: Shell,
        mirror: Mirror,
        rendering: Instant,
        picture: Option<(u64, egui::TextureHandle)>,
    }

    fn opening_scene(folder: &settings::testing::Folder) -> Harness<'static, Opening> {
        use crate::mirror::tests::photo;
        let (_, settings) = Keeper::open(folder.storage());
        let mut mirror = crate::mirror::tests::mirror();
        let asked = Instant::now();
        mirror.opening(PathBuf::from("IMG_2041.ARW"), 2, asked);
        // The camera's picture: a gradient, sky over ground.
        let (width, height) = (300, 200);
        let pixels: Vec<u8> = (0..width * height)
            .flat_map(|i| {
                let (x, y) = (i % width, i / width);
                let sky = y < height * 3 / 5;
                let shade = (x * 120 / width) as u8;
                if sky {
                    [90 + shade / 2, 140 + shade / 3, 200, 255]
                } else {
                    [70 + shade / 3, 110, 60 + shade / 4, 255]
                }
            })
            .collect();
        mirror.apply(Event::OpenPreview(apertrust_engine::session::OpenPreview {
            open: 2,
            picture: Some(apertrust_engine::session::library::Thumbnail {
                width,
                height,
                pixels: pixels.into(),
            }),
            source: apertrust_engine::session::PreviewSource::Embedded,
        }));
        mirror.apply(Event::Opened(Photo {
            path: PathBuf::from("IMG_2041.ARW"),
            ..photo(2)
        }));
        mirror.apply(Event::Values(Values {
            live: false,
            // Its edit crops it square: the 3:2 picture letterboxed in the square place.
            extent: [600, 600],
            ..photo_values(1.5, 3)
        }));
        Harness::builder()
            .with_size(egui::vec2(1000.0, 640.0))
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
            .build_ui_state(
                |ui, scene: &mut Opening| {
                    let t = theme(ui.ctx());
                    let full = ui.max_rect();
                    let side = egui::Rect::from_min_max(
                        egui::pos2(full.right() - 360.0, full.top()),
                        full.max,
                    );
                    let centre =
                        egui::Rect::from_min_max(full.min, egui::pos2(side.left(), full.bottom()));
                    ui.painter()
                        .rect_filled(centre, 0.0, t.colors.photo_surround);
                    ui.painter().rect_filled(side, 0.0, t.colors.panel);
                    let now = scene.rendering;
                    ui.scope_builder(egui::UiBuilder::new().max_rect(centre), |ui| {
                        let rendering = scene
                            .mirror
                            .rendering(now)
                            .then(|| tr(ui.ctx(), "photo-rendering"));
                        let layer = tr(ui.ctx(), "layer-image");
                        CentralBar::new(&layer, Zoom::Fit)
                            .toggles(
                                Toggle::new(false, false),
                                Toggle::new(false, false),
                                Toggle::new(false, false),
                                Toggle::new(false, false),
                            )
                            .status(rendering.as_deref())
                            .show(ui);
                        let slot = egui::Rect::from_min_max(ui.cursor().min, centre.max);
                        let block = PhotoFrame::new(slot, Margin::Points(24.0));
                        let ppp = ui.pixels_per_point();
                        let room = block.bounds();
                        let bounds = ((room.x * ppp) as u32, (room.y * ppp) as u32);
                        // As `App::central_panel` places and draws it.
                        let (picture, generation) = scene.mirror.picture().expect("the picture");
                        let values = Some(&scene.mirror.values);
                        let placed = place_photo(&block, bounds, ppp, values, Some(picture))
                            .expect("a place");
                        let painter = ui.painter_at(slot);
                        paint_picture(&painter, &placed, picture, generation, &mut scene.picture);
                    });
                    ui.scope_builder(egui::UiBuilder::new().max_rect(side), |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        let panels = modules::Panels::default();
                        edit_tab(ui, &scene.shell, &scene.mirror, &panels, false);
                    });
                },
                Opening {
                    shell: Shell::new(&settings, Locale::EnGb, OperatingSystem::Mac),
                    mirror,
                    rendering: asked,
                    picture: None,
                },
            )
    }

    /// The central panel showing a photo's picture while it opens, its controls disabled,
    /// with no Rendering… before a second, then with it (decision 77). Look at them.
    #[test]
    fn a_photo_opening_shows_its_picture_unmarked_then_rendering() {
        let folder = settings::testing::Folder::new("opening-scene");
        let mut harness = opening_scene(&folder);
        harness.run();
        assert!(harness.query_by_label("Rendering…").is_none());
        assert_eq!(exposure_slider(&harness), (String::from("+1.500 EV"), true));
        harness.snapshot("opening-picture");
        let later = harness.state().rendering + mirror::RENDERING_AFTER;
        harness.state_mut().rendering = later;
        harness.run();
        assert!(harness.query_by_label("Rendering…").is_some());
        harness.snapshot("opening-rendering");
    }

    /// A photo's slot as the central panel places and draws it while the photo opens, its
    /// picture's rectangle and the photo's place kept from the last frame.
    struct Places {
        mirror: Mirror,
        picture: Option<(u64, egui::TextureHandle)>,
        drawn: Option<egui::Rect>,
        placed: Option<Placed>,
    }

    /// A slot at twice the scale whose room is 1800 pixels wide, with a photo opening whose
    /// picture, `width` by `height`, has come and nothing else.
    fn places_scene(width: u32, height: u32) -> Harness<'static, Places> {
        let mut mirror = crate::mirror::tests::mirror();
        mirror.opening(PathBuf::from("IMG_2041.ARW"), 2, Instant::now());
        mirror.apply(Event::OpenPreview(apertrust_engine::session::OpenPreview {
            open: 2,
            picture: Some(apertrust_engine::session::library::Thumbnail {
                width,
                height,
                pixels: vec![128; (width * height * 4) as usize].into(),
            }),
            source: apertrust_engine::session::PreviewSource::Embedded,
        }));
        Harness::builder()
            .with_size(egui::vec2(964.0, 1000.0))
            .with_pixels_per_point(2.0)
            .build_ui_state(
                |ui, scene: &mut Places| {
                    // As `App::central_panel` places and draws it.
                    let slot = ui.max_rect();
                    let block = PhotoFrame::new(slot, Margin::Points(24.0));
                    let ppp = ui.pixels_per_point();
                    let room = block.bounds();
                    let bounds = ((room.x * ppp).floor() as u32, (room.y * ppp).floor() as u32);
                    let open = matches!(scene.mirror.photo, PhotoState::Open(_));
                    let values = open.then_some(&scene.mirror.values);
                    let picture = scene.mirror.picture();
                    scene.placed =
                        place_photo(&block, bounds, ppp, values, picture.map(|(p, _)| p));
                    let painter = ui.painter_at(slot);
                    scene.drawn = match (picture, &scene.placed) {
                        (Some((shown, generation)), Some(placed)) => Some(paint_picture(
                            &painter,
                            placed,
                            shown,
                            generation,
                            &mut scene.picture,
                        )),
                        _ => None,
                    };
                },
                Places {
                    mirror,
                    picture: None,
                    drawn: None,
                    placed: None,
                },
            )
    }

    /// The photo's place, and where its picture was drawn, in the last frame.
    fn places(harness: &Harness<'_, Places>) -> (egui::Rect, Option<egui::Rect>) {
        let scene = harness.state();
        (scene.placed.expect("a place").photo, scene.drawn)
    }

    /// An opening photo's picture is scaled to the photo's place, upscaled when smaller
    /// than the room and downscaled when larger: before its values the picture's own shape
    /// fitted to the room, from them on the place the engine renders at, which the live
    /// frame covers (decision 77). The two agree within a pixel for pictures of 1024 pixels
    /// or more; a 128 or 256 pixel thumbnail's rounded shape may step as the values come.
    #[test]
    fn an_opening_photo_s_picture_covers_the_place_its_live_frame_takes() {
        use crate::mirror::tests::photo;
        // A 6000 by 4000 photo's picture from the cache, and its camera's, larger than the
        // room.
        for (width, height) in [(1024, 683), (2048, 1365)] {
            let mut harness = places_scene(width, height);
            harness.run();
            let (before, drawn) = places(&harness);
            assert_eq!(
                drawn,
                Some(before),
                "{width} by {height}, before its values"
            );
            assert_eq!(before.width() * 2.0, 1800.0, "{width} by {height}");
            let mirror = &mut harness.state_mut().mirror;
            mirror.apply(Event::Opened(Photo {
                path: PathBuf::from("IMG_2041.ARW"),
                ..photo(2)
            }));
            mirror.apply(Event::Values(Values {
                live: false,
                extent: [6000, 4000],
                ..photo_values(0.0, 0)
            }));
            harness.run();
            let (after, drawn) = places(&harness);
            assert_eq!(drawn, Some(after), "{width} by {height}, with its values");
            assert_eq!(after.size() * 2.0, egui::vec2(1800.0, 1200.0));
            // The picture's sides are rounded, its shape the photo's to within a pixel.
            let apart = (before.size() - after.size()) * 2.0;
            assert!(
                apart.abs().max_elem() <= 1.0,
                "{width} by {height}: {apart:?}"
            );
        }
    }

    /// A photo whose edit crops it to another shape than its picture's: from its values on,
    /// the picture is letterboxed inside the place the live frame covers, centred and
    /// smaller than it on one side, never stretched to it (decision 77).
    #[test]
    fn an_opening_photo_s_picture_of_another_shape_is_letterboxed_in_its_place() {
        use crate::mirror::tests::photo;
        for (width, height) in [(1024, 683), (2048, 1365)] {
            let mut harness = places_scene(width, height);
            let mirror = &mut harness.state_mut().mirror;
            mirror.apply(Event::Opened(Photo {
                path: PathBuf::from("IMG_2041.ARW"),
                ..photo(2)
            }));
            // A square crop of the 3:2 photo.
            mirror.apply(Event::Values(Values {
                live: false,
                extent: [4000, 4000],
                ..photo_values(0.0, 0)
            }));
            harness.run();
            let (place, drawn) = places(&harness);
            let drawn = drawn.expect("the picture drawn");
            assert!(
                (place.width() - place.height()).abs() * 2.0 <= 1.0,
                "{place:?}"
            );
            assert!(place.contains_rect(drawn), "{drawn:?} in {place:?}");
            assert_eq!(drawn.width(), place.width(), "{width} by {height}");
            assert!(drawn.height() < place.height(), "{width} by {height}");
            let off = (drawn.center() - place.center()) * 2.0;
            assert!(off.abs().max_elem() <= 1.0, "{width} by {height}: {off:?}");
            // Its own shape, within a pixel.
            let shape = drawn.width() / drawn.height();
            assert!(
                (shape - width as f32 / height as f32).abs() < 0.01,
                "{width} by {height}: {shape}"
            );
        }
    }

    #[test]
    fn while_the_next_photo_opens_the_panel_draws_no_photo_until_its_values_arrive() {
        use crate::mirror::tests::photo;
        let report = photo_values;
        let folder = settings::testing::Folder::new("panel-switch");
        let mut harness = editing(&folder);
        let mirror = &mut harness.state_mut().mirror;
        mirror.apply(Event::Opened(photo(1)));
        mirror.apply(Event::Values(report(1.5, 1)));
        mirror.apply(Event::Scopes(scopes::testing::reading(true, false, false)));
        harness.run();
        assert_eq!(
            exposure_slider(&harness),
            (String::from("+1.500 EV"), false)
        );
        harness.get_by_role_and_label(Role::Button, "Scopes");
        // Another photo is asked for: every frame from the next until its values arrive
        // draws no photo, never a panel without sections.
        harness
            .state_mut()
            .mirror
            .opening(PathBuf::from("/photos/b.ARW"), 2, Instant::now());
        for step in 0..3 {
            if step == 1 {
                harness.state_mut().mirror.apply(Event::Opened(photo(2)));
            }
            harness.run();
            draws_no_photo(&harness, BASE);
            assert_eq!(exposure_slider(&harness), (String::from("+0.000 EV"), true));
        }
        // Its values arrive: the next frame is the next photo's, Scopes with it.
        harness
            .state_mut()
            .mirror
            .apply(Event::Values(report(0.5, 2)));
        harness.run();
        assert_eq!(
            exposure_slider(&harness),
            (String::from("+0.500 EV"), false)
        );
        assert_eq!(graph_says_in(&harness, "Histogram"), "Nothing measured yet");
        // Every tab's sections, live, keep ids of their own: the Effect tab's three among
        // them, whose sliders share names.
        for tab in [2, 3, 4] {
            harness.state_mut().shell.panel_tab = tab;
            harness.run();
            for title in titles(tab) {
                harness.get_by_role_and_label(Role::Button, &title);
            }
        }
    }

    /// What the scopes' graph, `name`, says to screen readers, in the editing tab.
    fn graph_says_in(harness: &Harness<'_, Editing>, name: &str) -> String {
        let graph = harness.get_by_role_and_label(Role::Image, name);
        let said = graph.accesskit_node().description();
        said.unwrap_or_default().to_owned()
    }

    #[test]
    fn scopes_are_pinned_over_the_base_tab_s_modules_and_count_the_histogram() {
        let folder = settings::testing::Folder::new("scopes-pinned");
        let mut harness = side(&folder);
        harness.run();
        assert_eq!(harness.state().shell.panel_tab, BASE);
        let scopes = harness.get_by_role_and_label(Role::Button, "Scopes").rect();
        let exposure = harness
            .get_by_role_and_label(Role::Button, "Exposure")
            .rect();
        let graph = harness
            .get_by_role_and_label(Role::Image, "Histogram")
            .rect();
        assert!(
            scopes.bottom() <= graph.top() && graph.bottom() <= exposure.top(),
            "Scopes {scopes:?}, its graph {graph:?}, Exposure {exposure:?}"
        );
        assert_eq!(graph_says(&harness, "Histogram"), "Nothing measured yet");
        // Every graph can be chosen; the overlays wait for the app to draw them.
        let vectorscope = harness.get_by_role_and_label(Role::RadioButton, "Vectorscope");
        assert!(!vectorscope.accesskit_node().is_disabled());
        for name in ["Show shadow clipping", "Show highlight clipping"] {
            let control = harness.get_by_label(name).accesskit_node();
            assert!(control.is_disabled(), "{name}");
        }
        assert_eq!(harness.state().sent, [counted(true, false, false)]);
        // Every editing tab pins it.
        harness.get_by_role_and_label(Role::Tab, "Correct").click();
        harness.run();
        assert_eq!(harness.state().shell.panel_tab, 3);
        harness.get_by_role_and_label(Role::Button, "Scopes");
        assert_eq!(harness.state().sent, [counted(true, false, false)]);
    }

    #[test]
    fn the_latest_reading_reaches_the_scopes() {
        let folder = settings::testing::Folder::new("scopes-reading");
        let mut harness = side(&folder);
        harness.run();
        let reading = scopes::testing::reading(true, false, false);
        harness.state_mut().scopes.receive(&reading);
        harness.run();
        assert_eq!(
            graph_says(&harness, "Histogram"),
            "Shadows clipped: 0.01%, highlights clipped: 5%"
        );
        // The waveform chosen, the reading in hand has none until the next arrives.
        harness
            .get_by_role_and_label(Role::RadioButton, "Waveform")
            .click();
        harness.run();
        assert_eq!(graph_says(&harness, "Waveform"), "Nothing measured yet");
        let reading = scopes::testing::reading(false, true, false);
        harness.state_mut().scopes.receive(&reading);
        harness.run();
        assert_eq!(
            graph_says(&harness, "Waveform"),
            "Shadows clipped: 0.01%, highlights clipped: 5%"
        );
        // A photo closing takes its reading with it.
        harness.state_mut().scopes.forget();
        harness.run();
        assert_eq!(graph_says(&harness, "Waveform"), "Nothing measured yet");
    }

    #[test]
    fn the_scopes_ask_again_as_the_graph_changes_and_the_section_opens_and_closes() {
        let folder = settings::testing::Folder::new("scopes-asking");
        let mut harness = side(&folder);
        harness.run();
        let sent = |harness: &Harness<'_, Side>| harness.state().sent.clone();
        assert_eq!(sent(&harness), [counted(true, false, false)]);
        // The waveform's counts serve the RGB parade too.
        for graph in ["Waveform", "RGB parade"] {
            harness
                .get_by_role_and_label(Role::RadioButton, graph)
                .click();
            harness.run();
        }
        assert_eq!(
            sent(&harness),
            [counted(true, false, false), counted(false, true, false)]
        );
        // Closed, the totals alone; open again, the graph shown.
        let graph = |harness: &Harness<'_, Side>| {
            let parade = harness.query_by_role_and_label(Role::Image, "RGB parade");
            parade.is_some()
        };
        for open in [false, true] {
            harness
                .get_by_role_and_label(Role::Button, "Scopes")
                .click();
            harness.run();
            assert_eq!(graph(&harness), open);
        }
        assert_eq!(
            sent(&harness),
            [
                counted(true, false, false),
                counted(false, true, false),
                counted(false, false, false),
                counted(false, true, false)
            ]
        );
    }

    #[test]
    fn the_vectorscope_is_counted_while_it_is_chosen_and_open_and_shown_on_the_next_run() {
        let folder = settings::testing::Folder::new("scopes-vectorscope");
        let mut harness = side(&folder);
        harness.run();
        harness
            .get_by_role_and_label(Role::RadioButton, "Vectorscope")
            .click();
        harness.run();
        let sent = |harness: &Harness<'_, Side>| harness.state().sent.clone();
        let vectorscope = counted(false, false, true);
        assert_eq!(
            sent(&harness),
            [counted(true, false, false), vectorscope.clone()]
        );
        // Closed, the totals alone; open again, the vectorscope.
        for _ in 0..2 {
            harness
                .get_by_role_and_label(Role::Button, "Scopes")
                .click();
            harness.run();
        }
        assert_eq!(
            sent(&harness)[2..],
            [counted(false, false, false), vectorscope.clone()]
        );
        // Kept in settings, the next run shows it and asks for it from the start.
        harness.state_mut().keeper.flush();
        drop(harness);
        let mut harness = side(&folder);
        harness.run();
        harness.get_by_role_and_label(Role::Image, "Vectorscope");
        assert_eq!(sent(&harness), [vectorscope]);
    }

    #[test]
    fn a_reading_with_the_vectorscope_reaches_its_graph() {
        let folder = settings::testing::Folder::new("scopes-vectorscope-reading");
        let mut harness = side(&folder);
        harness.run();
        harness
            .get_by_role_and_label(Role::RadioButton, "Vectorscope")
            .click();
        harness.run();
        // The vectorscope's counts, as a texture as tall as the graph.
        let drawn = |harness: &Harness<'_, Side>| {
            harness.output().shapes.iter().any(|clipped| {
                matches!(&clipped.shape, egui::Shape::Mesh(mesh)
                    if mesh.texture_id != egui::TextureId::default()
                        && mesh.calc_bounds().height() > 100.0)
            })
        };
        // A reading without it, as one counted before it was chosen, measures nothing.
        let histogram = scopes::testing::reading(true, false, false);
        harness.state_mut().scopes.receive(&histogram);
        harness.run();
        assert_eq!(graph_says(&harness, "Vectorscope"), "Nothing measured yet");
        assert!(!drawn(&harness));
        let reading = scopes::testing::reading(false, false, true);
        harness.state_mut().scopes.receive(&reading);
        harness.run();
        assert_eq!(
            graph_says(&harness, "Vectorscope"),
            "Shadows clipped: 0.01%, highlights clipped: 5%"
        );
        assert!(drawn(&harness));
    }

    #[test]
    fn the_grid_bar_s_choices_change_the_grid_and_survive_a_new_shell() {
        use apertrust_ui::components::{Sort, SortKey, ThumbnailSize};
        let folder = settings::testing::Folder::new("grid-kept");
        let (mut keeper, settings) = Keeper::open(folder.storage());
        let ctx = egui::Context::default();
        let mut shell = Shell::new(&settings, Locale::EnGb, OperatingSystem::Mac);
        // With no photos in the grid panel its items rest.
        let resting = shell.state(&ctx, false, false);
        assert!(shell.run(ActionId::SORT[1], &resting, &ctx, &[]).is_none());
        assert_eq!(shell.grid, settings::Grid::default());
        let shown = State {
            shown: true,
            ..resting
        };
        for id in [
            ActionId::SORT[1],
            ActionId::SORT_DESCENDING,
            ActionId::THUMBNAILS[3],
            // The grid bar's Labels, which turns the captions off.
            ActionId::SHOW_LABELS,
            // Not built: Rating changes nothing.
            ActionId::SORT[3],
        ] {
            shell.run(id, &shown, &ctx, &[]);
        }
        let chosen = settings::Grid {
            sort: Sort {
                key: SortKey::FileName,
                descending: true,
            },
            thumbnails: ThumbnailSize::ExtraLarge,
            labels: false,
            ..settings::Grid::default()
        };
        assert_eq!(shell.grid, chosen);
        // The menus check what the grid holds.
        let state = State {
            shown: true,
            ..shell.state(&ctx, false, false)
        };
        let checked = |id| actions::action(id).and_then(|a| a.checked(&state));
        assert_eq!(checked(ActionId::SORT[1]), Some(true));
        assert_eq!(checked(ActionId::SORT[0]), Some(false));
        assert_eq!(checked(ActionId::SORT_DESCENDING), Some(true));
        assert_eq!(checked(ActionId::THUMBNAILS[1]), Some(false));
        assert_eq!(checked(ActionId::SHOW_LABELS), Some(false));
        // The app ends, and starts again.
        keeper.frame(std::time::Instant::now(), &shell.to_settings(&ctx));
        keeper.flush();
        let (_, settings) = Keeper::open(folder.storage());
        assert_eq!(settings.grid, chosen);
        assert_eq!(
            Shell::new(&settings, Locale::EnGb, OperatingSystem::Mac).grid,
            chosen
        );
    }

    /// A frame as the app's: the surround's menu read in the central panel, and the
    /// frame's popups drawn after it.
    struct Surround {
        shell: Shell,
        chosen: Vec<ActionId>,
    }

    #[test]
    fn the_surround_s_drawn_menu_gives_the_background_chosen_in_it() {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(600.0, 400.0))
            .with_os(OperatingSystem::Nix)
            .with_context_setup(|ctx| {
                apertrust_ui::install(ctx, Theme::default());
                registry_keys_only(ctx);
            })
            .build_ui_state(
                |ui, surround: &mut Surround| {
                    let ctx = ui.ctx().clone();
                    let slot = ui.allocate_rect(ui.max_rect(), egui::Sense::click());
                    let shell = &surround.shell;
                    let chosen = surround_menu(&ctx, &slot, None, |at| {
                        let state = shell.state(&ctx, false, false);
                        menus::context(&ctx, &shell.keymap, &state, &menus::BACKGROUND, at)
                    });
                    surround.chosen.extend(chosen);
                    popup::show(&ctx);
                },
                Surround {
                    shell: Shell::new(&Settings::default(), Locale::EnGb, OperatingSystem::Nix),
                    chosen: Vec::new(),
                },
            );
        let at = egui::pos2(300.0, 200.0);
        harness.hover_at(at);
        harness.run();
        for pressed in [true, false] {
            harness.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: Modifiers::NONE,
            });
        }
        harness.run();
        harness
            .get_by_role_and_label(Role::MenuItemCheckBox, "White")
            .click();
        harness.run();
        assert_eq!(harness.state().chosen, [ActionId::BACKGROUND[5]]);
    }

    #[test]
    fn the_background_follows_the_theme_until_chosen_and_is_kept() {
        let folder = settings::testing::Folder::new("background-kept");
        let (mut keeper, settings) = Keeper::open(folder.storage());
        let ctx = egui::Context::default();
        let mut shell = Shell::new(&settings, Locale::EnGb, OperatingSystem::Mac);
        prepare(&ctx, &shell, 1.0);
        let state = shell.state(&ctx, false, false);
        let checked = |state: &State, id| actions::action(id).and_then(|a| a.checked(state));
        // The dark theme's own surround is Dark, checked with nothing chosen.
        assert_eq!(state.background, Some(Background::Dark));
        assert_eq!(checked(&state, ActionId::BACKGROUND[2]), Some(true));
        // Light's is Medium.
        set_in_settings(&mut shell, &mut keeper, "theme", "light", &ctx);
        let state = shell.state(&ctx, false, false);
        assert_eq!(state.background, Some(Background::Medium));
        // Chosen with no photo open, it stays through a change of theme.
        shell.run(ActionId::BACKGROUND[5], &state, &ctx, &[]);
        assert_eq!(theme(&ctx).colors.photo_surround, egui::Color32::WHITE);
        set_in_settings(&mut shell, &mut keeper, "theme", "dark", &ctx);
        let state = shell.state(&ctx, false, false);
        assert_eq!(checked(&state, ActionId::BACKGROUND[5]), Some(true));
        assert_eq!(checked(&state, ActionId::BACKGROUND[2]), Some(false));
        // The caption turns dark on it.
        assert_eq!(theme(&ctx).colors.text_on_surround(), egui::Color32::BLACK);
        keeper.frame(std::time::Instant::now(), &shell.to_settings(&ctx));
        keeper.flush();
        let (_, settings) = Keeper::open(folder.storage());
        assert_eq!(settings.central_panel.background, Some(Background::White));
    }

    #[test]
    fn the_grid_panel_s_rows_are_kept_and_never_more_than_half_the_window() {
        use apertrust_ui::components::{ThumbnailSize, grid_panel_height};
        let folder = settings::testing::Folder::new("grid-rows");
        let (mut keeper, settings) = Keeper::open(folder.storage());
        let ctx = egui::Context::default();
        apertrust_ui::install(&ctx, Theme::default());
        let mut shell = Shell::new(&settings, Locale::EnGb, OperatingSystem::Mac);
        shell.grid.position = apertrust_ui::components::GridPosition::Bottom;
        assert_eq!(shell.grid.rows, 1);
        shell.resize_grid(GridEdgeAction::Resize(3));
        keeper.frame(std::time::Instant::now(), &shell.to_settings(&ctx));
        keeper.flush();
        let (_, settings) = Keeper::open(folder.storage());
        assert_eq!(settings.grid.rows, 3);
        let mut shell = Shell::new(&settings, Locale::EnGb, OperatingSystem::Mac);
        // A window where two rows of Medium fit in half its height shows two, keeping three.
        let size = ThumbnailSize::Medium;
        let window = 2.0 * grid_panel_height(&ctx, size, true, 2) + 1.0;
        assert_eq!(
            shell
                .grid_edge(&ctx, egui::vec2(1600.0, window))
                .lines(&ctx),
            2
        );
        assert_eq!(shell.grid.rows, 3);
        // The rows count the captions: without them five fit in half a window where four
        // rows with them do.
        let window = 2.0 * grid_panel_height(&ctx, size, true, 4) + 1.0;
        shell.resize_grid(GridEdgeAction::Resize(9));
        let rows = |shell: &Shell| {
            shell
                .grid_edge(&ctx, egui::vec2(1600.0, window))
                .lines(&ctx)
        };
        assert_eq!(rows(&shell), 4);
        shell.grid.labels = false;
        assert_eq!(rows(&shell), 5);
        shell.grid.labels = true;
        shell.resize_grid(GridEdgeAction::Resize(3));
        assert_eq!(
            shell
                .grid_edge(&ctx, egui::vec2(1600.0, 3000.0))
                .lines(&ctx),
            3
        );
        // However small, one; a reset goes back to it.
        assert_eq!(
            shell.grid_edge(&ctx, egui::vec2(1600.0, 10.0)).lines(&ctx),
            1
        );
        shell.resize_grid(GridEdgeAction::Reset);
        assert_eq!(shell.grid.rows, 1);
    }

    /// The window as the app's frame lays out the side panel and the grid panel, with
    /// `count` photos in the grid panel; what its grid bar chooses runs as the app runs it,
    /// and its edge resizes it as the app keeps it.
    struct Window {
        shell: Shell,
        count: usize,
        side: egui::Rect,
    }

    fn window(settings: &Settings, count: usize) -> Harness<'static, Window> {
        Harness::builder()
            .with_size(egui::vec2(1400.0, 900.0))
            .with_step_dt(1.0 / 60.0)
            .with_context_setup(|ctx| {
                apertrust_ui::install(ctx, Theme::default());
                registry_keys_only(ctx);
            })
            .build_ui_state(
                |ui, window: &mut Window| {
                    let ctx = ui.ctx().clone();
                    let side = egui::Panel::right("side panel")
                        .exact_size(320.0)
                        .resizable(false)
                        .show(ui, |_| {});
                    window.side = side.response.rect;
                    let shell = &mut window.shell;
                    let grid = shell.grid;
                    let (frame, edge, lines) = shell.grid_frame(&ctx);
                    let names: Vec<String> = (0..window.count)
                        .map(|i| format!("IMG_{i:04}.ARW"))
                        .collect();
                    let panel = frame.show(ui, |ui| {
                        let thumb = |i: usize| Thumb {
                            name: &names[i],
                            edited: false,
                            status: None,
                            selected: false,
                            picture: None,
                            rating: None,
                        };
                        GridPanel::new(names.len(), None, thumb)
                            .sort(grid.sort)
                            .size(grid.thumbnails)
                            .lines(lines)
                            .position(grid.position)
                            .labels(grid.labels)
                            .show(ui)
                            .bar
                    });
                    egui::CentralPanel::default().show(ui, |_| {});
                    if let Some(action) = edge.show(ui, panel.response.rect) {
                        shell.resize_grid(action);
                    }
                    if let Some(chosen) = panel.inner {
                        let state = State {
                            shown: !names.is_empty(),
                            ..shell.state(&ctx, false, false)
                        };
                        shell.run(ActionId(chosen.id()), &state, &ctx, &[]);
                    }
                },
                Window {
                    shell: Shell::new(settings, Locale::EnGb, OperatingSystem::Mac),
                    count,
                    side: egui::Rect::NOTHING,
                },
            )
    }

    fn grid_list(harness: &Harness<'_, Window>) -> egui::Rect {
        harness
            .get_by_role_and_label(Role::List, "Grid panel")
            .rect()
    }

    /// Right-clicks the grid bar's empty part, its middle between Sort at its start and
    /// Zoom at its far end, and chooses Position's `place`.
    fn move_grid(harness: &mut Harness<'_, Window>, place: &str) {
        let bar = harness
            .get_by_role_and_label(Role::Toolbar, "Grid bar")
            .rect();
        let at = bar.center();
        harness.hover_at(at);
        harness.run();
        for pressed in [true, false] {
            harness.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: Modifiers::NONE,
            });
        }
        harness.run();
        harness.get_by_label("Position").click();
        harness.run();
        harness
            .get_by_role_and_label(Role::MenuItemCheckBox, place)
            .click();
        harness.run();
        harness.run();
    }

    #[test]
    fn the_grid_bar_s_options_move_the_panel_and_each_place_keeps_its_lines() {
        use apertrust_ui::components::GridPosition;
        let settings = Settings {
            grid: settings::Grid {
                rows: 2,
                columns: 3,
                position: GridPosition::Bottom,
                ..settings::Grid::default()
            },
            ..Settings::default()
        };
        let mut harness = window(&settings, 30);
        harness.run();
        // At the bottom, under the photo and beside the side panel, two rows high; the
        // harness's ui keeps 8 points round the window.
        let list = grid_list(&harness);
        let side = harness.state().side;
        assert_eq!(list.right(), side.left());
        assert_eq!(list.bottom(), 900.0 - 8.0);
        let height = |harness: &Harness<'_, Window>| {
            harness
                .get_by_role_and_label(Role::Splitter, "Grid panel height")
                .accesskit_node()
                .numeric_value()
        };
        assert_eq!(height(&harness), Some(2.0));
        // At the right, a column between the photo and the side panel, three columns wide.
        move_grid(&mut harness, "Right");
        assert_eq!(harness.state().shell.grid.position, GridPosition::Right);
        let list = grid_list(&harness);
        assert_eq!(list.right(), side.left());
        assert_eq!(list.top(), 8.0);
        let width = |harness: &Harness<'_, Window>| {
            harness
                .get_by_role_and_label(Role::Splitter, "Grid panel width")
                .accesskit_node()
                .numeric_value()
        };
        assert_eq!(width(&harness), Some(3.0));
        // At the left, at the window's left edge.
        move_grid(&mut harness, "Left");
        let list = grid_list(&harness);
        assert_eq!(list.left(), 8.0);
        assert_eq!(width(&harness), Some(3.0));
        // Back at the bottom with its two rows; the place is kept.
        move_grid(&mut harness, "Bottom");
        assert_eq!(height(&harness), Some(2.0));
        let ctx = egui::Context::default();
        let kept = harness.state().shell.to_settings(&ctx).grid;
        assert_eq!(kept.position, GridPosition::Bottom);
        assert_eq!((kept.rows, kept.columns), (2, 3));
    }

    #[test]
    fn the_grid_bar_s_labels_turns_the_captions_off_and_the_panel_loses_their_height() {
        use apertrust_ui::components::{GridPosition, ThumbnailSize, grid_panel_height};
        // At the bottom, where the captions add to the panel's height.
        let settings = Settings {
            grid: settings::Grid {
                position: GridPosition::Bottom,
                ..settings::Grid::default()
            },
            ..Settings::default()
        };
        let mut harness = window(&settings, 30);
        harness.run();
        let ctx = harness.ctx.clone();
        let size = ThumbnailSize::Medium;
        assert_eq!(
            grid_list(&harness).height(),
            grid_panel_height(&ctx, size, true, 1)
        );
        let labels = harness.get_by_role_and_label(Role::Button, "Labels");
        assert_eq!(labels.accesskit_node().toggled(), Some(Toggled::True));
        labels.click();
        harness.run();
        harness.run();
        assert!(!harness.state().shell.grid.labels);
        assert_eq!(
            grid_list(&harness).height(),
            grid_panel_height(&ctx, size, false, 1)
        );
        let labels = harness.get_by_role_and_label(Role::Button, "Labels");
        assert_eq!(labels.accesskit_node().toggled(), Some(Toggled::False));
        // Kept for the next start.
        let kept = harness.state().shell.to_settings(&egui::Context::default());
        assert!(!kept.grid.labels);
    }

    #[test]
    fn the_grid_panel_s_columns_are_never_more_than_half_the_window_s_width() {
        use apertrust_ui::components::{GridPosition, ThumbnailSize, grid_panel_width};
        let ctx = egui::Context::default();
        apertrust_ui::install(&ctx, Theme::default());
        let settings = Settings {
            grid: settings::Grid {
                position: GridPosition::Left,
                columns: 9,
                ..settings::Grid::default()
            },
            ..Settings::default()
        };
        let shell = Shell::new(&settings, Locale::EnGb, OperatingSystem::Mac);
        let size = ThumbnailSize::Medium;
        let window = egui::vec2(2.0 * grid_panel_width(&ctx, size, 4) + 1.0, 400.0);
        assert_eq!(shell.grid_edge(&ctx, window).lines(&ctx), 4);
        assert_eq!(shell.grid.columns, 9);
    }

    /// Every choice of the grid bar's menus is an action of the registry, so View's items
    /// run the same.
    #[test]
    fn every_grid_bar_choice_is_an_action() {
        use apertrust_ui::components::GridBarAction;
        for chosen in GridBarAction::ALL {
            let action = actions::action(ActionId(chosen.id())).expect("declared");
            assert_eq!(action.message, format!("action-{}", chosen.id()));
        }
    }

    #[test]
    fn the_scopes_view_and_whether_they_are_open_survive_a_new_shell() {
        let folder = settings::testing::Folder::new("scopes-kept");
        let mut harness = side(&folder);
        harness.run();
        harness
            .get_by_role_and_label(Role::RadioButton, "RGB parade")
            .click();
        harness.run();
        harness.get_by_label("Blue channel").click();
        harness.run();
        harness
            .get_by_role_and_label(Role::Button, "Scopes")
            .click();
        harness.run();
        // The app ends, and starts again.
        harness.state_mut().keeper.flush();
        drop(harness);
        let (_, settings) = Keeper::open(folder.storage());
        let parade = ScopeView {
            scope: apertrust_ui::components::Scope::Parade,
            channels: [true, true, false, false],
            ..ScopeView::default()
        };
        assert_eq!(settings.scopes, parade);
        let shell = Shell::new(&settings, Locale::EnGb, OperatingSystem::Mac);
        assert_eq!(shell.scopes, parade);
        assert!(!shell.section_open(scopes::ID, scopes::OPEN));
        // Closed, the next run asks for the totals alone.
        assert_eq!(shell.scopes_wanted(), Some(Graphs::default()));
    }

    #[test]
    fn the_system_s_language_is_kept_only_once_the_user_chooses_one() {
        let shell = Shell::new(&Settings::default(), Locale::Pl, OperatingSystem::Nix);
        assert_eq!(shell.locale, Locale::Pl);
        let ctx = egui::Context::default();
        assert_eq!(shell.to_settings(&ctx).locale, None);
        let chosen = Settings {
            locale: Some(Locale::Uk),
            ..Settings::default()
        };
        let shell = Shell::new(&chosen, Locale::Pl, OperatingSystem::Nix);
        assert_eq!(shell.locale, Locale::Uk);
        assert_eq!(shell.to_settings(&ctx).locale, Some(Locale::Uk));
    }

    #[test]
    fn a_key_from_settings_runs_its_action_and_a_clashing_one_is_refused() {
        let settings = Settings {
            keys: BTreeMap::from([
                ("settings".to_owned(), vec!["Ctrl+K".to_owned()]),
                // Ctrl+Q quits.
                ("full-screen".to_owned(), vec!["Ctrl+Q".to_owned()]),
            ]),
            ..Settings::default()
        };
        let mut harness = bench_with(OperatingSystem::Windows, &settings);
        harness.run();
        harness.key_press_modifiers(Modifiers::COMMAND, Key::Comma);
        harness.run();
        assert!(!harness.state().settings_open());
        let keymap = &harness.state().shell.keymap;
        let keys: Vec<_> = keymap
            .keys(ActionId::SETTINGS)
            .iter()
            .map(|k| k.text(OperatingSystem::Windows))
            .collect();
        assert_eq!(keys, ["Ctrl+K"]);
        harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
        harness.run();
        assert!(harness.state().settings_open());
        let keymap = &harness.state().shell.keymap;
        assert_eq!(
            keymap.keys(ActionId::FULL_SCREEN),
            Keymap::defaults(OperatingSystem::Windows).keys(ActionId::FULL_SCREEN)
        );
        // Both stay in settings as they were.
        assert_eq!(keymap.overrides(), &settings.keys);
    }

    /// A frame's input for a window at (x, 30) sized `w` by 700 logical pixels on a display
    /// of scale factor 2, in egui's points at the interface's `scale`.
    fn window_frame(
        maximized: bool,
        fullscreen: bool,
        x: f32,
        w: f32,
        scale: f32,
    ) -> egui::RawInput {
        let points = |x: f32, y: f32| egui::vec2(x, y) / scale;
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                points(w, 700.0),
            )),
            ..Default::default()
        };
        let viewport = input.viewports.entry(egui::ViewportId::ROOT).or_default();
        viewport.native_pixels_per_point = Some(2.0);
        viewport.maximized = Some(maximized);
        viewport.fullscreen = Some(fullscreen);
        viewport.outer_rect = Some(egui::Rect::from_min_size(
            points(x, 30.0).to_pos2(),
            points(w, 730.0),
        ));
        input
    }

    /// Runs a frame of `ctx` with `input`, without a painter to take the font atlas.
    fn run_frame(ctx: &egui::Context, input: egui::RawInput, f: impl FnMut(&mut egui::Ui)) {
        ctx.run_ui(input, f).textures_delta.clear();
    }

    #[test]
    fn a_maximised_or_full_screen_window_keeps_its_last_normal_placement() {
        let mut shell = Shell::new(&Settings::default(), Locale::EnGb, OperatingSystem::Mac);
        let ctx = egui::Context::default();
        let run =
            |shell: &mut Shell, input| run_frame(&ctx, input, |ui| shell.track_window(ui.ctx()));
        run(&mut shell, window_frame(false, false, 40.0, 1200.0, 1.0));
        // The position in physical pixels, with the scale factor that gives points.
        let normal = settings::Window {
            size: Some([1200.0, 700.0]),
            position: Some(settings::Position {
                pixels: [80.0, 60.0],
                scale_factor: 2.0,
            }),
            maximized: false,
        };
        assert_eq!(shell.window, normal);
        run(&mut shell, window_frame(true, false, 0.0, 1440.0, 1.0));
        if MAXIMISES {
            let maximised = settings::Window {
                maximized: true,
                ..normal
            };
            assert_eq!(shell.window, maximised);
        } else {
            // macOS's zoom is not followed: the window just has another size and place.
            assert!(!shell.window.maximized);
            assert_eq!(shell.window.size, Some([1440.0, 700.0]));
        }
        let before = shell.window;
        run(&mut shell, window_frame(false, true, 0.0, 1600.0, 1.0));
        assert_eq!(shell.window, before);
        // At a scale of 1.5 the window has the same logical size and place, in fewer points.
        ctx.options_mut(|o| o.zoom_factor = 1.5);
        run(&mut shell, window_frame(false, false, 40.0, 1200.0, 1.5));
        assert_eq!(shell.window, normal);
    }

    #[test]
    fn the_first_frame_after_a_scale_is_restored_is_at_that_scale() {
        let settings = Settings {
            scale: 1.5,
            ..Settings::default()
        };
        let shell = Shell::new(&settings, Locale::EnGb, OperatingSystem::Windows);
        let ctx = egui::Context::default();
        prepare(&ctx, &shell, settings.scale);
        // A zoom set with `set_zoom_factor` would apply from the second frame, the first
        // given egui's default screen of 10000 points scaled instead of the window's.
        let mut first = None;
        run_frame(&ctx, window_frame(false, false, 40.0, 1200.0, 1.5), |ui| {
            let screen = ui.ctx().input(|i| i.content_rect().size());
            first = Some((ui.ctx().zoom_factor(), screen));
        });
        assert_eq!(first, Some((1.5, egui::vec2(1200.0, 700.0) / 1.5)));
        // So the first frame's settings are those read, and nothing is written.
        assert_eq!(shell.to_settings(&ctx).scale, settings.scale);
    }

    #[test]
    fn a_window_opens_on_a_monitor_that_is_still_there() {
        let monitor = |x: f32, w: f32, h: f32, scale_factor: f32, primary: bool| Monitor {
            rect: egui::Rect::from_min_size(egui::pos2(x, 0.0), egui::vec2(w, h)),
            scale_factor,
            primary,
        };
        let laptop = monitor(0.0, 2880.0, 1800.0, 2.0, true);
        let screen = monitor(2880.0, 3840.0, 2160.0, 1.0, false);
        let at = |x: f32, y: f32, scale_factor: f32| settings::Position {
            pixels: [x, y],
            scale_factor,
        };
        let size = [1200.0, 700.0];
        // On macOS, in points; a window on a monitor since unplugged goes where the system puts it.
        let both = [laptop, screen];
        let on_mac = on_screen(&at(3000.0, 100.0, 1.0), size, &both, true);
        assert_eq!(on_mac, Some(Spot::Points(egui::pos2(3000.0, 100.0))));
        let laptop = monitor(0.0, 2880.0, 1800.0, 2.0, true);
        let alone = [laptop];
        assert_eq!(on_screen(&at(3000.0, 100.0, 1.0), size, &alone, true), None);
        // On Windows, in pixels, kept on its monitor or brought to the primary one.
        let on_windows = on_screen(&at(3000.0, 100.0, 1.0), size, &both, false);
        assert_eq!(on_windows, Some(Spot::Pixels(egui::pos2(3000.0, 100.0))));
        let gone = on_screen(&at(3000.0, 100.0, 1.0), size, &alone, false);
        // 1200 by 732 points at a scale factor of 2, inside 2880 by 1800 pixels.
        assert_eq!(gone, Some(Spot::Pixels(egui::pos2(480.0, 100.0))));
        assert_eq!(on_screen(&at(0.0, 0.0, 1.0), size, &[], false), None);
    }

    /// The Mac of 1470 by 956 points at 2, the report's.
    fn mac_monitor() -> Monitor {
        Monitor {
            rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(2940.0, 1912.0)),
            scale_factor: 2.0,
            primary: true,
        }
    }

    /// A window kept as wide as the screen, zoomed on a Mac of 1470 by 956 points at 2,
    /// opens at that size and place: given its size again once it exists, as one created
    /// at about the screen's visible size came back 20 points smaller; at most the largest
    /// monitor's, each way apart, as eframe caps it; and at the size it is made at
    /// elsewhere.
    #[test]
    fn a_full_width_window_reopens_at_its_size_and_place() {
        let kept = settings::Window {
            size: Some([1470.0, 868.0]),
            position: Some(settings::Position {
                pixels: [0.0, 66.0],
                scale_factor: 2.0,
            }),
            maximized: false,
        };
        let position = kept.position.expect("a place");
        let size = inner_size(&kept, false);
        let monitors = [mac_monitor()];
        let spot = on_screen(&position, size, &monitors, true).expect("on the screen");
        assert_eq!(spot, Spot::Points(egui::pos2(0.0, 33.0)));
        let restored = size_on_screen(size, false, &monitors, true);
        assert_eq!(restored, Some([1470.0, 868.0]));
        // Larger than the screen, as when kept on a larger one.
        let larger = size_on_screen([1600.0, 1000.0], false, &monitors, true);
        assert_eq!(larger, Some([1470.0, 956.0]));
        // The main window never under its least; the start window at its own size.
        let small = size_on_screen([800.0, 500.0], false, &monitors, true);
        assert_eq!(small, Some(WINDOW_MIN));
        let start = start_placement(&kept, 1.0);
        let start_size = inner_size(&start, true);
        assert_eq!(start_size, [820.0, 520.0]);
        let start_restored = size_on_screen(start_size, true, &monitors, true);
        assert_eq!(start_restored, Some(start_size));
        // Elsewhere, and with no monitor known, the size it was made at stands.
        assert_eq!(size_on_screen(size, false, &monitors, false), None);
        assert_eq!(size_on_screen(size, false, &[], true), None);
    }

    /// On a first run the main window follows the start window at the default size, larger
    /// than the Mac's 1470 by 956 points: `reshape` gives it at most the screen's, as eframe
    /// gives a window it makes, so it never runs past the screen's edge to be kept so; the
    /// size it asks for stands elsewhere.
    #[test]
    fn the_main_window_after_the_start_window_fits_the_screen() {
        let first = settings::Window {
            size: None,
            position: None,
            maximized: false,
        };
        let size = inner_size(&first, false);
        assert_eq!(size, settings::registry::WINDOW_SIZE.default());
        let monitors = [mac_monitor()];
        let fitted = size_on_screen(size, false, &monitors, true);
        assert_eq!(fitted, Some([1470.0, 956.0]));
        assert_eq!(size_on_screen(size, false, &monitors, false), None);
    }

    /// With a second monitor at a scale of 1 to the right of the Mac's: a corner on it is
    /// found on its rectangle in points, and the size is capped at the largest width and
    /// the largest height of the two, as eframe caps it. winit gives a Mac's monitor's corner
    /// as its place in points times its own scale factor, so this one's, 1470 points to the
    /// right, is at 1470 pixels.
    #[test]
    fn a_window_on_a_second_monitor_is_capped_at_the_largest_one() {
        let screen = Monitor {
            rect: egui::Rect::from_min_size(egui::pos2(1470.0, 0.0), egui::vec2(2560.0, 1440.0)),
            scale_factor: 1.0,
            primary: false,
        };
        let points = egui::Rect::from_min_max(egui::pos2(1470.0, 0.0), egui::pos2(4030.0, 1440.0));
        assert_eq!(screen.points(), points);
        let monitors = [mac_monitor(), screen];
        let corner = settings::Position {
            pixels: [3000.0, 100.0],
            scale_factor: 1.0,
        };
        let spot = on_screen(&corner, [1200.0, 700.0], &monitors, true);
        assert_eq!(spot, Some(Spot::Points(egui::pos2(3000.0, 100.0))));
        let capped = size_on_screen([3000.0, 2000.0], false, &monitors, true);
        assert_eq!(capped, Some([2560.0, 1440.0]));
        // A height the Mac's screen alone would cut, under the second's.
        let tall = size_on_screen([1000.0, 1200.0], false, &monitors, true);
        assert_eq!(tall, Some([1000.0, 1200.0]));
    }

    /// The start window's size and place never reach the main window's kept placement: not
    /// while it shows, nor while the main window, given its own back, still has the start
    /// window's size; the main window's own size is kept once it has it.
    #[test]
    fn the_start_window_leaves_the_main_window_s_placement_as_kept() {
        let folder = settings::testing::Folder::new("start-placement");
        let (mut keeper, settings) = Keeper::open(folder.storage());
        let kept = settings::Window {
            size: Some([1470.0, 868.0]),
            position: Some(settings::Position {
                pixels: [0.0, 66.0],
                scale_factor: 2.0,
            }),
            maximized: false,
        };
        let settings = Settings {
            window: kept,
            ..settings
        };
        let mut shell = Shell::new(&settings, Locale::EnGb, OperatingSystem::Mac);
        let mut start = start::Window::new(true, 1.0);
        let ctx = egui::Context::default();
        // A frame of the window at `size` points, its corner at `at`, at a scale factor of 2.
        let frame = |size: egui::Vec2, at: egui::Pos2| {
            let mut input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                ..Default::default()
            };
            let viewport = input.viewports.entry(egui::ViewportId::ROOT).or_default();
            viewport.native_pixels_per_point = Some(2.0);
            viewport.maximized = Some(false);
            viewport.fullscreen = Some(false);
            viewport.outer_rect = Some(egui::Rect::from_min_size(at, size));
            input
        };
        let mut run = |shell: &mut Shell, start: &mut start::Window, input| {
            run_frame(&ctx, input, |ui| {
                let size = ui.ctx().input(|i| i.viewport_rect().size()) * ui.ctx().zoom_factor();
                let tracks = start.keeps_placement(size);
                keep(shell, &mut keeper, ui.ctx(), tracks);
            });
        };
        let centred = egui::pos2(325.0, 207.0);
        run(&mut shell, &mut start, frame(start::SIZE, centred));
        assert_eq!(shell.window, kept);
        // Something opens: the main window, still at the start window's size for a frame.
        start.shown = false;
        start.settling = Some(start::SIZE);
        run(&mut shell, &mut start, frame(start::SIZE, centred));
        assert_eq!(shell.window, kept);
        // Then at its own, which it keeps from then on.
        let own = egui::vec2(1470.0, 868.0);
        run(&mut shell, &mut start, frame(own, egui::pos2(0.0, 33.0)));
        assert_eq!(shell.window, kept);
        let narrower = egui::vec2(1200.0, 868.0);
        run(
            &mut shell,
            &mut start,
            frame(narrower, egui::pos2(0.0, 33.0)),
        );
        assert_eq!(shell.window.size, Some([1200.0, 868.0]));
    }

    #[test]
    fn the_pseudo_locale_is_only_a_debug_build_s() {
        assert!(offered(Locale::Pseudo, true));
        assert!(!offered(Locale::Pseudo, false));
        assert!(offered(Locale::Uk, false));
    }

    /// The shell as settings keeping `tab` as Edit's open it.
    fn opened_on(tab: &str) -> Shell {
        let settings = Settings {
            edit: settings::ViewState {
                tab: Some(tab.to_owned()),
                ..settings::ViewState::default()
            },
            ..Settings::default()
        };
        Shell::new(&settings, Locale::EnGb, OperatingSystem::Nix)
    }

    #[test]
    fn a_tab_kept_by_its_id_opens_and_is_kept_again() {
        let ctx = egui::Context::default();
        for (tab, id) in TAB_IDS.iter().enumerate().filter(|(i, _)| TABS_ENABLED[*i]) {
            let shell = opened_on(id);
            assert_eq!(shell.panel_tab, tab, "{id}");
            assert_eq!(shell.to_settings(&ctx).edit.tab.as_deref(), Some(*id));
        }
        let mut shell = opened_on("base");
        shell.panel_tab = 3;
        assert_eq!(shell.to_settings(&ctx).edit.tab.as_deref(), Some("correct"));
        assert_eq!(
            Shell::new(&Settings::default(), Locale::EnGb, OperatingSystem::Nix).panel_tab,
            BASE
        );
        assert_eq!(TAB_IDS.len(), TABS.len());
    }

    /// An older build's tab (decision 38 replaced Profile, Adjust, Optics and Detail, and
    /// `library` was the Photos tab's id) is read as any unknown id is, under decision 43:
    /// Base is shown, and the id stays in settings until the user chooses a tab.
    #[test]
    fn a_tab_this_build_lacks_stays_in_settings_until_the_user_chooses_one() {
        let ctx = egui::Context::default();
        for old in [
            "profile", "adjust", "optics", "detail", "presets", "library",
        ] {
            let mut shell = opened_on(old);
            assert_eq!(shell.panel_tab, BASE, "{old}");
            assert_eq!(shell.to_settings(&ctx).edit.tab.as_deref(), Some(old));
            shell.panel_tab = 2;
            assert_eq!(shell.to_settings(&ctx).edit.tab.as_deref(), Some("color"));
        }
    }

    /// A tab kept in settings that cannot be chosen yet, History, opens Base, its id kept
    /// until the user chooses another; the Photos tab's opens the Photos tab.
    #[test]
    fn a_kept_tab_that_cannot_be_chosen_opens_base_and_stays_kept() {
        let ctx = egui::Context::default();
        assert_eq!(opened_on("photos").panel_tab, PHOTOS);
        let mut shell = opened_on("history");
        assert_eq!(shell.panel_tab, BASE);
        assert_eq!(shell.to_settings(&ctx).edit.tab.as_deref(), Some("history"));
        shell.panel_tab = 4;
        assert_eq!(shell.to_settings(&ctx).edit.tab.as_deref(), Some("effect"));
    }

    /// A folder opened joins the recents at the top, once; Browse's rows say why one cannot
    /// be opened, its failure this run before the check's finding it gone.
    #[test]
    fn a_folder_opened_joins_the_recents_and_a_missing_one_says_why() {
        let mut shell = opened_on("photos");
        let (a, b) = (PathBuf::from("/photos/a"), PathBuf::from("/photos/b"));
        for path in [&a, &b, &a] {
            shell.remember_folder(path.clone());
        }
        let shown = |shell: &Shell| -> Vec<(PathBuf, Option<String>)> {
            let rows = shell.recent_folders();
            rows.iter()
                .map(|r| (r.path.to_owned(), r.missing.map(|m| format!("{m:?}"))))
                .collect()
        };
        assert_eq!(shown(&shell), [(a.clone(), None), (b.clone(), None)]);
        shell.recents.mark(recents::OpenKind::Folder, &b, false);
        shell.missing.insert(a.clone(), "refused".to_owned());
        shell.recents.mark(recents::OpenKind::Folder, &a, false);
        let failed = Some(format!("{:?}", library::Missing::Failed("refused")));
        let gone = Some(format!("{:?}", library::Missing::Gone));
        assert_eq!(shown(&shell), [(a.clone(), failed), (b, gone)]);
        // Opened again, it is neither.
        shell.remember_folder(a.clone());
        assert_eq!(shown(&shell)[0], (a, None));
    }

    /// Through a real session: a way in opens the folder as the root and enters the
    /// recents; a folder chosen in its tree is opened by the library, the grid panel showing
    /// it, but the root stays and its `Opened` asks nothing of the recents.
    #[test]
    fn a_folder_chosen_in_the_tree_is_opened_but_keeps_the_root_and_the_recents() {
        use apertrust_engine::gpu::GpuOptions;
        use apertrust_engine::session::LibraryEvent;
        let Ok(gpu) = pollster::block_on(GpuContext::new(&GpuOptions::default())) else {
            eprintln!("no GPU adapter: a session needs one");
            return;
        };
        let (woken, wakes) = std::sync::mpsc::channel();
        let session = apertrust_engine::session::testing::start(
            gpu,
            apertrust_operations::OPERATIONS,
            apertrust_modules::MODULES,
            "0.4.0+12~g1a2b3c4",
            move || {
                let _ = woken.send(());
            },
        );
        let mut mirror = Mirror::new(apertrust_modules::MODULES, jobs::Mirror::new(None));
        let root =
            std::env::temp_dir().join(format!("apertrust-app-choose-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let day = root.join("Day 1");
        std::fs::create_dir_all(&day).unwrap();
        // The reactions to the session's events, but the jobs', until the open numbered `open`
        // says it has opened.
        let until_opened = |mirror: &mut Mirror, open: u64| {
            let deadline = Instant::now() + std::time::Duration::from_secs(60);
            let mut reactions = Vec::new();
            loop {
                assert!(Instant::now() < deadline, "opened in time");
                let _ = wakes.recv_timeout(std::time::Duration::from_millis(50));
                let mut done = false;
                for event in session.events() {
                    done |= matches!(&event, Event::Library(LibraryEvent::Opened { open: o, .. }) if *o == open);
                    let folders = mirror.apply(event).into_iter();
                    reactions.extend(folders.filter(|r| !matches!(r, Reaction::Jobs(_))));
                }
                if done {
                    return reactions;
                }
            }
        };

        open_folder(root.clone(), &mut mirror, &mut |c| session.send(c));
        let way_in = mirror.folder.latest();
        let opened = until_opened(&mut mirror, way_in);
        assert_eq!(opened, [Reaction::FolderOpened(root.clone())]);

        choose_folder(day.clone(), &mut mirror, &mut |c| session.send(c));
        let chosen = mirror.folder.latest();
        assert_eq!(mirror.folder.root(), Some(root.as_path()));
        let reactions = until_opened(&mut mirror, chosen);
        assert!(reactions.is_empty(), "{reactions:?}");
        // The library opened the chosen folder for the grid panel.
        let folder::Folder::Open(listing) = &mirror.folder.folder else {
            panic!("{:?}", mirror.folder.folder);
        };
        assert_eq!(listing.path, day);
        assert_eq!(mirror.folder.root(), Some(root.as_path()));
        drop(session);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The tree keeps what was opened for the run: the root starts open and the others
    /// closed, and each toggle flips it.
    #[test]
    fn the_tree_keeps_what_was_opened_for_the_run() {
        let mut shell = opened_on("photos");
        let (open, sub) = (
            Path::new("/photos/Wedding"),
            Path::new("/photos/Wedding/Edits"),
        );
        assert!(!shell.toggle_tree(open, true));
        assert!(shell.toggle_tree(sub, false));
        assert!(shell.toggle_tree(open, true));
        assert_eq!(shell.tree.get(sub), Some(&true));
    }

    /// Remove from the List takes a recent out, and Locate…'s answer points it elsewhere in
    /// its place, no longer failed.
    #[test]
    fn a_missing_recent_is_removed_or_located() {
        let mut shell = opened_on("photos");
        let (a, b, c) = (
            PathBuf::from("/photos/a"),
            PathBuf::from("/photos/b"),
            PathBuf::from("/photos/c"),
        );
        for path in [&a, &b] {
            shell.remember_folder(path.clone());
        }
        shell.missing.insert(b.clone(), "refused".to_owned());
        shell.forget_recent(&b);
        let paths = |shell: &Shell| -> Vec<PathBuf> {
            shell
                .recent_folders()
                .iter()
                .map(|r| r.path.to_owned())
                .collect()
        };
        assert_eq!(paths(&shell), std::slice::from_ref(&a));
        assert!(!shell.missing.contains_key(&b));
        shell.missing.insert(a.clone(), "refused".to_owned());
        shell.located(&a, c.clone());
        assert_eq!(paths(&shell), [c]);
        assert!(shell.missing.is_empty());
    }

    /// Locate…'s answer, the platform's folder dialog's, points the missing recent to the
    /// folder chosen, which opens.
    #[test]
    fn locate_s_answer_points_the_recent_elsewhere_and_opens_it() {
        let mut harness = bench(OperatingSystem::Nix);
        let (gone, found) = (
            PathBuf::from("/photos/Gone"),
            PathBuf::from("/photos/Found"),
        );
        harness.state_mut().shell.remember_folder(gone.clone());
        harness.state_mut().shell.locate = Some(gone.clone());
        harness.run();
        let asked = harness.state().dialog.picks.last().cloned();
        assert_eq!(asked, Some(dialogs::Pick::Locate(gone.clone())));
        // It starts in the folder's parent, unchecked.
        assert_eq!(
            harness.state().dialog.asked.last(),
            Some(&Some(PathBuf::from("/photos")))
        );
        harness.state_mut().dialog.answer = Some(Some(vec![found.clone()]));
        harness.run();
        assert_eq!(
            harness.state().commands,
            vec![Command::Library(LibraryCommand::Open(Source::Folder(
                found.clone()
            )))]
        );
        let recents: Vec<PathBuf> = harness
            .state()
            .shell
            .recent_folders()
            .iter()
            .map(|r| r.path.to_owned())
            .collect();
        assert_eq!(recents, [found]);
    }

    /// Browse's Folders starts open, unlike a module's groups. A click on its row closes it,
    /// hiding its rows, and it is kept closed as `browse.folders` in `edit.groups`; a
    /// click on Browse's header closes Browse, kept as `browse` in `edit.sections`.
    #[test]
    fn a_click_on_folders_closes_it_and_keeps_it_closed() {
        let shell = opened_on("photos");
        let mut harness = Harness::builder()
            .with_size(egui::vec2(320.0, 600.0))
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
            .build_ui_state(
                |ui, shell: &mut Shell| {
                    let keys = library::Keys::default();
                    let browse = library::Browse {
                        folder: &folder::Folder::None,
                        root: None,
                        counts: &HashMap::new(),
                        recent: &[],
                        open: shell.browse_open(),
                        tree: &BTreeMap::new(),
                        inside: &std::collections::HashMap::new(),
                        keys: &keys,
                        home: None,
                    };
                    let mut actions = Vec::new();
                    PanelBody::new().show(ui, |ui| {
                        actions = library::browse(ui, &browse);
                    });
                    for action in actions {
                        match action {
                            library::Action::Toggle => shell.toggle_browse(false),
                            library::Action::ToggleFolders => shell.toggle_browse(true),
                            other => panic!("{other:?}"),
                        }
                    }
                },
                shell,
            );
        harness.run();
        assert!(harness.state().browse_open().folders);
        harness
            .get_by_role_and_label(Role::Button, "Folders")
            .click();
        harness.run();
        assert!(!harness.state().browse_open().folders);
        let ctx = egui::Context::default();
        let kept = harness.state().to_settings(&ctx).edit;
        let folders = kept.groups.get("browse").and_then(|g| g.get("folders"));
        assert_eq!(folders, Some(&false));
        harness
            .get_by_role_and_label(Role::Button, "Browse")
            .click();
        harness.run();
        let kept = harness.state().to_settings(&ctx).edit;
        assert_eq!(kept.sections.get("browse"), Some(&false));
    }

    /// The id settings had is forgotten once the user chooses a tab: one this build lacks
    /// or cannot choose yet, after Colour and then Base, leaves Base in the file, read
    /// as Base at the next start, never the id that warned.
    #[test]
    fn a_tab_read_at_the_start_is_forgotten_once_the_user_chooses_one() {
        let ctx = egui::Context::default();
        for old in ["optics", "history"] {
            let folder = settings::testing::Folder::new(&format!("forgotten-tab-{old}"));
            let text = format!("version = 1\n\n[edit]\ntab = \"{old}\"\n");
            std::fs::write(folder.settings(), text).expect("written");
            let (mut keeper, settings) = Keeper::open(folder.storage());
            let mut shell = Shell::new(&settings, Locale::EnGb, OperatingSystem::Nix);
            assert_eq!(shell.panel_tab, BASE, "{old}");
            for tab in [2, BASE] {
                shell.apply(Intent::PanelTab(tab), &ctx, &[]);
                keeper.frame(Instant::now(), &shell.to_settings(&ctx));
            }
            assert_eq!(shell.to_settings(&ctx).edit.tab.as_deref(), Some("base"));
            keeper.flush();
            assert!(!folder.text().contains(old), "{old}: {}", folder.text());
            let (_, settings) = Keeper::open(folder.storage());
            let default = settings::registry::EDIT_TAB.default();
            let kept = settings.edit.tab.as_deref().unwrap_or(default);
            assert_eq!(kept, "base", "{old}");
        }
    }

    /// Each tab lists its modules as darktable's right panel does, in pipeline order with
    /// the last module at the top (decision 38): the order of their operations in
    /// [`apertrust_operations::OPERATIONS`], reversed. A module's id is its operation's but
    /// for the Tone curve's, whose Luma curve is RGB curve's (decision 2).
    #[test]
    fn each_tab_lists_its_modules_last_in_the_pipeline_first() {
        let place = |module: &str| {
            let operation = match module {
                "tone_curve" => "rgbcurve",
                other => other,
            };
            apertrust_operations::OPERATIONS
                .iter()
                .position(|op| op.manifest().id == operation)
                .unwrap_or_else(|| panic!("{module} has no operation {operation}"))
        };
        for (modules, id) in TAB_MODULES.iter().zip(TAB_IDS) {
            let places: Vec<usize> = modules.iter().map(|module| place(module)).collect();
            assert!(
                places.windows(2).all(|pair| pair[0] > pair[1]),
                "{id}: {modules:?} at {places:?}"
            );
        }
    }

    /// Every built-in module is drawn in some tab, once, and every module a tab names is
    /// built in, so a module that joins `MODULES` joins a tab too.
    #[test]
    fn every_built_in_module_is_in_one_tab() {
        let listed: Vec<&str> = TAB_MODULES
            .iter()
            .flat_map(|tab| tab.iter().copied())
            .collect();
        for module in apertrust_modules::MODULES {
            let id = module.definition().id;
            let tabs = listed.iter().filter(|listed| **listed == id).count();
            assert_eq!(tabs, 1, "{id} is in {tabs} tabs");
        }
        for id in listed {
            assert!(
                apertrust_modules::MODULES
                    .iter()
                    .any(|module| module.definition().id == id),
                "{id} is in a tab but not built in"
            );
        }
    }

    #[test]
    fn the_side_panel_may_take_at_most_half_the_window() {
        assert_eq!(panel_widths(1600.0), 240.0..=600.0);
        assert_eq!(panel_widths(960.0), 240.0..=480.0);
        assert_eq!(panel_widths(1001.0), 240.0..=500.0);
        assert_eq!(panel_widths(500.0), 240.0..=250.0);
        // Narrower than twice the narrowest, the panel stays at the narrowest until the
        // shell's design below 960 points says otherwise.
        assert_eq!(panel_widths(400.0), 240.0..=240.0);
    }

    /// The shell beside the photo, with the actions the keys ran.
    struct Panels {
        shell: Shell,
        pressed: Vec<ActionId>,
    }

    /// The side panel and the photo as the app's frame lays them out, the keys read first,
    /// in a harness `width` points wide, whose ui keeps 8 points either side; the shell as
    /// `settings` leave it, with two photos, so ← and → would go from one to the other.
    fn panels(width: f32, settings: &Settings) -> Harness<'static, Panels> {
        Harness::builder()
            .with_size(egui::vec2(width, 600.0))
            // Frames a sixtieth of a second apart, so two clicks make a double-click.
            .with_step_dt(1.0 / 60.0)
            .with_context_setup(|ctx| {
                apertrust_ui::install(ctx, Theme::default());
                registry_keys_only(ctx);
            })
            .build_ui_state(
                |ui, panels: &mut Panels| {
                    let ctx = ui.ctx().clone();
                    let shell = &mut panels.shell;
                    let state = State {
                        photos: 2,
                        ..shell.state(&ctx, true, false)
                    };
                    let keys = actions::read(&ctx, &shell.keymap, &state);
                    panels.pressed.extend(keys.pressed);
                    let name = tr(&ctx, "panel-width");
                    let edge = shell.panel_edge(&ctx, ui.available_width(), &name);
                    let panel = egui::Panel::right("side panel")
                        .exact_size(edge.width(&ctx))
                        .resizable(false)
                        .show(ui, |ui| shell.tab_bar(ui));
                    egui::CentralPanel::default().show(ui, |_| {});
                    if let Some(action) = edge.show(ui, panel.response.rect) {
                        shell.resize_panel(action);
                    }
                },
                Panels {
                    shell: Shell::new(settings, Locale::EnGb, OperatingSystem::Mac),
                    pressed: Vec::new(),
                },
            )
    }

    /// The side panel's edge, as screen readers find it: its value is the panel's width.
    fn panel_edge<'h>(harness: &'h Harness<'_, Panels>) -> egui_kittest::Node<'h> {
        harness.get_by_role_and_label(Role::Splitter, "Side panel width")
    }

    fn shown_width(harness: &Harness<'_, Panels>) -> Option<f64> {
        panel_edge(harness).accesskit_node().numeric_value()
    }

    /// Drags the side panel's edge `by` points along the window.
    fn drag_panel_edge(harness: &mut Harness<'_, Panels>, by: f32) {
        let from = panel_edge(harness).rect().center();
        let to = from + egui::vec2(by, 0.0);
        harness.hover_at(from);
        harness.run();
        harness.drag_at(from);
        harness.run();
        harness.hover_at(to);
        harness.run();
        harness.drop_at(to);
        harness.run();
    }

    #[test]
    fn the_side_panel_keeps_the_width_its_edge_was_dragged_to() {
        let mut harness = panels(1600.0, &Settings::default());
        harness.run();
        assert_eq!(shown_width(&harness), Some(320.0));
        drag_panel_edge(&mut harness, -80.0);
        assert_eq!(harness.state().shell.panel_width, Some(400.0));
        assert_eq!(shown_width(&harness), Some(400.0));
        let ctx = egui::Context::default();
        let kept = |harness: &Harness<'_, Panels>| harness.state().shell.to_settings(&ctx);
        assert_eq!(kept(&harness).panel_width, Some(400.0));
        // Never wider than its widths.
        drag_panel_edge(&mut harness, -400.0);
        assert_eq!(harness.state().shell.panel_width, Some(600.0));

        // A double-click goes back to the theme's width, which settings then leave out.
        for _ in 0..30 {
            harness.step();
        }
        for _ in 0..2 {
            panel_edge(&harness).click();
            harness.step();
        }
        harness.run();
        assert_eq!(harness.state().shell.panel_width, None);
        assert_eq!(shown_width(&harness), Some(320.0));
        assert_eq!(kept(&harness).panel_width, None);
    }

    #[test]
    fn the_side_panel_never_takes_more_than_half_the_window() {
        // A window 960 points wide, at the width the user kept in a wider one.
        let settings = Settings {
            panel_width: Some(600.0),
            ..Settings::default()
        };
        let mut harness = panels(976.0, &settings);
        harness.run();
        assert_eq!(shown_width(&harness), Some(480.0));
        let edge = panel_edge(&harness);
        assert_eq!(edge.accesskit_node().max_numeric_value(), Some(480.0));
        // The width kept stays, for a wider window, while a drag changes nothing, as one
        // against the limit does, until one does.
        let ctx = egui::Context::default();
        let kept = harness.state().shell.to_settings(&ctx).panel_width;
        assert_eq!(kept, Some(600.0));
        drag_panel_edge(&mut harness, -100.0);
        assert_eq!(harness.state().shell.panel_width, Some(600.0));
        assert_eq!(shown_width(&harness), Some(480.0));
        drag_panel_edge(&mut harness, 60.0);
        assert_eq!(harness.state().shell.panel_width, Some(420.0));
    }

    #[test]
    fn the_focused_side_panel_edge_keeps_its_keys_from_the_actions() {
        // ← and → are the photos' keys, which do nothing yet, so End opens Settings here.
        let settings = Settings {
            keys: BTreeMap::from([("settings".to_owned(), vec!["End".to_owned()])]),
            ..Settings::default()
        };
        let mut harness = panels(1600.0, &settings);
        harness.run();
        harness.key_press(Key::End);
        harness.run();
        assert_eq!(harness.state().pressed, vec![ActionId::SETTINGS]);
        harness.state_mut().pressed.clear();

        panel_edge(&harness).focus();
        harness.run();
        let width = |harness: &Harness<'_, Panels>| harness.state().shell.panel_width;
        harness.key_press(Key::ArrowLeft);
        harness.run();
        assert_eq!(width(&harness), Some(330.0));
        harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowLeft);
        harness.run();
        assert_eq!(width(&harness), Some(380.0));
        harness.key_press(Key::ArrowRight);
        harness.run();
        assert_eq!(width(&harness), Some(370.0));
        harness.key_press(Key::End);
        harness.run();
        assert_eq!(width(&harness), Some(600.0));
        harness.key_press(Key::Home);
        harness.run();
        assert_eq!(width(&harness), Some(240.0));
        // Backspace is the Mac's Delete, and the previous photo's key.
        harness.key_press(Key::Backspace);
        harness.run();
        assert_eq!(width(&harness), None);
        harness.key_press(Key::ArrowLeft);
        harness.run();
        harness.key_press(Key::Delete);
        harness.run();
        assert_eq!(width(&harness), None);
        assert!(panel_edge(&harness).is_focused());
        assert_eq!(harness.state().pressed, vec![]);
    }

    /// The app's keys read first, as its frame reads them, then a point picker on the photo
    /// (`docs/pickers.md`, On the photo), and what each asked for.
    struct Picking {
        shell: Shell,
        pressed: Vec<ActionId>,
        sent: Vec<PickerCommand>,
    }

    /// With the picker on the photo focused, its keys reach it before the app's actions,
    /// as a focused slider's do: → and Space are Next Photo's, ← and Backspace Previous
    /// Photo's, and here ↑ and Delete open Settings. Without the focus, the actions take
    /// them and the picker hears none.
    #[test]
    fn the_focused_picker_takes_its_keys_before_the_app_s_actions() {
        use apertrust_engine::session::{PickerRef, Region};
        let settings = Settings {
            keys: BTreeMap::from([(
                "settings".to_owned(),
                vec!["ArrowUp".to_owned(), "Delete".to_owned()],
            )]),
            ..Settings::default()
        };
        let mut harness = Harness::builder()
            .with_size(egui::vec2(800.0, 600.0))
            .with_context_setup(|ctx| {
                apertrust_ui::install(ctx, Theme::default());
                registry_keys_only(ctx);
            })
            .build_ui_state(
                |ui, picking: &mut Picking| {
                    let ctx = ui.ctx().clone();
                    let state = State {
                        photos: 2,
                        ..picking.shell.state(&ctx, true, false)
                    };
                    let keys = actions::read(&ctx, &picking.shell.keymap, &state);
                    picking.pressed.extend(keys.pressed);
                    let photo = egui::Rect::from_min_size(
                        egui::pos2(100.0, 100.0),
                        egui::vec2(400.0, 300.0),
                    );
                    let on = pickers::OnPhoto {
                        picker: PickerRef::ColourPicker,
                        photo,
                        pixels: [400, 300],
                        region: Region::Point([0.5, 0.5]),
                        switched: Region::Area([0.02, 0.02, 0.98, 0.98]),
                        colour: egui::Color32::GRAY,
                        name: "",
                    };
                    picking.sent.extend(pickers::on_photo(ui, &on));
                },
                Picking {
                    shell: Shell::new(&settings, Locale::EnGb, OperatingSystem::Mac),
                    pressed: Vec::new(),
                    sent: Vec::new(),
                },
            );
        harness.run();
        let keys = [
            Key::ArrowRight,
            Key::Space,
            Key::ArrowLeft,
            Key::Backspace,
            Key::ArrowUp,
            Key::Delete,
        ];
        for key in keys {
            harness.key_press(key);
            harness.run();
        }
        assert_eq!(
            harness.state().pressed,
            [ActionId::SETTINGS, ActionId::SETTINGS]
        );
        assert_eq!(harness.state().sent, []);

        harness.state_mut().pressed.clear();
        harness
            .get_by(|node| node.role() == Role::Group && node.value().is_some())
            .focus();
        harness.run();
        for key in keys.into_iter().chain([Key::Escape]) {
            harness.key_press(key);
            harness.run();
        }
        assert_eq!(harness.state().pressed, []);
        let point = |x: f32, y: f32| PickerCommand::Point {
            at: [x, y],
            done: true,
            keys: Default::default(),
        };
        assert_eq!(
            harness.state().sent,
            [
                point(0.5 + 1.0 / 400.0, 0.5),
                PickerCommand::Switch,
                point(0.5 - 1.0 / 400.0, 0.5),
                PickerCommand::Reset,
                point(0.5, 0.5 - 1.0 / 300.0),
                PickerCommand::Reset,
                PickerCommand::Off
            ]
        );
    }

    /// The shell as the app's frame runs it, without the engine, which `App` needs a GPU
    /// for: the keys read first, then the top panel, the side panel's tabs and a section,
    /// the settings dialog, then what the keys and clicks asked for, Open…'s dialog, which a
    /// stand-in for the person answers, and last what was dropped onto the window.
    struct Bench {
        shell: Shell,
        photo: bool,
        edited: bool,
        /// Whether a folder or a photo is open, as the app's mirror says: File › Close's.
        anything_open: bool,
        /// The photo's history, as the app's mirror holds it: what Undo and Redo would do.
        history: Option<apertrust_engine::session::History>,
        /// Every action the keys asked for that could run.
        pressed: Vec<ActionId>,
        /// The hold actions held in the last frame.
        held: Vec<ActionId>,
        /// Every action chosen in the ☰'s menu.
        chosen: Vec<ActionId>,
        /// The commands sent to the session.
        commands: Vec<Command>,
        peeking: bool,
        /// The top panel's and the side panel's rectangles, the regions F6 goes through.
        regions: Vec<egui::Rect>,
        /// The room the window's buttons take where the top panel is the title bar.
        title_bar: Option<f32>,
        /// Whether the top panel draws the caption buttons, and the window is maximised.
        caption: Option<bool>,
        /// The window's platform, as the top panel's gestures see it.
        system: Stub,
        /// The open dialog, as Open… sees it.
        dialog: Person,
        /// The dialog host, which shows Settings in its platform's form.
        dialogs: dialog_host::AppDialogs,
        host: FakeHost,
        /// What was dropped, while it is looked at.
        drops: dropped::Drops,
        /// The files the system asked the app to open, Open With's, while they are looked at.
        opened: dropped::Drops,
        /// Each drop or Open… that opened nothing: the path it names, and why.
        refused: Vec<(PathBuf, dropped::Refusal)>,
        /// The alert up, and what each refusal's asking of it came to.
        alerts: alerts::Alerts,
        asked: Vec<dialog_host::Ask>,
    }

    /// A stand-in for the open dialog and the person answering it: `answer` is what they
    /// will choose, raws or a folder, or None to cancel, and None while they have not
    /// answered yet; `asked` holds the folder of every dialog asked for, a second over the
    /// first too.
    #[derive(Default)]
    struct Person {
        asking: bool,
        answer: Option<Option<Vec<PathBuf>>>,
        asked: Vec<Option<PathBuf>>,
        /// What each dialog asked for, a raw or a folder.
        picks: Vec<dialogs::Pick>,
        pick: Option<dialogs::Pick>,
    }

    impl dialogs::OpenDialog for Person {
        fn asking(&self) -> bool {
            self.asking
        }

        fn ask(&mut self, pick: dialogs::Pick, folder: Option<&Path>) {
            self.asking = true;
            self.pick = Some(pick.clone());
            self.picks.push(pick);
            self.asked.push(folder.map(Path::to_owned));
        }

        fn chosen(&mut self) -> Option<dialogs::Chosen> {
            if !self.asking {
                return None;
            }
            let answer = self.answer.take()?;
            self.asking = false;
            let pick = self.pick.take()?;
            answer.map(|paths| dialogs::Chosen { pick, paths })
        }
    }

    /// A stand-in for the window's platform: its double-click setting, and whether it
    /// moves and zooms the window itself (`native`), else egui's `StartDrag` and
    /// `Maximized` do; `drags` and `zooms` count what was asked of it, and `menus` holds
    /// where the window's menu was asked for.
    struct Stub {
        double_click: DoubleClick,
        native: bool,
        drags: usize,
        zooms: usize,
        menus: Vec<egui::Pos2>,
    }

    impl WindowControl for Stub {
        fn double_click(&self) -> DoubleClick {
            self.double_click
        }

        fn zoom(&mut self) -> bool {
            self.zooms += 1;
            self.native
        }

        fn drag(&mut self) -> bool {
            self.drags += 1;
            self.native
        }

        fn menu(&mut self, at: egui::Pos2) {
            self.menus.push(at);
        }
    }

    fn bench(os: OperatingSystem) -> Harness<'static, Bench> {
        bench_with(os, &Settings::default())
    }

    /// The bench with the shell as `settings` leave it.
    fn bench_with(os: OperatingSystem, settings: &Settings) -> Harness<'static, Bench> {
        let host = FakeHost::on(os);
        Harness::builder()
            .with_size(egui::vec2(1200.0, 600.0))
            .with_os(os)
            .with_context_setup(move |ctx| {
                apertrust_ui::install(ctx, Theme::default());
                registry_keys_only(ctx);
                ctx.global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
            })
            .build_ui_state(
                |ui, bench: &mut Bench| {
                    let ctx = ui.ctx().clone();
                    // As `App::logic` words them before each frame.
                    let modules = apertrust_modules::MODULES;
                    let words = translator(&ctx);
                    bench.shell.steps = history::steps(bench.history.as_ref(), modules, &words);
                    let state = bench.state(&ctx);
                    bench.shell.access_key(&ctx, &state);
                    let keys = actions::read(&ctx, &bench.shell.keymap, &state);
                    bench.shell.close_menu(&keys.pressed);
                    bench.pressed.extend(&keys.pressed);
                    bench.held.clone_from(&keys.held);
                    let mut run = keys.pressed;
                    let top = ui.scope(|ui| {
                        let (title_bar, caption) = (bench.title_bar, bench.caption);
                        bench
                            .shell
                            .top_panel(ui, &state, SaveState::None, title_bar, caption, &[])
                    });
                    let mut window = None;
                    match top.inner {
                        Some(FromTopPanel::Action(id)) => run.push(id),
                        Some(FromTopPanel::Window(gesture)) => window = Some(gesture),
                        Some(FromTopPanel::Jobs | FromTopPanel::NotSaved) | None => {}
                    }
                    let name = tr(&ctx, "panel-width");
                    let edge = bench.shell.panel_edge(&ctx, ui.available_width(), &name);
                    let width = edge.width(&ctx);
                    let side = ui.scope(|ui| {
                        ui.set_width(width);
                        let tab = bench.shell.tab_bar(ui);
                        let out = Section::new("Exposure", true)
                            .module()
                            .peek(keys.held.contains(&ActionId::PEEK))
                            .show(ui, |ui| {
                                SliderRow::new("Brightness", 0.0, -100.0..=100.0).show(ui);
                            });
                        bench.peeking = out.peeking;
                        tab
                    });
                    run.extend(side.inner);
                    bench.shell.about.show(&ctx);
                    let chosen = bench.shell.popups(&ctx);
                    bench.chosen.extend(chosen);
                    run.extend(chosen);
                    bench.regions = vec![top.response.rect, side.response.rect];
                    for id in run {
                        let state = bench.state(&ctx);
                        let regions = bench.regions.clone();
                        let command = bench.shell.run(id, &state, &ctx, &regions);
                        bench.commands.extend(command);
                    }
                    // Settings asked for goes to the host, as `run_action` hands it on.
                    if std::mem::take(&mut bench.shell.settings) {
                        let (keeper, _) = Keeper::nowhere();
                        let close = ctx.os() != OperatingSystem::Mac;
                        let settings = settings_dialog::Settings::new(&keeper, close);
                        bench.dialogs.open(settings, None, &ctx);
                    }
                    let state = bench.state(&ctx);
                    bench.shell.open_menu(&ctx, &state);
                    let asked = if std::mem::take(&mut bench.shell.open) {
                        Some(dialogs::Pick::Files)
                    } else if std::mem::take(&mut bench.shell.open_folder) {
                        Some(dialogs::Pick::Folder)
                    } else {
                        bench.shell.locate.take().map(dialogs::Pick::Locate)
                    };
                    let pictures = || Some(PathBuf::from("Pictures"));
                    // The app's own start and answer, `App` needing a session and a GPU.
                    let start = dialogs::locate_start(asked.as_ref());
                    let folder = || start.unwrap_or_else(pictures);
                    let opens = |opens| match opens {
                        Opens::Photo(path) => Command::Open(path),
                        Opens::Folder(path) => {
                            Command::Library(LibraryCommand::Open(Source::Folder(path)))
                        }
                    };
                    // What Open… or a drop opens, or the alert asked, as `carry_out_opening`.
                    let carry_out = |bench: &mut Bench, opening: super::Opening| match opening {
                        Ok(chosen) => bench.commands.push(opens(chosen)),
                        Err((path, why)) => {
                            let request = alerts::refused_request(&ctx, &file_name(&path), why);
                            let purpose = alerts::Purpose::Refused;
                            bench.asked.push(bench.alerts.ask(
                                &mut bench.dialogs,
                                purpose,
                                &request,
                                &ctx,
                            ));
                            bench.refused.push((path, why));
                        }
                    };
                    let chosen = dialogs::open(&mut bench.dialog, asked, folder);
                    if let Some(opening) = chosen.and_then(|c| answered(&mut bench.shell, c)) {
                        carry_out(bench, opening);
                    }
                    let modal = bench.state(&ctx).modal;
                    if let Some(dropped) = bench.drops.looked().filter(|_| !modal) {
                        carry_out(bench, dropped_opens(dropped));
                    }
                    bench.drops.take(&ctx, modal);
                    if !modal && let Some(opened) = bench.opened.looked() {
                        carry_out(bench, dropped_opens(opened));
                    }
                    if let Some(gesture) = window {
                        run_title_bar(gesture, &ctx, &mut bench.system);
                    }
                },
                Bench {
                    shell: Shell::new(settings, Locale::EnGb, os),
                    photo: true,
                    edited: false,
                    anything_open: false,
                    history: None,
                    pressed: Vec::new(),
                    held: Vec::new(),
                    chosen: Vec::new(),
                    commands: Vec::new(),
                    peeking: false,
                    regions: Vec::new(),
                    title_bar: None,
                    caption: None,
                    system: Stub {
                        double_click: DoubleClick::Zoom,
                        native: false,
                        drags: 0,
                        zooms: 0,
                        menus: Vec::new(),
                    },
                    dialog: Person::default(),
                    dialogs: dialog_host::AppDialogs::new(host.clone()),
                    host,
                    drops: dropped::Drops::default(),
                    opened: dropped::Drops::default(),
                    refused: Vec::new(),
                    alerts: alerts::Alerts::default(),
                    asked: Vec::new(),
                },
            )
    }

    impl Bench {
        /// The actions' availability, the shell's with the host's dialog, as the app has it.
        fn state(&self, ctx: &egui::Context) -> State {
            let history = self.history.as_ref();
            let shell = State {
                opened: self.anything_open,
                undo: history.is_some_and(|h| h.undo.is_some()),
                redo: history.is_some_and(|h| h.redo.is_some()),
                ..self.shell.state(ctx, self.photo, self.edited)
            };
            // The open dialog asking and an alert up, as `App::asking_modal`.
            let asking = self.dialog.asking || self.alerts.pending().is_some();
            with_dialogs(shell, asking, self.dialogs.shown())
        }

        /// Whether Settings is open in the host.
        fn settings_open(&self) -> bool {
            let hosted = self.host.0.borrow();
            hosted.shown.is_some() && !hosted.settings.is_empty()
        }
    }

    /// Runs one frame with `events` as its input, all of them, as egui-winit hands over
    /// what arrived since the last frame.
    fn frame(harness: &mut Harness<'_, Bench>, events: impl IntoIterator<Item = egui::Event>) {
        harness.input_mut().events.extend(events);
        harness.step();
    }

    fn pointer(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        }
    }

    /// Whether the last frame asked the window for `command`.
    fn asked(harness: &Harness<'_, Bench>, command: &egui::ViewportCommand) -> bool {
        harness
            .output()
            .viewport_output
            .values()
            .any(|v| v.commands.contains(command))
    }

    /// An empty point of the top panel: between its last action and its end.
    fn empty_point(harness: &Harness<'_, Bench>) -> egui::Pos2 {
        let apply = harness.get_by_label("Apply edit").rect();
        let bar = harness
            .get_by_role_and_label(Role::Toolbar, "Top panel")
            .rect();
        egui::pos2((apply.right() + bar.right()) / 2.0, apply.center().y)
    }

    #[test]
    fn where_the_top_panel_is_the_title_bar_its_empty_areas_move_and_zoom_the_window() {
        let mut harness = bench(OperatingSystem::Mac);
        harness.state_mut().title_bar = Some(90.0);
        harness.run();
        // The window's buttons keep the room they take.
        let import = harness
            .get_by_role_and_label(Role::Button, "Import…")
            .rect();
        assert!(import.left() >= 90.0, "{import:?}");
        let empty = empty_point(&harness);

        // A double-click does what the system's setting says, zooming after the frame.
        let double_click = [
            egui::Event::PointerMoved(empty),
            pointer(empty, true),
            pointer(empty, false),
            pointer(empty, true),
            pointer(empty, false),
        ];
        let zoom = egui::ViewportCommand::Maximized(true);
        let unzoom = egui::ViewportCommand::Maximized(false);
        let minimise = egui::ViewportCommand::Minimized(true);
        frame(&mut harness, double_click.clone());
        assert!(asked(&harness, &zoom));
        // Where the platform zooms the window itself, egui is not asked to.
        harness.state_mut().system.native = true;
        harness.run_steps(30);
        frame(&mut harness, double_click.clone());
        assert_eq!(harness.state().system.zooms, 2);
        assert!(!asked(&harness, &zoom) && !asked(&harness, &unzoom));
        harness.state_mut().system.native = false;
        harness.state_mut().system.double_click = DoubleClick::Minimise;
        harness.run_steps(30);
        frame(&mut harness, double_click.clone());
        assert!(asked(&harness, &minimise));
        harness.state_mut().system.double_click = DoubleClick::Nothing;
        harness.run_steps(30);
        frame(&mut harness, double_click);
        let window = |harness: &Harness<'_, Bench>| {
            [&zoom, &unzoom, &minimise]
                .iter()
                .any(|command| asked(harness, command))
        };
        assert!(!window(&harness));

        // A drag moves it once it has moved, never on the press.
        harness.run_steps(30);
        let start = egui::ViewportCommand::StartDrag;
        frame(
            &mut harness,
            [egui::Event::PointerMoved(empty), pointer(empty, true)],
        );
        assert!(!asked(&harness, &start));
        let moved = empty + egui::vec2(0.0, 12.0);
        frame(&mut harness, [egui::Event::PointerMoved(moved)]);
        assert!(asked(&harness, &start));
        frame(&mut harness, [pointer(moved, false)]);
        assert_eq!(harness.state().system.drags, 1);
        // Where the platform moves the window itself, egui's `StartDrag` is not sent too.
        harness.state_mut().system.native = true;
        harness.run_steps(30);
        frame(
            &mut harness,
            [egui::Event::PointerMoved(empty), pointer(empty, true)],
        );
        frame(&mut harness, [egui::Event::PointerMoved(moved)]);
        assert_eq!(harness.state().system.drags, 2);
        assert!(!asked(&harness, &start));
        frame(&mut harness, [pointer(moved, false)]);
        harness.state_mut().system.native = false;

        // Not from Undo, disabled until it is built: still a button, not an empty area.
        harness.state_mut().system.double_click = DoubleClick::Zoom;
        harness.run_steps(30);
        let undo = harness
            .get_by_role_and_label(Role::Button, "Undo")
            .rect()
            .center();
        frame(
            &mut harness,
            [egui::Event::PointerMoved(undo), pointer(undo, true)],
        );
        frame(
            &mut harness,
            [egui::Event::PointerMoved(undo + egui::vec2(0.0, 12.0))],
        );
        assert!(!asked(&harness, &start));
        frame(&mut harness, [pointer(undo, false)]);
        harness.run_steps(30);
        frame(
            &mut harness,
            [
                egui::Event::PointerMoved(undo),
                pointer(undo, true),
                pointer(undo, false),
                pointer(undo, true),
                pointer(undo, false),
            ],
        );
        assert!(!window(&harness));
    }

    #[test]
    fn the_caption_buttons_and_a_right_click_ask_the_window() {
        let mut harness = bench(OperatingSystem::Nix);
        harness.state_mut().title_bar = Some(0.0);
        harness.state_mut().caption = Some(false);
        harness.run();
        let minimise = egui::ViewportCommand::Minimized(true);
        harness
            .get_by_role_and_label(Role::Button, "Minimise")
            .click();
        harness.step();
        assert!(asked(&harness, &minimise));
        harness.run_steps(30);
        harness
            .get_by_role_and_label(Role::Button, "Maximise")
            .click();
        harness.step();
        assert!(asked(&harness, &egui::ViewportCommand::Maximized(true)));
        // Closing quits, as the Quit action does.
        harness.run_steps(30);
        harness.get_by_role_and_label(Role::Button, "Close").click();
        harness.step();
        assert!(asked(&harness, &egui::ViewportCommand::Close));
        // A maximised window's middle button restores it.
        harness.state_mut().caption = Some(true);
        harness.run_steps(30);
        assert!(harness.query_by_label("Maximise").is_none());
        harness.get_by_role_and_label(Role::Button, "Restore");

        // A right-click on an empty area opens the window's menu where it was let go, in
        // physical pixels.
        let empty = empty_point(&harness);
        let right = |pressed| egui::Event::PointerButton {
            pos: empty,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        harness.run_steps(30);
        frame(
            &mut harness,
            [egui::Event::PointerMoved(empty), right(true), right(false)],
        );
        let ppp = harness.ctx.pixels_per_point();
        assert_eq!(harness.state().system.menus, vec![empty * ppp]);
    }

    #[test]
    fn the_bands_inside_the_window_s_edges_resize_it() {
        use egui::ResizeDirection as Way;
        let window = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        let at = |x, y| resize_direction(window, egui::pos2(x, y), RESIZE_BAND);
        assert_eq!(at(0.0, 0.0), Some(Way::NorthWest));
        assert_eq!(at(799.0, 5.0), Some(Way::NorthEast));
        assert_eq!(at(3.0, 599.0), Some(Way::SouthWest));
        assert_eq!(at(796.0, 596.0), Some(Way::SouthEast));
        assert_eq!(at(400.0, 5.9), Some(Way::North));
        assert_eq!(at(400.0, 595.0), Some(Way::South));
        assert_eq!(at(0.0, 300.0), Some(Way::West));
        assert_eq!(at(794.5, 300.0), Some(Way::East));
        // Nothing beyond the band's width, nor outside the window.
        assert_eq!(at(400.0, 6.0), None);
        assert_eq!(at(6.0, 300.0), None);
        assert_eq!(at(793.0, 300.0), None);
        assert_eq!(at(-1.0, 300.0), None);

        // A press in a band is taken from egui's input; any other event is left.
        let ctx = egui::Context::default();
        let press = |x, y| egui::Event::PointerButton {
            pos: egui::pos2(x, y),
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        };
        let input = |maximized| {
            let mut input = egui::RawInput {
                screen_rect: Some(window),
                events: vec![
                    egui::Event::PointerMoved(egui::pos2(2.0, 2.0)),
                    press(2.0, 2.0),
                ],
                ..Default::default()
            };
            input
                .viewports
                .entry(input.viewport_id)
                .or_default()
                .maximized = Some(maximized);
            input
        };
        let mut normal = input(false);
        assert_eq!(
            resize_press(&ctx, &mut normal, RESIZE_BAND),
            Some(Way::NorthWest)
        );
        assert_eq!(normal.events.len(), 1);
        // Away from the bands, the press stays egui's.
        let mut inside = input(false);
        inside.events = vec![press(100.0, 100.0)];
        assert_eq!(resize_press(&ctx, &mut inside, RESIZE_BAND), None);
        assert_eq!(inside.events.len(), 1);
        // A maximised window is not resized from its edges.
        let mut maximised = input(true);
        assert_eq!(resize_press(&ctx, &mut maximised, RESIZE_BAND), None);
        assert_eq!(maximised.events.len(), 2);
    }

    #[test]
    fn the_photo_area_s_size_waits_until_the_window_has_kept_its_size_a_while() {
        let start = Instant::now();
        let at = |ms: u64| start + std::time::Duration::from_millis(ms);
        let mut bounds = Bounds::default();
        // The first size goes at once, whatever the window does.
        assert_eq!(bounds.update((800, 600), true, at(0)), Some((800, 600)));
        // While the window resizes, sizes wait, and a still frame between two steps does
        // not let them through.
        assert_eq!(bounds.update((820, 610), true, at(16)), None);
        assert_eq!(bounds.update((820, 610), false, at(40)), None);
        assert_eq!(bounds.update((840, 620), true, at(60)), None);
        assert_eq!(
            bounds.wait(at(100)),
            Some(RESIZE_SETTLED - std::time::Duration::from_millis(40))
        );
        // Once the window has kept its size long enough, the last goes.
        let settled = at(60) + RESIZE_SETTLED;
        assert_eq!(bounds.update((840, 620), false, settled), Some((840, 620)));
        assert_eq!(bounds.wait(settled), None);
        // A change with the window still, as the proof margin's, goes at once.
        let later = settled + RESIZE_SETTLED;
        assert_eq!(bounds.update((780, 560), false, later), Some((780, 560)));
        assert_eq!(bounds.update((780, 560), false, later), None);
    }

    #[test]
    fn the_buttons_room_follows_where_they_are_and_keeps_the_title_bar_when_unknown() {
        // In the window system's points, at an interface scale of 1.5.
        let placed = Some(WindowButtons::Placed(90.0));
        assert_eq!(button_room(placed, 1.5, false, true), Some(60.0));
        let unplaced = Some(WindowButtons::Unplaced(75.0));
        assert_eq!(button_room(unplaced, 1.5, false, true), Some(50.0));
        assert_eq!(
            button_room(Some(WindowButtons::None), 1.0, false, true),
            None
        );
        // The window out of reach: AppKit's own room, unless in full screen or where the
        // system draws the title bar.
        assert_eq!(button_room(None, 1.0, false, true), Some(BUTTONS_ROOM));
        assert_eq!(button_room(None, 1.0, true, true), None);
        assert_eq!(button_room(None, 1.0, false, false), None);
    }

    #[test]
    fn under_a_modal_dialog_the_main_window_takes_no_input_and_asks_for_no_frames() {
        let click = |pressed| egui::Event::PointerButton {
            pos: egui::pos2(10.0, 10.0),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        let mut input = egui::RawInput {
            events: vec![
                egui::Event::WindowFocused(false),
                click(true),
                click(false),
                egui::Event::Text("a".to_owned()),
            ],
            ..Default::default()
        };
        under_modal(&mut input);
        assert_eq!(
            input.events,
            vec![egui::Event::WindowFocused(false), egui::Event::PointerGone]
        );
        // A frame with nothing taken gets nothing, so it asks for no other.
        let mut quiet = egui::RawInput::default();
        under_modal(&mut quiet);
        assert!(quiet.events.is_empty());
    }

    #[test]
    fn under_an_alert_a_press_on_the_main_window_raises_it_where_the_host_draws_it() {
        use OperatingSystem::{Mac, Nix, Windows};
        for os in [Mac, Windows, Nix] {
            assert!(raises_under_modal(true, false, os), "{os:?}");
            assert!(!raises_under_modal(false, false, os), "{os:?}");
        }
        // The Mac's `NSAlert` keeps the front itself; Linux's drawn alert is raised.
        assert!(!raises_under_modal(true, true, Mac));
        assert!(raises_under_modal(true, true, Nix));
    }

    #[test]
    fn a_press_or_the_focus_on_the_main_window_reaches_for_it_under_a_modal_dialog() {
        let input = |events| egui::RawInput {
            events,
            ..Default::default()
        };
        let at = egui::pos2(10.0, 10.0);
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        assert!(reaches_under_modal(&input(vec![button(true)])));
        assert!(reaches_under_modal(&input(vec![
            egui::Event::WindowFocused(true)
        ])));
        assert!(!reaches_under_modal(&input(vec![
            egui::Event::PointerMoved(at),
            button(false),
            egui::Event::WindowFocused(false),
        ])));
    }

    /// A dialog host the tests answer as a person would: what it shows, in the form its
    /// platform gives it where the test names one (`os`), else in the form it is given (a
    /// modeless window, as Settings is on the Mac, unless a test asks for a sheet), the
    /// Settings it opened with and where, how often an open one came to the front, what
    /// its window changed, what went, and whether it was asked to close the dialog open.
    #[derive(Default)]
    struct Hosted {
        os: Option<OperatingSystem>,
        form: Option<dialog_host::Form>,
        shown: Option<dialog_host::Shown>,
        settings: Vec<(settings_dialog::Settings, Option<settings::Position>)>,
        fronted: usize,
        outputs: Vec<dialog_host::Output>,
        closed: Option<dialog_host::Closed>,
        closing: bool,
        /// The Export settings each opening of the Export dialog held.
        exports: Vec<export::Values>,
        /// The numbers of the previews handed to the dialog.
        previews: Vec<u64>,
        /// The jobs window, if open, and each time it was asked for open: its place and
        /// whether it took the focus.
        jobs: Option<dialog_host::Shown>,
        jobs_opened: Vec<(Option<settings::Position>, bool)>,
        jobs_fronted: usize,
        /// The window open, as the host holds it: a later open goes to it (`again`).
        jobs_window: Option<jobs::Window>,
        /// The jobs the window was last given, and whether it was asked to close.
        jobs_shown: Vec<apertrust_ui::components::Job>,
        jobs_closing: bool,
        /// Each alert asked, and whether the platform shows alerts (`false` by default
        /// is the platform's: a test turns it on).
        alerts: Vec<apertrust_ui::alert::Request>,
        native_alerts: bool,
        /// What asking an alert comes to instead, when a test says so.
        alert_result: Option<dialog_host::Ask>,
        /// The answer a person gave to the alert up, until the app takes it.
        answer: Option<dialog_host::Answer>,
        /// Whether the host draws its alerts, as on Linux: each alert asked is then a modal
        /// window of its own, `alert`, shown over the dialog until it is answered or closed,
        /// and brought forward `alert_fronted` times.
        drawn_alerts: bool,
        alert: Option<dialog_host::Shown>,
        alert_fronted: usize,
    }

    #[derive(Clone, Default)]
    struct FakeHost(std::rc::Rc<std::cell::RefCell<Hosted>>);

    impl dialog_host::Dialogs for FakeHost {
        fn open(
            &mut self,
            dialog: Box<dyn dialog_host::Dialog>,
            place: Option<settings::Position>,
            main: &egui::Context,
        ) {
            // As the main window draws again when a window of the host's takes its focus.
            main.request_repaint();
            let mut hosted = self.0.borrow_mut();
            match &mut hosted.shown {
                // As the host: a modeless window comes to the front, with the focus.
                Some(shown) if shown.form == dialog_host::Form::Modeless => {
                    shown.focused = true;
                    hosted.fronted += 1;
                }
                Some(_) => {}
                None => {
                    // As the platform gives it, where the test names the platform.
                    let given = hosted
                        .os
                        .map(|os| dialog_host::platform::form(dialog.form(os), os));
                    let dialog = dialog as Box<dyn std::any::Any>;
                    if let Some(settings) = dialog.downcast_ref::<settings_dialog::Settings>() {
                        hosted.settings.push((settings.clone(), place));
                    } else if let Some(export) = dialog.downcast_ref::<export::Export>() {
                        hosted.exports.push(export.values().clone());
                    }
                    let form = given.or(hosted.form).unwrap_or(dialog_host::Form::Modeless);
                    let focused = true;
                    hosted.shown = Some(dialog_host::Shown { form, focused });
                }
            }
        }

        fn shown(&self) -> Option<dialog_host::Shown> {
            let hosted = self.0.borrow();
            hosted.alert.or(hosted.shown)
        }

        fn alert_shown(&self) -> bool {
            self.0.borrow().alert.is_some()
        }

        fn raise(&mut self) {
            let mut hosted = self.0.borrow_mut();
            if let Some(alert) = &mut hosted.alert {
                alert.focused = true;
                hosted.alert_fronted += 1;
            } else if let Some(shown) = &mut hosted.shown
                && shown.form == dialog_host::Form::Modal
            {
                shown.focused = true;
            }
        }

        fn closed(&mut self) -> Option<dialog_host::Closed> {
            self.0.borrow_mut().closed.take()
        }

        fn outputs(&mut self) -> Vec<dialog_host::Output> {
            std::mem::take(&mut self.0.borrow_mut().outputs)
        }

        fn close(&mut self) {
            let mut hosted = self.0.borrow_mut();
            // As the host: an alert it draws closes first, dismissed, the dialog staying.
            if hosted.alert.take().is_some() {
                hosted.answer = Some(dialog_host::Answer::Dismissed);
            } else {
                hosted.closing = true;
            }
        }

        fn preview(&mut self, through: u64, _preview: apertrust_engine::session::ExportPreview) {
            self.0.borrow_mut().previews.push(through);
        }

        fn storage(&mut self, _view: storage::View) {}

        fn hardware(&mut self, _: hardware::Running, _: Vec<settings_dialog::Change>) {}

        fn open_jobs(
            &mut self,
            window: jobs::Window,
            place: Option<settings::Position>,
            _main: &egui::Context,
            focus: bool,
        ) {
            let mut hosted = self.0.borrow_mut();
            match &mut hosted.jobs {
                Some(shown) => {
                    if focus {
                        shown.focused = true;
                        hosted.jobs_fronted += 1;
                    }
                    if let Some(open) = &mut hosted.jobs_window {
                        open.again(window, focus);
                    }
                }
                None => {
                    hosted.jobs_opened.push((place, focus));
                    hosted.jobs_shown = window.jobs().to_vec();
                    hosted.jobs_window = Some(window);
                    let form = dialog_host::Form::Modeless;
                    hosted.jobs = Some(dialog_host::Shown {
                        form,
                        focused: focus,
                    });
                }
            }
        }

        fn jobs_shown(&self) -> Option<dialog_host::Shown> {
            self.0.borrow().jobs
        }

        fn set_jobs(&mut self, jobs: Vec<apertrust_ui::components::Job>) {
            let mut hosted = self.0.borrow_mut();
            if hosted.jobs.is_some() {
                if let Some(open) = &mut hosted.jobs_window {
                    open.set(jobs.clone());
                }
                hosted.jobs_shown = jobs;
            }
        }

        fn close_jobs(&mut self) {
            self.0.borrow_mut().jobs_closing = true;
        }

        /// The log window is drawn in its own tests (`log_window::tests`).
        fn open_log(
            &mut self,
            _: log_window::Window,
            _: Option<settings::Position>,
            _: &egui::Context,
        ) {
        }

        fn log_shown(&self) -> Option<dialog_host::Shown> {
            None
        }

        fn close_log(&mut self) {}

        fn ask(
            &mut self,
            request: &apertrust_ui::alert::Request,
            _main: &egui::Context,
        ) -> dialog_host::Ask {
            let mut hosted = self.0.borrow_mut();
            match hosted.alert_result {
                Some(result) => result,
                None if hosted.native_alerts => {
                    hosted.alerts.push(request.clone());
                    if hosted.drawn_alerts {
                        let (form, focused) = (dialog_host::Form::Modal, true);
                        hosted.alert = Some(dialog_host::Shown { form, focused });
                    }
                    dialog_host::Ask::Shown
                }
                None => dialog_host::Ask::NoAlerts,
            }
        }

        fn answer(&mut self) -> Option<dialog_host::Answer> {
            self.0.borrow_mut().answer.take()
        }
    }

    impl FakeHost {
        /// A host that shows each dialog in the form `os` gives it.
        fn on(os: OperatingSystem) -> FakeHost {
            let host = FakeHost::default();
            host.0.borrow_mut().os = Some(os);
            host
        }

        /// A host that shows its dialogs as sheets.
        fn sheets() -> FakeHost {
            let host = FakeHost::default();
            host.0.borrow_mut().form = Some(dialog_host::Form::Sheet);
            host
        }

        /// The dialog goes, as the host closes it after a callback, a sheet once AppKit has
        /// animated it away, the focus going back to `opener`.
        fn gone(&self, opener: Option<egui::Id>) {
            let mut hosted = self.0.borrow_mut();
            hosted.shown = None;
            hosted.closing = false;
            hosted.closed = Some(dialog_host::Closed { opener });
        }

        /// The dialog's window takes the focus, or gives it to the main window.
        fn focus(&self, focused: bool) {
            if let Some(shown) = &mut self.0.borrow_mut().shown {
                shown.focused = focused;
            }
        }
    }

    /// The main window's frame with its dialogs as the app runs them, Settings in a window
    /// of its own as on the Mac: what the dialogs bring before it, as `App::logic` takes it,
    /// with Quit then; a button to hold the focus; and the actions the test presses, each run as `App::run_actions` runs it, and the alerts
    /// as `App::logic` answers them, over the jobs the test reports.
    struct Dialogged {
        shell: Shell,
        /// The settings' keeper, which keeps what Settings changes.
        keeper: Keeper,
        dialogs: dialog_host::AppDialogs,
        button: Option<egui::Id>,
        pressed: Vec<ActionId>,
        /// The jobs the user started, as the mirror folds them.
        jobs: jobs::Mirror,
        alerts: alerts::Alerts,
        quit: alerts::Guard,
        /// Quit is to run again, and the jobs the app stopped.
        quit_again: bool,
        stopped: Vec<apertrust_engine::jobs::JobId>,
        /// Whether Export would have been sent: the overwrite question answered yes.
        exported: usize,
        /// Export Again, and the commands the app would have sent the session for it.
        again: export_again::Again,
        sent: Vec<Command>,
        /// Quit waits for the overwrite question's answer, as `App::quit_held` does.
        quit_held: bool,
        /// The open photo's edit as Quit finds it.
        edit: TestEdit,
        /// Restart now, as `App::restart` notes it; its slot is the relaunch `main` would
        /// start, never started here.
        restart: restart::Restart,
        relaunch: restart::Slot,
    }

    /// The open photo's edit, as `quit_edit` reads the mirror.
    #[derive(Clone, Debug, PartialEq, Eq)]
    enum TestEdit {
        Saved,
        Writing,
        Failed(String),
    }

    impl TestEdit {
        fn edit(&self) -> alerts::Edit<'_> {
            match self {
                TestEdit::Saved => alerts::Edit::Saved,
                TestEdit::Writing => alerts::Edit::Writing,
                TestEdit::Failed(why) => alerts::Edit::Failed(why),
            }
        }
    }

    /// As `App::overwrite_confirmed`: the export is sent, Export Again's if it was waiting,
    /// else the dialog's, which closes with it.
    fn overwrite_confirmed(d: &mut Dialogged) {
        d.exported += 1;
        // The export is sent, and has no job yet.
        d.jobs.starting(d.exported as u64);
        match d.again.confirmed() {
            Some(command) => d.sent.push(command),
            None => d.dialogs.close(),
        }
    }

    /// As `App::ask_overwrite`: the question is asked unless the setting says not to, when
    /// the export goes ahead as a yes would send it. Returns whether it was settled.
    fn ask_overwrite(d: &mut Dialogged, ctx: &egui::Context) -> bool {
        if !d.shell.overwrite.asks() {
            overwrite_confirmed(d);
            return true;
        }
        let request = alerts::overwrite_request(ctx, d.shell.overwrite.held());
        let purpose = alerts::Purpose::Overwrite;
        let asked = d.alerts.ask(&mut d.dialogs, purpose, &request, ctx);
        assert_eq!(asked, dialog_host::Ask::Shown);
        true
    }

    /// As `App::export_again`: the export of the settings as last kept for the photo open,
    /// sent at once unless On conflict is Overwrite, which asks first.
    fn export_again(d: &mut Dialogged, ctx: &egui::Context) {
        if d.quit.quitting() || d.quit_held {
            return;
        }
        let photo = Path::new(export_again::tests::PHOTO);
        let command = export_again::export(photo, &d.shell.export, &export::places());
        if !export::asks_before(d.shell.export.on_conflict) {
            d.sent.push(command);
            return;
        }
        d.again.hold(command);
        if !ask_overwrite(d, ctx) {
            d.again.declined();
        }
    }

    fn dialogged(host: FakeHost) -> Harness<'static, Dialogged> {
        let shell = Shell::new(&Settings::default(), Locale::EnGb, OperatingSystem::Mac);
        let relaunch = restart::Slot::default();
        Harness::builder()
            .with_os(OperatingSystem::Mac)
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
            .build_ui_state(
                |ui, d: &mut Dialogged| {
                    let ctx = ui.ctx().clone();
                    let run = |d: &mut Dialogged, id| {
                        let id = quit_to_restart(id, &mut d.restart).ok()?;
                        let overwrite = d.alerts.pending() == Some(alerts::Purpose::Overwrite);
                        if !hold_for_panel(id, overwrite, &mut d.quit_held) {
                            return None;
                        }
                        let state = d.shell.state(&ctx, true, true);
                        let asking = d.alerts.pending().is_some();
                        let state = with_dialogs(state, asking, d.dialogs.shown());
                        let running = d.jobs.running();
                        let edit = d.edit.edit();
                        let mut leave = alerts::Leave::new(&mut d.quit, running, edit);
                        // The rows as the app's `jobs_view` holds them after its drain.
                        let view = d.jobs.view(&translator(&ctx));
                        let (shell, dialogs, keeper) = (&mut d.shell, &mut d.dialogs, &d.keeper);
                        let command = run_action(
                            shell,
                            dialogs,
                            keeper,
                            &view,
                            (&storage::View::default(), &hardware::Running::default()),
                            id,
                            &state,
                            &ctx,
                            &[],
                            &mut || leave.may(),
                        );
                        if leave.ask {
                            let (alerts, quit) = (&mut d.alerts, &mut d.quit);
                            let dialogs = &mut d.dialogs;
                            d.quit_again |=
                                alerts::ask_quit(alerts, dialogs, quit, running, edit, &ctx);
                        }
                        if std::mem::take(&mut d.shell.export_again) {
                            export_again(d, &ctx);
                        }
                        command
                    };
                    // As `App::logic` takes what happened before a frame.
                    if let Some((purpose, answer)) = d.alerts.answered(&mut d.dialogs) {
                        d.shell.overwrite.answered(purpose, answer);
                        match alerts::then(purpose, answer, &mut d.quit, &d.jobs) {
                            alerts::Then::Nothing => {
                                d.again.declined();
                                if purpose == alerts::Purpose::Quit {
                                    d.restart.cancel();
                                }
                            }
                            alerts::Then::Export => overwrite_confirmed(d),
                            alerts::Then::Stop(jobs) => {
                                d.stopped.extend(jobs);
                                d.quit_again = true;
                            }
                            alerts::Then::RetrySave => d.sent.push(Command::RetrySave),
                            alerts::Then::Leave => d.quit_again = true,
                            alerts::Then::Told => {}
                        }
                    }
                    if d.quit_held && d.alerts.pending() != Some(alerts::Purpose::Overwrite) {
                        d.quit_held = false;
                        run(d, ActionId::QUIT);
                    }
                    let holds = d.quit.holds_close(d.jobs.running(), d.edit.edit());
                    let jobs_hold = hold_close_for_jobs(&ctx, holds);
                    let asking = d.alerts.pending().is_some();
                    let state =
                        with_dialogs(d.shell.state(&ctx, true, true), asking, d.dialogs.shown());
                    let taken = take_dialogs(&mut d.shell, &mut d.dialogs, &state, &ctx);
                    apply_settings(&taken.settings, &mut d.shell, &mut d.keeper, &ctx);
                    for id in taken.actions {
                        run(d, id);
                    }
                    let finished = d.quit.confirmed() && d.jobs.running().none();
                    let written = d.quit.waits_for_write() && d.edit != TestEdit::Writing;
                    let again = std::mem::take(&mut d.quit_again);
                    if taken.quit || jobs_hold || again || finished || written {
                        run(d, ActionId::QUIT);
                    }
                    d.button = Some(ui.button("Opener").id);
                    for id in std::mem::take(&mut d.pressed) {
                        run(d, id);
                    }
                },
                Dialogged {
                    shell,
                    keeper: Keeper::nowhere().0,
                    dialogs: dialog_host::AppDialogs::new(host),
                    button: None,
                    pressed: Vec::new(),
                    jobs: jobs::Mirror::new(None),
                    alerts: alerts::Alerts::default(),
                    quit: alerts::Guard::default(),
                    quit_again: false,
                    stopped: Vec::new(),
                    exported: 0,
                    again: export_again::Again::default(),
                    sent: Vec::new(),
                    quit_held: false,
                    edit: TestEdit::Saved,
                    restart: restart::Restart::new(
                        vec!["-d".into(), "gpu".into()],
                        true,
                        relaunch.clone(),
                    )
                    .finding(restart::tests::launcher),
                    relaunch,
                },
            )
    }

    fn press_action(harness: &mut Harness<'_, Dialogged>, id: ActionId) {
        harness.state_mut().pressed.push(id);
        harness.step();
    }

    /// Whether the last frame asked the main window to close, or cancelled its close.
    fn commanded(harness: &Harness<'_, Dialogged>, command: &egui::ViewportCommand) -> bool {
        harness
            .output()
            .viewport_output
            .values()
            .any(|v| v.commands.contains(command))
    }

    fn closing(harness: &Harness<'_, Dialogged>) -> bool {
        commanded(harness, &egui::ViewportCommand::Close)
    }

    /// The jobs the events say, and the jobs as the window then draws them.
    fn jobs_after(
        mirror: &mut jobs::Mirror,
        events: &[Event],
    ) -> (jobs::Show, Vec<apertrust_ui::components::Job>) {
        let show = events
            .iter()
            .map(|event| mirror.apply(event))
            .max()
            .unwrap_or(jobs::Show::Nothing);
        (show, mirror.view(&Translator::new(Locale::EnGb)))
    }

    #[test]
    fn a_job_the_user_started_opens_the_jobs_window_beside_a_dialog_and_a_system_job_never_does() {
        use apertrust_engine::jobs::Queue;
        let host = FakeHost::sheets();
        let mut dialogs = dialog_host::AppDialogs::new(host.clone());
        let ctx = egui::Context::default();
        let mut mirror = jobs::Mirror::new(None);
        let ids = jobs::tests::ids(3);

        // A system job turns the spinner and shows in the window, if it is open; it opens
        // nothing.
        let thumbnails = jobs::tests::added(ids[0], Queue::SystemBackground, None);
        let (show, view) = jobs_after(&mut mirror, &[thumbnails]);
        jobs::refresh(&view, Vec::new(), &mut dialogs, show, None, &ctx);
        assert_eq!(view.len(), 1);
        assert!(host.0.borrow().jobs.is_none() && host.0.borrow().jobs_opened.is_empty());

        // The export of the user opens it, with the Export sheet still sliding away, and
        // without taking the focus from the window the user is working in.
        host.0.borrow_mut().shown = Some(dialog_host::Shown {
            form: dialog_host::Form::Sheet,
            focused: true,
        });
        let export = jobs::tests::added(ids[1], Queue::UserExport, Some(1));
        let (show, view) = jobs_after(&mut mirror, &[export]);
        jobs::refresh(&view, Vec::new(), &mut dialogs, show, None, &ctx);
        let hosted = host.0.borrow();
        assert_eq!(hosted.jobs_opened, [(None, false)]);
        assert_eq!(hosted.jobs_shown.len(), 2, "every job is listed");
        assert!(hosted.shown.is_some_and(|s| s.modal()), "the sheet stays");
        drop(hosted);

        // Another system job while it is open shows in it, and does not bring it forward.
        let later = jobs::tests::added(ids[2], Queue::SystemForeground, None);
        let (show, view) = jobs_after(&mut mirror, &[later]);
        jobs::refresh(&view, Vec::new(), &mut dialogs, show, None, &ctx);
        let hosted = host.0.borrow();
        assert_eq!(hosted.jobs_opened.len(), 1);
        assert_eq!(hosted.jobs_shown.len(), 3);
        assert_eq!(hosted.jobs_fronted, 0);
    }

    #[test]
    fn the_spinner_opens_the_jobs_window_or_brings_it_to_the_front() {
        use apertrust_engine::jobs::Queue;
        let host = FakeHost::default();
        let mut dialogs = dialog_host::AppDialogs::new(host.clone());
        let ctx = egui::Context::default();
        let mut mirror = jobs::Mirror::new(None);
        let id = jobs::tests::ids(1)[0];
        let thumbnails = jobs::tests::added(id, Queue::SystemBackground, None);
        let (_, view) = jobs_after(&mut mirror, &[thumbnails]);

        // Closed: a click opens it and it takes the focus.
        jobs::refresh(
            &view,
            Vec::new(),
            &mut dialogs,
            jobs::Show::Front,
            None,
            &ctx,
        );
        assert_eq!(host.0.borrow().jobs_opened, [(None, true)]);
        // Open but behind the main window: a click brings it forward.
        host.0.borrow_mut().jobs = Some(dialog_host::Shown {
            form: dialog_host::Form::Modeless,
            focused: false,
        });
        jobs::refresh(
            &view,
            Vec::new(),
            &mut dialogs,
            jobs::Show::Front,
            None,
            &ctx,
        );
        let hosted = host.0.borrow();
        assert_eq!(hosted.jobs_opened.len(), 1, "not opened twice");
        assert_eq!(hosted.jobs_fronted, 1);
        assert!(hosted.jobs.is_some_and(|s| s.focused));
    }

    /// Whether `window` closes itself, drawn as the host draws it every half second for
    /// five seconds, on the Mac without the focus and with no pointer over it, so nothing
    /// the person does keeps it open.
    fn closes_itself(window: jobs::Window) -> bool {
        let mut harness = Harness::builder()
            .with_os(OperatingSystem::Mac)
            .with_step_dt(0.5)
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
            .build_ui_state(
                |ui, (window, closed): &mut (jobs::Window, bool)| {
                    *closed |= dialog_host::draw(ui, window).close;
                },
                (jobs::Window::new(Vec::new()), false),
            );
        harness.run();
        harness.input_mut().focused = false;
        harness.input_mut().events.push(egui::Event::PointerGone);
        harness.state_mut().0 = window;
        harness.run_steps(10);
        harness.state().1
    }

    #[test]
    fn view_jobs_opens_the_jobs_window_or_brings_it_forward_and_it_never_closes_itself() {
        use apertrust_engine::jobs::{Queue, State};
        let host = FakeHost::default();
        let mut harness = dialogged(host.clone());
        harness.run();
        let ctx = harness.ctx.clone();
        let ids = jobs::tests::ids(2);
        // An export that has finished: its row waits in the window, which is closed.
        let mirror = &mut harness.state_mut().jobs;
        mirror.apply(&jobs::tests::added(ids[0], Queue::UserExport, Some(1)));
        mirror.apply(&jobs::tests::ended(ids[0], State::Finished, None));

        // Closed: View › Jobs opens it with the focus, the finished row in it.
        press_action(&mut harness, ActionId::SHOW_JOBS);
        let opened = host.0.borrow().jobs_window.clone().expect("opened");
        {
            let hosted = host.0.borrow();
            assert_eq!(hosted.jobs_opened, [(None, true)]);
            assert_eq!(hosted.jobs_shown.len(), 1);
            let finished = apertrust_ui::components::JobPhase::Finished;
            assert_eq!(hosted.jobs_shown[0].phase, finished);
        }
        // Its jobs have all ended, and still it stays: the person opened it.
        assert!(!closes_itself(opened));

        // Open but behind the main window: it comes to the front, not opened twice.
        host.0.borrow_mut().jobs.as_mut().expect("open").focused = false;
        press_action(&mut harness, ActionId::SHOW_JOBS);
        {
            let hosted = host.0.borrow();
            assert_eq!(hosted.jobs_opened.len(), 1);
            assert_eq!(hosted.jobs_fronted, 1);
            assert!(hosted.jobs.is_some_and(|s| s.focused));
        }

        // Closed again, a new export opens it by itself, to close once that export ends...
        host.0.borrow_mut().jobs = None;
        let mirror = &mut harness.state_mut().jobs;
        let show = mirror.apply(&jobs::tests::added(ids[1], Queue::UserExport, Some(2)));
        assert_eq!(show, jobs::Show::Open);
        let view = mirror.view(&Translator::new(Locale::EnGb));
        let opened = mirror.opened();
        let dialogs = &mut harness.state_mut().dialogs;
        jobs::refresh(&view, opened, dialogs, show, None, &ctx);
        let mirror = &mut harness.state_mut().jobs;
        mirror.apply(&jobs::tests::ended(ids[1], State::Finished, None));
        let view = mirror.view(&Translator::new(Locale::EnGb));
        let dialogs = &mut harness.state_mut().dialogs;
        jobs::refresh(&view, Vec::new(), dialogs, jobs::Show::Update, None, &ctx);
        let by_itself = host.0.borrow().jobs_window.clone().expect("opened");
        assert!(closes_itself(by_itself));
        // ...but brought to the front by View › Jobs, it stays.
        press_action(&mut harness, ActionId::SHOW_JOBS);
        let fronted = host.0.borrow().jobs_window.clone().expect("open");
        assert!(!closes_itself(fronted));
    }

    #[test]
    fn closing_the_jobs_window_stops_no_job_and_a_later_user_job_opens_it_again() {
        use apertrust_engine::jobs::Queue;
        let host = FakeHost::default();
        let mut harness = dialogged(host.clone());
        harness.run();
        let ctx = harness.ctx.clone();
        let mut mirror = jobs::Mirror::new(None);
        let ids = jobs::tests::ids(2);
        let export = jobs::tests::added(ids[0], Queue::UserExport, Some(1));
        let (show, view) = jobs_after(&mut mirror, &[export]);
        jobs::refresh(
            &view,
            Vec::new(),
            &mut harness.state_mut().dialogs,
            show,
            None,
            &ctx,
        );
        assert!(host.0.borrow().jobs.is_some());

        // The user closes the window: the host reports it gone.
        host.0.borrow_mut().jobs = None;
        let state = harness.state_mut();
        let app = state.shell.state(&ctx, true, true);
        let taken = take_dialogs(&mut state.shell, &mut state.dialogs, &app, &ctx);
        // Nothing was asked of any job, and the job and its row are as they were.
        assert!(taken.jobs.is_empty() && !taken.quit);
        let view = mirror.view(&Translator::new(Locale::EnGb));
        assert_eq!(view.len(), 1);
        assert!(view[0].phase.active() && view[0].cancellable);

        // The job goes on, and the next user job opens the window once more.
        let progress = Event::Job {
            event: apertrust_engine::session::JobEvent::Progress {
                job: ids[0],
                done: 1,
                total: Some(2),
            },
            command: None,
        };
        assert_eq!(mirror.apply(&progress), jobs::Show::Update);
        let import = jobs::tests::added(ids[1], Queue::UserForeground, Some(2));
        let (show, view) = jobs_after(&mut mirror, &[import]);
        jobs::refresh(
            &view,
            Vec::new(),
            &mut harness.state_mut().dialogs,
            show,
            None,
            &ctx,
        );
        assert_eq!(host.0.borrow().jobs_opened.len(), 2);
    }

    #[test]
    fn the_jobs_window_opens_where_the_user_left_it_kept_in_settings() {
        use apertrust_engine::jobs::Queue;
        let host = FakeHost::default();
        let mut harness = dialogged(host.clone());
        harness.run();
        let ctx = harness.ctx.clone();
        let left = settings::Position {
            pixels: [1200.0, 160.0],
            scale_factor: 2.0,
        };
        // The host reports the place as the window goes.
        host.0.borrow_mut().outputs = vec![dialog_host::Output::JobsPlaced(left)];
        harness.step();
        let shell = &harness.state().shell;
        assert_eq!(shell.jobs_window, Some(left));
        assert_eq!(shell.to_settings(&ctx).jobs_window, Some(left));
        assert_eq!(shell.to_settings(&ctx).settings_window, None);

        // Opened again, there; the host keeps it on a screen or centres it.
        let mut mirror = jobs::Mirror::new(None);
        let id = jobs::tests::ids(1)[0];
        let export = jobs::tests::added(id, Queue::UserExport, Some(1));
        let (show, view) = jobs_after(&mut mirror, &[export]);
        let place = harness.state().shell.jobs_window;
        jobs::refresh(
            &view,
            Vec::new(),
            &mut harness.state_mut().dialogs,
            show,
            place,
            &ctx,
        );
        assert_eq!(host.0.borrow().jobs_opened, [(Some(left), false)]);

        // And on the next start, from the settings.
        let kept = Settings {
            jobs_window: Some(left),
            ..Settings::default()
        };
        let shell = Shell::new(&kept, Locale::EnGb, OperatingSystem::Mac);
        assert_eq!(shell.jobs_window, Some(left));
    }

    #[test]
    fn the_jobs_window_s_place_reaches_the_file_when_quit_closes_it() {
        let folder = settings::testing::Folder::new("jobs-window-quit");
        let (mut keeper, kept) = Keeper::open(folder.storage());
        let mut shell = Shell::new(&kept, Locale::EnGb, OperatingSystem::Mac);
        let host = FakeHost::default();
        let mut dialogs = dialog_host::AppDialogs::new(host.clone());
        let ctx = egui::Context::default();
        // The jobs window is open; Quit closes it and waits for it to go.
        host.0.borrow_mut().jobs = Some(dialog_host::Shown {
            form: dialog_host::Form::Modeless,
            focused: false,
        });
        assert!(!dialogs.quit());
        assert!(host.0.borrow().jobs_closing);
        // It goes, reporting where it was left, as the host does only as it closes.
        let left = settings::Position {
            pixels: [1200.0, 160.0],
            scale_factor: 2.0,
        };
        host.0.borrow_mut().jobs = None;
        host.0.borrow_mut().outputs = vec![dialog_host::Output::JobsPlaced(left)];
        let app = shell.state(&ctx, true, true);
        let taken = take_dialogs(&mut shell, &mut dialogs, &app, &ctx);
        assert!(taken.quit);
        settle(taken.quit, true, &mut shell, &mut keeper, &ctx);
        keeper.flush();
        assert!(folder.text().contains("[jobs_window]"), "{}", folder.text());
        let (_, read) = Keeper::open(folder.storage());
        assert_eq!(read.jobs_window, Some(left));
    }

    #[test]
    fn close_window_closes_the_jobs_window_with_the_focus_and_quit_closes_it_too() {
        let host = FakeHost::default();
        let mut harness = dialogged(host.clone());
        harness.run();
        // Settings and the jobs window open together, the jobs window with the focus.
        press_action(&mut harness, ActionId::SETTINGS);
        host.0.borrow_mut().jobs = Some(dialog_host::Shown {
            form: dialog_host::Form::Modeless,
            focused: true,
        });
        host.focus(false);
        press_action(&mut harness, ActionId::CLOSE_WINDOW);
        assert!(host.0.borrow().jobs_closing);
        assert!(!host.0.borrow().closing, "Settings stays");
        host.0.borrow_mut().jobs_closing = false;

        // Settings with the focus: it closes, the jobs window stays.
        host.0.borrow_mut().jobs = Some(dialog_host::Shown {
            form: dialog_host::Form::Modeless,
            focused: false,
        });
        host.focus(true);
        press_action(&mut harness, ActionId::CLOSE_WINDOW);
        assert!(host.0.borrow().closing);
        assert!(!host.0.borrow().jobs_closing);
        host.gone(None);
        harness.step();

        // Quit with only the jobs window open closes it, and the app leaves once it has gone.
        press_action(&mut harness, ActionId::QUIT);
        assert!(host.0.borrow().jobs_closing);
        assert!(!closing(&harness), "not while the jobs window is open");
        host.0.borrow_mut().jobs = None;
        harness.step();
        assert!(closing(&harness));
    }

    #[test]
    fn the_jobs_spinner_asks_for_the_window_and_is_there_only_while_there_are_jobs() {
        use apertrust_ui::components::{Job, JobMessage, JobPhase};
        let shown = |jobs: Vec<Job>| {
            let shell = Shell::new(&Settings::default(), Locale::EnGb, OperatingSystem::Mac);
            let asked = std::cell::RefCell::new(Vec::new());
            let mut harness = Harness::builder()
                .with_size(egui::vec2(1200.0, 200.0))
                .with_os(OperatingSystem::Mac)
                .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
                .build_ui(|ui| {
                    let state = shell.state(ui.ctx(), true, false);
                    let top = shell.top_panel(ui, &state, SaveState::None, None, None, &jobs);
                    asked.borrow_mut().extend(top);
                });
            harness.run();
            let spinner = harness.query_by_label("Jobs").is_some();
            if spinner {
                harness.get_by_label("Jobs").click();
                harness.run();
            }
            drop(harness);
            (spinner, asked.into_inner())
        };
        let (spinner, asked) = shown(Vec::new());
        assert!(!spinner && asked.is_empty());
        let job = Job::new(1, JobPhase::Running, JobMessage::new("job-thumbnails"));
        let (spinner, asked) = shown(vec![job]);
        assert!(spinner);
        assert_eq!(asked, [FromTopPanel::Jobs]);
    }

    #[test]
    fn export_opens_its_sheet_with_the_settings_as_the_shell_keeps_them() {
        let host = FakeHost::sheets();
        let mut harness = dialogged(host.clone());
        harness.run();
        let kept = export::Values {
            path: "~/proofs/$(FILE_NAME)".to_owned(),
            ..export::Values::default()
        };
        harness.state_mut().shell.export = kept.clone();
        press_action(&mut harness, ActionId::EXPORT);
        assert_eq!(host.0.borrow().exports, [kept]);
        let shown = host.0.borrow().shown;
        assert_eq!(shown.map(|s| s.form), Some(dialog_host::Form::Sheet));
        // One dialog at a time: asked again, nothing opens.
        press_action(&mut harness, ActionId::EXPORT);
        assert_eq!(host.0.borrow().exports.len(), 1);
    }

    #[test]
    fn export_is_disabled_with_no_photo_open_on_every_platform() {
        let ctx = egui::Context::default();
        let shell = Shell::new(&Settings::default(), Locale::EnGb, OperatingSystem::Mac);
        // Export Again does what the dialog's Export does, so it is available as it is.
        // The host shows the dialog everywhere: a sheet on the Mac, a modal window
        // elsewhere.
        for id in [ActionId::EXPORT, ActionId::EXPORT_AGAIN] {
            let export = actions::action(id).expect("declared");
            for os in [
                OperatingSystem::Mac,
                OperatingSystem::Windows,
                OperatingSystem::Nix,
            ] {
                ctx.set_os(os);
                assert!(
                    export.enabled(&shell.state(&ctx, true, false)),
                    "{id:?} {os:?}"
                );
                assert!(
                    !export.enabled(&shell.state(&ctx, false, false)),
                    "{id:?} {os:?}"
                );
            }
        }
    }

    #[test]
    fn the_settings_the_export_dialog_changes_are_kept_by_the_shell_and_its_asks_reach_the_app() {
        let host = FakeHost::sheets();
        let mut harness = dialogged(host.clone());
        harness.run();
        let changed = export::Values {
            format: apertrust_export::settings::FileFormat::Png,
            ..export::Values::default()
        };
        let asked = export::Asked {
            values: Some(changed.clone()),
            start: false,
            confirm: false,
            opened: true,
        };
        host.0.borrow_mut().outputs = vec![dialog_host::Output::Export(asked.clone())];
        let ctx = harness.ctx.clone();
        let state = harness.state_mut();
        let app = state.shell.state(&ctx, true, true);
        let (shell, dialogs) = (&mut state.shell, &mut state.dialogs);
        let taken = take_dialogs(shell, dialogs, &app, &ctx);
        assert!(!taken.quit);
        assert_eq!(taken.export, [asked]);
        // What the shell keeps is what settings write.
        assert_eq!(shell.export, changed);
        assert_eq!(shell.to_settings(&ctx).export, changed);
    }

    #[test]
    fn settings_opens_in_a_window_of_its_own_and_comes_to_the_front_when_asked_again() {
        let host = FakeHost::default();
        let mut harness = dialogged(host.clone());
        harness.run();
        let ctx = harness.ctx.clone();
        let d = harness.state_mut();
        set_in_settings(&mut d.shell, &mut d.keeper, "theme", "light", &ctx);
        press_action(&mut harness, ActionId::SETTINGS);
        // The host opened it with the values kept, no Close button, as the Mac's Settings
        // windows have none, centred the first time, and nothing is drawn over the main window.
        let opened = host.0.borrow().settings.clone();
        assert_eq!(opened.len(), 1);
        let (settings, place) = opened[0].clone();
        let theme = settings.row("theme").expect("the theme's row");
        assert_eq!(theme.value, Some("light".into()));
        assert!(!settings.close_button);
        assert_eq!(place, None);
        assert!(!harness.state().shell.settings);
        harness.run();
        assert!(harness.query_by_label("Close").is_none());
        // Asked again from the main window, the open one comes to the front: no second.
        host.focus(false);
        press_action(&mut harness, ActionId::SETTINGS);
        assert_eq!(host.0.borrow().settings.len(), 1);
        assert_eq!(host.0.borrow().fronted, 1);
        assert!(host.0.borrow().shown.is_some_and(|s| s.focused));
    }

    #[test]
    fn settings_opens_where_it_was_last_left_kept_in_settings() {
        let host = FakeHost::default();
        let mut harness = dialogged(host.clone());
        harness.run();
        press_action(&mut harness, ActionId::SETTINGS);
        // Closed where the user left it: the host reports the place as the window goes.
        let left = settings::Position {
            pixels: [960.0, 520.0],
            scale_factor: 2.0,
        };
        host.0.borrow_mut().outputs = vec![dialog_host::Output::Placed(left)];
        host.gone(None);
        harness.step();
        let shell = &harness.state().shell;
        assert_eq!(shell.settings_window, Some(left));
        assert_eq!(shell.to_settings(&harness.ctx).settings_window, Some(left));
        // Opened again, there; the host keeps it on a screen or centres it.
        press_action(&mut harness, ActionId::SETTINGS);
        assert_eq!(host.0.borrow().settings[1].1, Some(left));
        // And on the next start, from the settings.
        let kept = Settings {
            settings_window: Some(left),
            ..Settings::default()
        };
        let shell = Shell::new(&kept, Locale::EnGb, OperatingSystem::Mac);
        assert_eq!(shell.settings_window, Some(left));
    }

    /// The theme, the language and the sliders' marker reach the main window at once.
    #[test]
    fn what_the_settings_window_changes_reaches_the_shell_and_the_main_window() {
        use settings_dialog::Change;
        let host = FakeHost::default();
        let mut harness = dialogged(host.clone());
        harness.run();
        press_action(&mut harness, ActionId::SETTINGS);
        host.0.borrow_mut().outputs = vec![
            dialog_host::Output::Settings(Change::Set("theme", "light".into())),
            dialog_host::Output::Settings(Change::Set("locale", "uk".into())),
            dialog_host::Output::Settings(Change::Set("sliders.marker", "circle".into())),
        ];
        harness.step();
        let shell = &harness.state().shell;
        assert_eq!(shell.theme, Builtin::Light);
        assert_eq!(shell.marker, Marker::Circle);
        assert_eq!(
            apertrust_ui::components::marker(&harness.ctx),
            Marker::Circle
        );
        assert_eq!(
            (shell.locale, shell.kept_locale),
            (Locale::Uk, Some(Locale::Uk))
        );
        assert_eq!(
            theme(&harness.ctx).colors,
            Theme::builtin(Builtin::Light).colors
        );
        assert_eq!(translator(&harness.ctx).locale(), Locale::Uk);
    }

    /// Square thumbnails, the Photos page's, and the side panel's tab names, the Editing
    /// page's, reach the shell at once; the engine's settings are kept for its next start
    /// and the page shown for the dialog's next opening.
    #[test]
    fn the_settings_window_s_other_rows_reach_the_grid_and_the_side_panel_at_once() {
        use settings_dialog::Change;
        let host = FakeHost::default();
        let mut harness = dialogged(host.clone());
        harness.run();
        press_action(&mut harness, ActionId::SETTINGS);
        assert!(!harness.state().shell.grid.square_thumbnails);
        host.0.borrow_mut().outputs = vec![
            dialog_host::Output::Settings(Change::Set("settings_window.page", "photos".into())),
            dialog_host::Output::Settings(Change::Set("grid_panel.square_thumbnails", true.into())),
            dialog_host::Output::Settings(Change::Set("side_panel.tab_names", false.into())),
            dialog_host::Output::Settings(Change::Set("engine.memory_budget", 4096.into())),
        ];
        harness.step();
        let d = harness.state();
        assert!(d.shell.grid.square_thumbnails);
        assert!(!d.shell.tab_names);
        assert_eq!(d.keeper.engine().budget, 4096 << 20);
        assert_eq!(d.keeper.page(), apertrust_base::settings::Page::Photos);
        // And back to its default.
        host.0.borrow_mut().outputs = vec![dialog_host::Output::Settings(Change::Reset(
            "grid_panel.square_thumbnails",
        ))];
        harness.step();
        assert!(!harness.state().shell.grid.square_thumbnails);
    }

    #[test]
    fn close_window_closes_the_focused_dialog_s_window_or_else_the_main_window() {
        let host = FakeHost::default();
        let mut harness = dialogged(host.clone());
        harness.run();
        press_action(&mut harness, ActionId::SETTINGS);
        // Settings' window has the focus: Cmd+W closes it, and the app goes on.
        press_action(&mut harness, ActionId::CLOSE_WINDOW);
        assert!(host.0.borrow().closing);
        host.gone(None);
        harness.step();
        harness.step();
        assert!(!closing(&harness));
        // Open again, the main window focused: Settings closes first, then the main window.
        press_action(&mut harness, ActionId::SETTINGS);
        host.focus(false);
        press_action(&mut harness, ActionId::CLOSE_WINDOW);
        assert!(host.0.borrow().closing);
        assert!(!closing(&harness), "not while Settings is open");
        host.gone(None);
        harness.step();
        assert!(closing(&harness));
        // With no dialog, at once.
        let mut harness = dialogged(FakeHost::default());
        harness.run();
        press_action(&mut harness, ActionId::CLOSE_WINDOW);
        assert!(closing(&harness));
    }

    #[test]
    fn the_main_window_s_close_button_closes_the_dialog_first() {
        let close = |harness: &mut Harness<'_, Dialogged>| {
            let input = harness.input_mut();
            let root = input.viewports.entry(egui::ViewportId::ROOT).or_default();
            root.events.push(egui::ViewportEvent::Close);
            harness.step();
        };
        let cancelled = |harness: &Harness<'_, Dialogged>| {
            commanded(harness, &egui::ViewportCommand::CancelClose)
        };
        // With no dialog, the window closes as eframe closes it.
        let mut harness = dialogged(FakeHost::default());
        harness.run();
        close(&mut harness);
        assert!(!cancelled(&harness));
        // With Settings open, its close waits for Settings to go, then comes again.
        let host = FakeHost::default();
        let mut harness = dialogged(host.clone());
        harness.run();
        press_action(&mut harness, ActionId::SETTINGS);
        host.focus(false);
        close(&mut harness);
        assert!(cancelled(&harness));
        assert!(host.0.borrow().closing);
        harness.step();
        assert!(!closing(&harness));
        host.gone(None);
        harness.step();
        assert!(closing(&harness));
    }

    // The alerts (`docs/gui.md`, Dialogs and alerts). The system's alert cannot be shown in
    // a test: these drive the app's decisions with a host that records what it is asked and
    // is answered as a person would.

    /// A host whose platform shows alerts.
    fn with_alerts() -> FakeHost {
        let host = FakeHost::default();
        host.0.borrow_mut().native_alerts = true;
        host
    }

    /// A host that draws its alerts, as Linux's does.
    fn with_drawn_alerts() -> FakeHost {
        let host = with_alerts();
        host.0.borrow_mut().drawn_alerts = true;
        host
    }

    /// The person answers the alert up, which goes.
    fn answer_alert(host: &FakeHost, answer: dialog_host::Answer) {
        let mut hosted = host.0.borrow_mut();
        hosted.answer = Some(answer);
        hosted.alert = None;
    }

    /// An export of the user's starts running in the harness's jobs.
    fn start_export(harness: &mut Harness<'_, Dialogged>) -> apertrust_engine::jobs::JobId {
        let job = jobs::tests::ids(1)[0];
        let event = jobs::tests::added(job, apertrust_engine::jobs::Queue::UserExport, None);
        harness.state_mut().jobs.apply(&event);
        job
    }

    /// The export ends, cancelled as the engine reports it.
    fn end_export(harness: &mut Harness<'_, Dialogged>, job: apertrust_engine::jobs::JobId) {
        let event = jobs::tests::ended(job, apertrust_engine::jobs::State::Cancelled, None);
        harness.state_mut().jobs.apply(&event);
    }

    fn asked_alerts(host: &FakeHost) -> usize {
        host.0.borrow().alerts.len()
    }

    #[test]
    fn quit_with_nothing_running_asks_nothing_and_a_system_job_does_not_count() {
        let host = with_alerts();
        let mut harness = dialogged(host.clone());
        harness.run();
        press_action(&mut harness, ActionId::QUIT);
        assert!(closing(&harness));
        assert_eq!(asked_alerts(&host), 0);

        let host = with_alerts();
        let mut harness = dialogged(host.clone());
        harness.run();
        let job = jobs::tests::ids(1)[0];
        let thumbnails =
            jobs::tests::added(job, apertrust_engine::jobs::Queue::SystemBackground, None);
        harness.state_mut().jobs.apply(&thumbnails);
        press_action(&mut harness, ActionId::QUIT);
        assert!(closing(&harness));
        assert_eq!(asked_alerts(&host), 0);
    }

    #[test]
    fn quit_waits_for_the_edit_s_write_and_leaves_once_it_is_saved() {
        let host = with_alerts();
        let mut harness = dialogged(host.clone());
        harness.run();
        harness.state_mut().edit = TestEdit::Writing;
        press_action(&mut harness, ActionId::QUIT);
        harness.step();
        assert!(!closing(&harness));
        assert_eq!(asked_alerts(&host), 0);
        // Confirmed: Quit runs again by itself, and leaves.
        harness.state_mut().edit = TestEdit::Saved;
        harness.step();
        assert!(closing(&harness));
        assert_eq!(asked_alerts(&host), 0);
    }

    #[test]
    fn quit_after_a_failed_write_asks_with_cancel_the_default_and_quit_anyway() {
        let host = with_alerts();
        let mut harness = dialogged(host.clone());
        harness.run();
        // The last write is under way, then fails: Quit asks once it knows.
        harness.state_mut().edit = TestEdit::Writing;
        press_action(&mut harness, ActionId::QUIT);
        harness.state_mut().edit = TestEdit::Failed("disk full".to_owned());
        harness.step();
        assert!(!closing(&harness));
        assert_eq!(asked_alerts(&host), 1);
        let request = host.0.borrow().alerts[0].clone();
        assert_eq!(request.title, "Quit without your last changes?");
        assert_eq!(
            request.message,
            "Your last changes couldn’t be saved: disk full"
        );
        assert_eq!(request.buttons[0].label, "Quit Anyway");
        assert_eq!((request.default, request.cancel()), (1, Some(1)));
        // Cancel stays; the next Quit asks again, and Quit Anyway leaves.
        answer_alert(&host, dialog_host::Answer::Pressed(1));
        harness.step();
        assert!(!closing(&harness));
        press_action(&mut harness, ActionId::QUIT);
        assert_eq!(asked_alerts(&host), 2);
        answer_alert(&host, dialog_host::Answer::Pressed(0));
        harness.step();
        assert!(closing(&harness));
    }

    #[test]
    fn quit_with_an_export_running_asks_once_and_stays_while_the_alert_is_up() {
        let host = with_alerts();
        let mut harness = dialogged(host.clone());
        harness.run();
        start_export(&mut harness);
        press_action(&mut harness, ActionId::QUIT);
        assert!(!closing(&harness));
        assert_eq!(asked_alerts(&host), 1);
        let request = host.0.borrow().alerts[0].clone();
        assert_eq!(request.title, "Quit while a job is running?");
        assert_eq!(request.buttons[0].label, "Quit");
        assert_eq!(
            request.buttons[0].role,
            apertrust_ui::alert::Role::Destructive
        );
        assert_eq!(request.cancel(), Some(1));
        // Quit again, Close Window, the window's close button: none asks a second time, and
        // none leaves.
        press_action(&mut harness, ActionId::QUIT);
        press_action(&mut harness, ActionId::CLOSE_WINDOW);
        assert_eq!(asked_alerts(&host), 1);
        assert!(!closing(&harness));
        assert!(harness.state().stopped.is_empty());
    }

    #[test]
    fn under_a_drawn_alert_quit_and_close_window_do_nothing_as_under_the_mac_s() {
        // The quit alert up, as Linux draws it: Quit, Close Window and the window's close
        // button neither take it away nor ask it again, frame after frame.
        let host = with_drawn_alerts();
        let mut harness = dialogged(host.clone());
        harness.run();
        start_export(&mut harness);
        press_action(&mut harness, ActionId::QUIT);
        assert_eq!(asked_alerts(&host), 1);
        assert!(host.0.borrow().alert.is_some());
        press_action(&mut harness, ActionId::QUIT);
        press_action(&mut harness, ActionId::CLOSE_WINDOW);
        harness.step();
        harness.step();
        assert_eq!(asked_alerts(&host), 1);
        assert!(host.0.borrow().alert.is_some(), "still up");
        assert!(host.0.borrow().answer.is_none(), "not dismissed");
        assert!(!closing(&harness));
        // The save warning's alert up: Quit leaves it as it is, never the quit alert.
        let host = with_drawn_alerts();
        let mut harness = dialogged(host.clone());
        harness.run();
        harness.state_mut().edit = TestEdit::Failed("disk full".to_owned());
        let ctx = harness.ctx.clone();
        let request = alerts::save_request(&ctx, "photo.raw", "disk full");
        let state = harness.state_mut();
        let purpose = alerts::Purpose::Save;
        state
            .alerts
            .ask(&mut state.dialogs, purpose, &request, &ctx);
        press_action(&mut harness, ActionId::QUIT);
        harness.step();
        assert_eq!(asked_alerts(&host), 1);
        assert_eq!(harness.state().alerts.pending(), Some(purpose));
        assert!(host.0.borrow().alert.is_some());
        assert!(!closing(&harness));
        // Over a modal dialog, Quit neither closes the alert nor the dialog, nor waits for
        // them: nothing is left to quit once they go.
        let host = FakeHost::on(OperatingSystem::Nix);
        {
            let mut hosted = host.0.borrow_mut();
            hosted.native_alerts = true;
            hosted.drawn_alerts = true;
        }
        let mut dialogs = dialog_host::AppDialogs::new(host.clone());
        let ctx = egui::Context::default();
        let export = export::Export::new(export::Values::default());
        dialogs.open(export, None, &ctx);
        let request = alerts::overwrite_request(&ctx, false);
        assert_eq!(dialogs.ask(&request, &ctx), dialog_host::Ask::Shown);
        assert!(!dialogs.quit());
        assert!(!host.0.borrow().closing);
        assert!(host.0.borrow().alert.is_some());
        answer_alert(&host, dialog_host::Answer::Pressed(1));
        assert!(!dialogs.frame(&ctx), "no Quit was held");
    }

    #[test]
    fn under_a_drawn_alert_a_press_on_the_main_window_raises_the_alert() {
        let host = with_drawn_alerts();
        let mut dialogs = dialog_host::AppDialogs::new(host.clone());
        let ctx = egui::Context::default();
        let request = alerts::save_request(&ctx, "photo.raw", "disk full");
        assert_eq!(dialogs.ask(&request, &ctx), dialog_host::Ask::Shown);
        host.0.borrow_mut().alert.as_mut().expect("up").focused = false;
        // As `raw_input_hook` asks it on Linux, where the host draws the alert.
        assert!(raises_under_modal(
            dialogs.shown().is_some_and(|shown| shown.modal()),
            true,
            OperatingSystem::Nix
        ));
        dialogs.raise();
        assert_eq!(host.0.borrow().alert_fronted, 1);
        assert!(host.0.borrow().alert.expect("up").focused);
    }

    #[test]
    fn cancelling_the_quit_alert_keeps_the_app_and_the_job_and_the_next_quit_asks_again() {
        for answer in [
            dialog_host::Answer::Pressed(1),
            dialog_host::Answer::Dismissed,
        ] {
            let host = with_alerts();
            let mut harness = dialogged(host.clone());
            harness.run();
            start_export(&mut harness);
            press_action(&mut harness, ActionId::QUIT);
            answer_alert(&host, answer);
            harness.step();
            assert!(!closing(&harness), "{answer:?}");
            assert!(harness.state().stopped.is_empty(), "{answer:?}");
            assert!(!harness.state().quit.confirmed(), "{answer:?}");
            press_action(&mut harness, ActionId::QUIT);
            assert_eq!(asked_alerts(&host), 2, "{answer:?}");
        }
    }

    #[test]
    fn confirming_the_quit_alert_stops_the_export_and_the_app_leaves_once_it_has_ended() {
        let host = with_alerts();
        let mut harness = dialogged(host.clone());
        harness.run();
        let job = start_export(&mut harness);
        press_action(&mut harness, ActionId::QUIT);
        answer_alert(&host, dialog_host::Answer::Pressed(0));
        harness.step();
        // The export is asked to stop, and the app stays until the engine says it has.
        assert_eq!(harness.state().stopped, [job]);
        assert!(!closing(&harness));
        harness.step();
        assert!(!closing(&harness));
        end_export(&mut harness, job);
        harness.step();
        assert!(closing(&harness));
        assert_eq!(asked_alerts(&host), 1);
    }

    #[test]
    fn quit_after_the_engine_stops_mid_export_leaves_without_asking_or_waiting() {
        let host = with_alerts();
        let mut harness = dialogged(host.clone());
        harness.run();
        start_export(&mut harness);
        // A second export sent and never added, as when the engine dies before adding it.
        harness.state_mut().jobs.starting(8);
        let show = harness.state_mut().jobs.engine_stopped();
        assert_eq!(show, jobs::Show::Update);
        press_action(&mut harness, ActionId::QUIT);
        assert!(closing(&harness));
        assert_eq!(asked_alerts(&host), 0);
        assert!(harness.state().stopped.is_empty());
    }

    #[test]
    fn confirming_waits_for_a_job_that_cannot_be_stopped_and_stops_only_the_others() {
        let host = with_alerts();
        let mut harness = dialogged(host.clone());
        harness.run();
        // Two numbers from one control, since each control numbers its own jobs from 1.
        let numbers = jobs::tests::ids(2);
        let (export, stuck) = (numbers[0], numbers[1]);
        let event = jobs::tests::added(export, apertrust_engine::jobs::Queue::UserExport, None);
        harness.state_mut().jobs.apply(&event);
        let delete = jobs::tests::added_stoppable(
            stuck,
            apertrust_engine::jobs::Queue::UserForeground,
            None,
            false,
        );
        harness.state_mut().jobs.apply(&delete);
        press_action(&mut harness, ActionId::QUIT);
        // The message says both: some stop, the app waits for the others.
        let message = host.0.borrow().alerts[0].message.clone();
        assert!(
            message.contains("once the others have finished"),
            "{message}"
        );
        answer_alert(&host, dialog_host::Answer::Pressed(0));
        harness.step();
        assert_eq!(harness.state().stopped, [export]);
        end_export(&mut harness, export);
        harness.step();
        assert!(!closing(&harness), "the delete still runs");
        let ended = jobs::tests::ended(stuck, apertrust_engine::jobs::State::Finished, None);
        harness.state_mut().jobs.apply(&ended);
        harness.step();
        assert!(closing(&harness));
    }

    #[test]
    fn the_window_s_close_button_asks_the_quit_alert_too() {
        let host = with_alerts();
        let mut harness = dialogged(host.clone());
        harness.run();
        start_export(&mut harness);
        let input = harness.input_mut();
        let root = input.viewports.entry(egui::ViewportId::ROOT).or_default();
        root.events.push(egui::ViewportEvent::Close);
        harness.step();
        // The close is cancelled, and the app asks instead.
        assert!(commanded(&harness, &egui::ViewportCommand::CancelClose));
        assert!(!closing(&harness));
        assert_eq!(asked_alerts(&host), 1);
    }

    #[test]
    fn where_the_platform_shows_no_alert_quit_leaves_as_it_did() {
        let host = FakeHost::default();
        let mut harness = dialogged(host.clone());
        harness.run();
        start_export(&mut harness);
        press_action(&mut harness, ActionId::QUIT);
        harness.step();
        assert!(closing(&harness));
        assert_eq!(asked_alerts(&host), 0);
    }

    #[test]
    fn a_dialog_is_closed_before_the_quit_alert_rises() {
        let host = FakeHost::sheets();
        host.0.borrow_mut().native_alerts = true;
        let mut harness = dialogged(host.clone());
        harness.run();
        start_export(&mut harness);
        press_action(&mut harness, ActionId::SETTINGS);
        press_action(&mut harness, ActionId::QUIT);
        // The sheet closes as Cancel and Quit waits for it, the alert still not asked.
        assert!(host.0.borrow().closing);
        assert_eq!(asked_alerts(&host), 0);
        host.gone(None);
        harness.step();
        assert_eq!(asked_alerts(&host), 1);
        assert!(!closing(&harness));
    }

    #[test]
    fn the_overwrite_question_sends_the_export_on_a_yes_and_leaves_the_dialog_on_a_no() {
        let ask = |harness: &mut Harness<'_, Dialogged>| {
            let ctx = harness.ctx.clone();
            let request = alerts::overwrite_request(&ctx, false);
            let state = harness.state_mut();
            let asked = state.alerts.ask(
                &mut state.dialogs,
                alerts::Purpose::Overwrite,
                &request,
                &ctx,
            );
            assert_eq!(asked, dialog_host::Ask::Shown);
            assert_eq!(state.alerts.pending(), Some(alerts::Purpose::Overwrite));
        };
        let host = with_alerts();
        let mut harness = dialogged(host.clone());
        harness.run();
        ask(&mut harness);
        let request = host.0.borrow().alerts[0].clone();
        assert_eq!(request.title, "Export in overwrite mode?");
        assert_eq!(request.buttons[0].label, "Overwrite");
        // Until it is answered nothing is sent, and the dialog stays.
        harness.step();
        assert_eq!(harness.state().exported, 0);
        assert!(!host.0.borrow().closing);
        answer_alert(&host, dialog_host::Answer::Pressed(0));
        harness.step();
        assert_eq!(harness.state().exported, 1);
        assert!(host.0.borrow().closing, "the dialog closes with the export");
        assert!(harness.state().alerts.pending().is_none());

        for answer in [
            dialog_host::Answer::Pressed(1),
            dialog_host::Answer::Dismissed,
        ] {
            let host = with_alerts();
            let mut harness = dialogged(host.clone());
            harness.run();
            ask(&mut harness);
            answer_alert(&host, answer);
            harness.step();
            assert_eq!(harness.state().exported, 0, "{answer:?}");
            assert!(!host.0.borrow().closing, "{answer:?}");
            assert!(harness.state().alerts.pending().is_none());
        }
    }

    /// The settings the Export dialog left, as the shell keeps them.
    fn kept(
        harness: &mut Harness<'_, Dialogged>,
        on_conflict: apertrust_export::settings::OnConflict,
    ) -> export::Values {
        let values = export::Values {
            on_conflict,
            jpeg_quality: 64,
            ..export::Values::default()
        };
        harness.state_mut().shell.export = values.clone();
        values
    }

    #[test]
    fn export_again_exports_the_settings_as_last_kept_at_once_where_it_asks_nothing() {
        use apertrust_export::settings::OnConflict;
        use export_again::tests::PHOTO;
        // Only Overwrite asks (darktable's rule): the others export at once, a file there
        // or not, and no dialog opens.
        for conflict in [
            OnConflict::CreateUniqueFilename,
            OnConflict::OverwriteIfChanged,
            OnConflict::Skip,
        ] {
            let host = with_alerts();
            let mut harness = dialogged(host.clone());
            harness.run();
            let values = kept(&mut harness, conflict);
            press_action(&mut harness, ActionId::EXPORT_AGAIN);
            let sent = harness.state().sent.clone();
            let [
                Command::Export {
                    photos, settings, ..
                },
            ] = &sent[..]
            else {
                panic!("the export, with no preview first: {sent:?}");
            };
            assert_eq!(photos, &[PathBuf::from(PHOTO)], "{conflict:?}");
            assert_eq!(settings, &values.settings(), "{conflict:?}");
            assert!(host.0.borrow().exports.is_empty());
            assert!(host.0.borrow().alerts.is_empty(), "{conflict:?}");
            assert!(harness.state().alerts.pending().is_none());
        }
    }

    /// The dialog's Export with a file to replace, as `App::export` takes it.
    fn export_replacing(harness: &mut Harness<'_, Dialogged>) {
        let ctx = harness.ctx.clone();
        ask_overwrite(harness.state_mut(), &ctx);
        harness.step();
    }

    #[test]
    fn a_yes_with_dont_ask_again_ticked_turns_the_question_off_and_nothing_is_asked_after() {
        let host = with_alerts();
        let mut harness = dialogged(host.clone());
        harness.run();
        assert!(harness.state().shell.overwrite.asks());
        export_replacing(&mut harness);
        assert_eq!(asked_alerts(&host), 1);
        let request = host.0.borrow().alerts[0].clone();
        assert_eq!(request.suppression.as_deref(), Some("Don’t ask again"));
        answer_alert(&host, dialog_host::Answer::Ticked(0));
        harness.step();
        // The export went, and the setting is off where the shell hands it to settings.
        assert_eq!(harness.state().exported, 1);
        assert!(!harness.state().shell.overwrite.asks());
        let ctx = harness.ctx.clone();
        assert!(!harness.state().shell.to_settings(&ctx).ask_before_overwrite);
        // The next Export with a file to replace goes ahead without a question, the dialog
        // closing with it.
        host.0.borrow_mut().closing = false;
        export_replacing(&mut harness);
        assert_eq!(asked_alerts(&host), 1, "no second question");
        assert_eq!(harness.state().exported, 2);
        assert!(harness.state().alerts.pending().is_none());
        assert!(host.0.borrow().closing);
    }

    #[test]
    fn a_yes_without_the_box_ticked_asks_again_next_time_and_so_does_a_ticked_no() {
        for answer in [
            dialog_host::Answer::Pressed(0),
            dialog_host::Answer::Ticked(1),
            dialog_host::Answer::Dismissed,
        ] {
            let host = with_alerts();
            let mut harness = dialogged(host.clone());
            harness.run();
            export_replacing(&mut harness);
            answer_alert(&host, answer);
            harness.step();
            assert!(harness.state().shell.overwrite.asks(), "{answer:?}");
            let ctx = harness.ctx.clone();
            assert!(harness.state().shell.to_settings(&ctx).ask_before_overwrite);
            export_replacing(&mut harness);
            // Only the yes exported; the second time asks too, and exports nothing yet.
            let exported = usize::from(answer == dialog_host::Answer::Pressed(0));
            assert_eq!(harness.state().exported, exported, "{answer:?}");
            assert_eq!(asked_alerts(&host), 2, "{answer:?}");
        }
    }

    #[test]
    fn a_setting_conf_holds_stays_as_it_is_whatever_the_alert_is_answered() {
        let host = with_alerts();
        let mut harness = dialogged(host.clone());
        harness.run();
        harness.state_mut().shell.overwrite = alerts::Overwrite::new(true, true);
        export_replacing(&mut harness);
        // The alert shows no box, since ticking it would do nothing.
        assert_eq!(host.0.borrow().alerts[0].suppression, None);
        answer_alert(&host, dialog_host::Answer::Ticked(0));
        harness.step();
        // The export went, and the question stays on: the store would refuse the write.
        assert_eq!(harness.state().exported, 1);
        assert!(harness.state().shell.overwrite.asks());
        export_replacing(&mut harness);
        assert_eq!(asked_alerts(&host), 2);
    }

    #[test]
    fn export_again_in_overwrite_mode_asks_whether_or_not_a_file_is_there_and_exports_only_on_a_yes()
     {
        use apertrust_export::settings::OnConflict;
        for (answer, exports) in [
            (dialog_host::Answer::Pressed(0), 1),
            (dialog_host::Answer::Pressed(1), 0),
            (dialog_host::Answer::Dismissed, 0),
        ] {
            let host = with_alerts();
            let mut harness = dialogged(host.clone());
            harness.run();
            kept(&mut harness, OnConflict::Overwrite);
            press_action(&mut harness, ActionId::EXPORT_AGAIN);
            // The question the dialog's Export asks, with no preview to decide it, and
            // nothing sent while it is up.
            let asked = host.0.borrow().alerts.clone();
            let ctx = harness.ctx.clone();
            assert_eq!(
                asked,
                [alerts::overwrite_request(&ctx, false)],
                "{answer:?}"
            );
            assert!(harness.state().sent.is_empty(), "{answer:?}");
            answer_alert(&host, answer);
            harness.step();
            assert_eq!(harness.state().sent.len(), exports, "{answer:?}");
            // No dialog was open, so none is closed.
            assert!(!host.0.borrow().closing, "{answer:?}");
            assert!(harness.state().alerts.pending().is_none());
            // Answered either way, it is not asked about again.
            assert_eq!(harness.state_mut().again.confirmed(), None);
        }
    }

    #[test]
    fn export_again_follows_the_setting_and_a_ticked_yes_turns_it_off_there_too() {
        use apertrust_export::settings::OnConflict;
        // Off already: Overwrite exports at once, with no alert.
        let host = with_alerts();
        let mut harness = dialogged(host.clone());
        harness.run();
        harness.state_mut().shell.overwrite = alerts::Overwrite::new(false, false);
        kept(&mut harness, OnConflict::Overwrite);
        press_action(&mut harness, ActionId::EXPORT_AGAIN);
        assert_eq!(asked_alerts(&host), 0);
        assert!(harness.state().alerts.pending().is_none());
        assert_eq!(harness.state().sent.len(), 1);
        // On: it asks, and a yes with the box ticked exports and turns the question off for
        // the Export Again after.
        let host = with_alerts();
        let mut harness = dialogged(host.clone());
        harness.run();
        kept(&mut harness, OnConflict::Overwrite);
        press_action(&mut harness, ActionId::EXPORT_AGAIN);
        assert_eq!(asked_alerts(&host), 1);
        assert!(harness.state().sent.is_empty());
        answer_alert(&host, dialog_host::Answer::Ticked(0));
        harness.step();
        assert_eq!(harness.state().sent.len(), 1);
        assert!(!harness.state().shell.overwrite.asks());
        press_action(&mut harness, ActionId::EXPORT_AGAIN);
        assert_eq!(asked_alerts(&host), 1, "no second question");
        assert_eq!(harness.state().sent.len(), 2);
    }

    #[test]
    fn export_again_sends_nothing_while_the_quit_alert_is_up_or_said_yes() {
        use apertrust_export::settings::OnConflict;
        let host = with_alerts();
        let mut harness = dialogged(host.clone());
        harness.run();
        kept(&mut harness, OnConflict::CreateUniqueFilename);
        // An export of the user's runs, so Quit asks.
        start_export(&mut harness);
        press_action(&mut harness, ActionId::QUIT);
        assert_eq!(
            harness.state().alerts.pending(),
            Some(alerts::Purpose::Quit)
        );
        // The alert is up: the action rests.
        press_action(&mut harness, ActionId::EXPORT_AGAIN);
        assert!(harness.state().sent.is_empty());
        // After a yes the app waits for the jobs it counted, and starts none.
        answer_alert(&host, dialog_host::Answer::Pressed(0));
        harness.step();
        press_action(&mut harness, ActionId::EXPORT_AGAIN);
        assert!(harness.state().sent.is_empty());
    }

    #[test]
    fn export_again_rests_with_no_photo_and_under_the_export_sheet_or_an_alert() {
        let host = FakeHost::sheets();
        let mut harness = dialogged(host.clone());
        harness.run();
        let ctx = harness.ctx.clone();
        let again = actions::action(ActionId::EXPORT_AGAIN).expect("declared");
        let state = harness.state().shell.state(&ctx, true, false);
        assert!(again.enabled(&state));
        assert!(!again.enabled(&harness.state().shell.state(&ctx, false, false)));
        // An alert is up.
        let asking = with_dialogs(state, true, None);
        assert!(!again.enabled(&asking));
        // The Export sheet is up: pressing it does nothing.
        press_action(&mut harness, ActionId::EXPORT);
        assert!(host.0.borrow().shown.is_some());
        press_action(&mut harness, ActionId::EXPORT_AGAIN);
        assert!(harness.state().sent.is_empty());
    }

    #[test]
    fn an_alert_is_not_asked_where_the_platform_shows_none() {
        let host = FakeHost::default();
        let mut harness = dialogged(host.clone());
        harness.run();
        let ctx = harness.ctx.clone();
        let request = alerts::overwrite_request(&ctx, false);
        let state = harness.state_mut();
        let asked = state.alerts.ask(
            &mut state.dialogs,
            alerts::Purpose::Overwrite,
            &request,
            &ctx,
        );
        assert_eq!(asked, dialog_host::Ask::NoAlerts);
        assert!(state.alerts.pending().is_none());
    }

    #[test]
    fn a_second_alert_is_refused_as_busy_and_asks_nothing() {
        let host = with_alerts();
        let mut harness = dialogged(host.clone());
        harness.run();
        let ctx = harness.ctx.clone();
        let request = alerts::overwrite_request(&ctx, false);
        let state = harness.state_mut();
        let purpose = alerts::Purpose::Overwrite;
        assert_eq!(
            state
                .alerts
                .ask(&mut state.dialogs, purpose, &request, &ctx),
            dialog_host::Ask::Shown
        );
        assert_eq!(
            state
                .alerts
                .ask(&mut state.dialogs, alerts::Purpose::Quit, &request, &ctx),
            dialog_host::Ask::Busy
        );
        // The first is still the one up, and the host was asked once.
        assert_eq!(state.alerts.pending(), Some(purpose));
        assert_eq!(asked_alerts(&host), 1);
    }

    #[test]
    fn an_export_sent_and_not_yet_added_is_asked_about_when_quitting() {
        // Cmd+Q right after Export, before the session has added the job.
        let host = with_alerts();
        let mut harness = dialogged(host.clone());
        harness.run();
        harness.state_mut().jobs.starting(7);
        press_action(&mut harness, ActionId::QUIT);
        assert!(!closing(&harness));
        assert_eq!(asked_alerts(&host), 1);
        // It cannot be stopped before it has a number: the message says the app waits.
        let message = host.0.borrow().alerts[0].message.clone();
        assert!(message.starts_with("Apertrust quits once"), "{message}");
        // A yes stops nothing yet and the app stays; once the job is added it is stoppable,
        // and once it has ended the app leaves.
        answer_alert(&host, dialog_host::Answer::Pressed(0));
        harness.step();
        assert!(harness.state().stopped.is_empty());
        assert!(!closing(&harness));
        let job = jobs::tests::ids(1)[0];
        let queue = apertrust_engine::jobs::Queue::UserExport;
        let added = jobs::tests::added(job, queue, Some(7));
        harness.state_mut().jobs.apply(&added);
        end_export(&mut harness, job);
        harness.step();
        assert!(closing(&harness));
    }

    #[test]
    fn quit_pressed_under_the_overwrite_alert_asks_the_quit_question_once_the_export_is_sent() {
        let host = with_alerts();
        let mut harness = dialogged(host.clone());
        harness.run();
        let ctx = harness.ctx.clone();
        let request = alerts::overwrite_request(&ctx, false);
        let state = harness.state_mut();
        let purpose = alerts::Purpose::Overwrite;
        state
            .alerts
            .ask(&mut state.dialogs, purpose, &request, &ctx);
        // Quit waits for the answer: no second alert, no leaving.
        press_action(&mut harness, ActionId::QUIT);
        assert_eq!(asked_alerts(&host), 1);
        assert!(!closing(&harness));
        // A yes sends the export, which has no job yet, and the held Quit asks about it.
        answer_alert(&host, dialog_host::Answer::Pressed(0));
        harness.step();
        assert_eq!(harness.state().exported, 1);
        assert_eq!(asked_alerts(&host), 2);
        assert_eq!(
            host.0.borrow().alerts[1].title,
            "Quit while a job is running?"
        );
        assert!(!closing(&harness));
        // A no to the overwrite question leaves the held Quit to leave, with nothing running.
        let host = with_alerts();
        let mut harness = dialogged(host.clone());
        harness.run();
        let ctx = harness.ctx.clone();
        let state = harness.state_mut();
        state
            .alerts
            .ask(&mut state.dialogs, purpose, &request, &ctx);
        press_action(&mut harness, ActionId::QUIT);
        answer_alert(&host, dialog_host::Answer::Pressed(1));
        harness.step();
        assert_eq!(asked_alerts(&host), 1);
        assert!(closing(&harness));
    }

    #[test]
    fn when_showing_the_quit_alert_fails_that_quit_leaves_and_the_next_asks_again() {
        let host = with_alerts();
        host.0.borrow_mut().alert_result = Some(dialog_host::Ask::Failed);
        let mut harness = dialogged(host.clone());
        harness.run();
        start_export(&mut harness);
        press_action(&mut harness, ActionId::QUIT);
        harness.step();
        assert!(closing(&harness));
        // The protection is not off for good.
        host.0.borrow_mut().alert_result = None;
        harness.step();
        press_action(&mut harness, ActionId::QUIT);
        assert_eq!(asked_alerts(&host), 1);
        assert!(!closing(&harness));
    }

    #[test]
    fn ctrl_q_in_a_modal_settings_window_closes_it_then_quits() {
        // Settings as Windows shows it: a modal window, whose own window has no keymap.
        let host = FakeHost::on(OperatingSystem::Windows);
        let mut harness = dialogged(host.clone());
        harness.run();
        press_action(&mut harness, ActionId::SETTINGS);
        let shown = host.0.borrow().shown;
        assert_eq!(shown.map(|s| s.form), Some(dialog_host::Form::Modal));
        // Ctrl+Q, which the window left alone, reaches the app's keymap through the host.
        let ctrl = Modifiers {
            ctrl: true,
            command: true,
            ..Modifiers::NONE
        };
        let unread = dialog_host::Unread {
            events: vec![egui::Event::Key {
                key: Key::Q,
                physical_key: Some(Key::Q),
                pressed: true,
                repeat: false,
                modifiers: ctrl,
            }],
            focus: actions::Focus::None,
            os: OperatingSystem::Windows,
        };
        host.0.borrow_mut().outputs = vec![dialog_host::Output::Keys(unread.clone())];
        harness.step();
        assert!(
            host.0.borrow().closing,
            "Quit closes Settings as its Cancel"
        );
        assert!(!closing(&harness), "and leaves only once it has gone");
        host.gone(None);
        harness.step();
        assert!(closing(&harness));
        // A key no action takes there does nothing: with Settings modal, Ctrl+O rests.
        let host = FakeHost::on(OperatingSystem::Windows);
        let mut harness = dialogged(host.clone());
        harness.run();
        press_action(&mut harness, ActionId::SETTINGS);
        let open = dialog_host::Unread {
            events: vec![egui::Event::Key {
                key: Key::O,
                physical_key: Some(Key::O),
                pressed: true,
                repeat: false,
                modifiers: ctrl,
            }],
            ..unread
        };
        host.0.borrow_mut().outputs = vec![dialog_host::Output::Keys(open)];
        harness.step();
        assert!(!host.0.borrow().closing && !closing(&harness));
        assert!(
            !harness.state().shell.open,
            "no open dialog over a modal one"
        );
    }

    /// Settings' Restart now runs the `restart` action through the app's own quit path:
    /// Settings closed as its Cancel, the app leaving once it has gone, and only as it has
    /// ended the relaunch handed to `main`, with this run's flags and what is open, never
    /// started here. With a job running the quit alert asks; a no keeps the app, and the
    /// restart is asked no more, so a later Quit only quits.
    #[test]
    fn restart_now_quits_as_quit_does_and_then_hands_main_the_relaunch() {
        let host = FakeHost::on(OperatingSystem::Windows);
        let mut harness = dialogged(host.clone());
        harness.run();
        press_action(&mut harness, ActionId::SETTINGS);
        host.0.borrow_mut().outputs = vec![dialog_host::Output::Restart];
        harness.step();
        assert!(host.0.borrow().closing, "Settings closed as its Cancel");
        assert!(
            !closing(&harness),
            "and the app leaves only once it has gone"
        );
        host.gone(None);
        harness.step();
        assert!(closing(&harness));
        let taken = |harness: &Harness<'_, Dialogged>| {
            let relaunch = harness.state().relaunch.lock().expect("slot").take();
            relaunch.map(|relaunch| relaunch.arguments)
        };
        assert_eq!(taken(&harness), None, "nothing before the app has ended");
        // As `on_exit`, with a photo open alone, the graphics settings as they started.
        let folder = settings::testing::Folder::new("restart-photo");
        let photo = folder.0.join("IMG_1.CR3");
        std::fs::write(&photo, b"").expect("a raw");
        let open = restart::Open::Photo(photo.clone());
        harness.state_mut().restart.ended(open, false);
        let expected: Vec<OsString> = vec![
            "-d".into(),
            "gpu".into(),
            "--safe-mode".into(),
            photo.into(),
        ];
        assert_eq!(taken(&harness), Some(expected));

        let host = with_alerts();
        let mut harness = dialogged(host.clone());
        harness.run();
        start_export(&mut harness);
        press_action(&mut harness, ActionId::RESTART);
        assert_eq!(asked_alerts(&host), 1);
        answer_alert(&host, dialog_host::Answer::Pressed(1));
        harness.step();
        assert!(!closing(&harness));
        harness
            .state_mut()
            .restart
            .ended(restart::Open::Nothing, false);
        assert_eq!(taken(&harness), None);
    }

    #[test]
    fn quit_closes_a_dialog_first_and_leaves_once_it_has_gone() {
        let mut harness = dialogged(FakeHost::default());
        harness.run();
        // With no dialog, Quit leaves at once.
        press_action(&mut harness, ActionId::QUIT);
        assert!(closing(&harness));

        // A sheet stays until AppKit has animated it away, the host reporting it shown.
        let host = FakeHost::sheets();
        let mut harness = dialogged(host.clone());
        harness.run();
        press_action(&mut harness, ActionId::SETTINGS);
        assert!(host.0.borrow().shown.is_some_and(|s| s.modal()));
        press_action(&mut harness, ActionId::QUIT);
        assert!(
            host.0.borrow().closing,
            "Quit closes the dialog as its Cancel"
        );
        assert!(!closing(&harness), "and leaves only once it has gone");
        harness.step();
        assert!(!closing(&harness));
        host.gone(None);
        harness.step();
        assert!(closing(&harness));
    }

    #[test]
    fn a_dialog_that_closes_gives_the_focus_back_to_the_control_that_had_it() {
        let host = FakeHost::default();
        let mut harness = dialogged(host.clone());
        harness.run();
        let button = harness.state().button.expect("the button is drawn");
        assert!(!harness.ctx.memory(|m| m.has_focus(button)));
        // The host read the button's focus as the dialog was asked for.
        host.gone(Some(button));
        harness.run();
        assert!(harness.ctx.memory(|m| m.has_focus(button)));
    }

    #[test]
    fn quit_waits_for_the_open_dialog_to_answer() {
        let mut held = false;
        // Nothing asking: Quit runs, and holds nothing.
        assert!(hold_for_panel(ActionId::QUIT, false, &mut held));
        assert!(!held);
        // The open panel up: Quit is held for its answer, and every other action runs.
        assert!(!hold_for_panel(ActionId::QUIT, true, &mut held));
        assert!(held);
        assert!(hold_for_panel(ActionId::SETTINGS, true, &mut held));
    }

    #[test]
    fn the_open_dialog_a_sheet_and_a_focused_window_each_rest_their_actions() {
        let ctx = egui::Context::default();
        let shell = Shell::new(&Settings::default(), Locale::EnGb, OperatingSystem::Mac);
        let state = shell.state(&ctx, true, true);
        let rests = |state: State, id| !actions::action(id).expect("declared").enabled(&state);
        let window = |focused| dialog_host::Shown {
            form: dialog_host::Form::Modeless,
            focused,
        };
        let sheet = dialog_host::Shown {
            form: dialog_host::Form::Sheet,
            focused: true,
        };
        // Nothing open: all can run.
        let none = with_dialogs(state, false, None);
        assert_eq!(none, state);
        // The open dialog asking, and a sheet: modal, so nothing opens a second dialog,
        // Settings under the open panel included.
        for modal in [
            with_dialogs(state, true, None),
            with_dialogs(state, false, Some(sheet)),
        ] {
            assert!(modal.modal);
            for id in [ActionId::SETTINGS, ActionId::OPEN, ActionId::ABOUT] {
                assert!(rests(modal, id), "{id:?}");
            }
        }
        // A sheet takes the keys from the main window too; the open dialog takes its own.
        assert!(with_dialogs(state, false, Some(sheet)).dialog);
        assert!(!with_dialogs(state, true, None).dialog);
        // Settings' window, focused: the view's keys and the main window's items rest, and
        // Settings comes to the front; unfocused, the main window has everything back.
        let focused = with_dialogs(state, false, Some(window(true)));
        assert!(focused.dialog && focused.dialog_window && !focused.modal);
        assert!(rests(focused, ActionId::FULL_SCREEN));
        assert!(!rests(focused, ActionId::SETTINGS));
        assert_eq!(with_dialogs(state, false, Some(window(false))), state);
        // About is modal over the window, as the shell reports it, and so is Settings in its
        // modal window on Windows and Linux, as the host reports it.
        let mut shell = shell;
        shell.about.open(&ctx);
        assert!(shell.state(&ctx, true, true).modal);
        let modal = dialog_host::Shown {
            form: dialog_host::Form::Modal,
            focused: true,
        };
        let state = with_dialogs(state, false, Some(modal));
        assert!(state.modal && state.dialog && !state.dialog_window);
        for id in [ActionId::SETTINGS, ActionId::OPEN, ActionId::ABOUT] {
            assert!(rests(state, id), "{id:?}");
        }
    }

    #[test]
    fn a_press_a_window_move_was_handed_off_from_is_released_for_egui() {
        let ctx = egui::Context::default();
        let at = egui::pos2(300.0, 20.0);
        let press = egui::RawInput {
            events: vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
            ],
            ..Default::default()
        };
        run_frame(&ctx, press, |_| {});
        assert!(ctx.input(|i| i.pointer.primary_down()));
        let mut next = egui::RawInput::default();
        release_after_drag(&ctx, &mut next);
        let releases = |input: &egui::RawInput| {
            input
                .events
                .iter()
                .filter(|e| matches!(e, egui::Event::PointerButton { pressed: false, .. }))
                .count()
        };
        assert_eq!(releases(&next), 1);
        run_frame(&ctx, next, |_| {});
        assert!(!ctx.input(|i| i.pointer.primary_down()));
        // Once released, nothing more is given.
        let mut after = egui::RawInput::default();
        release_after_drag(&ctx, &mut after);
        assert_eq!(releases(&after), 0);
    }

    /// A key as egui-winit hands it over: `key` is the logical key, or the physical one
    /// where egui has no name for the logical key.
    fn press(harness: &Harness<'_, Bench>, modifiers: Modifiers, key: Key, physical: Key) {
        harness.event_modifiers(
            egui::Event::Key {
                key,
                physical_key: Some(physical),
                pressed: true,
                repeat: false,
                modifiers,
            },
            modifiers,
        );
    }

    #[test]
    fn ctrl_comma_opens_settings() {
        let mut harness = bench(OperatingSystem::Windows);
        harness.run();
        assert!(harness.query_by_label("Close").is_none());
        harness.key_press_modifiers(Modifiers::COMMAND, Key::Comma);
        harness.run();
        assert!(harness.state().settings_open());
        // In a modal window of the host's, with its Close button, as Windows and Linux
        // have it.
        let hosted = harness.state().host.0.borrow().shown;
        assert_eq!(hosted.map(|s| s.form), Some(dialog_host::Form::Modal));
        assert!(harness.state().host.0.borrow().settings[0].0.close_button);
        // While the dialog is open, Ctrl+, again changes nothing, and Ctrl+O opens no file
        // dialog over it: the window is modal.
        harness.key_press_modifiers(Modifiers::COMMAND, Key::Comma);
        harness.run();
        assert!(harness.state().settings_open());
        assert_eq!(harness.state().host.0.borrow().settings.len(), 1);
        harness.key_press_modifiers(Modifiers::COMMAND, Key::O);
        harness.run();
        assert!(harness.state().dialog.asked.is_empty());
    }

    #[test]
    fn a_double_click_on_the_mark_or_back_leaves_about_open() {
        use about::Page;
        let mut harness = bench(OperatingSystem::Windows);
        harness.run();
        // Two clicks 0.12 s apart, within egui's 0.3 s for a double-click, with frames
        // between them as a window repaints, in which the dialog lays itself out and
        // shows: the first click opens it, the second lands on it or on the scrim.
        let double_click = |harness: &mut Harness<'_, Bench>, at: egui::Pos2| {
            harness.run_steps(30);
            let click = vec![
                egui::Event::PointerMoved(at),
                pointer(at, true),
                pointer(at, false),
            ];
            let frames = [click.clone(), Vec::new(), Vec::new(), Vec::new(), click];
            for events in frames {
                let now = harness.ctx.input(|i| i.time);
                harness.input_mut().time = Some(now + 0.03);
                frame(harness, events);
            }
            // The harness keeps a time it is given: back to its own steps.
            harness.input_mut().time = None;
            harness.run();
        };
        let mark = harness
            .get_by_role_and_label(Role::Button, "About Apertrust")
            .rect()
            .center();
        double_click(&mut harness, mark);
        assert_eq!(harness.state().shell.about.page(), Some(Page::About));
        // Back's second click lands outside About, narrower than Credits.
        harness
            .get_by_role_and_label(Role::Button, "Credits")
            .click();
        harness.run();
        let back = harness
            .get_by_role_and_label(Role::Button, "Back")
            .rect()
            .center();
        double_click(&mut harness, back);
        assert_eq!(harness.state().shell.about.page(), Some(Page::About));
    }

    /// A click at `at`: the pointer there, pressed and let go in one frame.
    fn click_at(harness: &mut Harness<'_, Bench>, at: egui::Pos2) {
        harness.run_steps(30);
        frame(
            harness,
            [
                egui::Event::PointerMoved(at),
                pointer(at, true),
                pointer(at, false),
            ],
        );
        harness.run();
    }

    #[test]
    fn the_mark_opens_about_and_its_pages_go_one_step_in_and_back() {
        use about::Page;
        let mut harness = bench(OperatingSystem::Windows);
        harness.run();
        let page = |harness: &Harness<'_, Bench>| harness.state().shell.about.page();
        let open = |harness: &mut Harness<'_, Bench>| {
            harness
                .get_by_role_and_label(Role::Button, "About Apertrust")
                .click();
            harness.run();
        };
        let focused = |harness: &Harness<'_, Bench>, name: &str| {
            harness
                .get_by_role_and_label(Role::Button, name)
                .is_focused()
        };
        // The mark runs the action: About opens, named for screen readers, its version
        // the banner's, nothing focused, so no button is drawn highlighted.
        open(&mut harness);
        assert_eq!(page(&harness), Some(Page::About));
        harness.get_by_role_and_label(Role::Dialog, "About Apertrust");
        let version = apertrust_base::log::full_version(apertrust_base::program!());
        assert!(
            harness
                .query_by_label(&format!("Version {version}"))
                .is_some()
        );
        assert!(!focused(&harness, "Credits"));
        // Tab goes between its buttons alone: the dialog itself is no Tab stop.
        for button in ["Credits", "Licence", "Credits"] {
            harness.key_press(Key::Tab);
            harness.run();
            assert!(focused(&harness, button), "{button}");
        }
        // While it is open, the view's keys rest.
        assert!(
            harness
                .state()
                .shell
                .state(&harness.ctx, true, false)
                .dialog
        );
        harness.key_press(Key::Escape);
        harness.run();
        assert_eq!(page(&harness), None);
        // Closed, it gives the focus back to the mark that opened it.
        assert!(focused(&harness, "About Apertrust"));

        // A click outside it closes it, and so does a click on it off its buttons.
        open(&mut harness);
        click_at(&mut harness, egui::pos2(20.0, 580.0));
        assert_eq!(page(&harness), None);
        assert!(focused(&harness, "About Apertrust"));
        open(&mut harness);
        let logo = harness
            .get_by_role_and_label(Role::Image, "Apertrust")
            .rect();
        click_at(&mut harness, logo.center());
        assert_eq!(page(&harness), None);
        assert!(focused(&harness, "About Apertrust"));

        // Credits and Licence go one step in; a click there moves no focus, so no button is
        // drawn highlighted. A click inside stays, and Esc comes back, the button they were
        // opened from focused, since Esc is a key.
        for (button, title) in [("Credits", "Credits"), ("Licence", "Licence")] {
            open(&mut harness);
            harness.get_by_role_and_label(Role::Button, button).click();
            harness.run();
            let shown = if button == "Credits" {
                Page::Credits
            } else {
                Page::Licence
            };
            assert_eq!(page(&harness), Some(shown));
            assert!(!focused(&harness, "Back"));
            let heading = harness.get_by_label(title).rect();
            click_at(&mut harness, heading.center());
            assert_eq!(page(&harness), Some(shown));
            harness.key_press(Key::Escape);
            harness.run();
            assert_eq!(page(&harness), Some(Page::About));
            assert!(focused(&harness, button));
            harness.get_by_role_and_label(Role::Button, button).click();
            harness.run();
            harness.get_by_role_and_label(Role::Button, "Back").click();
            harness.run();
            assert_eq!(page(&harness), Some(Page::About));
            // From a page, a click outside closes the dialog.
            harness.get_by_role_and_label(Role::Button, button).click();
            harness.run();
            click_at(&mut harness, egui::pos2(20.0, 580.0));
            assert_eq!(page(&harness), None);
        }

        // Steps taken with the mouse move no focus, so no button is drawn highlighted:
        // into Credits and back again.
        open(&mut harness);
        let credits = harness
            .get_by_role_and_label(Role::Button, "Credits")
            .rect();
        click_at(&mut harness, credits.center());
        assert_eq!(page(&harness), Some(Page::Credits));
        assert!(!focused(&harness, "Back"));
        let back = harness.get_by_role_and_label(Role::Button, "Back").rect();
        click_at(&mut harness, back.center());
        assert_eq!(page(&harness), Some(Page::About));
        assert!(!focused(&harness, "Credits"));

        // By keyboard the focus follows: Tab to Credits, Enter goes in with Back focused,
        // Esc comes back with Credits focused.
        harness.key_press(Key::Tab);
        harness.run();
        assert!(focused(&harness, "Credits"));
        harness.key_press(Key::Enter);
        harness.run();
        harness.run();
        assert_eq!(page(&harness), Some(Page::Credits));
        assert!(focused(&harness, "Back"));
        harness.key_press(Key::Escape);
        harness.run();
        harness.run();
        assert_eq!(page(&harness), Some(Page::About));
        assert!(focused(&harness, "Credits"));
    }

    #[test]
    fn ctrl_1_to_5_choose_the_side_panel_s_tabs() {
        let mut harness = bench(OperatingSystem::Windows);
        harness.run();
        assert_eq!(harness.state().shell.panel_tab, BASE);
        let keys = [Key::Num2, Key::Num3, Key::Num4, Key::Num5];
        let names = ["Base", "Colour", "Correct", "Effect"];
        for (tab, (key, name)) in (1..).zip(keys.into_iter().zip(names)) {
            harness.key_press_modifiers(Modifiers::COMMAND, key);
            harness.run();
            assert_eq!(harness.state().shell.panel_tab, tab);
            let node = harness.get_by_role_and_label(Role::Tab, name);
            let selected = node.accesskit_node().toggled();
            assert_eq!(selected, Some(egui::accesskit::Toggled::True), "{name}");
        }
        // The Photos tab is the first: chosen by Ctrl+1, and not drawn disabled.
        let photos = harness.get_by_role_and_label(Role::Tab, "Photos");
        assert!(!photos.accesskit_node().is_disabled());
        harness.key_press_modifiers(Modifiers::COMMAND, Key::Num1);
        harness.run();
        assert_eq!(harness.state().shell.panel_tab, PHOTOS);
        // A click on a tab runs the same action.
        harness.get_by_role_and_label(Role::Tab, "Colour").click();
        harness.run();
        assert_eq!(harness.state().shell.panel_tab, 2);
        harness.get_by_role_and_label(Role::Tab, "Photos").click();
        harness.run();
        assert_eq!(harness.state().shell.panel_tab, PHOTOS);
    }

    #[test]
    fn holding_h_still_peeks_but_not_while_typing_nor_in_a_dialog() {
        let mut harness = bench(OperatingSystem::Windows);
        harness.run();
        let header = harness
            .get_by_role_and_label(Role::Button, "Exposure")
            .rect();
        harness.hover_at(header.center());
        harness.key_down(Key::H);
        harness.run();
        assert_eq!(harness.state().held, vec![ActionId::PEEK]);
        assert!(harness.state().peeking);
        harness.key_up(Key::H);
        harness.run();
        assert!(!harness.state().peeking);

        // With the header in focus, the pointer elsewhere.
        harness
            .get_by_role_and_label(Role::Button, "Exposure")
            .focus();
        harness.event(egui::Event::PointerGone);
        harness.key_down(Key::H);
        harness.run();
        assert!(harness.state().peeking);
        harness.key_up(Key::H);
        harness.run();

        // An H typed in a value field is typing: Enter on the slider opens its field.
        harness
            .get_by_role_and_label(Role::Slider, "Brightness")
            .focus();
        harness.run();
        harness.key_press(Key::Enter);
        harness.run();
        assert!(harness.query_by_label("Brightness: exact value").is_some());
        harness.hover_at(header.center());
        harness.key_down(Key::H);
        harness.run();
        assert!(harness.state().held.is_empty());
        assert!(!harness.state().peeking);
        harness.key_up(Key::H);
        harness.key_press(Key::Escape);
        harness.run();

        // Nor does the view's H reach behind a dialog.
        harness.state_mut().shell.settings = true;
        harness.run();
        harness.hover_at(header.center());
        harness.key_down(Key::H);
        harness.run();
        assert!(harness.state().held.is_empty());
        assert!(!harness.state().peeking);
    }

    #[test]
    fn a_tooltip_carries_its_key_in_the_platform_s_spelling() {
        for (os, keys) in [
            (OperatingSystem::Windows, "Ctrl+Shift+C"),
            (OperatingSystem::Nix, "Ctrl+Shift+C"),
            (OperatingSystem::Mac, "Shift+Cmd+C"),
        ] {
            let mut harness = bench(os);
            harness.run();
            let copy = harness.get_by_role_and_label(Role::Button, "Copy edit");
            assert_eq!(copy.accesskit_node().data().keyboard_shortcut(), Some(keys));
            copy.hover();
            harness.run();
            let tip = format!("Copy edit ({keys})");
            assert!(harness.query_by_label(&tip).is_some(), "{tip}");
        }
        let mut harness = bench(OperatingSystem::Mac);
        harness.run();
        let colour = harness.get_by_role_and_label(Role::Tab, "Colour");
        assert_eq!(
            colour.accesskit_node().data().keyboard_shortcut(),
            Some("Cmd+3")
        );
        colour.hover();
        harness.run();
        assert!(harness.query_by_label("Colour (Cmd+3)").is_some());
        // The Photos tab is the first.
        let photos = harness.get_by_role_and_label(Role::Tab, "Photos");
        assert_eq!(
            photos.accesskit_node().data().keyboard_shortcut(),
            Some("Cmd+1")
        );
        photos.hover();
        harness.run();
        assert!(harness.query_by_label("Photos (Cmd+1)").is_some());
        // Reset has no key by default, as darktable's discard history, so its tooltip is
        // its name alone.
        let reset = harness.get_by_role_and_label(Role::Button, "Reset");
        assert_eq!(reset.accesskit_node().data().keyboard_shortcut(), None);
    }

    /// This restates egui-winit's substitution rather than testing it: egui has no Й, so
    /// egui-winit sends Ctrl+Й on a Ukrainian keyboard with the key it sits on, Q, in the
    /// logical key's place, the same event as a Latin Ctrl+Q. The registry needs nothing
    /// more for Ctrl shortcuts, which type no text.
    #[test]
    fn ctrl_with_a_ukrainian_letter_arrives_as_the_latin_key_it_sits_on() {
        let mut harness = bench(OperatingSystem::Windows);
        harness.run();
        press(&harness, Modifiers::COMMAND, Key::Q, Key::Q);
        harness.run();
        assert_eq!(harness.state().pressed, vec![ActionId::QUIT]);
        // Ctrl+Б, on the comma's key.
        press(&harness, Modifiers::COMMAND, Key::Comma, Key::Comma);
        harness.run();
        assert!(harness.state().settings_open());
    }

    /// A plain L in one frame, as egui-winit sends it: the press, what it typed, the release.
    fn type_l(harness: &mut Harness<'_, Bench>, typed: &str) {
        let key = |pressed| egui::Event::Key {
            key: Key::L,
            physical_key: Some(Key::L),
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        let events = &mut harness.input_mut().events;
        events.extend([key(true), egui::Event::Text(typed.to_owned()), key(false)]);
        harness.run();
    }

    #[test]
    fn a_plain_letter_runs_unless_its_press_typed_another_key_s_character() {
        let mut harness = bench(OperatingSystem::Windows);
        // L chooses Browse's tab from another.
        harness.state_mut().shell.panel_tab = BASE;
        harness.run();
        // AltGr+L types ł on a Polish keyboard, and arrives as L.
        type_l(&mut harness, "ł");
        assert!(harness.state().pressed.is_empty());
        assert_eq!(harness.state().shell.panel_tab, BASE);
        // Д is the Ukrainian layout's own letter on the L key.
        type_l(&mut harness, "д");
        assert_eq!(harness.state().pressed, vec![ActionId::SHOW_BROWSE]);
        assert_eq!(harness.state().shell.panel_tab, BROWSE);
        harness.state_mut().shell.panel_tab = BASE;
        type_l(&mut harness, "l");
        assert_eq!(harness.state().pressed, vec![ActionId::SHOW_BROWSE; 2]);
    }

    #[test]
    fn view_browse_chooses_the_first_tab_holding_browse_and_pressed_again_nothing() {
        let mut harness = bench(OperatingSystem::Windows);
        harness.state_mut().shell.panel_tab = 3;
        harness.run();
        harness.key_press(Key::L);
        harness.run();
        assert_eq!(harness.state().shell.panel_tab, BROWSE);
        assert_eq!(BROWSE, PHOTOS);
        // The tab the settings had is forgotten, as a click on a tab forgets it.
        assert_eq!(harness.state().shell.opening_tab, None);
        harness.key_press(Key::L);
        harness.run();
        assert_eq!(harness.state().shell.panel_tab, BROWSE);
        assert_eq!(harness.state().pressed, vec![ActionId::SHOW_BROWSE; 2]);
        // From the ☰'s menu too, View › Browse, with its key.
        harness.state_mut().shell.panel_tab = 3;
        alt(&mut harness, Key::V, Some("v"));
        let item = harness.get_by_role_and_label(Role::MenuItem, "Browse");
        assert_eq!(item.accesskit_node().data().keyboard_shortcut(), Some("L"));
        item.click();
        harness.run();
        assert_eq!(harness.state().shell.panel_tab, BROWSE);
    }

    #[test]
    fn a_digit_under_a_symbol_still_chooses_its_tab_but_a_letter_under_one_does_not_quit() {
        let mut harness = bench(OperatingSystem::Windows);
        harness.run();
        // French AZERTY types ' on the 4 key.
        press(&harness, Modifiers::COMMAND, Key::Quote, Key::Num4);
        harness.run();
        assert_eq!(harness.state().shell.panel_tab, 3);
        // Dvorak types ' where QWERTY has Q: Ctrl+' is not Ctrl+Q.
        press(&harness, Modifiers::COMMAND, Key::Quote, Key::Q);
        harness.run();
        assert_eq!(harness.state().pressed, vec![ActionId::PANEL_TABS[3]]);
        // Czech types + on the 1 key: the digit comes before Ctrl++, so the press is the
        // Photos tab's, and never zooms.
        press(&harness, Modifiers::COMMAND, Key::Plus, Key::Num1);
        harness.run();
        assert_eq!(harness.state().shell.panel_tab, PHOTOS);
        assert_eq!(
            harness.state().pressed.last(),
            Some(&ActionId::PANEL_TABS[0])
        );
        assert_eq!(harness.ctx.zoom_factor(), 1.0);
    }

    #[test]
    fn a_disabled_action_s_key_does_nothing() {
        let mut harness = bench(OperatingSystem::Windows);
        harness.run();
        for (modifiers, key) in [
            // Nothing to undo or redo yet; not built yet: Copy edit, the next photo.
            (Modifiers::COMMAND, Key::Z),
            (Modifiers::COMMAND, Key::Y),
            (Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::C),
            (Modifiers::NONE, Key::Space),
            // Can't run now: Edit is showing, the scale is already 100%.
            (Modifiers::NONE, Key::E),
            (Modifiers::COMMAND, Key::Num0),
        ] {
            harness.key_press_modifiers(modifiers, key);
            harness.run();
        }
        assert!(harness.state().pressed.is_empty());
        assert!(harness.state().commands.is_empty());
        assert_eq!(harness.ctx.zoom_factor(), 1.0);
        // And their buttons are drawn disabled.
        for name in ["Undo", "Reset"] {
            let node = harness.get_by_role_and_label(Role::Button, name);
            assert!(node.accesskit_node().is_disabled(), "{name}");
        }
    }

    #[test]
    fn reset_runs_from_its_button_once_there_is_an_edit() {
        let mut harness = bench(OperatingSystem::Windows);
        harness.run();
        harness.get_by_role_and_label(Role::Button, "Reset").click();
        harness.run();
        assert!(harness.state().commands.is_empty());
        harness.state_mut().edited = true;
        harness.run();
        harness.get_by_role_and_label(Role::Button, "Reset").click();
        harness.run();
        assert_eq!(harness.state().commands, vec![Command::Reset]);
    }

    /// A history of one step, Exposure, with `undo` and `redo` as given.
    fn one_step(
        undo: Option<apertrust_engine::session::Move>,
        redo: Option<apertrust_engine::session::Move>,
    ) -> apertrust_engine::session::History {
        let step = history::tests::step(7, &["exposure"], None);
        history::tests::history(3, vec![step], undo, redo)
    }

    #[test]
    fn undo_and_redo_send_their_commands_from_their_keys_once_there_is_a_step() {
        use apertrust_engine::session::{Move, StepId};
        for (os, command) in [
            (
                OperatingSystem::Windows,
                Modifiers::CTRL.plus(Modifiers::COMMAND),
            ),
            (
                OperatingSystem::Mac,
                Modifiers::MAC_CMD.plus(Modifiers::COMMAND),
            ),
        ] {
            let shift = command.plus(Modifiers::SHIFT);
            let mut harness = bench(os);
            harness.run();
            harness.key_press_modifiers(command, Key::Z);
            harness.run();
            assert!(harness.state().commands.is_empty(), "{os:?}");
            // A step to take back: Ctrl+Z (Cmd+Z), and its button, undo it.
            harness.state_mut().history = Some(one_step(
                Some(Move::Steps {
                    first: StepId(7),
                    count: 1,
                }),
                None,
            ));
            harness.run();
            harness.key_press_modifiers(command, Key::Z);
            harness.run();
            harness.get_by_role_and_label(Role::Button, "Undo").click();
            harness.run();
            harness.key_press_modifiers(shift, Key::Z);
            harness.run();
            assert_eq!(harness.state().commands, [Command::Undo, Command::Undo]);
            // Then one to put back: Shift+Ctrl+Z, and Ctrl+Y off the Mac.
            harness.state_mut().history = Some(one_step(
                None,
                Some(Move::Steps {
                    first: StepId(7),
                    count: 1,
                }),
            ));
            harness.state_mut().commands.clear();
            harness.run();
            harness.key_press_modifiers(command, Key::Z);
            harness.run();
            harness.key_press_modifiers(shift, Key::Z);
            harness.run();
            harness.key_press_modifiers(command, Key::Y);
            harness.run();
            let redone = if os == OperatingSystem::Mac { 1 } else { 2 };
            assert_eq!(harness.state().commands, vec![Command::Redo; redone]);
            // Held, each repeat is one more.
            harness.state_mut().commands.clear();
            harness.event_modifiers(
                egui::Event::Key {
                    key: Key::Z,
                    physical_key: Some(Key::Z),
                    pressed: true,
                    repeat: true,
                    modifiers: shift,
                },
                shift,
            );
            harness.run();
            assert_eq!(harness.state().commands, [Command::Redo]);
            // A photo with nothing to take back or put back: nothing.
            harness.state_mut().history = Some(one_step(None, None));
            harness.state_mut().commands.clear();
            harness.run();
            harness.key_press_modifiers(command, Key::Z);
            harness.key_press_modifiers(shift, Key::Z);
            harness.run();
            assert!(harness.state().commands.is_empty());
            for name in ["Undo", "Redo"] {
                let node = harness.get_by_role_and_label(Role::Button, name);
                assert!(node.accesskit_node().is_disabled(), "{name}");
            }
        }
    }

    #[test]
    fn edit_s_undo_and_redo_name_their_steps_and_send_their_commands() {
        use apertrust_engine::session::{Move, StepId};
        // Redo shows Ctrl+Y first on Windows, Ctrl+Shift+Z first on Linux.
        for (os, redo_key) in [
            (OperatingSystem::Windows, "Ctrl+Y"),
            (OperatingSystem::Nix, "Ctrl+Shift+Z"),
        ] {
            let mut harness = bench(os);
            let history = one_step(
                Some(Move::Steps {
                    first: StepId(7),
                    count: 1,
                }),
                Some(Move::GoTo),
            );
            harness.state_mut().history = Some(history);
            harness.run();
            alt(&mut harness, Key::E, Some("e"));
            let undo = harness.get_by_role_and_label(Role::MenuItem, "Undo Exposure");
            assert!(!undo.accesskit_node().is_disabled(), "{os:?}");
            assert_eq!(
                undo.accesskit_node().data().keyboard_shortcut(),
                Some("Ctrl+Z")
            );
            let redo = harness.get_by_role_and_label(Role::MenuItem, "Redo Go to Step");
            assert_eq!(
                redo.accesskit_node().data().keyboard_shortcut(),
                Some(redo_key)
            );
            undo.click();
            harness.run();
            assert_eq!(harness.state().commands, [Command::Undo]);
            alt(&mut harness, Key::E, Some("e"));
            harness
                .get_by_role_and_label(Role::MenuItem, "Redo Go to Step")
                .click();
            harness.run();
            assert_eq!(harness.state().commands, [Command::Undo, Command::Redo]);
            // Nothing to take back: Undo alone, disabled.
            harness.state_mut().history = Some(one_step(None, None));
            harness.run();
            alt(&mut harness, Key::E, Some("e"));
            let undo = harness.get_by_role_and_label(Role::MenuItem, "Undo");
            assert!(undo.accesskit_node().is_disabled());
        }
    }

    /// The shell beside a text field, as a value field or a dialog's name field has one:
    /// the keys read first, the top panel and the field, then what the keys and the menus
    /// ran, and the ☰'s menu opened where it is drawn.
    struct Field {
        shell: Shell,
        text: String,
        /// Whether the field is to take the focus this frame.
        focus: bool,
        /// What the menus chose, as the Mac's come before the frame.
        chosen: Vec<ActionId>,
        commands: Vec<Command>,
    }

    fn field(os: OperatingSystem) -> Harness<'static, Field> {
        Harness::builder()
            .with_os(os)
            .with_context_setup(|ctx| {
                apertrust_ui::install(ctx, Theme::default());
                registry_keys_only(ctx);
            })
            .build_ui_state(
                |ui, field: &mut Field| {
                    let ctx = ui.ctx().clone();
                    // A step on each side, so Undo and Redo can run.
                    let state = |ctx: &egui::Context, shell: &Shell| State {
                        undo: true,
                        redo: true,
                        ..shell.state(ctx, true, true)
                    };
                    let now = state(&ctx, &field.shell);
                    let keys = actions::read(&ctx, &field.shell.keymap, &now);
                    field.shell.close_menu(&keys.pressed);
                    let top = field
                        .shell
                        .top_panel(ui, &now, SaveState::None, None, None, &[]);
                    let name = ui.label("Name");
                    let response = ui.text_edit_singleline(&mut field.text);
                    let response = response.labelled_by(name.id);
                    if std::mem::take(&mut field.focus) {
                        response.request_focus();
                    }
                    let mut run = keys.pressed;
                    run.append(&mut field.chosen);
                    if let Some(FromTopPanel::Action(id)) = top {
                        run.push(id);
                    }
                    run.extend(field.shell.popups(&ctx));
                    for id in run {
                        let state = state(&ctx, &field.shell);
                        let command = field.shell.run(id, &state, &ctx, &[]);
                        field.commands.extend(command);
                    }
                    let now = state(&ctx, &field.shell);
                    field.shell.open_menu(&ctx, &now);
                },
                Field {
                    shell: Shell::new(&Settings::default(), Locale::EnGb, os),
                    text: "photo".to_owned(),
                    focus: true,
                    chosen: Vec::new(),
                    commands: Vec::new(),
                },
            )
    }

    /// Runs `id` as a menu's choice, then a frame with what it handed the field, as the
    /// app's `raw_input_hook` adds it.
    fn choose(harness: &mut Harness<'_, Field>, id: ActionId) {
        harness.state_mut().chosen.push(id);
        harness.run();
        let keys = std::mem::take(&mut harness.state_mut().shell.field_keys);
        harness.input_mut().events.extend(keys);
        harness.run();
    }

    #[test]
    fn a_text_field_keeps_its_own_undo_and_the_menus_hand_it_theirs() {
        for (os, command) in [
            (
                OperatingSystem::Mac,
                Modifiers::MAC_CMD.plus(Modifiers::COMMAND),
            ),
            (
                OperatingSystem::Windows,
                Modifiers::CTRL.plus(Modifiers::COMMAND),
            ),
        ] {
            let mut harness = field(os);
            harness.run();
            harness.run();
            harness
                .input_mut()
                .events
                .push(egui::Event::Text("x".to_owned()));
            harness.run();
            assert_eq!(harness.state().text, "photox", "{os:?}");
            // The keys are the field's: its text goes back, and the photo's history is
            // never sent a command.
            harness.key_press_modifiers(command, Key::Z);
            harness.run();
            assert_eq!(harness.state().text, "photo");
            harness.key_press_modifiers(command.plus(Modifiers::SHIFT), Key::Z);
            harness.run();
            assert_eq!(harness.state().text, "photox");
            // Edit › Undo and Redo chosen from a menu act on the field too.
            choose(&mut harness, ActionId::UNDO);
            assert_eq!(harness.state().text, "photo");
            choose(&mut harness, ActionId::REDO);
            assert_eq!(harness.state().text, "photox");
            assert!(harness.state().commands.is_empty());
            // Without the focus, they are the photo's.
            harness.ctx.memory_mut(|m| m.stop_text_input());
            harness.run();
            choose(&mut harness, ActionId::UNDO);
            assert_eq!(harness.state().text, "photox");
            assert_eq!(harness.state().commands, [Command::Undo]);
        }
    }

    /// The ☰'s menu opened by F10 gives the focus back to the field before Edit › Undo
    /// runs, which hands the field its own key.
    #[test]
    fn edit_s_undo_from_the_menu_f10_opened_acts_on_the_focused_field() {
        let mut harness = field(OperatingSystem::Nix);
        harness.run();
        harness.run();
        harness
            .input_mut()
            .events
            .push(egui::Event::Text("x".to_owned()));
        harness.run();
        assert_eq!(harness.state().text, "photox");
        harness.key_press(Key::F10);
        harness.run();
        harness.key_press(Key::ArrowDown);
        harness.key_press(Key::ArrowRight);
        harness.run();
        // The field's own undo: Undo alone, enabled, with its key.
        let undo = harness.get_by_role_and_label(Role::MenuItem, "Undo");
        assert!(!undo.accesskit_node().is_disabled());
        assert_eq!(
            undo.accesskit_node().data().keyboard_shortcut(),
            Some("Ctrl+Z")
        );
        undo.click();
        harness.run();
        let keys = std::mem::take(&mut harness.state_mut().shell.field_keys);
        assert_eq!(keys.len(), 2);
        harness.input_mut().events.extend(keys);
        harness.run();
        assert_eq!(harness.state().text, "photo");
        assert!(harness.state().commands.is_empty());
    }

    #[test]
    fn open_opens_the_first_raw_chosen_and_cancelling_opens_nothing() {
        for (os, command) in [
            (
                OperatingSystem::Windows,
                Modifiers::CTRL.plus(Modifiers::COMMAND),
            ),
            (
                OperatingSystem::Mac,
                Modifiers::MAC_CMD.plus(Modifiers::COMMAND),
            ),
        ] {
            let mut harness = bench(os);
            harness.state_mut().photo = false;
            harness.run();
            // Cancelled: the dialog asked, and nothing opens.
            harness.state_mut().dialog.answer = Some(None);
            harness.key_press_modifiers(command, Key::O);
            harness.run();
            let bench = harness.state();
            assert_eq!(bench.pressed, vec![ActionId::OPEN], "{os:?}");
            assert_eq!(bench.dialog.asked, vec![Some(PathBuf::from("Pictures"))]);
            assert!(bench.commands.is_empty(), "{os:?}");

            // While a dialog asks, Open… shows no second one.
            harness.key_press_modifiers(command, Key::O);
            harness.run();
            harness.key_press_modifiers(command, Key::O);
            harness.run();
            assert_eq!(harness.state().dialog.asked.len(), 2, "{os:?}");
            assert!(harness.state().commands.is_empty(), "{os:?}");
            assert_eq!(harness.state().dialog.picks[1], dialogs::Pick::Files);

            // The raw chosen opens through the session, a frame after the dialog asked; the
            // app opens its folder behind it (`App::open`).
            let raw = PathBuf::from("raws/IMG_2041.CR3");
            harness.state_mut().dialog.answer = Some(Some(vec![raw.clone()]));
            harness.run();
            assert_eq!(harness.state().commands, vec![Command::Open(raw)], "{os:?}");

            // Of several from one folder, the first opens; the others wait for the grid
            // panel's selection.
            harness.state_mut().commands.clear();
            harness.key_press_modifiers(command, Key::O);
            harness.run();
            let raws = ["raws/IMG_2043.CR3", "raws/IMG_2042.CR3"].map(PathBuf::from);
            harness.state_mut().dialog.answer = Some(Some(raws.to_vec()));
            harness.run();
            let first = Command::Open(raws[0].clone());
            assert_eq!(harness.state().commands, vec![first], "{os:?}");
        }
    }

    /// `paths` dropped onto the bench's window, as winit drops them, then the frame that
    /// takes them and, once they have been looked at, the frames that open what they open.
    /// One frame takes them, never a run, whose frames would take the answer too as soon
    /// as the drop's thread gave it, opening what it opens within the same run.
    fn drop_onto(harness: &mut Harness<'_, Bench>, paths: &[&Path]) {
        for path in paths {
            let file = dropped::tests::File(path.to_path_buf());
            harness.input_mut().dropped_files.push(Arc::new(file));
        }
        harness.step();
        harness.state_mut().drops.wait();
        harness.run();
    }

    #[test]
    fn a_folder_or_raws_dropped_open_as_open_folder_and_open_do() {
        let folder = settings::testing::Folder::new("drop-onto");
        let (wedding, portraits) = (folder.0.join("Wedding"), folder.0.join("Portraits"));
        for made in [&wedding, &portraits] {
            std::fs::create_dir(made).expect("a folder");
        }
        let raws = [
            wedding.join("B.CR3"),
            portraits.join("C.CR3"),
            wedding.join("A.CR3"),
        ];
        for raw in &raws {
            std::fs::write(raw, b"").expect("a raw");
        }
        let mut harness = bench(OperatingSystem::Windows);
        harness.run();
        let open_folder =
            |path: &Path| Command::Library(LibraryCommand::Open(Source::Folder(path.to_owned())));
        // A folder; of two, the first.
        drop_onto(&mut harness, &[&wedding]);
        drop_onto(&mut harness, &[&portraits, &wedding]);
        // Raws: the first, from its folder, the others from it to be selected with it.
        drop_onto(&mut harness, &[&raws[0], &raws[1], &raws[2]]);
        assert_eq!(
            harness.state().commands,
            vec![
                open_folder(&wedding),
                open_folder(&portraits),
                Command::Open(raws[0].clone())
            ]
        );
        assert!(harness.state().refused.is_empty());
    }

    #[test]
    fn a_drop_that_opens_nothing_says_why_and_none_is_taken_under_a_modal_dialog() {
        let folder = settings::testing::Folder::new("drop-refused");
        let (notes, library) = (folder.0.join("notes.txt"), folder.0.join("Home.aplibrary"));
        let raw = folder.0.join("IMG_1.CR3");
        for file in [&notes, &library, &raw] {
            std::fs::write(file, b"").expect("a file");
        }
        let mut harness = bench(OperatingSystem::Windows);
        harness.run();
        drop_onto(&mut harness, &[&notes]);
        drop_onto(&mut harness, &[&notes, &library]);
        // The alert names the library, since it says libraries come later.
        assert_eq!(
            harness.state().refused,
            vec![
                (notes.clone(), dropped::Refusal::Unknown),
                (library.clone(), dropped::Refusal::Library)
            ]
        );
        // A file that opens nothing beside a raw: the raw opens.
        drop_onto(&mut harness, &[&notes, &raw]);
        assert_eq!(harness.state().commands, vec![Command::Open(raw.clone())]);

        // Under Settings, a modal window on Windows, nothing is taken.
        harness.key_press_modifiers(Modifiers::COMMAND, Key::Comma);
        harness.run();
        assert!(harness.state().state(&harness.ctx).modal);
        dropped_untaken(&mut harness, &raw);
        assert_eq!(harness.state().refused.len(), 2);
    }

    /// `path` dropped onto the bench's window under a modal dialog: nothing is taken, and
    /// nothing opens.
    fn dropped_untaken(harness: &mut Harness<'_, Bench>, path: &Path) {
        let before = harness.state().commands.len();
        let file = dropped::tests::File(path.to_path_buf());
        harness.input_mut().dropped_files.push(Arc::new(file));
        harness.run();
        assert!(!harness.state().drops.looking());
        assert_eq!(harness.state().commands.len(), before);
    }

    #[test]
    fn no_drop_is_taken_under_about_the_open_dialog_or_an_alert() {
        let folder = settings::testing::Folder::new("drop-modal");
        let (raw, notes) = (folder.0.join("IMG_1.CR3"), folder.0.join("notes.txt"));
        for file in [&raw, &notes] {
            std::fs::write(file, b"").expect("a file");
        }
        let mut harness = bench(OperatingSystem::Mac);
        harness.run();
        // About, over the window.
        let ctx = harness.ctx.clone();
        harness.state_mut().shell.about.open(&ctx);
        harness.run();
        dropped_untaken(&mut harness, &raw);
        harness.state_mut().shell.about = about::About::default();
        harness.run();
        // The open dialog asking, a sheet on the Mac.
        harness.key_press_modifiers(Modifiers::COMMAND, Key::O);
        harness.run();
        assert!(harness.state().dialog.asking);
        dropped_untaken(&mut harness, &raw);
        harness.state_mut().dialog.answer = Some(None);
        harness.run();
        // An alert, the refusal's own: the next drop waits for its answer.
        harness.state().host.0.borrow_mut().native_alerts = true;
        drop_onto(&mut harness, &[&notes]);
        assert_eq!(harness.state().asked, vec![dialog_host::Ask::Shown]);
        dropped_untaken(&mut harness, &raw);
        assert!(harness.state().commands.is_empty());
    }

    #[test]
    fn a_drop_looked_at_while_a_modal_dialog_opens_opens_nothing() {
        let folder = settings::testing::Folder::new("drop-meanwhile");
        let raw = folder.0.join("IMG_1.CR3");
        std::fs::write(&raw, b"").expect("a raw");
        let mut harness = bench(OperatingSystem::Windows);
        harness.run();
        let file = dropped::tests::File(raw.clone());
        harness.input_mut().dropped_files.push(Arc::new(file));
        // One frame, so none takes the answer before About opens.
        harness.step();
        assert!(harness.state().drops.looking());
        // About opens, modal over the window, before the frame the answer is looked for.
        let ctx = harness.ctx.clone();
        harness.state_mut().shell.about.open(&ctx);
        harness.state_mut().drops.wait();
        harness.step();
        assert!(!harness.state().drops.looking());
        assert!(harness.state().commands.is_empty());
        // Nor once About has gone: the answer was taken and set aside.
        harness.state_mut().shell.about = about::About::default();
        harness.run();
        assert!(harness.state().commands.is_empty());
    }

    #[test]
    fn open_refuses_a_library_s_file_or_another_typed_into_its_dialog() {
        let mut harness = bench(OperatingSystem::Windows);
        harness.run();
        for (typed, why) in [
            ("/photos/Home.aplibrary", dropped::Refusal::Library),
            ("/photos/notes.txt", dropped::Refusal::Unknown),
        ] {
            harness.key_press_modifiers(Modifiers::COMMAND, Key::O);
            harness.run();
            let typed = PathBuf::from(typed);
            harness.state_mut().dialog.answer = Some(Some(vec![typed.clone()]));
            harness.run();
            assert_eq!(harness.state().refused.last(), Some(&(typed, why)));
        }
        assert!(harness.state().commands.is_empty());
    }

    #[test]
    fn the_refused_alert_goes_through_alerts_and_another_that_frame_is_busy() {
        let host = FakeHost::default();
        host.0.borrow_mut().native_alerts = true;
        let mut dialogs = dialog_host::AppDialogs::new(host.clone());
        let mut alerts = alerts::Alerts::default();
        let ctx = egui::Context::default();
        apertrust_ui::install(&ctx, Theme::default());
        let refused = alerts::refused_request(&ctx, "notes.txt", dropped::Refusal::Unknown);
        let purpose = alerts::Purpose::Refused;
        let asked = alerts.ask(&mut dialogs, purpose, &refused, &ctx);
        assert_eq!(asked, dialog_host::Ask::Shown);
        // The save warning's, asked in the same frame, waits: one alert at a time.
        let save = alerts::save_request(&ctx, "IMG_1.CR3", "disk full");
        let later = alerts.ask(&mut dialogs, alerts::Purpose::Save, &save, &ctx);
        assert_eq!(later, dialog_host::Ask::Busy);
        assert_eq!(host.0.borrow().alerts, vec![refused]);
        // OK pressed: the refusal's answer, which does nothing.
        host.0.borrow_mut().answer = Some(dialog_host::Answer::Pressed(0));
        let answered = alerts.answered(&mut dialogs);
        assert_eq!(answered, Some((purpose, dialog_host::Answer::Pressed(0))));
        assert_eq!(alerts.pending(), None);
    }

    #[test]
    fn ctrl_plus_and_minus_scale_the_interface_and_ctrl_0_resets_it() {
        let mut harness = bench(OperatingSystem::Windows);
        harness.run();
        harness.key_press_modifiers(Modifiers::COMMAND, Key::Equals);
        harness.run();
        assert_eq!(harness.ctx.zoom_factor(), 1.1);
        // Ctrl++ is Ctrl+Shift+= on a US keyboard.
        harness.key_press_modifiers(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::Plus);
        harness.run();
        assert_eq!(harness.ctx.zoom_factor(), 1.2);
        harness.key_press_modifiers(Modifiers::COMMAND, Key::Minus);
        harness.run();
        assert_eq!(harness.ctx.zoom_factor(), 1.1);
        harness.key_press_modifiers(Modifiers::COMMAND, Key::Num0);
        harness.run();
        assert_eq!(harness.ctx.zoom_factor(), 1.0);
    }

    #[test]
    fn f6_moves_the_focus_from_region_to_region() {
        let mut harness = bench(OperatingSystem::Windows);
        harness.run();
        let region_of_focus = |harness: &Harness<'_, Bench>| {
            let focused = harness.query_by(|node| node.is_focused())?;
            let centre = focused.rect().center();
            harness
                .state()
                .regions
                .iter()
                .position(|r| r.contains(centre))
        };
        // Into the top panel, on its first control: the ☰.
        harness.key_press(Key::F6);
        harness.run();
        assert_eq!(region_of_focus(&harness), Some(0));
        assert!(
            harness
                .get_by_role_and_label(Role::Button, "Menu")
                .is_focused()
        );
        harness.key_press(Key::F6);
        harness.run();
        assert_eq!(region_of_focus(&harness), Some(1));
        harness.key_press(Key::F6);
        harness.run();
        assert_eq!(region_of_focus(&harness), Some(0));
        harness.key_press_modifiers(Modifiers::SHIFT, Key::F6);
        harness.run();
        assert_eq!(region_of_focus(&harness), Some(1));
    }

    /// Presses and releases a key in a frame of its own, and says whether that frame asked
    /// to close the window.
    fn closes(harness: &mut Harness<'_, Bench>, modifiers: Modifiers, key: Key) -> bool {
        for pressed in [true, false] {
            harness.input_mut().events.push(egui::Event::Key {
                key,
                physical_key: Some(key),
                pressed,
                repeat: false,
                modifiers,
            });
        }
        harness.step();
        harness
            .output()
            .viewport_output
            .values()
            .any(|v| v.commands.contains(&egui::ViewportCommand::Close))
    }

    #[test]
    fn only_the_registry_s_quit_key_closes_the_window() {
        let mut harness = bench(OperatingSystem::Windows);
        harness.run();
        // egui's own Ctrl+Q would take these too.
        let ctrl = Modifiers::CTRL.plus(Modifiers::COMMAND);
        assert!(!closes(&mut harness, ctrl.plus(Modifiers::SHIFT), Key::Q));
        assert!(!closes(&mut harness, ctrl.plus(Modifiers::ALT), Key::Q));
        assert!(closes(&mut harness, ctrl, Key::Q));
        let mut harness = bench(OperatingSystem::Mac);
        harness.run();
        let cmd = Modifiers::MAC_CMD.plus(Modifiers::COMMAND);
        assert!(!closes(&mut harness, cmd.plus(Modifiers::ALT), Key::Q));
        assert!(closes(&mut harness, cmd, Key::Q));
    }

    #[test]
    fn full_screen_and_quit_have_their_platform_s_keys() {
        let mut harness = bench(OperatingSystem::Mac);
        harness.run();
        let control_command = Modifiers::CTRL
            .plus(Modifiers::MAC_CMD)
            .plus(Modifiers::COMMAND);
        harness.key_press_modifiers(control_command, Key::F);
        harness.key_press_modifiers(Modifiers::MAC_CMD.plus(Modifiers::COMMAND), Key::Q);
        harness.run();
        assert_eq!(
            harness.state().pressed,
            vec![ActionId::FULL_SCREEN, ActionId::QUIT]
        );
        let mut harness = bench(OperatingSystem::Windows);
        harness.run();
        harness.key_press(Key::F11);
        harness.run();
        assert_eq!(harness.state().pressed, vec![ActionId::FULL_SCREEN]);
    }

    #[test]
    fn the_icon_decodes_as_a_square() {
        let icon = icon();
        assert!(icon.width >= 256 && icon.width == icon.height);
        assert_eq!(icon.rgba.len(), 4 * (icon.width * icon.height) as usize);
    }

    /// The menu named `name` is open.
    fn menu_open(harness: &Harness<'_, Bench>, name: &str) -> bool {
        harness.query_by_role_and_label(Role::Menu, name).is_some()
    }

    /// The name of what has the focus.
    fn focused_name(harness: &Harness<'_, Bench>) -> String {
        harness
            .get_by(|node| node.is_focused())
            .accesskit_node()
            .label()
            .unwrap_or_default()
    }

    /// The line `name` of an open menu, as screen readers read it: whether it is enabled,
    /// its keys and whether it is checked.
    fn menu_line(
        harness: &Harness<'_, Bench>,
        role: Role,
        name: &str,
    ) -> (bool, Option<String>, Option<Toggled>) {
        let node = harness.get_by_role_and_label(role, name).accesskit_node();
        let keys = node.data().keyboard_shortcut().map(str::to_owned);
        (!node.is_disabled(), keys, node.toggled())
    }

    /// Alt with the letter key `key` in one frame, as egui-winit sends it: the press, what
    /// it typed, if anything, and the release.
    fn alt(harness: &mut Harness<'_, Bench>, key: Key, typed: Option<&str>) {
        alt_with(harness, Modifiers::ALT, key, typed);
    }

    fn alt_with(
        harness: &mut Harness<'_, Bench>,
        modifiers: Modifiers,
        key: Key,
        typed: Option<&str>,
    ) {
        let event = |pressed| egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed,
            repeat: false,
            modifiers,
        };
        let events = &mut harness.input_mut().events;
        events.push(event(true));
        events.extend(typed.map(|t| egui::Event::Text(t.to_owned())));
        events.push(event(false));
        harness.run();
    }

    #[test]
    fn the_menu_button_comes_first_and_opens_the_menus() {
        for os in [OperatingSystem::Windows, OperatingSystem::Nix] {
            let mut harness = bench(os);
            harness.run();
            let button = harness.get_by_role_and_label(Role::Button, "Menu");
            let mark = harness.get_by_role_and_label(Role::Button, "About Apertrust");
            assert!(button.rect().right() < mark.rect().left(), "{os:?}");
            // Its tooltip and screen readers give F10 with it.
            assert_eq!(
                button.accesskit_node().data().keyboard_shortcut(),
                Some("F10"),
                "{os:?}"
            );
            button.click();
            harness.run();
            assert!(menu_open(&harness, "Menu"), "{os:?}");
            assert_eq!(focused_name(&harness), "File", "{os:?}");
            // The menus `MENUS` has off the Mac, at its top.
            for title in ["File", "Edit", "Photo", "View", "Help"] {
                assert!(
                    harness
                        .query_by_role_and_label(Role::MenuItem, title)
                        .is_some(),
                    "{os:?}: {title}"
                );
            }
            assert!(harness.query_by_label("Window").is_none(), "{os:?}");
            // A press of the ☰ closes it.
            harness.get_by_role_and_label(Role::Button, "Menu").click();
            harness.run();
            assert!(!menu_open(&harness, "Menu"), "{os:?}");
        }
        // The Mac keeps its native menu bar.
        let mut harness = bench(OperatingSystem::Mac);
        harness.run();
        assert!(
            harness
                .query_by_role_and_label(Role::Button, "Menu")
                .is_none()
        );
        harness.key_press(Key::F10);
        harness.run();
        alt(&mut harness, Key::F, Some("f"));
        assert!(!menu_open(&harness, "Menu"));
    }

    #[test]
    fn f10_opens_the_menu_at_its_top_and_closes_it() {
        let mut harness = bench(OperatingSystem::Nix);
        harness.run();
        harness.key_press(Key::F10);
        harness.run();
        assert!(menu_open(&harness, "Menu"));
        assert_eq!(focused_name(&harness), "File");
        assert!(!menu_open(&harness, "File"));
        harness.key_press(Key::F10);
        harness.run();
        assert!(!menu_open(&harness, "Menu"));
        // Not over a dialog, where the view's keys rest too.
        harness.state_mut().shell.settings = true;
        harness.run();
        harness.key_press(Key::F10);
        harness.run();
        assert!(!menu_open(&harness, "Menu"));
    }

    #[test]
    fn alt_with_a_menu_s_access_key_opens_the_menu_at_that_menu() {
        let mut harness = bench(OperatingSystem::Nix);
        harness.run();
        alt(&mut harness, Key::F, Some("f"));
        assert!(menu_open(&harness, "Menu") && menu_open(&harness, "File"));
        assert_eq!(focused_name(&harness), "Open…");
        // Alt with another opens it at that one instead.
        alt(&mut harness, Key::E, Some("e"));
        assert!(menu_open(&harness, "Edit") && !menu_open(&harness, "File"));
        assert_eq!(focused_name(&harness), "Undo");
        harness.key_press(Key::Escape);
        harness.run();
        harness.key_press(Key::Escape);
        harness.run();
        assert!(!menu_open(&harness, "Menu"));
        // Neither the press nor its letter reached anything else.
        assert!(harness.state().pressed.is_empty());
        // A letter no menu has, Alt with Shift or Ctrl, and AltGr, which egui never sees
        // as Alt, open nothing.
        alt(&mut harness, Key::Q, Some("q"));
        alt_with(
            &mut harness,
            Modifiers::ALT | Modifiers::SHIFT,
            Key::F,
            Some("F"),
        );
        alt_with(
            &mut harness,
            Modifiers::ALT | Modifiers::COMMAND,
            Key::F,
            None,
        );
        alt_with(&mut harness, Modifiers::NONE, Key::E, Some("ę"));
        assert!(!menu_open(&harness, "Menu"));
        // An English interface on a Cyrillic layout: Alt+F types а, and F is File's.
        alt(&mut harness, Key::F, Some("а"));
        assert!(menu_open(&harness, "File"));
        harness.key_press(Key::Escape);
        harness.key_press(Key::Escape);
        harness.run();
        // In Ukrainian, Alt with the key that types ф, which egui reports as A, and then,
        // with the menu open, the key that types п, reported as G.
        apertrust_ui::set_locale(&harness.ctx, Locale::Uk);
        harness.run();
        alt(&mut harness, Key::A, Some("ф"));
        assert!(menu_open(&harness, "Меню") && menu_open(&harness, "Файл"));
        alt(&mut harness, Key::G, Some("п"));
        assert!(menu_open(&harness, "Перегляд") && !menu_open(&harness, "Файл"));
        harness.key_press(Key::Escape);
        harness.key_press(Key::Escape);
        harness.run();
        // In Polish, Help's is M, since Plik has P.
        apertrust_ui::set_locale(&harness.ctx, Locale::Pl);
        harness.run();
        alt(&mut harness, Key::M, Some("m"));
        assert!(menu_open(&harness, "Pomoc"));
    }

    #[test]
    fn the_zoom_menu_shows_the_keys_of_its_entries_actions() {
        let tr = Translator::new(Locale::EnGb);
        for os in [OperatingSystem::Mac, OperatingSystem::Nix] {
            let keymap = Keymap::defaults(os);
            let keys: Vec<_> = ZOOM_MENU
                .iter()
                .map(|(_, id)| keymap.tip(*id, os, &tr))
                .collect();
            // Z for 100%; Ctrl+0, Ctrl+= and Ctrl+− stay the interface's size's until the
            // viewport can zoom.
            assert_eq!(keys, [None, Some("Z".to_owned()), None, None], "{os:?}");
        }
    }

    #[test]
    fn the_menu_shows_each_item_as_the_registry_has_it() {
        let mut harness = bench(OperatingSystem::Nix);
        harness.run();
        alt(&mut harness, Key::F, Some("f"));
        let item = Role::MenuItem;
        let check = Role::MenuItemCheckBox;
        let keys = |keys: &str| Some(keys.to_owned());
        assert_eq!(
            menu_line(&harness, item, "Open…"),
            (true, keys("Ctrl+O"), None)
        );
        // Built and able to run, with a photo open: enabled, with its key.
        assert_eq!(
            menu_line(&harness, item, "Export…"),
            (true, keys("Ctrl+E"), None)
        );
        assert_eq!(
            menu_line(&harness, item, "Quit"),
            (true, keys("Ctrl+Q"), None)
        );
        // Full screen and the side panel's tabs are checked as they are.
        alt(&mut harness, Key::V, Some("v"));
        assert_eq!(
            menu_line(&harness, check, "Full screen"),
            (true, keys("F11"), Some(Toggled::False))
        );
        // The photo's zoom first, declared but not built: disabled, with its keys.
        assert_eq!(menu_line(&harness, item, "Zoom in"), (false, None, None));
        assert_eq!(
            menu_line(&harness, item, "Zoom to 100%"),
            (false, keys("Z"), None)
        );
        // Then Sort By, its checks the grid panel's sort, resting with no photos shown.
        for _ in 0..4 {
            harness.key_press(Key::ArrowDown);
        }
        harness.key_press(Key::ArrowRight);
        harness.run();
        assert!(menu_open(&harness, "Sort By"));
        assert_eq!(
            menu_line(&harness, check, "Capture time"),
            (false, None, Some(Toggled::True))
        );
        assert_eq!(
            menu_line(&harness, check, "Descending"),
            (false, None, Some(Toggled::False))
        );
        // Show labels after Thumbnail Size, checked while the captions show.
        assert_eq!(
            menu_line(&harness, check, "Show labels"),
            (false, None, Some(Toggled::True))
        );
        harness.key_press(Key::ArrowLeft);
        for _ in 0..5 {
            harness.key_press(Key::ArrowDown);
        }
        harness.key_press(Key::ArrowRight);
        harness.run();
        assert!(menu_open(&harness, "Side Panel Tab"));
        let tab = |harness: &Harness<'_, Bench>, n| {
            menu_line(harness, check, &format!("Side-panel tab {n}")).2
        };
        assert_eq!(tab(&harness, BASE + 1), Some(Toggled::True));
        assert_eq!(tab(&harness, 1), Some(Toggled::False));
        // The Photos tab is the first, built.
        let photos = menu_line(&harness, check, "Side-panel tab 1");
        assert!(photos.0);
        // Reset is disabled while the photo is as it opened.
        harness.key_press(Key::Escape);
        harness.run();
        alt(&mut harness, Key::E, Some("e"));
        assert!(!menu_line(&harness, item, "Reset").0);
    }

    /// File › Close: Ctrl+W on Windows and Linux, and the ☰'s File › Close, with something
    /// open; with nothing open both rest.
    #[test]
    fn close_runs_by_ctrl_w_and_from_the_drawn_file_menu_with_something_open() {
        for os in [OperatingSystem::Windows, OperatingSystem::Nix] {
            let mut harness = bench(os);
            harness.run();
            harness.key_press_modifiers(Modifiers::COMMAND, Key::W);
            harness.run();
            assert!(!harness.state().shell.close, "{os:?}: nothing open");
            harness.state_mut().anything_open = true;
            harness.run();
            harness.key_press_modifiers(Modifiers::COMMAND, Key::W);
            harness.run();
            assert!(
                std::mem::take(&mut harness.state_mut().shell.close),
                "{os:?}"
            );
            assert_eq!(harness.state().pressed, [ActionId::CLOSE], "{os:?}");
            alt(&mut harness, Key::F, Some("f"));
            harness
                .get_by_role_and_label(Role::MenuItem, "Close")
                .click();
            harness.run();
            assert!(harness.state().shell.close, "{os:?}");
            assert_eq!(harness.state().chosen, [ActionId::CLOSE], "{os:?}");
        }
    }

    /// The ☰'s File › Open Recent: the recents by label, a choice asking for its folder, a
    /// missing one disabled, and Clear Menu emptying the list, after which Clear Menu alone
    /// is there, disabled.
    #[test]
    fn the_drawn_open_recent_opens_a_recent_and_clear_menu_empties_it() {
        let mut harness = bench(OperatingSystem::Nix);
        let (wedding, gone) = (
            PathBuf::from("/photos/Wedding"),
            PathBuf::from("/photos/Gone"),
        );
        harness.state_mut().shell.remember_folder(gone.clone());
        harness.state_mut().shell.remember_folder(wedding.clone());
        let recents = &mut harness.state_mut().shell.recents;
        recents.mark(recents::OpenKind::Folder, &gone, false);
        harness.run();
        let open_recent = |harness: &mut Harness<'_, Bench>| {
            alt(harness, Key::F, Some("f"));
            harness.key_press(Key::ArrowDown);
            harness.key_press(Key::ArrowDown);
            harness.key_press(Key::ArrowRight);
            harness.run();
            assert!(menu_open(harness, "Open Recent"));
        };
        open_recent(&mut harness);
        let item = Role::MenuItem;
        assert!(menu_line(&harness, item, "Folder: Wedding").0);
        assert!(
            !menu_line(&harness, item, "Folder: Gone").0,
            "missing: disabled"
        );
        harness
            .get_by_role_and_label(item, "Folder: Wedding")
            .click();
        harness.run();
        assert_eq!(harness.state().chosen, [ActionId::OPEN_RECENT[0]]);
        assert_eq!(harness.state_mut().shell.open_recent.take(), Some(wedding));
        open_recent(&mut harness);
        harness.get_by_role_and_label(item, "Clear menu").click();
        harness.run();
        assert_eq!(harness.state().shell.recents.entries().count(), 0);
        open_recent(&mut harness);
        assert!(harness.query_by_label("Folder: Wedding").is_none());
        assert!(
            !menu_line(&harness, item, "Clear menu").0,
            "nothing to clear"
        );
    }

    /// The ☰'s Open Recent, open while the recents reorder, as a slow open landing does:
    /// its first item opens the folder it shows, not the one now first.
    #[test]
    fn a_choice_in_the_drawn_open_recent_opens_what_it_showed_though_the_list_changed() {
        let mut harness = bench(OperatingSystem::Nix);
        let (older, newer) = (
            PathBuf::from("/photos/Older"),
            PathBuf::from("/photos/Newer"),
        );
        harness.state_mut().shell.remember_folder(older.clone());
        harness.state_mut().shell.remember_folder(newer.clone());
        harness.run();
        alt(&mut harness, Key::F, Some("f"));
        harness.key_press(Key::ArrowDown);
        harness.key_press(Key::ArrowDown);
        harness.key_press(Key::ArrowRight);
        harness.run();
        assert!(menu_open(&harness, "Open Recent"));
        harness.state_mut().shell.remember_folder(older);
        harness.run();
        assert!(menu_open(&harness, "Open Recent"));
        harness
            .get_by_role_and_label(Role::MenuItem, "Folder: Newer")
            .click();
        harness.run();
        assert_eq!(harness.state().chosen, [ActionId::OPEN_RECENT[0]]);
        assert_eq!(harness.state_mut().shell.open_recent.take(), Some(newer));
    }

    #[test]
    fn a_choice_in_the_menu_runs_its_action_once() {
        let mut harness = bench(OperatingSystem::Nix);
        harness.run();
        alt(&mut harness, Key::E, Some("e"));
        harness
            .get_by_role_and_label(Role::MenuItem, "Settings")
            .click();
        harness.run();
        assert!(!menu_open(&harness, "Menu"));
        assert_eq!(harness.state().chosen, vec![ActionId::SETTINGS]);
        assert!(harness.state().settings_open());
        harness.run_steps(10);
        assert_eq!(harness.state().chosen, vec![ActionId::SETTINGS]);
        // From the keys too: Enter on File's first item opens the open dialog, once, once
        // Settings has gone.
        harness.state().host.gone(None);
        harness.run();
        alt(&mut harness, Key::F, Some("f"));
        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(
            harness.state().chosen,
            vec![ActionId::SETTINGS, ActionId::OPEN]
        );
        assert_eq!(harness.state().dialog.asked.len(), 1);
        // A disabled item does nothing.
        harness.state_mut().dialog.answer = Some(None);
        harness.run();
        alt(&mut harness, Key::F, Some("f"));
        harness
            .get_by_role_and_label(Role::MenuItem, "Import…")
            .click();
        harness.run();
        assert_eq!(harness.state().chosen.len(), 2);
    }

    #[test]
    fn a_key_that_runs_an_action_closes_the_menu_and_a_dialog_keeps_the_focus() {
        let mut harness = bench(OperatingSystem::Nix);
        harness.run();
        alt(&mut harness, Key::F, Some("f"));
        assert!(menu_open(&harness, "File"));
        // Ctrl+, opens Settings over it: the menu closes, and Escape in Settings' window
        // closes the dialog (its keys, held in `dialog_host`'s tests), as the host reports.
        press(&harness, Modifiers::COMMAND, Key::Comma, Key::Comma);
        harness.run();
        assert!(harness.state().settings_open());
        assert!(!menu_open(&harness, "Menu"));
        harness.state().host.gone(None);
        harness.run();
        assert!(!harness.state().settings_open());
        // F11 runs Full screen and closes it too, giving the focus back to what had it.
        harness
            .get_by_role_and_label(Role::Button, "About Apertrust")
            .focus();
        harness.run();
        alt(&mut harness, Key::V, Some("v"));
        assert!(menu_open(&harness, "View"));
        assert_ne!(focused_name(&harness), "About Apertrust");
        harness.key_press(Key::F11);
        harness.run();
        assert!(!menu_open(&harness, "Menu"));
        assert_eq!(focused_name(&harness), "About Apertrust");
        // A dialog however opened closes it.
        harness.key_press(Key::F10);
        harness.run();
        assert!(menu_open(&harness, "Menu"));
        harness.state_mut().shell.settings = true;
        harness.run();
        assert!(!menu_open(&harness, "Menu"));
    }
}
