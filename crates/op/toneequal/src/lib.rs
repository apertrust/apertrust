// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2018-2026  darktable developers (ported from src/iop/toneequal.c)
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

//! Tone equalizer, ported from darktable (`src/iop/toneequal.c`): nine bands of
//! exposure correction over a luminance mask, through a curve of eight Gaussians fitted
//! to them. The operation behind the Tone equalizer module (decision 2). See
//! `docs/ports/toneequal.md`.
//!
//! Every luminance estimator, with no mask filter (darktable's "preserve details: no"),
//! EIGF, darktable's default, or averaged EIGF, whose mask is built by whole-image passes
//! over the whole image at the pipe's scale (decision 15). The guided filters return
//! [`NotPorted`]. It is in `OPERATIONS` with `default_enabled` off: an unedited photo
//! does not run it, and the session plans it only while a module's binding sets one of
//! its parameters (decision 28).

pub mod filters;
mod setup;

use apertrust_plugin::{
    Domain, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation, Param,
    ParamValues, Provenance, Schema, Slot as PipelineSlot, Stage, Value,
};
pub use setup::*;

/// Generated from `slang/toneequal.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "toneequal",
    wgsl: include_str!("../generated/toneequal.wgsl"),
    derivatives: true,
};

pub struct ToneEqual;

static MANIFEST: Manifest = Manifest {
    id: "toneequal",
    version: 1,
    // darktable's v5.0 place: in camera RGB after exposure (21) and before the input
    // profile (28), so its mask sees what darktable's does (decision 22; docs/engine.md,
    // Order).
    slot: PipelineSlot {
        stage: Stage::CameraRgb,
        order: 24.0,
    },
    domain: Domain::Scene,
    layers: true,
    differentiable: true,
    // `main` reads only its own pixel; the mask's whole-image passes cover the whole
    // image in every pipe, an export's tiles in two rounds (decision 15).
    radius: 0,
    // Planned only while a module sets one of its parameters (decision 28).
    default_enabled: false,
    provenance: Some(Provenance {
        darktable_module: "toneequal",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &[
            "src/iop/toneequal.c",
            "src/common/luminance_mask.h",
            "src/common/eigf.h",
            "src/common/fast_guided_filter.h",
            "src/common/gaussian.c",
            "src/iop/choleski.h",
        ],
    }),
};

const fn band(name: &'static str) -> Param {
    Param {
        name,
        kind: Kind::Float {
            default: 0.0,
            min: -2.0,
            max: 2.0,
        },
    }
}

const fn float(name: &'static str, default: f32, min: f32, max: f32) -> Param {
    Param {
        name,
        kind: Kind::Float { default, min, max },
    }
}

/// darktable `dt_iop_toneequalizer_params_t` version 2, by its field names, with its
/// introspection's ranges and defaults. `smoothing` has none there; it takes the GUI
/// slider's.
static SCHEMA: Schema = Schema {
    version: 1,
    params: &[
        band("noise"),
        band("ultra_deep_blacks"),
        band("deep_blacks"),
        band("blacks"),
        band("shadows"),
        band("midtones"),
        band("highlights"),
        band("whites"),
        band("speculars"),
        float("blending", 5.0, 0.01, 100.0),
        float(
            "smoothing",
            std::f32::consts::SQRT_2,
            SMOOTHING_MIN,
            SMOOTHING_MAX,
        ),
        float("feathering", 1.0, 0.01, 10000.0),
        float("quantization", 0.0, 0.0, 2.0),
        float("contrast_boost", 0.0, -16.0, 16.0),
        float("exposure_boost", 0.0, -16.0, 16.0),
        Param {
            name: "details",
            kind: Kind::Choice {
                default: 4,
                choices: &[
                    "none",
                    "averaged_guided_filter",
                    "guided_filter",
                    "averaged_eigf",
                    "eigf",
                ],
            },
        },
        Param {
            name: "method",
            kind: Kind::Choice {
                default: 4,
                choices: &[
                    "rgb_average",
                    "hsl_lightness",
                    "hsv_value",
                    "rgb_sum",
                    "rgb_euclidean_norm",
                    "rgb_power_norm",
                    "rgb_geometric_mean",
                ],
            },
        },
        Param {
            name: "iterations",
            kind: Kind::Int {
                default: 1,
                min: 1,
                max: 20,
            },
        },
    ],
};

/// The bands' parameter names, -8 EV to 0 EV.
pub const BANDS: [&str; 9] = [
    "noise",
    "ultra_deep_blacks",
    "deep_blacks",
    "blacks",
    "shadows",
    "midtones",
    "highlights",
    "whites",
    "speculars",
];

/// Parameter values for darktable's settings, the inverse of [`settings`].
pub fn values(settings: &Settings) -> ParamValues {
    let mut values = ParamValues::new();
    for (name, &band) in BANDS.iter().zip(&settings.bands) {
        values.set(name, Value::Float(band));
    }
    for (name, value) in [
        ("blending", settings.blending),
        ("smoothing", settings.smoothing),
        ("feathering", settings.feathering),
        ("quantization", settings.quantization),
        ("contrast_boost", settings.contrast_boost),
        ("exposure_boost", settings.exposure_boost),
    ] {
        values.set(name, Value::Float(value));
    }
    values.set(
        "details",
        Value::Choice(settings.details.to_darktable() as u32),
    );
    values.set(
        "method",
        Value::Choice(settings.method.to_darktable() as u32),
    );
    values.set("iterations", Value::Int(i64::from(settings.iterations)));
    values
}

/// The settings from resolved parameter values.
pub fn settings(params: &ParamValues) -> Settings {
    Settings {
        bands: BANDS.map(|name| params.float(name)),
        blending: params.float("blending"),
        smoothing: params.float("smoothing"),
        feathering: params.float("feathering"),
        quantization: params.float("quantization"),
        contrast_boost: params.float("contrast_boost"),
        exposure_boost: params.float("exposure_boost"),
        details: Filter::from_darktable(params.choice("details") as i32)
            .expect("the schema's choices are darktable's"),
        method: Estimator::from_darktable(params.choice("method") as i32)
            .expect("the schema's choices are darktable's"),
        iterations: params.int("iterations") as i32,
    }
}

impl Operation for ToneEqual {
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
    ) -> Result<apertrust_plugin::Prepared, NotPorted> {
        let prepared = prepare(&settings(params), &geometry(image, input))?;
        Ok(apertrust_plugin::Prepared {
            output: *input,
            passes: prepared.passes(),
            scratch: prepared.scratch(),
        })
    }

    /// EIGF's grid samples are each computed by the tile that holds their north-west
    /// node, and `eigf_mask` writes the tile's own pixels (docs/ports/toneequal.md,
    /// Tiling). The guided filters' passes would have to own theirs too.
    fn tiles_whole_image_passes(&self, params: &ParamValues) -> bool {
        matches!(
            settings(params).details,
            Filter::None | Filter::Eigf | Filter::AveragedEigf
        )
    }
}

/// Where the operation runs. darktable's radius comes from `piece->iwidth` and
/// `iheight`, the pipe's input: for a raw its readout as decoded, before rawprepare's
/// crop. The mask covers the whole image at the pipe's scale, its input's stage
/// (decision 16), of which the input is all or, in a tile of an export, a part.
fn geometry(image: &ImageContext, input: &ImageDesc) -> Geometry {
    let full = image
        .camera
        .as_ref()
        .map_or([image.width, image.height], |camera| {
            [camera.width, camera.height]
        });
    Geometry {
        full,
        scale: image.scale,
        whole: image.whole_image(input),
        input: [input.width, input.height],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn per_pixel() -> ParamValues {
        let mut values = ParamValues::new();
        values.set("details", Value::Choice(0));
        SCHEMA.resolve(&values).unwrap()
    }

    #[test]
    fn schema_defaults_are_darktables() {
        assert_eq!(settings(&SCHEMA.defaults()), Settings::defaults());
        assert_eq!(SCHEMA.params.len(), 18);
    }

    #[test]
    fn every_parameter_reaches_the_settings() {
        let base = settings(&per_pixel());
        for param in SCHEMA.params {
            let moved = match param.kind {
                Kind::Float { default, max, .. } => Value::Float((default + max) / 2.0),
                Kind::Int { default, .. } => Value::Int(default + 1),
                Kind::Choice { choices, .. } => {
                    let current = per_pixel().choice(param.name);
                    Value::Choice((current + 1) % choices.len() as u32)
                }
                Kind::Bool { default } => Value::Bool(!default),
            };
            let mut values = per_pixel();
            values.set(param.name, moved);
            let changed = settings(&SCHEMA.resolve(&values).unwrap());
            assert_ne!(changed, base, "{} reaches no field", param.name);
        }
    }

    #[test]
    fn the_bands_are_in_darktables_order() {
        let mut values = per_pixel();
        for (i, name) in BANDS.iter().enumerate() {
            values.set(name, Value::Float(i as f32 / 10.0 - 0.4));
        }
        let bands = settings(&SCHEMA.resolve(&values).unwrap()).bands;
        assert_eq!(bands, std::array::from_fn(|i| i as f32 / 10.0 - 0.4));
    }

    #[test]
    fn values_round_trip_through_the_settings() {
        let mut settings = Settings::defaults();
        settings.bands = std::array::from_fn(|i| i as f32 / 10.0 - 0.4);
        settings.blending = 3.0;
        settings.quantization = 0.5;
        settings.iterations = 4;
        settings.details = Filter::AveragedEigf;
        settings.method = Estimator::PowerNorm;
        assert_eq!(
            super::settings(&SCHEMA.resolve(&values(&settings)).unwrap()),
            settings
        );
    }

    #[test]
    fn prepare_gives_main_alone_without_a_filter() {
        let prepared = ToneEqual
            .prepare(
                &per_pixel(),
                &ImageContext::full(1, 1),
                &ImageDesc::rgba(1, 1),
            )
            .unwrap();
        assert_eq!(prepared.passes.len(), 1);
        let pass = &prepared.passes[0];
        assert_eq!(pass.entry_point, "main");
        assert_eq!(pass.params.len(), PARAMS_SIZE);
        assert!(pass.tables.is_empty());
    }

    #[test]
    fn darktables_default_eigf_prepares_its_whole_image_passes() {
        let (image, input) = (ImageContext::full(64, 48), ImageDesc::rgba(64, 48));
        let prepared = ToneEqual
            .prepare(&SCHEMA.defaults(), &image, &input)
            .unwrap();
        assert_eq!(prepared.passes.len(), 6);
        assert_eq!(prepared.passes[0].entry_point, "eigf_downsample");
        assert_eq!(prepared.output, input);
    }

    #[test]
    fn the_guided_filters_are_not_ported() {
        let (image, input) = (ImageContext::full(1, 1), ImageDesc::rgba(1, 1));
        for details in [1, 2] {
            let mut values = per_pixel();
            values.set("details", Value::Choice(details));
            let params = SCHEMA.resolve(&values).unwrap();
            assert!(ToneEqual.prepare(&params, &image, &input).is_err());
        }
    }

    #[test]
    fn a_tile_takes_the_whole_image_for_its_grid() {
        // A tile at (64, 32) of its stage, as the planner describes it.
        let image = ImageContext::full(200, 100);
        let input = ImageDesc {
            origin: [64, 32],
            extent: [200, 100],
            scaled_extent: [200, 100],
            ..ImageDesc::rgba(41, 21)
        };
        let tile = geometry(&image, &input);
        assert_eq!(tile.whole, [200, 100]);
        assert_eq!(tile.input, [41, 21]);
        let whole = geometry(&image, &ImageDesc::rgba(200, 100));
        assert_eq!(whole.whole, [200, 100]);
        assert_eq!(whole.full, [200, 100]);
        // After a quarter turn its stage is the turned image (decision 16); the radius
        // is still measured on the pipe's input.
        let turned = geometry(
            &image,
            &ImageDesc {
                extent: [100, 200],
                scaled_extent: [100, 200],
                ..input
            },
        );
        assert_eq!((turned.whole, turned.full), ([100, 200], [200, 100]));
    }

    #[test]
    fn the_radius_takes_a_raws_readout() {
        // darktable's piece->iwidth is the pipe's input, the raw before its crop:
        // mire1.cr2's 3944 × 2622 for a developed image of 3908 × 2602.
        let camera = apertrust_plugin::Camera {
            make: String::new(),
            model: String::new(),
            alias: String::new(),
            width: 3944,
            height: 2622,
            cfa: None,
            black: apertrust_plugin::Pattern {
                width: 1,
                height: 1,
                values: vec![0.0],
            },
            white: 1.0,
            white_balance: None,
            xyz_to_camera: None,
            crop: [30, 18, 3908, 2602],
            declared: [30, 18, 3908, 2602],
            exposure_bias: 0.0,
            iso: 0.0,
            linear_response_limit: 1.0,
            lens_correction: None,
            orientation: 0,
        };
        let image = ImageContext {
            camera: Some(camera),
            ..ImageContext::full(3908, 2602)
        };
        let geometry = geometry(&image, &ImageDesc::rgba(3908, 2602));
        assert_eq!(geometry.full, [3944, 2622]);
        assert_eq!(geometry.whole, [3908, 2602]);
    }

    #[test]
    fn the_operation_is_differentiable() {
        // That its kernel agrees is the operations' own check.
        assert!(ToneEqual.manifest().differentiable);
    }
}
