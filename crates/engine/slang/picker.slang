// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2016-2025  darktable developers (ported from src/common/color_picker.c and src/common/bspline.h)
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

// The colour pickers' reduction (docs/pickers.md, The kernel; decision 79): for each of
// up to 64 boxes of an RGBA buffer or a mosaic, the sum, the minimum and the maximum of
// each of four channels, from which the engine makes the mean on the CPU: darktable's
// _color_picker_work_4ch, _color_picker_work_1ch and _update_stats_by_ch
// (src/common/color_picker.c) in the buffer's own values. Three entry points, the first
// or the second dispatched before the third:
//
// - picker_rgba: each workgroup takes 64 × 64 pixels of one box, each invocation 4 × 4 of
//   them, 16 apart, added row by row, each pixel first blurred by darktable's B-spline
//   where its box asks for denoise; the workgroup adds its 256 invocations' sums as a
//   tree in workgroup memory and writes one partial, at its index among every box's
//   workgroups.
// - picker_mosaic: the same over photosites, each added to the channel of its CFA
//   colour, as darktable's Bayer and X-Trans workers add it; the engine counts each
//   colour's photosites on the CPU from the pattern.
// - picker_combine: one workgroup a box, each invocation adding that box's partials 256
//   apart in their index order, then the workgroup's as a tree again, and writes the
//   box's sum, minimum and maximum.
//
// So a sum never depends on scheduling, nor on how the workgroups are dispatched: across
// and down, or in row strips over windows of a buffer larger than one binding (decision
// 14), each workgroup keeping its index among every box's. The CPU gets the same bits
// from the same tree. The minimum and the maximum are exact. The mean is the sum over the
// number of pixels, or of a colour's photosites, divided on the CPU as darktable divides
// it, since a GPU's division need only be within 2.5 ulp: so it too is the same bits on
// every GPU. The engine's own kernel, run at an operation's place in the live viewport
// pipe and on the frame: no operation's pass, no golden case and no derivatives. No NaN
// checks: the operations before it sanitise their outputs (AGENTS.md).

import pass;
import components;

// The boxes one reduction takes.
static const uint MAX_BOXES = 64;
static const uint WORKGROUP = 256;
// Each invocation adds BLOCK × BLOCK pixels, STEP apart, so each row of a workgroup's
// invocations reads 16 neighbouring pixels at a time and a workgroup covers TILE × TILE.
static const uint BLOCK = 4;
static const uint STEP = 16;
static const uint TILE = BLOCK * STEP;
// darktable's FLT_MAX, the minimum's start and the maximum's, negated.
static const float FLT_MAX = 3.40282347e+38;
// A box's flag: its pixels blurred first, darktable's DT_COLOR_PICKER_DENOISE.
static const uint DENOISE = 1;

struct PickerBox
{
    // Left, top, right and bottom in the buffer's pixels, right and bottom exclusive:
    // darktable's box, never empty.
    uint4 bounds;
    // Its first workgroup among every box's, its workgroups across, its workgroups in
    // all, and its flags.
    uint4 tiles;
};

struct PickerParams
{
    // The boxes this reduction takes, at most MAX_BOXES.
    uint count;
    // The dispatch's workgroups across: a workgroup's index is first + y × across + x.
    uint across;
    // Where the dispatch's workgroups end: those at and past it have nothing to do.
    uint end;
    // The dispatch's first workgroup among every box's: a row strip's, else 0.
    uint first;
    // The first box's place in the results.
    uint results;
    uint _pad0;
    uint _pad1;
    uint _pad2;
    PickerBox boxes[MAX_BOXES];
    // A mosaic's colour pattern: its width and height, then the mosaic's top-left
    // photosite on the sensor.
    uint4 pattern;
    // Each position's CFA colour, row by row, four to a vector: 0 red, 1 green, 2 blue.
    uint4 colours[9];
};

struct Params
{
    PassHeader pass;
    PickerParams picker;
};

// A workgroup's sum, minimum and maximum, and a box's in the results.
struct Partial
{
    float4 sum;
    float4 low;
    float4 high;
};

// The RGBA image, or the mosaic: the whole buffer, or the window of it a row strip reads,
// which starts at the header's input_base.
[[vk::binding(0, 0)]] StructuredBuffer<float4> image;
[[vk::binding(1, 0)]] RWStructuredBuffer<Partial> partials;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;
[[vk::binding(3, 0)]] RWStructuredBuffer<Partial> results;
[[vk::binding(4, 0)]] StructuredBuffer<float> mosaic;

groupshared float4 shared_sum[WORKGROUP];
groupshared float4 shared_low[WORKGROUP];
groupshared float4 shared_high[WORKGROUP];

// Adds the workgroup's 256 sums as a tree, pairs 128 apart first, and takes their
// minimum and maximum, into element 0. Every invocation calls it, so each barrier is
// reached by the whole workgroup.
void reduce(uint index, float4 sum, float4 low, float4 high)
{
    shared_sum[index] = sum;
    shared_low[index] = low;
    shared_high[index] = high;
    GroupMemoryBarrierWithGroupSync();
    for (uint stride = WORKGROUP / 2; stride > 0; stride /= 2)
    {
        if (index < stride)
        {
            shared_sum[index] = shared_sum[index] + shared_sum[index + stride];
            shared_low[index] = min(shared_low[index], shared_low[index + stride]);
            shared_high[index] = max(shared_high[index], shared_high[index + stride]);
        }
        GroupMemoryBarrierWithGroupSync();
    }
}

// Where a workgroup works: its index among every box's, whether it has work, its box and
// the first pixel its invocation adds.
struct Place
{
    uint g;
    bool working;
    PickerBox box;
    uint2 corner;
};

Place place(uint3 group, uint3 local)
{
    Place p;
    p.g = params.picker.first + group.y * params.picker.across + group.x;
    p.working = p.g < params.picker.end;
    // The box whose workgroups hold this one: the last whose first is at or before it.
    uint b = 0;
    for (uint k = 1; k < params.picker.count; k++)
    {
        if (p.g >= params.picker.boxes[k].tiles.x)
            b = k;
    }
    p.box = params.picker.boxes[b];
    uint tile = p.working ? p.g - p.box.tiles.x : 0;
    p.corner = p.box.bounds.xy + uint2(tile % p.box.tiles.y, tile / p.box.tiles.y) * TILE + local.xy;
    return p;
}

// The pixel at (x, y) of the RGBA image, read through the window bound.
float4 pixel(uint x, uint y)
{
    return image[y * params.pass.width + x - params.pass.input_base / 4];
}

// darktable's B-spline weights, 1, 4, 6, 4, 1 over 16 (bspline.h's filter).
static const float F0 = 1.0 / 16.0;
static const float F1 = 4.0 / 16.0;
static const float F2 = 6.0 / 16.0;

// The vertical B-spline sum of column x over rows r0 to r4, as darktable's
// _bspline_vertical_pass adds it.
float4 column(uint x, uint r0, uint r1, uint r2, uint r3, uint r4)
{
    return F0 * pixel(x, r0) + F1 * pixel(x, r1) + F2 * pixel(x, r2) + F1 * pixel(x, r3)
        + F0 * pixel(x, r4);
}

// The pixel at `at` blurred by darktable's blur_2D_Bspline at one scale (mult 1), its
// negatives kept: the vertical pass over the rows two either side, then the horizontal
// over the columns two either side, each index clamped to the image as darktable's MAX
// and MIN clamp it. darktable blurs the whole image first; this reads only what the pixel
// needs, which gives the same values.
float4 blurred(uint2 at)
{
    int x = int(at.x);
    int y = int(at.y);
    int right = int(params.pass.width) - 1;
    int bottom = int(params.pass.height) - 1;
    uint r0 = uint(max(y - 2, 0));
    uint r1 = uint(max(y - 1, 0));
    uint r3 = uint(min(y + 1, bottom));
    uint r4 = uint(min(y + 2, bottom));
    float4 c0 = column(uint(max(x - 2, 0)), r0, r1, at.y, r3, r4);
    float4 c1 = column(uint(max(x - 1, 0)), r0, r1, at.y, r3, r4);
    float4 c2 = column(at.x, r0, r1, at.y, r3, r4);
    float4 c3 = column(uint(min(x + 1, right)), r0, r1, at.y, r3, r4);
    float4 c4 = column(uint(min(x + 2, right)), r0, r1, at.y, r3, r4);
    return F0 * c0 + F1 * c1 + F2 * c2 + F1 * c3 + F0 * c4;
}

// Writes the workgroup's partial, at its index among every box's.
void partial(Place p, uint index)
{
    if (p.working && index == 0)
    {
        Partial partial;
        partial.sum = shared_sum[0];
        partial.low = shared_low[0];
        partial.high = shared_high[0];
        partials[p.g] = partial;
    }
}

[shader("compute")]
[numthreads(16, 16, 1)]
void picker_rgba(uint3 group: SV_GroupID, uint3 local: SV_GroupThreadID, uint index: SV_GroupIndex)
{
    Place p = place(group, local);
    bool denoise = (p.box.tiles.w & DENOISE) != 0;
    float4 sum = float4(0.0);
    float4 low = float4(FLT_MAX);
    float4 high = float4(-FLT_MAX);
    if (p.working)
    {
        for (uint by = 0; by < BLOCK; by++)
        {
            for (uint bx = 0; bx < BLOCK; bx++)
            {
                uint2 at = p.corner + uint2(bx, by) * STEP;
                if (at.x < p.box.bounds.z && at.y < p.box.bounds.w)
                {
                    float4 v = denoise ? blurred(at) : pixel(at.x, at.y);
                    sum = sum + v;
                    low = min(low, v);
                    high = max(high, v);
                }
            }
        }
    }
    reduce(index, sum, low, high);
    partial(p, index);
}

// The CFA colour of the photosite at (x, y) of the mosaic: its pattern's, at its place on
// the sensor, as darktable's FC and FCxtrans give it from the region's corner.
uint colour(uint x, uint y)
{
    uint4 pattern = params.picker.pattern;
    uint i = ((pattern.w + y) % pattern.y) * pattern.x + (pattern.z + x) % pattern.x;
    return params.picker.colours[i / 4][i % 4];
}

[shader("compute")]
[numthreads(16, 16, 1)]
void picker_mosaic(uint3 group: SV_GroupID, uint3 local: SV_GroupThreadID, uint index: SV_GroupIndex)
{
    Place p = place(group, local);
    float4 sum = float4(0.0);
    float4 low = float4(FLT_MAX);
    float4 high = float4(-FLT_MAX);
    if (p.working)
    {
        for (uint by = 0; by < BLOCK; by++)
        {
            for (uint bx = 0; bx < BLOCK; bx++)
            {
                uint2 at = p.corner + uint2(bx, by) * STEP;
                if (at.x < p.box.bounds.z && at.y < p.box.bounds.w)
                {
                    float v = mosaic[at.y * params.pass.width + at.x - params.pass.input_base];
                    uint c = colour(at.x, at.y);
                    sum = add_at(sum, c, v);
                    low = set_at(low, c, min(low[c], v));
                    high = set_at(high, c, max(high[c], v));
                }
            }
        }
    }
    reduce(index, sum, low, high);
    partial(p, index);
}

[shader("compute")]
[numthreads(16, 16, 1)]
void picker_combine(uint3 group: SV_GroupID, uint index: SV_GroupIndex)
{
    PickerBox box = params.picker.boxes[group.x];
    float4 sum = float4(0.0);
    float4 low = float4(FLT_MAX);
    float4 high = float4(-FLT_MAX);
    for (uint i = index; i < box.tiles.z; i += WORKGROUP)
    {
        Partial partial = partials[box.tiles.x + i];
        sum = sum + partial.sum;
        low = min(low, partial.low);
        high = max(high, partial.high);
    }
    reduce(index, sum, low, high);
    if (index == 0)
    {
        // The sum, which the engine divides by the number of pixels on the CPU, as
        // darktable's mean divides: a division on the GPU need not be correctly rounded.
        Partial result;
        result.sum = shared_sum[0];
        result.low = shared_low[0];
        result.high = shared_high[0];
        results[params.picker.results + group.x] = result;
    }
}
