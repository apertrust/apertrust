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

//! The pipeline (`docs/engine.md`): a plan of operation passes for a pipe, and the GPU
//! resources that run it. Four pipes so far: export, the whole image at full
//! resolution; an export at a size, resampled as darktable resamples it (decision 56);
//! the viewport, whose raw stage and demosaic run at full resolution and whose later
//! operations run at the viewport's size (decision 12); and a tile of an export
//! (decision 13). One dispatch per pass, or one per row strip when an image it
//! binds is larger than the GPU binds at once (decision 14). A render restarts from the
//! newest cache point a change leaves valid (decision 12), in the viewport skipping the
//! whole-image passes whose result their scratch still holds (decision 15), and a
//! pipeline rebuilt over the same input keeps its compiled kernels and the cache points
//! and whole-image results that still hold (decision 28).

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::num::NonZeroU64;
use std::ops::Range;
use std::time::Instant;

use apertrust_plugin::{
    Binding, Extent, Format, ImageContext, ImageDesc, Kernel, Kind, NotPorted, Operation,
    OutputProfile, ParamError, ParamValues, Pass, PassHeader, PassKind, PipeKind, Prepared,
    Resource, Schema, Slot, Stage, Strips, Value,
};
use thiserror::Error;
use wgpu::util::DeviceExt;

use crate::compute::{
    ComputeError, DERIVATIVE_IN_BINDING, DERIVATIVE_OUT_BINDING, GpuImage, GpuTimer, ImageKernel,
    Timings, WORKGROUP,
};
use crate::gpu::GpuContext;
use crate::resample::{AxisPlan, input_length};

/// Generated from `slang/downscale.slang`: the viewport pipe's downscale, a box filter.
pub const DOWNSCALE: Kernel = Kernel {
    name: "downscale",
    wgsl: include_str!("../generated/downscale.wgsl"),
    derivatives: false,
};

/// Generated from `slang/region.slang`: a tile's input, copied out of the whole image.
pub const REGION: Kernel = Kernel {
    name: "region",
    wgsl: include_str!("../generated/region.wgsl"),
    derivatives: false,
};

/// Generated from `slang/resample.slang`: an export's resample to its size, darktable's
/// lanczos3 (decision 56; [`crate::resample`]).
pub const RESAMPLE: Kernel = Kernel {
    name: "resample",
    wgsl: include_str!("../generated/resample.wgsl"),
    derivatives: false,
};

/// The engine's own kernels, besides the operations': the pipeline's, and the scopes',
/// which count each viewport frame after it ([`crate::scopes`]).
pub const KERNELS: &[Kernel] = &[DOWNSCALE, REGION, RESAMPLE, crate::scopes::KERNEL];

/// darktable's finalscale place, display 69.4 between vignette and colorout, where an
/// export at a size resamples with high quality resampling or above a scale of 1
/// (decision 56).
pub const FINALSCALE: Slot = Slot {
    stage: Stage::Display,
    order: 69.4,
};

/// The GPU memory a render may take before it tiles, the engine's budget by default
/// (decision 12).
pub const BUDGET: u64 = 2 << 30;

/// What a plan must fit (decision 14): the GPU's limits, which a plan never exceeds, and
/// the engine's memory budget, beyond which an export tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The most bytes of a storage buffer a kernel binds at once: wgpu's
    /// `max_storage_buffer_binding_size`. A pass whose image is larger runs in row
    /// strips, binding a window of it each, if it says how ([`Strips`]).
    pub binding: u64,
    /// The largest buffer the GPU makes: `max_buffer_size`.
    pub buffer: u64,
    /// The offset of a bound window is a multiple of this many bytes:
    /// `min_storage_buffer_offset_alignment`.
    pub offset_alignment: u32,
    /// The GPU memory a render may take before an export tiles (decision 12).
    pub budget: u64,
}

impl Limits {
    /// No limits: every pass runs whole, as a GPU that binds anything runs it.
    pub const NONE: Self = Self {
        binding: u64::MAX,
        buffer: u64::MAX,
        offset_alignment: 256,
        budget: u64::MAX,
    };

    /// The device's limits, with the engine's budget.
    pub fn of(gpu: &GpuContext) -> Self {
        let limits = gpu.device.limits();
        Self {
            binding: limits.max_storage_buffer_binding_size,
            buffer: limits.max_buffer_size,
            offset_alignment: limits.min_storage_buffer_offset_alignment,
            budget: BUDGET,
        }
    }
}

/// An operation to run, with the parameters an edit sets; the rest keep the
/// operation's defaults for the image.
#[derive(Clone)]
pub struct Instance<'a> {
    pub operation: &'a dyn Operation,
    pub params: ParamValues,
}

/// An export at a size other than full resolution (decision 56;
/// `docs/ports/finalscale.md`): the size `apertrust-export`'s `size` computes, and where
/// darktable resamples it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resample {
    /// The output image: `floor(scale × W)` by `floor(scale × H)` of the image W × H at
    /// full resolution, as darktable rounds it in double.
    pub output: [u32; 2],
    /// darktable's export scale in f32, as its pipe's region of interest holds it: the
    /// scale the resample's plans take, and every operation after it sees. At 1, where a
    /// double scale such as 0.99999999 rounds to 1 in f32 but its output is a pixel short
    /// of the image, darktable's resample copies (`copymode`): the planner cuts the
    /// output's part of the image out instead of resampling it.
    pub scale: f32,
    /// Whether it resamples at finalscale's place ([`FINALSCALE`]), every operation
    /// before it at full resolution, rather than where the raw stage ends, or after haze
    /// removal while it is planned (decision 15), every later operation at the export's
    /// scale: darktable's `hq_process`, high quality resampling or a scale above 1.
    pub finalscale: bool,
}

/// Which render a plan is for (`docs/engine.md`, Pipes).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pipe {
    /// The whole image at full resolution: export and the CLI.
    Export,
    /// The whole image at an export's size (decision 56): resampled where the raw stage
    /// ends, or after haze removal while it is planned (decision 15), every later
    /// operation at the export's scale, or at finalscale's place, every operation before
    /// it at full resolution. The input is the export's at full resolution.
    Resampled(Resample),
    /// The image fitted into `width` × `height` physical pixels. The raw stage and
    /// demosaic run at full resolution and their result is downscaled once; every later
    /// operation runs at the viewport's size (decision 12).
    Viewport { width: u32, height: u32 },
    /// One tile of an export: `region`, x, y, width and height of the output image at
    /// full resolution, the developed image's or, after an operation that moves pixels,
    /// the last one's output (decision 16), from an input that holds the whole image as
    /// far as the operations before the tiles took it. The plan first copies the part of
    /// the input the tile's operations need, the region they read grown by their radii
    /// (decision 13). In an export at a size, `region` is the output image's at its
    /// scale, and `resample` the export's: the tile resamples at its place, walked back
    /// through the resample's plans to the part of the image at full resolution the
    /// operations before it make, unless its input is past that place already, an RGBA
    /// image of the export's scale (decision 56).
    Tile {
        region: [u32; 4],
        resample: Option<Resample>,
    },
}

/// Which image buffer an operation reads or writes: the pipe's input, or a working
/// buffer of the plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Buffer {
    Input,
    Work(usize),
}

/// One dispatch of a pass: the whole of it, or a strip of its rows with a window of each
/// image it binds (decision 14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dispatch {
    /// The first row of the pass's invocations it runs, and how many rows: the pass
    /// header's origin and height.
    pub rows: [u32; 2],
    /// The part of the input's and the output's buffers it binds; `None` binds all of
    /// it.
    pub input: Option<Window>,
    pub output: Option<Window>,
}

/// A range of a buffer bound to a kernel, in bytes: its offset is a multiple of the
/// GPU's offset alignment, its size of 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    pub offset: u64,
    pub size: u64,
}

impl Window {
    /// The 32-bit word it starts at, for the pass header.
    fn base(window: Option<Self>) -> u32 {
        window.map_or(0, |w| (w.offset / 4) as u32)
    }
}

/// One operation of the plan, prepared.
#[derive(Debug, Clone)]
pub struct PlannedOperation {
    pub id: &'static str,
    pub kernel: &'static Kernel,
    pub input: Buffer,
    pub output: Buffer,
    /// The image the operation writes; every pass dispatches one invocation per pixel
    /// of it.
    pub output_desc: ImageDesc,
    pub passes: Vec<Pass>,
    /// How each pass dispatches: once over all of it, or once per row strip.
    pub dispatches: Vec<Vec<Dispatch>>,
    /// The operation's scratch buffers, as indices into [`Plan::scratch`].
    pub scratch: Vec<usize>,
    /// Pixels at the operation's scale per pixel at full resolution: 1 up to the
    /// viewport's downscale, the viewport's scale after it.
    pub scale: f32,
    /// Where its output starts in its stage's image at its scale, for the pass header: a
    /// tile's corner after the raw stage, which places its images on the sensor itself,
    /// or with geometry its output region's (decision 16).
    pub origin: [u32; 2],
    /// Its stage's image at full resolution, for the pass header's full size: the
    /// developed image, or after an operation that moves pixels, that operation's output,
    /// its own included (decision 16).
    pub extent: [u32; 2],
    /// What its output depends on: its own prepared bytes and every earlier
    /// operation's. A cache point holds valid data while its key matches (decision 12).
    pub key: u64,
    /// Whether the operation said an export may run its whole-image passes in a first
    /// round of tiles ([`Operation::tiles_whole_image_passes`]; decision 15).
    pub tiles_whole_image_passes: bool,
    /// In the viewport pipe, its whole-image passes and their key, which a render skips
    /// while the operation's scratch holds their result (decision 15); `None` without
    /// them, and in an export, whose rounds of tiles hold the result by construction.
    pub whole_image: Option<WholeImage>,
    /// In the viewport, the buffer its passes bind as [`Resource::FullResolution`], the
    /// full-resolution image the downscale read, for an operation whose whole-image
    /// results the viewport computes at full resolution (decision 15); `None` for every
    /// other operation.
    pub full_resolution: Option<Buffer>,
}

/// An operation's whole-image passes in the viewport, and the key of their result
/// (decision 15): its passes up to its last whole-image one, keyed on their input's key
/// and their own prepared bytes, never on its later passes', so a change to those alone
/// leaves the result in its scratch, and a render runs only the passes after them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WholeImage {
    /// How many of the operation's passes, from its first, make the result.
    pub passes: usize,
    /// What the result depends on: the input's key, the operation's ID, version and
    /// scale, its output image, those passes' entry points, kinds, sizes, bindings,
    /// uniforms and tables, and its scratch buffers' sizes; no origin, since they cover
    /// the whole image, and no row strips, which change only how they dispatch.
    pub key: u64,
    /// For an operation prepared with the full-resolution image the viewport's downscale
    /// read, the part of those passes that runs over it, kept on its own key, so a new
    /// viewport size runs only the passes after it (decision 15); `None` for every
    /// other operation, and for one whose part depends on the viewport's size.
    pub full_resolution: Option<FullResolutionPart>,
}

/// An operation's whole-image passes up to the last that binds
/// [`Resource::FullResolution`], and the key of their result (decision 15): nothing in
/// it holds the viewport's size, so their result outlives a resize, while the rest of
/// the whole-image passes, which bring it to the viewport's size, run again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FullResolutionPart {
    /// How many of the operation's passes, from its first, make the part's result.
    pub passes: usize,
    /// What it depends on: the full-resolution image's key, the operation's ID and
    /// version, scale 1, those passes' entry points, kinds, sizes, bindings, uniforms and
    /// tables, and the sizes of the scratch buffers they bind.
    pub key: u64,
}

/// The operations of a pipe in order, with their buffers: plain data, built without a
/// GPU.
#[derive(Debug, Clone)]
pub struct Plan {
    pub pipe: Pipe,
    pub image: ImageContext,
    pub input: ImageDesc,
    pub operations: Vec<PlannedOperation>,
    /// The working buffers' images: operations alternate between them, and one is
    /// reused whenever its size and format fit, it is not the operation's input and no
    /// cache point owns it.
    pub work: Vec<ImageDesc>,
    /// The largest image the plan's operations write in each working buffer: the
    /// buffer's own, but for a viewport's tiles, whose buffers have room for a tile a
    /// footprint and a colour pattern larger (decision 14). A pipeline takes a plan whose images its buffers
    /// hold.
    pub work_used: Vec<ImageDesc>,
    /// The scratch buffers' sizes in bytes.
    pub scratch: Vec<u64>,
    /// The bytes the plan's operations use of each scratch buffer, as `work_used`.
    pub scratch_used: Vec<u64>,
    /// The operations whose outputs are cache points, kept between renders in buffers
    /// of their own: in the viewport pipe, the full-resolution image the raw stage ends
    /// with and its downscale (`docs/engine.md`, Caching).
    pub cached: Vec<usize>,
    /// The image of the cache point at the edited operation's input, the viewport's;
    /// `None` for the export pipe. It is the largest image after its start, and holds any
    /// of them by bytes, since after a geometry they need not all be its size: a quarter
    /// turn swaps the sides (decision 16).
    pub edit_cache: Option<ImageDesc>,
    /// The operations of the viewport's tiles, when it has them (decision 14): per tile,
    /// its copy, the raw stage's operations after the last reduction, and its downscale.
    /// No render restarts inside them, and none keeps an input of theirs.
    pub tiled: Option<Range<usize>>,
    /// In an export at a size, the operations its resample adds (decision 56): its own,
    /// its copy of the output's part of the image at a scale of 1, or none where that is
    /// the whole image; every later operation runs at the export's scale, and every
    /// earlier one at full resolution (decision 15). Empty at the start in a tile whose
    /// input is past it; `None` in every other plan.
    pub resample: Option<Range<usize>>,
}

impl Plan {
    /// Where the result is, and what it is: the last operation's output, or the input
    /// if nothing runs.
    pub fn output(&self) -> (Buffer, ImageDesc) {
        self.operations
            .last()
            .map_or((Buffer::Input, self.input), |op| {
                (op.output, op.output_desc)
            })
    }

    /// The output's stage at full resolution: the developed image, or the last geometry's
    /// output (decision 16).
    pub fn extent(&self) -> [u32; 2] {
        self.operations
            .last()
            .map_or([self.image.width, self.image.height], |op| op.extent)
    }

    /// Every pass of every operation, in dispatch order.
    pub fn passes(&self) -> impl Iterator<Item = (&PlannedOperation, &Pass)> {
        self.operations
            .iter()
            .flat_map(|op| op.passes.iter().map(move |pass| (op, pass)))
    }

    /// The GPU memory its buffers take: the input, the working and scratch buffers and
    /// the edit cache.
    pub fn bytes(&self) -> u64 {
        self.buffers().sum()
    }

    /// The size of each buffer the plan needs, in bytes.
    fn buffers(&self) -> impl Iterator<Item = u64> + '_ {
        self.work
            .iter()
            .chain(&self.edit_cache)
            .map(ImageDesc::buffer_size)
            .chain(self.scratch.iter().copied())
            .chain([self.input.buffer_size()])
    }

    /// Whether the GPU can run the plan within `limits`: no buffer larger than it makes,
    /// none bound larger than it binds, and all of them within the budget.
    pub fn fits(&self, limits: &Limits) -> bool {
        self.bytes() <= limits.budget && self.binds_within(limits)
    }

    /// Whether the GPU can run the plan at all: no buffer larger than it makes, none
    /// bound larger than it binds.
    pub fn binds_within(&self, limits: &Limits) -> bool {
        self.buffers().all(|bytes| bytes <= limits.buffer)
            && self
                .oversized_binding(limits.binding, self.input.buffer_size())
                .is_none()
    }

    /// The GPU memory the viewport's tiles take of their own: the buffers every tile
    /// shares and their operations' scratch buffers, not the viewport image.
    fn tile_bytes(&self) -> u64 {
        let Some(tiled) = &self.tiled else {
            return 0;
        };
        let operations = &self.operations[tiled.clone()];
        let viewport = operations.last().map(|op| op.output);
        let mut work: Vec<usize> = operations
            .iter()
            .filter(|op| Some(op.output) != viewport)
            .filter_map(|op| match op.output {
                Buffer::Work(index) => Some(index),
                Buffer::Input => None,
            })
            .collect();
        let mut scratch: Vec<usize> = operations
            .iter()
            .flat_map(|op| op.scratch.iter().copied())
            .collect();
        for indices in [&mut work, &mut scratch] {
            indices.sort_unstable();
            indices.dedup();
        }
        work.iter()
            .map(|&index| self.work[index].buffer_size())
            .chain(scratch.iter().map(|&index| self.scratch[index]))
            .sum()
    }

    /// The first binding of more than `limit` bytes, with its operation and entry point:
    /// a window where the pass binds one, otherwise the whole buffer, the input's being
    /// `input_bytes`.
    fn oversized_binding(
        &self,
        limit: u64,
        input_bytes: u64,
    ) -> Option<(&'static str, &'static str, u64)> {
        self.operations
            .iter()
            .find_map(|op| self.oversized_in(op, limit, input_bytes))
    }

    /// `op`'s first binding of more than `limit` bytes, with its entry point.
    fn oversized_in(
        &self,
        op: &PlannedOperation,
        limit: u64,
        input_bytes: u64,
    ) -> Option<(&'static str, &'static str, u64)> {
        let whole = |buffer: Buffer| match buffer {
            Buffer::Input => input_bytes,
            Buffer::Work(index) => self.work[index].buffer_size(),
        };
        op.passes
            .iter()
            .zip(&op.dispatches)
            .find_map(|(pass, dispatches)| {
                let largest = dispatches
                    .iter()
                    .flat_map(|dispatch| {
                        pass.bindings.iter().map(|binding| match binding.resource {
                            Resource::Input => dispatch.input.map_or(whole(op.input), |w| w.size),
                            Resource::Output => {
                                dispatch.output.map_or(whole(op.output), |w| w.size)
                            }
                            Resource::Table(index) => pass.tables[index].len() as u64,
                            Resource::Scratch(index) => self.scratch[op.scratch[index]],
                            Resource::FullResolution => op.full_resolution.map_or(0, whole),
                            Resource::Uniforms => 0,
                        })
                    })
                    .max()
                    .unwrap_or(0);
                (largest > limit).then_some((op.id, pass.entry_point, largest))
            })
    }

    /// The first operation outside the viewport's tiles that binds more than the GPU
    /// binds at once, or writes a buffer larger than it makes: its index, and the bytes
    /// and the limit.
    fn oversized_outside_tiles(&self, limits: &Limits) -> Option<(usize, u64, u64)> {
        let input = self.input.buffer_size();
        if input > limits.buffer {
            return Some((0, input, limits.buffer));
        }
        let tiled = self.tiled.clone().unwrap_or_default();
        self.operations.iter().enumerate().find_map(|(index, op)| {
            if tiled.contains(&index) {
                return None;
            }
            if let Some((_, _, bytes)) = self.oversized_in(op, limits.binding, input) {
                return Some((index, bytes, limits.binding));
            }
            let output = match op.output {
                Buffer::Work(work) => self.work[work].buffer_size(),
                Buffer::Input => 0,
            };
            op.scratch
                .iter()
                .map(|&scratch| self.scratch[scratch])
                .chain([output])
                .find(|&bytes| bytes > limits.buffer)
                .map(|bytes| (index, bytes, limits.buffer))
        })
    }

    /// Keeps only the passes of operation `id` that `keep` accepts, with their dispatches:
    /// each round of an export's tiles runs part of an operation with whole-image passes
    /// (decision 15). Keys stay as planned: an export's tiles have no cache points, and
    /// skip whole-image passes by leaving them out, never by their key.
    pub(crate) fn retain_passes(&mut self, id: &str, keep: impl Fn(&Pass) -> bool) {
        for op in self.operations.iter_mut().filter(|op| op.id == id) {
            let kept: Vec<(Pass, Vec<Dispatch>)> = std::mem::take(&mut op.passes)
                .into_iter()
                .zip(std::mem::take(&mut op.dispatches))
                .filter(|(pass, _)| keep(pass))
                .collect();
            (op.passes, op.dispatches) = kept.into_iter().unzip();
            op.whole_image = None;
        }
    }

    /// Whether the edit cache may keep the input of operation `op` (decision 12): the
    /// plan has one, `op` is not the first, no cache point holds that input already, it
    /// is not inside the viewport's tiles, whose buffers every tile shares, the edit
    /// cache holds its bytes, whatever its width and height (decision 16), and `op` runs
    /// a pass, where the copies happen: a geometry with none writes nothing, its output
    /// its input's buffer (decision 16).
    fn edit_cache_takes(&self, op: usize) -> bool {
        let Some(edit) = self.edit_cache else {
            return false;
        };
        let cached_input = self
            .cached
            .iter()
            .any(|&c| self.operations[c].output == self.operations[op].input);
        let in_tiles = self.tiled.as_ref().is_some_and(|t| t.contains(&op));
        let holds = |input: &ImageDesc| {
            input.format == edit.format && input.buffer_size() <= edit.buffer_size()
        };
        op > 0
            && !cached_input
            && !in_tiles
            && holds(&self.operations[op - 1].output_desc)
            && !self.operations[op].passes.is_empty()
    }

    /// Whether `other` runs the same passes, with the same bindings, over buffers in the
    /// same roles, so a pipeline built for this plan can run it once its buffers are
    /// large enough (decision 13): only sizes, uniforms and tables may differ.
    fn same_shape(&self, other: &Plan) -> bool {
        // The same number of dispatches, binding windows of the same images; the rows
        // and windows may differ.
        let dispatch_shape = |dispatches: &[Dispatch]| {
            dispatches
                .iter()
                .map(|d| (d.input.is_some(), d.output.is_some()))
                .collect::<Vec<_>>()
        };
        let same_op = |a: &PlannedOperation, b: &PlannedOperation| {
            (a.id, a.input, a.output, &a.scratch, a.full_resolution)
                == (b.id, b.input, b.output, &b.scratch, b.full_resolution)
                && a.output_desc.format == b.output_desc.format
                && a.passes.len() == b.passes.len()
                && a.dispatches
                    .iter()
                    .zip(&b.dispatches)
                    .all(|(p, q)| dispatch_shape(p) == dispatch_shape(q))
                && a.passes.iter().zip(&b.passes).all(|(p, q)| {
                    (
                        p.entry_point,
                        p.kind,
                        &p.bindings,
                        p.params.len(),
                        p.tables.len(),
                    ) == (
                        q.entry_point,
                        q.kind,
                        &q.bindings,
                        q.params.len(),
                        q.tables.len(),
                    )
                })
        };
        let pipe = |pipe: Pipe| match pipe {
            Pipe::Tile { resample, .. } => Pipe::Tile {
                region: [0; 4],
                resample,
            },
            Pipe::Viewport { .. } => Pipe::Viewport {
                width: 0,
                height: 0,
            },
            Pipe::Export | Pipe::Resampled(_) => pipe,
        };
        pipe(self.pipe) == pipe(other.pipe)
            && (self.image.width, self.image.height, &self.image.camera)
                == (other.image.width, other.image.height, &other.image.camera)
            && self.input == other.input
            && self.work.len() == other.work.len()
            && self.scratch.len() == other.scratch.len()
            && self.cached == other.cached
            && self.tiled == other.tiled
            && self.edit_cache.is_some() == other.edit_cache.is_some()
            && self.operations.len() == other.operations.len()
            && self
                .operations
                .iter()
                .zip(&other.operations)
                .all(|(a, b)| same_op(a, b))
    }
}

#[derive(Debug, Error)]
pub enum PlanError {
    #[error("{operation}: {source}")]
    Param {
        operation: &'static str,
        source: ParamError,
    },
    #[error(transparent)]
    NotPorted(#[from] NotPorted),
    #[error("{operation} runs in the {stage:?} stage, but its input is {format:?}")]
    Format {
        operation: &'static str,
        stage: Stage,
        format: Format,
    },
    #[error("{operation}: a pass binds {resource:?}, which it does not have")]
    Binding {
        operation: &'static str,
        resource: Resource,
    },
    #[error("{operation} leaves an empty image, {desc}")]
    Empty {
        operation: &'static str,
        desc: ImageDesc,
    },
    #[error("a tile cannot start from a {0:?} image")]
    TileInput(Format),
    #[error("{operation} changes its {what} from one tile of the viewport to another")]
    TileShape {
        operation: &'static str,
        what: &'static str,
    },
    #[error(
        "{operation} made {desc} for the viewport's tile at {region:?}, not that part of the image"
    )]
    TileRegion {
        operation: &'static str,
        desc: ImageDesc,
        region: [u32; 4],
    },
    #[error("{operation}: {what}")]
    Geometry {
        operation: &'static str,
        what: &'static str,
    },
    #[error(
        "{operation} {} {desc} where the plan asks for {}×{} at {},{}",
        if *made { "made" } else { "got" },
        region[2],
        region[3],
        region[0],
        region[1]
    )]
    Region {
        operation: &'static str,
        /// Whether `desc` is what it made, or what it was given.
        made: bool,
        desc: ImageDesc,
        region: [u32; 4],
    },
    #[error(
        "{operation} needs {:.1} MiB at the {width}×{height} viewport's size, more than the GPU allows ({:.1} MiB){}",
        mebibytes(*bytes),
        mebibytes(*limit),
        if *smaller_fits { ": a smaller viewport fits" } else { "" }
    )]
    Viewport {
        operation: &'static str,
        width: u32,
        height: u32,
        bytes: u64,
        limit: u64,
        /// Whether what does not fit is the viewport image, which a smaller viewport
        /// makes smaller.
        smaller_fits: bool,
    },
}

/// Plans the instances in pipeline order for `pipe` (decisions 20 and 21): each
/// operation's parameters are resolved over its defaults for the image and prepared for
/// the image it receives, with the pipe's output profile the instances report in its
/// image context, or where none reports one, the profile `image` brings, as an export's
/// parts before and inside its tiles bring the whole export's (decision 26), and its
/// output goes to a working buffer, so the input stays untouched for the next render.
/// The viewport pipe downscales the image once, before the first operation after the raw
/// stage, tile by tile when its full resolution does not fit (decision 14). A pass that
/// binds an image larger than `limits` allows runs in row strips, if it can; whether the
/// whole plan fits is [`Plan::fits`]'s to say. With an operation that moves pixels,
/// every operation after the raw stage makes the region of its stage's image the next
/// one reads (decision 16; `docs/engine.md`, Geometry).
pub fn plan(
    instances: &[Instance],
    image: &ImageContext,
    input: ImageDesc,
    pipe: Pipe,
    limits: &Limits,
) -> Result<Plan, PlanError> {
    let mut ordered: Vec<&Instance> = instances.iter().collect();
    ordered.sort_by(|a, b| {
        a.operation
            .manifest()
            .slot
            .pipeline_order(&b.operation.manifest().slot)
    });
    // The profile is `prepare`'s alone (decision 26): neither the instances' defaults
    // nor their geometry see it. The pipe's kind every instance sees (decision 16).
    let bare = ImageContext {
        output_profile: None,
        pipe: match pipe {
            Pipe::Export | Pipe::Resampled(_) | Pipe::Tile { .. } => PipeKind::Export,
            Pipe::Viewport { .. } => PipeKind::Viewport,
        },
        ..image.clone()
    };
    let output_profile = output_profile(&ordered, &bare)?.or(image.output_profile);
    let regions = regions(&ordered, &bare, &input, pipe)?;
    if let Some(regions) = &regions {
        let [x, y, width, height] = regions.region;
        let [full_width, full_height] = regions.start.full;
        let line = format!(
            "geometry: from {full_width}×{full_height}, scale {:.3}, reading {width}×{height} at {x},{y}",
            regions.start.scale
        );
        // An export's tiles at trace level, as their plans are.
        if matches!(pipe, Pipe::Tile { .. }) {
            tracing::trace!(target: "pipe", "{line}");
        } else {
            tracing::debug!(target: "pipe", "{line}");
        }
    }
    let image = &ImageContext {
        output_profile,
        ..bare
    };
    let (whole, lines) = build(&ordered, image, input, pipe, limits, None, regions.as_ref())?;
    let viewport = match pipe {
        Pipe::Viewport { width, height } => viewport_split(&whole, limits).map(|(split, full)| {
            let fitted = match &regions {
                Some(regions) => (regions.start.scaled[0], regions.start.scaled[1]),
                None => fit((full.width, full.height), (width, height)),
            };
            (split, full, fitted)
        }),
        Pipe::Export | Pipe::Resampled(_) | Pipe::Tile { .. } => None,
    };
    let Some((split, full, (fitted_width, fitted_height))) = viewport else {
        log(&whole, &lines);
        log_full_resolution(&whole, limits);
        return Ok(whole);
    };
    // The largest tiles, halving from 8192 pixels of the image, whose plan the GPU can
    // run and whose own buffers stay within the budget; the smallest tried otherwise,
    // which the pipeline then refuses with the binding that does not fit.
    let source = [full.width, full.height];
    let output = [fitted_width, fitted_height];
    // The viewport's own size, for its errors: with geometry, the frame, not the
    // developed image at its scale (decision 16).
    let [frame_width, frame_height] = regions
        .as_ref()
        .map_or(output, |regions| [regions.output[2], regions.output[3]]);
    // The viewport image is bound whole, by every tile and every later operation: no
    // tile size helps when it does not fit, a smaller viewport does.
    let viewport = ImageDesc {
        width: fitted_width,
        height: fitted_height,
        ..full
    };
    let limit = limits.binding.min(limits.buffer);
    if viewport.buffer_size() > limit {
        return Err(PlanError::Viewport {
            operation: DOWNSCALE.name,
            width: frame_width,
            height: frame_height,
            bytes: viewport.buffer_size(),
            limit,
            smaller_fits: true,
        });
    }
    let mut edge = 8192;
    loop {
        let tiling = Tiling {
            split,
            full,
            output,
            tiles: viewport_tiles(source, output, edge),
        };
        let (plan, lines) = build(
            &ordered,
            image,
            input,
            pipe,
            limits,
            Some(&tiling),
            regions.as_ref(),
        )?;
        let tiled = plan.tiled.clone().expect("a tiled viewport");
        // What does not fit outside the tiles no tile size helps. After them, the
        // viewport image fits, as checked above, so it is a table or a scratch buffer
        // of an operation at the viewport's size; before them, in the raw stage, the
        // pipeline refuses the plan, naming the pass.
        let outside = plan.oversized_outside_tiles(limits);
        if let Some((op, bytes, limit)) = outside
            && op >= tiled.end
        {
            return Err(PlanError::Viewport {
                operation: plan.operations[op].id,
                width: frame_width,
                height: frame_height,
                bytes,
                limit,
                smaller_fits: false,
            });
        }
        if outside.is_some()
            || edge <= SMALLEST_TILE
            || (plan.binds_within(limits) && plan.tile_bytes() <= limits.budget)
        {
            log(&plan, &lines);
            if plan.binds_within(limits) {
                let largest = |axis: usize| {
                    tiling
                        .tiles
                        .iter()
                        .map(|tile| tile.source[axis + 2])
                        .max()
                        .unwrap_or(0)
                };
                tracing::debug!(
                    target: "pipe",
                    "viewport: demosaic in {} tiles, up to {}×{}",
                    tiling.tiles.len(),
                    largest(0),
                    largest(1)
                );
            }
            return Ok(plan);
        }
        edge /= 2;
    }
}

/// Logs, under `-d memory`, a viewport whose buffers come to more than the budget with
/// the whole-image scratch its operations keep at full resolution (decision 15): the
/// viewport's tiling is decided by demosaic's full-resolution image alone, so that
/// scratch, 40 bytes a pixel of the full resolution for haze removal, is held beside it,
/// counted in the plan's bytes, as an export holds its rounds' (decision 15).
fn log_full_resolution(plan: &Plan, limits: &Limits) {
    let full: Vec<&PlannedOperation> = plan
        .operations
        .iter()
        .filter(|op| op.full_resolution.is_some())
        .collect();
    let held: u64 = full
        .iter()
        .flat_map(|op| &op.scratch)
        .map(|&index| plan.scratch[index])
        .sum();
    if held > 0 && plan.bytes() > limits.budget {
        let ids: Vec<&str> = full.iter().map(|op| op.id).collect();
        tracing::debug!(
            target: "memory",
            "viewport: {:.0} MiB of buffers, {:.0} MiB of them whole-image scratch at full resolution for {}, over the {:.0} MiB budget",
            mebibytes(plan.bytes()),
            mebibytes(held),
            ids.join(", "),
            mebibytes(limits.budget)
        );
    }
}

/// The pipe's output profile (decision 26): the last answer of the instances in
/// pipeline order, each asked with its parameters, colorout from its `type`; `None`, the
/// working profile, when none answers, as in a plan without colorout.
fn output_profile(
    ordered: &[&Instance],
    image: &ImageContext,
) -> Result<Option<OutputProfile>, PlanError> {
    let mut profile = None;
    for instance in ordered {
        let params = resolve(instance, image)?;
        profile = instance.operation.output_profile(&params).or(profile);
    }
    Ok(profile)
}

/// The image `instances` make of the photo `image` describes, at full resolution: the
/// developed image through each geometry's extent, as [`Plan::extent`] finds it once
/// planned (decision 16), with no plan.
pub fn extent(instances: &[Instance], image: &ImageContext) -> Result<[u32; 2], PlanError> {
    let mut extent = [image.width, image.height];
    for instance in instances {
        if instance.operation.manifest().slot.stage == Stage::Raw {
            continue;
        }
        let params = resolve(instance, image)?;
        if let Some(geometry) = instance.operation.geometry(&params, image) {
            extent = geometry.extent(extent);
        }
    }
    Ok(extent)
}

/// An instance's parameters over its defaults for the image. The defaults never see the
/// pipe's output profile, which is `prepare`'s alone (decision 26), so they are the
/// same when the planner asks for the profile and when it prepares the instance.
fn resolve(instance: &Instance, image: &ImageContext) -> Result<ParamValues, PlanError> {
    let operation = instance.operation;
    let image = ImageContext {
        output_profile: None,
        ..image.clone()
    };
    operation
        .schema()
        .resolve_over(&operation.defaults(&image), &instance.params)
        .map_err(|source| PlanError::Param {
            operation: operation.manifest().id,
            source,
        })
}

/// Logs a plan: what it renders, at debug level, or at trace for an export's tile, whose
/// tiles the export sums up; then `lines`, each operation's, at trace.
fn log(plan: &Plan, lines: &[String]) {
    let pipe = match plan.pipe {
        Pipe::Export => "export".to_owned(),
        Pipe::Resampled(Resample {
            output: [width, height],
            ..
        }) => format!("export at {width}×{height}"),
        Pipe::Viewport { .. } => "viewport".to_owned(),
        Pipe::Tile {
            region: [x, y, width, height],
            ..
        } => format!("tile {width}×{height} at {x},{y}"),
    };
    // A viewport's tiles run their operations once a tile.
    let mut ids: Vec<&str> = plan.operations.iter().map(|op| op.id).collect();
    ids.sort_unstable();
    ids.dedup();
    let passes = plan.operations.iter().map(|op| op.passes.len()).sum();
    let (_, output) = plan.output();
    let summary = format!(
        "plan: {pipe}, {}, {}, {output}",
        count(ids.len(), "op", "ops"),
        count(passes, "pass", "passes")
    );
    if matches!(plan.pipe, Pipe::Tile { .. }) {
        tracing::trace!(target: "pipe", "{summary}");
    } else {
        tracing::debug!(target: "pipe", "{summary}");
    }
    for line in lines {
        tracing::trace!(target: "pipe", "{line}");
    }
}

/// The characters a log line's message takes after its time and domain, so the line
/// fits `apertrust_base::log`'s 100 columns.
const MESSAGE: usize = 80;

/// `items` after `head`, comma-separated, in as many lines of at most [`MESSAGE`]
/// characters as they need, each starting with `head`.
fn pack(head: &str, items: &[String]) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for item in items {
        if !line.is_empty() && line.chars().count() + 2 + item.chars().count() > MESSAGE {
            lines.push(std::mem::take(&mut line));
        }
        if line.is_empty() {
            line = format!("{head} {item}");
        } else {
            line.push_str(", ");
            line.push_str(item);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// The smallest square tiles, in pixels of the image at full resolution, a viewport or
/// an export tries before it gives up.
pub(crate) const SMALLEST_TILE: u32 = 64;

/// Where the operations that need the whole image end: after the last one with a
/// whole-image pass that cannot take a first round of an export's tiles, every one in
/// the raw stage among them (highlight reconstruction's), and after the 16-bit mosaic,
/// which a tile cannot copy from (decisions 13 and 14). Whole-image passes after
/// demosaic that can take one, the tone equalizer's, do instead (decision 15; [`runs`]).
pub(crate) fn split(operations: &[PlannedOperation]) -> usize {
    let whole_image = operations
        .iter()
        .rposition(|op| {
            has_whole_image_passes(op)
                && (op.output_desc.format.is_mosaic() || !takes_a_first_round(op))
        })
        .map_or(0, |index| index + 1);
    let unpacked = operations
        .iter()
        .position(|op| op.output_desc.format != Format::Mosaic16)
        .map_or(operations.len(), |index| index + 1);
    whole_image.max(unpacked).min(operations.len())
}

fn has_whole_image_passes(op: &PlannedOperation) -> bool {
    op.passes
        .iter()
        .any(|pass| pass.kind == PassKind::Reduction)
}

/// Whether an operation's whole-image passes can take a first round of an export's
/// tiles (decision 15): it says its passes that read the input write only what each
/// tile owns ([`Operation::tiles_whole_image_passes`]), and they are one run, those
/// passes first, then those that read only scratch, then the rest, so every tile runs
/// the first, the last tile the second, and the last round the rest. Otherwise it runs
/// over the whole image, with every operation before it.
fn takes_a_first_round(op: &PlannedOperation) -> bool {
    let order = |pass: &Pass| match pass.kind {
        PassKind::Reduction if reads_the_whole_image(pass) => 0,
        PassKind::Reduction => 1,
        _ => 2,
    };
    op.tiles_whole_image_passes
        && op.passes.first().is_some_and(reads_the_whole_image)
        && op
            .passes
            .windows(2)
            .all(|pair| order(&pair[0]) <= order(&pair[1]))
}

/// The operations from `split` on with whole-image passes, which an export's tiles meet
/// after demosaic: each takes a first round of tiles of its own before the last round
/// (decision 15). The export holds each one's scratch by its ID, so each must be the
/// only instance of its operation in the tiles.
pub(crate) fn runs(operations: &[PlannedOperation], split: usize) -> Vec<&'static str> {
    let tiles = &operations[split..];
    let runs: Vec<&'static str> = tiles
        .iter()
        .filter(|op| has_whole_image_passes(op))
        .map(|op| op.id)
        .collect();
    for id in &runs {
        let instances = tiles.iter().filter(|op| op.id == *id).count();
        assert_eq!(
            instances, 1,
            "{id}: {instances} instances in an export's tiles, but its first round holds \
             its scratch by operation ID; several instances would need it held \
             by instance"
        );
    }
    runs
}

/// Whether a pass is one of its operation's whole-image passes that read its input, the
/// part of a run each tile of a first round runs.
pub(crate) fn reads_the_whole_image(pass: &Pass) -> bool {
    pass.kind == PassKind::Reduction
        && pass
            .bindings
            .iter()
            .any(|binding| binding.resource == Resource::Input)
}

/// Whether a viewport needs tiles (decision 14): when its full-resolution part needs a
/// binding or a buffer the GPU lacks, or demosaic's full-resolution image, its first
/// cache point, would take more than the budget, decision 12's. Then where the tiles
/// begin, and the full-resolution image they make between them.
fn viewport_split(whole: &Plan, limits: &Limits) -> Option<(usize, ImageDesc)> {
    let &last = whole.cached.first()?;
    let full = whole.operations[last].output_desc;
    if full.format != Format::Rgba
        || (whole.binds_within(limits) && full.buffer_size() <= limits.budget)
    {
        return None;
    }
    // Only the raw stage's reductions matter: one after demosaic, at the viewport's
    // size, runs after the tiles.
    let split = split(&whole.operations[..=last]);
    (split <= last).then_some((split, full))
}

/// A viewport's tiles (decision 14): the raw-stage instances from `split` on run once per
/// tile, each tile then downscaled into the viewport image.
#[derive(Debug, Clone)]
struct Tiling {
    /// The first instance, in pipeline order, the tiles run.
    split: usize,
    /// The image the raw stage ends with at full resolution, as the untiled viewport's
    /// demosaic makes it.
    full: ImageDesc,
    /// The viewport image's width and height.
    output: [u32; 2],
    tiles: Vec<ViewportTile>,
}

/// One tile of the viewport: its part of the developed image at full resolution, and of
/// the viewport image, each x, y, width and height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ViewportTile {
    source: [u32; 4],
    output: [u32; 4],
}

/// The viewport image cut into tiles, rows by columns, each about `edge` pixels of the
/// image across and down, split evenly in the viewport. Output pixel `o` of `output`
/// pixels covers source pixels from `o × source / output` to `(o + 1) × source / output`,
/// as the downscale computes them, so the tile of output pixels from `a` to `b` covers
/// exactly the source pixels from `a × source / output` to `b × source / output`, and
/// the tiles' parts of the source cut it without overlapping.
fn viewport_tiles(source: [u32; 2], output: [u32; 2], edge: u32) -> Vec<ViewportTile> {
    let bounds = |axis: usize| -> Vec<[u32; 2]> {
        let (whole, out) = (u64::from(source[axis]), u64::from(output[axis]));
        let tiles = whole.div_ceil(u64::from(edge.max(1))).clamp(1, out.max(1));
        (0..=tiles)
            .map(|k| {
                let o = k * out / tiles;
                [o as u32, (o * whole / out.max(1)) as u32]
            })
            .collect()
    };
    let (columns, rows) = (bounds(0), bounds(1));
    let span = |a: [u32; 2], b: [u32; 2], i: usize| [a[i], b[i] - a[i]];
    rows.windows(2)
        .flat_map(|y| {
            columns.windows(2).map(move |x| {
                let ([sx, sw], [sy, sh]) = (span(x[0], x[1], 1), span(y[0], y[1], 1));
                let ([ox, ow], [oy, oh]) = (span(x[0], x[1], 0), span(y[0], y[1], 0));
                ViewportTile {
                    source: [sx, sy, sw, sh],
                    output: [ox, oy, ow, oh],
                }
            })
        })
        .collect()
}

/// Where an edit with geometry takes the image (decision 16): which part of its stage
/// each operation after the raw stage makes, at the pipe's scale.
#[derive(Debug, Clone)]
struct Regions {
    /// The stage the operations after the raw stage start from: the developed image, or
    /// for an RGBA input, a tile's after a split, the input's stage.
    start: Extent,
    /// The region of it the first of them reads: what demosaic makes, a tile copies, or
    /// the viewport's downscale is cut to.
    region: [u32; 4],
    /// The pipe's output region at its scale: the whole output, a tile, or the
    /// viewport's frame.
    output: [u32; 4],
    /// Per instance in pipeline order, its place; `None` in the raw stage.
    staged: Vec<Option<Staged>>,
    /// In an export at a size, its resample in the stage its place gives, walked back
    /// through the geometries after it (decision 56).
    at_size: Option<AtSize>,
}

/// An operation's place after the raw stage, with geometry (decision 16).
#[derive(Debug, Clone, Copy)]
struct Staged {
    /// The stage it writes, at full resolution.
    extent: [u32; 2],
    /// The stage it writes, at the pipe's scale (decision 15).
    scaled: [u32; 2],
    /// Its output region at the pipe's scale.
    region: [u32; 4],
    /// The region of its input stage it reads: `region`, but for a geometry's.
    input: [u32; 4],
    geometry: bool,
    /// Whether the pipe's whole output, untiled, needs all of its input stage at the
    /// pipe's scale, which its whole-image passes then cover (decision 16): not before a
    /// geometry that keeps part of it, as crop's window.
    whole: bool,
}

/// Works out [`Regions`] for the ordered instances (decision 16), as darktable's pixel
/// pipe does: each stage's extent forward from the developed image at full resolution
/// through each geometry's `extent` (`dt_dev_pixelpipe_get_dimensions`), the pipe's
/// scale from the output's, the viewport's fitted to it, and each operation's output
/// region backward from the pipe's output region through each geometry's
/// `input_region` (`modify_roi_in`); every other operation reads the region it makes,
/// since none after the raw stage reaches past it yet. The whole output, untiled, is
/// worked back the same way, so each operation knows whether it needs all of its stage,
/// which whole-image passes cover. Every stage's size at the scale is its extent times
/// the fitted scale, rounded as `fit` rounds the output, so a quarter turn swaps the
/// sides exactly. `None` when no instance moves pixels: such a plan is planned as
/// before geometry, its every operation making the whole image or the pipe's region.
fn regions(
    ordered: &[&Instance],
    image: &ImageContext,
    input: &ImageDesc,
    pipe: Pipe,
) -> Result<Option<Regions>, PlanError> {
    let mut geometries = Vec::with_capacity(ordered.len());
    for instance in ordered {
        let operation = instance.operation;
        let manifest = operation.manifest();
        let params = operation
            .schema()
            .resolve_over(&operation.defaults(image), &instance.params)
            .map_err(|source| PlanError::Param {
                operation: manifest.id,
                source,
            })?;
        let geometry = operation.geometry(&params, image);
        if geometry.is_some() && manifest.slot.stage == Stage::Raw {
            return Err(PlanError::Geometry {
                operation: manifest.id,
                what: "it moves pixels in the raw stage",
            });
        }
        geometries.push(geometry);
    }
    if geometries.iter().all(Option::is_none) {
        return Ok(None);
    }
    // An export at a size: its resample and the pipe's output region at the export's
    // scale, in the output image (decision 56). It resamples on either side of a geometry
    // of whole pixels, a quarter turn or a mirror, in the stage its place gives, as
    // darktable's does; one that interpolates or crops is not planned with it yet.
    let at_size = match pipe {
        Pipe::Resampled(resample) => {
            Some((resample, [0, 0, resample.output[0], resample.output[1]]))
        }
        Pipe::Tile {
            region,
            resample: Some(resample),
        } => Some((resample, region)),
        _ => None,
    };
    if at_size.is_some()
        && let Some((instance, _)) = ordered
            .iter()
            .zip(&geometries)
            .find(|(_, g)| g.as_ref().is_some_and(|g| !g.whole_pixels()))
    {
        return Err(PlanError::Geometry {
            operation: instance.operation.manifest().id,
            what: "an export at a size with an operation that interpolates or crops is not \
                   planned yet",
        });
    }
    let start = match input.format {
        // An RGBA image made outside a plan from another's description, a mosaic's.
        Format::Rgba if input.extent.contains(&0) => {
            return Err(PlanError::Geometry {
                operation: "the input",
                what: "an RGBA image whose extent is not set",
            });
        }
        Format::Rgba => input.extent,
        Format::Mosaic | Format::Mosaic16 => [image.width, image.height],
    };
    let mut extent = start;
    // Each instance's input and output stage, at full resolution.
    let mut extents = Vec::with_capacity(ordered.len());
    for (instance, geometry) in ordered.iter().zip(&geometries) {
        if instance.operation.manifest().slot.stage == Stage::Raw {
            extents.push(None);
            continue;
        }
        let input = extent;
        if let Some(geometry) = geometry {
            extent = geometry.extent(input);
            if extent.contains(&0) {
                return Err(PlanError::Geometry {
                    operation: instance.operation.manifest().id,
                    what: "it leaves an empty image",
                });
            }
        }
        extents.push(Some((input, extent)));
    }
    let (fitted, scale, output) = match pipe {
        Pipe::Export => (1.0, 1.0, [0, 0, extent[0], extent[1]]),
        Pipe::Resampled(resample) => (
            1.0,
            resample.scale,
            [0, 0, resample.output[0], resample.output[1]],
        ),
        Pipe::Tile { region, resample } => (1.0, resample.map_or(1.0, |r| r.scale), region),
        Pipe::Viewport { width, height } => {
            let fitted = fit_scale((extent[0], extent[1]), (width, height));
            let [width, height] = at_scale(extent, fitted);
            (
                fitted,
                width as f32 / extent[0] as f32,
                [0, 0, width, height],
            )
        }
    };
    // Before an export's resample, or with none, a stage at the pipe's scale; after it,
    // at the export's scale, its size the output's taken back through the geometries
    // after the resample, each of whole pixels, its extent its own inverse.
    let at = |full: [u32; 2]| Extent {
        full,
        scaled: at_scale(full, fitted),
        scale: if at_size.is_some() { 1.0 } else { scale },
    };
    let place = match at_size {
        Some((resample, _)) => resample_place(ordered, image, resample.finalscale)?,
        None => 0,
    };
    // A tile whose RGBA input is past its export's resample: all its operations after it.
    let past = at_size.is_some()
        && matches!(pipe, Pipe::Tile { .. })
        && input.format == Format::Rgba
        && place == 0;
    let mut stage_scaled = at_size.map_or(at(extent).scaled, |(resample, _)| resample.output);
    let mut stage_full = extent;
    if !within(output, stage_scaled) {
        return Err(PlanError::Geometry {
            operation: "a tile",
            what: "its region lies outside the output image",
        });
    }
    let mut wanted = output;
    // What the whole output, untiled, needs of each stage: `wanted`, but in a tile.
    let mut needed = [0, 0, stage_scaled[0], stage_scaled[1]];
    let mut staged = vec![None; ordered.len()];
    // The resample once the walk has crossed its place: the region it makes at the
    // export's scale in its own stage, and its plans from that stage at full resolution.
    let mut resampled: Option<AtSize> = None;
    let cross = |wanted: &mut [u32; 4],
                 needed: &mut [u32; 4],
                 stage_scaled: [u32; 2],
                 stage_full: [u32; 2]|
     -> Option<AtSize> {
        let (resample, _) = at_size?;
        let resample = Resample {
            output: stage_scaled,
            ..resample
        };
        let plans = (!past && resample.scale != 1.0).then(|| {
            [0, 1].map(|axis| {
                let output = stage_scaled[axis];
                let length = input_length(
                    output,
                    resample.scale,
                    stage_full[axis],
                    resample.finalscale,
                )
                .min(stage_full[axis]);
                AxisPlan::new(length, output, resample.scale)
            })
        });
        let at_size = AtSize {
            resample,
            region: *wanted,
            pending: !past,
            place,
            plans,
        };
        if !past {
            *wanted = at_size.input_region();
            *needed = [0, 0, stage_full[0], stage_full[1]];
        }
        Some(at_size)
    };
    for (index, geometry) in geometries.iter().enumerate().rev() {
        if resampled.is_none() && !past && index < place {
            resampled = cross(&mut wanted, &mut needed, stage_scaled, stage_full);
        }
        let Some((input, extent)) = extents[index] else {
            continue;
        };
        let after = at_size.is_some() && resampled.is_none();
        let input = if after {
            Extent {
                full: input,
                scaled: geometry
                    .as_ref()
                    .map_or(stage_scaled, |g| g.extent(stage_scaled)),
                scale,
            }
        } else {
            at(input)
        };
        let scaled = if after {
            stage_scaled
        } else {
            at(extent).scaled
        };
        let (read, need) = match geometry {
            Some(geometry) => {
                let read = geometry.input_region(&input, wanted);
                if !within(read, input.scaled) || read[2] == 0 || read[3] == 0 {
                    return Err(PlanError::Geometry {
                        operation: ordered[index].operation.manifest().id,
                        what: "it reads a region outside its input",
                    });
                }
                (read, geometry.input_region(&input, needed))
            }
            None => (wanted, needed),
        };
        staged[index] = Some(Staged {
            extent,
            scaled,
            region: wanted,
            input: read,
            geometry: geometry.is_some(),
            whole: need == [0, 0, input.scaled[0], input.scaled[1]],
        });
        (wanted, needed) = (read, need);
        (stage_scaled, stage_full) = (input.scaled, input.full);
    }
    if resampled.is_none() {
        resampled = cross(&mut wanted, &mut needed, stage_scaled, stage_full);
    }
    Ok(Some(Regions {
        start: at(start),
        region: wanted,
        output,
        staged,
        at_size: resampled,
    }))
}

/// Whether `region` lies inside an image of `size`.
fn within(region: [u32; 4], size: [u32; 2]) -> bool {
    let [x, y, width, height] = region.map(u64::from);
    x + width <= u64::from(size[0]) && y + height <= u64::from(size[1])
}

/// Plans the ordered instances, the viewport's in tiles if `tiling` says, with geometry
/// where `regions` say, with the lines to log, each with whether it is detail.
fn build(
    ordered: &[&Instance],
    image: &ImageContext,
    input: ImageDesc,
    pipe: Pipe,
    limits: &Limits,
    tiling: Option<&Tiling>,
    regions: Option<&Regions>,
) -> Result<(Plan, Vec<String>), PlanError> {
    // The stage the operations after the raw stage start in, at full resolution.
    let stage = match regions {
        Some(regions) => regions.start.full,
        // A tile's RGBA input after a split past a geometry lies in that geometry's
        // stage, though no operation of the tile's own moves pixels (decision 16).
        None if input.format == Format::Rgba && !input.extent.contains(&0) => input.extent,
        None => [image.width, image.height],
    };
    let at_size = match regions.and_then(|regions| regions.at_size.clone()) {
        Some(at_size) => Some(at_size),
        None => AtSize::of(pipe, ordered, image, &input, stage)?,
    };
    // The part of the developed image at full resolution the raw stage makes: the
    // whole image, or a tile's region, or with geometry the region the operations after
    // it read (decision 16), or at a size the region a tile's resample reads (decision
    // 56). The viewport's demosaic always makes the whole image.
    let region = match (pipe, regions) {
        (Pipe::Viewport { .. }, _) | (Pipe::Export | Pipe::Resampled(_), None) => None,
        (Pipe::Tile { region, .. }, None) => Some(
            at_size
                .as_ref()
                .map_or(region, |at_size| at_size.input_region()),
        ),
        (Pipe::Export | Pipe::Resampled(_) | Pipe::Tile { .. }, Some(regions)) => {
            Some(regions.region)
        }
    };
    let mut builder = Builder {
        plan: Plan {
            pipe,
            image: image.clone(),
            input,
            operations: Vec::new(),
            work: Vec::new(),
            work_used: Vec::new(),
            scratch: Vec::new(),
            scratch_used: Vec::new(),
            cached: Vec::new(),
            edit_cache: None,
            tiled: None,
            resample: None,
        },
        reserved: Vec::new(),
        limits: *limits,
        current: Buffer::Input,
        current_desc: input,
        key: {
            let mut hasher = DefaultHasher::new();
            hash_desc(&input, &mut hasher);
            hasher.finish()
        },
        context: match region {
            Some(region) => ImageContext {
                region,
                ..image.clone()
            },
            None => image.clone(),
        },
        downscaled: false,
        tiling: tiling.cloned(),
        slot: None,
        slots: Slots::default(),
        sizing: false,
        lines: Vec::new(),
        regions: regions.cloned(),
        stage,
        scaled: None,
        full_resolution: None,
        scaled_origin: None,
        at_size,
    };
    // A tile past its export's resample starts at the export's scale.
    if let Some(at_size) = builder.at_size.as_ref().filter(|at| !at.pending) {
        let [x, y, ..] = at_size.region;
        builder.context.scale = at_size.resample.scale;
        builder.scaled = Some(at_size.resample.output);
        builder.scaled_origin = Some([x, y]);
        builder.plan.resample = Some(0..0);
    }
    if let (Pipe::Tile { .. }, Some(region)) = (pipe, region) {
        // The tile's operations need the region grown by their radii.
        let border: u32 = ordered
            .iter()
            .map(|instance| instance.operation.manifest().radius)
            .sum();
        builder.tile_input(region, border)?;
    }
    // The raw-stage instances the viewport's tiles run, from the tiling's split on.
    let mut tiled = Vec::new();
    let mut started = false;
    for (index, &instance) in ordered.iter().enumerate() {
        let stage = instance.operation.manifest().slot.stage;
        if stage != Stage::Raw {
            builder.viewport(&tiled)?;
            if !started {
                started = true;
                builder.start()?;
            }
        } else if tiling.is_some_and(|tiling| index >= tiling.split) {
            tiled.push(instance);
            continue;
        }
        builder.resample(index)?;
        let staged = regions.and_then(|regions| regions.staged[index]);
        builder.add(instance, staged)?;
    }
    if builder.current_desc.format == Format::Rgba || !tiled.is_empty() {
        builder.viewport(&tiled)?;
    }
    builder.resample(ordered.len())?;
    Ok((builder.plan, builder.lines))
}

/// Where among `ordered` an export at a size resamples (decisions 15 and 56): the index
/// of the first instance after it. With high quality resampling, or above a scale of 1, at
/// finalscale's place, every operation before it at full resolution. Without, after the
/// last instance whose whole-image results are computed at full resolution
/// ([`Operation::whole_image_at_full_resolution`], haze removal's, decision 15), so that
/// it runs as in the full-size export, and after the raw stage while none is planned.
fn resample_place(
    ordered: &[&Instance],
    image: &ImageContext,
    finalscale: bool,
) -> Result<usize, PlanError> {
    let slot = |instance: &&Instance| instance.operation.manifest().slot;
    let first = |past: &dyn Fn(Slot) -> bool| {
        ordered
            .iter()
            .position(|instance| past(slot(instance)))
            .unwrap_or(ordered.len())
    };
    if finalscale {
        return Ok(first(&|slot| slot.pipeline_order(&FINALSCALE).is_gt()));
    }
    let mut place = first(&|slot| slot.stage != Stage::Raw);
    for (index, instance) in ordered.iter().enumerate() {
        let params = resolve(instance, image)?;
        if instance.operation.whole_image_at_full_resolution(&params) {
            place = place.max(index + 1);
        }
    }
    Ok(place)
}

/// An export at a size, as a plan of it meets it (decision 56).
#[derive(Debug, Clone)]
struct AtSize {
    resample: Resample,
    /// The plan's output region at the export's scale: the whole output, or a tile.
    region: [u32; 4],
    /// Whether the plan has the resample still to run: not once it has, nor where the
    /// plan's input is past it.
    pending: bool,
    /// Where it resamples: before the instance of the plan at this index, or at the end.
    place: usize,
    /// The resample's plans across and down, the whole output's; `None` at a scale of 1,
    /// where darktable copies the output's part of the image (`copymode`).
    plans: Option<[AxisPlan; 2]>,
}

impl AtSize {
    /// What `pipe` asks of a plan of `ordered` from `input` in a stage of `stage` at full
    /// resolution: a whole export at a size always resamples; a tile of one does unless
    /// its input is an RGBA image past the resample's place, one whose operations all
    /// come after it ([`resample_place`]).
    fn of(
        pipe: Pipe,
        ordered: &[&Instance],
        image: &ImageContext,
        input: &ImageDesc,
        stage: [u32; 2],
    ) -> Result<Option<Self>, PlanError> {
        let (resample, region) = match pipe {
            Pipe::Resampled(resample) => (resample, [0, 0, resample.output[0], resample.output[1]]),
            Pipe::Tile {
                region,
                resample: Some(resample),
            } => (resample, region),
            Pipe::Export | Pipe::Viewport { .. } | Pipe::Tile { resample: None, .. } => {
                return Ok(None);
            }
        };
        let place = resample_place(ordered, image, resample.finalscale)?;
        let past = matches!(pipe, Pipe::Tile { .. }) && input.format == Format::Rgba && place == 0;
        let plans = (!past && resample.scale != 1.0).then(|| {
            [0, 1].map(|axis| {
                let output = resample.output[axis];
                let length = input_length(output, resample.scale, stage[axis], resample.finalscale)
                    .min(stage[axis]);
                AxisPlan::new(length, output, resample.scale)
            })
        });
        Ok(Some(Self {
            resample,
            region,
            pending: !past,
            place,
            plans,
        }))
    }

    /// The region at full resolution the plan's resample reads, its output region walked
    /// back through its plans; the output region itself where the plan's input is past
    /// it, or at a scale of 1.
    fn input_region(&self) -> [u32; 4] {
        let Some([across, down]) = self.plans.as_ref().filter(|_| self.pending) else {
            return self.region;
        };
        let [x, y, width, height] = self.region;
        let columns = across.input_span(x..x + width);
        let rows = down.input_span(y..y + height);
        [
            columns.start,
            rows.start,
            columns.len() as u32,
            rows.len() as u32,
        ]
    }
}

/// A plan as it is built: where the image is so far and what produced it.
struct Builder {
    plan: Plan,
    /// Working buffers no later operation may write: the cache points'.
    reserved: Vec<bool>,
    limits: Limits,
    current: Buffer,
    current_desc: ImageDesc,
    /// The key of what `current` holds.
    key: u64,
    /// The image context the next operation sees, at its scale.
    context: ImageContext,
    downscaled: bool,
    /// The viewport's tiles, when it has them.
    tiling: Option<Tiling>,
    /// Inside a viewport tile, the place of the next operation in it, whose buffers it
    /// shares with the same operation in every other tile.
    slot: Option<usize>,
    slots: Slots,
    /// While a tile sizes the tiles' buffers, its operations are dropped afterwards.
    sizing: bool,
    /// Each operation's lines, for trace level: only the plan `plan` returns logs them.
    lines: Vec<String>,
    /// Where geometry takes the image, when an operation moves pixels (decision 16).
    regions: Option<Regions>,
    /// The stage the next operation writes, at full resolution: the developed image, an
    /// RGBA input's own, or the last geometry's output.
    stage: [u32; 2],
    /// That stage at the pipe's scale once the pipe has left full resolution (decision
    /// 15): after the viewport's downscale its fitted size, with geometry each stage's
    /// at the viewport's scale, and after an export's resample its output. `None` before
    /// them, where it is `stage`.
    scaled: Option<[u32; 2]>,
    /// In an untiled viewport without geometry, the full-resolution image its downscale
    /// read, for an operation that computes its whole-image results at full resolution
    /// (decision 15).
    full_resolution: Option<FullImage>,
    /// In an export at a size, after its resample, the corner of the plan's output region
    /// at the export's scale, where every later operation's output lies (decision 56).
    scaled_origin: Option<[u32; 2]>,
    /// The export's size and its resample, in an export at a size.
    at_size: Option<AtSize>,
}

/// The full-resolution image the viewport's downscale read (decision 15): the buffer
/// that holds it, a cache point or the input, what it is, and its key.
#[derive(Debug, Clone, Copy)]
struct FullImage {
    buffer: Buffer,
    desc: ImageDesc,
    key: u64,
}

/// The buffers the operations of a viewport's tiles write, by their place in a tile:
/// each working buffer as large as the largest tile's image, each scratch buffer as the
/// largest tile's.
#[derive(Default)]
struct Slots {
    outputs: Vec<usize>,
    scratch: Vec<Vec<usize>>,
}

/// Where an operation's output lies, for the pass header's origin.
#[derive(Clone, Copy)]
enum Place {
    /// On the sensor, as the raw stage's images place themselves: the origin is 0.
    Sensor,
    /// In the developed image, at the pipe's region's corner and scale.
    Region,
    /// At this corner of the image at the pipe's scale.
    At([u32; 2]),
}

impl Builder {
    /// Prepares an instance for the current image, and adds it; with geometry, at the
    /// place `staged` gives it (decision 16).
    fn add(&mut self, instance: &Instance, staged: Option<Staged>) -> Result<(), PlanError> {
        let operation = instance.operation;
        let manifest = operation.manifest();
        let id = manifest.id;
        // The raw stage reads mosaics; every later stage reads RGBA.
        if self.current_desc.format.is_mosaic() != (manifest.slot.stage == Stage::Raw) {
            return Err(PlanError::Format {
                operation: id,
                stage: manifest.slot.stage,
                format: self.current_desc.format,
            });
        }
        let params = resolve(instance, &self.context)?;
        let head = format!("{id} params:");
        let lines = pack(&head, &describe(operation.schema(), &params));
        self.lines.extend(lines);
        let full_resolution = self.full_resolution_for(id, operation, &params);
        let context = ImageContext {
            full_resolution: full_resolution.map(|full| full.desc),
            ..self.context.clone()
        };
        let mut prepared = operation.prepare(&params, &context, &self.current_desc)?;
        // An operation that moves no pixel and runs no pass, keeping its input as it is,
        // has nothing to do with these parameters: it leaves the plan, as darktable's
        // `commit_params` disables its piece (orientation at no rotation, decision 16).
        if staged.is_none_or(|staged| !staged.geometry)
            && manifest.slot.stage != Stage::Raw
            && prepared.passes.is_empty()
            && prepared.scratch.is_empty()
            && prepared.output == self.current_desc
        {
            tracing::debug!(target: "pipe", "{id}: nothing to do, left out of the plan");
            return Ok(());
        }
        for pass in &prepared.passes {
            // Spatial passes cover the whole image until regions arrive (step 5), as
            // reductions always do: their partial results and the statistic stay in
            // the operation's scratch buffers for its later passes (decision 15).
            for binding in &pass.bindings {
                let valid = match binding.resource {
                    Resource::Table(index) => index < pass.tables.len(),
                    Resource::Scratch(index) => index < prepared.scratch.len(),
                    Resource::FullResolution => full_resolution.is_some(),
                    Resource::Input | Resource::Output | Resource::Uniforms => true,
                };
                if !valid {
                    return Err(PlanError::Binding {
                        operation: id,
                        resource: binding.resource,
                    });
                }
            }
        }
        let place = match staged {
            _ if manifest.slot.stage == Stage::Raw => Place::Sensor,
            Some(staged) => {
                self.place(id, staged, &mut prepared)?;
                Place::At([staged.region[0], staged.region[1]])
            }
            None => Place::Region,
        };
        // Demosaic's image lies at the corner of the region the raw stage makes, every
        // later one where its header's origin says (decision 16).
        if prepared.output.format == Format::Rgba && staged.is_none() {
            let [x, y, ..] = self.context.region;
            prepared.output.origin = match place {
                Place::Sensor => [x, y],
                Place::Region | Place::At(_) => self.origin(place),
            };
        }
        // A geometry that is a translation of its input region runs no pass: its output
        // is its input's buffer, the origin moved.
        let in_place = staged.is_some_and(|staged| staged.geometry) && prepared.passes.is_empty();
        // The edit cache may keep this input: it is sized for the largest image after
        // its start, which after a geometry need not be the first (decision 16).
        if let Some(edit) = &mut self.plan.edit_cache
            && self.current_desc.buffer_size() > edit.buffer_size()
        {
            *edit = self.current_desc;
        }
        let scratch: Vec<u64> = prepared.scratch.iter().map(|s| s.bytes).collect();
        self.push(
            id,
            manifest.version,
            operation.kernel(),
            &manifest.slot.to_string(),
            place,
            prepared.output,
            prepared.passes,
            scratch.clone(),
            in_place,
        )?;
        let planned = self.plan.operations.last_mut().expect("just pushed");
        planned.tiles_whole_image_passes = operation.tiles_whole_image_passes(&params);
        planned.full_resolution = full_resolution.map(|full| full.buffer);
        if let (Some(full), Some(whole)) = (full_resolution, &mut planned.whole_image) {
            whole.full_resolution = full_resolution_part(
                full.key,
                id,
                manifest.version,
                &planned.passes[..whole.passes],
                &scratch,
            );
        }
        if let Some(staged) = staged.filter(|staged| staged.geometry) {
            // What its output depends on that its prepared bytes need not hold, a
            // translation having none: the region it reads and the stage it makes.
            let mut hasher = DefaultHasher::new();
            (planned.key, staged.input, staged.extent).hash(&mut hasher);
            planned.key = hasher.finish();
            self.key = planned.key;
        }
        Ok(())
    }

    /// The full-resolution image, and its buffer, that `operation` with `params` may
    /// compute its whole-image results from in the viewport (decision 15): where it
    /// opts in, and comes straight after the untiled viewport's downscale, so that image
    /// is its input at full resolution. Elsewhere it estimates at the pipe's scale, as
    /// every operation does, logged: in a tiled viewport, whose full resolution exists
    /// a tile at a time, or after another operation at the viewport's scale, which the
    /// first operation ported between demosaic and it, denoise, will need to run at full
    /// resolution first.
    fn full_resolution_for(
        &self,
        id: &'static str,
        operation: &dyn Operation,
        params: &ParamValues,
    ) -> Option<FullImage> {
        if !operation.whole_image_at_full_resolution(params)
            || self.context.pipe != PipeKind::Viewport
        {
            return None;
        }
        let after_downscale = self
            .plan
            .operations
            .last()
            .is_some_and(|op| op.id == DOWNSCALE.name);
        let full = self.full_resolution.filter(|_| after_downscale);
        if full.is_none() && self.downscaled {
            tracing::debug!(
                target: "pipe",
                "{id}: whole-image results at the viewport's scale, as its full resolution {}",
                if self.plan.tiled.is_some() {
                    "is in tiles"
                } else {
                    "is not its input"
                }
            );
        }
        full
    }

    /// With geometry, checks that an operation after the raw stage got the region of its
    /// input it reads and made its output region, and places its output there: the
    /// planner works regions backward through geometries alone, so every other
    /// operation must keep the size it is given (decision 16).
    fn place(
        &mut self,
        id: &'static str,
        staged: Staged,
        prepared: &mut Prepared,
    ) -> Result<(), PlanError> {
        // A whole-image pass covers its stage's extent at the pipe's scale, which its
        // operation reads from `ImageDesc::extent` (decision 16). Before a
        // geometry that keeps part of that stage, as crop's window, it would have to
        // cover that part alone, which no operation lays its scratch out as yet.
        if !staged.whole
            && prepared
                .passes
                .iter()
                .any(|pass| pass.kind == PassKind::Reduction)
        {
            return Err(PlanError::Geometry {
                operation: id,
                what: "whole-image passes before a geometry that keeps part of their stage \
                       are not planned yet",
            });
        }
        // A warp's input rows vary from strip to strip, and a quarter turn reads every
        // input row for any output row (decision 16).
        if staged.geometry && prepared.passes.iter().any(|pass| pass.strips.is_some()) {
            return Err(PlanError::Geometry {
                operation: id,
                what: "its passes take no row strips",
            });
        }
        let got = self.current_desc;
        if got.region() != staged.input {
            return Err(PlanError::Region {
                operation: id,
                made: false,
                desc: got,
                region: staged.input,
            });
        }
        let made = prepared.output;
        let [.., width, height] = staged.region;
        if made.format != Format::Rgba || [made.width, made.height] != [width, height] {
            return Err(PlanError::Region {
                operation: id,
                made: true,
                desc: made,
                region: staged.region,
            });
        }
        let translation = [got.width, got.height] == [width, height];
        if staged.geometry
            && prepared.passes.is_empty()
            && !(translation && prepared.scratch.is_empty())
        {
            return Err(PlanError::Geometry {
                operation: id,
                what: "it runs no pass, but does not keep its input region's size",
            });
        }
        prepared.output.origin = [staged.region[0], staged.region[1]];
        self.stage = staged.extent;
        self.scaled = Some(staged.scaled);
        Ok(())
    }

    /// The pass header's origin of an output at `place`.
    fn origin(&self, place: Place) -> [u32; 2] {
        let scale = self.context.scale;
        match place {
            Place::Sensor => [0, 0],
            // After an export's resample, at its output region's corner at its scale.
            Place::Region if self.scaled_origin.is_some() => {
                self.scaled_origin.expect("just checked")
            }
            // Images after the raw stage lie in the developed image, a tile's at its
            // corner.
            Place::Region => {
                let [x, y, ..] = self.context.region;
                [(x as f32 * scale) as u32, (y as f32 * scale) as u32]
            }
            Place::At(origin) => origin,
        }
    }

    /// Adds an operation reading the current image, its output in a working buffer.
    #[allow(
        clippy::too_many_arguments,
        reason = "one operation's whole description"
    )]
    fn push(
        &mut self,
        id: &'static str,
        version: u32,
        kernel: &'static Kernel,
        slot: &str,
        place: Place,
        mut output_desc: ImageDesc,
        passes: Vec<Pass>,
        scratch: Vec<u64>,
        in_place: bool,
    ) -> Result<(), PlanError> {
        if output_desc.width == 0 || output_desc.height == 0 {
            return Err(PlanError::Empty {
                operation: id,
                desc: output_desc,
            });
        }
        // Every RGBA image is a region of its stage's (decision 16), which whole-image
        // passes cover at the pipe's scale (decision 15).
        if output_desc.format == Format::Rgba {
            output_desc.extent = self.stage;
            output_desc.scaled_extent = self.scaled.unwrap_or(self.stage);
        }
        let origin = self.origin(place);
        // A geometry's translation writes nothing: its output is its input's buffer.
        let output = if in_place {
            self.current
        } else {
            Buffer::Work(self.work_buffer(id, &output_desc)?)
        };
        let plan = &mut self.plan;
        let line = format!(
            "{id} at {slot}: {} → {output_desc}, {}{}",
            self.current_desc,
            count(passes.len(), "pass", "passes"),
            if scratch.is_empty() {
                String::new()
            } else {
                format!(", {} scratch", scratch.len())
            }
        );
        self.lines.push(line);
        let scale = self.context.scale;
        let key = key(self.key, id, version, scale, origin, &output_desc, &passes);
        // Kept by key in the viewport, outside its tiles, whose operations share buffers.
        let keeps = matches!(plan.pipe, Pipe::Viewport { .. }) && self.slot.is_none();
        let whole_image = keeps
            .then(|| {
                whole_image(
                    self.key,
                    id,
                    version,
                    scale,
                    &output_desc,
                    &passes,
                    &scratch,
                )
            })
            .flatten();
        let dispatches: Vec<Vec<Dispatch>> = passes
            .iter()
            .map(|pass| dispatches(pass, &self.current_desc, &output_desc, &self.limits))
            .collect();
        let scratch = match self.slot {
            Some(place) => match self.slots.scratch.get(place) {
                Some(indices) => {
                    if indices.len() != scratch.len() {
                        return Err(PlanError::TileShape {
                            operation: id,
                            what: "number of scratch buffers",
                        });
                    }
                    for (&index, bytes) in indices.iter().zip(scratch) {
                        plan.scratch[index] = plan.scratch[index].max(bytes);
                        if !self.sizing {
                            plan.scratch_used[index] = plan.scratch_used[index].max(bytes);
                        }
                    }
                    indices.clone()
                }
                None => {
                    let indices: Vec<usize> = scratch
                        .into_iter()
                        .map(|bytes| {
                            plan.scratch.push(bytes);
                            plan.scratch_used.push(if self.sizing { 0 } else { bytes });
                            plan.scratch.len() - 1
                        })
                        .collect();
                    self.slots.scratch.push(indices.clone());
                    indices
                }
            },
            None => scratch
                .into_iter()
                .map(|bytes| {
                    plan.scratch.push(bytes);
                    plan.scratch_used.push(bytes);
                    plan.scratch.len() - 1
                })
                .collect(),
        };
        plan.operations.push(PlannedOperation {
            id,
            kernel,
            input: self.current,
            output,
            output_desc,
            passes,
            dispatches,
            scratch,
            scale,
            origin,
            extent: self.stage,
            key,
            tiles_whole_image_passes: false,
            whole_image,
            full_resolution: None,
        });
        (self.current, self.current_desc, self.key) = (output, output_desc, key);
        if let Some(place) = &mut self.slot {
            *place += 1;
        }
        Ok(())
    }

    /// The working buffer an operation writes `output_desc` to: in a viewport tile, the
    /// buffer of the operation at its place in every tile; otherwise one that fits and
    /// that nothing still needs, or a new one.
    fn work_buffer(
        &mut self,
        id: &'static str,
        output_desc: &ImageDesc,
    ) -> Result<usize, PlanError> {
        let plan = &mut self.plan;
        Ok(match self.slot {
            // In a viewport tile, the buffer of the operation at this place in every
            // tile, as large as the largest image it holds.
            Some(place) => {
                let index = match self.slots.outputs.get(place) {
                    Some(&index) => index,
                    None => {
                        // Grown below by the first image it holds.
                        let empty = ImageDesc {
                            width: 0,
                            height: 0,
                            ..*output_desc
                        };
                        plan.work.push(empty);
                        plan.work_used.push(empty);
                        self.reserved.push(false);
                        self.slots.outputs.push(plan.work.len() - 1);
                        plan.work.len() - 1
                    }
                };
                // A tile the size its buffers are made for counts towards their size
                // only, not towards what the plan uses.
                let mut grown = vec![&mut plan.work[index]];
                if !self.sizing {
                    grown.push(&mut plan.work_used[index]);
                }
                for held in grown {
                    if held.format != output_desc.format {
                        return Err(PlanError::TileShape {
                            operation: id,
                            what: "output's format",
                        });
                    }
                    if output_desc.buffer_size() > held.buffer_size() {
                        *held = *output_desc;
                    }
                }
                index
            }
            // Never a viewport tile's buffer: its size is what it was made for, not
            // what the plan uses of it.
            None => plan
                .work
                .iter()
                .enumerate()
                .position(|(index, desc)| {
                    desc.fits(output_desc)
                        && self.current != Buffer::Work(index)
                        && !self.reserved[index]
                        && !self.slots.outputs.contains(&index)
                })
                .unwrap_or_else(|| {
                    plan.work.push(*output_desc);
                    plan.work_used.push(*output_desc);
                    self.reserved.push(false);
                    plan.work.len() - 1
                }),
        })
    }

    /// In the viewport pipe, once, where the raw stage ends: makes the image the raw
    /// stage ends with a cache point, downscales it to fit the viewport, makes the
    /// downscale a cache point too, and has every later operation see the viewport's
    /// scale. With tiles, the raw-stage instances in `tiled` run tile by tile first. With
    /// geometry, the developed image is downscaled to the scale the output fits at
    /// (decision 16).
    fn viewport(&mut self, tiled: &[&Instance]) -> Result<(), PlanError> {
        let Pipe::Viewport { width, height } = self.plan.pipe else {
            return Ok(());
        };
        if self.downscaled {
            return Ok(());
        }
        self.downscaled = true;
        self.cache_current();
        if let Some(tiling) = self.tiling.take() {
            return self.viewport_tiles(&tiling, tiled);
        }
        let full = self.current_desc;
        let ([fitted_width, fitted_height], scale) = match &self.regions {
            Some(regions) => (regions.start.scaled, regions.start.scale),
            None => {
                let (fitted_width, fitted_height) = fit((full.width, full.height), (width, height));
                (
                    [fitted_width, fitted_height],
                    fitted_width as f32 / full.width as f32,
                )
            }
        };
        if (fitted_width, fitted_height) != (full.width, full.height) {
            self.context.scale = scale;
            self.scaled = Some([fitted_width, fitted_height]);
            if self.regions.is_none() {
                self.full_resolution = Some(FullImage {
                    buffer: self.current,
                    desc: full,
                    key: self.key,
                });
            }
            let source = [full.width, full.height];
            let params =
                downscale_params(source, [fitted_width, fitted_height], [0, 0, full.width])?;
            let output = ImageDesc {
                width: fitted_width,
                height: fitted_height,
                origin: [0, 0],
                ..full
            };
            self.push(
                DOWNSCALE.name,
                1,
                &DOWNSCALE,
                "viewport scale",
                Place::Region,
                output,
                vec![Pass::per_pixel(params, Vec::new())],
                Vec::new(),
                false,
            )?;
            self.cache_current();
        }
        self.plan.edit_cache = Some(self.current_desc);
        Ok(())
    }

    /// In an export at a size, its resample (decision 56), when the plan still has it to
    /// run and `next`, the index of the next instance, or at the end their number, is at
    /// or past its place ([`resample_place`]), recording the operations it adds as
    /// [`Plan::resample`].
    fn resample(&mut self, next: usize) -> Result<(), PlanError> {
        if !self
            .at_size
            .as_ref()
            .is_some_and(|at_size| at_size.pending && next >= at_size.place)
        {
            return Ok(());
        }
        let start = self.plan.operations.len();
        self.resample_here()?;
        self.plan.resample = Some(start..self.plan.operations.len());
        Ok(())
    }

    /// The export's resample, here: it reads the current image, the region at full
    /// resolution its plans walk the output region back to, which the operations before
    /// it made, and makes the output region at the export's scale, which every later
    /// operation then runs at. Its two passes, across into scratch and then down, apply
    /// darktable's plans sliced to the region, bound as tables padded to darktable's most
    /// taps a pixel, so every tile of a size binds tables of one size.
    fn resample_here(&mut self) -> Result<(), PlanError> {
        let at_size = self.at_size.as_mut().expect("an export at a size");
        let input = self.current_desc;
        let region = at_size.input_region();
        at_size.pending = false;
        let plans = at_size.plans.take();
        let resample = at_size.resample;
        let [x, y, width, height] = at_size.region;
        let [left, top] = input.origin;
        let held = input.format == Format::Rgba
            && region[0] >= left
            && region[1] >= top
            && region[0] + region[2] <= left + input.width
            && region[1] + region[3] <= top + input.height;
        if !held {
            return Err(PlanError::Region {
                operation: RESAMPLE.name,
                made: false,
                desc: input,
                region,
            });
        }
        self.context.scale = resample.scale;
        self.scaled = Some(resample.output);
        self.scaled_origin = Some([x, y]);
        let Some([across, down]) = plans else {
            // At a scale of 1, the output's part of the image, as darktable's resample
            // copies it, cut out where the image is more.
            if input.region() == region {
                return Ok(());
            }
            let params = [input.width, x - left, y - top, 4]
                .iter()
                .flat_map(|word| word.to_le_bytes())
                .collect();
            let output = ImageDesc {
                width,
                height,
                origin: [x, y],
                ..input
            };
            // The copy's header origin is its first row, 0.
            return self.push(
                REGION.name,
                1,
                &REGION,
                "the export's size",
                Place::Sensor,
                output,
                vec![Pass::per_pixel(params, Vec::new())],
                Vec::new(),
                false,
            );
        };
        // darktable's `maxtapsapixel`, with a tap to spare, as darktable's ceil can round
        // up an integer.
        let most = if resample.scale > 1.0 {
            6
        } else {
            (6.0 / resample.scale).ceil() as usize + 1
        };
        let tables = |plan: AxisPlan, range: Range<u32>, base: u32| {
            let pixels = range.len();
            let mut slice = plan.slice(range, base);
            slice.index.resize(pixels * most, 0);
            slice.weight.resize(pixels * most, 0.0);
            slice.tables()
        };
        let rows = region[3];
        let params: Vec<u8> = [input.width, region[1] - top, width, 0]
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .collect();
        let bindings = |image: Binding| {
            vec![
                image,
                Binding::new(2, Resource::Uniforms),
                Binding::new(3, Resource::Table(0)),
                Binding::new(4, Resource::Table(1)),
                Binding::new(5, Resource::Table(2)),
                Binding::new(10, Resource::Scratch(0)),
            ]
        };
        let kind = PassKind::Spatial {
            radius: (3.0 / resample.scale.min(1.0)).ceil() as u32,
        };
        let across = Pass {
            entry_point: "across",
            kind,
            params: params.clone(),
            tables: tables(across, x..x + width, left),
            bindings: bindings(Binding::new(0, Resource::Input)),
            size: Some([width, rows]),
            strips: None,
        };
        let down = Pass {
            entry_point: "main",
            kind,
            params,
            tables: tables(down, y..y + height, region[1]),
            bindings: bindings(Binding::new(1, Resource::Output)),
            size: None,
            strips: None,
        };
        let output = ImageDesc {
            width,
            height,
            origin: [x, y],
            ..input
        };
        let scratch = u64::from(width) * u64::from(rows) * 16;
        self.push(
            RESAMPLE.name,
            1,
            &RESAMPLE,
            "the export's size",
            Place::At([x, y]),
            output,
            vec![across, down],
            vec![scratch],
            false,
        )
    }

    /// With geometry, where the operations after the raw stage start: the region the
    /// first of them reads, cut out by the engine's `region` copy when what the raw stage,
    /// the viewport's downscale or a tile's copy made is more (decision 16). An export's
    /// demosaic and a tile's copy make it exactly, so only the viewport and an RGBA input
    /// cut.
    fn start(&mut self) -> Result<(), PlanError> {
        let Some(region) = self.regions.as_ref().map(|regions| regions.region) else {
            return Ok(());
        };
        let current = self.current_desc;
        if current.region() == region {
            return Ok(());
        }
        let [x, y, width, height] = region;
        let [left, top] = current.origin;
        let within_current = x
            .checked_sub(left)
            .zip(y.checked_sub(top))
            .filter(|&(dx, dy)| within([dx, dy, width, height], [current.width, current.height]));
        let Some((dx, dy)) = within_current.filter(|_| current.format == Format::Rgba) else {
            return Err(PlanError::Region {
                operation: REGION.name,
                made: false,
                desc: current,
                region,
            });
        };
        let params = [current.width, dx, dy, 4]
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .collect();
        let output = ImageDesc {
            width,
            height,
            origin: [x, y],
            ..current
        };
        // The copy's header origin is its first row, 0.
        self.push(
            REGION.name,
            1,
            &REGION,
            "the first operation's region",
            Place::Sensor,
            output,
            vec![Pass::per_pixel(params, Vec::new())],
            Vec::new(),
            false,
        )?;
        if self.plan.edit_cache.is_some() {
            self.plan.edit_cache = Some(self.current_desc);
        }
        Ok(())
    }

    /// The viewport's tiles (decision 14): from the raw stage's output, a cache point,
    /// each tile copies its part with the border the `tiled` instances need, runs them,
    /// and downscales the part of the image they make into its part of the viewport
    /// image, which then is a cache point for every later operation, at the viewport's
    /// scale.
    fn viewport_tiles(&mut self, tiling: &Tiling, tiled: &[&Instance]) -> Result<(), PlanError> {
        let (source, source_desc) = (self.current, self.current_desc);
        let full = tiling.full;
        let [fitted_width, fitted_height] = tiling.output;
        let viewport = ImageDesc {
            width: fitted_width,
            height: fitted_height,
            origin: [0, 0],
            ..full
        };
        let scale = self
            .regions
            .as_ref()
            .map_or(fitted_width as f32 / full.width as f32, |regions| {
                regions.start.scale
            });
        let border: u32 = tiled
            .iter()
            .map(|instance| instance.operation.manifest().radius)
            .sum();
        let first = self.plan.operations.len();

        // Every tile's buffers hold the nominal tile, the image over the tiles across or
        // down, one footprint and one colour pattern larger: a tile of a viewport cut
        // into n tiles across is at most ⌈source / n⌉ + ⌈source / output⌉ wide, and its
        // copy starts up to a pattern early. The same tiles at a smaller viewport whose
        // footprint is no larger then always fit these buffers, and at one whose
        // footprint is larger they usually do; a larger viewport needs a larger
        // viewport image. A tile that large, its operations prepared and dropped, sizes them.
        let period = self
            .context
            .camera
            .as_ref()
            .and_then(|camera| camera.cfa.as_ref())
            .map_or([1, 1], |cfa| [cfa.width, cfa.height]);
        let image = [full.width, full.height];
        let across = [
            tiling
                .tiles
                .iter()
                .filter(|tile| tile.output[1] == 0)
                .count() as u32,
            tiling
                .tiles
                .iter()
                .filter(|tile| tile.output[0] == 0)
                .count() as u32,
        ];
        let probe: [u32; 2] = std::array::from_fn(|axis| {
            let nominal = image[axis].div_ceil(across[axis].max(1));
            let footprint = image[axis].div_ceil(tiling.output[axis]);
            let largest = tiling.tiles.iter().map(|tile| tile.source[axis + 2]).max();
            (nominal + footprint + period[axis])
                .max(largest.unwrap_or(0))
                .min(image[axis])
        });
        let probe = [
            (image[0] - probe[0]) / 2,
            (image[1] - probe[1]) / 2,
            probe[0],
            probe[1],
        ];
        let (key, lines) = (self.key, self.lines.len());
        self.context = ImageContext {
            region: probe,
            ..self.plan.image.clone()
        };
        self.slot = Some(0);
        self.sizing = true;
        self.tile_input(probe, border)?;
        for instance in tiled {
            self.add(instance, None)?;
        }
        self.sizing = false;
        self.plan.operations.truncate(first);
        self.lines.truncate(lines);
        (self.current, self.current_desc, self.key) = (source, source_desc, key);

        for tile in &tiling.tiles {
            (self.current, self.current_desc) = (source, source_desc);
            self.context = ImageContext {
                region: tile.source,
                ..self.plan.image.clone()
            };
            self.scaled = None;
            self.slot = Some(0);
            self.tile_input(tile.source, border)?;
            for instance in tiled {
                self.add(instance, None)?;
            }
            // The tile's part of the image, exactly, or its downscale would read the
            // wrong pixels: demosaic writes all of its input when the region is not
            // inside it.
            let [x, y, width, height] = tile.source;
            let made = self.current_desc;
            let origin = [full.sensor_origin[0] + x, full.sensor_origin[1] + y];
            if (made.width, made.height, made.sensor_origin, made.format)
                != (width, height, origin, Format::Rgba)
            {
                return Err(PlanError::TileRegion {
                    operation: self.plan.operations.last().map_or("", |op| op.id),
                    desc: made,
                    region: tile.source,
                });
            }
            let [x, y, width, _] = tile.source;
            let params = downscale_params([full.width, full.height], tiling.output, [x, y, width])?;
            let [ox, oy, ow, oh] = tile.output;
            self.context.scale = scale;
            self.scaled = Some(tiling.output);
            self.push(
                DOWNSCALE.name,
                1,
                &DOWNSCALE,
                "viewport scale",
                Place::At([ox, oy]),
                viewport,
                vec![Pass {
                    size: Some([ow, oh]),
                    ..Pass::per_pixel(params, Vec::new())
                }],
                Vec::new(),
                false,
            )?;
        }
        self.slot = None;
        self.plan.tiled = Some(first..self.plan.operations.len());
        self.cache_current();
        self.context = ImageContext {
            scale,
            ..self.plan.image.clone()
        };
        self.plan.edit_cache = Some(self.current_desc);
        Ok(())
    }

    /// Copies the part of the whole input a tile of `region` needs, `border` more on
    /// every side, into a buffer of its own: aligned to the colour pattern, so a mosaic
    /// keeps its pattern's phase, and clipped to the input, so the tile meets the image's
    /// real edges where it reaches them.
    fn tile_input(&mut self, region: [u32; 4], border: u32) -> Result<(), PlanError> {
        let input = self.current_desc;
        let channels = match input.format {
            Format::Mosaic => 1,
            Format::Rgba => 4,
            format @ Format::Mosaic16 => return Err(PlanError::TileInput(format)),
        };
        let camera = self.context.camera.as_ref();
        let period = match (input.format, camera.and_then(|c| c.cfa.as_ref())) {
            (Format::Mosaic, Some(cfa)) => [cfa.width.max(1), cfa.height.max(1)],
            _ => [1, 1],
        };
        // The region in the input's own coordinates: a mosaic's from the readout, an RGBA
        // image's from where it starts in its stage, which need not be the developed
        // image's corner after a geometry (decision 16).
        let declared = camera.map_or([0, 0], |c| [c.declared[0], c.declared[1]]);
        let limit = [input.width, input.height];
        let mut start = [0; 2];
        let mut size = [0; 2];
        for axis in 0..2 {
            let local = match input.format {
                Format::Rgba => region[axis].saturating_sub(input.origin[axis]),
                Format::Mosaic | Format::Mosaic16 => {
                    (declared[axis] + region[axis]).saturating_sub(input.sensor_origin[axis])
                }
            };
            start[axis] = local.saturating_sub(border) / period[axis] * period[axis];
            let end = (local + region[axis + 2] + border).min(limit[axis]);
            size[axis] = end.saturating_sub(start[axis]);
        }
        let output = ImageDesc {
            width: size[0],
            height: size[1],
            sensor_origin: [
                input.sensor_origin[0] + start[0],
                input.sensor_origin[1] + start[1],
            ],
            // An RGBA input's part lies where it starts in its stage (decision 16).
            origin: match input.format {
                Format::Rgba => [input.origin[0] + start[0], input.origin[1] + start[1]],
                Format::Mosaic | Format::Mosaic16 => input.origin,
            },
            ..input
        };
        let mut params = Vec::with_capacity(16);
        for word in [input.width, start[0], start[1], channels] {
            params.extend_from_slice(&word.to_le_bytes());
        }
        // A copy pixel by pixel, which binds only the rows of the whole image it reads
        // when that is larger than the GPU binds at once (decision 14).
        let copy = Pass {
            strips: Some(Strips::PER_PIXEL),
            ..Pass::per_pixel(params, Vec::new())
        };
        self.push(
            REGION.name,
            1,
            &REGION,
            "the tile's input",
            Place::Sensor,
            output,
            vec![copy],
            Vec::new(),
            false,
        )
    }

    /// Makes the current image, unless it is the pipe's input, a cache point.
    fn cache_current(&mut self) {
        if let Buffer::Work(index) = self.current {
            self.reserved[index] = true;
            self.plan.cached.push(self.plan.operations.len() - 1);
        }
    }
}

/// `DownscaleParams` in the kernel: the whole `source` down to the whole `output`, from
/// an input whose first pixel lies at (x, y) of the source and whose rows are this wide.
/// The kernel multiplies each output position by the source's size in 32 bits.
fn downscale_params(
    source: [u32; 2],
    output: [u32; 2],
    input: [u32; 3],
) -> Result<Vec<u8>, PlanError> {
    if (0..2).any(|axis| u64::from(source[axis]) * u64::from(output[axis]) >= 1 << 32) {
        return Err(PlanError::NotPorted(NotPorted {
            operation: DOWNSCALE.name,
            what: "a viewport this large of an image this large",
        }));
    }
    // A larger output would give some pixels an empty footprint.
    if (0..2).any(|axis| output[axis] == 0 || output[axis] > source[axis]) {
        return Err(PlanError::NotPorted(NotPorted {
            operation: DOWNSCALE.name,
            what: "a viewport larger than its image",
        }));
    }
    let [x, y, width] = input;
    Ok([source[0], source[1], output[0], output[1], x, y, width, 0]
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .collect())
}

/// How a pass of an operation that reads `input` and writes `output` dispatches under
/// `limits`: once over all of it, unless an image it binds is larger than the GPU binds at
/// once and it can run in row strips; then once per strip, binding the window of each image
/// the strip reads or writes (decision 14). A pass that would bind too much and cannot runs
/// whole, and the plan does not fit. A pass that binds only scratch and can run in strips
/// runs in bands of its rows wherever `output` is larger than the GPU binds, each as many
/// rows as a strip binding `output`'s rows has and binding no window, its scratch whole: so
/// a job's export, which plans strips of a step's size, cuts it into steps too.
fn dispatches(
    pass: &Pass,
    input: &ImageDesc,
    output: &ImageDesc,
    limits: &Limits,
) -> Vec<Dispatch> {
    let [_, height] = pass.size.unwrap_or([output.width, output.height]);
    let whole = vec![Dispatch {
        rows: [0, height],
        input: None,
        output: None,
    }];
    let binds = |resource| pass.bindings.iter().any(|b| b.resource == resource);
    let (reads, writes) = (binds(Resource::Input), binds(Resource::Output));
    // A pass of scratch alone is sized as if it wrote the output's rows.
    let covers_output = writes || !reads;
    let too_large = (reads && input.buffer_size() > limits.binding)
        || (covers_output && output.buffer_size() > limits.binding);
    let Some(strips) = pass.strips.filter(|_| too_large) else {
        return whole;
    };
    let Some(strip) = strip_height(&strips, input, output, reads, covers_output, limits) else {
        return whole;
    };
    let k = i64::from(strips.rows.max(1));
    let halo = i64::from(strips.halo);
    // The input's rows lie where the two images' places on the sensor put them.
    let shift = i64::from(output.sensor_origin[1]) - i64::from(input.sensor_origin[1]);
    let alignment = u64::from(limits.offset_alignment);
    (0..height)
        .step_by(strip as usize)
        .map(|first| {
            let end = (first + strip).min(height);
            let [a, b] = [i64::from(first) * k, i64::from(end) * k];
            Dispatch {
                rows: [first, end - first],
                input: reads
                    .then(|| window(input, [a - halo + shift, b + halo + shift], alignment)),
                output: writes.then(|| window(output, [a, b], alignment)),
            }
        })
        .collect()
}

/// The most rows of invocations a strip may have so that the windows it binds fit a
/// binding, a multiple of the pass's granularity; `None` if not even one granule fits.
fn strip_height(
    strips: &Strips,
    input: &ImageDesc,
    output: &ImageDesc,
    reads: bool,
    writes: bool,
    limits: &Limits,
) -> Option<u32> {
    let k = u64::from(strips.rows.max(1));
    let granularity = u64::from(strips.granularity.max(1));
    // A window starts up to an alignment early and ends up to a word late.
    let room = limits
        .binding
        .checked_sub(u64::from(limits.offset_alignment) + 4)?;
    let mut rows = u64::MAX;
    if reads {
        let input_rows = (room / row_bytes(input)).checked_sub(2 * u64::from(strips.halo))?;
        rows = rows.min(input_rows / k);
    }
    if writes {
        rows = rows.min(room / row_bytes(output) / k);
    }
    let rows = rows / granularity * granularity;
    (rows > 0).then(|| rows.min(u64::from(u32::MAX)) as u32)
}

/// Bytes in a row of the image, as its buffer packs it.
fn row_bytes(image: &ImageDesc) -> u64 {
    let per_pixel = match image.format {
        Format::Mosaic16 => 2,
        Format::Mosaic => 4,
        Format::Rgba => 16,
    };
    u64::from(image.width.max(1)) * per_pixel
}

/// The window of the image's buffer holding its rows `rows`, clipped to the image and at
/// least one row: its start aligned down to `alignment`, its end up to a whole word, as
/// the 16-bit mosaic's rows may end inside one.
fn window(image: &ImageDesc, rows: [i64; 2], alignment: u64) -> Window {
    let height = i64::from(image.height.max(1));
    let first = rows[0].clamp(0, height - 1) as u64;
    let end = (rows[1].clamp(0, height) as u64).max(first + 1);
    let row = row_bytes(image);
    let offset = first * row / alignment * alignment;
    let end = (end * row).next_multiple_of(4).min(image.buffer_size());
    Window {
        offset,
        size: end - offset,
    }
}

/// The largest size with `image`'s aspect that fits in `bounds`, never larger than the
/// image itself.
pub(crate) fn fit(image: (u32, u32), bounds: (u32, u32)) -> (u32, u32) {
    let [width, height] = at_scale([image.0, image.1], fit_scale(image, bounds));
    (width, height)
}

/// The scale at which `image` fits in `bounds`, at most 1.
fn fit_scale(image: (u32, u32), bounds: (u32, u32)) -> f64 {
    (bounds.0 as f64 / image.0 as f64)
        .min(bounds.1 as f64 / image.1 as f64)
        .min(1.0)
}

/// An image of `extent` at `scale`, each side rounded and at least a pixel: the viewport
/// [`fit`] makes, and with geometry every stage at the viewport's scale (decision 16).
fn at_scale(extent: [u32; 2], scale: f64) -> [u32; 2] {
    extent.map(|side| ((side as f64 * scale).round() as u32).max(1))
}

/// An operation's key: the previous key and everything the operation's output depends
/// on that `prepare` decided.
fn key(
    previous: u64,
    id: &str,
    version: u32,
    scale: f32,
    origin: [u32; 2],
    output: &ImageDesc,
    passes: &[Pass],
) -> u64 {
    let mut hasher = DefaultHasher::new();
    (previous, id, version, scale.to_bits(), origin).hash(&mut hasher);
    hash_desc(output, &mut hasher);
    for pass in passes {
        (pass.entry_point, pass.size, &pass.params, &pass.tables).hash(&mut hasher);
    }
    hasher.finish()
}

/// The whole-image passes of an operation whose input has the key `input`, and their
/// result's key (decision 15): its passes up to its last whole-image one, those in
/// between included, since they feed it. `None` without one, or when one of them writes
/// the output, which a render skipping them would leave as an earlier operation wrote it.
fn whole_image(
    input: u64,
    id: &str,
    version: u32,
    scale: f32,
    output: &ImageDesc,
    passes: &[Pass],
    scratch: &[u64],
) -> Option<WholeImage> {
    let count = passes
        .iter()
        .rposition(|pass| pass.kind == PassKind::Reduction)?
        + 1;
    let passes = &passes[..count];
    let writes_output = passes
        .iter()
        .flat_map(|pass| &pass.bindings)
        .any(|binding| binding.resource == Resource::Output);
    if writes_output {
        return None;
    }
    let mut hasher = DefaultHasher::new();
    (input, id, version, scale.to_bits(), count, scratch).hash(&mut hasher);
    hash_desc(output, &mut hasher);
    hash_passes(passes, &mut hasher);
    Some(WholeImage {
        passes: count,
        key: hasher.finish(),
        full_resolution: None,
    })
}

/// The full-resolution part of an operation's whole-image passes `passes` in the
/// viewport, and its key (decision 15): its passes up to the last that binds
/// [`Resource::FullResolution`], keyed on that image's key `image`, the operation's ID
/// and version, scale 1, those passes' bytes and the sizes of the scratch buffers they
/// bind, of the operation's `scratch`. `None`, the operation keeping one part, where
/// none binds that image, or where the part depends on the viewport's size: a pass of
/// it binds the input or the output, the viewport's images, or covers the output,
/// having no size of its own.
fn full_resolution_part(
    image: u64,
    id: &str,
    version: u32,
    passes: &[Pass],
    scratch: &[u64],
) -> Option<FullResolutionPart> {
    let count = passes
        .iter()
        .rposition(|pass| binds(pass, Resource::FullResolution))?
        + 1;
    let passes = &passes[..count];
    let viewport = passes.iter().any(|pass| {
        pass.size.is_none() || binds(pass, Resource::Input) || binds(pass, Resource::Output)
    });
    if viewport {
        return None;
    }
    let mut hasher = DefaultHasher::new();
    (image, id, version, 1.0f32.to_bits(), count).hash(&mut hasher);
    for index in bound_scratch(passes) {
        (index, scratch[index]).hash(&mut hasher);
    }
    hash_passes(passes, &mut hasher);
    Some(FullResolutionPart {
        passes: count,
        key: hasher.finish(),
    })
}

/// Whether `pass` binds `resource`.
fn binds(pass: &Pass, resource: Resource) -> bool {
    pass.bindings
        .iter()
        .any(|binding| binding.resource == resource)
}

/// The operation's scratch buffers that `passes` bind, as indices into its own, in
/// order.
fn bound_scratch(passes: &[Pass]) -> Vec<usize> {
    let mut bound: Vec<usize> = passes
        .iter()
        .flat_map(|pass| &pass.bindings)
        .filter_map(|binding| match binding.resource {
            Resource::Scratch(index) => Some(index),
            _ => None,
        })
        .collect();
    bound.sort_unstable();
    bound.dedup();
    bound
}

/// Hashes what whole-image passes compute: their entry points, kinds, uniforms, tables,
/// bindings and sizes, never their row strips, which change only how they dispatch
/// (decision 14).
fn hash_passes(passes: &[Pass], hasher: &mut DefaultHasher) {
    for pass in passes {
        let Pass {
            entry_point,
            kind,
            params,
            tables,
            bindings,
            size,
            strips: _,
        } = pass;
        (entry_point, kind, params, tables, bindings, size).hash(hasher);
    }
}

/// Hashes what an image is, but for its place in its stage, `extent`, `scaled_extent`
/// and `origin` (decisions 15 and 16), which keys hash where they vary: an operation's
/// key its origin, a geometry's its regions and extent, which every later key chains,
/// and a whole-image operation's the passes and scratch it prepares from its stage at the
/// pipe's scale. So a plan with no geometry and no resample keeps its keys.
fn hash_desc(desc: &ImageDesc, hasher: &mut DefaultHasher) {
    let format = match desc.format {
        Format::Mosaic16 => 0u8,
        Format::Mosaic => 1,
        Format::Rgba => 2,
    };
    (
        desc.width,
        desc.height,
        format,
        desc.sensor_origin,
        desc.late_correction,
    )
        .hash(hasher);
    for value in desc.maximum.iter().chain(&desc.white_balance) {
        value.to_bits().hash(hasher);
    }
}

/// "1 pass", "3 passes".
fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// The perf domain's detail for a render: each pass's GPU time, `passes` in dispatch
/// order beside the operation each belongs to, `operations`. A pass named after an entry
/// point of its operation's kernel is named after the operation too, `pass demosaic
/// rcd_step_1: 2.1 ms`, so the lines add up by operation as darktable's `-d perf` lists
/// its modules; a pass that runs once a viewport tile is added up, `(20×)`.
fn pass_lines(passes: &[(&str, f64)], operations: &[&str]) -> Vec<String> {
    let mut totals: Vec<(&str, &str, f64, usize)> = Vec::new();
    for (&(name, ms), &operation) in passes.iter().zip(operations) {
        match totals
            .iter_mut()
            .find(|(op, pass, ..)| (*op, *pass) == (operation, name))
        {
            Some((.., total, runs)) => {
                *total += ms;
                *runs += 1;
            }
            None => totals.push((operation, name, ms, 1)),
        }
    }
    totals
        .into_iter()
        .map(|(operation, name, ms, runs)| {
            let pass = if name == operation {
                name.to_owned()
            } else {
                format!("{operation} {name}")
            };
            match runs {
                1 => format!("pass {pass}: {ms:.1} ms"),
                _ => format!("pass {pass}: {ms:.1} ms ({runs}×)"),
            }
        })
        .collect()
}

/// Parameter values as `name value` pairs, choices by name.
fn describe(schema: &Schema, values: &ParamValues) -> Vec<String> {
    schema
        .params
        .iter()
        .filter_map(|param| {
            let value = values.get(param.name)?;
            Some(match (param.kind, value) {
                (Kind::Choice { choices, .. }, Value::Choice(index)) => {
                    format!(
                        "{} {}",
                        param.name,
                        choices.get(index as usize).unwrap_or(&"?")
                    )
                }
                (_, value) => format!("{} {value}", param.name),
            })
        })
        .collect()
}

/// The size of the buffer made for a scratch buffer of `bytes`: at least a word, since
/// an empty buffer cannot be bound.
fn scratch_size(bytes: u64) -> u64 {
    bytes.max(4)
}

/// Mebibytes, for the memory domain.
fn mebibytes(bytes: u64) -> f64 {
    bytes as f64 / (1 << 20) as f64
}

/// The uniform block of a dispatch of its operation's pass `index`: the header for its
/// operation's region of its stage and scale, or, for a pass of its full-resolution part,
/// for the whole image at scale 1 it runs over (decision 15), its rows and the windows it
/// binds, then the pass's parameters.
fn uniform_block(op: &PlannedOperation, index: usize, dispatch: &Dispatch) -> Vec<u8> {
    let pass = &op.passes[index];
    let [width, _] = pass
        .size
        .unwrap_or([op.output_desc.width, op.output_desc.height]);
    let at_full_resolution = op
        .whole_image
        .and_then(|whole| whole.full_resolution)
        .is_some_and(|part| index < part.passes);
    let (origin, scale) = if at_full_resolution {
        ([0, 0], 1.0)
    } else {
        (op.origin, op.scale)
    };
    let header = PassHeader {
        origin: [origin[0], origin[1] + dispatch.rows[0]],
        full_width: op.extent[0],
        full_height: op.extent[1],
        scale,
        input_base: Window::base(dispatch.input),
        output_base: Window::base(dispatch.output),
        ..PassHeader::whole(width, dispatch.rows[1])
    };
    header.uniform_block(&pass.params)
}

/// Workgroups covering a dispatch of a pass: one invocation per pixel of the pass's
/// width and the dispatch's rows.
fn workgroups(op: &PlannedOperation, pass: &Pass, dispatch: &Dispatch) -> (u32, u32) {
    let [width, _] = pass
        .size
        .unwrap_or([op.output_desc.width, op.output_desc.height]);
    (
        width.div_ceil(WORKGROUP),
        dispatch.rows[1].div_ceil(WORKGROUP),
    )
}

/// A buffer bound whole, or the window of it a dispatch binds.
fn bound(buffer: &wgpu::Buffer, window: Option<Window>) -> wgpu::BindingResource<'_> {
    match window {
        Some(window) => wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer,
            offset: window.offset,
            size: NonZeroU64::new(window.size),
        }),
        None => buffer.as_entire_binding(),
    }
}

/// The resources a dispatch binds besides its own uniforms.
struct Resources<'a> {
    input: &'a wgpu::Buffer,
    output: &'a wgpu::Buffer,
    /// The viewport's full-resolution image, for an operation that binds it (decision
    /// 15).
    full_resolution: Option<&'a wgpu::Buffer>,
    tables: &'a [wgpu::Buffer],
    /// The operation's scratch buffers, in its own order.
    scratch: Vec<&'a wgpu::Buffer>,
}

/// The bind group of one dispatch of a pass: exactly the resources the pass lists, its
/// images whole or the dispatch's windows of them.
fn bind_group(
    gpu: &GpuContext,
    kernel: &ImageKernel,
    label: &str,
    pass: &Pass,
    dispatch: &Dispatch,
    uniform: &wgpu::Buffer,
    resources: &Resources,
) -> wgpu::BindGroup {
    let entries: Vec<wgpu::BindGroupEntry> = pass
        .bindings
        .iter()
        .map(|binding| wgpu::BindGroupEntry {
            binding: binding.binding,
            resource: match binding.resource {
                Resource::Input => bound(resources.input, dispatch.input),
                Resource::Output => bound(resources.output, dispatch.output),
                Resource::Uniforms => uniform.as_entire_binding(),
                Resource::Table(index) => resources.tables[index].as_entire_binding(),
                Resource::Scratch(index) => resources.scratch[index].as_entire_binding(),
                Resource::FullResolution => resources
                    .full_resolution
                    .expect("planned with the full-resolution image")
                    .as_entire_binding(),
            },
        })
        .collect();
    gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout: &kernel.pipeline().get_bind_group_layout(0),
        entries: &entries,
    })
}

/// The kernels a pipeline has compiled, one per kernel and entry point however many
/// passes run it, kept across every rebuild over the same input (decision 28): an
/// operation that joins the plan compiles once for the photo, and again never. The
/// session keeps them across photos too, a pipeline taking them as it is built
/// ([`Pipeline::with_kernels`]) and giving them back as it goes ([`Pipeline::into_kernels`]),
/// and compiles a plan's before its raw arrives ([`compile`]), as a photo opens in stages
/// (decision 77).
#[derive(Default, Clone)]
pub(crate) struct Kernels {
    compiled: Vec<ImageKernel>,
    /// Where each kernel's entry point is in `compiled`, by kernel name and entry point.
    index: HashMap<(&'static str, String), usize>,
    /// Each kernel's WGSL by its name, to check that a name means one kernel.
    #[cfg(debug_assertions)]
    sources: HashMap<&'static str, &'static str>,
}

impl Kernels {
    /// Where `kernel`'s `entry_point` is in `compiled`, compiled now if it is not yet.
    async fn get(
        &mut self,
        gpu: &GpuContext,
        kernel: &'static Kernel,
        entry_point: &str,
        label: &str,
    ) -> Result<usize, ComputeError> {
        #[cfg(debug_assertions)]
        {
            let source = self.sources.entry(kernel.name).or_insert(kernel.wgsl);
            assert!(
                *source == kernel.wgsl,
                "two kernels named {} with different WGSL",
                kernel.name
            );
        }
        if let Some(&index) = self.index.get(&(kernel.name, entry_point.to_owned())) {
            return Ok(index);
        }
        let compiled = ImageKernel::with_entry_point(gpu, label, kernel.wgsl, entry_point).await?;
        self.compiled.push(compiled);
        let index = self.compiled.len() - 1;
        self.index
            .insert((kernel.name, entry_point.to_owned()), index);
        Ok(index)
    }

    /// How many entry points it holds.
    pub(crate) fn len(&self) -> usize {
        self.compiled.len()
    }
}

/// Compiles into `kernels` every entry point `plan`'s passes run that they lack, and
/// returns how many it compiled, so a pipeline built for the plan later compiles none.
pub(crate) async fn compile(
    gpu: &GpuContext,
    plan: &Plan,
    kernels: &mut Kernels,
) -> Result<usize, ComputeError> {
    let had = kernels.len();
    for op in &plan.operations {
        for pass in &op.passes {
            let label = if pass.entry_point == "main" {
                op.id
            } else {
                pass.entry_point
            };
            kernels.get(gpu, op.kernel, pass.entry_point, label).await?;
        }
    }
    Ok(kernels.len() - had)
}

/// The keys of the whole-image results an operation's scratch buffers hold: all its
/// whole-image passes' (decision 15), and its full-resolution part's (decision 15).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Results {
    whole: Option<u64>,
    full_resolution: Option<u64>,
}

impl Results {
    /// What an operation's scratch holds once its whole-image passes `whole` have run.
    fn of(whole: Option<WholeImage>) -> Self {
        Self {
            whole: whole.map(|whole| whole.key),
            full_resolution: whole
                .and_then(|whole| whole.full_resolution)
                .map(|part| part.key),
        }
    }

    /// Whether a render skips pass `index` of an operation whose whole-image passes are
    /// `whole`: it is one of them, or of their full-resolution part, whose result these
    /// hold.
    fn skip(&self, whole: Option<WholeImage>, index: usize) -> bool {
        let Some(whole) = whole else {
            return false;
        };
        let part = whole.full_resolution;
        (index < whole.passes && self.whole == Some(whole.key))
            || part
                .is_some_and(|part| index < part.passes && self.full_resolution == Some(part.key))
    }
}

/// What a pipeline rebuilt over the same input takes from the one it replaces (decision
/// 28): the input, every compiled kernel, the buffer of each cache point of the new plan
/// that an old one holds with the same key at the same size, the edit cache's too, the
/// scratch buffers of each operation whose whole-image result an old one holds with the
/// same key (decision 15), or of its full-resolution part only, those that part binds
/// (decision 15), and every other working and scratch buffer of the same size at the
/// same place, for its memory only. Everything else is freed before the new pipeline
/// makes its buffers.
struct Kept {
    input: GpuImage,
    kernels: Kernels,
    /// Per working buffer of the new plan, the buffer it takes over, if any.
    work: Vec<Option<GpuImage>>,
    /// Per scratch buffer of the new plan, the buffer it takes over, if any.
    scratch: Vec<Option<wgpu::Buffer>>,
    /// Per cache point of the new plan, the key its buffer holds already.
    held: Vec<Option<u64>>,
    /// Per operation of the new plan, the keys of the whole-image results its scratch
    /// buffers hold already.
    results: Vec<Results>,
    edit: Option<EditCache>,
}

impl Kept {
    /// Only the input, for a pipeline built from nothing.
    fn nothing(input: GpuImage, plan: &Plan) -> Self {
        Self {
            input,
            kernels: Kernels::default(),
            work: plan.work.iter().map(|_| None).collect(),
            scratch: plan.scratch.iter().map(|_| None).collect(),
            held: vec![None; plan.cached.len()],
            results: vec![Results::default(); plan.operations.len()],
            edit: None,
        }
    }

    /// Takes the scratch buffers `held` holds for the operations of `plan` they belong to,
    /// in place of any other.
    fn hold(&mut self, plan: &Plan, held: &Held) {
        for (id, buffers) in held {
            for op in plan.operations.iter().filter(|op| op.id == *id) {
                assert_eq!(op.scratch.len(), buffers.len(), "{id}'s scratch buffers");
                for (&index, buffer) in op.scratch.iter().zip(buffers) {
                    assert_eq!(
                        buffer.size(),
                        scratch_size(plan.scratch[index]),
                        "{id} holds a scratch buffer of another size"
                    );
                    self.scratch[index] = Some(buffer.clone());
                }
            }
        }
    }

    /// The bytes of GPU memory it hands over, for the memory domain.
    fn bytes(&self) -> u64 {
        let work = self
            .work
            .iter()
            .flatten()
            .map(|image| image.buffer().size());
        let scratch = self.scratch.iter().flatten().map(wgpu::Buffer::size);
        let edit = self.edit.iter().map(|edit| edit.buffer.buffer().size());
        work.chain(scratch).chain(edit).sum()
    }
}

/// Scratch buffers an export holds across its tiles and rounds (decision 15): an
/// operation's, by its ID, in its own order, which every pipeline that runs it binds in
/// place of its own.
pub(crate) type Held = [(&'static str, Vec<wgpu::Buffer>)];

/// Which derivative entry point [`Pipeline::derivative`] dispatches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Derivative {
    /// Reverse mode, `<kernel>_backward`.
    Backward,
    /// Forward mode, `<kernel>_forward_diff`.
    ForwardDiff,
}

/// A plan's GPU resources: its compiled kernels, its passes with their uniforms, tables
/// and bind groups, the buffers they read and write, and its cache points.
pub struct Pipeline {
    plan: Plan,
    input: GpuImage,
    work: Vec<GpuImage>,
    scratch: Vec<wgpu::Buffer>,
    /// Each entry point the plan runs, and those of the plans before it over the same
    /// input.
    kernels: Kernels,
    passes: Vec<CompiledPass>,
    timer: Option<GpuTimer>,
    /// The key each cache point's buffer holds, in [`Plan::cached`]'s order; `None`
    /// until it is first written.
    held: Vec<Option<u64>>,
    /// Per operation, the keys of the whole-image results its scratch buffers hold
    /// ([`WholeImage`]); none until its whole-image passes run.
    results: Vec<Results>,
    /// The cache point at the edited operation's input.
    edit: Option<EditCache>,
    /// Each operation's key at the last render.
    rendered: Vec<u64>,
    /// The work and scratch buffers it made that no render has used yet, by index, which a
    /// render in parts touches first, each in a step of its own.
    unused: Unused,
}

/// A pipeline's work and scratch buffers that it made and no render has used yet.
#[derive(Default)]
struct Unused {
    work: Vec<bool>,
    scratch: Vec<bool>,
}

/// A copy of the edited operation's input at the viewport's size, so that while one
/// operation's parameters change, every render starts at it (`docs/engine.md`, Caching).
struct EditCache {
    buffer: GpuImage,
    /// The operation whose input it holds, and that input's key.
    held: Option<(usize, u64)>,
}

struct CompiledPass {
    /// Its operation, as an index into [`Plan::operations`].
    op: usize,
    /// Its place among its operation's passes.
    index: usize,
    label: &'static str,
    /// Its entry point, as an index into the pipeline's compiled kernels.
    kernel: usize,
    tables: Vec<wgpu::Buffer>,
    /// One per dispatch: the whole pass, or each of its row strips.
    dispatches: Vec<CompiledDispatch>,
}

struct CompiledDispatch {
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    workgroups: (u32, u32),
}

/// What a render's submission ran, for the bookkeeping and the timings after it.
struct Submitted {
    /// Each operation's key as it ran.
    keys: Vec<u64>,
    /// The operation it began at.
    first: usize,
    /// The operation whose input the edit cache took.
    capture: Option<usize>,
    /// Whether each compiled pass ran.
    runs: Vec<bool>,
    /// The passes that ran, in order.
    names: Vec<&'static str>,
    /// Their timestamps, copied out for the CPU, where they were taken.
    copied: Option<wgpu::Buffer>,
}

/// A render in parts ([`Pipeline::parts`]), each one dispatch of one pass or a touch of a
/// new buffer, for a job to submit one a step.
pub(crate) struct Parts {
    /// What the render runs, for the bookkeeping after its last part.
    submitted: Submitted,
    /// The parts, in order.
    order: Vec<Part>,
}

/// The bytes a [`Part::Touch`] clears at the start of its buffer.
const TOUCH: u64 = 256;

/// One part of a render in parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    /// A compiled pass's dispatch, by their indices.
    Dispatch(usize, usize),
    /// A clear of the first bytes of a buffer the pipeline made, before the first
    /// dispatch that binds it, so that whatever its first use on the GPU costs, Metal
    /// making it resident on the M4, is a step of its own, not the first strip's.
    Touch(Touched),
}

/// A buffer a [`Part::Touch`] clears the start of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Touched {
    Work(usize),
    Scratch(usize),
}

impl Parts {
    /// How many parts the render takes.
    pub(crate) fn len(&self) -> usize {
        self.order.len()
    }
}

impl Pipeline {
    /// Compiles the plan's passes over `input`, which must be the plan's input image,
    /// or fails if the GPU cannot make or bind the buffers the plan needs.
    pub async fn new(gpu: &GpuContext, plan: Plan, input: GpuImage) -> Result<Self, ComputeError> {
        Self::holding(gpu, plan, input, &[]).await
    }

    /// [`Self::new`], taking `kernels`, compiled for earlier photos, so it compiles only
    /// what they lack.
    pub(crate) async fn with_kernels(
        gpu: &GpuContext,
        plan: Plan,
        input: GpuImage,
        kernels: Kernels,
    ) -> Result<Self, ComputeError> {
        let mut kept = Kept::nothing(input, &plan);
        kept.kernels = kernels;
        Self::build(gpu, plan, kept).await
    }

    /// Its compiled kernels, for the next photo's pipeline, the rest freed.
    pub(crate) fn into_kernels(self) -> Kernels {
        self.kernels
    }

    /// [`Self::new`], binding the scratch buffers `held` holds for the operations they
    /// belong to instead of making its own.
    pub(crate) async fn holding(
        gpu: &GpuContext,
        plan: Plan,
        input: GpuImage,
        held: &Held,
    ) -> Result<Self, ComputeError> {
        let mut kept = Kept::nothing(input, &plan);
        kept.hold(&plan, held);
        Self::build(gpu, plan, kept).await
    }

    /// The scratch buffers of the operation `id`, in its own order, for an export to hold
    /// (decision 15).
    pub(crate) fn scratch_of(&self, id: &str) -> Vec<wgpu::Buffer> {
        self.plan
            .operations
            .iter()
            .find(|op| op.id == id)
            .map(|op| {
                op.scratch
                    .iter()
                    .map(|&i| self.scratch[i].clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Compiles the plan's passes over the input `kept` holds, taking the kernels and the
    /// buffers it keeps and making every other buffer.
    async fn build(gpu: &GpuContext, plan: Plan, kept: Kept) -> Result<Self, ComputeError> {
        let kept_bytes = kept.bytes();
        let Kept {
            input,
            mut kernels,
            work: kept_work,
            scratch: kept_scratch,
            held,
            results,
            edit: kept_edit,
        } = kept;
        assert_eq!(input.desc(), plan.input, "the input the plan was made for");
        let unused = Unused {
            work: kept_work.iter().map(Option::is_none).collect(),
            scratch: kept_scratch.iter().map(Option::is_none).collect(),
        };
        let start = Instant::now();
        let limits = gpu.device.limits();
        if let Some((operation, pass, bytes)) = plan.oversized_binding(
            limits.max_storage_buffer_binding_size,
            input.buffer().size(),
        ) {
            return Err(ComputeError::Binding {
                operation,
                pass,
                bytes,
                limit: limits.max_storage_buffer_binding_size,
            });
        }
        for op in &plan.operations {
            for &index in &op.scratch {
                if plan.scratch[index] > limits.max_buffer_size {
                    return Err(ComputeError::Buffer {
                        operation: op.id,
                        bytes: plan.scratch[index],
                        limit: limits.max_buffer_size,
                    });
                }
            }
            let alignment = u64::from(limits.min_storage_buffer_offset_alignment);
            let windows = op.dispatches.iter().flatten();
            assert!(
                windows
                    .flat_map(|d| [d.input, d.output])
                    .flatten()
                    .all(|w| w.offset % alignment == 0),
                "{}: the plan was made for a GPU with another offset alignment",
                op.id
            );
        }
        for op in &plan.operations {
            let most = op.dispatches.iter().map(Vec::len).max().unwrap_or(0);
            if most > 1 {
                // The largest window a strip binds: within the GPU's binding, or within the
                // strips' size a job's export plans.
                let window = (op.dispatches.iter().flatten())
                    .flat_map(|d| [d.input, d.output])
                    .flatten()
                    .map(|w| w.size)
                    .max()
                    .unwrap_or(0);
                tracing::debug!(
                    target: "pipe",
                    "{} in row strips: up to {most} a pass, windows up to {:.1} MiB",
                    op.id,
                    mebibytes(window),
                );
                let passes: Vec<String> = op
                    .passes
                    .iter()
                    .zip(&op.dispatches)
                    .map(|(pass, d)| format!("{} {}", pass.entry_point, d.len()))
                    .collect();
                for line in pack(&format!("{} strips:", op.id), &passes) {
                    tracing::trace!(target: "pipe", "{line}");
                }
            }
        }
        let work_bytes: u64 = plan.work.iter().map(ImageDesc::buffer_size).sum();
        let scratch_bytes: u64 = plan.scratch.iter().sum();
        let edit_bytes = plan.edit_cache.map_or(0, |desc| desc.buffer_size());
        let input_bytes = plan.input.buffer_size();
        let mut parts = vec![format!("work {:.0}", mebibytes(work_bytes))];
        if !plan.scratch.is_empty() {
            parts.push(format!("scratch {:.0}", mebibytes(scratch_bytes)));
        }
        if edit_bytes > 0 {
            parts.push(format!("edit cache {:.0}", mebibytes(edit_bytes)));
        }
        parts.push(format!("input {:.0}", mebibytes(input_bytes)));
        tracing::debug!(
            target: "memory",
            "buffers: {:.0} MiB ({}){}",
            mebibytes(work_bytes + scratch_bytes + edit_bytes + input_bytes),
            parts.join(", "),
            match (work_bytes + scratch_bytes + edit_bytes).saturating_sub(kept_bytes) {
                _ if kept_bytes == 0 => String::new(),
                0 => ", all kept".to_owned(),
                new => format!(", new: {:.0} MiB", mebibytes(new)),
            }
        );
        for (index, desc) in plan.work.iter().enumerate() {
            tracing::trace!(target: "memory", "work buffer {index}: {desc}");
        }
        for (index, &bytes) in plan.scratch.iter().enumerate() {
            let size = match bytes {
                0..1024 => format!("{bytes} B"),
                1024..1_048_576 => format!("{:.0} KiB", bytes as f64 / 1024.0),
                _ => format!("{:.1} MiB", mebibytes(bytes)),
            };
            tracing::trace!(target: "memory", "scratch buffer {index}: {size}");
        }
        // A kept buffer instead of a new one.
        let work = plan
            .work
            .iter()
            .zip(kept_work)
            .map(|(desc, kept)| match kept {
                Some(mut image) => {
                    image.set_desc(*desc);
                    Ok(image)
                }
                None => GpuImage::with_desc(gpu, *desc),
            })
            .collect::<Result<Vec<_>, _>>()?;
        let edit = match (kept_edit, plan.edit_cache) {
            (Some(edit), Some(_)) => Some(edit),
            (_, Some(desc)) => Some(EditCache {
                buffer: GpuImage::with_desc(gpu, desc)?,
                held: None,
            }),
            (_, None) => None,
        };
        let scratch: Vec<wgpu::Buffer> = plan
            .scratch
            .iter()
            .zip(kept_scratch)
            .map(|(&size, kept)| {
                kept.unwrap_or_else(|| {
                    gpu.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("scratch"),
                        size: scratch_size(size),
                        // A render in parts touches it with a clear.
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    })
                })
            })
            .collect();
        let image = |buffer: Buffer| match buffer {
            Buffer::Input => &input,
            Buffer::Work(index) => &work[index],
        };

        let had = kernels.compiled.len();
        let mut passes = Vec::new();
        for (index, op) in plan.operations.iter().enumerate() {
            for (place, (pass, dispatches)) in op.passes.iter().zip(&op.dispatches).enumerate() {
                let label = if pass.entry_point == "main" {
                    op.id
                } else {
                    pass.entry_point
                };
                let kernel = kernels.get(gpu, op.kernel, pass.entry_point, label).await?;
                let tables: Vec<wgpu::Buffer> = pass
                    .tables
                    .iter()
                    .map(|contents| {
                        gpu.device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some(label),
                                contents,
                                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                            })
                    })
                    .collect();
                let resources = Resources {
                    input: image(op.input).buffer(),
                    output: image(op.output).buffer(),
                    full_resolution: op.full_resolution.map(|buffer| image(buffer).buffer()),
                    tables: &tables,
                    scratch: op.scratch.iter().map(|&i| &scratch[i]).collect(),
                };
                let dispatches = dispatches
                    .iter()
                    .map(|dispatch| {
                        let uniform =
                            gpu.device
                                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                    label: Some(label),
                                    contents: &uniform_block(op, place, dispatch),
                                    usage: wgpu::BufferUsages::UNIFORM
                                        | wgpu::BufferUsages::COPY_DST,
                                });
                        let bind_group = bind_group(
                            gpu,
                            &kernels.compiled[kernel],
                            label,
                            pass,
                            dispatch,
                            &uniform,
                            &resources,
                        );
                        CompiledDispatch {
                            uniform,
                            bind_group,
                            workgroups: workgroups(op, pass, dispatch),
                        }
                    })
                    .collect();
                passes.push(CompiledPass {
                    op: index,
                    index: place,
                    label,
                    kernel,
                    tables,
                    dispatches,
                });
            }
        }
        let timer = GpuTimer::new(gpu, passes.len() as u32)
            .filter(|timer| timer.slots() as usize >= passes.len());
        // The plan's kernels that an earlier plan compiled.
        let mut reused: Vec<usize> = passes
            .iter()
            .map(|pass| pass.kernel)
            .filter(|&kernel| kernel < had)
            .collect();
        reused.sort_unstable();
        reused.dedup();
        tracing::debug!(
            target: "perf",
            "compiled {} in {:.0} ms, passes: {}{}",
            count(kernels.compiled.len() - had, "kernel", "kernels"),
            start.elapsed().as_secs_f64() * 1e3,
            passes.len(),
            if reused.is_empty() {
                String::new()
            } else {
                format!(", reused: {}", reused.len())
            }
        );
        Ok(Self {
            held,
            results,
            plan,
            input,
            work,
            scratch,
            kernels,
            passes,
            timer,
            edit,
            rendered: Vec::new(),
            unused,
        })
    }

    /// Takes the pipeline apart for `plan`, a new plan over the same input, keeping what
    /// still holds (decision 28): every compiled kernel, and each cache point of the new
    /// plan that an old one holds with the same key, in a buffer of the size the new
    /// plan makes for it, the edit cache's too, and the scratch buffers of each operation
    /// whose whole-image result an old one holds with the same key, each the size the
    /// new plan makes (decision 15), or of its full-resolution part's result alone, those
    /// that part binds (decision 15). Keys chain everything upstream, so a kept buffer
    /// holds what the new plan would write there. Every other working and scratch buffer
    /// moves to the same place in the new plan when it is the size the new plan makes
    /// there, for its memory only; the rest are freed here, before the new pipeline makes
    /// its own.
    fn keep(self, plan: &Plan) -> Kept {
        let Self {
            plan: old,
            input,
            work,
            scratch,
            kernels,
            passes,
            timer,
            held,
            results,
            edit,
            rendered: _,
            // A kept buffer counts as used, so it is never touched.
            unused: _,
        } = self;
        let mut work: Vec<Option<GpuImage>> = work.into_iter().map(Some).collect();
        let mut kept: Vec<Option<GpuImage>> = plan.work.iter().map(|_| None).collect();
        let mut kept_held = vec![None; plan.cached.len()];
        // The same photo, as `Plan::same_shape` asks: the pass header carries its size.
        let same_image = (old.image.width, old.image.height, &old.image.camera)
            == (plan.image.width, plan.image.height, &plan.image.camera);
        for (kept_key, &op) in kept_held.iter_mut().zip(&plan.cached) {
            let new = &plan.operations[op];
            let (true, Buffer::Work(to)) = (same_image, new.output) else {
                continue;
            };
            let from = old
                .cached
                .iter()
                .zip(&held)
                .find(|&(_, &key)| key == Some(new.key))
                .map(|(&c, _)| old.operations[c].output);
            let Some(Buffer::Work(from)) = from else {
                continue;
            };
            if let Some(image) = work[from].take_if(|image| image.sized_for(&plan.work[to])) {
                kept[to] = Some(image);
                *kept_key = Some(new.key);
            }
        }
        // The edit cache, at the same size, still holding the input of the operation
        // whose input in the new plan has the key it held.
        let edit = edit
            .filter(|_| same_image)
            .zip(plan.edit_cache)
            .filter(|(edit, desc)| edit.buffer.sized_for(desc))
            .map(|(mut edit, desc)| {
                edit.buffer.set_desc(desc);
                edit.held = edit.held.and_then(|(_, key)| {
                    (1..plan.operations.len())
                        .find(|&op| plan.operations[op - 1].key == key && plan.edit_cache_takes(op))
                        .map(|op| (op, key))
                });
                edit
            });
        // Whole-image results move by their keys as cache points do: every scratch buffer
        // of the operation whose result has the key, or, failing that, the scratch buffers
        // its full-resolution part binds, where an old one holds that part's result, as
        // after a resize (decision 15); the first of the two whose buffers are each the
        // size the new plan makes for them. A viewport plane left larger by a smaller
        // viewport taken in place keeps the whole from moving, never the part.
        let mut scratch: Vec<Option<wgpu::Buffer>> = scratch.into_iter().map(Some).collect();
        let mut kept_scratch: Vec<Option<wgpu::Buffer>> =
            plan.scratch.iter().map(|_| None).collect();
        let mut kept_results = vec![Results::default(); plan.operations.len()];
        for (kept_keys, new) in kept_results.iter_mut().zip(&plan.operations) {
            let Some(whole) = new.whole_image.filter(|_| same_image) else {
                continue;
            };
            let holding = |key: u64, of: fn(&Results) -> Option<u64>| {
                old.operations
                    .iter()
                    .zip(&results)
                    .find(|(_, held)| of(held) == Some(key))
            };
            let part = whole.full_resolution;
            let all = holding(whole.key, |held| held.whole).map(|(from, held)| {
                let keys = Results {
                    whole: Some(whole.key),
                    full_resolution: part
                        .map(|part| part.key)
                        .filter(|&key| held.full_resolution == Some(key)),
                };
                (from, (0..new.scratch.len()).collect::<Vec<_>>(), keys)
            });
            let part = part.and_then(|part| {
                let (from, _) = holding(part.key, |held| held.full_resolution)?;
                let keys = Results {
                    whole: None,
                    full_resolution: Some(part.key),
                };
                Some((from, bound_scratch(&new.passes[..part.passes]), keys))
            });
            let fits = |from: &PlannedOperation, buffers: &[usize]| {
                from.scratch.len() == new.scratch.len()
                    && buffers.iter().all(|&index| {
                        scratch[from.scratch[index]].as_ref().is_some_and(|buffer| {
                            buffer.size() == scratch_size(plan.scratch[new.scratch[index]])
                        })
                    })
            };
            let Some((from, buffers, keys)) = [all, part]
                .into_iter()
                .flatten()
                .find(|(from, buffers, _)| fits(from, buffers))
            else {
                continue;
            };
            for &index in &buffers {
                kept_scratch[new.scratch[index]] = scratch[from.scratch[index]].take();
            }
            *kept_keys = keys;
        }
        let points: Vec<&str> = plan
            .cached
            .iter()
            .zip(&kept_held)
            .filter(|(_, key)| key.is_some())
            .map(|(&op, _)| plan.operations[op].id)
            .collect();
        tracing::debug!(
            target: "pipe",
            "rebuild: plan changed, kept {}, cached: {}{}",
            count(kernels.compiled.len(), "kernel", "kernels"),
            if points.is_empty() {
                "none".to_owned()
            } else {
                points.join(", ")
            },
            edit.as_ref()
                .and_then(|edit| edit.held)
                .map_or(String::new(), |(op, _)| format!(
                    ", edit cache: {}'s input",
                    plan.operations[op].id
                ))
        );
        let whole_image: Vec<String> = plan
            .operations
            .iter()
            .zip(&kept_results)
            .filter_map(|(op, keys)| match keys {
                Results { whole: Some(_), .. } => Some(op.id.to_owned()),
                Results {
                    full_resolution: Some(_),
                    ..
                } => Some(format!("{} at full resolution", op.id)),
                _ => None,
            })
            .collect();
        for line in pack("rebuild: kept whole-image results:", &whole_image) {
            tracing::debug!(target: "pipe", "{line}");
        }
        // Every other buffer the same size at the same place, for its memory alone. A
        // render starts after a cache point, or at the operation whose input the edit
        // cache copies back, and writes every other working buffer before it reads it;
        // scratch holds nothing from one render to the next but a whole-image result,
        // which runs again unless it moved by its key above. So what these hold is never
        // read, and the first render after a rebuild does not pay for wgpu clearing new
        // buffers at their first use.
        for (index, (slot, desc)) in kept.iter_mut().zip(&plan.work).enumerate() {
            if slot.is_none() {
                *slot = work
                    .get_mut(index)
                    .and_then(|old| old.take_if(|image| image.sized_for(desc)));
            }
        }
        for (index, (slot, &bytes)) in kept_scratch.iter_mut().zip(&plan.scratch).enumerate() {
            if slot.is_none() {
                let made = scratch_size(bytes);
                *slot = scratch
                    .get_mut(index)
                    .and_then(|old| old.take_if(|buffer| buffer.size() == made));
            }
        }
        // The old buffers, freed before the new pipeline makes its own.
        drop((work, scratch, passes, timer, old));
        Kept {
            input,
            kernels,
            work: kept,
            scratch: kept_scratch,
            held: kept_held,
            results: kept_results,
            edit,
        }
    }

    /// Takes a new plan for the same input. When it runs the same passes and its images
    /// fit the buffers this pipeline has (a slider moved, or the next tile of an export),
    /// the changed uniforms and tables are written in place, a dispatch whose windows
    /// moved gets a new bind group, and the cache points stay, so the next render
    /// restarts from the newest one the change leaves valid. Otherwise (an operation
    /// joined or left, or the viewport grew) the pipeline is rebuilt over the same input,
    /// keeping its compiled kernels and every cache point whose key and size still match,
    /// so the next render restarts there, and every other buffer the new plan makes at
    /// the same size and place; the rest are freed first (decision 28).
    pub async fn update(self, gpu: &GpuContext, plan: Plan) -> Result<Self, ComputeError> {
        self.update_holding(gpu, plan, &[]).await
    }

    /// [`Self::update`] for a pipeline built [`Self::holding`] `held`, which a rebuild
    /// binds again.
    pub(crate) async fn update_holding(
        mut self,
        gpu: &GpuContext,
        plan: Plan,
        held: &Held,
    ) -> Result<Self, ComputeError> {
        if !self.accepts(&plan) {
            let mut kept = self.keep(&plan);
            kept.hold(&plan, held);
            return Self::build(gpu, plan, kept).await;
        }
        // The same shape keeps the same scratch buffers, the held ones among them.
        debug_assert!(
            held.iter()
                .all(|(id, buffers)| self.scratch_of(id) == *buffers)
        );
        let Self {
            plan: old,
            input,
            work,
            scratch,
            kernels,
            passes,
            ..
        } = &mut self;
        let image = |buffer: Buffer| match buffer {
            Buffer::Input => &*input,
            Buffer::Work(index) => &work[index],
        };
        let new = plan
            .operations
            .iter()
            .flat_map(|op| op.passes.iter().zip(&op.dispatches).map(move |p| (op, p)));
        let before = old
            .operations
            .iter()
            .flat_map(|op| op.passes.iter().zip(&op.dispatches).map(move |p| (op, p)));
        for ((compiled, (before_op, (before, before_dispatches))), (op, (after, dispatches))) in
            passes.iter_mut().zip(before).zip(new)
        {
            for ((buffer, table), previous) in compiled
                .tables
                .iter()
                .zip(&after.tables)
                .zip(&before.tables)
            {
                if table != previous {
                    gpu.queue.write_buffer(buffer, 0, table);
                }
            }
            let resources = Resources {
                input: image(op.input).buffer(),
                output: image(op.output).buffer(),
                full_resolution: op.full_resolution.map(|buffer| image(buffer).buffer()),
                tables: &compiled.tables,
                scratch: op.scratch.iter().map(|&i| &scratch[i]).collect(),
            };
            for ((target, previous), dispatch) in compiled
                .dispatches
                .iter_mut()
                .zip(before_dispatches)
                .zip(dispatches)
            {
                let block = uniform_block(op, compiled.index, dispatch);
                if block != uniform_block(before_op, compiled.index, previous) {
                    gpu.queue.write_buffer(&target.uniform, 0, &block);
                }
                if (dispatch.input, dispatch.output) != (previous.input, previous.output) {
                    target.bind_group = bind_group(
                        gpu,
                        &kernels.compiled[compiled.kernel],
                        compiled.label,
                        after,
                        dispatch,
                        &target.uniform,
                        &resources,
                    );
                }
                target.workgroups = workgroups(op, after, dispatch);
            }
        }
        for (image, desc) in self.work.iter_mut().zip(&plan.work_used) {
            image.set_desc(*desc);
        }
        if let (Some(edit), Some(desc)) = (&mut self.edit, plan.edit_cache) {
            edit.buffer.set_desc(desc);
        }
        self.plan = plan;
        Ok(self)
    }

    /// Whether a new plan runs the same passes as this pipeline's, over images and
    /// tables its buffers can hold: [`Self::update`] takes such a plan in place, and
    /// rebuilds the pipeline for any other.
    pub fn accepts(&self, plan: &Plan) -> bool {
        let tables_fit = self
            .passes
            .iter()
            .zip(plan.passes())
            .all(|(compiled, (_, pass))| {
                compiled
                    .tables
                    .iter()
                    .zip(&pass.tables)
                    .all(|(buffer, table)| table.len() as u64 <= buffer.size())
            });
        self.plan.same_shape(plan)
            && self
                .work
                .iter()
                .zip(&plan.work_used)
                .all(|(image, desc)| image.holds(desc))
            && self
                .scratch
                .iter()
                .zip(&plan.scratch_used)
                .all(|(buffer, &bytes)| bytes <= buffer.size())
            && tables_fit
            && match (&self.edit, plan.edit_cache) {
                (Some(edit), Some(desc)) => edit.buffer.holds(&desc),
                (None, None) => true,
                _ => false,
            }
    }

    /// Runs the passes from the newest valid cache point to the end, in one submission,
    /// and waits for the GPU, but for the whole-image passes whose result their
    /// operation's scratch still holds (decision 15). On the way it keeps the cache
    /// points it passes, and the input of the operation the last change began at.
    pub fn render(&mut self, gpu: &GpuContext) -> Result<Timings, ComputeError> {
        self.render_then(gpu, |_| {})
    }

    /// [`Self::render`], recording `then` after the passes in the same submission: for
    /// copying [`Self::output`] without a submission, and a turn at the submit gate, of
    /// its own.
    pub(crate) fn render_then(
        &mut self,
        gpu: &GpuContext,
        then: impl FnOnce(&mut wgpu::CommandEncoder),
    ) -> Result<Timings, ComputeError> {
        let start = Instant::now();
        let submitted = self.submit(gpu, then, true);
        gpu.wait()?;
        self.settle(&submitted);
        let Submitted {
            first,
            runs,
            names,
            copied,
            ..
        } = submitted;
        let ticks = self.timer.as_ref().zip(copied.as_ref());
        let timings = Timings::read(gpu, ticks, &names, start)?;
        self.log_render(first, &runs, &names, &timings);
        Ok(timings)
    }

    /// [`Self::render_then`] without waiting for the GPU, and so untimed: the passes and
    /// `then` in one submission, whose work the caller waits for as it chooses, as a job's
    /// GPU step does (decision 57). The cache points and the scratch's results are taken
    /// as the submission leaves them, which every later submission on the queue sees.
    pub(crate) fn submit_then(
        &mut self,
        gpu: &GpuContext,
        then: impl FnOnce(&mut wgpu::CommandEncoder),
    ) {
        let submitted = self.submit(gpu, then, false);
        self.settle(&submitted);
        let from = self
            .plan
            .operations
            .get(submitted.first)
            .map_or(String::new(), |op| format!(" from {}", op.id));
        let passes = count(submitted.names.len(), "pass", "passes");
        tracing::trace!(target: "perf", "submitted {passes}{from}");
    }

    /// Records the passes from the newest valid cache point to the end, the timestamps
    /// where `timed` and the device has them, then `then`, and submits them.
    fn submit(
        &mut self,
        gpu: &GpuContext,
        then: impl FnOnce(&mut wgpu::CommandEncoder),
        timed: bool,
    ) -> Submitted {
        let keys: Vec<u64> = self.plan.operations.iter().map(|op| op.key).collect();
        let mut first = self.newest_cache_point(&keys);
        // The edited operation: the first whose key changed since the last render.
        let edited = (self.rendered.len() == keys.len())
            .then(|| keys.iter().zip(&self.rendered).position(|(a, b)| a != b))
            .flatten();
        let mut restore = None;
        let mut capture = None;
        if let Some(edit) = &self.edit {
            if let Some((op, key)) = edit.held
                && op > first
                && keys[op - 1] == key
            {
                (first, restore) = (op, Some(op));
            }
            if let Some(op) = edited
                && op >= first
                && restore != Some(op)
                && self.plan.edit_cache_takes(op)
            {
                capture = Some(op);
            }
        }
        let runs = self.runs_from(first);

        let image = |buffer: Buffer| match buffer {
            Buffer::Input => &self.input,
            Buffer::Work(index) => &self.work[index],
        };
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        let mut names = Vec::new();
        let mut previous = None;
        let run = self.passes.iter().zip(&runs).filter(|&(_, &runs)| runs);
        for (pass, _) in run {
            if previous != Some(pass.op) {
                previous = Some(pass.op);
                let input = image(self.plan.operations[pass.op].input).buffer();
                let edit = self.edit.as_ref().map(|edit| edit.buffer.buffer());
                if let (Some(edit), true) = (edit, restore == Some(pass.op)) {
                    encoder.copy_buffer_to_buffer(edit, 0, input, 0, input.size());
                }
                if let (Some(edit), true) = (edit, capture == Some(pass.op)) {
                    encoder.copy_buffer_to_buffer(input, 0, edit, 0, input.size());
                }
            }
            let slot = names.len() as u32;
            let timer = self.timer.as_ref().filter(|_| timed);
            let mut compute = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(pass.label),
                timestamp_writes: timer.map(|timer| timer.writes(slot)),
            });
            compute.set_pipeline(self.kernels.compiled[pass.kernel].pipeline());
            // A pass in row strips dispatches once per strip, in the same compute pass.
            for dispatch in &pass.dispatches {
                compute.set_bind_group(0, &dispatch.bind_group, &[]);
                compute.dispatch_workgroups(dispatch.workgroups.0, dispatch.workgroups.1, 1);
            }
            drop(compute);
            names.push(pass.label);
        }
        let copied = self
            .timer
            .as_ref()
            .filter(|_| timed)
            .map(|timer| timer.copy_out(gpu, &mut encoder, names.len() as u32));
        then(&mut encoder);
        gpu.submit([encoder.finish()]);
        Submitted {
            keys,
            first,
            capture,
            runs,
            names,
            copied,
        }
    }

    /// The operation a render begins at: the one after the newest cache point still
    /// holding what the render would write there, given each operation's `keys`.
    fn newest_cache_point(&self, keys: &[u64]) -> usize {
        self.plan
            .cached
            .iter()
            .zip(&self.held)
            .filter(|&(&op, held)| *held == Some(keys[op]))
            .map(|(&op, _)| op + 1)
            .max()
            .unwrap_or(0)
    }

    /// Whether a render from operation `first` runs each compiled pass: every one from
    /// `first` on, but an operation's whole-image passes while its scratch holds their
    /// result (decision 15), or their full-resolution part while it holds that part's
    /// (decision 15).
    fn runs_from(&self, first: usize) -> Vec<bool> {
        self.passes
            .iter()
            .map(|pass| {
                let whole = self.plan.operations[pass.op].whole_image;
                pass.op >= first && !self.results[pass.op].skip(whole, pass.index)
            })
            .collect()
    }

    /// [`Self::render`] in parts, for a job that submits them one a step: each dispatch of
    /// each pass the render runs is a part, a pass whole or one of its row strips (decision
    /// 14), so a render planned in strips of a size takes steps of that size, but for the
    /// passes that take no strips. The parts run in the order one submission records them,
    /// so they compute what it does. Before the first dispatch that binds a work or scratch
    /// buffer the pipeline made and no render has used, a part clears its first bytes
    /// ([`Part::Touch`]); a buffer no render has used holds zeros, as wgpu clears it, so
    /// the clear changes nothing. Untimed, and only for a pipe with no edit cache, an
    /// export's.
    pub(crate) fn parts(&mut self) -> Parts {
        assert!(self.edit.is_none(), "a render in parts keeps no edit cache");
        let keys: Vec<u64> = self.plan.operations.iter().map(|op| op.key).collect();
        let first = self.newest_cache_point(&keys);
        let runs = self.runs_from(first);
        let run = || {
            self.passes
                .iter()
                .enumerate()
                .zip(&runs)
                .filter(|(_, r)| **r)
        };
        let mut unused = std::mem::take(&mut self.unused);
        let mut order = Vec::new();
        for ((index, pass), _) in run() {
            let op = &self.plan.operations[pass.op];
            let work = |buffer: Buffer| match buffer {
                Buffer::Work(index) => Some(Touched::Work(index)),
                Buffer::Input => None,
            };
            let bindings = &op.passes[pass.index].bindings;
            for binding in bindings {
                let touched = match binding.resource {
                    Resource::Input => work(op.input),
                    Resource::Output => work(op.output),
                    Resource::Scratch(i) => Some(Touched::Scratch(op.scratch[i])),
                    _ => None,
                };
                let flag = match touched {
                    Some(Touched::Work(i)) => unused.work.get_mut(i),
                    Some(Touched::Scratch(i)) => unused.scratch.get_mut(i),
                    None => None,
                };
                if let (Some(touched), Some(flag)) = (touched, flag)
                    && std::mem::take(flag)
                {
                    order.push(Part::Touch(touched));
                }
            }
            order.extend((0..pass.dispatches.len()).map(|d| Part::Dispatch(index, d)));
        }
        let names = run().map(|((_, pass), _)| pass.label).collect();
        let submitted = Submitted {
            keys,
            first,
            capture: None,
            runs,
            names,
            copied: None,
        };
        let parts = Parts { submitted, order };
        if parts.order.is_empty() {
            self.settle(&parts.submitted);
        }
        parts
    }

    /// Submits the part `part` of `parts`, one dispatch or one touch in a submission of its
    /// own, without waiting for the GPU. After the last, the cache points and the
    /// scratch's results are taken as the parts leave them, as [`Self::submit_then`] takes
    /// them.
    pub(crate) fn submit_part(&mut self, gpu: &GpuContext, parts: &Parts, part: usize) {
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        let label = match parts.order[part] {
            Part::Dispatch(index, dispatch) => {
                let pass = &self.passes[index];
                let dispatch = &pass.dispatches[dispatch];
                let mut compute = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some(pass.label),
                    timestamp_writes: None,
                });
                compute.set_pipeline(self.kernels.compiled[pass.kernel].pipeline());
                compute.set_bind_group(0, &dispatch.bind_group, &[]);
                compute.dispatch_workgroups(dispatch.workgroups.0, dispatch.workgroups.1, 1);
                pass.label
            }
            Part::Touch(touched) => {
                let (buffer, label) = match touched {
                    Touched::Work(index) => (self.work[index].buffer(), "touch work"),
                    Touched::Scratch(index) => (&self.scratch[index], "touch scratch"),
                };
                // A word at least, as a clear's size is a multiple of 4.
                let bytes = (buffer.size() / 4 * 4).min(TOUCH);
                encoder.clear_buffer(buffer, 0, Some(bytes));
                label
            }
        };
        gpu.submit([encoder.finish()]);
        tracing::trace!(target: "perf", "submitted part {} of {}: {label}", part + 1, parts.len());
        if part + 1 == parts.len() {
            self.settle(&parts.submitted);
        }
    }

    /// Takes the cache points, the edit cache and the scratch's results as `submitted`
    /// leaves them.
    fn settle(&mut self, submitted: &Submitted) {
        let Submitted {
            keys,
            first,
            capture,
            ..
        } = submitted;
        let first = *first;
        for (held, &op) in self.held.iter_mut().zip(&self.plan.cached) {
            if op >= first {
                *held = Some(keys[op]);
            }
        }
        if let (Some(edit), Some(op)) = (&mut self.edit, *capture) {
            edit.held = Some((op, keys[op - 1]));
        }
        // What ran from `first` on, or was skipped because the scratch held it already.
        for (result, op) in self
            .results
            .iter_mut()
            .zip(&self.plan.operations)
            .skip(first)
        {
            *result = Results::of(op.whole_image);
        }
        self.rendered = keys.clone();
        self.unused = Unused::default();
    }

    /// The perf domain's lines for a render from operation `first` that ran the passes
    /// `runs` marks, `names`, in `timings`.
    fn log_render(&self, first: usize, runs: &[bool], names: &[&str], timings: &Timings) {
        let from = self
            .plan
            .operations
            .get(first)
            .map_or(String::new(), |op| format!(" from {}", op.id));
        // The operations whose whole-image results it kept rather than ran.
        let mut kept: Vec<String> = self
            .passes
            .iter()
            .zip(runs)
            .filter(|&(pass, &runs)| pass.op >= first && !runs)
            .map(|(pass, _)| self.plan.operations[pass.op].id.to_owned())
            .collect();
        kept.dedup();
        let gpu_ms: f64 = timings.passes.iter().map(|(_, ms)| ms).sum();
        let mut rendered = format!(
            "rendered {}{from} in {:.1} ms{}",
            count(names.len(), "pass", "passes"),
            timings.wall_ms,
            if gpu_ms.is_nan() {
                String::new()
            } else {
                format!(", GPU: {gpu_ms:.1} ms")
            }
        );
        // Named when they fit the line (decision 44), otherwise counted, and named at
        // trace level.
        let named = format!(", kept: {}", kept.join(", "));
        let fits = rendered.chars().count() + named.chars().count() <= MESSAGE;
        match (kept.len(), fits) {
            (0, _) => {}
            (_, true) => rendered.push_str(&named),
            (n, false) => rendered.push_str(&format!(", kept: {}", count(n, "op", "ops"))),
        }
        // An export's tiles at trace level, since the export sums them up.
        if matches!(self.plan.pipe, Pipe::Tile { .. }) {
            tracing::trace!(target: "perf", "{rendered}");
        } else {
            tracing::debug!(target: "perf", "{rendered}");
        }
        if !kept.is_empty() && !fits {
            for line in pack("kept:", &kept) {
                tracing::trace!(target: "perf", "{line}");
            }
        }
        if !gpu_ms.is_nan() && tracing::enabled!(target: "perf", tracing::Level::TRACE) {
            let operations: Vec<&str> = self
                .passes
                .iter()
                .zip(runs)
                .filter(|&(_, &runs)| runs)
                .map(|(pass, _)| self.plan.operations[pass.op].id)
                .collect();
            for line in pass_lines(&timings.passes, &operations) {
                tracing::trace!(target: "perf", "{line}");
            }
        }
    }

    pub fn plan(&self) -> &Plan {
        &self.plan
    }

    /// How many kernel entry points the pipeline has compiled, those kept from the plans
    /// before it over the same input included.
    pub fn kernels(&self) -> usize {
        self.kernels.compiled.len()
    }

    /// Runs a derivative entry point of the plan's last operation, `<kernel>_backward` or
    /// `<kernel>_forward_diff`, over the image its output pass writes, the pass that binds
    /// [`Resource::Output`], with that pass's bindings but its output, and
    /// `derivative_in` and `derivative_out` at bindings 8 and 9, over the scratch buffers
    /// the last render left: an operation's derivatives are those of the pass that writes
    /// its output (decision 15). Waits for the GPU. An error for an empty plan, an
    /// operation with no output pass, or an output pass in row strips.
    pub async fn derivative(
        &mut self,
        gpu: &GpuContext,
        derivative: Derivative,
        derivative_in: &GpuImage,
        derivative_out: &GpuImage,
    ) -> Result<(), ComputeError> {
        let refuse = |operation, reason| ComputeError::Derivative { operation, reason };
        let index = self
            .plan
            .operations
            .len()
            .checked_sub(1)
            .ok_or(refuse("the plan", "it runs no operation"))?;
        let op = &self.plan.operations[index];
        let position = op
            .passes
            .iter()
            .rposition(|pass| {
                pass.bindings
                    .iter()
                    .any(|binding| binding.resource == Resource::Output)
            })
            .ok_or(refuse(op.id, "no pass writes its output"))?;
        let pass = &op.passes[position];
        let [dispatch] = op.dispatches[position][..] else {
            return Err(refuse(op.id, "its output pass runs in row strips"));
        };
        let suffix = match derivative {
            Derivative::Backward => "backward",
            Derivative::ForwardDiff => "forward_diff",
        };
        let entry_point = format!("{}_{suffix}", op.kernel.name);
        let kernel = self
            .kernels
            .get(gpu, op.kernel, &entry_point, op.id)
            .await?;
        let kernel = &self.kernels.compiled[kernel];
        let compiled = self
            .passes
            .iter()
            .filter(|compiled| compiled.op == index)
            .nth(position)
            .expect("a compiled pass per pass");
        let uniform = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(op.id),
                contents: &uniform_block(op, position, &dispatch),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let input = match op.input {
            Buffer::Input => &self.input,
            Buffer::Work(index) => &self.work[index],
        };
        let mut entries: Vec<wgpu::BindGroupEntry> = pass
            .bindings
            .iter()
            .filter_map(|binding| {
                let resource = match binding.resource {
                    Resource::Input => input.buffer().as_entire_binding(),
                    Resource::Output => return None,
                    Resource::Uniforms => uniform.as_entire_binding(),
                    Resource::Table(i) => compiled.tables[i].as_entire_binding(),
                    Resource::Scratch(i) => self.scratch[op.scratch[i]].as_entire_binding(),
                    // The derivative entry points read what the forward render left.
                    Resource::FullResolution => return None,
                };
                Some(wgpu::BindGroupEntry {
                    binding: binding.binding,
                    resource,
                })
            })
            .collect();
        for (binding, image) in [
            (DERIVATIVE_IN_BINDING, derivative_in),
            (DERIVATIVE_OUT_BINDING, derivative_out),
        ] {
            entries.push(wgpu::BindGroupEntry {
                binding,
                resource: image.buffer().as_entire_binding(),
            });
        }
        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(op.id),
            layout: &kernel.pipeline().get_bind_group_layout(0),
            entries: &entries,
        });
        let (x, y) = workgroups(op, pass, &dispatch);
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        {
            let mut compute = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(op.id),
                timestamp_writes: None,
            });
            compute.set_pipeline(kernel.pipeline());
            compute.set_bind_group(0, &bind_group, &[]);
            compute.dispatch_workgroups(x, y, 1);
        }
        gpu.submit([encoder.finish()]);
        gpu.wait()
    }

    /// The result of the last render, described as the plan's last operation made it: a
    /// working buffer is shared, and its own description is its first image's.
    pub fn output(&self) -> GpuImage {
        let (buffer, desc) = self.plan.output();
        let mut image = match buffer {
            Buffer::Input => self.input.clone(),
            Buffer::Work(index) => self.work[index].clone(),
        };
        image.set_desc(desc);
        image
    }
}

#[cfg(test)]
mod tests {
    use apertrust_plugin::{
        Binding, Domain, Geometry, Kind, Manifest, Param, PassKind, Prepared, Schema, Scratch,
        Slot, Value,
    };

    use super::*;

    /// A pass is named with its operation where its entry point names it, and the same
    /// pass of each viewport tile is added up, so the lines add up by operation.
    #[test]
    fn pass_lines_name_each_pass_with_its_operation() {
        let passes = [
            ("rawprepare", 0.9),
            ("rcd_step_1", 2.3),
            ("rcd_step_2", 1.0),
            ("downscale", 0.5),
            ("rcd_step_1", 2.0),
            ("rcd_step_2", 1.3),
            ("downscale", 0.8),
            ("agx", 3.7),
        ];
        let operations = [
            "rawprepare",
            "demosaic",
            "demosaic",
            "downscale",
            "demosaic",
            "demosaic",
            "downscale",
            "agx",
        ];
        assert_eq!(
            pass_lines(&passes, &operations),
            [
                "pass rawprepare: 0.9 ms",
                "pass demosaic rcd_step_1: 4.3 ms (2×)",
                "pass demosaic rcd_step_2: 2.3 ms (2×)",
                "pass downscale: 1.3 ms (2×)",
                "pass agx: 3.7 ms",
            ]
        );
    }

    /// Plans as [`super::plan`] does on a GPU that binds anything.
    fn plan(
        instances: &[Instance],
        image: &ImageContext,
        input: ImageDesc,
        pipe: Pipe,
    ) -> Result<Plan, PlanError> {
        super::plan(instances, image, input, pipe, &Limits::NONE)
    }

    /// An operation with a place, one parameter, and whatever output, passes and scratch
    /// buffers a test gives it.
    struct Fake {
        manifest: &'static Manifest,
        output: Option<ImageDesc>,
        /// Writes RGBA of the size of the pipe's region, as demosaic does.
        demosaics: bool,
        passes: Vec<Pass>,
        scratch: Vec<Scratch>,
        image_default: Option<f32>,
        /// What it answers to `tiles_whole_image_passes`.
        tiles: bool,
        /// What it answers to `whole_image_at_full_resolution`: where the viewport gives
        /// it the full-resolution image, it estimates from it as haze removal does.
        estimates: bool,
    }

    static KERNEL: Kernel = Kernel {
        name: "fake",
        wgsl: "",
        derivatives: false,
    };

    static SCHEMA: Schema = Schema {
        version: 1,
        params: &[Param {
            name: "amount",
            kind: Kind::Float {
                default: 0.0,
                min: -1.0,
                max: 1.0,
            },
        }],
    };

    impl Operation for Fake {
        fn manifest(&self) -> &'static Manifest {
            self.manifest
        }

        fn schema(&self) -> &'static Schema {
            &SCHEMA
        }

        fn kernel(&self) -> &'static Kernel {
            &KERNEL
        }

        fn defaults(&self, _: &ImageContext) -> ParamValues {
            let mut values = SCHEMA.defaults();
            if let Some(amount) = self.image_default {
                values.set("amount", Value::Float(amount));
            }
            values
        }

        fn prepare(
            &self,
            params: &ParamValues,
            image: &ImageContext,
            input: &ImageDesc,
        ) -> Result<Prepared, NotPorted> {
            let amount = params.float("amount").to_le_bytes().to_vec();
            let mut prepared = Prepared::per_pixel(input, amount, Vec::new());
            if let Some(output) = self.output {
                prepared.output = output;
            }
            if self.demosaics {
                // The region of a declared image at the sensor's corner.
                let [x, y, width, height] = image.region;
                prepared.output = ImageDesc {
                    width,
                    height,
                    format: Format::Rgba,
                    sensor_origin: [x, y],
                    ..*input
                };
            }
            if !self.passes.is_empty() {
                prepared.passes = self.passes.clone();
            }
            prepared.scratch = self.scratch.clone();
            if let (true, Some(full)) = (self.estimates, image.full_resolution) {
                (prepared.passes, prepared.scratch) = estimate(&full, input);
            }
            Ok(prepared)
        }

        fn tiles_whole_image_passes(&self, _: &ParamValues) -> bool {
            self.tiles
        }

        fn whole_image_at_full_resolution(&self, _: &ParamValues) -> bool {
            self.estimates
        }
    }

    /// Haze removal's shape in the viewport (decision 15): an estimate from the
    /// full-resolution image `full` into scratch 0, combined into scratch 1 by one
    /// workgroup, filtered at full resolution, then downscaled into scratch 2 at the
    /// viewport's size, `input`'s, and `main` reading that and the statistic.
    fn estimate(full: &ImageDesc, input: &ImageDesc) -> (Vec<Pass>, Vec<Scratch>) {
        let pass = |entry_point, size, bindings: &[(u32, Resource)]| Pass {
            entry_point,
            kind: PassKind::Reduction,
            bindings: [(2, Resource::Uniforms)]
                .iter()
                .chain(bindings)
                .map(|&(binding, resource)| Binding::new(binding, resource))
                .collect(),
            size: Some(size),
            ..Pass::per_pixel(Vec::new(), Vec::new())
        };
        let full_size = [full.width, full.height];
        let passes = vec![
            pass(
                "estimate",
                full_size,
                &[(16, Resource::FullResolution), (10, Resource::Scratch(0))],
            ),
            pass(
                "combine",
                [16, 16],
                &[(10, Resource::Scratch(0)), (11, Resource::Scratch(1))],
            ),
            pass(
                "filter",
                full_size,
                &[(16, Resource::FullResolution), (10, Resource::Scratch(0))],
            ),
            pass(
                "plane_downscale",
                [input.width, input.height],
                &[(10, Resource::Scratch(0)), (17, Resource::Scratch(2))],
            ),
            Pass {
                bindings: [
                    (0, Resource::Input),
                    (1, Resource::Output),
                    (2, Resource::Uniforms),
                    (10, Resource::Scratch(2)),
                    (11, Resource::Scratch(1)),
                ]
                .iter()
                .map(|&(binding, resource)| Binding::new(binding, resource))
                .collect(),
                ..Pass::per_pixel(Vec::new(), Vec::new())
            },
        ];
        let scratch = vec![
            Scratch::per_pixel(full, 4),
            Scratch { bytes: 64 },
            Scratch::per_pixel(input, 4),
        ];
        (passes, scratch)
    }

    fn fake(id: &'static str, stage: Stage, order: f32) -> Fake {
        Fake {
            // Leaked, so it can be 'static as the trait asks; tests can afford it.
            manifest: Box::leak(Box::new(Manifest {
                id,
                version: 1,
                slot: Slot { stage, order },
                domain: Domain::Scene,
                layers: true,
                differentiable: true,
                radius: 0,
                default_enabled: true,
                provenance: None,
            })),
            output: None,
            demosaics: false,
            passes: Vec::new(),
            scratch: Vec::new(),
            image_default: None,
            tiles: false,
            estimates: false,
        }
    }

    fn instance(operation: &dyn Operation) -> Instance<'_> {
        Instance {
            operation,
            params: ParamValues::new(),
        }
    }

    const RGBA: ImageDesc = ImageDesc::rgba(4, 4);

    fn image() -> ImageContext {
        ImageContext::full(4, 4)
    }

    #[test]
    fn operations_run_in_slot_order_and_alternate_buffers() {
        let tone = fake("tone", Stage::WorkingSpace, 45.5);
        let exposure = fake("exposure", Stage::CameraRgb, 21.0);
        let colour = fake("colour", Stage::WorkingSpace, 28.5);
        let plan = plan(
            &[instance(&tone), instance(&exposure), instance(&colour)],
            &image(),
            RGBA,
            Pipe::Export,
        )
        .unwrap();
        let order: Vec<_> = plan.operations.iter().map(|op| op.id).collect();
        assert_eq!(order, ["exposure", "colour", "tone"]);
        let buffers: Vec<_> = plan
            .operations
            .iter()
            .map(|op| (op.input, op.output))
            .collect();
        assert_eq!(
            buffers,
            [
                (Buffer::Input, Buffer::Work(0)),
                (Buffer::Work(0), Buffer::Work(1)),
                (Buffer::Work(1), Buffer::Work(0)),
            ]
        );
        assert_eq!(plan.output(), (Buffer::Work(0), RGBA));
        assert_eq!(plan.work, [RGBA, RGBA]);
    }

    #[test]
    fn an_empty_plan_leaves_the_input() {
        let plan = plan(&[], &image(), RGBA, Pipe::Export).unwrap();
        assert_eq!(plan.output(), (Buffer::Input, RGBA));
        assert!(plan.work.is_empty());
    }

    #[test]
    fn the_raw_stage_changes_size_and_format_and_gets_its_scratch() {
        let sensor = ImageDesc::new(9, 7, Format::Mosaic16);
        let cropped = ImageDesc {
            sensor_origin: [1, 2],
            ..ImageDesc::new(6, 4, Format::Mosaic)
        };
        // The developed image, the stage every RGBA image lies in (decision 16), at the
        // export's scale of 1 (decision 15).
        let rgb = ImageDesc {
            format: Format::Rgba,
            extent: [6, 4],
            scaled_extent: [6, 4],
            ..cropped
        };
        let mut prepare_raw = fake("rawprepare", Stage::Raw, 1.0);
        prepare_raw.output = Some(cropped);
        let mut temperature = fake("temperature", Stage::Raw, 3.0);
        temperature.output = Some(cropped);
        let mut demosaic = fake("demosaic", Stage::Raw, 8.0);
        demosaic.output = Some(rgb);
        demosaic.scratch = vec![Scratch::per_pixel(&rgb, 4); 2];
        demosaic.passes = vec![
            Pass {
                entry_point: "populate",
                kind: PassKind::Spatial { radius: 1 },
                params: Vec::new(),
                tables: Vec::new(),
                bindings: vec![
                    Binding::new(0, Resource::Input),
                    Binding::new(10, Resource::Scratch(0)),
                    Binding::new(11, Resource::Scratch(1)),
                ],
                size: Some([9, 7]),
                strips: None,
            },
            Pass {
                entry_point: "write",
                kind: PassKind::PerPixel,
                params: Vec::new(),
                tables: Vec::new(),
                bindings: vec![
                    Binding::new(1, Resource::Output),
                    Binding::new(10, Resource::Scratch(0)),
                ],
                size: None,
                strips: None,
            },
        ];
        let exposure = fake("exposure", Stage::CameraRgb, 21.0);
        let plan = plan(
            &[
                instance(&exposure),
                instance(&demosaic),
                instance(&temperature),
                instance(&prepare_raw),
            ],
            &ImageContext::full(6, 4),
            sensor,
            Pipe::Export,
        )
        .unwrap();

        let ops: Vec<_> = plan
            .operations
            .iter()
            .map(|op| (op.id, op.input, op.output))
            .collect();
        assert_eq!(
            ops,
            [
                ("rawprepare", Buffer::Input, Buffer::Work(0)),
                ("temperature", Buffer::Work(0), Buffer::Work(1)),
                ("demosaic", Buffer::Work(1), Buffer::Work(2)),
                ("exposure", Buffer::Work(2), Buffer::Work(3)),
            ]
        );
        assert_eq!(plan.work, [cropped, cropped, rgb, rgb]);
        assert_eq!(plan.operations[2].scratch, [0, 1]);
        assert_eq!(plan.scratch, [6 * 4 * 4, 6 * 4 * 4]);
        let entry_points: Vec<_> = plan.passes().map(|(_, pass)| pass.entry_point).collect();
        assert_eq!(entry_points, ["main", "main", "populate", "write", "main"]);
    }

    #[test]
    fn each_stage_takes_only_its_own_format() {
        let mosaic = ImageDesc {
            format: Format::Mosaic,
            ..RGBA
        };
        let exposure = fake("exposure", Stage::CameraRgb, 21.0);
        assert!(matches!(
            plan(&[instance(&exposure)], &image(), mosaic, Pipe::Export),
            Err(PlanError::Format {
                operation: "exposure",
                ..
            })
        ));
        let temperature = fake("temperature", Stage::Raw, 3.0);
        assert!(matches!(
            plan(&[instance(&temperature)], &image(), RGBA, Pipe::Export),
            Err(PlanError::Format {
                operation: "temperature",
                ..
            })
        ));
    }

    #[test]
    fn a_pass_may_bind_only_what_its_operation_has() {
        let mut broken = fake("broken", Stage::WorkingSpace, 30.0);
        broken.passes = vec![Pass {
            bindings: vec![Binding::new(10, Resource::Scratch(0))],
            ..Pass::per_pixel(Vec::new(), Vec::new())
        }];
        assert!(matches!(
            plan(&[instance(&broken)], &image(), RGBA, Pipe::Export),
            Err(PlanError::Binding {
                operation: "broken",
                resource: Resource::Scratch(0)
            })
        ));
    }

    #[test]
    fn parameters_resolve_over_the_image_defaults() {
        let mut exposure = fake("exposure", Stage::CameraRgb, 21.0);
        exposure.image_default = Some(0.25);
        let planned = plan(&[instance(&exposure)], &image(), RGBA, Pipe::Export).unwrap();
        assert_eq!(
            planned.operations[0].passes[0].params,
            0.25f32.to_le_bytes()
        );

        let mut set = instance(&exposure);
        set.params.set("amount", Value::Float(0.5));
        let planned = plan(&[set], &image(), RGBA, Pipe::Export).unwrap();
        assert_eq!(planned.operations[0].passes[0].params, 0.5f32.to_le_bytes());

        let mut wrong = instance(&exposure);
        wrong.params.set("amount", Value::Float(2.0));
        assert!(matches!(
            plan(&[wrong], &image(), RGBA, Pipe::Export),
            Err(PlanError::Param {
                operation: "exposure",
                ..
            })
        ));
    }

    #[test]
    fn a_reduction_plans_its_statistic_into_scratch_for_the_passes_after_it() {
        let mosaic = ImageDesc::new(6, 4, Format::Mosaic);
        let mut highlights = fake("highlights", Stage::Raw, 4.0);
        highlights.output = Some(mosaic);
        // Partial sums, then the statistic, then the pass that reads it.
        highlights.scratch = vec![Scratch { bytes: 32 }, Scratch { bytes: 16 }];
        let pass = |entry_point, kind, size, bindings: &[(u32, Resource)]| Pass {
            entry_point,
            kind,
            params: Vec::new(),
            tables: Vec::new(),
            bindings: bindings
                .iter()
                .map(|&(binding, resource)| Binding::new(binding, resource))
                .collect(),
            size,
            strips: None,
        };
        highlights.passes = vec![
            pass(
                "partials",
                PassKind::Reduction,
                Some([2, 1]),
                &[(0, Resource::Input), (10, Resource::Scratch(0))],
            ),
            pass(
                "combine",
                PassKind::Reduction,
                Some([1, 1]),
                &[(10, Resource::Scratch(0)), (11, Resource::Scratch(1))],
            ),
            pass(
                "apply",
                PassKind::PerPixel,
                None,
                &[
                    (0, Resource::Input),
                    (1, Resource::Output),
                    (11, Resource::Scratch(1)),
                ],
            ),
        ];
        let plan = plan(
            &[instance(&highlights)],
            &ImageContext::full(6, 4),
            mosaic,
            Pipe::Export,
        )
        .unwrap();
        let passes: Vec<_> = plan
            .passes()
            .map(|(_, pass)| (pass.entry_point, pass.kind))
            .collect();
        assert_eq!(
            passes,
            [
                ("partials", PassKind::Reduction),
                ("combine", PassKind::Reduction),
                ("apply", PassKind::PerPixel),
            ]
        );
        assert_eq!(plan.scratch, [32, 16]);
        assert_eq!(plan.output(), (Buffer::Work(0), mosaic));
    }

    /// rawprepare and demosaic on a 12×8 sensor, then an operation in camera RGB and
    /// one in the working space.
    fn raw_chain() -> [Fake; 4] {
        let mosaic = ImageDesc::new(12, 8, Format::Mosaic);
        let mut prepare_raw = fake("rawprepare", Stage::Raw, 1.0);
        prepare_raw.output = Some(mosaic);
        let mut demosaic = fake("demosaic", Stage::Raw, 8.0);
        demosaic.output = Some(ImageDesc {
            format: Format::Rgba,
            ..mosaic
        });
        [
            prepare_raw,
            demosaic,
            fake("exposure", Stage::CameraRgb, 21.0),
            fake("tone", Stage::WorkingSpace, 45.0),
        ]
    }

    fn plan_raw(chain: &[Fake; 4], pipe: Pipe) -> Plan {
        let instances: Vec<Instance> = chain.iter().map(|op| instance(op)).collect();
        let sensor = ImageDesc::new(12, 8, Format::Mosaic16);
        plan(&instances, &ImageContext::full(12, 8), sensor, pipe).unwrap()
    }

    #[test]
    fn the_viewport_pipe_downscales_once_where_the_raw_stage_ends() {
        let chain = raw_chain();
        let viewport = plan_raw(
            &chain,
            Pipe::Viewport {
                width: 6,
                height: 6,
            },
        );
        let ids: Vec<_> = viewport.operations.iter().map(|op| op.id).collect();
        assert_eq!(
            ids,
            ["rawprepare", "demosaic", "downscale", "exposure", "tone"]
        );
        let small = ImageDesc {
            width: 6,
            height: 4,
            format: Format::Rgba,
            // A downscale of the whole developed image (decision 16), which is its
            // stage at the viewport's scale (decision 15).
            extent: [12, 8],
            scaled_extent: [6, 4],
            ..ImageDesc::new(12, 8, Format::Mosaic)
        };
        assert_eq!(viewport.operations[2].output_desc, small);
        let scales: Vec<_> = viewport.operations.iter().map(|op| op.scale).collect();
        assert_eq!(scales, [1.0, 1.0, 0.5, 0.5, 0.5]);
        // Demosaic's full-resolution image and its downscale are cache points, which no
        // later operation writes over.
        assert_eq!(viewport.cached, [1, 2]);
        let kept = [viewport.operations[1].output, viewport.operations[2].output];
        assert!(
            viewport.operations[3..]
                .iter()
                .all(|op| !kept.contains(&op.output))
        );
        assert_eq!(viewport.edit_cache, Some(small));

        let export = plan_raw(&chain, Pipe::Export);
        assert!(
            export
                .operations
                .iter()
                .all(|op| op.id != "downscale" && op.scale == 1.0)
        );
        assert!(export.cached.is_empty() && export.edit_cache.is_none());
    }

    #[test]
    fn an_image_that_fits_the_viewport_is_not_downscaled() {
        let chain = raw_chain();
        let viewport = plan_raw(
            &chain,
            Pipe::Viewport {
                width: 100,
                height: 100,
            },
        );
        let ids: Vec<_> = viewport.operations.iter().map(|op| op.id).collect();
        assert_eq!(ids, ["rawprepare", "demosaic", "exposure", "tone"]);
        assert_eq!(viewport.cached, [1]);
        assert!(viewport.operations.iter().all(|op| op.scale == 1.0));
        assert_eq!(
            viewport.edit_cache,
            Some(viewport.operations[1].output_desc)
        );
    }

    #[test]
    fn keys_change_from_the_changed_operation_on() {
        let [a, b, c] = [
            fake("a", Stage::CameraRgb, 21.0),
            fake("b", Stage::WorkingSpace, 30.0),
            fake("c", Stage::WorkingSpace, 45.0),
        ];
        let with_b = |amount| {
            let mut changed = instance(&b);
            changed.params.set("amount", Value::Float(amount));
            plan(
                &[instance(&a), changed, instance(&c)],
                &image(),
                RGBA,
                Pipe::Export,
            )
            .unwrap()
        };
        let keys = |plan: &Plan| plan.operations.iter().map(|op| op.key).collect::<Vec<_>>();
        let (before, after) = (with_b(0.25), with_b(0.5));
        assert_eq!(keys(&before), keys(&with_b(0.25)));
        assert_eq!(keys(&before)[0], keys(&after)[0]);
        assert!(keys(&before)[1] != keys(&after)[1] && keys(&before)[2] != keys(&after)[2]);
        // A changed value keeps the structure, so a pipeline only rewrites uniforms.
        assert!(before.same_shape(&after));
    }

    #[test]
    fn a_new_viewport_size_keeps_the_shape_and_another_operation_does_not() {
        let chain = raw_chain();
        let small = plan_raw(
            &chain,
            Pipe::Viewport {
                width: 6,
                height: 6,
            },
        );
        let smaller = plan_raw(
            &chain,
            Pipe::Viewport {
                width: 4,
                height: 4,
            },
        );
        assert!(small.same_shape(&smaller));
        let [prepare_raw, demosaic, exposure, _] = &chain;
        let instances = [
            instance(prepare_raw),
            instance(demosaic),
            instance(exposure),
        ];
        let sensor = ImageDesc::new(12, 8, Format::Mosaic16);
        let without_tone = plan(
            &instances,
            &ImageContext::full(12, 8),
            sensor,
            Pipe::Viewport {
                width: 6,
                height: 6,
            },
        )
        .unwrap();
        assert!(!small.same_shape(&without_tone));
    }

    /// A raw stage whose images are larger than 32 KiB: a crop from a 16-bit mosaic
    /// whose rows end inside words, then an operation adding up blocks of four rows per
    /// invocation into a partial result per workgroup, combining them, and applying them
    /// with a neighbourhood of one.
    fn banded_raw_stage() -> ([Fake; 2], ImageDesc, ImageDesc) {
        let sensor = ImageDesc::new(75, 300, Format::Mosaic16);
        let cropped = ImageDesc {
            sensor_origin: [1, 3],
            ..ImageDesc::new(72, 290, Format::Mosaic)
        };
        let mut crop = fake("rawprepare", Stage::Raw, 1.0);
        crop.output = Some(cropped);
        crop.passes = vec![Pass {
            strips: Some(Strips::PER_PIXEL),
            ..Pass::per_pixel(Vec::new(), Vec::new())
        }];
        let mut reduce = fake("highlights", Stage::Raw, 4.0);
        reduce.output = Some(cropped);
        reduce.scratch = vec![Scratch { bytes: 64 }];
        let pass = |entry_point, kind, size, strips, bindings: &[(u32, Resource)]| Pass {
            entry_point,
            kind,
            params: Vec::new(),
            tables: Vec::new(),
            bindings: bindings
                .iter()
                .map(|&(binding, resource)| Binding::new(binding, resource))
                .collect(),
            size,
            strips,
        };
        reduce.passes = vec![
            pass(
                "sums",
                PassKind::Reduction,
                Some([18, 73]),
                Some(Strips {
                    rows: 4,
                    granularity: 16,
                    halo: 1,
                }),
                &[(0, Resource::Input), (10, Resource::Scratch(0))],
            ),
            pass(
                "combine",
                PassKind::Reduction,
                Some([16, 16]),
                None,
                &[(10, Resource::Scratch(0))],
            ),
            pass(
                "apply",
                PassKind::Spatial { radius: 1 },
                None,
                Some(Strips {
                    halo: 1,
                    ..Strips::PER_PIXEL
                }),
                &[
                    (0, Resource::Input),
                    (1, Resource::Output),
                    (10, Resource::Scratch(0)),
                ],
            ),
        ];
        ([crop, reduce], sensor, cropped)
    }

    const SMALL: Limits = Limits {
        binding: 32 << 10,
        buffer: u64::MAX,
        offset_alignment: 256,
        budget: u64::MAX,
    };

    #[test]
    fn passes_that_bind_too_much_run_in_row_strips_over_aligned_windows() {
        let ([crop, reduce], sensor, cropped) = banded_raw_stage();
        let instances = [instance(&crop), instance(&reduce)];
        let image = ImageContext::full(72, 290);
        let plan = super::plan(&instances, &image, sensor, Pipe::Export, &SMALL).unwrap();
        assert!(plan.fits(&SMALL));
        let row = |image: &ImageDesc| row_bytes(image) as i64;
        for (op, input) in plan.operations.iter().zip([sensor, cropped]) {
            for (pass, dispatches) in op.passes.iter().zip(&op.dispatches) {
                let name = pass.entry_point;
                let Some(strips) = pass.strips else {
                    assert_eq!(dispatches.len(), 1, "{name}");
                    assert!(dispatches[0].input.is_none() && dispatches[0].output.is_none());
                    continue;
                };
                assert!(dispatches.len() > 1, "{name}");
                let height = pass.size.map_or(cropped.height, |size| size[1]);
                let (k, halo) = (i64::from(strips.rows), i64::from(strips.halo));
                let shift = i64::from(cropped.sensor_origin[1] - input.sensor_origin[1]);
                let mut next = 0;
                for (i, dispatch) in dispatches.iter().enumerate() {
                    let [first, rows] = dispatch.rows;
                    assert_eq!(first, next, "{name}: the strips run in order");
                    next += rows;
                    if i + 1 < dispatches.len() {
                        assert_eq!(rows % strips.granularity, 0, "{name}");
                    }
                    let (a, b) = (i64::from(first) * k, i64::from(first + rows) * k);
                    // Each window binds what the strip reads or writes, and no more than a
                    // binding, at an aligned offset.
                    let read = [
                        (a - halo + shift).max(0),
                        (b + halo + shift).min(i64::from(input.height)),
                    ];
                    let written = [a, b.min(i64::from(cropped.height))];
                    for (window, image, rows) in [
                        (dispatch.input, &input, read),
                        (dispatch.output, &cropped, written),
                    ] {
                        let Some(window) = window else { continue };
                        assert_eq!(window.offset % 256, 0, "{name}");
                        assert_eq!(window.size % 4, 0, "{name}");
                        assert!(window.size <= SMALL.binding, "{name}");
                        assert!(window.offset + window.size <= image.buffer_size(), "{name}");
                        assert!(window.offset as i64 <= rows[0] * row(image), "{name}");
                        assert!(
                            (window.offset + window.size) as i64 >= rows[1] * row(image),
                            "{name}"
                        );
                    }
                }
                assert_eq!(next, height, "{name}: the strips cover the pass");
            }
        }
        // Without limits, every pass runs whole.
        let whole = super::plan(&instances, &image, sensor, Pipe::Export, &Limits::NONE).unwrap();
        assert!(
            whole
                .operations
                .iter()
                .flat_map(|op| &op.dispatches)
                .all(|d| { d.len() == 1 && d[0].input.is_none() && d[0].output.is_none() })
        );
        assert!(whole.same_shape(&whole.clone()) && !whole.same_shape(&plan));
    }

    #[test]
    fn a_pass_that_cannot_run_in_strips_leaves_a_plan_that_does_not_fit() {
        let ([crop, mut reduce], sensor, _) = banded_raw_stage();
        reduce.passes[2].strips = None;
        let plan = super::plan(
            &[instance(&crop), instance(&reduce)],
            &ImageContext::full(72, 290),
            sensor,
            Pipe::Export,
            &SMALL,
        )
        .unwrap();
        assert_eq!(plan.operations[1].dispatches[2].len(), 1);
        assert!(!plan.fits(&SMALL));
        assert_eq!(
            plan.oversized_binding(SMALL.binding, sensor.buffer_size()),
            Some(("highlights", "apply", 72 * 290 * 4))
        );
    }

    /// The tiles' parts of the image and of the viewport each cut the whole without
    /// overlapping, and each tile's part of the image is exactly the union of the
    /// footprints of its output pixels, as the downscale computes them.
    fn check_tiles(tiles: &[ViewportTile], source: [u32; 2], output: [u32; 2]) {
        let footprint = |o: u32, axis: usize| {
            let (s, out) = (u64::from(source[axis]), u64::from(output[axis]));
            [
                (u64::from(o) * s / out) as u32,
                (u64::from(o + 1) * s / out) as u32,
            ]
        };
        for (rects, size) in [(0, source), (1, output)] {
            let mut covered = vec![0u8; (size[0] * size[1]) as usize];
            for tile in tiles {
                let [x, y, w, h] = if rects == 0 { tile.source } else { tile.output };
                for row in y..y + h {
                    for col in x..x + w {
                        covered[(row * size[0] + col) as usize] += 1;
                    }
                }
            }
            assert!(
                covered.iter().all(|&n| n == 1),
                "{source:?} into {output:?}"
            );
        }
        for tile in tiles {
            for axis in 0..2 {
                let (o, n) = (tile.output[axis], tile.output[axis + 2]);
                let (s, m) = (tile.source[axis], tile.source[axis + 2]);
                assert!(n > 0 && m > 0);
                for pixel in o..o + n {
                    let [start, end] = footprint(pixel, axis);
                    assert!(start >= s && end <= s + m && end > start, "{tile:?}");
                }
                assert_eq!(footprint(o, axis)[0], s, "{tile:?}");
                assert_eq!(footprint(o + n - 1, axis)[1], s + m, "{tile:?}");
            }
        }
    }

    #[test]
    fn viewport_tiles_fall_on_the_downscales_footprints() {
        for (source, output) in [
            // The 7R V's 9504 × 6336 into 2400 × 1600, a fifth as large.
            ([1901, 1267], [480, 320]),
            ([1000, 700], [286, 200]),
            ([752, 500], [200, 133]),
            ([97, 61], [97, 61]),
            ([5000, 3], [7, 1]),
        ] {
            for edge in [64, 128, 256, 2048, 8192] {
                let tiles = viewport_tiles(source, output, edge);
                let across = source[0].div_ceil(edge).clamp(1, output[0]);
                let down = source[1].div_ceil(edge).clamp(1, output[1]);
                assert_eq!(tiles.len() as u32, across * down);
                check_tiles(&tiles, source, output);
            }
        }
    }

    /// rawprepare, a demosaic that writes its region, then an operation in camera RGB
    /// and one in the working space, on a 1000 × 700 sensor.
    fn tiled_chain() -> [Fake; 4] {
        let mosaic = ImageDesc::new(1000, 700, Format::Mosaic);
        let mut prepare_raw = fake("rawprepare", Stage::Raw, 1.0);
        prepare_raw.output = Some(mosaic);
        prepare_raw.passes = vec![Pass {
            strips: Some(Strips::PER_PIXEL),
            ..Pass::per_pixel(Vec::new(), Vec::new())
        }];
        let mut demosaic = fake("demosaic", Stage::Raw, 8.0);
        demosaic.demosaics = true;
        demosaic.scratch = vec![Scratch { bytes: 1000 }];
        [
            prepare_raw,
            demosaic,
            fake("exposure", Stage::CameraRgb, 21.0),
            fake("tone", Stage::WorkingSpace, 45.0),
        ]
    }

    #[test]
    fn a_viewport_too_large_to_bind_demosaics_and_downscales_in_tiles() {
        let chain = tiled_chain();
        let instances: Vec<Instance> = chain.iter().map(|op| instance(op)).collect();
        let image = ImageContext::full(1000, 700);
        let sensor = ImageDesc::new(1000, 700, Format::Mosaic16);
        let pipe = Pipe::Viewport {
            width: 300,
            height: 200,
        };
        // Demosaic's whole image, 11 MB, binds more than 2 MiB.
        let limits = Limits {
            binding: 2 << 20,
            ..Limits::NONE
        };
        let untiled = super::plan(&instances, &image, sensor, pipe, &Limits::NONE).unwrap();
        assert!(untiled.tiled.is_none());
        let plan = super::plan(&instances, &image, sensor, pipe, &limits).unwrap();
        assert!(plan.fits(&limits) && !untiled.fits(&limits));

        // Per tile: the copy, demosaic and the downscale, each always into the same
        // buffer; then the operations at the viewport's size.
        let tiled = plan.tiled.clone().unwrap();
        let ops = &plan.operations;
        assert_eq!(tiled.start, 1);
        let count = tiled.len() / 3;
        assert_eq!(count, 12, "tiles of 256 at most, 4 by 3");
        for (i, op) in ops[tiled.clone()].iter().enumerate() {
            assert_eq!(op.id, ["region", "demosaic", "downscale"][i % 3]);
            assert_eq!(op.output, ops[tiled.start + i % 3].output);
            assert_eq!(op.scratch, ops[tiled.start + i % 3].scratch);
        }
        let ids: Vec<_> = ops[tiled.end..].iter().map(|op| op.id).collect();
        assert_eq!(ids, ["exposure", "tone"]);
        // The raw stage's output and the viewport image are the cache points, and the
        // viewport image is the untiled plan's downscale.
        assert_eq!(plan.cached, [0, tiled.end - 1]);
        let viewport = ops[tiled.end - 1].output_desc;
        assert_eq!(viewport, untiled.operations[2].output_desc);
        assert_eq!(plan.edit_cache, Some(viewport));
        // Later operations are prepared as the untiled plan prepares them.
        for (a, b) in ops[tiled.end..].iter().zip(&untiled.operations[3..]) {
            assert_eq!(
                (a.passes.clone(), a.scale, a.origin),
                (b.passes.clone(), b.scale, b.origin)
            );
        }

        // Each downscale writes its part of the viewport from its demosaic's part of the
        // image: their parameters give the tiles back.
        let word = |op: &PlannedOperation, i: usize| {
            u32::from_le_bytes(op.passes[0].params[4 * i..4 * i + 4].try_into().unwrap())
        };
        let tiles: Vec<ViewportTile> = (0..count)
            .map(|t| {
                let (demosaic, downscale) =
                    (&ops[tiled.start + 3 * t + 1], &ops[tiled.start + 3 * t + 2]);
                let [width, height] = downscale.passes[0].size.unwrap();
                assert_eq!([word(downscale, 0), word(downscale, 1)], [1000, 700]);
                assert_eq!(
                    [word(downscale, 2), word(downscale, 3)],
                    [viewport.width, viewport.height]
                );
                assert_eq!(word(downscale, 6), demosaic.output_desc.width);
                ViewportTile {
                    source: [
                        word(downscale, 4),
                        word(downscale, 5),
                        demosaic.output_desc.width,
                        demosaic.output_desc.height,
                    ],
                    output: [downscale.origin[0], downscale.origin[1], width, height],
                }
            })
            .collect();
        check_tiles(&tiles, [1000, 700], [viewport.width, viewport.height]);

        // Without the limit, or within it, the viewport runs whole, as before.
        let within = Limits {
            binding: 16 << 20,
            ..Limits::NONE
        };
        assert!(
            super::plan(&instances, &image, sensor, pipe, &within)
                .unwrap()
                .tiled
                .is_none()
        );
        // Over the budget, demosaic's full-resolution image tiles too.
        let budget = Limits {
            budget: 8 << 20,
            ..Limits::NONE
        };
        assert!(
            super::plan(&instances, &image, sensor, pipe, &budget)
                .unwrap()
                .tiled
                .is_some()
        );
    }

    fn tiled_viewport(chain: &[Fake], width: u32, limits: &Limits) -> Result<Plan, PlanError> {
        let instances: Vec<Instance> = chain.iter().map(|op| instance(op)).collect();
        let pipe = Pipe::Viewport { width, height: 200 };
        let sensor = ImageDesc::new(1000, 700, Format::Mosaic16);
        super::plan(
            &instances,
            &ImageContext::full(1000, 700),
            sensor,
            pipe,
            limits,
        )
    }

    const TWO_MIB: Limits = Limits {
        binding: 2 << 20,
        ..Limits::NONE
    };

    #[test]
    fn a_reduction_after_demosaic_leaves_the_viewport_in_tiles() {
        // A tone mapper with a statistic of its own runs at the viewport's size, after
        // the tiles; only the raw stage's reductions decide where they begin.
        let [prepare_raw, demosaic, exposure, mut tone] = tiled_chain();
        tone.scratch = vec![Scratch { bytes: 64 }];
        tone.passes = vec![
            Pass {
                kind: PassKind::Reduction,
                bindings: vec![
                    Binding::new(0, Resource::Input),
                    Binding::new(2, Resource::Uniforms),
                    Binding::new(10, Resource::Scratch(0)),
                ],
                ..Pass::per_pixel(Vec::new(), Vec::new())
            },
            Pass::per_pixel(Vec::new(), Vec::new()),
        ];
        let plan = tiled_viewport(&[prepare_raw, demosaic, exposure, tone], 300, &TWO_MIB).unwrap();
        assert_eq!(plan.tiled.map(|tiled| tiled.start), Some(1));
    }

    /// A whole-image pass writing scratch 0, reading the input or not.
    fn statistic(input: bool) -> Pass {
        Pass {
            kind: PassKind::Reduction,
            bindings: [Binding::new(2, Resource::Uniforms)]
                .into_iter()
                .chain(input.then(|| Binding::new(0, Resource::Input)))
                .chain([Binding::new(10, Resource::Scratch(0))])
                .collect(),
            ..Pass::per_pixel(Vec::new(), Vec::new())
        }
    }

    /// The tiled chain with these passes in its last operation, `tiles` its answer to
    /// `tiles_whole_image_passes`, planned for an export.
    fn exported_tone(passes: Vec<Pass>, tiles: bool) -> Plan {
        let [prepare_raw, demosaic, exposure, mut tone] = tiled_chain();
        tone.scratch = vec![Scratch { bytes: 64 }];
        tone.passes = passes;
        tone.tiles = tiles;
        let chain = [prepare_raw, demosaic, exposure, tone];
        let instances: Vec<Instance> = chain.iter().map(|op| instance(op)).collect();
        let sensor = ImageDesc::new(1000, 700, Format::Mosaic16);
        let image = ImageContext::full(1000, 700);
        super::plan(&instances, &image, sensor, Pipe::Export, &Limits::NONE).unwrap()
    }

    #[test]
    fn an_exports_tiles_take_a_round_for_a_whole_image_pass_after_demosaic() {
        // The export's split stops after the 16-bit mosaic's reader, not after the tone
        // mapper's statistic, which takes a first round of tiles of its own (decision 15):
        // its pass that reads the input there, then, cut away, the pass after it.
        let passes = vec![
            statistic(true),
            statistic(false),
            Pass::per_pixel(Vec::new(), Vec::new()),
        ];
        let mut plan = exported_tone(passes, true);
        let split = split(&plan.operations);
        assert_eq!(split, 1);
        assert_eq!(runs(&plan.operations, split), ["tone"]);
        let reads: Vec<bool> = plan.operations[3]
            .passes
            .iter()
            .map(reads_the_whole_image)
            .collect();
        assert_eq!(reads, [true, false, false]);
        plan.retain_passes("tone", |pass| pass.kind == PassKind::Reduction);
        let tone = &plan.operations[3];
        assert_eq!((tone.passes.len(), tone.dispatches.len()), (2, 2));
        assert!(
            tone.passes
                .iter()
                .all(|pass| pass.kind == PassKind::Reduction)
        );
    }

    #[test]
    fn a_whole_image_pass_after_demosaic_runs_whole_unless_it_takes_one_round() {
        // Without the operation's word that its passes own what each tile writes, or
        // with passes that are not one run, a pass reading the input after one reading
        // only scratch or after a per-pixel one, the operations up to it run whole, as
        // they did before first rounds existed, and no round is planned.
        let pixel = || Pass::per_pixel(Vec::new(), Vec::new());
        let one_run = || vec![statistic(true), statistic(false), pixel()];
        let whole = [
            (one_run(), false),
            (
                vec![statistic(true), statistic(false), statistic(true), pixel()],
                true,
            ),
            (vec![statistic(true), pixel(), statistic(false)], true),
            (vec![statistic(false), statistic(true), pixel()], true),
        ];
        for (passes, tiles) in whole {
            let plan = exported_tone(passes, tiles);
            let split = split(&plan.operations);
            assert_eq!(split, 4, "{:?}", plan.operations[3].passes);
            assert!(runs(&plan.operations, split).is_empty());
        }
        let plan = exported_tone(one_run(), true);
        assert_eq!(split(&plan.operations), 1);
    }

    #[test]
    fn a_whole_image_result_is_keyed_apart_from_the_passes_after_it() {
        // Two whole-image passes with a per-pixel one into scratch between them, then
        // `main`: their result's key covers the three and their input's, never `main`'s
        // uniforms, so a change to those leaves it (decision 15).
        let pixel = |params: u8| Pass::per_pixel(vec![params], Vec::new());
        let passes = |mask: u8, main: u8| {
            vec![
                Pass {
                    params: vec![mask],
                    ..statistic(true)
                },
                Pass {
                    bindings: vec![
                        Binding::new(2, Resource::Uniforms),
                        Binding::new(10, Resource::Scratch(0)),
                    ],
                    ..pixel(mask)
                },
                Pass {
                    params: vec![mask],
                    ..statistic(false)
                },
                pixel(main),
            ]
        };
        let planned = |upstream: f32, passes: Vec<Pass>, pipe: Pipe| {
            let [prepare_raw, demosaic, mut exposure, mut tone] = raw_chain();
            exposure.image_default = Some(upstream);
            tone.scratch = vec![Scratch { bytes: 64 }];
            tone.passes = passes;
            plan_raw(&[prepare_raw, demosaic, exposure, tone], pipe)
        };
        let viewport = Pipe::Viewport {
            width: 6,
            height: 6,
        };
        let whole = |plan: Plan| plan.operations.last().unwrap().whole_image;
        let before = planned(0.0, passes(1, 1), viewport);
        assert!(
            before.operations[..before.operations.len() - 1]
                .iter()
                .all(|op| op.whole_image.is_none())
        );
        let before = whole(before).unwrap();
        assert_eq!(before.passes, 3);
        assert_eq!(whole(planned(0.0, passes(1, 2), viewport)), Some(before));
        // The same bytes bound elsewhere, or of another kind, compute something else.
        let mut rebound = passes(1, 1);
        rebound[2].bindings[1] = Binding::new(11, Resource::Scratch(0));
        let mut spatial = passes(1, 1);
        spatial[1].kind = PassKind::Spatial { radius: 1 };
        for changed in [
            planned(0.0, passes(2, 1), viewport),
            planned(0.5, passes(1, 1), viewport),
            planned(0.0, rebound, viewport),
            planned(0.0, spatial, viewport),
        ] {
            assert!(whole(changed).unwrap().key != before.key);
        }
        // An export's rounds hold the result by construction, never by key.
        assert_eq!(whole(planned(0.0, passes(1, 1), Pipe::Export)), None);
        // A pass among them that writes the output is not skipped: a render would leave
        // the output as an earlier operation wrote it.
        let writes = vec![statistic(true), pixel(1), statistic(false), pixel(1)];
        assert_eq!(whole(planned(0.0, writes, viewport)), None);
    }

    #[test]
    fn a_full_resolution_estimate_is_keyed_apart_from_the_viewports_size() {
        // An operation prepared with the full-resolution image keeps its passes up to
        // the last that binds it on that image's key, with nothing of the viewport's
        // size in it, so two viewport sizes share it; the whole-image passes as a whole,
        // which bring the estimate to the viewport's size, keep a key that holds it
        // (decision 15).
        let haze_at = |pipe: Pipe, upstream: f32| {
            let [prepare_raw, mut demosaic, exposure, tone] = raw_chain();
            demosaic.image_default = Some(upstream);
            let mut haze = fake("haze", Stage::CameraRgb, 14.0);
            haze.estimates = true;
            let chain = [&prepare_raw, &demosaic, &haze, &exposure, &tone];
            let instances: Vec<Instance> = chain.iter().map(|&op| instance(op)).collect();
            let sensor = ImageDesc::new(12, 8, Format::Mosaic16);
            let plan = plan(&instances, &ImageContext::full(12, 8), sensor, pipe).unwrap();
            plan.operations
                .into_iter()
                .find(|op| op.id == "haze")
                .unwrap()
        };
        let viewport = |width, height| Pipe::Viewport { width, height };
        let (small, smaller) = (haze_at(viewport(6, 6), 0.0), haze_at(viewport(4, 4), 0.0));
        assert!(small.full_resolution.is_some());
        assert_ne!(small.output_desc.width, smaller.output_desc.width);
        let whole = |op: &PlannedOperation| op.whole_image.unwrap();
        let part = |op: &PlannedOperation| whole(op).full_resolution.unwrap();
        // Up to the filter, the last pass that binds the full-resolution image.
        assert_eq!(part(&small).passes, 3);
        assert_eq!(part(&small), part(&smaller));
        assert_eq!(whole(&small).passes, 4);
        assert_ne!(whole(&small).key, whole(&smaller).key);
        // A change upstream reaches both.
        let changed = haze_at(viewport(6, 6), 0.5);
        assert_ne!(part(&changed).key, part(&small).key);
        assert_ne!(whole(&changed).key, whole(&small).key);
        // The part's pass header says the whole image at scale 1 it runs over.
        let scale = |op: &PlannedOperation, index: usize| {
            let block = uniform_block(op, index, &op.dispatches[index][0]);
            f32::from_le_bytes(block[24..28].try_into().unwrap())
        };
        assert_eq!((scale(&small, 2), scale(&small, 3)), (1.0, small.scale));
        assert!(small.scale < 1.0);

        // A part that binds the viewport's images, or runs over the output, depends on
        // the viewport's size: the operation keeps one part.
        let (passes, scratch) = estimate(&ImageDesc::rgba(12, 8), &ImageDesc::rgba(6, 4));
        let sizes: Vec<u64> = scratch.iter().map(|s| s.bytes).collect();
        let keyed = |passes: &[Pass]| full_resolution_part(1, "haze", 1, passes, &sizes);
        assert!(keyed(&passes[..4]).is_some());
        let mut reads_input = passes.clone();
        reads_input[1]
            .bindings
            .push(Binding::new(0, Resource::Input));
        let mut over_output = passes.clone();
        over_output[1].size = None;
        for changed in [reads_input, over_output] {
            assert_eq!(keyed(&changed[..4]), None);
        }
    }

    #[test]
    #[should_panic(expected = "2 instances in an export's tiles")]
    fn a_first_round_takes_one_instance_of_its_operation() {
        // The export holds a run's scratch by operation ID.
        let pixel = Pass::per_pixel(Vec::new(), Vec::new());
        let mut plan = exported_tone(vec![statistic(true), pixel], true);
        let again = plan.operations[3].clone();
        plan.operations.push(again);
        runs(&plan.operations, 1);
    }

    #[test]
    fn a_tile_that_does_not_make_its_region_is_refused() {
        // A demosaic that writes the whole image whatever the region, as demosaic does
        // when the region is not inside its input.
        let [prepare_raw, mut demosaic, exposure, tone] = tiled_chain();
        demosaic.demosaics = false;
        demosaic.output = Some(ImageDesc::rgba(1000, 700));
        assert!(matches!(
            tiled_viewport(&[prepare_raw, demosaic, exposure, tone], 300, &TWO_MIB),
            Err(PlanError::TileRegion {
                operation: "demosaic",
                ..
            })
        ));
    }

    #[test]
    fn a_viewport_too_large_to_bind_is_refused_at_once() {
        // 1000 × 700 RGBA takes 11 MB; demosaic's image fits the viewport whole, so no
        // tile helps the operations after it.
        let limits = Limits {
            binding: 4 << 20,
            ..Limits::NONE
        };
        let chain = tiled_chain();
        let instances: Vec<Instance> = chain.iter().map(|op| instance(op)).collect();
        let pipe = Pipe::Viewport {
            width: 1000,
            height: 700,
        };
        let sensor = ImageDesc::new(1000, 700, Format::Mosaic16);
        let image = ImageContext::full(1000, 700);
        let refused = super::plan(&instances, &image, sensor, pipe, &limits);
        assert!(
            matches!(
                refused,
                Err(PlanError::Viewport {
                    width: 1000,
                    height: 700,
                    ..
                })
            ),
            "{refused:?}"
        );
    }

    #[test]
    fn the_same_tiles_at_another_viewport_size_fit_the_same_buffers() {
        // A pipeline takes a plan of the same shape whose images, scratch and edit cache
        // its buffers hold (`Pipeline::accepts`): every smaller width with the same tiles
        // here, the footprint growing from 4 to 5 pixels on the way.
        let chain = tiled_chain();
        let first = tiled_viewport(&chain, 300, &TWO_MIB).unwrap();
        let held = |a: &ImageDesc, b: &ImageDesc| {
            a.format == b.format && b.buffer_size() <= a.buffer_size()
        };
        for width in (230..300).rev() {
            let resized = tiled_viewport(&chain, width, &TWO_MIB).unwrap();
            assert!(first.same_shape(&resized), "{width}");
            let work = first.work.iter().zip(&resized.work_used);
            assert!(work.clone().all(|(a, b)| held(a, b)), "{width}");
            let scratch = first.scratch.iter().zip(&resized.scratch_used);
            assert!(scratch.clone().all(|(a, b)| b <= a), "{width}");
            let edit = (first.edit_cache.unwrap(), resized.edit_cache.unwrap());
            assert!(held(&edit.0, &edit.1), "{width}");
        }
    }

    #[test]
    fn an_operation_after_the_tiles_never_writes_a_tiles_buffer() {
        // An operation at the viewport's size whose image is as large as a tile buffer
        // is made, as a crop might make it: reusing that buffer would leave what the
        // plan uses of it at the largest tile's.
        let first = tiled_viewport(&tiled_chain(), 300, &TWO_MIB).unwrap();
        let tiled = first.tiled.clone().unwrap();
        let Buffer::Work(demosaic) = first.operations[tiled.start + 1].output else {
            panic!("demosaic writes a working buffer");
        };
        let [prepare_raw, demosaic_op, exposure, mut tone] = tiled_chain();
        tone.output = Some(first.work[demosaic]);
        let plan =
            tiled_viewport(&[prepare_raw, demosaic_op, exposure, tone], 300, &TWO_MIB).unwrap();
        let tone = plan.operations.last().unwrap();
        assert!(
            plan.operations[tiled]
                .iter()
                .all(|op| op.output != tone.output)
        );
        for op in &plan.operations {
            let Buffer::Work(index) = op.output else {
                continue;
            };
            let used = plan.work_used[index];
            assert!(
                used.format == op.output_desc.format
                    && op.output_desc.buffer_size() <= used.buffer_size(),
                "{}",
                op.id
            );
        }
    }

    #[test]
    fn an_empty_output_is_refused() {
        let mut crop = fake("rawprepare", Stage::Raw, 1.0);
        crop.output = Some(ImageDesc::new(0, 4, Format::Mosaic));
        let mosaic = ImageDesc {
            format: Format::Mosaic16,
            ..RGBA
        };
        assert!(matches!(
            plan(&[instance(&crop)], &image(), mosaic, Pipe::Export),
            Err(PlanError::Empty {
                operation: "rawprepare",
                ..
            })
        ));
    }

    /// A geometry that doubles its input's width, at orientation's place: an image that
    /// grows, as rotate and perspective's can (decision 16).
    struct Widen;

    static WIDEN: Manifest = Manifest {
        id: "widen",
        version: 1,
        slot: Slot {
            stage: Stage::CameraRgb,
            order: 16.0,
        },
        domain: Domain::Scene,
        layers: false,
        differentiable: false,
        radius: 0,
        default_enabled: false,
        provenance: None,
    };

    impl Geometry for Widen {
        fn extent(&self, [width, height]: [u32; 2]) -> [u32; 2] {
            [2 * width, height]
        }

        fn input_region(&self, input: &Extent, [x, y, width, height]: [u32; 4]) -> [u32; 4] {
            let right = (x + width).div_ceil(2).min(input.scaled[0]);
            [x / 2, y, right - x / 2, height]
        }

        fn forward(&self, _: [u32; 2], [x, y]: [f32; 2]) -> Option<[f32; 2]> {
            Some([2.0 * x, y])
        }

        fn backward(&self, _: [u32; 2], [x, y]: [f32; 2]) -> Option<[f32; 2]> {
            Some([x / 2.0, y])
        }
    }

    impl Operation for Widen {
        fn manifest(&self) -> &'static Manifest {
            &WIDEN
        }

        fn schema(&self) -> &'static Schema {
            &SCHEMA
        }

        fn kernel(&self) -> &'static Kernel {
            &KERNEL
        }

        fn prepare(
            &self,
            _: &ParamValues,
            _: &ImageContext,
            input: &ImageDesc,
        ) -> Result<Prepared, NotPorted> {
            let mut prepared = Prepared::per_pixel(input, Vec::new(), Vec::new());
            prepared.output.width *= 2;
            Ok(prepared)
        }

        fn geometry(&self, _: &ParamValues, _: &ImageContext) -> Option<Box<dyn Geometry>> {
            Some(Box::new(Widen))
        }
    }

    #[test]
    fn the_edit_cache_holds_the_largest_image_after_its_start() {
        // A 20 by 10 image the viewport shows whole, widened to 40 by 10: the edit cache,
        // made for the input, grows to hold the widened image, which exposure reads
        // (decision 16).
        let exposure = fake("exposure", Stage::CameraRgb, 21.0);
        let viewport = Pipe::Viewport {
            width: 100,
            height: 100,
        };
        let input = ImageDesc::rgba(20, 10);
        let instances = [instance(&Widen), instance(&exposure)];
        let plan = plan(&instances, &ImageContext::full(20, 10), input, viewport).unwrap();
        let ids: Vec<&str> = plan.operations.iter().map(|op| op.id).collect();
        assert_eq!(ids, ["widen", "exposure"]);
        let widened = plan.operations[0].output_desc;
        assert_eq!((widened.width, widened.height), (40, 10));
        assert_eq!(plan.edit_cache, Some(widened));
        assert!(plan.edit_cache_takes(1));
    }
}
