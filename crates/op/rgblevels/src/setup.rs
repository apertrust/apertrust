// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2019-2026  darktable developers (ported from src/iop/rgblevels.c)
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

//! RGB levels: setup for the `rgblevels` kernel, ported from darktable's
//! `commit_params()` and `_compute_lut()`. See `docs/ports/rgblevels.md`.

use apertrust_color::{RgbNorm, WorkingProfile};
use apertrust_plugin::NotPorted;

/// darktable `dt_iop_rgblevels_autoscale_t`: whether one set of levels drives every
/// channel or each channel has its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// `DT_IOP_RGBLEVELS_LINKED_CHANNELS`, "RGB, linked channels": the red levels alone,
    /// applied to each channel or to a norm of the pixel.
    Linked,
    /// `DT_IOP_RGBLEVELS_INDEPENDENT_CHANNELS`, "RGB, independent channels": each channel
    /// its own levels.
    Independent,
}

impl Mode {
    pub fn from_darktable(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Linked),
            1 => Some(Self::Independent),
            _ => None,
        }
    }

    pub fn to_darktable(self) -> i32 {
        match self {
            Self::Linked => 0,
            Self::Independent => 1,
        }
    }
}

/// One channel's levels, `levels[c]` in darktable: its black, grey and white points, in
/// the working space's linear values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Levels {
    pub black: f32,
    pub grey: f32,
    pub white: f32,
}

impl Levels {
    /// darktable's defaults (`init`): black 0, grey 0.5, white 1, the identity.
    pub const DEFAULT: Self = Self {
        black: 0.0,
        grey: 0.5,
        white: 1.0,
    };

    /// darktable `_compute_lut()`'s `inv_gamma`: the power of the curve these levels
    /// give, 10 to the grey point's offset from the middle of black and white, in half
    /// their distance. In f32, where darktable takes the power of 10 in f64 and rounds
    /// it; the kernel computes the same in its setup.
    pub fn power(&self) -> f32 {
        let delta = (self.white - self.black) / 2.0;
        let mid = self.black + delta;
        let offset = (self.grey - mid) / delta;
        (offset * std::f32::consts::LN_10).exp()
    }

    /// darktable `process()`'s `mult`: 1 over the distance from black to white.
    pub fn scale(&self) -> f32 {
        1.0 / (self.white - self.black)
    }

    /// The value at `point`.
    pub fn get(&self, point: Point) -> f32 {
        match point {
            Point::Black => self.black,
            Point::Grey => self.grey,
            Point::White => self.white,
        }
    }

    fn get_mut(&mut self, point: Point) -> &mut f32 {
        match point {
            Point::Black => &mut self.black,
            Point::Grey => &mut self.grey,
            Point::White => &mut self.white,
        }
    }
}

/// The module's settings: darktable params version 1, field for field.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub mode: Mode,
    /// Used in linked mode only.
    pub preserve_colors: RgbNorm,
    /// Red, green and blue; in linked mode, red alone counts.
    pub levels: [Levels; 3],
}

impl Default for Settings {
    /// darktable's defaults (`init`): linked, preserving colours by luminance, every
    /// channel's levels at 0, 0.5 and 1.
    fn default() -> Self {
        Self {
            mode: Mode::Linked,
            preserve_colors: RgbNorm::Luminance,
            levels: [Levels::DEFAULT; 3],
        }
    }
}

/// One of a channel's three points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Point {
    Black,
    Grey,
    White,
}

impl Point {
    pub const ALL: [Self; 3] = [Self::Black, Self::Grey, Self::White];

    fn index(self) -> u32 {
        match self {
            Self::Black => 0,
            Self::Grey => 1,
            Self::White => 2,
        }
    }
}

/// A level the derivative entry points differentiate against: a channel's black, grey or
/// white point, in darktable's units.
///
/// When linked, the kernel reads the first channel's levels alone, so a slot naming
/// green or blue stands for nothing and its gradient is 0. Two slots naming the same
/// level each get its whole gradient.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    /// 0 red (or the linked levels), 1 green, 2 blue.
    pub channel: usize,
    pub point: Point,
}

/// What the kernel needs besides the image.
#[derive(Debug, Clone, PartialEq)]
pub struct Prepared {
    pub mode: Mode,
    pub preserve_colors: RgbNorm,
    /// The Y row of the working space's RGB to XYZ D50 matrix, for the luminance norm.
    pub luminance: [f32; 3],
    /// The levels the kernel reads: when linked, the first channel's in every channel, as
    /// darktable's `commit_params()` copies them.
    pub levels: [Levels; 3],
    /// The four levels the derivative entry points differentiate against, in their
    /// order. `main` ignores them.
    pub gradient: [Slot; 4],
}

/// Bytes of `RgbLevelsParams`, the kernel's parameters after the pass header.
pub const PARAMS_SIZE: usize = 112;

/// The gradient slots `prepare` chooses: the first channel's black, grey and white, then
/// green's grey, which stands for nothing when linked.
pub const DEFAULT_GRADIENT: [Slot; 4] = [
    Slot {
        channel: 0,
        point: Point::Black,
    },
    Slot {
        channel: 0,
        point: Point::Grey,
    },
    Slot {
        channel: 0,
        point: Point::White,
    },
    Slot {
        channel: 1,
        point: Point::Grey,
    },
];

impl Prepared {
    /// Little-endian `RgbLevelsParams`, in the kernel's field order.
    pub fn params(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(PARAMS_SIZE);
        let mut put = |word: [u8; 4]| bytes.extend_from_slice(&word);
        for point in Point::ALL {
            for levels in &self.levels {
                put(levels.get(point).to_le_bytes());
            }
            put(0f32.to_le_bytes());
        }
        for v in [self.luminance[0], self.luminance[1], self.luminance[2], 0.0] {
            put(v.to_le_bytes());
        }
        put((self.mode.to_darktable() as u32).to_le_bytes());
        put((self.preserve_colors.to_darktable() as u32).to_le_bytes());
        put(0u32.to_le_bytes());
        put(0u32.to_le_bytes());
        for slot in &self.gradient {
            put((slot.channel as u32).to_le_bytes());
        }
        for slot in &self.gradient {
            put(slot.point.index().to_le_bytes());
        }
        debug_assert_eq!(bytes.len(), PARAMS_SIZE);
        bytes
    }

    /// The level a gradient slot stands for.
    pub fn value_mut(&mut self, slot: Slot) -> &mut f32 {
        self.levels[slot.channel].get_mut(slot.point)
    }
}

const OPERATION: &str = "rgblevels";

/// darktable `commit_params()`: the levels each channel uses, and the working space's
/// luminance for the luminance norm. The power and scale `_compute_lut()` and
/// `process()` derive from the levels are the kernel's setup, so that its derivatives
/// reach the levels.
pub fn prepare(settings: &Settings, profile: &WorkingProfile) -> Result<Prepared, NotPorted> {
    let levels = match settings.mode {
        Mode::Linked => [settings.levels[0]; 3],
        Mode::Independent => settings.levels,
    };
    // Linked, darktable reads the first channel's levels alone, so the others need not
    // be valid.
    let used = match settings.mode {
        Mode::Linked => 1,
        Mode::Independent => 3,
    };
    for channel in &settings.levels[..used] {
        check(channel)?;
    }
    Ok(Prepared {
        mode: settings.mode,
        preserve_colors: settings.preserve_colors,
        luminance: profile.rgb_to_xyz_d50[1],
        levels,
        gradient: DEFAULT_GRADIENT,
    })
}

/// darktable's own invariant for one channel's levels, which its handles and pickers
/// keep: 0 ≤ black ≤ grey ≤ white, white above black. It bounds the power to [0.1, 10]
/// and the scale to a finite positive value, and keeps a norm above black positive.
fn check(levels: &Levels) -> Result<(), NotPorted> {
    let refuse = |what| {
        Err(NotPorted {
            operation: OPERATION,
            what,
        })
    };
    let Levels { black, grey, white } = *levels;
    if ![black, grey, white].iter().all(|v| v.is_finite()) {
        return refuse("a black, grey or white point that is not a finite number");
    }
    if black < 0.0 {
        return refuse("a black point below 0");
    }
    if white <= black {
        return refuse("a white point at or below the black point");
    }
    if !(black..=white).contains(&grey) {
        return refuse("a grey point outside the black and white points");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec2020() -> WorkingProfile {
        WorkingProfile::linear_rec2020()
    }

    /// Test 0054's red levels (`crates/golden/cases/rgblevels.xmp`).
    const TEST_0054: Levels = Levels {
        black: 0.011636,
        grey: 0.355420,
        white: 0.6208815,
    };

    #[test]
    fn the_default_levels_are_the_identity() {
        assert_eq!(Levels::DEFAULT.power(), 1.0);
        assert_eq!(Levels::DEFAULT.scale(), 1.0);
    }

    #[test]
    fn the_power_is_darktables() {
        // darktable: delta 0.304623, mid 0.316259, tmp 0.128556, 10^tmp in f64.
        let darktable = 10f64.powf(f64::from(
            (TEST_0054.grey - (TEST_0054.black + (TEST_0054.white - TEST_0054.black) / 2.0))
                / ((TEST_0054.white - TEST_0054.black) / 2.0),
        ));
        assert!((f64::from(TEST_0054.power()) - darktable).abs() < 1e-6 * darktable);
        assert!(
            (TEST_0054.power() - 1.3445).abs() < 1e-3,
            "{}",
            TEST_0054.power()
        );
        // Grey at black or white: the power's range, 0.1 to 10.
        let low = Levels {
            grey: 0.0,
            ..Levels::DEFAULT
        };
        let high = Levels {
            grey: 1.0,
            ..Levels::DEFAULT
        };
        assert!((low.power() - 0.1).abs() < 1e-6 && (high.power() - 10.0).abs() < 1e-5);
    }

    #[test]
    fn linked_levels_are_the_first_channels() {
        let settings = Settings {
            levels: [
                TEST_0054,
                Levels {
                    black: 0.5,
                    grey: 0.2,
                    white: 0.1,
                },
                Levels::DEFAULT,
            ],
            ..Settings::default()
        };
        let prepared = prepare(&settings, &rec2020()).unwrap();
        assert_eq!(prepared.levels, [TEST_0054; 3]);
        let independent = Settings {
            mode: Mode::Independent,
            ..settings
        };
        assert!(prepare(&independent, &rec2020()).is_err());
    }

    #[test]
    fn levels_outside_darktables_order_are_not_ported() {
        let refused = |black, grey, white| {
            let settings = Settings {
                levels: [Levels { black, grey, white }; 3],
                ..Settings::default()
            };
            prepare(&settings, &rec2020())
                .err()
                .map(|refusal| refusal.what)
        };
        // Grey past white: the power would be 10^39, infinite in f32.
        assert_eq!(
            refused(0.0, 1.0, 0.05),
            Some("a grey point outside the black and white points")
        );
        assert_eq!(
            refused(0.3, 0.2, 0.8),
            Some("a grey point outside the black and white points")
        );
        // A negative black lets a norm of 0 lie above it, which the ratio divides by.
        assert_eq!(refused(-0.01, 0.5, 1.0), Some("a black point below 0"));
        assert_eq!(
            refused(0.0, f32::NAN, 1.0),
            Some("a black, grey or white point that is not a finite number")
        );
        assert_eq!(
            refused(0.0, 0.5, f32::INFINITY),
            Some("a black, grey or white point that is not a finite number")
        );
        // Grey at black or at white is darktable's too: the powers 0.1 and 10.
        assert_eq!(refused(0.2, 0.2, 0.6), None);
        assert_eq!(refused(0.2, 0.6, 0.6), None);
    }

    #[test]
    fn white_at_or_below_black_is_not_ported() {
        for white in [0.2, 0.1] {
            let settings = Settings {
                levels: [Levels {
                    black: 0.2,
                    grey: 0.2,
                    white,
                }; 3],
                ..Settings::default()
            };
            assert!(prepare(&settings, &rec2020()).is_err());
        }
    }

    #[test]
    fn layout_matches_the_kernel() {
        let settings = Settings {
            mode: Mode::Independent,
            preserve_colors: RgbNorm::Power,
            levels: [
                TEST_0054,
                Levels {
                    black: 0.1,
                    grey: 0.2,
                    white: 0.3,
                },
                Levels::DEFAULT,
            ],
        };
        let mut prepared = prepare(&settings, &rec2020()).unwrap();
        prepared.gradient[3] = Slot {
            channel: 2,
            point: Point::White,
        };
        let params = prepared.params();
        assert_eq!(params.len(), PARAMS_SIZE);
        let word = |i: usize| u32::from_le_bytes(params[4 * i..4 * i + 4].try_into().unwrap());
        let float = |i: usize| f32::from_bits(word(i));
        // Black, grey and white, a float4 each across the channels.
        assert_eq!([float(0), float(1), float(2)], [TEST_0054.black, 0.1, 0.0]);
        assert_eq!([float(4), float(5), float(6)], [TEST_0054.grey, 0.2, 0.5]);
        assert_eq!([float(8), float(9), float(10)], [TEST_0054.white, 0.3, 1.0]);
        assert_eq!(float(13), rec2020().rgb_to_xyz_d50[1][1]);
        assert_eq!([word(16), word(17)], [1, 6]);
        // Slots: channels, then points.
        assert_eq!([word(20), word(21), word(22), word(23)], [0, 0, 0, 2]);
        assert_eq!([word(24), word(25), word(26), word(27)], [0, 1, 2, 2]);
    }

    #[test]
    fn a_slot_moves_its_level() {
        let mut prepared = prepare(&Settings::default(), &rec2020()).unwrap();
        let slot = Slot {
            channel: 0,
            point: Point::Grey,
        };
        *prepared.value_mut(slot) += 0.1;
        assert_eq!(prepared.levels[0].grey, 0.6);
        assert_eq!(prepared.levels[1], Levels::DEFAULT);
    }

    #[test]
    fn darktable_enums_round_trip() {
        for mode in [Mode::Linked, Mode::Independent] {
            assert_eq!(Mode::from_darktable(mode.to_darktable()), Some(mode));
        }
        assert_eq!(Mode::from_darktable(2), None);
    }
}
