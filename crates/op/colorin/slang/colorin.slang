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
// matrix paths: camera RGB, with temperature's late correction applied, to the working
// space. Without gamut clipping (`_cmatrix_fastpath_simple`) one matrix, camera to XYZ
// D50 to working RGB; darktable goes through Lab between the two, which only rounds. With
// it (`_cmatrix_fastpath_clipping`), camera RGB to the clipping profile's linear RGB,
// each channel clamped to 0 to 1, then on to the working space. With blue mapping
// (`_process_cmatrix_bm`), `_apply_blue_mapping` on camera RGB first, and no late
// correction, as darktable's CPU path. It has no continuous parameters, so no
// derivatives, and its clamp is darktable's.

import pass;

struct ColorinParams
{
    // Camera RGB to working RGB, by rows; with gamut clipping, camera RGB to the clipping
    // profile's linear RGB.
    float4 rows[3];
    // With gamut clipping, the clipping profile's linear RGB to working RGB, by rows.
    float4 clipped[3];
    // darktable's late correction: the camera's D65 multipliers over its as-shot ones,
    // or 1.
    float4 correction;
    // x: gamut clipping, y: blue mapping; 1 on, 0 off.
    uint4 mode;
};

struct Params
{
    PassHeader pass;
    ColorinParams colorin;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;

float3 apply(float4 rows[3], float3 rgb)
{
    return float3(dot(rows[0].xyz, rgb), dot(rows[1].xyz, rgb), dot(rows[2].xyz, rgb));
}

// darktable `_apply_blue_mapping`: where blue is more than half of the three channels'
// sum, part of it moves to green, up to 0.11, less for dark pixels.
float3 blue_mapping(float3 rgb)
{
    const float sum = rgb.r + rgb.g + rgb.b;
    if (sum > 0.0)
    {
        const float z = rgb.b / sum;
        if (z > 0.5)
        {
            const float t = (z - 0.5) / (1.0 - 0.5) * min(1.0, sum / 0.5);
            rgb.g += t * 0.11;
            rgb.b -= t * 0.11;
        }
    }
    return rgb;
}

// The per-pixel function.
float3 colorin(float3 rgb, ColorinParams p)
{
    float3 camera = rgb * p.correction.xyz;
    if (p.mode.y != 0)
        camera = blue_mapping(camera);
    float3 working = apply(p.rows, camera);
    if (p.mode.x != 0)
        working = apply(p.clipped, clamp(working, 0.0, 1.0));
    return working;
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
