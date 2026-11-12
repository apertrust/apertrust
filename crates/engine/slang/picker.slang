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

// The colour pickers' reduction (docs/pickers.md, The kernel; decision 79): for each of
// up to 64 boxes of an RGBA buffer, the sum, the minimum and the maximum of each of its
// four channels, from which the engine makes the mean on the CPU, darktable's _color_picker_work_4ch and _update_stats_by_ch
// (src/common/color_picker.c) in the buffer's own values. Two entry points, dispatched
// one after the other:
//
// - picker_rgba: each workgroup takes 64 × 64 pixels of one box, each invocation 4 × 4 of
//   them, 16 apart, added row by row; the workgroup adds its 256 invocations' sums as a
//   tree in workgroup memory and writes one partial, at its index in the dispatch.
// - picker_combine: one workgroup a box, each invocation adding that box's partials 256
//   apart in their index order, then the workgroup's as a tree again, and writes the
//   box's sum, minimum and maximum.
//
// So a sum never depends on scheduling, and the CPU gets the same bits from the same
// tree. The minimum and the maximum are exact. The mean is the sum over the number of
// pixels, divided on the CPU as darktable divides it, since a GPU's division need only be
// within 2.5 ulp: so it too is the same bits on every GPU. The engine's own kernel, read back after
// the frame: no operation's pass, no golden case and no derivatives. No NaN checks: the
// operations before it sanitise their outputs (AGENTS.md).

import pass;

// The boxes one dispatch takes.
static const uint MAX_BOXES = 64;
static const uint WORKGROUP = 256;
// Each invocation adds BLOCK × BLOCK pixels, STEP apart, so each row of a workgroup's
// invocations reads 16 neighbouring pixels at a time and a workgroup covers TILE × TILE.
static const uint BLOCK = 4;
static const uint STEP = 16;
static const uint TILE = BLOCK * STEP;
// darktable's FLT_MAX, the minimum's start and the maximum's, negated.
static const float FLT_MAX = 3.40282347e+38;

struct PickerBox
{
    // Left, top, right and bottom in the buffer's pixels, right and bottom exclusive:
    // darktable's box, never empty.
    uint4 bounds;
    // Its first workgroup in the dispatch, its workgroups across, its workgroups in all,
    // and a word unused.
    uint4 tiles;
};

struct PickerParams
{
    // The boxes this dispatch reduces, at most MAX_BOXES.
    uint count;
    // The dispatch's workgroups across: a workgroup's index is y × across + x.
    uint across;
    // Every box's workgroups together: those at and past it have nothing to do.
    uint workgroups;
    uint _pad0;
    PickerBox boxes[MAX_BOXES];
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

[[vk::binding(0, 0)]] StructuredBuffer<float4> image;
[[vk::binding(1, 0)]] RWStructuredBuffer<Partial> partials;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;
[[vk::binding(3, 0)]] RWStructuredBuffer<Partial> results;

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

[shader("compute")]
[numthreads(16, 16, 1)]
void picker_rgba(uint3 group: SV_GroupID, uint3 local: SV_GroupThreadID, uint index: SV_GroupIndex)
{
    uint g = group.y * params.picker.across + group.x;
    bool working = g < params.picker.workgroups;
    // The box whose workgroups hold this one: the last whose first is at or before it.
    uint b = 0;
    for (uint k = 1; k < params.picker.count; k++)
    {
        if (g >= params.picker.boxes[k].tiles.x)
            b = k;
    }
    PickerBox box = params.picker.boxes[b];
    uint tile = working ? g - box.tiles.x : 0;
    uint2 corner = box.bounds.xy + uint2(tile % box.tiles.y, tile / box.tiles.y) * TILE + local.xy;

    float4 sum = float4(0.0);
    float4 low = float4(FLT_MAX);
    float4 high = float4(-FLT_MAX);
    if (working)
    {
        for (uint by = 0; by < BLOCK; by++)
        {
            for (uint bx = 0; bx < BLOCK; bx++)
            {
                uint2 at = corner + uint2(bx, by) * STEP;
                if (at.x < box.bounds.z && at.y < box.bounds.w)
                {
                    float4 v = image[at.y * params.pass.width + at.x];
                    sum = sum + v;
                    low = min(low, v);
                    high = max(high, v);
                }
            }
        }
    }
    reduce(index, sum, low, high);
    if (working && index == 0)
    {
        Partial partial;
        partial.sum = shared_sum[0];
        partial.low = shared_low[0];
        partial.high = shared_high[0];
        partials[g] = partial;
    }
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
        results[group.x] = result;
    }
}
