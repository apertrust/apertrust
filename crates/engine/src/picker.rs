// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2016-2025  darktable developers (ported from src/common/color_picker.c)
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

//! The colour pickers (decision 79; `docs/pickers.md`): what the session's commands and
//! events about them carry, where a region lies in the photo and in a buffer, and the
//! reduction that measures it on the GPU.
//!
//! - **Regions** come in the photo's coordinates ([`Region`]) and are kept in the original
//!   image's, darktable's, mapped back through every geometry ([`Place`], [`Geometries`]),
//!   so an area stays on the same part of the scene as the photo is turned; they are
//!   mapped forward where they are measured and drawn, by darktable's sorted, averaged
//!   corners.
//! - **The reduction** ([`Reducer`], `slang/picker.slang`) gives each box's mean, minimum
//!   and maximum of four channels, its sums a fixed tree, and reads the results back
//!   through the staging pair the scopes share ([`crate::readback`]), never waiting: 48
//!   bytes a box.
//! - **The frame's reading** ([`FrameReading`]) is the frame's RGB, as the window shows
//!   it, and each statistic in Lab D50, as darktable's color picker utility reads its
//!   samples on the display-referred frame.
//!
//! Built: the Colour picker module's own picker, measured on the frame. Module pickers,
//! measured at their operation, and stored samples are to come.

use std::num::NonZeroU64;
use std::time::Instant;

pub use apertrust_modules::pick::{PickKeys, Region, Stats};
use apertrust_plugin::{ImageContext, Kernel, PassHeader};

use crate::compute::{ComputeError, GpuImage, GpuTimer, ImageKernel, POLL_INTERVAL};
use crate::gpu::GpuContext;
use crate::pipeline::{self, Instance, PlanError};
use crate::readback::Readback;
use crate::scopes::Profile;

/// Generated from `slang/picker.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "picker",
    wgsl: include_str!("../generated/picker.wgsl"),
    derivatives: false,
};

/// The boxes one reduction takes.
pub const MAX_BOXES: usize = 64;
/// The pixels a workgroup reduces across and down: 16 invocations each adding 4, 16 apart.
const TILE: u32 = 64;
/// A partial and a result in the kernel: a sum or a mean, a minimum and a maximum, each of
/// four channels.
const PARTIAL_BYTES: u64 = 48;
/// `PickerParams` in the kernel: the count, the dispatch's workgroups across, every box's
/// workgroups together and a word unused, then the boxes, 32 bytes each.
const PARAMS_BYTES: usize = 16 + MAX_BOXES * 32;
/// The two passes' start and end ticks, before the results in a staging buffer.
const TICKS_BYTES: u64 = 32;

/// darktable's default point, the photo's centre (`dt_lib_colorpicker_reset_point`).
pub const DEFAULT_POINT: Region = Region::Point([0.5, 0.5]);
/// darktable's default area, 2 % to 98 % of the photo on each axis
/// (`dt_lib_colorpicker_reset_box_area`).
pub const DEFAULT_AREA: Region = Region::Area([0.02, 0.02, 0.98, 0.98]);

/// A picker, by its owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PickerRef {
    /// A module's picker, by its module's id and the picker's id in its definition. No
    /// module declares one yet, so `On` refuses it.
    Module {
        module: &'static str,
        picker: &'static str,
    },
    /// The Colour picker module's, darktable's primary picker with no module.
    ColourPicker,
}

/// What a client asks of the active colour picker (decision 79): view state, never a
/// command on the edit. Points are in the photo's coordinates, each clamped to 0 to 1.
#[derive(Debug, Clone, PartialEq)]
pub enum PickerCommand {
    /// Turns `picker` on, the one on before it off: an area with `area`, else a point,
    /// at the picker's last point or area this session, else darktable's default.
    /// Refused before the photo is live, and for a picker no module declares.
    On {
        picker: PickerRef,
        area: bool,
        keys: PickKeys,
    },
    /// The active picker's point moved, a click or a drag's step, `done` on the release;
    /// a picker measuring an area measures this point from then on.
    Point {
        at: [f32; 2],
        done: bool,
        keys: PickKeys,
    },
    /// The active picker's area, from one corner to the other in any order, a drag's
    /// step or, with `done`, its release; a picker measuring a point measures this area
    /// from then on.
    Area {
        from: [f32; 2],
        to: [f32; 2],
        done: bool,
        keys: PickKeys,
    },
    /// darktable's Ctrl-click on the photo: point to area or back, each at its last place,
    /// else the default.
    Switch,
    /// The active picker back to its default for its kind.
    Reset,
    /// The active picker off, its point and area kept for its next `On`; with none on,
    /// nothing.
    Off,
}

/// The active picker's reading, `Event::Picker`'s: a fact, `None` while no picker is on.
#[derive(Debug, Clone, PartialEq)]
pub struct PickerReading {
    /// Every command the session had applied when it measured: the frame's, and any after
    /// it that rendered nothing, so a client knows its moves are in it. A command that
    /// changes nothing brings no reading with a newer `through`: a client must not wait
    /// for one.
    pub through: u64,
    pub picker: PickerRef,
    /// As drawn, in the photo's coordinates: an area the average of its sorted corners,
    /// as darktable's `dt_color_picker_transform_box` draws it.
    pub region: Region,
    /// The frame's colour under the region; `None` where the region misses the frame.
    pub frame: Option<FrameReading>,
}

/// darktable's `display` and `lab` of a sample (`_pixelpipe_pick_samples`): mean, min
/// and max.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameReading {
    /// The frame's RGB, as the window shows it and the scopes count it: sRGB, darktable's
    /// default display and histogram profile, so both its `display` and its `scope`.
    pub rgb: [[f32; 3]; 3],
    /// Each of the three in Lab D50, as darktable converts mean, min and max.
    pub lab: [[f32; 3]; 3],
}

impl FrameReading {
    /// The reading `stats`, the frame's own statistics, give: each statistic's RGB read
    /// through the frame's profile into XYZ D50 and Lab, as darktable's
    /// `dt_ioppr_transform_image_colorspace` reads each of mean, min and max through the
    /// display profile.
    pub fn of(stats: &Stats) -> Self {
        let profile = Profile::srgb();
        let rgb = [stats.mean, stats.min, stats.max].map(|v| [v[0], v[1], v[2]]);
        let lab = rgb.map(|v| apertrust_color::xyz_to_lab_d50(profile.xyz(v)));
        Self { rgb, lab }
    }
}

/// A point or area in the original image's coordinates, 0 to 1 across the developed image
/// at full resolution, before any geometry, where the session keeps a picker's region, as
/// darktable keeps it (`color_picker.c:307-330`). An area is its four corners, which a
/// turn or a distortion may make any quadrilateral.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Place {
    Point([f32; 2]),
    Area([[f32; 2]; 4]),
}

impl Place {
    /// `region`, in the photo's coordinates, mapped back, as darktable's
    /// `dt_color_picker_backtransform_box` maps it: a point, or an area's four corners,
    /// each through every geometry backward and clamped to the original image.
    pub fn of(region: Region, geometries: &Geometries) -> Self {
        match region {
            Region::Point(point) => Place::Point(geometries.back(point)),
            Region::Area([left, top, right, bottom]) => Place::Area(
                [[left, top], [right, bottom], [right, top], [left, bottom]]
                    .map(|corner| geometries.back(corner)),
            ),
        }
    }

    /// The region as drawn, in the photo's coordinates, as darktable's
    /// `dt_color_picker_transform_box` draws it: a point mapped forward; an area's four
    /// corners mapped forward, sorted on each axis, the two lowest and the two highest
    /// averaged.
    pub fn region(&self, geometries: &Geometries) -> Region {
        match self {
            Place::Point(point) => Region::Point(geometries.forward(*point)),
            Place::Area(corners) => {
                let corners = corners.map(|corner| geometries.forward(corner));
                let axis = |a: usize| {
                    let mut values = corners.map(|corner| corner[a]);
                    values.sort_by(f32::total_cmp);
                    [0.5 * (values[0] + values[1]), 0.5 * (values[2] + values[3])]
                };
                let ([left, right], [top, bottom]) = (axis(0), axis(1));
                Region::Area([left, top, right, bottom])
            }
        }
    }
}

/// The maps between the photo's coordinates and the original image's: every operation
/// that moves pixels, in pipeline order, each with its input's extent at full resolution
/// (decision 16).
pub struct Geometries {
    /// The developed image at full resolution, the original image's coordinates' extent.
    developed: [u32; 2],
    maps: Vec<pipeline::Map>,
    /// The photo at full resolution, after every map: the photo's coordinates' extent.
    extent: [u32; 2],
}

impl Geometries {
    /// The maps from the developed image, `developed` at full resolution, through `maps`,
    /// each with its input's extent.
    pub fn new(developed: [u32; 2], maps: Vec<pipeline::Map>) -> Self {
        let extent = maps
            .last()
            .map_or(developed, |(input, geometry)| geometry.extent(*input));
        Self {
            developed,
            maps,
            extent,
        }
    }

    /// The maps the operations among `instances` make of the photo `image` describes.
    pub(crate) fn of(instances: &[Instance], image: &ImageContext) -> Result<Self, PlanError> {
        let maps = pipeline::geometries(instances, image)?;
        Ok(Self::new([image.width, image.height], maps))
    }

    /// A point in the photo's coordinates in the original image's, clamped to it: through
    /// each geometry's `backward`, last first, at full resolution. A geometry that gives
    /// no place for it leaves it where it is.
    pub fn back(&self, point: [f32; 2]) -> [f32; 2] {
        let point = clamped(point);
        let mut at = [0, 1].map(|a| point[a] * self.extent[a] as f32);
        for (input, geometry) in self.maps.iter().rev() {
            at = geometry.backward(*input, at).unwrap_or(at);
        }
        clamped([0, 1].map(|a| at[a] / self.developed[a].max(1) as f32))
    }

    /// A point in the original image's coordinates in the photo's: through each
    /// geometry's `forward`, first first, at full resolution. A geometry that gives no
    /// place for it leaves it where it is.
    pub fn forward(&self, point: [f32; 2]) -> [f32; 2] {
        let mut at = [0, 1].map(|a| point[a] * self.developed[a] as f32);
        for (input, geometry) in &self.maps {
            at = geometry.forward(*input, at).unwrap_or(at);
        }
        [0, 1].map(|a| at[a] / self.extent[a].max(1) as f32)
    }
}

/// `point` clamped to 0 to 1 on each axis, a coordinate that is not a number at 0.
pub fn clamped(point: [f32; 2]) -> [f32; 2] {
    point.map(|v| if v >= 0.0 { v.min(1.0) } else { 0.0 })
}

/// The box `region`, in the photo's coordinates, covers in a buffer of `size` pixels that
/// shows the whole photo, as darktable's `dt_color_picker_box` makes it: left, top, right
/// and bottom, right and bottom exclusive, each truncated toward zero as C converts a
/// float, at least one pixel wide and high, clamped to the buffer. A point is the one
/// pixel it falls in. `None` where the region misses the buffer, as a point on its right
/// or bottom edge does.
pub fn pixels(region: Region, [width, height]: [u32; 2]) -> Option<[u32; 4]> {
    let [left, top, right, bottom] = match region {
        Region::Point([x, y]) => [x, y, x, y],
        Region::Area(area) => area,
    };
    let scaled = |v: f32, side: u32| (v * side as f32) as i64;
    let mut bounds = [
        scaled(left, width),
        scaled(top, height),
        scaled(right, width),
        scaled(bottom, height),
    ];
    bounds[2] = bounds[2].max(bounds[0] + 1);
    bounds[3] = bounds[3].max(bounds[1] + 1);
    let (width, height) = (i64::from(width), i64::from(height));
    if bounds[0] >= width || bounds[1] >= height || bounds[2] < 0 || bounds[3] < 0 {
        return None;
    }
    let bounds = [
        bounds[0].clamp(0, width - 1),
        bounds[1].clamp(0, height - 1),
        bounds[2].clamp(1, width),
        bounds[3].clamp(1, height),
    ];
    if bounds[2] - bounds[0] < 1 || bounds[3] - bounds[1] < 1 {
        return None;
    }
    Some(bounds.map(|v| v as u32))
}

/// A reduction on its way, with what its owner keeps of it.
struct Measured<T> {
    tag: T,
    /// Each box's pixels, which its sum is divided by.
    pixels: Vec<u32>,
    submitted: Instant,
    /// Whether the passes' ticks were resolved, so the reading has a GPU time.
    timed: bool,
}

/// What a reduction gave: each box's statistics, in the order given.
#[derive(Debug, Clone, PartialEq)]
pub struct Reduced<T> {
    /// What its owner gave with it.
    pub tag: T,
    pub stats: Vec<Stats>,
    /// Both passes' GPU time; NaN where the device has no timestamp queries, or gave none.
    pub gpu_ms: f64,
    /// From the reduction's submission to its result being picked up.
    pub latency_ms: f64,
}

/// Reduces boxes of an RGBA buffer on the GPU and reads the results back through two
/// staging buffers in turn, never waiting: the pickers' GPU side, which the session keeps
/// for its life. `T` is what its owner keeps of each reduction until it arrives.
pub struct Reducer<T> {
    rgba: ImageKernel,
    combine: ImageKernel,
    uniform: wgpu::Buffer,
    /// Each workgroup's partial, grown to what a reduction needs.
    partials: wgpu::Buffer,
    results: wgpu::Buffer,
    readback: Readback<Measured<T>>,
    timer: Option<GpuTimer>,
}

impl<T> Reducer<T> {
    /// Compiles the two entry points and makes the buffers: the uniforms, the results and
    /// their staging pair, 3 KiB each, and the partials, grown as a reduction needs them.
    pub async fn new(gpu: &GpuContext) -> Result<Self, ComputeError> {
        let rgba = ImageKernel::with_entry_point(gpu, KERNEL.name, KERNEL.wgsl, "picker_rgba");
        let combine =
            ImageKernel::with_entry_point(gpu, KERNEL.name, KERNEL.wgsl, "picker_combine");
        let uniform = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("picker uniforms"),
            size: (PassHeader::SIZE + PARAMS_BYTES) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let results_bytes = MAX_BOXES as u64 * PARTIAL_BYTES;
        let results = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("picker results"),
            size: results_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        Ok(Self {
            rgba: rgba.await?,
            combine: combine.await?,
            uniform,
            partials: partials(gpu, PARTIAL_BYTES),
            results,
            readback: Readback::new(gpu, "picker", TICKS_BYTES + results_bytes),
            timer: GpuTimer::new(gpu, 2),
        })
    }

    /// Reduces `boxes` of `frame`, an RGBA image, as [`Self::measure_buffer`] does.
    pub fn measure(&mut self, gpu: &GpuContext, frame: &GpuImage, boxes: &[[u32; 4]], tag: T) {
        let size = [frame.width(), frame.height()];
        self.measure_buffer(gpu, frame.buffer(), size, boxes, tag);
    }

    /// Reduces `boxes` of the RGBA image in `buffer`, of `size` pixels, in a submission of
    /// its own after everything submitted before it, and returns without waiting; the
    /// result arrives through [`Self::poll`] with `tag`. Each box is left, top, right and
    /// bottom, right and bottom exclusive, never empty and within the image, as
    /// [`pixels`] gives it; at most [`MAX_BOXES`] of them.
    pub(crate) fn measure_buffer(
        &mut self,
        gpu: &GpuContext,
        buffer: &wgpu::Buffer,
        [width, height]: [u32; 2],
        boxes: &[[u32; 4]],
        tag: T,
    ) {
        assert!(
            (1..=MAX_BOXES).contains(&boxes.len()),
            "{} boxes, where a reduction takes 1 to {MAX_BOXES}",
            boxes.len()
        );
        // Each box's workgroups, one after another in the dispatch.
        let mut workgroups = 0u32;
        let mut params = vec![0u8; 16];
        for &[left, top, right, bottom] in boxes {
            debug_assert!(left < right && top < bottom && right <= width && bottom <= height);
            let across = (right - left).div_ceil(TILE);
            let count = across * (bottom - top).div_ceil(TILE);
            for word in [left, top, right, bottom, workgroups, across, count, 0] {
                params.extend_from_slice(&word.to_le_bytes());
            }
            workgroups += count;
        }
        params.resize(PARAMS_BYTES, 0);
        let limit = gpu.device.limits().max_compute_workgroups_per_dimension;
        let across = workgroups.min(limit);
        for (at, word) in [boxes.len() as u32, across, workgroups].iter().enumerate() {
            params[at * 4..at * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        let block = PassHeader::whole(width, height).uniform_block(&params);
        gpu.queue.write_buffer(&self.uniform, 0, &block);
        let needed = u64::from(workgroups) * PARTIAL_BYTES;
        if self.partials.size() < needed {
            self.partials = partials(gpu, needed);
        }

        let bytes = NonZeroU64::new(u64::from(width) * u64::from(height) * 16);
        let image = wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer,
            offset: 0,
            size: bytes,
        });
        let entry = |binding, resource| wgpu::BindGroupEntry { binding, resource };
        let rgba = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("picker_rgba"),
            layout: &self.rgba.pipeline().get_bind_group_layout(0),
            entries: &[
                entry(0, image),
                entry(1, self.partials.as_entire_binding()),
                entry(2, self.uniform.as_entire_binding()),
            ],
        });
        let combine = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("picker_combine"),
            layout: &self.combine.pipeline().get_bind_group_layout(0),
            entries: &[
                entry(1, self.partials.as_entire_binding()),
                entry(2, self.uniform.as_entire_binding()),
                entry(3, self.results.as_entire_binding()),
            ],
        });
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        for (slot, (kernel, bind_group, groups)) in [
            (&self.rgba, &rgba, [across, workgroups.div_ceil(across)]),
            (&self.combine, &combine, [boxes.len() as u32, 1]),
        ]
        .into_iter()
        .enumerate()
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(KERNEL.name),
                timestamp_writes: self.timer.as_ref().map(|timer| timer.writes(slot as u32)),
            });
            pass.set_pipeline(kernel.pipeline());
            pass.set_bind_group(0, bind_group, &[]);
            pass.dispatch_workgroups(groups[0], groups[1], 1);
        }
        if let Some(timer) = &self.timer {
            timer.resolve(&mut encoder, 2);
        }
        let measured = Measured {
            tag,
            pixels: boxes
                .iter()
                .map(|&[left, top, right, bottom]| (right - left) * (bottom - top))
                .collect(),
            submitted: Instant::now(),
            timed: self.timer.is_some(),
        };
        let (results, timer) = (&self.results, self.timer.as_ref());
        let bytes = TICKS_BYTES + boxes.len() as u64 * PARTIAL_BYTES;
        (self.readback).submit(gpu, encoder, measured, bytes, |encoder, staging, bytes| {
            copy(encoder, timer, results, staging, bytes)
        });
    }

    /// Adds the reductions whose mappings have completed to `reduced`, in the order they
    /// were submitted, after polling the device without waiting; and, if a staging buffer
    /// has freed and a result is waiting for one, submits its copy and maps it. A mapping
    /// that failed is reported after the others are added: its result is lost.
    pub fn poll(
        &mut self,
        gpu: &GpuContext,
        reduced: &mut Vec<Reduced<T>>,
    ) -> Result<(), ComputeError> {
        let (results, timer) = (&self.results, self.timer.as_ref());
        self.readback.poll(
            gpu,
            |encoder, staging, bytes| copy(encoder, timer, results, staging, bytes),
            |measured, bytes| {
                let ticks: [u64; 4] = bytemuck::pod_read_unaligned(&bytes[..TICKS_BYTES as usize]);
                let floats: Vec<f32> = bytemuck::pod_collect_to_vec(&bytes[TICKS_BYTES as usize..]);
                let stats = floats
                    .as_chunks::<12>()
                    .0
                    .iter()
                    .zip(&measured.pixels)
                    // darktable's mean, `acc / (float)size`, divided here so that it is
                    // the same bits on every GPU, whose division need only be within 2.5
                    // ulp.
                    .map(|(v, &pixels)| Stats {
                        mean: [v[0], v[1], v[2], v[3]].map(|sum| sum / pixels as f32),
                        min: [v[4], v[5], v[6], v[7]],
                        max: [v[8], v[9], v[10], v[11]],
                    })
                    .collect();
                // Ticks the GPU did not give are no time.
                let gpu_ms = match timer {
                    Some(timer) if measured.timed && ticks[1] > ticks[0] && ticks[3] > ticks[2] => {
                        timer.milliseconds(ticks[0], ticks[1])
                            + timer.milliseconds(ticks[2], ticks[3])
                    }
                    _ => f64::NAN,
                };
                let latency_ms = measured.submitted.elapsed().as_secs_f64() * 1e3;
                tracing::debug!(
                    target: "perf",
                    "picked {} box(es), read back after {latency_ms:.1} ms{}",
                    measured.pixels.len(),
                    if gpu_ms.is_nan() {
                        String::new()
                    } else {
                        format!(", GPU: {gpu_ms:.3} ms")
                    }
                );
                reduced.push(Reduced {
                    tag: measured.tag,
                    stats,
                    gpu_ms,
                    latency_ms,
                });
            },
        )
    }

    /// Whether a result is still on its way.
    pub fn pending(&self) -> bool {
        self.readback.pending()
    }

    /// Drops every result on its way, as when the picker turns off: those mapping still
    /// arrive, and are dropped then.
    pub fn discard(&mut self) {
        self.readback.discard();
    }

    /// Waits until every result on its way has arrived, polling as [`GpuContext::wait`]
    /// does, and returns them: for the CLI and tests, never the session, which polls
    /// between commands.
    pub fn wait(&mut self, gpu: &GpuContext) -> Result<Vec<Reduced<T>>, ComputeError> {
        let mut reduced = Vec::new();
        loop {
            self.poll(gpu, &mut reduced)?;
            if !self.pending() {
                return Ok(reduced);
            }
            std::thread::sleep(POLL_INTERVAL);
        }
    }

    /// Whether a result `which` picks is yet to arrive.
    #[cfg(test)]
    pub(crate) fn awaits(&mut self, which: impl Fn(&T) -> bool) -> bool {
        self.readback.awaits(|measured| which(&measured.tag))
    }
}

/// A partials buffer of `bytes`.
fn partials(gpu: &GpuContext, bytes: u64) -> wgpu::Buffer {
    gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("picker partials"),
        size: bytes,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    })
}

/// Records copying both passes' ticks, resolved after them, and the results, `bytes` in
/// all with the ticks, into `staging`.
fn copy(
    encoder: &mut wgpu::CommandEncoder,
    timer: Option<&GpuTimer>,
    results: &wgpu::Buffer,
    staging: &wgpu::Buffer,
    bytes: u64,
) {
    if let Some(timer) = timer {
        encoder.copy_buffer_to_buffer(timer.resolved(), 0, staging, 0, TICKS_BYTES);
    }
    encoder.copy_buffer_to_buffer(results, 0, staging, TICKS_BYTES, bytes - TICKS_BYTES);
}
