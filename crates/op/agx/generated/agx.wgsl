// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2025-2026  darktable developers (ported from src/iop/agx.c)
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

// Generated from slang/agx.slang by tools/build-kernels.sh. Do not edit.

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

struct AgxParams_std140_0
{
    @align(16) pipe_to_base_r0_0 : vec4<f32>,
    @align(16) pipe_to_base_r1_0 : vec4<f32>,
    @align(16) pipe_to_base_r2_0 : vec4<f32>,
    @align(16) base_to_rendering_r0_0 : vec4<f32>,
    @align(16) base_to_rendering_r1_0 : vec4<f32>,
    @align(16) base_to_rendering_r2_0 : vec4<f32>,
    @align(16) rendering_to_pipe_r0_0 : vec4<f32>,
    @align(16) rendering_to_pipe_r1_0 : vec4<f32>,
    @align(16) rendering_to_pipe_r2_0 : vec4<f32>,
    @align(16) rendering_luminance_0 : vec4<f32>,
    @align(16) black_ev_0 : f32,
    @align(4) range_ev_0 : f32,
    @align(8) gamma_0 : f32,
    @align(4) slope_0 : f32,
    @align(16) intercept_0 : f32,
    @align(4) target_black_0 : f32,
    @align(8) target_white_0 : f32,
    @align(4) _pad0_0 : f32,
    @align(16) toe_transition_x_0 : f32,
    @align(4) toe_transition_y_0 : f32,
    @align(8) toe_scale_0 : f32,
    @align(4) toe_power_0 : f32,
    @align(16) toe_fallback_coefficient_0 : f32,
    @align(4) toe_fallback_power_0 : f32,
    @align(8) toe_fallback_0 : u32,
    @align(4) _pad1_0 : f32,
    @align(16) shoulder_transition_x_0 : f32,
    @align(4) shoulder_transition_y_0 : f32,
    @align(8) shoulder_scale_0 : f32,
    @align(4) shoulder_power_0 : f32,
    @align(16) shoulder_fallback_coefficient_0 : f32,
    @align(4) shoulder_fallback_power_0 : f32,
    @align(8) shoulder_fallback_0 : u32,
    @align(4) _pad2_0 : f32,
    @align(16) look_slope_0 : f32,
    @align(4) look_lift_0 : f32,
    @align(8) look_power_0 : f32,
    @align(4) look_saturation_0 : f32,
    @align(16) look_hue_mix_0 : f32,
    @align(4) _pad3_0 : f32,
    @align(8) _pad4_0 : f32,
    @align(4) _pad5_0 : f32,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) agx_0 : AgxParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(0) @group(0) var<storage, read> input_0 : array<vec4<f32>>;

@binding(1) @group(0) var<storage, read_write> output_0 : array<vec4<f32>>;

@binding(8) @group(0) var<storage, read> derivative_in_0 : array<vec4<f32>>;

@binding(9) @group(0) var<storage, read_write> derivative_out_0 : array<vec4<f32>>;

struct Tuple_0
{
     value0_0 : f32,
};

fn s_apply_abs_0( x_0 : f32) -> Tuple_0
{
    var _S1 : Tuple_0 = Tuple_0( abs(x_0) );
    return _S1;
}

struct s_bwdCallableCtx_abs_0
{
     _S2 : f32,
};

struct DiffPair_float_0
{
     primal_0 : f32,
     differential_0 : f32,
};

fn _d_abs_0( dpx_0 : ptr<function, DiffPair_float_0>,  dOut_0 : f32)
{
    var _S3 : f32 = select(select(-1.0f, 0.0f, ((*dpx_0).primal_0) == 0.0f), 1.0f, (((*dpx_0).primal_0) > 0.0f)) * dOut_0;
    (*dpx_0).primal_0 = (*dpx_0).primal_0;
    (*dpx_0).differential_0 = _S3;
    return;
}

fn s_bwdProp_abs_0( ctx_0 : s_bwdCallableCtx_abs_0,  d_x_0 : ptr<function, f32>,  d_out_0 : f32)
{
    var _S4 : DiffPair_float_0;
    _S4.primal_0 = ctx_0._S2;
    _d_abs_0(&(_S4), d_out_0);
    (*d_x_0) = _S4.differential_0;
    return;
}

struct Tuple_1
{
     value0_1 : f32,
};

fn s_apply_max_0( x_1 : f32,  y_0 : f32) -> Tuple_1
{
    var _S5 : Tuple_1 = Tuple_1( max(x_1, y_0) );
    return _S5;
}

struct s_bwdCallableCtx_max_0
{
     _S6 : f32,
     _S7 : f32,
};

fn _d_max_0( dpx_1 : ptr<function, DiffPair_float_0>,  dpy_0 : ptr<function, DiffPair_float_0>,  dOut_1 : f32)
{
    var _S8 : DiffPair_float_0 = (*dpx_1);
    var _S9 : f32;
    if((((*dpx_1).primal_0) > ((*dpy_0).primal_0)))
    {
        _S9 = dOut_1;
    }
    else
    {
        if((((*dpx_1).primal_0) < ((*dpy_0).primal_0)))
        {
            _S9 = 0.0f;
        }
        else
        {
            _S9 = 0.5f * dOut_1;
        }
    }
    (*dpx_1).primal_0 = _S8.primal_0;
    (*dpx_1).differential_0 = _S9;
    var _S10 : DiffPair_float_0 = (*dpy_0);
    if((((*dpy_0).primal_0) > (_S8.primal_0)))
    {
        _S9 = dOut_1;
    }
    else
    {
        if((((*dpy_0).primal_0) < ((*dpx_1).primal_0)))
        {
            _S9 = 0.0f;
        }
        else
        {
            _S9 = 0.5f * dOut_1;
        }
    }
    (*dpy_0).primal_0 = _S10.primal_0;
    (*dpy_0).differential_0 = _S9;
    return;
}

fn s_bwdProp_max_0( ctx_1 : s_bwdCallableCtx_max_0,  d_x_1 : ptr<function, f32>,  d_y_0 : ptr<function, f32>,  d_out_1 : f32)
{
    var _S11 : DiffPair_float_0;
    _S11.primal_0 = ctx_1._S6;
    var _S12 : DiffPair_float_0;
    _S12.primal_0 = ctx_1._S7;
    _d_max_0(&(_S11), &(_S12), d_out_1);
    (*d_x_1) = _S11.differential_0;
    (*d_y_0) = _S12.differential_0;
    return;
}

struct Tuple_2
{
     value0_2 : f32,
};

fn s_apply_min_0( x_2 : f32,  y_1 : f32) -> Tuple_2
{
    var _S13 : Tuple_2 = Tuple_2( min(x_2, y_1) );
    return _S13;
}

struct s_bwdCallableCtx_min_0
{
     _S14 : f32,
     _S15 : f32,
};

fn _d_min_0( dpx_2 : ptr<function, DiffPair_float_0>,  dpy_1 : ptr<function, DiffPair_float_0>,  dOut_2 : f32)
{
    var _S16 : DiffPair_float_0 = (*dpx_2);
    var _S17 : f32;
    if((((*dpx_2).primal_0) < ((*dpy_1).primal_0)))
    {
        _S17 = dOut_2;
    }
    else
    {
        if((((*dpx_2).primal_0) > ((*dpy_1).primal_0)))
        {
            _S17 = 0.0f;
        }
        else
        {
            _S17 = 0.5f * dOut_2;
        }
    }
    (*dpx_2).primal_0 = _S16.primal_0;
    (*dpx_2).differential_0 = _S17;
    var _S18 : DiffPair_float_0 = (*dpy_1);
    if((((*dpy_1).primal_0) < (_S16.primal_0)))
    {
        _S17 = dOut_2;
    }
    else
    {
        if((((*dpy_1).primal_0) > ((*dpx_2).primal_0)))
        {
            _S17 = 0.0f;
        }
        else
        {
            _S17 = 0.5f * dOut_2;
        }
    }
    (*dpy_1).primal_0 = _S18.primal_0;
    (*dpy_1).differential_0 = _S17;
    return;
}

fn s_bwdProp_min_0( ctx_2 : s_bwdCallableCtx_min_0,  d_x_2 : ptr<function, f32>,  d_y_1 : ptr<function, f32>,  d_out_2 : f32)
{
    var _S19 : DiffPair_float_0;
    _S19.primal_0 = ctx_2._S14;
    var _S20 : DiffPair_float_0;
    _S20.primal_0 = ctx_2._S15;
    _d_min_0(&(_S19), &(_S20), d_out_2);
    (*d_x_2) = _S19.differential_0;
    (*d_y_1) = _S20.differential_0;
    return;
}

struct Tuple_3
{
     value0_3 : vec3<f32>,
};

fn hsv_in_sector_0( hsv_0 : vec3<f32>,  sector_0 : f32,  base_0 : f32) -> vec3<f32>
{
    var _S21 : f32 = hsv_0.z;
    var c_0 : f32 = hsv_0.y * _S21;
    var m_0 : f32 = _S21 - c_0;
    var fc_0 : f32 = (hsv_0.x * 6.0f - base_0) * c_0;
    var top_0 : f32 = c_0 + m_0;
    var rising_0 : f32 = fc_0 + m_0;
    var falling_0 : f32 = top_0 - fc_0;
    var _S22 : vec3<f32> = vec3<f32>(top_0, m_0, falling_0);
    var rgb_0 : vec3<f32>;
    if(sector_0 == 0.0f)
    {
        rgb_0 = vec3<f32>(top_0, rising_0, m_0);
    }
    else
    {
        if(sector_0 == 1.0f)
        {
            rgb_0 = vec3<f32>(falling_0, top_0, m_0);
        }
        else
        {
            if(sector_0 == 2.0f)
            {
                rgb_0 = vec3<f32>(m_0, top_0, rising_0);
            }
            else
            {
                if(sector_0 == 3.0f)
                {
                    rgb_0 = vec3<f32>(m_0, falling_0, top_0);
                }
                else
                {
                    if(sector_0 == 4.0f)
                    {
                        rgb_0 = vec3<f32>(rising_0, m_0, top_0);
                    }
                    else
                    {
                        rgb_0 = _S22;
                    }
                }
            }
        }
    }
    return rgb_0;
}

fn hsv_to_rgb_0( hsv_1 : vec3<f32>) -> vec3<f32>
{
    var base_1 : f32 = floor(hsv_1.x * 6.0f);
    var _S23 : f32;
    if((base_1 >= 6.0f))
    {
        _S23 = 0.0f;
    }
    else
    {
        _S23 = base_1;
    }
    return hsv_in_sector_0(hsv_1, _S23, base_1);
}

fn s_apply_hsv_to_rgb_0( hsv_2 : vec3<f32>) -> Tuple_3
{
    var _S24 : Tuple_3 = Tuple_3( hsv_to_rgb_0(hsv_2) );
    return _S24;
}

struct s_bwdCallableCtx_hsv_to_rgb_0
{
     _S25 : vec3<f32>,
};

struct DiffPair_vectorx3Cfloatx2C3x3E_0
{
     primal_0 : vec3<f32>,
     differential_0 : vec3<f32>,
};

fn hsv_sides_0( h_0 : f32,  right_0 : ptr<function, f32>,  right_base_0 : ptr<function, f32>,  left_0 : ptr<function, f32>,  left_base_0 : ptr<function, f32>)
{
    var nearest_0 : f32 = round(h_0);
    var boundary_0 : bool = ((abs(h_0 - nearest_0)) < 9.9999997473787516e-06f);
    var _S26 : f32;
    if(boundary_0)
    {
        _S26 = nearest_0;
    }
    else
    {
        _S26 = floor(h_0);
    }
    (*right_base_0) = _S26;
    if(boundary_0)
    {
        _S26 = nearest_0 - 1.0f;
    }
    else
    {
        _S26 = (*right_base_0);
    }
    (*left_base_0) = _S26;
    (*right_0) = (*right_base_0) - 6.0f * floor((*right_base_0) / 6.0f);
    (*left_0) = (*left_base_0) - 6.0f * floor((*left_base_0) / 6.0f);
    return;
}

struct s_paramCtx_s_bwdCallableCtx_hsv_in_sector_0
{
     value0_4 : vec3<f32>,
     value1_0 : f32,
     value2_0 : f32,
};

struct s_bwdCallableCtx_hsv_in_sector_0
{
     value0_5 : s_paramCtx_s_bwdCallableCtx_hsv_in_sector_0,
};

fn s_bwdProp_hsv_in_sector_0( _S27 : s_bwdCallableCtx_hsv_in_sector_0,  _S28 : ptr<function, vec3<f32>>,  _S29 : vec3<f32>)
{
    var _S30 : f32 = _S27.value0_5.value0_4.y;
    var _S31 : f32 = _S27.value0_5.value0_4.z;
    var c_1 : f32 = _S30 * _S31;
    var _S32 : f32 = _S27.value0_5.value0_4.x * 6.0f - _S27.value0_5.value2_0;
    var _S33 : bool = (_S27.value0_5.value1_0) == 0.0f;
    var _S34 : bool;
    var _S35 : bool;
    var _S36 : bool;
    var _S37 : bool;
    if(_S33)
    {
        _S34 = false;
        _S35 = false;
        _S36 = false;
        _S37 = false;
    }
    else
    {
        var _S38 : bool = (_S27.value0_5.value1_0) == 1.0f;
        if(_S38)
        {
            _S34 = false;
            _S35 = false;
            _S36 = false;
        }
        else
        {
            var _S39 : bool = (_S27.value0_5.value1_0) == 2.0f;
            if(_S39)
            {
                _S34 = false;
                _S35 = false;
            }
            else
            {
                var _S40 : bool = (_S27.value0_5.value1_0) == 3.0f;
                if(_S40)
                {
                    _S34 = false;
                }
                else
                {
                    _S34 = (_S27.value0_5.value1_0) == 4.0f;
                }
                var _S41 : bool = _S34;
                _S34 = _S40;
                _S35 = _S41;
            }
            var _S42 : bool = _S34;
            var _S43 : bool = _S35;
            _S34 = _S39;
            _S35 = _S42;
            _S36 = _S43;
        }
        var _S44 : bool = _S34;
        var _S45 : bool = _S35;
        var _S46 : bool = _S36;
        _S34 = _S38;
        _S35 = _S44;
        _S36 = _S45;
        _S37 = _S46;
    }
    var _S47 : vec3<f32> = vec3<f32>(0.0f);
    var _S48 : vec3<f32>;
    var _S49 : f32;
    var _S50 : f32;
    var _S51 : f32;
    var _S52 : f32;
    if(_S33)
    {
        _S48 = _S47;
        _S49 = 0.0f;
        _S50 = _S29[i32(1)];
        _S51 = _S29[i32(0)];
        _S52 = _S29[i32(2)];
    }
    else
    {
        if(_S34)
        {
            _S48 = _S47;
            _S49 = _S29[i32(0)];
            _S50 = 0.0f;
            _S51 = _S29[i32(1)];
            _S52 = _S29[i32(2)];
        }
        else
        {
            if(_S35)
            {
                _S48 = _S47;
                _S49 = 0.0f;
                _S50 = _S29[i32(2)];
                _S51 = _S29[i32(1)];
                _S52 = _S29[i32(0)];
            }
            else
            {
                if(_S36)
                {
                    _S48 = _S47;
                    _S49 = _S29[i32(1)];
                    _S50 = 0.0f;
                    _S51 = _S29[i32(2)];
                    _S52 = _S29[i32(0)];
                }
                else
                {
                    if(_S37)
                    {
                        _S48 = _S47;
                        _S49 = _S29[i32(0)];
                        _S50 = _S29[i32(2)];
                        _S51 = _S29[i32(1)];
                    }
                    else
                    {
                        _S48 = _S29;
                        _S49 = 0.0f;
                        _S50 = 0.0f;
                        _S51 = 0.0f;
                    }
                    var _S53 : f32 = _S49;
                    var _S54 : f32 = _S50;
                    var _S55 : f32 = _S51;
                    _S49 = 0.0f;
                    _S50 = _S53;
                    _S51 = _S54;
                    _S52 = _S55;
                }
            }
        }
    }
    var _S56 : f32 = _S48[i32(2)] + _S49;
    var _S57 : f32 = _S48[i32(0)] + _S56 + _S51;
    var _S58 : f32 = - _S56 + _S50;
    var _S59 : f32 = _S48[i32(1)] + _S50 + _S57 + _S52;
    var _S60 : f32 = _S57 + _S32 * _S58 + - _S59;
    (*_S28) = vec3<f32>(6.0f * (c_1 * _S58), _S31 * _S60, _S59 + _S30 * _S60);
    return;
}

fn _S61( _S62 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  _S63 : f32,  _S64 : f32,  _S65 : vec3<f32>)
{
    var _S66 : s_paramCtx_s_bwdCallableCtx_hsv_in_sector_0;
    _S66.value0_4 = (*_S62).primal_0;
    _S66.value1_0 = _S63;
    _S66.value2_0 = _S64;
    var _S67 : s_bwdCallableCtx_hsv_in_sector_0 = s_bwdCallableCtx_hsv_in_sector_0( _S66 );
    var _S68 : vec3<f32> = (*_S62).differential_0;
    s_bwdProp_hsv_in_sector_0(_S67, &(_S68), _S65);
    (*_S62).differential_0 = _S68;
    return;
}

fn hsv_to_rgb_backward_0( hsv_3 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  d_out_3 : vec3<f32>)
{
    var right_1 : f32;
    var right_base_1 : f32;
    var left_1 : f32;
    var left_base_1 : f32;
    hsv_sides_0((*hsv_3).primal_0.x * 6.0f, &(right_1), &(right_base_1), &(left_1), &(left_base_1));
    var _S69 : vec3<f32> = vec3<f32>(0.0f);
    var on_right_0 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    on_right_0.primal_0 = (*hsv_3).primal_0;
    on_right_0.differential_0 = _S69;
    var on_left_0 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    on_left_0.primal_0 = (*hsv_3).primal_0;
    on_left_0.differential_0 = _S69;
    var _S70 : vec3<f32> = vec3<f32>(0.5f) * d_out_3;
    _S61(&(on_right_0), right_1, right_base_1, _S70);
    _S61(&(on_left_0), left_1, left_base_1, _S70);
    var _S71 : vec3<f32> = on_right_0.differential_0 + on_left_0.differential_0;
    (*hsv_3).primal_0 = (*hsv_3).primal_0;
    (*hsv_3).differential_0 = _S71;
    return;
}

fn s_bwdProp_hsv_to_rgb_0( ctx_3 : s_bwdCallableCtx_hsv_to_rgb_0,  d_hsv_0 : ptr<function, vec3<f32>>,  d_out_4 : vec3<f32>)
{
    var _S72 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S72.primal_0 = ctx_3._S25;
    hsv_to_rgb_backward_0(&(_S72), d_out_4);
    (*d_hsv_0) = _S72.differential_0;
    return;
}

struct Tuple_4
{
     value0_6 : f32,
};

fn pow0_0( x_3 : f32,  y_2 : f32) -> f32
{
    if((x_3 <= 0.0f))
    {
        return 0.0f;
    }
    return pow(x_3, y_2);
}

fn s_apply_pow0_0( x_4 : f32,  y_3 : f32) -> Tuple_4
{
    var _S73 : Tuple_4 = Tuple_4( pow0_0(x_4, y_3) );
    return _S73;
}

struct s_bwdCallableCtx_pow0_0
{
     _S74 : f32,
     _S75 : f32,
};

fn pow0_backward_0( x_5 : ptr<function, DiffPair_float_0>,  y_4 : ptr<function, DiffPair_float_0>,  d_out_5 : f32)
{
    if((((*x_5).primal_0) <= 0.0f))
    {
        (*x_5).primal_0 = (*x_5).primal_0;
        (*x_5).differential_0 = 0.0f;
        (*y_4).primal_0 = (*y_4).primal_0;
        (*y_4).differential_0 = 0.0f;
        return;
    }
    var d_x_3 : f32 = d_out_5 * (*y_4).primal_0 * pow((*x_5).primal_0, (*y_4).primal_0 - 1.0f);
    var d_y_2 : f32 = d_out_5 * pow((*x_5).primal_0, (*y_4).primal_0) * log((*x_5).primal_0);
    (*x_5).primal_0 = (*x_5).primal_0;
    (*x_5).differential_0 = d_x_3;
    (*y_4).primal_0 = (*y_4).primal_0;
    (*y_4).differential_0 = d_y_2;
    return;
}

fn s_bwdProp_pow0_0( ctx_4 : s_bwdCallableCtx_pow0_0,  d_x_4 : ptr<function, f32>,  d_y_3 : ptr<function, f32>,  d_out_6 : f32)
{
    var _S76 : DiffPair_float_0;
    _S76.primal_0 = ctx_4._S74;
    var _S77 : DiffPair_float_0;
    _S77.primal_0 = ctx_4._S75;
    pow0_backward_0(&(_S76), &(_S77), d_out_6);
    (*d_x_4) = _S76.differential_0;
    (*d_y_3) = _S77.differential_0;
    return;
}

struct Tuple_5
{
     value0_7 : f32,
};

fn s_apply_dot_0( x_6 : vec3<f32>,  y_5 : vec3<f32>) -> Tuple_5
{
    var _S78 : Tuple_5 = Tuple_5( dot(x_6, y_5) );
    return _S78;
}

struct s_bwdCallableCtx_dot_0
{
     _S79 : vec3<f32>,
     _S80 : vec3<f32>,
};

fn _d_dot_0( dpx_3 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dpy_2 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_3 : f32)
{
    var x_d_result_0 : vec3<f32>;
    x_d_result_0[i32(0)] = (*dpy_2).primal_0[i32(0)] * dOut_3;
    var y_d_result_0 : vec3<f32>;
    y_d_result_0[i32(0)] = (*dpx_3).primal_0[i32(0)] * dOut_3;
    x_d_result_0[i32(1)] = (*dpy_2).primal_0[i32(1)] * dOut_3;
    y_d_result_0[i32(1)] = (*dpx_3).primal_0[i32(1)] * dOut_3;
    x_d_result_0[i32(2)] = (*dpy_2).primal_0[i32(2)] * dOut_3;
    y_d_result_0[i32(2)] = (*dpx_3).primal_0[i32(2)] * dOut_3;
    (*dpx_3).primal_0 = (*dpx_3).primal_0;
    (*dpx_3).differential_0 = x_d_result_0;
    (*dpy_2).primal_0 = (*dpy_2).primal_0;
    (*dpy_2).differential_0 = y_d_result_0;
    return;
}

fn s_bwdProp_dot_0( ctx_5 : s_bwdCallableCtx_dot_0,  d_x_5 : ptr<function, vec3<f32>>,  d_y_4 : ptr<function, vec3<f32>>,  d_out_7 : f32)
{
    var _S81 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S81.primal_0 = ctx_5._S79;
    var _S82 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S82.primal_0 = ctx_5._S80;
    _d_dot_0(&(_S81), &(_S82), d_out_7);
    (*d_x_5) = _S81.differential_0;
    (*d_y_4) = _S82.differential_0;
    return;
}

struct Tuple_6
{
     value0_8 : vec3<f32>,
};

fn s_apply_min_1( x_7 : vec3<f32>,  y_6 : vec3<f32>) -> Tuple_6
{
    var _S83 : Tuple_6 = Tuple_6( min(x_7, y_6) );
    return _S83;
}

struct s_bwdCallableCtx_min_1
{
     _S84 : vec3<f32>,
     _S85 : vec3<f32>,
};

fn _d_min_vector_0( dpx_4 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dpy_3 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_4 : vec3<f32>)
{
    var left_dp_0 : DiffPair_float_0;
    left_dp_0.primal_0 = (*dpx_4).primal_0[i32(0)];
    left_dp_0.differential_0 = 0.0f;
    var right_dp_0 : DiffPair_float_0;
    right_dp_0.primal_0 = (*dpy_3).primal_0[i32(0)];
    right_dp_0.differential_0 = 0.0f;
    _d_min_0(&(left_dp_0), &(right_dp_0), dOut_4[i32(0)]);
    var left_d_result_0 : vec3<f32>;
    left_d_result_0[i32(0)] = left_dp_0.differential_0;
    var right_d_result_0 : vec3<f32>;
    right_d_result_0[i32(0)] = right_dp_0.differential_0;
    var left_dp_1 : DiffPair_float_0;
    left_dp_1.primal_0 = (*dpx_4).primal_0[i32(1)];
    left_dp_1.differential_0 = 0.0f;
    var right_dp_1 : DiffPair_float_0;
    right_dp_1.primal_0 = (*dpy_3).primal_0[i32(1)];
    right_dp_1.differential_0 = 0.0f;
    _d_min_0(&(left_dp_1), &(right_dp_1), dOut_4[i32(1)]);
    left_d_result_0[i32(1)] = left_dp_1.differential_0;
    right_d_result_0[i32(1)] = right_dp_1.differential_0;
    var left_dp_2 : DiffPair_float_0;
    left_dp_2.primal_0 = (*dpx_4).primal_0[i32(2)];
    left_dp_2.differential_0 = 0.0f;
    var right_dp_2 : DiffPair_float_0;
    right_dp_2.primal_0 = (*dpy_3).primal_0[i32(2)];
    right_dp_2.differential_0 = 0.0f;
    _d_min_0(&(left_dp_2), &(right_dp_2), dOut_4[i32(2)]);
    left_d_result_0[i32(2)] = left_dp_2.differential_0;
    right_d_result_0[i32(2)] = right_dp_2.differential_0;
    (*dpx_4).primal_0 = (*dpx_4).primal_0;
    (*dpx_4).differential_0 = left_d_result_0;
    (*dpy_3).primal_0 = (*dpy_3).primal_0;
    (*dpy_3).differential_0 = right_d_result_0;
    return;
}

fn s_bwdProp_min_1( ctx_6 : s_bwdCallableCtx_min_1,  d_x_6 : ptr<function, vec3<f32>>,  d_y_5 : ptr<function, vec3<f32>>,  d_out_8 : vec3<f32>)
{
    var _S86 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S86.primal_0 = ctx_6._S84;
    var _S87 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S87.primal_0 = ctx_6._S85;
    _d_min_vector_0(&(_S86), &(_S87), d_out_8);
    (*d_x_6) = _S86.differential_0;
    (*d_y_5) = _S87.differential_0;
    return;
}

struct s_bwdCallableCtx_mul_0
{
     _S88 : mat3x3<f32>,
     _S89 : vec3<f32>,
};

struct DiffPair_matrixx3Cfloatx2C3x2C3x3E_0
{
     primal_0 : mat3x3<f32>,
     differential_0 : mat3x3<f32>,
};

fn _d_mul_0( left_2 : ptr<function, DiffPair_matrixx3Cfloatx2C3x2C3x3E_0>,  right_2 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_5 : vec3<f32>)
{
    var _S90 : f32 = (*left_2).primal_0[i32(0)][i32(0)] * dOut_5[i32(0)];
    var left_d_result_1 : mat3x3<f32>;
    left_d_result_1[i32(0)][i32(0)] = (*right_2).primal_0[i32(0)] * dOut_5[i32(0)];
    var sum_0 : f32 = _S90 + (*left_2).primal_0[i32(1)][i32(0)] * dOut_5[i32(1)];
    left_d_result_1[i32(1)][i32(0)] = (*right_2).primal_0[i32(0)] * dOut_5[i32(1)];
    var sum_1 : f32 = sum_0 + (*left_2).primal_0[i32(2)][i32(0)] * dOut_5[i32(2)];
    left_d_result_1[i32(2)][i32(0)] = (*right_2).primal_0[i32(0)] * dOut_5[i32(2)];
    var right_d_result_1 : vec3<f32>;
    right_d_result_1[i32(0)] = sum_1;
    var _S91 : f32 = (*left_2).primal_0[i32(0)][i32(1)] * dOut_5[i32(0)];
    left_d_result_1[i32(0)][i32(1)] = (*right_2).primal_0[i32(1)] * dOut_5[i32(0)];
    var sum_2 : f32 = _S91 + (*left_2).primal_0[i32(1)][i32(1)] * dOut_5[i32(1)];
    left_d_result_1[i32(1)][i32(1)] = (*right_2).primal_0[i32(1)] * dOut_5[i32(1)];
    var sum_3 : f32 = sum_2 + (*left_2).primal_0[i32(2)][i32(1)] * dOut_5[i32(2)];
    left_d_result_1[i32(2)][i32(1)] = (*right_2).primal_0[i32(1)] * dOut_5[i32(2)];
    right_d_result_1[i32(1)] = sum_3;
    var _S92 : f32 = (*left_2).primal_0[i32(0)][i32(2)] * dOut_5[i32(0)];
    left_d_result_1[i32(0)][i32(2)] = (*right_2).primal_0[i32(2)] * dOut_5[i32(0)];
    var sum_4 : f32 = _S92 + (*left_2).primal_0[i32(1)][i32(2)] * dOut_5[i32(1)];
    left_d_result_1[i32(1)][i32(2)] = (*right_2).primal_0[i32(2)] * dOut_5[i32(1)];
    var sum_5 : f32 = sum_4 + (*left_2).primal_0[i32(2)][i32(2)] * dOut_5[i32(2)];
    left_d_result_1[i32(2)][i32(2)] = (*right_2).primal_0[i32(2)] * dOut_5[i32(2)];
    right_d_result_1[i32(2)] = sum_5;
    (*left_2).primal_0 = (*left_2).primal_0;
    (*left_2).differential_0 = left_d_result_1;
    (*right_2).primal_0 = (*right_2).primal_0;
    (*right_2).differential_0 = right_d_result_1;
    return;
}

fn s_bwdProp_mul_0( ctx_7 : s_bwdCallableCtx_mul_0,  d_left_0 : ptr<function, mat3x3<f32>>,  d_right_0 : ptr<function, vec3<f32>>,  d_out_9 : vec3<f32>)
{
    var _S93 : DiffPair_matrixx3Cfloatx2C3x2C3x3E_0;
    _S93.primal_0 = ctx_7._S88;
    var _S94 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S94.primal_0 = ctx_7._S89;
    _d_mul_0(&(_S93), &(_S94), d_out_9);
    (*d_left_0) = _S93.differential_0;
    (*d_right_0) = _S94.differential_0;
    return;
}

struct Tuple_7
{
     value0_9 : vec3<f32>,
};

fn s_apply_max_1( x_8 : vec3<f32>,  y_7 : vec3<f32>) -> Tuple_7
{
    var _S95 : Tuple_7 = Tuple_7( max(x_8, y_7) );
    return _S95;
}

struct s_bwdCallableCtx_max_1
{
     _S96 : vec3<f32>,
     _S97 : vec3<f32>,
};

fn _d_max_vector_0( dpx_5 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dpy_4 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_6 : vec3<f32>)
{
    var left_dp_3 : DiffPair_float_0;
    left_dp_3.primal_0 = (*dpx_5).primal_0[i32(0)];
    left_dp_3.differential_0 = 0.0f;
    var right_dp_3 : DiffPair_float_0;
    right_dp_3.primal_0 = (*dpy_4).primal_0[i32(0)];
    right_dp_3.differential_0 = 0.0f;
    _d_max_0(&(left_dp_3), &(right_dp_3), dOut_6[i32(0)]);
    var left_d_result_2 : vec3<f32>;
    left_d_result_2[i32(0)] = left_dp_3.differential_0;
    var right_d_result_2 : vec3<f32>;
    right_d_result_2[i32(0)] = right_dp_3.differential_0;
    var left_dp_4 : DiffPair_float_0;
    left_dp_4.primal_0 = (*dpx_5).primal_0[i32(1)];
    left_dp_4.differential_0 = 0.0f;
    var right_dp_4 : DiffPair_float_0;
    right_dp_4.primal_0 = (*dpy_4).primal_0[i32(1)];
    right_dp_4.differential_0 = 0.0f;
    _d_max_0(&(left_dp_4), &(right_dp_4), dOut_6[i32(1)]);
    left_d_result_2[i32(1)] = left_dp_4.differential_0;
    right_d_result_2[i32(1)] = right_dp_4.differential_0;
    var left_dp_5 : DiffPair_float_0;
    left_dp_5.primal_0 = (*dpx_5).primal_0[i32(2)];
    left_dp_5.differential_0 = 0.0f;
    var right_dp_5 : DiffPair_float_0;
    right_dp_5.primal_0 = (*dpy_4).primal_0[i32(2)];
    right_dp_5.differential_0 = 0.0f;
    _d_max_0(&(left_dp_5), &(right_dp_5), dOut_6[i32(2)]);
    left_d_result_2[i32(2)] = left_dp_5.differential_0;
    right_d_result_2[i32(2)] = right_dp_5.differential_0;
    (*dpx_5).primal_0 = (*dpx_5).primal_0;
    (*dpx_5).differential_0 = left_d_result_2;
    (*dpy_4).primal_0 = (*dpy_4).primal_0;
    (*dpy_4).differential_0 = right_d_result_2;
    return;
}

fn s_bwdProp_max_1( ctx_8 : s_bwdCallableCtx_max_1,  d_x_7 : ptr<function, vec3<f32>>,  d_y_6 : ptr<function, vec3<f32>>,  d_out_10 : vec3<f32>)
{
    var _S98 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S98.primal_0 = ctx_8._S96;
    var _S99 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S99.primal_0 = ctx_8._S97;
    _d_max_vector_0(&(_S98), &(_S99), d_out_10);
    (*d_x_7) = _S98.differential_0;
    (*d_y_6) = _S99.differential_0;
    return;
}

fn _d_min_1( dpx_6 : DiffPair_float_0,  dpy_5 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S100 : f32 = min(dpx_6.primal_0, dpy_5.primal_0);
    var _S101 : f32;
    if(((dpx_6.primal_0) < (dpy_5.primal_0)))
    {
        _S101 = dpx_6.differential_0;
    }
    else
    {
        if(((dpx_6.primal_0) > (dpy_5.primal_0)))
        {
            _S101 = dpy_5.differential_0;
        }
        else
        {
            _S101 = 0.5f * (dpx_6.differential_0 + dpy_5.differential_0);
        }
    }
    var _S102 : DiffPair_float_0 = DiffPair_float_0( _S100, _S101 );
    return _S102;
}

fn _d_min_vector_1( dpx_7 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpy_6 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S103 : DiffPair_float_0 = DiffPair_float_0( dpx_7.primal_0[i32(0)], dpx_7.differential_0[i32(0)] );
    var _S104 : DiffPair_float_0 = DiffPair_float_0( dpy_6.primal_0[i32(0)], dpy_6.differential_0[i32(0)] );
    var dp_elem_0 : DiffPair_float_0 = _d_min_1(_S103, _S104);
    var result_0 : vec3<f32>;
    result_0[i32(0)] = dp_elem_0.primal_0;
    var d_result_0 : vec3<f32>;
    d_result_0[i32(0)] = dp_elem_0.differential_0;
    var _S105 : DiffPair_float_0 = DiffPair_float_0( dpx_7.primal_0[i32(1)], dpx_7.differential_0[i32(1)] );
    var _S106 : DiffPair_float_0 = DiffPair_float_0( dpy_6.primal_0[i32(1)], dpy_6.differential_0[i32(1)] );
    var dp_elem_1 : DiffPair_float_0 = _d_min_1(_S105, _S106);
    result_0[i32(1)] = dp_elem_1.primal_0;
    d_result_0[i32(1)] = dp_elem_1.differential_0;
    var _S107 : DiffPair_float_0 = DiffPair_float_0( dpx_7.primal_0[i32(2)], dpx_7.differential_0[i32(2)] );
    var _S108 : DiffPair_float_0 = DiffPair_float_0( dpy_6.primal_0[i32(2)], dpy_6.differential_0[i32(2)] );
    var dp_elem_2 : DiffPair_float_0 = _d_min_1(_S107, _S108);
    result_0[i32(2)] = dp_elem_2.primal_0;
    d_result_0[i32(2)] = dp_elem_2.differential_0;
    var _S109 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( result_0, d_result_0 );
    return _S109;
}

fn _d_max_1( dpx_8 : DiffPair_float_0,  dpy_7 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S110 : f32 = max(dpx_8.primal_0, dpy_7.primal_0);
    var _S111 : f32;
    if(((dpx_8.primal_0) > (dpy_7.primal_0)))
    {
        _S111 = dpx_8.differential_0;
    }
    else
    {
        if(((dpx_8.primal_0) < (dpy_7.primal_0)))
        {
            _S111 = dpy_7.differential_0;
        }
        else
        {
            _S111 = 0.5f * (dpx_8.differential_0 + dpy_7.differential_0);
        }
    }
    var _S112 : DiffPair_float_0 = DiffPair_float_0( _S110, _S111 );
    return _S112;
}

fn _d_max_vector_1( dpx_9 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpy_8 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S113 : DiffPair_float_0 = DiffPair_float_0( dpx_9.primal_0[i32(0)], dpx_9.differential_0[i32(0)] );
    var _S114 : DiffPair_float_0 = DiffPair_float_0( dpy_8.primal_0[i32(0)], dpy_8.differential_0[i32(0)] );
    var dp_elem_3 : DiffPair_float_0 = _d_max_1(_S113, _S114);
    var result_1 : vec3<f32>;
    result_1[i32(0)] = dp_elem_3.primal_0;
    var d_result_1 : vec3<f32>;
    d_result_1[i32(0)] = dp_elem_3.differential_0;
    var _S115 : DiffPair_float_0 = DiffPair_float_0( dpx_9.primal_0[i32(1)], dpx_9.differential_0[i32(1)] );
    var _S116 : DiffPair_float_0 = DiffPair_float_0( dpy_8.primal_0[i32(1)], dpy_8.differential_0[i32(1)] );
    var dp_elem_4 : DiffPair_float_0 = _d_max_1(_S115, _S116);
    result_1[i32(1)] = dp_elem_4.primal_0;
    d_result_1[i32(1)] = dp_elem_4.differential_0;
    var _S117 : DiffPair_float_0 = DiffPair_float_0( dpx_9.primal_0[i32(2)], dpx_9.differential_0[i32(2)] );
    var _S118 : DiffPair_float_0 = DiffPair_float_0( dpy_8.primal_0[i32(2)], dpy_8.differential_0[i32(2)] );
    var dp_elem_5 : DiffPair_float_0 = _d_max_1(_S117, _S118);
    result_1[i32(2)] = dp_elem_5.primal_0;
    d_result_1[i32(2)] = dp_elem_5.differential_0;
    var _S119 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( result_1, d_result_1 );
    return _S119;
}

fn _d_abs_1( dpx_10 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S120 : DiffPair_float_0 = DiffPair_float_0( abs(dpx_10.primal_0), select(select(-1.0f, 0.0f, (dpx_10.primal_0) == 0.0f), 1.0f, ((dpx_10.primal_0) > 0.0f)) * dpx_10.differential_0 );
    return _S120;
}

fn _d_dot_1( dpx_11 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpy_9 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_float_0
{
    var _S121 : DiffPair_float_0 = DiffPair_float_0( dpx_11.primal_0[i32(0)] * dpy_9.primal_0[i32(0)] + dpx_11.primal_0[i32(1)] * dpy_9.primal_0[i32(1)] + dpx_11.primal_0[i32(2)] * dpy_9.primal_0[i32(2)], dpx_11.primal_0[i32(0)] * dpy_9.differential_0[i32(0)] + dpy_9.primal_0[i32(0)] * dpx_11.differential_0[i32(0)] + dpx_11.primal_0[i32(1)] * dpy_9.differential_0[i32(1)] + dpy_9.primal_0[i32(1)] * dpx_11.differential_0[i32(1)] + dpx_11.primal_0[i32(2)] * dpy_9.differential_0[i32(2)] + dpy_9.primal_0[i32(2)] * dpx_11.differential_0[i32(2)] );
    return _S121;
}

struct AgxLook_0
{
     slope_1 : f32,
     lift_0 : f32,
     power_0 : f32,
     saturation_0 : f32,
     hue_mix_0 : f32,
};

fn AgxLook_x24_syn_dadd_0( SLANG_anonymous_0_0 : AgxLook_0,  SLANG_anonymous_1_0 : AgxLook_0) -> AgxLook_0
{
    var result_2 : AgxLook_0;
    result_2.slope_1 = SLANG_anonymous_0_0.slope_1 + SLANG_anonymous_1_0.slope_1;
    result_2.lift_0 = SLANG_anonymous_0_0.lift_0 + SLANG_anonymous_1_0.lift_0;
    result_2.power_0 = SLANG_anonymous_0_0.power_0 + SLANG_anonymous_1_0.power_0;
    result_2.saturation_0 = SLANG_anonymous_0_0.saturation_0 + SLANG_anonymous_1_0.saturation_0;
    result_2.hue_mix_0 = SLANG_anonymous_0_0.hue_mix_0 + SLANG_anonymous_1_0.hue_mix_0;
    return result_2;
}

fn AgxLook_x24_syn_dzero_0() -> AgxLook_0
{
    var result_3 : AgxLook_0;
    result_3.slope_1 = 0.0f;
    result_3.lift_0 = 0.0f;
    result_3.power_0 = 0.0f;
    result_3.saturation_0 = 0.0f;
    result_3.hue_mix_0 = 0.0f;
    return result_3;
}

fn compress_into_gamut_0( rgb_1 : vec3<f32>) -> vec3<f32>
{
    const weights_0 : vec3<f32> = vec3<f32>(0.2658180296421051f, 0.59846985340118408f, 0.13571210205554962f);
    var _S122 : f32 = rgb_1.x;
    var _S123 : f32 = rgb_1.y;
    var _S124 : f32 = rgb_1.z;
    var opponent_0 : vec3<f32> = vec3<f32>(max(_S122, max(_S123, _S124))) - rgb_1;
    var y_compensate_negative_0 : f32 = max(opponent_0.x, max(opponent_0.y, opponent_0.z)) - dot(opponent_0, weights_0) + dot(rgb_1, weights_0);
    var rgb_offset_0 : vec3<f32> = rgb_1 + vec3<f32>(max(- min(_S122, min(_S123, _S124)), 0.0f));
    var opponent_offset_0 : vec3<f32> = vec3<f32>(max(rgb_offset_0.x, max(rgb_offset_0.y, rgb_offset_0.z))) - rgb_offset_0;
    var y_new_0 : f32 = max(opponent_offset_0.x, max(opponent_offset_0.y, opponent_offset_0.z)) - dot(opponent_offset_0, weights_0) + dot(rgb_offset_0, weights_0);
    var _S125 : bool;
    if((y_new_0 > y_compensate_negative_0))
    {
        _S125 = (y_new_0 > 9.99999997475242708e-07f);
    }
    else
    {
        _S125 = false;
    }
    var ratio_0 : f32;
    if(_S125)
    {
        ratio_0 = y_compensate_negative_0 / y_new_0;
    }
    else
    {
        ratio_0 = 1.0f;
    }
    return vec3<f32>(ratio_0) * rgb_offset_0;
}

fn rgb_to_hsv_0( rgb_2 : vec3<f32>) -> vec3<f32>
{
    var _S126 : f32 = rgb_2.x;
    var _S127 : f32 = rgb_2.y;
    var _S128 : f32 = rgb_2.z;
    var _S129 : f32 = max(_S126, max(_S127, _S128));
    var delta_0 : f32 = _S129 - min(_S126, min(_S127, _S128));
    var chromatic_0 : bool;
    if(((abs(_S129)) > 9.99999997475242708e-07f))
    {
        chromatic_0 = ((abs(delta_0)) > 9.99999997475242708e-07f);
    }
    else
    {
        chromatic_0 = false;
    }
    var safe_delta_0 : f32;
    if(chromatic_0)
    {
        safe_delta_0 = delta_0;
    }
    else
    {
        safe_delta_0 = 1.0f;
    }
    var safe_hi_0 : f32;
    if(chromatic_0)
    {
        safe_hi_0 = _S129;
    }
    else
    {
        safe_hi_0 = 1.0f;
    }
    var hue_r_0 : f32 = (_S127 - _S128) / safe_delta_0;
    var hue_g_0 : f32 = 2.0f + (_S128 - _S126) / safe_delta_0;
    var hue_b_0 : f32 = 4.0f + (_S126 - _S127) / safe_delta_0;
    if(_S126 == _S129)
    {
        safe_delta_0 = hue_r_0;
    }
    else
    {
        if(_S127 == _S129)
        {
            safe_delta_0 = hue_g_0;
        }
        else
        {
            safe_delta_0 = hue_b_0;
        }
    }
    var hue_0 : f32 = safe_delta_0 / 6.0f;
    var hue_1 : f32 = hue_0 - floor(hue_0);
    if(chromatic_0)
    {
        safe_delta_0 = hue_1;
    }
    else
    {
        safe_delta_0 = 0.0f;
    }
    if(chromatic_0)
    {
        safe_hi_0 = delta_0 / safe_hi_0;
    }
    else
    {
        safe_hi_0 = 0.0f;
    }
    return vec3<f32>(safe_delta_0, safe_hi_0, _S129);
}

struct Tuple_8
{
     value0_10 : vec3<f32>,
};

fn s_apply_rgb_to_hsv_0( _S130 : vec3<f32>) -> Tuple_8
{
    var _S131 : f32 = _S130.x;
    var _S132 : f32 = _S130.y;
    var _S133 : f32 = _S130.z;
    var _S134 : Tuple_1 = s_apply_max_0(_S131, s_apply_max_0(_S132, _S133).value0_1);
    var delta_1 : f32 = _S134.value0_1 - s_apply_min_0(_S131, s_apply_min_0(_S132, _S133).value0_2).value0_2;
    var chromatic_1 : bool;
    if(((s_apply_abs_0(_S134.value0_1).value0_0) > 9.99999997475242708e-07f))
    {
        chromatic_1 = ((s_apply_abs_0(delta_1).value0_0) > 9.99999997475242708e-07f);
    }
    else
    {
        chromatic_1 = false;
    }
    var safe_delta_1 : f32;
    if(chromatic_1)
    {
        safe_delta_1 = delta_1;
    }
    else
    {
        safe_delta_1 = 1.0f;
    }
    var safe_hi_1 : f32;
    if(chromatic_1)
    {
        safe_hi_1 = _S134.value0_1;
    }
    else
    {
        safe_hi_1 = 1.0f;
    }
    var hue_r_1 : f32 = (_S132 - _S133) / safe_delta_1;
    var hue_g_1 : f32 = 2.0f + (_S133 - _S131) / safe_delta_1;
    var hue_b_1 : f32 = 4.0f + (_S131 - _S132) / safe_delta_1;
    if(_S131 == (_S134.value0_1))
    {
        safe_delta_1 = hue_r_1;
    }
    else
    {
        if(_S132 == (_S134.value0_1))
        {
            safe_delta_1 = hue_g_1;
        }
        else
        {
            safe_delta_1 = hue_b_1;
        }
    }
    var hue_2 : f32 = safe_delta_1 / 6.0f;
    var hue_3 : f32 = hue_2 - floor(hue_2);
    if(chromatic_1)
    {
        safe_delta_1 = hue_3;
    }
    else
    {
        safe_delta_1 = 0.0f;
    }
    if(chromatic_1)
    {
        safe_hi_1 = delta_1 / safe_hi_1;
    }
    else
    {
        safe_hi_1 = 0.0f;
    }
    var _S135 : Tuple_8;
    _S135.value0_10 = vec3<f32>(safe_delta_1, safe_hi_1, _S134.value0_1);
    return _S135;
}

struct s_paramCtx_s_bwdCallableCtx_rgb_to_hsv_0
{
     value0_11 : vec3<f32>,
};

struct s_bwdCallableCtx_rgb_to_hsv_0
{
     value0_12 : s_paramCtx_s_bwdCallableCtx_rgb_to_hsv_0,
};

fn s_bwdProp_rgb_to_hsv_0( _S136 : s_bwdCallableCtx_rgb_to_hsv_0,  _S137 : ptr<function, vec3<f32>>,  _S138 : vec3<f32>)
{
    var _S139 : f32 = _S136.value0_12.value0_11.x;
    var _S140 : f32 = _S136.value0_12.value0_11.y;
    var _S141 : f32 = _S136.value0_12.value0_11.z;
    var _S142 : Tuple_2 = s_apply_min_0(_S140, _S141);
    var _S143 : s_bwdCallableCtx_min_0;
    _S143._S14 = _S140;
    _S143._S15 = _S141;
    var _S144 : s_bwdCallableCtx_min_0 = _S143;
    var _S145 : Tuple_2 = s_apply_min_0(_S139, _S142.value0_2);
    var _S146 : s_bwdCallableCtx_min_0;
    _S146._S14 = _S139;
    _S146._S15 = _S142.value0_2;
    var _S147 : s_bwdCallableCtx_min_0 = _S146;
    var _S148 : Tuple_1 = s_apply_max_0(_S140, _S141);
    var _S149 : s_bwdCallableCtx_max_0;
    _S149._S6 = _S140;
    _S149._S7 = _S141;
    var _S150 : s_bwdCallableCtx_max_0 = _S149;
    var _S151 : Tuple_1 = s_apply_max_0(_S139, _S148.value0_1);
    var _S152 : s_bwdCallableCtx_max_0;
    _S152._S6 = _S139;
    _S152._S7 = _S148.value0_1;
    var _S153 : s_bwdCallableCtx_max_0 = _S152;
    var delta_2 : f32 = _S151.value0_1 - _S145.value0_2;
    var _S154 : Tuple_0 = s_apply_abs_0(_S151.value0_1);
    var _S155 : s_bwdCallableCtx_abs_0;
    _S155._S2 = _S151.value0_1;
    var _S156 : s_bwdCallableCtx_abs_0 = _S155;
    var _S157 : bool = ((_S154.value0_0) > 9.99999997475242708e-07f);
    var chromatic_2 : bool;
    var _S158 : s_bwdCallableCtx_abs_0;
    if(_S157)
    {
        var _S159 : Tuple_0 = s_apply_abs_0(delta_2);
        var _S160 : s_bwdCallableCtx_abs_0;
        _S160._S2 = delta_2;
        chromatic_2 = ((_S159.value0_0) > 9.99999997475242708e-07f);
        _S158 = _S160;
    }
    else
    {
        chromatic_2 = false;
        _S158._S2 = 0.0f;
    }
    var safe_delta_2 : f32;
    if(chromatic_2)
    {
        safe_delta_2 = delta_2;
    }
    else
    {
        safe_delta_2 = 1.0f;
    }
    var safe_hi_2 : f32;
    if(chromatic_2)
    {
        safe_hi_2 = _S151.value0_1;
    }
    else
    {
        safe_hi_2 = 1.0f;
    }
    var _S161 : f32 = _S140 - _S141;
    var _S162 : f32 = safe_delta_2 * safe_delta_2;
    var _S163 : f32 = _S141 - _S139;
    var _S164 : f32 = _S139 - _S140;
    var _S165 : bool = _S139 == (_S151.value0_1);
    var _S166 : bool;
    if(_S165)
    {
        _S166 = false;
    }
    else
    {
        _S166 = _S140 == (_S151.value0_1);
    }
    var _S167 : f32;
    if(chromatic_2)
    {
        _S167 = safe_hi_2 * safe_hi_2;
    }
    else
    {
        _S167 = 0.0f;
    }
    if(chromatic_2)
    {
        var _S168 : f32 = _S138[i32(1)] / _S167;
        var _S169 : f32 = safe_hi_2 * _S168;
        safe_hi_2 = delta_2 * - _S168;
        _S167 = _S169;
    }
    else
    {
        safe_hi_2 = 0.0f;
        _S167 = 0.0f;
    }
    var _S170 : f32;
    if(chromatic_2)
    {
        _S170 = _S138[i32(0)];
    }
    else
    {
        _S170 = 0.0f;
    }
    var _S171 : f32 = 0.1666666716337204f * _S170;
    var _S172 : f32;
    var _S173 : f32;
    if(_S165)
    {
        _S170 = 0.0f;
        _S172 = 0.0f;
        _S173 = _S171;
    }
    else
    {
        if(_S166)
        {
            _S170 = 0.0f;
            _S172 = _S171;
        }
        else
        {
            _S170 = _S171;
            _S172 = 0.0f;
        }
        _S173 = 0.0f;
    }
    var _S174 : f32 = _S170 / _S162;
    var _S175 : f32 = safe_delta_2 * _S174;
    var _S176 : f32 = _S172 / _S162;
    var _S177 : f32 = safe_delta_2 * _S176;
    var _S178 : f32 = _S173 / _S162;
    var _S179 : f32 = safe_delta_2 * _S178;
    var _S180 : f32 = _S164 * - _S174 + _S163 * - _S176 + _S161 * - _S178;
    var _S181 : f32 = _S175 + - _S177;
    var _S182 : f32 = - _S175 + _S179;
    var _S183 : f32 = _S177 + - _S179;
    if(chromatic_2)
    {
        safe_delta_2 = safe_hi_2 + _S138[i32(2)];
    }
    else
    {
        safe_delta_2 = _S138[i32(2)];
    }
    if(chromatic_2)
    {
        safe_hi_2 = _S180 + _S167;
    }
    else
    {
        safe_hi_2 = _S167;
    }
    if(_S157)
    {
        var _S184 : f32 = 0.0f;
        s_bwdProp_abs_0(_S158, &(_S184), 0.0f);
        safe_hi_2 = _S184 + safe_hi_2;
    }
    var _S185 : f32 = 0.0f;
    s_bwdProp_abs_0(_S156, &(_S185), 0.0f);
    var _S186 : f32 = - safe_hi_2;
    var _S187 : f32 = _S185 + safe_hi_2 + safe_delta_2;
    var _S188 : f32 = 0.0f;
    var _S189 : f32 = 0.0f;
    s_bwdProp_max_0(_S153, &(_S188), &(_S189), _S187);
    var _S190 : f32 = 0.0f;
    var _S191 : f32 = 0.0f;
    s_bwdProp_max_0(_S150, &(_S190), &(_S191), _S189);
    var _S192 : f32 = 0.0f;
    var _S193 : f32 = 0.0f;
    s_bwdProp_min_0(_S147, &(_S192), &(_S193), _S186);
    var _S194 : f32 = 0.0f;
    var _S195 : f32 = 0.0f;
    s_bwdProp_min_0(_S144, &(_S194), &(_S195), _S193);
    (*_S137) = vec3<f32>(_S188 + _S192 + _S181, _S190 + _S194 + _S182, _S191 + _S195 + _S183);
    return;
}

fn s_fwd_rgb_to_hsv_0( dprgb_0 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S196 : f32 = dprgb_0.primal_0.x;
    var _S197 : f32 = dprgb_0.differential_0.x;
    var _S198 : f32 = dprgb_0.primal_0.y;
    var _S199 : f32 = dprgb_0.differential_0.y;
    var _S200 : f32 = dprgb_0.primal_0.z;
    var _S201 : f32 = dprgb_0.differential_0.z;
    var _S202 : DiffPair_float_0 = DiffPair_float_0( _S198, _S199 );
    var _S203 : DiffPair_float_0 = DiffPair_float_0( _S200, _S201 );
    var _S204 : DiffPair_float_0 = _d_min_1(_S202, _S203);
    var _S205 : DiffPair_float_0 = DiffPair_float_0( _S196, _S197 );
    var _S206 : DiffPair_float_0 = DiffPair_float_0( _S204.primal_0, _S204.differential_0 );
    var _S207 : DiffPair_float_0 = _d_min_1(_S205, _S206);
    var _S208 : DiffPair_float_0 = _d_max_1(_S202, _S203);
    var _S209 : DiffPair_float_0 = DiffPair_float_0( _S208.primal_0, _S208.differential_0 );
    var _S210 : DiffPair_float_0 = _d_max_1(_S205, _S209);
    var delta_3 : f32 = _S210.primal_0 - _S207.primal_0;
    var s_diff_delta_0 : f32 = _S210.differential_0 - _S207.differential_0;
    var _S211 : DiffPair_float_0 = DiffPair_float_0( _S210.primal_0, _S210.differential_0 );
    var chromatic_3 : bool;
    if(((_d_abs_1(_S211).primal_0) > 9.99999997475242708e-07f))
    {
        var _S212 : DiffPair_float_0 = DiffPair_float_0( delta_3, s_diff_delta_0 );
        chromatic_3 = ((_d_abs_1(_S212).primal_0) > 9.99999997475242708e-07f);
    }
    else
    {
        chromatic_3 = false;
    }
    var safe_delta_3 : f32;
    var s_diff_safe_delta_0 : f32;
    if(chromatic_3)
    {
        safe_delta_3 = delta_3;
        s_diff_safe_delta_0 = s_diff_delta_0;
    }
    else
    {
        safe_delta_3 = 1.0f;
        s_diff_safe_delta_0 = 0.0f;
    }
    var safe_hi_3 : f32;
    var s_diff_safe_hi_0 : f32;
    if(chromatic_3)
    {
        safe_hi_3 = _S210.primal_0;
        s_diff_safe_hi_0 = _S210.differential_0;
    }
    else
    {
        safe_hi_3 = 1.0f;
        s_diff_safe_hi_0 = 0.0f;
    }
    var _S213 : f32 = _S198 - _S200;
    var hue_r_2 : f32 = _S213 / safe_delta_3;
    var _S214 : f32 = safe_delta_3 * safe_delta_3;
    var s_diff_hue_r_0 : f32 = ((_S199 - _S201) * safe_delta_3 - _S213 * s_diff_safe_delta_0) / _S214;
    var _S215 : f32 = _S200 - _S196;
    var _S216 : f32 = ((_S201 - _S197) * safe_delta_3 - _S215 * s_diff_safe_delta_0) / _S214;
    var hue_g_2 : f32 = 2.0f + _S215 / safe_delta_3;
    var _S217 : f32 = _S196 - _S198;
    var _S218 : f32 = ((_S197 - _S199) * safe_delta_3 - _S217 * s_diff_safe_delta_0) / _S214;
    var hue_b_2 : f32 = 4.0f + _S217 / safe_delta_3;
    if(_S196 == (_S210.primal_0))
    {
        safe_delta_3 = hue_r_2;
        s_diff_safe_delta_0 = s_diff_hue_r_0;
    }
    else
    {
        if(_S198 == (_S210.primal_0))
        {
            safe_delta_3 = hue_g_2;
            s_diff_safe_delta_0 = _S216;
        }
        else
        {
            safe_delta_3 = hue_b_2;
            s_diff_safe_delta_0 = _S218;
        }
    }
    var hue_4 : f32 = safe_delta_3 / 6.0f;
    var s_diff_hue_0 : f32 = s_diff_safe_delta_0 * 0.1666666716337204f;
    var hue_5 : f32 = hue_4 - floor(hue_4);
    if(chromatic_3)
    {
        safe_delta_3 = hue_5;
        s_diff_safe_delta_0 = s_diff_hue_0;
    }
    else
    {
        safe_delta_3 = 0.0f;
        s_diff_safe_delta_0 = 0.0f;
    }
    if(chromatic_3)
    {
        var _S219 : f32 = (s_diff_delta_0 * safe_hi_3 - delta_3 * s_diff_safe_hi_0) / (safe_hi_3 * safe_hi_3);
        safe_hi_3 = delta_3 / safe_hi_3;
        s_diff_safe_hi_0 = _S219;
    }
    else
    {
        safe_hi_3 = 0.0f;
        s_diff_safe_hi_0 = 0.0f;
    }
    var _S220 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(safe_delta_3, safe_hi_3, _S210.primal_0), vec3<f32>(s_diff_safe_delta_0, s_diff_safe_hi_0, _S210.differential_0) );
    return _S220;
}

struct AgxParams_0
{
     pipe_to_base_r0_0 : vec4<f32>,
     pipe_to_base_r1_0 : vec4<f32>,
     pipe_to_base_r2_0 : vec4<f32>,
     base_to_rendering_r0_0 : vec4<f32>,
     base_to_rendering_r1_0 : vec4<f32>,
     base_to_rendering_r2_0 : vec4<f32>,
     rendering_to_pipe_r0_0 : vec4<f32>,
     rendering_to_pipe_r1_0 : vec4<f32>,
     rendering_to_pipe_r2_0 : vec4<f32>,
     rendering_luminance_0 : vec4<f32>,
     black_ev_0 : f32,
     range_ev_0 : f32,
     gamma_0 : f32,
     slope_0 : f32,
     intercept_0 : f32,
     target_black_0 : f32,
     target_white_0 : f32,
     _pad0_0 : f32,
     toe_transition_x_0 : f32,
     toe_transition_y_0 : f32,
     toe_scale_0 : f32,
     toe_power_0 : f32,
     toe_fallback_coefficient_0 : f32,
     toe_fallback_power_0 : f32,
     toe_fallback_0 : u32,
     _pad1_0 : f32,
     shoulder_transition_x_0 : f32,
     shoulder_transition_y_0 : f32,
     shoulder_scale_0 : f32,
     shoulder_power_0 : f32,
     shoulder_fallback_coefficient_0 : f32,
     shoulder_fallback_power_0 : f32,
     shoulder_fallback_0 : u32,
     _pad2_0 : f32,
     look_slope_0 : f32,
     look_lift_0 : f32,
     look_power_0 : f32,
     look_saturation_0 : f32,
     look_hue_mix_0 : f32,
     _pad3_0 : f32,
     _pad4_0 : f32,
     _pad5_0 : f32,
};

fn log_encode_0( x_9 : f32,  p_0 : AgxParams_0) -> f32
{
    return clamp((log2(max(9.99999997475242708e-07f, x_9 / 0.18000000715255737f)) - p_0.black_ev_0) / p_0.range_ev_0, 0.0f, 1.0f);
}

fn pow0_forward_0( x_10 : DiffPair_float_0,  y_8 : DiffPair_float_0) -> DiffPair_float_0
{
    if(((x_10.primal_0) <= 0.0f))
    {
        var _S221 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
        return _S221;
    }
    var value_0 : f32 = pow(x_10.primal_0, y_8.primal_0);
    var _S222 : DiffPair_float_0 = DiffPair_float_0( value_0, x_10.differential_0 * y_8.primal_0 * pow(x_10.primal_0, y_8.primal_0 - 1.0f) + y_8.differential_0 * value_0 * log(x_10.primal_0) );
    return _S222;
}

fn agx_sigmoid_0( t_0 : f32,  power_1 : f32) -> f32
{
    if((t_0 <= 1.0f))
    {
        return t_0 * pow(1.0f + pow0_0(t_0, power_1), -1.0f / power_1);
    }
    return pow(1.0f + pow(t_0, - power_1), -1.0f / power_1);
}

fn agx_curve_0( x_11 : f32,  p_1 : AgxParams_0) -> f32
{
    var y_9 : f32;
    if((x_11 < (p_1.toe_transition_x_0)))
    {
        if((p_1.toe_fallback_0) != u32(0))
        {
            y_9 = p_1.target_black_0 + max(0.0f, p_1.toe_fallback_coefficient_0 * pow0_0(x_11, p_1.toe_fallback_power_0));
        }
        else
        {
            y_9 = p_1.toe_scale_0 * agx_sigmoid_0(p_1.slope_0 * (x_11 - p_1.toe_transition_x_0) / p_1.toe_scale_0, p_1.toe_power_0) + p_1.toe_transition_y_0;
        }
    }
    else
    {
        if((x_11 <= (p_1.shoulder_transition_x_0)))
        {
            y_9 = p_1.slope_0 * x_11 + p_1.intercept_0;
        }
        else
        {
            if((p_1.shoulder_fallback_0) != u32(0))
            {
                y_9 = p_1.target_white_0 - max(0.0f, p_1.shoulder_fallback_coefficient_0 * pow0_0(1.0f - x_11, p_1.shoulder_fallback_power_0));
            }
            else
            {
                y_9 = p_1.shoulder_scale_0 * agx_sigmoid_0(p_1.slope_0 * (x_11 - p_1.shoulder_transition_x_0) / p_1.shoulder_scale_0, p_1.shoulder_power_0) + p_1.shoulder_transition_y_0;
            }
        }
    }
    return clamp(y_9, p_1.target_black_0, p_1.target_white_0);
}

fn agx_tone_curve_0( rgb_in_0 : vec3<f32>,  p_2 : AgxParams_0) -> vec4<f32>
{
    var rendering_0 : vec3<f32> = (((compress_into_gamut_0((((clamp(rgb_in_0, vec3<f32>(-1.0e+06f), vec3<f32>(1.0e+06f))) * (mat3x3<f32>(p_2.pipe_to_base_r0_0.xyz, p_2.pipe_to_base_r1_0.xyz, p_2.pipe_to_base_r2_0.xyz)))))) * (mat3x3<f32>(p_2.base_to_rendering_r0_0.xyz, p_2.base_to_rendering_r1_0.xyz, p_2.base_to_rendering_r2_0.xyz))));
    return vec4<f32>(agx_curve_0(log_encode_0(rendering_0.x, p_2), p_2), agx_curve_0(log_encode_0(rendering_0.y, p_2), p_2), agx_curve_0(log_encode_0(rendering_0.z, p_2), p_2), rgb_to_hsv_0(rendering_0).x);
}

fn pow0_1( x_12 : vec3<f32>,  y_10 : vec3<f32>) -> vec3<f32>
{
    return vec3<f32>(pow0_0(x_12.x, y_10.x), pow0_0(x_12.y, y_10.y), pow0_0(x_12.z, y_10.z));
}

struct Tuple_9
{
     value0_13 : vec3<f32>,
};

fn s_apply_pow0_1( _S223 : vec3<f32>,  _S224 : vec3<f32>) -> Tuple_9
{
    var _S225 : Tuple_9;
    _S225.value0_13 = vec3<f32>(s_apply_pow0_0(_S223.x, _S224.x).value0_6, s_apply_pow0_0(_S223.y, _S224.y).value0_6, s_apply_pow0_0(_S223.z, _S224.z).value0_6);
    return _S225;
}

struct s_paramCtx_s_bwdCallableCtx_pow0_0
{
     value0_14 : vec3<f32>,
     value1_1 : vec3<f32>,
};

struct s_bwdCallableCtx_pow0_1
{
     value0_15 : s_paramCtx_s_bwdCallableCtx_pow0_0,
};

fn s_bwdProp_pow0_1( _S226 : s_bwdCallableCtx_pow0_1,  _S227 : ptr<function, vec3<f32>>,  _S228 : ptr<function, vec3<f32>>,  _S229 : vec3<f32>)
{
    var _S230 : f32 = _S226.value0_15.value1_1.x;
    var _S231 : s_bwdCallableCtx_pow0_0;
    _S231._S74 = _S226.value0_15.value0_14.x;
    _S231._S75 = _S230;
    var _S232 : f32 = _S226.value0_15.value1_1.y;
    var _S233 : s_bwdCallableCtx_pow0_0;
    _S233._S74 = _S226.value0_15.value0_14.y;
    _S233._S75 = _S232;
    var _S234 : f32 = _S226.value0_15.value1_1.z;
    var _S235 : s_bwdCallableCtx_pow0_0;
    _S235._S74 = _S226.value0_15.value0_14.z;
    _S235._S75 = _S234;
    var _S236 : f32 = 0.0f;
    var _S237 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S235, &(_S236), &(_S237), _S229[i32(2)]);
    var _S238 : f32 = 0.0f;
    var _S239 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S233, &(_S238), &(_S239), _S229[i32(1)]);
    var _S240 : f32 = 0.0f;
    var _S241 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S231, &(_S240), &(_S241), _S229[i32(0)]);
    var _S242 : vec3<f32> = vec3<f32>(_S241, _S239, _S237);
    (*_S227) = vec3<f32>(_S240, _S238, _S236);
    (*_S228) = _S242;
    return;
}

fn s_fwd_pow0_0( dpx_12 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpy_10 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S243 : DiffPair_float_0 = DiffPair_float_0( dpx_12.primal_0.x, dpx_12.differential_0.x );
    var _S244 : DiffPair_float_0 = DiffPair_float_0( dpy_10.primal_0.x, dpy_10.differential_0.x );
    var _S245 : DiffPair_float_0 = pow0_forward_0(_S243, _S244);
    var _S246 : DiffPair_float_0 = DiffPair_float_0( dpx_12.primal_0.y, dpx_12.differential_0.y );
    var _S247 : DiffPair_float_0 = DiffPair_float_0( dpy_10.primal_0.y, dpy_10.differential_0.y );
    var _S248 : DiffPair_float_0 = pow0_forward_0(_S246, _S247);
    var _S249 : DiffPair_float_0 = DiffPair_float_0( dpx_12.primal_0.z, dpx_12.differential_0.z );
    var _S250 : DiffPair_float_0 = DiffPair_float_0( dpy_10.primal_0.z, dpy_10.differential_0.z );
    var _S251 : DiffPair_float_0 = pow0_forward_0(_S249, _S250);
    var _S252 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S245.primal_0, _S248.primal_0, _S251.primal_0), vec3<f32>(_S245.differential_0, _S248.differential_0, _S251.differential_0) );
    return _S252;
}

fn apply_look_0( y_11 : vec3<f32>,  luminance_0 : vec3<f32>,  look_0 : AgxLook_0) -> vec3<f32>
{
    var m_1 : f32 = look_0.slope_1 / (1.0f + look_0.lift_0);
    var v_0 : vec3<f32> = vec3<f32>(m_1) * y_11 + vec3<f32>((look_0.lift_0 * m_1));
    var _S253 : vec3<f32> = vec3<f32>(0.0f);
    var v_1 : vec3<f32> = pow0_1(max(v_0, _S253), vec3<f32>(look_0.power_0)) + min(v_0, _S253);
    var _S254 : vec3<f32> = vec3<f32>(dot(luminance_0, v_1));
    return _S254 + vec3<f32>(look_0.saturation_0) * (v_1 - _S254);
}

struct Tuple_10
{
     value0_16 : vec3<f32>,
};

fn s_apply_apply_look_0( _S255 : vec3<f32>,  luminance_1 : vec3<f32>,  _S256 : AgxLook_0) -> Tuple_10
{
    var m_2 : f32 = _S256.slope_1 / (1.0f + _S256.lift_0);
    var v_2 : vec3<f32> = vec3<f32>(m_2) * _S255 + vec3<f32>((_S256.lift_0 * m_2));
    var _S257 : vec3<f32> = vec3<f32>(0.0f);
    var v_3 : vec3<f32> = s_apply_pow0_1(s_apply_max_1(v_2, _S257).value0_9, vec3<f32>(_S256.power_0)).value0_13 + s_apply_min_1(v_2, _S257).value0_8;
    var _S258 : vec3<f32> = vec3<f32>(s_apply_dot_0(luminance_1, v_3).value0_7);
    var _S259 : Tuple_10;
    _S259.value0_16 = _S258 + vec3<f32>(_S256.saturation_0) * (v_3 - _S258);
    return _S259;
}

struct s_paramCtx_s_bwdCallableCtx_apply_look_0
{
     value0_17 : vec3<f32>,
     value1_2 : vec3<f32>,
     value2_1 : AgxLook_0,
};

struct s_bwdCallableCtx_apply_look_0
{
     value0_18 : s_paramCtx_s_bwdCallableCtx_apply_look_0,
};

fn s_bwdProp_apply_look_0( _S260 : s_bwdCallableCtx_apply_look_0,  _S261 : ptr<function, vec3<f32>>,  _S262 : ptr<function, AgxLook_0>,  _S263 : vec3<f32>)
{
    var _S264 : f32 = 1.0f + _S260.value0_18.value2_1.lift_0;
    var m_3 : f32 = _S260.value0_18.value2_1.slope_1 / _S264;
    var _S265 : f32 = _S264 * _S264;
    var _S266 : vec3<f32> = vec3<f32>(m_3);
    var v_4 : vec3<f32> = _S266 * _S260.value0_18.value0_17 + vec3<f32>((_S260.value0_18.value2_1.lift_0 * m_3));
    var _S267 : vec3<f32> = vec3<f32>(0.0f);
    var _S268 : Tuple_7 = s_apply_max_1(v_4, _S267);
    var _S269 : s_bwdCallableCtx_max_1;
    _S269._S96 = v_4;
    _S269._S97 = _S267;
    var _S270 : vec3<f32> = vec3<f32>(_S260.value0_18.value2_1.power_0);
    var _S271 : Tuple_9 = s_apply_pow0_1(_S268.value0_9, _S270);
    var _S272 : s_paramCtx_s_bwdCallableCtx_pow0_0;
    _S272.value0_14 = _S268.value0_9;
    _S272.value1_1 = _S270;
    var _S273 : s_bwdCallableCtx_pow0_1 = s_bwdCallableCtx_pow0_1( _S272 );
    var _S274 : Tuple_6 = s_apply_min_1(v_4, _S267);
    var _S275 : s_bwdCallableCtx_min_1;
    _S275._S84 = v_4;
    _S275._S85 = _S267;
    var v_5 : vec3<f32> = _S271.value0_13 + _S274.value0_8;
    var _S276 : Tuple_5 = s_apply_dot_0(_S260.value0_18.value1_2, v_5);
    var _S277 : s_bwdCallableCtx_dot_0;
    _S277._S79 = _S260.value0_18.value1_2;
    _S277._S80 = v_5;
    var _S278 : AgxLook_0 = AgxLook_x24_syn_dzero_0();
    var _S279 : vec3<f32> = vec3<f32>(_S260.value0_18.value2_1.saturation_0) * _S263;
    var _S280 : vec3<f32> = (v_5 - vec3<f32>(_S276.value0_7)) * _S263;
    var _S281 : f32 = _S280[i32(0)] + _S280[i32(1)] + _S280[i32(2)];
    var _S282 : vec3<f32> = (vec3<f32>(0) - _S279);
    var _S283 : f32 = _S263[i32(0)] + _S263[i32(1)] + _S263[i32(2)] + (_S282[i32(0)] + _S282[i32(1)] + _S282[i32(2)]);
    var _S284 : vec3<f32> = vec3<f32>(0.0f);
    var _S285 : vec3<f32> = _S284;
    var _S286 : vec3<f32> = _S284;
    s_bwdProp_dot_0(_S277, &(_S285), &(_S286), _S283);
    var _S287 : vec3<f32> = _S279 + _S286;
    var _S288 : vec3<f32> = _S284;
    var _S289 : vec3<f32> = _S284;
    s_bwdProp_min_1(_S275, &(_S288), &(_S289), _S287);
    var _S290 : vec3<f32> = _S284;
    var _S291 : vec3<f32> = _S284;
    s_bwdProp_pow0_1(_S273, &(_S290), &(_S291), _S287);
    var _S292 : f32 = _S291[i32(0)] + _S291[i32(1)] + _S291[i32(2)];
    var _S293 : vec3<f32> = _S284;
    var _S294 : vec3<f32> = _S284;
    s_bwdProp_max_1(_S269, &(_S293), &(_S294), _S290);
    var _S295 : vec3<f32> = _S288 + _S293;
    var _S296 : f32 = _S295[i32(0)] + _S295[i32(1)] + _S295[i32(2)];
    var _S297 : vec3<f32> = _S266 * _S295;
    var _S298 : vec3<f32> = _S260.value0_18.value0_17 * _S295;
    var _S299 : f32 = (_S260.value0_18.value2_1.lift_0 * _S296 + (_S298[i32(0)] + _S298[i32(1)] + _S298[i32(2)])) / _S265;
    var _S300 : f32 = _S264 * _S299;
    var _S301 : f32 = m_3 * _S296 + _S260.value0_18.value2_1.slope_1 * - _S299;
    var _S302 : AgxLook_0 = _S278;
    _S302.saturation_0 = _S281;
    _S302.power_0 = _S292;
    _S302.lift_0 = _S301;
    _S302.slope_1 = _S300;
    var _S303 : AgxLook_0 = AgxLook_x24_syn_dadd_0(_S278, _S302);
    (*_S261) = _S297;
    (*_S262) = _S303;
    return;
}

struct DiffPair_AgxLook_0
{
     primal_0 : AgxLook_0,
     differential_0 : AgxLook_0,
};

fn s_fwd_apply_look_0( dpy_11 : DiffPair_vectorx3Cfloatx2C3x3E_0,  luminance_2 : vec3<f32>,  dplook_0 : DiffPair_AgxLook_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S304 : f32 = 1.0f + dplook_0.primal_0.lift_0;
    var m_4 : f32 = dplook_0.primal_0.slope_1 / _S304;
    var s_diff_m_0 : f32 = (dplook_0.differential_0.slope_1 * _S304 - dplook_0.primal_0.slope_1 * dplook_0.differential_0.lift_0) / (_S304 * _S304);
    var _S305 : vec3<f32> = vec3<f32>(m_4);
    var _S306 : vec3<f32> = vec3<f32>(0.0f);
    var _S307 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S305 * dpy_11.primal_0 + vec3<f32>((dplook_0.primal_0.lift_0 * m_4)), vec3<f32>(s_diff_m_0) * dpy_11.primal_0 + dpy_11.differential_0 * _S305 + vec3<f32>((dplook_0.differential_0.lift_0 * m_4 + s_diff_m_0 * dplook_0.primal_0.lift_0)) );
    var _S308 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.0f), _S306 );
    var _S309 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_max_vector_1(_S307, _S308);
    var _S310 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S309.primal_0, _S309.differential_0 );
    var _S311 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(dplook_0.primal_0.power_0), vec3<f32>(dplook_0.differential_0.power_0) );
    var _S312 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_pow0_0(_S310, _S311);
    var _S313 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_min_vector_1(_S307, _S308);
    var v_6 : vec3<f32> = _S312.primal_0 + _S313.primal_0;
    var s_diff_v_0 : vec3<f32> = _S312.differential_0 + _S313.differential_0;
    var _S314 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( luminance_2, _S306 );
    var _S315 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( v_6, s_diff_v_0 );
    var _S316 : DiffPair_float_0 = _d_dot_1(_S314, _S315);
    var _S317 : vec3<f32> = vec3<f32>(_S316.primal_0);
    var _S318 : vec3<f32> = v_6 - _S317;
    var _S319 : vec3<f32> = vec3<f32>(_S316.differential_0);
    var _S320 : vec3<f32> = vec3<f32>(dplook_0.primal_0.saturation_0);
    var _S321 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S317 + _S320 * _S318, _S319 + (vec3<f32>(dplook_0.differential_0.saturation_0) * _S318 + (s_diff_v_0 - _S319) * _S320) );
    return _S321;
}

fn s_fwd_hsv_in_sector_0( dphsv_0 : DiffPair_vectorx3Cfloatx2C3x3E_0,  sector_1 : f32,  base_2 : f32) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S322 : f32 = dphsv_0.primal_0.y;
    var _S323 : f32 = dphsv_0.primal_0.z;
    var _S324 : f32 = dphsv_0.differential_0.z;
    var c_2 : f32 = _S322 * _S323;
    var s_diff_c_0 : f32 = dphsv_0.differential_0.y * _S323 + _S324 * _S322;
    var m_5 : f32 = _S323 - c_2;
    var s_diff_m_1 : f32 = _S324 - s_diff_c_0;
    var _S325 : f32 = dphsv_0.primal_0.x * 6.0f - base_2;
    var fc_1 : f32 = _S325 * c_2;
    var s_diff_fc_0 : f32 = dphsv_0.differential_0.x * 6.0f * c_2 + s_diff_c_0 * _S325;
    var top_1 : f32 = c_2 + m_5;
    var s_diff_top_0 : f32 = s_diff_c_0 + s_diff_m_1;
    var rising_1 : f32 = fc_1 + m_5;
    var s_diff_rising_0 : f32 = s_diff_fc_0 + s_diff_m_1;
    var falling_1 : f32 = top_1 - fc_1;
    var s_diff_falling_0 : f32 = s_diff_top_0 - s_diff_fc_0;
    var _S326 : vec3<f32> = vec3<f32>(top_1, m_5, falling_1);
    var _S327 : vec3<f32> = vec3<f32>(s_diff_top_0, s_diff_m_1, s_diff_falling_0);
    var rgb_3 : vec3<f32>;
    var s_diff_rgb_0 : vec3<f32>;
    if(sector_1 == 0.0f)
    {
        var _S328 : vec3<f32> = vec3<f32>(s_diff_top_0, s_diff_rising_0, s_diff_m_1);
        rgb_3 = vec3<f32>(top_1, rising_1, m_5);
        s_diff_rgb_0 = _S328;
    }
    else
    {
        if(sector_1 == 1.0f)
        {
            var _S329 : vec3<f32> = vec3<f32>(s_diff_falling_0, s_diff_top_0, s_diff_m_1);
            rgb_3 = vec3<f32>(falling_1, top_1, m_5);
            s_diff_rgb_0 = _S329;
        }
        else
        {
            if(sector_1 == 2.0f)
            {
                var _S330 : vec3<f32> = vec3<f32>(s_diff_m_1, s_diff_top_0, s_diff_rising_0);
                rgb_3 = vec3<f32>(m_5, top_1, rising_1);
                s_diff_rgb_0 = _S330;
            }
            else
            {
                if(sector_1 == 3.0f)
                {
                    var _S331 : vec3<f32> = vec3<f32>(s_diff_m_1, s_diff_falling_0, s_diff_top_0);
                    rgb_3 = vec3<f32>(m_5, falling_1, top_1);
                    s_diff_rgb_0 = _S331;
                }
                else
                {
                    if(sector_1 == 4.0f)
                    {
                        var _S332 : vec3<f32> = vec3<f32>(s_diff_rising_0, s_diff_m_1, s_diff_top_0);
                        rgb_3 = vec3<f32>(rising_1, m_5, top_1);
                        s_diff_rgb_0 = _S332;
                    }
                    else
                    {
                        rgb_3 = _S326;
                        s_diff_rgb_0 = _S327;
                    }
                }
            }
        }
    }
    var _S333 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( rgb_3, s_diff_rgb_0 );
    return _S333;
}

fn hsv_sides_1( h_1 : f32,  right_3 : ptr<function, f32>,  right_base_2 : ptr<function, f32>,  left_3 : ptr<function, f32>,  left_base_2 : ptr<function, f32>)
{
    var nearest_1 : f32 = round(h_1);
    var boundary_1 : bool = ((abs(h_1 - nearest_1)) < 9.9999997473787516e-06f);
    var _S334 : f32;
    if(boundary_1)
    {
        _S334 = nearest_1;
    }
    else
    {
        _S334 = floor(h_1);
    }
    (*right_base_2) = _S334;
    if(boundary_1)
    {
        _S334 = nearest_1 - 1.0f;
    }
    else
    {
        _S334 = (*right_base_2);
    }
    (*left_base_2) = _S334;
    (*right_3) = (*right_base_2) - 6.0f * floor((*right_base_2) / 6.0f);
    (*left_3) = (*left_base_2) - 6.0f * floor((*left_base_2) / 6.0f);
    return;
}

fn hsv_to_rgb_forward_0( hsv_4 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var right_4 : f32;
    var right_base_3 : f32;
    var left_4 : f32;
    var left_base_3 : f32;
    hsv_sides_1(hsv_4.primal_0.x * 6.0f, &(right_4), &(right_base_3), &(left_4), &(left_base_3));
    var _S335 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( hsv_to_rgb_0(hsv_4.primal_0), vec3<f32>(0.5f) * (s_fwd_hsv_in_sector_0(hsv_4, right_4, right_base_3).differential_0 + s_fwd_hsv_in_sector_0(hsv_4, left_4, left_base_3).differential_0) );
    return _S335;
}

fn restore_hue_0( rgb_4 : vec3<f32>,  hue_in_0 : f32,  mix_0 : f32) -> vec3<f32>
{
    var _S336 : vec3<f32> = rgb_to_hsv_0(rgb_4);
    var hsv_5 : vec3<f32> = _S336;
    var difference_0 : f32 = _S336.x - hue_in_0;
    var mixed_0 : f32 = (1.0f - mix_0) * (difference_0 - round(difference_0)) + hue_in_0;
    hsv_5[i32(0)] = mixed_0 - floor(mixed_0);
    return hsv_to_rgb_0(hsv_5);
}

struct Tuple_11
{
     value0_19 : vec3<f32>,
};

fn s_apply_restore_hue_0( _S337 : vec3<f32>,  hue_in_1 : f32,  _S338 : f32) -> Tuple_11
{
    var _S339 : Tuple_8 = s_apply_rgb_to_hsv_0(_S337);
    var difference_1 : f32 = _S339.value0_10.x - hue_in_1;
    var mixed_1 : f32 = (1.0f - _S338) * (difference_1 - round(difference_1)) + hue_in_1;
    var _S340 : f32 = mixed_1 - floor(mixed_1);
    var _S341 : vec3<f32> = _S339.value0_10;
    _S341[i32(0)] = _S340;
    var _S342 : Tuple_11;
    _S342.value0_19 = s_apply_hsv_to_rgb_0(_S341).value0_3;
    return _S342;
}

struct s_paramCtx_s_bwdCallableCtx_restore_hue_0
{
     value0_20 : vec3<f32>,
     value1_3 : f32,
     value2_2 : f32,
};

struct s_bwdCallableCtx_restore_hue_0
{
     value0_21 : s_paramCtx_s_bwdCallableCtx_restore_hue_0,
};

fn s_bwdProp_restore_hue_0( _S343 : s_bwdCallableCtx_restore_hue_0,  _S344 : ptr<function, vec3<f32>>,  _S345 : ptr<function, f32>,  _S346 : vec3<f32>)
{
    var _S347 : Tuple_8 = s_apply_rgb_to_hsv_0(_S343.value0_21.value0_20);
    var _S348 : s_paramCtx_s_bwdCallableCtx_rgb_to_hsv_0;
    _S348.value0_11 = _S343.value0_21.value0_20;
    var _S349 : s_bwdCallableCtx_rgb_to_hsv_0 = s_bwdCallableCtx_rgb_to_hsv_0( _S348 );
    var difference_2 : f32 = _S347.value0_10.x - _S343.value0_21.value1_3;
    var difference_3 : f32 = difference_2 - round(difference_2);
    var _S350 : f32 = 1.0f - _S343.value0_21.value2_2;
    var mixed_2 : f32 = _S350 * difference_3 + _S343.value0_21.value1_3;
    var _S351 : f32 = mixed_2 - floor(mixed_2);
    var _S352 : vec3<f32> = _S347.value0_10;
    _S352[i32(0)] = _S351;
    var _S353 : s_bwdCallableCtx_hsv_to_rgb_0;
    _S353._S25 = _S352;
    var _S354 : vec3<f32> = vec3<f32>(0.0f);
    var _S355 : vec3<f32> = _S354;
    s_bwdProp_hsv_to_rgb_0(_S353, &(_S355), _S346);
    _S352 = _S355;
    _S352[i32(0)] = 0.0f;
    var _S356 : f32 = - (difference_3 * _S355[i32(0)]);
    var _S357 : vec3<f32> = _S352 + vec3<f32>(_S350 * _S355[i32(0)], 0.0f, 0.0f);
    var _S358 : vec3<f32> = _S354;
    s_bwdProp_rgb_to_hsv_0(_S349, &(_S358), _S357);
    (*_S344) = _S358;
    (*_S345) = _S356;
    return;
}

fn s_fwd_restore_hue_0( dprgb_1 : DiffPair_vectorx3Cfloatx2C3x3E_0,  hue_in_2 : f32,  dpmix_0 : DiffPair_float_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S359 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( dprgb_1.primal_0, dprgb_1.differential_0 );
    var _S360 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_rgb_to_hsv_0(_S359);
    var difference_4 : f32 = _S360.primal_0.x - hue_in_2;
    var difference_5 : f32 = difference_4 - round(difference_4);
    var _S361 : f32 = 1.0f - dpmix_0.primal_0;
    var _S362 : f32 = (0.0f - dpmix_0.differential_0) * difference_5 + _S360.differential_0.x * _S361;
    var mixed_3 : f32 = _S361 * difference_5 + hue_in_2;
    var _S363 : f32 = mixed_3 - floor(mixed_3);
    var _S364 : vec3<f32> = _S360.primal_0;
    _S364[i32(0)] = _S363;
    var _S365 : vec3<f32> = _S360.differential_0;
    _S365[i32(0)] = _S362;
    var _S366 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S364, _S365 );
    var _S367 : DiffPair_vectorx3Cfloatx2C3x3E_0 = hsv_to_rgb_forward_0(_S366);
    var _S368 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S367.primal_0, _S367.differential_0 );
    return _S368;
}

struct s_paramCtx_s_bwdCallableCtx_agx_0
{
     value0_22 : vec3<f32>,
     value1_4 : AgxParams_0,
     value2_3 : AgxLook_0,
};

struct s_bwdCallableCtx_agx_0
{
     value0_23 : s_paramCtx_s_bwdCallableCtx_agx_0,
};

fn s_bwdProp_agx_0( _S369 : s_bwdCallableCtx_agx_0,  _S370 : ptr<function, AgxLook_0>,  _S371 : vec3<f32>)
{
    var curve_0 : vec4<f32> = agx_tone_curve_0(_S369.value0_23.value0_22, _S369.value0_23.value1_4);
    var _S372 : vec3<f32> = curve_0.xyz;
    var _S373 : vec3<f32> = _S369.value0_23.value1_4.rendering_luminance_0.xyz;
    var _S374 : Tuple_10 = s_apply_apply_look_0(_S372, _S373, _S369.value0_23.value2_3);
    var _S375 : s_paramCtx_s_bwdCallableCtx_apply_look_0;
    _S375.value0_17 = _S372;
    _S375.value1_2 = _S373;
    _S375.value2_1 = _S369.value0_23.value2_3;
    var _S376 : s_bwdCallableCtx_apply_look_0 = s_bwdCallableCtx_apply_look_0( _S375 );
    var _S377 : vec3<f32> = vec3<f32>(0.0f);
    var _S378 : Tuple_7 = s_apply_max_1(_S374.value0_16, _S377);
    var _S379 : s_bwdCallableCtx_max_1;
    _S379._S96 = _S374.value0_16;
    _S379._S97 = _S377;
    var _S380 : vec3<f32> = vec3<f32>(_S369.value0_23.value1_4.gamma_0);
    var _S381 : Tuple_9 = s_apply_pow0_1(_S378.value0_9, _S380);
    var _S382 : s_paramCtx_s_bwdCallableCtx_pow0_0;
    _S382.value0_14 = _S378.value0_9;
    _S382.value1_1 = _S380;
    var _S383 : s_bwdCallableCtx_pow0_1 = s_bwdCallableCtx_pow0_1( _S382 );
    var _S384 : f32 = curve_0.w;
    var _S385 : Tuple_11 = s_apply_restore_hue_0(_S381.value0_13, _S384, _S369.value0_23.value2_3.hue_mix_0);
    var _S386 : s_paramCtx_s_bwdCallableCtx_restore_hue_0;
    _S386.value0_20 = _S381.value0_13;
    _S386.value1_3 = _S384;
    _S386.value2_2 = _S369.value0_23.value2_3.hue_mix_0;
    var _S387 : s_bwdCallableCtx_restore_hue_0 = s_bwdCallableCtx_restore_hue_0( _S386 );
    var _S388 : s_bwdCallableCtx_mul_0;
    _S388._S88 = mat3x3<f32>(_S369.value0_23.value1_4.rendering_to_pipe_r0_0.xyz, _S369.value0_23.value1_4.rendering_to_pipe_r1_0.xyz, _S369.value0_23.value1_4.rendering_to_pipe_r2_0.xyz);
    _S388._S89 = _S385.value0_19;
    var _S389 : AgxLook_0 = AgxLook_x24_syn_dzero_0();
    var _S390 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S391 : vec3<f32> = vec3<f32>(0.0f);
    var _S392 : vec3<f32> = _S391;
    s_bwdProp_mul_0(_S388, &(_S390), &(_S392), _S371);
    var _S393 : vec3<f32> = _S391;
    var _S394 : f32 = 0.0f;
    s_bwdProp_restore_hue_0(_S387, &(_S393), &(_S394), _S392);
    var _S395 : vec3<f32> = _S391;
    var _S396 : vec3<f32> = _S391;
    s_bwdProp_pow0_1(_S383, &(_S395), &(_S396), _S393);
    var _S397 : vec3<f32> = _S391;
    var _S398 : vec3<f32> = _S391;
    s_bwdProp_max_1(_S379, &(_S397), &(_S398), _S395);
    var _S399 : vec3<f32> = _S391;
    var _S400 : AgxLook_0;
    _S400.slope_1 = 0.0f;
    _S400.lift_0 = 0.0f;
    _S400.power_0 = 0.0f;
    _S400.saturation_0 = 0.0f;
    _S400.hue_mix_0 = 0.0f;
    s_bwdProp_apply_look_0(_S376, &(_S399), &(_S400), _S397);
    var _S401 : AgxLook_0 = AgxLook_x24_syn_dadd_0(_S400, _S389);
    var _S402 : AgxLook_0 = _S389;
    _S402.hue_mix_0 = _S394;
    (*_S370) = AgxLook_x24_syn_dadd_0(_S401, _S402);
    return;
}

fn agx_look_0() -> AgxLook_0
{
    var look_1 : AgxLook_0;
    look_1.slope_1 = params_0.agx_0.look_slope_0;
    look_1.lift_0 = params_0.agx_0.look_lift_0;
    look_1.power_0 = params_0.agx_0.look_power_0;
    look_1.saturation_0 = params_0.agx_0.look_saturation_0;
    look_1.hue_mix_0 = params_0.agx_0.look_hue_mix_0;
    return look_1;
}

fn log_encode_1( _S403 : f32) -> f32
{
    return clamp((log2(max(9.99999997475242708e-07f, _S403 / 0.18000000715255737f)) - params_0.agx_0.black_ev_0) / params_0.agx_0.range_ev_0, 0.0f, 1.0f);
}

fn agx_curve_1( _S404 : f32) -> f32
{
    var _S405 : f32 = params_0.agx_0.toe_transition_x_0;
    var y_12 : f32;
    if((_S404 < (params_0.agx_0.toe_transition_x_0)))
    {
        if((params_0.agx_0.toe_fallback_0) != u32(0))
        {
            y_12 = params_0.agx_0.target_black_0 + max(0.0f, params_0.agx_0.toe_fallback_coefficient_0 * pow0_0(_S404, params_0.agx_0.toe_fallback_power_0));
        }
        else
        {
            y_12 = params_0.agx_0.toe_scale_0 * agx_sigmoid_0(params_0.agx_0.slope_0 * (_S404 - _S405) / params_0.agx_0.toe_scale_0, params_0.agx_0.toe_power_0) + params_0.agx_0.toe_transition_y_0;
        }
    }
    else
    {
        var _S406 : f32 = params_0.agx_0.shoulder_transition_x_0;
        if((_S404 <= (params_0.agx_0.shoulder_transition_x_0)))
        {
            y_12 = params_0.agx_0.slope_0 * _S404 + params_0.agx_0.intercept_0;
        }
        else
        {
            if((params_0.agx_0.shoulder_fallback_0) != u32(0))
            {
                y_12 = params_0.agx_0.target_white_0 - max(0.0f, params_0.agx_0.shoulder_fallback_coefficient_0 * pow0_0(1.0f - _S404, params_0.agx_0.shoulder_fallback_power_0));
            }
            else
            {
                y_12 = params_0.agx_0.shoulder_scale_0 * agx_sigmoid_0(params_0.agx_0.slope_0 * (_S404 - _S406) / params_0.agx_0.shoulder_scale_0, params_0.agx_0.shoulder_power_0) + params_0.agx_0.shoulder_transition_y_0;
            }
        }
    }
    return clamp(y_12, params_0.agx_0.target_black_0, params_0.agx_0.target_white_0);
}

fn agx_tone_curve_1( _S407 : vec3<f32>) -> vec4<f32>
{
    var rendering_1 : vec3<f32> = (((compress_into_gamut_0((((clamp(_S407, vec3<f32>(-1.0e+06f), vec3<f32>(1.0e+06f))) * (mat3x3<f32>(params_0.agx_0.pipe_to_base_r0_0.xyz, params_0.agx_0.pipe_to_base_r1_0.xyz, params_0.agx_0.pipe_to_base_r2_0.xyz)))))) * (mat3x3<f32>(params_0.agx_0.base_to_rendering_r0_0.xyz, params_0.agx_0.base_to_rendering_r1_0.xyz, params_0.agx_0.base_to_rendering_r2_0.xyz))));
    return vec4<f32>(agx_curve_1(log_encode_1(rendering_1.x)), agx_curve_1(log_encode_1(rendering_1.y)), agx_curve_1(log_encode_1(rendering_1.z)), rgb_to_hsv_0(rendering_1).x);
}

fn agx_1( _S408 : vec3<f32>,  _S409 : AgxLook_0) -> vec3<f32>
{
    var _S410 : vec4<f32> = agx_tone_curve_1(_S408);
    return (((restore_hue_0(pow0_1(max(apply_look_0(_S410.xyz, params_0.agx_0.rendering_luminance_0.xyz, _S409), vec3<f32>(0.0f)), vec3<f32>(params_0.agx_0.gamma_0)), _S410.w, _S409.hue_mix_0)) * (mat3x3<f32>(params_0.agx_0.rendering_to_pipe_r0_0.xyz, params_0.agx_0.rendering_to_pipe_r1_0.xyz, params_0.agx_0.rendering_to_pipe_r2_0.xyz))));
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_0 : vec3<u32>)
{
    var _S411 : u32 = id_0.x;
    var _S412 : bool;
    if((_S411 >= (params_0.pass_0.width_0)))
    {
        _S412 = true;
    }
    else
    {
        _S412 = ((id_0.y) >= (params_0.pass_0.height_0));
    }
    if(_S412)
    {
        return;
    }
    var i_0 : u32 = id_0.y * params_0.pass_0.width_0 + _S411;
    var pixel_0 : vec4<f32> = input_0[i_0];
    var rgb_5 : vec3<f32> = pixel_0.xyz;
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
        var _S413 : vec3<f32> = agx_1(rgb_5, agx_look_0());
        var _S414 : u32 = k_0 + u32(1);
        rgb_5 = _S413;
        k_0 = _S414;
    }
    output_0[i_0] = vec4<f32>(rgb_5, pixel_0.w);
    return;
}

fn _S415( _S416 : vec3<f32>,  _S417 : ptr<function, DiffPair_AgxLook_0>,  _S418 : vec3<f32>)
{
    var _S419 : s_paramCtx_s_bwdCallableCtx_agx_0;
    _S419.value0_22 = _S416;
    _S419.value1_4.pipe_to_base_r0_0 = params_0.agx_0.pipe_to_base_r0_0;
    _S419.value1_4.pipe_to_base_r1_0 = params_0.agx_0.pipe_to_base_r1_0;
    _S419.value1_4.pipe_to_base_r2_0 = params_0.agx_0.pipe_to_base_r2_0;
    _S419.value1_4.base_to_rendering_r0_0 = params_0.agx_0.base_to_rendering_r0_0;
    _S419.value1_4.base_to_rendering_r1_0 = params_0.agx_0.base_to_rendering_r1_0;
    _S419.value1_4.base_to_rendering_r2_0 = params_0.agx_0.base_to_rendering_r2_0;
    _S419.value1_4.rendering_to_pipe_r0_0 = params_0.agx_0.rendering_to_pipe_r0_0;
    _S419.value1_4.rendering_to_pipe_r1_0 = params_0.agx_0.rendering_to_pipe_r1_0;
    _S419.value1_4.rendering_to_pipe_r2_0 = params_0.agx_0.rendering_to_pipe_r2_0;
    _S419.value1_4.rendering_luminance_0 = params_0.agx_0.rendering_luminance_0;
    _S419.value1_4.black_ev_0 = params_0.agx_0.black_ev_0;
    _S419.value1_4.range_ev_0 = params_0.agx_0.range_ev_0;
    _S419.value1_4.gamma_0 = params_0.agx_0.gamma_0;
    _S419.value1_4.slope_0 = params_0.agx_0.slope_0;
    _S419.value1_4.intercept_0 = params_0.agx_0.intercept_0;
    _S419.value1_4.target_black_0 = params_0.agx_0.target_black_0;
    _S419.value1_4.target_white_0 = params_0.agx_0.target_white_0;
    _S419.value1_4._pad0_0 = params_0.agx_0._pad0_0;
    _S419.value1_4.toe_transition_x_0 = params_0.agx_0.toe_transition_x_0;
    _S419.value1_4.toe_transition_y_0 = params_0.agx_0.toe_transition_y_0;
    _S419.value1_4.toe_scale_0 = params_0.agx_0.toe_scale_0;
    _S419.value1_4.toe_power_0 = params_0.agx_0.toe_power_0;
    _S419.value1_4.toe_fallback_coefficient_0 = params_0.agx_0.toe_fallback_coefficient_0;
    _S419.value1_4.toe_fallback_power_0 = params_0.agx_0.toe_fallback_power_0;
    _S419.value1_4.toe_fallback_0 = params_0.agx_0.toe_fallback_0;
    _S419.value1_4._pad1_0 = params_0.agx_0._pad1_0;
    _S419.value1_4.shoulder_transition_x_0 = params_0.agx_0.shoulder_transition_x_0;
    _S419.value1_4.shoulder_transition_y_0 = params_0.agx_0.shoulder_transition_y_0;
    _S419.value1_4.shoulder_scale_0 = params_0.agx_0.shoulder_scale_0;
    _S419.value1_4.shoulder_power_0 = params_0.agx_0.shoulder_power_0;
    _S419.value1_4.shoulder_fallback_coefficient_0 = params_0.agx_0.shoulder_fallback_coefficient_0;
    _S419.value1_4.shoulder_fallback_power_0 = params_0.agx_0.shoulder_fallback_power_0;
    _S419.value1_4.shoulder_fallback_0 = params_0.agx_0.shoulder_fallback_0;
    _S419.value1_4._pad2_0 = params_0.agx_0._pad2_0;
    _S419.value1_4.look_slope_0 = params_0.agx_0.look_slope_0;
    _S419.value1_4.look_lift_0 = params_0.agx_0.look_lift_0;
    _S419.value1_4.look_power_0 = params_0.agx_0.look_power_0;
    _S419.value1_4.look_saturation_0 = params_0.agx_0.look_saturation_0;
    _S419.value1_4.look_hue_mix_0 = params_0.agx_0.look_hue_mix_0;
    _S419.value1_4._pad3_0 = params_0.agx_0._pad3_0;
    _S419.value1_4._pad4_0 = params_0.agx_0._pad4_0;
    _S419.value1_4._pad5_0 = params_0.agx_0._pad5_0;
    _S419.value2_3 = (*_S417).primal_0;
    var _S420 : s_bwdCallableCtx_agx_0 = s_bwdCallableCtx_agx_0( _S419 );
    var _S421 : AgxLook_0 = (*_S417).differential_0;
    s_bwdProp_agx_0(_S420, &(_S421), _S418);
    (*_S417).differential_0 = _S421;
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn agx_backward(@builtin(global_invocation_id) id_1 : vec3<u32>)
{
    var _S422 : u32 = id_1.x;
    var _S423 : bool;
    if((_S422 >= (params_0.pass_0.width_0)))
    {
        _S423 = true;
    }
    else
    {
        _S423 = ((id_1.y) >= (params_0.pass_0.height_0));
    }
    if(_S423)
    {
        return;
    }
    var i_1 : u32 = id_1.y * params_0.pass_0.width_0 + _S422;
    var _S424 : AgxLook_0 = AgxLook_x24_syn_dzero_0();
    var look_2 : DiffPair_AgxLook_0;
    look_2.primal_0 = agx_look_0();
    look_2.differential_0 = _S424;
    _S415(input_0[i_1].xyz, &(look_2), derivative_in_0[i_1].xyz);
    derivative_out_0[i_1] = vec4<f32>(look_2.differential_0.slope_1, look_2.differential_0.power_0, look_2.differential_0.saturation_0, look_2.differential_0.hue_mix_0);
    return;
}

fn s_fwd_agx_0( _S425 : vec3<f32>,  _S426 : DiffPair_AgxLook_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S427 : vec4<f32> = agx_tone_curve_1(_S425);
    var _S428 : vec3<f32> = vec3<f32>(0.0f);
    var _S429 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S427.xyz, _S428 );
    var _S430 : DiffPair_AgxLook_0 = DiffPair_AgxLook_0( _S426.primal_0, _S426.differential_0 );
    var _S431 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_apply_look_0(_S429, params_0.agx_0.rendering_luminance_0.xyz, _S430);
    var _S432 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S431.primal_0, _S431.differential_0 );
    var _S433 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.0f), _S428 );
    var _S434 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_max_vector_1(_S432, _S433);
    var _S435 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S434.primal_0, _S434.differential_0 );
    var _S436 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(params_0.agx_0.gamma_0), _S428 );
    var _S437 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_pow0_0(_S435, _S436);
    var _S438 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S437.primal_0, _S437.differential_0 );
    var _S439 : DiffPair_float_0 = DiffPair_float_0( _S426.primal_0.hue_mix_0, _S426.differential_0.hue_mix_0 );
    var _S440 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_restore_hue_0(_S438, _S427.w, _S439);
    var rendering_to_pipe_0 : mat3x3<f32> = mat3x3<f32>(params_0.agx_0.rendering_to_pipe_r0_0.xyz, params_0.agx_0.rendering_to_pipe_r1_0.xyz, params_0.agx_0.rendering_to_pipe_r2_0.xyz);
    var _S441 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( (((_S440.primal_0) * (rendering_to_pipe_0))), (((_S440.primal_0) * (mat3x3<f32>(_S428, _S428, _S428)))) + (((_S440.differential_0) * (rendering_to_pipe_0))) );
    return _S441;
}

@compute
@workgroup_size(16, 16, 1)
fn agx_forward_diff(@builtin(global_invocation_id) id_2 : vec3<u32>)
{
    var _S442 : u32 = id_2.x;
    var _S443 : bool;
    if((_S442 >= (params_0.pass_0.width_0)))
    {
        _S443 = true;
    }
    else
    {
        _S443 = ((id_2.y) >= (params_0.pass_0.height_0));
    }
    if(_S443)
    {
        return;
    }
    var i_2 : u32 = id_2.y * params_0.pass_0.width_0 + _S442;
    var v_7 : vec4<f32> = derivative_in_0[i_2];
    var direction_0 : AgxLook_0;
    direction_0.slope_1 = v_7.x;
    direction_0.lift_0 = 0.0f;
    direction_0.power_0 = v_7.y;
    direction_0.saturation_0 = v_7.z;
    direction_0.hue_mix_0 = v_7.w;
    var _S444 : DiffPair_AgxLook_0 = DiffPair_AgxLook_0( agx_look_0(), direction_0 );
    derivative_out_0[i_2] = vec4<f32>(s_fwd_agx_0(input_0[i_2].xyz, _S444).differential_0, 0.0f);
    return;
}

