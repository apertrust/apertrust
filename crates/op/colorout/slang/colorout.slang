// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/iop/colorout.c)
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

// Output colour profile, ported from darktable src/iop/colorout.c, process() through its
// matrix path: working RGB to the output profile's RGB by one matrix, through XYZ D50,
// then the profile's tone curve, sRGB's or none. darktable reaches it from Lab, which
// only rounds, and evaluates the curve from a table below 1. It has no continuous
// parameters, so no derivatives.

import pass;

struct ColoroutParams
{
    // Working RGB to the output's linear RGB, by rows.
    float4 rows[3];
    // 1 for sRGB's tone curve, 0 for none.
    uint curve;
};

struct Params
{
    PassHeader pass;
    ColoroutParams colorout;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;

// sRGB's tone curve, darktable's parameters (2.4, 1/1.055, 0.055/1.055, 1/12.92,
// 0.04045) inverted: negative values clip to 0, as darktable's table does.
float srgb(float v)
{
    float x = max(v, 0.0);
    return x <= 0.0031308 ? 12.92 * x : 1.055 * pow(x, 1.0 / 2.4) - 0.055;
}

// The per-pixel function.
float3 colorout(float3 rgb, ColoroutParams p)
{
    float3 linear = float3(dot(p.rows[0].xyz, rgb), dot(p.rows[1].xyz, rgb), dot(p.rows[2].xyz, rgb));
    if (p.curve == 0)
        return linear;
    return float3(srgb(linear.x), srgb(linear.y), srgb(linear.z));
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    float4 pixel = input[i];
    float3 rgb = pixel.rgb;
    for (uint k = 0; k < params.pass.repeat; k++)
        rgb = colorout(rgb, params.colorout);
    output[i] = float4(rgb, pixel.a);
}
