// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
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

// A tile's input (decision 13): the rectangle of the source image the header's size
// covers, from (x, y), copied into a buffer of its own, `channels` floats per pixel: 1
// for a mosaic, 4 for RGBA. The engine binds only the rows of the source it needs, a
// window from word params.pass.input_base on, and may copy in row strips, each from the
// header's origin (decision 14). The engine's own pass; not a darktable port: no golden
// case and no derivatives.

import pass;

struct RegionParams
{
    uint source_width;
    uint x;
    uint y;
    uint channels;
};

struct Params
{
    PassHeader pass;
    RegionParams region;
};

[[vk::binding(0, 0)]] StructuredBuffer<float> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    RegionParams r = params.region;
    uint row = params.pass.origin_y + id.y;
    uint from = ((r.y + row) * r.source_width + r.x + id.x) * r.channels - params.pass.input_base;
    uint to = (row * params.pass.width + id.x) * r.channels - params.pass.output_base;
    for (uint c = 0; c < r.channels; c++)
        output[to + c] = input[from + c];
}
