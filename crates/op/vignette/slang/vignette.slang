// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/iop/vignette.c)
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

// Vignetting, ported from darktable src/iop/vignette.c, process(). See
// docs/ports/vignette.md; `prepare` in src/setup.rs fills the parameters, in darktable's
// units. The geometry darktable derives from them before its loop is computed here, so
// that the derivatives reach the parameters themselves. A pixel's weight depends on its
// position in the whole image at the pipe's scale, the pass header's origin plus the
// invocation, so a tile or the viewport computes what the whole image would. Without
// dithering, which is not ported, darktable's CPU path keeps the weight linear.

import color;
import pass;

struct VignetteParams
{
    // darktable's parameters: the fall-off's start and width in percent of half the
    // image, the strengths on brightness and saturation, the centre in half the image
    // from its middle, the width-to-height ratio and the shape.
    float scale;
    float falloff_scale;
    float brightness;
    float saturation;
    float center_x;
    float center_y;
    float whratio;
    float shape;
    // darktable's autoratio (the ratio follows the image's) and unbound (no clipping).
    uint autoratio;
    uint unbound;
    uint pad0;
    uint pad1;
    // The parameters the derivative entry points differentiate against, one per slot:
    // 0 scale, 1 falloff_scale, 2 brightness, 3 saturation, 4 center_x, 5 center_y,
    // 6 whratio, 7 shape.
    uint4 gradient;
};

struct Params
{
    PassHeader pass;
    VignetteParams vignette;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;

// The parameters in darktable's units: what the derivatives are taken against.
struct Vignette : IDifferentiable
{
    float scale;
    float falloff_scale;
    float brightness;
    float saturation;
    float2 center;
    float whratio;
    float shape;
};

Vignette vignette_of(VignetteParams p)
{
    Vignette v;
    v.scale = p.scale;
    v.falloff_scale = p.falloff_scale;
    v.brightness = p.brightness;
    v.saturation = p.saturation;
    v.center = float2(p.center_x, p.center_y);
    v.whratio = p.whratio;
    v.shape = p.shape;
    return v;
}

// The image the vignette lies on: its size at full resolution (darktable's
// piece->buf_in), the pipe's scale (roi->scale), and the two switches.
struct Frame
{
    float2 full;
    float scale;
    bool autoratio;
    bool unbound;
};

Frame frame_of(Params p)
{
    Frame f;
    f.full = float2(float(p.pass.full_width), float(p.pass.full_height));
    f.scale = p.pass.scale;
    f.autoratio = p.vignette.autoratio != 0;
    f.unbound = p.vignette.unbound != 0;
    return f;
}

// darktable process() before its loop: where the vignette lies on the image at the
// pipe's scale.
struct Geometry : IDifferentiable
{
    // xscale and yscale: from pixels at the pipe's scale to the vignette's units.
    float2 scale;
    // roi_center_scaled: the centre at the pipe's scale, in the vignette's units.
    float2 center;
    // dscale and fscale: the inner radius and the fall-off's width.
    float inner;
    float falloff;
    // exp1 and exp2: the superellipse's exponent and its reciprocal.
    float exponent;
    float root;
};

// Setup, once per image (darktable process()). The kernel works in the whole image's
// coordinates, so the centre needs no region corner subtracted. The ratio is never 0 or
// 2 here: prepare keeps it within [0.001, 1.999].
[Differentiable]
Geometry geometry(Vignette v, no_diff Frame f)
{
    float2 scale;
    if (f.autoratio)
        // The ratio follows the image's.
        scale = 2.0 / (f.full * f.scale);
    else
    {
        // A fixed ratio, scaled to the longer side: up to 1 the width over the height,
        // from 1 to 2 the height over the width taken from 2.
        float basis = 2.0 / (max(f.full.x, f.full.y) * f.scale);
        if (v.whratio <= 1.0)
            scale = float2(basis / v.whratio, basis);
        else
            scale = float2(basis, basis / (2.0 - v.whratio));
    }
    float2 center = f.full * 0.5 + v.center * f.full / 2.0;
    // A minimum fall-off of about a pixel at full resolution, against aliasing.
    float min_falloff = 100.0 / min(f.full.x, f.full.y);
    float shape = max(v.shape, 0.001);
    Geometry g;
    g.scale = scale;
    g.center = center * f.scale * scale;
    g.inner = v.scale / 100.0;
    g.falloff = max(v.falloff_scale, min_falloff) / 100.0;
    g.exponent = 2.0 / shape;
    g.root = shape / 2.0;
    return g;
}

// The pixel's weight in the vignette (darktable process()): 0 inside the inner radius,
// rising linearly to 1 across the fall-off. darktable's length from the centre,
// (pv.x^exp1 + pv.y^exp1)^exp2, is written m · (1 + (n/m)^exp1)^exp2, with m the larger
// of pv.x and pv.y and n the smaller: the same length, since exp1 · exp2 = 1, whose base
// lies between 1 and 2, so it neither underflows for small shapes nor overflows far
// outside a narrow ratio, as darktable's does.
[Differentiable]
float weight(no_diff float2 position, Geometry g)
{
    float2 pv = abs(position * g.scale - g.center);
    float m = max(pv.x, pv.y);
    float n = min(pv.x, pv.y);
    float length = m * pow0(1.0 + pow0(n / max(m, 1e-30), g.exponent), g.root);
    return clamp((length - g.inner) / g.falloff, 0.0, 1.0);
}

// The vignetted pixel's colour (darktable process()): the brightness scales the pixel
// below 0 and adds to it from 0, then the saturation moves each channel from the
// channels' mean; each step clips to [0, 1] unless unbound.
[Differentiable]
float3 shade(no_diff float3 rgb, float w, float brightness, float saturation, no_diff bool unbound)
{
    float3 col;
    if (brightness < 0.0)
        col = rgb * (1.0 + w * brightness);
    else
        col = rgb + w * brightness;
    if (!unbound)
        col = clamp(col, float3(0.0), float3(1.0));
    float mv = (col.x + col.y + col.z) / 3.0;
    col = col - (mv - col) * (w * saturation);
    if (!unbound)
        col = clamp(col, float3(0.0), float3(1.0));
    return col;
}

// The per-pixel function. `vignetted` is whether the weight is above 0, found before
// the differentiable part, so the derivative passes get darktable's gate as an argument
// rather than recomputing it: inside the inner radius the pixel is copied, unclipped.
[Differentiable]
float3 vignette(no_diff float3 rgb, no_diff float2 position, no_diff bool vignetted, Geometry g, float brightness, float saturation, no_diff bool unbound)
{
    if (!vignetted)
        return rgb;
    return shade(rgb, weight(position, g), brightness, saturation, unbound);
}

// Setup and per-pixel function together, for the gradient with respect to darktable's
// parameters through both.
[Differentiable]
float3 vignette_from_settings(no_diff float3 rgb, no_diff float2 position, no_diff bool vignetted, Vignette v, no_diff Frame f)
{
    return vignette(rgb, position, vignetted, geometry(v, f), v.brightness, v.saturation, f.unbound);
}

// The invocation's pixel in the whole image at the pipe's scale.
float2 position_of(uint3 id)
{
    return float2(float(params.pass.origin_x + id.x), float(params.pass.origin_y + id.y));
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    Vignette v = vignette_of(params.vignette);
    Frame f = frame_of(params);
    Geometry g = geometry(v, f);
    float2 position = position_of(id);
    bool vignetted = weight(position, g) > 0.0;
    float4 pixel = input[i];
    float3 rgb = pixel.rgb;
    for (uint k = 0; k < params.pass.repeat; k++)
        rgb = vignette(rgb, position, vignetted, g, v.brightness, v.saturation, f.unbound);
    output[i] = float4(rgb, pixel.a);
}

// Derivative entry points, both per pixel, with parameters ordered as the gradient slots
// (gradient's x, y, z and w), each one of darktable's eight continuous parameters. With
// the automatic ratio, a slot naming whratio stands for nothing and gets 0.
[[vk::binding(8, 0)]] StructuredBuffer<float4> derivative_in;
[[vk::binding(9, 0)]] RWStructuredBuffer<float4> derivative_out;

// One parameter's value in a derivative of the parameters.
float pick(Vignette.Differential d, uint which)
{
    switch (which)
    {
    case 0: return d.scale;
    case 1: return d.falloff_scale;
    case 2: return d.brightness;
    case 3: return d.saturation;
    case 4: return d.center.x;
    case 5: return d.center.y;
    case 6: return d.whratio;
    case 7: return d.shape;
    default: return 0.0;
    }
}

// Reverse mode: given dLoss/dOutput, the gradient of the loss with respect to the
// slots' parameters.
[shader("compute")]
[numthreads(16, 16, 1)]
void vignette_backward(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    VignetteParams p = params.vignette;
    Vignette v = vignette_of(p);
    Frame f = frame_of(params);
    float2 position = position_of(id);
    bool vignetted = weight(position, geometry(v, f)) > 0.0;
    var d = diffPair(v);
    bwd_diff(vignette_from_settings)(input[i].rgb, position, vignetted, d, f, derivative_in[i].rgb);
    derivative_out[i] = float4(
        pick(d.d, p.gradient.x),
        pick(d.d, p.gradient.y),
        pick(d.d, p.gradient.z),
        pick(d.d, p.gradient.w));
}

// Adds x to the parameter `which` in a direction.
void add_slot(inout Vignette.Differential d, uint which, float x)
{
    d.scale += which == 0 ? x : 0.0;
    d.falloff_scale += which == 1 ? x : 0.0;
    d.brightness += which == 2 ? x : 0.0;
    d.saturation += which == 3 ? x : 0.0;
    d.center.x += which == 4 ? x : 0.0;
    d.center.y += which == 5 ? x : 0.0;
    d.whratio += which == 6 ? x : 0.0;
    d.shape += which == 7 ? x : 0.0;
}

// Forward mode: given a direction in the slots' parameters, the directional derivative
// of the output. Checks the reverse mode: for any direction v and output weights w,
// w · (J v) must equal (Jᵀ w) · v.
[shader("compute")]
[numthreads(16, 16, 1)]
void vignette_forward_diff(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    VignetteParams p = params.vignette;
    Vignette v = vignette_of(p);
    Frame f = frame_of(params);
    float2 position = position_of(id);
    bool vignetted = weight(position, geometry(v, f)) > 0.0;
    float4 direction_in = derivative_in[i];
    Vignette.Differential direction;
    direction.scale = 0.0;
    direction.falloff_scale = 0.0;
    direction.brightness = 0.0;
    direction.saturation = 0.0;
    direction.center = float2(0.0);
    direction.whratio = 0.0;
    direction.shape = 0.0;
    add_slot(direction, p.gradient.x, direction_in.x);
    add_slot(direction, p.gradient.y, direction_in.y);
    add_slot(direction, p.gradient.z, direction_in.z);
    add_slot(direction, p.gradient.w, direction_in.w);
    let result = fwd_diff(vignette_from_settings)(input[i].rgb, position, vignetted, diffPair(v, direction), f);
    derivative_out[i] = float4(result.d, 0.0);
}
