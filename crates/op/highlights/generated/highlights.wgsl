// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/iop/highlights.c,
// src/iop/hlreconstruct/opposed.c and src/iop/hlreconstruct/segbased.c)
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

// Generated from slang/highlights.slang by tools/build-kernels.sh. Do not edit.

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

struct HighlightsParams_std140_0
{
    @align(16) width_1 : u32,
    @align(4) height_1 : u32,
    @align(8) mask_width_0 : u32,
    @align(4) mask_height_0 : u32,
    @align(16) mask_size_0 : u32,
    @align(4) mask_stride_0 : u32,
    @align(8) partials_across_0 : u32,
    @align(4) partials_0 : u32,
    @align(16) clips_0 : vec4<f32>,
    @align(16) correction_0 : vec4<f32>,
    @align(16) colors_0 : array<vec4<u32>, i32(9)>,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) highlights_0 : HighlightsParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(0) @group(0) var<storage, read> input_0 : array<f32>;

@binding(10) @group(0) var<storage, read_write> clipped_mask_0 : array<u32>;

@binding(11) @group(0) var<storage, read_write> dilated_mask_0 : array<u32>;

struct Partial_std430_0
{
    @align(16) sums_0 : vec4<f32>,
    @align(16) counts_0 : vec4<u32>,
};

@binding(12) @group(0) var<storage, read_write> partials_1 : array<Partial_std430_0>;

@binding(13) @group(0) var<storage, read_write> chroma_0 : array<vec4<f32>>;

@binding(1) @group(0) var<storage, read_write> output_0 : array<f32>;

fn color_at_0( col_0 : u32,  row_0 : u32) -> u32
{
    var position_0 : u32 = row_0 % u32(6) * u32(6) + col_0 % u32(6);
    return params_0.highlights_0.colors_0[position_0 / u32(4)][position_0 % u32(4)];
}

fn mosaic_at_0( col_1 : u32,  row_1 : u32) -> f32
{
    return input_0[row_1 * params_0.highlights_0.width_1 + col_1 - params_0.pass_0.input_base_0];
}

fn clip_of_0( color_0 : u32) -> f32
{
    return params_0.highlights_0.clips_0[color_0];
}

@compute
@workgroup_size(16, 16, 1)
fn opposed_mask(@builtin(global_invocation_id) id_0 : vec3<u32>)
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
    var mask_width_1 : u32 = params_0.highlights_0.mask_width_0;
    var i_0 : u32 = (params_0.pass_0.origin_y_0 + id_0.y) * params_0.highlights_0.mask_width_0 + _S1;
    if((i_0 >= (params_0.highlights_0.mask_size_0)))
    {
        return;
    }
    var mrow_0 : u32 = i_0 / mask_width_1;
    var mcol_0 : u32 = i_0 % mask_width_1;
    if(((mrow_0 + u32(1)) < (params_0.highlights_0.mask_height_0)))
    {
        _S2 = ((mcol_0 + u32(1)) < mask_width_1);
    }
    else
    {
        _S2 = false;
    }
    var bits_0 : u32;
    if(_S2)
    {
        var y_0 : u32 = u32(0);
        var bits_1 : u32 = u32(0);
        loop
        {
            if((y_0 < u32(3)))
            {
            }
            else
            {
                break;
            }
            var x_0 : u32 = u32(0);
            bits_0 = bits_1;
            loop
            {
                if((x_0 < u32(3)))
                {
                }
                else
                {
                    break;
                }
                var row_2 : u32 = u32(3) * mrow_0 + y_0;
                var col_2 : u32 = u32(3) * mcol_0 + x_0;
                var color_1 : u32 = color_at_0(col_2, row_2);
                if(((mosaic_at_0(col_2, row_2)) >= (clip_of_0(color_1))))
                {
                    bits_0 = (bits_0 | (((u32(1) << (color_1)))));
                }
                x_0 = x_0 + u32(1);
            }
            y_0 = y_0 + u32(1);
            bits_1 = bits_0;
        }
        bits_0 = bits_1;
    }
    else
    {
        bits_0 = u32(0);
    }
    clipped_mask_0[i_0] = bits_0;
    return;
}

fn mask_entry_0( id_1 : vec3<u32>) -> u32
{
    return (params_0.pass_0.origin_y_0 + id_1.y) * params_0.highlights_0.mask_stride_0 + id_1.x;
}

fn dilated_0( i_1 : u32) -> u32
{
    var _S3 : i32 = i32(params_0.highlights_0.mask_width_0);
    var dy_0 : i32 = i32(-3);
    var bits_2 : u32 = u32(0);
    loop
    {
        if((dy_0 <= i32(3)))
        {
        }
        else
        {
            break;
        }
        var dx_0 : i32 = i32(-3);
        var bits_3 : u32 = bits_2;
        loop
        {
            if((dx_0 <= i32(3)))
            {
            }
            else
            {
                break;
            }
            var _S4 : bool;
            if((abs(dx_0)) == i32(3))
            {
                _S4 = (abs(dy_0)) == i32(3);
            }
            else
            {
                _S4 = false;
            }
            if(_S4)
            {
                dx_0 = dx_0 + i32(1);
                continue;
            }
            bits_3 = (bits_3 | (clipped_mask_0[i32(i_1) + dy_0 * _S3 + dx_0]));
            dx_0 = dx_0 + i32(1);
        }
        dy_0 = dy_0 + i32(1);
        bits_2 = bits_3;
    }
    return bits_2;
}

@compute
@workgroup_size(16, 16, 1)
fn opposed_dilate(@builtin(global_invocation_id) id_2 : vec3<u32>)
{
    var safe_0 : bool;
    if(((id_2.x) >= (params_0.pass_0.width_0)))
    {
        safe_0 = true;
    }
    else
    {
        safe_0 = ((id_2.y) >= (params_0.pass_0.height_0));
    }
    if(safe_0)
    {
        return;
    }
    var i_2 : u32 = mask_entry_0(id_2);
    var mask_width_2 : i32 = i32(params_0.highlights_0.mask_width_0);
    var mask_height_1 : i32 = i32(params_0.highlights_0.mask_height_0);
    var _S5 : i32 = i32(i_2);
    var mrow_1 : i32 = _S5 / mask_width_2;
    var mcol_1 : i32 = _S5 % mask_width_2;
    var bits_4 : u32;
    if((mrow_1 < mask_height_1))
    {
        if((mcol_1 >= i32(3)))
        {
            safe_0 = (mrow_1 >= i32(3));
        }
        else
        {
            safe_0 = false;
        }
        if(safe_0)
        {
            safe_0 = (mcol_1 < (mask_width_2 - i32(4)));
        }
        else
        {
            safe_0 = false;
        }
        if(safe_0)
        {
            safe_0 = (mrow_1 < (mask_height_1 - i32(4)));
        }
        else
        {
            safe_0 = false;
        }
        if(safe_0)
        {
            bits_4 = dilated_0(i_2);
        }
        else
        {
            bits_4 = clipped_mask_0[i_2];
        }
        bits_4 = (bits_4 & (u32(7)));
    }
    else
    {
        bits_4 = u32(0);
    }
    dilated_mask_0[i_2] = bits_4;
    return;
}

fn dilated_at_0( i_3 : u32,  color_2 : u32) -> bool
{
    var size_0 : u32 = params_0.highlights_0.mask_size_0;
    var _S6 : u32;
    var _S7 : u32;
    if((i_3 >= (params_0.highlights_0.mask_size_0)))
    {
        var _S8 : u32 = i_3 - size_0;
        _S6 = color_2 + u32(1);
        _S7 = _S8;
    }
    else
    {
        _S6 = color_2;
        _S7 = i_3;
    }
    var _S9 : bool;
    if((_S6 < u32(3)))
    {
        _S9 = ((((dilated_mask_0[_S7] >> (_S6))) & (u32(1)))) != u32(0);
    }
    else
    {
        _S9 = false;
    }
    return _S9;
}

fn add_at_0( v_0 : vec3<f32>,  c_0 : u32,  x_1 : f32) -> vec3<f32>
{
    return select(v_0, v_0 + vec3<f32>(x_1), vec3<u32>(u32(0), u32(1), u32(2)) == vec3<u32>(c_0));
}

fn cube_root_0( x_2 : f32) -> f32
{
    if((x_2 <= 0.0f))
    {
        return 0.0f;
    }
    var y_1 : f32 = pow(x_2, 0.3333333432674408f);
    return y_1 - (y_1 * y_1 * y_1 - x_2) / (3.0f * y_1 * y_1);
}

fn mean_root_0( c_1 : u32,  mean_0 : f32,  count_0 : f32) -> f32
{
    var _S10 : f32;
    if((count_0 > 0.0f))
    {
        _S10 = cube_root_0(params_0.highlights_0.correction_0[c_1] * mean_0 / count_0);
    }
    else
    {
        _S10 = 0.0f;
    }
    return _S10;
}

fn opposing_average_0( col_3 : i32,  row_3 : i32) -> f32
{
    var _S11 : i32 = i32(params_0.highlights_0.width_1);
    var _S12 : i32 = i32(params_0.highlights_0.height_1);
    var _S13 : vec3<f32> = vec3<f32>(0.0f);
    var _S14 : i32 = max(i32(0), row_3 - i32(1));
    var mean_1 : vec3<f32> = _S13;
    var count_1 : vec3<f32> = _S13;
    var dy_1 : i32 = _S14;
    loop
    {
        if((dy_1 < (min(_S12 - i32(1), row_3 + i32(2)))))
        {
        }
        else
        {
            break;
        }
        var dx_1 : i32 = max(i32(0), col_3 - i32(1));
        loop
        {
            if((dx_1 < (min(_S11 - i32(1), col_3 + i32(2)))))
            {
            }
            else
            {
                break;
            }
            var _S15 : u32 = u32(dx_1);
            var _S16 : u32 = u32(dy_1);
            var c_2 : u32 = color_at_0(_S15, _S16);
            var _S17 : vec3<f32> = add_at_0(mean_1, c_2, max(0.0f, mosaic_at_0(_S15, _S16)));
            var _S18 : vec3<f32> = add_at_0(count_1, c_2, 1.0f);
            var _S19 : i32 = dx_1 + i32(1);
            mean_1 = _S17;
            count_1 = _S18;
            dx_1 = _S19;
        }
        dy_1 = dy_1 + i32(1);
    }
    var _S20 : f32 = mean_root_0(u32(0), mean_1.x, count_1.x);
    var _S21 : f32 = mean_root_0(u32(1), mean_1.y, count_1.y);
    var _S22 : f32 = mean_root_0(u32(2), mean_1.z, count_1.z);
    var opposing_0 : vec3<f32> = vec3<f32>(0.5f * (_S21 + _S22), 0.5f * (_S20 + _S22), 0.5f * (_S20 + _S21));
    var _S23 : u32 = color_at_0(u32(col_3), u32(row_3));
    var average_0 : f32 = opposing_0[_S23];
    return average_0 * average_0 * opposing_0[_S23];
}

fn add_at_1( v_1 : vec4<f32>,  c_3 : u32,  x_3 : f32) -> vec4<f32>
{
    return select(v_1, v_1 + vec4<f32>(x_3), vec4<u32>(u32(0), u32(1), u32(2), u32(3)) == vec4<u32>(c_3));
}

fn add_at_2( v_2 : vec4<u32>,  c_4 : u32,  x_4 : u32) -> vec4<u32>
{
    return select(v_2, v_2 + vec4<u32>(x_4), vec4<u32>(u32(0), u32(1), u32(2), u32(3)) == vec4<u32>(c_4));
}

var<workgroup> shared_sums_0 : array<vec4<f32>, i32(256)>;

var<workgroup> shared_counts_0 : array<vec4<u32>, i32(256)>;

struct Partial_0
{
     sums_0 : vec4<f32>,
     counts_0 : vec4<u32>,
};

@compute
@workgroup_size(16, 16, 1)
fn opposed_sums(@builtin(global_invocation_id) id_3 : vec3<u32>, @builtin(workgroup_id) group_0 : vec3<u32>, @builtin(local_invocation_index) t_0 : u32)
{
    var _S24 : u32 = params_0.highlights_0.width_1;
    var _S25 : u32 = params_0.highlights_0.height_1;
    var _S26 : vec4<f32> = vec4<f32>(0.0f);
    var _S27 : vec4<u32> = vec4<u32>(vec4<i32>(i32(0)));
    var _S28 : u32 = id_3.x;
    var _S29 : bool;
    if((_S28 < (params_0.pass_0.width_0)))
    {
        _S29 = ((id_3.y) < (params_0.pass_0.height_0));
    }
    else
    {
        _S29 = false;
    }
    var sums_1 : vec4<f32>;
    var counts_1 : vec4<u32>;
    var stride_0 : u32;
    if(_S29)
    {
        var sums_2 : vec4<f32> = _S26;
        var counts_2 : vec4<u32> = _S27;
        stride_0 = u32(0);
        loop
        {
            if((stride_0 < u32(4)))
            {
            }
            else
            {
                break;
            }
            sums_1 = sums_2;
            counts_1 = counts_2;
            var x_5 : u32 = u32(0);
            loop
            {
                if((x_5 < u32(4)))
                {
                }
                else
                {
                    break;
                }
                var row_4 : u32 = u32(4) * (params_0.pass_0.origin_y_0 + id_3.y) + stride_0;
                var col_4 : u32 = u32(4) * _S28 + x_5;
                if((row_4 >= _S25))
                {
                    _S29 = true;
                }
                else
                {
                    _S29 = (col_4 >= _S24);
                }
                if(_S29)
                {
                    x_5 = x_5 + u32(1);
                    continue;
                }
                var color_3 : u32 = color_at_0(col_4, row_4);
                var value_0 : f32 = mosaic_at_0(col_4, row_4);
                var clip_0 : f32 = clip_of_0(color_3);
                var entry_0 : u32 = row_4 / u32(3) * params_0.highlights_0.mask_width_0 + col_4 / u32(3);
                var _S30 : bool;
                if((value_0 < clip_0))
                {
                    _S30 = (value_0 > (0.20000000298023224f * clip_0));
                }
                else
                {
                    _S30 = false;
                }
                var _S31 : bool;
                if(_S30)
                {
                    _S31 = dilated_at_0(entry_0, color_3);
                }
                else
                {
                    _S31 = false;
                }
                var sums_3 : vec4<f32>;
                var counts_3 : vec4<u32>;
                if(_S31)
                {
                    var _S32 : vec4<u32> = add_at_2(counts_1, color_3, u32(1));
                    sums_3 = add_at_1(sums_1, color_3, value_0 - opposing_average_0(i32(col_4), i32(row_4)));
                    counts_3 = _S32;
                }
                else
                {
                    sums_3 = sums_1;
                    counts_3 = counts_1;
                }
                sums_1 = sums_3;
                counts_1 = counts_3;
                x_5 = x_5 + u32(1);
            }
            var _S33 : u32 = stride_0 + u32(1);
            sums_2 = sums_1;
            counts_2 = counts_1;
            stride_0 = _S33;
        }
        sums_1 = sums_2;
        counts_1 = counts_2;
    }
    else
    {
        sums_1 = _S26;
        counts_1 = _S27;
    }
    shared_sums_0[t_0] = sums_1;
    shared_counts_0[t_0] = counts_1;
    workgroupBarrier();
    stride_0 = u32(128);
    loop
    {
        if((stride_0 > u32(0)))
        {
        }
        else
        {
            break;
        }
        if((t_0 < stride_0))
        {
            shared_sums_0[t_0] = shared_sums_0[t_0] + shared_sums_0[t_0 + stride_0];
            shared_counts_0[t_0] = shared_counts_0[t_0] + shared_counts_0[t_0 + stride_0];
        }
        workgroupBarrier();
        stride_0 = stride_0 / u32(2);
    }
    if(t_0 == u32(0))
    {
        var partial_0 : Partial_0;
        partial_0.sums_0 = shared_sums_0[i32(0)];
        partial_0.counts_0 = shared_counts_0[i32(0)];
        partials_1[(group_0.y + params_0.pass_0.origin_y_0 / u32(16)) * params_0.highlights_0.partials_across_0 + group_0.x].sums_0 = partial_0.sums_0;
        partials_1[(group_0.y + params_0.pass_0.origin_y_0 / u32(16)) * params_0.highlights_0.partials_across_0 + group_0.x].counts_0 = partial_0.counts_0;
    }
    return;
}

fn color_offset_0( c_5 : u32) -> f32
{
    var count_2 : u32 = shared_counts_0[i32(0)][c_5];
    var _S34 : f32;
    if(((shared_counts_0[i32(0)][c_5]) > u32(100)))
    {
        _S34 = shared_sums_0[i32(0)][c_5] / f32(count_2);
    }
    else
    {
        _S34 = 0.0f;
    }
    return _S34;
}

@compute
@workgroup_size(16, 16, 1)
fn opposed_chroma(@builtin(local_invocation_index) t_1 : u32)
{
    var _S35 : vec4<f32> = vec4<f32>(0.0f);
    var _S36 : vec4<u32> = vec4<u32>(vec4<i32>(i32(0)));
    var i_4 : u32 = t_1;
    var sums_4 : vec4<f32> = _S35;
    var counts_4 : vec4<u32> = _S36;
    loop
    {
        if((i_4 < (params_0.highlights_0.partials_0)))
        {
        }
        else
        {
            break;
        }
        var sums_5 : vec4<f32> = sums_4 + partials_1[i_4].sums_0;
        var counts_5 : vec4<u32> = counts_4 + partials_1[i_4].counts_0;
        i_4 = i_4 + u32(256);
        sums_4 = sums_5;
        counts_4 = counts_5;
    }
    shared_sums_0[t_1] = sums_4;
    shared_counts_0[t_1] = counts_4;
    workgroupBarrier();
    var stride_1 : u32 = u32(128);
    loop
    {
        if((stride_1 > u32(0)))
        {
        }
        else
        {
            break;
        }
        if((t_1 < stride_1))
        {
            shared_sums_0[t_1] = shared_sums_0[t_1] + shared_sums_0[t_1 + stride_1];
            shared_counts_0[t_1] = shared_counts_0[t_1] + shared_counts_0[t_1 + stride_1];
        }
        workgroupBarrier();
        stride_1 = stride_1 / u32(2);
    }
    if(t_1 == u32(0))
    {
        chroma_0[i32(0)] = vec4<f32>(color_offset_0(u32(0)), color_offset_0(u32(1)), color_offset_0(u32(2)), 0.0f);
    }
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn opposed_inpaint(@builtin(global_invocation_id) id_4 : vec3<u32>)
{
    var _S37 : u32 = id_4.x;
    var _S38 : bool;
    if((_S37 >= (params_0.pass_0.width_0)))
    {
        _S38 = true;
    }
    else
    {
        _S38 = ((id_4.y) >= (params_0.pass_0.height_0));
    }
    if(_S38)
    {
        return;
    }
    var row_5 : u32 = params_0.pass_0.origin_y_0 + id_4.y;
    var value_1 : f32 = mosaic_at_0(_S37, row_5);
    var color_4 : u32 = color_at_0(_S37, row_5);
    var value_2 : f32;
    if((value_1 >= (clip_of_0(color_4))))
    {
        value_2 = max(value_1, opposing_average_0(i32(_S37), i32(row_5)) + chroma_0[i32(0)][color_4]);
    }
    else
    {
        value_2 = value_1;
    }
    output_0[row_5 * params_0.highlights_0.width_1 + _S37 - params_0.pass_0.output_base_0] = value_2;
    return;
}

