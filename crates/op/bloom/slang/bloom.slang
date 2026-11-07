// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/iop/bloom.c)
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

// Bloom, ported from darktable src/iop/bloom.c, process(): the Lab lightness scaled and
// thresholded into lights, the lights blurred by darktable's box mean, eight times along
// the rows and then the columns, and the blur screen-blended onto the lightness. See
// docs/ports/bloom.md; `prepare` in src/setup.rs fills the parameters and lists the
// passes. darktable's bloom is a Lab module: the passes that read the input convert the
// working RGB to Lab themselves, and `main` converts back. Only lightness changes.
//
// The lights live in two scratch buffers laid out as the whole image at the pipe's
// scale, one float a pixel: `threshold` writes them into A, and each box pass reads one
// buffer and writes the other, A to B along the rows, B to A along the columns, so the
// blur ends in A, which `main` reads. Binding 10 is the buffer a pass reads, 11 the one
// it writes.

import box_filter;
import color;
import pass;

// ln 2, for the strength's derivative.
static const float LN2 = 0.69314718055994530942;

struct BloomParams
{
    // x, y: the whole image's width and height at the pipe's scale, the scratch's
    // layout; z: the box mean's radius; w: the axis a box pass runs along, 0 the rows,
    // 1 the columns.
    uint4 image;
    // x: the lights' scale, 2^(min(100, strength + 1) / 100); y: the threshold, in
    // lightness; z: the strength, for main's derivatives.
    float4 lights;
    // Working RGB to XYZ D50 and back, by rows.
    float4 rgb_to_xyz_r0;
    float4 rgb_to_xyz_r1;
    float4 rgb_to_xyz_r2;
    float4 xyz_to_rgb_r0;
    float4 xyz_to_rgb_r1;
    float4 xyz_to_rgb_r2;
};

struct Params
{
    PassHeader pass;
    BloomParams bloom;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;
[[vk::binding(10, 0)]] RWStructuredBuffer<float> scratch_in;
[[vk::binding(11, 0)]] RWStructuredBuffer<float> scratch_out;

struct Colour
{
    float3x3 rgb_to_xyz;
    float3x3 xyz_to_rgb;
};

Colour colour_of(BloomParams p)
{
    Colour c;
    c.rgb_to_xyz = float3x3(p.rgb_to_xyz_r0.xyz, p.rgb_to_xyz_r1.xyz, p.rgb_to_xyz_r2.xyz);
    c.xyz_to_rgb = float3x3(p.xyz_to_rgb_r0.xyz, p.xyz_to_rgb_r1.xyz, p.xyz_to_rgb_r2.xyz);
    return c;
}

// darktable's working RGB to Lab and back around a Lab module
// (_transform_rgb_to_lab_matrix and _transform_lab_to_rgb_matrix).
float3 rgb_to_lab(float3 rgb, Colour c)
{
    return xyz_to_lab_d50(mul(c.rgb_to_xyz, rgb));
}

[Differentiable]
float3 lab_to_rgb(float3 lab, no_diff Colour c)
{
    return mul(c.xyz_to_rgb, lab_to_xyz_d50(lab));
}

// The pixel's index in the scratch, laid out as the whole image: its place there is the
// pass's origin plus its coordinates, so a tile of an export reads and writes its own
// part (decision 15).
uint scratch_index(uint3 id, BloomParams p)
{
    return (params.pass.origin_y + id.y) * p.image.x + params.pass.origin_x + id.x;
}

// A guard, not darktable's: the largest light a pixel adds. The box passes keep running
// sums along whole lines, so one infinite light would make every later value of its row
// infinite, and the column passes, subtracting it again, NaN, over the whole image; the
// operation's boundary bounds it instead (docs/ports/bloom.md, Precision). A lightness of
// 10⁶ is far past anything an image reaches: 10⁶ in linear RGB is about 1.2 · 10⁴.
static const float MAX_LIGHT = 1e6;

// darktable process()'s first loop: the lightness times the scale, kept where it passes
// the threshold, 0 elsewhere, and bounded by the guard.
[shader("compute")]
[numthreads(16, 16, 1)]
void threshold(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    BloomParams p = params.bloom;
    float lightness = rgb_to_lab(input[id.y * params.pass.width + id.x].rgb, colour_of(p)).x;
    float l = min(lightness * p.lights.x, MAX_LIGHT);
    scratch_out[scratch_index(id, p)] = l > p.lights.y ? l : 0.0;
}

// A row or a column of the lights, read from binding 10 and written to binding 11.
struct Line : IBoxLine<float>
{
    uint start;
    uint stride;

    float read(uint i)
    {
        return scratch_in[start + i * stride];
    }

    void write(uint i, float value)
    {
        scratch_out[start + i * stride] = value;
    }
};

// One of dt_box_mean's passes along the rows or the columns of the whole image, one
// invocation per line, in rows of 16: the pass's size is 16 × the lines over 16.
[shader("compute")]
[numthreads(16, 16, 1)]
void box_mean(uint3 id: SV_DispatchThreadID)
{
    BloomParams p = params.bloom;
    uint width = p.image.x;
    uint height = p.image.y;
    bool rows = p.image.w == 0;
    uint index = id.y * 16 + id.x;
    if (index >= (rows ? height : width))
        return;
    Line line;
    line.start = rows ? index * width : index;
    line.stride = rows ? 1 : width;
    box_mean_line<float, Line>(line, rows ? width : height, p.image.z);
}

// What the derivative entry points differentiate against.
struct Settings : IDifferentiable
{
    float strength;
};

Settings settings_of(BloomParams p)
{
    Settings s;
    s.strength = p.lights.z;
    return s;
}

// The blurred lights at the pixel, `blurred`, read from the scratch the render left, as
// a function of the strength for the derivatives. The strength scales the lights by
// 2^(min(100, strength + 1) / 100), and the box mean is linear, so with the pixels past
// the threshold held, the blur's derivative is blurred · ln 2 / 100, and 0 where the
// strength is clamped. Pixels crossing the threshold are not seen
// (docs/ports/bloom.md, Gradients).
[ForwardDerivative(lights_forward)]
[BackwardDerivative(lights_backward)]
float lights(no_diff float blurred, float strength)
{
    return blurred;
}

float lights_slope(float blurred, float strength)
{
    return strength + 1.0 < 100.0 ? blurred * (LN2 / 100.0) : 0.0;
}

DifferentialPair<float> lights_forward(float blurred, DifferentialPair<float> strength)
{
    return diffPair(blurred, lights_slope(blurred, strength.p) * strength.d);
}

void lights_backward(float blurred, inout DifferentialPair<float> strength, float d_out)
{
    strength = diffPair(strength.p, lights_slope(blurred, strength.p) * d_out);
}

// The per-pixel function, darktable process()'s second loop: the blurred lights
// screen-blended onto the lightness; a and b stay.
[Differentiable]
float3 bloom(no_diff float3 lab, no_diff float blurred, no_diff Colour c, Settings s)
{
    float blur = lights(blurred, s.strength);
    float l = 100.0 - ((100.0 - lab.x) * (100.0 - blur)) / 100.0;
    return lab_to_rgb(float3(l, lab.y, lab.z), c);
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    BloomParams p = params.bloom;
    uint i = id.y * params.pass.width + id.x;
    Colour c = colour_of(p);
    Settings s = settings_of(p);
    float blurred = scratch_in[scratch_index(id, p)];
    float4 pixel = input[i];
    float3 rgb = pixel.rgb;
    for (uint k = 0; k < params.pass.repeat; k++)
        rgb = bloom(rgb_to_lab(rgb, c), blurred, c, s);
    output[i] = float4(rgb, pixel.a);
}

// Derivative entry points of `main`, per pixel, over the scratch the forward render left
// (decision 15). Bloom declares no gradient slot (decision 11): component 0 carries
// `main`'s derivative with respect to the strength with the pixels past the threshold
// held, which is not the whole render's, and the others are 0. They bind what `main`
// binds but its output.
[[vk::binding(8, 0)]] StructuredBuffer<float4> derivative_in;
[[vk::binding(9, 0)]] RWStructuredBuffer<float4> derivative_out;

// Reverse mode: given dLoss/dOutput, the gradient of the loss with respect to the
// strength.
[shader("compute")]
[numthreads(16, 16, 1)]
void bloom_backward(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    BloomParams p = params.bloom;
    uint i = id.y * params.pass.width + id.x;
    Colour c = colour_of(p);
    float3 lab = rgb_to_lab(input[i].rgb, c);
    float blurred = scratch_in[scratch_index(id, p)];
    var s = diffPair(settings_of(p));
    bwd_diff(bloom)(lab, blurred, c, s, derivative_in[i].rgb);
    derivative_out[i] = float4(s.d.strength, 0.0, 0.0, 0.0);
}

// Forward mode: given a direction in the strength, the directional derivative of the
// output. Checks the reverse mode.
[shader("compute")]
[numthreads(16, 16, 1)]
void bloom_forward_diff(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    BloomParams p = params.bloom;
    uint i = id.y * params.pass.width + id.x;
    Colour c = colour_of(p);
    float3 lab = rgb_to_lab(input[i].rgb, c);
    float blurred = scratch_in[scratch_index(id, p)];
    Settings.Differential direction;
    direction.strength = derivative_in[i].x;
    let result = fwd_diff(bloom)(lab, blurred, c, diffPair(settings_of(p), direction));
    derivative_out[i] = float4(result.d, 0.0);
}
