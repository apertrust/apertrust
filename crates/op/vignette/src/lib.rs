// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/iop/vignette.c)
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

//! Vignetting, ported from darktable (`src/iop/vignette.c`): darkening or
//! lightening, and desaturating or saturating, towards the image's edges, in a
//! superellipse around a centre. The Vignette module (`docs/modules/README.md`). See
//! `docs/ports/vignette.md`.
//!
//! It is in `OPERATIONS` with `default_enabled` off: an unedited photo does not run
//! it, and the session plans it only while a module's binding sets one of its
//! parameters, the Vignette module's (decision 28).

mod setup;

use apertrust_plugin::{
    Domain, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation, Param,
    ParamValues, Provenance, Schema, Slot, Stage,
};
pub use setup::*;

/// Generated from `slang/vignette.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "vignette",
    wgsl: include_str!("../generated/vignette.wgsl"),
    derivatives: true,
};

pub struct Vignette;

static MANIFEST: Manifest = Manifest {
    id: "vignette",
    version: 1,
    // darktable's v5.0 order: after the tone mappers (AgX 45.5) and split-toning (67),
    // before final scale (69.4) and output colour (70); display-referred.
    slot: Slot {
        stage: Stage::Display,
        order: 68.0,
    },
    domain: Domain::Display,
    layers: true,
    differentiable: true,
    // Per pixel: its position comes from the pass header, so a tile needs no border.
    radius: 0,
    // Planned only while a module sets one of its parameters (decision 28).
    default_enabled: false,
    provenance: Some(Provenance {
        darktable_module: "vignette",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &["src/iop/vignette.c", "data/kernels/extended.cl"],
    }),
};

const fn float(name: &'static str, default: f32, min: f32, max: f32) -> Param {
    Param {
        name,
        kind: Kind::Float { default, min, max },
    }
}

/// darktable `dt_iop_vignette_params_t` version 4, by its field names, with its
/// introspection's ranges and defaults; `center.x` and `center.y` become `center_x` and
/// `center_y`.
static SCHEMA: Schema = Schema {
    version: 1,
    params: &[
        float("scale", 80.0, 0.0, 200.0),
        float("falloff_scale", 50.0, 0.0, 200.0),
        float("brightness", -0.5, -1.0, 1.0),
        float("saturation", -0.5, -1.0, 1.0),
        float("center_x", 0.0, -1.0, 1.0),
        float("center_y", 0.0, -1.0, 1.0),
        Param {
            name: "autoratio",
            kind: Kind::Bool { default: false },
        },
        float("whratio", 1.0, 0.0, 2.0),
        float("shape", 1.0, 0.0, 5.0),
        Param {
            name: "dithering",
            kind: Kind::Choice {
                default: 0,
                choices: &["off", "8bit", "16bit"],
            },
        },
        Param {
            name: "unbound",
            kind: Kind::Bool { default: true },
        },
    ],
};

/// The settings from resolved parameter values.
pub fn settings(params: &ParamValues) -> Settings {
    Settings {
        scale: params.float("scale"),
        falloff_scale: params.float("falloff_scale"),
        brightness: params.float("brightness"),
        saturation: params.float("saturation"),
        center: [params.float("center_x"), params.float("center_y")],
        autoratio: params.flag("autoratio"),
        whratio: params.float("whratio"),
        shape: params.float("shape"),
        dithering: Dithering::from_darktable(params.choice("dithering") as i32)
            .expect("the schema's choices are darktable's"),
        unbound: params.flag("unbound"),
    }
}

impl Operation for Vignette {
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
        let prepared = prepare(&settings(params))?;
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
        assert_eq!(SCHEMA.params.len(), 11);
        for parameter in Parameter::ALL {
            assert!(SCHEMA.param(parameter.name()).is_some(), "{parameter:?}");
        }
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
                Kind::Choice { default, .. } => Value::Choice(if default == 0 { 1 } else { 0 }),
                Kind::Bool { default } => Value::Bool(!default),
                Kind::Int { .. } => {
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
    fn prepare_gives_one_per_pixel_pass_without_tables() {
        let prepared = Vignette
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
        assert_eq!(prepared.output, ImageDesc::rgba(1, 1));
    }

    #[test]
    fn the_operation_is_differentiable() {
        // That its kernel agrees is the operations' own check.
        assert!(Vignette.manifest().differentiable);
    }

    #[test]
    fn dithering_is_not_ported() {
        for choice in [1, 2] {
            let mut values = ParamValues::new();
            values.set("dithering", Value::Choice(choice));
            let params = SCHEMA.resolve(&values).unwrap();
            let (image, input) = (ImageContext::full(1, 1), ImageDesc::rgba(1, 1));
            assert!(Vignette.prepare(&params, &image, &input).is_err());
        }
    }
}
