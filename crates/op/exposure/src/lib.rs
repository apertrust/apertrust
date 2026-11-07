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

//! Exposure, ported from darktable (`src/iop/exposure.c`): manual mode, with
//! the camera's exposure bias compensated. Deflicker needs the raw histogram, and
//! highlight-preservation compensation makernote fields rawler does not read, so
//! neither is ported yet: their parameters are in the schema, as darktable's are, and
//! any value but the one rendered is refused. The kernel computes its scale from the
//! black level and the exposure, so the operation's setup only packs them.

use apertrust_plugin::{
    Domain, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation, Param,
    ParamValues, Prepared, Provenance, Schema, Slot, Stage, Value,
};

/// Generated from `slang/exposure.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "exposure",
    wgsl: include_str!("../generated/exposure.wgsl"),
    derivatives: true,
};

pub struct Exposure;

static MANIFEST: Manifest = Manifest {
    id: "exposure",
    version: 1,
    slot: Slot {
        stage: Stage::CameraRgb,
        order: 21.0,
    },
    domain: Domain::Scene,
    layers: true,
    differentiable: true,
    radius: 0,
    default_enabled: true,
    provenance: Some(Provenance {
        darktable_module: "exposure",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &["src/iop/exposure.c", "data/kernels/basic.cl"],
    }),
};

/// darktable's modes, `dt_iop_exposure_mode_t`: manual, and automatic, its deflicker.
const MODES: [&str; 2] = ["manual", "automatic"];
const MANUAL: u32 = 0;

/// The deflicker's percentile and target level at darktable's defaults, the only values
/// ported: they act only in automatic mode.
const PERCENTILE: f32 = 50.0;
const TARGET_LEVEL: f32 = -4.0;

/// darktable `dt_iop_exposure_params_t`, in its order, with its ranges and its
/// scene-referred defaults for a colour raw (`reload_defaults`), but for
/// highlight-preservation compensation, which darktable turns on for a new instance and
/// Apertrust leaves off, as the whole-pipeline test does on darktable's side (decisions
/// 21 and 62). Version 2 adds the mode, the deflicker's settings and that compensation,
/// each at the value version 1 rendered, so a version 1 edit renders as before.
static SCHEMA: Schema = Schema {
    version: 2,
    params: &[
        Param {
            name: "mode",
            kind: Kind::Choice {
                default: MANUAL,
                choices: &MODES,
            },
        },
        Param {
            name: "black",
            kind: Kind::Float {
                // darktable's -0.000244140625, exactly -2^-12.
                default: -1.0 / 4096.0,
                min: -1.0,
                max: 1.0,
            },
        },
        Param {
            name: "exposure",
            kind: Kind::Float {
                default: 0.7,
                min: -18.0,
                max: 18.0,
            },
        },
        Param {
            name: "deflicker_percentile",
            kind: Kind::Float {
                default: PERCENTILE,
                min: 0.0,
                max: 100.0,
            },
        },
        Param {
            name: "deflicker_target_level",
            kind: Kind::Float {
                default: TARGET_LEVEL,
                min: -18.0,
                max: 18.0,
            },
        },
        Param {
            name: "compensate_exposure_bias",
            kind: Kind::Bool { default: true },
        },
        Param {
            name: "compensate_hilite_pres",
            kind: Kind::Bool { default: false },
        },
    ],
};

/// What of `params` is not ported, if anything: automatic mode, the deflicker's settings
/// off darktable's defaults, and highlight-preservation compensation.
fn not_ported(params: &ParamValues) -> Option<&'static str> {
    if params.choice("mode") != MANUAL {
        Some("automatic mode, darktable's deflicker, which needs the raw histogram")
    } else if params.float("deflicker_percentile") != PERCENTILE
        || params.float("deflicker_target_level") != TARGET_LEVEL
    {
        Some("deflicker settings other than darktable's defaults")
    } else if params.flag("compensate_hilite_pres") {
        Some("highlight-preservation compensation, which needs maker notes rawler does not read")
    } else {
        None
    }
}

/// darktable `_get_exposure_bias`: the camera's exposure compensation, trusted only
/// within ±5 EV.
fn exposure_bias(image: &ImageContext) -> f32 {
    image
        .camera
        .as_ref()
        .map_or(0.0, |camera| camera.exposure_bias.clamp(-5.0, 5.0))
}

/// The exposure the kernel applies, as darktable's `commit_params` computes it: the
/// parameter, less the camera's exposure bias when it is compensated. darktable's
/// highlight-preservation compensation is not ported.
pub fn effective_exposure(params: &ParamValues, image: &ImageContext) -> f32 {
    let exposure = params.float("exposure");
    if params.flag("compensate_exposure_bias") {
        exposure - exposure_bias(image)
    } else {
        exposure
    }
}

impl Operation for Exposure {
    fn manifest(&self) -> &'static Manifest {
        &MANIFEST
    }

    fn schema(&self) -> &'static Schema {
        &SCHEMA
    }

    fn kernel(&self) -> &'static Kernel {
        &KERNEL
    }

    /// darktable's `reload_defaults`: +0.7 EV, a slightly lowered black and the
    /// camera's exposure bias compensated for a colour raw; no exposure change for a
    /// monochrome raw, and none at all for an image that is not a raw.
    fn defaults(&self, image: &ImageContext) -> ParamValues {
        let mut values = SCHEMA.defaults();
        match &image.camera {
            Some(camera) if camera.cfa.is_some() => {}
            Some(_) => {
                values.set("exposure", Value::Float(0.0));
                values.set("black", Value::Float(0.0));
            }
            None => {
                values.set("exposure", Value::Float(0.0));
                values.set("black", Value::Float(0.0));
                values.set("compensate_exposure_bias", Value::Bool(false));
            }
        }
        values
    }

    fn prepare(
        &self,
        params: &ParamValues,
        image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<Prepared, NotPorted> {
        if let Some(what) = not_ported(params) {
            return Err(NotPorted {
                operation: "exposure",
                what,
            });
        }
        // `ExposureParams` in the kernel: black, then exposure.
        let bytes = [params.float("black"), effective_exposure(params, image)]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        Ok(Prepared::per_pixel(input, bytes, Vec::new()))
    }
}

#[cfg(test)]
mod tests {
    use apertrust_plugin::{Camera, Pattern};

    use super::*;

    fn raw(cfa: bool, exposure_bias: f32) -> ImageContext {
        let pattern = |values: Vec<u32>| Pattern {
            width: 2,
            height: 2,
            values,
        };
        ImageContext {
            camera: Some(Camera {
                make: String::new(),
                model: String::new(),
                alias: String::new(),
                width: 8,
                height: 8,
                cfa: cfa.then(|| pattern(vec![0, 1, 1, 2])),
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
                exposure_bias,
                iso: 0.0,
                linear_response_limit: 1.0,
                lens_correction: None,
                orientation: 0,
            }),
            ..ImageContext::full(8, 8)
        }
    }

    /// The kernel's two parameters for these settings on this image.
    fn kernel_params(image: &ImageContext, values: &ParamValues) -> [f32; 2] {
        let params = SCHEMA
            .resolve_over(&Exposure.defaults(image), values)
            .unwrap();
        let input = ImageDesc::rgba(8, 8);
        let prepared = Exposure.prepare(&params, image, &input).unwrap();
        let bytes = &prepared.passes[0].params;
        [0, 4].map(|at| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()))
    }

    #[test]
    fn defaults_follow_darktable_by_image() {
        let none = ParamValues::new();
        assert_eq!(kernel_params(&raw(true, 0.0), &none), [-1.0 / 4096.0, 0.7]);
        assert_eq!(kernel_params(&raw(false, 0.0), &none), [0.0, 0.0]);
        assert_eq!(kernel_params(&ImageContext::full(8, 8), &none), [0.0, 0.0]);
    }

    #[test]
    fn the_camera_exposure_bias_is_compensated_within_5_ev() {
        let none = ParamValues::new();
        assert_eq!(kernel_params(&raw(true, 1.0), &none)[1], 0.7 - 1.0);
        assert_eq!(kernel_params(&raw(true, -8.0), &none)[1], 0.7 + 5.0);
        let mut off = ParamValues::new();
        off.set("compensate_exposure_bias", Value::Bool(false));
        assert_eq!(kernel_params(&raw(true, 1.0), &off)[1], 0.7);
    }

    #[test]
    fn only_manual_mode_without_highlight_preservation_is_ported() {
        let image = raw(true, -0.67);
        let input = ImageDesc::rgba(8, 8);
        // Every parameter written at its default renders as none is.
        let mut defaults = ParamValues::new();
        defaults.set("mode", Value::Choice(MANUAL));
        defaults.set("deflicker_percentile", Value::Float(PERCENTILE));
        defaults.set("deflicker_target_level", Value::Float(TARGET_LEVEL));
        defaults.set("compensate_hilite_pres", Value::Bool(false));
        assert_eq!(
            kernel_params(&image, &defaults),
            kernel_params(&image, &ParamValues::new())
        );
        for (name, value) in [
            ("mode", Value::Choice(1)),
            ("deflicker_percentile", Value::Float(60.0)),
            ("deflicker_target_level", Value::Float(-3.0)),
            ("compensate_hilite_pres", Value::Bool(true)),
        ] {
            let mut values = ParamValues::new();
            values.set(name, value);
            let params = SCHEMA
                .resolve_over(&Exposure.defaults(&image), &values)
                .unwrap();
            assert!(Exposure.prepare(&params, &image, &input).is_err(), "{name}");
        }
    }

    #[test]
    fn the_effective_exposure_is_what_the_kernel_gets() {
        for (image, set) in [
            (raw(true, 1.0), None),
            (raw(true, -8.0), None),
            (raw(true, 1.0), Some(false)),
            (ImageContext::full(8, 8), None),
        ] {
            let mut values = ParamValues::new();
            if let Some(flag) = set {
                values.set("compensate_exposure_bias", Value::Bool(flag));
            }
            let params = SCHEMA
                .resolve_over(&Exposure.defaults(&image), &values)
                .unwrap();
            assert_eq!(
                effective_exposure(&params, &image),
                kernel_params(&image, &values)[1]
            );
        }
    }
}
