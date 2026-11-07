// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2018-2026  darktable developers (ported from src/iop/filmicrgb.c,
// src/common/bspline.h, src/develop/noise_generator.h, src/common/gamut_mapping.h and
// src/common/math.h)
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

// Generated from slang/filmicrgb.slang by tools/build-kernels.sh. Do not edit.

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

struct FilmicParams_std140_0
{
    @align(16) image_0 : vec4<u32>,
    @align(16) wavelet_0 : vec4<u32>,
    @align(16) mask_0 : vec4<f32>,
    @align(16) balance_0 : vec4<f32>,
    @align(16) modes_0 : vec4<u32>,
    @align(16) flags_0 : vec4<u32>,
    @align(16) luminance_0 : vec4<f32>,
    @align(16) to_lms_0 : array<vec4<f32>, i32(3)>,
    @align(16) from_lms_0 : array<vec4<f32>, i32(3)>,
    @align(16) export_to_lms_0 : array<vec4<f32>, i32(3)>,
    @align(16) export_from_lms_0 : array<vec4<f32>, i32(3)>,
    @align(16) curve_0 : array<vec4<f32>, i32(7)>,
    @align(16) directions_0 : array<vec4<f32>, i32(28)>,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) filmic_0 : FilmicParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(0) @group(0) var<storage, read> input_0 : array<vec4<f32>>;

@binding(11) @group(0) var<storage, read_write> counts_0 : array<u32>;

@binding(15) @group(0) var<storage, read_write> reconstructed_0 : array<vec4<f32>>;

@binding(1) @group(0) var<storage, read_write> output_0 : array<vec4<f32>>;

@binding(8) @group(0) var<storage, read> derivative_in_0 : array<vec4<f32>>;

@binding(9) @group(0) var<storage, read_write> derivative_out_0 : array<vec4<f32>>;

@binding(10) @group(0) var<storage, read_write> mask_buffer_0 : array<f32>;

@binding(12) @group(0) var<storage, read_write> detail_buffer_0 : array<vec4<f32>>;

@binding(14) @group(0) var<storage, read_write> temp_buffer_0 : array<vec4<f32>>;

@binding(13) @group(0) var<storage, read_write> low_buffer_0 : array<vec4<f32>>;

@binding(16) @group(0) var<storage, read_write> norm_buffer_0 : array<f32>;

struct Tuple_0
{
     value0_0 : f32,
};

fn s_apply_log2_0( x_0 : f32) -> Tuple_0
{
    var _S1 : Tuple_0 = Tuple_0( log2(x_0) );
    return _S1;
}

struct s_bwdCallableCtx_log2_0
{
     _S2 : f32,
};

struct DiffPair_float_0
{
     primal_0 : f32,
     differential_0 : f32,
};

fn _d_log2_0( dpx_0 : ptr<function, DiffPair_float_0>,  dOut_0 : f32)
{
    var _S3 : f32 = 1.0f / ((*dpx_0).primal_0 * 0.69314718246459961f) * dOut_0;
    (*dpx_0).primal_0 = (*dpx_0).primal_0;
    (*dpx_0).differential_0 = _S3;
    return;
}

fn s_bwdProp_log2_0( ctx_0 : s_bwdCallableCtx_log2_0,  d_x_0 : ptr<function, f32>,  d_out_0 : f32)
{
    var _S4 : DiffPair_float_0;
    _S4.primal_0 = ctx_0._S2;
    _d_log2_0(&(_S4), d_out_0);
    (*d_x_0) = _S4.differential_0;
    return;
}

struct Tuple_1
{
     value0_1 : f32,
};

fn s_apply_length_0( x_1 : vec2<f32>) -> Tuple_1
{
    var _S5 : Tuple_1 = Tuple_1( length(x_1) );
    return _S5;
}

struct s_bwdCallableCtx_length_0
{
     _S6 : vec2<f32>,
};

struct DiffPair_vectorx3Cfloatx2C2x3E_0
{
     primal_0 : vec2<f32>,
     differential_0 : vec2<f32>,
};

struct s_paramCtx_s_bwdCallableCtx_length_impl_0
{
     value0_2 : vec2<f32>,
};

struct s_bwdCallableCtx_length_impl_0
{
     value0_3 : s_paramCtx_s_bwdCallableCtx_length_impl_0,
};

struct s_bwdCallableCtx_sqrt_0
{
     _S7 : f32,
};

fn _d_sqrt_0( dpx_1 : ptr<function, DiffPair_float_0>,  dOut_1 : f32)
{
    var _S8 : f32 = 0.5f / sqrt(max(1.00000001168609742e-07f, (*dpx_1).primal_0)) * dOut_1;
    (*dpx_1).primal_0 = (*dpx_1).primal_0;
    (*dpx_1).differential_0 = _S8;
    return;
}

fn s_bwdProp_sqrt_0( ctx_1 : s_bwdCallableCtx_sqrt_0,  d_x_1 : ptr<function, f32>,  d_out_1 : f32)
{
    var _S9 : DiffPair_float_0;
    _S9.primal_0 = ctx_1._S7;
    _d_sqrt_0(&(_S9), d_out_1);
    (*d_x_1) = _S9.differential_0;
    return;
}

fn s_bwdProp_length_impl_0( _S10 : s_bwdCallableCtx_length_impl_0,  _S11 : ptr<function, vec2<f32>>,  _S12 : f32)
{
    var _S13 : f32 = _S10.value0_3.value0_2[i32(0)];
    var _S14 : f32 = _S10.value0_3.value0_2[i32(1)];
    var _S15 : s_bwdCallableCtx_sqrt_0;
    _S15._S7 = _S13 * _S13 + _S14 * _S14;
    var _S16 : f32 = 0.0f;
    s_bwdProp_sqrt_0(_S15, &(_S16), _S12);
    var _S17 : f32 = _S10.value0_3.value0_2[i32(1)] * _S16;
    var _S18 : f32 = _S17 + _S17;
    var _S19 : f32 = _S10.value0_3.value0_2[i32(0)] * _S16;
    var _S20 : f32 = _S19 + _S19;
    var _S21 : vec2<f32> = vec2<f32>(0.0f);
    _S21[i32(1)] = _S18;
    _S21[i32(0)] = _S20;
    (*_S11) = _S21;
    return;
}

fn _S22( _S23 : ptr<function, DiffPair_vectorx3Cfloatx2C2x3E_0>,  _S24 : f32)
{
    var _S25 : s_paramCtx_s_bwdCallableCtx_length_impl_0;
    _S25.value0_2 = (*_S23).primal_0;
    var _S26 : s_bwdCallableCtx_length_impl_0 = s_bwdCallableCtx_length_impl_0( _S25 );
    var _S27 : vec2<f32> = (*_S23).differential_0;
    s_bwdProp_length_impl_0(_S26, &(_S27), _S24);
    (*_S23).differential_0 = _S27;
    return;
}

fn s_bwdProp_length_0( ctx_2 : s_bwdCallableCtx_length_0,  d_x_2 : ptr<function, vec2<f32>>,  d_out_2 : f32)
{
    var _S28 : DiffPair_vectorx3Cfloatx2C2x3E_0;
    _S28.primal_0 = ctx_2._S6;
    _S22(&(_S28), d_out_2);
    (*d_x_2) = _S28.differential_0;
    return;
}

struct Tuple_2
{
     value0_4 : vec3<f32>,
};

fn s_apply_abs_0( x_2 : vec3<f32>) -> Tuple_2
{
    var _S29 : Tuple_2 = Tuple_2( abs(x_2) );
    return _S29;
}

struct s_bwdCallableCtx_abs_0
{
     _S30 : vec3<f32>,
};

struct DiffPair_vectorx3Cfloatx2C3x3E_0
{
     primal_0 : vec3<f32>,
     differential_0 : vec3<f32>,
};

fn _d_abs_vector_0( dpx_2 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_2 : vec3<f32>)
{
    var _S31 : vec3<f32> = vec3<f32>(0.0f);
    var _S32 : vec3<f32> = select(select(vec3<f32>(-1.0f), _S31, ((*dpx_2).primal_0) == _S31), vec3<f32>(1.0f), (((*dpx_2).primal_0) > _S31)) * dOut_2;
    (*dpx_2).primal_0 = (*dpx_2).primal_0;
    (*dpx_2).differential_0 = _S32;
    return;
}

fn s_bwdProp_abs_0( ctx_3 : s_bwdCallableCtx_abs_0,  d_x_3 : ptr<function, vec3<f32>>,  d_out_3 : vec3<f32>)
{
    var _S33 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S33.primal_0 = ctx_3._S30;
    _d_abs_vector_0(&(_S33), d_out_3);
    (*d_x_3) = _S33.differential_0;
    return;
}

struct Tuple_3
{
     value0_5 : f32,
};

fn s_apply_sqrt_0( x_3 : f32) -> Tuple_3
{
    var _S34 : Tuple_3 = Tuple_3( sqrt(x_3) );
    return _S34;
}

struct Tuple_4
{
     value0_6 : f32,
};

fn s_apply_exp_0( x_4 : f32) -> Tuple_4
{
    var _S35 : Tuple_4 = Tuple_4( exp(x_4) );
    return _S35;
}

struct s_bwdCallableCtx_exp_0
{
     _S36 : f32,
};

fn _d_exp_0( dpx_3 : ptr<function, DiffPair_float_0>,  dOut_3 : f32)
{
    var _S37 : f32 = exp((*dpx_3).primal_0) * dOut_3;
    (*dpx_3).primal_0 = (*dpx_3).primal_0;
    (*dpx_3).differential_0 = _S37;
    return;
}

fn s_bwdProp_exp_0( ctx_4 : s_bwdCallableCtx_exp_0,  d_x_4 : ptr<function, f32>,  d_out_4 : f32)
{
    var _S38 : DiffPair_float_0;
    _S38.primal_0 = ctx_4._S36;
    _d_exp_0(&(_S38), d_out_4);
    (*d_x_4) = _S38.differential_0;
    return;
}

struct Tuple_5
{
     value0_7 : vec3<f32>,
};

fn s_apply_mul_0( left_0 : mat3x3<f32>,  right_0 : vec3<f32>) -> Tuple_5
{
    var _S39 : Tuple_5 = Tuple_5( (((right_0) * (left_0))) );
    return _S39;
}

struct s_bwdCallableCtx_mul_0
{
     _S40 : mat3x3<f32>,
     _S41 : vec3<f32>,
};

struct DiffPair_matrixx3Cfloatx2C3x2C3x3E_0
{
     primal_0 : mat3x3<f32>,
     differential_0 : mat3x3<f32>,
};

fn _d_mul_0( left_1 : ptr<function, DiffPair_matrixx3Cfloatx2C3x2C3x3E_0>,  right_1 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_4 : vec3<f32>)
{
    var _S42 : f32 = (*left_1).primal_0[i32(0)][i32(0)] * dOut_4[i32(0)];
    var left_d_result_0 : mat3x3<f32>;
    left_d_result_0[i32(0)][i32(0)] = (*right_1).primal_0[i32(0)] * dOut_4[i32(0)];
    var sum_0 : f32 = _S42 + (*left_1).primal_0[i32(1)][i32(0)] * dOut_4[i32(1)];
    left_d_result_0[i32(1)][i32(0)] = (*right_1).primal_0[i32(0)] * dOut_4[i32(1)];
    var sum_1 : f32 = sum_0 + (*left_1).primal_0[i32(2)][i32(0)] * dOut_4[i32(2)];
    left_d_result_0[i32(2)][i32(0)] = (*right_1).primal_0[i32(0)] * dOut_4[i32(2)];
    var right_d_result_0 : vec3<f32>;
    right_d_result_0[i32(0)] = sum_1;
    var _S43 : f32 = (*left_1).primal_0[i32(0)][i32(1)] * dOut_4[i32(0)];
    left_d_result_0[i32(0)][i32(1)] = (*right_1).primal_0[i32(1)] * dOut_4[i32(0)];
    var sum_2 : f32 = _S43 + (*left_1).primal_0[i32(1)][i32(1)] * dOut_4[i32(1)];
    left_d_result_0[i32(1)][i32(1)] = (*right_1).primal_0[i32(1)] * dOut_4[i32(1)];
    var sum_3 : f32 = sum_2 + (*left_1).primal_0[i32(2)][i32(1)] * dOut_4[i32(2)];
    left_d_result_0[i32(2)][i32(1)] = (*right_1).primal_0[i32(1)] * dOut_4[i32(2)];
    right_d_result_0[i32(1)] = sum_3;
    var _S44 : f32 = (*left_1).primal_0[i32(0)][i32(2)] * dOut_4[i32(0)];
    left_d_result_0[i32(0)][i32(2)] = (*right_1).primal_0[i32(2)] * dOut_4[i32(0)];
    var sum_4 : f32 = _S44 + (*left_1).primal_0[i32(1)][i32(2)] * dOut_4[i32(1)];
    left_d_result_0[i32(1)][i32(2)] = (*right_1).primal_0[i32(2)] * dOut_4[i32(1)];
    var sum_5 : f32 = sum_4 + (*left_1).primal_0[i32(2)][i32(2)] * dOut_4[i32(2)];
    left_d_result_0[i32(2)][i32(2)] = (*right_1).primal_0[i32(2)] * dOut_4[i32(2)];
    right_d_result_0[i32(2)] = sum_5;
    (*left_1).primal_0 = (*left_1).primal_0;
    (*left_1).differential_0 = left_d_result_0;
    (*right_1).primal_0 = (*right_1).primal_0;
    (*right_1).differential_0 = right_d_result_0;
    return;
}

fn s_bwdProp_mul_0( ctx_5 : s_bwdCallableCtx_mul_0,  d_left_0 : ptr<function, mat3x3<f32>>,  d_right_0 : ptr<function, vec3<f32>>,  d_out_5 : vec3<f32>)
{
    var _S45 : DiffPair_matrixx3Cfloatx2C3x2C3x3E_0;
    _S45.primal_0 = ctx_5._S40;
    var _S46 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S46.primal_0 = ctx_5._S41;
    _d_mul_0(&(_S45), &(_S46), d_out_5);
    (*d_left_0) = _S45.differential_0;
    (*d_right_0) = _S46.differential_0;
    return;
}

struct Tuple_6
{
     value0_8 : f32,
};

fn s_apply_abs_1( x_5 : f32) -> Tuple_6
{
    var _S47 : Tuple_6 = Tuple_6( abs(x_5) );
    return _S47;
}

struct s_bwdCallableCtx_abs_1
{
     _S48 : f32,
};

fn _d_abs_0( dpx_4 : ptr<function, DiffPair_float_0>,  dOut_5 : f32)
{
    var _S49 : f32 = select(select(-1.0f, 0.0f, ((*dpx_4).primal_0) == 0.0f), 1.0f, (((*dpx_4).primal_0) > 0.0f)) * dOut_5;
    (*dpx_4).primal_0 = (*dpx_4).primal_0;
    (*dpx_4).differential_0 = _S49;
    return;
}

fn s_bwdProp_abs_1( ctx_6 : s_bwdCallableCtx_abs_1,  d_x_5 : ptr<function, f32>,  d_out_6 : f32)
{
    var _S50 : DiffPair_float_0;
    _S50.primal_0 = ctx_6._S48;
    _d_abs_0(&(_S50), d_out_6);
    (*d_x_5) = _S50.differential_0;
    return;
}

struct Tuple_7
{
     value0_9 : vec3<f32>,
};

fn s_apply_max_0( x_6 : vec3<f32>,  y_0 : vec3<f32>) -> Tuple_7
{
    var _S51 : Tuple_7 = Tuple_7( max(x_6, y_0) );
    return _S51;
}

struct s_bwdCallableCtx_max_0
{
     _S52 : vec3<f32>,
     _S53 : vec3<f32>,
};

fn _d_max_0( dpx_5 : ptr<function, DiffPair_float_0>,  dpy_0 : ptr<function, DiffPair_float_0>,  dOut_6 : f32)
{
    var _S54 : DiffPair_float_0 = (*dpx_5);
    var _S55 : f32;
    if((((*dpx_5).primal_0) > ((*dpy_0).primal_0)))
    {
        _S55 = dOut_6;
    }
    else
    {
        if((((*dpx_5).primal_0) < ((*dpy_0).primal_0)))
        {
            _S55 = 0.0f;
        }
        else
        {
            _S55 = 0.5f * dOut_6;
        }
    }
    (*dpx_5).primal_0 = _S54.primal_0;
    (*dpx_5).differential_0 = _S55;
    var _S56 : DiffPair_float_0 = (*dpy_0);
    if((((*dpy_0).primal_0) > (_S54.primal_0)))
    {
        _S55 = dOut_6;
    }
    else
    {
        if((((*dpy_0).primal_0) < ((*dpx_5).primal_0)))
        {
            _S55 = 0.0f;
        }
        else
        {
            _S55 = 0.5f * dOut_6;
        }
    }
    (*dpy_0).primal_0 = _S56.primal_0;
    (*dpy_0).differential_0 = _S55;
    return;
}

fn _d_max_vector_0( dpx_6 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dpy_1 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_7 : vec3<f32>)
{
    var left_dp_0 : DiffPair_float_0;
    left_dp_0.primal_0 = (*dpx_6).primal_0[i32(0)];
    left_dp_0.differential_0 = 0.0f;
    var right_dp_0 : DiffPair_float_0;
    right_dp_0.primal_0 = (*dpy_1).primal_0[i32(0)];
    right_dp_0.differential_0 = 0.0f;
    _d_max_0(&(left_dp_0), &(right_dp_0), dOut_7[i32(0)]);
    var left_d_result_1 : vec3<f32>;
    left_d_result_1[i32(0)] = left_dp_0.differential_0;
    var right_d_result_1 : vec3<f32>;
    right_d_result_1[i32(0)] = right_dp_0.differential_0;
    var left_dp_1 : DiffPair_float_0;
    left_dp_1.primal_0 = (*dpx_6).primal_0[i32(1)];
    left_dp_1.differential_0 = 0.0f;
    var right_dp_1 : DiffPair_float_0;
    right_dp_1.primal_0 = (*dpy_1).primal_0[i32(1)];
    right_dp_1.differential_0 = 0.0f;
    _d_max_0(&(left_dp_1), &(right_dp_1), dOut_7[i32(1)]);
    left_d_result_1[i32(1)] = left_dp_1.differential_0;
    right_d_result_1[i32(1)] = right_dp_1.differential_0;
    var left_dp_2 : DiffPair_float_0;
    left_dp_2.primal_0 = (*dpx_6).primal_0[i32(2)];
    left_dp_2.differential_0 = 0.0f;
    var right_dp_2 : DiffPair_float_0;
    right_dp_2.primal_0 = (*dpy_1).primal_0[i32(2)];
    right_dp_2.differential_0 = 0.0f;
    _d_max_0(&(left_dp_2), &(right_dp_2), dOut_7[i32(2)]);
    left_d_result_1[i32(2)] = left_dp_2.differential_0;
    right_d_result_1[i32(2)] = right_dp_2.differential_0;
    (*dpx_6).primal_0 = (*dpx_6).primal_0;
    (*dpx_6).differential_0 = left_d_result_1;
    (*dpy_1).primal_0 = (*dpy_1).primal_0;
    (*dpy_1).differential_0 = right_d_result_1;
    return;
}

fn s_bwdProp_max_0( ctx_7 : s_bwdCallableCtx_max_0,  d_x_6 : ptr<function, vec3<f32>>,  d_y_0 : ptr<function, vec3<f32>>,  d_out_7 : vec3<f32>)
{
    var _S57 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S57.primal_0 = ctx_7._S52;
    var _S58 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S58.primal_0 = ctx_7._S53;
    _d_max_vector_0(&(_S57), &(_S58), d_out_7);
    (*d_x_6) = _S57.differential_0;
    (*d_y_0) = _S58.differential_0;
    return;
}

struct Tuple_8
{
     value0_10 : f32,
};

fn pow0_0( x_7 : f32,  y_1 : f32) -> f32
{
    if((x_7 <= 0.0f))
    {
        return 0.0f;
    }
    return pow(x_7, y_1);
}

fn s_apply_pow0_0( x_8 : f32,  y_2 : f32) -> Tuple_8
{
    var _S59 : Tuple_8 = Tuple_8( pow0_0(x_8, y_2) );
    return _S59;
}

struct s_bwdCallableCtx_pow0_0
{
     _S60 : f32,
     _S61 : f32,
};

fn pow0_backward_0( x_9 : ptr<function, DiffPair_float_0>,  y_3 : ptr<function, DiffPair_float_0>,  d_out_8 : f32)
{
    if((((*x_9).primal_0) <= 0.0f))
    {
        (*x_9).primal_0 = (*x_9).primal_0;
        (*x_9).differential_0 = 0.0f;
        (*y_3).primal_0 = (*y_3).primal_0;
        (*y_3).differential_0 = 0.0f;
        return;
    }
    var d_x_7 : f32 = d_out_8 * (*y_3).primal_0 * pow((*x_9).primal_0, (*y_3).primal_0 - 1.0f);
    var d_y_1 : f32 = d_out_8 * pow((*x_9).primal_0, (*y_3).primal_0) * log((*x_9).primal_0);
    (*x_9).primal_0 = (*x_9).primal_0;
    (*x_9).differential_0 = d_x_7;
    (*y_3).primal_0 = (*y_3).primal_0;
    (*y_3).differential_0 = d_y_1;
    return;
}

fn s_bwdProp_pow0_0( ctx_8 : s_bwdCallableCtx_pow0_0,  d_x_8 : ptr<function, f32>,  d_y_2 : ptr<function, f32>,  d_out_9 : f32)
{
    var _S62 : DiffPair_float_0;
    _S62.primal_0 = ctx_8._S60;
    var _S63 : DiffPair_float_0;
    _S63.primal_0 = ctx_8._S61;
    pow0_backward_0(&(_S62), &(_S63), d_out_9);
    (*d_x_8) = _S62.differential_0;
    (*d_y_2) = _S63.differential_0;
    return;
}

struct Tuple_9
{
     value0_11 : f32,
};

fn s_apply_clamp_0( x_10 : f32,  minBound_0 : f32,  maxBound_0 : f32) -> Tuple_9
{
    var _S64 : Tuple_9 = Tuple_9( clamp(x_10, minBound_0, maxBound_0) );
    return _S64;
}

struct s_bwdCallableCtx_clamp_0
{
     _S65 : f32,
     _S66 : f32,
     _S67 : f32,
};

fn _d_clamp_0( dpx_7 : ptr<function, DiffPair_float_0>,  dpMin_0 : ptr<function, DiffPair_float_0>,  dpMax_0 : ptr<function, DiffPair_float_0>,  dOut_8 : f32)
{
    var _S68 : DiffPair_float_0 = (*dpx_7);
    var _S69 : bool;
    if((((*dpx_7).primal_0) >= ((*dpMin_0).primal_0)))
    {
        _S69 = (((*dpx_7).primal_0) <= ((*dpMax_0).primal_0));
    }
    else
    {
        _S69 = false;
    }
    var _S70 : f32;
    if(_S69)
    {
        _S70 = dOut_8;
    }
    else
    {
        _S70 = 0.0f;
    }
    (*dpx_7).primal_0 = _S68.primal_0;
    (*dpx_7).differential_0 = _S70;
    var _S71 : DiffPair_float_0 = (*dpMin_0);
    if(((_S68.primal_0) < ((*dpMin_0).primal_0)))
    {
        _S70 = dOut_8;
    }
    else
    {
        _S70 = 0.0f;
    }
    (*dpMin_0).primal_0 = _S71.primal_0;
    (*dpMin_0).differential_0 = _S70;
    var _S72 : DiffPair_float_0 = (*dpMax_0);
    if((((*dpx_7).primal_0) > ((*dpMax_0).primal_0)))
    {
        _S70 = dOut_8;
    }
    else
    {
        _S70 = 0.0f;
    }
    (*dpMax_0).primal_0 = _S72.primal_0;
    (*dpMax_0).differential_0 = _S70;
    return;
}

fn s_bwdProp_clamp_0( ctx_9 : s_bwdCallableCtx_clamp_0,  d_x_9 : ptr<function, f32>,  d_minBound_0 : ptr<function, f32>,  d_maxBound_0 : ptr<function, f32>,  d_out_10 : f32)
{
    var _S73 : DiffPair_float_0;
    _S73.primal_0 = ctx_9._S65;
    var _S74 : DiffPair_float_0;
    _S74.primal_0 = ctx_9._S66;
    var _S75 : DiffPair_float_0;
    _S75.primal_0 = ctx_9._S67;
    _d_clamp_0(&(_S73), &(_S74), &(_S75), d_out_10);
    (*d_x_9) = _S73.differential_0;
    (*d_minBound_0) = _S74.differential_0;
    (*d_maxBound_0) = _S75.differential_0;
    return;
}

struct Tuple_10
{
     value0_12 : f32,
};

fn log2_dt_0( x_11 : f32) -> f32
{
    var bits_0 : u32 = (bitcast<u32>((x_11)));
    var mantissa_0 : f32 = (bitcast<f32>(((((bits_0 & (u32(8388607)))) | (u32(1065353216))))));
    return ((((0.059651549905538559f * mantissa_0 - 0.46572563052177429f) * mantissa_0 + 1.4811664819717407f) * mantissa_0 - 2.520749568939209f) * mantissa_0 + 2.888270378112793f) * (mantissa_0 - 1.0f) + (f32((((bits_0 & (u32(2139095040)))) >> (u32(23)))) - 127.0f);
}

fn round_away_0( x_12 : f32) -> f32
{
    var t_0 : f32 = trunc(x_12);
    var t_1 : f32;
    if(((abs(x_12 - t_0)) >= 0.5f))
    {
        if((x_12 < 0.0f))
        {
            t_1 = -1.0f;
        }
        else
        {
            t_1 = 1.0f;
        }
        t_1 = t_0 + t_1;
    }
    else
    {
        t_1 = t_0;
    }
    return t_1;
}

fn exp2_dt_0( y_4 : f32,  slope_0 : ptr<function, f32>) -> f32
{
    var x_13 : f32 = clamp(y_4, -126.99999237060547f, 129.0f);
    var ipart_0 : f32 = round_away_0(x_13 - 0.5f);
    var fpart_0 : f32 = x_13 - ipart_0;
    var power_0 : f32 = (bitcast<f32>(((u32(i32(127) + i32(ipart_0)) << (u32(23))))));
    var fit_0 : f32 = (((0.013534166850149632f * fpart_0 + 0.052011463791131973f) * fpart_0 + 0.24144275486469269f) * fpart_0 + 0.69300383329391479f) * fpart_0 + 1.0000026226043701f;
    var fit_slope_0 : f32 = ((0.054136667400598526f * fpart_0 + 0.15603439509868622f) * fpart_0 + 0.48288550972938538f) * fpart_0 + 0.69300383329391479f;
    var _S76 : bool;
    if((y_4 > -126.99999237060547f))
    {
        _S76 = (y_4 < 129.0f);
    }
    else
    {
        _S76 = false;
    }
    var _S77 : f32;
    if(_S76)
    {
        _S77 = power_0 * fit_slope_0;
    }
    else
    {
        _S77 = 0.0f;
    }
    (*slope_0) = _S77;
    return power_0 * fit_0;
}

fn pow_dt_0( x_14 : f32,  p_0 : f32) -> f32
{
    var slope_1 : f32;
    return exp2_dt_0(log2_dt_0(x_14) * p_0, &(slope_1));
}

fn s_apply_pow_dt_0( x_15 : f32,  p_1 : f32) -> Tuple_10
{
    var _S78 : Tuple_10 = Tuple_10( pow_dt_0(x_15, p_1) );
    return _S78;
}

struct s_bwdCallableCtx_pow_dt_0
{
     _S79 : f32,
     _S80 : f32,
};

fn exp2_dt_1( y_5 : f32,  slope_2 : ptr<function, f32>) -> f32
{
    var x_16 : f32 = clamp(y_5, -126.99999237060547f, 129.0f);
    var ipart_1 : f32 = round_away_0(x_16 - 0.5f);
    var fpart_1 : f32 = x_16 - ipart_1;
    var power_1 : f32 = (bitcast<f32>(((u32(i32(127) + i32(ipart_1)) << (u32(23))))));
    var fit_1 : f32 = (((0.013534166850149632f * fpart_1 + 0.052011463791131973f) * fpart_1 + 0.24144275486469269f) * fpart_1 + 0.69300383329391479f) * fpart_1 + 1.0000026226043701f;
    var fit_slope_1 : f32 = ((0.054136667400598526f * fpart_1 + 0.15603439509868622f) * fpart_1 + 0.48288550972938538f) * fpart_1 + 0.69300383329391479f;
    var _S81 : bool;
    if((y_5 > -126.99999237060547f))
    {
        _S81 = (y_5 < 129.0f);
    }
    else
    {
        _S81 = false;
    }
    var _S82 : f32;
    if(_S81)
    {
        _S82 = power_1 * fit_slope_1;
    }
    else
    {
        _S82 = 0.0f;
    }
    (*slope_2) = _S82;
    return power_1 * fit_1;
}

fn log2_dt_slope_0( x_17 : f32) -> f32
{
    if((x_17 <= 0.0f))
    {
        return 0.0f;
    }
    var mantissa_1 : f32 = (bitcast<f32>((((((bitcast<u32>((x_17))) & (u32(8388607)))) | (u32(1065353216))))));
    return ((((0.23860619962215424f * mantissa_1 - 1.3971768617630005f) * mantissa_1 + 2.9623329639434814f) * mantissa_1 - 2.520749568939209f) * (mantissa_1 - 1.0f) + ((((0.059651549905538559f * mantissa_1 - 0.46572563052177429f) * mantissa_1 + 1.4811664819717407f) * mantissa_1 - 2.520749568939209f) * mantissa_1 + 2.888270378112793f)) * (mantissa_1 / x_17);
}

fn pow_dt_backward_0( x_18 : ptr<function, DiffPair_float_0>,  p_2 : ptr<function, DiffPair_float_0>,  d_out_11 : f32)
{
    var log_0 : f32 = log2_dt_0((*x_18).primal_0);
    var slope_3 : f32;
    var _S83 : f32 = exp2_dt_1(log_0 * (*p_2).primal_0, &(slope_3));
    var _S84 : f32 = d_out_11 * slope_3 * (*p_2).primal_0 * log2_dt_slope_0((*x_18).primal_0);
    (*x_18).primal_0 = (*x_18).primal_0;
    (*x_18).differential_0 = _S84;
    var _S85 : f32 = d_out_11 * slope_3 * log_0;
    (*p_2).primal_0 = (*p_2).primal_0;
    (*p_2).differential_0 = _S85;
    return;
}

fn s_bwdProp_pow_dt_0( ctx_10 : s_bwdCallableCtx_pow_dt_0,  d_x_10 : ptr<function, f32>,  d_p_0 : ptr<function, f32>,  d_out_12 : f32)
{
    var _S86 : DiffPair_float_0;
    _S86.primal_0 = ctx_10._S79;
    var _S87 : DiffPair_float_0;
    _S87.primal_0 = ctx_10._S80;
    pow_dt_backward_0(&(_S86), &(_S87), d_out_12);
    (*d_x_10) = _S86.differential_0;
    (*d_p_0) = _S87.differential_0;
    return;
}

struct Tuple_11
{
     value0_13 : vec3<f32>,
};

fn s_apply_clamp_1( x_19 : vec3<f32>,  minBound_1 : vec3<f32>,  maxBound_1 : vec3<f32>) -> Tuple_11
{
    var _S88 : Tuple_11 = Tuple_11( clamp(x_19, minBound_1, maxBound_1) );
    return _S88;
}

struct s_bwdCallableCtx_clamp_1
{
     _S89 : vec3<f32>,
     _S90 : vec3<f32>,
     _S91 : vec3<f32>,
};

fn _d_clamp_vector_0( dpx_8 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dpy_2 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dpz_0 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_9 : vec3<f32>)
{
    var left_dp_3 : DiffPair_float_0;
    left_dp_3.primal_0 = (*dpx_8).primal_0[i32(0)];
    left_dp_3.differential_0 = 0.0f;
    var middle_dp_0 : DiffPair_float_0;
    middle_dp_0.primal_0 = (*dpy_2).primal_0[i32(0)];
    middle_dp_0.differential_0 = 0.0f;
    var right_dp_3 : DiffPair_float_0;
    right_dp_3.primal_0 = (*dpz_0).primal_0[i32(0)];
    right_dp_3.differential_0 = 0.0f;
    _d_clamp_0(&(left_dp_3), &(middle_dp_0), &(right_dp_3), dOut_9[i32(0)]);
    var left_d_result_2 : vec3<f32>;
    left_d_result_2[i32(0)] = left_dp_3.differential_0;
    var middle_d_result_0 : vec3<f32>;
    middle_d_result_0[i32(0)] = middle_dp_0.differential_0;
    var right_d_result_2 : vec3<f32>;
    right_d_result_2[i32(0)] = right_dp_3.differential_0;
    var left_dp_4 : DiffPair_float_0;
    left_dp_4.primal_0 = (*dpx_8).primal_0[i32(1)];
    left_dp_4.differential_0 = 0.0f;
    var middle_dp_1 : DiffPair_float_0;
    middle_dp_1.primal_0 = (*dpy_2).primal_0[i32(1)];
    middle_dp_1.differential_0 = 0.0f;
    var right_dp_4 : DiffPair_float_0;
    right_dp_4.primal_0 = (*dpz_0).primal_0[i32(1)];
    right_dp_4.differential_0 = 0.0f;
    _d_clamp_0(&(left_dp_4), &(middle_dp_1), &(right_dp_4), dOut_9[i32(1)]);
    left_d_result_2[i32(1)] = left_dp_4.differential_0;
    middle_d_result_0[i32(1)] = middle_dp_1.differential_0;
    right_d_result_2[i32(1)] = right_dp_4.differential_0;
    var left_dp_5 : DiffPair_float_0;
    left_dp_5.primal_0 = (*dpx_8).primal_0[i32(2)];
    left_dp_5.differential_0 = 0.0f;
    var middle_dp_2 : DiffPair_float_0;
    middle_dp_2.primal_0 = (*dpy_2).primal_0[i32(2)];
    middle_dp_2.differential_0 = 0.0f;
    var right_dp_5 : DiffPair_float_0;
    right_dp_5.primal_0 = (*dpz_0).primal_0[i32(2)];
    right_dp_5.differential_0 = 0.0f;
    _d_clamp_0(&(left_dp_5), &(middle_dp_2), &(right_dp_5), dOut_9[i32(2)]);
    left_d_result_2[i32(2)] = left_dp_5.differential_0;
    middle_d_result_0[i32(2)] = middle_dp_2.differential_0;
    right_d_result_2[i32(2)] = right_dp_5.differential_0;
    (*dpx_8).primal_0 = (*dpx_8).primal_0;
    (*dpx_8).differential_0 = left_d_result_2;
    (*dpy_2).primal_0 = (*dpy_2).primal_0;
    (*dpy_2).differential_0 = middle_d_result_0;
    (*dpz_0).primal_0 = (*dpz_0).primal_0;
    (*dpz_0).differential_0 = right_d_result_2;
    return;
}

fn s_bwdProp_clamp_1( ctx_11 : s_bwdCallableCtx_clamp_1,  d_x_11 : ptr<function, vec3<f32>>,  d_minBound_1 : ptr<function, vec3<f32>>,  d_maxBound_1 : ptr<function, vec3<f32>>,  d_out_13 : vec3<f32>)
{
    var _S92 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S92.primal_0 = ctx_11._S89;
    var _S93 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S93.primal_0 = ctx_11._S90;
    var _S94 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S94.primal_0 = ctx_11._S91;
    _d_clamp_vector_0(&(_S92), &(_S93), &(_S94), d_out_13);
    (*d_x_11) = _S92.differential_0;
    (*d_minBound_1) = _S93.differential_0;
    (*d_maxBound_1) = _S94.differential_0;
    return;
}

struct Tuple_12
{
     value0_14 : f32,
};

fn s_apply_dot_0( x_20 : vec3<f32>,  y_6 : vec3<f32>) -> Tuple_12
{
    var _S95 : Tuple_12 = Tuple_12( dot(x_20, y_6) );
    return _S95;
}

struct s_bwdCallableCtx_dot_0
{
     _S96 : vec3<f32>,
     _S97 : vec3<f32>,
};

fn _d_dot_0( dpx_9 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dpy_3 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_10 : f32)
{
    var x_d_result_0 : vec3<f32>;
    x_d_result_0[i32(0)] = (*dpy_3).primal_0[i32(0)] * dOut_10;
    var y_d_result_0 : vec3<f32>;
    y_d_result_0[i32(0)] = (*dpx_9).primal_0[i32(0)] * dOut_10;
    x_d_result_0[i32(1)] = (*dpy_3).primal_0[i32(1)] * dOut_10;
    y_d_result_0[i32(1)] = (*dpx_9).primal_0[i32(1)] * dOut_10;
    x_d_result_0[i32(2)] = (*dpy_3).primal_0[i32(2)] * dOut_10;
    y_d_result_0[i32(2)] = (*dpx_9).primal_0[i32(2)] * dOut_10;
    (*dpx_9).primal_0 = (*dpx_9).primal_0;
    (*dpx_9).differential_0 = x_d_result_0;
    (*dpy_3).primal_0 = (*dpy_3).primal_0;
    (*dpy_3).differential_0 = y_d_result_0;
    return;
}

fn s_bwdProp_dot_0( ctx_12 : s_bwdCallableCtx_dot_0,  d_x_12 : ptr<function, vec3<f32>>,  d_y_3 : ptr<function, vec3<f32>>,  d_out_14 : f32)
{
    var _S98 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S98.primal_0 = ctx_12._S96;
    var _S99 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S99.primal_0 = ctx_12._S97;
    _d_dot_0(&(_S98), &(_S99), d_out_14);
    (*d_x_12) = _S98.differential_0;
    (*d_y_3) = _S99.differential_0;
    return;
}

struct Tuple_13
{
     value0_15 : f32,
};

fn s_apply_max_1( x_21 : f32,  y_7 : f32) -> Tuple_13
{
    var _S100 : Tuple_13 = Tuple_13( max(x_21, y_7) );
    return _S100;
}

struct s_bwdCallableCtx_max_1
{
     _S101 : f32,
     _S102 : f32,
};

fn s_bwdProp_max_1( ctx_13 : s_bwdCallableCtx_max_1,  d_x_13 : ptr<function, f32>,  d_y_4 : ptr<function, f32>,  d_out_15 : f32)
{
    var _S103 : DiffPair_float_0;
    _S103.primal_0 = ctx_13._S101;
    var _S104 : DiffPair_float_0;
    _S104.primal_0 = ctx_13._S102;
    _d_max_0(&(_S103), &(_S104), d_out_15);
    (*d_x_13) = _S103.differential_0;
    (*d_y_4) = _S104.differential_0;
    return;
}

struct Tuple_14
{
     value0_16 : f32,
};

fn s_apply_min_0( x_22 : f32,  y_8 : f32) -> Tuple_14
{
    var _S105 : Tuple_14 = Tuple_14( min(x_22, y_8) );
    return _S105;
}

struct s_bwdCallableCtx_min_0
{
     _S106 : f32,
     _S107 : f32,
};

fn _d_min_0( dpx_10 : ptr<function, DiffPair_float_0>,  dpy_4 : ptr<function, DiffPair_float_0>,  dOut_11 : f32)
{
    var _S108 : DiffPair_float_0 = (*dpx_10);
    var _S109 : f32;
    if((((*dpx_10).primal_0) < ((*dpy_4).primal_0)))
    {
        _S109 = dOut_11;
    }
    else
    {
        if((((*dpx_10).primal_0) > ((*dpy_4).primal_0)))
        {
            _S109 = 0.0f;
        }
        else
        {
            _S109 = 0.5f * dOut_11;
        }
    }
    (*dpx_10).primal_0 = _S108.primal_0;
    (*dpx_10).differential_0 = _S109;
    var _S110 : DiffPair_float_0 = (*dpy_4);
    if((((*dpy_4).primal_0) < (_S108.primal_0)))
    {
        _S109 = dOut_11;
    }
    else
    {
        if((((*dpy_4).primal_0) > ((*dpx_10).primal_0)))
        {
            _S109 = 0.0f;
        }
        else
        {
            _S109 = 0.5f * dOut_11;
        }
    }
    (*dpy_4).primal_0 = _S110.primal_0;
    (*dpy_4).differential_0 = _S109;
    return;
}

fn s_bwdProp_min_0( ctx_14 : s_bwdCallableCtx_min_0,  d_x_14 : ptr<function, f32>,  d_y_5 : ptr<function, f32>,  d_out_16 : f32)
{
    var _S111 : DiffPair_float_0;
    _S111.primal_0 = ctx_14._S106;
    var _S112 : DiffPair_float_0;
    _S112.primal_0 = ctx_14._S107;
    _d_min_0(&(_S111), &(_S112), d_out_16);
    (*d_x_14) = _S111.differential_0;
    (*d_y_5) = _S112.differential_0;
    return;
}

fn _d_sqrt_1( dpx_11 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S113 : DiffPair_float_0 = DiffPair_float_0( sqrt(dpx_11.primal_0), 0.5f / sqrt(max(1.00000001168609742e-07f, dpx_11.primal_0)) * dpx_11.differential_0 );
    return _S113;
}

fn s_fwd_length_impl_0( dpx_12 : DiffPair_vectorx3Cfloatx2C2x3E_0) -> DiffPair_float_0
{
    var _S114 : f32 = dpx_12.primal_0[i32(0)];
    var _S115 : f32 = dpx_12.differential_0[i32(0)] * dpx_12.primal_0[i32(0)];
    var _S116 : f32 = dpx_12.primal_0[i32(1)];
    var _S117 : f32 = dpx_12.differential_0[i32(1)] * dpx_12.primal_0[i32(1)];
    var _S118 : DiffPair_float_0 = DiffPair_float_0( _S114 * _S114 + _S116 * _S116, _S115 + _S115 + (_S117 + _S117) );
    var _S119 : DiffPair_float_0 = _d_sqrt_1(_S118);
    var _S120 : DiffPair_float_0 = DiffPair_float_0( _S119.primal_0, _S119.differential_0 );
    return _S120;
}

struct DiffPair_vectorx3Cfloatx2C4x3E_0
{
     primal_0 : vec4<f32>,
     differential_0 : vec4<f32>,
};

struct Curve_0
{
     toe_0 : vec4<f32>,
     shoulder_0 : vec4<f32>,
     line_0 : vec4<f32>,
     nodes_0 : vec4<f32>,
     tone_0 : vec4<f32>,
     display_0 : vec4<f32>,
     extra_0 : vec4<f32>,
};

struct Directions_0
{
     base_0 : Curve_0,
     d0_0 : Curve_0,
     d1_0 : Curve_0,
     d2_0 : Curve_0,
     d3_0 : Curve_0,
};

struct Modes_0
{
     version_0 : u32,
     norm_0 : u32,
     toe_1 : u32,
     shoulder_1 : u32,
     white_0 : bool,
     luminance_1 : vec3<f32>,
     to_lms_1 : mat3x3<f32>,
     from_lms_1 : mat3x3<f32>,
     export_to_lms_1 : mat3x3<f32>,
     export_from_lms_1 : mat3x3<f32>,
};

struct s_paramCtx_s_bwdCallableCtx_filmic_along_0
{
     value0_17 : vec3<f32>,
     value1_0 : vec4<f32>,
     value2_0 : Directions_0,
     value3_0 : Modes_0,
};

struct s_bwdCallableCtx_filmic_along_0
{
     value0_18 : s_paramCtx_s_bwdCallableCtx_filmic_along_0,
};

struct Tuple_15
{
     value0_19 : Curve_0,
};

fn s_apply_along_0( d_0 : Directions_0,  _S121 : vec4<f32>) -> Tuple_15
{
    var _S122 : vec4<f32> = vec4<f32>(_S121.x);
    var _S123 : vec4<f32> = vec4<f32>(_S121.y);
    var _S124 : vec4<f32> = vec4<f32>(_S121.z);
    var _S125 : vec4<f32> = vec4<f32>(_S121.w);
    var _S126 : vec4<f32> = d_0.base_0.shoulder_0 + _S122 * d_0.d0_0.shoulder_0 + _S123 * d_0.d1_0.shoulder_0 + _S124 * d_0.d2_0.shoulder_0 + _S125 * d_0.d3_0.shoulder_0;
    var _S127 : vec4<f32> = d_0.base_0.line_0 + _S122 * d_0.d0_0.line_0 + _S123 * d_0.d1_0.line_0 + _S124 * d_0.d2_0.line_0 + _S125 * d_0.d3_0.line_0;
    var _S128 : vec4<f32> = d_0.base_0.nodes_0 + _S122 * d_0.d0_0.nodes_0 + _S123 * d_0.d1_0.nodes_0 + _S124 * d_0.d2_0.nodes_0 + _S125 * d_0.d3_0.nodes_0;
    var _S129 : vec4<f32> = d_0.base_0.tone_0 + _S122 * d_0.d0_0.tone_0 + _S123 * d_0.d1_0.tone_0 + _S124 * d_0.d2_0.tone_0 + _S125 * d_0.d3_0.tone_0;
    var _S130 : vec4<f32> = d_0.base_0.display_0 + _S122 * d_0.d0_0.display_0 + _S123 * d_0.d1_0.display_0 + _S124 * d_0.d2_0.display_0 + _S125 * d_0.d3_0.display_0;
    var _S131 : vec4<f32> = d_0.base_0.extra_0 + _S122 * d_0.d0_0.extra_0 + _S123 * d_0.d1_0.extra_0 + _S124 * d_0.d2_0.extra_0 + _S125 * d_0.d3_0.extra_0;
    var _S132 : Tuple_15;
    _S132.value0_19.toe_0 = d_0.base_0.toe_0 + _S122 * d_0.d0_0.toe_0 + _S123 * d_0.d1_0.toe_0 + _S124 * d_0.d2_0.toe_0 + _S125 * d_0.d3_0.toe_0;
    _S132.value0_19.shoulder_0 = _S126;
    _S132.value0_19.line_0 = _S127;
    _S132.value0_19.nodes_0 = _S128;
    _S132.value0_19.tone_0 = _S129;
    _S132.value0_19.display_0 = _S130;
    _S132.value0_19.extra_0 = _S131;
    return _S132;
}

struct s_paramCtx_s_bwdCallableCtx_along_0
{
     value0_20 : Directions_0,
};

struct s_bwdCallableCtx_along_0
{
     value0_21 : s_paramCtx_s_bwdCallableCtx_along_0,
};

struct s_paramCtx_s_bwdCallableCtx_filmic_0
{
     value0_22 : vec3<f32>,
     value1_1 : Curve_0,
     value2_1 : Modes_0,
};

struct s_bwdCallableCtx_filmic_0
{
     value0_23 : s_paramCtx_s_bwdCallableCtx_filmic_0,
};

struct s_paramCtx_s_bwdCallableCtx_filmic_v7_0
{
     value0_24 : vec3<f32>,
     value1_2 : Curve_0,
     value2_2 : Modes_0,
};

struct s_paramCtx_s_bwdCallableCtx_filmic_v6_0
{
     value0_25 : vec3<f32>,
     value1_3 : Curve_0,
     value2_3 : Modes_0,
};

struct s_paramCtx_s_bwdCallableCtx_split_v1_0
{
     value0_26 : vec3<f32>,
     value1_4 : Curve_0,
     value2_4 : Modes_0,
};

struct s_paramCtx_s_bwdCallableCtx_split_v2_v3_0
{
     value0_27 : vec3<f32>,
     value1_5 : Curve_0,
     value2_5 : Modes_0,
};

struct s_paramCtx_s_bwdCallableCtx_chroma_v1_0
{
     value0_28 : vec3<f32>,
     value1_6 : Curve_0,
     value2_6 : Modes_0,
};

struct s_paramCtx_s_bwdCallableCtx_chroma_v2_v3_0
{
     value0_29 : vec3<f32>,
     value1_7 : Curve_0,
     value2_7 : Modes_0,
};

struct s_bwdCallableCtx_filmic_v7_0
{
     value0_30 : s_paramCtx_s_bwdCallableCtx_filmic_v7_0,
};

struct s_bwdCallableCtx_filmic_v6_0
{
     value0_31 : s_paramCtx_s_bwdCallableCtx_filmic_v6_0,
};

struct s_bwdCallableCtx_split_v1_0
{
     value0_32 : s_paramCtx_s_bwdCallableCtx_split_v1_0,
};

struct s_bwdCallableCtx_split_v2_v3_0
{
     value0_33 : s_paramCtx_s_bwdCallableCtx_split_v2_v3_0,
};

struct s_bwdCallableCtx_chroma_v1_0
{
     value0_34 : s_paramCtx_s_bwdCallableCtx_chroma_v1_0,
};

struct s_bwdCallableCtx_chroma_v2_v3_0
{
     value0_35 : s_paramCtx_s_bwdCallableCtx_chroma_v2_v3_0,
};

fn Curve_x24_syn_dzero_0() -> Curve_0
{
    var result_0 : Curve_0;
    var _S133 : vec4<f32> = vec4<f32>(0.0f);
    result_0.toe_0 = _S133;
    result_0.shoulder_0 = _S133;
    result_0.line_0 = _S133;
    result_0.nodes_0 = _S133;
    result_0.tone_0 = _S133;
    result_0.display_0 = _S133;
    result_0.extra_0 = _S133;
    return result_0;
}

struct s_paramCtx_s_bwdCallableCtx_pixel_norm_0
{
     value0_36 : vec3<f32>,
     value1_8 : u32,
     value2_8 : vec3<f32>,
};

struct s_paramCtx_s_bwdCallableCtx_sanitised_0
{
     value0_37 : vec3<f32>,
};

struct s_paramCtx_s_bwdCallableCtx_log_v2_0
{
     value0_38 : f32,
     value1_9 : Curve_0,
     value2_9 : bool,
};

struct s_paramCtx_s_bwdCallableCtx_desaturate_v2_0
{
     value0_39 : f32,
     value1_10 : Curve_0,
};

struct s_paramCtx_s_bwdCallableCtx_spline_0
{
     value0_40 : f32,
     value1_11 : Curve_0,
     value2_10 : u32,
     value3_1 : u32,
};

struct s_bwdCallableCtx_pixel_norm_0
{
     value0_41 : s_paramCtx_s_bwdCallableCtx_pixel_norm_0,
};

struct s_bwdCallableCtx_spline_0
{
     value0_42 : s_paramCtx_s_bwdCallableCtx_spline_0,
};

struct s_bwdCallableCtx_desaturate_v2_0
{
     value0_43 : s_paramCtx_s_bwdCallableCtx_desaturate_v2_0,
};

struct s_bwdCallableCtx_log_v2_0
{
     value0_44 : s_paramCtx_s_bwdCallableCtx_log_v2_0,
};

struct s_bwdCallableCtx_sanitised_0
{
     value0_45 : s_paramCtx_s_bwdCallableCtx_sanitised_0,
};

struct Tuple_16
{
     value0_46 : f32,
};

fn s_apply_pixel_norm_0( _S134 : vec3<f32>,  norm_1 : u32,  luminance_2 : vec3<f32>) -> Tuple_16
{
    var _S135 : bool = norm_1 == u32(1);
    var _S136 : f32;
    if(_S135)
    {
        _S136 = s_apply_max_1(s_apply_max_1(_S134.x, _S134.y).value0_15, _S134.z).value0_15;
    }
    var _S137 : bool = !_S135;
    if(_S137)
    {
        var _runFlag_0 : bool;
        if(norm_1 == u32(3))
        {
            var _S138 : Tuple_2 = s_apply_abs_0(_S134);
            var _S139 : vec3<f32> = _S138.value0_4;
            var squared_0 : vec3<f32> = _S139 * _S139;
            var _S140 : f32 = squared_0.x;
            var _S141 : f32 = squared_0.y;
            var _S142 : f32 = squared_0.z;
            var _S143 : f32 = (_S140 * _S138.value0_4.x + _S141 * _S138.value0_4.y + _S142 * _S138.value0_4.z) / s_apply_max_1(_S140 + _S141 + _S142, 9.999999960041972e-13f).value0_15;
            _runFlag_0 = false;
            _S136 = _S143;
        }
        else
        {
            _runFlag_0 = _S137;
        }
        if(_runFlag_0)
        {
            if(norm_1 == u32(4))
            {
                var _S144 : f32 = _S134.x;
                var _S145 : f32 = _S134.y;
                var _S146 : f32 = _S134.z;
                var _S147 : Tuple_3 = s_apply_sqrt_0(_S144 * _S144 + _S145 * _S145 + _S146 * _S146);
                _runFlag_0 = false;
                _S136 = _S147.value0_5;
            }
            if(_runFlag_0)
            {
                if(norm_1 == u32(5))
                {
                    var _S148 : f32 = _S134.x;
                    var _S149 : f32 = _S134.y;
                    var _S150 : f32 = _S134.z;
                    var _S151 : f32 = s_apply_sqrt_0(_S148 * _S148 + _S149 * _S149 + _S150 * _S150).value0_5 * 0.57735025882720947f;
                    _runFlag_0 = false;
                    _S136 = _S151;
                }
                if(_runFlag_0)
                {
                    _S136 = luminance_2.x * _S134.x + luminance_2.y * _S134.y + luminance_2.z * _S134.z;
                }
            }
        }
    }
    var _S152 : Tuple_16;
    _S152.value0_46 = _S136;
    return _S152;
}

struct Tuple_17
{
     value0_47 : vec3<f32>,
};

fn s_apply_sanitised_0( _S153 : vec3<f32>) -> Tuple_17
{
    var _S154 : Tuple_14 = s_apply_min_0(s_apply_min_0(_S153.x, _S153.y).value0_16, _S153.z);
    var _S155 : vec3<f32>;
    if(((_S154.value0_16) < 0.0f))
    {
        _S155 = _S153 - vec3<f32>(_S154.value0_16);
    }
    else
    {
        _S155 = _S153;
    }
    var _S156 : Tuple_17;
    _S156.value0_47 = _S155;
    return _S156;
}

struct Tuple_18
{
     value0_48 : f32,
};

struct Tuple_19
{
     value0_49 : f32,
};

fn s_apply_log_encoding_0( _S157 : f32,  _S158 : Curve_0) -> Tuple_19
{
    var _S159 : Tuple_19;
    _S159.value0_49 = (s_apply_log2_0(_S157 / _S158.nodes_0.z).value0_0 - _S158.nodes_0.w) / _S158.tone_0.x;
    return _S159;
}

fn s_apply_log_v2_0( _S160 : f32,  _S161 : Curve_0,  white_1 : bool) -> Tuple_18
{
    var _S162 : f32;
    if(!white_1)
    {
        _S162 = s_apply_clamp_0(s_apply_log_encoding_0(_S160, _S161).value0_49, 0.0f, 1.0f).value0_11;
    }
    else
    {
        _S162 = 1.0f;
    }
    var _S163 : Tuple_18;
    _S163.value0_48 = _S162;
    return _S163;
}

struct Tuple_20
{
     value0_50 : f32,
};

fn s_apply_desaturate_v2_0( _S164 : f32,  _S165 : Curve_0) -> Tuple_20
{
    var _S166 : Tuple_13 = s_apply_max_1(_S165.tone_0.z, 0.0f);
    var sat2_0 : f32 = 0.5f / s_apply_sqrt_0(s_apply_max_1(_S166.value0_15, 1.00000000317107685e-30f).value0_15).value0_5;
    var _S167 : f32 = 1.0f - _S164;
    var _S168 : Tuple_20;
    _S168.value0_50 = _S166.value0_15 - (s_apply_exp_0(- _S164 * _S164 / s_apply_max_1(_S165.tone_0.w, 1.17549435082228751e-38f).value0_15 * sat2_0).value0_6 + s_apply_exp_0(- _S167 * _S167 / s_apply_max_1(_S165.display_0.x, 1.17549435082228751e-38f).value0_15 * sat2_0).value0_6) * _S166.value0_15;
    return _S168;
}

struct Tuple_21
{
     value0_51 : f32,
};

fn s_apply_spline_0( _S169 : f32,  _S170 : Curve_0,  toe_2 : u32,  shoulder_2 : u32) -> Tuple_21
{
    var _S171 : f32 = _S170.nodes_0.x;
    var _S172 : f32;
    var _runFlag_1 : bool;
    if((_S169 < _S171))
    {
        var _S173 : bool = toe_2 == u32(2);
        if(_S173)
        {
            var xi_0 : f32 = _S171 - _S169;
            var rat_0 : f32 = xi_0 * (xi_0 * _S170.toe_0.y + 1.0f);
            _S172 = _S170.toe_0.w - _S170.toe_0.x * rat_0 / (rat_0 + _S170.toe_0.z);
        }
        var _S174 : bool = !_S173;
        if(_S174)
        {
            var d_1 : f32 = _S169 - _S171;
            _S172 = _S170.toe_0.x + d_1 * (_S170.toe_0.y + d_1 * (_S170.toe_0.z + d_1 * (_S170.toe_0.w + d_1 * _S170.line_0.x)));
        }
        _runFlag_1 = false;
    }
    else
    {
        _runFlag_1 = true;
    }
    if(_runFlag_1)
    {
        var _S175 : f32 = _S170.nodes_0.y;
        if((_S169 > _S175))
        {
            if(shoulder_2 == u32(2))
            {
                var xi_1 : f32 = _S169 - _S175;
                var rat_1 : f32 = xi_1 * (xi_1 * _S170.shoulder_0.y + 1.0f);
                var _S176 : f32 = _S170.shoulder_0.w + _S170.shoulder_0.x * rat_1 / (rat_1 + _S170.shoulder_0.z);
                _runFlag_1 = false;
                _S172 = _S176;
            }
            if(_runFlag_1)
            {
                var d_2 : f32 = _S169 - _S175;
                _S172 = _S170.shoulder_0.x + d_2 * (_S170.shoulder_0.y + d_2 * (_S170.shoulder_0.z + d_2 * (_S170.shoulder_0.w + d_2 * _S170.line_0.y)));
            }
            _runFlag_1 = false;
        }
        if(_runFlag_1)
        {
            _S172 = _S170.line_0.z + _S169 * _S170.line_0.w;
        }
    }
    var _S177 : Tuple_21;
    _S177.value0_51 = _S172;
    return _S177;
}

fn s_bwdProp_pixel_norm_0( _S178 : s_bwdCallableCtx_pixel_norm_0,  _S179 : ptr<function, vec3<f32>>,  _S180 : f32)
{
    var _S181 : vec3<f32> = vec3<f32>(0.0f);
    var _S182 : bool = (_S178.value0_41.value1_8) == u32(1);
    var _S183 : s_bwdCallableCtx_max_1;
    var _S184 : s_bwdCallableCtx_max_1;
    if(_S182)
    {
        var _S185 : f32 = _S178.value0_41.value0_36.x;
        var _S186 : f32 = _S178.value0_41.value0_36.y;
        var _S187 : Tuple_13 = s_apply_max_1(_S185, _S186);
        var _S188 : s_bwdCallableCtx_max_1;
        _S188._S101 = _S185;
        _S188._S102 = _S186;
        var _S189 : f32 = _S178.value0_41.value0_36.z;
        var _S190 : s_bwdCallableCtx_max_1;
        _S190._S101 = _S187.value0_15;
        _S190._S102 = _S189;
        _S183 = _S190;
        _S184 = _S188;
    }
    else
    {
        _S183._S101 = 0.0f;
        _S183._S102 = 0.0f;
        _S184._S101 = 0.0f;
        _S184._S102 = 0.0f;
    }
    var _S191 : bool = !_S182;
    var _S192 : s_bwdCallableCtx_max_1;
    var _runFlag_2 : bool;
    var _runFlag_3 : bool;
    var _runFlag_4 : bool;
    var _S193 : bool;
    var _S194 : bool;
    var _S195 : bool;
    var _S196 : f32;
    var _S197 : f32;
    var _S198 : f32;
    var _S199 : f32;
    var _S200 : f32;
    var _S201 : f32;
    var _S202 : f32;
    var _S203 : f32;
    var _S204 : f32;
    var _S205 : f32;
    var _S206 : f32;
    var _S207 : f32;
    var _S208 : f32;
    var _S209 : f32;
    var _S210 : f32;
    var _S211 : f32;
    var _S212 : f32;
    var _S213 : f32;
    var _S214 : vec3<f32>;
    var _S215 : s_bwdCallableCtx_abs_0;
    var _S216 : s_bwdCallableCtx_sqrt_0;
    var _S217 : s_bwdCallableCtx_sqrt_0;
    if(_S191)
    {
        var _S218 : bool = (_S178.value0_41.value1_8) == u32(3);
        if(_S218)
        {
            var _S219 : Tuple_2 = s_apply_abs_0(_S178.value0_41.value0_36);
            var _S220 : s_bwdCallableCtx_abs_0;
            _S220._S30 = _S178.value0_41.value0_36;
            var squared_1 : vec3<f32> = _S219.value0_4 * _S219.value0_4;
            var _S221 : f32 = squared_1.x;
            var _S222 : f32 = _S219.value0_4.x;
            var _S223 : f32 = squared_1.y;
            var _S224 : f32 = _S219.value0_4.y;
            var _S225 : f32 = squared_1.z;
            var _S226 : f32 = _S219.value0_4.z;
            var numerator_0 : f32 = _S221 * _S222 + _S223 * _S224 + _S225 * _S226;
            var denominator_0 : f32 = _S221 + _S223 + _S225;
            var _S227 : Tuple_13 = s_apply_max_1(denominator_0, 9.999999960041972e-13f);
            var _S228 : s_bwdCallableCtx_max_1;
            _S228._S101 = denominator_0;
            _S228._S102 = 9.999999960041972e-13f;
            var _S229 : f32 = _S227.value0_15 * _S227.value0_15;
            _runFlag_2 = false;
            _S196 = _S229;
            _S197 = numerator_0;
            _S198 = _S227.value0_15;
            _S192 = _S228;
            _S199 = _S225;
            _S200 = _S226;
            _S201 = _S223;
            _S202 = _S224;
            _S203 = _S221;
            _S204 = _S222;
            _S214 = _S219.value0_4;
            _S215 = _S220;
        }
        else
        {
            _runFlag_2 = _S191;
            _S196 = 0.0f;
            _S197 = 0.0f;
            _S198 = 0.0f;
            _S192._S101 = 0.0f;
            _S192._S102 = 0.0f;
            _S199 = 0.0f;
            _S200 = 0.0f;
            _S201 = 0.0f;
            _S202 = 0.0f;
            _S203 = 0.0f;
            _S204 = 0.0f;
            _S214 = _S181;
            _S215._S30 = _S181;
        }
        if(_runFlag_2)
        {
            var _S230 : bool = (_S178.value0_41.value1_8) == u32(4);
            if(_S230)
            {
                var _S231 : f32 = _S178.value0_41.value0_36.x;
                var _S232 : f32 = _S178.value0_41.value0_36.y;
                var _S233 : f32 = _S178.value0_41.value0_36.z;
                var _S234 : s_bwdCallableCtx_sqrt_0;
                _S234._S7 = _S231 * _S231 + _S232 * _S232 + _S233 * _S233;
                _runFlag_3 = false;
                _S216 = _S234;
                _S205 = _S233;
                _S206 = _S232;
                _S207 = _S231;
            }
            else
            {
                _runFlag_3 = _runFlag_2;
                _S216._S7 = 0.0f;
                _S205 = 0.0f;
                _S206 = 0.0f;
                _S207 = 0.0f;
            }
            if(_runFlag_3)
            {
                var _S235 : bool = (_S178.value0_41.value1_8) == u32(5);
                if(_S235)
                {
                    var _S236 : f32 = _S178.value0_41.value0_36.x;
                    var _S237 : f32 = _S178.value0_41.value0_36.y;
                    var _S238 : f32 = _S178.value0_41.value0_36.z;
                    var _S239 : s_bwdCallableCtx_sqrt_0;
                    _S239._S7 = _S236 * _S236 + _S237 * _S237 + _S238 * _S238;
                    _runFlag_4 = false;
                    _S217 = _S239;
                    _S208 = _S238;
                    _S209 = _S237;
                    _S210 = _S236;
                }
                else
                {
                    _runFlag_4 = _runFlag_3;
                    _S217._S7 = 0.0f;
                    _S208 = 0.0f;
                    _S209 = 0.0f;
                    _S210 = 0.0f;
                }
                if(_runFlag_4)
                {
                    var _S240 : f32 = _S178.value0_41.value2_8.x;
                    var _S241 : f32 = _S178.value0_41.value2_8.y;
                    _S211 = _S178.value0_41.value2_8.z;
                    _S212 = _S241;
                    _S213 = _S240;
                }
                else
                {
                    _S211 = 0.0f;
                    _S212 = 0.0f;
                    _S213 = 0.0f;
                }
                var _S242 : f32 = _S208;
                var _S243 : f32 = _S209;
                var _S244 : f32 = _S210;
                _S208 = _S211;
                _S209 = _S212;
                _S210 = _S213;
                _S193 = _S235;
                _S211 = _S242;
                _S212 = _S243;
                _S213 = _S244;
            }
            else
            {
                _runFlag_4 = false;
                _S208 = 0.0f;
                _S209 = 0.0f;
                _S210 = 0.0f;
                _S193 = false;
                _S217._S7 = 0.0f;
                _S211 = 0.0f;
                _S212 = 0.0f;
                _S213 = 0.0f;
            }
            var _S245 : s_bwdCallableCtx_sqrt_0 = _S216;
            var _S246 : f32 = _S205;
            var _S247 : f32 = _S206;
            var _S248 : f32 = _S207;
            _S205 = _S208;
            _S206 = _S209;
            _S207 = _S210;
            _S216 = _S217;
            _S208 = _S211;
            _S209 = _S212;
            _S210 = _S213;
            _S194 = _S230;
            _S217 = _S245;
            _S211 = _S246;
            _S212 = _S247;
            _S213 = _S248;
        }
        else
        {
            _runFlag_3 = false;
            _runFlag_4 = false;
            _S205 = 0.0f;
            _S206 = 0.0f;
            _S207 = 0.0f;
            _S193 = false;
            _S216._S7 = 0.0f;
            _S208 = 0.0f;
            _S209 = 0.0f;
            _S210 = 0.0f;
            _S194 = false;
            _S217._S7 = 0.0f;
            _S211 = 0.0f;
            _S212 = 0.0f;
            _S213 = 0.0f;
        }
        var _S249 : f32 = _S196;
        var _S250 : f32 = _S197;
        var _S251 : f32 = _S198;
        var _S252 : f32 = _S199;
        var _S253 : f32 = _S200;
        var _S254 : f32 = _S201;
        var _S255 : f32 = _S202;
        var _S256 : f32 = _S203;
        var _S257 : f32 = _S204;
        _S196 = _S205;
        _S197 = _S206;
        _S198 = _S207;
        _S199 = _S208;
        _S200 = _S209;
        _S201 = _S210;
        _S202 = _S211;
        _S203 = _S212;
        _S204 = _S213;
        _S195 = _S218;
        _S205 = _S249;
        _S206 = _S250;
        _S207 = _S251;
        _S208 = _S252;
        _S209 = _S253;
        _S210 = _S254;
        _S211 = _S255;
        _S212 = _S256;
        _S213 = _S257;
    }
    else
    {
        _runFlag_2 = false;
        _runFlag_3 = false;
        _runFlag_4 = false;
        _S196 = 0.0f;
        _S197 = 0.0f;
        _S198 = 0.0f;
        _S193 = false;
        _S216._S7 = 0.0f;
        _S199 = 0.0f;
        _S200 = 0.0f;
        _S201 = 0.0f;
        _S194 = false;
        _S217._S7 = 0.0f;
        _S202 = 0.0f;
        _S203 = 0.0f;
        _S204 = 0.0f;
        _S195 = false;
        _S205 = 0.0f;
        _S206 = 0.0f;
        _S207 = 0.0f;
        _S192._S101 = 0.0f;
        _S192._S102 = 0.0f;
        _S208 = 0.0f;
        _S209 = 0.0f;
        _S210 = 0.0f;
        _S211 = 0.0f;
        _S212 = 0.0f;
        _S213 = 0.0f;
        _S214 = _S181;
        _S215._S30 = _S181;
    }
    if(_S191)
    {
        var _S258 : vec3<f32>;
        if(_runFlag_2)
        {
            if(_runFlag_3)
            {
                if(_runFlag_4)
                {
                    var _S259 : vec3<f32> = vec3<f32>(_S198 * _S180, _S197 * _S180, _S196 * _S180);
                    _S196 = 0.0f;
                    _S258 = _S259;
                }
                else
                {
                    _S196 = _S180;
                    _S258 = _S181;
                }
                if(_S193)
                {
                    var _S260 : f32 = 0.57735025882720947f * _S196;
                    var _S261 : f32 = 0.0f;
                    s_bwdProp_sqrt_0(_S216, &(_S261), _S260);
                    var _S262 : f32 = _S199 * _S261;
                    var _S263 : f32 = _S200 * _S261;
                    var _S264 : f32 = _S201 * _S261;
                    var _S265 : vec3<f32> = _S258 + vec3<f32>(_S264 + _S264, _S263 + _S263, _S262 + _S262);
                    _S196 = 0.0f;
                    _S258 = _S265;
                }
            }
            else
            {
                _S196 = _S180;
                _S258 = _S181;
            }
            if(_S194)
            {
                var _S266 : f32 = 0.0f;
                s_bwdProp_sqrt_0(_S217, &(_S266), _S196);
                var _S267 : f32 = _S202 * _S266;
                var _S268 : f32 = _S203 * _S266;
                var _S269 : f32 = _S204 * _S266;
                var _S270 : vec3<f32> = _S258 + vec3<f32>(_S269 + _S269, _S268 + _S268, _S267 + _S267);
                _S196 = 0.0f;
                _S258 = _S270;
            }
        }
        else
        {
            _S196 = _S180;
            _S258 = _S181;
        }
        if(_S195)
        {
            var _S271 : f32 = _S196 / _S205;
            var _S272 : f32 = _S206 * - _S271;
            var s_diff_numerator_T_0 : f32 = _S207 * _S271;
            var _S273 : f32 = 0.0f;
            var _S274 : f32 = 0.0f;
            s_bwdProp_max_1(_S192, &(_S273), &(_S274), _S272);
            var _S275 : vec3<f32> = _S214 * vec3<f32>(_S273 + _S213 * s_diff_numerator_T_0, _S273 + _S211 * s_diff_numerator_T_0, _S273 + _S209 * s_diff_numerator_T_0);
            var _S276 : vec3<f32> = _S275 + _S275 + vec3<f32>(_S212 * s_diff_numerator_T_0, _S210 * s_diff_numerator_T_0, _S208 * s_diff_numerator_T_0);
            var _S277 : vec3<f32> = _S181;
            s_bwdProp_abs_0(_S215, &(_S277), _S276);
            var _S278 : vec3<f32> = _S277 + _S258;
            _S196 = 0.0f;
            _S214 = _S278;
        }
        else
        {
            _S214 = _S258;
        }
    }
    else
    {
        _S196 = _S180;
        _S214 = _S181;
    }
    if(_S182)
    {
        var _S279 : f32 = 0.0f;
        var _S280 : f32 = 0.0f;
        s_bwdProp_max_1(_S183, &(_S279), &(_S280), _S196);
        var _S281 : f32 = 0.0f;
        var _S282 : f32 = 0.0f;
        s_bwdProp_max_1(_S184, &(_S281), &(_S282), _S279);
        _S214 = _S214 + vec3<f32>(_S281, _S282, _S280);
    }
    (*_S179) = _S214;
    return;
}

fn Curve_x24_syn_dadd_0( SLANG_anonymous_0_0 : Curve_0,  SLANG_anonymous_1_0 : Curve_0) -> Curve_0
{
    var result_1 : Curve_0;
    result_1.toe_0 = SLANG_anonymous_0_0.toe_0 + SLANG_anonymous_1_0.toe_0;
    result_1.shoulder_0 = SLANG_anonymous_0_0.shoulder_0 + SLANG_anonymous_1_0.shoulder_0;
    result_1.line_0 = SLANG_anonymous_0_0.line_0 + SLANG_anonymous_1_0.line_0;
    result_1.nodes_0 = SLANG_anonymous_0_0.nodes_0 + SLANG_anonymous_1_0.nodes_0;
    result_1.tone_0 = SLANG_anonymous_0_0.tone_0 + SLANG_anonymous_1_0.tone_0;
    result_1.display_0 = SLANG_anonymous_0_0.display_0 + SLANG_anonymous_1_0.display_0;
    result_1.extra_0 = SLANG_anonymous_0_0.extra_0 + SLANG_anonymous_1_0.extra_0;
    return result_1;
}

fn s_bwdProp_spline_0( _S283 : s_bwdCallableCtx_spline_0,  _S284 : ptr<function, f32>,  _S285 : ptr<function, Curve_0>,  _S286 : f32)
{
    var _S287 : f32 = _S283.value0_42.value1_11.nodes_0.x;
    var _S288 : bool = ((_S283.value0_42.value0_40) < _S287);
    var _S289 : f32;
    var _S290 : f32;
    var _S291 : f32;
    var _S292 : f32;
    var _S293 : f32;
    var _S294 : f32;
    var _S295 : f32;
    var _S296 : f32;
    var _S297 : f32;
    var _S298 : f32;
    var _S299 : f32;
    var _S300 : f32;
    var _S301 : f32;
    var _runFlag_5 : bool;
    var _S302 : bool;
    var _S303 : bool;
    if(_S288)
    {
        var _S304 : bool = (_S283.value0_42.value2_10) == u32(2);
        if(_S304)
        {
            var xi_2 : f32 = _S287 - _S283.value0_42.value0_40;
            var _S305 : f32 = _S283.value0_42.value1_11.toe_0.y;
            var _S306 : f32 = xi_2 * _S305 + 1.0f;
            var rat_2 : f32 = xi_2 * _S306;
            var _S307 : f32 = _S283.value0_42.value1_11.toe_0.x;
            var _S308 : f32 = _S307 * rat_2;
            var _S309 : f32 = rat_2 + _S283.value0_42.value1_11.toe_0.z;
            _S289 = _S309 * _S309;
            _S290 = _S308;
            _S291 = _S309;
            _S292 = _S307;
            _S293 = rat_2;
            _S294 = xi_2;
            _S295 = _S306;
            _S296 = _S305;
        }
        else
        {
            _S289 = 0.0f;
            _S290 = 0.0f;
            _S291 = 0.0f;
            _S292 = 0.0f;
            _S293 = 0.0f;
            _S294 = 0.0f;
            _S295 = 0.0f;
            _S296 = 0.0f;
        }
        var _S310 : bool = !_S304;
        if(_S310)
        {
            var d_3 : f32 = _S283.value0_42.value0_40 - _S287;
            var _S311 : f32 = _S283.value0_42.value1_11.line_0.x;
            var _S312 : f32 = _S283.value0_42.value1_11.toe_0.w + d_3 * _S311;
            var _S313 : f32 = _S283.value0_42.value1_11.toe_0.z + d_3 * _S312;
            var _S314 : f32 = _S283.value0_42.value1_11.toe_0.y + d_3 * _S313;
            _S297 = d_3;
            _S298 = _S314;
            _S299 = _S313;
            _S300 = _S312;
            _S301 = _S311;
        }
        else
        {
            _S297 = 0.0f;
            _S298 = 0.0f;
            _S299 = 0.0f;
            _S300 = 0.0f;
            _S301 = 0.0f;
        }
        var _S315 : f32 = _S289;
        var _S316 : f32 = _S290;
        var _S317 : f32 = _S291;
        var _S318 : f32 = _S292;
        var _S319 : f32 = _S293;
        var _S320 : f32 = _S294;
        var _S321 : f32 = _S295;
        var _S322 : f32 = _S296;
        _runFlag_5 = false;
        _S302 = _S310;
        _S289 = _S297;
        _S290 = _S298;
        _S291 = _S299;
        _S292 = _S300;
        _S293 = _S301;
        _S303 = _S304;
        _S294 = _S315;
        _S295 = _S316;
        _S296 = _S317;
        _S297 = _S318;
        _S298 = _S319;
        _S299 = _S320;
        _S300 = _S321;
        _S301 = _S322;
    }
    else
    {
        _runFlag_5 = true;
        _S302 = false;
        _S289 = 0.0f;
        _S290 = 0.0f;
        _S291 = 0.0f;
        _S292 = 0.0f;
        _S293 = 0.0f;
        _S303 = false;
        _S294 = 0.0f;
        _S295 = 0.0f;
        _S296 = 0.0f;
        _S297 = 0.0f;
        _S298 = 0.0f;
        _S299 = 0.0f;
        _S300 = 0.0f;
        _S301 = 0.0f;
    }
    var _S323 : f32;
    var _S324 : f32;
    var _S325 : f32;
    var _S326 : f32;
    var _S327 : f32;
    var _S328 : f32;
    var _S329 : f32;
    var _S330 : f32;
    var _S331 : f32;
    var _S332 : f32;
    var _S333 : f32;
    var _S334 : f32;
    var _S335 : f32;
    var _S336 : f32;
    var _runFlag_6 : bool;
    var _S337 : bool;
    var _S338 : bool;
    var _S339 : bool;
    if(_runFlag_5)
    {
        var _S340 : f32 = _S283.value0_42.value1_11.nodes_0.y;
        var _S341 : bool = ((_S283.value0_42.value0_40) > _S340);
        if(_S341)
        {
            var _S342 : bool = (_S283.value0_42.value3_1) == u32(2);
            if(_S342)
            {
                var xi_3 : f32 = _S283.value0_42.value0_40 - _S340;
                var _S343 : f32 = _S283.value0_42.value1_11.shoulder_0.y;
                var _S344 : f32 = xi_3 * _S343 + 1.0f;
                var rat_3 : f32 = xi_3 * _S344;
                var _S345 : f32 = _S283.value0_42.value1_11.shoulder_0.x;
                var _S346 : f32 = _S345 * rat_3;
                var _S347 : f32 = rat_3 + _S283.value0_42.value1_11.shoulder_0.z;
                var _S348 : f32 = _S347 * _S347;
                _runFlag_6 = false;
                _S323 = _S348;
                _S324 = _S346;
                _S325 = _S347;
                _S326 = _S345;
                _S327 = rat_3;
                _S328 = xi_3;
                _S329 = _S344;
                _S330 = _S343;
            }
            else
            {
                _runFlag_6 = _runFlag_5;
                _S323 = 0.0f;
                _S324 = 0.0f;
                _S325 = 0.0f;
                _S326 = 0.0f;
                _S327 = 0.0f;
                _S328 = 0.0f;
                _S329 = 0.0f;
                _S330 = 0.0f;
            }
            if(_runFlag_6)
            {
                var d_4 : f32 = _S283.value0_42.value0_40 - _S340;
                var _S349 : f32 = _S283.value0_42.value1_11.line_0.y;
                var _S350 : f32 = _S283.value0_42.value1_11.shoulder_0.w + d_4 * _S349;
                var _S351 : f32 = _S283.value0_42.value1_11.shoulder_0.z + d_4 * _S350;
                var _S352 : f32 = _S283.value0_42.value1_11.shoulder_0.y + d_4 * _S351;
                _S331 = d_4;
                _S332 = _S352;
                _S333 = _S351;
                _S334 = _S350;
                _S335 = _S349;
            }
            else
            {
                _S331 = 0.0f;
                _S332 = 0.0f;
                _S333 = 0.0f;
                _S334 = 0.0f;
                _S335 = 0.0f;
            }
            var _S353 : bool = _runFlag_6;
            var _S354 : f32 = _S323;
            var _S355 : f32 = _S324;
            var _S356 : f32 = _S325;
            var _S357 : f32 = _S326;
            var _S358 : f32 = _S327;
            var _S359 : f32 = _S328;
            var _S360 : f32 = _S329;
            var _S361 : f32 = _S330;
            _runFlag_6 = false;
            _S337 = _S353;
            _S323 = _S331;
            _S324 = _S332;
            _S325 = _S333;
            _S326 = _S334;
            _S327 = _S335;
            _S338 = _S342;
            _S328 = _S354;
            _S329 = _S355;
            _S330 = _S356;
            _S331 = _S357;
            _S332 = _S358;
            _S333 = _S359;
            _S334 = _S360;
            _S335 = _S361;
        }
        else
        {
            _runFlag_6 = _runFlag_5;
            _S337 = false;
            _S323 = 0.0f;
            _S324 = 0.0f;
            _S325 = 0.0f;
            _S326 = 0.0f;
            _S327 = 0.0f;
            _S338 = false;
            _S328 = 0.0f;
            _S329 = 0.0f;
            _S330 = 0.0f;
            _S331 = 0.0f;
            _S332 = 0.0f;
            _S333 = 0.0f;
            _S334 = 0.0f;
            _S335 = 0.0f;
        }
        if(_runFlag_6)
        {
            _S336 = _S283.value0_42.value1_11.line_0.w;
        }
        else
        {
            _S336 = 0.0f;
        }
        var _S362 : bool = _S337;
        var _S363 : f32 = _S323;
        var _S364 : f32 = _S324;
        var _S365 : f32 = _S325;
        var _S366 : f32 = _S326;
        var _S367 : f32 = _S327;
        var _S368 : bool = _S338;
        var _S369 : f32 = _S328;
        var _S370 : f32 = _S329;
        var _S371 : f32 = _S330;
        var _S372 : f32 = _S331;
        var _S373 : f32 = _S332;
        var _S374 : f32 = _S333;
        var _S375 : f32 = _S334;
        var _S376 : f32 = _S335;
        _S323 = _S336;
        _S337 = _S341;
        _S338 = _S362;
        _S324 = _S363;
        _S325 = _S364;
        _S326 = _S365;
        _S327 = _S366;
        _S328 = _S367;
        _S339 = _S368;
        _S329 = _S369;
        _S330 = _S370;
        _S331 = _S371;
        _S332 = _S372;
        _S333 = _S373;
        _S334 = _S374;
        _S335 = _S375;
        _S336 = _S376;
    }
    else
    {
        _runFlag_6 = false;
        _S323 = 0.0f;
        _S337 = false;
        _S338 = false;
        _S324 = 0.0f;
        _S325 = 0.0f;
        _S326 = 0.0f;
        _S327 = 0.0f;
        _S328 = 0.0f;
        _S339 = false;
        _S329 = 0.0f;
        _S330 = 0.0f;
        _S331 = 0.0f;
        _S332 = 0.0f;
        _S333 = 0.0f;
        _S334 = 0.0f;
        _S335 = 0.0f;
        _S336 = 0.0f;
    }
    var _S377 : vec4<f32> = vec4<f32>(0.0f);
    var _S378 : Curve_0 = Curve_x24_syn_dzero_0();
    var _S379 : Curve_0;
    var _S380 : vec4<f32>;
    if(_runFlag_5)
    {
        var _S381 : f32;
        if(_runFlag_6)
        {
            var _S382 : f32 = _S323 * _S286;
            var _S383 : vec4<f32> = vec4<f32>(0.0f, 0.0f, _S286, _S283.value0_42.value0_40 * _S286);
            var _S384 : Curve_0 = _S378;
            _S384.line_0 = _S383;
            var _S385 : Curve_0 = Curve_x24_syn_dadd_0(_S378, _S384);
            _S323 = 0.0f;
            _S379 = _S385;
            _S381 = _S382;
        }
        else
        {
            _S323 = _S286;
            _S379 = _S378;
            _S381 = 0.0f;
        }
        if(_S337)
        {
            if(_S338)
            {
                var _S386 : f32 = _S324 * _S323;
                var _S387 : f32 = _S324 * _S386;
                var _S388 : f32 = _S324 * _S387;
                var _S389 : vec4<f32> = vec4<f32>(0.0f, _S324 * _S388, 0.0f, 0.0f);
                var _S390 : vec4<f32> = vec4<f32>(_S323, _S386, _S387, _S388);
                var _S391 : f32 = _S325 * _S323 + _S326 * _S386 + _S327 * _S387 + _S328 * _S388;
                var _S392 : f32 = - _S391;
                var _S393 : Curve_0 = _S378;
                _S393.line_0 = _S389;
                _S393.shoulder_0 = _S390;
                var _S394 : Curve_0 = Curve_x24_syn_dadd_0(_S379, _S393);
                var _S395 : f32 = _S391 + _S381;
                _S323 = 0.0f;
                _S379 = _S394;
                _S324 = _S395;
                _S325 = _S392;
            }
            else
            {
                _S324 = _S381;
                _S325 = 0.0f;
            }
            if(_S339)
            {
                var _S396 : f32 = _S323 / _S329;
                var _S397 : f32 = _S330 * - _S396;
                var _S398 : f32 = _S331 * _S396;
                var _S399 : f32 = _S397 + _S332 * _S398;
                var _S400 : f32 = _S334 * _S399;
                var _S401 : vec4<f32> = vec4<f32>(_S333 * _S398, _S334 * _S400, _S397, _S323);
                var _S402 : f32 = _S335 * _S399 + _S336 * _S400;
                var _S403 : f32 = - _S402;
                var _S404 : Curve_0 = _S378;
                _S404.shoulder_0 = _S401;
                var _S405 : Curve_0 = Curve_x24_syn_dadd_0(_S379, _S404);
                var _S406 : f32 = _S402 + _S324;
                _S323 = _S403 + _S325;
                _S324 = 0.0f;
                _S379 = _S405;
                _S325 = _S406;
            }
            else
            {
                var _S407 : f32 = _S323;
                var _S408 : f32 = _S324;
                _S323 = _S325;
                _S324 = _S407;
                _S325 = _S408;
            }
        }
        else
        {
            var _S409 : f32 = _S323;
            _S323 = 0.0f;
            _S324 = _S409;
            _S325 = _S381;
        }
        var _S410 : vec4<f32> = vec4<f32>(0.0f, _S323, 0.0f, 0.0f);
        _S323 = _S324;
        _S324 = _S325;
        _S380 = _S410;
    }
    else
    {
        _S323 = _S286;
        _S379 = _S378;
        _S324 = 0.0f;
        _S380 = _S377;
    }
    if(_S288)
    {
        if(_S302)
        {
            var _S411 : f32 = _S289 * _S323;
            var _S412 : f32 = _S289 * _S411;
            var _S413 : f32 = _S289 * _S412;
            var _S414 : vec4<f32> = vec4<f32>(_S289 * _S413, 0.0f, 0.0f, 0.0f);
            var _S415 : vec4<f32> = vec4<f32>(_S323, _S411, _S412, _S413);
            var _S416 : f32 = _S290 * _S323 + _S291 * _S411 + _S292 * _S412 + _S293 * _S413;
            var _S417 : f32 = - _S416;
            var _S418 : Curve_0 = _S378;
            _S418.line_0 = _S414;
            _S418.toe_0 = _S415;
            var _S419 : Curve_0 = Curve_x24_syn_dadd_0(_S379, _S418);
            var _S420 : f32 = _S416 + _S324;
            _S289 = 0.0f;
            _S379 = _S419;
            _S290 = _S417;
            _S291 = _S420;
        }
        else
        {
            _S289 = _S323;
            _S290 = 0.0f;
            _S291 = _S324;
        }
        if(_S303)
        {
            var _S421 : f32 = - _S289 / _S294;
            var _S422 : f32 = _S295 * - _S421;
            var _S423 : f32 = _S296 * _S421;
            var _S424 : f32 = _S422 + _S297 * _S423;
            var _S425 : f32 = _S299 * _S424;
            var _S426 : vec4<f32> = vec4<f32>(_S298 * _S423, _S299 * _S425, _S422, _S289);
            var _S427 : f32 = _S300 * _S424 + _S301 * _S425;
            var _S428 : f32 = - _S427;
            var _S429 : Curve_0 = _S378;
            _S429.toe_0 = _S426;
            var _S430 : Curve_0 = Curve_x24_syn_dadd_0(_S379, _S429);
            var _S431 : f32 = _S428 + _S291;
            _S289 = _S427 + _S290;
            _S379 = _S430;
            _S290 = _S431;
        }
        else
        {
            _S289 = _S290;
            _S290 = _S291;
        }
    }
    else
    {
        _S289 = 0.0f;
        _S290 = _S324;
    }
    var _S432 : vec4<f32> = _S380 + vec4<f32>(_S289, 0.0f, 0.0f, 0.0f);
    var _S433 : Curve_0 = _S378;
    _S433.nodes_0 = _S432;
    var _S434 : Curve_0 = Curve_x24_syn_dadd_0(_S379, _S433);
    (*_S284) = _S290;
    (*_S285) = _S434;
    return;
}

fn s_bwdProp_desaturate_v2_0( _S435 : s_bwdCallableCtx_desaturate_v2_0,  _S436 : ptr<function, f32>,  _S437 : ptr<function, Curve_0>,  _S438 : f32)
{
    var _S439 : f32 = _S435.value0_43.value1_10.tone_0.z;
    var _S440 : Tuple_13 = s_apply_max_1(_S439, 0.0f);
    var _S441 : s_bwdCallableCtx_max_1;
    _S441._S101 = _S439;
    _S441._S102 = 0.0f;
    var _S442 : Tuple_13 = s_apply_max_1(_S440.value0_15, 1.00000000317107685e-30f);
    var _S443 : s_bwdCallableCtx_max_1;
    _S443._S101 = _S440.value0_15;
    _S443._S102 = 1.00000000317107685e-30f;
    var _S444 : Tuple_3 = s_apply_sqrt_0(_S442.value0_15);
    var _S445 : s_bwdCallableCtx_sqrt_0;
    _S445._S7 = _S442.value0_15;
    var sat2_1 : f32 = 0.5f / _S444.value0_5;
    var _S446 : f32 = _S444.value0_5 * _S444.value0_5;
    var _S447 : f32 = - _S435.value0_43.value0_39;
    var _S448 : f32 = _S447 * _S435.value0_43.value0_39;
    var _S449 : f32 = _S435.value0_43.value1_10.tone_0.w;
    var _S450 : Tuple_13 = s_apply_max_1(_S449, 1.17549435082228751e-38f);
    var _S451 : s_bwdCallableCtx_max_1;
    _S451._S101 = _S449;
    _S451._S102 = 1.17549435082228751e-38f;
    var _S452 : f32 = _S448 / _S450.value0_15;
    var _S453 : f32 = _S450.value0_15 * _S450.value0_15;
    var _S454 : f32 = _S452 * sat2_1;
    var _S455 : Tuple_4 = s_apply_exp_0(_S454);
    var _S456 : s_bwdCallableCtx_exp_0;
    _S456._S36 = _S454;
    var _S457 : f32 = 1.0f - _S435.value0_43.value0_39;
    var _S458 : f32 = - _S457;
    var _S459 : f32 = _S458 * _S457;
    var _S460 : f32 = _S435.value0_43.value1_10.display_0.x;
    var _S461 : Tuple_13 = s_apply_max_1(_S460, 1.17549435082228751e-38f);
    var _S462 : s_bwdCallableCtx_max_1;
    _S462._S101 = _S460;
    _S462._S102 = 1.17549435082228751e-38f;
    var _S463 : f32 = _S459 / _S461.value0_15;
    var _S464 : f32 = _S461.value0_15 * _S461.value0_15;
    var _S465 : f32 = _S463 * sat2_1;
    var _S466 : Tuple_4 = s_apply_exp_0(_S465);
    var _S467 : s_bwdCallableCtx_exp_0;
    _S467._S36 = _S465;
    var _S468 : Curve_0 = Curve_x24_syn_dzero_0();
    var _S469 : f32 = - _S438;
    var _S470 : f32 = (_S455.value0_6 + _S466.value0_6) * _S469;
    var _S471 : f32 = _S440.value0_15 * _S469;
    var _S472 : f32 = 0.0f;
    s_bwdProp_exp_0(_S467, &(_S472), _S471);
    var _S473 : f32 = _S463 * _S472;
    var _S474 : f32 = sat2_1 * _S472 / _S464;
    var _S475 : f32 = _S459 * - _S474;
    var _S476 : f32 = _S461.value0_15 * _S474;
    var _S477 : f32 = 0.0f;
    var _S478 : f32 = 0.0f;
    s_bwdProp_max_1(_S462, &(_S477), &(_S478), _S475);
    var _S479 : vec4<f32> = vec4<f32>(_S477, 0.0f, 0.0f, 0.0f);
    var _S480 : f32 = - (_S458 * _S476 + - (_S457 * _S476));
    var _S481 : f32 = 0.0f;
    s_bwdProp_exp_0(_S456, &(_S481), _S471);
    var _S482 : f32 = _S452 * _S481;
    var _S483 : f32 = sat2_1 * _S481 / _S453;
    var _S484 : f32 = _S448 * - _S483;
    var _S485 : f32 = _S450.value0_15 * _S483;
    var _S486 : f32 = 0.0f;
    var _S487 : f32 = 0.0f;
    s_bwdProp_max_1(_S451, &(_S486), &(_S487), _S484);
    var _S488 : f32 = _S447 * _S485;
    var _S489 : f32 = - (_S435.value0_43.value0_39 * _S485);
    var _S490 : f32 = 0.5f * - ((_S473 + _S482) / _S446);
    var _S491 : f32 = 0.0f;
    s_bwdProp_sqrt_0(_S445, &(_S491), _S490);
    var _S492 : f32 = 0.0f;
    var _S493 : f32 = 0.0f;
    s_bwdProp_max_1(_S443, &(_S492), &(_S493), _S491);
    var _S494 : f32 = _S438 + _S470 + _S492;
    var _S495 : f32 = 0.0f;
    var _S496 : f32 = 0.0f;
    s_bwdProp_max_1(_S441, &(_S495), &(_S496), _S494);
    var _S497 : vec4<f32> = vec4<f32>(0.0f, 0.0f, _S495, _S486);
    var _S498 : Curve_0 = _S468;
    _S498.display_0 = _S479;
    _S498.tone_0 = _S497;
    var _S499 : Curve_0 = Curve_x24_syn_dadd_0(_S468, _S498);
    (*_S436) = _S480 + _S488 + _S489;
    (*_S437) = _S499;
    return;
}

struct s_paramCtx_s_bwdCallableCtx_log_encoding_0
{
     value0_52 : f32,
     value1_12 : Curve_0,
};

struct s_bwdCallableCtx_log_encoding_0
{
     value0_53 : s_paramCtx_s_bwdCallableCtx_log_encoding_0,
};

fn s_bwdProp_log_encoding_0( _S500 : s_bwdCallableCtx_log_encoding_0,  _S501 : ptr<function, f32>,  _S502 : ptr<function, Curve_0>,  _S503 : f32)
{
    var _S504 : f32 = _S500.value0_53.value1_12.nodes_0.z;
    var _S505 : f32 = _S500.value0_53.value0_52 / _S504;
    var _S506 : f32 = _S504 * _S504;
    var _S507 : Tuple_0 = s_apply_log2_0(_S505);
    var _S508 : s_bwdCallableCtx_log2_0;
    _S508._S2 = _S505;
    var _S509 : f32 = _S500.value0_53.value1_12.tone_0.x;
    var _S510 : Curve_0 = Curve_x24_syn_dzero_0();
    var _S511 : f32 = _S503 / (_S509 * _S509);
    var _S512 : f32 = _S509 * _S511;
    var _S513 : vec4<f32> = vec4<f32>((_S507.value0_0 - _S500.value0_53.value1_12.nodes_0.w) * - _S511, 0.0f, 0.0f, 0.0f);
    var _S514 : f32 = - _S512;
    var _S515 : f32 = 0.0f;
    s_bwdProp_log2_0(_S508, &(_S515), _S512);
    var _S516 : f32 = _S515 / _S506;
    var _S517 : f32 = _S504 * _S516;
    var _S518 : vec4<f32> = vec4<f32>(0.0f, 0.0f, _S500.value0_53.value0_52 * - _S516, _S514);
    var _S519 : Curve_0 = _S510;
    _S519.tone_0 = _S513;
    _S519.nodes_0 = _S518;
    var _S520 : Curve_0 = Curve_x24_syn_dadd_0(_S510, _S519);
    (*_S501) = _S517;
    (*_S502) = _S520;
    return;
}

fn s_bwdProp_log_v2_0( _S521 : s_bwdCallableCtx_log_v2_0,  _S522 : ptr<function, f32>,  _S523 : ptr<function, Curve_0>,  _S524 : f32)
{
    var _S525 : vec4<f32> = vec4<f32>(0.0f);
    var _S526 : Curve_0 = Curve_0( _S525, _S525, _S525, _S525, _S525, _S525, _S525 );
    var _S527 : s_paramCtx_s_bwdCallableCtx_log_encoding_0 = s_paramCtx_s_bwdCallableCtx_log_encoding_0( 0.0f, _S526 );
    var _S528 : bool = !_S521.value0_44.value2_9;
    var _S529 : s_bwdCallableCtx_clamp_0;
    var _S530 : s_bwdCallableCtx_log_encoding_0;
    if(_S528)
    {
        var _S531 : Tuple_19 = s_apply_log_encoding_0(_S521.value0_44.value0_38, _S521.value0_44.value1_9);
        var _S532 : s_paramCtx_s_bwdCallableCtx_log_encoding_0;
        _S532.value0_52 = _S521.value0_44.value0_38;
        _S532.value1_12 = _S521.value0_44.value1_9;
        var _S533 : s_bwdCallableCtx_clamp_0;
        _S533._S65 = _S531.value0_49;
        _S533._S66 = 0.0f;
        _S533._S67 = 1.0f;
        _S529 = _S533;
        _S530.value0_53 = _S532;
    }
    else
    {
        _S529._S65 = 0.0f;
        _S529._S66 = 0.0f;
        _S529._S67 = 0.0f;
        _S530.value0_53 = _S527;
    }
    var _S534 : Curve_0 = Curve_x24_syn_dzero_0();
    var _S535 : f32;
    var _S536 : Curve_0;
    if(_S528)
    {
        var _S537 : f32 = 0.0f;
        var _S538 : f32 = 0.0f;
        var _S539 : f32 = 0.0f;
        s_bwdProp_clamp_0(_S529, &(_S537), &(_S538), &(_S539), _S524);
        var _S540 : f32 = 0.0f;
        var _S541 : Curve_0;
        _S541.toe_0 = _S525;
        _S541.shoulder_0 = _S525;
        _S541.line_0 = _S525;
        _S541.nodes_0 = _S525;
        _S541.tone_0 = _S525;
        _S541.display_0 = _S525;
        _S541.extra_0 = _S525;
        s_bwdProp_log_encoding_0(_S530, &(_S540), &(_S541), _S537);
        var _S542 : Curve_0 = Curve_x24_syn_dadd_0(_S541, _S534);
        _S535 = _S540;
        _S536 = _S542;
    }
    else
    {
        _S535 = 0.0f;
        _S536 = _S534;
    }
    (*_S522) = _S535;
    (*_S523) = _S536;
    return;
}

fn s_bwdProp_sanitised_0( _S543 : s_bwdCallableCtx_sanitised_0,  _S544 : ptr<function, vec3<f32>>,  _S545 : vec3<f32>)
{
    var _S546 : f32 = _S543.value0_45.value0_37.x;
    var _S547 : f32 = _S543.value0_45.value0_37.y;
    var _S548 : Tuple_14 = s_apply_min_0(_S546, _S547);
    var _S549 : s_bwdCallableCtx_min_0;
    _S549._S106 = _S546;
    _S549._S107 = _S547;
    var _S550 : s_bwdCallableCtx_min_0 = _S549;
    var _S551 : f32 = _S543.value0_45.value0_37.z;
    var _S552 : Tuple_14 = s_apply_min_0(_S548.value0_16, _S551);
    var _S553 : s_bwdCallableCtx_min_0;
    _S553._S106 = _S548.value0_16;
    _S553._S107 = _S551;
    var _S554 : s_bwdCallableCtx_min_0 = _S553;
    var _S555 : f32;
    if(((_S552.value0_16) < 0.0f))
    {
        var _S556 : vec3<f32> = (vec3<f32>(0) - _S545);
        _S555 = _S556[i32(0)] + _S556[i32(1)] + _S556[i32(2)];
    }
    else
    {
        _S555 = 0.0f;
    }
    var _S557 : f32 = 0.0f;
    var _S558 : f32 = 0.0f;
    s_bwdProp_min_0(_S554, &(_S557), &(_S558), _S555);
    var _S559 : f32 = 0.0f;
    var _S560 : f32 = 0.0f;
    s_bwdProp_min_0(_S550, &(_S559), &(_S560), _S557);
    (*_S544) = _S545 + vec3<f32>(_S559, _S560, _S558);
    return;
}

fn s_bwdProp_chroma_v2_v3_0( _S561 : s_bwdCallableCtx_chroma_v2_v3_0,  _S562 : ptr<function, vec3<f32>>,  _S563 : ptr<function, Curve_0>,  _S564 : vec3<f32>)
{
    var _S565 : vec3<f32> = vec3<f32>(0.0f);
    var _S566 : s_paramCtx_s_bwdCallableCtx_pixel_norm_0 = s_paramCtx_s_bwdCallableCtx_pixel_norm_0( _S565, u32(0), _S565 );
    var _S567 : s_paramCtx_s_bwdCallableCtx_sanitised_0 = s_paramCtx_s_bwdCallableCtx_sanitised_0( _S565 );
    var _S568 : vec4<f32> = vec4<f32>(0.0f);
    var _S569 : Curve_0 = Curve_0( _S568, _S568, _S568, _S568, _S568, _S568, _S568 );
    var _S570 : s_paramCtx_s_bwdCallableCtx_log_v2_0 = s_paramCtx_s_bwdCallableCtx_log_v2_0( 0.0f, _S569, false );
    var _S571 : s_paramCtx_s_bwdCallableCtx_desaturate_v2_0 = s_paramCtx_s_bwdCallableCtx_desaturate_v2_0( 0.0f, _S569 );
    var _S572 : s_paramCtx_s_bwdCallableCtx_spline_0 = s_paramCtx_s_bwdCallableCtx_spline_0( 0.0f, _S569, u32(0), u32(0) );
    var _S573 : bool = !((_S561.value0_35.value1_7.tone_0.z) < 0.0f);
    var norm_2 : f32;
    var _S574 : f32;
    var _S575 : f32;
    var _S576 : f32;
    var _S577 : f32;
    var _S578 : f32;
    var _S579 : f32;
    var _S580 : s_bwdCallableCtx_max_1;
    var _S581 : s_bwdCallableCtx_max_1;
    var _S582 : s_bwdCallableCtx_max_1;
    var _S583 : s_bwdCallableCtx_max_1;
    var _S584 : s_bwdCallableCtx_pixel_norm_0;
    var _S585 : s_bwdCallableCtx_pixel_norm_0;
    var _S586 : s_bwdCallableCtx_clamp_1;
    var _S587 : vec3<f32>;
    var _S588 : vec3<f32>;
    var _S589 : vec3<f32>;
    var _S590 : s_bwdCallableCtx_max_0;
    var _S591 : s_bwdCallableCtx_max_0;
    var _S592 : bool;
    var _S593 : bool;
    var _S594 : s_bwdCallableCtx_pow0_0;
    var _S595 : s_bwdCallableCtx_clamp_0;
    var _S596 : s_bwdCallableCtx_spline_0;
    var _S597 : s_bwdCallableCtx_desaturate_v2_0;
    var _S598 : s_bwdCallableCtx_log_v2_0;
    var _S599 : s_bwdCallableCtx_sanitised_0;
    if(_S573)
    {
        var _S600 : Tuple_16 = s_apply_pixel_norm_0(_S561.value0_35.value0_29, _S561.value0_35.value2_7.norm_0, _S561.value0_35.value2_7.luminance_1);
        var _S601 : s_paramCtx_s_bwdCallableCtx_pixel_norm_0;
        _S601.value0_36 = _S561.value0_35.value0_29;
        _S601.value1_8 = _S561.value0_35.value2_7.norm_0;
        _S601.value2_8 = _S561.value0_35.value2_7.luminance_1;
        var _S602 : s_paramCtx_s_bwdCallableCtx_pixel_norm_0 = _S601;
        var _S603 : Tuple_13 = s_apply_max_1(_S600.value0_46, 1.52587890625e-05f);
        var _S604 : s_bwdCallableCtx_max_1;
        _S604._S101 = _S600.value0_46;
        _S604._S102 = 1.52587890625e-05f;
        var _S605 : s_bwdCallableCtx_max_1 = _S604;
        var _S606 : vec3<f32> = _S561.value0_35.value0_29 / vec3<f32>(_S603.value0_15);
        var _S607 : f32 = _S603.value0_15 * _S603.value0_15;
        var _S608 : Tuple_17 = s_apply_sanitised_0(_S606);
        var _S609 : s_paramCtx_s_bwdCallableCtx_sanitised_0;
        _S609.value0_37 = _S606;
        var _S610 : s_paramCtx_s_bwdCallableCtx_sanitised_0 = _S609;
        var _S611 : Tuple_18 = s_apply_log_v2_0(_S603.value0_15, _S561.value0_35.value1_7, _S561.value0_35.value2_7.white_0);
        var _S612 : s_paramCtx_s_bwdCallableCtx_log_v2_0;
        _S612.value0_38 = _S603.value0_15;
        _S612.value1_9 = _S561.value0_35.value1_7;
        _S612.value2_9 = _S561.value0_35.value2_7.white_0;
        var _S613 : s_paramCtx_s_bwdCallableCtx_log_v2_0 = _S612;
        var _S614 : Tuple_20 = s_apply_desaturate_v2_0(_S611.value0_48, _S561.value0_35.value1_7);
        var _S615 : s_paramCtx_s_bwdCallableCtx_desaturate_v2_0;
        _S615.value0_39 = _S611.value0_48;
        _S615.value1_10 = _S561.value0_35.value1_7;
        var _S616 : s_paramCtx_s_bwdCallableCtx_desaturate_v2_0 = _S615;
        var _S617 : Tuple_21 = s_apply_spline_0(_S611.value0_48, _S561.value0_35.value1_7, _S561.value0_35.value2_7.toe_1, _S561.value0_35.value2_7.shoulder_1);
        var _S618 : s_paramCtx_s_bwdCallableCtx_spline_0;
        _S618.value0_40 = _S611.value0_48;
        _S618.value1_11 = _S561.value0_35.value1_7;
        _S618.value2_10 = _S561.value0_35.value2_7.toe_1;
        _S618.value3_1 = _S561.value0_35.value2_7.shoulder_1;
        var _S619 : s_paramCtx_s_bwdCallableCtx_spline_0 = _S618;
        var _S620 : Tuple_9 = s_apply_clamp_0(_S617.value0_51, 0.0f, 1.0f);
        var _S621 : s_bwdCallableCtx_clamp_0;
        _S621._S65 = _S617.value0_51;
        _S621._S66 = 0.0f;
        _S621._S67 = 1.0f;
        var _S622 : s_bwdCallableCtx_clamp_0 = _S621;
        var _S623 : f32 = _S561.value0_35.value1_7.tone_0.y;
        var _S624 : Tuple_8 = s_apply_pow0_0(_S620.value0_11, _S623);
        var _S625 : s_bwdCallableCtx_pow0_0;
        _S625._S60 = _S620.value0_11;
        _S625._S61 = _S623;
        var _S626 : s_bwdCallableCtx_pow0_0 = _S625;
        var _S627 : vec3<f32> = vec3<f32>(1.0f) - _S608.value0_47;
        var _S628 : f32 = 1.0f - _S614.value0_50;
        var _S629 : vec3<f32> = _S608.value0_47 + _S627 * vec3<f32>(_S628);
        var _S630 : vec3<f32> = vec3<f32>(0.0f);
        var _S631 : Tuple_7 = s_apply_max_0(_S629, _S630);
        var _S632 : s_bwdCallableCtx_max_0;
        _S632._S52 = _S629;
        _S632._S53 = _S630;
        var _S633 : s_bwdCallableCtx_max_0 = _S632;
        var _S634 : bool = (_S561.value0_35.value2_7.version_0) == u32(2);
        if(_S634)
        {
            var _S635 : Tuple_16 = s_apply_pixel_norm_0(_S631.value0_9, _S561.value0_35.value2_7.norm_0, _S561.value0_35.value2_7.luminance_1);
            var _S636 : s_paramCtx_s_bwdCallableCtx_pixel_norm_0;
            _S636.value0_36 = _S631.value0_9;
            _S636.value1_8 = _S561.value0_35.value2_7.norm_0;
            _S636.value2_8 = _S561.value0_35.value2_7.luminance_1;
            var _S637 : Tuple_13 = s_apply_max_1(_S635.value0_46, 1.52587890625e-05f);
            var _S638 : s_bwdCallableCtx_max_1;
            _S638._S101 = _S635.value0_46;
            _S638._S102 = 1.52587890625e-05f;
            var _S639 : f32 = _S637.value0_15 * _S637.value0_15;
            norm_2 = _S624.value0_10 / _S637.value0_15;
            _S574 = _S639;
            _S575 = _S637.value0_15;
            _S580 = _S638;
            _S584.value0_41 = _S636;
        }
        else
        {
            norm_2 = _S624.value0_10;
            _S574 = 0.0f;
            _S575 = 0.0f;
            _S580._S101 = 0.0f;
            _S580._S102 = 0.0f;
            _S584.value0_41 = _S566;
        }
        var out_0 : vec3<f32> = _S631.value0_9 * vec3<f32>(norm_2);
        var _S640 : f32 = out_0.x;
        var _S641 : f32 = out_0.y;
        var _S642 : Tuple_13 = s_apply_max_1(_S640, _S641);
        var _S643 : s_bwdCallableCtx_max_1;
        _S643._S101 = _S640;
        _S643._S102 = _S641;
        var _S644 : s_bwdCallableCtx_max_1 = _S643;
        var _S645 : f32 = out_0.z;
        var _S646 : Tuple_13 = s_apply_max_1(_S642.value0_15, _S645);
        var _S647 : s_bwdCallableCtx_max_1;
        _S647._S101 = _S642.value0_15;
        _S647._S102 = _S645;
        var _S648 : s_bwdCallableCtx_max_1 = _S647;
        var _S649 : bool = ((_S646.value0_15) > 1.0f);
        if(_S649)
        {
            var _S650 : vec3<f32> = _S631.value0_9 + vec3<f32>((1.0f - _S646.value0_15));
            var _S651 : Tuple_7 = s_apply_max_0(_S650, _S630);
            var _S652 : s_bwdCallableCtx_max_0;
            _S652._S52 = _S650;
            _S652._S53 = _S630;
            var _S653 : vec3<f32> = vec3<f32>(1.0f);
            var _S654 : s_bwdCallableCtx_clamp_1;
            _S654._S89 = _S651.value0_9 * vec3<f32>(norm_2);
            _S654._S90 = _S630;
            _S654._S91 = _S653;
            _S586 = _S654;
            _S587 = _S651.value0_9;
            _S590 = _S652;
        }
        else
        {
            _S586._S89 = _S565;
            _S586._S90 = _S565;
            _S586._S91 = _S565;
            _S587 = _S565;
            _S590._S52 = _S565;
            _S590._S53 = _S565;
        }
        var _S655 : f32 = _S575;
        var _S656 : s_bwdCallableCtx_max_1 = _S580;
        _S592 = _S649;
        _S580 = _S648;
        _S581 = _S644;
        _S588 = _S631.value0_9;
        _S593 = _S634;
        _S575 = _S624.value0_10;
        _S576 = _S655;
        _S582 = _S656;
        _S591 = _S633;
        _S589 = _S627;
        _S577 = _S628;
        _S594 = _S626;
        _S595 = _S622;
        _S596.value0_42 = _S619;
        _S597.value0_43 = _S616;
        _S598.value0_44 = _S613;
        _S599.value0_45 = _S610;
        _S578 = _S607;
        _S579 = _S603.value0_15;
        _S583 = _S605;
        _S585.value0_41 = _S602;
    }
    else
    {
        _S592 = false;
        _S586._S89 = _S565;
        _S586._S90 = _S565;
        _S586._S91 = _S565;
        _S587 = _S565;
        norm_2 = 0.0f;
        _S590._S52 = _S565;
        _S590._S53 = _S565;
        _S580._S101 = 0.0f;
        _S580._S102 = 0.0f;
        _S581._S101 = 0.0f;
        _S581._S102 = 0.0f;
        _S588 = _S565;
        _S593 = false;
        _S574 = 0.0f;
        _S575 = 0.0f;
        _S576 = 0.0f;
        _S582._S101 = 0.0f;
        _S582._S102 = 0.0f;
        _S584.value0_41 = _S566;
        _S591._S52 = _S565;
        _S591._S53 = _S565;
        _S589 = _S565;
        _S577 = 0.0f;
        _S594._S60 = 0.0f;
        _S594._S61 = 0.0f;
        _S595._S65 = 0.0f;
        _S595._S66 = 0.0f;
        _S595._S67 = 0.0f;
        _S596.value0_42 = _S572;
        _S597.value0_43 = _S571;
        _S598.value0_44 = _S570;
        _S599.value0_45 = _S567;
        _S578 = 0.0f;
        _S579 = 0.0f;
        _S583._S101 = 0.0f;
        _S583._S102 = 0.0f;
        _S585.value0_41 = _S566;
    }
    var _S657 : Curve_0 = Curve_x24_syn_dzero_0();
    var _S658 : vec4<f32>;
    var _S659 : Curve_0;
    if(_S573)
    {
        var _S660 : f32;
        var _S661 : f32;
        var _S662 : vec3<f32>;
        if(_S592)
        {
            var _S663 : vec3<f32> = _S565;
            var _S664 : vec3<f32> = _S565;
            var _S665 : vec3<f32> = _S565;
            s_bwdProp_clamp_1(_S586, &(_S663), &(_S664), &(_S665), _S564);
            var _S666 : vec3<f32> = _S587 * _S663;
            var _S667 : f32 = _S666[i32(0)] + _S666[i32(1)] + _S666[i32(2)];
            var _S668 : vec3<f32> = vec3<f32>(norm_2) * _S663;
            var _S669 : vec3<f32> = _S565;
            var _S670 : vec3<f32> = _S565;
            s_bwdProp_max_0(_S590, &(_S669), &(_S670), _S668);
            _S660 = - (_S669[i32(0)] + _S669[i32(1)] + _S669[i32(2)]);
            _S587 = _S565;
            _S661 = _S667;
            _S662 = _S669;
        }
        else
        {
            _S660 = 0.0f;
            _S587 = _S564;
            _S661 = 0.0f;
            _S662 = _S565;
        }
        var _S671 : f32 = 0.0f;
        var _S672 : f32 = 0.0f;
        s_bwdProp_max_1(_S580, &(_S671), &(_S672), _S660);
        var _S673 : f32 = 0.0f;
        var _S674 : f32 = 0.0f;
        s_bwdProp_max_1(_S581, &(_S673), &(_S674), _S671);
        var _S675 : vec3<f32> = _S587 + vec3<f32>(_S673, _S674, _S672);
        var _S676 : vec3<f32> = _S588 * _S675;
        var _S677 : f32 = _S676[i32(0)] + _S676[i32(1)] + _S676[i32(2)] + _S661;
        var _S678 : vec3<f32> = vec3<f32>(norm_2) * _S675 + _S662;
        if(_S593)
        {
            var _S679 : f32 = _S677 / _S574;
            var _S680 : f32 = _S575 * - _S679;
            var _S681 : f32 = _S576 * _S679;
            var _S682 : f32 = 0.0f;
            var _S683 : f32 = 0.0f;
            s_bwdProp_max_1(_S582, &(_S682), &(_S683), _S680);
            var _S684 : vec3<f32> = _S565;
            s_bwdProp_pixel_norm_0(_S584, &(_S684), _S682);
            _S587 = _S684 + _S678;
            norm_2 = _S681;
        }
        else
        {
            _S587 = _S678;
            norm_2 = _S677;
        }
        var _S685 : vec3<f32> = _S565;
        var _S686 : vec3<f32> = _S565;
        s_bwdProp_max_0(_S591, &(_S685), &(_S686), _S587);
        var _S687 : vec3<f32> = _S589 * _S685;
        var _S688 : f32 = - (_S687[i32(0)] + _S687[i32(1)] + _S687[i32(2)]);
        var _S689 : vec3<f32> = (vec3<f32>(0) - vec3<f32>(_S577) * _S685);
        var _S690 : f32 = 0.0f;
        var _S691 : f32 = 0.0f;
        s_bwdProp_pow0_0(_S594, &(_S690), &(_S691), norm_2);
        var _S692 : f32 = 0.0f;
        var _S693 : f32 = 0.0f;
        var _S694 : f32 = 0.0f;
        s_bwdProp_clamp_0(_S595, &(_S692), &(_S693), &(_S694), _S690);
        var _S695 : f32 = 0.0f;
        var _S696 : Curve_0;
        _S696.toe_0 = _S568;
        _S696.shoulder_0 = _S568;
        _S696.line_0 = _S568;
        _S696.nodes_0 = _S568;
        _S696.tone_0 = _S568;
        _S696.display_0 = _S568;
        _S696.extra_0 = _S568;
        s_bwdProp_spline_0(_S596, &(_S695), &(_S696), _S692);
        var _S697 : f32 = 0.0f;
        var _S698 : Curve_0;
        _S698.toe_0 = _S568;
        _S698.shoulder_0 = _S568;
        _S698.line_0 = _S568;
        _S698.nodes_0 = _S568;
        _S698.tone_0 = _S568;
        _S698.display_0 = _S568;
        _S698.extra_0 = _S568;
        s_bwdProp_desaturate_v2_0(_S597, &(_S697), &(_S698), _S688);
        var _S699 : f32 = _S695 + _S697;
        var _S700 : f32 = 0.0f;
        var _S701 : Curve_0;
        _S701.toe_0 = _S568;
        _S701.shoulder_0 = _S568;
        _S701.line_0 = _S568;
        _S701.nodes_0 = _S568;
        _S701.tone_0 = _S568;
        _S701.display_0 = _S568;
        _S701.extra_0 = _S568;
        s_bwdProp_log_v2_0(_S598, &(_S700), &(_S701), _S699);
        var _S702 : vec3<f32> = _S685 + _S689;
        var _S703 : vec3<f32> = _S565;
        s_bwdProp_sanitised_0(_S599, &(_S703), _S702);
        var _S704 : vec3<f32> = _S703 / vec3<f32>(_S578);
        var _S705 : vec3<f32> = _S561.value0_35.value0_29 * (vec3<f32>(0) - _S704);
        var _S706 : vec3<f32> = vec3<f32>(_S579) * _S704;
        var _S707 : f32 = _S700 + (_S705[i32(0)] + _S705[i32(1)] + _S705[i32(2)]);
        var _S708 : f32 = 0.0f;
        var _S709 : f32 = 0.0f;
        s_bwdProp_max_1(_S583, &(_S708), &(_S709), _S707);
        var _S710 : vec3<f32> = _S565;
        s_bwdProp_pixel_norm_0(_S585, &(_S710), _S708);
        var _S711 : vec3<f32> = _S706 + _S710;
        var _S712 : Curve_0 = Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(_S696, _S698), _S701), _S657);
        _S658 = vec4<f32>(0.0f, _S691, 0.0f, 0.0f);
        _S659 = _S712;
        _S587 = _S711;
    }
    else
    {
        _S658 = _S568;
        _S659 = _S657;
        _S587 = _S565;
    }
    var _S713 : Curve_0 = _S657;
    _S713.tone_0 = _S658;
    var _S714 : Curve_0 = Curve_x24_syn_dadd_0(_S659, _S713);
    (*_S562) = _S587;
    (*_S563) = _S714;
    return;
}

struct Tuple_22
{
     value0_54 : f32,
};

fn s_apply_log_v1_0( _S715 : f32,  _S716 : Curve_0,  white_2 : bool) -> Tuple_22
{
    var _S717 : f32;
    if(!white_2)
    {
        _S717 = s_apply_clamp_0(s_apply_log_encoding_0(_S715, _S716).value0_49, 1.52587890625e-05f, 1.0f).value0_11;
    }
    else
    {
        _S717 = 1.0f;
    }
    var _S718 : Tuple_22;
    _S718.value0_54 = _S717;
    return _S718;
}

struct s_paramCtx_s_bwdCallableCtx_log_v1_0
{
     value0_55 : f32,
     value1_13 : Curve_0,
     value2_11 : bool,
};

struct s_bwdCallableCtx_log_v1_0
{
     value0_56 : s_paramCtx_s_bwdCallableCtx_log_v1_0,
};

struct Tuple_23
{
     value0_57 : f32,
};

struct Tuple_24
{
     value0_58 : f32,
};

fn s_apply_away_from_zero_0( _S719 : f32,  floor_0 : f32) -> Tuple_24
{
    var _S720 : f32;
    var _runFlag_7 : bool;
    if(((s_apply_abs_1(_S719).value0_8) < floor_0))
    {
        if((_S719 < 0.0f))
        {
            _S720 = - floor_0;
        }
        else
        {
            _S720 = floor_0;
        }
        _runFlag_7 = false;
    }
    else
    {
        _runFlag_7 = true;
    }
    if(_runFlag_7)
    {
        _S720 = _S719;
    }
    var _S721 : Tuple_24;
    _S721.value0_58 = _S720;
    return _S721;
}

fn s_apply_desaturate_v1_0( _S722 : f32,  _S723 : Curve_0) -> Tuple_23
{
    var _S724 : f32 = 1.0f - _S722;
    var _S725 : Tuple_23;
    _S725.value0_57 = 1.0f - s_apply_clamp_0((s_apply_exp_0(-0.5f * _S722 * _S722 / s_apply_max_1(_S723.tone_0.w, 1.17549435082228751e-38f).value0_15).value0_6 + s_apply_exp_0(-0.5f * _S724 * _S724 / s_apply_max_1(_S723.display_0.x, 1.17549435082228751e-38f).value0_15).value0_6) / s_apply_away_from_zero_0(_S723.tone_0.z, 1.00000000317107685e-30f).value0_58, 0.0f, 1.0f).value0_11;
    return _S725;
}

struct s_paramCtx_s_bwdCallableCtx_desaturate_v1_0
{
     value0_59 : f32,
     value1_14 : Curve_0,
};

struct s_bwdCallableCtx_desaturate_v1_0
{
     value0_60 : s_paramCtx_s_bwdCallableCtx_desaturate_v1_0,
};

struct s_paramCtx_s_bwdCallableCtx_away_from_zero_0
{
     value0_61 : f32,
     value1_15 : f32,
};

struct s_bwdCallableCtx_away_from_zero_0
{
     value0_62 : s_paramCtx_s_bwdCallableCtx_away_from_zero_0,
};

fn s_bwdProp_away_from_zero_0( _S726 : s_bwdCallableCtx_away_from_zero_0,  _S727 : ptr<function, f32>,  _S728 : f32)
{
    var _S729 : Tuple_6 = s_apply_abs_1(_S726.value0_62.value0_61);
    var _S730 : s_bwdCallableCtx_abs_1;
    _S730._S48 = _S726.value0_62.value0_61;
    var _S731 : s_bwdCallableCtx_abs_1 = _S730;
    var _S732 : f32;
    if(!((_S729.value0_8) < (_S726.value0_62.value1_15)))
    {
        _S732 = _S728;
    }
    else
    {
        _S732 = 0.0f;
    }
    var _S733 : f32 = 0.0f;
    s_bwdProp_abs_1(_S731, &(_S733), 0.0f);
    (*_S727) = _S733 + _S732;
    return;
}

fn s_bwdProp_desaturate_v1_0( _S734 : s_bwdCallableCtx_desaturate_v1_0,  _S735 : ptr<function, f32>,  _S736 : ptr<function, Curve_0>,  _S737 : f32)
{
    var _S738 : f32 = -0.5f * _S734.value0_60.value0_59;
    var _S739 : f32 = _S738 * _S734.value0_60.value0_59;
    var _S740 : f32 = _S734.value0_60.value1_14.tone_0.w;
    var _S741 : Tuple_13 = s_apply_max_1(_S740, 1.17549435082228751e-38f);
    var _S742 : s_bwdCallableCtx_max_1;
    _S742._S101 = _S740;
    _S742._S102 = 1.17549435082228751e-38f;
    var _S743 : f32 = _S739 / _S741.value0_15;
    var _S744 : f32 = _S741.value0_15 * _S741.value0_15;
    var _S745 : Tuple_4 = s_apply_exp_0(_S743);
    var _S746 : s_bwdCallableCtx_exp_0;
    _S746._S36 = _S743;
    var _S747 : f32 = 1.0f - _S734.value0_60.value0_59;
    var _S748 : f32 = -0.5f * _S747;
    var _S749 : f32 = _S748 * _S747;
    var _S750 : f32 = _S734.value0_60.value1_14.display_0.x;
    var _S751 : Tuple_13 = s_apply_max_1(_S750, 1.17549435082228751e-38f);
    var _S752 : s_bwdCallableCtx_max_1;
    _S752._S101 = _S750;
    _S752._S102 = 1.17549435082228751e-38f;
    var _S753 : f32 = _S749 / _S751.value0_15;
    var _S754 : f32 = _S751.value0_15 * _S751.value0_15;
    var _S755 : Tuple_4 = s_apply_exp_0(_S753);
    var _S756 : s_bwdCallableCtx_exp_0;
    _S756._S36 = _S753;
    var _S757 : f32 = _S745.value0_6 + _S755.value0_6;
    var _S758 : f32 = _S734.value0_60.value1_14.tone_0.z;
    var _S759 : Tuple_24 = s_apply_away_from_zero_0(_S758, 1.00000000317107685e-30f);
    var _S760 : s_paramCtx_s_bwdCallableCtx_away_from_zero_0;
    _S760.value0_61 = _S758;
    _S760.value1_15 = 1.00000000317107685e-30f;
    var _S761 : s_bwdCallableCtx_away_from_zero_0 = s_bwdCallableCtx_away_from_zero_0( _S760 );
    var _S762 : f32 = _S759.value0_58 * _S759.value0_58;
    var _S763 : s_bwdCallableCtx_clamp_0;
    _S763._S65 = _S757 / _S759.value0_58;
    _S763._S66 = 0.0f;
    _S763._S67 = 1.0f;
    var _S764 : Curve_0 = Curve_x24_syn_dzero_0();
    var _S765 : f32 = - _S737;
    var _S766 : f32 = 0.0f;
    var _S767 : f32 = 0.0f;
    var _S768 : f32 = 0.0f;
    s_bwdProp_clamp_0(_S763, &(_S766), &(_S767), &(_S768), _S765);
    var _S769 : f32 = _S766 / _S762;
    var _S770 : f32 = _S757 * - _S769;
    var _S771 : f32 = _S759.value0_58 * _S769;
    var _S772 : f32 = 0.0f;
    s_bwdProp_away_from_zero_0(_S761, &(_S772), _S770);
    var _S773 : f32 = 0.0f;
    s_bwdProp_exp_0(_S756, &(_S773), _S771);
    var _S774 : f32 = _S773 / _S754;
    var _S775 : f32 = _S749 * - _S774;
    var _S776 : f32 = _S751.value0_15 * _S774;
    var _S777 : f32 = 0.0f;
    var _S778 : f32 = 0.0f;
    s_bwdProp_max_1(_S752, &(_S777), &(_S778), _S775);
    var _S779 : vec4<f32> = vec4<f32>(_S777, 0.0f, 0.0f, 0.0f);
    var _S780 : f32 = - (_S748 * _S776 + -0.5f * (_S747 * _S776));
    var _S781 : f32 = 0.0f;
    s_bwdProp_exp_0(_S746, &(_S781), _S771);
    var _S782 : f32 = _S781 / _S744;
    var _S783 : f32 = _S739 * - _S782;
    var _S784 : f32 = _S741.value0_15 * _S782;
    var _S785 : f32 = 0.0f;
    var _S786 : f32 = 0.0f;
    s_bwdProp_max_1(_S742, &(_S785), &(_S786), _S783);
    var _S787 : vec4<f32> = vec4<f32>(0.0f, 0.0f, _S772, _S785);
    var _S788 : f32 = _S738 * _S784;
    var _S789 : f32 = -0.5f * (_S734.value0_60.value0_59 * _S784);
    var _S790 : Curve_0 = _S764;
    _S790.display_0 = _S779;
    _S790.tone_0 = _S787;
    var _S791 : Curve_0 = Curve_x24_syn_dadd_0(_S764, _S790);
    (*_S735) = _S780 + _S788 + _S789;
    (*_S736) = _S791;
    return;
}

fn s_bwdProp_log_v1_0( _S792 : s_bwdCallableCtx_log_v1_0,  _S793 : ptr<function, f32>,  _S794 : ptr<function, Curve_0>,  _S795 : f32)
{
    var _S796 : vec4<f32> = vec4<f32>(0.0f);
    var _S797 : Curve_0 = Curve_0( _S796, _S796, _S796, _S796, _S796, _S796, _S796 );
    var _S798 : s_paramCtx_s_bwdCallableCtx_log_encoding_0 = s_paramCtx_s_bwdCallableCtx_log_encoding_0( 0.0f, _S797 );
    var _S799 : bool = !_S792.value0_56.value2_11;
    var _S800 : s_bwdCallableCtx_clamp_0;
    var _S801 : s_bwdCallableCtx_log_encoding_0;
    if(_S799)
    {
        var _S802 : Tuple_19 = s_apply_log_encoding_0(_S792.value0_56.value0_55, _S792.value0_56.value1_13);
        var _S803 : s_paramCtx_s_bwdCallableCtx_log_encoding_0;
        _S803.value0_52 = _S792.value0_56.value0_55;
        _S803.value1_12 = _S792.value0_56.value1_13;
        var _S804 : s_bwdCallableCtx_clamp_0;
        _S804._S65 = _S802.value0_49;
        _S804._S66 = 1.52587890625e-05f;
        _S804._S67 = 1.0f;
        _S800 = _S804;
        _S801.value0_53 = _S803;
    }
    else
    {
        _S800._S65 = 0.0f;
        _S800._S66 = 0.0f;
        _S800._S67 = 0.0f;
        _S801.value0_53 = _S798;
    }
    var _S805 : Curve_0 = Curve_x24_syn_dzero_0();
    var _S806 : f32;
    var _S807 : Curve_0;
    if(_S799)
    {
        var _S808 : f32 = 0.0f;
        var _S809 : f32 = 0.0f;
        var _S810 : f32 = 0.0f;
        s_bwdProp_clamp_0(_S800, &(_S808), &(_S809), &(_S810), _S795);
        var _S811 : f32 = 0.0f;
        var _S812 : Curve_0;
        _S812.toe_0 = _S796;
        _S812.shoulder_0 = _S796;
        _S812.line_0 = _S796;
        _S812.nodes_0 = _S796;
        _S812.tone_0 = _S796;
        _S812.display_0 = _S796;
        _S812.extra_0 = _S796;
        s_bwdProp_log_encoding_0(_S801, &(_S811), &(_S812), _S808);
        var _S813 : Curve_0 = Curve_x24_syn_dadd_0(_S812, _S805);
        _S806 = _S811;
        _S807 = _S813;
    }
    else
    {
        _S806 = 0.0f;
        _S807 = _S805;
    }
    (*_S793) = _S806;
    (*_S794) = _S807;
    return;
}

fn s_bwdProp_chroma_v1_0( _S814 : s_bwdCallableCtx_chroma_v1_0,  _S815 : ptr<function, vec3<f32>>,  _S816 : ptr<function, Curve_0>,  _S817 : vec3<f32>)
{
    var _S818 : Tuple_16 = s_apply_pixel_norm_0(_S814.value0_34.value0_28, _S814.value0_34.value2_6.norm_0, _S814.value0_34.value2_6.luminance_1);
    var _S819 : s_paramCtx_s_bwdCallableCtx_pixel_norm_0;
    _S819.value0_36 = _S814.value0_34.value0_28;
    _S819.value1_8 = _S814.value0_34.value2_6.norm_0;
    _S819.value2_8 = _S814.value0_34.value2_6.luminance_1;
    var _S820 : s_bwdCallableCtx_pixel_norm_0 = s_bwdCallableCtx_pixel_norm_0( _S819 );
    var _S821 : Tuple_13 = s_apply_max_1(_S818.value0_46, 1.52587890625e-05f);
    var _S822 : s_bwdCallableCtx_max_1;
    _S822._S101 = _S818.value0_46;
    _S822._S102 = 1.52587890625e-05f;
    var _S823 : vec3<f32> = vec3<f32>(_S821.value0_15);
    var _S824 : vec3<f32> = _S814.value0_34.value0_28 / _S823;
    var _S825 : f32 = _S821.value0_15 * _S821.value0_15;
    var _S826 : Tuple_17 = s_apply_sanitised_0(_S824);
    var _S827 : s_paramCtx_s_bwdCallableCtx_sanitised_0;
    _S827.value0_37 = _S824;
    var _S828 : s_bwdCallableCtx_sanitised_0 = s_bwdCallableCtx_sanitised_0( _S827 );
    var _S829 : Tuple_22 = s_apply_log_v1_0(_S821.value0_15, _S814.value0_34.value1_6, _S814.value0_34.value2_6.white_0);
    var _S830 : s_paramCtx_s_bwdCallableCtx_log_v1_0;
    _S830.value0_55 = _S821.value0_15;
    _S830.value1_13 = _S814.value0_34.value1_6;
    _S830.value2_11 = _S814.value0_34.value2_6.white_0;
    var _S831 : s_bwdCallableCtx_log_v1_0 = s_bwdCallableCtx_log_v1_0( _S830 );
    var _S832 : Tuple_23 = s_apply_desaturate_v1_0(_S829.value0_54, _S814.value0_34.value1_6);
    var _S833 : s_paramCtx_s_bwdCallableCtx_desaturate_v1_0;
    _S833.value0_59 = _S829.value0_54;
    _S833.value1_14 = _S814.value0_34.value1_6;
    var _S834 : s_bwdCallableCtx_desaturate_v1_0 = s_bwdCallableCtx_desaturate_v1_0( _S833 );
    var _S835 : vec3<f32> = vec3<f32>(_S829.value0_54);
    var ratios_0 : vec3<f32> = _S826.value0_47 * _S835;
    var _S836 : Tuple_12 = s_apply_dot_0(_S814.value0_34.value2_6.luminance_1, ratios_0);
    var _S837 : s_bwdCallableCtx_dot_0;
    _S837._S96 = _S814.value0_34.value2_6.luminance_1;
    _S837._S97 = ratios_0;
    var _S838 : vec3<f32> = vec3<f32>(_S836.value0_14);
    var _S839 : vec3<f32> = ratios_0 - _S838;
    var _S840 : vec3<f32> = vec3<f32>(_S832.value0_57);
    var _S841 : vec3<f32> = _S838 + _S840 * _S839;
    var ratios_1 : vec3<f32> = _S841 / _S835;
    var _S842 : f32 = _S829.value0_54 * _S829.value0_54;
    var _S843 : Tuple_21 = s_apply_spline_0(_S829.value0_54, _S814.value0_34.value1_6, _S814.value0_34.value2_6.toe_1, _S814.value0_34.value2_6.shoulder_1);
    var _S844 : s_paramCtx_s_bwdCallableCtx_spline_0;
    _S844.value0_40 = _S829.value0_54;
    _S844.value1_11 = _S814.value0_34.value1_6;
    _S844.value2_10 = _S814.value0_34.value2_6.toe_1;
    _S844.value3_1 = _S814.value0_34.value2_6.shoulder_1;
    var _S845 : s_bwdCallableCtx_spline_0 = s_bwdCallableCtx_spline_0( _S844 );
    var _S846 : Tuple_9 = s_apply_clamp_0(_S843.value0_51, 0.0f, 1.0f);
    var _S847 : s_bwdCallableCtx_clamp_0;
    _S847._S65 = _S843.value0_51;
    _S847._S66 = 0.0f;
    _S847._S67 = 1.0f;
    var _S848 : f32 = _S814.value0_34.value1_6.tone_0.y;
    var _S849 : Tuple_8 = s_apply_pow0_0(_S846.value0_11, _S848);
    var _S850 : s_bwdCallableCtx_pow0_0;
    _S850._S60 = _S846.value0_11;
    _S850._S61 = _S848;
    var _S851 : Curve_0 = Curve_x24_syn_dzero_0();
    var _S852 : vec3<f32> = ratios_1 * _S817;
    var _S853 : f32 = _S852[i32(0)] + _S852[i32(1)] + _S852[i32(2)];
    var s_diff_ratios_T_0 : vec3<f32> = vec3<f32>(_S849.value0_10) * _S817;
    var _S854 : f32 = 0.0f;
    var _S855 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S850, &(_S854), &(_S855), _S853);
    var _S856 : vec4<f32> = vec4<f32>(0.0f, _S855, 0.0f, 0.0f);
    var _S857 : f32 = 0.0f;
    var _S858 : f32 = 0.0f;
    var _S859 : f32 = 0.0f;
    s_bwdProp_clamp_0(_S847, &(_S857), &(_S858), &(_S859), _S854);
    var _S860 : f32 = 0.0f;
    var _S861 : vec4<f32> = vec4<f32>(0.0f);
    var _S862 : Curve_0;
    _S862.toe_0 = _S861;
    _S862.shoulder_0 = _S861;
    _S862.line_0 = _S861;
    _S862.nodes_0 = _S861;
    _S862.tone_0 = _S861;
    _S862.display_0 = _S861;
    _S862.extra_0 = _S861;
    s_bwdProp_spline_0(_S845, &(_S860), &(_S862), _S857);
    var _S863 : vec3<f32> = s_diff_ratios_T_0 / vec3<f32>(_S842);
    var _S864 : vec3<f32> = _S841 * (vec3<f32>(0) - _S863);
    var _S865 : f32 = _S864[i32(0)] + _S864[i32(1)] + _S864[i32(2)];
    var _S866 : vec3<f32> = _S835 * _S863;
    var _S867 : vec3<f32> = _S840 * _S866;
    var _S868 : vec3<f32> = _S839 * _S866;
    var _S869 : f32 = _S868[i32(0)] + _S868[i32(1)] + _S868[i32(2)];
    var _S870 : vec3<f32> = (vec3<f32>(0) - _S867);
    var _S871 : f32 = _S866[i32(0)] + _S866[i32(1)] + _S866[i32(2)] + (_S870[i32(0)] + _S870[i32(1)] + _S870[i32(2)]);
    var _S872 : vec3<f32> = vec3<f32>(0.0f);
    var _S873 : vec3<f32> = _S872;
    var _S874 : vec3<f32> = _S872;
    s_bwdProp_dot_0(_S837, &(_S873), &(_S874), _S871);
    var _S875 : vec3<f32> = _S867 + _S874;
    var _S876 : vec3<f32> = _S826.value0_47 * _S875;
    var _S877 : f32 = _S876[i32(0)] + _S876[i32(1)] + _S876[i32(2)];
    var _S878 : vec3<f32> = _S835 * _S875;
    var _S879 : f32 = 0.0f;
    var _S880 : Curve_0;
    _S880.toe_0 = _S861;
    _S880.shoulder_0 = _S861;
    _S880.line_0 = _S861;
    _S880.nodes_0 = _S861;
    _S880.tone_0 = _S861;
    _S880.display_0 = _S861;
    _S880.extra_0 = _S861;
    s_bwdProp_desaturate_v1_0(_S834, &(_S879), &(_S880), _S869);
    var _S881 : f32 = _S860 + _S865 + _S877 + _S879;
    var _S882 : f32 = 0.0f;
    var _S883 : Curve_0;
    _S883.toe_0 = _S861;
    _S883.shoulder_0 = _S861;
    _S883.line_0 = _S861;
    _S883.nodes_0 = _S861;
    _S883.tone_0 = _S861;
    _S883.display_0 = _S861;
    _S883.extra_0 = _S861;
    s_bwdProp_log_v1_0(_S831, &(_S882), &(_S883), _S881);
    var _S884 : vec3<f32> = _S872;
    s_bwdProp_sanitised_0(_S828, &(_S884), _S878);
    var _S885 : vec3<f32> = _S884 / vec3<f32>(_S825);
    var _S886 : vec3<f32> = _S814.value0_34.value0_28 * (vec3<f32>(0) - _S885);
    var _S887 : vec3<f32> = _S823 * _S885;
    var _S888 : f32 = _S882 + (_S886[i32(0)] + _S886[i32(1)] + _S886[i32(2)]);
    var _S889 : f32 = 0.0f;
    var _S890 : f32 = 0.0f;
    s_bwdProp_max_1(_S822, &(_S889), &(_S890), _S888);
    var _S891 : vec3<f32> = _S872;
    s_bwdProp_pixel_norm_0(_S820, &(_S891), _S889);
    var _S892 : vec3<f32> = _S887 + _S891;
    var _S893 : Curve_0 = Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(_S862, _S880), _S883), _S851);
    var _S894 : Curve_0 = _S851;
    _S894.tone_0 = _S856;
    var _S895 : Curve_0 = Curve_x24_syn_dadd_0(_S893, _S894);
    (*_S815) = _S892;
    (*_S816) = _S895;
    return;
}

fn s_bwdProp_split_v2_v3_0( _S896 : s_bwdCallableCtx_split_v2_v3_0,  _S897 : ptr<function, vec3<f32>>,  _S898 : ptr<function, Curve_0>,  _S899 : vec3<f32>)
{
    var _S900 : vec4<f32> = vec4<f32>(0.0f);
    var _S901 : Curve_0 = Curve_0( _S900, _S900, _S900, _S900, _S900, _S900, _S900 );
    var _S902 : s_paramCtx_s_bwdCallableCtx_log_v2_0 = s_paramCtx_s_bwdCallableCtx_log_v2_0( 0.0f, _S901, false );
    var _S903 : vec3<f32> = vec3<f32>(0.0f);
    var _S904 : s_paramCtx_s_bwdCallableCtx_desaturate_v2_0 = s_paramCtx_s_bwdCallableCtx_desaturate_v2_0( 0.0f, _S901 );
    var _S905 : s_paramCtx_s_bwdCallableCtx_spline_0 = s_paramCtx_s_bwdCallableCtx_spline_0( 0.0f, _S901, u32(0), u32(0) );
    var _S906 : bool = !((_S896.value0_33.value1_5.tone_0.z) < 0.0f);
    var _S907 : s_bwdCallableCtx_pow_dt_0;
    var _S908 : s_bwdCallableCtx_pow_dt_0;
    var _S909 : s_bwdCallableCtx_pow_dt_0;
    var _S910 : s_bwdCallableCtx_clamp_1;
    var _S911 : s_bwdCallableCtx_spline_0;
    var _S912 : s_bwdCallableCtx_spline_0;
    var _S913 : s_bwdCallableCtx_spline_0;
    var _S914 : f32;
    var _S915 : vec3<f32>;
    var _S916 : s_bwdCallableCtx_desaturate_v2_0;
    var _S917 : s_bwdCallableCtx_dot_0;
    var _S918 : s_bwdCallableCtx_log_v2_0;
    var _S919 : s_bwdCallableCtx_log_v2_0;
    var _S920 : s_bwdCallableCtx_log_v2_0;
    var _S921 : s_bwdCallableCtx_max_1;
    var _S922 : s_bwdCallableCtx_max_1;
    var _S923 : s_bwdCallableCtx_max_1;
    if(_S906)
    {
        var _S924 : f32 = _S896.value0_33.value0_27.x;
        var _S925 : Tuple_13 = s_apply_max_1(_S924, 1.52587890625e-05f);
        var _S926 : s_bwdCallableCtx_max_1;
        _S926._S101 = _S924;
        _S926._S102 = 1.52587890625e-05f;
        var _S927 : Tuple_18 = s_apply_log_v2_0(_S925.value0_15, _S896.value0_33.value1_5, _S896.value0_33.value2_5.white_0);
        var _S928 : s_paramCtx_s_bwdCallableCtx_log_v2_0;
        _S928.value0_38 = _S925.value0_15;
        _S928.value1_9 = _S896.value0_33.value1_5;
        _S928.value2_9 = _S896.value0_33.value2_5.white_0;
        var _S929 : f32 = _S896.value0_33.value0_27.y;
        var _S930 : Tuple_13 = s_apply_max_1(_S929, 1.52587890625e-05f);
        var _S931 : s_bwdCallableCtx_max_1;
        _S931._S101 = _S929;
        _S931._S102 = 1.52587890625e-05f;
        var _S932 : Tuple_18 = s_apply_log_v2_0(_S930.value0_15, _S896.value0_33.value1_5, _S896.value0_33.value2_5.white_0);
        var _S933 : s_paramCtx_s_bwdCallableCtx_log_v2_0;
        _S933.value0_38 = _S930.value0_15;
        _S933.value1_9 = _S896.value0_33.value1_5;
        _S933.value2_9 = _S896.value0_33.value2_5.white_0;
        var _S934 : f32 = _S896.value0_33.value0_27.z;
        var _S935 : Tuple_13 = s_apply_max_1(_S934, 1.52587890625e-05f);
        var _S936 : s_bwdCallableCtx_max_1;
        _S936._S101 = _S934;
        _S936._S102 = 1.52587890625e-05f;
        var _S937 : Tuple_18 = s_apply_log_v2_0(_S935.value0_15, _S896.value0_33.value1_5, _S896.value0_33.value2_5.white_0);
        var _S938 : s_paramCtx_s_bwdCallableCtx_log_v2_0;
        _S938.value0_38 = _S935.value0_15;
        _S938.value1_9 = _S896.value0_33.value1_5;
        _S938.value2_9 = _S896.value0_33.value2_5.white_0;
        var t_2 : vec3<f32> = vec3<f32>(_S927.value0_48, _S932.value0_48, _S937.value0_48);
        var _S939 : Tuple_12 = s_apply_dot_0(_S896.value0_33.value2_5.luminance_1, t_2);
        var _S940 : s_bwdCallableCtx_dot_0;
        _S940._S96 = _S896.value0_33.value2_5.luminance_1;
        _S940._S97 = t_2;
        var _S941 : Tuple_20 = s_apply_desaturate_v2_0(_S939.value0_14, _S896.value0_33.value1_5);
        var _S942 : s_paramCtx_s_bwdCallableCtx_desaturate_v2_0;
        _S942.value0_39 = _S939.value0_14;
        _S942.value1_10 = _S896.value0_33.value1_5;
        var _S943 : vec3<f32> = vec3<f32>(_S939.value0_14);
        var _S944 : vec3<f32> = t_2 - _S943;
        var mapped_0 : vec3<f32> = _S943 + vec3<f32>(_S941.value0_50) * _S944;
        var _S945 : f32 = mapped_0.x;
        var _S946 : Tuple_21 = s_apply_spline_0(_S945, _S896.value0_33.value1_5, _S896.value0_33.value2_5.toe_1, _S896.value0_33.value2_5.shoulder_1);
        var _S947 : s_paramCtx_s_bwdCallableCtx_spline_0;
        _S947.value0_40 = _S945;
        _S947.value1_11 = _S896.value0_33.value1_5;
        _S947.value2_10 = _S896.value0_33.value2_5.toe_1;
        _S947.value3_1 = _S896.value0_33.value2_5.shoulder_1;
        var _S948 : f32 = mapped_0.y;
        var _S949 : Tuple_21 = s_apply_spline_0(_S948, _S896.value0_33.value1_5, _S896.value0_33.value2_5.toe_1, _S896.value0_33.value2_5.shoulder_1);
        var _S950 : s_paramCtx_s_bwdCallableCtx_spline_0;
        _S950.value0_40 = _S948;
        _S950.value1_11 = _S896.value0_33.value1_5;
        _S950.value2_10 = _S896.value0_33.value2_5.toe_1;
        _S950.value3_1 = _S896.value0_33.value2_5.shoulder_1;
        var _S951 : f32 = mapped_0.z;
        var _S952 : Tuple_21 = s_apply_spline_0(_S951, _S896.value0_33.value1_5, _S896.value0_33.value2_5.toe_1, _S896.value0_33.value2_5.shoulder_1);
        var _S953 : s_paramCtx_s_bwdCallableCtx_spline_0;
        _S953.value0_40 = _S951;
        _S953.value1_11 = _S896.value0_33.value1_5;
        _S953.value2_10 = _S896.value0_33.value2_5.toe_1;
        _S953.value3_1 = _S896.value0_33.value2_5.shoulder_1;
        var _S954 : vec3<f32> = vec3<f32>(_S946.value0_51, _S949.value0_51, _S952.value0_51);
        var _S955 : vec3<f32> = vec3<f32>(0.0f);
        var _S956 : vec3<f32> = vec3<f32>(1.0f);
        var _S957 : Tuple_11 = s_apply_clamp_1(_S954, _S955, _S956);
        var _S958 : s_bwdCallableCtx_clamp_1;
        _S958._S89 = _S954;
        _S958._S90 = _S955;
        _S958._S91 = _S956;
        var _S959 : f32 = _S896.value0_33.value1_5.tone_0.y;
        var _S960 : s_bwdCallableCtx_pow_dt_0;
        _S960._S79 = _S957.value0_13.x;
        _S960._S80 = _S959;
        var _S961 : s_bwdCallableCtx_pow_dt_0;
        _S961._S79 = _S957.value0_13.y;
        _S961._S80 = _S959;
        var _S962 : s_bwdCallableCtx_pow_dt_0;
        _S962._S79 = _S957.value0_13.z;
        _S962._S80 = _S959;
        _S907 = _S962;
        _S908 = _S961;
        _S909 = _S960;
        _S910 = _S958;
        _S911.value0_42 = _S953;
        _S912.value0_42 = _S950;
        _S913.value0_42 = _S947;
        _S914 = _S941.value0_50;
        _S915 = _S944;
        _S916.value0_43 = _S942;
        _S917 = _S940;
        _S918.value0_44 = _S938;
        _S921 = _S936;
        _S919.value0_44 = _S933;
        _S922 = _S931;
        _S920.value0_44 = _S928;
        _S923 = _S926;
    }
    else
    {
        _S907._S79 = 0.0f;
        _S907._S80 = 0.0f;
        _S908._S79 = 0.0f;
        _S908._S80 = 0.0f;
        _S909._S79 = 0.0f;
        _S909._S80 = 0.0f;
        _S910._S89 = _S903;
        _S910._S90 = _S903;
        _S910._S91 = _S903;
        _S911.value0_42 = _S905;
        _S912.value0_42 = _S905;
        _S913.value0_42 = _S905;
        _S914 = 0.0f;
        _S915 = _S903;
        _S916.value0_43 = _S904;
        _S917._S96 = _S903;
        _S917._S97 = _S903;
        _S918.value0_44 = _S902;
        _S921._S101 = 0.0f;
        _S921._S102 = 0.0f;
        _S919.value0_44 = _S902;
        _S922._S101 = 0.0f;
        _S922._S102 = 0.0f;
        _S920.value0_44 = _S902;
        _S923._S101 = 0.0f;
        _S923._S102 = 0.0f;
    }
    var _S963 : Curve_0 = Curve_x24_syn_dzero_0();
    var _S964 : vec4<f32>;
    var _S965 : Curve_0;
    if(_S906)
    {
        var _S966 : f32 = 0.0f;
        var _S967 : f32 = 0.0f;
        s_bwdProp_pow_dt_0(_S907, &(_S966), &(_S967), _S899[i32(2)]);
        var _S968 : f32 = 0.0f;
        var _S969 : f32 = 0.0f;
        s_bwdProp_pow_dt_0(_S908, &(_S968), &(_S969), _S899[i32(1)]);
        var _S970 : f32 = 0.0f;
        var _S971 : f32 = 0.0f;
        s_bwdProp_pow_dt_0(_S909, &(_S970), &(_S971), _S899[i32(0)]);
        var _S972 : f32 = _S967 + _S969 + _S971;
        var _S973 : vec3<f32> = vec3<f32>(_S970, _S968, _S966);
        var _S974 : vec3<f32> = _S903;
        var _S975 : vec3<f32> = _S903;
        var _S976 : vec3<f32> = _S903;
        s_bwdProp_clamp_1(_S910, &(_S974), &(_S975), &(_S976), _S973);
        var _S977 : f32 = 0.0f;
        var _S978 : Curve_0;
        _S978.toe_0 = _S900;
        _S978.shoulder_0 = _S900;
        _S978.line_0 = _S900;
        _S978.nodes_0 = _S900;
        _S978.tone_0 = _S900;
        _S978.display_0 = _S900;
        _S978.extra_0 = _S900;
        s_bwdProp_spline_0(_S911, &(_S977), &(_S978), _S974[i32(2)]);
        var _S979 : f32 = 0.0f;
        var _S980 : Curve_0;
        _S980.toe_0 = _S900;
        _S980.shoulder_0 = _S900;
        _S980.line_0 = _S900;
        _S980.nodes_0 = _S900;
        _S980.tone_0 = _S900;
        _S980.display_0 = _S900;
        _S980.extra_0 = _S900;
        s_bwdProp_spline_0(_S912, &(_S979), &(_S980), _S974[i32(1)]);
        var _S981 : f32 = 0.0f;
        var _S982 : Curve_0;
        _S982.toe_0 = _S900;
        _S982.shoulder_0 = _S900;
        _S982.line_0 = _S900;
        _S982.nodes_0 = _S900;
        _S982.tone_0 = _S900;
        _S982.display_0 = _S900;
        _S982.extra_0 = _S900;
        s_bwdProp_spline_0(_S913, &(_S981), &(_S982), _S974[i32(0)]);
        var s_diff_mapped_T_0 : vec3<f32> = vec3<f32>(_S981, _S979, _S977);
        var _S983 : f32 = _S981 + _S979 + _S977;
        var _S984 : vec3<f32> = vec3<f32>(_S914) * s_diff_mapped_T_0;
        var _S985 : vec3<f32> = _S915 * s_diff_mapped_T_0;
        var _S986 : f32 = _S985[i32(0)] + _S985[i32(1)] + _S985[i32(2)];
        var _S987 : vec3<f32> = (vec3<f32>(0) - _S984);
        var _S988 : f32 = _S987[i32(0)] + _S987[i32(1)] + _S987[i32(2)];
        var _S989 : f32 = 0.0f;
        var _S990 : Curve_0;
        _S990.toe_0 = _S900;
        _S990.shoulder_0 = _S900;
        _S990.line_0 = _S900;
        _S990.nodes_0 = _S900;
        _S990.tone_0 = _S900;
        _S990.display_0 = _S900;
        _S990.extra_0 = _S900;
        s_bwdProp_desaturate_v2_0(_S916, &(_S989), &(_S990), _S986);
        var _S991 : f32 = _S983 + _S988 + _S989;
        var _S992 : vec3<f32> = _S903;
        var _S993 : vec3<f32> = _S903;
        s_bwdProp_dot_0(_S917, &(_S992), &(_S993), _S991);
        var _S994 : vec3<f32> = _S984 + _S993;
        var _S995 : f32 = 0.0f;
        var _S996 : Curve_0;
        _S996.toe_0 = _S900;
        _S996.shoulder_0 = _S900;
        _S996.line_0 = _S900;
        _S996.nodes_0 = _S900;
        _S996.tone_0 = _S900;
        _S996.display_0 = _S900;
        _S996.extra_0 = _S900;
        s_bwdProp_log_v2_0(_S918, &(_S995), &(_S996), _S994[i32(2)]);
        var _S997 : f32 = 0.0f;
        var _S998 : f32 = 0.0f;
        s_bwdProp_max_1(_S921, &(_S997), &(_S998), _S995);
        var _S999 : f32 = 0.0f;
        var _S1000 : Curve_0;
        _S1000.toe_0 = _S900;
        _S1000.shoulder_0 = _S900;
        _S1000.line_0 = _S900;
        _S1000.nodes_0 = _S900;
        _S1000.tone_0 = _S900;
        _S1000.display_0 = _S900;
        _S1000.extra_0 = _S900;
        s_bwdProp_log_v2_0(_S919, &(_S999), &(_S1000), _S994[i32(1)]);
        var _S1001 : f32 = 0.0f;
        var _S1002 : f32 = 0.0f;
        s_bwdProp_max_1(_S922, &(_S1001), &(_S1002), _S999);
        var _S1003 : f32 = 0.0f;
        var _S1004 : Curve_0;
        _S1004.toe_0 = _S900;
        _S1004.shoulder_0 = _S900;
        _S1004.line_0 = _S900;
        _S1004.nodes_0 = _S900;
        _S1004.tone_0 = _S900;
        _S1004.display_0 = _S900;
        _S1004.extra_0 = _S900;
        s_bwdProp_log_v2_0(_S920, &(_S1003), &(_S1004), _S994[i32(0)]);
        var _S1005 : f32 = 0.0f;
        var _S1006 : f32 = 0.0f;
        s_bwdProp_max_1(_S923, &(_S1005), &(_S1006), _S1003);
        var _S1007 : Curve_0 = Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(_S978, _S980), _S982), _S990), _S996), _S1000), _S1004), _S963);
        var _S1008 : vec3<f32> = vec3<f32>(_S1005, _S1001, _S997);
        _S964 = vec4<f32>(0.0f, _S972, 0.0f, 0.0f);
        _S965 = _S1007;
        _S915 = _S1008;
    }
    else
    {
        _S964 = _S900;
        _S965 = _S963;
        _S915 = _S903;
    }
    var _S1009 : Curve_0 = _S963;
    _S1009.tone_0 = _S964;
    var _S1010 : Curve_0 = Curve_x24_syn_dadd_0(_S965, _S1009);
    (*_S897) = _S915;
    (*_S898) = _S1010;
    return;
}

fn s_bwdProp_split_v1_0( _S1011 : s_bwdCallableCtx_split_v1_0,  _S1012 : ptr<function, vec3<f32>>,  _S1013 : ptr<function, Curve_0>,  _S1014 : vec3<f32>)
{
    var _S1015 : f32 = _S1011.value0_32.value0_26.x;
    var _S1016 : Tuple_13 = s_apply_max_1(_S1015, 1.52587890625e-05f);
    var _S1017 : s_bwdCallableCtx_max_1;
    _S1017._S101 = _S1015;
    _S1017._S102 = 1.52587890625e-05f;
    var _S1018 : Tuple_22 = s_apply_log_v1_0(_S1016.value0_15, _S1011.value0_32.value1_4, _S1011.value0_32.value2_4.white_0);
    var _S1019 : s_paramCtx_s_bwdCallableCtx_log_v1_0;
    _S1019.value0_55 = _S1016.value0_15;
    _S1019.value1_13 = _S1011.value0_32.value1_4;
    _S1019.value2_11 = _S1011.value0_32.value2_4.white_0;
    var _S1020 : s_bwdCallableCtx_log_v1_0 = s_bwdCallableCtx_log_v1_0( _S1019 );
    var _S1021 : f32 = _S1011.value0_32.value0_26.y;
    var _S1022 : Tuple_13 = s_apply_max_1(_S1021, 1.52587890625e-05f);
    var _S1023 : s_bwdCallableCtx_max_1;
    _S1023._S101 = _S1021;
    _S1023._S102 = 1.52587890625e-05f;
    var _S1024 : Tuple_22 = s_apply_log_v1_0(_S1022.value0_15, _S1011.value0_32.value1_4, _S1011.value0_32.value2_4.white_0);
    var _S1025 : s_paramCtx_s_bwdCallableCtx_log_v1_0;
    _S1025.value0_55 = _S1022.value0_15;
    _S1025.value1_13 = _S1011.value0_32.value1_4;
    _S1025.value2_11 = _S1011.value0_32.value2_4.white_0;
    var _S1026 : s_bwdCallableCtx_log_v1_0 = s_bwdCallableCtx_log_v1_0( _S1025 );
    var _S1027 : f32 = _S1011.value0_32.value0_26.z;
    var _S1028 : Tuple_13 = s_apply_max_1(_S1027, 1.52587890625e-05f);
    var _S1029 : s_bwdCallableCtx_max_1;
    _S1029._S101 = _S1027;
    _S1029._S102 = 1.52587890625e-05f;
    var _S1030 : Tuple_22 = s_apply_log_v1_0(_S1028.value0_15, _S1011.value0_32.value1_4, _S1011.value0_32.value2_4.white_0);
    var _S1031 : s_paramCtx_s_bwdCallableCtx_log_v1_0;
    _S1031.value0_55 = _S1028.value0_15;
    _S1031.value1_13 = _S1011.value0_32.value1_4;
    _S1031.value2_11 = _S1011.value0_32.value2_4.white_0;
    var _S1032 : s_bwdCallableCtx_log_v1_0 = s_bwdCallableCtx_log_v1_0( _S1031 );
    var t_3 : vec3<f32> = vec3<f32>(_S1018.value0_54, _S1024.value0_54, _S1030.value0_54);
    var _S1033 : Tuple_12 = s_apply_dot_0(_S1011.value0_32.value2_4.luminance_1, t_3);
    var _S1034 : s_bwdCallableCtx_dot_0;
    _S1034._S96 = _S1011.value0_32.value2_4.luminance_1;
    _S1034._S97 = t_3;
    var _S1035 : Tuple_23 = s_apply_desaturate_v1_0(_S1033.value0_14, _S1011.value0_32.value1_4);
    var _S1036 : s_paramCtx_s_bwdCallableCtx_desaturate_v1_0;
    _S1036.value0_59 = _S1033.value0_14;
    _S1036.value1_14 = _S1011.value0_32.value1_4;
    var _S1037 : s_bwdCallableCtx_desaturate_v1_0 = s_bwdCallableCtx_desaturate_v1_0( _S1036 );
    var _S1038 : vec3<f32> = vec3<f32>(_S1033.value0_14);
    var _S1039 : vec3<f32> = t_3 - _S1038;
    var _S1040 : vec3<f32> = vec3<f32>(_S1035.value0_57);
    var mapped_1 : vec3<f32> = _S1038 + _S1040 * _S1039;
    var _S1041 : f32 = mapped_1.x;
    var _S1042 : Tuple_21 = s_apply_spline_0(_S1041, _S1011.value0_32.value1_4, _S1011.value0_32.value2_4.toe_1, _S1011.value0_32.value2_4.shoulder_1);
    var _S1043 : s_paramCtx_s_bwdCallableCtx_spline_0;
    _S1043.value0_40 = _S1041;
    _S1043.value1_11 = _S1011.value0_32.value1_4;
    _S1043.value2_10 = _S1011.value0_32.value2_4.toe_1;
    _S1043.value3_1 = _S1011.value0_32.value2_4.shoulder_1;
    var _S1044 : s_bwdCallableCtx_spline_0 = s_bwdCallableCtx_spline_0( _S1043 );
    var _S1045 : f32 = mapped_1.y;
    var _S1046 : Tuple_21 = s_apply_spline_0(_S1045, _S1011.value0_32.value1_4, _S1011.value0_32.value2_4.toe_1, _S1011.value0_32.value2_4.shoulder_1);
    var _S1047 : s_paramCtx_s_bwdCallableCtx_spline_0;
    _S1047.value0_40 = _S1045;
    _S1047.value1_11 = _S1011.value0_32.value1_4;
    _S1047.value2_10 = _S1011.value0_32.value2_4.toe_1;
    _S1047.value3_1 = _S1011.value0_32.value2_4.shoulder_1;
    var _S1048 : s_bwdCallableCtx_spline_0 = s_bwdCallableCtx_spline_0( _S1047 );
    var _S1049 : f32 = mapped_1.z;
    var _S1050 : Tuple_21 = s_apply_spline_0(_S1049, _S1011.value0_32.value1_4, _S1011.value0_32.value2_4.toe_1, _S1011.value0_32.value2_4.shoulder_1);
    var _S1051 : s_paramCtx_s_bwdCallableCtx_spline_0;
    _S1051.value0_40 = _S1049;
    _S1051.value1_11 = _S1011.value0_32.value1_4;
    _S1051.value2_10 = _S1011.value0_32.value2_4.toe_1;
    _S1051.value3_1 = _S1011.value0_32.value2_4.shoulder_1;
    var _S1052 : s_bwdCallableCtx_spline_0 = s_bwdCallableCtx_spline_0( _S1051 );
    var _S1053 : vec3<f32> = vec3<f32>(_S1042.value0_51, _S1046.value0_51, _S1050.value0_51);
    var _S1054 : vec3<f32> = vec3<f32>(0.0f);
    var _S1055 : vec3<f32> = vec3<f32>(1.0f);
    var _S1056 : Tuple_11 = s_apply_clamp_1(_S1053, _S1054, _S1055);
    var _S1057 : s_bwdCallableCtx_clamp_1;
    _S1057._S89 = _S1053;
    _S1057._S90 = _S1054;
    _S1057._S91 = _S1055;
    var _S1058 : f32 = _S1011.value0_32.value1_4.tone_0.y;
    var _S1059 : s_bwdCallableCtx_pow_dt_0;
    _S1059._S79 = _S1056.value0_13.x;
    _S1059._S80 = _S1058;
    var _S1060 : s_bwdCallableCtx_pow_dt_0;
    _S1060._S79 = _S1056.value0_13.y;
    _S1060._S80 = _S1058;
    var _S1061 : s_bwdCallableCtx_pow_dt_0;
    _S1061._S79 = _S1056.value0_13.z;
    _S1061._S80 = _S1058;
    var _S1062 : Curve_0 = Curve_x24_syn_dzero_0();
    var _S1063 : f32 = 0.0f;
    var _S1064 : f32 = 0.0f;
    s_bwdProp_pow_dt_0(_S1061, &(_S1063), &(_S1064), _S1014[i32(2)]);
    var _S1065 : f32 = 0.0f;
    var _S1066 : f32 = 0.0f;
    s_bwdProp_pow_dt_0(_S1060, &(_S1065), &(_S1066), _S1014[i32(1)]);
    var _S1067 : f32 = 0.0f;
    var _S1068 : f32 = 0.0f;
    s_bwdProp_pow_dt_0(_S1059, &(_S1067), &(_S1068), _S1014[i32(0)]);
    var _S1069 : vec4<f32> = vec4<f32>(0.0f, _S1064 + _S1066 + _S1068, 0.0f, 0.0f);
    var _S1070 : vec3<f32> = vec3<f32>(_S1067, _S1065, _S1063);
    var _S1071 : vec3<f32> = vec3<f32>(0.0f);
    var _S1072 : vec3<f32> = _S1071;
    var _S1073 : vec3<f32> = _S1071;
    var _S1074 : vec3<f32> = _S1071;
    s_bwdProp_clamp_1(_S1057, &(_S1072), &(_S1073), &(_S1074), _S1070);
    var _S1075 : f32 = 0.0f;
    var _S1076 : vec4<f32> = vec4<f32>(0.0f);
    var _S1077 : Curve_0;
    _S1077.toe_0 = _S1076;
    _S1077.shoulder_0 = _S1076;
    _S1077.line_0 = _S1076;
    _S1077.nodes_0 = _S1076;
    _S1077.tone_0 = _S1076;
    _S1077.display_0 = _S1076;
    _S1077.extra_0 = _S1076;
    s_bwdProp_spline_0(_S1052, &(_S1075), &(_S1077), _S1072[i32(2)]);
    var _S1078 : f32 = 0.0f;
    var _S1079 : Curve_0;
    _S1079.toe_0 = _S1076;
    _S1079.shoulder_0 = _S1076;
    _S1079.line_0 = _S1076;
    _S1079.nodes_0 = _S1076;
    _S1079.tone_0 = _S1076;
    _S1079.display_0 = _S1076;
    _S1079.extra_0 = _S1076;
    s_bwdProp_spline_0(_S1048, &(_S1078), &(_S1079), _S1072[i32(1)]);
    var _S1080 : f32 = 0.0f;
    var _S1081 : Curve_0;
    _S1081.toe_0 = _S1076;
    _S1081.shoulder_0 = _S1076;
    _S1081.line_0 = _S1076;
    _S1081.nodes_0 = _S1076;
    _S1081.tone_0 = _S1076;
    _S1081.display_0 = _S1076;
    _S1081.extra_0 = _S1076;
    s_bwdProp_spline_0(_S1044, &(_S1080), &(_S1081), _S1072[i32(0)]);
    var s_diff_mapped_T_1 : vec3<f32> = vec3<f32>(_S1080, _S1078, _S1075);
    var _S1082 : f32 = _S1080 + _S1078 + _S1075;
    var _S1083 : vec3<f32> = _S1040 * s_diff_mapped_T_1;
    var _S1084 : vec3<f32> = _S1039 * s_diff_mapped_T_1;
    var _S1085 : f32 = _S1084[i32(0)] + _S1084[i32(1)] + _S1084[i32(2)];
    var _S1086 : vec3<f32> = (vec3<f32>(0) - _S1083);
    var _S1087 : f32 = _S1086[i32(0)] + _S1086[i32(1)] + _S1086[i32(2)];
    var _S1088 : f32 = 0.0f;
    var _S1089 : Curve_0;
    _S1089.toe_0 = _S1076;
    _S1089.shoulder_0 = _S1076;
    _S1089.line_0 = _S1076;
    _S1089.nodes_0 = _S1076;
    _S1089.tone_0 = _S1076;
    _S1089.display_0 = _S1076;
    _S1089.extra_0 = _S1076;
    s_bwdProp_desaturate_v1_0(_S1037, &(_S1088), &(_S1089), _S1085);
    var _S1090 : f32 = _S1082 + _S1087 + _S1088;
    var _S1091 : vec3<f32> = _S1071;
    var _S1092 : vec3<f32> = _S1071;
    s_bwdProp_dot_0(_S1034, &(_S1091), &(_S1092), _S1090);
    var _S1093 : vec3<f32> = _S1083 + _S1092;
    var _S1094 : f32 = 0.0f;
    var _S1095 : Curve_0;
    _S1095.toe_0 = _S1076;
    _S1095.shoulder_0 = _S1076;
    _S1095.line_0 = _S1076;
    _S1095.nodes_0 = _S1076;
    _S1095.tone_0 = _S1076;
    _S1095.display_0 = _S1076;
    _S1095.extra_0 = _S1076;
    s_bwdProp_log_v1_0(_S1032, &(_S1094), &(_S1095), _S1093[i32(2)]);
    var _S1096 : f32 = 0.0f;
    var _S1097 : f32 = 0.0f;
    s_bwdProp_max_1(_S1029, &(_S1096), &(_S1097), _S1094);
    var _S1098 : f32 = 0.0f;
    var _S1099 : Curve_0;
    _S1099.toe_0 = _S1076;
    _S1099.shoulder_0 = _S1076;
    _S1099.line_0 = _S1076;
    _S1099.nodes_0 = _S1076;
    _S1099.tone_0 = _S1076;
    _S1099.display_0 = _S1076;
    _S1099.extra_0 = _S1076;
    s_bwdProp_log_v1_0(_S1026, &(_S1098), &(_S1099), _S1093[i32(1)]);
    var _S1100 : f32 = 0.0f;
    var _S1101 : f32 = 0.0f;
    s_bwdProp_max_1(_S1023, &(_S1100), &(_S1101), _S1098);
    var _S1102 : f32 = 0.0f;
    var _S1103 : Curve_0;
    _S1103.toe_0 = _S1076;
    _S1103.shoulder_0 = _S1076;
    _S1103.line_0 = _S1076;
    _S1103.nodes_0 = _S1076;
    _S1103.tone_0 = _S1076;
    _S1103.display_0 = _S1076;
    _S1103.extra_0 = _S1076;
    s_bwdProp_log_v1_0(_S1020, &(_S1102), &(_S1103), _S1093[i32(0)]);
    var _S1104 : f32 = 0.0f;
    var _S1105 : f32 = 0.0f;
    s_bwdProp_max_1(_S1017, &(_S1104), &(_S1105), _S1102);
    var _S1106 : Curve_0 = Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(_S1077, _S1079), _S1081), _S1089), _S1095), _S1099), _S1103), _S1062);
    var _S1107 : Curve_0 = _S1062;
    _S1107.tone_0 = _S1069;
    var _S1108 : Curve_0 = Curve_x24_syn_dadd_0(_S1106, _S1107);
    (*_S1012) = vec3<f32>(_S1104, _S1100, _S1096);
    (*_S1013) = _S1108;
    return;
}

struct s_paramCtx_s_bwdCallableCtx_rgb_tone_mapping_0
{
     value0_63 : vec3<f32>,
     value1_16 : Curve_0,
     value2_12 : Modes_0,
};

struct s_paramCtx_s_bwdCallableCtx_rgb_to_ych_0
{
     value0_64 : vec3<f32>,
     value1_17 : mat3x3<f32>,
};

struct s_paramCtx_s_bwdCallableCtx_gamut_mapping_0
{
     value0_65 : vec4<f32>,
     value1_18 : vec4<f32>,
     value2_13 : Curve_0,
     value3_2 : f32,
     value4_0 : Modes_0,
};

struct s_paramCtx_s_bwdCallableCtx_norm_tone_mapping_0
{
     value0_66 : vec3<f32>,
     value1_19 : Curve_0,
     value2_14 : u32,
     value3_3 : Modes_0,
};

struct Tuple_25
{
     value0_67 : vec4<f32>,
};

struct Tuple_26
{
     value0_68 : vec3<f32>,
};

fn s_apply_lms_to_yrg_0( _S1109 : vec3<f32>) -> Tuple_26
{
    var _S1110 : f32 = _S1109.x;
    var _S1111 : f32 = _S1109.y;
    var y_9 : f32 = 0.68990272283554077f * _S1110 + 0.34832188487052917f * _S1111;
    var a_0 : f32 = _S1110 + _S1111 + _S1109.z;
    var _S1112 : vec3<f32> = vec3<f32>(0.0f);
    var normalized_0 : vec3<f32>;
    if(a_0 != 0.0f)
    {
        normalized_0 = _S1109 / vec3<f32>(a_0);
    }
    else
    {
        normalized_0 = _S1112;
    }
    var _S1113 : Tuple_5 = s_apply_mul_0(mat3x3<f32>(1.0877193212509155f, -0.66666668653488159f, 0.020618559792637825f, -0.087719298899173737f, 1.6666666269302368f, -0.051546391099691391f, 0.0f, 0.0f, 1.0309278964996338f), normalized_0);
    var _S1114 : Tuple_26;
    _S1114.value0_68 = vec3<f32>(y_9, _S1113.value0_7.x, _S1113.value0_7.y);
    return _S1114;
}

struct Tuple_27
{
     value0_69 : vec4<f32>,
};

fn s_apply_yrg_to_ych_0( _S1115 : vec3<f32>) -> Tuple_27
{
    var rg_0 : vec2<f32> = _S1115.yz - vec2<f32>(0.21902142465114594f, 0.54371398687362671f);
    var _S1116 : Tuple_1 = s_apply_length_0(rg_0);
    const _S1117 : vec2<f32> = vec2<f32>(1.0f, 0.0f);
    var cos_sin_0 : vec2<f32>;
    if((_S1116.value0_1) != 0.0f)
    {
        cos_sin_0 = rg_0 / vec2<f32>(_S1116.value0_1);
    }
    else
    {
        cos_sin_0 = _S1117;
    }
    var _S1118 : Tuple_27;
    _S1118.value0_69 = vec4<f32>(_S1115.x, _S1116.value0_1, cos_sin_0.x, cos_sin_0.y);
    return _S1118;
}

fn s_apply_rgb_to_ych_0( _S1119 : vec3<f32>,  to_lms_2 : mat3x3<f32>) -> Tuple_25
{
    var _S1120 : Tuple_13 = s_apply_max_1(s_apply_max_1(s_apply_abs_1(_S1119.x).value0_8, s_apply_abs_1(_S1119.y).value0_8).value0_15, s_apply_abs_1(_S1119.z).value0_8);
    var _S1121 : bool = ((_S1120.value0_15) <= 0.0f);
    var _S1122 : vec4<f32>;
    if(_S1121)
    {
        _S1122 = s_apply_yrg_to_ych_0(s_apply_lms_to_yrg_0(s_apply_mul_0(to_lms_2, _S1119).value0_7).value0_68).value0_69;
    }
    var _S1123 : bool = !_S1121;
    if(_S1123)
    {
        var _S1124 : Tuple_27 = s_apply_yrg_to_ych_0(s_apply_lms_to_yrg_0(s_apply_mul_0(to_lms_2, _S1119 / vec3<f32>(_S1120.value0_15)).value0_7).value0_68);
        _S1122 = vec4<f32>(_S1124.value0_69.x * _S1120.value0_15, _S1124.value0_69.y, _S1124.value0_69.z, _S1124.value0_69.w);
    }
    var _S1125 : Tuple_25;
    _S1125.value0_67 = _S1122;
    return _S1125;
}

struct s_bwdCallableCtx_rgb_to_ych_0
{
     value0_70 : s_paramCtx_s_bwdCallableCtx_rgb_to_ych_0,
};

struct s_bwdCallableCtx_gamut_mapping_0
{
     value0_71 : s_paramCtx_s_bwdCallableCtx_gamut_mapping_0,
};

struct s_bwdCallableCtx_rgb_tone_mapping_0
{
     value0_72 : s_paramCtx_s_bwdCallableCtx_rgb_tone_mapping_0,
};

struct Tuple_28
{
     value0_73 : vec3<f32>,
};

struct Tuple_29
{
     value0_74 : f32,
};

fn s_apply_channel_tone_mapping_0( _S1126 : f32,  _S1127 : Curve_0,  m_0 : Modes_0) -> Tuple_29
{
    var _S1128 : Tuple_29;
    _S1128.value0_74 = s_apply_pow_dt_0(s_apply_clamp_0(s_apply_spline_0(s_apply_log_v2_0(s_apply_max_1(_S1126, 1.00000000317107685e-30f).value0_15, _S1127, false).value0_48, _S1127, m_0.toe_1, m_0.shoulder_1).value0_51, 0.0f, _S1127.display_0.z).value0_11, _S1127.tone_0.y).value0_12;
    return _S1128;
}

fn s_apply_rgb_tone_mapping_0( _S1129 : vec3<f32>,  _S1130 : Curve_0,  m_1 : Modes_0) -> Tuple_28
{
    var _S1131 : Tuple_28;
    _S1131.value0_73 = vec3<f32>(s_apply_channel_tone_mapping_0(_S1129.x, _S1130, m_1).value0_74, s_apply_channel_tone_mapping_0(_S1129.y, _S1130, m_1).value0_74, s_apply_channel_tone_mapping_0(_S1129.z, _S1130, m_1).value0_74);
    return _S1131;
}

struct s_bwdCallableCtx_norm_tone_mapping_0
{
     value0_75 : s_paramCtx_s_bwdCallableCtx_norm_tone_mapping_0,
};

struct Tuple_30
{
     value0_76 : vec3<f32>,
};

fn s_apply_norm_tone_mapping_0( _S1132 : vec3<f32>,  _S1133 : Curve_0,  norm_type_0 : u32,  m_2 : Modes_0) -> Tuple_30
{
    var _S1134 : Tuple_9 = s_apply_clamp_0(s_apply_pixel_norm_0(_S1132, norm_type_0, m_2.luminance_1).value0_46, _S1133.display_0.w, _S1133.extra_0.x);
    var _S1135 : Tuple_30;
    _S1135.value0_76 = _S1132 / vec3<f32>(_S1134.value0_11) * vec3<f32>(s_apply_pow0_0(s_apply_clamp_0(s_apply_spline_0(s_apply_log_v2_0(_S1134.value0_11, _S1133, false).value0_48, _S1133, m_2.toe_1, m_2.shoulder_1).value0_51, _S1133.display_0.y, _S1133.display_0.z).value0_11, _S1133.tone_0.y).value0_10);
    return _S1135;
}

struct Tuple_31
{
     value0_77 : vec4<f32>,
};

fn s_apply_desaturate_v4_0( _S1136 : vec4<f32>,  _S1137 : vec4<f32>,  _S1138 : f32) -> Tuple_31
{
    var _S1139 : f32 = _S1136.x;
    var chroma_original_0 : f32 = _S1136.y * _S1139;
    var _S1140 : f32 = _S1137.x;
    var chroma_final_0 : f32 = _S1137.y * _S1140;
    var delta_0 : f32 = _S1138 * (chroma_original_0 - chroma_final_0);
    var resaturates_0 : bool = (chroma_original_0 < chroma_final_0);
    var desaturates_0 : bool = (chroma_original_0 > chroma_final_0);
    var _S1141 : bool;
    if((_S1140 > _S1139))
    {
        _S1141 = resaturates_0;
    }
    else
    {
        _S1141 = false;
    }
    var chroma_0 : f32;
    if(_S1141)
    {
        chroma_0 = (chroma_original_0 + chroma_final_0) / 2.0f;
    }
    else
    {
        if((_S1138 > 0.0f))
        {
            _S1141 = desaturates_0;
        }
        else
        {
            _S1141 = false;
        }
        if(_S1141)
        {
            _S1141 = true;
        }
        else
        {
            _S1141 = (_S1138 < 0.0f);
        }
        if(_S1141)
        {
            chroma_0 = chroma_final_0 + delta_0;
        }
        else
        {
            chroma_0 = chroma_final_0;
        }
    }
    var c_0 : f32;
    if((_S1140 > 0.0f))
    {
        c_0 = s_apply_max_1(chroma_0 / _S1140, 0.0f).value0_15;
    }
    else
    {
        c_0 = 0.0f;
    }
    var _S1142 : Tuple_31;
    _S1142.value0_77 = vec4<f32>(_S1140, c_0, _S1137.z, _S1137.w);
    return _S1142;
}

struct s_paramCtx_s_bwdCallableCtx_desaturate_v4_0
{
     value0_78 : vec4<f32>,
     value1_20 : vec4<f32>,
     value2_15 : f32,
};

struct s_bwdCallableCtx_desaturate_v4_0
{
     value0_79 : s_paramCtx_s_bwdCallableCtx_desaturate_v4_0,
};

struct Tuple_32
{
     value0_80 : vec4<f32>,
};

struct Tuple_33
{
     value0_81 : vec3<f32>,
};

fn s_apply_ych_to_yrg_0( _S1143 : vec4<f32>) -> Tuple_33
{
    var rg_1 : vec2<f32> = vec2<f32>(_S1143.y) * _S1143.zw + vec2<f32>(0.21902142465114594f, 0.54371398687362671f);
    var _S1144 : Tuple_33;
    _S1144.value0_81 = vec3<f32>(_S1143.x, rg_1.x, rg_1.y);
    return _S1144;
}

fn s_apply_gamut_check_yrg_0( _S1145 : vec4<f32>) -> Tuple_32
{
    var _S1146 : Tuple_33 = s_apply_ych_to_yrg_0(_S1145);
    var max_c_0 : f32 = _S1145.y;
    var _S1147 : f32 = _S1146.value0_81.y;
    var max_c_1 : f32;
    if((_S1147 < 0.0f))
    {
        max_c_1 = s_apply_min_0(-0.21902142465114594f / _S1145.z, max_c_0).value0_16;
    }
    else
    {
        max_c_1 = max_c_0;
    }
    var _S1148 : f32 = _S1146.value0_81.z;
    if((_S1148 < 0.0f))
    {
        max_c_1 = s_apply_min_0(-0.54371398687362671f / _S1145.w, max_c_1).value0_16;
    }
    if(((_S1147 + _S1148) > 1.0f))
    {
        max_c_1 = s_apply_min_0(0.23726457357406616f / (_S1145.z + _S1145.w), max_c_1).value0_16;
    }
    var _S1149 : Tuple_32;
    _S1149.value0_80 = vec4<f32>(_S1145.x, max_c_1, _S1145.z, _S1145.w);
    return _S1149;
}

struct s_paramCtx_s_bwdCallableCtx_gamut_check_yrg_0
{
     value0_82 : vec4<f32>,
};

struct s_bwdCallableCtx_gamut_check_yrg_0
{
     value0_83 : s_paramCtx_s_bwdCallableCtx_gamut_check_yrg_0,
};

struct Tuple_34
{
     value0_84 : vec3<f32>,
};

struct Tuple_35
{
     value0_85 : vec3<f32>,
};

struct Tuple_36
{
     value0_86 : vec3<f32>,
};

fn s_apply_yrg_to_lms_0( _S1150 : vec3<f32>) -> Tuple_36
{
    var _S1151 : f32 = _S1150.y;
    var _S1152 : f32 = _S1150.z;
    var _S1153 : Tuple_5 = s_apply_mul_0(mat3x3<f32>(0.94999998807907104f, 0.37999999523162842f, 0.0f, 0.05000000074505806f, 0.62000000476837158f, 0.029999999329447746f, 0.0f, 0.0f, 0.97000002861022949f), vec3<f32>(_S1151, _S1152, 1.0f - _S1151 - _S1152));
    var denominator_1 : f32 = 0.68990272283554077f * _S1153.value0_7.x + 0.34832188487052917f * _S1153.value0_7.y;
    var a_1 : f32;
    if(denominator_1 != 0.0f)
    {
        a_1 = _S1150.x / denominator_1;
    }
    else
    {
        a_1 = 0.0f;
    }
    var _S1154 : Tuple_36;
    _S1154.value0_86 = _S1153.value0_7 * vec3<f32>(a_1);
    return _S1154;
}

fn s_apply_ych_to_rgb_0( _S1155 : vec4<f32>,  from_lms_2 : mat3x3<f32>) -> Tuple_35
{
    var _S1156 : Tuple_35;
    _S1156.value0_85 = s_apply_mul_0(from_lms_2, s_apply_yrg_to_lms_0(s_apply_ych_to_yrg_0(_S1155).value0_81).value0_86).value0_7;
    return _S1156;
}

struct Tuple_37
{
     value0_87 : f32,
};

struct Tuple_38
{
     value0_88 : f32,
};

struct Tuple_39
{
     value0_89 : f32,
};

struct Tuple_40
{
     value0_90 : f32,
};

fn s_apply_chroma_denominator_0( coeffs_0 : vec3<f32>,  _S1157 : f32,  _S1158 : f32) -> Tuple_40
{
    var _S1159 : Tuple_40;
    _S1159.value0_90 = coeffs_0.x * (0.97938144207000732f * _S1157 + 0.39175257086753845f * _S1158) + coeffs_0.y * (0.020618556067347527f * _S1157 + 0.60824739933013916f * _S1158) - coeffs_0.z * (_S1157 + _S1158);
    return _S1159;
}

fn s_apply_clip_chroma_white_raw_0( coeffs_1 : vec3<f32>,  _S1160 : f32,  _S1161 : f32,  _S1162 : f32,  _S1163 : f32) -> Tuple_39
{
    var _S1164 : Tuple_40 = s_apply_chroma_denominator_0(coeffs_1, _S1162, _S1163);
    var denominator_target_0 : f32 = _S1160 * (0.68285983800888062f * _S1162 + 0.4821370542049408f * _S1163);
    var _S1165 : bool = !((_S1164.value0_90) == 0.0f);
    var _S1166 : f32;
    if(_S1165)
    {
        var _runFlag_8 : bool;
        if((_S1161 <= (denominator_target_0 / _S1164.value0_90)))
        {
            _runFlag_8 = false;
        }
        else
        {
            _runFlag_8 = _S1165;
        }
        if(_runFlag_8)
        {
            _S1166 = -0.42750686407089233f * (_S1161 * (coeffs_1.x + 0.85649234056472778f * coeffs_1.y + 0.55499595403671265f * coeffs_1.z) - 0.98823773860931396f * _S1160) / (_S1161 * _S1164.value0_90 - denominator_target_0);
        }
        else
        {
            _S1166 = 3.4028234663852886e+38f;
        }
    }
    else
    {
        _S1166 = 3.4028234663852886e+38f;
    }
    var _S1167 : Tuple_39;
    _S1167.value0_89 = _S1166;
    return _S1167;
}

fn s_apply_clip_chroma_white_0( coeffs_2 : vec3<f32>,  _S1168 : f32,  _S1169 : f32,  _S1170 : f32,  _S1171 : f32) -> Tuple_38
{
    var max_y_0 : f32 = 1.0578552484512329f * _S1168;
    var _S1172 : Tuple_13 = s_apply_max_1(max_y_0 - _S1169, 0.0f);
    var max_chroma_0 : f32;
    var _runFlag_9 : bool;
    if(((_S1172.value0_15) < 0.0010000000474974513f))
    {
        var _S1173 : Tuple_39 = s_apply_clip_chroma_white_raw_0(coeffs_2, _S1168, 0.99900001287460327f * max_y_0, _S1170, _S1171);
        var _S1174 : bool = !((_S1173.value0_89) >= 3.4028234663852886e+38f);
        if(_S1174)
        {
            max_chroma_0 = _S1172.value0_15 / (0.0010000000474974513f * max_y_0) * _S1173.value0_89;
        }
        _runFlag_9 = _S1174;
    }
    else
    {
        var _S1175 : Tuple_39 = s_apply_clip_chroma_white_raw_0(coeffs_2, _S1168, _S1169, _S1170, _S1171);
        _runFlag_9 = true;
        max_chroma_0 = _S1175.value0_89;
    }
    if(_runFlag_9)
    {
        if((max_chroma_0 >= 0.0f))
        {
        }
        else
        {
            max_chroma_0 = 3.4028234663852886e+38f;
        }
    }
    else
    {
        max_chroma_0 = 3.4028234663852886e+38f;
    }
    var _S1176 : Tuple_38;
    _S1176.value0_88 = max_chroma_0;
    return _S1176;
}

struct Tuple_41
{
     value0_91 : f32,
};

fn s_apply_clip_chroma_black_0( coeffs_3 : vec3<f32>,  _S1177 : f32,  _S1178 : f32) -> Tuple_41
{
    var _S1179 : Tuple_40 = s_apply_chroma_denominator_0(coeffs_3, _S1177, _S1178);
    var _S1180 : f32;
    if(!((_S1179.value0_90) == 0.0f))
    {
        var max_chroma_1 : f32 = -0.42750686407089233f * (coeffs_3.x + 0.85649234056472778f * coeffs_3.y + 0.55499595403671265f * coeffs_3.z) / _S1179.value0_90;
        if((max_chroma_1 >= 0.0f))
        {
            _S1180 = max_chroma_1;
        }
        else
        {
            _S1180 = 3.4028234663852886e+38f;
        }
    }
    else
    {
        _S1180 = 3.4028234663852886e+38f;
    }
    var _S1181 : Tuple_41;
    _S1181.value0_91 = _S1180;
    return _S1181;
}

fn s_apply_ych_max_chroma_0( from_lms_3 : mat3x3<f32>,  _S1182 : f32,  _S1183 : f32,  _S1184 : f32,  _S1185 : f32) -> Tuple_37
{
    var _S1186 : Tuple_37;
    _S1186.value0_87 = s_apply_min_0(s_apply_min_0(s_apply_min_0(s_apply_clip_chroma_black_0(from_lms_3[i32(0)], _S1184, _S1185).value0_91, s_apply_clip_chroma_black_0(from_lms_3[i32(1)], _S1184, _S1185).value0_91).value0_16, s_apply_clip_chroma_black_0(from_lms_3[i32(2)], _S1184, _S1185).value0_91).value0_16, s_apply_min_0(s_apply_min_0(s_apply_clip_chroma_white_0(from_lms_3[i32(0)], _S1182, _S1183, _S1184, _S1185).value0_88, s_apply_clip_chroma_white_0(from_lms_3[i32(1)], _S1182, _S1183, _S1184, _S1185).value0_88).value0_16, s_apply_clip_chroma_white_0(from_lms_3[i32(2)], _S1182, _S1183, _S1184, _S1185).value0_88).value0_16).value0_16;
    return _S1186;
}

fn s_apply_gamut_check_rgb_0( _S1187 : vec4<f32>,  to_lms_3 : mat3x3<f32>,  from_lms_4 : mat3x3<f32>,  _S1188 : f32,  _S1189 : f32) -> Tuple_34
{
    var _S1190 : Tuple_35 = s_apply_ych_to_rgb_0(_S1187, from_lms_4);
    var _S1191 : Tuple_9 = s_apply_clamp_0((_S1187.x + s_apply_rgb_to_ych_0(_S1190.value0_85 + vec3<f32>(s_apply_max_1(- s_apply_min_0(s_apply_min_0(_S1190.value0_85.x, _S1190.value0_85.y).value0_16, _S1190.value0_85.z).value0_16, 0.0f).value0_15), to_lms_3).value0_67.x) / 2.0f, 1.0578552484512329f * _S1188, 1.0578552484512329f * _S1189);
    var cos_h_0 : f32 = _S1187.z;
    var sin_h_0 : f32 = _S1187.w;
    var _S1192 : Tuple_34;
    _S1192.value0_84 = s_apply_clamp_1(s_apply_ych_to_rgb_0(vec4<f32>(_S1191.value0_11, s_apply_min_0(_S1187.y, s_apply_ych_max_chroma_0(from_lms_4, _S1189, _S1191.value0_11, cos_h_0, sin_h_0).value0_87).value0_16, cos_h_0, sin_h_0), from_lms_4).value0_85, vec3<f32>(0.0f), vec3<f32>(_S1189)).value0_13;
    return _S1192;
}

struct s_paramCtx_s_bwdCallableCtx_gamut_check_rgb_0
{
     value0_92 : vec4<f32>,
     value1_21 : mat3x3<f32>,
     value2_16 : mat3x3<f32>,
     value3_4 : f32,
     value4_1 : f32,
};

struct s_bwdCallableCtx_gamut_check_rgb_0
{
     value0_93 : s_paramCtx_s_bwdCallableCtx_gamut_check_rgb_0,
};

struct s_paramCtx_s_bwdCallableCtx_ych_to_rgb_0
{
     value0_94 : vec4<f32>,
     value1_22 : mat3x3<f32>,
};

struct s_bwdCallableCtx_ych_to_rgb_0
{
     value0_95 : s_paramCtx_s_bwdCallableCtx_ych_to_rgb_0,
};

struct s_paramCtx_s_bwdCallableCtx_ych_max_chroma_0
{
     value0_96 : mat3x3<f32>,
     value1_23 : f32,
     value2_17 : f32,
     value3_5 : f32,
     value4_2 : f32,
};

struct s_bwdCallableCtx_ych_max_chroma_0
{
     value0_97 : s_paramCtx_s_bwdCallableCtx_ych_max_chroma_0,
};

struct s_paramCtx_s_bwdCallableCtx_ych_to_yrg_0
{
     value0_98 : vec4<f32>,
};

struct s_bwdCallableCtx_ych_to_yrg_0
{
     value0_99 : s_paramCtx_s_bwdCallableCtx_ych_to_yrg_0,
};

struct s_paramCtx_s_bwdCallableCtx_yrg_to_lms_0
{
     value0_100 : vec3<f32>,
};

struct s_bwdCallableCtx_yrg_to_lms_0
{
     value0_101 : s_paramCtx_s_bwdCallableCtx_yrg_to_lms_0,
};

fn s_bwdProp_yrg_to_lms_0( _S1193 : s_bwdCallableCtx_yrg_to_lms_0,  _S1194 : ptr<function, vec3<f32>>,  _S1195 : vec3<f32>)
{
    var _S1196 : f32 = _S1193.value0_101.value0_100.y;
    var _S1197 : f32 = _S1193.value0_101.value0_100.z;
    var _S1198 : vec3<f32> = vec3<f32>(_S1196, _S1197, 1.0f - _S1196 - _S1197);
    var _S1199 : Tuple_5 = s_apply_mul_0(mat3x3<f32>(0.94999998807907104f, 0.37999999523162842f, 0.0f, 0.05000000074505806f, 0.62000000476837158f, 0.029999999329447746f, 0.0f, 0.0f, 0.97000002861022949f), _S1198);
    var _S1200 : s_bwdCallableCtx_mul_0;
    _S1200._S40 = mat3x3<f32>(0.94999998807907104f, 0.37999999523162842f, 0.0f, 0.05000000074505806f, 0.62000000476837158f, 0.029999999329447746f, 0.0f, 0.0f, 0.97000002861022949f);
    _S1200._S41 = _S1198;
    var _S1201 : s_bwdCallableCtx_mul_0 = _S1200;
    var denominator_2 : f32 = 0.68990272283554077f * _S1199.value0_7.x + 0.34832188487052917f * _S1199.value0_7.y;
    var _S1202 : bool = denominator_2 != 0.0f;
    var a_2 : f32;
    var _S1203 : f32;
    var _S1204 : f32;
    if(_S1202)
    {
        var _S1205 : f32 = _S1193.value0_101.value0_100.x;
        var _S1206 : f32 = denominator_2 * denominator_2;
        a_2 = _S1205 / denominator_2;
        _S1203 = _S1206;
        _S1204 = _S1205;
    }
    else
    {
        a_2 = 0.0f;
        _S1203 = 0.0f;
        _S1204 = 0.0f;
    }
    var _S1207 : vec3<f32> = vec3<f32>(0.0f);
    var _S1208 : vec3<f32> = _S1199.value0_7 * _S1195;
    var _S1209 : f32 = _S1208[i32(0)] + _S1208[i32(1)] + _S1208[i32(2)];
    var _S1210 : vec3<f32> = vec3<f32>(a_2) * _S1195;
    var _S1211 : vec3<f32>;
    if(_S1202)
    {
        var _S1212 : f32 = _S1209 / _S1203;
        var _S1213 : vec3<f32> = vec3<f32>(denominator_2 * _S1212, 0.0f, 0.0f);
        a_2 = _S1204 * - _S1212;
        _S1211 = _S1213;
    }
    else
    {
        a_2 = 0.0f;
        _S1211 = _S1207;
    }
    var _S1214 : vec3<f32> = _S1210 + vec3<f32>(0.68990272283554077f * a_2, 0.34832188487052917f * a_2, 0.0f);
    var _S1215 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S1216 : vec3<f32> = _S1207;
    s_bwdProp_mul_0(_S1201, &(_S1215), &(_S1216), _S1214);
    var _S1217 : f32 = - _S1216[i32(2)];
    (*_S1194) = _S1211 + vec3<f32>(0.0f, _S1216[i32(0)] + _S1217, _S1216[i32(1)] + _S1217);
    return;
}

fn s_bwdProp_ych_to_yrg_0( _S1218 : s_bwdCallableCtx_ych_to_yrg_0,  _S1219 : ptr<function, vec4<f32>>,  _S1220 : vec3<f32>)
{
    var s_diff_rg_T_0 : vec2<f32> = vec2<f32>(_S1220[i32(1)], _S1220[i32(2)]);
    var _S1221 : vec2<f32> = vec2<f32>(_S1218.value0_99.value0_98.y) * s_diff_rg_T_0;
    var _S1222 : vec2<f32> = _S1218.value0_99.value0_98.zw * s_diff_rg_T_0;
    (*_S1219) = vec4<f32>(_S1220[i32(0)], _S1222[i32(0)] + _S1222[i32(1)], _S1221[i32(0)], _S1221[i32(1)]);
    return;
}

fn s_bwdProp_ych_to_rgb_0( _S1223 : s_bwdCallableCtx_ych_to_rgb_0,  _S1224 : ptr<function, vec4<f32>>,  _S1225 : vec3<f32>)
{
    var _S1226 : Tuple_33 = s_apply_ych_to_yrg_0(_S1223.value0_95.value0_94);
    var _S1227 : s_paramCtx_s_bwdCallableCtx_ych_to_yrg_0;
    _S1227.value0_98 = _S1223.value0_95.value0_94;
    var _S1228 : s_bwdCallableCtx_ych_to_yrg_0 = s_bwdCallableCtx_ych_to_yrg_0( _S1227 );
    var _S1229 : Tuple_36 = s_apply_yrg_to_lms_0(_S1226.value0_81);
    var _S1230 : s_paramCtx_s_bwdCallableCtx_yrg_to_lms_0;
    _S1230.value0_100 = _S1226.value0_81;
    var _S1231 : s_bwdCallableCtx_yrg_to_lms_0 = s_bwdCallableCtx_yrg_to_lms_0( _S1230 );
    var _S1232 : s_bwdCallableCtx_mul_0;
    _S1232._S40 = _S1223.value0_95.value1_22;
    _S1232._S41 = _S1229.value0_86;
    var _S1233 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S1234 : vec3<f32> = vec3<f32>(0.0f);
    var _S1235 : vec3<f32> = _S1234;
    s_bwdProp_mul_0(_S1232, &(_S1233), &(_S1235), _S1225);
    var _S1236 : vec3<f32> = _S1234;
    s_bwdProp_yrg_to_lms_0(_S1231, &(_S1236), _S1235);
    var _S1237 : vec4<f32> = vec4<f32>(0.0f);
    s_bwdProp_ych_to_yrg_0(_S1228, &(_S1237), _S1236);
    (*_S1224) = _S1237;
    return;
}

struct s_paramCtx_s_bwdCallableCtx_clip_chroma_white_0
{
     value0_102 : vec3<f32>,
     value1_24 : f32,
     value2_18 : f32,
     value3_6 : f32,
     value4_3 : f32,
};

struct s_bwdCallableCtx_clip_chroma_white_0
{
     value0_103 : s_paramCtx_s_bwdCallableCtx_clip_chroma_white_0,
};

struct s_paramCtx_s_bwdCallableCtx_clip_chroma_black_0
{
     value0_104 : vec3<f32>,
     value1_25 : f32,
     value2_19 : f32,
};

struct s_bwdCallableCtx_clip_chroma_black_0
{
     value0_105 : s_paramCtx_s_bwdCallableCtx_clip_chroma_black_0,
};

struct s_paramCtx_s_bwdCallableCtx_chroma_denominator_0
{
     value0_106 : vec3<f32>,
};

struct s_bwdCallableCtx_chroma_denominator_0
{
     value0_107 : s_paramCtx_s_bwdCallableCtx_chroma_denominator_0,
};

fn s_bwdProp_chroma_denominator_0( _S1238 : s_bwdCallableCtx_chroma_denominator_0,  _S1239 : ptr<function, f32>,  _S1240 : ptr<function, f32>,  _S1241 : f32)
{
    var _S1242 : f32 = _S1238.value0_107.value0_106.z * - _S1241;
    var _S1243 : f32 = _S1238.value0_107.value0_106.y * _S1241;
    var _S1244 : f32 = _S1238.value0_107.value0_106.x * _S1241;
    var _S1245 : f32 = _S1242 + 0.60824739933013916f * _S1243 + 0.39175257086753845f * _S1244;
    (*_S1239) = _S1242 + 0.020618556067347527f * _S1243 + 0.97938144207000732f * _S1244;
    (*_S1240) = _S1245;
    return;
}

fn s_bwdProp_clip_chroma_black_0( _S1246 : s_bwdCallableCtx_clip_chroma_black_0,  _S1247 : ptr<function, f32>,  _S1248 : ptr<function, f32>,  _S1249 : f32)
{
    var _S1250 : Tuple_40 = s_apply_chroma_denominator_0(_S1246.value0_105.value0_104, _S1246.value0_105.value1_25, _S1246.value0_105.value2_19);
    var _S1251 : s_paramCtx_s_bwdCallableCtx_chroma_denominator_0;
    _S1251.value0_106 = _S1246.value0_105.value0_104;
    var _S1252 : s_bwdCallableCtx_chroma_denominator_0 = s_bwdCallableCtx_chroma_denominator_0( _S1251 );
    var _S1253 : bool = !((_S1250.value0_90) == 0.0f);
    var _S1254 : bool;
    var _S1255 : f32;
    var _S1256 : f32;
    if(_S1253)
    {
        var numerator_1 : f32 = -0.42750686407089233f * (_S1246.value0_105.value0_104.x + 0.85649234056472778f * _S1246.value0_105.value0_104.y + 0.55499595403671265f * _S1246.value0_105.value0_104.z);
        var _S1257 : f32 = _S1250.value0_90 * _S1250.value0_90;
        _S1254 = ((numerator_1 / _S1250.value0_90) >= 0.0f);
        _S1255 = _S1257;
        _S1256 = numerator_1;
    }
    else
    {
        _S1254 = false;
        _S1255 = 0.0f;
        _S1256 = 0.0f;
    }
    if(_S1253)
    {
        var _S1258 : f32;
        if(_S1254)
        {
            _S1258 = _S1249;
        }
        else
        {
            _S1258 = 0.0f;
        }
        _S1255 = _S1256 * - (_S1258 / _S1255);
    }
    else
    {
        _S1255 = 0.0f;
    }
    var _S1259 : f32 = 0.0f;
    var _S1260 : f32 = 0.0f;
    s_bwdProp_chroma_denominator_0(_S1252, &(_S1259), &(_S1260), _S1255);
    (*_S1247) = _S1259;
    (*_S1248) = _S1260;
    return;
}

struct s_paramCtx_s_bwdCallableCtx_clip_chroma_white_raw_0
{
     value0_108 : vec3<f32>,
     value1_26 : f32,
     value2_20 : f32,
     value3_7 : f32,
     value4_4 : f32,
};

struct s_bwdCallableCtx_clip_chroma_white_raw_0
{
     value0_109 : s_paramCtx_s_bwdCallableCtx_clip_chroma_white_raw_0,
};

fn s_bwdProp_clip_chroma_white_raw_0( _S1261 : s_bwdCallableCtx_clip_chroma_white_raw_0,  _S1262 : ptr<function, f32>,  _S1263 : ptr<function, f32>,  _S1264 : ptr<function, f32>,  _S1265 : ptr<function, f32>,  _S1266 : f32)
{
    var _S1267 : Tuple_40 = s_apply_chroma_denominator_0(_S1261.value0_109.value0_108, _S1261.value0_109.value3_7, _S1261.value0_109.value4_4);
    var _S1268 : s_paramCtx_s_bwdCallableCtx_chroma_denominator_0;
    _S1268.value0_106 = _S1261.value0_109.value0_108;
    var _S1269 : s_bwdCallableCtx_chroma_denominator_0 = s_bwdCallableCtx_chroma_denominator_0( _S1268 );
    var _S1270 : f32 = 0.68285983800888062f * _S1261.value0_109.value3_7 + 0.4821370542049408f * _S1261.value0_109.value4_4;
    var denominator_target_1 : f32 = _S1261.value0_109.value1_26 * _S1270;
    var _S1271 : bool = !((_S1267.value0_90) == 0.0f);
    var _runFlag_10 : bool;
    var _S1272 : f32;
    var _S1273 : f32;
    var _S1274 : f32;
    var _S1275 : f32;
    if(_S1271)
    {
        if(((_S1261.value0_109.value2_20) <= (denominator_target_1 / _S1267.value0_90)))
        {
            _runFlag_10 = false;
        }
        else
        {
            _runFlag_10 = _S1271;
        }
        if(_runFlag_10)
        {
            var denominator_3 : f32 = _S1261.value0_109.value2_20 * _S1267.value0_90 - denominator_target_1;
            var _S1276 : f32 = _S1261.value0_109.value0_108.x + 0.85649234056472778f * _S1261.value0_109.value0_108.y + 0.55499595403671265f * _S1261.value0_109.value0_108.z;
            var numerator_2 : f32 = -0.42750686407089233f * (_S1261.value0_109.value2_20 * _S1276 - 0.98823773860931396f * _S1261.value0_109.value1_26);
            _S1272 = denominator_3 * denominator_3;
            _S1273 = numerator_2;
            _S1274 = denominator_3;
            _S1275 = _S1276;
        }
        else
        {
            _S1272 = 0.0f;
            _S1273 = 0.0f;
            _S1274 = 0.0f;
            _S1275 = 0.0f;
        }
    }
    else
    {
        _runFlag_10 = false;
        _S1272 = 0.0f;
        _S1273 = 0.0f;
        _S1274 = 0.0f;
        _S1275 = 0.0f;
    }
    if(_S1271)
    {
        if(_runFlag_10)
        {
            var _S1277 : f32 = _S1266 / _S1272;
            var s_diff_denominator_T_0 : f32 = _S1273 * - _S1277;
            var _S1278 : f32 = -0.42750686407089233f * (_S1274 * _S1277);
            var _S1279 : f32 = 0.98823773860931396f * - _S1278;
            var _S1280 : f32 = - s_diff_denominator_T_0;
            var _S1281 : f32 = _S1275 * _S1278 + _S1267.value0_90 * s_diff_denominator_T_0;
            _S1272 = _S1261.value0_109.value2_20 * s_diff_denominator_T_0;
            _S1273 = _S1280;
            _S1274 = _S1279;
            _S1275 = _S1281;
        }
        else
        {
            _S1272 = 0.0f;
            _S1273 = 0.0f;
            _S1274 = 0.0f;
            _S1275 = 0.0f;
        }
        var _S1282 : f32 = denominator_target_1 * - 0.0f + _S1272;
        _S1272 = _S1273;
        _S1273 = _S1282;
    }
    else
    {
        _S1272 = 0.0f;
        _S1273 = 0.0f;
        _S1274 = 0.0f;
        _S1275 = 0.0f;
    }
    var _S1283 : f32 = _S1261.value0_109.value1_26 * _S1272;
    var _S1284 : f32 = _S1270 * _S1272;
    var _S1285 : f32 = 0.4821370542049408f * _S1283;
    var _S1286 : f32 = 0.68285983800888062f * _S1283;
    var _S1287 : f32 = 0.0f;
    var _S1288 : f32 = 0.0f;
    s_bwdProp_chroma_denominator_0(_S1269, &(_S1287), &(_S1288), _S1273);
    var _S1289 : f32 = _S1285 + _S1288;
    var _S1290 : f32 = _S1286 + _S1287;
    (*_S1262) = _S1284 + _S1274;
    (*_S1263) = _S1275;
    (*_S1264) = _S1290;
    (*_S1265) = _S1289;
    return;
}

fn s_bwdProp_clip_chroma_white_0( _S1291 : s_bwdCallableCtx_clip_chroma_white_0,  _S1292 : ptr<function, f32>,  _S1293 : ptr<function, f32>,  _S1294 : ptr<function, f32>,  _S1295 : ptr<function, f32>,  _S1296 : f32)
{
    var _S1297 : s_paramCtx_s_bwdCallableCtx_clip_chroma_white_raw_0 = s_paramCtx_s_bwdCallableCtx_clip_chroma_white_raw_0( vec3<f32>(0.0f), 0.0f, 0.0f, 0.0f, 0.0f );
    var max_y_1 : f32 = 1.0578552484512329f * _S1291.value0_103.value1_24;
    var _S1298 : f32 = max_y_1 - _S1291.value0_103.value2_18;
    var _S1299 : Tuple_13 = s_apply_max_1(_S1298, 0.0f);
    var _S1300 : s_bwdCallableCtx_max_1;
    _S1300._S101 = _S1298;
    _S1300._S102 = 0.0f;
    var _S1301 : s_bwdCallableCtx_max_1 = _S1300;
    var _S1302 : bool = ((_S1299.value0_15) < 0.0010000000474974513f);
    var max_chroma_2 : f32;
    var _S1303 : f32;
    var _S1304 : f32;
    var _S1305 : f32;
    var _S1306 : f32;
    var _runFlag_11 : bool;
    var _S1307 : bool;
    var _S1308 : s_bwdCallableCtx_clip_chroma_white_raw_0;
    var _S1309 : s_bwdCallableCtx_clip_chroma_white_raw_0;
    if(_S1302)
    {
        var _S1310 : f32 = 0.99900001287460327f * max_y_1;
        var _S1311 : Tuple_39 = s_apply_clip_chroma_white_raw_0(_S1291.value0_103.value0_102, _S1291.value0_103.value1_24, _S1310, _S1291.value0_103.value3_6, _S1291.value0_103.value4_3);
        var _S1312 : s_paramCtx_s_bwdCallableCtx_clip_chroma_white_raw_0;
        _S1312.value0_108 = _S1291.value0_103.value0_102;
        _S1312.value1_26 = _S1291.value0_103.value1_24;
        _S1312.value2_20 = _S1310;
        _S1312.value3_7 = _S1291.value0_103.value3_6;
        _S1312.value4_4 = _S1291.value0_103.value4_3;
        var _S1313 : s_paramCtx_s_bwdCallableCtx_clip_chroma_white_raw_0 = _S1312;
        var _S1314 : bool = !((_S1311.value0_89) >= 3.4028234663852886e+38f);
        if(_S1314)
        {
            var _S1315 : f32 = 0.0010000000474974513f * max_y_1;
            var _S1316 : f32 = _S1299.value0_15 / _S1315;
            var _S1317 : f32 = _S1315 * _S1315;
            max_chroma_2 = _S1316 * _S1311.value0_89;
            _S1303 = _S1316;
            _S1304 = _S1317;
            _S1305 = _S1315;
        }
        else
        {
            _S1303 = 0.0f;
            _S1304 = 0.0f;
            _S1305 = 0.0f;
        }
        var _S1318 : f32 = _S1304;
        var _S1319 : f32 = _S1305;
        _runFlag_11 = _S1314;
        _S1307 = _S1314;
        _S1304 = _S1311.value0_89;
        _S1305 = _S1318;
        _S1306 = _S1319;
        _S1308.value0_109 = _S1313;
        _S1309.value0_109 = _S1297;
    }
    else
    {
        var _S1320 : Tuple_39 = s_apply_clip_chroma_white_raw_0(_S1291.value0_103.value0_102, _S1291.value0_103.value1_24, _S1291.value0_103.value2_18, _S1291.value0_103.value3_6, _S1291.value0_103.value4_3);
        var _S1321 : s_paramCtx_s_bwdCallableCtx_clip_chroma_white_raw_0;
        _S1321.value0_108 = _S1291.value0_103.value0_102;
        _S1321.value1_26 = _S1291.value0_103.value1_24;
        _S1321.value2_20 = _S1291.value0_103.value2_18;
        _S1321.value3_7 = _S1291.value0_103.value3_6;
        _S1321.value4_4 = _S1291.value0_103.value4_3;
        _runFlag_11 = true;
        max_chroma_2 = _S1320.value0_89;
        _S1307 = false;
        _S1303 = 0.0f;
        _S1304 = 0.0f;
        _S1305 = 0.0f;
        _S1306 = 0.0f;
        _S1308.value0_109 = _S1297;
        _S1309.value0_109 = _S1321;
    }
    var _S1322 : bool;
    if(_runFlag_11)
    {
        _S1322 = (max_chroma_2 >= 0.0f);
    }
    else
    {
        _S1322 = false;
    }
    if(_runFlag_11)
    {
        if(_S1322)
        {
            max_chroma_2 = _S1296;
        }
        else
        {
            max_chroma_2 = 0.0f;
        }
    }
    else
    {
        max_chroma_2 = 0.0f;
    }
    var _S1323 : f32;
    if(_S1302)
    {
        if(_S1307)
        {
            var _S1324 : f32 = _S1304 * max_chroma_2 / _S1305;
            var _S1325 : f32 = _S1306 * _S1324;
            var _S1326 : f32 = 0.0010000000474974513f * (_S1299.value0_15 * - _S1324);
            max_chroma_2 = _S1303 * max_chroma_2;
            _S1303 = _S1326;
            _S1304 = _S1325;
        }
        else
        {
            max_chroma_2 = 0.0f;
            _S1303 = 0.0f;
            _S1304 = 0.0f;
        }
        var _S1327 : f32 = 0.0f;
        var _S1328 : f32 = 0.0f;
        var _S1329 : f32 = 0.0f;
        var _S1330 : f32 = 0.0f;
        s_bwdProp_clip_chroma_white_raw_0(_S1308, &(_S1327), &(_S1328), &(_S1329), &(_S1330), max_chroma_2);
        var _S1331 : f32 = 0.99900001287460327f * _S1328 + _S1303;
        max_chroma_2 = _S1304;
        _S1303 = _S1331;
        _S1304 = 0.0f;
        _S1305 = _S1327;
        _S1306 = _S1329;
        _S1323 = _S1330;
    }
    else
    {
        var _S1332 : f32 = 0.0f;
        var _S1333 : f32 = 0.0f;
        var _S1334 : f32 = 0.0f;
        var _S1335 : f32 = 0.0f;
        s_bwdProp_clip_chroma_white_raw_0(_S1309, &(_S1332), &(_S1333), &(_S1334), &(_S1335), max_chroma_2);
        max_chroma_2 = 0.0f;
        _S1303 = 0.0f;
        _S1304 = _S1333;
        _S1305 = _S1332;
        _S1306 = _S1334;
        _S1323 = _S1335;
    }
    var _S1336 : f32 = 0.0f;
    var _S1337 : f32 = 0.0f;
    s_bwdProp_max_1(_S1301, &(_S1336), &(_S1337), max_chroma_2);
    var _S1338 : f32 = - _S1336 + _S1304;
    (*_S1292) = 1.0578552484512329f * (_S1336 + _S1303) + _S1305;
    (*_S1293) = _S1338;
    (*_S1294) = _S1306;
    (*_S1295) = _S1323;
    return;
}

fn s_bwdProp_ych_max_chroma_0( _S1339 : s_bwdCallableCtx_ych_max_chroma_0,  _S1340 : ptr<function, f32>,  _S1341 : ptr<function, f32>,  _S1342 : ptr<function, f32>,  _S1343 : ptr<function, f32>,  _S1344 : f32)
{
    var _S1345 : Tuple_38 = s_apply_clip_chroma_white_0(_S1339.value0_97.value0_96[i32(0)], _S1339.value0_97.value1_23, _S1339.value0_97.value2_17, _S1339.value0_97.value3_5, _S1339.value0_97.value4_2);
    var _S1346 : s_paramCtx_s_bwdCallableCtx_clip_chroma_white_0;
    _S1346.value0_102 = _S1339.value0_97.value0_96[i32(0)];
    _S1346.value1_24 = _S1339.value0_97.value1_23;
    _S1346.value2_18 = _S1339.value0_97.value2_17;
    _S1346.value3_6 = _S1339.value0_97.value3_5;
    _S1346.value4_3 = _S1339.value0_97.value4_2;
    var _S1347 : s_bwdCallableCtx_clip_chroma_white_0 = s_bwdCallableCtx_clip_chroma_white_0( _S1346 );
    var _S1348 : Tuple_38 = s_apply_clip_chroma_white_0(_S1339.value0_97.value0_96[i32(1)], _S1339.value0_97.value1_23, _S1339.value0_97.value2_17, _S1339.value0_97.value3_5, _S1339.value0_97.value4_2);
    var _S1349 : s_paramCtx_s_bwdCallableCtx_clip_chroma_white_0;
    _S1349.value0_102 = _S1339.value0_97.value0_96[i32(1)];
    _S1349.value1_24 = _S1339.value0_97.value1_23;
    _S1349.value2_18 = _S1339.value0_97.value2_17;
    _S1349.value3_6 = _S1339.value0_97.value3_5;
    _S1349.value4_3 = _S1339.value0_97.value4_2;
    var _S1350 : s_bwdCallableCtx_clip_chroma_white_0 = s_bwdCallableCtx_clip_chroma_white_0( _S1349 );
    var _S1351 : Tuple_14 = s_apply_min_0(_S1345.value0_88, _S1348.value0_88);
    var _S1352 : s_bwdCallableCtx_min_0;
    _S1352._S106 = _S1345.value0_88;
    _S1352._S107 = _S1348.value0_88;
    var _S1353 : Tuple_38 = s_apply_clip_chroma_white_0(_S1339.value0_97.value0_96[i32(2)], _S1339.value0_97.value1_23, _S1339.value0_97.value2_17, _S1339.value0_97.value3_5, _S1339.value0_97.value4_2);
    var _S1354 : s_paramCtx_s_bwdCallableCtx_clip_chroma_white_0;
    _S1354.value0_102 = _S1339.value0_97.value0_96[i32(2)];
    _S1354.value1_24 = _S1339.value0_97.value1_23;
    _S1354.value2_18 = _S1339.value0_97.value2_17;
    _S1354.value3_6 = _S1339.value0_97.value3_5;
    _S1354.value4_3 = _S1339.value0_97.value4_2;
    var _S1355 : s_bwdCallableCtx_clip_chroma_white_0 = s_bwdCallableCtx_clip_chroma_white_0( _S1354 );
    var _S1356 : Tuple_14 = s_apply_min_0(_S1351.value0_16, _S1353.value0_88);
    var _S1357 : s_bwdCallableCtx_min_0;
    _S1357._S106 = _S1351.value0_16;
    _S1357._S107 = _S1353.value0_88;
    var _S1358 : Tuple_41 = s_apply_clip_chroma_black_0(_S1339.value0_97.value0_96[i32(0)], _S1339.value0_97.value3_5, _S1339.value0_97.value4_2);
    var _S1359 : s_paramCtx_s_bwdCallableCtx_clip_chroma_black_0;
    _S1359.value0_104 = _S1339.value0_97.value0_96[i32(0)];
    _S1359.value1_25 = _S1339.value0_97.value3_5;
    _S1359.value2_19 = _S1339.value0_97.value4_2;
    var _S1360 : s_bwdCallableCtx_clip_chroma_black_0 = s_bwdCallableCtx_clip_chroma_black_0( _S1359 );
    var _S1361 : Tuple_41 = s_apply_clip_chroma_black_0(_S1339.value0_97.value0_96[i32(1)], _S1339.value0_97.value3_5, _S1339.value0_97.value4_2);
    var _S1362 : s_paramCtx_s_bwdCallableCtx_clip_chroma_black_0;
    _S1362.value0_104 = _S1339.value0_97.value0_96[i32(1)];
    _S1362.value1_25 = _S1339.value0_97.value3_5;
    _S1362.value2_19 = _S1339.value0_97.value4_2;
    var _S1363 : s_bwdCallableCtx_clip_chroma_black_0 = s_bwdCallableCtx_clip_chroma_black_0( _S1362 );
    var _S1364 : Tuple_14 = s_apply_min_0(_S1358.value0_91, _S1361.value0_91);
    var _S1365 : s_bwdCallableCtx_min_0;
    _S1365._S106 = _S1358.value0_91;
    _S1365._S107 = _S1361.value0_91;
    var _S1366 : Tuple_41 = s_apply_clip_chroma_black_0(_S1339.value0_97.value0_96[i32(2)], _S1339.value0_97.value3_5, _S1339.value0_97.value4_2);
    var _S1367 : s_paramCtx_s_bwdCallableCtx_clip_chroma_black_0;
    _S1367.value0_104 = _S1339.value0_97.value0_96[i32(2)];
    _S1367.value1_25 = _S1339.value0_97.value3_5;
    _S1367.value2_19 = _S1339.value0_97.value4_2;
    var _S1368 : s_bwdCallableCtx_clip_chroma_black_0 = s_bwdCallableCtx_clip_chroma_black_0( _S1367 );
    var _S1369 : Tuple_14 = s_apply_min_0(_S1364.value0_16, _S1366.value0_91);
    var _S1370 : s_bwdCallableCtx_min_0;
    _S1370._S106 = _S1364.value0_16;
    _S1370._S107 = _S1366.value0_91;
    var _S1371 : s_bwdCallableCtx_min_0;
    _S1371._S106 = _S1369.value0_16;
    _S1371._S107 = _S1356.value0_16;
    var _S1372 : f32 = 0.0f;
    var _S1373 : f32 = 0.0f;
    s_bwdProp_min_0(_S1371, &(_S1372), &(_S1373), _S1344);
    var _S1374 : f32 = 0.0f;
    var _S1375 : f32 = 0.0f;
    s_bwdProp_min_0(_S1370, &(_S1374), &(_S1375), _S1372);
    var _S1376 : f32 = 0.0f;
    var _S1377 : f32 = 0.0f;
    s_bwdProp_clip_chroma_black_0(_S1368, &(_S1376), &(_S1377), _S1375);
    var _S1378 : f32 = 0.0f;
    var _S1379 : f32 = 0.0f;
    s_bwdProp_min_0(_S1365, &(_S1378), &(_S1379), _S1374);
    var _S1380 : f32 = 0.0f;
    var _S1381 : f32 = 0.0f;
    s_bwdProp_clip_chroma_black_0(_S1363, &(_S1380), &(_S1381), _S1379);
    var _S1382 : f32 = 0.0f;
    var _S1383 : f32 = 0.0f;
    s_bwdProp_clip_chroma_black_0(_S1360, &(_S1382), &(_S1383), _S1378);
    var _S1384 : f32 = 0.0f;
    var _S1385 : f32 = 0.0f;
    s_bwdProp_min_0(_S1357, &(_S1384), &(_S1385), _S1373);
    var _S1386 : f32 = 0.0f;
    var _S1387 : f32 = 0.0f;
    var _S1388 : f32 = 0.0f;
    var _S1389 : f32 = 0.0f;
    s_bwdProp_clip_chroma_white_0(_S1355, &(_S1386), &(_S1387), &(_S1388), &(_S1389), _S1385);
    var _S1390 : f32 = 0.0f;
    var _S1391 : f32 = 0.0f;
    s_bwdProp_min_0(_S1352, &(_S1390), &(_S1391), _S1384);
    var _S1392 : f32 = 0.0f;
    var _S1393 : f32 = 0.0f;
    var _S1394 : f32 = 0.0f;
    var _S1395 : f32 = 0.0f;
    s_bwdProp_clip_chroma_white_0(_S1350, &(_S1392), &(_S1393), &(_S1394), &(_S1395), _S1391);
    var _S1396 : f32 = 0.0f;
    var _S1397 : f32 = 0.0f;
    var _S1398 : f32 = 0.0f;
    var _S1399 : f32 = 0.0f;
    s_bwdProp_clip_chroma_white_0(_S1347, &(_S1396), &(_S1397), &(_S1398), &(_S1399), _S1390);
    var _S1400 : f32 = _S1376 + _S1380 + _S1382 + _S1388 + _S1394 + _S1398;
    var _S1401 : f32 = _S1377 + _S1381 + _S1383 + _S1389 + _S1395 + _S1399;
    var _S1402 : f32 = _S1387 + _S1393 + _S1397;
    (*_S1340) = _S1386 + _S1392 + _S1396;
    (*_S1341) = _S1402;
    (*_S1342) = _S1400;
    (*_S1343) = _S1401;
    return;
}

struct s_paramCtx_s_bwdCallableCtx_lms_to_yrg_0
{
     value0_110 : vec3<f32>,
};

struct s_paramCtx_s_bwdCallableCtx_yrg_to_ych_0
{
     value0_111 : vec3<f32>,
};

struct s_bwdCallableCtx_yrg_to_ych_0
{
     value0_112 : s_paramCtx_s_bwdCallableCtx_yrg_to_ych_0,
};

struct s_bwdCallableCtx_lms_to_yrg_0
{
     value0_113 : s_paramCtx_s_bwdCallableCtx_lms_to_yrg_0,
};

fn s_bwdProp_yrg_to_ych_0( _S1403 : s_bwdCallableCtx_yrg_to_ych_0,  _S1404 : ptr<function, vec3<f32>>,  _S1405 : vec4<f32>)
{
    var rg_2 : vec2<f32> = _S1403.value0_112.value0_111.yz - vec2<f32>(0.21902142465114594f, 0.54371398687362671f);
    var _S1406 : Tuple_1 = s_apply_length_0(rg_2);
    var _S1407 : s_bwdCallableCtx_length_0;
    _S1407._S6 = rg_2;
    var _S1408 : s_bwdCallableCtx_length_0 = _S1407;
    var _S1409 : bool = (_S1406.value0_1) != 0.0f;
    var _S1410 : f32;
    if(_S1409)
    {
        _S1410 = _S1406.value0_1 * _S1406.value0_1;
    }
    else
    {
        _S1410 = 0.0f;
    }
    var _S1411 : vec2<f32> = vec2<f32>(0.0f);
    var _S1412 : vec2<f32> = vec2<f32>(_S1405[i32(2)], _S1405[i32(3)]);
    var _S1413 : vec3<f32> = vec3<f32>(_S1405[i32(0)], 0.0f, 0.0f);
    var _S1414 : vec2<f32>;
    if(_S1409)
    {
        var _S1415 : vec2<f32> = _S1412 / vec2<f32>(_S1410);
        var _S1416 : vec2<f32> = rg_2 * (vec2<f32>(0) - _S1415);
        var _S1417 : vec2<f32> = vec2<f32>(_S1406.value0_1) * _S1415;
        _S1410 = _S1416[i32(0)] + _S1416[i32(1)] + _S1405[i32(1)];
        _S1414 = _S1417;
    }
    else
    {
        _S1410 = _S1405[i32(1)];
        _S1414 = _S1411;
    }
    var _S1418 : vec2<f32> = _S1411;
    s_bwdProp_length_0(_S1408, &(_S1418), _S1410);
    var _S1419 : vec2<f32> = _S1418 + _S1414;
    (*_S1404) = _S1413 + vec3<f32>(0.0f, _S1419[i32(0)], _S1419[i32(1)]);
    return;
}

fn s_bwdProp_lms_to_yrg_0( _S1420 : s_bwdCallableCtx_lms_to_yrg_0,  _S1421 : ptr<function, vec3<f32>>,  _S1422 : vec3<f32>)
{
    var a_3 : f32 = _S1420.value0_113.value0_110.x + _S1420.value0_113.value0_110.y + _S1420.value0_113.value0_110.z;
    var _S1423 : vec3<f32> = vec3<f32>(0.0f);
    var _S1424 : bool = a_3 != 0.0f;
    var normalized_1 : vec3<f32>;
    var _S1425 : f32;
    if(_S1424)
    {
        var _S1426 : f32 = a_3 * a_3;
        normalized_1 = _S1420.value0_113.value0_110 / vec3<f32>(a_3);
        _S1425 = _S1426;
    }
    else
    {
        normalized_1 = _S1423;
        _S1425 = 0.0f;
    }
    var _S1427 : s_bwdCallableCtx_mul_0;
    _S1427._S40 = mat3x3<f32>(1.0877193212509155f, -0.66666668653488159f, 0.020618559792637825f, -0.087719298899173737f, 1.6666666269302368f, -0.051546391099691391f, 0.0f, 0.0f, 1.0309278964996338f);
    _S1427._S41 = normalized_1;
    var _S1428 : s_bwdCallableCtx_mul_0 = _S1427;
    var _S1429 : vec3<f32> = vec3<f32>(0.0f);
    var _S1430 : vec3<f32> = vec3<f32>(_S1422[i32(1)], _S1422[i32(2)], 0.0f);
    var _S1431 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S1432 : vec3<f32> = _S1429;
    s_bwdProp_mul_0(_S1428, &(_S1431), &(_S1432), _S1430);
    var _S1433 : vec3<f32> = _S1432;
    if(_S1424)
    {
        var _S1434 : vec3<f32> = _S1433 / vec3<f32>(_S1425);
        var _S1435 : vec3<f32> = _S1420.value0_113.value0_110 * (vec3<f32>(0) - _S1434);
        var _S1436 : vec3<f32> = vec3<f32>(a_3) * _S1434;
        _S1425 = _S1435[i32(0)] + _S1435[i32(1)] + _S1435[i32(2)];
        normalized_1 = _S1436;
    }
    else
    {
        _S1425 = 0.0f;
        normalized_1 = _S1429;
    }
    (*_S1421) = normalized_1 + vec3<f32>(_S1425 + 0.68990272283554077f * _S1422[i32(0)], _S1425 + 0.34832188487052917f * _S1422[i32(0)], _S1425);
    return;
}

fn s_bwdProp_rgb_to_ych_0( _S1437 : s_bwdCallableCtx_rgb_to_ych_0,  _S1438 : ptr<function, vec3<f32>>,  _S1439 : vec4<f32>)
{
    var _S1440 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S1441 : vec3<f32> = vec3<f32>(0.0f);
    var _S1442 : s_paramCtx_s_bwdCallableCtx_lms_to_yrg_0 = s_paramCtx_s_bwdCallableCtx_lms_to_yrg_0( _S1441 );
    var _S1443 : s_paramCtx_s_bwdCallableCtx_yrg_to_ych_0 = s_paramCtx_s_bwdCallableCtx_yrg_to_ych_0( _S1441 );
    var _S1444 : f32 = _S1437.value0_70.value0_64.x;
    var _S1445 : Tuple_6 = s_apply_abs_1(_S1444);
    var _S1446 : s_bwdCallableCtx_abs_1;
    _S1446._S48 = _S1444;
    var _S1447 : s_bwdCallableCtx_abs_1 = _S1446;
    var _S1448 : f32 = _S1437.value0_70.value0_64.y;
    var _S1449 : Tuple_6 = s_apply_abs_1(_S1448);
    var _S1450 : s_bwdCallableCtx_abs_1;
    _S1450._S48 = _S1448;
    var _S1451 : s_bwdCallableCtx_abs_1 = _S1450;
    var _S1452 : Tuple_13 = s_apply_max_1(_S1445.value0_8, _S1449.value0_8);
    var _S1453 : s_bwdCallableCtx_max_1;
    _S1453._S101 = _S1445.value0_8;
    _S1453._S102 = _S1449.value0_8;
    var _S1454 : s_bwdCallableCtx_max_1 = _S1453;
    var _S1455 : f32 = _S1437.value0_70.value0_64.z;
    var _S1456 : Tuple_6 = s_apply_abs_1(_S1455);
    var _S1457 : s_bwdCallableCtx_abs_1;
    _S1457._S48 = _S1455;
    var _S1458 : s_bwdCallableCtx_abs_1 = _S1457;
    var _S1459 : Tuple_13 = s_apply_max_1(_S1452.value0_15, _S1456.value0_8);
    var _S1460 : s_bwdCallableCtx_max_1;
    _S1460._S101 = _S1452.value0_15;
    _S1460._S102 = _S1456.value0_8;
    var _S1461 : s_bwdCallableCtx_max_1 = _S1460;
    var _S1462 : bool = ((_S1459.value0_15) <= 0.0f);
    var _S1463 : s_bwdCallableCtx_yrg_to_ych_0;
    var _S1464 : s_bwdCallableCtx_lms_to_yrg_0;
    var _S1465 : s_bwdCallableCtx_mul_0;
    if(_S1462)
    {
        var _S1466 : Tuple_5 = s_apply_mul_0(_S1437.value0_70.value1_17, _S1437.value0_70.value0_64);
        var _S1467 : s_bwdCallableCtx_mul_0;
        _S1467._S40 = _S1437.value0_70.value1_17;
        _S1467._S41 = _S1437.value0_70.value0_64;
        var _S1468 : Tuple_26 = s_apply_lms_to_yrg_0(_S1466.value0_7);
        var _S1469 : s_paramCtx_s_bwdCallableCtx_lms_to_yrg_0;
        _S1469.value0_110 = _S1466.value0_7;
        var _S1470 : s_paramCtx_s_bwdCallableCtx_yrg_to_ych_0;
        _S1470.value0_111 = _S1468.value0_68;
        _S1463.value0_112 = _S1470;
        _S1464.value0_113 = _S1469;
        _S1465 = _S1467;
    }
    else
    {
        _S1463.value0_112 = _S1443;
        _S1464.value0_113 = _S1442;
        _S1465._S40 = _S1440;
        _S1465._S41 = _S1441;
    }
    var _S1471 : bool = !_S1462;
    var _S1472 : s_bwdCallableCtx_yrg_to_ych_0;
    var _S1473 : s_bwdCallableCtx_lms_to_yrg_0;
    var _S1474 : s_bwdCallableCtx_mul_0;
    if(_S1471)
    {
        var _S1475 : vec3<f32> = _S1437.value0_70.value0_64 / vec3<f32>(_S1459.value0_15);
        var _S1476 : Tuple_5 = s_apply_mul_0(_S1437.value0_70.value1_17, _S1475);
        var _S1477 : s_bwdCallableCtx_mul_0;
        _S1477._S40 = _S1437.value0_70.value1_17;
        _S1477._S41 = _S1475;
        var _S1478 : Tuple_26 = s_apply_lms_to_yrg_0(_S1476.value0_7);
        var _S1479 : s_paramCtx_s_bwdCallableCtx_lms_to_yrg_0;
        _S1479.value0_110 = _S1476.value0_7;
        var _S1480 : s_paramCtx_s_bwdCallableCtx_yrg_to_ych_0;
        _S1480.value0_111 = _S1478.value0_68;
        _S1472.value0_112 = _S1480;
        _S1473.value0_113 = _S1479;
        _S1474 = _S1477;
    }
    else
    {
        _S1472.value0_112 = _S1443;
        _S1473.value0_113 = _S1442;
        _S1474._S40 = _S1440;
        _S1474._S41 = _S1441;
    }
    var _S1481 : vec4<f32> = vec4<f32>(0.0f);
    var _S1482 : vec4<f32>;
    var _S1483 : vec3<f32>;
    if(_S1471)
    {
        var _S1484 : vec4<f32> = vec4<f32>(_S1459.value0_15 * _S1439[i32(0)], _S1439[i32(1)], _S1439[i32(2)], _S1439[i32(3)]);
        var _S1485 : vec3<f32> = _S1441;
        s_bwdProp_yrg_to_ych_0(_S1472, &(_S1485), _S1484);
        var _S1486 : vec3<f32> = _S1441;
        s_bwdProp_lms_to_yrg_0(_S1473, &(_S1486), _S1485);
        var _S1487 : mat3x3<f32> = _S1440;
        var _S1488 : vec3<f32> = _S1441;
        s_bwdProp_mul_0(_S1474, &(_S1487), &(_S1488), _S1486);
        var _S1489 : vec3<f32> = _S1488 / vec3<f32>(_S1459.value0_15);
        _S1482 = _S1481;
        _S1483 = _S1489;
    }
    else
    {
        _S1482 = _S1439;
        _S1483 = _S1441;
    }
    if(_S1462)
    {
        var _S1490 : vec3<f32> = _S1441;
        s_bwdProp_yrg_to_ych_0(_S1463, &(_S1490), _S1482);
        var _S1491 : vec3<f32> = _S1441;
        s_bwdProp_lms_to_yrg_0(_S1464, &(_S1491), _S1490);
        var _S1492 : mat3x3<f32> = _S1440;
        var _S1493 : vec3<f32> = _S1441;
        s_bwdProp_mul_0(_S1465, &(_S1492), &(_S1493), _S1491);
        _S1483 = _S1493 + _S1483;
    }
    var _S1494 : f32 = 0.0f;
    var _S1495 : f32 = 0.0f;
    s_bwdProp_max_1(_S1461, &(_S1494), &(_S1495), 0.0f);
    var _S1496 : f32 = 0.0f;
    s_bwdProp_abs_1(_S1458, &(_S1496), _S1495);
    var _S1497 : f32 = 0.0f;
    var _S1498 : f32 = 0.0f;
    s_bwdProp_max_1(_S1454, &(_S1497), &(_S1498), _S1494);
    var _S1499 : f32 = 0.0f;
    s_bwdProp_abs_1(_S1451, &(_S1499), _S1498);
    var _S1500 : f32 = 0.0f;
    s_bwdProp_abs_1(_S1447, &(_S1500), _S1497);
    (*_S1438) = _S1483 + vec3<f32>(_S1500, _S1499, _S1496);
    return;
}

fn s_bwdProp_gamut_check_rgb_0( _S1501 : s_bwdCallableCtx_gamut_check_rgb_0,  _S1502 : ptr<function, vec4<f32>>,  _S1503 : ptr<function, f32>,  _S1504 : ptr<function, f32>,  _S1505 : vec3<f32>)
{
    var _S1506 : Tuple_35 = s_apply_ych_to_rgb_0(_S1501.value0_93.value0_92, _S1501.value0_93.value2_16);
    var _S1507 : s_paramCtx_s_bwdCallableCtx_ych_to_rgb_0;
    _S1507.value0_94 = _S1501.value0_93.value0_92;
    _S1507.value1_22 = _S1501.value0_93.value2_16;
    var _S1508 : s_bwdCallableCtx_ych_to_rgb_0 = s_bwdCallableCtx_ych_to_rgb_0( _S1507 );
    var _S1509 : f32 = _S1506.value0_85.x;
    var _S1510 : f32 = _S1506.value0_85.y;
    var _S1511 : Tuple_14 = s_apply_min_0(_S1509, _S1510);
    var _S1512 : s_bwdCallableCtx_min_0;
    _S1512._S106 = _S1509;
    _S1512._S107 = _S1510;
    var _S1513 : f32 = _S1506.value0_85.z;
    var _S1514 : Tuple_14 = s_apply_min_0(_S1511.value0_16, _S1513);
    var _S1515 : s_bwdCallableCtx_min_0;
    _S1515._S106 = _S1511.value0_16;
    _S1515._S107 = _S1513;
    var _S1516 : f32 = - _S1514.value0_16;
    var _S1517 : Tuple_13 = s_apply_max_1(_S1516, 0.0f);
    var _S1518 : s_bwdCallableCtx_max_1;
    _S1518._S101 = _S1516;
    _S1518._S102 = 0.0f;
    var brightened_0 : vec3<f32> = _S1506.value0_85 + vec3<f32>(_S1517.value0_15);
    var _S1519 : Tuple_25 = s_apply_rgb_to_ych_0(brightened_0, _S1501.value0_93.value1_21);
    var _S1520 : s_paramCtx_s_bwdCallableCtx_rgb_to_ych_0;
    _S1520.value0_64 = brightened_0;
    _S1520.value1_17 = _S1501.value0_93.value1_21;
    var _S1521 : s_bwdCallableCtx_rgb_to_ych_0 = s_bwdCallableCtx_rgb_to_ych_0( _S1520 );
    var _S1522 : f32 = (_S1501.value0_93.value0_92.x + _S1519.value0_67.x) / 2.0f;
    var _S1523 : f32 = 1.0578552484512329f * _S1501.value0_93.value3_4;
    var _S1524 : f32 = 1.0578552484512329f * _S1501.value0_93.value4_1;
    var _S1525 : Tuple_9 = s_apply_clamp_0(_S1522, _S1523, _S1524);
    var _S1526 : s_bwdCallableCtx_clamp_0;
    _S1526._S65 = _S1522;
    _S1526._S66 = _S1523;
    _S1526._S67 = _S1524;
    var cos_h_1 : f32 = _S1501.value0_93.value0_92.z;
    var sin_h_1 : f32 = _S1501.value0_93.value0_92.w;
    var _S1527 : f32 = _S1501.value0_93.value0_92.y;
    var _S1528 : Tuple_37 = s_apply_ych_max_chroma_0(_S1501.value0_93.value2_16, _S1501.value0_93.value4_1, _S1525.value0_11, cos_h_1, sin_h_1);
    var _S1529 : s_paramCtx_s_bwdCallableCtx_ych_max_chroma_0;
    _S1529.value0_96 = _S1501.value0_93.value2_16;
    _S1529.value1_23 = _S1501.value0_93.value4_1;
    _S1529.value2_17 = _S1525.value0_11;
    _S1529.value3_5 = cos_h_1;
    _S1529.value4_2 = sin_h_1;
    var _S1530 : s_bwdCallableCtx_ych_max_chroma_0 = s_bwdCallableCtx_ych_max_chroma_0( _S1529 );
    var _S1531 : Tuple_14 = s_apply_min_0(_S1527, _S1528.value0_87);
    var _S1532 : s_bwdCallableCtx_min_0;
    _S1532._S106 = _S1527;
    _S1532._S107 = _S1528.value0_87;
    var _S1533 : vec4<f32> = vec4<f32>(_S1525.value0_11, _S1531.value0_16, cos_h_1, sin_h_1);
    var _S1534 : Tuple_35 = s_apply_ych_to_rgb_0(_S1533, _S1501.value0_93.value2_16);
    var _S1535 : s_paramCtx_s_bwdCallableCtx_ych_to_rgb_0;
    _S1535.value0_94 = _S1533;
    _S1535.value1_22 = _S1501.value0_93.value2_16;
    var _S1536 : s_bwdCallableCtx_ych_to_rgb_0 = s_bwdCallableCtx_ych_to_rgb_0( _S1535 );
    var _S1537 : vec3<f32> = vec3<f32>(0.0f);
    var _S1538 : vec3<f32> = vec3<f32>(_S1501.value0_93.value4_1);
    var _S1539 : s_bwdCallableCtx_clamp_1;
    _S1539._S89 = _S1534.value0_85;
    _S1539._S90 = _S1537;
    _S1539._S91 = _S1538;
    var _S1540 : vec3<f32> = vec3<f32>(0.0f);
    var _S1541 : vec3<f32> = _S1540;
    var _S1542 : vec3<f32> = _S1540;
    var _S1543 : vec3<f32> = _S1540;
    s_bwdProp_clamp_1(_S1539, &(_S1541), &(_S1542), &(_S1543), _S1505);
    var _S1544 : f32 = _S1543[i32(0)] + _S1543[i32(1)] + _S1543[i32(2)];
    var _S1545 : vec4<f32> = vec4<f32>(0.0f);
    var _S1546 : vec4<f32> = _S1545;
    s_bwdProp_ych_to_rgb_0(_S1536, &(_S1546), _S1541);
    var _S1547 : f32 = 0.0f;
    var _S1548 : f32 = 0.0f;
    s_bwdProp_min_0(_S1532, &(_S1547), &(_S1548), _S1546[i32(1)]);
    var _S1549 : f32 = 0.0f;
    var _S1550 : f32 = 0.0f;
    var _S1551 : f32 = 0.0f;
    var _S1552 : f32 = 0.0f;
    s_bwdProp_ych_max_chroma_0(_S1530, &(_S1549), &(_S1550), &(_S1551), &(_S1552), _S1548);
    var _S1553 : f32 = _S1546[i32(3)] + _S1552;
    var _S1554 : f32 = _S1546[i32(2)] + _S1551;
    var _S1555 : f32 = _S1546[i32(0)] + _S1550;
    var _S1556 : f32 = 0.0f;
    var _S1557 : f32 = 0.0f;
    var _S1558 : f32 = 0.0f;
    s_bwdProp_clamp_0(_S1526, &(_S1556), &(_S1557), &(_S1558), _S1555);
    var _S1559 : f32 = 1.0578552484512329f * _S1558;
    var _S1560 : f32 = 1.0578552484512329f * _S1557;
    var _S1561 : f32 = 0.5f * _S1556;
    var _S1562 : vec4<f32> = vec4<f32>(_S1561, 0.0f, 0.0f, 0.0f);
    var _S1563 : vec3<f32> = _S1540;
    s_bwdProp_rgb_to_ych_0(_S1521, &(_S1563), _S1562);
    var _S1564 : f32 = _S1563[i32(0)] + _S1563[i32(1)] + _S1563[i32(2)];
    var _S1565 : f32 = 0.0f;
    var _S1566 : f32 = 0.0f;
    s_bwdProp_max_1(_S1518, &(_S1565), &(_S1566), _S1564);
    var _S1567 : f32 = - _S1565;
    var _S1568 : f32 = 0.0f;
    var _S1569 : f32 = 0.0f;
    s_bwdProp_min_0(_S1515, &(_S1568), &(_S1569), _S1567);
    var _S1570 : f32 = 0.0f;
    var _S1571 : f32 = 0.0f;
    s_bwdProp_min_0(_S1512, &(_S1570), &(_S1571), _S1568);
    var _S1572 : vec3<f32> = _S1563 + vec3<f32>(_S1570, _S1571, _S1569);
    var _S1573 : vec4<f32> = _S1545;
    s_bwdProp_ych_to_rgb_0(_S1508, &(_S1573), _S1572);
    var _S1574 : f32 = _S1544 + _S1549 + _S1559;
    (*_S1502) = _S1573 + vec4<f32>(_S1561, _S1547, _S1554, _S1553);
    (*_S1503) = _S1560;
    (*_S1504) = _S1574;
    return;
}

fn s_bwdProp_gamut_check_yrg_0( _S1575 : s_bwdCallableCtx_gamut_check_yrg_0,  _S1576 : ptr<function, vec4<f32>>,  _S1577 : vec4<f32>)
{
    var _S1578 : Tuple_33 = s_apply_ych_to_yrg_0(_S1575.value0_83.value0_82);
    var _S1579 : s_paramCtx_s_bwdCallableCtx_ych_to_yrg_0;
    _S1579.value0_98 = _S1575.value0_83.value0_82;
    var _S1580 : s_bwdCallableCtx_ych_to_yrg_0 = s_bwdCallableCtx_ych_to_yrg_0( _S1579 );
    var max_c_2 : f32 = _S1575.value0_83.value0_82.y;
    var _S1581 : f32 = _S1578.value0_81.y;
    var _S1582 : bool = (_S1581 < 0.0f);
    var max_c_3 : f32;
    var _S1583 : f32;
    var _S1584 : s_bwdCallableCtx_min_0;
    if(_S1582)
    {
        var _S1585 : f32 = _S1575.value0_83.value0_82.z;
        var _S1586 : f32 = -0.21902142465114594f / _S1585;
        var _S1587 : f32 = _S1585 * _S1585;
        var _S1588 : Tuple_14 = s_apply_min_0(_S1586, max_c_2);
        var _S1589 : s_bwdCallableCtx_min_0;
        _S1589._S106 = _S1586;
        _S1589._S107 = max_c_2;
        max_c_3 = _S1588.value0_16;
        _S1584 = _S1589;
        _S1583 = _S1587;
    }
    else
    {
        max_c_3 = max_c_2;
        _S1584._S106 = 0.0f;
        _S1584._S107 = 0.0f;
        _S1583 = 0.0f;
    }
    var _S1590 : f32 = _S1578.value0_81.z;
    var _S1591 : bool = (_S1590 < 0.0f);
    var _S1592 : f32;
    var _S1593 : s_bwdCallableCtx_min_0;
    if(_S1591)
    {
        var _S1594 : f32 = _S1575.value0_83.value0_82.w;
        var _S1595 : f32 = -0.54371398687362671f / _S1594;
        var _S1596 : f32 = _S1594 * _S1594;
        var _S1597 : Tuple_14 = s_apply_min_0(_S1595, max_c_3);
        var _S1598 : s_bwdCallableCtx_min_0;
        _S1598._S106 = _S1595;
        _S1598._S107 = max_c_3;
        max_c_3 = _S1597.value0_16;
        _S1593 = _S1598;
        _S1592 = _S1596;
    }
    else
    {
        _S1593._S106 = 0.0f;
        _S1593._S107 = 0.0f;
        _S1592 = 0.0f;
    }
    var _S1599 : bool = ((_S1581 + _S1590) > 1.0f);
    var _S1600 : s_bwdCallableCtx_min_0;
    if(_S1599)
    {
        var _S1601 : f32 = _S1575.value0_83.value0_82.z + _S1575.value0_83.value0_82.w;
        var _S1602 : f32 = _S1601 * _S1601;
        var _S1603 : s_bwdCallableCtx_min_0;
        _S1603._S106 = 0.23726457357406616f / _S1601;
        _S1603._S107 = max_c_3;
        _S1600 = _S1603;
        max_c_3 = _S1602;
    }
    else
    {
        _S1600._S106 = 0.0f;
        _S1600._S107 = 0.0f;
        max_c_3 = 0.0f;
    }
    var _S1604 : vec4<f32> = vec4<f32>(_S1577[i32(0)], 0.0f, _S1577[i32(2)], _S1577[i32(3)]);
    var _S1605 : vec4<f32>;
    if(_S1599)
    {
        var _S1606 : f32 = 0.0f;
        var _S1607 : f32 = 0.0f;
        s_bwdProp_min_0(_S1600, &(_S1606), &(_S1607), _S1577[i32(1)]);
        var _S1608 : f32 = 0.23726457357406616f * - (_S1606 / max_c_3);
        var _S1609 : vec4<f32> = _S1604 + vec4<f32>(0.0f, 0.0f, _S1608, _S1608);
        max_c_3 = _S1607;
        _S1605 = _S1609;
    }
    else
    {
        max_c_3 = _S1577[i32(1)];
        _S1605 = _S1604;
    }
    if(_S1591)
    {
        var _S1610 : f32 = 0.0f;
        var _S1611 : f32 = 0.0f;
        s_bwdProp_min_0(_S1593, &(_S1610), &(_S1611), max_c_3);
        var _S1612 : vec4<f32> = _S1605 + vec4<f32>(0.0f, 0.0f, 0.0f, -0.54371398687362671f * - (_S1610 / _S1592));
        max_c_3 = _S1611;
        _S1605 = _S1612;
    }
    if(_S1582)
    {
        var _S1613 : f32 = 0.0f;
        var _S1614 : f32 = 0.0f;
        s_bwdProp_min_0(_S1584, &(_S1613), &(_S1614), max_c_3);
        var _S1615 : vec4<f32> = _S1605 + vec4<f32>(0.0f, 0.0f, -0.21902142465114594f * - (_S1613 / _S1583), 0.0f);
        max_c_3 = _S1614;
        _S1605 = _S1615;
    }
    const _S1616 : vec3<f32> = vec3<f32>(0.0f, 0.0f, 0.0f);
    var _S1617 : vec4<f32> = vec4<f32>(0.0f);
    s_bwdProp_ych_to_yrg_0(_S1580, &(_S1617), _S1616);
    (*_S1576) = _S1617 + _S1605 + vec4<f32>(0.0f, max_c_3, 0.0f, 0.0f);
    return;
}

fn s_bwdProp_desaturate_v4_0( _S1618 : s_bwdCallableCtx_desaturate_v4_0,  _S1619 : ptr<function, vec4<f32>>,  _S1620 : ptr<function, vec4<f32>>,  _S1621 : ptr<function, f32>,  _S1622 : vec4<f32>)
{
    var _S1623 : f32 = _S1618.value0_79.value0_78.y;
    var _S1624 : f32 = _S1618.value0_79.value0_78.x;
    var chroma_original_1 : f32 = _S1623 * _S1624;
    var _S1625 : f32 = _S1618.value0_79.value1_20.y;
    var _S1626 : f32 = _S1618.value0_79.value1_20.x;
    var chroma_final_1 : f32 = _S1625 * _S1626;
    var _S1627 : f32 = chroma_original_1 - chroma_final_1;
    var delta_1 : f32 = _S1618.value0_79.value2_15 * _S1627;
    var resaturates_1 : bool = (chroma_original_1 < chroma_final_1);
    var desaturates_1 : bool = (chroma_original_1 > chroma_final_1);
    var _S1628 : bool;
    if((_S1626 > _S1624))
    {
        _S1628 = resaturates_1;
    }
    else
    {
        _S1628 = false;
    }
    var _S1629 : bool;
    var chroma_1 : f32;
    if(_S1628)
    {
        chroma_1 = (chroma_original_1 + chroma_final_1) / 2.0f;
        _S1629 = false;
    }
    else
    {
        if(((_S1618.value0_79.value2_15) > 0.0f))
        {
            _S1629 = desaturates_1;
        }
        else
        {
            _S1629 = false;
        }
        if(_S1629)
        {
            _S1629 = true;
        }
        else
        {
            _S1629 = ((_S1618.value0_79.value2_15) < 0.0f);
        }
        if(_S1629)
        {
            chroma_1 = chroma_final_1 + delta_1;
        }
        else
        {
            chroma_1 = chroma_final_1;
        }
    }
    var _S1630 : bool = (_S1626 > 0.0f);
    var _S1631 : f32;
    var _S1632 : s_bwdCallableCtx_max_1;
    if(_S1630)
    {
        var _S1633 : f32 = _S1626 * _S1626;
        var _S1634 : s_bwdCallableCtx_max_1;
        _S1634._S101 = chroma_1 / _S1626;
        _S1634._S102 = 0.0f;
        _S1632 = _S1634;
        _S1631 = _S1633;
    }
    else
    {
        _S1632._S101 = 0.0f;
        _S1632._S102 = 0.0f;
        _S1631 = 0.0f;
    }
    var _S1635 : vec4<f32> = vec4<f32>(0.0f, 0.0f, _S1622[i32(2)], _S1622[i32(3)]);
    if(_S1630)
    {
        var _S1636 : f32 = 0.0f;
        var _S1637 : f32 = 0.0f;
        s_bwdProp_max_1(_S1632, &(_S1636), &(_S1637), _S1622[i32(1)]);
        var _S1638 : f32 = _S1636 / _S1631;
        var _S1639 : f32 = chroma_1 * - _S1638 + _S1622[i32(0)];
        chroma_1 = _S1626 * _S1638;
        _S1631 = _S1639;
    }
    else
    {
        chroma_1 = 0.0f;
        _S1631 = _S1622[i32(0)];
    }
    var _S1640 : f32;
    var _S1641 : f32;
    if(_S1628)
    {
        var _S1642 : f32 = 0.5f * chroma_1;
        chroma_1 = 0.0f;
        _S1640 = _S1642;
        _S1641 = _S1642;
    }
    else
    {
        if(_S1629)
        {
            _S1640 = chroma_1;
        }
        else
        {
            _S1640 = 0.0f;
        }
        var _S1643 : f32 = chroma_1;
        chroma_1 = _S1640;
        _S1640 = _S1643;
        _S1641 = 0.0f;
    }
    var _S1644 : f32 = _S1618.value0_79.value2_15 * chroma_1;
    var _S1645 : f32 = _S1627 * chroma_1;
    var _S1646 : f32 = - _S1644 + _S1640;
    var _S1647 : f32 = _S1644 + _S1641;
    var _S1648 : vec4<f32> = _S1635 + vec4<f32>(_S1625 * _S1646 + _S1631, _S1626 * _S1646, 0.0f, 0.0f);
    (*_S1619) = vec4<f32>(_S1623 * _S1647, _S1624 * _S1647, 0.0f, 0.0f);
    (*_S1620) = _S1648;
    (*_S1621) = _S1645;
    return;
}

fn s_bwdProp_gamut_mapping_0( _S1649 : s_bwdCallableCtx_gamut_mapping_0,  _S1650 : ptr<function, vec4<f32>>,  _S1651 : ptr<function, vec4<f32>>,  _S1652 : ptr<function, Curve_0>,  _S1653 : ptr<function, f32>,  _S1654 : vec3<f32>)
{
    var _S1655 : f32 = _S1649.value0_71.value0_65.x;
    var _S1656 : f32 = _S1649.value0_71.value2_13.display_0.y;
    var _S1657 : f32 = 1.0578552484512329f * _S1656;
    var _S1658 : f32 = _S1649.value0_71.value2_13.display_0.z;
    var _S1659 : f32 = 1.0578552484512329f * _S1658;
    var _S1660 : Tuple_9 = s_apply_clamp_0(_S1655, _S1657, _S1659);
    var _S1661 : s_bwdCallableCtx_clamp_0;
    _S1661._S65 = _S1655;
    _S1661._S66 = _S1657;
    _S1661._S67 = _S1659;
    var _S1662 : vec4<f32> = vec4<f32>(_S1660.value0_11, _S1649.value0_71.value0_65.y, _S1649.value0_71.value1_18.z, _S1649.value0_71.value1_18.w);
    var _S1663 : Tuple_31 = s_apply_desaturate_v4_0(_S1649.value0_71.value1_18, _S1662, _S1649.value0_71.value3_2);
    var _S1664 : s_paramCtx_s_bwdCallableCtx_desaturate_v4_0;
    _S1664.value0_78 = _S1649.value0_71.value1_18;
    _S1664.value1_20 = _S1662;
    _S1664.value2_15 = _S1649.value0_71.value3_2;
    var _S1665 : s_bwdCallableCtx_desaturate_v4_0 = s_bwdCallableCtx_desaturate_v4_0( _S1664 );
    var _S1666 : Tuple_32 = s_apply_gamut_check_yrg_0(_S1663.value0_77);
    var _S1667 : s_paramCtx_s_bwdCallableCtx_gamut_check_yrg_0;
    _S1667.value0_82 = _S1663.value0_77;
    var _S1668 : s_bwdCallableCtx_gamut_check_yrg_0 = s_bwdCallableCtx_gamut_check_yrg_0( _S1667 );
    var _S1669 : Tuple_34 = s_apply_gamut_check_rgb_0(_S1666.value0_80, _S1649.value0_71.value4_0.export_to_lms_1, _S1649.value0_71.value4_0.export_from_lms_1, _S1656, _S1658);
    var _S1670 : s_paramCtx_s_bwdCallableCtx_gamut_check_rgb_0;
    _S1670.value0_92 = _S1666.value0_80;
    _S1670.value1_21 = _S1649.value0_71.value4_0.export_to_lms_1;
    _S1670.value2_16 = _S1649.value0_71.value4_0.export_from_lms_1;
    _S1670.value3_4 = _S1656;
    _S1670.value4_1 = _S1658;
    var _S1671 : s_bwdCallableCtx_gamut_check_rgb_0 = s_bwdCallableCtx_gamut_check_rgb_0( _S1670 );
    var _S1672 : Tuple_5 = s_apply_mul_0(_S1649.value0_71.value4_0.export_to_lms_1, _S1669.value0_84);
    var _S1673 : s_bwdCallableCtx_mul_0;
    _S1673._S40 = _S1649.value0_71.value4_0.export_to_lms_1;
    _S1673._S41 = _S1669.value0_84;
    var _S1674 : s_bwdCallableCtx_mul_0;
    _S1674._S40 = _S1649.value0_71.value4_0.from_lms_1;
    _S1674._S41 = _S1672.value0_7;
    var _S1675 : Curve_0 = Curve_x24_syn_dzero_0();
    var _S1676 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S1677 : mat3x3<f32> = _S1676;
    var _S1678 : vec3<f32> = vec3<f32>(0.0f);
    var _S1679 : vec3<f32> = _S1678;
    s_bwdProp_mul_0(_S1674, &(_S1677), &(_S1679), _S1654);
    var _S1680 : mat3x3<f32> = _S1676;
    var _S1681 : vec3<f32> = _S1678;
    s_bwdProp_mul_0(_S1673, &(_S1680), &(_S1681), _S1679);
    var _S1682 : vec4<f32> = vec4<f32>(0.0f);
    var _S1683 : vec4<f32> = _S1682;
    var _S1684 : f32 = 0.0f;
    var _S1685 : f32 = 0.0f;
    s_bwdProp_gamut_check_rgb_0(_S1671, &(_S1683), &(_S1684), &(_S1685), _S1681);
    var _S1686 : vec4<f32> = _S1682;
    s_bwdProp_gamut_check_yrg_0(_S1668, &(_S1686), _S1683);
    var _S1687 : vec4<f32> = _S1682;
    var _S1688 : vec4<f32> = _S1682;
    var _S1689 : f32 = 0.0f;
    s_bwdProp_desaturate_v4_0(_S1665, &(_S1687), &(_S1688), &(_S1689), _S1686);
    var _S1690 : f32 = 0.0f;
    var _S1691 : f32 = 0.0f;
    var _S1692 : f32 = 0.0f;
    s_bwdProp_clamp_0(_S1661, &(_S1690), &(_S1691), &(_S1692), _S1688[i32(0)]);
    var _S1693 : vec4<f32> = vec4<f32>(0.0f, _S1684 + 1.0578552484512329f * _S1691, _S1685 + 1.0578552484512329f * _S1692, 0.0f);
    var _S1694 : vec4<f32> = vec4<f32>(_S1690, _S1688[i32(1)], 0.0f, 0.0f);
    var _S1695 : Curve_0 = _S1675;
    _S1695.display_0 = _S1693;
    var _S1696 : Curve_0 = Curve_x24_syn_dadd_0(_S1675, _S1695);
    var _S1697 : vec4<f32> = _S1687 + vec4<f32>(0.0f, 0.0f, _S1688[i32(2)], _S1688[i32(3)]);
    (*_S1650) = _S1694;
    (*_S1651) = _S1697;
    (*_S1652) = _S1696;
    (*_S1653) = _S1689;
    return;
}

fn s_bwdProp_norm_tone_mapping_0( _S1698 : s_bwdCallableCtx_norm_tone_mapping_0,  _S1699 : ptr<function, vec3<f32>>,  _S1700 : ptr<function, Curve_0>,  _S1701 : vec3<f32>)
{
    var _S1702 : Tuple_16 = s_apply_pixel_norm_0(_S1698.value0_75.value0_66, _S1698.value0_75.value2_14, _S1698.value0_75.value3_3.luminance_1);
    var _S1703 : s_paramCtx_s_bwdCallableCtx_pixel_norm_0;
    _S1703.value0_36 = _S1698.value0_75.value0_66;
    _S1703.value1_8 = _S1698.value0_75.value2_14;
    _S1703.value2_8 = _S1698.value0_75.value3_3.luminance_1;
    var _S1704 : s_bwdCallableCtx_pixel_norm_0 = s_bwdCallableCtx_pixel_norm_0( _S1703 );
    var _S1705 : f32 = _S1698.value0_75.value1_19.display_0.w;
    var _S1706 : f32 = _S1698.value0_75.value1_19.extra_0.x;
    var _S1707 : Tuple_9 = s_apply_clamp_0(_S1702.value0_46, _S1705, _S1706);
    var _S1708 : s_bwdCallableCtx_clamp_0;
    _S1708._S65 = _S1702.value0_46;
    _S1708._S66 = _S1705;
    _S1708._S67 = _S1706;
    var _S1709 : vec3<f32> = vec3<f32>(_S1707.value0_11);
    var ratios_2 : vec3<f32> = _S1698.value0_75.value0_66 / _S1709;
    var _S1710 : f32 = _S1707.value0_11 * _S1707.value0_11;
    var _S1711 : Tuple_18 = s_apply_log_v2_0(_S1707.value0_11, _S1698.value0_75.value1_19, false);
    var _S1712 : s_paramCtx_s_bwdCallableCtx_log_v2_0;
    _S1712.value0_38 = _S1707.value0_11;
    _S1712.value1_9 = _S1698.value0_75.value1_19;
    _S1712.value2_9 = false;
    var _S1713 : s_bwdCallableCtx_log_v2_0 = s_bwdCallableCtx_log_v2_0( _S1712 );
    var _S1714 : Tuple_21 = s_apply_spline_0(_S1711.value0_48, _S1698.value0_75.value1_19, _S1698.value0_75.value3_3.toe_1, _S1698.value0_75.value3_3.shoulder_1);
    var _S1715 : s_paramCtx_s_bwdCallableCtx_spline_0;
    _S1715.value0_40 = _S1711.value0_48;
    _S1715.value1_11 = _S1698.value0_75.value1_19;
    _S1715.value2_10 = _S1698.value0_75.value3_3.toe_1;
    _S1715.value3_1 = _S1698.value0_75.value3_3.shoulder_1;
    var _S1716 : s_bwdCallableCtx_spline_0 = s_bwdCallableCtx_spline_0( _S1715 );
    var _S1717 : f32 = _S1698.value0_75.value1_19.display_0.y;
    var _S1718 : f32 = _S1698.value0_75.value1_19.display_0.z;
    var _S1719 : Tuple_9 = s_apply_clamp_0(_S1714.value0_51, _S1717, _S1718);
    var _S1720 : s_bwdCallableCtx_clamp_0;
    _S1720._S65 = _S1714.value0_51;
    _S1720._S66 = _S1717;
    _S1720._S67 = _S1718;
    var _S1721 : f32 = _S1698.value0_75.value1_19.tone_0.y;
    var _S1722 : Tuple_8 = s_apply_pow0_0(_S1719.value0_11, _S1721);
    var _S1723 : s_bwdCallableCtx_pow0_0;
    _S1723._S60 = _S1719.value0_11;
    _S1723._S61 = _S1721;
    var _S1724 : Curve_0 = Curve_x24_syn_dzero_0();
    var _S1725 : vec3<f32> = ratios_2 * _S1701;
    var _S1726 : f32 = _S1725[i32(0)] + _S1725[i32(1)] + _S1725[i32(2)];
    var s_diff_ratios_T_1 : vec3<f32> = vec3<f32>(_S1722.value0_10) * _S1701;
    var _S1727 : f32 = 0.0f;
    var _S1728 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S1723, &(_S1727), &(_S1728), _S1726);
    var _S1729 : vec4<f32> = vec4<f32>(0.0f, _S1728, 0.0f, 0.0f);
    var _S1730 : f32 = 0.0f;
    var _S1731 : f32 = 0.0f;
    var _S1732 : f32 = 0.0f;
    s_bwdProp_clamp_0(_S1720, &(_S1730), &(_S1731), &(_S1732), _S1727);
    var _S1733 : f32 = 0.0f;
    var _S1734 : vec4<f32> = vec4<f32>(0.0f);
    var _S1735 : Curve_0;
    _S1735.toe_0 = _S1734;
    _S1735.shoulder_0 = _S1734;
    _S1735.line_0 = _S1734;
    _S1735.nodes_0 = _S1734;
    _S1735.tone_0 = _S1734;
    _S1735.display_0 = _S1734;
    _S1735.extra_0 = _S1734;
    s_bwdProp_spline_0(_S1716, &(_S1733), &(_S1735), _S1730);
    var _S1736 : f32 = 0.0f;
    var _S1737 : Curve_0;
    _S1737.toe_0 = _S1734;
    _S1737.shoulder_0 = _S1734;
    _S1737.line_0 = _S1734;
    _S1737.nodes_0 = _S1734;
    _S1737.tone_0 = _S1734;
    _S1737.display_0 = _S1734;
    _S1737.extra_0 = _S1734;
    s_bwdProp_log_v2_0(_S1713, &(_S1736), &(_S1737), _S1733);
    var _S1738 : vec3<f32> = s_diff_ratios_T_1 / vec3<f32>(_S1710);
    var _S1739 : vec3<f32> = _S1698.value0_75.value0_66 * (vec3<f32>(0) - _S1738);
    var _S1740 : vec3<f32> = _S1709 * _S1738;
    var _S1741 : f32 = _S1736 + (_S1739[i32(0)] + _S1739[i32(1)] + _S1739[i32(2)]);
    var _S1742 : f32 = 0.0f;
    var _S1743 : f32 = 0.0f;
    var _S1744 : f32 = 0.0f;
    s_bwdProp_clamp_0(_S1708, &(_S1742), &(_S1743), &(_S1744), _S1741);
    var _S1745 : vec4<f32> = vec4<f32>(_S1744, 0.0f, 0.0f, 0.0f);
    var _S1746 : vec4<f32> = vec4<f32>(0.0f, _S1731, _S1732, _S1743);
    var _S1747 : vec3<f32> = vec3<f32>(0.0f);
    s_bwdProp_pixel_norm_0(_S1704, &(_S1747), _S1742);
    var _S1748 : vec3<f32> = _S1740 + _S1747;
    var _S1749 : Curve_0 = Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(_S1735, _S1737), _S1724);
    var _S1750 : Curve_0 = _S1724;
    _S1750.tone_0 = _S1729;
    _S1750.extra_0 = _S1745;
    _S1750.display_0 = _S1746;
    var _S1751 : Curve_0 = Curve_x24_syn_dadd_0(_S1749, _S1750);
    (*_S1699) = _S1748;
    (*_S1700) = _S1751;
    return;
}

struct s_paramCtx_s_bwdCallableCtx_channel_tone_mapping_0
{
     value0_114 : f32,
     value1_27 : Curve_0,
     value2_21 : Modes_0,
};

struct s_bwdCallableCtx_channel_tone_mapping_0
{
     value0_115 : s_paramCtx_s_bwdCallableCtx_channel_tone_mapping_0,
};

fn s_bwdProp_channel_tone_mapping_0( _S1752 : s_bwdCallableCtx_channel_tone_mapping_0,  _S1753 : ptr<function, f32>,  _S1754 : ptr<function, Curve_0>,  _S1755 : f32)
{
    var _S1756 : Tuple_13 = s_apply_max_1(_S1752.value0_115.value0_114, 1.00000000317107685e-30f);
    var _S1757 : s_bwdCallableCtx_max_1;
    _S1757._S101 = _S1752.value0_115.value0_114;
    _S1757._S102 = 1.00000000317107685e-30f;
    var _S1758 : Tuple_18 = s_apply_log_v2_0(_S1756.value0_15, _S1752.value0_115.value1_27, false);
    var _S1759 : s_paramCtx_s_bwdCallableCtx_log_v2_0;
    _S1759.value0_38 = _S1756.value0_15;
    _S1759.value1_9 = _S1752.value0_115.value1_27;
    _S1759.value2_9 = false;
    var _S1760 : s_bwdCallableCtx_log_v2_0 = s_bwdCallableCtx_log_v2_0( _S1759 );
    var _S1761 : Tuple_21 = s_apply_spline_0(_S1758.value0_48, _S1752.value0_115.value1_27, _S1752.value0_115.value2_21.toe_1, _S1752.value0_115.value2_21.shoulder_1);
    var _S1762 : s_paramCtx_s_bwdCallableCtx_spline_0;
    _S1762.value0_40 = _S1758.value0_48;
    _S1762.value1_11 = _S1752.value0_115.value1_27;
    _S1762.value2_10 = _S1752.value0_115.value2_21.toe_1;
    _S1762.value3_1 = _S1752.value0_115.value2_21.shoulder_1;
    var _S1763 : s_bwdCallableCtx_spline_0 = s_bwdCallableCtx_spline_0( _S1762 );
    var _S1764 : f32 = _S1752.value0_115.value1_27.display_0.z;
    var _S1765 : Tuple_9 = s_apply_clamp_0(_S1761.value0_51, 0.0f, _S1764);
    var _S1766 : s_bwdCallableCtx_clamp_0;
    _S1766._S65 = _S1761.value0_51;
    _S1766._S66 = 0.0f;
    _S1766._S67 = _S1764;
    var _S1767 : f32 = _S1752.value0_115.value1_27.tone_0.y;
    var _S1768 : s_bwdCallableCtx_pow_dt_0;
    _S1768._S79 = _S1765.value0_11;
    _S1768._S80 = _S1767;
    var _S1769 : Curve_0 = Curve_x24_syn_dzero_0();
    var _S1770 : f32 = 0.0f;
    var _S1771 : f32 = 0.0f;
    s_bwdProp_pow_dt_0(_S1768, &(_S1770), &(_S1771), _S1755);
    var _S1772 : vec4<f32> = vec4<f32>(0.0f, _S1771, 0.0f, 0.0f);
    var _S1773 : f32 = 0.0f;
    var _S1774 : f32 = 0.0f;
    var _S1775 : f32 = 0.0f;
    s_bwdProp_clamp_0(_S1766, &(_S1773), &(_S1774), &(_S1775), _S1770);
    var _S1776 : vec4<f32> = vec4<f32>(0.0f, 0.0f, _S1775, 0.0f);
    var _S1777 : f32 = 0.0f;
    var _S1778 : vec4<f32> = vec4<f32>(0.0f);
    var _S1779 : Curve_0;
    _S1779.toe_0 = _S1778;
    _S1779.shoulder_0 = _S1778;
    _S1779.line_0 = _S1778;
    _S1779.nodes_0 = _S1778;
    _S1779.tone_0 = _S1778;
    _S1779.display_0 = _S1778;
    _S1779.extra_0 = _S1778;
    s_bwdProp_spline_0(_S1763, &(_S1777), &(_S1779), _S1773);
    var _S1780 : f32 = 0.0f;
    var _S1781 : Curve_0;
    _S1781.toe_0 = _S1778;
    _S1781.shoulder_0 = _S1778;
    _S1781.line_0 = _S1778;
    _S1781.nodes_0 = _S1778;
    _S1781.tone_0 = _S1778;
    _S1781.display_0 = _S1778;
    _S1781.extra_0 = _S1778;
    s_bwdProp_log_v2_0(_S1760, &(_S1780), &(_S1781), _S1777);
    var _S1782 : f32 = 0.0f;
    var _S1783 : f32 = 0.0f;
    s_bwdProp_max_1(_S1757, &(_S1782), &(_S1783), _S1780);
    var _S1784 : Curve_0 = Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(_S1779, _S1781), _S1769);
    var _S1785 : Curve_0 = _S1769;
    _S1785.tone_0 = _S1772;
    _S1785.display_0 = _S1776;
    var _S1786 : Curve_0 = Curve_x24_syn_dadd_0(_S1784, _S1785);
    (*_S1753) = _S1782;
    (*_S1754) = _S1786;
    return;
}

fn s_bwdProp_rgb_tone_mapping_0( _S1787 : s_bwdCallableCtx_rgb_tone_mapping_0,  _S1788 : ptr<function, vec3<f32>>,  _S1789 : ptr<function, Curve_0>,  _S1790 : vec3<f32>)
{
    var _S1791 : s_paramCtx_s_bwdCallableCtx_channel_tone_mapping_0;
    _S1791.value0_114 = _S1787.value0_72.value0_63.x;
    _S1791.value1_27 = _S1787.value0_72.value1_16;
    _S1791.value2_21 = _S1787.value0_72.value2_12;
    var _S1792 : s_bwdCallableCtx_channel_tone_mapping_0 = s_bwdCallableCtx_channel_tone_mapping_0( _S1791 );
    var _S1793 : s_paramCtx_s_bwdCallableCtx_channel_tone_mapping_0;
    _S1793.value0_114 = _S1787.value0_72.value0_63.y;
    _S1793.value1_27 = _S1787.value0_72.value1_16;
    _S1793.value2_21 = _S1787.value0_72.value2_12;
    var _S1794 : s_bwdCallableCtx_channel_tone_mapping_0 = s_bwdCallableCtx_channel_tone_mapping_0( _S1793 );
    var _S1795 : s_paramCtx_s_bwdCallableCtx_channel_tone_mapping_0;
    _S1795.value0_114 = _S1787.value0_72.value0_63.z;
    _S1795.value1_27 = _S1787.value0_72.value1_16;
    _S1795.value2_21 = _S1787.value0_72.value2_12;
    var _S1796 : s_bwdCallableCtx_channel_tone_mapping_0 = s_bwdCallableCtx_channel_tone_mapping_0( _S1795 );
    var _S1797 : Curve_0 = Curve_x24_syn_dzero_0();
    var _S1798 : f32 = 0.0f;
    var _S1799 : vec4<f32> = vec4<f32>(0.0f);
    var _S1800 : Curve_0;
    _S1800.toe_0 = _S1799;
    _S1800.shoulder_0 = _S1799;
    _S1800.line_0 = _S1799;
    _S1800.nodes_0 = _S1799;
    _S1800.tone_0 = _S1799;
    _S1800.display_0 = _S1799;
    _S1800.extra_0 = _S1799;
    s_bwdProp_channel_tone_mapping_0(_S1796, &(_S1798), &(_S1800), _S1790[i32(2)]);
    var _S1801 : f32 = 0.0f;
    var _S1802 : Curve_0;
    _S1802.toe_0 = _S1799;
    _S1802.shoulder_0 = _S1799;
    _S1802.line_0 = _S1799;
    _S1802.nodes_0 = _S1799;
    _S1802.tone_0 = _S1799;
    _S1802.display_0 = _S1799;
    _S1802.extra_0 = _S1799;
    s_bwdProp_channel_tone_mapping_0(_S1794, &(_S1801), &(_S1802), _S1790[i32(1)]);
    var _S1803 : f32 = 0.0f;
    var _S1804 : Curve_0;
    _S1804.toe_0 = _S1799;
    _S1804.shoulder_0 = _S1799;
    _S1804.line_0 = _S1799;
    _S1804.nodes_0 = _S1799;
    _S1804.tone_0 = _S1799;
    _S1804.display_0 = _S1799;
    _S1804.extra_0 = _S1799;
    s_bwdProp_channel_tone_mapping_0(_S1792, &(_S1803), &(_S1804), _S1790[i32(0)]);
    var _S1805 : Curve_0 = Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(_S1800, _S1802), _S1804), _S1797);
    (*_S1788) = vec3<f32>(_S1803, _S1801, _S1798);
    (*_S1789) = _S1805;
    return;
}

fn s_bwdProp_filmic_v6_0( _S1806 : s_bwdCallableCtx_filmic_v6_0,  _S1807 : ptr<function, vec3<f32>>,  _S1808 : ptr<function, Curve_0>,  _S1809 : vec3<f32>)
{
    var _S1810 : vec3<f32> = vec3<f32>(0.0f);
    var _S1811 : vec4<f32> = vec4<f32>(0.0f);
    var _S1812 : Curve_0 = Curve_0( _S1811, _S1811, _S1811, _S1811, _S1811, _S1811, _S1811 );
    var _S1813 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S1814 : Modes_0 = Modes_0( u32(0), u32(0), u32(0), u32(0), false, _S1810, _S1813, _S1813, _S1813, _S1813 );
    var _S1815 : s_paramCtx_s_bwdCallableCtx_rgb_tone_mapping_0 = s_paramCtx_s_bwdCallableCtx_rgb_tone_mapping_0( _S1810, _S1812, _S1814 );
    var _S1816 : s_paramCtx_s_bwdCallableCtx_rgb_to_ych_0 = s_paramCtx_s_bwdCallableCtx_rgb_to_ych_0( _S1810, _S1813 );
    var _S1817 : s_paramCtx_s_bwdCallableCtx_gamut_mapping_0 = s_paramCtx_s_bwdCallableCtx_gamut_mapping_0( _S1811, _S1811, _S1812, 0.0f, _S1814 );
    var _S1818 : s_paramCtx_s_bwdCallableCtx_norm_tone_mapping_0 = s_paramCtx_s_bwdCallableCtx_norm_tone_mapping_0( _S1810, _S1812, u32(0), _S1814 );
    var _S1819 : Tuple_25 = s_apply_rgb_to_ych_0(_S1806.value0_31.value0_25, _S1806.value0_31.value2_3.to_lms_1);
    var _S1820 : s_paramCtx_s_bwdCallableCtx_rgb_to_ych_0;
    _S1820.value0_64 = _S1806.value0_31.value0_25;
    _S1820.value1_17 = _S1806.value0_31.value2_3.to_lms_1;
    var _S1821 : s_bwdCallableCtx_rgb_to_ych_0 = s_bwdCallableCtx_rgb_to_ych_0( _S1820 );
    var _S1822 : bool = (_S1806.value0_31.value2_3.norm_0) == u32(0);
    var _S1823 : vec4<f32>;
    var _S1824 : s_bwdCallableCtx_gamut_mapping_0;
    var _S1825 : s_bwdCallableCtx_min_0;
    var _S1826 : s_bwdCallableCtx_rgb_to_ych_0;
    var _S1827 : s_bwdCallableCtx_rgb_tone_mapping_0;
    if(_S1822)
    {
        var _S1828 : Tuple_28 = s_apply_rgb_tone_mapping_0(_S1806.value0_31.value0_25, _S1806.value0_31.value1_3, _S1806.value0_31.value2_3);
        var _S1829 : s_paramCtx_s_bwdCallableCtx_rgb_tone_mapping_0;
        _S1829.value0_63 = _S1806.value0_31.value0_25;
        _S1829.value1_16 = _S1806.value0_31.value1_3;
        _S1829.value2_12 = _S1806.value0_31.value2_3;
        var _S1830 : Tuple_25 = s_apply_rgb_to_ych_0(_S1828.value0_73, _S1806.value0_31.value2_3.to_lms_1);
        var _S1831 : s_paramCtx_s_bwdCallableCtx_rgb_to_ych_0;
        _S1831.value0_64 = _S1828.value0_73;
        _S1831.value1_17 = _S1806.value0_31.value2_3.to_lms_1;
        var _S1832 : f32 = _S1819.value0_67.y;
        var _S1833 : f32 = _S1830.value0_67.y;
        var _S1834 : Tuple_14 = s_apply_min_0(_S1832, _S1833);
        var _S1835 : s_bwdCallableCtx_min_0;
        _S1835._S106 = _S1832;
        _S1835._S107 = _S1833;
        _S1823 = _S1830.value0_67;
        _S1823[i32(1)] = _S1834.value0_16;
        var _S1836 : f32 = _S1806.value0_31.value1_3.tone_0.z;
        var _S1837 : s_paramCtx_s_bwdCallableCtx_gamut_mapping_0;
        _S1837.value0_65 = _S1823;
        _S1837.value1_18 = _S1819.value0_67;
        _S1837.value2_13 = _S1806.value0_31.value1_3;
        _S1837.value3_2 = _S1836;
        _S1837.value4_0 = _S1806.value0_31.value2_3;
        _S1824.value0_71 = _S1837;
        _S1825 = _S1835;
        _S1826.value0_70 = _S1831;
        _S1827.value0_72 = _S1829;
    }
    else
    {
        _S1824.value0_71 = _S1817;
        _S1825._S106 = 0.0f;
        _S1825._S107 = 0.0f;
        _S1826.value0_70 = _S1816;
        _S1827.value0_72 = _S1815;
    }
    var _S1838 : bool = !_S1822;
    var _S1839 : s_bwdCallableCtx_gamut_mapping_0;
    var _S1840 : s_bwdCallableCtx_rgb_to_ych_0;
    var _S1841 : s_bwdCallableCtx_norm_tone_mapping_0;
    if(_S1838)
    {
        var _S1842 : Tuple_30 = s_apply_norm_tone_mapping_0(_S1806.value0_31.value0_25, _S1806.value0_31.value1_3, _S1806.value0_31.value2_3.norm_0, _S1806.value0_31.value2_3);
        var _S1843 : s_paramCtx_s_bwdCallableCtx_norm_tone_mapping_0;
        _S1843.value0_66 = _S1806.value0_31.value0_25;
        _S1843.value1_19 = _S1806.value0_31.value1_3;
        _S1843.value2_14 = _S1806.value0_31.value2_3.norm_0;
        _S1843.value3_3 = _S1806.value0_31.value2_3;
        var _S1844 : Tuple_25 = s_apply_rgb_to_ych_0(_S1842.value0_76, _S1806.value0_31.value2_3.to_lms_1);
        var _S1845 : s_paramCtx_s_bwdCallableCtx_rgb_to_ych_0;
        _S1845.value0_64 = _S1842.value0_76;
        _S1845.value1_17 = _S1806.value0_31.value2_3.to_lms_1;
        var _S1846 : f32 = _S1806.value0_31.value1_3.tone_0.z;
        var _S1847 : s_paramCtx_s_bwdCallableCtx_gamut_mapping_0;
        _S1847.value0_65 = _S1844.value0_67;
        _S1847.value1_18 = _S1819.value0_67;
        _S1847.value2_13 = _S1806.value0_31.value1_3;
        _S1847.value3_2 = _S1846;
        _S1847.value4_0 = _S1806.value0_31.value2_3;
        _S1839.value0_71 = _S1847;
        _S1840.value0_70 = _S1845;
        _S1841.value0_75 = _S1843;
    }
    else
    {
        _S1839.value0_71 = _S1817;
        _S1840.value0_70 = _S1816;
        _S1841.value0_75 = _S1818;
    }
    var _S1848 : Curve_0 = Curve_x24_syn_dzero_0();
    var _S1849 : vec3<f32>;
    var _S1850 : vec3<f32>;
    var _S1851 : Curve_0;
    if(_S1838)
    {
        var _S1852 : vec4<f32> = _S1811;
        var _S1853 : vec4<f32> = _S1811;
        var _S1854 : Curve_0;
        _S1854.toe_0 = _S1811;
        _S1854.shoulder_0 = _S1811;
        _S1854.line_0 = _S1811;
        _S1854.nodes_0 = _S1811;
        _S1854.tone_0 = _S1811;
        _S1854.display_0 = _S1811;
        _S1854.extra_0 = _S1811;
        var _S1855 : f32 = 0.0f;
        s_bwdProp_gamut_mapping_0(_S1839, &(_S1852), &(_S1853), &(_S1854), &(_S1855), _S1809);
        var _S1856 : vec4<f32> = vec4<f32>(0.0f, 0.0f, _S1855, 0.0f);
        var _S1857 : vec3<f32> = _S1810;
        s_bwdProp_rgb_to_ych_0(_S1840, &(_S1857), _S1852);
        var _S1858 : vec3<f32> = _S1810;
        var _S1859 : Curve_0;
        _S1859.toe_0 = _S1811;
        _S1859.shoulder_0 = _S1811;
        _S1859.line_0 = _S1811;
        _S1859.nodes_0 = _S1811;
        _S1859.tone_0 = _S1811;
        _S1859.display_0 = _S1811;
        _S1859.extra_0 = _S1811;
        s_bwdProp_norm_tone_mapping_0(_S1841, &(_S1858), &(_S1859), _S1857);
        var _S1860 : Curve_0 = Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(_S1854, _S1859), _S1848);
        var _S1861 : Curve_0 = _S1848;
        _S1861.tone_0 = _S1856;
        var _S1862 : Curve_0 = Curve_x24_syn_dadd_0(_S1860, _S1861);
        _S1849 = _S1810;
        _S1851 = _S1862;
        _S1823 = _S1853;
        _S1850 = _S1858;
    }
    else
    {
        _S1849 = _S1809;
        _S1851 = _S1848;
        _S1823 = _S1811;
        _S1850 = _S1810;
    }
    if(_S1822)
    {
        var _S1863 : vec4<f32> = _S1811;
        var _S1864 : vec4<f32> = _S1811;
        var _S1865 : Curve_0;
        _S1865.toe_0 = _S1811;
        _S1865.shoulder_0 = _S1811;
        _S1865.line_0 = _S1811;
        _S1865.nodes_0 = _S1811;
        _S1865.tone_0 = _S1811;
        _S1865.display_0 = _S1811;
        _S1865.extra_0 = _S1811;
        var _S1866 : f32 = 0.0f;
        s_bwdProp_gamut_mapping_0(_S1824, &(_S1863), &(_S1864), &(_S1865), &(_S1866), _S1849);
        var _S1867 : vec4<f32> = vec4<f32>(0.0f, 0.0f, _S1866, 0.0f);
        var _S1868 : vec4<f32> = _S1863;
        _S1868[i32(1)] = 0.0f;
        var _S1869 : f32 = 0.0f;
        var _S1870 : f32 = 0.0f;
        s_bwdProp_min_0(_S1825, &(_S1869), &(_S1870), _S1863[i32(1)]);
        var _S1871 : vec4<f32> = _S1868 + vec4<f32>(0.0f, _S1870, 0.0f, 0.0f);
        var _S1872 : vec3<f32> = _S1810;
        s_bwdProp_rgb_to_ych_0(_S1826, &(_S1872), _S1871);
        var _S1873 : vec3<f32> = _S1810;
        var _S1874 : Curve_0;
        _S1874.toe_0 = _S1811;
        _S1874.shoulder_0 = _S1811;
        _S1874.line_0 = _S1811;
        _S1874.nodes_0 = _S1811;
        _S1874.tone_0 = _S1811;
        _S1874.display_0 = _S1811;
        _S1874.extra_0 = _S1811;
        s_bwdProp_rgb_tone_mapping_0(_S1827, &(_S1873), &(_S1874), _S1872);
        var _S1875 : Curve_0 = Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(_S1865, _S1874), _S1851);
        var _S1876 : Curve_0 = _S1848;
        _S1876.tone_0 = _S1867;
        var _S1877 : Curve_0 = Curve_x24_syn_dadd_0(_S1875, _S1876);
        var _S1878 : vec3<f32> = _S1873 + _S1850;
        _S1823 = _S1864 + _S1823 + vec4<f32>(0.0f, _S1869, 0.0f, 0.0f);
        _S1849 = _S1878;
        _S1851 = _S1877;
    }
    else
    {
        _S1849 = _S1850;
    }
    var _S1879 : vec3<f32> = _S1810;
    s_bwdProp_rgb_to_ych_0(_S1821, &(_S1879), _S1823);
    (*_S1807) = _S1879 + _S1849;
    (*_S1808) = _S1851;
    return;
}

fn s_bwdProp_filmic_v7_0( _S1880 : s_bwdCallableCtx_filmic_v7_0,  _S1881 : ptr<function, vec3<f32>>,  _S1882 : ptr<function, Curve_0>,  _S1883 : vec3<f32>)
{
    var _S1884 : Tuple_28 = s_apply_rgb_tone_mapping_0(_S1880.value0_30.value0_24, _S1880.value0_30.value1_2, _S1880.value0_30.value2_2);
    var _S1885 : s_paramCtx_s_bwdCallableCtx_rgb_tone_mapping_0;
    _S1885.value0_63 = _S1880.value0_30.value0_24;
    _S1885.value1_16 = _S1880.value0_30.value1_2;
    _S1885.value2_12 = _S1880.value0_30.value2_2;
    var _S1886 : s_bwdCallableCtx_rgb_tone_mapping_0 = s_bwdCallableCtx_rgb_tone_mapping_0( _S1885 );
    var _S1887 : Tuple_30 = s_apply_norm_tone_mapping_0(_S1880.value0_30.value0_24, _S1880.value0_30.value1_2, u32(1), _S1880.value0_30.value2_2);
    var _S1888 : s_paramCtx_s_bwdCallableCtx_norm_tone_mapping_0;
    _S1888.value0_66 = _S1880.value0_30.value0_24;
    _S1888.value1_19 = _S1880.value0_30.value1_2;
    _S1888.value2_14 = u32(1);
    _S1888.value3_3 = _S1880.value0_30.value2_2;
    var _S1889 : s_bwdCallableCtx_norm_tone_mapping_0 = s_bwdCallableCtx_norm_tone_mapping_0( _S1888 );
    var s_0 : f32 = _S1880.value0_30.value1_2.tone_0.z;
    var _S1890 : vec3<f32> = vec3<f32>((0.5f - s_0));
    var _S1891 : vec3<f32> = vec3<f32>((0.5f + s_0));
    var mixed_0 : vec3<f32> = _S1890 * _S1884.value0_73 + _S1891 * _S1887.value0_76;
    var _S1892 : Tuple_25 = s_apply_rgb_to_ych_0(_S1880.value0_30.value0_24, _S1880.value0_30.value2_2.to_lms_1);
    var _S1893 : s_paramCtx_s_bwdCallableCtx_rgb_to_ych_0;
    _S1893.value0_64 = _S1880.value0_30.value0_24;
    _S1893.value1_17 = _S1880.value0_30.value2_2.to_lms_1;
    var _S1894 : s_bwdCallableCtx_rgb_to_ych_0 = s_bwdCallableCtx_rgb_to_ych_0( _S1893 );
    var _S1895 : Tuple_25 = s_apply_rgb_to_ych_0(mixed_0, _S1880.value0_30.value2_2.to_lms_1);
    var _S1896 : s_paramCtx_s_bwdCallableCtx_rgb_to_ych_0;
    _S1896.value0_64 = mixed_0;
    _S1896.value1_17 = _S1880.value0_30.value2_2.to_lms_1;
    var _S1897 : s_bwdCallableCtx_rgb_to_ych_0 = s_bwdCallableCtx_rgb_to_ych_0( _S1896 );
    var _S1898 : f32 = _S1892.value0_67.y;
    var _S1899 : f32 = _S1895.value0_67.y;
    var _S1900 : Tuple_14 = s_apply_min_0(_S1898, _S1899);
    var _S1901 : s_bwdCallableCtx_min_0;
    _S1901._S106 = _S1898;
    _S1901._S107 = _S1899;
    var _S1902 : vec4<f32> = _S1895.value0_67;
    _S1902[i32(1)] = _S1900.value0_16;
    var _S1903 : s_paramCtx_s_bwdCallableCtx_gamut_mapping_0;
    _S1903.value0_65 = _S1902;
    _S1903.value1_18 = _S1892.value0_67;
    _S1903.value2_13 = _S1880.value0_30.value1_2;
    _S1903.value3_2 = 0.0f;
    _S1903.value4_0 = _S1880.value0_30.value2_2;
    var _S1904 : s_bwdCallableCtx_gamut_mapping_0 = s_bwdCallableCtx_gamut_mapping_0( _S1903 );
    var _S1905 : Curve_0 = Curve_x24_syn_dzero_0();
    var _S1906 : vec4<f32> = vec4<f32>(0.0f);
    var _S1907 : vec4<f32> = _S1906;
    var _S1908 : vec4<f32> = _S1906;
    var _S1909 : Curve_0;
    _S1909.toe_0 = _S1906;
    _S1909.shoulder_0 = _S1906;
    _S1909.line_0 = _S1906;
    _S1909.nodes_0 = _S1906;
    _S1909.tone_0 = _S1906;
    _S1909.display_0 = _S1906;
    _S1909.extra_0 = _S1906;
    var _S1910 : f32 = 0.0f;
    s_bwdProp_gamut_mapping_0(_S1904, &(_S1907), &(_S1908), &(_S1909), &(_S1910), _S1883);
    _S1902 = _S1907;
    _S1902[i32(1)] = 0.0f;
    var _S1911 : f32 = 0.0f;
    var _S1912 : f32 = 0.0f;
    s_bwdProp_min_0(_S1901, &(_S1911), &(_S1912), _S1907[i32(1)]);
    var _S1913 : vec4<f32> = _S1902 + vec4<f32>(0.0f, _S1912, 0.0f, 0.0f);
    var _S1914 : vec3<f32> = vec3<f32>(0.0f);
    var _S1915 : vec3<f32> = _S1914;
    s_bwdProp_rgb_to_ych_0(_S1897, &(_S1915), _S1913);
    var _S1916 : vec4<f32> = _S1908 + vec4<f32>(0.0f, _S1911, 0.0f, 0.0f);
    var _S1917 : vec3<f32> = _S1914;
    s_bwdProp_rgb_to_ych_0(_S1894, &(_S1917), _S1916);
    var _S1918 : vec3<f32> = _S1891 * _S1915;
    var _S1919 : vec3<f32> = _S1887.value0_76 * _S1915;
    var _S1920 : vec3<f32> = _S1890 * _S1915;
    var _S1921 : vec3<f32> = _S1884.value0_73 * _S1915;
    var _S1922 : vec4<f32> = vec4<f32>(0.0f, 0.0f, _S1919[i32(0)] + _S1919[i32(1)] + _S1919[i32(2)] + - (_S1921[i32(0)] + _S1921[i32(1)] + _S1921[i32(2)]), 0.0f);
    var _S1923 : vec3<f32> = _S1914;
    var _S1924 : Curve_0;
    _S1924.toe_0 = _S1906;
    _S1924.shoulder_0 = _S1906;
    _S1924.line_0 = _S1906;
    _S1924.nodes_0 = _S1906;
    _S1924.tone_0 = _S1906;
    _S1924.display_0 = _S1906;
    _S1924.extra_0 = _S1906;
    s_bwdProp_norm_tone_mapping_0(_S1889, &(_S1923), &(_S1924), _S1918);
    var _S1925 : vec3<f32> = _S1914;
    var _S1926 : Curve_0;
    _S1926.toe_0 = _S1906;
    _S1926.shoulder_0 = _S1906;
    _S1926.line_0 = _S1906;
    _S1926.nodes_0 = _S1906;
    _S1926.tone_0 = _S1906;
    _S1926.display_0 = _S1906;
    _S1926.extra_0 = _S1906;
    s_bwdProp_rgb_tone_mapping_0(_S1886, &(_S1925), &(_S1926), _S1920);
    var _S1927 : Curve_0 = Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(Curve_x24_syn_dadd_0(_S1909, _S1924), _S1926), _S1905);
    var _S1928 : Curve_0 = _S1905;
    _S1928.tone_0 = _S1922;
    var _S1929 : Curve_0 = Curve_x24_syn_dadd_0(_S1927, _S1928);
    (*_S1881) = _S1917 + _S1923 + _S1925;
    (*_S1882) = _S1929;
    return;
}

fn s_bwdProp_filmic_0( _S1930 : s_bwdCallableCtx_filmic_0,  _S1931 : ptr<function, Curve_0>,  _S1932 : vec3<f32>)
{
    var _S1933 : vec3<f32> = vec3<f32>(0.0f);
    var _S1934 : vec4<f32> = vec4<f32>(0.0f);
    var _S1935 : Curve_0 = Curve_0( _S1934, _S1934, _S1934, _S1934, _S1934, _S1934, _S1934 );
    var _S1936 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S1937 : Modes_0 = Modes_0( u32(0), u32(0), u32(0), u32(0), false, _S1933, _S1936, _S1936, _S1936, _S1936 );
    var _S1938 : s_paramCtx_s_bwdCallableCtx_filmic_v7_0 = s_paramCtx_s_bwdCallableCtx_filmic_v7_0( _S1933, _S1935, _S1937 );
    var _S1939 : s_paramCtx_s_bwdCallableCtx_filmic_v6_0 = s_paramCtx_s_bwdCallableCtx_filmic_v6_0( _S1933, _S1935, _S1937 );
    var _S1940 : s_paramCtx_s_bwdCallableCtx_split_v1_0 = s_paramCtx_s_bwdCallableCtx_split_v1_0( _S1933, _S1935, _S1937 );
    var _S1941 : s_paramCtx_s_bwdCallableCtx_split_v2_v3_0 = s_paramCtx_s_bwdCallableCtx_split_v2_v3_0( _S1933, _S1935, _S1937 );
    var _S1942 : s_paramCtx_s_bwdCallableCtx_chroma_v1_0 = s_paramCtx_s_bwdCallableCtx_chroma_v1_0( _S1933, _S1935, _S1937 );
    var _S1943 : s_paramCtx_s_bwdCallableCtx_chroma_v2_v3_0 = s_paramCtx_s_bwdCallableCtx_chroma_v2_v3_0( _S1933, _S1935, _S1937 );
    var _S1944 : bool = (_S1930.value0_23.value2_1.version_0) == u32(4);
    var _S1945 : s_bwdCallableCtx_filmic_v7_0;
    if(_S1944)
    {
        var _S1946 : s_paramCtx_s_bwdCallableCtx_filmic_v7_0;
        _S1946.value0_24 = _S1930.value0_23.value0_22;
        _S1946.value1_2 = _S1930.value0_23.value1_1;
        _S1946.value2_2 = _S1930.value0_23.value2_1;
        _S1945.value0_30 = _S1946;
    }
    else
    {
        _S1945.value0_30 = _S1938;
    }
    var _S1947 : bool = !_S1944;
    var _runFlag_12 : bool;
    var _runFlag_13 : bool;
    var _S1948 : bool;
    var _S1949 : bool;
    var _runFlag_14 : bool;
    var _S1950 : bool;
    var _S1951 : bool;
    var _S1952 : bool;
    var _S1953 : s_bwdCallableCtx_filmic_v6_0;
    var _S1954 : s_bwdCallableCtx_split_v1_0;
    var _S1955 : s_bwdCallableCtx_split_v2_v3_0;
    var _S1956 : s_bwdCallableCtx_chroma_v1_0;
    var _S1957 : s_bwdCallableCtx_chroma_v2_v3_0;
    if(_S1947)
    {
        var _S1958 : bool = (_S1930.value0_23.value2_1.version_0) == u32(3);
        if(_S1958)
        {
            var _S1959 : s_paramCtx_s_bwdCallableCtx_filmic_v6_0;
            _S1959.value0_25 = _S1930.value0_23.value0_22;
            _S1959.value1_3 = _S1930.value0_23.value1_1;
            _S1959.value2_3 = _S1930.value0_23.value2_1;
            _runFlag_12 = false;
            _S1953.value0_31 = _S1959;
        }
        else
        {
            _runFlag_12 = _S1947;
            _S1953.value0_31 = _S1939;
        }
        if(_runFlag_12)
        {
            var _S1960 : bool = (_S1930.value0_23.value2_1.norm_0) == u32(0);
            if(_S1960)
            {
                var _S1961 : bool = (_S1930.value0_23.value2_1.version_0) == u32(0);
                if(_S1961)
                {
                    var _S1962 : s_paramCtx_s_bwdCallableCtx_split_v1_0;
                    _S1962.value0_26 = _S1930.value0_23.value0_22;
                    _S1962.value1_4 = _S1930.value0_23.value1_1;
                    _S1962.value2_4 = _S1930.value0_23.value2_1;
                    _runFlag_13 = false;
                    _S1954.value0_32 = _S1962;
                }
                else
                {
                    _runFlag_13 = _runFlag_12;
                    _S1954.value0_32 = _S1940;
                }
                if(_runFlag_13)
                {
                    var _S1963 : s_paramCtx_s_bwdCallableCtx_split_v2_v3_0;
                    _S1963.value0_27 = _S1930.value0_23.value0_22;
                    _S1963.value1_5 = _S1930.value0_23.value1_1;
                    _S1963.value2_5 = _S1930.value0_23.value2_1;
                    _S1955.value0_33 = _S1963;
                }
                else
                {
                    _S1955.value0_33 = _S1941;
                }
                var _S1964 : bool = _runFlag_13;
                _runFlag_13 = false;
                _S1948 = _S1964;
                _S1949 = _S1961;
            }
            else
            {
                _runFlag_13 = _runFlag_12;
                _S1948 = false;
                _S1955.value0_33 = _S1941;
                _S1949 = false;
                _S1954.value0_32 = _S1940;
            }
            if(_runFlag_13)
            {
                var _S1965 : bool = (_S1930.value0_23.value2_1.version_0) == u32(0);
                if(_S1965)
                {
                    var _S1966 : s_paramCtx_s_bwdCallableCtx_chroma_v1_0;
                    _S1966.value0_28 = _S1930.value0_23.value0_22;
                    _S1966.value1_6 = _S1930.value0_23.value1_1;
                    _S1966.value2_6 = _S1930.value0_23.value2_1;
                    _runFlag_14 = false;
                    _S1956.value0_34 = _S1966;
                }
                else
                {
                    _runFlag_14 = _runFlag_13;
                    _S1956.value0_34 = _S1942;
                }
                if(_runFlag_14)
                {
                    var _S1967 : s_paramCtx_s_bwdCallableCtx_chroma_v2_v3_0;
                    _S1967.value0_29 = _S1930.value0_23.value0_22;
                    _S1967.value1_7 = _S1930.value0_23.value1_1;
                    _S1967.value2_7 = _S1930.value0_23.value2_1;
                    _S1957.value0_35 = _S1967;
                }
                else
                {
                    _S1957.value0_35 = _S1943;
                }
                _S1950 = _S1965;
            }
            else
            {
                _runFlag_14 = false;
                _S1957.value0_35 = _S1943;
                _S1950 = false;
                _S1956.value0_34 = _S1942;
            }
            var _S1968 : bool = _S1948;
            var _S1969 : bool = _S1949;
            _S1948 = _runFlag_14;
            _S1949 = _S1950;
            _runFlag_14 = _S1960;
            _S1950 = _S1968;
            _S1951 = _S1969;
        }
        else
        {
            _runFlag_13 = false;
            _S1948 = false;
            _S1957.value0_35 = _S1943;
            _S1949 = false;
            _S1956.value0_34 = _S1942;
            _runFlag_14 = false;
            _S1950 = false;
            _S1955.value0_33 = _S1941;
            _S1951 = false;
            _S1954.value0_32 = _S1940;
        }
        _S1952 = _S1958;
    }
    else
    {
        _runFlag_12 = false;
        _runFlag_13 = false;
        _S1948 = false;
        _S1957.value0_35 = _S1943;
        _S1949 = false;
        _S1956.value0_34 = _S1942;
        _runFlag_14 = false;
        _S1950 = false;
        _S1955.value0_33 = _S1941;
        _S1951 = false;
        _S1954.value0_32 = _S1940;
        _S1952 = false;
        _S1953.value0_31 = _S1939;
    }
    var _S1970 : Curve_0 = Curve_x24_syn_dzero_0();
    var _S1971 : vec3<f32>;
    var _S1972 : Curve_0;
    if(_S1947)
    {
        if(_runFlag_12)
        {
            if(_runFlag_13)
            {
                if(_S1948)
                {
                    var _S1973 : vec3<f32> = _S1933;
                    var _S1974 : Curve_0;
                    _S1974.toe_0 = _S1934;
                    _S1974.shoulder_0 = _S1934;
                    _S1974.line_0 = _S1934;
                    _S1974.nodes_0 = _S1934;
                    _S1974.tone_0 = _S1934;
                    _S1974.display_0 = _S1934;
                    _S1974.extra_0 = _S1934;
                    s_bwdProp_chroma_v2_v3_0(_S1957, &(_S1973), &(_S1974), _S1932);
                    var _S1975 : Curve_0 = Curve_x24_syn_dadd_0(_S1974, _S1970);
                    _S1971 = _S1933;
                    _S1972 = _S1975;
                }
                else
                {
                    _S1971 = _S1932;
                    _S1972 = _S1970;
                }
                if(_S1949)
                {
                    var _S1976 : vec3<f32> = _S1933;
                    var _S1977 : Curve_0;
                    _S1977.toe_0 = _S1934;
                    _S1977.shoulder_0 = _S1934;
                    _S1977.line_0 = _S1934;
                    _S1977.nodes_0 = _S1934;
                    _S1977.tone_0 = _S1934;
                    _S1977.display_0 = _S1934;
                    _S1977.extra_0 = _S1934;
                    s_bwdProp_chroma_v1_0(_S1956, &(_S1976), &(_S1977), _S1971);
                    var _S1978 : Curve_0 = Curve_x24_syn_dadd_0(_S1977, _S1972);
                    _S1971 = _S1933;
                    _S1972 = _S1978;
                }
            }
            else
            {
                _S1971 = _S1932;
                _S1972 = _S1970;
            }
            if(_runFlag_14)
            {
                if(_S1950)
                {
                    var _S1979 : vec3<f32> = _S1933;
                    var _S1980 : Curve_0;
                    _S1980.toe_0 = _S1934;
                    _S1980.shoulder_0 = _S1934;
                    _S1980.line_0 = _S1934;
                    _S1980.nodes_0 = _S1934;
                    _S1980.tone_0 = _S1934;
                    _S1980.display_0 = _S1934;
                    _S1980.extra_0 = _S1934;
                    s_bwdProp_split_v2_v3_0(_S1955, &(_S1979), &(_S1980), _S1971);
                    var _S1981 : Curve_0 = Curve_x24_syn_dadd_0(_S1980, _S1972);
                    _S1971 = _S1933;
                    _S1972 = _S1981;
                }
                if(_S1951)
                {
                    var _S1982 : vec3<f32> = _S1933;
                    var _S1983 : Curve_0;
                    _S1983.toe_0 = _S1934;
                    _S1983.shoulder_0 = _S1934;
                    _S1983.line_0 = _S1934;
                    _S1983.nodes_0 = _S1934;
                    _S1983.tone_0 = _S1934;
                    _S1983.display_0 = _S1934;
                    _S1983.extra_0 = _S1934;
                    s_bwdProp_split_v1_0(_S1954, &(_S1982), &(_S1983), _S1971);
                    var _S1984 : Curve_0 = Curve_x24_syn_dadd_0(_S1983, _S1972);
                    _S1971 = _S1933;
                    _S1972 = _S1984;
                }
            }
        }
        else
        {
            _S1971 = _S1932;
            _S1972 = _S1970;
        }
        if(_S1952)
        {
            var _S1985 : vec3<f32> = _S1933;
            var _S1986 : Curve_0;
            _S1986.toe_0 = _S1934;
            _S1986.shoulder_0 = _S1934;
            _S1986.line_0 = _S1934;
            _S1986.nodes_0 = _S1934;
            _S1986.tone_0 = _S1934;
            _S1986.display_0 = _S1934;
            _S1986.extra_0 = _S1934;
            s_bwdProp_filmic_v6_0(_S1953, &(_S1985), &(_S1986), _S1971);
            var _S1987 : Curve_0 = Curve_x24_syn_dadd_0(_S1986, _S1972);
            _S1971 = _S1933;
            _S1972 = _S1987;
        }
    }
    else
    {
        _S1971 = _S1932;
        _S1972 = _S1970;
    }
    if(_S1944)
    {
        var _S1988 : vec3<f32> = _S1933;
        var _S1989 : Curve_0;
        _S1989.toe_0 = _S1934;
        _S1989.shoulder_0 = _S1934;
        _S1989.line_0 = _S1934;
        _S1989.nodes_0 = _S1934;
        _S1989.tone_0 = _S1934;
        _S1989.display_0 = _S1934;
        _S1989.extra_0 = _S1934;
        s_bwdProp_filmic_v7_0(_S1945, &(_S1988), &(_S1989), _S1971);
        _S1972 = Curve_x24_syn_dadd_0(_S1989, _S1972);
    }
    (*_S1931) = _S1972;
    return;
}

fn s_bwdProp_along_0( _S1990 : s_bwdCallableCtx_along_0,  _S1991 : ptr<function, vec4<f32>>,  _S1992 : Curve_0)
{
    var _S1993 : vec4<f32> = _S1990.value0_21.value0_20.d3_0.extra_0 * _S1992.extra_0;
    var _S1994 : vec4<f32> = _S1990.value0_21.value0_20.d2_0.extra_0 * _S1992.extra_0;
    var _S1995 : vec4<f32> = _S1990.value0_21.value0_20.d1_0.extra_0 * _S1992.extra_0;
    var _S1996 : vec4<f32> = _S1990.value0_21.value0_20.d0_0.extra_0 * _S1992.extra_0;
    var _S1997 : vec4<f32> = _S1990.value0_21.value0_20.d3_0.display_0 * _S1992.display_0;
    var _S1998 : vec4<f32> = _S1990.value0_21.value0_20.d2_0.display_0 * _S1992.display_0;
    var _S1999 : vec4<f32> = _S1990.value0_21.value0_20.d1_0.display_0 * _S1992.display_0;
    var _S2000 : vec4<f32> = _S1990.value0_21.value0_20.d0_0.display_0 * _S1992.display_0;
    var _S2001 : vec4<f32> = _S1990.value0_21.value0_20.d3_0.tone_0 * _S1992.tone_0;
    var _S2002 : vec4<f32> = _S1990.value0_21.value0_20.d2_0.tone_0 * _S1992.tone_0;
    var _S2003 : vec4<f32> = _S1990.value0_21.value0_20.d1_0.tone_0 * _S1992.tone_0;
    var _S2004 : vec4<f32> = _S1990.value0_21.value0_20.d0_0.tone_0 * _S1992.tone_0;
    var _S2005 : vec4<f32> = _S1990.value0_21.value0_20.d3_0.nodes_0 * _S1992.nodes_0;
    var _S2006 : vec4<f32> = _S1990.value0_21.value0_20.d2_0.nodes_0 * _S1992.nodes_0;
    var _S2007 : vec4<f32> = _S1990.value0_21.value0_20.d1_0.nodes_0 * _S1992.nodes_0;
    var _S2008 : vec4<f32> = _S1990.value0_21.value0_20.d0_0.nodes_0 * _S1992.nodes_0;
    var _S2009 : vec4<f32> = _S1990.value0_21.value0_20.d3_0.line_0 * _S1992.line_0;
    var _S2010 : vec4<f32> = _S1990.value0_21.value0_20.d2_0.line_0 * _S1992.line_0;
    var _S2011 : vec4<f32> = _S1990.value0_21.value0_20.d1_0.line_0 * _S1992.line_0;
    var _S2012 : vec4<f32> = _S1990.value0_21.value0_20.d0_0.line_0 * _S1992.line_0;
    var _S2013 : vec4<f32> = _S1990.value0_21.value0_20.d3_0.shoulder_0 * _S1992.shoulder_0;
    var _S2014 : vec4<f32> = _S1990.value0_21.value0_20.d2_0.shoulder_0 * _S1992.shoulder_0;
    var _S2015 : vec4<f32> = _S1990.value0_21.value0_20.d1_0.shoulder_0 * _S1992.shoulder_0;
    var _S2016 : vec4<f32> = _S1990.value0_21.value0_20.d0_0.shoulder_0 * _S1992.shoulder_0;
    var _S2017 : vec4<f32> = _S1990.value0_21.value0_20.d3_0.toe_0 * _S1992.toe_0;
    var _S2018 : vec4<f32> = _S1990.value0_21.value0_20.d2_0.toe_0 * _S1992.toe_0;
    var _S2019 : vec4<f32> = _S1990.value0_21.value0_20.d1_0.toe_0 * _S1992.toe_0;
    var _S2020 : vec4<f32> = _S1990.value0_21.value0_20.d0_0.toe_0 * _S1992.toe_0;
    (*_S1991) = vec4<f32>(_S1996[i32(0)] + _S1996[i32(1)] + _S1996[i32(2)] + _S1996[i32(3)] + (_S2000[i32(0)] + _S2000[i32(1)] + _S2000[i32(2)] + _S2000[i32(3)]) + (_S2004[i32(0)] + _S2004[i32(1)] + _S2004[i32(2)] + _S2004[i32(3)]) + (_S2008[i32(0)] + _S2008[i32(1)] + _S2008[i32(2)] + _S2008[i32(3)]) + (_S2012[i32(0)] + _S2012[i32(1)] + _S2012[i32(2)] + _S2012[i32(3)]) + (_S2016[i32(0)] + _S2016[i32(1)] + _S2016[i32(2)] + _S2016[i32(3)]) + (_S2020[i32(0)] + _S2020[i32(1)] + _S2020[i32(2)] + _S2020[i32(3)]), _S1995[i32(0)] + _S1995[i32(1)] + _S1995[i32(2)] + _S1995[i32(3)] + (_S1999[i32(0)] + _S1999[i32(1)] + _S1999[i32(2)] + _S1999[i32(3)]) + (_S2003[i32(0)] + _S2003[i32(1)] + _S2003[i32(2)] + _S2003[i32(3)]) + (_S2007[i32(0)] + _S2007[i32(1)] + _S2007[i32(2)] + _S2007[i32(3)]) + (_S2011[i32(0)] + _S2011[i32(1)] + _S2011[i32(2)] + _S2011[i32(3)]) + (_S2015[i32(0)] + _S2015[i32(1)] + _S2015[i32(2)] + _S2015[i32(3)]) + (_S2019[i32(0)] + _S2019[i32(1)] + _S2019[i32(2)] + _S2019[i32(3)]), _S1994[i32(0)] + _S1994[i32(1)] + _S1994[i32(2)] + _S1994[i32(3)] + (_S1998[i32(0)] + _S1998[i32(1)] + _S1998[i32(2)] + _S1998[i32(3)]) + (_S2002[i32(0)] + _S2002[i32(1)] + _S2002[i32(2)] + _S2002[i32(3)]) + (_S2006[i32(0)] + _S2006[i32(1)] + _S2006[i32(2)] + _S2006[i32(3)]) + (_S2010[i32(0)] + _S2010[i32(1)] + _S2010[i32(2)] + _S2010[i32(3)]) + (_S2014[i32(0)] + _S2014[i32(1)] + _S2014[i32(2)] + _S2014[i32(3)]) + (_S2018[i32(0)] + _S2018[i32(1)] + _S2018[i32(2)] + _S2018[i32(3)]), _S1993[i32(0)] + _S1993[i32(1)] + _S1993[i32(2)] + _S1993[i32(3)] + (_S1997[i32(0)] + _S1997[i32(1)] + _S1997[i32(2)] + _S1997[i32(3)]) + (_S2001[i32(0)] + _S2001[i32(1)] + _S2001[i32(2)] + _S2001[i32(3)]) + (_S2005[i32(0)] + _S2005[i32(1)] + _S2005[i32(2)] + _S2005[i32(3)]) + (_S2009[i32(0)] + _S2009[i32(1)] + _S2009[i32(2)] + _S2009[i32(3)]) + (_S2013[i32(0)] + _S2013[i32(1)] + _S2013[i32(2)] + _S2013[i32(3)]) + (_S2017[i32(0)] + _S2017[i32(1)] + _S2017[i32(2)] + _S2017[i32(3)]));
    return;
}

fn s_bwdProp_filmic_along_0( _S2021 : s_bwdCallableCtx_filmic_along_0,  _S2022 : ptr<function, vec4<f32>>,  _S2023 : vec3<f32>)
{
    var _S2024 : Tuple_15 = s_apply_along_0(_S2021.value0_18.value2_0, _S2021.value0_18.value1_0);
    var _S2025 : s_paramCtx_s_bwdCallableCtx_along_0;
    _S2025.value0_20 = _S2021.value0_18.value2_0;
    var _S2026 : s_bwdCallableCtx_along_0 = s_bwdCallableCtx_along_0( _S2025 );
    var _S2027 : s_paramCtx_s_bwdCallableCtx_filmic_0;
    _S2027.value0_22 = _S2021.value0_18.value0_17;
    _S2027.value1_1 = _S2024.value0_19;
    _S2027.value2_1 = _S2021.value0_18.value3_0;
    var _S2028 : s_bwdCallableCtx_filmic_0 = s_bwdCallableCtx_filmic_0( _S2027 );
    var _S2029 : vec4<f32> = vec4<f32>(0.0f);
    var _S2030 : Curve_0;
    _S2030.toe_0 = _S2029;
    _S2030.shoulder_0 = _S2029;
    _S2030.line_0 = _S2029;
    _S2030.nodes_0 = _S2029;
    _S2030.tone_0 = _S2029;
    _S2030.display_0 = _S2029;
    _S2030.extra_0 = _S2029;
    s_bwdProp_filmic_0(_S2028, &(_S2030), _S2023);
    var _S2031 : vec4<f32> = _S2029;
    s_bwdProp_along_0(_S2026, &(_S2031), _S2030);
    (*_S2022) = _S2031;
    return;
}

fn _S2032( _S2033 : vec3<f32>,  _S2034 : ptr<function, DiffPair_vectorx3Cfloatx2C4x3E_0>,  _S2035 : Directions_0,  _S2036 : Modes_0,  _S2037 : vec3<f32>)
{
    var _S2038 : s_paramCtx_s_bwdCallableCtx_filmic_along_0;
    _S2038.value0_17 = _S2033;
    _S2038.value1_0 = (*_S2034).primal_0;
    _S2038.value2_0 = _S2035;
    _S2038.value3_0 = _S2036;
    var _S2039 : s_bwdCallableCtx_filmic_along_0 = s_bwdCallableCtx_filmic_along_0( _S2038 );
    var _S2040 : vec4<f32> = (*_S2034).differential_0;
    s_bwdProp_filmic_along_0(_S2039, &(_S2040), _S2037);
    (*_S2034).differential_0 = _S2040;
    return;
}

fn _d_max_1( dpx_13 : DiffPair_float_0,  dpy_5 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S2041 : f32 = max(dpx_13.primal_0, dpy_5.primal_0);
    var _S2042 : f32;
    if(((dpx_13.primal_0) > (dpy_5.primal_0)))
    {
        _S2042 = dpx_13.differential_0;
    }
    else
    {
        if(((dpx_13.primal_0) < (dpy_5.primal_0)))
        {
            _S2042 = dpy_5.differential_0;
        }
        else
        {
            _S2042 = 0.5f * (dpx_13.differential_0 + dpy_5.differential_0);
        }
    }
    var _S2043 : DiffPair_float_0 = DiffPair_float_0( _S2041, _S2042 );
    return _S2043;
}

fn _d_max_vector_1( dpx_14 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpy_6 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S2044 : DiffPair_float_0 = DiffPair_float_0( dpx_14.primal_0[i32(0)], dpx_14.differential_0[i32(0)] );
    var _S2045 : DiffPair_float_0 = DiffPair_float_0( dpy_6.primal_0[i32(0)], dpy_6.differential_0[i32(0)] );
    var dp_elem_0 : DiffPair_float_0 = _d_max_1(_S2044, _S2045);
    var result_2 : vec3<f32>;
    result_2[i32(0)] = dp_elem_0.primal_0;
    var d_result_0 : vec3<f32>;
    d_result_0[i32(0)] = dp_elem_0.differential_0;
    var _S2046 : DiffPair_float_0 = DiffPair_float_0( dpx_14.primal_0[i32(1)], dpx_14.differential_0[i32(1)] );
    var _S2047 : DiffPair_float_0 = DiffPair_float_0( dpy_6.primal_0[i32(1)], dpy_6.differential_0[i32(1)] );
    var dp_elem_1 : DiffPair_float_0 = _d_max_1(_S2046, _S2047);
    result_2[i32(1)] = dp_elem_1.primal_0;
    d_result_0[i32(1)] = dp_elem_1.differential_0;
    var _S2048 : DiffPair_float_0 = DiffPair_float_0( dpx_14.primal_0[i32(2)], dpx_14.differential_0[i32(2)] );
    var _S2049 : DiffPair_float_0 = DiffPair_float_0( dpy_6.primal_0[i32(2)], dpy_6.differential_0[i32(2)] );
    var dp_elem_2 : DiffPair_float_0 = _d_max_1(_S2048, _S2049);
    result_2[i32(2)] = dp_elem_2.primal_0;
    d_result_0[i32(2)] = dp_elem_2.differential_0;
    var _S2050 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( result_2, d_result_0 );
    return _S2050;
}

fn _d_exp_1( dpx_15 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S2051 : f32 = exp(dpx_15.primal_0);
    var _S2052 : DiffPair_float_0 = DiffPair_float_0( _S2051, _S2051 * dpx_15.differential_0 );
    return _S2052;
}

fn _d_dot_1( dpx_16 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpy_7 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_float_0
{
    var _S2053 : DiffPair_float_0 = DiffPair_float_0( dpx_16.primal_0[i32(0)] * dpy_7.primal_0[i32(0)] + dpx_16.primal_0[i32(1)] * dpy_7.primal_0[i32(1)] + dpx_16.primal_0[i32(2)] * dpy_7.primal_0[i32(2)], dpx_16.primal_0[i32(0)] * dpy_7.differential_0[i32(0)] + dpy_7.primal_0[i32(0)] * dpx_16.differential_0[i32(0)] + dpx_16.primal_0[i32(1)] * dpy_7.differential_0[i32(1)] + dpy_7.primal_0[i32(1)] * dpx_16.differential_0[i32(1)] + dpx_16.primal_0[i32(2)] * dpy_7.differential_0[i32(2)] + dpy_7.primal_0[i32(2)] * dpx_16.differential_0[i32(2)] );
    return _S2053;
}

fn _d_clamp_1( dpx_17 : DiffPair_float_0,  dpMin_1 : DiffPair_float_0,  dpMax_1 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S2054 : f32 = clamp(dpx_17.primal_0, dpMin_1.primal_0, dpMax_1.primal_0);
    var _S2055 : bool;
    if(((dpx_17.primal_0) >= (dpMin_1.primal_0)))
    {
        _S2055 = ((dpx_17.primal_0) <= (dpMax_1.primal_0));
    }
    else
    {
        _S2055 = false;
    }
    var _S2056 : f32;
    if(_S2055)
    {
        _S2056 = dpx_17.differential_0;
    }
    else
    {
        if(((dpx_17.primal_0) < (dpMin_1.primal_0)))
        {
            _S2056 = dpMin_1.differential_0;
        }
        else
        {
            _S2056 = dpMax_1.differential_0;
        }
    }
    var _S2057 : DiffPair_float_0 = DiffPair_float_0( _S2054, _S2056 );
    return _S2057;
}

fn _d_clamp_vector_1( dpx_18 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpy_8 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpz_1 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S2058 : DiffPair_float_0 = DiffPair_float_0( dpx_18.primal_0[i32(0)], dpx_18.differential_0[i32(0)] );
    var _S2059 : DiffPair_float_0 = DiffPair_float_0( dpy_8.primal_0[i32(0)], dpy_8.differential_0[i32(0)] );
    var _S2060 : DiffPair_float_0 = DiffPair_float_0( dpz_1.primal_0[i32(0)], dpz_1.differential_0[i32(0)] );
    var dp_elem_3 : DiffPair_float_0 = _d_clamp_1(_S2058, _S2059, _S2060);
    var result_3 : vec3<f32>;
    result_3[i32(0)] = dp_elem_3.primal_0;
    var d_result_1 : vec3<f32>;
    d_result_1[i32(0)] = dp_elem_3.differential_0;
    var _S2061 : DiffPair_float_0 = DiffPair_float_0( dpx_18.primal_0[i32(1)], dpx_18.differential_0[i32(1)] );
    var _S2062 : DiffPair_float_0 = DiffPair_float_0( dpy_8.primal_0[i32(1)], dpy_8.differential_0[i32(1)] );
    var _S2063 : DiffPair_float_0 = DiffPair_float_0( dpz_1.primal_0[i32(1)], dpz_1.differential_0[i32(1)] );
    var dp_elem_4 : DiffPair_float_0 = _d_clamp_1(_S2061, _S2062, _S2063);
    result_3[i32(1)] = dp_elem_4.primal_0;
    d_result_1[i32(1)] = dp_elem_4.differential_0;
    var _S2064 : DiffPair_float_0 = DiffPair_float_0( dpx_18.primal_0[i32(2)], dpx_18.differential_0[i32(2)] );
    var _S2065 : DiffPair_float_0 = DiffPair_float_0( dpy_8.primal_0[i32(2)], dpy_8.differential_0[i32(2)] );
    var _S2066 : DiffPair_float_0 = DiffPair_float_0( dpz_1.primal_0[i32(2)], dpz_1.differential_0[i32(2)] );
    var dp_elem_5 : DiffPair_float_0 = _d_clamp_1(_S2064, _S2065, _S2066);
    result_3[i32(2)] = dp_elem_5.primal_0;
    d_result_1[i32(2)] = dp_elem_5.differential_0;
    var _S2067 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( result_3, d_result_1 );
    return _S2067;
}

fn _d_min_1( dpx_19 : DiffPair_float_0,  dpy_9 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S2068 : f32 = min(dpx_19.primal_0, dpy_9.primal_0);
    var _S2069 : f32;
    if(((dpx_19.primal_0) < (dpy_9.primal_0)))
    {
        _S2069 = dpx_19.differential_0;
    }
    else
    {
        if(((dpx_19.primal_0) > (dpy_9.primal_0)))
        {
            _S2069 = dpy_9.differential_0;
        }
        else
        {
            _S2069 = 0.5f * (dpx_19.differential_0 + dpy_9.differential_0);
        }
    }
    var _S2070 : DiffPair_float_0 = DiffPair_float_0( _S2068, _S2069 );
    return _S2070;
}

fn _d_abs_vector_1( dpx_20 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S2071 : vec3<f32> = vec3<f32>(0.0f);
    var _S2072 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( abs(dpx_20.primal_0), select(select(vec3<f32>(-1.0f), _S2071, (dpx_20.primal_0) == _S2071), vec3<f32>(1.0f), ((dpx_20.primal_0) > _S2071)) * dpx_20.differential_0 );
    return _S2072;
}

fn _d_abs_1( dpx_21 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S2073 : DiffPair_float_0 = DiffPair_float_0( abs(dpx_21.primal_0), select(select(-1.0f, 0.0f, (dpx_21.primal_0) == 0.0f), 1.0f, ((dpx_21.primal_0) > 0.0f)) * dpx_21.differential_0 );
    return _S2073;
}

fn _d_log2_1( dpx_22 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S2074 : DiffPair_float_0 = DiffPair_float_0( log2(dpx_22.primal_0), 1.0f / (dpx_22.primal_0 * 0.69314718246459961f) * dpx_22.differential_0 );
    return _S2074;
}

fn log_encoding_0( x_23 : f32,  c_1 : Curve_0) -> f32
{
    return (log2(x_23 / c_1.nodes_0.z) - c_1.nodes_0.w) / c_1.tone_0.x;
}

struct DiffPair_Curve_0
{
     primal_0 : Curve_0,
     differential_0 : Curve_0,
};

fn s_fwd_log_encoding_0( dpx_23 : DiffPair_float_0,  dpc_0 : DiffPair_Curve_0) -> DiffPair_float_0
{
    var _S2075 : f32 = dpc_0.primal_0.nodes_0.z;
    var _S2076 : DiffPair_float_0 = DiffPair_float_0( dpx_23.primal_0 / _S2075, (dpx_23.differential_0 * _S2075 - dpx_23.primal_0 * dpc_0.differential_0.nodes_0.z) / (_S2075 * _S2075) );
    var _S2077 : DiffPair_float_0 = _d_log2_1(_S2076);
    var _S2078 : f32 = _S2077.primal_0 - dpc_0.primal_0.nodes_0.w;
    var _S2079 : f32 = dpc_0.primal_0.tone_0.x;
    var _S2080 : DiffPair_float_0 = DiffPair_float_0( _S2078 / _S2079, ((_S2077.differential_0 - dpc_0.differential_0.nodes_0.w) * _S2079 - _S2078 * dpc_0.differential_0.tone_0.x) / (_S2079 * _S2079) );
    return _S2080;
}

fn log_v2_0( x_24 : f32,  c_2 : Curve_0,  white_3 : bool) -> f32
{
    if(white_3)
    {
        return 1.0f;
    }
    return clamp(log_encoding_0(x_24, c_2), 0.0f, 1.0f);
}

fn s_fwd_log_v2_0( dpx_24 : DiffPair_float_0,  dpc_1 : DiffPair_Curve_0,  white_4 : bool) -> DiffPair_float_0
{
    if(white_4)
    {
        var _S2081 : DiffPair_float_0 = DiffPair_float_0( 1.0f, 0.0f );
        return _S2081;
    }
    var _S2082 : DiffPair_float_0 = DiffPair_float_0( dpx_24.primal_0, dpx_24.differential_0 );
    var _S2083 : DiffPair_Curve_0 = DiffPair_Curve_0( dpc_1.primal_0, dpc_1.differential_0 );
    var _S2084 : DiffPair_float_0 = s_fwd_log_encoding_0(_S2082, _S2083);
    var _S2085 : DiffPair_float_0 = DiffPair_float_0( _S2084.primal_0, _S2084.differential_0 );
    var _S2086 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S2087 : DiffPair_float_0 = DiffPair_float_0( 1.0f, 0.0f );
    var _S2088 : DiffPair_float_0 = _d_clamp_1(_S2085, _S2086, _S2087);
    var _S2089 : DiffPair_float_0 = DiffPair_float_0( _S2088.primal_0, _S2088.differential_0 );
    return _S2089;
}

fn spline_0( x_25 : f32,  c_3 : Curve_0,  toe_3 : u32,  shoulder_3 : u32) -> f32
{
    var _S2090 : f32 = c_3.nodes_0.x;
    if((x_25 < _S2090))
    {
        if(toe_3 == u32(2))
        {
            var xi_4 : f32 = _S2090 - x_25;
            var rat_4 : f32 = xi_4 * (xi_4 * c_3.toe_0.y + 1.0f);
            return c_3.toe_0.w - c_3.toe_0.x * rat_4 / (rat_4 + c_3.toe_0.z);
        }
        var d_5 : f32 = x_25 - _S2090;
        return c_3.toe_0.x + d_5 * (c_3.toe_0.y + d_5 * (c_3.toe_0.z + d_5 * (c_3.toe_0.w + d_5 * c_3.line_0.x)));
    }
    var _S2091 : f32 = c_3.nodes_0.y;
    if((x_25 > _S2091))
    {
        if(shoulder_3 == u32(2))
        {
            var xi_5 : f32 = x_25 - _S2091;
            var rat_5 : f32 = xi_5 * (xi_5 * c_3.shoulder_0.y + 1.0f);
            return c_3.shoulder_0.w + c_3.shoulder_0.x * rat_5 / (rat_5 + c_3.shoulder_0.z);
        }
        var d_6 : f32 = x_25 - _S2091;
        return c_3.shoulder_0.x + d_6 * (c_3.shoulder_0.y + d_6 * (c_3.shoulder_0.z + d_6 * (c_3.shoulder_0.w + d_6 * c_3.line_0.y)));
    }
    return c_3.line_0.z + x_25 * c_3.line_0.w;
}

fn s_fwd_spline_0( dpx_25 : DiffPair_float_0,  dpc_2 : DiffPair_Curve_0,  toe_4 : u32,  shoulder_4 : u32) -> DiffPair_float_0
{
    var _S2092 : f32 = dpc_2.primal_0.nodes_0.x;
    var _S2093 : f32 = dpc_2.differential_0.nodes_0.x;
    if(((dpx_25.primal_0) < _S2092))
    {
        if(toe_4 == u32(2))
        {
            var xi_6 : f32 = _S2092 - dpx_25.primal_0;
            var s_diff_xi_0 : f32 = _S2093 - dpx_25.differential_0;
            var _S2094 : f32 = dpc_2.primal_0.toe_0.y;
            var _S2095 : f32 = xi_6 * _S2094 + 1.0f;
            var rat_6 : f32 = xi_6 * _S2095;
            var s_diff_rat_0 : f32 = s_diff_xi_0 * _S2095 + (s_diff_xi_0 * _S2094 + dpc_2.differential_0.toe_0.y * xi_6) * xi_6;
            var _S2096 : f32 = dpc_2.primal_0.toe_0.x;
            var _S2097 : f32 = _S2096 * rat_6;
            var _S2098 : f32 = rat_6 + dpc_2.primal_0.toe_0.z;
            var _S2099 : DiffPair_float_0 = DiffPair_float_0( dpc_2.primal_0.toe_0.w - _S2097 / _S2098, dpc_2.differential_0.toe_0.w - ((dpc_2.differential_0.toe_0.x * rat_6 + s_diff_rat_0 * _S2096) * _S2098 - _S2097 * (s_diff_rat_0 + dpc_2.differential_0.toe_0.z)) / (_S2098 * _S2098) );
            return _S2099;
        }
        var d_7 : f32 = dpx_25.primal_0 - _S2092;
        var s_diff_d_0 : f32 = dpx_25.differential_0 - _S2093;
        var _S2100 : f32 = dpc_2.primal_0.line_0.x;
        var _S2101 : f32 = dpc_2.primal_0.toe_0.w + d_7 * _S2100;
        var _S2102 : f32 = dpc_2.primal_0.toe_0.z + d_7 * _S2101;
        var _S2103 : f32 = dpc_2.primal_0.toe_0.y + d_7 * _S2102;
        var _S2104 : DiffPair_float_0 = DiffPair_float_0( dpc_2.primal_0.toe_0.x + d_7 * _S2103, dpc_2.differential_0.toe_0.x + (s_diff_d_0 * _S2103 + (dpc_2.differential_0.toe_0.y + (s_diff_d_0 * _S2102 + (dpc_2.differential_0.toe_0.z + (s_diff_d_0 * _S2101 + (dpc_2.differential_0.toe_0.w + (s_diff_d_0 * _S2100 + dpc_2.differential_0.line_0.x * d_7)) * d_7)) * d_7)) * d_7) );
        return _S2104;
    }
    var _S2105 : f32 = dpc_2.primal_0.nodes_0.y;
    var _S2106 : f32 = dpc_2.differential_0.nodes_0.y;
    if(((dpx_25.primal_0) > _S2105))
    {
        if(shoulder_4 == u32(2))
        {
            var xi_7 : f32 = dpx_25.primal_0 - _S2105;
            var s_diff_xi_1 : f32 = dpx_25.differential_0 - _S2106;
            var _S2107 : f32 = dpc_2.primal_0.shoulder_0.y;
            var _S2108 : f32 = xi_7 * _S2107 + 1.0f;
            var rat_7 : f32 = xi_7 * _S2108;
            var s_diff_rat_1 : f32 = s_diff_xi_1 * _S2108 + (s_diff_xi_1 * _S2107 + dpc_2.differential_0.shoulder_0.y * xi_7) * xi_7;
            var _S2109 : f32 = dpc_2.primal_0.shoulder_0.x;
            var _S2110 : f32 = _S2109 * rat_7;
            var _S2111 : f32 = rat_7 + dpc_2.primal_0.shoulder_0.z;
            var _S2112 : DiffPair_float_0 = DiffPair_float_0( dpc_2.primal_0.shoulder_0.w + _S2110 / _S2111, dpc_2.differential_0.shoulder_0.w + ((dpc_2.differential_0.shoulder_0.x * rat_7 + s_diff_rat_1 * _S2109) * _S2111 - _S2110 * (s_diff_rat_1 + dpc_2.differential_0.shoulder_0.z)) / (_S2111 * _S2111) );
            return _S2112;
        }
        var d_8 : f32 = dpx_25.primal_0 - _S2105;
        var s_diff_d_1 : f32 = dpx_25.differential_0 - _S2106;
        var _S2113 : f32 = dpc_2.primal_0.line_0.y;
        var _S2114 : f32 = dpc_2.primal_0.shoulder_0.w + d_8 * _S2113;
        var _S2115 : f32 = dpc_2.primal_0.shoulder_0.z + d_8 * _S2114;
        var _S2116 : f32 = dpc_2.primal_0.shoulder_0.y + d_8 * _S2115;
        var _S2117 : DiffPair_float_0 = DiffPair_float_0( dpc_2.primal_0.shoulder_0.x + d_8 * _S2116, dpc_2.differential_0.shoulder_0.x + (s_diff_d_1 * _S2116 + (dpc_2.differential_0.shoulder_0.y + (s_diff_d_1 * _S2115 + (dpc_2.differential_0.shoulder_0.z + (s_diff_d_1 * _S2114 + (dpc_2.differential_0.shoulder_0.w + (s_diff_d_1 * _S2113 + dpc_2.differential_0.line_0.y * d_8)) * d_8)) * d_8)) * d_8) );
        return _S2117;
    }
    var _S2118 : f32 = dpc_2.primal_0.line_0.w;
    var _S2119 : DiffPair_float_0 = DiffPair_float_0( dpc_2.primal_0.line_0.z + dpx_25.primal_0 * _S2118, dpc_2.differential_0.line_0.z + (dpx_25.differential_0 * _S2118 + dpc_2.differential_0.line_0.w * dpx_25.primal_0) );
    return _S2119;
}

fn exp2_dt_2( y_10 : f32,  slope_4 : ptr<function, f32>) -> f32
{
    var x_26 : f32 = clamp(y_10, -126.99999237060547f, 129.0f);
    var ipart_2 : f32 = round_away_0(x_26 - 0.5f);
    var fpart_2 : f32 = x_26 - ipart_2;
    var power_2 : f32 = (bitcast<f32>(((u32(i32(127) + i32(ipart_2)) << (u32(23))))));
    var fit_2 : f32 = (((0.013534166850149632f * fpart_2 + 0.052011463791131973f) * fpart_2 + 0.24144275486469269f) * fpart_2 + 0.69300383329391479f) * fpart_2 + 1.0000026226043701f;
    var fit_slope_2 : f32 = ((0.054136667400598526f * fpart_2 + 0.15603439509868622f) * fpart_2 + 0.48288550972938538f) * fpart_2 + 0.69300383329391479f;
    var _S2120 : bool;
    if((y_10 > -126.99999237060547f))
    {
        _S2120 = (y_10 < 129.0f);
    }
    else
    {
        _S2120 = false;
    }
    var _S2121 : f32;
    if(_S2120)
    {
        _S2121 = power_2 * fit_slope_2;
    }
    else
    {
        _S2121 = 0.0f;
    }
    (*slope_4) = _S2121;
    return power_2 * fit_2;
}

fn pow_dt_forward_0( x_27 : DiffPair_float_0,  p_3 : DiffPair_float_0) -> DiffPair_float_0
{
    var log_1 : f32 = log2_dt_0(x_27.primal_0);
    var slope_5 : f32;
    var value_0 : f32 = exp2_dt_2(log_1 * p_3.primal_0, &(slope_5));
    var _S2122 : DiffPair_float_0 = DiffPair_float_0( value_0, slope_5 * (p_3.primal_0 * log2_dt_slope_0(x_27.primal_0) * x_27.differential_0 + log_1 * p_3.differential_0) );
    return _S2122;
}

fn channel_tone_mapping_0( x_28 : f32,  c_4 : Curve_0,  m_3 : Modes_0) -> f32
{
    return pow_dt_0(clamp(spline_0(log_v2_0(max(x_28, 1.00000000317107685e-30f), c_4, false), c_4, m_3.toe_1, m_3.shoulder_1), 0.0f, c_4.display_0.z), c_4.tone_0.y);
}

fn s_fwd_channel_tone_mapping_0( dpx_26 : DiffPair_float_0,  dpc_3 : DiffPair_Curve_0,  m_4 : Modes_0) -> DiffPair_float_0
{
    var _S2123 : DiffPair_float_0 = DiffPair_float_0( dpx_26.primal_0, dpx_26.differential_0 );
    var _S2124 : DiffPair_float_0 = DiffPair_float_0( 1.00000000317107685e-30f, 0.0f );
    var _S2125 : DiffPair_float_0 = _d_max_1(_S2123, _S2124);
    var _S2126 : DiffPair_float_0 = DiffPair_float_0( _S2125.primal_0, _S2125.differential_0 );
    var _S2127 : DiffPair_Curve_0 = DiffPair_Curve_0( dpc_3.primal_0, dpc_3.differential_0 );
    var _S2128 : DiffPair_float_0 = s_fwd_log_v2_0(_S2126, _S2127, false);
    var _S2129 : DiffPair_float_0 = DiffPair_float_0( _S2128.primal_0, _S2128.differential_0 );
    var _S2130 : DiffPair_float_0 = s_fwd_spline_0(_S2129, _S2127, m_4.toe_1, m_4.shoulder_1);
    var _S2131 : DiffPair_float_0 = DiffPair_float_0( _S2130.primal_0, _S2130.differential_0 );
    var _S2132 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S2133 : DiffPair_float_0 = DiffPair_float_0( dpc_3.primal_0.display_0.z, dpc_3.differential_0.display_0.z );
    var _S2134 : DiffPair_float_0 = _d_clamp_1(_S2131, _S2132, _S2133);
    var _S2135 : DiffPair_float_0 = DiffPair_float_0( _S2134.primal_0, _S2134.differential_0 );
    var _S2136 : DiffPair_float_0 = DiffPair_float_0( dpc_3.primal_0.tone_0.y, dpc_3.differential_0.tone_0.y );
    var _S2137 : DiffPair_float_0 = pow_dt_forward_0(_S2135, _S2136);
    var _S2138 : DiffPair_float_0 = DiffPair_float_0( _S2137.primal_0, _S2137.differential_0 );
    return _S2138;
}

fn rgb_tone_mapping_0( rgb_0 : vec3<f32>,  c_5 : Curve_0,  m_5 : Modes_0) -> vec3<f32>
{
    return vec3<f32>(channel_tone_mapping_0(rgb_0.x, c_5, m_5), channel_tone_mapping_0(rgb_0.y, c_5, m_5), channel_tone_mapping_0(rgb_0.z, c_5, m_5));
}

fn s_fwd_rgb_tone_mapping_0( dprgb_0 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpc_4 : DiffPair_Curve_0,  m_6 : Modes_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S2139 : DiffPair_float_0 = DiffPair_float_0( dprgb_0.primal_0.x, dprgb_0.differential_0.x );
    var _S2140 : DiffPair_Curve_0 = DiffPair_Curve_0( dpc_4.primal_0, dpc_4.differential_0 );
    var _S2141 : DiffPair_float_0 = s_fwd_channel_tone_mapping_0(_S2139, _S2140, m_6);
    var _S2142 : DiffPair_float_0 = DiffPair_float_0( dprgb_0.primal_0.y, dprgb_0.differential_0.y );
    var _S2143 : DiffPair_float_0 = s_fwd_channel_tone_mapping_0(_S2142, _S2140, m_6);
    var _S2144 : DiffPair_float_0 = DiffPair_float_0( dprgb_0.primal_0.z, dprgb_0.differential_0.z );
    var _S2145 : DiffPair_float_0 = s_fwd_channel_tone_mapping_0(_S2144, _S2140, m_6);
    var _S2146 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S2141.primal_0, _S2143.primal_0, _S2145.primal_0), vec3<f32>(_S2141.differential_0, _S2143.differential_0, _S2145.differential_0) );
    return _S2146;
}

fn pixel_norm_0( c_6 : vec3<f32>,  norm_3 : u32,  luminance_3 : vec3<f32>) -> f32
{
    if(norm_3 == u32(1))
    {
        return max(max(c_6.x, c_6.y), c_6.z);
    }
    if(norm_3 == u32(3))
    {
        var v_0 : vec3<f32> = abs(c_6);
        var squared_2 : vec3<f32> = v_0 * v_0;
        var _S2147 : f32 = squared_2.x;
        var _S2148 : f32 = squared_2.y;
        var _S2149 : f32 = squared_2.z;
        return (_S2147 * v_0.x + _S2148 * v_0.y + _S2149 * v_0.z) / max(_S2147 + _S2148 + _S2149, 9.999999960041972e-13f);
    }
    if(norm_3 == u32(4))
    {
        var _S2150 : f32 = c_6.x;
        var _S2151 : f32 = c_6.y;
        var _S2152 : f32 = c_6.z;
        return sqrt(_S2150 * _S2150 + _S2151 * _S2151 + _S2152 * _S2152);
    }
    if(norm_3 == u32(5))
    {
        var _S2153 : f32 = c_6.x;
        var _S2154 : f32 = c_6.y;
        var _S2155 : f32 = c_6.z;
        return sqrt(_S2153 * _S2153 + _S2154 * _S2154 + _S2155 * _S2155) * 0.57735025882720947f;
    }
    return luminance_3.x * c_6.x + luminance_3.y * c_6.y + luminance_3.z * c_6.z;
}

fn s_fwd_pixel_norm_0( dpc_5 : DiffPair_vectorx3Cfloatx2C3x3E_0,  norm_4 : u32,  luminance_4 : vec3<f32>) -> DiffPair_float_0
{
    if(norm_4 == u32(1))
    {
        var _S2156 : DiffPair_float_0 = DiffPair_float_0( dpc_5.primal_0.x, dpc_5.differential_0.x );
        var _S2157 : DiffPair_float_0 = DiffPair_float_0( dpc_5.primal_0.y, dpc_5.differential_0.y );
        var _S2158 : DiffPair_float_0 = _d_max_1(_S2156, _S2157);
        var _S2159 : DiffPair_float_0 = DiffPair_float_0( _S2158.primal_0, _S2158.differential_0 );
        var _S2160 : DiffPair_float_0 = DiffPair_float_0( dpc_5.primal_0.z, dpc_5.differential_0.z );
        var _S2161 : DiffPair_float_0 = _d_max_1(_S2159, _S2160);
        var _S2162 : DiffPair_float_0 = DiffPair_float_0( _S2161.primal_0, _S2161.differential_0 );
        return _S2162;
    }
    if(norm_4 == u32(3))
    {
        var _S2163 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( dpc_5.primal_0, dpc_5.differential_0 );
        var _S2164 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_abs_vector_1(_S2163);
        var _S2165 : vec3<f32> = _S2164.primal_0;
        var squared_3 : vec3<f32> = _S2165 * _S2165;
        var _S2166 : vec3<f32> = _S2164.differential_0 * _S2164.primal_0;
        var s_diff_squared_0 : vec3<f32> = _S2166 + _S2166;
        var _S2167 : f32 = squared_3.x;
        var _S2168 : f32 = s_diff_squared_0.x;
        var _S2169 : f32 = _S2164.primal_0.x;
        var _S2170 : f32 = squared_3.y;
        var _S2171 : f32 = s_diff_squared_0.y;
        var _S2172 : f32 = _S2164.primal_0.y;
        var _S2173 : f32 = squared_3.z;
        var _S2174 : f32 = s_diff_squared_0.z;
        var _S2175 : f32 = _S2164.primal_0.z;
        var numerator_3 : f32 = _S2167 * _S2169 + _S2170 * _S2172 + _S2173 * _S2175;
        var _S2176 : DiffPair_float_0 = DiffPair_float_0( _S2167 + _S2170 + _S2173, _S2168 + _S2171 + _S2174 );
        var _S2177 : DiffPair_float_0 = DiffPair_float_0( 9.999999960041972e-13f, 0.0f );
        var _S2178 : DiffPair_float_0 = _d_max_1(_S2176, _S2177);
        var _S2179 : DiffPair_float_0 = DiffPair_float_0( numerator_3 / _S2178.primal_0, ((_S2168 * _S2169 + _S2164.differential_0.x * _S2167 + (_S2171 * _S2172 + _S2164.differential_0.y * _S2170) + (_S2174 * _S2175 + _S2164.differential_0.z * _S2173)) * _S2178.primal_0 - numerator_3 * _S2178.differential_0) / (_S2178.primal_0 * _S2178.primal_0) );
        return _S2179;
    }
    if(norm_4 == u32(4))
    {
        var _S2180 : f32 = dpc_5.primal_0.x;
        var _S2181 : f32 = dpc_5.differential_0.x * _S2180;
        var _S2182 : f32 = dpc_5.primal_0.y;
        var _S2183 : f32 = dpc_5.differential_0.y * _S2182;
        var _S2184 : f32 = dpc_5.primal_0.z;
        var _S2185 : f32 = dpc_5.differential_0.z * _S2184;
        var _S2186 : DiffPair_float_0 = DiffPair_float_0( _S2180 * _S2180 + _S2182 * _S2182 + _S2184 * _S2184, _S2181 + _S2181 + (_S2183 + _S2183) + (_S2185 + _S2185) );
        var _S2187 : DiffPair_float_0 = _d_sqrt_1(_S2186);
        var _S2188 : DiffPair_float_0 = DiffPair_float_0( _S2187.primal_0, _S2187.differential_0 );
        return _S2188;
    }
    if(norm_4 == u32(5))
    {
        var _S2189 : f32 = dpc_5.primal_0.x;
        var _S2190 : f32 = dpc_5.differential_0.x * _S2189;
        var _S2191 : f32 = dpc_5.primal_0.y;
        var _S2192 : f32 = dpc_5.differential_0.y * _S2191;
        var _S2193 : f32 = dpc_5.primal_0.z;
        var _S2194 : f32 = dpc_5.differential_0.z * _S2193;
        var _S2195 : DiffPair_float_0 = DiffPair_float_0( _S2189 * _S2189 + _S2191 * _S2191 + _S2193 * _S2193, _S2190 + _S2190 + (_S2192 + _S2192) + (_S2194 + _S2194) );
        var _S2196 : DiffPair_float_0 = _d_sqrt_1(_S2195);
        var _S2197 : DiffPair_float_0 = DiffPair_float_0( _S2196.primal_0 * 0.57735025882720947f, _S2196.differential_0 * 0.57735025882720947f );
        return _S2197;
    }
    var _S2198 : f32 = luminance_4.x;
    var _S2199 : f32 = luminance_4.y;
    var _S2200 : f32 = luminance_4.z;
    var _S2201 : DiffPair_float_0 = DiffPair_float_0( _S2198 * dpc_5.primal_0.x + _S2199 * dpc_5.primal_0.y + _S2200 * dpc_5.primal_0.z, dpc_5.differential_0.x * _S2198 + dpc_5.differential_0.y * _S2199 + dpc_5.differential_0.z * _S2200 );
    return _S2201;
}

fn pow0_forward_0( x_29 : DiffPair_float_0,  y_11 : DiffPair_float_0) -> DiffPair_float_0
{
    if(((x_29.primal_0) <= 0.0f))
    {
        var _S2202 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
        return _S2202;
    }
    var value_1 : f32 = pow(x_29.primal_0, y_11.primal_0);
    var _S2203 : DiffPair_float_0 = DiffPair_float_0( value_1, x_29.differential_0 * y_11.primal_0 * pow(x_29.primal_0, y_11.primal_0 - 1.0f) + y_11.differential_0 * value_1 * log(x_29.primal_0) );
    return _S2203;
}

fn norm_tone_mapping_0( rgb_1 : vec3<f32>,  c_7 : Curve_0,  norm_type_1 : u32,  m_7 : Modes_0) -> vec3<f32>
{
    var norm_5 : f32 = clamp(pixel_norm_0(rgb_1, norm_type_1, m_7.luminance_1), c_7.display_0.w, c_7.extra_0.x);
    return rgb_1 / vec3<f32>(norm_5) * vec3<f32>(pow0_0(clamp(spline_0(log_v2_0(norm_5, c_7, false), c_7, m_7.toe_1, m_7.shoulder_1), c_7.display_0.y, c_7.display_0.z), c_7.tone_0.y));
}

fn s_fwd_norm_tone_mapping_0( dprgb_1 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpc_6 : DiffPair_Curve_0,  norm_type_2 : u32,  m_8 : Modes_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S2204 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( dprgb_1.primal_0, dprgb_1.differential_0 );
    var _S2205 : DiffPair_float_0 = s_fwd_pixel_norm_0(_S2204, norm_type_2, m_8.luminance_1);
    var _S2206 : DiffPair_float_0 = DiffPair_float_0( _S2205.primal_0, _S2205.differential_0 );
    var _S2207 : DiffPair_float_0 = DiffPair_float_0( dpc_6.primal_0.display_0.w, dpc_6.differential_0.display_0.w );
    var _S2208 : DiffPair_float_0 = DiffPair_float_0( dpc_6.primal_0.extra_0.x, dpc_6.differential_0.extra_0.x );
    var _S2209 : DiffPair_float_0 = _d_clamp_1(_S2206, _S2207, _S2208);
    var _S2210 : vec3<f32> = vec3<f32>(_S2209.primal_0);
    var ratios_3 : vec3<f32> = dprgb_1.primal_0 / _S2210;
    var _S2211 : DiffPair_float_0 = DiffPair_float_0( _S2209.primal_0, _S2209.differential_0 );
    var _S2212 : DiffPair_Curve_0 = DiffPair_Curve_0( dpc_6.primal_0, dpc_6.differential_0 );
    var _S2213 : DiffPair_float_0 = s_fwd_log_v2_0(_S2211, _S2212, false);
    var _S2214 : DiffPair_float_0 = DiffPair_float_0( _S2213.primal_0, _S2213.differential_0 );
    var _S2215 : DiffPair_float_0 = s_fwd_spline_0(_S2214, _S2212, m_8.toe_1, m_8.shoulder_1);
    var _S2216 : DiffPair_float_0 = DiffPair_float_0( _S2215.primal_0, _S2215.differential_0 );
    var _S2217 : DiffPair_float_0 = DiffPair_float_0( dpc_6.primal_0.display_0.y, dpc_6.differential_0.display_0.y );
    var _S2218 : DiffPair_float_0 = DiffPair_float_0( dpc_6.primal_0.display_0.z, dpc_6.differential_0.display_0.z );
    var _S2219 : DiffPair_float_0 = _d_clamp_1(_S2216, _S2217, _S2218);
    var _S2220 : DiffPair_float_0 = DiffPair_float_0( _S2219.primal_0, _S2219.differential_0 );
    var _S2221 : DiffPair_float_0 = DiffPair_float_0( dpc_6.primal_0.tone_0.y, dpc_6.differential_0.tone_0.y );
    var _S2222 : DiffPair_float_0 = pow0_forward_0(_S2220, _S2221);
    var _S2223 : vec3<f32> = vec3<f32>(_S2222.primal_0);
    var _S2224 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( ratios_3 * _S2223, (dprgb_1.differential_0 * _S2210 - dprgb_1.primal_0 * vec3<f32>(_S2209.differential_0)) / vec3<f32>((_S2209.primal_0 * _S2209.primal_0)) * _S2223 + vec3<f32>(_S2222.differential_0) * ratios_3 );
    return _S2224;
}

fn lms_to_yrg_0( lms_0 : vec3<f32>) -> vec3<f32>
{
    var _S2225 : f32 = lms_0.x;
    var _S2226 : f32 = lms_0.y;
    var y_12 : f32 = 0.68990272283554077f * _S2225 + 0.34832188487052917f * _S2226;
    var a_4 : f32 = _S2225 + _S2226 + lms_0.z;
    var _S2227 : vec3<f32> = vec3<f32>(0.0f);
    var normalized_2 : vec3<f32>;
    if(a_4 != 0.0f)
    {
        normalized_2 = lms_0 / vec3<f32>(a_4);
    }
    else
    {
        normalized_2 = _S2227;
    }
    var rgb_2 : vec3<f32> = (((normalized_2) * (mat3x3<f32>(1.0877193212509155f, -0.66666668653488159f, 0.020618559792637825f, -0.087719298899173737f, 1.6666666269302368f, -0.051546391099691391f, 0.0f, 0.0f, 1.0309278964996338f))));
    return vec3<f32>(y_12, rgb_2.x, rgb_2.y);
}

fn s_fwd_lms_to_yrg_0( dplms_0 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S2228 : f32 = dplms_0.primal_0.x;
    var _S2229 : f32 = dplms_0.differential_0.x;
    var _S2230 : f32 = dplms_0.primal_0.y;
    var _S2231 : f32 = dplms_0.differential_0.y;
    var y_13 : f32 = 0.68990272283554077f * _S2228 + 0.34832188487052917f * _S2230;
    var s_diff_y_0 : f32 = _S2229 * 0.68990272283554077f + _S2231 * 0.34832188487052917f;
    var a_5 : f32 = _S2228 + _S2230 + dplms_0.primal_0.z;
    var s_diff_a_0 : f32 = _S2229 + _S2231 + dplms_0.differential_0.z;
    var _S2232 : vec3<f32> = vec3<f32>(0.0f);
    var _S2233 : vec3<f32> = vec3<f32>(0.0f);
    var normalized_3 : vec3<f32>;
    var s_diff_normalized_0 : vec3<f32>;
    if(a_5 != 0.0f)
    {
        var _S2234 : vec3<f32> = vec3<f32>(a_5);
        var _S2235 : vec3<f32> = (dplms_0.differential_0 * _S2234 - dplms_0.primal_0 * vec3<f32>(s_diff_a_0)) / vec3<f32>((a_5 * a_5));
        normalized_3 = dplms_0.primal_0 / _S2234;
        s_diff_normalized_0 = _S2235;
    }
    else
    {
        normalized_3 = _S2232;
        s_diff_normalized_0 = _S2233;
    }
    var _S2236 : vec3<f32> = (((normalized_3) * (mat3x3<f32>(1.0877193212509155f, -0.66666668653488159f, 0.020618559792637825f, -0.087719298899173737f, 1.6666666269302368f, -0.051546391099691391f, 0.0f, 0.0f, 1.0309278964996338f))));
    var _S2237 : vec3<f32> = (((normalized_3) * (mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f)))) + (((s_diff_normalized_0) * (mat3x3<f32>(1.0877193212509155f, -0.66666668653488159f, 0.020618559792637825f, -0.087719298899173737f, 1.6666666269302368f, -0.051546391099691391f, 0.0f, 0.0f, 1.0309278964996338f))));
    var _S2238 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(y_13, _S2236.x, _S2236.y), vec3<f32>(s_diff_y_0, _S2237.x, _S2237.y) );
    return _S2238;
}

fn yrg_to_ych_0( yrg_0 : vec3<f32>) -> vec4<f32>
{
    var rg_3 : vec2<f32> = yrg_0.yz - vec2<f32>(0.21902142465114594f, 0.54371398687362671f);
    var c_8 : f32 = length(rg_3);
    const _S2239 : vec2<f32> = vec2<f32>(1.0f, 0.0f);
    var cos_sin_1 : vec2<f32>;
    if(c_8 != 0.0f)
    {
        cos_sin_1 = rg_3 / vec2<f32>(c_8);
    }
    else
    {
        cos_sin_1 = _S2239;
    }
    return vec4<f32>(yrg_0.x, c_8, cos_sin_1.x, cos_sin_1.y);
}

fn s_fwd_yrg_to_ych_0( dpyrg_0 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C4x3E_0
{
    var _S2240 : vec2<f32> = dpyrg_0.differential_0.yz;
    var rg_4 : vec2<f32> = dpyrg_0.primal_0.yz - vec2<f32>(0.21902142465114594f, 0.54371398687362671f);
    var _S2241 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( rg_4, _S2240 );
    var _S2242 : DiffPair_float_0 = s_fwd_length_impl_0(_S2241);
    const _S2243 : vec2<f32> = vec2<f32>(1.0f, 0.0f);
    const _S2244 : vec2<f32> = vec2<f32>(0.0f, 0.0f);
    var cos_sin_2 : vec2<f32>;
    var s_diff_cos_sin_0 : vec2<f32>;
    if((_S2242.primal_0) != 0.0f)
    {
        var _S2245 : vec2<f32> = vec2<f32>(_S2242.primal_0);
        var _S2246 : vec2<f32> = (_S2240 * _S2245 - rg_4 * vec2<f32>(_S2242.differential_0)) / vec2<f32>((_S2242.primal_0 * _S2242.primal_0));
        cos_sin_2 = rg_4 / _S2245;
        s_diff_cos_sin_0 = _S2246;
    }
    else
    {
        cos_sin_2 = _S2243;
        s_diff_cos_sin_0 = _S2244;
    }
    var _S2247 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( vec4<f32>(dpyrg_0.primal_0.x, _S2242.primal_0, cos_sin_2.x, cos_sin_2.y), vec4<f32>(dpyrg_0.differential_0.x, _S2242.differential_0, s_diff_cos_sin_0.x, s_diff_cos_sin_0.y) );
    return _S2247;
}

fn rgb_to_ych_0( rgb_3 : vec3<f32>,  to_lms_4 : mat3x3<f32>) -> vec4<f32>
{
    var _S2248 : f32 = max(max(abs(rgb_3.x), abs(rgb_3.y)), abs(rgb_3.z));
    if((_S2248 <= 0.0f))
    {
        return yrg_to_ych_0(lms_to_yrg_0((((rgb_3) * (to_lms_4)))));
    }
    var ych_0 : vec4<f32> = yrg_to_ych_0(lms_to_yrg_0((((rgb_3 / vec3<f32>(_S2248)) * (to_lms_4)))));
    return vec4<f32>(ych_0.x * _S2248, ych_0.y, ych_0.z, ych_0.w);
}

fn s_fwd_rgb_to_ych_0( dprgb_2 : DiffPair_vectorx3Cfloatx2C3x3E_0,  to_lms_5 : mat3x3<f32>) -> DiffPair_vectorx3Cfloatx2C4x3E_0
{
    var _S2249 : DiffPair_float_0 = DiffPair_float_0( dprgb_2.primal_0.x, dprgb_2.differential_0.x );
    var _S2250 : DiffPair_float_0 = _d_abs_1(_S2249);
    var _S2251 : DiffPair_float_0 = DiffPair_float_0( dprgb_2.primal_0.y, dprgb_2.differential_0.y );
    var _S2252 : DiffPair_float_0 = _d_abs_1(_S2251);
    var _S2253 : DiffPair_float_0 = DiffPair_float_0( _S2250.primal_0, _S2250.differential_0 );
    var _S2254 : DiffPair_float_0 = DiffPair_float_0( _S2252.primal_0, _S2252.differential_0 );
    var _S2255 : DiffPair_float_0 = _d_max_1(_S2253, _S2254);
    var _S2256 : DiffPair_float_0 = DiffPair_float_0( dprgb_2.primal_0.z, dprgb_2.differential_0.z );
    var _S2257 : DiffPair_float_0 = _d_abs_1(_S2256);
    var _S2258 : DiffPair_float_0 = DiffPair_float_0( _S2255.primal_0, _S2255.differential_0 );
    var _S2259 : DiffPair_float_0 = DiffPair_float_0( _S2257.primal_0, _S2257.differential_0 );
    var _S2260 : DiffPair_float_0 = _d_max_1(_S2258, _S2259);
    if(((_S2260.primal_0) <= 0.0f))
    {
        var _S2261 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( (((dprgb_2.primal_0) * (to_lms_5))), (((dprgb_2.primal_0) * (mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f)))) + (((dprgb_2.differential_0) * (to_lms_5))) );
        var _S2262 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_lms_to_yrg_0(_S2261);
        var _S2263 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2262.primal_0, _S2262.differential_0 );
        var _S2264 : DiffPair_vectorx3Cfloatx2C4x3E_0 = s_fwd_yrg_to_ych_0(_S2263);
        var _S2265 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( _S2264.primal_0, _S2264.differential_0 );
        return _S2265;
    }
    var _S2266 : vec3<f32> = vec3<f32>(_S2260.primal_0);
    var _S2267 : vec3<f32> = dprgb_2.primal_0 / _S2266;
    var _S2268 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( (((_S2267) * (to_lms_5))), (((_S2267) * (mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f)))) + (((dprgb_2.differential_0 / _S2266) * (to_lms_5))) );
    var _S2269 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_lms_to_yrg_0(_S2268);
    var _S2270 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2269.primal_0, _S2269.differential_0 );
    var _S2271 : DiffPair_vectorx3Cfloatx2C4x3E_0 = s_fwd_yrg_to_ych_0(_S2270);
    var _S2272 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( vec4<f32>(_S2271.primal_0.x * _S2260.primal_0, _S2271.primal_0.y, _S2271.primal_0.z, _S2271.primal_0.w), vec4<f32>(_S2271.differential_0.x * _S2260.primal_0, _S2271.differential_0.y, _S2271.differential_0.z, _S2271.differential_0.w) );
    return _S2272;
}

fn desaturate_v4_0( original_0 : vec4<f32>,  final_0 : vec4<f32>,  saturation_0 : f32) -> vec4<f32>
{
    var _S2273 : f32 = original_0.x;
    var chroma_original_2 : f32 = original_0.y * _S2273;
    var _S2274 : f32 = final_0.x;
    var chroma_final_2 : f32 = final_0.y * _S2274;
    var delta_2 : f32 = saturation_0 * (chroma_original_2 - chroma_final_2);
    var resaturates_2 : bool = (chroma_original_2 < chroma_final_2);
    var desaturates_2 : bool = (chroma_original_2 > chroma_final_2);
    var _S2275 : bool;
    if((_S2274 > _S2273))
    {
        _S2275 = resaturates_2;
    }
    else
    {
        _S2275 = false;
    }
    var chroma_2 : f32;
    if(_S2275)
    {
        chroma_2 = (chroma_original_2 + chroma_final_2) / 2.0f;
    }
    else
    {
        if((saturation_0 > 0.0f))
        {
            _S2275 = desaturates_2;
        }
        else
        {
            _S2275 = false;
        }
        if(_S2275)
        {
            _S2275 = true;
        }
        else
        {
            _S2275 = (saturation_0 < 0.0f);
        }
        if(_S2275)
        {
            chroma_2 = chroma_final_2 + delta_2;
        }
        else
        {
            chroma_2 = chroma_final_2;
        }
    }
    var c_9 : f32;
    if((_S2274 > 0.0f))
    {
        c_9 = max(chroma_2 / _S2274, 0.0f);
    }
    else
    {
        c_9 = 0.0f;
    }
    return vec4<f32>(_S2274, c_9, final_0.z, final_0.w);
}

fn s_fwd_desaturate_v4_0( dporiginal_0 : DiffPair_vectorx3Cfloatx2C4x3E_0,  dpfinal_0 : DiffPair_vectorx3Cfloatx2C4x3E_0,  dpsaturation_0 : DiffPair_float_0) -> DiffPair_vectorx3Cfloatx2C4x3E_0
{
    var _S2276 : f32 = dporiginal_0.primal_0.y;
    var _S2277 : f32 = dporiginal_0.primal_0.x;
    var chroma_original_3 : f32 = _S2276 * _S2277;
    var s_diff_chroma_original_0 : f32 = dporiginal_0.differential_0.y * _S2277 + dporiginal_0.differential_0.x * _S2276;
    var _S2278 : f32 = dpfinal_0.primal_0.y;
    var _S2279 : f32 = dpfinal_0.primal_0.x;
    var _S2280 : f32 = dpfinal_0.differential_0.x;
    var chroma_final_3 : f32 = _S2278 * _S2279;
    var s_diff_chroma_final_0 : f32 = dpfinal_0.differential_0.y * _S2279 + _S2280 * _S2278;
    var _S2281 : f32 = chroma_original_3 - chroma_final_3;
    var delta_3 : f32 = dpsaturation_0.primal_0 * _S2281;
    var s_diff_delta_0 : f32 = dpsaturation_0.differential_0 * _S2281 + (s_diff_chroma_original_0 - s_diff_chroma_final_0) * dpsaturation_0.primal_0;
    var resaturates_3 : bool = (chroma_original_3 < chroma_final_3);
    var desaturates_3 : bool = (chroma_original_3 > chroma_final_3);
    var _S2282 : bool;
    if((_S2279 > _S2277))
    {
        _S2282 = resaturates_3;
    }
    else
    {
        _S2282 = false;
    }
    var chroma_3 : f32;
    var s_diff_chroma_0 : f32;
    if(_S2282)
    {
        var _S2283 : f32 = (s_diff_chroma_original_0 + s_diff_chroma_final_0) * 0.5f;
        chroma_3 = (chroma_original_3 + chroma_final_3) / 2.0f;
        s_diff_chroma_0 = _S2283;
    }
    else
    {
        if(((dpsaturation_0.primal_0) > 0.0f))
        {
            _S2282 = desaturates_3;
        }
        else
        {
            _S2282 = false;
        }
        if(_S2282)
        {
            _S2282 = true;
        }
        else
        {
            _S2282 = ((dpsaturation_0.primal_0) < 0.0f);
        }
        if(_S2282)
        {
            var _S2284 : f32 = s_diff_chroma_final_0 + s_diff_delta_0;
            chroma_3 = chroma_final_3 + delta_3;
            s_diff_chroma_0 = _S2284;
        }
        else
        {
            chroma_3 = chroma_final_3;
            s_diff_chroma_0 = s_diff_chroma_final_0;
        }
    }
    var c_10 : f32;
    var s_diff_c_0 : f32;
    if((_S2279 > 0.0f))
    {
        var _S2285 : DiffPair_float_0 = DiffPair_float_0( chroma_3 / _S2279, (s_diff_chroma_0 * _S2279 - chroma_3 * _S2280) / (_S2279 * _S2279) );
        var _S2286 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
        var _S2287 : DiffPair_float_0 = _d_max_1(_S2285, _S2286);
        c_10 = _S2287.primal_0;
        s_diff_c_0 = _S2287.differential_0;
    }
    else
    {
        c_10 = 0.0f;
        s_diff_c_0 = 0.0f;
    }
    var _S2288 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( vec4<f32>(_S2279, c_10, dpfinal_0.primal_0.z, dpfinal_0.primal_0.w), vec4<f32>(_S2280, s_diff_c_0, dpfinal_0.differential_0.z, dpfinal_0.differential_0.w) );
    return _S2288;
}

fn ych_to_yrg_0( ych_1 : vec4<f32>) -> vec3<f32>
{
    var rg_5 : vec2<f32> = vec2<f32>(ych_1.y) * ych_1.zw + vec2<f32>(0.21902142465114594f, 0.54371398687362671f);
    return vec3<f32>(ych_1.x, rg_5.x, rg_5.y);
}

fn s_fwd_ych_to_yrg_0( dpych_0 : DiffPair_vectorx3Cfloatx2C4x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S2289 : vec2<f32> = dpych_0.primal_0.zw;
    var _S2290 : vec2<f32> = vec2<f32>(dpych_0.primal_0.y);
    var _S2291 : vec2<f32> = vec2<f32>(dpych_0.differential_0.y) * _S2289 + dpych_0.differential_0.zw * _S2290;
    var rg_6 : vec2<f32> = _S2290 * _S2289 + vec2<f32>(0.21902142465114594f, 0.54371398687362671f);
    var _S2292 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(dpych_0.primal_0.x, rg_6.x, rg_6.y), vec3<f32>(dpych_0.differential_0.x, _S2291.x, _S2291.y) );
    return _S2292;
}

fn gamut_check_yrg_0( ych_2 : vec4<f32>) -> vec4<f32>
{
    var yrg_1 : vec3<f32> = ych_to_yrg_0(ych_2);
    var max_c_4 : f32 = ych_2.y;
    var _S2293 : f32 = yrg_1.y;
    var max_c_5 : f32;
    if((_S2293 < 0.0f))
    {
        max_c_5 = min(-0.21902142465114594f / ych_2.z, max_c_4);
    }
    else
    {
        max_c_5 = max_c_4;
    }
    var _S2294 : f32 = yrg_1.z;
    if((_S2294 < 0.0f))
    {
        max_c_5 = min(-0.54371398687362671f / ych_2.w, max_c_5);
    }
    if(((_S2293 + _S2294) > 1.0f))
    {
        max_c_5 = min(0.23726457357406616f / (ych_2.z + ych_2.w), max_c_5);
    }
    return vec4<f32>(ych_2.x, max_c_5, ych_2.z, ych_2.w);
}

fn s_fwd_gamut_check_yrg_0( dpych_1 : DiffPair_vectorx3Cfloatx2C4x3E_0) -> DiffPair_vectorx3Cfloatx2C4x3E_0
{
    var _S2295 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( dpych_1.primal_0, dpych_1.differential_0 );
    var _S2296 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_ych_to_yrg_0(_S2295);
    var max_c_6 : f32 = dpych_1.primal_0.y;
    var s_diff_max_c_0 : f32 = dpych_1.differential_0.y;
    var _S2297 : f32 = _S2296.primal_0.y;
    var max_c_7 : f32;
    var s_diff_max_c_1 : f32;
    if((_S2297 < 0.0f))
    {
        var _S2298 : f32 = dpych_1.primal_0.z;
        var _S2299 : DiffPair_float_0 = DiffPair_float_0( -0.21902142465114594f / _S2298, (0.0f - -0.21902142465114594f * dpych_1.differential_0.z) / (_S2298 * _S2298) );
        var _S2300 : DiffPair_float_0 = DiffPair_float_0( max_c_6, s_diff_max_c_0 );
        var _S2301 : DiffPair_float_0 = _d_min_1(_S2299, _S2300);
        max_c_7 = _S2301.primal_0;
        s_diff_max_c_1 = _S2301.differential_0;
    }
    else
    {
        max_c_7 = max_c_6;
        s_diff_max_c_1 = s_diff_max_c_0;
    }
    var _S2302 : f32 = _S2296.primal_0.z;
    if((_S2302 < 0.0f))
    {
        var _S2303 : f32 = dpych_1.primal_0.w;
        var _S2304 : DiffPair_float_0 = DiffPair_float_0( -0.54371398687362671f / _S2303, (0.0f - -0.54371398687362671f * dpych_1.differential_0.w) / (_S2303 * _S2303) );
        var _S2305 : DiffPair_float_0 = DiffPair_float_0( max_c_7, s_diff_max_c_1 );
        var _S2306 : DiffPair_float_0 = _d_min_1(_S2304, _S2305);
        max_c_7 = _S2306.primal_0;
        s_diff_max_c_1 = _S2306.differential_0;
    }
    if(((_S2297 + _S2302) > 1.0f))
    {
        var _S2307 : f32 = dpych_1.primal_0.z + dpych_1.primal_0.w;
        var _S2308 : DiffPair_float_0 = DiffPair_float_0( 0.23726457357406616f / _S2307, (0.0f - 0.23726457357406616f * (dpych_1.differential_0.z + dpych_1.differential_0.w)) / (_S2307 * _S2307) );
        var _S2309 : DiffPair_float_0 = DiffPair_float_0( max_c_7, s_diff_max_c_1 );
        var _S2310 : DiffPair_float_0 = _d_min_1(_S2308, _S2309);
        max_c_7 = _S2310.primal_0;
        s_diff_max_c_1 = _S2310.differential_0;
    }
    var _S2311 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( vec4<f32>(dpych_1.primal_0.x, max_c_7, dpych_1.primal_0.z, dpych_1.primal_0.w), vec4<f32>(dpych_1.differential_0.x, s_diff_max_c_1, dpych_1.differential_0.z, dpych_1.differential_0.w) );
    return _S2311;
}

fn yrg_to_lms_0( yrg_2 : vec3<f32>) -> vec3<f32>
{
    var _S2312 : f32 = yrg_2.y;
    var _S2313 : f32 = yrg_2.z;
    var lms_1 : vec3<f32> = (((vec3<f32>(_S2312, _S2313, 1.0f - _S2312 - _S2313)) * (mat3x3<f32>(0.94999998807907104f, 0.37999999523162842f, 0.0f, 0.05000000074505806f, 0.62000000476837158f, 0.029999999329447746f, 0.0f, 0.0f, 0.97000002861022949f))));
    var denominator_4 : f32 = 0.68990272283554077f * lms_1.x + 0.34832188487052917f * lms_1.y;
    var a_6 : f32;
    if(denominator_4 != 0.0f)
    {
        a_6 = yrg_2.x / denominator_4;
    }
    else
    {
        a_6 = 0.0f;
    }
    return lms_1 * vec3<f32>(a_6);
}

fn s_fwd_yrg_to_lms_0( dpyrg_1 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S2314 : f32 = dpyrg_1.primal_0.y;
    var _S2315 : f32 = dpyrg_1.differential_0.y;
    var _S2316 : f32 = dpyrg_1.primal_0.z;
    var _S2317 : f32 = dpyrg_1.differential_0.z;
    var _S2318 : vec3<f32> = vec3<f32>(_S2314, _S2316, 1.0f - _S2314 - _S2316);
    var _S2319 : vec3<f32> = (((_S2318) * (mat3x3<f32>(0.94999998807907104f, 0.37999999523162842f, 0.0f, 0.05000000074505806f, 0.62000000476837158f, 0.029999999329447746f, 0.0f, 0.0f, 0.97000002861022949f))));
    var _S2320 : vec3<f32> = (((_S2318) * (mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f)))) + (((vec3<f32>(_S2315, _S2317, 0.0f - _S2315 - _S2317)) * (mat3x3<f32>(0.94999998807907104f, 0.37999999523162842f, 0.0f, 0.05000000074505806f, 0.62000000476837158f, 0.029999999329447746f, 0.0f, 0.0f, 0.97000002861022949f))));
    var denominator_5 : f32 = 0.68990272283554077f * _S2319.x + 0.34832188487052917f * _S2319.y;
    var s_diff_denominator_0 : f32 = _S2320.x * 0.68990272283554077f + _S2320.y * 0.34832188487052917f;
    var a_7 : f32;
    var s_diff_a_1 : f32;
    if(denominator_5 != 0.0f)
    {
        var _S2321 : f32 = dpyrg_1.primal_0.x;
        var _S2322 : f32 = (dpyrg_1.differential_0.x * denominator_5 - _S2321 * s_diff_denominator_0) / (denominator_5 * denominator_5);
        a_7 = _S2321 / denominator_5;
        s_diff_a_1 = _S2322;
    }
    else
    {
        a_7 = 0.0f;
        s_diff_a_1 = 0.0f;
    }
    var _S2323 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2319 * vec3<f32>(a_7), _S2320 * vec3<f32>(a_7) + vec3<f32>(s_diff_a_1) * _S2319 );
    return _S2323;
}

fn ych_to_rgb_0( ych_3 : vec4<f32>,  from_lms_5 : mat3x3<f32>) -> vec3<f32>
{
    return (((yrg_to_lms_0(ych_to_yrg_0(ych_3))) * (from_lms_5)));
}

fn s_fwd_ych_to_rgb_0( dpych_2 : DiffPair_vectorx3Cfloatx2C4x3E_0,  from_lms_6 : mat3x3<f32>) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S2324 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( dpych_2.primal_0, dpych_2.differential_0 );
    var _S2325 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_ych_to_yrg_0(_S2324);
    var _S2326 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2325.primal_0, _S2325.differential_0 );
    var _S2327 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_yrg_to_lms_0(_S2326);
    var _S2328 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( (((_S2327.primal_0) * (from_lms_6))), (((_S2327.primal_0) * (mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f)))) + (((_S2327.differential_0) * (from_lms_6))) );
    return _S2328;
}

fn chroma_denominator_0( coeffs_4 : vec3<f32>,  cos_h_2 : f32,  sin_h_2 : f32) -> f32
{
    return coeffs_4.x * (0.97938144207000732f * cos_h_2 + 0.39175257086753845f * sin_h_2) + coeffs_4.y * (0.020618556067347527f * cos_h_2 + 0.60824739933013916f * sin_h_2) - coeffs_4.z * (cos_h_2 + sin_h_2);
}

fn s_fwd_chroma_denominator_0( coeffs_5 : vec3<f32>,  dpcos_h_0 : DiffPair_float_0,  dpsin_h_0 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S2329 : f32 = coeffs_5.x;
    var _S2330 : f32 = coeffs_5.y;
    var _S2331 : f32 = coeffs_5.z;
    var _S2332 : DiffPair_float_0 = DiffPair_float_0( _S2329 * (0.97938144207000732f * dpcos_h_0.primal_0 + 0.39175257086753845f * dpsin_h_0.primal_0) + _S2330 * (0.020618556067347527f * dpcos_h_0.primal_0 + 0.60824739933013916f * dpsin_h_0.primal_0) - _S2331 * (dpcos_h_0.primal_0 + dpsin_h_0.primal_0), (dpcos_h_0.differential_0 * 0.97938144207000732f + dpsin_h_0.differential_0 * 0.39175257086753845f) * _S2329 + (dpcos_h_0.differential_0 * 0.020618556067347527f + dpsin_h_0.differential_0 * 0.60824739933013916f) * _S2330 - (dpcos_h_0.differential_0 + dpsin_h_0.differential_0) * _S2331 );
    return _S2332;
}

fn clip_chroma_white_raw_0( coeffs_6 : vec3<f32>,  target_white_0 : f32,  y_14 : f32,  cos_h_3 : f32,  sin_h_3 : f32) -> f32
{
    var denominator_y_0 : f32 = chroma_denominator_0(coeffs_6, cos_h_3, sin_h_3);
    var denominator_target_2 : f32 = target_white_0 * (0.68285983800888062f * cos_h_3 + 0.4821370542049408f * sin_h_3);
    if(denominator_y_0 == 0.0f)
    {
        return 3.4028234663852886e+38f;
    }
    if((y_14 <= (denominator_target_2 / denominator_y_0)))
    {
        return 3.4028234663852886e+38f;
    }
    return -0.42750686407089233f * (y_14 * (coeffs_6.x + 0.85649234056472778f * coeffs_6.y + 0.55499595403671265f * coeffs_6.z) - 0.98823773860931396f * target_white_0) / (y_14 * denominator_y_0 - denominator_target_2);
}

fn s_fwd_clip_chroma_white_raw_0( coeffs_7 : vec3<f32>,  dptarget_white_0 : DiffPair_float_0,  dpy_10 : DiffPair_float_0,  dpcos_h_1 : DiffPair_float_0,  dpsin_h_1 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S2333 : DiffPair_float_0 = DiffPair_float_0( dpcos_h_1.primal_0, dpcos_h_1.differential_0 );
    var _S2334 : DiffPair_float_0 = DiffPair_float_0( dpsin_h_1.primal_0, dpsin_h_1.differential_0 );
    var _S2335 : DiffPair_float_0 = s_fwd_chroma_denominator_0(coeffs_7, _S2333, _S2334);
    var _S2336 : f32 = 0.68285983800888062f * dpcos_h_1.primal_0 + 0.4821370542049408f * dpsin_h_1.primal_0;
    var denominator_target_3 : f32 = dptarget_white_0.primal_0 * _S2336;
    var s_diff_denominator_target_0 : f32 = dptarget_white_0.differential_0 * _S2336 + (dpcos_h_1.differential_0 * 0.68285983800888062f + dpsin_h_1.differential_0 * 0.4821370542049408f) * dptarget_white_0.primal_0;
    if((_S2335.primal_0) == 0.0f)
    {
        var _S2337 : DiffPair_float_0 = DiffPair_float_0( 3.4028234663852886e+38f, 0.0f );
        return _S2337;
    }
    if(((dpy_10.primal_0) <= (denominator_target_3 / _S2335.primal_0)))
    {
        var _S2338 : DiffPair_float_0 = DiffPair_float_0( 3.4028234663852886e+38f, 0.0f );
        return _S2338;
    }
    var denominator_6 : f32 = dpy_10.primal_0 * _S2335.primal_0 - denominator_target_3;
    var _S2339 : f32 = coeffs_7.x + 0.85649234056472778f * coeffs_7.y + 0.55499595403671265f * coeffs_7.z;
    var numerator_4 : f32 = -0.42750686407089233f * (dpy_10.primal_0 * _S2339 - 0.98823773860931396f * dptarget_white_0.primal_0);
    var _S2340 : DiffPair_float_0 = DiffPair_float_0( numerator_4 / denominator_6, ((dpy_10.differential_0 * _S2339 - dptarget_white_0.differential_0 * 0.98823773860931396f) * -0.42750686407089233f * denominator_6 - numerator_4 * (dpy_10.differential_0 * _S2335.primal_0 + _S2335.differential_0 * dpy_10.primal_0 - s_diff_denominator_target_0)) / (denominator_6 * denominator_6) );
    return _S2340;
}

fn clip_chroma_white_0( coeffs_8 : vec3<f32>,  target_white_1 : f32,  y_15 : f32,  cos_h_4 : f32,  sin_h_4 : f32) -> f32
{
    var max_y_2 : f32 = 1.0578552484512329f * target_white_1;
    var _S2341 : f32 = max(max_y_2 - y_15, 0.0f);
    var max_chroma_3 : f32;
    if((_S2341 < 0.0010000000474974513f))
    {
        var raw_0 : f32 = clip_chroma_white_raw_0(coeffs_8, target_white_1, 0.99900001287460327f * max_y_2, cos_h_4, sin_h_4);
        if((raw_0 >= 3.4028234663852886e+38f))
        {
            return 3.4028234663852886e+38f;
        }
        max_chroma_3 = _S2341 / (0.0010000000474974513f * max_y_2) * raw_0;
    }
    else
    {
        max_chroma_3 = clip_chroma_white_raw_0(coeffs_8, target_white_1, y_15, cos_h_4, sin_h_4);
    }
    if((max_chroma_3 >= 0.0f))
    {
    }
    else
    {
        max_chroma_3 = 3.4028234663852886e+38f;
    }
    return max_chroma_3;
}

fn s_fwd_clip_chroma_white_0( coeffs_9 : vec3<f32>,  dptarget_white_1 : DiffPair_float_0,  dpy_11 : DiffPair_float_0,  dpcos_h_2 : DiffPair_float_0,  dpsin_h_2 : DiffPair_float_0) -> DiffPair_float_0
{
    var max_y_3 : f32 = 1.0578552484512329f * dptarget_white_1.primal_0;
    var _S2342 : f32 = dptarget_white_1.differential_0 * 1.0578552484512329f;
    var _S2343 : DiffPair_float_0 = DiffPair_float_0( max_y_3 - dpy_11.primal_0, _S2342 - dpy_11.differential_0 );
    var _S2344 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S2345 : DiffPair_float_0 = _d_max_1(_S2343, _S2344);
    var max_chroma_4 : f32;
    var s_diff_max_chroma_0 : f32;
    if(((_S2345.primal_0) < 0.0010000000474974513f))
    {
        var _S2346 : DiffPair_float_0 = DiffPair_float_0( dptarget_white_1.primal_0, dptarget_white_1.differential_0 );
        var _S2347 : DiffPair_float_0 = DiffPair_float_0( 0.99900001287460327f * max_y_3, _S2342 * 0.99900001287460327f );
        var _S2348 : DiffPair_float_0 = DiffPair_float_0( dpcos_h_2.primal_0, dpcos_h_2.differential_0 );
        var _S2349 : DiffPair_float_0 = DiffPair_float_0( dpsin_h_2.primal_0, dpsin_h_2.differential_0 );
        var _S2350 : DiffPair_float_0 = s_fwd_clip_chroma_white_raw_0(coeffs_9, _S2346, _S2347, _S2348, _S2349);
        if(((_S2350.primal_0) >= 3.4028234663852886e+38f))
        {
            var _S2351 : DiffPair_float_0 = DiffPair_float_0( 3.4028234663852886e+38f, 0.0f );
            return _S2351;
        }
        var _S2352 : f32 = 0.0010000000474974513f * max_y_3;
        var _S2353 : f32 = _S2345.primal_0 / _S2352;
        var _S2354 : f32 = (_S2345.differential_0 * _S2352 - _S2345.primal_0 * (_S2342 * 0.0010000000474974513f)) / (_S2352 * _S2352) * _S2350.primal_0 + _S2350.differential_0 * _S2353;
        max_chroma_4 = _S2353 * _S2350.primal_0;
        s_diff_max_chroma_0 = _S2354;
    }
    else
    {
        var _S2355 : DiffPair_float_0 = DiffPair_float_0( dptarget_white_1.primal_0, dptarget_white_1.differential_0 );
        var _S2356 : DiffPair_float_0 = DiffPair_float_0( dpy_11.primal_0, dpy_11.differential_0 );
        var _S2357 : DiffPair_float_0 = DiffPair_float_0( dpcos_h_2.primal_0, dpcos_h_2.differential_0 );
        var _S2358 : DiffPair_float_0 = DiffPair_float_0( dpsin_h_2.primal_0, dpsin_h_2.differential_0 );
        var _S2359 : DiffPair_float_0 = s_fwd_clip_chroma_white_raw_0(coeffs_9, _S2355, _S2356, _S2357, _S2358);
        max_chroma_4 = _S2359.primal_0;
        s_diff_max_chroma_0 = _S2359.differential_0;
    }
    if((max_chroma_4 >= 0.0f))
    {
    }
    else
    {
        max_chroma_4 = 3.4028234663852886e+38f;
        s_diff_max_chroma_0 = 0.0f;
    }
    var _S2360 : DiffPair_float_0 = DiffPair_float_0( max_chroma_4, s_diff_max_chroma_0 );
    return _S2360;
}

fn clip_chroma_black_0( coeffs_10 : vec3<f32>,  cos_h_5 : f32,  sin_h_5 : f32) -> f32
{
    var denominator_7 : f32 = chroma_denominator_0(coeffs_10, cos_h_5, sin_h_5);
    if(denominator_7 == 0.0f)
    {
        return 3.4028234663852886e+38f;
    }
    var max_chroma_5 : f32 = -0.42750686407089233f * (coeffs_10.x + 0.85649234056472778f * coeffs_10.y + 0.55499595403671265f * coeffs_10.z) / denominator_7;
    var _S2361 : f32;
    if((max_chroma_5 >= 0.0f))
    {
        _S2361 = max_chroma_5;
    }
    else
    {
        _S2361 = 3.4028234663852886e+38f;
    }
    return _S2361;
}

fn s_fwd_clip_chroma_black_0( coeffs_11 : vec3<f32>,  dpcos_h_3 : DiffPair_float_0,  dpsin_h_3 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S2362 : DiffPair_float_0 = DiffPair_float_0( dpcos_h_3.primal_0, dpcos_h_3.differential_0 );
    var _S2363 : DiffPair_float_0 = DiffPair_float_0( dpsin_h_3.primal_0, dpsin_h_3.differential_0 );
    var _S2364 : DiffPair_float_0 = s_fwd_chroma_denominator_0(coeffs_11, _S2362, _S2363);
    if((_S2364.primal_0) == 0.0f)
    {
        var _S2365 : DiffPair_float_0 = DiffPair_float_0( 3.4028234663852886e+38f, 0.0f );
        return _S2365;
    }
    var numerator_5 : f32 = -0.42750686407089233f * (coeffs_11.x + 0.85649234056472778f * coeffs_11.y + 0.55499595403671265f * coeffs_11.z);
    var max_chroma_6 : f32 = numerator_5 / _S2364.primal_0;
    var s_diff_max_chroma_1 : f32 = (0.0f - numerator_5 * _S2364.differential_0) / (_S2364.primal_0 * _S2364.primal_0);
    var _S2366 : f32;
    var _S2367 : f32;
    if((max_chroma_6 >= 0.0f))
    {
        _S2366 = max_chroma_6;
        _S2367 = s_diff_max_chroma_1;
    }
    else
    {
        _S2366 = 3.4028234663852886e+38f;
        _S2367 = 0.0f;
    }
    var _S2368 : DiffPair_float_0 = DiffPair_float_0( _S2366, _S2367 );
    return _S2368;
}

fn ych_max_chroma_0( from_lms_7 : mat3x3<f32>,  target_white_2 : f32,  y_16 : f32,  cos_h_6 : f32,  sin_h_6 : f32) -> f32
{
    return min(min(min(clip_chroma_black_0(from_lms_7[i32(0)], cos_h_6, sin_h_6), clip_chroma_black_0(from_lms_7[i32(1)], cos_h_6, sin_h_6)), clip_chroma_black_0(from_lms_7[i32(2)], cos_h_6, sin_h_6)), min(min(clip_chroma_white_0(from_lms_7[i32(0)], target_white_2, y_16, cos_h_6, sin_h_6), clip_chroma_white_0(from_lms_7[i32(1)], target_white_2, y_16, cos_h_6, sin_h_6)), clip_chroma_white_0(from_lms_7[i32(2)], target_white_2, y_16, cos_h_6, sin_h_6)));
}

fn s_fwd_ych_max_chroma_0( from_lms_8 : mat3x3<f32>,  dptarget_white_2 : DiffPair_float_0,  dpy_12 : DiffPair_float_0,  dpcos_h_4 : DiffPair_float_0,  dpsin_h_4 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S2369 : DiffPair_float_0 = DiffPair_float_0( dptarget_white_2.primal_0, dptarget_white_2.differential_0 );
    var _S2370 : DiffPair_float_0 = DiffPair_float_0( dpy_12.primal_0, dpy_12.differential_0 );
    var _S2371 : DiffPair_float_0 = DiffPair_float_0( dpcos_h_4.primal_0, dpcos_h_4.differential_0 );
    var _S2372 : DiffPair_float_0 = DiffPair_float_0( dpsin_h_4.primal_0, dpsin_h_4.differential_0 );
    var _S2373 : DiffPair_float_0 = s_fwd_clip_chroma_white_0(from_lms_8[i32(0)], _S2369, _S2370, _S2371, _S2372);
    var _S2374 : DiffPair_float_0 = s_fwd_clip_chroma_white_0(from_lms_8[i32(1)], _S2369, _S2370, _S2371, _S2372);
    var _S2375 : DiffPair_float_0 = DiffPair_float_0( _S2373.primal_0, _S2373.differential_0 );
    var _S2376 : DiffPair_float_0 = DiffPair_float_0( _S2374.primal_0, _S2374.differential_0 );
    var _S2377 : DiffPair_float_0 = _d_min_1(_S2375, _S2376);
    var _S2378 : DiffPair_float_0 = s_fwd_clip_chroma_white_0(from_lms_8[i32(2)], _S2369, _S2370, _S2371, _S2372);
    var _S2379 : DiffPair_float_0 = DiffPair_float_0( _S2377.primal_0, _S2377.differential_0 );
    var _S2380 : DiffPair_float_0 = DiffPair_float_0( _S2378.primal_0, _S2378.differential_0 );
    var _S2381 : DiffPair_float_0 = _d_min_1(_S2379, _S2380);
    var _S2382 : DiffPair_float_0 = s_fwd_clip_chroma_black_0(from_lms_8[i32(0)], _S2371, _S2372);
    var _S2383 : DiffPair_float_0 = s_fwd_clip_chroma_black_0(from_lms_8[i32(1)], _S2371, _S2372);
    var _S2384 : DiffPair_float_0 = DiffPair_float_0( _S2382.primal_0, _S2382.differential_0 );
    var _S2385 : DiffPair_float_0 = DiffPair_float_0( _S2383.primal_0, _S2383.differential_0 );
    var _S2386 : DiffPair_float_0 = _d_min_1(_S2384, _S2385);
    var _S2387 : DiffPair_float_0 = s_fwd_clip_chroma_black_0(from_lms_8[i32(2)], _S2371, _S2372);
    var _S2388 : DiffPair_float_0 = DiffPair_float_0( _S2386.primal_0, _S2386.differential_0 );
    var _S2389 : DiffPair_float_0 = DiffPair_float_0( _S2387.primal_0, _S2387.differential_0 );
    var _S2390 : DiffPair_float_0 = _d_min_1(_S2388, _S2389);
    var _S2391 : DiffPair_float_0 = DiffPair_float_0( _S2390.primal_0, _S2390.differential_0 );
    var _S2392 : DiffPair_float_0 = DiffPair_float_0( _S2381.primal_0, _S2381.differential_0 );
    var _S2393 : DiffPair_float_0 = _d_min_1(_S2391, _S2392);
    var _S2394 : DiffPair_float_0 = DiffPair_float_0( _S2393.primal_0, _S2393.differential_0 );
    return _S2394;
}

fn gamut_check_rgb_0( ych_4 : vec4<f32>,  to_lms_6 : mat3x3<f32>,  from_lms_9 : mat3x3<f32>,  display_black_0 : f32,  display_white_0 : f32) -> vec3<f32>
{
    var brightened_1 : vec3<f32> = ych_to_rgb_0(ych_4, from_lms_9);
    var y_17 : f32 = clamp((ych_4.x + rgb_to_ych_0(brightened_1 + vec3<f32>(max(- min(min(brightened_1.x, brightened_1.y), brightened_1.z), 0.0f)), to_lms_6).x) / 2.0f, 1.0578552484512329f * display_black_0, 1.0578552484512329f * display_white_0);
    var cos_h_7 : f32 = ych_4.z;
    var sin_h_7 : f32 = ych_4.w;
    return clamp(ych_to_rgb_0(vec4<f32>(y_17, min(ych_4.y, ych_max_chroma_0(from_lms_9, display_white_0, y_17, cos_h_7, sin_h_7)), cos_h_7, sin_h_7), from_lms_9), vec3<f32>(0.0f), vec3<f32>(display_white_0));
}

fn s_fwd_gamut_check_rgb_0( dpych_3 : DiffPair_vectorx3Cfloatx2C4x3E_0,  to_lms_7 : mat3x3<f32>,  from_lms_10 : mat3x3<f32>,  dpdisplay_black_0 : DiffPair_float_0,  dpdisplay_white_0 : DiffPair_float_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S2395 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( dpych_3.primal_0, dpych_3.differential_0 );
    var _S2396 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_ych_to_rgb_0(_S2395, from_lms_10);
    var _S2397 : DiffPair_float_0 = DiffPair_float_0( _S2396.primal_0.x, _S2396.differential_0.x );
    var _S2398 : DiffPair_float_0 = DiffPair_float_0( _S2396.primal_0.y, _S2396.differential_0.y );
    var _S2399 : DiffPair_float_0 = _d_min_1(_S2397, _S2398);
    var _S2400 : DiffPair_float_0 = DiffPair_float_0( _S2399.primal_0, _S2399.differential_0 );
    var _S2401 : DiffPair_float_0 = DiffPair_float_0( _S2396.primal_0.z, _S2396.differential_0.z );
    var _S2402 : DiffPair_float_0 = _d_min_1(_S2400, _S2401);
    var _S2403 : DiffPair_float_0 = DiffPair_float_0( - _S2402.primal_0, - _S2402.differential_0 );
    var _S2404 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S2405 : DiffPair_float_0 = _d_max_1(_S2403, _S2404);
    var _S2406 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2396.primal_0 + vec3<f32>(_S2405.primal_0), _S2396.differential_0 + vec3<f32>(_S2405.differential_0) );
    var _S2407 : DiffPair_vectorx3Cfloatx2C4x3E_0 = s_fwd_rgb_to_ych_0(_S2406, to_lms_7);
    var _S2408 : DiffPair_float_0 = DiffPair_float_0( (dpych_3.primal_0.x + _S2407.primal_0.x) / 2.0f, (dpych_3.differential_0.x + _S2407.differential_0.x) * 0.5f );
    var _S2409 : DiffPair_float_0 = DiffPair_float_0( 1.0578552484512329f * dpdisplay_black_0.primal_0, dpdisplay_black_0.differential_0 * 1.0578552484512329f );
    var _S2410 : DiffPair_float_0 = DiffPair_float_0( 1.0578552484512329f * dpdisplay_white_0.primal_0, dpdisplay_white_0.differential_0 * 1.0578552484512329f );
    var _S2411 : DiffPair_float_0 = _d_clamp_1(_S2408, _S2409, _S2410);
    var cos_h_8 : f32 = dpych_3.primal_0.z;
    var s_diff_cos_h_0 : f32 = dpych_3.differential_0.z;
    var sin_h_8 : f32 = dpych_3.primal_0.w;
    var s_diff_sin_h_0 : f32 = dpych_3.differential_0.w;
    var _S2412 : DiffPair_float_0 = DiffPair_float_0( dpdisplay_white_0.primal_0, dpdisplay_white_0.differential_0 );
    var _S2413 : DiffPair_float_0 = DiffPair_float_0( _S2411.primal_0, _S2411.differential_0 );
    var _S2414 : DiffPair_float_0 = DiffPair_float_0( cos_h_8, s_diff_cos_h_0 );
    var _S2415 : DiffPair_float_0 = DiffPair_float_0( sin_h_8, s_diff_sin_h_0 );
    var _S2416 : DiffPair_float_0 = s_fwd_ych_max_chroma_0(from_lms_10, _S2412, _S2413, _S2414, _S2415);
    var _S2417 : DiffPair_float_0 = DiffPair_float_0( dpych_3.primal_0.y, dpych_3.differential_0.y );
    var _S2418 : DiffPair_float_0 = DiffPair_float_0( _S2416.primal_0, _S2416.differential_0 );
    var _S2419 : DiffPair_float_0 = _d_min_1(_S2417, _S2418);
    var _S2420 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( vec4<f32>(_S2411.primal_0, _S2419.primal_0, cos_h_8, sin_h_8), vec4<f32>(_S2411.differential_0, _S2419.differential_0, s_diff_cos_h_0, s_diff_sin_h_0) );
    var _S2421 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_ych_to_rgb_0(_S2420, from_lms_10);
    var _S2422 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2421.primal_0, _S2421.differential_0 );
    var _S2423 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.0f), vec3<f32>(0.0f) );
    var _S2424 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(dpdisplay_white_0.primal_0), vec3<f32>(dpdisplay_white_0.differential_0) );
    var _S2425 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_clamp_vector_1(_S2422, _S2423, _S2424);
    var _S2426 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2425.primal_0, _S2425.differential_0 );
    return _S2426;
}

fn gamut_mapping_0( final_1 : vec4<f32>,  original_1 : vec4<f32>,  c_11 : Curve_0,  saturation_1 : f32,  m_9 : Modes_0) -> vec3<f32>
{
    var _S2427 : f32 = c_11.display_0.y;
    var _S2428 : f32 = c_11.display_0.z;
    return ((((((gamut_check_rgb_0(gamut_check_yrg_0(desaturate_v4_0(original_1, vec4<f32>(clamp(final_1.x, 1.0578552484512329f * _S2427, 1.0578552484512329f * _S2428), final_1.y, original_1.z, original_1.w), saturation_1)), m_9.export_to_lms_1, m_9.export_from_lms_1, _S2427, _S2428)) * (m_9.export_to_lms_1)))) * (m_9.from_lms_1)));
}

fn s_fwd_gamut_mapping_0( dpfinal_1 : DiffPair_vectorx3Cfloatx2C4x3E_0,  dporiginal_1 : DiffPair_vectorx3Cfloatx2C4x3E_0,  dpc_7 : DiffPair_Curve_0,  dpsaturation_1 : DiffPair_float_0,  m_10 : Modes_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S2429 : f32 = dpc_7.primal_0.display_0.y;
    var _S2430 : f32 = dpc_7.differential_0.display_0.y;
    var _S2431 : f32 = dpc_7.primal_0.display_0.z;
    var _S2432 : f32 = dpc_7.differential_0.display_0.z;
    var _S2433 : DiffPair_float_0 = DiffPair_float_0( dpfinal_1.primal_0.x, dpfinal_1.differential_0.x );
    var _S2434 : DiffPair_float_0 = DiffPair_float_0( 1.0578552484512329f * _S2429, _S2430 * 1.0578552484512329f );
    var _S2435 : DiffPair_float_0 = DiffPair_float_0( 1.0578552484512329f * _S2431, _S2432 * 1.0578552484512329f );
    var _S2436 : DiffPair_float_0 = _d_clamp_1(_S2433, _S2434, _S2435);
    var _S2437 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( dporiginal_1.primal_0, dporiginal_1.differential_0 );
    var _S2438 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( vec4<f32>(_S2436.primal_0, dpfinal_1.primal_0.y, dporiginal_1.primal_0.z, dporiginal_1.primal_0.w), vec4<f32>(_S2436.differential_0, dpfinal_1.differential_0.y, dporiginal_1.differential_0.z, dporiginal_1.differential_0.w) );
    var _S2439 : DiffPair_float_0 = DiffPair_float_0( dpsaturation_1.primal_0, dpsaturation_1.differential_0 );
    var _S2440 : DiffPair_vectorx3Cfloatx2C4x3E_0 = s_fwd_desaturate_v4_0(_S2437, _S2438, _S2439);
    var _S2441 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( _S2440.primal_0, _S2440.differential_0 );
    var _S2442 : DiffPair_vectorx3Cfloatx2C4x3E_0 = s_fwd_gamut_check_yrg_0(_S2441);
    var _S2443 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( _S2442.primal_0, _S2442.differential_0 );
    var _S2444 : DiffPair_float_0 = DiffPair_float_0( _S2429, _S2430 );
    var _S2445 : DiffPair_float_0 = DiffPair_float_0( _S2431, _S2432 );
    var _S2446 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_gamut_check_rgb_0(_S2443, m_10.export_to_lms_1, m_10.export_from_lms_1, _S2444, _S2445);
    var _S2447 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S2448 : vec3<f32> = (((_S2446.primal_0) * (m_10.export_to_lms_1)));
    var _S2449 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( (((_S2448) * (m_10.from_lms_1))), (((_S2448) * (_S2447))) + ((((((_S2446.primal_0) * (_S2447))) + (((_S2446.differential_0) * (m_10.export_to_lms_1)))) * (m_10.from_lms_1))) );
    return _S2449;
}

fn filmic_v7_0( rgb_4 : vec3<f32>,  c_12 : Curve_0,  m_11 : Modes_0) -> vec3<f32>
{
    var s_1 : f32 = c_12.tone_0.z;
    var original_2 : vec4<f32> = rgb_to_ych_0(rgb_4, m_11.to_lms_1);
    var _S2450 : vec4<f32> = rgb_to_ych_0(vec3<f32>((0.5f - s_1)) * rgb_tone_mapping_0(rgb_4, c_12, m_11) + vec3<f32>((0.5f + s_1)) * norm_tone_mapping_0(rgb_4, c_12, u32(1), m_11), m_11.to_lms_1);
    var final_2 : vec4<f32> = _S2450;
    final_2[i32(1)] = min(original_2.y, _S2450.y);
    return gamut_mapping_0(final_2, original_2, c_12, 0.0f, m_11);
}

fn s_fwd_filmic_v7_0( dprgb_3 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpc_8 : DiffPair_Curve_0,  m_12 : Modes_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S2451 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( dprgb_3.primal_0, dprgb_3.differential_0 );
    var _S2452 : DiffPair_Curve_0 = DiffPair_Curve_0( dpc_8.primal_0, dpc_8.differential_0 );
    var _S2453 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_rgb_tone_mapping_0(_S2451, _S2452, m_12);
    var _S2454 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_norm_tone_mapping_0(_S2451, _S2452, u32(1), m_12);
    var s_2 : f32 = dpc_8.primal_0.tone_0.z;
    var s_diff_s_0 : f32 = dpc_8.differential_0.tone_0.z;
    var _S2455 : vec3<f32> = vec3<f32>((0.5f - s_2));
    var _S2456 : vec3<f32> = vec3<f32>((0.5f + s_2));
    var _S2457 : DiffPair_vectorx3Cfloatx2C4x3E_0 = s_fwd_rgb_to_ych_0(_S2451, m_12.to_lms_1);
    var _S2458 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2455 * _S2453.primal_0 + _S2456 * _S2454.primal_0, vec3<f32>((0.0f - s_diff_s_0)) * _S2453.primal_0 + _S2453.differential_0 * _S2455 + (vec3<f32>(s_diff_s_0) * _S2454.primal_0 + _S2454.differential_0 * _S2456) );
    var _S2459 : DiffPair_vectorx3Cfloatx2C4x3E_0 = s_fwd_rgb_to_ych_0(_S2458, m_12.to_lms_1);
    var _S2460 : DiffPair_float_0 = DiffPair_float_0( _S2457.primal_0.y, _S2457.differential_0.y );
    var _S2461 : DiffPair_float_0 = DiffPair_float_0( _S2459.primal_0.y, _S2459.differential_0.y );
    var _S2462 : DiffPair_float_0 = _d_min_1(_S2460, _S2461);
    var _S2463 : vec4<f32> = _S2459.primal_0;
    _S2463[i32(1)] = _S2462.primal_0;
    var _S2464 : vec4<f32> = _S2459.differential_0;
    _S2464[i32(1)] = _S2462.differential_0;
    var _S2465 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( _S2463, _S2464 );
    var _S2466 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( _S2457.primal_0, _S2457.differential_0 );
    var _S2467 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S2468 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_gamut_mapping_0(_S2465, _S2466, _S2452, _S2467, m_12);
    var _S2469 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2468.primal_0, _S2468.differential_0 );
    return _S2469;
}

fn filmic_v6_0( rgb_5 : vec3<f32>,  c_13 : Curve_0,  m_13 : Modes_0) -> vec3<f32>
{
    var original_3 : vec4<f32> = rgb_to_ych_0(rgb_5, m_13.to_lms_1);
    if((m_13.norm_0) == u32(0))
    {
        var _S2470 : vec4<f32> = rgb_to_ych_0(rgb_tone_mapping_0(rgb_5, c_13, m_13), m_13.to_lms_1);
        var final_3 : vec4<f32> = _S2470;
        final_3[i32(1)] = min(original_3.y, _S2470.y);
        return gamut_mapping_0(final_3, original_3, c_13, c_13.tone_0.z, m_13);
    }
    return gamut_mapping_0(rgb_to_ych_0(norm_tone_mapping_0(rgb_5, c_13, m_13.norm_0, m_13), m_13.to_lms_1), original_3, c_13, c_13.tone_0.z, m_13);
}

fn s_fwd_filmic_v6_0( dprgb_4 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpc_9 : DiffPair_Curve_0,  m_14 : Modes_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S2471 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( dprgb_4.primal_0, dprgb_4.differential_0 );
    var _S2472 : DiffPair_vectorx3Cfloatx2C4x3E_0 = s_fwd_rgb_to_ych_0(_S2471, m_14.to_lms_1);
    if((m_14.norm_0) == u32(0))
    {
        var _S2473 : DiffPair_Curve_0 = DiffPair_Curve_0( dpc_9.primal_0, dpc_9.differential_0 );
        var _S2474 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_rgb_tone_mapping_0(_S2471, _S2473, m_14);
        var _S2475 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2474.primal_0, _S2474.differential_0 );
        var _S2476 : DiffPair_vectorx3Cfloatx2C4x3E_0 = s_fwd_rgb_to_ych_0(_S2475, m_14.to_lms_1);
        var _S2477 : DiffPair_float_0 = DiffPair_float_0( _S2472.primal_0.y, _S2472.differential_0.y );
        var _S2478 : DiffPair_float_0 = DiffPair_float_0( _S2476.primal_0.y, _S2476.differential_0.y );
        var _S2479 : DiffPair_float_0 = _d_min_1(_S2477, _S2478);
        var _S2480 : vec4<f32> = _S2476.primal_0;
        _S2480[i32(1)] = _S2479.primal_0;
        var _S2481 : vec4<f32> = _S2476.differential_0;
        _S2481[i32(1)] = _S2479.differential_0;
        var _S2482 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( _S2480, _S2481 );
        var _S2483 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( _S2472.primal_0, _S2472.differential_0 );
        var _S2484 : DiffPair_float_0 = DiffPair_float_0( dpc_9.primal_0.tone_0.z, dpc_9.differential_0.tone_0.z );
        var _S2485 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_gamut_mapping_0(_S2482, _S2483, _S2473, _S2484, m_14);
        var _S2486 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2485.primal_0, _S2485.differential_0 );
        return _S2486;
    }
    var _S2487 : DiffPair_Curve_0 = DiffPair_Curve_0( dpc_9.primal_0, dpc_9.differential_0 );
    var _S2488 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_norm_tone_mapping_0(_S2471, _S2487, m_14.norm_0, m_14);
    var _S2489 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2488.primal_0, _S2488.differential_0 );
    var _S2490 : DiffPair_vectorx3Cfloatx2C4x3E_0 = s_fwd_rgb_to_ych_0(_S2489, m_14.to_lms_1);
    var _S2491 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( _S2490.primal_0, _S2490.differential_0 );
    var _S2492 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( _S2472.primal_0, _S2472.differential_0 );
    var _S2493 : DiffPair_float_0 = DiffPair_float_0( dpc_9.primal_0.tone_0.z, dpc_9.differential_0.tone_0.z );
    var _S2494 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_gamut_mapping_0(_S2491, _S2492, _S2487, _S2493, m_14);
    var _S2495 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2494.primal_0, _S2494.differential_0 );
    return _S2495;
}

fn log_v1_0( x_30 : f32,  c_14 : Curve_0,  white_5 : bool) -> f32
{
    if(white_5)
    {
        return 1.0f;
    }
    return clamp(log_encoding_0(x_30, c_14), 1.52587890625e-05f, 1.0f);
}

fn s_fwd_log_v1_0( dpx_27 : DiffPair_float_0,  dpc_10 : DiffPair_Curve_0,  white_6 : bool) -> DiffPair_float_0
{
    if(white_6)
    {
        var _S2496 : DiffPair_float_0 = DiffPair_float_0( 1.0f, 0.0f );
        return _S2496;
    }
    var _S2497 : DiffPair_float_0 = DiffPair_float_0( dpx_27.primal_0, dpx_27.differential_0 );
    var _S2498 : DiffPair_Curve_0 = DiffPair_Curve_0( dpc_10.primal_0, dpc_10.differential_0 );
    var _S2499 : DiffPair_float_0 = s_fwd_log_encoding_0(_S2497, _S2498);
    var _S2500 : DiffPair_float_0 = DiffPair_float_0( _S2499.primal_0, _S2499.differential_0 );
    var _S2501 : DiffPair_float_0 = DiffPair_float_0( 1.52587890625e-05f, 0.0f );
    var _S2502 : DiffPair_float_0 = DiffPair_float_0( 1.0f, 0.0f );
    var _S2503 : DiffPair_float_0 = _d_clamp_1(_S2500, _S2501, _S2502);
    var _S2504 : DiffPair_float_0 = DiffPair_float_0( _S2503.primal_0, _S2503.differential_0 );
    return _S2504;
}

fn away_from_zero_0( v_1 : f32,  floor_1 : f32) -> f32
{
    if(((abs(v_1)) < floor_1))
    {
        var _S2505 : f32;
        if((v_1 < 0.0f))
        {
            _S2505 = - floor_1;
        }
        else
        {
            _S2505 = floor_1;
        }
        return _S2505;
    }
    return v_1;
}

fn s_fwd_away_from_zero_0( dpv_0 : DiffPair_float_0,  floor_2 : f32) -> DiffPair_float_0
{
    var _S2506 : DiffPair_float_0 = DiffPair_float_0( dpv_0.primal_0, dpv_0.differential_0 );
    if(((_d_abs_1(_S2506).primal_0) < floor_2))
    {
        var _S2507 : f32;
        if(((dpv_0.primal_0) < 0.0f))
        {
            _S2507 = - floor_2;
        }
        else
        {
            _S2507 = floor_2;
        }
        var _S2508 : DiffPair_float_0 = DiffPair_float_0( _S2507, 0.0f );
        return _S2508;
    }
    return _S2506;
}

fn desaturate_v1_0( x_31 : f32,  c_15 : Curve_0) -> f32
{
    var _S2509 : f32 = 1.0f - x_31;
    return 1.0f - clamp((exp(-0.5f * x_31 * x_31 / max(c_15.tone_0.w, 1.17549435082228751e-38f)) + exp(-0.5f * _S2509 * _S2509 / max(c_15.display_0.x, 1.17549435082228751e-38f))) / away_from_zero_0(c_15.tone_0.z, 1.00000000317107685e-30f), 0.0f, 1.0f);
}

fn s_fwd_desaturate_v1_0( dpx_28 : DiffPair_float_0,  dpc_11 : DiffPair_Curve_0) -> DiffPair_float_0
{
    var _S2510 : f32 = -0.5f * dpx_28.primal_0;
    var _S2511 : f32 = _S2510 * dpx_28.primal_0;
    var _S2512 : DiffPair_float_0 = DiffPair_float_0( dpc_11.primal_0.tone_0.w, dpc_11.differential_0.tone_0.w );
    var _S2513 : DiffPair_float_0 = DiffPair_float_0( 1.17549435082228751e-38f, 0.0f );
    var _S2514 : DiffPair_float_0 = _d_max_1(_S2512, _S2513);
    var _S2515 : DiffPair_float_0 = DiffPair_float_0( _S2511 / _S2514.primal_0, ((dpx_28.differential_0 * -0.5f * dpx_28.primal_0 + dpx_28.differential_0 * _S2510) * _S2514.primal_0 - _S2511 * _S2514.differential_0) / (_S2514.primal_0 * _S2514.primal_0) );
    var _S2516 : DiffPair_float_0 = _d_exp_1(_S2515);
    var _S2517 : f32 = 1.0f - dpx_28.primal_0;
    var _S2518 : f32 = 0.0f - dpx_28.differential_0;
    var _S2519 : f32 = -0.5f * _S2517;
    var _S2520 : f32 = _S2519 * _S2517;
    var _S2521 : DiffPair_float_0 = DiffPair_float_0( dpc_11.primal_0.display_0.x, dpc_11.differential_0.display_0.x );
    var _S2522 : DiffPair_float_0 = _d_max_1(_S2521, _S2513);
    var _S2523 : DiffPair_float_0 = DiffPair_float_0( _S2520 / _S2522.primal_0, ((_S2518 * -0.5f * _S2517 + _S2518 * _S2519) * _S2522.primal_0 - _S2520 * _S2522.differential_0) / (_S2522.primal_0 * _S2522.primal_0) );
    var _S2524 : DiffPair_float_0 = _d_exp_1(_S2523);
    var _S2525 : f32 = _S2516.primal_0 + _S2524.primal_0;
    var _S2526 : DiffPair_float_0 = DiffPair_float_0( dpc_11.primal_0.tone_0.z, dpc_11.differential_0.tone_0.z );
    var _S2527 : DiffPair_float_0 = s_fwd_away_from_zero_0(_S2526, 1.00000000317107685e-30f);
    var _S2528 : DiffPair_float_0 = DiffPair_float_0( _S2525 / _S2527.primal_0, ((_S2516.differential_0 + _S2524.differential_0) * _S2527.primal_0 - _S2525 * _S2527.differential_0) / (_S2527.primal_0 * _S2527.primal_0) );
    var _S2529 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S2530 : DiffPair_float_0 = DiffPair_float_0( 1.0f, 0.0f );
    var _S2531 : DiffPair_float_0 = _d_clamp_1(_S2528, _S2529, _S2530);
    var _S2532 : DiffPair_float_0 = DiffPair_float_0( 1.0f - _S2531.primal_0, 0.0f - _S2531.differential_0 );
    return _S2532;
}

fn split_v1_0( rgb_6 : vec3<f32>,  c_16 : Curve_0,  m_15 : Modes_0) -> vec3<f32>
{
    var t_4 : vec3<f32> = vec3<f32>(log_v1_0(max(rgb_6.x, 1.52587890625e-05f), c_16, m_15.white_0), log_v1_0(max(rgb_6.y, 1.52587890625e-05f), c_16, m_15.white_0), log_v1_0(max(rgb_6.z, 1.52587890625e-05f), c_16, m_15.white_0));
    var lum_0 : f32 = dot(m_15.luminance_1, t_4);
    var _S2533 : vec3<f32> = vec3<f32>(lum_0);
    var mapped_2 : vec3<f32> = _S2533 + vec3<f32>(desaturate_v1_0(lum_0, c_16)) * (t_4 - _S2533);
    var out_1 : vec3<f32> = clamp(vec3<f32>(spline_0(mapped_2.x, c_16, m_15.toe_1, m_15.shoulder_1), spline_0(mapped_2.y, c_16, m_15.toe_1, m_15.shoulder_1), spline_0(mapped_2.z, c_16, m_15.toe_1, m_15.shoulder_1)), vec3<f32>(0.0f), vec3<f32>(1.0f));
    var _S2534 : f32 = c_16.tone_0.y;
    return vec3<f32>(pow_dt_0(out_1.x, _S2534), pow_dt_0(out_1.y, _S2534), pow_dt_0(out_1.z, _S2534));
}

fn s_fwd_split_v1_0( dprgb_5 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpc_12 : DiffPair_Curve_0,  m_16 : Modes_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S2535 : DiffPair_float_0 = DiffPair_float_0( dprgb_5.primal_0.x, dprgb_5.differential_0.x );
    var _S2536 : DiffPair_float_0 = DiffPair_float_0( 1.52587890625e-05f, 0.0f );
    var _S2537 : DiffPair_float_0 = _d_max_1(_S2535, _S2536);
    var _S2538 : DiffPair_float_0 = DiffPair_float_0( _S2537.primal_0, _S2537.differential_0 );
    var _S2539 : DiffPair_Curve_0 = DiffPair_Curve_0( dpc_12.primal_0, dpc_12.differential_0 );
    var _S2540 : DiffPair_float_0 = s_fwd_log_v1_0(_S2538, _S2539, m_16.white_0);
    var _S2541 : DiffPair_float_0 = DiffPair_float_0( dprgb_5.primal_0.y, dprgb_5.differential_0.y );
    var _S2542 : DiffPair_float_0 = _d_max_1(_S2541, _S2536);
    var _S2543 : DiffPair_float_0 = DiffPair_float_0( _S2542.primal_0, _S2542.differential_0 );
    var _S2544 : DiffPair_float_0 = s_fwd_log_v1_0(_S2543, _S2539, m_16.white_0);
    var _S2545 : DiffPair_float_0 = DiffPair_float_0( dprgb_5.primal_0.z, dprgb_5.differential_0.z );
    var _S2546 : DiffPair_float_0 = _d_max_1(_S2545, _S2536);
    var _S2547 : DiffPair_float_0 = DiffPair_float_0( _S2546.primal_0, _S2546.differential_0 );
    var _S2548 : DiffPair_float_0 = s_fwd_log_v1_0(_S2547, _S2539, m_16.white_0);
    var t_5 : vec3<f32> = vec3<f32>(_S2540.primal_0, _S2544.primal_0, _S2548.primal_0);
    var s_diff_t_0 : vec3<f32> = vec3<f32>(_S2540.differential_0, _S2544.differential_0, _S2548.differential_0);
    var _S2549 : vec3<f32> = vec3<f32>(0.0f);
    var _S2550 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( m_16.luminance_1, _S2549 );
    var _S2551 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( t_5, s_diff_t_0 );
    var _S2552 : DiffPair_float_0 = _d_dot_1(_S2550, _S2551);
    var _S2553 : DiffPair_float_0 = DiffPair_float_0( _S2552.primal_0, _S2552.differential_0 );
    var _S2554 : DiffPair_float_0 = s_fwd_desaturate_v1_0(_S2553, _S2539);
    var _S2555 : vec3<f32> = vec3<f32>(_S2552.primal_0);
    var _S2556 : vec3<f32> = t_5 - _S2555;
    var _S2557 : vec3<f32> = vec3<f32>(_S2552.differential_0);
    var _S2558 : vec3<f32> = vec3<f32>(_S2554.primal_0);
    var mapped_3 : vec3<f32> = _S2555 + _S2558 * _S2556;
    var s_diff_mapped_0 : vec3<f32> = _S2557 + (vec3<f32>(_S2554.differential_0) * _S2556 + (s_diff_t_0 - _S2557) * _S2558);
    var _S2559 : DiffPair_float_0 = DiffPair_float_0( mapped_3.x, s_diff_mapped_0.x );
    var _S2560 : DiffPair_float_0 = s_fwd_spline_0(_S2559, _S2539, m_16.toe_1, m_16.shoulder_1);
    var _S2561 : DiffPair_float_0 = DiffPair_float_0( mapped_3.y, s_diff_mapped_0.y );
    var _S2562 : DiffPair_float_0 = s_fwd_spline_0(_S2561, _S2539, m_16.toe_1, m_16.shoulder_1);
    var _S2563 : DiffPair_float_0 = DiffPair_float_0( mapped_3.z, s_diff_mapped_0.z );
    var _S2564 : DiffPair_float_0 = s_fwd_spline_0(_S2563, _S2539, m_16.toe_1, m_16.shoulder_1);
    var _S2565 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S2560.primal_0, _S2562.primal_0, _S2564.primal_0), vec3<f32>(_S2560.differential_0, _S2562.differential_0, _S2564.differential_0) );
    var _S2566 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.0f), _S2549 );
    var _S2567 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(1.0f), _S2549 );
    var _S2568 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_clamp_vector_1(_S2565, _S2566, _S2567);
    var _S2569 : DiffPair_float_0 = DiffPair_float_0( _S2568.primal_0.x, _S2568.differential_0.x );
    var _S2570 : DiffPair_float_0 = DiffPair_float_0( dpc_12.primal_0.tone_0.y, dpc_12.differential_0.tone_0.y );
    var _S2571 : DiffPair_float_0 = pow_dt_forward_0(_S2569, _S2570);
    var _S2572 : DiffPair_float_0 = DiffPair_float_0( _S2568.primal_0.y, _S2568.differential_0.y );
    var _S2573 : DiffPair_float_0 = pow_dt_forward_0(_S2572, _S2570);
    var _S2574 : DiffPair_float_0 = DiffPair_float_0( _S2568.primal_0.z, _S2568.differential_0.z );
    var _S2575 : DiffPair_float_0 = pow_dt_forward_0(_S2574, _S2570);
    var _S2576 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S2571.primal_0, _S2573.primal_0, _S2575.primal_0), vec3<f32>(_S2571.differential_0, _S2573.differential_0, _S2575.differential_0) );
    return _S2576;
}

fn desaturate_v2_0( x_32 : f32,  c_17 : Curve_0) -> f32
{
    var _S2577 : f32 = max(c_17.tone_0.z, 0.0f);
    var sat2_2 : f32 = 0.5f / sqrt(max(_S2577, 1.00000000317107685e-30f));
    var _S2578 : f32 = 1.0f - x_32;
    return _S2577 - (exp(- x_32 * x_32 / max(c_17.tone_0.w, 1.17549435082228751e-38f) * sat2_2) + exp(- _S2578 * _S2578 / max(c_17.display_0.x, 1.17549435082228751e-38f) * sat2_2)) * _S2577;
}

fn s_fwd_desaturate_v2_0( dpx_29 : DiffPair_float_0,  dpc_13 : DiffPair_Curve_0) -> DiffPair_float_0
{
    var _S2579 : DiffPair_float_0 = DiffPair_float_0( dpc_13.primal_0.tone_0.z, dpc_13.differential_0.tone_0.z );
    var _S2580 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S2581 : DiffPair_float_0 = _d_max_1(_S2579, _S2580);
    var _S2582 : DiffPair_float_0 = DiffPair_float_0( _S2581.primal_0, _S2581.differential_0 );
    var _S2583 : DiffPair_float_0 = DiffPair_float_0( 1.00000000317107685e-30f, 0.0f );
    var _S2584 : DiffPair_float_0 = _d_max_1(_S2582, _S2583);
    var _S2585 : DiffPair_float_0 = DiffPair_float_0( _S2584.primal_0, _S2584.differential_0 );
    var _S2586 : DiffPair_float_0 = _d_sqrt_1(_S2585);
    var sat2_3 : f32 = 0.5f / _S2586.primal_0;
    var s_diff_sat2_0 : f32 = (0.0f - 0.5f * _S2586.differential_0) / (_S2586.primal_0 * _S2586.primal_0);
    var _S2587 : f32 = - dpx_29.primal_0;
    var _S2588 : f32 = _S2587 * dpx_29.primal_0;
    var _S2589 : DiffPair_float_0 = DiffPair_float_0( dpc_13.primal_0.tone_0.w, dpc_13.differential_0.tone_0.w );
    var _S2590 : DiffPair_float_0 = DiffPair_float_0( 1.17549435082228751e-38f, 0.0f );
    var _S2591 : DiffPair_float_0 = _d_max_1(_S2589, _S2590);
    var _S2592 : f32 = _S2588 / _S2591.primal_0;
    var _S2593 : DiffPair_float_0 = DiffPair_float_0( _S2592 * sat2_3, ((- dpx_29.differential_0 * dpx_29.primal_0 + dpx_29.differential_0 * _S2587) * _S2591.primal_0 - _S2588 * _S2591.differential_0) / (_S2591.primal_0 * _S2591.primal_0) * sat2_3 + s_diff_sat2_0 * _S2592 );
    var _S2594 : DiffPair_float_0 = _d_exp_1(_S2593);
    var _S2595 : f32 = 1.0f - dpx_29.primal_0;
    var _S2596 : f32 = 0.0f - dpx_29.differential_0;
    var _S2597 : f32 = - _S2595;
    var _S2598 : f32 = _S2597 * _S2595;
    var _S2599 : DiffPair_float_0 = DiffPair_float_0( dpc_13.primal_0.display_0.x, dpc_13.differential_0.display_0.x );
    var _S2600 : DiffPair_float_0 = _d_max_1(_S2599, _S2590);
    var _S2601 : f32 = _S2598 / _S2600.primal_0;
    var _S2602 : DiffPair_float_0 = DiffPair_float_0( _S2601 * sat2_3, ((- _S2596 * _S2595 + _S2596 * _S2597) * _S2600.primal_0 - _S2598 * _S2600.differential_0) / (_S2600.primal_0 * _S2600.primal_0) * sat2_3 + s_diff_sat2_0 * _S2601 );
    var _S2603 : DiffPair_float_0 = _d_exp_1(_S2602);
    var _S2604 : f32 = _S2594.primal_0 + _S2603.primal_0;
    var _S2605 : DiffPair_float_0 = DiffPair_float_0( _S2581.primal_0 - _S2604 * _S2581.primal_0, _S2581.differential_0 - ((_S2594.differential_0 + _S2603.differential_0) * _S2581.primal_0 + _S2581.differential_0 * _S2604) );
    return _S2605;
}

fn split_v2_v3_0( rgb_7 : vec3<f32>,  c_18 : Curve_0,  m_17 : Modes_0) -> vec3<f32>
{
    if(((c_18.tone_0.z) < 0.0f))
    {
        return vec3<f32>(0.0f);
    }
    var t_6 : vec3<f32> = vec3<f32>(log_v2_0(max(rgb_7.x, 1.52587890625e-05f), c_18, m_17.white_0), log_v2_0(max(rgb_7.y, 1.52587890625e-05f), c_18, m_17.white_0), log_v2_0(max(rgb_7.z, 1.52587890625e-05f), c_18, m_17.white_0));
    var lum_1 : f32 = dot(m_17.luminance_1, t_6);
    var _S2606 : vec3<f32> = vec3<f32>(lum_1);
    var mapped_4 : vec3<f32> = _S2606 + vec3<f32>(desaturate_v2_0(lum_1, c_18)) * (t_6 - _S2606);
    var out_2 : vec3<f32> = clamp(vec3<f32>(spline_0(mapped_4.x, c_18, m_17.toe_1, m_17.shoulder_1), spline_0(mapped_4.y, c_18, m_17.toe_1, m_17.shoulder_1), spline_0(mapped_4.z, c_18, m_17.toe_1, m_17.shoulder_1)), vec3<f32>(0.0f), vec3<f32>(1.0f));
    var _S2607 : f32 = c_18.tone_0.y;
    return vec3<f32>(pow_dt_0(out_2.x, _S2607), pow_dt_0(out_2.y, _S2607), pow_dt_0(out_2.z, _S2607));
}

fn s_fwd_split_v2_v3_0( dprgb_6 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpc_14 : DiffPair_Curve_0,  m_18 : Modes_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    if(((dpc_14.primal_0.tone_0.z) < 0.0f))
    {
        var _S2608 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.0f), vec3<f32>(0.0f) );
        return _S2608;
    }
    var _S2609 : DiffPair_float_0 = DiffPair_float_0( dprgb_6.primal_0.x, dprgb_6.differential_0.x );
    var _S2610 : DiffPair_float_0 = DiffPair_float_0( 1.52587890625e-05f, 0.0f );
    var _S2611 : DiffPair_float_0 = _d_max_1(_S2609, _S2610);
    var _S2612 : DiffPair_float_0 = DiffPair_float_0( _S2611.primal_0, _S2611.differential_0 );
    var _S2613 : DiffPair_Curve_0 = DiffPair_Curve_0( dpc_14.primal_0, dpc_14.differential_0 );
    var _S2614 : DiffPair_float_0 = s_fwd_log_v2_0(_S2612, _S2613, m_18.white_0);
    var _S2615 : DiffPair_float_0 = DiffPair_float_0( dprgb_6.primal_0.y, dprgb_6.differential_0.y );
    var _S2616 : DiffPair_float_0 = _d_max_1(_S2615, _S2610);
    var _S2617 : DiffPair_float_0 = DiffPair_float_0( _S2616.primal_0, _S2616.differential_0 );
    var _S2618 : DiffPair_float_0 = s_fwd_log_v2_0(_S2617, _S2613, m_18.white_0);
    var _S2619 : DiffPair_float_0 = DiffPair_float_0( dprgb_6.primal_0.z, dprgb_6.differential_0.z );
    var _S2620 : DiffPair_float_0 = _d_max_1(_S2619, _S2610);
    var _S2621 : DiffPair_float_0 = DiffPair_float_0( _S2620.primal_0, _S2620.differential_0 );
    var _S2622 : DiffPair_float_0 = s_fwd_log_v2_0(_S2621, _S2613, m_18.white_0);
    var t_7 : vec3<f32> = vec3<f32>(_S2614.primal_0, _S2618.primal_0, _S2622.primal_0);
    var s_diff_t_1 : vec3<f32> = vec3<f32>(_S2614.differential_0, _S2618.differential_0, _S2622.differential_0);
    var _S2623 : vec3<f32> = vec3<f32>(0.0f);
    var _S2624 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( m_18.luminance_1, _S2623 );
    var _S2625 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( t_7, s_diff_t_1 );
    var _S2626 : DiffPair_float_0 = _d_dot_1(_S2624, _S2625);
    var _S2627 : DiffPair_float_0 = DiffPair_float_0( _S2626.primal_0, _S2626.differential_0 );
    var _S2628 : DiffPair_float_0 = s_fwd_desaturate_v2_0(_S2627, _S2613);
    var _S2629 : vec3<f32> = vec3<f32>(_S2626.primal_0);
    var _S2630 : vec3<f32> = t_7 - _S2629;
    var _S2631 : vec3<f32> = vec3<f32>(_S2626.differential_0);
    var _S2632 : vec3<f32> = vec3<f32>(_S2628.primal_0);
    var mapped_5 : vec3<f32> = _S2629 + _S2632 * _S2630;
    var s_diff_mapped_1 : vec3<f32> = _S2631 + (vec3<f32>(_S2628.differential_0) * _S2630 + (s_diff_t_1 - _S2631) * _S2632);
    var _S2633 : DiffPair_float_0 = DiffPair_float_0( mapped_5.x, s_diff_mapped_1.x );
    var _S2634 : DiffPair_float_0 = s_fwd_spline_0(_S2633, _S2613, m_18.toe_1, m_18.shoulder_1);
    var _S2635 : DiffPair_float_0 = DiffPair_float_0( mapped_5.y, s_diff_mapped_1.y );
    var _S2636 : DiffPair_float_0 = s_fwd_spline_0(_S2635, _S2613, m_18.toe_1, m_18.shoulder_1);
    var _S2637 : DiffPair_float_0 = DiffPair_float_0( mapped_5.z, s_diff_mapped_1.z );
    var _S2638 : DiffPair_float_0 = s_fwd_spline_0(_S2637, _S2613, m_18.toe_1, m_18.shoulder_1);
    var _S2639 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S2634.primal_0, _S2636.primal_0, _S2638.primal_0), vec3<f32>(_S2634.differential_0, _S2636.differential_0, _S2638.differential_0) );
    var _S2640 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.0f), _S2623 );
    var _S2641 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(1.0f), _S2623 );
    var _S2642 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_clamp_vector_1(_S2639, _S2640, _S2641);
    var _S2643 : DiffPair_float_0 = DiffPair_float_0( _S2642.primal_0.x, _S2642.differential_0.x );
    var _S2644 : DiffPair_float_0 = DiffPair_float_0( dpc_14.primal_0.tone_0.y, dpc_14.differential_0.tone_0.y );
    var _S2645 : DiffPair_float_0 = pow_dt_forward_0(_S2643, _S2644);
    var _S2646 : DiffPair_float_0 = DiffPair_float_0( _S2642.primal_0.y, _S2642.differential_0.y );
    var _S2647 : DiffPair_float_0 = pow_dt_forward_0(_S2646, _S2644);
    var _S2648 : DiffPair_float_0 = DiffPair_float_0( _S2642.primal_0.z, _S2642.differential_0.z );
    var _S2649 : DiffPair_float_0 = pow_dt_forward_0(_S2648, _S2644);
    var _S2650 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S2645.primal_0, _S2647.primal_0, _S2649.primal_0), vec3<f32>(_S2645.differential_0, _S2647.differential_0, _S2649.differential_0) );
    return _S2650;
}

fn sanitised_0( ratios_4 : vec3<f32>) -> vec3<f32>
{
    var _S2651 : f32 = min(min(ratios_4.x, ratios_4.y), ratios_4.z);
    var _S2652 : vec3<f32>;
    if((_S2651 < 0.0f))
    {
        _S2652 = ratios_4 - vec3<f32>(_S2651);
    }
    else
    {
        _S2652 = ratios_4;
    }
    return _S2652;
}

fn s_fwd_sanitised_0( dpratios_0 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S2653 : DiffPair_float_0 = DiffPair_float_0( dpratios_0.primal_0.x, dpratios_0.differential_0.x );
    var _S2654 : DiffPair_float_0 = DiffPair_float_0( dpratios_0.primal_0.y, dpratios_0.differential_0.y );
    var _S2655 : DiffPair_float_0 = _d_min_1(_S2653, _S2654);
    var _S2656 : DiffPair_float_0 = DiffPair_float_0( _S2655.primal_0, _S2655.differential_0 );
    var _S2657 : DiffPair_float_0 = DiffPair_float_0( dpratios_0.primal_0.z, dpratios_0.differential_0.z );
    var _S2658 : DiffPair_float_0 = _d_min_1(_S2656, _S2657);
    var _S2659 : vec3<f32>;
    var _S2660 : vec3<f32>;
    if(((_S2658.primal_0) < 0.0f))
    {
        var _S2661 : vec3<f32> = dpratios_0.differential_0 - vec3<f32>(_S2658.differential_0);
        _S2659 = dpratios_0.primal_0 - vec3<f32>(_S2658.primal_0);
        _S2660 = _S2661;
    }
    else
    {
        _S2659 = dpratios_0.primal_0;
        _S2660 = dpratios_0.differential_0;
    }
    var _S2662 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2659, _S2660 );
    return _S2662;
}

fn chroma_v1_0( rgb_8 : vec3<f32>,  c_19 : Curve_0,  m_19 : Modes_0) -> vec3<f32>
{
    var _S2663 : f32 = max(pixel_norm_0(rgb_8, m_19.norm_0, m_19.luminance_1), 1.52587890625e-05f);
    var norm_6 : f32 = log_v1_0(_S2663, c_19, m_19.white_0);
    var _S2664 : vec3<f32> = vec3<f32>(norm_6);
    var ratios_5 : vec3<f32> = sanitised_0(rgb_8 / vec3<f32>(_S2663)) * _S2664;
    var _S2665 : vec3<f32> = vec3<f32>(dot(m_19.luminance_1, ratios_5));
    return (_S2665 + vec3<f32>(desaturate_v1_0(norm_6, c_19)) * (ratios_5 - _S2665)) / _S2664 * vec3<f32>(pow0_0(clamp(spline_0(norm_6, c_19, m_19.toe_1, m_19.shoulder_1), 0.0f, 1.0f), c_19.tone_0.y));
}

fn s_fwd_chroma_v1_0( dprgb_7 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpc_15 : DiffPair_Curve_0,  m_20 : Modes_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S2666 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( dprgb_7.primal_0, dprgb_7.differential_0 );
    var _S2667 : DiffPair_float_0 = s_fwd_pixel_norm_0(_S2666, m_20.norm_0, m_20.luminance_1);
    var _S2668 : DiffPair_float_0 = DiffPair_float_0( _S2667.primal_0, _S2667.differential_0 );
    var _S2669 : DiffPair_float_0 = DiffPair_float_0( 1.52587890625e-05f, 0.0f );
    var _S2670 : DiffPair_float_0 = _d_max_1(_S2668, _S2669);
    var _S2671 : vec3<f32> = vec3<f32>(_S2670.primal_0);
    var _S2672 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( dprgb_7.primal_0 / _S2671, (dprgb_7.differential_0 * _S2671 - dprgb_7.primal_0 * vec3<f32>(_S2670.differential_0)) / vec3<f32>((_S2670.primal_0 * _S2670.primal_0)) );
    var _S2673 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_sanitised_0(_S2672);
    var _S2674 : DiffPair_float_0 = DiffPair_float_0( _S2670.primal_0, _S2670.differential_0 );
    var _S2675 : DiffPair_Curve_0 = DiffPair_Curve_0( dpc_15.primal_0, dpc_15.differential_0 );
    var _S2676 : DiffPair_float_0 = s_fwd_log_v1_0(_S2674, _S2675, m_20.white_0);
    var _S2677 : DiffPair_float_0 = DiffPair_float_0( _S2676.primal_0, _S2676.differential_0 );
    var _S2678 : DiffPair_float_0 = s_fwd_desaturate_v1_0(_S2677, _S2675);
    var _S2679 : vec3<f32> = vec3<f32>(_S2676.primal_0);
    var ratios_6 : vec3<f32> = _S2673.primal_0 * _S2679;
    var _S2680 : vec3<f32> = vec3<f32>(_S2676.differential_0);
    var s_diff_ratios_0 : vec3<f32> = _S2673.differential_0 * _S2679 + _S2680 * _S2673.primal_0;
    var _S2681 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( m_20.luminance_1, vec3<f32>(0.0f) );
    var _S2682 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( ratios_6, s_diff_ratios_0 );
    var _S2683 : DiffPair_float_0 = _d_dot_1(_S2681, _S2682);
    var _S2684 : vec3<f32> = vec3<f32>(_S2683.primal_0);
    var _S2685 : vec3<f32> = ratios_6 - _S2684;
    var _S2686 : vec3<f32> = vec3<f32>(_S2683.differential_0);
    var _S2687 : vec3<f32> = vec3<f32>(_S2678.primal_0);
    var _S2688 : vec3<f32> = _S2684 + _S2687 * _S2685;
    var ratios_7 : vec3<f32> = _S2688 / _S2679;
    var _S2689 : DiffPair_float_0 = s_fwd_spline_0(_S2677, _S2675, m_20.toe_1, m_20.shoulder_1);
    var _S2690 : DiffPair_float_0 = DiffPair_float_0( _S2689.primal_0, _S2689.differential_0 );
    var _S2691 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S2692 : DiffPair_float_0 = DiffPair_float_0( 1.0f, 0.0f );
    var _S2693 : DiffPair_float_0 = _d_clamp_1(_S2690, _S2691, _S2692);
    var _S2694 : DiffPair_float_0 = DiffPair_float_0( _S2693.primal_0, _S2693.differential_0 );
    var _S2695 : DiffPair_float_0 = DiffPair_float_0( dpc_15.primal_0.tone_0.y, dpc_15.differential_0.tone_0.y );
    var _S2696 : DiffPair_float_0 = pow0_forward_0(_S2694, _S2695);
    var _S2697 : vec3<f32> = vec3<f32>(_S2696.primal_0);
    var _S2698 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( ratios_7 * _S2697, ((_S2686 + (vec3<f32>(_S2678.differential_0) * _S2685 + (s_diff_ratios_0 - _S2686) * _S2687)) * _S2679 - _S2688 * _S2680) / vec3<f32>((_S2676.primal_0 * _S2676.primal_0)) * _S2697 + vec3<f32>(_S2696.differential_0) * ratios_7 );
    return _S2698;
}

fn chroma_v2_v3_0( rgb_9 : vec3<f32>,  c_20 : Curve_0,  m_21 : Modes_0) -> vec3<f32>
{
    if(((c_20.tone_0.z) < 0.0f))
    {
        return vec3<f32>(0.0f);
    }
    var _S2699 : f32 = max(pixel_norm_0(rgb_9, m_21.norm_0, m_21.luminance_1), 1.52587890625e-05f);
    var ratios_8 : vec3<f32> = sanitised_0(rgb_9 / vec3<f32>(_S2699));
    var norm_7 : f32 = log_v2_0(_S2699, c_20, m_21.white_0);
    var norm_8 : f32 = pow0_0(clamp(spline_0(norm_7, c_20, m_21.toe_1, m_21.shoulder_1), 0.0f, 1.0f), c_20.tone_0.y);
    var _S2700 : vec3<f32> = vec3<f32>(0.0f);
    var _S2701 : vec3<f32> = max(ratios_8 + (vec3<f32>(1.0f) - ratios_8) * vec3<f32>((1.0f - desaturate_v2_0(norm_7, c_20))), _S2700);
    var norm_9 : f32;
    if((m_21.version_0) == u32(2))
    {
        norm_9 = norm_8 / max(pixel_norm_0(_S2701, m_21.norm_0, m_21.luminance_1), 1.52587890625e-05f);
    }
    else
    {
        norm_9 = norm_8;
    }
    var out_3 : vec3<f32> = _S2701 * vec3<f32>(norm_9);
    var _S2702 : f32 = max(max(out_3.x, out_3.y), out_3.z);
    var out_4 : vec3<f32>;
    if((_S2702 > 1.0f))
    {
        out_4 = clamp(max(_S2701 + vec3<f32>((1.0f - _S2702)), _S2700) * vec3<f32>(norm_9), _S2700, vec3<f32>(1.0f));
    }
    else
    {
        out_4 = out_3;
    }
    return out_4;
}

fn s_fwd_chroma_v2_v3_0( dprgb_8 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpc_16 : DiffPair_Curve_0,  m_22 : Modes_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    if(((dpc_16.primal_0.tone_0.z) < 0.0f))
    {
        var _S2703 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.0f), vec3<f32>(0.0f) );
        return _S2703;
    }
    var _S2704 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( dprgb_8.primal_0, dprgb_8.differential_0 );
    var _S2705 : DiffPair_float_0 = s_fwd_pixel_norm_0(_S2704, m_22.norm_0, m_22.luminance_1);
    var _S2706 : DiffPair_float_0 = DiffPair_float_0( _S2705.primal_0, _S2705.differential_0 );
    var _S2707 : DiffPair_float_0 = DiffPair_float_0( 1.52587890625e-05f, 0.0f );
    var _S2708 : DiffPair_float_0 = _d_max_1(_S2706, _S2707);
    var _S2709 : vec3<f32> = vec3<f32>(_S2708.primal_0);
    var _S2710 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( dprgb_8.primal_0 / _S2709, (dprgb_8.differential_0 * _S2709 - dprgb_8.primal_0 * vec3<f32>(_S2708.differential_0)) / vec3<f32>((_S2708.primal_0 * _S2708.primal_0)) );
    var _S2711 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_sanitised_0(_S2710);
    var _S2712 : DiffPair_float_0 = DiffPair_float_0( _S2708.primal_0, _S2708.differential_0 );
    var _S2713 : DiffPair_Curve_0 = DiffPair_Curve_0( dpc_16.primal_0, dpc_16.differential_0 );
    var _S2714 : DiffPair_float_0 = s_fwd_log_v2_0(_S2712, _S2713, m_22.white_0);
    var _S2715 : DiffPair_float_0 = DiffPair_float_0( _S2714.primal_0, _S2714.differential_0 );
    var _S2716 : DiffPair_float_0 = s_fwd_desaturate_v2_0(_S2715, _S2713);
    var _S2717 : DiffPair_float_0 = s_fwd_spline_0(_S2715, _S2713, m_22.toe_1, m_22.shoulder_1);
    var _S2718 : DiffPair_float_0 = DiffPair_float_0( _S2717.primal_0, _S2717.differential_0 );
    var _S2719 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S2720 : DiffPair_float_0 = DiffPair_float_0( 1.0f, 0.0f );
    var _S2721 : DiffPair_float_0 = _d_clamp_1(_S2718, _S2719, _S2720);
    var _S2722 : DiffPair_float_0 = DiffPair_float_0( _S2721.primal_0, _S2721.differential_0 );
    var _S2723 : DiffPair_float_0 = DiffPair_float_0( dpc_16.primal_0.tone_0.y, dpc_16.differential_0.tone_0.y );
    var _S2724 : DiffPair_float_0 = pow0_forward_0(_S2722, _S2723);
    var _S2725 : vec3<f32> = vec3<f32>(1.0f) - _S2711.primal_0;
    var _S2726 : vec3<f32> = vec3<f32>(0.0f);
    var _S2727 : vec3<f32> = vec3<f32>((1.0f - _S2716.primal_0));
    var _S2728 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2711.primal_0 + _S2725 * _S2727, _S2711.differential_0 + ((_S2726 - _S2711.differential_0) * _S2727 + vec3<f32>((0.0f - _S2716.differential_0)) * _S2725) );
    var _S2729 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.0f), _S2726 );
    var _S2730 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_max_vector_1(_S2728, _S2729);
    var norm_10 : f32;
    var s_diff_norm_0 : f32;
    if((m_22.version_0) == u32(2))
    {
        var _S2731 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2730.primal_0, _S2730.differential_0 );
        var _S2732 : DiffPair_float_0 = s_fwd_pixel_norm_0(_S2731, m_22.norm_0, m_22.luminance_1);
        var _S2733 : DiffPair_float_0 = DiffPair_float_0( _S2732.primal_0, _S2732.differential_0 );
        var _S2734 : DiffPair_float_0 = _d_max_1(_S2733, _S2707);
        var _S2735 : f32 = (_S2724.differential_0 * _S2734.primal_0 - _S2724.primal_0 * _S2734.differential_0) / (_S2734.primal_0 * _S2734.primal_0);
        norm_10 = _S2724.primal_0 / _S2734.primal_0;
        s_diff_norm_0 = _S2735;
    }
    else
    {
        norm_10 = _S2724.primal_0;
        s_diff_norm_0 = _S2724.differential_0;
    }
    var out_5 : vec3<f32> = _S2730.primal_0 * vec3<f32>(norm_10);
    var s_diff_out_0 : vec3<f32> = _S2730.differential_0 * vec3<f32>(norm_10) + vec3<f32>(s_diff_norm_0) * _S2730.primal_0;
    var _S2736 : DiffPair_float_0 = DiffPair_float_0( out_5.x, s_diff_out_0.x );
    var _S2737 : DiffPair_float_0 = DiffPair_float_0( out_5.y, s_diff_out_0.y );
    var _S2738 : DiffPair_float_0 = _d_max_1(_S2736, _S2737);
    var _S2739 : DiffPair_float_0 = DiffPair_float_0( _S2738.primal_0, _S2738.differential_0 );
    var _S2740 : DiffPair_float_0 = DiffPair_float_0( out_5.z, s_diff_out_0.z );
    var _S2741 : DiffPair_float_0 = _d_max_1(_S2739, _S2740);
    var out_6 : vec3<f32>;
    var s_diff_out_1 : vec3<f32>;
    if(((_S2741.primal_0) > 1.0f))
    {
        var _S2742 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2730.primal_0 + vec3<f32>((1.0f - _S2741.primal_0)), _S2730.differential_0 + vec3<f32>((0.0f - _S2741.differential_0)) );
        var _S2743 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_max_vector_1(_S2742, _S2729);
        var _S2744 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2743.primal_0 * vec3<f32>(norm_10), _S2743.differential_0 * vec3<f32>(norm_10) + vec3<f32>(s_diff_norm_0) * _S2743.primal_0 );
        var _S2745 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(1.0f), _S2726 );
        var _S2746 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_clamp_vector_1(_S2744, _S2729, _S2745);
        out_6 = _S2746.primal_0;
        s_diff_out_1 = _S2746.differential_0;
    }
    else
    {
        out_6 = out_5;
        s_diff_out_1 = s_diff_out_0;
    }
    var _S2747 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( out_6, s_diff_out_1 );
    return _S2747;
}

fn filmic_1( rgb_10 : vec3<f32>,  c_21 : Curve_0,  m_23 : Modes_0) -> vec3<f32>
{
    if((m_23.version_0) == u32(4))
    {
        return filmic_v7_0(rgb_10, c_21, m_23);
    }
    if((m_23.version_0) == u32(3))
    {
        return filmic_v6_0(rgb_10, c_21, m_23);
    }
    if((m_23.norm_0) == u32(0))
    {
        if((m_23.version_0) == u32(0))
        {
            return split_v1_0(rgb_10, c_21, m_23);
        }
        return split_v2_v3_0(rgb_10, c_21, m_23);
    }
    if((m_23.version_0) == u32(0))
    {
        return chroma_v1_0(rgb_10, c_21, m_23);
    }
    return chroma_v2_v3_0(rgb_10, c_21, m_23);
}

fn s_fwd_filmic_0( rgb_11 : vec3<f32>,  dpc_17 : DiffPair_Curve_0,  m_24 : Modes_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    if((m_24.version_0) == u32(4))
    {
        var _S2748 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( rgb_11, vec3<f32>(0.0f) );
        var _S2749 : DiffPair_Curve_0 = DiffPair_Curve_0( dpc_17.primal_0, dpc_17.differential_0 );
        var _S2750 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_filmic_v7_0(_S2748, _S2749, m_24);
        var _S2751 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2750.primal_0, _S2750.differential_0 );
        return _S2751;
    }
    if((m_24.version_0) == u32(3))
    {
        var _S2752 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( rgb_11, vec3<f32>(0.0f) );
        var _S2753 : DiffPair_Curve_0 = DiffPair_Curve_0( dpc_17.primal_0, dpc_17.differential_0 );
        var _S2754 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_filmic_v6_0(_S2752, _S2753, m_24);
        var _S2755 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2754.primal_0, _S2754.differential_0 );
        return _S2755;
    }
    if((m_24.norm_0) == u32(0))
    {
        if((m_24.version_0) == u32(0))
        {
            var _S2756 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( rgb_11, vec3<f32>(0.0f) );
            var _S2757 : DiffPair_Curve_0 = DiffPair_Curve_0( dpc_17.primal_0, dpc_17.differential_0 );
            var _S2758 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_split_v1_0(_S2756, _S2757, m_24);
            var _S2759 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2758.primal_0, _S2758.differential_0 );
            return _S2759;
        }
        var _S2760 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( rgb_11, vec3<f32>(0.0f) );
        var _S2761 : DiffPair_Curve_0 = DiffPair_Curve_0( dpc_17.primal_0, dpc_17.differential_0 );
        var _S2762 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_split_v2_v3_0(_S2760, _S2761, m_24);
        var _S2763 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2762.primal_0, _S2762.differential_0 );
        return _S2763;
    }
    if((m_24.version_0) == u32(0))
    {
        var _S2764 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( rgb_11, vec3<f32>(0.0f) );
        var _S2765 : DiffPair_Curve_0 = DiffPair_Curve_0( dpc_17.primal_0, dpc_17.differential_0 );
        var _S2766 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_chroma_v1_0(_S2764, _S2765, m_24);
        var _S2767 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2766.primal_0, _S2766.differential_0 );
        return _S2767;
    }
    var _S2768 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( rgb_11, vec3<f32>(0.0f) );
    var _S2769 : DiffPair_Curve_0 = DiffPair_Curve_0( dpc_17.primal_0, dpc_17.differential_0 );
    var _S2770 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_chroma_v2_v3_0(_S2768, _S2769, m_24);
    var _S2771 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2770.primal_0, _S2770.differential_0 );
    return _S2771;
}

fn curve_at_0( _S2772 : u32) -> Curve_0
{
    var c_22 : Curve_0;
    var _S2773 : bool = _S2772 == u32(0);
    var _S2774 : vec4<f32>;
    if(_S2773)
    {
        _S2774 = params_0.filmic_0.curve_0[i32(0)];
    }
    else
    {
        _S2774 = params_0.filmic_0.directions_0[_S2772 - u32(7)];
    }
    c_22.toe_0 = _S2774;
    if(_S2773)
    {
        _S2774 = params_0.filmic_0.curve_0[i32(1)];
    }
    else
    {
        _S2774 = params_0.filmic_0.directions_0[_S2772 - u32(6)];
    }
    c_22.shoulder_0 = _S2774;
    if(_S2773)
    {
        _S2774 = params_0.filmic_0.curve_0[i32(2)];
    }
    else
    {
        _S2774 = params_0.filmic_0.directions_0[_S2772 - u32(5)];
    }
    c_22.line_0 = _S2774;
    if(_S2773)
    {
        _S2774 = params_0.filmic_0.curve_0[i32(3)];
    }
    else
    {
        _S2774 = params_0.filmic_0.directions_0[_S2772 - u32(4)];
    }
    c_22.nodes_0 = _S2774;
    if(_S2773)
    {
        _S2774 = params_0.filmic_0.curve_0[i32(4)];
    }
    else
    {
        _S2774 = params_0.filmic_0.directions_0[_S2772 - u32(3)];
    }
    c_22.tone_0 = _S2774;
    if(_S2773)
    {
        _S2774 = params_0.filmic_0.curve_0[i32(5)];
    }
    else
    {
        _S2774 = params_0.filmic_0.directions_0[_S2772 - u32(2)];
    }
    c_22.display_0 = _S2774;
    if(_S2773)
    {
        _S2774 = params_0.filmic_0.curve_0[i32(6)];
    }
    else
    {
        _S2774 = params_0.filmic_0.directions_0[_S2772 - u32(1)];
    }
    c_22.extra_0 = _S2774;
    return c_22;
}

fn curve_of_0() -> Curve_0
{
    return curve_at_0(u32(0));
}

fn matrix_of_0() -> mat3x3<f32>
{
    return mat3x3<f32>(params_0.filmic_0.to_lms_0[i32(0)].xyz, params_0.filmic_0.to_lms_0[i32(1)].xyz, params_0.filmic_0.to_lms_0[i32(2)].xyz);
}

fn matrix_of_1() -> mat3x3<f32>
{
    return mat3x3<f32>(params_0.filmic_0.from_lms_0[i32(0)].xyz, params_0.filmic_0.from_lms_0[i32(1)].xyz, params_0.filmic_0.from_lms_0[i32(2)].xyz);
}

fn matrix_of_2() -> mat3x3<f32>
{
    return mat3x3<f32>(params_0.filmic_0.export_to_lms_0[i32(0)].xyz, params_0.filmic_0.export_to_lms_0[i32(1)].xyz, params_0.filmic_0.export_to_lms_0[i32(2)].xyz);
}

fn matrix_of_3() -> mat3x3<f32>
{
    return mat3x3<f32>(params_0.filmic_0.export_from_lms_0[i32(0)].xyz, params_0.filmic_0.export_from_lms_0[i32(1)].xyz, params_0.filmic_0.export_from_lms_0[i32(2)].xyz);
}

fn modes_of_0() -> Modes_0
{
    var m_25 : Modes_0;
    m_25.version_0 = params_0.filmic_0.modes_0.x;
    m_25.norm_0 = params_0.filmic_0.modes_0.y;
    m_25.toe_1 = params_0.filmic_0.modes_0.z;
    m_25.shoulder_1 = params_0.filmic_0.modes_0.w;
    m_25.white_0 = (params_0.filmic_0.flags_0.z) != u32(0);
    m_25.luminance_1 = params_0.filmic_0.luminance_0.xyz;
    m_25.to_lms_1 = matrix_of_0();
    m_25.from_lms_1 = matrix_of_1();
    m_25.export_to_lms_1 = matrix_of_2();
    m_25.export_from_lms_1 = matrix_of_3();
    return m_25;
}

fn tone_input_0( _S2775 : u32,  _S2776 : u32,  _S2777 : u32) -> vec3<f32>
{
    var _S2778 : vec3<f32> = input_0[_S2775].xyz;
    var _S2779 : bool;
    if((params_0.filmic_0.flags_0.x) != u32(0))
    {
        _S2779 = (counts_0[i32(0)] > u32(9));
    }
    else
    {
        _S2779 = false;
    }
    var rgb_12 : vec3<f32>;
    if(_S2779)
    {
        rgb_12 = reconstructed_0[_S2777 * params_0.filmic_0.image_0.x + _S2776].xyz;
    }
    else
    {
        rgb_12 = _S2778;
    }
    return rgb_12;
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_0 : vec3<u32>)
{
    var _S2780 : u32 = id_0.x;
    var _S2781 : bool;
    if((_S2780 >= (params_0.pass_0.width_0)))
    {
        _S2781 = true;
    }
    else
    {
        _S2781 = ((id_0.y) >= (params_0.pass_0.height_0));
    }
    if(_S2781)
    {
        return;
    }
    var _S2782 : u32 = id_0.y;
    var i_0 : u32 = _S2782 * params_0.pass_0.width_0 + _S2780;
    var _S2783 : Curve_0 = curve_of_0();
    var _S2784 : Modes_0 = modes_of_0();
    var rgb_13 : vec3<f32> = tone_input_0(i_0, params_0.pass_0.origin_x_0 + _S2780, params_0.pass_0.origin_y_0 + _S2782);
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
        var _S2785 : vec3<f32> = filmic_1(rgb_13, _S2783, _S2784);
        var _S2786 : u32 = k_0 + u32(1);
        rgb_13 = _S2785;
        k_0 = _S2786;
    }
    output_0[i_0] = vec4<f32>(rgb_13, input_0[i_0].w);
    return;
}

fn s_fwd_along_0( d_9 : Directions_0,  dpt_0 : DiffPair_vectorx3Cfloatx2C4x3E_0) -> DiffPair_Curve_0
{
    var _S2787 : vec4<f32> = vec4<f32>(dpt_0.primal_0.x);
    var _S2788 : vec4<f32> = vec4<f32>(dpt_0.differential_0.x);
    var _S2789 : vec4<f32> = vec4<f32>(dpt_0.primal_0.y);
    var _S2790 : vec4<f32> = vec4<f32>(dpt_0.differential_0.y);
    var _S2791 : vec4<f32> = vec4<f32>(dpt_0.primal_0.z);
    var _S2792 : vec4<f32> = vec4<f32>(dpt_0.differential_0.z);
    var _S2793 : vec4<f32> = vec4<f32>(dpt_0.primal_0.w);
    var _S2794 : vec4<f32> = vec4<f32>(dpt_0.differential_0.w);
    var _S2795 : Curve_0 = Curve_0( d_9.base_0.toe_0 + _S2787 * d_9.d0_0.toe_0 + _S2789 * d_9.d1_0.toe_0 + _S2791 * d_9.d2_0.toe_0 + _S2793 * d_9.d3_0.toe_0, d_9.base_0.shoulder_0 + _S2787 * d_9.d0_0.shoulder_0 + _S2789 * d_9.d1_0.shoulder_0 + _S2791 * d_9.d2_0.shoulder_0 + _S2793 * d_9.d3_0.shoulder_0, d_9.base_0.line_0 + _S2787 * d_9.d0_0.line_0 + _S2789 * d_9.d1_0.line_0 + _S2791 * d_9.d2_0.line_0 + _S2793 * d_9.d3_0.line_0, d_9.base_0.nodes_0 + _S2787 * d_9.d0_0.nodes_0 + _S2789 * d_9.d1_0.nodes_0 + _S2791 * d_9.d2_0.nodes_0 + _S2793 * d_9.d3_0.nodes_0, d_9.base_0.tone_0 + _S2787 * d_9.d0_0.tone_0 + _S2789 * d_9.d1_0.tone_0 + _S2791 * d_9.d2_0.tone_0 + _S2793 * d_9.d3_0.tone_0, d_9.base_0.display_0 + _S2787 * d_9.d0_0.display_0 + _S2789 * d_9.d1_0.display_0 + _S2791 * d_9.d2_0.display_0 + _S2793 * d_9.d3_0.display_0, d_9.base_0.extra_0 + _S2787 * d_9.d0_0.extra_0 + _S2789 * d_9.d1_0.extra_0 + _S2791 * d_9.d2_0.extra_0 + _S2793 * d_9.d3_0.extra_0 );
    var _S2796 : Curve_0 = Curve_0( _S2788 * d_9.d0_0.toe_0 + _S2790 * d_9.d1_0.toe_0 + _S2792 * d_9.d2_0.toe_0 + _S2794 * d_9.d3_0.toe_0, _S2788 * d_9.d0_0.shoulder_0 + _S2790 * d_9.d1_0.shoulder_0 + _S2792 * d_9.d2_0.shoulder_0 + _S2794 * d_9.d3_0.shoulder_0, _S2788 * d_9.d0_0.line_0 + _S2790 * d_9.d1_0.line_0 + _S2792 * d_9.d2_0.line_0 + _S2794 * d_9.d3_0.line_0, _S2788 * d_9.d0_0.nodes_0 + _S2790 * d_9.d1_0.nodes_0 + _S2792 * d_9.d2_0.nodes_0 + _S2794 * d_9.d3_0.nodes_0, _S2788 * d_9.d0_0.tone_0 + _S2790 * d_9.d1_0.tone_0 + _S2792 * d_9.d2_0.tone_0 + _S2794 * d_9.d3_0.tone_0, _S2788 * d_9.d0_0.display_0 + _S2790 * d_9.d1_0.display_0 + _S2792 * d_9.d2_0.display_0 + _S2794 * d_9.d3_0.display_0, _S2788 * d_9.d0_0.extra_0 + _S2790 * d_9.d1_0.extra_0 + _S2792 * d_9.d2_0.extra_0 + _S2794 * d_9.d3_0.extra_0 );
    var _S2797 : DiffPair_Curve_0 = DiffPair_Curve_0( _S2795, _S2796 );
    return _S2797;
}

fn s_fwd_filmic_along_0( rgb_14 : vec3<f32>,  dpt_1 : DiffPair_vectorx3Cfloatx2C4x3E_0,  d_10 : Directions_0,  m_26 : Modes_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S2798 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( dpt_1.primal_0, dpt_1.differential_0 );
    var _S2799 : DiffPair_Curve_0 = s_fwd_along_0(d_10, _S2798);
    var _S2800 : DiffPair_Curve_0 = DiffPair_Curve_0( _S2799.primal_0, _S2799.differential_0 );
    var _S2801 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_filmic_0(rgb_14, _S2800, m_26);
    var _S2802 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2801.primal_0, _S2801.differential_0 );
    return _S2802;
}

fn direction_of_0( _S2803 : u32) -> Curve_0
{
    return curve_at_0(u32(7) * (_S2803 + u32(1)));
}

fn directions_of_0() -> Directions_0
{
    var d_11 : Directions_0;
    d_11.base_0 = curve_of_0();
    d_11.d0_0 = direction_of_0(u32(0));
    d_11.d1_0 = direction_of_0(u32(1));
    d_11.d2_0 = direction_of_0(u32(2));
    d_11.d3_0 = direction_of_0(u32(3));
    return d_11;
}

@compute
@workgroup_size(16, 16, 1)
fn filmicrgb_backward(@builtin(global_invocation_id) id_1 : vec3<u32>)
{
    var _S2804 : u32 = id_1.x;
    var _S2805 : bool;
    if((_S2804 >= (params_0.pass_0.width_0)))
    {
        _S2805 = true;
    }
    else
    {
        _S2805 = ((id_1.y) >= (params_0.pass_0.height_0));
    }
    if(_S2805)
    {
        return;
    }
    var _S2806 : u32 = id_1.y;
    var i_1 : u32 = _S2806 * params_0.pass_0.width_0 + _S2804;
    var _S2807 : vec3<f32> = tone_input_0(i_1, params_0.pass_0.origin_x_0 + _S2804, params_0.pass_0.origin_y_0 + _S2806);
    var _S2808 : vec4<f32> = vec4<f32>(0.0f);
    var t_8 : DiffPair_vectorx3Cfloatx2C4x3E_0;
    t_8.primal_0 = vec4<f32>(0.0f);
    t_8.differential_0 = _S2808;
    _S2032(_S2807, &(t_8), directions_of_0(), modes_of_0(), derivative_in_0[i_1].xyz);
    derivative_out_0[i_1] = t_8.differential_0;
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn filmicrgb_forward_diff(@builtin(global_invocation_id) id_2 : vec3<u32>)
{
    var _S2809 : u32 = id_2.x;
    var _S2810 : bool;
    if((_S2809 >= (params_0.pass_0.width_0)))
    {
        _S2810 = true;
    }
    else
    {
        _S2810 = ((id_2.y) >= (params_0.pass_0.height_0));
    }
    if(_S2810)
    {
        return;
    }
    var _S2811 : u32 = id_2.y;
    var i_2 : u32 = _S2811 * params_0.pass_0.width_0 + _S2809;
    var _S2812 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( vec4<f32>(0.0f), derivative_in_0[i_2] );
    derivative_out_0[i_2] = vec4<f32>(s_fwd_filmic_along_0(tone_input_0(i_2, params_0.pass_0.origin_x_0 + _S2809, params_0.pass_0.origin_y_0 + _S2811), _S2812, directions_of_0(), modes_of_0()).differential_0, 0.0f);
    return;
}

var<workgroup> shared_counts_0 : array<u32, i32(256)>;

@compute
@workgroup_size(16, 16, 1)
fn hr_mask(@builtin(global_invocation_id) id_3 : vec3<u32>, @builtin(workgroup_id) group_0 : vec3<u32>, @builtin(local_invocation_index) t_9 : u32)
{
    var _S2813 : u32 = id_3.x;
    var _S2814 : vec4<u32> = params_0.filmic_0.image_0;
    var _S2815 : u32 = params_0.filmic_0.image_0.x;
    var _S2816 : bool;
    if((_S2813 < _S2815))
    {
        _S2816 = ((id_3.y) < (_S2814.y));
    }
    else
    {
        _S2816 = false;
    }
    var clipped_0 : u32;
    if(_S2816)
    {
        var i_3 : u32 = id_3.y * _S2815 + _S2813;
        var c_23 : vec3<f32> = input_0[i_3].xyz;
        var _S2817 : f32 = c_23.x;
        var _S2818 : f32 = c_23.y;
        var _S2819 : f32 = c_23.z;
        var argument_0 : f32 = - sqrt(_S2817 * _S2817 + _S2818 * _S2818 + _S2819 * _S2819) * params_0.filmic_0.mask_0.x + params_0.filmic_0.mask_0.y;
        mask_buffer_0[i_3] = clamp(1.0f / (1.0f + exp2(clamp(argument_0, -126.0f, 126.0f))), 0.0f, 1.0f);
        var _S2820 : i32;
        if((4.0f > argument_0))
        {
            _S2820 = i32(1);
        }
        else
        {
            _S2820 = i32(0);
        }
        clipped_0 = u32(_S2820);
    }
    else
    {
        clipped_0 = u32(0);
    }
    shared_counts_0[t_9] = clipped_0;
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
        if((t_9 < stride_0))
        {
            shared_counts_0[t_9] = shared_counts_0[t_9] + shared_counts_0[t_9 + stride_0];
        }
        workgroupBarrier();
        stride_0 = stride_0 / u32(2);
    }
    if(t_9 == u32(0))
    {
        counts_0[u32(1) + group_0.y * _S2814.z + group_0.x] = shared_counts_0[i32(0)];
    }
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn hr_count(@builtin(local_invocation_index) t_10 : u32)
{
    var _S2821 : u32 = params_0.filmic_0.flags_0.y;
    var i_4 : u32 = t_10;
    var sum_6 : u32 = u32(0);
    loop
    {
        if((i_4 < _S2821))
        {
        }
        else
        {
            break;
        }
        var sum_7 : u32 = sum_6 + counts_0[u32(1) + i_4];
        i_4 = i_4 + u32(256);
        sum_6 = sum_7;
    }
    shared_counts_0[t_10] = sum_6;
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
        if((t_10 < stride_1))
        {
            shared_counts_0[t_10] = shared_counts_0[t_10] + shared_counts_0[t_10 + stride_1];
        }
        workgroupBarrier();
        stride_1 = stride_1 / u32(2);
    }
    if(t_10 == u32(0))
    {
        counts_0[i32(0)] = shared_counts_0[i32(0)];
    }
    return;
}

fn mul_wide_0( a_8 : u32,  b_0 : u32) -> vec2<u32>
{
    var a_lo_0 : u32 = (a_8 & (u32(65535)));
    var a_hi_0 : u32 = (a_8 >> (u32(16)));
    var b_lo_0 : u32 = (b_0 & (u32(65535)));
    var b_hi_0 : u32 = (b_0 >> (u32(16)));
    var lo_lo_0 : u32 = a_lo_0 * b_lo_0;
    var t_11 : u32 = a_hi_0 * b_lo_0 + ((lo_lo_0 >> (u32(16))));
    var t_12 : u32 = a_lo_0 * b_hi_0 + ((t_11 & (u32(65535))));
    return vec2<u32>(a_hi_0 * b_hi_0 + ((t_11 >> (u32(16)))) + ((t_12 >> (u32(16)))), (((t_12 << (u32(16)))) | (((lo_lo_0 & (u32(65535)))))));
}

fn mul64_0( a_9 : vec2<u32>,  b_1 : vec2<u32>) -> vec2<u32>
{
    var _S2822 : u32 = a_9.y;
    var _S2823 : u32 = b_1.y;
    var low_0 : vec2<u32> = mul_wide_0(_S2822, _S2823);
    return vec2<u32>(low_0.x + a_9.x * _S2823 + _S2822 * b_1.x, low_0.y);
}

fn splitmix32_0( seed_0 : vec2<u32>) -> u32
{
    var _S2824 : u32 = seed_0.x;
    var r_0 : vec2<u32> = mul64_0(vec2<u32>(_S2824, ((seed_0.y) ^ (((_S2824 >> (u32(1))))))), vec2<u32>(u32(1655298541), u32(2039940597)));
    var _S2825 : u32 = r_0.x;
    var _S2826 : u32 = r_0.y;
    return mul64_0(vec2<u32>((_S2825 ^ (((_S2825 >> (u32(28)))))), (_S2826 ^ (((((_S2826 >> (u32(28)))) | (((_S2825 << (u32(4)))))))))), vec2<u32>(u32(3408187557), u32(3364631987))).x;
}

fn xoshiro128plus_0( s_3 : ptr<function, vec4<u32>>) -> f32
{
    var result_4 : u32 = (*s_3).x + (*s_3).w;
    var t_13 : u32 = (((*s_3).y) << (u32(9)));
    var _S2827 : u32 = (((*s_3)[i32(2)]) ^ (((*s_3).x)));
    (*s_3)[i32(2)] = _S2827;
    (*s_3)[i32(3)] = (((*s_3)[i32(3)]) ^ (((*s_3).y)));
    (*s_3)[i32(1)] = (((*s_3)[i32(1)]) ^ (((*s_3).z)));
    (*s_3)[i32(0)] = (((*s_3)[i32(0)]) ^ (((*s_3).w)));
    (*s_3)[i32(2)] = (_S2827 ^ (t_13));
    (*s_3)[i32(3)] = (((((*s_3).w) << (u32(11)))) | (((((*s_3).w) >> (u32(21))))));
    return f32((result_4 >> (u32(8)))) * (bitcast<f32>((u32(864026624))));
}

fn xoshiro128plus_1( s_4 : ptr<function, vec4<u32>>) -> f32
{
    var result_5 : u32 = (*s_4).x + (*s_4).w;
    var t_14 : u32 = (((*s_4).y) << (u32(9)));
    var _S2828 : u32 = (((*s_4)[i32(2)]) ^ (((*s_4).x)));
    (*s_4)[i32(2)] = _S2828;
    (*s_4)[i32(3)] = (((*s_4)[i32(3)]) ^ (((*s_4).y)));
    (*s_4)[i32(1)] = (((*s_4)[i32(1)]) ^ (((*s_4).z)));
    (*s_4)[i32(0)] = (((*s_4)[i32(0)]) ^ (((*s_4).w)));
    (*s_4)[i32(2)] = (_S2828 ^ (t_14));
    (*s_4)[i32(3)] = (((((*s_4).w) << (u32(11)))) | (((((*s_4).w) >> (u32(21))))));
    return f32((result_5 >> (u32(8)))) * (bitcast<f32>((u32(864026624))));
}

fn noise_state_0( i_5 : u32,  j_0 : u32) -> vec4<u32>
{
    var _S2829 : u32 = j_0 + u32(1);
    var s_5 : vec4<u32> = vec4<u32>(splitmix32_0(vec2<u32>(u32(0), _S2829)), splitmix32_0(mul_wide_0(_S2829, i_5 + u32(3))), splitmix32_0(vec2<u32>(u32(0), u32(1337))), splitmix32_0(vec2<u32>(u32(0), u32(666))));
    var _S2830 : f32 = xoshiro128plus_0(&(s_5));
    var _S2831 : f32 = xoshiro128plus_0(&(s_5));
    var _S2832 : f32 = xoshiro128plus_0(&(s_5));
    var _S2833 : f32 = xoshiro128plus_0(&(s_5));
    return s_5;
}

fn normal_0( u1_0 : f32,  u2_0 : f32,  flip_0 : bool) -> f32
{
    var radius_0 : f32 = sqrt(-2.0f * log(u1_0));
    var _S2834 : f32;
    if(flip_0)
    {
        _S2834 = radius_0 * cos(6.2831854820251465f * u2_0);
    }
    else
    {
        _S2834 = radius_0 * sin(6.2831854820251465f * u2_0);
    }
    return _S2834;
}

fn noise_0( distribution_0 : u32,  mu_0 : vec3<f32>,  sigma_0 : vec3<f32>,  s_6 : ptr<function, vec4<u32>>) -> vec3<f32>
{
    var _S2835 : bool = distribution_0 == u32(1);
    var _S2836 : bool;
    if(_S2835)
    {
        _S2836 = true;
    }
    else
    {
        _S2836 = distribution_0 == u32(2);
    }
    if(_S2836)
    {
        var u1_1 : vec3<f32>;
        var u2_1 : vec3<f32>;
        if(_S2835)
        {
            var _S2837 : f32 = xoshiro128plus_1(&((*s_6)));
            var _S2838 : f32 = max(_S2837, 1.17549435082228751e-38f);
            var _S2839 : f32 = xoshiro128plus_1(&((*s_6)));
            var _S2840 : f32 = max(_S2839, 1.17549435082228751e-38f);
            var _S2841 : f32 = xoshiro128plus_1(&((*s_6)));
            u1_1 = vec3<f32>(_S2838, _S2840, max(_S2841, 1.17549435082228751e-38f));
            var _S2842 : f32 = xoshiro128plus_1(&((*s_6)));
            var _S2843 : f32 = xoshiro128plus_1(&((*s_6)));
            var _S2844 : f32 = xoshiro128plus_1(&((*s_6)));
            u2_1 = vec3<f32>(_S2842, _S2843, _S2844);
        }
        else
        {
            var _S2845 : f32 = xoshiro128plus_1(&((*s_6)));
            u1_1[i32(0)] = max(_S2845, 1.17549435082228751e-38f);
            var _S2846 : f32 = xoshiro128plus_1(&((*s_6)));
            u2_1[i32(0)] = _S2846;
            var _S2847 : f32 = xoshiro128plus_1(&((*s_6)));
            u1_1[i32(1)] = max(_S2847, 1.17549435082228751e-38f);
            var _S2848 : f32 = xoshiro128plus_1(&((*s_6)));
            u2_1[i32(1)] = _S2848;
            var _S2849 : f32 = xoshiro128plus_1(&((*s_6)));
            u1_1[i32(2)] = max(_S2849, 1.17549435082228751e-38f);
            var _S2850 : f32 = xoshiro128plus_1(&((*s_6)));
            u2_1[i32(2)] = _S2850;
        }
        var n_0 : vec3<f32> = vec3<f32>(normal_0(u1_1.x, u2_1.x, true), normal_0(u1_1.y, u2_1.y, false), normal_0(u1_1.z, u2_1.z, true));
        if(_S2835)
        {
            return n_0 * sigma_0 + mu_0;
        }
        var _S2851 : vec3<f32> = vec3<f32>(0.375f);
        var r_1 : vec3<f32> = n_0 * sigma_0 + vec3<f32>(2.0f) * sqrt(max(mu_0 + _S2851, vec3<f32>(0.0f)));
        return (r_1 * r_1 - sigma_0 * sigma_0) / vec3<f32>(4.0f) - _S2851;
    }
    var _S2852 : f32 = xoshiro128plus_1(&((*s_6)));
    var _S2853 : f32 = xoshiro128plus_1(&((*s_6)));
    var _S2854 : f32 = xoshiro128plus_1(&((*s_6)));
    return mu_0 + vec3<f32>(2.0f) * (vec3<f32>(_S2852, _S2853, _S2854) - vec3<f32>(0.5f)) * sigma_0;
}

@compute
@workgroup_size(16, 16, 1)
fn hr_inpaint(@builtin(global_invocation_id) id_4 : vec3<u32>)
{
    var _S2855 : u32 = id_4.x;
    var _S2856 : vec4<u32> = params_0.filmic_0.image_0;
    var _S2857 : u32 = params_0.filmic_0.image_0.x;
    var _S2858 : bool;
    if((_S2855 >= _S2857))
    {
        _S2858 = true;
    }
    else
    {
        _S2858 = ((id_4.y) >= (_S2856.y));
    }
    if(_S2858)
    {
        return;
    }
    var _S2859 : u32 = id_4.y;
    var i_6 : u32 = _S2859 * _S2857 + _S2855;
    var s_7 : vec4<u32> = noise_state_0(_S2859, _S2855);
    var c_24 : vec4<f32> = input_0[i_6];
    var _S2860 : vec3<f32> = c_24.xyz;
    var n_1 : vec3<f32> = noise_0(params_0.filmic_0.wavelet_0.w, _S2860, _S2860 * vec3<f32>(params_0.filmic_0.mask_0.z) / vec3<f32>(params_0.filmic_0.mask_0.w), &(s_7));
    detail_buffer_0[i_6] = vec4<f32>(max(_S2860 * vec3<f32>((1.0f - mask_buffer_0[i_6])) + vec3<f32>(mask_buffer_0[i_6]) * n_1, vec3<f32>(0.0f)), c_24.w);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn hr_init(@builtin(global_invocation_id) id_5 : vec3<u32>)
{
    var _S2861 : u32 = id_5.x;
    var _S2862 : vec4<u32> = params_0.filmic_0.image_0;
    var _S2863 : u32 = params_0.filmic_0.image_0.x;
    var _S2864 : bool;
    if((_S2861 >= _S2863))
    {
        _S2864 = true;
    }
    else
    {
        _S2864 = ((id_5.y) >= (_S2862.y));
    }
    if(_S2864)
    {
        return;
    }
    var i_7 : u32 = id_5.y * _S2863 + _S2861;
    reconstructed_0[i_7] = max(detail_buffer_0[i_7] * vec4<f32>((1.0f - mask_buffer_0[i_7])), vec4<f32>(0.0f));
    return;
}

fn bspline_0( a_10 : vec4<f32>,  b_2 : vec4<f32>,  c_25 : vec4<f32>,  d_12 : vec4<f32>,  e_0 : vec4<f32>) -> vec4<f32>
{
    var _S2865 : vec4<f32> = vec4<f32>(0.0625f);
    var _S2866 : vec4<f32> = vec4<f32>(0.25f);
    return max(vec4<f32>(0.0f), _S2865 * a_10 + _S2866 * b_2 + vec4<f32>(0.375f) * c_25 + _S2866 * d_12 + _S2865 * e_0);
}

@compute
@workgroup_size(16, 16, 1)
fn hr_blur_v(@builtin(global_invocation_id) id_6 : vec3<u32>)
{
    var _S2867 : vec4<u32> = params_0.filmic_0.image_0;
    var width_1 : u32 = params_0.filmic_0.image_0.x;
    var height_1 : u32 = params_0.filmic_0.image_0.y;
    var _S2868 : u32 = id_6.x;
    var _S2869 : bool;
    if((_S2868 >= width_1))
    {
        _S2869 = true;
    }
    else
    {
        _S2869 = ((id_6.y) >= height_1);
    }
    if(_S2869)
    {
        return;
    }
    var _S2870 : u32 = id_6.y;
    var row_0 : i32 = i32(_S2870);
    var mult_0 : i32 = i32(_S2867.w);
    var last_0 : i32 = i32(height_1) - i32(1);
    var _S2871 : i32 = i32(2) * mult_0;
    temp_buffer_0[_S2870 * width_1 + _S2868] = bspline_0(detail_buffer_0[u32(max(row_0 - _S2871, i32(0))) * width_1 + _S2868], detail_buffer_0[u32(max(row_0 - mult_0, i32(0))) * width_1 + _S2868], detail_buffer_0[u32(row_0) * width_1 + _S2868], detail_buffer_0[u32(min(row_0 + mult_0, last_0)) * width_1 + _S2868], detail_buffer_0[u32(min(row_0 + _S2871, last_0)) * width_1 + _S2868]);
    return;
}

fn horizontal_0( x_33 : u32,  y_18 : u32,  mult_1 : i32,  width_2 : u32) -> vec4<f32>
{
    var col_0 : i32 = i32(x_33);
    var last_1 : i32 = i32(width_2) - i32(1);
    var row_1 : u32 = y_18 * width_2;
    var _S2872 : i32 = i32(2) * mult_1;
    return bspline_0(temp_buffer_0[row_1 + u32(max(col_0 - _S2872, i32(0)))], temp_buffer_0[row_1 + u32(max(col_0 - mult_1, i32(0)))], temp_buffer_0[row_1 + x_33], temp_buffer_0[row_1 + u32(min(col_0 + mult_1, last_1))], temp_buffer_0[row_1 + u32(min(col_0 + _S2872, last_1))]);
}

@compute
@workgroup_size(16, 16, 1)
fn hr_blur_h(@builtin(global_invocation_id) id_7 : vec3<u32>)
{
    var _S2873 : u32 = id_7.x;
    var _S2874 : vec4<u32> = params_0.filmic_0.image_0;
    var _S2875 : u32 = params_0.filmic_0.image_0.x;
    var _S2876 : bool;
    if((_S2873 >= _S2875))
    {
        _S2876 = true;
    }
    else
    {
        _S2876 = ((id_7.y) >= (_S2874.y));
    }
    if(_S2876)
    {
        return;
    }
    var _S2877 : u32 = id_7.y;
    var i_8 : u32 = _S2877 * _S2875 + _S2873;
    var low_1 : vec4<f32> = horizontal_0(_S2873, _S2877, i32(_S2874.w), _S2875);
    low_buffer_0[i_8] = low_1;
    detail_buffer_0[i_8] = detail_buffer_0[i_8] - low_1;
    return;
}

fn max_abs_0( a_11 : f32,  b_3 : f32) -> f32
{
    var _S2878 : f32;
    if(((abs(a_11)) > (abs(b_3))))
    {
        _S2878 = a_11;
    }
    else
    {
        _S2878 = b_3;
    }
    return _S2878;
}

@compute
@workgroup_size(16, 16, 1)
fn hr_reconstruct(@builtin(global_invocation_id) id_8 : vec3<u32>)
{
    var _S2879 : u32 = id_8.x;
    var _S2880 : vec4<u32> = params_0.filmic_0.image_0;
    var _S2881 : u32 = params_0.filmic_0.image_0.x;
    var _S2882 : bool;
    if((_S2879 >= _S2881))
    {
        _S2882 = true;
    }
    else
    {
        _S2882 = ((id_8.y) >= (_S2880.y));
    }
    if(_S2882)
    {
        return;
    }
    var _S2883 : u32 = id_8.y;
    var i_9 : u32 = _S2883 * _S2881 + _S2879;
    var hf_0 : vec3<f32> = horizontal_0(_S2879, _S2883, i32(1), _S2881).xyz;
    var lf_0 : vec3<f32> = low_buffer_0[i_9].xyz;
    var texture_0 : vec3<f32> = detail_buffer_0[i_9].xyz;
    var alpha_0 : f32 = mask_buffer_0[i_9];
    var gamma_0 : f32 = params_0.filmic_0.balance_0.x;
    var gamma_comp_0 : f32 = 1.0f - gamma_0;
    var beta_0 : f32 = params_0.filmic_0.balance_0.y;
    var beta_comp_0 : f32 = 1.0f - beta_0;
    var delta_4 : f32 = params_0.filmic_0.balance_0.z;
    var last_2 : bool = (params_0.filmic_0.wavelet_0.x) == (params_0.filmic_0.wavelet_0.y);
    var grey_texture_0 : f32 = max_abs_0(max_abs_0(texture_0.x, texture_0.y), texture_0.z);
    var grey_details_0 : f32 = (hf_0.x + hf_0.y + hf_0.z) / 3.0f;
    var _S2884 : vec3<f32> = vec3<f32>(0.0f);
    var details_0 : vec3<f32>;
    var residual_0 : vec3<f32>;
    if((params_0.filmic_0.wavelet_0.z) == u32(0))
    {
        var _S2885 : vec3<f32> = vec3<f32>(beta_0);
        var _S2886 : vec3<f32> = (vec3<f32>(gamma_comp_0) * hf_0 + vec3<f32>(gamma_0) * texture_0) * _S2885 + vec3<f32>((beta_comp_0 * (gamma_comp_0 * grey_details_0 + gamma_0 * grey_texture_0)));
        if(last_2)
        {
            details_0 = vec3<f32>((beta_comp_0 * (lf_0.x + lf_0.y + lf_0.z) / 3.0f)) + lf_0 * _S2885;
        }
        else
        {
            details_0 = _S2884;
        }
        var _S2887 : vec3<f32> = details_0;
        details_0 = _S2886;
        residual_0 = _S2887;
    }
    else
    {
        var _S2888 : vec3<f32> = vec3<f32>(0.5f) * (vec3<f32>(gamma_comp_0) * hf_0 + vec3<f32>(gamma_0) * texture_0 + vec3<f32>((gamma_comp_0 * grey_details_0 + gamma_0 * grey_texture_0)));
        if(last_2)
        {
            details_0 = lf_0;
        }
        else
        {
            details_0 = _S2884;
        }
        var _S2889 : vec3<f32> = details_0;
        details_0 = _S2888;
        residual_0 = _S2889;
    }
    reconstructed_0[i_9] = vec4<f32>(reconstructed_0[i_9].xyz + vec3<f32>(alpha_0) * (vec3<f32>(delta_4) * details_0 + residual_0), reconstructed_0[i_9].w);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn hr_ratios(@builtin(global_invocation_id) id_9 : vec3<u32>)
{
    var _S2890 : u32 = id_9.x;
    var _S2891 : vec4<u32> = params_0.filmic_0.image_0;
    var _S2892 : u32 = params_0.filmic_0.image_0.x;
    var _S2893 : bool;
    if((_S2890 >= _S2892))
    {
        _S2893 = true;
    }
    else
    {
        _S2893 = ((id_9.y) >= (_S2891.y));
    }
    if(_S2893)
    {
        return;
    }
    var i_10 : u32 = id_9.y * _S2892 + _S2890;
    var c_26 : vec4<f32> = reconstructed_0[i_10];
    var _S2894 : f32 = reconstructed_0[i_10].x;
    var _S2895 : f32 = reconstructed_0[i_10].y;
    var _S2896 : f32 = reconstructed_0[i_10].z;
    var _S2897 : f32 = max(sqrt(_S2894 * _S2894 + _S2895 * _S2895 + _S2896 * _S2896), 1.52587890625e-05f);
    norm_buffer_0[i_10] = _S2897;
    detail_buffer_0[i_10] = c_26 / vec4<f32>(_S2897);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn hr_restore(@builtin(global_invocation_id) id_10 : vec3<u32>)
{
    var _S2898 : u32 = id_10.x;
    var _S2899 : vec4<u32> = params_0.filmic_0.image_0;
    var _S2900 : u32 = params_0.filmic_0.image_0.x;
    var _S2901 : bool;
    if((_S2898 >= _S2900))
    {
        _S2901 = true;
    }
    else
    {
        _S2901 = ((id_10.y) >= (_S2899.y));
    }
    if(_S2901)
    {
        return;
    }
    var i_11 : u32 = id_10.y * _S2900 + _S2898;
    reconstructed_0[i_11] = clamp(reconstructed_0[i_11], vec4<f32>(0.0f), vec4<f32>(1.0f)) * vec4<f32>(norm_buffer_0[i_11]);
    return;
}

