// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2024  darktable developers (ported from src/iop/grain.c)
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

//! Film grain, ported from darktable (`src/iop/grain.c`): simplex noise at each
//! pixel's position added to Lab lightness through a photographic paper's response. The
//! Grain module (`docs/modules/README.md`). See `docs/ports/grain.md`.
//!
//! It is in `OPERATIONS` with `default_enabled` off: an unedited photo does not run
//! it, and the session plans it only while a module's binding sets one of its
//! parameters, the Grain module's (decision 28).

mod setup;

use apertrust_color::WorkingProfile;
use apertrust_plugin::{
    Domain, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation, Param,
    ParamValues, Provenance, Schema, Slot, Stage,
};
pub use setup::*;

/// Generated from `slang/grain.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "grain",
    wgsl: include_str!("../generated/grain.wgsl"),
    derivatives: true,
};

pub struct Grain;

static MANIFEST: Manifest = Manifest {
    id: "grain",
    version: 1,
    // darktable's v5.0 order: after the tone mappers (AgX 45.5) and monochrome (64),
    // before soften (66) and vignette (68); display-referred, in Lab.
    slot: Slot {
        stage: Stage::Display,
        order: 65.0,
    },
    domain: Domain::Display,
    // Grain is background only.
    layers: false,
    differentiable: true,
    // Per pixel: its position comes from the pass header, so a tile needs no border.
    radius: 0,
    // Planned only while a module sets one of its parameters (decision 28).
    default_enabled: false,
    provenance: Some(Provenance {
        darktable_module: "grain",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &[
            "src/iop/grain.c",
            "src/common/iop_profile.c",
            "src/common/colorspaces_inline_conversions.h",
        ],
    }),
};

/// darktable `dt_iop_grain_params_t` version 2's continuous fields, by their names, with
/// its introspection's ranges and defaults (decision 21). darktable's `channel` is left
/// out, since its `process()` ignores it, and so is the offset darktable derives from the
/// file name, which is not a field of darktable's.
static SCHEMA: Schema = Schema {
    version: 1,
    params: &[
        Param {
            name: "scale",
            kind: Kind::Float {
                default: (1600.0 / SCALE_FACTOR) as f32,
                min: (20.0 / SCALE_FACTOR) as f32,
                max: (6400.0 / SCALE_FACTOR) as f32,
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
        Param {
            name: "midtones_bias",
            kind: Kind::Float {
                default: 100.0,
                min: 0.0,
                max: 100.0,
            },
        },
    ],
};

/// The settings from resolved parameter values, at the [`PROVISIONAL_OFFSET`].
pub fn settings(params: &ParamValues) -> Settings {
    Settings {
        scale: params.float("scale"),
        strength: params.float("strength"),
        midtones_bias: params.float("midtones_bias"),
        offset: PROVISIONAL_OFFSET,
    }
}

impl Operation for Grain {
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
        _image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<apertrust_plugin::Prepared, NotPorted> {
        // The image's size at full resolution and the pipe's scale reach the kernel in
        // the pass header.
        let prepared = prepare(&settings(params), &WorkingProfile::linear_rec2020())?;
        Ok(apertrust_plugin::Prepared::per_pixel(
            input,
            prepared.params(),
            vec![permutation_table()],
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apertrust_plugin::Value;

    #[test]
    fn schema_defaults_are_darktables() {
        assert_eq!(settings(&SCHEMA.defaults()), Settings::default());
        assert_eq!(SCHEMA.params.len(), 3);
        for parameter in Parameter::ALL {
            assert!(SCHEMA.param(parameter.name()).is_some(), "{parameter:?}");
        }
        let (min, max) = scale_range();
        assert_eq!(
            SCHEMA.param("scale").unwrap().kind,
            Kind::Float {
                default: Settings::default().scale,
                min,
                max
            }
        );
    }

    #[test]
    fn every_parameter_reaches_the_settings() {
        let base = settings(&SCHEMA.defaults());
        for param in SCHEMA.params {
            let moved = match param.kind {
                Kind::Float { default, min, max } => Value::Float(if default < (min + max) / 2.0 {
                    max
                } else {
                    min
                }),
                Kind::Int { .. } | Kind::Choice { .. } | Kind::Bool { .. } => {
                    unreachable!("{} has a kind the schema does not use", param.name)
                }
            };
            let mut values = ParamValues::new();
            values.set(param.name, moved);
            let changed = settings(&SCHEMA.resolve(&values).unwrap());
            assert_ne!(changed, base, "{} reaches no field", param.name);
        }
    }

    #[test]
    fn prepare_gives_one_per_pixel_pass_with_the_permutation() {
        let prepared = Grain
            .prepare(
                &SCHEMA.defaults(),
                &ImageContext::full(1, 1),
                &ImageDesc::rgba(1, 1),
            )
            .unwrap();
        assert_eq!(prepared.passes.len(), 1);
        let pass = &prepared.passes[0];
        assert_eq!(pass.params.len(), PARAMS_SIZE);
        assert_eq!(pass.tables, [permutation_table()]);
        assert_eq!(prepared.output, ImageDesc::rgba(1, 1));
    }

    #[test]
    fn the_operation_is_differentiable() {
        // That its kernel agrees is the operations' own check.
        assert!(Grain.manifest().differentiable);
    }

    #[test]
    fn the_offset_is_provisional_and_not_a_parameter() {
        // darktable has no such field (decision 21), so an edit saves none; every image
        // starts its noise at the provisional offset until the planned seed in the edit
        // (docs/ports/grain.md, Deliberate differences).
        assert!(SCHEMA.param("offset").is_none());
        assert_eq!(settings(&SCHEMA.defaults()).offset, PROVISIONAL_OFFSET);
        let (image, input) = (ImageContext::full(1, 1), ImageDesc::rgba(1, 1));
        let ours = Grain.prepare(&SCHEMA.defaults(), &image, &input).unwrap();
        let direct = prepare(&Settings::default(), &WorkingProfile::linear_rec2020()).unwrap();
        assert_eq!(ours.passes[0].params, direct.params());
    }
}
