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

//! Exports (`docs/engine.md`, Tiling): the whole image at full resolution, in tiles
//! when it would not fit (decision 13). The operations up to the raw stage's last one
//! that needs the whole image, a whole-image pass's (highlight reconstruction so far),
//! run once over it; the rest run tile by tile, each tile from a copy of the part of
//! that image it needs, and each finished tile is read back into the output image. An
//! operation with whole-image passes after demosaic, the tone equalizer's mask, takes a
//! first round of tiles that builds their result in scratch the export holds, before
//! the last round renders the tiles (decision 15). A tiled export equals an untiled one
//! bit for bit. On a GPU that binds less than the image at once, the operations before
//! the tiles run in row strips, and each tile copies from a window of their output
//! (decision 14). Tiles are regions of the output image, after any operation that moves
//! pixels, and a tile copies the region its operations read back through it (decision
//! 16). An export at a size resamples as darktable does, after demosaic or at
//! finalscale's place, its tiles regions of the output at its scale walked back through
//! the resample's plans, so a tiled export at a size equals an untiled one too (decision
//! 56); without high quality it resamples after haze removal while that is planned. A
//! first round there takes tiles of the image its whole-image passes cover: at full
//! resolution before the resample, and the output's own at the export's scale after it,
//! so one export may take both (decision 15).
//!
//! A session's export job shares the GPU with the viewport (`export_handing`, decision 56):
//! it plans, compiles and allocates where it runs, on the job's worker, and hands each
//! render the export makes to the engine thread as a `Submit`, one submission, one step,
//! each finished tile read back through a mapping the worker waits for, which the engine
//! thread's polls give. It always tiles, its tiles at most [`Tiling::AtMost`]'s edge (spike
//! X1). Beside a live frame the operations before the tiles (the raw stage on a default
//! edit) take a step per pass, a pass in row strips of the size the job gives a step per
//! strip, each buffer they make touched in a step of its own first; with nothing live
//! sharing the GPU they are one step. The renders and their order are [`export_at`]'s, and
//! strips change only how a pass dispatches (decision 14), so the two give the same pixels.

use std::sync::mpsc;
use std::time::Instant;

use apertrust_plugin::{ImageContext, ImageDesc, PassKind, Resource};
use thiserror::Error;

use crate::compute::{ComputeError, GpuImage, Timings};
use crate::gpu::GpuContext;
use crate::jobs::{GpuTask, Step, TaskError};
pub use crate::pipeline::BUDGET;
use crate::pipeline::{
    self, Instance, Limits, Parts, Pipe, Pipeline, Plan, PlanError, Resample, SMALLEST_TILE,
    reads_the_whole_image, runs, split,
};

/// The narrowest tile a row or column of tiles may end with. A tile at the image's edge
/// gets the operations' border on one side only, demosaic's 12 so far, and demosaic needs
/// 16 photosites, 20 for RCD; a last tile narrower than this shares its width with the
/// one before it instead.
const NARROWEST_TILE: u32 = 32;

/// How an export is cut up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tiling {
    /// Tiles only when the whole image would take more than `budget` bytes of GPU
    /// memory or a buffer larger than the adapter can bind, then the largest square
    /// tiles that fit.
    Auto { budget: u64 },
    /// The whole image at once.
    Whole,
    /// Tiles of at most this many pixels across and down.
    Tiles([u32; 2]),
    /// Always tiles, even where the whole image fits, square ones of at most `edge`
    /// pixels, or the smaller ones `Auto`'s search finds within `budget`: a session's
    /// export job, each of whose tiles is one submission beside the viewport (spike X1).
    /// `edge` must be a power of two of at least the search's smallest tile, 64, one of
    /// the sizes the search tries, so that its tiles are those it would find below
    /// `edge`.
    AtMost { budget: u64, edge: u32 },
}

/// A finished export: RGBA in the output profile's encoding, rows top to bottom.
pub struct Exported {
    pub desc: ImageDesc,
    pub pixels: Vec<[f32; 4]>,
    /// How many tiles it took; 1 when the image ran whole.
    pub tiles: usize,
    /// How many rounds of tiles: one, and one more per operation whose whole-image
    /// passes after demosaic took a first round (decision 15); 1 when it ran whole.
    pub rounds: usize,
}

#[derive(Debug, Error)]
pub enum ExportError {
    #[error(transparent)]
    Plan(#[from] PlanError),
    #[error(transparent)]
    Compute(#[from] ComputeError),
    #[error(
        "no tiles down to {SMALLEST_TILE} pixels fit the GPU, which binds at most {:.1} MiB at once, within the {:.0} MiB budget",
        *binding as f64 / (1 << 20) as f64,
        *budget as f64 / (1 << 20) as f64
    )]
    NoTiles { binding: u64, budget: u64 },
    /// Whole-image scratch an export holds across its rounds of tiles, which no tile
    /// size makes smaller and which is bound whole, larger than the GPU binds or makes
    /// at once (decision 15), until decision 15's image-shaped scratch in row strips.
    #[error(
        "{operation}'s {pass} binds {:.1} MiB of whole-image scratch at once, which no tile size makes smaller, more than the GPU allows ({:.1} MiB)",
        *bytes as f64 / (1 << 20) as f64,
        *limit as f64 / (1 << 20) as f64
    )]
    HeldScratch {
        operation: &'static str,
        pass: &'static str,
        bytes: u64,
        limit: u64,
    },
    /// A render handed to the engine thread did not come back: the job was cancelled,
    /// the engine stopped, or its step failed.
    #[error(transparent)]
    Handed(#[from] TaskError),
}

/// Renders `instances` over the whole image of `input` at full resolution, tiled as
/// `tiling` says.
pub async fn export(
    gpu: &GpuContext,
    instances: &[Instance<'_>],
    image: &ImageContext,
    input: GpuImage,
    tiling: Tiling,
) -> Result<Exported, ExportError> {
    export_at(gpu, instances, image, input, tiling, None).await
}

/// Renders `instances` over the whole image of `input`, at full resolution or at the
/// size `resample` gives, tiled as `tiling` says (decision 56).
pub async fn export_at(
    gpu: &GpuContext,
    instances: &[Instance<'_>],
    image: &ImageContext,
    input: GpuImage,
    tiling: Tiling,
    resample: Option<Resample>,
) -> Result<Exported, ExportError> {
    render_export(
        gpu,
        instances,
        image,
        input,
        tiling,
        resample,
        Turns::Waiting,
    )
    .await
}

/// What a thread that may not submit does with each of an export's renders: hands it to
/// the thread that may, the engine thread for a session's job, and waits for its one
/// submission to be made; then, where it reads the render back, waits for the readback's
/// mapping, which that thread's polls give. Both waits return at a cancel.
pub(crate) trait Hand {
    /// Hands `submit` and waits for its step.
    fn hand(&mut self, submit: Submit) -> Result<Rendered, TaskError>;
    /// Waits for a readback's mapping.
    fn wait(&mut self, mapping: &Mapping) -> Result<MapResult, TaskError>;
    /// The most bytes a row strip of a pass before the tiles binds of an image, asked as
    /// each export's operations before the tiles are planned: the passes that take strips
    /// run in strips of it, a step each, and every pass is a step. `None` where no live
    /// frame shares the GPU, as in the CLI's session: those operations are then one step,
    /// which takes less time alone than steps of a pass (spike X1).
    fn strip(&self) -> Option<u64>;
}

/// A readback's mapping, as wgpu answers it.
pub(crate) type MapResult = Result<(), wgpu::BufferAsyncError>;

/// Where a readback's mapping is heard.
pub(crate) type Mapping = mpsc::Receiver<MapResult>;

/// [`export_at`] for a session's export job, run on the job's worker (decision 57):
/// planned, compiled and allocated there, and each render handed through `hand` as a
/// [`Submit`], which the engine thread steps after the live frame, so it only ever makes
/// submissions and asks for readbacks' mappings, never waiting for either. Where
/// [`Hand::strip`] gives a size, the operations before the tiles take a step per pass, a
/// pass that takes row strips planned in strips whose windows bind at most its bytes, a
/// step per strip; where it gives none, one step. The renders are untimed, so the perf line
/// gives the export's time alone.
pub(crate) async fn export_handing(
    gpu: &GpuContext,
    instances: &[Instance<'_>],
    image: &ImageContext,
    input: GpuImage,
    tiling: Tiling,
    resample: Option<Resample>,
    hand: &mut dyn Hand,
) -> Result<Exported, ExportError> {
    let turns = Turns::Handed(hand);
    render_export(gpu, instances, image, input, tiling, resample, turns).await
}

/// How an export's renders reach the GPU.
enum Turns<'a> {
    /// Rendered and read back here, waiting for each and timing the renders, as a caller
    /// with the GPU to itself does.
    Waiting,
    /// Handed to another thread ([`export_handing`]), the operations before the tiles in
    /// parts, a step each, where it gives a strip's size.
    Handed(&'a mut dyn Hand),
}

/// [`export_at`], its renders reaching the GPU as `turns` says.
async fn render_export(
    gpu: &GpuContext,
    instances: &[Instance<'_>],
    image: &ImageContext,
    input: GpuImage,
    tiling: Tiling,
    resample: Option<Resample>,
    mut turns: Turns<'_>,
) -> Result<Exported, ExportError> {
    let start = Instant::now();
    let pipe = resample.map_or(Pipe::Export, Pipe::Resampled);
    let tile = |region| Pipe::Tile { region, resample };
    let limits = match tiling {
        Tiling::Auto { budget } | Tiling::AtMost { budget, .. } => Limits {
            budget,
            ..Limits::of(gpu)
        },
        Tiling::Whole | Tiling::Tiles(_) => Limits::of(gpu),
    };
    let whole = pipeline::plan(instances, image, input.desc(), pipe, &limits)?;
    // The parts before the tiles and the first rounds' tiles leave colorout out, so every
    // part is planned with the whole export's output profile (decision 26).
    let image = &ImageContext {
        output_profile: whole.image.output_profile,
        ..image.clone()
    };
    let (split, resampled) = split_at_size(&whole);
    let size = match tiling {
        Tiling::Whole => None,
        Tiling::Tiles(size) => Some(size),
        Tiling::Auto { .. } => auto_tile(&whole, split, instances, image, &limits, resample, None)?,
        Tiling::AtMost { edge, .. } => {
            debug_assert!(
                edge.is_power_of_two() && edge >= SMALLEST_TILE,
                "Tiling::AtMost's edge, {edge}, is not a power of two of at least {SMALLEST_TILE}"
            );
            auto_tile(
                &whole,
                split,
                instances,
                image,
                &limits,
                resample,
                Some(edge),
            )?
        }
    };
    let Some([tile_width, tile_height]) = size.filter(|_| split < whole.operations.len()) else {
        let pipeline = Pipeline::new(gpu, whole, input).await?;
        let desc = pipeline.output().desc();
        let (_, pixels) = render_and_read(pipeline, gpu, &mut turns, &mut Vec::new())?;
        return Ok(Exported {
            desc,
            pixels,
            tiles: 1,
            rounds: 1,
        });
    };
    let runs = runs(&whole.operations, split);

    // The operations before the tiles (the raw stage on a default edit), once over the
    // whole image, at the export's size once they take in its resample; only their output
    // is kept. Handed beside a live frame, in row strips of the job's size where a pass
    // takes them, which change only how it dispatches (decision 14), each strip, each pass
    // and each touch of a new buffer a step; with none, in one step.
    let (before, after) = divide(instances, &whole, split);
    let before_pipe = match resample {
        Some(resample) if resampled => Pipe::Resampled(in_its_stage(&whole, resample)),
        _ => Pipe::Export,
    };
    let strip = match &turns {
        Turns::Waiting => None,
        Turns::Handed(hand) => hand.strip(),
    };
    let before_limits = Limits {
        binding: strip.map_or(limits.binding, |strip| limits.binding.min(strip)),
        ..limits
    };
    let prefix = Pipeline::new(
        gpu,
        pipeline::plan(&before, image, input.desc(), before_pipe, &before_limits)?,
        input,
    )
    .await?;
    let mut renders = Vec::new();
    let prefix = match strip {
        Some(_) => render_in_parts(prefix, gpu, &mut turns, &mut renders)?,
        None => render(prefix, gpu, &mut turns, &mut renders)?,
    };
    let source = prefix.output().clone();
    drop(prefix);

    // Tiles of the output image, after any geometry (decision 16), at the export's size
    // (decision 56).
    let extent = output_extent(&whole);
    let regions = regions(extent, [tile_width, tile_height]);
    tracing::debug!(
        target: "pipe",
        "export: {} tiles, up to {tile_width}×{tile_height}, after {}{}",
        regions.len(),
        whole.operations[split - 1].id,
        match runs.len() {
            0 => String::new(),
            n => format!(", {} rounds for {}", n + 1, runs.join(", ")),
        }
    );
    // A first round of tiles per operation with whole-image passes after demosaic
    // (decision 15): each tile runs up to it and through its whole-image passes that read
    // its input, writing its part of their result into scratch the export holds, and the
    // last tile then runs the passes that read only scratch. Its tiles overlap by a pixel
    // to the right and below, so a grid sample on a tile's edge reads its next node there.
    // Each run's tiles are of the image it covers, by its place against the resample in
    // the plan (decision 15): after the resample, or at full size, the output image's,
    // walked back through the resample as the last round's are; before it, at
    // finalscale's place or without high quality haze removal's, the image's at full
    // resolution, none larger than the region the largest output tile walks back to less
    // its taps' reach, so a tile size the last round fits fits them (decision 56). One
    // export may take both, haze removal's at full resolution and the tone equalizer's at
    // the export's scale.
    // A run before a geometry, haze removal's before orientation's turn, covers its own
    // stage, the photo before the turn: its tiles are of that stage, which its plans
    // without the geometry make (decision 16).
    let stage_of = |run: &str| {
        whole
            .operations
            .iter()
            .find(|op| op.id == run)
            .map_or(whole.extent(), |op| op.extent)
    };
    let full_resolution = |run: &str| {
        resample.map(|resample| {
            let full = stage_of(run);
            let largest = |axis: usize| regions.iter().map(|r| r[axis + 2]).max().unwrap_or(1);
            // The output's tiles in the stage's own frame: a quarter turn's sides swapped.
            let output = whole.extent();
            let turned = full != output && full == [output[1], output[0]];
            let edge = [0, 1].map(|axis| {
                let axis = if turned { 1 - axis } else { axis };
                let side = f64::from(largest(axis)) / f64::from(resample.scale);
                (side.ceil() as u32).max(1)
            });
            (full, self::regions(full, edge))
        })
    };
    let mut held: Vec<(&'static str, Vec<wgpu::Buffer>)> = Vec::new();
    for &run in &runs {
        let through = &instances_through(&after, run);
        let own_stage;
        let (round_extent, round_regions, round_resample) = match full_resolution(run) {
            Some((full, full_regions)) if !past_the_resample(&whole, run) => {
                own_stage = full_regions;
                (full, &own_stage, None)
            }
            None if stage_of(run) != extent => {
                let stage = stage_of(run);
                let [width, height] = [tile_width, tile_height];
                let edge = if stage == [extent[1], extent[0]] {
                    [height, width]
                } else {
                    [width, height]
                };
                own_stage = self::regions(stage, edge);
                (stage, &own_stage, None)
            }
            _ => (extent, &regions, resample),
        };
        let last = round_regions.len() - 1;
        let plans = round_regions
            .iter()
            .enumerate()
            .map(|(k, &region)| {
                let pipe = Pipe::Tile {
                    region: grown(region, round_extent),
                    resample: round_resample,
                };
                let mut plan = pipeline::plan(through, image, source.desc(), pipe, &limits)?;
                for (id, _) in &held {
                    plan.retain_passes(id, |pass| pass.kind != PassKind::Reduction);
                }
                plan.retain_passes(run, |pass| {
                    reads_the_whole_image(pass) || (k == last && pass.kind == PassKind::Reduction)
                });
                Ok(plan)
            })
            .collect::<Result<Vec<_>, PlanError>>()?;
        let mut tiles =
            Pipeline::holding(gpu, largest(&plans).clone(), source.clone(), &held).await?;
        held.push((run, tiles.scratch_of(run)));
        for plan in plans {
            tiles = tiles.update_holding(gpu, plan, &held).await?;
            tiles = render(tiles, gpu, &mut turns, &mut renders)?;
        }
    }

    // The last round, every tile through every operation, without the whole-image passes
    // the first rounds ran, binding their results.
    let plans = regions
        .iter()
        .map(|&region| {
            let mut plan = pipeline::plan(&after, image, source.desc(), tile(region), &limits)?;
            for (id, _) in &held {
                plan.retain_passes(id, |pass| pass.kind != PassKind::Reduction);
            }
            Ok(plan)
        })
        .collect::<Result<Vec<_>, PlanError>>()?;
    // Buffers for the largest tile, so every other one fits them.
    let mut tiles = Pipeline::holding(gpu, largest(&plans).clone(), source, &held).await?;
    let (width, height) = (extent[0] as usize, extent[1] as usize);
    let mut pixels = vec![[0.0; 4]; width * height];
    for (region, plan) in regions.iter().zip(plans) {
        tiles = tiles.update_holding(gpu, plan, &held).await?;
        let output = tiles.output();
        let [x, y, w, h] = region.map(|v| v as usize);
        assert_eq!(
            (output.width() as usize, output.height() as usize),
            (w, h),
            "a tile renders its region"
        );
        let tile;
        (tiles, tile) = render_and_read(tiles, gpu, &mut turns, &mut renders)?;
        for row in 0..h {
            let to = (y + row) * width + x;
            pixels[to..to + w].copy_from_slice(&tile[row * w..(row + 1) * w]);
        }
    }
    tracing::debug!(
        target: "perf",
        "{}",
        exported(regions.len(), start.elapsed().as_secs_f64() * 1e3, &renders)
    );
    Ok(Exported {
        desc: ImageDesc::rgba(extent[0], extent[1]),
        pixels,
        tiles: regions.len(),
        rounds: 1 + runs.len(),
    })
}

/// Renders `pipeline` once: here, waiting, its timings kept in `renders`, or handed.
fn render(
    mut pipeline: Pipeline,
    gpu: &GpuContext,
    turns: &mut Turns,
    renders: &mut Vec<Timings>,
) -> Result<Pipeline, ExportError> {
    match turns {
        Turns::Waiting => renders.push(pipeline.render(gpu)?),
        Turns::Handed(hand) => {
            pipeline = hand.hand(Submit::new(pipeline, None))?.pipeline;
        }
    }
    Ok(pipeline)
}

/// [`render`], but handed in parts, a step each ([`Pipeline::parts`]).
fn render_in_parts(
    mut pipeline: Pipeline,
    gpu: &GpuContext,
    turns: &mut Turns,
    renders: &mut Vec<Timings>,
) -> Result<Pipeline, ExportError> {
    let Turns::Handed(hand) = turns else {
        return render(pipeline, gpu, turns, renders);
    };
    let parts = pipeline.parts();
    let submit = Submit {
        pipeline: Some(pipeline),
        readback: None,
        parts: Some((parts, 0)),
    };
    Ok(hand.hand(submit)?.pipeline)
}

/// Renders `pipeline` once and reads its output back: here, waiting, its timings kept in
/// `renders`, or handed with a buffer the CPU maps, into which its submission copies the
/// output, its mapping waited for here, so a readback takes the engine thread no step of
/// its own (spike X1).
fn render_and_read(
    mut pipeline: Pipeline,
    gpu: &GpuContext,
    turns: &mut Turns,
    renders: &mut Vec<Timings>,
) -> Result<(Pipeline, Vec<[f32; 4]>), ExportError> {
    let output = pipeline.output();
    let Turns::Handed(hand) = turns else {
        renders.push(pipeline.render(gpu)?);
        let pixels = output.download(gpu)?;
        return Ok((pipeline, pixels));
    };
    let staging = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("export readback"),
        size: output.desc().buffer_size(),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let Rendered { pipeline, readback } = hand.hand(Submit::new(pipeline, Some(staging)))?;
    let Readback {
        buffer,
        mapping,
        submitted,
    } = readback.expect("a readback was asked for");
    hand.wait(&mapping)?
        .map_err(|error| TaskError::Failed(format!("reading the export back: {error}")))?;
    let ms = submitted.elapsed().as_secs_f64() * 1e3;
    tracing::trace!(target: "perf", "readback mapped {ms:.2} ms after its submission");
    // Copied here, on the worker, and unmapped, so the engine thread copies nothing.
    let start = Instant::now();
    let pixels = bytemuck::pod_collect_to_vec(
        &buffer
            .get_mapped_range(..)
            .expect("its mapping was answered"),
    );
    buffer.unmap();
    tracing::trace!(
        target: "perf",
        "copied the readback, {:.0} MiB, in {:.1} ms",
        buffer.size() as f64 / f64::from(1 << 20),
        start.elapsed().as_secs_f64() * 1e3
    );
    Ok((pipeline, pixels))
}

/// One render of an export's pipeline, built on the job's worker and handed to the engine
/// thread as its [`GpuTask`] (decision 57), in one step: it submits the passes, with the
/// output's copy into the readback buffer where there is one, asks for that buffer's
/// mapping and hands the pipeline back. The engine thread polls until the step's work is
/// done before it steps again, and the poll that finds it done answers the mapping too, for
/// which the worker waits ([`Hand::wait`]); so a readback takes no turn of its own (spike
/// X1). A later submission, and the uniforms the worker writes for it, come after this one
/// on the queue. A render in parts, the operations before the tiles, takes a step per part
/// instead, with no readback.
pub(crate) struct Submit {
    pipeline: Option<Pipeline>,
    readback: Option<wgpu::Buffer>,
    /// A render in parts and the next part to submit; `None` for one submission.
    parts: Option<(Parts, usize)>,
}

/// A [`Submit`] done: its pipeline, and its readback where it has one, on its way.
pub(crate) struct Rendered {
    pipeline: Pipeline,
    readback: Option<Readback>,
}

/// A readback asked for: the buffer the submission copies the output into, for the
/// worker to copy and unmap once mapped, so the engine thread never copies an image, and
/// where its mapping is heard.
struct Readback {
    buffer: wgpu::Buffer,
    mapping: Mapping,
    /// When it was submitted, for `-d perf`'s trace (spike X1).
    submitted: Instant,
}

impl Submit {
    fn new(pipeline: Pipeline, readback: Option<wgpu::Buffer>) -> Self {
        let pipeline = Some(pipeline);
        let parts = None;
        Self {
            pipeline,
            readback,
            parts,
        }
    }

    /// Whether it reads its output back, for a test that refuses those renders.
    #[cfg(test)]
    pub(crate) fn reads_back(&self) -> bool {
        self.readback.is_some()
    }
}

impl GpuTask for Submit {
    type Output = Rendered;

    fn step(&mut self, gpu: &GpuContext) -> Result<Step<Rendered>, String> {
        if let Some((parts, next)) = &mut self.parts {
            if *next < parts.len() {
                let pipeline = self.pipeline.as_mut().expect("stepped until done");
                pipeline.submit_part(gpu, parts, *next);
                *next += 1;
            }
            if *next < parts.len() {
                return Ok(Step::Again);
            }
            let pipeline = self.pipeline.take().expect("stepped until done");
            let readback = None;
            return Ok(Step::Done(Rendered { pipeline, readback }));
        }
        let mut pipeline = self.pipeline.take().expect("a render is one step");
        let output = pipeline.output();
        pipeline.submit_then(gpu, |encoder| {
            if let Some(readback) = &self.readback {
                let bytes = readback.size();
                encoder.copy_buffer_to_buffer(output.buffer(), 0, readback, 0, bytes);
            }
        });
        let readback = self.readback.take().map(|buffer| {
            let (sender, mapping) = mpsc::channel();
            buffer.map_async(wgpu::MapMode::Read, .., move |result| {
                // A worker that has stopped waiting hears nothing.
                let _ = sender.send(result);
            });
            let submitted = Instant::now();
            Readback {
                buffer,
                mapping,
                submitted,
            }
        });
        Ok(Step::Done(Rendered { pipeline, readback }))
    }
}

/// The perf domain's line for a tiled export of `tiles` that took `ms`, from planning to
/// the last tile read back, compiling included, with its `renders` added up, the
/// operations' before the tiles and each tile's, from submission to the GPU finishing:
/// `exported 20 tiles in 1195 ms, rendering: 632.1 ms, GPU: 610.3 ms`. Without them, as a
/// job's untimed renders, the time alone.
fn exported(tiles: usize, ms: f64, renders: &[Timings]) -> String {
    let tiles = format!("{tiles} {}", if tiles == 1 { "tile" } else { "tiles" });
    if renders.is_empty() {
        return format!("exported {tiles} in {ms:.0} ms");
    }
    let rendering: f64 = renders.iter().map(|render| render.wall_ms).sum();
    let gpu: f64 = renders
        .iter()
        .flat_map(|render| &render.passes)
        .map(|(_, ms)| ms)
        .sum();
    let gpu = if gpu.is_nan() {
        String::new()
    } else {
        format!(", GPU: {gpu:.1} ms")
    };
    format!("exported {tiles} in {ms:.0} ms, rendering: {rendering:.1} ms{gpu}")
}

/// The output image's size: the last operation's whole output, after any geometry
/// (decision 16) and at the export's size (decision 56).
fn output_extent(whole: &Plan) -> [u32; 2] {
    let (_, output) = whole.output();
    [output.width, output.height]
}

/// `resample` with its output in the stage it runs in, as the whole plan resamples: the
/// output image's size taken back through the geometries after it, a quarter turn's sides
/// swapped back (decision 16), for a plan of the operations before them alone.
fn in_its_stage(whole: &Plan, resample: Resample) -> Resample {
    let made = whole
        .resample
        .as_ref()
        .map(|range| match range.end.checked_sub(1) {
            Some(index) => whole.operations[index].output_desc,
            None => whole.input,
        });
    match made {
        Some(desc) => Resample {
            output: [desc.width, desc.height],
            ..resample
        },
        None => resample,
    }
}

/// Where an export's tiles begin ([`split`]), and whether the operations before them
/// take in its resample, wherever the plan has it (decisions 15 and 56): a split just
/// before the resample takes it in too, so a tile's RGBA input is past the resample
/// whenever the operations before it were, and its first rounds lie on either side.
fn split_at_size(whole: &Plan) -> (usize, bool) {
    let split = split(&whole.operations);
    match &whole.resample {
        Some(resample) if resample.contains(&split) => (resample.end, true),
        Some(resample) => (split, resample.end <= split),
        None => (split, false),
    }
}

/// Whether the operation `run` comes after the export's resample in its plan, its
/// whole-image passes covering its stage at the export's scale, the output, rather than
/// at full resolution (decision 15): its first round then takes the output's own tiles.
fn past_the_resample(whole: &Plan, run: &str) -> bool {
    let index = whole.operations.iter().position(|op| op.id == run);
    whole
        .resample
        .as_ref()
        .zip(index)
        .is_some_and(|(resample, index)| index >= resample.end)
}

/// The plan of the largest tile, whose buffers every other one fits.
fn largest(plans: &[Plan]) -> &Plan {
    plans
        .iter()
        .max_by_key(|plan| plan.bytes())
        .expect("at least one tile")
}

/// The instances in the tiles up to and including the operation `id`, for its first round.
fn instances_through<'a>(instances: &[Instance<'a>], id: &str) -> Vec<Instance<'a>> {
    let slot = |instance: &Instance| instance.operation.manifest().slot;
    let last = instances
        .iter()
        .find(|instance| instance.operation.manifest().id == id)
        .map(slot)
        .expect("the run's operation is in the tiles");
    instances
        .iter()
        .filter(|instance| slot(instance).pipeline_order(&last).is_le())
        .cloned()
        .collect()
}

/// A first round's tile: `region` a pixel larger to the right and below, but at the
/// edges of the output image, `extent`, so the grid samples it places can read their
/// nodes (decision 15).
fn grown(region: [u32; 4], extent: [u32; 2]) -> [u32; 4] {
    let [x, y, width, height] = region;
    [
        x,
        y,
        (width + 1).min(extent[0] - x),
        (height + 1).min(extent[1] - y),
    ]
}

/// The instances before the tiles and those in them.
fn divide<'a>(
    instances: &[Instance<'a>],
    whole: &Plan,
    split: usize,
) -> (Vec<Instance<'a>>, Vec<Instance<'a>>) {
    let before: Vec<&str> = whole.operations[..split].iter().map(|op| op.id).collect();
    instances
        .iter()
        .cloned()
        .partition(|instance| before.contains(&instance.operation.manifest().id))
}

/// The output image, `extent` at full resolution, the developed image's but after a
/// geometry (decision 16), cut into tiles of at most `size`, row by row.
fn regions(extent: [u32; 2], size: [u32; 2]) -> Vec<[u32; 4]> {
    let [tile_width, tile_height] = size.map(|v| v.max(1));
    let columns = spans(extent[0], tile_width);
    spans(extent[1], tile_height)
        .into_iter()
        .flat_map(|(y, height)| columns.iter().map(move |&(x, width)| [x, y, width, height]))
        .collect()
}

/// `length` cut into spans of at most `size`, start and length: all of `size` but the
/// last, which shares the length of the last two evenly when it would be narrower than
/// [`NARROWEST_TILE`].
fn spans(length: u32, size: u32) -> Vec<(u32, u32)> {
    let mut spans: Vec<(u32, u32)> = (0..length)
        .step_by(size as usize)
        .map(|start| (start, size.min(length - start)))
        .collect();
    if let [.., (start, before), (_, last)] = spans[..]
        && last < NARROWEST_TILE
    {
        let shared = (before + last) / 2;
        let n = spans.len();
        spans[n - 2] = (start, shared);
        spans[n - 1] = (start + shared, before + last - shared);
    }
    spans
}

/// The bytes of whole-image scratch `plan` gives the operations in `runs`, which an
/// export holds across its rounds and no tile size changes (decision 15).
fn held_bytes(plan: &Plan, runs: &[&str]) -> u64 {
    plan.operations
        .iter()
        .filter(|op| runs.contains(&op.id))
        .flat_map(|op| &op.scratch)
        .map(|&index| plan.scratch[index])
        .sum()
}

/// The first pass of an operation in `runs` that binds a scratch buffer larger than the
/// GPU binds or makes at once: an export holds that scratch whole across its rounds and
/// binds it whole, so no tile size helps (decision 15), and the export is refused,
/// naming the pass, rather than searching every tile size for nothing.
fn held_scratch_binds(whole: &Plan, runs: &[&str], limits: &Limits) -> Result<(), ExportError> {
    let limit = limits.binding.min(limits.buffer);
    for op in whole.operations.iter().filter(|op| runs.contains(&op.id)) {
        for pass in &op.passes {
            for binding in &pass.bindings {
                if let Resource::Scratch(index) = binding.resource {
                    let bytes = whole.scratch[op.scratch[index]];
                    if bytes > limit {
                        return Err(ExportError::HeldScratch {
                            operation: op.id,
                            pass: pass.entry_point,
                            bytes,
                            limit,
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

/// Logs, under `-d memory`, the whole-image scratch an export holds for `runs`, `held`
/// bytes, where it and `own`, the largest plan's buffers beside it, `what`, come to more
/// than the budget (decision 15).
fn log_held(held: u64, own: u64, runs: &[&str], what: &str, limits: &Limits) {
    let mebibytes = |bytes: u64| bytes as f64 / (1 << 20) as f64;
    if held > 0 && held + own > limits.budget {
        tracing::debug!(
            target: "memory",
            "export: {:.0} MiB of whole-image scratch for {} and {:.0} MiB for {what}, {:.0} MiB, over the {:.0} MiB budget",
            mebibytes(held),
            runs.join(", "),
            mebibytes(own),
            mebibytes(held + own),
            mebibytes(limits.budget)
        );
    }
}

/// No tiles when the whole image fits; otherwise the largest square tiles, halving from
/// 8192 to [`SMALLEST_TILE`], whose plans all fit beside the image the tiles start from.
/// Both are sized without the whole-image scratch the export holds across its rounds,
/// which no tile size changes, so an export whose other buffers fit runs whole, in one
/// round; that scratch is logged where it and the largest plan's other buffers exceed
/// the budget, and every buffer must still bind, a held one refused naming its pass
/// (decision 15). With `at_most`, always tiles, the search halving from that edge, so
/// the tiles are the smaller of it and those the search from 8192 would find.
fn auto_tile(
    whole: &Plan,
    split: usize,
    instances: &[Instance],
    image: &ImageContext,
    limits: &Limits,
    resample: Option<Resample>,
    at_most: Option<u32>,
) -> Result<Option<[u32; 2]>, ExportError> {
    if split == whole.operations.len() {
        return Ok(None);
    }
    let runs = runs(&whole.operations, split);
    let held = held_bytes(whole, &runs);
    let own = whole.bytes() - held;
    if at_most.is_none() && own <= limits.budget && whole.binds_within(limits) {
        log_held(held, own, &runs, "the whole image", limits);
        return Ok(None);
    }
    held_scratch_binds(whole, &runs, limits)?;
    let (_, after) = divide(instances, whole, split);
    let source = whole.operations[split - 1].output_desc;
    let mut edge = at_most.unwrap_or(8192);
    while edge >= SMALLEST_TILE {
        let mut largest = Some(0);
        for region in regions(output_extent(whole), [edge, edge]) {
            let pipe = Pipe::Tile { region, resample };
            let plan = pipeline::plan(&after, image, source, pipe, limits)?;
            let own = plan.bytes() - held_bytes(&plan, &runs);
            if own > limits.budget || !plan.binds_within(limits) {
                largest = None;
                break;
            }
            largest = largest.map(|bytes: u64| bytes.max(own));
        }
        if let Some(largest) = largest {
            log_held(held, largest, &runs, "each tile", limits);
            return Ok(Some([edge, edge]));
        }
        edge /= 2;
    }
    Err(ExportError::NoTiles {
        binding: limits.binding,
        budget: limits.budget,
    })
}

#[cfg(test)]
mod tests {
    use apertrust_modules::testing as modules;
    use apertrust_operations::{OPERATIONS, bloom, hazeremoval};

    use super::*;
    use crate::edit::{self, Edit};
    use crate::gpu::GpuOptions;

    /// An export handed a submission at a time, each stepped as the engine thread steps it,
    /// waiting between steps, gives what one rendered here gives, bit for bit: in tiles, at
    /// a size after demosaic and at finalscale's place, through bloom, whose first round of
    /// tiles holds its scratch across rounds (decision 15), the operations before the tiles
    /// (the raw stage on a default edit) in row strips, a step each, as beside a live
    /// frame, or in one step, as with none.
    #[test]
    fn a_handed_export_equals_one_rendered_where_it_runs() {
        let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default()))
            .expect("a GPU adapter, hardware or software");
        let image = modules::image();
        let mut instances = edit::instances(&Edit::default(), OPERATIONS, &[], &image)
            .all()
            .unwrap();
        let settings = bloom::Settings::default();
        instances.push(Instance {
            operation: &bloom::Bloom,
            params: bloom::values(&settings),
        });
        let input = || GpuImage::upload_mosaic(&gpu, 96, 72, &modules::mosaic()).unwrap();
        // The synthetic photo is 84 × 62.
        let half = |finalscale| Resample {
            output: [42, 31],
            scale: 0.5,
            finalscale,
        };
        // And with haze removal, whose first round at full resolution comes before the
        // resample without high quality and bloom's at the export's scale after it
        // (decision 15).
        let mut hazy = instances.clone();
        hazy.push(Instance {
            operation: &hazeremoval::HazeRemoval,
            params: hazeremoval::values(&hazeremoval::Settings {
                strength: 0.9,
                distance: 0.8,
            }),
        });
        // A job's tiling, which tiles where the budget would not: two tiles across.
        let job = Tiling::AtMost {
            budget: BUDGET,
            edge: SMALLEST_TILE,
        };
        let cases = [
            (&instances, Tiling::Whole, None, 1),
            (&instances, Tiling::Tiles([24, 20]), None, 2),
            (&instances, job, None, 2),
            (&instances, Tiling::Tiles([24, 20]), Some(half(false)), 2),
            (&instances, Tiling::Tiles([24, 20]), Some(half(true)), 2),
            (&hazy, Tiling::Tiles([24, 20]), Some(half(false)), 3),
        ];
        let strips = [Some(SYNTHETIC_STRIP), None];
        for ((instances, tiling, resample, rounds), strip) in cases
            .into_iter()
            .flat_map(|case| strips.map(|strip| (case, strip)))
        {
            let at = format!("{tiling:?} at {resample:?} in {rounds} rounds, strip {strip:?}");
            let here = export_at(&gpu, instances, &image, input(), tiling, resample);
            let here = pollster::block_on(here).unwrap_or_else(|e| panic!("{at}: {e}"));
            let mut hand = Stepped::new(&gpu, strip);
            let handed = export_handing(
                &gpu,
                instances,
                &image,
                input(),
                tiling,
                resample,
                &mut hand,
            );
            let handed = pollster::block_on(handed).unwrap_or_else(|e| panic!("{at}: {e}"));
            let Stepped { steps, .. } = hand;
            let submissions = steps.len();
            // Each tile's render one step, its readback's included: a turn of the engine
            // thread each, a readback taking no turn of its own (spike X1); the operations
            // before them, the raw stage on a default edit, a step per strip of each pass
            // and per pass without them, rawprepare, temperature and highlights' mask,
            // dilation, sums and inpainting in two strips, its chroma (`opposed_chroma`)
            // whole, and a step touching each new buffer before its first use, its two work
            // buffers and highlights' four scratch buffers; or, with no strip's size, one
            // step.
            match (tiling, strip) {
                (Tiling::Whole, _) => assert_eq!(steps, [1], "{at}"),
                (_, None) => assert!(steps.iter().all(|&n| n == 1), "{at}: {steps:?}"),
                _ => {
                    let touches = 2 + 4;
                    let before = 2 * 6 + 1 + touches;
                    assert_eq!(steps[0], before, "{at}: the steps before the tiles");
                    assert!(steps[1..].iter().all(|&n| n == 1), "{at}: {steps:?}");
                }
            }
            assert_eq!(
                (handed.tiles, handed.rounds, handed.desc),
                (here.tiles, here.rounds, here.desc),
                "{at}"
            );
            assert_eq!(here.rounds, rounds, "{at}");
            assert!(tiling == Tiling::Whole || here.tiles > 1, "{at}: one tile");
            // Whole, one; tiled, the operations before the tiles, then each tile of each
            // round, all of the output's but haze removal's, at full resolution.
            match (tiling, rounds) {
                (Tiling::Whole, _) => assert_eq!(submissions, 1, "{at}"),
                (_, 2) => assert_eq!(submissions, 1 + here.tiles * 2, "{at}"),
                _ => assert!(submissions > 1 + here.tiles * 2, "{at}: {submissions}"),
            }
            assert!(handed.pixels == here.pixels, "{at}: the pixels differ");
        }
    }

    /// What a strip of the synthetic mosaic binds at most in
    /// [`a_handed_export_equals_one_rendered_where_it_runs`]: 66 of the 68 rows of 92
    /// f32 photosites rawprepare's crop leaves, less up to an alignment, so that every
    /// pass that takes strips takes two, highlights' sums, in strips of 64 rows and a row
    /// above and below, and its dilation, in bands of 22 of its 24 rows, included.
    const SYNTHETIC_STRIP: u64 = 24 << 10;

    /// A [`Hand`] that steps each render as the engine thread steps it, waiting for its
    /// work before the next, as the engine thread's polls do, and counts each render's
    /// steps, those of the operations before the tiles in strips of `strip` bytes, or one
    /// with none.
    struct Stepped<'a> {
        gpu: &'a GpuContext,
        strip: Option<u64>,
        steps: Vec<usize>,
    }

    impl<'a> Stepped<'a> {
        fn new(gpu: &'a GpuContext, strip: Option<u64>) -> Self {
            let steps = Vec::new();
            Self { gpu, strip, steps }
        }
    }

    impl Hand for Stepped<'_> {
        fn hand(&mut self, mut submit: Submit) -> Result<Rendered, TaskError> {
            self.steps.push(0);
            loop {
                *self.steps.last_mut().expect("this render's") += 1;
                let step = submit.step(self.gpu).map_err(TaskError::Failed)?;
                self.gpu.wait().unwrap();
                if let Step::Done(rendered) = step {
                    return Ok(rendered);
                }
            }
        }

        fn wait(&mut self, mapping: &Mapping) -> Result<MapResult, TaskError> {
            // The wait after the step answered it.
            mapping.try_recv().map_err(|_| TaskError::Stopped)
        }

        fn strip(&self) -> Option<u64> {
            self.strip
        }
    }

    /// A job's export of the corpus's largest raws, the 7R V's 61 MP and the X-T5's 40 MP
    /// X-Trans, the operations before its tiles (the raw stage on a default edit) in row
    /// strips of [`STRIP_BINDING`] a step each, as beside a live frame, or in one step, as
    /// with none, gives what the same tiles rendered here give, bit for bit, so its file is
    /// the same too.
    #[test]
    #[ignore = "needs the raw.pixls.us corpus: tools/fetch-raw-corpus.sh"]
    fn a_job_s_raw_stage_in_strips_or_whole_exports_the_corpus_s_largest_raws_unchanged() {
        use crate::export_job::{LARGEST_TILE, STRIP_BINDING};
        use crate::raw;
        let corpus = std::env::var_os("APERTRUST_RAW_CORPUS").map_or_else(
            || {
                let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
                manifest.join("../../test-images/raw.pixls.us")
            },
            std::path::PathBuf::from,
        );
        let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default()))
            .expect("a GPU adapter, hardware or software");
        let tiling = Tiling::AtMost {
            budget: BUDGET,
            edge: LARGEST_TILE,
        };
        let raws = [
            "Sony/ILCE-7RM5/7RM5-LosslessCompressedLarge.ARW",
            "Fujifilm/X-T5/DSCF0021.RAF",
        ];
        for name in raws {
            let decoded = apertrust_raw::decode_file(corpus.join(name)).expect("the raw decodes");
            let input = || raw::upload(&gpu, &decoded).expect("a mosaic").unwrap();
            let image = raw::image_context(&decoded);
            let instances = edit::instances(&Edit::default(), OPERATIONS, &[], &image)
                .all()
                .unwrap();
            let here = export_at(&gpu, &instances, &image, input(), tiling, None);
            let here = pollster::block_on(here).unwrap_or_else(|e| panic!("{name}: {e}"));
            for strip in [Some(STRIP_BINDING), None] {
                let mut hand = Stepped::new(&gpu, strip);
                let handed =
                    export_handing(&gpu, &instances, &image, input(), tiling, None, &mut hand);
                let at = format!("{name}, strip {strip:?}");
                let handed = pollster::block_on(handed).unwrap_or_else(|e| panic!("{at}: {e}"));
                // The raw stage's seven passes, a default edit's operations before the
                // tiles, in more steps than seven, in strips; or in one.
                match strip {
                    Some(_) => assert!(hand.steps[0] > 7, "{at}: {:?}", &hand.steps[..2]),
                    None => assert_eq!(hand.steps[0], 1, "{at}"),
                }
                assert_eq!(handed.tiles, here.tiles, "{at}");
                assert!(handed.pixels == here.pixels, "{at}: the pixels differ");
            }
        }
    }

    /// A job's tiling always tiles, at most its edge, where the budget would run the whole
    /// image, and never larger than the tiles the budget's search finds, so a budget that
    /// asks for smaller tiles gets them.
    #[test]
    fn a_job_s_tiles_are_at_most_its_edge_and_the_budget_s() {
        let image = modules::image();
        let instances = edit::instances(&Edit::default(), OPERATIONS, &[], &image)
            .all()
            .unwrap();
        let mosaic = ImageDesc::new(96, 72, apertrust_plugin::Format::Mosaic16);
        let whole = pipeline::plan(&instances, &image, mosaic, Pipe::Export, &Limits::NONE);
        let whole = whole.unwrap();
        let split = split(&whole.operations);
        let tiles = |budget, at_most| {
            let limits = Limits {
                budget,
                ..Limits::NONE
            };
            auto_tile(&whole, split, &instances, &image, &limits, None, at_most).unwrap()
        };
        assert_eq!(tiles(BUDGET, None), None, "the whole image fits");
        assert_eq!(tiles(BUDGET, Some(1024)), Some([1024, 1024]));
        // The 84 × 62 photo in one tile takes one byte more than this budget, and in the
        // search's next tiles, of 64, two across, it fits.
        let (_, after) = divide(&instances, &whole, split);
        let source = whole.operations[split - 1].output_desc;
        let pipe = Pipe::Tile {
            region: [0, 0, 84, 62],
            resample: None,
        };
        let one = pipeline::plan(&after, &image, source, pipe, &Limits::NONE).unwrap();
        let budget = one.bytes() - 1;
        assert_eq!(tiles(budget, None), Some([64, 64]));
        assert_eq!(tiles(budget, Some(1024)), Some([64, 64]));
    }

    /// A tiled export's line adds up its renders, the GPU's times only where the device
    /// gave them all.
    #[test]
    fn a_tiled_export_adds_up_its_renders() {
        let render = |wall_ms, passes: &[(&'static str, f64)]| Timings {
            passes: passes.to_vec(),
            wall_ms,
        };
        let renders = [
            render(6.6, &[("rawprepare", 4.5)]),
            render(300.3, &[("rcd_step_1", 150.0), ("agx", 100.5)]),
            render(325.0, &[("rcd_step_1", 160.0), ("agx", 95.0)]),
        ];
        assert_eq!(
            exported(2, 1195.4, &renders),
            "exported 2 tiles in 1195 ms, rendering: 631.9 ms, GPU: 510.0 ms"
        );
        let untimed = [render(6.6, &[("rawprepare", f64::NAN)]), render(300.0, &[])];
        assert_eq!(
            exported(1, 400.0, &untimed),
            "exported 1 tile in 400 ms, rendering: 306.6 ms"
        );
        // A job's renders are untimed.
        assert_eq!(exported(3, 812.4, &[]), "exported 3 tiles in 812 ms");
    }

    /// A run's side of the resample is its place in the plan (decision 15), not whether
    /// its stage at the pipe's scale differs from its stage at full resolution. The two
    /// rules agree wherever an export can tell them apart: before the resample the stage
    /// at the pipe's scale is the stage itself, and after it the output, which equals the
    /// stage only at a scale of exactly 1, where a round in the output's tiles and one in
    /// tiles at full resolution take the same tiles and make the same pixels. So only
    /// this test, of the rule itself, fails under the extents': the tone equalizer after
    /// a resample at 1 is past it, its stage at the export's scale its stage, and haze
    /// removal before it is not.
    #[test]
    fn a_runs_side_of_the_resample_is_its_place_in_the_plan() {
        use apertrust_operations::hazeremoval::HazeRemoval;
        use apertrust_operations::toneequal::ToneEqual;
        use apertrust_plugin::ParamValues;
        let [width, height] = [96, 64];
        let instances =
            [&ToneEqual as &dyn apertrust_plugin::Operation, &HazeRemoval].map(|operation| {
                Instance {
                    operation,
                    params: ParamValues::new(),
                }
            });
        let resample = Resample {
            output: [width, height],
            scale: 1.0,
            finalscale: false,
        };
        let whole = pipeline::plan(
            &instances,
            &ImageContext::full(width, height),
            ImageDesc::rgba(width, height),
            Pipe::Resampled(resample),
            &Limits::NONE,
        )
        .unwrap();
        let ids: Vec<&str> = whole.operations.iter().map(|op| op.id).collect();
        assert_eq!(ids, ["hazeremoval", "toneequal"], "no pass at a scale of 1");
        assert_eq!(whole.resample, Some(1..1));
        let tone = &whole.operations[1].output_desc;
        assert_eq!(tone.scaled_extent, tone.extent);
        assert!(past_the_resample(&whole, "toneequal"));
        assert!(!past_the_resample(&whole, "hazeremoval"));
    }

    #[test]
    fn tiles_cover_the_image_and_none_is_narrower_than_demosaic_needs() {
        let check = |length: u32, size: u32| {
            let spans = spans(length, size);
            assert_eq!(
                spans.len(),
                length.div_ceil(size) as usize,
                "{length} by {size}"
            );
            let mut next = 0;
            for &(start, span) in &spans {
                assert_eq!(start, next, "{length} by {size}: {spans:?}");
                assert!(span <= size, "{length} by {size}: {spans:?}");
                if spans.len() > 1 {
                    assert!(
                        span >= NARROWEST_TILE.min(size / 2),
                        "{length} by {size}: {spans:?}"
                    );
                }
                next += span;
            }
            assert_eq!(next, length);
            spans
        };
        // The 7R V's declared width in tiles of 4096: the last is wide enough as it is.
        assert_eq!(check(9504, 4096), [(0, 4096), (4096, 4096), (8192, 1312)]);
        // One pixel over a tile: the last two share it.
        assert_eq!(check(8193, 4096), [(0, 4096), (4096, 2048), (6144, 2049)]);
        for length in [1, 31, 32, 33, 98, 1000, 6336, 9504] {
            for size in [24, 37, 64, 256, 2048, 8192] {
                check(length, size);
            }
        }
    }
}
