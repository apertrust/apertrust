// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/common/histogram.c)
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

// The raw's histogram (decision 87, docs/ports/exposure.md): how many photosites of the
// decoded mosaic hold each 16-bit value, over a region of the sensor, as darktable's
// deflicker counts it (src/common/histogram.c, _bin_raw): every photosite in the region,
// whatever its filter colour, its value its own bin, before rawprepare. The engine's own
// pass, outside the plan, counted once per photo and read back once: no golden case and
// no derivatives.
//
// Counts are integers, so atomics add them, straight into the counts buffer, one 32-bit
// word per value, 65536 of them, which the engine clears before the pass; 256 KiB is more
// than workgroup memory holds. It uses none.
//
// It reads the mosaic as the engine uploads it, two photosites to a 32-bit word, the even
// one of the readout in the low half, in row windows where the mosaic is larger than one
// binding (decision 14): the header's origin is the region's left column and the window's
// first row of the region, its full width the readout's width, and its input_base the
// window's first word. Each invocation counts PER photosites of one row.

import pass;

static const uint PER = 4;

struct RawHistogramParams
{
    // The region's right edge, the first column after it.
    uint right;
    uint _pad0;
    uint _pad1;
    uint _pad2;
};

struct Params
{
    PassHeader pass;
    RawHistogramParams histogram;
};

[[vk::binding(0, 0)]] StructuredBuffer<uint> mosaic;
[[vk::binding(1, 0)]] RWStructuredBuffer<Atomic<uint>> counts;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;

// The 16-bit value of photosite `index` of the readout, in its row-major order.
uint value_at(uint index, uint base)
{
    uint word = mosaic[(index >> 1) - base];
    return (index & 1) == 0 ? word & 0xffff : word >> 16;
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    PassHeader h = params.pass;
    if (id.x >= h.width || id.y >= h.height)
        return;
    uint y = h.origin_y + id.y;
    uint first = h.origin_x + id.x * PER;
    for (uint k = 0; k < PER; k++)
    {
        uint x = first + k;
        if (x < params.histogram.right)
            counts[value_at(y * h.full_width + x, h.input_base)].add(1);
    }
}
