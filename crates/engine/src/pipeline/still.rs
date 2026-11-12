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

//! A still of the live viewport pipe: its saved edit rendered at a thumbnail's size, one
//! background turn at a time, reading the live pipe's cached images and whole-image
//! results in place and writing only buffers of its own (`docs/engine.md`, Rendered
//! thumbnails of the live photo).

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use super::*;
use crate::jobs::{GpuTask, Step};

/// The source bytes one band of a still's downscale reads at most, so that no turn holds
/// the live frame back much longer than one of the frame's own passes: the 61 MP raw
/// stage's image, 963 MB, in 16 bands.
const BAND_BYTES: u64 = 64 << 20;

#[cfg(test)]
thread_local! {
    /// In place of [`BAND_BYTES`] on a test's thread, to split a small image into bands.
    pub(crate) static TEST_BAND_BYTES: std::cell::Cell<Option<u64>> =
        const { std::cell::Cell::new(None) };
}

/// [`BAND_BYTES`], or a test's own.
fn band_bytes() -> u64 {
    #[cfg(test)]
    if let Some(bytes) = TEST_BAND_BYTES.get() {
        return bytes;
    }
    BAND_BYTES
}

/// How long after the live photo's last change a still's turns wait, so that a person
/// editing, who may resume mid-render, never has a frame wait behind one: a turn such as
/// haze removal's passes after its whole-image result cannot be split.
pub(crate) const EDITING_QUIET: Duration = Duration::from_millis(150);

/// When the live photo last changed, which a still yields to; the session sets it with
/// each live frame.
pub(crate) type LastChange = Arc<Mutex<Option<Instant>>>;

/// The live pipe's saved edit at a thumbnail's size, each command one background turn: a
/// band of the downscale, a dispatch, or all of an operation's dispatches after a
/// whole-image result it reads in place, so that no live frame runs between them.
pub(crate) struct Still {
    commands: VecDeque<wgpu::CommandBuffer>,
    /// How many of `commands`, from the first, read the live pipe's buffers: once they are
    /// submitted, its [`Sources`] hold no live frame back.
    reading: usize,
    released: Arc<AtomicBool>,
    /// Set as its photo leaves: no live frame is left to wait for, so the commands that
    /// read the live pipe go at once, and its buffers are freed as soon as the GPU is done.
    hurry: Arc<AtomicBool>,
    /// When the live photo last changed: no turn is taken within [`EDITING_QUIET`] of it.
    yields_to: Option<LastChange>,
    readback: wgpu::Buffer,
    mapped: Option<mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>>,
    size: [u32; 2],
    /// Makes the equality test prove it exercised cached-result reuse.
    #[cfg(test)]
    pub(crate) reused_results: usize,
    /// How many bands its downscale runs in, for the equality test.
    #[cfg(test)]
    pub(crate) bands: usize,
}

/// What a still reads of the live pipe in place, by key: cache points' images and
/// operations' whole-image results. Until the still's last command reading them is
/// submitted, its owner cancels it before a live frame that writes one
/// ([`Pipeline::writes`]).
#[derive(Debug, Clone, Default)]
pub(crate) struct Sources {
    images: Vec<u64>,
    results: Vec<(&'static str, Reused)>,
    released: Arc<AtomicBool>,
    hurry: Arc<AtomicBool>,
}

/// A whole-image result a still reads: all of an operation's whole-image passes', or
/// their full-resolution part's, by its key.
#[derive(Debug, Clone, Copy)]
enum Reused {
    Whole(u64),
    Part(u64),
}

impl Sources {
    /// Whether the still reads nothing of the live pipe any more, or never did.
    pub(crate) fn released(&self) -> bool {
        (self.images.is_empty() && self.results.is_empty()) || self.released.load(Ordering::Relaxed)
    }

    /// Its photo has left: the still submits what it still reads of the live pipe in its
    /// next turn, all at once.
    pub(crate) fn hurry(&self) {
        self.hurry.store(true, Ordering::Relaxed);
    }

    #[cfg(test)]
    pub(crate) fn hurried(&self) -> bool {
        self.hurry.load(Ordering::Relaxed)
    }
}

/// Why a still was not made.
#[derive(Debug, Error)]
pub(crate) enum StillError {
    /// It never can be for this plan and live pipe: no cache point holds the edit's raw
    /// stage, a binding or a buffer is too large, or its own buffers are past the budget.
    #[error("{0}")]
    Refused(String),
    /// Compiling a kernel failed, which may not happen again.
    #[error(transparent)]
    Compute(#[from] ComputeError),
}

impl Pipeline {
    /// Records a still of `plan`, the viewport pipe at a thumbnail's size, without a second
    /// pipeline or decode: it starts after the newest cache point whose image the live pipe
    /// holds at the same key and size, and skips the whole-image passes whose result the
    /// live pipe holds at the same keys, reading both in place ([`Sources`]). Its own
    /// working buffers, scratch, uniforms and tables keep it from writing anything of the
    /// live pipe's. Nothing is submitted here. `budget` bounds the bytes it makes.
    pub(crate) async fn still(
        &mut self,
        gpu: &GpuContext,
        plan: Plan,
        budget: u64,
    ) -> Result<(Still, Sources), StillError> {
        // The cache points the live pipe holds at the still's keys and sizes.
        let mut first = 0;
        let mut cached = Vec::new();
        for (index, op) in plan.operations.iter().enumerate() {
            for (&old, held) in self.plan.cached.iter().zip(&self.held) {
                let previous = &self.plan.operations[old];
                if *held == Some(op.key) && previous.output_desc == op.output_desc {
                    first = index + 1;
                    cached.push((op.output, self.image(previous.output).clone(), op.key));
                }
            }
        }
        if first == 0 {
            return Err(StillError::Refused(
                "the live pipe holds no cache point of its raw stage".to_owned(),
            ));
        }
        // Whole-image results, or their full-resolution parts, by the keys a live rebuild
        // reuses them by: the scratch they leave that the passes after them read is bound
        // as the live pipe's.
        let mut skipped = vec![0; plan.operations.len()];
        let mut shared: Vec<Option<wgpu::Buffer>> = vec![None; plan.scratch.len()];
        let mut results = Vec::new();
        for (place, op) in plan.operations.iter().enumerate().skip(first) {
            let Some(whole) = op.whole_image else {
                continue;
            };
            let holding = |key: u64, of: fn(&Results) -> Option<u64>| {
                (self.plan.operations.iter())
                    .zip(&self.results)
                    .find(|(old, held)| old.id == op.id && of(held) == Some(key))
                    .map(|(old, _)| old)
            };
            let whole_match = holding(whole.key, |held| held.whole)
                .map(|old| (old, whole.passes, Reused::Whole(whole.key)));
            let part_match = whole.full_resolution.and_then(|part| {
                holding(part.key, |held| held.full_resolution)
                    .map(|old| (old, part.passes, Reused::Part(part.key)))
            });
            for (old, passes, reused) in [whole_match, part_match].into_iter().flatten() {
                if old.scratch.len() != op.scratch.len() {
                    continue;
                }
                let produced = bound_scratch(&op.passes[..passes]);
                let read = bound_scratch(&op.passes[passes..])
                    .into_iter()
                    .filter(|local| produced.contains(local))
                    .map(|local| (op.scratch[local], self.scratch[old.scratch[local]].clone()))
                    .collect::<Vec<_>>();
                let sizes = |(target, source): &(usize, wgpu::Buffer)| {
                    source.size() == scratch_size(plan.scratch[*target])
                };
                if !read.iter().all(sizes) {
                    continue;
                }
                skipped[place] = passes;
                for (target, source) in read {
                    shared[target] = Some(source);
                }
                results.push((op.id, reused));
                break;
            }
        }
        let uses = |place: usize, buffer| {
            let op = &plan.operations[place];
            op.passes[skipped[place]..]
                .iter()
                .flat_map(|pass| &pass.bindings)
                .any(|binding| match binding.resource {
                    Resource::Input => op.input == buffer,
                    Resource::Output => op.output == buffer,
                    Resource::FullResolution => op.full_resolution == Some(buffer),
                    _ => false,
                })
        };
        let used = |buffer| {
            (first..plan.operations.len()).any(|place| uses(place, buffer))
                || plan.output().0 == buffer
        };
        let live = |buffer| {
            (cached.iter())
                .find(|(cached, _, _)| *cached == buffer)
                .map(|(_, image, key)| (image, *key))
        };
        let private_work =
            |index: usize| used(Buffer::Work(index)) && live(Buffer::Work(index)).is_none();
        let private_scratch = |index: usize| {
            shared[index].is_none()
                && (plan.operations.iter().enumerate().skip(first)).any(|(place, op)| {
                    bound_scratch(&op.passes[skipped[place]..])
                        .iter()
                        .any(|&local| op.scratch[local] == index)
                })
        };
        // What it binds and makes, before it makes anything: only what the passes it runs
        // bind.
        let hardware = Limits::of(gpu);
        if let Some((operation, pass, bytes)) = plan.oversized_binding_from(
            first,
            &skipped,
            hardware.binding,
            self.input.buffer().size(),
        ) {
            return Err(StillError::Refused(format!(
                "{operation}'s {pass} binds {bytes} bytes, more than the GPU's {}",
                hardware.binding
            )));
        }
        if let Some(index) = (0..plan.scratch.len())
            .find(|&index| private_scratch(index) && plan.scratch[index] > hardware.buffer)
        {
            return Err(StillError::Refused(format!(
                "a scratch buffer of {} bytes, more than the GPU's {}",
                plan.scratch[index], hardware.buffer
            )));
        }
        let bytes: u64 = plan.output().1.buffer_size()
            + (0..plan.work.len())
                .filter(|&index| private_work(index))
                .map(|index| plan.work[index].buffer_size())
                .sum::<u64>()
            + (0..plan.scratch.len())
                .filter(|&index| private_scratch(index))
                .map(|index| scratch_size(plan.scratch[index]))
                .sum::<u64>();
        let mib = |bytes: u64| bytes as f64 / 1048576.0;
        if bytes > budget {
            return Err(StillError::Refused(format!(
                "its buffers take {:.1} MiB, over the budget's {:.1}",
                mib(bytes),
                mib(budget)
            )));
        }
        tracing::debug!(
            target: "memory",
            "thumbnail still: {:.1} MiB of its own, the live pipe's {:.1} MiB",
            mib(bytes),
            mib(self.plan.bytes())
        );
        let work = (0..plan.work.len())
            .map(|index| {
                private_work(index)
                    .then(|| GpuImage::with_desc(gpu, plan.work[index]))
                    .transpose()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let scratch = (0..plan.scratch.len())
            .map(|index| {
                shared[index].clone().or_else(|| {
                    private_scratch(index).then(|| {
                        gpu.device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some("thumbnail scratch"),
                            size: scratch_size(plan.scratch[index]),
                            usage: wgpu::BufferUsages::STORAGE,
                            mapped_at_creation: false,
                        })
                    })
                })
            })
            .collect::<Vec<_>>();
        let unused_scratch = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("unused thumbnail scratch"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let image = |buffer| match live(buffer) {
            Some((image, _)) => image,
            None => match buffer {
                Buffer::Input => &self.input,
                Buffer::Work(index) => work[index].as_ref().expect("a used still buffer"),
            },
        };
        let mut images = Vec::new();
        let mut kernels = self.kernels.clone();
        let mut commands = VecDeque::new();
        let mut reading = 0;
        #[cfg(test)]
        let mut banded = 0;
        for (place, op) in plan.operations.iter().enumerate().skip(first) {
            // An operation that reads a whole-image result in place runs in one turn.
            let shares = op.scratch.iter().any(|&index| shared[index].is_some());
            let mut grouped =
                shares.then(|| gpu.device.create_command_encoder(&Default::default()));
            for (index, (pass, dispatches)) in op
                .passes
                .iter()
                .zip(&op.dispatches)
                .enumerate()
                .skip(skipped[place])
            {
                let kernel = kernels.get(gpu, op.kernel, pass.entry_point, op.id).await?;
                let tables = pass
                    .tables
                    .iter()
                    .map(|contents| {
                        gpu.device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("thumbnail table"),
                                contents,
                                usage: wgpu::BufferUsages::STORAGE,
                            })
                    })
                    .collect::<Vec<_>>();
                let full = (pass.bindings.iter())
                    .any(|binding| binding.resource == Resource::FullResolution)
                    .then(|| op.full_resolution.expect("a full-resolution binding"));
                let resources = Resources {
                    input: image(op.input).buffer(),
                    output: image(op.output).buffer(),
                    full_resolution: full.map(|buffer| image(buffer).buffer()),
                    tables: &tables,
                    scratch: op
                        .scratch
                        .iter()
                        .map(|&i| scratch[i].as_ref().unwrap_or(&unused_scratch))
                        .collect(),
                };
                let read: Vec<u64> = [Some(op.input), full]
                    .into_iter()
                    .flatten()
                    .filter_map(|buffer| live(buffer).map(|(_, key)| key))
                    .collect();
                for &key in &read {
                    if !images.contains(&key) {
                        images.push(key);
                    }
                }
                let input_bytes = image(op.input).buffer().size();
                let dispatches = bands(op.id, dispatches, input_bytes);
                #[cfg(test)]
                if op.id == DOWNSCALE.name {
                    banded += dispatches.len();
                }
                for dispatch in dispatches {
                    let uniform =
                        gpu.device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("thumbnail uniform"),
                                contents: &uniform_block(op, index, &dispatch),
                                usage: wgpu::BufferUsages::UNIFORM,
                            });
                    let group = bind_group(
                        gpu,
                        &kernels.compiled[kernel],
                        op.id,
                        pass,
                        &dispatch,
                        &uniform,
                        &resources,
                    );
                    let mut own = None;
                    let encoder = match &mut grouped {
                        Some(encoder) => encoder,
                        None => own.insert(gpu.device.create_command_encoder(&Default::default())),
                    };
                    {
                        let mut compute = encoder.begin_compute_pass(&Default::default());
                        compute.set_pipeline(kernels.compiled[kernel].pipeline());
                        compute.set_bind_group(0, &group, &[]);
                        let groups = workgroups(op, pass, &dispatch);
                        compute.dispatch_workgroups(groups.0, groups.1, 1);
                    }
                    if let Some(encoder) = own {
                        commands.push_back(encoder.finish());
                        if !read.is_empty() {
                            reading = commands.len();
                        }
                    }
                }
            }
            if let Some(encoder) = grouped {
                commands.push_back(encoder.finish());
                reading = commands.len();
            }
        }
        let (buffer, desc) = plan.output();
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("thumbnail readback"),
            size: desc.buffer_size(),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(image(buffer).buffer(), 0, &readback, 0, desc.buffer_size());
        commands.push_back(encoder.finish());
        self.kernels = kernels;
        let sources = Sources {
            images,
            results,
            released: Arc::new(AtomicBool::new(reading == 0)),
            hurry: Arc::default(),
        };
        let still = Still {
            commands,
            reading,
            released: Arc::clone(&sources.released),
            hurry: Arc::clone(&sources.hurry),
            yields_to: None,
            readback,
            mapped: None,
            size: [desc.width, desc.height],
            #[cfg(test)]
            reused_results: skipped.iter().filter(|&&passes| passes > 0).count(),
            #[cfg(test)]
            bands: banded,
        };
        Ok((still, sources))
    }

    /// Whether the next render, of the plan the last update gave, writes what `sources`
    /// reads: the image of a cache point it no longer holds or renders from before, or a
    /// whole-image result whose passes it runs or whose scratch no longer holds it. The
    /// passes after an operation's result, which every render runs, leave the result as
    /// it is, as the live pipe's own reuse of it requires.
    pub(crate) fn writes(&self, sources: &Sources) -> bool {
        let keys: Vec<u64> = self.plan.operations.iter().map(|op| op.key).collect();
        let first = self.newest_cache_point(&keys);
        self.writes_running(sources, first, keys.len())
    }

    /// [`Self::writes`] for the sample-only render a module's colour picker at operation
    /// `op` runs, reading its output too with `output` (decision 79): it writes only what
    /// it runs, from its first operation up to `op`, and `op` itself with `output`.
    pub(crate) fn sample_writes(&self, sources: &Sources, op: usize, output: bool) -> bool {
        match self.sampling(op, output) {
            Some(Sampling::Passes { first, .. }) => {
                self.writes_running(sources, first, if output { op + 1 } else { op })
            }
            Some(Sampling::EditCache) | None => false,
        }
    }

    /// Whether a render running the operations from `first` up to `end`, exclusive,
    /// writes what `sources` reads.
    fn writes_running(&self, sources: &Sources, first: usize, end: usize) -> bool {
        if sources.released() {
            return false;
        }
        let kept = |key: u64| {
            (self.plan.cached.iter())
                .zip(&self.held)
                .any(|(&op, held)| *held == Some(key) && !(first..end).contains(&op))
        };
        if !sources.images.iter().all(|&key| kept(key)) {
            return true;
        }
        let runs: Vec<bool> = (self.runs_from(first).into_iter())
            .zip(&self.passes)
            .map(|(runs, pass)| runs && pass.op < end)
            .collect();
        sources.results.iter().any(|&(id, reused)| {
            let Some(place) = self.plan.operations.iter().position(|op| op.id == id) else {
                return true;
            };
            let Some(whole) = self.plan.operations[place].whole_image else {
                return true;
            };
            let held = self.results[place];
            let (key, holds, passes) = match reused {
                Reused::Whole(key) => (key, held.whole, whole.passes),
                Reused::Part(key) => (
                    key,
                    held.full_resolution,
                    whole
                        .full_resolution
                        .map_or(whole.passes, |part| part.passes),
                ),
            };
            holds != Some(key)
                || (self.passes.iter().zip(&runs))
                    .any(|(pass, &runs)| runs && pass.op == place && pass.index < passes)
        })
    }

    /// The image in `buffer`: the input, or a working buffer.
    pub(super) fn image(&self, buffer: Buffer) -> &GpuImage {
        match buffer {
            Buffer::Input => &self.input,
            Buffer::Work(index) => &self.work[index],
        }
    }
}

/// How a still runs a pass's `dispatches`: as planned, but the downscale from an image of
/// `input_bytes`, planned whole, in bands of its output rows that each read about
/// [`BAND_BYTES`] of it, so a live frame waits for one band at most. The downscale writes
/// each output pixel from its own footprint, wherever a band starts.
fn bands(id: &str, dispatches: &[Dispatch], input_bytes: u64) -> Vec<Dispatch> {
    let whole = matches!(dispatches, [only] if only.input.is_none() && only.output.is_none());
    let band_bytes = band_bytes();
    if id != DOWNSCALE.name || !whole || input_bytes <= band_bytes {
        return dispatches.to_vec();
    }
    let [first, rows] = dispatches[0].rows;
    let band = (u64::from(rows) * band_bytes / input_bytes).max(1) as u32;
    (0..rows)
        .step_by(band as usize)
        .map(|start| Dispatch {
            rows: [first + start, band.min(rows - start)],
            input: None,
            output: None,
        })
        .collect()
}

impl Still {
    /// Takes no turn within [`EDITING_QUIET`] of the live photo's last change, `last`.
    pub(crate) fn yield_to(&mut self, last: LastChange) {
        self.yields_to = Some(last);
    }

    /// When its next turn may come, where the live photo changed within
    /// [`EDITING_QUIET`] of now.
    fn editing(&self) -> Option<Instant> {
        let last = self
            .yields_to
            .as_ref()
            .and_then(|last| *last.lock().unwrap_or_else(PoisonError::into_inner));
        last.map(|at| at + EDITING_QUIET)
            .filter(|until| *until > Instant::now())
    }

    /// The commands it has still to submit, for a test.
    #[cfg(test)]
    pub(crate) fn left(&self) -> usize {
        self.commands.len()
    }
}

/// A still dropped, done or not, reads nothing of the live pipe any more.
impl Drop for Still {
    fn drop(&mut self) {
        self.released.store(true, Ordering::Relaxed);
    }
}

impl GpuTask for Still {
    type Output = ([u32; 2], Vec<[f32; 4]>);

    fn step(&mut self, gpu: &GpuContext) -> Result<Step<Self::Output>, String> {
        // A departed photo's: every command reading its buffers in one submission.
        if self.reading > 0 && self.hurry.load(Ordering::Relaxed) {
            let reading = self.commands.drain(..self.reading).collect::<Vec<_>>();
            gpu.submit(reading);
            self.reading = 0;
            self.released.store(true, Ordering::Relaxed);
            return Ok(Step::Again);
        }
        // A person editing goes first: each turn waits until the live photo has stood
        // still for a moment.
        if !self.commands.is_empty()
            && let Some(until) = self.editing()
        {
            return Ok(Step::Wait(until));
        }
        if let Some(command) = self.commands.pop_front() {
            gpu.submit([command]);
            if self.reading > 0 {
                self.reading -= 1;
                if self.reading == 0 {
                    self.released.store(true, Ordering::Relaxed);
                }
            }
            return Ok(Step::Again);
        }
        let mapped = self.mapped.get_or_insert_with(|| {
            let (send, recv) = mpsc::channel();
            self.readback
                .map_async(wgpu::MapMode::Read, .., move |result| {
                    let _ = send.send(result);
                });
            recv
        });
        match mapped.try_recv() {
            Ok(Ok(())) => {
                let bytes = self
                    .readback
                    .get_mapped_range(..)
                    .map_err(|e| e.to_string())?;
                let pixels = bytemuck::cast_slice::<u8, [f32; 4]>(&bytes).to_vec();
                drop(bytes);
                self.readback.unmap();
                Ok(Step::Done((self.size, pixels)))
            }
            Ok(Err(error)) => Err(error.to_string()),
            Err(mpsc::TryRecvError::Empty) => Ok(Step::Again),
            Err(error) => Err(error.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_downscale_from_a_large_image_runs_in_bands_covering_every_row_once() {
        let whole = [Dispatch {
            rows: [0, 683],
            input: None,
            output: None,
        }];
        let bands = bands(DOWNSCALE.name, &whole, 963 << 20);
        assert_eq!(bands.len(), 16);
        let mut next = 0;
        for band in &bands {
            assert_eq!(band.rows[0], next);
            next += band.rows[1];
        }
        assert_eq!(next, 683);
        // A small image, or another operation, runs as planned.
        assert_eq!(super::bands(DOWNSCALE.name, &whole, 32 << 20), whole);
        assert_eq!(super::bands("hazeremoval", &whole, 963 << 20), whole);
    }
}
