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
//! the camera matrix (darktable's standard matrix, rawler's here, decision 23, and a
//! DNG's embedded matrix, derived from its tags as darktable derives it), linear Rec2020
//! and linear Rec709 as input, linear Rec2020 as the working space, darktable's gamut
//! clipping to its four profiles and its blue mapping. `docs/ports/colorin.md` is its
//! spec.

use apertrust_color::{
    ADOBE_RGB_PRIMARIES, Mat3d, REC2020_PRIMARIES, SRGB_PRIMARIES, camera_to_xyz_d50, inverse_d,
    mul_d, profile_rgb_to_xyz_d50,
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
                choices: &GAMUT_CLIPPING,
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

/// darktable's gamut clipping, `dt_iop_color_normalize_t`: the schema's `normalize`.
pub const GAMUT_CLIPPING: [&str; 5] = ["off", "srgb", "adobe_rgb", "lin_rec709", "lin_rec2020"];

/// The clipping profile's linear RGB to XYZ D50, darktable's `lmatrix`, for gamut
/// clipping `normalize`; `None` when it is off. darktable's sRGB, Adobe RGB (compatible),
/// linear Rec709 and linear Rec2020 are matrix profiles built from their primaries and
/// `D65xyY` (`colorspaces.c`); `commit_params` reads their colorants and leaves their
/// curves out, so the clamp works on linear values whatever the profile's curve.
fn clipping_profile(normalize: u32) -> Option<Mat3d> {
    let primaries = match GAMUT_CLIPPING[normalize as usize] {
        "srgb" | "lin_rec709" => SRGB_PRIMARIES,
        "adobe_rgb" => ADOBE_RGB_PRIMARIES,
        "lin_rec2020" => REC2020_PRIMARIES,
        _ => return None,
    };
    Some(profile_rgb_to_xyz_d50(PROFILE_D65, primaries))
}

/// darktable `_apply_blue_mapping`, on camera RGB: where blue is more than half of the
/// three channels' sum, `t · 0.11` moves from blue to green, with
/// `t = (z − 0.5) / 0.5 · min(1, sum / 0.5)` and `z` blue over the sum. The kernel's
/// `blue_mapping`, restated for tests.
pub fn blue_mapping(rgb: [f32; 3]) -> [f32; 3] {
    let [r, g, b] = rgb;
    let sum = r + g + b;
    if sum > 0.0 {
        let z = b / sum;
        if z > 0.5 {
            let t = (z - 0.5) / (1.0 - 0.5) * (sum / 0.5).min(1.0) * 0.11;
            return [r, g + t, b - t];
        }
    }
    rgb
}

/// Whether darktable runs blue mapping on the image,
/// `dt_image_is_matrix_correction_supported`: a raw that is not monochrome. Here a raw
/// with colour filters, since the pipeline refuses a raw without them as it opens, a
/// linear DNG among them.
fn matrix_correction_supported(image: &ImageContext) -> bool {
    image
        .camera
        .as_ref()
        .is_some_and(|camera| camera.cfa.is_some())
}

/// The input profile's RGB to XYZ D50.
fn input_profile(name: &str, image: &ImageContext) -> Result<Mat3d, NotPorted> {
    let not_ported = |what| NotPorted {
        operation: "colorin",
        what,
    };
    match name {
        // darktable's standard matrix is its own camera table's, where Apertrust has
        // rawler's (decision 23); a DNG's embedded one is the file's, derived as darktable
        // derives it (the engine's image context). The enhanced, vendor and alternate
        // matrices are darktable's own tables', which Apertrust lacks: not ported.
        "standard_matrix" | "embedded_matrix" => {
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

    /// darktable's `reload_defaults` for a raw: a DNG's embedded matrix where the image
    /// context has one, a linear DNG's too, else darktable's standard matrix on a colour
    /// raw (both the matrix the image context holds); linear Rec2020 for an image that is
    /// not a raw, until other images are opened.
    fn defaults(&self, image: &ImageContext) -> ParamValues {
        let mut values = SCHEMA.defaults();
        let default = match &image.camera {
            Some(camera) if camera.embedded_matrix => "embedded_matrix",
            Some(camera) if camera.cfa.is_some() => "standard_matrix",
            _ => "lin_rec2020",
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
        if PROFILES[params.choice("type_work") as usize] != "lin_rec2020" {
            return Err(not_ported("working spaces other than linear Rec2020"));
        }
        let to_xyz = input_profile(PROFILES[params.choice("type") as usize], image)?;
        let from_xyz = inverse_d(&profile_rgb_to_xyz_d50(PROFILE_D65, REC2020_PRIMARIES));
        // darktable `commit_params`: with gamut clipping, camera RGB to the clipping
        // profile's linear RGB (`nmatrix`), then from it to XYZ D50 (`lmatrix`), here on
        // to the working space; without, camera RGB to XYZ D50 (`cmatrix`), here on to
        // the working space. Composed in f64.
        let clipping = clipping_profile(params.choice("normalize"));
        let (matrix, clipped) = match clipping {
            Some(clipped_to_xyz) => (
                mul_d(&inverse_d(&clipped_to_xyz), &to_xyz),
                mul_d(&from_xyz, &clipped_to_xyz),
            ),
            None => (mul_d(&from_xyz, &to_xyz), [[0.0; 3]; 3]),
        };
        // darktable `process_cmatrix`: blue mapping only on a raw that is not monochrome.
        let blue_mapping = params.flag("blue_mapping") && matrix_correction_supported(image);
        // darktable's `process` takes the late correction for every input profile but
        // Lab; its blue-mapping path, `_process_cmatrix_bm`, leaves it out of the pixels.
        let correction = if input.late_correction {
            late_correction(image)
        } else {
            [1.0; 3]
        };
        let applied = if blue_mapping { [1.0; 3] } else { correction };
        // `ColorinParams` in the kernel.
        let mut bytes = Vec::with_capacity(128);
        for row in matrix.iter().chain(&clipped) {
            for value in [row[0] as f32, row[1] as f32, row[2] as f32, 0.0] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        for value in applied.into_iter().chain([1.0]) {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for flag in [clipping.is_some(), blue_mapping, false, false] {
            bytes.extend_from_slice(&u32::from(flag).to_le_bytes());
        }
        let mut prepared = Prepared::per_pixel(input, bytes, Vec::new());
        // darktable `process`: after the correction, the image is at the camera's D65
        // reference, and its saturation level moves with it, whether or not the pixels
        // took it, as on its blue-mapping path.
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
                embedded_matrix: false,
                crop: [0, 0, 8, 8],
                declared: [0, 0, 8, 8],
                exposure_bias: 0.0,
                iso: 0.0,
                linear_response_limit: 1.0,
                lens_correction: None,
                highlight_preservation: 0.0,
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
        let uniforms = uniforms(prepared);
        (uniforms.rows, uniforms.correction)
    }

    /// `ColorinParams` as the kernel reads it.
    struct Uniforms {
        rows: [[f32; 3]; 3],
        clipped: [[f32; 3]; 3],
        correction: [f32; 3],
        clipping: bool,
        blue_mapping: bool,
    }

    fn uniforms(prepared: &Prepared) -> Uniforms {
        let bytes = &prepared.passes[0].params;
        assert_eq!(bytes.len(), 128);
        let word = |at: usize| <[u8; 4]>::try_from(&bytes[at..at + 4]).unwrap();
        let float = |at: usize| f32::from_le_bytes(word(at));
        let flag = |at: usize| match u32::from_le_bytes(word(at)) {
            0 => false,
            1 => true,
            other => panic!("{other}"),
        };
        let matrix = |at: usize| [0, 1, 2].map(|r| [0, 1, 2].map(|c| float(at + 16 * r + 4 * c)));
        Uniforms {
            rows: matrix(0),
            clipped: matrix(48),
            correction: [96, 100, 104].map(float),
            clipping: flag(112),
            blue_mapping: flag(116),
        }
    }

    fn prepared(image: &ImageContext, input: &ImageDesc, set: &[(&str, Value)]) -> Prepared {
        let mut values = ParamValues::new();
        for (name, value) in set {
            values.set(name, *value);
        }
        let params = SCHEMA
            .resolve_over(&Colorin.defaults(image), &values)
            .unwrap();
        Colorin.prepare(&params, image, input).unwrap()
    }

    fn product(a: &[[f32; 3]; 3], b: &[[f32; 3]; 3]) -> [[f32; 3]; 3] {
        std::array::from_fn(|r| std::array::from_fn(|c| (0..3).map(|k| a[r][k] * b[k][c]).sum()))
    }

    /// Gamut clipping splits the matrix in two at the clipping profile's linear RGB:
    /// camera RGB to it, then on to the working space, the two together the matrix
    /// without clipping. sRGB and linear Rec709 clip alike, darktable's sRGB curve being
    /// left out, and linear Rec2020, the working space, goes on by the identity.
    #[test]
    fn gamut_clipping_goes_through_the_clipping_profile_s_linear_rgb() {
        let (image, input) = raw(false);
        let off = uniforms(&prepared(&image, &input, &[]));
        assert!(!off.clipping && !off.blue_mapping);
        assert_eq!(off.clipped, [[0.0; 3]; 3]);
        let mut by_name = Vec::new();
        for (index, name) in GAMUT_CLIPPING.iter().enumerate().skip(1) {
            let on = uniforms(&prepared(
                &image,
                &input,
                &[("normalize", Value::Choice(index as u32))],
            ));
            assert!(on.clipping && !on.blue_mapping, "{name}");
            let whole = product(&on.clipped, &on.rows);
            for (row, expected) in whole.iter().zip(&off.rows) {
                for (value, expected) in row.iter().zip(expected) {
                    assert!((value - expected).abs() < 1e-5, "{name}: {whole:?}");
                }
            }
            by_name.push((*name, on.rows, on.clipped));
        }
        let find = |name| by_name.iter().find(|(n, ..)| *n == name).unwrap();
        assert_eq!(find("srgb").1, find("lin_rec709").1);
        assert_eq!(find("srgb").2, find("lin_rec709").2);
        assert_ne!(find("srgb").1, find("adobe_rgb").1);
        for (r, row) in find("lin_rec2020").2.iter().enumerate() {
            for (c, &value) in row.iter().enumerate() {
                let identity = if r == c { 1.0 } else { 0.0 };
                assert!((value - identity).abs() < 1e-6, "{row:?}");
            }
        }
    }

    /// darktable's `_apply_blue_mapping`, at values worked by hand.
    #[test]
    fn blue_mapping_moves_blue_to_green_where_blue_dominates() {
        // z = 0.8, bright: t = 0.6, 0.066 moved.
        let [r, g, b] = blue_mapping([0.1, 0.1, 0.8]);
        assert_eq!(r, 0.1);
        assert!(
            (g - 0.166).abs() < 1e-6 && (b - 0.734).abs() < 1e-6,
            "{g} {b}"
        );
        // z = 0.6 with a sum of 0.25, half of 0.5: t = 0.2 · 0.5, 0.011 moved.
        let [_, g, b] = blue_mapping([0.05, 0.05, 0.15]);
        assert!(
            (g - 0.061).abs() < 1e-6 && (b - 0.139).abs() < 1e-6,
            "{g} {b}"
        );
        // The sum moves nothing.
        let mapped = blue_mapping([0.2, 0.3, 1.5]);
        assert!((mapped.iter().sum::<f32>() - 2.0).abs() < 1e-6);
        // Blue at half the sum or less, a sum of zero or below: unchanged.
        for rgb in [
            [0.25, 0.25, 0.5],
            [0.5, 0.3, 0.2],
            [0.0; 3],
            [-0.5, 0.1, 0.3],
            [0.1, -0.2, -0.3],
        ] {
            assert_eq!(blue_mapping(rgb), rgb);
        }
    }

    /// darktable runs blue mapping only on a raw that is not monochrome, and its path
    /// leaves the late correction out of the pixels while moving the saturation level and
    /// the white balance as if it were in.
    #[test]
    fn blue_mapping_runs_on_a_colour_raw_without_the_late_correction() {
        let set = [("blue_mapping", Value::Bool(true))];
        let (image, input) = raw(true);
        let without = prepared(&image, &input, &[]);
        let with = prepared(&image, &input, &set);
        let on = uniforms(&with);
        assert!(on.blue_mapping && !on.clipping);
        assert_eq!(on.correction, [1.0; 3]);
        assert_eq!(on.rows, uniforms(&without).rows);
        assert_eq!(with.output, without.output);
        assert_ne!(uniforms(&without).correction, [1.0; 3]);
        // With gamut clipping too.
        let both = uniforms(&prepared(
            &image,
            &input,
            &[set[0], ("normalize", Value::Choice(1))],
        ));
        assert!(both.blue_mapping && both.clipping);
        // Not on a raw without colour filters, nor on an image that is not a raw: the
        // late correction is the pixels' again.
        let mut monochrome = image.clone();
        monochrome.camera.as_mut().unwrap().cfa = None;
        let not_raw = ImageContext::full(8, 8);
        let set = [
            ("blue_mapping", Value::Bool(true)),
            ("type", Value::Choice(profile("lin_rec2020"))),
        ];
        for image in [&monochrome, &not_raw] {
            let off = uniforms(&prepared(image, &raw(false).1, &set));
            assert!(!off.blue_mapping);
        }
        let monochrome = uniforms(&prepared(&monochrome, &input, &set));
        assert_eq!(monochrome.correction, uniforms(&without).correction);
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

    /// darktable's default is a DNG's embedded matrix where it has one, and the standard
    /// matrix on other raws; both are the matrix the image context holds, so they render
    /// alike.
    #[test]
    fn a_dng_with_its_own_matrix_defaults_to_the_embedded_matrix() {
        let (mut image, input) = raw(false);
        let default =
            |image: &ImageContext| PROFILES[Colorin.defaults(image).choice("type") as usize];
        assert_eq!(default(&image), "standard_matrix");
        let standard = Colorin
            .prepare(
                &SCHEMA
                    .resolve_over(&Colorin.defaults(&image), &ParamValues::new())
                    .unwrap(),
                &image,
                &input,
            )
            .unwrap();
        image.camera.as_mut().unwrap().embedded_matrix = true;
        assert_eq!(default(&image), "embedded_matrix");
        let embedded = Colorin
            .prepare(
                &SCHEMA
                    .resolve_over(&Colorin.defaults(&image), &ParamValues::new())
                    .unwrap(),
                &image,
                &input,
            )
            .unwrap();
        assert_eq!(embedded.passes[0].params, standard.passes[0].params);
        // A linear DNG, with no colour filters, keeps its embedded matrix, as darktable's
        // `reload_defaults` checks for one before the raw's matrix.
        image.camera.as_mut().unwrap().cfa = None;
        assert_eq!(default(&image), "embedded_matrix");
        assert_eq!(default(&ImageContext::full(8, 8)), "lin_rec2020");
    }

    #[test]
    fn other_profiles_and_options_are_not_ported() {
        let (image, input) = raw(false);
        for (name, value) in [
            ("type", Value::Choice(profile("srgb"))),
            // darktable's own tables' matrices, never the camera's under their names.
            ("type", Value::Choice(profile("enhanced_matrix"))),
            ("type", Value::Choice(profile("vendor_matrix"))),
            ("type", Value::Choice(profile("alternate_matrix"))),
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
