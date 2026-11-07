// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2015-2025  darktable developers (ported from src/iop/rawprepare.c)
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

// Generated from slang/rawprepare.slang by tools/build-kernels.sh. Do not edit.

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

struct RawprepareParams_std140_0
{
    @align(16) left_0 : u32,
    @align(4) top_0 : u32,
    @align(8) sensor_width_0 : u32,
    @align(4) padding_0 : u32,
    @align(16) sub_0 : vec4<f32>,
    @align(16) div_0 : vec4<f32>,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) rawprepare_0 : RawprepareParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(0) @group(0) var<storage, read> input_0 : array<u32>;

@binding(1) @group(0) var<storage, read_write> output_0 : array<f32>;

fn photosite_0( x_0 : u32,  y_0 : u32) -> f32
{
    var i_0 : u32 = y_0 * params_0.rawprepare_0.sensor_width_0 + x_0;
    var word_0 : u32 = input_0[i_0 / u32(2) - params_0.pass_0.input_base_0];
    var _S1 : u32;
    if(((i_0 & (u32(1)))) == u32(0))
    {
        _S1 = (word_0 & (u32(65535)));
    }
    else
    {
        _S1 = (word_0 >> (u32(16)));
    }
    return f32(_S1);
}

fn rawprepare_1( raw_0 : f32,  sub_1 : f32,  div_1 : f32) -> f32
{
    return (raw_0 - sub_1) / div_1;
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_0 : vec3<u32>)
{
    var _S2 : u32 = id_0.x;
    var _S3 : bool;
    if((_S2 >= (params_0.pass_0.width_0)))
    {
        _S3 = true;
    }
    else
    {
        _S3 = ((id_0.y) >= (params_0.pass_0.height_0));
    }
    if(_S3)
    {
        return;
    }
    var p_0 : RawprepareParams_std140_0 = params_0.rawprepare_0;
    var col_0 : u32 = params_0.pass_0.origin_x_0 + _S2;
    var row_0 : u32 = params_0.pass_0.origin_y_0 + id_0.y;
    var x_1 : u32 = col_0 + params_0.rawprepare_0.left_0;
    var y_1 : u32 = row_0 + params_0.rawprepare_0.top_0;
    var _S4 : u32 = (((((y_1 & (u32(1)))) << (u32(1)))) | (((x_1 & (u32(1))))));
    var value_0 : f32 = photosite_0(x_1, y_1);
    var k_0 : u32 = u32(0);
    loop
    {
        if((k_0 < (params_0.pass_0.repeat_0)))
        {
        }
        else
        {
            break;
        }
        var _S5 : f32 = rawprepare_1(value_0, p_0.sub_0[_S4], p_0.div_0[_S4]);
        var _S6 : u32 = k_0 + u32(1);
        value_0 = _S5;
        k_0 = _S6;
    }
    output_0[row_0 * params_0.pass_0.width_0 + col_0 - params_0.pass_0.output_base_0] = value_0;
    return;
}

