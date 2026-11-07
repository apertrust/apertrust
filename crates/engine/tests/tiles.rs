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

//! A tiled export on a GPU adapter, hardware or software, equals the untiled one bit
//! for bit (`docs/engine.md`, Testing): through every operation, Bayer and X-Trans, with
//! tiles that do not divide the image and a declared crop at an odd offset. So does an
//! export on the same adapter opened to bind less than the image at once, whose raw
//! stage runs in row strips and whose tiles copy from windows of its output (decision
//! 14). Every part of a tiled export sees the whole export's output profile. So does an
//! export at a size, resampled after demosaic, after haze removal or at finalscale's
//! place, its tiles walked back through the resample's plans (decisions 15 and 56).

use std::sync::Mutex;

use apertrust_engine::compute::GpuImage;
use apertrust_engine::edit::{self, Edit};
use apertrust_engine::export::{BUDGET, ExportError, Exported, Tiling, export, export_at};
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe, Resample};
use apertrust_modules::MODULES;
use apertrust_operations::OPERATIONS;
use apertrust_plugin::{
    Camera, Format, Geometry, ImageContext, ImageDesc, Kernel, Manifest, NotPorted, Operation,
    OutputProfile, ParamValues, Pattern, Prepared, Schema, Value,
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
/// image `declared`: a colour gradient with a highlight that clips, and `spots` that
/// clip too, each a centre and a radius in photosites on the sensor.
struct Raw {
    cfa: Pattern<u32>,
    sensor: (u32, u32),
    crop: [u32; 4],
    declared: [u32; 4],
    spots: Vec<(f32, f32, f32)>,
}

impl Raw {
    fn values(&self) -> Vec<u16> {
        let (width, height) = self.sensor;
        let cfa = &self.cfa;
        (0..width * height)
            .map(|i| {
                let (x, y) = (i % width, i / width);
                let c = cfa.values[((y % cfa.height) * cfa.width + x % cfa.width) as usize];
                let (fx, fy) = (x as f32 / width as f32, y as f32 / height as f32);
                let r2 = (fx - 0.6).powi(2) + (fy - 0.4).powi(2);
                let mut level =
                    0.15 + 0.5 * fx + 0.1 * c as f32 + 0.2 * fy + 1.2 * (-r2 / 0.01).exp();
                for &(cx, cy, radius) in &self.spots {
                    let d2 = (x as f32 - cx).powi(2) + (y as f32 - cy).powi(2);
                    level += (1.0 + 0.1 * c as f32) * (-d2 / (radius * radius)).exp();
                }
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

    /// The default pipeline an unedited photo runs.
    fn instances(&self) -> Vec<Instance<'static>> {
        edit::instances(&Edit::default(), OPERATIONS, MODULES, &self.image())
            .all()
            .unwrap()
    }

    fn export(&self, gpu: &GpuContext, tiling: Tiling) -> Exported {
        self.export_through(gpu, tiling, &self.instances())
    }

    fn export_through(&self, gpu: &GpuContext, tiling: Tiling, instances: &[Instance]) -> Exported {
        let (width, height) = self.sensor;
        let input = GpuImage::upload_mosaic(gpu, width, height, &self.values()).unwrap();
        pollster::block_on(export(gpu, instances, &self.image(), input, tiling)).unwrap()
    }

    /// Its export at `resample`'s size.
    fn export_at(
        &self,
        gpu: &GpuContext,
        tiling: Tiling,
        instances: &[Instance],
        resample: Resample,
    ) -> Result<Exported, apertrust_engine::export::ExportError> {
        let (width, height) = self.sensor;
        let input = GpuImage::upload_mosaic(gpu, width, height, &self.values()).unwrap();
        pollster::block_on(export_at(
            gpu,
            instances,
            &self.image(),
            input,
            tiling,
            Some(resample),
        ))
    }

    /// Its export at `scale`, darktable's output size, resampled at finalscale's place or
    /// after demosaic.
    fn at_scale(&self, scale: f64, finalscale: bool) -> Resample {
        let [.., width, height] = self.declared;
        Resample {
            output: [width, height].map(|side| (scale * f64::from(side)).floor() as u32),
            scale: scale as f32,
            finalscale,
        }
    }
}

fn tiled_equals_untiled(raw: Raw) {
    tiled_through_equals_untiled(&raw, &raw.instances(), 1);
}

/// `raw` exported through `instances` whole and in tiles of several sizes, in `rounds`
/// rounds when tiled.
fn tiled_through_equals_untiled(raw: &Raw, instances: &[Instance], rounds: usize) {
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default()))
        .expect("a GPU adapter, hardware or software");
    let whole = raw.export_through(&gpu, Tiling::Whole, instances);
    assert_eq!((whole.tiles, whole.rounds), (1, 1));
    let whole = whole.pixels;
    let [.., width, height] = raw.declared;
    assert_eq!(whole.len(), (width * height) as usize);
    for size in [[24, 20], [37, 29], [64, 1000], [1000, 1000]] {
        let exported = raw.export_through(&gpu, Tiling::Tiles(size), instances);
        let (tiles, tiled) = (exported.tiles, exported.pixels);
        let expected = (width.div_ceil(size[0]) * height.div_ceil(size[1])) as usize;
        assert_eq!(tiles, expected, "{size:?}");
        assert_eq!(exported.rounds, rounds, "{size:?}");
        if let Some(i) = tiled.iter().zip(&whole).position(|(a, b)| a != b) {
            panic!(
                "tiles of {size:?}: pixel ({}, {}) is {:?} tiled, {:?} whole",
                i as u32 % width,
                i as u32 / width,
                tiled[i],
                whole[i]
            );
        }
    }
}

#[test]
fn a_tiled_bayer_export_equals_the_whole_one() {
    tiled_equals_untiled(Raw {
        cfa: Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 1, 2],
        },
        sensor: (110, 86),
        crop: [2, 2, 106, 82],
        declared: [5, 4, 98, 76],
        spots: Vec::new(),
    });
}

/// The tone equalizer at test 0131's bands on EIGF iterated three times with
/// quantisation: its mask in a first round of tiles (decision 15), radius 5 on the
/// 110-photosite readout, so a grid of 24 x 19 at a downscaling of 4.
fn tone_equalizer() -> ParamValues {
    use apertrust_operations::toneequal::{Filter, ToneEqual, settings, values};
    let mut tone = settings(&ToneEqual.schema().defaults());
    tone.bands = [
        -1.283, -0.788, 0.445, 1.148, 1.321, 0.994, 0.211, -0.449, -0.755,
    ];
    (tone.details, tone.iterations) = (Filter::Eigf, 3);
    (tone.blending, tone.quantization, tone.exposure_boost) = (10.0, 0.5, 1.0);
    values(&tone)
}

#[test]
fn a_tiled_export_with_the_tone_equalizers_mask_equals_the_whole_one() {
    // The default pipeline and the tone equalizer, its mask in a first round of tiles.
    use apertrust_operations::toneequal::ToneEqual;
    let raw = Raw {
        cfa: Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 1, 2],
        },
        sensor: (110, 86),
        crop: [2, 2, 106, 82],
        declared: [5, 4, 98, 76],
        spots: vec![(60.0, 30.0, 3.0)],
    };
    let mut instances = raw.instances();
    instances.push(Instance {
        operation: &ToneEqual,
        params: tone_equalizer(),
    });
    tiled_through_equals_untiled(&raw, &instances, 2);
}

/// Runs as `inner` does, and keeps the output profile each of its `prepare` calls saw.
struct Witness {
    inner: &'static dyn Operation,
    seen: Mutex<Vec<Option<OutputProfile>>>,
}

impl Witness {
    fn new(inner: &'static dyn Operation) -> Self {
        Self {
            inner,
            seen: Mutex::new(Vec::new()),
        }
    }

    /// What it saw since the last call.
    fn seen(&self) -> Vec<Option<OutputProfile>> {
        std::mem::take(&mut *self.seen.lock().unwrap())
    }
}

impl Operation for Witness {
    fn manifest(&self) -> &'static Manifest {
        self.inner.manifest()
    }
    fn schema(&self) -> &'static Schema {
        self.inner.schema()
    }
    fn kernel(&self) -> &'static Kernel {
        self.inner.kernel()
    }
    fn defaults(&self, image: &ImageContext) -> ParamValues {
        self.inner.defaults(image)
    }
    fn prepare(
        &self,
        params: &ParamValues,
        image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<Prepared, NotPorted> {
        self.seen.lock().unwrap().push(image.output_profile);
        self.inner.prepare(params, image, input)
    }
    fn output_profile(&self, params: &ParamValues) -> Option<OutputProfile> {
        self.inner.output_profile(params)
    }
    fn tiles_whole_image_passes(&self, params: &ParamValues) -> bool {
        self.inner.tiles_whole_image_passes(params)
    }
    fn geometry(&self, params: &ParamValues, image: &ImageContext) -> Option<Box<dyn Geometry>> {
        self.inner.geometry(params, image)
    }
}

#[test]
fn every_part_of_a_tiled_export_sees_the_whole_exports_output_profile() {
    // Temperature runs before the tiles, which start after highlight reconstruction's
    // whole-image passes, and the tone equalizer in a first round of tiles: neither part
    // has colorout among its instances, which reports the profile (decision 26).
    use apertrust_operations::temperature::Temperature;
    use apertrust_operations::toneequal::ToneEqual;
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default()))
        .expect("a GPU adapter, hardware or software");
    let raw = Raw {
        cfa: Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 1, 2],
        },
        sensor: (110, 86),
        crop: [2, 2, 106, 82],
        declared: [5, 4, 98, 76],
        spots: vec![(60.0, 30.0, 3.0)],
    };
    let (temperature, tone) = (Witness::new(&Temperature), Witness::new(&ToneEqual));
    let mut instances: Vec<Instance> = raw.instances();
    for instance in &mut instances {
        if instance.operation.manifest().id == "temperature" {
            instance.operation = &temperature;
        }
    }
    instances.push(Instance {
        operation: &tone,
        params: tone_equalizer(),
    });

    raw.export_through(&gpu, Tiling::Whole, &instances);
    let untiled = temperature.seen();
    assert!(matches!(untiled[..], [Some(_)]), "{untiled:?}");
    assert_eq!(tone.seen(), untiled);

    let exported = raw.export_through(&gpu, Tiling::Tiles([37, 29]), &instances);
    assert_eq!((exported.tiles, exported.rounds), (9, 2));
    for (witness, parts) in [(&temperature, 2), (&tone, 1 + 2 * 9)] {
        let seen = witness.seen();
        let id = witness.manifest().id;
        assert_eq!(seen.len(), parts, "{id}");
        assert!(
            seen.iter().all(|profile| *profile == untiled[0]),
            "{id}: {seen:?}"
        );
    }
}

#[test]
fn a_tiled_export_with_local_contrast_equals_the_whole_one() {
    // The default pipeline and local contrast, in each mode: its pyramid or grid in
    // whole-image passes it does not tile, so everything up to it runs whole and only
    // what follows runs in tiles, in one round (docs/ports/bilat.md, Neighbourhood and
    // scale).
    use apertrust_operations::bilat::{Bilat, Mode, Settings, values};
    let raw = Raw {
        cfa: Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 1, 2],
        },
        sensor: (110, 86),
        crop: [2, 2, 106, 82],
        declared: [5, 4, 98, 76],
        spots: vec![(60.0, 30.0, 3.0)],
    };
    let bilateral_grid = Settings {
        mode: Mode::BilateralGrid,
        sigma_s: 50.0,
        sigma_r: 20.0,
        detail: 1.0,
        ..Settings::default()
    };
    for settings in [Settings::default(), bilateral_grid] {
        let mut instances = raw.instances();
        instances.push(Instance {
            operation: &Bilat,
            params: values(&settings),
        });
        tiled_through_equals_untiled(&raw, &instances, 1);
    }
}

#[test]
fn a_tiled_export_through_haze_removal_equals_the_whole_one() {
    // The default pipeline and haze removal at test 0166's settings, and at a negative
    // strength, which orders the transmission's filters the other way. Its whole-image
    // passes are one run (decision 15): a first round of tiles copies each tile's input
    // into the whole image's scratch, its last tile estimates the ambient light, the
    // depth and the transmission from there once, and the second round runs `main`, so
    // nothing before haze removal runs whole but the raw stage: two rounds.
    use apertrust_operations::hazeremoval::{HazeRemoval, Settings, values};
    let raw = Raw {
        cfa: Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 1, 2],
        },
        sensor: (110, 86),
        crop: [2, 2, 106, 82],
        declared: [5, 4, 98, 76],
        spots: vec![(60.0, 30.0, 3.0), (20.0, 60.0, 5.0)],
    };
    for (strength, distance) in [(0.9, 0.8), (-0.5, 0.3)] {
        let mut instances = raw.instances();
        instances.push(Instance {
            operation: &HazeRemoval,
            params: values(&Settings { strength, distance }),
        });
        tiled_through_equals_untiled(&raw, &instances, 2);
    }
}

#[test]
fn an_exports_budget_leaves_out_the_scratch_it_holds() {
    // Haze removal's whole-image scratch, 40 bytes a pixel of the whole image (decision
    // 15), is held by the export across its rounds and no tile size changes it, so the
    // export is sized without it (decision 15). At a budget below that scratch alone,
    // which no tile's plan fits with it, the export still tiles, equal to the whole one;
    // at a budget the whole image's other buffers fit, it runs whole, in one round, as
    // tiles would save nothing of the scratch.
    use apertrust_operations::hazeremoval::{HazeRemoval, Settings, values};
    let raw = Raw {
        cfa: Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 1, 2],
        },
        sensor: (110, 86),
        crop: [2, 2, 106, 82],
        declared: [5, 4, 98, 76],
        spots: vec![(60.0, 30.0, 3.0)],
    };
    let mut instances = raw.instances();
    instances.push(Instance {
        operation: &HazeRemoval,
        params: values(&Settings {
            strength: 0.9,
            distance: 0.8,
        }),
    });
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default())).unwrap();
    let (width, height) = raw.sensor;
    let sensor = GpuImage::upload_mosaic(&gpu, width, height, &raw.values()).unwrap();
    let plan = pipeline::plan(
        &instances,
        &raw.image(),
        sensor.desc(),
        Pipe::Export,
        &Limits::of(&gpu),
    )
    .unwrap();
    let haze = plan
        .operations
        .iter()
        .find(|op| op.id == "hazeremoval")
        .unwrap();
    let held: u64 = haze.scratch.iter().map(|&i| plan.scratch[i]).sum();
    let [.., declared_width, declared_height] = raw.declared;
    let pixels = u64::from(declared_width * declared_height);
    // The selections' 2048 bins, 16 words of state, and the ambient sums' partial
    // results, 16 bytes for each 64 pixels square: 2 across and 2 down.
    let fixed = 2048 * 4 + 16 * 4 + 4 * 16;
    assert_eq!(held, 40 * pixels + fixed);
    let whole = raw.export_through(&gpu, Tiling::Whole, &instances).pixels;
    let exported = raw.export_through(&gpu, Tiling::Auto { budget: held - 1 }, &instances);
    assert!(exported.tiles > 1, "{} tiles", exported.tiles);
    assert_eq!(exported.rounds, 2);
    assert!(exported.pixels == whole, "the tiled export differs");
    // Its own buffers fit, though not beside the scratch: whole, in one round.
    let budget = plan.bytes() - held;
    let exported = raw.export_through(&gpu, Tiling::Auto { budget }, &instances);
    assert_eq!((exported.tiles, exported.rounds), (1, 1));
    assert!(exported.pixels == whole, "the export differs");
}

#[test]
fn a_tiled_export_with_bloom_equals_the_whole_one() {
    // The default pipeline and bloom: its lights in a first round of tiles, each writing
    // its own pixels into the whole image's scratch, the box means once over it in the
    // round's last tile, and `main` in the second round (decision 15;
    // docs/ports/bloom.md, Neighbourhood and scale). darktable's defaults, a radius of
    // 53 past the image's half width, and a radius of 2 with a low threshold, so the box
    // means' bulk runs too.
    use apertrust_operations::bloom::{Bloom, Settings, values};
    let raw = Raw {
        cfa: Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 1, 2],
        },
        sensor: (110, 86),
        crop: [2, 2, 106, 82],
        declared: [5, 4, 98, 76],
        spots: vec![(60.0, 30.0, 3.0)],
    };
    let small = Settings {
        size: 0.0,
        threshold: 40.0,
        strength: 60.0,
    };
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default())).unwrap();
    let without = raw.export(&gpu, Tiling::Whole).pixels;
    for settings in [Settings::default(), small] {
        let mut instances = raw.instances();
        instances.push(Instance {
            operation: &Bloom,
            params: values(&settings),
        });
        // Each blooms, not just the Lab round trip's rounding, about 1e-6: the screen blend
        // brightens some pixel's RGB by 0.93 at darktable's defaults on the M4, and by
        // 1.03 at the radius of 2.
        let with = raw.export_through(&gpu, Tiling::Whole, &instances).pixels;
        let brightening = with
            .iter()
            .zip(&without)
            .map(|(a, b)| (0..3).map(|c| a[c] - b[c]).sum::<f32>())
            .fold(f32::MIN, f32::max);
        assert!(brightening > 0.1, "{settings:?} brightens by {brightening}");
        tiled_through_equals_untiled(&raw, &instances, 2);
    }
}

#[test]
fn a_tiled_export_with_the_colour_equalizer_equals_the_whole_one() {
    // The default pipeline and the colour equalizer with its guided filter: `sample`
    // in a first round of tiles, each writing its own pixels' chromaticity, saturation
    // and lightness into the whole image's scratch, the filters' passes once over it,
    // and `main` in the second round (decision 15; docs/ports/colorequal.md,
    // Neighbourhood and scale). darktable's defaults, both filters blurring in place, and
    // an effect radius of 25 with curves, the guide on a grid a fourth of the image.
    use apertrust_operations::colorequal::{ColorEqual, Settings, values};
    let raw = Raw {
        cfa: Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 1, 2],
        },
        sensor: (110, 86),
        crop: [2, 2, 106, 82],
        declared: [5, 4, 98, 76],
        spots: vec![(60.0, 30.0, 3.0)],
    };
    let curved = Settings {
        param_size: 25.0,
        chroma_size: 2.0,
        saturation: [1.3, 1.0, 0.7, 1.0, 1.3, 1.0, 0.7, 1.0],
        hue: [40.0, -20.0, 0.0, 0.0, 40.0, 0.0, -45.0, 0.0],
        brightness: [1.0, 1.2, 1.0, 0.8, 1.0, 1.3, 1.0, 0.7],
        ..Settings::default()
    };
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default())).unwrap();
    let without = raw.export(&gpu, Tiling::Whole).pixels;
    for settings in [Settings::default(), curved] {
        let mut instances = raw.instances();
        instances.push(Instance {
            operation: &ColorEqual,
            params: values(&settings),
        });
        let with = raw.export_through(&gpu, Tiling::Whole, &instances).pixels;
        let change = with
            .iter()
            .zip(&without)
            .map(|(a, b)| (0..3).map(|c| (a[c] - b[c]).abs()).sum::<f32>())
            .fold(0.0, f32::max);
        assert!(change > 1e-3, "{settings:?} changes by {change}");
        tiled_through_equals_untiled(&raw, &instances, 2);
    }
}

#[test]
fn a_tiled_export_with_soften_equals_the_whole_one() {
    // The default pipeline and soften: its softened image in a first round of tiles, each
    // writing its own pixels into the whole image's scratch, the box means once over it
    // in the round's last tile, and `main` in the second round (decision 15;
    // docs/ports/soften.md, Neighbourhood and scale). The readout's diagonal, 200
    // pixels, makes darktable's largest radius 2: darktable's defaults blur by 1, size
    // 100 by 2, so the box means reach 8 and 16 pixels, past the tiles' edges.
    use apertrust_operations::soften::{Settings, Soften, values};
    let raw = Raw {
        cfa: Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 1, 2],
        },
        sensor: (160, 120),
        crop: [2, 2, 156, 116],
        declared: [5, 4, 148, 110],
        spots: vec![(90.0, 40.0, 3.0)],
    };
    let largest = Settings {
        size: 100.0,
        saturation: 40.0,
        brightness: 1.0,
        amount: 80.0,
    };
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default())).unwrap();
    let without = raw.export(&gpu, Tiling::Whole).pixels;
    for settings in [Settings::default(), largest] {
        let mut instances = raw.instances();
        instances.push(Instance {
            operation: &Soften,
            params: values(&settings),
        });
        let with = raw.export_through(&gpu, Tiling::Whole, &instances).pixels;
        let change = with
            .iter()
            .zip(&without)
            .map(|(a, b)| (0..3).map(|c| (a[c] - b[c]).abs()).sum::<f32>())
            .fold(0.0, f32::max);
        assert!(change > 0.1, "{settings:?} changes by {change}");
        tiled_through_equals_untiled(&raw, &instances, 2);
    }
}

#[test]
fn a_tiled_export_with_filmic_rgb_equals_the_whole_one() {
    // The default pipeline with filmic rgb as its tone mapper in AgX's place, highlight
    // reconstruction on at -3 EV, where the raw's highlight and spot count as clipped:
    // its wavelets in whole-image passes it does not tile, so everything up to it runs
    // whole and only what follows runs in tiles, in one round
    // (docs/ports/filmicrgb.md, Neighbourhood and scale).
    use apertrust_operations::filmicrgb::{FilmicRgb, Settings, values};
    let raw = Raw {
        cfa: Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 1, 2],
        },
        sensor: (110, 86),
        crop: [2, 2, 106, 82],
        declared: [5, 4, 98, 76],
        spots: vec![(60.0, 30.0, 3.0)],
    };
    let settings = Settings {
        enable_highlight_reconstruction: true,
        reconstruct_threshold: -3.0,
        ..Settings::default()
    };
    let mut instances: Vec<Instance> = raw
        .instances()
        .into_iter()
        .filter(|instance| instance.operation.manifest().id != "agx")
        .collect();
    instances.push(Instance {
        operation: &FilmicRgb,
        params: values(&settings),
    });
    tiled_through_equals_untiled(&raw, &instances, 1);
}

/// Vignetting off its defaults, whose kernel places each pixel from the pass header: after
/// a resample after demosaic, at the export's scale.
fn vignetting() -> Instance<'static> {
    let mut params = ParamValues::new();
    params.set("brightness", Value::Float(-0.5));
    params.set("saturation", Value::Float(-0.5));
    Instance {
        operation: &apertrust_operations::vignette::Vignette,
        params,
    }
}

/// `raw` exported at scales 0.5, 1/3 and 2, after demosaic and at finalscale's place,
/// whole and in tiles of the output image at its scale, with vignetting, which places
/// its pixels from the header's origin and scale (decision 56).
fn tiled_at_sizes_equals_untiled(raw: &Raw) {
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default()))
        .expect("a GPU adapter, hardware or software");
    let mut instances = raw.instances();
    instances.push(vignetting());
    for scale in [0.5, 1.0 / 3.0, 2.0] {
        for finalscale in [false, true] {
            let resample = raw.at_scale(scale, finalscale);
            let whole = raw
                .export_at(&gpu, Tiling::Whole, &instances, resample)
                .unwrap();
            let [width, height] = resample.output;
            assert_eq!(
                (whole.desc.width, whole.desc.height, whole.tiles),
                (width, height, 1)
            );
            assert!(whole.pixels.iter().flatten().all(|v| v.is_finite()));
            for size in [[24, 20], [37, 29], [1000, 1000]] {
                let exported = raw
                    .export_at(&gpu, Tiling::Tiles(size), &instances, resample)
                    .unwrap();
                let expected = (width.div_ceil(size[0]) * height.div_ceil(size[1])) as usize;
                let at = format!("scale {scale}, finalscale {finalscale}, tiles of {size:?}");
                assert_eq!(exported.tiles, expected, "{at}");
                if let Some(i) = exported
                    .pixels
                    .iter()
                    .zip(&whole.pixels)
                    .position(|(a, b)| a != b)
                {
                    panic!(
                        "{at}: pixel ({}, {}) is {:?} tiled, {:?} whole",
                        i as u32 % width,
                        i as u32 / width,
                        exported.pixels[i],
                        whole.pixels[i]
                    );
                }
            }
        }
    }
}

#[test]
fn an_export_a_pixel_short_at_a_scale_of_1_is_the_full_exports_corner() {
    // 0.99999999 is 1 in f32, so darktable's resample copies (`copymode`), but its output
    // is floor(0.99999999 × W) by floor(0.99999999 × H), a pixel short: the full-size
    // export's top-left part, whole and in tiles, after demosaic and at finalscale's
    // place, vignetting placing its pixels as at full size.
    let raw = Raw {
        cfa: Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 1, 2],
        },
        sensor: (110, 86),
        crop: [2, 2, 106, 82],
        declared: [5, 4, 98, 76],
        spots: vec![(60.0, 30.0, 3.0)],
    };
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default())).unwrap();
    let mut instances = raw.instances();
    instances.push(vignetting());
    let full = raw.export_through(&gpu, Tiling::Whole, &instances);
    let [.., width, height] = raw.declared;
    let corner: Vec<[f32; 4]> = (0..height - 1)
        .flat_map(|y| {
            let row = (y * width) as usize;
            full.pixels[row..row + width as usize - 1].to_vec()
        })
        .collect();
    for finalscale in [false, true] {
        let resample = raw.at_scale(0.99999999, finalscale);
        assert_eq!(
            (resample.output, resample.scale),
            ([width - 1, height - 1], 1.0)
        );
        for tiling in [Tiling::Whole, Tiling::Tiles([24, 20])] {
            let cut = raw.export_at(&gpu, tiling, &instances, resample).unwrap();
            assert!(cut.pixels == corner, "finalscale {finalscale}, {tiling:?}");
        }
    }
}

#[test]
fn a_tiled_bayer_export_at_a_size_equals_the_whole_one() {
    tiled_at_sizes_equals_untiled(&Raw {
        cfa: Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 1, 2],
        },
        sensor: (110, 86),
        crop: [2, 2, 106, 82],
        declared: [5, 4, 98, 76],
        spots: vec![(60.0, 30.0, 3.0)],
    });
}

#[test]
fn a_tiled_x_trans_export_at_a_size_equals_the_whole_one() {
    tiled_at_sizes_equals_untiled(&Raw {
        cfa: Pattern {
            width: 6,
            height: 6,
            values: X_TRANS.to_vec(),
        },
        sensor: (120, 96),
        crop: [1, 3, 116, 90],
        declared: [4, 5, 108, 84],
        spots: Vec::new(),
    });
}

/// The tone equalizer at test 0131's bands, and haze removal at test 0166's settings:
/// operations whose whole-image passes take a first round of an export's tiles.
fn with_whole_image_passes() -> [Instance<'static>; 2] {
    use apertrust_operations::hazeremoval::{self, HazeRemoval};
    use apertrust_operations::toneequal::ToneEqual;
    [
        Instance {
            operation: &ToneEqual,
            params: tone_equalizer(),
        },
        Instance {
            operation: &HazeRemoval,
            params: hazeremoval::values(&hazeremoval::Settings {
                strength: 0.9,
                distance: 0.8,
            }),
        },
    ]
}

#[test]
fn whole_image_passes_at_a_size_take_a_first_round_at_full_resolution() {
    // Before finalscale's place the tone equalizer's mask and haze removal's estimate run
    // at full resolution, so at a size, as at full size, they take a first round of
    // tiles, of the image at full resolution, before the last round's tiles of the output
    // walk back to it (decisions 15 and 56): nothing but the raw stage runs whole, and
    // the export equals the untiled one, down and up.
    let raw = Raw {
        cfa: Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 1, 2],
        },
        sensor: (110, 86),
        crop: [2, 2, 106, 82],
        declared: [5, 4, 98, 76],
        spots: vec![(60.0, 30.0, 3.0), (20.0, 60.0, 5.0)],
    };
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default())).unwrap();
    for extra in with_whole_image_passes() {
        let id = extra.operation.manifest().id;
        let mut instances = raw.instances();
        instances.push(extra);
        for scale in [0.5, 1.0 / 3.0, 2.0] {
            let resample = raw.at_scale(scale, true);
            let whole = raw
                .export_at(&gpu, Tiling::Whole, &instances, resample)
                .unwrap();
            let tiled = raw
                .export_at(&gpu, Tiling::Tiles([24, 20]), &instances, resample)
                .unwrap();
            let at = format!("{id} at {scale}");
            assert!(
                tiled.tiles > 1 && tiled.rounds == 2,
                "{at}: {} tiles, {} rounds",
                tiled.tiles,
                tiled.rounds
            );
            assert!(
                tiled.pixels == whole.pixels,
                "{at}: tiles equal the whole export"
            );
        }
    }
}

/// The operations with whole-image passes that come after a resample after demosaic,
/// each with the rounds a tiled export through it takes: the tone equalizer, bloom,
/// soften and the colour equalizer two, a first round of tiles at the export's scale,
/// and local contrast and filmic rgb's reconstruction one, run whole with everything
/// before them (decision 15).
fn after_the_resample() -> [(&'static dyn Operation, ParamValues, usize); 6] {
    use apertrust_operations::{bilat, bloom, colorequal, filmicrgb, soften};
    let filmic = filmicrgb::Settings {
        enable_highlight_reconstruction: true,
        reconstruct_threshold: -3.0,
        ..filmicrgb::Settings::default()
    };
    // At darktable's defaults the colour equalizer only filters, which so small an image
    // at 1/3 hardly shows: its curves, as its tiled test has them.
    let curved = colorequal::Settings {
        param_size: 25.0,
        chroma_size: 2.0,
        saturation: [1.3, 1.0, 0.7, 1.0, 1.3, 1.0, 0.7, 1.0],
        hue: [40.0, -20.0, 0.0, 0.0, 40.0, 0.0, -45.0, 0.0],
        brightness: [1.0, 1.2, 1.0, 0.8, 1.0, 1.3, 1.0, 0.7],
        ..colorequal::Settings::default()
    };
    [
        (
            &apertrust_operations::toneequal::ToneEqual,
            tone_equalizer(),
            2,
        ),
        (&bloom::Bloom, bloom::values(&bloom::Settings::default()), 2),
        (
            &soften::Soften,
            soften::values(&soften::Settings::default()),
            2,
        ),
        (&colorequal::ColorEqual, colorequal::values(&curved), 2),
        (&bilat::Bilat, bilat::values(&bilat::Settings::default()), 1),
        (&filmicrgb::FilmicRgb, filmicrgb::values(&filmic), 1),
    ]
}

/// `raw`'s default pipeline with `operation` added, filmic rgb as the tone mapper in
/// AgX's place.
fn through(
    raw: &Raw,
    operation: &'static dyn Operation,
    params: &ParamValues,
) -> Vec<Instance<'static>> {
    let id = operation.manifest().id;
    let mut instances: Vec<Instance> = raw
        .instances()
        .into_iter()
        .filter(|instance| id != "filmicrgb" || instance.operation.manifest().id != "agx")
        .collect();
    instances.push(Instance {
        operation,
        params: params.clone(),
    });
    instances
}

#[test]
fn whole_image_passes_after_a_resample_cover_its_output_in_an_untiled_export() {
    // Without high quality the resample comes after demosaic, and every whole-image pass
    // after it covers its output, the stage at the export's scale (decision 15): an untiled
    // export at 0.5 and 1/3 through the tone equalizer, bloom, soften, the colour
    // equalizer, local contrast and filmic rgb's reconstruction is the output's size,
    // finite, and changed by each.
    let raw = Raw {
        cfa: Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 1, 2],
        },
        sensor: (160, 120),
        crop: [2, 2, 156, 116],
        declared: [5, 4, 148, 110],
        spots: vec![(90.0, 40.0, 3.0), (30.0, 80.0, 5.0)],
    };
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default())).unwrap();
    for scale in [0.5, 1.0 / 3.0] {
        let resample = raw.at_scale(scale, false);
        let [width, height] = resample.output;
        let without = raw
            .export_at(&gpu, Tiling::Whole, &raw.instances(), resample)
            .unwrap()
            .pixels;
        for (operation, params, _) in &after_the_resample() {
            let id = operation.manifest().id;
            let at = format!("{id} at {scale}");
            let instances = through(&raw, *operation, params);
            let whole = raw
                .export_at(&gpu, Tiling::Whole, &instances, resample)
                .unwrap();
            assert_eq!(
                (whole.desc.width, whole.desc.height, whole.tiles),
                (width, height, 1),
                "{at}"
            );
            assert!(whole.pixels.iter().flatten().all(|v| v.is_finite()), "{at}");
            let change = whole
                .pixels
                .iter()
                .zip(&without)
                .map(|(a, b)| (0..3).map(|c| (a[c] - b[c]).abs()).sum::<f32>())
                .fold(0.0, f32::max);
            assert!(change > 1e-3, "{at} changes by {change}");
        }
    }
}

/// `raw` exported at 0.5 and 1/3 without high quality through each operation of
/// [`after_the_resample`], or only those that take a first round where `first_rounds`
/// says, whole on the adapter as it is and in tiles of the output at its scale as
/// `tilings` say, each on its GPU with what it is: every tiled export takes the rounds
/// its operation takes, more than one tile where `first_rounds` says, and equals the
/// whole one bit for bit.
fn tiled_after_the_resample_equals_untiled(
    raw: &Raw,
    tilings: &[(&str, &GpuContext, Tiling)],
    first_rounds: bool,
) {
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default()))
        .expect("a GPU adapter, hardware or software");
    for scale in [0.5, 1.0 / 3.0] {
        let resample = raw.at_scale(scale, false);
        let [width, height] = resample.output;
        for (operation, params, rounds) in &after_the_resample() {
            if first_rounds && *rounds == 1 {
                continue;
            }
            let id = operation.manifest().id;
            let instances = through(raw, *operation, params);
            let whole = raw
                .export_at(&gpu, Tiling::Whole, &instances, resample)
                .unwrap();
            assert_eq!((whole.desc.width, whole.desc.height), (width, height));
            for (what, gpu, tiling) in tilings {
                let at = format!("{id} at {scale}, {what}");
                let tiled = raw
                    .export_at(gpu, *tiling, &instances, resample)
                    .unwrap_or_else(|error| panic!("{at}: {error}"));
                assert_eq!(tiled.rounds, *rounds, "{at}");
                assert!(
                    !first_rounds || tiled.tiles > 1,
                    "{at}: {} tiles",
                    tiled.tiles
                );
                if let Some(i) = tiled
                    .pixels
                    .iter()
                    .zip(&whole.pixels)
                    .position(|(a, b)| a != b)
                {
                    panic!(
                        "{at}: pixel ({}, {}) is {:?} tiled, {:?} whole",
                        i as u32 % width,
                        i as u32 / width,
                        tiled.pixels[i],
                        whole.pixels[i]
                    );
                }
            }
        }
    }
}

/// [`tiled_after_the_resample_equals_untiled`] in tiles of two sizes that divide neither
/// side of the output, and in one tile.
fn tiles_after_the_resample(raw: &Raw) {
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default())).unwrap();
    let tilings: Vec<(&str, &GpuContext, Tiling)> = [[24, 20], [37, 29], [1000, 1000]]
        .into_iter()
        .map(|size| ("tiles", &gpu, Tiling::Tiles(size)))
        .collect();
    tiled_after_the_resample_equals_untiled(raw, &tilings, false);
}

#[test]
fn a_bayer_export_at_a_size_takes_its_first_rounds_at_the_exports_scale() {
    // Without high quality, the whole-image passes after the resample take their first
    // round in the output's own tiles at the export's scale, each grown a pixel and walked
    // back through the resample (decision 15), and local contrast and filmic rgb run whole
    // with everything before them: each tiled export equals the untiled one.
    tiles_after_the_resample(&Raw {
        cfa: Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 1, 2],
        },
        sensor: (160, 120),
        crop: [2, 2, 156, 116],
        declared: [5, 4, 148, 110],
        spots: vec![(90.0, 40.0, 3.0), (30.0, 80.0, 5.0)],
    });
}

#[test]
fn an_x_trans_export_at_a_size_takes_its_first_rounds_at_the_exports_scale() {
    tiles_after_the_resample(&Raw {
        cfa: Pattern {
            width: 6,
            height: 6,
            values: X_TRANS.to_vec(),
        },
        sensor: (162, 126),
        crop: [1, 3, 158, 120],
        declared: [4, 5, 150, 112],
        spots: vec![(90.0, 40.0, 3.0), (30.0, 80.0, 5.0)],
    });
}

/// The adapter, opened to bind at most `bytes` of a buffer at once.
fn gpu_binding(bytes: u64) -> GpuContext {
    let options = GpuOptions {
        max_storage_binding: Some(bytes),
        storage_offset_alignment: Some(256),
        ..GpuOptions::default()
    };
    pollster::block_on(GpuContext::new(&options)).expect("a GPU adapter, hardware or software")
}

#[test]
fn a_sized_export_tiles_its_whole_image_passes_where_the_image_does_not_fit() {
    // The large raw at 0.3 with high quality: where the GPU binds less than its image at
    // full resolution, 6 MB, the tone equalizer's mask, 1.5 MB, still binds, so its
    // export runs the raw stage in row strips and the rest in two rounds of tiles; where
    // the budget is smaller than the image's buffers, the tone equalizer's and haze
    // removal's exports tile in two rounds too (haze removal's copy of the image binds
    // only where the image does). Each equals the untiled export.
    let raw = large(
        Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 1, 2],
        },
        (767, 514),
        [2, 3, 762, 508],
        [5, 6, 752, 500],
    );
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default())).unwrap();
    let narrow = gpu_binding(2 << 20);
    let resample = raw.at_scale(0.3, true);
    for (k, extra) in with_whole_image_passes().into_iter().enumerate() {
        let id = extra.operation.manifest().id;
        let mut instances = raw.instances();
        instances.push(extra);
        let whole = raw
            .export_at(&gpu, Tiling::Whole, &instances, resample)
            .unwrap();
        let mut cases = vec![("a 16 MiB budget", &gpu, Tiling::Auto { budget: 16 << 20 })];
        if k == 0 {
            cases.push(("a 2 MiB binding", &narrow, Tiling::Auto { budget: BUDGET }));
        }
        for (what, gpu, tiling) in cases {
            let tiled = raw.export_at(gpu, tiling, &instances, resample).unwrap();
            assert!(
                tiled.tiles > 1 && tiled.rounds == 2,
                "{id}, {what}: {} tiles, {} rounds",
                tiled.tiles,
                tiled.rounds
            );
            assert!(
                tiled.pixels == whole.pixels,
                "{id}, {what}: equals the whole export"
            );
        }
    }
}

/// [`tiled_after_the_resample_equals_untiled`] for the operations that take a first
/// round, where the image does not fit: under a 16 MiB budget, less than the whole
/// export's buffers, and on a GPU binding 2 MiB at once, less than the image at full
/// resolution, so demosaic runs only in tiles, each walked back from the output's. Local
/// contrast and filmic rgb run whole with everything before them, demosaic among it,
/// which binds 6 MB there.
fn tiles_after_the_resample_where_the_image_does_not_fit(raw: &Raw) {
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default())).unwrap();
    let narrow = gpu_binding(2 << 20);
    let tilings = [
        ("a 16 MiB budget", &gpu, Tiling::Auto { budget: 16 << 20 }),
        ("a 2 MiB binding", &narrow, Tiling::Auto { budget: BUDGET }),
    ];
    tiled_after_the_resample_equals_untiled(raw, &tilings, true);
}

#[test]
fn a_bayer_export_at_a_size_tiles_its_first_rounds_where_the_image_does_not_fit() {
    tiles_after_the_resample_where_the_image_does_not_fit(&large(
        Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 1, 2],
        },
        (767, 514),
        [2, 3, 762, 508],
        [5, 6, 752, 500],
    ));
}

#[test]
fn an_x_trans_export_at_a_size_tiles_its_first_rounds_where_the_image_does_not_fit() {
    tiles_after_the_resample_where_the_image_does_not_fit(&large(
        Pattern {
            width: 6,
            height: 6,
            values: X_TRANS.to_vec(),
        },
        (771, 520),
        [1, 3, 766, 512],
        [4, 5, 756, 504],
    ));
}

/// `raw`'s default pipeline with haze removal and the tone equalizer
/// ([`with_whole_image_passes`]).
fn with_haze_and_tone(raw: &Raw) -> Vec<Instance<'static>> {
    let mut instances = raw.instances();
    instances.extend(with_whole_image_passes());
    instances
}

/// `raw` exported at 0.5 and 1/3 without high quality through haze removal and the tone
/// equalizer, whole and as `tilings` say. Haze removal comes before the resample, its first
/// round in tiles of the image at full resolution, and the tone equalizer after it, its
/// first round in the output's tiles at the export's scale (decision 15): every tiled
/// export takes those two rounds and the last, in more than one tile, and equals the whole
/// one bit for bit, which haze removal changes.
fn first_rounds_on_both_sides_equal_untiled(raw: &Raw, tilings: &[Tiling]) {
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default()))
        .expect("a GPU adapter, hardware or software");
    let instances = with_haze_and_tone(raw);
    let without_haze: Vec<Instance> = instances
        .iter()
        .filter(|instance| instance.operation.manifest().id != "hazeremoval")
        .cloned()
        .collect();
    for scale in [0.5, 1.0 / 3.0] {
        let resample = raw.at_scale(scale, false);
        let [width, height] = resample.output;
        let whole = raw
            .export_at(&gpu, Tiling::Whole, &instances, resample)
            .unwrap();
        assert_eq!(
            (whole.desc.width, whole.desc.height, whole.tiles),
            (width, height, 1)
        );
        assert!(whole.pixels.iter().flatten().all(|v| v.is_finite()));
        let without = raw
            .export_at(&gpu, Tiling::Whole, &without_haze, resample)
            .unwrap();
        assert!(whole.pixels != without.pixels, "haze removal at {scale}");
        for tiling in tilings {
            let at = format!("at {scale}, {tiling:?}");
            let tiled = raw
                .export_at(&gpu, *tiling, &instances, resample)
                .unwrap_or_else(|error| panic!("{at}: {error}"));
            assert!(
                tiled.tiles > 1 && tiled.rounds == 3,
                "{at}: {} tiles, {} rounds",
                tiled.tiles,
                tiled.rounds
            );
            if let Some(i) = tiled
                .pixels
                .iter()
                .zip(&whole.pixels)
                .position(|(a, b)| a != b)
            {
                panic!(
                    "{at}: pixel ({}, {}) is {:?} tiled, {:?} whole",
                    i as u32 % width,
                    i as u32 / width,
                    tiled.pixels[i],
                    whole.pixels[i]
                );
            }
        }
    }
}

#[test]
fn a_bayer_export_at_a_size_takes_first_rounds_on_both_sides_of_the_resample() {
    first_rounds_on_both_sides_equal_untiled(
        &Raw {
            cfa: Pattern {
                width: 2,
                height: 2,
                values: vec![0, 1, 1, 2],
            },
            sensor: (160, 120),
            crop: [2, 2, 156, 116],
            declared: [5, 4, 148, 110],
            spots: vec![(90.0, 40.0, 3.0), (30.0, 80.0, 5.0)],
        },
        &[Tiling::Tiles([24, 20]), Tiling::Tiles([37, 29])],
    );
}

#[test]
fn a_sized_export_with_haze_removal_tiles_both_first_rounds_under_a_small_budget() {
    // Under a 16 MiB budget, less than the whole export's buffers, Bayer and X-Trans.
    let budget = [Tiling::Auto { budget: 16 << 20 }];
    first_rounds_on_both_sides_equal_untiled(
        &large(
            Pattern {
                width: 2,
                height: 2,
                values: vec![0, 1, 1, 2],
            },
            (767, 514),
            [2, 3, 762, 508],
            [5, 6, 752, 500],
        ),
        &budget,
    );
    first_rounds_on_both_sides_equal_untiled(
        &large(
            Pattern {
                width: 6,
                height: 6,
                values: X_TRANS.to_vec(),
            },
            (771, 520),
            [1, 3, 766, 512],
            [4, 5, 756, 504],
        ),
        &budget,
    );
}

#[test]
fn a_tiled_export_a_pixel_short_of_1_with_local_contrast_equals_the_whole_one() {
    // A pixel short of 1 the resample is a copy of the output's part of the image, after
    // demosaic, and local contrast and filmic rgb's reconstruction, which take no first
    // round, run whole before the tiles with everything before them, the copy included,
    // over the output (decision 15). A split that left the copy out would run them over the
    // image at full size, and the tiled export would differ.
    let raw = Raw {
        cfa: Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 1, 2],
        },
        sensor: (110, 86),
        crop: [2, 2, 106, 82],
        declared: [5, 4, 98, 76],
        spots: vec![(60.0, 30.0, 3.0), (20.0, 60.0, 5.0)],
    };
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default())).unwrap();
    let resample = raw.at_scale(0.999_999_99, false);
    for (operation, params, rounds) in &after_the_resample() {
        if *rounds != 1 {
            continue;
        }
        let id = operation.manifest().id;
        let instances = through(&raw, *operation, params);
        let whole = raw
            .export_at(&gpu, Tiling::Whole, &instances, resample)
            .unwrap();
        assert_eq!(
            [whole.desc.width, whole.desc.height],
            resample.output,
            "{id}"
        );
        for tiling in [Tiling::Tiles([24, 20]), Tiling::Tiles([37, 29])] {
            let tiled = raw.export_at(&gpu, tiling, &instances, resample).unwrap();
            assert!(tiled.tiles > 1, "{id}, {tiling:?}: {} tiles", tiled.tiles);
            assert!(
                tiled.pixels == whole.pixels,
                "{id}, {tiling:?}: equals the whole export"
            );
        }
    }
}

#[test]
fn a_tiled_export_at_a_scale_of_1_with_the_tone_equalizer_equals_the_whole_one() {
    // At a scale of 1 the resample makes the image itself, so the export at that size is
    // the full-size export, and in tiles, the tone equalizer's first round taken by its
    // place after the resample in the plan, it equals the whole export bit for bit; with
    // haze removal too, before the resample. A pixel short of 1, likewise, the tone
    // equalizer covering the output.
    let raw = Raw {
        cfa: Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 1, 2],
        },
        sensor: (110, 86),
        crop: [2, 2, 106, 82],
        declared: [5, 4, 98, 76],
        spots: vec![(60.0, 30.0, 3.0), (20.0, 60.0, 5.0)],
    };
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default())).unwrap();
    let mut tone = raw.instances();
    tone.push(with_whole_image_passes()[0].clone());
    let both = with_haze_and_tone(&raw);
    for (what, instances, rounds) in [("tone", &tone, 2), ("haze and tone", &both, 3)] {
        let full = raw.export_through(&gpu, Tiling::Whole, instances);
        for scale in [1.0, 0.999_999_99] {
            let resample = raw.at_scale(scale, false);
            let at = format!("{what} at {scale}");
            let whole = raw
                .export_at(&gpu, Tiling::Whole, instances, resample)
                .unwrap();
            if scale == 1.0 {
                assert!(whole.pixels == full.pixels, "{at}: the full-size export");
            }
            for tiling in [Tiling::Tiles([24, 20]), Tiling::Tiles([37, 29])] {
                let tiled = raw.export_at(&gpu, tiling, instances, resample).unwrap();
                assert!(
                    tiled.tiles > 1 && tiled.rounds == rounds,
                    "{at}, {tiling:?}: {} tiles, {} rounds",
                    tiled.tiles,
                    tiled.rounds
                );
                assert!(
                    tiled.pixels == whole.pixels,
                    "{at}, {tiling:?}: equals the whole export"
                );
            }
        }
    }
}

#[test]
fn a_tiled_x_trans_export_equals_the_whole_one() {
    tiled_equals_untiled(Raw {
        cfa: Pattern {
            width: 6,
            height: 6,
            values: X_TRANS.to_vec(),
        },
        sensor: (120, 96),
        crop: [1, 3, 116, 90],
        declared: [4, 5, 108, 84],
        spots: Vec::new(),
    });
}

/// The forced binding: the 16-bit mosaics below take about 790 KB, the float ones about
/// 1.5 MB, and an RGBA tile of 128 pixels square 256 KB.
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
    let limits = gpu.device.limits();
    assert_eq!(limits.max_storage_buffer_binding_size, SMALL_BINDING);
    assert_eq!(limits.min_storage_buffer_offset_alignment, 256);
    gpu
}

/// A raw of about 770 by 515 photosites, its 16-bit rows an odd number of photosites
/// wide, with spots that clip across the rows where the raw stage's strips meet under
/// `SMALL_BINDING`: every 128 rows for highlight reconstruction's sums, every 171 or so
/// for the other passes.
fn large(cfa: Pattern<u32>, sensor: (u32, u32), crop: [u32; 4], declared: [u32; 4]) -> Raw {
    let y = |row: f32| crop[1] as f32 + row;
    Raw {
        cfa,
        sensor,
        crop,
        declared,
        spots: vec![
            (150.0, y(128.0), 9.0),
            (330.0, y(171.0), 7.0),
            (420.0, y(256.0), 12.0),
            (560.0, y(342.0), 8.0),
            (640.0, y(384.0), 10.0),
        ],
    }
}

/// On a GPU that binds less than the image at once, the raw stage runs in row strips and
/// the tiles copy from windows of its output; the export equals the whole one on the
/// adapter as it is, bit for bit.
fn a_small_binding_equals_the_whole(raw: Raw) {
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default()))
        .expect("a GPU adapter, hardware or software");
    let whole = raw.export(&gpu, Tiling::Whole).pixels;
    let small = small_gpu();

    // The raw stage runs in strips there, every pass that reads the mosaic; the 16-bit
    // mosaic is read through windows too.
    let raw_stage: Vec<Instance> = OPERATIONS
        .iter()
        .filter(|op| ["rawprepare", "temperature", "highlights"].contains(&op.manifest().id))
        .map(|&operation| Instance {
            operation,
            params: ParamValues::new(),
        })
        .collect();
    let (width, height) = raw.sensor;
    let sensor = ImageDesc::new(width, height, Format::Mosaic16);
    assert!(sensor.buffer_size() > SMALL_BINDING);
    let plan = pipeline::plan(
        &raw_stage,
        &raw.image(),
        sensor,
        Pipe::Export,
        &Limits::of(&small),
    )
    .unwrap();
    for op in &plan.operations {
        for (pass, dispatches) in op.passes.iter().zip(&op.dispatches) {
            let reads = pass.strips.is_some();
            assert_eq!(
                dispatches.len() > 1,
                reads,
                "{} {}",
                op.id,
                pass.entry_point
            );
        }
    }
    let rawprepare = &plan.operations[0].dispatches[0];
    assert!(
        rawprepare
            .iter()
            .all(|d| d.input.is_some() && d.output.is_some())
    );
    assert!(plan.fits(&Limits::of(&small)));

    let [.., declared_width, declared_height] = raw.declared;
    for tiling in [Tiling::Auto { budget: BUDGET }, Tiling::Tiles([100, 90])] {
        let Exported {
            tiles,
            pixels: banded,
            ..
        } = raw.export(&small, tiling);
        assert!(tiles > 1, "{tiling:?}");
        if let Some(i) = banded
            .iter()
            .zip(&whole)
            .position(|(a, b)| a.map(f32::to_bits) != b.map(f32::to_bits))
        {
            panic!(
                "{tiling:?}: pixel ({}, {}) is {:?} binding little, {:?} whole",
                i as u32 % declared_width,
                i as u32 / declared_width,
                banded[i],
                whole[i]
            );
        }
        assert_eq!(banded.len(), (declared_width * declared_height) as usize);
    }
}

#[test]
fn a_bayer_export_binding_little_at_once_equals_the_whole_one() {
    a_small_binding_equals_the_whole(large(
        Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 1, 2],
        },
        (767, 514),
        [2, 3, 762, 508],
        [5, 6, 752, 500],
    ));
}

#[test]
fn an_x_trans_export_binding_little_at_once_equals_the_whole_one() {
    a_small_binding_equals_the_whole(large(
        Pattern {
            width: 6,
            height: 6,
            values: X_TRANS.to_vec(),
        },
        (771, 520),
        [1, 3, 766, 512],
        [4, 5, 756, 504],
    ));
}

#[test]
fn whole_image_scratch_no_tile_can_bind_is_refused_naming_its_pass() {
    // On a GPU that binds little, the whole-image scratch an export holds across its
    // rounds, bound whole and the same at any tile size, cannot bind: the export is
    // refused naming the operation, the pass and the bytes, before any tile size is
    // tried (decision 15), for haze removal's copy of the image, bloom's lights and
    // soften's planes alike (decision 7).
    use apertrust_operations::{bloom, hazeremoval, soften};
    let raw = large(
        Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 1, 2],
        },
        (767, 514),
        [2, 3, 762, 508],
        [5, 6, 752, 500],
    );
    let small = small_gpu();
    let haze = hazeremoval::values(&hazeremoval::Settings {
        strength: 0.9,
        distance: 0.8,
    });
    let cases: [(&'static dyn Operation, ParamValues, &str, u64); 3] = [
        (&hazeremoval::HazeRemoval, haze, "image_copy", 16),
        (
            &bloom::Bloom,
            bloom::values(&bloom::Settings::default()),
            "threshold",
            4,
        ),
        (
            &soften::Soften,
            soften::values(&soften::Settings::default()),
            "overexpose",
            4,
        ),
    ];
    for (operation, params, entry_point, bytes_a_pixel) in cases {
        let id = operation.manifest().id;
        let mut instances = raw.instances();
        instances.push(Instance { operation, params });
        let (width, height) = raw.sensor;
        let input = GpuImage::upload_mosaic(&small, width, height, &raw.values()).unwrap();
        let tiling = Tiling::Auto { budget: BUDGET };
        let refused = pollster::block_on(export(&small, &instances, &raw.image(), input, tiling));
        let Err(ExportError::HeldScratch {
            operation,
            pass,
            bytes,
            limit,
        }) = refused
        else {
            panic!("{id}: {:?}", refused.err());
        };
        assert_eq!((operation, pass, limit), (id, entry_point, SMALL_BINDING));
        assert_eq!(bytes, 752 * 500 * bytes_a_pixel, "{id}");
    }
}
