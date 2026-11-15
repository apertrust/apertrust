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

// Generated from slang/raw_histogram.slang by tools/build-kernels.sh. Do not edit.

struct PassHeader_std140_0
{
    @align(16) width_0 : u32,
    @align(4) height_0 : u32,
    @align(8) origin_x_0 : u32,
    @align(4) origin_y_0 : u32,
    @align(16) full_width_0 : u32,
    @align(4) full_height_0 : u32,
    @align(8) scale_0 : f32,
    @align(4) repeat_0 : u32,
    @align(16) input_base_0 : u32,
    @align(4) output_base_0 : u32,
    @align(8) reserved0_0 : u32,
    @align(4) reserved1_0 : u32,
};

struct RawHistogramParams_std140_0
{
    @align(16) right_0 : u32,
    @align(4) _pad0_0 : u32,
    @align(8) _pad1_0 : u32,
    @align(4) _pad2_0 : u32,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) histogram_0 : RawHistogramParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(0) @group(0) var<storage, read> mosaic_0 : array<u32>;

@binding(1) @group(0) var<storage, read_write> counts_0 : array<atomic<u32>>;

fn value_at_0( index_0 : u32,  base_0 : u32) -> u32
{
    var word_0 : u32 = mosaic_0[((index_0 >> (u32(1)))) - base_0];
    var _S1 : u32;
    if(((index_0 & (u32(1)))) == u32(0))
    {
        _S1 = (word_0 & (u32(65535)));
    }
    else
    {
        _S1 = (word_0 >> (u32(16)));
    }
    return _S1;
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_0 : vec3<u32>)
{
    var h_0 : PassHeader_std140_0 = params_0.pass_0;
    var _S2 : u32 = id_0.x;
    var _S3 : bool;
    if((_S2 >= (params_0.pass_0.width_0)))
    {
        _S3 = true;
    }
    else
    {
        _S3 = ((id_0.y) >= (h_0.height_0));
    }
    if(_S3)
    {
        return;
    }
    var _S4 : u32 = h_0.origin_y_0 + id_0.y;
    var _S5 : u32 = h_0.origin_x_0 + _S2 * u32(4);
    var k_0 : u32 = u32(0);
    loop
    {
        if((k_0 < u32(4)))
        {
        }
        else
        {
            break;
        }
        var x_0 : u32 = _S5 + k_0;
        if((x_0 < (params_0.histogram_0.right_0)))
        {
            var _S6 : u32 = atomicAdd(&(counts_0[value_at_0(_S4 * h_0.full_width_0 + x_0, h_0.input_base_0)]), u32(1));
        }
        k_0 = k_0 + u32(1);
    }
    return;
}

