// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2019-2026  darktable developers (ported from src/iop/filmicrgb.c)
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

//! Filmic rgb, the third of darktable's scene-referred tone mappers beside AgX and
//! sigmoid, ported from darktable (`src/iop/filmicrgb.c`): its five colour
//! sciences, every norm, the three curves of its toe and shoulder, and its highlight
//! reconstruction, whole-image passes of wavelets before the per-pixel `main`. See
//! `docs/ports/filmicrgb.md`.
//!
//! It is in `OPERATIONS` with `default_enabled` off: nothing plans it until an edit can
//! choose it as its tone mapper (decision 2).

mod setup;

use apertrust_color::WorkingProfile;
use apertrust_plugin::{
    Domain, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation, Param,
    ParamValues, Prepared, Provenance, Schema, Slot, Stage, Value,
};
pub use setup::*;

/// Generated from `slang/filmicrgb.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "filmicrgb",
    wgsl: include_str!("../generated/filmicrgb.wgsl"),
    derivatives: true,
};

pub struct FilmicRgb;

static MANIFEST: Manifest = Manifest {
    id: "filmicrgb",
    version: 1,
    // darktable's v5.0 place: after AgX (45.5), the last tone mapper, before lut3d and
    // colisa (47).
    slot: Slot {
        stage: Stage::WorkingSpace,
        order: 46.0,
    },
    domain: Domain::Scene,
    // Background only: the tone mapper's parameters are global (decision 33).
    layers: false,
    differentiable: true,
    // The reconstruction's passes see the whole image and `main` reads only its own
    // pixel and their scratch, so a tile needs no border.
    radius: 0,
    // Planned only while an edit chooses it as its tone mapper (decision 2).
    default_enabled: false,
    provenance: Some(Provenance {
        darktable_module: "filmicrgb",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &[
            "src/iop/filmicrgb.c",
            "src/common/bspline.h",
            "src/develop/noise_generator.h",
            "src/iop/gaussian_elimination.h",
            "src/common/gamut_mapping.h",
            "src/common/colorspaces_inline_conversions.h",
            "src/common/iop_profile.h",
            "src/common/math.h",
        ],
    }),
};

const fn float(name: &'static str, default: f32, min: f32, max: f32) -> Param {
    Param {
        name,
        kind: Kind::Float { default, min, max },
    }
}

const fn choice(name: &'static str, default: u32, choices: &'static [&'static str]) -> Param {
    Param {
        name,
        kind: Kind::Choice { default, choices },
    }
}

const fn flag(name: &'static str, default: bool) -> Param {
    Param {
        name,
        kind: Kind::Bool { default },
    }
}

/// darktable `dt_iop_filmicrgb_params_t` version 6, by its field names, with its
/// introspection's ranges and defaults (decision 21).
static SCHEMA: Schema = Schema {
    version: 1,
    params: &[
        float("grey_point_source", 18.45, 0.0, 100.0),
        float("black_point_source", -8.0, -16.0, -0.1),
        float("white_point_source", 4.0, 0.1, 16.0),
        float("reconstruct_threshold", 0.0, -6.0, 6.0),
        float("reconstruct_feather", 3.0, 0.25, 6.0),
        float("reconstruct_bloom_vs_details", 100.0, -100.0, 100.0),
        float("reconstruct_grey_vs_color", 100.0, -100.0, 100.0),
        float("reconstruct_structure_vs_texture", 0.0, -100.0, 100.0),
        float("security_factor", 0.0, -50.0, 200.0),
        float("grey_point_target", 18.45, 1.0, 50.0),
        float("black_point_target", 0.01517634, 0.0, 20.0),
        float("white_point_target", 100.0, 0.0, 1600.0),
        float("output_power", 4.0, 1.0, 10.0),
        float("latitude", 0.01, 0.01, 99.0),
        float("contrast", 1.0, 0.0, 5.0),
        float("saturation", 0.0, -200.0, 200.0),
        float("balance", 0.0, -50.0, 50.0),
        float("noise_level", 0.2, 0.0, 6.0),
        choice("preserve_color", Norm::PowerNorm as u32, &Norm::NAMES),
        choice("version", ColorScience::V7 as u32, &ColorScience::NAMES),
        flag("auto_hardness", true),
        flag("custom_grey", false),
        Param {
            name: "high_quality_reconstruction",
            kind: Kind::Int {
                default: 1,
                min: 0,
                max: 10,
            },
        },
        choice("noise_distribution", Noise::Gaussian as u32, &Noise::NAMES),
        choice("shadows", CurveType::Hard as u32, &CurveType::NAMES),
        choice("highlights", CurveType::Hard as u32, &CurveType::NAMES),
        flag("compensate_icc_black", false),
        choice(
            "spline_version",
            SplineVersion::V3 as u32,
            &SplineVersion::NAMES,
        ),
        flag("enable_highlight_reconstruction", false),
    ],
};

/// The settings from resolved parameter values.
pub fn settings(params: &ParamValues) -> Settings {
    let defaults = Settings::default();
    Settings {
        grey_point_source: params.float("grey_point_source"),
        black_point_source: params.float("black_point_source"),
        white_point_source: params.float("white_point_source"),
        reconstruct_threshold: params.float("reconstruct_threshold"),
        reconstruct_feather: params.float("reconstruct_feather"),
        reconstruct_bloom_vs_details: params.float("reconstruct_bloom_vs_details"),
        reconstruct_grey_vs_color: params.float("reconstruct_grey_vs_color"),
        reconstruct_structure_vs_texture: params.float("reconstruct_structure_vs_texture"),
        security_factor: params.float("security_factor"),
        grey_point_target: params.float("grey_point_target"),
        black_point_target: params.float("black_point_target"),
        white_point_target: params.float("white_point_target"),
        output_power: params.float("output_power"),
        latitude: params.float("latitude"),
        contrast: params.float("contrast"),
        saturation: params.float("saturation"),
        balance: params.float("balance"),
        noise_level: params.float("noise_level"),
        preserve_color: Norm::from_index(params.choice("preserve_color"))
            .unwrap_or(defaults.preserve_color),
        version: ColorScience::from_index(params.choice("version")).unwrap_or(defaults.version),
        auto_hardness: params.flag("auto_hardness"),
        custom_grey: params.flag("custom_grey"),
        high_quality_reconstruction: params.int("high_quality_reconstruction").clamp(0, 10) as u32,
        noise_distribution: Noise::from_index(params.choice("noise_distribution"))
            .unwrap_or(defaults.noise_distribution),
        shadows: CurveType::from_index(params.choice("shadows")).unwrap_or(defaults.shadows),
        highlights: CurveType::from_index(params.choice("highlights"))
            .unwrap_or(defaults.highlights),
        compensate_icc_black: params.flag("compensate_icc_black"),
        spline_version: SplineVersion::from_index(params.choice("spline_version"))
            .unwrap_or(defaults.spline_version),
        enable_highlight_reconstruction: params.flag("enable_highlight_reconstruction"),
    }
}

/// The parameter values of `settings`, as an edit or the golden harness sets them.
pub fn values(settings: &Settings) -> ParamValues {
    let mut values = ParamValues::new();
    let floats = [
        ("grey_point_source", settings.grey_point_source),
        ("black_point_source", settings.black_point_source),
        ("white_point_source", settings.white_point_source),
        ("reconstruct_threshold", settings.reconstruct_threshold),
        ("reconstruct_feather", settings.reconstruct_feather),
        (
            "reconstruct_bloom_vs_details",
            settings.reconstruct_bloom_vs_details,
        ),
        (
            "reconstruct_grey_vs_color",
            settings.reconstruct_grey_vs_color,
        ),
        (
            "reconstruct_structure_vs_texture",
            settings.reconstruct_structure_vs_texture,
        ),
        ("security_factor", settings.security_factor),
        ("grey_point_target", settings.grey_point_target),
        ("black_point_target", settings.black_point_target),
        ("white_point_target", settings.white_point_target),
        ("output_power", settings.output_power),
        ("latitude", settings.latitude),
        ("contrast", settings.contrast),
        ("saturation", settings.saturation),
        ("balance", settings.balance),
        ("noise_level", settings.noise_level),
    ];
    for (name, value) in floats {
        values.set(name, Value::Float(value));
    }
    let choices = [
        ("preserve_color", settings.preserve_color as u32),
        ("version", settings.version as u32),
        ("noise_distribution", settings.noise_distribution as u32),
        ("shadows", settings.shadows as u32),
        ("highlights", settings.highlights as u32),
        ("spline_version", settings.spline_version as u32),
    ];
    for (name, value) in choices {
        values.set(name, Value::Choice(value));
    }
    let flags = [
        ("auto_hardness", settings.auto_hardness),
        ("custom_grey", settings.custom_grey),
        ("compensate_icc_black", settings.compensate_icc_black),
        (
            "enable_highlight_reconstruction",
            settings.enable_highlight_reconstruction,
        ),
    ];
    for (name, value) in flags {
        values.set(name, Value::Bool(value));
    }
    values.set(
        "high_quality_reconstruction",
        Value::Int(i64::from(settings.high_quality_reconstruction)),
    );
    values
}

impl Operation for FilmicRgb {
    fn manifest(&self) -> &'static Manifest {
        &MANIFEST
    }

    fn schema(&self) -> &'static Schema {
        &SCHEMA
    }

    fn kernel(&self) -> &'static Kernel {
        &KERNEL
    }

    /// darktable `reload_defaults`: on a raw with colour, the exposures moved by the
    /// camera's exposure bias and the hardness from them.
    fn defaults(&self, image: &ImageContext) -> ParamValues {
        match &image.camera {
            Some(camera) if camera.cfa.is_some() => {
                values(&Settings::default().for_raw(camera.exposure_bias))
            }
            _ => SCHEMA.defaults(),
        }
    }

    fn prepare(
        &self,
        params: &ParamValues,
        image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<Prepared, NotPorted> {
        let profile = WorkingProfile::linear_rec2020();
        // The image context carries the pipe's output profile (decision 26), but filmic
        // does not read it yet: v6 and v7 gamut-map against the working space, as
        // darktable does for a linear Rec2020 export (docs/ports/filmicrgb.md,
        // Deliberate differences).
        prepare(
            &settings(params),
            input,
            Geometry {
                size: image.width.max(image.height),
                scale: image.scale,
            },
            &profile,
            &profile,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apertrust_plugin::{Camera, Pattern};

    #[test]
    fn schema_defaults_are_darktables() {
        assert_eq!(settings(&SCHEMA.defaults()), Settings::default());
        let names: Vec<&str> = SCHEMA.params.iter().map(|p| p.name).collect();
        assert_eq!(
            names,
            [
                "grey_point_source",
                "black_point_source",
                "white_point_source",
                "reconstruct_threshold",
                "reconstruct_feather",
                "reconstruct_bloom_vs_details",
                "reconstruct_grey_vs_color",
                "reconstruct_structure_vs_texture",
                "security_factor",
                "grey_point_target",
                "black_point_target",
                "white_point_target",
                "output_power",
                "latitude",
                "contrast",
                "saturation",
                "balance",
                "noise_level",
                "preserve_color",
                "version",
                "auto_hardness",
                "custom_grey",
                "high_quality_reconstruction",
                "noise_distribution",
                "shadows",
                "highlights",
                "compensate_icc_black",
                "spline_version",
                "enable_highlight_reconstruction",
            ]
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
                Kind::Choice { default, .. } => Value::Choice(if default == 0 { 1 } else { 0 }),
                Kind::Bool { default } => Value::Bool(!default),
                Kind::Int { default, min, max } => {
                    Value::Int(if default == min { max } else { min })
                }
            };
            let mut values = ParamValues::new();
            values.set(param.name, moved);
            let changed = settings(&SCHEMA.resolve(&values).unwrap());
            assert_ne!(changed, base, "{} reaches no field", param.name);
        }
    }

    #[test]
    fn values_round_trip_through_the_schema() {
        let settings = Settings {
            version: ColorScience::V5,
            preserve_color: Norm::Euclidean,
            shadows: CurveType::Safe,
            highlights: CurveType::Soft,
            noise_distribution: Noise::Poissonian,
            high_quality_reconstruction: 6,
            enable_highlight_reconstruction: true,
            custom_grey: true,
            contrast: 1.35,
            ..Settings::default()
        };
        let resolved = SCHEMA.resolve(&values(&settings)).unwrap();
        assert_eq!(super::settings(&resolved), settings);
    }

    fn raw(bias: f32, colour: bool) -> ImageContext {
        let pattern = |values| Pattern {
            width: 1,
            height: 1,
            values,
        };
        ImageContext {
            camera: Some(Camera {
                make: String::new(),
                model: String::new(),
                alias: String::new(),
                width: 8,
                height: 8,
                cfa: colour.then(|| pattern(vec![0])),
                black: Pattern {
                    width: 1,
                    height: 1,
                    values: vec![0.0],
                },
                white: 1.0,
                white_balance: None,
                xyz_to_camera: None,
                crop: [0, 0, 8, 8],
                declared: [0, 0, 8, 8],
                exposure_bias: bias,
                linear_response_limit: 1.0,
                lens_correction: None,
                orientation: 0,
                iso: 0.0,
            }),
            ..ImageContext::full(8, 8)
        }
    }

    #[test]
    fn defaults_follow_darktables_reload_defaults() {
        let on_raw = settings(&FilmicRgb.defaults(&raw(0.0, true)));
        assert_eq!(on_raw.black_point_source, -7.65);
        assert_eq!(on_raw.white_point_source, 4.56);
        assert_eq!(on_raw.output_power, 3.614_816_2);
        let biased = settings(&FilmicRgb.defaults(&raw(-1.0, true)));
        assert_eq!(biased.white_point_source, 4.0 + 0.8 * 1.7);
        for image in [raw(0.0, false), ImageContext::full(8, 8)] {
            assert_eq!(settings(&FilmicRgb.defaults(&image)), Settings::default());
        }
    }

    #[test]
    fn prepare_uses_the_images_longer_side_and_the_pipes_scale() {
        let mut values = SCHEMA.defaults();
        values.set("enable_highlight_reconstruction", Value::Bool(true));
        let input = ImageDesc::rgba(977, 650);
        let image = ImageContext {
            scale: 0.25,
            ..ImageContext::full(3908, 2602)
        };
        let ours = FilmicRgb.prepare(&values, &image, &input).unwrap();
        let profile = WorkingProfile::linear_rec2020();
        let direct = prepare(
            &settings(&values),
            &input,
            Geometry {
                size: 3908,
                scale: 0.25,
            },
            &profile,
            &profile,
        )
        .unwrap();
        assert_eq!(ours, direct);
        // 6 scales at a quarter: 3 + (1 + 24) + (2 + 1 + 24) + 1.
        assert_eq!(ours.passes.len(), 3 + 25 + 27 + 1);
    }

    #[test]
    fn the_operation_is_differentiable() {
        // That its kernel agrees is the operations' own check.
        assert!(FilmicRgb.manifest().differentiable);
    }
}
