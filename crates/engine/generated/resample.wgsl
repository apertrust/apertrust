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

// Generated from slang/resample.slang by tools/build-kernels.sh. Do not edit.

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

struct ResampleParams_std140_0
{
    @align(16) input_width_0 : u32,
    @align(4) first_row_0 : u32,
    @align(8) output_width_0 : u32,
    @align(4) reserved_0 : u32,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) resample_0 : ResampleParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(3) @group(0) var<storage, read> spans_0 : array<vec2<u32>>;

@binding(4) @group(0) var<storage, read> taps_index_0 : array<u32>;

@binding(0) @group(0) var<storage, read> input_0 : array<vec4<f32>>;

@binding(5) @group(0) var<storage, read> taps_weight_0 : array<f32>;

@binding(10) @group(0) var<storage, read_write> across_rows_0 : array<vec4<f32>>;

@binding(1) @group(0) var<storage, read_write> output_0 : array<vec4<f32>>;

@compute
@workgroup_size(16, 16, 1)
fn across(@builtin(global_invocation_id) id_0 : vec3<u32>)
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
    var r_0 : ResampleParams_std140_0 = params_0.resample_0;
    var _S3 : vec2<u32> = spans_0[_S1];
    var _S4 : u32 = id_0.y;
    var _S5 : u32 = (params_0.resample_0.first_row_0 + _S4) * params_0.resample_0.input_width_0;
    var _S6 : vec4<f32> = vec4<f32>(0.0f);
    var k_0 : u32 = u32(0);
    var sum_0 : vec4<f32> = _S6;
    loop
    {
        if((k_0 < (_S3.y)))
        {
        }
        else
        {
            break;
        }
        var _S7 : u32 = _S3.x + k_0;
        var sum_1 : vec4<f32> = sum_0 + input_0[_S5 + taps_index_0[_S7]] * vec4<f32>(taps_weight_0[_S7]);
        k_0 = k_0 + u32(1);
        sum_0 = sum_1;
    }
    across_rows_0[_S4 * r_0.output_width_0 + _S1] = sum_0;
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_1 : vec3<u32>)
{
    var _S8 : u32 = id_1.x;
    var _S9 : bool;
    if((_S8 >= (params_0.pass_0.width_0)))
    {
        _S9 = true;
    }
    else
    {
        _S9 = ((id_1.y) >= (params_0.pass_0.height_0));
    }
    if(_S9)
    {
        return;
    }
    var r_1 : ResampleParams_std140_0 = params_0.resample_0;
    var _S10 : u32 = id_1.y;
    var _S11 : vec2<u32> = spans_0[_S10];
    var _S12 : vec4<f32> = vec4<f32>(0.0f);
    var k_1 : u32 = u32(0);
    var sum_2 : vec4<f32> = _S12;
    loop
    {
        if((k_1 < (_S11.y)))
        {
        }
        else
        {
            break;
        }
        var _S13 : u32 = _S11.x + k_1;
        var sum_3 : vec4<f32> = sum_2 + across_rows_0[taps_index_0[_S13] * r_1.output_width_0 + _S8] * vec4<f32>(taps_weight_0[_S13]);
        k_1 = k_1 + u32(1);
        sum_2 = sum_3;
    }
    output_0[_S10 * r_1.output_width_0 + _S8] = sum_2;
    return;
}

