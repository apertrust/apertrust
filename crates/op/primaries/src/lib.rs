// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2023-2025  darktable developers (ported from src/iop/primaries.c)
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

//! RGB primaries, ported from darktable (`src/iop/primaries.c`): the working
//! space's primaries turned about the white and moved towards or away from it, and the
//! white tinted, as one matrix. The RGB primaries module drives it. See
//! `docs/ports/primaries.md`.
//!
//! It is in `OPERATIONS` with `default_enabled` off: an unedited photo does not run it,
//! and the session plans it only while a module's binding sets one of its parameters
//! (decision 28).

mod setup;

use std::f32::consts::PI;

use apertrust_color::RgbSpace;
use apertrust_plugin::{
    Domain, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation, Param,
    ParamValues, Provenance, Schema, Slot, Stage, Value,
};
pub use setup::*;

/// Generated from `slang/primaries.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "primaries",
    wgsl: include_str!("../generated/primaries.wgsl"),
    derivatives: true,
};

pub struct Primaries;

static MANIFEST: Manifest = Manifest {
    id: "primaries",
    version: 1,
    // darktable's v5.0 place: the last of its operations at 28.5, after colour
    // calibration, before colour balance rgb; 28.6 to 28.8 stay for diffuse or sharpen,
    // censorize, negadoctor and blurs, which come before it in darktable's list
    // (decision 22's rule, `docs/engine.md`, Order).
    slot: Slot {
        stage: Stage::WorkingSpace,
        order: 28.9,
    },
    domain: Domain::Scene,
    layers: true,
    differentiable: true,
    radius: 0,
    // Planned only while a module sets one of its parameters (decision 28).
    default_enabled: false,
    provenance: Some(Provenance {
        darktable_module: "primaries",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &[
            "src/iop/primaries.c",
            "src/common/custom_primaries.c",
            "src/common/colorspaces.c",
            "src/common/matrices.c",
            "data/kernels/extended.cl",
        ],
    }),
};

const fn float(name: &'static str, default: f32, min: f32, max: f32) -> Param {
    Param {
        name,
        kind: Kind::Float { default, min, max },
    }
}

/// A hue, in radians, darktable's -π to π (as f32), default 0.
const fn hue(name: &'static str) -> Param {
    float(name, 0.0, -PI, PI)
}

/// A primary's purity, darktable's 0.01 to 5, default 1.
const fn purity(name: &'static str) -> Param {
    float(name, 1.0, 0.01, 5.0)
}

/// darktable `dt_iop_primaries_params_t` version 1, by its field names, in its order,
/// with its ranges and defaults.
static SCHEMA: Schema = Schema {
    version: 1,
    params: &[
        hue("achromatic_tint_hue"),
        float("achromatic_tint_purity", 0.0, 0.0, 0.99),
        hue("red_hue"),
        purity("red_purity"),
        hue("green_hue"),
        purity("green_purity"),
        hue("blue_hue"),
        purity("blue_purity"),
    ],
};

/// The settings from resolved parameter values.
pub fn settings(params: &ParamValues) -> Settings {
    let mut settings = Settings::default();
    for parameter in Parameter::ALL {
        *settings.get_mut(parameter) = params.float(parameter.name());
    }
    settings
}

/// The parameter values for these settings, by the schema's names: the inverse of
/// [`settings`]. They still need checking against the schema ([`Schema::resolve`]).
pub fn values(settings: &Settings) -> ParamValues {
    let mut values = ParamValues::new();
    for parameter in Parameter::ALL {
        values.set(parameter.name(), Value::Float(settings.get(parameter)));
    }
    values
}

impl Operation for Primaries {
    fn manifest(&self) -> &'static Manifest {
        &MANIFEST
    }

    fn schema(&self) -> &'static Schema {
        &SCHEMA
    }

    fn kernel(&self) -> &'static Kernel {
        &KERNEL
    }

    /// darktable's `_calculate_adjustment_matrix`, which `process` runs.
    fn prepare(
        &self,
        params: &ParamValues,
        _image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<apertrust_plugin::Prepared, NotPorted> {
        // Apertrust's working space is fixed: linear Rec2020.
        let prepared = prepare(&settings(params), &RgbSpace::LINEAR_REC2020);
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

    #[test]
    fn the_schema_is_darktables_version_1_in_order() {
        let names: Vec<&str> = SCHEMA.params.iter().map(|p| p.name).collect();
        let fields: Vec<&str> = Parameter::ALL.iter().map(|p| p.name()).collect();
        assert_eq!(names, fields);
        assert_eq!(settings(&SCHEMA.defaults()), Settings::default());
        let range = |name: &str| match SCHEMA.param(name).unwrap().kind {
            Kind::Float { default, min, max } => (default, min, max),
            other => panic!("{name} is {other:?}"),
        };
        // darktable's $MIN and $MAX: -M_PI and M_PI, as f32.
        assert_eq!(range("red_hue"), (0.0, -PI, PI));
        assert_eq!(range("achromatic_tint_hue"), (0.0, -PI, PI));
        assert_eq!(range("achromatic_tint_purity"), (0.0, 0.0, 0.99));
        assert_eq!(range("blue_purity"), (1.0, 0.01, 5.0));
    }

    #[test]
    fn every_parameter_reaches_its_own_field() {
        let defaults = settings(&SCHEMA.defaults());
        for param in SCHEMA.params {
            let Kind::Float { max, .. } = param.kind else {
                panic!("{} is not a float", param.name);
            };
            let mut one = ParamValues::new();
            one.set(param.name, Value::Float(max));
            let resolved = SCHEMA.resolve(&one).unwrap();
            let changed = settings(&resolved);
            assert_ne!(changed, defaults, "{} reaches no field", param.name);
            assert_eq!(
                values(&changed),
                resolved,
                "{} does not round-trip",
                param.name
            );
        }
    }

    #[test]
    fn prepare_gives_one_per_pixel_pass_without_tables() {
        let prepared = Primaries
            .prepare(
                &SCHEMA.defaults(),
                &ImageContext::full(1, 1),
                &ImageDesc::rgba(1, 1),
            )
            .unwrap();
        assert_eq!(prepared.output, ImageDesc::rgba(1, 1));
        assert_eq!(prepared.passes.len(), 1);
        let pass = &prepared.passes[0];
        assert_eq!(pass.entry_point, "main");
        assert_eq!(pass.params.len(), PARAMS_SIZE);
        assert!(pass.tables.is_empty());
        let setup = prepare(&Settings::default(), &RgbSpace::LINEAR_REC2020);
        assert_eq!(pass.params, setup.params());
    }

    #[test]
    fn the_operation_is_differentiable_and_per_pixel() {
        // That its kernel agrees, and its slot, are the operations' own checks.
        let manifest = Primaries.manifest();
        assert!(manifest.differentiable);
        assert!(!manifest.default_enabled);
        assert_eq!(manifest.radius, 0);
    }
}
