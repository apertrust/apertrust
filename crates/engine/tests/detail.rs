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

//! The zoomed view's detail pipeline (decision 99; `docs/engine.md`, The zoomed view), on
//! a GPU adapter, hardware or software: planned as an export tile up to its start and as
//! the viewport after it, over demosaic's full-resolution image, which it reads in place
//! from the fitted viewport pipeline by its key. At 100% its frame is the same pixels of
//! the untiled export bit for bit, with every operation that has whole-image passes too,
//! their results computed over the whole image in rounds of their own, held by keys with
//! no region in them, haze removal's borrowed from the fitted pipeline; below 100% it is
//! the same pixels of the fitted viewport at its scale, its rounds at that scale. A
//! change restarts it at its edit cache, and the plans of every other pipe keep their
//! bytes and keys.

mod common;

use apertrust_engine::compute::{GpuImage, Timings};
use apertrust_engine::detail::Results;
use apertrust_engine::export::{ExportError, Tiling};
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::pipeline::{
    self, Instance, Limits, Pipe, Pipeline, Plan, PlanError, PlannedOperation,
};
use apertrust_operations::{bilat, bloom, colorequal, hazeremoval, soften, toneequal};
use apertrust_plugin::{Format, ImageDesc, Operation, ParamValues};
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

/// A detail at its scale: its whole-image results, its own pipeline, rendered after the rounds
/// its frame ran, those rounds' operations, and the timings of its own passes.
struct Detail {
    results: Results,
    pipeline: Pipeline,
    rounds: Vec<&'static str>,
    /// The passes those rounds ran.
    round_passes: Vec<&'static str>,
    timings: Timings,
}

/// A raw as these tests develop it: through the fitted viewport, its detail over the
/// viewport's demosaic image, and the untiled export.
trait Scene {
    /// The default pipeline with these parameters set ([`common::instances`]).
    fn with(&self, settings: &[(&str, &str, f32)]) -> Vec<Instance<'static>>;
    /// The same, and `operation` with `params`.
    fn adding(
        &self,
        settings: &[(&str, &str, f32)],
        operation: &'static dyn Operation,
        params: ParamValues,
    ) -> Vec<Instance<'static>>;
    fn plan(&self, instances: &[Instance], pipe: Pipe) -> Result<Plan, PlanError>;
    /// The fitted viewport pipeline of `instances` for `viewport` within `limits`,
    /// rendered: in tiles where they ask (decision 14).
    fn fitted_within(
        &self,
        gpu: &GpuContext,
        instances: &[Instance],
        limits: &Limits,
        viewport: Pipe,
    ) -> Pipeline;
    /// The fitted viewport pipeline of `instances` in 40 × 40, untiled, rendered.
    fn fitted(&self, gpu: &GpuContext, instances: &[Instance]) -> Pipeline {
        let fitted = self.fitted_within(gpu, instances, &Limits::NONE, VIEWPORT);
        assert!(fitted.plan().tiled.is_none());
        fitted
    }
    /// The detail of `region` at `scale` over `fitted`'s source within `limits`,
    /// demosaic's image or a tiled viewport's raw-stage output, as `results` plans it: its
    /// plan, keyed on that image, its rounds, and that image.
    fn detail_plan_at(
        &self,
        results: &mut Results,
        fitted: &Pipeline,
        instances: &[Instance],
        region: [u32; 4],
        scale: f64,
        limits: &Limits,
    ) -> Result<(Plan, apertrust_engine::detail::Rounds, GpuImage), ExportError>;
    /// [`Scene::detail_plan_at`] at 100%.
    fn detail_plan(
        &self,
        results: &mut Results,
        fitted: &Pipeline,
        instances: &[Instance],
        region: [u32; 4],
        limits: &Limits,
    ) -> Result<(Plan, apertrust_engine::detail::Rounds, GpuImage), ExportError> {
        self.detail_plan_at(results, fitted, instances, region, 1.0, limits)
    }
    /// `old`, or a fresh detail where it is `None`, rendered for `instances` at `region`
    /// and `scale` over `fitted`'s demosaic image within `limits`: its results planned,
    /// its pipeline updated or built, its rounds run, then its own passes.
    #[allow(clippy::too_many_arguments, reason = "a detail's whole description")]
    fn render_at(
        &self,
        gpu: &GpuContext,
        fitted: &Pipeline,
        old: Option<Detail>,
        instances: &[Instance],
        region: [u32; 4],
        scale: f64,
        limits: &Limits,
    ) -> Detail;
    /// [`Scene::render_at`] at 100%.
    fn render(
        &self,
        gpu: &GpuContext,
        fitted: &Pipeline,
        old: Option<Detail>,
        instances: &[Instance],
        region: [u32; 4],
        limits: &Limits,
    ) -> Detail {
        self.render_at(gpu, fitted, old, instances, region, 1.0, limits)
    }
    /// A fresh detail of `region` over `fitted`'s demosaic image, rendered.
    fn detail(
        &self,
        gpu: &GpuContext,
        fitted: &Pipeline,
        instances: &[Instance],
        region: [u32; 4],
    ) -> Detail {
        self.render(gpu, fitted, None, instances, region, &Limits::NONE)
    }
    /// The untiled export of `instances`: its width and pixels.
    fn whole(&self, gpu: &GpuContext, instances: &[Instance]) -> (u32, Vec<[f32; 4]>);
}

impl Scene for Raw {
    fn with(&self, settings: &[(&str, &str, f32)]) -> Vec<Instance<'static>> {
        common::instances(&self.image(), settings)
    }

    fn adding(
        &self,
        settings: &[(&str, &str, f32)],
        operation: &'static dyn Operation,
        params: ParamValues,
    ) -> Vec<Instance<'static>> {
        let mut instances = self.with(settings);
        instances.push(Instance { operation, params });
        instances
    }

    fn plan(&self, instances: &[Instance], pipe: Pipe) -> Result<Plan, PlanError> {
        pipeline::plan(instances, &self.image(), self.sensor(), pipe, &Limits::NONE)
    }

    fn fitted_within(
        &self,
        gpu: &GpuContext,
        instances: &[Instance],
        limits: &Limits,
        viewport: Pipe,
    ) -> Pipeline {
        let image = self.image();
        let plan = pipeline::plan(instances, &image, self.sensor(), viewport, limits).unwrap();
        let mut fitted = pollster::block_on(Pipeline::new(gpu, plan, self.upload(gpu))).unwrap();
        fitted.render(gpu).unwrap();
        fitted
    }

    fn detail_plan_at(
        &self,
        results: &mut Results,
        fitted: &Pipeline,
        instances: &[Instance],
        region: [u32; 4],
        scale: f64,
        limits: &Limits,
    ) -> Result<(Plan, apertrust_engine::detail::Rounds, GpuImage), ExportError> {
        let (source, key) = fitted.detail_source().expect("the viewport's source, held");
        let instances = &pipeline::detail_instances(instances, fitted.plan());
        let pipe = Pipe::Detail { region, scale };
        let image = self.image();
        let planned = pipeline::plan(instances, &image, source.desc(), pipe, limits)?;
        let (plan, rounds) = results.plan(planned, key, instances, &image, limits, Some(fitted))?;
        Ok((plan, rounds, source))
    }

    fn render_at(
        &self,
        gpu: &GpuContext,
        fitted: &Pipeline,
        old: Option<Detail>,
        instances: &[Instance],
        region: [u32; 4],
        scale: f64,
        limits: &Limits,
    ) -> Detail {
        let (mut results, old) = match old {
            Some(old) => (old.results, Some(old.pipeline)),
            None => (Results::default(), None),
        };
        let (plan, rounds, source) = self
            .detail_plan_at(&mut results, fitted, instances, region, scale, limits)
            .unwrap();
        let built = results.pipeline(gpu, plan, source.clone(), old);
        let mut pipeline = pollster::block_on(built).unwrap();
        let ran = rounds.operations();
        let round_timings = pollster::block_on(results.run(gpu, rounds, &source)).unwrap();
        let timings = pipeline.render(gpu).unwrap();
        Detail {
            results,
            pipeline,
            rounds: ran,
            round_passes: (round_timings.passes.iter())
                .map(|(name, _)| *name)
                .collect(),
            timings,
        }
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
/// its whole width, one down its whole height, and all of it. Their edges fall at odd
/// places, between a tone equalizer grid's samples.
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

/// The detail's frame equals the same pixels of `exported`, an image `width` across.
fn assert_region(
    gpu: &GpuContext,
    detail: &Detail,
    exported: &[[f32; 4]],
    width: u32,
    region: [u32; 4],
) {
    let output = detail.pipeline.output();
    let desc = output.desc();
    assert_eq!([desc.width, desc.height], [region[2], region[3]]);
    let pixels = output.download(gpu).unwrap();
    let expected = cut(exported, width, region);
    let difference = first_difference(&pixels, &expected);
    assert_eq!(difference, None, "region {region:?}");
}

/// For each region, the detail frame of `instances` at 100% within `limits` is the same
/// pixels of the untiled export, bit for bit, and each fresh detail runs the rounds of
/// `rounds`.
fn equals_the_exports_regions_within(
    gpu: &GpuContext,
    scene: &Raw,
    instances: &[Instance],
    limits: &Limits,
    rounds: &[&str],
) {
    let fitted = scene.fitted(gpu, instances);
    let (width, exported) = scene.whole(gpu, instances);
    let height = (exported.len() / width as usize) as u32;
    assert_eq!([width, height], fitted.plan().extent());
    for region in regions(width, height) {
        let detail = scene.render(gpu, &fitted, None, instances, region, limits);
        assert_region(gpu, &detail, &exported, width, region);
        assert_eq!(detail.rounds, rounds, "region {region:?}");
    }
}

/// [`equals_the_exports_regions_within`] with no limits.
fn equals_the_exports_regions(
    gpu: &GpuContext,
    scene: &Raw,
    instances: &[Instance],
    rounds: &[&str],
) {
    equals_the_exports_regions_within(gpu, scene, instances, &Limits::NONE, rounds);
}

#[test]
fn a_detail_at_100_equals_the_exports_region() {
    let gpu = gpu();
    for scene in [bayer(), x_trans()] {
        equals_the_exports_regions(&gpu, &scene, &scene.with(&[]), &[]);
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
            equals_the_exports_regions(&gpu, &scene, &instances, &[]);
        }
    }
}

/// The tone equalizer at test 0131's bands, its mask filtered by `filter` iterated twice.
fn tone_equalizer(filter: toneequal::Filter) -> ParamValues {
    use apertrust_operations::toneequal::{ToneEqual, settings, values};
    let mut tone = settings(&ToneEqual.schema().defaults());
    tone.bands = [
        -1.283, -0.788, 0.445, 1.148, 1.321, 0.994, 0.211, -0.449, -0.755,
    ];
    (tone.details, tone.iterations) = (filter, 2);
    (tone.blending, tone.quantization, tone.exposure_boost) = (10.0, 0.5, 1.0);
    values(&tone)
}

#[test]
fn a_detail_with_the_tone_equalizer_equals_the_exports_region() {
    let gpu = gpu();
    for scene in [bayer(), x_trans()] {
        for filter in [toneequal::Filter::Eigf, toneequal::Filter::AveragedEigf] {
            let params = tone_equalizer(filter);
            let instances = scene.adding(&[], &toneequal::ToneEqual, params);
            equals_the_exports_regions(&gpu, &scene, &instances, &["toneequal"]);
        }
    }
}

#[test]
fn a_detail_with_bloom_equals_the_exports_region() {
    let gpu = gpu();
    let scene = bayer();
    let small = bloom::Settings {
        size: 0.0,
        threshold: 40.0,
        strength: 60.0,
    };
    for settings in [bloom::Settings::default(), small] {
        let instances = scene.adding(&[], &bloom::Bloom, bloom::values(&settings));
        equals_the_exports_regions(&gpu, &scene, &instances, &["bloom"]);
    }
}

#[test]
fn a_detail_with_soften_equals_the_exports_region() {
    let gpu = gpu();
    let scene = bayer();
    let largest = soften::Settings {
        size: 100.0,
        saturation: 40.0,
        brightness: 1.0,
        amount: 80.0,
    };
    for settings in [soften::Settings::default(), largest] {
        let instances = scene.adding(&[], &soften::Soften, soften::values(&settings));
        equals_the_exports_regions(&gpu, &scene, &instances, &["soften"]);
    }
}

#[test]
fn a_detail_with_the_colour_equalizer_equals_the_exports_region() {
    // With its guided filter, which gives it its whole-image passes, at an effect radius
    // of 25 with curves.
    let gpu = gpu();
    let scene = bayer();
    let curved = colorequal::Settings {
        param_size: 25.0,
        chroma_size: 2.0,
        use_filter: true,
        saturation: [1.3, 1.0, 0.7, 1.0, 1.3, 1.0, 0.7, 1.0],
        hue: [40.0, -20.0, 0.0, 0.0, 40.0, 0.0, -45.0, 0.0],
        brightness: [1.0, 1.2, 1.0, 0.8, 1.0, 1.3, 1.0, 0.7],
        ..colorequal::Settings::default()
    };
    let params = colorequal::values(&curved);
    let instances = scene.adding(&[], &colorequal::ColorEqual, params);
    equals_the_exports_regions(&gpu, &scene, &instances, &["colorequal"]);
}

/// Haze removal at test 0166's settings.
fn haze_removal() -> ParamValues {
    hazeremoval::values(&hazeremoval::Settings {
        strength: 0.9,
        distance: 0.8,
    })
}

#[test]
fn a_detail_with_haze_removal_equals_the_exports_region() {
    // Upright, its estimate at full resolution is the fitted pipeline's, borrowed: no
    // round. With a quarter turn after it the fitted pipeline estimates at its own scale,
    // so the detail estimates in a round of its own.
    let gpu = gpu();
    let scene = bayer();
    let upright = scene.adding(&[], &hazeremoval::HazeRemoval, haze_removal());
    equals_the_exports_regions(&gpu, &scene, &upright, &[]);
    let turn = [("flip", "orientation", 7.0)];
    let turned = scene.adding(&turn, &hazeremoval::HazeRemoval, haze_removal());
    equals_the_exports_regions(&gpu, &scene, &turned, &["hazeremoval"]);
}

#[test]
fn a_detail_with_local_contrast_equals_the_exports_region() {
    // Local laplacian, darktable's default, and the bilateral grid: neither opts into
    // tiles, so each round runs whole, with every operation before it.
    let gpu = gpu();
    let scene = bayer();
    let bilateral_grid = bilat::Settings {
        mode: bilat::Mode::BilateralGrid,
        sigma_s: 50.0,
        sigma_r: 20.0,
        detail: 1.0,
        ..bilat::Settings::default()
    };
    for settings in [bilat::Settings::default(), bilateral_grid] {
        let instances = scene.adding(&[], &bilat::Bilat, bilat::values(&settings));
        equals_the_exports_regions(&gpu, &scene, &instances, &["bilat"]);
    }
}

#[test]
fn a_detail_with_filmic_rgbs_reconstruction_equals_the_exports_region() {
    // Filmic rgb as the tone mapper in AgX's place, highlight reconstruction on at -3 EV,
    // where the raw's highlight counts as clipped: its wavelets in a round over the whole
    // image, with every operation before it.
    use apertrust_operations::filmicrgb::{FilmicRgb, Settings, values};
    let gpu = gpu();
    let scene = bayer();
    let settings = Settings {
        enable_highlight_reconstruction: true,
        reconstruct_threshold: -3.0,
        ..Settings::default()
    };
    let mut instances = scene.adding(&[], &FilmicRgb, values(&settings));
    instances.retain(|instance| instance.operation.manifest().id != "agx");
    equals_the_exports_regions(&gpu, &scene, &instances, &["filmicrgb"]);
}

#[test]
fn a_detail_whose_rounds_take_tiles_equals_the_exports_region() {
    // The tone equalizer and bloom, each a round of its own, bloom's binding the tone
    // equalizer's result, at the largest budget that cuts the first round into tiles.
    let gpu = gpu();
    let scene = bayer();
    let mut instances = scene.adding(
        &[],
        &toneequal::ToneEqual,
        tone_equalizer(toneequal::Filter::Eigf),
    );
    instances.push(Instance {
        operation: &bloom::Bloom,
        params: bloom::values(&bloom::Settings::default()),
    });
    let fitted = scene.fitted(&gpu, &instances);
    let region = [10, 10, 40, 30];
    let tiled = |budget: u64| {
        let limits = Limits {
            budget,
            ..Limits::NONE
        };
        let planned = scene.detail_plan(
            &mut Results::default(),
            &fitted,
            &instances,
            region,
            &limits,
        );
        planned.is_ok_and(|(_, rounds, _)| rounds.tiles().iter().all(|&tiles| tiles > 1))
    };
    // Budgets a sixteenth smaller each time, from far more than the whole image takes.
    let budget = std::iter::successors(Some(64u64 << 20), |budget| Some(budget * 15 / 16))
        .take_while(|&budget| budget > 0)
        .find(|&budget| tiled(budget))
        .expect("a budget that tiles both rounds");
    let limits = Limits {
        budget,
        ..Limits::NONE
    };
    equals_the_exports_regions_within(&gpu, &scene, &instances, &limits, &["toneequal", "bloom"]);
}

/// `detail`, at `region`, rendered again for `instances`, its fitted pipeline updated and
/// rendered first, as the session renders a change at 100%; the frame equals a fresh
/// detail's bit for bit.
fn change(
    gpu: &GpuContext,
    scene: &Raw,
    fitted: Pipeline,
    detail: Detail,
    instances: &[Instance],
    region: [u32; 4],
) -> (Pipeline, Detail) {
    let plan = scene.plan(instances, VIEWPORT).unwrap();
    let mut fitted = pollster::block_on(fitted.update(gpu, plan)).unwrap();
    fitted.render(gpu).unwrap();
    let limits = &Limits::NONE;
    let detail = scene.render(gpu, &fitted, Some(detail), instances, region, limits);
    let restarted = detail.pipeline.output().download(gpu).unwrap();
    let fresh = scene.detail(gpu, &fitted, instances, region);
    let fresh = fresh.pipeline.output().download(gpu).unwrap();
    let difference = first_difference(&restarted, &fresh);
    assert_eq!(difference, None, "{:?}", detail.rounds);
    (fitted, detail)
}

/// The names of the passes `timings` ran.
fn ran(timings: &Timings) -> Vec<&'static str> {
    timings.passes.iter().map(|(name, _)| *name).collect()
}

#[test]
fn a_detail_restarts_at_its_edit_cache() {
    let gpu = gpu();
    let scene = bayer();
    let region = [30, 20, 40, 30];
    let fitted = scene.fitted(&gpu, &scene.with(&[]));
    let detail = scene.detail(&gpu, &fitted, &scene.with(&[]), region);
    assert_eq!(ran(&detail.timings)[0], "region");
    let starts_at = |settings: &[(&str, &str, f32)], fitted, detail, at: &str| {
        let instances = scene.with(settings);
        let (fitted, detail) = change(&gpu, &scene, fitted, detail, &instances, region);
        assert_eq!(ran(&detail.timings)[0], at, "{settings:?}");
        (fitted, detail)
    };
    // Exposure reads the region's copy, a cache point: the render starts at exposure.
    let exposure = ("exposure", "exposure", 0.5);
    let (fitted, detail) = starts_at(&[exposure], fitted, detail, "exposure");
    // The first change to AgX starts at exposure, keeping AgX's input in the edit
    // cache; the next starts at AgX.
    let saturation = |value| ("agx", "look_saturation", value);
    let (fitted, detail) = starts_at(&[exposure, saturation(1.2)], fitted, detail, "exposure");
    let (fitted, detail) = starts_at(&[exposure, saturation(1.4)], fitted, detail, "agx");
    // A change to the raw stage changes the fitted pipeline's demosaic image, whose key
    // the detail's start chains from, though not its description: the detail starts at
    // its copy again. (One that changes its description too, as white balance's
    // coefficients, needs a detail pipeline of its own over it, as the session builds.)
    let clip = ("highlights", "clip", 0.8);
    starts_at(&[exposure, saturation(1.4), clip], fitted, detail, "region");
}

/// The tone equalizer's pipeline with `settings`, its fifth band moved by `band`.
fn toned(scene: &Raw, settings: &[(&str, &str, f32)], band: f32) -> Vec<Instance<'static>> {
    use apertrust_operations::toneequal::{settings as tone_settings, values};
    let mut tone = tone_settings(&tone_equalizer(toneequal::Filter::Eigf));
    tone.bands[4] += band;
    scene.adding(settings, &toneequal::ToneEqual, values(&tone))
}

#[test]
fn a_band_drag_at_100_runs_no_round() {
    // A band is read by `main` alone: the mask's key holds, and the region's passes run
    // from the tone equalizer's input, kept in the edit cache.
    let gpu = gpu();
    let scene = bayer();
    let region = [30, 20, 40, 30];
    let instances = toned(&scene, &[], 0.0);
    let mut fitted = scene.fitted(&gpu, &instances);
    let mut detail = scene.detail(&gpu, &fitted, &instances, region);
    assert_eq!(detail.rounds, ["toneequal"]);
    for band in [0.3, 0.6] {
        let instances = toned(&scene, &[], band);
        (fitted, detail) = change(&gpu, &scene, fitted, detail, &instances, region);
        assert!(detail.rounds.is_empty(), "{:?}", detail.rounds);
        let (width, exported) = scene.whole(&gpu, &instances);
        assert_region(&gpu, &detail, &exported, width, region);
    }
    let passes = ["toneequal", "colorin", "channelmixerrgb", "agx", "colorout"];
    assert_eq!(ran(&detail.timings), passes);
}

#[test]
fn an_exposure_drag_at_100_reruns_the_round() {
    let gpu = gpu();
    let scene = bayer();
    let region = [30, 20, 40, 30];
    let instances = toned(&scene, &[], 0.0);
    let mut fitted = scene.fitted(&gpu, &instances);
    let mut detail = scene.detail(&gpu, &fitted, &instances, region);
    for value in [0.3, 0.6] {
        let instances = toned(&scene, &[("exposure", "exposure", value)], 0.0);
        (fitted, detail) = change(&gpu, &scene, fitted, detail, &instances, region);
        assert_eq!(detail.rounds, ["toneequal"]);
        assert_eq!(ran(&detail.timings)[0], "exposure");
        let (width, exported) = scene.whole(&gpu, &instances);
        assert_region(&gpu, &detail, &exported, width, region);
    }
}

#[test]
fn a_pan_at_100_runs_no_round() {
    // A region elsewhere, of the same size, keeps the result's key, which holds no region.
    let gpu = gpu();
    let scene = bayer();
    let instances = toned(&scene, &[], 0.0);
    let fitted = scene.fitted(&gpu, &instances);
    let (width, exported) = scene.whole(&gpu, &instances);
    let mut detail = scene.detail(&gpu, &fitted, &instances, [0, 0, 40, 30]);
    assert_eq!(detail.rounds, ["toneequal"]);
    for region in [[30, 20, 40, 30], [44, 32, 40, 30], [7, 31, 40, 30]] {
        let old = Some(detail);
        detail = scene.render(&gpu, &fitted, old, &instances, region, &Limits::NONE);
        assert!(detail.rounds.is_empty(), "{region:?}: {:?}", detail.rounds);
        assert_region(&gpu, &detail, &exported, width, region);
    }
}

#[test]
fn a_fitted_view_at_scale_1_lends_its_whole_image_results() {
    // A photo no larger than the bounds: the fitted viewport runs at scale 1, and the
    // detail binds its whole-image results, the tone equalizer's and haze removal's,
    // running no round, its frame still the export's region.
    let gpu = gpu();
    let scene = bayer();
    let mut instances = toned(&scene, &[], 0.0);
    instances.push(Instance {
        operation: &hazeremoval::HazeRemoval,
        params: haze_removal(),
    });
    let bounds = Pipe::Viewport {
        width: 200,
        height: 200,
    };
    let plan = scene.plan(&instances, bounds).unwrap();
    let mut fitted = pollster::block_on(Pipeline::new(&gpu, plan, scene.upload(&gpu))).unwrap();
    fitted.render(&gpu).unwrap();
    let (width, exported) = scene.whole(&gpu, &instances);
    let region = [30, 20, 40, 30];
    let detail = scene.render(&gpu, &fitted, None, &instances, region, &Limits::NONE);
    assert!(detail.rounds.is_empty(), "{:?}", detail.rounds);
    assert_region(&gpu, &detail, &exported, width, region);
}

#[test]
fn haze_removal_at_100_borrows_its_estimate() {
    // Upright, the fitted pipeline estimates at full resolution: the detail binds its
    // result, runs no round, and of haze removal's passes only `main`; a strength change
    // runs `main` on.
    let estimate = [
        "image_copy",
        "image_rows",
        "plane_cols",
        "plane_rows",
        "select_begin",
        "select_count",
        "select_scan",
        "ambient_sums",
        "ambient_combine",
        "guide_coefficients",
        "guide_result",
        "plane_downscale",
    ];
    let gpu = gpu();
    let scene = bayer();
    let region = [30, 20, 40, 30];
    let instances = scene.adding(&[], &hazeremoval::HazeRemoval, haze_removal());
    let fitted = scene.fitted(&gpu, &instances);
    let detail = scene.detail(&gpu, &fitted, &instances, region);
    assert!(detail.rounds.is_empty(), "{:?}", detail.rounds);
    let passes = ran(&detail.timings);
    assert!(passes.contains(&"hazeremoval"), "{passes:?}");
    assert!(
        passes.iter().all(|pass| !estimate.contains(pass)),
        "{passes:?}"
    );
    // The detail makes none of the estimate's buffers: it binds what `main` reads of the
    // fitted pipeline's, plane A and the statistics.
    let plan = detail.pipeline.plan();
    let haze = (plan.operations.iter())
        .find(|op| op.id == "hazeremoval")
        .unwrap();
    let pixels = u64::from(scene.declared[2] * scene.declared[3]);
    let scratch: u64 = haze.scratch.iter().map(|&index| plan.scratch[index]).sum();
    assert!(scratch < 5 * pixels, "{scratch} bytes");
    let weaker = hazeremoval::values(&hazeremoval::Settings {
        strength: 0.5,
        distance: 0.8,
    });
    let instances = scene.adding(&[], &hazeremoval::HazeRemoval, weaker);
    let (_, detail) = change(&gpu, &scene, fitted, detail, &instances, region);
    assert!(detail.rounds.is_empty(), "{:?}", detail.rounds);
    let passes = ran(&detail.timings);
    assert_eq!(passes[0], "hazeremoval");
    assert!(
        passes.iter().all(|pass| !estimate.contains(pass)),
        "{passes:?}"
    );
}

#[test]
fn a_round_too_large_to_bind_refuses_the_view() {
    // Soften's planes are scratch a detail holds whole and binds whole, as an export's:
    // where one passes the GPU's binding, no tile helps, and the detail is refused with
    // the export's message, naming the operation, the pass and the bytes (decision 15).
    let scene = bayer();
    let softened = soften::values(&soften::Settings::default());
    let instances = scene.adding(&[], &soften::Soften, softened);
    let source = {
        let plan = scene.plan(&instances, VIEWPORT).unwrap();
        plan.operations[plan.cached[0]].output_desc
    };
    let limits = Limits {
        binding: 16 << 10,
        ..Limits::NONE
    };
    let pipe = Pipe::Detail {
        region: [10, 10, 40, 30],
        scale: 1.0,
    };
    let image = scene.image();
    let planned = pipeline::plan(&instances, &image, source, pipe, &limits).unwrap();
    let refused = Results::default().plan(planned, 0, &instances, &image, &limits, None);
    match refused {
        Err(
            error @ ExportError::HeldScratch {
                operation,
                bytes,
                limit,
                ..
            },
        ) => {
            assert_eq!((operation, limit), ("soften", 16 << 10));
            assert!(bytes > limit, "{bytes} bytes");
            assert!(error.to_string().starts_with("soften's "), "{error}");
        }
        other => panic!("refused, not {:?}", other.map(|(plan, _)| plan.pipe)),
    }
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
    let detail = scene.detail(&gpu, &fitted, &instances, region);
    // It starts at its copy of the region, and runs none of the raw stage's passes.
    let raw_stage = ["rawprepare", "temperature", "highlights", "demosaic"];
    let ran = ran(&detail.timings);
    assert_eq!(ran[0], "region");
    assert!(ran.iter().all(|name| !raw_stage.contains(name)), "{ran:?}");
    let plan = detail.pipeline.plan();
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
    assert!(detail.pipeline.demosaiced().is_none());
}

#[test]
fn a_detail_refuses_a_region_or_a_scale_it_cannot_render() {
    let scene = bayer();
    let instances = scene.with(&[]);
    let source = {
        let plan = scene.plan(&instances, VIEWPORT).unwrap();
        plan.operations[plan.cached[0]].output_desc
    };
    let at = |region, scale| {
        let pipe = Pipe::Detail { region, scale };
        pipeline::plan(&instances, &scene.image(), source, pipe, &Limits::NONE)
    };
    assert!(at([10, 10, 40, 30], 1.0).is_ok());
    // A region past the image's edge is refused, not clipped; an empty one too, and one
    // whose end does not fit 32 bits.
    for region in [
        [60, 10, 40, 30],
        [10, 50, 40, 30],
        [10, 10, 0, 30],
        [u32::MAX, 0, 2, 2],
    ] {
        let planned = at(region, 1.0);
        let refused = matches!(planned, Err(PlanError::Detail { .. }));
        assert!(refused, "{region:?}: {planned:?}");
    }
    // Below 100%, a region of the image at the scale, 42 × 31 at 50%; never above 100%,
    // nor at a scale not above 0.
    assert!(at([10, 10, 30, 20], 0.5).is_ok());
    for (region, scale) in [
        ([30, 10, 20, 10], 0.5),
        ([10, 10, 40, 30], 1.5),
        ([10, 10, 40, 30], 0.0),
        ([10, 10, 40, 30], -1.0),
        ([10, 10, 40, 30], f64::NAN),
    ] {
        let planned = at(region, scale);
        let refused = matches!(planned, Err(PlanError::Detail { .. }));
        assert!(refused, "{region:?} at {scale}: {planned:?}");
    }
}

/// The output image `extent` at full resolution at `scale`, each side rounded and at least
/// a pixel, as the view places it (`view::Placement::size`).
fn at_scale(extent: [u32; 2], scale: f64) -> [u32; 2] {
    extent.map(|side| ((f64::from(side) * scale).round() as u32).max(1))
}

/// The fitted viewport of `instances` planned with bounds of the whole image at `scale`,
/// rendered: its width and pixels, the image a detail at that scale is a region of
/// (decision 99).
fn fitted_at(
    gpu: &GpuContext,
    scene: &Raw,
    instances: &[Instance],
    scale: f64,
) -> (u32, Vec<[f32; 4]>) {
    let extent = scene.plan(instances, Pipe::Export).unwrap().extent();
    let [width, height] = at_scale(extent, scale);
    let fitted = scene.fitted_within(
        gpu,
        instances,
        &Limits::NONE,
        Pipe::Viewport { width, height },
    );
    let desc = fitted.output().desc();
    assert_eq!([desc.width, desc.height], [width, height], "at {scale}");
    (width, fitted.output().download(gpu).unwrap())
}

/// Regions of the smaller images below 100%: [`regions`]' places, at about a third of the
/// image's sides.
fn regions_below_100(width: u32, height: u32) -> Vec<[u32; 4]> {
    let (w, h) = (width / 3 + 1, height / 3 + 1);
    vec![
        [0, 0, w, h],
        [width - w, 0, w, h],
        [0, height - h, w, h],
        [width - w, height - h, w, h],
        [width / 2 - w / 2, height / 2 - h / 2, w, h],
        [0, height / 3, width, h / 2 + 1],
        [width / 3, 0, w / 2 + 1, height],
        [0, 0, width, height],
    ]
}

/// The scales the tests below 100% take: 50%, and one whose footprints are uneven.
const BELOW_100: [f64; 2] = [0.5, 0.37];

/// For each region of the image at `scale`, the detail frame of `instances` within
/// `limits` is the same pixels of the fitted viewport at that scale, bit for bit, and each
/// fresh detail runs the rounds of `rounds`.
fn equals_the_fitted_regions_within(
    gpu: &GpuContext,
    scene: &Raw,
    instances: &[Instance],
    scale: f64,
    limits: &Limits,
    rounds: &[&str],
) {
    let fitted = scene.fitted(gpu, instances);
    let (width, expected) = fitted_at(gpu, scene, instances, scale);
    let height = (expected.len() / width as usize) as u32;
    for region in regions_below_100(width, height) {
        let detail = scene.render_at(gpu, &fitted, None, instances, region, scale, limits);
        assert_region(gpu, &detail, &expected, width, region);
        assert_eq!(detail.rounds, rounds, "region {region:?} at {scale}");
    }
}

#[test]
fn a_detail_below_100_equals_the_fitted_viewport_at_its_scale() {
    // Upright and with orientation's quarter turn, its region walked back through it at
    // the scale; its start the downscale of the region from demosaic's image.
    let gpu = gpu();
    for scene in [bayer(), x_trans()] {
        for settings in [&[][..], &[("flip", "orientation", 7.0)]] {
            let instances = scene.with(settings);
            for scale in BELOW_100 {
                equals_the_fitted_regions_within(
                    &gpu,
                    &scene,
                    &instances,
                    scale,
                    &Limits::NONE,
                    &[],
                );
            }
        }
    }
    let scene = bayer();
    let fitted = scene.fitted(&gpu, &scene.with(&[]));
    let detail = scene.render_at(
        &gpu,
        &fitted,
        None,
        &scene.with(&[]),
        [5, 4, 20, 15],
        0.5,
        &Limits::NONE,
    );
    assert_eq!(ran(&detail.timings)[0], "main_region");
    let plan = detail.pipeline.plan();
    assert_eq!(
        (plan.operations[0].id, &plan.cached[..]),
        ("downscale", &[0][..])
    );
}

#[test]
fn a_detail_below_100_with_the_tone_equalizer_equals_the_fitted_viewport_at_its_scale() {
    // Its mask over the whole image at the scale, in a round of its own; once more at a
    // budget that cuts that round into tiles, each a detail at the scale, on a raw large
    // enough to take them.
    let gpu = gpu();
    for scene in [bayer(), x_trans()] {
        for filter in [toneequal::Filter::Eigf, toneequal::Filter::AveragedEigf] {
            let instances = scene.adding(&[], &toneequal::ToneEqual, tone_equalizer(filter));
            for scale in BELOW_100 {
                equals_the_fitted_regions_within(
                    &gpu,
                    &scene,
                    &instances,
                    scale,
                    &Limits::NONE,
                    &["toneequal"],
                );
            }
        }
    }
    let scene = medium_bayer();
    let instances = toned(&scene, &[], 0.0);
    let limits = tiling_budget(&gpu, &scene, &instances, 0.5);
    equals_the_fitted_regions_within(&gpu, &scene, &instances, 0.5, &limits, &["toneequal"]);
}

#[test]
fn a_detail_below_100_with_bloom_equals_the_fitted_viewport_at_its_scale() {
    let gpu = gpu();
    let scene = bayer();
    let small = bloom::Settings {
        size: 0.0,
        threshold: 40.0,
        strength: 60.0,
    };
    for settings in [bloom::Settings::default(), small] {
        let instances = scene.adding(&[], &bloom::Bloom, bloom::values(&settings));
        for scale in BELOW_100 {
            equals_the_fitted_regions_within(
                &gpu,
                &scene,
                &instances,
                scale,
                &Limits::NONE,
                &["bloom"],
            );
        }
    }
}

#[test]
fn a_detail_below_100_with_haze_removal_equals_the_fitted_viewport_at_its_scale() {
    // Upright, the fitted viewport at the scale estimates at full resolution and brings
    // the transmission's input to its scale with `plane_downscale`: the detail borrows
    // the 40 × 40 fitted pipeline's estimate, and its round runs `plane_downscale` alone,
    // or none where the scale's image is the fitted frame's size, whose result it borrows
    // whole. With a quarter turn after it, the fitted viewport estimates at its own scale,
    // as the detail does, in a round of its own, both at the scale the turned width
    // gives: at 36/62 the Bayer raw's width before the turn gives 49/84, and a dark
    // channel's radius of 4 rather than 3.
    let gpu = gpu();
    for scene in [bayer(), x_trans()] {
        for settings in [&[][..], &[("flip", "orientation", 7.0)]] {
            let instances = scene.adding(settings, &hazeremoval::HazeRemoval, haze_removal());
            let fitted = scene.fitted(&gpu, &instances);
            let fitted_size = fitted.output().desc();
            for scale in [BELOW_100[0], BELOW_100[1], 36.0 / 62.0] {
                let (width, expected) = fitted_at(&gpu, &scene, &instances, scale);
                let height = (expected.len() / width as usize) as u32;
                let whole = [width, height] == [fitted_size.width, fitted_size.height];
                for region in regions_below_100(width, height) {
                    let limits = &Limits::NONE;
                    let detail =
                        scene.render_at(&gpu, &fitted, None, &instances, region, scale, limits);
                    assert_region(&gpu, &detail, &expected, width, region);
                    let at = format!("{settings:?} {region:?} at {scale}");
                    let round = &detail.round_passes;
                    if !settings.is_empty() {
                        assert_eq!(detail.rounds, ["hazeremoval"], "{at}");
                        assert!(round.contains(&"image_copy"), "{at}: {round:?}");
                    } else if whole {
                        assert!(detail.rounds.is_empty(), "{at}: {:?}", detail.rounds);
                    } else {
                        assert_eq!(detail.rounds, ["hazeremoval"], "{at}");
                        assert_eq!(round, &["plane_downscale"], "{at}");
                    }
                }
            }
        }
    }
}

/// The largest budget, a sixteenth smaller each time from far more than the whole image
/// takes, at which every round of a detail of `instances` at `scale` takes more than one
/// tile.
fn tiling_budget(gpu: &GpuContext, scene: &Raw, instances: &[Instance], scale: f64) -> Limits {
    let fitted = scene.fitted(gpu, instances);
    let tiled = |budget: u64| {
        let limits = Limits {
            budget,
            ..Limits::NONE
        };
        let fresh = &mut Results::default();
        let planned =
            scene.detail_plan_at(fresh, &fitted, instances, [5, 5, 20, 15], scale, &limits);
        planned.is_ok_and(|(_, rounds, _)| rounds.tiles().iter().all(|&tiles| tiles > 1))
    };
    let budget = std::iter::successors(Some(64u64 << 20), |budget| Some(budget * 15 / 16))
        .take_while(|&budget| budget > 0)
        .find(|&budget| tiled(budget))
        .expect("a budget that tiles every round");
    Limits {
        budget,
        ..Limits::NONE
    }
}

#[test]
fn a_scale_change_reruns_the_rounds_and_a_pan_does_not() {
    // The tone equalizer's mask is held under the key of the whole image at the detail's
    // scale: a pan at that scale keeps it, a change of scale makes another, and going back
    // computes the first again.
    let gpu = gpu();
    let scene = bayer();
    let instances = toned(&scene, &[], 0.0);
    let fitted = scene.fitted(&gpu, &instances);
    let mut detail: Option<Detail> = None;
    for (scale, region, rounds) in [
        (0.5, [0, 0, 20, 15], &["toneequal"][..]),
        (0.5, [12, 9, 20, 15], &[]),
        (0.6, [12, 9, 20, 15], &["toneequal"]),
        (0.6, [30, 22, 20, 15], &[]),
        (0.5, [22, 16, 20, 15], &["toneequal"]),
    ] {
        let rendered = scene.render_at(
            &gpu,
            &fitted,
            detail,
            &instances,
            region,
            scale,
            &Limits::NONE,
        );
        assert_eq!(rendered.rounds, rounds, "{region:?} at {scale}");
        let (width, expected) = fitted_at(&gpu, &scene, &instances, scale);
        assert_region(&gpu, &rendered, &expected, width, region);
        detail = Some(rendered);
    }
}

#[test]
fn a_new_scale_never_reuses_another_scales_pixels() {
    // 50% then 60% round the same centre, the region the view renders at each: the second
    // renders at 60% from its start, its frame the fitted viewport's at 60%.
    use apertrust_engine::view::{ANTICIPATE_MOVE, View, ViewScale, place, rendered};
    let gpu = gpu();
    let scene = bayer();
    let instances = toned(&scene, &[], 0.0);
    let fitted = scene.fitted(&gpu, &instances);
    let extent = fitted.plan().extent();
    let mut detail: Option<Detail> = None;
    for scale in [0.5, 0.6] {
        let view = View {
            scale: ViewScale::Factor(scale as f32),
            centre: [0.45, 0.55],
        };
        let placed = place(extent, (16, 12), &view);
        assert_eq!(placed.render, f64::from(scale as f32));
        let region = rendered(&placed, ANTICIPATE_MOVE, u64::MAX).expect("a region");
        let rendered = scene.render_at(
            &gpu,
            &fitted,
            detail,
            &instances,
            region,
            placed.render,
            &Limits::NONE,
        );
        assert_eq!(ran(&rendered.timings)[0], "main_region", "at {scale}");
        let desc = rendered.pipeline.output().desc();
        assert_eq!([desc.width, desc.height], [region[2], region[3]]);
        let (width, expected) = fitted_at(&gpu, &scene, &instances, placed.render);
        assert_region(&gpu, &rendered, &expected, width, region);
        detail = Some(rendered);
    }
}

/// The binding the tiled tests open the GPU with: the medium raws' demosaiced images,
/// about 2 MB, and their float mosaics, about 550 KB, pass it, so their fitted viewports
/// demosaic in tiles and their raw stages run in row strips, as lavapipe's do a large
/// photo's (decision 14).
const SMALL_BINDING: u64 = 512 << 10;

/// A Bayer raw whose fitted viewport tiles at [`SMALL_BINDING`]: 400 × 320.
fn medium_bayer() -> Raw {
    Raw::bayer((420, 340), [2, 3, 414, 334], [5, 6, 400, 320])
}

/// An X-Trans raw whose fitted viewport tiles at [`SMALL_BINDING`]: 400 × 320.
fn medium_x_trans() -> Raw {
    Raw::x_trans((422, 343), [1, 3, 416, 336], [4, 5, 400, 320])
}

/// [`regions`] but the whole image, which passes [`SMALL_BINDING`], and one inside it of
/// about half the binding's RGBA pixels.
fn tiled_regions(width: u32, height: u32) -> Vec<[u32; 4]> {
    let mut regions = regions(width, height);
    regions.pop();
    regions.push([100, 80, 160, 100]);
    regions
}

/// The fitted viewport of `instances` on `small`, in tiles, its raw stage in row strips.
fn tiled_fitted(small: &GpuContext, scene: &Raw, instances: &[Instance]) -> Pipeline {
    let fitted = scene.fitted_within(small, instances, &Limits::of(small), VIEWPORT);
    let plan = fitted.plan();
    let tiles = plan.tiled.clone().expect("a tiled viewport");
    let strips = |op: &PlannedOperation| op.dispatches.iter().any(|pass| pass.len() > 1);
    assert!(plan.operations[..tiles.start].iter().any(strips));
    fitted
}

/// `old`, or a fresh detail, of `region` on `small` over `fitted`, a tiled viewport, from
/// the raw stage's output it keeps (decision 99).
fn tiled_detail(
    small: &GpuContext,
    scene: &Raw,
    fitted: &Pipeline,
    old: Option<Detail>,
    instances: &[Instance],
    region: [u32; 4],
) -> Detail {
    let limits = Limits::of(small);
    let detail = scene.render(small, fitted, old, instances, region, &limits);
    assert_eq!(detail.pipeline.plan().input.format, Format::Mosaic);
    detail
}

#[test]
fn a_tiled_viewports_detail_equals_the_untiled_one() {
    // Upright and with a quarter turn, the region walked back through it to the mosaic.
    let (gpu, small) = (gpu(), common::small_gpu(SMALL_BINDING));
    for scene in [medium_bayer(), medium_x_trans()] {
        for settings in [&[][..], &[("flip", "orientation", 7.0)]] {
            let instances = scene.with(settings);
            let untiled = scene.fitted(&gpu, &instances);
            let tiled = tiled_fitted(&small, &scene, &instances);
            let [width, height] = untiled.plan().extent();
            for region in tiled_regions(width, height) {
                let expected = scene.detail(&gpu, &untiled, &instances, region);
                let expected = expected.pipeline.output().download(&gpu).unwrap();
                let detail = tiled_detail(&small, &scene, &tiled, None, &instances, region);
                let pixels = detail.pipeline.output().download(&small).unwrap();
                let difference = first_difference(&pixels, &expected);
                assert_eq!(difference, None, "{settings:?} {region:?}");
            }
        }
    }
}

#[test]
fn a_tiled_viewports_detail_below_100_equals_the_untiled_one() {
    // At 50%, upright and with a quarter turn: from the raw stage's output, the photosites
    // under the region's footprints copied, grown by demosaic's reach, demosaiced and
    // downscaled; where they pass the binding, as the whole image's do, in tiles, as the
    // fitted viewport's start.
    let (gpu, small) = (gpu(), common::small_gpu(SMALL_BINDING));
    for scene in [medium_bayer(), medium_x_trans()] {
        for settings in [&[][..], &[("flip", "orientation", 7.0)]] {
            let instances = scene.with(settings);
            let tiled = tiled_fitted(&small, &scene, &instances);
            let (width, expected) = fitted_at(&gpu, &scene, &instances, 0.5);
            let height = (expected.len() / width as usize) as u32;
            for (region, in_tiles) in [
                ([0, 0, 60, 40], false),
                ([width - 60, height - 40, 60, 40], false),
                ([width / 2 - 31, height / 2 - 23, 61, 47], false),
                ([0, height / 3, width, 9], false),
                ([width / 3, 0, 9, height], false),
                ([0, 0, width, height], true),
                ([width / 4, height / 5, width / 2 + 3, height / 2 + 5], true),
            ] {
                let limits = Limits::of(&small);
                let detail =
                    scene.render_at(&small, &tiled, None, &instances, region, 0.5, &limits);
                let plan = detail.pipeline.plan();
                assert_eq!(plan.input.format, Format::Mosaic);
                assert_eq!(plan.tiled.is_some(), in_tiles, "{settings:?} {region:?}");
                assert_region(&small, &detail, &expected, width, region);
            }
        }
    }
    // With the tone equalizer, its round at 50% in tiles from the raw stage's output too.
    let scene = medium_bayer();
    let instances = toned(&scene, &[], 0.0);
    let tiled = tiled_fitted(&small, &scene, &instances);
    let (width, expected) = fitted_at(&gpu, &scene, &instances, 0.5);
    let region = [20, 30, 120, 90];
    let limits = Limits::of(&small);
    let detail = scene.render_at(&small, &tiled, None, &instances, region, 0.5, &limits);
    assert_eq!(detail.rounds, ["toneequal"]);
    assert_region(&small, &detail, &expected, width, region);
}

#[test]
fn a_detail_whose_whole_image_scratch_cannot_bind_stops_its_tile_search() {
    // At 50% from a tiled viewport's mosaic, haze removal's scratch over the whole image
    // at the scale binds more at once than the GPU allows, which no tile of the detail's
    // start makes smaller: the start takes the largest tiles that bind themselves, 2 × 2
    // of 128 pixels, not the 4 × 3 of the smallest the search tries, and the detail's
    // results refuse the binding, as an export's are.
    let scene = medium_bayer();
    let instances = scene.adding(&[], &hazeremoval::HazeRemoval, haze_removal());
    let limits = Limits {
        binding: 256 << 10,
        ..Limits::NONE
    };
    let image = scene.image();
    let fitted = pipeline::plan(&instances, &image, scene.sensor(), VIEWPORT, &limits).unwrap();
    assert!(fitted.tiled.is_some());
    let source = fitted.operations[fitted.cached[0]].output_desc;
    assert_eq!(source.format, Format::Mosaic);
    let after = pipeline::detail_instances(&instances, &fitted);
    let pipe = Pipe::Detail {
        region: [0, 0, 100, 80],
        scale: 0.5,
    };
    let planned = pipeline::plan(&after, &image, source, pipe, &limits).unwrap();
    let tiled = planned.tiled.clone().expect("its start in tiles");
    let tiles = &planned.operations[tiled];
    let downscales = tiles.iter().filter(|op| op.id == "downscale").count();
    assert_eq!(downscales, 4);
    for op in tiles {
        assert!(
            op.output_desc.buffer_size() <= 256 << 10,
            "{}",
            op.output_desc
        );
    }
    let refused = Results::default().plan(planned, 0, &after, &image, &limits, None);
    match refused {
        Err(ExportError::HeldScratch {
            operation, limit, ..
        }) => assert_eq!((operation, limit), ("hazeremoval", 256 << 10)),
        other => panic!("refused, not {:?}", other.map(|(plan, _)| plan.pipe)),
    }
}

/// The whole-image operations the sweep below runs with: none, the tone equalizer, haze
/// removal.
#[derive(Debug, Clone, Copy)]
enum Whole {
    None,
    ToneEqualizer,
    HazeRemoval,
}

impl Whole {
    /// The default pipeline with `settings`, and this whole-image operation.
    fn instances(self, scene: &Raw, settings: &[(&str, &str, f32)]) -> Vec<Instance<'static>> {
        match self {
            Whole::None => scene.with(settings),
            Whole::ToneEqualizer => toned(scene, settings, 0.0),
            Whole::HazeRemoval => scene.adding(settings, &hazeremoval::HazeRemoval, haze_removal()),
        }
    }
}

/// Fill's scale of an odd ratio: the photo of `extent` in bounds of 50 × 23.
fn odd_fill(extent: [u32; 2]) -> f64 {
    use apertrust_engine::view::{View, ViewScale, place};
    let fill = View {
        scale: ViewScale::Fill,
        centre: [0.5, 0.5],
    };
    place(extent, (50, 23), &fill).render
}

#[test]
fn a_detail_below_100_equals_the_fitted_viewport_across_scales_turns_sources_and_rounds() {
    // Every case's frame is the same pixels of the fitted viewport planned at the whole
    // output's size at the scale, bit for bit: Bayer and X-Trans, upright and with the
    // quarter turn, whose width before and after the turn gives different scales; the
    // detail's start from demosaic's image (the small raws) or in tiles from a tiled
    // viewport's mosaic (the medium raws, with a region whose footprints pass the
    // binding); no whole-image operation, the tone equalizer's round, or haze removal's,
    // borrowed upright and at the detail's scale turned. A tiled viewport's haze removal
    // estimates at its own scale, which the fitted viewport at the scale does not, so it
    // is left out. The scales: non-half ones and Fill's of an odd ratio, each one whose
    // output size a fitted viewport has (at 0.83 the 84 × 62 raw's output, 70 × 51, is no
    // fitted viewport's: one 70 wide is 52 high).
    let (full, small) = (gpu(), common::small_gpu(SMALL_BINDING));
    let fill = f64::NAN;
    let cases: [(bool, Whole, &[f64]); 5] = [
        (false, Whole::None, &[0.37, 0.5, 0.61, 0.81, fill]),
        (false, Whole::ToneEqualizer, &[0.61, fill]),
        (false, Whole::HazeRemoval, &[0.61, 0.81, fill]),
        (true, Whole::None, &[0.61, 0.81]),
        (true, Whole::ToneEqualizer, &[0.61, 0.81]),
    ];
    for (tiled, whole, scales) in cases {
        let scenes = if tiled {
            [medium_bayer(), medium_x_trans()]
        } else {
            [bayer(), x_trans()]
        };
        for scene in &scenes {
            for settings in [&[][..], &[("flip", "orientation", 7.0)]] {
                let instances = whole.instances(scene, settings);
                let extent = scene.plan(&instances, Pipe::Export).unwrap().extent();
                let (gpu, fitted, limits) = if tiled {
                    (
                        &small,
                        tiled_fitted(&small, scene, &instances),
                        Limits::of(&small),
                    )
                } else {
                    (&full, scene.fitted(&full, &instances), Limits::NONE)
                };
                for &scale in scales {
                    let scale = if scale.is_nan() {
                        odd_fill(extent)
                    } else {
                        scale
                    };
                    let (width, expected) = fitted_at(&full, scene, &instances, scale);
                    let height = (expected.len() / width as usize) as u32;
                    let (w, h) = (width, height);
                    let regions = if tiled {
                        [[w / 8, h / 8, w * 9 / 16, h * 9 / 16], [0, 0, w / 3, h / 3]]
                    } else {
                        [
                            [w / 3, h / 3, w / 3 + 1, h / 3 + 1],
                            [0, h / 3, w, h / 4 + 1],
                        ]
                    };
                    for (k, region) in regions.into_iter().enumerate() {
                        let at = format!("{whole:?} {settings:?} {region:?} at {scale}");
                        let detail =
                            scene.render_at(gpu, &fitted, None, &instances, region, scale, &limits);
                        // A tiled viewport's first region starts in tiles.
                        let in_tiles = detail.pipeline.plan().tiled.is_some();
                        assert_eq!(in_tiles, tiled && k == 0, "{at}");
                        let pixels = detail.pipeline.output().download(gpu).unwrap();
                        let difference = first_difference(&pixels, &cut(&expected, width, region));
                        assert_eq!(difference, None, "{at}");
                    }
                }
            }
        }
    }
}

/// The raw stage's passes on the medium raws.
const RAW_STAGE: [&str; 4] = [
    "rawprepare",
    "temperature",
    "opposed_mask",
    "opposed_inpaint",
];

#[test]
fn a_tiled_viewports_pan_demosaics_only_the_new_region() {
    // Regions of one size on the 64-pixel grid, as the margin moves them: each pan copies
    // the new region's photosites, grown by demosaic's reach, and demosaics them alone,
    // in place, running none of the raw stage.
    let (gpu, small) = (gpu(), common::small_gpu(SMALL_BINDING));
    let scene = medium_bayer();
    let instances = scene.with(&[]);
    let fitted = tiled_fitted(&small, &scene, &instances);
    let (width, exported) = scene.whole(&gpu, &instances);
    let [w, h] = [128, 96];
    let mut detail = tiled_detail(&small, &scene, &fitted, None, &instances, [64, 64, w, h]);
    for [x, y] in [[192, 128], [0, 192], [256, 0]] {
        let region = [x, y, w, h];
        let limits = Limits::of(&small);
        let fresh = &mut Results::default();
        let (plan, _, _) =
            (scene.detail_plan(fresh, &fitted, &instances, region, &limits)).unwrap();
        assert!(detail.pipeline.accepts(&plan), "{region:?}: in place");
        detail = tiled_detail(&small, &scene, &fitted, Some(detail), &instances, region);
        let ran = ran(&detail.timings);
        assert_eq!(ran[0], "region", "{region:?}");
        assert!(ran.iter().all(|pass| !RAW_STAGE.contains(pass)), "{ran:?}");
        let plan = detail.pipeline.plan();
        let copy = plan.operations[0].output_desc;
        assert!(
            copy.width <= w + 2 * 12 + 1 && copy.height <= h + 2 * 12 + 1,
            "{copy}"
        );
        let at = (plan.operations.iter())
            .position(|op| op.id == "demosaic")
            .expect("demosaic");
        let demosaiced = plan.operations[at].output_desc;
        assert_eq!([demosaiced.width, demosaiced.height], [w, h]);
        assert_eq!(plan.cached, [0, at]);
        assert_region(&small, &detail, &exported, width, region);
    }
}

#[test]
fn a_tiled_viewports_white_balance_change_runs_the_raw_stage_once() {
    // The fitted viewport runs the raw stage again, once; the detail starts at its copy
    // of the new mosaic, which its plan's keys chain from.
    let (gpu, small) = (gpu(), common::small_gpu(SMALL_BINDING));
    let scene = medium_bayer();
    let region = [64, 64, 128, 96];
    let fitted = tiled_fitted(&small, &scene, &scene.with(&[]));
    let detail = tiled_detail(&small, &scene, &fitted, None, &scene.with(&[]), region);
    let red = scene.with(&[("temperature", "red", 2.2)]);
    let limits = Limits::of(&small);
    let plan = pipeline::plan(&red, &scene.image(), scene.sensor(), VIEWPORT, &limits).unwrap();
    let mut fitted = pollster::block_on(fitted.update(&small, plan)).unwrap();
    let mut passes = ran(&fitted.render(&small).unwrap());
    let detail = tiled_detail(&small, &scene, &fitted, Some(detail), &red, region);
    let detail_passes = ran(&detail.timings);
    assert_eq!(detail_passes[0], "region");
    passes.extend(detail_passes);
    for raw_stage in RAW_STAGE {
        let times = passes.iter().filter(|&&name| name == raw_stage).count();
        assert_eq!(times, 1, "{raw_stage} in {passes:?}");
    }
    let (width, exported) = scene.whole(&gpu, &red);
    assert_region(&small, &detail, &exported, width, region);
}

#[test]
fn a_tiled_viewports_detail_with_the_tone_equalizer_equals_the_exports_region() {
    // Its mask in a round over the whole image from the raw stage's output, in tiles of
    // the binding, each demosaicing its part, as an export's first round does.
    let (gpu, small) = (gpu(), common::small_gpu(SMALL_BINDING));
    let limits = Limits::of(&small);
    for scene in [medium_bayer(), medium_x_trans()] {
        let params = tone_equalizer(toneequal::Filter::Eigf);
        let instances = scene.adding(&[], &toneequal::ToneEqual, params);
        let fitted = tiled_fitted(&small, &scene, &instances);
        let (width, exported) = scene.whole(&gpu, &instances);
        let height = (exported.len() / width as usize) as u32;
        let region = [100, 80, 160, 100];
        let planned = scene.detail_plan(
            &mut Results::default(),
            &fitted,
            &instances,
            region,
            &limits,
        );
        let (_, rounds, _) = planned.unwrap();
        assert!(
            rounds.tiles().iter().all(|&tiles| tiles > 1),
            "{:?}",
            rounds.tiles()
        );
        for region in tiled_regions(width, height) {
            let detail = tiled_detail(&small, &scene, &fitted, None, &instances, region);
            assert_eq!(detail.rounds, ["toneequal"], "{region:?}");
            assert_region(&small, &detail, &exported, width, region);
        }
    }
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

#[test]
fn a_round_of_one_tile_that_cannot_bind_refuses_the_view() {
    // Local contrast takes one tile of the whole stage, which no tile size makes smaller:
    // at 50%, a tile whose downscale binds demosaic's whole image past the GPU's binding
    // is refused at planning, as rounds whose tiles never fit are, and the view stays
    // at Fit.
    let gpu = gpu();
    let scene = bayer();
    let instances = scene.adding(
        &[],
        &bilat::Bilat,
        bilat::values(&bilat::Settings::default()),
    );
    let fitted = scene.fitted(&gpu, &instances);
    let region = [10, 10, 20, 15];
    let at = |kib: u64| {
        let limits = Limits {
            binding: kib << 10,
            ..Limits::NONE
        };
        let fresh = &mut Results::default();
        scene.detail_plan_at(fresh, &fitted, &instances, region, 0.5, &limits)
    };
    assert!(at(512).is_ok_and(|(_, rounds, _)| rounds.tiles() == [1]));
    match at(80) {
        Err(ExportError::NoTiles { binding, .. }) => assert_eq!(binding, 80 << 10),
        other => panic!("refused, not {:?}", other.map(|(plan, _, _)| plan.pipe)),
    }
}
