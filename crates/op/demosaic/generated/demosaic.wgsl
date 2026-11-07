// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/iop/demosaic.c,
// src/iop/demosaicing/ppg.c and rcd.c, and data/kernels/demosaic_rcd.cl)
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

// Generated from slang/demosaic.slang by tools/build-kernels.sh. Do not edit.

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

struct DemosaicParams_std140_0
{
    @align(16) width_1 : u32,
    @align(4) height_1 : u32,
    @align(8) stride_0 : u32,
    @align(4) step_0 : u32,
    @align(16) crop_x_0 : u32,
    @align(4) crop_y_0 : u32,
    @align(8) crop_width_0 : u32,
    @align(4) crop_height_0 : u32,
    @align(16) scale_1 : f32,
    @align(4) unscale_0 : f32,
    @align(8) solitary_x_0 : u32,
    @align(4) solitary_y_0 : u32,
    @align(16) colors_0 : array<vec4<u32>, i32(9)>,
    @align(16) hexes_0 : array<vec4<i32>, i32(36)>,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) demosaic_0 : DemosaicParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(0) @group(0) var<storage, read> input_0 : array<f32>;

@binding(15) @group(0) var<storage, read_write> rgb0_0 : array<f32>;

@binding(16) @group(0) var<storage, read_write> rgb1_0 : array<f32>;

@binding(17) @group(0) var<storage, read_write> rgb2_0 : array<f32>;

@binding(1) @group(0) var<storage, read_write> output_0 : array<vec4<f32>>;

@binding(10) @group(0) var<storage, read_write> cfa_0 : array<f32>;

@binding(11) @group(0) var<storage, read_write> vh_dir_0 : array<f32>;

@binding(12) @group(0) var<storage, read_write> pq_dir_0 : array<f32>;

@binding(13) @group(0) var<storage, read_write> p_diff_0 : array<f32>;

@binding(14) @group(0) var<storage, read_write> q_diff_0 : array<f32>;

@binding(18) @group(0) var<storage, read_write> xrgb0_0 : array<vec4<f32>>;

@binding(19) @group(0) var<storage, read_write> xrgb1_0 : array<vec4<f32>>;

@binding(20) @group(0) var<storage, read_write> xrgb2_0 : array<vec4<f32>>;

@binding(21) @group(0) var<storage, read_write> xrgb3_0 : array<vec4<f32>>;

@binding(23) @group(0) var<storage, read_write> work_0 : array<f32>;

@binding(22) @group(0) var<storage, read_write> aux_0 : array<vec4<f32>>;

fn photosite_0( id_0 : vec3<u32>,  col_0 : ptr<function, i32>,  row_0 : ptr<function, i32>) -> bool
{
    (*col_0) = i32(id_0.x);
    (*row_0) = i32(id_0.y);
    var _S1 : bool;
    if(((*col_0) < i32(params_0.demosaic_0.width_1)))
    {
        _S1 = ((*row_0) < i32(params_0.demosaic_0.height_1));
    }
    else
    {
        _S1 = false;
    }
    return _S1;
}

fn color_at_0( position_0 : u32) -> u32
{
    return params_0.demosaic_0.colors_0[position_0 / u32(4)][position_0 % u32(4)];
}

fn fc_0( row_1 : i32,  col_1 : i32) -> u32
{
    return color_at_0(u32((((((row_1 & (i32(1)))) << (u32(1)))) | (((col_1 & (i32(1))))))));
}

fn mosaic_0( col_2 : i32,  row_2 : i32) -> f32
{
    return input_0[row_2 * i32(params_0.demosaic_0.width_1) + col_2];
}

fn add_at_0( v_0 : vec3<f32>,  c_0 : u32,  x_0 : f32) -> vec3<f32>
{
    return select(v_0, v_0 + vec3<f32>(x_0), vec3<u32>(u32(0), u32(1), u32(2)) == vec3<u32>(c_0));
}

fn ppg_border_0( c_1 : u32,  f_0 : u32,  sum_0 : f32,  count_0 : f32,  own_0 : f32) -> f32
{
    var _S2 : bool;
    if(c_1 != f_0)
    {
        _S2 = (count_0 > 0.0f);
    }
    else
    {
        _S2 = false;
    }
    var _S3 : f32;
    if(_S2)
    {
        _S3 = max(0.0f, sum_0 / count_0);
    }
    else
    {
        _S3 = own_0;
    }
    return _S3;
}

fn set_at_0( v_1 : vec3<f32>,  c_2 : u32,  x_1 : f32) -> vec3<f32>
{
    return select(v_1, vec3<f32>(x_1), vec3<u32>(u32(0), u32(1), u32(2)) == vec3<u32>(c_2));
}

fn plane_0( col_3 : i32,  row_3 : i32) -> i32
{
    return row_3 * i32(params_0.demosaic_0.stride_0) + col_3;
}

fn store_rgb_0( idx_0 : i32,  rgb_0 : vec3<f32>)
{
    rgb0_0[idx_0] = rgb_0.x;
    rgb1_0[idx_0] = rgb_0.y;
    rgb2_0[idx_0] = rgb_0.z;
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn ppg_green(@builtin(global_invocation_id) id_1 : vec3<u32>)
{
    var col_4 : i32;
    var row_4 : i32;
    var _S4 : bool = photosite_0(id_1, &(col_4), &(row_4));
    if(!_S4)
    {
        return;
    }
    var width_2 : i32 = i32(params_0.demosaic_0.width_1);
    var height_2 : i32 = i32(params_0.demosaic_0.height_1);
    var _S5 : vec3<f32> = vec3<f32>(0.0f);
    var color_0 : vec3<f32> = _S5;
    var _S6 : bool;
    if((col_4 < i32(3)))
    {
        _S6 = true;
    }
    else
    {
        _S6 = (row_4 < i32(3));
    }
    if(_S6)
    {
        _S6 = true;
    }
    else
    {
        _S6 = (col_4 >= (width_2 - i32(3)));
    }
    if(_S6)
    {
        _S6 = true;
    }
    else
    {
        _S6 = (row_4 >= (height_2 - i32(3)));
    }
    if(_S6)
    {
        var _S7 : i32 = row_4 - i32(1);
        var sum_1 : vec3<f32> = _S5;
        var count_1 : vec3<f32> = _S5;
        var y_0 : i32 = _S7;
        loop
        {
            if(y_0 != (row_4 + i32(2)))
            {
            }
            else
            {
                break;
            }
            var _S8 : i32 = col_4 - i32(1);
            var sum_2 : vec3<f32> = sum_1;
            var count_2 : vec3<f32> = count_1;
            var x_2 : i32 = _S8;
            loop
            {
                if(x_2 != (col_4 + i32(2)))
                {
                }
                else
                {
                    break;
                }
                if((y_0 >= i32(0)))
                {
                    _S6 = (x_2 >= i32(0));
                }
                else
                {
                    _S6 = false;
                }
                var _S9 : bool;
                if(_S6)
                {
                    _S9 = (y_0 < height_2);
                }
                else
                {
                    _S9 = false;
                }
                var _S10 : bool;
                if(_S9)
                {
                    _S10 = (x_2 < width_2);
                }
                else
                {
                    _S10 = false;
                }
                if(_S10)
                {
                    var f_1 : u32 = fc_0(y_0, x_2);
                    var _S11 : vec3<f32> = add_at_0(count_2, f_1, 1.0f);
                    sum_2 = add_at_0(sum_2, f_1, mosaic_0(x_2, y_0));
                    count_2 = _S11;
                }
                x_2 = x_2 + i32(1);
            }
            var _S12 : i32 = y_0 + i32(1);
            sum_1 = sum_2;
            count_1 = count_2;
            y_0 = _S12;
        }
        var f_2 : u32 = fc_0(row_4, col_4);
        var _S13 : f32 = max(0.0f, mosaic_0(col_4, row_4));
        color_0 = vec3<f32>(ppg_border_0(u32(0), f_2, sum_1.x, count_1.x, _S13), ppg_border_0(u32(1), f_2, sum_1.y, count_1.y, _S13), ppg_border_0(u32(2), f_2, sum_1.z, count_1.z, _S13));
    }
    else
    {
        var c_3 : u32 = fc_0(row_4, col_4);
        var pc_0 : f32 = mosaic_0(col_4, row_4);
        if(c_3 == u32(0))
        {
            _S6 = true;
        }
        else
        {
            _S6 = c_3 == u32(2);
        }
        if(_S6)
        {
            color_0 = set_at_0(color_0, c_3, pc_0);
            var pym_0 : f32 = mosaic_0(col_4, row_4 - i32(1));
            var pym2_0 : f32 = mosaic_0(col_4, row_4 - i32(2));
            var pyM_0 : f32 = mosaic_0(col_4, row_4 + i32(1));
            var pyM2_0 : f32 = mosaic_0(col_4, row_4 + i32(2));
            var pxm_0 : f32 = mosaic_0(col_4 - i32(1), row_4);
            var pxm2_0 : f32 = mosaic_0(col_4 - i32(2), row_4);
            var pxM_0 : f32 = mosaic_0(col_4 + i32(1), row_4);
            var pxM2_0 : f32 = mosaic_0(col_4 + i32(2), row_4);
            var guessx_0 : f32 = (pxm_0 + pc_0 + pxM_0) * 2.0f - pxM2_0 - pxm2_0;
            var guessy_0 : f32 = (pym_0 + pc_0 + pyM_0) * 2.0f - pyM2_0 - pym2_0;
            if((((abs(pxm2_0 - pc_0) + abs(pxM2_0 - pc_0) + abs(pxm_0 - pxM_0)) * 3.0f + (abs(mosaic_0(col_4 + i32(3), row_4) - pxM_0) + abs(mosaic_0(col_4 - i32(3), row_4) - pxm_0)) * 2.0f) > ((abs(pym2_0 - pc_0) + abs(pyM2_0 - pc_0) + abs(pym_0 - pyM_0)) * 3.0f + (abs(mosaic_0(col_4, row_4 + i32(3)) - pyM_0) + abs(mosaic_0(col_4, row_4 - i32(3)) - pym_0)) * 2.0f)))
            {
                color_0[i32(1)] = max(min(guessy_0 * 0.25f, max(pym_0, pyM_0)), min(pym_0, pyM_0));
            }
            else
            {
                color_0[i32(1)] = max(min(guessx_0 * 0.25f, max(pxm_0, pxM_0)), min(pxm_0, pxM_0));
            }
        }
        else
        {
            color_0[i32(1)] = pc_0;
        }
        color_0 = max(color_0, _S5);
    }
    store_rgb_0(plane_0(col_4, row_4), color_0);
    return;
}

fn load_rgb_0( idx_1 : i32) -> vec3<f32>
{
    return vec3<f32>(rgb0_0[idx_1], rgb1_0[idx_1], rgb2_0[idx_1]);
}

fn write_output_0( col_5 : i32,  row_5 : i32,  rgb_1 : vec3<f32>)
{
    var x_3 : i32 = col_5 - i32(params_0.demosaic_0.crop_x_0);
    var y_1 : i32 = row_5 - i32(params_0.demosaic_0.crop_y_0);
    var _S14 : bool;
    if((x_3 < i32(0)))
    {
        _S14 = true;
    }
    else
    {
        _S14 = (y_1 < i32(0));
    }
    if(_S14)
    {
        _S14 = true;
    }
    else
    {
        _S14 = (x_3 >= i32(params_0.demosaic_0.crop_width_0));
    }
    if(_S14)
    {
        _S14 = true;
    }
    else
    {
        _S14 = (y_1 >= i32(params_0.demosaic_0.crop_height_0));
    }
    if(_S14)
    {
        return;
    }
    output_0[y_1 * i32(params_0.demosaic_0.crop_width_0) + x_3] = vec4<f32>(max(rgb_1, vec3<f32>(0.0f)), 0.0f);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn ppg_redblue(@builtin(global_invocation_id) id_2 : vec3<u32>)
{
    var col_6 : i32;
    var row_6 : i32;
    var _S15 : bool = photosite_0(id_2, &(col_6), &(row_6));
    if(!_S15)
    {
        return;
    }
    var width_3 : i32 = i32(params_0.demosaic_0.width_1);
    var height_3 : i32 = i32(params_0.demosaic_0.height_1);
    var idx_2 : i32 = plane_0(col_6, row_6);
    var color_1 : vec3<f32> = load_rgb_0(idx_2);
    var _S16 : bool;
    if((row_6 > i32(0)))
    {
        _S16 = (col_6 > i32(0));
    }
    else
    {
        _S16 = false;
    }
    if(_S16)
    {
        _S16 = (col_6 < (width_3 - i32(1)));
    }
    else
    {
        _S16 = false;
    }
    if(_S16)
    {
        _S16 = (row_6 < (height_3 - i32(1)));
    }
    else
    {
        _S16 = false;
    }
    if(_S16)
    {
        var c_4 : u32 = fc_0(row_6, col_6);
        var s_0 : i32 = i32(params_0.demosaic_0.stride_0);
        if(c_4 == u32(1))
        {
            var nt_0 : vec3<f32> = load_rgb_0(idx_2 - s_0);
            var nb_0 : vec3<f32> = load_rgb_0(idx_2 + s_0);
            var nl_0 : vec3<f32> = load_rgb_0(idx_2 - i32(1));
            var nr_0 : vec3<f32> = load_rgb_0(idx_2 + i32(1));
            if((fc_0(row_6, col_6 + i32(1))) == u32(0))
            {
                color_1[i32(2)] = (nt_0.z + nb_0.z + 2.0f * color_1.y - nt_0.y - nb_0.y) * 0.5f;
                color_1[i32(0)] = (nl_0.x + nr_0.x + 2.0f * color_1.y - nl_0.y - nr_0.y) * 0.5f;
            }
            else
            {
                color_1[i32(0)] = (nt_0.x + nb_0.x + 2.0f * color_1.y - nt_0.y - nb_0.y) * 0.5f;
                color_1[i32(2)] = (nl_0.z + nr_0.z + 2.0f * color_1.y - nl_0.y - nr_0.y) * 0.5f;
            }
        }
        else
        {
            var _S17 : i32 = idx_2 - i32(1);
            var ntl_0 : vec3<f32> = load_rgb_0(_S17 - s_0);
            var _S18 : i32 = idx_2 + i32(1);
            var ntr_0 : vec3<f32> = load_rgb_0(_S18 - s_0);
            var nbl_0 : vec3<f32> = load_rgb_0(_S17 + s_0);
            var nbr_0 : vec3<f32> = load_rgb_0(_S18 + s_0);
            var o_0 : u32 = u32(2) - c_4;
            var _S19 : f32 = ntl_0.y;
            var _S20 : f32 = nbr_0.y;
            var diff1_0 : f32 = abs(ntl_0[o_0] - nbr_0[o_0]) + abs(_S19 - color_1.y) + abs(_S20 - color_1.y);
            var guess1_0 : f32 = ntl_0[o_0] + nbr_0[o_0] + 2.0f * color_1.y - _S19 - _S20;
            var _S21 : f32 = ntr_0.y;
            var _S22 : f32 = nbl_0.y;
            var diff2_0 : f32 = abs(ntr_0[o_0] - nbl_0[o_0]) + abs(_S21 - color_1.y) + abs(_S22 - color_1.y);
            var guess2_0 : f32 = ntr_0[o_0] + nbl_0[o_0] + 2.0f * color_1.y - _S21 - _S22;
            var fill_0 : f32;
            if((diff1_0 > diff2_0))
            {
                fill_0 = guess2_0 * 0.5f;
            }
            else
            {
                if((diff1_0 < diff2_0))
                {
                    fill_0 = guess1_0 * 0.5f;
                }
                else
                {
                    fill_0 = (guess1_0 + guess2_0) * 0.25f;
                }
            }
            color_1 = set_at_0(color_1, o_0, fill_0);
        }
    }
    write_output_0(col_6, row_6, color_1);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn rcd_populate(@builtin(global_invocation_id) id_3 : vec3<u32>)
{
    var col_7 : i32;
    var row_7 : i32;
    var _S23 : bool = photosite_0(id_3, &(col_7), &(row_7));
    if(!_S23)
    {
        return;
    }
    var value_0 : f32 = max(0.0f, mosaic_0(col_7, row_7)) * params_0.demosaic_0.scale_1;
    var idx_3 : i32 = plane_0(col_7, row_7);
    cfa_0[idx_3] = value_0;
    var c0_0 : u32 = fc_0(row_7, i32(0));
    var c1_0 : u32 = fc_0(row_7, i32(1));
    var _S24 : bool;
    if(c0_0 == u32(0))
    {
        _S24 = true;
    }
    else
    {
        _S24 = c1_0 == u32(0);
    }
    var _S25 : f32;
    if(_S24)
    {
        _S25 = value_0;
    }
    else
    {
        _S25 = 0.0f;
    }
    rgb0_0[idx_3] = _S25;
    if(c0_0 == u32(1))
    {
        _S24 = true;
    }
    else
    {
        _S24 = c1_0 == u32(1);
    }
    if(_S24)
    {
        _S25 = value_0;
    }
    else
    {
        _S25 = 0.0f;
    }
    rgb1_0[idx_3] = _S25;
    if(c0_0 == u32(2))
    {
        _S24 = true;
    }
    else
    {
        _S24 = c1_0 == u32(2);
    }
    if(_S24)
    {
        _S25 = value_0;
    }
    else
    {
        _S25 = 0.0f;
    }
    rgb2_0[idx_3] = _S25;
    return;
}

fn sqr_0( x_4 : f32) -> f32
{
    return x_4 * x_4;
}

fn hpf_0( idx_4 : i32,  step_1 : i32) -> f32
{
    var _S26 : i32 = i32(3) * step_1;
    var _S27 : i32 = i32(2) * step_1;
    return sqr_0(cfa_0[idx_4 - _S26] - cfa_0[idx_4 - step_1] - cfa_0[idx_4 + step_1] + cfa_0[idx_4 + _S26] - 3.0f * (cfa_0[idx_4 - _S27] + cfa_0[idx_4 + _S27]) + 6.0f * cfa_0[idx_4]);
}

@compute
@workgroup_size(16, 16, 1)
fn rcd_step_1(@builtin(global_invocation_id) id_4 : vec3<u32>)
{
    var col_8 : i32;
    var row_8 : i32;
    var _S28 : bool = photosite_0(id_4, &(col_8), &(row_8));
    if(!_S28)
    {
        return;
    }
    var idx_5 : i32 = plane_0(col_8, row_8);
    var s_1 : i32 = i32(params_0.demosaic_0.stride_0);
    var _S29 : bool;
    if((row_8 < i32(4)))
    {
        _S29 = true;
    }
    else
    {
        _S29 = (col_8 < i32(4));
    }
    if(_S29)
    {
        _S29 = true;
    }
    else
    {
        _S29 = (row_8 >= (i32(params_0.demosaic_0.height_1) - i32(4)));
    }
    if(_S29)
    {
        _S29 = true;
    }
    else
    {
        _S29 = (col_8 >= (i32(params_0.demosaic_0.width_1) - i32(4)));
    }
    if(_S29)
    {
        vh_dir_0[idx_5] = 0.0f;
        return;
    }
    var _S30 : f32 = max(1.00000001335143196e-10f, hpf_0(idx_5 - s_1, s_1) + hpf_0(idx_5, s_1) + hpf_0(idx_5 + s_1, s_1));
    vh_dir_0[idx_5] = _S30 / (_S30 + max(1.00000001335143196e-10f, hpf_0(idx_5 - i32(1), i32(1)) + hpf_0(idx_5, i32(1)) + hpf_0(idx_5 + i32(1), i32(1))));
    return;
}

fn red_or_blue_0( col_9 : i32,  row_9 : i32) -> bool
{
    return (((col_9 - i32(((fc_0(row_9, i32(0))) & (u32(1))))) & (i32(1)))) == i32(0);
}

@compute
@workgroup_size(16, 16, 1)
fn rcd_step_2(@builtin(global_invocation_id) id_5 : vec3<u32>)
{
    var col_10 : i32;
    var row_10 : i32;
    var _S31 : bool = photosite_0(id_5, &(col_10), &(row_10));
    if(!_S31)
    {
        return;
    }
    var _S32 : bool;
    if((row_10 < i32(2)))
    {
        _S32 = true;
    }
    else
    {
        _S32 = (col_10 < i32(2));
    }
    if(_S32)
    {
        _S32 = true;
    }
    else
    {
        _S32 = (row_10 >= (i32(params_0.demosaic_0.height_1) - i32(2)));
    }
    if(_S32)
    {
        _S32 = true;
    }
    else
    {
        _S32 = (col_10 >= (i32(params_0.demosaic_0.width_1) - i32(2)));
    }
    if(_S32)
    {
        _S32 = true;
    }
    else
    {
        _S32 = !red_or_blue_0(col_10, row_10);
    }
    if(_S32)
    {
        return;
    }
    var idx_6 : i32 = plane_0(col_10, row_10);
    var s_2 : i32 = i32(params_0.demosaic_0.stride_0);
    var _S33 : i32 = idx_6 - s_2;
    var _S34 : i32 = idx_6 + s_2;
    pq_dir_0[idx_6 / i32(2)] = cfa_0[idx_6] + 0.5f * (cfa_0[_S33] + cfa_0[_S34] + cfa_0[idx_6 - i32(1)] + cfa_0[idx_6 + i32(1)]) + 0.25f * (cfa_0[_S33 - i32(1)] + cfa_0[_S33 + i32(1)] + cfa_0[_S34 - i32(1)] + cfa_0[_S34 + i32(1)]);
    return;
}

fn inner_red_or_blue_0( col_11 : i32,  row_11 : i32) -> bool
{
    var _S35 : bool;
    if((row_11 >= i32(4)))
    {
        _S35 = (col_11 >= i32(4));
    }
    else
    {
        _S35 = false;
    }
    if(_S35)
    {
        _S35 = (row_11 < (i32(params_0.demosaic_0.height_1) - i32(4)));
    }
    else
    {
        _S35 = false;
    }
    if(_S35)
    {
        _S35 = (col_11 < (i32(params_0.demosaic_0.width_1) - i32(4)));
    }
    else
    {
        _S35 = false;
    }
    if(_S35)
    {
        _S35 = red_or_blue_0(col_11, row_11);
    }
    else
    {
        _S35 = false;
    }
    return _S35;
}

fn discrimination_0( central_0 : f32,  neighbourhood_0 : f32) -> f32
{
    var _S36 : f32;
    if(((abs(0.5f - central_0)) < (abs(0.5f - neighbourhood_0))))
    {
        _S36 = neighbourhood_0;
    }
    else
    {
        _S36 = central_0;
    }
    return _S36;
}

fn interpolate_0( a_0 : f32,  b_0 : f32,  c_5 : f32) -> f32
{
    return a_0 * (b_0 - c_5) + c_5;
}

@compute
@workgroup_size(16, 16, 1)
fn rcd_step_3(@builtin(global_invocation_id) id_6 : vec3<u32>)
{
    var col_12 : i32;
    var row_12 : i32;
    var _S37 : bool = photosite_0(id_6, &(col_12), &(row_12));
    var _S38 : bool;
    if(!_S37)
    {
        _S38 = true;
    }
    else
    {
        _S38 = !inner_red_or_blue_0(col_12, row_12);
    }
    if(_S38)
    {
        return;
    }
    var idx_7 : i32 = plane_0(col_12, row_12);
    var s_3 : i32 = i32(params_0.demosaic_0.stride_0);
    var lp_0 : i32 = idx_7 / i32(2);
    var _S39 : i32 = idx_7 - s_3;
    var _S40 : i32 = idx_7 + s_3;
    var _S41 : i32 = i32(2) * s_3;
    var _S42 : i32 = idx_7 - _S41;
    var _S43 : i32 = i32(3) * s_3;
    var _S44 : i32 = i32(4) * s_3;
    var n_grad_0 : f32 = 9.9999997473787516e-06f + abs(cfa_0[_S39] - cfa_0[_S40]) + abs(cfa_0[idx_7] - cfa_0[_S42]) + abs(cfa_0[_S39] - cfa_0[idx_7 - _S43]) + abs(cfa_0[_S42] - cfa_0[idx_7 - _S44]);
    var _S45 : i32 = idx_7 + _S41;
    var s_grad_0 : f32 = 9.9999997473787516e-06f + abs(cfa_0[_S40] - cfa_0[_S39]) + abs(cfa_0[idx_7] - cfa_0[_S45]) + abs(cfa_0[_S40] - cfa_0[idx_7 + _S43]) + abs(cfa_0[_S45] - cfa_0[idx_7 + _S44]);
    var _S46 : i32 = idx_7 - i32(1);
    var _S47 : i32 = idx_7 + i32(1);
    var _S48 : i32 = idx_7 - i32(2);
    var w_grad_0 : f32 = 9.9999997473787516e-06f + abs(cfa_0[_S46] - cfa_0[_S47]) + abs(cfa_0[idx_7] - cfa_0[_S48]) + abs(cfa_0[_S46] - cfa_0[idx_7 - i32(3)]) + abs(cfa_0[_S48] - cfa_0[idx_7 - i32(4)]);
    var _S49 : i32 = idx_7 + i32(2);
    var e_grad_0 : f32 = 9.9999997473787516e-06f + abs(cfa_0[_S47] - cfa_0[_S46]) + abs(cfa_0[idx_7] - cfa_0[_S49]) + abs(cfa_0[_S47] - cfa_0[idx_7 + i32(3)]) + abs(cfa_0[_S49] - cfa_0[idx_7 + i32(4)]);
    var _S50 : f32 = pq_dir_0[lp_0] + pq_dir_0[lp_0];
    var _S51 : f32 = 9.9999997473787516e-06f + pq_dir_0[lp_0];
    rgb1_0[idx_7] = interpolate_0(clamp(discrimination_0(vh_dir_0[idx_7], 0.25f * (vh_dir_0[_S39 - i32(1)] + vh_dir_0[_S39 + i32(1)] + vh_dir_0[_S40 - i32(1)] + vh_dir_0[_S40 + i32(1)])), 0.0f, 1.0f), (w_grad_0 * (cfa_0[_S47] * _S50 / (_S51 + pq_dir_0[lp_0 + i32(1)])) + e_grad_0 * (cfa_0[_S46] * _S50 / (_S51 + pq_dir_0[lp_0 - i32(1)]))) / (e_grad_0 + w_grad_0), (s_grad_0 * (cfa_0[_S39] * _S50 / (_S51 + pq_dir_0[lp_0 - s_3])) + n_grad_0 * (cfa_0[_S40] * _S50 / (_S51 + pq_dir_0[lp_0 + s_3]))) / (n_grad_0 + s_grad_0));
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn rcd_step_4_0(@builtin(global_invocation_id) id_7 : vec3<u32>)
{
    var col_13 : i32;
    var row_13 : i32;
    var _S52 : bool = photosite_0(id_7, &(col_13), &(row_13));
    if(!_S52)
    {
        return;
    }
    var _S53 : bool;
    if((row_13 < i32(3)))
    {
        _S53 = true;
    }
    else
    {
        _S53 = (col_13 < i32(3));
    }
    if(_S53)
    {
        _S53 = true;
    }
    else
    {
        _S53 = (row_13 >= (i32(params_0.demosaic_0.height_1) - i32(3)));
    }
    if(_S53)
    {
        _S53 = true;
    }
    else
    {
        _S53 = (col_13 >= (i32(params_0.demosaic_0.width_1) - i32(3)));
    }
    if(_S53)
    {
        _S53 = true;
    }
    else
    {
        _S53 = ((col_13 & (i32(1)))) == i32(0);
    }
    if(_S53)
    {
        return;
    }
    var idx_8 : i32 = plane_0(col_13, row_13);
    var s_4 : i32 = i32(params_0.demosaic_0.stride_0);
    p_diff_0[idx_8 / i32(2)] = hpf_0(idx_8, s_4 + i32(1));
    q_diff_0[idx_8 / i32(2)] = hpf_0(idx_8, s_4 - i32(1));
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn rcd_step_4_1(@builtin(global_invocation_id) id_8 : vec3<u32>)
{
    var col_14 : i32;
    var row_14 : i32;
    var _S54 : bool = photosite_0(id_8, &(col_14), &(row_14));
    var _S55 : bool;
    if(!_S54)
    {
        _S55 = true;
    }
    else
    {
        _S55 = !inner_red_or_blue_0(col_14, row_14);
    }
    if(_S55)
    {
        return;
    }
    var idx_9 : i32 = plane_0(col_14, row_14);
    var s_5 : i32 = i32(params_0.demosaic_0.stride_0);
    var i2_0 : i32 = idx_9 / i32(2);
    var i3_0 : i32 = (idx_9 - s_5 - i32(1)) / i32(2);
    var i4_0 : i32 = (idx_9 + s_5 - i32(1)) / i32(2);
    var _S56 : f32 = max(1.00000001335143196e-10f, p_diff_0[i3_0] + p_diff_0[i2_0] + p_diff_0[i4_0 + i32(1)]);
    pq_dir_0[i2_0] = _S56 / (_S56 + max(1.00000001335143196e-10f, q_diff_0[i3_0 + i32(1)] + q_diff_0[i2_0] + q_diff_0[i4_0]));
    return;
}

fn rgb_of_0( c_6 : u32,  idx_10 : i32) -> f32
{
    var _S57 : f32;
    if(c_6 == u32(0))
    {
        _S57 = rgb0_0[idx_10];
    }
    else
    {
        if(c_6 == u32(1))
        {
            _S57 = rgb1_0[idx_10];
        }
        else
        {
            _S57 = rgb2_0[idx_10];
        }
    }
    return _S57;
}

@compute
@workgroup_size(16, 16, 1)
fn rcd_step_4_2(@builtin(global_invocation_id) id_9 : vec3<u32>)
{
    var col_15 : i32;
    var row_15 : i32;
    var _S58 : bool = photosite_0(id_9, &(col_15), &(row_15));
    var _S59 : bool;
    if(!_S58)
    {
        _S59 = true;
    }
    else
    {
        _S59 = !inner_red_or_blue_0(col_15, row_15);
    }
    if(_S59)
    {
        return;
    }
    var idx_11 : i32 = plane_0(col_15, row_15);
    var s_6 : i32 = i32(params_0.demosaic_0.stride_0);
    var c_7 : u32 = u32(2) - fc_0(row_15, col_15);
    var _S60 : i32 = idx_11 - s_6;
    var _S61 : i32 = _S60 - i32(1);
    var pq2_0 : i32 = _S61 / i32(2);
    var _S62 : i32 = idx_11 + s_6;
    var _S63 : i32 = _S62 - i32(1);
    var pq3_0 : i32 = _S63 / i32(2);
    var _S64 : i32 = _S62 + i32(1);
    var _S65 : i32 = i32(3) * s_6;
    var _S66 : i32 = idx_11 - _S65;
    var _S67 : i32 = i32(2) * s_6;
    var _S68 : i32 = idx_11 - _S67;
    var nw_grad_0 : f32 = 9.9999997473787516e-06f + abs(rgb_of_0(c_7, _S61) - rgb_of_0(c_7, _S64)) + abs(rgb_of_0(c_7, _S61) - rgb_of_0(c_7, _S66 - i32(3))) + abs(rgb1_0[idx_11] - rgb1_0[_S68 - i32(2)]);
    var _S69 : i32 = _S60 + i32(1);
    var ne_grad_0 : f32 = 9.9999997473787516e-06f + abs(rgb_of_0(c_7, _S69) - rgb_of_0(c_7, _S63)) + abs(rgb_of_0(c_7, _S69) - rgb_of_0(c_7, _S66 + i32(3))) + abs(rgb1_0[idx_11] - rgb1_0[_S68 + i32(2)]);
    var _S70 : i32 = idx_11 + _S65;
    var _S71 : i32 = idx_11 + _S67;
    var sw_grad_0 : f32 = 9.9999997473787516e-06f + abs(rgb_of_0(c_7, _S69) - rgb_of_0(c_7, _S63)) + abs(rgb_of_0(c_7, _S63) - rgb_of_0(c_7, _S70 - i32(3))) + abs(rgb1_0[idx_11] - rgb1_0[_S71 - i32(2)]);
    var se_grad_0 : f32 = 9.9999997473787516e-06f + abs(rgb_of_0(c_7, _S61) - rgb_of_0(c_7, _S64)) + abs(rgb_of_0(c_7, _S64) - rgb_of_0(c_7, _S70 + i32(3))) + abs(rgb1_0[idx_11] - rgb1_0[_S71 + i32(2)]);
    var value_1 : f32 = rgb1_0[idx_11] + interpolate_0(clamp(discrimination_0(pq_dir_0[idx_11 / i32(2)], 0.25f * (pq_dir_0[pq2_0] + pq_dir_0[pq2_0 + i32(1)] + pq_dir_0[pq3_0] + pq_dir_0[pq3_0 + i32(1)])), 0.0f, 1.0f), (ne_grad_0 * (rgb_of_0(c_7, _S63) - rgb1_0[_S63]) + sw_grad_0 * (rgb_of_0(c_7, _S69) - rgb1_0[_S69])) / (ne_grad_0 + sw_grad_0), (nw_grad_0 * (rgb_of_0(c_7, _S64) - rgb1_0[_S64]) + se_grad_0 * (rgb_of_0(c_7, _S61) - rgb1_0[_S61])) / (nw_grad_0 + se_grad_0));
    if(c_7 == u32(0))
    {
        rgb0_0[idx_11] = value_1;
    }
    else
    {
        rgb2_0[idx_11] = value_1;
    }
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn rcd_step_4_3(@builtin(global_invocation_id) id_10 : vec3<u32>)
{
    var col_16 : i32;
    var row_16 : i32;
    var _S72 : bool = photosite_0(id_10, &(col_16), &(row_16));
    if(!_S72)
    {
        return;
    }
    var _S73 : bool;
    if((row_16 < i32(4)))
    {
        _S73 = true;
    }
    else
    {
        _S73 = (col_16 < i32(4));
    }
    if(_S73)
    {
        _S73 = true;
    }
    else
    {
        _S73 = (row_16 >= (i32(params_0.demosaic_0.height_1) - i32(4)));
    }
    if(_S73)
    {
        _S73 = true;
    }
    else
    {
        _S73 = (col_16 >= (i32(params_0.demosaic_0.width_1) - i32(4)));
    }
    if(_S73)
    {
        return;
    }
    if((((col_16 - i32(((fc_0(row_16, i32(1))) & (u32(1))))) & (i32(1)))) != i32(0))
    {
        return;
    }
    var idx_12 : i32 = plane_0(col_16, row_16);
    var s_7 : i32 = i32(params_0.demosaic_0.stride_0);
    var _S74 : i32 = idx_12 - s_7;
    var _S75 : i32 = idx_12 + s_7;
    var _S76 : f32 = discrimination_0(vh_dir_0[idx_12], 0.25f * (vh_dir_0[_S74 - i32(1)] + vh_dir_0[_S74 + i32(1)] + vh_dir_0[_S75 - i32(1)] + vh_dir_0[_S75 + i32(1)]));
    var g_0 : f32 = rgb1_0[idx_12];
    var _S77 : i32 = i32(2) * s_7;
    var _S78 : f32 = 9.9999997473787516e-06f + abs(rgb1_0[idx_12] - rgb1_0[idx_12 - _S77]);
    var _S79 : f32 = 9.9999997473787516e-06f + abs(rgb1_0[idx_12] - rgb1_0[idx_12 + _S77]);
    var _S80 : f32 = 9.9999997473787516e-06f + abs(rgb1_0[idx_12] - rgb1_0[idx_12 - i32(2)]);
    var _S81 : f32 = 9.9999997473787516e-06f + abs(rgb1_0[idx_12] - rgb1_0[idx_12 + i32(2)]);
    var _S82 : f32 = rgb1_0[_S74];
    var _S83 : f32 = rgb1_0[_S75];
    var _S84 : i32 = idx_12 - i32(1);
    var _S85 : f32 = rgb1_0[_S84];
    var _S86 : i32 = idx_12 + i32(1);
    var _S87 : f32 = rgb1_0[_S86];
    var c_8 : u32 = u32(0);
    loop
    {
        if((c_8 <= u32(2)))
        {
        }
        else
        {
            break;
        }
        var sn_abs_0 : f32 = abs(rgb_of_0(c_8, _S74) - rgb_of_0(c_8, _S75));
        var ew_abs_0 : f32 = abs(rgb_of_0(c_8, _S84) - rgb_of_0(c_8, _S86));
        var _S88 : i32 = i32(3) * s_7;
        var n_grad_1 : f32 = _S78 + sn_abs_0 + abs(rgb_of_0(c_8, _S74) - rgb_of_0(c_8, idx_12 - _S88));
        var s_grad_1 : f32 = _S79 + sn_abs_0 + abs(rgb_of_0(c_8, _S75) - rgb_of_0(c_8, idx_12 + _S88));
        var w_grad_1 : f32 = _S80 + ew_abs_0 + abs(rgb_of_0(c_8, _S84) - rgb_of_0(c_8, idx_12 - i32(3)));
        var e_grad_1 : f32 = _S81 + ew_abs_0 + abs(rgb_of_0(c_8, _S86) - rgb_of_0(c_8, idx_12 + i32(3)));
        var value_2 : f32 = g_0 + interpolate_0(clamp(_S76, 0.0f, 1.0f), (e_grad_1 * (rgb_of_0(c_8, _S84) - _S85) + w_grad_1 * (rgb_of_0(c_8, _S86) - _S87)) / (e_grad_1 + w_grad_1), (n_grad_1 * (rgb_of_0(c_8, _S75) - _S83) + s_grad_1 * (rgb_of_0(c_8, _S74) - _S82)) / (n_grad_1 + s_grad_1));
        if(c_8 == u32(0))
        {
            rgb0_0[idx_12] = value_2;
        }
        else
        {
            rgb2_0[idx_12] = value_2;
        }
        c_8 = c_8 + u32(2);
    }
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn rcd_write(@builtin(global_invocation_id) id_11 : vec3<u32>)
{
    var col_17 : i32;
    var row_17 : i32;
    var _S89 : bool = photosite_0(id_11, &(col_17), &(row_17));
    if(!_S89)
    {
        return;
    }
    var _S90 : bool;
    if((row_17 < i32(9)))
    {
        _S90 = true;
    }
    else
    {
        _S90 = (col_17 < i32(9));
    }
    if(_S90)
    {
        _S90 = true;
    }
    else
    {
        _S90 = (row_17 >= (i32(params_0.demosaic_0.height_1) - i32(9)));
    }
    if(_S90)
    {
        _S90 = true;
    }
    else
    {
        _S90 = (col_17 >= (i32(params_0.demosaic_0.width_1) - i32(9)));
    }
    if(_S90)
    {
        return;
    }
    write_output_0(col_17, row_17, vec3<f32>(params_0.demosaic_0.unscale_0) * max(load_rgb_0(plane_0(col_17, row_17)), vec3<f32>(0.0f)));
    return;
}

fn within_0( col_18 : i32,  row_18 : i32,  border_0 : i32) -> bool
{
    var _S91 : bool;
    if((col_18 >= border_0))
    {
        _S91 = (row_18 >= border_0);
    }
    else
    {
        _S91 = false;
    }
    if(_S91)
    {
        _S91 = (col_18 < (i32(params_0.demosaic_0.width_1) - border_0));
    }
    else
    {
        _S91 = false;
    }
    if(_S91)
    {
        _S91 = (row_18 < (i32(params_0.demosaic_0.height_1) - border_0));
    }
    else
    {
        _S91 = false;
    }
    return _S91;
}

fn fcx_0( row_19 : i32,  col_19 : i32) -> u32
{
    return color_at_0(u32(row_19 % i32(6) * i32(6) + col_19 % i32(6)));
}

@compute
@workgroup_size(16, 16, 1)
fn xt_border(@builtin(global_invocation_id) id_12 : vec3<u32>)
{
    var col_20 : i32;
    var row_20 : i32;
    var _S92 : bool = photosite_0(id_12, &(col_20), &(row_20));
    var _S93 : bool;
    if(!_S92)
    {
        _S93 = true;
    }
    else
    {
        _S93 = within_0(col_20, row_20, i32(12));
    }
    if(_S93)
    {
        return;
    }
    var width_4 : i32 = i32(params_0.demosaic_0.width_1);
    var height_4 : i32 = i32(params_0.demosaic_0.height_1);
    var f_3 : u32 = fcx_0(row_20, col_20);
    var _S94 : vec3<f32> = vec3<f32>(0.0f);
    if(col_20 == i32(0))
    {
        _S93 = true;
    }
    else
    {
        _S93 = row_20 == i32(0);
    }
    if(_S93)
    {
        _S93 = true;
    }
    else
    {
        _S93 = col_20 == (width_4 - i32(1));
    }
    if(_S93)
    {
        _S93 = true;
    }
    else
    {
        _S93 = row_20 == (height_4 - i32(1));
    }
    var color_2 : vec3<f32>;
    var count_3 : vec3<f32>;
    var sum_3 : vec3<f32>;
    var count_4 : vec3<f32>;
    var y_2 : i32;
    var x_5 : i32;
    var _S95 : f32;
    var _S96 : f32;
    var _S97 : f32;
    if(_S93)
    {
        var _S98 : i32 = row_20 - i32(1);
        sum_3 = _S94;
        count_4 = _S94;
        y_2 = _S98;
        loop
        {
            if(y_2 != (row_20 + i32(2)))
            {
            }
            else
            {
                break;
            }
            var _S99 : i32 = col_20 - i32(1);
            color_2 = sum_3;
            count_3 = count_4;
            x_5 = _S99;
            loop
            {
                if(x_5 != (col_20 + i32(2)))
                {
                }
                else
                {
                    break;
                }
                if((y_2 >= i32(0)))
                {
                    _S93 = (x_5 >= i32(0));
                }
                else
                {
                    _S93 = false;
                }
                var _S100 : bool;
                if(_S93)
                {
                    _S100 = (y_2 < height_4);
                }
                else
                {
                    _S100 = false;
                }
                var _S101 : bool;
                if(_S100)
                {
                    _S101 = (x_5 < width_4);
                }
                else
                {
                    _S101 = false;
                }
                if(_S101)
                {
                    var c_9 : u32 = fcx_0(y_2, x_5);
                    var _S102 : vec3<f32> = add_at_0(count_3, c_9, 1.0f);
                    color_2 = add_at_0(color_2, c_9, max(0.0f, mosaic_0(x_5, y_2)));
                    count_3 = _S102;
                }
                x_5 = x_5 + i32(1);
            }
            var _S103 : i32 = y_2 + i32(1);
            sum_3 = color_2;
            count_4 = count_3;
            y_2 = _S103;
        }
        var _S104 : f32 = max(0.0f, mosaic_0(col_20, row_20));
        if(f_3 != u32(0))
        {
            _S93 = (count_4.x) != 0.0f;
        }
        else
        {
            _S93 = false;
        }
        if(_S93)
        {
            _S95 = sum_3.x / count_4.x;
        }
        else
        {
            _S95 = _S104;
        }
        if(f_3 != u32(1))
        {
            _S93 = (count_4.y) != 0.0f;
        }
        else
        {
            _S93 = false;
        }
        if(_S93)
        {
            _S96 = sum_3.y / count_4.y;
        }
        else
        {
            _S96 = _S104;
        }
        if(f_3 != u32(2))
        {
            _S93 = (count_4.z) != 0.0f;
        }
        else
        {
            _S93 = false;
        }
        if(_S93)
        {
            _S97 = sum_3.z / count_4.z;
        }
        else
        {
            _S97 = _S104;
        }
        color_2 = vec3<f32>(_S95, _S96, _S97);
    }
    else
    {
        color_2 = _S94;
        count_3 = _S94;
        y_2 = i32(-1);
        loop
        {
            if((y_2 <= i32(1)))
            {
            }
            else
            {
                break;
            }
            sum_3 = color_2;
            count_4 = count_3;
            x_5 = i32(-1);
            loop
            {
                if((x_5 <= i32(1)))
                {
                }
                else
                {
                    break;
                }
                var c_10 : u32 = fcx_0(row_20 + y_2, col_20 + x_5);
                if(c_10 == f_3)
                {
                    x_5 = x_5 + i32(1);
                    continue;
                }
                var _S105 : i32;
                if(y_2 == i32(0))
                {
                    _S105 = i32(1);
                }
                else
                {
                    _S105 = i32(0);
                }
                var _S106 : i32;
                if(x_5 == i32(0))
                {
                    _S106 = i32(1);
                }
                else
                {
                    _S106 = i32(0);
                }
                var weight_0 : f32 = f32((i32(1) << (u32(_S105 + _S106))));
                var _S107 : vec3<f32> = add_at_0(count_4, c_10, weight_0);
                sum_3 = add_at_0(sum_3, c_10, max(0.0f, mosaic_0(col_20 + x_5, row_20 + y_2)) * weight_0);
                count_4 = _S107;
                x_5 = x_5 + i32(1);
            }
            var _S108 : i32 = y_2 + i32(1);
            color_2 = sum_3;
            count_3 = count_4;
            y_2 = _S108;
        }
        var own_1 : f32 = mosaic_0(col_20, row_20);
        if(f_3 == u32(0))
        {
            _S95 = own_1;
        }
        else
        {
            _S95 = color_2.x / count_3.x;
        }
        if(f_3 == u32(1))
        {
            _S96 = own_1;
        }
        else
        {
            _S96 = color_2.y / count_3.y;
        }
        if(f_3 == u32(2))
        {
            _S97 = own_1;
        }
        else
        {
            _S97 = color_2.z / count_3.z;
        }
        color_2 = vec3<f32>(_S95, _S96, _S97);
    }
    write_output_0(col_20, row_20, color_2);
    return;
}

fn index_0( col_21 : i32,  row_21 : i32) -> i32
{
    return row_21 * i32(params_0.demosaic_0.width_1) + col_21;
}

@compute
@workgroup_size(16, 16, 1)
fn xt_initial(@builtin(global_invocation_id) id_13 : vec3<u32>)
{
    var col_22 : i32;
    var row_22 : i32;
    var _S109 : bool = photosite_0(id_13, &(col_22), &(row_22));
    if(!_S109)
    {
        return;
    }
    var f_4 : u32 = fcx_0(row_22, col_22);
    var _S110 : f32 = max(0.0f, mosaic_0(col_22, row_22));
    var _S111 : f32;
    if(f_4 == u32(0))
    {
        _S111 = _S110;
    }
    else
    {
        _S111 = 0.0f;
    }
    var _S112 : f32;
    if(f_4 == u32(1))
    {
        _S112 = _S110;
    }
    else
    {
        _S112 = 0.0f;
    }
    var _S113 : f32;
    if(f_4 == u32(2))
    {
        _S113 = _S110;
    }
    else
    {
        _S113 = 0.0f;
    }
    var pixel_0 : vec4<f32> = vec4<f32>(_S111, _S112, _S113, 0.0f);
    var idx_13 : i32 = index_0(col_22, row_22);
    xrgb0_0[idx_13] = pixel_0;
    xrgb1_0[idx_13] = pixel_0;
    xrgb2_0[idx_13] = pixel_0;
    xrgb3_0[idx_13] = pixel_0;
    return;
}

fn hexagon_0( col_23 : i32,  row_23 : i32,  c_11 : u32) -> vec2<i32>
{
    var e_0 : u32 = u32((row_23 % i32(3) * i32(3) + col_23 % i32(3)) * i32(8)) + c_11;
    var v_2 : vec4<i32> = params_0.demosaic_0.hexes_0[e_0 / u32(2)];
    var _S114 : vec2<i32>;
    if(((e_0 & (u32(1)))) == u32(0))
    {
        _S114 = v_2.xy;
    }
    else
    {
        _S114 = v_2.zw;
    }
    return _S114;
}

fn index_1( p_0 : vec2<i32>) -> i32
{
    return index_0(p_0.x, p_0.y);
}

fn green_bounds_0( col_24 : i32,  row_24 : i32) -> vec2<f32>
{
    var lo_0 : f32 = 3.4028234663852886e+38f;
    var hi_0 : f32 = 0.0f;
    var c_12 : u32 = u32(0);
    loop
    {
        if((c_12 < u32(6)))
        {
        }
        else
        {
            break;
        }
        var green_0 : f32 = xrgb0_0[index_1(vec2<i32>(col_24, row_24) + hexagon_0(col_24, row_24, c_12))].y;
        var _S115 : f32 = min(lo_0, green_0);
        var _S116 : f32 = max(hi_0, green_0);
        var _S117 : u32 = c_12 + u32(1);
        lo_0 = _S115;
        hi_0 = _S116;
        c_12 = _S117;
    }
    return vec2<f32>(lo_0, hi_0);
}

@compute
@workgroup_size(16, 16, 1)
fn xt_green_bounds(@builtin(global_invocation_id) id_14 : vec3<u32>)
{
    var col_25 : i32;
    var row_25 : i32;
    var _S118 : bool = photosite_0(id_14, &(col_25), &(row_25));
    var _S119 : bool;
    if(!_S118)
    {
        _S119 = true;
    }
    else
    {
        _S119 = !within_0(col_25, row_25, i32(3));
    }
    if(_S119)
    {
        _S119 = true;
    }
    else
    {
        _S119 = (fcx_0(row_25, col_25)) == u32(1);
    }
    if(_S119)
    {
        return;
    }
    var _S120 : vec2<f32> = green_bounds_0(col_25, row_25);
    var partner_0 : vec2<i32>;
    if((fcx_0(row_25, col_25 - i32(1))) != u32(1))
    {
        partner_0 = vec2<i32>(col_25 - i32(1), row_25);
    }
    else
    {
        if((fcx_0(row_25 - i32(1), col_25)) != u32(1))
        {
            partner_0 = vec2<i32>(col_25, row_25 - i32(1));
        }
        else
        {
            partner_0 = vec2<i32>(i32(-1), i32(-1));
        }
    }
    var _S121 : i32 = partner_0.x;
    var bounds_0 : vec2<f32>;
    if((_S121 >= i32(0)))
    {
        var first_0 : vec2<f32> = green_bounds_0(_S121, partner_0.y);
        if((first_0.y) != 0.0f)
        {
            bounds_0 = first_0;
        }
        else
        {
            bounds_0 = _S120;
        }
    }
    else
    {
        bounds_0 = _S120;
    }
    var _S122 : i32 = i32(2) * index_0(col_25, row_25);
    work_0[_S122] = bounds_0.x;
    work_0[_S122 + i32(1)] = bounds_0.y;
    return;
}

fn hexagon_green_0( p_1 : vec2<i32>,  h_0 : vec2<i32>,  own_2 : f32,  f_5 : u32) -> f32
{
    var _S123 : vec2<i32> = vec2<i32>(i32(3)) * h_0;
    return 0.640625f * xrgb0_0[index_1(p_1 + h_0)].y + 0.359375f * xrgb0_0[index_1(p_1 - vec2<i32>(i32(2)) * h_0)].y + 0.12890625f * (2.0f * own_2 - xrgb0_0[index_1(p_1 + _S123)][f_5] - xrgb0_0[index_1(p_1 - _S123)][f_5]);
}

fn solitary_row_0( row_26 : i32) -> bool
{
    return ((row_26 - i32(params_0.demosaic_0.solitary_y_0)) % i32(3)) == i32(0);
}

fn xrgb_0( d_0 : u32,  idx_14 : i32) -> vec4<f32>
{
    switch(d_0)
    {
    case u32(0):
        {
            return xrgb0_0[idx_14];
        }
    case u32(1):
        {
            return xrgb1_0[idx_14];
        }
    case u32(2):
        {
            return xrgb2_0[idx_14];
        }
    default :
        {
            return xrgb3_0[idx_14];
        }
    }
}

fn clamps_0( a_1 : f32,  lo_1 : f32,  hi_1 : f32) -> f32
{
    var _S124 : f32;
    if((a_1 > lo_1))
    {
        if((a_1 < hi_1))
        {
            _S124 = a_1;
        }
        else
        {
            _S124 = hi_1;
        }
    }
    else
    {
        _S124 = lo_1;
    }
    return _S124;
}

fn set_xrgb_0( d_1 : u32,  idx_15 : i32,  value_3 : vec4<f32>)
{
    switch(d_1)
    {
    case u32(0):
        {
            xrgb0_0[idx_15] = value_3;
        }
    case u32(1):
        {
            xrgb1_0[idx_15] = value_3;
        }
    case u32(2):
        {
            xrgb2_0[idx_15] = value_3;
        }
    default :
        {
            xrgb3_0[idx_15] = value_3;
        }
    }
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn xt_green(@builtin(global_invocation_id) id_15 : vec3<u32>)
{
    var col_26 : i32;
    var row_27 : i32;
    var _S125 : bool = photosite_0(id_15, &(col_26), &(row_27));
    var _S126 : bool;
    if(!_S125)
    {
        _S126 = true;
    }
    else
    {
        _S126 = !within_0(col_26, row_27, i32(3));
    }
    if(_S126)
    {
        return;
    }
    var f_6 : u32 = fcx_0(row_27, col_26);
    if(f_6 == u32(1))
    {
        return;
    }
    var p_2 : vec2<i32> = vec2<i32>(col_26, row_27);
    var idx_16 : i32 = index_1(p_2);
    var _S127 : i32 = i32(2) * idx_16;
    var _S128 : f32 = work_0[_S127];
    var _S129 : f32 = work_0[_S127 + i32(1)];
    var h0_0 : vec2<i32> = hexagon_0(col_26, row_27, u32(0));
    var h1_0 : vec2<i32> = hexagon_0(col_26, row_27, u32(1));
    var h2_0 : vec2<i32> = hexagon_0(col_26, row_27, u32(2));
    var h3_0 : vec2<i32> = hexagon_0(col_26, row_27, u32(3));
    var color_3 : vec4<f32>;
    var _S130 : vec2<i32> = vec2<i32>(i32(2));
    color_3[i32(0)] = 0.6796875f * (xrgb0_0[index_1(p_2 + h1_0)].y + xrgb0_0[index_1(p_2 + h0_0)].y) - 0.1796875f * (xrgb0_0[index_1(p_2 + _S130 * h1_0)].y + xrgb0_0[index_1(p_2 + _S130 * h0_0)].y);
    color_3[i32(1)] = 0.87109375f * xrgb0_0[index_1(p_2 + h3_0)].y + xrgb0_0[index_1(p_2 + h2_0)].y * 0.12999999523162842f + 0.359375f * (xrgb0_0[idx_16][f_6] - xrgb0_0[index_1(p_2 - h2_0)][f_6]);
    color_3[i32(2)] = hexagon_green_0(p_2, hexagon_0(col_26, row_27, u32(4)), xrgb0_0[idx_16][f_6], f_6);
    color_3[i32(3)] = hexagon_green_0(p_2, hexagon_0(col_26, row_27, u32(5)), xrgb0_0[idx_16][f_6], f_6);
    var _S131 : i32;
    if(solitary_row_0(row_27))
    {
        _S131 = i32(1);
    }
    else
    {
        _S131 = i32(0);
    }
    var _S132 : u32 = u32(_S131);
    var c_13 : u32 = u32(0);
    loop
    {
        if((c_13 < u32(4)))
        {
        }
        else
        {
            break;
        }
        var d_2 : u32 = (c_13 ^ (_S132));
        var pixel_1 : vec4<f32> = xrgb_0(d_2, idx_16);
        pixel_1[i32(1)] = clamps_0(color_3[c_13], _S128, _S129);
        set_xrgb_0(d_2, idx_16, pixel_1);
        c_13 = c_13 + u32(1);
    }
    return;
}

fn solitary_col_0( col_27 : i32) -> bool
{
    return ((col_27 - i32(params_0.demosaic_0.solitary_x_0)) % i32(3)) == i32(0);
}

fn set_at_1( v_3 : vec2<f32>,  c_14 : u32,  x_6 : f32) -> vec2<f32>
{
    return select(v_3, vec2<f32>(x_6), vec2<u32>(u32(0), u32(1)) == vec2<u32>(c_14));
}

@compute
@workgroup_size(16, 16, 1)
fn xt_solitary(@builtin(global_invocation_id) id_16 : vec3<u32>)
{
    var col_28 : i32;
    var row_28 : i32;
    var _S133 : bool = photosite_0(id_16, &(col_28), &(row_28));
    var _S134 : bool;
    if(!_S133)
    {
        _S134 = true;
    }
    else
    {
        _S134 = !within_0(col_28, row_28, i32(6));
    }
    if(_S134)
    {
        _S134 = true;
    }
    else
    {
        _S134 = !solitary_row_0(row_28);
    }
    if(_S134)
    {
        _S134 = true;
    }
    else
    {
        _S134 = !solitary_col_0(col_28);
    }
    if(_S134)
    {
        return;
    }
    var d_3 : u32 = params_0.demosaic_0.step_0;
    var _S135 : i32;
    var target_0 : u32;
    if(((params_0.demosaic_0.step_0) < u32(2)))
    {
        target_0 = d_3;
    }
    else
    {
        if((d_3 < u32(4)))
        {
            _S135 = i32(2);
        }
        else
        {
            _S135 = i32(3);
        }
        target_0 = u32(_S135);
    }
    var _S136 : u32 = (d_3 & (u32(1)));
    var _S137 : vec2<i32>;
    if(_S136 == u32(0))
    {
        _S137 = vec2<i32>(i32(1), i32(0));
    }
    else
    {
        _S137 = vec2<i32>(i32(0), i32(1));
    }
    var _S138 : u32 = ((fcx_0(row_28, col_28 + i32(1))) ^ ((_S136 * u32(2))));
    var p_3 : vec2<i32> = vec2<i32>(col_28, row_28);
    var idx_17 : i32 = index_1(p_3);
    var centre_0 : vec4<f32> = xrgb_0(target_0, idx_17);
    var color_4 : vec2<f32> = vec2<f32>(0.0f);
    var c_15 : u32 = u32(0);
    var h_1 : u32 = _S138;
    var diff_0 : f32 = 0.0f;
    loop
    {
        if((c_15 < u32(2)))
        {
        }
        else
        {
            break;
        }
        var off_0 : vec2<i32> = (_S137 << (vec2<u32>(c_15)));
        var ahead_0 : vec4<f32> = xrgb_0(target_0, index_1(p_3 + off_0));
        var behind_0 : vec4<f32> = xrgb_0(target_0, index_1(p_3 - off_0));
        var _S139 : f32 = ahead_0.y;
        var _S140 : f32 = behind_0.y;
        var g_1 : f32 = 2.0f * centre_0.y - _S139 - _S140;
        if(h_1 != u32(0))
        {
            _S135 = i32(1);
        }
        else
        {
            _S135 = i32(0);
        }
        var _S141 : u32 = h_1;
        var _S142 : u32 = h_1;
        var _S143 : vec2<f32> = set_at_1(color_4, u32(_S135), g_1 + ahead_0[h_1] + behind_0[h_1]);
        if((d_3 > u32(1)))
        {
            diff_0 = diff_0 + (sqr_0(_S139 - _S140 - ahead_0[_S141] + behind_0[_S142]) + sqr_0(g_1));
        }
        var _S144 : u32 = c_15 + u32(1);
        var h_2 : u32 = (h_1 ^ (u32(2)));
        color_4 = _S143;
        c_15 = _S144;
        h_1 = h_2;
    }
    if(d_3 == u32(2))
    {
        _S134 = true;
    }
    else
    {
        _S134 = d_3 == u32(4);
    }
    if(_S134)
    {
        aux_0[idx_17] = vec4<f32>(color_4, diff_0, 0.0f);
        return;
    }
    if((d_3 > u32(1)))
    {
        var previous_0 : vec4<f32> = aux_0[idx_17];
        if(((aux_0[idx_17].z) < diff_0))
        {
            color_4 = previous_0.xy;
        }
    }
    centre_0[i32(0)] = color_4.x / 2.0f;
    centre_0[i32(2)] = color_4.y / 2.0f;
    set_xrgb_0(target_0, idx_17, centre_0);
    return;
}

fn set_at_2( v_4 : vec4<f32>,  c_16 : u32,  x_7 : f32) -> vec4<f32>
{
    return select(v_4, vec4<f32>(x_7), vec4<u32>(u32(0), u32(1), u32(2), u32(3)) == vec4<u32>(c_16));
}

@compute
@workgroup_size(16, 16, 1)
fn xt_red_blue(@builtin(global_invocation_id) id_17 : vec3<u32>)
{
    var col_29 : i32;
    var row_29 : i32;
    var _S145 : bool = photosite_0(id_17, &(col_29), &(row_29));
    var along_0 : bool;
    if(!_S145)
    {
        along_0 = true;
    }
    else
    {
        along_0 = !within_0(col_29, row_29, i32(6));
    }
    if(along_0)
    {
        return;
    }
    var f_7 : u32 = u32(2) - fcx_0(row_29, col_29);
    if(f_7 == u32(1))
    {
        return;
    }
    var d_4 : u32 = params_0.demosaic_0.step_0;
    var p_4 : vec2<i32> = vec2<i32>(col_29, row_29);
    var idx_18 : i32 = index_1(p_4);
    var horizontal_0 : bool = solitary_row_0(row_29);
    var c_17 : vec2<i32>;
    if(horizontal_0)
    {
        c_17 = vec2<i32>(i32(1), i32(0));
    }
    else
    {
        c_17 = vec2<i32>(i32(0), i32(1));
    }
    var h_3 : vec2<i32>;
    if(horizontal_0)
    {
        h_3 = vec2<i32>(i32(0), i32(3));
    }
    else
    {
        h_3 = vec2<i32>(i32(3), i32(0));
    }
    var centre_1 : vec4<f32> = xrgb_0(d_4, idx_18);
    var g_2 : f32 = centre_1.y;
    if((d_4 > u32(1)))
    {
        along_0 = true;
    }
    else
    {
        if(d_4 == u32(0))
        {
            along_0 = horizontal_0;
        }
        else
        {
            along_0 = false;
        }
    }
    if(along_0)
    {
        along_0 = true;
    }
    else
    {
        if(d_4 == u32(1))
        {
            along_0 = !horizontal_0;
        }
        else
        {
            along_0 = false;
        }
    }
    if(along_0)
    {
        along_0 = true;
    }
    else
    {
        along_0 = ((abs(g_2 - xrgb_0(d_4, index_1(p_4 + c_17)).y) + abs(g_2 - xrgb_0(d_4, index_1(p_4 - c_17)).y)) < (2.0f * (abs(g_2 - xrgb_0(d_4, index_1(p_4 + h_3)).y) + abs(g_2 - xrgb_0(d_4, index_1(p_4 - h_3)).y))));
    }
    var i_0 : vec2<i32>;
    if(along_0)
    {
        i_0 = c_17;
    }
    else
    {
        i_0 = h_3;
    }
    var ahead_1 : vec4<f32> = xrgb_0(d_4, index_1(p_4 + i_0));
    var behind_1 : vec4<f32> = xrgb_0(d_4, index_1(p_4 - i_0));
    set_xrgb_0(d_4, idx_18, set_at_2(centre_1, f_7, (ahead_1[f_7] + behind_1[f_7] + 2.0f * g_2 - ahead_1.y - behind_1.y) / 2.0f));
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn xt_two_by_two(@builtin(global_invocation_id) id_18 : vec3<u32>)
{
    var col_30 : i32;
    var row_30 : i32;
    var _S146 : bool = photosite_0(id_18, &(col_30), &(row_30));
    var _S147 : bool;
    if(!_S146)
    {
        _S147 = true;
    }
    else
    {
        _S147 = !within_0(col_30, row_30, i32(8));
    }
    if(_S147)
    {
        _S147 = true;
    }
    else
    {
        _S147 = solitary_row_0(row_30);
    }
    if(_S147)
    {
        _S147 = true;
    }
    else
    {
        _S147 = solitary_col_0(col_30);
    }
    if(_S147)
    {
        return;
    }
    var n_0 : u32 = params_0.demosaic_0.step_0 / u32(2);
    var p_5 : vec2<i32> = vec2<i32>(col_30, row_30);
    var idx_19 : i32 = index_1(p_5);
    var a_2 : vec2<i32> = hexagon_0(col_30, row_30, params_0.demosaic_0.step_0);
    var b_1 : vec2<i32> = hexagon_0(col_30, row_30, params_0.demosaic_0.step_0 + u32(1));
    var centre_2 : vec4<f32> = xrgb_0(n_0, idx_19);
    var pa_0 : vec4<f32> = xrgb_0(n_0, index_1(p_5 + a_2));
    var pb_0 : vec4<f32> = xrgb_0(n_0, index_1(p_5 + b_1));
    if((any(((a_2 + b_1) != vec2<i32>(i32(0))))))
    {
        var g_3 : f32 = 3.0f * centre_2.y - 2.0f * pa_0.y - pb_0.y;
        centre_2[i32(0)] = (g_3 + 2.0f * pa_0.x + pb_0.x) / 3.0f;
        centre_2[i32(2)] = (g_3 + 2.0f * pa_0.z + pb_0.z) / 3.0f;
    }
    else
    {
        var g_4 : f32 = 2.0f * centre_2.y - pa_0.y - pb_0.y;
        centre_2[i32(0)] = (g_4 + pa_0.x + pb_0.x) / 2.0f;
        centre_2[i32(2)] = (g_4 + pa_0.z + pb_0.z) / 2.0f;
    }
    set_xrgb_0(n_0, idx_19, centre_2);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn xt_yuv(@builtin(global_invocation_id) id_19 : vec3<u32>)
{
    var col_31 : i32;
    var row_31 : i32;
    var _S148 : bool = photosite_0(id_19, &(col_31), &(row_31));
    var _S149 : bool;
    if(!_S148)
    {
        _S149 = true;
    }
    else
    {
        _S149 = !within_0(col_31, row_31, i32(8));
    }
    if(_S149)
    {
        return;
    }
    var idx_20 : i32 = index_0(col_31, row_31);
    var rgb_2 : vec4<f32> = xrgb_0(params_0.demosaic_0.step_0, idx_20);
    var _S150 : f32 = rgb_2.x;
    var _S151 : f32 = rgb_2.z;
    var y_3 : f32 = 0.26269999146461487f * _S150 + 0.67799997329711914f * rgb_2.y + 0.059300001710653305f * _S151;
    aux_0[idx_20] = vec4<f32>(y_3, (_S151 - y_3) * 0.56432998180389404f, (_S150 - y_3) * 0.6781499981880188f, 0.0f);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn xt_derivative(@builtin(global_invocation_id) id_20 : vec3<u32>)
{
    var col_32 : i32;
    var row_32 : i32;
    var _S152 : bool = photosite_0(id_20, &(col_32), &(row_32));
    var _S153 : bool;
    if(!_S152)
    {
        _S153 = true;
    }
    else
    {
        _S153 = !within_0(col_32, row_32, i32(9));
    }
    if(_S153)
    {
        return;
    }
    var d_5 : u32 = params_0.demosaic_0.step_0;
    var dir_0 : vec2<i32>;
    if((params_0.demosaic_0.step_0) == u32(0))
    {
        dir_0 = vec2<i32>(i32(1), i32(0));
    }
    else
    {
        if(d_5 == u32(1))
        {
            dir_0 = vec2<i32>(i32(0), i32(1));
        }
        else
        {
            if(d_5 == u32(2))
            {
                dir_0 = vec2<i32>(i32(1), i32(1));
            }
            else
            {
                dir_0 = vec2<i32>(i32(-1), i32(1));
            }
        }
    }
    var p_6 : vec2<i32> = vec2<i32>(col_32, row_32);
    var idx_21 : i32 = index_1(p_6);
    var second_0 : vec3<f32> = vec3<f32>(2.0f) * aux_0[idx_21].xyz - aux_0[index_1(p_6 + dir_0)].xyz - aux_0[index_1(p_6 - dir_0)].xyz;
    work_0[i32(d_5) * i32(params_0.demosaic_0.width_1 * params_0.demosaic_0.height_1) + idx_21] = dot(second_0, second_0);
    return;
}

fn homogeneity_0( d_6 : i32,  col_33 : i32,  row_33 : i32,  plane_1 : i32,  threshold_0 : f32) -> f32
{
    var v_5 : i32 = i32(-1);
    var count_5 : f32 = 0.0f;
    loop
    {
        if((v_5 <= i32(1)))
        {
        }
        else
        {
            break;
        }
        var h_4 : i32 = i32(-1);
        loop
        {
            if((h_4 <= i32(1)))
            {
            }
            else
            {
                break;
            }
            var _S154 : f32;
            if((work_0[d_6 * plane_1 + index_0(col_33 + h_4, row_33 + v_5)] <= threshold_0))
            {
                _S154 = 1.0f;
            }
            else
            {
                _S154 = 0.0f;
            }
            var count_6 : f32 = count_5 + _S154;
            h_4 = h_4 + i32(1);
            count_5 = count_6;
        }
        v_5 = v_5 + i32(1);
    }
    return count_5;
}

@compute
@workgroup_size(16, 16, 1)
fn xt_homogeneity(@builtin(global_invocation_id) id_21 : vec3<u32>)
{
    var col_34 : i32;
    var row_34 : i32;
    var _S155 : bool = photosite_0(id_21, &(col_34), &(row_34));
    var _S156 : bool;
    if(!_S155)
    {
        _S156 = true;
    }
    else
    {
        _S156 = !within_0(col_34, row_34, i32(10));
    }
    if(_S156)
    {
        return;
    }
    var idx_22 : i32 = index_0(col_34, row_34);
    var plane_2 : i32 = i32(params_0.demosaic_0.width_1 * params_0.demosaic_0.height_1);
    var d_7 : i32 = i32(0);
    var threshold_1 : f32 = 3.4028234663852886e+38f;
    loop
    {
        if((d_7 < i32(4)))
        {
        }
        else
        {
            break;
        }
        var _S157 : f32 = min(threshold_1, work_0[d_7 * plane_2 + idx_22]);
        d_7 = d_7 + i32(1);
        threshold_1 = _S157;
    }
    var threshold_2 : f32 = threshold_1 * 8.0f;
    aux_0[idx_22] = vec4<f32>(homogeneity_0(i32(0), col_34, row_34, plane_2, threshold_2), homogeneity_0(i32(1), col_34, row_34, plane_2, threshold_2), homogeneity_0(i32(2), col_34, row_34, plane_2, threshold_2), homogeneity_0(i32(3), col_34, row_34, plane_2, threshold_2));
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn xt_homogeneity_sum(@builtin(global_invocation_id) id_22 : vec3<u32>)
{
    var h_5 : i32;
    var col_35 : i32;
    var row_35 : i32;
    var _S158 : bool = photosite_0(id_22, &(col_35), &(row_35));
    var _S159 : bool;
    if(!_S158)
    {
        _S159 = true;
    }
    else
    {
        _S159 = !within_0(col_35, row_35, i32(12));
    }
    if(_S159)
    {
        return;
    }
    var _S160 : vec4<f32> = vec4<f32>(0.0f);
    var v_6 : i32 = i32(-2);
    var sum_4 : vec4<f32> = _S160;
    loop
    {
        if((v_6 <= i32(2)))
        {
        }
        else
        {
            break;
        }
        h_5 = i32(-2);
        loop
        {
            if((h_5 <= i32(2)))
            {
            }
            else
            {
                break;
            }
            var sum_5 : vec4<f32> = sum_4 + aux_0[index_0(col_35 + h_5, row_35 + v_6)];
            h_5 = h_5 + i32(1);
            sum_4 = sum_5;
        }
        v_6 = v_6 + i32(1);
    }
    var _S161 : i32 = index_0(col_35, row_35);
    var _S162 : i32 = i32(params_0.demosaic_0.width_1 * params_0.demosaic_0.height_1);
    h_5 = i32(0);
    loop
    {
        if((h_5 < i32(4)))
        {
        }
        else
        {
            break;
        }
        work_0[h_5 * _S162 + _S161] = sum_4[h_5];
        h_5 = h_5 + i32(1);
    }
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn xt_final(@builtin(global_invocation_id) id_23 : vec3<u32>)
{
    var col_36 : i32;
    var row_36 : i32;
    var _S163 : bool = photosite_0(id_23, &(col_36), &(row_36));
    var _S164 : bool;
    if(!_S163)
    {
        _S164 = true;
    }
    else
    {
        _S164 = !within_0(col_36, row_36, i32(12));
    }
    if(_S164)
    {
        return;
    }
    var idx_23 : i32 = index_0(col_36, row_36);
    var plane_3 : i32 = i32(params_0.demosaic_0.width_1 * params_0.demosaic_0.height_1);
    var _S165 : u32 = u32(work_0[idx_23]);
    var _S166 : u32 = u32(work_0[plane_3 + idx_23]);
    var _S167 : u32 = u32(work_0[i32(2) * plane_3 + idx_23]);
    var _S168 : u32 = u32(work_0[i32(3) * plane_3 + idx_23]);
    var hm_0 : vec4<u32> = vec4<u32>(_S165, _S166, _S167, _S168);
    var _S169 : u32 = max(max(_S165, _S166), max(_S167, _S168));
    var best_0 : u32 = _S169 - ((_S169 >> (u32(3))));
    var _S170 : vec4<f32> = vec4<f32>(0.0f);
    var d_8 : u32 = u32(0);
    var sum_6 : vec4<f32> = _S170;
    loop
    {
        if((d_8 < u32(4)))
        {
        }
        else
        {
            break;
        }
        if(((hm_0[d_8]) >= best_0))
        {
            sum_6 = sum_6 + vec4<f32>(xrgb_0(d_8, idx_23).xyz, 1.0f);
        }
        d_8 = d_8 + u32(1);
    }
    write_output_0(col_36, row_36, sum_6.xyz / vec3<f32>(sum_6.w));
    return;
}

