// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2011-2025  darktable developers (ported from src/iop/flip.c)
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

//! Orientation, ported from darktable (`src/iop/flip.c`, `docs/ports/flip.md`): turns or
//! mirrors the image in steps of 90°, exactly, with no interpolation. Its one parameter is
//! darktable's `orientation`, auto by default, which takes the photo's own orientation
//! from its EXIF data, so every photo opens the right way up.
//!
//! It is the first real operation that moves pixels (decision 16): its [`Geometry`] holds
//! darktable's `modify_roi_out`, `modify_roi_in`, `distort_transform` and
//! `distort_backtransform`, and its kernel copies each output pixel from the one its
//! inverse map names in the input region, which is exactly the output region turned back.
//! At no rotation it runs no pass and moves nothing, so the planner leaves it out, as
//! darktable's `commit_params` disables its piece: a photo the right way up plans as it
//! would without it.

use apertrust_plugin::{
    Domain, Extent, Geometry, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted,
    Operation, Param, ParamValues, Pass, Prepared, Provenance, Schema, Slot, Stage,
};

/// Generated from `slang/flip.slang`. It has no continuous parameter to differentiate.
pub const KERNEL: Kernel = Kernel {
    name: "flip",
    wgsl: include_str!("../generated/flip.wgsl"),
    derivatives: false,
};

pub struct Flip;

static MANIFEST: Manifest = Manifest {
    id: "flip",
    version: 1,
    slot: Slot {
        stage: Stage::CameraRgb,
        order: 16.0,
    },
    domain: Domain::Scene,
    layers: false,
    differentiable: false,
    radius: 0,
    // darktable's `reload_defaults` enables it on every image, at auto.
    default_enabled: true,
    provenance: Some(Provenance {
        darktable_module: "flip",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &[
            "src/iop/flip.c",
            "src/imageio/imageio.c",
            "src/common/image.h",
        ],
    }),
};

/// darktable's flip bits, `dt_image_orientation_t` (`src/common/image.h`): the rows
/// flipped, the columns flipped, then the axes swapped.
pub const FLIP_Y: u8 = 1;
pub const FLIP_X: u8 = 2;
pub const SWAP_XY: u8 = 4;

/// darktable's `dt_image_orientation_t` values with their `$DESCRIPTION`s, in its order
/// from -1: autodetect, no rotation, flip vertically, flip horizontally, rotate 180°,
/// transpose, rotate -90° (clockwise), rotate 90° (anticlockwise) and transverse. The
/// choice at index `i` is darktable's value `i - 1`.
pub const ORIENTATIONS: [&str; 9] = [
    "auto",
    "none",
    "flip_vertically",
    "flip_horizontally",
    "rotate_180",
    "transpose",
    "rotate_cw_90",
    "rotate_ccw_90",
    "transverse",
];

/// The choice of auto, darktable's `ORIENTATION_NULL`.
pub const AUTO: u32 = 0;

/// darktable `dt_iop_flip_params_t`, version 2: the orientation, auto by default
/// (`reload_defaults`).
static SCHEMA: Schema = Schema {
    version: 2,
    params: &[Param {
        name: "orientation",
        kind: Kind::Choice {
            default: AUTO,
            choices: &ORIENTATIONS,
        },
    }],
};

/// The flip bits `params` render on `image`, as darktable's `commit_params` takes them:
/// auto is the photo's own orientation (`dt_image_orientation`), none for an image that
/// records none or is not a raw; any other choice its own bits.
pub fn orientation(params: &ParamValues, image: &ImageContext) -> u8 {
    match params.choice("orientation") {
        AUTO => image
            .camera
            .as_ref()
            .map_or(0, |camera| camera.orientation & 7),
        choice => (choice - 1) as u8,
    }
}

/// The input region's size and the flip bits, `FlipParams` in the kernel.
fn kernel_params(input: &ImageDesc, orientation: u8) -> Vec<u8> {
    [input.width, input.height, u32::from(orientation), 0]
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .collect()
}

/// darktable's region and point maps for one orientation, 1 to 7.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlipGeometry(pub u8);

impl FlipGeometry {
    fn has(self, bit: u8) -> bool {
        self.0 & bit != 0
    }

    /// darktable's `backtransform`: the input pixel an output pixel shows, the input
    /// being `iw` by `ih` (the output's sides swapped back where the axes swap).
    fn back(self, [x, y]: [i64; 2], [iw, ih]: [i64; 2]) -> [i64; 2] {
        let [mut i, mut j] = if self.has(SWAP_XY) { [y, x] } else { [x, y] };
        if self.has(FLIP_X) {
            i = iw - i - 1;
        }
        if self.has(FLIP_Y) {
            j = ih - j - 1;
        }
        [i, j]
    }
}

impl Geometry for FlipGeometry {
    /// darktable's `modify_roi_out`: the sides swapped where the axes swap.
    fn extent(&self, [width, height]: [u32; 2]) -> [u32; 2] {
        if self.has(SWAP_XY) {
            [height, width]
        } else {
            [width, height]
        }
    }

    /// darktable's `modify_roi_in`: the bounding box of the output region's corner
    /// pixels turned back, in the input at the pipe's scale. A bijection of whole pixels,
    /// so it is exactly the output region's pixels and needs no clipping.
    fn input_region(&self, input: &Extent, [x, y, width, height]: [u32; 4]) -> [u32; 4] {
        let size = input.scaled.map(i64::from);
        let (x0, y0) = (i64::from(x), i64::from(y));
        let (x1, y1) = (x0 + i64::from(width) - 1, y0 + i64::from(height) - 1);
        let corners = [[x0, y0], [x1, y0], [x0, y1], [x1, y1]].map(|c| self.back(c, size));
        let low = |axis: usize| corners.iter().map(|c| c[axis]).min().unwrap_or(0);
        let high = |axis: usize| corners.iter().map(|c| c[axis]).max().unwrap_or(0);
        let [left, top] = [low(0), low(1)];
        [
            left as u32,
            top as u32,
            (high(0) - left + 1) as u32,
            (high(1) - top + 1) as u32,
        ]
    }

    /// darktable's `distort_transform`: the flips on the input's sides, then the swap.
    fn forward(&self, [width, height]: [u32; 2], [x, y]: [f32; 2]) -> Option<[f32; 2]> {
        let x = if self.has(FLIP_X) {
            width as f32 - x
        } else {
            x
        };
        let y = if self.has(FLIP_Y) {
            height as f32 - y
        } else {
            y
        };
        Some(if self.has(SWAP_XY) { [y, x] } else { [x, y] })
    }

    /// A quarter turn or a mirror of whole pixels: an export at a size resamples on
    /// either side of it, as darktable's does.
    fn whole_pixels(&self) -> bool {
        true
    }

    /// darktable's `distort_backtransform`: the swap, then the flips on the input's
    /// sides.
    fn backward(&self, [width, height]: [u32; 2], [x, y]: [f32; 2]) -> Option<[f32; 2]> {
        let [x, y] = if self.has(SWAP_XY) { [y, x] } else { [x, y] };
        let x = if self.has(FLIP_X) {
            width as f32 - x
        } else {
            x
        };
        let y = if self.has(FLIP_Y) {
            height as f32 - y
        } else {
            y
        };
        Some([x, y])
    }
}

impl Operation for Flip {
    fn manifest(&self) -> &'static Manifest {
        &MANIFEST
    }

    fn schema(&self) -> &'static Schema {
        &SCHEMA
    }

    fn kernel(&self) -> &'static Kernel {
        &KERNEL
    }

    /// One pass turning its input region into the output region, or none at no
    /// rotation, its output then its input as it is, which the planner leaves out of the
    /// plan, as darktable's `commit_params` disables its piece.
    fn prepare(
        &self,
        params: &ParamValues,
        image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<Prepared, NotPorted> {
        let orientation = orientation(params, image);
        if orientation == 0 {
            return Ok(Prepared {
                output: *input,
                passes: Vec::new(),
                scratch: Vec::new(),
            });
        }
        let [width, height] = FlipGeometry(orientation).extent([input.width, input.height]);
        Ok(Prepared {
            output: ImageDesc {
                width,
                height,
                ..*input
            },
            passes: vec![Pass::per_pixel(
                kernel_params(input, orientation),
                Vec::new(),
            )],
            scratch: Vec::new(),
        })
    }

    /// Its maps for the orientation `params` render, none at no rotation.
    fn geometry(&self, params: &ParamValues, image: &ImageContext) -> Option<Box<dyn Geometry>> {
        match orientation(params, image) {
            0 => None,
            bits => Some(Box::new(FlipGeometry(bits))),
        }
    }
}

#[cfg(test)]
mod tests {
    use apertrust_plugin::{Camera, Pattern, Value};

    use super::*;

    fn photo(orientation: u8) -> ImageContext {
        ImageContext {
            camera: Some(Camera {
                make: String::new(),
                model: String::new(),
                alias: String::new(),
                width: 8,
                height: 6,
                cfa: None,
                black: Pattern {
                    width: 1,
                    height: 1,
                    values: vec![0.0],
                },
                white: 1.0,
                white_balance: None,
                xyz_to_camera: None,
                crop: [0, 0, 8, 6],
                declared: [0, 0, 8, 6],
                exposure_bias: 0.0,
                iso: 0.0,
                linear_response_limit: 1.0,
                lens_correction: None,
                orientation,
            }),
            ..ImageContext::full(8, 6)
        }
    }

    fn params(choice: u32) -> ParamValues {
        let mut values = ParamValues::new();
        values.set("orientation", Value::Choice(choice));
        SCHEMA.resolve(&values).unwrap()
    }

    /// darktable's `dt_imageio_flip_buffers`, restated: where input pixel (i, j) of an
    /// image `wd` by `ht` lands, the output `ht` wide where the axes swap.
    fn flip_buffers(orientation: u8, [wd, ht]: [i64; 2], [i, j]: [i64; 2]) -> [i64; 2] {
        let (mut ii, mut jj) = (0, 0);
        let (mut si, mut sj) = (1, wd);
        if orientation & SWAP_XY != 0 {
            sj = 1;
            si = ht;
        }
        if orientation & FLIP_Y != 0 {
            jj = ht - jj - 1;
            sj = -sj;
        }
        if orientation & FLIP_X != 0 {
            ii = wd - ii - 1;
            si = -si;
        }
        let at = sj.abs() * jj + si.abs() * ii + sj * j + si * i;
        let out_width = if orientation & SWAP_XY != 0 { ht } else { wd };
        [at % out_width, at / out_width]
    }

    #[test]
    fn auto_is_the_photo_s_orientation_and_the_rest_darktable_s_values() {
        for bits in 0..8 {
            assert_eq!(orientation(&params(AUTO), &photo(bits)), bits);
        }
        assert_eq!(orientation(&params(AUTO), &ImageContext::full(8, 6)), 0);
        for choice in 1..9 {
            for bits in [0, 5] {
                assert_eq!(orientation(&params(choice), &photo(bits)), choice as u8 - 1);
            }
        }
        assert_eq!(SCHEMA.defaults().choice("orientation"), AUTO);
    }

    #[test]
    fn no_rotation_runs_no_pass_and_moves_nothing() {
        let input = ImageDesc::rgba(8, 6);
        for (choice, image) in [(AUTO, photo(0)), (1, photo(6))] {
            let p = params(choice);
            let prepared = Flip.prepare(&p, &image, &input).unwrap();
            assert!(prepared.passes.is_empty() && prepared.scratch.is_empty());
            assert_eq!(prepared.output, input);
            assert!(Flip.geometry(&p, &image).is_none());
        }
    }

    #[test]
    fn its_kernel_s_uniforms_and_output_follow_the_orientation() {
        let input = ImageDesc::rgba(8, 6);
        for bits in 1..8u8 {
            let prepared = Flip.prepare(&params(u32::from(bits) + 1), &photo(0), &input);
            let prepared = prepared.unwrap();
            let swapped = bits & SWAP_XY != 0;
            let size = [prepared.output.width, prepared.output.height];
            assert_eq!(size, if swapped { [6, 8] } else { [8, 6] });
            let words: Vec<u32> = prepared.passes[0]
                .params
                .chunks(4)
                .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
                .collect();
            assert_eq!(words, [8, 6, u32::from(bits), 0]);
        }
    }

    /// The region map, darktable's `modify_roi_in`, at every orientation and on every
    /// region of a small image at a scale: exactly the output region's pixels turned back
    /// by darktable's buffer flip.
    #[test]
    fn a_region_reads_exactly_its_pixels_turned_back() {
        let full = [16, 10];
        let scaled = [8, 5];
        for bits in 1..8u8 {
            let geometry = FlipGeometry(bits);
            let input = Extent {
                full,
                scaled,
                scale: 0.5,
            };
            let [ow, oh] = geometry.extent(scaled);
            for (x, y, w, h) in [(0, 0, ow, oh), (1, 2, 3, 2), (ow - 2, oh - 1, 2, 1)] {
                let read = geometry.input_region(&input, [x, y, w, h]);
                let mut lands = Vec::new();
                for j in read[1]..read[1] + read[3] {
                    for i in read[0]..read[0] + read[2] {
                        let s = scaled.map(i64::from);
                        let [ox, oy] = flip_buffers(bits, s, [i64::from(i), i64::from(j)]);
                        lands.push((oy, ox));
                    }
                }
                lands.sort_unstable();
                let wanted: Vec<(i64, i64)> = (y..y + h)
                    .flat_map(|oy| (x..x + w).map(move |ox| (i64::from(oy), i64::from(ox))))
                    .collect();
                assert_eq!(lands, wanted, "orientation {bits}, region {x} {y} {w} {h}");
            }
        }
    }

    /// The point maps, darktable's `distort_transform` and `distort_backtransform`: each
    /// the other's inverse, and a pixel's centre landing on the centre of the pixel the
    /// buffer flip moves it to.
    #[test]
    fn the_point_maps_are_inverses_and_follow_the_buffer_flip() {
        let size = [16u32, 10];
        for bits in 1..8u8 {
            let geometry = FlipGeometry(bits);
            for (i, j) in [(0, 0), (3, 7), (15, 9), (8, 2)] {
                let centre = [i as f32 + 0.5, j as f32 + 0.5];
                let there = geometry.forward(size, centre).unwrap();
                let [ox, oy] = flip_buffers(bits, size.map(i64::from), [i, j]);
                assert_eq!(there, [ox as f32 + 0.5, oy as f32 + 0.5], "{bits}");
                assert_eq!(geometry.backward(size, there).unwrap(), centre, "{bits}");
            }
        }
    }
}
