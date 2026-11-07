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

//! The operation trait, and what `prepare` gives the engine.

use crate::{
    Extent, ImageContext, ImageDesc, Kernel, Manifest, OutputProfile, ParamValues, Schema,
};

/// The contract between the engine and an operation (architecture decisions 20 and 21). The engine plans and runs the pipeline through it alone.
pub trait Operation: Send + Sync {
    fn manifest(&self) -> &'static Manifest;

    fn schema(&self) -> &'static Schema;

    /// The WGSL whose entry points the operation's passes run.
    fn kernel(&self) -> &'static Kernel;

    /// The parameter values an unedited image starts from: the schema's defaults,
    /// unless they depend on the image, as a raw's crop and levels, its as-shot white
    /// balance or its CFA do.
    fn defaults(&self, _image: &ImageContext) -> ParamValues {
        self.schema().defaults()
    }

    /// Turns parameter values into the passes to dispatch on `input`, with their
    /// uniforms, tables and scratch buffers, and the image they produce, as darktable's
    /// `commit_params` does. `params` hold a value for every parameter in the schema,
    /// checked against it ([`Schema::resolve`]).
    fn prepare(
        &self,
        params: &ParamValues,
        image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<Prepared, NotPorted>;

    /// The profile `params` write the pipe's output in, as colorout reports its `type`'s
    /// (decision 26): the planner asks every instance before any `prepare` and hands the
    /// last answer to every `prepare` as [`ImageContext::output_profile`]. `None`, the
    /// default, for an operation that does not write the output. One that does answers
    /// for a profile without a matrix too, with sRGB's, as darktable does.
    fn output_profile(&self, _params: &ParamValues) -> Option<OutputProfile> {
        None
    }

    /// Whether an export may run the whole-image passes `params` prepare after demosaic
    /// in a first round of tiles (decision 15): every one of them that reads the input
    /// writes only the part of its scratch that the tile it runs in owns, so the tiles
    /// together build what one run over the whole image would. `false`, the default,
    /// runs the operations up to it over the whole image instead.
    fn tiles_whole_image_passes(&self, _params: &ParamValues) -> bool {
        false
    }

    /// Whether the viewport computes the whole-image results `params` prepare at full
    /// resolution (decision 15), for an operation whose estimate is not scale-invariant,
    /// as haze removal's is: where it can, the planner then gives `prepare` the
    /// full-resolution image as [`ImageContext::full_resolution`], which its whole-image
    /// passes bind as [`Resource::FullResolution`], and the operation downscales what its
    /// later passes read per pixel by the viewport's own box filter. An export at a size
    /// without high quality resampling resamples after the last such operation, which
    /// so runs at full resolution there (decision 15). `false`, the default, computes
    /// them at the pipe's scale, as every pipe does.
    fn whole_image_at_full_resolution(&self, _params: &ParamValues) -> bool {
        false
    }

    /// How the operation moves pixels with `params` (decision 16); `None`, the default,
    /// for one that leaves every pixel where it is. An operation that returns one runs
    /// after the raw stage, and its `prepare` gets the input region its
    /// [`Geometry::input_region`] asked for and returns an image of the output region's
    /// size, or no passes at all when its map is a translation of that region, which then
    /// is its output, its origin moved. The planner places either in its stage.
    fn geometry(&self, _params: &ParamValues, _image: &ImageContext) -> Option<Box<dyn Geometry>> {
        None
    }
}

/// How an operation that moves pixels maps its image (decision 16): darktable's four
/// hooks (`src/develop/imageop.h`) as pure CPU functions, in f32 where they compute, so
/// that sizes match darktable's. The planner sizes the stages and works regions backward
/// through them; the operation's kernel evaluates the same map per pixel. A region is x,
/// y, width and height; a point (x, y), pixel (i, j) covering i to i + 1 and j to j + 1.
pub trait Geometry: Send + Sync {
    /// Its output's extent at full resolution, from its input's: darktable's
    /// `modify_roi_out` at scale 1.
    fn extent(&self, input: [u32; 2]) -> [u32; 2];

    /// The region of `input` at the pipe's scale that `output`, a region of its output
    /// at that scale, needs, its interpolator's margin included and clipped to `input`:
    /// darktable's `modify_roi_in`.
    fn input_region(&self, input: &Extent, output: [u32; 4]) -> [u32; 4];

    /// Where a point of its input, at full resolution, lies in its output, or `None`
    /// where it has no image there, as outside a crop: darktable's `distort_transform`.
    /// `input` is the input's extent.
    fn forward(&self, input: [u32; 2], point: [f32; 2]) -> Option<[f32; 2]>;

    /// Where a point of its output comes from in its input: darktable's
    /// `distort_backtransform`.
    fn backward(&self, input: [u32; 2], point: [f32; 2]) -> Option<[f32; 2]>;

    /// Whether it is a permutation of whole pixels with no interpolation, a quarter turn
    /// or a mirror, its `extent` its own inverse: an export at a size may then resample
    /// on either side of it, in the stage its place gives (decision 16). `false`, the
    /// default, for a map that interpolates or crops.
    fn whole_pixels(&self) -> bool {
        false
    }
}

/// What an operation runs, and what it produces.
#[derive(Debug, Clone, PartialEq)]
pub struct Prepared {
    /// The image the operation writes to its output.
    pub output: ImageDesc,
    /// The passes, in order. Each dispatches one invocation per output pixel.
    pub passes: Vec<Pass>,
    /// Buffers the passes share, each sized to the output image.
    pub scratch: Vec<Scratch>,
}

impl Prepared {
    /// One per-pixel pass whose output has the input's size and format.
    pub fn per_pixel(input: &ImageDesc, params: Vec<u8>, tables: Vec<Vec<u8>>) -> Self {
        Self {
            output: *input,
            passes: vec![Pass::per_pixel(params, tables)],
            scratch: Vec::new(),
        }
    }
}

/// One dispatch of one entry point of the operation's kernel.
#[derive(Debug, Clone, PartialEq)]
pub struct Pass {
    pub entry_point: &'static str,
    pub kind: PassKind,
    /// The uniform block after the pass header, as little-endian bytes.
    pub params: Vec<u8>,
    /// Read-only tables, bound where `bindings` says.
    pub tables: Vec<Vec<u8>>,
    /// Exactly the resources the entry point uses: wgpu refuses a bind group with more.
    pub bindings: Vec<Binding>,
    /// The pixels the pass runs over, one invocation each, width and height: the
    /// operation's output when `None`. A demosaic's passes run over its input, larger
    /// than the declared crop it writes (decision 21).
    pub size: Option<[u32; 2]>,
    /// How the engine may run the pass in row strips, when its input or output is
    /// larger than the GPU binds at once (decision 14); `None` when it needs its images
    /// whole, which then must fit. A pass that binds neither image, only scratch, may
    /// take strips too: it runs in bands of its rows wherever its operation's output
    /// would, each as many rows as a strip binding that image's rows has.
    pub strips: Option<Strips>,
}

impl Pass {
    /// A per-pixel pass of the kernel's `main` entry point, bound as every kernel's
    /// `main` is: input at 0, output at 1, uniforms at 2, tables from 3.
    pub fn per_pixel(params: Vec<u8>, tables: Vec<Vec<u8>>) -> Self {
        let mut bindings = vec![
            Binding::new(0, Resource::Input),
            Binding::new(1, Resource::Output),
            Binding::new(2, Resource::Uniforms),
        ];
        bindings.extend((0..tables.len()).map(|i| Binding::new(3 + i as u32, Resource::Table(i))));
        Self {
            entry_point: "main",
            kind: PassKind::PerPixel,
            params,
            tables,
            bindings,
            size: None,
            strips: None,
        }
    }
}

/// How a pass splits into row strips (decision 14). Each strip is one dispatch over a
/// band of the pass's rows of invocations, with a window of its input and output
/// buffers bound: the rows the strip reads and writes. The kernel adds the header's
/// origin to its invocation's row and indexes the whole image less the header's
/// `input_base` and `output_base`, so a strip computes exactly what the whole image
/// would. Scratch buffers stay whole: a pass that binds only scratch, as highlights'
/// dilation of its masks, is a band of its rows with no window, which its kernel finds
/// from the header's origin alone, and its `rows` say how many rows of its operation's
/// output a row of its invocations stands for, so its bands are as large as that
/// image's strips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Strips {
    /// Rows of the operation's output image each row of invocations covers: 1 for a pass
    /// per photosite, more for a pass that adds up blocks of them. The rows of its input
    /// lie where the two images' sensor origins put them.
    pub rows: u32,
    /// A strip's rows of invocations are a multiple of this: the workgroup's 16 for a
    /// pass that writes a partial result per workgroup at its index, so each workgroup
    /// adds up what it does over the whole image.
    pub granularity: u32,
    /// Rows of its input above and below a strip's own that the pass reads.
    pub halo: u32,
}

impl Strips {
    /// A pass that reads and writes only its own photosite.
    pub const PER_PIXEL: Self = Self {
        rows: 1,
        granularity: 1,
        halo: 0,
    };
}

/// A resource at one binding of group 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Binding {
    pub binding: u32,
    pub resource: Resource,
}

impl Binding {
    pub fn new(binding: u32, resource: Resource) -> Self {
        Self { binding, resource }
    }
}

/// What a binding holds. By convention, input is binding 0, output 1, uniforms 2,
/// tables 3 to 7, derivatives 8 and 9, and scratch buffers 10 and up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Resource {
    Input,
    Output,
    Uniforms,
    /// The pass's table with this index.
    Table(usize),
    /// The operation's scratch buffer with this index.
    Scratch(usize),
    /// In the viewport, the full-resolution image its downscale read, for the
    /// whole-image passes of an operation whose estimate the viewport computes at full
    /// resolution ([`Operation::whole_image_at_full_resolution`], decision 15), which
    /// `prepare` binds only while [`ImageContext::full_resolution`](crate::ImageContext)
    /// gives it.
    FullResolution,
}

/// A buffer an operation's passes share. Its size is the operation's to choose: a
/// demosaic's planes have its input's size, while its output is the smaller declared
/// crop (decision 21).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scratch {
    pub bytes: u64,
}

impl Scratch {
    /// `bytes_per_pixel` for every pixel of `image`.
    pub fn per_pixel(image: &ImageDesc, bytes_per_pixel: u32) -> Self {
        Self {
            bytes: u64::from(image.width) * u64::from(image.height) * u64::from(bytes_per_pixel),
        }
    }
}

/// What a pass reads besides the pixel it writes (`docs/engine.md`, Regions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PassKind {
    /// Only that pixel, so the engine can fuse it with its neighbours.
    PerPixel,
    /// Pixels within this radius, at full resolution.
    Spatial { radius: u32 },
    /// Part of computing a statistic of the whole image into the operation's scratch
    /// buffers, where its later passes read it with no readback: the passes that
    /// prepare its input, those writing a partial result per workgroup, and the one
    /// combining them (WGSL has no float atomics). They cover the whole image (decision
    /// 15), in row strips when it is larger than the GPU binds at once, each workgroup
    /// writing its partial result at its index in the whole image, so the statistic is
    /// the whole image's (decision 14). The viewport keeps their result while their
    /// input and their own uniforms and tables stay the same, and runs only the passes
    /// after them (decision 15), so none of them writes the output, and a pass after
    /// them writes only scratch they do not write.
    Reduction,
}

/// A setting combination the port does not cover yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{operation}: {what} is not ported yet")]
pub struct NotPorted {
    pub operation: &'static str,
    pub what: &'static str,
}
