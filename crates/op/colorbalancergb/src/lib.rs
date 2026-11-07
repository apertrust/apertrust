// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2020-2026  darktable developers (ported from src/iop/colorbalancergb.c)
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

//! Color balance rgb, ported from darktable (`src/iop/colorbalancergb.c`), with
//! both saturation formulas: darktable UCS, darktable's default, and JzAzBz. In the first
//! pass the Colour balance RGB module, darktable's, drives it and owns every parameter
//! (decision 2). See `docs/ports/colorbalancergb.md`.
//!
//! It is in `OPERATIONS` with `default_enabled` off: an unedited photo does not run
//! it, and the session plans it only while a module's binding sets one of its
//! parameters (decisions 28 and 32).

#![allow(
    clippy::excessive_precision,
    reason = "defaults are copied digit for digit from darktable"
)]

mod setup;

use apertrust_color::WorkingProfile;
use apertrust_plugin::{
    Domain, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation, Param,
    ParamValues, Provenance, Schema, Slot, Stage, Value,
};
pub use setup::*;

/// Generated from `slang/colorbalancergb.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "colorbalancergb",
    wgsl: include_str!("../generated/colorbalancergb.wgsl"),
    derivatives: true,
};

pub struct ColorBalanceRgb;

static MANIFEST: Manifest = Manifest {
    id: "colorbalancergb",
    version: 1,
    // darktable's v5.0 place: after the input profile and color calibration, before RGB
    // curve, RGB levels and the tone mapper (decision 22).
    slot: Slot {
        stage: Stage::WorkingSpace,
        order: 41.5,
    },
    domain: Domain::Scene,
    layers: true,
    differentiable: true,
    radius: 0,
    // Planned only while a module sets one of its parameters (decision 28).
    default_enabled: false,
    provenance: Some(Provenance {
        darktable_module: "colorbalancergb",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &[
            "src/iop/colorbalancergb.c",
            "src/common/colorspaces_inline_conversions.h",
            "src/common/chromatic_adaptation.h",
            "src/common/math.h",
            "src/common/darktable_ucs_22_helpers.h",
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

/// A luminance or chroma offset, darktable's -1 to 1, default 0.
const fn offset(name: &'static str) -> Param {
    float(name, 0.0, -1.0, 1.0)
}

/// A colour wheel's chroma, 0 to 1, and hue, 0° to 360°.
const fn chroma(name: &'static str) -> Param {
    float(name, 0.0, 0.0, 1.0)
}

const fn hue(name: &'static str) -> Param {
    float(name, 0.0, 0.0, 360.0)
}

/// darktable `dt_iop_colorbalancergb_params_t` version 5, by its field names, in its
/// order, with its ranges and defaults: `saturation_formula` defaults to darktable UCS,
/// as a new instance in darktable does (decision 22).
static SCHEMA: Schema = Schema {
    version: 1,
    params: &[
        offset("shadows_Y"),
        chroma("shadows_C"),
        hue("shadows_H"),
        offset("midtones_Y"),
        chroma("midtones_C"),
        hue("midtones_H"),
        offset("highlights_Y"),
        chroma("highlights_C"),
        hue("highlights_H"),
        offset("global_Y"),
        chroma("global_C"),
        hue("global_H"),
        float("shadows_weight", 1.0, 0.0, 3.0),
        float("white_fulcrum", 0.0, -16.0, 16.0),
        float("highlights_weight", 1.0, 0.0, 3.0),
        offset("chroma_shadows"),
        offset("chroma_highlights"),
        offset("chroma_global"),
        offset("chroma_midtones"),
        offset("saturation_global"),
        offset("saturation_highlights"),
        offset("saturation_midtones"),
        offset("saturation_shadows"),
        float("hue_angle", 0.0, -180.0, 180.0),
        offset("brilliance_global"),
        offset("brilliance_highlights"),
        offset("brilliance_midtones"),
        offset("brilliance_shadows"),
        float("mask_grey_fulcrum", 0.1845, 0.0, 1.0),
        offset("vibrance"),
        float("grey_fulcrum", 0.1845, 0.0, 1.0),
        offset("contrast"),
        Param {
            name: "saturation_formula",
            kind: Kind::Choice {
                default: 1,
                choices: &["jzazbz", "ucs"],
            },
        },
    ],
};

/// The settings from resolved parameter values.
pub fn settings(params: &ParamValues) -> Settings {
    let f = |name: &str| params.float(name);
    Settings {
        shadows_y: f("shadows_Y"),
        shadows_c: f("shadows_C"),
        shadows_h: f("shadows_H"),
        midtones_y: f("midtones_Y"),
        midtones_c: f("midtones_C"),
        midtones_h: f("midtones_H"),
        highlights_y: f("highlights_Y"),
        highlights_c: f("highlights_C"),
        highlights_h: f("highlights_H"),
        global_y: f("global_Y"),
        global_c: f("global_C"),
        global_h: f("global_H"),
        shadows_weight: f("shadows_weight"),
        white_fulcrum: f("white_fulcrum"),
        highlights_weight: f("highlights_weight"),
        chroma_shadows: f("chroma_shadows"),
        chroma_highlights: f("chroma_highlights"),
        chroma_global: f("chroma_global"),
        chroma_midtones: f("chroma_midtones"),
        saturation_global: f("saturation_global"),
        saturation_highlights: f("saturation_highlights"),
        saturation_midtones: f("saturation_midtones"),
        saturation_shadows: f("saturation_shadows"),
        hue_angle: f("hue_angle"),
        brilliance_global: f("brilliance_global"),
        brilliance_highlights: f("brilliance_highlights"),
        brilliance_midtones: f("brilliance_midtones"),
        brilliance_shadows: f("brilliance_shadows"),
        mask_grey_fulcrum: f("mask_grey_fulcrum"),
        vibrance: f("vibrance"),
        grey_fulcrum: f("grey_fulcrum"),
        contrast: f("contrast"),
        saturation_formula: SaturationFormula::from_darktable(params.choice("saturation_formula"))
            .expect("the schema's choices are darktable's"),
    }
}

/// The parameter values for these settings, by the schema's names: the inverse of
/// [`settings`]. They still need checking against the schema ([`Schema::resolve`]).
pub fn values(settings: &Settings) -> ParamValues {
    let s = settings;
    let mut values = ParamValues::new();
    for (name, value) in [
        ("shadows_Y", s.shadows_y),
        ("shadows_C", s.shadows_c),
        ("shadows_H", s.shadows_h),
        ("midtones_Y", s.midtones_y),
        ("midtones_C", s.midtones_c),
        ("midtones_H", s.midtones_h),
        ("highlights_Y", s.highlights_y),
        ("highlights_C", s.highlights_c),
        ("highlights_H", s.highlights_h),
        ("global_Y", s.global_y),
        ("global_C", s.global_c),
        ("global_H", s.global_h),
        ("shadows_weight", s.shadows_weight),
        ("white_fulcrum", s.white_fulcrum),
        ("highlights_weight", s.highlights_weight),
        ("chroma_shadows", s.chroma_shadows),
        ("chroma_highlights", s.chroma_highlights),
        ("chroma_global", s.chroma_global),
        ("chroma_midtones", s.chroma_midtones),
        ("saturation_global", s.saturation_global),
        ("saturation_highlights", s.saturation_highlights),
        ("saturation_midtones", s.saturation_midtones),
        ("saturation_shadows", s.saturation_shadows),
        ("hue_angle", s.hue_angle),
        ("brilliance_global", s.brilliance_global),
        ("brilliance_highlights", s.brilliance_highlights),
        ("brilliance_midtones", s.brilliance_midtones),
        ("brilliance_shadows", s.brilliance_shadows),
        ("mask_grey_fulcrum", s.mask_grey_fulcrum),
        ("vibrance", s.vibrance),
        ("grey_fulcrum", s.grey_fulcrum),
        ("contrast", s.contrast),
    ] {
        values.set(name, Value::Float(value));
    }
    values.set(
        "saturation_formula",
        Value::Choice(s.saturation_formula.to_darktable()),
    );
    values
}

impl Operation for ColorBalanceRgb {
    fn manifest(&self) -> &'static Manifest {
        &MANIFEST
    }

    fn schema(&self) -> &'static Schema {
        &SCHEMA
    }

    fn kernel(&self) -> &'static Kernel {
        &KERNEL
    }

    /// darktable's `commit_params`. The gamut LUT is the working space's, built once
    /// ([`gamut_lut`]), so a call costs microseconds after the first.
    fn prepare(
        &self,
        params: &ParamValues,
        _image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<apertrust_plugin::Prepared, NotPorted> {
        // Apertrust's working space is fixed: linear Rec2020.
        let prepared = prepare(&settings(params), &WorkingProfile::linear_rec2020())?;
        let table = prepared.table();
        Ok(apertrust_plugin::Prepared::per_pixel(
            input,
            prepared.params,
            vec![table],
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prepare_values(values: &ParamValues) -> Result<apertrust_plugin::Prepared, NotPorted> {
        let params = SCHEMA.resolve(values).unwrap();
        ColorBalanceRgb.prepare(&params, &ImageContext::full(1, 1), &ImageDesc::rgba(1, 1))
    }

    #[test]
    fn schema_defaults_are_darktables_with_the_ucs_formula() {
        let defaults = settings(&SCHEMA.defaults());
        assert_eq!(defaults, Settings::default());
        assert_eq!(defaults.saturation_formula, SaturationFormula::Ucs);
    }

    #[test]
    fn the_schema_is_darktables_version_5_in_order() {
        // 33 fields: 32 floats and the saturation formula, in the struct's order.
        assert_eq!(SCHEMA.params.len(), 33);
        assert_eq!(SCHEMA.params[0].name, "shadows_Y");
        assert_eq!(SCHEMA.params[23].name, "hue_angle");
        assert_eq!(SCHEMA.params[28].name, "mask_grey_fulcrum");
        assert_eq!(SCHEMA.params[31].name, "contrast");
        assert_eq!(SCHEMA.params[32].name, "saturation_formula");
        let names: std::collections::HashSet<_> = SCHEMA.params.iter().map(|p| p.name).collect();
        assert_eq!(names.len(), SCHEMA.params.len(), "a name repeats");
    }

    #[test]
    fn the_contrast_controls_have_darktables_ranges() {
        // What Exposure's contrast and pivot bind to.
        let range = |name: &str| match SCHEMA.param(name).unwrap().kind {
            Kind::Float { default, min, max } => (default, min, max),
            other => panic!("{name} is {other:?}"),
        };
        assert_eq!(range("contrast"), (0.0, -1.0, 1.0));
        assert_eq!(range("grey_fulcrum"), (0.1845, 0.0, 1.0));
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
                Kind::Choice { default, .. } => Value::Choice(1 - default),
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
    fn prepare_gives_one_per_pixel_pass_with_the_gamut_lut() {
        let prepared = prepare_values(&ParamValues::new()).unwrap();
        assert_eq!(prepared.output, ImageDesc::rgba(1, 1));
        assert_eq!(prepared.passes.len(), 1);
        let pass = &prepared.passes[0];
        assert_eq!(pass.entry_point, "main");
        assert_eq!(pass.params.len(), PARAMS_SIZE);
        assert_eq!(pass.tables.len(), 1);
        let lut = gamut_lut(&WorkingProfile::linear_rec2020(), SaturationFormula::Ucs);
        assert_eq!(
            pass.tables[0],
            lut.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn prepare_is_the_setups_prepare() {
        let mut values = ParamValues::new();
        values.set("contrast", Value::Float(0.3));
        values.set("grey_fulcrum", Value::Float(0.25));
        values.set("chroma_global", Value::Float(-0.2));
        let prepared = prepare_values(&values).unwrap();
        let setup = prepare(
            &settings(&SCHEMA.resolve(&values).unwrap()),
            &WorkingProfile::linear_rec2020(),
        )
        .unwrap();
        assert_eq!(prepared.passes[0].params, setup.params);
        assert_eq!(prepared.passes[0].tables, vec![setup.table()]);
    }

    #[test]
    fn each_formula_gets_its_own_gamut_lut() {
        let profile = WorkingProfile::linear_rec2020();
        for formula in [SaturationFormula::JzAzBz, SaturationFormula::Ucs] {
            let mut values = ParamValues::new();
            values.set("saturation_formula", Value::Choice(formula.to_darktable()));
            let prepared = prepare_values(&values).unwrap();
            let lut = gamut_lut(&profile, formula);
            assert_eq!(
                prepared.passes[0].tables[0],
                lut.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>(),
                "{formula:?}"
            );
        }
    }

    #[test]
    fn the_operation_is_differentiable_and_per_pixel() {
        // That its kernel agrees, and its slot, are the operations' own checks.
        let manifest = ColorBalanceRgb.manifest();
        assert!(manifest.differentiable);
        assert_eq!(manifest.radius, 0);
    }
}
