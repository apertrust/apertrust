// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2022-2026  darktable developers (ported from src/iop/colorequal.c)
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

//! darktable's color equalizer `process()` restated on the CPU, loop by loop, in f32 and
//! darktable's order of arithmetic, its buffers blurred in place where darktable's are:
//! the reference the kernel's passes are tested against (`crates/golden/tests/
//! colorequal.rs`), with the colour library's Gaussian and bilinear resampling.
//! Nothing renders through it.

#![allow(
    clippy::excessive_precision,
    reason = "constants are copied digit for digit from darktable"
)]

use apertrust_color::bilinear::interpolate_bilinear;
use apertrust_color::gaussian::gaussian_mean_blur;
use apertrust_color::{WorkingProfile, apply, ucs};

use crate::setup::{
    BRIGHT_EFFECT, CHROMA_FEATHERING, Grid, PARAM_FEATHERING, SAT_EFFECT, Settings, Setup,
    satweight,
};

const NORM_MIN: f32 = 1.525_878_9e-5;

/// darktable `lookup_gamut`.
fn lookup(table: &[f32], hue: f32) -> f32 {
    use std::f32::consts::PI;
    let size = table.len();
    let x = size as f32 * (hue + PI) / (2.0 * PI);
    let x_prev = x.floor();
    let x_next = x.ceil();
    let i = (x_prev as i32) as usize & (size - 1);
    let j = (x_next as i32) as usize & (size - 1);
    let y = table[i];
    y + if i != j {
        (x - x_prev) * (table[j] - y)
    } else {
        0.0
    }
}

/// darktable `soft_clip`.
fn soft_clip(x: f32, soft: f32, hard: f32) -> f32 {
    let norm = hard - soft;
    if x > soft {
        soft + (1.0 - (-(x - soft) / norm).exp()) * norm
    } else {
        x
    }
}

/// darktable `gamut_map_HSB`, in place.
fn gamut_map(hsb: &mut [f32; 3], gamut: &[f32], white: f32) {
    let jch = ucs::hsb_to_jch(*hsb);
    let max_colorfulness = lookup(gamut, jch[2]);
    let max_chroma = 15.932993652962535
        * (jch[0] * white).powf(0.6523997524738018)
        * max_colorfulness.powf(0.6007557017508491)
        / white;
    let boundary = ucs::jch_to_hsb([jch[0], max_chroma, jch[2]]);
    hsb[1] = soft_clip(hsb[1], 0.8 * boundary[1], boundary[1]);
}

/// darktable `scharr_gradient`, at (row, col) of `p`, `w` across.
fn scharr(p: &[f32], k: usize, w: usize) -> f32 {
    let at = |offset: isize| p[(k as isize + offset) as usize];
    let w = w as isize;
    let gx = 47.0 / 255.0 * (at(-w - 1) - at(-w + 1) + at(w - 1) - at(w + 1))
        + 162.0 / 255.0 * (at(-1) - at(1));
    let gy = 47.0 / 255.0 * (at(-w - 1) - at(w - 1) + at(-w + 1) - at(w + 1))
        + 162.0 / 255.0 * (at(-w) - at(w));
    (gx * gx + gy * gy).sqrt()
}

/// `_init_covariance`: the products of the 2-channel guide, 4 channels.
fn init_covariance(uv: &[f32]) -> Vec<f32> {
    uv.as_chunks::<2>()
        .0
        .iter()
        .flat_map(|v| [v[0] * v[0], v[0] * v[1], v[0] * v[1], v[1] * v[1]])
        .collect()
}

/// `_finish_covariance`, in place.
fn finish_covariance(uv: &[f32], covariance: &mut [f32]) {
    for (v, c) in uv
        .as_chunks::<2>()
        .0
        .iter()
        .zip(covariance.as_chunks_mut::<4>().0)
    {
        c[0] -= v[0] * v[0];
        c[1] -= v[0] * v[1];
        c[2] -= v[0] * v[1];
        c[3] -= v[1] * v[1];
    }
}

/// The regularised 2×2 covariance's inverse, or `None` for a singular one.
fn inverse(c: &[f32], eps: f32) -> Option<[f32; 4]> {
    let sigma = [c[0] + eps, c[1], c[2], c[3] + eps];
    let det = sigma[0] * sigma[3] - sigma[1] * sigma[2];
    (det.abs() > 4.0 * f32::EPSILON).then(|| {
        [
            sigma[3] / det,
            -sigma[1] / det,
            -sigma[2] / det,
            sigma[0] / det,
        ]
    })
}

/// darktable `_prefilter_chromaticity`, `uv` in place. `in_place`, darktable's where its
/// grid is not resized, blurs the image's own UV, so the mix starts from its blur;
/// otherwise the blur is a copy's, on the image itself where the grid is not resized
/// (a smaller view whose export downsamples, `Grid::scaled`).
fn prefilter(
    uv: &mut [f32],
    saturation: &[f32],
    size: [usize; 2],
    sigma: f32,
    sat_shift: f32,
    weights: &[f32],
    in_place: bool,
) {
    let grid = Grid::new(sigma, size.map(|s| s as u32));
    assert!(
        !(in_place && grid.resized),
        "only a grid that is the image blurs in place"
    );
    let ds = grid.size.map(|s| s as usize);
    let mut ds_uv = if grid.resized {
        interpolate_bilinear(uv, size, ds, 2)
    } else {
        uv.to_vec()
    };
    let mut covariance = init_covariance(&ds_uv);
    gaussian_mean_blur(&mut ds_uv, ds[0], ds[1], 2, grid.sigma);
    gaussian_mean_blur(&mut covariance, ds[0], ds[1], 4, grid.sigma);
    finish_covariance(&ds_uv, &mut covariance);
    let mut a = vec![0.0f32; 4 * ds[0] * ds[1]];
    let mut b = vec![0.0f32; 2 * ds[0] * ds[1]];
    for k in 0..ds[0] * ds[1] {
        let c = &covariance[4 * k..4 * k + 4];
        if let Some(inv) = inverse(c, CHROMA_FEATHERING) {
            a[4 * k] = c[0] * inv[0] + c[1] * inv[1];
            a[4 * k + 1] = c[0] * inv[2] + c[1] * inv[3];
            a[4 * k + 2] = c[2] * inv[0] + c[3] * inv[1];
            a[4 * k + 3] = c[2] * inv[2] + c[3] * inv[3];
        }
        let (u, v) = (ds_uv[2 * k], ds_uv[2 * k + 1]);
        b[2 * k] = u - a[4 * k] * u - a[4 * k + 1] * v;
        b[2 * k + 1] = v - a[4 * k + 2] * u - a[4 * k + 3] * v;
    }
    if in_place {
        // darktable blurred the image's own UV in place.
        uv.copy_from_slice(&ds_uv);
    }
    gaussian_mean_blur(&mut a, ds[0], ds[1], 4, grid.sigma);
    gaussian_mean_blur(&mut b, ds[0], ds[1], 2, grid.sigma);
    if grid.resized {
        a = interpolate_bilinear(&a, ds, size, 4);
        b = interpolate_bilinear(&b, ds, size, 2);
    }
    for k in 0..size[0] * size[1] {
        let (u, v) = (uv[2 * k], uv[2 * k + 1]);
        let cv = [
            a[4 * k] * u + a[4 * k + 1] * v + b[2 * k],
            a[4 * k + 2] * u + a[4 * k + 3] * v + b[2 * k + 1],
        ];
        let w = satweight(weights, saturation[k] - sat_shift);
        uv[2 * k] = w * (cv[0] - u) + u;
        uv[2 * k + 1] = w * (cv[1] - v) + v;
    }
}

/// darktable `_guide_with_chromaticity`, `uv`, `corrections` and `b_corrections` in place.
/// `in_place`, darktable's where its grid is not resized, blurs the image's own UV and
/// corrections, so the hue correction ends blurred and the guide applies to the blurred
/// UV; otherwise the blurs are copies', as `prefilter`'s.
#[allow(clippy::too_many_arguments, reason = "darktable's signature")]
fn guide(
    uv: &mut [f32],
    corrections: &mut [f32],
    saturation: &[f32],
    b_corrections: &mut [f32],
    gradients: &[f32],
    size: [usize; 2],
    sigma: f32,
    bright_shift: f32,
    sat_shift: f32,
    weights: &[f32],
    in_place: bool,
) {
    let grid = Grid::new(sigma, size.map(|s| s as u32));
    assert!(
        !(in_place && grid.resized),
        "only a grid that is the image blurs in place"
    );
    let ds = grid.size.map(|s| s as usize);
    let cells = ds[0] * ds[1];
    let (mut ds_uv, mut ds_corrections, mut ds_b) = if grid.resized {
        (
            interpolate_bilinear(uv, size, ds, 2),
            interpolate_bilinear(corrections, size, ds, 2),
            interpolate_bilinear(b_corrections, size, ds, 1),
        )
    } else {
        (uv.to_vec(), corrections.to_vec(), b_corrections.to_vec())
    };
    let mut covariance = init_covariance(&ds_uv);
    let mut correlations: Vec<f32> = (0..cells)
        .flat_map(|k| {
            let (u, v) = (ds_uv[2 * k], ds_uv[2 * k + 1]);
            let (c, b) = (ds_corrections[2 * k + 1], ds_b[k]);
            [u * c, v * c, u * b, v * b]
        })
        .collect();
    gaussian_mean_blur(&mut ds_uv, ds[0], ds[1], 2, grid.sigma);
    gaussian_mean_blur(&mut covariance, ds[0], ds[1], 4, grid.sigma);
    gaussian_mean_blur(&mut ds_corrections, ds[0], ds[1], 2, grid.sigma);
    gaussian_mean_blur(&mut ds_b, ds[0], ds[1], 1, 0.1 * grid.sigma);
    gaussian_mean_blur(&mut correlations, ds[0], ds[1], 4, grid.sigma);
    finish_covariance(&ds_uv, &mut covariance);
    for k in 0..cells {
        let (u, v) = (ds_uv[2 * k], ds_uv[2 * k + 1]);
        let (c, b) = (ds_corrections[2 * k + 1], ds_b[k]);
        correlations[4 * k] -= u * c;
        correlations[4 * k + 1] -= v * c;
        correlations[4 * k + 2] -= u * b;
        correlations[4 * k + 3] -= v * b;
    }
    let mut a = vec![0.0f32; 4 * cells];
    let mut b = vec![0.0f32; 2 * cells];
    for k in 0..cells {
        let c = &correlations[4 * k..4 * k + 4];
        if let Some(inv) = inverse(&covariance[4 * k..4 * k + 4], PARAM_FEATHERING) {
            a[4 * k] = c[0] * inv[0] + c[1] * inv[1];
            a[4 * k + 1] = c[0] * inv[2] + c[1] * inv[3];
            a[4 * k + 2] = c[2] * inv[0] + c[3] * inv[1];
            a[4 * k + 3] = c[2] * inv[2] + c[3] * inv[3];
        }
        let (u, v) = (ds_uv[2 * k], ds_uv[2 * k + 1]);
        b[2 * k] = ds_corrections[2 * k + 1] - a[4 * k] * u - a[4 * k + 1] * v;
        b[2 * k + 1] = ds_b[k] - a[4 * k + 2] * u - a[4 * k + 3] * v;
    }
    if in_place {
        // darktable blurred the image's own buffers in place.
        uv.copy_from_slice(&ds_uv);
        corrections.copy_from_slice(&ds_corrections);
        b_corrections.copy_from_slice(&ds_b);
    }
    gaussian_mean_blur(&mut a, ds[0], ds[1], 4, grid.sigma);
    gaussian_mean_blur(&mut b, ds[0], ds[1], 2, grid.sigma);
    if grid.resized {
        a = interpolate_bilinear(&a, ds, size, 4);
        b = interpolate_bilinear(&b, ds, size, 2);
    }
    for k in 0..size[0] * size[1] {
        let (u, v) = (uv[2 * k], uv[2 * k + 1]);
        let cv = [
            a[4 * k] * u + a[4 * k + 1] * v + b[2 * k],
            a[4 * k + 2] * u + a[4 * k + 3] * v + b[2 * k + 1],
        ];
        corrections[2 * k + 1] =
            1.0 + (cv[0] - 1.0) * satweight(weights, saturation[k] - sat_shift);
        let gradient_weight = 1.0 - gradients[k].clamp(0.0, 1.0);
        b_corrections[k] =
            cv[1] * gradient_weight * satweight(weights, saturation[k] - bright_shift);
    }
}

/// What darktable's `process()` applies at each pixel in its last step: the
/// chromaticity, prefiltered with the guided filter, and the corrections of hue,
/// saturation and brightness, guided with it.
#[derive(Debug, Clone, PartialEq)]
pub struct Guided {
    pub uv: Vec<[f32; 2]>,
    pub corrections: Vec<[f32; 3]>,
}

/// Where the guided filters blur the image's own buffers in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InPlace {
    /// darktable's at the pipe's scale: where its grid is not resized.
    Darktable,
    /// The export's, as the operation does at every scale (`Grid::scaled`): where the
    /// grid at full resolution is not resized. At scale 1, darktable's.
    Export,
}

/// A pixel's lightness L*, as `process()` STEP 1 computes it.
pub fn l_star(setup: &Setup, rgb: [f32; 3]) -> f32 {
    ucs::y_to_l_star(ucs::d65_xyz_to_xyy(apply(&setup.rgb_to_xyz_d65, rgb))[2])
}

/// darktable `process()`'s first four steps on a `width` × `height` RGBA image at
/// `scale` pixels per pixel at full resolution, in the working space `profile`, the
/// filters blurring in place as `in_place` says.
pub fn guided(
    settings: &Settings,
    width: usize,
    height: usize,
    pixels: &[[f32; 4]],
    scale: f32,
    profile: &WorkingProfile,
    in_place: InPlace,
) -> Guided {
    let size = [width, height];
    let setup = Setup::new(settings, [width as u32, height as u32], scale, profile);
    let p = setup.process;
    let n = width * height;
    let in_place_of = |grid: &Grid| match in_place {
        InPlace::Darktable => !grid.resized,
        InPlace::Export => grid.in_place,
    };

    // STEP 1
    let mut uv = vec![0.0f32; 2 * n];
    let mut l_scharr = vec![0.0f32; n];
    let mut saturation = vec![0.0f32; n];
    for (k, pixel) in pixels.iter().enumerate() {
        let rgb = [pixel[0], pixel[1], pixel[2]];
        let xyy = ucs::d65_xyz_to_xyy(apply(&setup.rgb_to_xyz_d65, rgb));
        let dmin = rgb[0].min(rgb[1]).min(rgb[2]);
        let dmax = rgb[0].max(rgb[1]).max(rgb[2]);
        let delta = dmax - dmin;
        saturation[k] = if dmax > NORM_MIN && delta > NORM_MIN {
            delta / dmax
        } else {
            0.0
        };
        let [u, v] = ucs::xy_to_uv_star_prime([xyy[0], xyy[1]]);
        uv[2 * k] = u;
        uv[2 * k + 1] = v;
        l_scharr[k] = ucs::y_to_l_star(xyy[2]);
    }
    gaussian_mean_blur(&mut saturation, width, height, 1, p.sat_sigma);

    // STEP 2
    if settings.use_filter {
        prefilter(
            &mut uv,
            &saturation,
            size,
            p.hue_sigma,
            p.sat_shift,
            &setup.weights,
            in_place_of(&setup.prefilter),
        );
    }
    // What the last step's HSB is computed from, before the guide blurs it.
    let prefiltered = uv.as_chunks::<2>().0.to_vec();

    // STEP 3
    let mut corrections = vec![0.0f32; 2 * n];
    let mut b_corrections = vec![0.0f32; n];
    for row in 0..height {
        for col in 0..width {
            let k = row * width + col;
            let jch = ucs::luv_to_jch(l_scharr[k], p.white, [uv[2 * k], uv[2 * k + 1]]);
            let hsb = ucs::jch_to_hsb(jch);
            if settings.use_filter {
                let vrow = (height - 2).min(row.max(1));
                let vcol = (width - 2).min(col.max(1));
                let kk = vrow * width + vcol;
                let gradient = (scharr(&saturation, kk, width) - 0.02).max(0.0);
                l_scharr[k] = p.gradient_amp * (gradient * gradient);
            }
            if jch[1] > NORM_MIN {
                corrections[2 * k] = lookup(&setup.curves.hue, hsb[0]);
                corrections[2 * k + 1] = lookup(&setup.curves.saturation, hsb[0]);
                b_corrections[k] = hsb[1] * (lookup(&setup.curves.brightness, hsb[0]) - 1.0);
            } else {
                corrections[2 * k] = 0.0;
                corrections[2 * k + 1] = 1.0;
                b_corrections[k] = 0.0;
            }
        }
    }

    if settings.use_filter {
        gaussian_mean_blur(&mut l_scharr, width, height, 1, p.scharr_sigma);
        // STEP 4
        guide(
            &mut uv,
            &mut corrections,
            &saturation,
            &mut b_corrections,
            &l_scharr,
            size,
            p.par_sigma,
            p.bright_shift,
            p.sat_shift,
            &setup.weights,
            in_place_of(&setup.guide),
        );
    }
    Guided {
        uv: prefiltered,
        corrections: (0..n)
            .map(|k| [corrections[2 * k], corrections[2 * k + 1], b_corrections[k]])
            .collect(),
    }
}

/// darktable `process()`'s last step at a pixel of lightness `l_star` whose
/// chromaticity and corrections are `uv` and `corrections`: the corrections applied, the
/// saturation gamut-mapped and the result back to working RGB.
pub fn finish(setup: &Setup, l_star: f32, uv: [f32; 2], corrections: [f32; 3]) -> [f32; 3] {
    let white = setup.process.white;
    let mut out = ucs::jch_to_hsb(ucs::luv_to_jch(l_star, white, uv));
    out[0] += corrections[0];
    out[1] = (out[1] * (1.0 + SAT_EFFECT * (corrections[1] - 1.0))).max(0.0);
    out[2] = (out[2] * (1.0 + BRIGHT_EFFECT * corrections[2])).max(0.0);
    gamut_map(&mut out, &setup.gamut, white);
    let xyz = ucs::xyy_to_xyz(ucs::jch_to_xyy(ucs::hsb_to_jch(out), white));
    apply(&setup.xyz_d65_to_rgb, xyz)
}

/// darktable `process()` on a `width` × `height` RGBA image at `scale` pixels per pixel
/// at full resolution, in the working space `profile`, without its mask display, the
/// filters blurring in place as `in_place` says.
pub fn process(
    settings: &Settings,
    width: usize,
    height: usize,
    pixels: &[[f32; 4]],
    scale: f32,
    profile: &WorkingProfile,
    in_place: InPlace,
) -> Vec<[f32; 4]> {
    let setup = Setup::new(settings, [width as u32, height as u32], scale, profile);
    let guided = guided(settings, width, height, pixels, scale, profile, in_place);
    pixels
        .iter()
        .enumerate()
        .map(|(k, pixel)| {
            let l_star = l_star(&setup, [pixel[0], pixel[1], pixel[2]]);
            let [r, g, b] = finish(&setup, l_star, guided.uv[k], guided.corrections[k]);
            [r, g, b, pixel[3]]
        })
        .collect()
}
