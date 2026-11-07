// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2012-2026  darktable developers (ported from src/iop/bilat.c,
// src/common/locallaplacian.c, src/common/bilateral.c and src/common/math.h)
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

//! Local contrast, ported from darktable (`src/iop/bilat.c`): Lab lightness
//! through a local laplacian filter (`src/common/locallaplacian.c`), darktable's default,
//! or a bilateral grid (`src/common/bilateral.c`), each built over the whole image in
//! whole-image passes and read by a per-pixel `main`. The Local contrast module
//! (`docs/modules/README.md`). See `docs/ports/bilat.md`.
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

/// Generated from `slang/bilat.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "bilat",
    wgsl: include_str!("../generated/bilat.wgsl"),
    derivatives: true,
};

pub struct Bilat;

static MANIFEST: Manifest = Manifest {
    id: "bilat",
    version: 1,
    // darktable's v5.0 order: after the tone mappers (AgX 45.5) and relight (53), before
    // colorcorrection (55) and grain (65); display-referred, in Lab.
    slot: Slot {
        stage: Stage::Display,
        order: 54.0,
    },
    domain: Domain::Display,
    // Local contrast works on layers.
    layers: true,
    differentiable: true,
    // The whole-image passes see the whole image and `main` reads only its own pixel
    // and their scratch, so a tile needs no border.
    radius: 0,
    // Planned only while a module sets one of its parameters (decision 28).
    default_enabled: false,
    provenance: Some(Provenance {
        darktable_module: "bilat",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &[
            "src/iop/bilat.c",
            "src/common/locallaplacian.c",
            "src/common/bilateral.c",
            "src/common/math.h",
            "src/common/iop_profile.c",
            "src/common/colorspaces_inline_conversions.h",
        ],
    }),
};

/// darktable `dt_iop_bilat_params_t` version 3, by its field names, with its
/// introspection's ranges and defaults (decision 21): the local laplacian filter by
/// default (decision 22).
static SCHEMA: Schema = Schema {
    version: 1,
    params: &[
        Param {
            name: "mode",
            kind: Kind::Choice {
                default: Mode::LocalLaplacian as u32,
                choices: &Mode::NAMES,
            },
        },
        Param {
            name: "sigma_r",
            kind: Kind::Float {
                default: 0.5,
                min: 0.0,
                max: 100.0,
            },
        },
        Param {
            name: "sigma_s",
            kind: Kind::Float {
                default: 0.5,
                min: 0.0,
                max: 100.0,
            },
        },
        Param {
            name: "detail",
            kind: Kind::Float {
                default: 0.25,
                min: -1.0,
                max: 4.0,
            },
        },
        Param {
            name: "midtone",
            kind: Kind::Float {
                default: 0.5,
                min: 0.001,
                max: 1.0,
            },
        },
    ],
};

/// The settings from resolved parameter values.
pub fn settings(params: &ParamValues) -> Settings {
    Settings {
        mode: Mode::from_index(params.choice("mode")).unwrap_or(Mode::LocalLaplacian),
        sigma_r: params.float("sigma_r"),
        sigma_s: params.float("sigma_s"),
        detail: params.float("detail"),
        midtone: params.float("midtone"),
    }
}

/// The parameter values of `settings`, as an edit or the golden harness sets them.
pub fn values(settings: &Settings) -> ParamValues {
    let mut values = ParamValues::new();
    values.set("mode", Value::Choice(settings.mode as u32));
    values.set("sigma_r", Value::Float(settings.sigma_r));
    values.set("sigma_s", Value::Float(settings.sigma_s));
    values.set("detail", Value::Float(settings.detail));
    values.set("midtone", Value::Float(settings.midtone));
    values
}

impl Operation for Bilat {
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
        prepare(
            &settings(params),
            input,
            image.scale,
            &WorkingProfile::linear_rec2020(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_defaults_are_darktables() {
        assert_eq!(settings(&SCHEMA.defaults()), Settings::default());
        let names: Vec<&str> = SCHEMA.params.iter().map(|p| p.name).collect();
        assert_eq!(names, ["mode", "sigma_r", "sigma_s", "detail", "midtone"]);
        assert_eq!(Mode::NAMES[Mode::BilateralGrid as usize], "bilateral_grid");
        assert_eq!(
            Mode::NAMES[Mode::LocalLaplacian as usize],
            "local_laplacian"
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
                Kind::Choice { default, .. } => Value::Choice(1 - default),
                Kind::Int { .. } | Kind::Bool { .. } => {
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
    fn prepare_uses_the_pipes_scale_and_the_input() {
        let mut values = ParamValues::new();
        values.set("mode", Value::Choice(Mode::BilateralGrid as u32));
        values.set("sigma_s", Value::Float(50.0));
        values.set("sigma_r", Value::Float(20.0));
        let params = SCHEMA.resolve(&values).unwrap();
        let input = ImageDesc::rgba(3908, 2602);
        let ours = Bilat
            .prepare(&params, &ImageContext::full(3908, 2602), &input)
            .unwrap();
        let direct = prepare(
            &settings(&params),
            &input,
            1.0,
            &WorkingProfile::linear_rec2020(),
        )
        .unwrap();
        assert_eq!(ours, direct);
        assert_eq!(ours.passes[0].size, Some([79, 53]));
    }

    #[test]
    fn values_round_trip_through_the_schema() {
        let settings = Settings {
            mode: Mode::BilateralGrid,
            sigma_r: 20.0,
            sigma_s: 50.0,
            detail: 1.0,
            midtone: 0.3,
        };
        let resolved = SCHEMA.resolve(&values(&settings)).unwrap();
        assert_eq!(super::settings(&resolved), settings);
    }

    #[test]
    fn the_operation_is_differentiable() {
        // That its kernel agrees is the operations' own check.
        assert!(Bilat.manifest().differentiable);
    }
}
