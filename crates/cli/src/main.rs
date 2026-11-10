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

//! `apertrust-cli`: headless Apertrust.
//!
//! It reads the engine's settings, `[engine]`, and the export settings, `[export]`, from
//! `settings.toml` in its user data folder, the app's, found by `apertrust_base::folders`
//! (`--user-data-dir`, then `APERTRUST_USER_DATA_DIR`, then the platform's), and never
//! writes them: it creates nothing in finding the folder (`Folders::find_to_read`) and its
//! store is read-only (decision 43). Only the commands that run the engine read them. A
//! flag of its own, `--adapter`, comes first, then `--conf KEY=VALUE`, which sets one for
//! the run and is never written, then the file. `export` takes from the export settings
//! what darktable-cli takes from darktablerc, the format's options, and its own flags give
//! the rest (decision 56).

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use apertrust_base::folders::{self, Folders, Given};
use apertrust_base::settings::{Declared, FileStorage, Overrides, Setting, Store, Stored};
use apertrust_engine::compute::{GpuImage, Timings};
use apertrust_engine::edit::{self, Edit};
use apertrust_engine::export::{Tiling, export_at};
use apertrust_engine::export_job;
use apertrust_engine::gpu::{self, GpuContext};
use apertrust_engine::jobs::JobEvent;
use apertrust_engine::names::{self, Shot};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe, Pipeline};
use apertrust_engine::scopes::{Graphs, Meter, Reading};
use apertrust_engine::session::{
    self, ExportSettings, ExportStatus, PhotoResult, Session, SessionError,
};
use apertrust_engine::settings::{self as engine, Flags, Source};
use apertrust_engine::{raw, sidecar};
use apertrust_export::disk::{
    Disk, Error as DiskError, Prepared, Skipped, Stored as DiskStored, Target, Why,
};
use apertrust_export::settings::registry::{
    EXPORT_DISK_ON_CONFLICT, EXPORT_DISK_PATH, EXPORT_FORMAT, EXPORT_JPEG_QUALITY,
    EXPORT_JPEG_SUBSAMPLE, EXPORT_PNG_BIT_DEPTH, EXPORT_PNG_COMPRESSION, EXPORT_SIZE_DPI,
    EXPORT_SIZE_MODE,
};
use apertrust_export::settings::{self as export_settings, FileFormat, SizeMode};
use apertrust_export::size::Request;
use apertrust_export::{Format, Places, png, variables};
use apertrust_modules::{Control, ControlKind, ControlValue, Mark, Module};
use apertrust_plugin::{ImageContext, ImageDesc, Operation, ParamValues};
use apertrust_raw::{CfaColor, Pixels, Rect};
use clap::{Parser, Subcommand};

mod beside;
mod library;

/// The operations and the modules the CLI hands the engine, as the app hands them.
const OPERATIONS: &[&dyn Operation] = apertrust_operations::OPERATIONS;
const MODULES: &[&dyn Module] = apertrust_modules::MODULES;

/// The control `preview` times: the Exposure module's Exposure.
const EXPOSURE: (&str, &str) = ("exposure", "exposure");

#[derive(Debug, Parser)]
#[command(version, about = "Headless Apertrust")]
struct Cli {
    #[command(subcommand)]
    command: Command,
    /// Switch on a debug domain, as darktable's -d, on stderr: gpu, raw, pipe, perf,
    /// memory, golden or settings; `all` for every one, `verbose` for more detail. Repeat
    /// for more. With a domain on, a banner naming the build comes first.
    #[arg(short = 'd', long = "debug", global = true, value_name = "DOMAIN")]
    debug: Vec<String>,
    /// The user data folder, whose settings.toml gives the engine's settings as it gives
    /// the app's: read, never written, and nothing created. In place of
    /// APERTRUST_USER_DATA_DIR's, else the platform's. A leading ~ is the home folder; one
    /// given that is missing is an error. The last given counts.
    #[arg(
        long,
        global = true,
        value_name = "DIR",
        visible_aliases = ["config-dir", "configdir"],
        overrides_with = "user_data_dir"
    )]
    user_data_dir: Option<PathBuf>,
    /// The cache folder, in place of APERTRUST_CACHE_DIR's, else the user data folder's
    /// cache when that is given, else the platform's. Nothing is created. A leading ~ is
    /// the home folder. The last given counts.
    #[arg(long, global = true, value_name = "DIR", overrides_with = "cache_dir")]
    cache_dir: Option<PathBuf>,
    /// Set an engine or export setting for this run only, as darktable's --conf, never
    /// written: KEY is its id in settings.toml, such as engine.memory_budget or
    /// export.jpeg.quality, and VALUE as the file holds it, but text without quotes. Before
    /// the file; a flag of its own, such as --adapter, comes first. Repeat for more; the
    /// last for one key counts.
    #[arg(long, global = true, value_name = "KEY=VALUE")]
    conf: Vec<String>,
    /// Open the GPU binding at most this many MiB of a buffer at once, as on a GPU that
    /// binds less, such as lavapipe's 128: the engine then runs the raw stage in row
    /// strips and tiles as it would there.
    #[arg(long, global = true, value_name = "MIB")]
    max_binding: Option<u64>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List GPU adapters and show the one the engine opens.
    Gpu {
        /// Use the first adapter whose name contains this text, ignoring case; one that
        /// names none is an error. Before the settings' engine.adapter; empty for wgpu's
        /// choice.
        #[arg(long)]
        adapter: Option<String>,
    },
    /// Decode a raw file and show what the decoder reports about it.
    Info { file: PathBuf },
    /// The library's commands, run in this process.
    Library {
        #[command(subcommand)]
        command: library::Command,
    },
    /// Render a raw through the viewport pipe, time changes of the Exposure module's
    /// Exposure as a slider drag sends them, and optionally save the viewport image. The
    /// raw stage and demosaic run at full resolution, every later operation at the
    /// viewport's size, and each change restarts from the newest cache point it leaves
    /// valid.
    Preview {
        file: PathBuf,
        /// Viewport bounds in pixels; the image is fitted inside.
        #[arg(long, default_value_t = 2560)]
        width: u32,
        #[arg(long, default_value_t = 1600)]
        height: u32,
        /// The Exposure module's Exposure, in EV as darktable's slider shows it, −18 to
        /// +18 (+0.7 for a colour raw as first shown); the saved edit's when left out.
        #[arg(long, allow_negative_numbers = true)]
        exposure: Option<f32>,
        /// Exposure changes to time.
        #[arg(long, default_value_t = 30)]
        changes: u32,
        /// Save the viewport image here, as export writes a PNG.
        #[arg(long)]
        png: Option<PathBuf>,
        /// Then time an operation an unedited photo does not run joining the plan at its
        /// defaults, as a module adds it, leaving, and joining again: each rebuild keeps
        /// the compiled kernels and the cache points that still hold (decision 28), and
        /// the first is compared with a pipeline built from nothing. Any operation
        /// whose manifest leaves `default_enabled` off, such as colorbalancergb or grain.
        #[arg(long, value_name = "OPERATION")]
        join: Option<String>,
        /// Then time the same exposure changes with each frame counted for the scopes as
        /// the app's session counts it, the count and its readback submitted after the
        /// frame and picked up without waiting, and print what the last frame clips:
        /// `--scopes=totals` alone, as with the Scopes section closed, or with the graph
        /// shown, `--scopes=histogram` (`--scopes` alone), `--scopes=waveform` or
        /// `--scopes=vectorscope`.
        #[arg(
            long,
            value_name = "GRAPH",
            num_args = 0..=1,
            require_equals = true,
            default_missing_value = "histogram"
        )]
        scopes: Option<ScopesGraph>,
        /// Spike X1: in place of the timings
        /// above, time the changes through an in-process session, as the app sends them,
        /// one every --interval, with nothing else to do and while the session's export
        /// job exports the photo as a JPEG at full size, in this many rounds of --changes
        /// changes alone, an export alone and an export beside changes. The export tiles
        /// as the session's budget says, engine.memory_budget.
        #[arg(
            long,
            value_name = "ROUNDS",
            conflicts_with_all = ["png", "join", "scopes", "exposure"]
        )]
        beside_export: Option<u32>,
        /// With --beside-export, the milliseconds between two changes, as a drag sends one
        /// a display frame.
        #[arg(
            long,
            value_name = "MS",
            default_value_t = 16.7,
            value_parser = interval_ms,
            requires = "beside_export"
        )]
        interval: f64,
        /// Use the first adapter whose name contains this text, ignoring case; one that
        /// names none is an error. Before the settings' engine.adapter; empty for wgpu's
        /// choice.
        #[arg(long)]
        adapter: Option<String>,
    },
    /// Change the edit saved beside a raw, `FILE.apxmp` (version 1: module values, and
    /// the parameters no module owns yet), then show it. `export` and `preview` start
    /// from it. Resets apply before sets. An edit a newer Apertrust wrote is refused and
    /// left as it is; `export` and `preview` render what this build reads of it.
    Edit {
        file: PathBuf,
        #[command(flatten)]
        flags: SetFlags,
        /// Return a control to its default for the image, no longer kept; repeat for
        /// more.
        #[arg(long = "reset", value_name = "MODULE.CONTROL")]
        resets: Vec<String>,
        /// Return a parameter `--param` sets to its default for the image; repeat for
        /// more.
        #[arg(long = "reset-param", value_name = "OPERATION.PARAMETER")]
        param_resets: Vec<String>,
        /// Start from no settings at all. The saved edit is still read first, and one
        /// this build cannot read, or a newer Apertrust's, is refused and left as it is;
        /// delete `FILE.apxmp` to start over.
        #[arg(long)]
        clear: bool,
    },
    /// Render raws through the engine's operations and save each, with darktable's sRGB
    /// profile, as a JPEG or a PNG as darktable writes them, with the format's options
    /// the user data folder's export settings give, as darktable-cli takes them from
    /// darktablerc: export.jpeg.quality and .subsample, by default 95 and auto, and
    /// export.png.bit_depth and .compression, by default 8 and 5; in the cm and inch size
    /// modes a JPEG's density is export.size.dpi. --conf export.… sets one for the run. At
    /// full resolution, or within --width and --height as darktable-cli sizes it: the size
    /// comes from these flags alone, as darktable-cli's, never from export.size.
    /// Where the files go is File on disk, as darktable-cli has it: -o is a pattern of
    /// darktable's variables, over export.disk.path, and export.disk.on_conflict decides
    /// what happens to a file already there (create unique filename, overwrite, overwrite
    /// if changed or skip). Each file is written beside its name and renamed into place.
    /// The photo is turned upright as its EXIF orientation says, unless its edit's
    /// Orientation says otherwise, and a size fits the photo as it is turned.
    Export {
        #[arg(required = true, value_name = "FILE")]
        files: Vec<PathBuf>,
        /// Where to write: a pattern of darktable's variables such as
        /// `$(FILE_FOLDER)/exported/$(FILE_NAME)`, export.disk.path where left out; a
        /// folder that exists is `FOLDER/$(FILE_NAME)`. An extension chooses the format, as
        /// darktable-cli's does, whatever export.format says (jpg or jpeg for JPEG, png
        /// for PNG), and is not part of the name; without one the format is export.format's.
        /// With several files and no variable, `_$(SEQUENCE)` is added.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// The largest width in pixels, as darktable-cli's --width; 0 for no limit.
        #[arg(long, default_value_t = 0)]
        width: u32,
        /// The largest height in pixels, as darktable-cli's --height; 0 for no limit.
        #[arg(long, default_value_t = 0)]
        height: u32,
        /// High quality resampling, as darktable-cli's --hq (0, 1, false or true): every
        /// operation before output colour at full resolution and the resample just
        /// before it; otherwise the resample after demosaic. True by default, as in
        /// darktable-cli.
        #[arg(
            long,
            value_name = "BOOL",
            default_value = "true",
            value_parser = parse_bool,
            action = clap::ArgAction::Set
        )]
        hq: bool,
        /// Allow upscaling, as darktable-cli's --upscale (0, 1, false or true): a size
        /// larger than the image grows it, up to 100 times; a side left at 0 takes the
        /// photo's own, as darktable-cli has it, so a single limit never upscales. False
        /// by default.
        #[arg(
            long,
            value_name = "BOOL",
            default_value = "false",
            value_parser = parse_bool,
            action = clap::ArgAction::Set
        )]
        upscale: bool,
        #[command(flatten)]
        flags: SetFlags,
        /// Leave an operation out; repeat for more.
        #[arg(long = "skip", value_name = "OPERATION")]
        skips: Vec<String>,
        /// Use the first adapter whose name contains this text, ignoring case; one that
        /// names none is an error. Before the settings' engine.adapter; empty for wgpu's
        /// choice.
        #[arg(long)]
        adapter: Option<String>,
        /// Render each photo this many times directly, outside the export job, and log
        /// each render's time under `-d perf`: at 1 tiled as it needs, as the job does, as
        /// the timing workflow exports; above 1 whole, with each operation's median GPU
        /// time and the median render, the first, on a cold GPU, counted too.
        #[arg(long)]
        runs: Option<u32>,
        /// Render in tiles of this many pixels across and down. Without it the export
        /// tiles only when the whole image would take more GPU memory than the engine's
        /// budget or a buffer larger than the adapter can bind.
        #[arg(long, value_name = "PIXELS")]
        tile: Option<u32>,
    },
}

/// What `preview --scopes` counts besides the totals.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum ScopesGraph {
    Totals,
    Histogram,
    Waveform,
    Vectorscope,
}

impl ScopesGraph {
    fn graphs(self) -> Graphs {
        Graphs {
            histogram: matches!(self, Self::Histogram),
            waveform: matches!(self, Self::Waveform),
            vectorscope: matches!(self, Self::Vectorscope),
        }
    }
}

/// Values set on top of the saved edit, as a client of the session sets them: module
/// controls, and parameters no module owns yet (decision 28).
#[derive(Debug, Default, clap::Args)]
struct SetFlags {
    /// Set a module's control, such as `colorbalancergb.contrast=20`, which keeps it;
    /// repeat for more.
    #[arg(long = "set", value_name = "MODULE.CONTROL=VALUE")]
    sets: Vec<String>,
    /// Set a parameter no module owns yet, of an operation an unedited photo runs, such as
    /// `colorin.type=lin_rec709`; any other is refused, since modules set it. Repeat for
    /// more.
    #[arg(long = "param", value_name = "OPERATION.PARAMETER=VALUE")]
    params: Vec<String>,
}

impl SetFlags {
    fn is_empty(&self) -> bool {
        self.sets.is_empty() && self.params.is_empty()
    }
}

/// What the banner and `--version` list this binary as built with.
fn build(program: &apertrust_base::log::Program) -> Vec<apertrust_base::log::Item> {
    vec![
        apertrust_base::log::wgpu(program, gpu::backends()),
        apertrust_base::log::rawler(program),
        apertrust_base::log::kernels(program),
        apertrust_base::log::onnx_runtime(),
    ]
}

fn main() -> Result<()> {
    if apertrust_base::log::version_asked() {
        let program = apertrust_base::program!();
        apertrust_base::log::print_version(program, &build(&program));
        return Ok(());
    }
    let cli = Cli::parse();
    let _logging = apertrust_base::log::init(&cli.debug)?;
    apertrust_base::log::starting(apertrust_base::program!(), build);
    // `--conf`'s values for this run, checked against the engine's and the export settings
    // before anything is read, by every command: one the CLI cannot take stops it, as a
    // usage error, with clap's and the app's exit status, 2 (decision 43).
    let overrides = match Overrides::read(&cli.conf, &registries()) {
        Ok(overrides) => overrides,
        Err(error) => {
            eprintln!("apertrust-cli: {error}");
            std::process::exit(2);
        }
    };
    // The user data folder, before anything is read, for reading alone: nothing is created
    // or probed in it, and one given that is missing or cannot be read stops the start,
    // never falling back to the platform's (decision 43). Its error once, cause and all.
    let given = cli.user_data_dir.clone().map(|folder| Given {
        flag: user_data_flag(std::env::args_os()),
        folder,
    });
    let cache = cli.cache_dir.clone();
    let folders = Folders::find_to_read(given, cache, apertrust_base::program!().dev)
        .map_err(|e| anyhow!("{e}"))?;
    // The settings, read only by the commands that run the engine, `--conf`'s over the
    // file's.
    let store = || settings(folders.as_ref()).with_overrides(&overrides);
    // The engine's, under the command's own flags, as its options; the developer's switch,
    // never a setting, lowers the binding after them.
    let engine = |store: &Store, adapter| {
        let mut options = engine::Options::read(store, &Flags { adapter });
        options.gpu.max_storage_binding = cli.max_binding.map(|mib| mib << 20);
        options
    };
    match cli.command {
        Command::Gpu { adapter } => pollster::block_on(show_gpu(engine(&store(), adapter))),
        Command::Info { file } => show_info(&file),
        Command::Library { command } => library::run(command),
        Command::Edit {
            file,
            flags,
            resets,
            param_resets,
            clear,
        } => edit(
            &file,
            EditArgs {
                flags: &flags,
                resets: &resets,
                param_resets: &param_resets,
                clear,
            },
            OPERATIONS,
            MODULES,
        ),
        Command::Preview {
            file,
            width,
            height,
            changes,
            beside_export: Some(rounds),
            interval,
            adapter,
            ..
        } => {
            let store = store();
            beside::run(
                &file,
                beside::Options {
                    bounds: (width, height),
                    changes,
                    interval: Duration::from_secs_f64(interval / 1e3),
                    rounds,
                    format: format_with_options(&store, FileFormat::Jpeg),
                },
                engine(&store, adapter),
            )
        }
        Command::Preview {
            file,
            width,
            height,
            exposure,
            changes,
            png,
            join,
            scopes,
            adapter,
            ..
        } => pollster::block_on(preview(
            &file,
            PreviewOptions {
                pipe: Pipe::Viewport { width, height },
                exposure,
                changes,
                join,
                scopes,
            },
            png.as_deref(),
            engine(&store(), adapter),
        )),
        Command::Export {
            files,
            output,
            width,
            height,
            hq,
            upscale,
            flags,
            skips,
            adapter,
            runs,
            tile,
        } => {
            let store = store();
            let engine = engine(&store, adapter);
            pollster::block_on(export_all(
                &files,
                output.as_deref(),
                &Request {
                    width,
                    height,
                    scale: None,
                    upscale,
                    high_quality: hq,
                    // darktable-cli's rules, not its export module's.
                    dialog: false,
                },
                &store,
                &flags,
                &skips,
                engine,
                runs.map(|runs| runs.max(1)),
                tile,
            ))
        }
    }
}

/// The settings the CLI reads: the engine's, which it hands the engine as options, and the
/// export settings, whose format options `export` takes (decisions 43 and 56).
fn registries() -> Vec<&'static dyn Declared> {
    let export = export_settings::registry::ALL.iter();
    engine::registry::ALL
        .iter()
        .chain(export)
        .copied()
        .collect()
}

/// The store of the engine's and the export settings in `folders`' user data folder,
/// read-only, so the CLI never writes settings; the defaults where there is no folder.
fn settings(folders: Option<&Folders>) -> Store {
    match folders {
        Some(folders) => {
            let storage = FileStorage::new(folders.settings());
            Store::read_only(&storage, &registries())
        }
        None => Store::nowhere(&registries()),
    }
}

/// The flag that named the user data folder, for the log and errors: the last of
/// `folders::FLAGS` in `args`, alone or as `FLAG=DIR`, since clap keeps no alias's name.
fn user_data_flag(args: impl IntoIterator<Item = OsString>) -> &'static str {
    let mut named = folders::FLAGS[0];
    for arg in args {
        if arg == "--" {
            break;
        }
        let bytes = arg.as_encoded_bytes();
        let flag = folders::FLAGS.into_iter().find(|flag| {
            let rest = bytes.strip_prefix(flag.as_bytes());
            rest.is_some_and(|rest| rest.first().is_none_or(|&b| b == b'='))
        });
        if let Some(flag) = flag {
            named = flag;
        }
    }
    named
}

/// The edit saved beside the photo (`sidecar`), read as far as this build can (a newer
/// Apertrust's too, decision 43), or none.
fn saved(
    photo: &Path,
    operations: &[&dyn Operation],
    modules: &[&dyn Module],
) -> Result<Option<sidecar::Loaded>> {
    let path = sidecar::path_for(photo);
    if !path.exists() {
        return Ok(None);
    }
    let loaded = sidecar::read(&path, operations, modules, apertrust_modules::RETIRED)
        .with_context(|| format!("reading {}", path.display()))?;
    tracing::debug!(target: "pipe", "read {}", apertrust_base::log::file_name(&path));
    Ok(Some(loaded))
}

/// The edit `export` and `preview` render: what this build reads of the one saved beside
/// the photo, the warnings logged, or none; and whether a newer Apertrust wrote it, which
/// is rendered as it was (decision 43).
fn saved_edit(
    photo: &Path,
    operations: &[&dyn Operation],
    modules: &[&dyn Module],
) -> Result<(Edit, bool)> {
    Ok(saved(photo, operations, modules)?.map_or_else(
        || (Edit::default(), false),
        |loaded| {
            let newer = loaded.view_only().is_some();
            (loaded.edit, newer)
        },
    ))
}

/// One `--set MODULE.CONTROL=VALUE`, read: a value its control takes, which only the
/// photo's list of a choice's options may still refuse.
struct ControlSet<'m> {
    flag: &'m str,
    module: &'m dyn Module,
    control: &'static Control,
    value: ControlValue,
}

impl ControlSet<'_> {
    /// Whether it sets a choice, whose options depend on the photo (decision 30).
    fn is_choice(&self) -> bool {
        matches!(self.control.kind, ControlKind::Choice { .. })
    }
}

/// Reads each `--set`, refusing one that names no control or a value the control cannot
/// take, before any photo is decoded.
fn read_sets<'m>(sets: &'m [String], modules: &[&'m dyn Module]) -> Result<Vec<ControlSet<'m>>> {
    sets.iter()
        .map(|set| read_set(modules, set).with_context(|| format!("--set {set}")))
        .collect()
}

/// `MODULE.CONTROL=VALUE`.
fn read_set<'m>(modules: &[&'m dyn Module], set: &'m str) -> Result<ControlSet<'m>> {
    let (target, text) = set
        .split_once('=')
        .context("expected MODULE.CONTROL=VALUE")?;
    let (id, name) = target.split_once('.').context("expected MODULE.CONTROL")?;
    let module = edit::module(modules, id)?;
    let control = module
        .definition()
        .control(name)
        .with_context(|| format!("{id} has no control {name}"))?;
    let value = control.parse(text)?;
    control.check(&value)?;
    Ok(ControlSet {
        flag: set,
        module,
        control,
        value,
    })
}

/// Sets each control read on the photo `image`, bringing back inside its range a number
/// the change's move of a range leaves outside (decision 30).
fn set_controls(edit: &mut Edit, sets: &[ControlSet], image: &ImageContext) -> Result<()> {
    for set in sets {
        let before = edit.clone();
        edit.set_control(set.module, set.control.id, set.value.clone(), image)
            .and_then(|()| edit.clamp(&before, &[set.module], image))
            .with_context(|| format!("--set {}", set.flag))?;
    }
    Ok(())
}

/// Brings a saved edit's numbers inside the ranges its modules' values give on `image`
/// (decision 30), each with a warning. Never for an edit a newer Apertrust wrote, which
/// is rendered as it was, as the session shows it (decision 43).
fn clamp_read(edit: &mut Edit, modules: &[&dyn Module], image: &ImageContext) {
    for warning in edit.clamp_read(modules, image) {
        tracing::warn!(target: "pipe", "{warning}");
    }
}

/// Sets each `--param OPERATION.PARAMETER=VALUE`, which no photo changes.
fn set_params(
    edit: &mut Edit,
    params: &[String],
    operations: &[&dyn Operation],
    modules: &[&dyn Module],
) -> Result<()> {
    for set in params {
        set_param(edit, operations, modules, set).with_context(|| format!("--param {set}"))?;
    }
    Ok(())
}

/// `--reset MODULE.CONTROL`, on the photo `image`, bringing back inside its range a
/// number the reset's move of a range leaves outside (decision 30).
fn reset_control(
    edit: &mut Edit,
    modules: &[&dyn Module],
    reset: &str,
    image: &ImageContext,
) -> Result<()> {
    let (id, name) = reset.split_once('.').context("expected MODULE.CONTROL")?;
    let before = edit.clone();
    edit.reset_control(edit::module(modules, id)?, name)?;
    edit.clamp(&before, modules, image)?;
    Ok(())
}

/// `OPERATION.PARAMETER=VALUE`.
fn set_param(
    edit: &mut Edit,
    operations: &[&dyn Operation],
    modules: &[&dyn Module],
    set: &str,
) -> Result<()> {
    let (target, text) = set
        .split_once('=')
        .context("expected OPERATION.PARAMETER=VALUE")?;
    let (id, name) = target
        .split_once('.')
        .context("expected OPERATION.PARAMETER")?;
    let (_, param) = edit::unowned(operations, modules, id, name)?;
    edit.set_param(operations, modules, id, name, param.parse(text)?)?;
    Ok(())
}

/// `--reset-param OPERATION.PARAMETER`.
fn reset_param(
    edit: &mut Edit,
    operations: &[&dyn Operation],
    modules: &[&dyn Module],
    reset: &str,
) -> Result<()> {
    let (id, name) = reset
        .split_once('.')
        .context("expected OPERATION.PARAMETER")?;
    edit.reset_param(operations, modules, id, name)?;
    Ok(())
}

/// The instances `edit` renders on `image` (`edit::instances`), less those left out.
fn instances(
    edit: &Edit,
    image: &ImageContext,
    skips: &[String],
) -> Result<Vec<Instance<'static>>> {
    for id in skips {
        if apertrust_operations::operation(id).is_none() {
            bail!("no operation {id}");
        }
    }
    // A module whose binding fails is an error here: an export renders the edit as it
    // is, or not at all.
    let mut instances = edit::instances(edit, OPERATIONS, MODULES, image).all()?;
    instances.retain(|instance| {
        !skips
            .iter()
            .any(|id| id == instance.operation.manifest().id)
    });
    Ok(instances)
}

/// The edit as lines to show: each module it touches, then each operation it sets a
/// parameter of.
fn describe(edit: &Edit, operations: &[&dyn Operation], modules: &[&dyn Module]) -> Vec<String> {
    let mut lines = Vec::new();
    for module in modules {
        let definition = module.definition();
        let Some(values) = edit.module(definition.id) else {
            continue;
        };
        let controls: Vec<String> = definition
            .controls
            .iter()
            .filter_map(|control| {
                let mark = match values.mark(control.id) {
                    Some(Mark::Kept) => " (kept)",
                    Some(Mark::Auto) => " (Auto)",
                    None => "",
                };
                let value = match values.get(control.id) {
                    Some(value) => control.text(value),
                    None if !mark.is_empty() => "at its default".to_owned(),
                    None => return None,
                };
                Some(format!("{} {value}{mark}", control.id))
            })
            .collect();
        if !controls.is_empty() {
            lines.push(format!("{}: {}", definition.id, controls.join(", ")));
        }
    }
    for operation in operations {
        let manifest = operation.manifest();
        let Some(params) = edit.params(manifest.id) else {
            continue;
        };
        let schema = operation.schema();
        let settings: Vec<String> = params
            .iter()
            .map(|(name, value)| {
                let text = schema
                    .param(name)
                    .map_or_else(|| value.to_string(), |p| p.text(value));
                format!("{name} {text}")
            })
            .collect();
        if !settings.is_empty() {
            lines.push(format!("{}: {}", manifest.id, settings.join(", ")));
        }
    }
    lines
}

/// What `edit` asks of the edit saved beside the photo.
struct EditArgs<'a> {
    flags: &'a SetFlags,
    resets: &'a [String],
    param_resets: &'a [String],
    clear: bool,
}

/// Changes the edit saved beside `photo`, then shows it. An edit a newer Apertrust wrote
/// is refused whatever the flags, `--clear` included, and left as it is (decision 43).
fn edit(
    photo: &Path,
    args: EditArgs,
    operations: &[&dyn Operation],
    modules: &[&dyn Module],
) -> Result<()> {
    if !photo.is_file() {
        bail!("no file {}", photo.display());
    }
    let path = sidecar::path_for(photo);
    let saved = saved(photo, operations, modules)?;
    if let Some(view_only) = saved.as_ref().and_then(sidecar::Loaded::view_only) {
        return Err(view_only).with_context(|| format!("not changing {}", path.display()));
    }
    let mut edit = match saved {
        Some(loaded) if !args.clear => loaded.edit,
        _ => Edit::default(),
    };
    let sets = read_sets(&args.flags.sets, modules)?;
    // Only a choice reads the photo, for the options it lists (decision 30), so the raw
    // is decoded only for a --set that names one; any other is set on an empty image, as
    // no range reads the photo (decision 30).
    let image = if sets.iter().any(ControlSet::is_choice) {
        let raw = apertrust_raw::decode_file(photo)
            .with_context(|| format!("decoding {} for its options", photo.display()))?;
        raw::image_context(&raw)
    } else {
        ImageContext::full(0, 0)
    };
    clamp_read(&mut edit, modules, &image);
    for reset in args.resets {
        reset_control(&mut edit, modules, reset, &image)
            .with_context(|| format!("--reset {reset}"))?;
    }
    for reset in args.param_resets {
        reset_param(&mut edit, operations, modules, reset)
            .with_context(|| format!("--reset-param {reset}"))?;
    }
    set_controls(&mut edit, &sets, &image)?;
    set_params(&mut edit, &args.flags.params, operations, modules)?;
    if args.clear
        || !args.flags.is_empty()
        || !args.resets.is_empty()
        || !args.param_resets.is_empty()
    {
        sidecar::write(
            &path,
            &edit,
            operations,
            modules,
            apertrust_base::program!().version,
        )
        .with_context(|| format!("writing {}", path.display()))?;
        println!("Wrote {}", path.display());
    }
    let lines = describe(&edit, operations, modules);
    if lines.is_empty() {
        println!("No settings: every module and operation keeps its defaults for the image.");
    }
    for line in lines {
        println!("{line}");
    }
    Ok(())
}

/// What `export` does for every photo of its run.
struct Run<'a> {
    request: &'a Request,
    flags: &'a SetFlags,
    skips: &'a [String],
    engine: &'a engine::Options,
    runs: u32,
    tile: Option<u32>,
    format: Format,
    disk: Disk,
    /// What the variables take from the run itself.
    names: names::Export,
}

/// `export`: `files` to the files File on disk (decision 56) names, `output` over
/// export.disk.path, as darktable-cli has it, through an in-process session's export job
/// ([`export_in_session`]), as the app exports; with `--runs` above 1, `--tile`, `--skip`,
/// `--set` or `--param`, each through [`export`] in turn, rendered directly outside the
/// job, for the timings and the harness, and for what a flag changes of the saved edit,
/// which the job takes as it is (D12). Either way a photo that fails is reported and the
/// others go on, and the status is that of any having failed, as darktable-cli's; a
/// failure of the destination, a folder that cannot be made or written, stops the run,
/// since every later photo would fail the same way, as darktable's export job stops
/// (`Error::destination`).
#[allow(
    clippy::too_many_arguments,
    reason = "the export command's flags, one by one"
)]
async fn export_all(
    files: &[PathBuf],
    output: Option<&Path>,
    request: &Request,
    store: &Store,
    flags: &SetFlags,
    skips: &[String],
    engine: engine::Options,
    runs: Option<u32>,
    tile: Option<u32>,
) -> Result<()> {
    // The destination and the pattern are checked before any raw is decoded (D8).
    let (pattern, format) = destination(output, store)?;
    log_setting(store, &EXPORT_DISK_ON_CONFLICT, "");
    let on_conflict = store.get(&EXPORT_DISK_ON_CONFLICT);
    let largest = |side: u32| {
        if side == 0 {
            0
        } else {
            side.min(format.largest())
        }
    };
    let names = names::Export::new(
        names::now(),
        variables::Size {
            width: largest(request.width),
            height: largest(request.height),
        },
        request.upscale,
        apertrust_base::program!().version,
        places(),
    );
    let disk = Disk::new(&pattern, on_conflict, files.len(), names.home())?;
    // Any of these renders each photo directly, outside the job: the timings, a tile size
    // and what a flag changes of the saved edit, which the job takes as it is (D12).
    let Some(runs) =
        runs.or_else(|| (tile.is_some() || !skips.is_empty() || !flags.is_empty()).then_some(1))
    else {
        let settings = ExportSettings {
            path: pattern,
            on_conflict,
            format,
            size: request.clone(),
        };
        return export_in_session(files, settings, places(), engine).await;
    };
    let run = Run {
        request,
        flags,
        skips,
        engine: &engine,
        runs,
        tile,
        format,
        disk,
        names,
    };
    // Made once, for the first photo that is decoded, and kept for the others.
    let mut gpu = None;
    let mut failed = 0;
    for (index, file) in files.iter().enumerate() {
        let number = index as u32 + 1;
        let Err(error) = export(file, number, &run, &mut gpu).await else {
            continue;
        };
        if files.len() == 1 {
            return Err(error);
        }
        eprintln!("apertrust-cli: {}: {error:#}", file.display());
        failed += 1;
        let destination = destination_failed(&error);
        if destination {
            let rest = files.len() - index - 1;
            if rest > 0 {
                eprintln!("apertrust-cli: the destination failed: {rest} not reached");
            }
            failed += rest;
            break;
        }
    }
    if failed > 0 {
        bail!("{failed} of {} photos not exported", files.len());
    }
    Ok(())
}

/// `export` through an in-process session's export job (decision 56; `docs/engine.md`,
/// The CLI): `Export` sent, and each photo's result printed as the job reports it, until
/// the job ends. A single photo's failure is the command's error, as before the job.
async fn export_in_session(
    files: &[PathBuf],
    settings: ExportSettings,
    places: Places,
    engine: engine::Options,
) -> Result<()> {
    let gpu = GpuContext::new(&engine.gpu).await?;
    let (woken, wakes) = mpsc::channel();
    let session = Session::start(
        gpu,
        OPERATIONS,
        MODULES,
        apertrust_base::program!().version,
        engine.budget,
        move || {
            let _ = woken.send(());
        },
    );
    let command = session.send(session::Command::Export {
        photos: files.to_vec(),
        settings,
        places,
    });
    let mut job = None;
    let mut printed = Printed::new(files);
    let mut ended = None;
    while ended.is_none() {
        // The session wakes this as events arrive; a look now and then besides, should
        // its thread end without a word.
        let _ = wakes.recv_timeout(Duration::from_millis(200));
        for event in session.events() {
            match event {
                session::Event::Job {
                    event: JobEvent::Added { job: added, .. },
                    command: Some(sent),
                } if sent == command => job = Some(added),
                session::Event::Export { job: own, status } if Some(own) == job => {
                    printed.status(status);
                }
                session::Event::Job {
                    event:
                        JobEvent::Ended {
                            job: own, failure, ..
                        },
                    ..
                } if Some(own) == job => ended = Some(failure),
                session::Event::Failed(SessionError::Stopped) => bail!("the engine stopped"),
                session::Event::Failed(error) => tracing::warn!(target: "pipe", "{error}"),
                _ => {}
            }
        }
    }
    if let Some(message) = printed.single {
        return Err(anyhow!(message));
    }
    // A job that ended failed before any photo, which the checks before it leave only to
    // the engine itself.
    if let Some(Some(failure)) = ended
        && printed.done == 0
    {
        bail!("{}", failure.reason);
    }
    if printed.failed > 0 {
        bail!("{} of {} photos not exported", printed.failed, files.len());
    }
    Ok(())
}

/// The person's folders and name on this machine, for darktable's variables, which the
/// engine reads from no platform folder itself (decision 43).
fn places() -> Places {
    let dirs = directories::UserDirs::new();
    let folder = |pick: fn(&directories::UserDirs) -> Option<&Path>| {
        dirs.as_ref().and_then(pick).map(Path::to_owned)
    };
    Places {
        home: folder(|dirs| Some(dirs.home_dir())),
        pictures: folder(directories::UserDirs::picture_dir),
        desktop: folder(directories::UserDirs::desktop_dir),
        user_name: ["USER", "USERNAME", "LOGNAME"]
            .iter()
            .find_map(|name| std::env::var(name).ok().filter(|v| !v.is_empty()))
            .unwrap_or_default(),
    }
}

/// What `export` has printed of its job's photos.
struct Printed<'a> {
    files: &'a [PathBuf],
    /// When each photo began, for its line.
    started: Vec<Instant>,
    /// How many photos have ended.
    done: usize,
    /// How many were not exported: failed, or not reached after a failure of the
    /// destination.
    failed: usize,
    /// A single photo's failure, which is the command's error, as before the job.
    single: Option<String>,
}

impl<'a> Printed<'a> {
    fn new(files: &'a [PathBuf]) -> Self {
        Self {
            files,
            started: vec![Instant::now(); files.len()],
            done: 0,
            failed: 0,
            single: None,
        }
    }

    /// Prints what the job reports, as `export` printed each photo before the job.
    fn status(&mut self, status: ExportStatus) {
        match status {
            ExportStatus::Started { index, .. } => self.started[index] = Instant::now(),
            ExportStatus::Done { index, result } => {
                self.done += 1;
                self.photo(index, result);
            }
            ExportStatus::Finished(summary) => {
                if summary.not_reached > 0 && self.failed > 0 && self.files.len() > 1 {
                    let rest = summary.not_reached;
                    eprintln!("apertrust-cli: the destination failed: {rest} not reached");
                }
                self.failed += summary.not_reached;
            }
        }
    }

    fn photo(&mut self, index: usize, result: PhotoResult) {
        match result {
            PhotoResult::Written {
                path,
                width,
                height,
                tiles,
                ..
            } => {
                let tiles = if tiles > 1 {
                    format!(" in {tiles} tiles")
                } else {
                    String::new()
                };
                let stored = DiskStored::Written(path);
                let started = self.started[index];
                println!("{}", stored_line(&stored, width, height, &tiles, started));
            }
            PhotoResult::Skipped { path, why } => {
                println!("{}", skipped_line(&Skipped { path, why }));
            }
            PhotoResult::Failed(failure) => {
                self.failed += 1;
                if self.files.len() == 1 {
                    self.single = Some(failure.message);
                } else {
                    let file = self.files[index].display();
                    eprintln!("apertrust-cli: {file}: {}", failure.message);
                }
            }
        }
    }
}

/// Photo number `number` of the run: rendered, and written as File on disk says, or
/// skipped before it is rendered where On conflict leaves it out.
async fn export(
    path: &Path,
    number: u32,
    run: &Run<'_>,
    gpu: &mut Option<GpuContext>,
) -> Result<()> {
    let Run {
        request,
        flags,
        skips,
        engine,
        runs,
        tile,
        format,
        disk,
        names,
    } = run;
    let runs = (*runs).max(1);
    // The flags are checked before the raw is decoded, and a choice's option against the
    // photo's list after it (decision 30).
    let (mut edit, newer) = saved_edit(path, OPERATIONS, MODULES)?;
    let sets = read_sets(&flags.sets, MODULES)?;
    set_params(&mut edit, &flags.params, OPERATIONS, MODULES)?;
    let start = Instant::now();
    let raw = apertrust_raw::decode_file(path)?;
    let image = raw::image_context(&raw);
    if !newer {
        clamp_read(&mut edit, MODULES, &image);
    }
    set_controls(&mut edit, &sets, &image)?;
    if gpu.is_none() {
        *gpu = Some(GpuContext::new(&engine.gpu).await?);
    }
    let gpu = gpu.as_ref().expect("made above");
    let instances = instances(&edit, &image, skips)?;
    let input =
        raw::upload(gpu, &raw).context("the raw stage takes only 16-bit mosaics so far")??;
    let limits = Limits {
        budget: engine.budget,
        ..Limits::of(gpu)
    };
    let (resample, extent) =
        export_job::size(request, format, &instances, &image, input.desc(), &limits)?;
    // The file's name, and so whether the photo is exported at all, before it is rendered:
    // the folder made, On conflict applied, and the temporary file the render is written to.
    let sizes = names::Sizes {
        sensor: variables::Size {
            width: raw.width,
            height: raw.height,
        },
        developed: variables::Size {
            width: image.width,
            height: image.height,
        },
        crop: variables::Size {
            width: extent[0],
            height: extent[1],
        },
    };
    let values = names.values(path, &Shot::of(&raw), number, sizes);
    // The edit's change time, for Overwrite if changed: the sidecar's; none, no edit.
    let changed = std::fs::metadata(sidecar::path_for(path))
        .and_then(|m| m.modified())
        .ok();
    let target = match disk.prepare(&values, format.extension(), changed)? {
        Prepared::Write(target) => target,
        Prepared::Skip(skipped) => {
            println!("{}", skipped_line(&skipped));
            return Ok(());
        }
    };
    if runs == 1 {
        let tiling = tile.map_or(
            Tiling::Auto {
                budget: engine.budget,
            },
            |edge| Tiling::Tiles([edge, edge]),
        );
        let exported = export_at(gpu, &instances, &image, input, tiling, resample).await?;
        let ImageDesc { width, height, .. } = exported.desc;
        let stored = write(target, format, &exported.pixels, width, height)?;
        let tiles = if exported.tiles > 1 {
            format!(" in {} tiles", exported.tiles)
        } else {
            String::new()
        };
        println!("{}", stored_line(&stored, width, height, &tiles, start));
        return Ok(());
    }
    let pipe = resample.map_or(Pipe::Export, Pipe::Resampled);
    let plan = pipeline::plan(&instances, &image, input.desc(), pipe, &limits)?;
    let (_, output_desc) = plan.output();
    let (width, height) = (output_desc.width, output_desc.height);
    // Each operation's passes, in dispatch order, to add their times up.
    let operations: Vec<(&str, usize)> = plan
        .operations
        .iter()
        .map(|op| (op.id, op.passes.len()))
        .collect();
    let mut pipeline = Pipeline::new(gpu, plan, input).await?;
    let mut renders = Vec::new();
    for _ in 0..runs {
        renders.push(pipeline.render(gpu)?);
    }
    if runs > 1 {
        log_medians(&operations, &renders);
    }

    let stored = write(
        target,
        format,
        &pipeline.output().download(gpu)?,
        width,
        height,
    )?;
    println!("{}", stored_line(&stored, width, height, "", start));
    Ok(())
}

/// What `export` says of a photo it left out.
fn skipped_line(skipped: &Skipped) -> String {
    let why = match skipped.why {
        Why::Exists => "a file is there",
        Why::NotChanged => "not changed since it was exported",
    };
    format!("Skipped {} ({why})", skipped.path.display())
}

/// What `export` says of a photo it stored, `extra` after the size.
fn stored_line(
    stored: &DiskStored,
    width: u32,
    height: u32,
    extra: &str,
    start: Instant,
) -> String {
    match stored {
        DiskStored::Written(path) => format!(
            "Wrote {} ({width}×{height}){extra} in {:.2} s",
            path.display(),
            start.elapsed().as_secs_f64()
        ),
        DiskStored::Skipped(skipped) => skipped_line(skipped),
    }
}

/// `--interval`: milliseconds above 0, a finite number.
fn interval_ms(value: &str) -> Result<f64, String> {
    match value.parse::<f64>() {
        Ok(ms) if ms.is_finite() && ms > 0.0 => Ok(ms),
        _ => Err(format!("milliseconds above 0, not '{value}'")),
    }
}

/// darktable-cli's boolean options' values, `--hq` and `--upscale`: 0, 1, false or true,
/// whatever their case.
fn parse_bool(value: &str) -> Result<bool, String> {
    match value.to_ascii_uppercase().as_str() {
        "0" | "FALSE" => Ok(false),
        "1" | "TRUE" => Ok(true),
        _ => Err(format!("{value} is none of 0, 1, false and true")),
    }
}

/// The format `-o`'s extension chooses, as darktable-cli chooses it (`src/cli/main.c`):
/// `jpg` or `jpeg` for JPEG and `png` for PNG, its case counting, whatever `export.format`
/// says; with the options the export settings in `store` give it, as darktable-cli's
/// format takes them from darktablerc (decision 56). Each is logged under `-d settings`
/// with where it came from.
fn format_of(output: &Path, store: &Store) -> Result<Format> {
    let extension = output.extension().map(|e| e.to_string_lossy());
    let chosen = match extension.as_deref() {
        None | Some("") => bail!("no output file extension given: {}", output.display()),
        Some("jpg" | "jpeg") => FileFormat::Jpeg,
        Some("png") => FileFormat::Png,
        Some(other) => bail!("unknown extension '.{other}': export writes .jpg, .jpeg and .png"),
    };
    let (id, value) = (EXPORT_FORMAT.id, shown(&EXPORT_FORMAT, &chosen));
    tracing::debug!(target: "settings", "{id}: {value}, from: {}", Source::Flag("-o"));
    Ok(format_with_options(store, chosen))
}

/// `chosen` with the options the export settings in `store` give it, each logged with
/// where it came from.
fn format_with_options(store: &Store, chosen: FileFormat) -> Format {
    let format = export_settings::format(store, chosen);
    match format {
        Format::Jpeg(options) => {
            log_setting(store, &EXPORT_JPEG_QUALITY, "");
            log_setting(store, &EXPORT_JPEG_SUBSAMPLE, "");
            // The density follows the size's mode, as darktable's `metadata/resolution`,
            // and the dpi gives it in the print modes.
            let density = format!("density: {} dpi", options.dpi);
            log_setting(store, &EXPORT_SIZE_MODE, &density);
            if matches!(store.get(&EXPORT_SIZE_MODE), SizeMode::Cm | SizeMode::Inch) {
                log_setting(store, &EXPORT_SIZE_DPI, "");
            }
        }
        Format::Png(_) => {
            log_setting(store, &EXPORT_PNG_BIT_DEPTH, "");
            log_setting(store, &EXPORT_PNG_COMPRESSION, "");
        }
    }
    format
}

/// Where `-o` sends the files, as darktable-cli reads it (`src/cli/main.c`), as a pattern
/// of darktable's variables without the format's extension, and the format:
///
/// - left out: export.disk.path, and the format export.format gives;
/// - a folder that exists: `FOLDER/$(FILE_NAME)`, and the format export.format gives, where
///   darktable-cli's is JPEG;
/// - a name with an extension: the name without it, and the format the extension chooses
///   ([`format_of`]), whatever export.format says.
fn destination(output: Option<&Path>, store: &Store) -> Result<(String, Format)> {
    let from_settings = || {
        log_setting(store, &EXPORT_FORMAT, "");
        format_with_options(store, store.get(&EXPORT_FORMAT))
    };
    // A pattern reads `/` only: a `\` would escape.
    let text = |path: &Path| {
        let text = path.to_string_lossy();
        if cfg!(windows) {
            text.replace('\\', "/")
        } else {
            text.into_owned()
        }
    };
    let Some(output) = output else {
        log_setting(store, &EXPORT_DISK_PATH, "");
        return Ok((store.get(&EXPORT_DISK_PATH), from_settings()));
    };
    if output.is_dir() {
        let folder = text(output);
        let pattern = format!("{}/$(FILE_NAME)", folder.trim_end_matches('/'));
        tracing::debug!(target: "pipe", "output is a folder: the pattern is {pattern}");
        return Ok((pattern, from_settings()));
    }
    let format = format_of(output, store)?;
    Ok((text(&output.with_extension("")), format))
}

/// Logs `store`'s value of `setting` under `-d settings`, with where it came from, then
/// `note` where there is one.
fn log_setting<T: Stored>(store: &Store, setting: &Setting<T>, note: &str) {
    let value = shown(setting, &store.get(setting));
    let from = source(store, setting);
    let note = if note.is_empty() {
        String::new()
    } else {
        format!(", {note}")
    };
    tracing::debug!(target: "settings", "{}: {value}, from: {from}{note}", setting.id);
}

/// Where `store`'s value of `setting` came from: `--conf`, the file or the default.
fn source<T: Stored>(store: &Store, setting: &Setting<T>) -> Source {
    if store.overridden(setting) {
        Source::Conf
    } else if store.kept(setting).is_some() {
        Source::File
    } else {
        Source::Default
    }
}

/// `value` of `setting` as `settings.toml` holds it: a choice by its id, in quotes.
fn shown<T: Stored>(setting: &Setting<T>, value: &T::Owned) -> String {
    setting
        .write(value)
        .map_or_else(String::new, |value| value.to_string())
}

/// Writes an image output colour has encoded for darktable's sRGB in `format` to `path`
/// as it is, for `preview --png`.
fn write_file(
    path: &Path,
    format: &Format,
    rgba: &[[f32; 4]],
    width: u32,
    height: u32,
) -> Result<()> {
    let image = apertrust_export::Image::new(rgba, width, height)?;
    let file =
        std::fs::File::create(path).with_context(|| format!("cannot create {}", path.display()))?;
    let mut out = std::io::BufWriter::new(file);
    let context = || format!("cannot write {}", path.display());
    format.write(image, &mut out).with_context(context)?;
    out.flush().with_context(context)
}

/// Whether `error` is the destination's, not the photo's: a folder that cannot be made or
/// written, or a disk full, over quota or read-only, found by the rename or by the encoder
/// as it writes. A run of several photos stops at one.
fn destination_failed(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<DiskError>()
        .is_some_and(DiskError::destination)
        || error
            .downcast_ref::<apertrust_export::Error>()
            .is_some_and(apertrust_export::Error::destination)
}

/// Writes an image output colour has encoded for darktable's sRGB in `format`, through
/// `apertrust-export`, into the photo's temporary file, and renames it into place. Dropped
/// on an error, the target removes the file.
fn write(
    mut target: Target,
    format: &Format,
    rgba: &[[f32; 4]],
    width: u32,
    height: u32,
) -> Result<DiskStored> {
    let image = apertrust_export::Image::new(rgba, width, height)?;
    let start = Instant::now();
    let path = target.path();
    let mut out = std::io::BufWriter::new(target.file());
    let context = || format!("cannot write {}", path.display());
    format.write(image, &mut out).with_context(context)?;
    // The buffer's last bytes can meet a full disk too: an export error, so it is told.
    out.flush()
        .map_err(apertrust_export::Error::Io)
        .with_context(context)?;
    drop(out);
    let stored = target.finish()?;
    tracing::debug!(
        target: "perf",
        "wrote {} in {:.0} ms",
        apertrust_base::log::file_name(&path),
        start.elapsed().as_secs_f64() * 1e3
    );
    Ok(stored)
}

/// The median of the whole render and of each operation's GPU time, its passes added
/// up, over several renders: one line each, since they are what `--runs` asks for.
fn log_medians(operations: &[(&str, usize)], renders: &[Timings]) {
    let median = |mut values: Vec<f64>| {
        values.sort_by(f64::total_cmp);
        values[values.len() / 2]
    };
    let gpu = median(
        renders
            .iter()
            .map(|r| r.passes.iter().map(|(_, ms)| ms).sum())
            .collect(),
    );
    let wall = median(renders.iter().map(|r| r.wall_ms).collect());
    tracing::debug!(
        target: "perf",
        "median of {} renders: {wall:.1} ms, GPU: {gpu:.1} ms",
        renders.len(),
    );
    let mut first = 0;
    for &(id, count) in operations {
        let times = renders
            .iter()
            .map(|r| {
                r.passes[first..first + count]
                    .iter()
                    .map(|(_, ms)| ms)
                    .sum()
            })
            .collect();
        tracing::debug!(target: "perf", "median {id}: {:.1} ms GPU", median(times));
        first += count;
    }
}

struct PreviewOptions {
    pipe: Pipe,
    exposure: Option<f32>,
    changes: u32,
    /// An optional operation to time joining the plan.
    join: Option<String>,
    /// The scopes to count while timing the changes again.
    scopes: Option<ScopesGraph>,
}

/// The built-in operations an unedited photo does not run, which `preview --join` adds
/// as their modules do (decision 28).
fn optional() -> impl Iterator<Item = &'static dyn Operation> {
    OPERATIONS
        .iter()
        .copied()
        .filter(|operation| !operation.manifest().default_enabled)
}

async fn preview(
    path: &Path,
    options: PreviewOptions,
    png_path: Option<&Path>,
    engine: engine::Options,
) -> Result<()> {
    let joining = options
        .join
        .as_deref()
        .map(|id| {
            optional()
                .find(|operation| operation.manifest().id == id)
                .with_context(|| {
                    let ids: Vec<&str> = optional().map(|op| op.manifest().id).collect();
                    format!("--join {id}: expected one of {}", ids.join(", "))
                })
        })
        .transpose()?;
    let start = Instant::now();
    let raw = apertrust_raw::decode_file(path)?;
    let decoded_ms = start.elapsed().as_secs_f64() * 1e3;
    let gpu = GpuContext::new(&engine.gpu).await?;
    println!("GPU: {}", gpu.adapter.get_info().name);
    let image = raw::image_context(&raw);
    let input =
        raw::upload(&gpu, &raw).context("the raw stage takes only 16-bit mosaics so far")??;
    let sensor = input.desc();

    // Each change sets the Exposure module's Exposure, as the app's slider does.
    let (mut edit, newer) = saved_edit(path, OPERATIONS, MODULES)?;
    if !newer {
        clamp_read(&mut edit, MODULES, &image);
    }
    let module = edit::module(MODULES, EXPOSURE.0)?;
    let base_ev = match options.exposure {
        Some(ev) => ev,
        None => edit
            .module(EXPOSURE.0)
            .and_then(|values| values.number(EXPOSURE.1))
            .unwrap_or_else(|| module.defaults(&image).number(EXPOSURE.1)),
    };
    // Each change moves a hundredth of a stop, back from +4 rather than past it.
    let step = if base_ev + 0.01 > 4.0 { -0.01 } else { 0.01 };
    let limits = Limits {
        budget: engine.budget,
        ..Limits::of(&gpu)
    };
    let mut plan_at = |ev: f32| -> Result<pipeline::Plan> {
        edit.set_control(module, EXPOSURE.1, ControlValue::Number(ev), &image)
            .context("preview --exposure")?;
        let instances = instances(&edit, &image, &[])?;
        Ok(pipeline::plan(
            &instances,
            &image,
            sensor,
            options.pipe,
            &limits,
        )?)
    };
    let mut pipeline = Pipeline::new(&gpu, plan_at(base_ev)?, input.clone()).await?;
    let first = pipeline.render(&gpu)?;
    let desc = pipeline.output().desc();
    let plan = pipeline.plan();
    let tiles = plan.tiled.as_ref().map_or(String::new(), |tiled| {
        // One downscale per tile.
        let count = plan.operations[tiled.clone()]
            .iter()
            .filter(|op| op.id == "downscale")
            .count();
        format!(", demosaic in {count} tiles")
    });
    println!(
        "Decoded in {decoded_ms:.0} ms. First render, {}×{}, the raw stage and demosaic at full resolution{tiles}: {:.1} ms ({})",
        desc.width,
        desc.height,
        first.wall_ms,
        gpu_times(&first)
    );

    // Exposure changes, as a slider drag sends them: each re-plans, writes the changed
    // uniforms and restarts at exposure, which reads the downscale's cache point.
    let mut runs = Vec::new();
    for i in 0..options.changes {
        let ev = base_ev + if i % 2 == 0 { step } else { 0.0 };
        pipeline = pipeline.update(&gpu, plan_at(ev)?).await?;
        runs.push(pipeline.render(&gpu)?);
    }
    if !runs.is_empty() {
        let (wall, wall_min) = median(runs.iter().map(|t| t.wall_ms).collect());
        println!(
            "Per exposure change, median of {}: {wall:.2} ms submit to done (min {wall_min:.2})",
            runs.len()
        );
        for (pass, (name, _)) in runs[0].passes.iter().enumerate() {
            let (median_ms, min_ms) = median(runs.iter().map(|t| t.passes[pass].1).collect());
            println!("  {name}: {median_ms:.2} ms GPU (min {min_ms:.2})");
        }
    }

    match (options.scopes, runs.is_empty()) {
        (Some(_), true) => tracing::warn!(
            target: "perf",
            "--scopes with --changes 0: no changes to count scopes on"
        ),
        (Some(graph), false) => {
            let changes = Changes {
                base_ev,
                step,
                without: &runs,
            };
            pipeline = time_scopes(&gpu, pipeline, &mut plan_at, changes, graph).await?;
        }
        (None, _) => {}
    }

    if let Some(png_path) = png_path {
        pipeline = pipeline.update(&gpu, plan_at(base_ev)?).await?;
        pipeline.render(&gpu)?;
        let desc = pipeline.output().desc();
        write_file(
            png_path,
            &Format::Png(png::Options::default()),
            &pipeline.output().download(&gpu)?,
            desc.width,
            desc.height,
        )?;
        println!(
            "Wrote {}×{} to {}",
            desc.width,
            desc.height,
            png_path.display()
        );
    }

    if let Some(operation) = joining {
        // From the photo at the Exposure set, rendered, as a drag leaves it.
        let id = operation.manifest().id;
        edit.set_control(module, EXPOSURE.1, ControlValue::Number(base_ev), &image)
            .context("preview --exposure")?;
        let unjoined = instances(&edit, &image, &[])?;
        if unjoined.iter().any(|i| i.operation.manifest().id == id) {
            bail!("--join {id}: the photo's saved edit runs it already");
        }
        // At its schema's defaults, as its module's first step adds it.
        let mut with = unjoined.clone();
        with.push(Instance {
            operation,
            params: ParamValues::new(),
        });
        let plan = |instances: &[Instance]| {
            pipeline::plan(instances, &image, sensor, options.pipe, &limits)
        };
        let (base, joined) = (plan(&unjoined)?, plan(&with)?);
        pipeline = pipeline.update(&gpu, base.clone()).await?;
        pipeline.render(&gpu)?;
        time_rebuilds(&gpu, pipeline, input, id, base, joined).await?;
    }
    Ok(())
}

/// The median of `values` and their least.
fn median(mut values: Vec<f64>) -> (f64, f64) {
    values.sort_by(f64::total_cmp);
    (values[values.len() / 2], values[0])
}

/// The exposure changes `preview` timed: from `base_ev`, every other one `step` away.
struct Changes<'a> {
    base_ev: f32,
    step: f32,
    /// Their timings without the scopes.
    without: &'a [Timings],
}

/// Times the same exposure changes again, each frame counted for the scopes as the
/// session counts it: the count and its copy into a staging buffer submitted after the
/// frame, the reading picked up by polling between changes, never waited for. Prints what
/// that adds to a change, how late the readings arrive, and what the last frame clips
/// (the pipeline timings).
async fn time_scopes(
    gpu: &GpuContext,
    mut pipeline: Pipeline,
    mut plan_at: impl FnMut(f32) -> Result<pipeline::Plan>,
    changes: Changes<'_>,
    graph: ScopesGraph,
) -> Result<Pipeline> {
    let Changes {
        base_ev,
        step,
        without,
    } = changes;
    let graphs = graph.graphs();
    let mut meter = Meter::new(gpu).await?;
    let (mut with, mut counting, mut readings) = (Vec::new(), Vec::new(), Vec::new());
    for i in 0..without.len() {
        let ev = base_ev + if i % 2 == 0 { step } else { 0.0 };
        pipeline = pipeline.update(gpu, plan_at(ev)?).await?;
        with.push(pipeline.render(gpu)?.wall_ms);
        let start = Instant::now();
        meter.measure(gpu, &pipeline.output(), graphs, i as u64 + 1);
        counting.push(start.elapsed().as_secs_f64() * 1e3);
        meter.poll(gpu, &mut readings)?;
    }
    readings.extend(meter.wait(gpu)?);
    // The same frame counted with nothing else to do, its reading polled for as the
    // session's thread polls between commands.
    let mut idle: Vec<Reading> = Vec::new();
    for through in 0..10 {
        meter.measure(gpu, &pipeline.output(), graphs, through);
        idle.extend(meter.wait(gpu)?);
    }

    let (wall, wall_min) = median(with);
    let (plain, _) = median(without.iter().map(|t| t.wall_ms).collect());
    let (cpu, _) = median(counting);
    // A count the GPU gave no times for has none.
    let timed: Vec<f64> = readings
        .iter()
        .chain(&idle)
        .map(|r| r.gpu_ms)
        .filter(|ms| !ms.is_nan())
        .collect();
    let on_gpu = if timed.is_empty() {
        String::new()
    } else {
        let (gpu_ms, gpu_min) = median(timed);
        format!(", {gpu_ms:.2} ms on the GPU (min {gpu_min:.2})")
    };
    let (late, late_min) = median(readings.iter().map(|r| r.latency_ms).collect());
    let (idle_late, idle_min) = median(idle.iter().map(|r| r.latency_ms).collect());
    let name = match graph {
        ScopesGraph::Totals => "the totals",
        ScopesGraph::Histogram => "the totals and the histogram",
        ScopesGraph::Waveform => "the totals and the waveform",
        ScopesGraph::Vectorscope => "the totals and the vectorscope",
    };
    println!(
        "With {name} counted after each frame, per exposure change, median of {}: {wall:.2} ms submit to done (min {wall_min:.2}), {:+.2} ms; counting took {cpu:.3} ms to submit{on_gpu}",
        without.len(),
        wall - plain,
    );
    let replaced = without.len() - readings.len();
    println!(
        "Read back: {} of {} frames' readings{}; each picked up {late:.2} ms after its count was submitted (min {late_min:.2}), polled for between changes; {idle_late:.2} ms (min {idle_min:.2}) with nothing else to do, median of {}",
        readings.len(),
        without.len(),
        if replaced == 0 {
            String::new()
        } else {
            format!(
                ", {replaced} replaced by a later frame's while both staging buffers were mapping"
            )
        },
        idle.len()
    );
    let last = readings
        .last()
        .context("the last frame's reading arrives")?;
    let share = |count: u32| 100.0 * f64::from(count) / f64::from(last.totals.pixels.max(1));
    println!(
        "The last frame, {}×{}: {} pixels counted, {:.2}% with a channel at or below 0, {:.2}% with one at or above 1",
        last.width,
        last.height,
        last.totals.pixels,
        share(last.totals.shadows),
        share(last.totals.highlights)
    );
    Ok(pipeline)
}

/// Times operation `id` joining the plan, leaving it and joining again, each a rebuild
/// that keeps the compiled kernels and the cache points that still hold (decision 28),
/// then the joined plan built from nothing, as every rebuild was before, and checks that
/// the two give the same frame bit for bit.
async fn time_rebuilds(
    gpu: &GpuContext,
    mut pipeline: Pipeline,
    input: GpuImage,
    id: &str,
    base: pipeline::Plan,
    joined: pipeline::Plan,
) -> Result<()> {
    let ms = |since: Instant| since.elapsed().as_secs_f64() * 1e3;
    let mut kept = None;
    for (what, plan) in [
        ("joins", &joined),
        ("leaves", &base),
        ("joins again", &joined),
    ] {
        let kernels = pipeline.kernels();
        let start = Instant::now();
        pipeline = pipeline.update(gpu, plan.clone()).await?;
        let rebuilt_ms = ms(start);
        let timings = pipeline.render(gpu)?;
        let compiled = pipeline.kernels() - kernels;
        println!("{id} {what}: {}", rebuild(rebuilt_ms, compiled, &timings));
        if kept.is_none() {
            kept = Some(pipeline.output().download(gpu)?);
        }
    }

    // Every buffer freed and every kernel compiled again.
    drop(pipeline);
    let start = Instant::now();
    let mut fresh = Pipeline::new(gpu, joined, input).await?;
    let built_ms = ms(start);
    let timings = fresh.render(gpu)?;
    println!(
        "From nothing, as every rebuild was before: {}",
        rebuild(built_ms, fresh.kernels(), &timings)
    );
    let fresh = fresh.output().download(gpu)?;
    let kept = kept.expect("it joined");
    let same = kept.len() == fresh.len()
        && kept
            .iter()
            .zip(&fresh)
            .all(|(a, b)| a.map(f32::to_bits) == b.map(f32::to_bits));
    if !same {
        bail!("the frame after {id} joined differs from the one built from nothing");
    }
    println!("The frame after {id} joined equals the one built from nothing, bit for bit.");
    Ok(())
}

/// A rebuild's time, the kernels it compiled, and its first render's: where it began,
/// from submission to the GPU finishing, and on the GPU.
fn rebuild(rebuilt_ms: f64, compiled: usize, timings: &Timings) -> String {
    let gpu_ms: f64 = timings.passes.iter().map(|(_, ms)| ms).sum();
    let from = timings.passes.first().map_or("nothing", |(name, _)| name);
    format!(
        "rebuilt in {rebuilt_ms:.1} ms, compiling {compiled} kernel{}; its first render, from {from}, {:.1} ms submit to done{}; {:.1} ms in all",
        if compiled == 1 { "" } else { "s" },
        timings.wall_ms,
        if gpu_ms.is_nan() {
            String::new()
        } else {
            format!(" ({gpu_ms:.1} ms on the GPU)")
        },
        rebuilt_ms + timings.wall_ms
    )
}

/// Each pass's GPU time, those of the same name added up, as a viewport's tiles run
/// theirs once per tile.
fn gpu_times(timings: &Timings) -> String {
    let mut passes: Vec<(&str, f64, usize)> = Vec::new();
    for &(name, ms) in &timings.passes {
        match passes.iter_mut().find(|(seen, ..)| *seen == name) {
            Some((_, total, count)) => {
                *total += ms;
                *count += 1;
            }
            None => passes.push((name, ms, 1)),
        }
    }
    passes
        .iter()
        .map(|&(name, ms, count)| match count {
            1 => format!("{name} {ms:.2} ms GPU"),
            _ => format!("{name} {ms:.2} ms GPU in {count}"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn show_info(path: &Path) -> Result<()> {
    let start = Instant::now();
    let raw = apertrust_raw::decode_file(path)?;
    let decoded = start.elapsed();

    let camera = &raw.camera;
    if camera.alias == camera.model {
        println!("Camera: {} {}", camera.make, camera.model);
    } else {
        println!(
            "Camera: {} {}, an alias of the {}",
            camera.make, camera.alias, camera.model
        );
    }
    println!(
        "Size: {}×{}, {} value(s) per pixel, orientation {:?}",
        raw.width, raw.height, raw.components, raw.orientation
    );
    let rect = |r: Rect| format!("{}×{} at ({}, {})", r.width, r.height, r.x, r.y);
    println!(
        "Active area: {}; default crop: {}; {} masked area(s)",
        rect(raw.active_area),
        raw.default_crop.map_or("none".to_owned(), rect),
        raw.masked_areas.len()
    );
    match &raw.cfa {
        Some(cfa) => {
            let rows: Vec<String> = cfa
                .colors
                .chunks(cfa.width as usize)
                .map(|row| row.iter().map(|&c| cfa_letter(c)).collect())
                .collect();
            println!("Mosaic: {}×{}, {}", cfa.width, cfa.height, rows.join(" "));
        }
        None => println!("Mosaic: none"),
    }
    let black = &raw.black_level;
    println!(
        "Levels: black {}×{} {:?}, white {:?}",
        black.width, black.height, black.values, raw.white_levels
    );
    if let Some(masked) = raw.masked_black() {
        println!(
            "Black measured from the masked areas, as darktable measures it: {:?}",
            masked.values
        );
    }
    match &raw.white_balance {
        Some(wb) => println!("White balance as shot: {wb:?}"),
        None => println!("White balance as shot: none"),
    }
    match raw.iso {
        Some(iso) => println!("ISO: {iso}"),
        None => println!("ISO: none recorded"),
    }
    match raw.exposure_bias {
        Some(ev) => println!("Exposure bias: {ev:+.2} EV"),
        None => println!("Exposure bias: none recorded"),
    }
    if let Some(limit) = raw.linear_response_limit {
        println!("Linear response limit: {limit}");
    }
    for matrix in &raw.color_matrices {
        println!(
            "XYZ to camera ({:?}): {:?}",
            matrix.illuminant, matrix.xyz_to_camera
        );
    }
    let (min, max) = match &raw.pixels {
        Pixels::U16(values) => {
            let (min, max) = values
                .iter()
                .fold((u16::MAX, 0), |(lo, hi), &v| (lo.min(v), hi.max(v)));
            (f64::from(min), f64::from(max))
        }
        Pixels::F32(values) => values.iter().fold((f64::MAX, f64::MIN), |(lo, hi), &v| {
            (lo.min(f64::from(v)), hi.max(f64::from(v)))
        }),
    };
    println!("Values: {min} to {max}");
    println!("Read and decoded in {:.0} ms", decoded.as_secs_f64() * 1e3);
    Ok(())
}

fn cfa_letter(color: CfaColor) -> char {
    match color {
        CfaColor::Red => 'R',
        CfaColor::Green => 'G',
        CfaColor::Blue => 'B',
        CfaColor::Cyan => 'C',
        CfaColor::Magenta => 'M',
        CfaColor::Yellow => 'Y',
        CfaColor::White => 'W',
        CfaColor::FujiGreen => 'g',
        CfaColor::Unknown => '?',
    }
}

async fn show_gpu(engine: engine::Options) -> Result<()> {
    println!("Adapters:");
    for adapter in gpu::adapters(&gpu::instance()).await {
        let info = adapter.get_info();
        println!(
            "  {} ({:?}, {:?})",
            info.name, info.backend, info.device_type
        );
    }

    let gpu = GpuContext::new(&engine.gpu).await?;
    let info = gpu.adapter.get_info();
    let limits = gpu.device.limits();
    const MIB: u64 = 1 << 20;
    println!();
    println!(
        "Engine device: {} ({:?}, {:?})",
        info.name, info.backend, info.device_type
    );
    if !info.driver.is_empty() {
        println!("  Driver: {} {}", info.driver, info.driver_info);
    }
    println!("  Max buffer: {} MiB", limits.max_buffer_size / MIB);
    println!(
        "  Max storage buffer binding: {} MiB",
        limits.max_storage_buffer_binding_size / MIB
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use apertrust_engine::edit::EditError;
    use apertrust_export::jpeg;
    use apertrust_modules::testing::Sample;
    use apertrust_plugin::Value;

    use super::*;

    const WITH_SAMPLE: &[&dyn Module] = &[&Sample];

    #[test]
    fn a_full_disk_stops_a_run_whether_the_rename_or_the_encoder_found_it() {
        let full = || std::io::Error::from(std::io::ErrorKind::StorageFull);
        let renamed = anyhow::Error::new(DiskError::Write {
            path: "a.jpg".into(),
            source: full(),
        });
        let encoded =
            anyhow::Error::new(apertrust_export::Error::Io(full())).context("cannot write a.jpg");
        assert!(destination_failed(&renamed));
        assert!(destination_failed(&encoded));
        // A photo's own error does not.
        let bad = anyhow::Error::new(apertrust_export::Error::Quality(1)).context("cannot write");
        assert!(!destination_failed(&bad));
        assert!(!destination_failed(&anyhow!("any other")));
    }

    fn flags(sets: &[&str], params: &[&str]) -> SetFlags {
        SetFlags {
            sets: sets.iter().map(|s| (*s).to_owned()).collect(),
            params: params.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    /// The edit error under the CLI's context, if that is what failed.
    fn refusal(result: Result<()>) -> String {
        format!("{:#}", result.unwrap_err())
    }

    /// Applies the flags as `edit` and `export` do, on `image`.
    fn apply_on(
        edit: &mut Edit,
        flags: &SetFlags,
        operations: &[&dyn Operation],
        modules: &[&dyn Module],
        image: &ImageContext,
    ) -> Result<()> {
        let sets = read_sets(&flags.sets, modules)?;
        set_controls(edit, &sets, image)?;
        set_params(edit, &flags.params, operations, modules)
    }

    /// Applies the flags on Sample's photo, `testing::image`, which records no exposure
    /// bias.
    fn apply(
        edit: &mut Edit,
        flags: &SetFlags,
        operations: &[&dyn Operation],
        modules: &[&dyn Module],
    ) -> Result<()> {
        let image = apertrust_modules::testing::image();
        apply_on(edit, flags, operations, modules, &image)
    }

    /// `--set` refuses a choice's option the photo does not list, after reading every
    /// flag, and takes it on a photo that lists it (decision 30): Sample's method lists
    /// camera only on a photo that records an exposure bias.
    #[test]
    fn set_refuses_an_option_the_photo_does_not_list() {
        let camera = flags(&["sample.method=camera"], &[]);
        let mut edit = Edit::default();
        let said = refusal(apply(&mut edit, &camera, OPERATIONS, WITH_SAMPLE));
        assert!(said.contains("this photo does not list camera"), "{said}");
        assert!(edit.is_empty());
        // A flag that cannot be read is refused before any photo, here none.
        let unread = flags(&["sample.method=sepia"], &[]);
        assert!(read_sets(&unread.sets, WITH_SAMPLE).is_err());
        assert!(read_sets(&camera.sets, WITH_SAMPLE).is_ok_and(|s| s[0].is_choice()));
        let mut biased = apertrust_modules::testing::image();
        if let Some(camera) = &mut biased.camera {
            camera.exposure_bias = -0.7;
        }
        apply_on(&mut edit, &camera, OPERATIONS, WITH_SAMPLE, &biased).unwrap();
        assert_eq!(
            edit.module("sample").unwrap().get("method"),
            Some(&ControlValue::Choice(3))
        );
    }

    #[test]
    fn set_and_reset_take_module_controls() {
        let mut edit = Edit::default();
        apply(
            &mut edit,
            &flags(&["sample.brightness=0.5", "sample.size=coarse"], &[]),
            OPERATIONS,
            WITH_SAMPLE,
        )
        .unwrap();
        let values = edit.module("sample").unwrap();
        assert_eq!(values.number("brightness"), Some(0.5));
        assert_eq!(values.get("size"), Some(&ControlValue::Choice(2)));
        assert_eq!(values.mark("brightness"), Some(Mark::Kept));
        assert_eq!(
            describe(&edit, OPERATIONS, WITH_SAMPLE),
            ["sample: brightness 0.5 (kept), size coarse (kept)"]
        );

        for (set, says) in [
            ("sample.brightness=9", "outside -3 to 4"),
            ("sample.brightness=bright", "cannot read"),
            ("sample.contrast=1", "no control contrast"),
            ("exposure.exposure=1", "no module exposure"),
            ("sample.brightness", "expected MODULE.CONTROL=VALUE"),
            ("brightness=1", "expected MODULE.CONTROL"),
        ] {
            let said = refusal(apply(
                &mut edit,
                &flags(&[set], &[]),
                OPERATIONS,
                WITH_SAMPLE,
            ));
            assert!(said.contains(says), "--set {set}: {said}");
        }
        reset_control(
            &mut edit,
            WITH_SAMPLE,
            "sample.brightness",
            &ImageContext::full(0, 0),
        )
        .unwrap();
        reset_control(
            &mut edit,
            WITH_SAMPLE,
            "sample.size",
            &ImageContext::full(0, 0),
        )
        .unwrap();
        assert!(edit.is_empty());
        assert!(
            reset_control(
                &mut edit,
                WITH_SAMPLE,
                "sample.contrast",
                &ImageContext::full(0, 0)
            )
            .is_err()
        );
    }

    /// `--set` and `--reset` bring back inside its range a number their move of a range
    /// leaves outside, in the same change, kept (decision 30): AgX's pivot reaches from
    /// black to white relative exposure.
    #[test]
    fn set_and_reset_clamp_what_a_moved_range_leaves_outside() {
        use apertrust_modules::agx::PIVOT;
        let pivot = |edit: &Edit| {
            let values = edit.module("agx").unwrap();
            (values.number(PIVOT), values.mark(PIVOT))
        };
        // Black raised past the pivot at −5 EV.
        let mut edit = Edit::default();
        let raised = flags(
            &["agx.curve_pivot_x=-5", "agx.range_black_relative_ev=-4"],
            &[],
        );
        apply(&mut edit, &raised, OPERATIONS, MODULES).unwrap();
        assert_eq!(pivot(&edit), (Some(-4.0), Some(Mark::Kept)));
        // Black reset from −15 EV to its default of −10, past the pivot at −12.
        let mut edit = Edit::default();
        let lowered = flags(
            &["agx.range_black_relative_ev=-15", "agx.curve_pivot_x=-12"],
            &[],
        );
        apply(&mut edit, &lowered, OPERATIONS, MODULES).unwrap();
        edit.keep(edit::module(MODULES, "agx").unwrap(), PIVOT, false)
            .unwrap();
        let image = ImageContext::full(0, 0);
        reset_control(&mut edit, MODULES, "agx.range_black_relative_ev", &image).unwrap();
        assert_eq!(pivot(&edit), (Some(-10.0), Some(Mark::Kept)));
    }

    #[test]
    fn param_takes_only_unowned_parameters_of_operations_an_unedited_photo_runs() {
        let mut edit = Edit::default();
        apply(
            &mut edit,
            &flags(&[], &["exposure.black=0.01", "agx.base_primaries=srgb"]),
            OPERATIONS,
            WITH_SAMPLE,
        )
        .unwrap();
        assert_eq!(
            edit.params("exposure").unwrap().get("black"),
            Some(Value::Float(0.01))
        );
        for (set, refused) in [
            (
                "exposure.exposure=1",
                EditError::Owned {
                    operation: "exposure",
                    param: "exposure",
                    module: "sample",
                },
            ),
            (
                "grain.strength=10",
                EditError::Optional { operation: "grain" },
            ),
        ] {
            let error = apply(&mut edit, &flags(&[], &[set]), OPERATIONS, WITH_SAMPLE).unwrap_err();
            assert_eq!(
                error.downcast_ref::<EditError>(),
                Some(&refused),
                "--param {set}"
            );
        }
        let said = refusal(apply(
            &mut edit,
            &flags(&[], &["exposure.black=2"]),
            OPERATIONS,
            WITH_SAMPLE,
        ));
        assert!(said.contains("outside -1 to 1"), "{said}");
        // The built-in Exposure module owns every exposure parameter, so its control sets
        // them instead.
        let error = apply(
            &mut edit,
            &flags(&[], &["exposure.exposure=1"]),
            OPERATIONS,
            MODULES,
        )
        .unwrap_err();
        assert_eq!(
            error.downcast_ref::<EditError>(),
            Some(&EditError::Owned {
                operation: "exposure",
                param: "exposure",
                module: "exposure",
            })
        );
        apply(
            &mut edit,
            &flags(&["exposure.exposure=1"], &[]),
            OPERATIONS,
            MODULES,
        )
        .unwrap();
        assert_eq!(
            edit.module("exposure").unwrap().number("exposure"),
            Some(1.0)
        );
        reset_control(
            &mut edit,
            MODULES,
            "exposure.exposure",
            &ImageContext::full(0, 0),
        )
        .unwrap();
        reset_param(&mut edit, OPERATIONS, WITH_SAMPLE, "exposure.black").unwrap();
        assert!(reset_param(&mut edit, OPERATIONS, WITH_SAMPLE, "grain.strength").is_err());
        assert_eq!(
            describe(&edit, OPERATIONS, WITH_SAMPLE),
            ["agx: base_primaries srgb"]
        );
    }

    #[test]
    fn the_edit_command_saves_what_its_flags_set_beside_the_photo() {
        let directory =
            std::env::temp_dir().join(format!("apertrust-cli-edit-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let photo = directory.join("IMG.ARW");
        std::fs::write(&photo, b"not read").unwrap();
        let run = |sets: &[&str], params: &[&str], resets: &[&str], clear| {
            let resets: Vec<String> = resets.iter().map(|s| (*s).to_owned()).collect();
            edit(
                &photo,
                EditArgs {
                    flags: &flags(sets, params),
                    resets: &resets,
                    param_resets: &[],
                    clear,
                },
                OPERATIONS,
                WITH_SAMPLE,
            )
        };
        run(&["sample.grain=40"], &["exposure.black=0.01"], &[], false).unwrap();
        // A choice's options are the photo's (decision 30), so setting one decodes the
        // raw, which this file is not: refused, the saved edit as it was.
        let refused = run(&["sample.size=fine"], &[], &[], false).unwrap_err();
        assert!(
            format!("{refused:#}").contains("for its options"),
            "{refused:#}"
        );
        run(&["sample.brightness=0.5"], &[], &[], false).unwrap();
        let (saved, newer) = saved_edit(&photo, OPERATIONS, WITH_SAMPLE).unwrap();
        assert!(!newer);
        let values = saved.module("sample").unwrap();
        assert_eq!(values.number("grain"), Some(40.0));
        assert_eq!(values.number("brightness"), Some(0.5));
        assert_eq!(values.choice("size"), None);
        assert_eq!(
            saved.params("exposure").unwrap().get("black"),
            Some(Value::Float(0.01))
        );
        let xml = std::fs::read_to_string(sidecar::path_for(&photo)).unwrap();
        assert!(xml.contains(r#"apertrust:version="1""#), "{xml}");
        // The build that wrote it: this one's version (decision 44).
        let build = format!(
            r#"x:xmptk="Apertrust {}""#,
            apertrust_base::program!().version
        );
        assert!(xml.contains(&build), "{xml}");

        // The instances an export renders from it: grain joins the default pipeline.
        let ids: Vec<&str> = edit::instances(
            &saved,
            OPERATIONS,
            WITH_SAMPLE,
            &apertrust_modules::testing::image(),
        )
        .all()
        .unwrap()
        .iter()
        .map(|instance| instance.operation.manifest().id)
        .collect();
        assert!(ids.contains(&"grain"), "{ids:?}");

        run(&[], &[], &["sample.grain"], false).unwrap();
        assert!(
            saved_edit(&photo, OPERATIONS, WITH_SAMPLE)
                .unwrap()
                .0
                .module("sample")
                .unwrap()
                .get("grain")
                .is_none()
        );
        run(&[], &[], &[], true).unwrap();
        assert!(
            saved_edit(&photo, OPERATIONS, WITH_SAMPLE)
                .unwrap()
                .0
                .is_empty()
        );
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn the_edit_command_refuses_a_newer_apertrust_s_edit_and_leaves_it_as_it_is() {
        let directory =
            std::env::temp_dir().join(format!("apertrust-cli-newer-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let photo = directory.join("IMG.ARW");
        std::fs::write(&photo, b"not read").unwrap();
        // Sample's grain, and a module of a newer Apertrust's that this build lacks.
        let mut grainy = Edit::default();
        grainy
            .set_control(
                &Sample,
                "grain",
                ControlValue::Number(40.0),
                &apertrust_modules::testing::image(),
            )
            .unwrap();
        let xml = sidecar::to_xml(&grainy, OPERATIONS, WITH_SAMPLE, "0.9.0").replacen(
            "<rdf:Seq>\n",
            "<rdf:Seq>\n<rdf:li><rdf:Description apertrust:module=\"lens\" apertrust:moduleVersion=\"1\"/></rdf:li>\n",
            1,
        );
        let path = sidecar::path_for(&photo);
        std::fs::write(&path, &xml).unwrap();
        for (sets, clear) in [(&["sample.grain=10"][..], false), (&[], true), (&[], false)] {
            let refused = edit(
                &photo,
                EditArgs {
                    flags: &flags(sets, &[]),
                    resets: &[],
                    param_resets: &[],
                    clear,
                },
                OPERATIONS,
                WITH_SAMPLE,
            )
            .unwrap_err();
            let view_only = refused
                .downcast_ref::<sidecar::ViewOnly>()
                .expect("refused as a newer Apertrust's");
            assert_eq!(view_only.build.as_deref(), Some("0.9.0"));
            assert!(
                format!("{refused:#}").contains("Apertrust 0.9.0"),
                "{refused:#}"
            );
            assert_eq!(std::fs::read_to_string(&path).unwrap(), xml);
        }
        // Export and preview render what this build reads of it, as it was: no read clamp.
        let (saved, newer) = saved_edit(&photo, OPERATIONS, WITH_SAMPLE).unwrap();
        assert_eq!(saved, grainy);
        assert!(newer);
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn output_extension_chooses_the_format_as_darktable_cli_does() {
        let defaults = Store::nowhere(&registries());
        let format =
            |output: &str| format_of(Path::new(output), &defaults).map_err(|e| e.to_string());
        let jpeg = Ok(Format::Jpeg(jpeg::Options::default()));
        assert_eq!(format("out.jpg"), jpeg);
        assert_eq!(format("folder.d/out.jpeg"), jpeg);
        assert_eq!(format("out.png"), Ok(Format::Png(png::Options::default())));
        // darktable-cli compares the extension as it is, so an upper-case one is unknown.
        for output in ["out.JPG", "out.tif", "out.webp"] {
            assert!(format(output).unwrap_err().starts_with("unknown extension"));
        }
        for output in ["out", "out.", "folder.d/out"] {
            assert!(
                format(output)
                    .unwrap_err()
                    .starts_with("no output file extension")
            );
        }
    }

    /// `-o` is a pattern over export.disk.path, as darktable-cli reads it: its extension
    /// is the format and no part of the name, a folder that exists is `FOLDER/$(FILE_NAME)`
    /// and an `-o` left out is the setting's.
    #[test]
    fn output_is_the_pattern_over_export_disk_path() {
        let folder =
            std::env::temp_dir().join(format!("apertrust-cli-dest-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a folder");
        let conf = ["export.disk.path=~/shots/$(FILE_NAME)", "export.format=png"];
        let overrides = Overrides::read(conf, &registries()).expect("read");
        let store = Store::nowhere(&registries()).with_overrides(&overrides);
        let jpeg = Format::Jpeg(jpeg::Options::default());
        let png = Format::Png(png::Options::default());
        let destination = |output: Option<&Path>| destination(output, &store).expect("a pattern");

        assert_eq!(
            destination(Some(Path::new("$(FILE_NAME)_x.jpg"))),
            ("$(FILE_NAME)_x".to_owned(), jpeg)
        );
        assert_eq!(
            destination(Some(Path::new("out/a.b.png"))),
            ("out/a.b".to_owned(), png)
        );
        // Left out: the setting's pattern and format.
        assert_eq!(destination(None), ("~/shots/$(FILE_NAME)".to_owned(), png));
        // A folder that exists: its files by their names, in export.format's format.
        let text = folder.to_string_lossy().replace('\\', "/");
        let expected = (format!("{text}/$(FILE_NAME)"), png);
        assert_eq!(destination(Some(&folder)), expected);
        assert_eq!(destination(Some(&folder.join(""))), expected);
        std::fs::remove_dir_all(&folder).expect("removed");
        // A name with no extension that is no folder is an error, as darktable-cli's.
        assert!(
            super::destination(Some(&folder), &store)
                .unwrap_err()
                .to_string()
                .starts_with("no output file extension")
        );
    }

    /// Several files, and `-o` left out, parse.
    #[test]
    fn export_takes_several_files_and_an_optional_output() {
        let parsed = |args: &[&str]| {
            let args = [&["apertrust-cli", "export"], args].concat();
            match Cli::try_parse_from(args).map(|cli| cli.command) {
                Ok(Command::Export { files, output, .. }) => Ok((files.len(), output)),
                Ok(other) => panic!("{other:?}"),
                Err(error) => Err(error.kind()),
            }
        };
        assert_eq!(
            parsed(&["a.ARW", "b.ARW", "-o", "out/$(FILE_NAME).jpg"]),
            Ok((2, Some(PathBuf::from("out/$(FILE_NAME).jpg"))))
        );
        assert_eq!(parsed(&["a.ARW"]), Ok((1, None)));
        assert!(parsed(&["-o", "out.jpg"]).is_err());
    }

    /// The format's options are the export settings', and `-o`'s extension comes over
    /// `export.format` (decision 56); the settings file's are `tests/settings.rs`'.
    #[test]
    fn the_format_s_options_are_the_settings_and_its_kind_the_extension_s() {
        let conf = [
            "export.format=png",
            "export.jpeg.quality=80",
            "export.png.compression=9",
        ];
        let overrides = Overrides::read(conf, &registries()).expect("read");
        let store = Store::nowhere(&registries()).with_overrides(&overrides);
        let jpeg = jpeg::Options {
            quality: 80,
            ..jpeg::Options::default()
        };
        let format = |output: &str| format_of(Path::new(output), &store).expect("a format");
        assert_eq!(format("out.jpg"), Format::Jpeg(jpeg));
        let png = png::Options {
            compression: 9,
            ..png::Options::default()
        };
        assert_eq!(format("out.png"), Format::Png(png));
    }

    #[test]
    fn scopes_takes_its_graph_only_after_an_equals_sign() {
        let scopes = |args: &[&str]| match Cli::try_parse_from(args).map(|cli| cli.command) {
            Ok(Command::Preview { file, scopes, .. }) => {
                Ok((file, scopes.map(ScopesGraph::graphs)))
            }
            Ok(other) => panic!("{other:?}"),
            Err(error) => Err(error.kind()),
        };
        let histogram = ScopesGraph::Histogram.graphs();
        // Bare, before the file: the histogram, and the file stays the file.
        assert_eq!(
            scopes(&["apertrust-cli", "preview", "--scopes", "photo.CR2"]),
            Ok((PathBuf::from("photo.CR2"), Some(histogram)))
        );
        assert_eq!(
            scopes(&["apertrust-cli", "preview", "photo.CR2", "--scopes=waveform"]),
            Ok((
                PathBuf::from("photo.CR2"),
                Some(ScopesGraph::Waveform.graphs())
            ))
        );
        assert_eq!(
            scopes(&["apertrust-cli", "preview", "photo.CR2"]),
            Ok((PathBuf::from("photo.CR2"), None))
        );
        assert_eq!(
            scopes(&[
                "apertrust-cli",
                "preview",
                "photo.CR2",
                "--scopes=vectorscope"
            ]),
            Ok((
                PathBuf::from("photo.CR2"),
                Some(ScopesGraph::Vectorscope.graphs())
            ))
        );
        assert!(scopes(&["apertrust-cli", "preview", "photo.CR2", "--scopes=all"]).is_err());
    }

    /// darktable-cli's size flags and their defaults: no limit, high quality resampling,
    /// no upscaling; `--hq` and `--upscale` take 0, 1, false or true in any case.
    #[test]
    fn export_takes_darktable_cli_s_size_flags() {
        let size = |args: &[&str]| {
            let args = [
                &["apertrust-cli", "export", "photo.CR2", "-o", "out.jpg"],
                args,
            ]
            .concat();
            match Cli::try_parse_from(args).map(|cli| cli.command) {
                Ok(Command::Export {
                    width,
                    height,
                    hq,
                    upscale,
                    ..
                }) => Ok((width, height, hq, upscale)),
                Ok(other) => panic!("{other:?}"),
                Err(error) => Err(error.kind()),
            }
        };
        assert_eq!(size(&[]), Ok((0, 0, true, false)));
        assert_eq!(
            size(&["--width", "2048", "--height", "1024", "--hq", "FALSE"]),
            Ok((2048, 1024, false, false))
        );
        assert_eq!(
            size(&["--hq", "0", "--upscale", "True"]),
            Ok((0, 0, false, true))
        );
        assert_eq!(size(&["--upscale", "1"]), Ok((0, 0, true, true)));
        assert!(size(&["--hq", "yes"]).is_err());
        assert!(size(&["--width", "-5"]).is_err());
    }

    /// `--user-data-dir` and its aliases, apart or joined to the folder, before the command
    /// or after it, the last given counting and named; a leading `~` is left to the folders,
    /// which take it as the home folder in both forms.
    #[test]
    fn the_user_data_folder_is_named_by_any_of_its_flags_the_last_counting() {
        let parsed = |args: &[&str]| {
            let cli = Cli::try_parse_from(args).expect("parsed");
            let flag = user_data_flag(args.iter().map(OsString::from));
            (cli.user_data_dir, flag)
        };
        for flag in folders::FLAGS {
            let joined = format!("{flag}=~/dev");
            for args in [
                ["apertrust-cli", flag, "~/dev", "gpu"].as_slice(),
                &["apertrust-cli", "gpu", &joined],
            ] {
                assert_eq!(
                    parsed(args),
                    (Some(PathBuf::from("~/dev")), flag),
                    "{args:?}"
                );
            }
        }
        let last = [
            "apertrust-cli",
            "--configdir",
            "one",
            "gpu",
            "--user-data-dir=two",
        ];
        assert_eq!(parsed(&last), (Some("two".into()), "--user-data-dir"));
        let last = [
            "apertrust-cli",
            "--user-data-dir",
            "one",
            "--config-dir=two",
            "gpu",
        ];
        assert_eq!(parsed(&last), (Some("two".into()), "--config-dir"));
        // Past `--`, an argument names a file, never a flag.
        let past = [
            "apertrust-cli",
            "--configdir",
            "one",
            "info",
            "--",
            "--config-dir",
        ];
        assert_eq!(parsed(&past), (Some("one".into()), "--configdir"));
        // A flag without its folder is refused, so the platform's is never used for it.
        for args in [
            ["apertrust-cli", "gpu", "--user-data-dir"].as_slice(),
            &["apertrust-cli", "--configdir=", "gpu"],
        ] {
            assert!(Cli::try_parse_from(args).is_err(), "{args:?}");
        }
        // None given: the variable's, then the platform's (`Folders::find`).
        assert_eq!(parsed(&["apertrust-cli", "gpu"]).0, None);
    }

    /// `--cache-dir`, apart or joined, before the command or after it, the last counting;
    /// one without its folder is refused.
    #[test]
    fn the_cache_folder_is_named_by_its_flag_the_last_counting() {
        let cache = |args: &[&str]| Cli::try_parse_from(args).expect("parsed").cache_dir;
        let args = [
            "apertrust-cli",
            "--cache-dir",
            "one",
            "gpu",
            "--cache-dir=two",
        ];
        assert_eq!(cache(&args), Some(PathBuf::from("two")));
        assert_eq!(cache(&["apertrust-cli", "gpu"]), None);
        for args in [
            ["apertrust-cli", "gpu", "--cache-dir"].as_slice(),
            &["apertrust-cli", "--cache-dir=", "gpu"],
        ] {
            assert!(Cli::try_parse_from(args).is_err(), "{args:?}");
        }
    }
}
