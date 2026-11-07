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

//! The pipeline on a GPU adapter, hardware or software: real operations give exactly
//! what their kernels give alone, and mosaics, size changes, explicit bindings and
//! scratch buffers reach the kernels as planned. And, without one, the pipe's output
//! profile reaching every operation's `prepare`, and the planner telling a whole-image
//! operation its whole image at the pipe's scale, after an export's resample its output.

use std::sync::Mutex;

use apertrust_color::{
    PROFILE_D65, REC2020_PRIMARIES, SRGB_PRIMARIES, inverse_d, profile_rgb_to_xyz_d50, to_f32,
};
use apertrust_engine::compute::{GpuImage, ImageKernel};
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe, Pipeline, Resample};
use apertrust_operations::agx::Agx;
use apertrust_operations::colorout::{Colorout, PROFILES};
use apertrust_operations::demosaic::Demosaic;
use apertrust_operations::exposure::Exposure;
use apertrust_operations::rawprepare::Rawprepare;
use apertrust_operations::temperature::Temperature;
use apertrust_plugin::{
    Binding, Camera, Domain, Format, ImageContext, ImageDesc, Kernel, Manifest, NotPorted,
    Operation, OutputProfile, ParamValues, Pass, PassKind, Pattern, Prepared, Resource, Schema,
    Scratch, Slot, Stage, Value,
};

/// A small image whose width and height are not multiples of the 16-pixel workgroup,
/// from deep shadow to well above white, with every hue.
fn pixels(width: u32, height: u32) -> Vec<[f32; 4]> {
    (0..height)
        .flat_map(|y| {
            (0..width).map(move |x| {
                let level = 2f32.powf(-8.0 + 12.0 * x as f32 / width as f32);
                let hue = y as f32 / height as f32;
                [level * (1.0 - hue), level * (0.2 + hue), level * 0.5, 1.0]
            })
        })
        .collect()
}

#[test]
fn exposure_then_agx_match_their_kernels_run_alone() {
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default()))
        .expect("a GPU adapter, hardware or software");
    let (width, height) = (37, 23);
    let source = pixels(width, height);
    let image = ImageContext::full(width, height);
    let mut exposure = ParamValues::new();
    exposure.set("exposure", Value::Float(1.25));
    let instances = [
        Instance {
            operation: &Agx,
            params: ParamValues::new(),
        },
        Instance {
            operation: &Exposure,
            params: exposure,
        },
    ];

    let plan = pipeline::plan(
        &instances,
        &image,
        ImageDesc::rgba(width, height),
        Pipe::Export,
        &Limits::NONE,
    )
    .unwrap();
    let order: Vec<_> = plan.operations.iter().map(|op| op.id).collect();
    assert_eq!(order, ["exposure", "agx"]);
    let expected_params: Vec<_> = plan.passes().map(|(_, pass)| pass.params.clone()).collect();
    let input = GpuImage::upload(&gpu, width, height, &source).unwrap();
    let mut pipeline = pollster::block_on(Pipeline::new(&gpu, plan, input)).unwrap();
    pipeline.render(&gpu).unwrap();
    let actual = pipeline.output().download(&gpu).unwrap();

    // The same two kernels, dispatched one after the other.
    let mut image = GpuImage::upload(&gpu, width, height, &source).unwrap();
    let operations: [&dyn Operation; 2] = [&Exposure, &Agx];
    for (operation, params) in operations.iter().zip(&expected_params) {
        let kernel = operation.kernel();
        let compiled =
            pollster::block_on(ImageKernel::new(&gpu, kernel.name, kernel.wgsl)).unwrap();
        let output = GpuImage::new(&gpu, width, height);
        compiled.dispatch(&gpu, &image, &output, params, &[]);
        image = output;
    }
    let expected = image.download(&gpu).unwrap();

    assert_eq!(actual.len(), expected.len());
    for (i, (a, e)) in actual.iter().zip(&expected).enumerate() {
        assert_eq!(
            a.map(f32::to_bits),
            e.map(f32::to_bits),
            "pixel {i}: {a:?} != {e:?}"
        );
    }
    // AgX moved the pixels: the comparison is not of two untouched images.
    assert_ne!(actual[0], source[0]);
}

/// Kernels for two fake raw-stage operations: `unpack` crops the 16-bit mosaic and
/// scales it by 2^-16, exactly on any GPU, since WGSL's division need not round
/// correctly; `spread` doubles it into a scratch buffer in one pass and
/// writes it out as grey RGBA in a second.
const WGSL: &str = r#"
struct Header {
    width: u32, height: u32, origin_x: u32, origin_y: u32,
    full_width: u32, full_height: u32, scale: f32, repeat: u32,
    input_base: u32, output_base: u32, reserved0: u32, reserved1: u32,
}
struct Params { header: Header, crop_x: u32, crop_y: u32, sensor_width: u32, pad: u32 }

@group(0) @binding(0) var<storage, read> packed: array<u32>;
@group(0) @binding(1) var<storage, read_write> mosaic: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= params.header.width || id.y >= params.header.height) { return; }
    let i = (id.y + params.crop_y) * params.sensor_width + id.x + params.crop_x;
    let word = packed[i / 2u];
    let value = select(word >> 16u, word & 0xffffu, i % 2u == 0u);
    mosaic[id.y * params.header.width + id.x] = f32(value) * 0.0000152587890625;
}
"#;

const SPREAD_WGSL: &str = r#"
struct Header {
    width: u32, height: u32, origin_x: u32, origin_y: u32,
    full_width: u32, full_height: u32, scale: f32, repeat: u32,
    input_base: u32, output_base: u32, reserved0: u32, reserved1: u32,
}

@group(0) @binding(0) var<storage, read> mosaic: array<f32>;
@group(0) @binding(1) var<storage, read_write> rgba: array<vec4<f32>>;
@group(0) @binding(2) var<uniform> header: Header;
@group(0) @binding(10) var<storage, read_write> doubled: array<f32>;

@compute @workgroup_size(16, 16)
fn double(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= header.width || id.y >= header.height) { return; }
    let i = id.y * header.width + id.x;
    doubled[i] = 2.0 * mosaic[i];
}

@compute @workgroup_size(16, 16)
fn assemble(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= header.width || id.y >= header.height) { return; }
    let i = id.y * header.width + id.x;
    rgba[i] = vec4<f32>(doubled[i], doubled[i], doubled[i], 1.0);
}
"#;

static UNPACK_KERNEL: Kernel = Kernel {
    name: "unpack",
    wgsl: WGSL,
    derivatives: false,
};
static SPREAD_KERNEL: Kernel = Kernel {
    name: "spread",
    wgsl: SPREAD_WGSL,
    derivatives: false,
};
static NO_PARAMS: Schema = Schema {
    version: 1,
    params: &[],
};

fn manifest(id: &'static str, order: f32) -> &'static Manifest {
    Box::leak(Box::new(Manifest {
        id,
        version: 1,
        slot: Slot {
            stage: Stage::Raw,
            order,
        },
        domain: Domain::Scene,
        layers: false,
        differentiable: false,
        radius: 0,
        default_enabled: true,
        provenance: None,
    }))
}

/// Crops `CROP` out of the sensor's mosaic.
struct Unpack;
const SENSOR: (u32, u32) = (9, 7);
const CROP: [u32; 4] = [1, 2, 6, 4];

impl Operation for Unpack {
    fn manifest(&self) -> &'static Manifest {
        manifest("unpack", 1.0)
    }
    fn schema(&self) -> &'static Schema {
        &NO_PARAMS
    }
    fn kernel(&self) -> &'static Kernel {
        &UNPACK_KERNEL
    }
    fn prepare(
        &self,
        _: &ParamValues,
        _: &ImageContext,
        input: &ImageDesc,
    ) -> Result<Prepared, NotPorted> {
        assert_eq!(input.format, Format::Mosaic16);
        let params = [CROP[0], CROP[1], input.width, 0]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let mut prepared = Prepared::per_pixel(input, params, Vec::new());
        prepared.output = ImageDesc {
            sensor_origin: [CROP[0], CROP[1]],
            ..ImageDesc::new(CROP[2], CROP[3], Format::Mosaic)
        };
        Ok(prepared)
    }
}

/// Mosaic to RGBA in two passes through a scratch buffer.
struct Spread;

impl Operation for Spread {
    fn manifest(&self) -> &'static Manifest {
        manifest("spread", 8.0)
    }
    fn schema(&self) -> &'static Schema {
        &NO_PARAMS
    }
    fn kernel(&self) -> &'static Kernel {
        &SPREAD_KERNEL
    }
    fn prepare(
        &self,
        _: &ParamValues,
        _: &ImageContext,
        input: &ImageDesc,
    ) -> Result<Prepared, NotPorted> {
        let pass = |entry_point, bindings| Pass {
            entry_point,
            kind: PassKind::PerPixel,
            params: Vec::new(),
            tables: Vec::new(),
            bindings,
            size: None,
            strips: None,
        };
        Ok(Prepared {
            output: ImageDesc {
                format: Format::Rgba,
                ..*input
            },
            passes: vec![
                pass(
                    "double",
                    vec![
                        Binding::new(0, Resource::Input),
                        Binding::new(2, Resource::Uniforms),
                        Binding::new(10, Resource::Scratch(0)),
                    ],
                ),
                pass(
                    "assemble",
                    vec![
                        Binding::new(1, Resource::Output),
                        Binding::new(2, Resource::Uniforms),
                        Binding::new(10, Resource::Scratch(0)),
                    ],
                ),
            ],
            scratch: vec![Scratch::per_pixel(input, 4)],
        })
    }
}

#[test]
fn a_mosaic_is_cropped_and_spread_through_scratch() {
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default()))
        .expect("a GPU adapter, hardware or software");
    let (width, height) = SENSOR;
    // An odd number of photosites, so the last 32-bit word is half padding.
    let values: Vec<u16> = (0..width * height)
        .map(|i| (i * 997 % 65536) as u16)
        .collect();
    let input = GpuImage::upload_mosaic(&gpu, width, height, &values).unwrap();
    let instances = [
        Instance {
            operation: &Spread,
            params: ParamValues::new(),
        },
        Instance {
            operation: &Unpack,
            params: ParamValues::new(),
        },
    ];
    let image = ImageContext::full(CROP[2], CROP[3]);
    let plan = pipeline::plan(
        &instances,
        &image,
        input.desc(),
        Pipe::Export,
        &Limits::NONE,
    )
    .unwrap();
    let mut pipeline = pollster::block_on(Pipeline::new(&gpu, plan, input)).unwrap();
    let timings = pipeline.render(&gpu).unwrap();
    let labels: Vec<_> = timings.passes.iter().map(|(label, _)| *label).collect();
    assert_eq!(labels, ["unpack", "double", "assemble"]);

    let actual = pipeline.output().download(&gpu).unwrap();
    let [x0, y0, w, h] = CROP;
    let expected: Vec<[f32; 4]> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| {
            let v =
                2.0 * (f32::from(values[((y + y0) * width + x + x0) as usize]) * 2f32.powi(-16));
            [v, v, v, 1.0]
        })
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn the_raw_stage_crops_scales_and_white_balances_each_photosite() {
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default()))
        .expect("a GPU adapter, hardware or software");
    // An odd-sized sensor whose crop starts at an odd row and column, so the parity
    // positions and filter colours on the sensor differ from those in the crop: red
    // and blue would swap if temperature took them from the crop.
    let (width, height) = (37, 21);
    let values: Vec<u16> = (0..width * height)
        .map(|i| (1000 + (i * 7919) % 12000) as u16)
        .collect();
    let crop = [3, 5, 29, 13];
    let black = [1030, 1031, 1026, 1027];
    let white = 13600;
    let coefficients = [2.25f32, 1.0, 1.5];
    let image = ImageContext {
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
                width: 2,
                height: 2,
                values: black.map(|b| b as f32).to_vec(),
            },
            white: white as f32,
            white_balance: Some(coefficients.to_vec()),
            xyz_to_camera: None,
            crop,
            declared: crop,
            exposure_bias: 0.0,
            iso: 0.0,
            linear_response_limit: 1.0,
            lens_correction: None,
            orientation: 0,
        }),
        ..ImageContext::full(crop[2], crop[3])
    };
    let instances = [
        Instance {
            operation: &Temperature,
            params: ParamValues::new(),
        },
        Instance {
            operation: &Rawprepare,
            params: ParamValues::new(),
        },
    ];
    let plan = pipeline::plan(
        &instances,
        &image,
        ImageDesc::new(width, height, Format::Mosaic16),
        Pipe::Export,
        &Limits::NONE,
    )
    .unwrap();
    let input = GpuImage::upload_mosaic(&gpu, width, height, &values).unwrap();
    let mut pipeline = pollster::block_on(Pipeline::new(&gpu, plan, input)).unwrap();
    pipeline.render(&gpu).unwrap();
    let actual = pipeline.output().download_mosaic(&gpu).unwrap();

    let [x0, y0, w, h] = crop;
    for y in 0..h {
        for x in 0..w {
            let (sx, sy) = (x + x0, y + y0);
            let position = (((sy & 1) << 1) | (sx & 1)) as usize;
            let b = black[position] as f32;
            let raw = f32::from(values[(sy * width + sx) as usize]);
            // RGGB from the sensor's corner.
            let coefficient = coefficients[[0, 1, 1, 2][position]];
            let expected = (raw - b) / (white as f32 - b) * coefficient;
            let got = actual[(y * w + x) as usize];
            // WGSL's division is within 2.5 ULP, and the product adds one rounding.
            assert!(
                (got - expected).abs() <= 4.0 * f32::EPSILON * expected.abs().max(1.0),
                "({x}, {y}): {got} != {expected}"
            );
        }
    }
}

#[test]
fn the_raw_stage_demosaics_a_grey_bayer_scene_to_grey_in_the_declared_crop() {
    demosaic_grey(Pattern {
        width: 2,
        height: 2,
        values: vec![0, 1, 1, 2],
    });
}

#[test]
fn the_raw_stage_demosaics_a_grey_x_trans_scene_to_grey_in_the_declared_crop() {
    demosaic_grey(Pattern {
        width: 6,
        height: 6,
        values: vec![
            1, 1, 0, 1, 1, 2, //
            1, 1, 2, 1, 1, 0, //
            2, 0, 1, 0, 2, 1, //
            1, 1, 2, 1, 1, 0, //
            1, 1, 0, 1, 1, 2, //
            0, 2, 1, 2, 0, 1, //
        ],
    });
}

/// A grey scene: red and blue read less than green by the white balance's factors, so
/// temperature evens them out, and demosaic must give grey everywhere, in the edge band
/// (PPG, or linear VNG for X-Trans) and in the interior (RCD or Markesteijn).
fn demosaic_grey(cfa: Pattern<u32>) {
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default()))
        .expect("a GPU adapter, hardware or software");
    let (width, height) = (45, 37);
    let coefficients = [2.0f32, 1.0, 1.5];
    let grey = 0.3f32;
    let white = 4000.0f32;
    let values: Vec<u16> = (0..width * height)
        .map(|i| {
            let (x, y) = (i % width, i / width);
            let color = cfa.values[((y % cfa.height) * cfa.width + x % cfa.width) as usize];
            (grey / coefficients[color as usize] * white).round() as u16
        })
        .collect();
    let crop = [1, 1, 43, 35];
    let declared = [4, 3, 36, 30];
    let image = ImageContext {
        camera: Some(Camera {
            make: String::new(),
            model: String::new(),
            alias: String::new(),
            width,
            height,
            cfa: Some(cfa.clone()),
            black: Pattern {
                width: 1,
                height: 1,
                values: vec![0.0],
            },
            white,
            white_balance: Some(coefficients.to_vec()),
            xyz_to_camera: None,
            crop,
            declared,
            exposure_bias: 0.0,
            iso: 0.0,
            linear_response_limit: 1.0,
            lens_correction: None,
            orientation: 0,
        }),
        ..ImageContext::full(declared[2], declared[3])
    };
    let instances: Vec<Instance> = [&Rawprepare as &dyn Operation, &Temperature, &Demosaic]
        .into_iter()
        .map(|operation| Instance {
            operation,
            params: ParamValues::new(),
        })
        .collect();
    let plan = pipeline::plan(
        &instances,
        &image,
        ImageDesc::new(width, height, Format::Mosaic16),
        Pipe::Export,
        &Limits::NONE,
    )
    .unwrap();
    let (_, output) = plan.output();
    assert_eq!((output.width, output.height), (36, 30));
    assert_eq!(output.format, Format::Rgba);
    assert_eq!(output.sensor_origin, [4, 3]);
    let input = GpuImage::upload_mosaic(&gpu, width, height, &values).unwrap();
    let mut pipeline = pollster::block_on(Pipeline::new(&gpu, plan, input)).unwrap();
    pipeline.render(&gpu).unwrap();
    let pixels = pipeline.output().download(&gpu).unwrap();
    // The photosites round to whole raw values, so grey holds to about 1e-3.
    for (i, pixel) in pixels.iter().enumerate() {
        for &channel in &pixel[..3] {
            assert!((channel - grey).abs() < 2e-3, "pixel {i}: {pixel:?}");
        }
    }
}

/// A working-space operation, before colorout as filmic rgb is, that keeps the output
/// profile its `defaults` and its `prepare` were given, each time.
#[derive(Default)]
struct Witness {
    defaults: Mutex<Vec<Option<OutputProfile>>>,
    prepared: Mutex<Vec<Option<OutputProfile>>>,
}

static WITNESS_MANIFEST: Manifest = Manifest {
    id: "witness",
    version: 1,
    slot: Slot {
        stage: Stage::WorkingSpace,
        order: 40.0,
    },
    domain: Domain::Scene,
    layers: false,
    differentiable: false,
    radius: 0,
    default_enabled: true,
    provenance: None,
};

/// Planned, never compiled.
static WITNESS_KERNEL: Kernel = Kernel {
    name: "witness",
    wgsl: "",
    derivatives: false,
};

impl Operation for Witness {
    fn manifest(&self) -> &'static Manifest {
        &WITNESS_MANIFEST
    }
    fn schema(&self) -> &'static Schema {
        &NO_PARAMS
    }
    fn kernel(&self) -> &'static Kernel {
        &WITNESS_KERNEL
    }
    fn defaults(&self, image: &ImageContext) -> ParamValues {
        self.defaults.lock().unwrap().push(image.output_profile);
        NO_PARAMS.defaults()
    }
    fn prepare(
        &self,
        _: &ParamValues,
        image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<Prepared, NotPorted> {
        self.prepared.lock().unwrap().push(image.output_profile);
        Ok(Prepared::per_pixel(input, Vec::new(), Vec::new()))
    }
}

/// What the witness's `defaults` and `prepare` see in `pipe`, planned alone or, if
/// `profile` names one, with colorout of that type.
fn witnessed(
    profile: Option<&str>,
    pipe: Pipe,
) -> (Vec<Option<OutputProfile>>, Vec<Option<OutputProfile>>) {
    let witness = Witness::default();
    let mut instances = vec![Instance {
        operation: &witness,
        params: ParamValues::new(),
    }];
    if let Some(profile) = profile {
        let index = PROFILES.iter().position(|&p| p == profile).unwrap() as u32;
        let mut params = ParamValues::new();
        params.set("type", Value::Choice(index));
        instances.push(Instance {
            operation: &Colorout,
            params,
        });
    }
    let (width, height) = (8, 6);
    pipeline::plan(
        &instances,
        &ImageContext::full(width, height),
        ImageDesc::rgba(width, height),
        pipe,
        &Limits::NONE,
    )
    .unwrap();
    drop(instances);
    (
        witness.defaults.into_inner().unwrap(),
        witness.prepared.into_inner().unwrap(),
    )
}

/// An RGB space's matrices as darktable's built-in profiles have them.
fn profile(primaries: [[f64; 2]; 3]) -> OutputProfile {
    let to_xyz = profile_rgb_to_xyz_d50(PROFILE_D65, primaries);
    OutputProfile {
        rgb_to_xyz_d50: to_f32(to_xyz),
        xyz_d50_to_rgb: to_f32(inverse_d(&to_xyz)),
    }
}

#[test]
fn prepare_sees_the_output_profile_colorout_reports_and_defaults_never_do() {
    let viewport = Pipe::Viewport {
        width: 4,
        height: 3,
    };
    for pipe in [Pipe::Export, viewport] {
        let (defaults, prepared) = witnessed(Some("srgb"), pipe);
        assert_eq!(prepared, [Some(profile(SRGB_PRIMARIES))], "{pipe:?}");
        assert!(!defaults.is_empty(), "{pipe:?}");
        assert!(
            defaults.iter().all(Option::is_none),
            "{pipe:?}: {defaults:?}"
        );
    }
}

#[test]
fn a_plan_without_colorout_has_the_working_profile() {
    let (_, prepared) = witnessed(None, Pipe::Export);
    assert_eq!(prepared, [None]);
}

#[test]
fn colorout_s_type_changes_the_output_profile_prepare_sees() {
    let (_, prepared) = witnessed(Some("lin_rec2020"), Pipe::Export);
    assert_eq!(prepared, [Some(profile(REC2020_PRIMARIES))]);
    assert_ne!(profile(REC2020_PRIMARIES), profile(SRGB_PRIMARIES));
}

#[test]
fn a_viewport_whose_scale_rounds_to_1_lays_blooms_scratch_out_as_its_input() {
    // A 1:2 image in a viewport a pixel shorter: the scale, from the width, rounds back
    // to 1 (99.5 to 100) while the height is 199. The viewport's input is still all of
    // its stage at the pipe's scale (decision 16), which the planner gives bloom as the
    // input's scaled extent (decision 15).
    use apertrust_operations::bloom::Bloom;
    let instances = [Instance {
        operation: &Bloom,
        params: ParamValues::new(),
    }];
    let pipe = Pipe::Viewport {
        width: 200,
        height: 199,
    };
    let image = ImageContext::full(100, 200);
    let input = ImageDesc::rgba(100, 200);
    let plan = pipeline::plan(&instances, &image, input, pipe, &Limits::NONE).unwrap();
    let bloom = plan.operations.iter().find(|op| op.id == "bloom").unwrap();
    assert_eq!(bloom.scale, 1.0);
    let desc = bloom.output_desc;
    assert_eq!(
        (desc.width, desc.height, desc.extent, desc.scaled_extent),
        (100, 199, [100, 200], [100, 199])
    );
    let scratch: Vec<u64> = bloom.scratch.iter().map(|&i| plan.scratch[i]).collect();
    assert_eq!(scratch, [100 * 199 * 4; 2]);
}

/// The operations with whole-image passes that an export at a size without high quality
/// runs after its resample, with parameters under which they have them (decision 15).
fn after_the_resample() -> Vec<Instance<'static>> {
    use apertrust_operations::{bilat, bloom, colorequal, filmicrgb, soften, toneequal};
    let mut tone = toneequal::settings(&toneequal::ToneEqual.schema().defaults());
    tone.details = toneequal::Filter::Eigf;
    let filmic = filmicrgb::Settings {
        enable_highlight_reconstruction: true,
        ..filmicrgb::Settings::default()
    };
    let instances: [(&'static dyn Operation, ParamValues); 6] = [
        (&toneequal::ToneEqual, toneequal::values(&tone)),
        (&bloom::Bloom, ParamValues::new()),
        (&soften::Soften, ParamValues::new()),
        (&colorequal::ColorEqual, ParamValues::new()),
        (&bilat::Bilat, bilat::values(&bilat::Settings::default())),
        (&filmicrgb::FilmicRgb, filmicrgb::values(&filmic)),
    ];
    instances
        .into_iter()
        .map(|(operation, params)| Instance { operation, params })
        .collect()
}

#[test]
fn whole_image_passes_after_an_exports_resample_cover_its_output() {
    // The D5100's 4928 × 3264 at 640 without high quality: darktable's double floor makes
    // its output 639 × 423, where the scale in f32 would make it 640 × 423. Each
    // operation with whole-image passes after the resample covers that output, its stage
    // at the export's scale (decision 15): it plans the passes and scratch it plans for
    // that output as the whole image of an export at the export's scale. A tile past the
    // resample holds a part of it.
    let full = [4928, 3264];
    let scale = 640.0f64 / 4928.0;
    let output = full.map(|side| (scale * f64::from(side)).floor() as u32);
    assert_eq!(output, [639, 423]);
    assert_eq!(
        full.map(|side| (scale as f32 * side as f32).floor() as u32),
        [640, 423]
    );
    let resample = Resample {
        output,
        scale: scale as f32,
        finalscale: false,
    };
    let image = ImageContext::full(full[0], full[1]);
    let at_scale = ImageContext {
        scale: resample.scale,
        ..image.clone()
    };
    let input = ImageDesc::rgba(full[0], full[1]);
    for instance in after_the_resample() {
        let id = instance.operation.manifest().id;
        let instances = std::slice::from_ref(&instance);
        let plan = pipeline::plan(
            instances,
            &image,
            input,
            Pipe::Resampled(resample),
            &Limits::NONE,
        )
        .unwrap_or_else(|error| panic!("{id}: {error}"));
        let ids: Vec<&str> = plan.operations.iter().map(|op| op.id).collect();
        assert_eq!(ids, ["resample", id]);
        let resampled = plan.operations[0].output_desc;
        assert_eq!(
            (
                [resampled.width, resampled.height],
                resampled.origin,
                resampled.extent,
                resampled.scaled_extent
            ),
            (output, [0, 0], full, output),
            "{id}"
        );
        assert_eq!(at_scale.whole_image(&resampled), output, "{id}");
        let op = &plan.operations[1];
        assert_eq!(op.scale, resample.scale, "{id}");
        assert!(
            op.passes
                .iter()
                .any(|pass| pass.kind == PassKind::Reduction),
            "{id}: no whole-image pass"
        );
        let alone = pipeline::plan(
            instances,
            &at_scale,
            ImageDesc::rgba(output[0], output[1]),
            Pipe::Export,
            &Limits::NONE,
        )
        .unwrap();
        let reference = &alone.operations[0];
        assert_eq!(
            format!("{:?}", op.passes),
            format!("{:?}", reference.passes),
            "{id}'s passes"
        );
        let scratch = |plan: &pipeline::Plan, op: &pipeline::PlannedOperation| -> Vec<u64> {
            op.scratch.iter().map(|&i| plan.scratch[i]).collect()
        };
        assert_eq!(
            scratch(&plan, op),
            scratch(&alone, reference),
            "{id}'s scratch"
        );
    }

    // A tile at (320, 200) past the resample, from its output: bloom lays its lights out
    // as the whole output, 4 bytes a pixel.
    let bloom = Instance {
        operation: &apertrust_operations::bloom::Bloom,
        params: ParamValues::new(),
    };
    let resampled = pipeline::plan(&[], &image, input, Pipe::Resampled(resample), &Limits::NONE)
        .unwrap()
        .output()
        .1;
    let tile = Pipe::Tile {
        region: [320, 200, 64, 48],
        resample: Some(resample),
    };
    let plan = pipeline::plan(&[bloom], &image, resampled, tile, &Limits::NONE).unwrap();
    let ids: Vec<&str> = plan.operations.iter().map(|op| op.id).collect();
    assert_eq!(ids, ["region", "bloom"]);
    let part = plan.operations[0].output_desc;
    assert_eq!((part.origin, part.scaled_extent), ([320, 200], output));
    assert_eq!(at_scale.whole_image(&part), output);
    let lights = plan.scratch[plan.operations[1].scratch[0]];
    assert_eq!(lights, 639 * 423 * 4);
}

#[test]
fn an_export_at_a_size_resamples_right_after_haze_removal() {
    // Haze removal's estimate is the full-size export's, at full resolution (decision 15):
    // without high quality the resample comes right after it (decision 15), so it runs at a
    // scale of 1 over the image at full resolution, planned as in the full-size export, and
    // the tone equalizer after it at the export's scale. With high quality the resample
    // stays at finalscale's place, after both.
    let full = [4928, 3264];
    let image = ImageContext::full(full[0], full[1]);
    let input = ImageDesc::rgba(full[0], full[1]);
    let resample = Resample {
        output: [639, 423],
        scale: (640.0f64 / 4928.0) as f32,
        finalscale: false,
    };
    let instances = [
        Instance {
            operation: &apertrust_operations::toneequal::ToneEqual,
            params: ParamValues::new(),
        },
        Instance {
            operation: &apertrust_operations::hazeremoval::HazeRemoval,
            params: ParamValues::new(),
        },
    ];
    let plan = |pipe| pipeline::plan(&instances, &image, input, pipe, &Limits::NONE).unwrap();
    let ids =
        |plan: &pipeline::Plan| -> Vec<&str> { plan.operations.iter().map(|op| op.id).collect() };
    let at_size = plan(Pipe::Resampled(resample));
    assert_eq!(ids(&at_size), ["hazeremoval", "resample", "toneequal"]);
    assert_eq!(at_size.resample, Some(1..2));
    let haze = &at_size.operations[0];
    assert_eq!(haze.scale, 1.0);
    assert_eq!(
        (
            [haze.output_desc.width, haze.output_desc.height],
            haze.output_desc.scaled_extent
        ),
        (full, full)
    );
    let export = plan(Pipe::Export);
    assert_eq!(ids(&export), ["hazeremoval", "toneequal"]);
    let in_export = &export.operations[0];
    assert_eq!(
        format!("{:?} {:?}", haze.passes, haze.key),
        format!("{:?} {:?}", in_export.passes, in_export.key),
        "haze removal as in the full-size export"
    );
    let tone = &at_size.operations[2];
    assert_eq!(tone.scale, resample.scale);
    assert_eq!(tone.output_desc.scaled_extent, resample.output);

    let high_quality = plan(Pipe::Resampled(Resample {
        finalscale: true,
        ..resample
    }));
    assert_eq!(ids(&high_quality), ["hazeremoval", "toneequal", "resample"]);
    assert_eq!(high_quality.resample, Some(2..3));
}
