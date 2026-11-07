// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2023-2025  darktable developers (ported from src/iop/primaries.c)
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

//! RGB primaries: setup for the `primaries` kernel, ported from darktable's
//! `_calculate_adjustment_matrix()`. See `docs/ports/primaries.md`.

use apertrust_color::{self as color, Chromaticities, Mat3, Mat3d, RgbSpace};

/// One of darktable's settings: a field of `dt_iop_primaries_params_t`, in its order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parameter {
    AchromaticTintHue,
    AchromaticTintPurity,
    RedHue,
    RedPurity,
    GreenHue,
    GreenPurity,
    BlueHue,
    BluePurity,
}

impl Parameter {
    /// darktable's order.
    pub const ALL: [Self; 8] = [
        Self::AchromaticTintHue,
        Self::AchromaticTintPurity,
        Self::RedHue,
        Self::RedPurity,
        Self::GreenHue,
        Self::GreenPurity,
        Self::BlueHue,
        Self::BluePurity,
    ];

    /// darktable's field name, which the schema keeps.
    pub const fn name(self) -> &'static str {
        match self {
            Self::AchromaticTintHue => "achromatic_tint_hue",
            Self::AchromaticTintPurity => "achromatic_tint_purity",
            Self::RedHue => "red_hue",
            Self::RedPurity => "red_purity",
            Self::GreenHue => "green_hue",
            Self::GreenPurity => "green_purity",
            Self::BlueHue => "blue_hue",
            Self::BluePurity => "blue_purity",
        }
    }

    /// Its index in darktable's params struct, as the kernel's gradient slots name it.
    pub fn index(self) -> u32 {
        Self::ALL
            .iter()
            .position(|p| *p == self)
            .expect("ALL holds every parameter") as u32
    }
}

/// The module's settings: darktable params version 1, field for field. Hues are in
/// radians, purities plain ratios.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub achromatic_tint_hue: f32,
    pub achromatic_tint_purity: f32,
    pub red_hue: f32,
    pub red_purity: f32,
    pub green_hue: f32,
    pub green_purity: f32,
    pub blue_hue: f32,
    pub blue_purity: f32,
}

impl Default for Settings {
    /// darktable's defaults (its introspection's): no rotation, every primary on the
    /// gamut's edge, no tint.
    fn default() -> Self {
        Self {
            achromatic_tint_hue: 0.0,
            achromatic_tint_purity: 0.0,
            red_hue: 0.0,
            red_purity: 1.0,
            green_hue: 0.0,
            green_purity: 1.0,
            blue_hue: 0.0,
            blue_purity: 1.0,
        }
    }
}

impl Settings {
    pub fn get(&self, parameter: Parameter) -> f32 {
        let mut copy = *self;
        *copy.get_mut(parameter)
    }

    pub fn get_mut(&mut self, parameter: Parameter) -> &mut f32 {
        match parameter {
            Parameter::AchromaticTintHue => &mut self.achromatic_tint_hue,
            Parameter::AchromaticTintPurity => &mut self.achromatic_tint_purity,
            Parameter::RedHue => &mut self.red_hue,
            Parameter::RedPurity => &mut self.red_purity,
            Parameter::GreenHue => &mut self.green_hue,
            Parameter::GreenPurity => &mut self.green_purity,
            Parameter::BlueHue => &mut self.blue_hue,
            Parameter::BluePurity => &mut self.blue_purity,
        }
    }

    /// The hues of the tint, red, green and blue, in the kernel's order.
    fn hues(&self) -> [f32; 4] {
        [
            self.achromatic_tint_hue,
            self.red_hue,
            self.green_hue,
            self.blue_hue,
        ]
    }

    /// Their purities, likewise.
    fn purities(&self) -> [f32; 4] {
        [
            self.achromatic_tint_purity,
            self.red_purity,
            self.green_purity,
            self.blue_purity,
        ]
    }
}

/// The gradient slots `prepare` chooses: one of each path through the setup, a
/// primary's rotation, a primary's scale, and the white's rotation and scale.
pub const DEFAULT_GRADIENT: [Parameter; 4] = [
    Parameter::RedHue,
    Parameter::GreenPurity,
    Parameter::AchromaticTintHue,
    Parameter::AchromaticTintPurity,
];

/// What the kernel needs besides the image.
#[derive(Debug, Clone, PartialEq)]
pub struct Prepared {
    /// The adjustment, working RGB to working RGB: what `main` applies.
    pub matrix: Mat3,
    /// Whether the new primaries' matrix is singular, so the adjustment is 0.
    pub singular: bool,
    /// The rest is for the derivative entry points, which build the adjustment again from
    /// the settings.
    pub settings: Settings,
    /// The working profile's chromaticities, as darktable reads them back from it.
    pub chromaticities: Chromaticities,
    /// The working profile's XYZ D50 to RGB.
    pub to_rgb: Mat3d,
    /// The four settings the derivative entry points differentiate against, in their
    /// order. `main` ignores them.
    pub gradient: [Parameter; 4],
}

/// Bytes of `PrimariesParams`, the kernel's parameters after the pass header.
pub const PARAMS_SIZE: usize = 192;

/// The kernel's inputs for these settings on the working space `working`: the matrix
/// from [`adjustment`], rounded to f32, and what the derivative entry points need to
/// build it again.
pub fn prepare(settings: &Settings, working: &RgbSpace) -> Prepared {
    let matrix = adjustment(settings, working);
    Prepared {
        matrix: color::to_f32(matrix),
        // The only way the matrix is 0: darktable's singular fallback, since the working
        // space's own matrix is invertible.
        singular: matrix == [[0.0; 3]; 3],
        settings: *settings,
        chromaticities: working.chromaticities(),
        to_rgb: color::inverse_d(&working.rgb_to_xyz_d50()),
        gradient: DEFAULT_GRADIENT,
    }
}

/// darktable `_calculate_adjustment_matrix()`, in f64 where darktable computes in f32:
/// working RGB read as amounts of the settings' primaries, which add up to the settings'
/// white, to working RGB. 0 when those primaries' matrix is singular, as darktable's
/// `mat3SSEinv` leaves it.
pub fn adjustment(settings: &Settings, working: &RgbSpace) -> Mat3d {
    let chromaticities = working.chromaticities();
    let moved = |scaling: f32, rotation: f32, index: usize| {
        color::rotate_and_scale_primary(
            &chromaticities,
            f64::from(scaling),
            f64::from(rotation),
            index,
        )
    };
    let primaries = [
        moved(settings.red_purity, settings.red_hue, 0),
        moved(settings.green_purity, settings.green_hue, 1),
        moved(settings.blue_purity, settings.blue_hue, 2),
    ];
    // The tint moves the white from the red primary's direction, as darktable anchors it.
    let white = moved(
        settings.achromatic_tint_purity,
        settings.achromatic_tint_hue,
        0,
    );
    let rgb_to_xyz = color::rgb_to_xyz_from_chromaticities(primaries, white);
    let to_rgb = color::inverse_d(&working.rgb_to_xyz_d50());
    color::mul_d(&to_rgb, &rgb_to_xyz)
}

impl Prepared {
    /// Little-endian `PrimariesParams`, in the kernel's field order.
    pub fn params(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(PARAMS_SIZE);
        let mut vec4 = |v: [f32; 4]| {
            for x in v {
                bytes.extend_from_slice(&x.to_le_bytes());
            }
        };
        let row = |r: [f32; 3]| [r[0], r[1], r[2], 0.0];
        for r in self.matrix {
            vec4(row(r));
        }
        vec4(self.settings.hues());
        vec4(self.settings.purities());
        let c = self.chromaticities;
        let xy = |p: [f64; 2]| p.map(|v| v as f32);
        let [white, red, green, blue] = [c.white, c.primaries[0], c.primaries[1], c.primaries[2]];
        vec4(
            [xy(white), xy(red)]
                .concat()
                .try_into()
                .expect("four values"),
        );
        vec4(
            [xy(green), xy(blue)]
                .concat()
                .try_into()
                .expect("four values"),
        );
        for r in color::to_f32(self.to_rgb) {
            vec4(row(r));
        }
        for index in self.gradient.map(Parameter::index) {
            bytes.extend_from_slice(&index.to_le_bytes());
        }
        for word in [u32::from(self.singular), 0, 0, 0] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        debug_assert_eq!(bytes.len(), PARAMS_SIZE);
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec2020() -> RgbSpace {
        RgbSpace::LINEAR_REC2020
    }

    /// Test 0152's settings (`crates/golden/cases/primaries.xmp`).
    fn test_0152() -> Settings {
        Settings {
            achromatic_tint_hue: 0.579_449_4,
            achromatic_tint_purity: 0.134,
            red_hue: 0.165_806_32,
            red_purity: 0.843,
            green_hue: -0.143_117_01,
            green_purity: 1.235,
            blue_hue: 0.228_638_14,
            blue_purity: 0.82,
        }
    }

    fn apply(m: &Mat3, v: [f32; 3]) -> [f32; 3] {
        color::apply(m, v)
    }

    #[test]
    fn the_defaults_keep_greys_and_leak_a_little_red() {
        // The D50-adapted red's clipped Z moves its chromaticity off the profile's
        // matrix, so the unmoved primaries' matrix is not the profile's, as in darktable.
        let prepared = prepare(&Settings::default(), &rec2020());
        let m = prepared.matrix;
        assert!(!prepared.singular);
        for (r, row) in m.iter().enumerate() {
            let sum: f32 = row.iter().sum();
            assert!((sum - 1.0).abs() < 1e-6, "row {r}: {row:?}");
        }
        // Red leaks into green and blue, which give up as much of themselves; nothing
        // else moves.
        let (green, blue) = (2.4775e-5, 2.42450e-3);
        let expected = [
            [1.0, 0.0, 0.0],
            [green, 1.0 - green, 0.0],
            [blue, 0.0, 1.0 - blue],
        ];
        for r in 0..3 {
            for c in 0..3 {
                assert!((m[r][c] - expected[r][c]).abs() < 1e-7, "{m:?}");
            }
        }
    }

    #[test]
    fn test_0152s_matrix_is_darktables_maths() {
        // From a float64 model of darktable's maths, written apart from this crate.
        let expected = [
            [0.763_358_748_322, 0.078_978_828_588, 0.258_836_446_155],
            [0.165_695_345_849, 0.730_794_624_565, 0.072_508_537_46],
            [0.053_615_310_049, -0.055_136_435_168, 0.841_588_125_992],
        ];
        let m = prepare(&test_0152(), &rec2020()).matrix;
        for r in 0..3 {
            for c in 0..3 {
                assert!((f64::from(m[r][c]) - expected[r][c]).abs() < 1e-7, "{m:?}");
            }
        }
    }

    #[test]
    fn white_stays_white_without_a_tint() {
        let settings = Settings {
            achromatic_tint_purity: 0.0,
            ..test_0152()
        };
        let white = apply(&prepare(&settings, &rec2020()).matrix, [1.0; 3]);
        assert!(white.iter().all(|v| (v - 1.0).abs() < 1e-6), "{white:?}");
        // The tint moves it.
        let tinted = apply(&prepare(&test_0152(), &rec2020()).matrix, [1.0; 3]);
        assert!((tinted[0] - 1.101_174).abs() < 1e-5, "{tinted:?}");
        assert!((tinted[2] - 0.840_067).abs() < 1e-5, "{tinted:?}");
    }

    #[test]
    fn hues_of_minus_and_plus_pi_meet() {
        // The hue sliders' two ends: the angle π as f32 is 1.7e-7 past π, either way.
        for parameter in [
            Parameter::AchromaticTintHue,
            Parameter::RedHue,
            Parameter::GreenHue,
            Parameter::BlueHue,
        ] {
            let mut low = test_0152();
            let mut high = test_0152();
            *low.get_mut(parameter) = -std::f32::consts::PI;
            *high.get_mut(parameter) = std::f32::consts::PI;
            let (a, b) = (prepare(&low, &rec2020()), prepare(&high, &rec2020()));
            for r in 0..3 {
                for c in 0..3 {
                    let (x, y) = (a.matrix[r][c], b.matrix[r][c]);
                    assert!((x - y).abs() < 1e-5 * x.abs().max(1.0), "{parameter:?}");
                }
            }
        }
    }

    #[test]
    fn a_primary_turned_onto_the_far_edge_at_purity_1_is_singular() {
        // Red turned into the sector between green and blue lands on their edge, in line
        // with them: darktable's mat3SSEinv refuses the matrix (in f32 its rounding can
        // pass the threshold; in f64 it never does), and the adjustment is 0.
        for degrees in [125.0_f32, 150.0, 180.0, -150.0, -120.0] {
            let settings = Settings {
                red_hue: degrees.to_radians(),
                ..Settings::default()
            };
            let prepared = prepare(&settings, &rec2020());
            assert!(prepared.singular, "{degrees}°");
            assert_eq!(prepared.matrix, [[0.0; 3]; 3], "{degrees}°");
        }
        // Any settings that put the three on one line: red at 150° and green at +60°,
        // both at purity 1, both land on the edge through blue.
        let both = Settings {
            red_hue: 150_f32.to_radians(),
            green_hue: 60_f32.to_radians(),
            ..Settings::default()
        };
        assert!(prepare(&both, &rec2020()).singular);
        // Within its own sectors, or off the edge, it is not.
        for (degrees, purity) in [(115.0_f32, 1.0), (-110.0, 1.0), (150.0, 0.9), (150.0, 1.1)] {
            let settings = Settings {
                red_hue: degrees.to_radians(),
                red_purity: purity,
                ..Settings::default()
            };
            let prepared = prepare(&settings, &rec2020());
            assert!(!prepared.singular, "{degrees}° at {purity}");
            assert!(prepared.matrix.iter().flatten().all(|v| v.is_finite()));
        }
    }

    #[test]
    fn the_matrix_stays_finite_at_every_range_end() {
        let ends = |parameter: Parameter| match parameter {
            Parameter::AchromaticTintPurity => [0.0, 0.99],
            Parameter::RedPurity | Parameter::GreenPurity | Parameter::BluePurity => [0.01, 5.0],
            _ => [-std::f32::consts::PI, std::f32::consts::PI],
        };
        for parameter in Parameter::ALL {
            for end in ends(parameter) {
                for base in [Settings::default(), test_0152()] {
                    let mut settings = base;
                    *settings.get_mut(parameter) = end;
                    let m = prepare(&settings, &rec2020()).matrix;
                    assert!(
                        m.iter().flatten().all(|v| v.is_finite() && v.abs() < 1e3),
                        "{parameter:?} at {end}: {m:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_uniform_has_the_kernels_layout() {
        let mut prepared = prepare(&test_0152(), &rec2020());
        prepared.gradient = [
            Parameter::BluePurity,
            Parameter::AchromaticTintHue,
            Parameter::GreenHue,
            Parameter::RedPurity,
        ];
        let bytes = prepared.params();
        assert_eq!(bytes.len(), PARAMS_SIZE);
        let float = |at: usize| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        // The matrix by rows, w unused.
        assert_eq!(float(4), prepared.matrix[0][1]);
        assert_eq!(float(16 + 8), prepared.matrix[1][2]);
        assert_eq!(float(12), 0.0);
        // Hues, then purities: tint, red, green, blue.
        assert_eq!(float(48), 0.579_449_4);
        assert_eq!(float(48 + 12), 0.228_638_14);
        assert_eq!(float(64 + 4), 0.843);
        // The white and red, then green and blue.
        let c = prepared.chromaticities;
        assert_eq!(float(80), c.white[0] as f32);
        assert_eq!(float(80 + 12), c.primaries[0][1] as f32);
        assert_eq!(float(96 + 8), c.primaries[2][0] as f32);
        // XYZ D50 to RGB by rows.
        assert_eq!(float(112 + 16 + 4), prepared.to_rgb[1][1] as f32);
        // The slots by darktable's field index, then whether the matrix is singular.
        assert_eq!([0, 4, 8, 12].map(|k| word(160 + k)), [7, 0, 4, 3]);
        assert_eq!(word(176), 0);
    }

    #[test]
    fn parameters_are_darktables_fields_in_order() {
        let names: Vec<&str> = Parameter::ALL.iter().map(|p| p.name()).collect();
        assert_eq!(
            names,
            [
                "achromatic_tint_hue",
                "achromatic_tint_purity",
                "red_hue",
                "red_purity",
                "green_hue",
                "green_purity",
                "blue_hue",
                "blue_purity"
            ]
        );
        assert_eq!(
            Parameter::ALL.map(Parameter::index),
            [0, 1, 2, 3, 4, 5, 6, 7]
        );
        let mut settings = Settings::default();
        for (k, parameter) in Parameter::ALL.into_iter().enumerate() {
            *settings.get_mut(parameter) = k as f32;
        }
        assert_eq!(settings.hues(), [0.0, 2.0, 4.0, 6.0]);
        assert_eq!(settings.purities(), [1.0, 3.0, 5.0, 7.0]);
    }
}
