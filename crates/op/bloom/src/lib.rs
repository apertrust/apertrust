// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/iop/bloom.c)
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

//! Bloom, ported from darktable (`src/iop/bloom.c`): the Lab lightness scaled and
//! thresholded into lights, blurred over the whole image by darktable's box mean
//! (`src/common/box_filters.cc`, shared in the colour library) in whole-image passes,
//! and screen-blended onto the lightness by a per-pixel `main`. The Bloom module
//! (`docs/modules/README.md`). See `docs/ports/bloom.md`.
//!
//! It is in `OPERATIONS` with `default_enabled` off: an unedited photo does not run
//! it, and the session plans it only while a module's binding sets one of its
//! parameters (decision 28).

mod setup;

use apertrust_color::WorkingProfile;
use apertrust_plugin::{
    Domain, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation, Param,
    ParamValues, Prepared, Provenance, Schema, Slot, Stage, Value,
};
pub use setup::*;

/// Generated from `slang/bloom.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "bloom",
    wgsl: include_str!("../generated/bloom.wgsl"),
    derivatives: true,
};

pub struct Bloom;

static MANIFEST: Manifest = Manifest {
    id: "bloom",
    version: 1,
    // darktable's v5.0 order: after colorzones (60), before colorize (62) and grain
    // (65); display-referred, in Lab.
    slot: Slot {
        stage: Stage::Display,
        order: 61.0,
    },
    domain: Domain::Display,
    // Bloom works on layers.
    layers: true,
    differentiable: true,
    // The whole-image passes see the whole image and `main` reads only its own pixel
    // and their scratch, so a tile needs no border.
    radius: 0,
    // Planned only while a module sets one of its parameters (decision 28).
    default_enabled: false,
    provenance: Some(Provenance {
        darktable_module: "bloom",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &[
            "src/iop/bloom.c",
            "src/common/box_filters.cc",
            "src/common/iop_profile.c",
            "src/common/colorspaces_inline_conversions.h",
        ],
    }),
};

/// darktable `dt_iop_bloom_params_t` version 1, by its field names, with its
/// introspection's ranges and defaults (decision 21).
static SCHEMA: Schema = Schema {
    version: 1,
    params: &[
        Param {
            name: "size",
            kind: Kind::Float {
                default: 20.0,
                min: 0.0,
                max: 100.0,
            },
        },
        Param {
            name: "threshold",
            kind: Kind::Float {
                default: 90.0,
                min: 0.0,
                max: 100.0,
            },
        },
        Param {
            name: "strength",
            kind: Kind::Float {
                default: 25.0,
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
        threshold: params.float("threshold"),
        strength: params.float("strength"),
    }
}

/// The parameter values of `settings`, as an edit or the golden harness sets them.
pub fn values(settings: &Settings) -> ParamValues {
    let mut values = ParamValues::new();
    values.set("size", Value::Float(settings.size));
    values.set("threshold", Value::Float(settings.threshold));
    values.set("strength", Value::Float(settings.strength));
    values
}

impl Operation for Bloom {
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
            image.scale,
            &WorkingProfile::linear_rec2020(),
        )
    }

    /// `threshold`, the one whole-image pass that reads the input, writes the lights of
    /// the tile's own pixels at their place in the whole image (docs/ports/bloom.md,
    /// Neighbourhood and scale).
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
        assert_eq!(names, ["size", "threshold", "strength"]);
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
            size: 40.0,
            threshold: 85.0,
            strength: 70.0,
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
        let ours = Bloom.prepare(&params, &image, &input).unwrap();
        let direct = prepare(
            &settings(&params),
            &input,
            [1954, 1301],
            0.5,
            &WorkingProfile::linear_rec2020(),
        )
        .unwrap();
        assert_eq!(ours, direct);
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
        let prepared = Bloom.prepare(&params, &image, &tile).unwrap();
        assert_eq!(prepared.output, tile);
        assert_eq!(prepared.scratch[0].bytes, 3908 * 2602 * 4);
        assert!(Bloom.tiles_whole_image_passes(&params));
    }

    #[test]
    fn the_operation_is_differentiable() {
        // That its kernel agrees is the operations' own check.
        assert!(Bloom.manifest().differentiable);
    }
}
