// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/iop/colorin.c)
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

//! Input colour profile (darktable's colorin), ported from darktable
//! (`src/iop/colorin.c` with `src/common/colorspaces.c`): camera RGB to the working
//! space, linear Rec2020, through the camera's matrix as darktable builds a profile from
//! it, with temperature's late correction towards the camera's D65 reference. Ported:
//! the camera matrix (darktable's standard and embedded matrices, both rawler's matrix
//! here, decision 23), linear Rec2020 and linear Rec709 as input, linear Rec2020 as the
//! working space, no gamut clipping. `docs/ports/colorin.md` is its spec.

use apertrust_color::{
    Mat3d, REC2020_PRIMARIES, SRGB_PRIMARIES, camera_to_xyz_d50, inverse_d, mul_d,
    profile_rgb_to_xyz_d50,
};
use apertrust_plugin::{
    Domain, Format, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation, Param,
    ParamValues, Prepared, Provenance, Schema, Slot, Stage, Value,
};

/// Generated from `slang/colorin.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "colorin",
    wgsl: include_str!("../generated/colorin.wgsl"),
    derivatives: false,
};

pub struct Colorin;

static MANIFEST: Manifest = Manifest {
    id: "colorin",
    version: 1,
    slot: Slot {
        stage: Stage::CameraRgb,
        order: 28.0,
    },
    domain: Domain::Scene,
    layers: false,
    differentiable: false,
    radius: 0,
    default_enabled: true,
    provenance: Some(Provenance {
        darktable_module: "colorin",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &["src/iop/colorin.c", "src/common/colorspaces.c"],
    }),
};

/// darktable's colour profile types (`apertrust-color`).
pub use apertrust_color::PROFILE_TYPES as PROFILES;

fn profile(name: &str) -> u32 {
    PROFILES
        .iter()
        .position(|&p| p == name)
        .expect("a profile of PROFILES") as u32
}

/// darktable `dt_iop_colorin_params_t` version 7, without the file names, which name
/// ICC profiles on disk.
static SCHEMA: Schema = Schema {
    version: 1,
    params: &[
        Param {
            name: "type",
            kind: Kind::Choice {
                default: 12,
                choices: PROFILES,
            },
        },
        Param {
            name: "intent",
            kind: Kind::Choice {
                default: 0,
                choices: &[
                    "perceptual",
                    "relative_colorimetric",
                    "saturation",
                    "absolute_colorimetric",
                ],
            },
        },
        Param {
            name: "normalize",
            kind: Kind::Choice {
                default: 0,
                choices: &["off", "srgb", "adobe_rgb", "lin_rec709", "lin_rec2020"],
            },
        },
        Param {
            name: "blue_mapping",
            kind: Kind::Bool { default: false },
        },
        Param {
            name: "type_work",
            kind: Kind::Choice {
                default: 4,
                choices: PROFILES,
            },
        },
    ],
};

/// darktable's D65 for its built-in RGB profiles, `D65xyY`.
const PROFILE_D65: [f64; 2] = [0.31271, 0.32902];

/// The input profile's RGB to XYZ D50.
fn input_profile(name: &str, image: &ImageContext) -> Result<Mat3d, NotPorted> {
    let not_ported = |what| NotPorted {
        operation: "colorin",
        what,
    };
    match name {
        // darktable's standard matrix is its own camera table's, where Apertrust has
        // rawler's (decision 23); a DNG's embedded one is the file's, derived as darktable
        // derives it (the engine's image context).
        "standard_matrix" | "embedded_matrix" | "enhanced_matrix" => {
            let matrix = image
                .camera
                .as_ref()
                .and_then(|camera| camera.xyz_to_camera.as_ref())
                .ok_or(not_ported("a raw without a colour matrix"))?;
            Ok(camera_to_xyz_d50(matrix))
        }
        "lin_rec2020" => Ok(profile_rgb_to_xyz_d50(PROFILE_D65, REC2020_PRIMARIES)),
        "lin_rec709" => Ok(profile_rgb_to_xyz_d50(PROFILE_D65, SRGB_PRIMARIES)),
        _ => Err(not_ported(
            "input profiles other than the camera matrix and linear RGB",
        )),
    }
}

/// darktable's late correction: the camera's D65 multipliers over its as-shot ones,
/// `chr->D65coeffs / chr->as_shot`, as temperature's `reload_defaults` finds them.
fn late_correction(image: &ImageContext) -> [f32; 3] {
    let Some(camera) = &image.camera else {
        return [1.0; 3];
    };
    let Some(matrix) = &camera.xyz_to_camera else {
        return [1.0; 3];
    };
    apertrust_color::late_correction(camera.white_balance.as_deref(), matrix)
}

impl Operation for Colorin {
    fn manifest(&self) -> &'static Manifest {
        &MANIFEST
    }

    fn schema(&self) -> &'static Schema {
        &SCHEMA
    }

    fn kernel(&self) -> &'static Kernel {
        &KERNEL
    }

    /// darktable's `reload_defaults` for a raw: the camera's matrix (darktable's
    /// standard matrix, or a DNG's embedded one, which the image context holds alike);
    /// linear Rec2020 for an image that is not a raw, until other images are opened.
    fn defaults(&self, image: &ImageContext) -> ParamValues {
        let mut values = SCHEMA.defaults();
        let raw = image
            .camera
            .as_ref()
            .is_some_and(|camera| camera.cfa.is_some());
        let default = if raw {
            "standard_matrix"
        } else {
            "lin_rec2020"
        };
        values.set("type", Value::Choice(profile(default)));
        values
    }

    fn prepare(
        &self,
        params: &ParamValues,
        image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<Prepared, NotPorted> {
        let not_ported = |what| NotPorted {
            operation: "colorin",
            what,
        };
        if input.format != Format::Rgba {
            return Err(not_ported("images other than RGB"));
        }
        if params.choice("normalize") != 0 {
            return Err(not_ported("gamut clipping"));
        }
        if params.flag("blue_mapping") {
            return Err(not_ported("blue mapping"));
        }
        if PROFILES[params.choice("type_work") as usize] != "lin_rec2020" {
            return Err(not_ported("working spaces other than linear Rec2020"));
        }
        let to_xyz = input_profile(PROFILES[params.choice("type") as usize], image)?;
        let working = profile_rgb_to_xyz_d50(PROFILE_D65, REC2020_PRIMARIES);
        let matrix = mul_d(&inverse_d(&working), &to_xyz);
        // darktable applies the late correction to every input profile but Lab.
        let correction = if input.late_correction {
            late_correction(image)
        } else {
            [1.0; 3]
        };
        // `ColorinParams` in the kernel.
        let mut bytes = Vec::with_capacity(64);
        for row in matrix {
            for value in [row[0] as f32, row[1] as f32, row[2] as f32, 0.0] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        for value in correction.into_iter().chain([1.0]) {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        let mut prepared = Prepared::per_pixel(input, bytes, Vec::new());
        // darktable `process`: after the correction, the image is at the camera's D65
        // reference, and its saturation level moves with it.
        if input.late_correction {
            let output = &mut prepared.output;
            output.maximum = [0, 1, 2].map(|c| input.maximum[c] * correction[c]);
            output.white_balance = [0, 1, 2].map(|c| input.white_balance[c] * correction[c]);
            output.late_correction = false;
        }
        Ok(prepared)
    }
}

#[cfg(test)]
mod tests {
    use apertrust_color::daylight_multipliers;
    use apertrust_plugin::{Camera, Pattern};

    use super::*;

    /// XYZ to linear sRGB, D65: a camera whose matrix is sRGB's.
    const XYZ_TO_SRGB: [[f32; 3]; 3] = [
        [3.240_454, -1.537_138_5, -0.498_531_4],
        [-0.969_266, 1.876_010_8, 0.041_556],
        [0.055_643_4, -0.204_025_9, 1.057_225_2],
    ];

    /// A camera whose matrix is sRGB's.
    fn raw(late_correction: bool) -> (ImageContext, ImageDesc) {
        let image = ImageContext {
            camera: Some(Camera {
                make: String::new(),
                model: String::new(),
                alias: String::new(),
                width: 8,
                height: 8,
                cfa: Some(Pattern {
                    width: 2,
                    height: 2,
                    values: vec![0, 1, 1, 2],
                }),
                black: Pattern {
                    width: 1,
                    height: 1,
                    values: vec![0.0],
                },
                white: 1.0,
                white_balance: Some(vec![2.0, 1.0, 1.5]),
                xyz_to_camera: Some(XYZ_TO_SRGB),
                crop: [0, 0, 8, 8],
                declared: [0, 0, 8, 8],
                exposure_bias: 0.0,
                iso: 0.0,
                linear_response_limit: 1.0,
                lens_correction: None,
                orientation: 0,
            }),
            ..ImageContext::full(8, 8)
        };
        let input = ImageDesc {
            maximum: [2.0, 1.0, 1.5],
            white_balance: [2.0, 1.0, 1.5],
            late_correction,
            ..ImageDesc::rgba(8, 8)
        };
        (image, input)
    }

    fn kernel(prepared: &Prepared) -> ([[f32; 3]; 3], [f32; 3]) {
        let bytes = &prepared.passes[0].params;
        let float = |at: usize| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        let rows = [0, 1, 2].map(|r| [0, 1, 2].map(|c| float(16 * r + 4 * c)));
        (rows, [48, 52, 56].map(float))
    }

    #[test]
    fn white_stays_white() {
        let (image, input) = raw(false);
        let params = SCHEMA
            .resolve_over(&Colorin.defaults(&image), &ParamValues::new())
            .unwrap();
        let prepared = Colorin.prepare(&params, &image, &input).unwrap();
        let (rows, correction) = kernel(&prepared);
        assert_eq!(correction, [1.0; 3]);
        // Camera white, D65 for darktable's camera profile, lands near Rec2020's white:
        // the two D65s differ in the fifth decimal.
        for row in rows {
            let sum: f32 = row.iter().sum();
            assert!((sum - 1.0).abs() < 1e-3, "{rows:?}");
        }
    }

    #[test]
    fn linear_rec2020_in_is_the_identity() {
        let (image, input) = raw(false);
        let mut values = ParamValues::new();
        values.set("type", Value::Choice(profile("lin_rec2020")));
        let params = SCHEMA
            .resolve_over(&Colorin.defaults(&image), &values)
            .unwrap();
        let (rows, _) = kernel(&Colorin.prepare(&params, &image, &input).unwrap());
        for (r, row) in rows.iter().enumerate() {
            for (c, &value) in row.iter().enumerate() {
                let identity = if r == c { 1.0 } else { 0.0 };
                assert!((value - identity).abs() < 1e-6, "{rows:?}");
            }
        }
    }

    #[test]
    fn the_late_correction_moves_to_the_cameras_d65_reference() {
        let (image, input) = raw(true);
        let params = SCHEMA
            .resolve_over(&Colorin.defaults(&image), &ParamValues::new())
            .unwrap();
        let prepared = Colorin.prepare(&params, &image, &input).unwrap();
        let (_, correction) = kernel(&prepared);
        let daylight = daylight_multipliers(&XYZ_TO_SRGB);
        let expected = [daylight[0] / 2.0, 1.0, daylight[2] / 1.5].map(|v| v as f32);
        assert_eq!(correction, expected);
        let output = prepared.output;
        assert!(!output.late_correction);
        assert_eq!(
            output.maximum,
            [0, 1, 2].map(|c| input.maximum[c] * correction[c])
        );
    }

    #[test]
    fn other_profiles_and_options_are_not_ported() {
        let (image, input) = raw(false);
        for (name, value) in [
            ("type", Value::Choice(profile("srgb"))),
            ("normalize", Value::Choice(1)),
            ("blue_mapping", Value::Bool(true)),
            ("type_work", Value::Choice(profile("prophoto_rgb"))),
        ] {
            let mut values = ParamValues::new();
            values.set(name, value);
            let params = SCHEMA
                .resolve_over(&Colorin.defaults(&image), &values)
                .unwrap();
            assert!(Colorin.prepare(&params, &image, &input).is_err(), "{name}");
        }
    }
}
