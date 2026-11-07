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

//! The viewport pipe on a GPU adapter, hardware or software: a render that restarts
//! from a cache point gives the same image, bit for bit, as a full render of the same
//! edit (`docs/engine.md`, Testing), and it restarts where the change allows. On the
//! same adapter opened to bind less than demosaic's image at once, the viewport
//! demosaics and downscales tile by tile, and gives the untiled viewport's image bit
//! for bit (decision 14). A pipeline rebuilt because an operation joined or left or the
//! viewport grew keeps its kernels and the cache points that still hold, runs the raw
//! stage before its first cache point only when a change reaches it, and gives a fresh
//! pipeline's image bit for bit (decision 28). An operation's whole-image passes run
//! only when their own key changes, not when only its later passes' parameters do, and
//! their result is kept across a rebuild (decision 15); haze removal's estimate at full
//! resolution across a new viewport size too (decision 15).

use apertrust_engine::compute::{GpuImage, Timings};
use apertrust_engine::edit::{self, Edit};
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe, Pipeline, Plan, PlannedOperation};
use apertrust_operations::OPERATIONS;
use apertrust_plugin::{
    Camera, Format, ImageContext, ImageDesc, Kind, ParamValues, Pattern, Resource, Value,
};

const VIEWPORT: Pipe = Pipe::Viewport {
    width: 40,
    height: 40,
};

#[rustfmt::skip]
const X_TRANS: [u32; 36] = [
    1, 1, 0, 1, 1, 2,
    1, 1, 2, 1, 1, 0,
    2, 0, 1, 0, 2, 1,
    1, 1, 2, 1, 1, 0,
    1, 1, 0, 1, 1, 2,
    0, 2, 1, 2, 0, 1,
];

/// A raw `sensor` photosites across and down, its active area `crop` and its declared
/// image `declared`: a colour gradient with a corner that clips.
struct Scene {
    cfa: Pattern<u32>,
    sensor: (u32, u32),
    crop: [u32; 4],
    declared: [u32; 4],
}

impl Scene {
    /// A small Bayer raw, its declared crop inside its active area.
    fn small() -> Self {
        Self {
            cfa: Pattern {
                width: 2,
                height: 2,
                values: vec![0, 1, 1, 2],
            },
            sensor: (96, 72),
            crop: [2, 2, 92, 68],
            declared: [6, 4, 84, 62],
        }
    }

    /// A raw whose demosaiced image, about 6 MB, is far larger than `SMALL_BINDING`.
    fn large(cfa: Pattern<u32>) -> Self {
        Self {
            cfa,
            sensor: (767, 514),
            crop: [2, 3, 762, 508],
            declared: [5, 6, 752, 500],
        }
    }

    fn raw(&self) -> Vec<u16> {
        let (width, height) = self.sensor;
        let cfa = &self.cfa;
        (0..width * height)
            .map(|i| {
                let (x, y) = (i % width, i / width);
                let c = cfa.values[((y % cfa.height) * cfa.width + x % cfa.width) as usize] as f32;
                let level =
                    0.1 + 0.6 * x as f32 / width as f32 + 0.1 * c + 0.2 * y as f32 / height as f32;
                (level.min(1.0) * 4000.0).round() as u16
            })
            .collect()
    }

    fn image(&self) -> ImageContext {
        let (width, height) = self.sensor;
        ImageContext {
            camera: Some(Camera {
                make: String::new(),
                model: String::new(),
                alias: String::new(),
                width,
                height,
                cfa: Some(self.cfa.clone()),
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
                crop: self.crop,
                declared: self.declared,
                exposure_bias: 0.0,
                iso: 0.0,
                linear_response_limit: 1.0,
                lens_correction: None,
                orientation: 0,
            }),
            ..ImageContext::full(self.declared[2], self.declared[3])
        }
    }

    fn plan(&self, gpu: &GpuContext, settings: &[(&str, &str, f32)], pipe: Pipe) -> pipeline::Plan {
        self.plan_within(settings, pipe, &Limits::of(gpu))
    }

    /// [`Self::plan`] within `limits`, as for a GPU that binds less.
    fn plan_within(
        &self,
        settings: &[(&str, &str, f32)],
        pipe: Pipe,
        limits: &Limits,
    ) -> pipeline::Plan {
        let sensor = ImageDesc::new(self.sensor.0, self.sensor.1, Format::Mosaic16);
        pipeline::plan(
            &self.instances(settings),
            &self.image(),
            sensor,
            pipe,
            limits,
        )
        .unwrap()
    }

    /// The default pipeline with these parameters set, as an edit sets them, and every
    /// operation an unedited photo does not run that a setting names, with its settings,
    /// as a module's binding adds it (decision 28); no modules, so the tests stay about
    /// the pipeline.
    fn instances(&self, settings: &[(&str, &str, f32)]) -> Vec<Instance<'static>> {
        let optional = |id: &str| {
            OPERATIONS
                .iter()
                .copied()
                .find(|op| op.manifest().id == id && !op.manifest().default_enabled)
        };
        let mut edit = Edit::default();
        // An edit refuses an optional operation's parameters, which only modules set.
        let mut joined: Vec<Instance<'static>> = Vec::new();
        for &(id, param, value) in settings {
            let Some(operation) = optional(id) else {
                edit.set_param(OPERATIONS, &[], id, param, Value::Float(value))
                    .unwrap();
                continue;
            };
            let at = match joined.iter().position(|i| i.operation.manifest().id == id) {
                Some(at) => at,
                None => {
                    joined.push(Instance {
                        operation,
                        params: ParamValues::new(),
                    });
                    joined.len() - 1
                }
            };
            // A choice or a whole number by its index or value.
            let kind = operation.schema().params.iter().find(|p| p.name == param);
            let value = match kind.map(|p| p.kind) {
                Some(Kind::Choice { .. }) => Value::Choice(value as u32),
                Some(Kind::Int { .. }) => Value::Int(value as i64),
                _ => Value::Float(value),
            };
            joined[at].params.set(param, value);
        }
        let mut instances = edit::instances(&edit, OPERATIONS, &[], &self.image())
            .all()
            .unwrap();
        assert!(
            joined.iter().all(|j| instances
                .iter()
                .all(|i| i.operation.manifest().id != j.operation.manifest().id)),
            "an optional operation in the default pipeline"
        );
        instances.extend(joined);
        instances
    }

    fn upload(&self, gpu: &GpuContext) -> GpuImage {
        GpuImage::upload_mosaic(gpu, self.sensor.0, self.sensor.1, &self.raw()).unwrap()
    }

    /// A fresh pipeline's full render of these settings.
    fn fresh(&self, gpu: &GpuContext, settings: &[(&str, &str, f32)], pipe: Pipe) -> Vec<[f32; 4]> {
        let plan = self.plan(gpu, settings, pipe);
        let mut pipeline = pollster::block_on(Pipeline::new(gpu, plan, self.upload(gpu))).unwrap();
        pipeline.render(gpu).unwrap();
        pipeline.output().download(gpu).unwrap()
    }

    /// Updates the pipeline to these settings, renders, and checks where the render
    /// began, that it ran every pass from there to the end and the raw stage only if it
    /// began there, and that it matches a full render bit for bit.
    fn change(
        &self,
        gpu: &GpuContext,
        pipeline: Pipeline,
        settings: &[(&str, &str, f32)],
        pipe: Pipe,
        starts_at: &str,
    ) -> Pipeline {
        self.change_kept(gpu, pipeline, settings, pipe, starts_at, Kept::NOTHING)
    }

    /// [`Self::change`], the render running none of the whole-image passes of the
    /// operations `kept` names, whose results their scratch holds (decision 15).
    fn change_keeping(
        &self,
        gpu: &GpuContext,
        pipeline: Pipeline,
        settings: &[(&str, &str, f32)],
        pipe: Pipe,
        starts_at: &str,
        kept: &[&str],
    ) -> Pipeline {
        let kept = Kept {
            whole: kept,
            ..Kept::NOTHING
        };
        self.change_kept(gpu, pipeline, settings, pipe, starts_at, kept)
    }

    /// [`Self::change`], the render running none of the passes of the full-resolution
    /// part of the operations `estimated` names, whose results their scratch holds
    /// (decision 15).
    fn change_keeping_estimate(
        &self,
        gpu: &GpuContext,
        pipeline: Pipeline,
        settings: &[(&str, &str, f32)],
        pipe: Pipe,
        starts_at: &str,
        estimated: &[&str],
    ) -> Pipeline {
        let kept = Kept {
            estimate: estimated,
            ..Kept::NOTHING
        };
        self.change_kept(gpu, pipeline, settings, pipe, starts_at, kept)
    }

    fn change_kept(
        &self,
        gpu: &GpuContext,
        pipeline: Pipeline,
        settings: &[(&str, &str, f32)],
        pipe: Pipe,
        starts_at: &str,
        kept: Kept,
    ) -> Pipeline {
        let plan = self.plan(gpu, settings, pipe);
        let mut pipeline = pollster::block_on(pipeline.update(gpu, plan)).unwrap();
        let timings = pipeline.render(gpu).unwrap();
        assert_eq!(timings.passes[0].0, starts_at, "{settings:?}");
        restarts_where_it_says(pipeline.plan(), &timings, starts_at, kept);
        let restarted = pipeline.output().download(gpu).unwrap();
        let full = self.fresh(gpu, settings, pipe);
        assert_eq!(restarted.len(), full.len());
        if let Some(i) = restarted
            .iter()
            .zip(&full)
            .position(|(a, b)| a.map(f32::to_bits) != b.map(f32::to_bits))
        {
            panic!(
                "{settings:?}: pixel {i} is {:?} after a restart, {:?} in a full render",
                restarted[i], full[i]
            );
        }
        pipeline
    }
}

/// The whole-image passes a render keeps rather than runs: every one of the operations
/// `whole` names (decision 15), and those of the full-resolution part of the operations
/// `estimate` names (decision 15).
#[derive(Clone, Copy)]
struct Kept<'a> {
    whole: &'a [&'a str],
    estimate: &'a [&'a str],
}

impl Kept<'_> {
    const NOTHING: Self = Self {
        whole: &[],
        estimate: &[],
    };

    /// How many of `op`'s passes, from its first, the render keeps.
    fn passes(&self, op: &PlannedOperation) -> usize {
        let Some(whole) = op.whole_image else {
            return 0;
        };
        if self.whole.contains(&op.id) {
            whole.passes
        } else if self.estimate.contains(&op.id) {
            whole.full_resolution.map_or(0, |part| part.passes)
        } else {
            0
        }
    }
}

/// The labels a render gives these operations' passes in its timings, in dispatch order,
/// without the passes `kept` names.
fn labels(operations: &[PlannedOperation], kept: Kept) -> Vec<&'static str> {
    operations
        .iter()
        .flat_map(|op| {
            let skipped = kept.passes(op);
            op.passes[skipped..].iter().map(|pass| {
                if pass.entry_point == "main" {
                    op.id
                } else {
                    pass.entry_point
                }
            })
        })
        .collect()
}

/// The render ran the plan's passes from one to the last, the first `starts_at`, but the
/// passes `kept` names, and none of the raw stage's up to its first cache point,
/// demosaic's image or, in tiles, the raw stage's output, unless it began there.
fn restarts_where_it_says(plan: &Plan, timings: &Timings, starts_at: &str, kept: Kept) {
    for id in kept.whole.iter().chain(kept.estimate) {
        let op = plan.operations.iter().find(|op| op.id == *id);
        assert!(
            op.is_some_and(|op| kept.passes(op) > 0),
            "{id} has no whole-image passes to keep"
        );
    }
    let all = labels(&plan.operations, kept);
    let ran: Vec<&str> = timings.passes.iter().map(|(name, _)| *name).collect();
    assert_eq!(ran, all[all.len() - ran.len()..], "the passes to the end");
    let end = plan.cached.first().map_or(0, |&op| op + 1);
    let raw_stage = labels(&plan.operations[..end], Kept::NOTHING);
    if !raw_stage.contains(&starts_at) {
        assert!(
            ran.iter().all(|name| !raw_stage.contains(name)),
            "the raw stage ran: {ran:?}"
        );
    }
}

fn default_gpu() -> GpuContext {
    pollster::block_on(GpuContext::new(&GpuOptions::default()))
        .expect("a GPU adapter, hardware or software")
}

#[test]
fn a_restarted_render_equals_a_full_one_and_starts_where_the_change_does() {
    let gpu = default_gpu();
    let scene = Scene::small();
    let plan = scene.plan(&gpu, &[], VIEWPORT);
    assert!(plan.tiled.is_none());
    let mut pipeline = pollster::block_on(Pipeline::new(&gpu, plan, scene.upload(&gpu))).unwrap();
    let first = pipeline.render(&gpu).unwrap();
    assert_eq!(first.passes[0].0, "rawprepare");
    let ids: Vec<_> = pipeline.plan().operations.iter().map(|op| op.id).collect();
    assert!(ids.contains(&"downscale"), "{ids:?}");
    let desc = pipeline.output().desc();
    assert_eq!((desc.width, desc.height), (40, 30));

    // Exposure reads the downscale, a cache point.
    let exposure = ("exposure", "exposure", 0.5);
    pipeline = scene.change(&gpu, pipeline, &[exposure], VIEWPORT, "exposure");
    // The first change to AgX also starts at the downscale and keeps AgX's input;
    // the next starts at AgX.
    let saturation = |value| ("agx", "look_saturation", value);
    pipeline = scene.change(
        &gpu,
        pipeline,
        &[exposure, saturation(1.2)],
        VIEWPORT,
        "exposure",
    );
    pipeline = scene.change(
        &gpu,
        pipeline,
        &[exposure, saturation(1.4)],
        VIEWPORT,
        "agx",
    );
    // A change upstream of the kept input leaves only the downscale valid.
    let exposure = ("exposure", "exposure", 0.7);
    pipeline = scene.change(
        &gpu,
        pipeline,
        &[exposure, saturation(1.4)],
        VIEWPORT,
        "exposure",
    );
    // A raw-stage change runs everything.
    let red = ("temperature", "red", 2.2);
    pipeline = scene.change(
        &gpu,
        pipeline,
        &[exposure, saturation(1.4), red],
        VIEWPORT,
        "rawprepare",
    );
    // A smaller viewport fits the pipeline's buffers, and demosaic's image is still
    // valid: the render starts at the downscale.
    let smaller = Pipe::Viewport {
        width: 24,
        height: 24,
    };
    let pipeline = scene.change(
        &gpu,
        pipeline,
        &[exposure, saturation(1.4), red],
        smaller,
        "downscale",
    );
    // A larger one needs larger buffers: the pipeline is rebuilt, keeping demosaic's
    // image, and starts at the downscale (decision 28).
    let larger = Pipe::Viewport {
        width: 60,
        height: 60,
    };
    scene.change(
        &gpu,
        pipeline,
        &[exposure, saturation(1.4), red],
        larger,
        "downscale",
    );
}

#[test]
fn an_operation_joining_or_leaving_keeps_the_kernels_and_cache_points_that_hold() {
    let gpu = default_gpu();
    let scene = Scene::small();
    let plan = scene.plan(&gpu, &[], VIEWPORT);
    let mut pipeline = pollster::block_on(Pipeline::new(&gpu, plan, scene.upload(&gpu))).unwrap();
    pipeline.render(&gpu).unwrap();
    let untouched = pipeline.output().download(&gpu).unwrap();
    let kernels = pipeline.kernels();

    // Grain joins after the downscale: the pipeline is rebuilt, keeping demosaic's image
    // and its downscale, and compiles only grain's kernel.
    let grain = |strength| ("grain", "strength", strength);
    pipeline = scene.change(&gpu, pipeline, &[grain(25.0)], VIEWPORT, "exposure");
    assert_eq!(pipeline.kernels(), kernels + 1);
    assert!(pipeline.output().download(&gpu).unwrap() != untouched);
    // Two grain changes: the first keeps grain's input, the second starts there.
    pipeline = scene.change(&gpu, pipeline, &[grain(40.0)], VIEWPORT, "exposure");
    pipeline = scene.change(&gpu, pipeline, &[grain(50.0)], VIEWPORT, "grain");

    // Grain leaves: the edit cache still holds AgX's output, now colorout's input, and
    // the frame is the untouched one again.
    pipeline = scene.change(&gpu, pipeline, &[], VIEWPORT, "colorout");
    let left = pipeline.output().download(&gpu).unwrap();
    assert!(
        left.iter()
            .zip(&untouched)
            .all(|(a, b)| a.map(f32::to_bits) == b.map(f32::to_bits))
    );
    assert_eq!(pipeline.kernels(), kernels + 1);

    // Vignette joins, then grain again before it, each starting at its own input, which
    // the edit cache holds; grain's kernel is not compiled again.
    let vignette = ("vignette", "brightness", -0.5);
    pipeline = scene.change(&gpu, pipeline, &[vignette], VIEWPORT, "vignette");
    assert_eq!(pipeline.kernels(), kernels + 2);
    pipeline = scene.change(&gpu, pipeline, &[grain(50.0), vignette], VIEWPORT, "grain");
    assert_eq!(pipeline.kernels(), kernels + 2);

    // A larger viewport keeps demosaic's image and starts at the downscale.
    let larger = Pipe::Viewport {
        width: 60,
        height: 60,
    };
    let pipeline = scene.change(
        &gpu,
        pipeline,
        &[grain(50.0), vignette],
        larger,
        "downscale",
    );
    assert_eq!(pipeline.kernels(), kernels + 2);
}

#[test]
fn a_tiled_viewport_rebuilt_keeps_the_raw_stages_output() {
    let gpu = small_gpu();
    let scene = Scene::large(Pattern {
        width: 2,
        height: 2,
        values: vec![0, 1, 1, 2],
    });
    let plan = scene.plan(&gpu, &[], SMALL_VIEWPORT);
    assert!(plan.tiled.is_some());
    let mut pipeline = pollster::block_on(Pipeline::new(&gpu, plan, scene.upload(&gpu))).unwrap();
    pipeline.render(&gpu).unwrap();
    let kernels = pipeline.kernels();

    // Grain joins after the tiles: the viewport image is kept, a cache point.
    let grain = [("grain", "strength", 25.0)];
    pipeline = scene.change(&gpu, pipeline, &grain, SMALL_VIEWPORT, "exposure");
    assert_eq!(pipeline.kernels(), kernels + 1);
    // A larger viewport, whose image still binds within the limit, keeps the raw
    // stage's output and starts at the tiles.
    let larger = Pipe::Viewport {
        width: 220,
        height: 220,
    };
    pipeline = scene.change(&gpu, pipeline, &grain, larger, "region");
    assert!(pipeline.plan().tiled.is_some());
    // Grain leaves.
    pipeline = scene.change(&gpu, pipeline, &[], larger, "exposure");
    assert_eq!(pipeline.kernels(), kernels + 1);
    // Grain joins with a raw-stage change, which no cache point holds.
    let red = ("temperature", "red", 2.2);
    scene.change(&gpu, pipeline, &[grain[0], red], larger, "rawprepare");
}

#[test]
fn a_rebuild_with_a_change_before_a_cache_point_runs_from_there() {
    // The session renders the commands that arrived together at once, so a raw-stage
    // change can come with an operation joining, as when a demosaic setting changes in
    // the same batch as Grain's first step: the cache points' keys change, and the
    // rebuild keeps none of them.
    let gpu = default_gpu();
    let scene = Scene::small();
    let plan = scene.plan(&gpu, &[], VIEWPORT);
    let mut pipeline = pollster::block_on(Pipeline::new(&gpu, plan, scene.upload(&gpu))).unwrap();
    pipeline.render(&gpu).unwrap();
    let grain = ("grain", "strength", 25.0);
    let red = ("temperature", "red", 2.2);
    pipeline = scene.change(&gpu, pipeline, &[grain, red], VIEWPORT, "rawprepare");
    // Grain leaves while exposure changes: demosaic's image and its downscale hold.
    let exposure = ("exposure", "exposure", 0.5);
    pipeline = scene.change(&gpu, pipeline, &[red, exposure], VIEWPORT, "exposure");
    // Grain joins while temperature goes back.
    let larger = Pipe::Viewport {
        width: 60,
        height: 60,
    };
    scene.change(&gpu, pipeline, &[grain, exposure], larger, "rawprepare");
}

#[test]
fn a_band_drag_runs_the_tone_equalizers_main_alone() {
    // The mask's whole-image passes are keyed apart from `main`, so while only the bands
    // change their result stays in scratch, across a rebuild too, and no frame after the
    // first runs them; a mask setting or a change upstream runs them again (decision 15).
    let gpu = default_gpu();
    let scene = Scene::small();
    let plan = scene.plan(&gpu, &[], VIEWPORT);
    let mut pipeline = pollster::block_on(Pipeline::new(&gpu, plan, scene.upload(&gpu))).unwrap();
    pipeline.render(&gpu).unwrap();
    let band = |ev| ("toneequal", "shadows", ev);
    let kept = &["toneequal"];
    // It joins with EIGF, its default filter, and runs every pass.
    pipeline = scene.change(&gpu, pipeline, &[band(0.5)], VIEWPORT, "exposure");
    let whole = pipeline
        .plan()
        .operations
        .iter()
        .find(|op| op.id == "toneequal");
    assert!(
        whole
            .and_then(|op| op.whole_image)
            .is_some_and(|w| w.passes == 5)
    );
    // A drag: its first step keeps the tone equalizer's input, the next starts there.
    for (ev, starts_at) in [(0.8, "exposure"), (1.0, "toneequal"), (-0.4, "toneequal")] {
        pipeline = scene.change_keeping(&gpu, pipeline, &[band(ev)], VIEWPORT, starts_at, kept);
    }
    // A mask setting runs them from its input, which the edit cache holds; then the
    // bands alone again do not.
    let boost = ("toneequal", "exposure_boost", 1.0);
    let settings = [band(-0.4), boost];
    pipeline = scene.change(&gpu, pipeline, &settings, VIEWPORT, "eigf_downsample");
    let settings = [band(0.6), boost];
    pipeline = scene.change_keeping(&gpu, pipeline, &settings, VIEWPORT, "toneequal", kept);
    // A change upstream runs them.
    let exposure = ("exposure", "exposure", 0.5);
    let settings = [exposure, band(0.6), boost];
    pipeline = scene.change(&gpu, pipeline, &settings, VIEWPORT, "exposure");
    // Grain joins after it: the rebuild keeps their result with its key, and the render
    // from the downscale's cache point does not run them, nor the drag after it.
    let grain = ("grain", "strength", 25.0);
    let settings = [exposure, band(0.6), boost, grain];
    pipeline = scene.change_keeping(&gpu, pipeline, &settings, VIEWPORT, "exposure", kept);
    let settings = [exposure, band(0.9), boost, grain];
    pipeline = scene.change_keeping(&gpu, pipeline, &settings, VIEWPORT, "exposure", kept);
    // A larger viewport changes their scale and size: they run again.
    let larger = Pipe::Viewport {
        width: 60,
        height: 60,
    };
    scene.change(&gpu, pipeline, &settings, larger, "downscale");
}

#[test]
fn a_haze_removal_strength_drag_runs_its_main_alone() {
    // Strength and distance act only in `main` (decision 15); the sign of the strength
    // orders the transmission's filters, so crossing zero runs them again.
    let gpu = default_gpu();
    let scene = Scene::small();
    let plan = scene.plan(&gpu, &[], VIEWPORT);
    let mut pipeline = pollster::block_on(Pipeline::new(&gpu, plan, scene.upload(&gpu))).unwrap();
    pipeline.render(&gpu).unwrap();
    let strength = |s| ("hazeremoval", "strength", s);
    let kept = &["hazeremoval"];
    // Its input is the downscale, a cache point: every render starts there.
    pipeline = scene.change(&gpu, pipeline, &[strength(0.3)], VIEWPORT, "image_rows");
    for value in [0.5, 0.9] {
        let settings = [strength(value)];
        pipeline = scene.change_keeping(&gpu, pipeline, &settings, VIEWPORT, "hazeremoval", kept);
    }
    let distance = ("hazeremoval", "distance", 0.6);
    let settings = [strength(0.9), distance];
    pipeline = scene.change_keeping(&gpu, pipeline, &settings, VIEWPORT, "hazeremoval", kept);
    // Grain joins: the rebuild keeps their result with its key, and the drag after it
    // still runs `main` alone.
    let grain = ("grain", "strength", 25.0);
    for value in [0.9, 0.7] {
        let settings = [strength(value), distance, grain];
        pipeline = scene.change_keeping(&gpu, pipeline, &settings, VIEWPORT, "hazeremoval", kept);
    }
    let settings = [strength(-0.3), distance, grain];
    scene.change(&gpu, pipeline, &settings, VIEWPORT, "image_rows");
}

#[test]
fn haze_removal_estimates_from_the_viewports_full_resolution() {
    // Decision 15: in the untiled viewport, haze removal's whole-image passes bind the
    // full-resolution image the downscale read, with no copy of it, and run at its size,
    // their scratch 24 bytes a pixel of it, and `plane_downscale` brings the
    // transmission's input to the viewport's size for `main`, 4 bytes a pixel of it more.
    // A change upstream reruns that estimate, every frame a fresh pipeline's; in a tiled
    // viewport, whose full resolution exists a tile at a time, it estimates at the
    // viewport's scale.
    let gpu = default_gpu();
    let scene = Scene::small();
    let strength = [("hazeremoval", "strength", 0.9)];
    let plan = scene.plan(&gpu, &strength, VIEWPORT);
    let haze = plan
        .operations
        .iter()
        .find(|op| op.id == "hazeremoval")
        .unwrap();
    assert!(haze.full_resolution.is_some());
    let binds = |op: &PlannedOperation, resource: Resource| {
        op.passes[0].bindings.iter().any(|b| b.resource == resource)
    };
    assert!(binds(haze, Resource::FullResolution));
    let entry_points: Vec<&str> = haze.passes.iter().map(|p| p.entry_point).collect();
    assert_eq!(
        entry_points[entry_points.len() - 2..],
        ["plane_downscale", "main"]
    );
    let [.., width, height] = scene.declared;
    let full = u64::from(width * height);
    let viewport = u64::from(haze.output_desc.width * haze.output_desc.height);
    assert!(viewport < full);
    let partials = width.div_ceil(4).div_ceil(16) * height.div_ceil(4).div_ceil(16);
    let fixed = 2048 * 4 + 16 * 4 + u64::from(partials) * 16;
    let bytes: u64 = haze.scratch.iter().map(|&i| plan.scratch[i]).sum();
    assert_eq!(bytes, 24 * full + fixed + 4 * viewport);

    let mut pipeline = pollster::block_on(Pipeline::new(&gpu, plan, scene.upload(&gpu))).unwrap();
    pipeline.render(&gpu).unwrap();
    // A raw-stage change reruns everything, the estimate at full resolution among it.
    let red = ("temperature", "red", 2.2);
    scene.change(&gpu, pipeline, &[strength[0], red], VIEWPORT, "rawprepare");

    let limits = Limits {
        budget: 16 << 10,
        ..Limits::of(&gpu)
    };
    let tiled = scene.plan_within(&strength, VIEWPORT, &limits);
    assert!(tiled.tiled.is_some());
    let haze = tiled
        .operations
        .iter()
        .find(|op| op.id == "hazeremoval")
        .unwrap();
    assert!(haze.full_resolution.is_none());
    assert!(binds(haze, Resource::Input));
}

#[test]
fn a_resize_keeps_haze_removals_estimate() {
    // Decision 15, amended: haze removal's passes up to its guided filter's result run
    // over the full-resolution image and are keyed on it, nothing of the viewport's size
    // in their key, so a larger viewport, a rebuild, and a smaller one, in place, run
    // only `plane_downscale` and `main` of its passes; a strength drag runs `main` alone,
    // and a raw-stage change both parts. Every frame is a fresh pipeline's at its size.
    let gpu = default_gpu();
    let scene = Scene::small();
    let strength = |s| ("hazeremoval", "strength", s);
    let haze = |plan: &Plan| {
        let op = plan.operations.iter().find(|op| op.id == "hazeremoval");
        op.unwrap().clone()
    };
    let plan = scene.plan(&gpu, &[strength(0.9)], VIEWPORT);
    let planned = haze(&plan);
    let whole = planned.whole_image.unwrap();
    let part = whole.full_resolution.unwrap();
    assert_eq!(planned.passes[part.passes - 1].entry_point, "guide_result");
    let after: Vec<&str> = planned.passes[part.passes..whole.passes]
        .iter()
        .map(|pass| pass.entry_point)
        .collect();
    assert_eq!(after, ["plane_downscale"]);
    let mut pipeline = pollster::block_on(Pipeline::new(&gpu, plan, scene.upload(&gpu))).unwrap();
    pipeline.render(&gpu).unwrap();

    let estimated = &["hazeremoval"];
    let larger = Pipe::Viewport {
        width: 60,
        height: 60,
    };
    let smaller = Pipe::Viewport {
        width: 24,
        height: 24,
    };
    // The larger rebuilds the pipeline, the smaller fits its buffers.
    for (pipe, in_place) in [(larger, false), (smaller, true)] {
        let settings = [strength(0.9)];
        let resized = scene.plan(&gpu, &settings, pipe);
        assert_eq!(pipeline.accepts(&resized), in_place);
        pipeline =
            scene.change_keeping_estimate(&gpu, pipeline, &settings, pipe, "downscale", estimated);
        let resized = haze(pipeline.plan());
        assert!(resized.full_resolution.is_some());
        assert_eq!(
            resized.whole_image.and_then(|w| w.full_resolution),
            Some(part)
        );
    }
    // A strength drag: its input, the downscale, is a cache point.
    let kept = &["hazeremoval"];
    for value in [0.7, 0.5] {
        let settings = [strength(value)];
        pipeline = scene.change_keeping(&gpu, pipeline, &settings, smaller, "hazeremoval", kept);
    }
    // A raw-stage change runs both parts.
    let red = ("temperature", "red", 2.2);
    scene.change(&gpu, pipeline, &[strength(0.5), red], smaller, "rawprepare");
}

#[test]
fn a_rebuild_after_a_smaller_viewport_keeps_haze_removals_estimate() {
    // A smaller viewport taken in place leaves haze removal's viewport plane the size the
    // larger one made, so a rebuild at the smaller size, as grain joining makes, cannot
    // move its whole result, whose key still holds: it moves the full-resolution part's,
    // and runs `plane_downscale` and `main` of its passes (decision 15).
    let gpu = default_gpu();
    let scene = Scene::small();
    let strength = |s| ("hazeremoval", "strength", s);
    let larger = Pipe::Viewport {
        width: 60,
        height: 60,
    };
    let smaller = Pipe::Viewport {
        width: 24,
        height: 24,
    };
    let plan = scene.plan(&gpu, &[strength(0.9)], larger);
    let mut pipeline = pollster::block_on(Pipeline::new(&gpu, plan, scene.upload(&gpu))).unwrap();
    pipeline.render(&gpu).unwrap();
    let estimated = &["hazeremoval"];
    let settings = [strength(0.9)];
    assert!(pipeline.accepts(&scene.plan(&gpu, &settings, smaller)));
    pipeline =
        scene.change_keeping_estimate(&gpu, pipeline, &settings, smaller, "downscale", estimated);
    let grain = ("grain", "strength", 25.0);
    let settings = [strength(0.9), grain];
    assert!(!pipeline.accepts(&scene.plan(&gpu, &settings, smaller)));
    pipeline =
        scene.change_keeping_estimate(&gpu, pipeline, &settings, smaller, "downscale", estimated);
    // A strength drag after it runs `main` alone again.
    let settings = [strength(0.7), grain];
    let kept = &["hazeremoval"];
    scene.change_keeping(&gpu, pipeline, &settings, smaller, "hazeremoval", kept);
}

#[test]
fn a_local_contrast_detail_drag_runs_its_pyramid_again() {
    // The local laplacian filter's every parameter feeds its pyramid: nothing is kept.
    // The bilateral grid's detail acts only in `main`, so a drag of it keeps the grid.
    let gpu = default_gpu();
    let scene = Scene::small();
    let plan = scene.plan(&gpu, &[], VIEWPORT);
    let mut pipeline = pollster::block_on(Pipeline::new(&gpu, plan, scene.upload(&gpu))).unwrap();
    pipeline.render(&gpu).unwrap();
    let detail = |d| ("bilat", "detail", d);
    pipeline = scene.change(&gpu, pipeline, &[detail(0.5)], VIEWPORT, "exposure");
    pipeline = scene.change(&gpu, pipeline, &[detail(0.6)], VIEWPORT, "exposure");
    pipeline = scene.change(&gpu, pipeline, &[detail(0.7)], VIEWPORT, "ll_reduce_input");
    // The bilateral grid, choice 0: other passes, so a rebuild, which keeps the edit
    // cache at local contrast's input.
    let grid = ("bilat", "mode", 0.0);
    pipeline = scene.change(&gpu, pipeline, &[grid, detail(0.7)], VIEWPORT, "bl_splat");
    for value in [0.9, 1.0] {
        let settings = [grid, detail(value)];
        pipeline = scene.change_keeping(&gpu, pipeline, &settings, VIEWPORT, "bilat", &["bilat"]);
    }
}

#[test]
fn a_raw_stage_restart_keeps_highlight_reconstructions_statistic() {
    // Planned as for a GPU that binds less, the raw stage runs in row strips and the
    // viewport demosaics in tiles, so no cache point of the untiled plan holds and the
    // render starts at rawprepare. Highlight reconstruction's statistic is the same in
    // strips, and its key holds across the rebuild: only its inpainting runs (decisions
    // 14 and 15).
    let gpu = default_gpu();
    let scene = Scene::large(Pattern {
        width: 2,
        height: 2,
        values: vec![0, 1, 1, 2],
    });
    let untiled = scene.plan(&gpu, &[], SMALL_VIEWPORT);
    assert!(untiled.tiled.is_none());
    let mut pipeline =
        pollster::block_on(Pipeline::new(&gpu, untiled, scene.upload(&gpu))).unwrap();
    pipeline.render(&gpu).unwrap();

    let limits = Limits {
        binding: SMALL_BINDING,
        ..Limits::of(&gpu)
    };
    let plan = scene.plan_within(&[], SMALL_VIEWPORT, &limits);
    assert!(plan.tiled.is_some());
    let highlights = plan.operations.iter().find(|op| op.id == "highlights");
    let mask = highlights.map(|op| (op.passes[0].entry_point, op.dispatches[0].len()));
    assert!(
        mask.is_some_and(|(entry_point, strips)| entry_point == "opposed_mask" && strips > 1),
        "{mask:?}"
    );
    let mut pipeline = pollster::block_on(pipeline.update(&gpu, plan)).unwrap();
    let timings = pipeline.render(&gpu).unwrap();
    let ran: Vec<&str> = timings.passes.iter().map(|(name, _)| *name).collect();
    assert_eq!(ran[0], "rawprepare");
    assert!(ran.contains(&"opposed_inpaint"), "{ran:?}");
    for statistic in [
        "opposed_mask",
        "opposed_dilate",
        "opposed_sums",
        "opposed_chroma",
    ] {
        assert!(!ran.contains(&statistic), "{statistic} ran: {ran:?}");
    }
    let restarted = pipeline.output().download(&gpu).unwrap();

    let plan = scene.plan_within(&[], SMALL_VIEWPORT, &limits);
    let mut fresh = pollster::block_on(Pipeline::new(&gpu, plan, scene.upload(&gpu))).unwrap();
    fresh.render(&gpu).unwrap();
    let full = fresh.output().download(&gpu).unwrap();
    assert_eq!(restarted.len(), full.len());
    if let Some(i) = restarted
        .iter()
        .zip(&full)
        .position(|(a, b)| a.map(f32::to_bits) != b.map(f32::to_bits))
    {
        panic!(
            "pixel {i} is {:?} after the rebuild, {:?} in a fresh render",
            restarted[i], full[i]
        );
    }
}

/// The forced binding: demosaic's image of the large scenes takes about 6 MB, their
/// float mosaics 1.6 MB, the viewport image 425 KB.
const SMALL_BINDING: u64 = 512 << 10;

/// The adapter, opened to bind at most `SMALL_BINDING` bytes of a buffer at once, at
/// offsets aligned to 256 bytes, WebGPU's largest alignment.
fn small_gpu() -> GpuContext {
    let options = GpuOptions {
        max_storage_binding: Some(SMALL_BINDING),
        storage_offset_alignment: Some(256),
        ..GpuOptions::default()
    };
    let gpu =
        pollster::block_on(GpuContext::new(&options)).expect("a GPU adapter, hardware or software");
    assert_eq!(
        gpu.device.limits().max_storage_buffer_binding_size,
        SMALL_BINDING
    );
    gpu
}

const SMALL_VIEWPORT: Pipe = Pipe::Viewport {
    width: 200,
    height: 200,
};

/// The viewport of `scene` on a GPU binding `SMALL_BINDING` at once, in tiles, equals
/// the untiled viewport on the adapter as it is, bit for bit.
fn tiles_equal_the_whole(scene: &Scene) {
    let gpu = default_gpu();
    let untiled = scene.plan(&gpu, &[], SMALL_VIEWPORT);
    assert!(untiled.tiled.is_none());
    let whole = scene.fresh(&gpu, &[], SMALL_VIEWPORT);

    let small = small_gpu();
    let plan = scene.plan(&small, &[], SMALL_VIEWPORT);
    let tiled = plan.tiled.clone().expect("tiles on the small GPU");
    let tiles = plan.operations[tiled]
        .iter()
        .filter(|op| op.id == "downscale")
        .count();
    assert!(tiles > 4, "{tiles} tiles");
    assert!(plan.fits(&Limits::of(&small)));
    let banded = scene.fresh(&small, &[], SMALL_VIEWPORT);
    assert_eq!(banded.len(), whole.len());
    if let Some(i) = banded
        .iter()
        .zip(&whole)
        .position(|(a, b)| a.map(f32::to_bits) != b.map(f32::to_bits))
    {
        panic!(
            "pixel {i} is {:?} in tiles, {:?} whole",
            banded[i], whole[i]
        );
    }
}

#[test]
fn a_tiled_bayer_viewport_equals_the_untiled_one() {
    tiles_equal_the_whole(&Scene::large(Pattern {
        width: 2,
        height: 2,
        values: vec![0, 1, 1, 2],
    }));
}

#[test]
fn a_tiled_x_trans_viewport_equals_the_untiled_one() {
    tiles_equal_the_whole(&Scene::large(Pattern {
        width: 6,
        height: 6,
        values: X_TRANS.to_vec(),
    }));
}

#[test]
fn a_tiled_viewport_restarts_where_the_change_does() {
    let gpu = small_gpu();
    let scene = Scene::large(Pattern {
        width: 2,
        height: 2,
        values: vec![0, 1, 1, 2],
    });
    let pipe = SMALL_VIEWPORT;
    let plan = scene.plan(&gpu, &[], pipe);
    let tiled = plan.tiled.clone().expect("tiles on the small GPU");
    let mut pipeline = pollster::block_on(Pipeline::new(&gpu, plan, scene.upload(&gpu))).unwrap();
    assert_eq!(pipeline.render(&gpu).unwrap().passes[0].0, "rawprepare");

    // Exposure reads the viewport image, a cache point after the tiles.
    let exposure = ("exposure", "exposure", 0.5);
    pipeline = scene.change(&gpu, pipeline, &[exposure], pipe, "exposure");
    let saturation = |value| ("agx", "look_saturation", value);
    pipeline = scene.change(
        &gpu,
        pipeline,
        &[exposure, saturation(1.2)],
        pipe,
        "exposure",
    );
    pipeline = scene.change(&gpu, pipeline, &[exposure, saturation(1.4)], pipe, "agx");
    // A raw-stage change before the tiles runs everything.
    let red = ("temperature", "red", 2.2);
    pipeline = scene.change(
        &gpu,
        pipeline,
        &[exposure, saturation(1.4), red],
        pipe,
        "rawprepare",
    );
    // The same tiles at a smaller viewport restart from the raw stage's output, a cache
    // point, since their images fit the buffers here; a tile that grew past them would
    // rebuild the pipeline instead.
    let smaller = Pipe::Viewport {
        width: 180,
        height: 180,
    };
    let settings = [exposure, saturation(1.4), red];
    let plan = scene.plan(&gpu, &settings, smaller);
    assert_eq!(plan.tiled, Some(tiled));
    let mut pipeline = pollster::block_on(pipeline.update(&gpu, plan)).unwrap();
    assert_eq!(pipeline.render(&gpu).unwrap().passes[0].0, "region");
    let restarted = pipeline.output().download(&gpu).unwrap();
    let full = scene.fresh(&gpu, &settings, smaller);
    assert!(
        restarted
            .iter()
            .zip(&full)
            .all(|(a, b)| a.map(f32::to_bits) == b.map(f32::to_bits))
    );
}
