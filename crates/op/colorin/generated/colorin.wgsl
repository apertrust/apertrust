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
    @align(16) clipped_0 : array<vec4<f32>, i32(3)>,
    @align(16) correction_0 : vec4<f32>,
    @align(16) mode_0 : vec4<u32>,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) colorin_0 : ColorinParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(0) @group(0) var<storage, read> input_0 : array<vec4<f32>>;

@binding(1) @group(0) var<storage, read_write> output_0 : array<vec4<f32>>;

fn blue_mapping_0( rgb_0 : vec3<f32>) -> vec3<f32>
{
    var _S1 : vec3<f32> = rgb_0;
    var sum_0 : f32 = rgb_0.x + rgb_0.y + rgb_0.z;
    if((sum_0 > 0.0f))
    {
        var z_0 : f32 = _S1.z / sum_0;
        if((z_0 > 0.5f))
        {
            var _S2 : f32 = (z_0 - 0.5f) / 0.5f * min(1.0f, sum_0 / 0.5f) * 0.10999999940395355f;
            _S1[i32(1)] = _S1[i32(1)] + _S2;
            _S1[i32(2)] = _S1[i32(2)] - _S2;
        }
    }
    return _S1;
}

fn apply_0( _S3 : vec3<f32>) -> vec3<f32>
{
    return vec3<f32>(dot(params_0.colorin_0.rows_0[i32(0)].xyz, _S3), dot(params_0.colorin_0.rows_0[i32(1)].xyz, _S3), dot(params_0.colorin_0.rows_0[i32(2)].xyz, _S3));
}

fn apply_1( _S4 : vec3<f32>) -> vec3<f32>
{
    return vec3<f32>(dot(params_0.colorin_0.clipped_0[i32(0)].xyz, _S4), dot(params_0.colorin_0.clipped_0[i32(1)].xyz, _S4), dot(params_0.colorin_0.clipped_0[i32(2)].xyz, _S4));
}

fn colorin_1( _S5 : vec3<f32>) -> vec3<f32>
{
    var camera_0 : vec3<f32> = _S5 * params_0.colorin_0.correction_0.xyz;
    var _S6 : vec4<u32> = params_0.colorin_0.mode_0;
    var camera_1 : vec3<f32>;
    if((params_0.colorin_0.mode_0.y) != u32(0))
    {
        camera_1 = blue_mapping_0(camera_0);
    }
    else
    {
        camera_1 = camera_0;
    }
    var _S7 : vec3<f32> = apply_0(camera_1);
    var working_0 : vec3<f32>;
    if((_S6.x) != u32(0))
    {
        working_0 = apply_1(clamp(_S7, vec3<f32>(0.0f), vec3<f32>(1.0f)));
    }
    else
    {
        working_0 = _S7;
    }
    return working_0;
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_0 : vec3<u32>)
{
    var _S8 : u32 = id_0.x;
    var _S9 : bool;
    if((_S8 >= (params_0.pass_0.width_0)))
    {
        _S9 = true;
    }
    else
    {
        _S9 = ((id_0.y) >= (params_0.pass_0.height_0));
    }
    if(_S9)
    {
        return;
    }
    var i_0 : u32 = id_0.y * params_0.pass_0.width_0 + _S8;
    var pixel_0 : vec4<f32> = input_0[i_0];
    var rgb_1 : vec3<f32> = pixel_0.xyz;
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
        var _S10 : vec3<f32> = colorin_1(rgb_1);
        var _S11 : u32 = k_0 + u32(1);
        rgb_1 = _S10;
        k_0 = _S11;
    }
    output_0[i_0] = vec4<f32>(rgb_1, pixel_0.w);
    return;
}

