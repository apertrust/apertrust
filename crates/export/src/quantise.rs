// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/imageio/imageio.c)
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

//! An export's float output quantised as darktable's `dt_imageio_export_with_flags`
//! quantises it (`src/imageio/imageio.c`): each channel scaled to the format's range,
//! clamped to it and rounded, halves away from zero, `roundf(CLAMP(v × 255, 0, 255))` for
//! 8 bits and `roundf(CLAMP(v × 65535, 0, 65535))` for 16. The gamma module's
//! `fminf(roundf(255 × fmaxf(v, 0)), 255)`, darktable's 8-bit rounding without high
//! quality processing, gives the same bytes.

/// One channel at 8 bits.
pub fn eight_bit(v: f32) -> u8 {
    (v * 255.0).clamp(0.0, 255.0).round() as u8
}

/// One channel at 16 bits.
pub fn sixteen_bit(v: f32) -> u16 {
    (v * 65535.0).clamp(0.0, 65535.0).round() as u16
}

/// RGB at 8 bits, three bytes a pixel, alpha dropped.
pub fn rgb8(pixels: &[[f32; 4]]) -> Vec<u8> {
    pixels
        .iter()
        .flat_map(|p| [eight_bit(p[0]), eight_bit(p[1]), eight_bit(p[2])])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_halves_away_from_zero_and_clamps() {
        // Ends, a half (127.5 and 32767.5), values either side of a rounding step, and
        // values outside 0 to 1.
        let at_8 = [0.0, 0.5, 1.0, -0.2, 1.7, 10.49 / 255.0, 10.51 / 255.0];
        assert_eq!(at_8.map(eight_bit), [0, 128, 255, 0, 255, 10, 11]);
        let at_16 = [0.0, 0.5, 1.0, -0.2, 1.7, 0.25, 0.75];
        assert_eq!(
            at_16.map(sixteen_bit),
            [0, 32768, 65535, 0, 65535, 16384, 49151]
        );
    }
}
