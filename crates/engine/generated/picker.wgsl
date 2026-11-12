// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2016-2025  darktable developers (ported from src/common/color_picker.c and src/common/bspline.h)
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

// Generated from slang/picker.slang by tools/build-kernels.sh. Do not edit.

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

struct PickerBox_std140_0
{
    @align(16) bounds_0 : vec4<u32>,
    @align(16) tiles_0 : vec4<u32>,
};

struct _Array_std140_PickerBox64_0
{
    @align(16) data_0 : array<PickerBox_std140_0, i32(64)>,
};

struct PickerParams_std140_0
{
    @align(16) count_0 : u32,
    @align(4) across_0 : u32,
    @align(8) end_0 : u32,
    @align(4) first_0 : u32,
    @align(16) results_0 : u32,
    @align(4) _pad0_0 : u32,
    @align(8) _pad1_0 : u32,
    @align(4) _pad2_0 : u32,
    @align(16) boxes_0 : _Array_std140_PickerBox64_0,
    @align(16) pattern_0 : vec4<u32>,
    @align(16) colours_0 : array<vec4<u32>, i32(9)>,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) picker_0 : PickerParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(0) @group(0) var<storage, read> image_0 : array<vec4<f32>>;

struct Partial_std430_0
{
    @align(16) sum_0 : vec4<f32>,
    @align(16) low_0 : vec4<f32>,
    @align(16) high_0 : vec4<f32>,
};

@binding(1) @group(0) var<storage, read_write> partials_0 : array<Partial_std430_0>;

@binding(4) @group(0) var<storage, read> mosaic_0 : array<f32>;

@binding(3) @group(0) var<storage, read_write> results_1 : array<Partial_std430_0>;

struct PickerBox_0
{
     bounds_0 : vec4<u32>,
     tiles_0 : vec4<u32>,
};

struct Place_0
{
     g_0 : u32,
     working_0 : bool,
     box_0 : PickerBox_0,
     corner_0 : vec2<u32>,
};

fn place_0( group_0 : vec3<u32>,  local_0 : vec3<u32>) -> Place_0
{
    var p_0 : Place_0;
    var _S1 : u32 = params_0.picker_0.first_0 + group_0.y * params_0.picker_0.across_0 + group_0.x;
    p_0.g_0 = _S1;
    p_0.working_0 = (_S1 < (params_0.picker_0.end_0));
    var b_0 : u32 = u32(0);
    var k_0 : u32 = u32(1);
    loop
    {
        if((k_0 < (params_0.picker_0.count_0)))
        {
        }
        else
        {
            break;
        }
        if(((p_0.g_0) >= (params_0.picker_0.boxes_0.data_0[k_0].tiles_0.x)))
        {
            b_0 = k_0;
        }
        k_0 = k_0 + u32(1);
    }
    p_0.box_0.bounds_0 = params_0.picker_0.boxes_0.data_0[b_0].bounds_0;
    p_0.box_0.tiles_0 = params_0.picker_0.boxes_0.data_0[b_0].tiles_0;
    var tile_0 : u32;
    if(p_0.working_0)
    {
        tile_0 = p_0.g_0 - p_0.box_0.tiles_0.x;
    }
    else
    {
        tile_0 = u32(0);
    }
    var _S2 : vec2<u32> = p_0.box_0.bounds_0.xy;
    var _S3 : u32 = tile_0 % p_0.box_0.tiles_0.y;
    var _S4 : u32 = tile_0 / p_0.box_0.tiles_0.y;
    p_0.corner_0 = _S2 + vec2<u32>(_S3, _S4) * vec2<u32>(u32(64)) + local_0.xy;
    return p_0;
}

fn pixel_0( x_0 : u32,  y_0 : u32) -> vec4<f32>
{
    return image_0[y_0 * params_0.pass_0.width_0 + x_0 - params_0.pass_0.input_base_0 / u32(4)];
}

fn column_0( x_1 : u32,  r0_0 : u32,  r1_0 : u32,  r2_0 : u32,  r3_0 : u32,  r4_0 : u32) -> vec4<f32>
{
    var _S5 : vec4<f32> = vec4<f32>(0.0625f);
    var _S6 : vec4<f32> = vec4<f32>(0.25f);
    return _S5 * pixel_0(x_1, r0_0) + _S6 * pixel_0(x_1, r1_0) + vec4<f32>(0.375f) * pixel_0(x_1, r2_0) + _S6 * pixel_0(x_1, r3_0) + _S5 * pixel_0(x_1, r4_0);
}

fn blurred_0( at_0 : vec2<u32>) -> vec4<f32>
{
    var _S7 : u32 = at_0.x;
    var x_2 : i32 = i32(_S7);
    var _S8 : u32 = at_0.y;
    var y_1 : i32 = i32(_S8);
    var right_0 : i32 = i32(params_0.pass_0.width_0) - i32(1);
    var bottom_0 : i32 = i32(params_0.pass_0.height_0) - i32(1);
    var r0_1 : u32 = u32(max(y_1 - i32(2), i32(0)));
    var r1_1 : u32 = u32(max(y_1 - i32(1), i32(0)));
    var r3_1 : u32 = u32(min(y_1 + i32(1), bottom_0));
    var r4_1 : u32 = u32(min(y_1 + i32(2), bottom_0));
    var _S9 : vec4<f32> = vec4<f32>(0.0625f);
    var _S10 : vec4<f32> = vec4<f32>(0.25f);
    return _S9 * column_0(u32(max(x_2 - i32(2), i32(0))), r0_1, r1_1, _S8, r3_1, r4_1) + _S10 * column_0(u32(max(x_2 - i32(1), i32(0))), r0_1, r1_1, _S8, r3_1, r4_1) + vec4<f32>(0.375f) * column_0(_S7, r0_1, r1_1, _S8, r3_1, r4_1) + _S10 * column_0(u32(min(x_2 + i32(1), right_0)), r0_1, r1_1, _S8, r3_1, r4_1) + _S9 * column_0(u32(min(x_2 + i32(2), right_0)), r0_1, r1_1, _S8, r3_1, r4_1);
}

var<workgroup> shared_sum_0 : array<vec4<f32>, i32(256)>;

var<workgroup> shared_low_0 : array<vec4<f32>, i32(256)>;

var<workgroup> shared_high_0 : array<vec4<f32>, i32(256)>;

fn reduce_0( index_0 : u32,  sum_1 : vec4<f32>,  low_1 : vec4<f32>,  high_1 : vec4<f32>)
{
    shared_sum_0[index_0] = sum_1;
    shared_low_0[index_0] = low_1;
    shared_high_0[index_0] = high_1;
    workgroupBarrier();
    var stride_0 : u32 = u32(128);
    loop
    {
        if((stride_0 > u32(0)))
        {
        }
        else
        {
            break;
        }
        if((index_0 < stride_0))
        {
            var _S11 : u32 = index_0 + stride_0;
            shared_sum_0[index_0] = shared_sum_0[index_0] + shared_sum_0[_S11];
            shared_low_0[index_0] = min(shared_low_0[index_0], shared_low_0[_S11]);
            shared_high_0[index_0] = max(shared_high_0[index_0], shared_high_0[_S11]);
        }
        workgroupBarrier();
        stride_0 = stride_0 / u32(2);
    }
    return;
}

struct Partial_0
{
     sum_0 : vec4<f32>,
     low_0 : vec4<f32>,
     high_0 : vec4<f32>,
};

fn partial_0( p_1 : Place_0,  index_1 : u32)
{
    var _S12 : bool;
    if(p_1.working_0)
    {
        _S12 = index_1 == u32(0);
    }
    else
    {
        _S12 = false;
    }
    if(_S12)
    {
        var partial_1 : Partial_0;
        partial_1.sum_0 = shared_sum_0[i32(0)];
        partial_1.low_0 = shared_low_0[i32(0)];
        partial_1.high_0 = shared_high_0[i32(0)];
        partials_0[p_1.g_0].sum_0 = partial_1.sum_0;
        partials_0[p_1.g_0].low_0 = partial_1.low_0;
        partials_0[p_1.g_0].high_0 = partial_1.high_0;
    }
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn picker_rgba(@builtin(workgroup_id) group_1 : vec3<u32>, @builtin(local_invocation_id) local_1 : vec3<u32>, @builtin(local_invocation_index) index_2 : u32)
{
    var p_2 : Place_0 = place_0(group_1, local_1);
    var _S13 : bool = (((p_2.box_0.tiles_0.w) & (u32(1)))) != u32(0);
    var _S14 : vec4<f32> = vec4<f32>(0.0f);
    var _S15 : vec4<f32> = vec4<f32>(3.4028234663852886e+38f);
    var _S16 : vec4<f32> = vec4<f32>(-3.4028234663852886e+38f);
    var sum_2 : vec4<f32>;
    var low_2 : vec4<f32>;
    var high_2 : vec4<f32>;
    if(p_2.working_0)
    {
        var sum_3 : vec4<f32> = _S14;
        var low_3 : vec4<f32> = _S15;
        var high_3 : vec4<f32> = _S16;
        var by_0 : u32 = u32(0);
        loop
        {
            if((by_0 < u32(4)))
            {
            }
            else
            {
                break;
            }
            sum_2 = sum_3;
            low_2 = low_3;
            high_2 = high_3;
            var bx_0 : u32 = u32(0);
            loop
            {
                if((bx_0 < u32(4)))
                {
                }
                else
                {
                    break;
                }
                var at_1 : vec2<u32> = p_2.corner_0 + vec2<u32>(bx_0, by_0) * vec2<u32>(u32(16));
                var _S17 : u32 = at_1.x;
                var _S18 : bool;
                if((_S17 < (p_2.box_0.bounds_0.z)))
                {
                    _S18 = ((at_1.y) < (p_2.box_0.bounds_0.w));
                }
                else
                {
                    _S18 = false;
                }
                if(_S18)
                {
                    var v_0 : vec4<f32>;
                    if(_S13)
                    {
                        v_0 = blurred_0(at_1);
                    }
                    else
                    {
                        v_0 = pixel_0(_S17, at_1.y);
                    }
                    var _S19 : vec4<f32> = min(low_2, v_0);
                    var _S20 : vec4<f32> = max(high_2, v_0);
                    sum_2 = sum_2 + v_0;
                    low_2 = _S19;
                    high_2 = _S20;
                }
                bx_0 = bx_0 + u32(1);
            }
            var _S21 : u32 = by_0 + u32(1);
            sum_3 = sum_2;
            low_3 = low_2;
            high_3 = high_2;
            by_0 = _S21;
        }
        sum_2 = sum_3;
        low_2 = low_3;
        high_2 = high_3;
    }
    else
    {
        sum_2 = _S14;
        low_2 = _S15;
        high_2 = _S16;
    }
    reduce_0(index_2, sum_2, low_2, high_2);
    partial_0(p_2, index_2);
    return;
}

fn colour_0( x_3 : u32,  y_2 : u32) -> u32
{
    var pattern_1 : vec4<u32> = params_0.picker_0.pattern_0;
    var _S22 : u32 = (params_0.picker_0.pattern_0.w + y_2) % params_0.picker_0.pattern_0.y;
    var _S23 : u32 = pattern_1.x;
    var _S24 : u32 = _S22 * _S23;
    var _S25 : u32 = (pattern_1.z + x_3) % _S23;
    var i_0 : u32 = _S24 + _S25;
    return params_0.picker_0.colours_0[i_0 / u32(4)][i_0 % u32(4)];
}

fn add_at_0( v_1 : vec4<f32>,  c_0 : u32,  x_4 : f32) -> vec4<f32>
{
    return select(v_1, v_1 + vec4<f32>(x_4), vec4<u32>(u32(0), u32(1), u32(2), u32(3)) == vec4<u32>(c_0));
}

fn set_at_0( v_2 : vec4<f32>,  c_1 : u32,  x_5 : f32) -> vec4<f32>
{
    return select(v_2, vec4<f32>(x_5), vec4<u32>(u32(0), u32(1), u32(2), u32(3)) == vec4<u32>(c_1));
}

@compute
@workgroup_size(16, 16, 1)
fn picker_mosaic(@builtin(workgroup_id) group_2 : vec3<u32>, @builtin(local_invocation_id) local_2 : vec3<u32>, @builtin(local_invocation_index) index_3 : u32)
{
    var p_3 : Place_0 = place_0(group_2, local_2);
    var _S26 : vec4<f32> = vec4<f32>(0.0f);
    var _S27 : vec4<f32> = vec4<f32>(3.4028234663852886e+38f);
    var _S28 : vec4<f32> = vec4<f32>(-3.4028234663852886e+38f);
    var sum_4 : vec4<f32>;
    var low_4 : vec4<f32>;
    var high_4 : vec4<f32>;
    if(p_3.working_0)
    {
        var sum_5 : vec4<f32> = _S26;
        var low_5 : vec4<f32> = _S27;
        var high_5 : vec4<f32> = _S28;
        var by_1 : u32 = u32(0);
        loop
        {
            if((by_1 < u32(4)))
            {
            }
            else
            {
                break;
            }
            sum_4 = sum_5;
            low_4 = low_5;
            high_4 = high_5;
            var bx_1 : u32 = u32(0);
            loop
            {
                if((bx_1 < u32(4)))
                {
                }
                else
                {
                    break;
                }
                var at_2 : vec2<u32> = p_3.corner_0 + vec2<u32>(bx_1, by_1) * vec2<u32>(u32(16));
                var _S29 : u32 = at_2.x;
                var _S30 : bool;
                if((_S29 < (p_3.box_0.bounds_0.z)))
                {
                    _S30 = ((at_2.y) < (p_3.box_0.bounds_0.w));
                }
                else
                {
                    _S30 = false;
                }
                if(_S30)
                {
                    var _S31 : u32 = at_2.y;
                    var v_3 : f32 = mosaic_0[_S31 * params_0.pass_0.width_0 + _S29 - params_0.pass_0.input_base_0];
                    var c_2 : u32 = colour_0(_S29, _S31);
                    var _S32 : vec4<f32> = set_at_0(low_4, c_2, min(low_4[c_2], v_3));
                    var _S33 : vec4<f32> = set_at_0(high_4, c_2, max(high_4[c_2], v_3));
                    sum_4 = add_at_0(sum_4, c_2, v_3);
                    low_4 = _S32;
                    high_4 = _S33;
                }
                bx_1 = bx_1 + u32(1);
            }
            var _S34 : u32 = by_1 + u32(1);
            sum_5 = sum_4;
            low_5 = low_4;
            high_5 = high_4;
            by_1 = _S34;
        }
        sum_4 = sum_5;
        low_4 = low_5;
        high_4 = high_5;
    }
    else
    {
        sum_4 = _S26;
        low_4 = _S27;
        high_4 = _S28;
    }
    reduce_0(index_3, sum_4, low_4, high_4);
    partial_0(p_3, index_3);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn picker_combine(@builtin(workgroup_id) group_3 : vec3<u32>, @builtin(local_invocation_index) index_4 : u32)
{
    var _S35 : u32 = group_3.x;
    var _S36 : PickerBox_std140_0 = params_0.picker_0.boxes_0.data_0[_S35];
    var _S37 : vec4<f32> = vec4<f32>(3.4028234663852886e+38f);
    var _S38 : vec4<f32> = vec4<f32>(-3.4028234663852886e+38f);
    var sum_6 : vec4<f32> = vec4<f32>(0.0f);
    var low_6 : vec4<f32> = _S37;
    var high_6 : vec4<f32> = _S38;
    var i_1 : u32 = index_4;
    loop
    {
        if((i_1 < (_S36.tiles_0.z)))
        {
        }
        else
        {
            break;
        }
        var _S39 : vec4<f32> = sum_6 + partials_0[_S36.tiles_0.x + i_1].sum_0;
        var _S40 : vec4<f32> = min(low_6, partials_0[_S36.tiles_0.x + i_1].low_0);
        var _S41 : vec4<f32> = max(high_6, partials_0[_S36.tiles_0.x + i_1].high_0);
        var i_2 : u32 = i_1 + u32(256);
        sum_6 = _S39;
        low_6 = _S40;
        high_6 = _S41;
        i_1 = i_2;
    }
    reduce_0(index_4, sum_6, low_6, high_6);
    if(index_4 == u32(0))
    {
        var result_0 : Partial_0;
        result_0.sum_0 = shared_sum_0[i32(0)];
        result_0.low_0 = shared_low_0[i32(0)];
        result_0.high_0 = shared_high_0[i32(0)];
        results_1[params_0.picker_0.results_0 + _S35].sum_0 = result_0.sum_0;
        results_1[params_0.picker_0.results_0 + _S35].low_0 = result_0.low_0;
        results_1[params_0.picker_0.results_0 + _S35].high_0 = result_0.high_0;
    }
    return;
}

