// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2020-2026  darktable developers (ported from src/iop/colorbalancergb.c)
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

// Color balance rgb, ported from darktable src/iop/colorbalancergb.c, process(), with
// both saturation formulas, JzAzBz and darktable UCS. See docs/ports/colorbalancergb.md;
// the setup, computed once per image on the CPU, is `prepare` in src/setup.rs.

import color;
import pass;

struct ColorBalanceParams
{
    // Working RGB to LMS 2006 D65, and XYZ D65 to working RGB, by rows.
    float4 input_r0;
    float4 input_r1;
    float4 input_r2;
    float4 output_r0;
    float4 output_r1;
    float4 output_r2;
    // Grading RGB: global offset, shadows and highlights slopes, midtones power.
    float4 global;
    float4 shadows;
    float4 highlights;
    float4 midtones;
    // Per tonal range (shadows, midtones, highlights).
    float4 chroma;
    float4 saturation;
    float4 brilliance;
    float chroma_global;
    float vibrance;
    float contrast;
    float saturation_global;
    float brilliance_global;
    float hue_cos;
    float hue_sin;
    float shadows_weight;
    float highlights_weight;
    float midtones_weight;
    float mask_grey_fulcrum;
    float white_fulcrum;
    float grey_fulcrum;
    float midtones_y;
    float l_white;
    uint saturation_formula;
};

struct Params
{
    PassHeader pass;
    ColorBalanceParams color_balance;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;
// The working gamut's boundary per hue, 512 entries over [-π, π): the largest JzAzBz
// saturation Cz / Jz, or with darktable UCS the boundary's colourfulness squared, M².
[[vk::binding(3, 0)]] StructuredBuffer<float> gamut_lut;

static const uint LUT_SIZE = 512;

// `saturation_formula`: darktable's dt_iop_colorbalancrgb_saturation_t.
static const uint SATURATION_JZAZBZ = 0;

// The smallest chroma the darktable UCS path's saturation divides by, where darktable's
// is FLT_MIN: small enough that no visible chroma moves, large enough that the ratio
// it bounds and its derivative, which divides by its square, stay finite in fp32 for
// every brightness a grey reaches within darktable's ranges (at most about 1265).
static const float UCS_SMALLEST_CHROMA = 1e-15;

// Linear interpolation between the two entries around the hue, wrapping around. Unlike
// darktable's, it has no branch for a hue on an entry, where the weight is 0 anyway:
// Slang's backward pass recomputes such a branch's condition rather than saving it, and
// when its recomputed hue lands on the other side of an entry, it reads a neighbour the
// forward pass never saved.
[Differentiable]
float lookup_gamut(float hue)
{
    float x = float(LUT_SIZE) * (hue + PI) / (2.0 * PI);
    float x_prev = no_diff floor(x);
    uint i = asuint(int(x_prev)) & (LUT_SIZE - 1);
    uint j = (i + 1) & (LUT_SIZE - 1);
    float y = gamut_lut[i];
    return y + (x - x_prev) * (gamut_lut[j] - y);
}

// The parameters the backward pass differentiates against. The per-pixel function
// reads them from here rather than from ColorBalanceParams, which is a constant.
struct Grading : IDifferentiable
{
    float chroma_global;
    float contrast;
    float saturation_global;
    float brilliance_global;
};

Grading grading(ColorBalanceParams p)
{
    Grading g;
    g.chroma_global = p.chroma_global;
    g.contrast = p.contrast;
    g.saturation_global = p.saturation_global;
    g.brilliance_global = p.brilliance_global;
    return g;
}

// 1 / (1 + e^t), with t clamped so e^t stays finite.
[Differentiable]
float logistic(float t)
{
    return 1.0 / (1.0 + exp(clamp(t, -80.0, 80.0)));
}

// The largest power of two `bounded_pow` returns: far beyond JzAzBz's pole, Y about
// 8.7e5 or 2^19.7, past which darktable's output is degenerate too, and small enough that
// the derivatives' squares of such values stay finite in fp32 (Yrg's normalisation
// divides by its sum squared).
static const float POWER_LIMIT = 32.0;

// x^y for x > 0, else 0, as pow0, but never above 2^POWER_LIMIT: the exponent is taken
// in the log domain and capped there, so the result stays finite for any finite x and
// y. The settings' powers need it: within darktable's ranges the midtones wheel and
// midtones_Y give exponents up to ±10^6, where darktable's powf overflows to infinity.
[Differentiable]
float bounded_pow(float x, float y)
{
    if (x <= 0.0)
        return 0.0;
    return exp2(min(y * log2(x), POWER_LIMIT));
}

[Differentiable]
float3 bounded_pow(float3 x, float3 y)
{
    return float3(bounded_pow(x.x, y.x), bounded_pow(x.y, y.y), bounded_pow(x.z, y.z));
}

// Opacities of the shadows, midtones and highlights masks at luminance mask value x.
[Differentiable]
float3 opacity_masks(float x, no_diff ColorBalanceParams p)
{
    float offset = x - p.mask_grey_fulcrum;
    float norm = offset / p.mask_grey_fulcrum;
    float shadows = logistic(norm * p.shadows_weight);
    float highlights = logistic(-norm * p.highlights_weight);
    float a = 1.0 - shadows;
    float b = 1.0 - highlights;
    float midtones = exp(-offset * offset * p.midtones_weight / 4.0) * a * a * b * b * 8.0;
    return float3(shadows, midtones, highlights);
}

// The per-pixel function.
[Differentiable]
float3 color_balance(no_diff float3 rgb_in, no_diff ColorBalanceParams p, Grading g)
{
    float3x3 input_matrix = float3x3(p.input_r0.xyz, p.input_r1.xyz, p.input_r2.xyz);
    float3x3 output_matrix = float3x3(p.output_r0.xyz, p.output_r1.xyz, p.output_r2.xyz);

    // Pipeline RGB to Ych, no negative values.
    float4 ych = yrg_to_ych(lms_to_yrg(mul(input_matrix, max(rgb_in, float3(0.0)))));
    ych.x = max(ych.x, 0.0);
    float3 masks = opacity_masks(pow0(ych.x, 0.4101205819200422), p);

    // Hue shift.
    ych = float4(
        ych.x,
        ych.y,
        p.hue_cos * ych.z - p.hue_sin * ych.w,
        p.hue_sin * ych.z + p.hue_cos * ych.w);

    // Linear chroma and vibrance, then clip to the Yrg gamut.
    float vibrance = 0.0;
    if (p.vibrance != 0.0)
        vibrance = p.vibrance * (1.0 - pow0(ych.y, abs(p.vibrance)));
    float chroma_factor = 1.0 + g.chroma_global + dot(masks, p.chroma.xyz) + vibrance;
    ych.y *= max(chroma_factor, 0.0);
    ych = gamut_check_yrg(ych);

    // Colour balance in grading RGB: offset, slopes, then power around the white fulcrum.
    float3 rgb = mul(LMS_TO_GRADING_RGB, yrg_to_lms(ych_to_yrg(ych)));
    rgb += p.global.xyz;
    rgb *= (1.0 - masks.z) * ((1.0 - masks.x) + masks.x * p.shadows.xyz)
        + masks.z * p.highlights.xyz;
    float w = p.white_fulcrum;
    float3 sign = float3(rgb.x < 0.0 ? -1.0 : 1.0, rgb.y < 0.0 ? -1.0 : 1.0, rgb.z < 0.0 ? -1.0 : 1.0);
    rgb = sign * bounded_pow(abs(rgb) / w, p.midtones.xyz) * w;

    // Luminance gamma around the white fulcrum, then contrast around the grey fulcrum.
    float3 yrg = lms_to_yrg(mul(GRADING_RGB_TO_LMS, rgb));
    yrg.x = bounded_pow(max(yrg.x / w, 0.0), p.midtones_y) * w;
    yrg.x = p.grey_fulcrum * bounded_pow(yrg.x / p.grey_fulcrum, g.contrast);
    float3 xyz = mul(LMS_2006_TO_XYZ_D65, yrg_to_lms(yrg));

    // Perceptual brilliance and saturation, then gamut mapping, in the formula's space.
    if (p.saturation_formula == SATURATION_JZAZBZ)
        xyz = jzazbz_saturation(xyz, masks, p, g);
    else
        xyz = ucs_saturation(xyz, masks, p, g);
    return max(mul(output_matrix, xyz), float3(0.0));
}

// Brilliance, saturation and gamut mapping in JzAzBz, from and to XYZ D65.
[Differentiable]
float3 jzazbz_saturation(float3 xyz, float3 masks, no_diff ColorBalanceParams p, Grading g)
{
    // Brilliance and saturation: rotate (Jz, Cz) onto the saturation axis.
    float3 jab = xyz_to_jzazbz(xyz);
    float2 jc = float2(jab.x, length(jab.yz));
    float h = atan2(jab.z, jab.y);
    float t = atan2(jc.y, jc.x);
    float sin_t = sin(t);
    float cos_t = cos(t);
    float brilliance = 1.0 + g.brilliance_global + dot(masks, p.brilliance.xyz);
    float saturation = g.saturation_global + dot(masks, p.saturation.xyz);
    float s = jc.x * cos_t + jc.y * sin_t;
    float o = s * min(max(t * saturation, -t), HALF_PI - t);
    s = max(s * brilliance, 0.0);
    jc = max(float2(s * cos_t - o * sin_t, s * sin_t + o * cos_t), float2(0.0));

    // Gamut mapping: soft-clip saturation to the working gamut at this hue.
    float max_saturation = lookup_gamut(h);
    float sat = max_saturation;
    if (jc.x > 0.0)
        sat = soft_clip(jc.y / jc.x, 0.8 * max_saturation, max_saturation);
    float max_j_at_sat = jc.x;
    if (sat > 0.0)
        max_j_at_sat = jc.y / sat;
    jc = float2((jc.x + max_j_at_sat) / 2.0, (jc.y + jc.x * sat) / 2.0);

    // Clip chroma at constant Iz and hue so L'M'S' stays non-negative.
    float cos_h = cos(h);
    float sin_h = sin(h);
    float iz = jz_to_iz(jc.x);
    float3 lms_test = mul(JZ_IAB_TO_LMS, float3(iz, jc.y * cos_h, jc.y * sin_h));
    float max_c = jc.y;
    if (lms_test.x < 0.0)
        max_c = min(-iz / (0.1386050432715393 * cos_h + 0.0580473161561189 * sin_h), max_c);
    if (lms_test.y < 0.0)
        max_c = min(-iz / (-0.1386050432715393 * cos_h - 0.0580473161561189 * sin_h), max_c);
    if (lms_test.z < 0.0)
        max_c = min(-iz / (-0.0960192420263190 * cos_h - 0.8118918960560390 * sin_h), max_c);

    return jzazbz_to_xyz(float3(jc.x, max_c * cos_h, max_c * sin_h));
}

// Brilliance, saturation and gamut mapping in darktable UCS 22, from and to XYZ D65.
[Differentiable]
float3 ucs_saturation(float3 xyz, float3 masks, no_diff ColorBalanceParams p, Grading g)
{
    float l_white = p.l_white;
    float3 hcb = ucs_jch_to_hcb(xyy_to_ucs_jch(d65_xyz_to_xyy(xyz), l_white));

    // Rotate (C, B) onto the saturation axis: P along chroma, W the radius, with the
    // rotation that brings the axis back.
    float radius = sqrt(hcb.y * hcb.y + hcb.z * hcb.z);
    float sin_t = 0.0;
    float cos_t = 0.0;
    if (radius > 0.0)
    {
        sin_t = hcb.y / radius;
        cos_t = hcb.z / radius;
    }
    float chroma = max(UCS_SMALLEST_CHROMA, hcb.y);
    float w = sin_t * hcb.y + cos_t * hcb.z;

    // Saturation a, soft-clipped so the square root below stays real, and brilliance b.
    float a = max(1.0 + g.saturation_global + dot(masks, p.saturation.xyz), 0.0);
    float b = max(1.0 + g.brilliance_global + dot(masks, p.brilliance.xyz), 0.0);
    float max_a = sqrt(chroma * chroma + w * w) / chroma;
    a = soft_clip(a, 0.5 * max_a, max_a);
    float p_prime = (a - 1.0) * chroma;
    float w_prime = sqrt(max(chroma * chroma * (1.0 - a * a) + w * w, 0.0)) * b;
    hcb.y = max(cos_t * p_prime + sin_t * w_prime, 0.0);
    hcb.z = max(-sin_t * p_prime + cos_t * w_prime, 0.0);
    float3 jch = ucs_hcb_to_jch(hcb);

    // Gamut mapping: soft-clip saturation at constant brightness to the working gamut's
    // boundary at this hue and lightness, the table holding its colourfulness squared.
    float max_colorfulness = lookup_gamut(jch.z);
    float max_chroma = 15.932993652962535 * pow0(jch.x * l_white, 0.6523997524738018)
        * pow0(max_colorfulness, 0.6007557017508491) / l_white;
    float3 boundary = ucs_jch_to_hsb(float3(jch.x, max_chroma, jch.z));
    float saturation = 0.0;
    if (hcb.z > 0.0)
        saturation = hcb.y / hcb.z;
    saturation = soft_clip(saturation, 0.8 * boundary.y, boundary.y);
    jch = ucs_hsb_to_jch(float3(hcb.x, saturation, hcb.z));
    return xyy_to_xyz(ucs_jch_to_xyy(jch, l_white));
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    ColorBalanceParams p = params.color_balance;
    float4 pixel = input[i];
    float3 rgb = pixel.rgb;
    for (uint k = 0; k < params.pass.repeat; k++)
        rgb = color_balance(rgb, p, grading(p));
    output[i] = float4(rgb, pixel.a);
}

// Derivative entry points, both per pixel, with parameters ordered (chroma_global,
// contrast, saturation_global, brilliance_global). The contrast here is the kernel's
// 1 + contrast, whose derivative equals the setting's.
[[vk::binding(8, 0)]] StructuredBuffer<float4> derivative_in;
[[vk::binding(9, 0)]] RWStructuredBuffer<float4> derivative_out;

// Reverse mode: given dLoss/dOutput, the gradient of the loss with respect to the
// parameters.
[shader("compute")]
[numthreads(16, 16, 1)]
void colorbalancergb_backward(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    ColorBalanceParams p = params.color_balance;
    var g = diffPair(grading(p));
    bwd_diff(color_balance)(input[i].rgb, p, g, derivative_in[i].rgb);
    derivative_out[i] = float4(g.d.chroma_global, g.d.contrast, g.d.saturation_global, g.d.brilliance_global);
}

// Forward mode: given a direction in parameter space, the directional derivative of the
// output. Checks the reverse mode: for any direction v and output weights w,
// w · (J v) must equal (Jᵀ w) · v.
[shader("compute")]
[numthreads(16, 16, 1)]
void colorbalancergb_forward_diff(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    ColorBalanceParams p = params.color_balance;
    float4 v = derivative_in[i];
    Grading.Differential direction;
    direction.chroma_global = v.x;
    direction.contrast = v.y;
    direction.saturation_global = v.z;
    direction.brilliance_global = v.w;
    let result = fwd_diff(color_balance)(input[i].rgb, p, diffPair(grading(p), direction));
    derivative_out[i] = float4(result.d, 0.0);
}
