// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/common/colorspaces.h)
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

//! darktable's HSL (`src/common/colorspaces.h`: `rgb2hsl`, `hue2rgb`, `hsl2rgb`) restated
//! on the CPU with darktable's own arithmetic: the reference the kernel's
//! `slang/common/hsl.slang`, in f32, is tested against. Nothing renders through it.
//! darktable writes several steps with double constants; one operation on f32 values
//! computed in double and rounded to f32 is the f32 operation, so only the two steps of
//! two operations each stay in double here (`docs/ports/soften.md`, Precision).

/// darktable's floor on the saturation's denominators, 2^-16.
const SATURATION_FLOOR: f32 = 1.525_878_9e-5;

/// darktable `rgb2hsl`: hue in [0, 1], saturation and lightness.
pub fn rgb_to_hsl([r, g, b]: [f32; 3]) -> [f32; 3] {
    let pmax = r.max(g).max(b);
    let pmin = r.min(g).min(b);
    let delta = pmax - pmin;
    let mut h = 0.0_f32;
    let mut s = 0.0_f32;
    let l = (pmin + pmax) / 2.0;
    if delta != 0.0 {
        s = if l < 0.5 {
            delta / (pmax + pmin).max(SATURATION_FLOOR)
        } else {
            // `2.0 - pmax - pmin`, two steps in double, rounded once for `fmaxf`.
            delta / ((2.0 - f64::from(pmax) - f64::from(pmin)) as f32).max(SATURATION_FLOOR)
        };
        if pmax == r {
            h = (g - b) / delta;
        } else if pmax == g {
            h = 2.0 + (b - r) / delta;
        } else if pmax == b {
            h = 4.0 + (r - g) / delta;
        }
        h /= 6.0;
        if h < 0.0 {
            h += 1.0;
        } else if h > 1.0 {
            h -= 1.0;
        }
    }
    [h, s, l]
}

/// darktable `hue2rgb`: one channel from the hue, already scaled to [0, 6).
fn hue_to_rgb(m1: f32, m2: f32, hue: f32) -> f32 {
    if hue < 1.0 {
        m1 + (m2 - m1) * hue
    } else if hue < 3.0 {
        m2
    } else if hue < 4.0 {
        m1 + (m2 - m1) * (4.0 - hue)
    } else {
        m1
    }
}

/// darktable `hsl2rgb`, from hue in [0, 1], saturation and lightness.
pub fn hsl_to_rgb([h, s, l]: [f32; 3]) -> [f32; 3] {
    if s == 0.0 {
        return [l; 3];
    }
    // `l * (1.0 + s)`, two steps in double, rounded once.
    let m2 = if l < 0.5 {
        (f64::from(l) * (1.0 + f64::from(s))) as f32
    } else {
        l + s - l * s
    };
    let m1 = 2.0 * l - m2;
    let h = h * 6.0;
    [
        hue_to_rgb(m1, m2, if h < 4.0 { h + 2.0 } else { h - 4.0 }),
        hue_to_rgb(m1, m2, h),
        hue_to_rgb(m1, m2, if h > 2.0 { h - 2.0 } else { h + 4.0 }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primaries_and_greys_have_their_hsl() {
        assert_eq!(rgb_to_hsl([1.0, 0.0, 0.0]), [0.0, 1.0, 0.5]);
        assert_eq!(rgb_to_hsl([0.0, 1.0, 0.0]), [1.0 / 3.0, 1.0, 0.5]);
        assert_eq!(rgb_to_hsl([0.0, 0.0, 1.0]), [2.0 / 3.0, 1.0, 0.5]);
        assert_eq!(rgb_to_hsl([0.25, 0.25, 0.25]), [0.0, 0.0, 0.25]);
        // Magenta's hue is negative before it wraps.
        let [h, s, l] = rgb_to_hsl([1.0, 0.0, 0.5]);
        assert!((h - 11.0 / 12.0).abs() < 1e-6, "{h}");
        assert_eq!((s, l), (1.0, 0.5));
    }

    #[test]
    fn hsl_round_trips_within_the_unit_cube() {
        for r in 0..=8 {
            for g in 0..=8 {
                for b in 0..=8 {
                    let rgb = [r, g, b].map(|v| v as f32 / 8.0);
                    let back = hsl_to_rgb(rgb_to_hsl(rgb));
                    for c in 0..3 {
                        assert!((back[c] - rgb[c]).abs() < 1e-6, "{rgb:?}: {back:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_saturation_of_zero_is_grey() {
        assert_eq!(hsl_to_rgb([0.3, 0.0, 0.7]), [0.7; 3]);
    }

    #[test]
    fn the_saturations_floor_bounds_it_for_dark_and_bright_colours() {
        // Below black, pmax + pmin is under the floor; above white, 2 − pmax − pmin.
        let [_, dark, _] = rgb_to_hsl([0.0, -0.5, -0.5]);
        assert_eq!(dark, 0.5 / SATURATION_FLOOR);
        let [_, bright, _] = rgb_to_hsl([3.0, 2.0, 2.0]);
        assert_eq!(bright, 1.0 / SATURATION_FLOOR);
    }
}
