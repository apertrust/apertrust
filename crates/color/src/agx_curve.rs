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

//! AgX's tone curve (darktable `src/iop/agx.c`: the curve part of
//! `_calculate_tone_mapping_params`, `_apply_log_encoding` and `_apply_curve`): log
//! encoding around mid-grey, then a linear section through the pivot between a toe and
//! a shoulder, each a sigmoid or, where the sigmoid cannot bend that way, a power.
//!
//! The operation's setup (`apertrust-op-agx`) packs its coefficients for the kernel; the
//! AgX tool's Pivot target output picker, which sets the pivot from an area, evaluates it
//! too (decision 65). It runs on the CPU, never on a pixel.
//!
//! The curve is computed in f32 like darktable's, except `_scale`, which works in the
//! log domain so it stays finite.

/// darktable's `_epsilon`.
pub const EPSILON: f32 = 1e-6;
/// darktable's `_default_gamma`: the gamma the contrast is defined at.
pub const DEFAULT_GAMMA: f32 = 2.2;

/// The settings the curve depends on: darktable params version 7's range and curve
/// fields, under their names.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveSettings {
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
}

/// darktable `CLAMPF`: `fminf(fmaxf(x, low), high)`, which never panics.
fn clampf(x: f32, low: f32, high: f32) -> f32 {
    x.max(low).min(high)
}

/// pow that is 0 for a base of 0 or less, like the kernel's `pow0`.
fn pow0(x: f32, y: f32) -> f32 {
    if x <= 0.0 { 0.0 } else { x.powf(y) }
}

/// The toe or the shoulder of the curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub transition_x: f32,
    pub transition_y: f32,
    pub scale: f32,
    pub power: f32,
    /// Whether the sigmoid cannot bend this way and the power fallback is used
    /// (darktable's `need_convex_toe`, `need_concave_shoulder`).
    pub fallback: bool,
    pub fallback_coefficient: f32,
    pub fallback_power: f32,
}

/// The tone curve: log encoding and the curve's coefficients (the curve part of
/// darktable's `tone_mapping_params_t`). A pure function of the settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Curve {
    pub black_ev: f32,
    pub range_ev: f32,
    pub gamma: f32,
    pub pivot_x: f32,
    pub pivot_y: f32,
    pub slope: f32,
    pub intercept: f32,
    pub target_black: f32,
    pub target_white: f32,
    pub toe: Segment,
    pub shoulder: Segment,
}

/// darktable `_scale`: the sigmoid's scale that takes it from the transition at `slope`
/// to the limit. darktable computes `max(ε, a^-p - r^-p)^(-1/p)`, which overflows f32
/// for powers above about 6.4 when the rise `a` is clamped to ε; this is the same in
/// the log domain, finite for every setting.
fn scale(
    limit_x: f32,
    limit_y: f32,
    transition_x: f32,
    transition_y: f32,
    slope: f32,
    power: f32,
) -> f32 {
    let projected_rise = slope * EPSILON.max(limit_x - transition_x);
    let actual_rise = EPSILON.max(limit_y - transition_y);
    // log2(a^-p - r^-p) = -p log2 a + log2(1 - (a / r)^p), for r > a.
    let log_base = if projected_rise > actual_rise {
        let ratio = (-power * (projected_rise.log2() - actual_rise.log2())).exp2();
        -power * actual_rise.log2() + (1.0 - ratio).log2()
    } else {
        f32::NEG_INFINITY
    };
    let log_base = log_base.max(EPSILON.log2());
    (-log_base / power).exp2().min(1e9)
}

/// darktable `_sigmoid` for `t ≥ 0`, without overflowing `t^p`.
fn sigmoid(t: f32, power: f32) -> f32 {
    if t <= 1.0 {
        t * (1.0 + pow0(t, power)).powf(-1.0 / power)
    } else {
        (1.0 + t.powf(-power)).powf(-1.0 / power)
    }
}

impl Curve {
    /// darktable `_calculate_tone_mapping_params`, curve part, with `_adjust_pivot` and
    /// `_calculate_slope_gamma_compensation`.
    pub fn new(s: &CurveSettings) -> Self {
        let black_ev = s.range_black_relative_ev;
        let range_ev = s.range_white_relative_ev - black_ev;

        let pivot_x = clampf(s.curve_pivot_x, EPSILON, 1.0 - EPSILON);
        let pivot_y_linear = s.curve_pivot_y_linear_output;
        // Auto gamma clamped to the gamma slider's range, as darktable master does
        // (3526ecc350); the reference release leaves it unbounded.
        let gamma = if s.auto_gamma && pivot_x > 0.0 && pivot_y_linear > 0.0 {
            clampf(pivot_y_linear.log2() / pivot_x.log2(), 0.01, 100.0)
        } else {
            s.curve_gamma
        };
        let black_ratio = s.curve_target_display_black_ratio;
        let white_ratio = s.curve_target_display_white_ratio;
        let pivot_y_at =
            |gamma: f32| clampf(pivot_y_linear, black_ratio, white_ratio).powf(1.0 / gamma);
        let pivot_y = pivot_y_at(gamma);

        // The slope, normalised to the default range and compensated for gamma so the
        // linearised slope at the pivot is what it is at the default gamma.
        let range_adjusted_slope = s.curve_contrast_around_pivot * (range_ev / 16.5);
        let derivative_at_gamma = gamma * EPSILON.max(pivot_y).powf(gamma - 1.0);
        let derivative_at_default = DEFAULT_GAMMA
            * EPSILON
                .max(pivot_y_at(DEFAULT_GAMMA))
                .powf(DEFAULT_GAMMA - 1.0);
        let slope = range_adjusted_slope / (derivative_at_gamma / derivative_at_default);

        // Toe, from (0, target black) to its transition below the pivot.
        let target_black = black_ratio.powf(1.0 / gamma);
        let toe_power = 0.01f32.max(s.curve_toe_power);
        let toe_length_y = (pivot_y - target_black) * s.curve_linear_ratio_below_pivot;
        let toe_transition_x = EPSILON.max(pivot_x - toe_length_y / slope);
        let toe_transition_y = pivot_y - slope * (pivot_x - toe_transition_x);
        // The shoulder's formula, on the toe flipped left to right and up to down.
        let toe_scale = -scale(
            1.0,
            1.0 - target_black,
            1.0 - toe_transition_x,
            1.0 - toe_transition_y,
            slope,
            toe_power,
        );
        let toe_dx = toe_transition_x;
        let toe_dy = EPSILON.max(toe_transition_y - target_black);
        let toe_fallback_power = slope * toe_dx / toe_dy;
        let toe = Segment {
            transition_x: toe_transition_x,
            transition_y: toe_transition_y,
            scale: toe_scale,
            power: toe_power,
            fallback: toe_dy / toe_dx > slope,
            fallback_coefficient: toe_dy / toe_dx.powf(toe_fallback_power),
            fallback_power: toe_fallback_power,
        };
        let intercept = toe_transition_y - slope * toe_transition_x;

        // Shoulder, from its transition above the pivot to (1, target white).
        let target_white = white_ratio.powf(1.0 / gamma);
        let shoulder_length_y = (target_white - pivot_y) * s.curve_linear_ratio_above_pivot;
        let shoulder_transition_x = (1.0 - EPSILON).min(pivot_x + shoulder_length_y / slope);
        let shoulder_transition_y = pivot_y + slope * (shoulder_transition_x - pivot_x);
        let shoulder_power = 0.01f32.max(s.curve_shoulder_power);
        let shoulder_scale = scale(
            1.0,
            target_white,
            shoulder_transition_x,
            shoulder_transition_y,
            slope,
            shoulder_power,
        );
        let shoulder_dx = 1.0 - shoulder_transition_x;
        let shoulder_dy = EPSILON.max(target_white - shoulder_transition_y);
        let shoulder_fallback_power = slope * shoulder_dx / shoulder_dy;
        let shoulder = Segment {
            transition_x: shoulder_transition_x,
            transition_y: shoulder_transition_y,
            scale: shoulder_scale,
            power: shoulder_power,
            fallback: shoulder_dy / shoulder_dx > slope,
            fallback_coefficient: shoulder_dy / shoulder_dx.powf(shoulder_fallback_power),
            fallback_power: shoulder_fallback_power,
        };

        Self {
            black_ev,
            range_ev,
            gamma,
            pivot_x,
            pivot_y,
            slope,
            intercept,
            target_black,
            target_white,
            toe,
            shoulder,
        }
    }

    /// darktable `_apply_log_encoding`: scene-linear `x` to the curve's input in [0, 1].
    pub fn encode(&self, x: f32) -> f32 {
        let relative = EPSILON.max(x / 0.18);
        clampf((relative.log2() - self.black_ev) / self.range_ev, 0.0, 1.0)
    }

    /// darktable `_apply_curve`, as the kernel evaluates it, for `x` in [0, 1]. The
    /// output is display-encoded: linear output is its power `gamma`.
    pub fn eval(&self, x: f32) -> f32 {
        let (toe, shoulder) = (&self.toe, &self.shoulder);
        let y = if x < toe.transition_x {
            if toe.fallback {
                self.target_black
                    + 0.0f32.max(toe.fallback_coefficient * pow0(x, toe.fallback_power))
            } else {
                toe.scale * sigmoid(self.slope * (x - toe.transition_x) / toe.scale, toe.power)
                    + toe.transition_y
            }
        } else if x <= shoulder.transition_x {
            self.slope * x + self.intercept
        } else if shoulder.fallback {
            self.target_white
                - 0.0f32.max(shoulder.fallback_coefficient * pow0(1.0 - x, shoulder.fallback_power))
        } else {
            shoulder.scale
                * sigmoid(
                    self.slope * (x - shoulder.transition_x) / shoulder.scale,
                    shoulder.power,
                )
                + shoulder.transition_y
        };
        clampf(y, self.target_black, self.target_white)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// darktable `_set_scene_referred_default_params`, the curve's fields.
    fn defaults() -> CurveSettings {
        let black = -10.0f32;
        let white = 6.5f32;
        CurveSettings {
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
        }
    }

    /// Range -2 to +2.5 EV with the pivot at mid-grey: too little contrast for an S, so
    /// both fallbacks are needed.
    fn narrow() -> CurveSettings {
        let (black, white) = (-2.0, 2.5);
        CurveSettings {
            range_black_relative_ev: black,
            range_white_relative_ev: white,
            curve_pivot_x: -black / (white - black),
            ..defaults()
        }
    }

    #[test]
    fn default_curve_matches_darktable() {
        let c = Curve::new(&defaults());
        assert!((c.pivot_x - 10.0 / 16.5).abs() < 1e-6);
        assert!((c.pivot_y - 0.18f32.powf(1.0 / 2.2)).abs() < 1e-6);
        assert!((c.slope - 3.0).abs() < 1e-5, "{c:?}");
        assert!(!c.toe.fallback && !c.shoulder.fallback);
    }

    #[test]
    fn narrow_range_needs_both_fallbacks() {
        let c = Curve::new(&narrow());
        assert!(c.toe.fallback && c.shoulder.fallback, "{c:?}");
    }

    #[test]
    fn curve_runs_from_black_through_the_pivot_to_white() {
        let mut steep = defaults();
        steep.curve_linear_ratio_below_pivot = 0.2;
        steep.curve_linear_ratio_above_pivot = 0.3;
        steep.curve_target_display_black_ratio = 0.01;
        steep.curve_target_display_white_ratio = 0.9;
        for settings in [defaults(), narrow(), steep] {
            let c = Curve::new(&settings);
            assert!((c.eval(0.0) - c.target_black).abs() < 1e-4, "{c:?}");
            assert!((c.eval(1.0) - c.target_white).abs() < 1e-4, "{c:?}");
            assert!((c.eval(c.pivot_x) - c.pivot_y).abs() < 1e-5, "{c:?}");
            let mut previous = c.eval(0.0);
            for k in 1..=1000 {
                let y = c.eval(k as f32 / 1000.0);
                assert!(y >= previous - 1e-6, "not monotonic at {k}: {c:?}");
                previous = y;
            }
            // The pieces join with equal slope.
            for x in [c.toe.transition_x, c.shoulder.transition_x] {
                let h = 1e-3;
                let left = (c.eval(x) - c.eval(x - h)) / h;
                let right = (c.eval(x + h) - c.eval(x)) / h;
                assert!((left - right).abs() < 0.05 * c.slope, "kink at {x}: {c:?}");
            }
        }
    }

    #[test]
    fn mid_grey_encodes_to_the_pivot() {
        let c = Curve::new(&defaults());
        assert!((c.encode(0.18) - c.pivot_x).abs() < 1e-6);
        assert!((c.eval(c.encode(0.18)).powf(c.gamma) - 0.18).abs() < 1e-5);
    }

    #[test]
    fn scale_stays_finite_where_darktable_overflows() {
        // Pivot target output at target white and shoulder power 10: darktable's
        // f32 `_scale` overflows to infinity, and the curve to NaN.
        let settings = CurveSettings {
            curve_pivot_y_linear_output: 1.0,
            curve_shoulder_power: 10.0,
            curve_toe_power: 10.0,
            ..defaults()
        };
        let c = Curve::new(&settings);
        for k in 0..=100 {
            assert!(c.eval(k as f32 / 100.0).is_finite(), "{c:?}");
        }
    }

    #[test]
    fn log_domain_scale_matches_darktables_where_finite() {
        let darktable = |lx: f32, ly: f32, tx: f32, ty: f32, m: f32, p: f32| {
            let projected = m * EPSILON.max(lx - tx);
            let actual = EPSILON.max(ly - ty);
            let base = EPSILON.max(actual.powf(-p) - projected.powf(-p));
            base.powf(-1.0 / p).min(1e9)
        };
        for (lx, ly, tx, ty, m, p) in [
            (1.0, 1.0, 0.606, 0.4587, 3.0, 3.3),
            (1.0, 1.0, 0.394, 0.5413, 3.0, 1.5),
            (1.0, 0.977, 0.663, 0.545, 2.686, 2.05),
            (1.0, 1.0, 0.9, 0.95, 0.4, 2.0),
        ] {
            let ours = scale(lx, ly, tx, ty, m, p);
            let theirs = darktable(lx, ly, tx, ty, m, p);
            assert!((ours - theirs).abs() <= 1e-5 * theirs, "{ours} != {theirs}");
        }
    }
}
