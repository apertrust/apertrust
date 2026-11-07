// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2022-2026  darktable developers (ported from src/iop/colorequal.c)
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

// Generated from slang/colorequal.slang by tools/build-kernels.sh. Do not edit.

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

struct ColorEqualParams_std140_0
{
    @align(16) size_0 : vec4<u32>,
    @align(16) mode_0 : vec4<u32>,
    @align(16) deriche_a_0 : vec4<f32>,
    @align(16) deriche_b_0 : vec4<f32>,
    @align(16) shifts_0 : vec4<f32>,
    @align(16) white_0 : vec4<f32>,
    @align(16) slots_0 : vec4<f32>,
    @align(16) rgb_to_xyz_r0_0 : vec4<f32>,
    @align(16) rgb_to_xyz_r1_0 : vec4<f32>,
    @align(16) rgb_to_xyz_r2_0 : vec4<f32>,
    @align(16) xyz_to_rgb_r0_0 : vec4<f32>,
    @align(16) xyz_to_rgb_r1_0 : vec4<f32>,
    @align(16) xyz_to_rgb_r2_0 : vec4<f32>,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) ce_0 : ColorEqualParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(0) @group(0) var<storage, read> input_0 : array<vec4<f32>>;

@binding(10) @group(0) var<storage, read_write> uv_0 : array<vec2<f32>>;

@binding(16) @group(0) var<storage, read_write> corrections_0 : array<vec2<f32>>;

@binding(20) @group(0) var<storage, read_write> brightness_0 : array<f32>;

@binding(3) @group(0) var<storage, read> table_0 : array<f32>;

@binding(1) @group(0) var<storage, read_write> output_0 : array<vec4<f32>>;

@binding(8) @group(0) var<storage, read> derivative_in_0 : array<vec4<f32>>;

@binding(9) @group(0) var<storage, read_write> derivative_out_0 : array<vec4<f32>>;

@binding(18) @group(0) var<storage, read_write> saturation_0 : array<f32>;

@binding(19) @group(0) var<storage, read_write> lightness_0 : array<f32>;

@binding(21) @group(0) var<storage, read_write> line2_in_0 : array<vec2<f32>>;

@binding(22) @group(0) var<storage, read_write> line2_out_0 : array<vec2<f32>>;

@binding(23) @group(0) var<storage, read_write> line1_in_0 : array<f32>;

@binding(24) @group(0) var<storage, read_write> line1_out_0 : array<f32>;

@binding(11) @group(0) var<storage, read_write> grid_uv_0 : array<vec2<f32>>;

@binding(12) @group(0) var<storage, read_write> p1_0 : array<vec2<f32>>;

@binding(13) @group(0) var<storage, read_write> p2_0 : array<vec2<f32>>;

@binding(14) @group(0) var<storage, read_write> c1_0 : array<vec2<f32>>;

@binding(17) @group(0) var<storage, read_write> weights_0 : array<vec2<f32>>;

@binding(25) @group(0) var<storage, read_write> grid_corrections_0 : array<vec2<f32>>;

@binding(26) @group(0) var<storage, read_write> grid_brightness_0 : array<f32>;

@binding(15) @group(0) var<storage, read_write> c2_0 : array<vec2<f32>>;

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
     value0_1 : vec2<f32>,
};

fn s_apply_abs_0( x_1 : vec2<f32>) -> Tuple_1
{
    var _S9 : Tuple_1 = Tuple_1( abs(x_1) );
    return _S9;
}

struct s_bwdCallableCtx_abs_0
{
     _S10 : vec2<f32>,
};

struct DiffPair_vectorx3Cfloatx2C2x3E_0
{
     primal_0 : vec2<f32>,
     differential_0 : vec2<f32>,
};

fn _d_abs_vector_0( dpx_1 : ptr<function, DiffPair_vectorx3Cfloatx2C2x3E_0>,  dOut_1 : vec2<f32>)
{
    var _S11 : vec2<f32> = vec2<f32>(0.0f);
    var _S12 : vec2<f32> = select(select(vec2<f32>(-1.0f), _S11, ((*dpx_1).primal_0) == _S11), vec2<f32>(1.0f), (((*dpx_1).primal_0) > _S11)) * dOut_1;
    (*dpx_1).primal_0 = (*dpx_1).primal_0;
    (*dpx_1).differential_0 = _S12;
    return;
}

fn s_bwdProp_abs_0( ctx_1 : s_bwdCallableCtx_abs_0,  d_x_1 : ptr<function, vec2<f32>>,  d_out_1 : vec2<f32>)
{
    var _S13 : DiffPair_vectorx3Cfloatx2C2x3E_0;
    _S13.primal_0 = ctx_1._S10;
    _d_abs_vector_0(&(_S13), d_out_1);
    (*d_x_1) = _S13.differential_0;
    return;
}

struct Tuple_2
{
     value0_2 : f32,
};

fn s_apply_sin_0( x_2 : f32) -> Tuple_2
{
    var _S14 : Tuple_2 = Tuple_2( sin(x_2) );
    return _S14;
}

struct s_bwdCallableCtx_sin_0
{
     _S15 : f32,
};

fn _d_sin_0( dpx_2 : ptr<function, DiffPair_float_0>,  dOut_2 : f32)
{
    var _S16 : f32 = cos((*dpx_2).primal_0) * dOut_2;
    (*dpx_2).primal_0 = (*dpx_2).primal_0;
    (*dpx_2).differential_0 = _S16;
    return;
}

fn s_bwdProp_sin_0( ctx_2 : s_bwdCallableCtx_sin_0,  d_x_2 : ptr<function, f32>,  d_out_2 : f32)
{
    var _S17 : DiffPair_float_0;
    _S17.primal_0 = ctx_2._S15;
    _d_sin_0(&(_S17), d_out_2);
    (*d_x_2) = _S17.differential_0;
    return;
}

struct Tuple_3
{
     value0_3 : f32,
};

fn s_apply_cos_0( x_3 : f32) -> Tuple_3
{
    var _S18 : Tuple_3 = Tuple_3( cos(x_3) );
    return _S18;
}

struct s_bwdCallableCtx_cos_0
{
     _S19 : f32,
};

fn _d_cos_0( dpx_3 : ptr<function, DiffPair_float_0>,  dOut_3 : f32)
{
    var _S20 : f32 = - sin((*dpx_3).primal_0) * dOut_3;
    (*dpx_3).primal_0 = (*dpx_3).primal_0;
    (*dpx_3).differential_0 = _S20;
    return;
}

fn s_bwdProp_cos_0( ctx_3 : s_bwdCallableCtx_cos_0,  d_x_3 : ptr<function, f32>,  d_out_3 : f32)
{
    var _S21 : DiffPair_float_0;
    _S21.primal_0 = ctx_3._S19;
    _d_cos_0(&(_S21), d_out_3);
    (*d_x_3) = _S21.differential_0;
    return;
}

struct Tuple_4
{
     value0_4 : f32,
};

fn s_apply_clamp_0( x_4 : f32,  minBound_0 : f32,  maxBound_0 : f32) -> Tuple_4
{
    var _S22 : Tuple_4 = Tuple_4( clamp(x_4, minBound_0, maxBound_0) );
    return _S22;
}

struct s_bwdCallableCtx_clamp_0
{
     _S23 : f32,
     _S24 : f32,
     _S25 : f32,
};

fn _d_clamp_0( dpx_4 : ptr<function, DiffPair_float_0>,  dpMin_0 : ptr<function, DiffPair_float_0>,  dpMax_0 : ptr<function, DiffPair_float_0>,  dOut_4 : f32)
{
    var _S26 : DiffPair_float_0 = (*dpx_4);
    var _S27 : bool;
    if((((*dpx_4).primal_0) >= ((*dpMin_0).primal_0)))
    {
        _S27 = (((*dpx_4).primal_0) <= ((*dpMax_0).primal_0));
    }
    else
    {
        _S27 = false;
    }
    var _S28 : f32;
    if(_S27)
    {
        _S28 = dOut_4;
    }
    else
    {
        _S28 = 0.0f;
    }
    (*dpx_4).primal_0 = _S26.primal_0;
    (*dpx_4).differential_0 = _S28;
    var _S29 : DiffPair_float_0 = (*dpMin_0);
    if(((_S26.primal_0) < ((*dpMin_0).primal_0)))
    {
        _S28 = dOut_4;
    }
    else
    {
        _S28 = 0.0f;
    }
    (*dpMin_0).primal_0 = _S29.primal_0;
    (*dpMin_0).differential_0 = _S28;
    var _S30 : DiffPair_float_0 = (*dpMax_0);
    if((((*dpx_4).primal_0) > ((*dpMax_0).primal_0)))
    {
        _S28 = dOut_4;
    }
    else
    {
        _S28 = 0.0f;
    }
    (*dpMax_0).primal_0 = _S30.primal_0;
    (*dpMax_0).differential_0 = _S28;
    return;
}

fn s_bwdProp_clamp_0( ctx_4 : s_bwdCallableCtx_clamp_0,  d_x_4 : ptr<function, f32>,  d_minBound_0 : ptr<function, f32>,  d_maxBound_0 : ptr<function, f32>,  d_out_4 : f32)
{
    var _S31 : DiffPair_float_0;
    _S31.primal_0 = ctx_4._S23;
    var _S32 : DiffPair_float_0;
    _S32.primal_0 = ctx_4._S24;
    var _S33 : DiffPair_float_0;
    _S33.primal_0 = ctx_4._S25;
    _d_clamp_0(&(_S31), &(_S32), &(_S33), d_out_4);
    (*d_x_4) = _S31.differential_0;
    (*d_minBound_0) = _S32.differential_0;
    (*d_maxBound_0) = _S33.differential_0;
    return;
}

struct Tuple_5
{
     value0_5 : f32,
};

fn s_apply_exp_0( x_5 : f32) -> Tuple_5
{
    var _S34 : Tuple_5 = Tuple_5( exp(x_5) );
    return _S34;
}

struct s_bwdCallableCtx_exp_0
{
     _S35 : f32,
};

fn _d_exp_0( dpx_5 : ptr<function, DiffPair_float_0>,  dOut_5 : f32)
{
    var _S36 : f32 = exp((*dpx_5).primal_0) * dOut_5;
    (*dpx_5).primal_0 = (*dpx_5).primal_0;
    (*dpx_5).differential_0 = _S36;
    return;
}

fn s_bwdProp_exp_0( ctx_5 : s_bwdCallableCtx_exp_0,  d_x_5 : ptr<function, f32>,  d_out_5 : f32)
{
    var _S37 : DiffPair_float_0;
    _S37.primal_0 = ctx_5._S35;
    _d_exp_0(&(_S37), d_out_5);
    (*d_x_5) = _S37.differential_0;
    return;
}

struct Tuple_6
{
     value0_6 : f32,
};

fn s_apply_atan2_0( y_1 : f32,  x_6 : f32) -> Tuple_6
{
    var _S38 : Tuple_6 = Tuple_6( atan2(y_1, x_6) );
    return _S38;
}

struct s_bwdCallableCtx_atan2_0
{
     _S39 : f32,
     _S40 : f32,
};

fn _d_atan2_0( dpy_1 : ptr<function, DiffPair_float_0>,  dpx_6 : ptr<function, DiffPair_float_0>,  dOut_6 : f32)
{
    var _S41 : DiffPair_float_0 = (*dpx_6);
    var _S42 : f32 = - (*dpy_1).primal_0 / ((*dpx_6).primal_0 * (*dpx_6).primal_0 + (*dpy_1).primal_0 * (*dpy_1).primal_0) * dOut_6;
    (*dpx_6).primal_0 = (*dpx_6).primal_0;
    (*dpx_6).differential_0 = _S42;
    var _S43 : f32 = _S41.primal_0 / (_S41.primal_0 * _S41.primal_0 + (*dpy_1).primal_0 * (*dpy_1).primal_0) * dOut_6;
    (*dpy_1).primal_0 = (*dpy_1).primal_0;
    (*dpy_1).differential_0 = _S43;
    return;
}

fn s_bwdProp_atan2_0( ctx_6 : s_bwdCallableCtx_atan2_0,  d_y_1 : ptr<function, f32>,  d_x_6 : ptr<function, f32>,  d_out_6 : f32)
{
    var _S44 : DiffPair_float_0;
    _S44.primal_0 = ctx_6._S39;
    var _S45 : DiffPair_float_0;
    _S45.primal_0 = ctx_6._S40;
    _d_atan2_0(&(_S44), &(_S45), d_out_6);
    (*d_y_1) = _S44.differential_0;
    (*d_x_6) = _S45.differential_0;
    return;
}

struct Tuple_7
{
     value0_7 : vec3<f32>,
};

fn s_apply_mul_0( left_0 : mat3x3<f32>,  right_0 : vec3<f32>) -> Tuple_7
{
    var _S46 : Tuple_7 = Tuple_7( (((right_0) * (left_0))) );
    return _S46;
}

struct s_bwdCallableCtx_mul_0
{
     _S47 : mat3x3<f32>,
     _S48 : vec3<f32>,
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

fn _d_mul_0( left_1 : ptr<function, DiffPair_matrixx3Cfloatx2C3x2C3x3E_0>,  right_1 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_7 : vec3<f32>)
{
    var _S49 : f32 = (*left_1).primal_0[i32(0)][i32(0)] * dOut_7[i32(0)];
    var left_d_result_0 : mat3x3<f32>;
    left_d_result_0[i32(0)][i32(0)] = (*right_1).primal_0[i32(0)] * dOut_7[i32(0)];
    var sum_0 : f32 = _S49 + (*left_1).primal_0[i32(1)][i32(0)] * dOut_7[i32(1)];
    left_d_result_0[i32(1)][i32(0)] = (*right_1).primal_0[i32(0)] * dOut_7[i32(1)];
    var sum_1 : f32 = sum_0 + (*left_1).primal_0[i32(2)][i32(0)] * dOut_7[i32(2)];
    left_d_result_0[i32(2)][i32(0)] = (*right_1).primal_0[i32(0)] * dOut_7[i32(2)];
    var right_d_result_0 : vec3<f32>;
    right_d_result_0[i32(0)] = sum_1;
    var _S50 : f32 = (*left_1).primal_0[i32(0)][i32(1)] * dOut_7[i32(0)];
    left_d_result_0[i32(0)][i32(1)] = (*right_1).primal_0[i32(1)] * dOut_7[i32(0)];
    var sum_2 : f32 = _S50 + (*left_1).primal_0[i32(1)][i32(1)] * dOut_7[i32(1)];
    left_d_result_0[i32(1)][i32(1)] = (*right_1).primal_0[i32(1)] * dOut_7[i32(1)];
    var sum_3 : f32 = sum_2 + (*left_1).primal_0[i32(2)][i32(1)] * dOut_7[i32(2)];
    left_d_result_0[i32(2)][i32(1)] = (*right_1).primal_0[i32(1)] * dOut_7[i32(2)];
    right_d_result_0[i32(1)] = sum_3;
    var _S51 : f32 = (*left_1).primal_0[i32(0)][i32(2)] * dOut_7[i32(0)];
    left_d_result_0[i32(0)][i32(2)] = (*right_1).primal_0[i32(2)] * dOut_7[i32(0)];
    var sum_4 : f32 = _S51 + (*left_1).primal_0[i32(1)][i32(2)] * dOut_7[i32(1)];
    left_d_result_0[i32(1)][i32(2)] = (*right_1).primal_0[i32(2)] * dOut_7[i32(1)];
    var sum_5 : f32 = sum_4 + (*left_1).primal_0[i32(2)][i32(2)] * dOut_7[i32(2)];
    left_d_result_0[i32(2)][i32(2)] = (*right_1).primal_0[i32(2)] * dOut_7[i32(2)];
    right_d_result_0[i32(2)] = sum_5;
    (*left_1).primal_0 = (*left_1).primal_0;
    (*left_1).differential_0 = left_d_result_0;
    (*right_1).primal_0 = (*right_1).primal_0;
    (*right_1).differential_0 = right_d_result_0;
    return;
}

fn s_bwdProp_mul_0( ctx_7 : s_bwdCallableCtx_mul_0,  d_left_0 : ptr<function, mat3x3<f32>>,  d_right_0 : ptr<function, vec3<f32>>,  d_out_7 : vec3<f32>)
{
    var _S52 : DiffPair_matrixx3Cfloatx2C3x2C3x3E_0;
    _S52.primal_0 = ctx_7._S47;
    var _S53 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S53.primal_0 = ctx_7._S48;
    _d_mul_0(&(_S52), &(_S53), d_out_7);
    (*d_left_0) = _S52.differential_0;
    (*d_right_0) = _S53.differential_0;
    return;
}

struct Tuple_8
{
     value0_8 : f32,
};

fn pow0_0( x_7 : f32,  y_2 : f32) -> f32
{
    if((x_7 <= 0.0f))
    {
        return 0.0f;
    }
    return pow(x_7, y_2);
}

fn s_apply_pow0_0( x_8 : f32,  y_3 : f32) -> Tuple_8
{
    var _S54 : Tuple_8 = Tuple_8( pow0_0(x_8, y_3) );
    return _S54;
}

struct s_bwdCallableCtx_pow0_0
{
     _S55 : f32,
     _S56 : f32,
};

fn pow0_backward_0( x_9 : ptr<function, DiffPair_float_0>,  y_4 : ptr<function, DiffPair_float_0>,  d_out_8 : f32)
{
    if((((*x_9).primal_0) <= 0.0f))
    {
        (*x_9).primal_0 = (*x_9).primal_0;
        (*x_9).differential_0 = 0.0f;
        (*y_4).primal_0 = (*y_4).primal_0;
        (*y_4).differential_0 = 0.0f;
        return;
    }
    var d_x_7 : f32 = d_out_8 * (*y_4).primal_0 * pow((*x_9).primal_0, (*y_4).primal_0 - 1.0f);
    var d_y_2 : f32 = d_out_8 * pow((*x_9).primal_0, (*y_4).primal_0) * log((*x_9).primal_0);
    (*x_9).primal_0 = (*x_9).primal_0;
    (*x_9).differential_0 = d_x_7;
    (*y_4).primal_0 = (*y_4).primal_0;
    (*y_4).differential_0 = d_y_2;
    return;
}

fn s_bwdProp_pow0_0( ctx_8 : s_bwdCallableCtx_pow0_0,  d_x_8 : ptr<function, f32>,  d_y_3 : ptr<function, f32>,  d_out_9 : f32)
{
    var _S57 : DiffPair_float_0;
    _S57.primal_0 = ctx_8._S55;
    var _S58 : DiffPair_float_0;
    _S58.primal_0 = ctx_8._S56;
    pow0_backward_0(&(_S57), &(_S58), d_out_9);
    (*d_x_8) = _S57.differential_0;
    (*d_y_3) = _S58.differential_0;
    return;
}

struct Tuple_9
{
     value0_9 : f32,
};

fn s_apply_max_0( x_10 : f32,  y_5 : f32) -> Tuple_9
{
    var _S59 : Tuple_9 = Tuple_9( max(x_10, y_5) );
    return _S59;
}

struct s_bwdCallableCtx_max_0
{
     _S60 : f32,
     _S61 : f32,
};

fn _d_max_0( dpx_7 : ptr<function, DiffPair_float_0>,  dpy_2 : ptr<function, DiffPair_float_0>,  dOut_8 : f32)
{
    var _S62 : DiffPair_float_0 = (*dpx_7);
    var _S63 : f32;
    if((((*dpx_7).primal_0) > ((*dpy_2).primal_0)))
    {
        _S63 = dOut_8;
    }
    else
    {
        if((((*dpx_7).primal_0) < ((*dpy_2).primal_0)))
        {
            _S63 = 0.0f;
        }
        else
        {
            _S63 = 0.5f * dOut_8;
        }
    }
    (*dpx_7).primal_0 = _S62.primal_0;
    (*dpx_7).differential_0 = _S63;
    var _S64 : DiffPair_float_0 = (*dpy_2);
    if((((*dpy_2).primal_0) > (_S62.primal_0)))
    {
        _S63 = dOut_8;
    }
    else
    {
        if((((*dpy_2).primal_0) < ((*dpx_7).primal_0)))
        {
            _S63 = 0.0f;
        }
        else
        {
            _S63 = 0.5f * dOut_8;
        }
    }
    (*dpy_2).primal_0 = _S64.primal_0;
    (*dpy_2).differential_0 = _S63;
    return;
}

fn s_bwdProp_max_0( ctx_9 : s_bwdCallableCtx_max_0,  d_x_9 : ptr<function, f32>,  d_y_4 : ptr<function, f32>,  d_out_10 : f32)
{
    var _S65 : DiffPair_float_0;
    _S65.primal_0 = ctx_9._S60;
    var _S66 : DiffPair_float_0;
    _S66.primal_0 = ctx_9._S61;
    _d_max_0(&(_S65), &(_S66), d_out_10);
    (*d_x_9) = _S65.differential_0;
    (*d_y_4) = _S66.differential_0;
    return;
}

struct Tuple_10
{
     value0_10 : f32,
};

fn s_apply_exp2_0( x_11 : f32) -> Tuple_10
{
    var _S67 : Tuple_10 = Tuple_10( exp2(x_11) );
    return _S67;
}

struct s_bwdCallableCtx_exp2_0
{
     _S68 : f32,
};

fn _d_exp2_0( dpx_8 : ptr<function, DiffPair_float_0>,  dOut_9 : f32)
{
    var _S69 : f32 = exp2((*dpx_8).primal_0) * 0.69314718246459961f * dOut_9;
    (*dpx_8).primal_0 = (*dpx_8).primal_0;
    (*dpx_8).differential_0 = _S69;
    return;
}

fn s_bwdProp_exp2_0( ctx_10 : s_bwdCallableCtx_exp2_0,  d_x_10 : ptr<function, f32>,  d_out_11 : f32)
{
    var _S70 : DiffPair_float_0;
    _S70.primal_0 = ctx_10._S68;
    _d_exp2_0(&(_S70), d_out_11);
    (*d_x_10) = _S70.differential_0;
    return;
}

fn _d_sin_1( dpx_9 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S71 : DiffPair_float_0 = DiffPair_float_0( sin(dpx_9.primal_0), cos(dpx_9.primal_0) * dpx_9.differential_0 );
    return _S71;
}

fn _d_cos_1( dpx_10 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S72 : DiffPair_float_0 = DiffPair_float_0( cos(dpx_10.primal_0), - sin(dpx_10.primal_0) * dpx_10.differential_0 );
    return _S72;
}

fn _d_clamp_1( dpx_11 : DiffPair_float_0,  dpMin_1 : DiffPair_float_0,  dpMax_1 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S73 : f32 = clamp(dpx_11.primal_0, dpMin_1.primal_0, dpMax_1.primal_0);
    var _S74 : bool;
    if(((dpx_11.primal_0) >= (dpMin_1.primal_0)))
    {
        _S74 = ((dpx_11.primal_0) <= (dpMax_1.primal_0));
    }
    else
    {
        _S74 = false;
    }
    var _S75 : f32;
    if(_S74)
    {
        _S75 = dpx_11.differential_0;
    }
    else
    {
        if(((dpx_11.primal_0) < (dpMin_1.primal_0)))
        {
            _S75 = dpMin_1.differential_0;
        }
        else
        {
            _S75 = dpMax_1.differential_0;
        }
    }
    var _S76 : DiffPair_float_0 = DiffPair_float_0( _S73, _S75 );
    return _S76;
}

fn _d_exp_1( dpx_12 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S77 : f32 = exp(dpx_12.primal_0);
    var _S78 : DiffPair_float_0 = DiffPair_float_0( _S77, _S77 * dpx_12.differential_0 );
    return _S78;
}

fn _d_atan2_1( dpy_3 : DiffPair_float_0,  dpx_13 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S79 : f32 = dpx_13.primal_0;
    var _S80 : f32 = _S79 * _S79 + dpy_3.primal_0 * dpy_3.primal_0;
    var _S81 : DiffPair_float_0 = DiffPair_float_0( atan2(dpy_3.primal_0, dpx_13.primal_0), - dpy_3.primal_0 / _S80 * dpx_13.differential_0 + dpx_13.primal_0 / _S80 * dpy_3.differential_0 );
    return _S81;
}

fn _d_exp2_1( dpx_14 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S82 : f32 = exp2(dpx_14.primal_0);
    var _S83 : DiffPair_float_0 = DiffPair_float_0( _S82, _S82 * 0.69314718246459961f * dpx_14.differential_0 );
    return _S83;
}

fn _d_abs_vector_1( dpx_15 : DiffPair_vectorx3Cfloatx2C2x3E_0) -> DiffPair_vectorx3Cfloatx2C2x3E_0
{
    var _S84 : vec2<f32> = vec2<f32>(0.0f);
    var _S85 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( abs(dpx_15.primal_0), select(select(vec2<f32>(-1.0f), _S84, (dpx_15.primal_0) == _S84), vec2<f32>(1.0f), ((dpx_15.primal_0) > _S84)) * dpx_15.differential_0 );
    return _S85;
}

fn _d_min_1( dpx_16 : DiffPair_float_0,  dpy_4 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S86 : f32 = min(dpx_16.primal_0, dpy_4.primal_0);
    var _S87 : f32;
    if(((dpx_16.primal_0) < (dpy_4.primal_0)))
    {
        _S87 = dpx_16.differential_0;
    }
    else
    {
        if(((dpx_16.primal_0) > (dpy_4.primal_0)))
        {
            _S87 = dpy_4.differential_0;
        }
        else
        {
            _S87 = 0.5f * (dpx_16.differential_0 + dpy_4.differential_0);
        }
    }
    var _S88 : DiffPair_float_0 = DiffPair_float_0( _S86, _S87 );
    return _S88;
}

fn _d_max_1( dpx_17 : DiffPair_float_0,  dpy_5 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S89 : f32 = max(dpx_17.primal_0, dpy_5.primal_0);
    var _S90 : f32;
    if(((dpx_17.primal_0) > (dpy_5.primal_0)))
    {
        _S90 = dpx_17.differential_0;
    }
    else
    {
        if(((dpx_17.primal_0) < (dpy_5.primal_0)))
        {
            _S90 = dpy_5.differential_0;
        }
        else
        {
            _S90 = 0.5f * (dpx_17.differential_0 + dpy_5.differential_0);
        }
    }
    var _S91 : DiffPair_float_0 = DiffPair_float_0( _S89, _S90 );
    return _S91;
}

struct Slots_0
{
     hue_red_0 : f32,
     saturation_red_0 : f32,
     brightness_red_0 : f32,
     white_level_0 : f32,
};

fn Slots_x24_syn_dadd_0( SLANG_anonymous_0_0 : Slots_0,  SLANG_anonymous_1_0 : Slots_0) -> Slots_0
{
    var result_0 : Slots_0;
    result_0.hue_red_0 = SLANG_anonymous_0_0.hue_red_0 + SLANG_anonymous_1_0.hue_red_0;
    result_0.saturation_red_0 = SLANG_anonymous_0_0.saturation_red_0 + SLANG_anonymous_1_0.saturation_red_0;
    result_0.brightness_red_0 = SLANG_anonymous_0_0.brightness_red_0 + SLANG_anonymous_1_0.brightness_red_0;
    result_0.white_level_0 = SLANG_anonymous_0_0.white_level_0 + SLANG_anonymous_1_0.white_level_0;
    return result_0;
}

fn Slots_x24_syn_dzero_0() -> Slots_0
{
    var result_1 : Slots_0;
    result_1.hue_red_0 = 0.0f;
    result_1.saturation_red_0 = 0.0f;
    result_1.brightness_red_0 = 0.0f;
    result_1.white_level_0 = 0.0f;
    return result_1;
}

fn d65_xyz_to_xyy_0( xyz_0 : vec3<f32>) -> vec3<f32>
{
    var _S92 : vec3<f32> = max(xyz_0, vec3<f32>(0.0f));
    var _S93 : f32 = _S92.y;
    var sum_6 : f32 = _S92.x + _S93 + _S92.z;
    var xy_0 : vec2<f32>;
    if((sum_6 > 0.0f))
    {
        xy_0 = _S92.xy / vec2<f32>(sum_6);
    }
    else
    {
        xy_0 = vec2<f32>(0.3127099871635437f, 0.32901999354362488f);
    }
    return vec3<f32>(xy_0.x, xy_0.y, _S93);
}

fn away_from_flt_min_0( d_0 : f32) -> f32
{
    var _S94 : f32;
    if((d_0 >= 0.0f))
    {
        _S94 = max(1.17549435082228751e-38f, d_0);
    }
    else
    {
        _S94 = min(-1.17549435082228751e-38f, d_0);
    }
    return _S94;
}

struct Tuple_11
{
     value0_11 : f32,
};

fn s_apply_away_from_flt_min_0( _S95 : f32) -> Tuple_11
{
    var _S96 : f32;
    if((_S95 >= 0.0f))
    {
        _S96 = s_apply_max_0(1.17549435082228751e-38f, _S95).value0_9;
    }
    else
    {
        _S96 = s_apply_min_0(-1.17549435082228751e-38f, _S95).value0_0;
    }
    var _S97 : Tuple_11;
    _S97.value0_11 = _S96;
    return _S97;
}

struct s_paramCtx_s_bwdCallableCtx_away_from_flt_min_0
{
     value0_12 : f32,
};

struct s_bwdCallableCtx_away_from_flt_min_0
{
     value0_13 : s_paramCtx_s_bwdCallableCtx_away_from_flt_min_0,
};

fn s_bwdProp_away_from_flt_min_0( _S98 : s_bwdCallableCtx_away_from_flt_min_0,  _S99 : ptr<function, f32>,  _S100 : f32)
{
    var _S101 : bool = ((_S98.value0_13.value0_12) >= 0.0f);
    var _S102 : s_bwdCallableCtx_max_0;
    var _S103 : s_bwdCallableCtx_min_0;
    if(_S101)
    {
        var _S104 : s_bwdCallableCtx_max_0;
        _S104._S60 = 1.17549435082228751e-38f;
        _S104._S61 = _S98.value0_13.value0_12;
        _S102 = _S104;
        _S103._S2 = 0.0f;
        _S103._S3 = 0.0f;
    }
    else
    {
        var _S105 : s_bwdCallableCtx_min_0;
        _S105._S2 = -1.17549435082228751e-38f;
        _S105._S3 = _S98.value0_13.value0_12;
        _S102._S60 = 0.0f;
        _S102._S61 = 0.0f;
        _S103 = _S105;
    }
    var _S106 : f32;
    if(_S101)
    {
        var _S107 : f32 = 0.0f;
        var _S108 : f32 = 0.0f;
        s_bwdProp_max_0(_S102, &(_S107), &(_S108), _S100);
        _S106 = _S108;
    }
    else
    {
        var _S109 : f32 = 0.0f;
        var _S110 : f32 = 0.0f;
        s_bwdProp_min_0(_S103, &(_S109), &(_S110), _S100);
        _S106 = _S110;
    }
    (*_S99) = _S106;
    return;
}

fn s_fwd_away_from_flt_min_0( dpd_0 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S111 : f32;
    var _S112 : f32;
    if(((dpd_0.primal_0) >= 0.0f))
    {
        var _S113 : DiffPair_float_0 = DiffPair_float_0( 1.17549435082228751e-38f, 0.0f );
        var _S114 : DiffPair_float_0 = DiffPair_float_0( dpd_0.primal_0, dpd_0.differential_0 );
        var _S115 : DiffPair_float_0 = _d_max_1(_S113, _S114);
        _S111 = _S115.primal_0;
        _S112 = _S115.differential_0;
    }
    else
    {
        var _S116 : DiffPair_float_0 = DiffPair_float_0( -1.17549435082228751e-38f, 0.0f );
        var _S117 : DiffPair_float_0 = DiffPair_float_0( dpd_0.primal_0, dpd_0.differential_0 );
        var _S118 : DiffPair_float_0 = _d_min_1(_S116, _S117);
        _S111 = _S118.primal_0;
        _S112 = _S118.differential_0;
    }
    var _S119 : DiffPair_float_0 = DiffPair_float_0( _S111, _S112 );
    return _S119;
}

fn xy_to_ucs_uv_0( xy_1 : vec2<f32>) -> vec2<f32>
{
    var uvd_0 : vec3<f32> = vec3<f32>(-0.78394103050231934f, 0.74527353048324585f, 0.31870728731155396f) * vec3<f32>(xy_1.x) + vec3<f32>(0.27751299738883972f, -0.20537586510181427f, 2.1674368381500244f) * vec3<f32>(xy_1.y) + vec3<f32>(0.15383657813072205f, -0.16547837853431702f, 0.2913205623626709f);
    var uv_1 : vec2<f32> = uvd_0.xy / vec2<f32>(away_from_flt_min_0(uvd_0.z));
    var uv_star_0 : vec2<f32> = vec2<f32>(1.3965622186660767f, 1.4513953924179077f) * uv_1 / (abs(uv_1) + vec2<f32>(1.4921735525131226f, 1.5248863697052002f));
    var _S120 : f32 = uv_star_0.x;
    var _S121 : f32 = uv_star_0.y;
    return vec2<f32>(-1.1249839067459106f * _S120 - 0.98048371076583862f * _S121, 1.863233208656311f * _S120 + 1.9718531370162964f * _S121);
}

fn pow0_forward_0( x_12 : DiffPair_float_0,  y_6 : DiffPair_float_0) -> DiffPair_float_0
{
    if(((x_12.primal_0) <= 0.0f))
    {
        var _S122 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
        return _S122;
    }
    var value_0 : f32 = pow(x_12.primal_0, y_6.primal_0);
    var _S123 : DiffPair_float_0 = DiffPair_float_0( value_0, x_12.differential_0 * y_6.primal_0 * pow(x_12.primal_0, y_6.primal_0 - 1.0f) + y_6.differential_0 * value_0 * log(x_12.primal_0) );
    return _S123;
}

fn y_to_ucs_l_star_0( y_7 : f32) -> f32
{
    var y_hat_0 : f32 = pow0_0(y_7, 0.63165134191513062f);
    return 2.0988838672637939f * y_hat_0 / (y_hat_0 + 1.1242676973342896f);
}

struct Tuple_12
{
     value0_14 : f32,
};

fn s_apply_y_to_ucs_l_star_0( _S124 : f32) -> Tuple_12
{
    var _S125 : Tuple_8 = s_apply_pow0_0(_S124, 0.63165134191513062f);
    var _S126 : Tuple_12;
    _S126.value0_14 = 2.0988838672637939f * _S125.value0_8 / (_S125.value0_8 + 1.1242676973342896f);
    return _S126;
}

struct s_paramCtx_s_bwdCallableCtx_y_to_ucs_l_star_0
{
     value0_15 : f32,
};

struct s_bwdCallableCtx_y_to_ucs_l_star_0
{
     value0_16 : s_paramCtx_s_bwdCallableCtx_y_to_ucs_l_star_0,
};

fn s_bwdProp_y_to_ucs_l_star_0( _S127 : s_bwdCallableCtx_y_to_ucs_l_star_0,  _S128 : ptr<function, f32>,  _S129 : f32)
{
    var _S130 : Tuple_8 = s_apply_pow0_0(_S127.value0_16.value0_15, 0.63165134191513062f);
    var _S131 : s_bwdCallableCtx_pow0_0;
    _S131._S55 = _S127.value0_16.value0_15;
    _S131._S56 = 0.63165134191513062f;
    var _S132 : f32 = _S130.value0_8 + 1.1242676973342896f;
    var _S133 : f32 = _S129 / (_S132 * _S132);
    var _S134 : f32 = 2.0988838672637939f * _S130.value0_8 * - _S133 + 2.0988838672637939f * (_S132 * _S133);
    var _S135 : f32 = 0.0f;
    var _S136 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S131, &(_S135), &(_S136), _S134);
    (*_S128) = _S135;
    return;
}

fn s_fwd_y_to_ucs_l_star_0( dpy_6 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S137 : DiffPair_float_0 = DiffPair_float_0( dpy_6.primal_0, dpy_6.differential_0 );
    var _S138 : DiffPair_float_0 = DiffPair_float_0( 0.63165134191513062f, 0.0f );
    var _S139 : DiffPair_float_0 = pow0_forward_0(_S137, _S138);
    var _S140 : f32 = 2.0988838672637939f * _S139.primal_0;
    var _S141 : f32 = _S139.primal_0 + 1.1242676973342896f;
    var _S142 : DiffPair_float_0 = DiffPair_float_0( _S140 / _S141, (_S139.differential_0 * 2.0988838672637939f * _S141 - _S140 * _S139.differential_0) / (_S141 * _S141) );
    return _S142;
}

fn luv_to_jch_0( l_star_0 : f32,  l_white_0 : f32,  uv_star_1 : vec2<f32>) -> vec3<f32>
{
    var _S143 : f32 = uv_star_1.x;
    var _S144 : f32 = uv_star_1.y;
    return vec3<f32>(l_star_0 / l_white_0, 15.93299388885498f * pow0_0(l_star_0, 0.65239977836608887f) * pow0_0(_S143 * _S143 + _S144 * _S144, 0.60075569152832031f) / l_white_0, atan2(_S144, _S143));
}

struct Tuple_13
{
     value0_17 : vec3<f32>,
};

fn s_apply_luv_to_jch_0( _S145 : f32,  _S146 : f32,  _S147 : vec2<f32>) -> Tuple_13
{
    var _S148 : f32 = _S147.x;
    var _S149 : f32 = _S147.y;
    var _S150 : Tuple_13;
    _S150.value0_17 = vec3<f32>(_S145 / _S146, 15.93299388885498f * s_apply_pow0_0(_S145, 0.65239977836608887f).value0_8 * s_apply_pow0_0(_S148 * _S148 + _S149 * _S149, 0.60075569152832031f).value0_8 / _S146, s_apply_atan2_0(_S149, _S148).value0_6);
    return _S150;
}

struct s_paramCtx_s_bwdCallableCtx_luv_to_jch_0
{
     value0_18 : f32,
     value1_0 : f32,
     value2_0 : vec2<f32>,
};

struct s_bwdCallableCtx_luv_to_jch_0
{
     value0_19 : s_paramCtx_s_bwdCallableCtx_luv_to_jch_0,
};

fn s_bwdProp_luv_to_jch_0( _S151 : s_bwdCallableCtx_luv_to_jch_0,  _S152 : ptr<function, f32>,  _S153 : ptr<function, f32>,  _S154 : ptr<function, vec2<f32>>,  _S155 : vec3<f32>)
{
    var _S156 : f32 = _S151.value0_19.value2_0.x;
    var _S157 : f32 = _S151.value0_19.value2_0.y;
    var m2_0 : f32 = _S156 * _S156 + _S157 * _S157;
    var _S158 : f32 = _S151.value0_19.value1_0;
    var _S159 : f32 = _S158 * _S158;
    var _S160 : Tuple_8 = s_apply_pow0_0(_S151.value0_19.value0_18, 0.65239977836608887f);
    var _S161 : s_bwdCallableCtx_pow0_0;
    _S161._S55 = _S151.value0_19.value0_18;
    _S161._S56 = 0.65239977836608887f;
    var _S162 : f32 = 15.93299388885498f * _S160.value0_8;
    var _S163 : Tuple_8 = s_apply_pow0_0(m2_0, 0.60075569152832031f);
    var _S164 : s_bwdCallableCtx_pow0_0;
    _S164._S55 = m2_0;
    _S164._S56 = 0.60075569152832031f;
    var _S165 : f32 = _S162 * _S163.value0_8;
    var _S166 : s_bwdCallableCtx_atan2_0;
    _S166._S39 = _S157;
    _S166._S40 = _S156;
    var _S167 : f32 = 0.0f;
    var _S168 : f32 = 0.0f;
    s_bwdProp_atan2_0(_S166, &(_S167), &(_S168), _S155[i32(2)]);
    var _S169 : f32 = _S155[i32(1)] / _S159;
    var _S170 : f32 = _S165 * - _S169;
    var _S171 : f32 = _S151.value0_19.value1_0 * _S169;
    var _S172 : f32 = _S162 * _S171;
    var _S173 : f32 = _S163.value0_8 * _S171;
    var _S174 : f32 = 0.0f;
    var _S175 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S164, &(_S174), &(_S175), _S172);
    var _S176 : f32 = 15.93299388885498f * _S173;
    var _S177 : f32 = 0.0f;
    var _S178 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S161, &(_S177), &(_S178), _S176);
    var _S179 : f32 = _S155[i32(0)] / _S159;
    var _S180 : f32 = _S157 * _S174;
    var _S181 : f32 = _S156 * _S174;
    var _S182 : f32 = _S170 + _S151.value0_19.value0_18 * - _S179;
    var _S183 : vec2<f32> = vec2<f32>(_S168 + _S181 + _S181, _S167 + _S180 + _S180);
    (*_S152) = _S177 + _S151.value0_19.value1_0 * _S179;
    (*_S153) = _S182;
    (*_S154) = _S183;
    return;
}

fn s_fwd_luv_to_jch_0( dpl_star_0 : DiffPair_float_0,  dpl_white_0 : DiffPair_float_0,  dpuv_star_0 : DiffPair_vectorx3Cfloatx2C2x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S184 : f32 = dpuv_star_0.primal_0.x;
    var _S185 : f32 = dpuv_star_0.differential_0.x;
    var _S186 : f32 = _S185 * _S184;
    var _S187 : f32 = dpuv_star_0.primal_0.y;
    var _S188 : f32 = dpuv_star_0.differential_0.y;
    var _S189 : f32 = _S188 * _S187;
    var _S190 : f32 = dpl_white_0.primal_0 * dpl_white_0.primal_0;
    var _S191 : DiffPair_float_0 = DiffPair_float_0( dpl_star_0.primal_0, dpl_star_0.differential_0 );
    var _S192 : DiffPair_float_0 = DiffPair_float_0( 0.65239977836608887f, 0.0f );
    var _S193 : DiffPair_float_0 = pow0_forward_0(_S191, _S192);
    var _S194 : f32 = 15.93299388885498f * _S193.primal_0;
    var _S195 : DiffPair_float_0 = DiffPair_float_0( _S184 * _S184 + _S187 * _S187, _S186 + _S186 + (_S189 + _S189) );
    var _S196 : DiffPair_float_0 = DiffPair_float_0( 0.60075569152832031f, 0.0f );
    var _S197 : DiffPair_float_0 = pow0_forward_0(_S195, _S196);
    var _S198 : f32 = _S194 * _S197.primal_0;
    var _S199 : DiffPair_float_0 = DiffPair_float_0( _S187, _S188 );
    var _S200 : DiffPair_float_0 = DiffPair_float_0( _S184, _S185 );
    var _S201 : DiffPair_float_0 = _d_atan2_1(_S199, _S200);
    var _S202 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(dpl_star_0.primal_0 / dpl_white_0.primal_0, _S198 / dpl_white_0.primal_0, _S201.primal_0), vec3<f32>((dpl_star_0.differential_0 * dpl_white_0.primal_0 - dpl_star_0.primal_0 * dpl_white_0.differential_0) / _S190, ((_S193.differential_0 * 15.93299388885498f * _S197.primal_0 + _S197.differential_0 * _S194) * dpl_white_0.primal_0 - _S198 * dpl_white_0.differential_0) / _S190, _S201.differential_0) );
    return _S202;
}

fn ucs_jch_to_hsb_0( jch_0 : vec3<f32>) -> vec3<f32>
{
    var _S203 : f32 = jch_0.y;
    var b_0 : f32 = jch_0.x * (pow0_0(_S203, 1.3365422487258911f) + 1.0f);
    var s_0 : f32;
    if((b_0 > 0.0f))
    {
        s_0 = _S203 / b_0;
    }
    else
    {
        s_0 = 0.0f;
    }
    return vec3<f32>(jch_0.z, s_0, b_0);
}

struct Tuple_14
{
     value0_20 : vec3<f32>,
};

fn s_apply_ucs_jch_to_hsb_0( _S204 : vec3<f32>) -> Tuple_14
{
    var _S205 : f32 = _S204.y;
    var b_1 : f32 = _S204.x * (s_apply_pow0_0(_S205, 1.3365422487258911f).value0_8 + 1.0f);
    var s_1 : f32;
    if((b_1 > 0.0f))
    {
        s_1 = _S205 / b_1;
    }
    else
    {
        s_1 = 0.0f;
    }
    var _S206 : Tuple_14;
    _S206.value0_20 = vec3<f32>(_S204.z, s_1, b_1);
    return _S206;
}

struct s_paramCtx_s_bwdCallableCtx_ucs_jch_to_hsb_0
{
     value0_21 : vec3<f32>,
};

struct s_bwdCallableCtx_ucs_jch_to_hsb_0
{
     value0_22 : s_paramCtx_s_bwdCallableCtx_ucs_jch_to_hsb_0,
};

fn s_bwdProp_ucs_jch_to_hsb_0( _S207 : s_bwdCallableCtx_ucs_jch_to_hsb_0,  _S208 : ptr<function, vec3<f32>>,  _S209 : vec3<f32>)
{
    var _S210 : f32 = _S207.value0_22.value0_21.x;
    var _S211 : f32 = _S207.value0_22.value0_21.y;
    var _S212 : Tuple_8 = s_apply_pow0_0(_S211, 1.3365422487258911f);
    var _S213 : s_bwdCallableCtx_pow0_0;
    _S213._S55 = _S211;
    _S213._S56 = 1.3365422487258911f;
    var _S214 : s_bwdCallableCtx_pow0_0 = _S213;
    var _S215 : f32 = _S212.value0_8 + 1.0f;
    var b_2 : f32 = _S210 * _S215;
    var _S216 : bool = (b_2 > 0.0f);
    var _S217 : f32;
    if(_S216)
    {
        _S217 = b_2 * b_2;
    }
    else
    {
        _S217 = 0.0f;
    }
    var _S218 : vec3<f32> = vec3<f32>(0.0f, 0.0f, _S209[i32(0)]);
    var _S219 : f32;
    if(_S216)
    {
        var _S220 : f32 = _S209[i32(1)] / _S217;
        var _S221 : f32 = b_2 * _S220;
        _S217 = _S211 * - _S220 + _S209[i32(2)];
        _S219 = _S221;
    }
    else
    {
        _S217 = _S209[i32(2)];
        _S219 = 0.0f;
    }
    var _S222 : f32 = _S210 * _S217;
    var _S223 : f32 = _S215 * _S217;
    var _S224 : f32 = 0.0f;
    var _S225 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S214, &(_S224), &(_S225), _S222);
    (*_S208) = _S218 + vec3<f32>(_S223, _S224 + _S219, 0.0f);
    return;
}

fn s_fwd_ucs_jch_to_hsb_0( dpjch_0 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S226 : f32 = dpjch_0.primal_0.x;
    var _S227 : f32 = dpjch_0.primal_0.y;
    var _S228 : f32 = dpjch_0.differential_0.y;
    var _S229 : DiffPair_float_0 = DiffPair_float_0( _S227, _S228 );
    var _S230 : DiffPair_float_0 = DiffPair_float_0( 1.3365422487258911f, 0.0f );
    var _S231 : DiffPair_float_0 = pow0_forward_0(_S229, _S230);
    var _S232 : f32 = _S231.primal_0 + 1.0f;
    var b_3 : f32 = _S226 * _S232;
    var s_diff_b_0 : f32 = dpjch_0.differential_0.x * _S232 + _S231.differential_0 * _S226;
    var s_2 : f32;
    var s_diff_s_0 : f32;
    if((b_3 > 0.0f))
    {
        var _S233 : f32 = (_S228 * b_3 - _S227 * s_diff_b_0) / (b_3 * b_3);
        s_2 = _S227 / b_3;
        s_diff_s_0 = _S233;
    }
    else
    {
        s_2 = 0.0f;
        s_diff_s_0 = 0.0f;
    }
    var _S234 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(dpjch_0.primal_0.z, s_2, b_3), vec3<f32>(dpjch_0.differential_0.z, s_diff_s_0, s_diff_b_0) );
    return _S234;
}

fn lookup_0( offset_0 : u32,  hue_0 : f32) -> f32
{
    var x_13 : f32 = 512.0f * (hue_0 + 3.1415927410125732f) / 6.2831854820251465f;
    var x_prev_0 : f32 = floor(x_13);
    var i_0 : u32 = ((bitcast<u32>((i32(x_prev_0)))) & (u32(511)));
    var y_8 : f32 = table_0[offset_0 + i_0];
    return y_8 + (x_13 - x_prev_0) * (table_0[offset_0 + (((i_0 + u32(1)) & (u32(511))))] - y_8);
}

fn s_fwd_lookup_0( offset_1 : u32,  dphue_0 : DiffPair_float_0) -> DiffPair_float_0
{
    var x_14 : f32 = 512.0f * (dphue_0.primal_0 + 3.1415927410125732f) / 6.2831854820251465f;
    var x_prev_1 : f32 = floor(x_14);
    var i_1 : u32 = ((bitcast<u32>((i32(x_prev_1)))) & (u32(511)));
    var y_9 : f32 = table_0[offset_1 + i_1];
    var _S235 : f32 = table_0[offset_1 + (((i_1 + u32(1)) & (u32(511))))] - y_9;
    var _S236 : DiffPair_float_0 = DiffPair_float_0( y_9 + (x_14 - x_prev_1) * _S235, dphue_0.differential_0 * 512.0f * 0.15915493667125702f * _S235 );
    return _S236;
}

struct s_minCtx_s_bwdCallableCtx_lookup_0
{
     value0_23 : f32,
     value1_1 : f32,
};

struct Tuple_15
{
     value0_24 : f32,
     value1_2 : s_minCtx_s_bwdCallableCtx_lookup_0,
};

fn s_apply_lookup_0( offset_2 : u32,  _S237 : f32) -> Tuple_15
{
    var _S238 : s_minCtx_s_bwdCallableCtx_lookup_0;
    _S238.value0_23 = 0.0f;
    _S238.value1_1 = 0.0f;
    var x_15 : f32 = 512.0f * (_S237 + 3.1415927410125732f) / 6.2831854820251465f;
    var x_prev_2 : f32 = floor(x_15);
    var i_2 : u32 = ((bitcast<u32>((i32(x_prev_2)))) & (u32(511)));
    var j_0 : u32 = ((i_2 + u32(1)) & (u32(511)));
    var y_10 : f32 = table_0[offset_2 + i_2];
    _S238.value0_23 = y_10;
    var _S239 : f32 = x_15 - x_prev_2;
    var _S240 : f32 = table_0[offset_2 + j_0];
    _S238.value1_1 = _S240;
    var _S241 : Tuple_15;
    _S241.value0_24 = y_10 + _S239 * (_S240 - y_10);
    _S241.value1_2 = _S238;
    return _S241;
}

struct s_bwdCallableCtx_lookup_0
{
     value1_3 : s_minCtx_s_bwdCallableCtx_lookup_0,
};

fn s_bwdProp_lookup_0( _S242 : s_bwdCallableCtx_lookup_0,  _S243 : ptr<function, f32>,  _S244 : f32)
{
    (*_S243) = 512.0f * (0.15915493667125702f * ((_S242.value1_3.value1_1 - _S242.value1_3.value0_23) * _S244));
    return;
}

fn curve_0( offset_3 : u32,  derivative_0 : u32,  hue_1 : f32,  slot_0 : f32,  value_1 : f32,  slots_1 : bool) -> f32
{
    var v_0 : f32 = lookup_0(offset_3, hue_1);
    var v_1 : f32;
    if(slots_1)
    {
        v_1 = v_0 + lookup_0(derivative_0, hue_1) * (slot_0 - value_1);
    }
    else
    {
        v_1 = v_0;
    }
    return v_1;
}

fn s_fwd_curve_0( offset_4 : u32,  derivative_1 : u32,  dphue_1 : DiffPair_float_0,  dpslot_0 : DiffPair_float_0,  dpvalue_0 : DiffPair_float_0,  slots_2 : bool) -> DiffPair_float_0
{
    var _S245 : DiffPair_float_0 = DiffPair_float_0( dphue_1.primal_0, dphue_1.differential_0 );
    var _S246 : DiffPair_float_0 = s_fwd_lookup_0(offset_4, _S245);
    var v_2 : f32;
    var s_diff_v_0 : f32;
    if(slots_2)
    {
        var _S247 : DiffPair_float_0 = s_fwd_lookup_0(derivative_1, _S245);
        var _S248 : f32 = dpslot_0.primal_0 - dpvalue_0.primal_0;
        var _S249 : f32 = _S246.differential_0 + (_S247.differential_0 * _S248 + (dpslot_0.differential_0 - dpvalue_0.differential_0) * _S247.primal_0);
        v_2 = _S246.primal_0 + _S247.primal_0 * _S248;
        s_diff_v_0 = _S249;
    }
    else
    {
        v_2 = _S246.primal_0;
        s_diff_v_0 = _S246.differential_0;
    }
    var _S250 : DiffPair_float_0 = DiffPair_float_0( v_2, s_diff_v_0 );
    return _S250;
}

struct s_minCtx_s_bwdCallableCtx_curve_0
{
     value0_25 : s_minCtx_s_bwdCallableCtx_lookup_0,
     value1_4 : f32,
     value2_1 : s_minCtx_s_bwdCallableCtx_lookup_0,
};

struct Tuple_16
{
     value0_26 : f32,
     value1_5 : s_minCtx_s_bwdCallableCtx_curve_0,
};

fn s_apply_curve_0( offset_5 : u32,  derivative_2 : u32,  _S251 : f32,  _S252 : f32,  _S253 : f32,  slots_3 : bool) -> Tuple_16
{
    var _S254 : s_minCtx_s_bwdCallableCtx_lookup_0 = s_minCtx_s_bwdCallableCtx_lookup_0( 0.0f, 0.0f );
    var _S255 : s_minCtx_s_bwdCallableCtx_curve_0;
    _S255.value0_25 = _S254;
    _S255.value1_4 = 0.0f;
    _S255.value2_1 = _S254;
    var _S256 : Tuple_15 = s_apply_lookup_0(offset_5, _S251);
    _S255.value0_25 = _S256.value1_2;
    var v_3 : f32;
    if(slots_3)
    {
        var _S257 : Tuple_15 = s_apply_lookup_0(derivative_2, _S251);
        _S255.value1_4 = _S257.value0_24;
        _S255.value2_1 = _S257.value1_2;
        v_3 = _S256.value0_24 + _S257.value0_24 * (_S252 - _S253);
    }
    else
    {
        v_3 = _S256.value0_24;
    }
    var _S258 : Tuple_16;
    _S258.value0_26 = v_3;
    _S258.value1_5 = _S255;
    return _S258;
}

struct s_paramCtx_s_bwdCallableCtx_curve_0
{
     value3_0 : f32,
     value4_0 : f32,
     value5_0 : bool,
};

struct s_bwdCallableCtx_curve_0
{
     value0_27 : s_paramCtx_s_bwdCallableCtx_curve_0,
     value1_6 : s_minCtx_s_bwdCallableCtx_curve_0,
};

fn s_bwdProp_curve_0( _S259 : s_bwdCallableCtx_curve_0,  _S260 : ptr<function, f32>,  _S261 : ptr<function, f32>,  _S262 : ptr<function, f32>,  _S263 : f32)
{
    var _S264 : s_minCtx_s_bwdCallableCtx_lookup_0 = s_minCtx_s_bwdCallableCtx_lookup_0( 0.0f, 0.0f );
    var _S265 : s_bwdCallableCtx_lookup_0 = s_bwdCallableCtx_lookup_0( _S259.value1_6.value0_25 );
    var _S266 : f32;
    var _S267 : s_bwdCallableCtx_lookup_0;
    if(_S259.value0_27.value5_0)
    {
        _S266 = _S259.value0_27.value3_0 - _S259.value0_27.value4_0;
        _S267.value1_3 = _S259.value1_6.value2_1;
    }
    else
    {
        _S266 = 0.0f;
        _S267.value1_3 = _S264;
    }
    var _S268 : f32;
    var _S269 : f32;
    if(_S259.value0_27.value5_0)
    {
        var _S270 : f32 = _S259.value1_6.value1_4 * _S263;
        var _S271 : f32 = _S266 * _S263;
        var _S272 : f32 = - _S270;
        var _S273 : f32 = 0.0f;
        s_bwdProp_lookup_0(_S267, &(_S273), _S271);
        _S266 = _S273;
        _S268 = _S270;
        _S269 = _S272;
    }
    else
    {
        _S266 = 0.0f;
        _S268 = 0.0f;
        _S269 = 0.0f;
    }
    var _S274 : f32 = 0.0f;
    s_bwdProp_lookup_0(_S265, &(_S274), _S263);
    (*_S260) = _S274 + _S266;
    (*_S261) = _S268;
    (*_S262) = _S269;
    return;
}

fn ucs_hsb_to_jch_0( hsb_0 : vec3<f32>) -> vec3<f32>
{
    var _S275 : f32 = hsb_0.z;
    var c_0 : f32 = hsb_0.y * _S275;
    return vec3<f32>(_S275 / (pow0_0(c_0, 1.3365422487258911f) + 1.0f), c_0, hsb_0.x);
}

struct Tuple_17
{
     value0_28 : vec3<f32>,
};

fn s_apply_ucs_hsb_to_jch_0( _S276 : vec3<f32>) -> Tuple_17
{
    var _S277 : f32 = _S276.z;
    var c_1 : f32 = _S276.y * _S277;
    var _S278 : Tuple_17;
    _S278.value0_28 = vec3<f32>(_S277 / (s_apply_pow0_0(c_1, 1.3365422487258911f).value0_8 + 1.0f), c_1, _S276.x);
    return _S278;
}

struct s_paramCtx_s_bwdCallableCtx_ucs_hsb_to_jch_0
{
     value0_29 : vec3<f32>,
};

struct s_bwdCallableCtx_ucs_hsb_to_jch_0
{
     value0_30 : s_paramCtx_s_bwdCallableCtx_ucs_hsb_to_jch_0,
};

fn s_bwdProp_ucs_hsb_to_jch_0( _S279 : s_bwdCallableCtx_ucs_hsb_to_jch_0,  _S280 : ptr<function, vec3<f32>>,  _S281 : vec3<f32>)
{
    var _S282 : f32 = _S279.value0_30.value0_29.y;
    var _S283 : f32 = _S279.value0_30.value0_29.z;
    var c_2 : f32 = _S282 * _S283;
    var _S284 : Tuple_8 = s_apply_pow0_0(c_2, 1.3365422487258911f);
    var _S285 : s_bwdCallableCtx_pow0_0;
    _S285._S55 = c_2;
    _S285._S56 = 1.3365422487258911f;
    var _S286 : f32 = _S284.value0_8 + 1.0f;
    var _S287 : f32 = _S281[i32(0)] / (_S286 * _S286);
    var _S288 : f32 = _S283 * - _S287;
    var _S289 : f32 = _S286 * _S287;
    var _S290 : f32 = 0.0f;
    var _S291 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S285, &(_S290), &(_S291), _S288);
    var _S292 : f32 = _S281[i32(1)] + _S290;
    (*_S280) = vec3<f32>(_S281[i32(2)], _S283 * _S292, _S289 + _S282 * _S292);
    return;
}

fn s_fwd_ucs_hsb_to_jch_0( dphsb_0 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S293 : f32 = dphsb_0.primal_0.y;
    var _S294 : f32 = dphsb_0.primal_0.z;
    var _S295 : f32 = dphsb_0.differential_0.z;
    var c_3 : f32 = _S293 * _S294;
    var s_diff_c_0 : f32 = dphsb_0.differential_0.y * _S294 + _S295 * _S293;
    var _S296 : DiffPair_float_0 = DiffPair_float_0( c_3, s_diff_c_0 );
    var _S297 : DiffPair_float_0 = DiffPair_float_0( 1.3365422487258911f, 0.0f );
    var _S298 : DiffPair_float_0 = pow0_forward_0(_S296, _S297);
    var _S299 : f32 = _S298.primal_0 + 1.0f;
    var _S300 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S294 / _S299, c_3, dphsb_0.primal_0.x), vec3<f32>((_S295 * _S299 - _S294 * _S298.differential_0) / (_S299 * _S299), s_diff_c_0, dphsb_0.differential_0.x) );
    return _S300;
}

fn soft_clip_0( x_16 : f32,  soft_0 : f32,  hard_0 : f32) -> f32
{
    if((x_16 <= soft_0))
    {
        return x_16;
    }
    var norm_0 : f32 = hard_0 - soft_0;
    if((norm_0 <= 0.0f))
    {
        return soft_0;
    }
    return soft_0 + (1.0f - exp(- (x_16 - soft_0) / norm_0)) * norm_0;
}

struct Tuple_18
{
     value0_31 : f32,
};

fn s_apply_soft_clip_0( _S301 : f32,  _S302 : f32,  _S303 : f32) -> Tuple_18
{
    var _S304 : bool = !(_S301 <= _S302);
    var _S305 : f32;
    if(_S304)
    {
        var norm_1 : f32 = _S303 - _S302;
        var _runFlag_0 : bool;
        if((norm_1 <= 0.0f))
        {
            _runFlag_0 = false;
            _S305 = _S302;
        }
        else
        {
            _runFlag_0 = _S304;
            _S305 = _S301;
        }
        if(_runFlag_0)
        {
            _S305 = _S302 + (1.0f - s_apply_exp_0(- (_S301 - _S302) / norm_1).value0_5) * norm_1;
        }
    }
    else
    {
        _S305 = _S301;
    }
    var _S306 : Tuple_18;
    _S306.value0_31 = _S305;
    return _S306;
}

struct s_paramCtx_s_bwdCallableCtx_soft_clip_0
{
     value0_32 : f32,
     value1_7 : f32,
     value2_2 : f32,
};

struct s_bwdCallableCtx_soft_clip_0
{
     value0_33 : s_paramCtx_s_bwdCallableCtx_soft_clip_0,
};

fn s_bwdProp_soft_clip_0( _S307 : s_bwdCallableCtx_soft_clip_0,  _S308 : ptr<function, f32>,  _S309 : ptr<function, f32>,  _S310 : ptr<function, f32>,  _S311 : f32)
{
    var _S312 : bool = ((_S307.value0_33.value0_32) <= (_S307.value0_33.value1_7));
    var _S313 : bool = !_S312;
    var _runFlag_1 : bool;
    var _S314 : bool;
    var _S315 : f32;
    var _S316 : f32;
    var _S317 : f32;
    var _S318 : f32;
    var _S319 : s_bwdCallableCtx_exp_0;
    if(_S313)
    {
        var norm_2 : f32 = _S307.value0_33.value2_2 - _S307.value0_33.value1_7;
        var _S320 : bool = (norm_2 <= 0.0f);
        if(_S320)
        {
            _runFlag_1 = false;
        }
        else
        {
            _runFlag_1 = _S313;
        }
        if(_runFlag_1)
        {
            var _S321 : f32 = - (_S307.value0_33.value0_32 - _S307.value0_33.value1_7);
            var _S322 : f32 = _S321 / norm_2;
            var _S323 : f32 = norm_2 * norm_2;
            var _S324 : Tuple_5 = s_apply_exp_0(_S322);
            var _S325 : s_bwdCallableCtx_exp_0;
            _S325._S35 = _S322;
            _S315 = 1.0f - _S324.value0_5;
            _S319 = _S325;
            _S316 = _S323;
            _S317 = _S321;
        }
        else
        {
            _S315 = 0.0f;
            _S319._S35 = 0.0f;
            _S316 = 0.0f;
            _S317 = 0.0f;
        }
        var _S326 : f32 = _S316;
        var _S327 : f32 = _S317;
        _S316 = norm_2;
        _S317 = _S326;
        _S318 = _S327;
        _S314 = _S320;
    }
    else
    {
        _runFlag_1 = false;
        _S315 = 0.0f;
        _S316 = 0.0f;
        _S319._S35 = 0.0f;
        _S317 = 0.0f;
        _S318 = 0.0f;
        _S314 = false;
    }
    if(_S313)
    {
        if(_runFlag_1)
        {
            var _S328 : f32 = _S315 * _S311;
            var _S329 : f32 = - (_S316 * _S311);
            var _S330 : f32 = 0.0f;
            s_bwdProp_exp_0(_S319, &(_S330), _S329);
            var _S331 : f32 = _S330 / _S317;
            var _S332 : f32 = - (_S316 * _S331);
            var _S333 : f32 = _S311 + - _S332;
            var _S334 : f32 = _S328 + _S318 * - _S331;
            _S315 = 0.0f;
            _S316 = _S333;
            _S317 = _S334;
            _S318 = _S332;
        }
        else
        {
            _S315 = _S311;
            _S316 = 0.0f;
            _S317 = 0.0f;
            _S318 = 0.0f;
        }
        if(_S314)
        {
            _S315 = _S315 + _S316;
            _S316 = 0.0f;
        }
        else
        {
            var _S335 : f32 = _S315;
            _S315 = _S316;
            _S316 = _S335;
        }
        var _S336 : f32 = - _S317 + _S315;
        var _S337 : f32 = _S317;
        _S315 = _S316;
        _S316 = _S318;
        _S317 = _S336;
        _S318 = _S337;
    }
    else
    {
        _S315 = _S311;
        _S316 = 0.0f;
        _S317 = 0.0f;
        _S318 = 0.0f;
    }
    if(_S312)
    {
        _S315 = _S315 + _S316;
    }
    else
    {
        _S315 = _S316;
    }
    (*_S308) = _S315;
    (*_S309) = _S317;
    (*_S310) = _S318;
    return;
}

fn s_fwd_soft_clip_0( dpx_18 : DiffPair_float_0,  dpsoft_0 : DiffPair_float_0,  dphard_0 : DiffPair_float_0) -> DiffPair_float_0
{
    if(((dpx_18.primal_0) <= (dpsoft_0.primal_0)))
    {
        var _S338 : DiffPair_float_0 = DiffPair_float_0( dpx_18.primal_0, dpx_18.differential_0 );
        return _S338;
    }
    var norm_3 : f32 = dphard_0.primal_0 - dpsoft_0.primal_0;
    var s_diff_norm_0 : f32 = dphard_0.differential_0 - dpsoft_0.differential_0;
    if((norm_3 <= 0.0f))
    {
        var _S339 : DiffPair_float_0 = DiffPair_float_0( dpsoft_0.primal_0, dpsoft_0.differential_0 );
        return _S339;
    }
    var _S340 : f32 = - (dpx_18.primal_0 - dpsoft_0.primal_0);
    var _S341 : DiffPair_float_0 = DiffPair_float_0( _S340 / norm_3, (- (dpx_18.differential_0 - dpsoft_0.differential_0) * norm_3 - _S340 * s_diff_norm_0) / (norm_3 * norm_3) );
    var _S342 : DiffPair_float_0 = _d_exp_1(_S341);
    var _S343 : f32 = 1.0f - _S342.primal_0;
    var _S344 : DiffPair_float_0 = DiffPair_float_0( dpsoft_0.primal_0 + _S343 * norm_3, dpsoft_0.differential_0 + ((0.0f - _S342.differential_0) * norm_3 + s_diff_norm_0 * _S343) );
    return _S344;
}

fn ucs_l_star_to_y_0( l_star_1 : f32) -> f32
{
    return pow0_0(1.1242676973342896f * l_star_1 / (2.0988838672637939f - l_star_1), 1.5831518173217773f);
}

struct Tuple_19
{
     value0_34 : f32,
};

fn s_apply_ucs_l_star_to_y_0( _S345 : f32) -> Tuple_19
{
    var _S346 : Tuple_19;
    _S346.value0_34 = s_apply_pow0_0(1.1242676973342896f * _S345 / (2.0988838672637939f - _S345), 1.5831518173217773f).value0_8;
    return _S346;
}

struct s_paramCtx_s_bwdCallableCtx_ucs_l_star_to_y_0
{
     value0_35 : f32,
};

struct s_bwdCallableCtx_ucs_l_star_to_y_0
{
     value0_36 : s_paramCtx_s_bwdCallableCtx_ucs_l_star_to_y_0,
};

fn s_bwdProp_ucs_l_star_to_y_0( _S347 : s_bwdCallableCtx_ucs_l_star_to_y_0,  _S348 : ptr<function, f32>,  _S349 : f32)
{
    var _S350 : f32 = 1.1242676973342896f * _S347.value0_36.value0_35;
    var _S351 : f32 = 2.0988838672637939f - _S347.value0_36.value0_35;
    var _S352 : f32 = _S351 * _S351;
    var _S353 : s_bwdCallableCtx_pow0_0;
    _S353._S55 = _S350 / _S351;
    _S353._S56 = 1.5831518173217773f;
    var _S354 : f32 = 0.0f;
    var _S355 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S353, &(_S354), &(_S355), _S349);
    var _S356 : f32 = _S354 / _S352;
    (*_S348) = - (_S350 * - _S356) + 1.1242676973342896f * (_S351 * _S356);
    return;
}

fn s_fwd_ucs_l_star_to_y_0( dpl_star_1 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S357 : f32 = 1.1242676973342896f * dpl_star_1.primal_0;
    var _S358 : f32 = 2.0988838672637939f - dpl_star_1.primal_0;
    var _S359 : DiffPair_float_0 = DiffPair_float_0( _S357 / _S358, (dpl_star_1.differential_0 * 1.1242676973342896f * _S358 - _S357 * (0.0f - dpl_star_1.differential_0)) / (_S358 * _S358) );
    var _S360 : DiffPair_float_0 = DiffPair_float_0( 1.5831518173217773f, 0.0f );
    var _S361 : DiffPair_float_0 = pow0_forward_0(_S359, _S360);
    var _S362 : DiffPair_float_0 = DiffPair_float_0( _S361.primal_0, _S361.differential_0 );
    return _S362;
}

fn ucs_jch_to_xyy_0( jch_1 : vec3<f32>,  l_white_1 : f32) -> vec3<f32>
{
    var l_star_2 : f32 = clamp(jch_1.x * l_white_1, 0.0f, 2.0988500118255615f);
    var m_0 : f32;
    if(l_star_2 != 0.0f)
    {
        m_0 = pow0_0(jch_1.y * l_white_1 / (15.93299388885498f * pow0_0(l_star_2, 0.65239977836608887f)), 0.83228504657745361f);
    }
    else
    {
        m_0 = 0.0f;
    }
    var _S363 : f32 = jch_1.z;
    var uv_star_prime_0 : vec2<f32> = vec2<f32>(m_0) * vec2<f32>(cos(_S363), sin(_S363));
    var _S364 : f32 = uv_star_prime_0.x;
    var _S365 : f32 = uv_star_prime_0.y;
    var uv_star_2 : vec2<f32> = vec2<f32>(-5.0375223159790039f * _S364 - 2.5048563480377197f * _S365, 4.7600293159484863f * _S364 + 2.8740129470825195f * _S365);
    var uv_2 : vec2<f32> = (vec2<f32>(0) - vec2<f32>(1.4921735525131226f, 1.5248863697052002f)) * uv_star_2 / (abs(uv_star_2) - vec2<f32>(1.3965622186660767f, 1.4513953924179077f));
    var xyd_0 : vec3<f32> = vec3<f32>(0.16717147827148438f, -0.15095908939838409f, 0.94025474786758423f) * vec3<f32>(uv_2.x) + vec3<f32>(0.14129979908466339f, -0.15518505871295929f, 1.0f) * vec3<f32>(uv_2.y) + vec3<f32>(-0.0080153131857514381f, -0.0084331240504980087f, -0.025632597506046295f);
    var xy_2 : vec2<f32> = xyd_0.xy / vec2<f32>(away_from_flt_min_0(xyd_0.z));
    return vec3<f32>(xy_2.x, xy_2.y, ucs_l_star_to_y_0(l_star_2));
}

struct Tuple_20
{
     value0_37 : vec3<f32>,
};

fn s_apply_ucs_jch_to_xyy_0( _S366 : vec3<f32>,  l_white_2 : f32) -> Tuple_20
{
    var _S367 : Tuple_4 = s_apply_clamp_0(_S366.x * l_white_2, 0.0f, 2.0988500118255615f);
    var m_1 : f32;
    if((_S367.value0_4) != 0.0f)
    {
        m_1 = s_apply_pow0_0(_S366.y * l_white_2 / (15.93299388885498f * s_apply_pow0_0(_S367.value0_4, 0.65239977836608887f).value0_8), 0.83228504657745361f).value0_8;
    }
    else
    {
        m_1 = 0.0f;
    }
    var _S368 : f32 = _S366.z;
    var uv_star_prime_1 : vec2<f32> = vec2<f32>(m_1) * vec2<f32>(s_apply_cos_0(_S368).value0_3, s_apply_sin_0(_S368).value0_2);
    var _S369 : f32 = uv_star_prime_1.x;
    var _S370 : f32 = uv_star_prime_1.y;
    var uv_star_3 : vec2<f32> = vec2<f32>(-5.0375223159790039f * _S369 - 2.5048563480377197f * _S370, 4.7600293159484863f * _S369 + 2.8740129470825195f * _S370);
    var uv_3 : vec2<f32> = (vec2<f32>(0) - vec2<f32>(1.4921735525131226f, 1.5248863697052002f)) * uv_star_3 / (s_apply_abs_0(uv_star_3).value0_1 - vec2<f32>(1.3965622186660767f, 1.4513953924179077f));
    var xyd_1 : vec3<f32> = vec3<f32>(0.16717147827148438f, -0.15095908939838409f, 0.94025474786758423f) * vec3<f32>(uv_3.x) + vec3<f32>(0.14129979908466339f, -0.15518505871295929f, 1.0f) * vec3<f32>(uv_3.y) + vec3<f32>(-0.0080153131857514381f, -0.0084331240504980087f, -0.025632597506046295f);
    var xy_3 : vec2<f32> = xyd_1.xy / vec2<f32>(s_apply_away_from_flt_min_0(xyd_1.z).value0_11);
    var _S371 : Tuple_20;
    _S371.value0_37 = vec3<f32>(xy_3.x, xy_3.y, s_apply_ucs_l_star_to_y_0(_S367.value0_4).value0_34);
    return _S371;
}

struct s_paramCtx_s_bwdCallableCtx_ucs_jch_to_xyy_0
{
     value0_38 : vec3<f32>,
     value1_8 : f32,
};

struct s_bwdCallableCtx_ucs_jch_to_xyy_0
{
     value0_39 : s_paramCtx_s_bwdCallableCtx_ucs_jch_to_xyy_0,
};

fn s_bwdProp_ucs_jch_to_xyy_0( _S372 : s_bwdCallableCtx_ucs_jch_to_xyy_0,  _S373 : ptr<function, vec3<f32>>,  _S374 : vec3<f32>)
{
    var _S375 : f32 = _S372.value0_39.value0_38.x * _S372.value0_39.value1_8;
    var _S376 : Tuple_4 = s_apply_clamp_0(_S375, 0.0f, 2.0988500118255615f);
    var _S377 : s_bwdCallableCtx_clamp_0;
    _S377._S23 = _S375;
    _S377._S24 = 0.0f;
    _S377._S25 = 2.0988500118255615f;
    var _S378 : s_bwdCallableCtx_clamp_0 = _S377;
    var _S379 : bool = (_S376.value0_4) != 0.0f;
    var m_2 : f32;
    var _S380 : f32;
    var _S381 : f32;
    var _S382 : f32;
    var _S383 : s_bwdCallableCtx_pow0_0;
    var _S384 : s_bwdCallableCtx_pow0_0;
    if(_S379)
    {
        var _S385 : f32 = _S372.value0_39.value0_38.y * _S372.value0_39.value1_8;
        var _S386 : Tuple_8 = s_apply_pow0_0(_S376.value0_4, 0.65239977836608887f);
        var _S387 : s_bwdCallableCtx_pow0_0;
        _S387._S55 = _S376.value0_4;
        _S387._S56 = 0.65239977836608887f;
        var _S388 : f32 = 15.93299388885498f * _S386.value0_8;
        var _S389 : f32 = _S385 / _S388;
        var _S390 : f32 = _S388 * _S388;
        var _S391 : Tuple_8 = s_apply_pow0_0(_S389, 0.83228504657745361f);
        var _S392 : s_bwdCallableCtx_pow0_0;
        _S392._S55 = _S389;
        _S392._S56 = 0.83228504657745361f;
        m_2 = _S391.value0_8;
        _S383 = _S392;
        _S380 = _S390;
        _S381 = _S385;
        _S382 = _S388;
        _S384 = _S387;
    }
    else
    {
        m_2 = 0.0f;
        _S383._S55 = 0.0f;
        _S383._S56 = 0.0f;
        _S380 = 0.0f;
        _S381 = 0.0f;
        _S382 = 0.0f;
        _S384._S55 = 0.0f;
        _S384._S56 = 0.0f;
    }
    var _S393 : f32 = _S372.value0_39.value0_38.z;
    var _S394 : Tuple_3 = s_apply_cos_0(_S393);
    var _S395 : s_bwdCallableCtx_cos_0;
    _S395._S19 = _S393;
    var _S396 : s_bwdCallableCtx_cos_0 = _S395;
    var _S397 : Tuple_2 = s_apply_sin_0(_S393);
    var _S398 : s_bwdCallableCtx_sin_0;
    _S398._S15 = _S393;
    var _S399 : s_bwdCallableCtx_sin_0 = _S398;
    var _S400 : vec2<f32> = vec2<f32>(_S394.value0_3, _S397.value0_2);
    var uv_star_prime_2 : vec2<f32> = vec2<f32>(m_2) * _S400;
    var _S401 : f32 = uv_star_prime_2.x;
    var _S402 : f32 = uv_star_prime_2.y;
    var uv_star_4 : vec2<f32> = vec2<f32>(-5.0375223159790039f * _S401 - 2.5048563480377197f * _S402, 4.7600293159484863f * _S401 + 2.8740129470825195f * _S402);
    var _S403 : vec2<f32> = (vec2<f32>(0) - vec2<f32>(1.4921735525131226f, 1.5248863697052002f));
    var _S404 : vec2<f32> = _S403 * uv_star_4;
    var _S405 : Tuple_1 = s_apply_abs_0(uv_star_4);
    var _S406 : s_bwdCallableCtx_abs_0;
    _S406._S10 = uv_star_4;
    var _S407 : s_bwdCallableCtx_abs_0 = _S406;
    var _S408 : vec2<f32> = _S405.value0_1 - vec2<f32>(1.3965622186660767f, 1.4513953924179077f);
    var uv_4 : vec2<f32> = _S404 / _S408;
    var _S409 : vec2<f32> = _S408 * _S408;
    const _S410 : vec3<f32> = vec3<f32>(0.16717147827148438f, -0.15095908939838409f, 0.94025474786758423f);
    const _S411 : vec3<f32> = vec3<f32>(0.14129979908466339f, -0.15518505871295929f, 1.0f);
    var xyd_2 : vec3<f32> = _S410 * vec3<f32>(uv_4.x) + _S411 * vec3<f32>(uv_4.y) + vec3<f32>(-0.0080153131857514381f, -0.0084331240504980087f, -0.025632597506046295f);
    var _S412 : vec2<f32> = xyd_2.xy;
    var _S413 : f32 = xyd_2.z;
    var _S414 : Tuple_11 = s_apply_away_from_flt_min_0(_S413);
    var _S415 : s_paramCtx_s_bwdCallableCtx_away_from_flt_min_0;
    _S415.value0_12 = _S413;
    var _S416 : s_bwdCallableCtx_away_from_flt_min_0 = s_bwdCallableCtx_away_from_flt_min_0( _S415 );
    var _S417 : f32 = _S414.value0_11 * _S414.value0_11;
    var _S418 : s_paramCtx_s_bwdCallableCtx_ucs_l_star_to_y_0;
    _S418.value0_35 = _S376.value0_4;
    var _S419 : s_bwdCallableCtx_ucs_l_star_to_y_0 = s_bwdCallableCtx_ucs_l_star_to_y_0( _S418 );
    var _S420 : f32 = 0.0f;
    s_bwdProp_ucs_l_star_to_y_0(_S419, &(_S420), _S374[i32(2)]);
    var _S421 : f32 = _S420;
    var _S422 : vec2<f32> = vec2<f32>(_S374[i32(0)], _S374[i32(1)]) / vec2<f32>(_S417);
    var _S423 : vec2<f32> = _S412 * (vec2<f32>(0) - _S422);
    var _S424 : f32 = _S423[i32(0)] + _S423[i32(1)];
    var _S425 : vec2<f32> = vec2<f32>(_S414.value0_11) * _S422;
    var _S426 : f32 = 0.0f;
    s_bwdProp_away_from_flt_min_0(_S416, &(_S426), _S424);
    var s_diff_xyd_T_0 : vec3<f32> = vec3<f32>(_S425[i32(0)], _S425[i32(1)], _S426);
    var _S427 : vec3<f32> = _S411 * s_diff_xyd_T_0;
    var _S428 : vec3<f32> = _S410 * s_diff_xyd_T_0;
    var _S429 : vec2<f32> = vec2<f32>(_S428[i32(0)] + _S428[i32(1)] + _S428[i32(2)], _S427[i32(0)] + _S427[i32(1)] + _S427[i32(2)]) / _S409;
    var _S430 : vec2<f32> = _S404 * (vec2<f32>(0) - _S429);
    var _S431 : vec2<f32> = _S408 * _S429;
    var _S432 : vec2<f32> = vec2<f32>(0.0f);
    s_bwdProp_abs_0(_S407, &(_S432), _S430);
    var _S433 : vec2<f32> = _S432 + _S403 * _S431;
    var s_diff_uv_star_prime_T_0 : vec2<f32> = vec2<f32>(4.7600293159484863f * _S433[i32(1)] + -5.0375223159790039f * _S433[i32(0)], 2.8740129470825195f * _S433[i32(1)] + 2.5048563480377197f * - _S433[i32(0)]);
    var _S434 : vec2<f32> = vec2<f32>(m_2) * s_diff_uv_star_prime_T_0;
    var _S435 : vec2<f32> = _S400 * s_diff_uv_star_prime_T_0;
    var _S436 : f32 = _S435[i32(0)] + _S435[i32(1)];
    var _S437 : f32 = 0.0f;
    s_bwdProp_sin_0(_S399, &(_S437), _S434[i32(1)]);
    var _S438 : f32 = 0.0f;
    s_bwdProp_cos_0(_S396, &(_S438), _S434[i32(0)]);
    var _S439 : vec3<f32> = vec3<f32>(0.0f, 0.0f, _S437 + _S438);
    var _S440 : vec3<f32>;
    if(_S379)
    {
        var _S441 : f32 = 0.0f;
        var _S442 : f32 = 0.0f;
        s_bwdProp_pow0_0(_S383, &(_S441), &(_S442), _S436);
        var _S443 : f32 = _S441 / _S380;
        var _S444 : f32 = _S382 * _S443;
        var _S445 : f32 = 15.93299388885498f * (_S381 * - _S443);
        var _S446 : f32 = 0.0f;
        var _S447 : f32 = 0.0f;
        s_bwdProp_pow0_0(_S384, &(_S446), &(_S447), _S445);
        var _S448 : vec3<f32> = _S439 + vec3<f32>(0.0f, _S372.value0_39.value1_8 * _S444, 0.0f);
        m_2 = _S446 + _S421;
        _S440 = _S448;
    }
    else
    {
        m_2 = _S421;
        _S440 = _S439;
    }
    var _S449 : f32 = 0.0f;
    var _S450 : f32 = 0.0f;
    var _S451 : f32 = 0.0f;
    s_bwdProp_clamp_0(_S378, &(_S449), &(_S450), &(_S451), m_2);
    (*_S373) = _S440 + vec3<f32>(_S372.value0_39.value1_8 * _S449, 0.0f, 0.0f);
    return;
}

fn s_fwd_ucs_jch_to_xyy_0( dpjch_1 : DiffPair_vectorx3Cfloatx2C3x3E_0,  l_white_3 : f32) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S452 : DiffPair_float_0 = DiffPair_float_0( dpjch_1.primal_0.x * l_white_3, dpjch_1.differential_0.x * l_white_3 );
    var _S453 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S454 : DiffPair_float_0 = DiffPair_float_0( 2.0988500118255615f, 0.0f );
    var _S455 : DiffPair_float_0 = _d_clamp_1(_S452, _S453, _S454);
    var m_3 : f32;
    var s_diff_m_0 : f32;
    if((_S455.primal_0) != 0.0f)
    {
        var _S456 : f32 = dpjch_1.primal_0.y * l_white_3;
        var _S457 : DiffPair_float_0 = DiffPair_float_0( _S455.primal_0, _S455.differential_0 );
        var _S458 : DiffPair_float_0 = DiffPair_float_0( 0.65239977836608887f, 0.0f );
        var _S459 : DiffPair_float_0 = pow0_forward_0(_S457, _S458);
        var _S460 : f32 = 15.93299388885498f * _S459.primal_0;
        var _S461 : DiffPair_float_0 = DiffPair_float_0( _S456 / _S460, (dpjch_1.differential_0.y * l_white_3 * _S460 - _S456 * (_S459.differential_0 * 15.93299388885498f)) / (_S460 * _S460) );
        var _S462 : DiffPair_float_0 = DiffPair_float_0( 0.83228504657745361f, 0.0f );
        var _S463 : DiffPair_float_0 = pow0_forward_0(_S461, _S462);
        m_3 = _S463.primal_0;
        s_diff_m_0 = _S463.differential_0;
    }
    else
    {
        m_3 = 0.0f;
        s_diff_m_0 = 0.0f;
    }
    var _S464 : DiffPair_float_0 = DiffPair_float_0( dpjch_1.primal_0.z, dpjch_1.differential_0.z );
    var _S465 : DiffPair_float_0 = _d_cos_1(_S464);
    var _S466 : DiffPair_float_0 = _d_sin_1(_S464);
    var _S467 : vec2<f32> = vec2<f32>(_S465.primal_0, _S466.primal_0);
    var uv_star_prime_3 : vec2<f32> = vec2<f32>(m_3) * _S467;
    var s_diff_uv_star_prime_0 : vec2<f32> = vec2<f32>(s_diff_m_0) * _S467 + vec2<f32>(_S465.differential_0, _S466.differential_0) * vec2<f32>(m_3);
    var _S468 : f32 = uv_star_prime_3.x;
    var _S469 : f32 = s_diff_uv_star_prime_0.x;
    var _S470 : f32 = uv_star_prime_3.y;
    var _S471 : f32 = s_diff_uv_star_prime_0.y;
    var uv_star_5 : vec2<f32> = vec2<f32>(-5.0375223159790039f * _S468 - 2.5048563480377197f * _S470, 4.7600293159484863f * _S468 + 2.8740129470825195f * _S470);
    var s_diff_uv_star_0 : vec2<f32> = vec2<f32>(_S469 * -5.0375223159790039f - _S471 * 2.5048563480377197f, _S469 * 4.7600293159484863f + _S471 * 2.8740129470825195f);
    var _S472 : vec2<f32> = (vec2<f32>(0) - vec2<f32>(1.4921735525131226f, 1.5248863697052002f));
    var _S473 : vec2<f32> = _S472 * uv_star_5;
    var _S474 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( uv_star_5, s_diff_uv_star_0 );
    var _S475 : DiffPair_vectorx3Cfloatx2C2x3E_0 = _d_abs_vector_1(_S474);
    var _S476 : vec2<f32> = _S475.primal_0 - vec2<f32>(1.3965622186660767f, 1.4513953924179077f);
    var uv_5 : vec2<f32> = _S473 / _S476;
    var s_diff_uv_0 : vec2<f32> = (((vec2<f32>(0) - vec2<f32>(0.0f, 0.0f)) * uv_star_5 + s_diff_uv_star_0 * _S472) * _S476 - _S473 * _S475.differential_0) / (_S476 * _S476);
    const _S477 : vec3<f32> = vec3<f32>(0.16717147827148438f, -0.15095908939838409f, 0.94025474786758423f);
    const _S478 : vec3<f32> = vec3<f32>(0.14129979908466339f, -0.15518505871295929f, 1.0f);
    var _S479 : vec3<f32> = vec3<f32>(s_diff_uv_0.x) * _S477 + vec3<f32>(s_diff_uv_0.y) * _S478;
    var xyd_3 : vec3<f32> = _S477 * vec3<f32>(uv_5.x) + _S478 * vec3<f32>(uv_5.y) + vec3<f32>(-0.0080153131857514381f, -0.0084331240504980087f, -0.025632597506046295f);
    var _S480 : vec2<f32> = xyd_3.xy;
    var _S481 : DiffPair_float_0 = DiffPair_float_0( xyd_3.z, _S479.z );
    var _S482 : DiffPair_float_0 = s_fwd_away_from_flt_min_0(_S481);
    var _S483 : vec2<f32> = vec2<f32>(_S482.primal_0);
    var xy_4 : vec2<f32> = _S480 / _S483;
    var s_diff_xy_0 : vec2<f32> = (_S479.xy * _S483 - _S480 * vec2<f32>(_S482.differential_0)) / vec2<f32>((_S482.primal_0 * _S482.primal_0));
    var _S484 : DiffPair_float_0 = DiffPair_float_0( _S455.primal_0, _S455.differential_0 );
    var _S485 : DiffPair_float_0 = s_fwd_ucs_l_star_to_y_0(_S484);
    var _S486 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(xy_4.x, xy_4.y, _S485.primal_0), vec3<f32>(s_diff_xy_0.x, s_diff_xy_0.y, _S485.differential_0) );
    return _S486;
}

fn xyy_to_xyz_0( xyy_0 : vec3<f32>) -> vec3<f32>
{
    var _S487 : f32 = xyy_0.y;
    if(_S487 == 0.0f)
    {
        return vec3<f32>(0.0f);
    }
    var _S488 : f32 = xyy_0.z;
    var _S489 : f32 = xyy_0.x;
    return vec3<f32>(_S488 * _S489 / _S487, _S488, _S488 * (1.0f - _S489 - _S487) / _S487);
}

struct Tuple_21
{
     value0_40 : vec3<f32>,
};

fn s_apply_xyy_to_xyz_0( _S490 : vec3<f32>) -> Tuple_21
{
    var _S491 : f32 = _S490.y;
    var _S492 : bool = _S491 == 0.0f;
    var _S493 : vec3<f32>;
    if(_S492)
    {
        _S493 = vec3<f32>(0.0f);
    }
    var _S494 : bool = !_S492;
    if(_S494)
    {
        var _S495 : f32 = _S490.z;
        var _S496 : f32 = _S490.x;
        _S493 = vec3<f32>(_S495 * _S496 / _S491, _S495, _S495 * (1.0f - _S496 - _S491) / _S491);
    }
    var _S497 : Tuple_21;
    _S497.value0_40 = _S493;
    return _S497;
}

struct s_paramCtx_s_bwdCallableCtx_xyy_to_xyz_0
{
     value0_41 : vec3<f32>,
};

struct s_bwdCallableCtx_xyy_to_xyz_0
{
     value0_42 : s_paramCtx_s_bwdCallableCtx_xyy_to_xyz_0,
};

fn s_bwdProp_xyy_to_xyz_0( _S498 : s_bwdCallableCtx_xyy_to_xyz_0,  _S499 : ptr<function, vec3<f32>>,  _S500 : vec3<f32>)
{
    var _S501 : f32 = _S498.value0_42.value0_41.y;
    var _S502 : bool = !(_S501 == 0.0f);
    var _S503 : f32;
    var _S504 : f32;
    var _S505 : f32;
    var _S506 : f32;
    var _S507 : f32;
    var _S508 : f32;
    if(_S502)
    {
        var _S509 : f32 = _S498.value0_42.value0_41.z;
        var _S510 : f32 = _S498.value0_42.value0_41.x;
        var _S511 : f32 = _S509 * _S510;
        var _S512 : f32 = 1.0f - _S510 - _S501;
        var _S513 : f32 = _S509 * _S512;
        _S503 = _S501 * _S501;
        _S504 = _S513;
        _S505 = _S509;
        _S506 = _S512;
        _S507 = _S511;
        _S508 = _S510;
    }
    else
    {
        _S503 = 0.0f;
        _S504 = 0.0f;
        _S505 = 0.0f;
        _S506 = 0.0f;
        _S507 = 0.0f;
        _S508 = 0.0f;
    }
    var _S514 : vec3<f32> = vec3<f32>(0.0f);
    var _S515 : vec3<f32>;
    if(_S502)
    {
        var _S516 : f32 = _S500[i32(2)] / _S503;
        var _S517 : f32 = _S501 * _S516;
        var _S518 : f32 = - (_S505 * _S517);
        var _S519 : f32 = _S500[i32(0)] / _S503;
        var _S520 : f32 = _S501 * _S519;
        var _S521 : vec3<f32> = vec3<f32>(_S518 + _S505 * _S520, 0.0f, _S500[i32(1)] + _S506 * _S517 + _S508 * _S520);
        _S503 = _S504 * - _S516 + _S518 + _S507 * - _S519;
        _S515 = _S521;
    }
    else
    {
        _S503 = 0.0f;
        _S515 = _S514;
    }
    (*_S499) = _S515 + vec3<f32>(0.0f, _S503, 0.0f);
    return;
}

fn s_fwd_xyy_to_xyz_0( dpxyy_0 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S522 : f32 = dpxyy_0.primal_0.y;
    var _S523 : f32 = dpxyy_0.differential_0.y;
    if(_S522 == 0.0f)
    {
        var _S524 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.0f), vec3<f32>(0.0f) );
        return _S524;
    }
    var _S525 : f32 = dpxyy_0.primal_0.z;
    var _S526 : f32 = dpxyy_0.differential_0.z;
    var _S527 : f32 = dpxyy_0.primal_0.x;
    var _S528 : f32 = dpxyy_0.differential_0.x;
    var _S529 : f32 = _S525 * _S527;
    var _S530 : f32 = _S522 * _S522;
    var _S531 : f32 = 1.0f - _S527 - _S522;
    var _S532 : f32 = _S525 * _S531;
    var _S533 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S529 / _S522, _S525, _S532 / _S522), vec3<f32>(((_S526 * _S527 + _S528 * _S525) * _S522 - _S529 * _S523) / _S530, _S526, ((_S526 * _S531 + (0.0f - _S528 - _S523) * _S525) * _S522 - _S532 * _S523) / _S530) );
    return _S533;
}

struct ColorEqualParams_0
{
     size_0 : vec4<u32>,
     mode_0 : vec4<u32>,
     deriche_a_0 : vec4<f32>,
     deriche_b_0 : vec4<f32>,
     shifts_0 : vec4<f32>,
     white_0 : vec4<f32>,
     slots_0 : vec4<f32>,
     rgb_to_xyz_r0_0 : vec4<f32>,
     rgb_to_xyz_r1_0 : vec4<f32>,
     rgb_to_xyz_r2_0 : vec4<f32>,
     xyz_to_rgb_r0_0 : vec4<f32>,
     xyz_to_rgb_r1_0 : vec4<f32>,
     xyz_to_rgb_r2_0 : vec4<f32>,
};

fn xyz_to_rgb_of_0( p_0 : ColorEqualParams_0) -> mat3x3<f32>
{
    return mat3x3<f32>(p_0.xyz_to_rgb_r0_0.xyz, p_0.xyz_to_rgb_r1_0.xyz, p_0.xyz_to_rgb_r2_0.xyz);
}

struct s_paramCtx_s_bwdCallableCtx_colorequal_0
{
     value0_43 : f32,
     value1_9 : vec2<f32>,
     value2_3 : vec3<f32>,
     value3_1 : ColorEqualParams_0,
     value4_1 : bool,
     value5_1 : Slots_0,
};

struct s_minCtx_s_bwdCallableCtx_colorequal_0
{
     value0_44 : f32,
     value1_10 : s_minCtx_s_bwdCallableCtx_curve_0,
     value2_4 : f32,
     value3_2 : s_minCtx_s_bwdCallableCtx_curve_0,
     value4_2 : f32,
     value5_2 : s_minCtx_s_bwdCallableCtx_curve_0,
     value6_0 : f32,
     value7_0 : s_minCtx_s_bwdCallableCtx_lookup_0,
};

struct s_bwdCallableCtx_colorequal_0
{
     value0_45 : s_paramCtx_s_bwdCallableCtx_colorequal_0,
     value1_11 : s_minCtx_s_bwdCallableCtx_colorequal_0,
};

fn s_bwdProp_colorequal_0( _S534 : s_bwdCallableCtx_colorequal_0,  _S535 : ptr<function, Slots_0>,  _S536 : vec3<f32>)
{
    var _S537 : s_paramCtx_s_bwdCallableCtx_y_to_ucs_l_star_0 = s_paramCtx_s_bwdCallableCtx_y_to_ucs_l_star_0( 0.0f );
    var _S538 : f32 = _S534.value0_45.value3_1.white_0.x;
    var white_1 : f32;
    var _S539 : s_bwdCallableCtx_y_to_ucs_l_star_0;
    var _S540 : s_bwdCallableCtx_exp2_0;
    if(_S534.value0_45.value4_1)
    {
        var _S541 : Tuple_10 = s_apply_exp2_0(_S534.value0_45.value5_1.white_level_0);
        var _S542 : s_bwdCallableCtx_exp2_0;
        _S542._S68 = _S534.value0_45.value5_1.white_level_0;
        var _S543 : Tuple_12 = s_apply_y_to_ucs_l_star_0(_S541.value0_10);
        var _S544 : s_paramCtx_s_bwdCallableCtx_y_to_ucs_l_star_0;
        _S544.value0_15 = _S541.value0_10;
        white_1 = _S543.value0_14;
        _S539.value0_16 = _S544;
        _S540 = _S542;
    }
    else
    {
        white_1 = _S538;
        _S539.value0_16 = _S537;
        _S540._S68 = 0.0f;
    }
    var _S545 : Tuple_13 = s_apply_luv_to_jch_0(_S534.value0_45.value0_43, white_1, _S534.value0_45.value1_9);
    var _S546 : s_paramCtx_s_bwdCallableCtx_luv_to_jch_0;
    _S546.value0_18 = _S534.value0_45.value0_43;
    _S546.value1_0 = white_1;
    _S546.value2_0 = _S534.value0_45.value1_9;
    var _S547 : s_bwdCallableCtx_luv_to_jch_0 = s_bwdCallableCtx_luv_to_jch_0( _S546 );
    var _S548 : Tuple_14 = s_apply_ucs_jch_to_hsb_0(_S545.value0_17);
    var _S549 : s_paramCtx_s_bwdCallableCtx_ucs_jch_to_hsb_0;
    _S549.value0_21 = _S545.value0_17;
    var _S550 : s_bwdCallableCtx_ucs_jch_to_hsb_0 = s_bwdCallableCtx_ucs_jch_to_hsb_0( _S549 );
    var h_0 : f32 = _S548.value0_20.x;
    var _S551 : f32 = _S534.value0_45.value3_1.slots_0.x;
    var _S552 : s_paramCtx_s_bwdCallableCtx_curve_0;
    _S552.value3_0 = _S534.value0_45.value5_1.hue_red_0;
    _S552.value4_0 = _S551;
    _S552.value5_0 = _S534.value0_45.value4_1;
    var _S553 : s_bwdCallableCtx_curve_0 = s_bwdCallableCtx_curve_0( _S552, _S534.value1_11.value1_10 );
    var _S554 : f32 = _S534.value0_45.value3_1.slots_0.y;
    var _S555 : s_paramCtx_s_bwdCallableCtx_curve_0;
    _S555.value3_0 = _S534.value0_45.value5_1.saturation_red_0;
    _S555.value4_0 = _S554;
    _S555.value5_0 = _S534.value0_45.value4_1;
    var _S556 : s_bwdCallableCtx_curve_0 = s_bwdCallableCtx_curve_0( _S555, _S534.value1_11.value3_2 );
    var _S557 : f32 = _S534.value0_45.value3_1.slots_0.z;
    var _S558 : s_paramCtx_s_bwdCallableCtx_curve_0;
    _S558.value3_0 = _S534.value0_45.value5_1.brightness_red_0;
    _S558.value4_0 = _S557;
    _S558.value5_0 = _S534.value0_45.value4_1;
    var _S559 : s_bwdCallableCtx_curve_0 = s_bwdCallableCtx_curve_0( _S558, _S534.value1_11.value5_2 );
    var guided_mode_0 : bool = (_S534.value0_45.value3_1.mode_0.z) != u32(0);
    var chromatic_0 : bool = ((_S545.value0_17.y) > 1.52587890625e-05f);
    var hue_correction_0 : f32;
    if(guided_mode_0)
    {
        hue_correction_0 = _S534.value0_45.value2_3.x;
    }
    else
    {
        if(chromatic_0)
        {
            hue_correction_0 = _S534.value1_11.value0_44;
        }
        else
        {
            hue_correction_0 = 0.0f;
        }
    }
    var saturation_correction_0 : f32;
    if(guided_mode_0)
    {
        saturation_correction_0 = _S534.value0_45.value2_3.y;
    }
    else
    {
        if(chromatic_0)
        {
            saturation_correction_0 = _S534.value1_11.value2_4;
        }
        else
        {
            saturation_correction_0 = 1.0f;
        }
    }
    var brightness_correction_0 : f32;
    var _S560 : f32;
    var _S561 : f32;
    if(guided_mode_0)
    {
        brightness_correction_0 = _S534.value0_45.value2_3.z;
        _S560 = 0.0f;
        _S561 = 0.0f;
    }
    else
    {
        if(chromatic_0)
        {
            var _S562 : f32 = _S548.value0_20.y;
            var _S563 : f32 = _S534.value1_11.value4_2 - 1.0f;
            brightness_correction_0 = _S562 * _S563;
            _S560 = _S562;
            _S561 = _S563;
        }
        else
        {
            brightness_correction_0 = 0.0f;
            _S560 = 0.0f;
            _S561 = 0.0f;
        }
    }
    var hue_2 : f32 = h_0 + hue_correction_0;
    var _S564 : f32 = _S548.value0_20.y;
    var _S565 : f32 = 1.0f + 2.0f * (saturation_correction_0 - 1.0f);
    var _S566 : f32 = _S564 * _S565;
    var _S567 : Tuple_9 = s_apply_max_0(0.0f, _S566);
    var _S568 : s_bwdCallableCtx_max_0;
    _S568._S60 = 0.0f;
    _S568._S61 = _S566;
    var _S569 : s_bwdCallableCtx_max_0 = _S568;
    var _S570 : f32 = _S548.value0_20.z;
    var _S571 : f32 = 1.0f + 8.0f * brightness_correction_0;
    var _S572 : f32 = _S570 * _S571;
    var _S573 : Tuple_9 = s_apply_max_0(0.0f, _S572);
    var _S574 : s_bwdCallableCtx_max_0;
    _S574._S60 = 0.0f;
    _S574._S61 = _S572;
    var _S575 : s_bwdCallableCtx_max_0 = _S574;
    var _S576 : vec3<f32> = vec3<f32>(hue_2, _S567.value0_9, _S573.value0_9);
    var _S577 : Tuple_17 = s_apply_ucs_hsb_to_jch_0(_S576);
    var _S578 : s_paramCtx_s_bwdCallableCtx_ucs_hsb_to_jch_0;
    _S578.value0_29 = _S576;
    var _S579 : s_bwdCallableCtx_ucs_hsb_to_jch_0 = s_bwdCallableCtx_ucs_hsb_to_jch_0( _S578 );
    var _S580 : f32 = _S577.value0_28.z;
    var _S581 : s_bwdCallableCtx_lookup_0 = s_bwdCallableCtx_lookup_0( _S534.value1_11.value7_0 );
    var _S582 : f32 = _S577.value0_28.x;
    var _S583 : f32 = _S582 * white_1;
    var _S584 : Tuple_8 = s_apply_pow0_0(_S583, 0.65239977836608887f);
    var _S585 : s_bwdCallableCtx_pow0_0;
    _S585._S55 = _S583;
    _S585._S56 = 0.65239977836608887f;
    var _S586 : s_bwdCallableCtx_pow0_0 = _S585;
    var _S587 : f32 = 15.93299388885498f * _S584.value0_8;
    var _S588 : Tuple_8 = s_apply_pow0_0(_S534.value1_11.value6_0, 0.60075569152832031f);
    var _S589 : s_bwdCallableCtx_pow0_0;
    _S589._S55 = _S534.value1_11.value6_0;
    _S589._S56 = 0.60075569152832031f;
    var _S590 : s_bwdCallableCtx_pow0_0 = _S589;
    var _S591 : f32 = _S587 * _S588.value0_8;
    var _S592 : f32 = white_1 * white_1;
    var _S593 : vec3<f32> = vec3<f32>(_S582, _S591 / white_1, _S580);
    var _S594 : Tuple_14 = s_apply_ucs_jch_to_hsb_0(_S593);
    var _S595 : s_paramCtx_s_bwdCallableCtx_ucs_jch_to_hsb_0;
    _S595.value0_21 = _S593;
    var _S596 : s_bwdCallableCtx_ucs_jch_to_hsb_0 = s_bwdCallableCtx_ucs_jch_to_hsb_0( _S595 );
    var _S597 : f32 = _S594.value0_20.y;
    var _S598 : f32 = 0.80000001192092896f * _S597;
    var _S599 : Tuple_18 = s_apply_soft_clip_0(_S567.value0_9, _S598, _S597);
    var _S600 : s_paramCtx_s_bwdCallableCtx_soft_clip_0;
    _S600.value0_32 = _S567.value0_9;
    _S600.value1_7 = _S598;
    _S600.value2_2 = _S597;
    var _S601 : s_bwdCallableCtx_soft_clip_0 = s_bwdCallableCtx_soft_clip_0( _S600 );
    var _S602 : vec3<f32> = vec3<f32>(hue_2, _S599.value0_31, _S573.value0_9);
    var _S603 : Tuple_17 = s_apply_ucs_hsb_to_jch_0(_S602);
    var _S604 : s_paramCtx_s_bwdCallableCtx_ucs_hsb_to_jch_0;
    _S604.value0_29 = _S602;
    var _S605 : s_bwdCallableCtx_ucs_hsb_to_jch_0 = s_bwdCallableCtx_ucs_hsb_to_jch_0( _S604 );
    var _S606 : f32 = _S603.value0_28.x;
    var _S607 : f32 = _S603.value0_28.y;
    var _S608 : vec3<f32> = vec3<f32>(_S606 * white_1, _S607 * white_1, _S603.value0_28.z);
    var _S609 : Tuple_20 = s_apply_ucs_jch_to_xyy_0(_S608, 1.0f);
    var _S610 : s_paramCtx_s_bwdCallableCtx_ucs_jch_to_xyy_0;
    _S610.value0_38 = _S608;
    _S610.value1_8 = 1.0f;
    var _S611 : s_bwdCallableCtx_ucs_jch_to_xyy_0 = s_bwdCallableCtx_ucs_jch_to_xyy_0( _S610 );
    var _S612 : Tuple_21 = s_apply_xyy_to_xyz_0(_S609.value0_37);
    var _S613 : s_paramCtx_s_bwdCallableCtx_xyy_to_xyz_0;
    _S613.value0_41 = _S609.value0_37;
    var _S614 : s_bwdCallableCtx_xyy_to_xyz_0 = s_bwdCallableCtx_xyy_to_xyz_0( _S613 );
    var _S615 : s_bwdCallableCtx_mul_0;
    _S615._S47 = xyz_to_rgb_of_0(_S534.value0_45.value3_1);
    _S615._S48 = _S612.value0_40;
    var _S616 : s_bwdCallableCtx_mul_0 = _S615;
    var _S617 : Slots_0 = Slots_x24_syn_dzero_0();
    var _S618 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S619 : vec3<f32> = vec3<f32>(0.0f);
    var _S620 : vec3<f32> = _S619;
    s_bwdProp_mul_0(_S616, &(_S618), &(_S620), _S536);
    var _S621 : vec3<f32> = _S619;
    s_bwdProp_xyy_to_xyz_0(_S614, &(_S621), _S620);
    var _S622 : vec3<f32> = _S619;
    s_bwdProp_ucs_jch_to_xyy_0(_S611, &(_S622), _S621);
    var _S623 : f32 = _S607 * _S622[i32(1)];
    var _S624 : f32 = _S606 * _S622[i32(0)];
    var _S625 : vec3<f32> = vec3<f32>(white_1 * _S622[i32(0)], white_1 * _S622[i32(1)], _S622[i32(2)]);
    var _S626 : vec3<f32> = _S619;
    s_bwdProp_ucs_hsb_to_jch_0(_S605, &(_S626), _S625);
    var _S627 : f32 = 0.0f;
    var _S628 : f32 = 0.0f;
    var _S629 : f32 = 0.0f;
    s_bwdProp_soft_clip_0(_S601, &(_S627), &(_S628), &(_S629), _S626[i32(1)]);
    var _S630 : vec3<f32> = vec3<f32>(0.0f, _S629 + 0.80000001192092896f * _S628, 0.0f);
    var _S631 : vec3<f32> = _S619;
    s_bwdProp_ucs_jch_to_hsb_0(_S596, &(_S631), _S630);
    var _S632 : f32 = _S631[i32(1)] / _S592;
    var _S633 : f32 = _S591 * - _S632;
    var _S634 : f32 = white_1 * _S632;
    var _S635 : f32 = _S587 * _S634;
    var _S636 : f32 = _S588.value0_8 * _S634;
    var _S637 : f32 = 0.0f;
    var _S638 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S590, &(_S637), &(_S638), _S635);
    var _S639 : f32 = 15.93299388885498f * _S636;
    var _S640 : f32 = 0.0f;
    var _S641 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S586, &(_S640), &(_S641), _S639);
    var _S642 : f32 = _S582 * _S640;
    var _S643 : f32 = _S631[i32(0)] + white_1 * _S640;
    var _S644 : f32 = 0.0f;
    s_bwdProp_lookup_0(_S581, &(_S644), _S637);
    var _S645 : vec3<f32> = vec3<f32>(_S643, 0.0f, _S631[i32(2)] + _S644);
    var _S646 : vec3<f32> = _S619;
    s_bwdProp_ucs_hsb_to_jch_0(_S579, &(_S646), _S645);
    var _S647 : f32 = _S626[i32(2)] + _S646[i32(2)];
    var _S648 : f32 = 0.0f;
    var _S649 : f32 = 0.0f;
    s_bwdProp_max_0(_S575, &(_S648), &(_S649), _S647);
    var _S650 : f32 = _S571 * _S649;
    var _S651 : f32 = 8.0f * (_S570 * _S649);
    var _S652 : f32 = _S627 + _S646[i32(1)];
    var _S653 : f32 = 0.0f;
    var _S654 : f32 = 0.0f;
    s_bwdProp_max_0(_S569, &(_S653), &(_S654), _S652);
    var _S655 : f32 = 2.0f * (_S564 * _S654);
    var _S656 : f32 = _S626[i32(0)] + _S646[i32(0)];
    var _S657 : f32 = _S623 + _S624 + _S633 + _S642;
    var _S658 : vec3<f32> = vec3<f32>(0.0f, _S565 * _S654, _S650);
    var _S659 : vec3<f32>;
    if(guided_mode_0)
    {
        white_1 = 0.0f;
        _S659 = _S658;
    }
    else
    {
        if(chromatic_0)
        {
            var _S660 : vec3<f32> = _S658 + vec3<f32>(0.0f, _S561 * _S651, 0.0f);
            white_1 = _S560 * _S651;
            _S659 = _S660;
        }
        else
        {
            white_1 = 0.0f;
            _S659 = _S658;
        }
    }
    if(guided_mode_0)
    {
        hue_correction_0 = 0.0f;
    }
    else
    {
        if(chromatic_0)
        {
            hue_correction_0 = _S655;
        }
        else
        {
            hue_correction_0 = 0.0f;
        }
    }
    if(guided_mode_0)
    {
        saturation_correction_0 = 0.0f;
    }
    else
    {
        if(chromatic_0)
        {
            saturation_correction_0 = _S656;
        }
        else
        {
            saturation_correction_0 = 0.0f;
        }
    }
    var _S661 : f32 = 0.0f;
    var _S662 : f32 = 0.0f;
    var _S663 : f32 = 0.0f;
    s_bwdProp_curve_0(_S559, &(_S661), &(_S662), &(_S663), white_1);
    var _S664 : f32 = 0.0f;
    var _S665 : f32 = 0.0f;
    var _S666 : f32 = 0.0f;
    s_bwdProp_curve_0(_S556, &(_S664), &(_S665), &(_S666), hue_correction_0);
    var _S667 : f32 = 0.0f;
    var _S668 : f32 = 0.0f;
    var _S669 : f32 = 0.0f;
    s_bwdProp_curve_0(_S553, &(_S667), &(_S668), &(_S669), saturation_correction_0);
    var _S670 : vec3<f32> = _S659 + vec3<f32>(_S661 + _S664 + _S667 + _S656, 0.0f, 0.0f);
    var _S671 : vec3<f32> = _S619;
    s_bwdProp_ucs_jch_to_hsb_0(_S550, &(_S671), _S670);
    var _S672 : f32 = 0.0f;
    var _S673 : f32 = 0.0f;
    var _S674 : vec2<f32> = vec2<f32>(0.0f);
    s_bwdProp_luv_to_jch_0(_S547, &(_S672), &(_S673), &(_S674), _S671);
    var _S675 : f32 = _S673 + _S657;
    var _S676 : Slots_0 = _S617;
    _S676.brightness_red_0 = _S662;
    _S676.saturation_red_0 = _S665;
    _S676.hue_red_0 = _S668;
    var _S677 : Slots_0 = Slots_x24_syn_dadd_0(_S617, _S676);
    var _S678 : Slots_0;
    if(_S534.value0_45.value4_1)
    {
        var _S679 : f32 = 0.0f;
        s_bwdProp_y_to_ucs_l_star_0(_S539, &(_S679), _S675);
        var _S680 : f32 = 0.0f;
        s_bwdProp_exp2_0(_S540, &(_S680), _S679);
        var _S681 : Slots_0 = _S617;
        _S681.white_level_0 = _S680;
        _S678 = Slots_x24_syn_dadd_0(_S677, _S681);
    }
    else
    {
        _S678 = _S677;
    }
    (*_S535) = _S678;
    return;
}

fn slots_of_0() -> Slots_0
{
    var s_3 : Slots_0;
    s_3.hue_red_0 = params_0.ce_0.slots_0.x;
    s_3.saturation_red_0 = params_0.ce_0.slots_0.y;
    s_3.brightness_red_0 = params_0.ce_0.slots_0.z;
    s_3.white_level_0 = params_0.ce_0.slots_0.w;
    return s_3;
}

fn place_0( _S682 : vec3<u32>) -> u32
{
    return (params_0.pass_0.origin_y_0 + _S682.y) * params_0.ce_0.size_0.x + params_0.pass_0.origin_x_0 + _S682.x;
}

struct Pixel_0
{
     l_star_3 : f32,
     uv_star_6 : vec2<f32>,
     guided_0 : vec3<f32>,
};

struct Sample_0
{
     uv_6 : vec2<f32>,
     l_star_4 : f32,
     saturation_1 : f32,
};

fn rgb_to_xyz_of_0() -> mat3x3<f32>
{
    return mat3x3<f32>(params_0.ce_0.rgb_to_xyz_r0_0.xyz, params_0.ce_0.rgb_to_xyz_r1_0.xyz, params_0.ce_0.rgb_to_xyz_r2_0.xyz);
}

fn sample_of_0( _S683 : vec3<f32>) -> Sample_0
{
    var rgb_0 : vec3<f32> = clamp(_S683, vec3<f32>(-1.00000001504746622e+30f), vec3<f32>(1.00000001504746622e+30f));
    var _S684 : f32 = rgb_0.x;
    var _S685 : f32 = rgb_0.y;
    var _S686 : f32 = rgb_0.z;
    var _S687 : f32 = max(max(_S684, _S685), _S686);
    var delta_0 : f32 = _S687 - min(min(_S684, _S685), _S686);
    var s_4 : Sample_0;
    var _S688 : bool;
    if((_S687 > 1.52587890625e-05f))
    {
        _S688 = (delta_0 > 1.52587890625e-05f);
    }
    else
    {
        _S688 = false;
    }
    var _S689 : f32;
    if(_S688)
    {
        _S689 = delta_0 / _S687;
    }
    else
    {
        _S689 = 0.0f;
    }
    s_4.saturation_1 = _S689;
    var xyy_1 : vec3<f32> = d65_xyz_to_xyy_0((((rgb_0) * (rgb_to_xyz_of_0()))));
    s_4.uv_6 = xy_to_ucs_uv_0(xyy_1.xy);
    s_4.l_star_4 = y_to_ucs_l_star_0(xyy_1.z);
    return s_4;
}

fn pixel_of_0( _S690 : vec3<f32>,  _S691 : u32) -> Pixel_0
{
    var _S692 : Sample_0 = sample_of_0(_S690);
    var px_0 : Pixel_0;
    px_0.l_star_3 = _S692.l_star_4;
    px_0.uv_star_6 = _S692.uv_6;
    px_0.guided_0 = vec3<f32>(0.0f, 1.0f, 0.0f);
    if((params_0.ce_0.mode_0.z) != u32(0))
    {
        px_0.uv_star_6 = uv_0[_S691];
        px_0.guided_0 = vec3<f32>(corrections_0[_S691].x, corrections_0[_S691].y, brightness_0[_S691]);
    }
    return px_0;
}

fn xyz_to_rgb_of_1() -> mat3x3<f32>
{
    return mat3x3<f32>(params_0.ce_0.xyz_to_rgb_r0_0.xyz, params_0.ce_0.xyz_to_rgb_r1_0.xyz, params_0.ce_0.xyz_to_rgb_r2_0.xyz);
}

fn colorequal_0( _S693 : f32,  _S694 : vec2<f32>,  _S695 : vec3<f32>,  _S696 : bool,  _S697 : Slots_0) -> vec3<f32>
{
    var _S698 : f32 = params_0.ce_0.white_0.x;
    var white_2 : f32;
    if(_S696)
    {
        white_2 = y_to_ucs_l_star_0(exp2(_S697.white_level_0));
    }
    else
    {
        white_2 = _S698;
    }
    var jch_2 : vec3<f32> = luv_to_jch_0(_S693, white_2, _S694);
    var hsb_1 : vec3<f32> = ucs_jch_to_hsb_0(jch_2);
    var h_1 : f32 = hsb_1.x;
    var hue_curve_0 : f32 = curve_0(u32(0), u32(2048), h_1, _S697.hue_red_0, params_0.ce_0.slots_0.x, _S696);
    var saturation_curve_0 : f32 = curve_0(u32(512), u32(2560), h_1, _S697.saturation_red_0, params_0.ce_0.slots_0.y, _S696);
    var brightness_curve_0 : f32 = curve_0(u32(1024), u32(3072), h_1, _S697.brightness_red_0, params_0.ce_0.slots_0.z, _S696);
    var guided_mode_1 : bool = (params_0.ce_0.mode_0.z) != u32(0);
    var chromatic_1 : bool = ((jch_2.y) > 1.52587890625e-05f);
    var hue_correction_1 : f32;
    if(guided_mode_1)
    {
        hue_correction_1 = _S695.x;
    }
    else
    {
        if(chromatic_1)
        {
            hue_correction_1 = hue_curve_0;
        }
        else
        {
            hue_correction_1 = 0.0f;
        }
    }
    var saturation_correction_1 : f32;
    if(guided_mode_1)
    {
        saturation_correction_1 = _S695.y;
    }
    else
    {
        if(chromatic_1)
        {
            saturation_correction_1 = saturation_curve_0;
        }
        else
        {
            saturation_correction_1 = 1.0f;
        }
    }
    var brightness_correction_1 : f32;
    if(guided_mode_1)
    {
        brightness_correction_1 = _S695.z;
    }
    else
    {
        if(chromatic_1)
        {
            brightness_correction_1 = hsb_1.y * (brightness_curve_0 - 1.0f);
        }
        else
        {
            brightness_correction_1 = 0.0f;
        }
    }
    var hue_3 : f32 = h_1 + hue_correction_1;
    var _S699 : f32 = max(0.0f, hsb_1.y * (1.0f + 2.0f * (saturation_correction_1 - 1.0f)));
    var _S700 : f32 = max(0.0f, hsb_1.z * (1.0f + 8.0f * brightness_correction_1));
    var mapped_0 : vec3<f32> = ucs_hsb_to_jch_0(vec3<f32>(hue_3, _S699, _S700));
    var _S701 : f32 = mapped_0.z;
    var _S702 : f32 = mapped_0.x;
    var _S703 : f32 = ucs_jch_to_hsb_0(vec3<f32>(_S702, 15.93299388885498f * pow0_0(_S702 * white_2, 0.65239977836608887f) * pow0_0(lookup_0(u32(1536), _S701), 0.60075569152832031f) / white_2, _S701)).y;
    var jch_out_0 : vec3<f32> = ucs_hsb_to_jch_0(vec3<f32>(hue_3, soft_clip_0(_S699, 0.80000001192092896f * _S703, _S703), _S700));
    return (((xyy_to_xyz_0(ucs_jch_to_xyy_0(vec3<f32>(jch_out_0.x * white_2, jch_out_0.y * white_2, jch_out_0.z), 1.0f))) * (xyz_to_rgb_of_1())));
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_0 : vec3<u32>)
{
    var _S704 : u32 = id_0.x;
    var _S705 : bool;
    if((_S704 >= (params_0.pass_0.width_0)))
    {
        _S705 = true;
    }
    else
    {
        _S705 = ((id_0.y) >= (params_0.pass_0.height_0));
    }
    if(_S705)
    {
        return;
    }
    var i_3 : u32 = id_0.y * params_0.pass_0.width_0 + _S704;
    var pixel_0 : vec4<f32> = input_0[i_3];
    var _S706 : Slots_0 = slots_of_0();
    var _S707 : u32 = place_0(id_0);
    var rgb_1 : vec3<f32> = pixel_0.xyz;
    var r_0 : u32 = u32(0);
    loop
    {
        if((r_0 < (params_0.pass_0.repeat_0)))
        {
        }
        else
        {
            break;
        }
        var _S708 : Pixel_0 = pixel_of_0(rgb_1, _S707);
        var _S709 : vec3<f32> = colorequal_0(_S708.l_star_3, _S708.uv_star_6, _S708.guided_0, false, _S706);
        var _S710 : u32 = r_0 + u32(1);
        rgb_1 = _S709;
        r_0 = _S710;
    }
    output_0[i_3] = vec4<f32>(rgb_1, pixel_0.w);
    return;
}

struct DiffPair_Slots_0
{
     primal_0 : Slots_0,
     differential_0 : Slots_0,
};

struct Tuple_22
{
     value0_46 : vec3<f32>,
     value1_12 : s_minCtx_s_bwdCallableCtx_colorequal_0,
};

fn s_apply_colorequal_0( _S711 : f32,  _S712 : vec2<f32>,  _S713 : vec3<f32>,  _S714 : bool,  _S715 : Slots_0) -> Tuple_22
{
    var _S716 : s_minCtx_s_bwdCallableCtx_lookup_0 = s_minCtx_s_bwdCallableCtx_lookup_0( 0.0f, 0.0f );
    var _S717 : s_minCtx_s_bwdCallableCtx_curve_0 = s_minCtx_s_bwdCallableCtx_curve_0( _S716, 0.0f, _S716 );
    var _S718 : s_minCtx_s_bwdCallableCtx_colorequal_0;
    _S718.value0_44 = 0.0f;
    _S718.value1_10 = _S717;
    _S718.value2_4 = 0.0f;
    _S718.value3_2 = _S717;
    _S718.value4_2 = 0.0f;
    _S718.value5_2 = _S717;
    _S718.value6_0 = 0.0f;
    _S718.value7_0 = _S716;
    var _S719 : f32 = params_0.ce_0.white_0.x;
    var white_3 : f32;
    if(_S714)
    {
        white_3 = s_apply_y_to_ucs_l_star_0(s_apply_exp2_0(_S715.white_level_0).value0_10).value0_14;
    }
    else
    {
        white_3 = _S719;
    }
    var _S720 : Tuple_13 = s_apply_luv_to_jch_0(_S711, white_3, _S712);
    var _S721 : Tuple_14 = s_apply_ucs_jch_to_hsb_0(_S720.value0_17);
    var h_2 : f32 = _S721.value0_20.x;
    var _S722 : Tuple_16 = s_apply_curve_0(u32(0), u32(2048), h_2, _S715.hue_red_0, params_0.ce_0.slots_0.x, _S714);
    _S718.value0_44 = _S722.value0_26;
    _S718.value1_10 = _S722.value1_5;
    var _S723 : Tuple_16 = s_apply_curve_0(u32(512), u32(2560), h_2, _S715.saturation_red_0, params_0.ce_0.slots_0.y, _S714);
    _S718.value2_4 = _S723.value0_26;
    _S718.value3_2 = _S723.value1_5;
    var _S724 : Tuple_16 = s_apply_curve_0(u32(1024), u32(3072), h_2, _S715.brightness_red_0, params_0.ce_0.slots_0.z, _S714);
    _S718.value4_2 = _S724.value0_26;
    _S718.value5_2 = _S724.value1_5;
    var guided_mode_2 : bool = (params_0.ce_0.mode_0.z) != u32(0);
    var chromatic_2 : bool = ((_S720.value0_17.y) > 1.52587890625e-05f);
    var hue_correction_2 : f32;
    if(guided_mode_2)
    {
        hue_correction_2 = _S713.x;
    }
    else
    {
        if(chromatic_2)
        {
            hue_correction_2 = _S722.value0_26;
        }
        else
        {
            hue_correction_2 = 0.0f;
        }
    }
    var saturation_correction_2 : f32;
    if(guided_mode_2)
    {
        saturation_correction_2 = _S713.y;
    }
    else
    {
        if(chromatic_2)
        {
            saturation_correction_2 = _S723.value0_26;
        }
        else
        {
            saturation_correction_2 = 1.0f;
        }
    }
    var brightness_correction_2 : f32;
    if(guided_mode_2)
    {
        brightness_correction_2 = _S713.z;
    }
    else
    {
        if(chromatic_2)
        {
            brightness_correction_2 = _S721.value0_20.y * (_S724.value0_26 - 1.0f);
        }
        else
        {
            brightness_correction_2 = 0.0f;
        }
    }
    var hue_4 : f32 = h_2 + hue_correction_2;
    var _S725 : Tuple_9 = s_apply_max_0(0.0f, _S721.value0_20.y * (1.0f + 2.0f * (saturation_correction_2 - 1.0f)));
    var _S726 : Tuple_9 = s_apply_max_0(0.0f, _S721.value0_20.z * (1.0f + 8.0f * brightness_correction_2));
    var _S727 : Tuple_17 = s_apply_ucs_hsb_to_jch_0(vec3<f32>(hue_4, _S725.value0_9, _S726.value0_9));
    var _S728 : f32 = _S727.value0_28.z;
    var _S729 : Tuple_15 = s_apply_lookup_0(u32(1536), _S728);
    _S718.value6_0 = _S729.value0_24;
    _S718.value7_0 = _S729.value1_2;
    var _S730 : f32 = _S727.value0_28.x;
    var _S731 : f32 = s_apply_ucs_jch_to_hsb_0(vec3<f32>(_S730, 15.93299388885498f * s_apply_pow0_0(_S730 * white_3, 0.65239977836608887f).value0_8 * s_apply_pow0_0(_S729.value0_24, 0.60075569152832031f).value0_8 / white_3, _S728)).value0_20.y;
    var _S732 : Tuple_17 = s_apply_ucs_hsb_to_jch_0(vec3<f32>(hue_4, s_apply_soft_clip_0(_S725.value0_9, 0.80000001192092896f * _S731, _S731).value0_31, _S726.value0_9));
    var _S733 : Tuple_22;
    _S733.value0_46 = s_apply_mul_0(xyz_to_rgb_of_1(), s_apply_xyy_to_xyz_0(s_apply_ucs_jch_to_xyy_0(vec3<f32>(_S732.value0_28.x * white_3, _S732.value0_28.y * white_3, _S732.value0_28.z), 1.0f).value0_37).value0_40).value0_7;
    _S733.value1_12 = _S718;
    return _S733;
}

fn _S734( _S735 : f32,  _S736 : vec2<f32>,  _S737 : vec3<f32>,  _S738 : bool,  _S739 : ptr<function, DiffPair_Slots_0>,  _S740 : vec3<f32>)
{
    var _S741 : Tuple_22 = s_apply_colorequal_0(_S735, _S736, _S737, _S738, (*_S739).primal_0);
    var _S742 : s_paramCtx_s_bwdCallableCtx_colorequal_0;
    _S742.value0_43 = _S735;
    _S742.value1_9 = _S736;
    _S742.value2_3 = _S737;
    _S742.value3_1.size_0 = params_0.ce_0.size_0;
    _S742.value3_1.mode_0 = params_0.ce_0.mode_0;
    _S742.value3_1.deriche_a_0 = params_0.ce_0.deriche_a_0;
    _S742.value3_1.deriche_b_0 = params_0.ce_0.deriche_b_0;
    _S742.value3_1.shifts_0 = params_0.ce_0.shifts_0;
    _S742.value3_1.white_0 = params_0.ce_0.white_0;
    _S742.value3_1.slots_0 = params_0.ce_0.slots_0;
    _S742.value3_1.rgb_to_xyz_r0_0 = params_0.ce_0.rgb_to_xyz_r0_0;
    _S742.value3_1.rgb_to_xyz_r1_0 = params_0.ce_0.rgb_to_xyz_r1_0;
    _S742.value3_1.rgb_to_xyz_r2_0 = params_0.ce_0.rgb_to_xyz_r2_0;
    _S742.value3_1.xyz_to_rgb_r0_0 = params_0.ce_0.xyz_to_rgb_r0_0;
    _S742.value3_1.xyz_to_rgb_r1_0 = params_0.ce_0.xyz_to_rgb_r1_0;
    _S742.value3_1.xyz_to_rgb_r2_0 = params_0.ce_0.xyz_to_rgb_r2_0;
    _S742.value4_1 = _S738;
    _S742.value5_1 = (*_S739).primal_0;
    var _S743 : s_bwdCallableCtx_colorequal_0 = s_bwdCallableCtx_colorequal_0( _S742, _S741.value1_12 );
    var _S744 : Slots_0 = (*_S739).differential_0;
    s_bwdProp_colorequal_0(_S743, &(_S744), _S740);
    (*_S739).differential_0 = _S744;
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn colorequal_backward(@builtin(global_invocation_id) id_1 : vec3<u32>)
{
    var _S745 : u32 = id_1.x;
    var _S746 : bool;
    if((_S745 >= (params_0.pass_0.width_0)))
    {
        _S746 = true;
    }
    else
    {
        _S746 = ((id_1.y) >= (params_0.pass_0.height_0));
    }
    if(_S746)
    {
        return;
    }
    var i_4 : u32 = id_1.y * params_0.pass_0.width_0 + _S745;
    var _S747 : Pixel_0 = pixel_of_0(input_0[i_4].xyz, place_0(id_1));
    var _S748 : Slots_0 = Slots_x24_syn_dzero_0();
    var s_5 : DiffPair_Slots_0;
    s_5.primal_0 = slots_of_0();
    s_5.differential_0 = _S748;
    _S734(_S747.l_star_3, _S747.uv_star_6, _S747.guided_0, true, &(s_5), derivative_in_0[i_4].xyz);
    derivative_out_0[i_4] = vec4<f32>(s_5.differential_0.hue_red_0, s_5.differential_0.saturation_red_0, s_5.differential_0.brightness_red_0, s_5.differential_0.white_level_0);
    return;
}

fn s_fwd_colorequal_0( _S749 : f32,  _S750 : vec2<f32>,  _S751 : vec3<f32>,  _S752 : bool,  _S753 : DiffPair_Slots_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S754 : f32 = params_0.ce_0.white_0.x;
    var white_4 : f32;
    var s_diff_white_0 : f32;
    if(_S752)
    {
        var _S755 : DiffPair_float_0 = DiffPair_float_0( _S753.primal_0.white_level_0, _S753.differential_0.white_level_0 );
        var _S756 : DiffPair_float_0 = _d_exp2_1(_S755);
        var _S757 : DiffPair_float_0 = DiffPair_float_0( _S756.primal_0, _S756.differential_0 );
        var _S758 : DiffPair_float_0 = s_fwd_y_to_ucs_l_star_0(_S757);
        white_4 = _S758.primal_0;
        s_diff_white_0 = _S758.differential_0;
    }
    else
    {
        white_4 = _S754;
        s_diff_white_0 = 0.0f;
    }
    var _S759 : DiffPair_float_0 = DiffPair_float_0( _S749, 0.0f );
    var _S760 : DiffPair_float_0 = DiffPair_float_0( white_4, s_diff_white_0 );
    var _S761 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( _S750, vec2<f32>(0.0f) );
    var _S762 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_luv_to_jch_0(_S759, _S760, _S761);
    var _S763 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S762.primal_0, _S762.differential_0 );
    var _S764 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_ucs_jch_to_hsb_0(_S763);
    var h_3 : f32 = _S764.primal_0.x;
    var s_diff_h_0 : f32 = _S764.differential_0.x;
    var _S765 : DiffPair_float_0 = DiffPair_float_0( h_3, s_diff_h_0 );
    var _S766 : DiffPair_float_0 = DiffPair_float_0( _S753.primal_0.hue_red_0, _S753.differential_0.hue_red_0 );
    var _S767 : DiffPair_float_0 = DiffPair_float_0( params_0.ce_0.slots_0.x, 0.0f );
    var _S768 : DiffPair_float_0 = s_fwd_curve_0(u32(0), u32(2048), _S765, _S766, _S767, _S752);
    var _S769 : DiffPair_float_0 = DiffPair_float_0( _S753.primal_0.saturation_red_0, _S753.differential_0.saturation_red_0 );
    var _S770 : DiffPair_float_0 = DiffPair_float_0( params_0.ce_0.slots_0.y, 0.0f );
    var _S771 : DiffPair_float_0 = s_fwd_curve_0(u32(512), u32(2560), _S765, _S769, _S770, _S752);
    var _S772 : DiffPair_float_0 = DiffPair_float_0( _S753.primal_0.brightness_red_0, _S753.differential_0.brightness_red_0 );
    var _S773 : DiffPair_float_0 = DiffPair_float_0( params_0.ce_0.slots_0.z, 0.0f );
    var _S774 : DiffPair_float_0 = s_fwd_curve_0(u32(1024), u32(3072), _S765, _S772, _S773, _S752);
    var guided_mode_3 : bool = (params_0.ce_0.mode_0.z) != u32(0);
    var chromatic_3 : bool = ((_S762.primal_0.y) > 1.52587890625e-05f);
    var hue_correction_3 : f32;
    var s_diff_hue_correction_0 : f32;
    if(guided_mode_3)
    {
        hue_correction_3 = _S751.x;
        s_diff_hue_correction_0 = 0.0f;
    }
    else
    {
        if(chromatic_3)
        {
            hue_correction_3 = _S768.primal_0;
            s_diff_hue_correction_0 = _S768.differential_0;
        }
        else
        {
            hue_correction_3 = 0.0f;
            s_diff_hue_correction_0 = 0.0f;
        }
    }
    var saturation_correction_3 : f32;
    var s_diff_saturation_correction_0 : f32;
    if(guided_mode_3)
    {
        saturation_correction_3 = _S751.y;
        s_diff_saturation_correction_0 = 0.0f;
    }
    else
    {
        if(chromatic_3)
        {
            saturation_correction_3 = _S771.primal_0;
            s_diff_saturation_correction_0 = _S771.differential_0;
        }
        else
        {
            saturation_correction_3 = 1.0f;
            s_diff_saturation_correction_0 = 0.0f;
        }
    }
    var brightness_correction_3 : f32;
    var s_diff_brightness_correction_0 : f32;
    if(guided_mode_3)
    {
        brightness_correction_3 = _S751.z;
        s_diff_brightness_correction_0 = 0.0f;
    }
    else
    {
        if(chromatic_3)
        {
            var _S775 : f32 = _S764.primal_0.y;
            var _S776 : f32 = _S774.primal_0 - 1.0f;
            var _S777 : f32 = _S764.differential_0.y * _S776 + _S774.differential_0 * _S775;
            brightness_correction_3 = _S775 * _S776;
            s_diff_brightness_correction_0 = _S777;
        }
        else
        {
            brightness_correction_3 = 0.0f;
            s_diff_brightness_correction_0 = 0.0f;
        }
    }
    var hue_5 : f32 = h_3 + hue_correction_3;
    var s_diff_hue_0 : f32 = s_diff_h_0 + s_diff_hue_correction_0;
    var _S778 : f32 = _S764.primal_0.y;
    var _S779 : f32 = 1.0f + 2.0f * (saturation_correction_3 - 1.0f);
    var _S780 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S781 : DiffPair_float_0 = DiffPair_float_0( _S778 * _S779, _S764.differential_0.y * _S779 + s_diff_saturation_correction_0 * 2.0f * _S778 );
    var _S782 : DiffPair_float_0 = _d_max_1(_S780, _S781);
    var _S783 : f32 = _S764.primal_0.z;
    var _S784 : f32 = 1.0f + 8.0f * brightness_correction_3;
    var _S785 : DiffPair_float_0 = DiffPair_float_0( _S783 * _S784, _S764.differential_0.z * _S784 + s_diff_brightness_correction_0 * 8.0f * _S783 );
    var _S786 : DiffPair_float_0 = _d_max_1(_S780, _S785);
    var _S787 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(hue_5, _S782.primal_0, _S786.primal_0), vec3<f32>(s_diff_hue_0, _S782.differential_0, _S786.differential_0) );
    var _S788 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_ucs_hsb_to_jch_0(_S787);
    var _S789 : f32 = _S788.primal_0.z;
    var _S790 : f32 = _S788.differential_0.z;
    var _S791 : DiffPair_float_0 = DiffPair_float_0( _S789, _S790 );
    var _S792 : DiffPair_float_0 = s_fwd_lookup_0(u32(1536), _S791);
    var _S793 : f32 = _S788.primal_0.x;
    var _S794 : f32 = _S788.differential_0.x;
    var _S795 : DiffPair_float_0 = DiffPair_float_0( _S793 * white_4, _S794 * white_4 + s_diff_white_0 * _S793 );
    var _S796 : DiffPair_float_0 = DiffPair_float_0( 0.65239977836608887f, 0.0f );
    var _S797 : DiffPair_float_0 = pow0_forward_0(_S795, _S796);
    var _S798 : f32 = 15.93299388885498f * _S797.primal_0;
    var _S799 : DiffPair_float_0 = DiffPair_float_0( _S792.primal_0, _S792.differential_0 );
    var _S800 : DiffPair_float_0 = DiffPair_float_0( 0.60075569152832031f, 0.0f );
    var _S801 : DiffPair_float_0 = pow0_forward_0(_S799, _S800);
    var _S802 : f32 = _S798 * _S801.primal_0;
    var _S803 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S793, _S802 / white_4, _S789), vec3<f32>(_S794, ((_S797.differential_0 * 15.93299388885498f * _S801.primal_0 + _S801.differential_0 * _S798) * white_4 - _S802 * s_diff_white_0) / (white_4 * white_4), _S790) );
    var _S804 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_ucs_jch_to_hsb_0(_S803);
    var _S805 : f32 = _S804.primal_0.y;
    var _S806 : f32 = _S804.differential_0.y;
    var _S807 : DiffPair_float_0 = DiffPair_float_0( _S782.primal_0, _S782.differential_0 );
    var _S808 : DiffPair_float_0 = DiffPair_float_0( 0.80000001192092896f * _S805, _S806 * 0.80000001192092896f );
    var _S809 : DiffPair_float_0 = DiffPair_float_0( _S805, _S806 );
    var _S810 : DiffPair_float_0 = s_fwd_soft_clip_0(_S807, _S808, _S809);
    var _S811 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(hue_5, _S810.primal_0, _S786.primal_0), vec3<f32>(s_diff_hue_0, _S810.differential_0, _S786.differential_0) );
    var _S812 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_ucs_hsb_to_jch_0(_S811);
    var _S813 : f32 = _S812.primal_0.x;
    var _S814 : f32 = _S812.primal_0.y;
    var _S815 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S813 * white_4, _S814 * white_4, _S812.primal_0.z), vec3<f32>(_S812.differential_0.x * white_4 + s_diff_white_0 * _S813, _S812.differential_0.y * white_4 + s_diff_white_0 * _S814, _S812.differential_0.z) );
    var _S816 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_ucs_jch_to_xyy_0(_S815, 1.0f);
    var _S817 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S816.primal_0, _S816.differential_0 );
    var _S818 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_xyy_to_xyz_0(_S817);
    var _S819 : mat3x3<f32> = xyz_to_rgb_of_1();
    var _S820 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( (((_S818.primal_0) * (_S819))), (((_S818.primal_0) * (mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f)))) + (((_S818.differential_0) * (_S819))) );
    return _S820;
}

@compute
@workgroup_size(16, 16, 1)
fn colorequal_forward_diff(@builtin(global_invocation_id) id_2 : vec3<u32>)
{
    var _S821 : u32 = id_2.x;
    var _S822 : bool;
    if((_S821 >= (params_0.pass_0.width_0)))
    {
        _S822 = true;
    }
    else
    {
        _S822 = ((id_2.y) >= (params_0.pass_0.height_0));
    }
    if(_S822)
    {
        return;
    }
    var i_5 : u32 = id_2.y * params_0.pass_0.width_0 + _S821;
    var _S823 : Pixel_0 = pixel_of_0(input_0[i_5].xyz, place_0(id_2));
    var v_4 : vec4<f32> = derivative_in_0[i_5];
    var direction_0 : Slots_0;
    direction_0.hue_red_0 = v_4.x;
    direction_0.saturation_red_0 = v_4.y;
    direction_0.brightness_red_0 = v_4.z;
    direction_0.white_level_0 = v_4.w;
    var _S824 : DiffPair_Slots_0 = DiffPair_Slots_0( slots_of_0(), direction_0 );
    derivative_out_0[i_5] = vec4<f32>(s_fwd_colorequal_0(_S823.l_star_3, _S823.uv_star_6, _S823.guided_0, true, _S824).differential_0, 0.0f);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn sample(@builtin(global_invocation_id) id_3 : vec3<u32>)
{
    var _S825 : u32 = id_3.x;
    var _S826 : bool;
    if((_S825 >= (params_0.pass_0.width_0)))
    {
        _S826 = true;
    }
    else
    {
        _S826 = ((id_3.y) >= (params_0.pass_0.height_0));
    }
    if(_S826)
    {
        return;
    }
    var _S827 : Sample_0 = sample_of_0(input_0[id_3.y * params_0.pass_0.width_0 + _S825].xyz);
    var _S828 : u32 = place_0(id_3);
    uv_0[_S828] = _S827.uv_6;
    saturation_0[_S828] = _S827.saturation_1;
    lightness_0[_S828] = _S827.l_star_4;
    return;
}

fn clampf_0( a_0 : f32,  low_0 : f32,  high_0 : f32) -> f32
{
    var _S829 : f32;
    if((a_0 < low_0))
    {
        _S829 = low_0;
    }
    else
    {
        _S829 = min(a_0, high_0);
    }
    return _S829;
}

fn clampf_1( a_1 : vec2<f32>,  low_1 : vec2<f32>,  high_1 : vec2<f32>) -> vec2<f32>
{
    return vec2<f32>(clampf_0(a_1.x, low_1.x, high_1.x), clampf_0(a_1.y, low_1.y, high_1.y));
}

struct Deriche_0
{
     a0_0 : f32,
     a1_0 : f32,
     a2_0 : f32,
     a3_0 : f32,
     b1_0 : f32,
     b2_0 : f32,
     coefp_0 : f32,
     coefn_0 : f32,
};

fn causal_0( d_1 : Deriche_0,  xc_0 : vec2<f32>,  xp_0 : vec2<f32>,  yp_0 : vec2<f32>,  yb_0 : vec2<f32>) -> vec2<f32>
{
    return vec2<f32>(d_1.a0_0) * xc_0 + vec2<f32>(d_1.a1_0) * xp_0 - vec2<f32>(d_1.b1_0) * yp_0 - vec2<f32>(d_1.b2_0) * yb_0;
}

fn anticausal_0( d_2 : Deriche_0,  xn_0 : vec2<f32>,  xa_0 : vec2<f32>,  yn_0 : vec2<f32>,  ya_0 : vec2<f32>) -> vec2<f32>
{
    return vec2<f32>(d_2.a2_0) * xn_0 + vec2<f32>(d_2.a3_0) * xa_0 - vec2<f32>(d_2.b1_0) * yn_0 - vec2<f32>(d_2.b2_0) * ya_0;
}

fn walk2_0( start_0 : u32,  stride_0 : u32,  count_0 : u32,  d_3 : Deriche_0)
{
    var low_2 : vec2<f32> = vec2<f32>(-1.0e+09f);
    var high_2 : vec2<f32> = vec2<f32>(1.0e+09f);
    var xp_1 : vec2<f32> = clampf_1(line2_in_0[start_0], low_2, high_2);
    var yb_1 : vec2<f32> = xp_1 * vec2<f32>(d_3.coefp_0);
    var xp_2 : vec2<f32> = xp_1;
    var yp_1 : vec2<f32> = yb_1;
    var yb_2 : vec2<f32> = yb_1;
    var n_0 : u32 = u32(0);
    loop
    {
        if((n_0 < count_0))
        {
        }
        else
        {
            break;
        }
        var at_0 : u32 = start_0 + n_0 * stride_0;
        var xc_1 : vec2<f32> = clampf_1(line2_in_0[at_0], low_2, high_2);
        var yc_0 : vec2<f32> = causal_0(d_3, xc_1, xp_2, yp_1, yb_2);
        line2_out_0[at_0] = yc_0;
        var _S830 : u32 = n_0 + u32(1);
        xp_2 = xc_1;
        yb_2 = yp_1;
        yp_1 = yc_0;
        n_0 = _S830;
    }
    var xn_1 : vec2<f32> = clampf_1(line2_in_0[start_0 + (count_0 - u32(1)) * stride_0], low_2, high_2);
    var yn_1 : vec2<f32> = xn_1 * vec2<f32>(d_3.coefn_0);
    var xn_2 : vec2<f32> = xn_1;
    var xa_1 : vec2<f32> = xn_1;
    var yn_2 : vec2<f32> = yn_1;
    var ya_1 : vec2<f32> = yn_1;
    n_0 = count_0;
    loop
    {
        if((n_0 > u32(0)))
        {
        }
        else
        {
            break;
        }
        var _S831 : u32 = n_0 - u32(1);
        var at_1 : u32 = start_0 + _S831 * stride_0;
        var xc_2 : vec2<f32> = clampf_1(line2_in_0[at_1], low_2, high_2);
        var yc_1 : vec2<f32> = anticausal_0(d_3, xn_2, xa_1, yn_2, ya_1);
        line2_out_0[at_1] = line2_out_0[at_1] + yc_1;
        xa_1 = xn_2;
        xn_2 = xc_2;
        ya_1 = yn_2;
        yn_2 = yc_1;
        n_0 = _S831;
    }
    return;
}

fn deriche_of_0() -> Deriche_0
{
    var d_4 : Deriche_0;
    d_4.a0_0 = params_0.ce_0.deriche_a_0.x;
    d_4.a1_0 = params_0.ce_0.deriche_a_0.y;
    d_4.a2_0 = params_0.ce_0.deriche_a_0.z;
    d_4.a3_0 = params_0.ce_0.deriche_a_0.w;
    d_4.b1_0 = params_0.ce_0.deriche_b_0.x;
    d_4.b2_0 = params_0.ce_0.deriche_b_0.y;
    d_4.coefp_0 = params_0.ce_0.deriche_b_0.z;
    d_4.coefn_0 = params_0.ce_0.deriche_b_0.w;
    return d_4;
}

@compute
@workgroup_size(16, 16, 1)
fn blur_columns2(@builtin(global_invocation_id) id_4 : vec3<u32>)
{
    var column_0 : u32 = id_4.y * u32(16) + id_4.x;
    var _S832 : vec4<u32> = params_0.ce_0.size_0;
    var _S833 : u32 = params_0.ce_0.size_0.z;
    if((column_0 >= _S833))
    {
        return;
    }
    walk2_0(column_0, _S833, _S832.w, deriche_of_0());
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn blur_rows2(@builtin(global_invocation_id) id_5 : vec3<u32>)
{
    var row_0 : u32 = id_5.y * u32(16) + id_5.x;
    var _S834 : vec4<u32> = params_0.ce_0.size_0;
    if((row_0 >= (params_0.ce_0.size_0.w)))
    {
        return;
    }
    var _S835 : u32 = _S834.z;
    walk2_0(row_0 * _S835, u32(1), _S835, deriche_of_0());
    return;
}

fn causal_1( d_5 : Deriche_0,  xc_3 : f32,  xp_3 : f32,  yp_2 : f32,  yb_3 : f32) -> f32
{
    return d_5.a0_0 * xc_3 + d_5.a1_0 * xp_3 - d_5.b1_0 * yp_2 - d_5.b2_0 * yb_3;
}

fn anticausal_1( d_6 : Deriche_0,  xn_3 : f32,  xa_2 : f32,  yn_3 : f32,  ya_2 : f32) -> f32
{
    return d_6.a2_0 * xn_3 + d_6.a3_0 * xa_2 - d_6.b1_0 * yn_3 - d_6.b2_0 * ya_2;
}

fn walk1_0( start_1 : u32,  stride_1 : u32,  count_1 : u32,  d_7 : Deriche_0)
{
    var xp_4 : f32 = clampf_0(line1_in_0[start_1], -1.0e+09f, 1.0e+09f);
    var yb_4 : f32 = xp_4 * d_7.coefp_0;
    var xp_5 : f32 = xp_4;
    var yp_3 : f32 = yb_4;
    var yb_5 : f32 = yb_4;
    var n_1 : u32 = u32(0);
    loop
    {
        if((n_1 < count_1))
        {
        }
        else
        {
            break;
        }
        var at_2 : u32 = start_1 + n_1 * stride_1;
        var xc_4 : f32 = clampf_0(line1_in_0[at_2], -1.0e+09f, 1.0e+09f);
        var yc_2 : f32 = causal_1(d_7, xc_4, xp_5, yp_3, yb_5);
        line1_out_0[at_2] = yc_2;
        var _S836 : u32 = n_1 + u32(1);
        xp_5 = xc_4;
        yb_5 = yp_3;
        yp_3 = yc_2;
        n_1 = _S836;
    }
    var xn_4 : f32 = clampf_0(line1_in_0[start_1 + (count_1 - u32(1)) * stride_1], -1.0e+09f, 1.0e+09f);
    var yn_4 : f32 = xn_4 * d_7.coefn_0;
    var xn_5 : f32 = xn_4;
    var xa_3 : f32 = xn_4;
    var yn_5 : f32 = yn_4;
    var ya_3 : f32 = yn_4;
    n_1 = count_1;
    loop
    {
        if((n_1 > u32(0)))
        {
        }
        else
        {
            break;
        }
        var _S837 : u32 = n_1 - u32(1);
        var at_3 : u32 = start_1 + _S837 * stride_1;
        var xc_5 : f32 = clampf_0(line1_in_0[at_3], -1.0e+09f, 1.0e+09f);
        var yc_3 : f32 = anticausal_1(d_7, xn_5, xa_3, yn_5, ya_3);
        line1_out_0[at_3] = line1_out_0[at_3] + yc_3;
        xa_3 = xn_5;
        xn_5 = xc_5;
        ya_3 = yn_5;
        yn_5 = yc_3;
        n_1 = _S837;
    }
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn blur_columns1(@builtin(global_invocation_id) id_6 : vec3<u32>)
{
    var column_1 : u32 = id_6.y * u32(16) + id_6.x;
    var _S838 : vec4<u32> = params_0.ce_0.size_0;
    var _S839 : u32 = params_0.ce_0.size_0.z;
    if((column_1 >= _S839))
    {
        return;
    }
    walk1_0(column_1, _S839, _S838.w, deriche_of_0());
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn blur_rows1(@builtin(global_invocation_id) id_7 : vec3<u32>)
{
    var row_1 : u32 = id_7.y * u32(16) + id_7.x;
    var _S840 : vec4<u32> = params_0.ce_0.size_0;
    if((row_1 >= (params_0.ce_0.size_0.w)))
    {
        return;
    }
    var _S841 : u32 = _S840.z;
    walk1_0(row_1 * _S841, u32(1), _S841, deriche_of_0());
    return;
}

struct Taps_0
{
     x_prev_3 : u32,
     x_next_0 : u32,
     y_prev_0 : u32,
     y_next_0 : u32,
     dx_next_0 : f32,
     dx_prev_0 : f32,
     dy_next_0 : f32,
     dy_prev_0 : f32,
};

fn bilinear_0( at_4 : vec2<u32>,  size_in_0 : vec2<u32>,  size_out_0 : vec2<u32>) -> Taps_0
{
    var _S842 : u32 = size_in_0.x;
    var x_in_0 : f32 = f32(at_4.x) / f32(size_out_0.x) * f32(_S842);
    var _S843 : u32 = size_in_0.y;
    var y_in_0 : f32 = f32(at_4.y) / f32(size_out_0.y) * f32(_S843);
    var x_prev_4 : u32 = u32(floor(x_in_0));
    var y_prev_1 : u32 = u32(floor(y_in_0));
    var t_0 : Taps_0;
    var _S844 : u32 = _S842 - u32(1);
    var _S845 : u32 = min(x_prev_4 + u32(1), _S844);
    t_0.x_next_0 = _S845;
    var _S846 : u32 = _S843 - u32(1);
    var _S847 : u32 = min(y_prev_1 + u32(1), _S846);
    t_0.y_next_0 = _S847;
    t_0.x_prev_3 = min(x_prev_4, _S844);
    t_0.y_prev_0 = min(y_prev_1, _S846);
    var _S848 : f32 = f32(_S845) - x_in_0;
    t_0.dx_next_0 = _S848;
    t_0.dx_prev_0 = 1.0f - _S848;
    var _S849 : f32 = f32(_S847) - y_in_0;
    t_0.dy_next_0 = _S849;
    t_0.dy_prev_0 = 1.0f - _S849;
    return t_0;
}

fn interpolate_0( t_1 : Taps_0,  nw_0 : vec2<f32>,  ne_0 : vec2<f32>,  sw_0 : vec2<f32>,  se_0 : vec2<f32>) -> vec2<f32>
{
    var _S850 : vec2<f32> = vec2<f32>(t_1.dx_next_0);
    var _S851 : vec2<f32> = vec2<f32>(t_1.dx_prev_0);
    return vec2<f32>(t_1.dy_prev_0) * (sw_0 * _S850 + se_0 * _S851) + vec2<f32>(t_1.dy_next_0) * (nw_0 * _S850 + ne_0 * _S851);
}

fn uv_on_grid_0( _S852 : vec2<u32>) -> vec2<f32>
{
    var _S853 : vec4<u32> = params_0.ce_0.size_0;
    var w_0 : u32 = params_0.ce_0.size_0.x;
    if((params_0.ce_0.mode_0.y) == u32(0))
    {
        return uv_0[_S852.y * w_0 + _S852.x];
    }
    var t_2 : Taps_0 = bilinear_0(_S852, _S853.xy, _S853.zw);
    var _S854 : u32 = t_2.y_prev_0 * w_0;
    var _S855 : u32 = t_2.y_next_0 * w_0;
    return interpolate_0(t_2, uv_0[_S854 + t_2.x_prev_3], uv_0[_S854 + t_2.x_next_0], uv_0[_S855 + t_2.x_prev_3], uv_0[_S855 + t_2.x_next_0]);
}

@compute
@workgroup_size(16, 16, 1)
fn prefilter_init(@builtin(global_invocation_id) id_8 : vec3<u32>)
{
    var _S856 : u32 = id_8.x;
    var _S857 : vec4<u32> = params_0.ce_0.size_0;
    var _S858 : u32 = params_0.ce_0.size_0.z;
    var _S859 : bool;
    if((_S856 >= _S858))
    {
        _S859 = true;
    }
    else
    {
        _S859 = ((id_8.y) >= (_S857.w));
    }
    if(_S859)
    {
        return;
    }
    var d_8 : u32 = id_8.y * _S858 + _S856;
    var _S860 : vec2<f32> = uv_on_grid_0(id_8.xy);
    grid_uv_0[d_8] = _S860;
    var _S861 : f32 = _S860.x;
    var _S862 : f32 = _S860.y;
    var _S863 : f32 = _S861 * _S862;
    p1_0[d_8] = vec2<f32>(_S861 * _S861, _S863);
    p2_0[d_8] = vec2<f32>(_S863, _S862 * _S862);
    return;
}

fn inverse_of_0( sigma_0 : vec4<f32>,  invertible_0 : ptr<function, bool>) -> vec4<f32>
{
    var _S864 : f32 = sigma_0.x;
    var _S865 : f32 = sigma_0.w;
    var _S866 : f32 = sigma_0.y;
    var _S867 : f32 = sigma_0.z;
    var det_0 : f32 = _S864 * _S865 - _S866 * _S867;
    var _S868 : bool = ((abs(det_0)) > 4.76837158203125e-07f);
    (*invertible_0) = _S868;
    if(!_S868)
    {
        return vec4<f32>(0.0f);
    }
    return vec4<f32>(_S865 / det_0, - _S866 / det_0, - _S867 / det_0, _S864 / det_0);
}

@compute
@workgroup_size(16, 16, 1)
fn prefilter_coefficients(@builtin(global_invocation_id) id_9 : vec3<u32>)
{
    var _S869 : u32 = id_9.x;
    var _S870 : vec4<u32> = params_0.ce_0.size_0;
    var _S871 : u32 = params_0.ce_0.size_0.z;
    var _S872 : bool;
    if((_S869 >= _S871))
    {
        _S872 = true;
    }
    else
    {
        _S872 = ((id_9.y) >= (_S870.w));
    }
    if(_S872)
    {
        return;
    }
    var d_9 : u32 = id_9.y * _S871 + _S869;
    var _S873 : f32 = grid_uv_0[d_9].x;
    var _S874 : f32 = p1_0[d_9].x - _S873 * _S873;
    var _S875 : f32 = grid_uv_0[d_9].y;
    var _S876 : f32 = _S873 * _S875;
    var _S877 : f32 = p1_0[d_9].y - _S876;
    var _S878 : f32 = p2_0[d_9].x - _S876;
    var _S879 : f32 = p2_0[d_9].y - _S875 * _S875;
    var eps_0 : f32 = params_0.ce_0.shifts_0.z;
    var invertible_1 : bool;
    var inv_0 : vec4<f32> = inverse_of_0(vec4<f32>(_S874 + eps_0, _S877, _S878, _S879 + eps_0), &(invertible_1));
    var _S880 : vec4<f32> = vec4<f32>(0.0f);
    var a_2 : vec4<f32>;
    if(invertible_1)
    {
        var _S881 : f32 = inv_0.x;
        var _S882 : f32 = inv_0.y;
        var _S883 : f32 = inv_0.z;
        var _S884 : f32 = inv_0.w;
        a_2 = vec4<f32>(_S874 * _S881 + _S877 * _S882, _S874 * _S883 + _S877 * _S884, _S878 * _S881 + _S879 * _S882, _S878 * _S883 + _S879 * _S884);
    }
    else
    {
        a_2 = _S880;
    }
    p1_0[d_9] = a_2.xy;
    p2_0[d_9] = a_2.zw;
    c1_0[d_9] = vec2<f32>(_S873 - a_2.x * _S873 - a_2.y * _S875, _S875 - a_2.z * _S873 - a_2.w * _S875);
    return;
}

fn satweight_0( offset_6 : u32,  sat_0 : f32) -> f32
{
    var isat_0 : f32 = 4096.0f * (1.0f + clamp(sat_0, -1.0f, 0.999755859375f));
    var base_0 : f32 = floor(isat_0);
    var _S885 : u32 = offset_6 + u32(base_0);
    var w_1 : f32 = table_0[_S885];
    return w_1 + (isat_0 - base_0) * (table_0[_S885 + u32(1)] - w_1);
}

struct Coefficients_0
{
     a_3 : vec4<f32>,
     b_4 : vec2<f32>,
};

fn coefficients_at_0( _S886 : vec2<u32>,  _S887 : u32) -> Coefficients_0
{
    var c_4 : Coefficients_0;
    if((params_0.ce_0.mode_0.y) == u32(0))
    {
        c_4.a_3 = vec4<f32>(p1_0[_S887], p2_0[_S887]);
        c_4.b_4 = c1_0[_S887];
        return c_4;
    }
    var w_2 : u32 = params_0.ce_0.size_0.z;
    var t_3 : Taps_0 = bilinear_0(_S886, params_0.ce_0.size_0.zw, params_0.ce_0.size_0.xy);
    var _S888 : u32 = t_3.y_prev_0 * w_2;
    var nw_1 : u32 = _S888 + t_3.x_prev_3;
    var ne_1 : u32 = _S888 + t_3.x_next_0;
    var _S889 : u32 = t_3.y_next_0 * w_2;
    var sw_1 : u32 = _S889 + t_3.x_prev_3;
    var se_1 : u32 = _S889 + t_3.x_next_0;
    c_4.a_3 = vec4<f32>(interpolate_0(t_3, p1_0[nw_1], p1_0[ne_1], p1_0[sw_1], p1_0[se_1]), interpolate_0(t_3, p2_0[nw_1], p2_0[ne_1], p2_0[sw_1], p2_0[se_1]));
    c_4.b_4 = interpolate_0(t_3, c1_0[nw_1], c1_0[ne_1], c1_0[sw_1], c1_0[se_1]);
    return c_4;
}

@compute
@workgroup_size(16, 16, 1)
fn prefilter_apply(@builtin(global_invocation_id) id_10 : vec3<u32>)
{
    var _S890 : u32 = id_10.x;
    var _S891 : vec4<u32> = params_0.ce_0.size_0;
    var _S892 : u32 = params_0.ce_0.size_0.x;
    var _S893 : bool;
    if((_S890 >= _S892))
    {
        _S893 = true;
    }
    else
    {
        _S893 = ((id_10.y) >= (_S891.y));
    }
    if(_S893)
    {
        return;
    }
    var k_0 : u32 = id_10.y * _S892 + _S890;
    var base_1 : vec2<f32>;
    if((params_0.ce_0.mode_0.w) != u32(0))
    {
        base_1 = grid_uv_0[k_0];
    }
    else
    {
        base_1 = uv_0[k_0];
    }
    var _S894 : Coefficients_0 = coefficients_at_0(id_10.xy, k_0);
    var _S895 : f32 = base_1.x;
    var _S896 : f32 = base_1.y;
    uv_0[k_0] = vec2<f32>(satweight_0(u32(0), saturation_0[k_0] - params_0.ce_0.shifts_0.x)) * (vec2<f32>(_S894.a_3.x * _S895 + _S894.a_3.y * _S896 + _S894.b_4.x, _S894.a_3.z * _S895 + _S894.a_3.w * _S896 + _S894.b_4.y) - base_1) + base_1;
    return;
}

fn scharr_at_0( _S897 : i32,  _S898 : i32) -> f32
{
    var w_3 : i32 = i32(params_0.ce_0.size_0.x);
    var _S899 : i32 = i32(params_0.ce_0.size_0.y) - i32(1);
    var up_0 : i32 = clamp(_S897 - i32(1), i32(0), _S899) * w_3;
    var mid_0 : i32 = clamp(_S897, i32(0), _S899) * w_3;
    var down_0 : i32 = clamp(_S897 + i32(1), i32(0), _S899) * w_3;
    var _S900 : i32 = w_3 - i32(1);
    var left_2 : i32 = clamp(_S898 - i32(1), i32(0), _S900);
    var centre_0 : i32 = clamp(_S898, i32(0), _S900);
    var right_2 : i32 = clamp(_S898 + i32(1), i32(0), _S900);
    var gx_0 : f32 = 0.18431372940540314f * (saturation_0[up_0 + left_2] - saturation_0[up_0 + right_2] + saturation_0[down_0 + left_2] - saturation_0[down_0 + right_2]) + 0.63529413938522339f * (saturation_0[mid_0 + left_2] - saturation_0[mid_0 + right_2]);
    var gy_0 : f32 = 0.18431372940540314f * (saturation_0[up_0 + left_2] - saturation_0[down_0 + left_2] + saturation_0[up_0 + right_2] - saturation_0[down_0 + right_2]) + 0.63529413938522339f * (saturation_0[up_0 + centre_0] - saturation_0[down_0 + centre_0]);
    return sqrt(gx_0 * gx_0 + gy_0 * gy_0);
}

@compute
@workgroup_size(16, 16, 1)
fn corrections_pass(@builtin(global_invocation_id) id_11 : vec3<u32>)
{
    var _S901 : u32 = id_11.x;
    var _S902 : vec4<u32> = params_0.ce_0.size_0;
    var _S903 : u32 = params_0.ce_0.size_0.x;
    var _S904 : bool;
    if((_S901 >= _S903))
    {
        _S904 = true;
    }
    else
    {
        _S904 = ((id_11.y) >= (_S902.y));
    }
    if(_S904)
    {
        return;
    }
    var _S905 : u32 = id_11.y;
    var k_1 : u32 = _S905 * _S903 + _S901;
    var jch_3 : vec3<f32> = luv_to_jch_0(lightness_0[k_1], params_0.ce_0.white_0.x, uv_0[k_1]);
    var hsb_2 : vec3<f32> = ucs_jch_to_hsb_0(jch_3);
    var _S906 : f32 = max(0.0f, scharr_at_0(min(i32(_S902.y) - i32(2), max(i32(1), i32(_S905))), min(i32(_S903) - i32(2), max(i32(1), i32(_S901)))) - 0.019999999552965164f);
    var _S907 : vec4<f32> = params_0.ce_0.shifts_0;
    lightness_0[k_1] = params_0.ce_0.shifts_0.w * (_S906 * _S906);
    const _S908 : vec2<f32> = vec2<f32>(0.0f, 1.0f);
    var c_5 : vec2<f32>;
    var b_5 : f32;
    if(((jch_3.y) > 1.52587890625e-05f))
    {
        var _S909 : f32 = hsb_2.x;
        var _S910 : f32 = hsb_2.y * (lookup_0(u32(1024), _S909) - 1.0f);
        c_5 = vec2<f32>(lookup_0(u32(0), _S909), lookup_0(u32(512), _S909));
        b_5 = _S910;
    }
    else
    {
        c_5 = _S908;
        b_5 = 0.0f;
    }
    corrections_0[k_1] = c_5;
    brightness_0[k_1] = b_5;
    weights_0[k_1] = vec2<f32>(satweight_0(u32(1536), saturation_0[k_1] - _S907.x), satweight_0(u32(1536), saturation_0[k_1] - _S907.y));
    return;
}

fn interpolate_1( t_4 : Taps_0,  nw_2 : f32,  ne_2 : f32,  sw_2 : f32,  se_2 : f32) -> f32
{
    return t_4.dy_prev_0 * (sw_2 * t_4.dx_next_0 + se_2 * t_4.dx_prev_0) + t_4.dy_next_0 * (nw_2 * t_4.dx_next_0 + ne_2 * t_4.dx_prev_0);
}

@compute
@workgroup_size(16, 16, 1)
fn guide_downsample(@builtin(global_invocation_id) id_12 : vec3<u32>)
{
    var _S911 : u32 = id_12.x;
    var _S912 : vec4<u32> = params_0.ce_0.size_0;
    var _S913 : u32 = params_0.ce_0.size_0.z;
    var _S914 : bool;
    if((_S911 >= _S913))
    {
        _S914 = true;
    }
    else
    {
        _S914 = ((id_12.y) >= (_S912.w));
    }
    if(_S914)
    {
        return;
    }
    var d_10 : u32 = id_12.y * _S913 + _S911;
    if((params_0.ce_0.mode_0.y) == u32(0))
    {
        grid_corrections_0[d_10] = corrections_0[d_10];
        grid_brightness_0[d_10] = brightness_0[d_10];
        return;
    }
    var w_4 : u32 = _S912.x;
    var t_5 : Taps_0 = bilinear_0(id_12.xy, _S912.xy, _S912.zw);
    var _S915 : u32 = t_5.y_prev_0 * w_4;
    var nw_3 : u32 = _S915 + t_5.x_prev_3;
    var ne_3 : u32 = _S915 + t_5.x_next_0;
    var _S916 : u32 = t_5.y_next_0 * w_4;
    var sw_3 : u32 = _S916 + t_5.x_prev_3;
    var se_3 : u32 = _S916 + t_5.x_next_0;
    grid_corrections_0[d_10] = interpolate_0(t_5, corrections_0[nw_3], corrections_0[ne_3], corrections_0[sw_3], corrections_0[se_3]);
    grid_brightness_0[d_10] = interpolate_1(t_5, brightness_0[nw_3], brightness_0[ne_3], brightness_0[sw_3], brightness_0[se_3]);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn guide_init(@builtin(global_invocation_id) id_13 : vec3<u32>)
{
    var _S917 : u32 = id_13.x;
    var _S918 : vec4<u32> = params_0.ce_0.size_0;
    var _S919 : u32 = params_0.ce_0.size_0.z;
    var _S920 : bool;
    if((_S917 >= _S919))
    {
        _S920 = true;
    }
    else
    {
        _S920 = ((id_13.y) >= (_S918.w));
    }
    if(_S920)
    {
        return;
    }
    var d_11 : u32 = id_13.y * _S919 + _S917;
    var _S921 : vec2<f32> = uv_on_grid_0(id_13.xy);
    var c_6 : f32 = grid_corrections_0[d_11].y;
    var b_6 : f32 = grid_brightness_0[d_11];
    grid_uv_0[d_11] = _S921;
    var _S922 : f32 = _S921.x;
    var _S923 : f32 = _S921.y;
    var _S924 : f32 = _S922 * _S923;
    p1_0[d_11] = vec2<f32>(_S922 * _S922, _S924);
    p2_0[d_11] = vec2<f32>(_S924, _S923 * _S923);
    c1_0[d_11] = vec2<f32>(_S922 * c_6, _S923 * c_6);
    c2_0[d_11] = vec2<f32>(_S922 * b_6, _S923 * b_6);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn guide_coefficients(@builtin(global_invocation_id) id_14 : vec3<u32>)
{
    var _S925 : u32 = id_14.x;
    var _S926 : vec4<u32> = params_0.ce_0.size_0;
    var _S927 : u32 = params_0.ce_0.size_0.z;
    var _S928 : bool;
    if((_S925 >= _S927))
    {
        _S928 = true;
    }
    else
    {
        _S928 = ((id_14.y) >= (_S926.w));
    }
    if(_S928)
    {
        return;
    }
    var d_12 : u32 = id_14.y * _S927 + _S925;
    var mc_0 : f32 = grid_corrections_0[d_12].y;
    var mb_0 : f32 = grid_brightness_0[d_12];
    var _S929 : f32 = grid_uv_0[d_12].x;
    var _S930 : f32 = grid_uv_0[d_12].y;
    var _S931 : f32 = _S929 * _S930;
    var _S932 : f32 = c1_0[d_12].x - _S929 * mc_0;
    var _S933 : f32 = c1_0[d_12].y - _S930 * mc_0;
    var _S934 : f32 = c2_0[d_12].x - _S929 * grid_brightness_0[d_12];
    var _S935 : f32 = c2_0[d_12].y - _S930 * grid_brightness_0[d_12];
    var eps_1 : f32 = params_0.ce_0.shifts_0.z;
    var invertible_2 : bool;
    var inv_1 : vec4<f32> = inverse_of_0(vec4<f32>(p1_0[d_12].x - _S929 * _S929 + eps_1, p1_0[d_12].y - _S931, p2_0[d_12].x - _S931, p2_0[d_12].y - _S930 * _S930 + eps_1), &(invertible_2));
    var _S936 : vec4<f32> = vec4<f32>(0.0f);
    var a_4 : vec4<f32>;
    if(invertible_2)
    {
        var _S937 : f32 = inv_1.x;
        var _S938 : f32 = inv_1.y;
        var _S939 : f32 = inv_1.z;
        var _S940 : f32 = inv_1.w;
        a_4 = vec4<f32>(_S932 * _S937 + _S933 * _S938, _S932 * _S939 + _S933 * _S940, _S934 * _S937 + _S935 * _S938, _S934 * _S939 + _S935 * _S940);
    }
    else
    {
        a_4 = _S936;
    }
    p1_0[d_12] = a_4.xy;
    p2_0[d_12] = a_4.zw;
    c1_0[d_12] = vec2<f32>(mc_0 - a_4.x * _S929 - a_4.y * _S930, mb_0 - a_4.z * _S929 - a_4.w * _S930);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn guide_apply(@builtin(global_invocation_id) id_15 : vec3<u32>)
{
    var _S941 : u32 = id_15.x;
    var _S942 : vec4<u32> = params_0.ce_0.size_0;
    var _S943 : u32 = params_0.ce_0.size_0.x;
    var _S944 : bool;
    if((_S941 >= _S943))
    {
        _S944 = true;
    }
    else
    {
        _S944 = ((id_15.y) >= (_S942.y));
    }
    if(_S944)
    {
        return;
    }
    var k_2 : u32 = id_15.y * _S943 + _S941;
    var _S945 : Coefficients_0 = coefficients_at_0(id_15.xy, k_2);
    var _S946 : f32 = grid_uv_0[k_2].x;
    var _S947 : f32 = grid_uv_0[k_2].y;
    var _S948 : f32 = _S945.a_3.z * _S946 + _S945.a_3.w * _S947 + _S945.b_4.y;
    var w_5 : vec2<f32> = weights_0[k_2];
    corrections_0[k_2] = vec2<f32>(corrections_0[k_2].x, 1.0f + (_S945.a_3.x * _S946 + _S945.a_3.y * _S947 + _S945.b_4.x - 1.0f) * weights_0[k_2].x);
    brightness_0[k_2] = _S948 * (1.0f - clamp(lightness_0[k_2], 0.0f, 1.0f)) * w_5.y;
    return;
}

