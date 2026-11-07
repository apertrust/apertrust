// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2019-2026  darktable developers (ported from src/iop/rgbcurve.c)
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

// RGB curve, ported from darktable src/iop/rgbcurve.c, process(). See
// docs/ports/rgbcurve.md; the setup, computed once per image on the CPU, is `prepare` in
// src/setup.rs. darktable samples each curve into a 16-bit table of 65536 entries; this
// kernel evaluates the cubic darktable samples, from the knots and the tangents its
// interpolation gives them.

import color;
import pass;

static const uint MAX_NODES = 20;
// darktable's curve_autoscale: each channel its own curve.
static const uint INDEPENDENT = 1;
// What a gradient slot stands for.
static const uint KIND_X = 0;
static const uint KIND_Y = 1;
static const uint KIND_TANGENT = 2;
static const uint KIND_END_VALUE = 3;
static const uint KIND_END_POWER = 4;

struct RgbCurveParams
{
    // The Y row of working RGB to XYZ D50, for the luminance norm.
    float4 luminance;
    // Knots per curve: red, or the linked curve, then green and blue.
    uint4 count;
    // Past its last knot, curve c is end_value[c] · (v / x_last)^end_power[c].
    float4 end_value;
    float4 end_power;
    // darktable's curve_autoscale and preserve_colors (RGB_NORM_*).
    uint autoscale;
    uint preserve_colors;
    uint pad0;
    uint pad1;
    // The four values the derivative entry points differentiate against: per slot, a
    // curve, a knot (0 for the end value and power) and a KIND_*.
    uint4 gradient_curve;
    uint4 gradient_node;
    uint4 gradient_kind;
};

struct Params
{
    PassHeader pass;
    RgbCurveParams rgb_curve;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;
// Per curve, MAX_NODES knots (x, y, tangent, 0) in linear values, the tangent dy/dx.
[[vk::binding(3, 0)]] StructuredBuffer<float4> knots;

// Where a value meets a curve. It is found before the differentiable part, so the
// derivative passes get the segment and the side as arguments rather than recomputing
// the comparisons that chose them, which on Metal need not repeat bit for bit.
struct Point
{
    uint curve;
    // The segment from knot `segment` to the next that the value falls in.
    uint segment;
    // 1 at or past the curve's last knot, where the extrapolation takes over.
    uint above;
    float v;
};

// darktable's segment search (catmull_rom_val, spline_cubic_val): the first segment
// whose right knot lies above v, or the last.
Point locate(uint curve, float v, RgbCurveParams p)
{
    uint n = p.count[curve];
    uint base = curve * MAX_NODES;
    uint segment = n - 2;
    for (uint k = 0; k + 2 < n; k++)
    {
        if (v < knots[base + k + 1].x)
        {
            segment = k;
            break;
        }
    }
    Point point;
    point.curve = curve;
    point.segment = segment;
    point.above = v >= knots[base + n - 1].x ? 1 : 0;
    point.v = v;
    return point;
}

// How a pixel meets the curves (darktable process()).
static const uint CHANNELS = 0;  // each channel through its curve
static const uint RATIO = 1;     // every channel scaled by the curve of a norm, over the norm
static const uint UNCHANGED = 2; // a norm of 0 or less: the pixel as it is

struct Lookup
{
    uint mode;
    Point r;
    Point g;
    Point b;
};

Lookup locate_pixel(float3 rgb, RgbCurveParams p)
{
    Lookup l;
    if (p.autoscale == INDEPENDENT)
    {
        l.mode = CHANNELS;
        l.r = locate(0, rgb.x, p);
        l.g = locate(1, rgb.y, p);
        l.b = locate(2, rgb.z, p);
    }
    else if (p.preserve_colors == RGB_NORM_NONE)
    {
        l.mode = CHANNELS;
        l.r = locate(0, rgb.x, p);
        l.g = locate(0, rgb.y, p);
        l.b = locate(0, rgb.z, p);
    }
    else
    {
        float norm = rgb_norm(rgb, p.preserve_colors, p.luminance.xyz);
        l.mode = norm > 0.0 ? RATIO : UNCHANGED;
        l.r = locate(0, norm, p);
        l.g = l.r;
        l.b = l.r;
    }
    return l;
}

// The values the derivative entry points differentiate against: the gradient slots'.
// The per-pixel function reads them from here rather than from the tables, which are
// constants.
struct Gradient : IDifferentiable
{
    float s0;
    float s1;
    float s2;
    float s3;
};

float slot_value(uint curve, uint node, uint kind, RgbCurveParams p)
{
    if (kind == KIND_END_VALUE)
        return p.end_value[curve];
    if (kind == KIND_END_POWER)
        return p.end_power[curve];
    float4 knot = knots[curve * MAX_NODES + node];
    return kind == KIND_X ? knot.x : (kind == KIND_Y ? knot.y : knot.z);
}

Gradient gradient(RgbCurveParams p)
{
    Gradient g;
    g.s0 = slot_value(p.gradient_curve.x, p.gradient_node.x, p.gradient_kind.x, p);
    g.s1 = slot_value(p.gradient_curve.y, p.gradient_node.y, p.gradient_kind.y, p);
    g.s2 = slot_value(p.gradient_curve.z, p.gradient_node.z, p.gradient_kind.z, p);
    g.s3 = slot_value(p.gradient_curve.w, p.gradient_node.w, p.gradient_kind.w, p);
    return g;
}

// `value`, a curve's value of this kind at this knot, or the gradient slot's that stands
// for it, so derivatives reach the slot. Without `slots`, as in `main`, where the slots
// hold the tables' values anyway, `value` as it is. `main` passes a literal false; with
// it, the comparisons cost nothing measurable (`docs/ports/rgbcurve.md`, Gradients).
[Differentiable]
float slotted(float value, no_diff uint curve, no_diff uint node, no_diff uint kind, no_diff RgbCurveParams p, no_diff bool slots, Gradient g)
{
    if (!slots)
        return value;
    if (p.gradient_curve.x == curve && p.gradient_node.x == node && p.gradient_kind.x == kind)
        value = g.s0;
    if (p.gradient_curve.y == curve && p.gradient_node.y == node && p.gradient_kind.y == kind)
        value = g.s1;
    if (p.gradient_curve.z == curve && p.gradient_node.z == node && p.gradient_kind.z == kind)
        value = g.s2;
    if (p.gradient_curve.w == curve && p.gradient_node.w == node && p.gradient_kind.w == kind)
        value = g.s3;
    return value;
}

// Knot k of a curve: x, y and tangent.
[Differentiable]
float3 knot(no_diff uint curve, no_diff uint k, no_diff RgbCurveParams p, no_diff bool slots, Gradient g)
{
    float4 t = knots[curve * MAX_NODES + k];
    return float3(
        slotted(t.x, curve, k, KIND_X, p, slots, g),
        slotted(t.y, curve, k, KIND_Y, p, slots, g),
        slotted(t.z, curve, k, KIND_TANGENT, p, slots, g));
}

// The curve at the point's value. Below the last knot: the cubic Hermite segment the
// value falls in (darktable catmull_rom_val), flat at the first knot's y below it, and
// clamped to [0, 1] as darktable clamps its table. From the last knot on: darktable's
// power-law extrapolation (dt_iop_eval_exp).
[Differentiable]
float curve_at(no_diff Point point, no_diff RgbCurveParams p, no_diff bool slots, Gradient g)
{
    uint c = point.curve;
    if (point.above != 0)
    {
        float x_last = knot(c, p.count[c] - 1, p, slots, g).x;
        float value = slotted(p.end_value[c], c, 0, KIND_END_VALUE, p, slots, g);
        float power = slotted(p.end_power[c], c, 0, KIND_END_POWER, p, slots, g);
        return value * pow0(point.v / x_last, power);
    }
    float3 a = knot(c, point.segment, p, slots, g);
    float3 b = knot(c, point.segment + 1, p, slots, g);
    float h = b.x - a.x;
    float t = (max(point.v, a.x) - a.x) / h;
    float t2 = t * t;
    float t3 = t * t2;
    float y = (2.0 * t3 - 3.0 * t2 + 1.0) * a.y
        + (t3 - 2.0 * t2 + t) * h * a.z
        + (-2.0 * t3 + 3.0 * t2) * b.y
        + (t3 - t2) * h * b.z;
    return clamp(y, 0.0, 1.0);
}

// The per-pixel function. `slots` makes it a function of the gradient slots' values in
// `g`, for the derivative entry points; without, it reads the tables alone.
[Differentiable]
float3 rgb_curve(no_diff float3 rgb, no_diff Lookup l, no_diff RgbCurveParams p, no_diff bool slots, Gradient g)
{
    if (l.mode == UNCHANGED)
        return rgb;
    if (l.mode == RATIO)
        return rgb * (curve_at(l.r, p, slots, g) / l.r.v);
    return float3(curve_at(l.r, p, slots, g), curve_at(l.g, p, slots, g), curve_at(l.b, p, slots, g));
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    RgbCurveParams p = params.rgb_curve;
    Gradient unused = { 0.0, 0.0, 0.0, 0.0 };
    float4 pixel = input[i];
    float3 rgb = pixel.rgb;
    for (uint k = 0; k < params.pass.repeat; k++)
        rgb = rgb_curve(rgb, locate_pixel(rgb, p), p, false, unused);
    output[i] = float4(rgb, pixel.a);
}

// Derivative entry points, both per pixel, with parameters ordered as the gradient
// slots (gradient_curve, gradient_node and gradient_kind's x, y, z and w). Each stands
// for one of the kernel's own values: a knot's x, y or tangent, or a curve's end value
// or end power. A slot naming a knot past the curve's count stands for nothing and gets
// a zero gradient.
[[vk::binding(8, 0)]] StructuredBuffer<float4> derivative_in;
[[vk::binding(9, 0)]] RWStructuredBuffer<float4> derivative_out;

// Reverse mode: given dLoss/dOutput, the gradient of the loss with respect to the
// slots' values.
[shader("compute")]
[numthreads(16, 16, 1)]
void rgbcurve_backward(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    RgbCurveParams p = params.rgb_curve;
    float3 rgb = input[i].rgb;
    var g = diffPair(gradient(p));
    bwd_diff(rgb_curve)(rgb, locate_pixel(rgb, p), p, true, g, derivative_in[i].rgb);
    derivative_out[i] = float4(g.d.s0, g.d.s1, g.d.s2, g.d.s3);
}

// Forward mode: given a direction in the slots' values, the directional derivative of
// the output. Checks the reverse mode: for any direction v and output weights w,
// w · (J v) must equal (Jᵀ w) · v.
[shader("compute")]
[numthreads(16, 16, 1)]
void rgbcurve_forward_diff(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    RgbCurveParams p = params.rgb_curve;
    float3 rgb = input[i].rgb;
    float4 v = derivative_in[i];
    Gradient.Differential direction;
    direction.s0 = v.x;
    direction.s1 = v.y;
    direction.s2 = v.z;
    direction.s3 = v.w;
    let result = fwd_diff(rgb_curve)(rgb, locate_pixel(rgb, p), p, true, diffPair(gradient(p), direction));
    derivative_out[i] = float4(result.d, 0.0);
}
