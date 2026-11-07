// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/iop/colorin.c)
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

// Generated from slang/colorin.slang by tools/build-kernels.sh. Do not edit.

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

struct ColorinParams_std140_0
{
    @align(16) rows_0 : array<vec4<f32>, i32(3)>,
    @align(16) correction_0 : vec4<f32>,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) colorin_0 : ColorinParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(0) @group(0) var<storage, read> input_0 : array<vec4<f32>>;

@binding(1) @group(0) var<storage, read_write> output_0 : array<vec4<f32>>;

fn colorin_1( _S1 : vec3<f32>) -> vec3<f32>
{
    var camera_0 : vec3<f32> = _S1 * params_0.colorin_0.correction_0.xyz;
    return vec3<f32>(dot(params_0.colorin_0.rows_0[i32(0)].xyz, camera_0), dot(params_0.colorin_0.rows_0[i32(1)].xyz, camera_0), dot(params_0.colorin_0.rows_0[i32(2)].xyz, camera_0));
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
    var i_0 : u32 = id_0.y * params_0.pass_0.width_0 + _S2;
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
        var _S4 : vec3<f32> = colorin_1(rgb_0);
        var _S5 : u32 = k_0 + u32(1);
        rgb_0 = _S4;
        k_0 = _S5;
    }
    output_0[i_0] = vec4<f32>(rgb_0, pixel_0.w);
    return;
}

