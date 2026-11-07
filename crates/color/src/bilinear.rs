// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2019-2024  darktable developers (ported from src/common/fast_guided_filter.h)
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

//! darktable's bilinear resampling between grids (`src/common/fast_guided_filter.h`,
//! `interpolate_bilinear`), restated on the CPU in f32 and darktable's order of
//! arithmetic: the reference the kernels' `slang/bilinear.slang` is tested against.
//! Corner-aligned, with no half-pixel offset, the nodes clamped to the input.
//!
//! Shared by the colour equalizer (`apertrust-op-colorequal`), its second user, and the
//! tone equalizer's EIGF mask, whose own copy in `apertrust-op-toneequal` is to move here
//! (decision 65).

/// Where `interpolate_bilinear()` reads an image `size_in` across and down for the
/// pixel at `at` of one `size_out`: the four nodes and their weights, darktable's names.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Taps {
    pub x_prev: usize,
    pub x_next: usize,
    pub y_prev: usize,
    pub y_next: usize,
    /// `Dx_next`, the weight of `x_prev`, and `Dx_prev`, that of `x_next`.
    pub dx_next: f32,
    pub dx_prev: f32,
    pub dy_next: f32,
    pub dy_prev: f32,
}

impl Taps {
    pub fn new(at: [usize; 2], size_in: [usize; 2], size_out: [usize; 2]) -> Self {
        let x_in = (at[0] as f32 / size_out[0] as f32) * size_in[0] as f32;
        let y_in = (at[1] as f32 / size_out[1] as f32) * size_in[1] as f32;
        let x_prev = x_in.floor() as usize;
        let y_prev = y_in.floor() as usize;
        let x_next = (x_prev + 1).min(size_in[0] - 1);
        let y_next = (y_prev + 1).min(size_in[1] - 1);
        let x_prev = x_prev.min(size_in[0] - 1);
        let y_prev = y_prev.min(size_in[1] - 1);
        let dy_next = y_next as f32 - y_in;
        let dx_next = x_next as f32 - x_in;
        Self {
            x_prev,
            x_next,
            y_prev,
            y_next,
            dx_next,
            dx_prev: 1.0 - dx_next,
            dy_next,
            dy_prev: 1.0 - dy_next,
        }
    }

    /// The interpolation's sum for one channel from the values at the nodes, north
    /// `y_prev`, west `x_prev`.
    pub fn interpolate(&self, nw: f32, ne: f32, sw: f32, se: f32) -> f32 {
        self.dy_prev * (sw * self.dx_next + se * self.dx_prev)
            + self.dy_next * (nw * self.dx_next + ne * self.dx_prev)
    }
}

/// darktable `interpolate_bilinear()` on `ch` interleaved channels, from `size_in` to
/// `size_out`, each width then height.
pub fn interpolate_bilinear(
    input: &[f32],
    size_in: [usize; 2],
    size_out: [usize; 2],
    ch: usize,
) -> Vec<f32> {
    let [width_in, _] = size_in;
    let [width_out, height_out] = size_out;
    assert_eq!(
        input.len(),
        size_in[0] * size_in[1] * ch,
        "a {size_in:?} image"
    );
    let mut out = vec![0.0; width_out * height_out * ch];
    for i in 0..height_out {
        for j in 0..width_out {
            let t = Taps::new([j, i], size_in, size_out);
            let at = |y: usize, x: usize, c: usize| input[(y * width_in + x) * ch + c];
            for c in 0..ch {
                out[(i * width_out + j) * ch + c] = t.interpolate(
                    at(t.y_prev, t.x_prev, c),
                    at(t.y_prev, t.x_next, c),
                    at(t.y_next, t.x_prev, c),
                    at(t.y_next, t.x_next, c),
                );
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_downscale_by_four_reads_every_fourth_pixel() {
        // Corner-aligned: the output's pixel j sits at the input's 4j, so a downscale by
        // a whole factor decimates, up to the weights' rounding.
        let (w, h) = (16, 8);
        let input: Vec<f32> = (0..w * h).map(|i| i as f32).collect();
        let out = interpolate_bilinear(&input, [w, h], [w / 4, h / 4], 1);
        for y in 0..h / 4 {
            for x in 0..w / 4 {
                let expected = input[(4 * y) * w + 4 * x];
                assert!((out[y * (w / 4) + x] - expected).abs() < 1e-3, "{x}, {y}");
            }
        }
    }

    #[test]
    fn an_upscale_interpolates_and_clamps_at_the_far_edge() {
        let input = [0.0, 1.0, 2.0, 3.0];
        let out = interpolate_bilinear(&input, [2, 2], [4, 4], 1);
        // Row 0: 0, 0.5, 1, 1 (the last two clamp to the input's last column).
        assert_eq!(&out[..4], &[0.0, 0.5, 1.0, 1.0]);
        // Row 2: y_in 1, the last row.
        assert_eq!(&out[8..12], &[2.0, 2.5, 3.0, 3.0]);
    }
}
