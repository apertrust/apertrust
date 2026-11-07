// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/iop/vignette.c)
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

//! Vignetting: setup for the `vignette` kernel, ported from darktable's
//! `commit_params()`, which copies the parameters, and the start of `process()`, whose
//! geometry the kernel computes. See `docs/ports/vignette.md`.

use apertrust_plugin::NotPorted;

/// darktable `dt_iop_dither_t`: random noise against banding, at the precision of an
/// 8-bit or a 16-bit output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dithering {
    /// `DITHER_OFF`.
    Off,
    /// `DITHER_8BIT`: noise of 1/256.
    Bits8,
    /// `DITHER_16BIT`: noise of 1/65536.
    Bits16,
}

impl Dithering {
    pub const ALL: [Self; 3] = [Self::Off, Self::Bits8, Self::Bits16];

    pub fn from_darktable(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Off),
            1 => Some(Self::Bits8),
            2 => Some(Self::Bits16),
            _ => None,
        }
    }

    pub fn to_darktable(self) -> i32 {
        match self {
            Self::Off => 0,
            Self::Bits8 => 1,
            Self::Bits16 => 2,
        }
    }
}

/// The module's settings: darktable params version 4, field for field.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// "fall-off start": the inner radius, in percent of half the image.
    pub scale: f32,
    /// "fall-off radius": the fall-off's width, in the same units.
    pub falloff_scale: f32,
    /// −1 to 1: below 0 scales the pixel down, from 0 adds to it.
    pub brightness: f32,
    /// −1 to 1: below 0 desaturates, above saturates.
    pub saturation: f32,
    /// The centre, x then y, in half the image's width and height from its middle.
    pub center: [f32; 2],
    /// Whether the vignette's width-to-height ratio follows the image's.
    pub autoratio: bool,
    /// Without `autoratio`: up to 1 the width over the height, from 1 to 2 the height
    /// over the width taken from 2.
    pub whratio: f32,
    /// The superellipse's exponent is 2 / shape: 0 a rectangle, 1 an ellipse, 2 a
    /// diamond.
    pub shape: f32,
    pub dithering: Dithering,
    /// Whether values stay unclipped; otherwise a vignetted pixel is clipped to [0, 1].
    pub unbound: bool,
}

impl Default for Settings {
    /// darktable's defaults: its introspection's, the module having no `init`.
    fn default() -> Self {
        Self {
            scale: 80.0,
            falloff_scale: 50.0,
            brightness: -0.5,
            saturation: -0.5,
            center: [0.0, 0.0],
            autoratio: false,
            whratio: 1.0,
            shape: 1.0,
            dithering: Dithering::Off,
            unbound: true,
        }
    }
}

impl Settings {
    /// darktable's one preset, "lomo" (`init_presets`).
    pub fn lomo() -> Self {
        Self {
            scale: 40.0,
            falloff_scale: 100.0,
            brightness: -1.0,
            saturation: 0.5,
            ..Self::default()
        }
    }
}

/// One of the continuous parameters the derivative entry points differentiate against,
/// in darktable's units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parameter {
    Scale,
    FalloffScale,
    Brightness,
    Saturation,
    CenterX,
    CenterY,
    Whratio,
    Shape,
}

impl Parameter {
    pub const ALL: [Self; 8] = [
        Self::Scale,
        Self::FalloffScale,
        Self::Brightness,
        Self::Saturation,
        Self::CenterX,
        Self::CenterY,
        Self::Whratio,
        Self::Shape,
    ];

    /// The kernel's number for it, in darktable's field order.
    pub fn index(self) -> u32 {
        match self {
            Self::Scale => 0,
            Self::FalloffScale => 1,
            Self::Brightness => 2,
            Self::Saturation => 3,
            Self::CenterX => 4,
            Self::CenterY => 5,
            Self::Whratio => 6,
            Self::Shape => 7,
        }
    }

    /// Its name in the schema.
    pub fn name(self) -> &'static str {
        match self {
            Self::Scale => "scale",
            Self::FalloffScale => "falloff_scale",
            Self::Brightness => "brightness",
            Self::Saturation => "saturation",
            Self::CenterX => "center_x",
            Self::CenterY => "center_y",
            Self::Whratio => "whratio",
            Self::Shape => "shape",
        }
    }
}

/// What the kernel needs besides the image and the pass header, which gives the image's
/// size at full resolution and the pipe's scale.
#[derive(Debug, Clone, PartialEq)]
pub struct Prepared {
    pub scale: f32,
    pub falloff_scale: f32,
    pub brightness: f32,
    pub saturation: f32,
    pub center: [f32; 2],
    pub autoratio: bool,
    /// Kept within [`WHRATIO_LIMIT`], 2 − [`WHRATIO_LIMIT`].
    pub whratio: f32,
    pub shape: f32,
    pub unbound: bool,
    /// The four parameters the derivative entry points differentiate against, in their
    /// order. `main` ignores them.
    pub gradient: [Parameter; 4],
}

/// Bytes of `VignetteParams`, the kernel's parameters after the pass header.
pub const PARAMS_SIZE: usize = 64;

/// How close the ratio may come to 0 or 2, where darktable divides by zero.
pub const WHRATIO_LIMIT: f32 = 1e-3;

/// The gradient slots `prepare` chooses: the strengths, then the radii.
pub const DEFAULT_GRADIENT: [Parameter; 4] = [
    Parameter::Brightness,
    Parameter::Saturation,
    Parameter::Scale,
    Parameter::FalloffScale,
];

impl Prepared {
    /// Little-endian `VignetteParams`, in the kernel's field order.
    pub fn params(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(PARAMS_SIZE);
        let mut put = |word: [u8; 4]| bytes.extend_from_slice(&word);
        for v in [
            self.scale,
            self.falloff_scale,
            self.brightness,
            self.saturation,
            self.center[0],
            self.center[1],
            self.whratio,
            self.shape,
        ] {
            put(v.to_le_bytes());
        }
        for v in [u32::from(self.autoratio), u32::from(self.unbound), 0, 0] {
            put(v.to_le_bytes());
        }
        for parameter in &self.gradient {
            put(parameter.index().to_le_bytes());
        }
        debug_assert_eq!(bytes.len(), PARAMS_SIZE);
        bytes
    }

    /// The value a gradient slot stands for.
    pub fn value_mut(&mut self, parameter: Parameter) -> &mut f32 {
        match parameter {
            Parameter::Scale => &mut self.scale,
            Parameter::FalloffScale => &mut self.falloff_scale,
            Parameter::Brightness => &mut self.brightness,
            Parameter::Saturation => &mut self.saturation,
            Parameter::CenterX => &mut self.center[0],
            Parameter::CenterY => &mut self.center[1],
            Parameter::Whratio => &mut self.whratio,
            Parameter::Shape => &mut self.shape,
        }
    }
}

const OPERATION: &str = "vignette";

/// darktable `commit_params()`: the parameters as they are, for the kernel's setup,
/// which follows the start of `process()`. Dithering is not ported, nor the
/// raised-cosine fall-off darktable's CPU path uses only with it.
pub fn prepare(settings: &Settings) -> Result<Prepared, NotPorted> {
    let refuse = |what| {
        Err(NotPorted {
            operation: OPERATION,
            what,
        })
    };
    if settings.dithering != Dithering::Off {
        return refuse(
            "dithering, whose noise darktable draws in an order that depends on its thread \
             count, and with which its fall-off becomes a raised cosine",
        );
    }
    let values = [
        settings.scale,
        settings.falloff_scale,
        settings.brightness,
        settings.saturation,
        settings.center[0],
        settings.center[1],
        settings.whratio,
        settings.shape,
    ];
    if !values.iter().all(|v| v.is_finite()) {
        return refuse("a parameter that is not a finite number");
    }
    Ok(Prepared {
        scale: settings.scale,
        falloff_scale: settings.falloff_scale,
        brightness: settings.brightness,
        saturation: settings.saturation,
        center: settings.center,
        autoratio: settings.autoratio,
        // At exactly 0 or 2 darktable divides by zero and leaves the image unchanged;
        // this is the limit from inside, an extremely narrow vignette.
        whratio: settings.whratio.clamp(WHRATIO_LIMIT, 2.0 - WHRATIO_LIMIT),
        shape: settings.shape,
        unbound: settings.unbound,
        gradient: DEFAULT_GRADIENT,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test 0045's settings (`crates/golden/cases/vignette.xmp`), with dithering off.
    fn test_0045() -> Settings {
        Settings {
            scale: 69.4,
            falloff_scale: 32.08,
            brightness: -0.968,
            saturation: -0.82,
            center: [-0.116, 0.13],
            autoratio: false,
            whratio: 1.172,
            shape: 1.49,
            dithering: Dithering::Off,
            unbound: true,
        }
    }

    #[test]
    fn dithering_is_not_ported() {
        for dithering in [Dithering::Bits8, Dithering::Bits16] {
            let settings = Settings {
                dithering,
                ..test_0045()
            };
            assert!(prepare(&settings).is_err(), "{dithering:?}");
        }
        assert!(prepare(&test_0045()).is_ok());
    }

    #[test]
    fn values_that_are_not_finite_are_not_ported() {
        for parameter in Parameter::ALL {
            for bad in [f32::NAN, f32::INFINITY] {
                let mut prepared = prepare(&test_0045()).unwrap();
                *prepared.value_mut(parameter) = bad;
                let settings = Settings {
                    scale: prepared.scale,
                    falloff_scale: prepared.falloff_scale,
                    brightness: prepared.brightness,
                    saturation: prepared.saturation,
                    center: prepared.center,
                    whratio: prepared.whratio,
                    shape: prepared.shape,
                    ..test_0045()
                };
                assert_eq!(
                    prepare(&settings).err().map(|refusal| refusal.what),
                    Some("a parameter that is not a finite number"),
                    "{parameter:?} {bad}"
                );
            }
        }
    }

    #[test]
    fn the_ratio_stays_off_darktables_division_by_zero() {
        let ratio = |whratio| {
            prepare(&Settings {
                whratio,
                ..test_0045()
            })
            .unwrap()
            .whratio
        };
        assert_eq!(ratio(0.0), WHRATIO_LIMIT);
        assert_eq!(ratio(2.0), 2.0 - WHRATIO_LIMIT);
        for whratio in [0.01, 0.63, 1.0, 1.172, 1.99] {
            assert_eq!(ratio(whratio), whratio);
        }
    }

    #[test]
    fn prepare_passes_darktables_parameters() {
        let settings = test_0045();
        let prepared = prepare(&settings).unwrap();
        assert_eq!(
            (prepared.scale, prepared.falloff_scale),
            (settings.scale, settings.falloff_scale)
        );
        assert_eq!(
            (prepared.brightness, prepared.saturation),
            (settings.brightness, settings.saturation)
        );
        assert_eq!(prepared.center, settings.center);
        assert_eq!(
            (prepared.autoratio, prepared.whratio, prepared.shape),
            (false, settings.whratio, settings.shape)
        );
        assert!(prepared.unbound);
        assert_eq!(prepared.gradient, DEFAULT_GRADIENT);
    }

    #[test]
    fn layout_matches_the_kernel() {
        let mut prepared = prepare(&Settings {
            autoratio: true,
            unbound: false,
            ..test_0045()
        })
        .unwrap();
        prepared.gradient = [
            Parameter::CenterY,
            Parameter::Shape,
            Parameter::Whratio,
            Parameter::Scale,
        ];
        let params = prepared.params();
        assert_eq!(params.len(), PARAMS_SIZE);
        let word = |i: usize| u32::from_le_bytes(params[4 * i..4 * i + 4].try_into().unwrap());
        let float = |i: usize| f32::from_bits(word(i));
        assert_eq!(
            [float(0), float(1), float(2), float(3)],
            [69.4, 32.08, -0.968, -0.82]
        );
        assert_eq!(
            [float(4), float(5), float(6), float(7)],
            [-0.116, 0.13, 1.172, 1.49]
        );
        assert_eq!([word(8), word(9), word(10), word(11)], [1, 0, 0, 0]);
        assert_eq!([word(12), word(13), word(14), word(15)], [5, 7, 6, 0]);
    }

    #[test]
    fn a_slot_moves_its_parameter() {
        let base = prepare(&test_0045()).unwrap();
        for parameter in Parameter::ALL {
            let mut moved = base.clone();
            *moved.value_mut(parameter) += 0.25;
            let changed: Vec<Parameter> = Parameter::ALL
                .into_iter()
                .filter(|&p| moved.clone().value_mut(p) != base.clone().value_mut(p))
                .collect();
            assert_eq!(changed, [parameter]);
        }
        let indices: Vec<u32> = Parameter::ALL.iter().map(|p| p.index()).collect();
        assert_eq!(indices, (0..8).collect::<Vec<_>>());
    }

    #[test]
    fn darktable_enums_round_trip() {
        for dithering in Dithering::ALL {
            assert_eq!(
                Dithering::from_darktable(dithering.to_darktable()),
                Some(dithering)
            );
        }
        assert_eq!(Dithering::from_darktable(3), None);
    }
}
