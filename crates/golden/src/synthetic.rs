// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
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

//! Synthetic test images, for checks that need no darktable render.
//!
//! The sweep covers every hue, from neutral to beyond the working gamut, at levels from
//! deep shadow to well above white, so a check sees colours a test photo may lack.

use crate::pfm::Image;

/// Hues across the sweep, one per degree.
pub const HUES: usize = 360;
/// Saturations down each level's band: 0 is neutral, 1 a pure hue with one channel at
/// zero, and above 1 out of the working gamut, with a negative channel.
pub const SATURATIONS: [f32; 23] = [
    0.0, 0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.45, 0.5, 0.55, 0.6, 0.65, 0.7, 0.75, 0.8,
    0.85, 0.9, 0.95, 1.0, 1.1, 1.25,
];
/// Levels of the brightest channel, one band each, top to bottom.
pub const LEVELS: [f32; 8] = [0.001, 0.01, 0.05, 0.18, 0.5, 1.0, 2.0, 4.0];

/// The sweep in linear working RGB: hue across, saturation down within each level's
/// band. Pixel (x, y) has hue x degrees, saturation `SATURATIONS[y % 23]` and level
/// `LEVELS[y / 23]`.
pub fn sweep() -> Image {
    let mut pixels = Vec::with_capacity(HUES * SATURATIONS.len() * LEVELS.len());
    for level in LEVELS {
        for saturation in SATURATIONS {
            for hue in 0..HUES {
                let pure = pure_hue(hue as f32);
                pixels.push(pure.map(|c| level * (1.0 + saturation * (c - 1.0))));
            }
        }
    }
    Image::new(HUES, SATURATIONS.len() * LEVELS.len(), pixels)
        .expect("the sweep's size matches its pixels")
}

/// Where pixel `index` of the sweep sits, for reports.
pub fn describe(index: usize) -> String {
    let (x, y) = (index % HUES, index / HUES);
    format!(
        "hue {x}°, saturation {}, level {}",
        SATURATIONS[y % SATURATIONS.len()],
        LEVELS[y / SATURATIONS.len()]
    )
}

/// The HSV colour of this hue at full saturation and value: the brightest channel at 1,
/// the dimmest at 0.
fn pure_hue(degrees: f32) -> [f32; 3] {
    let h = degrees / 60.0;
    let channel = |n: f32| {
        let k = (n + h) % 6.0;
        1.0 - k.min(4.0 - k).clamp(0.0, 1.0)
    };
    [channel(5.0), channel(3.0), channel(1.0)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primaries_and_secondaries() {
        assert_eq!(pure_hue(0.0), [1.0, 0.0, 0.0]);
        assert_eq!(pure_hue(60.0), [1.0, 1.0, 0.0]);
        assert_eq!(pure_hue(120.0), [0.0, 1.0, 0.0]);
        assert_eq!(pure_hue(240.0), [0.0, 0.0, 1.0]);
        assert_eq!(pure_hue(30.0), [1.0, 0.5, 0.0]);
    }

    #[test]
    fn sweep_layout() {
        let image = sweep();
        assert_eq!(image.width, HUES);
        assert_eq!(image.height, SATURATIONS.len() * LEVELS.len());
        // Neutral at saturation 0, whatever the hue.
        assert_eq!(image.pixels[123], [0.001; 3]);
        // Level 0.18, saturation 1, hue 120°: pure green.
        let index = (3 * SATURATIONS.len() + 20) * HUES + 120;
        assert_eq!(image.pixels[index], [0.0, 0.18, 0.0]);
        assert_eq!(describe(index), "hue 120°, saturation 1, level 0.18");
        // Saturation above 1 leaves the gamut.
        let last = image.pixels.len() - HUES;
        assert!(image.pixels[last][1] < 0.0);
    }
}
