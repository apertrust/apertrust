// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/iop/temperature.c)
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

// Generated from slang/temperature.slang by tools/build-kernels.sh. Do not edit.

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

struct TemperatureParams_std140_0
{
    @align(16) pattern_width_0 : u32,
    @align(4) pattern_height_0 : u32,
    @align(8) origin_x_1 : u32,
    @align(4) origin_y_1 : u32,
    @align(16) coefficients_0 : array<vec4<f32>, i32(9)>,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) temperature_0 : TemperatureParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(0) @group(0) var<storage, read> input_0 : array<f32>;

@binding(1) @group(0) var<storage, read_write> output_0 : array<f32>;

fn coefficient_0( x_0 : u32,  y_0 : u32) -> f32
{
    var width_1 : u32 = params_0.temperature_0.pattern_width_0;
    var _S1 : u32 = y_0 % params_0.temperature_0.pattern_height_0;
    var _S2 : u32 = _S1 * width_1;
    var _S3 : u32 = x_0 % width_1;
    var i_0 : u32 = _S2 + _S3;
    return params_0.temperature_0.coefficients_0[i_0 / u32(4)][i_0 % u32(4)];
}

fn temperature_1( value_0 : f32,  coefficient_1 : f32) -> f32
{
    return value_0 * coefficient_1;
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_0 : vec3<u32>)
{
    var _S4 : u32 = id_0.x;
    var _S5 : bool;
    if((_S4 >= (params_0.pass_0.width_0)))
    {
        _S5 = true;
    }
    else
    {
        _S5 = ((id_0.y) >= (params_0.pass_0.height_0));
    }
    if(_S5)
    {
        return;
    }
    var col_0 : u32 = params_0.pass_0.origin_x_0 + _S4;
    var row_0 : u32 = params_0.pass_0.origin_y_0 + id_0.y;
    var _S6 : f32 = coefficient_0(params_0.temperature_0.origin_x_1 + col_0, params_0.temperature_0.origin_y_1 + row_0);
    var i_1 : u32 = row_0 * params_0.pass_0.width_0 + col_0;
    var value_1 : f32 = input_0[i_1 - params_0.pass_0.input_base_0];
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
        var _S7 : f32 = temperature_1(value_1, _S6);
        var _S8 : u32 = k_0 + u32(1);
        value_1 = _S7;
        k_0 = _S8;
    }
    output_0[i_1 - params_0.pass_0.output_base_0] = value_1;
    return;
}

