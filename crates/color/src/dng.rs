// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/common/exif.cc)
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

//! A DNG's camera matrix as darktable derives it from the file's colour calibration
//! (`dt_exif_read` in `exif.cc`), the matrix colorin's embedded profile and temperature's
//! daylight multipliers start from.

use crate::{Mat3, mul};

/// One of a DNG's calibrations, the tags numbered 1, 2 or 3, as darktable reads it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DngCalibration {
    /// CalibrationIlluminant, an EXIF light source code; 0, unknown, if the file has
    /// no such tag.
    pub illuminant: u16,
    /// ColorMatrix: XYZ to camera RGB under the light source, by rows; `None` unless
    /// the file records nine values.
    pub color_matrix: Option<Mat3>,
    /// CameraCalibration: this camera's deviation from its model's ColorMatrix; `None`
    /// unless the file records nine values.
    pub camera_calibration: Option<Mat3>,
}

/// A DNG's camera matrix, as darktable chooses and adapts it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DngMatrix {
    /// The calibration chosen: 0, 1 or 2 for the tags numbered 1, 2 or 3.
    pub calibration: usize,
    /// Its light source as darktable takes it, with "other" and an unknown one read as
    /// D65.
    pub illuminant: u16,
    /// XYZ to camera RGB under D65, by rows: darktable's `d65_color_matrix`.
    pub xyz_to_camera: Mat3,
}

/// A light source as darktable knows it (`dt_dng_illuminant_data_t`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DngLightSource {
    pub name: &'static str,
    /// Colour temperature in kelvin; 0 for the codes the DNG specification leaves
    /// undefined.
    pub temperature: i32,
    /// Bradford adaptation from the light source to D65, by rows, from Bruce Lindbloom's
    /// tables or the DNG SDK's chromaticities.
    pub to_d65: Mat3,
}

const UNKNOWN: u16 = 0;
const D65: u16 = 21;
/// darktable's last light source, "Other illuminant", standing for every code above
/// ISOStudioTungsten (24), the EXIF "other light source" (255) among them.
const OTHER: u16 = 25;

const IDENTITY: Mat3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
const NONE: Mat3 = [[0.0; 3]; 3];

const DAYLIGHT_TO_D65: Mat3 = [
    [0.9726856, -0.0135482, 0.0361731],
    [-0.0167463, 1.0049102, 0.0120598],
    [0.0070026, -0.0116372, 1.1869548],
];
const TUNGSTEN_TO_D65: Mat3 = [
    [0.8446965, -0.1179225, 0.3948108],
    [-0.1366303, 1.1041226, 0.1291718],
    [0.0798489, -0.1348999, 3.1924009],
];
const D75_TO_D65: Mat3 = [
    [1.0206905, 0.0091588, -0.0228796],
    [0.0115005, 0.9984917, -0.0076762],
    [-0.0043619, 0.0072053, 0.8853432],
];

/// darktable's `illuminant_data`, indexed by EXIF light source code.
const LIGHT_SOURCES: [DngLightSource; 26] = [
    light("Unknown Illuminant", 0, NONE),
    light("Daylight", 5500, DAYLIGHT_TO_D65),
    light(
        "Fluorescent",
        4230,
        [
            [0.9212269, -0.0449128, 0.1211620],
            [-0.0553723, 1.0277243, 0.0403563],
            [0.0235086, -0.0391019, 1.6390644],
        ],
    ),
    light("Tungsten", 2850, TUNGSTEN_TO_D65),
    light("Flash", 5500, DAYLIGHT_TO_D65),
    light("Unknown Illuminant5", 0, NONE),
    light("Unknown Illuminant6", 0, NONE),
    light("Unknown Illuminant7", 0, NONE),
    light("Unknown Illuminant8", 0, NONE),
    light("FineWeather", 5500, DAYLIGHT_TO_D65),
    light("CloudyWeather", 6500, IDENTITY),
    light("Shade", 7500, D75_TO_D65),
    light(
        "DaylightFluorescent",
        6430,
        [
            [1.0096114, 0.0061501, 0.0068113],
            [0.0102539, 0.9888663, 0.0015575],
            [0.0023119, -0.0044823, 1.0525915],
        ],
    ),
    light(
        "DayWhiteFluorescent",
        5000,
        [
            [0.9554129, -0.0231280, 0.0637169],
            [-0.0283629, 1.0099053, 0.0211824],
            [0.0124188, -0.0206922, 1.3330592],
        ],
    ),
    light(
        "CoolWhiteFluorescent",
        4150,
        [
            [0.9147843, -0.0492842, 0.1202810],
            [-0.0622085, 1.034984, 0.0404480],
            [0.0228014, -0.0375807, 1.6259804],
        ],
    ),
    light(
        "WhiteFluorescent",
        3450,
        [
            [0.8805388, -0.0774890, 0.2293784],
            [-0.0932136, 1.0589267, 0.0757827],
            [0.0453660, -0.0760107, 2.2417979],
        ],
    ),
    light(
        "WarmWhiteFluorescent",
        2940,
        [
            [0.8488316, -0.1107439, 0.3471428],
            [-0.1310107, 1.0986874, 0.1141548],
            [0.0694025, -0.1167541, 2.9109462],
        ],
    ),
    light("StandardLightA", 2850, TUNGSTEN_TO_D65),
    light(
        "StandardLightB",
        4871,
        [
            [0.9415037, -0.0321240, 0.0584672],
            [-0.0428238, 1.0250998, 0.0203309],
            [0.0101511, -0.0161170, 1.2847354],
        ],
    ),
    light(
        "StandardLightC",
        6774,
        [
            [0.9904476, -0.0071683, -0.0116156],
            [-0.0123712, 1.0155950, -0.0029282],
            [-0.0035635, 0.0067697, 0.9181569],
        ],
    ),
    light("D55", 5500, DAYLIGHT_TO_D65),
    light("D65", 6500, IDENTITY),
    light("D75", 7500, D75_TO_D65),
    light(
        "D50",
        5000,
        [
            [0.9555766, -0.0230393, 0.0631636],
            [-0.0282895, 1.0099416, 0.0210077],
            [0.0122982, -0.0204830, 1.3299098],
        ],
    ),
    light(
        "ISOStudioTungsten",
        3200,
        [
            [0.8663030, -0.0913083, 0.2771784],
            [-0.1090504, 1.0746895, 0.0913841],
            [0.0550856, -0.0924636, 2.5119387],
        ],
    ),
    light("Other illuminant", 6500, IDENTITY),
];

const fn light(name: &'static str, temperature: i32, to_d65: Mat3) -> DngLightSource {
    DngLightSource {
        name,
        temperature,
        to_d65,
    }
}

/// darktable's light source for an EXIF light source code: codes above 24 are its
/// "Other illuminant".
pub fn dng_light_source(code: u16) -> &'static DngLightSource {
    &LIGHT_SOURCES[usize::from(code.min(OTHER))]
}

/// darktable's camera matrix for a DNG (`dt_exif_read`), or `None` when no calibration
/// qualifies, in its order:
///
/// 1. "Other" light sources count as D65.
/// 2. The calibrations are scanned in the order of their tags, and one is chosen when
///    its colour temperature is higher than that of the one chosen so far and at least
///    as near D65's: D65 itself, else the nearest, ties going to the higher temperature.
///    A nearer light source of lower temperature after a farther one of higher
///    temperature does not replace it.
/// 3. With no light source known, the first calibration with a ColorMatrix, as D65.
/// 4. `CA · AB · CC · CM`: the ColorMatrix, multiplied on the left by the
///    CameraCalibration, then the AnalogBalance (each the identity if missing), then
///    darktable's Bradford matrix from the light source to D65, in `f32`. darktable
///    multiplies the adaptation on the camera side of the matrix, not on its XYZ side.
///
/// darktable ignores the ForwardMatrix, the profile's hue and saturation maps and look
/// table, and does not interpolate between two calibrations. A chosen calibration
/// without a ColorMatrix gives `None` here, where darktable composes the identity in its
/// place.
pub fn dng_xyz_to_camera(
    calibrations: &[DngCalibration; 3],
    analog_balance: Option<[f32; 3]>,
) -> Option<DngMatrix> {
    let mut illuminants = calibrations.map(|calibration| match calibration.illuminant {
        code if code >= OTHER => D65,
        code => code,
    });
    let d65 = dng_light_source(D65).temperature;
    let (mut chosen, mut chosen_temperature, mut nearest) = (None, 0, d65);
    for (k, &code) in illuminants.iter().enumerate() {
        if code == UNKNOWN {
            continue;
        }
        let temperature = dng_light_source(code).temperature;
        let distance = (temperature - d65).abs();
        if temperature > chosen_temperature && distance <= nearest {
            chosen = Some(k);
            chosen_temperature = temperature;
            nearest = distance;
        }
    }
    let k = chosen.or_else(|| {
        let k = (0..3)
            .find(|&k| illuminants[k] == UNKNOWN && calibrations[k].color_matrix.is_some())?;
        illuminants[k] = D65;
        Some(k)
    })?;
    let calibration = &calibrations[k];
    let color_matrix = calibration.color_matrix?;
    let analog_balance = analog_balance.map_or(IDENTITY, |[r, g, b]| {
        [[r, 0.0, 0.0], [0.0, g, 0.0], [0.0, 0.0, b]]
    });
    let camera_calibration = calibration.camera_calibration.unwrap_or(IDENTITY);
    let xyz_to_camera = mul(&mul(&analog_balance, &camera_calibration), &color_matrix);
    Some(DngMatrix {
        calibration: k,
        illuminant: illuminants[k],
        xyz_to_camera: mul(&dng_light_source(illuminants[k]).to_d65, &xyz_to_camera),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: u16 = 17;
    const D50: u16 = 23;
    const D75: u16 = 22;
    const DAYLIGHT_FLUORESCENT: u16 = 12;

    // ColorMatrix1 and 2 of three DNGs in the raw.pixls.us corpus, and the
    // d65_color_matrix darktable prints for each with `-d imageio`, to its four
    // decimals.
    const PIXEL_4A: [Mat3; 2] = [
        [
            [1.0577, -0.4995, -0.0103],
            [-0.33, 1.1359, 0.221],
            [-0.0204, 0.1332, 0.5939],
        ],
        [
            [1.0626, -0.4466, -0.1116],
            [-0.4205, 1.2766, 0.1562],
            [-0.0739, 0.2132, 0.5474],
        ],
    ];
    const PIXEL_4A_DARKTABLE: Mat3 = [
        [1.0987, -0.4618, -0.1154],
        [-0.4205, 1.2766, 0.1562],
        [-0.0722, 0.2082, 0.5345],
    ];
    const Q2: [Mat3; 2] = [
        [
            [1.2587, -0.5232, -0.1496],
            [-0.361, 1.0841, 0.0277],
            [-0.0911, 0.1674, 0.207],
        ],
        [
            [0.8389, -0.3198, -0.1019],
            [-0.3834, 1.0222, 0.0661],
            [-0.1122, 0.19, 0.3415],
        ],
    ];
    const Q2_DARKTABLE: Mat3 = [
        [0.8034, -0.3171, -0.0773],
        [-0.4133, 1.0454, 0.0768],
        [-0.1310, 0.2278, 0.4516],
    ];
    const EOS_5D_MARK_III: Mat3 = [
        [0.6722, -0.0635, -0.0963],
        [-0.4287, 1.246, 0.2028],
        [-0.0908, 0.2162, 0.5668],
    ];
    const EOS_5D_MARK_III_DARKTABLE: Mat3 = [
        [0.6735, -0.0636, -0.0965],
        [-0.4287, 1.2460, 0.2028],
        [-0.0914, 0.2175, 0.5703],
    ];

    fn diagonal(d: [f32; 3]) -> Mat3 {
        [[d[0], 0.0, 0.0], [0.0, d[1], 0.0], [0.0, 0.0, d[2]]]
    }

    fn calibration(illuminant: u16, color_matrix: Mat3) -> DngCalibration {
        DngCalibration {
            illuminant,
            color_matrix: Some(color_matrix),
            camera_calibration: None,
        }
    }

    fn chosen(calibrations: [DngCalibration; 3]) -> Option<(usize, u16)> {
        dng_xyz_to_camera(&calibrations, None).map(|m| (m.calibration, m.illuminant))
    }

    fn assert_rounds_to(m: &Mat3, darktable: &Mat3) {
        for (row, expected) in m.iter().zip(darktable) {
            for (value, expected) in row.iter().zip(expected) {
                assert!((value - expected).abs() <= 5e-5, "{m:?} != {darktable:?}");
            }
        }
    }

    #[test]
    fn the_light_sources_are_darktable_s() {
        assert_eq!(dng_light_source(D65).to_d65, IDENTITY);
        assert_eq!(dng_light_source(D65).temperature, 6500);
        assert_eq!(dng_light_source(A).name, "StandardLightA");
        assert_eq!(dng_light_source(5).temperature, 0);
        assert_eq!(dng_light_source(255).name, "Other illuminant");
        assert_eq!(dng_light_source(30).name, "Other illuminant");
    }

    #[test]
    fn d65_is_chosen_in_either_order_and_kept_bit_for_bit() {
        let [a, d65] = PIXEL_4A;
        let forward = [calibration(A, a), calibration(D65, d65), Default::default()];
        let backward = [calibration(D65, d65), calibration(A, a), Default::default()];
        assert_eq!(chosen(forward), Some((1, D65)));
        assert_eq!(chosen(backward), Some((0, D65)));
        let matrix = dng_xyz_to_camera(&forward, None).unwrap().xyz_to_camera;
        assert_eq!(matrix, d65);
        // An analog balance of 1 changes nothing either.
        let balanced = dng_xyz_to_camera(&forward, Some([1.0; 3])).unwrap();
        assert_eq!(balanced.xyz_to_camera, d65);
    }

    #[test]
    fn camera_calibration_scales_the_camera_side_as_darktable_does() {
        let [a, d65] = PIXEL_4A;
        let calibrations = [
            DngCalibration {
                camera_calibration: Some(diagonal([1.0235, 1.0, 0.9555])),
                ..calibration(A, a)
            },
            DngCalibration {
                camera_calibration: Some(diagonal([1.034, 1.0, 0.9764])),
                ..calibration(D65, d65)
            },
            Default::default(),
        ];
        let matrix = dng_xyz_to_camera(&calibrations, Some([1.0; 3])).unwrap();
        assert_rounds_to(&matrix.xyz_to_camera, &PIXEL_4A_DARKTABLE);

        let calibrations = [
            DngCalibration {
                camera_calibration: Some(diagonal([1.002, 1.0, 1.0062])),
                ..calibration(D65, EOS_5D_MARK_III)
            },
            Default::default(),
            Default::default(),
        ];
        let matrix = dng_xyz_to_camera(&calibrations, Some([1.0; 3])).unwrap();
        assert_rounds_to(&matrix.xyz_to_camera, &EOS_5D_MARK_III_DARKTABLE);
    }

    #[test]
    fn without_d65_the_nearest_is_adapted_to_d65() {
        let [a, d50] = Q2;
        let calibrations = [calibration(A, a), calibration(D50, d50), Default::default()];
        let matrix = dng_xyz_to_camera(&calibrations, None).unwrap();
        assert_eq!((matrix.calibration, matrix.illuminant), (1, D50));
        assert_rounds_to(&matrix.xyz_to_camera, &Q2_DARKTABLE);
        assert_eq!(
            matrix.xyz_to_camera,
            mul(&dng_light_source(D50).to_d65, &d50)
        );
    }

    #[test]
    fn analog_balance_then_camera_calibration_then_color_matrix() {
        // AB · CC differs from CC · AB when CC mixes colours; the adaptation comes last,
        // on the left.
        let shear = [[1.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let calibrations = [
            DngCalibration {
                camera_calibration: Some(shear),
                ..calibration(D50, IDENTITY)
            },
            Default::default(),
            Default::default(),
        ];
        let matrix = dng_xyz_to_camera(&calibrations, Some([2.0, 1.0, 1.0])).unwrap();
        let ab_cc = [[2.0, 2.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        assert_eq!(
            matrix.xyz_to_camera,
            mul(&dng_light_source(D50).to_d65, &ab_cc)
        );
    }

    #[test]
    fn an_earlier_higher_temperature_is_kept() {
        let m = PIXEL_4A[1];
        // 6430 K is nearer D65 than 7500 K, but lower: darktable keeps the first.
        let hot_first = [
            calibration(D75, m),
            calibration(DAYLIGHT_FLUORESCENT, m),
            Default::default(),
        ];
        let cool_first = [
            calibration(DAYLIGHT_FLUORESCENT, m),
            calibration(D75, m),
            Default::default(),
        ];
        assert_eq!(chosen(hot_first), Some((0, D75)));
        assert_eq!(chosen(cool_first), Some((0, DAYLIGHT_FLUORESCENT)));
        // Equally far: the higher temperature, whichever comes first.
        let tie = [calibration(20, m), calibration(D75, m), Default::default()];
        assert_eq!(chosen(tie), Some((1, D75)));
    }

    #[test]
    fn other_light_sources_count_as_d65() {
        let [a, other] = Q2;
        let calibrations = [
            calibration(A, a),
            calibration(255, other),
            Default::default(),
        ];
        let matrix = dng_xyz_to_camera(&calibrations, None).unwrap();
        assert_eq!((matrix.calibration, matrix.illuminant), (1, D65));
        assert_eq!(matrix.xyz_to_camera, other);
    }

    #[test]
    fn an_unknown_light_source_is_d65_only_when_none_is_known() {
        let [a, unknown] = Q2;
        let alone = [
            Default::default(),
            calibration(UNKNOWN, unknown),
            Default::default(),
        ];
        assert_eq!(chosen(alone), Some((1, D65)));
        let beside_a = [
            calibration(UNKNOWN, unknown),
            calibration(A, a),
            Default::default(),
        ];
        assert_eq!(chosen(beside_a), Some((1, A)));
    }

    #[test]
    fn nothing_qualifies() {
        assert_eq!(chosen(Default::default()), None);
        // Codes 5 to 8 are undefined, at 0 K, and never chosen.
        let undefined = [
            calibration(5, IDENTITY),
            Default::default(),
            Default::default(),
        ];
        assert_eq!(chosen(undefined), None);
        // The chosen light source has no ColorMatrix.
        let missing = [
            calibration(A, IDENTITY),
            DngCalibration {
                illuminant: D65,
                ..Default::default()
            },
            Default::default(),
        ];
        assert_eq!(chosen(missing), None);
    }
}
