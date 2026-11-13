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
//!   and maximum of four channels, of an RGBA buffer, denoised by darktable's B-spline
//!   where asked, or of a mosaic per CFA colour, its sums a fixed tree, and reads the
//!   results back through the staging pair the scopes share (`readback.rs`), never
//!   waiting: 48 bytes a box. A buffer larger than one binding is read in row windows
//!   (decision 14).
//! - **The frame's reading** ([`FrameReading`]) is the frame's RGB, as the window shows
//!   it, and each statistic in Lab D50, as darktable's color picker utility reads its
//!   samples on the display-referred frame.
//! - **A module's reading** ([`ModuleReading`]) is its operation's input and output
//!   statistics in the live viewport pipe, and what the module's pick showed of them.
//!
//! Built: the Colour picker module's picker, measured on the frame, and module pickers,
//! measured at their operation and applied by the session. Stored samples and the spaces
//! other than the operation's own are to come.

use std::num::NonZeroU64;
use std::time::Instant;

use apertrust_modules::ControlValue;
pub use apertrust_modules::pick::{PickKeys, Reading, Region, Shown, Stats};
use apertrust_plugin::{Format, ImageContext, ImageDesc, Kernel, PassHeader};

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

/// The boxes one reduction takes, across its buffers.
pub const MAX_BOXES: usize = 64;
/// The buffers one reduction reads: a module picker's operation's input and output.
const MAX_GROUPS: usize = 2;
/// The pixels a workgroup reduces across and down: 16 invocations each adding 4, 16 apart.
const TILE: u32 = 64;
/// A partial and a result in the kernel: a sum or a mean, a minimum and a maximum, each of
/// four channels.
const PARTIAL_BYTES: u64 = 48;
/// `PickerParams` in the kernel: the count, the dispatch's workgroups across, where they
/// end and begin, the first box's place in the results and three words unused; the
/// boxes, 32 bytes each; then a mosaic's pattern and its colours, 16 and 144 bytes.
const PARAMS_BYTES: usize = 32 + MAX_BOXES * 32 + 16 + 144;
/// The positions of the largest colour pattern the kernel holds: X-Trans's 6×6.
const MAX_PATTERN: usize = 36;
/// Each pass's start and end ticks, before the results in a staging buffer: a reduction
/// or a mosaic's and a combination, for each buffer.
const TICKS_BYTES: u64 = 2 * MAX_GROUPS as u64 * 16;
/// The rows of halo a denoised box reads above and below it: the B-spline's reach.
const DENOISE_HALO: u32 = 2;

/// darktable's default point, the photo's centre (`dt_lib_colorpicker_reset_point`).
pub const DEFAULT_POINT: Region = Region::Point([0.5, 0.5]);
/// darktable's default area, 2 % to 98 % of the photo on each axis
/// (`dt_lib_colorpicker_reset_box_area`).
pub const DEFAULT_AREA: Region = Region::Area([0.02, 0.02, 0.98, 0.98]);

/// A picker, by its owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PickerRef {
    /// A module's picker, by its module's id and the picker's id in its definition
    /// (`Definition::pickers`).
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
    /// Turns `picker` on, the one on before it off: an area with `area` where the picker
    /// takes both, else a point; a picker of one kind, that kind. It starts at the
    /// picker's last point or area this session, else darktable's default. A module's
    /// picker turns its module on, makes its operation the edited instance, and has its
    /// first reading applied as one completed change; `tools` are the tool settings the
    /// module's pickers read, as the client keeps them, and `channel` the channel tab the
    /// module shows. Refused before the photo is live, and for a module's picker on a
    /// photo open for viewing only.
    On {
        picker: PickerRef,
        area: bool,
        keys: PickKeys,
        tools: Vec<(&'static str, ControlValue)>,
        channel: Option<&'static str>,
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
    /// The tool settings of `module`'s pickers changed, as the client keeps them: kept,
    /// and the module's last reading applied again where the picker that read it reads
    /// one of them, as one completed change, the tool settings that pick measures never
    /// handed back (`Shown::tools` empty), since the client has just sent its own.
    /// Refused for a setting none of its pickers reads.
    Tools {
        module: &'static str,
        tools: Vec<(&'static str, ControlValue)>,
    },
    /// The channel tab the active picker's module shows changed: kept for its next pick,
    /// applying nothing, as darktable's tab switch applies nothing; with no picker on,
    /// nothing.
    Channel { channel: Option<&'static str> },
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
    /// A module picker's statistics at its operation; `None` for the Colour picker's, and
    /// until the first arrives.
    pub module: Option<ModuleReading>,
    /// The frame's colour under the region; `None` until measured, or where the region
    /// misses the frame.
    pub frame: Option<FrameReading>,
}

/// What a module's picker measured at its operation, darktable's `picked_color`,
/// `picked_color_min` and `picked_color_max` of its input and output, each in the
/// picker's space.
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleReading {
    /// `None` where the region misses the operation's image, as darktable's empty box.
    pub input: Option<Stats>,
    /// With the picker's `output` only.
    pub output: Option<Stats>,
    /// What the module's pick made of the last reading it applied, for its panel; `None`
    /// where it made nothing.
    pub picked: Option<Shown>,
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

/// The box `region`, in its stage's coordinates (0 to 1 across that stage's image at full
/// resolution), covers in the buffer `desc` describes: an RGBA image at the pipe's scale,
/// its stage's whole image or a region of it at its origin; or a mosaic, the developed
/// image's photosites on the sensor, as `image`'s camera places its declared area, read
/// from the mosaic's own corner. darktable's box, as [`pixels`] makes it, then held to
/// the buffer; `None` where it misses the buffer.
pub(crate) fn boxed(region: Region, desc: &ImageDesc, image: &ImageContext) -> Option<[u32; 4]> {
    let ([left, top, right, bottom], corner) = match desc.format {
        Format::Rgba => {
            let bounds = pixels(region, desc.scaled_extent)?;
            (bounds, desc.origin.map(i64::from))
        }
        Format::Mosaic | Format::Mosaic16 => {
            let bounds = pixels(region, [image.width, image.height])?;
            let declared = image
                .camera
                .as_ref()
                .map_or([0, 0], |camera| [camera.declared[0], camera.declared[1]]);
            let corner = [0, 1].map(|a| i64::from(desc.sensor_origin[a]) - i64::from(declared[a]));
            (bounds, corner)
        }
    };
    let held = |v: u32, a: usize, side: u32| (i64::from(v) - corner[a]).clamp(0, i64::from(side));
    let bounds = [
        held(left, 0, desc.width),
        held(top, 1, desc.height),
        held(right, 0, desc.width),
        held(bottom, 1, desc.height),
    ];
    (bounds[0] < bounds[2] && bounds[1] < bounds[3]).then(|| bounds.map(|v| v as u32))
}

/// A mosaic's colour filter array as the reduction reads it: darktable's `filters` or
/// X-Trans pattern, at the mosaic's place on the sensor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cfa {
    /// The pattern's width and height: 2 × 2 for Bayer, 6 × 6 for X-Trans.
    pub size: [u32; 2],
    /// Each position's colour, row by row: 0 red, 1 green, 2 blue.
    pub colours: Vec<u32>,
    /// The mosaic's top-left photosite on the sensor.
    pub origin: [u32; 2],
}

impl Cfa {
    /// The pattern of `image`'s camera, for the mosaic `desc` describes; `None` for an
    /// image with no colour filter array or a pattern larger than X-Trans's.
    pub fn of(image: &ImageContext, desc: &ImageDesc) -> Option<Self> {
        let cfa = image.camera.as_ref()?.cfa.as_ref()?;
        let positions = (cfa.width * cfa.height) as usize;
        (positions > 0 && positions <= MAX_PATTERN && cfa.values.len() == positions).then(|| Self {
            size: [cfa.width, cfa.height],
            colours: cfa.values.clone(),
            origin: desc.sensor_origin,
        })
    }

    /// How many photosites of each colour the box `bounds` holds, darktable's `weights`.
    pub fn counts(&self, [left, top, right, bottom]: [u32; 4]) -> [u32; 4] {
        // Of the columns or rows from `from` to `to`, those at each position of the
        // pattern along an axis.
        let along = |from: u32, to: u32, axis: usize| {
            let side = self.size[axis];
            let mut at = vec![0u32; side as usize];
            let first = (self.origin[axis] + from) % side;
            for (i, count) in at.iter_mut().enumerate() {
                let offset = (i as u32 + side - first) % side;
                *count = (to - from).saturating_sub(offset).div_ceil(side);
            }
            at
        };
        let (columns, rows) = (along(left, right, 0), along(top, bottom, 1));
        let mut counts = [0u32; 4];
        for (y, &down) in rows.iter().enumerate() {
            for (x, &across) in columns.iter().enumerate() {
                let colour = self.colours[y * self.size[0] as usize + x] as usize;
                if let Some(count) = counts.get_mut(colour) {
                    *count += down * across;
                }
            }
        }
        counts
    }
}

/// How a reduction reads its buffer.
#[derive(Debug, Clone, Copy)]
pub enum Layout<'a> {
    /// `f32` RGBA, four channels a pixel, each pixel first blurred by darktable's
    /// B-spline with `denoise` (`DT_COLOR_PICKER_DENOISE`).
    Rgba { denoise: bool },
    /// A mosaic, one `f32` a photosite, read per CFA colour.
    Mosaic(&'a Cfa),
}

/// A buffer a reduction reads, and its boxes.
#[derive(Debug, Clone, Copy)]
pub struct Group<'a> {
    pub buffer: &'a wgpu::Buffer,
    /// Its image's width and height in pixels, or photosites.
    pub size: [u32; 2],
    pub layout: Layout<'a>,
    /// Left, top, right and bottom, right and bottom exclusive, never empty and within the
    /// image, as [`pixels`] gives them.
    pub boxes: &'a [[u32; 4]],
}

impl Group<'_> {
    /// Its image's bytes a row.
    fn row_bytes(&self) -> u64 {
        let channels = match self.layout {
            Layout::Rgba { .. } => 4,
            Layout::Mosaic(_) => 1,
        };
        u64::from(self.size[0]) * channels * 4
    }

    /// The rows its boxes read beyond their own: the B-spline's reach where denoised.
    fn halo(&self) -> u32 {
        match self.layout {
            Layout::Rgba { denoise: true } => DENOISE_HALO,
            _ => 0,
        }
    }

    /// What each box's sum is divided by, per channel: its pixels, or each colour's
    /// photosites in it.
    fn divisors(&self) -> impl Iterator<Item = [u32; 4]> + '_ {
        self.boxes.iter().map(
            move |&bounds @ [left, top, right, bottom]| match self.layout {
                Layout::Rgba { .. } => [(right - left) * (bottom - top); 4],
                Layout::Mosaic(cfa) => cfa.counts(bounds),
            },
        )
    }
}

/// The bytes the session's two reducers hold at most for a viewport whose frame is at
/// most `viewport` and whose pickers may read a mosaic of `mosaic` photosites: their
/// uniforms, results and staging pairs, the frame's partials for [`MAX_BOXES`] boxes over
/// the whole frame, and a module picker's for one box over the larger of its
/// operation's images, which the reductions of its input and output share; and the
/// uniform blocks, one a dispatch: the frame's one, and for the operation's input and
/// output, read in row windows at worst a window a row of workgroups. The plan's memory
/// counts them (`docs/pickers.md`, Memory, strips and failures).
pub fn reserved(viewport: [u32; 2], mosaic: [u32; 2]) -> u64 {
    let workgroups = |[width, height]: [u32; 2]| {
        u64::from(width.div_ceil(TILE)) * u64::from(height.div_ceil(TILE))
    };
    let frame = MAX_BOXES as u64 * workgroups(viewport);
    let operation = workgroups(viewport).max(workgroups(mosaic));
    let partials = (frame + operation) * PARTIAL_BYTES;
    let results = MAX_BOXES as u64 * PARTIAL_BYTES;
    let rows = u64::from(viewport[1].max(mosaic[1]).div_ceil(TILE));
    let blocks = 1 + MAX_GROUPS as u64 * rows;
    let uniforms = blocks * (PassHeader::SIZE + PARAMS_BYTES) as u64;
    partials + uniforms + 2 * (results + 2 * (TICKS_BYTES + results))
}

/// A reduction on its way, with what its owner keeps of it.
struct Measured<T> {
    tag: T,
    /// What each box's sums are divided by, per channel.
    divisors: Vec<[u32; 4]>,
    submitted: Instant,
    /// The passes whose ticks were resolved, so the reading has a GPU time; 0 where the
    /// device has no timestamp queries.
    timed: u32,
}

/// A reduction recorded in an encoder whose owner submits it ([`Reducer::record`]),
/// waiting for [`Reducer::submitted`].
#[must_use = "a reduction recorded is read back only once its submission is told"]
pub(crate) struct Recorded {
    divisors: Vec<[u32; 4]>,
    bytes: u64,
    timed: u32,
    /// The staging buffer its copy fills, or none while both are busy.
    slot: Option<usize>,
}

/// What a reduction gave: each box's statistics, in the order given.
#[derive(Debug, Clone, PartialEq)]
pub struct Reduced<T> {
    /// What its owner gave with it.
    pub tag: T,
    pub stats: Vec<Stats>,
    /// Every pass's GPU time; NaN where the device has no timestamp queries, or gave none.
    pub gpu_ms: f64,
    /// From the reduction's submission to its result being picked up.
    pub latency_ms: f64,
}

/// Reduces boxes of RGBA buffers and mosaics on the GPU and reads the results back
/// through two staging buffers in turn, never waiting: the pickers' GPU side, which the
/// session keeps for its life, one for the frame and one at a module picker's
/// operation. `T` is what its owner keeps of each reduction until it arrives.
pub struct Reducer<T> {
    rgba: ImageKernel,
    mosaic: ImageKernel,
    combine: ImageKernel,
    /// One uniform block a dispatch, grown as a reduction needs them.
    uniforms: Vec<wgpu::Buffer>,
    /// Each workgroup's partial, grown to what a reduction needs.
    partials: wgpu::Buffer,
    results: wgpu::Buffer,
    readback: Readback<Measured<T>>,
    timer: Option<GpuTimer>,
    /// The most workgroups a dispatch takes across: the device's, or fewer, for a test
    /// of the dispatch across and down.
    across: u32,
}

impl<T> Reducer<T> {
    /// Compiles the three entry points and makes the buffers: the results and their
    /// staging pair, 3 KiB each, and the uniforms and partials, grown as a reduction needs
    /// them.
    pub async fn new(gpu: &GpuContext) -> Result<Self, ComputeError> {
        let kernel = |entry| ImageKernel::with_entry_point(gpu, KERNEL.name, KERNEL.wgsl, entry);
        let (rgba, mosaic, combine) = (
            kernel("picker_rgba"),
            kernel("picker_mosaic"),
            kernel("picker_combine"),
        );
        let results_bytes = MAX_BOXES as u64 * PARTIAL_BYTES;
        let results = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("picker results"),
            size: results_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        Ok(Self {
            rgba: rgba.await?,
            mosaic: mosaic.await?,
            combine: combine.await?,
            uniforms: Vec::new(),
            partials: partials(gpu, PARTIAL_BYTES),
            results,
            readback: Readback::new(gpu, "picker", TICKS_BYTES + results_bytes),
            timer: GpuTimer::new(gpu, 2 * MAX_GROUPS as u32),
            across: gpu.device.limits().max_compute_workgroups_per_dimension,
        })
    }

    /// Reduces `boxes` of `frame`, an RGBA image, as [`Self::measure_groups`] does.
    pub fn measure(&mut self, gpu: &GpuContext, frame: &GpuImage, boxes: &[[u32; 4]], tag: T) {
        let size = [frame.width(), frame.height()];
        self.measure_buffer(gpu, frame.buffer(), size, boxes, tag);
    }

    /// Reduces `boxes` of the RGBA image in `buffer`, of `size` pixels, with no denoise,
    /// as [`Self::measure_groups`] does.
    pub(crate) fn measure_buffer(
        &mut self,
        gpu: &GpuContext,
        buffer: &wgpu::Buffer,
        size: [u32; 2],
        boxes: &[[u32; 4]],
        tag: T,
    ) {
        let group = Group {
            buffer,
            size,
            layout: Layout::Rgba { denoise: false },
            boxes,
        };
        // The frame always fits one binding: the viewport's plan holds it whole.
        if let Err(error) = self.measure_groups(gpu, &[group], tag) {
            unreachable!("the frame binds whole: {error}");
        }
    }

    /// Reduces each group's boxes, in a submission of its own after everything submitted
    /// before it, and returns without waiting; the result arrives through [`Self::poll`]
    /// with `tag`, every box's statistics in the order given. Refused, with nothing
    /// submitted, where a buffer larger than one binding cannot be read in windows.
    pub fn measure_groups(
        &mut self,
        gpu: &GpuContext,
        groups: &[Group],
        tag: T,
    ) -> Result<(), ComputeError> {
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        let recorded = self.record(gpu, &mut encoder, groups)?;
        gpu.submit([encoder.finish()]);
        self.submitted(recorded, tag);
        Ok(())
    }

    /// Records reducing each group's boxes into `encoder`, every pass after what it holds
    /// so far, then the results' copy into a free staging buffer: for a reduction at an
    /// operation's place in a render's encoder, which its owner submits, then tells
    /// [`Self::submitted`]. At most [`MAX_GROUPS`] groups and [`MAX_BOXES`] boxes in all.
    /// A buffer larger than one binding is bound in row windows, each window's workgroups
    /// writing their partials at their index in the whole box (decision 14); refused,
    /// with nothing recorded, where a window of one workgroup's rows would not bind.
    pub(crate) fn record(
        &mut self,
        gpu: &GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        groups: &[Group],
    ) -> Result<Recorded, ComputeError> {
        let boxes: usize = groups.iter().map(|group| group.boxes.len()).sum();
        assert!(
            (1..=MAX_GROUPS).contains(&groups.len()) && (1..=MAX_BOXES).contains(&boxes),
            "{} buffers and {boxes} boxes, where a reduction takes 1 to {MAX_GROUPS} and 1 \
             to {MAX_BOXES}",
            groups.len()
        );
        let limits = gpu.device.limits();
        let (binding, alignment) = (
            limits.max_storage_buffer_binding_size,
            u64::from(limits.min_storage_buffer_offset_alignment),
        );
        // Each group's dispatches, with their uniforms, laid out before any is recorded,
        // so a refusal records nothing.
        let mut planned = Vec::with_capacity(groups.len());
        let mut needed = 0u64;
        let mut results = 0u32;
        for group in groups {
            let dispatches = dispatches(group, binding, alignment, self.across)?;
            let workgroups = dispatches.iter().map(|d| d.end).max().unwrap_or(0);
            needed = needed.max(u64::from(workgroups) * PARTIAL_BYTES);
            planned.push((group, dispatches, results));
            results += group.boxes.len() as u32;
        }
        if self.partials.size() < needed {
            self.partials = partials(gpu, needed);
        }
        let blocks: usize = planned.iter().map(|(_, d, _)| d.len()).sum();
        while self.uniforms.len() < blocks {
            self.uniforms
                .push(gpu.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("picker uniforms"),
                    size: (PassHeader::SIZE + PARAMS_BYTES) as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
        }

        let entry = |binding, resource| wgpu::BindGroupEntry { binding, resource };
        let mut uniform = 0;
        let mut slot = 0u32;
        let timer = self.timer.as_ref();
        for (group, dispatches, results) in planned {
            let (kernel, binding) = match group.layout {
                Layout::Rgba { .. } => (&self.rgba, 0),
                Layout::Mosaic(_) => (&self.mosaic, 4),
            };
            let first_block = uniform;
            let mut bind_groups = Vec::with_capacity(dispatches.len());
            for dispatch in &dispatches {
                let block = params(group, dispatch, results);
                let buffer = &self.uniforms[uniform];
                gpu.queue.write_buffer(buffer, 0, &block);
                uniform += 1;
                let image = wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: group.buffer,
                    offset: dispatch.window[0],
                    size: NonZeroU64::new(dispatch.window[1]),
                });
                bind_groups.push(gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(kernel_label(group)),
                    layout: &kernel.pipeline().get_bind_group_layout(0),
                    entries: &[
                        entry(binding, image),
                        entry(1, self.partials.as_entire_binding()),
                        entry(2, buffer.as_entire_binding()),
                    ],
                }));
            }
            let combine = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("picker_combine"),
                layout: &self.combine.pipeline().get_bind_group_layout(0),
                entries: &[
                    entry(1, self.partials.as_entire_binding()),
                    entry(2, self.uniforms[first_block].as_entire_binding()),
                    entry(3, self.results.as_entire_binding()),
                ],
            });
            // The reduction, its windows' dispatches in one compute pass, then the
            // combination.
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(kernel_label(group)),
                timestamp_writes: timer.map(|timer| timer.writes(slot)),
            });
            pass.set_pipeline(kernel.pipeline());
            for (dispatch, bind_group) in dispatches.iter().zip(&bind_groups) {
                pass.set_bind_group(0, bind_group, &[]);
                pass.dispatch_workgroups(dispatch.groups[0], dispatch.groups[1], 1);
            }
            drop(pass);
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("picker_combine"),
                timestamp_writes: timer.map(|timer| timer.writes(slot + 1)),
            });
            pass.set_pipeline(self.combine.pipeline());
            pass.set_bind_group(0, &combine, &[]);
            pass.dispatch_workgroups(group.boxes.len() as u32, 1, 1);
            drop(pass);
            slot += 2;
        }
        if let Some(timer) = timer {
            timer.resolve(encoder, slot);
        }
        let bytes = TICKS_BYTES + boxes as u64 * PARTIAL_BYTES;
        let (results, timer) = (&self.results, self.timer.as_ref());
        let slot_used = self
            .readback
            .record(encoder, bytes, |encoder, staging, bytes| {
                copy(encoder, timer, results, staging, bytes)
            });
        Ok(Recorded {
            divisors: groups.iter().flat_map(Group::divisors).collect(),
            bytes,
            timed: if timer.is_some() { slot } else { 0 },
            slot: slot_used,
        })
    }

    /// Tells the reducer that the submission `recorded` was recorded in has been
    /// submitted: its result arrives through [`Self::poll`] with `tag`.
    pub(crate) fn submitted(&mut self, recorded: Recorded, tag: T) {
        let measured = Measured {
            tag,
            divisors: recorded.divisors,
            submitted: Instant::now(),
            timed: recorded.timed,
        };
        self.readback.sent(recorded.slot, measured, recorded.bytes);
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
                let ticks: Vec<u64> = bytemuck::pod_collect_to_vec(&bytes[..TICKS_BYTES as usize]);
                let floats: Vec<f32> = bytemuck::pod_collect_to_vec(&bytes[TICKS_BYTES as usize..]);
                let stats = floats
                    .as_chunks::<12>()
                    .0
                    .iter()
                    .zip(&measured.divisors)
                    .map(|(v, divisors)| Stats {
                        // darktable's mean, `acc / (float)size`, or for a colour no
                        // photosite of the box has, 0; divided here so that it is the
                        // same bits on every GPU, whose division need only be within
                        // 2.5 ulp.
                        mean: std::array::from_fn(|c| match divisors[c] {
                            0 => 0.0,
                            count => v[c] / count as f32,
                        }),
                        min: [v[4], v[5], v[6], v[7]],
                        max: [v[8], v[9], v[10], v[11]],
                    })
                    .collect();
                // Ticks the GPU did not give are no time.
                let passes = &ticks.as_chunks::<2>().0[..measured.timed as usize];
                let gpu_ms = match timer {
                    Some(timer)
                        if measured.timed > 0 && passes.iter().all(|[from, to]| to > from) =>
                    {
                        passes
                            .iter()
                            .map(|&[from, to]| timer.milliseconds(from, to))
                            .sum()
                    }
                    _ => f64::NAN,
                };
                let latency_ms = measured.submitted.elapsed().as_secs_f64() * 1e3;
                tracing::debug!(
                    target: "perf",
                    "picked {} box(es), read back after {latency_ms:.1} ms{}",
                    measured.divisors.len(),
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

    /// The tags of the results still on their way, never those [`Self::discard`] dropped.
    pub(crate) fn on_its_way(&self) -> impl Iterator<Item = &T> {
        self.readback.on_its_way().map(|measured| &measured.tag)
    }

    /// The tag of the result waiting for a staging buffer, if any.
    #[cfg(test)]
    pub(crate) fn waiting(&self) -> Option<&T> {
        self.readback.waiting().map(|measured| &measured.tag)
    }

    /// Has a result waiting for a staging buffer pass on to the one replacing it what
    /// `keep` takes of its tag, as a picker's due application, so it is not lost.
    pub(crate) fn keeping(&mut self, keep: fn(&T, &mut T))
    where
        T: 'static,
    {
        let keep = move |old: &Measured<T>, new: &mut Measured<T>| keep(&old.tag, &mut new.tag);
        self.readback.keeping(Box::new(keep));
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

    /// At most `across` workgroups across a dispatch, as a device that allows fewer
    /// dispatches them, so a test reaches the dispatch across and down.
    #[cfg(test)]
    pub(crate) fn cap_across(&mut self, across: u32) {
        self.across = across;
    }
}

/// One dispatch of a reduction: its workgroups, the window of the buffer it binds and
/// where that window starts in 32-bit words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Dispatch {
    /// Its first workgroup among every box's, and where they end.
    first: u32,
    end: u32,
    /// Its workgroups across and down.
    groups: [u32; 2],
    /// The window's offset and size in bytes.
    window: [u64; 2],
}

/// The dispatches that reduce `group`'s boxes, each workgroup at its index among every
/// box's, at most `limit` across: one over the whole buffer where it binds whole, else,
/// box by box, row strips of whole workgroups, each over the window of the rows it reads,
/// its offset aligned to `alignment`.
fn dispatches(
    group: &Group,
    binding: u64,
    alignment: u64,
    limit: u32,
) -> Result<Vec<Dispatch>, ComputeError> {
    let shape = |first: u32, end: u32, window: [u64; 2]| {
        let count = end - first;
        let across = count.clamp(1, limit);
        Dispatch {
            first,
            end,
            groups: [across, count.div_ceil(across)],
            window,
        }
    };
    let row = group.row_bytes();
    let whole = row * u64::from(group.size[1]);
    let mut first = 0;
    let tiles: Vec<[u32; 2]> = group
        .boxes
        .iter()
        .map(|&[left, top, right, bottom]| {
            [(right - left).div_ceil(TILE), (bottom - top).div_ceil(TILE)]
        })
        .collect();
    if whole <= binding {
        let workgroups = tiles.iter().map(|[across, down]| across * down).sum();
        return Ok(vec![shape(0, workgroups, [0, whole])]);
    }
    // The rows a window may hold, less what aligning its offset down may add.
    let halo = group.halo();
    let rows = (binding.saturating_sub(alignment) / row) as u32;
    let strip = rows.saturating_sub(2 * halo) / TILE;
    if strip == 0 {
        return Err(ComputeError::Binding {
            operation: "picker",
            pass: kernel_label(group),
            bytes: row * u64::from(TILE + 2 * halo) + alignment,
            limit: binding,
        });
    }
    let mut dispatches = Vec::new();
    for (&[_, top, _, bottom], &[across, down]) in group.boxes.iter().zip(&tiles) {
        for from in (0..down).step_by(strip as usize) {
            let to = (from + strip).min(down);
            let read_top = (top + from * TILE).saturating_sub(halo);
            let read_bottom = (top + to * TILE + halo)
                .min(bottom + halo)
                .min(group.size[1]);
            let start = u64::from(read_top) * row / alignment * alignment;
            let end = u64::from(read_bottom) * row;
            dispatches.push(shape(
                first + from * across,
                first + to * across,
                [start, end - start],
            ));
        }
        first += across * down;
    }
    Ok(dispatches)
}

/// The uniform block of `dispatch` of `group`, whose first box's result goes at
/// `results`.
fn params(group: &Group, dispatch: &Dispatch, results: u32) -> Vec<u8> {
    let mut params = Vec::with_capacity(PARAMS_BYTES);
    let words = [
        group.boxes.len() as u32,
        dispatch.groups[0],
        dispatch.end,
        dispatch.first,
        results,
        0,
        0,
        0,
    ];
    let mut push = |word: u32| params.extend_from_slice(&word.to_le_bytes());
    words.into_iter().for_each(&mut push);
    let denoise = matches!(group.layout, Layout::Rgba { denoise: true });
    let mut first = 0u32;
    for &[left, top, right, bottom] in group.boxes {
        let across = (right - left).div_ceil(TILE);
        let count = across * (bottom - top).div_ceil(TILE);
        for word in [
            left,
            top,
            right,
            bottom,
            first,
            across,
            count,
            u32::from(denoise),
        ] {
            push(word);
        }
        first += count;
    }
    params.resize(32 + MAX_BOXES * 32, 0);
    if let Layout::Mosaic(cfa) = group.layout {
        let mut push = |word: u32| params.extend_from_slice(&word.to_le_bytes());
        [cfa.size[0], cfa.size[1], cfa.origin[0], cfa.origin[1]]
            .into_iter()
            .for_each(&mut push);
        for position in 0..MAX_PATTERN {
            push(cfa.colours.get(position).copied().unwrap_or(0));
        }
    }
    params.resize(PARAMS_BYTES, 0);
    let header = PassHeader {
        // The window's start, in 32-bit words of the whole buffer.
        input_base: (dispatch.window[0] / 4) as u32,
        ..PassHeader::whole(group.size[0], group.size[1])
    };
    header.uniform_block(&params)
}

/// The entry point that reduces `group`.
fn kernel_label(group: &Group) -> &'static str {
    match group.layout {
        Layout::Rgba { .. } => "picker_rgba",
        Layout::Mosaic(_) => "picker_mosaic",
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

/// Records copying every pass's ticks, resolved after them, and the results, `bytes` in
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::GpuOptions;

    fn gpu() -> GpuContext {
        pollster::block_on(GpuContext::new(&GpuOptions::default()))
            .expect("a GPU adapter, hardware or software")
    }

    /// An RGBA buffer of `width` × `height` with values that differ in every pixel.
    fn frame(gpu: &GpuContext, width: u32, height: u32) -> GpuImage {
        let pixels: Vec<[f32; 4]> = (0..width * height)
            .map(|i| {
                let v = (i.wrapping_mul(2_654_435_761) >> 8) as f32 / (1 << 24) as f32;
                [v, 1.0 - v, v * 3.0 - 1.0, v * v]
            })
            .collect();
        GpuImage::upload(gpu, width, height, &pixels).expect("the frame uploads")
    }

    fn stats(
        gpu: &GpuContext,
        reducer: &mut Reducer<()>,
        image: &GpuImage,
        boxes: &[[u32; 4]],
    ) -> Vec<Stats> {
        reducer.measure(gpu, image, boxes, ());
        let mut reduced = reducer.wait(gpu).expect("it reads back");
        reduced.pop().expect("a reading").stats
    }

    #[test]
    fn workgroups_dispatched_across_and_down_reduce_as_in_one_row() {
        let gpu = gpu();
        let image = frame(&gpu, 700, 300);
        // 11 × 5 workgroups for the whole buffer, and two more boxes after it.
        let boxes = [[0, 0, 700, 300], [3, 5, 650, 290], [100, 100, 101, 101]];
        let mut reducer = pollster::block_on(Reducer::new(&gpu)).expect("it compiles");
        let one_row = stats(&gpu, &mut reducer, &image, &boxes);
        // At most 7 across: 112 workgroups in 16 rows, the last part idle.
        reducer.cap_across(7);
        let across_and_down = stats(&gpu, &mut reducer, &image, &boxes);
        let bits = |stats: &[Stats]| -> Vec<[[u32; 4]; 3]> {
            stats
                .iter()
                .map(|s| [s.mean, s.min, s.max].map(|v| v.map(f32::to_bits)))
                .collect()
        };
        assert_eq!(bits(&across_and_down), bits(&one_row));
    }

    #[test]
    fn a_colours_photosites_are_counted_wherever_the_box_and_mosaic_lie() {
        // RGGB, the mosaic's corner on an odd column and an even row.
        let bayer = Cfa {
            size: [2, 2],
            colours: vec![0, 1, 1, 2],
            origin: [1, 0],
        };
        let count = |cfa: &Cfa, [left, top, right, bottom]: [u32; 4]| {
            let mut counts = [0u32; 4];
            for y in top..bottom {
                for x in left..right {
                    let [px, py] = [0, 1].map(|a| (cfa.origin[a] + [x, y][a]) % cfa.size[a]);
                    counts[cfa.colours[(py * cfa.size[0] + px) as usize] as usize] += 1;
                }
            }
            counts
        };
        let xtrans = Cfa {
            size: [6, 6],
            colours: (0..36)
                .map(|i| [1, 1, 0, 1, 1, 2][(i * 7 % 36) % 6])
                .collect(),
            origin: [5, 3],
        };
        for cfa in [&bayer, &xtrans] {
            for bounds in [[0, 0, 1, 1], [3, 2, 4, 9], [1, 1, 14, 12], [7, 5, 31, 6]] {
                assert_eq!(cfa.counts(bounds), count(cfa, bounds), "{cfa:?} {bounds:?}");
            }
        }
    }

    #[test]
    fn a_region_maps_to_its_box_in_a_viewport_buffer_and_in_the_mosaic() {
        let mut image = apertrust_modules::testing::image();
        let camera = image.camera.as_mut().unwrap();
        camera.declared = [6, 4, 84, 62];
        let (width, height) = (image.width, image.height);
        // A region of a 40 × 30 buffer at the viewport's scale: its stage's whole image.
        let rgba = ImageDesc {
            scaled_extent: [40, 30],
            ..ImageDesc::rgba(40, 30)
        };
        let area = Region::Area([0.25, 0.5, 0.75, 1.0]);
        assert_eq!(boxed(area, &rgba, &image), Some([10, 15, 30, 30]));
        // The mosaic after rawprepare, its corner at (2, 2) of the sensor: the developed
        // image's photosites lie 4 across and 2 down from it.
        let mosaic = ImageDesc {
            sensor_origin: [2, 2],
            ..ImageDesc::new(92, 68, Format::Mosaic)
        };
        assert_eq!((width, height), (84, 62));
        assert_eq!(boxed(area, &mosaic, &image), Some([25, 33, 67, 64]));
        assert_eq!(boxed(Region::Point([1.0, 0.5]), &mosaic, &image), None);
    }
}
