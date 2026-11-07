// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2011-2025  darktable developers (ported from src/iop/flip.c)
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

// Orientation, ported from darktable src/iop/flip.c, process(), which turns its input
// region with dt_imageio_flip_buffers (src/imageio/imageio.c): each pixel (i, j) of the
// input region, iw by ih, goes to (iw - 1 - i) where the columns flip and (ih - 1 - j)
// where the rows flip, its two coordinates then swapped where the axes swap. The input
// region is exactly the output region turned back (modify_roi_in), so the turn needs no
// position: each output pixel copies the one input pixel the inverse map names, alpha
// included, with no interpolation. It has no continuous parameter, so it exports no
// derivatives.

import pass;

// darktable's dt_image_orientation_t flip bits.
static const uint FLIP_Y = 1;
static const uint FLIP_X = 2;
static const uint SWAP_XY = 4;

struct FlipParams
{
    // The input region's size.
    uint input_width;
    uint input_height;
    // The flip bits, 1 to 7.
    uint orientation;
    uint reserved;
};

struct Params
{
    PassHeader pass;
    FlipParams flip;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;

// The input region's pixel that output pixel `xy` of the output region shows: the
// inverse of dt_imageio_flip_buffers' map, the axes swapped back, then the flips undone.
uint2 source(uint2 xy, FlipParams p)
{
    uint2 at = (p.orientation & SWAP_XY) != 0 ? xy.yx : xy;
    if ((p.orientation & FLIP_X) != 0)
        at.x = p.input_width - 1 - at.x;
    if ((p.orientation & FLIP_Y) != 0)
        at.y = p.input_height - 1 - at.y;
    return at;
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    FlipParams p = params.flip;
    uint2 from = source(id.xy, p);
    output[id.y * params.pass.width + id.x] = input[from.y * p.input_width + from.x];
}
