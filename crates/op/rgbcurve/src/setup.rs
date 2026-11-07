// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2019-2026  darktable developers (ported from src/iop/rgbcurve.c)
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

//! RGB curve: setup for the `rgbcurve` kernel, ported from darktable's `commit_params()`
//! and `_generate_curve_lut()`. See `docs/ports/rgbcurve.md`.

use apertrust_color::WorkingProfile;
use apertrust_color::curve::{self, Interpolation, Knot, MAX_NODES};
use apertrust_plugin::NotPorted;

/// darktable `curve_autoscale`: whether one curve drives every channel or each channel
/// has its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// `DT_S_SCALE_AUTOMATIC_RGB`, "RGB, linked channels": the red curve alone, applied
    /// to each channel or to a norm of the pixel.
    Linked,
    /// `DT_S_SCALE_MANUAL_RGB`, "RGB, independent channels": each channel its own curve.
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

/// darktable `dt_iop_rgb_norms_t`: how a linked curve preserves colours, shared with RGB
/// levels.
pub use apertrust_color::RgbNorm as Norm;

/// One curve node, in the curve's own coordinates: linear, or lightness when middle
/// grey is compensated.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Node {
    pub x: f32,
    pub y: f32,
}

/// One of darktable's three curves: its nodes, the first `curve_num_nodes` of
/// `curve_nodes`, and its `curve_type`.
#[derive(Debug, Clone, PartialEq)]
pub struct Curve {
    pub nodes: Vec<Node>,
    pub interpolation: Interpolation,
}

impl Curve {
    /// darktable's default: two nodes, (0, 0) and (1, 1), monotone Hermite.
    pub fn identity() -> Self {
        Self {
            nodes: vec![Node { x: 0.0, y: 0.0 }, Node { x: 1.0, y: 1.0 }],
            interpolation: Interpolation::MonotoneHermite,
        }
    }
}

/// The module's settings: darktable params version 1, field for field.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// Red, green and blue; in linked mode, red alone counts.
    pub curves: [Curve; 3],
    pub mode: Mode,
    /// The nodes are lightness, Lab L / 100, rather than linear values, so middle grey
    /// sits at 0.5.
    pub compensate_middle_grey: bool,
    /// Used in linked mode only.
    pub preserve_colors: Norm,
}

impl Default for Settings {
    /// darktable's defaults (`init`): identity curves, linked, preserving colours by
    /// luminance, middle grey not compensated.
    fn default() -> Self {
        Self {
            curves: [Curve::identity(), Curve::identity(), Curve::identity()],
            mode: Mode::Linked,
            compensate_middle_grey: false,
            preserve_colors: Norm::Luminance,
        }
    }
}

/// One curve as the kernel evaluates it: cubic Hermite segments through its knots,
/// linear values, and past its last knot `end_value · (v / x_last)^end_power`.
#[derive(Debug, Clone, PartialEq)]
pub struct KernelCurve {
    pub knots: Vec<Knot>,
    pub end_value: f32,
    pub end_power: f32,
}

impl KernelCurve {
    fn identity() -> Self {
        let knot = |v| Knot {
            x: v,
            y: v,
            tangent: 1.0,
        };
        Self {
            knots: vec![knot(0.0), knot(1.0)],
            end_value: 1.0,
            end_power: 1.0,
        }
    }
}

/// A value of the kernel's curves, as the derivative entry points take it.
///
/// A slot naming a knot past its curve's count, or a curve the mode does not read, stands
/// for nothing the kernel uses, and its gradient is 0. Two slots naming the same value
/// split nothing: the last of them gets its whole gradient, the others 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    /// The curve: 0 red (or the linked curve), 1 green, 2 blue.
    pub curve: usize,
    /// The knot, for `X`, `Y` and `Tangent`; ignored for `EndValue` and `EndPower`.
    pub node: usize,
    pub kind: SlotKind,
}

impl Slot {
    /// The knot the kernel compares, which it takes as 0 for a curve's end.
    fn kernel_node(&self) -> u32 {
        match self.kind {
            SlotKind::X | SlotKind::Y | SlotKind::Tangent => self.node as u32,
            SlotKind::EndValue | SlotKind::EndPower => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotKind {
    X,
    Y,
    Tangent,
    EndValue,
    EndPower,
}

impl SlotKind {
    fn index(self) -> u32 {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Tangent => 2,
            Self::EndValue => 3,
            Self::EndPower => 4,
        }
    }
}

/// What the kernel needs besides the image.
#[derive(Debug, Clone, PartialEq)]
pub struct Prepared {
    pub mode: Mode,
    pub preserve_colors: Norm,
    /// The Y row of the working space's RGB to XYZ D50 matrix, for the luminance norm.
    pub luminance: [f32; 3],
    pub curves: [KernelCurve; 3],
    /// The four values the derivative entry points differentiate against, in their
    /// order. `main` ignores them.
    pub gradient: [Slot; 4],
}

/// Bytes of `RgbCurveParams`, the kernel's parameters after the pass header.
pub const PARAMS_SIZE: usize = 128;
/// Bytes of the knot table at binding 3: [`MAX_NODES`] `vec4<f32>` per curve.
pub const KNOTS_SIZE: usize = 3 * MAX_NODES * 16;

/// The gradient slots `prepare` chooses: the linked (red) curve's first two knots, the
/// second's tangent and its end power.
pub const DEFAULT_GRADIENT: [Slot; 4] = [
    Slot {
        curve: 0,
        node: 0,
        kind: SlotKind::Y,
    },
    Slot {
        curve: 0,
        node: 1,
        kind: SlotKind::Y,
    },
    Slot {
        curve: 0,
        node: 1,
        kind: SlotKind::Tangent,
    },
    Slot {
        curve: 0,
        node: 0,
        kind: SlotKind::EndPower,
    },
];

impl Prepared {
    /// Little-endian `RgbCurveParams`, in the kernel's field order.
    pub fn params(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(PARAMS_SIZE);
        let mut put = |word: [u8; 4]| bytes.extend_from_slice(&word);
        let per_curve = |f: &dyn Fn(&KernelCurve) -> f32| {
            [
                f(&self.curves[0]),
                f(&self.curves[1]),
                f(&self.curves[2]),
                0.0,
            ]
        };
        for v in [self.luminance[0], self.luminance[1], self.luminance[2], 0.0] {
            put(v.to_le_bytes());
        }
        for curve in &self.curves {
            put((curve.knots.len() as u32).to_le_bytes());
        }
        put(0u32.to_le_bytes());
        for v in per_curve(&|c| c.end_value) {
            put(v.to_le_bytes());
        }
        for v in per_curve(&|c| c.end_power) {
            put(v.to_le_bytes());
        }
        put((self.mode.to_darktable() as u32).to_le_bytes());
        put((self.preserve_colors.to_darktable() as u32).to_le_bytes());
        put(0u32.to_le_bytes());
        put(0u32.to_le_bytes());
        for field in [
            |s: &Slot| s.curve as u32,
            |s: &Slot| s.kernel_node(),
            |s: &Slot| s.kind.index(),
        ] {
            for slot in &self.gradient {
                put(field(slot).to_le_bytes());
            }
        }
        debug_assert_eq!(bytes.len(), PARAMS_SIZE);
        bytes
    }

    /// The knot table: per curve, [`MAX_NODES`] `(x, y, tangent, 0)`, unused ones zero.
    pub fn knots(&self) -> Vec<u8> {
        let mut bytes = vec![0u8; KNOTS_SIZE];
        for (c, curve) in self.curves.iter().enumerate() {
            for (k, knot) in curve.knots.iter().enumerate() {
                let at = (c * MAX_NODES + k) * 16;
                for (i, v) in [knot.x, knot.y, knot.tangent].into_iter().enumerate() {
                    bytes[at + 4 * i..at + 4 * i + 4].copy_from_slice(&v.to_le_bytes());
                }
            }
        }
        bytes
    }

    /// The value a gradient slot stands for. Panics for a knot the curve does not have.
    pub fn value_mut(&mut self, slot: Slot) -> &mut f32 {
        let curve = &mut self.curves[slot.curve];
        match slot.kind {
            SlotKind::X => &mut curve.knots[slot.node].x,
            SlotKind::Y => &mut curve.knots[slot.node].y,
            SlotKind::Tangent => &mut curve.knots[slot.node].tangent,
            SlotKind::EndValue => &mut curve.end_value,
            SlotKind::EndPower => &mut curve.end_power,
        }
    }
}

const OPERATION: &str = "rgbcurve";

pub fn prepare(settings: &Settings, profile: &WorkingProfile) -> Result<Prepared, NotPorted> {
    // darktable builds all three curves; in linked mode only red's is read, so the
    // others need not be valid.
    let used = match settings.mode {
        Mode::Linked => 1,
        Mode::Independent => 3,
    };
    let mut curves = [
        KernelCurve::identity(),
        KernelCurve::identity(),
        KernelCurve::identity(),
    ];
    for (curve, kernel_curve) in settings.curves.iter().zip(&mut curves).take(used) {
        *kernel_curve = kernel_curve_from(curve, settings.compensate_middle_grey, profile)?;
    }
    Ok(Prepared {
        mode: settings.mode,
        preserve_colors: settings.preserve_colors,
        luminance: profile.rgb_to_xyz_d50[1],
        curves,
        gradient: DEFAULT_GRADIENT,
    })
}

/// darktable `_generate_curve_lut()` for one curve, without the table: the nodes in
/// linear values, the slope of darktable's interpolating curve at each, and the
/// extrapolation fitted past the last.
fn kernel_curve_from(
    curve: &Curve,
    compensate_middle_grey: bool,
    profile: &WorkingProfile,
) -> Result<KernelCurve, NotPorted> {
    let linear = |v: f32| {
        if compensate_middle_grey {
            profile.uncompensate_middle_grey(v)
        } else {
            v
        }
    };
    let x: Vec<f32> = curve.nodes.iter().map(|node| linear(node.x)).collect();
    let y: Vec<f32> = curve.nodes.iter().map(|node| linear(node.y)).collect();
    // Where darktable's interpolation fails, it keeps whatever its table held before.
    let tangents = curve::tangents(curve.interpolation, &x, &y).ok_or(NotPorted {
        operation: OPERATION,
        what: "a curve without 2 to 20 nodes in strictly increasing x",
    })?;
    let knots: Vec<Knot> = (0..x.len())
        .map(|i| Knot {
            x: x[i],
            y: y[i],
            tangent: tangents[i],
        })
        .collect();
    let (end_value, end_power) = curve::extrapolation(&knots);
    Ok(KernelCurve {
        knots,
        end_value,
        end_power,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec2020() -> WorkingProfile {
        WorkingProfile::linear_rec2020()
    }

    #[test]
    fn the_default_curve_is_the_identity() {
        let prepared = prepare(&Settings::default(), &rec2020()).unwrap();
        for curve in &prepared.curves {
            assert_eq!(curve.knots, KernelCurve::identity().knots);
            assert_eq!(curve.end_value, 1.0);
            assert!((curve.end_power - 1.0).abs() < 1e-6, "{}", curve.end_power);
        }
    }

    #[test]
    fn compensated_nodes_become_linear() {
        let settings = Settings {
            curves: [
                Curve {
                    nodes: vec![
                        Node { x: 0.0, y: 0.0 },
                        Node { x: 0.5, y: 0.6 },
                        Node { x: 1.0, y: 1.0 },
                    ],
                    interpolation: Interpolation::MonotoneHermite,
                },
                Curve::identity(),
                Curve::identity(),
            ],
            compensate_middle_grey: true,
            ..Settings::default()
        };
        let prepared = prepare(&settings, &rec2020()).unwrap();
        let knot = prepared.curves[0].knots[1];
        assert!((knot.x - 0.184187).abs() < 1e-5, "{knot:?}");
        assert!((knot.y - rec2020().uncompensate_middle_grey(0.6)).abs() < 1e-7);
    }

    #[test]
    fn extrapolation_continues_a_power_curve() {
        // A curve that is y = x² at its nodes extrapolates with a power near 2.
        let x = [0.0, 0.2, 0.4, 0.6, 0.8];
        let curve = Curve {
            nodes: x.iter().map(|&x| Node { x, y: x * x }).collect(),
            interpolation: Interpolation::CubicSpline,
        };
        let settings = Settings {
            curves: [curve, Curve::identity(), Curve::identity()],
            ..Settings::default()
        };
        let prepared = prepare(&settings, &rec2020()).unwrap();
        let curve = &prepared.curves[0];
        assert!((curve.end_value - 0.64).abs() < 1e-6);
        assert!((curve.end_power - 2.0).abs() < 0.05, "{}", curve.end_power);
    }

    #[test]
    fn a_curve_ending_at_zero_extrapolates_to_zero() {
        let curve = Curve {
            nodes: vec![Node { x: 0.0, y: 0.5 }, Node { x: 1.0, y: 0.0 }],
            interpolation: Interpolation::CatmullRom,
        };
        let settings = Settings {
            curves: [curve, Curve::identity(), Curve::identity()],
            ..Settings::default()
        };
        let curve = &prepare(&settings, &rec2020()).unwrap().curves[0];
        assert_eq!(curve.end_value, 0.0);
        assert!(curve.end_power.is_finite());
    }

    #[test]
    fn unused_curves_need_not_be_valid() {
        let broken = Curve {
            nodes: vec![Node { x: 0.5, y: 0.5 }],
            interpolation: Interpolation::CubicSpline,
        };
        let linked = Settings {
            curves: [Curve::identity(), broken.clone(), broken],
            ..Settings::default()
        };
        assert!(prepare(&linked, &rec2020()).is_ok());
        let independent = Settings {
            mode: Mode::Independent,
            ..linked
        };
        assert!(prepare(&independent, &rec2020()).is_err());
    }

    #[test]
    fn layout_matches_the_kernel() {
        let mut prepared = prepare(&Settings::default(), &rec2020()).unwrap();
        prepared.mode = Mode::Independent;
        prepared.preserve_colors = Norm::Power;
        let params = prepared.params();
        assert_eq!(params.len(), PARAMS_SIZE);
        let word = |i: usize| u32::from_le_bytes(params[4 * i..4 * i + 4].try_into().unwrap());
        let float = |i: usize| f32::from_bits(word(i));
        assert_eq!(float(1), rec2020().rgb_to_xyz_d50[1][1]);
        assert_eq!([word(4), word(5), word(6)], [2, 2, 2]);
        assert_eq!(float(8), 1.0);
        assert_eq!([word(16), word(17)], [1, 6]);
        // Slots: curves, nodes, kinds.
        assert_eq!([word(20), word(24), word(28)], [0, 0, 1]);
        assert_eq!([word(23), word(27), word(31)], [0, 0, 4]);
        let knots = prepared.knots();
        assert_eq!(knots.len(), KNOTS_SIZE);
        let knot_word = |i: usize| f32::from_le_bytes(knots[4 * i..4 * i + 4].try_into().unwrap());
        // Curve 1's second knot: (1, 1, tangent 1).
        let at = (MAX_NODES + 1) * 4;
        assert_eq!(
            [knot_word(at), knot_word(at + 1), knot_word(at + 2)],
            [1.0; 3]
        );
    }

    #[test]
    fn an_end_slot_ignores_its_node() {
        let mut prepared = prepare(&Settings::default(), &rec2020()).unwrap();
        prepared.gradient[3] = Slot {
            curve: 2,
            node: 7,
            kind: SlotKind::EndValue,
        };
        let params = prepared.params();
        let word = |i: usize| u32::from_le_bytes(params[4 * i..4 * i + 4].try_into().unwrap());
        assert_eq!([word(23), word(27), word(31)], [2, 0, 3]);
    }

    #[test]
    fn darktable_enums_round_trip() {
        // The norms' round trip is `apertrust-color`'s, `rgb_norms_are_darktables`.
        for mode in [Mode::Linked, Mode::Independent] {
            assert_eq!(Mode::from_darktable(mode.to_darktable()), Some(mode));
        }
    }
}
