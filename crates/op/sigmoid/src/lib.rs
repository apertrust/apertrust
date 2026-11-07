// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2020-2026  darktable developers (ported from src/iop/sigmoid.c)
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

//! Sigmoid, a tone mapper darktable offers beside AgX and filmic rgb, ported from
//! darktable (`src/iop/sigmoid.c`) with both colour processings, the hue
//! preservation and the primaries. See `docs/ports/sigmoid.md`.
//!
//! It is in `OPERATIONS` with `default_enabled` off: nothing plans it until an edit can
//! choose it as its tone mapper (decision 2).

mod setup;

use apertrust_color::RgbSpace;
use apertrust_plugin::{
    Domain, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation, Param,
    ParamValues, Provenance, Schema, Slot, Stage, Value,
};
pub use setup::*;

/// Generated from `slang/sigmoid.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "sigmoid",
    wgsl: include_str!("../generated/sigmoid.wgsl"),
    derivatives: true,
};

pub struct Sigmoid;

static MANIFEST: Manifest = Manifest {
    id: "sigmoid",
    version: 1,
    // darktable's v5.0 place: after RGB levels (43), before AgX (45.5).
    slot: Slot {
        stage: Stage::WorkingSpace,
        order: 45.3,
    },
    domain: Domain::Scene,
    // Background only: the tone mapper's parameters are global (decision 33).
    layers: false,
    differentiable: true,
    radius: 0,
    // Planned only while an edit chooses it as its tone mapper (decision 2).
    default_enabled: false,
    provenance: Some(Provenance {
        darktable_module: "sigmoid",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &["src/iop/sigmoid.c", "data/kernels/sigmoid.cl"],
    }),
};

const fn float(name: &'static str, default: f32, min: f32, max: f32) -> Param {
    Param {
        name,
        kind: Kind::Float { default, min, max },
    }
}

/// A primary's attenuation, 0 to 0.99, and rotation, ±0.4 radians.
const fn inset(name: &'static str) -> Param {
    float(name, 0.0, 0.0, 0.99)
}

const fn rotation(name: &'static str) -> Param {
    float(name, 0.0, -0.4, 0.4)
}

/// darktable `dt_iop_sigmoid_params_t` version 3, by its field names, in its order, with
/// its ranges and defaults (`Settings::default`).
static SCHEMA: Schema = Schema {
    version: 1,
    params: &[
        float("middle_grey_contrast", 1.5, 0.1, 10.0),
        float("contrast_skewness", 0.0, -1.0, 1.0),
        float("display_white_target", 100.0, 20.0, 1600.0),
        float("display_black_target", 0.0152, 0.0, 15.0),
        Param {
            name: "color_processing",
            kind: Kind::Choice {
                default: 0,
                choices: &["per_channel", "rgb_ratio"],
            },
        },
        float("hue_preservation", 100.0, 0.0, 100.0),
        inset("red_inset"),
        rotation("red_rotation"),
        inset("green_inset"),
        rotation("green_rotation"),
        inset("blue_inset"),
        rotation("blue_rotation"),
        float("purity", 0.0, 0.0, 1.0),
        Param {
            name: "base_primaries",
            kind: Kind::Choice {
                default: 0,
                choices: &[
                    "working_profile",
                    "rec2020",
                    "display_p3",
                    "adobe_rgb",
                    "srgb",
                ],
            },
        },
    ],
};

/// The settings from resolved parameter values.
pub fn settings(params: &ParamValues) -> Settings {
    let per_primary = |what: &str| {
        ["red", "green", "blue"].map(|primary| params.float(&format!("{primary}_{what}")))
    };
    let choice = |name: &str| params.choice(name) as i32;
    Settings {
        middle_grey_contrast: params.float("middle_grey_contrast"),
        contrast_skewness: params.float("contrast_skewness"),
        display_white_target: params.float("display_white_target"),
        display_black_target: params.float("display_black_target"),
        color_processing: ColorProcessing::from_darktable(choice("color_processing"))
            .expect("the schema's choices are darktable's"),
        hue_preservation: params.float("hue_preservation"),
        inset: per_primary("inset"),
        rotation: per_primary("rotation"),
        purity: params.float("purity"),
        base_primaries: BasePrimaries::from_darktable(choice("base_primaries"))
            .expect("the schema's choices are darktable's"),
    }
}

/// The parameter values for these settings, by the schema's names: the inverse of
/// [`settings`]. They still need checking against the schema ([`Schema::resolve`]).
pub fn values(settings: &Settings) -> ParamValues {
    let s = settings;
    let mut values = ParamValues::new();
    for (name, value) in [
        ("middle_grey_contrast", s.middle_grey_contrast),
        ("contrast_skewness", s.contrast_skewness),
        ("display_white_target", s.display_white_target),
        ("display_black_target", s.display_black_target),
        ("hue_preservation", s.hue_preservation),
        ("red_inset", s.inset[0]),
        ("red_rotation", s.rotation[0]),
        ("green_inset", s.inset[1]),
        ("green_rotation", s.rotation[1]),
        ("blue_inset", s.inset[2]),
        ("blue_rotation", s.rotation[2]),
        ("purity", s.purity),
    ] {
        values.set(name, Value::Float(value));
    }
    let choice = |value: i32| Value::Choice(value as u32);
    values.set(
        "color_processing",
        choice(s.color_processing.to_darktable()),
    );
    values.set("base_primaries", choice(s.base_primaries.to_darktable()));
    values
}

impl Operation for Sigmoid {
    fn manifest(&self) -> &'static Manifest {
        &MANIFEST
    }

    fn schema(&self) -> &'static Schema {
        &SCHEMA
    }

    fn kernel(&self) -> &'static Kernel {
        &KERNEL
    }

    /// darktable's `commit_params`, with the matrices its `process` builds. Every setting
    /// is ported.
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
    fn schema_defaults_are_darktables() {
        assert_eq!(settings(&SCHEMA.defaults()), Settings::default());
    }

    #[test]
    fn the_schema_is_darktables_version_3_in_order() {
        let names: Vec<&str> = SCHEMA.params.iter().map(|p| p.name).collect();
        assert_eq!(
            names,
            [
                "middle_grey_contrast",
                "contrast_skewness",
                "display_white_target",
                "display_black_target",
                "color_processing",
                "hue_preservation",
                "red_inset",
                "red_rotation",
                "green_inset",
                "green_rotation",
                "blue_inset",
                "blue_rotation",
                "purity",
                "base_primaries",
            ]
        );
    }

    #[test]
    fn every_parameter_reaches_its_own_field() {
        // Move every parameter off its default: the settings change, and the values
        // come back unchanged.
        let defaults = settings(&SCHEMA.defaults());
        for param in SCHEMA.params {
            let moved = match param.kind {
                Kind::Float { default, min, max } => {
                    Value::Float(if default < max { max } else { min })
                }
                Kind::Choice { default, .. } => Value::Choice(if default == 1 { 2 } else { 1 }),
                other => panic!("{} is {other:?}", param.name),
            };
            let mut one = ParamValues::new();
            one.set(param.name, moved);
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
    fn prepare_gives_one_per_pixel_pass_of_the_setup() {
        let mut values = ParamValues::new();
        values.set("middle_grey_contrast", Value::Float(2.0));
        values.set("color_processing", Value::Choice(1));
        values.set("base_primaries", Value::Choice(4));
        values.set("red_inset", Value::Float(0.2));
        let params = SCHEMA.resolve(&values).unwrap();
        let prepared = Sigmoid
            .prepare(&params, &ImageContext::full(1, 1), &ImageDesc::rgba(1, 1))
            .unwrap();
        assert_eq!(prepared.output, ImageDesc::rgba(1, 1));
        assert_eq!(prepared.passes.len(), 1);
        let pass = &prepared.passes[0];
        assert_eq!(pass.entry_point, "main");
        assert!(pass.tables.is_empty());
        let setup = prepare(&settings(&params), &RgbSpace::LINEAR_REC2020);
        assert_eq!(pass.params, setup.params());
        assert_eq!(pass.params.len(), PARAMS_SIZE);
    }

    #[test]
    fn the_operation_is_a_differentiable_per_pixel_tone_mapper() {
        // That its kernel agrees, and its slot, are the operations' own checks.
        let manifest = Sigmoid.manifest();
        assert!(manifest.differentiable);
        assert_eq!(manifest.radius, 0);
        assert!(!manifest.layers && !manifest.default_enabled);
    }
}
