// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2025-2026  darktable developers (ported from src/iop/agx.c)
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

//! AgX: setup for the `agx` kernel, ported from darktable's `commit_params()`
//! (`_calculate_tone_mapping_params`, `_get_primaries_params`) and the matrices
//! `process()` builds (`_create_matrices`). See `docs/ports/agx.md`.
//!
//! The curve is `apertrust_color::agx_curve`'s, which the AgX tool's pickers are to
//! share. The matrices are computed in f64 and rounded once.

#![allow(
    clippy::excessive_precision,
    reason = "constants are copied digit for digit from darktable"
)]

use apertrust_plugin::NotPorted;

pub use apertrust_color::agx_curve::{Curve, Segment};
use apertrust_color::agx_curve::{CurveSettings, DEFAULT_GAMMA, EPSILON};
use apertrust_color::{self as color, Mat3, Mat3d, RgbSpace};

/// The space whose primaries AgX insets and rotates (darktable
/// `dt_iop_agx_base_primaries_t`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BasePrimaries {
    ExportProfile,
    WorkingProfile,
    Rec2020,
    DisplayP3,
    AdobeRgb,
    Srgb,
}

impl BasePrimaries {
    pub fn from_darktable(value: i32) -> Option<Self> {
        Some(match value {
            0 => Self::ExportProfile,
            1 => Self::WorkingProfile,
            2 => Self::Rec2020,
            3 => Self::DisplayP3,
            4 => Self::AdobeRgb,
            5 => Self::Srgb,
            _ => return None,
        })
    }

    /// darktable's value, the inverse of [`Self::from_darktable`].
    pub fn to_darktable(self) -> i32 {
        match self {
            Self::ExportProfile => 0,
            Self::WorkingProfile => 1,
            Self::Rec2020 => 2,
            Self::DisplayP3 => 3,
            Self::AdobeRgb => 4,
            Self::Srgb => 5,
        }
    }
}

/// The module's settings: darktable params version 7, field for field, except
/// `dynamic_range_scaling`, which only darktable's pickers use. Rotations are in
/// radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub look_lift: f32,
    pub look_slope: f32,
    pub look_brightness: f32,
    pub look_saturation: f32,
    pub look_original_hue_mix_ratio: f32,
    pub range_black_relative_ev: f32,
    pub range_white_relative_ev: f32,
    pub curve_pivot_x: f32,
    pub curve_pivot_y_linear_output: f32,
    pub curve_contrast_around_pivot: f32,
    pub curve_linear_ratio_below_pivot: f32,
    pub curve_linear_ratio_above_pivot: f32,
    pub curve_toe_power: f32,
    pub curve_shoulder_power: f32,
    pub curve_gamma: f32,
    pub auto_gamma: bool,
    pub curve_target_display_black_ratio: f32,
    pub curve_target_display_white_ratio: f32,
    pub base_primaries: BasePrimaries,
    pub disable_primaries_adjustments: bool,
    /// Red, green, blue.
    pub inset: [f32; 3],
    pub rotation: [f32; 3],
    pub master_outset_ratio: f32,
    pub master_unrotation_ratio: f32,
    pub outset: [f32; 3],
    pub unrotation: [f32; 3],
    pub completely_reverse_primaries: bool,
}

impl Settings {
    /// darktable `_set_scene_referred_default_params`: the defaults in a scene-referred
    /// workflow, and what `legacy_params` turns params versions 1 to 6 into.
    pub fn scene_referred_default() -> Self {
        let black = -10.0f32;
        let white = 6.5f32;
        // Blender-like primaries (`_set_blenderlike_primaries`).
        let rotation = [0.03540329, -0.02108586, -0.06305724];
        Self {
            look_lift: 0.0,
            look_slope: 1.0,
            look_brightness: 1.0,
            look_saturation: 1.0,
            look_original_hue_mix_ratio: 0.6,
            range_black_relative_ev: black,
            range_white_relative_ev: white,
            curve_pivot_x: -black / (white - black),
            curve_pivot_y_linear_output: 0.18,
            curve_contrast_around_pivot: 3.0,
            curve_linear_ratio_below_pivot: 0.0,
            curve_linear_ratio_above_pivot: 0.0,
            curve_toe_power: 1.5,
            curve_shoulder_power: 3.3,
            curve_gamma: DEFAULT_GAMMA,
            auto_gamma: false,
            curve_target_display_black_ratio: 0.0,
            curve_target_display_white_ratio: 1.0,
            base_primaries: BasePrimaries::Rec2020,
            disable_primaries_adjustments: false,
            inset: [0.29462451, 0.25861925, 0.14641371],
            rotation,
            master_outset_ratio: 1.0,
            master_unrotation_ratio: 0.0,
            outset: [0.290776401758, 0.263155400753, 0.045810721815],
            unrotation: rotation,
            completely_reverse_primaries: false,
        }
    }

    /// The fields the curve depends on.
    pub fn curve(&self) -> CurveSettings {
        CurveSettings {
            range_black_relative_ev: self.range_black_relative_ev,
            range_white_relative_ev: self.range_white_relative_ev,
            curve_pivot_x: self.curve_pivot_x,
            curve_pivot_y_linear_output: self.curve_pivot_y_linear_output,
            curve_contrast_around_pivot: self.curve_contrast_around_pivot,
            curve_linear_ratio_below_pivot: self.curve_linear_ratio_below_pivot,
            curve_linear_ratio_above_pivot: self.curve_linear_ratio_above_pivot,
            curve_toe_power: self.curve_toe_power,
            curve_shoulder_power: self.curve_shoulder_power,
            curve_gamma: self.curve_gamma,
            auto_gamma: self.auto_gamma,
            curve_target_display_black_ratio: self.curve_target_display_black_ratio,
            curve_target_display_white_ratio: self.curve_target_display_white_ratio,
        }
    }
}

/// The look applied after the curve, and the hue restoration. The kernel differentiates
/// against these.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    pub slope: f32,
    pub lift: f32,
    /// The power darktable derives from `look_brightness`.
    pub power: f32,
    pub saturation: f32,
    pub hue_mix: f32,
}

impl Look {
    pub fn new(s: &Settings) -> Self {
        let brightness = s.look_brightness;
        Self {
            slope: s.look_slope,
            lift: s.look_lift,
            power: if brightness < 1.0 {
                1.0 / EPSILON.max(brightness).sqrt()
            } else {
                1.0 / brightness
            },
            saturation: s.look_saturation,
            hue_mix: s.look_original_hue_mix_ratio,
        }
    }
}

/// The matrices of darktable's `_create_matrices`, in f64, as `M · v`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrices {
    pub pipe_to_base: Mat3d,
    pub base_to_rendering: Mat3d,
    /// 0 where the outbound matrix is singular, which darktable refuses to invert.
    pub rendering_to_pipe: Mat3d,
    /// The inset and rotated primaries to XYZ D50, whose Y row is the look's luminance.
    pub rendering_to_xyz: Mat3d,
}

impl Matrices {
    /// `working` is the pipeline's working space.
    pub fn new(s: &Settings, working: &RgbSpace) -> Result<Self, NotPorted> {
        // darktable `_get_primaries_params`.
        let (mut inset, mut rotation, mut outset, mut unrotation) =
            (s.inset, s.rotation, s.outset, s.unrotation);
        let (mut master_outset, mut master_unrotation) =
            (s.master_outset_ratio, s.master_unrotation_ratio);
        if s.disable_primaries_adjustments {
            inset = [0.0; 3];
            rotation = [0.0; 3];
            outset = [0.0; 3];
            unrotation = [0.0; 3];
        } else if s.completely_reverse_primaries {
            outset = inset;
            unrotation = rotation;
            master_outset = 1.0;
            master_unrotation = 1.0;
        }

        // darktable `_agx_get_base_profile`.
        let base = match s.base_primaries {
            BasePrimaries::ExportProfile => {
                return Err(NotPorted {
                    operation: "agx",
                    what: "base primaries from the export profile",
                });
            }
            BasePrimaries::WorkingProfile => *working,
            BasePrimaries::Rec2020 => RgbSpace::LINEAR_REC2020,
            BasePrimaries::DisplayP3 => RgbSpace::DISPLAY_P3,
            BasePrimaries::AdobeRgb => RgbSpace::ADOBE_RGB,
            BasePrimaries::Srgb => RgbSpace::SRGB,
        };

        let base_out = color::inverse_d(&base.rgb_to_xyz_d50());
        // darktable skips the conversion when the base profile is the working profile.
        let (pipe_to_base, base_to_pipe) = if base == *working {
            (color::IDENTITY_D, color::IDENTITY_D)
        } else {
            let pipe_to_base = color::mul_d(&base_out, &working.rgb_to_xyz_d50());
            (pipe_to_base, color::inverse_d(&pipe_to_base))
        };

        let chromaticities = base.chromaticities();
        let primaries = |scaling: [f64; 3], rotation: [f64; 3]| -> [[f64; 2]; 3] {
            std::array::from_fn(|i| {
                color::rotate_and_scale_primary(&chromaticities, scaling[i], rotation[i], i)
            })
        };
        let to_xyz = |primaries: [[f64; 2]; 3]| {
            color::rgb_to_xyz_from_chromaticities(primaries, chromaticities.white)
        };

        // Inbound: base RGB treated as coordinates over the inset, rotated primaries.
        let rendering_to_xyz = to_xyz(primaries(
            inset.map(|v| 1.0 - f64::from(v)),
            rotation.map(f64::from),
        ));
        let base_to_rendering = color::mul_d(&base_out, &rendering_to_xyz);

        // Outbound: the inverse of the same construction over the outset, unrotated
        // primaries, which restores purity and undoes the rotation.
        let outset_to_xyz = to_xyz(primaries(
            outset.map(|v| 1.0 - f64::from(master_outset) * f64::from(v)),
            unrotation.map(|v| f64::from(master_unrotation) * f64::from(v)),
        ));
        // darktable inverts with `mat3SSEinv`, which refuses a singular matrix and leaves
        // `rendering_to_base` uninitialised: an outbound primary on the white, or the white
        // on the line through two of them. The setup renders black there
        // (docs/ports/agx.md, Precision).
        let rendering_to_base = color::checked_inverse_d(&color::mul_d(&base_out, &outset_to_xyz))
            .unwrap_or([[0.0; 3]; 3]);
        let rendering_to_pipe = color::mul_d(&base_to_pipe, &rendering_to_base);

        Ok(Self {
            pipe_to_base,
            base_to_rendering,
            rendering_to_pipe,
            rendering_to_xyz,
        })
    }
}

/// Everything the kernel needs, kept apart so the gradient check can move the look's
/// parameters in the kernel's own units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Prepared {
    pub matrices: Matrices,
    pub curve: Curve,
    pub look: Look,
}

/// The size of `AgxParams` in the kernel, after the 48-byte pass header.
pub const PARAMS_SIZE: usize = 288;

pub fn prepare(settings: &Settings, working: &RgbSpace) -> Result<Prepared, NotPorted> {
    let prepared = Prepared {
        matrices: Matrices::new(settings, working)?,
        curve: Curve::new(&settings.curve()),
        look: Look::new(settings),
    };
    if settings.look_lift <= -1.0
        || prepared
            .params()
            .as_chunks::<4>()
            .0
            .iter()
            .any(|bytes| !f32::from_le_bytes(*bytes).is_finite())
    {
        return Err(NotPorted {
            operation: "agx",
            what: "settings that produce a singular curve or look",
        });
    }
    Ok(prepared)
}

impl Prepared {
    /// Little-endian `AgxParams`, field by field in the kernel's order. It sits at
    /// offset 48 of the uniform, after the pass header the engine writes.
    pub fn params(&self) -> Vec<u8> {
        let mut out = Uniform::default();
        let m = &self.matrices;
        for matrix in [m.pipe_to_base, m.base_to_rendering, m.rendering_to_pipe] {
            let matrix: Mat3 = color::to_f32(matrix);
            for row in matrix {
                out.vec4([row[0], row[1], row[2], 0.0]);
            }
        }
        let luminance = color::to_f32(m.rendering_to_xyz)[1];
        out.vec4([luminance[0], luminance[1], luminance[2], 0.0]);

        let c = &self.curve;
        out.vec4([c.black_ev, c.range_ev, c.gamma, c.slope]);
        out.vec4([c.intercept, c.target_black, c.target_white, 0.0]);
        for segment in [&c.toe, &c.shoulder] {
            // The shader reads only the selected branch. Inactive coefficients can
            // overflow during setup without changing the curve; never upload them.
            let packed = |value: f32, active: bool, placeholder: f32| {
                if !active && !value.is_finite() {
                    placeholder
                } else {
                    value
                }
            };
            out.vec4([
                segment.transition_x,
                segment.transition_y,
                packed(segment.scale, !segment.fallback, 1.0),
                packed(segment.power, !segment.fallback, 1.0),
            ]);
            out.f32(packed(segment.fallback_coefficient, segment.fallback, 0.0));
            out.f32(packed(segment.fallback_power, segment.fallback, 1.0));
            out.u32(u32::from(segment.fallback));
            out.f32(0.0);
        }

        let l = &self.look;
        out.vec4([l.slope, l.lift, l.power, l.saturation]);
        out.vec4([l.hue_mix, 0.0, 0.0, 0.0]);
        debug_assert_eq!(out.0.len(), PARAMS_SIZE);
        out.0
    }
}

#[derive(Default)]
struct Uniform(Vec<u8>);

impl Uniform {
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    fn f32(&mut self, v: f32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    fn vec4(&mut self, v: [f32; 4]) {
        debug_assert_eq!(self.0.len() % 16, 0, "vec4 must sit at a 16-byte offset");
        for c in v {
            self.f32(c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn singular_curve_and_look_are_refused_before_packing() {
        let zero_pivot = Settings {
            curve_pivot_y_linear_output: 0.0,
            curve_gamma: 10.0,
            auto_gamma: false,
            ..Settings::scene_referred_default()
        };
        assert!(prepare(&zero_pivot, &RgbSpace::LINEAR_REC2020).is_err());
        let singular_look = Settings {
            look_lift: -1.0,
            ..Settings::scene_referred_default()
        };
        assert!(prepare(&singular_look, &RgbSpace::LINEAR_REC2020).is_err());
    }

    fn defaults() -> Settings {
        Settings::scene_referred_default()
    }

    #[test]
    fn params_fill_the_kernel_struct() {
        let prepared = prepare(&defaults(), &RgbSpace::LINEAR_REC2020).unwrap();
        assert_eq!(prepared.params().len(), PARAMS_SIZE);
    }

    #[test]
    fn base_primaries_round_trip_through_darktable_values() {
        for value in 0..6 {
            let primaries = BasePrimaries::from_darktable(value).unwrap();
            assert_eq!(primaries.to_darktable(), value);
        }
        assert_eq!(BasePrimaries::from_darktable(6), None);
    }

    #[test]
    fn unmodified_primaries_cancel() {
        // With no inset, rotation or outset, the rendering space is the base space up to
        // darktable's clamp of a negative colorant (Rec2020's red), and the way out
        // undoes the way in.
        let settings = Settings {
            inset: [0.0; 3],
            rotation: [0.0; 3],
            outset: [0.0; 3],
            unrotation: [0.0; 3],
            ..defaults()
        };
        for base in [
            BasePrimaries::Rec2020,
            BasePrimaries::AdobeRgb,
            BasePrimaries::Srgb,
        ] {
            let settings = Settings {
                base_primaries: base,
                ..settings
            };
            let m = Matrices::new(&settings, &RgbSpace::LINEAR_REC2020).unwrap();
            let identity = |matrix: &Mat3d| {
                matrix.iter().enumerate().all(|(r, row)| {
                    row.iter()
                        .enumerate()
                        .all(|(c, v)| (v - if r == c { 1.0 } else { 0.0 }).abs() < 1e-9)
                })
            };
            let round_trip = color::mul_d(
                &m.rendering_to_pipe,
                &color::mul_d(&m.base_to_rendering, &m.pipe_to_base),
            );
            assert!(identity(&round_trip), "{base:?}: {m:?}");
            assert_eq!(
                identity(&m.base_to_rendering),
                base != BasePrimaries::Rec2020,
                "{base:?}: {m:?}"
            );
        }
    }

    #[test]
    fn matrices_keep_white() {
        // Every matrix is built around the same D50 white, so grey stays grey.
        for base in [
            BasePrimaries::Rec2020,
            BasePrimaries::DisplayP3,
            BasePrimaries::Srgb,
        ] {
            let settings = Settings {
                base_primaries: base,
                ..defaults()
            };
            let m = Matrices::new(&settings, &RgbSpace::LINEAR_REC2020).unwrap();
            for matrix in [m.pipe_to_base, m.base_to_rendering, m.rendering_to_pipe] {
                for row in matrix {
                    assert!(
                        (row.iter().sum::<f64>() - 1.0).abs() < 1e-9,
                        "{base:?}: {m:?}"
                    );
                }
            }
        }
    }

    const BASES: [BasePrimaries; 4] = [
        BasePrimaries::Rec2020,
        BasePrimaries::DisplayP3,
        BasePrimaries::AdobeRgb,
        BasePrimaries::Srgb,
    ];

    const ZERO: Mat3d = [[0.0; 3]; 3];

    fn finite(m: &Matrices) -> bool {
        [
            m.pipe_to_base,
            m.base_to_rendering,
            m.rendering_to_pipe,
            m.rendering_to_xyz,
        ]
        .iter()
        .flatten()
        .flatten()
        .all(|v| v.is_finite())
    }

    #[test]
    fn outbound_primaries_on_the_white_render_black() {
        // A master purity boost times a purity boost of 1 puts that outbound primary on
        // the white, and the outbound matrix is singular: one primary there leaves the
        // white no share of the other two, two make the primaries' matrix 0. darktable's
        // mat3SSEinv refuses it and leaves the outbound matrix uninitialised; the setup
        // renders black. 1.25 · 0.8 is 1 in f32 but 1 + 1.5e-8 in f64, inside the refusal.
        for (master, outset) in [
            (2.0, [0.5, 0.5, 0.0]),
            (2.0, [0.5, 0.0, 0.0]),
            (2.0, [0.0, 0.99, 0.5]),
            (2.0, [0.5, 0.5, 0.5]),
            (1.25, [0.8, 0.0, 0.0]),
        ] {
            for base in BASES {
                let settings = Settings {
                    base_primaries: base,
                    master_outset_ratio: master,
                    outset,
                    ..defaults()
                };
                let m = Matrices::new(&settings, &RgbSpace::LINEAR_REC2020).unwrap();
                assert!(finite(&m), "{base:?} {master} {outset:?}: {m:?}");
                assert_eq!(
                    m.rendering_to_pipe, ZERO,
                    "{base:?} {master} {outset:?}: {m:?}"
                );
            }
        }
    }

    #[test]
    fn primaries_matrices_are_finite_at_every_corner() {
        // The outbound settings at their ends, and the purity boost where master 2 makes
        // the product 1; then the inbound ones under reverse all, which uses them on the
        // way out too. Black exactly where an outbound primary sits on the white;
        // elsewhere darktable's matrices, at most 8.9e3 (Rec2020 under reverse all, red
        // and green attenuated 99% and turned -30° and +30°, which puts the white just
        // past the line through them).
        let turn = [-crate::ROTATION, 0.0, crate::ROTATION];
        let boost = [0.0, 0.5, 0.99];
        let ends = |k: usize, values: [f32; 3]| [k % 3, k / 3 % 3, k / 9 % 3].map(|j| values[j]);
        for base in BASES {
            for master_outset in [0.0, 1.0, 2.0] {
                for master_unrotation in [0.0, 1.0, 2.0] {
                    for k in 0..27 * 27 {
                        let settings = Settings {
                            base_primaries: base,
                            master_outset_ratio: master_outset,
                            master_unrotation_ratio: master_unrotation,
                            outset: ends(k % 27, boost),
                            unrotation: ends(k / 27, turn),
                            ..defaults()
                        };
                        let m = Matrices::new(&settings, &RgbSpace::LINEAR_REC2020).unwrap();
                        let on_white = settings.outset.iter().any(|&o| master_outset * o == 1.0);
                        assert!(finite(&m), "{settings:?}: {m:?}");
                        assert_eq!(m.rendering_to_pipe == ZERO, on_white, "{settings:?}: {m:?}");
                        let largest = m.rendering_to_pipe.iter().flatten().map(|v| v.abs());
                        assert!(largest.fold(0.0, f64::max) < 1e4, "{settings:?}: {m:?}");
                    }
                }
            }
            for k in 0..27 * 27 {
                let settings = Settings {
                    base_primaries: base,
                    inset: ends(k % 27, boost),
                    rotation: ends(k / 27, turn),
                    completely_reverse_primaries: true,
                    ..defaults()
                };
                let m = Matrices::new(&settings, &RgbSpace::LINEAR_REC2020).unwrap();
                assert!(finite(&m), "{settings:?}: {m:?}");
                assert_ne!(m.rendering_to_pipe, ZERO, "{settings:?}: {m:?}");
                let largest = m.rendering_to_pipe.iter().flatten().map(|v| v.abs());
                assert!(largest.fold(0.0, f64::max) < 1e4, "{settings:?}: {m:?}");
            }
        }
    }

    #[test]
    fn export_profile_is_not_ported() {
        let settings = Settings {
            base_primaries: BasePrimaries::ExportProfile,
            ..defaults()
        };
        assert!(prepare(&settings, &RgbSpace::LINEAR_REC2020).is_err());
    }

    #[test]
    fn brightness_maps_to_power() {
        let look = |b| {
            Look::new(&Settings {
                look_brightness: b,
                ..defaults()
            })
            .power
        };
        assert_eq!(look(1.0), 1.0);
        assert!((look(2.0) - 0.5).abs() < 1e-7);
        assert!((look(0.25) - 2.0).abs() < 1e-6);
    }
}
