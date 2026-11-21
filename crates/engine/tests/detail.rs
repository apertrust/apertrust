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
//! pixels of the untiled export bit for bit, with every operation that has whole-image
//! passes too, their results computed over the whole image in rounds of their own, held
//! by keys with no region in them, haze removal's borrowed from the fitted pipeline; a
//! change restarts it at its edit cache, and the plans of every other pipe keep their
//! bytes and keys.

mod common;

use apertrust_engine::compute::{GpuImage, Timings};
use apertrust_engine::detail::Results;
use apertrust_engine::export::{ExportError, Tiling};
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe, Pipeline, Plan, PlanError};
use apertrust_operations::{bilat, bloom, colorequal, hazeremoval, soften, toneequal};
use apertrust_plugin::{ImageDesc, Operation, ParamValues};
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

/// A detail at 100%: its whole-image results, its own pipeline, rendered after the rounds
/// its frame ran, those rounds' operations, and the timings of its own passes.
struct Detail {
    results: Results,
    pipeline: Pipeline,
    rounds: Vec<&'static str>,
    timings: Timings,
}

/// A raw as these tests develop it: through the fitted viewport, its detail at 100% over
/// the viewport's demosaic image, and the untiled export.
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
    /// The fitted viewport pipeline of `instances`, rendered.
    fn fitted(&self, gpu: &GpuContext, instances: &[Instance]) -> Pipeline;
    /// The detail of `region` at 100% over `fitted`'s demosaic image within `limits`,
    /// as `results` plans it: its plan, keyed on that image, its rounds, and that image.
    fn detail_plan(
        &self,
        results: &mut Results,
        fitted: &Pipeline,
        instances: &[Instance],
        region: [u32; 4],
        limits: &Limits,
    ) -> Result<(Plan, apertrust_engine::detail::Rounds, GpuImage), ExportError>;
    /// `old`, or a fresh detail where it is `None`, rendered for `instances` at `region`
    /// over `fitted`'s demosaic image within `limits`: its results planned, its pipeline
    /// updated or built, its rounds run, then its own passes.
    fn render(
        &self,
        gpu: &GpuContext,
        fitted: &Pipeline,
        old: Option<Detail>,
        instances: &[Instance],
        region: [u32; 4],
        limits: &Limits,
    ) -> Detail;
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

    fn fitted(&self, gpu: &GpuContext, instances: &[Instance]) -> Pipeline {
        let plan = self.plan(instances, VIEWPORT).unwrap();
        assert!(plan.tiled.is_none());
        let mut fitted = pollster::block_on(Pipeline::new(gpu, plan, self.upload(gpu))).unwrap();
        fitted.render(gpu).unwrap();
        fitted
    }

    fn detail_plan(
        &self,
        results: &mut Results,
        fitted: &Pipeline,
        instances: &[Instance],
        region: [u32; 4],
        limits: &Limits,
    ) -> Result<(Plan, apertrust_engine::detail::Rounds, GpuImage), ExportError> {
        let (source, key) = fitted.demosaiced().expect("demosaic's image, held");
        let pipe = Pipe::Detail { region, scale: 1.0 };
        let image = self.image();
        let planned = pipeline::plan(instances, &image, source.desc(), pipe, limits)?;
        let (plan, rounds) = results.plan(planned, key, instances, &image, limits, Some(fitted))?;
        Ok((plan, rounds, source))
    }

    fn render(
        &self,
        gpu: &GpuContext,
        fitted: &Pipeline,
        old: Option<Detail>,
        instances: &[Instance],
        region: [u32; 4],
        limits: &Limits,
    ) -> Detail {
        let (mut results, old) = match old {
            Some(old) => (old.results, Some(old.pipeline)),
            None => (Results::default(), None),
        };
        let (plan, rounds, source) = self
            .detail_plan(&mut results, fitted, instances, region, limits)
            .unwrap();
        let built = results.pipeline(gpu, plan, source.clone(), old);
        let mut pipeline = pollster::block_on(built).unwrap();
        let ran = rounds.operations();
        pollster::block_on(results.run(gpu, rounds, &source)).unwrap();
        let timings = pipeline.render(gpu).unwrap();
        Detail {
            results,
            pipeline,
            rounds: ran,
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
    // Only at 100% so far.
    let refused = at([10, 10, 40, 30], 0.5);
    let refused = matches!(refused, Err(PlanError::Detail { .. }));
    assert!(refused, "a detail at 50%");
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
