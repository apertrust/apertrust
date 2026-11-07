// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2018-2026  darktable developers (ported from src/iop/toneequal.c,
// src/common/luminance_mask.h and src/common/eigf.h)
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

// Tone equalizer, ported from darktable src/iop/toneequal.c: compute_luminance_mask()
// with no mask filter ("preserve details: no"), EIGF or averaged EIGF
// (fast_eigf_surface_blur() of src/common/eigf.h), and apply_toneequalizer(), with the
// estimators of src/common/luminance_mask.h. See docs/ports/toneequal.md; the setup,
// computed once per image on the CPU, is `prepare` in src/setup.rs, and the passes'
// order is `passes` there. darktable reads the curve from a table of 80,001 entries,
// 1e-4 EV apart; this kernel evaluates the sum of Gaussians the table samples, as
// darktable keeps behind DT_TONEEQ_USE_LUT for a GPU.
//
// EIGF's mask is built by whole-image passes (decision 15) over the whole image at the
// pipe's scale, W × H, and its grid, w × h, into scratch buffers that main then reads at
// the pixel's place: eigf_mask (when iterating), then per iteration eigf_downsample,
// eigf_range_partial, eigf_range, eigf_columns, eigf_rows and, but for the last,
// eigf_blend. Only the first reads the input; in an export's tiles it runs in a first
// round, each tile writing its part, and the rest once (docs/engine.md, Tiling).

import color;
import pass;
import filters.eigf;

// darktable dt_iop_luminance_mask_method_t.
static const uint RGB_AVERAGE = 0;
static const uint HSL_LIGHTNESS = 1;
static const uint HSV_VALUE = 2;
static const uint RGB_SUM = 3;
static const uint RGB_EUCLIDEAN_NORM = 4;
static const uint RGB_POWER_NORM = 5;
static const uint RGB_GEOMETRIC_MEAN = 6;

// darktable dt_iop_toneequalizer_filter_t, the filters ported.
static const uint FILTER_NONE = 0;
static const uint FILTER_AVERAGED_EIGF = 3;
static const uint FILTER_EIGF = 4;

// The curve's range, darktable DT_TONEEQ_MIN_EV and DT_TONEEQ_MAX_EV: a mask outside
// takes the correction at its end.
static const float MIN_EV = -8.0;
static const float MAX_EV = 0.0;
// The eight Gaussians' centres, darktable centers_ops: -8 + 8j/7 EV.
static const float4 CENTRES_LOW = float4(-8.0, -6.857142857142857, -5.714285714285714, -4.571428571428571);
static const float4 CENTRES_HIGH = float4(-3.4285714285714284, -2.2857142857142856, -1.1428571428571428, 0.0);
// What the estimators see is clamped to this: real images stay far inside it, and the
// power norm's cubes stay finite, so log2 never sees infinity. The output scales the
// unclamped pixel.
static const float ESTIMATOR_LIMIT = 1.0e6;
// The power norm's denominator and the geometric mean's product are at least these. A
// smaller one gives a mask below the floor even at the largest exposure compensation,
// 2^16, as darktable's 0 / 0 and cube root of 0 do.
static const float MIN_SQUARES = 1.0e-30;
static const float MIN_PRODUCT = 1.0e-30;
// eigf_variance_analysis()'s starting minimum and maximum of each channel.
static const float RANGE_MIN = 10000000.0;
static const float RANGE_MAX = 0.0;
// Larger than any mask value: the range's identity for an invocation past the grid.
static const float FLT_MAX = 3.402823466e38;

static const uint WORKGROUP = 256;

struct ToneEqualParams
{
    // The curve's weights, one per Gaussian, at CENTRES_LOW then CENTRES_HIGH:
    // darktable's fitted factors.
    float4 weights_low;
    float4 weights_high;
    // The weights' derivative along each of the four directions in band space the
    // derivative entry points differentiate against.
    float4 direction0_low;
    float4 direction0_high;
    float4 direction1_low;
    float4 direction1_high;
    float4 direction2_low;
    float4 direction2_high;
    float4 direction3_low;
    float4 direction3_high;
    // 1 / (2σ²), σ darktable's smoothing.
    float inverse_denominator;
    uint pad0;
    uint pad1;
    uint pad2;
};

// What the mask's passes read: the whole-image passes' uniforms hold this and zeros in
// place of the curve, so a band's change leaves their bytes unchanged.
struct MaskParams
{
    // RGB_* above, FILTER_* above, darktable's iterations, and whether the guide is
    // quantised (quantization is not 0).
    uint method;
    uint filter;
    uint iterations;
    uint quantised;
    // darktable's exposure_boost, linear; luminance_mask()'s fulcrum and contrast,
    // 2^-4 and the linear contrast_boost for EIGF, 0 and 1 otherwise; and the filter's
    // regularisation, 1 / feathering, darktable's d->feathering.
    float exposure_boost;
    float fulcrum;
    float contrast_boost;
    float feathering;
    // The quantiser's step in EV; the range pass's workgroups across and in all.
    float quantization;
    uint partials_across;
    uint partials;
    uint pad;
    // The whole image at the pipe's scale, and the grid.
    uint width;
    uint height;
    uint grid_width;
    uint grid_height;
    // The part of the whole image the operation's input holds: all of it, or a tile;
    // its corner is the pass header's origin.
    uint input_width;
    uint input_height;
    uint pad3;
    uint pad4;
    // The Gaussian on the grid, darktable's _compute_gauss_params().
    float4 a;
    float4 b;
};

struct Params
{
    PassHeader pass;
    ToneEqualParams tone_equal;
    MaskParams mask;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;

// The scratch buffers. The grid, w × h, of the mask and its quantised value; the
// range pass's partial results, two per workgroup (the minima, then the maxima, of the
// guide, the mask and their product), and the ranges, the guide's and its square's,
// then the mask's and the product's, each minima then maxima; the column pass's sums of
// the guide and its square, and of the mask and the product; the statistics, the
// guide's average and variance, and the mask's average and the covariance; and, when
// iterating, the mask at W × H. A buffer the settings do not need is 16 bytes, bound
// and never read.
[[vk::binding(10, 0)]] RWStructuredBuffer<float2> samples;
[[vk::binding(11, 0)]] RWStructuredBuffer<float4> partials;
[[vk::binding(12, 0)]] RWStructuredBuffer<float4> ranges;
[[vk::binding(13, 0)]] RWStructuredBuffer<float2> columns_guide;
[[vk::binding(14, 0)]] RWStructuredBuffer<float2> columns_mask;
[[vk::binding(15, 0)]] RWStructuredBuffer<float2> statistics_guide;
[[vk::binding(16, 0)]] RWStructuredBuffer<float2> statistics_mask;
[[vk::binding(17, 0)]] RWStructuredBuffer<float> mask;

groupshared float4 shared_low[WORKGROUP];
groupshared float4 shared_high[WORKGROUP];

// darktable's estimators of luminance_mask.h times exposure_boost.
float estimate(float3 rgb, MaskParams m)
{
    float3 c = clamp(rgb, float3(-ESTIMATOR_LIMIT), float3(ESTIMATOR_LIMIT));
    float3 a = abs(c);
    float e = m.exposure_boost;
    if (m.method == RGB_AVERAGE)
        return e * (c.x + c.y + c.z) / 3.0;
    if (m.method == HSL_LIGHTNESS)
        return e * (max(max(c.x, c.y), c.z) + min(min(c.x, c.y), c.z)) / 2.0;
    if (m.method == HSV_VALUE)
        return e * max(max(c.x, c.y), c.z);
    if (m.method == RGB_SUM)
        return e * (a.x + a.y + a.z);
    if (m.method == RGB_POWER_NORM)
        return e * dot(a * a, a) / max(dot(a, a), MIN_SQUARES);
    if (m.method == RGB_GEOMETRIC_MEAN)
        return e * pow0(max(a.x * a.y * a.z, MIN_PRODUCT), 1.0 / 3.0);
    return e * sqrt(dot(c, c));
}

// darktable luminance_mask(): the estimate, its contrast compensated around the
// fulcrum (linear_contrast()), floored.
float compensated(float3 rgb, MaskParams m)
{
    return max((estimate(rgb, m) - m.fulcrum) * m.contrast_boost + m.fulcrum, MIN_MASK);
}

// The quantised guide darktable builds from the mask, or the mask itself without
// quantisation, as quantize() copies it.
float guide_of(float value, MaskParams m)
{
    return m.quantised != 0 ? quantize(value, m.quantization) : value;
}

// The grid's statistics upsampled to the pixel at `at` of the whole image at the pipe's
// scale, as interpolate_bilinear() upsamples them: the guide's average and variance, and
// the mask's average and the covariance when quantising.
float4 statistics_at(uint2 at, MaskParams m)
{
    uint w = m.grid_width;
    Taps t = bilinear(at, uint2(w, m.grid_height), uint2(m.width, m.height));
    uint nw = t.y_prev * w + t.x_prev;
    uint ne = t.y_prev * w + t.x_next;
    uint sw = t.y_next * w + t.x_prev;
    uint se = t.y_next * w + t.x_next;
    float2 guide = interpolate2(t, statistics_guide[nw], statistics_guide[ne], statistics_guide[sw],
                                statistics_guide[se]);
    float2 masked = float2(0.0);
    if (m.quantised != 0)
        masked = interpolate2(t, statistics_mask[nw], statistics_mask[ne], statistics_mask[sw],
                              statistics_mask[se]);
    return float4(guide, masked);
}

// One EIGF blend of the mask `image` at `at`: eigf_blending_no_mask(), or with
// quantisation eigf_blending(), then the blend's floor, or the geometric mean with the
// unfiltered mask.
float eigf_blend_at(float image, uint2 at, MaskParams m, bool geomean)
{
    float4 s = statistics_at(at, m);
    float mapped = m.quantised != 0
        ? eigf_guided(image, quantize(image, m.quantization), s.x, s.y, s.z, s.w, m.feathering)
        : eigf_no_mask(image, s.x, s.y, m.feathering);
    return blend(image, mapped, geomean);
}

// The pixel's mask, as compute_luminance_mask() leaves it, at `at` in the whole image at
// the pipe's scale: the compensated estimate, or for EIGF its last blend, from the
// estimate or, when iterating, from the mask the earlier iterations left.
float mask_value(float3 rgb, uint2 at, MaskParams m)
{
    if (m.filter == FILTER_NONE)
        return compensated(rgb, m);
    float image = m.iterations > 1 ? mask[at.y * m.width + at.x] : compensated(rgb, m);
    return eigf_blend_at(image, at, m, m.filter == FILTER_AVERAGED_EIGF);
}

// apply_toneequalizer()'s exposure: the mask's log2, clamped to the curve's range.
float mask_exposure(float3 rgb, uint2 at, MaskParams m)
{
    return clamp(log2(mask_value(rgb, at, m)), MIN_EV, MAX_EV);
}

// Each Gaussian of the curve at a mask's exposure. They depend on the pixel alone, not
// on the bands, so the differentiable part takes them as constants.
struct Gaussians
{
    float4 low;
    float4 high;
};

Gaussians gaussians(float exposure, ToneEqualParams p)
{
    float4 d_low = float4(exposure) - CENTRES_LOW;
    float4 d_high = float4(exposure) - CENTRES_HIGH;
    Gaussians g;
    g.low = exp(-(d_low * d_low) * p.inverse_denominator);
    g.high = exp(-(d_high * d_high) * p.inverse_denominator);
    return g;
}

// The four values the derivative entry points differentiate against: how far the band
// gains move along each direction, 0 for the settings themselves.
struct Bands : IDifferentiable
{
    float t0;
    float t1;
    float t2;
    float t3;
};

// The correction for a pixel whose curve's Gaussians are `g`: the band curve, darktable's
// pixel_correction() before its table, clamped to 2 EV each way.
[Differentiable]
float correction(no_diff Gaussians g, no_diff ToneEqualParams p, Bands t)
{
    float4 weights_low = p.weights_low + t.t0 * p.direction0_low + t.t1 * p.direction1_low
        + t.t2 * p.direction2_low + t.t3 * p.direction3_low;
    float4 weights_high = p.weights_high + t.t0 * p.direction0_high + t.t1 * p.direction1_high
        + t.t2 * p.direction2_high + t.t3 * p.direction3_high;
    return clamp(dot(weights_low, g.low) + dot(weights_high, g.high), 0.25, 4.0);
}

// The per-pixel function: every channel times the pixel's correction.
[Differentiable]
float3 toneequal(no_diff float3 rgb, no_diff Gaussians g, no_diff ToneEqualParams p, Bands t)
{
    return rgb * correction(g, p, t);
}

// The pixel's place in the whole image at the pipe's scale.
uint2 place(uint3 id)
{
    return uint2(params.pass.origin_x + id.x, params.pass.origin_y + id.y);
}

// The output pass: the mask at the pixel, then the curve. Its bindings are the same for
// every filter, the scratch buffers the settings leave unused 16 bytes each, so its
// derivative entry points serve them all (decision 15).
[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    ToneEqualParams p = params.tone_equal;
    MaskParams m = params.mask;
    Bands settings = { 0.0, 0.0, 0.0, 0.0 };
    float4 pixel = input[i];
    float3 rgb = pixel.rgb;
    for (uint k = 0; k < params.pass.repeat; k++)
        rgb = toneequal(rgb, gaussians(mask_exposure(rgb, place(id), m), p), p, settings);
    output[i] = float4(rgb, pixel.a);
}

// When iterating: the compensated mask of the input's pixels, at their place in the
// mask at W × H. In a tile, the tile's part of it.
[shader("compute")]
[numthreads(16, 16, 1)]
void eigf_mask(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    MaskParams m = params.mask;
    uint2 at = place(id);
    mask[at.y * m.width + at.x] = compensated(input[id.y * params.pass.width + id.x].rgb, m);
}

// The grid sample at (x, y) of w × h, read from `read(x, y)` of the mask at W × H by
// interpolate_bilinear(): the mask and its quantised guide, each sampled as darktable
// samples the full-resolution image and its quantised copy.
float2 grid_sample(Taps t, float nw, float ne, float sw, float se, MaskParams m)
{
    float value = interpolate(t, nw, ne, sw, se);
    float guide = interpolate(t, guide_of(nw, m), guide_of(ne, m), guide_of(sw, m), guide_of(se, m));
    return float2(value, guide);
}

// Without iterating: the grid from the input, the compensated mask computed at the four
// nodes. In a tile, the samples whose north-west node it holds, but in its last column
// and row unless they are the image's: an export's first-round tiles overlap by a pixel
// to the right and below, so each sample is computed once, from nodes the tile holds.
[shader("compute")]
[numthreads(16, 16, 1)]
void eigf_downsample(uint3 id: SV_DispatchThreadID)
{
    MaskParams m = params.mask;
    if (id.x >= m.grid_width || id.y >= m.grid_height)
        return;
    Taps t = bilinear(id.xy, uint2(m.width, m.height), uint2(m.grid_width, m.grid_height));
    uint2 corner = uint2(params.pass.origin_x, params.pass.origin_y);
    uint2 size = uint2(m.input_width, m.input_height);
    uint2 owned = select(corner + size < uint2(m.width, m.height), size - 1, size);
    if (t.x_prev < corner.x || t.y_prev < corner.y || t.x_prev >= corner.x + owned.x
        || t.y_prev >= corner.y + owned.y)
        return;
    uint north = (t.y_prev - corner.y) * size.x;
    uint south = (t.y_next - corner.y) * size.x;
    uint west = t.x_prev - corner.x;
    uint east = t.x_next - corner.x;
    float nw = compensated(input[north + west].rgb, m);
    float ne = compensated(input[north + east].rgb, m);
    float sw = compensated(input[south + west].rgb, m);
    float se = compensated(input[south + east].rgb, m);
    samples[id.y * m.grid_width + id.x] = grid_sample(t, nw, ne, sw, se, m);
}

// When iterating: the grid from the mask the earlier iterations left.
[shader("compute")]
[numthreads(16, 16, 1)]
void eigf_downsample_mask(uint3 id: SV_DispatchThreadID)
{
    MaskParams m = params.mask;
    if (id.x >= m.grid_width || id.y >= m.grid_height)
        return;
    Taps t = bilinear(id.xy, uint2(m.width, m.height), uint2(m.grid_width, m.grid_height));
    float nw = mask[t.y_prev * m.width + t.x_prev];
    float ne = mask[t.y_prev * m.width + t.x_next];
    float sw = mask[t.y_next * m.width + t.x_prev];
    float se = mask[t.y_next * m.width + t.x_next];
    samples[id.y * m.grid_width + id.x] = grid_sample(t, nw, ne, sw, se, m);
}

// The guide, the mask and their product at a grid sample: the guide is the quantised
// mask, or the mask itself without quantisation.
float3 channels_of(float2 sample)
{
    return float3(sample.y, sample.x, sample.x * sample.y);
}

// The ranges' first pass (decision 15): each workgroup's minima and maxima of the guide,
// the mask and their product over its grid samples.
[shader("compute")]
[numthreads(16, 16, 1)]
void eigf_range_partial(uint3 id: SV_DispatchThreadID, uint3 group: SV_GroupID, uint t: SV_GroupIndex)
{
    MaskParams m = params.mask;
    float4 low = float4(FLT_MAX);
    float4 high = float4(-FLT_MAX);
    if (id.x < m.grid_width && id.y < m.grid_height)
    {
        float3 c = channels_of(samples[id.y * m.grid_width + id.x]);
        low = float4(c, 0.0);
        high = float4(c, 0.0);
    }
    shared_low[t] = low;
    shared_high[t] = high;
    GroupMemoryBarrierWithGroupSync();
    for (uint stride = WORKGROUP / 2; stride > 0; stride /= 2)
    {
        if (t < stride)
        {
            shared_low[t] = min(shared_low[t], shared_low[t + stride]);
            shared_high[t] = max(shared_high[t], shared_high[t + stride]);
        }
        GroupMemoryBarrierWithGroupSync();
    }
    if (t == 0)
    {
        uint index = group.y * m.partials_across + group.x;
        partials[2 * index] = shared_low[0];
        partials[2 * index + 1] = shared_high[0];
    }
}

// The ranges' second pass, one workgroup: the partial results combined, from
// eigf_variance_analysis()'s starting values, the square's range from the guide's,
// which is positive. Writes the guide's and its square's minima, then maxima, then the
// mask's and the product's.
[shader("compute")]
[numthreads(16, 16, 1)]
void eigf_range(uint t: SV_GroupIndex)
{
    MaskParams m = params.mask;
    float4 low = float4(FLT_MAX);
    float4 high = float4(-FLT_MAX);
    for (uint k = t; k < m.partials; k += WORKGROUP)
    {
        low = min(low, partials[2 * k]);
        high = max(high, partials[2 * k + 1]);
    }
    shared_low[t] = low;
    shared_high[t] = high;
    GroupMemoryBarrierWithGroupSync();
    for (uint stride = WORKGROUP / 2; stride > 0; stride /= 2)
    {
        if (t < stride)
        {
            shared_low[t] = min(shared_low[t], shared_low[t + stride]);
            shared_high[t] = max(shared_high[t], shared_high[t + stride]);
        }
        GroupMemoryBarrierWithGroupSync();
    }
    if (t == 0)
    {
        float3 lowest = shared_low[0].xyz;
        float3 highest = shared_high[0].xyz;
        float2 low_guide = min(float2(RANGE_MIN), float2(lowest.x, lowest.x * lowest.x));
        float2 high_guide = max(float2(RANGE_MAX), float2(highest.x, highest.x * highest.x));
        float2 low_mask = min(float2(RANGE_MIN), lowest.yz);
        float2 high_mask = max(float2(RANGE_MAX), highest.yz);
        ranges[0] = float4(low_guide, high_guide);
        ranges[1] = float4(low_mask, high_mask);
    }
}

// The Gaussian's coefficients from the uniforms.
Deriche deriche(MaskParams m)
{
    Deriche d;
    d.a0 = m.a.x;
    d.a1 = m.a.y;
    d.a2 = m.a.z;
    d.a3 = m.a.w;
    d.b1 = m.b.x;
    d.b2 = m.b.y;
    d.coefp = m.b.z;
    d.coefn = m.b.w;
    return d;
}

// The two channels the guide's blur and the mask's blur take at a sample, clamped to
// their ranges: the guide and its square, or the mask and the product.
float2 guide_channels(float2 sample, float4 range)
{
    float g = sample.y;
    return clampf2(float2(g, g * g), range.xy, range.zw);
}

float2 mask_channels(float2 sample, float4 range)
{
    return clampf2(float2(sample.x, sample.x * sample.y), range.xy, range.zw);
}

// dt_gaussian_blur()'s vertical pass, one invocation per grid column, each walking its
// rows as darktable's loop does: the causal sums written, then the anticausal ones added.
// The guide's two channels, and with quantisation the mask's.
[shader("compute")]
[numthreads(16, 16, 1)]
void eigf_columns(uint3 id: SV_DispatchThreadID)
{
    MaskParams m = params.mask;
    uint column = id.y * 16 + id.x;
    uint w = m.grid_width;
    uint h = m.grid_height;
    if (column >= w)
        return;
    Deriche d = deriche(m);
    float4 range_guide = ranges[0];
    float4 range_mask = ranges[1];
    bool quantised = m.quantised != 0;

    float2 first = samples[column];
    float2 xp = guide_channels(first, range_guide);
    float2 yb = xp * d.coefp;
    float2 yp = yb;
    float2 xp_m = mask_channels(first, range_mask);
    float2 yb_m = xp_m * d.coefp;
    float2 yp_m = yb_m;
    for (uint row = 0; row < h; row++)
    {
        uint at = row * w + column;
        float2 sample = samples[at];
        float2 xc = guide_channels(sample, range_guide);
        float2 yc = causal(d, xc, xp, yp, yb);
        columns_guide[at] = yc;
        xp = xc;
        yb = yp;
        yp = yc;
        if (quantised)
        {
            float2 xc_m = mask_channels(sample, range_mask);
            float2 yc_m = causal(d, xc_m, xp_m, yp_m, yb_m);
            columns_mask[at] = yc_m;
            xp_m = xc_m;
            yb_m = yp_m;
            yp_m = yc_m;
        }
    }

    float2 last = samples[(h - 1) * w + column];
    float2 xn = guide_channels(last, range_guide);
    float2 xa = xn;
    float2 yn = xn * d.coefn;
    float2 ya = yn;
    float2 xn_m = mask_channels(last, range_mask);
    float2 xa_m = xn_m;
    float2 yn_m = xn_m * d.coefn;
    float2 ya_m = yn_m;
    for (uint row = h; row > 0; row--)
    {
        uint at = (row - 1) * w + column;
        float2 sample = samples[at];
        float2 xc = guide_channels(sample, range_guide);
        float2 yc = anticausal(d, xn, xa, yn, ya);
        xa = xn;
        xn = xc;
        ya = yn;
        yn = yc;
        columns_guide[at] = columns_guide[at] + yc;
        if (quantised)
        {
            float2 xc_m = mask_channels(sample, range_mask);
            float2 yc_m = anticausal(d, xn_m, xa_m, yn_m, ya_m);
            xa_m = xn_m;
            xn_m = xc_m;
            ya_m = yn_m;
            yn_m = yc_m;
            columns_mask[at] = columns_mask[at] + yc_m;
        }
    }
}

// dt_gaussian_blur()'s horizontal pass, one invocation per grid row, its input the
// vertical pass's output clamped to the same ranges, then eigf_variance_analysis()'s
// variance and covariance from the blurred channels.
[shader("compute")]
[numthreads(16, 16, 1)]
void eigf_rows(uint3 id: SV_DispatchThreadID)
{
    MaskParams m = params.mask;
    uint row = id.y * 16 + id.x;
    uint w = m.grid_width;
    if (row >= m.grid_height)
        return;
    Deriche d = deriche(m);
    float4 range_guide = ranges[0];
    float4 range_mask = ranges[1];
    bool quantised = m.quantised != 0;
    uint start = row * w;

    float2 xp = clampf2(columns_guide[start], range_guide.xy, range_guide.zw);
    float2 yb = xp * d.coefp;
    float2 yp = yb;
    // The mask's channels only with quantisation: their buffers are 16 bytes otherwise.
    float2 xp_m = float2(0.0);
    if (quantised)
        xp_m = clampf2(columns_mask[start], range_mask.xy, range_mask.zw);
    float2 yb_m = xp_m * d.coefp;
    float2 yp_m = yb_m;
    for (uint x = 0; x < w; x++)
    {
        float2 xc = clampf2(columns_guide[start + x], range_guide.xy, range_guide.zw);
        float2 yc = causal(d, xc, xp, yp, yb);
        statistics_guide[start + x] = yc;
        xp = xc;
        yb = yp;
        yp = yc;
        if (quantised)
        {
            float2 xc_m = clampf2(columns_mask[start + x], range_mask.xy, range_mask.zw);
            float2 yc_m = causal(d, xc_m, xp_m, yp_m, yb_m);
            statistics_mask[start + x] = yc_m;
            xp_m = xc_m;
            yb_m = yp_m;
            yp_m = yc_m;
        }
    }

    float2 xn = clampf2(columns_guide[start + w - 1], range_guide.xy, range_guide.zw);
    float2 xa = xn;
    float2 yn = xn * d.coefn;
    float2 ya = yn;
    float2 xn_m = float2(0.0);
    if (quantised)
        xn_m = clampf2(columns_mask[start + w - 1], range_mask.xy, range_mask.zw);
    float2 xa_m = xn_m;
    float2 yn_m = xn_m * d.coefn;
    float2 ya_m = yn_m;
    for (uint x = w; x > 0; x--)
    {
        uint at = start + x - 1;
        float2 xc = clampf2(columns_guide[at], range_guide.xy, range_guide.zw);
        float2 yc = anticausal(d, xn, xa, yn, ya);
        xa = xn;
        xn = xc;
        ya = yn;
        yn = yc;
        float2 guide = statistics_guide[at] + yc;
        // The average, and the variance: the blurred square less the average's square.
        statistics_guide[at] = float2(guide.x, guide.y - guide.x * guide.x);
        if (quantised)
        {
            float2 xc_m = clampf2(columns_mask[at], range_mask.xy, range_mask.zw);
            float2 yc_m = anticausal(d, xn_m, xa_m, yn_m, ya_m);
            xa_m = xn_m;
            xn_m = xc_m;
            ya_m = yn_m;
            yn_m = yc_m;
            float2 masked = statistics_mask[at] + yc_m;
            // The mask's average, and the covariance: the blurred product less the
            // product of the averages.
            statistics_mask[at] = float2(masked.x, masked.y - guide.x * masked.x);
        }
    }
}

// Every iteration's blend but the last, which main does: the mask at W × H blended
// linearly in place, as fast_eigf_surface_blur() blends its intermediate images.
[shader("compute")]
[numthreads(16, 16, 1)]
void eigf_blend(uint3 id: SV_DispatchThreadID)
{
    MaskParams m = params.mask;
    if (id.x >= m.width || id.y >= m.height)
        return;
    uint at = id.y * m.width + id.x;
    mask[at] = eigf_blend_at(mask[at], id.xy, m, false);
}

// Derivative entry points, both per pixel, with parameters ordered as the directions:
// direction0 to direction3, each the derivative of the output with respect to the band
// gains moved along it, EV per unit, at the settings. The mask does not depend on the
// bands, so it is a constant, read as main reads it.
[[vk::binding(8, 0)]] StructuredBuffer<float4> derivative_in;
[[vk::binding(9, 0)]] RWStructuredBuffer<float4> derivative_out;

// Reverse mode: given dLoss/dOutput, the gradient of the loss along the directions.
[shader("compute")]
[numthreads(16, 16, 1)]
void toneequal_backward(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    ToneEqualParams p = params.tone_equal;
    float3 rgb = input[i].rgb;
    Bands settings = { 0.0, 0.0, 0.0, 0.0 };
    var t = diffPair(settings);
    Gaussians g = gaussians(mask_exposure(rgb, place(id), params.mask), p);
    bwd_diff(toneequal)(rgb, g, p, t, derivative_in[i].rgb);
    derivative_out[i] = float4(t.d.t0, t.d.t1, t.d.t2, t.d.t3);
}

// Forward mode: given a direction in t, the directional derivative of the output.
// Checks the reverse mode: for any direction v and output weights w, w · (J v) must
// equal (Jᵀ w) · v.
[shader("compute")]
[numthreads(16, 16, 1)]
void toneequal_forward_diff(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    ToneEqualParams p = params.tone_equal;
    float3 rgb = input[i].rgb;
    float4 v = derivative_in[i];
    Bands settings = { 0.0, 0.0, 0.0, 0.0 };
    Bands.Differential direction;
    direction.t0 = v.x;
    direction.t1 = v.y;
    direction.t2 = v.z;
    direction.t3 = v.w;
    Gaussians g = gaussians(mask_exposure(rgb, place(id), params.mask), p);
    let result = fwd_diff(toneequal)(rgb, g, p, diffPair(settings, direction));
    derivative_out[i] = float4(result.d, 0.0);
}
