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

//! Planning under a GPU's limits, without a GPU (decision 14): an export and a viewport
//! of the Sony 7R V's 61 MP raw and the Fujifilm X-T5's 40 MP X-Trans raw fit lavapipe,
//! which binds at most 128 MiB at once, the raw stage in row strips and the rest in
//! tiles; on WARP and the M4 the viewport stays whole. The 7R V
//! stopped there before: "a 9568×6376 image needs 244022272 bytes, more than the GPU
//! allows".

use apertrust_engine::edit::{self, Edit};
use apertrust_engine::export::BUDGET;
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe, Plan};
use apertrust_modules::MODULES;
use apertrust_operations::OPERATIONS;
use apertrust_plugin::{Camera, Format, ImageContext, ImageDesc, ParamValues, Pattern};

/// lavapipe's limits: 128 MiB in one binding, buffers up to 2 GiB as wgpu caps them on
/// Mesa, and WebGPU's largest offset alignment.
const LAVAPIPE: Limits = Limits {
    binding: 128 << 20,
    buffer: (1 << 31) - 1,
    offset_alignment: 256,
    budget: BUDGET,
};

/// WARP's: 2047 MiB in one binding and one buffer.
const WARP: Limits = Limits {
    binding: (1 << 31) - 1,
    buffer: (1 << 31) - 1,
    offset_alignment: 16,
    budget: BUDGET,
};

/// The M4's: everything in one binding.
const M4: Limits = Limits {
    binding: 4095 << 20,
    buffer: 4095 << 20,
    offset_alignment: 32,
    budget: BUDGET,
};

/// The viewport `apertrust-cli preview` renders by default.
const VIEWPORT: Pipe = Pipe::Viewport {
    width: 2560,
    height: 1600,
};

/// A raw's geometry as rawler decodes it: the readout, the active area and the declared
/// image, with the colour pattern.
struct Geometry {
    cfa: Pattern<u32>,
    readout: (u32, u32),
    crop: [u32; 4],
    declared: [u32; 4],
}

impl Geometry {
    /// The Sony 7R V's `7RM5-LosslessCompressedLarge.ARW`.
    fn sony_7r_v() -> Self {
        Self {
            cfa: Pattern {
                width: 2,
                height: 2,
                values: vec![0, 1, 1, 2],
            },
            readout: (9728, 6656),
            crop: [0, 0, 9568, 6376],
            declared: [32, 20, 9504, 6336],
        }
    }

    /// The Fujifilm X-T5's `DSCF0021.RAF`.
    fn fujifilm_x_t5() -> Self {
        Self {
            cfa: Pattern {
                width: 6,
                height: 6,
                #[rustfmt::skip]
                values: vec![
                    1, 1, 0, 1, 1, 2,
                    1, 1, 2, 1, 1, 0,
                    2, 0, 1, 0, 2, 1,
                    1, 1, 2, 1, 1, 0,
                    1, 1, 0, 1, 1, 2,
                    0, 2, 1, 2, 0, 1,
                ],
            },
            readout: (7872, 5196),
            crop: [0, 5, 7752, 5184],
            declared: [12, 21, 7728, 5152],
        }
    }

    fn image(&self) -> ImageContext {
        let (width, height) = self.readout;
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
                    values: vec![512.0],
                },
                white: 15360.0,
                white_balance: Some(vec![2.48, 1.0, 1.74]),
                xyz_to_camera: Some([
                    [0.82, -0.2976, -0.0719],
                    [-0.4296, 1.2053, 0.2532],
                    [-0.0429, 0.1282, 0.5774],
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
        ImageDesc::new(self.readout.0, self.readout.1, Format::Mosaic16)
    }
}

/// The default pipeline: what an empty edit renders.
fn defaults(image: &ImageContext) -> Vec<Instance<'static>> {
    edit::instances(&Edit::default(), OPERATIONS, MODULES, image)
        .all()
        .unwrap()
}

/// The operations of `ids`, at their defaults.
fn named(ids: &[&str]) -> Vec<Instance<'static>> {
    OPERATIONS
        .iter()
        .filter(|op| ids.contains(&op.manifest().id))
        .map(|&operation| Instance {
            operation,
            params: ParamValues::new(),
        })
        .collect()
}

/// How many dispatches each pass of the plan runs in, by operation and entry point.
fn strips(plan: &Plan) -> Vec<(&str, &str, usize)> {
    plan.operations
        .iter()
        .flat_map(|op| {
            op.passes
                .iter()
                .zip(&op.dispatches)
                .map(move |(pass, d)| (op.id, pass.entry_point, d.len()))
        })
        .collect()
}

/// The export as `apertrust-engine::export` plans it on lavapipe: the whole image does
/// not fit; the raw stage does, in row strips; and tiles of 2048 do, copying from windows
/// of the raw stage's output, where tiles of 4096 would bind more than 128 MiB.
fn fits_lavapipe(geometry: Geometry) {
    let image = geometry.image();
    let whole = pipeline::plan(
        &defaults(&image),
        &image,
        geometry.sensor(),
        Pipe::Export,
        &LAVAPIPE,
    )
    .unwrap();
    assert!(!whole.fits(&LAVAPIPE));
    // On a GPU that binds anything, nothing runs in strips.
    let anything = pipeline::plan(
        &defaults(&image),
        &image,
        geometry.sensor(),
        Pipe::Export,
        &Limits::NONE,
    )
    .unwrap();
    assert!(strips(&anything).iter().all(|&(_, _, n)| n == 1));

    let raw_stage = pipeline::plan(
        &named(&["rawprepare", "temperature", "highlights"]),
        &image,
        geometry.sensor(),
        Pipe::Export,
        &LAVAPIPE,
    )
    .unwrap();
    assert!(raw_stage.fits(&LAVAPIPE), "{:?}", strips(&raw_stage));
    // Every pass that reads the float mosaic, 4 bytes a photosite, runs in strips, and
    // highlights' dilation of its masks in bands of rows as its mask does; only its
    // chroma, one workgroup, runs whole.
    for (id, entry_point, n) in strips(&raw_stage) {
        let whole_only = entry_point == "opposed_chroma";
        assert_eq!(n > 1, !whole_only, "{id} {entry_point}: {n}");
    }

    let source = raw_stage.output().1;
    let later = named(&[
        "demosaic",
        "exposure",
        "colorin",
        "channelmixerrgb",
        "agx",
        "colorout",
    ]);
    let [.., width, height] = geometry.declared;
    let tile = |region| {
        let pipe = Pipe::Tile {
            region,
            resample: None,
        };
        pipeline::plan(&later, &image, source, pipe, &LAVAPIPE).unwrap()
    };
    // The first tile, an inner one and the last, narrower and shorter.
    for region in [
        [0, 0, 2048, 2048],
        [2048, 2048, 2048, 2048],
        [width - 1200, height - 1000, 1200, 1000],
    ] {
        let plan = tile(region);
        assert!(plan.fits(&LAVAPIPE), "{region:?}");
        // The copy binds only the rows the tile needs, not all of the raw stage's output.
        let copy = &plan.operations[0];
        assert_eq!(copy.id, "region");
        assert!(copy.dispatches[0].iter().all(|d| d.input.is_some()));
    }
    assert!(!tile([2048, 2048, 4096, 4096]).fits(&LAVAPIPE));
}

#[test]
fn a_61_mp_bayer_export_fits_lavapipe() {
    fits_lavapipe(Geometry::sony_7r_v());
}

#[test]
fn a_40_mp_x_trans_export_fits_lavapipe() {
    fits_lavapipe(Geometry::fujifilm_x_t5());
}

/// The viewport on lavapipe: the raw stage in row strips, then demosaic and the
/// downscale in tiles of 2048 at most, then the rest at the viewport's size. WARP and
/// the M4 bind demosaic's whole image, within the budget, so their viewport stays whole.
fn viewport_fits_lavapipe(geometry: Geometry, tiles: usize) {
    let image = geometry.image();
    let plan = |limits: &Limits| {
        pipeline::plan(
            &defaults(&image),
            &image,
            geometry.sensor(),
            VIEWPORT,
            limits,
        )
        .unwrap()
    };
    let tiled = plan(&LAVAPIPE);
    assert!(tiled.fits(&LAVAPIPE), "{:?}", strips(&tiled));
    let range = tiled.tiled.clone().expect("tiles on lavapipe");
    let tile_ops = &tiled.operations[range.clone()];
    let downscales = tile_ops.iter().filter(|op| op.id == "downscale");
    assert_eq!(downscales.clone().count(), tiles);
    // Tiles of 2048, each up to a footprint larger where its edges fall, demosaic's part
    // of the image exactly that; the copy adds demosaic's border of 12 on each side and
    // up to a colour pattern's alignment.
    let viewport = downscales.clone().next_back().unwrap().output_desc;
    let [.., width, height] = geometry.declared;
    let bound = [
        2048 + width.div_ceil(viewport.width),
        2048 + height.div_ceil(viewport.height),
    ];
    for op in tile_ops.iter().filter(|op| op.id != "downscale") {
        let border = if op.id == "region" { 2 * 12 + 5 } else { 0 };
        let desc = op.output_desc;
        assert!(
            desc.width <= bound[0] + border && desc.height <= bound[1] + border,
            "{} {desc}",
            op.id
        );
    }
    let ids: Vec<_> = tiled.operations[range.end..]
        .iter()
        .map(|op| op.id)
        .collect();
    assert_eq!(
        ids,
        ["exposure", "colorin", "channelmixerrgb", "agx", "colorout"]
    );
    for limits in [WARP, M4] {
        let whole = plan(&limits);
        // All its buffers together take more than the budget, as today; only demosaic's
        // image, the cache point, counts against it here.
        assert!(whole.tiled.is_none() && whole.binds_within(&limits));
        assert!(strips(&whole).iter().all(|&(_, _, n)| n == 1));
    }
}

#[test]
fn a_61_mp_bayer_viewport_fits_lavapipe() {
    viewport_fits_lavapipe(Geometry::sony_7r_v(), 20);
}

#[test]
fn a_40_mp_x_trans_viewport_fits_lavapipe() {
    viewport_fits_lavapipe(Geometry::fujifilm_x_t5(), 12);
}
