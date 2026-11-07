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

// Generated from slang/downscale.slang by tools/build-kernels.sh. Do not edit.

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

struct DownscaleParams_std140_0
{
    @align(16) source_width_0 : u32,
    @align(4) source_height_0 : u32,
    @align(8) output_width_0 : u32,
    @align(4) output_height_0 : u32,
    @align(16) input_x_0 : u32,
    @align(4) input_y_0 : u32,
    @align(8) input_width_0 : u32,
    @align(4) _pad_0 : u32,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) downscale_0 : DownscaleParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(1) @group(0) var<storage, read_write> output_0 : array<vec4<f32>>;

@binding(0) @group(0) var<storage, read> input_0 : array<vec4<f32>>;

struct Source_0
{
     x_0 : u32,
     y_0 : u32,
     width_1 : u32,
};

fn Source_read_0( this_0 : Source_0,  sx_0 : u32,  sy_0 : u32) -> vec4<f32>
{
    return input_0[(sy_0 - this_0.y_0) * this_0.width_1 + sx_0 - this_0.x_0];
}

fn footprint_mean_0( source_0 : Source_0,  o_0 : vec2<u32>,  source_size_0 : vec2<u32>,  output_size_0 : vec2<u32>) -> vec4<f32>
{
    var _S1 : u32;
    var start_0 : vec2<u32> = o_0 * source_size_0 / output_size_0;
    var end_0 : vec2<u32> = (o_0 + vec2<u32>(u32(1))) * source_size_0 / output_size_0;
    var _S2 : u32 = start_0.y;
    var sum_0 : vec4<f32> = vec4<f32>(0.0f);
    var y_1 : u32 = _S2;
    loop
    {
        var _S3 : u32 = end_0.y;
        _S1 = _S3;
        if((y_1 < _S3))
        {
        }
        else
        {
            break;
        }
        var x_1 : u32 = start_0.x;
        loop
        {
            if((x_1 < (end_0.x)))
            {
            }
            else
            {
                break;
            }
            var _S4 : vec4<f32> = sum_0 + Source_read_0(source_0, x_1, y_1);
            var _S5 : u32 = x_1 + u32(1);
            sum_0 = _S4;
            x_1 = _S5;
        }
        y_1 = y_1 + u32(1);
    }
    return sum_0 / vec4<f32>(f32(i32((end_0.x - start_0.x) * (_S1 - _S2))));
}

fn Source_x24init_0( x_2 : u32,  y_2 : u32,  width_2 : u32) -> Source_0
{
    var _S6 : Source_0;
    _S6.x_0 = x_2;
    _S6.y_0 = y_2;
    _S6.width_1 = width_2;
    return _S6;
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_0 : vec3<u32>)
{
    var _S7 : bool;
    if(((id_0.x) >= (params_0.pass_0.width_0)))
    {
        _S7 = true;
    }
    else
    {
        _S7 = ((id_0.y) >= (params_0.pass_0.height_0));
    }
    if(_S7)
    {
        return;
    }
    var o_1 : vec2<u32> = vec2<u32>(params_0.pass_0.origin_x_0, params_0.pass_0.origin_y_0) + id_0.xy;
    var _S8 : vec4<f32> = footprint_mean_0(Source_x24init_0(params_0.downscale_0.input_x_0, params_0.downscale_0.input_y_0, params_0.downscale_0.input_width_0), o_1, vec2<u32>(params_0.downscale_0.source_width_0, params_0.downscale_0.source_height_0), vec2<u32>(params_0.downscale_0.output_width_0, params_0.downscale_0.output_height_0));
    output_0[o_1.y * params_0.downscale_0.output_width_0 + o_1.x] = _S8;
    return;
}

