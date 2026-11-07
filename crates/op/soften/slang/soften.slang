// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2011-2026  darktable developers (ported from src/iop/soften.c)
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

// Soften, ported from darktable src/iop/soften.c, process(): each pixel's HSL saturation
// and lightness scaled and clipped (the "overexposed" image), that blurred by
// darktable's box mean, eight times along the rows and then the columns, and mixed into
// the input. See docs/ports/soften.md; `prepare` in src/setup.rs fills the parameters
// and lists the passes.
//
// The softened image lives in scratch laid out as the whole image at the pipe's scale,
// in planes of one float a pixel, red, green and blue, so each binds as little as
// bloom's lights: `overexpose` writes it into A, and each box pass reads one image and
// writes the other, A to B along the rows, B to A along the columns, so the blur ends
// in A, which `main` reads. Bindings 10 to 12 are the planes a pass reads, 13 to 15
// those it writes. darktable's blur has a fourth channel, its alpha, always 0, which no
// output reads.

import box_filter;
import pass;
import common.hsl;

struct SoftenParams
{
    // x, y: the whole image's width and height at the pipe's scale, the scratch's
    // layout; z: the box mean's radius; w: the axis a box pass runs along, 0 the rows,
    // 1 the columns.
    uint4 image;
    // x: the saturation's factor, saturation / 100; y: the lightness's, 1 / 2^-brightness;
    // z: the mix, amount / 100, for main.
    float4 factors;
};

struct Params
{
    PassHeader pass;
    SoftenParams soften;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;
[[vk::binding(10, 0)]] RWStructuredBuffer<float> read_r;
[[vk::binding(11, 0)]] RWStructuredBuffer<float> read_g;
[[vk::binding(12, 0)]] RWStructuredBuffer<float> read_b;
[[vk::binding(13, 0)]] RWStructuredBuffer<float> write_r;
[[vk::binding(14, 0)]] RWStructuredBuffer<float> write_g;
[[vk::binding(15, 0)]] RWStructuredBuffer<float> write_b;

// The pixel's index in the scratch, laid out as the whole image: its place there is the
// pass's origin plus its coordinates, so a tile of an export reads and writes its own
// part (decision 15).
uint scratch_index(uint3 id, SoftenParams p)
{
    return (params.pass.origin_y + id.y) * p.image.x + params.pass.origin_x + id.x;
}

float3 read_blur(uint i)
{
    return float3(read_r[i], read_g[i], read_b[i]);
}

// A guard, not darktable's: each channel bounded before the HSL conversion, so an
// infinite pixel gives a finite softened pixel. One that was not would make every later
// value of its row infinite in the box passes' running sums, and the column passes,
// subtracting it again, NaN, over the whole image, at every mix, 0 included. No colour
// a photo has comes near it, and past about 2 / brightness a pixel's lightness clips to
// 1 anyway (docs/ports/soften.md, Precision).
static const float MAX_INPUT = 1e6;

// darktable process()'s first loop: the HSL saturation times its factor and the
// lightness times its own, each clipped to [0, 1] (darktable's CLIP), and back to RGB.
[shader("compute")]
[numthreads(16, 16, 1)]
void overexpose(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    SoftenParams p = params.soften;
    float3 rgb = clamp(input[id.y * params.pass.width + id.x].rgb, -MAX_INPUT, MAX_INPUT);
    float3 hsl = rgb_to_hsl(rgb);
    float s = clamp(hsl.y * p.factors.x, 0.0, 1.0);
    float l = clamp(hsl.z * p.factors.y, 0.0, 1.0);
    float3 softened = hsl_to_rgb(float3(hsl.x, s, l));
    uint i = scratch_index(id, p);
    write_r[i] = softened.r;
    write_g[i] = softened.g;
    write_b[i] = softened.b;
}

// A row or a column of the softened image, read from bindings 10 to 12 and written to
// 13 to 15. darktable's channels are independent, each its own running sum.
struct Line : IBoxLine<float3>
{
    uint start;
    uint stride;

    float3 read(uint i)
    {
        return read_blur(start + i * stride);
    }

    void write(uint i, float3 value)
    {
        uint k = start + i * stride;
        write_r[k] = value.r;
        write_g[k] = value.g;
        write_b[k] = value.b;
    }
};

// One of dt_box_mean's passes along the rows or the columns of the whole image, one
// invocation per line, in rows of 16: the pass's size is 16 × the lines over 16.
[shader("compute")]
[numthreads(16, 16, 1)]
void box_mean(uint3 id: SV_DispatchThreadID)
{
    SoftenParams p = params.soften;
    uint width = p.image.x;
    uint height = p.image.y;
    bool rows = p.image.w == 0;
    uint index = id.y * 16 + id.x;
    if (index >= (rows ? height : width))
        return;
    Line line;
    line.start = rows ? index * width : index;
    line.stride = rows ? 1 : width;
    box_mean_line<float3, Line>(line, rows ? width : height, p.image.z);
}

// What the derivative entry points differentiate against: the mix as a fraction,
// amount / 100. The entry points report the derivative with respect to darktable's
// amount, in percent, a hundredth of it.
struct Mix : IDifferentiable
{
    float mix;
};

Mix mix_of(SoftenParams p)
{
    Mix m;
    m.mix = p.factors.z;
    return m;
}

// The per-pixel function, darktable's dt_iop_image_linear_blend: the blurred image
// times the mix plus the input times one less the mix.
[Differentiable]
float3 soften(no_diff float3 rgb, no_diff float3 blur, Mix m)
{
    return m.mix * blur + (1.0 - m.mix) * rgb;
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    SoftenParams p = params.soften;
    uint i = id.y * params.pass.width + id.x;
    float3 blur = read_blur(scratch_index(id, p));
    Mix m = mix_of(p);
    float4 pixel = input[i];
    float3 rgb = pixel.rgb;
    for (uint k = 0; k < params.pass.repeat; k++)
        rgb = soften(rgb, blur, m);
    output[i] = float4(rgb, pixel.a);
}

// Derivative entry points of `main`, per pixel, over the scratch the forward render left
// (decision 15). Component 0 is the amount's, which only `main` reads, so it is the
// whole render's (decision 11); the others are 0. They bind what `main` binds but its
// output.
[[vk::binding(8, 0)]] StructuredBuffer<float4> derivative_in;
[[vk::binding(9, 0)]] RWStructuredBuffer<float4> derivative_out;

// Reverse mode: given dLoss/dOutput, the gradient of the loss with respect to the
// amount.
[shader("compute")]
[numthreads(16, 16, 1)]
void soften_backward(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    SoftenParams p = params.soften;
    uint i = id.y * params.pass.width + id.x;
    float3 blur = read_blur(scratch_index(id, p));
    var m = diffPair(mix_of(p));
    bwd_diff(soften)(input[i].rgb, blur, m, derivative_in[i].rgb);
    derivative_out[i] = float4(m.d.mix / 100.0, 0.0, 0.0, 0.0);
}

// Forward mode: given a direction in the amount, the directional derivative of the
// output. Checks the reverse mode.
[shader("compute")]
[numthreads(16, 16, 1)]
void soften_forward_diff(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    SoftenParams p = params.soften;
    uint i = id.y * params.pass.width + id.x;
    float3 blur = read_blur(scratch_index(id, p));
    Mix.Differential direction;
    direction.mix = derivative_in[i].x / 100.0;
    let result = fwd_diff(soften)(input[i].rgb, blur, diffPair(mix_of(p), direction));
    derivative_out[i] = float4(result.d, 0.0);
}
