// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2012-2026  darktable developers (ported from src/common/eigf.h,
// src/common/fast_guided_filter.h and src/common/gaussian.c)
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

// darktable's helpers from src/common/ that the tone equalizer's EIGF mask uses, in
// darktable's order of arithmetic: the bilinear resampling of fast_guided_filter.h
// (interpolate_bilinear), its quantiser (quantize), the clamp and the recursion of
// gaussian.c's Deriche filter (dt_gaussian_blur), and eigf.h's blends. They stay in the
// operation's crate until a second operation needs them (decision 15); src/filters.rs
// holds their Rust references. In a folder of its own, so build-kernels.sh compiles it
// only as the kernel imports it.
module eigf;

// darktable MIN_FLOAT, 2^-16: the mask's floor.
public static const float MIN_MASK = 1.52587890625e-5;
// The quantiser's range, as toneequal.c passes it: 2^-14 to 4.
static const float QUANTIZE_MIN = 6.103515625e-5;
static const float QUANTIZE_MAX = 4.0;

// Where interpolate_bilinear() reads an image size_in across and down for the pixel at
// `at` of one size_out: the four nodes and their weights.
public struct Taps
{
    public uint x_prev;
    public uint x_next;
    public uint y_prev;
    public uint y_next;
    // Dx_next is x_prev's weight and Dx_prev x_next's, as darktable names them.
    public float dx_next;
    public float dx_prev;
    public float dy_next;
    public float dy_prev;
};

// darktable interpolate_bilinear(), for one output pixel: corner-aligned, no half-pixel
// offset, the nodes clamped to the image, the weights from the clamped next node.
public Taps bilinear(uint2 at, uint2 size_in, uint2 size_out)
{
    float x_in = (float(at.x) / float(size_out.x)) * float(size_in.x);
    float y_in = (float(at.y) / float(size_out.y)) * float(size_in.y);
    uint x_prev = uint(floor(x_in));
    uint y_prev = uint(floor(y_in));
    Taps t;
    t.x_next = min(x_prev + 1, size_in.x - 1);
    t.y_next = min(y_prev + 1, size_in.y - 1);
    t.x_prev = min(x_prev, size_in.x - 1);
    t.y_prev = min(y_prev, size_in.y - 1);
    t.dx_next = float(t.x_next) - x_in;
    t.dx_prev = 1.0 - t.dx_next;
    t.dy_next = float(t.y_next) - y_in;
    t.dy_prev = 1.0 - t.dy_next;
    return t;
}

// interpolate_bilinear()'s sum for one channel, from the values at the nodes: north is
// y_prev, west x_prev.
public float interpolate(Taps t, float nw, float ne, float sw, float se)
{
    return t.dy_prev * (sw * t.dx_next + se * t.dx_prev) + t.dy_next * (nw * t.dx_next + ne * t.dx_prev);
}

// The same for two channels at once, each as interpolate() computes it.
public float2 interpolate2(Taps t, float2 nw, float2 ne, float2 sw, float2 se)
{
    return t.dy_prev * (sw * t.dx_next + se * t.dx_prev) + t.dy_next * (nw * t.dx_next + ne * t.dx_prev);
}

// darktable quantize() for one value, `sampling` EV apart, sampling not 0: its fast
// track for 1 is the same arithmetic.
public float quantize(float value, float sampling)
{
    return max(min(exp2(floor(log2(value) / sampling) * sampling), QUANTIZE_MAX), QUANTIZE_MIN);
}

// darktable CLAMPF(a, mn, mx), which takes mn when a is below it even when mn > mx.
public float clampf(float a, float low, float high)
{
    return a < low ? low : min(a, high);
}

public float2 clampf2(float2 a, float2 low, float2 high)
{
    return float2(clampf(a.x, low.x, high.x), clampf(a.y, low.y, high.y));
}

// dt_gaussian_blur()'s coefficients, _compute_gauss_params() for order 0, computed on the
// CPU in darktable's f32 arithmetic.
public struct Deriche
{
    public float a0;
    public float a1;
    public float a2;
    public float a3;
    public float b1;
    public float b2;
    public float coefp;
    public float coefn;
};

// One step of the causal recursion: y = a0 x + a1 x[-1] - b1 y[-1] - b2 y[-2].
public float2 causal(Deriche d, float2 xc, float2 xp, float2 yp, float2 yb)
{
    return (d.a0 * xc) + (d.a1 * xp) - (d.b1 * yp) - (d.b2 * yb);
}

// One step of the anticausal recursion: y = a2 x[+1] + a3 x[+2] - b1 y[+1] - b2 y[+2].
public float2 anticausal(Deriche d, float2 xn, float2 xa, float2 yn, float2 ya)
{
    return (d.a2 * xn) + (d.a3 * xa) - (d.b1 * yn) - (d.b2 * ya);
}

// eigf_blending_no_mask()'s affine map of one pixel, before its floor: the guide is the
// mask itself, its average and variance on the grid upsampled to the pixel.
public float eigf_no_mask(float image, float avg_g, float var_g, float feathering)
{
    float norm_g = max(avg_g * image, 1.0e-6);
    float normalized_var_guide = var_g / norm_g;
    float a = normalized_var_guide / (normalized_var_guide + feathering);
    float b = avg_g - a * avg_g;
    return image * a + b;
}

// eigf_blending()'s, with the quantised mask as the guide: `image` the mask, `mask` its
// quantised value, darktable's names; the guide's average and variance, the mask's
// average and their covariance upsampled to the pixel.
public float eigf_guided(float image, float mask, float avg_g, float var_g, float avg_m, float covar_mg,
                         float feathering)
{
    float norm_g = max(avg_g * image, 1.0e-6);
    float norm_m = max(avg_m * mask, 1.0e-6);
    float normalized_var_guide = var_g / norm_g;
    float normalized_covar = covar_mg / sqrt(norm_g * norm_m);
    float a = normalized_covar / (normalized_var_guide + feathering);
    float b = avg_m - a * avg_g;
    return image * a + b;
}

// The blend's last step, DT_GF_BLENDING_LINEAR or, with `geomean`, DT_GF_BLENDING_GEOMEAN:
// the floored map, or its geometric mean with the unfiltered mask.
public float blend(float image, float mapped, bool geomean)
{
    float floored = max(mapped, MIN_MASK);
    return geomean ? sqrt(image * floored) : floored;
}
