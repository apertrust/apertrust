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

//! The zoomed view's detail pipeline at 100% (decision 99; `docs/engine.md`, The zoomed
//! view), on a GPU adapter, hardware or software: planned as an export tile up to its
//! start and as the viewport after it, over demosaic's full-resolution image, which it
//! reads in place from the fitted viewport pipeline by its key. Its frame is the same
//! pixels of the untiled export bit for bit, a change restarts it at its edit cache, and
//! the plans of every other pipe keep their bytes and keys.

mod common;

use apertrust_engine::compute::{GpuImage, Timings};
use apertrust_engine::export::Tiling;
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe, Pipeline, Plan, PlanError};
use apertrust_plugin::ImageDesc;
use common::fingerprint::{fnv, keys_before_geometry, structure};
use common::raw::Raw;

const VIEWPORT: Pipe = Pipe::Viewport {
    width: 40,
    height: 40,
};

/// A small Bayer raw, its declared crop inside its active area: 84 × 62.
fn bayer() -> Raw {
    Raw::bayer((96, 72), [2, 2, 92, 68], [6, 4, 84, 62])
}

/// A small X-Trans raw: 108 × 84.
fn x_trans() -> Raw {
    Raw::x_trans((120, 96), [1, 3, 116, 90], [4, 5, 108, 84])
}

/// A raw as these tests develop it: through the fitted viewport, its detail at 100% over
/// the viewport's demosaic image, and the untiled export.
trait Scene {
    /// The default pipeline with these parameters set ([`common::instances`]).
    fn with(&self, settings: &[(&str, &str, f32)]) -> Vec<Instance<'static>>;
    fn plan(&self, instances: &[Instance], pipe: Pipe) -> Result<Plan, PlanError>;
    /// The fitted viewport pipeline of `instances`, rendered.
    fn fitted(&self, gpu: &GpuContext, instances: &[Instance]) -> Pipeline;
    /// The plan of the detail of `region` at 100% over `fitted`'s demosaic image, keyed
    /// on it, and that image.
    fn detail_plan(
        &self,
        fitted: &Pipeline,
        instances: &[Instance],
        region: [u32; 4],
    ) -> (Plan, GpuImage);
    /// A fresh detail pipeline of `region` over `fitted`'s demosaic image, rendered, and
    /// its timings.
    fn detail(
        &self,
        gpu: &GpuContext,
        fitted: &Pipeline,
        instances: &[Instance],
        region: [u32; 4],
    ) -> (Pipeline, Timings);
    /// The untiled export of `instances`: its width and pixels.
    fn whole(&self, gpu: &GpuContext, instances: &[Instance]) -> (u32, Vec<[f32; 4]>);
}

impl Scene for Raw {
    fn with(&self, settings: &[(&str, &str, f32)]) -> Vec<Instance<'static>> {
        common::instances(&self.image(), settings)
    }

    fn plan(&self, instances: &[Instance], pipe: Pipe) -> Result<Plan, PlanError> {
        pipeline::plan(instances, &self.image(), self.sensor(), pipe, &Limits::NONE)
    }

    fn fitted(&self, gpu: &GpuContext, instances: &[Instance]) -> Pipeline {
        let plan = self.plan(instances, VIEWPORT).unwrap();
        assert!(plan.tiled.is_none());
        let mut fitted = pollster::block_on(Pipeline::new(gpu, plan, self.upload(gpu))).unwrap();
        fitted.render(gpu).unwrap();
        fitted
    }

    fn detail_plan(
        &self,
        fitted: &Pipeline,
        instances: &[Instance],
        region: [u32; 4],
    ) -> (Plan, GpuImage) {
        let (source, key) = fitted.demosaiced().expect("demosaic's image, held");
        let pipe = Pipe::Detail { region, scale: 1.0 };
        let plan = pipeline::plan(instances, &self.image(), source.desc(), pipe, &Limits::NONE)
            .unwrap()
            .keyed_on(key);
        (plan, source)
    }

    fn detail(
        &self,
        gpu: &GpuContext,
        fitted: &Pipeline,
        instances: &[Instance],
        region: [u32; 4],
    ) -> (Pipeline, Timings) {
        let (plan, source) = self.detail_plan(fitted, instances, region);
        let mut detail = pollster::block_on(Pipeline::new(gpu, plan, source)).unwrap();
        let timings = detail.render(gpu).unwrap();
        (detail, timings)
    }

    fn whole(&self, gpu: &GpuContext, instances: &[Instance]) -> (u32, Vec<[f32; 4]>) {
        let exported = self.export(gpu, instances, Tiling::Whole);
        (exported.desc.width, exported.pixels)
    }
}

fn gpu() -> GpuContext {
    pollster::block_on(GpuContext::new(&GpuOptions::default()))
        .expect("a GPU adapter, hardware or software")
}

/// The pixels of `region` of an image `width` across.
fn cut(pixels: &[[f32; 4]], width: u32, [x, y, w, h]: [u32; 4]) -> Vec<[f32; 4]> {
    (y..y + h)
        .flat_map(|row| {
            let start = (row * width + x) as usize;
            pixels[start..start + w as usize].iter().copied()
        })
        .collect()
}

/// The first pixel where `a` and `b` differ in a bit, if any.
fn first_difference(a: &[[f32; 4]], b: &[[f32; 4]]) -> Option<usize> {
    assert_eq!(a.len(), b.len());
    (a.iter().zip(b)).position(|(p, q)| p.map(f32::to_bits) != q.map(f32::to_bits))
}

/// Regions of an image `width` × `height`: its four corners, its middle, a band across
/// its whole width, one down its whole height, and all of it.
fn regions(width: u32, height: u32) -> Vec<[u32; 4]> {
    let (w, h) = (31, 23);
    vec![
        [0, 0, w, h],
        [width - w, 0, w, h],
        [0, height - h, w, h],
        [width - w, height - h, w, h],
        [width / 2 - 13, height / 2 - 9, 27, 19],
        [0, height / 3, width, 17],
        [width / 3, 0, 19, height],
        [0, 0, width, height],
    ]
}

/// For each region, the detail frame of `instances` at 100% is the same pixels of the
/// untiled export, bit for bit.
fn equals_the_exports_regions(gpu: &GpuContext, scene: &Raw, instances: &[Instance]) {
    let fitted = scene.fitted(gpu, instances);
    let (width, exported) = scene.whole(gpu, instances);
    let height = (exported.len() / width as usize) as u32;
    assert_eq!([width, height], fitted.plan().extent());
    for region in regions(width, height) {
        let (detail, _) = scene.detail(gpu, &fitted, instances, region);
        let desc = detail.output().desc();
        assert_eq!([desc.width, desc.height], [region[2], region[3]]);
        let pixels = detail.output().download(gpu).unwrap();
        let expected = cut(&exported, width, region);
        let difference = first_difference(&pixels, &expected);
        assert_eq!(difference, None, "region {region:?}");
    }
}

#[test]
fn a_detail_at_100_equals_the_exports_region() {
    let gpu = gpu();
    for scene in [bayer(), x_trans()] {
        equals_the_exports_regions(&gpu, &scene, &scene.with(&[]));
    }
}

#[test]
fn a_detail_of_a_turned_photo_equals_the_exports_region() {
    // Orientation's quarter turn, its axes swapped and its columns flipped (darktable's
    // flip bits 6, choice 7 of its schema), and its other turn.
    let gpu = gpu();
    for scene in [bayer(), x_trans()] {
        for choice in [7.0, 6.0] {
            let instances = scene.with(&[("flip", "orientation", choice)]);
            let plan = scene.plan(&instances, Pipe::Export).unwrap();
            let [width, height] = plan.extent();
            assert_eq!([height, width], [scene.declared[2], scene.declared[3]]);
            equals_the_exports_regions(&gpu, &scene, &instances);
        }
    }
}

/// The detail of `region` updated to `settings`, rendered: it began at `starts_at`, and
/// equals a fresh detail pipeline's frame bit for bit.
fn change(
    gpu: &GpuContext,
    scene: &Raw,
    fitted: Pipeline,
    detail: Pipeline,
    settings: &[(&str, &str, f32)],
    region: [u32; 4],
    starts_at: &str,
) -> (Pipeline, Pipeline) {
    let instances = scene.with(settings);
    let plan = scene.plan(&instances, VIEWPORT).unwrap();
    let mut fitted = pollster::block_on(fitted.update(gpu, plan)).unwrap();
    fitted.render(gpu).unwrap();
    let (plan, _) = scene.detail_plan(&fitted, &instances, region);
    let mut detail = pollster::block_on(detail.update(gpu, plan)).unwrap();
    let timings = detail.render(gpu).unwrap();
    assert_eq!(timings.passes[0].0, starts_at, "{settings:?}");
    let restarted = detail.output().download(gpu).unwrap();
    let (fresh, _) = scene.detail(gpu, &fitted, &instances, region);
    let fresh = fresh.output().download(gpu).unwrap();
    assert_eq!(first_difference(&restarted, &fresh), None, "{settings:?}");
    (fitted, detail)
}

#[test]
fn a_detail_restarts_at_its_edit_cache() {
    let gpu = gpu();
    let scene = bayer();
    let region = [30, 20, 40, 30];
    let fitted = scene.fitted(&gpu, &scene.with(&[]));
    let (detail, first) = scene.detail(&gpu, &fitted, &scene.with(&[]), region);
    assert_eq!(first.passes[0].0, "region");
    // Exposure reads the region's copy, a cache point: the render starts at exposure.
    let exposure = ("exposure", "exposure", 0.5);
    let (fitted, detail) = change(
        &gpu,
        &scene,
        fitted,
        detail,
        &[exposure],
        region,
        "exposure",
    );
    // The first change to AgX starts at exposure, keeping AgX's input in the edit
    // cache; the next starts at AgX.
    let saturation = |value| ("agx", "look_saturation", value);
    let (fitted, detail) = change(
        &gpu,
        &scene,
        fitted,
        detail,
        &[exposure, saturation(1.2)],
        region,
        "exposure",
    );
    let (fitted, detail) = change(
        &gpu,
        &scene,
        fitted,
        detail,
        &[exposure, saturation(1.4)],
        region,
        "agx",
    );
    // A change to the raw stage changes the fitted pipeline's demosaic image, whose key
    // the detail's start chains from, though not its description: the detail starts at
    // its copy again. (One that changes its description too, as white balance's
    // coefficients, needs a detail pipeline of its own over it, as the session builds.)
    let clip = ("highlights", "clip", 0.8);
    change(
        &gpu,
        &scene,
        fitted,
        detail,
        &[exposure, saturation(1.4), clip],
        region,
        "region",
    );
}

#[test]
fn a_detail_reads_demosaics_image_in_place() {
    let gpu = gpu();
    let scene = bayer();
    let instances = scene.with(&[]);
    let fitted = scene.fitted(&gpu, &instances);
    let (source, _) = fitted.demosaiced().unwrap();
    let full = source.desc();
    let region = [10, 10, 40, 30];
    let (detail, timings) = scene.detail(&gpu, &fitted, &instances, region);
    // It starts at its copy of the region, and runs none of the raw stage's passes.
    let raw_stage = ["rawprepare", "temperature", "highlights", "demosaic"];
    let ran: Vec<&str> = timings.passes.iter().map(|(name, _)| *name).collect();
    assert_eq!(ran[0], "region");
    assert!(ran.iter().all(|name| !raw_stage.contains(name)), "{ran:?}");
    let plan = detail.plan();
    assert!(plan.operations.iter().all(|op| !raw_stage.contains(&op.id)));
    // Its input is the fitted pipeline's demosaic image; every buffer of its own is the
    // region's size.
    assert_eq!(plan.input, full);
    let region_bytes = ImageDesc::rgba(region[2], region[3]).buffer_size();
    for desc in plan.work.iter().chain(&plan.edit_cache) {
        assert!(desc.buffer_size() <= region_bytes, "{desc}");
    }
    assert!(region_bytes < full.buffer_size());
    // The copy is a cache point, and the edit cache starts after it; a detail has no
    // demosaic image of its own to lend.
    assert_eq!(plan.cached, [0]);
    assert_eq!(plan.operations[0].id, "region");
    assert!(detail.demosaiced().is_none());
}

#[test]
fn a_detail_refuses_whole_image_passes_after_demosaic() {
    let scene = bayer();
    let source = {
        let plan = scene.plan(&scene.with(&[]), VIEWPORT).unwrap();
        plan.operations[plan.cached[0]].output_desc
    };
    let detail = Pipe::Detail {
        region: [10, 10, 40, 30],
        scale: 1.0,
    };
    let plan = |instances: &[Instance]| {
        pipeline::plan(instances, &scene.image(), source, detail, &Limits::NONE)
    };
    assert!(plan(&scene.with(&[])).is_ok());
    let toned = scene.with(&[("toneequal", "noise", 0.1)]);
    assert!(
        toned
            .iter()
            .any(|i| i.operation.manifest().id == "toneequal"),
        "the tone equalizer on"
    );
    match plan(&toned) {
        Err(PlanError::Detail { operation, .. }) => assert_eq!(operation, "toneequal"),
        other => panic!("refused, not {other:?}"),
    }
    // A region past the image's edge is refused, not clipped.
    let at = |region| {
        let pipe = Pipe::Detail { region, scale: 1.0 };
        pipeline::plan(
            &scene.with(&[]),
            &scene.image(),
            source,
            pipe,
            &Limits::NONE,
        )
    };
    // An empty one too, and one whose end does not fit 32 bits.
    for region in [
        [60, 10, 40, 30],
        [10, 50, 40, 30],
        [10, 10, 0, 30],
        [u32::MAX, 0, 2, 2],
    ] {
        let planned = at(region);
        let refused = matches!(planned, Err(PlanError::Detail { .. }));
        assert!(refused, "{region:?}: {planned:?}");
    }
    // Only at 100% so far.
    let half = Pipe::Detail {
        region: [10, 10, 40, 30],
        scale: 0.5,
    };
    let refused = pipeline::plan(
        &scene.with(&[]),
        &scene.image(),
        source,
        half,
        &Limits::NONE,
    );
    assert!(
        matches!(refused, Err(PlanError::Detail { .. })),
        "{refused:?}"
    );
}

/// The fitted viewport's and the export's plans of the default pipeline, Bayer and
/// X-Trans, keep the bytes and keys they had before the detail pipe. The fingerprints are
/// those plans' as they were then: the X-Trans export's is the one `geometry.rs` recorded
/// long before, and the others were taken with the planner unchanged by it.
#[test]
fn plans_of_other_pipes_keep_their_bytes() {
    let fingerprints = [
        ("bayer viewport", 0xe172_0d1c_6863_50f9),
        ("bayer export", 0x5e56_255d_dabf_030c),
        ("x-trans viewport", 0x4b7b_9ba7_b1dd_3982),
        ("x-trans export", 0x9dc1_89df_b56f_4d9b),
    ];
    let mut plans = Vec::new();
    for (name, scene) in [("bayer", bayer()), ("x-trans", x_trans())] {
        let instances = scene.with(&[]);
        for (pipe, pipe_name) in [(VIEWPORT, "viewport"), (Pipe::Export, "export")] {
            plans.push((
                format!("{name} {pipe_name}"),
                scene.plan(&instances, pipe).unwrap(),
            ));
        }
    }
    for ((name, plan), (expected_name, expected)) in plans.iter().zip(fingerprints) {
        assert_eq!(name, expected_name);
        assert_eq!(fnv(&structure(plan)), expected, "{name}'s plan");
        let planned: Vec<(u64, Option<u64>)> = (plan.operations.iter())
            .map(|op| (op.key, op.whole_image.map(|whole| whole.key)))
            .collect();
        assert_eq!(planned, keys_before_geometry(plan), "{name}'s keys");
    }
}
