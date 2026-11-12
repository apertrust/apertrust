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

//! The scopes (`docs/gui.md`, The scopes; `docs/engine.md`, Scopes; decision 19): the
//! viewport's frame counted on the GPU, as the window shows it, and read back without the
//! engine thread ever waiting for it. Each count gives the three totals (the pixels
//! counted, and those with a channel at or below black or at or above white) and the
//! graphs asked for, the histogram, the waveform and the vectorscope (`slang/scopes.slang`
//! says what each count holds). The vectorscope is darktable's default (`Plane`).
//!
//! A [`Meter`] submits the count of a frame and reads the counts back through the staging
//! pair it shares with the colour pickers ([`crate::readback`]): two staging buffers in
//! turn, mapped without waiting and picked up by [`Meter::poll`], the counts waiting in
//! the counts buffer while both are busy, each frame's replacing the last's. The last
//! frame's reading always arrives, and the scopes lag the photo by about a frame.

use std::num::NonZeroU64;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use apertrust_color::illuminants::D50_XY;
use apertrust_color::{Mat3, SRGB_PRIMARIES, apply, profile_rgb_to_xyz_d50, to_f32};
use apertrust_plugin::{Kernel, PassHeader};

use crate::compute::{ComputeError, GpuImage, GpuTimer, ImageKernel, POLL_INTERVAL};
use crate::gpu::GpuContext;
use crate::readback::Readback;

/// Generated from `slang/scopes.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "scopes",
    wgsl: include_str!("../generated/scopes.wgsl"),
    derivatives: false,
};

/// The histogram's bins in each channel, evenly spaced from 0 to 1.
pub const BINS: usize = 256;
/// The waveform's columns, evenly spaced across the frame.
pub const COLUMNS: usize = 256;
/// The waveform's levels in each column, evenly spaced from 0 to 1.
pub const LEVELS: usize = 128;
/// The vectorscope's cells across and down.
pub const CELLS: usize = 128;
/// Red, green, blue and luminance.
const CHANNELS: usize = 4;

/// The counts buffer's parts, in 32-bit words, as the kernel lays them out: the totals and
/// a word unused, the histogram, the vectorscope, the waveform.
const TOTALS_WORDS: usize = 4;
const HISTOGRAM_WORDS: usize = CHANNELS * BINS;
const VECTORSCOPE_WORDS: usize = CELLS * CELLS;
const WAVEFORM_WORDS: usize = CHANNELS * COLUMNS * LEVELS;
const ALL_WORDS: usize = TOTALS_WORDS + HISTOGRAM_WORDS + VECTORSCOPE_WORDS + WAVEFORM_WORDS;
/// `ScopesParams` in the kernel: the flags, then the vectorscope's plane.
const PARAMS_BYTES: usize = 80;
/// A staging buffer starts with the count's start and end ticks, then the counts.
const TICKS_BYTES: u64 = 16;
/// The pixels a workgroup counts across and down: 16 invocations each counting 4, 16
/// apart.
const WORKGROUP_PIXELS: u32 = 64;

/// The graphs to count besides the totals, which every count gives: only the graph a
/// client shows, since the waveform takes 512 KiB a frame to read back and the
/// vectorscope 64 KiB.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Graphs {
    pub histogram: bool,
    /// For the waveform and the RGB parade.
    pub waveform: bool,
    pub vectorscope: bool,
}

impl Graphs {
    /// The counts buffer's words these graphs read back, from its start.
    fn words(self) -> usize {
        if self.waveform {
            ALL_WORDS
        } else if self.vectorscope {
            TOTALS_WORDS + HISTOGRAM_WORDS + VECTORSCOPE_WORDS
        } else if self.histogram {
            TOTALS_WORDS + HISTOGRAM_WORDS
        } else {
            TOTALS_WORDS
        }
    }

    /// Whether these graphs include one that `other` does not.
    pub fn adds_to(self, other: Graphs) -> bool {
        (self.histogram && !other.histogram)
            || (self.waveform && !other.waveform)
            || (self.vectorscope && !other.vectorscope)
    }
}

/// How many pixels were counted, and how many clip: with any of red, green and blue at or
/// below 0, and with any at or above 1. A pixel may count in both.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Totals {
    pub pixels: u32,
    pub shadows: u32,
    pub highlights: u32,
}

/// For each channel, [`BINS`] counts evenly spaced from 0 to 1, black first; a value below
/// 0 counts in the first, one at 1 or above in the last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Histogram {
    pub red: Vec<u32>,
    pub green: Vec<u32>,
    pub blue: Vec<u32>,
    /// The encoded relative luminance, in which a grey falls in the same bin as in the
    /// other three.
    pub luminance: Vec<u32>,
}

/// For each channel, `levels` counts for each of `columns` columns of the frame, column
/// after column from its left, the darkest level first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Waveform {
    pub columns: usize,
    pub levels: usize,
    pub red: Vec<u32>,
    pub green: Vec<u32>,
    pub blue: Vec<u32>,
    pub luminance: Vec<u32>,
}

/// The vectorscope: `size` × `size` counts over darktable's default plane (`Plane`), row
/// after row from the top, as darktable shows it by default: neutral at the centre, in the
/// cell `size / 2` of row `size / 2`, red and magenta up, yellow to the left and blue to
/// the right, saturation out from the centre on darktable's logarithmic scale to sRGB's
/// most saturated colour at the edge. Each sample is the mean of a 2 × 2 block of the
/// frame, a last odd row or column left out; one off the grid counts nowhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vectorscope {
    pub size: usize,
    pub counts: Vec<u32>,
    /// Each cell's colour, as darktable paints the plane under its counts: the hue at the
    /// cell's angle, each channel 0 to 255 as darktable hands it to its display, the same
    /// table for every reading.
    pub colours: Arc<[[u8; 3]]>,
}

/// What one frame holds, as counted. Plain data, so it can move into the protocol.
#[derive(Debug, Clone, PartialEq)]
pub struct Reading {
    /// The frame it counts, by the commands it includes, as its frame event says.
    pub through: u64,
    /// The frame's size.
    pub width: u32,
    pub height: u32,
    pub totals: Totals,
    /// While asked for ([`Graphs`]).
    pub histogram: Option<Histogram>,
    pub waveform: Option<Waveform>,
    /// Boxed, so the session's events stay small.
    pub vectorscope: Option<Box<Vectorscope>>,
    /// The count's GPU time; NaN where the device has no timestamp queries, or gave none,
    /// or there was nothing to count.
    pub gpu_ms: f64,
    /// From the count's submission to the reading being picked up.
    pub latency_ms: f64,
}

impl Reading {
    /// The reading in `words`, the counts buffer's first words, as `measured` asked.
    fn new(measured: &Measured, words: &[u32], gpu_ms: f64) -> Self {
        let channels = |at: usize, each: usize| -> [Vec<u32>; CHANNELS] {
            std::array::from_fn(|c| words[at + c * each..at + (c + 1) * each].to_vec())
        };
        let histogram = measured.graphs.histogram.then(|| {
            let [red, green, blue, luminance] = channels(TOTALS_WORDS, BINS);
            Histogram {
                red,
                green,
                blue,
                luminance,
            }
        });
        let waveform = measured.graphs.waveform.then(|| {
            let [red, green, blue, luminance] = channels(
                TOTALS_WORDS + HISTOGRAM_WORDS + VECTORSCOPE_WORDS,
                COLUMNS * LEVELS,
            );
            Waveform {
                columns: COLUMNS,
                levels: LEVELS,
                red,
                green,
                blue,
                luminance,
            }
        });
        let vectorscope = measured.graphs.vectorscope.then(|| {
            let at = TOTALS_WORDS + HISTOGRAM_WORDS;
            Box::new(Vectorscope {
                size: CELLS,
                counts: words[at..at + VECTORSCOPE_WORDS].to_vec(),
                colours: Plane::get().colours.clone(),
            })
        });
        Self {
            through: measured.through,
            width: measured.width,
            height: measured.height,
            totals: Totals {
                pixels: words[0],
                shadows: words[1],
                highlights: words[2],
            },
            histogram,
            waveform,
            vectorscope,
            gpu_ms,
            latency_ms: measured.submitted.elapsed().as_secs_f64() * 1e3,
        }
    }
}

/// A frame counted, whose reading is on its way.
#[derive(Debug, Clone, Copy)]
struct Measured {
    through: u64,
    width: u32,
    height: u32,
    graphs: Graphs,
    submitted: Instant,
    /// Whether a pass ran and its ticks were resolved, so the reading has a GPU time.
    timed: bool,
}

/// Counts frames and reads the counts back through two staging buffers in turn, never
/// waiting: the scopes' GPU side, which the session keeps for its life.
pub struct Meter {
    kernel: ImageKernel,
    uniform: wgpu::Buffer,
    counts: wgpu::Buffer,
    readback: Readback<Measured>,
    timer: Option<GpuTimer>,
}

impl Meter {
    /// Compiles the kernel and makes the buffers: about 0.5 MiB for the counts, twice that
    /// for the staging buffers.
    pub async fn new(gpu: &GpuContext) -> Result<Self, ComputeError> {
        let kernel = ImageKernel::new(gpu, KERNEL.name, KERNEL.wgsl).await?;
        let uniform = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scopes uniforms"),
            size: (PassHeader::SIZE + PARAMS_BYTES) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let counts = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scopes counts"),
            size: (ALL_WORDS * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Ok(Self {
            kernel,
            uniform,
            counts,
            readback: Readback::new(gpu, "scopes", TICKS_BYTES + (ALL_WORDS * 4) as u64),
            timer: GpuTimer::new(gpu, 1),
        })
    }

    /// Counts `frame`, an RGBA image, for the totals and `graphs`, in a submission of its
    /// own after everything submitted before it, and returns without waiting; its reading
    /// arrives through [`Self::poll`], `through` naming the frame.
    pub fn measure(&mut self, gpu: &GpuContext, frame: &GpuImage, graphs: Graphs, through: u64) {
        self.measure_buffer(
            gpu,
            frame.buffer(),
            [frame.width(), frame.height()],
            graphs,
            through,
        );
    }

    /// [`Self::measure`] on a frame in `buffer`, of `size` RGBA pixels.
    pub(crate) fn measure_buffer(
        &mut self,
        gpu: &GpuContext,
        buffer: &wgpu::Buffer,
        size: [u32; 2],
        graphs: Graphs,
        through: u64,
    ) {
        let [width, height] = size;
        let mut measured = Measured {
            through,
            width,
            height,
            graphs,
            submitted: Instant::now(),
            timed: false,
        };
        let words = graphs.words();
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        encoder.clear_buffer(&self.counts, 0, Some((words * 4) as u64));
        // An empty frame has nothing to count, and its totals are the cleared zeros.
        if let Some(bytes) = NonZeroU64::new(u64::from(width) * u64::from(height) * 16) {
            let block = PassHeader::whole(width, height).uniform_block(&params(graphs));
            gpu.queue.write_buffer(&self.uniform, 0, &block);
            let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(KERNEL.name),
                layout: &self.kernel.pipeline().get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer,
                            offset: 0,
                            size: Some(bytes),
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.counts.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.uniform.as_entire_binding(),
                    },
                ],
            });
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(KERNEL.name),
                timestamp_writes: self.timer.as_ref().map(|timer| timer.writes(0)),
            });
            pass.set_pipeline(self.kernel.pipeline());
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(
                width.div_ceil(WORKGROUP_PIXELS),
                height.div_ceil(WORKGROUP_PIXELS),
                1,
            );
            drop(pass);
            if let Some(timer) = &self.timer {
                timer.resolve(&mut encoder, 1);
                measured.timed = true;
            }
        }
        let (counts, timer) = (&self.counts, self.timer.as_ref());
        let bytes = TICKS_BYTES + (words * 4) as u64;
        (self.readback).submit(gpu, encoder, measured, bytes, |encoder, staging, bytes| {
            copy(encoder, timer, counts, staging, bytes)
        });
    }

    /// Adds the readings whose mappings have completed to `readings`, in the order they
    /// were counted, after polling the device without waiting; and, if a staging buffer
    /// has freed and a frame's counts are waiting for one, submits their copy and maps it.
    /// A mapping that failed is reported after the others are added: its reading is lost
    /// and its buffer free again.
    pub fn poll(
        &mut self,
        gpu: &GpuContext,
        readings: &mut Vec<Reading>,
    ) -> Result<(), ComputeError> {
        let (counts, timer) = (&self.counts, self.timer.as_ref());
        let mut arrived = Vec::new();
        let polled = self.readback.poll(
            gpu,
            |encoder, staging, bytes| copy(encoder, timer, counts, staging, bytes),
            |measured, bytes| {
                let ticks: [u64; 2] = bytemuck::pod_read_unaligned(&bytes[..TICKS_BYTES as usize]);
                let words: Vec<u32> = bytemuck::pod_collect_to_vec(&bytes[TICKS_BYTES as usize..]);
                // Ticks the GPU did not give are no time: on the M4, a meter's first count
                // reads zero.
                let gpu_ms = match timer {
                    Some(timer) if measured.timed && ticks[1] > ticks[0] => {
                        timer.milliseconds(ticks[0], ticks[1])
                    }
                    _ => f64::NAN,
                };
                arrived.push(Reading::new(&measured, &words, gpu_ms));
            },
        );
        for reading in arrived {
            tracing::debug!(
                target: "perf",
                "counted scopes of {}×{}{}, read back after {:.1} ms",
                reading.width,
                reading.height,
                if reading.gpu_ms.is_nan() {
                    String::new()
                } else {
                    format!(", GPU: {:.2} ms", reading.gpu_ms)
                },
                reading.latency_ms
            );
            readings.push(reading);
        }
        polled
    }

    /// Whether a reading is still on its way: a staging buffer mapping, or counts waiting
    /// for one.
    pub fn pending(&self) -> bool {
        self.readback.pending()
    }

    /// Drops every reading on its way, as when the photo they measure closes: those
    /// mapping still arrive, and are dropped then.
    pub fn discard(&mut self) {
        self.readback.discard();
    }

    /// Waits until every reading on its way has arrived, polling as
    /// [`GpuContext::wait`] does, and returns them: for the CLI and tests, never the
    /// session, which polls between commands.
    pub fn wait(&mut self, gpu: &GpuContext) -> Result<Vec<Reading>, ComputeError> {
        let mut readings = Vec::new();
        loop {
            self.poll(gpu, &mut readings)?;
            if !self.pending() {
                return Ok(readings);
            }
            std::thread::sleep(POLL_INTERVAL);
        }
    }

    /// Whether the reading of the frame `through` names is yet to arrive: its counts wait
    /// for a staging buffer, or its mapping's callback has not fired. Whatever counted the
    /// frame and returned without waiting for its mapping leaves it so.
    #[cfg(test)]
    pub(crate) fn awaits(&mut self, through: u64) -> bool {
        self.readback.awaits(|measured| measured.through == through)
    }
}

/// Records copying the ticks of the count that wrote the counts, resolved after it, and
/// the counts buffer's first words, `bytes` in all with the ticks, into `staging`.
fn copy(
    encoder: &mut wgpu::CommandEncoder,
    timer: Option<&GpuTimer>,
    counts: &wgpu::Buffer,
    staging: &wgpu::Buffer,
    bytes: u64,
) {
    if let Some(timer) = timer {
        encoder.copy_buffer_to_buffer(timer.resolved(), 0, staging, 0, TICKS_BYTES);
    }
    encoder.copy_buffer_to_buffer(counts, 0, staging, TICKS_BYTES, bytes - TICKS_BYTES);
}

/// The kernel's `ScopesParams`: the graphs to count, then the vectorscope's plane.
fn params(graphs: Graphs) -> Vec<u8> {
    let plane = Plane::get();
    let flags = [graphs.histogram, graphs.waveform, graphs.vectorscope, false].map(u32::from);
    let mut bytes: Vec<u8> = flags.iter().flat_map(|word| word.to_le_bytes()).collect();
    let rows = plane.profile.to_xyz.map(|[r, g, b]| [r, g, b, 0.0]);
    let scalars = [plane.profile.top, plane.profile.exponent, plane.radius, 0.0];
    for value in rows.iter().flatten().chain(&scalars) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

/// darktable's D65 for its built-in RGB profiles, `D65xyY`, as colorout's sRGB has it.
const PROFILE_D65: [f64; 2] = [0.31271, 0.32902];
/// D50 in CIE 1976 u'v', as `dt_xyY_to_Luv` has it.
#[allow(clippy::excessive_precision)]
const D50_UV: [f32; 2] = [0.20915914598542354, 0.488075320769787];
/// darktable's XYZ D50 to linear sRGB (`dt_XYZ_to_Rec709_D50`), by rows.
#[allow(clippy::excessive_precision)]
const XYZ_TO_SRGB: Mat3 = [
    [3.1338561, -1.6168667, -0.4906146],
    [-0.9787684, 1.9161415, 0.0334540],
    [0.0719453, -0.2289914, 1.4052427],
];
/// The hue ring's corners, the RGB cube's that are neither black nor white: red, yellow,
/// green, cyan, blue, magenta.
const CORNERS: [[f32; 3]; 6] = [
    [1.0, 0.0, 0.0],
    [1.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 1.0, 1.0],
    [0.0, 0.0, 1.0],
    [1.0, 0.0, 1.0],
];
/// The ring's samples along each edge between two corners, `VECTORSCOPE_HUES`.
const HUES: usize = 48;

/// darktable's default vectorscope plane (`src/libs/scopes/vectorscope.c`): CIE
/// 1976 u*v* against D50 of the frame read through sRGB, darktable's default histogram
/// profile, which is how the frame is encoded, so darktable's conversion into it from its
/// display profile has nothing to do; on darktable's logarithmic scale, its default too;
/// the grid's edge at the most saturated colour of sRGB's hue ring; and each cell's
/// colour, as darktable paints the plane under its counts. Made once: the plane never
/// changes.
struct Plane {
    profile: Profile,
    /// The hue ring's largest chroma: from neutral to the grid's edge.
    radius: f32,
    /// Each cell's colour, laid out as the counts.
    colours: Arc<[[u8; 3]]>,
}

impl Plane {
    fn get() -> &'static Plane {
        static PLANE: OnceLock<Plane> = OnceLock::new();
        PLANE.get_or_init(Plane::new)
    }

    /// darktable `_lib_histogram_vectorscope_bkgd` for u*v*: the hue ring, traced along
    /// the RGB cube's six edges that touch neither black nor white, 48 samples an edge;
    /// its largest chroma; and the background darktable paints from it, a triangle from
    /// neutral to each pair of neighbouring samples, every sample but the first moved out
    /// to the largest chroma so far, its colour going from one sample's to the other's
    /// along the triangle's outer edge, so the same along each ray from neutral. A cell
    /// takes the colour of the background's pixel at the same place, as darktable draws
    /// the counts over it pixel for pixel, its 8-bit value rounded.
    fn new() -> Self {
        let profile = Profile::srgb();
        let mut radius = 0.0f32;
        // Each sample's corner in the background's mesh, and its colour.
        let mut ring: Vec<([f32; 2], [f32; 3])> = Vec::with_capacity(6 * HUES);
        for (k, from) in CORNERS.iter().enumerate() {
            let to = CORNERS[(k + 1) % CORNERS.len()];
            let delta: [f32; 3] = std::array::from_fn(|c| (to[c] - from[c]) / HUES as f32);
            for i in 0..HUES {
                let rgb = std::array::from_fn(|c| from[c] + delta[c] * i as f32);
                let xyz = profile.xyz(rgb);
                let [u, v] = luv(xyz);
                let h = u.hypot(v);
                radius = radius.max(h);
                // darktable's crude gamut clipping: the colour over its largest channel.
                let display = apply(&XYZ_TO_SRGB, xyz);
                let most = display[0].max(display[1]).max(display[2]);
                let colour = display.map(|c| c / most);
                let corner = if ring.is_empty() || h < f32::MIN_POSITIVE {
                    [u, v]
                } else {
                    [u * (radius / h), v * (radius / h)]
                };
                ring.push((corner, colour));
            }
        }
        // darktable's background is a pixel per cell, neutral at its centre, half a cell
        // from where the counts put neutral.
        let centre = CELLS as f32 / 2.0;
        let mut colours = vec![[0u8; 3]; VECTORSCOPE_WORDS];
        for u in 0..CELLS {
            for v in 0..CELLS {
                let towards = [u as f32 + 0.5 - centre, v as f32 + 0.5 - centre];
                let colour = background(&ring, towards);
                colours[cell(u, v)] = colour.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8);
            }
        }
        Plane {
            profile,
            radius,
            colours: colours.into(),
        }
    }
}

/// Where the counts put darktable's bin `u` across u* and `v` across v*: the vectorscope
/// turned 270°, darktable's default, so u* points up and v* to the left.
fn cell(u: usize, v: usize) -> usize {
    (CELLS - 1 - u) * CELLS + (CELLS - 1 - v)
}

/// The colour darktable's background paints in direction `towards` from neutral. cairo
/// paints the mesh's triangles in order, so the last that the ray crosses; its colour
/// that of the point where the ray crosses its outer edge, between the colours of the
/// samples at the edge's ends.
fn background(ring: &[([f32; 2], [f32; 3])], towards: [f32; 2]) -> [f32; 3] {
    let cross = |a: [f32; 2], b: [f32; 2]| a[0] * b[1] - a[1] * b[0];
    for (k, &(start, from)) in ring.iter().enumerate().rev() {
        let (end, to) = ring[(k + 1) % ring.len()];
        let edge = [end[0] - start[0], end[1] - start[1]];
        let across = cross(edge, towards);
        if across == 0.0 {
            continue;
        }
        // The ray reaches start + t × edge at s × towards.
        let t = -cross(start, towards) / across;
        let s = -cross(start, edge) / across;
        if (0.0..=1.0).contains(&t) && s > 0.0 {
            return std::array::from_fn(|c| from[c] * (1.0 - t) + to[c] * t);
        }
    }
    [0.0; 3]
}

/// sRGB as darktable's vectorscope reads it into XYZ D50 (`dt_ioppr_rgb_matrix_to_xyz`),
/// and as its colour picker reads a sample into Lab (`_transform_rgb_to_lab_matrix`,
/// [`crate::picker`]): darktable's default display and histogram profile.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Profile {
    /// Linear RGB to XYZ D50, as lcms builds darktable's sRGB profile.
    to_xyz: Mat3,
    /// darktable's power law for a value at or above 1, `top × v^exponent`.
    top: f32,
    exponent: f32,
}

impl Profile {
    /// darktable's sRGB as `_ioppr_generate_profile_info` reads it: lcms's RGB to XYZ D50,
    /// and for values at and above 1 `dt_iop_estimate_exp` on its curve's table at 0.7,
    /// 0.8, 0.9 and 1 (`_init_unbounded_coeffs`): `top` the curve at 1, `exponent` the mean
    /// of the other three's. The table samples the curve; this evaluates it.
    pub(crate) fn srgb() -> Self {
        let x = [0.7f32, 0.8, 0.9, 1.0];
        let y = x.map(srgb_to_linear);
        let (x0, y0) = (x[3], y[3]);
        let mut sum = 0.0f32;
        let mut count = 0;
        for k in 0..3 {
            let (yy, xx) = (y[k] / y0, x[k] / x0);
            if yy > 0.0 && xx > 0.0 {
                sum += yy.ln() / xx.ln();
                count += 1;
            }
        }
        Profile {
            to_xyz: to_f32(profile_rgb_to_xyz_d50(PROFILE_D65, SRGB_PRIMARIES)),
            top: y0,
            exponent: if count > 0 {
                sum * (1.0 / count as f32)
            } else {
                1.0
            },
        }
    }

    /// darktable `dt_ioppr_apply_trc`: the curve below 1, 0 below 0 where its table
    /// clamps; the power law at and above 1.
    fn linear(self, v: f32) -> f32 {
        if v < 1.0 {
            srgb_to_linear(v.max(0.0))
        } else {
            self.top * v.powf(self.exponent)
        }
    }

    pub(crate) fn xyz(self, rgb: [f32; 3]) -> [f32; 3] {
        apply(&self.to_xyz, rgb.map(|v| self.linear(v)))
    }
}

/// sRGB's curve inverted, as the kernel computes it.
fn srgb_to_linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// darktable `dt_D50_XYZ_to_xyY`, `dt_xyY_to_uvY` and `dt_xyY_to_Luv`: CIE 1976 u*v*
/// against D50 of a colour in XYZ D50, black taking D50's chromaticity.
fn luv(xyz: [f32; 3]) -> [f32; 2] {
    let [x, y, z] = xyz.map(|v| v.max(0.0));
    let sum = x + y + z;
    let [cx, cy] = if sum > 0.0 {
        [x / sum, y / sum]
    } else {
        D50_XY
    };
    let denominator = -2.0 * cx + 12.0 * cy + 3.0;
    let (u, v) = (4.0 * cx / denominator, 9.0 * cy / denominator);
    let edge = (6.0f32 / 29.0).powi(3);
    let lstar = if y <= edge {
        (29.0f32 / 3.0).powi(3) * y
    } else {
        116.0 * y.cbrt() - 16.0
    };
    [
        13.0 * lstar * (u - D50_UV[0]),
        13.0 * lstar * (v - D50_UV[1]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_uniforms_fill_the_kernels_struct() {
        let graphs = Graphs {
            histogram: false,
            waveform: true,
            vectorscope: true,
        };
        let bytes = params(graphs);
        assert_eq!(bytes.len(), PARAMS_BYTES);
        let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        assert_eq!([word(0), word(4), word(8), word(12)], [0, 1, 1, 0]);
        let float = |at: usize| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        let plane = Plane::get();
        assert_eq!(float(16 + 4), plane.profile.to_xyz[0][1]);
        assert_eq!(float(64 + 8), plane.radius);
    }

    #[test]
    fn the_plane_is_darktables_srgb_hue_ring() {
        let plane = Plane::get();
        // White is neutral, and the power law above 1 continues the curve.
        let [u, v] = luv(plane.profile.xyz([1.0; 3]));
        assert!(u.abs() < 0.05 && v.abs() < 0.05, "{u} {v}");
        assert_eq!(plane.profile.top, 1.0);
        assert!((2.2..2.3).contains(&plane.profile.exponent));
        // The grid reaches the most saturated colour, red in u*v*.
        let [u, v] = luv(plane.profile.xyz([1.0, 0.0, 0.0]));
        assert_eq!(plane.radius, u.hypot(v));
        // The cells under the primaries take their hues: red at the top's middle, blue at
        // the right's.
        let colour = |row: usize, column: usize| plane.colours[row * CELLS + column];
        let [r, g, b] = colour(0, CELLS / 2);
        assert!(r == 255 && g < 128 && b < 128, "{r} {g} {b}");
        let [r, g, b] = colour(CELLS / 2, CELLS - 1);
        assert!(b == 255 && r < 128 && g < 128, "{r} {g} {b}");
    }
}
