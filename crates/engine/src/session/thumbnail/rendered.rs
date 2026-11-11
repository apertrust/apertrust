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

//! Rendered thumbnails of the live photo, made from its saved edit once that has settled
//! and as it stops being live (`docs/engine.md`, Rendered thumbnails of the live photo).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant};

use apertrust_base::cache::THUMBNAILS;
use apertrust_library::ops::{
    self as library, LibraryEvent, RawThumbnails, Thumbnail, ThumbnailKey,
};

use super::Upkeep;
use crate::edit::{self, Edit};
use crate::export_job::Snapshot;
use crate::jobs::{self, Context, JobId, Queue, Tasks};
use crate::pipeline::{self, Pipe, Pipeline, Plan, Sources, Still, StillError};
use crate::session::open::EmbeddedRead;
use crate::session::{Engine, Event, Load, Open, Outbox};

/// How long after the last successful write of the live photo's sidecar its thumbnail is
/// made.
pub const THUMBNAIL_SETTLE: Duration = Duration::from_secs(1);

/// How long after a failed preparation or write the same saved edit is tried again; a
/// departure tries at once.
const RETRY_AFTER: Duration = Duration::from_secs(30);

/// The sizes the client asked of each photo of the current folder, by path, with the
/// folder's open.
type Asked = Arc<Mutex<BTreeMap<PathBuf, (u64, BTreeSet<u32>)>>>;

/// A thumbnail job under way, for the saved edit of `hash`.
struct Running {
    hash: [u8; 32],
    job: JobId,
    /// What its still reads of the live pipe in place.
    sources: Sources,
}

#[derive(Default)]
pub(crate) struct State {
    asked: Asked,
    /// The saved edit of each photo nothing is left to do for: its thumbnails written,
    /// found in the cache already, or refused.
    done: Arc<Mutex<BTreeMap<PathBuf, [u8; 32]>>>,
    running: BTreeMap<PathBuf, Running>,
    /// When the saved edit of each photo may be tried again without a departure: after a
    /// failure, or after a live frame cancelled its still.
    not_before: BTreeMap<PathBuf, ([u8; 32], Instant)>,
    pub(in crate::session) generations: BTreeMap<PathBuf, Arc<AtomicU64>>,
    /// How many stills were prepared, for a test.
    #[cfg(test)]
    pub(in crate::session) stills: usize,
}

impl State {
    /// A different saved edit makes every earlier completion and attempt obsolete.
    pub(in crate::session) fn invalidate(&mut self, path: &Path) {
        if let Some(generation) = self.generations.get(path) {
            generation.fetch_add(1, Ordering::Relaxed);
        }
        self.done
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(path);
        self.not_before.remove(path);
    }

    pub(crate) fn asked(&mut self, open: u64, folder: &Path, photo: &str, size: u32) {
        if Path::new(photo).file_name() != Some(std::ffi::OsStr::new(photo)) {
            return;
        }
        let mut asked = self.asked.lock().unwrap_or_else(PoisonError::into_inner);
        let entry = asked
            .entry(folder.join(photo))
            .or_insert_with(|| (open, BTreeSet::new()));
        entry.0 = open;
        entry.1.insert(size);
    }

    /// Before a live frame of the photo at `path` is submitted, its pipeline updated:
    /// cancels its still where the frame writes what the still reads in place. An already
    /// submitted read precedes the frame on the same queue, and no later step runs. It is
    /// tried again once the saved edit has settled after this frame, or as the photo
    /// leaves.
    pub(in crate::session) fn before_frame(
        &mut self,
        path: &Path,
        control: &apertrust_base::jobs::Control,
        pipeline: &Pipeline,
    ) {
        self.forget_ended(control);
        let Some(running) = self.running.get(path) else {
            return;
        };
        if !pipeline.writes(&running.sources) {
            return;
        }
        control.cancel(running.job);
        if let Some(generation) = self.generations.get(path) {
            generation.fetch_add(1, Ordering::Relaxed);
        }
        let hash = running.hash;
        self.running.remove(path);
        let at = Instant::now() + THUMBNAIL_SETTLE;
        self.not_before.insert(path.to_owned(), (hash, at));
    }

    /// The photo at `path` leaves: its still under way submits what it reads of the live
    /// pipe at once, which nothing live writes any more. The job of that still, to wake
    /// its task where it sleeps.
    pub(in crate::session) fn depart(&self, path: &Path) -> Option<JobId> {
        let running = self.running.get(path)?;
        running.sources.hurry();
        Some(running.job)
    }

    /// Forgets the jobs that have ended, their stills read through or never run.
    fn forget_ended(&mut self, control: &apertrust_base::jobs::Control) {
        self.running.retain(|_, running| {
            matches!(
                control.state(running.job),
                Some(jobs::State::Queued | jobs::State::Running)
            )
        });
    }

    #[cfg(test)]
    pub(in crate::session) fn hurried(&self, path: &Path) -> bool {
        (self.running.get(path)).is_some_and(|running| running.sources.hurried())
    }

    /// Whether the still of the photo at `path` reads nothing of the live pipe any more.
    #[cfg(test)]
    pub(in crate::session) fn released(&self, path: &Path) -> bool {
        (self.running.get(path)).is_some_and(|running| running.sources.released())
    }

    #[cfg(test)]
    pub(in crate::session) fn job(&self, path: &Path) -> Option<JobId> {
        self.running.get(path).map(|running| running.job)
    }

    /// Under another setting, every photo's thumbnail is chosen again.
    pub(in crate::session) fn forget_done(&mut self) {
        self.done
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
        self.not_before.clear();
    }

    pub(crate) fn clear_asked(&mut self) {
        self.asked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    pub(crate) fn cancel_all(&mut self, control: &apertrust_base::jobs::Control) {
        for generation in self.generations.values() {
            generation.fetch_add(1, Ordering::Relaxed);
        }
        for running in self.running.values() {
            control.cancel(running.job);
        }
    }

    fn done(&self, path: &Path, hash: &[u8; 32]) -> bool {
        self.done
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(path)
            == Some(hash)
    }

    fn finish(&self, path: &Path, hash: [u8; 32]) {
        self.done
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(path.to_owned(), hash);
    }
}

/// What a photo's thumbnail is made from: the viewport pipe at 1024 pixels on its longer
/// side, its output sRGB; `unaltered` where the edit renders as a photo with no edit does,
/// which shows its render only where darktable's raw-thumbnail rule does.
pub(in crate::session) struct ThumbnailPlan {
    pub plan: Box<Plan>,
    pub unaltered: bool,
}

impl Engine {
    /// The live photo's saved edit as a thumbnail sees it: the photo's path, the hash of
    /// its sidecar's bytes, and when the writer said it was saved, or `None` for one read
    /// as the photo opened. Before an acknowledgement, only as the photo leaves, by the
    /// hash of what was handed over. `None` while nothing can be made: no cache, the photo
    /// opening, viewed only, with a change not handed over, a write that failed, or never
    /// edited.
    fn thumbnail_saved(&self, leaving: bool) -> Option<(PathBuf, [u8; 32], Option<Instant>)> {
        let open = self.open.as_ref()?;
        if self.cache.is_none() || open.input.is_none() || open.view_only.is_some() || open.unsaved
        {
            return None;
        }
        let path = open.sidecar.with_extension("");
        if let Some((hash, at)) = self.writer.saved_for(&open.sidecar, open.thumbnail_through) {
            return Some((path, hash, Some(at)));
        }
        if open.thumbnail_through.is_none() {
            // Never edited: rendered as its defaults where the setting renders those.
            // A photo never edited is rendered as its defaults, where the rule says.
            let hash = open.thumbnail_hash.unwrap_or(library::UNALTERED);
            return Some((path, hash, open.thumbnail_saved_at));
        }
        let hash = open.thumbnail_hash?;
        // Handed over and not yet acknowledged: the writer's read barrier checks the bytes
        // before the GPU starts.
        (leaving && !self.writer.failed(&open.sidecar)).then_some((path, hash, None))
    }

    /// How long the engine thread may wait for commands before the live photo's thumbnail
    /// is due: until its saved edit has settled, or until a failure may be tried again;
    /// `None` where nothing is due until something else happens, a command, the writer's
    /// acknowledgement or a job's end.
    pub(in crate::session) fn thumbnail_wait(&self) -> Option<Duration> {
        let (path, hash, saved) = self.thumbnail_saved(false)?;
        let state = &self.rendered_thumbnails;
        if state.done(&path, &hash) {
            return None;
        }
        let retry = (state.not_before.get(&path))
            .filter(|(tried, _)| *tried == hash)
            .map(|(_, at)| *at);
        // A job under way wakes nothing as it fails: the retry's time is a wake of its own.
        let at = match state.running.get(&path) {
            Some(running) if running.hash == hash && self.thumbnail_running(running.job) => retry?,
            _ => (saved? + THUMBNAIL_SETTLE).max(retry.unwrap_or(Instant::now())),
        };
        Some(at.saturating_duration_since(Instant::now()))
    }

    fn thumbnail_running(&self, job: JobId) -> bool {
        matches!(
            self.jobs.control.state(job),
            Some(jobs::State::Queued | jobs::State::Running)
        )
    }

    /// Once per turn, or before the live photo leaves (`leaving`): makes its thumbnail
    /// where it is due. The writer's read barrier covers a departure's last gesture: no GPU
    /// work starts until that edit is on disk, and a failed write makes no thumbnail.
    pub(in crate::session) fn thumbnail_tick(&mut self, leaving: bool) {
        self.rendered_thumbnails.forget_ended(&self.jobs.control);
        let Some((path, hash, saved)) = self.thumbnail_saved(leaving) else {
            return;
        };
        let state = &self.rendered_thumbnails;
        if state.done(&path, &hash) {
            return;
        }
        if let Some(running) = state.running.get(&path) {
            if running.hash == hash && self.thumbnail_running(running.job) {
                return;
            }
            self.jobs.control.cancel(running.job);
        }
        if !leaving {
            let settled = saved.is_some_and(|at| at.elapsed() >= THUMBNAIL_SETTLE);
            let waiting = (state.not_before.get(&path))
                .is_some_and(|(tried, at)| *tried == hash && Instant::now() < *at);
            if !settled || waiting {
                return;
            }
        }
        self.start_thumbnail(path, hash);
    }

    /// The plan of the live photo's thumbnail, from its edit as it stands.
    pub(in crate::session) fn thumbnail_plan(&self, open: &Open) -> Result<ThumbnailPlan, String> {
        let bound = edit::instances(&open.edit, self.operations, &self.modules, &open.image);
        if let Some((module, error)) = bound.failed.first() {
            return Err(format!("module {module} left out: {error}"));
        }
        let defaults = edit::instances(
            &Edit::default(),
            self.operations,
            &self.modules,
            &open.image,
        );
        let unchanged = bound.instances.len() == defaults.instances.len()
            && (bound.instances.iter().zip(&defaults.instances)).all(|(a, b)| {
                let base = a.operation.defaults(&open.image);
                let schema = a.operation.schema();
                a.operation.manifest().id == b.operation.manifest().id
                    && matches!(
                        (schema.resolve_over(&base, &a.params), schema.resolve_over(&base, &b.params)),
                        (Ok(actual), Ok(expected)) if actual == expected
                    )
            });
        let mut instances = bound.instances;
        // Thumbnails are sRGB, as their JPEG says, whatever the monitor and whatever output
        // profile the edit chose.
        if let Some(output) = instances
            .iter_mut()
            .find(|op| op.operation.manifest().id == "colorout")
        {
            let srgb = match output
                .operation
                .schema()
                .param("type")
                .map(|param| param.kind)
            {
                Some(apertrust_plugin::Kind::Choice { choices, .. }) => {
                    choices.iter().position(|choice| *choice == "srgb")
                }
                _ => None,
            };
            let srgb = srgb.ok_or("colorout has no sRGB profile")?;
            output
                .params
                .set("type", apertrust_plugin::Value::Choice(srgb as u32));
        }
        let pipe = Pipe::Viewport {
            width: 1024,
            height: 1024,
        };
        pipeline::plan(&instances, &open.image, open.sensor, pipe, &self.limits)
            .map(|plan| ThumbnailPlan {
                plan: Box::new(plan),
                unaltered: unchanged,
            })
            .map_err(|error| error.to_string())
    }

    /// Starts the live photo's thumbnail job for its saved edit of `hash`, unless the cache
    /// holds that render already or it cannot be made.
    fn start_thumbnail(&mut self, path: PathBuf, hash: [u8; 32]) {
        let Some(upkeep) = self.cache.clone() else {
            return;
        };
        let photo = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let cache = upkeep.cache();
        let version = |size| {
            (cache
                .version(THUMBNAILS, &library::thumbnail_slot(&path, size))
                .ok())
            .flatten()
        };
        let Some(open) = self.open.as_ref() else {
            return;
        };
        let raw = self.raw_thumbnails;
        let plan = self.thumbnail_plan(open);
        // What darktable's rule shows at each size of the photo with no edit, from what its
        // open read of the raw: a render, the embedded picture, or `None` where only the
        // embedded picture's unread size would tell.
        let shows = |size: u32| -> Option<bool> {
            let asked = (size, size);
            if open.phase_one || raw.renders_at(asked) {
                return Some(true);
            }
            let facts = |embedded| library::RawFacts {
                embedded,
                raw: (open.sensor.width, open.sensor.height),
                phase_one: open.phase_one,
            };
            match open.embedded {
                EmbeddedRead::Sized(size) => Some(raw.shows_render(asked, &facts(size))),
                EmbeddedRead::Present if raw == RawThumbnails::Never => Some(false),
                EmbeddedRead::Present | EmbeddedRead::Unread => None,
            }
        };
        let sizes = library::THUMBNAIL_SIZES;
        // What this saved edit made before, in this session or an earlier one: the render
        // at 1024; for a photo with no edit, each size as the rule shows it, a render that
        // records the facts of the raw the open read.
        let current = |version: &[u8]| {
            (open.stamp.as_ref())
                .is_some_and(|stamp| library::unaltered_facts(version, stamp).is_some())
        };
        let made = match &plan {
            Ok(ThumbnailPlan {
                unaltered: false, ..
            }) => version(1024).is_some_and(|version| library::rendered_matches(&version, &hash)),
            Ok(ThumbnailPlan {
                unaltered: true, ..
            }) => sizes.iter().all(|&size| {
                version(size).is_some_and(|version| match shows(size) {
                    Some(true) => current(&version),
                    Some(false) => version.starts_with(library::EMBEDDED),
                    None => false,
                })
            }),
            Err(_) => false,
        };
        if made {
            tracing::debug!(target: "perf", "thumbnail of {photo}: cached already");
            self.rendered_thumbnails.finish(&path, hash);
            return;
        }
        // A still only where a size may show a render: none for a photo with no edit
        // whose every size shows the embedded picture, as under never.
        let renders = sizes.iter().any(|&size| shows(size) != Some(false));
        let (plan, unaltered) = match plan {
            Ok(ThumbnailPlan { plan, unaltered }) => (*plan, unaltered),
            Err(error) => {
                tracing::warn!(target: "pipe", "no thumbnail of {photo}: {error}");
                self.rendered_thumbnails.finish(&path, hash);
                return;
            }
        };
        let facts = ((open.sensor.width, open.sensor.height), open.phase_one);
        let stamp = open.stamp;
        let state = &mut self.rendered_thumbnails;
        let retry = Instant::now() + RETRY_AFTER;
        // None after a failed render: tried again later, never every turn.
        let Some(pipeline) = self.open.as_mut().and_then(|open| open.pipeline.as_mut()) else {
            state.not_before.insert(path, (hash, retry));
            return;
        };
        let prepared = if !unaltered || renders {
            #[cfg(test)]
            {
                state.stills += 1;
            }
            Some(pollster::block_on(pipeline.still(
                &self.gpu,
                plan,
                self.limits.budget,
            )))
        } else {
            None
        };
        let still = match prepared {
            None => None,
            Some(Ok(still)) => Some(still),
            Some(Err(StillError::Refused(why))) => {
                tracing::warn!(target: "pipe", "no thumbnail of {photo}: {why}");
                // A photo with no edit still gets its embedded picture in place of a
                // render left from before.
                if !unaltered {
                    state.finish(&path, hash);
                    return;
                }
                None
            }
            Some(Err(StillError::Compute(error))) => {
                tracing::warn!(target: "pipe", "cannot prepare thumbnail: {error}");
                state.not_before.insert(path, (hash, retry));
                return;
            }
        };
        let Some(open) = self.open.as_ref() else {
            return;
        };
        state.not_before.insert(path.clone(), (hash, retry));
        let generation = state.generations.entry(path.clone()).or_default().clone();
        let at = generation.fetch_add(1, Ordering::Relaxed) + 1;
        let canonical = apertrust_library::index::hash(
            crate::sidecar::to_xml(&open.edit, self.operations, &self.modules, &self.build)
                .as_bytes(),
        );
        // A person editing goes first.
        let still = still.map(|(mut still, sources)| {
            still.yield_to(Arc::clone(&self.last_change));
            (still, sources)
        });
        let (still, sources) = still.unzip();
        let render = Render {
            folder: path.parent().unwrap_or(Path::new("")).to_owned(),
            photo,
            path: path.clone(),
            hash,
            canonical,
            snapshots: self.writer.read(vec![open.sidecar.clone()]),
            upkeep,
            asked: Arc::clone(&state.asked),
            done: Arc::clone(&state.done),
            outbox: self.jobs.outbox.clone(),
            generation,
            at,
            unaltered,
            raw,
            facts,
            stamp,
            load: Arc::clone(&self.load),
        };
        let description = format!("rendered thumbnail of {}", render.photo);
        let dedup = [path.as_os_str().as_encoded_bytes(), &hash].concat();
        let job = self.jobs.add(0, Queue::SystemForeground, |tasks| {
            jobs::Job::new("rendered-thumbnail", description, move |context| {
                render.run(context, &tasks, still);
                Ok(())
            })
            .dedup(dedup)
        });
        let sources = sources.unwrap_or_default();
        state.running.insert(path, Running { hash, job, sources });
    }
}

/// A thumbnail job's body: what it renders or reads, writes and tells.
struct Render {
    folder: PathBuf,
    photo: String,
    path: PathBuf,
    hash: [u8; 32],
    /// The hash of the edit as this build writes it, which a held write's bytes may differ
    /// from.
    canonical: [u8; 32],
    snapshots: mpsc::Receiver<Vec<Snapshot>>,
    upkeep: Upkeep,
    asked: Asked,
    done: Arc<Mutex<BTreeMap<PathBuf, [u8; 32]>>>,
    outbox: Outbox,
    generation: Arc<AtomicU64>,
    at: u64,
    /// Whether the edit renders as a photo with no edit does: its render is shown only
    /// where `raw` shows one, and versioned as [`library::UNALTERED`].
    unaltered: bool,
    raw: RawThumbnails,
    /// The raw's size and whether Phase One made it, for darktable's rule.
    facts: ((u32, u32), bool),
    /// The raw's size and modified time as the open read them: its facts are recorded only
    /// for that raw, never for one replaced since.
    stamp: Option<library::RawStamp>,
    /// How the raw is read, for its embedded picture.
    load: Load,
}

impl Render {
    /// Waits for the saved edit to be on disk, renders it with `still`, reads the camera's
    /// embedded picture where there is none or the edit is at the defaults, writes every
    /// size, each the render or the embedded picture as the setting shows it, and tells
    /// the sizes the client asked for.
    fn run(self, context: &Context<'_>, tasks: &Tasks, still: Option<Still>) {
        let current = || !context.cancelled() && self.generation.load(Ordering::Relaxed) == self.at;
        let cancel = || !current();
        let Ok(mut snapshots) = context.wait(&self.snapshots) else {
            return;
        };
        // The edit's bytes, none for a photo never edited, which a render of its defaults
        // stands for.
        let actual = match snapshots.pop().map(|snapshot| snapshot.xml()) {
            Some(Ok(Some(xml))) => apertrust_library::index::hash(xml.as_bytes()),
            Some(Ok(None)) if self.hash == library::UNALTERED => library::UNALTERED,
            _ => return,
        };
        if !current() || (actual != self.hash && actual != self.canonical) {
            return;
        }
        let photo = &self.photo;
        // The still first, so a departed photo's hurried reads of the live pipe go first.
        let rendered = match still {
            Some(still) => {
                let started = Instant::now();
                let (size, pixels) = match tasks.run(context, still) {
                    Ok(picture) => picture,
                    Err(error) => {
                        if current() {
                            tracing::warn!(target: "pipe", "cannot render thumbnail: {error}");
                        }
                        return;
                    }
                };
                let ms = started.elapsed().as_secs_f64() * 1e3;
                tracing::debug!(target: "perf", "thumbnail of {photo} at 1024: rendered in {ms:.1} ms");
                Some(rgba(size, &pixels))
            }
            None => None,
        };
        // Then the camera's embedded picture, read once at the largest size, with the
        // stored size of the largest, for a photo with no edit; none that cannot be read,
        // or of a raw replaced since the open, whose render then records no facts, so a
        // later read looks at the raw again.
        let stamp = library::raw_stamp(&self.path).filter(|now| Some(*now) == self.stamp);
        let (read, (embedded, embedded_size)) = match (self.load)(&self.path) {
            Ok(file) if self.unaltered => (true, file.preview(1024, true)),
            Ok(_) => (true, (None, None)),
            Err(_) => (false, (None, None)),
        };
        let (raw, phase_one) = self.facts;
        let facts = library::RawFacts {
            embedded: embedded_size,
            raw,
            phase_one,
        };
        // A photo with no edit shows its render only at the sizes darktable's rule does.
        let shows = |size| !self.unaltered || self.raw.shows_render((size, size), &facts);
        let version = match (self.unaltered, stamp.filter(|_| read)) {
            (true, Some(stamp)) => library::unaltered_version(&facts, &stamp),
            (true, None) => library::rendered_version(&library::UNALTERED),
            (false, _) => library::rendered_version(&actual),
        };
        let writing = Instant::now();
        let mut all_written = true;
        for size in library::THUMBNAIL_SIZES.into_iter().rev() {
            if !current() {
                return;
            }
            let render = rendered.as_ref().filter(|_| shows(size));
            let upkeep = &self.upkeep;
            let image;
            all_written &= match (render, &embedded) {
                (Some(render), _) => {
                    image = Some(average(render, size));
                    let key = ThumbnailKey {
                        slot: library::thumbnail_slot(&self.path, size),
                        version: version.clone(),
                    };
                    let picture = image.as_ref().expect("a render");
                    super::write_if_current(upkeep, &key, picture, photo, &cancel)
                }
                (None, Some(embedded)) => {
                    image = Some(average(embedded, size));
                    let picture = image.as_ref().expect("an embedded picture");
                    library::thumbnail_key(&self.folder, photo, size).is_some_and(|key| {
                        super::replace_with_embedded(upkeep, &key, picture, photo, &cancel)
                    })
                }
                (None, None) => {
                    image = None;
                    super::remove_rendered_if_current(upkeep, &self.path, size, &cancel)
                }
            };
            super::publish_if_current(&cancel, || {
                let asked = self.asked.lock().unwrap_or_else(PoisonError::into_inner);
                if let Some((open, sizes)) = asked.get(&self.path)
                    && sizes.contains(&size)
                {
                    self.outbox.send(Event::Library(LibraryEvent::Thumbnail {
                        open: *open,
                        photo: photo.clone(),
                        size,
                        image,
                    }));
                }
            });
        }
        let ms = writing.elapsed().as_secs_f64() * 1e3;
        tracing::debug!(target: "perf", "thumbnail of {photo}: JPEG writes in {ms:.1} ms");
        if all_written {
            let mut done = self.done.lock().unwrap_or_else(PoisonError::into_inner);
            if current() {
                done.insert(self.path, self.hash);
            }
        }
    }
}

/// The still's pixels as 8-bit sRGB, quantised as an export's JPEG is.
pub(in crate::session) fn rgba([width, height]: [u32; 2], pixels: &[[f32; 4]]) -> Thumbnail {
    let pixels = pixels
        .iter()
        .flat_map(|pixel| {
            [
                apertrust_export::quantise::eight_bit(pixel[0]),
                apertrust_export::quantise::eight_bit(pixel[1]),
                apertrust_export::quantise::eight_bit(pixel[2]),
                255,
            ]
        })
        .collect::<Vec<_>>();
    Thumbnail {
        width,
        height,
        pixels: pixels.into(),
    }
}

/// Box averaging of the 1024 render in encoded sRGB, including partial edge pixels.
pub(in crate::session) fn average(image: &Thumbnail, longest: u32) -> Thumbnail {
    let (width, height) = pipeline::fit((image.width, image.height), (longest, longest));
    if (width, height) == (image.width, image.height) {
        return image.clone();
    }
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let (left, right) = (
                x as f64 * image.width as f64 / width as f64,
                (x + 1) as f64 * image.width as f64 / width as f64,
            );
            let (top, bottom) = (
                y as f64 * image.height as f64 / height as f64,
                (y + 1) as f64 * image.height as f64 / height as f64,
            );
            let mut sum = [0.0; 3];
            for sy in top.floor() as u32..(bottom.ceil() as u32).min(image.height) {
                for sx in left.floor() as u32..(right.ceil() as u32).min(image.width) {
                    let weight = (right.min((sx + 1) as f64) - left.max(sx as f64))
                        * (bottom.min((sy + 1) as f64) - top.max(sy as f64));
                    let at = ((sy * image.width + sx) * 4) as usize;
                    for (channel, sum) in sum.iter_mut().enumerate() {
                        *sum += image.pixels[at + channel] as f64 * weight;
                    }
                }
            }
            let area = (right - left) * (bottom - top);
            pixels.extend(sum.map(|value| (value / area).round() as u8));
            pixels.push(255);
        }
    }
    Thumbnail {
        width,
        height,
        pixels: pixels.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smaller_slots_average_all_source_pixels_with_partial_edges() {
        let image = Thumbnail {
            width: 3,
            height: 1,
            pixels: vec![0, 0, 0, 255, 120, 120, 120, 255, 240, 240, 240, 255].into(),
        };
        let two = average(&image, 2);
        assert_eq!((two.width, two.height), (2, 1));
        assert_eq!(&*two.pixels, &[40, 40, 40, 255, 200, 200, 200, 255]);
        assert_eq!(&*average(&image, 1).pixels, &[120, 120, 120, 255]);
        assert_eq!(average(&image, 1024), image);
    }

    #[test]
    fn an_edit_reversal_invalidates_old_completion_and_retry_delay() {
        let mut state = State::default();
        let path = PathBuf::from("photo.ARW");
        let hash = [1; 32];
        let generation = Arc::new(AtomicU64::new(2));
        state
            .generations
            .insert(path.clone(), Arc::clone(&generation));
        state.finish(&path, hash);
        state
            .not_before
            .insert(path.clone(), (hash, Instant::now()));
        state.invalidate(&path);
        assert_eq!(generation.load(Ordering::Relaxed), 3);
        assert!(!state.done(&path, &hash));
        assert!(!state.not_before.contains_key(&path));
    }
}
