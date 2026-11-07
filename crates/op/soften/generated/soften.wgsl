// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2011-2026  darktable developers (ported from src/iop/soften.c)
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

// Generated from slang/soften.slang by tools/build-kernels.sh. Do not edit.

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

struct SoftenParams_std140_0
{
    @align(16) image_0 : vec4<u32>,
    @align(16) factors_0 : vec4<f32>,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) soften_0 : SoftenParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(0) @group(0) var<storage, read> input_0 : array<vec4<f32>>;

@binding(13) @group(0) var<storage, read_write> write_r_0 : array<f32>;

@binding(14) @group(0) var<storage, read_write> write_g_0 : array<f32>;

@binding(15) @group(0) var<storage, read_write> write_b_0 : array<f32>;

@binding(10) @group(0) var<storage, read_write> read_r_0 : array<f32>;

@binding(11) @group(0) var<storage, read_write> read_g_0 : array<f32>;

@binding(12) @group(0) var<storage, read_write> read_b_0 : array<f32>;

@binding(1) @group(0) var<storage, read_write> output_0 : array<vec4<f32>>;

@binding(8) @group(0) var<storage, read> derivative_in_0 : array<vec4<f32>>;

@binding(9) @group(0) var<storage, read_write> derivative_out_0 : array<vec4<f32>>;

struct Mix_0
{
     mix_0 : f32,
};

struct DiffPair_Mix_0
{
     primal_0 : Mix_0,
     differential_0 : Mix_0,
};

struct s_paramCtx_s_bwdCallableCtx_soften_0
{
     value0_0 : vec3<f32>,
     value1_0 : vec3<f32>,
};

struct s_bwdCallableCtx_soften_0
{
     value0_1 : s_paramCtx_s_bwdCallableCtx_soften_0,
};

fn Mix_x24_syn_dzero_0() -> Mix_0
{
    var result_0 : Mix_0;
    result_0.mix_0 = 0.0f;
    return result_0;
}

fn Mix_x24_syn_dadd_0( SLANG_anonymous_0_0 : Mix_0,  SLANG_anonymous_1_0 : Mix_0) -> Mix_0
{
    var result_1 : Mix_0;
    result_1.mix_0 = SLANG_anonymous_0_0.mix_0 + SLANG_anonymous_1_0.mix_0;
    return result_1;
}

fn s_bwdProp_soften_0( _S1 : s_bwdCallableCtx_soften_0,  _S2 : ptr<function, Mix_0>,  _S3 : vec3<f32>)
{
    var _S4 : Mix_0 = Mix_x24_syn_dzero_0();
    var _S5 : vec3<f32> = _S1.value0_1.value0_0 * _S3;
    var _S6 : vec3<f32> = _S1.value0_1.value1_0 * _S3;
    var _S7 : f32 = - (_S5[i32(0)] + _S5[i32(1)] + _S5[i32(2)]) + (_S6[i32(0)] + _S6[i32(1)] + _S6[i32(2)]);
    var _S8 : Mix_0 = _S4;
    _S8.mix_0 = _S7;
    (*_S2) = Mix_x24_syn_dadd_0(_S4, _S8);
    return;
}

fn _S9( _S10 : vec3<f32>,  _S11 : vec3<f32>,  _S12 : ptr<function, DiffPair_Mix_0>,  _S13 : vec3<f32>)
{
    var _S14 : s_paramCtx_s_bwdCallableCtx_soften_0;
    _S14.value0_0 = _S10;
    _S14.value1_0 = _S11;
    var _S15 : s_bwdCallableCtx_soften_0 = s_bwdCallableCtx_soften_0( _S14 );
    var _S16 : Mix_0 = (*_S12).differential_0;
    s_bwdProp_soften_0(_S15, &(_S16), _S13);
    (*_S12).differential_0 = _S16;
    return;
}

struct Line_0
{
     start_0 : u32,
     stride_0 : u32,
};

fn read_blur_0( i_0 : u32) -> vec3<f32>
{
    return vec3<f32>(read_r_0[i_0], read_g_0[i_0], read_b_0[i_0]);
}

fn Line_read_0( this_0 : Line_0,  i_1 : u32) -> vec3<f32>
{
    return read_blur_0(this_0.start_0 + i_1 * this_0.stride_0);
}

fn Line_write_0( this_1 : Line_0,  i_2 : u32,  value_0 : vec3<f32>)
{
    var k_0 : u32 = this_1.start_0 + i_2 * this_1.stride_0;
    write_r_0[k_0] = value_0.x;
    write_g_0[k_0] = value_0.y;
    write_b_0[k_0] = value_0.z;
    return;
}

fn box_mean_line_0( line_0 : Line_0,  count_0 : u32,  radius_0 : u32)
{
    var _S17 : bool;
    var sum_0 : vec3<f32> = vec3<f32>(0.0f);
    var x_0 : u32 = u32(0);
    var hits_0 : u32 = u32(0);
    loop
    {
        if((x_0 < (min(radius_0, count_0))))
        {
        }
        else
        {
            break;
        }
        var _S18 : u32 = hits_0 + u32(1);
        var _S19 : vec3<f32> = sum_0 + Line_read_0(line_0, x_0);
        var _S20 : u32 = x_0 + u32(1);
        sum_0 = _S19;
        x_0 = _S20;
        hits_0 = _S18;
    }
    x_0 = u32(0);
    loop
    {
        if((x_0 <= radius_0))
        {
            _S17 = ((x_0 + radius_0) < count_0);
        }
        else
        {
            _S17 = false;
        }
        if(_S17)
        {
        }
        else
        {
            break;
        }
        var hits_1 : u32 = hits_0 + u32(1);
        var _S21 : vec3<f32> = sum_0 + Line_read_0(line_0, x_0 + radius_0);
        Line_write_0(line_0, x_0, _S21 / vec3<f32>(f32(i32(hits_1))));
        var _S22 : u32 = x_0 + u32(1);
        sum_0 = _S21;
        x_0 = _S22;
        hits_0 = hits_1;
    }
    loop
    {
        if((x_0 <= radius_0))
        {
            _S17 = (x_0 < count_0);
        }
        else
        {
            _S17 = false;
        }
        if(_S17)
        {
        }
        else
        {
            break;
        }
        Line_write_0(line_0, x_0, sum_0 / vec3<f32>(f32(i32(hits_0))));
        x_0 = x_0 + u32(1);
    }
    loop
    {
        var _S23 : u32 = x_0 + radius_0;
        if((_S23 < count_0))
        {
        }
        else
        {
            break;
        }
        var _S24 : vec3<f32> = sum_0 - Line_read_0(line_0, x_0 - radius_0 - u32(1)) + Line_read_0(line_0, _S23);
        Line_write_0(line_0, x_0, _S24 / vec3<f32>(f32(i32(hits_0))));
        var _S25 : u32 = x_0 + u32(1);
        sum_0 = _S24;
        x_0 = _S25;
    }
    loop
    {
        if((x_0 < count_0))
        {
        }
        else
        {
            break;
        }
        var hits_2 : u32 = hits_0 - u32(1);
        var _S26 : vec3<f32> = sum_0 - Line_read_0(line_0, x_0 - radius_0 - u32(1));
        Line_write_0(line_0, x_0, _S26 / vec3<f32>(f32(i32(hits_2))));
        var _S27 : u32 = x_0 + u32(1);
        sum_0 = _S26;
        x_0 = _S27;
        hits_0 = hits_2;
    }
    return;
}

fn rgb_to_hsl_0( rgb_0 : vec3<f32>) -> vec3<f32>
{
    var r_0 : f32 = rgb_0.x;
    var g_0 : f32 = rgb_0.y;
    var b_0 : f32 = rgb_0.z;
    var _S28 : f32 = max(max(r_0, g_0), b_0);
    var _S29 : f32 = min(min(r_0, g_0), b_0);
    var delta_0 : f32 = _S28 - _S29;
    var l_0 : f32 = (_S29 + _S28) / 2.0f;
    var s_0 : f32;
    var h_0 : f32;
    if(delta_0 != 0.0f)
    {
        if((l_0 < 0.5f))
        {
            s_0 = delta_0 / max(_S28 + _S29, 1.52587890625e-05f);
        }
        else
        {
            s_0 = delta_0 / max(2.0f - _S28 - _S29, 1.52587890625e-05f);
        }
        if(_S28 == r_0)
        {
            h_0 = (g_0 - b_0) / delta_0;
        }
        else
        {
            if(_S28 == g_0)
            {
                h_0 = 2.0f + (b_0 - r_0) / delta_0;
            }
            else
            {
                if(_S28 == b_0)
                {
                    h_0 = 4.0f + (r_0 - g_0) / delta_0;
                }
                else
                {
                    h_0 = 0.0f;
                }
            }
        }
        var h_1 : f32 = h_0 / 6.0f;
        if((h_1 < 0.0f))
        {
            h_0 = h_1 + 1.0f;
        }
        else
        {
            if((h_1 > 1.0f))
            {
                h_0 = h_1 - 1.0f;
            }
            else
            {
                h_0 = h_1;
            }
        }
    }
    else
    {
        s_0 = 0.0f;
        h_0 = 0.0f;
    }
    return vec3<f32>(h_0, s_0, l_0);
}

fn hue_to_rgb_0( m1_0 : f32,  m2_0 : f32,  hue_0 : f32) -> f32
{
    if((hue_0 < 1.0f))
    {
        return m1_0 + (m2_0 - m1_0) * hue_0;
    }
    else
    {
        if((hue_0 < 3.0f))
        {
            return m2_0;
        }
        else
        {
            var _S30 : f32;
            if((hue_0 < 4.0f))
            {
                _S30 = m1_0 + (m2_0 - m1_0) * (4.0f - hue_0);
            }
            else
            {
                _S30 = m1_0;
            }
            return _S30;
        }
    }
}

fn hsl_to_rgb_0( hsl_0 : vec3<f32>) -> vec3<f32>
{
    var h_2 : f32 = hsl_0.x;
    var s_1 : f32 = hsl_0.y;
    var l_1 : f32 = hsl_0.z;
    if(s_1 == 0.0f)
    {
        return vec3<f32>(l_1, l_1, l_1);
    }
    var m2_1 : f32;
    if((l_1 < 0.5f))
    {
        m2_1 = l_1 * (1.0f + s_1);
    }
    else
    {
        m2_1 = l_1 + s_1 - l_1 * s_1;
    }
    var m1_1 : f32 = 2.0f * l_1 - m2_1;
    var h_3 : f32 = h_2 * 6.0f;
    var _S31 : f32;
    if((h_3 < 4.0f))
    {
        _S31 = h_3 + 2.0f;
    }
    else
    {
        _S31 = h_3 - 4.0f;
    }
    var _S32 : f32 = hue_to_rgb_0(m1_1, m2_1, _S31);
    var _S33 : f32 = hue_to_rgb_0(m1_1, m2_1, h_3);
    if((h_3 > 2.0f))
    {
        _S31 = h_3 - 2.0f;
    }
    else
    {
        _S31 = h_3 + 4.0f;
    }
    return vec3<f32>(_S32, _S33, hue_to_rgb_0(m1_1, m2_1, _S31));
}

fn scratch_index_0( id_0 : vec3<u32>,  p_0 : ptr<function, SoftenParams_std140_0>) -> u32
{
    return (params_0.pass_0.origin_y_0 + id_0.y) * (*p_0).image_0.x + params_0.pass_0.origin_x_0 + id_0.x;
}

@compute
@workgroup_size(16, 16, 1)
fn overexpose(@builtin(global_invocation_id) id_1 : vec3<u32>)
{
    var _S34 : u32 = id_1.x;
    var _S35 : bool;
    if((_S34 >= (params_0.pass_0.width_0)))
    {
        _S35 = true;
    }
    else
    {
        _S35 = ((id_1.y) >= (params_0.pass_0.height_0));
    }
    if(_S35)
    {
        return;
    }
    var _S36 : SoftenParams_std140_0 = params_0.soften_0;
    var hsl_1 : vec3<f32> = rgb_to_hsl_0(clamp(input_0[id_1.y * params_0.pass_0.width_0 + _S34].xyz, vec3<f32>(-1.0e+06f), vec3<f32>(1.0e+06f)));
    var softened_0 : vec3<f32> = hsl_to_rgb_0(vec3<f32>(hsl_1.x, clamp(hsl_1.y * _S36.factors_0.x, 0.0f, 1.0f), clamp(hsl_1.z * _S36.factors_0.y, 0.0f, 1.0f)));
    var _S37 : u32 = scratch_index_0(id_1, &(_S36));
    write_r_0[_S37] = softened_0.x;
    write_g_0[_S37] = softened_0.y;
    write_b_0[_S37] = softened_0.z;
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn box_mean(@builtin(global_invocation_id) id_2 : vec3<u32>)
{
    var p_1 : SoftenParams_std140_0 = params_0.soften_0;
    var width_1 : u32 = params_0.soften_0.image_0.x;
    var height_1 : u32 = params_0.soften_0.image_0.y;
    var rows_0 : bool = (params_0.soften_0.image_0.w) == u32(0);
    var index_0 : u32 = id_2.y * u32(16) + id_2.x;
    var _S38 : u32;
    if(rows_0)
    {
        _S38 = height_1;
    }
    else
    {
        _S38 = width_1;
    }
    if((index_0 >= _S38))
    {
        return;
    }
    var line_1 : Line_0;
    if(rows_0)
    {
        _S38 = index_0 * width_1;
    }
    else
    {
        _S38 = index_0;
    }
    line_1.start_0 = _S38;
    if(rows_0)
    {
        _S38 = u32(1);
    }
    else
    {
        _S38 = width_1;
    }
    line_1.stride_0 = _S38;
    var _S39 : Line_0 = line_1;
    if(rows_0)
    {
        _S38 = width_1;
    }
    else
    {
        _S38 = height_1;
    }
    box_mean_line_0(_S39, _S38, p_1.image_0.z);
    return;
}

fn mix_of_0( p_2 : ptr<function, SoftenParams_std140_0>) -> Mix_0
{
    var m_0 : Mix_0;
    m_0.mix_0 = (*p_2).factors_0.z;
    return m_0;
}

fn soften_1( rgb_1 : vec3<f32>,  blur_0 : vec3<f32>,  m_1 : Mix_0) -> vec3<f32>
{
    return vec3<f32>(m_1.mix_0) * blur_0 + vec3<f32>((1.0f - m_1.mix_0)) * rgb_1;
}

struct DiffPair_vectorx3Cfloatx2C3x3E_0
{
     primal_0 : vec3<f32>,
     differential_0 : vec3<f32>,
};

fn s_fwd_soften_0( rgb_2 : vec3<f32>,  blur_1 : vec3<f32>,  dpm_0 : DiffPair_Mix_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S40 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(dpm_0.primal_0.mix_0) * blur_1 + vec3<f32>((1.0f - dpm_0.primal_0.mix_0)) * rgb_2, vec3<f32>(dpm_0.differential_0.mix_0) * blur_1 + vec3<f32>((0.0f - dpm_0.differential_0.mix_0)) * rgb_2 );
    return _S40;
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_3 : vec3<u32>)
{
    var _S41 : u32 = id_3.x;
    var _S42 : bool;
    if((_S41 >= (params_0.pass_0.width_0)))
    {
        _S42 = true;
    }
    else
    {
        _S42 = ((id_3.y) >= (params_0.pass_0.height_0));
    }
    if(_S42)
    {
        return;
    }
    var _S43 : SoftenParams_std140_0 = params_0.soften_0;
    var i_3 : u32 = id_3.y * params_0.pass_0.width_0 + _S41;
    var _S44 : u32 = scratch_index_0(id_3, &(_S43));
    var _S45 : vec3<f32> = read_blur_0(_S44);
    var _S46 : Mix_0 = mix_of_0(&(_S43));
    var pixel_0 : vec4<f32> = input_0[i_3];
    var rgb_3 : vec3<f32> = pixel_0.xyz;
    var k_1 : u32 = u32(0);
    loop
    {
        if((k_1 < (params_0.pass_0.repeat_0)))
        {
        }
        else
        {
            break;
        }
        var _S47 : vec3<f32> = soften_1(rgb_3, _S45, _S46);
        var _S48 : u32 = k_1 + u32(1);
        rgb_3 = _S47;
        k_1 = _S48;
    }
    output_0[i_3] = vec4<f32>(rgb_3, pixel_0.w);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn soften_backward(@builtin(global_invocation_id) id_4 : vec3<u32>)
{
    var _S49 : u32 = id_4.x;
    var _S50 : bool;
    if((_S49 >= (params_0.pass_0.width_0)))
    {
        _S50 = true;
    }
    else
    {
        _S50 = ((id_4.y) >= (params_0.pass_0.height_0));
    }
    if(_S50)
    {
        return;
    }
    var _S51 : SoftenParams_std140_0 = params_0.soften_0;
    var i_4 : u32 = id_4.y * params_0.pass_0.width_0 + _S49;
    var _S52 : u32 = scratch_index_0(id_4, &(_S51));
    var blur_2 : vec3<f32> = read_blur_0(_S52);
    var _S53 : Mix_0 = mix_of_0(&(_S51));
    var _S54 : Mix_0 = Mix_x24_syn_dzero_0();
    var m_2 : DiffPair_Mix_0;
    m_2.primal_0 = _S53;
    m_2.differential_0 = _S54;
    _S9(input_0[i_4].xyz, blur_2, &(m_2), derivative_in_0[i_4].xyz);
    derivative_out_0[i_4] = vec4<f32>(m_2.differential_0.mix_0 / 100.0f, 0.0f, 0.0f, 0.0f);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn soften_forward_diff(@builtin(global_invocation_id) id_5 : vec3<u32>)
{
    var _S55 : u32 = id_5.x;
    var _S56 : bool;
    if((_S55 >= (params_0.pass_0.width_0)))
    {
        _S56 = true;
    }
    else
    {
        _S56 = ((id_5.y) >= (params_0.pass_0.height_0));
    }
    if(_S56)
    {
        return;
    }
    var _S57 : SoftenParams_std140_0 = params_0.soften_0;
    var i_5 : u32 = id_5.y * params_0.pass_0.width_0 + _S55;
    var _S58 : u32 = scratch_index_0(id_5, &(_S57));
    var blur_3 : vec3<f32> = read_blur_0(_S58);
    var direction_0 : Mix_0;
    direction_0.mix_0 = derivative_in_0[i_5].x / 100.0f;
    var _S59 : vec3<f32> = input_0[i_5].xyz;
    var _S60 : Mix_0 = mix_of_0(&(_S57));
    var _S61 : DiffPair_Mix_0 = DiffPair_Mix_0( _S60, direction_0 );
    derivative_out_0[i_5] = vec4<f32>(s_fwd_soften_0(_S59, blur_3, _S61).differential_0, 0.0f);
    return;
}

