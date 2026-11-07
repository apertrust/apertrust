// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2019-2026  darktable developers (ported from src/iop/rgblevels.c)
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

// RGB levels, ported from darktable src/iop/rgblevels.c, _compute_lut() and process().
// See docs/ports/rgblevels.md; `prepare` in src/setup.rs fills the parameters. The
// power and scale darktable derives from each channel's levels are computed here, once
// per pixel, so that the derivatives reach the levels themselves. darktable samples the
// power curve below the white point into a table of 65536 entries; this kernel
// evaluates the curve, except within the table's first TABLE_STEPS entries above black,
// where it reads darktable's entries, to match darktable's renders (the spec's
// Deliberate differences).

import color;
import components;
import pass;

// darktable's autoscale: each channel its own levels.
static const uint INDEPENDENT = 1;
static const float LN_10 = 2.30258509299404568402;
// darktable's table: 65536 entries from black to white, entry i the curve at i / 65536.
static const float TABLE_SIZE = 65536.0;
// The entries above black the kernel copies rather than evaluating the curve.
static const float TABLE_STEPS = 4.0;

struct RgbLevelsParams
{
    // Black, grey and white points per channel: red, or the linked levels, then green
    // and blue. w is unused.
    float4 black;
    float4 grey;
    float4 white;
    // The Y row of working RGB to XYZ D50, for the luminance norm.
    float4 luminance;
    // darktable's autoscale and preserve_colors (RGB_NORM_*).
    uint autoscale;
    uint preserve_colors;
    uint pad0;
    uint pad1;
    // The four levels the derivative entry points differentiate against: per slot a
    // channel (0 to 2) and a point (0 black, 1 grey, 2 white).
    uint4 gradient_channel;
    uint4 gradient_point;
};

struct Params
{
    PassHeader pass;
    RgbLevelsParams rgb_levels;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;

// The levels in darktable's units: what the derivatives are taken against.
struct Levels : IDifferentiable
{
    float3 black;
    float3 grey;
    float3 white;
};

Levels levels_of(RgbLevelsParams p)
{
    Levels l;
    l.black = p.black.xyz;
    l.grey = p.grey.xyz;
    l.white = p.white.xyz;
    return l;
}

// Each channel's curve: 0 at or below black, ((v - black) · scale)^power above.
struct Shape : IDifferentiable
{
    float3 black;
    float3 scale;
    float3 power;
    // Where t = (v - black) · scale stops in t^(power - 1), where darktable's powf would
    // overflow, to infinity, and to NaN where a norm's ratio meets a channel at 0. A
    // channel's curve stops below 2^64 there; a norm's ratio keeps the unclamped t / n,
    // so past the limit the norm's curve continues linearly, finite and continuous.
    float3 limit;
};

// Setup, once per image (darktable _compute_lut() and process()): the power is 10 to
// the grey point's offset from the middle of black and white, in half their distance;
// the scale is darktable's mult. Linked, every channel takes the first channel's
// levels, as darktable's commit_params() copies them; reading them from the first
// alone sends their whole derivative there.
[Differentiable]
Shape shape(Levels l, no_diff bool linked)
{
    float3 black = l.black;
    float3 grey = l.grey;
    float3 white = l.white;
    if (linked)
    {
        black = l.black.xxx;
        grey = l.grey.xxx;
        white = l.white.xxx;
    }
    float3 delta = (white - black) / 2.0;
    float3 mid = black + delta;
    float3 offset = (grey - mid) / delta;
    Shape s;
    s.black = black;
    s.scale = 1.0 / (white - black);
    s.power = exp(offset * LN_10);
    s.limit = exp2(64.0 / max(s.power, float3(1.0)));
    return s;
}

// darktable's table entry for t = (v - black) · scale when v lies within the table's
// first TABLE_STEPS entries above black, indexed as darktable's process() does,
// (int)(t · 65536); otherwise -1, for the curve.
float table_step(float t)
{
    return t > 0.0 && t * TABLE_SIZE < TABLE_STEPS ? floor(t * TABLE_SIZE) : -1.0;
}

// darktable's table entry `step`, (step / 65536)^power, written q · q^(power - 1) so
// that a power of 1 gives q exactly. Entry 0 is 0.
float table_entry(float step, float power)
{
    float q = step / TABLE_SIZE;
    return q * pow0(q, power - 1.0);
}

// `curve` with the value of darktable's table entry `step` when it is at least 0, and
// the curve's derivatives always: the table is flat between its entries, so its own
// derivatives would be 0 there, where finite differences, and an optimiser, see the
// curve. The value is the entry within rounding, exactly for entry 0.
[Differentiable]
float with_table_value(float curve, no_diff float step, float power)
{
    if (step < 0.0)
        return curve;
    float entry = no_diff table_entry(step, power);
    return curve + detach(entry - curve);
}

// One channel through its curve: darktable's table entry within its first steps above
// black (`step` at least 0), the curve beyond. darktable's table and its powf above
// white are the same curve, here written t · t^(power - 1) so that a power of 1 gives t
// exactly.
[Differentiable]
float level(no_diff float v, no_diff float step, float black, float scale, float power, float limit)
{
    float t = min((v - black) * scale, limit);
    return with_table_value(max(t, 0.0) * pow0(t, power - 1.0), step, power);
}

// The ratio a norm's curve scales every channel by, curve(norm) / norm, for a norm
// above black, written so that the default levels give exactly 1; within the table's
// first steps above black, from darktable's entry. Past the limit, only the power's
// factor stops: the norm's curve then grows linearly.
[Differentiable]
float ratio(no_diff float norm, no_diff float step, float black, float scale, float power, float limit)
{
    float t = min((norm - black) * scale, limit);
    float r = scale * (1.0 - black / norm) * pow0(t, power - 1.0);
    if (step < 0.0)
        return r;
    float entry = no_diff table_entry(step, power);
    return r + detach(entry / norm - r);
}

// How a pixel meets the curves (darktable process()).
static const uint CHANNELS = 0; // each channel through its curve
static const uint RATIO = 1;    // every channel scaled by the curve of a norm, over the norm
static const uint BLACK = 2;    // a norm at or below black: the pixel becomes black

// Found before the differentiable part, so the derivative passes get the comparison's
// outcome as an argument rather than recomputing it.
struct Lookup
{
    uint mode;
    float norm;
    // Per channel, or the norm's in x: the table entry to read (table_step), or -1.
    float3 step;
};

Lookup locate(float3 rgb, Shape s, RgbLevelsParams p)
{
    Lookup l;
    l.mode = CHANNELS;
    l.norm = 0.0;
    l.step = float3(
        table_step((rgb.x - s.black.x) * s.scale.x),
        table_step((rgb.y - s.black.y) * s.scale.y),
        table_step((rgb.z - s.black.z) * s.scale.z));
    if (p.autoscale != INDEPENDENT && p.preserve_colors != RGB_NORM_NONE)
    {
        l.norm = rgb_norm(rgb, p.preserve_colors, p.luminance.xyz);
        l.mode = l.norm > s.black.x ? RATIO : BLACK;
        l.step.x = table_step((l.norm - s.black.x) * s.scale.x);
    }
    return l;
}

// The per-pixel function.
[Differentiable]
float3 rgb_levels(no_diff float3 rgb, no_diff Lookup l, Shape s)
{
    if (l.mode == BLACK)
        return float3(0.0);
    if (l.mode == RATIO)
        return rgb * ratio(l.norm, l.step.x, s.black.x, s.scale.x, s.power.x, s.limit.x);
    return float3(
        level(rgb.x, l.step.x, s.black.x, s.scale.x, s.power.x, s.limit.x),
        level(rgb.y, l.step.y, s.black.y, s.scale.y, s.power.y, s.limit.y),
        level(rgb.z, l.step.z, s.black.z, s.scale.z, s.power.z, s.limit.z));
}

// Setup and per-pixel function together, for the gradient with respect to the levels
// through both.
[Differentiable]
float3 rgb_levels_from_levels(no_diff float3 rgb, no_diff Lookup l, no_diff bool linked, Levels levels)
{
    return rgb_levels(rgb, l, shape(levels, linked));
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    RgbLevelsParams p = params.rgb_levels;
    Shape s = shape(levels_of(p), p.autoscale != INDEPENDENT);
    float4 pixel = input[i];
    float3 rgb = pixel.rgb;
    for (uint k = 0; k < params.pass.repeat; k++)
        rgb = rgb_levels(rgb, locate(rgb, s, p), s);
    output[i] = float4(rgb, pixel.a);
}

// Derivative entry points, both per pixel, with parameters ordered as the gradient
// slots (gradient_channel and gradient_point's x, y, z and w), each a channel's black,
// grey or white point. Linked, a slot naming green or blue stands for nothing and gets a
// zero gradient.
[[vk::binding(8, 0)]] StructuredBuffer<float4> derivative_in;
[[vk::binding(9, 0)]] RWStructuredBuffer<float4> derivative_out;

// One slot's value in a derivative of the levels.
float pick(Levels.Differential d, uint channel, uint point)
{
    float3 v = point == 0 ? d.black : (point == 1 ? d.grey : d.white);
    return v[channel];
}

// Reverse mode: given dLoss/dOutput, the gradient of the loss with respect to the
// slots' levels.
[shader("compute")]
[numthreads(16, 16, 1)]
void rgblevels_backward(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    RgbLevelsParams p = params.rgb_levels;
    bool linked = p.autoscale != INDEPENDENT;
    Levels levels = levels_of(p);
    float3 rgb = input[i].rgb;
    Lookup l = locate(rgb, shape(levels, linked), p);
    var d = diffPair(levels);
    bwd_diff(rgb_levels_from_levels)(rgb, l, linked, d, derivative_in[i].rgb);
    derivative_out[i] = float4(
        pick(d.d, p.gradient_channel.x, p.gradient_point.x),
        pick(d.d, p.gradient_channel.y, p.gradient_point.y),
        pick(d.d, p.gradient_channel.z, p.gradient_point.z),
        pick(d.d, p.gradient_channel.w, p.gradient_point.w));
}

// Adds x to the slot's level in a direction: to component `channel` of the point it
// names, and to component 3, which changes nothing, of the others.
void add_slot(inout Levels.Differential d, uint channel, uint point, float x)
{
    d.black = add_at(d.black, point == 0 ? channel : 3, x);
    d.grey = add_at(d.grey, point == 1 ? channel : 3, x);
    d.white = add_at(d.white, point == 2 ? channel : 3, x);
}

// Forward mode: given a direction in the slots' levels, the directional derivative of
// the output. Checks the reverse mode: for any direction v and output weights w,
// w · (J v) must equal (Jᵀ w) · v.
[shader("compute")]
[numthreads(16, 16, 1)]
void rgblevels_forward_diff(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    RgbLevelsParams p = params.rgb_levels;
    bool linked = p.autoscale != INDEPENDENT;
    Levels levels = levels_of(p);
    float3 rgb = input[i].rgb;
    Lookup l = locate(rgb, shape(levels, linked), p);
    float4 v = derivative_in[i];
    Levels.Differential direction;
    direction.black = float3(0.0);
    direction.grey = float3(0.0);
    direction.white = float3(0.0);
    add_slot(direction, p.gradient_channel.x, p.gradient_point.x, v.x);
    add_slot(direction, p.gradient_channel.y, p.gradient_point.y, v.y);
    add_slot(direction, p.gradient_channel.z, p.gradient_point.z, v.z);
    add_slot(direction, p.gradient_channel.w, p.gradient_point.w, v.w);
    let result = fwd_diff(rgb_levels_from_levels)(rgb, l, linked, diffPair(levels, direction));
    derivative_out[i] = float4(result.d, 0.0);
}
