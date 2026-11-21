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

// The viewport pipe's downscale (decision 12): a box filter from the source image down
// to the output's size, each output pixel the mean of the source pixels whose right and
// bottom edges fall inside it: output pixel o covers source pixels o × source / output
// up to, not including, (o + 1) × source / output, in integers, so the planner knows
// each footprint exactly. The output is never larger than the source, which the setup
// checks, so every footprint has a pixel. A tile of the viewport (decision 14) writes
// the output pixels the header's size covers from its origin, reading a part of the
// source, its demosaic tile, that holds their footprints: the same sums in the same
// order as the whole image's. `main_region` writes into a buffer that holds one region
// of the output alone, at the output origin and with the rows its uniforms give: the
// zoomed view's detail between Fit and 100% (decision 99), and crop's first region once
// built. The engine's own pass, after demosaic; not a darktable port: no golden case and
// no derivatives. Its footprints and sums are
// box_downscale.slang's, which an operation that downscales what it computes at full
// resolution shares (decision 15).

import pass;
import box_downscale;

struct DownscaleParams
{
    // The whole source, the developed image at full resolution, and the whole output.
    uint source_width;
    uint source_height;
    uint output_width;
    uint output_height;
    // Where the input's first pixel lies in the source, and its rows' length.
    uint input_x;
    uint input_y;
    uint input_width;
    uint _pad;
};

struct Params
{
    PassHeader pass;
    DownscaleParams downscale;
};

// `main_region`'s: the whole output's footprints, as `main`'s, and the region of it the
// output buffer holds, so its pixel o lies at o less the region's corner in it.
struct DownscaleRegionParams
{
    DownscaleParams downscale;
    // The region's corner in the whole output, and the output buffer's rows' length.
    uint output_x;
    uint output_y;
    uint output_row;
    uint _pad;
};

struct RegionParams
{
    PassHeader pass;
    DownscaleRegionParams downscale;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;
// `main_region`'s uniform block, at a binding of its own, so `main` binds only its own and
// its plans keep their bytes.
[[vk::binding(3, 0)]] ConstantBuffer<RegionParams> region_params;

// The input, a part of the source whose first pixel lies at (x, y) of it.
struct Source : IDownscaleSource<float4>
{
    uint x;
    uint y;
    uint width;

    float4 read(uint sx, uint sy)
    {
        return input[(sy - y) * width + sx - x];
    }
};

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    DownscaleParams p = params.downscale;
    uint2 o = uint2(params.pass.origin_x, params.pass.origin_y) + id.xy;
    uint2 source = uint2(p.source_width, p.source_height);
    uint2 size = uint2(p.output_width, p.output_height);
    Source part = { p.input_x, p.input_y, p.input_width };
    output[o.y * p.output_width + o.x] = footprint_mean<float4>(part, o, source, size);
}

// `main`, writing into a buffer that holds the region of the output from
// (output_x, output_y), output_row pixels a row: the pass header's origin and size are in
// the whole output, and lie inside that region.
[shader("compute")]
[numthreads(16, 16, 1)]
void main_region(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= region_params.pass.width || id.y >= region_params.pass.height)
        return;
    DownscaleRegionParams r = region_params.downscale;
    DownscaleParams p = r.downscale;
    uint2 o = uint2(region_params.pass.origin_x, region_params.pass.origin_y) + id.xy;
    uint2 source = uint2(p.source_width, p.source_height);
    uint2 size = uint2(p.output_width, p.output_height);
    Source part = { p.input_x, p.input_y, p.input_width };
    uint at = (o.y - r.output_y) * r.output_row + o.x - r.output_x;
    output[at] = footprint_mean<float4>(part, o, source, size);
}
