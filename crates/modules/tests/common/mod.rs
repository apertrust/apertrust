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

//! The bench the modules' invariant tests share: the GPU, a colour raw's image context,
//! the synthetic images (`docs/modules/README.md`, Test images) and a small raw, a bench
//! rendering edits through one pipeline as the session does, and the jump between two
//! frames. Each test file includes it with `mod common;` and defines its own [`Measure`],
//! what its rows read.

use std::sync::OnceLock;

use apertrust_color::{Illuminant, WorkingProfile};
use apertrust_engine::compute::GpuImage;
use apertrust_engine::edit::{self, Edit};
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe, Pipeline};
use apertrust_modules::MODULES;
use apertrust_operations::OPERATIONS;
use apertrust_plugin::{Camera, ImageContext, Pattern, Value};
use golden::{delta_e, synthetic};

pub fn gpu() -> &'static GpuContext {
    static GPU: OnceLock<GpuContext> = OnceLock::new();
    GPU.get_or_init(|| {
        pollster::block_on(GpuContext::new(&GpuOptions::default()))
            .expect("a GPU adapter, hardware or software")
    })
}

pub fn profile() -> &'static WorkingProfile {
    static PROFILE: OnceLock<WorkingProfile> = OnceLock::new();
    PROFILE.get_or_init(WorkingProfile::linear_rec2020)
}

/// A colour raw's image context, `width` by `height` after its crop, with this exposure
/// bias.
pub fn raw_context(width: u32, height: u32, bias: f32) -> ImageContext {
    ImageContext {
        camera: Some(Camera {
            make: String::new(),
            model: String::new(),
            alias: String::new(),
            width,
            height,
            cfa: Some(Pattern {
                width: 2,
                height: 2,
                values: vec![0, 1, 1, 2],
            }),
            black: Pattern {
                width: 1,
                height: 1,
                values: vec![0.0],
            },
            white: 4000.0,
            white_balance: Some(vec![2.0, 1.0, 1.5]),
            xyz_to_camera: Some([
                [3.2406, -1.5372, -0.4986],
                [-0.9689, 1.8758, 0.0415],
                [0.0557, -0.2040, 1.0570],
            ]),
            crop: [0, 0, width, height],
            declared: [0, 0, width, height],
            exposure_bias: bias,
            iso: 0.0,
            linear_response_limit: 1.0,
            lens_correction: None,
            orientation: 0,
        }),
        ..ImageContext::full(width, height)
    }
}

/// The photo a row's edit sets its controls on, where its module lists each choice's
/// options alike on every photo (decision 30): a colour raw's context, which only a
/// choice reads.
pub fn photo() -> ImageContext {
    raw_context(84, 62, 0.0)
}

/// An image already in the working space, as demosaic leaves one.
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[f32; 4]>,
}

impl Picture {
    /// `pixels` in one row, over a colour raw's image context with this exposure bias.
    pub fn row(pixels: Vec<[f32; 4]>, bias: f32) -> (Picture, ImageContext) {
        let width = pixels.len() as u32;
        let picture = Picture {
            width,
            height: 1,
            pixels,
        };
        (picture, raw_context(width, 1, bias))
    }
}

pub fn grey(v: f32) -> [f32; 4] {
    [v, v, v, 1.0]
}

/// How many greys the ramp steps through: one per 1/16 EV from 2⁻¹⁶ to 2⁴.
pub const RAMP_STEPS: usize = 16 * 20 + 1;

/// `steps` neutral greys one per 1/16 EV from 2⁻¹⁶, in order.
fn greys(steps: usize) -> impl Iterator<Item = [f32; 4]> {
    (0..steps).map(|i| grey((-16.0 + i as f32 / 16.0).exp2()))
}

/// The ramp (`docs/modules/README.md`, Test images): neutral greys one per 1/16 EV from
/// 2⁻¹⁶ to 2⁴, [`RAMP_STEPS`] of them, then the greys `then`, which each test file
/// chooses. One row.
pub fn ramp(then: &[f32], bias: f32) -> (Picture, ImageContext) {
    let mut pixels: Vec<[f32; 4]> = greys(RAMP_STEPS).collect();
    pixels.extend(then.iter().map(|&v| grey(v)));
    Picture::row(pixels, bias)
}

/// The extended ramp: neutral greys one per 1/16 EV from 2⁻¹⁶ to 2⁶, in order.
pub fn extended_ramp(bias: f32) -> (Picture, ImageContext) {
    Picture::row(greys(16 * 22 + 1).collect(), bias)
}

/// The golden harness's sweep: 360 hues across, 23 saturations from neutral to beyond the
/// gamut within each of 8 levels from 0.001 to 4 down.
pub fn sweep(bias: f32) -> (Picture, ImageContext) {
    let sweep = synthetic::sweep();
    let (width, height) = (sweep.width as u32, sweep.height as u32);
    let pixels = sweep
        .pixels
        .iter()
        .map(|&[r, g, b]| [r, g, b, 1.0])
        .collect();
    (
        Picture {
            width,
            height,
            pixels,
        },
        raw_context(width, height, bias),
    )
}

/// A small Bayer raw, 96×72 photosites with its declared image inside: a colour gradient
/// with a corner that clips, through the whole pipeline from its mosaic.
pub fn small_raw(bias: f32) -> (Vec<u16>, ImageContext) {
    let (width, height) = (96, 72);
    let mosaic = (0..width * height)
        .map(|i| {
            let (x, y) = (i % width, i / width);
            let c = [0, 1, 1, 2][((y % 2) * 2 + x % 2) as usize] as f32;
            let level =
                0.1 + 0.6 * x as f32 / width as f32 + 0.1 * c + 0.2 * y as f32 / height as f32;
            (level.min(1.0) * 4000.0).round() as u16
        })
        .collect();
    let mut image = raw_context(84, 62, bias);
    let camera = image.camera.as_mut().unwrap();
    (camera.width, camera.height) = (width, height);
    camera.crop = [2, 2, 92, 68];
    camera.declared = [6, 4, 84, 62];
    (mosaic, image)
}

/// Operations a synthetic image, already demosaiced and in the working space, skips.
/// Colour calibration it runs, adapting from D50 ([`Bench::keep`]).
pub const BEFORE_THE_WORKING_SPACE: [&str; 5] = [
    "rawprepare",
    "temperature",
    "highlights",
    "demosaic",
    "colorin",
];

pub fn id(instance: &Instance) -> &'static str {
    instance.operation.manifest().id
}

/// What one render gave: its operations with their cache keys, and the output.
pub struct Rendered {
    plan: Vec<(&'static str, u64)>,
    pub pixels: Vec<[f32; 4]>,
}

/// What a row reads, as each test file defines it.
pub trait Measure {
    /// Keeps those of `instances`, in pipeline order, this measure plans: `synthetic` when
    /// the input is a synthetic image in the working space rather than a mosaic.
    fn keep(self, instances: &mut Vec<Instance<'static>>, synthetic: bool);
}

/// An image on the GPU and the one pipeline rendered over it, updated for every plan as
/// the session updates its own: in place for a plan of the same shape, rebuilt keeping
/// its kernels and the cache points that still hold for another.
pub struct Bench {
    pub image: ImageContext,
    input: GpuImage,
    /// Whether the input is a synthetic image in the working space rather than a mosaic.
    synthetic: bool,
    pub pipe: Pipe,
    pipeline: Option<Pipeline>,
}

impl Bench {
    pub fn picture(picture: &Picture, image: ImageContext, pipe: Pipe) -> Bench {
        let input =
            GpuImage::upload(gpu(), picture.width, picture.height, &picture.pixels).unwrap();
        Bench::new(image, input, true, pipe)
    }

    pub fn mosaic(mosaic: &[u16], image: ImageContext, pipe: Pipe) -> Bench {
        let camera = image.camera.as_ref().unwrap();
        let input = GpuImage::upload_mosaic(gpu(), camera.width, camera.height, mosaic).unwrap();
        Bench::new(image, input, false, pipe)
    }

    pub fn new(image: ImageContext, input: GpuImage, synthetic: bool, pipe: Pipe) -> Bench {
        Bench {
            image,
            input,
            synthetic,
            pipe,
            pipeline: None,
        }
    }

    /// `edit`'s instances (`edit::instances`), as this bench plans them for `measure`.
    pub fn instances(&self, edit: &Edit, measure: impl Measure) -> Vec<Instance<'static>> {
        let all = edit::instances(edit, OPERATIONS, MODULES, &self.image)
            .all()
            .expect("the edit binds");
        self.keep(all, measure)
    }

    /// `instances` in pipeline order, those `measure` plans. A synthetic image is already
    /// white balanced, so colour calibration adapts it from the pipeline's own illuminant,
    /// D50, unless an edit sets one: the camera's, a raw's default (decision 27), would
    /// balance it a second time.
    pub fn keep(
        &self,
        mut instances: Vec<Instance<'static>>,
        measure: impl Measure,
    ) -> Vec<Instance<'static>> {
        instances.sort_by(|a, b| {
            let (a, b) = (a.operation.manifest(), b.operation.manifest());
            a.slot.pipeline_order(&b.slot)
        });
        if self.synthetic {
            for instance in &mut instances {
                if id(instance) == "channelmixerrgb" && instance.params.get("illuminant").is_none()
                {
                    instance.params.set(
                        "illuminant",
                        Value::Choice(Illuminant::Pipe.to_darktable() as u32),
                    );
                }
            }
        }
        measure.keep(&mut instances, self.synthetic);
        instances
    }

    pub fn render(&mut self, instances: &[Instance]) -> Rendered {
        let plan = pipeline::plan(
            instances,
            &self.image,
            self.input.desc(),
            self.pipe,
            &Limits::of(gpu()),
        )
        .unwrap();
        let ids: Vec<(&'static str, u64)> =
            plan.operations.iter().map(|op| (op.id, op.key)).collect();
        let mut pipeline = match self.pipeline.take() {
            Some(pipeline) => pollster::block_on(pipeline.update(gpu(), plan)),
            None => pollster::block_on(Pipeline::new(gpu(), plan, self.input.clone())),
        }
        .unwrap();
        pipeline.render(gpu()).unwrap();
        let pixels = pipeline.output().download(gpu()).unwrap();
        self.pipeline = Some(pipeline);
        Rendered { plan: ids, pixels }
    }

    pub fn edit(&mut self, edit: &Edit, measure: impl Measure) -> Rendered {
        let instances = self.instances(edit, measure);
        self.render(&instances)
    }
}

/// Whether two renders are byte for byte the same, with the same operations and keys.
pub fn exact(a: &Rendered, b: &Rendered) -> bool {
    a.plan == b.plan
        && a.pixels.len() == b.pixels.len()
        && a.pixels
            .iter()
            .zip(&b.pixels)
            .all(|(x, y)| x.map(f32::to_bits) == y.map(f32::to_bits))
}

/// The jump between two frames in the output profile, sRGB by default, as the exit
/// test measures them: CIEDE2000 per pixel, its mean, the pixels over 1.0, its max.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Jump {
    pub pixels: usize,
    pub mean: f64,
    pub over_one: usize,
    pub max: f64,
}

impl Jump {
    pub fn share_over_one(&self) -> f64 {
        self.over_one as f64 / self.pixels.max(1) as f64
    }
}

impl std::fmt::Display for Jump {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ΔE2000 mean {:.4}, max {:.3}, {:.3}% over 1.0, over {} pixels",
            self.mean,
            self.max,
            100.0 * self.share_over_one(),
            self.pixels
        )
    }
}

/// The jump from `before` to `after` over the pixels `keep` takes, both sRGB clipped to
/// 0 to 1, on every core.
pub fn jump(before: &[[f32; 4]], after: &[[f32; 4]], keep: impl Fn(usize) -> bool + Sync) -> Jump {
    assert_eq!(before.len(), after.len());
    let lab = |p: [f32; 4]| delta_e::srgb_to_lab([0, 1, 2].map(|c| p[c].clamp(0.0, 1.0)));
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
    let chunk = before.len().div_ceil(threads).max(1);
    let parts: Vec<(usize, f64, usize, f64)> = std::thread::scope(|scope| {
        let keep = &keep;
        let handles: Vec<_> = (0..before.len())
            .step_by(chunk)
            .map(|start| {
                scope.spawn(move || {
                    let mut part = (0, 0.0, 0, 0.0_f64);
                    for i in start..(start + chunk).min(before.len()) {
                        if !keep(i) {
                            continue;
                        }
                        let difference = delta_e::ciede2000(lab(before[i]), lab(after[i]));
                        part.0 += 1;
                        part.1 += difference;
                        part.2 += usize::from(difference > 1.0);
                        part.3 = part.3.max(difference);
                    }
                    part
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let pixels = parts.iter().map(|p| p.0).sum();
    Jump {
        pixels,
        mean: parts.iter().map(|p| p.1).sum::<f64>() / pixels.max(1) as f64,
        over_one: parts.iter().map(|p| p.2).sum(),
        max: parts.iter().map(|p| p.3).fold(0.0, f64::max),
    }
}
