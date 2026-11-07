// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/iop/colorout.c)
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

// Generated from slang/colorout.slang by tools/build-kernels.sh. Do not edit.

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

struct ColoroutParams_std140_0
{
    @align(16) rows_0 : array<vec4<f32>, i32(3)>,
    @align(16) curve_0 : u32,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) colorout_0 : ColoroutParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(0) @group(0) var<storage, read> input_0 : array<vec4<f32>>;

@binding(1) @group(0) var<storage, read_write> output_0 : array<vec4<f32>>;

fn srgb_0( v_0 : f32) -> f32
{
    var _S1 : f32 = max(v_0, 0.0f);
    var _S2 : f32;
    if((_S1 <= 0.0031308000907301903f))
    {
        _S2 = 12.920000076293945f * _S1;
    }
    else
    {
        _S2 = 1.0549999475479126f * pow(_S1, 0.4166666567325592f) - 0.054999999701976776f;
    }
    return _S2;
}

fn colorout_1( _S3 : vec3<f32>) -> vec3<f32>
{
    var _S4 : f32 = dot(params_0.colorout_0.rows_0[i32(0)].xyz, _S3);
    var _S5 : f32 = dot(params_0.colorout_0.rows_0[i32(1)].xyz, _S3);
    var _S6 : f32 = dot(params_0.colorout_0.rows_0[i32(2)].xyz, _S3);
    var linear_0 : vec3<f32> = vec3<f32>(_S4, _S5, _S6);
    if((params_0.colorout_0.curve_0) == u32(0))
    {
        return linear_0;
    }
    return vec3<f32>(srgb_0(_S4), srgb_0(_S5), srgb_0(_S6));
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_0 : vec3<u32>)
{
    var _S7 : u32 = id_0.x;
    var _S8 : bool;
    if((_S7 >= (params_0.pass_0.width_0)))
    {
        _S8 = true;
    }
    else
    {
        _S8 = ((id_0.y) >= (params_0.pass_0.height_0));
    }
    if(_S8)
    {
        return;
    }
    var i_0 : u32 = id_0.y * params_0.pass_0.width_0 + _S7;
    var pixel_0 : vec4<f32> = input_0[i_0];
    var rgb_0 : vec3<f32> = pixel_0.xyz;
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
        var _S9 : vec3<f32> = colorout_1(rgb_0);
        var _S10 : u32 = k_0 + u32(1);
        rgb_0 = _S9;
        k_0 = _S10;
    }
    output_0[i_0] = vec4<f32>(rgb_0, pixel_0.w);
    return;
}

