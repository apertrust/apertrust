// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2020-2025  darktable developers (ported from src/common/chromatic_adaptation.h)
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

//! Chromatic adaptation as darktable does it (`src/common/chromatic_adaptation.h`): the
//! Bradford and CAT16 cone spaces, D50 in each, and the adaptations color calibration
//! offers. darktable's constants are kept digit for digit, so its pairs of matrices are
//! inverse only to their last decimal (about 5e-6 for CAT16, 1e-4 for Bradford). The
//! kernels' twins are in `color.slang`.

use crate::{Mat3, apply};

/// darktable `dt_adaptation_t`: the space color calibration's von Kries gains apply in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Adaptation {
    /// Bradford's cone space, gains alone (ICC v4).
    LinearBradford,
    /// CIECAM16's cone space.
    Cat16,
    /// Bradford's cone space, with the power on its third cone (CIECAM97s).
    FullBradford,
    /// Gains on CIE XYZ.
    Xyz,
    /// No adaptation: the mixer works on working RGB ("none (bypass)").
    Rgb,
}

impl Adaptation {
    /// In darktable's order, its values 0 to 4.
    pub const ALL: [Self; 5] = [
        Self::LinearBradford,
        Self::Cat16,
        Self::FullBradford,
        Self::Xyz,
        Self::Rgb,
    ];

    pub fn from_darktable(value: i32) -> Option<Self> {
        usize::try_from(value)
            .ok()
            .and_then(|i| Self::ALL.get(i).copied())
    }

    pub fn to_darktable(self) -> i32 {
        Self::ALL.iter().position(|&a| a == self).unwrap() as i32
    }

    /// darktable `convert_any_XYZ_to_LMS`: XYZ to this adaptation's cone space, the
    /// identity for XYZ and none.
    pub fn xyz_to_lms(self) -> Mat3 {
        match self {
            Self::LinearBradford | Self::FullBradford => XYZ_TO_BRADFORD_LMS,
            Self::Cat16 => XYZ_TO_CAT16_LMS,
            Self::Xyz | Self::Rgb => IDENTITY,
        }
    }

    /// darktable `convert_any_LMS_to_XYZ`.
    pub fn lms_to_xyz(self) -> Mat3 {
        match self {
            Self::LinearBradford | Self::FullBradford => BRADFORD_LMS_TO_XYZ,
            Self::Cat16 => CAT16_LMS_TO_XYZ,
            Self::Xyz | Self::Rgb => IDENTITY,
        }
    }

    /// darktable `convert_D50_to_LMS`: the pipeline's D50 white in this cone space, 1 for
    /// none. These are darktable's constants, not `xyz_to_lms` of its D50: CAT16's third
    /// differs by 4.2e-4, Bradford's by up to 4.7e-4.
    pub fn d50(self) -> [f32; 3] {
        match self {
            Self::LinearBradford | Self::FullBradford => BRADFORD_D50,
            Self::Cat16 => CAT16_D50,
            Self::Xyz => XYZ_D50,
            Self::Rgb => [1.0; 3],
        }
    }
}

pub const IDENTITY: Mat3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// Bradford's modified cone response (`XYZ_to_Bradford_LMS`).
pub const XYZ_TO_BRADFORD_LMS: Mat3 = [
    [0.8951, 0.2664, -0.1614],
    [-0.7502, 1.7135, 0.0367],
    [0.0389, -0.0685, 1.0296],
];
pub const BRADFORD_LMS_TO_XYZ: Mat3 = [
    [0.9870, -0.1471, 0.1600],
    [0.4323, 0.5184, 0.0493],
    [-0.0085, 0.0400, 0.9685],
];

/// CIECAM16's modified cone response (`XYZ_to_CAT16_LMS`).
pub const XYZ_TO_CAT16_LMS: Mat3 = [
    [0.401288, 0.650173, -0.051461],
    [-0.250268, 1.204414, 0.045854],
    [-0.002079, 0.048952, 0.953127],
];
pub const CAT16_LMS_TO_XYZ: Mat3 = [
    [1.862068, -1.011255, 0.149187],
    [0.38752, 0.621447, -0.008974],
    [-0.015841, -0.034123, 1.049964],
];

/// D50 in Bradford's cone space (`bradford_adapt_D50`).
pub const BRADFORD_D50: [f32; 3] = [0.996078, 1.020646, 0.818155];
/// D65 in Bradford's cone space (`bradford_adapt_D65`, `WB_coeffs_to_illuminant_xy`).
pub const BRADFORD_D65: [f32; 3] = [0.941238, 1.040633, 1.088932];
/// D50 in CAT16's cone space (`CAT16_adapt_D50`).
pub const CAT16_D50: [f32; 3] = [0.994535, 1.000997, 0.833036];
/// D50 in XYZ (`XYZ_adapt_D50`): not the ICC's 0.8249 for Z.
pub const XYZ_D50: [f32; 3] = [0.9642119944211994, 1.0, 0.8251882845188288];

/// darktable's `NORM_MIN` (`src/common/math.h`), 2⁻¹⁶: its floor under a norm it divides
/// by.
pub const NORM_MIN: f32 = 1.525_878_906_25e-5;

/// darktable `chroma_adapt_pixel`: `xyz`, CIE XYZ, adapted from the light whose cone
/// response is `illuminant`, in `adaptation`'s cone space as
/// [`Adaptation::xyz_to_lms`] gives it, to D50, as color calibration's picker
/// measures an area. Each cone over the illuminant's, times D50's ([`Adaptation::d50`]),
/// at unit luminance (darktable's `downscale_vector` and `upscale_vector`); non-linear
/// Bradford takes its third cone's ratio to the power `p` where it is positive, darktable's
/// `pow(0.818155 / max(NORM_MIN, illuminant[2]), 0.0834)`; none leaves it as it is.
pub fn chroma_adapt_pixel(
    xyz: [f32; 3],
    illuminant: [f32; 3],
    adaptation: Adaptation,
    p: f32,
) -> [f32; 3] {
    let y = xyz[1];
    // darktable's `downscale_vector` and `upscale_vector`, by Y, NORM_MIN where it is not
    // above it.
    let valid = y > NORM_MIN;
    let down = |v: [f32; 3]| {
        v.map(|c| {
            if valid {
                c / (y + NORM_MIN)
            } else {
                c / NORM_MIN
            }
        })
    };
    let up = |v: [f32; 3]| {
        v.map(|c| {
            if valid {
                c * (y + NORM_MIN)
            } else {
                c * NORM_MIN
            }
        })
    };
    let d50 = adaptation.d50();
    let gains = |lms: [f32; 3]| -> [f32; 3] {
        std::array::from_fn(|c| match adaptation {
            // `bradford_adapt_D50`: D50 times the ratio, the third to the power `p` where
            // positive.
            Adaptation::LinearBradford | Adaptation::FullBradford => {
                let ratio = lms[c] / illuminant[c];
                let full = adaptation == Adaptation::FullBradford && c == 2 && ratio > 0.0;
                d50[c] * if full { ratio.powf(p) } else { ratio }
            }
            // `CAT16_adapt_D50`, fully adapted, and `XYZ_adapt_D50`.
            _ => lms[c] * d50[c] / illuminant[c],
        })
    };
    match adaptation {
        Adaptation::Rgb => xyz,
        Adaptation::Xyz => up(gains(down(xyz))),
        _ => {
            let lms = down(apply(&adaptation.xyz_to_lms(), xyz));
            apply(&adaptation.lms_to_xyz(), up(gains(lms)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{apply, mul};

    fn assert_near_identity(m: &Mat3, tolerance: f32) {
        for (r, row) in m.iter().enumerate() {
            for (c, value) in row.iter().enumerate() {
                let want = if r == c { 1.0 } else { 0.0 };
                assert!((value - want).abs() < tolerance, "{m:?}");
            }
        }
    }

    #[test]
    fn darktables_pairs_are_inverse_to_their_decimals() {
        assert_near_identity(&mul(&CAT16_LMS_TO_XYZ, &XYZ_TO_CAT16_LMS), 1e-5);
        assert_near_identity(&mul(&BRADFORD_LMS_TO_XYZ, &XYZ_TO_BRADFORD_LMS), 2e-4);
        for adaptation in Adaptation::ALL {
            assert_near_identity(
                &mul(&adaptation.lms_to_xyz(), &adaptation.xyz_to_lms()),
                2e-4,
            );
        }
    }

    #[test]
    fn d50_constants_are_close_to_the_matrices_of_darktables_d50() {
        // Within the 4.7e-4 the spec records: darktable's constants are its own.
        for adaptation in [
            Adaptation::LinearBradford,
            Adaptation::Cat16,
            Adaptation::Xyz,
        ] {
            let lms = apply(&adaptation.xyz_to_lms(), XYZ_D50);
            for (got, want) in lms.iter().zip(adaptation.d50()) {
                assert!((got - want).abs() < 5e-4, "{adaptation:?}: {lms:?}");
            }
        }
    }

    #[test]
    fn darktable_values_round_trip() {
        for adaptation in Adaptation::ALL {
            assert_eq!(
                Adaptation::from_darktable(adaptation.to_darktable()),
                Some(adaptation)
            );
        }
        assert_eq!(Adaptation::from_darktable(1), Some(Adaptation::Cat16));
        assert_eq!(Adaptation::from_darktable(5), None);
        assert_eq!(Adaptation::from_darktable(-1), None);
    }

    /// darktable's `chroma_adapt_pixel` restated in f64, line for line, from
    /// `src/common/chromatic_adaptation.h`: the cone space, `downscale_vector`, the
    /// adaptation to D50, `upscale_vector` and back.
    fn darktable(xyz: [f64; 3], illuminant: [f64; 3], adaptation: Adaptation, p: f64) -> [f64; 3] {
        let wide = |m: Mat3| m.map(|row| row.map(f64::from));
        let times = |m: [[f64; 3]; 3], v: [f64; 3]| -> [f64; 3] {
            std::array::from_fn(|r| (0..3).map(|c| m[r][c] * v[c]).sum())
        };
        let norm_min = f64::from(NORM_MIN);
        let y = xyz[1];
        let valid = y > norm_min;
        let down = |v: [f64; 3]| {
            v.map(|c| {
                if valid {
                    c / (y + norm_min)
                } else {
                    c / norm_min
                }
            })
        };
        let up = |v: [f64; 3]| {
            v.map(|c| {
                if valid {
                    c * (y + norm_min)
                } else {
                    c * norm_min
                }
            })
        };
        let d50 = adaptation.d50().map(f64::from);
        match adaptation {
            Adaptation::FullBradford | Adaptation::LinearBradford => {
                let lms = down(times(wide(XYZ_TO_BRADFORD_LMS), xyz));
                let mut ratio: [f64; 3] = std::array::from_fn(|c| lms[c] / illuminant[c]);
                if adaptation == Adaptation::FullBradford && ratio[2] > 0.0 {
                    ratio[2] = ratio[2].powf(p);
                }
                let adapted = up(std::array::from_fn(|c| d50[c] * ratio[c]));
                times(wide(BRADFORD_LMS_TO_XYZ), adapted)
            }
            Adaptation::Cat16 => {
                let lms = down(times(wide(XYZ_TO_CAT16_LMS), xyz));
                let adapted = up(std::array::from_fn(|c| lms[c] * d50[c] / illuminant[c]));
                times(wide(CAT16_LMS_TO_XYZ), adapted)
            }
            Adaptation::Xyz => {
                let lms = down(xyz);
                up(std::array::from_fn(|c| lms[c] * d50[c] / illuminant[c]))
            }
            Adaptation::Rgb => xyz,
        }
    }

    /// Each adaptation against darktable's formulas restated in f64, within 1e-6 of the
    /// largest channel: colours light, dark, saturated and below `NORM_MIN`'s luminance,
    /// under lights warm, cool and green, darktable's `pp` for non-linear Bradford.
    #[test]
    fn chroma_adapt_pixel_is_darktable_s() {
        let colours: [[f32; 3]; 5] = [
            [0.9642, 1.0, 0.8249],
            [0.2, 0.18, 0.05],
            [0.05, 0.12, 0.4],
            [0.6, 0.3, 0.02],
            [1e-6, 5e-6, 2e-6],
        ];
        let lights: [[f32; 2]; 4] = [
            [0.44757, 0.40745],
            [0.3127, 0.329],
            [0.28, 0.45],
            [0.35, 0.3],
        ];
        let mut worst = 0.0_f64;
        for adaptation in Adaptation::ALL {
            for [x, y] in lights {
                let white = [x / y, 1.0, (1.0 - x - y) / y];
                let illuminant = apply(&adaptation.xyz_to_lms(), white);
                let p = (0.818155 / illuminant[2].max(NORM_MIN)).powf(0.0834);
                for xyz in colours {
                    let got = chroma_adapt_pixel(xyz, illuminant, adaptation, p);
                    let want = darktable(
                        xyz.map(f64::from),
                        illuminant.map(f64::from),
                        adaptation,
                        f64::from(p),
                    );
                    let scale = want.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
                    for (got, want) in got.iter().zip(want) {
                        let off = (f64::from(*got) - want).abs() / scale;
                        worst = worst.max(off);
                        assert!(
                            off < 1e-6,
                            "{adaptation:?} {xyz:?} under {x}, {y}: {got} {want}"
                        );
                    }
                }
            }
        }
        println!("chroma_adapt_pixel: at most {worst:.2e} of the largest channel off");
        // Adapted from D50 itself, a colour is left as it is, to darktable's constants.
        let d50 = apply(&XYZ_TO_CAT16_LMS, XYZ_D50);
        let kept = chroma_adapt_pixel([0.2, 0.18, 0.05], d50, Adaptation::Cat16, 1.0);
        for (got, want) in kept.iter().zip([0.2, 0.18, 0.05]) {
            assert!((got - want).abs() < 1e-3, "{kept:?}");
        }
    }
}
