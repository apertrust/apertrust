// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/iop/colorout.c)
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

//! Output colour profile (darktable's colorout), ported from darktable
//! (`src/iop/colorout.c` with `src/common/colorspaces.c`): the working space, linear
//! Rec2020, to the output profile, sRGB by default, through XYZ D50 as lcms builds
//! darktable's profiles, then the profile's tone curve. Ported: sRGB, linear Rec2020 and
//! linear Rec709. `docs/ports/colorout.md` is its spec.

use apertrust_color::{
    Mat3d, PROFILE_D65, REC2020_PRIMARIES, SRGB_PRIMARIES, inverse_d, mul_d,
    profile_rgb_to_xyz_d50, to_f32,
};
use apertrust_plugin::{
    Domain, Format, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation,
    OutputProfile, Param, ParamValues, Prepared, Provenance, Schema, Slot, Stage,
};

/// Generated from `slang/colorout.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "colorout",
    wgsl: include_str!("../generated/colorout.wgsl"),
    derivatives: false,
};

pub struct Colorout;

static MANIFEST: Manifest = Manifest {
    id: "colorout",
    version: 1,
    slot: Slot {
        stage: Stage::Display,
        order: 70.0,
    },
    domain: Domain::Display,
    layers: false,
    differentiable: false,
    radius: 0,
    default_enabled: true,
    provenance: Some(Provenance {
        darktable_module: "colorout",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &["src/iop/colorout.c", "src/common/colorspaces.c"],
    }),
};

/// darktable's colour profile types (`apertrust-color`).
pub use apertrust_color::PROFILE_TYPES as PROFILES;

/// darktable `dt_iop_colorout_params_t` version 5, without the file name.
static SCHEMA: Schema = Schema {
    version: 1,
    params: &[
        Param {
            name: "type",
            kind: Kind::Choice {
                default: 1,
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
    ],
};

/// The output profile's RGB to XYZ D50, and whether it has sRGB's tone curve.
fn profile(name: &str) -> Result<(Mat3d, bool), NotPorted> {
    match name {
        "srgb" => Ok((profile_rgb_to_xyz_d50(PROFILE_D65, SRGB_PRIMARIES), true)),
        "lin_rec2020" => Ok((
            profile_rgb_to_xyz_d50(PROFILE_D65, REC2020_PRIMARIES),
            false,
        )),
        "lin_rec709" => Ok((profile_rgb_to_xyz_d50(PROFILE_D65, SRGB_PRIMARIES), false)),
        _ => Err(NotPorted {
            operation: "colorout",
            what: "output profiles other than sRGB and linear Rec2020 or Rec709",
        }),
    }
}

impl Operation for Colorout {
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
    ) -> Result<Prepared, NotPorted> {
        if input.format != Format::Rgba {
            return Err(NotPorted {
                operation: "colorout",
                what: "images other than RGB",
            });
        }
        let (to_xyz, curve) = profile(PROFILES[params.choice("type") as usize])?;
        let working = profile_rgb_to_xyz_d50(PROFILE_D65, REC2020_PRIMARIES);
        let matrix = mul_d(&inverse_d(&to_xyz), &working);
        // `ColoroutParams` in the kernel.
        let mut bytes = Vec::with_capacity(64);
        for row in matrix {
            for value in [row[0] as f32, row[1] as f32, row[2] as f32, 0.0] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        bytes.extend_from_slice(&u32::from(curve).to_le_bytes());
        bytes.extend_from_slice(&[0; 12]);
        Ok(Prepared::per_pixel(input, bytes, Vec::new()))
    }

    /// Its `type`'s matrices, as darktable's `commit_params` publishes the pipe's output
    /// profile (`dt_ioppr_set_pipe_output_profile_info`, decision 26). darktable puts
    /// sRGB's in place of a profile without a matrix, Lab's among them, so the pipe always
    /// has one; here every type without a matrix reports sRGB's, those `prepare` refuses
    /// as not ported included.
    fn output_profile(&self, params: &ParamValues) -> Option<OutputProfile> {
        let to_xyz = match profile(PROFILES[params.choice("type") as usize]) {
            Ok((to_xyz, _)) => to_xyz,
            Err(_) => profile_rgb_to_xyz_d50(PROFILE_D65, SRGB_PRIMARIES),
        };
        Some(OutputProfile {
            rgb_to_xyz_d50: to_f32(to_xyz),
            xyz_d50_to_rgb: to_f32(inverse_d(&to_xyz)),
        })
    }
}

#[cfg(test)]
mod tests {
    use apertrust_plugin::Value;

    use super::*;

    fn params(profile: &str) -> ParamValues {
        let mut values = ParamValues::new();
        let index = PROFILES.iter().position(|&p| p == profile).unwrap() as u32;
        values.set("type", Value::Choice(index));
        SCHEMA.resolve(&values).unwrap()
    }

    fn prepared(profile: &str) -> ([[f32; 3]; 3], u32) {
        let params = params(profile);
        let input = ImageDesc::rgba(4, 4);
        let prepared = Colorout
            .prepare(&params, &ImageContext::full(4, 4), &input)
            .unwrap();
        let bytes = &prepared.passes[0].params;
        let float = |at: usize| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        let rows = [0, 1, 2].map(|r| [0, 1, 2].map(|c| float(16 * r + 4 * c)));
        (rows, u32::from_le_bytes(bytes[48..52].try_into().unwrap()))
    }

    #[test]
    fn srgb_keeps_white_and_takes_the_tone_curve() {
        let (rows, curve) = prepared("srgb");
        assert_eq!(curve, 1);
        for row in rows {
            let sum: f32 = row.iter().sum();
            assert!((sum - 1.0).abs() < 1e-5, "{rows:?}");
        }
        // Rec2020's red is outside sRGB: green and blue go negative.
        assert!(rows[1][0] < 0.0 && rows[2][0] < 0.0, "{rows:?}");
    }

    #[test]
    fn linear_rec2020_out_is_the_identity() {
        let (rows, curve) = prepared("lin_rec2020");
        assert_eq!(curve, 0);
        for (r, row) in rows.iter().enumerate() {
            for (c, &value) in row.iter().enumerate() {
                assert!(
                    (value - if r == c { 1.0 } else { 0.0 }).abs() < 1e-6,
                    "{rows:?}"
                );
            }
        }
    }

    #[test]
    fn each_ported_profile_reports_matrices_that_invert_each_other() {
        let product = |a: [[f32; 3]; 3], b: [[f32; 3]; 3]| {
            [0, 1, 2].map(|r| [0, 1, 2].map(|c| (0..3).map(|k| a[r][k] * b[k][c]).sum::<f32>()))
        };
        for profile in ["srgb", "lin_rec2020", "lin_rec709"] {
            let reported = Colorout.output_profile(&params(profile)).unwrap();
            let (to_xyz, to_rgb) = (reported.rgb_to_xyz_d50, reported.xyz_d50_to_rgb);
            for rows in [product(to_rgb, to_xyz), product(to_xyz, to_rgb)] {
                for (r, row) in rows.iter().enumerate() {
                    for (c, &value) in row.iter().enumerate() {
                        // Within a few roundings of f32: the largest is one, 1.2e-7.
                        let identity = if r == c { 1.0 } else { 0.0 };
                        assert!(
                            (value - identity).abs() <= 4.0 * f32::EPSILON,
                            "{profile}: {rows:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_profile_without_a_matrix_reports_srgbs() {
        // darktable has no matrix for Lab and puts sRGB's in its place
        // (`dt_ioppr_set_pipe_output_profile_info`).
        let srgb = Colorout.output_profile(&params("srgb"));
        assert!(srgb.is_some());
        assert_eq!(Colorout.output_profile(&params("lab")), srgb);
    }

    #[test]
    fn the_default_is_srgb_and_other_profiles_are_not_ported() {
        assert_eq!(PROFILES[SCHEMA.defaults().choice("type") as usize], "srgb");
        let mut values = ParamValues::new();
        values.set("type", Value::Choice(21));
        let params = SCHEMA.resolve(&values).unwrap();
        let input = ImageDesc::rgba(4, 4);
        assert!(
            Colorout
                .prepare(&params, &ImageContext::full(4, 4), &input)
                .is_err()
        );
    }
}
