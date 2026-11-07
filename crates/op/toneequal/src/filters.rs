// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2012-2026  darktable developers (ported from src/common/eigf.h,
// src/common/fast_guided_filter.h, src/common/gaussian.c and src/common/luminance_mask.h)
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

//! darktable's mask filters restated on the CPU, in f32 and darktable's order of
//! arithmetic, loop by loop: the references the kernel's whole-image passes are tested
//! against (`crates/golden/tests/toneequal.rs`). They are the Rust side of
//! `slang/filters/eigf.slang`, kept in the operation's crate until a second operation
//! needs them (decision 15); nothing renders through them.

use crate::{Estimator, Filter, Settings, radius};

/// darktable `MIN_FLOAT`, 2^-16: the mask's floor.
pub const MIN_FLOAT: f32 = 1.525_878_9e-5;
/// darktable `CONTRAST_FULCRUM`, 2^-4.
const CONTRAST_FULCRUM: f32 = 0.0625;

/// darktable `luminance_mask()` for one pixel: an estimator of `luminance_mask.h` times
/// `exposure_boost`, its contrast compensated around `fulcrum` (`linear_contrast()`),
/// floored.
pub fn luminance(
    rgb: [f32; 3],
    method: Estimator,
    exposure_boost: f32,
    fulcrum: f32,
    contrast: f32,
) -> f32 {
    let [r, g, b] = rgb;
    let lum = match method {
        Estimator::Average => exposure_boost * (0.0 + r + g + b) / 3.0,
        Estimator::Lightness => exposure_boost * (r.max(g).max(b) + r.min(g).min(b)) / 2.0,
        Estimator::Value => exposure_boost * r.max(g).max(b),
        Estimator::Sum => exposure_boost * (0.0 + r.abs() + g.abs() + b.abs()),
        Estimator::EuclideanNorm => exposure_boost * (0.0 + r * r + g * g + b * b).sqrt(),
        Estimator::PowerNorm => {
            let (mut numerator, mut denominator) = (0.0f32, 0.0f32);
            for value in rgb.map(f32::abs) {
                let square = value * value;
                numerator += square * value;
                denominator += square;
            }
            exposure_boost * numerator / denominator
        }
        Estimator::GeometricMean => {
            exposure_boost * (1.0 * r.abs() * g.abs() * b.abs()).powf(1.0 / 3.0)
        }
    };
    ((lum - fulcrum) * contrast + fulcrum).max(MIN_FLOAT)
}

/// darktable `interpolate_bilinear()` on `ch` interleaved channels.
pub fn interpolate_bilinear(
    input: &[f32],
    width_in: usize,
    height_in: usize,
    width_out: usize,
    height_out: usize,
    ch: usize,
) -> Vec<f32> {
    let mut out = vec![0.0; width_out * height_out * ch];
    for i in 0..height_out {
        for j in 0..width_out {
            let x_in = (j as f32 / width_out as f32) * width_in as f32;
            let y_in = (i as f32 / height_out as f32) * height_in as f32;
            let x_prev = x_in.floor() as usize;
            let y_prev = y_in.floor() as usize;
            let x_next = (x_prev + 1).min(width_in - 1);
            let y_next = (y_prev + 1).min(height_in - 1);
            let x_prev = x_prev.min(width_in - 1);
            let y_prev = y_prev.min(height_in - 1);
            let at = |y: usize, x: usize, c: usize| input[(y * width_in + x) * ch + c];
            let dy_next = y_next as f32 - y_in;
            let dy_prev = 1.0 - dy_next;
            let dx_next = x_next as f32 - x_in;
            let dx_prev = 1.0 - dx_next;
            for c in 0..ch {
                out[(i * width_out + j) * ch + c] = dy_prev
                    * (at(y_next, x_prev, c) * dx_next + at(y_next, x_next, c) * dx_prev)
                    + dy_next * (at(y_prev, x_prev, c) * dx_next + at(y_prev, x_next, c) * dx_prev);
            }
        }
    }
    out
}

/// darktable `quantize()` with toneequal's range, 2^-14 to 4: a copy for a step of 0.
pub fn quantize(image: &[f32], sampling: f32) -> Vec<f32> {
    if sampling == 0.0 {
        return image.to_vec();
    }
    image
        .iter()
        .map(|&v| {
            // fast_clamp(), fmaxf(fminf(...)): the same for a mask, never NaN.
            (((v.log2() / sampling).floor() * sampling).exp2()).clamp(6.103_515_6e-5, 4.0)
        })
        .collect()
}

/// darktable `CLAMPF`: `mn` when `a` is below it, even when `mn > mx`.
fn clampf(a: f32, mn: f32, mx: f32) -> f32 {
    if a >= mn {
        if a <= mx { a } else { mx }
    } else {
        mn
    }
}

/// darktable `dt_gaussian_blur()` (and `dt_gaussian_blur_4c()`, the same arithmetic per
/// channel) of order 0 on `ch` interleaved channels, each clamped to `min` and `max`:
/// Deriche's recursion down each column, then along each row of the result.
pub fn gaussian_blur(
    input: &[f32],
    width: usize,
    height: usize,
    ch: usize,
    min: &[f32],
    max: &[f32],
    sigma: f32,
) -> Vec<f32> {
    let d = crate::Deriche::new(sigma);
    let clamp = |v: f32, k: usize| clampf(v, min[k], max[k]);
    let mut temp = vec![0.0f32; input.len()];
    for i in 0..width {
        for k in 0..ch {
            let at = |j: usize| (j * width + i) * ch + k;
            let mut xp = clamp(input[at(0)], k);
            let mut yb = xp * d.coefp;
            let mut yp = yb;
            for j in 0..height {
                let xc = clamp(input[at(j)], k);
                let yc = (d.a0 * xc) + (d.a1 * xp) - (d.b1 * yp) - (d.b2 * yb);
                temp[at(j)] = yc;
                xp = xc;
                yb = yp;
                yp = yc;
            }
            let mut xn = clamp(input[at(height - 1)], k);
            let mut xa = xn;
            let mut yn = xn * d.coefn;
            let mut ya = yn;
            for j in (0..height).rev() {
                let xc = clamp(input[at(j)], k);
                let yc = (d.a2 * xn) + (d.a3 * xa) - (d.b1 * yn) - (d.b2 * ya);
                xa = xn;
                xn = xc;
                ya = yn;
                yn = yc;
                temp[at(j)] += yc;
            }
        }
    }
    let mut out = vec![0.0f32; input.len()];
    for j in 0..height {
        for k in 0..ch {
            let at = |i: usize| (j * width + i) * ch + k;
            let mut xp = clamp(temp[at(0)], k);
            let mut yb = xp * d.coefp;
            let mut yp = yb;
            for i in 0..width {
                let xc = clamp(temp[at(i)], k);
                let yc = (d.a0 * xc) + (d.a1 * xp) - (d.b1 * yp) - (d.b2 * yb);
                out[at(i)] = yc;
                xp = xc;
                yb = yp;
                yp = yc;
            }
            let mut xn = clamp(temp[at(width - 1)], k);
            let mut xa = xn;
            let mut yn = xn * d.coefn;
            let mut ya = yn;
            for i in (0..width).rev() {
                let xc = clamp(temp[at(i)], k);
                let yc = (d.a2 * xn) + (d.a3 * xa) - (d.b1 * yn) - (d.b2 * ya);
                xa = xn;
                xn = xc;
                ya = yn;
                yn = yc;
                out[at(i)] += yc;
            }
        }
    }
    out
}

/// darktable `eigf_variance_analysis_no_mask()`: the guide's average and variance.
pub fn eigf_variance_analysis_no_mask(
    guide: &[f32],
    width: usize,
    height: usize,
    sigma: f32,
) -> Vec<f32> {
    let mut input = Vec::with_capacity(guide.len() * 2);
    let (mut min, mut max) = ([10_000_000.0f32; 2], [0.0f32; 2]);
    for &g in guide {
        let channels = [g, g * g];
        for k in 0..2 {
            min[k] = min[k].min(channels[k]);
            max[k] = max[k].max(channels[k]);
        }
        input.extend(channels);
    }
    let mut out = gaussian_blur(&input, width, height, 2, &min, &max, sigma);
    for pixel in out.as_chunks_mut::<2>().0 {
        let avg = pixel[0];
        pixel[1] -= avg * avg;
    }
    out
}

/// darktable `eigf_variance_analysis()`: the guide's average and variance, the mask's
/// average and their covariance.
pub fn eigf_variance_analysis(
    guide: &[f32],
    mask: &[f32],
    width: usize,
    height: usize,
    sigma: f32,
) -> Vec<f32> {
    let mut input = Vec::with_capacity(guide.len() * 4);
    let (mut min, mut max) = ([10_000_000.0f32; 4], [0.0f32; 4]);
    for (&g, &m) in guide.iter().zip(mask) {
        let channels = [g, g * g, m, m * g];
        for k in 0..4 {
            min[k] = min[k].min(channels[k]);
            max[k] = max[k].max(channels[k]);
        }
        input.extend(channels);
    }
    let mut out = gaussian_blur(&input, width, height, 4, &min, &max, sigma);
    for pixel in out.as_chunks_mut::<4>().0 {
        pixel[1] -= pixel[0] * pixel[0];
        pixel[3] -= pixel[0] * pixel[2];
    }
    out
}

/// The blend's last step: `DT_GF_BLENDING_LINEAR`, or `DT_GF_BLENDING_GEOMEAN`.
fn blend(image: f32, mapped: f32, geomean: bool) -> f32 {
    if geomean {
        (image * mapped.max(MIN_FLOAT)).sqrt()
    } else {
        mapped.max(MIN_FLOAT)
    }
}

/// darktable `eigf_blending_no_mask()`, in place.
pub fn eigf_blending_no_mask(image: &mut [f32], av: &[f32], feathering: f32, geomean: bool) {
    for (pixel, av) in image.iter_mut().zip(av.as_chunks::<2>().0) {
        let (avg_g, var_g) = (av[0], av[1]);
        let norm_g = (avg_g * *pixel).max(1e-6);
        let normalized_var_guide = var_g / norm_g;
        let a = normalized_var_guide / (normalized_var_guide + feathering);
        let b = avg_g - a * avg_g;
        *pixel = blend(*pixel, *pixel * a + b, geomean);
    }
}

/// darktable `eigf_blending()`, in place, `mask` the quantised guide.
pub fn eigf_blending(image: &mut [f32], mask: &[f32], av: &[f32], feathering: f32, geomean: bool) {
    for ((pixel, &m), av) in image.iter_mut().zip(mask).zip(av.as_chunks::<4>().0) {
        let (avg_g, var_g, avg_m, covar_mg) = (av[0], av[1], av[2], av[3]);
        let norm_g = (avg_g * *pixel).max(1e-6);
        let norm_m = (avg_m * m).max(1e-6);
        let normalized_var_guide = var_g / norm_g;
        let normalized_covar = covar_mg / (norm_g * norm_m).sqrt();
        let a = normalized_covar / (normalized_var_guide + feathering);
        let b = avg_m - a * avg_g;
        *pixel = blend(*pixel, *pixel * a + b, geomean);
    }
}

/// darktable `fast_eigf_surface_blur()` on a grey image in place, with toneequal's
/// quantiser range; the grid's sides at least 1, as the port makes them.
#[allow(clippy::too_many_arguments, reason = "darktable's signature")]
pub fn fast_eigf_surface_blur(
    image: &mut [f32],
    width: usize,
    height: usize,
    sigma: f32,
    feathering: f32,
    iterations: u32,
    geomean: bool,
    quantization: f32,
) {
    let scaling = sigma.clamp(1.0, 4.0);
    let ds_sigma = (sigma / scaling).max(1.0);
    let ds_height = ((height as f32 / scaling) as usize).max(1);
    let ds_width = ((width as f32 / scaling) as usize).max(1);
    for i in 0..iterations {
        let last = i + 1 == iterations;
        let geomean = last && geomean;
        let ds_image = interpolate_bilinear(image, width, height, ds_width, ds_height, 1);
        if quantization != 0.0 {
            let mask = quantize(image, quantization);
            let ds_mask = interpolate_bilinear(&mask, width, height, ds_width, ds_height, 1);
            let ds_av = eigf_variance_analysis(&ds_mask, &ds_image, ds_width, ds_height, ds_sigma);
            let av = interpolate_bilinear(&ds_av, ds_width, ds_height, width, height, 4);
            eigf_blending(image, &mask, &av, feathering, geomean);
        } else {
            let ds_av = eigf_variance_analysis_no_mask(&ds_image, ds_width, ds_height, ds_sigma);
            let av = interpolate_bilinear(&ds_av, ds_width, ds_height, width, height, 2);
            eigf_blending_no_mask(image, &av, feathering, geomean);
        }
    }
}

/// darktable `compute_luminance_mask()` for the ported filters, on an RGB image at full
/// resolution: no filter, EIGF or averaged EIGF. The radius is measured on `full`,
/// darktable's `piece->iwidth` and `iheight`: for a raw its readout before its crop.
pub fn compute_luminance_mask(
    pixels: &[[f32; 3]],
    width: usize,
    height: usize,
    full: [u32; 2],
    settings: &Settings,
) -> Vec<f32> {
    let exposure_boost = settings.exposure_boost.exp2();
    let (fulcrum, contrast) = match settings.details {
        Filter::Eigf => (CONTRAST_FULCRUM, settings.contrast_boost.exp2()),
        _ => (0.0, 1.0),
    };
    let mut mask: Vec<f32> = pixels
        .iter()
        .map(|&rgb| luminance(rgb, settings.method, exposure_boost, fulcrum, contrast))
        .collect();
    if matches!(settings.details, Filter::Eigf | Filter::AveragedEigf) {
        let sigma = radius(settings.blending, full) as f32;
        fast_eigf_surface_blur(
            &mut mask,
            width,
            height,
            sigma,
            1.0 / settings.feathering,
            settings.iterations.max(1) as u32,
            settings.details == Filter::AveragedEigf,
            settings.quantization,
        );
    }
    mask
}
