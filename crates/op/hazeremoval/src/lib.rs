// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2017-2025  darktable developers (ported from src/iop/hazeremoval.c)
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

//! Haze removal, ported from darktable (`src/iop/hazeremoval.c`, with
//! `src/common/guided_filter.c` and `src/common/box_filters.cc`): He, Sun and Tang's
//! dark channel prior, with the ambient light, the image's depth and the filtered
//! transmission computed by whole-image passes on the GPU (decision 15). The
//! first pass's Haze removal module drives it (`docs/modules/README.md`). See
//! `docs/ports/hazeremoval.md`.
//!
//! It is in `OPERATIONS` with `default_enabled` off: an unedited photo does not run
//! it, and the session plans it only while a module's binding sets one of its
//! parameters (decision 28).

mod setup;

use apertrust_plugin::{
    Domain, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation, Param,
    ParamValues, Prepared, Provenance, Schema, Slot, Stage, Value,
};
pub use setup::*;

/// Generated from `slang/hazeremoval.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "hazeremoval",
    wgsl: include_str!("../generated/hazeremoval.wgsl"),
    derivatives: true,
};

pub struct HazeRemoval;

static MANIFEST: Manifest = Manifest {
    id: "hazeremoval",
    version: 1,
    // darktable's v5.0 place: after demosaic, lens correction and chromatic
    // aberrations, before rotation and orientation, in camera RGB (decision 22).
    slot: Slot {
        stage: Stage::CameraRgb,
        order: 14.0,
    },
    domain: Domain::Scene,
    layers: true,
    differentiable: true,
    // Its neighbourhood, two boxes of 6 and two of 9 pixels at full resolution, lies in
    // its whole-image passes, which see the whole image; its last pass reads its own
    // pixel and their results.
    radius: 0,
    // Planned only while a module sets one of its parameters (decision 28).
    default_enabled: false,
    provenance: Some(Provenance {
        darktable_module: "hazeremoval",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &[
            "src/iop/hazeremoval.c",
            "src/common/guided_filter.c",
            "src/common/box_filters.cc",
        ],
    }),
};

/// darktable `dt_iop_hazeremoval_params_t` version 3's sliders, by their field names,
/// with their ranges and defaults.
static SCHEMA: Schema = Schema {
    version: 1,
    params: &[
        Param {
            name: "strength",
            kind: Kind::Float {
                default: 0.2,
                min: -1.0,
                max: 1.0,
            },
        },
        Param {
            name: "distance",
            kind: Kind::Float {
                default: 0.2,
                min: 0.0,
                max: 1.0,
            },
        },
    ],
};

/// The settings from resolved parameter values.
pub fn settings(params: &ParamValues) -> Settings {
    Settings {
        strength: params.float("strength"),
        distance: params.float("distance"),
    }
}

/// The parameter values for these settings: the inverse of [`settings`]. They still
/// need checking against the schema ([`Schema::resolve`]).
pub fn values(settings: &Settings) -> ParamValues {
    let mut values = ParamValues::new();
    values.set("strength", Value::Float(settings.strength));
    values.set("distance", Value::Float(settings.distance));
    values
}

impl Operation for HazeRemoval {
    fn manifest(&self) -> &'static Manifest {
        &MANIFEST
    }

    fn schema(&self) -> &'static Schema {
        &SCHEMA
    }

    fn kernel(&self) -> &'static Kernel {
        &KERNEL
    }

    /// darktable's `process()` set-up: the windows at the pipe's scale, the quantile's
    /// rank and the passes, their scratch laid out as the whole image at the pipe's scale
    /// (decision 16).
    fn prepare(
        &self,
        params: &ParamValues,
        image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<Prepared, NotPorted> {
        setup::prepare(
            &settings(params),
            input,
            image.whole_image(input),
            image.scale,
            image.full_resolution.map(|full| [full.width, full.height]),
        )
    }

    /// `image_copy`, the one whole-image pass that reads the input, writes the tile's own
    /// pixels at their place in the whole image, and every later one reads only that
    /// scratch: one run, which an export's first round of tiles covers (decision 15).
    fn tiles_whole_image_passes(&self, _params: &ParamValues) -> bool {
        true
    }

    /// Its estimate is not scale-invariant: the dark channel's box minimum over a
    /// downscaled image finds fewer lows than the export's, which the strength magnifies
    /// in the transmission, so the viewport computes it at full resolution (decision
    /// 15; `docs/ports/hazeremoval.md`, Neighbourhood and scale).
    fn whole_image_at_full_resolution(&self, _params: &ParamValues) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_schema_is_darktables_sliders_with_their_defaults() {
        assert_eq!(settings(&SCHEMA.defaults()), Settings::default());
        let range = |name: &str| match SCHEMA.param(name).unwrap().kind {
            Kind::Float { default, min, max } => (default, min, max),
            other => panic!("{name} is {other:?}"),
        };
        assert_eq!(range("strength"), (0.2, -1.0, 1.0));
        assert_eq!(range("distance"), (0.2, 0.0, 1.0));
    }

    #[test]
    fn values_and_settings_are_inverse() {
        let settings = Settings {
            strength: -0.4,
            distance: 0.7,
        };
        let resolved = SCHEMA.resolve(&values(&settings)).unwrap();
        assert_eq!(self::settings(&resolved), settings);
    }

    #[test]
    fn prepare_runs_at_the_pipes_scale() {
        let image = ImageContext {
            scale: 0.5,
            ..ImageContext::full(200, 120)
        };
        let prepared = HazeRemoval
            .prepare(&SCHEMA.defaults(), &image, &ImageDesc::rgba(100, 60))
            .unwrap();
        let first = &prepared.passes[0].params;
        let word = |i: usize| u32::from_le_bytes(first[4 * i..4 * i + 4].try_into().unwrap());
        assert_eq!([word(0), word(1), word(2), word(3)], [100, 60, 3, 5]);
    }

    #[test]
    fn a_tile_of_an_export_lays_its_scratch_out_as_the_whole_image() {
        // A tile at (200, 160) of a 3908×2602 export: its first pass covers the tile, and
        // every later whole-image pass and the scratch the whole image.
        let image = ImageContext::full(3908, 2602);
        let tile = ImageDesc {
            origin: [200, 160],
            extent: [3908, 2602],
            scaled_extent: [3908, 2602],
            ..ImageDesc::rgba(64, 48)
        };
        let prepared = HazeRemoval
            .prepare(&SCHEMA.defaults(), &image, &tile)
            .unwrap();
        assert_eq!(prepared.output, tile);
        assert_eq!(prepared.passes[0].entry_point, "image_copy");
        assert_eq!(prepared.passes[0].size, None);
        assert_eq!(prepared.passes[1].size, Some([3908, 2602]));
        let first = &prepared.passes[0].params;
        let word = |i: usize| u32::from_le_bytes(first[4 * i..4 * i + 4].try_into().unwrap());
        assert_eq!([word(0), word(1)], [3908, 2602]);
        assert_eq!(prepared.scratch[0].bytes, 3908 * 2602 * 4);
        assert!(HazeRemoval.tiles_whole_image_passes(&SCHEMA.defaults()));
    }
}
