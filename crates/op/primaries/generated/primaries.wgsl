// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2023-2025  darktable developers (ported from src/iop/primaries.c)
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

// Generated from slang/primaries.slang by tools/build-kernels.sh. Do not edit.

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

struct PrimariesParams_std140_0
{
    @align(16) matrix_0_0 : vec4<f32>,
    @align(16) matrix_1_0 : vec4<f32>,
    @align(16) matrix_2_0 : vec4<f32>,
    @align(16) hue_0 : vec4<f32>,
    @align(16) purity_0 : vec4<f32>,
    @align(16) white_and_red_0 : vec4<f32>,
    @align(16) green_and_blue_0 : vec4<f32>,
    @align(16) to_rgb_0_0 : vec4<f32>,
    @align(16) to_rgb_1_0 : vec4<f32>,
    @align(16) to_rgb_2_0 : vec4<f32>,
    @align(16) gradient_0 : vec4<u32>,
    @align(16) singular_0 : u32,
    @align(4) pad0_0 : u32,
    @align(8) pad1_0 : u32,
    @align(4) pad2_0 : u32,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) primaries_0 : PrimariesParams_std140_0,
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

fn s_apply_min_0( x_0 : f32,  y_0 : f32) -> Tuple_0
{
    var _S1 : Tuple_0 = Tuple_0( min(x_0, y_0) );
    return _S1;
}

struct s_bwdCallableCtx_min_0
{
     _S2 : f32,
     _S3 : f32,
};

struct DiffPair_float_0
{
     primal_0 : f32,
     differential_0 : f32,
};

fn _d_min_0( dpx_0 : ptr<function, DiffPair_float_0>,  dpy_0 : ptr<function, DiffPair_float_0>,  dOut_0 : f32)
{
    var _S4 : DiffPair_float_0 = (*dpx_0);
    var _S5 : f32;
    if((((*dpx_0).primal_0) < ((*dpy_0).primal_0)))
    {
        _S5 = dOut_0;
    }
    else
    {
        if((((*dpx_0).primal_0) > ((*dpy_0).primal_0)))
        {
            _S5 = 0.0f;
        }
        else
        {
            _S5 = 0.5f * dOut_0;
        }
    }
    (*dpx_0).primal_0 = _S4.primal_0;
    (*dpx_0).differential_0 = _S5;
    var _S6 : DiffPair_float_0 = (*dpy_0);
    if((((*dpy_0).primal_0) < (_S4.primal_0)))
    {
        _S5 = dOut_0;
    }
    else
    {
        if((((*dpy_0).primal_0) > ((*dpx_0).primal_0)))
        {
            _S5 = 0.0f;
        }
        else
        {
            _S5 = 0.5f * dOut_0;
        }
    }
    (*dpy_0).primal_0 = _S6.primal_0;
    (*dpy_0).differential_0 = _S5;
    return;
}

fn s_bwdProp_min_0( ctx_0 : s_bwdCallableCtx_min_0,  d_x_0 : ptr<function, f32>,  d_y_0 : ptr<function, f32>,  d_out_0 : f32)
{
    var _S7 : DiffPair_float_0;
    _S7.primal_0 = ctx_0._S2;
    var _S8 : DiffPair_float_0;
    _S8.primal_0 = ctx_0._S3;
    _d_min_0(&(_S7), &(_S8), d_out_0);
    (*d_x_0) = _S7.differential_0;
    (*d_y_0) = _S8.differential_0;
    return;
}

struct Tuple_1
{
     value0_1 : f32,
};

fn s_apply_sin_0( x_1 : f32) -> Tuple_1
{
    var _S9 : Tuple_1 = Tuple_1( sin(x_1) );
    return _S9;
}

struct s_bwdCallableCtx_sin_0
{
     _S10 : f32,
};

fn _d_sin_0( dpx_1 : ptr<function, DiffPair_float_0>,  dOut_1 : f32)
{
    var _S11 : f32 = cos((*dpx_1).primal_0) * dOut_1;
    (*dpx_1).primal_0 = (*dpx_1).primal_0;
    (*dpx_1).differential_0 = _S11;
    return;
}

fn s_bwdProp_sin_0( ctx_1 : s_bwdCallableCtx_sin_0,  d_x_1 : ptr<function, f32>,  d_out_1 : f32)
{
    var _S12 : DiffPair_float_0;
    _S12.primal_0 = ctx_1._S10;
    _d_sin_0(&(_S12), d_out_1);
    (*d_x_1) = _S12.differential_0;
    return;
}

struct Tuple_2
{
     value0_2 : f32,
};

fn s_apply_cos_0( x_2 : f32) -> Tuple_2
{
    var _S13 : Tuple_2 = Tuple_2( cos(x_2) );
    return _S13;
}

struct s_bwdCallableCtx_cos_0
{
     _S14 : f32,
};

fn _d_cos_0( dpx_2 : ptr<function, DiffPair_float_0>,  dOut_2 : f32)
{
    var _S15 : f32 = - sin((*dpx_2).primal_0) * dOut_2;
    (*dpx_2).primal_0 = (*dpx_2).primal_0;
    (*dpx_2).differential_0 = _S15;
    return;
}

fn s_bwdProp_cos_0( ctx_2 : s_bwdCallableCtx_cos_0,  d_x_2 : ptr<function, f32>,  d_out_2 : f32)
{
    var _S16 : DiffPair_float_0;
    _S16.primal_0 = ctx_2._S14;
    _d_cos_0(&(_S16), d_out_2);
    (*d_x_2) = _S16.differential_0;
    return;
}

struct Tuple_3
{
     value0_3 : f32,
};

fn s_apply_atan2_0( y_1 : f32,  x_3 : f32) -> Tuple_3
{
    var _S17 : Tuple_3 = Tuple_3( atan2(y_1, x_3) );
    return _S17;
}

struct Tuple_4
{
     value0_4 : vec3<f32>,
};

fn s_apply_mul_0( left_0 : mat3x3<f32>,  right_0 : vec3<f32>) -> Tuple_4
{
    var _S18 : Tuple_4 = Tuple_4( (((right_0) * (left_0))) );
    return _S18;
}

struct s_bwdCallableCtx_mul_0
{
     _S19 : mat3x3<f32>,
     _S20 : vec3<f32>,
};

struct DiffPair_matrixx3Cfloatx2C3x2C3x3E_0
{
     primal_0 : mat3x3<f32>,
     differential_0 : mat3x3<f32>,
};

struct DiffPair_vectorx3Cfloatx2C3x3E_0
{
     primal_0 : vec3<f32>,
     differential_0 : vec3<f32>,
};

fn _d_mul_0( left_1 : ptr<function, DiffPair_matrixx3Cfloatx2C3x2C3x3E_0>,  right_1 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_3 : vec3<f32>)
{
    var _S21 : f32 = (*left_1).primal_0[i32(0)][i32(0)] * dOut_3[i32(0)];
    var left_d_result_0 : mat3x3<f32>;
    left_d_result_0[i32(0)][i32(0)] = (*right_1).primal_0[i32(0)] * dOut_3[i32(0)];
    var sum_0 : f32 = _S21 + (*left_1).primal_0[i32(1)][i32(0)] * dOut_3[i32(1)];
    left_d_result_0[i32(1)][i32(0)] = (*right_1).primal_0[i32(0)] * dOut_3[i32(1)];
    var sum_1 : f32 = sum_0 + (*left_1).primal_0[i32(2)][i32(0)] * dOut_3[i32(2)];
    left_d_result_0[i32(2)][i32(0)] = (*right_1).primal_0[i32(0)] * dOut_3[i32(2)];
    var right_d_result_0 : vec3<f32>;
    right_d_result_0[i32(0)] = sum_1;
    var _S22 : f32 = (*left_1).primal_0[i32(0)][i32(1)] * dOut_3[i32(0)];
    left_d_result_0[i32(0)][i32(1)] = (*right_1).primal_0[i32(1)] * dOut_3[i32(0)];
    var sum_2 : f32 = _S22 + (*left_1).primal_0[i32(1)][i32(1)] * dOut_3[i32(1)];
    left_d_result_0[i32(1)][i32(1)] = (*right_1).primal_0[i32(1)] * dOut_3[i32(1)];
    var sum_3 : f32 = sum_2 + (*left_1).primal_0[i32(2)][i32(1)] * dOut_3[i32(2)];
    left_d_result_0[i32(2)][i32(1)] = (*right_1).primal_0[i32(1)] * dOut_3[i32(2)];
    right_d_result_0[i32(1)] = sum_3;
    var _S23 : f32 = (*left_1).primal_0[i32(0)][i32(2)] * dOut_3[i32(0)];
    left_d_result_0[i32(0)][i32(2)] = (*right_1).primal_0[i32(2)] * dOut_3[i32(0)];
    var sum_4 : f32 = _S23 + (*left_1).primal_0[i32(1)][i32(2)] * dOut_3[i32(1)];
    left_d_result_0[i32(1)][i32(2)] = (*right_1).primal_0[i32(2)] * dOut_3[i32(1)];
    var sum_5 : f32 = sum_4 + (*left_1).primal_0[i32(2)][i32(2)] * dOut_3[i32(2)];
    left_d_result_0[i32(2)][i32(2)] = (*right_1).primal_0[i32(2)] * dOut_3[i32(2)];
    right_d_result_0[i32(2)] = sum_5;
    (*left_1).primal_0 = (*left_1).primal_0;
    (*left_1).differential_0 = left_d_result_0;
    (*right_1).primal_0 = (*right_1).primal_0;
    (*right_1).differential_0 = right_d_result_0;
    return;
}

fn s_bwdProp_mul_0( ctx_3 : s_bwdCallableCtx_mul_0,  d_left_0 : ptr<function, mat3x3<f32>>,  d_right_0 : ptr<function, vec3<f32>>,  d_out_3 : vec3<f32>)
{
    var _S24 : DiffPair_matrixx3Cfloatx2C3x2C3x3E_0;
    _S24.primal_0 = ctx_3._S19;
    var _S25 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S25.primal_0 = ctx_3._S20;
    _d_mul_0(&(_S24), &(_S25), d_out_3);
    (*d_left_0) = _S24.differential_0;
    (*d_right_0) = _S25.differential_0;
    return;
}

struct Tuple_5
{
     value0_5 : f32,
};

fn s_apply_dot_0( x_4 : vec3<f32>,  y_2 : vec3<f32>) -> Tuple_5
{
    var _S26 : Tuple_5 = Tuple_5( dot(x_4, y_2) );
    return _S26;
}

struct s_bwdCallableCtx_dot_0
{
     _S27 : vec3<f32>,
     _S28 : vec3<f32>,
};

fn _d_dot_0( dpx_3 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dpy_1 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_4 : f32)
{
    var x_d_result_0 : vec3<f32>;
    x_d_result_0[i32(0)] = (*dpy_1).primal_0[i32(0)] * dOut_4;
    var y_d_result_0 : vec3<f32>;
    y_d_result_0[i32(0)] = (*dpx_3).primal_0[i32(0)] * dOut_4;
    x_d_result_0[i32(1)] = (*dpy_1).primal_0[i32(1)] * dOut_4;
    y_d_result_0[i32(1)] = (*dpx_3).primal_0[i32(1)] * dOut_4;
    x_d_result_0[i32(2)] = (*dpy_1).primal_0[i32(2)] * dOut_4;
    y_d_result_0[i32(2)] = (*dpx_3).primal_0[i32(2)] * dOut_4;
    (*dpx_3).primal_0 = (*dpx_3).primal_0;
    (*dpx_3).differential_0 = x_d_result_0;
    (*dpy_1).primal_0 = (*dpy_1).primal_0;
    (*dpy_1).differential_0 = y_d_result_0;
    return;
}

fn s_bwdProp_dot_0( ctx_4 : s_bwdCallableCtx_dot_0,  d_x_3 : ptr<function, vec3<f32>>,  d_y_1 : ptr<function, vec3<f32>>,  d_out_4 : f32)
{
    var _S29 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S29.primal_0 = ctx_4._S27;
    var _S30 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S30.primal_0 = ctx_4._S28;
    _d_dot_0(&(_S29), &(_S30), d_out_4);
    (*d_x_3) = _S29.differential_0;
    (*d_y_1) = _S30.differential_0;
    return;
}

struct Tuple_6
{
     value0_6 : vec3<f32>,
};

fn s_apply_cross_0( left_2 : vec3<f32>,  right_2 : vec3<f32>) -> Tuple_6
{
    var _S31 : Tuple_6 = Tuple_6( cross(left_2, right_2) );
    return _S31;
}

struct s_bwdCallableCtx_cross_0
{
     _S32 : vec3<f32>,
     _S33 : vec3<f32>,
};

fn _d_cross_0( a_0 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  b_0 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_5 : vec3<f32>)
{
    var _S34 : f32 = dOut_5.y;
    var _S35 : f32 = dOut_5.z;
    var _S36 : f32 = dOut_5.x;
    var _S37 : f32 = (*a_0).primal_0.z * _S34 + - (*a_0).primal_0.y * _S35;
    var _S38 : f32 = - (*a_0).primal_0.z * _S36 + (*a_0).primal_0.x * _S35;
    var _S39 : f32 = (*a_0).primal_0.y * _S36 + - (*a_0).primal_0.x * _S34;
    var _S40 : vec3<f32> = vec3<f32>(- (*b_0).primal_0.z * _S34 + (*b_0).primal_0.y * _S35, (*b_0).primal_0.z * _S36 + - (*b_0).primal_0.x * _S35, - (*b_0).primal_0.y * _S36 + (*b_0).primal_0.x * _S34);
    (*a_0).primal_0 = (*a_0).primal_0;
    (*a_0).differential_0 = _S40;
    var _S41 : vec3<f32> = vec3<f32>(_S37, _S38, _S39);
    (*b_0).primal_0 = (*b_0).primal_0;
    (*b_0).differential_0 = _S41;
    return;
}

fn s_bwdProp_cross_0( ctx_5 : s_bwdCallableCtx_cross_0,  d_left_1 : ptr<function, vec3<f32>>,  d_right_1 : ptr<function, vec3<f32>>,  d_out_5 : vec3<f32>)
{
    var _S42 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S42.primal_0 = ctx_5._S32;
    var _S43 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S43.primal_0 = ctx_5._S33;
    _d_cross_0(&(_S42), &(_S43), d_out_5);
    (*d_left_1) = _S42.differential_0;
    (*d_right_1) = _S43.differential_0;
    return;
}

struct Settings_0
{
     hue_1 : vec4<f32>,
     purity_1 : vec4<f32>,
};

struct DiffPair_Settings_0
{
     primal_0 : Settings_0,
     differential_0 : Settings_0,
};

struct Profile_0
{
     white_0 : vec2<f32>,
     red_0 : vec2<f32>,
     green_0 : vec2<f32>,
     blue_0 : vec2<f32>,
     to_rgb_0 : mat3x3<f32>,
     singular_1 : bool,
};

struct s_paramCtx_s_bwdCallableCtx_primaries_from_settings_0
{
     value0_7 : vec3<f32>,
     value1_0 : Settings_0,
     value2_0 : Profile_0,
};

struct s_bwdCallableCtx_primaries_from_settings_0
{
     value0_8 : s_paramCtx_s_bwdCallableCtx_primaries_from_settings_0,
};

struct Tuple_7
{
     value0_9 : mat3x3<f32>,
};

struct Tuple_8
{
     value0_10 : vec2<f32>,
};

struct Tuple_9
{
     value0_11 : f32,
};

struct Tuple_10
{
     value0_12 : f32,
};

fn s_apply_determinant_0( _S44 : f32,  _S45 : f32,  _S46 : f32,  _S47 : f32) -> Tuple_10
{
    var _S48 : Tuple_10;
    _S48.value0_12 = _S44 * _S47 - _S45 * _S46;
    return _S48;
}

fn s_apply_edge_distance_0( white_1 : vec2<f32>,  _S49 : vec2<f32>,  a_1 : vec2<f32>,  b_1 : vec2<f32>) -> Tuple_9
{
    var _S50 : f32 = a_1.x;
    var _S51 : f32 = _S50 - b_1.x;
    var _S52 : f32 = a_1.y;
    var _S53 : f32 = _S52 - b_1.y;
    var _S54 : Tuple_10 = s_apply_determinant_0(- _S49.x, _S51, - _S49.y, _S53);
    var _S55 : f32;
    if(!((_S54.value0_12) == 0.0f))
    {
        var t_0 : f32 = s_apply_determinant_0(white_1.x - _S50, _S51, white_1.y - _S52, _S53).value0_12 / _S54.value0_12;
        if((t_0 >= 0.0f))
        {
            _S55 = t_0;
        }
        else
        {
            _S55 = 3.4028234663852886e+38f;
        }
    }
    else
    {
        _S55 = 3.4028234663852886e+38f;
    }
    var _S56 : Tuple_9;
    _S56.value0_11 = _S55;
    return _S56;
}

fn s_apply_rotate_and_scale_0( profile_0 : Profile_0,  primary_0 : vec2<f32>,  _S57 : f32,  _S58 : f32) -> Tuple_8
{
    var angle_0 : f32 = s_apply_atan2_0(primary_0.y - profile_0.white_0.y, primary_0.x - profile_0.white_0.x).value0_3 + _S58;
    var direction_0 : vec2<f32> = vec2<f32>(s_apply_cos_0(angle_0).value0_2, s_apply_sin_0(angle_0).value0_1);
    var _S59 : Tuple_8;
    _S59.value0_10 = profile_0.white_0 + vec2<f32>((_S57 * s_apply_min_0(s_apply_min_0(s_apply_edge_distance_0(profile_0.white_0, direction_0, profile_0.red_0, profile_0.green_0).value0_11, s_apply_edge_distance_0(profile_0.white_0, direction_0, profile_0.green_0, profile_0.blue_0).value0_11).value0_0, s_apply_edge_distance_0(profile_0.white_0, direction_0, profile_0.blue_0, profile_0.red_0).value0_11).value0_0)) * direction_0;
    return _S59;
}

struct Tuple_11
{
     value0_13 : vec3<f32>,
};

struct Tuple_12
{
     value0_14 : f32,
};

fn s_apply_sanitize_y_0( _S60 : f32) -> Tuple_12
{
    var _runFlag_0 : bool;
    if((_S60 < 1.1920928955078125e-07f))
    {
        _runFlag_0 = (_S60 >= 0.0f);
    }
    else
    {
        _runFlag_0 = false;
    }
    var _S61 : bool = !_runFlag_0;
    var _S62 : f32;
    if(_S61)
    {
        if((_S60 < 0.0f))
        {
            _runFlag_0 = (_S60 > -1.1920928955078125e-07f);
        }
        else
        {
            _runFlag_0 = false;
        }
        if(_runFlag_0)
        {
            _runFlag_0 = false;
            _S62 = -1.1920928955078125e-07f;
        }
        else
        {
            _runFlag_0 = _S61;
            _S62 = 1.1920928955078125e-07f;
        }
        if(_runFlag_0)
        {
            _S62 = _S60;
        }
    }
    else
    {
        _S62 = 1.1920928955078125e-07f;
    }
    var _S63 : Tuple_12;
    _S63.value0_14 = _S62;
    return _S63;
}

fn s_apply_chromaticity_to_xyz_0( _S64 : vec2<f32>) -> Tuple_11
{
    var _S65 : Tuple_12 = s_apply_sanitize_y_0(_S64.y);
    var _S66 : f32 = _S64.x;
    var _S67 : Tuple_11;
    _S67.value0_13 = vec3<f32>(_S66 / _S65.value0_14, 1.0f, (1.0f - _S66 - _S65.value0_14) / _S65.value0_14);
    return _S67;
}

fn s_apply_adjustment_0( _S68 : Settings_0,  profile_1 : Profile_0) -> Tuple_7
{
    var _S69 : Tuple_11 = s_apply_chromaticity_to_xyz_0(s_apply_rotate_and_scale_0(profile_1, profile_1.red_0, _S68.purity_1.y, _S68.hue_1.y).value0_10);
    var _S70 : Tuple_11 = s_apply_chromaticity_to_xyz_0(s_apply_rotate_and_scale_0(profile_1, profile_1.green_0, _S68.purity_1.z, _S68.hue_1.z).value0_10);
    var _S71 : Tuple_11 = s_apply_chromaticity_to_xyz_0(s_apply_rotate_and_scale_0(profile_1, profile_1.blue_0, _S68.purity_1.w, _S68.hue_1.w).value0_10);
    var _S72 : Tuple_11 = s_apply_chromaticity_to_xyz_0(s_apply_rotate_and_scale_0(profile_1, profile_1.red_0, _S68.purity_1.x, _S68.hue_1.x).value0_10);
    var _S73 : Tuple_6 = s_apply_cross_0(_S70.value0_13, _S71.value0_13);
    var _S74 : Tuple_6 = s_apply_cross_0(_S71.value0_13, _S69.value0_13);
    var _S75 : Tuple_6 = s_apply_cross_0(_S69.value0_13, _S70.value0_13);
    var _S76 : Tuple_5 = s_apply_dot_0(_S69.value0_13, _S73.value0_6);
    var _S77 : vec3<f32> = vec3<f32>(0.0f);
    var _S78 : bool;
    if(!profile_1.singular_1)
    {
        _S78 = (_S76.value0_5) != 0.0f;
    }
    else
    {
        _S78 = false;
    }
    var scale_1 : vec3<f32>;
    if(_S78)
    {
        scale_1 = vec3<f32>(s_apply_dot_0(_S73.value0_6, _S72.value0_13).value0_5, s_apply_dot_0(_S74.value0_6, _S72.value0_13).value0_5, s_apply_dot_0(_S75.value0_6, _S72.value0_13).value0_5) / vec3<f32>(_S76.value0_5);
    }
    else
    {
        scale_1 = _S77;
    }
    var _S79 : Tuple_4 = s_apply_mul_0(profile_1.to_rgb_0, _S69.value0_13 * vec3<f32>(scale_1.x));
    var _S80 : Tuple_4 = s_apply_mul_0(profile_1.to_rgb_0, _S70.value0_13 * vec3<f32>(scale_1.y));
    var _S81 : Tuple_4 = s_apply_mul_0(profile_1.to_rgb_0, _S71.value0_13 * vec3<f32>(scale_1.z));
    var _S82 : Tuple_7;
    _S82.value0_9 = mat3x3<f32>(vec3<f32>(_S79.value0_4.x, _S80.value0_4.x, _S81.value0_4.x), vec3<f32>(_S79.value0_4.y, _S80.value0_4.y, _S81.value0_4.y), vec3<f32>(_S79.value0_4.z, _S80.value0_4.z, _S81.value0_4.z));
    return _S82;
}

struct s_paramCtx_s_bwdCallableCtx_adjustment_0
{
     value0_15 : Settings_0,
     value1_1 : Profile_0,
};

struct s_bwdCallableCtx_adjustment_0
{
     value0_16 : s_paramCtx_s_bwdCallableCtx_adjustment_0,
};

struct s_paramCtx_s_bwdCallableCtx_primaries_0
{
     value0_17 : vec3<f32>,
     value1_2 : mat3x3<f32>,
};

struct s_bwdCallableCtx_primaries_0
{
     value0_18 : s_paramCtx_s_bwdCallableCtx_primaries_0,
};

fn Settings_x24_syn_dzero_0() -> Settings_0
{
    var result_0 : Settings_0;
    var _S83 : vec4<f32> = vec4<f32>(0.0f);
    result_0.hue_1 = _S83;
    result_0.purity_1 = _S83;
    return result_0;
}

fn s_bwdProp_primaries_0( _S84 : s_bwdCallableCtx_primaries_0,  _S85 : ptr<function, mat3x3<f32>>,  _S86 : vec3<f32>)
{
    var _S87 : s_bwdCallableCtx_mul_0;
    _S87._S19 = _S84.value0_18.value1_2;
    _S87._S20 = _S84.value0_18.value0_17;
    var _S88 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S89 : vec3<f32> = vec3<f32>(0.0f);
    s_bwdProp_mul_0(_S87, &(_S88), &(_S89), _S86);
    (*_S85) = _S88;
    return;
}

struct s_paramCtx_s_bwdCallableCtx_rotate_and_scale_0
{
     value0_19 : Profile_0,
     value1_3 : vec2<f32>,
     value2_1 : f32,
     value3_0 : f32,
};

struct s_bwdCallableCtx_rotate_and_scale_0
{
     value0_20 : s_paramCtx_s_bwdCallableCtx_rotate_and_scale_0,
};

struct s_paramCtx_s_bwdCallableCtx_chromaticity_to_xyz_0
{
     value0_21 : vec2<f32>,
};

struct s_bwdCallableCtx_chromaticity_to_xyz_0
{
     value0_22 : s_paramCtx_s_bwdCallableCtx_chromaticity_to_xyz_0,
};

struct s_paramCtx_s_bwdCallableCtx_sanitize_y_0
{
     value0_23 : f32,
};

struct s_bwdCallableCtx_sanitize_y_0
{
     value0_24 : s_paramCtx_s_bwdCallableCtx_sanitize_y_0,
};

fn s_bwdProp_sanitize_y_0( _S90 : s_bwdCallableCtx_sanitize_y_0,  _S91 : ptr<function, f32>,  _S92 : f32)
{
    var _runFlag_1 : bool;
    if(((_S90.value0_24.value0_23) < 1.1920928955078125e-07f))
    {
        _runFlag_1 = ((_S90.value0_24.value0_23) >= 0.0f);
    }
    else
    {
        _runFlag_1 = false;
    }
    var _S93 : bool = !_runFlag_1;
    if(_S93)
    {
        if(((_S90.value0_24.value0_23) < 0.0f))
        {
            _runFlag_1 = ((_S90.value0_24.value0_23) > -1.1920928955078125e-07f);
        }
        else
        {
            _runFlag_1 = false;
        }
        if(_runFlag_1)
        {
            _runFlag_1 = false;
        }
        else
        {
            _runFlag_1 = _S93;
        }
    }
    else
    {
        _runFlag_1 = false;
    }
    var _S94 : f32;
    if(_S93)
    {
        if(_runFlag_1)
        {
            _S94 = _S92;
        }
        else
        {
            _S94 = 0.0f;
        }
    }
    else
    {
        _S94 = 0.0f;
    }
    (*_S91) = _S94;
    return;
}

fn s_bwdProp_chromaticity_to_xyz_0( _S95 : s_bwdCallableCtx_chromaticity_to_xyz_0,  _S96 : ptr<function, vec2<f32>>,  _S97 : vec3<f32>)
{
    var _S98 : f32 = _S95.value0_22.value0_21.y;
    var _S99 : Tuple_12 = s_apply_sanitize_y_0(_S98);
    var _S100 : s_paramCtx_s_bwdCallableCtx_sanitize_y_0;
    _S100.value0_23 = _S98;
    var _S101 : s_bwdCallableCtx_sanitize_y_0 = s_bwdCallableCtx_sanitize_y_0( _S100 );
    var _S102 : f32 = _S95.value0_22.value0_21.x;
    var _S103 : f32 = _S99.value0_14 * _S99.value0_14;
    var _S104 : f32 = _S97[i32(2)] / _S103;
    var _S105 : f32 = - (_S99.value0_14 * _S104);
    var _S106 : f32 = _S97[i32(0)] / _S103;
    var _S107 : f32 = _S105 + _S99.value0_14 * _S106;
    var _S108 : f32 = (1.0f - _S102 - _S99.value0_14) * - _S104 + _S105 + _S102 * - _S106;
    var _S109 : f32 = 0.0f;
    s_bwdProp_sanitize_y_0(_S101, &(_S109), _S108);
    (*_S96) = vec2<f32>(_S107, _S109);
    return;
}

struct s_paramCtx_s_bwdCallableCtx_edge_distance_0
{
     value0_25 : vec2<f32>,
     value1_4 : vec2<f32>,
     value2_2 : vec2<f32>,
     value3_1 : vec2<f32>,
};

struct s_bwdCallableCtx_edge_distance_0
{
     value0_26 : s_paramCtx_s_bwdCallableCtx_edge_distance_0,
};

struct s_paramCtx_s_bwdCallableCtx_determinant_0
{
     value0_27 : f32,
     value1_5 : f32,
     value2_3 : f32,
     value3_2 : f32,
};

struct s_bwdCallableCtx_determinant_0
{
     value0_28 : s_paramCtx_s_bwdCallableCtx_determinant_0,
};

fn s_bwdProp_determinant_0( _S110 : s_bwdCallableCtx_determinant_0,  _S111 : ptr<function, f32>,  _S112 : ptr<function, f32>,  _S113 : ptr<function, f32>,  _S114 : ptr<function, f32>,  _S115 : f32)
{
    var _S116 : f32 = - _S115;
    var _S117 : f32 = _S110.value0_28.value1_5 * _S116;
    var _S118 : f32 = _S110.value0_28.value2_3 * _S116;
    var _S119 : f32 = _S110.value0_28.value0_27 * _S115;
    (*_S111) = _S110.value0_28.value3_2 * _S115;
    (*_S112) = _S118;
    (*_S113) = _S117;
    (*_S114) = _S119;
    return;
}

fn s_bwdProp_edge_distance_0( _S120 : s_bwdCallableCtx_edge_distance_0,  _S121 : ptr<function, vec2<f32>>,  _S122 : f32)
{
    var _S123 : f32 = - _S120.value0_26.value1_4.x;
    var _S124 : f32 = _S120.value0_26.value2_2.x;
    var _S125 : f32 = _S124 - _S120.value0_26.value3_1.x;
    var _S126 : f32 = - _S120.value0_26.value1_4.y;
    var _S127 : f32 = _S120.value0_26.value2_2.y;
    var _S128 : f32 = _S127 - _S120.value0_26.value3_1.y;
    var _S129 : Tuple_10 = s_apply_determinant_0(_S123, _S125, _S126, _S128);
    var _S130 : s_paramCtx_s_bwdCallableCtx_determinant_0;
    _S130.value0_27 = _S123;
    _S130.value1_5 = _S125;
    _S130.value2_3 = _S126;
    _S130.value3_2 = _S128;
    var _S131 : s_bwdCallableCtx_determinant_0 = s_bwdCallableCtx_determinant_0( _S130 );
    var _S132 : bool = !((_S129.value0_12) == 0.0f);
    var _S133 : bool;
    var _S134 : f32;
    var _S135 : f32;
    if(_S132)
    {
        var _S136 : Tuple_10 = s_apply_determinant_0(_S120.value0_26.value0_25.x - _S124, _S125, _S120.value0_26.value0_25.y - _S127, _S128);
        var _S137 : f32 = _S129.value0_12 * _S129.value0_12;
        _S133 = ((_S136.value0_12 / _S129.value0_12) >= 0.0f);
        _S134 = _S137;
        _S135 = _S136.value0_12;
    }
    else
    {
        _S133 = false;
        _S134 = 0.0f;
        _S135 = 0.0f;
    }
    if(_S132)
    {
        var _S138 : f32;
        if(_S133)
        {
            _S138 = _S122;
        }
        else
        {
            _S138 = 0.0f;
        }
        _S134 = _S135 * - (_S138 / _S134);
    }
    else
    {
        _S134 = 0.0f;
    }
    var _S139 : f32 = 0.0f;
    var _S140 : f32 = 0.0f;
    var _S141 : f32 = 0.0f;
    var _S142 : f32 = 0.0f;
    s_bwdProp_determinant_0(_S131, &(_S139), &(_S140), &(_S141), &(_S142), _S134);
    (*_S121) = vec2<f32>(- _S139, - _S141);
    return;
}

fn s_bwdProp_rotate_and_scale_0( _S143 : s_bwdCallableCtx_rotate_and_scale_0,  _S144 : ptr<function, f32>,  _S145 : ptr<function, f32>,  _S146 : vec2<f32>)
{
    var angle_1 : f32 = s_apply_atan2_0(_S143.value0_20.value1_3.y - _S143.value0_20.value0_19.white_0.y, _S143.value0_20.value1_3.x - _S143.value0_20.value0_19.white_0.x).value0_3 + _S143.value0_20.value3_0;
    var _S147 : Tuple_2 = s_apply_cos_0(angle_1);
    var _S148 : s_bwdCallableCtx_cos_0;
    _S148._S14 = angle_1;
    var _S149 : Tuple_1 = s_apply_sin_0(angle_1);
    var _S150 : s_bwdCallableCtx_sin_0;
    _S150._S10 = angle_1;
    var direction_1 : vec2<f32> = vec2<f32>(_S147.value0_2, _S149.value0_1);
    var _S151 : Tuple_9 = s_apply_edge_distance_0(_S143.value0_20.value0_19.white_0, direction_1, _S143.value0_20.value0_19.red_0, _S143.value0_20.value0_19.green_0);
    var _S152 : s_paramCtx_s_bwdCallableCtx_edge_distance_0;
    _S152.value0_25 = _S143.value0_20.value0_19.white_0;
    _S152.value1_4 = direction_1;
    _S152.value2_2 = _S143.value0_20.value0_19.red_0;
    _S152.value3_1 = _S143.value0_20.value0_19.green_0;
    var _S153 : s_bwdCallableCtx_edge_distance_0 = s_bwdCallableCtx_edge_distance_0( _S152 );
    var _S154 : Tuple_9 = s_apply_edge_distance_0(_S143.value0_20.value0_19.white_0, direction_1, _S143.value0_20.value0_19.green_0, _S143.value0_20.value0_19.blue_0);
    var _S155 : s_paramCtx_s_bwdCallableCtx_edge_distance_0;
    _S155.value0_25 = _S143.value0_20.value0_19.white_0;
    _S155.value1_4 = direction_1;
    _S155.value2_2 = _S143.value0_20.value0_19.green_0;
    _S155.value3_1 = _S143.value0_20.value0_19.blue_0;
    var _S156 : s_bwdCallableCtx_edge_distance_0 = s_bwdCallableCtx_edge_distance_0( _S155 );
    var _S157 : Tuple_0 = s_apply_min_0(_S151.value0_11, _S154.value0_11);
    var _S158 : s_bwdCallableCtx_min_0;
    _S158._S2 = _S151.value0_11;
    _S158._S3 = _S154.value0_11;
    var _S159 : Tuple_9 = s_apply_edge_distance_0(_S143.value0_20.value0_19.white_0, direction_1, _S143.value0_20.value0_19.blue_0, _S143.value0_20.value0_19.red_0);
    var _S160 : s_paramCtx_s_bwdCallableCtx_edge_distance_0;
    _S160.value0_25 = _S143.value0_20.value0_19.white_0;
    _S160.value1_4 = direction_1;
    _S160.value2_2 = _S143.value0_20.value0_19.blue_0;
    _S160.value3_1 = _S143.value0_20.value0_19.red_0;
    var _S161 : s_bwdCallableCtx_edge_distance_0 = s_bwdCallableCtx_edge_distance_0( _S160 );
    var _S162 : Tuple_0 = s_apply_min_0(_S157.value0_0, _S159.value0_11);
    var _S163 : s_bwdCallableCtx_min_0;
    _S163._S2 = _S157.value0_0;
    _S163._S3 = _S159.value0_11;
    var _S164 : vec2<f32> = vec2<f32>((_S143.value0_20.value2_1 * _S162.value0_0)) * _S146;
    var _S165 : vec2<f32> = direction_1 * _S146;
    var _S166 : f32 = _S165[i32(0)] + _S165[i32(1)];
    var _S167 : f32 = _S143.value0_20.value2_1 * _S166;
    var _S168 : f32 = _S162.value0_0 * _S166;
    var _S169 : f32 = 0.0f;
    var _S170 : f32 = 0.0f;
    s_bwdProp_min_0(_S163, &(_S169), &(_S170), _S167);
    var _S171 : vec2<f32> = vec2<f32>(0.0f);
    var _S172 : vec2<f32> = _S171;
    s_bwdProp_edge_distance_0(_S161, &(_S172), _S170);
    var _S173 : f32 = 0.0f;
    var _S174 : f32 = 0.0f;
    s_bwdProp_min_0(_S158, &(_S173), &(_S174), _S169);
    var _S175 : vec2<f32> = _S171;
    s_bwdProp_edge_distance_0(_S156, &(_S175), _S174);
    var _S176 : vec2<f32> = _S171;
    s_bwdProp_edge_distance_0(_S153, &(_S176), _S173);
    var _S177 : vec2<f32> = _S164 + _S172 + _S175 + _S176;
    var _S178 : f32 = 0.0f;
    s_bwdProp_sin_0(_S150, &(_S178), _S177[i32(1)]);
    var _S179 : f32 = 0.0f;
    s_bwdProp_cos_0(_S148, &(_S179), _S177[i32(0)]);
    var _S180 : f32 = _S178 + _S179;
    (*_S144) = _S168;
    (*_S145) = _S180;
    return;
}

fn Settings_x24_syn_dadd_0( SLANG_anonymous_0_0 : Settings_0,  SLANG_anonymous_1_0 : Settings_0) -> Settings_0
{
    var result_1 : Settings_0;
    result_1.hue_1 = SLANG_anonymous_0_0.hue_1 + SLANG_anonymous_1_0.hue_1;
    result_1.purity_1 = SLANG_anonymous_0_0.purity_1 + SLANG_anonymous_1_0.purity_1;
    return result_1;
}

fn s_bwdProp_adjustment_0( _S181 : s_bwdCallableCtx_adjustment_0,  _S182 : ptr<function, Settings_0>,  _S183 : mat3x3<f32>)
{
    var _S184 : vec3<f32> = vec3<f32>(0.0f);
    var _S185 : f32 = _S181.value0_16.value0_15.purity_1.y;
    var _S186 : f32 = _S181.value0_16.value0_15.hue_1.y;
    var _S187 : Tuple_8 = s_apply_rotate_and_scale_0(_S181.value0_16.value1_1, _S181.value0_16.value1_1.red_0, _S185, _S186);
    var _S188 : s_paramCtx_s_bwdCallableCtx_rotate_and_scale_0;
    _S188.value0_19 = _S181.value0_16.value1_1;
    _S188.value1_3 = _S181.value0_16.value1_1.red_0;
    _S188.value2_1 = _S185;
    _S188.value3_0 = _S186;
    var _S189 : s_bwdCallableCtx_rotate_and_scale_0 = s_bwdCallableCtx_rotate_and_scale_0( _S188 );
    var _S190 : Tuple_11 = s_apply_chromaticity_to_xyz_0(_S187.value0_10);
    var _S191 : s_paramCtx_s_bwdCallableCtx_chromaticity_to_xyz_0;
    _S191.value0_21 = _S187.value0_10;
    var _S192 : s_bwdCallableCtx_chromaticity_to_xyz_0 = s_bwdCallableCtx_chromaticity_to_xyz_0( _S191 );
    var _S193 : f32 = _S181.value0_16.value0_15.purity_1.z;
    var _S194 : f32 = _S181.value0_16.value0_15.hue_1.z;
    var _S195 : Tuple_8 = s_apply_rotate_and_scale_0(_S181.value0_16.value1_1, _S181.value0_16.value1_1.green_0, _S193, _S194);
    var _S196 : s_paramCtx_s_bwdCallableCtx_rotate_and_scale_0;
    _S196.value0_19 = _S181.value0_16.value1_1;
    _S196.value1_3 = _S181.value0_16.value1_1.green_0;
    _S196.value2_1 = _S193;
    _S196.value3_0 = _S194;
    var _S197 : s_bwdCallableCtx_rotate_and_scale_0 = s_bwdCallableCtx_rotate_and_scale_0( _S196 );
    var _S198 : Tuple_11 = s_apply_chromaticity_to_xyz_0(_S195.value0_10);
    var _S199 : s_paramCtx_s_bwdCallableCtx_chromaticity_to_xyz_0;
    _S199.value0_21 = _S195.value0_10;
    var _S200 : s_bwdCallableCtx_chromaticity_to_xyz_0 = s_bwdCallableCtx_chromaticity_to_xyz_0( _S199 );
    var _S201 : f32 = _S181.value0_16.value0_15.purity_1.w;
    var _S202 : f32 = _S181.value0_16.value0_15.hue_1.w;
    var _S203 : Tuple_8 = s_apply_rotate_and_scale_0(_S181.value0_16.value1_1, _S181.value0_16.value1_1.blue_0, _S201, _S202);
    var _S204 : s_paramCtx_s_bwdCallableCtx_rotate_and_scale_0;
    _S204.value0_19 = _S181.value0_16.value1_1;
    _S204.value1_3 = _S181.value0_16.value1_1.blue_0;
    _S204.value2_1 = _S201;
    _S204.value3_0 = _S202;
    var _S205 : s_bwdCallableCtx_rotate_and_scale_0 = s_bwdCallableCtx_rotate_and_scale_0( _S204 );
    var _S206 : Tuple_11 = s_apply_chromaticity_to_xyz_0(_S203.value0_10);
    var _S207 : s_paramCtx_s_bwdCallableCtx_chromaticity_to_xyz_0;
    _S207.value0_21 = _S203.value0_10;
    var _S208 : s_bwdCallableCtx_chromaticity_to_xyz_0 = s_bwdCallableCtx_chromaticity_to_xyz_0( _S207 );
    var _S209 : f32 = _S181.value0_16.value0_15.purity_1.x;
    var _S210 : f32 = _S181.value0_16.value0_15.hue_1.x;
    var _S211 : Tuple_8 = s_apply_rotate_and_scale_0(_S181.value0_16.value1_1, _S181.value0_16.value1_1.red_0, _S209, _S210);
    var _S212 : s_paramCtx_s_bwdCallableCtx_rotate_and_scale_0;
    _S212.value0_19 = _S181.value0_16.value1_1;
    _S212.value1_3 = _S181.value0_16.value1_1.red_0;
    _S212.value2_1 = _S209;
    _S212.value3_0 = _S210;
    var _S213 : s_bwdCallableCtx_rotate_and_scale_0 = s_bwdCallableCtx_rotate_and_scale_0( _S212 );
    var _S214 : Tuple_11 = s_apply_chromaticity_to_xyz_0(_S211.value0_10);
    var _S215 : s_paramCtx_s_bwdCallableCtx_chromaticity_to_xyz_0;
    _S215.value0_21 = _S211.value0_10;
    var _S216 : s_bwdCallableCtx_chromaticity_to_xyz_0 = s_bwdCallableCtx_chromaticity_to_xyz_0( _S215 );
    var _S217 : Tuple_6 = s_apply_cross_0(_S198.value0_13, _S206.value0_13);
    var _S218 : s_bwdCallableCtx_cross_0;
    _S218._S32 = _S198.value0_13;
    _S218._S33 = _S206.value0_13;
    var _S219 : s_bwdCallableCtx_cross_0 = _S218;
    var _S220 : Tuple_6 = s_apply_cross_0(_S206.value0_13, _S190.value0_13);
    var _S221 : s_bwdCallableCtx_cross_0;
    _S221._S32 = _S206.value0_13;
    _S221._S33 = _S190.value0_13;
    var _S222 : s_bwdCallableCtx_cross_0 = _S221;
    var _S223 : Tuple_6 = s_apply_cross_0(_S190.value0_13, _S198.value0_13);
    var _S224 : s_bwdCallableCtx_cross_0;
    _S224._S32 = _S190.value0_13;
    _S224._S33 = _S198.value0_13;
    var _S225 : s_bwdCallableCtx_cross_0 = _S224;
    var _S226 : Tuple_5 = s_apply_dot_0(_S190.value0_13, _S217.value0_6);
    var _S227 : s_bwdCallableCtx_dot_0;
    _S227._S27 = _S190.value0_13;
    _S227._S28 = _S217.value0_6;
    var _S228 : s_bwdCallableCtx_dot_0 = _S227;
    var _S229 : vec3<f32> = vec3<f32>(0.0f);
    var _S230 : bool;
    if(!_S181.value0_16.value1_1.singular_1)
    {
        _S230 = (_S226.value0_5) != 0.0f;
    }
    else
    {
        _S230 = false;
    }
    var scale_2 : vec3<f32>;
    var _S231 : vec3<f32>;
    var _S232 : f32;
    var _S233 : s_bwdCallableCtx_dot_0;
    var _S234 : s_bwdCallableCtx_dot_0;
    var _S235 : s_bwdCallableCtx_dot_0;
    if(_S230)
    {
        var _S236 : Tuple_5 = s_apply_dot_0(_S217.value0_6, _S214.value0_13);
        var _S237 : s_bwdCallableCtx_dot_0;
        _S237._S27 = _S217.value0_6;
        _S237._S28 = _S214.value0_13;
        var _S238 : Tuple_5 = s_apply_dot_0(_S220.value0_6, _S214.value0_13);
        var _S239 : s_bwdCallableCtx_dot_0;
        _S239._S27 = _S220.value0_6;
        _S239._S28 = _S214.value0_13;
        var _S240 : Tuple_5 = s_apply_dot_0(_S223.value0_6, _S214.value0_13);
        var _S241 : s_bwdCallableCtx_dot_0;
        _S241._S27 = _S223.value0_6;
        _S241._S28 = _S214.value0_13;
        var _S242 : vec3<f32> = vec3<f32>(_S236.value0_5, _S238.value0_5, _S240.value0_5);
        var _S243 : f32 = _S226.value0_5 * _S226.value0_5;
        scale_2 = _S242 / vec3<f32>(_S226.value0_5);
        _S232 = _S243;
        _S231 = _S242;
        _S233 = _S241;
        _S234 = _S239;
        _S235 = _S237;
    }
    else
    {
        scale_2 = _S229;
        _S232 = 0.0f;
        _S231 = _S184;
        _S233._S27 = _S184;
        _S233._S28 = _S184;
        _S234._S27 = _S184;
        _S234._S28 = _S184;
        _S235._S27 = _S184;
        _S235._S28 = _S184;
    }
    var _S244 : vec3<f32> = vec3<f32>(scale_2.x);
    var _S245 : vec3<f32> = _S190.value0_13 * _S244;
    var _S246 : s_bwdCallableCtx_mul_0;
    _S246._S19 = _S181.value0_16.value1_1.to_rgb_0;
    _S246._S20 = _S245;
    var _S247 : s_bwdCallableCtx_mul_0 = _S246;
    var _S248 : vec3<f32> = vec3<f32>(scale_2.y);
    var _S249 : vec3<f32> = _S198.value0_13 * _S248;
    var _S250 : s_bwdCallableCtx_mul_0;
    _S250._S19 = _S181.value0_16.value1_1.to_rgb_0;
    _S250._S20 = _S249;
    var _S251 : s_bwdCallableCtx_mul_0 = _S250;
    var _S252 : vec3<f32> = vec3<f32>(scale_2.z);
    var _S253 : vec3<f32> = _S206.value0_13 * _S252;
    var _S254 : s_bwdCallableCtx_mul_0;
    _S254._S19 = _S181.value0_16.value1_1.to_rgb_0;
    _S254._S20 = _S253;
    var _S255 : s_bwdCallableCtx_mul_0 = _S254;
    var _S256 : Settings_0 = Settings_x24_syn_dzero_0();
    var _S257 : vec3<f32> = vec3<f32>(_S183[i32(0)][i32(2)], _S183[i32(1)][i32(2)], _S183[i32(2)][i32(2)]);
    var _S258 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S259 : mat3x3<f32> = _S258;
    var _S260 : vec3<f32> = _S184;
    s_bwdProp_mul_0(_S255, &(_S259), &(_S260), _S257);
    var _S261 : vec3<f32> = _S206.value0_13 * _S260;
    var _S262 : f32 = _S261[i32(0)] + _S261[i32(1)] + _S261[i32(2)];
    var _S263 : vec3<f32> = _S252 * _S260;
    var _S264 : vec3<f32> = vec3<f32>(_S183[i32(0)][i32(1)], _S183[i32(1)][i32(1)], _S183[i32(2)][i32(1)]);
    var _S265 : mat3x3<f32> = _S258;
    var _S266 : vec3<f32> = _S184;
    s_bwdProp_mul_0(_S251, &(_S265), &(_S266), _S264);
    var _S267 : vec3<f32> = _S198.value0_13 * _S266;
    var _S268 : f32 = _S267[i32(0)] + _S267[i32(1)] + _S267[i32(2)];
    var _S269 : vec3<f32> = _S248 * _S266;
    var _S270 : vec3<f32> = vec3<f32>(_S183[i32(0)][i32(0)], _S183[i32(1)][i32(0)], _S183[i32(2)][i32(0)]);
    var _S271 : mat3x3<f32> = _S258;
    var _S272 : vec3<f32> = _S184;
    s_bwdProp_mul_0(_S247, &(_S271), &(_S272), _S270);
    var _S273 : vec3<f32> = _S190.value0_13 * _S272;
    var _S274 : vec3<f32> = _S244 * _S272;
    var _S275 : vec3<f32> = vec3<f32>(_S273[i32(0)] + _S273[i32(1)] + _S273[i32(2)], _S268, _S262);
    var _S276 : vec3<f32>;
    var _S277 : vec3<f32>;
    if(_S230)
    {
        var _S278 : vec3<f32> = _S275 / vec3<f32>(_S232);
        var _S279 : vec3<f32> = _S231 * (vec3<f32>(0) - _S278);
        var _S280 : f32 = _S279[i32(0)] + _S279[i32(1)] + _S279[i32(2)];
        var _S281 : vec3<f32> = vec3<f32>(_S226.value0_5) * _S278;
        var _S282 : vec3<f32> = _S184;
        var _S283 : vec3<f32> = _S184;
        s_bwdProp_dot_0(_S233, &(_S282), &(_S283), _S281[i32(2)]);
        var _S284 : vec3<f32> = _S184;
        var _S285 : vec3<f32> = _S184;
        s_bwdProp_dot_0(_S234, &(_S284), &(_S285), _S281[i32(1)]);
        var _S286 : vec3<f32> = _S184;
        var _S287 : vec3<f32> = _S184;
        s_bwdProp_dot_0(_S235, &(_S286), &(_S287), _S281[i32(0)]);
        var _S288 : vec3<f32> = _S283 + _S285 + _S287;
        _S232 = _S280;
        scale_2 = _S282;
        _S231 = _S284;
        _S276 = _S286;
        _S277 = _S288;
    }
    else
    {
        _S232 = 0.0f;
        scale_2 = _S184;
        _S231 = _S184;
        _S276 = _S184;
        _S277 = _S184;
    }
    var _S289 : vec3<f32> = _S184;
    var _S290 : vec3<f32> = _S184;
    s_bwdProp_dot_0(_S228, &(_S289), &(_S290), _S232);
    var _S291 : vec3<f32> = _S184;
    var _S292 : vec3<f32> = _S184;
    s_bwdProp_cross_0(_S225, &(_S291), &(_S292), scale_2);
    var _S293 : vec3<f32> = _S184;
    var _S294 : vec3<f32> = _S184;
    s_bwdProp_cross_0(_S222, &(_S293), &(_S294), _S231);
    var _S295 : vec3<f32> = _S290 + _S276;
    var _S296 : vec3<f32> = _S184;
    var _S297 : vec3<f32> = _S184;
    s_bwdProp_cross_0(_S219, &(_S296), &(_S297), _S295);
    var _S298 : vec2<f32> = vec2<f32>(0.0f);
    var _S299 : vec2<f32> = _S298;
    s_bwdProp_chromaticity_to_xyz_0(_S216, &(_S299), _S277);
    var _S300 : f32 = 0.0f;
    var _S301 : f32 = 0.0f;
    s_bwdProp_rotate_and_scale_0(_S213, &(_S300), &(_S301), _S299);
    var _S302 : vec3<f32> = _S293 + _S297 + _S263;
    var _S303 : vec2<f32> = _S298;
    s_bwdProp_chromaticity_to_xyz_0(_S208, &(_S303), _S302);
    var _S304 : f32 = 0.0f;
    var _S305 : f32 = 0.0f;
    s_bwdProp_rotate_and_scale_0(_S205, &(_S304), &(_S305), _S303);
    var _S306 : vec3<f32> = _S292 + _S296 + _S269;
    var _S307 : vec2<f32> = _S298;
    s_bwdProp_chromaticity_to_xyz_0(_S200, &(_S307), _S306);
    var _S308 : f32 = 0.0f;
    var _S309 : f32 = 0.0f;
    s_bwdProp_rotate_and_scale_0(_S197, &(_S308), &(_S309), _S307);
    var _S310 : vec3<f32> = _S289 + _S291 + _S294 + _S274;
    var _S311 : vec2<f32> = _S298;
    s_bwdProp_chromaticity_to_xyz_0(_S192, &(_S311), _S310);
    var _S312 : f32 = 0.0f;
    var _S313 : f32 = 0.0f;
    s_bwdProp_rotate_and_scale_0(_S189, &(_S312), &(_S313), _S311);
    var _S314 : vec4<f32> = vec4<f32>(_S301, _S313, _S309, _S305);
    var _S315 : vec4<f32> = vec4<f32>(_S300, _S312, _S308, _S304);
    var _S316 : Settings_0 = _S256;
    _S316.hue_1 = _S314;
    _S316.purity_1 = _S315;
    (*_S182) = Settings_x24_syn_dadd_0(_S256, _S316);
    return;
}

fn s_bwdProp_primaries_from_settings_0( _S317 : s_bwdCallableCtx_primaries_from_settings_0,  _S318 : ptr<function, Settings_0>,  _S319 : vec3<f32>)
{
    var _S320 : Tuple_7 = s_apply_adjustment_0(_S317.value0_8.value1_0, _S317.value0_8.value2_0);
    var _S321 : s_paramCtx_s_bwdCallableCtx_adjustment_0;
    _S321.value0_15 = _S317.value0_8.value1_0;
    _S321.value1_1 = _S317.value0_8.value2_0;
    var _S322 : s_bwdCallableCtx_adjustment_0 = s_bwdCallableCtx_adjustment_0( _S321 );
    var _S323 : s_paramCtx_s_bwdCallableCtx_primaries_0;
    _S323.value0_17 = _S317.value0_8.value0_7;
    _S323.value1_2 = _S320.value0_9;
    var _S324 : s_bwdCallableCtx_primaries_0 = s_bwdCallableCtx_primaries_0( _S323 );
    var _S325 : Settings_0 = Settings_x24_syn_dzero_0();
    var _S326 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    s_bwdProp_primaries_0(_S324, &(_S326), _S319);
    var _S327 : vec4<f32> = vec4<f32>(0.0f);
    var _S328 : Settings_0;
    _S328.hue_1 = _S327;
    _S328.purity_1 = _S327;
    s_bwdProp_adjustment_0(_S322, &(_S328), _S326);
    (*_S318) = Settings_x24_syn_dadd_0(_S328, _S325);
    return;
}

fn _S329( _S330 : vec3<f32>,  _S331 : ptr<function, DiffPair_Settings_0>,  _S332 : Profile_0,  _S333 : vec3<f32>)
{
    var _S334 : s_paramCtx_s_bwdCallableCtx_primaries_from_settings_0;
    _S334.value0_7 = _S330;
    _S334.value1_0 = (*_S331).primal_0;
    _S334.value2_0 = _S332;
    var _S335 : s_bwdCallableCtx_primaries_from_settings_0 = s_bwdCallableCtx_primaries_from_settings_0( _S334 );
    var _S336 : Settings_0 = (*_S331).differential_0;
    s_bwdProp_primaries_from_settings_0(_S335, &(_S336), _S333);
    (*_S331).differential_0 = _S336;
    return;
}

fn _d_dot_1( dpx_4 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpy_2 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_float_0
{
    var _S337 : DiffPair_float_0 = DiffPair_float_0( dpx_4.primal_0[i32(0)] * dpy_2.primal_0[i32(0)] + dpx_4.primal_0[i32(1)] * dpy_2.primal_0[i32(1)] + dpx_4.primal_0[i32(2)] * dpy_2.primal_0[i32(2)], dpx_4.primal_0[i32(0)] * dpy_2.differential_0[i32(0)] + dpy_2.primal_0[i32(0)] * dpx_4.differential_0[i32(0)] + dpx_4.primal_0[i32(1)] * dpy_2.differential_0[i32(1)] + dpy_2.primal_0[i32(1)] * dpx_4.differential_0[i32(1)] + dpx_4.primal_0[i32(2)] * dpy_2.differential_0[i32(2)] + dpy_2.primal_0[i32(2)] * dpx_4.differential_0[i32(2)] );
    return _S337;
}

fn _d_cross_1( a_2 : DiffPair_vectorx3Cfloatx2C3x3E_0,  b_2 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S338 : f32 = a_2.primal_0.y;
    var _S339 : f32 = b_2.primal_0.z;
    var _S340 : f32 = a_2.primal_0.z;
    var _S341 : f32 = b_2.primal_0.y;
    var _S342 : f32 = a_2.differential_0.y;
    var _S343 : f32 = b_2.differential_0.z;
    var _S344 : f32 = a_2.differential_0.z;
    var _S345 : f32 = b_2.differential_0.y;
    var _S346 : f32 = b_2.primal_0.x;
    var _S347 : f32 = a_2.primal_0.x;
    var _S348 : f32 = b_2.differential_0.x;
    var _S349 : f32 = a_2.differential_0.x;
    var _S350 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S338 * _S339 - _S340 * _S341, _S340 * _S346 - _S347 * _S339, _S347 * _S341 - _S338 * _S346), vec3<f32>(_S342 * _S339 + _S338 * _S343 - _S344 * _S341 - _S340 * _S345, _S344 * _S346 + _S340 * _S348 - _S349 * _S339 - _S347 * _S343, _S349 * _S341 + _S347 * _S345 - _S342 * _S346 - _S338 * _S348) );
    return _S350;
}

fn _d_min_1( dpx_5 : DiffPair_float_0,  dpy_3 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S351 : f32 = min(dpx_5.primal_0, dpy_3.primal_0);
    var _S352 : f32;
    if(((dpx_5.primal_0) < (dpy_3.primal_0)))
    {
        _S352 = dpx_5.differential_0;
    }
    else
    {
        if(((dpx_5.primal_0) > (dpy_3.primal_0)))
        {
            _S352 = dpy_3.differential_0;
        }
        else
        {
            _S352 = 0.5f * (dpx_5.differential_0 + dpy_3.differential_0);
        }
    }
    var _S353 : DiffPair_float_0 = DiffPair_float_0( _S351, _S352 );
    return _S353;
}

fn _d_sin_1( dpx_6 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S354 : DiffPair_float_0 = DiffPair_float_0( sin(dpx_6.primal_0), cos(dpx_6.primal_0) * dpx_6.differential_0 );
    return _S354;
}

fn _d_cos_1( dpx_7 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S355 : DiffPair_float_0 = DiffPair_float_0( cos(dpx_7.primal_0), - sin(dpx_7.primal_0) * dpx_7.differential_0 );
    return _S355;
}

fn _d_atan2_0( dpy_4 : DiffPair_float_0,  dpx_8 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S356 : f32 = dpx_8.primal_0;
    var _S357 : f32 = _S356 * _S356 + dpy_4.primal_0 * dpy_4.primal_0;
    var _S358 : DiffPair_float_0 = DiffPair_float_0( atan2(dpy_4.primal_0, dpx_8.primal_0), - dpy_4.primal_0 / _S357 * dpx_8.differential_0 + dpx_8.primal_0 / _S357 * dpy_4.differential_0 );
    return _S358;
}

fn rows_0( r0_0 : vec4<f32>,  r1_0 : vec4<f32>,  r2_0 : vec4<f32>) -> mat3x3<f32>
{
    return mat3x3<f32>(r0_0.xyz, r1_0.xyz, r2_0.xyz);
}

fn primaries_1( rgb_0 : vec3<f32>,  adjustment_0 : mat3x3<f32>) -> vec3<f32>
{
    return (((rgb_0) * (adjustment_0)));
}

fn s_fwd_primaries_0( rgb_1 : vec3<f32>,  dpadjustment_0 : DiffPair_matrixx3Cfloatx2C3x2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S359 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( (((rgb_1) * (dpadjustment_0.primal_0))), (((rgb_1) * (dpadjustment_0.differential_0))) + (((vec3<f32>(0.0f)) * (dpadjustment_0.primal_0))) );
    return _S359;
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_0 : vec3<u32>)
{
    var _S360 : u32 = id_0.x;
    var _S361 : bool;
    if((_S360 >= (params_0.pass_0.width_0)))
    {
        _S361 = true;
    }
    else
    {
        _S361 = ((id_0.y) >= (params_0.pass_0.height_0));
    }
    if(_S361)
    {
        return;
    }
    var i_0 : u32 = id_0.y * params_0.pass_0.width_0 + _S360;
    var _S362 : mat3x3<f32> = rows_0(params_0.primaries_0.matrix_0_0, params_0.primaries_0.matrix_1_0, params_0.primaries_0.matrix_2_0);
    var pixel_0 : vec4<f32> = input_0[i_0];
    var rgb_2 : vec3<f32> = pixel_0.xyz;
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
        var _S363 : vec3<f32> = primaries_1(rgb_2, _S362);
        var _S364 : u32 = k_0 + u32(1);
        rgb_2 = _S363;
        k_0 = _S364;
    }
    output_0[i_0] = vec4<f32>(rgb_2, pixel_0.w);
    return;
}

fn s_fwd_determinant_0( dpa_0 : DiffPair_float_0,  dpb_0 : DiffPair_float_0,  dpc_0 : DiffPair_float_0,  dpd_0 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S365 : DiffPair_float_0 = DiffPair_float_0( dpa_0.primal_0 * dpd_0.primal_0 - dpb_0.primal_0 * dpc_0.primal_0, dpa_0.differential_0 * dpd_0.primal_0 + dpd_0.differential_0 * dpa_0.primal_0 - (dpb_0.differential_0 * dpc_0.primal_0 + dpc_0.differential_0 * dpb_0.primal_0) );
    return _S365;
}

struct DiffPair_vectorx3Cfloatx2C2x3E_0
{
     primal_0 : vec2<f32>,
     differential_0 : vec2<f32>,
};

fn s_fwd_edge_distance_0( white_2 : vec2<f32>,  dpdirection_0 : DiffPair_vectorx3Cfloatx2C2x3E_0,  a_3 : vec2<f32>,  b_3 : vec2<f32>) -> DiffPair_float_0
{
    var _S366 : f32 = a_3.x;
    var _S367 : f32 = a_3.y;
    var _S368 : DiffPair_float_0 = DiffPair_float_0( - dpdirection_0.primal_0.x, - dpdirection_0.differential_0.x );
    var _S369 : DiffPair_float_0 = DiffPair_float_0( _S366 - b_3.x, 0.0f );
    var _S370 : DiffPair_float_0 = DiffPair_float_0( - dpdirection_0.primal_0.y, - dpdirection_0.differential_0.y );
    var _S371 : DiffPair_float_0 = DiffPair_float_0( _S367 - b_3.y, 0.0f );
    var _S372 : DiffPair_float_0 = s_fwd_determinant_0(_S368, _S369, _S370, _S371);
    if((_S372.primal_0) == 0.0f)
    {
        var _S373 : DiffPair_float_0 = DiffPair_float_0( 3.4028234663852886e+38f, 0.0f );
        return _S373;
    }
    var _S374 : DiffPair_float_0 = DiffPair_float_0( white_2.x - _S366, 0.0f );
    var _S375 : DiffPair_float_0 = DiffPair_float_0( white_2.y - _S367, 0.0f );
    var _S376 : DiffPair_float_0 = s_fwd_determinant_0(_S374, _S369, _S375, _S371);
    var t_1 : f32 = _S376.primal_0 / _S372.primal_0;
    var s_diff_t_0 : f32 = (_S376.differential_0 * _S372.primal_0 - _S376.primal_0 * _S372.differential_0) / (_S372.primal_0 * _S372.primal_0);
    var _S377 : f32;
    var _S378 : f32;
    if((t_1 >= 0.0f))
    {
        _S377 = t_1;
        _S378 = s_diff_t_0;
    }
    else
    {
        _S377 = 3.4028234663852886e+38f;
        _S378 = 0.0f;
    }
    var _S379 : DiffPair_float_0 = DiffPair_float_0( _S377, _S378 );
    return _S379;
}

fn s_fwd_rotate_and_scale_0( profile_2 : Profile_0,  primary_1 : vec2<f32>,  dpscaling_0 : DiffPair_float_0,  dprotation_0 : DiffPair_float_0) -> DiffPair_vectorx3Cfloatx2C2x3E_0
{
    var _S380 : DiffPair_float_0 = DiffPair_float_0( primary_1.y - profile_2.white_0.y, 0.0f );
    var _S381 : DiffPair_float_0 = DiffPair_float_0( primary_1.x - profile_2.white_0.x, 0.0f );
    var _S382 : DiffPair_float_0 = _d_atan2_0(_S380, _S381);
    var _S383 : DiffPair_float_0 = DiffPair_float_0( _S382.primal_0 + dprotation_0.primal_0, _S382.differential_0 + dprotation_0.differential_0 );
    var _S384 : DiffPair_float_0 = _d_cos_1(_S383);
    var _S385 : DiffPair_float_0 = _d_sin_1(_S383);
    var direction_2 : vec2<f32> = vec2<f32>(_S384.primal_0, _S385.primal_0);
    var s_diff_direction_0 : vec2<f32> = vec2<f32>(_S384.differential_0, _S385.differential_0);
    var _S386 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( direction_2, s_diff_direction_0 );
    var _S387 : DiffPair_float_0 = s_fwd_edge_distance_0(profile_2.white_0, _S386, profile_2.red_0, profile_2.green_0);
    var _S388 : DiffPair_float_0 = s_fwd_edge_distance_0(profile_2.white_0, _S386, profile_2.green_0, profile_2.blue_0);
    var _S389 : DiffPair_float_0 = DiffPair_float_0( _S387.primal_0, _S387.differential_0 );
    var _S390 : DiffPair_float_0 = DiffPair_float_0( _S388.primal_0, _S388.differential_0 );
    var _S391 : DiffPair_float_0 = _d_min_1(_S389, _S390);
    var _S392 : DiffPair_float_0 = s_fwd_edge_distance_0(profile_2.white_0, _S386, profile_2.blue_0, profile_2.red_0);
    var _S393 : DiffPair_float_0 = DiffPair_float_0( _S391.primal_0, _S391.differential_0 );
    var _S394 : DiffPair_float_0 = DiffPair_float_0( _S392.primal_0, _S392.differential_0 );
    var _S395 : DiffPair_float_0 = _d_min_1(_S393, _S394);
    var _S396 : vec2<f32> = vec2<f32>((dpscaling_0.primal_0 * _S395.primal_0));
    var _S397 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( profile_2.white_0 + _S396 * direction_2, vec2<f32>((dpscaling_0.differential_0 * _S395.primal_0 + _S395.differential_0 * dpscaling_0.primal_0)) * direction_2 + s_diff_direction_0 * _S396 );
    return _S397;
}

fn s_fwd_sanitize_y_0( dpy_5 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S398 : bool;
    if(((dpy_5.primal_0) < 1.1920928955078125e-07f))
    {
        _S398 = ((dpy_5.primal_0) >= 0.0f);
    }
    else
    {
        _S398 = false;
    }
    if(_S398)
    {
        var _S399 : DiffPair_float_0 = DiffPair_float_0( 1.1920928955078125e-07f, 0.0f );
        return _S399;
    }
    if(((dpy_5.primal_0) < 0.0f))
    {
        _S398 = ((dpy_5.primal_0) > -1.1920928955078125e-07f);
    }
    else
    {
        _S398 = false;
    }
    if(_S398)
    {
        var _S400 : DiffPair_float_0 = DiffPair_float_0( -1.1920928955078125e-07f, 0.0f );
        return _S400;
    }
    var _S401 : DiffPair_float_0 = DiffPair_float_0( dpy_5.primal_0, dpy_5.differential_0 );
    return _S401;
}

fn s_fwd_chromaticity_to_xyz_0( dpxy_0 : DiffPair_vectorx3Cfloatx2C2x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S402 : DiffPair_float_0 = DiffPair_float_0( dpxy_0.primal_0.y, dpxy_0.differential_0.y );
    var _S403 : DiffPair_float_0 = s_fwd_sanitize_y_0(_S402);
    var _S404 : f32 = dpxy_0.primal_0.x;
    var _S405 : f32 = dpxy_0.differential_0.x;
    var _S406 : f32 = _S403.primal_0 * _S403.primal_0;
    var _S407 : f32 = 1.0f - _S404 - _S403.primal_0;
    var _S408 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S404 / _S403.primal_0, 1.0f, _S407 / _S403.primal_0), vec3<f32>((_S405 * _S403.primal_0 - _S404 * _S403.differential_0) / _S406, 0.0f, ((0.0f - _S405 - _S403.differential_0) * _S403.primal_0 - _S407 * _S403.differential_0) / _S406) );
    return _S408;
}

fn s_fwd_adjustment_0( dps_0 : DiffPair_Settings_0,  profile_3 : Profile_0) -> DiffPair_matrixx3Cfloatx2C3x2C3x3E_0
{
    var _S409 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.purity_1.y, dps_0.differential_0.purity_1.y );
    var _S410 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.hue_1.y, dps_0.differential_0.hue_1.y );
    var _S411 : DiffPair_vectorx3Cfloatx2C2x3E_0 = s_fwd_rotate_and_scale_0(profile_3, profile_3.red_0, _S409, _S410);
    var _S412 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( _S411.primal_0, _S411.differential_0 );
    var _S413 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_chromaticity_to_xyz_0(_S412);
    var _S414 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.purity_1.z, dps_0.differential_0.purity_1.z );
    var _S415 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.hue_1.z, dps_0.differential_0.hue_1.z );
    var _S416 : DiffPair_vectorx3Cfloatx2C2x3E_0 = s_fwd_rotate_and_scale_0(profile_3, profile_3.green_0, _S414, _S415);
    var _S417 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( _S416.primal_0, _S416.differential_0 );
    var _S418 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_chromaticity_to_xyz_0(_S417);
    var _S419 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.purity_1.w, dps_0.differential_0.purity_1.w );
    var _S420 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.hue_1.w, dps_0.differential_0.hue_1.w );
    var _S421 : DiffPair_vectorx3Cfloatx2C2x3E_0 = s_fwd_rotate_and_scale_0(profile_3, profile_3.blue_0, _S419, _S420);
    var _S422 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( _S421.primal_0, _S421.differential_0 );
    var _S423 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_chromaticity_to_xyz_0(_S422);
    var _S424 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.purity_1.x, dps_0.differential_0.purity_1.x );
    var _S425 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.hue_1.x, dps_0.differential_0.hue_1.x );
    var _S426 : DiffPair_vectorx3Cfloatx2C2x3E_0 = s_fwd_rotate_and_scale_0(profile_3, profile_3.red_0, _S424, _S425);
    var _S427 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( _S426.primal_0, _S426.differential_0 );
    var _S428 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_chromaticity_to_xyz_0(_S427);
    var _S429 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S418.primal_0, _S418.differential_0 );
    var _S430 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S423.primal_0, _S423.differential_0 );
    var _S431 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_cross_1(_S429, _S430);
    var _S432 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S413.primal_0, _S413.differential_0 );
    var _S433 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_cross_1(_S430, _S432);
    var _S434 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_cross_1(_S432, _S429);
    var _S435 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S431.primal_0, _S431.differential_0 );
    var _S436 : DiffPair_float_0 = _d_dot_1(_S432, _S435);
    var _S437 : vec3<f32> = vec3<f32>(0.0f);
    var _S438 : vec3<f32> = vec3<f32>(0.0f);
    var _S439 : bool;
    if(!profile_3.singular_1)
    {
        _S439 = (_S436.primal_0) != 0.0f;
    }
    else
    {
        _S439 = false;
    }
    var scale_3 : vec3<f32>;
    var s_diff_scale_0 : vec3<f32>;
    if(_S439)
    {
        var _S440 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S428.primal_0, _S428.differential_0 );
        var _S441 : DiffPair_float_0 = _d_dot_1(_S435, _S440);
        var _S442 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S433.primal_0, _S433.differential_0 );
        var _S443 : DiffPair_float_0 = _d_dot_1(_S442, _S440);
        var _S444 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S434.primal_0, _S434.differential_0 );
        var _S445 : DiffPair_float_0 = _d_dot_1(_S444, _S440);
        var _S446 : vec3<f32> = vec3<f32>(_S441.primal_0, _S443.primal_0, _S445.primal_0);
        var _S447 : vec3<f32> = vec3<f32>(_S436.primal_0);
        var _S448 : vec3<f32> = (vec3<f32>(_S441.differential_0, _S443.differential_0, _S445.differential_0) * _S447 - _S446 * vec3<f32>(_S436.differential_0)) / vec3<f32>((_S436.primal_0 * _S436.primal_0));
        scale_3 = _S446 / _S447;
        s_diff_scale_0 = _S448;
    }
    else
    {
        scale_3 = _S437;
        s_diff_scale_0 = _S438;
    }
    var _S449 : vec3<f32> = vec3<f32>(scale_3.x);
    var _S450 : vec3<f32> = _S413.primal_0 * _S449;
    var _S451 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S452 : vec3<f32> = (((_S450) * (profile_3.to_rgb_0)));
    var _S453 : vec3<f32> = (((_S450) * (_S451))) + (((_S413.differential_0 * _S449 + vec3<f32>(s_diff_scale_0.x) * _S413.primal_0) * (profile_3.to_rgb_0)));
    var _S454 : vec3<f32> = vec3<f32>(scale_3.y);
    var _S455 : vec3<f32> = _S418.primal_0 * _S454;
    var _S456 : vec3<f32> = (((_S455) * (profile_3.to_rgb_0)));
    var _S457 : vec3<f32> = (((_S455) * (_S451))) + (((_S418.differential_0 * _S454 + vec3<f32>(s_diff_scale_0.y) * _S418.primal_0) * (profile_3.to_rgb_0)));
    var _S458 : vec3<f32> = vec3<f32>(scale_3.z);
    var _S459 : vec3<f32> = _S423.primal_0 * _S458;
    var _S460 : vec3<f32> = (((_S459) * (profile_3.to_rgb_0)));
    var _S461 : vec3<f32> = (((_S459) * (_S451))) + (((_S423.differential_0 * _S458 + vec3<f32>(s_diff_scale_0.z) * _S423.primal_0) * (profile_3.to_rgb_0)));
    var _S462 : DiffPair_matrixx3Cfloatx2C3x2C3x3E_0 = DiffPair_matrixx3Cfloatx2C3x2C3x3E_0( mat3x3<f32>(vec3<f32>(_S452.x, _S456.x, _S460.x), vec3<f32>(_S452.y, _S456.y, _S460.y), vec3<f32>(_S452.z, _S456.z, _S460.z)), mat3x3<f32>(vec3<f32>(_S453.x, _S457.x, _S461.x), vec3<f32>(_S453.y, _S457.y, _S461.y), vec3<f32>(_S453.z, _S457.z, _S461.z)) );
    return _S462;
}

fn s_fwd_primaries_from_settings_0( rgb_3 : vec3<f32>,  dps_1 : DiffPair_Settings_0,  profile_4 : Profile_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S463 : DiffPair_Settings_0 = DiffPair_Settings_0( dps_1.primal_0, dps_1.differential_0 );
    var _S464 : DiffPair_matrixx3Cfloatx2C3x2C3x3E_0 = s_fwd_adjustment_0(_S463, profile_4);
    var _S465 : DiffPair_matrixx3Cfloatx2C3x2C3x3E_0 = DiffPair_matrixx3Cfloatx2C3x2C3x3E_0( _S464.primal_0, _S464.differential_0 );
    var _S466 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_primaries_0(rgb_3, _S465);
    var _S467 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S466.primal_0, _S466.differential_0 );
    return _S467;
}

fn pick_0( d_0 : Settings_0,  index_0 : u32) -> f32
{
    var v_0 : vec4<f32>;
    if((index_0 % u32(2)) == u32(0))
    {
        v_0 = d_0.hue_1;
    }
    else
    {
        v_0 = d_0.purity_1;
    }
    return v_0[index_0 / u32(2)];
}

fn settings_of_0() -> Settings_0
{
    var s_0 : Settings_0;
    s_0.hue_1 = params_0.primaries_0.hue_0;
    s_0.purity_1 = params_0.primaries_0.purity_0;
    return s_0;
}

fn profile_of_0() -> Profile_0
{
    var profile_5 : Profile_0;
    profile_5.white_0 = params_0.primaries_0.white_and_red_0.xy;
    profile_5.red_0 = params_0.primaries_0.white_and_red_0.zw;
    profile_5.green_0 = params_0.primaries_0.green_and_blue_0.xy;
    profile_5.blue_0 = params_0.primaries_0.green_and_blue_0.zw;
    profile_5.to_rgb_0 = rows_0(params_0.primaries_0.to_rgb_0_0, params_0.primaries_0.to_rgb_1_0, params_0.primaries_0.to_rgb_2_0);
    profile_5.singular_1 = (params_0.primaries_0.singular_0) != u32(0);
    return profile_5;
}

@compute
@workgroup_size(16, 16, 1)
fn primaries_backward(@builtin(global_invocation_id) id_1 : vec3<u32>)
{
    var _S468 : u32 = id_1.x;
    var _S469 : bool;
    if((_S468 >= (params_0.pass_0.width_0)))
    {
        _S469 = true;
    }
    else
    {
        _S469 = ((id_1.y) >= (params_0.pass_0.height_0));
    }
    if(_S469)
    {
        return;
    }
    var i_1 : u32 = id_1.y * params_0.pass_0.width_0 + _S468;
    var _S470 : Settings_0 = Settings_x24_syn_dzero_0();
    var s_1 : DiffPair_Settings_0;
    s_1.primal_0 = settings_of_0();
    s_1.differential_0 = _S470;
    _S329(input_0[i_1].xyz, &(s_1), profile_of_0(), derivative_in_0[i_1].xyz);
    derivative_out_0[i_1] = vec4<f32>(pick_0(s_1.differential_0, params_0.primaries_0.gradient_0.x), pick_0(s_1.differential_0, params_0.primaries_0.gradient_0.y), pick_0(s_1.differential_0, params_0.primaries_0.gradient_0.z), pick_0(s_1.differential_0, params_0.primaries_0.gradient_0.w));
    return;
}

fn add_at_0( v_1 : vec4<f32>,  c_0 : u32,  x_5 : f32) -> vec4<f32>
{
    return select(v_1, v_1 + vec4<f32>(x_5), vec4<u32>(u32(0), u32(1), u32(2), u32(3)) == vec4<u32>(c_0));
}

fn add_slot_0( d_1 : ptr<function, Settings_0>,  index_1 : u32,  x_6 : f32)
{
    var _S471 : vec4<f32> = (*d_1).hue_1;
    var _S472 : u32 = index_1 % u32(2);
    var _S473 : u32;
    if(_S472 == u32(0))
    {
        _S473 = index_1 / u32(2);
    }
    else
    {
        _S473 = u32(4);
    }
    (*d_1).hue_1 = add_at_0(_S471, _S473, x_6);
    var _S474 : vec4<f32> = (*d_1).purity_1;
    if(_S472 == u32(1))
    {
        _S473 = index_1 / u32(2);
    }
    else
    {
        _S473 = u32(4);
    }
    (*d_1).purity_1 = add_at_0(_S474, _S473, x_6);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn primaries_forward_diff(@builtin(global_invocation_id) id_2 : vec3<u32>)
{
    var _S475 : u32 = id_2.x;
    var _S476 : bool;
    if((_S475 >= (params_0.pass_0.width_0)))
    {
        _S476 = true;
    }
    else
    {
        _S476 = ((id_2.y) >= (params_0.pass_0.height_0));
    }
    if(_S476)
    {
        return;
    }
    var i_2 : u32 = id_2.y * params_0.pass_0.width_0 + _S475;
    var v_2 : vec4<f32> = derivative_in_0[i_2];
    var direction_3 : Settings_0;
    var _S477 : vec4<f32> = vec4<f32>(0.0f);
    direction_3.hue_1 = _S477;
    direction_3.purity_1 = _S477;
    add_slot_0(&(direction_3), params_0.primaries_0.gradient_0.x, v_2.x);
    add_slot_0(&(direction_3), params_0.primaries_0.gradient_0.y, v_2.y);
    add_slot_0(&(direction_3), params_0.primaries_0.gradient_0.z, v_2.z);
    add_slot_0(&(direction_3), params_0.primaries_0.gradient_0.w, v_2.w);
    var _S478 : DiffPair_Settings_0 = DiffPair_Settings_0( settings_of_0(), direction_3 );
    derivative_out_0[i_2] = vec4<f32>(s_fwd_primaries_from_settings_0(input_0[i_2].xyz, _S478, profile_of_0()).differential_0, 0.0f);
    return;
}

