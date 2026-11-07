// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2018-2026  darktable developers (ported from src/iop/filmicrgb.c,
// src/common/bspline.h, src/develop/noise_generator.h, src/common/gamut_mapping.h and
// src/common/math.h)
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

// Filmic rgb, ported from darktable src/iop/filmicrgb.c, process(). See
// docs/ports/filmicrgb.md; `prepare` in src/setup.rs fills the parameters and lists the
// passes. The highlight reconstruction's whole-image passes (hr_*) rebuild clipped
// areas in scratch; `main` tone-maps the reconstructed image or the input, with the
// colour science the uniform names.
//
// Scratch: 10 the mask, a float per pixel; 11 the clipped counts, the total then one
// partial per workgroup; 12 the detail (the inpainted image, then each scale's detail
// and its high frequencies in place, then the ratios); 13 the low frequencies; 14 the
// vertical blur; 15 the reconstructed image; 16 the norms. 12 and 13 swap buffers from
// scale to scale, as darktable's LF_odd and LF_even.

import color;
import pass;

// darktable dt_iop_filmicrgb_colorscience_type_t, by its descriptions: V1 is v3 (2019).
static const uint V3_2019 = 0;
static const uint V4_2020 = 1;
static const uint V5_2021 = 2;
static const uint V6_2022 = 3;
static const uint V7_2023 = 4;

// darktable dt_iop_filmicrgb_methods_type_t.
static const uint NORM_NO = 0;
static const uint NORM_MAX_RGB = 1;
static const uint NORM_LUMINANCE = 2;
static const uint NORM_POWER = 3;
static const uint NORM_EUCLIDEAN_LEGACY = 4;
static const uint NORM_EUCLIDEAN = 5;

// darktable dt_iop_filmicrgb_curve_type_t.
static const uint CURVE_HARD = 0;
static const uint CURVE_SOFT = 1;
static const uint CURVE_SAFE = 2;

// darktable dt_noise_distribution_t.
static const uint NOISE_UNIFORM = 0;
static const uint NOISE_GAUSSIAN = 1;
static const uint NOISE_POISSONIAN = 2;

// darktable DT_FILMIC_RECONSTRUCT_RGB and _RATIOS.
static const uint VARIANT_RGB = 0;
static const uint VARIANT_RATIOS = 1;

static const float INVERSE_SQRT_3 = 0.5773502691896258;
static const float FLOAT_MAX = 3.40282347e+38;
static const float FLOAT_MIN = 1.17549435e-38;
// CIE 1931 Y to CIE 2006 Y for greys (gamut_mapping.h).
static const float Y_2006 = 1.05785528;
static const float TWO_PI = 6.28318530717958647693;

struct FilmicParams
{
    // The whole image at the pipe's scale: x width, y height; z the partial counts'
    // workgroups across; w a blur's step.
    uint4 image;
    // x the scale, y the last scale, z the variant, w the noise's distribution.
    uint4 wavelet;
    // x normalize, y feathering, z the noise level, w the threshold.
    float4 mask;
    // x gamma (structure vs texture), y beta (grey vs colour), z delta (bloom vs
    // details), each (p / 100 + 1) / 2.
    float4 balance;
    // x the colour science, y the norm, z the toe's curve, w the shoulder's.
    uint4 modes;
    // x 1 when the reconstruction runs, y the partial counts, z 1 when v3 to v5 encode
    // every pixel to 1 (a custom grey source of 0).
    uint4 flags;
    // The working profile's Y row of RGB to XYZ D50.
    float4 luminance;
    // Working RGB to CIE 2006 LMS and back, and the output profile's, by rows.
    float4 to_lms[3];
    float4 from_lms[3];
    float4 export_to_lms[3];
    float4 export_from_lms[3];
    // The curve's coefficients (Curve), then its derivative along each of the four
    // gradient directions, seven vectors each.
    float4 curve[7];
    float4 directions[28];
};

struct Params
{
    PassHeader pass;
    FilmicParams filmic;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;
[[vk::binding(10, 0)]] RWStructuredBuffer<float> mask_buffer;
[[vk::binding(11, 0)]] RWStructuredBuffer<uint> counts;
[[vk::binding(12, 0)]] RWStructuredBuffer<float4> detail_buffer;
[[vk::binding(13, 0)]] RWStructuredBuffer<float4> low_buffer;
[[vk::binding(14, 0)]] RWStructuredBuffer<float4> temp_buffer;
[[vk::binding(15, 0)]] RWStructuredBuffer<float4> reconstructed;
[[vk::binding(16, 0)]] RWStructuredBuffer<float> norm_buffer;

// The per-pixel tone mapping's coefficients, as `Curve::vectors` in the setup lays them
// out: what the derivatives are taken against.
struct Curve : IDifferentiable
{
    // The toe's coefficients: for the hard and soft curves the Taylor coefficients of
    // d^0 to d^3 around its node (Spline::taylor), d = x - latitude_min; for the safe
    // curve darktable's a, b, c and its node's display value.
    float4 toe;
    // The shoulder's, likewise around latitude_max.
    float4 shoulder;
    // x and y the toe's and the shoulder's Taylor coefficients of d^4 (0 for the soft
    // and safe curves), z the line's offset, w its slope.
    float4 line;
    // x latitude_min, y latitude_max, z the grey source, w the black source (EV).
    float4 nodes;
    // x the dynamic range (EV), y the hardness, z the saturation coefficient, w sigma_toe.
    float4 tone;
    // x sigma_shoulder, y display black, z display white, w the norm's minimum.
    float4 display;
    // x the norm's maximum.
    float4 extra;
};

// What every pixel shares besides the curve.
struct Modes
{
    uint version;
    uint norm;
    uint toe;
    uint shoulder;
    // v3 to v5 with a custom grey source of 0: every pixel encodes to 1, darktable's
    // clamp of the log of x / 0.
    bool white;
    float3 luminance;
    float3x3 to_lms;
    float3x3 from_lms;
    float3x3 export_to_lms;
    float3x3 export_from_lms;
};

float3x3 matrix_of(float4 rows[3])
{
    return float3x3(rows[0].xyz, rows[1].xyz, rows[2].xyz);
}

Modes modes_of(FilmicParams p)
{
    Modes m;
    m.version = p.modes.x;
    m.norm = p.modes.y;
    m.toe = p.modes.z;
    m.shoulder = p.modes.w;
    m.white = p.flags.z != 0;
    m.luminance = p.luminance.xyz;
    m.to_lms = matrix_of(p.to_lms);
    m.from_lms = matrix_of(p.from_lms);
    m.export_to_lms = matrix_of(p.export_to_lms);
    m.export_from_lms = matrix_of(p.export_from_lms);
    return m;
}

Curve curve_at(FilmicParams p, uint first)
{
    Curve c;
    c.toe = first == 0 ? p.curve[0] : p.directions[first - 7];
    c.shoulder = first == 0 ? p.curve[1] : p.directions[first - 6];
    c.line = first == 0 ? p.curve[2] : p.directions[first - 5];
    c.nodes = first == 0 ? p.curve[3] : p.directions[first - 4];
    c.tone = first == 0 ? p.curve[4] : p.directions[first - 3];
    c.display = first == 0 ? p.curve[5] : p.directions[first - 2];
    c.extra = first == 0 ? p.curve[6] : p.directions[first - 1];
    return c;
}

// The curve's coefficients, and their derivative along direction k (0 to 3).
Curve curve_of(FilmicParams p)
{
    return curve_at(p, 0);
}

Curve direction_of(FilmicParams p, uint k)
{
    return curve_at(p, 7 * (k + 1));
}

// The curve and its four directions, for the derivative entry points.
struct Directions
{
    Curve base;
    Curve d0;
    Curve d1;
    Curve d2;
    Curve d3;
};

// The curve at t along the four directions: the derivative entry points differentiate
// against t at 0.
[Differentiable]
Curve along(no_diff Directions d, float4 t)
{
    Curve c;
    c.toe = d.base.toe + t.x * d.d0.toe + t.y * d.d1.toe + t.z * d.d2.toe + t.w * d.d3.toe;
    c.shoulder = d.base.shoulder + t.x * d.d0.shoulder + t.y * d.d1.shoulder
        + t.z * d.d2.shoulder + t.w * d.d3.shoulder;
    c.line = d.base.line + t.x * d.d0.line + t.y * d.d1.line + t.z * d.d2.line
        + t.w * d.d3.line;
    c.nodes = d.base.nodes + t.x * d.d0.nodes + t.y * d.d1.nodes + t.z * d.d2.nodes
        + t.w * d.d3.nodes;
    c.tone = d.base.tone + t.x * d.d0.tone + t.y * d.d1.tone + t.z * d.d2.tone
        + t.w * d.d3.tone;
    c.display = d.base.display + t.x * d.d0.display + t.y * d.d1.display
        + t.z * d.d2.display + t.w * d.d3.display;
    c.extra = d.base.extra + t.x * d.d0.extra + t.y * d.d1.extra + t.z * d.d2.extra
        + t.w * d.d3.extra;
    return c;
}

// darktable filmic_spline: the toe below latitude_min, the shoulder above latitude_max,
// the line between. The hard and soft pieces are darktable's polynomials evaluated
// around their nodes, in Horner's form of d = x - node, their coefficients the setup's
// Taylor coefficients (Spline::taylor), so no large terms cancel (docs/ports/
// filmicrgb.md, Deliberate differences); the soft piece's quartic term is 0. The safe
// pieces are darktable's rational functions of the distance from the node.
[Differentiable]
float spline(float x, Curve c, no_diff uint toe, no_diff uint shoulder)
{
    if (x < c.nodes.x)
    {
        if (toe == CURVE_SAFE)
        {
            float xi = c.nodes.x - x;
            float rat = xi * (xi * c.toe.y + 1.0);
            return c.toe.w - c.toe.x * rat / (rat + c.toe.z);
        }
        float d = x - c.nodes.x;
        return c.toe.x + d * (c.toe.y + d * (c.toe.z + d * (c.toe.w + d * c.line.x)));
    }
    if (x > c.nodes.y)
    {
        if (shoulder == CURVE_SAFE)
        {
            float xi = x - c.nodes.y;
            float rat = xi * (xi * c.shoulder.y + 1.0);
            return c.shoulder.w + c.shoulder.x * rat / (rat + c.shoulder.z);
        }
        float d = x - c.nodes.y;
        return c.shoulder.x
            + d * (c.shoulder.y + d * (c.shoulder.z + d * (c.shoulder.w + d * c.line.y)));
    }
    return c.line.z + x * c.line.w;
}

// The log encoding before its clamp: (log2(x / grey) - black) / range, for x > 0.
[Differentiable]
float log_encoding(float x, Curve c)
{
    return (log2(x / c.nodes.z) - c.nodes.w) / c.tone.x;
}

// darktable log_tonemapping_v1, clamped to [2^-16, 1], and log_tonemapping_v2, to [0, 1].
// `white`, for v3 to v5 with a custom grey source of 0, gives every pixel darktable's
// clamp of log2(x / 0), 1, without dividing by 0 (docs/ports/filmicrgb.md, Precision).
[Differentiable]
float log_v1(float x, Curve c, no_diff bool white)
{
    if (white)
        return 1.0;
    return clamp(log_encoding(x, c), NORM_MIN, 1.0);
}

[Differentiable]
float log_v2(float x, Curve c, no_diff bool white)
{
    if (white)
        return 1.0;
    return clamp(log_encoding(x, c), 0.0, 1.0);
}

// darktable filmic_desaturate_v1. A saturation coefficient of 0, darktable's -50 %,
// divides by a tiny value of its sign, where darktable divides by 0 and its clamp takes
// the infinity to 1; below 0 the clamp gives 0, as darktable's does.
[Differentiable]
float desaturate_v1(float x, Curve c)
{
    float key_toe = exp(-0.5 * x * x / max(c.tone.w, FLOAT_MIN));
    float key_shoulder = exp(-0.5 * (1.0 - x) * (1.0 - x) / max(c.display.x, FLOAT_MIN));
    return 1.0 - clamp((key_toe + key_shoulder) / away_from_zero(c.tone.z, 1e-30), 0.0, 1.0);
}

// darktable filmic_desaturate_v2, for a saturation coefficient of 0 and above: at 0,
// darktable's -50 %, its limit, where darktable's 0.5 / sqrt(0) is infinite. Below 0
// darktable's square root is NaN, and its callers render 0 (black_below_zero).
[Differentiable]
float desaturate_v2(float x, Curve c)
{
    float saturation = max(c.tone.z, 0.0);
    float sat2 = 0.5 / sqrt(max(saturation, 1e-30));
    float key_toe = exp(-x * x / max(c.tone.w, FLOAT_MIN) * sat2);
    float key_shoulder = exp(-(1.0 - x) * (1.0 - x) / max(c.display.x, FLOAT_MIN) * sat2);
    return saturation - (key_toe + key_shoulder) * saturation;
}

// darktable get_pixel_norm; "no" and any other falls back to luminance.
[Differentiable]
float pixel_norm(float3 c, no_diff uint norm, no_diff float3 luminance)
{
    if (norm == NORM_MAX_RGB)
        return max(max(c.x, c.y), c.z);
    if (norm == NORM_POWER)
    {
        float3 v = abs(c);
        float3 squared = v * v;
        float numerator = squared.x * v.x + squared.y * v.y + squared.z * v.z;
        float denominator = squared.x + squared.y + squared.z;
        return numerator / max(denominator, 1e-12);
    }
    if (norm == NORM_EUCLIDEAN_LEGACY)
        return sqrt(c.x * c.x + c.y * c.y + c.z * c.z);
    if (norm == NORM_EUCLIDEAN)
        return sqrt(c.x * c.x + c.y * c.y + c.z * c.z) * INVERSE_SQRT_3;
    return luminance.x * c.x + luminance.y * c.y + luminance.z * c.z;
}

// darktable dt_vector_log2: the exponent and a degree-5 fit of the mantissa. Its slope
// is the fit's, within a binade.
float log2_dt(float x)
{
    uint bits = asuint(x);
    float mantissa = asfloat((bits & 0x007FFFFFu) | 0x3F800000u);
    float exponent = float((bits & 0x7F800000u) >> 23) - 127.0;
    float fit = ((((0.0596515482674574969533 * mantissa - 0.465725644288844778798) * mantissa
        + 1.48116647521213171641) * mantissa - 2.52074962577807006663) * mantissa)
        + 2.8882704548164776201;
    return fit * (mantissa - 1.0) + exponent;
}

float log2_dt_slope(float x)
{
    if (x <= 0.0)
        return 0.0;
    uint bits = asuint(x);
    float mantissa = asfloat((bits & 0x007FFFFFu) | 0x3F800000u);
    float fit = ((((0.0596515482674574969533 * mantissa - 0.465725644288844778798) * mantissa
        + 1.48116647521213171641) * mantissa - 2.52074962577807006663) * mantissa)
        + 2.8882704548164776201;
    float fit_slope = (((4.0 * 0.0596515482674574969533 * mantissa
        - 3.0 * 0.465725644288844778798) * mantissa + 2.0 * 1.48116647521213171641) * mantissa)
        - 2.52074962577807006663;
    // The mantissa is x over its binade's power of two.
    return (fit_slope * (mantissa - 1.0) + fit) * (mantissa / x);
}

// roundf: half away from zero, darktable's dt_vector_round on arm64.
float round_away(float x)
{
    float t = trunc(x);
    if (abs(x - t) >= 0.5)
        t += x < 0.0 ? -1.0 : 1.0;
    return t;
}

// darktable dt_vector_exp2: 2 to x's integer part built in the exponent, after rounding
// x - 1/2, and a degree-4 fit of the fraction in [0, 1].
float exp2_dt(float y, out float slope)
{
    float x = clamp(y, -126.99999, 129.0);
    float ipart = round_away(x - 0.5);
    float fpart = x - ipart;
    float power = asfloat(uint(127 + int(ipart)) << 23);
    float fit = ((((1.3534167e-2 * fpart + 5.2011464e-2) * fpart + 2.4144275e-1) * fpart
        + 6.9300383e-1) * fpart) + 1.0000026;
    float fit_slope = (((4.0 * 1.3534167e-2 * fpart + 3.0 * 5.2011464e-2) * fpart
        + 2.0 * 2.4144275e-1) * fpart) + 6.9300383e-1;
    slope = (y > -126.99999 && y < 129.0) ? power * fit_slope : 0.0;
    return power * fit;
}

// darktable dt_vector_powf, exp2_dt(p · log2_dt(x)), with the derivatives of the
// approximation itself, so finite differences agree with them.
[ForwardDerivative(pow_dt_forward)]
[BackwardDerivative(pow_dt_backward)]
float pow_dt(float x, float p)
{
    float slope;
    return exp2_dt(log2_dt(x) * p, slope);
}

DifferentialPair<float> pow_dt_forward(DifferentialPair<float> x, DifferentialPair<float> p)
{
    float log = log2_dt(x.p);
    float slope;
    float value = exp2_dt(log * p.p, slope);
    return diffPair(value, slope * (p.p * log2_dt_slope(x.p) * x.d + log * p.d));
}

void pow_dt_backward(inout DifferentialPair<float> x, inout DifferentialPair<float> p, float d_out)
{
    float log = log2_dt(x.p);
    float slope;
    exp2_dt(log * p.p, slope);
    x = diffPair(x.p, d_out * slope * p.p * log2_dt_slope(x.p));
    p = diffPair(p.p, d_out * slope * log);
}

// v3 (2019) without a norm: darktable filmic_split_v1.
[Differentiable]
float3 split_v1(float3 rgb, Curve c, no_diff Modes m)
{
    float3 t = float3(log_v1(max(rgb.x, NORM_MIN), c, m.white),
        log_v1(max(rgb.y, NORM_MIN), c, m.white), log_v1(max(rgb.z, NORM_MIN), c, m.white));
    float lum = dot(m.luminance, t);
    float desaturation = desaturate_v1(lum, c);
    float3 mapped = lum + desaturation * (t - lum);
    float3 out = clamp(float3(spline(mapped.x, c, m.toe, m.shoulder),
        spline(mapped.y, c, m.toe, m.shoulder), spline(mapped.z, c, m.toe, m.shoulder)), 0.0, 1.0);
    return float3(pow_dt(out.x, c.tone.y), pow_dt(out.y, c.tone.y), pow_dt(out.z, c.tone.y));
}

// v4 (2020) and v5 (2021) without a norm: darktable filmic_split_v2_v3. With a saturation
// coefficient below 0, darktable's desaturation is NaN, which its clip of the curve's
// value takes to 0: black.
[Differentiable]
float3 split_v2_v3(float3 rgb, Curve c, no_diff Modes m)
{
    if (c.tone.z < 0.0)
        return float3(0.0);
    float3 t = float3(log_v2(max(rgb.x, NORM_MIN), c, m.white),
        log_v2(max(rgb.y, NORM_MIN), c, m.white), log_v2(max(rgb.z, NORM_MIN), c, m.white));
    float lum = dot(m.luminance, t);
    float desaturation = desaturate_v2(lum, c);
    float3 mapped = lum + desaturation * (t - lum);
    float3 out = clamp(float3(spline(mapped.x, c, m.toe, m.shoulder),
        spline(mapped.y, c, m.toe, m.shoulder), spline(mapped.z, c, m.toe, m.shoulder)), 0.0, 1.0);
    return float3(pow_dt(out.x, c.tone.y), pow_dt(out.y, c.tone.y), pow_dt(out.z, c.tone.y));
}

// The ratios of a pixel to its norm, less their minimum where it is negative.
[Differentiable]
float3 sanitised(float3 ratios)
{
    float least = min(min(ratios.x, ratios.y), ratios.z);
    return least < 0.0 ? ratios - least : ratios;
}

// v3 (2019) with a norm: darktable filmic_chroma_v1.
[Differentiable]
float3 chroma_v1(float3 rgb, Curve c, no_diff Modes m)
{
    float norm = max(pixel_norm(rgb, m.norm, m.luminance), NORM_MIN);
    float3 ratios = sanitised(rgb / norm);
    norm = log_v1(norm, c, m.white);
    float desaturation = desaturate_v1(norm, c);
    ratios = ratios * norm;
    float lum = dot(m.luminance, ratios);
    ratios = (lum + desaturation * (ratios - lum)) / norm;
    norm = pow0(clamp(spline(norm, c, m.toe, m.shoulder), 0.0, 1.0), c.tone.y);
    return ratios * norm;
}

// v4 (2020) and v5 (2021) with a norm: darktable filmic_chroma_v2_v3. With a saturation
// coefficient below 0, darktable's desaturation is NaN, which its MAX takes to ratios of
// 0: black.
[Differentiable]
float3 chroma_v2_v3(float3 rgb, Curve c, no_diff Modes m)
{
    if (c.tone.z < 0.0)
        return float3(0.0);
    float norm = max(pixel_norm(rgb, m.norm, m.luminance), NORM_MIN);
    float3 ratios = sanitised(rgb / norm);
    norm = log_v2(norm, c, m.white);
    float desaturation = desaturate_v2(norm, c);
    norm = pow0(clamp(spline(norm, c, m.toe, m.shoulder), 0.0, 1.0), c.tone.y);
    ratios = max(ratios + (1.0 - ratios) * (1.0 - desaturation), 0.0);
    if (m.version == V5_2021)
        norm = norm / max(pixel_norm(ratios, m.norm, m.luminance), NORM_MIN);
    float3 out = ratios * norm;
    float max_pix = max(max(out.x, out.y), out.z);
    if (max_pix > 1.0)
    {
        ratios = max(ratios + (1.0 - max_pix), 0.0);
        out = clamp(ratios * norm, 0.0, 1.0);
    }
    return out;
}

// darktable RGB_tone_mapping_v4, one channel: a channel at or below the norm's minimum,
// non-positive ones included, whose log darktable's clip takes to 0, encodes to 0.
[Differentiable]
float channel_tone_mapping(float x, Curve c, no_diff Modes m)
{
    float encoded = log_v2(max(x, 1e-30), c, false);
    float mapped = clamp(spline(encoded, c, m.toe, m.shoulder), 0.0, c.display.z);
    return pow_dt(mapped, c.tone.y);
}

[Differentiable]
float3 rgb_tone_mapping(float3 rgb, Curve c, no_diff Modes m)
{
    return float3(channel_tone_mapping(rgb.x, c, m), channel_tone_mapping(rgb.y, c, m),
        channel_tone_mapping(rgb.z, c, m));
}

// darktable norm_tone_mapping_v4: the curve on the norm, clamped to the encoding's range
// first; its value clamped to the display's bounds after the power, before the power,
// as darktable does.
[Differentiable]
float3 norm_tone_mapping(float3 rgb, Curve c, no_diff uint norm_type, no_diff Modes m)
{
    float norm = clamp(pixel_norm(rgb, norm_type, m.luminance), c.display.w, c.extra.x);
    float3 ratios = rgb / norm;
    norm = log_v2(norm, c, false);
    norm = pow0(clamp(spline(norm, c, m.toe, m.shoulder), c.display.y, c.display.z), c.tone.y);
    return ratios * norm;
}

// gamut_mapping.h RGB_to_Ych and Ych_to_RGB, through CIE 2006 LMS and Yrg. Ych's chroma
// and hue do not depend on the pixel's scale and its Y is linear in it, so the pixel is
// converted scaled to its largest channel, the scale held constant: the same values
// within rounding and exactly the same derivatives, where Yrg's normalisation of a
// tiny pixel, 1e-20 and below, would square its sum past f32's range in the backward
// pass.
[Differentiable]
float4 rgb_to_ych(float3 rgb, no_diff float3x3 to_lms)
{
    float scale = detach(max(max(abs(rgb.x), abs(rgb.y)), abs(rgb.z)));
    if (scale <= 0.0)
        return yrg_to_ych(lms_to_yrg(mul(to_lms, rgb)));
    float4 ych = yrg_to_ych(lms_to_yrg(mul(to_lms, rgb / scale)));
    return float4(ych.x * scale, ych.y, ych.z, ych.w);
}

[Differentiable]
float3 ych_to_rgb(float4 ych, no_diff float3x3 from_lms)
{
    return mul(from_lms, yrg_to_lms(ych_to_yrg(ych)));
}

// gamut_mapping.h: the denominator's luminance term of a channel's chroma limit.
[Differentiable]
float chroma_denominator(no_diff float3 coeffs, float cos_h, float sin_h)
{
    return coeffs.x * (0.979381443298969 * cos_h + 0.391752577319588 * sin_h)
        + coeffs.y * (0.0206185567010309 * cos_h + 0.608247422680412 * sin_h)
        - coeffs.z * (cos_h + sin_h);
}

// _clip_chroma_white_raw: the chroma that takes the channel of `coeffs`, a row of LMS to
// output RGB, to the target white at luminance y.
[Differentiable]
float clip_chroma_white_raw(no_diff float3 coeffs, float target_white, float y, float cos_h,
    float sin_h)
{
    float denominator_y = chroma_denominator(coeffs, cos_h, sin_h);
    float denominator_target = target_white * (0.68285981628866 * cos_h + 0.482137060515464 * sin_h);
    if (denominator_y == 0.0)
        return FLOAT_MAX;
    float y_asymptote = denominator_target / denominator_y;
    if (y <= y_asymptote)
        return FLOAT_MAX;
    float denominator = y * denominator_y - denominator_target;
    float numerator = -0.427506877216495
        * (y * (coeffs.x + 0.856492345150334 * coeffs.y + 0.554995960637719 * coeffs.z)
            - 0.988237752433297 * target_white);
    return numerator / denominator;
}

// _clip_chroma_white: within 1e-3 of the white the limit falls linearly to 0. A limit
// absent there stays absent, where darktable scales FLT_MAX.
[Differentiable]
float clip_chroma_white(no_diff float3 coeffs, float target_white, float y, float cos_h,
    float sin_h)
{
    float eps = 1e-3;
    float max_y = Y_2006 * target_white;
    float delta_y = max(max_y - y, 0.0);
    float max_chroma;
    if (delta_y < eps)
    {
        float raw = clip_chroma_white_raw(coeffs, target_white, (1.0 - eps) * max_y, cos_h, sin_h);
        if (raw >= FLOAT_MAX)
            return FLOAT_MAX;
        max_chroma = delta_y / (eps * max_y) * raw;
    }
    else
        max_chroma = clip_chroma_white_raw(coeffs, target_white, y, cos_h, sin_h);
    return max_chroma >= 0.0 ? max_chroma : FLOAT_MAX;
}

// _clip_chroma_black: the chroma that takes the channel to 0.
[Differentiable]
float clip_chroma_black(no_diff float3 coeffs, float cos_h, float sin_h)
{
    float denominator = chroma_denominator(coeffs, cos_h, sin_h);
    if (denominator == 0.0)
        return FLOAT_MAX;
    float numerator = -0.427506877216495
        * (coeffs.x + 0.856492345150334 * coeffs.y + 0.554995960637719 * coeffs.z);
    float max_chroma = numerator / denominator;
    return max_chroma >= 0.0 ? max_chroma : FLOAT_MAX;
}

// Ych_max_chroma: the most chroma every channel of the output RGB allows.
[Differentiable]
float ych_max_chroma(no_diff float3x3 from_lms, float target_white, float y, float cos_h,
    float sin_h)
{
    float white = min(min(clip_chroma_white(from_lms[0], target_white, y, cos_h, sin_h),
        clip_chroma_white(from_lms[1], target_white, y, cos_h, sin_h)),
        clip_chroma_white(from_lms[2], target_white, y, cos_h, sin_h));
    float black = min(min(clip_chroma_black(from_lms[0], cos_h, sin_h),
        clip_chroma_black(from_lms[1], cos_h, sin_h)), clip_chroma_black(from_lms[2], cos_h, sin_h));
    return min(black, white);
}

// darktable gamut_check_RGB, in the RGB of `to_lms` and `from_lms`.
[Differentiable]
float3 gamut_check_rgb(float4 ych, no_diff float3x3 to_lms, no_diff float3x3 from_lms,
    float display_black, float display_white)
{
    float3 brightened = ych_to_rgb(ych, from_lms);
    float least = min(min(brightened.x, brightened.y), brightened.z);
    brightened = brightened + max(-least, 0.0);
    float4 ych_brightened = rgb_to_ych(brightened, to_lms);
    float y = clamp((ych.x + ych_brightened.x) / 2.0, Y_2006 * display_black,
        Y_2006 * display_white);
    float cos_h = ych.z;
    float sin_h = ych.w;
    float chroma = min(ych.y, ych_max_chroma(from_lms, display_white, y, cos_h, sin_h));
    float3 rgb = ych_to_rgb(float4(y, chroma, cos_h, sin_h), from_lms);
    return clamp(rgb, 0.0, display_white);
}

// darktable filmic_desaturate_v4: the output's chroma moved towards the input's by the
// saturation.
[Differentiable]
float4 desaturate_v4(float4 original, float4 final, float saturation)
{
    float chroma_original = original.y * original.x;
    float chroma_final = final.y * final.x;
    float delta = saturation * (chroma_original - chroma_final);
    bool brightens = final.x > original.x;
    bool resaturates = chroma_original < chroma_final;
    bool desaturates = chroma_original > chroma_final;
    float chroma = chroma_final;
    if (brightens && resaturates)
        chroma = (chroma_original + chroma_final) / 2.0;
    else if ((saturation > 0.0 && desaturates) || saturation < 0.0)
        chroma = chroma_final + delta;
    // darktable's MAX of a NaN at Y = 0 is 0.
    float c = final.x > 0.0 ? max(chroma / final.x, 0.0) : 0.0;
    return float4(final.x, c, final.z, final.w);
}

// darktable gamut_mapping, against the output profile, then back to working RGB.
[Differentiable]
float3 gamut_mapping(float4 final, float4 original, Curve c, float saturation, no_diff Modes m)
{
    float y = clamp(final.x, Y_2006 * c.display.y, Y_2006 * c.display.z);
    float4 ych = desaturate_v4(original, float4(y, final.y, original.z, original.w), saturation);
    ych = gamut_check_yrg(ych);
    float3 rgb = gamut_check_rgb(ych, m.export_to_lms, m.export_from_lms, c.display.y, c.display.z);
    return mul(m.from_lms, mul(m.export_to_lms, rgb));
}

// v6 (2022): darktable filmic_split_v4 without a norm, filmic_chroma_v4 with one.
[Differentiable]
float3 filmic_v6(float3 rgb, Curve c, no_diff Modes m)
{
    float4 original = rgb_to_ych(rgb, m.to_lms);
    if (m.norm == NORM_NO)
    {
        float4 final = rgb_to_ych(rgb_tone_mapping(rgb, c, m), m.to_lms);
        final.y = min(original.y, final.y);
        return gamut_mapping(final, original, c, c.tone.z, m);
    }
    float4 final = rgb_to_ych(norm_tone_mapping(rgb, c, m.norm, m), m.to_lms);
    return gamut_mapping(final, original, c, c.tone.z, m);
}

// v7 (2023), darktable's default: darktable filmic_v5, the per-channel tone mapping and
// the max RGB norm's mixed by the saturation, whatever the norm chosen.
[Differentiable]
float3 filmic_v7(float3 rgb, Curve c, no_diff Modes m)
{
    float3 naive = rgb_tone_mapping(rgb, c, m);
    float3 max_rgb = norm_tone_mapping(rgb, c, NORM_MAX_RGB, m);
    float s = c.tone.z;
    float3 mixed = (0.5 - s) * naive + (0.5 + s) * max_rgb;
    float4 original = rgb_to_ych(rgb, m.to_lms);
    float4 final = rgb_to_ych(mixed, m.to_lms);
    final.y = min(original.y, final.y);
    return gamut_mapping(final, original, c, 0.0, m);
}

// The per-pixel function: darktable's process after the reconstruction, by colour
// science and norm.
[Differentiable]
float3 filmic(no_diff float3 rgb, Curve c, no_diff Modes m)
{
    if (m.version == V7_2023)
        return filmic_v7(rgb, c, m);
    if (m.version == V6_2022)
        return filmic_v6(rgb, c, m);
    if (m.norm == NORM_NO)
    {
        if (m.version == V3_2019)
            return split_v1(rgb, c, m);
        return split_v2_v3(rgb, c, m);
    }
    if (m.version == V3_2019)
        return chroma_v1(rgb, c, m);
    return chroma_v2_v3(rgb, c, m);
}

[Differentiable]
float3 filmic_along(no_diff float3 rgb, float4 t, no_diff Directions d, no_diff Modes m)
{
    return filmic(rgb, along(d, t), m);
}

// What the tone mapping reads at (x, y): the reconstructed image where the
// reconstruction ran and more than 9 pixels clipped, the input otherwise.
float3 tone_input(uint i, uint x, uint y, FilmicParams p)
{
    float3 rgb = input[i].rgb;
    if (p.flags.x != 0 && counts[0] > 9)
        rgb = reconstructed[y * p.image.x + x].rgb;
    return rgb;
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    FilmicParams p = params.filmic;
    uint i = id.y * params.pass.width + id.x;
    uint x = params.pass.origin_x + id.x;
    uint y = params.pass.origin_y + id.y;
    Curve c = curve_of(p);
    Modes m = modes_of(p);
    float3 rgb = tone_input(i, x, y, p);
    for (uint k = 0; k < params.pass.repeat; k++)
        rgb = filmic(rgb, c, m);
    output[i] = float4(rgb, input[i].a);
}

// Derivative entry points of `main`, per pixel, over the scratch the forward render
// left (decision 15), with respect to t along the four gradient directions, in this
// order: contrast, output_power (the hardness), latitude and saturation, the last two
// as fractions (DIRECTIONS in the setup; docs/ports/filmicrgb.md, Gradients). They bind
// what `main` binds but its output.
[[vk::binding(8, 0)]] StructuredBuffer<float4> derivative_in;
[[vk::binding(9, 0)]] RWStructuredBuffer<float4> derivative_out;

Directions directions_of(FilmicParams p)
{
    Directions d;
    d.base = curve_of(p);
    d.d0 = direction_of(p, 0);
    d.d1 = direction_of(p, 1);
    d.d2 = direction_of(p, 2);
    d.d3 = direction_of(p, 3);
    return d;
}

// Reverse mode: given dLoss/dOutput, the gradient of the loss along each direction.
[shader("compute")]
[numthreads(16, 16, 1)]
void filmicrgb_backward(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    FilmicParams p = params.filmic;
    uint i = id.y * params.pass.width + id.x;
    float3 rgb = tone_input(i, params.pass.origin_x + id.x, params.pass.origin_y + id.y, p);
    var t = diffPair(float4(0.0));
    bwd_diff(filmic_along)(rgb, t, directions_of(p), modes_of(p), derivative_in[i].rgb);
    derivative_out[i] = t.d;
}

// Forward mode: given a direction in t, the directional derivative of the output.
// Checks the reverse mode.
[shader("compute")]
[numthreads(16, 16, 1)]
void filmicrgb_forward_diff(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    FilmicParams p = params.filmic;
    uint i = id.y * params.pass.width + id.x;
    float3 rgb = tone_input(i, params.pass.origin_x + id.x, params.pass.origin_y + id.y, p);
    let result = fwd_diff(filmic_along)(rgb, diffPair(float4(0.0), derivative_in[i]),
        directions_of(p), modes_of(p));
    derivative_out[i] = float4(result.d, 0.0);
}

// ---- Highlight reconstruction: whole-image passes over the image at the pipe's scale.

static const uint WORKGROUP = 256;
groupshared uint shared_counts[WORKGROUP];

// darktable mask_clipped_pixels: the mask's opacity, and whether the pixel counts as
// clipped, each workgroup's count written as its partial. The exponent is held within
// ±126, where darktable's flush to zero takes 1 / (1 + 2^x) to 0 beyond it anyway.
[shader("compute")]
[numthreads(16, 16, 1)]
void hr_mask(uint3 id: SV_DispatchThreadID, uint3 group: SV_GroupID, uint t: SV_GroupIndex)
{
    FilmicParams p = params.filmic;
    uint clipped = 0;
    if (id.x < p.image.x && id.y < p.image.y)
    {
        uint i = id.y * p.image.x + id.x;
        float3 c = input[i].rgb;
        float norm = sqrt(c.x * c.x + c.y * c.y + c.z * c.z);
        float argument = -norm * p.mask.x + p.mask.y;
        mask_buffer[i] = clamp(1.0 / (1.0 + exp2(clamp(argument, -126.0, 126.0))), 0.0, 1.0);
        clipped = 4.0 > argument ? 1 : 0;
    }
    shared_counts[t] = clipped;
    GroupMemoryBarrierWithGroupSync();
    for (uint stride = WORKGROUP / 2; stride > 0; stride /= 2)
    {
        if (t < stride)
            shared_counts[t] += shared_counts[t + stride];
        GroupMemoryBarrierWithGroupSync();
    }
    if (t == 0)
        counts[1 + group.y * p.image.z + group.x] = shared_counts[0];
}

// One workgroup: the workgroups' counts added up into the total.
[shader("compute")]
[numthreads(16, 16, 1)]
void hr_count(uint t: SV_GroupIndex)
{
    uint partials = params.filmic.flags.y;
    uint sum = 0;
    for (uint i = t; i < partials; i += WORKGROUP)
        sum += counts[1 + i];
    shared_counts[t] = sum;
    GroupMemoryBarrierWithGroupSync();
    for (uint stride = WORKGROUP / 2; stride > 0; stride /= 2)
    {
        if (t < stride)
            shared_counts[t] += shared_counts[t + stride];
        GroupMemoryBarrierWithGroupSync();
    }
    if (t == 0)
        counts[0] = shared_counts[0];
}

// a · b as 64 bits, (high, low), from 16-bit halves.
uint2 mul_wide(uint a, uint b)
{
    uint a_lo = a & 0xFFFFu;
    uint a_hi = a >> 16;
    uint b_lo = b & 0xFFFFu;
    uint b_hi = b >> 16;
    uint lo_lo = a_lo * b_lo;
    uint t = a_hi * b_lo + (lo_lo >> 16);
    uint w1 = t & 0xFFFFu;
    uint w2 = t >> 16;
    t = a_lo * b_hi + w1;
    return uint2(a_hi * b_hi + w2 + (t >> 16), (t << 16) | (lo_lo & 0xFFFFu));
}

// a · b modulo 2^64, each (high, low).
uint2 mul64(uint2 a, uint2 b)
{
    uint2 low = mul_wide(a.y, b.y);
    return uint2(low.x + a.x * b.y + a.y * b.x, low.y);
}

// darktable splitmix32 of a 64-bit seed (high, low).
uint splitmix32(uint2 seed)
{
    uint2 r = uint2(seed.x, seed.y ^ (seed.x >> 1));
    r = mul64(r, uint2(0x62a9d9edu, 0x799705f5u));
    r = uint2(r.x ^ (r.x >> 28), r.y ^ ((r.y >> 28) | (r.x << 4)));
    r = mul64(r, uint2(0xcb24d0a5u, 0xc88c35b3u));
    return r.x;
}

// darktable xoshiro128plus: a 24-bit uniform draw in [0, 1).
float xoshiro128plus(inout uint4 s)
{
    uint result = s.x + s.w;
    uint t = s.y << 9;
    s.z ^= s.x;
    s.w ^= s.y;
    s.y ^= s.z;
    s.x ^= s.w;
    s.z ^= t;
    s.w = (s.w << 11) | (s.w >> 21);
    return float(result >> 8) * asfloat(0x33800000u);
}

// The generator's state for row i and column j, as inpaint_noise seeds and advances it.
uint4 noise_state(uint i, uint j)
{
    uint4 s = uint4(splitmix32(uint2(0, j + 1)), splitmix32(mul_wide(j + 1, i + 3)),
        splitmix32(uint2(0, 1337)), splitmix32(uint2(0, 666)));
    xoshiro128plus(s);
    xoshiro128plus(s);
    xoshiro128plus(s);
    xoshiro128plus(s);
    return s;
}

// Box-Muller's normal draw from u1 and u2, cosine or sine.
float normal(float u1, float u2, bool flip)
{
    float radius = sqrt(-2.0 * log(u1));
    return flip ? radius * cos(TWO_PI * u2) : radius * sin(TWO_PI * u2);
}

// darktable dt_noise_generator_simd around mu with standard deviation sigma; red and
// blue take the cosine, green the sine.
float3 noise(uint distribution, float3 mu, float3 sigma, inout uint4 s)
{
    if (distribution == NOISE_GAUSSIAN || distribution == NOISE_POISSONIAN)
    {
        float3 u1;
        float3 u2;
        if (distribution == NOISE_GAUSSIAN)
        {
            u1 = float3(max(xoshiro128plus(s), FLOAT_MIN), max(xoshiro128plus(s), FLOAT_MIN),
                max(xoshiro128plus(s), FLOAT_MIN));
            u2 = float3(xoshiro128plus(s), xoshiro128plus(s), xoshiro128plus(s));
        }
        else
        {
            u1.x = max(xoshiro128plus(s), FLOAT_MIN);
            u2.x = xoshiro128plus(s);
            u1.y = max(xoshiro128plus(s), FLOAT_MIN);
            u2.y = xoshiro128plus(s);
            u1.z = max(xoshiro128plus(s), FLOAT_MIN);
            u2.z = xoshiro128plus(s);
        }
        float3 n = float3(normal(u1.x, u2.x, true), normal(u1.y, u2.y, false),
            normal(u1.z, u2.z, true));
        if (distribution == NOISE_GAUSSIAN)
            return n * sigma + mu;
        // Anscombe's transform of the gaussian noise.
        float3 r = n * sigma + 2.0 * sqrt(max(mu + 3.0 / 8.0, 0.0));
        return (r * r - sigma * sigma) / 4.0 - 3.0 / 8.0;
    }
    float3 u = float3(xoshiro128plus(s), xoshiro128plus(s), xoshiro128plus(s));
    return mu + 2.0 * (u - 0.5) * sigma;
}

// darktable inpaint_noise: noise in the masked areas.
[shader("compute")]
[numthreads(16, 16, 1)]
void hr_inpaint(uint3 id: SV_DispatchThreadID)
{
    FilmicParams p = params.filmic;
    if (id.x >= p.image.x || id.y >= p.image.y)
        return;
    uint i = id.y * p.image.x + id.x;
    uint4 s = noise_state(id.y, id.x);
    float4 c = input[i];
    float w = mask_buffer[i];
    float3 sigma = c.rgb * p.mask.z / p.mask.w;
    float3 n = noise(p.wavelet.w, c.rgb, sigma, s);
    detail_buffer[i] = float4(max(c.rgb * (1.0 - w) + w * n, 0.0), c.a);
}

// darktable init_reconstruct: the valid parts of the source.
[shader("compute")]
[numthreads(16, 16, 1)]
void hr_init(uint3 id: SV_DispatchThreadID)
{
    FilmicParams p = params.filmic;
    if (id.x >= p.image.x || id.y >= p.image.y)
        return;
    uint i = id.y * p.image.x + id.x;
    reconstructed[i] = max(detail_buffer[i] * (1.0 - mask_buffer[i]), 0.0);
}

// The B-spline [1 4 6 4 1] / 16 over five taps, clipped at 0 (sparse_scalar_product).
float4 bspline(float4 a, float4 b, float4 c, float4 d, float4 e)
{
    return max(0.0, (1.0 / 16.0) * a + (4.0 / 16.0) * b + (6.0 / 16.0) * c + (4.0 / 16.0) * d
        + (1.0 / 16.0) * e);
}

// _bspline_vertical_pass: the detail blurred along columns at a step of image.w, rows
// clamped to the image.
[shader("compute")]
[numthreads(16, 16, 1)]
void hr_blur_v(uint3 id: SV_DispatchThreadID)
{
    FilmicParams p = params.filmic;
    uint width = p.image.x;
    uint height = p.image.y;
    if (id.x >= width || id.y >= height)
        return;
    int row = int(id.y);
    int mult = int(p.image.w);
    int last = int(height) - 1;
    uint x = id.x;
    temp_buffer[id.y * width + x] = bspline(
        detail_buffer[uint(max(row - 2 * mult, 0)) * width + x],
        detail_buffer[uint(max(row - mult, 0)) * width + x],
        detail_buffer[uint(row) * width + x],
        detail_buffer[uint(min(row + mult, last)) * width + x],
        detail_buffer[uint(min(row + 2 * mult, last)) * width + x]);
}

// The vertical blur blurred along rows at a step of image.w, columns clamped.
float4 horizontal(uint x, uint y, int mult, uint width)
{
    int col = int(x);
    int last = int(width) - 1;
    uint row = y * width;
    return bspline(temp_buffer[row + uint(max(col - 2 * mult, 0))],
        temp_buffer[row + uint(max(col - mult, 0))], temp_buffer[row + x],
        temp_buffer[row + uint(min(col + mult, last))],
        temp_buffer[row + uint(min(col + 2 * mult, last))]);
}

// _bspline_horizontal, then the high frequencies: the low frequencies, and the detail
// less them in its place.
[shader("compute")]
[numthreads(16, 16, 1)]
void hr_blur_h(uint3 id: SV_DispatchThreadID)
{
    FilmicParams p = params.filmic;
    if (id.x >= p.image.x || id.y >= p.image.y)
        return;
    uint i = id.y * p.image.x + id.x;
    float4 low = horizontal(id.x, id.y, int(p.image.w), p.image.x);
    low_buffer[i] = low;
    detail_buffer[i] = detail_buffer[i] - low;
}

// fmaxabsf: the larger in magnitude, with its sign.
float max_abs(float a, float b)
{
    return abs(a) > abs(b) ? a : b;
}

// wavelets_reconstruct_RGB and _ratios: the high frequencies' horizontal blur at a step
// of 1, HF_RGB, and the scale's share added to the reconstruction.
[shader("compute")]
[numthreads(16, 16, 1)]
void hr_reconstruct(uint3 id: SV_DispatchThreadID)
{
    FilmicParams p = params.filmic;
    if (id.x >= p.image.x || id.y >= p.image.y)
        return;
    uint i = id.y * p.image.x + id.x;
    float3 hf = horizontal(id.x, id.y, 1, p.image.x).rgb;
    float3 lf = low_buffer[i].rgb;
    float3 texture = detail_buffer[i].rgb;
    float alpha = mask_buffer[i];
    float gamma = p.balance.x;
    float gamma_comp = 1.0 - gamma;
    float beta = p.balance.y;
    float beta_comp = 1.0 - beta;
    float delta = p.balance.z;
    bool last = p.wavelet.x == p.wavelet.y;
    float grey_texture = max_abs(max_abs(texture.x, texture.y), texture.z);
    float grey_details = (hf.x + hf.y + hf.z) / 3.0;
    float3 details;
    float3 residual = float3(0.0);
    if (p.wavelet.z == VARIANT_RGB)
    {
        float grey_hf = beta_comp * (gamma_comp * grey_details + gamma * grey_texture);
        details = (gamma_comp * hf + gamma * texture) * beta + grey_hf;
        if (last)
            residual = beta_comp * (lf.x + lf.y + lf.z) / 3.0 + lf * beta;
    }
    else
    {
        float grey_hf = gamma_comp * grey_details + gamma * grey_texture;
        details = 0.5 * ((gamma_comp * hf + gamma * texture) + grey_hf);
        if (last)
            residual = lf;
    }
    float4 r = reconstructed[i];
    reconstructed[i] = float4(r.rgb + alpha * (delta * details + residual), r.a);
}

// compute_ratios: the reconstruction's euclidean norm and its ratios to it.
[shader("compute")]
[numthreads(16, 16, 1)]
void hr_ratios(uint3 id: SV_DispatchThreadID)
{
    FilmicParams p = params.filmic;
    if (id.x >= p.image.x || id.y >= p.image.y)
        return;
    uint i = id.y * p.image.x + id.x;
    float4 c = reconstructed[i];
    float norm = max(sqrt(c.x * c.x + c.y * c.y + c.z * c.z), NORM_MIN);
    norm_buffer[i] = norm;
    detail_buffer[i] = c / norm;
}

// restore_ratios: the reconstructed ratios, clamped to [0, 1], times their norm.
[shader("compute")]
[numthreads(16, 16, 1)]
void hr_restore(uint3 id: SV_DispatchThreadID)
{
    FilmicParams p = params.filmic;
    if (id.x >= p.image.x || id.y >= p.image.y)
        return;
    uint i = id.y * p.image.x + id.x;
    reconstructed[i] = clamp(reconstructed[i], 0.0, 1.0) * norm_buffer[i];
}
