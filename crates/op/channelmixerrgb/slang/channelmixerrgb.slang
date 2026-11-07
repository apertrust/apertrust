// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/iop/channelmixerrgb.c)
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

// Color calibration, ported from darktable src/iop/channelmixerrgb.c, process() through
// _loop_switch(), _gamut_mapping() and _luma_chroma(), the version 3 colourfulness. See
// docs/ports/channelmixerrgb.md; the setup, computed once per image on the CPU, is
// `prepare` in src/setup.rs. The illuminant's conversion to the adaptation space is
// here, so derivatives reach its x and y.

import color;
import pass;

struct ChannelMixerParams
{
    // Working RGB to where the illuminant's gains apply, by rows: CAT16 or Bradford
    // LMS; XYZ for XYZ and for non-linear Bradford, which converts to Bradford per
    // pixel; the identity for none.
    float4 to_lms_0;
    float4 to_lms_1;
    float4 to_lms_2;
    // The mixer, then the adaptation space to XYZ (for none, working RGB to XYZ).
    float4 mix_0;
    float4 mix_1;
    float4 mix_2;
    // XYZ to the adaptation's cone space, for the illuminant and non-linear Bradford's
    // pixels: the identity for XYZ and none.
    float4 cone_0;
    float4 cone_1;
    float4 cone_2;
    // After gamut mapping, XYZ to where colourfulness and brightness work (the cone
    // space, XYZ, or working RGB for none), and back.
    float4 to_space_0;
    float4 to_space_1;
    float4 to_space_2;
    float4 from_space_0;
    float4 from_space_1;
    float4 from_space_2;
    // XYZ D50 to working RGB.
    float4 to_rgb_0;
    float4 to_rgb_1;
    float4 to_rgb_2;
    // The pipeline's D50 in the cone space; 1 for none.
    float4 d50;
    // Per channel of the space they work in: colourfulness, with darktable's sign
    // (minus the setting), brightness, and the grey weights, all normalised.
    float4 saturation;
    float4 lightness;
    float4 grey;
    // The illuminant's chromaticity, CIE 1931 2°, relative to the pipeline's D50.
    float x;
    float y;
    // 1 / gamut compression, 0 for none.
    float compression;
    // 0 with clipping, -FLT_MAX without.
    float clip_min;
    // darktable's dt_adaptation_t.
    uint adaptation;
    uint apply_grey;
    // What a negative luminance-weighted distance gives at this compression, as
    // darktable's powf does (NEGATIVE_*).
    uint negative_delta;
    uint unused;
};

struct Params
{
    PassHeader pass;
    ChannelMixerParams mixer;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;

static const uint LINEAR_BRADFORD = 0;
static const uint CAT16 = 1;
static const uint FULL_BRADFORD = 2;
static const uint XYZ = 3;
static const uint RGB = 4;

// A negative distance to the power: powf's NaN, after which darktable's clamps return
// D50 (a compression that is not a whole number), or a real power of either sign.
static const uint NEGATIVE_NEUTRAL = 0;
static const uint NEGATIVE_ODD = 1;
static const uint NEGATIVE_EVEN = 2;

static const float INVERSE_SQRT_3 = 0.5773502691896258;

// Guards darktable lacks, which keep every value the kernel computes finite for any
// input and any setting the schema allows (docs/ports/channelmixerrgb.md, Deliberate
// differences). Each changes only what is far beyond any photo's values, where
// darktable's results are absurd, infinite or NaN; none fires on the golden cases.
//
// Inputs are held within this, which also turns infinities finite: far beyond scene
// light, whose white is 1.
static const float INPUT_LIMIT = 1e8;
// XYZ is held within this after the mix and after gamut mapping.
static const float XYZ_LIMIT = 1e12;
// Non-linear Bradford's ratio of the third cone to the illuminant's, at unit luminance,
// is held below this before its power: a real colour's is within about 10.
static const float RATIO_LIMIT = 1e8;
// Gamut mapping's sum X + Y + Z counts as at least this fraction of |X| + |Y| + |Z|, and
// each u'v' denominator as at least NORM_MIN of the sum of its terms' sizes, so x, y, u
// and v stay bounded. Only XYZ with negative components, with clipping off, comes near.
static const float SUM_FLOOR = 1e-6;
// A negative distance's power, with clipping off, is held at most 2^20.
static const float LOG2_POWER_LIMIT = 20.0;
// Brightness's factor is held at most this; with clipping, it is at most 13.
static const float BRIGHTNESS_LIMIT = 1e6;

// The parameters the derivative entry points differentiate against.
struct Illuminant : IDifferentiable
{
    float x;
    float y;
};

Illuminant illuminant(ChannelMixerParams p)
{
    Illuminant i;
    i.x = p.x;
    i.y = p.y;
    return i;
}

float3x3 rows(float4 r0, float4 r1, float4 r2)
{
    return float3x3(r0.xyz, r1.xyz, r2.xyz);
}

// darktable's clip: negatives removed with clipping on, nothing without.
[Differentiable]
float3 clip(float3 v, no_diff float floor)
{
    return max(v, float3(floor));
}

// One u'v' component after compression: moved towards D50 by `moved`, but never past
// it (darktable's fmaxf and fminf about D50).
[Differentiable]
float towards_white(float uv, float moved, no_diff float white)
{
    return uv > white ? max(moved, white) : min(moved, white);
}

// v to the power c, for v ≥ 0 and c > 0, held at most 2^LOG2_POWER_LIMIT; below that,
// pow0's value. c may be large, 1 / gamut, so v^c is never formed above 1.
[Differentiable]
float bounded_power(float v, no_diff float c)
{
    if (v < 1.0)
        return pow0(v, c);
    return exp2(min(c * log2(v), LOG2_POWER_LIMIT));
}

// darktable _gamut_mapping: in xyY, the chromaticity moved towards D50 in u'v' by the
// luminance-weighted squared distance to the compression's power, then sanitised so y
// stays positive and Z non-negative.
[Differentiable]
float3 gamut_mapping(float3 xyz, no_diff ChannelMixerParams p)
{
    float sum = xyz.x + xyz.y + xyz.z;
    float luminance = xyz.y;
    float safe_sum = max(sum, SUM_FLOOR * (abs(xyz.x) + abs(xyz.y) + abs(xyz.z)));
    float2 xy = sum > 0.0 ? float2(xyz.x, xyz.y) / safe_sum : D50_XY;

    float2 uv = xy_to_uv(xy);
    float2 delta = D50_UV - uv;
    float distance = luminance * dot(delta, delta);

    // darktable's powf(distance, compression). A negative distance needs clipping off
    // and a negative luminance. From 1 on, the correction lands on D50 whatever its
    // size, so it is held there.
    float correction = 0.0;
    bool neutral = false;
    if (p.compression != 0.0)
    {
        if (distance >= 0.0)
            correction = bounded_power(distance, p.compression);
        else
        {
            float magnitude = bounded_power(-distance, p.compression);
            correction = p.negative_delta == NEGATIVE_ODD ? -magnitude : magnitude;
            neutral = p.negative_delta == NEGATIVE_NEUTRAL;
        }
    }
    correction = min(correction, 1.0);
    float2 moved = correction * delta + uv;
    uv = float2(towards_white(uv.x, moved.x, D50_UV.x), towards_white(uv.y, moved.y, D50_UV.y));
    uv = neutral ? D50_UV : uv;

    xy = uv_to_xy(uv);
    xy = max(xy, float2(p.clip_min));
    xy = float2(xy.x, max(xy.y, NORM_MIN));
    float scale = xy.x + xy.y;
    xy = scale >= 1.0 ? xy / scale : xy;
    return float3(luminance * xy.x / xy.y, luminance, luminance * (1.0 - xy.x - xy.y) / xy.y);
}

// The Euclidean norm, scaled by the largest component so its squares cannot overflow.
[Differentiable]
float scaled_length(float3 v)
{
    float largest = max(abs(v.x), max(abs(v.y), abs(v.z)));
    float safe = largest > 0.0 ? largest : 1.0;
    return length(v / safe) * safe;
}

// darktable _luma_chroma, version 3: colourfulness moves the channels' ratios to their
// norm away from or towards grey, keeping the norm, then brightness scales the pixel.
[Differentiable]
float3 luma_chroma(float3 c, no_diff ChannelMixerParams p)
{
    float norm = max(scaled_length(c), NORM_MIN);
    float average = max((c.x + c.y + c.z) / 3.0, NORM_MIN);
    float mix = dot(c, p.lightness.xyz);

    norm *= INVERSE_SQRT_3;
    float3 ratios = c / norm;
    float coefficient = dot(ratios, p.saturation.xyz) / 3.0;
    // A ratio that was negative may stay so; one that was not cannot become so.
    ratios = max((1.0 - ratios) * coefficient + ratios, min(ratios, float3(0.0)));
    norm /= max(scaled_length(ratios), NORM_MIN) * INVERSE_SQRT_3;
    norm *= clamp(1.0 + mix / average, 0.0, BRIGHTNESS_LIMIT);
    return ratios * norm;
}

// The per-pixel function, differentiable in the pixel and the illuminant.
[Differentiable]
float3 channel_mixer(float3 rgb, no_diff ChannelMixerParams p, Illuminant illuminant)
{
    // The illuminant in the cone space, and the gains that adapt it to D50. darktable
    // divides by the cone responses unguarded; they are held at least NORM_MIN from 0,
    // keeping their sign, so a real light whose response is negative (Bradford's
    // second cone for blue and violet light) keeps darktable's negative gain.
    float3x3 to_lms = no_diff rows(p.to_lms_0, p.to_lms_1, p.to_lms_2);
    float3x3 mix = no_diff rows(p.mix_0, p.mix_1, p.mix_2);
    float3x3 cone = no_diff rows(p.cone_0, p.cone_1, p.cone_2);
    float3x3 to_space = no_diff rows(p.to_space_0, p.to_space_1, p.to_space_2);
    float3x3 from_space = no_diff rows(p.from_space_0, p.from_space_1, p.from_space_2);
    float3x3 to_rgb = no_diff rows(p.to_rgb_0, p.to_rgb_1, p.to_rgb_2);

    float3 white = illuminant_xy_to_xyz(float2(illuminant.x, illuminant.y));
    float3 lms_white = away_from_zero(mul(cone, white), NORM_MIN);

    float3 lms = mul(to_lms, clip(clamp(rgb, -INPUT_LIMIT, INPUT_LIMIT), p.clip_min));
    if (p.adaptation == FULL_BRADFORD)
    {
        // In XYZ so far. CIECAM97s's power on the third cone, of the ratio to the
        // illuminant at unit luminance (darktable's downscale_vector), where positive.
        // An illuminant whose third cone responds negatively, as yellow and red light
        // do, gets the linear adaptation, where darktable's powf gives NaN.
        float scale = lms.y > NORM_MIN ? lms.y + NORM_MIN : NORM_MIN;
        float power = lms_white.z > 0.0 ? pow0(0.818155 / lms_white.z, 0.0834) : 1.0;
        float3 ratio = mul(cone, lms) / scale / lms_white;
        float blue = ratio.z > 0.0 ? pow0(min(ratio.z, RATIO_LIMIT), power) : ratio.z;
        lms = p.d50.xyz * float3(ratio.x, ratio.y, blue) * scale;
    }
    else if (p.adaptation != RGB)
    {
        lms = lms * (p.d50.xyz / lms_white);
    }

    float3 xyz = clamp(clip(mul(mix, lms), p.clip_min), -XYZ_LIMIT, XYZ_LIMIT);
    xyz = clamp(gamut_mapping(xyz, p), -XYZ_LIMIT, XYZ_LIMIT);
    float3 c = clip(mul(to_space, xyz), p.clip_min);
    c = clip(luma_chroma(c, p), p.clip_min);
    if (p.apply_grey != 0)
        return float3(max(dot(c, p.grey.xyz), 0.0));
    xyz = clip(mul(from_space, c), p.clip_min);
    return clip(mul(to_rgb, xyz), p.clip_min);
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    ChannelMixerParams p = params.mixer;
    float4 pixel = input[i];
    float3 rgb = pixel.rgb;
    for (uint k = 0; k < params.pass.repeat; k++)
        rgb = channel_mixer(rgb, p, illuminant(p));
    output[i] = float4(rgb, pixel.a);
}

// Derivative entry points, both per pixel, with parameters ordered (x, y): the
// illuminant's chromaticity. The per-pixel function is differentiable in the pixel too,
// for when the interface carries dLoss/dInput; neither entry point exports it yet.
[[vk::binding(8, 0)]] StructuredBuffer<float4> derivative_in;
[[vk::binding(9, 0)]] RWStructuredBuffer<float4> derivative_out;

// Reverse mode: given dLoss/dOutput, the gradient of the loss with respect to x and y.
[shader("compute")]
[numthreads(16, 16, 1)]
void channelmixerrgb_backward(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    ChannelMixerParams p = params.mixer;
    var pixel = diffPair(input[i].rgb);
    var white = diffPair(illuminant(p));
    bwd_diff(channel_mixer)(pixel, p, white, derivative_in[i].rgb);
    derivative_out[i] = float4(white.d.x, white.d.y, 0.0, 0.0);
}

// Forward mode: given a direction in (x, y), the directional derivative of the output.
// Checks the reverse mode: for any direction v and output weights w, w · (J v) must
// equal (Jᵀ w) · v.
[shader("compute")]
[numthreads(16, 16, 1)]
void channelmixerrgb_forward_diff(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    ChannelMixerParams p = params.mixer;
    float4 v = derivative_in[i];
    Illuminant.Differential direction;
    direction.x = v.x;
    direction.y = v.y;
    let result = fwd_diff(channel_mixer)(diffPair(input[i].rgb, float3(0.0)), p,
                                         diffPair(illuminant(p), direction));
    derivative_out[i] = float4(result.d, 0.0);
}
