// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2020-2026  darktable developers (ported from src/iop/sigmoid.c)
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

// Sigmoid, ported from darktable src/iop/sigmoid.c, process(). See docs/ports/sigmoid.md;
// the setup, computed once per image on the CPU, is `prepare` in src/setup.rs.
//
// A generalised log-logistic curve takes scene-referred light to the display. Per channel,
// darktable's default: working RGB goes to the base primaries' space, has negative values
// desaturated away, is read as coordinates over inset and rotated primaries, goes through
// the curve channel by channel, has its middle channel's hue partly restored at constant
// energy, and returns through the inverse of a purity-restoring matrix. RGB ratio: the
// curve maps the channels' mean, the pixel is scaled by it, and its chroma is compressed
// into the display's range.

import pass;

static const uint PER_CHANNEL = 0;
static const uint RGB_RATIO = 1;

// Where darktable's f32 paper exposure lies (setup.rs, PaperExposure).
static const uint PAPER_FINITE = 0;
static const uint PAPER_INFINITE = 1;
static const uint PAPER_ZERO = 2;

struct SigmoidParams
{
    // Matrices by rows: working to base RGB, base to rendering RGB, rendering to working
    // RGB. Only the per-channel processing reads them.
    float4 pipe_to_base_r0;
    float4 pipe_to_base_r1;
    float4 pipe_to_base_r2;
    float4 base_to_rendering_r0;
    float4 base_to_rendering_r1;
    float4 base_to_rendering_r2;
    float4 rendering_to_pipe_r0;
    float4 rendering_to_pipe_r1;
    float4 rendering_to_pipe_r2;
    // The curve: the display's white and black, the log2 of the paper exposure, the fog.
    float white_target;
    float black_target;
    float log2_paper_exposure;
    float film_fog;
    // The curve's powers, the hue preservation (0 to 1) and the colour processing.
    float film_power;
    float paper_power;
    float hue_preservation;
    uint color_processing;
    // Where darktable's f32 curve breaks down, found on the CPU: the film values from which
    // its film response overflows, and up to which it underflows to 0, and where its paper
    // exposure lies.
    float film_overflow;
    float film_underflow;
    uint paper_exposure;
    uint _pad0;
};

struct Params
{
    PassHeader pass;
    SigmoidParams sigmoid;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;

// The parameters the derivative entry points differentiate against. The per-pixel
// function reads them from here rather than from SigmoidParams, which is a constant.
struct SigmoidCurve : IDifferentiable
{
    float white_target;
    float black_target;
    float log2_paper_exposure;
    float film_fog;
    float film_power;
    float paper_power;
    float hue_preservation;
};

SigmoidCurve sigmoid_curve(SigmoidParams p)
{
    SigmoidCurve c;
    c.white_target = p.white_target;
    c.black_target = p.black_target;
    c.log2_paper_exposure = p.log2_paper_exposure;
    c.film_fog = p.film_fog;
    c.film_power = p.film_power;
    c.paper_power = p.paper_power;
    c.hue_preservation = p.hue_preservation;
    return c;
}

// darktable _generalized_loglogistic_sigmoid, white · (F / (P + F))^q with
// F = (fog + max(x, 0))^p. Where darktable's f32 F or P leaves f32's range, its result:
// the target white where F overflows (inf / inf, its NaN check) or P underflows to 0
// (F / F), black where F underflows to 0 or P overflows (F / inf). The setup finds those
// limits in f32, so the kernel compares film values and never tests for infinity.
// Elsewhere in the log domain: F / (P + F) = 1 / (1 + 2^u) with u = log2 P − p log2(fog + x).
[Differentiable]
float loglogistic(no_diff float x, no_diff SigmoidParams p, SigmoidCurve c)
{
    float film = c.film_fog + max(x, 0.0);
    if (film >= p.film_overflow || p.paper_exposure == PAPER_ZERO)
        return c.white_target;
    if (film <= p.film_underflow || p.paper_exposure == PAPER_INFINITE)
        return 0.0;
    float u = c.log2_paper_exposure - c.film_power * log2(film);
    // log2(1 + 2^u), without overflowing 2^u.
    float softplus;
    if (u > 0.0)
        softplus = u + log2(1.0 + exp2(-u));
    else
        softplus = log2(1.0 + exp2(u));
    return c.white_target * exp2(-c.paper_power * softplus);
}

// darktable _desaturate_negative_values: a pixel with a negative channel is moved
// towards its mean until that channel is 0; a negative mean makes it black.
float3 desaturate_negative_values(float3 rgb)
{
    float average = max((rgb.x + rgb.y + rgb.z) / 3.0, 0.0);
    float minimum = min(rgb.x, min(rgb.y, rgb.z));
    float factor = minimum < 0.0 ? -average / (minimum - average) : 1.0;
    return average + factor * (rgb - average);
}

float3 one_hot(uint i)
{
    return float3(i == 0 ? 1.0 : 0.0, i == 1 ? 1.0 : 0.0, i == 2 ? 1.0 : 0.0);
}

// The channels of a pixel by value, as darktable _pixel_channel_order sorts them, ties
// included, each as a one-hot vector, so the hue correction reads and writes channels
// chosen at run time without indexing a vector (decision 10); with the two ratios of
// _preserve_hue_and_energy that depend on the curve's input alone.
struct HueOrder
{
    float3 min;
    float3 mid;
    float3 max;
    // Where the middle channel sits between the others, and the blend of the energy
    // targets.
    float midscale;
    float blend;
};

HueOrder hue_order(float3 v)
{
    // (min, mid, max)
    uint3 order;
    if (v.x >= v.y)
    {
        if (v.y > v.z)
            order = uint3(2, 1, 0); // r >= g > b
        else if (v.z > v.x)
            order = uint3(1, 0, 2); // b > r >= g
        else if (v.z > v.y)
            order = uint3(1, 2, 0); // r >= b > g
        else
            order = uint3(2, 1, 0); // r >= g == b
    }
    else
    {
        if (v.x >= v.z)
            order = uint3(2, 0, 1); // g > r >= b
        else if (v.z > v.y)
            order = uint3(0, 1, 2); // b > g > r
        else
            order = uint3(0, 2, 1); // g >= b > r
    }
    HueOrder o;
    o.min = one_hot(order.x);
    o.mid = one_hot(order.y);
    o.max = one_hot(order.z);
    float low = dot(v, o.min);
    float middle = dot(v, o.mid);
    float high = dot(v, o.max);
    float chroma = high - low;
    o.midscale = chroma != 0.0 ? (middle - low) / chroma : 0.0;
    float low_plus_middle = low + middle;
    o.blend = low_plus_middle != 0.0 ? 2.0 * low / low_plus_middle : 0.0;
    return o;
}

// Working RGB to the rendering primaries, negative values desaturated in the base space.
float3 to_rendering(float3 rgb, SigmoidParams p)
{
    float3x3 pipe_to_base = float3x3(p.pipe_to_base_r0.xyz, p.pipe_to_base_r1.xyz, p.pipe_to_base_r2.xyz);
    float3x3 base_to_rendering =
        float3x3(p.base_to_rendering_r0.xyz, p.base_to_rendering_r1.xyz, p.base_to_rendering_r2.xyz);
    return mul(base_to_rendering, desaturate_negative_values(mul(pipe_to_base, rgb)));
}

// darktable _preserve_hue_and_energy: the middle channel moved towards the hue the
// input had, by `hue_preservation`, keeping the sum of the channels the blend of the
// per-channel and the naively corrected sums.
[Differentiable]
float3 preserve_hue_and_energy(float3 per_channel, no_diff HueOrder o, float hue_preservation)
{
    float low = dot(per_channel, o.min);
    float middle = dot(per_channel, o.mid);
    float high = dot(per_channel, o.max);
    float full_hue_correction = low + (high - low) * o.midscale;
    float naive_hue_mid = (1.0 - hue_preservation) * middle + hue_preservation * full_hue_correction;
    float per_channel_energy = per_channel.x + per_channel.y + per_channel.z;
    float naive_hue_energy = low + naive_hue_mid + high;
    float energy_target = o.blend * per_channel_energy + (1.0 - o.blend) * naive_hue_energy;

    float new_low;
    float new_mid;
    float new_high;
    if (naive_hue_mid <= middle)
    {
        new_mid = ((1.0 - hue_preservation) * middle
                   + hue_preservation * (o.midscale * high + (1.0 - o.midscale) * (energy_target - high)))
                  / (1.0 + hue_preservation * (1.0 - o.midscale));
        new_low = energy_target - high - new_mid;
        new_high = high;
    }
    else
    {
        new_mid = ((1.0 - hue_preservation) * middle
                   + hue_preservation * (low * (1.0 - o.midscale) + o.midscale * (energy_target - low)))
                  / (1.0 + hue_preservation * o.midscale);
        new_low = low;
        new_high = energy_target - low - new_mid;
    }
    return o.min * new_low + o.mid * new_mid + o.max * new_high;
}

// darktable process_loglogistic_per_channel.
[Differentiable]
float3 per_channel(no_diff float3 rgb, no_diff SigmoidParams p, SigmoidCurve c)
{
    float3 rendering = no_diff to_rendering(rgb, p);
    HueOrder order = no_diff hue_order(rendering);
    float3 curve = float3(loglogistic(rendering.x, p, c), loglogistic(rendering.y, p, c), loglogistic(rendering.z, p, c));
    float3 corrected = preserve_hue_and_energy(curve, order, c.hue_preservation);
    float3x3 rendering_to_pipe =
        float3x3(p.rendering_to_pipe_r0.xyz, p.rendering_to_pipe_r1.xyz, p.rendering_to_pipe_r2.xyz);
    return mul(rendering_to_pipe, corrected);
}

// darktable process_loglogistic_rgb_ratio.
[Differentiable]
float3 rgb_ratio(no_diff float3 rgb, no_diff SigmoidParams p, SigmoidCurve c)
{
    float3 positive = no_diff desaturate_negative_values(rgb);
    float luma = (positive.x + positive.y + positive.z) / 3.0;
    float mapped = loglogistic(luma, p, c);
    float3 scaled;
    if (luma > 1e-9)
        scaled = (mapped / luma) * positive;
    else
        scaled = float3(mapped);
    float low = min(scaled.x, min(scaled.y, scaled.z));
    float high = max(scaled.x, max(scaled.y, scaled.z));

    // How far the display's white and black are from the mapped luma, relative to the
    // pixel's own channels, and how far its lowest channel is from 0.
    const float epsilon = 1e-6;
    float display_border_vs_chroma_white = (c.white_target - mapped) / (high - mapped + epsilon);
    float display_border_vs_chroma_black = (c.black_target - mapped) / (low - mapped - epsilon);
    float display_border_vs_chroma = min(display_border_vs_chroma_white, display_border_vs_chroma_black);
    float chroma_vs_mapping_border = (mapped - low) / (mapped + epsilon);

    // Hyperbolic gamut compression: chroma near the achromatic axis kept, large chroma
    // compressed.
    float pixel_chroma_adjustment = 1.0 / (chroma_vs_mapping_border * display_border_vs_chroma + epsilon);
    float hyperbolic_chroma = 2.0 * chroma_vs_mapping_border
                              / (1.0 - chroma_vs_mapping_border * chroma_vs_mapping_border + epsilon)
                              * pixel_chroma_adjustment;
    float hyperbolic_z = sqrt(hyperbolic_chroma * hyperbolic_chroma + 1.0);
    float chroma_factor = hyperbolic_chroma / (1.0 + hyperbolic_z) * display_border_vs_chroma;
    return mapped + chroma_factor * (scaled - mapped);
}

// The per-pixel function.
[Differentiable]
float3 sigmoid(no_diff float3 rgb_in, no_diff SigmoidParams p, SigmoidCurve c)
{
    // Sums of three channels and the matrices stay finite; NaN never reaches a module.
    float3 rgb = no_diff clamp(rgb_in, float3(-1e30), float3(1e30));
    if (p.color_processing == RGB_RATIO)
        return rgb_ratio(rgb, p, c);
    return per_channel(rgb, p, c);
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    SigmoidParams p = params.sigmoid;
    float4 pixel = input[i];
    float3 rgb = pixel.rgb;
    for (uint k = 0; k < params.pass.repeat; k++)
        rgb = sigmoid(rgb, p, sigmoid_curve(p));
    output[i] = float4(rgb, pixel.a);
}

// Derivative entry points, both per pixel, with parameters ordered (film power, paper
// power, white target, then the hue preservation per channel or the black target with
// RGB ratio, the one each processing reads). The powers are the kernel's, which darktable
// fits to the contrast and the skew; the fog and the paper exposure are differentiated
// too but not returned.
[[vk::binding(8, 0)]] StructuredBuffer<float4> derivative_in;
[[vk::binding(9, 0)]] RWStructuredBuffer<float4> derivative_out;

// Reverse mode: given dLoss/dOutput, the gradient of the loss with respect to the
// parameters.
[shader("compute")]
[numthreads(16, 16, 1)]
void sigmoid_backward(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    SigmoidParams p = params.sigmoid;
    var curve = diffPair(sigmoid_curve(p));
    bwd_diff(sigmoid)(input[i].rgb, p, curve, derivative_in[i].rgb);
    float last = p.color_processing == RGB_RATIO ? curve.d.black_target : curve.d.hue_preservation;
    derivative_out[i] = float4(curve.d.film_power, curve.d.paper_power, curve.d.white_target, last);
}

// Forward mode: given a direction in parameter space, the directional derivative of the
// output. Checks the reverse mode: for any direction v and output weights w,
// w · (J v) must equal (Jᵀ w) · v.
[shader("compute")]
[numthreads(16, 16, 1)]
void sigmoid_forward_diff(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    SigmoidParams p = params.sigmoid;
    float4 v = derivative_in[i];
    bool ratio = p.color_processing == RGB_RATIO;
    SigmoidCurve.Differential direction;
    direction.white_target = v.z;
    direction.black_target = ratio ? v.w : 0.0;
    direction.log2_paper_exposure = 0.0;
    direction.film_fog = 0.0;
    direction.film_power = v.x;
    direction.paper_power = v.y;
    direction.hue_preservation = ratio ? 0.0 : v.w;
    let result = fwd_diff(sigmoid)(input[i].rgb, p, diffPair(sigmoid_curve(p), direction));
    derivative_out[i] = float4(result.d, 0.0);
}
