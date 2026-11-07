// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2020-2024  darktable developers (ported from src/common/illuminants.h)
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

//! Illuminants as darktable models them (`src/common/illuminants.h`): CIE 1931 2°
//! chromaticities of the standard lights, the daylight and Planckian loci, correlated
//! colour temperature, and the light a camera's as-shot white balance neutralises. In
//! `f32` like darktable, but where darktable computes in `f64`.
//!
//! Every chromaticity here is relative to darktable's pipeline, whose white is D50: a
//! camera's illuminant comes out Bradford-adapted from its matrix's D65 to D50, so
//! daylight reads about 5000 K (`docs/ports/channelmixerrgb.md`, Setup, 7).

use crate::adaptation::{BRADFORD_D50, BRADFORD_D65, BRADFORD_LMS_TO_XYZ, XYZ_TO_BRADFORD_LMS};
use crate::{Mat3, apply};

/// darktable `dt_illuminant_t`, by its values: 8 and 9 are the detection modes, which
/// darktable compiles out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Illuminant {
    /// The pipeline's own D50 ("same as pipeline").
    Pipe,
    /// CIE A, incandescent.
    A,
    /// The CIE daylight locus at a temperature.
    D,
    /// Equal energy.
    E,
    /// A CIE fluorescent light.
    F,
    /// A CIE LED light.
    Led,
    /// The Planckian locus at a temperature.
    BlackBody,
    /// x and y as given.
    Custom,
    DetectSurfaces,
    DetectEdges,
    /// The light the camera's as-shot white balance neutralises.
    Camera,
}

impl Illuminant {
    /// In darktable's order of values, 0 to 10.
    pub const ALL: [Self; 11] = [
        Self::Pipe,
        Self::A,
        Self::D,
        Self::E,
        Self::F,
        Self::Led,
        Self::BlackBody,
        Self::Custom,
        Self::DetectSurfaces,
        Self::DetectEdges,
        Self::Camera,
    ];

    pub fn from_darktable(value: i32) -> Option<Self> {
        usize::try_from(value)
            .ok()
            .and_then(|i| Self::ALL.get(i).copied())
    }

    pub fn to_darktable(self) -> i32 {
        Self::ALL.iter().position(|&i| i == self).unwrap() as i32
    }
}

/// darktable's D50, `D50xyY`, the pipeline's white.
pub const D50_XY: [f32; 2] = [0.34567, 0.35850];
/// CIE A.
pub const A_XY: [f32; 2] = [0.44757, 0.40745];

/// The CIE fluorescent illuminants F1 to F12 (`fluorescent`).
pub const FLUORESCENT: [[f32; 2]; 12] = [
    [0.31310, 0.33727],
    [0.37208, 0.37529],
    [0.40910, 0.39430],
    [0.44018, 0.40329],
    [0.31379, 0.34531],
    [0.37790, 0.38835],
    [0.31292, 0.32933],
    [0.34588, 0.35875],
    [0.37417, 0.37281],
    [0.34609, 0.35986],
    [0.38052, 0.37713],
    [0.43695, 0.40441],
];

/// The CIE LED illuminants B1 to B5, BH1, RGB1, V1 and V2 (`led`).
pub const LED: [[f32; 2]; 9] = [
    [0.4560, 0.4078],
    [0.4357, 0.4012],
    [0.3756, 0.3723],
    [0.3422, 0.3502],
    [0.3118, 0.3236],
    [0.4474, 0.4066],
    [0.4557, 0.4211],
    [0.4560, 0.4548],
    [0.3781, 0.3775],
];

/// The temperatures darktable's loci cover, in K.
pub const TEMPERATURE_RANGE: (f32, f32) = (1667.0, 25000.0);

/// darktable `xy_to_CCT`: Hernández-Andrés, Lee and Romero's fit of the correlated
/// colour temperature, valid from 3000 K to 50000 K and returned however far off.
pub fn xy_to_cct(x: f32, y: f32) -> f32 {
    let n = (x - 0.3366) / (y - 0.1735);
    -949.86315
        + 6253.80338 * (-n / 0.92159).exp()
        + 28.70599 * (-n / 0.20039).exp()
        + 0.00004 * (-n / 0.07125).exp()
}

/// darktable `CCT_to_xy_daylight`: the CIE daylight locus from 4000 K to 25000 K. Outside
/// it, x is 0 and y −0.275, which darktable takes for "no daylight".
pub fn cct_to_xy_daylight(t: f32) -> [f32; 2] {
    let x = if (4000.0..=7000.0).contains(&t) {
        ((-4.6070e9 / t + 2.9678e6) / t + 0.09911e3) / t + 0.244063
    } else if t > 7000.0 && t <= 25000.0 {
        ((-2.0064e9 / t + 1.9018e6) / t + 0.24748e3) / t + 0.237040
    } else {
        0.0
    };
    [x, (-3.0 * x + 2.87) * x - 0.275]
}

/// darktable `CCT_to_xy_blackbody`: Kim et al.'s cubic splines of the Planckian locus,
/// from 1667 K to 25000 K; (0, 0) outside.
pub fn cct_to_xy_blackbody(t: f32) -> [f32; 2] {
    let x = if (1667.0..=4000.0).contains(&t) {
        ((-0.2661239e9 / t - 0.2343589e6) / t + 0.8776956e3) / t + 0.179910
    } else if t > 4000.0 && t <= 25000.0 {
        ((-3.0258469e9 / t + 2.1070379e6) / t + 0.2226347e3) / t + 0.240390
    } else {
        0.0
    };
    let y = if (1667.0..=2222.0).contains(&t) {
        ((-1.1063814 * x - 1.34811020) * x + 2.18555832) * x - 0.20219683
    } else if t > 2222.0 && t <= 4000.0 {
        ((-0.9549476 * x - 1.37418593) * x + 2.09137015) * x - 0.16748867
    } else if t > 4000.0 && t <= 25000.0 {
        ((3.0817580 * x - 5.87338670) * x + 3.75112997) * x - 0.37001483
    } else {
        0.0
    };
    [x, y]
}

/// darktable `illuminant_xy_to_XYZ`: an illuminant's XYZ at Y = 1.
pub fn illuminant_xy_to_xyz([x, y]: [f32; 2]) -> [f32; 3] {
    [x / y, 1.0, (1.0 - x - y) / y]
}

/// darktable `xy_to_uv`. Its comment says CIE 1960, but it is Judd's 1935 UCS scaled by
/// 12, with 1.882 where Judd's 0.15735 × 12 is 1.8882; and its `1.9116` is a `double`,
/// so that sum is taken in `f64`.
pub fn xy_to_uv([x, y]: [f32; 2]) -> [f32; 2] {
    let denominator = 12.0 * y - 1.882 * x + 2.9088;
    let u = (f64::from(5.5932f32 * x) + 1.9116 * f64::from(y)) as f32;
    let v = 7.8972 * y;
    [u / denominator, v / denominator]
}

/// darktable `CCT_reverse_lookup`: the temperature of the nearest of 65536 points, spaced
/// by the fourth power from 1667 K to 25000 K, on the daylight locus from 4000 K and the
/// Planckian below, by distance in xy. The first point wins a tie, where darktable's
/// threads may pick another.
pub fn cct_reverse_lookup(x: f32, y: f32) -> f32 {
    const SAMPLES: usize = 1 << 16;
    let (min, max) = TEMPERATURE_RANGE;
    let mut best = (f32::MAX, 0.0);
    for i in 0..SAMPLES {
        let step = (i as f32 / (SAMPLES - 1) as f32).powf(4.0);
        let t = min + step * (max - min);
        let [xb, yb] = if t >= 4000.0 {
            cct_to_xy_daylight(t)
        } else {
            cct_to_xy_blackbody(t)
        };
        let radius = ((xb - x) * (xb - x) + (yb - y) * (yb - y)).sqrt();
        if radius < best.0 {
            best = (radius, t);
        }
    }
    best.1
}

/// darktable `illuminant_to_xy`: the chromaticity of a standard illuminant, or `None`
/// where darktable keeps the stored x and y (custom, the detection modes, a camera with no
/// illuminant). Its `switch` falls through: daylight outside its locus to the Planckian,
/// the Planckian outside its range to the camera, the camera without one to custom.
/// `fluorescent` and `led` index the tables.
pub fn illuminant_to_xy(
    illuminant: Illuminant,
    temperature: f32,
    fluorescent: usize,
    led: usize,
    camera: Option<[f32; 2]>,
) -> Option<[f32; 2]> {
    let found = |[x, y]: [f32; 2]| (x != 0.0 && y != 0.0).then_some([x, y]);
    let camera = || camera.and_then(found);
    let black_body = || found(cct_to_xy_blackbody(temperature)).or_else(camera);
    match illuminant {
        Illuminant::Pipe => Some(D50_XY),
        Illuminant::E => Some([1.0 / 3.0; 2]),
        Illuminant::A => Some(A_XY),
        Illuminant::F => FLUORESCENT.get(fluorescent).copied().and_then(found),
        Illuminant::Led => LED.get(led).copied().and_then(found),
        Illuminant::D => found(cct_to_xy_daylight(temperature)).or_else(black_body),
        Illuminant::BlackBody => black_body(),
        Illuminant::Camera => camera(),
        Illuminant::Custom | Illuminant::DetectSurfaces | Illuminant::DetectEdges => None,
    }
}

/// darktable `matrice_pseudoinverse` for a 3×3 matrix: `m (mᵀm)⁻¹`, by Gauss–Jordan on
/// the normal equations in `f32`, without pivoting. For an invertible `m` it is `m⁻ᵀ`.
pub fn pseudoinverse(m: &Mat3) -> Mat3 {
    let mut work = [[0.0f32; 6]; 3];
    for (i, row) in work.iter_mut().enumerate() {
        for (j, value) in row.iter_mut().enumerate() {
            *value = if j == i + 3 { 1.0 } else { 0.0 };
        }
        for j in 0..3 {
            for line in m {
                row[j] += line[i] * line[j];
            }
        }
    }
    for i in 0..3 {
        let pivot = work[i][i];
        for value in &mut work[i] {
            *value /= pivot;
        }
        for k in 0..3 {
            if k == i {
                continue;
            }
            let (factor, pivot_row) = (work[k][i], work[i]);
            for (value, pivot) in work[k].iter_mut().zip(pivot_row) {
                *value -= pivot * factor;
            }
        }
    }
    std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            let mut out = 0.0;
            for k in 0..3 {
                out += work[j][k + 3] * m[i][k];
            }
            out
        })
    })
}

/// darktable `WB_coeffs_to_illuminant_xy`: the chromaticity of the light that white
/// balance multipliers `wb` neutralise, seen through `cam_to_xyz` (as
/// [`pseudoinverse`] gives it, transposed) under D65, then Bradford-adapted to D50 with
/// darktable's rounded constants.
pub fn white_balance_to_illuminant_xy(cam_to_xyz: &Mat3, wb: [f32; 3]) -> [f32; 2] {
    let m = cam_to_xyz;
    let xyz: [f32; 3] =
        std::array::from_fn(|c| m[0][c] / wb[0] + m[1][c] / wb[1] + m[2][c] / wb[2]);
    let lms = apply(&XYZ_TO_BRADFORD_LMS, xyz);
    let lms: [f32; 3] = std::array::from_fn(|c| BRADFORD_D50[c] * (lms[c] / BRADFORD_D65[c]));
    let [x, y, z] = apply(&BRADFORD_LMS_TO_XYZ, lms);
    // darktable divides by Y, then Y by itself: 1.
    let (x, z, y) = (x / y, z / y, 1.0);
    let sum = x + y + z;
    [x / sum, y / sum]
}

/// darktable `find_temperature_from_raw_coeffs`: the light a camera's white balance
/// multipliers neutralise, from its XYZ to camera matrix under D65, or `None` when a
/// multiplier is not a normal, non-zero number. `wb` is darktable's
/// `img->wb_coeffs` times its `custom_wb`; its scale cancels.
pub fn camera_illuminant_xy(xyz_to_camera: &Mat3, wb: [f32; 3]) -> Option<[f32; 2]> {
    if !wb.iter().all(|v| v.is_normal()) {
        return None;
    }
    let cam_to_xyz = pseudoinverse(xyz_to_camera);
    let xy = white_balance_to_illuminant_xy(&cam_to_xyz, wb);
    xy.iter().all(|v| v.is_finite()).then_some(xy)
}

/// darktable `_check_if_close_to_daylight`: the correlated colour temperature of a
/// chromaticity, and the model darktable's defaults describe it with: daylight or
/// Planckian when either at that temperature lies within 0.005 in darktable's UCS
/// ([`xy_to_uv`]), whichever is nearer, a tie going to daylight; otherwise custom. Below
/// 3000 K, where `xy_to_cct`'s fit fails, the temperature is the reverse lookup's.
pub fn closest_cct_model(x: f32, y: f32) -> (f32, Illuminant) {
    let mut t = xy_to_cct(x, y);
    if t < 3000.0 && t > 1667.0 {
        t = cct_reverse_lookup(x, y);
    }
    let reference = xy_to_uv([x, y]);
    let distance = |illuminant| {
        let test = illuminant_to_xy(illuminant, t, FLUORESCENT.len(), LED.len(), None)
            .unwrap_or([0.0, 0.0]);
        let [u, v] = xy_to_uv(test);
        let (du, dv) = (u - reference[0], v - reference[1]);
        (du * du + dv * dv).sqrt()
    };
    let (daylight, black_body) = (distance(Illuminant::D), distance(Illuminant::BlackBody));
    let model = if black_body < 0.005 || daylight < 0.005 {
        if black_body < daylight {
            Illuminant::BlackBody
        } else {
            Illuminant::D
        }
    } else {
        Illuminant::Custom
    };
    (t, model)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// darktable's camera matrices for the cameras of its integration tests, rawspeed's
    /// `cameras.xml` at the commit darktable pins, divided by 10000.
    fn matrix(rows: [[f32; 3]; 3]) -> Mat3 {
        rows.map(|row| row.map(|v| v / 10000.0))
    }

    fn close(got: [f32; 2], want: [f32; 2], tolerance: f32) -> bool {
        (got[0] - want[0]).abs() < tolerance && (got[1] - want[1]).abs() < tolerance
    }

    /// The loci restated in `f64`, from the papers' coefficients darktable copies.
    fn daylight_f64(t: f64) -> [f64; 2] {
        let x = if t <= 7000.0 {
            -4.6070e9 / t.powi(3) + 2.9678e6 / t.powi(2) + 0.09911e3 / t + 0.244063
        } else {
            -2.0064e9 / t.powi(3) + 1.9018e6 / t.powi(2) + 0.24748e3 / t + 0.237040
        };
        [x, -3.0 * x * x + 2.87 * x - 0.275]
    }

    fn planckian_f64(t: f64) -> [f64; 2] {
        let x = if t <= 4000.0 {
            -0.2661239e9 / t.powi(3) - 0.2343589e6 / t.powi(2) + 0.8776956e3 / t + 0.179910
        } else {
            -3.0258469e9 / t.powi(3) + 2.1070379e6 / t.powi(2) + 0.2226347e3 / t + 0.240390
        };
        let y = if t <= 2222.0 {
            -1.1063814 * x.powi(3) - 1.34811020 * x * x + 2.18555832 * x - 0.20219683
        } else if t <= 4000.0 {
            -0.9549476 * x.powi(3) - 1.37418593 * x * x + 2.09137015 * x - 0.16748867
        } else {
            3.0817580 * x.powi(3) - 5.87338670 * x * x + 3.75112997 * x - 0.37001483
        };
        [x, y]
    }

    #[test]
    fn loci_match_their_restatement() {
        for t in [
            1667.0, 2000.0, 2222.0, 2500.0, 3927.06, 4000.0, 5003.0, 6500.0, 7000.0,
        ] {
            let [x, y] = planckian_f64(t);
            assert!(
                close(cct_to_xy_blackbody(t as f32), [x as f32, y as f32], 2e-6),
                "{t}"
            );
        }
        for t in [
            4000.0, 4422.0, 5003.0, 6500.0, 7000.0, 7001.0, 12000.0, 25000.0,
        ] {
            let [x, y] = daylight_f64(t);
            assert!(
                close(cct_to_xy_daylight(t as f32), [x as f32, y as f32], 2e-6),
                "{t}"
            );
        }
        // The spec's values: D at 5003 K, the Planckian at 3927 K.
        assert!(close(cct_to_xy_daylight(5003.0), [0.34565, 0.35860], 1e-5));
        assert!(close(cct_to_xy_blackbody(3927.0), [0.38382, 0.37878], 1e-5));
        // Outside their ranges, the markers darktable tests for.
        assert_eq!(cct_to_xy_daylight(3999.0), [0.0, -0.275]);
        assert_eq!(cct_to_xy_blackbody(1666.0), [0.0, 0.0]);
        assert_eq!(cct_to_xy_blackbody(25001.0), [0.0, 0.0]);
    }

    #[test]
    fn standard_illuminants_fall_through_like_darktables_switch() {
        let camera = Some([0.3, 0.31]);
        let xy = |illuminant, t, camera| illuminant_to_xy(illuminant, t, 2, 4, camera);
        assert_eq!(xy(Illuminant::Pipe, 5000.0, None), Some(D50_XY));
        assert_eq!(xy(Illuminant::E, 5000.0, None), Some([1.0 / 3.0; 2]));
        assert_eq!(xy(Illuminant::A, 5000.0, None), Some(A_XY));
        assert_eq!(xy(Illuminant::F, 5000.0, None), Some(FLUORESCENT[2]));
        assert_eq!(xy(Illuminant::Led, 5000.0, None), Some(LED[4]));
        assert_eq!(
            illuminant_to_xy(Illuminant::F, 5000.0, 12, 0, None),
            None,
            "past the table"
        );
        // Daylight below 4000 K is the Planckian; the Planckian outside its range is the
        // camera's, then none.
        assert_eq!(
            xy(Illuminant::D, 3927.0, None),
            Some(cct_to_xy_blackbody(3927.0))
        );
        assert_eq!(
            xy(Illuminant::D, 6500.0, None),
            Some(cct_to_xy_daylight(6500.0))
        );
        assert_eq!(xy(Illuminant::BlackBody, 1500.0, camera), camera);
        assert_eq!(xy(Illuminant::D, 30000.0, camera), camera);
        assert_eq!(xy(Illuminant::BlackBody, 30000.0, None), None);
        assert_eq!(xy(Illuminant::Camera, 5000.0, camera), camera);
        assert_eq!(xy(Illuminant::Camera, 5000.0, None), None);
        for kept in [
            Illuminant::Custom,
            Illuminant::DetectSurfaces,
            Illuminant::DetectEdges,
        ] {
            assert_eq!(xy(kept, 5000.0, camera), None);
        }
        assert_eq!(FLUORESCENT[2], [0.40910, 0.39430]);
        assert_eq!(LED[8], [0.3781, 0.3775]);
        for illuminant in Illuminant::ALL {
            assert_eq!(
                Illuminant::from_darktable(illuminant.to_darktable()),
                Some(illuminant)
            );
        }
        assert_eq!(Illuminant::from_darktable(10), Some(Illuminant::Camera));
        assert_eq!(Illuminant::from_darktable(11), None);
    }

    #[test]
    fn the_pseudoinverse_of_a_square_matrix_is_its_inverse_transposed() {
        let m = matrix([
            [13426.0, -6334.0, -1177.0],
            [-4244.0, 12136.0, 2371.0],
            [-580.0, 1303.0, 5980.0],
        ]);
        let p = pseudoinverse(&m);
        // m⁻ᵀ: pᵀ m = I.
        let identity: Mat3 = std::array::from_fn(|r| {
            std::array::from_fn(|c| (0..3).map(|k| p[k][r] * m[k][c]).sum())
        });
        for (r, row) in identity.iter().enumerate() {
            for (c, value) in row.iter().enumerate() {
                let want = if r == c { 1.0 } else { 0.0 };
                assert!((value - want).abs() < 1e-5, "{p:?}");
            }
        }
    }

    #[test]
    fn camera_illuminants_are_darktables() {
        // darktable's own results, the x and y its defaults stored in the
        // integration tests' sidecars from these cameras' as-shot multipliers (the
        // temperature entries, at "as shot to reference"): test 0174 on mire1.cr2, a
        // Canon EOS 40D, and test 0165 on mire1-xtrans.raf, a Fujifilm X-T4.
        let canon = matrix([
            [6071.0, -747.0, -856.0],
            [-7653.0, 15365.0, 2441.0],
            [-2025.0, 2553.0, 7315.0],
        ]);
        let xy = camera_illuminant_xy(&canon, [2.115234375, 1.0, 1.3984375]).unwrap();
        assert!(
            close(xy, [0.3819675147533417, 0.3699879050254822], 2e-7),
            "{xy:?}"
        );
        let fuji = matrix([
            [13426.0, -6334.0, -1177.0],
            [-4244.0, 12136.0, 2371.0],
            [-580.0, 1303.0, 5980.0],
        ]);
        let wb = [
            f32::from_le_bytes([0xea, 0xfa, 0xf4, 0x3f]),
            1.0,
            f32::from_le_bytes([0x58, 0x14, 0x0c, 0x40]),
        ];
        let xy = camera_illuminant_xy(&fuji, wb).unwrap();
        assert!(
            close(xy, [0.3892036974430084, 0.4166232645511627], 2e-7),
            "{xy:?}"
        );
        // The scale of the multipliers cancels.
        let scaled = camera_illuminant_xy(&fuji, wb.map(|v| v * 1024.0)).unwrap();
        assert!(close(scaled, xy, 1e-7));
        // No illuminant without normal multipliers.
        assert_eq!(camera_illuminant_xy(&fuji, [0.0, 1.0, 1.0]), None);
        assert_eq!(camera_illuminant_xy(&fuji, [f32::NAN, 1.0, 1.0]), None);
    }

    #[test]
    fn temperatures_and_models_are_darktables() {
        // darktable's defaults on its integration tests' raws: the camera's
        // chromaticity, the temperature and illuminant `_check_if_close_to_daylight`
        // stored for it.
        for (xy, temperature, model) in [
            // Test 0174, mire1.cr2: below 4000 K the daylight model is the Planckian, a
            // tie that goes to daylight.
            (
                [0.3819675147533417, 0.3699879050254822],
                3927.0595703125,
                Illuminant::D,
            ),
            // Test 0146, xtransIV.raf: the daylight locus.
            (
                [0.36629632115364075, 0.3818664252758026],
                4421.984375,
                Illuminant::D,
            ),
            // Test 0095, hlrecovery.arw: the Planckian locus.
            (
                [0.3587544858455658, 0.35791802406311035],
                4555.39794921875,
                Illuminant::BlackBody,
            ),
            // Test 0165, mire1-xtrans.raf: too far from both.
            (
                [0.3892036974430084, 0.4166232645511627],
                4005.07470703125,
                Illuminant::Custom,
            ),
        ] {
            let (t, found) = closest_cct_model(xy[0], xy[1]);
            assert!((t - temperature).abs() < 0.01, "{xy:?}: {t}");
            assert_eq!(found, model, "{xy:?}");
        }
    }

    #[test]
    fn the_reverse_lookup_finds_the_loci_temperatures() {
        for t in [1800.0, 2200.0, 2700.0, 2950.0] {
            let [x, y] = cct_to_xy_blackbody(t);
            let found = cct_reverse_lookup(x, y);
            assert!((found - t).abs() < 1.0, "{t}: {found}");
        }
        // Below 3000 K the fit is replaced by the lookup.
        let [x, y] = cct_to_xy_blackbody(2500.0);
        let (t, model) = closest_cct_model(x, y);
        assert!((t - 2500.0).abs() < 1.0, "{t}");
        assert_eq!(model, Illuminant::D, "a tie below 4000 K goes to daylight");
    }

    #[test]
    fn darktables_uv_is_judds_scaled() {
        let [u, v] = xy_to_uv([0.3, 0.3]);
        let denominator = 12.0 * 0.3 - 1.882 * 0.3 + 2.9088;
        assert!((u - (5.5932 * 0.3 + 1.9116 * 0.3) / denominator).abs() < 1e-7);
        assert!((v - 7.8972 * 0.3 / denominator).abs() < 1e-7);
        assert_eq!(illuminant_xy_to_xyz([0.25, 0.5]), [0.5, 1.0, 0.5]);
    }
}
