// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2012-2026  darktable developers (ported from src/common/interpolation.c)
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

// An export's resample (decision 56; docs/ports/finalscale.md): darktable's
// dt_interpolation_resample with its plans, lanczos3's taps made and normalised on the
// CPU (crates/engine/src/resample.rs) and bound per axis as tables. Each output pixel
// adds, for each of its rows' taps in order, that row's sum of its column taps in order,
// as darktable's loop does; `across` makes every row's sums for each output column
// once, into scratch, and `main` adds them up, so each pixel's arithmetic is darktable's
// in its order. All four channels, as darktable's. The engine's own passes, not an
// operation: no derivatives.

import pass;

struct ResampleParams
{
    // The input's width, the row its buffer starts at within the rows `across` reads,
    // and the output's width, which is the scratch's.
    uint input_width;
    uint first_row;
    uint output_width;
    uint reserved;
};

struct Params
{
    PassHeader pass;
    ResampleParams resample;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;
// One axis's plan, the pass's: per output column or row where its taps start and how
// many there are, then each tap's input pixel and weight.
[[vk::binding(3, 0)]] StructuredBuffer<uint2> spans;
[[vk::binding(4, 0)]] StructuredBuffer<uint> taps_index;
[[vk::binding(5, 0)]] StructuredBuffer<float> taps_weight;
// Each input row the output's rows read, resampled across: output width by those rows.
[[vk::binding(10, 0)]] RWStructuredBuffer<float4> across_rows;

// One invocation per output column and input row: the row's sum of the column's taps.
[shader("compute")]
[numthreads(16, 16, 1)]
void across(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    ResampleParams r = params.resample;
    uint2 span = spans[id.x];
    uint row = (r.first_row + id.y) * r.input_width;
    float4 sum = float4(0.0);
    for (uint k = 0; k < span.y; k++)
        sum += input[row + taps_index[span.x + k]] * taps_weight[span.x + k];
    across_rows[id.y * r.output_width + id.x] = sum;
}

// One invocation per output pixel: its rows' sums by their taps.
[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    ResampleParams r = params.resample;
    uint2 span = spans[id.y];
    float4 sum = float4(0.0);
    for (uint k = 0; k < span.y; k++)
        sum += across_rows[taps_index[span.x + k] * r.output_width + id.x] * taps_weight[span.x + k];
    output[id.y * r.output_width + id.x] = sum;
}
