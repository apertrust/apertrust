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

//! A decoded raw for the pipeline: its metadata as the operations' image context, and
//! its mosaic as the pipeline's input.

use apertrust_color::{DngCalibration, Mat3, dng_light_source, dng_xyz_to_camera};
use apertrust_plugin::{
    Camera, DngCorrection, FujifilmCorrection, ImageContext, LensCorrection, OlympusCorrection,
    Pattern, PipeKind, SonyCorrection,
};
use apertrust_raw::{CfaColor, DngColor, Illuminant, Pixels, RawImage, Rect};

use crate::compute::{ComputeError, GpuImage};
use crate::gpu::GpuContext;

/// What operations know about a raw (decision 21). rawprepare keeps rawler's active
/// area, every valid photosite, and the developed image is the manufacturer's declared
/// size inside it, rawler's default crop, or the active area for a raw that declares
/// none (decision 23).
pub fn image_context(raw: &RawImage) -> ImageContext {
    let crop = raw.active_area;
    let declared = raw
        .default_crop
        .map_or(crop, |declared| intersect(declared, crop));
    let cfa = raw.cfa.as_ref().map(|cfa| Pattern {
        width: cfa.width,
        height: cfa.height,
        // Red, green and blue as 0, 1 and 2; other filters as 3, which no operation
        // supports yet.
        values: cfa
            .colors
            .iter()
            .map(|color| match color {
                CfaColor::Red => 0,
                CfaColor::Green => 1,
                CfaColor::Blue => 2,
                _ => 3,
            })
            .collect(),
    });
    ImageContext {
        width: declared.width,
        height: declared.height,
        scale: 1.0,
        region: [0, 0, declared.width, declared.height],
        pipe: PipeKind::Export,
        camera: Some(Camera {
            make: raw.camera.make.clone(),
            model: raw.camera.model.clone(),
            alias: raw.camera.alias.clone(),
            width: raw.width,
            height: raw.height,
            cfa,
            black: Pattern {
                width: raw.black_level.width,
                height: raw.black_level.height,
                values: raw.black_level.values.clone(),
            },
            white: raw
                .white_levels
                .first()
                .copied()
                .unwrap_or(u32::from(u16::MAX)) as f32,
            white_balance: raw.white_balance.clone(),
            xyz_to_camera: xyz_to_camera(raw),
            crop: [crop.x, crop.y, crop.width, crop.height],
            declared: [declared.x, declared.y, declared.width, declared.height],
            exposure_bias: raw.exposure_bias.unwrap_or(0.0),
            iso: raw.iso.unwrap_or(0.0),
            linear_response_limit: raw.linear_response_limit.unwrap_or(1.0),
            lens_correction: raw.lens_correction.as_ref().map(lens_correction),
            orientation: flip_bits(raw.orientation),
        }),
        // The planner's, from the plan's colorout (decision 26).
        output_profile: None,
        // The viewport planner's, for one operation's `prepare` (decision 15).
        full_resolution: None,
    }
}

/// darktable's flip bits for an EXIF orientation, `dt_image_orientation_to_flip_bits`
/// (`src/common/image.h`): none where the raw records none or a value outside EXIF's.
fn flip_bits(orientation: apertrust_raw::Orientation) -> u8 {
    use apertrust_raw::Orientation;
    const FLIP_Y: u8 = 1;
    const FLIP_X: u8 = 2;
    const SWAP_XY: u8 = 4;
    match orientation {
        Orientation::Normal | Orientation::Unknown => 0,
        Orientation::MirrorHorizontal => FLIP_X,
        Orientation::Rotate180 => FLIP_Y | FLIP_X,
        Orientation::MirrorVertical => FLIP_Y,
        Orientation::Transpose => SWAP_XY,
        // EXIF's 6, rotate 90° clockwise: darktable's ORIENTATION_ROTATE_CW_90_DEG.
        Orientation::Rotate90 => FLIP_Y | SWAP_XY,
        Orientation::Transverse => FLIP_Y | FLIP_X | SWAP_XY,
        // EXIF's 8: darktable's ORIENTATION_ROTATE_CCW_90_DEG.
        Orientation::Rotate270 => FLIP_X | SWAP_XY,
    }
}

/// The raw's lens corrections, value for value (decision 25).
fn lens_correction(correction: &apertrust_raw::LensCorrection) -> LensCorrection {
    use apertrust_raw::LensCorrection as Raw;
    match correction {
        Raw::Sony(c) => LensCorrection::Sony(SonyCorrection {
            nc: c.nc,
            distortion: c.distortion,
            ca_r: c.ca_r,
            ca_b: c.ca_b,
            vignetting: c.vignetting,
        }),
        Raw::Fujifilm(c) => LensCorrection::Fujifilm(FujifilmCorrection {
            nc: c.nc,
            cropf: c.cropf,
            knots: c.knots,
            distortion: c.distortion,
            ca_r: c.ca_r,
            ca_b: c.ca_b,
            vignetting: c.vignetting,
        }),
        Raw::Dng(c) => LensCorrection::Dng(DngCorrection {
            planes: c.planes,
            cwarp: c.cwarp,
            centre_warp: c.centre_warp,
            cvig: c.cvig,
            centre_vig: c.centre_vig,
            has_warp: c.has_warp,
            has_vignette: c.has_vignette,
        }),
        Raw::Olympus(c) => LensCorrection::Olympus(OlympusCorrection {
            has_dist: c.has_dist,
            dist: c.dist,
            has_ca: c.has_ca,
            ca: c.ca,
        }),
    }
}

/// The camera's XYZ to camera RGB matrix under D65, by rows: a DNG's as darktable derives it from the file's calibration;
/// otherwise, or when none qualifies, as darktable falls back to its standard matrix,
/// rawler's D65 matrix, or its last.
pub fn xyz_to_camera(raw: &RawImage) -> Option<Mat3> {
    raw.dng_color.as_ref().and_then(dng_matrix).or_else(|| {
        raw.color_matrices
            .iter()
            .find(|m| m.illuminant == Illuminant::D65)
            .or(raw.color_matrices.last())
            .filter(|m| m.xyz_to_camera.len() == 3)
            .map(|m| [m.xyz_to_camera[0], m.xyz_to_camera[1], m.xyz_to_camera[2]])
    })
}

/// darktable's camera matrix for a DNG (`apertrust_color::dng_xyz_to_camera`): the
/// calibration nearest D65, its ColorMatrix times its CameraCalibration and the
/// AnalogBalance, adapted to D65; `None` when no calibration qualifies.
fn dng_matrix(dng: &DngColor) -> Option<Mat3> {
    // darktable reads matrices of nine values and a balance of three, nothing else.
    let mat3 = |values: &Option<Vec<f32>>| match values.as_deref() {
        Some(&[a, b, c, d, e, f, g, h, i]) => Some([[a, b, c], [d, e, f], [g, h, i]]),
        _ => None,
    };
    let calibrations: [DngCalibration; 3] = std::array::from_fn(|k| DngCalibration {
        illuminant: dng.calibration_illuminants[k].unwrap_or(0),
        color_matrix: mat3(&dng.color_matrices[k]),
        camera_calibration: mat3(&dng.camera_calibrations[k]),
    });
    let analog_balance = match dng.analog_balance.as_deref() {
        Some(&[r, g, b]) => Some([r, g, b]),
        _ => None,
    };
    let chosen = dng_xyz_to_camera(&calibrations, analog_balance)?;
    let n = chosen.calibration + 1;
    let mut applied = Vec::new();
    if calibrations[chosen.calibration]
        .camera_calibration
        .is_some()
    {
        applied.push(format!("CameraCalibration{n}"));
    }
    if analog_balance.is_some() {
        applied.push("AnalogBalance".to_owned());
    }
    let light = dng_light_source(chosen.illuminant);
    let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    tracing::debug!(
        target: "raw",
        "DNG matrix: ColorMatrix{n}, {} {} K{}{}",
        light.name,
        light.temperature,
        if light.to_d65 == identity {
            ""
        } else {
            ", adapted to D65"
        },
        if applied.is_empty() {
            String::new()
        } else {
            format!(", with: {}", applied.join(", "))
        },
    );
    Some(chosen.xyz_to_camera)
}

/// The part of `a` inside `b`; `b` itself when they do not overlap.
fn intersect(a: Rect, b: Rect) -> Rect {
    let (x0, y0) = (a.x.max(b.x), a.y.max(b.y));
    let x1 = (a.x + a.width).min(b.x + b.width);
    let y1 = (a.y + a.height).min(b.y + b.height);
    if x1 <= x0 || y1 <= y0 {
        return b;
    }
    Rect {
        x: x0,
        y: y0,
        width: x1 - x0,
        height: y1 - y0,
    }
}

/// The raw's 16-bit mosaic, as the raw stage's input; `None` for a raw that is not a
/// one-value-per-photosite mosaic of integers.
pub fn upload(gpu: &GpuContext, raw: &RawImage) -> Option<Result<GpuImage, ComputeError>> {
    let Pixels::U16(values) = &raw.pixels else {
        return None;
    };
    (raw.components == 1 && raw.cfa.is_some())
        .then(|| GpuImage::upload_mosaic(gpu, raw.width, raw.height, values))
}

#[cfg(test)]
mod tests {
    use apertrust_color::mul;
    use apertrust_raw::{BlackLevel, Camera as RawCamera, ColorMatrix, Orientation};

    use super::*;

    // The Leica Q2's ColorMatrix1 and 2, for StandardLightA and D50, and a D65 matrix.
    const UNDER_A: Mat3 = [
        [1.2587, -0.5232, -0.1496],
        [-0.361, 1.0841, 0.0277],
        [-0.0911, 0.1674, 0.207],
    ];
    const UNDER_D50: Mat3 = [
        [0.8389, -0.3198, -0.1019],
        [-0.3834, 1.0222, 0.0661],
        [-0.1122, 0.19, 0.3415],
    ];
    const UNDER_D65: Mat3 = [
        [1.0626, -0.4466, -0.1116],
        [-0.4205, 1.2766, 0.1562],
        [-0.0739, 0.2132, 0.5474],
    ];
    const STANDARD_LIGHT_A: u16 = 17;
    const D65: u16 = 21;
    const D50: u16 = 23;

    fn raw(color_matrices: &[(Illuminant, Mat3)], dng_color: Option<DngColor>) -> RawImage {
        RawImage {
            camera: RawCamera {
                make: "Test".into(),
                model: "Camera".into(),
                alias: "Camera".into(),
            },
            width: 2,
            height: 2,
            components: 1,
            pixels: Pixels::U16(vec![0; 4]),
            active_area: Rect {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            },
            default_crop: None,
            masked_areas: Vec::new(),
            cfa: None,
            black_level: BlackLevel {
                width: 1,
                height: 1,
                values: vec![0.0],
            },
            white_levels: vec![u32::from(u16::MAX)],
            white_balance: None,
            color_matrices: color_matrices
                .iter()
                .map(|&(illuminant, m)| ColorMatrix {
                    illuminant,
                    xyz_to_camera: m.to_vec(),
                })
                .collect(),
            dng_color,
            linear_response_limit: None,
            orientation: Orientation::Normal,
            exposure_bias: None,
            iso: None,
            lens_correction: None,
        }
    }

    fn xyz_to_camera(raw: &RawImage) -> Option<Mat3> {
        image_context(raw)
            .camera
            .and_then(|camera| camera.xyz_to_camera)
    }

    fn values(m: Mat3) -> Option<Vec<f32>> {
        Some(m.as_flattened().to_vec())
    }

    /// darktable's `dt_image_orientation_to_flip_bits`, by the EXIF value of each.
    #[test]
    fn each_exif_orientation_has_darktable_s_flip_bits() {
        let darktable = [
            (Orientation::Normal, 0),
            (Orientation::MirrorHorizontal, 2),
            (Orientation::Rotate180, 3),
            (Orientation::MirrorVertical, 1),
            (Orientation::Transpose, 4),
            (Orientation::Rotate90, 5),
            (Orientation::Transverse, 7),
            (Orientation::Rotate270, 6),
            (Orientation::Unknown, 0),
        ];
        for (orientation, bits) in darktable {
            let mut raw = raw(&[], None);
            raw.orientation = orientation;
            let camera = image_context(&raw).camera.unwrap();
            assert_eq!(camera.orientation, bits, "{orientation:?}");
        }
    }

    #[test]
    fn the_lens_corrections_are_copied_value_for_value() {
        use apertrust_raw::{
            DngCorrection, FujifilmCorrection, LensCorrection, OlympusCorrection, SonyCorrection,
        };
        fn floats<const N: usize>(from: f32) -> [f32; N] {
            std::array::from_fn(|i| from + i as f32)
        }
        fn shorts<const N: usize>(from: i16) -> [i16; N] {
            std::array::from_fn(|i| from + i as i16)
        }
        // Every value its own. Both crates' types have the same names and fields, so the
        // copy prints as the raw's only when every value reached its own field.
        let corrections = [
            None,
            Some(LensCorrection::Sony(SonyCorrection {
                nc: 16,
                distortion: shorts(1),
                ca_r: shorts(20),
                ca_b: shorts(40),
                vignetting: shorts(60),
            })),
            Some(LensCorrection::Fujifilm(FujifilmCorrection {
                nc: 9,
                cropf: 1.25,
                knots: floats(1.0),
                distortion: floats(20.0),
                ca_r: floats(40.0),
                ca_b: floats(60.0),
                vignetting: floats(80.0),
            })),
            Some(LensCorrection::Dng(DngCorrection {
                planes: 3,
                cwarp: [floats(1.0), floats(10.0), floats(20.0)],
                centre_warp: [0.25, 0.75],
                cvig: floats(30.0),
                centre_vig: [0.5, 0.125],
                has_warp: true,
                has_vignette: false,
            })),
            Some(LensCorrection::Olympus(OlympusCorrection {
                has_dist: false,
                dist: floats(1.0),
                has_ca: true,
                ca: floats(10.0),
            })),
        ];
        for correction in corrections {
            let mut raw = raw(&[], None);
            raw.lens_correction = correction.clone();
            let copied = image_context(&raw).camera.unwrap().lens_correction;
            assert_eq!(format!("{copied:?}"), format!("{correction:?}"));
        }
    }

    #[test]
    fn a_raw_that_is_not_a_dng_keeps_rawler_s_matrix() {
        let both = raw(
            &[(Illuminant::A, UNDER_A), (Illuminant::D65, UNDER_D65)],
            None,
        );
        assert_eq!(xyz_to_camera(&both), Some(UNDER_D65));
        let without_d65 = raw(
            &[(Illuminant::A, UNDER_A), (Illuminant::D50, UNDER_D50)],
            None,
        );
        assert_eq!(xyz_to_camera(&without_d65), Some(UNDER_D50));
        assert_eq!(xyz_to_camera(&raw(&[], None)), None);
    }

    #[test]
    fn a_dng_gets_darktable_s_matrix() {
        // The Q2's calibrations: darktable adapts D50's matrix to D65, where rawler's
        // matrices alone would give it as it is.
        let q2 = DngColor {
            calibration_illuminants: [Some(STANDARD_LIGHT_A), Some(D50), None],
            color_matrices: [values(UNDER_A), values(UNDER_D50), None],
            ..Default::default()
        };
        let matrices = [(Illuminant::A, UNDER_A), (Illuminant::D50, UNDER_D50)];
        assert_eq!(
            xyz_to_camera(&raw(&matrices, Some(q2))),
            Some(mul(&dng_light_source(D50).to_d65, &UNDER_D50))
        );
    }

    #[test]
    fn a_dng_s_tags_count_only_with_darktable_s_sizes() {
        let calibration = [[1.034, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.9764]];
        let dng = |analog_balance: Vec<f32>| DngColor {
            calibration_illuminants: [Some(STANDARD_LIGHT_A), Some(D65), None],
            color_matrices: [values(UNDER_A), values(UNDER_D65), None],
            camera_calibrations: [None, values(calibration), None],
            analog_balance: Some(analog_balance),
        };
        let matrices = [(Illuminant::A, UNDER_A), (Illuminant::D65, UNDER_D65)];
        let calibrated = mul(&calibration, &UNDER_D65);
        assert_eq!(
            xyz_to_camera(&raw(&matrices, Some(dng(vec![1.0, 1.0, 1.0])))),
            Some(calibrated)
        );
        // A balance of four values is not read.
        assert_eq!(
            xyz_to_camera(&raw(&matrices, Some(dng(vec![2.0, 1.0, 1.0, 1.0])))),
            Some(calibrated)
        );
        let balanced = mul(
            &[[2.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            &calibrated,
        );
        assert_eq!(
            xyz_to_camera(&raw(&matrices, Some(dng(vec![2.0, 1.0, 1.0])))),
            Some(balanced)
        );
    }

    #[test]
    fn a_dng_without_a_qualifying_calibration_falls_back_to_rawler_s() {
        // A monochrome DNG has no calibration at all.
        assert_eq!(xyz_to_camera(&raw(&[], Some(DngColor::default()))), None);
        // A twelve-value ColorMatrix, a four-colour camera's, is not read.
        let four_colours = DngColor {
            calibration_illuminants: [Some(D65), None, None],
            color_matrices: [Some(vec![0.5; 12]), None, None],
            ..Default::default()
        };
        assert_eq!(xyz_to_camera(&raw(&[], Some(four_colours))), None);
        // Light source 5 is undefined, so darktable chooses nothing; rawler reads the
        // matrix as D65's.
        let undefined = DngColor {
            calibration_illuminants: [Some(5), None, None],
            color_matrices: [values(UNDER_D65), None, None],
            ..Default::default()
        };
        let matrices = [(Illuminant::D65, UNDER_D65)];
        assert_eq!(
            xyz_to_camera(&raw(&matrices, Some(undefined))),
            Some(UNDER_D65)
        );
    }
}
