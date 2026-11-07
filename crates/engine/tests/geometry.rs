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

//! Geometry (decision 16), with two operations of the tests' own that move pixels: a
//! window, which crops as crop will and, a translation, runs no pass, and a quarter
//! turn with a kernel of its own. The planner works regions backward through them,
//! refusing what it cannot place, and on a GPU adapter, hardware or software, an export
//! through them gives their maps' CPU restatement, a tiled export the untiled one bit for
//! bit, and the viewport the export downscaled. A plan with no geometry is planned as
//! before it, with its keys, and one without an export's resample as before its stage at
//! the pipe's scale (decision 15).

use std::fmt::Write as _;
use std::hash::{DefaultHasher, Hash, Hasher};

use apertrust_engine::compute::GpuImage;
use apertrust_engine::edit::{self, Edit};
use apertrust_engine::export::{Exported, Tiling, export, export_at};
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe, Pipeline, Plan, PlanError};
use apertrust_operations::OPERATIONS;
use apertrust_operations::bloom::Bloom;
use apertrust_operations::colorequal::ColorEqual;
use apertrust_operations::flip::Flip;
use apertrust_operations::soften::Soften;
use apertrust_operations::toneequal::{self, ToneEqual};
use apertrust_operations::vignette::Vignette;
use apertrust_plugin::{
    Camera, Domain, Extent, Format, Geometry, ImageContext, ImageDesc, Kernel, Manifest, NotPorted,
    Operation, OutputProfile, ParamValues, Pass, PassKind, Pattern, PipeKind, Prepared, Schema,
    Slot, Stage, Strips, Value,
};

static NO_PARAMS: Schema = Schema {
    version: 1,
    params: &[],
};

/// A test operation's manifest, after demosaic at `order` in camera RGB.
const fn manifest(id: &'static str, order: f32) -> Manifest {
    Manifest {
        id,
        version: 1,
        slot: Slot {
            stage: Stage::CameraRgb,
            order,
        },
        domain: Domain::Scene,
        layers: false,
        differentiable: false,
        radius: 0,
        default_enabled: false,
        provenance: None,
    }
}

static WINDOW_MANIFEST: Manifest = manifest("window", 24.5);
static TURN_MANIFEST: Manifest = manifest("turn", 16.0);

/// A window of its input at full resolution, x, y, width and height, as crop keeps
/// one: a translation of its input region, so it runs no pass. At crop's place,
/// camera RGB 24.5.
struct Window([u32; 4]);

/// The window's kernel, which nothing compiles, since it runs no pass.
static NO_KERNEL: Kernel = Kernel {
    name: "window",
    wgsl: "",
    derivatives: false,
};

impl Geometry for Window {
    fn extent(&self, input: [u32; 2]) -> [u32; 2] {
        let [x, y, width, height] = self.0;
        [width.min(input[0] - x), height.min(input[1] - y)]
    }

    /// The region moved by the window's corner at the pipe's scale, its size kept, and
    /// kept inside the input: in the spirit of darktable's crop, which moves the corner
    /// and clamps it, not a port of it.
    fn input_region(&self, input: &Extent, output: [u32; 4]) -> [u32; 4] {
        let [ox, oy, width, height] = output;
        let corner = |axis: usize| {
            (self.0[axis] as f32 * input.scaled[axis] as f32 / input.full[axis] as f32) as u32
        };
        [
            (ox + corner(0)).min(input.scaled[0] - width),
            (oy + corner(1)).min(input.scaled[1] - height),
            width,
            height,
        ]
    }

    fn forward(&self, _: [u32; 2], [x, y]: [f32; 2]) -> Option<[f32; 2]> {
        let [left, top, width, height] = self.0.map(|side| side as f32);
        let (x, y) = (x - left, y - top);
        ((0.0..=width).contains(&x) && (0.0..=height).contains(&y)).then_some([x, y])
    }

    fn backward(&self, _: [u32; 2], [x, y]: [f32; 2]) -> Option<[f32; 2]> {
        let [left, top, ..] = self.0.map(|side| side as f32);
        Some([x + left, y + top])
    }
}

impl Operation for Window {
    fn manifest(&self) -> &'static Manifest {
        &WINDOW_MANIFEST
    }

    fn schema(&self) -> &'static Schema {
        &NO_PARAMS
    }

    fn kernel(&self) -> &'static Kernel {
        &NO_KERNEL
    }

    fn prepare(
        &self,
        _: &ParamValues,
        _: &ImageContext,
        input: &ImageDesc,
    ) -> Result<Prepared, NotPorted> {
        Ok(Prepared {
            output: *input,
            passes: Vec::new(),
            scratch: Vec::new(),
        })
    }

    fn geometry(&self, _: &ParamValues, _: &ImageContext) -> Option<Box<dyn Geometry>> {
        Some(Box::new(Window(self.0)))
    }
}

/// A quarter turn clockwise: input pixel (x, y) of an image `h` tall lands at (h - 1 -
/// y, x). At orientation's place, camera RGB 16.
struct Turn;

impl Geometry for Turn {
    fn extent(&self, [width, height]: [u32; 2]) -> [u32; 2] {
        [height, width]
    }

    /// As darktable's flip: the region turned back, so the input region is exactly the
    /// output region's pixels.
    fn input_region(&self, input: &Extent, [x, y, width, height]: [u32; 4]) -> [u32; 4] {
        [y, input.scaled[1] - x - width, height, width]
    }

    fn forward(&self, [_, height]: [u32; 2], [x, y]: [f32; 2]) -> Option<[f32; 2]> {
        Some([height as f32 - y, x])
    }

    fn backward(&self, [_, height]: [u32; 2], [x, y]: [f32; 2]) -> Option<[f32; 2]> {
        Some([y, height as f32 - x])
    }
}

/// The turn's kernel: each output pixel from its input region turned back, as
/// darktable's flip turns its whole input region, so it needs no position.
const TURN_WGSL: &str = r#"
struct Header {
    width: u32, height: u32, origin_x: u32, origin_y: u32,
    full_width: u32, full_height: u32, scale: f32, repeat: u32,
    input_base: u32, output_base: u32, reserved0: u32, reserved1: u32,
}
struct Params { header: Header, input_width: u32, input_height: u32, pad0: u32, pad1: u32 }

@group(0) @binding(0) var<storage, read> input: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= params.header.width || id.y >= params.header.height) { return; }
    let turned = (params.input_height - 1u - id.x) * params.input_width + id.y;
    output[id.y * params.header.width + id.x] = input[turned];
}
"#;

static TURN_KERNEL: Kernel = Kernel {
    name: "turn",
    wgsl: TURN_WGSL,
    derivatives: false,
};

impl Operation for Turn {
    fn manifest(&self) -> &'static Manifest {
        &TURN_MANIFEST
    }

    fn schema(&self) -> &'static Schema {
        &NO_PARAMS
    }

    fn kernel(&self) -> &'static Kernel {
        &TURN_KERNEL
    }

    fn prepare(
        &self,
        _: &ParamValues,
        _: &ImageContext,
        input: &ImageDesc,
    ) -> Result<Prepared, NotPorted> {
        let params = [input.width, input.height, 0, 0]
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .collect();
        Ok(Prepared {
            output: ImageDesc {
                width: input.height,
                height: input.width,
                ..*input
            },
            passes: vec![Pass::per_pixel(params, Vec::new())],
            scratch: Vec::new(),
        })
    }

    fn geometry(&self, _: &ParamValues, _: &ImageContext) -> Option<Box<dyn Geometry>> {
        Some(Box::new(Turn))
    }
}

fn instance(operation: &dyn Operation) -> Instance<'_> {
    Instance {
        operation,
        params: ParamValues::new(),
    }
}

/// A raw `sensor` photosites across and down, its active area `crop` and its declared
/// image `declared`: a colour gradient with a highlight that clips.
struct Raw {
    cfa: Pattern<u32>,
    sensor: (u32, u32),
    crop: [u32; 4],
    declared: [u32; 4],
}

impl Raw {
    fn bayer(sensor: (u32, u32), crop: [u32; 4], declared: [u32; 4]) -> Self {
        Self {
            cfa: Pattern {
                width: 2,
                height: 2,
                values: vec![0, 1, 1, 2],
            },
            sensor,
            crop,
            declared,
        }
    }

    #[rustfmt::skip]
    fn x_trans(sensor: (u32, u32), crop: [u32; 4], declared: [u32; 4]) -> Self {
        Self {
            cfa: Pattern {
                width: 6,
                height: 6,
                values: vec![
                    1, 1, 0, 1, 1, 2,
                    1, 1, 2, 1, 1, 0,
                    2, 0, 1, 0, 2, 1,
                    1, 1, 2, 1, 1, 0,
                    1, 1, 0, 1, 1, 2,
                    0, 2, 1, 2, 0, 1,
                ],
            },
            sensor,
            crop,
            declared,
        }
    }

    fn values(&self) -> Vec<u16> {
        let (width, height) = self.sensor;
        let cfa = &self.cfa;
        (0..width * height)
            .map(|i| {
                let (x, y) = (i % width, i / width);
                let c = cfa.values[((y % cfa.height) * cfa.width + x % cfa.width) as usize];
                let (fx, fy) = (x as f32 / width as f32, y as f32 / height as f32);
                let r2 = (fx - 0.6).powi(2) + (fy - 0.4).powi(2);
                let level = 0.15 + 0.5 * fx + 0.1 * c as f32 + 0.2 * fy + 1.2 * (-r2 / 0.01).exp();
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

    fn sensor(&self) -> ImageDesc {
        ImageDesc::new(self.sensor.0, self.sensor.1, Format::Mosaic16)
    }

    fn upload(&self, gpu: &GpuContext) -> GpuImage {
        GpuImage::upload_mosaic(gpu, self.sensor.0, self.sensor.1, &self.values()).unwrap()
    }

    /// The default pipeline an unedited photo runs.
    fn instances(&self) -> Vec<Instance<'static>> {
        edit::instances(&Edit::default(), OPERATIONS, &[], &self.image())
            .all()
            .unwrap()
    }

    fn export(&self, gpu: &GpuContext, instances: &[Instance], tiling: Tiling) -> Exported {
        pollster::block_on(export(
            gpu,
            instances,
            &self.image(),
            self.upload(gpu),
            tiling,
        ))
        .unwrap()
    }
}

fn gpu() -> GpuContext {
    pollster::block_on(GpuContext::new(&GpuOptions::default()))
        .expect("a GPU adapter, hardware or software")
}

/// The 98 by 76 photo the regions are worked out on, turned to 76 by 98 and then
/// windowed at (7, 9) to 61 by 83.
fn photo() -> Raw {
    Raw::bayer((110, 86), [2, 2, 106, 82], [5, 4, 98, 76])
}

const WINDOW: Window = Window([7, 9, 61, 83]);

/// The default pipeline with the turn and the window.
fn turned_and_windowed(raw: &Raw) -> Vec<Instance<'static>> {
    let mut instances = raw.instances();
    instances.extend([instance(&Turn), instance(&WINDOW)]);
    instances
}

fn op<'a>(plan: &'a Plan, id: &str) -> &'a pipeline::PlannedOperation {
    plan.operations.iter().find(|op| op.id == id).unwrap()
}

#[test]
fn regions_run_backward_through_a_turn_and_a_window() {
    let raw = photo();
    let plan = |pipe| {
        pipeline::plan(
            &turned_and_windowed(&raw),
            &raw.image(),
            raw.sensor(),
            pipe,
            &Limits::NONE,
        )
        .unwrap()
    };
    let export = plan(Pipe::Export);
    let ids: Vec<&str> = export.operations.iter().map(|op| op.id).collect();
    assert_eq!(
        ids,
        [
            "rawprepare",
            "temperature",
            "highlights",
            "demosaic",
            "turn",
            "exposure",
            "window",
            "colorin",
            "channelmixerrgb",
            "agx",
            "colorout"
        ]
    );
    // The window at (7, 9) of the turned 76 by 98 image reads that region; the turn
    // reads it turned back, rows 8 to 68 of the developed image, which demosaic makes.
    let region = |op: &pipeline::PlannedOperation| op.output_desc.region();
    let demosaic = op(&export, "demosaic");
    assert_eq!(region(demosaic), [9, 8, 83, 61]);
    assert_eq!(demosaic.output_desc.sensor_origin, [5 + 9, 4 + 8]);
    assert_eq!(demosaic.extent, [98, 76]);
    for id in ["turn", "exposure"] {
        let op = op(&export, id);
        assert_eq!(region(op), [7, 9, 61, 83], "{id}");
        assert_eq!((op.origin, op.extent), ([7, 9], [76, 98]), "{id}'s header");
    }
    // A translation runs no pass: its output is its input's buffer.
    let window = op(&export, "window");
    assert!(window.passes.is_empty());
    assert_eq!(window.output, op(&export, "exposure").output);
    for id in ["window", "colorin", "colorout"] {
        let op = op(&export, id);
        assert_eq!(region(op), [0, 0, 61, 83], "{id}");
        assert_eq!((op.origin, op.extent), ([0, 0], [61, 83]), "{id}'s header");
    }
    assert_eq!(export.extent(), [61, 83]);
    // Buffers by region: none after the raw stage holds more than the turn's input.
    for work in export
        .work
        .iter()
        .filter(|work| work.format == Format::Rgba)
    {
        assert_eq!(work.width * work.height, 83 * 61, "{work}");
    }

    // A tile of the output reads its region back through both.
    let tile = {
        let highlights = op(&export, "highlights").output_desc;
        let after: Vec<Instance> = turned_and_windowed(&raw)
            .into_iter()
            .filter(|i| i.operation.manifest().slot.order > 4.0)
            .collect();
        let pipe = Pipe::Tile {
            region: [10, 20, 30, 40],
            resample: None,
        };
        pipeline::plan(&after, &raw.image(), highlights, pipe, &Limits::NONE).unwrap()
    };
    assert_eq!(region(op(&tile, "demosaic")), [29, 29, 40, 30]);
    assert_eq!(region(op(&tile, "exposure")), [17, 29, 30, 40]);
    assert_eq!(region(op(&tile, "colorout")), [10, 20, 30, 40]);

    // The viewport fits the windowed image, cutting the turn's input out of the
    // downscale.
    let viewport = plan(Pipe::Viewport {
        width: 40,
        height: 40,
    });
    let ids: Vec<&str> = viewport.operations.iter().map(|op| op.id).collect();
    assert_eq!(ids[3..7], ["demosaic", "downscale", "region", "turn"]);
    let (_, output) = viewport.output();
    assert_eq!((output.width, output.height), (29, 40));
    // At 40 / 83 the developed image is 47 by 37, and the window's corner at that
    // scale (3, 4).
    assert_eq!(
        op(&viewport, "downscale").output_desc.region(),
        [0, 0, 47, 37]
    );
    assert_eq!(region(op(&viewport, "turn")), [3, 4, 29, 40]);

    // Keys follow the window: moving it changes every key from demosaic on.
    let moved = {
        let window = Window([8, 9, 61, 83]);
        let mut instances = raw.instances();
        instances.extend([instance(&Turn), instance(&window)]);
        pipeline::plan(
            &instances,
            &raw.image(),
            raw.sensor(),
            Pipe::Export,
            &Limits::NONE,
        )
        .unwrap()
    };
    for (a, b) in export.operations.iter().zip(&moved.operations) {
        let upstream = a.output_desc.format.is_mosaic();
        assert_eq!(a.key == b.key, upstream, "{}", a.id);
    }
}

/// The quarter turn, but asking for row strips, which a geometry's passes never take.
struct StripedTurn;

impl Operation for StripedTurn {
    fn manifest(&self) -> &'static Manifest {
        &TURN_MANIFEST
    }

    fn schema(&self) -> &'static Schema {
        &NO_PARAMS
    }

    fn kernel(&self) -> &'static Kernel {
        &TURN_KERNEL
    }

    fn prepare(
        &self,
        params: &ParamValues,
        image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<Prepared, NotPorted> {
        let mut prepared = Turn.prepare(params, image, input)?;
        for pass in &mut prepared.passes {
            pass.strips = Some(Strips::PER_PIXEL);
        }
        Ok(prepared)
    }

    fn geometry(&self, params: &ParamValues, image: &ImageContext) -> Option<Box<dyn Geometry>> {
        Turn.geometry(params, image)
    }
}

#[test]
fn a_plan_with_geometry_refuses_what_it_cannot_place() {
    let image = ImageContext::full(37, 23);
    let refused = |instances: &[Instance], input: ImageDesc| match pipeline::plan(
        instances,
        &image,
        input,
        Pipe::Export,
        &Limits::NONE,
    ) {
        Err(PlanError::Geometry { operation, what }) => format!("{operation}: {what}"),
        other => panic!("{other:?}"),
    };
    assert_eq!(
        refused(&[instance(&StripedTurn)], ImageDesc::rgba(37, 23)),
        "turn: its passes take no row strips"
    );
    // An RGBA image described from a mosaic's description lies in no stage.
    let unplaced = ImageDesc {
        format: Format::Rgba,
        ..ImageDesc::new(37, 23, Format::Mosaic)
    };
    assert_eq!(
        refused(&[instance(&Turn)], unplaced),
        "the input: an RGBA image whose extent is not set"
    );
    // Whole-image passes cover their stage, which the tone equalizer's before crop's
    // window would have to cover only in part (decision 16).
    let window = Window([3, 5, 15, 12]);
    assert_eq!(
        refused(
            &[instance(&ToneEqual), instance(&window)],
            ImageDesc::rgba(37, 23)
        ),
        "toneequal: whole-image passes before a geometry that keeps part of their stage are \
         not planned yet"
    );
}

#[test]
fn a_viewport_too_large_to_bind_is_named_by_its_frame() {
    // A 100 by 80 window of the 752 by 500 photo, fitted at a half: the developed image
    // at that scale, 376 by 250, binds more than 512 KiB, though the frame is 50 by 40.
    let raw = Raw::bayer((767, 514), [2, 3, 762, 508], [5, 6, 752, 500]);
    let window = Window([0, 0, 100, 80]);
    let mut instances = raw.instances();
    instances.push(instance(&window));
    let limits = Limits {
        binding: 512 << 10,
        buffer: u64::MAX,
        offset_alignment: 256,
        budget: pipeline::BUDGET,
    };
    let pipe = Pipe::Viewport {
        width: 50,
        height: 40,
    };
    match pipeline::plan(&instances, &raw.image(), raw.sensor(), pipe, &limits) {
        Err(error @ PlanError::Viewport { .. }) => {
            assert!(
                error.to_string().contains(" 50×40 viewport's size"),
                "{error}"
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_tile_finds_its_region_in_an_input_that_starts_inside_its_stage() {
    // A tile's RGBA input, the 30 by 20 region at (10, 5) of a 60 by 40 stage, as one
    // after a split past a geometry would be. The turned 40 by 60 output's tile at (20,
    // 12), 10 by 16, reads the stage's 16 by 10 at (12, 10): the input's (2, 5).
    let input = ImageDesc {
        origin: [10, 5],
        extent: [60, 40],
        scaled_extent: [60, 40],
        ..ImageDesc::rgba(30, 20)
    };
    let pipe = Pipe::Tile {
        region: [20, 12, 10, 16],
        resample: None,
    };
    let image = ImageContext::full(60, 40);
    let plan = pipeline::plan(&[instance(&Turn)], &image, input, pipe, &Limits::NONE).unwrap();
    let ids: Vec<&str> = plan.operations.iter().map(|op| op.id).collect();
    assert_eq!(ids, ["region", "turn"]);
    let copy = &plan.operations[0];
    assert_eq!(copy.output_desc.region(), [12, 10, 16, 10]);
    let words: Vec<u32> = copy.passes[0]
        .params
        .chunks(4)
        .map(|word| u32::from_le_bytes(word.try_into().unwrap()))
        .collect();
    // The source's width, the corner the copy reads from, and four channels.
    assert_eq!(words, [30, 2, 5, 4]);
    assert_eq!(plan.operations[1].output_desc.region(), [20, 12, 10, 16]);
}

#[test]
fn an_export_at_a_size_with_an_operation_that_moves_pixels_is_refused() {
    // How each stage's size at the export's scale rounds through a geometry is still open
    // (docs/ports/finalscale.md, Open questions).
    let resample = pipeline::Resample {
        output: [30, 20],
        scale: 0.5,
        finalscale: true,
    };
    let image = ImageContext::full(60, 40);
    for pipe in [
        Pipe::Resampled(resample),
        Pipe::Tile {
            region: [0, 0, 10, 10],
            resample: Some(resample),
        },
    ] {
        let planned = pipeline::plan(
            &[instance(&Turn)],
            &image,
            ImageDesc::rgba(60, 40),
            pipe,
            &Limits::NONE,
        );
        assert!(
            matches!(
                planned,
                Err(PlanError::Geometry {
                    operation: "turn",
                    ..
                })
            ),
            "{pipe:?}: {:?}",
            planned.err()
        );
    }
}

/// An RGBA image whose every pixel tells where it came from.
fn coordinates(width: u32, height: u32) -> Vec<[f32; 4]> {
    (0..height)
        .flat_map(|y| (0..width).map(move |x| [x as f32, y as f32, (x * y) as f32, 1.0]))
        .collect()
}

#[test]
fn an_export_through_a_turn_and_a_window_is_their_maps_restated() {
    let gpu = gpu();
    // Sides that are not multiples of the 16-pixel workgroup.
    let (width, height) = (37, 23);
    let source = coordinates(width, height);
    let image = ImageContext::full(width, height);
    let window = Window([3, 5, 15, 30]);
    let instances = [instance(&window), instance(&Turn)];
    let plan = pipeline::plan(
        &instances,
        &image,
        ImageDesc::rgba(width, height),
        Pipe::Export,
        &Limits::NONE,
    )
    .unwrap();
    let input = GpuImage::upload(&gpu, width, height, &source).unwrap();
    let mut pipeline = pollster::block_on(Pipeline::new(&gpu, plan, input)).unwrap();
    let timings = pipeline.render(&gpu).unwrap();
    // The input is cut to the region the turn reads, and the window runs no pass.
    let labels: Vec<&str> = timings.passes.iter().map(|(label, _)| *label).collect();
    assert_eq!(labels, ["region", "turn"]);
    let output = pipeline.output();
    assert_eq!((output.width(), output.height()), (15, 30));
    let pixels = output.download(&gpu).unwrap();

    // Each output pixel's centre mapped back through both, where the pixel it lands in
    // is the one the output holds.
    let turned = [height, width];
    for (i, pixel) in pixels.iter().enumerate() {
        let (x, y) = ((i % 15) as f32 + 0.5, (i / 15) as f32 + 0.5);
        let point = window.backward(turned, [x, y]).unwrap();
        let [sx, sy] = Turn.backward([width, height], point).unwrap();
        let expected = source[(sy as u32 * width + sx as u32) as usize];
        assert_eq!(*pixel, expected, "output pixel ({x}, {y})");
        // And forward again.
        let back = Turn.forward([width, height], [sx, sy]).unwrap();
        assert_eq!(window.forward(turned, back), Some([x, y]));
    }
}

#[test]
fn a_tiled_export_through_geometry_equals_the_whole_one() {
    let gpu = gpu();
    for raw in [
        photo(),
        Raw::x_trans((120, 96), [1, 3, 116, 90], [4, 5, 108, 84]),
    ] {
        let instances = turned_and_windowed(&raw);
        let whole = raw.export(&gpu, &instances, Tiling::Whole);
        assert_eq!((whole.desc.width, whole.desc.height), (61, 83));
        let whole = whole.pixels;
        for size in [[24, 20], [37, 29], [64, 1000], [1000, 1000]] {
            let exported = raw.export(&gpu, &instances, Tiling::Tiles(size));
            // Tiles of the output image, not of the developed image.
            let tiles = 61_u32.div_ceil(size[0]) * 83_u32.div_ceil(size[1]);
            assert_eq!(exported.tiles, tiles as usize, "{size:?}");
            assert_eq!((exported.desc.width, exported.desc.height), (61, 83));
            let tiled = exported.pixels;
            if let Some(i) = tiled
                .iter()
                .zip(&whole)
                .position(|(a, b)| a.map(f32::to_bits) != b.map(f32::to_bits))
            {
                panic!(
                    "tiles of {size:?}: pixel ({}, {}) is {:?} tiled, {:?} whole",
                    i % 61,
                    i / 61,
                    tiled[i],
                    whole[i]
                );
            }
        }
    }
}

#[test]
fn a_tiled_export_through_a_turn_and_the_tone_equalizer_equals_the_whole_one() {
    // The default pipeline, the turn and the tone equalizer: each tile's plan runs the
    // turn's pass ahead of the tone equalizer's first round, as orientation will on every
    // raw (decision 16).
    let gpu = gpu();
    let raw = photo();
    let mut instances = raw.instances();
    instances.extend([
        instance(&Turn),
        Instance {
            operation: &ToneEqual,
            params: tone_equalizer(),
        },
    ]);
    let whole = raw.export(&gpu, &instances, Tiling::Whole);
    assert_eq!((whole.desc.width, whole.desc.height), (76, 98));
    for size in [[24, 20], [37, 29]] {
        let tiled = raw.export(&gpu, &instances, Tiling::Tiles(size));
        assert_eq!(tiled.rounds, 2, "{size:?}");
        if let Some(i) = first_difference(&tiled.pixels, &whole.pixels) {
            panic!(
                "tiles of {size:?}: pixel ({}, {}) is {:?} tiled, {:?} whole",
                i % 76,
                i / 76,
                tiled.pixels[i],
                whole.pixels[i]
            );
        }
    }
}

/// `pixels`, `source` in size, box-downscaled by the viewport's footprints: output pixel o
/// of `output` covers source pixels o × source / output up to (o + 1) × source /
/// output.
fn downscale(pixels: &[[f32; 4]], source: [u32; 2], output: [u32; 2]) -> Vec<[f32; 4]> {
    let span = |o: u32, axis: usize| {
        let start = o * source[axis] / output[axis];
        start..(o + 1) * source[axis] / output[axis]
    };
    (0..output[1])
        .flat_map(|oy| (0..output[0]).map(move |ox| (ox, oy)))
        .map(|(ox, oy)| {
            let (mut sum, mut n) = ([0.0f32; 4], 0.0f32);
            for y in span(oy, 1) {
                for x in span(ox, 0) {
                    let pixel = pixels[(y * source[0] + x) as usize];
                    for c in 0..4 {
                        sum[c] += pixel[c];
                    }
                    n += 1.0;
                }
            }
            sum.map(|s| s / n)
        })
        .collect()
}

#[test]
fn the_viewport_of_a_turned_photo_is_its_export_downscaled() {
    let gpu = gpu();
    // An 80 by 60 photo turned to 60 by 80, windowed at (8, 12) to 40 by 52, and fitted
    // to half its size, so the downscale's footprints are 2 by 2 whichever way they are
    // turned. Up to exposure, which is affine, as after the downscale the rest would
    // differ from the export downscaled as any nonlinear operation does (decision 12).
    let raw = Raw::bayer((90, 70), [2, 2, 86, 66], [5, 4, 80, 60]);
    let window = Window([8, 12, 40, 52]);
    let mut instances: Vec<Instance> = raw
        .instances()
        .into_iter()
        .filter(|i| i.operation.manifest().slot.order <= 21.0)
        .collect();
    instances.extend([instance(&Turn), instance(&window)]);

    let exported = raw.export(&gpu, &instances, Tiling::Whole).pixels;
    let expected = downscale(&exported, [40, 52], [20, 26]);

    let pipe = Pipe::Viewport {
        width: 20,
        height: 26,
    };
    let plan = pipeline::plan(
        &instances,
        &raw.image(),
        raw.sensor(),
        pipe,
        &Limits::of(&gpu),
    )
    .unwrap();
    assert!(plan.operations.iter().any(|op| op.id == "region"));
    let mut pipeline = pollster::block_on(Pipeline::new(&gpu, plan, raw.upload(&gpu))).unwrap();
    pipeline.render(&gpu).unwrap();
    let output = pipeline.output();
    assert_eq!((output.width(), output.height()), (20, 26));
    let frame = output.download(&gpu).unwrap();
    // The same sums, but in another order: a few units in the last place apart, 4 at
    // most on the M4.
    for (i, (a, b)) in frame.iter().zip(&expected).enumerate() {
        for c in 0..4 {
            let tolerance = 1e-6 * a[c].abs().max(b[c].abs()).max(1.0);
            assert!(
                (a[c] - b[c]).abs() <= tolerance,
                "pixel ({}, {}): {a:?} in the viewport, {b:?} downscaled",
                i % 20,
                i / 20
            );
        }
    }
}

/// An RGBA image `size` across and down: gradients with a light that blooms, every value
/// a multiple of 1/256, so the viewport's 2 by 2 sums are exact in any order.
fn scene([width, height]: [u32; 2]) -> Vec<[f32; 4]> {
    let level = |k: u32| (k % 256) as f32 / 256.0;
    (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let (dx, dy) = (x.abs_diff(width * 2 / 3), y.abs_diff(height / 3));
            if dx * dx + dy * dy < 30 {
                [1.5, 1.25, 1.0, 1.0]
            } else {
                [
                    level(5 * x + 3 * y),
                    level(2 * x + 7 * y + 40),
                    level(3 * x + y + 90),
                    1.0,
                ]
            }
        })
        .collect()
}

/// `pixels`, `size` across and down, turned as the turn's kernel turns them: output pixel
/// (x, y) is input pixel (y, height - 1 - x).
fn turned(pixels: &[[f32; 4]], [width, height]: [u32; 2]) -> Vec<[f32; 4]> {
    (0..width)
        .flat_map(|y| (0..height).map(move |x| pixels[((height - 1 - x) * width + y) as usize]))
        .collect()
}

/// The part of `pixels`, `width` across, that a window at full resolution keeps.
fn windowed(pixels: &[[f32; 4]], width: u32, [x, y, w, h]: [u32; 4]) -> Vec<[f32; 4]> {
    (y..y + h)
        .flat_map(|row| (x..x + w).map(move |column| pixels[(row * width + column) as usize]))
        .collect()
}

/// The tone equalizer at test 0131's bands on EIGF, so that it changes the image: at its
/// defaults' zero bands it is an identity.
fn tone_equalizer() -> ParamValues {
    let mut tone = toneequal::settings(&ToneEqual.schema().defaults());
    tone.bands = [
        -1.283, -0.788, 0.445, 1.148, 1.321, 0.994, 0.211, -0.449, -0.755,
    ];
    toneequal::values(&tone)
}

/// Where `a` and `b` differ in any bit, if anywhere.
fn first_difference(a: &[[f32; 4]], b: &[[f32; 4]]) -> Option<usize> {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .position(|(a, b)| a.map(f32::to_bits) != b.map(f32::to_bits))
}

#[test]
fn whole_image_passes_after_a_turn_or_a_window_see_that_image() {
    // Each operation with whole-image passes after a geometry renders what it renders on
    // the turned or windowed image without one: its passes cover its stage's extent at
    // the pipe's scale (decision 16). The tone equalizer comes before crop's window.
    let gpu = gpu();
    let size = [64, 44];
    let source = scene(size);
    let image = ImageContext::full(size[0], size[1]);
    let upload = |[width, height]: [u32; 2], pixels: &[[f32; 4]]| {
        GpuImage::upload(&gpu, width, height, pixels).unwrap()
    };
    let window = Window([6, 4, 40, 36]);
    let operations: [Instance; 4] = [
        instance(&Bloom),
        instance(&Soften),
        instance(&ColorEqual),
        Instance {
            operation: &ToneEqual,
            params: tone_equalizer(),
        },
    ];
    for operation in &operations {
        let id = operation.operation.manifest().id;
        let mut geometries = vec![(instance(&Turn), turned(&source, size), [44, 64])];
        if id != "toneequal" {
            let moved = windowed(&source, size[0], window.0);
            geometries.push((instance(&window), moved, [40, 36]));
        }
        for (geometry, moved, [width, height]) in geometries {
            let name = geometry.operation.manifest().id;
            let with = [geometry, operation.clone()];
            let without = std::slice::from_ref(operation);
            let exported = |instances: &[Instance], input, tiling| {
                pollster::block_on(export(&gpu, instances, &image, input, tiling)).unwrap()
            };
            let expected = exported(without, upload([width, height], &moved), Tiling::Whole);
            let change = expected
                .pixels
                .iter()
                .zip(&moved)
                .map(|(a, b)| (0..3).map(|c| (a[c] - b[c]).abs()).fold(0.0, f32::max))
                .fold(0.0, f32::max);
            assert!(change > 1e-3, "{id} changes the image by {change}");

            // An export, whole and in tiles, its whole-image passes in a first round.
            for tiling in [Tiling::Whole, Tiling::Tiles([24, 20])] {
                let through = exported(&with, upload(size, &source), tiling);
                let rounds = if tiling == Tiling::Whole { 1 } else { 2 };
                assert_eq!(through.rounds, rounds, "{id} after the {name}");
                if let Some(i) = first_difference(&through.pixels, &expected.pixels) {
                    panic!(
                        "{id} after the {name}, {tiling:?}: pixel ({}, {}) is {:?}, {:?} \
                         without geometry",
                        i as u32 % width,
                        i as u32 / width,
                        through.pixels[i],
                        expected.pixels[i]
                    );
                }
            }

            // The viewport fitted at a half: the same 2 by 2 sums, in another order.
            let pipe = Pipe::Viewport {
                width: width / 2,
                height: height / 2,
            };
            let frame = |instances: &[Instance], input: GpuImage| {
                let limits = Limits::of(&gpu);
                let plan = pipeline::plan(instances, &image, input.desc(), pipe, &limits).unwrap();
                let mut pipeline = pollster::block_on(Pipeline::new(&gpu, plan, input)).unwrap();
                pipeline.render(&gpu).unwrap();
                pipeline.output().download(&gpu).unwrap()
            };
            let viewport = frame(&with, upload(size, &source));
            let expected = frame(without, upload([width, height], &moved));
            assert_eq!(viewport.len(), (width / 2 * height / 2) as usize);
            for (i, (a, b)) in viewport.iter().zip(&expected).enumerate() {
                for c in 0..4 {
                    let tolerance = 1e-6 * a[c].abs().max(b[c].abs()).max(1.0);
                    assert!(
                        (a[c] - b[c]).abs() <= tolerance,
                        "{id} after the {name}: pixel ({}, {}) is {a:?} in the viewport, \
                         {b:?} without geometry",
                        i as u32 % (width / 2),
                        i as u32 / (width / 2)
                    );
                }
            }
        }
    }
}

#[test]
fn a_quarter_turn_keeps_the_edit_cache() {
    // The 98 by 76 photo turned, in a 40 by 40 viewport: the downscale makes it 40 by 31,
    // the turn 31 by 40, which the edit cache holds by its bytes (decision 16).
    let gpu = gpu();
    let raw = photo();
    let pipe = Pipe::Viewport {
        width: 40,
        height: 40,
    };
    let plan = |exposure: f32| {
        let mut instances = raw.instances();
        for instance in &mut instances {
            if instance.operation.manifest().id == "exposure" {
                instance.params.set("exposure", Value::Float(exposure));
            }
        }
        instances.push(instance(&Turn));
        let limits = Limits::of(&gpu);
        pipeline::plan(&instances, &raw.image(), raw.sensor(), pipe, &limits).unwrap()
    };
    let first = plan(0.0);
    let edit = first.edit_cache.unwrap();
    assert_eq!((edit.width, edit.height), (40, 31));
    let turned = op(&first, "turn").output_desc;
    assert_eq!((turned.width, turned.height), (31, 40));
    let mut pipeline = pollster::block_on(Pipeline::new(&gpu, first, raw.upload(&gpu))).unwrap();
    pipeline.render(&gpu).unwrap();
    // The first change starts after the downscale, a cache point, and keeps exposure's
    // input, the turned image; the next starts at exposure.
    for (exposure, starts_at) in [(0.5, "turn"), (0.7, "exposure")] {
        pipeline = pollster::block_on(pipeline.update(&gpu, plan(exposure))).unwrap();
        let timings = pipeline.render(&gpu).unwrap();
        assert_eq!(timings.passes[0].0, starts_at, "exposure {exposure}");
        let frame = pipeline.output().download(&gpu).unwrap();
        let mut fresh =
            pollster::block_on(Pipeline::new(&gpu, plan(exposure), raw.upload(&gpu))).unwrap();
        fresh.render(&gpu).unwrap();
        let full = fresh.output().download(&gpu).unwrap();
        assert_eq!(first_difference(&frame, &full), None, "exposure {exposure}");
    }
}

/// The default pipeline an unedited photo runs, orientation at `choice` of its schema:
/// darktable's value `choice - 1`, 0 auto.
fn oriented(raw: &Raw, choice: u32) -> Vec<Instance<'static>> {
    let mut instances = raw.instances();
    let flip = instances
        .iter_mut()
        .find(|instance| instance.operation.manifest().id == "flip")
        .expect("orientation runs on every photo");
    flip.params.set("orientation", Value::Choice(choice));
    instances
}

/// darktable's `dt_imageio_flip_buffers`, restated: `pixels`, `width` by `height`, with
/// the columns flipped, the rows flipped and then the axes swapped as `orientation`'s bits
/// say, 2, 1 and 4.
fn flipped(pixels: &[[f32; 4]], [width, height]: [u32; 2], orientation: u32) -> Vec<[f32; 4]> {
    let swap = orientation & 4 != 0;
    let out_width = if swap { height } else { width };
    let mut out = vec![[0.0; 4]; pixels.len()];
    for j in 0..height {
        for i in 0..width {
            let x = if orientation & 2 != 0 {
                width - 1 - i
            } else {
                i
            };
            let y = if orientation & 1 != 0 {
                height - 1 - j
            } else {
                j
            };
            let (x, y) = if swap { (y, x) } else { (x, y) };
            out[(y * out_width + x) as usize] = pixels[(j * width + i) as usize];
        }
    }
    out
}

#[test]
fn orientation_turns_an_export_as_darktable_turns_its_buffer() {
    // Every orientation, through the default pipeline: the export the right way up with
    // its buffer flipped as darktable's flip does, bit for bit, every operation after it
    // per pixel; and tiles of its output the whole export, bit for bit.
    let gpu = gpu();
    for raw in [
        photo(),
        Raw::x_trans((120, 96), [1, 3, 116, 90], [4, 5, 108, 84]),
    ] {
        let [.., width, height] = raw.declared;
        let level = raw.export(&gpu, &oriented(&raw, 1), Tiling::Whole);
        assert_eq!([level.desc.width, level.desc.height], [width, height]);
        for bits in 1..8 {
            let instances = oriented(&raw, bits + 1);
            let whole = raw.export(&gpu, &instances, Tiling::Whole);
            let size = if bits & 4 != 0 {
                [height, width]
            } else {
                [width, height]
            };
            assert_eq!([whole.desc.width, whole.desc.height], size, "{bits}");
            let expected = flipped(&level.pixels, [width, height], bits);
            assert_eq!(first_difference(&whole.pixels, &expected), None, "{bits}");
            for tiles in [[24, 20], [37, 29]] {
                let tiled = raw.export(&gpu, &instances, Tiling::Tiles(tiles));
                assert!(tiled.tiles > 1, "{bits}, {tiles:?}");
                let difference = first_difference(&tiled.pixels, &whole.pixels);
                assert_eq!(difference, None, "{bits}, tiles of {tiles:?}");
            }
        }
    }
}

#[test]
fn an_export_at_a_size_resamples_a_turned_photo() {
    // A raw its EXIF turns, exported at a size, as darktable exports it: without high
    // quality the resample runs after demosaic in the photo's own stage, its sides the
    // output's turned back, and orientation turns its result, so the export is the
    // turned export of the photo the right way up at the swapped size, bit for bit; with
    // it, at finalscale's place after orientation, in the turned image. Either way its
    // tiles are the whole export, bit for bit.
    let gpu = gpu();
    for raw in [
        photo(),
        Raw::x_trans((120, 96), [1, 3, 116, 90], [4, 5, 108, 84]),
    ] {
        let [.., width, height] = raw.declared;
        let at = |output: [u32; 2], finalscale| pipeline::Resample {
            output,
            scale: 0.5,
            finalscale,
        };
        let exported = |image: &ImageContext, resample, tiling| {
            let instances = edit::instances(&Edit::default(), OPERATIONS, &[], image)
                .all()
                .unwrap();
            pollster::block_on(export_at(
                &gpu,
                &instances,
                image,
                raw.upload(&gpu),
                tiling,
                Some(resample),
            ))
            .unwrap()
        };
        let level_size = [width / 2, height / 2];
        for bits in 1..8u8 {
            let mut turned = raw.image();
            turned.camera.as_mut().unwrap().orientation = bits;
            let size = if bits & 4 != 0 {
                [level_size[1], level_size[0]]
            } else {
                level_size
            };
            for finalscale in [false, true] {
                let whole = exported(&turned, at(size, finalscale), Tiling::Whole);
                assert_eq!([whole.desc.width, whole.desc.height], size, "{bits}");
                if !finalscale {
                    let level = exported(&raw.image(), at(level_size, false), Tiling::Whole);
                    let expected = flipped(&level.pixels, level_size, u32::from(bits));
                    let difference = first_difference(&whole.pixels, &expected);
                    assert_eq!(difference, None, "orientation {bits}");
                }
                for tiles in [[24, 20], [17, 13]] {
                    let tiled = exported(&turned, at(size, finalscale), Tiling::Tiles(tiles));
                    assert!(tiled.tiles > 1, "{bits}, {tiles:?}");
                    let difference = first_difference(&tiled.pixels, &whole.pixels);
                    assert_eq!(
                        difference, None,
                        "orientation {bits}, finalscale {finalscale}, tiles of {tiles:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn haze_removal_before_orientation_tiles_as_it_runs_whole() {
    // Haze removal (camera RGB 14) builds its estimate in a first round of tiles of its
    // own stage, the photo before orientation (16) turns it: a turned raw's tiled export,
    // at full size and at a size, is its whole export, bit for bit.
    use apertrust_operations::hazeremoval::{self, HazeRemoval};
    let gpu = gpu();
    let raw = photo();
    let [.., width, height] = raw.declared;
    for bits in 1..8u8 {
        let mut turned = raw.image();
        turned.camera.as_mut().unwrap().orientation = bits;
        let mut instances = edit::instances(&Edit::default(), OPERATIONS, &[], &turned)
            .all()
            .unwrap();
        instances.push(Instance {
            operation: &HazeRemoval,
            params: hazeremoval::values(&hazeremoval::Settings {
                strength: 0.6,
                distance: 0.5,
            }),
        });
        instances.sort_by(|a, b| {
            let (a, b) = (a.operation.manifest(), b.operation.manifest());
            a.slot.pipeline_order(&b.slot)
        });
        let half = [width / 2, height / 2];
        let output = if bits & 4 != 0 {
            [half[1], half[0]]
        } else {
            half
        };
        for resample in [
            None,
            Some(pipeline::Resample {
                output,
                scale: 0.5,
                finalscale: false,
            }),
        ] {
            let exported = |tiling| {
                pollster::block_on(export_at(
                    &gpu,
                    &instances,
                    &turned,
                    raw.upload(&gpu),
                    tiling,
                    resample,
                ))
                .unwrap()
            };
            let whole = exported(Tiling::Whole);
            for tiles in [[24, 20], [37, 29]] {
                let tiled = exported(Tiling::Tiles(tiles));
                assert_eq!(tiled.rounds, 2);
                let difference = first_difference(&tiled.pixels, &whole.pixels);
                assert_eq!(difference, None, "{bits}, {resample:?}, tiles of {tiles:?}");
            }
        }
    }
}

#[test]
fn a_photo_the_right_way_up_plans_without_orientation() {
    // At no rotation orientation runs no pass, so the planner leaves it out, as
    // darktable's `commit_params` disables its piece; a raw its EXIF turns plans it.
    let gpu = gpu();
    let raw = photo();
    let limits = Limits::of(&gpu);
    let ids = |image: &ImageContext, pipe| {
        let plan = pipeline::plan(&raw.instances(), image, raw.sensor(), pipe, &limits).unwrap();
        let output = plan.output().1;
        let ids: Vec<&str> = plan.operations.iter().map(|op| op.id).collect();
        (ids, [output.width, output.height])
    };
    let viewport = Pipe::Viewport {
        width: 49,
        height: 49,
    };
    for pipe in [Pipe::Export, viewport] {
        let (level, size) = ids(&raw.image(), pipe);
        assert!(!level.contains(&"flip"), "{level:?}");
        let mut turned = raw.image();
        turned.camera.as_mut().unwrap().orientation = 6;
        let (ids, turned_size) = ids(&turned, pipe);
        assert!(ids.contains(&"flip"), "{ids:?}");
        assert_eq!(turned_size, [size[1], size[0]]);
    }
}

#[test]
fn a_vignette_after_orientation_frames_the_turned_image() {
    // darktable's vignette places itself by its input's size at full resolution, its
    // `buf_in`, which after orientation is the turned image: the pass header's full size
    // is each operation's stage (decision 16). So vignetting after every orientation
    // renders what it renders on the image flipped beforehand, bit for bit, whole and in
    // tiles; off centre and not round, it is not the vignetted image flipped.
    let gpu = gpu();
    let size = [64, 44];
    let source = scene(size);
    let image = ImageContext::full(size[0], size[1]);
    let upload = |[width, height]: [u32; 2], pixels: &[[f32; 4]]| {
        GpuImage::upload(&gpu, width, height, pixels).unwrap()
    };
    let exported = |instances: &[Instance], input, tiling| {
        pollster::block_on(export(&gpu, instances, &image, input, tiling)).unwrap()
    };
    let mut params = ParamValues::new();
    for (name, value) in [
        ("brightness", -0.5),
        ("saturation", -0.3),
        ("center_x", 0.3),
        ("center_y", -0.2),
        ("whratio", 1.3),
    ] {
        params.set(name, Value::Float(value));
    }
    params.set("autoratio", Value::Bool(false));
    let vignette = Instance {
        operation: &Vignette,
        params,
    };
    let flip = |bits: u32| {
        let mut params = ParamValues::new();
        params.set("orientation", Value::Choice(bits + 1));
        Instance {
            operation: &Flip,
            params,
        }
    };
    let alone = exported(
        std::slice::from_ref(&vignette),
        upload(size, &source),
        Tiling::Whole,
    );
    for bits in 1..8 {
        let turned_size = if bits & 4 != 0 {
            [size[1], size[0]]
        } else {
            size
        };
        let moved = flipped(&source, size, bits);
        let expected = exported(
            std::slice::from_ref(&vignette),
            upload(turned_size, &moved),
            Tiling::Whole,
        );
        let with = [flip(bits), vignette.clone()];
        for tiling in [Tiling::Whole, Tiling::Tiles([24, 20])] {
            let through = exported(&with, upload(size, &source), tiling);
            assert_eq!([through.desc.width, through.desc.height], turned_size);
            let difference = first_difference(&through.pixels, &expected.pixels);
            assert_eq!(difference, None, "orientation {bits}, {tiling:?}");
        }
        let vignetted_then_flipped = flipped(&alone.pixels, size, bits);
        assert!(
            first_difference(&expected.pixels, &vignetted_then_flipped).is_some(),
            "orientation {bits}"
        );
    }
}

/// FNV-1a, 64 bits: a hash that no Rust release changes.
fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// What a plan dispatches and binds, but for the bytes `prepare` gave its passes, which
/// may differ between platforms' maths libraries in their last bits, and the keys,
/// which hash them.
fn structure(plan: &Plan) -> String {
    let desc = |d: &ImageDesc| {
        format!(
            "{}x{} {:?} {:?}",
            d.width, d.height, d.format, d.sensor_origin
        )
    };
    // A tile's pipe as it printed before an export at a size gave tiles a resample
    // (decision 56), so a tile at full size keeps its fingerprint.
    let pipe = match plan.pipe {
        Pipe::Tile {
            region,
            resample: None,
        } => format!("Tile {{ region: {region:?} }}"),
        pipe => format!("{pipe:?}"),
    };
    let mut text = format!("{pipe} {}\n", desc(&plan.input));
    for op in &plan.operations {
        writeln!(
            text,
            "{} {:?}->{:?} {} {:?} {} {:?} {:?}",
            op.id,
            op.input,
            op.output,
            desc(&op.output_desc),
            op.scratch,
            op.scale.to_bits(),
            op.origin,
            op.whole_image.map(|whole| whole.passes),
        )
        .unwrap();
        for (pass, dispatches) in op.passes.iter().zip(&op.dispatches) {
            let tables: Vec<usize> = pass.tables.iter().map(Vec::len).collect();
            writeln!(
                text,
                " {} {:?} {:?} {:?} {:?} {} {tables:?} {dispatches:?}",
                pass.entry_point,
                pass.kind,
                pass.size,
                pass.strips,
                pass.bindings,
                pass.params.len(),
            )
            .unwrap();
        }
    }
    let work: Vec<String> = plan.work.iter().map(desc).collect();
    let used: Vec<String> = plan.work_used.iter().map(desc).collect();
    writeln!(
        text,
        "{work:?} {used:?} {:?} {:?} {:?} {:?} {:?}",
        plan.scratch,
        plan.scratch_used,
        plan.cached,
        plan.edit_cache.as_ref().map(desc),
        plan.tiled
    )
    .unwrap();
    text
}

/// The plans of the default pipeline without geometry: an export whole and one of its
/// tiles, and the viewport, whole and, on a GPU that binds little, in tiles; an X-Trans
/// export; and an export whose raw stage runs in row strips.
fn plans_without_geometry() -> Vec<(&'static str, Plan)> {
    let small = photo();
    let x_trans = Raw::x_trans((120, 96), [1, 3, 116, 90], [4, 5, 108, 84]);
    let large = Raw::bayer((767, 514), [2, 3, 762, 508], [5, 6, 752, 500]);
    let binding_little = Limits {
        binding: 512 << 10,
        buffer: u64::MAX,
        offset_alignment: 256,
        budget: pipeline::BUDGET,
    };
    let plan = |raw: &Raw, pipe: Pipe, limits: &Limits| {
        pipeline::plan(&raw.instances(), &raw.image(), raw.sensor(), pipe, limits).unwrap()
    };
    let export = plan(&small, Pipe::Export, &Limits::NONE);
    // A tile as an export plans it: demosaic on, from the raw stage's output.
    let highlights = export
        .operations
        .iter()
        .position(|op| op.id == "highlights")
        .unwrap();
    let source = export.operations[highlights].output_desc;
    let tiled: Vec<Instance> = small
        .instances()
        .into_iter()
        .filter(|instance| {
            let slot = instance.operation.manifest().slot;
            slot.stage != Stage::Raw || slot.order > 4.0
        })
        .collect();
    let tile = pipeline::plan(
        &tiled,
        &small.image(),
        source,
        Pipe::Tile {
            region: [37, 29, 37, 29],
            resample: None,
        },
        &Limits::NONE,
    )
    .unwrap();
    let viewport = Pipe::Viewport {
        width: 40,
        height: 40,
    };
    vec![
        ("export", export),
        ("tile", tile),
        ("viewport", plan(&small, viewport, &Limits::NONE)),
        ("tiled viewport", plan(&large, viewport, &binding_little)),
        (
            "x-trans export",
            plan(&x_trans, Pipe::Export, &Limits::NONE),
        ),
        (
            "export in row strips",
            plan(&large, Pipe::Export, &binding_little),
        ),
    ]
}

/// `hash_desc` in the pipeline, as it was before geometry: an image's size, format,
/// place on the sensor and colour bookkeeping, never its region of its stage.
fn hash_desc(desc: &ImageDesc, hasher: &mut DefaultHasher) {
    let format = match desc.format {
        Format::Mosaic16 => 0u8,
        Format::Mosaic => 1,
        Format::Rgba => 2,
    };
    (
        desc.width,
        desc.height,
        format,
        desc.sensor_origin,
        desc.late_correction,
    )
        .hash(hasher);
    for value in desc.maximum.iter().chain(&desc.white_balance) {
        value.to_bits().hash(hasher);
    }
}

/// Each operation's key and whole-image key as the pipeline made them before geometry:
/// the previous key, its ID, version, scale and origin, its output image and its passes'
/// prepared bytes; and for the whole-image passes, their input's key and their own.
fn keys_before_geometry(plan: &Plan) -> Vec<(u64, Option<u64>)> {
    let version = |id: &str| {
        OPERATIONS
            .iter()
            .find(|op| op.manifest().id == id)
            .map_or(1, |op| op.manifest().version)
    };
    let mut hasher = DefaultHasher::new();
    hash_desc(&plan.input, &mut hasher);
    let mut previous = hasher.finish();
    let mut keys = Vec::new();
    for op in &plan.operations {
        let mut hasher = DefaultHasher::new();
        (
            previous,
            op.id,
            version(op.id),
            op.scale.to_bits(),
            op.origin,
        )
            .hash(&mut hasher);
        hash_desc(&op.output_desc, &mut hasher);
        for pass in &op.passes {
            (pass.entry_point, pass.size, &pass.params, &pass.tables).hash(&mut hasher);
        }
        let key = hasher.finish();
        let whole_image = op.whole_image.map(|whole| {
            let scratch: Vec<u64> = op.scratch.iter().map(|&i| plan.scratch[i]).collect();
            let mut hasher = DefaultHasher::new();
            (
                previous,
                op.id,
                version(op.id),
                op.scale.to_bits(),
                whole.passes,
                scratch,
            )
                .hash(&mut hasher);
            hash_desc(&op.output_desc, &mut hasher);
            for pass in &op.passes[..whole.passes] {
                let Pass {
                    entry_point,
                    kind,
                    params,
                    tables,
                    bindings,
                    size,
                    strips: _,
                } = pass;
                (entry_point, kind, params, tables, bindings, size).hash(&mut hasher);
            }
            hasher.finish()
        });
        keys.push((key, whole_image));
        previous = key;
    }
    keys
}

/// Every plan with no geometry is planned as the fingerprints below record: the same
/// passes, dispatches, windows, origins, scales and buffers, the same pass headers, every
/// operation's stage the developed image, and the same keys, so geometry changes nothing
/// for them. A change that means to change these plans records the fingerprints again.
#[test]
fn a_plan_without_geometry_is_planned_as_before() {
    let fingerprints = [
        ("export", 0xc9e6_115b_a8d1_af64),
        ("tile", 0x0733_d235_0e05_e469),
        ("viewport", 0xb46f_d868_667d_9043),
        ("tiled viewport", 0xa9e9_4a2f_50ec_bf27),
        ("x-trans export", 0xeee7_2b61_5376_6fa2),
        ("export in row strips", 0xd732_b02f_4f66_4a23),
    ];
    let plans = plans_without_geometry();
    assert_eq!(plans.len(), fingerprints.len());
    assert!(plans[3].1.tiled.is_some(), "the viewport in tiles");
    let in_strips = |plan: &Plan| {
        let strips = |op: &pipeline::PlannedOperation| op.dispatches.iter().any(|d| d.len() > 1);
        plan.operations.iter().any(strips)
    };
    assert!(
        in_strips(&plans[5].1),
        "the export's raw stage in row strips"
    );
    for ((name, plan), (expected_name, expected)) in plans.iter().zip(fingerprints) {
        assert_eq!(*name, expected_name);
        assert_eq!(fnv(&structure(plan)), expected, "{name}'s plan");
        let developed = [plan.image.width, plan.image.height];
        for op in &plan.operations {
            assert_eq!(op.extent, developed, "{name}: {}'s header", op.id);
        }
        let keys: Vec<(u64, Option<u64>)> = plan
            .operations
            .iter()
            .map(|op| (op.key, op.whole_image.map(|whole| whole.key)))
            .collect();
        assert_eq!(keys, keys_before_geometry(plan), "{name}'s keys");
    }
    // And the plans do have whole-image keys to compare: highlight reconstruction's.
    assert!(
        plans[2]
            .1
            .operations
            .iter()
            .any(|op| op.whole_image.is_some())
    );
}

/// Runs as `inner` does, but its `prepare` is given its input's stage at the pipe's scale
/// as `ImageContext::whole_image` gave it before decision 15, by the pipe's kind: in an
/// export its `extent`, and in the viewport, never tiled after its downscale, the input
/// itself.
struct ByPipeKind(&'static dyn Operation);

impl Operation for ByPipeKind {
    fn manifest(&self) -> &'static Manifest {
        self.0.manifest()
    }
    fn schema(&self) -> &'static Schema {
        self.0.schema()
    }
    fn kernel(&self) -> &'static Kernel {
        self.0.kernel()
    }
    fn defaults(&self, image: &ImageContext) -> ParamValues {
        self.0.defaults(image)
    }
    fn prepare(
        &self,
        params: &ParamValues,
        image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<Prepared, NotPorted> {
        let scaled_extent = match image.pipe {
            PipeKind::Export => input.extent,
            PipeKind::Viewport => [input.width, input.height],
        };
        let input = ImageDesc {
            scaled_extent,
            ..*input
        };
        self.0.prepare(params, image, &input)
    }
    fn output_profile(&self, params: &ParamValues) -> Option<OutputProfile> {
        self.0.output_profile(params)
    }
    fn tiles_whole_image_passes(&self, params: &ParamValues) -> bool {
        self.0.tiles_whole_image_passes(params)
    }
    fn whole_image_at_full_resolution(&self, params: &ParamValues) -> bool {
        self.0.whole_image_at_full_resolution(params)
    }
}

/// Every operation with whole-image passes, with parameters under which it has them.
fn whole_image_operations() -> Vec<(&'static dyn Operation, ParamValues)> {
    use apertrust_operations::{bilat, filmicrgb, hazeremoval};
    let filmic = filmicrgb::Settings {
        enable_highlight_reconstruction: true,
        ..filmicrgb::Settings::default()
    };
    vec![
        (&ToneEqual, tone_equalizer()),
        (&Bloom, ParamValues::new()),
        (&Soften, ParamValues::new()),
        (&ColorEqual, ParamValues::new()),
        (&hazeremoval::HazeRemoval, ParamValues::new()),
        (&bilat::Bilat, bilat::values(&bilat::Settings::default())),
        (&filmicrgb::FilmicRgb, filmicrgb::values(&filmic)),
    ]
}

/// Everything a plan is but its operations' kernels, whose sources would drown a
/// difference: each operation's images, buffers, passes with their bytes, dispatches,
/// scale, origin, stage and keys.
fn planned(plan: &Plan) -> String {
    let mut text = format!(
        "{:?} {:?} {:?} {:?} {:?} {:?}\n",
        plan.work, plan.work_used, plan.scratch, plan.cached, plan.edit_cache, plan.tiled
    );
    for op in &plan.operations {
        writeln!(
            text,
            "{} {:?}->{:?} {:?} {:?} {:?} {} {:?} {:?} {:x} {:?} {:?}",
            op.id,
            op.input,
            op.output,
            op.output_desc,
            op.scratch,
            op.dispatches,
            op.scale,
            op.origin,
            op.extent,
            op.key,
            op.whole_image,
            op.full_resolution,
        )
        .unwrap();
        for pass in &op.passes {
            writeln!(text, " {pass:?}").unwrap();
        }
    }
    text
}

/// Every plan without a resample is planned as before decision 15 gave every RGBA image
/// its stage at the pipe's scale: whole-image passes get the stage the pipe's kind gave
/// them, so each operation with them plans the same passes, bytes, scratch, dispatches
/// and keys as when its `prepare` is given the stage by the pipe's kind, in an export and
/// one of its tiles, the viewport whole and in tiles, and after a quarter turn in all
/// three. With `a_plan_without_geometry_is_planned_as_before`, which holds the default
/// pipeline's plans to their fingerprints on the code before geometry, every plan without
/// a resample keeps its bytes and its keys.
#[test]
fn a_plan_without_a_resample_is_planned_as_before() {
    let small = photo();
    let large = Raw::bayer((767, 514), [2, 3, 762, 508], [5, 6, 752, 500]);
    let binding_little = Limits {
        binding: 512 << 10,
        buffer: u64::MAX,
        offset_alignment: 256,
        budget: pipeline::BUDGET,
    };
    let viewport = Pipe::Viewport {
        width: 40,
        height: 40,
    };
    // A tile as an export plans it: demosaic on, from the raw stage's output.
    let tile = Pipe::Tile {
        region: [37, 29, 37, 29],
        resample: None,
    };
    let export = pipeline::plan(
        &small.instances(),
        &small.image(),
        small.sensor(),
        Pipe::Export,
        &Limits::NONE,
    )
    .unwrap();
    let highlights = op(&export, "highlights").output_desc;
    let after_highlights = |instance: &Instance| {
        let slot = instance.operation.manifest().slot;
        slot.stage != Stage::Raw || slot.order > 4.0
    };
    // The turned plans start from an RGBA image, as geometry's GPU tests do.
    let (rgba, rgba_input) = (ImageContext::full(64, 44), ImageDesc::rgba(64, 44));
    let turned_tile = Pipe::Tile {
        region: [4, 8, 20, 30],
        resample: None,
    };
    let turned_viewport = Pipe::Viewport {
        width: 32,
        height: 32,
    };
    for (operation, params) in whole_image_operations() {
        let id = operation.manifest().id;
        let by_kind = ByPipeKind(operation);
        // The operation, or the operation given its stage by the pipe's kind.
        let whole_image = |old: bool| Instance {
            operation: if old { &by_kind } else { operation },
            params: params.clone(),
        };
        let plan = |instances: &[Instance], image: &ImageContext, input, pipe, limits| {
            pipeline::plan(instances, image, input, pipe, limits)
                .unwrap_or_else(|error| panic!("{id}: {error}"))
        };
        let plans = |old: bool| {
            let with = |raw: &Raw| {
                let mut instances = raw.instances();
                instances.push(whole_image(old));
                instances
            };
            let tiled: Vec<Instance> = with(&small).into_iter().filter(after_highlights).collect();
            let turned = [instance(&Turn), whole_image(old)];
            let (image, sensor) = (small.image(), small.sensor());
            [
                (
                    "export",
                    plan(&with(&small), &image, sensor, Pipe::Export, &Limits::NONE),
                ),
                (
                    "tile",
                    plan(&tiled, &image, highlights, tile, &Limits::NONE),
                ),
                (
                    "viewport",
                    plan(&with(&small), &image, sensor, viewport, &Limits::NONE),
                ),
                (
                    "tiled viewport",
                    plan(
                        &with(&large),
                        &large.image(),
                        large.sensor(),
                        viewport,
                        &binding_little,
                    ),
                ),
                (
                    "turned export",
                    plan(&turned, &rgba, rgba_input, Pipe::Export, &Limits::NONE),
                ),
                (
                    "turned tile",
                    plan(&turned, &rgba, rgba_input, turned_tile, &Limits::NONE),
                ),
                (
                    "turned viewport",
                    plan(&turned, &rgba, rgba_input, turned_viewport, &Limits::NONE),
                ),
            ]
        };
        let new = plans(false);
        assert!(new[3].1.tiled.is_some(), "the viewport in tiles");
        for ((name, new), (_, old)) in new.iter().zip(plans(true)) {
            let passes = new
                .operations
                .iter()
                .filter(|op| op.id == id)
                .flat_map(|op| &op.passes)
                .any(|pass| pass.kind == PassKind::Reduction);
            assert!(passes, "{id} in the {name}: no whole-image pass");
            let (new, old) = (planned(new), planned(&old));
            if let Some((a, b)) = new.lines().zip(old.lines()).find(|(a, b)| a != b) {
                panic!("{id} in the {name}:\n{a}\nby the pipe's kind:\n{b}");
            }
            assert_eq!(
                new.lines().count(),
                old.lines().count(),
                "{id} in the {name}"
            );
        }
    }
}

/// The plans of exports at a size without haze removal, the tone equalizer and bloom
/// after the default pipeline's operations: whole at 0.5 and 1/3 without high quality,
/// at 0.5 with it (and haze removal), above 1 and a pixel short at a scale of 1; a tile
/// at 0.5 from the raw stage's output; a tile past the resample, from its output; with
/// high quality, a tile from demosaic's RGBA output, before finalscale's place; and a
/// pixel short of 1, a tile past the copy, from its output.
fn plans_at_a_size() -> Vec<(&'static str, Plan)> {
    use apertrust_operations::hazeremoval::{self, HazeRemoval};
    let raw = photo();
    let [.., width, height] = raw.declared;
    let at = |scale: f64, finalscale: bool| pipeline::Resample {
        output: [width, height].map(|side| (scale * f64::from(side)).floor() as u32),
        scale: scale as f32,
        finalscale,
    };
    let mut instances = raw.instances();
    instances.extend([
        Instance {
            operation: &ToneEqual,
            params: tone_equalizer(),
        },
        Instance {
            operation: &Bloom,
            params: ParamValues::new(),
        },
    ]);
    let plan = |instances: &[Instance], input: ImageDesc, pipe: Pipe| {
        pipeline::plan(instances, &raw.image(), input, pipe, &Limits::NONE).unwrap()
    };
    let export =
        |instances: &[Instance], resample| plan(instances, raw.sensor(), Pipe::Resampled(resample));
    let mut with_haze = instances.clone();
    with_haze.push(Instance {
        operation: &HazeRemoval,
        params: hazeremoval::values(&hazeremoval::Settings {
            strength: 0.9,
            distance: 0.8,
        }),
    });
    let half = at(0.5, false);
    let exported = export(&instances, half);
    let highlights = op(&exported, "highlights").output_desc;
    let resampled = op(&exported, "resample").output_desc;
    let (high_quality, short) = (at(0.5, true), at(0.999_999_99, false));
    let demosaiced = op(&export(&instances, high_quality), "demosaic").output_desc;
    let cut = op(&export(&instances, short), "region").output_desc;
    // The instances after the raw stage's operation at `order`.
    let after = |order: f32| -> Vec<Instance> {
        instances
            .iter()
            .filter(|instance| {
                let slot = instance.operation.manifest().slot;
                slot.stage != Stage::Raw || slot.order > order
            })
            .cloned()
            .collect()
    };
    let tile = |region| Pipe::Tile {
        region,
        resample: Some(half),
    };
    vec![
        ("export at 0.5", exported.clone()),
        ("export at 1/3", export(&instances, at(1.0 / 3.0, false))),
        (
            "export at 0.5, high quality",
            export(&with_haze, at(0.5, true)),
        ),
        ("export at 2", export(&instances, at(2.0, true))),
        (
            "export a pixel short of 1",
            export(&instances, at(0.999_999_99, false)),
        ),
        (
            "tile at 0.5",
            plan(&after(4.0), highlights, tile([10, 8, 24, 20])),
        ),
        (
            "tile past the resample",
            plan(&after(f32::INFINITY), resampled, tile([10, 8, 24, 20])),
        ),
        (
            "tile from an RGBA input, high quality",
            plan(
                &after(8.0),
                demosaiced,
                Pipe::Tile {
                    region: [10, 8, 24, 20],
                    resample: Some(high_quality),
                },
            ),
        ),
        (
            "tile past the copy a pixel short of 1",
            plan(
                &after(8.0),
                cut,
                Pipe::Tile {
                    region: [37, 29, 37, 29],
                    resample: Some(short),
                },
            ),
        ),
    ]
}

/// Every plan at a size without haze removal, or with high quality resampling, is planned
/// as the fingerprints below record: the same passes, dispatches, windows, origins, scales
/// and buffers, and the same keys, so the resample's place after haze removal (decision
/// 15) changes nothing for them. A change that means to change these plans records the
/// fingerprints again.
#[test]
fn a_plan_at_a_size_without_haze_removal_is_planned_as_before() {
    let fingerprints = [
        ("export at 0.5", 0xa3ca_7ebe_1a09_5431),
        ("export at 1/3", 0x566b_ed12_aa20_625b),
        ("export at 0.5, high quality", 0x56e5_e711_7977_8597),
        ("export at 2", 0x5fc9_134f_19a9_6aa6),
        ("export a pixel short of 1", 0x0aaf_2e7f_6882_20ca),
        ("tile at 0.5", 0x8aaa_fdc6_88c2_adf0),
        ("tile past the resample", 0xc57d_03ed_33a4_b2d5),
        (
            "tile from an RGBA input, high quality",
            0xaca7_9ea5_52bc_31c7,
        ),
        (
            "tile past the copy a pixel short of 1",
            0xb2a6_fcac_ff70_98ef,
        ),
    ];
    let plans = plans_at_a_size();
    assert_eq!(plans.len(), fingerprints.len());
    for ((name, plan), (expected_name, expected)) in plans.iter().zip(fingerprints) {
        assert_eq!(*name, expected_name);
        assert_eq!(fnv(&structure(plan)), expected, "{name}'s plan");
        let keys: Vec<(u64, Option<u64>)> = plan
            .operations
            .iter()
            .map(|op| (op.key, op.whole_image.map(|whole| whole.key)))
            .collect();
        assert_eq!(keys, keys_before_geometry(plan), "{name}'s keys");
    }
}
