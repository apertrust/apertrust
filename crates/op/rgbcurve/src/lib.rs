// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2019-2026  darktable developers (ported from src/iop/rgbcurve.c)
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

//! RGB curve, ported from darktable (`src/iop/rgbcurve.c`): linked or independent
//! channels, every interpolation, every way of preserving colours, middle grey
//! compensated or not. The Tone curve module's curves (`docs/modules/README.md`). See
//! `docs/ports/rgbcurve.md`.
//!
//! It is in `OPERATIONS` with `default_enabled` off: an unedited photo does not run
//! it, and the session plans it only while a module's binding sets one of its
//! parameters, the Tone curve module's (decision 28).

mod setup;

use apertrust_color::WorkingProfile;
pub use apertrust_color::curve::{Interpolation, Knot, MAX_NODES};
use apertrust_plugin::{
    Domain, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation, Param,
    ParamValues, Provenance, Schema, Slot as PipelineSlot, Stage,
};
pub use setup::*;

/// Generated from `slang/rgbcurve.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "rgbcurve",
    wgsl: include_str!("../generated/rgbcurve.wgsl"),
    derivatives: true,
};

pub struct RgbCurve;

static MANIFEST: Manifest = Manifest {
    id: "rgbcurve",
    version: 1,
    // darktable's v5.0 place: after color balance rgb (41.5), before RGB levels (43) and
    // the tone mapper, on scene light (decision 22, `docs/engine.md`, Order).
    slot: PipelineSlot {
        stage: Stage::WorkingSpace,
        order: 42.0,
    },
    domain: Domain::Scene,
    layers: true,
    differentiable: true,
    radius: 0,
    // Planned only while a module sets one of its parameters (decision 28).
    default_enabled: false,
    provenance: Some(Provenance {
        darktable_module: "rgbcurve",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &[
            "src/iop/rgbcurve.c",
            "src/common/curve_tools.c",
            "src/common/rgb_norms.h",
            "src/common/iop_profile.h",
            "src/develop/imageop_math.h",
            "src/gui/draw.h",
            "data/kernels/rgbcurve.cl",
        ],
    }),
};

const fn float(name: &'static str, default: f32) -> Param {
    Param {
        name,
        kind: Kind::Float {
            default,
            min: 0.0,
            max: 1.0,
        },
    }
}

const fn nodes(name: &'static str) -> Param {
    Param {
        name,
        kind: Kind::Int {
            default: 2,
            min: 2,
            max: MAX_NODES as i64,
        },
    }
}

const fn interpolation(name: &'static str) -> Param {
    Param {
        name,
        kind: Kind::Choice {
            default: 2,
            choices: &["cubic_spline", "catmull_rom", "monotone_hermite"],
        },
    }
}

/// The schema: every node of every curve, `curve_nodes_<curve>_<node>_x` and `_y`, from
/// `(curve, node, default)` triples, then the other fields.
macro_rules! schema_params {
    ($(($c:literal, $k:literal, $default:literal)),* ; $($rest:expr),* $(,)?) => {
        [
            $(
                float(concat!("curve_nodes_", $c, "_", $k, "_x"), $default),
                float(concat!("curve_nodes_", $c, "_", $k, "_y"), $default),
            )*
            $($rest),*
        ]
    };
}

/// darktable `dt_iop_rgbcurve_params_t` version 1, by its field names, with its
/// defaults: two nodes, (0, 0) and (1, 1), per curve (`init`). The node count runs from
/// 2, the fewest darktable can interpolate, to 20, its `MAX_ANCHORS`.
static SCHEMA: Schema = Schema {
    version: 1,
    params: &schema_params![
        (0, 0, 0.0), (0, 1, 1.0), (0, 2, 0.0), (0, 3, 0.0), (0, 4, 0.0),
        (0, 5, 0.0), (0, 6, 0.0), (0, 7, 0.0), (0, 8, 0.0), (0, 9, 0.0),
        (0, 10, 0.0), (0, 11, 0.0), (0, 12, 0.0), (0, 13, 0.0), (0, 14, 0.0),
        (0, 15, 0.0), (0, 16, 0.0), (0, 17, 0.0), (0, 18, 0.0), (0, 19, 0.0),
        (1, 0, 0.0), (1, 1, 1.0), (1, 2, 0.0), (1, 3, 0.0), (1, 4, 0.0),
        (1, 5, 0.0), (1, 6, 0.0), (1, 7, 0.0), (1, 8, 0.0), (1, 9, 0.0),
        (1, 10, 0.0), (1, 11, 0.0), (1, 12, 0.0), (1, 13, 0.0), (1, 14, 0.0),
        (1, 15, 0.0), (1, 16, 0.0), (1, 17, 0.0), (1, 18, 0.0), (1, 19, 0.0),
        (2, 0, 0.0), (2, 1, 1.0), (2, 2, 0.0), (2, 3, 0.0), (2, 4, 0.0),
        (2, 5, 0.0), (2, 6, 0.0), (2, 7, 0.0), (2, 8, 0.0), (2, 9, 0.0),
        (2, 10, 0.0), (2, 11, 0.0), (2, 12, 0.0), (2, 13, 0.0), (2, 14, 0.0),
        (2, 15, 0.0), (2, 16, 0.0), (2, 17, 0.0), (2, 18, 0.0), (2, 19, 0.0);
        nodes("curve_num_nodes_0"),
        nodes("curve_num_nodes_1"),
        nodes("curve_num_nodes_2"),
        interpolation("curve_type_0"),
        interpolation("curve_type_1"),
        interpolation("curve_type_2"),
        Param {
            name: "curve_autoscale",
            kind: Kind::Choice {
                default: 0,
                choices: &["linked_channels", "independent_channels"],
            },
        },
        Param {
            name: "compensate_middle_grey",
            kind: Kind::Bool { default: false },
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
    ],
};

/// The settings from resolved parameter values.
pub fn settings(params: &ParamValues) -> Settings {
    let curve = |c: usize| {
        let count = params.int(&format!("curve_num_nodes_{c}")) as usize;
        let node = |k: usize| Node {
            x: params.float(&format!("curve_nodes_{c}_{k}_x")),
            y: params.float(&format!("curve_nodes_{c}_{k}_y")),
        };
        let kind = params.choice(&format!("curve_type_{c}")) as i32;
        Curve {
            nodes: (0..count).map(node).collect(),
            interpolation: Interpolation::from_darktable(kind)
                .expect("the schema's choices are darktable's"),
        }
    };
    Settings {
        curves: [curve(0), curve(1), curve(2)],
        mode: Mode::from_darktable(params.choice("curve_autoscale") as i32)
            .expect("the schema's choices are darktable's"),
        compensate_middle_grey: params.flag("compensate_middle_grey"),
        preserve_colors: Norm::from_darktable(params.choice("preserve_colors") as i32)
            .expect("the schema's choices are darktable's"),
    }
}

impl Operation for RgbCurve {
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
            vec![prepared.knots()],
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
        assert_eq!(SCHEMA.params.len(), 3 * MAX_NODES * 2 + 9);
    }

    #[test]
    fn every_parameter_has_a_name_of_its_own() {
        let names: std::collections::HashSet<_> = SCHEMA.params.iter().map(|p| p.name).collect();
        assert_eq!(names.len(), SCHEMA.params.len(), "a name repeats");
        assert!(SCHEMA.param("curve_nodes_2_19_y").is_some());
    }

    #[test]
    fn every_parameter_reaches_the_settings() {
        // With all twenty nodes in use, moving any parameter off its default changes
        // the settings.
        let mut all_nodes = ParamValues::new();
        for c in 0..3 {
            all_nodes.set(&format!("curve_num_nodes_{c}"), Value::Int(20));
        }
        let base = settings(&SCHEMA.resolve(&all_nodes).unwrap());
        for param in SCHEMA.params {
            let moved = match param.kind {
                Kind::Float { default, .. } => Value::Float(if default < 0.5 { 0.5 } else { 0.25 }),
                Kind::Int { .. } => Value::Int(3),
                Kind::Choice { default, .. } => Value::Choice(if default == 0 { 1 } else { 0 }),
                Kind::Bool { default } => Value::Bool(!default),
            };
            let mut values = all_nodes.clone();
            values.set(param.name, moved);
            let changed = settings(&SCHEMA.resolve(&values).unwrap());
            assert_ne!(changed, base, "{} reaches no field", param.name);
        }
    }

    #[test]
    fn nodes_past_the_count_are_ignored() {
        let mut values = ParamValues::new();
        values.set("curve_nodes_0_2_x", Value::Float(0.7));
        assert_eq!(
            settings(&SCHEMA.resolve(&values).unwrap()),
            Settings::default()
        );
    }

    #[test]
    fn prepare_gives_one_per_pixel_pass_with_the_knot_table() {
        let prepared = RgbCurve
            .prepare(
                &SCHEMA.defaults(),
                &ImageContext::full(1, 1),
                &ImageDesc::rgba(1, 1),
            )
            .unwrap();
        assert_eq!(prepared.passes.len(), 1);
        let pass = &prepared.passes[0];
        assert_eq!(pass.params.len(), PARAMS_SIZE);
        assert_eq!(pass.tables.len(), 1);
        assert_eq!(pass.tables[0].len(), KNOTS_SIZE);
    }

    #[test]
    fn the_operation_is_differentiable() {
        // That its kernel agrees is the operations' own check.
        assert!(RgbCurve.manifest().differentiable);
    }

    #[test]
    fn nodes_out_of_order_are_not_ported() {
        // darktable cannot interpolate them and keeps whatever its table held before.
        let mut values = ParamValues::new();
        values.set("curve_num_nodes_0", Value::Int(3));
        values.set("curve_nodes_0_1_x", Value::Float(0.8));
        values.set("curve_nodes_0_2_x", Value::Float(0.4));
        let params = SCHEMA.resolve(&values).unwrap();
        let (image, input) = (ImageContext::full(1, 1), ImageDesc::rgba(1, 1));
        assert!(RgbCurve.prepare(&params, &image, &input).is_err());
    }
}
