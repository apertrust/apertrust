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

//! The session's export job (decision 56; `docs/engine.md`, Export jobs): darktable's
//! `dt_control_export`, a `UserExport` job of the session's control ([`crate::jobs`],
//! decision 57), so one runs at a time and a later one waits, queued.
//!
//! - **Each photo's edit as it is when Export is pressed** (D12): the open photo's from the
//!   session's memory as the command is handled, every other's read from its sidecar by
//!   the session's edit writer, in order with the writes it was handed, with each
//!   sidecar's modification time as the photo's change time for Overwrite if changed
//!   (`Snapshot`).
//! - **Each photo in turn, on the job's worker:** decoded and uploaded (the open photo's
//!   uploaded raw serves its own export); named, its folder and temporary file made
//!   through `apertrust_export::disk` before it renders; planned, its kernels compiled and
//!   its buffers made, always in tiles of at most [`LARGEST_TILE`], each render handed
//!   to the engine thread as a GPU task it steps after the live frame, one submission,
//!   one step, each tile read back through a mapping the thread's polls answer, which
//!   the worker waits for ([`crate::export`]'s handing export), and the operations
//!   before the tiles (the raw stage on a default edit), beside a live frame, a step per
//!   pass, in row strips of [`STRIP_BINDING`] a step per strip, each new buffer touched
//!   first, and with no live frame, the CLI's session's, one step; then
//!   quantised, encoded and written into its temporary file, and renamed into place, by
//!   the worker itself, as darktable's store runs on its worker.
//! - **What the client hears:** the job's progress as `Event::Job`, "export images" with a
//!   message per photo and a fraction per photo done; each photo's start and result, and
//!   the job's counts, as `Event::Export` ([`ExportStatus`]).
//! - **Failures:** a photo's own fails that photo, and the job goes on; a failure of the
//!   destination, or the engine stopping under it ([`Failure::ends_job`]), ends the job
//!   failed, the photos after it not reached. A cancel takes effect before the job's next
//!   submission: the photo under way is dropped with its temporary file, and the files
//!   written stay.

use std::fmt::Display;
use std::io::{self, BufWriter, Write};
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::time::{Instant, SystemTime};

pub use apertrust_export::disk::Why;
use apertrust_export::disk::{self, Disk, Prepared, Skipped, Stored, Target};
use apertrust_export::size::{self, Request};
use apertrust_export::variables::{DateTime, Size};
use apertrust_export::{ExportSettings, Format, Image, Places};
use apertrust_modules::Module;
use apertrust_plugin::{ImageContext, ImageDesc, Operation};
use thiserror::Error;

use crate::compute::GpuImage;
use crate::edit::{self, Edit, panic_message};
use crate::export::{
    ExportError, Hand, MapResult, Mapping, Rendered, Submit, Tiling, export_handing,
};
use crate::gpu::GpuContext;
use crate::jobs::{self, Context, Interrupted, Job, Message, Outcome, TaskError, Tasks};
use crate::names::{self, Shot, Sizes};
use crate::pipeline::{self, Instance, Limits, Pipe, PlanError, Resample};
use crate::session::{Decoded, Event, Load, Outbox};
use crate::sidecar;

/// What the session reports of an export job's photos, as `Event::Export`, in order: each
/// photo's start and result, then the job's counts. The job itself, queued, running, its
/// fraction, message and end, comes as `Event::Job`.
#[derive(Debug, Clone, PartialEq)]
pub enum ExportStatus {
    /// Photo `index` of the job's photos, the raw at `source`, begins.
    Started { index: usize, source: PathBuf },
    /// Photo `index` ended so.
    Done { index: usize, result: PhotoResult },
    /// The job ended: what became of its photos.
    Finished(Summary),
}

/// What became of one photo.
#[derive(Debug, Clone, PartialEq)]
pub enum PhotoResult {
    /// Its file, renamed into place: its size, in how many tiles it rendered, and whether
    /// a newer Apertrust wrote its edit, exported as this build shows it (decision 43).
    Written {
        path: PathBuf,
        width: u32,
        height: u32,
        tiles: usize,
        newer: bool,
    },
    /// Left out as On conflict says, before it rendered or as its file was renamed.
    Skipped {
        path: PathBuf,
        why: Why,
    },
    Failed(Failure),
}

/// What became of a job's photos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Summary {
    pub written: usize,
    pub skipped: usize,
    pub failed: usize,
    /// The photos it never reached: after a failure of the destination or a cancel, the
    /// photo a cancel interrupted among them, or all of them where it was refused.
    pub not_reached: usize,
    pub cancelled: bool,
}

impl Summary {
    fn count(&mut self, result: &PhotoResult) {
        match result {
            PhotoResult::Written { .. } => self.written += 1,
            PhotoResult::Skipped { .. } => self.skipped += 1,
            PhotoResult::Failed(_) => self.failed += 1,
        }
    }
}

/// Why a photo was not written, or a job ended failed.
#[derive(Debug, Clone, PartialEq)]
pub struct Failure {
    /// What failed, as a code for a client to word ([`Cause::id`]).
    pub cause: Cause,
    /// The file it was writing, where one was named.
    pub file: Option<PathBuf>,
    /// The error, in plain words, for the log and a client without words of its own.
    pub message: String,
}

impl Failure {
    fn new(cause: Cause, file: Option<PathBuf>, error: impl Display) -> Self {
        Self {
            cause,
            file,
            message: error.to_string(),
        }
    }

    /// File on disk's error, the destination's where [`disk::Error::destination`] says so.
    fn disk(error: &disk::Error, file: Option<PathBuf>) -> Self {
        let cause = match error {
            disk::Error::Empty | disk::Error::Pattern(_) => Cause::Pattern,
            disk::Error::NoName(_) => Cause::Name,
            disk::Error::CreateFolder { source, .. } | disk::Error::WriteFolder { source, .. } => {
                Cause::of_kind(source.kind()).unwrap_or(Cause::Folder)
            }
            disk::Error::Write { source, .. } => {
                Cause::of_kind(source.kind()).unwrap_or(Cause::Write)
            }
        };
        Self::new(cause, file, error)
    }

    /// The encoder's error writing `file`: the destination's where
    /// [`apertrust_export::Error::destination`] says so.
    fn encoder(error: &apertrust_export::Error, file: &Path) -> Self {
        let cause = error
            .io_kind()
            .and_then(Cause::of_kind)
            .unwrap_or(Cause::Encode);
        let message = format!("cannot write {}: {error}", file.display());
        Self::new(cause, Some(file.to_owned()), message)
    }

    /// Whether the destination is at fault, a folder that cannot be made or written or a
    /// disk full, read-only or gone, which ends the job: [`disk::Error::destination`]'s
    /// rule, and [`apertrust_export::Error::destination`]'s for the file's own writes.
    pub fn destination(&self) -> bool {
        self.cause.destination()
    }

    /// Whether it ends the job, the photos after it not reached: the destination's, or the
    /// engine stopping under it, since every later photo would fail the same way.
    pub fn ends_job(&self) -> bool {
        self.destination() || self.cause == Cause::Engine
    }

    /// The job's failure, as its `Ended` carries it, the photo `photo`'s where it is one.
    fn ended(&self, photo: Option<&Path>) -> Message {
        let mut message = Message::new("job-export-failed")
            .with("cause", self.cause.id())
            .with("reason", self.message.as_str());
        if let Some(photo) = photo {
            message = message.with("photo", photo);
        }
        if let Some(file) = &self.file {
            message = message.with("file", file.as_path());
        }
        message
    }
}

/// What failed, as a code a client words: the activity's cause (`docs/gui.md`, The jobs
/// window).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    /// The raw is missing or cannot be decoded.
    Unreadable,
    /// Its edit cannot be read.
    Edit,
    /// A module's binding fails on its edit: an export renders the edit as it is or not
    /// at all.
    Binding,
    /// The format cannot hold the size asked for, or the size leaves no image (D8).
    Size,
    /// The path, empty or a pattern File on disk refuses (D8): the whole job's.
    Pattern,
    /// The path names no file for this photo.
    Name,
    /// Planning or rendering failed, the GPU's errors included.
    Render,
    /// The encoder failed.
    Encode,
    /// The disk is full or over its quota.
    DiskFull,
    /// The disk or the folder is read-only.
    ReadOnly,
    /// The folder, or the disk it is on, has gone.
    Gone,
    /// The folder cannot be made or written for another reason, such as permissions.
    Folder,
    /// The file could not be written or renamed into place for another reason.
    Write,
    /// A bug in the engine, or the engine stopping.
    Engine,
}

impl Cause {
    /// Its id, for a message's argument and a client's words.
    pub fn id(self) -> &'static str {
        match self {
            Cause::Unreadable => "unreadable",
            Cause::Edit => "edit",
            Cause::Binding => "binding",
            Cause::Size => "size",
            Cause::Pattern => "pattern",
            Cause::Name => "name",
            Cause::Render => "render",
            Cause::Encode => "encode",
            Cause::DiskFull => "disk-full",
            Cause::ReadOnly => "read-only",
            Cause::Gone => "gone",
            Cause::Folder => "folder",
            Cause::Write => "write",
            Cause::Engine => "engine",
        }
    }

    /// The destination's causes: those [`disk::Error::destination`] and
    /// [`apertrust_export::Error::destination`] say are.
    pub fn destination(self) -> bool {
        matches!(
            self,
            Cause::DiskFull | Cause::ReadOnly | Cause::Gone | Cause::Folder
        )
    }

    /// The cause of a write that failed so, where it is the destination's.
    fn of_kind(kind: io::ErrorKind) -> Option<Self> {
        use io::ErrorKind::{NotFound, QuotaExceeded, ReadOnlyFilesystem, StorageFull};
        match kind {
            StorageFull | QuotaExceeded => Some(Cause::DiskFull),
            ReadOnlyFilesystem => Some(Cause::ReadOnly),
            NotFound => Some(Cause::Gone),
            _ => None,
        }
    }
}

/// Why an export's size could not be computed.
#[derive(Debug, Error)]
pub enum SizeError {
    #[error(transparent)]
    Plan(#[from] PlanError),
    #[error(transparent)]
    Size(#[from] apertrust_export::Error),
}

/// The export's resample to the size `request` asks of `format`, as darktable computes it
/// from the photo's extent after its geometry (`apertrust_export::size`, decision 56), and
/// that extent; `None` at full size. `sensor` is the uploaded mosaic's description. A size
/// the format cannot hold stops the photo before it renders.
pub fn size(
    request: &Request,
    format: &Format,
    instances: &[Instance],
    image: &ImageContext,
    sensor: ImageDesc,
    limits: &Limits,
) -> Result<(Option<Resample>, [u32; 2]), SizeError> {
    let extent = pipeline::plan(instances, image, sensor, Pipe::Export, limits)?.extent();
    let size = size::size(request, format, extent)?;
    tracing::debug!(
        target: "pipe",
        "export size: {}×{} to {}×{}, scale {:.6}{}",
        extent[0],
        extent[1],
        size.width,
        size.height,
        size.scale,
        match (size.resamples(), size.finalscale) {
            (false, _) => ", no resample",
            (true, true) => ", resampled at finalscale's place",
            (true, false) => ", resampled after demosaic",
        }
    );
    let resample = size.resamples().then_some(Resample {
        output: [size.width, size.height],
        scale: size.scale as f32,
        finalscale: size.finalscale,
    });
    Ok((resample, extent))
}

/// The export's largest size as the storage is given it, `$(WIDTH.MAX)` and
/// `$(HEIGHT.MAX)`: each side capped by the format's largest, a side left at 0 staying 0.
pub(crate) fn largest_size(settings: &ExportSettings) -> Size {
    let largest = |side: u32| {
        if side == 0 {
            0
        } else {
            side.min(settings.format.largest())
        }
    };
    Size {
        width: largest(settings.size.width),
        height: largest(settings.size.height),
    }
}

/// The open photo as the session holds it when Export is pressed (D12): its edit from
/// memory, its image context and its uploaded raw, which serves its own export.
pub(crate) struct Live {
    /// Its edit's file, by which the job knows it among its photos.
    pub sidecar: PathBuf,
    pub edit: Edit,
    /// Whether a newer Apertrust wrote the edit.
    pub newer: bool,
    pub image: ImageContext,
    pub input: GpuImage,
    pub shot: Shot,
    /// Whether the edit has changed since it was last handed to the writer, so its change
    /// time is the export's moment rather than its file's.
    pub unsaved: bool,
}

/// A photo's sidecar as the edit writer read it for an export (D12), after every write
/// handed to it before: its text, none where there is no file, and its modification time,
/// the photo's change time for Overwrite if changed, none for a photo never edited.
#[derive(Debug)]
pub(crate) struct Snapshot {
    xml: io::Result<Option<String>>,
    changed: Option<SystemTime>,
}

impl Snapshot {
    /// The sidecar at `path` as it is now.
    pub(crate) fn read(path: &Path) -> Self {
        let xml = match std::fs::read_to_string(path) {
            Ok(xml) => Ok(Some(xml)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        };
        let changed = std::fs::metadata(path).and_then(|m| m.modified()).ok();
        Self { xml, changed }
    }
}

/// What an export job runs with: everything it needs of the session, owned, so its body
/// borrows nothing of the engine thread's.
pub(crate) struct Run {
    pub gpu: GpuContext,
    pub operations: &'static [&'static dyn Operation],
    pub modules: Vec<&'static dyn Module>,
    pub retired: &'static [&'static str],
    pub limits: Limits,
    pub load: Load,
    pub photos: Vec<PathBuf>,
    pub settings: ExportSettings,
    /// The person's folders and name, as the client gives them.
    pub places: Places,
    /// When Export was pressed, for the variables.
    pub now: DateTime,
    /// The build the session records, `$(VERSION)`.
    pub build: String,
    pub live: Option<Live>,
    /// Whether the session draws a live frame, which shares the GPU with the export: a
    /// photo open and the viewport's size given. Read as each photo's operations before
    /// the tiles are planned, so a photo opened under way changes the next one's.
    pub live_frame: Arc<AtomicBool>,
    /// The photos' sidecars, in their order, as the edit writer reads them.
    pub snapshot: Receiver<Vec<Snapshot>>,
    pub outbox: Outbox,
    /// Holds the job's first render step for a test.
    #[cfg(test)]
    pub hold: Option<testing::Hold>,
}

/// The largest tile a job's export renders, across and down, however much the budget
/// allows: spike X1 measured a tile of 1024 at 4.8 ms of GPU time on the M4 for a Bayer
/// raw and 13.4 ms for X-Trans, and a whole render up to 0.9 s, which a frame would wait
/// behind.
pub const LARGEST_TILE: u32 = 1024;

/// The most bytes a row strip of a job's export binds of an image of the operations before
/// the tiles (the raw stage on a default edit): 8 Mi photosites of the f32 mosaic, about 8
/// MP a step, where the raw stage over the whole image is one step of up to 45 ms of GPU
/// time on the M4 at 61 MP; 2 MP of an RGBA image, where an edit with an operation that
/// runs whole with everything before it, bilat's or filmic rgb's, takes demosaic and more
/// before the tiles. Highlights' dilation, which binds only its masks, runs in bands of as
/// many rows, a step each; a pass that takes no strips stays one step: its chroma
/// (`opposed_chroma`) and demosaic (spike X1). Only beside a live frame: with none those
/// operations are one step, the stepping costing an export alone about 8%.
pub const STRIP_BINDING: u64 = 32 << 20;

/// How a job's export reaches the engine thread: each render handed through `tasks` and
/// its readback's mapping waited for, both through the job's context, so either returns
/// at a cancel; the operations before the tiles in strips while `live_frame` says a live
/// frame shares the GPU.
struct Handing<'a, 'b> {
    tasks: &'a Tasks,
    context: &'a Context<'b>,
    live_frame: &'a AtomicBool,
    /// Holds the first render's step for a test, and tells it the strips chosen.
    #[cfg(test)]
    hold: Option<testing::Hold>,
}

impl Hand for Handing<'_, '_> {
    fn hand(&mut self, submit: Submit) -> Result<Rendered, TaskError> {
        #[cfg(test)]
        if let Some(hold) = &self.hold
            && hold.refuses()
        {
            if submit.reads_back() {
                return self.tasks.run(self.context, testing::Refused(submit));
            }
            return self.tasks.run(self.context, submit);
        }
        #[cfg(test)]
        if let Some(hold) = self.hold.take() {
            return self
                .tasks
                .run(self.context, testing::Held::new(hold, submit));
        }
        self.tasks.run(self.context, submit)
    }

    fn wait(&mut self, mapping: &Mapping) -> Result<MapResult, TaskError> {
        jobs::wait(self.context, mapping)
    }

    fn strip(&self) -> Option<u64> {
        let strip = self
            .live_frame
            .load(Ordering::Relaxed)
            .then_some(STRIP_BINDING);
        #[cfg(test)]
        if let Some(hold) = &self.hold {
            hold.chose(strip);
        }
        strip
    }
}

/// The job that exports `run`'s photos, its renders handed to the engine thread through
/// `tasks`: darktable's "export images", cancellable.
pub(crate) fn job(run: Run, tasks: Tasks) -> Job {
    let description = format!("export {} images", run.photos.len());
    Job::new("export", description, move |context| {
        run.body(&tasks, context)
    })
    .progress(Message::new("job-export"), true)
}

impl Run {
    /// Each photo in turn, until the last, a failure of the destination or a cancel.
    fn body(self, tasks: &Tasks, context: &Context<'_>) -> Outcome {
        let start = Instant::now();
        let total = self.photos.len();
        let job = context.job();
        let report = |status| {
            self.outbox.send(Event::Export { job, status });
        };
        let mut summary = Summary::default();
        let settings = &self.settings;
        let names = names::Export::new(
            self.now,
            largest_size(settings),
            settings.size.upscale,
            &self.build,
            self.places.clone(),
        );
        let refused = |failure: Failure, mut summary: Summary| {
            tracing::debug!(target: "pipe", "export refused: {}", failure.message);
            summary.not_reached = total;
            report(ExportStatus::Finished(summary));
            Err(failure.ended(None))
        };
        // The path and its pattern before any photo (D8).
        let home = names.home();
        let disk = match Disk::new(&settings.path, settings.on_conflict, total, home) {
            Ok(disk) => disk,
            Err(error) => return refused(Failure::disk(&error, None), summary),
        };
        // The edits as they were when Export was pressed (D12).
        let snapshot = match context.wait(&self.snapshot) {
            Ok(snapshot) => snapshot,
            Err(Interrupted::Cancelled) => {
                summary.not_reached = total;
                summary.cancelled = true;
                report(ExportStatus::Finished(summary));
                return Ok(());
            }
            Err(Interrupted::Disconnected) => {
                let failure = Failure::new(Cause::Engine, None, "the edit writer stopped");
                return refused(failure, summary);
            }
        };
        for (index, (path, snapshot)) in self.photos.iter().zip(snapshot).enumerate() {
            if context.cancelled() {
                summary.not_reached = total - index;
                summary.cancelled = true;
                break;
            }
            let source = path.clone();
            report(ExportStatus::Started { index, source });
            context.set_message(
                Message::new("job-export-photo")
                    .with("number", index + 1)
                    .with("total", total)
                    .with("storage", "disk")
                    .with("photo", path.as_path()),
            );
            let Some(result) = self.photo(tasks, context, index, path, snapshot, &disk, &names)
            else {
                // Cancelled with it under way: dropped, its temporary file removed.
                summary.not_reached = total - index;
                summary.cancelled = true;
                break;
            };
            summary.count(&result);
            let stop = match &result {
                PhotoResult::Failed(failure) => {
                    let number = index + 1;
                    tracing::debug!(target: "pipe", "export: photo {number} failed: {}", failure.message);
                    failure.ends_job().then(|| failure.ended(Some(path)))
                }
                _ => None,
            };
            report(ExportStatus::Done { index, result });
            context.set_fraction((index + 1) as f64 / total as f64);
            if let Some(failure) = stop {
                summary.not_reached = total - index - 1;
                let rest = summary.not_reached;
                tracing::debug!(target: "pipe", "export: ended by its failure, {rest} not reached");
                report(ExportStatus::Finished(summary));
                return Err(failure);
            }
        }
        let ms = start.elapsed().as_secs_f64() * 1e3;
        let photos = if total == 1 { "photo" } else { "photos" };
        tracing::debug!(target: "perf", "exported {total} {photos} in {ms:.0} ms");
        report(ExportStatus::Finished(summary));
        Ok(())
    }

    /// Photo `index` of the job, the raw at `path` with the edit `snapshot` holds, or the
    /// open photo's: its result, or `None` where a cancel stopped it.
    #[allow(
        clippy::too_many_arguments,
        reason = "the photo, its place, and what the job holds for every photo"
    )]
    fn photo(
        &self,
        tasks: &Tasks,
        context: &Context<'_>,
        index: usize,
        path: &Path,
        snapshot: Snapshot,
        disk: &Disk,
        names: &names::Export,
    ) -> Option<PhotoResult> {
        let failed = |cause, file, error: &dyn Display| {
            Some(PhotoResult::Failed(Failure::new(cause, file, error)))
        };
        let live = self.live.as_ref();
        let live = live.filter(|live| live.sidecar == sidecar::path_for(path));
        let (mut edit, newer, image, input, shot, changed) = match live {
            Some(live) => {
                let changed = if live.unsaved {
                    Some(SystemTime::now())
                } else {
                    snapshot.changed
                };
                let (image, input) = (live.image.clone(), live.input.clone());
                (
                    live.edit.clone(),
                    live.newer,
                    image,
                    input,
                    live.shot.clone(),
                    changed,
                )
            }
            None => {
                let (edit, newer) = match self.edit(path, snapshot.xml) {
                    Ok(edit) => edit,
                    Err(message) => return failed(Cause::Edit, None, &message),
                };
                let decoded = match decode(&self.gpu, self.load, path) {
                    Ok(decoded) => decoded,
                    Err(message) => return failed(Cause::Unreadable, None, &message),
                };
                let Decoded {
                    image, input, shot, ..
                } = decoded;
                (edit, newer, image, input, shot, snapshot.changed)
            }
        };
        // As the session reads an edit: a number outside its range clamped, a newer
        // Apertrust's edit exported as this build shows it (decisions 30 and 43).
        if !newer {
            for warning in edit.clamp_read(&self.modules, &image) {
                tracing::warn!(target: "pipe", "{warning}");
            }
        }
        // A module whose binding fails fails the photo: an export renders the edit as it
        // is, or not at all.
        let instances = match edit::instances(&edit, self.operations, &self.modules, &image).all() {
            Ok(instances) => instances,
            Err(error) => return failed(Cause::Binding, None, &error),
        };
        let settings = &self.settings;
        let sensor = input.desc();
        let sized = size(
            &settings.size,
            &settings.format,
            &instances,
            &image,
            sensor,
            &self.limits,
        );
        let (resample, extent) = match sized {
            Ok(sized) => sized,
            Err(error @ SizeError::Plan(_)) => return failed(Cause::Render, None, &error),
            Err(error @ SizeError::Size(_)) => return failed(Cause::Size, None, &error),
        };

        // The name, and so whether the photo is exported at all, before it renders: the
        // folder made, On conflict applied, and the temporary file the render goes into.
        let size = |width, height| Size { width, height };
        let sizes = Sizes {
            sensor: size(sensor.width, sensor.height),
            developed: size(image.width, image.height),
            crop: size(extent[0], extent[1]),
        };
        let values = names.values(path, &shot, index as u32 + 1, sizes);
        let extension = settings.format.extension();
        let mut target = match disk.prepare(&values, extension, changed) {
            Ok(Prepared::Write(target)) => target,
            Ok(Prepared::Skip(Skipped { path, why })) => {
                return Some(PhotoResult::Skipped { path, why });
            }
            Err(error) => {
                let file = disk.name(&values, extension).ok();
                return Some(PhotoResult::Failed(Failure::disk(&error, file)));
            }
        };
        let file = target.path();

        // From here a cancel or a failure drops the target, which removes its file.
        let tiling = Tiling::AtMost {
            budget: self.limits.budget,
            edge: LARGEST_TILE,
        };
        // Planned, compiled and allocated here; only the submissions go to the engine
        // thread, each waited for through the job's context.
        let mut hand = Handing {
            tasks,
            context,
            live_frame: &self.live_frame,
            #[cfg(test)]
            hold: self.hold.clone(),
        };
        let gpu = &self.gpu;
        let (exported, error) = scoped(gpu, || {
            pollster::block_on(export_handing(
                gpu, &instances, &image, input, tiling, resample, &mut hand,
            ))
        });
        let exported = match (exported, error) {
            (Err(ExportError::Handed(TaskError::Cancelled)), _) => return None,
            (Err(ExportError::Handed(TaskError::Stopped)), _) => {
                return failed(Cause::Engine, Some(file), &TaskError::Stopped);
            }
            (_, Some(error)) => return failed(Cause::Render, Some(file), &error),
            (Err(error), None) => return failed(Cause::Render, Some(file), &error),
            (Ok(exported), None) => exported,
        };
        let ImageDesc { width, height, .. } = exported.desc;
        if let Err(failure) = write(
            &mut target,
            &settings.format,
            &exported.pixels,
            width,
            height,
        ) {
            return Some(PhotoResult::Failed(failure));
        }
        if context.cancelled() {
            return None;
        }
        let start = Instant::now();
        let result = match target.finish() {
            Ok(Stored::Written(path)) => PhotoResult::Written {
                path,
                width,
                height,
                tiles: exported.tiles,
                newer,
            },
            Ok(Stored::Skipped(Skipped { path, why })) => PhotoResult::Skipped { path, why },
            Err(error) => PhotoResult::Failed(Failure::disk(&error, Some(file.clone()))),
        };
        let ms = start.elapsed().as_secs_f64() * 1e3;
        let name = sidecar::file_name(&file);
        tracing::debug!(target: "perf", "stored {name} in {ms:.0} ms");
        Some(result)
    }

    /// The edit a photo's sidecar holds, as `xml` the writer read it, and whether a newer
    /// Apertrust wrote it; none for a photo never edited.
    fn edit(&self, path: &Path, xml: io::Result<Option<String>>) -> Result<(Edit, bool), String> {
        let sidecar = sidecar::path_for(path);
        let reading = |error: &dyn Display| format!("reading {}: {error}", sidecar.display());
        let xml = match xml {
            Ok(Some(xml)) => xml,
            Ok(None) => return Ok((Edit::default(), false)),
            Err(error) => return Err(reading(&error)),
        };
        let loaded = sidecar::from_xml(&xml, self.operations, &self.modules, self.retired)
            .map_err(|error| reading(&error))?;
        for warning in &loaded.warnings {
            tracing::warn!(target: "pipe", "{}: {warning}", sidecar.display());
        }
        let newer = loaded.view_only().is_some();
        Ok((loaded.edit, newer))
    }
}

/// Decodes and uploads the raw at `path` through `load` on the job's worker, a panic and
/// the GPU's errors caught as its failure, in plain words.
fn decode(gpu: &GpuContext, load: Load, path: &Path) -> Result<Decoded, String> {
    let (decoded, error) = scoped(gpu, || {
        panic::catch_unwind(AssertUnwindSafe(|| load(gpu, path)))
    });
    if let Some(error) = error {
        return Err(format!("uploading {}: {error}", path.display()));
    }
    match decoded {
        Ok(Ok(decoded)) => Ok(decoded),
        Ok(Err(error)) => Err(error.to_string()),
        Err(panic) => Err(format!(
            "the decoder failed: {}",
            panic_message(panic.as_ref())
        )),
    }
}

/// Runs `work` on this thread inside error scopes of its own, and returns what it returned
/// and the first GPU error it caused, in plain words, rather than the panic wgpu makes of
/// one: the worker makes buffers and pipelines, which the engine thread's scopes never see.
fn scoped<T>(gpu: &GpuContext, work: impl FnOnce() -> T) -> (T, Option<String>) {
    let memory = gpu.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
    let validation = gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
    let result = work();
    let errors = [validation.pop(), memory.pop()].map(pollster::block_on);
    let error = errors.into_iter().flatten().next();
    (result, error.map(|error| format!("the GPU: {error}")))
}

/// Writes an image output colour has encoded for darktable's sRGB in `format` into
/// `target`'s temporary file, as `apertrust-cli export` wrote it.
fn write(
    target: &mut Target,
    format: &Format,
    pixels: &[[f32; 4]],
    width: u32,
    height: u32,
) -> Result<(), Failure> {
    let file = target.path();
    let image = Image::new(pixels, width, height)
        .map_err(|error| Failure::new(Cause::Encode, Some(file.clone()), error))?;
    let start = Instant::now();
    let mut out = BufWriter::new(target.file());
    // The buffer's last bytes can meet a full disk too.
    let written = format
        .write(image, &mut out)
        .and_then(|()| out.flush().map_err(apertrust_export::Error::Io));
    written.map_err(|error| Failure::encoder(&error, &file))?;
    let ms = start.elapsed().as_secs_f64() * 1e3;
    let name = sidecar::file_name(&file);
    tracing::debug!(target: "perf", "wrote {name} in {ms:.0} ms");
    Ok(())
}

/// What the tests hold an export with, and the paths they send it.
#[cfg(test)]
pub(crate) mod testing {
    use std::path::Path;
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::sync::{Arc, Mutex, PoisonError};

    use crate::gpu::GpuContext;
    use crate::jobs::{GpuTask, Step};

    /// An export's path of `rest` in `folder`, its folders written with `/`: a path reads
    /// `\` as an escape, so Windows' own separator would vanish, as in darktable.
    pub(crate) fn path_in(folder: &Path, rest: &str) -> String {
        format!("{}/{rest}", folder.to_string_lossy().replace('\\', "/"))
    }

    /// Holds the first step of an export's render, on the engine thread, as a long step
    /// would: it tells the test it has begun, then waits for the test's permit, or for the
    /// permit's sender to drop. Or, made by [`Hold::refusing`], holds nothing and has the
    /// GPU refuse every render that reads a tile back. Made [`Hold::telling_strips`], it
    /// also tells the strip each photo's operations before the tiles were planned with.
    #[derive(Debug, Clone)]
    pub struct Hold {
        began: Sender<()>,
        permit: Arc<Mutex<Receiver<()>>>,
        refuse: bool,
        strips: Option<Sender<Option<u64>>>,
    }

    impl PartialEq for Hold {
        fn eq(&self, other: &Self) -> bool {
            Arc::ptr_eq(&self.permit, &other.permit)
        }
    }

    impl Hold {
        /// The hold, what it tells as it begins, and its permit.
        pub fn new() -> (Hold, Receiver<()>, Sender<()>) {
            let (began, begun) = mpsc::channel();
            let (permit, permits) = mpsc::channel();
            let permit_lock = Arc::new(Mutex::new(permits));
            let hold = Hold {
                began,
                permit: permit_lock,
                refuse: false,
                strips: None,
            };
            (hold, begun, permit)
        }

        /// This hold, telling through what it returns, for each photo in turn, the strip
        /// its operations before the tiles were planned with ([`crate::export::Hand::strip`]).
        pub fn telling_strips(self) -> (Hold, Receiver<Option<u64>>) {
            let (strips, told) = mpsc::channel();
            let strips = Some(strips);
            (Hold { strips, ..self }, told)
        }

        pub(crate) fn chose(&self, strip: Option<u64>) {
            if let Some(strips) = &self.strips {
                let _ = strips.send(strip);
            }
        }

        /// A hold that refuses every render reading a tile back ([`Refused`]).
        pub fn refusing() -> Hold {
            let (hold, _, _) = Hold::new();
            Hold {
                refuse: true,
                ..hold
            }
        }

        pub(crate) fn refuses(&self) -> bool {
            self.refuse
        }

        pub(crate) fn wait(&self) {
            let _ = self.began.send(());
            let _ = self
                .permit
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .recv();
        }
    }

    /// A GPU task whose first step waits for its hold first.
    pub(super) struct Held<T> {
        hold: Option<Hold>,
        task: T,
    }

    impl<T> Held<T> {
        pub(super) fn new(hold: Hold, task: T) -> Self {
            let hold = Some(hold);
            Self { hold, task }
        }
    }

    impl<T: GpuTask> GpuTask for Held<T> {
        type Output = T::Output;

        fn step(&mut self, gpu: &GpuContext) -> Result<Step<T::Output>, String> {
            if let Some(hold) = self.hold.take() {
                hold.wait();
            }
            self.task.step(gpu)
        }
    }

    /// A GPU task whose every step ends with a copy the GPU refuses, as a refused render
    /// would: its step's error scopes hear it after the step has finished.
    pub(super) struct Refused<T>(pub T);

    impl<T: GpuTask> GpuTask for Refused<T> {
        type Output = T::Output;

        fn step(&mut self, gpu: &GpuContext) -> Result<Step<T::Output>, String> {
            let step = self.0.step(gpu)?;
            let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("refused"),
                size: 256,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let mut encoder = gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            // Past the buffer's end.
            encoder.copy_buffer_to_buffer(&buffer, 0, &buffer, 128, 1024);
            gpu.submit([encoder.finish()]);
            Ok(step)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::mpsc;

    use apertrust_export::png;
    use apertrust_export::settings::OnConflict;
    use apertrust_modules::ControlValue;
    use apertrust_modules::testing::{self as modules, Sample};
    use apertrust_operations::OPERATIONS;

    use super::*;
    use crate::export::export_at;
    use crate::gpu::GpuOptions;
    use crate::jobs::{Argument, JobEvent, JobId, Queue, State};
    use crate::session::{Command, Session, SessionError, TEST_WAIT};

    /// The build the tests' sessions record.
    const BUILD: &str = "0.4.0+12~g1a2b3c4";

    /// The modules the tests edit with: `Sample`'s brightness moves exposure.
    const MODULES: &[&dyn Module] = &[&Sample];

    fn gpu() -> GpuContext {
        pollster::block_on(GpuContext::new(&GpuOptions::default()))
            .expect("a GPU adapter, hardware or software")
    }

    /// The modules crate's synthetic photo, whatever the path, as the session's tests load
    /// it, or the wide one for a raw called `wide.ARW` ([`photo`]); one called `broken.ARW`
    /// cannot be read.
    fn synthetic(gpu: &GpuContext, path: &Path) -> Result<Decoded, SessionError> {
        if path.ends_with("broken.ARW") {
            return Err(SessionError::NotAMosaic);
        }
        let (image, [width, height], mosaic) = photo(path);
        Ok(Decoded {
            camera: "Synthetic".to_owned(),
            image,
            input: GpuImage::upload_mosaic(gpu, width, height, &mosaic).expect("it uploads"),
            shot: Shot::default(),
            decode_ms: 0.0,
            upload_ms: 0.0,
        })
    }

    /// The synthetic photo the tests' loader gives for `path`, its context, its sensor and
    /// its mosaic: the modules crate's, 96 × 72 photosites, or for `wide.ARW` the same
    /// gradient on 1100 × 72, its image 1088 × 62, wider than a job's largest tile and
    /// small enough for the budget to render whole.
    fn photo(path: &Path) -> (ImageContext, [u32; 2], Vec<u16>) {
        if !path.ends_with("wide.ARW") {
            return (modules::image(), [96, 72], modules::mosaic());
        }
        let [width, height] = [1100, 72];
        let mut camera = modules::image().camera.expect("a raw's");
        camera.width = width;
        camera.crop = [2, 2, 1096, 68];
        camera.declared = [6, 4, 1088, 62];
        let image = ImageContext {
            camera: Some(camera),
            ..ImageContext::full(1088, 62)
        };
        let mosaic = (0..width * height)
            .map(|i| {
                let (x, y) = (i % width, i / width);
                let c = [0, 1, 1, 2][((y % 2) * 2 + x % 2) as usize] as f32;
                let level =
                    0.1 + 0.6 * x as f32 / width as f32 + 0.1 * c + 0.2 * y as f32 / height as f32;
                (level.min(1.0) * 4000.0).round() as u16
            })
            .collect();
        (image, [width, height], mosaic)
    }

    /// A folder of the test's own, empty.
    fn scratch(name: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("apertrust-export-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    /// The names in `folder`, sorted.
    fn listed(folder: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// The temporary files of exports in `folder`.
    fn temporaries(folder: &Path) -> Vec<String> {
        let temporary = |name: &String| name.starts_with(".~") && name.ends_with(".tmp");
        listed(folder).into_iter().filter(temporary).collect()
    }

    /// An export of `photos` as 8-bit PNGs into `folder`, by name, at full size.
    fn export(photos: &[&Path], folder: &Path) -> Command {
        Command::Export {
            photos: photos.iter().map(|p| p.to_path_buf()).collect(),
            settings: ExportSettings {
                path: testing::path_in(folder, "$(FILE_NAME)"),
                on_conflict: OnConflict::CreateUniqueFilename,
                format: Format::Png(png::Options::default()),
                size: Request::default(),
            },
            places: Places::default(),
        }
    }

    fn brightness(value: f32, done: bool) -> Command {
        Command::SetControl {
            module: "sample".to_owned(),
            control: "brightness".to_owned(),
            value: ControlValue::Number(value),
            done,
        }
    }

    /// The synthetic photo's edit with `Sample`'s brightness at `value`.
    fn edit_at(value: Option<f32>) -> Edit {
        let mut edit = Edit::default();
        if let Some(value) = value {
            let number = ControlValue::Number(value);
            let image = modules::image();
            edit.set_control(&Sample, "brightness", number, &image)
                .unwrap();
        }
        edit
    }

    /// The PNG an export of the synthetic photo with brightness at `value` writes: rendered
    /// as an export alone renders it and encoded in memory, so the same process stamps the
    /// same profile.
    fn expected(gpu: &GpuContext, value: Option<f32>) -> Vec<u8> {
        expected_of(gpu, Path::new("a.ARW"), value)
    }

    /// [`expected`] for the synthetic photo the loader gives for `path`: rendered whole
    /// where the budget allows it, as `apertrust-cli export --runs` renders it.
    fn expected_of(gpu: &GpuContext, path: &Path, value: Option<f32>) -> Vec<u8> {
        let (image, [width, height], mosaic) = photo(path);
        let edit = edit_at(value);
        let instances = edit::instances(&edit, OPERATIONS, MODULES, &image)
            .all()
            .unwrap();
        let input = GpuImage::upload_mosaic(gpu, width, height, &mosaic).unwrap();
        let tiling = Tiling::Auto {
            budget: pipeline::BUDGET,
        };
        let exported = export_at(gpu, &instances, &image, input, tiling, None);
        let exported = pollster::block_on(exported).unwrap();
        assert_eq!(
            exported.tiles,
            1,
            "the budget renders {} whole",
            path.display()
        );
        let ImageDesc { width, height, .. } = exported.desc;
        let mut bytes = Vec::new();
        let image = Image::new(&exported.pixels, width, height).unwrap();
        Format::Png(png::Options::default())
            .write(image, &mut bytes)
            .unwrap();
        bytes
    }

    /// A session as a client holds it, with every event it has sent so far.
    struct Client {
        session: Session,
        wakes: Receiver<()>,
        events: Vec<Event>,
    }

    impl Client {
        fn start(gpu: &GpuContext) -> Self {
            let (woken, wakes) = mpsc::channel();
            let wake = move || {
                let _ = woken.send(());
            };
            let session = Session::spawn(
                gpu.clone(),
                OPERATIONS,
                MODULES,
                BUILD,
                pipeline::BUDGET,
                Arc::new(wake),
                synthetic,
            );
            Self {
                session,
                wakes,
                events: Vec::new(),
            }
        }

        /// A session with `path` open, its first frame shown.
        fn beside_a_viewport(gpu: &GpuContext, path: &Path) -> Self {
            let mut client = Client::start(gpu);
            client.send(Command::Resize {
                width: 64,
                height: 48,
            });
            client.send(Command::Open(path.to_owned()));
            let sent = client.send(brightness(0.5, false));
            client.frame(sent);
            client
        }

        fn send(&self, command: Command) -> u64 {
            self.session.send(command)
        }

        /// Reads events until one of them, of all so far, satisfies `last`.
        fn until(&mut self, last: impl Fn(&Event) -> bool) {
            while !self.events.iter().any(&last) {
                self.wakes.recv_timeout(TEST_WAIT).unwrap_or_else(|_| {
                    panic!("still waiting, {} events heard", self.events.len())
                });
                self.events.extend(self.session.events());
            }
        }

        /// Waits for a frame that includes the command numbered `sent`.
        fn frame(&mut self, sent: u64) {
            self.until(|e| matches!(e, Event::Frame(frame) if frame.through >= sent));
        }

        /// Waits for the job the command numbered `command` started to be added.
        fn job(&mut self, command: u64) -> JobId {
            let added = |e: &Event| match e {
                Event::Job {
                    event: JobEvent::Added { job, .. },
                    command: Some(sent),
                } if *sent == command => Some(*job),
                _ => None,
            };
            self.until(|e| added(e).is_some());
            self.events.iter().find_map(added).unwrap()
        }

        /// Waits for `job` to end, and returns its own events.
        fn ended(&mut self, job: JobId) -> Vec<JobEvent> {
            self.until(|e| ends(e, job));
            self.of_job(job)
        }

        fn of_job(&self, job: JobId) -> Vec<JobEvent> {
            let own = |event: &JobEvent| match event {
                JobEvent::Added { job, .. }
                | JobEvent::Progress { job, .. }
                | JobEvent::Message { job, .. }
                | JobEvent::State { job, .. }
                | JobEvent::Ended { job, .. } => *job,
            };
            let events = self.events.iter().filter_map(|e| match e {
                Event::Job { event, .. } if own(event) == job => Some(event.clone()),
                _ => None,
            });
            events.collect()
        }

        /// `job`'s statuses, in order.
        fn exported(&self, job: JobId) -> Vec<ExportStatus> {
            let events = self.events.iter().filter_map(|e| match e {
                Event::Export { job: own, status } if *own == job => Some(status.clone()),
                _ => None,
            });
            events.collect()
        }

        /// Where in the events the first that `wanted` accepts is.
        fn at(&self, wanted: impl Fn(&Event) -> bool) -> usize {
            self.events.iter().position(wanted).expect("the event came")
        }
    }

    /// Whether `event` is `job`'s end.
    fn ends(event: &Event, job: JobId) -> bool {
        matches!(
            event,
            Event::Job {
                event: JobEvent::Ended { job: own, .. },
                ..
            } if *own == job
        )
    }

    /// Whether `event` is `job`'s move to `state`.
    fn moves(event: &Event, job: JobId, state: State) -> bool {
        matches!(
            event,
            Event::Job {
                event: JobEvent::State { job: own, state: moved },
                ..
            } if *own == job && *moved == state
        )
    }

    /// The photos' results, in order.
    fn results(statuses: &[ExportStatus]) -> Vec<&PhotoResult> {
        statuses
            .iter()
            .filter_map(|status| match status {
                ExportStatus::Done { result, .. } => Some(result),
                _ => None,
            })
            .collect()
    }

    fn summary(statuses: &[ExportStatus]) -> Summary {
        match statuses.last() {
            Some(ExportStatus::Finished(summary)) => *summary,
            other => panic!("no counts: {other:?}"),
        }
    }

    #[test]
    fn three_photos_the_live_one_among_them_export_beside_a_viewport() {
        let gpu = gpu();
        let folder = scratch("three");
        let [a, live, b] = ["a.ARW", "live.ARW", "b.ARW"].map(|name| folder.join(name));
        // `b`'s edit is the one saved beside it.
        let saved = edit_at(Some(1.0));
        sidecar::write(&sidecar::path_for(&b), &saved, OPERATIONS, MODULES, BUILD).unwrap();
        let out = folder.join("out");
        let mut client = Client::beside_a_viewport(&gpu, &live);
        let command = client.send(export(&[&a, &live, &b], &out));
        let job = client.job(command);
        // The viewport goes on beside it.
        let sent = client.send(Command::Resize {
            width: 80,
            height: 60,
        });
        client.frame(sent);
        let own = client.ended(job);
        assert!(
            matches!(
                own.last(),
                Some(JobEvent::Ended {
                    state: State::Finished,
                    failure: None,
                    ..
                })
            ),
            "{own:?}"
        );
        let statuses = client.exported(job);
        let all = Summary {
            written: 3,
            ..Summary::default()
        };
        assert_eq!(summary(&statuses), all);
        // Each from its own edit: `a` none, the live photo the session's, `b` its file's.
        let photos = [
            ("a.png", None),
            ("live.png", Some(0.5)),
            ("b.png", Some(1.0)),
        ];
        for (result, (name, value)) in results(&statuses).into_iter().zip(photos) {
            let PhotoResult::Written {
                path,
                width,
                height,
                tiles,
                newer,
            } = result
            else {
                panic!("{name}: {result:?}");
            };
            assert_eq!(path, &out.join(name));
            assert_eq!((*width, *height, *tiles, *newer), (84, 62, 1, false));
            let bytes = std::fs::read(path).unwrap();
            assert!(bytes == expected(&gpu, value), "{name} differs");
        }
        assert_eq!(listed(&out), ["a.png", "b.png", "live.png"]);
        drop(client);
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// A job's operations before the tiles take row strips only beside a live frame: with
    /// no photo open one step, with a photo but no viewport's size one step, and with both
    /// strips of [`STRIP_BINDING`], each chosen as that photo's render is planned, so a
    /// photo opened under way changes the next photo's, not its own; and every file is the
    /// same.
    #[test]
    fn a_job_steps_its_raw_stage_in_strips_only_beside_a_live_frame() {
        let gpu = gpu();
        let folder = scratch("live-frame");
        let photos = ["a.ARW", "b.ARW", "c.ARW"].map(|name| folder.join(name));
        let out = folder.join("out");
        let mut client = Client::start(&gpu);
        let (hold, begun, permit) = testing::Hold::new();
        let (hold, strips) = hold.telling_strips();
        client.send(Command::HoldExport(hold));
        let command = client.send(export(&[&photos[0], &photos[1], &photos[2]], &out));
        let job = client.job(command);
        // Each photo's first render step holds the engine thread, its strip chosen before:
        // the first with no photo open, the photo opened as it waits; the second with no
        // viewport's size, given as it waits; the third beside both.
        let mut chosen = Vec::new();
        let between = [
            Some(Command::Open(folder.join("live.ARW"))),
            Some(Command::Resize {
                width: 64,
                height: 48,
            }),
            None,
        ];
        for command in between {
            begun.recv_timeout(TEST_WAIT).unwrap();
            chosen.push(strips.try_recv().expect("chosen as the render was planned"));
            if let Some(command) = command {
                client.send(command);
            }
            permit.send(()).unwrap();
        }
        assert_eq!(chosen, [None, None, Some(STRIP_BINDING)]);
        client.ended(job);
        let statuses = client.exported(job);
        let written = Summary {
            written: 3,
            ..Summary::default()
        };
        assert_eq!(summary(&statuses), written);
        let expected = expected(&gpu, None);
        for name in ["a.png", "b.png", "c.png"] {
            let bytes = std::fs::read(out.join(name)).unwrap();
            assert!(bytes == expected, "{name} differs");
        }
        drop(permit);
        drop(client);
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// A photo the budget renders whole the job renders in tiles of at most
    /// [`LARGEST_TILE`], and its file is the whole render's, byte for byte.
    #[test]
    fn a_job_tiles_a_photo_the_budget_renders_whole_and_writes_the_same_file() {
        let gpu = gpu();
        let folder = scratch("tiled");
        let wide = folder.join("wide.ARW");
        let out = folder.join("out");
        let mut client = Client::beside_a_viewport(&gpu, &folder.join("live.ARW"));
        let command = client.send(export(&[&wide], &out));
        let job = client.job(command);
        client.ended(job);
        let statuses = client.exported(job);
        let [
            PhotoResult::Written {
                path,
                width,
                height,
                tiles,
                ..
            },
        ] = results(&statuses)[..]
        else {
            panic!("{statuses:?}");
        };
        // 1088 across in tiles of at most 1024: two, the second 64 across.
        assert_eq!((*width, *height), (1088, 62));
        assert_eq!(*tiles, 1088_usize.div_ceil(LARGEST_TILE as usize));
        let bytes = std::fs::read(path).unwrap();
        assert!(bytes == expected_of(&gpu, &wide, None), "the file differs");
        drop(client);
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// A render the GPU refuses in the step that reads its tile back fails the photo, its
    /// file never written, though the step had finished and its readback was mapped.
    #[test]
    fn a_refused_readback_render_fails_the_photo_and_writes_no_file() {
        let gpu = gpu();
        let folder = scratch("refused");
        let out = folder.join("out");
        let mut client = Client::start(&gpu);
        client.send(Command::HoldExport(testing::Hold::refusing()));
        let command = client.send(export(&[&folder.join("a.ARW")], &out));
        let job = client.job(command);
        client.ended(job);
        let statuses = client.exported(job);
        let [PhotoResult::Failed(failure)] = results(&statuses)[..] else {
            panic!("{statuses:?}");
        };
        assert_eq!(failure.cause, Cause::Render, "{failure:?}");
        assert!(failure.message.contains("the GPU"), "{failure:?}");
        assert!(listed(&out).is_empty(), "{:?}", listed(&out));
        drop(client);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_cancel_mid_photo_leaves_no_temporary_file() {
        let gpu = gpu();
        let folder = scratch("cancel");
        let mut client = Client::start(&gpu);
        let (hold, begun, permit) = testing::Hold::new();
        client.send(Command::HoldExport(hold));
        let photos = [folder.join("a.ARW"), folder.join("b.ARW")];
        let command = client.send(export(&[&photos[0], &photos[1]], &folder));
        let job = client.job(command);
        // The first photo's render has begun: its file is being written beside its name.
        begun.recv_timeout(TEST_WAIT).unwrap();
        assert_eq!(temporaries(&folder).len(), 1, "{:?}", listed(&folder));
        client.send(Command::CancelJob { job });
        permit.send(()).unwrap();
        let own = client.ended(job);
        assert!(
            matches!(
                own.last(),
                Some(JobEvent::Ended {
                    state: State::Cancelled,
                    failure: None,
                    ..
                })
            ),
            "{own:?}"
        );
        let statuses = client.exported(job);
        let cancelled = Summary {
            not_reached: 2,
            cancelled: true,
            ..Summary::default()
        };
        assert_eq!(summary(&statuses), cancelled);
        assert!(results(&statuses).is_empty(), "{statuses:?}");
        // Nothing under a photo's name, and no temporary file.
        assert!(listed(&folder).is_empty(), "{:?}", listed(&folder));
        drop(permit);
        drop(client);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_session_dropped_with_an_export_in_flight_ends_and_leaves_no_temporary_file() {
        let gpu = gpu();
        let folder = scratch("dropped");
        let client = Client::start(&gpu);
        let (hold, begun, permit) = testing::Hold::new();
        client.send(Command::HoldExport(hold));
        let photos = [folder.join("a.ARW"), folder.join("b.ARW")];
        client.send(export(&[&photos[0], &photos[1]], &folder));
        // The first photo's first render step holds the engine thread, its file being
        // written beside its name.
        begun.recv_timeout(TEST_WAIT).unwrap();
        assert_eq!(temporaries(&folder).len(), 1, "{:?}", listed(&folder));
        let Client { session, .. } = client;
        let (ended, done) = mpsc::channel();
        std::thread::spawn(move || {
            drop(session);
            let _ = ended.send(());
        });
        // The step ends; the session's end cancels the export, which can be cancelled.
        permit.send(()).unwrap();
        done.recv_timeout(TEST_WAIT)
            .expect("the session ends with its export in flight");
        assert!(listed(&folder).is_empty(), "{:?}", listed(&folder));
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_second_export_waits_queued_behind_the_first() {
        let gpu = gpu();
        let folder = scratch("queued");
        let mut client = Client::start(&gpu);
        let (hold, begun, permit) = testing::Hold::new();
        client.send(Command::HoldExport(hold));
        let first = client.send(export(&[&folder.join("a.ARW")], &folder));
        let second = client.send(export(&[&folder.join("b.ARW")], &folder));
        let (first, second) = (client.job(first), client.job(second));
        begun.recv_timeout(TEST_WAIT).unwrap();
        client.until(|e| moves(e, second, State::Queued));
        // The first holds the engine thread in its first step: the second has not run.
        assert!(
            !client
                .events
                .iter()
                .any(|e| moves(e, second, State::Running))
        );
        drop(permit);
        client.ended(second);
        let first_ended = client.at(|e| ends(e, first));
        let second_ran = client.at(|e| moves(e, second, State::Running));
        assert!(first_ended < second_ran, "{:?}", client.events);
        let one = Summary {
            written: 1,
            ..Summary::default()
        };
        for job in [first, second] {
            assert_eq!(summary(&client.exported(job)), one);
        }
        assert_eq!(listed(&folder), ["a.png", "b.png"]);
        drop(client);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_photo_s_failure_is_its_own_and_the_destination_s_ends_the_job() {
        let gpu = gpu();
        let folder = scratch("destination");
        // A file where the export's folder would be made, so no photo can be written.
        std::fs::write(folder.join("blocker"), b"").unwrap();
        let photos = ["broken.ARW", "a.ARW", "b.ARW"].map(|name| folder.join(name));
        let mut client = Client::start(&gpu);
        let out = folder.join("blocker/out");
        let command = client.send(export(&[&photos[0], &photos[1], &photos[2]], &out));
        let job = client.job(command);
        let own = client.ended(job);
        let Some(JobEvent::Ended {
            state: State::Finished,
            failure: Some(failure),
            ..
        }) = own.last()
        else {
            panic!("it did not end failed: {own:?}");
        };
        assert_eq!(failure.id, "job-export-failed");
        let photo = Argument::Path(photos[1].clone());
        assert_eq!(failure.argument("photo"), Some(&photo));
        let statuses = client.exported(job);
        let counts = Summary {
            failed: 2,
            not_reached: 1,
            ..Summary::default()
        };
        assert_eq!(summary(&statuses), counts);
        let results = results(&statuses);
        let [
            PhotoResult::Failed(unreadable),
            PhotoResult::Failed(destination),
        ] = results[..]
        else {
            panic!("{results:?}");
        };
        // The raw that cannot be read is that photo's failure, and the job went on.
        assert_eq!(unreadable.cause, Cause::Unreadable);
        assert!(!unreadable.destination());
        // The folder that cannot be made is the destination's, and ends it.
        assert!(destination.destination(), "{destination:?}");
        assert_eq!(destination.file, Some(out.join("a.png")));
        assert_eq!(listed(&folder), ["blocker"]);
        drop(client);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_change_sent_after_export_is_absent_from_the_file() {
        let gpu = gpu();
        let folder = scratch("after");
        let live = folder.join("live.ARW");
        let mut client = Client::beside_a_viewport(&gpu, &live);
        // The drag's value, done, and so saved.
        let sent = client.send(brightness(0.5, true));
        client.until(|e| matches!(e, Event::Saved { through, .. } if *through >= sent));
        let (hold, begun, permit) = testing::Hold::new();
        client.send(Command::HoldExport(hold));
        let out = folder.join("held");
        let command = client.send(export(&[&live], &out));
        let job = client.job(command);
        // Sent while the export's render is under way, and applied before it ends.
        begun.recv_timeout(TEST_WAIT).unwrap();
        let after = client.send(brightness(1.5, true));
        permit.send(()).unwrap();
        client.frame(after);
        client.ended(job);
        let bytes = std::fs::read(out.join("live.png")).unwrap();
        let before = expected(&gpu, Some(0.5));
        assert!(
            bytes == before,
            "the change sent after Export is in the file"
        );

        // The photo's edit handed to the writer as another opens, then exported from its
        // file: the snapshot is read after that write, so it holds the change sent before.
        client.send(brightness(0.25, true));
        client.send(Command::Open(folder.join("other.ARW")));
        let out = folder.join("saved");
        let command = client.send(export(&[&live], &out));
        let job = client.job(command);
        client.ended(job);
        let bytes = std::fs::read(out.join("live.png")).unwrap();
        let saved = expected(&gpu, Some(0.25));
        assert!(
            bytes == saved,
            "the change saved before Export is not in the file"
        );
        drop(permit);
        drop(client);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn an_export_s_events_come_in_order() {
        let gpu = gpu();
        let folder = scratch("order");
        let photos = [folder.join("a.ARW"), folder.join("b.ARW")];
        let mut client = Client::start(&gpu);
        let command = client.send(export(&[&photos[0], &photos[1]], &folder));
        let job = client.job(command);
        let own = client.ended(job);
        // The job's own: added, queued, running, its messages and fractions, its end.
        let added = JobEvent::Added {
            job,
            kind: "export",
            queue: Some(Queue::UserExport),
            message: Message::new("job-export"),
            cancellable: true,
        };
        let queued = JobEvent::State {
            job,
            state: State::Queued,
        };
        let running = JobEvent::State {
            job,
            state: State::Running,
        };
        assert_eq!(own[..3], [added, queued, running]);
        let finished = JobEvent::Ended {
            job,
            state: State::Finished,
            failure: None,
        };
        assert_eq!(own.last(), Some(&finished));
        let photo = |number: usize| {
            Message::new("job-export-photo")
                .with("number", number)
                .with("total", 2)
                .with("storage", "disk")
                .with("photo", photos[number - 1].as_path())
        };
        let messages: Vec<&Message> = own
            .iter()
            .filter_map(|e| match e {
                JobEvent::Message { message, .. } => Some(message),
                _ => None,
            })
            .collect();
        // The first goes out at once and the last as the job ends, whatever the throttle.
        assert_eq!(messages.first(), Some(&&photo(1)));
        assert_eq!(messages.last(), Some(&&photo(2)));
        let fractions: Vec<f64> = own
            .iter()
            .filter_map(|e| match e {
                JobEvent::Progress { fraction, .. } => Some(*fraction),
                _ => None,
            })
            .collect();
        assert_eq!(fractions.first(), Some(&0.5));
        assert_eq!(fractions.last(), Some(&1.0));
        // The photos', each started then done, then the counts.
        let statuses = client.exported(job);
        let [
            ExportStatus::Started {
                index: 0,
                source: first,
            },
            ExportStatus::Done {
                index: 0,
                result: PhotoResult::Written { .. },
            },
            ExportStatus::Started {
                index: 1,
                source: second,
            },
            ExportStatus::Done {
                index: 1,
                result: PhotoResult::Written { .. },
            },
            ExportStatus::Finished(summary),
        ] = &statuses[..]
        else {
            panic!("{statuses:?}");
        };
        assert_eq!([first, second], [&photos[0], &photos[1]]);
        assert_eq!(summary.written, 2);
        // All between the job's running and its end.
        let ran = client.at(|e| moves(e, job, State::Running));
        let exported = |e: &Event| matches!(e, Event::Export { job: own, .. } if *own == job);
        let first_export = client.at(exported);
        let last_export = client.events.iter().rposition(exported).unwrap();
        let ended = client.at(|e| ends(e, job));
        assert!(
            ran < first_export && last_export < ended,
            "{:?}",
            client.events
        );
        drop(client);
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// A failure is the destination's exactly where `apertrust-export` says a write that
    /// failed so is, so the job stops where `apertrust-cli` stopped before it.
    #[test]
    fn a_failure_is_the_destination_s_where_the_disk_says_so() {
        use std::io::{Error, ErrorKind};
        let kinds = [
            ErrorKind::StorageFull,
            ErrorKind::QuotaExceeded,
            ErrorKind::ReadOnlyFilesystem,
            ErrorKind::NotFound,
            ErrorKind::PermissionDenied,
            ErrorKind::NotADirectory,
            ErrorKind::Other,
        ];
        let path = PathBuf::from("a.png");
        for kind in kinds {
            let errors = [
                disk::Error::Write {
                    path: path.clone(),
                    source: Error::from(kind),
                },
                disk::Error::CreateFolder {
                    path: path.clone(),
                    source: Error::from(kind),
                },
                disk::Error::WriteFolder {
                    path: path.clone(),
                    source: Error::from(kind),
                },
            ];
            for error in errors {
                let failure = Failure::disk(&error, None);
                assert_eq!(failure.destination(), error.destination(), "{error}");
            }
            let encoder = apertrust_export::Error::Io(Error::from(kind));
            let failure = Failure::encoder(&encoder, &path);
            assert_eq!(failure.destination(), encoder.destination(), "{kind}");
        }
        for error in [disk::Error::Empty, disk::Error::NoName(String::new())] {
            assert!(!Failure::disk(&error, None).destination());
        }
    }
}
