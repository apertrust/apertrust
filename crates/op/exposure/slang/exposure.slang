// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/iop/exposure.c)
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

// Exposure, ported from darktable src/iop/exposure.c, process():
// out = (in - black) · scale on RGB, with scale = 1 / (2^-exposure - black); alpha
// passes through.

import pass;

struct ExposureParams : IDifferentiable
{
    float black;
    float exposure;
};

struct Params
{
    PassHeader pass;
    ExposureParams exposure;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;

// Setup, once per image: the scale applied to every pixel.
[Differentiable]
float exposure_setup(ExposureParams p)
{
    return 1.0 / (exp2(-p.exposure) - p.black);
}

// The per-pixel function.
[Differentiable]
float3 exposure(float3 rgb, ExposureParams p, float scale)
{
    return (rgb - p.black) * scale;
}

// Setup and per-pixel function together, for the gradient with respect to the
// parameters through both.
[Differentiable]
float3 exposure_from_params(no_diff float3 rgb, ExposureParams p)
{
    return exposure(rgb, p, exposure_setup(p));
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    ExposureParams p = params.exposure;
    float scale = exposure_setup(p);
    float4 pixel = input[i];
    float3 rgb = pixel.rgb;
    for (uint k = 0; k < params.pass.repeat; k++)
        rgb = exposure(rgb, p, scale);
    output[i] = float4(rgb, pixel.a);
}

// Derivative entry points, both per pixel, with parameters ordered (black, exposure).
[[vk::binding(8, 0)]] StructuredBuffer<float4> derivative_in;
[[vk::binding(9, 0)]] RWStructuredBuffer<float4> derivative_out;

// Reverse mode: given dLoss/dOutput, the gradient of the loss with respect to the
// parameters.
[shader("compute")]
[numthreads(16, 16, 1)]
void exposure_backward(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    var p = diffPair(params.exposure);
    bwd_diff(exposure_from_params)(input[i].rgb, p, derivative_in[i].rgb);
    derivative_out[i] = float4(p.d.black, p.d.exposure, 0.0, 0.0);
}

// Forward mode: given a direction in parameter space, the directional derivative of the
// output. Checks the reverse mode: for any direction v and output weights w,
// w · (J v) must equal (Jᵀ w) · v.
[shader("compute")]
[numthreads(16, 16, 1)]
void exposure_forward_diff(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    ExposureParams.Differential direction;
    direction.black = derivative_in[i].x;
    direction.exposure = derivative_in[i].y;
    let result = fwd_diff(exposure_from_params)(input[i].rgb, diffPair(params.exposure, direction));
    derivative_out[i] = float4(result.d, 0.0);
}
