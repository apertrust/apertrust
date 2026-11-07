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

// Generated from slang/region.slang by tools/build-kernels.sh. Do not edit.

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

struct RegionParams_std140_0
{
    @align(16) source_width_0 : u32,
    @align(4) x_0 : u32,
    @align(8) y_0 : u32,
    @align(4) channels_0 : u32,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) region_0 : RegionParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(1) @group(0) var<storage, read_write> output_0 : array<f32>;

@binding(0) @group(0) var<storage, read> input_0 : array<f32>;

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_0 : vec3<u32>)
{
    var _S1 : u32 = id_0.x;
    var _S2 : bool;
    if((_S1 >= (params_0.pass_0.width_0)))
    {
        _S2 = true;
    }
    else
    {
        _S2 = ((id_0.y) >= (params_0.pass_0.height_0));
    }
    if(_S2)
    {
        return;
    }
    var r_0 : RegionParams_std140_0 = params_0.region_0;
    var row_0 : u32 = params_0.pass_0.origin_y_0 + id_0.y;
    var _S3 : u32 = ((params_0.region_0.y_0 + row_0) * params_0.region_0.source_width_0 + params_0.region_0.x_0 + _S1) * params_0.region_0.channels_0 - params_0.pass_0.input_base_0;
    var _S4 : u32 = (row_0 * params_0.pass_0.width_0 + _S1) * params_0.region_0.channels_0 - params_0.pass_0.output_base_0;
    var c_0 : u32 = u32(0);
    loop
    {
        if((c_0 < (r_0.channels_0)))
        {
        }
        else
        {
            break;
        }
        output_0[_S4 + c_0] = input_0[_S3 + c_0];
        c_0 = c_0 + u32(1);
    }
    return;
}

