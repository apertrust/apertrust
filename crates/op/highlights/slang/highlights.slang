// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/iop/highlights.c,
// src/iop/hlreconstruct/opposed.c and src/iop/hlreconstruct/segbased.c)
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

// Highlight reconstruction (darktable's highlights), inpaint opposed, ported from
// darktable src/iop/hlreconstruct/opposed.c, _process_opposed(), with _calc_refavg()
// from src/iop/hlreconstruct/segbased.c. A clipped photosite becomes the cube of the
// mean cube roots of the two opposing colours around it, plus the image's chrominance
// offset for its colour, if that is brighter. The offset is a reduction: the mean
// difference between the photosites and their opposing average, over the unclipped
// photosites close to clipped ones. Five passes:
//
// 1. opposed_mask: per 3×3 superpixel, which colours have a clipped photosite;
// 2. opposed_dilate: those masks dilated by about 3 superpixels;
// 3. opposed_sums: per workgroup, the sums and counts over the photosites the dilated
//    masks select;
// 4. opposed_chroma: one workgroup adds the workgroups' sums up into the offsets;
// 5. opposed_inpaint: every photosite, clipped ones reconstructed.
//
// The mask passes and the sums follow darktable's CPU loops' bounds and its masks'
// memory layout exactly. The raw stage exports no derivatives (decision 21).
//
// The passes that read the mosaic may run in row strips over windows of it (decision
// 14): photosites are found by their place in the whole mosaic, less the windows'
// starts in the pass header, and each strip's first row of invocations is the header's
// origin. The sums' strips are whole rows of workgroups, which write their partial
// results at their place in the whole image, so the offsets are the whole image's. The
// dilation, which binds only the masks, whole, may run in bands of its rows, each
// finding its entries from the header's origin.

import components;
import pass;

struct HighlightsParams
{
    // The mosaic.
    uint width;
    uint height;
    // darktable mwidth and mheight: the superpixels' grid, a third of the mosaic.
    uint mask_width;
    uint mask_height;
    // darktable msize: the grid's width and height each rounded up to 8, multiplied.
    // The masks' entries past the grid stay 0.
    uint mask_size;
    // The dilation runs over mask_size entries, this many to a row: the grid's width
    // rounded up to 8.
    uint mask_stride;
    // How many workgroups the sums pass runs across, and in all.
    uint partials_across;
    uint partials;
    // The clipping level of each colour; w unused.
    float4 clips;
    // The late correction of each colour, or 1; w unused.
    float4 correction;
    // The colour of each position of a 6×6 tile from the mosaic's corner, four to a
    // vector: X-Trans's pattern, or Bayer's repeated.
    uint4 colors[9];
};

struct Params
{
    PassHeader pass;
    HighlightsParams highlights;
};

// A workgroup's sums of photosite minus opposing average, and how many photosites, per
// colour; w unused.
struct Partial
{
    float4 sums;
    uint4 counts;
};

// The mosaic from word params.pass.input_base on.
[[vk::binding(0, 0)]] StructuredBuffer<float> input;
// From word params.pass.output_base on.
[[vk::binding(1, 0)]] RWStructuredBuffer<float> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;
// Per superpixel, a bit per colour: whether it has a clipped photosite of that colour.
[[vk::binding(10, 0)]] RWStructuredBuffer<uint> clipped_mask;
// The same, dilated.
[[vk::binding(11, 0)]] RWStructuredBuffer<uint> dilated_mask;
[[vk::binding(12, 0)]] RWStructuredBuffer<Partial> partials;
// The chrominance offset of each colour; w unused.
[[vk::binding(13, 0)]] RWStructuredBuffer<float4> chroma;

static const uint WORKGROUP = 256;
// The sums pass's photosites per invocation, across and down.
static const uint BLOCK = 4;

groupshared float4 shared_sums[WORKGROUP];
groupshared uint4 shared_counts[WORKGROUP];

// darktable fcol(): the filter colour of the photosite at (col, row) of the mosaic.
uint color_at(uint col, uint row)
{
    uint position = (row % 6) * 6 + col % 6;
    return params.highlights.colors[position / 4][position % 4];
}

float clip_of(uint color)
{
    return params.highlights.clips[color];
}

// The photosite at (col, row) of the whole mosaic.
float mosaic_at(uint col, uint row)
{
    return input[row * params.highlights.width + col - params.pass.input_base];
}

// The C library's cbrtf for x >= 0: pow is a few ulp off, and a Newton step from its
// estimate is as close as cbrtf. 0 is answered first, so pow never sees log2(0).
float cube_root(float x)
{
    if (x <= 0.0)
        return 0.0;
    float y = pow(x, 1.0 / 3.0);
    return y - (y * y * y - x) / (3.0 * y * y);
}

// The cube root of colour c's corrected mean, or 0 without photosites of that colour.
float mean_root(uint c, float mean, float count)
{
    return count > 0.0 ? cube_root((params.highlights.correction[c] * mean) / count) : 0.0;
}

// darktable _calc_refavg(), linear: the cube of the mean cube roots of the two colours
// opposing the photosite's own, over the 3×3 around it. As in darktable, the window
// stops before the mosaic's last row and column.
float opposing_average(int col, int row)
{
    int width = int(params.highlights.width);
    int height = int(params.highlights.height);
    float3 mean = float3(0.0);
    float3 count = float3(0.0);
    for (int dy = max(0, row - 1); dy < min(height - 1, row + 2); dy++)
    {
        for (int dx = max(0, col - 1); dx < min(width - 1, col + 2); dx++)
        {
            float value = max(0.0, mosaic_at(uint(dx), uint(dy)));
            uint c = color_at(uint(dx), uint(dy));
            mean = add_at(mean, c, value);
            count = add_at(count, c, 1.0);
        }
    }
    float3 root = float3(mean_root(0, mean.x, count.x), mean_root(1, mean.y, count.y),
                         mean_root(2, mean.z, count.z));
    float3 opposing = float3(0.5 * (root[1] + root[2]), 0.5 * (root[0] + root[2]), 0.5 * (root[0] + root[1]));
    float average = opposing[color_at(uint(col), uint(row))];
    return average * average * average;
}

// The dilation's entry: invocation (x, y) of a grid mask_stride wide covers entry
// y * mask_stride + x of mask_size, y counted from the header's origin, so a band of its
// rows covers its own entries; the grid row and column of entry i are i / mask_width and
// i % mask_width, as darktable lays its masks out.
uint mask_entry(uint3 id)
{
    return (params.pass.origin_y + id.y) * params.highlights.mask_stride + id.x;
}

// Pass 1. darktable sets a colour's bit when the 3×3 superpixel has a photosite of that
// colour at or above its clipping level, leaving the grid's last row and column 0. It
// runs over the grid's rows of mask_width entries, so a row of invocations reads three
// rows of the mosaic and a strip of them a band of it; the rows past the grid fill the
// rest of mask_size with 0.
[shader("compute")]
[numthreads(16, 16, 1)]
void opposed_mask(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint mask_width = params.highlights.mask_width;
    uint i = (params.pass.origin_y + id.y) * mask_width + id.x;
    if (i >= params.highlights.mask_size)
        return;
    uint mrow = i / mask_width;
    uint mcol = i % mask_width;
    uint bits = 0;
    if (mrow + 1 < params.highlights.mask_height && mcol + 1 < mask_width)
    {
        for (uint y = 0; y < 3; y++)
        {
            for (uint x = 0; x < 3; x++)
            {
                uint row = 3 * mrow + y;
                uint col = 3 * mcol + x;
                uint color = color_at(col, row);
                if (mosaic_at(col, row) >= clip_of(color))
                    bits |= 1u << color;
            }
        }
    }
    clipped_mask[i] = bits;
}

// darktable _mask_dilated() for the three colours at once: the masks' bits of every
// entry within a 7×7 square without its corners around entry i, ORed, so a colour's bit
// is set where darktable finds any entry of its mask set. Each entry is read once, not
// once a colour.
uint dilated(uint i)
{
    int width = int(params.highlights.mask_width);
    uint bits = 0;
    for (int dy = -3; dy <= 3; dy++)
    {
        for (int dx = -3; dx <= 3; dx++)
        {
            if (abs(dx) == 3 && abs(dy) == 3)
                continue;
            bits |= clipped_mask[int(i) + dy * width + dx];
        }
    }
    return bits;
}

// Pass 2. Superpixels at least 3 from the grid's top and left and 4 from its bottom and
// right get the dilated masks; the others keep their own.
[shader("compute")]
[numthreads(16, 16, 1)]
void opposed_dilate(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = mask_entry(id);
    int mask_width = int(params.highlights.mask_width);
    int mask_height = int(params.highlights.mask_height);
    int mrow = int(i) / mask_width;
    int mcol = int(i) % mask_width;
    uint bits = 0;
    if (mrow < mask_height)
    {
        bool safe = mcol >= 3 && mrow >= 3 && mcol < mask_width - 4 && mrow < mask_height - 4;
        bits = (safe ? dilated(i) : clipped_mask[i]) & 7u;
    }
    dilated_mask[i] = bits;
}

// Whether colour's dilated mask is set at entry i. darktable keeps the three dilated
// masks one after another, mask_size apart, and the photosites of the mosaic's last
// rows and columns, beyond the grid, can read past one mask into the next.
bool dilated_at(uint i, uint color)
{
    uint size = params.highlights.mask_size;
    if (i >= size)
    {
        i -= size;
        color += 1;
    }
    return color < 3 && ((dilated_mask[i] >> color) & 1u) != 0;
}

// Pass 3. Each invocation adds up a 4×4 block of photosites, then the workgroup adds
// its invocations' sums. The photosites counted are those between a fifth of their
// colour's clipping level and the level, in their colour's dilated mask. A strip starts
// at a row of workgroups, so the header's origin is a multiple of 16.
[shader("compute")]
[numthreads(16, 16, 1)]
void opposed_sums(uint3 id: SV_DispatchThreadID, uint3 group: SV_GroupID, uint t: SV_GroupIndex)
{
    uint width = params.highlights.width;
    uint height = params.highlights.height;
    float4 sums = float4(0.0);
    uint4 counts = uint4(0);
    if (id.x < params.pass.width && id.y < params.pass.height)
    {
        for (uint y = 0; y < BLOCK; y++)
        {
            for (uint x = 0; x < BLOCK; x++)
            {
                uint row = BLOCK * (params.pass.origin_y + id.y) + y;
                uint col = BLOCK * id.x + x;
                if (row >= height || col >= width)
                    continue;
                uint color = color_at(col, row);
                float value = mosaic_at(col, row);
                float clip = clip_of(color);
                uint entry = (row / 3) * params.highlights.mask_width + col / 3;
                if (value < clip && value > 0.2 * clip && dilated_at(entry, color))
                {
                    sums = add_at(sums, color, value - opposing_average(int(col), int(row)));
                    counts = add_at(counts, color, 1);
                }
            }
        }
    }
    shared_sums[t] = sums;
    shared_counts[t] = counts;
    GroupMemoryBarrierWithGroupSync();
    for (uint stride = WORKGROUP / 2; stride > 0; stride /= 2)
    {
        if (t < stride)
        {
            shared_sums[t] += shared_sums[t + stride];
            shared_counts[t] += shared_counts[t + stride];
        }
        GroupMemoryBarrierWithGroupSync();
    }
    if (t == 0)
    {
        Partial partial;
        partial.sums = shared_sums[0];
        partial.counts = shared_counts[0];
        uint group_row = group.y + params.pass.origin_y / 16;
        partials[group_row * params.highlights.partials_across + group.x] = partial;
    }
}

// Colour c's offset from the workgroup's total. darktable keeps it only when more than
// 100 photosites contributed to it.
float color_offset(uint c)
{
    uint count = shared_counts[0][c];
    return count > 100 ? shared_sums[0][c] / float(count) : 0.0;
}

// Pass 4, one workgroup: adds the workgroups' sums up into each colour's offset.
[shader("compute")]
[numthreads(16, 16, 1)]
void opposed_chroma(uint t: SV_GroupIndex)
{
    float4 sums = float4(0.0);
    uint4 counts = uint4(0);
    for (uint i = t; i < params.highlights.partials; i += WORKGROUP)
    {
        sums += partials[i].sums;
        counts += partials[i].counts;
    }
    shared_sums[t] = sums;
    shared_counts[t] = counts;
    GroupMemoryBarrierWithGroupSync();
    for (uint stride = WORKGROUP / 2; stride > 0; stride /= 2)
    {
        if (t < stride)
        {
            shared_sums[t] += shared_sums[t + stride];
            shared_counts[t] += shared_counts[t + stride];
        }
        GroupMemoryBarrierWithGroupSync();
    }
    if (t == 0)
        chroma[0] = float4(color_offset(0), color_offset(1), color_offset(2), 0.0);
}

// Pass 5. A photosite at or above its colour's clipping level becomes its opposing
// average plus its colour's offset, if that is brighter; the others pass through.
[shader("compute")]
[numthreads(16, 16, 1)]
void opposed_inpaint(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint col = id.x;
    uint row = params.pass.origin_y + id.y;
    float value = mosaic_at(col, row);
    uint color = color_at(col, row);
    if (value >= clip_of(color))
        value = max(value, opposing_average(int(col), int(row)) + chroma[0][color]);
    output[row * params.highlights.width + col - params.pass.output_base] = value;
}
