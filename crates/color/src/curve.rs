// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2011-2026  darktable developers (ported from src/common/curve_tools.c,
// src/develop/imageop_math.h and src/iop/rgbcurve.c)
// Copyright (C) 2004-2008  Shawn Freeman, Udi Fuchs (nikon_curve.c from UFRaw, on which
// darktable's curve_tools.c is partly based)
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

//! darktable's curve interpolation (`src/common/curve_tools.c`), in f32 like darktable,
//! and the curve RGB curve's kernel evaluates: cubic Hermite segments through the nodes,
//! with the tangents each of darktable's interpolators gives them. On every segment that
//! is the cubic darktable samples into its table, so the kernel needs no table. Past the
//! last node, darktable's power law (`dt_iop_estimate_exp`), fitted as RGB curve fits it.
//!
//! RGB curve's setup builds its kernel's curves with it, and the Tone curve module draws
//! its Luma curve with it, so the curve drawn is the curve rendered. darktable's
//! `tonecurve`, `basecurve` and `colorzones` use the same interpolators.

/// Most nodes a curve has: darktable's `MAX_ANCHORS`.
pub const MAX_NODES: usize = 20;

/// darktable's `curve_type`: how a curve interpolates between its nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interpolation {
    /// `CUBIC_SPLINE`: a natural cubic spline, with zero second derivative at both ends.
    CubicSpline,
    /// `CATMULL_ROM`, "centripetal spline" in darktable's interface: tangents from the
    /// neighbouring nodes.
    CatmullRom,
    /// `MONOTONE_HERMITE`, "monotonic spline": Fritsch–Carlson tangents, monotone
    /// between monotone nodes. darktable's default.
    MonotoneHermite,
}

impl Interpolation {
    /// The interpolation darktable stores as `type`.
    pub fn from_darktable(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::CubicSpline),
            1 => Some(Self::CatmullRom),
            2 => Some(Self::MonotoneHermite),
            _ => None,
        }
    }

    pub fn to_darktable(self) -> i32 {
        match self {
            Self::CubicSpline => 0,
            Self::CatmullRom => 1,
            Self::MonotoneHermite => 2,
        }
    }
}

/// One node of the kernel's curve: its position and its slope dy/dx.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Knot {
    pub x: f32,
    pub y: f32,
    pub tangent: f32,
}

/// The slope of darktable's curve at each node, so that the cubic Hermite segments
/// through `(x, y, slope)` are darktable's curve. `None` where darktable's
/// `interpolate_set` fails: fewer than two nodes, more than [`MAX_NODES`], or `x` not
/// strictly increasing.
pub fn tangents(interpolation: Interpolation, x: &[f32], y: &[f32]) -> Option<Vec<f32>> {
    let n = x.len();
    if !(2..=MAX_NODES).contains(&n) || y.len() != n || x.windows(2).any(|w| w[1] <= w[0]) {
        return None;
    }
    Some(match interpolation {
        Interpolation::CubicSpline => spline_tangents(x, y, &spline_cubic_set(x, y)?),
        Interpolation::CatmullRom => catmull_rom_set(x, y),
        Interpolation::MonotoneHermite => {
            let mut m = monotone_hermite_set(x, y);
            m.truncate(n);
            m
        }
    })
}

/// The kernel's curve at `v`, clamped to [0, 1] as darktable clamps its table: flat at
/// the first node's `y` below it, the cubic Hermite segment `v` falls in otherwise,
/// continued past the last node with the last segment's cubic. The CPU twin of the
/// kernel's `curve_at` below the last node, which [`extrapolation`] samples, and the
/// tests' reference.
pub fn evaluate(knots: &[Knot], v: f32) -> f32 {
    let n = knots.len();
    let v = v.max(knots[0].x);
    // darktable's segment search: the first segment whose right node lies above v, or
    // the last.
    let i = (0..n - 2).find(|&i| v < knots[i + 1].x).unwrap_or(n - 2);
    hermite(&knots[i], &knots[i + 1], v).clamp(0.0, 1.0)
}

/// darktable `dt_iop_estimate_exp` on the curve at 0.7, 0.8, 0.9 and 1 times its last
/// node's x, as RGB curve's `_generate_curve_lut` samples it: the power law
/// `y0 · (v / x0)^g` through the last of those points whose exponent is the average
/// fitted to the others, as `(y0, g)`. darktable samples its 16-bit table; this samples
/// the curve itself. A degenerate exponent (a curve ending at 0) is 1.
pub fn extrapolation(knots: &[Knot]) -> (f32, f32) {
    let last = knots[knots.len() - 1].x;
    let x = [0.7 * last, 0.8 * last, 0.9 * last, last];
    let y = x.map(|v| evaluate(knots, v));
    let (x0, y0) = (x[3], y[3]);
    let mut sum = 0.0f32;
    let mut count = 0;
    for k in 0..3 {
        let (yy, xx) = (y[k] / y0, x[k] / x0);
        if yy > 0.0 && xx > 0.0 {
            sum += yy.ln() / xx.ln();
            count += 1;
        }
    }
    let power = if count > 0 {
        sum * (1.0 / count as f32)
    } else {
        1.0
    };
    (y0, if power.is_finite() { power } else { 1.0 })
}

/// The cubic Hermite segment between `a` and `b` at `v`: darktable `catmull_rom_val`.
pub fn hermite(a: &Knot, b: &Knot, v: f32) -> f32 {
    let h = b.x - a.x;
    let t = (v - a.x) / h;
    let t2 = t * t;
    let t3 = t * t2;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    h00 * a.y + h10 * h * a.tangent + h01 * b.y + h11 * h * b.tangent
}

/// darktable `spline_cubic_set` with its fixed boundary conditions: zero second
/// derivative at both ends (`ibcbeg = ibcend = 2`, `ybcbeg = ybcend = 0`). The second
/// derivative at each node.
fn spline_cubic_set(t: &[f32], y: &[f32]) -> Option<Vec<f32>> {
    let n = t.len();
    let mut a = vec![0.0f32; 3 * n];
    let mut b = vec![0.0f32; n];
    // darktable divides by double literals and stores the quotient as a float.
    let sixth = |d: f32| (f64::from(d) / 6.0) as f32;
    let third = |d: f32| (f64::from(d) / 3.0) as f32;
    // First equation: second derivative 0.
    b[0] = 0.0;
    a[1] = 1.0;
    a[3] = 0.0;
    for i in 1..n - 1 {
        b[i] = (y[i + 1] - y[i]) / (t[i + 1] - t[i]) - (y[i] - y[i - 1]) / (t[i] - t[i - 1]);
        a[2 + (i - 1) * 3] = sixth(t[i] - t[i - 1]);
        a[1 + i * 3] = third(t[i + 1] - t[i - 1]);
        a[(i + 1) * 3] = sixth(t[i + 1] - t[i]);
    }
    // Last equation: second derivative 0.
    b[n - 1] = 0.0;
    a[2 + (n - 2) * 3] = 0.0;
    a[1 + (n - 1) * 3] = 1.0;
    d3_np_fs(&mut a, &b)
}

/// darktable `d3_np_fs`: solves a tridiagonal system, stored by diagonals in `a` (the
/// superdiagonal in `a[3 i]`, the diagonal in `a[3 i + 1]`, the subdiagonal in
/// `a[3 i + 2]`), without pivoting. `None` when a diagonal entry is zero.
fn d3_np_fs(a: &mut [f32], b: &[f32]) -> Option<Vec<f32>> {
    let n = b.len();
    if (0..n).any(|i| a[1 + i * 3] == 0.0) {
        return None;
    }
    let mut x = b.to_vec();
    for i in 1..n {
        let xmult = a[2 + (i - 1) * 3] / a[1 + (i - 1) * 3];
        a[1 + i * 3] -= xmult * a[i * 3];
        x[i] -= xmult * x[i - 1];
    }
    x[n - 1] /= a[1 + (n - 1) * 3];
    for i in (0..n - 1).rev() {
        x[i] = (x[i] - a[(i + 1) * 3] * x[i + 1]) / a[1 + i * 3];
    }
    Some(x)
}

/// A cubic spline's slope at each node from its second derivatives `ypp`: each node's
/// segment to the right at its left end, the last node's segment at its right end. In
/// f64, rounded once.
fn spline_tangents(t: &[f32], y: &[f32], ypp: &[f32]) -> Vec<f32> {
    let n = t.len();
    let slope = |i: usize, right_end: bool| {
        let (t0, t1) = (f64::from(t[i]), f64::from(t[i + 1]));
        let (y0, y1) = (f64::from(y[i]), f64::from(y[i + 1]));
        let (p0, p1) = (f64::from(ypp[i]), f64::from(ypp[i + 1]));
        let h = t1 - t0;
        let secant = (y1 - y0) / h;
        if right_end {
            secant + h * (p0 / 6.0 + p1 / 3.0)
        } else {
            secant - h * (p0 / 3.0 + p1 / 6.0)
        }
    };
    (0..n)
        .map(|i| {
            if i + 1 < n {
                slope(i, false)
            } else {
                slope(n - 2, true)
            }
        })
        .map(|m| m as f32)
        .collect()
}

/// darktable `catmull_rom_set`: each node's slope from its neighbours, one-sided at the
/// ends.
fn catmull_rom_set(x: &[f32], y: &[f32]) -> Vec<f32> {
    let n = x.len();
    let mut m = vec![0.0f32; n];
    m[0] = (y[1] - y[0]) / (x[1] - x[0]);
    for i in 1..n - 1 {
        m[i] = (y[i + 1] - y[i - 1]) / (x[i + 1] - x[i - 1]);
    }
    m[n - 1] = (y[n - 1] - y[n - 2]) / (x[n - 1] - x[n - 2]);
    m
}

/// darktable `monotone_hermite_set`: Fritsch–Carlson tangents. Like darktable's, the
/// result has `n + 1` entries, the last only scratch for the final step, and each step
/// of the limiter sees the tangent the previous step left.
fn monotone_hermite_set(x: &[f32], y: &[f32]) -> Vec<f32> {
    // darktable's EPSILON, 2 * FLT_MIN.
    const EPSILON: f32 = 2.0 * f32::MIN_POSITIVE;
    let n = x.len();
    let mut delta = vec![0.0f32; n];
    let mut m = vec![0.0f32; n + 1];
    for i in 0..n - 1 {
        delta[i] = (y[i + 1] - y[i]) / (x[i + 1] - x[i]);
    }
    delta[n - 1] = delta[n - 2];
    m[0] = delta[0];
    m[n - 1] = delta[n - 1];
    for i in 1..n - 1 {
        m[i] = (delta[i - 1] + delta[i]) * 0.5;
    }
    for i in 0..n {
        if delta[i].abs() < EPSILON {
            m[i] = 0.0;
            m[i + 1] = 0.0;
        } else {
            let alpha = m[i] / delta[i];
            let beta = m[i + 1] / delta[i];
            let tau = alpha * alpha + beta * beta;
            if tau > 9.0 {
                m[i] = 3.0 * alpha * delta[i] / tau.sqrt();
                m[i + 1] = 3.0 * beta * delta[i] / tau.sqrt();
            }
        }
    }
    m
}

/// darktable `spline_cubic_val`: the cubic spline at `v`, from its second derivatives.
#[cfg(test)]
fn spline_cubic_val(t: &[f32], v: f32, y: &[f32], ypp: &[f32]) -> f32 {
    let n = t.len();
    let i = (0..n - 1).find(|&i| v < t[i + 1]).unwrap_or(n - 2);
    let dt = v - t[i];
    let h = t[i + 1] - t[i];
    // As darktable, with its double literals.
    let (dt, h) = (f64::from(dt), f64::from(h));
    let (y0, y1) = (f64::from(y[i]), f64::from(y[i + 1]));
    let (p0, p1) = (f64::from(ypp[i]), f64::from(ypp[i + 1]));
    (y0 + dt
        * ((y1 - y0) / h - (p1 / 6.0 + p0 / 3.0) * h
            + dt * (0.5 * p0 + dt * ((p1 - p0) / (6.0 * h))))) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test 0060's red curve, as stored (darktable's compensated coordinates).
    const X: [f32; 9] = [
        0.056114, 0.125585, 0.182916, 0.277205, 0.409264, 0.5911, 0.797724, 0.882749, 1.0,
    ];
    const Y: [f32; 9] = [
        0.0, 0.13339, 0.290952, 0.397451, 0.563564, 0.654994, 0.764868, 0.906857, 1.0,
    ];

    fn knots(interpolation: Interpolation, x: &[f32], y: &[f32]) -> Vec<Knot> {
        let m = tangents(interpolation, x, y).unwrap();
        (0..x.len())
            .map(|i| Knot {
                x: x[i],
                y: y[i],
                tangent: m[i],
            })
            .collect()
    }

    /// Points across the curve's range, nodes included.
    fn samples() -> impl Iterator<Item = f32> {
        (0..=1000).map(|i| X[0] + (X[8] - X[0]) * i as f32 / 1000.0)
    }

    #[test]
    fn every_interpolation_passes_through_its_nodes() {
        for interpolation in [
            Interpolation::CubicSpline,
            Interpolation::CatmullRom,
            Interpolation::MonotoneHermite,
        ] {
            let knots = knots(interpolation, &X, &Y);
            for i in 0..X.len() {
                let y = evaluate(&knots, X[i]);
                assert!((y - Y[i]).abs() < 1e-6, "{interpolation:?} node {i}: {y}");
            }
        }
    }

    #[test]
    fn the_hermite_form_is_darktables_cubic_spline() {
        let ypp = spline_cubic_set(&X, &Y).unwrap();
        let knots = knots(Interpolation::CubicSpline, &X, &Y);
        for v in samples() {
            let darktable = spline_cubic_val(&X, v, &Y, &ypp).clamp(0.0, 1.0);
            let ours = evaluate(&knots, v);
            assert!(
                (ours - darktable).abs() < 2e-6,
                "at {v}: {ours} vs {darktable}"
            );
        }
    }

    #[test]
    fn the_spline_has_one_slope_at_each_node() {
        // Each interior node's slope from the segment to its left, at that segment's
        // right end, equals the one taken from the segment to its right.
        let ypp = spline_cubic_set(&X, &Y).unwrap();
        let m = spline_tangents(&X, &Y, &ypp);
        for i in 1..X.len() - 1 {
            let h = f64::from(X[i] - X[i - 1]);
            let from_left = f64::from(Y[i] - Y[i - 1]) / h
                + h * (f64::from(ypp[i - 1]) / 6.0 + f64::from(ypp[i]) / 3.0);
            assert!((from_left - f64::from(m[i])).abs() < 1e-5, "node {i}");
        }
    }

    #[test]
    fn natural_spline_ends_are_straight() {
        let ypp = spline_cubic_set(&X, &Y).unwrap();
        assert_eq!(ypp[0], 0.0);
        assert_eq!(ypp[X.len() - 1], 0.0);
    }

    #[test]
    fn catmull_rom_tangents_are_darktables() {
        let m = tangents(Interpolation::CatmullRom, &X, &Y).unwrap();
        assert_eq!(m[0], (Y[1] - Y[0]) / (X[1] - X[0]));
        assert_eq!(m[4], (Y[5] - Y[3]) / (X[5] - X[3]));
        assert_eq!(m[8], (Y[8] - Y[7]) / (X[8] - X[7]));
    }

    #[test]
    fn monotone_hermite_is_monotone_and_limits_steep_tangents() {
        // A step: a flat stretch, a jump, a flat stretch. Unlimited, the averaged
        // tangents would overshoot.
        let x = [0.0, 0.3, 0.35, 1.0];
        let y = [0.0, 0.0, 1.0, 1.0];
        let knots = knots(Interpolation::MonotoneHermite, &x, &y);
        let mut last = 0.0;
        for i in 0..=1000 {
            let v = evaluate(&knots, i as f32 / 1000.0);
            assert!(v >= last - 1e-7, "not monotone at {i}");
            last = v;
        }
        // Flat secants force zero tangents at their nodes.
        assert_eq!(knots[0].tangent, 0.0);
        assert_eq!(knots[1].tangent, 0.0);
        assert_eq!(knots[3].tangent, 0.0);
    }

    #[test]
    fn identity_is_exact_at_its_nodes_and_close_between() {
        for interpolation in [
            Interpolation::CubicSpline,
            Interpolation::CatmullRom,
            Interpolation::MonotoneHermite,
        ] {
            let knots = knots(interpolation, &[0.0, 1.0], &[0.0, 1.0]);
            assert_eq!(knots[0].tangent, 1.0);
            assert_eq!(knots[1].tangent, 1.0);
            for i in 0..=100 {
                let v = i as f32 / 100.0;
                assert!(
                    (evaluate(&knots, v) - v).abs() < 1e-6,
                    "{interpolation:?} {v}"
                );
            }
        }
    }

    #[test]
    fn below_the_first_node_the_curve_is_flat() {
        let knots = knots(Interpolation::CubicSpline, &X, &Y);
        assert_eq!(evaluate(&knots, 0.0), Y[0]);
        assert_eq!(evaluate(&knots, -5.0), Y[0]);
    }

    #[test]
    fn darktable_refuses_what_it_cannot_interpolate() {
        let i = Interpolation::MonotoneHermite;
        assert!(tangents(i, &[0.5], &[0.5]).is_none());
        assert!(tangents(i, &[0.0, 0.5, 0.5, 1.0], &[0.0, 0.2, 0.3, 1.0]).is_none());
        assert!(tangents(i, &[0.0, 0.6, 0.5, 1.0], &[0.0, 0.2, 0.3, 1.0]).is_none());
        let x: Vec<f32> = (0..=MAX_NODES).map(|i| i as f32 / 21.0).collect();
        assert!(tangents(i, &x, &x).is_none());
        assert!(tangents(i, &x[..MAX_NODES], &x[..MAX_NODES]).is_some());
    }
}
