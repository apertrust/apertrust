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

//! AgX, darktable's tone mapper, ported from darktable (`src/iop/agx.c`). See
//! `docs/ports/agx.md`.

#![allow(
    clippy::excessive_precision,
    reason = "defaults are copied digit for digit from darktable"
)]

mod setup;

use apertrust_color::RgbSpace;
use apertrust_plugin::{
    Domain, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation, Param,
    ParamValues, Provenance, Schema, Slot, Stage,
};
pub use setup::*;

/// Generated from `slang/agx.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "agx",
    wgsl: include_str!("../generated/agx.wgsl"),
    derivatives: true,
};

pub struct Agx;

static MANIFEST: Manifest = Manifest {
    id: "agx",
    version: 1,
    slot: Slot {
        stage: Stage::WorkingSpace,
        order: 45.5,
    },
    domain: Domain::Scene,
    // Background only: the tone mapper's parameters are global (decision 33).
    layers: false,
    differentiable: true,
    radius: 0,
    default_enabled: true,
    provenance: Some(Provenance {
        darktable_module: "agx",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &["src/iop/agx.c", "data/kernels/agx.cl"],
    }),
};

const fn float(name: &'static str, default: f32, min: f32, max: f32) -> Param {
    Param {
        name,
        kind: Kind::Float { default, min, max },
    }
}

const fn flag(name: &'static str) -> Param {
    Param {
        name,
        kind: Kind::Bool { default: false },
    }
}

/// The largest rotation darktable allows, in radians: about 30°.
#[allow(
    clippy::approx_constant,
    reason = "darktable's own limit, 0.5236, which is not π/6"
)]
const ROTATION: f32 = 0.5236;

/// darktable `dt_iop_agx_params_t` version 7, by its field names, with its ranges and
/// the scene-referred defaults (`Settings::scene_referred_default`), except
/// `dynamic_range_scaling`, which only darktable's pickers use.
static SCHEMA: Schema = Schema {
    version: 1,
    params: &[
        float("look_lift", 0.0, -1.0, 1.0),
        float("look_slope", 1.0, 0.0, 10.0),
        float("look_brightness", 1.0, 0.0, 100.0),
        float("look_saturation", 1.0, 0.0, 10.0),
        float("look_original_hue_mix_ratio", 0.6, 0.0, 1.0),
        float("range_black_relative_ev", -10.0, -20.0, -0.1),
        float("range_white_relative_ev", 6.5, 0.1, 20.0),
        float("curve_pivot_x", 10.0 / 16.5, 0.0, 1.0),
        float("curve_pivot_y_linear_output", 0.18, 0.0, 1.0),
        float("curve_contrast_around_pivot", 3.0, 0.1, 10.0),
        float("curve_linear_ratio_below_pivot", 0.0, 0.0, 1.0),
        float("curve_linear_ratio_above_pivot", 0.0, 0.0, 1.0),
        float("curve_toe_power", 1.5, 0.0, 10.0),
        float("curve_shoulder_power", 3.3, 0.0, 10.0),
        float("curve_gamma", 2.2, 0.01, 100.0),
        flag("auto_gamma"),
        float("curve_target_display_black_ratio", 0.0, 0.0, 0.15),
        float("curve_target_display_white_ratio", 1.0, 0.2, 1.0),
        Param {
            name: "base_primaries",
            kind: Kind::Choice {
                default: 2,
                choices: &[
                    "export_profile",
                    "working_profile",
                    "rec2020",
                    "display_p3",
                    "adobe_rgb",
                    "srgb",
                ],
            },
        },
        flag("disable_primaries_adjustments"),
        float("red_inset", 0.29462451, 0.0, 0.99),
        float("red_rotation", 0.03540329, -ROTATION, ROTATION),
        float("green_inset", 0.25861925, 0.0, 0.99),
        float("green_rotation", -0.02108586, -ROTATION, ROTATION),
        float("blue_inset", 0.14641371, 0.0, 0.99),
        float("blue_rotation", -0.06305724, -ROTATION, ROTATION),
        float("master_outset_ratio", 1.0, 0.0, 2.0),
        float("master_unrotation_ratio", 0.0, 0.0, 2.0),
        float("red_outset", 0.290776401758, 0.0, 0.99),
        float("red_unrotation", 0.03540329, -ROTATION, ROTATION),
        float("green_outset", 0.263155400753, 0.0, 0.99),
        float("green_unrotation", -0.02108586, -ROTATION, ROTATION),
        float("blue_outset", 0.045810721815, 0.0, 0.99),
        float("blue_unrotation", -0.06305724, -ROTATION, ROTATION),
        flag("completely_reverse_primaries"),
    ],
};

/// The settings from resolved parameter values.
pub fn settings(params: &ParamValues) -> Settings {
    let per_primary = |what: &str| {
        ["red", "green", "blue"].map(|primary| params.float(&format!("{primary}_{what}")))
    };
    let base_primaries = BasePrimaries::from_darktable(params.choice("base_primaries") as i32)
        .expect("the schema's choices are darktable's");
    Settings {
        look_lift: params.float("look_lift"),
        look_slope: params.float("look_slope"),
        look_brightness: params.float("look_brightness"),
        look_saturation: params.float("look_saturation"),
        look_original_hue_mix_ratio: params.float("look_original_hue_mix_ratio"),
        range_black_relative_ev: params.float("range_black_relative_ev"),
        range_white_relative_ev: params.float("range_white_relative_ev"),
        curve_pivot_x: params.float("curve_pivot_x"),
        curve_pivot_y_linear_output: params.float("curve_pivot_y_linear_output"),
        curve_contrast_around_pivot: params.float("curve_contrast_around_pivot"),
        curve_linear_ratio_below_pivot: params.float("curve_linear_ratio_below_pivot"),
        curve_linear_ratio_above_pivot: params.float("curve_linear_ratio_above_pivot"),
        curve_toe_power: params.float("curve_toe_power"),
        curve_shoulder_power: params.float("curve_shoulder_power"),
        curve_gamma: params.float("curve_gamma"),
        auto_gamma: params.flag("auto_gamma"),
        curve_target_display_black_ratio: params.float("curve_target_display_black_ratio"),
        curve_target_display_white_ratio: params.float("curve_target_display_white_ratio"),
        base_primaries,
        disable_primaries_adjustments: params.flag("disable_primaries_adjustments"),
        inset: per_primary("inset"),
        rotation: per_primary("rotation"),
        master_outset_ratio: params.float("master_outset_ratio"),
        master_unrotation_ratio: params.float("master_unrotation_ratio"),
        outset: per_primary("outset"),
        unrotation: per_primary("unrotation"),
        completely_reverse_primaries: params.flag("completely_reverse_primaries"),
    }
}

impl Operation for Agx {
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
        let prepared = prepare(&settings(params), &RgbSpace::LINEAR_REC2020)?;
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
    fn schema_defaults_are_the_scene_referred_defaults() {
        assert_eq!(
            settings(&SCHEMA.defaults()),
            Settings::scene_referred_default()
        );
    }

    #[test]
    fn every_field_has_its_own_parameter() {
        // Move every parameter off its default, and check the settings change in
        // exactly the matching field.
        for param in SCHEMA.params {
            let mut values = ParamValues::new();
            let moved = match param.kind {
                Kind::Float { default, min, max } => {
                    Value::Float(if default < max { max } else { min })
                }
                Kind::Int { default, min, max } => {
                    Value::Int(if default < max { max } else { min })
                }
                Kind::Choice { default, .. } => Value::Choice(if default == 3 { 4 } else { 3 }),
                Kind::Bool { default } => Value::Bool(!default),
            };
            values.set(param.name, moved);
            let changed = settings(&SCHEMA.resolve(&values).unwrap());
            assert_ne!(
                changed,
                Settings::scene_referred_default(),
                "{} reaches no field",
                param.name
            );
        }
        let names: std::collections::HashSet<_> = SCHEMA.params.iter().map(|p| p.name).collect();
        assert_eq!(names.len(), SCHEMA.params.len(), "a name repeats");
    }

    #[test]
    fn prepare_gives_one_per_pixel_pass() {
        let prepared = Agx
            .prepare(
                &SCHEMA.defaults(),
                &ImageContext::full(1, 1),
                &ImageDesc::rgba(1, 1),
            )
            .unwrap();
        assert_eq!(prepared.passes.len(), 1);
        assert_eq!(prepared.passes[0].params.len(), PARAMS_SIZE);
    }

    #[test]
    fn export_profile_primaries_are_not_ported() {
        let mut values = ParamValues::new();
        values.set("base_primaries", Value::Choice(0));
        let params = SCHEMA.resolve(&values).unwrap();
        let (image, input) = (ImageContext::full(1, 1), ImageDesc::rgba(1, 1));
        assert!(Agx.prepare(&params, &image, &input).is_err());
    }
}
