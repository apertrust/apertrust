// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2011-2026  darktable developers (ported from src/iop/soften.c)
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

//! Soften, ported from darktable (`src/iop/soften.c`): the Orton effect. Each
//! pixel's HSL saturation and lightness are scaled and clipped, the result blurred over
//! the whole image by darktable's box mean (`src/common/box_filters.cc`, shared in the
//! colour library) in whole-image passes, and mixed into the input by a per-pixel
//! `main`. The Soften module (`docs/modules/README.md`). See `docs/ports/soften.md`.
//!
//! It is in `OPERATIONS` with `default_enabled` off: an unedited photo does not run
//! it, and the session plans it only while a module's binding sets one of its
//! parameters (decision 28).

pub mod hsl;
mod setup;

use apertrust_plugin::{
    Domain, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation, Param,
    ParamValues, Prepared, Provenance, Schema, Slot, Stage, Value,
};
pub use setup::*;

/// Generated from `slang/soften.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "soften",
    wgsl: include_str!("../generated/soften.wgsl"),
    derivatives: true,
};

pub struct Soften;

static MANIFEST: Manifest = Manifest {
    id: "soften",
    version: 1,
    // darktable's v5.0 order: after grain (65), before splittoning (67) and vignette
    // (68); display-referred, in linear RGB.
    slot: Slot {
        stage: Stage::Display,
        order: 66.0,
    },
    domain: Domain::Display,
    // Soften works on layers.
    layers: true,
    differentiable: true,
    // The whole-image passes see the whole image and `main` reads only its own pixel
    // and their scratch, so a tile needs no border.
    radius: 0,
    // Planned only while a module sets one of its parameters (decision 28).
    default_enabled: false,
    provenance: Some(Provenance {
        darktable_module: "soften",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &[
            "src/iop/soften.c",
            "src/common/box_filters.cc",
            "src/common/colorspaces.h",
            "src/common/imagebuf.c",
        ],
    }),
};

/// darktable `dt_iop_soften_params_t` version 1, by its field names, with its
/// introspection's ranges and defaults (decision 21).
static SCHEMA: Schema = Schema {
    version: 1,
    params: &[
        Param {
            name: "size",
            kind: Kind::Float {
                default: 50.0,
                min: 0.0,
                max: 100.0,
            },
        },
        Param {
            name: "saturation",
            kind: Kind::Float {
                default: 100.0,
                min: 0.0,
                max: 100.0,
            },
        },
        Param {
            name: "brightness",
            kind: Kind::Float {
                default: 0.33,
                min: -2.0,
                max: 2.0,
            },
        },
        Param {
            name: "amount",
            kind: Kind::Float {
                default: 50.0,
                min: 0.0,
                max: 100.0,
            },
        },
    ],
};

/// The settings from resolved parameter values.
pub fn settings(params: &ParamValues) -> Settings {
    Settings {
        size: params.float("size"),
        saturation: params.float("saturation"),
        brightness: params.float("brightness"),
        amount: params.float("amount"),
    }
}

/// The parameter values of `settings`, as an edit or the golden harness sets them.
pub fn values(settings: &Settings) -> ParamValues {
    let mut values = ParamValues::new();
    values.set("size", Value::Float(settings.size));
    values.set("saturation", Value::Float(settings.saturation));
    values.set("brightness", Value::Float(settings.brightness));
    values.set("amount", Value::Float(settings.amount));
    values
}

/// darktable's `piece->iwidth` and `iheight` times `piece->iscale`, from which it
/// measures the largest radius: the pipe's input at full resolution, for a raw its
/// readout as decoded, before rawprepare's crop, as the tone equalizer's radius.
fn full(image: &ImageContext) -> [u32; 2] {
    image
        .camera
        .as_ref()
        .map_or([image.width, image.height], |camera| {
            [camera.width, camera.height]
        })
}

impl Operation for Soften {
    fn manifest(&self) -> &'static Manifest {
        &MANIFEST
    }

    fn schema(&self) -> &'static Schema {
        &SCHEMA
    }

    fn kernel(&self) -> &'static Kernel {
        &KERNEL
    }

    fn prepare(
        &self,
        params: &ParamValues,
        image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<Prepared, NotPorted> {
        // The scratch is laid out as the whole image at the pipe's scale (decision 16).
        prepare(
            &settings(params),
            input,
            image.whole_image(input),
            full(image),
            image.scale,
        )
    }

    /// `overexpose`, the one whole-image pass that reads the input, writes the softened
    /// image of the tile's own pixels at their place in the whole image
    /// (docs/ports/soften.md, Neighbourhood and scale).
    fn tiles_whole_image_passes(&self, _params: &ParamValues) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_defaults_are_darktables() {
        assert_eq!(settings(&SCHEMA.defaults()), Settings::default());
        let names: Vec<&str> = SCHEMA.params.iter().map(|p| p.name).collect();
        assert_eq!(names, ["size", "saturation", "brightness", "amount"]);
    }

    #[test]
    fn every_parameter_reaches_the_settings() {
        let base = settings(&SCHEMA.defaults());
        for param in SCHEMA.params {
            let Kind::Float { default, min, max } = param.kind else {
                unreachable!("{} has a kind the schema does not use", param.name)
            };
            let moved = if default < (min + max) / 2.0 {
                max
            } else {
                min
            };
            let mut values = ParamValues::new();
            values.set(param.name, Value::Float(moved));
            let changed = settings(&SCHEMA.resolve(&values).unwrap());
            assert_ne!(changed, base, "{} reaches no field", param.name);
        }
    }

    #[test]
    fn values_round_trip_through_the_schema() {
        let settings = Settings {
            size: 33.0,
            saturation: 100.0,
            brightness: 0.5,
            amount: 75.0,
        };
        let resolved = SCHEMA.resolve(&values(&settings)).unwrap();
        assert_eq!(super::settings(&resolved), settings);
    }

    #[test]
    fn prepare_uses_the_pipes_scale_and_the_input() {
        let params = SCHEMA.defaults();
        let input = ImageDesc::rgba(1954, 1301);
        let image = ImageContext {
            scale: 0.5,
            ..ImageContext::full(3908, 2602)
        };
        let ours = Soften.prepare(&params, &image, &input).unwrap();
        let direct = prepare(&settings(&params), &input, [1954, 1301], [3908, 2602], 0.5).unwrap();
        assert_eq!(ours, direct);
    }

    #[test]
    fn the_radius_takes_a_raws_readout() {
        // darktable's piece->iwidth is the pipe's input, the raw before its crop:
        // mire1.cr2's 3944 × 2622 for a developed image of 3908 × 2602.
        let camera = apertrust_plugin::Camera {
            make: String::new(),
            model: String::new(),
            alias: String::new(),
            width: 3944,
            height: 2622,
            cfa: None,
            black: apertrust_plugin::Pattern {
                width: 1,
                height: 1,
                values: vec![0.0],
            },
            white: 1.0,
            white_balance: None,
            xyz_to_camera: None,
            crop: [30, 18, 3908, 2602],
            declared: [30, 18, 3908, 2602],
            exposure_bias: 0.0,
            iso: 0.0,
            linear_response_limit: 1.0,
            lens_correction: None,
            orientation: 0,
        };
        let image = ImageContext {
            camera: Some(camera),
            ..ImageContext::full(3908, 2602)
        };
        assert_eq!(full(&image), [3944, 2622]);
        assert_eq!(full(&ImageContext::full(3908, 2602)), [3908, 2602]);
    }

    #[test]
    fn a_tile_lays_the_scratch_out_as_the_whole_image() {
        // A tile at (200, 160) of its stage, as the planner describes it.
        let params = SCHEMA.defaults();
        let tile = ImageDesc {
            origin: [200, 160],
            extent: [3908, 2602],
            scaled_extent: [3908, 2602],
            ..ImageDesc::rgba(100, 80)
        };
        let image = ImageContext::full(3908, 2602);
        let prepared = Soften.prepare(&params, &image, &tile).unwrap();
        assert_eq!(prepared.output, tile);
        assert_eq!(prepared.scratch[0].bytes, 3908 * 2602 * 4);
        assert!(Soften.tiles_whole_image_passes(&params));
    }

    #[test]
    fn the_operation_is_differentiable() {
        // That its kernel agrees is the operations' own check.
        assert!(Soften.manifest().differentiable);
    }
}
