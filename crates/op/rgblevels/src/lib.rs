// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2019-2026  darktable developers (ported from src/iop/rgblevels.c)
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

//! RGB levels, ported from darktable (`src/iop/rgblevels.c`): black, grey and
//! white points, linked or per channel, with every way of preserving colours. The Tone
//! curve module's black and white point handles (decision 2). See
//! `docs/ports/rgblevels.md`.
//!
//! It is in `OPERATIONS` with `default_enabled` off: an unedited photo does not run
//! it, and the session plans it only while a module's binding sets one of its
//! parameters: the Tone curve module's, if its handles stay on it (decision 28).

mod setup;

pub use apertrust_color::RgbNorm;
use apertrust_color::WorkingProfile;
use apertrust_plugin::{
    Domain, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation, Param,
    ParamValues, Provenance, Schema, Slot as PipelineSlot, Stage,
};
pub use setup::*;

/// Generated from `slang/rgblevels.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "rgblevels",
    wgsl: include_str!("../generated/rgblevels.wgsl"),
    derivatives: true,
};

pub struct RgbLevels;

static MANIFEST: Manifest = Manifest {
    id: "rgblevels",
    version: 1,
    // darktable's v5.0 place: after RGB curve (42), before the tone mapper, on scene
    // light (decision 22, `docs/engine.md`, Order).
    slot: PipelineSlot {
        stage: Stage::WorkingSpace,
        order: 43.0,
    },
    domain: Domain::Scene,
    layers: true,
    differentiable: true,
    radius: 0,
    // Planned only while a module sets one of its parameters (decision 28).
    default_enabled: false,
    provenance: Some(Provenance {
        darktable_module: "rgblevels",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &[
            "src/iop/rgblevels.c",
            "src/common/rgb_norms.h",
            "src/common/iop_profile.h",
            "data/kernels/rgblevels.cl",
        ],
    }),
};

const fn level(name: &'static str, default: f32) -> Param {
    Param {
        name,
        kind: Kind::Float {
            default,
            min: 0.0,
            max: 1.0,
        },
    }
}

/// darktable `dt_iop_rgblevels_params_t` version 1, by its field names, with its
/// defaults (`init`). `levels[c]`, black, grey and white, becomes `levels_<c>_black`,
/// `_grey` and `_white`, from 0 to 1 as darktable's handles keep them.
static SCHEMA: Schema = Schema {
    version: 1,
    params: &[
        Param {
            name: "autoscale",
            kind: Kind::Choice {
                default: 0,
                choices: &["linked_channels", "independent_channels"],
            },
        },
        Param {
            name: "preserve_colors",
            kind: Kind::Choice {
                default: 1,
                choices: &[
                    "none",
                    "luminance",
                    "max_rgb",
                    "average_rgb",
                    "sum_rgb",
                    "norm_rgb",
                    "basic_power",
                ],
            },
        },
        level("levels_0_black", 0.0),
        level("levels_0_grey", 0.5),
        level("levels_0_white", 1.0),
        level("levels_1_black", 0.0),
        level("levels_1_grey", 0.5),
        level("levels_1_white", 1.0),
        level("levels_2_black", 0.0),
        level("levels_2_grey", 0.5),
        level("levels_2_white", 1.0),
    ],
};

/// The settings from resolved parameter values.
pub fn settings(params: &ParamValues) -> Settings {
    let levels = |c: usize| Levels {
        black: params.float(&format!("levels_{c}_black")),
        grey: params.float(&format!("levels_{c}_grey")),
        white: params.float(&format!("levels_{c}_white")),
    };
    Settings {
        mode: Mode::from_darktable(params.choice("autoscale") as i32)
            .expect("the schema's choices are darktable's"),
        preserve_colors: RgbNorm::from_darktable(params.choice("preserve_colors") as i32)
            .expect("the schema's choices are darktable's"),
        levels: [levels(0), levels(1), levels(2)],
    }
}

impl Operation for RgbLevels {
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
        // Apertrust's working space is fixed: linear Rec2020.
        let prepared = prepare(&settings(params), &WorkingProfile::linear_rec2020())?;
        Ok(apertrust_plugin::Prepared::per_pixel(
            input,
            prepared.params(),
            Vec::new(),
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
        assert_eq!(SCHEMA.params.len(), 2 + 9);
    }

    #[test]
    fn every_parameter_reaches_the_settings() {
        let base = settings(&SCHEMA.defaults());
        for param in SCHEMA.params {
            let moved = match param.kind {
                Kind::Float { default, .. } => {
                    Value::Float(if default < 0.5 { 0.25 } else { 0.75 })
                }
                Kind::Choice { default, .. } => Value::Choice(if default == 0 { 1 } else { 0 }),
                _ => unreachable!("{} has a kind the schema does not use", param.name),
            };
            let mut values = ParamValues::new();
            values.set(param.name, moved);
            let changed = settings(&SCHEMA.resolve(&values).unwrap());
            assert_ne!(changed, base, "{} reaches no field", param.name);
        }
    }

    #[test]
    fn prepare_gives_one_per_pixel_pass_without_tables() {
        let prepared = RgbLevels
            .prepare(
                &SCHEMA.defaults(),
                &ImageContext::full(1, 1),
                &ImageDesc::rgba(1, 1),
            )
            .unwrap();
        assert_eq!(prepared.passes.len(), 1);
        let pass = &prepared.passes[0];
        assert_eq!(pass.params.len(), PARAMS_SIZE);
        assert!(pass.tables.is_empty());
    }

    #[test]
    fn the_operation_is_differentiable() {
        // That its kernel agrees is the operations' own check.
        assert!(RgbLevels.manifest().differentiable);
    }

    #[test]
    fn white_at_black_is_not_ported() {
        let mut values = ParamValues::new();
        values.set("levels_0_black", Value::Float(0.6));
        values.set("levels_0_white", Value::Float(0.6));
        let params = SCHEMA.resolve(&values).unwrap();
        let (image, input) = (ImageContext::full(1, 1), ImageDesc::rgba(1, 1));
        assert!(RgbLevels.prepare(&params, &image, &input).is_err());
    }
}
