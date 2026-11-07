// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2015-2025  darktable developers (ported from src/iop/rawprepare.c)
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

// Raw black and white points, ported from darktable src/iop/rawprepare.c, process():
// crops the sensor's 16-bit mosaic to the developed area and scales each photosite to
// (raw - black) / (white - black), with the black level of its position in the 2×2
// pattern of row and column parities on the sensor. The raw stage exports no
// derivatives (decision 21). It may run in row strips over windows of its mosaics
// (decision 14): photosites are found by their place in the whole mosaics, less the
// windows' starts in the pass header.

import pass;

struct RawprepareParams
{
    // The crop's top-left corner on the sensor.
    uint left;
    uint top;
    // The sensor's width: photosites per row of the input.
    uint sensor_width;
    uint padding;
    // Per parity position, (row & 1) * 2 + (column & 1) on the sensor: the black
    // level, and the white point less the black level.
    float4 sub;
    float4 div;
};

struct Params
{
    PassHeader pass;
    RawprepareParams rawprepare;
};

// The whole sensor's mosaic, two photosites to a word, the even one in the low half;
// from word params.pass.input_base on.
[[vk::binding(0, 0)]] StructuredBuffer<uint> input;
// From word params.pass.output_base on.
[[vk::binding(1, 0)]] RWStructuredBuffer<float> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;

// The photosite at (x, y) on the sensor.
float photosite(uint x, uint y)
{
    uint i = y * params.rawprepare.sensor_width + x;
    uint word = input[i / 2 - params.pass.input_base];
    return float((i & 1) == 0 ? (word & 0xffff) : (word >> 16));
}

// The per-pixel function.
float rawprepare(float raw, float sub, float div)
{
    return (raw - sub) / div;
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    RawprepareParams p = params.rawprepare;
    // The photosite in the output, then on the sensor.
    uint col = params.pass.origin_x + id.x;
    uint row = params.pass.origin_y + id.y;
    uint x = col + p.left;
    uint y = row + p.top;
    uint position = ((y & 1) << 1) | (x & 1);
    float value = photosite(x, y);
    for (uint k = 0; k < params.pass.repeat; k++)
        value = rawprepare(value, p.sub[position], p.div[position]);
    output[row * params.pass.width + col - params.pass.output_base] = value;
}
