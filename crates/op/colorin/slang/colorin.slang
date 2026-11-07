// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/iop/colorin.c)
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

// Input colour profile, ported from darktable src/iop/colorin.c, process() through its
// matrix path without gamut clipping: camera RGB, with temperature's late correction
// applied, to the working space by one matrix, camera to XYZ D50 to working RGB.
// darktable goes through Lab between the two, which only rounds. It has no continuous
// parameters, so no derivatives.

import pass;

struct ColorinParams
{
    // Camera RGB to working RGB, by rows.
    float4 rows[3];
    // darktable's late correction: the camera's D65 multipliers over its as-shot ones,
    // or 1.
    float4 correction;
};

struct Params
{
    PassHeader pass;
    ColorinParams colorin;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;

// The per-pixel function.
float3 colorin(float3 rgb, ColorinParams p)
{
    float3 camera = rgb * p.correction.xyz;
    return float3(dot(p.rows[0].xyz, camera), dot(p.rows[1].xyz, camera), dot(p.rows[2].xyz, camera));
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
        rgb = colorin(rgb, params.colorin);
    output[i] = float4(rgb, pixel.a);
}
