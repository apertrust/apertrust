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
    @align(8) workgroups_0 : u32,
    @align(4) _pad0_0 : u32,
    @align(16) boxes_0 : _Array_std140_PickerBox64_0,
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

@binding(3) @group(0) var<storage, read_write> results_0 : array<Partial_std430_0>;

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
            var _S1 : u32 = index_0 + stride_0;
            shared_sum_0[index_0] = shared_sum_0[index_0] + shared_sum_0[_S1];
            shared_low_0[index_0] = min(shared_low_0[index_0], shared_low_0[_S1]);
            shared_high_0[index_0] = max(shared_high_0[index_0], shared_high_0[_S1]);
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

@compute
@workgroup_size(16, 16, 1)
fn picker_rgba(@builtin(workgroup_id) group_0 : vec3<u32>, @builtin(local_invocation_id) local_0 : vec3<u32>, @builtin(local_invocation_index) index_1 : u32)
{
    var g_0 : u32 = group_0.y * params_0.picker_0.across_0 + group_0.x;
    var working_0 : bool = (g_0 < (params_0.picker_0.workgroups_0));
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
        if((g_0 >= (params_0.picker_0.boxes_0.data_0[k_0].tiles_0.x)))
        {
            b_0 = k_0;
        }
        k_0 = k_0 + u32(1);
    }
    var box_0 : PickerBox_std140_0 = params_0.picker_0.boxes_0.data_0[b_0];
    var tile_0 : u32;
    if(working_0)
    {
        tile_0 = g_0 - box_0.tiles_0.x;
    }
    else
    {
        tile_0 = u32(0);
    }
    var _S2 : vec2<u32> = box_0.bounds_0.xy;
    var _S3 : u32 = box_0.tiles_0.y;
    var _S4 : u32 = tile_0 % _S3;
    var _S5 : u32 = tile_0 / _S3;
    var _S6 : vec2<u32> = _S2 + vec2<u32>(_S4, _S5) * vec2<u32>(u32(64)) + local_0.xy;
    var _S7 : vec4<f32> = vec4<f32>(0.0f);
    var _S8 : vec4<f32> = vec4<f32>(3.4028234663852886e+38f);
    var _S9 : vec4<f32> = vec4<f32>(-3.4028234663852886e+38f);
    var sum_2 : vec4<f32>;
    var low_2 : vec4<f32>;
    var high_2 : vec4<f32>;
    var _S10 : bool;
    if(working_0)
    {
        var sum_3 : vec4<f32> = _S7;
        var low_3 : vec4<f32> = _S8;
        var high_3 : vec4<f32> = _S9;
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
                var at_0 : vec2<u32> = _S6 + vec2<u32>(bx_0, by_0) * vec2<u32>(u32(16));
                var _S11 : u32 = at_0.x;
                if((_S11 < (box_0.bounds_0.z)))
                {
                    _S10 = ((at_0.y) < (box_0.bounds_0.w));
                }
                else
                {
                    _S10 = false;
                }
                if(_S10)
                {
                    var v_0 : vec4<f32> = image_0[at_0.y * params_0.pass_0.width_0 + _S11];
                    var _S12 : vec4<f32> = min(low_2, v_0);
                    var _S13 : vec4<f32> = max(high_2, v_0);
                    sum_2 = sum_2 + v_0;
                    low_2 = _S12;
                    high_2 = _S13;
                }
                bx_0 = bx_0 + u32(1);
            }
            var _S14 : u32 = by_0 + u32(1);
            sum_3 = sum_2;
            low_3 = low_2;
            high_3 = high_2;
            by_0 = _S14;
        }
        sum_2 = sum_3;
        low_2 = low_3;
        high_2 = high_3;
    }
    else
    {
        sum_2 = _S7;
        low_2 = _S8;
        high_2 = _S9;
    }
    reduce_0(index_1, sum_2, low_2, high_2);
    if(working_0)
    {
        _S10 = index_1 == u32(0);
    }
    else
    {
        _S10 = false;
    }
    if(_S10)
    {
        var partial_0 : Partial_0;
        partial_0.sum_0 = shared_sum_0[i32(0)];
        partial_0.low_0 = shared_low_0[i32(0)];
        partial_0.high_0 = shared_high_0[i32(0)];
        partials_0[g_0].sum_0 = partial_0.sum_0;
        partials_0[g_0].low_0 = partial_0.low_0;
        partials_0[g_0].high_0 = partial_0.high_0;
    }
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn picker_combine(@builtin(workgroup_id) group_1 : vec3<u32>, @builtin(local_invocation_index) index_2 : u32)
{
    var _S15 : u32 = group_1.x;
    var _S16 : PickerBox_std140_0 = params_0.picker_0.boxes_0.data_0[_S15];
    var _S17 : vec4<f32> = vec4<f32>(3.4028234663852886e+38f);
    var _S18 : vec4<f32> = vec4<f32>(-3.4028234663852886e+38f);
    var sum_4 : vec4<f32> = vec4<f32>(0.0f);
    var low_4 : vec4<f32> = _S17;
    var high_4 : vec4<f32> = _S18;
    var i_0 : u32 = index_2;
    loop
    {
        if((i_0 < (_S16.tiles_0.z)))
        {
        }
        else
        {
            break;
        }
        var _S19 : vec4<f32> = sum_4 + partials_0[_S16.tiles_0.x + i_0].sum_0;
        var _S20 : vec4<f32> = min(low_4, partials_0[_S16.tiles_0.x + i_0].low_0);
        var _S21 : vec4<f32> = max(high_4, partials_0[_S16.tiles_0.x + i_0].high_0);
        var i_1 : u32 = i_0 + u32(256);
        sum_4 = _S19;
        low_4 = _S20;
        high_4 = _S21;
        i_0 = i_1;
    }
    reduce_0(index_2, sum_4, low_4, high_4);
    if(index_2 == u32(0))
    {
        var result_0 : Partial_0;
        result_0.sum_0 = shared_sum_0[i32(0)];
        result_0.low_0 = shared_low_0[i32(0)];
        result_0.high_0 = shared_high_0[i32(0)];
        results_0[_S15].sum_0 = result_0.sum_0;
        results_0[_S15].low_0 = result_0.low_0;
        results_0[_S15].high_0 = result_0.high_0;
    }
    return;
}

