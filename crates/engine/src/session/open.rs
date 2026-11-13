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

//! A photo that opens in stages (decision 77; `docs/engine.md`, Opening a photo): the
//! open's job, kind `open` on darktable's `SystemForeground` queue, with no progress. Its
//! body, on a worker, maps the raw once ([`RawFile`]) and:
//!
//! 1. sends the largest of the photo's rendered thumbnails the cache holds, stale or not,
//!    as [`Event::OpenPreview`];
//! 2. reads the raw's metadata without its pixels, its EXIF and its sidecar, once the
//!    edit writer has written what it was handed of it, and hands them to the engine
//!    thread ([`Stage::Read`]), which sends `Opened` and the values, not yet live;
//! 3. where step 1 found none, sends the camera's embedded picture at the longer side of
//!    the last `Resize`, or no picture where the raw holds none;
//! 4. waits for the session's one decode slot ([`Decodes`]) through `Context::wait`, so
//!    a cancel ends the wait, tells the engine thread the decode begins
//!    ([`Stage::Decoding`]), which compiles the photo's kernels meanwhile, decodes, and
//!    hands the mosaic to the engine thread ([`Stage::Decoded`]), which uploads and
//!    renders it; a decode whose open was cancelled meanwhile runs to its end, since
//!    rawler cannot be stopped, and its mosaic is dropped.
//!
//! A newer open cancels the job: queued, it leaves at once; running, it stops at its next
//! check. A job that ends without handing its mosaic over, dropped from the stack past 30
//! or ending early, says so ([`Stage::Gone`]), so the engine thread never waits for it.
//!
//! Under `-d perf`, each stage's time from the `Open`: the picture with its source, the
//! metadata read, the decode's start and end; the engine thread's, the values, the upload
//! and the first live frame.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::Instant;

use apertrust_base::cache::THUMBNAILS;
use apertrust_base::jobs::{Context, Interrupted};
use apertrust_library::ops::{self as library, RawStamp, RawThumbnails, Thumbnail};
use apertrust_modules::Module;
use apertrust_plugin::{Format, ImageContext, ImageDesc, Operation};
use apertrust_raw::{Exif, Pixels, RawImage};

use super::cache::Upkeep;
use super::{Decoded, Event, Inbox, OpenPreview, Outbox, PreviewSource, SessionError};
use crate::compute::GpuImage;
use crate::edit::Edit;
use crate::export_job::{Snapshot, Stamp};
use crate::gpu::GpuContext;
use crate::history::{Clock, Log};
use crate::jobs;
use crate::names::Shot;
use crate::sidecar::{self, Edits, ViewOnly};

/// What a photo's raw says of it without its pixels: what the engine thread shows the
/// photo's values with before its decode, and the decode gives again.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Metadata {
    /// The make and the model, as the decoder names them.
    pub camera: String,
    pub image: ImageContext,
    /// The mosaic the decode gives, which the pipeline is planned for.
    pub sensor: ImageDesc,
    /// What an export's variables and the caption read of the raw beside its EXIF.
    pub shot: Shot,
}

/// A decoded raw's mosaic, in memory, before its upload.
pub(crate) struct Mosaic {
    pub width: u32,
    pub height: u32,
    pub values: Vec<u16>,
}

impl Mosaic {
    /// Uploads it as the raw stage's input.
    pub(crate) fn upload(&self, gpu: &GpuContext) -> Result<GpuImage, SessionError> {
        Ok(GpuImage::upload_mosaic(
            gpu,
            self.width,
            self.height,
            &self.values,
        )?)
    }
}

/// A raw file a job reads, mapped once for all its readings (decision 77): its metadata,
/// its embedded picture and its decode. A test's synthetic photo stands in for one.
pub(crate) trait RawFile {
    /// Its metadata without the pixels, in milliseconds; refused where it is no mosaic
    /// the raw stage develops.
    fn metadata(&self) -> Result<Metadata, SessionError>;
    /// The camera's embedded picture, upright, at most `longest` on its longer side, and
    /// where `sized` the stored size of the largest the file holds, darktable's
    /// raw-thumbnail rule's; `None` for both where it holds none.
    fn preview(&self, longest: u32, sized: bool) -> (Option<Thumbnail>, Option<(u32, u32)>);
    /// Its decode: the metadata again, from the decode, and the mosaic.
    fn decode(&self) -> Result<(Metadata, Mosaic), SessionError>;
}

/// How the session gets a photo's raw from its path: [`files`], or a synthetic photo in
/// tests.
pub(crate) type Load = Arc<dyn Fn(&Path) -> Result<Box<dyn RawFile>, SessionError> + Send + Sync>;

/// The raw files on disk, read through `apertrust-raw`.
pub(crate) fn files() -> Load {
    Arc::new(|path: &Path| {
        let file = apertrust_raw::RawFile::open(path)?;
        Ok(Box::new(File {
            file,
            path: path.to_owned(),
        }) as Box<dyn RawFile>)
    })
}

/// Decodes the raw at `path` through `load` and uploads its mosaic: an export's photo, on
/// its job's worker.
pub(crate) fn decode(gpu: &GpuContext, load: &Load, path: &Path) -> Result<Decoded, SessionError> {
    let (metadata, mosaic) = load(path)?.decode()?;
    let input = mosaic.upload(gpu)?;
    Ok(Decoded {
        image: metadata.image,
        input,
        shot: metadata.shot,
    })
}

/// A raw file on disk.
struct File {
    file: apertrust_raw::RawFile,
    path: PathBuf,
}

impl File {
    /// What the raw stage reads of `raw`, with the shooting data `exif` records, refused
    /// where it is no 16-bit mosaic.
    fn metadata_of(&self, raw: &RawImage, exif: Option<Exif>) -> Result<Metadata, SessionError> {
        let mosaic = matches!(raw.pixels, Pixels::U16(_)) && raw.components == 1;
        if !mosaic || raw.cfa.is_none() {
            return Err(SessionError::NotAMosaic);
        }
        let shot = Shot::of(raw);
        let shot = match exif {
            Some(exif) => shot.with_exif(&exif),
            None => {
                let path = self.path.display();
                tracing::warn!(target: "raw", "{path}: no shooting data");
                shot
            }
        };
        Ok(Metadata {
            // The name the camera has, an alias's own.
            camera: format!("{} {}", raw.camera.make, raw.camera.alias),
            image: crate::raw::image_context(raw),
            sensor: ImageDesc::new(raw.width, raw.height, Format::Mosaic16),
            shot,
        })
    }
}

impl RawFile for File {
    fn metadata(&self) -> Result<Metadata, SessionError> {
        let (raw, exif) = self.file.metadata()?;
        self.metadata_of(&raw, exif)
    }

    fn preview(&self, longest: u32, sized: bool) -> (Option<Thumbnail>, Option<(u32, u32)>) {
        let read = if sized {
            self.file.embedded(longest)
        } else {
            self.file.preview(longest).map(|preview| (preview, None))
        };
        match read {
            Ok((preview, size)) => {
                let picture = preview.map(|preview| Thumbnail {
                    width: preview.width,
                    height: preview.height,
                    pixels: preview.pixels.into(),
                });
                (picture, size)
            }
            Err(error) => {
                let name = sidecar::file_name(&self.path);
                tracing::debug!(target: "raw", "no embedded picture of {name}: {error}");
                (None, None)
            }
        }
    }

    fn decode(&self) -> Result<(Metadata, Mosaic), SessionError> {
        let (raw, exif) = self.file.decode()?;
        let metadata = self.metadata_of(&raw, exif)?;
        let Pixels::U16(values) = raw.pixels else {
            return Err(SessionError::NotAMosaic);
        };
        let mosaic = Mosaic {
            width: raw.width,
            height: raw.height,
            values,
        };
        Ok((metadata, mosaic))
    }
}

/// The session's one decode slot (decision 77): one raw decodes at a time beside the
/// live one, the next open waiting for it through `Context::wait`, so a cancel ends the
/// wait. It goes to those waiting in the order they asked, passing over one cancelled.
#[derive(Default)]
pub(crate) struct Decodes {
    slot: Mutex<Slot>,
}

#[derive(Default)]
struct Slot {
    taken: bool,
    /// Those waiting, each told on its channel when the slot is theirs.
    waiting: VecDeque<mpsc::Sender<()>>,
}

/// The decode slot, held until it drops.
pub(crate) struct Turn(Arc<Decodes>);

impl Drop for Turn {
    fn drop(&mut self) {
        self.0.release();
    }
}

impl Decodes {
    fn lock(&self) -> std::sync::MutexGuard<'_, Slot> {
        self.slot.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Takes the slot, waiting for it while another holds it; at cancel, gives up.
    pub(crate) fn take(self: &Arc<Self>, context: &Context<'_>) -> Result<Turn, Interrupted> {
        let (told, turn) = mpsc::channel();
        {
            let mut slot = self.lock();
            if !slot.taken {
                slot.taken = true;
                return Ok(Turn(Arc::clone(self)));
            }
            slot.waiting.push_back(told);
        }
        match context.wait(&turn) {
            Ok(()) => Ok(Turn(Arc::clone(self))),
            Err(interrupted) => {
                // A slot handed over as the cancel came goes to the next, under the lock
                // its handing takes, so it is never lost.
                let slot = self.lock();
                let handed = turn.try_recv().is_ok();
                drop(turn);
                drop(slot);
                if handed {
                    self.release();
                }
                Err(interrupted)
            }
        }
    }

    /// Hands the slot to the first still waiting, or frees it.
    fn release(&self) {
        let mut slot = self.lock();
        while let Some(next) = slot.waiting.pop_front() {
            if next.send(()).is_ok() {
                return;
            }
        }
        slot.taken = false;
    }
}

/// What an open's job hands the engine thread, naming its open.
pub(crate) enum Stage {
    /// Step 2: the metadata, the EXIF and the sidecar, or why they cannot be read.
    Read {
        open: u64,
        read: Result<Box<Read>, SessionError>,
    },
    /// Step 4 has the decode slot and begins to decode, with what steps 1 and 3 read of
    /// the raw's embedded picture, and the raw's size and modified time as step 1 read
    /// them.
    Decoding {
        open: u64,
        embedded: EmbeddedRead,
        stamp: Option<RawStamp>,
    },
    /// Step 4's decode, or its failure.
    Decoded {
        open: u64,
        decoded: Result<Box<(Metadata, Mosaic)>, SessionError>,
    },
    /// The job ended without handing over its decode: `began` false where it never ran,
    /// dropped from the stack, or true where it stopped early.
    Gone { open: u64, began: bool },
}

impl Stage {
    pub(crate) fn open(&self) -> u64 {
        match *self {
            Stage::Read { open, .. }
            | Stage::Decoding { open, .. }
            | Stage::Decoded { open, .. }
            | Stage::Gone { open, .. } => open,
        }
    }
}

/// What the open read of the raw's embedded picture, for darktable's raw-thumbnail rule:
/// nothing, where a rendered picture was its first; that there is one, read at the panel's
/// size with no more, only under never or always, whose choice at the thumbnails' sizes
/// needs no size; or its stored size, `None` where there is none, read or recorded by a
/// render of the photo with no edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum EmbeddedRead {
    #[default]
    Unread,
    Present,
    Sized(Option<(u32, u32)>),
}

/// What step 2 read: the raw's metadata and the photo's edit.
pub(crate) struct Read {
    pub metadata: Metadata,
    pub edit: Edit,
    pub hash: Option<[u8; 32]>,
    pub view_only: Option<ViewOnly>,
    /// The edit's history ([`sidecar::Loaded::log`]) and its copy's clock vector.
    pub log: Log,
    pub clocks: Vec<Clock>,
    /// The sidecar as text, which a write lays the log on, with the photo's name and
    /// fingerprint.
    pub file: Edits,
    /// The sidecar's size and modified time as read, `None` where there was none, which
    /// the edit writer compares before it writes.
    pub stamp: Option<Stamp>,
}

/// What an open's job runs with, owned, so its body borrows nothing of the engine
/// thread's.
pub(crate) struct Run {
    /// The number of the `Open`.
    pub open: u64,
    pub path: PathBuf,
    /// When the `Open` reached the session, which each stage's time counts from.
    pub started: Instant,
    /// The last `Resize`, whose longer side the embedded picture is read at.
    pub room: Option<(u32, u32)>,
    /// From which size a photo with no edit shows a render of its raw.
    pub raw: RawThumbnails,
    pub load: Load,
    pub cache: Option<Upkeep>,
    pub operations: &'static [&'static dyn Operation],
    pub modules: Vec<&'static dyn Module>,
    pub retired: &'static [&'static str],
    /// The sidecar as the edit writer reads it once it has written what it was handed.
    pub snapshot: mpsc::Receiver<Vec<Snapshot>>,
    pub decodes: Arc<Decodes>,
    pub inbox: mpsc::Sender<Inbox>,
    pub outbox: Outbox,
    /// Holds the decode, the slot taken, for a test.
    #[cfg(test)]
    pub hold: Option<crate::export_job::testing::Hold>,
}

/// The sizes of the rendered thumbnails the cache may hold, largest first.
fn largest_first() -> impl Iterator<Item = u32> {
    library::THUMBNAIL_SIZES.into_iter().rev()
}

/// The open's job (decision 77), with no progress.
pub(crate) fn job(run: Run) -> jobs::Job {
    let description = format!("opening {}", run.path.display());
    // Never equal to another open's, so a newer open of the same photo is never taken for
    // the one it replaces.
    let key = format!("open {}", run.open).into_bytes();
    let mut delivery = Delivery {
        inbox: run.inbox.clone(),
        open: run.open,
        began: false,
        done: false,
    };
    jobs::Job::new("open", description, move |context| {
        delivery.began = true;
        body(&run, context, &mut delivery);
        Ok(())
    })
    .dedup(key)
}

/// What a job hands the engine thread, which says [`Stage::Gone`] as it drops should the
/// job end, or be dropped, before its last stage.
struct Delivery {
    inbox: mpsc::Sender<Inbox>,
    open: u64,
    began: bool,
    done: bool,
}

impl Delivery {
    /// Hands over `stage`; `last` where nothing more follows.
    fn send(&mut self, stage: Stage, last: bool) {
        self.done |= last;
        // An engine that has ended hears nothing.
        let _ = self.inbox.send(Inbox::Open(Box::new(stage)));
    }
}

impl Drop for Delivery {
    fn drop(&mut self) {
        if !self.done {
            let (open, began) = (self.open, self.began);
            let _ = self
                .inbox
                .send(Inbox::Open(Box::new(Stage::Gone { open, began })));
        }
    }
}

/// The milliseconds since `started`.
fn since(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1e3
}

fn body(run: &Run, context: &Context<'_>, delivery: &mut Delivery) {
    let (open, name) = (run.open, sidecar::file_name(&run.path));
    let cancelled = || context.cancelled();
    // Step 1, the rendered thumbnail; one of the photo with no edit only where the
    // setting shows it at the panel's size, decided from the raw's facts it records, else
    // after the embedded picture where that depends on its size.
    let room = run.room.unwrap_or((u32::MAX, u32::MAX));
    let stamp = library::raw_stamp(&run.path);
    let rendered = run
        .cache
        .as_ref()
        .and_then(|cache| rendered(cache, &run.path));
    if cancelled() {
        return;
    }
    let recorded = |version: &[u8]| {
        (stamp.as_ref()).and_then(|stamp| library::unaltered_facts(version, stamp))
    };
    let mut embedded_read = EmbeddedRead::Unread;
    let (rendered, unaltered) = match rendered {
        Some((version, picture)) if run.raw.renders_at(room) => {
            if let Some(facts) = recorded(&version) {
                embedded_read = EmbeddedRead::Sized(facts.embedded);
            }
            (Some(picture), None)
        }
        Some((version, picture)) if library::rendered_unaltered(&version) => {
            match recorded(&version) {
                Some(facts) => {
                    embedded_read = EmbeddedRead::Sized(facts.embedded);
                    (run.raw.shows_render(room, &facts).then_some(picture), None)
                }
                None => (None, Some(picture)),
            }
        }
        rendered => (rendered.map(|(_, picture)| picture), None),
    };
    let sent_rendered = rendered.is_some();
    if let Some(picture) = rendered {
        preview(run, &name, Some(picture), PreviewSource::Rendered);
    }
    // Step 2, the metadata, the EXIF and the sidecar.
    let file = match (run.load)(&run.path) {
        Ok(file) => file,
        Err(error) => {
            delivery.send(
                Stage::Read {
                    open,
                    read: Err(error),
                },
                true,
            );
            return;
        }
    };
    // What darktable's raw-thumbnail rule reads of the raw beside its embedded picture.
    let mut raw = (0, 0);
    let mut phase_one = false;
    let read = file.metadata().and_then(|metadata| {
        raw = (metadata.sensor.width, metadata.sensor.height);
        phase_one = library::RawFacts::phase_one(&metadata.camera);
        edit(run, context, metadata).map(Box::new)
    });
    if cancelled() {
        return;
    }
    let failed = read.is_err();
    delivery.send(Stage::Read { open, read }, failed);
    if failed {
        return;
    }
    // Step 3, the embedded picture, where no rendered one was sent, or the render it is
    // smaller than: its stored size read, from the same decode, where step 1 found no
    // record of it and the choice may depend on it: a render of the photo with no edit
    // waiting on it, or a setting but never and always, whose choice at the thumbnails'
    // sizes needs no size.
    if !sent_rendered {
        let by_size = !matches!(run.raw, RawThumbnails::Never | RawThumbnails::Always);
        let sized = embedded_read == EmbeddedRead::Unread && (unaltered.is_some() || by_size);
        let (picture, embedded) = file.preview(room.0.max(room.1), sized);
        if cancelled() {
            return;
        }
        embedded_read = match (embedded_read, sized, &picture) {
            (EmbeddedRead::Sized(recorded), ..) => EmbeddedRead::Sized(recorded),
            (_, true, _) | (_, false, None) => EmbeddedRead::Sized(embedded),
            (_, false, Some(_)) => EmbeddedRead::Present,
        };
        let facts = library::RawFacts {
            embedded,
            raw,
            phase_one,
        };
        match unaltered {
            Some(render) if run.raw.shows_render(room, &facts) => {
                preview(run, &name, Some(render), PreviewSource::Rendered);
            }
            _ => preview(run, &name, picture, PreviewSource::Embedded),
        }
    }
    // Step 4, the decode, one at a time.
    let Ok(turn) = run.decodes.take(context) else {
        return;
    };
    // Handed the slot as its cancel came: it decodes nothing, and the slot goes on to the
    // next as the turn drops.
    if cancelled() {
        return;
    }
    let embedded = embedded_read;
    delivery.send(
        Stage::Decoding {
            open,
            embedded,
            stamp,
        },
        false,
    );
    #[cfg(test)]
    if let Some(hold) = &run.hold {
        hold.wait();
    }
    let began = since(run.started);
    let decoded = file.decode();
    let ended = since(run.started);
    drop(turn);
    tracing::debug!(target: "perf", "open {name}: decode, {began:.0} to {ended:.0} ms");
    // A cancelled open's mosaic is dropped, never uploaded.
    if cancelled() {
        return;
    }
    let decoded = decoded.map(Box::new);
    delivery.send(Stage::Decoded { open, decoded }, true);
}

/// Sends the open's picture.
fn preview(run: &Run, name: &str, picture: Option<Thumbnail>, source: PreviewSource) {
    let ms = since(run.started);
    let what = match (&picture, source) {
        (None, _) => "none".to_owned(),
        (Some(p), PreviewSource::Rendered) => format!("rendered {}×{}", p.width, p.height),
        (Some(p), PreviewSource::Embedded) => format!("embedded {}×{}", p.width, p.height),
    };
    tracing::debug!(target: "perf", "open {name}: picture {what}, at {ms:.0} ms");
    run.outbox.send(Event::OpenPreview(OpenPreview {
        open: run.open,
        picture,
        source,
    }));
}

/// The largest of the photo's rendered thumbnails the cache holds, stale or not, with its
/// version; none where it holds none or one that cannot be read.
fn rendered(cache: &Upkeep, path: &Path) -> Option<(Vec<u8>, Thumbnail)> {
    largest_first().find_map(|size| {
        let slot = library::thumbnail_slot(path, size);
        let (version, jpeg) = cache.cache().read_any(THUMBNAILS, &slot).ok()??;
        if !version.starts_with(library::RENDERED) {
            return None;
        }
        let picture = apertrust_raw::read_jpeg(&jpeg).ok()?;
        let picture = Thumbnail {
            width: picture.width,
            height: picture.height,
            pixels: picture.pixels.into(),
        };
        Some((version, picture))
    })
}

/// The photo's edit, from its sidecar as the edit writer reads it once it has written what
/// it was handed of it, clamped as an opened edit is, with its history, whether it opens for
/// viewing only, its bytes' hash and the file's stamp, and the photo's name and fingerprint
/// for the file: no edit, history, hash or stamp for a photo never edited. `metadata`, the
/// raw's, is what the edit is clamped on.
fn edit(run: &Run, context: &Context<'_>, metadata: Metadata) -> Result<Read, SessionError> {
    let sidecar = sidecar::path_for(&run.path);
    let snapshot = context
        .wait(&run.snapshot)
        .map_err(|_| SessionError::Stopped)?
        .pop();
    let modified = snapshot.as_ref().and_then(Snapshot::changed);
    let stamp = snapshot.as_ref().and_then(Snapshot::stamp);
    let mut read = Read {
        metadata,
        edit: Edit::default(),
        hash: None,
        view_only: None,
        log: Log::default(),
        clocks: Vec::new(),
        file: Edits::new(""),
        stamp,
    };
    // What the file names the photo by: its name, and its fingerprint (decision 53).
    let identify = |file: &mut Edits| {
        file.photo = Some(sidecar::file_name(&run.path).into_owned());
        if let Ok(fingerprint) = sidecar::read_fingerprint(&run.path) {
            file.fingerprint = Some(fingerprint);
        }
    };
    let xml = match snapshot.map(Snapshot::xml) {
        Some(Ok(Some(xml))) => xml,
        Some(Ok(None)) | None => {
            identify(&mut read.file);
            return Ok(read);
        }
        Some(Err(error)) => {
            return Err(SessionError::Sidecar(sidecar::SidecarError::from(error)));
        }
    };
    tracing::debug!(target: "pipe", "read {}", sidecar::file_name(&sidecar));
    let start = std::time::Instant::now();
    let loaded = sidecar::from_xml_at(&xml, modified, run.operations, &run.modules, run.retired)?;
    let ms = start.elapsed().as_secs_f64() * 1e3;
    let steps = loaded.log.steps().len();
    tracing::debug!(target: "perf", "read {}: {steps} steps in {ms:.1} ms", sidecar::file_name(&sidecar));
    for warning in &loaded.warnings {
        tracing::warn!(target: "pipe", "{}: {warning}", sidecar.display());
    }
    for problem in &loaded.problems {
        tracing::warn!(target: "pipe", "{}: {problem}", sidecar.display());
    }
    let view_only = loaded.view_only();
    if let Some(view_only) = &view_only {
        let (name, by) = (
            sidecar::file_name(&sidecar),
            sidecar::writer(&view_only.build),
        );
        tracing::warn!(target: "pipe", "view only: {name}, by {by}");
    }
    read.clocks = loaded.clocks().to_vec();
    let sidecar::Loaded {
        mut edit,
        log,
        mut file,
        ..
    } = loaded;
    // A number outside the range its module's values give, as only a hand-written file
    // holds, is clamped as it is read (decisions 21 and 30); an edit open for viewing only
    // is shown as it was. The clamp is a change not yet a step.
    if view_only.is_none() {
        for warning in edit.clamp_read(&run.modules, &read.metadata.image) {
            tracing::warn!(target: "pipe", "{warning}");
        }
        identify(&mut file);
    }
    Ok(Read {
        edit,
        hash: Some(apertrust_library::index::hash(xml.as_bytes())),
        view_only,
        log,
        file,
        ..read
    })
}

/// A test's synthetic raws, read as files are: the engine's tests', and its clients' through
/// [`super::testing`].
#[cfg(any(test, feature = "testing"))]
pub(crate) mod testing {
    use super::*;

    /// What a test's synthetic photo gives for its path: its metadata and its mosaic.
    pub(crate) type Develop = fn(&Path) -> Result<(Metadata, Mosaic), SessionError>;

    /// The raws `develop` gives, whatever the path: the metadata read the decode's, an
    /// embedded picture only for a raw called `embedded.ARW`, 64 × 48 pixels made no larger
    /// than the longer side asked, and a decode that fails for one called
    /// `undecodable.ARW`, whose metadata reads.
    pub(crate) fn loading(develop: Develop) -> Load {
        Arc::new(move |path: &Path| {
            Ok(Box::new(Synthetic {
                path: path.to_owned(),
                develop,
            }) as Box<dyn RawFile>)
        })
    }

    /// `image`'s photo on a mosaic of `width` × `height` photosites holding `values`.
    pub(crate) fn photo(
        image: ImageContext,
        [width, height]: [u32; 2],
        values: Vec<u16>,
    ) -> (Metadata, Mosaic) {
        let metadata = Metadata {
            camera: "Synthetic".to_owned(),
            image,
            sensor: ImageDesc::new(width, height, Format::Mosaic16),
            shot: Shot::default(),
        };
        let mosaic = Mosaic {
            width,
            height,
            values,
        };
        (metadata, mosaic)
    }

    struct Synthetic {
        path: PathBuf,
        develop: Develop,
    }

    impl RawFile for Synthetic {
        fn metadata(&self) -> Result<Metadata, SessionError> {
            (self.develop)(&self.path).map(|(metadata, _)| metadata)
        }

        fn preview(&self, longest: u32, sized: bool) -> (Option<Thumbnail>, Option<(u32, u32)>) {
            if !self.path.ends_with("embedded.ARW") {
                return (None, None);
            }
            let width = longest.min(64);
            let height = width * 3 / 4;
            let pixels = vec![200; (width * height * 4) as usize];
            let picture = Thumbnail {
                width,
                height,
                pixels: pixels.into(),
            };
            (Some(picture), sized.then_some((64, 48)))
        }

        fn decode(&self) -> Result<(Metadata, Mosaic), SessionError> {
            if self.path.ends_with("undecodable.ARW") {
                let error = apertrust_raw::Error::Decode("a test's undecodable raw".to_owned());
                return Err(error.into());
            }
            (self.develop)(&self.path)
        }
    }
}
