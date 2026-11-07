// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2011-2025  darktable developers (ported from src/iop/flip.c)
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

// Generated from slang/flip.slang by tools/build-kernels.sh. Do not edit.

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

struct FlipParams_std140_0
{
    @align(16) input_width_0 : u32,
    @align(4) input_height_0 : u32,
    @align(8) orientation_0 : u32,
    @align(4) reserved_0 : u32,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) flip_0 : FlipParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(1) @group(0) var<storage, read_write> output_0 : array<vec4<f32>>;

@binding(0) @group(0) var<storage, read> input_0 : array<vec4<f32>>;

fn source_0( xy_0 : vec2<u32>,  p_0 : ptr<function, FlipParams_std140_0>) -> vec2<u32>
{
    var at_0 : vec2<u32>;
    var _S1 : u32 = (*p_0).orientation_0;
    var _S2 : vec2<u32>;
    if(((((*p_0).orientation_0) & (u32(4)))) != u32(0))
    {
        _S2 = xy_0.yx;
    }
    else
    {
        _S2 = xy_0;
    }
    at_0 = _S2;
    if(((_S1 & (u32(2)))) != u32(0))
    {
        at_0[i32(0)] = (*p_0).input_width_0 - u32(1) - at_0.x;
    }
    if(((_S1 & (u32(1)))) != u32(0))
    {
        at_0[i32(1)] = (*p_0).input_height_0 - u32(1) - at_0.y;
    }
    return at_0;
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_0 : vec3<u32>)
{
    var _S3 : u32 = id_0.x;
    var _S4 : bool;
    if((_S3 >= (params_0.pass_0.width_0)))
    {
        _S4 = true;
    }
    else
    {
        _S4 = ((id_0.y) >= (params_0.pass_0.height_0));
    }
    if(_S4)
    {
        return;
    }
    var _S5 : FlipParams_std140_0 = params_0.flip_0;
    var _S6 : vec2<u32> = source_0(id_0.xy, &(_S5));
    output_0[id_0.y * params_0.pass_0.width_0 + _S3] = input_0[_S6.y * _S5.input_width_0 + _S6.x];
    return;
}

