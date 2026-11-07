// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2020-2026  darktable developers (ported from src/iop/sigmoid.c)
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

// Generated from slang/sigmoid.slang by tools/build-kernels.sh. Do not edit.

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

struct SigmoidParams_std140_0
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
    @align(16) white_target_0 : f32,
    @align(4) black_target_0 : f32,
    @align(8) log2_paper_exposure_0 : f32,
    @align(4) film_fog_0 : f32,
    @align(16) film_power_0 : f32,
    @align(4) paper_power_0 : f32,
    @align(8) hue_preservation_0 : f32,
    @align(4) color_processing_0 : u32,
    @align(16) film_overflow_0 : f32,
    @align(4) film_underflow_0 : f32,
    @align(8) paper_exposure_0 : u32,
    @align(4) _pad0_0 : u32,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) sigmoid_0 : SigmoidParams_std140_0,
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

fn s_apply_dot_0( x_0 : vec3<f32>,  y_0 : vec3<f32>) -> Tuple_0
{
    var _S1 : Tuple_0 = Tuple_0( dot(x_0, y_0) );
    return _S1;
}

struct s_bwdCallableCtx_dot_0
{
     _S2 : vec3<f32>,
     _S3 : vec3<f32>,
};

struct DiffPair_vectorx3Cfloatx2C3x3E_0
{
     primal_0 : vec3<f32>,
     differential_0 : vec3<f32>,
};

fn _d_dot_0( dpx_0 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dpy_0 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_0 : f32)
{
    var x_d_result_0 : vec3<f32>;
    x_d_result_0[i32(0)] = (*dpy_0).primal_0[i32(0)] * dOut_0;
    var y_d_result_0 : vec3<f32>;
    y_d_result_0[i32(0)] = (*dpx_0).primal_0[i32(0)] * dOut_0;
    x_d_result_0[i32(1)] = (*dpy_0).primal_0[i32(1)] * dOut_0;
    y_d_result_0[i32(1)] = (*dpx_0).primal_0[i32(1)] * dOut_0;
    x_d_result_0[i32(2)] = (*dpy_0).primal_0[i32(2)] * dOut_0;
    y_d_result_0[i32(2)] = (*dpx_0).primal_0[i32(2)] * dOut_0;
    (*dpx_0).primal_0 = (*dpx_0).primal_0;
    (*dpx_0).differential_0 = x_d_result_0;
    (*dpy_0).primal_0 = (*dpy_0).primal_0;
    (*dpy_0).differential_0 = y_d_result_0;
    return;
}

fn s_bwdProp_dot_0( ctx_0 : s_bwdCallableCtx_dot_0,  d_x_0 : ptr<function, vec3<f32>>,  d_y_0 : ptr<function, vec3<f32>>,  d_out_0 : f32)
{
    var _S4 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S4.primal_0 = ctx_0._S2;
    var _S5 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S5.primal_0 = ctx_0._S3;
    _d_dot_0(&(_S4), &(_S5), d_out_0);
    (*d_x_0) = _S4.differential_0;
    (*d_y_0) = _S5.differential_0;
    return;
}

struct Tuple_1
{
     value0_1 : f32,
};

fn s_apply_exp2_0( x_1 : f32) -> Tuple_1
{
    var _S6 : Tuple_1 = Tuple_1( exp2(x_1) );
    return _S6;
}

struct s_bwdCallableCtx_exp2_0
{
     _S7 : f32,
};

struct DiffPair_float_0
{
     primal_0 : f32,
     differential_0 : f32,
};

fn _d_exp2_0( dpx_1 : ptr<function, DiffPair_float_0>,  dOut_1 : f32)
{
    var _S8 : f32 = exp2((*dpx_1).primal_0) * 0.69314718246459961f * dOut_1;
    (*dpx_1).primal_0 = (*dpx_1).primal_0;
    (*dpx_1).differential_0 = _S8;
    return;
}

fn s_bwdProp_exp2_0( ctx_1 : s_bwdCallableCtx_exp2_0,  d_x_1 : ptr<function, f32>,  d_out_1 : f32)
{
    var _S9 : DiffPair_float_0;
    _S9.primal_0 = ctx_1._S7;
    _d_exp2_0(&(_S9), d_out_1);
    (*d_x_1) = _S9.differential_0;
    return;
}

struct Tuple_2
{
     value0_2 : f32,
};

fn s_apply_log2_0( x_2 : f32) -> Tuple_2
{
    var _S10 : Tuple_2 = Tuple_2( log2(x_2) );
    return _S10;
}

struct s_bwdCallableCtx_log2_0
{
     _S11 : f32,
};

fn _d_log2_0( dpx_2 : ptr<function, DiffPair_float_0>,  dOut_2 : f32)
{
    var _S12 : f32 = 1.0f / ((*dpx_2).primal_0 * 0.69314718246459961f) * dOut_2;
    (*dpx_2).primal_0 = (*dpx_2).primal_0;
    (*dpx_2).differential_0 = _S12;
    return;
}

fn s_bwdProp_log2_0( ctx_2 : s_bwdCallableCtx_log2_0,  d_x_2 : ptr<function, f32>,  d_out_2 : f32)
{
    var _S13 : DiffPair_float_0;
    _S13.primal_0 = ctx_2._S11;
    _d_log2_0(&(_S13), d_out_2);
    (*d_x_2) = _S13.differential_0;
    return;
}

struct s_bwdCallableCtx_mul_0
{
     _S14 : mat3x3<f32>,
     _S15 : vec3<f32>,
};

struct DiffPair_matrixx3Cfloatx2C3x2C3x3E_0
{
     primal_0 : mat3x3<f32>,
     differential_0 : mat3x3<f32>,
};

fn _d_mul_0( left_0 : ptr<function, DiffPair_matrixx3Cfloatx2C3x2C3x3E_0>,  right_0 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_3 : vec3<f32>)
{
    var _S16 : f32 = (*left_0).primal_0[i32(0)][i32(0)] * dOut_3[i32(0)];
    var left_d_result_0 : mat3x3<f32>;
    left_d_result_0[i32(0)][i32(0)] = (*right_0).primal_0[i32(0)] * dOut_3[i32(0)];
    var sum_0 : f32 = _S16 + (*left_0).primal_0[i32(1)][i32(0)] * dOut_3[i32(1)];
    left_d_result_0[i32(1)][i32(0)] = (*right_0).primal_0[i32(0)] * dOut_3[i32(1)];
    var sum_1 : f32 = sum_0 + (*left_0).primal_0[i32(2)][i32(0)] * dOut_3[i32(2)];
    left_d_result_0[i32(2)][i32(0)] = (*right_0).primal_0[i32(0)] * dOut_3[i32(2)];
    var right_d_result_0 : vec3<f32>;
    right_d_result_0[i32(0)] = sum_1;
    var _S17 : f32 = (*left_0).primal_0[i32(0)][i32(1)] * dOut_3[i32(0)];
    left_d_result_0[i32(0)][i32(1)] = (*right_0).primal_0[i32(1)] * dOut_3[i32(0)];
    var sum_2 : f32 = _S17 + (*left_0).primal_0[i32(1)][i32(1)] * dOut_3[i32(1)];
    left_d_result_0[i32(1)][i32(1)] = (*right_0).primal_0[i32(1)] * dOut_3[i32(1)];
    var sum_3 : f32 = sum_2 + (*left_0).primal_0[i32(2)][i32(1)] * dOut_3[i32(2)];
    left_d_result_0[i32(2)][i32(1)] = (*right_0).primal_0[i32(1)] * dOut_3[i32(2)];
    right_d_result_0[i32(1)] = sum_3;
    var _S18 : f32 = (*left_0).primal_0[i32(0)][i32(2)] * dOut_3[i32(0)];
    left_d_result_0[i32(0)][i32(2)] = (*right_0).primal_0[i32(2)] * dOut_3[i32(0)];
    var sum_4 : f32 = _S18 + (*left_0).primal_0[i32(1)][i32(2)] * dOut_3[i32(1)];
    left_d_result_0[i32(1)][i32(2)] = (*right_0).primal_0[i32(2)] * dOut_3[i32(1)];
    var sum_5 : f32 = sum_4 + (*left_0).primal_0[i32(2)][i32(2)] * dOut_3[i32(2)];
    left_d_result_0[i32(2)][i32(2)] = (*right_0).primal_0[i32(2)] * dOut_3[i32(2)];
    right_d_result_0[i32(2)] = sum_5;
    (*left_0).primal_0 = (*left_0).primal_0;
    (*left_0).differential_0 = left_d_result_0;
    (*right_0).primal_0 = (*right_0).primal_0;
    (*right_0).differential_0 = right_d_result_0;
    return;
}

fn s_bwdProp_mul_0( ctx_3 : s_bwdCallableCtx_mul_0,  d_left_0 : ptr<function, mat3x3<f32>>,  d_right_0 : ptr<function, vec3<f32>>,  d_out_3 : vec3<f32>)
{
    var _S19 : DiffPair_matrixx3Cfloatx2C3x2C3x3E_0;
    _S19.primal_0 = ctx_3._S14;
    var _S20 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S20.primal_0 = ctx_3._S15;
    _d_mul_0(&(_S19), &(_S20), d_out_3);
    (*d_left_0) = _S19.differential_0;
    (*d_right_0) = _S20.differential_0;
    return;
}

struct Tuple_3
{
     value0_3 : f32,
};

fn s_apply_sqrt_0( x_3 : f32) -> Tuple_3
{
    var _S21 : Tuple_3 = Tuple_3( sqrt(x_3) );
    return _S21;
}

struct s_bwdCallableCtx_sqrt_0
{
     _S22 : f32,
};

fn _d_sqrt_0( dpx_3 : ptr<function, DiffPair_float_0>,  dOut_4 : f32)
{
    var _S23 : f32 = 0.5f / sqrt(max(1.00000001168609742e-07f, (*dpx_3).primal_0)) * dOut_4;
    (*dpx_3).primal_0 = (*dpx_3).primal_0;
    (*dpx_3).differential_0 = _S23;
    return;
}

fn s_bwdProp_sqrt_0( ctx_4 : s_bwdCallableCtx_sqrt_0,  d_x_3 : ptr<function, f32>,  d_out_4 : f32)
{
    var _S24 : DiffPair_float_0;
    _S24.primal_0 = ctx_4._S22;
    _d_sqrt_0(&(_S24), d_out_4);
    (*d_x_3) = _S24.differential_0;
    return;
}

struct Tuple_4
{
     value0_4 : f32,
};

fn s_apply_max_0( x_4 : f32,  y_1 : f32) -> Tuple_4
{
    var _S25 : Tuple_4 = Tuple_4( max(x_4, y_1) );
    return _S25;
}

struct s_bwdCallableCtx_max_0
{
     _S26 : f32,
     _S27 : f32,
};

fn _d_max_0( dpx_4 : ptr<function, DiffPair_float_0>,  dpy_1 : ptr<function, DiffPair_float_0>,  dOut_5 : f32)
{
    var _S28 : DiffPair_float_0 = (*dpx_4);
    var _S29 : f32;
    if((((*dpx_4).primal_0) > ((*dpy_1).primal_0)))
    {
        _S29 = dOut_5;
    }
    else
    {
        if((((*dpx_4).primal_0) < ((*dpy_1).primal_0)))
        {
            _S29 = 0.0f;
        }
        else
        {
            _S29 = 0.5f * dOut_5;
        }
    }
    (*dpx_4).primal_0 = _S28.primal_0;
    (*dpx_4).differential_0 = _S29;
    var _S30 : DiffPair_float_0 = (*dpy_1);
    if((((*dpy_1).primal_0) > (_S28.primal_0)))
    {
        _S29 = dOut_5;
    }
    else
    {
        if((((*dpy_1).primal_0) < ((*dpx_4).primal_0)))
        {
            _S29 = 0.0f;
        }
        else
        {
            _S29 = 0.5f * dOut_5;
        }
    }
    (*dpy_1).primal_0 = _S30.primal_0;
    (*dpy_1).differential_0 = _S29;
    return;
}

fn s_bwdProp_max_0( ctx_5 : s_bwdCallableCtx_max_0,  d_x_4 : ptr<function, f32>,  d_y_1 : ptr<function, f32>,  d_out_5 : f32)
{
    var _S31 : DiffPair_float_0;
    _S31.primal_0 = ctx_5._S26;
    var _S32 : DiffPair_float_0;
    _S32.primal_0 = ctx_5._S27;
    _d_max_0(&(_S31), &(_S32), d_out_5);
    (*d_x_4) = _S31.differential_0;
    (*d_y_1) = _S32.differential_0;
    return;
}

struct Tuple_5
{
     value0_5 : f32,
};

fn s_apply_min_0( x_5 : f32,  y_2 : f32) -> Tuple_5
{
    var _S33 : Tuple_5 = Tuple_5( min(x_5, y_2) );
    return _S33;
}

struct s_bwdCallableCtx_min_0
{
     _S34 : f32,
     _S35 : f32,
};

fn _d_min_0( dpx_5 : ptr<function, DiffPair_float_0>,  dpy_2 : ptr<function, DiffPair_float_0>,  dOut_6 : f32)
{
    var _S36 : DiffPair_float_0 = (*dpx_5);
    var _S37 : f32;
    if((((*dpx_5).primal_0) < ((*dpy_2).primal_0)))
    {
        _S37 = dOut_6;
    }
    else
    {
        if((((*dpx_5).primal_0) > ((*dpy_2).primal_0)))
        {
            _S37 = 0.0f;
        }
        else
        {
            _S37 = 0.5f * dOut_6;
        }
    }
    (*dpx_5).primal_0 = _S36.primal_0;
    (*dpx_5).differential_0 = _S37;
    var _S38 : DiffPair_float_0 = (*dpy_2);
    if((((*dpy_2).primal_0) < (_S36.primal_0)))
    {
        _S37 = dOut_6;
    }
    else
    {
        if((((*dpy_2).primal_0) > ((*dpx_5).primal_0)))
        {
            _S37 = 0.0f;
        }
        else
        {
            _S37 = 0.5f * dOut_6;
        }
    }
    (*dpy_2).primal_0 = _S38.primal_0;
    (*dpy_2).differential_0 = _S37;
    return;
}

fn s_bwdProp_min_0( ctx_6 : s_bwdCallableCtx_min_0,  d_x_5 : ptr<function, f32>,  d_y_2 : ptr<function, f32>,  d_out_6 : f32)
{
    var _S39 : DiffPair_float_0;
    _S39.primal_0 = ctx_6._S34;
    var _S40 : DiffPair_float_0;
    _S40.primal_0 = ctx_6._S35;
    _d_min_0(&(_S39), &(_S40), d_out_6);
    (*d_x_5) = _S39.differential_0;
    (*d_y_2) = _S40.differential_0;
    return;
}

struct Tuple_6
{
     value0_6 : vec3<f32>,
};

fn s_apply_clamp_0( x_6 : vec3<f32>,  minBound_0 : vec3<f32>,  maxBound_0 : vec3<f32>) -> Tuple_6
{
    var _S41 : Tuple_6 = Tuple_6( clamp(x_6, minBound_0, maxBound_0) );
    return _S41;
}

fn _d_clamp_0( dpx_6 : DiffPair_float_0,  dpMin_0 : DiffPair_float_0,  dpMax_0 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S42 : f32 = clamp(dpx_6.primal_0, dpMin_0.primal_0, dpMax_0.primal_0);
    var _S43 : bool;
    if(((dpx_6.primal_0) >= (dpMin_0.primal_0)))
    {
        _S43 = ((dpx_6.primal_0) <= (dpMax_0.primal_0));
    }
    else
    {
        _S43 = false;
    }
    var _S44 : f32;
    if(_S43)
    {
        _S44 = dpx_6.differential_0;
    }
    else
    {
        if(((dpx_6.primal_0) < (dpMin_0.primal_0)))
        {
            _S44 = dpMin_0.differential_0;
        }
        else
        {
            _S44 = dpMax_0.differential_0;
        }
    }
    var _S45 : DiffPair_float_0 = DiffPair_float_0( _S42, _S44 );
    return _S45;
}

fn _d_dot_1( dpx_7 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpy_3 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_float_0
{
    var _S46 : DiffPair_float_0 = DiffPair_float_0( dpx_7.primal_0[i32(0)] * dpy_3.primal_0[i32(0)] + dpx_7.primal_0[i32(1)] * dpy_3.primal_0[i32(1)] + dpx_7.primal_0[i32(2)] * dpy_3.primal_0[i32(2)], dpx_7.primal_0[i32(0)] * dpy_3.differential_0[i32(0)] + dpy_3.primal_0[i32(0)] * dpx_7.differential_0[i32(0)] + dpx_7.primal_0[i32(1)] * dpy_3.differential_0[i32(1)] + dpy_3.primal_0[i32(1)] * dpx_7.differential_0[i32(1)] + dpx_7.primal_0[i32(2)] * dpy_3.differential_0[i32(2)] + dpy_3.primal_0[i32(2)] * dpx_7.differential_0[i32(2)] );
    return _S46;
}

fn _d_sqrt_1( dpx_8 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S47 : DiffPair_float_0 = DiffPair_float_0( sqrt(dpx_8.primal_0), 0.5f / sqrt(max(1.00000001168609742e-07f, dpx_8.primal_0)) * dpx_8.differential_0 );
    return _S47;
}

fn _d_exp2_1( dpx_9 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S48 : f32 = exp2(dpx_9.primal_0);
    var _S49 : DiffPair_float_0 = DiffPair_float_0( _S48, _S48 * 0.69314718246459961f * dpx_9.differential_0 );
    return _S49;
}

fn _d_log2_1( dpx_10 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S50 : DiffPair_float_0 = DiffPair_float_0( log2(dpx_10.primal_0), 1.0f / (dpx_10.primal_0 * 0.69314718246459961f) * dpx_10.differential_0 );
    return _S50;
}

fn _d_min_1( dpx_11 : DiffPair_float_0,  dpy_4 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S51 : f32 = min(dpx_11.primal_0, dpy_4.primal_0);
    var _S52 : f32;
    if(((dpx_11.primal_0) < (dpy_4.primal_0)))
    {
        _S52 = dpx_11.differential_0;
    }
    else
    {
        if(((dpx_11.primal_0) > (dpy_4.primal_0)))
        {
            _S52 = dpy_4.differential_0;
        }
        else
        {
            _S52 = 0.5f * (dpx_11.differential_0 + dpy_4.differential_0);
        }
    }
    var _S53 : DiffPair_float_0 = DiffPair_float_0( _S51, _S52 );
    return _S53;
}

fn _d_max_1( dpx_12 : DiffPair_float_0,  dpy_5 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S54 : f32 = max(dpx_12.primal_0, dpy_5.primal_0);
    var _S55 : f32;
    if(((dpx_12.primal_0) > (dpy_5.primal_0)))
    {
        _S55 = dpx_12.differential_0;
    }
    else
    {
        if(((dpx_12.primal_0) < (dpy_5.primal_0)))
        {
            _S55 = dpy_5.differential_0;
        }
        else
        {
            _S55 = 0.5f * (dpx_12.differential_0 + dpy_5.differential_0);
        }
    }
    var _S56 : DiffPair_float_0 = DiffPair_float_0( _S54, _S55 );
    return _S56;
}

fn _d_clamp_vector_0( dpx_13 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpy_6 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpz_0 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S57 : DiffPair_float_0 = DiffPair_float_0( dpx_13.primal_0[i32(0)], dpx_13.differential_0[i32(0)] );
    var _S58 : DiffPair_float_0 = DiffPair_float_0( dpy_6.primal_0[i32(0)], dpy_6.differential_0[i32(0)] );
    var _S59 : DiffPair_float_0 = DiffPair_float_0( dpz_0.primal_0[i32(0)], dpz_0.differential_0[i32(0)] );
    var dp_elem_0 : DiffPair_float_0 = _d_clamp_0(_S57, _S58, _S59);
    var result_0 : vec3<f32>;
    result_0[i32(0)] = dp_elem_0.primal_0;
    var d_result_0 : vec3<f32>;
    d_result_0[i32(0)] = dp_elem_0.differential_0;
    var _S60 : DiffPair_float_0 = DiffPair_float_0( dpx_13.primal_0[i32(1)], dpx_13.differential_0[i32(1)] );
    var _S61 : DiffPair_float_0 = DiffPair_float_0( dpy_6.primal_0[i32(1)], dpy_6.differential_0[i32(1)] );
    var _S62 : DiffPair_float_0 = DiffPair_float_0( dpz_0.primal_0[i32(1)], dpz_0.differential_0[i32(1)] );
    var dp_elem_1 : DiffPair_float_0 = _d_clamp_0(_S60, _S61, _S62);
    result_0[i32(1)] = dp_elem_1.primal_0;
    d_result_0[i32(1)] = dp_elem_1.differential_0;
    var _S63 : DiffPair_float_0 = DiffPair_float_0( dpx_13.primal_0[i32(2)], dpx_13.differential_0[i32(2)] );
    var _S64 : DiffPair_float_0 = DiffPair_float_0( dpy_6.primal_0[i32(2)], dpy_6.differential_0[i32(2)] );
    var _S65 : DiffPair_float_0 = DiffPair_float_0( dpz_0.primal_0[i32(2)], dpz_0.differential_0[i32(2)] );
    var dp_elem_2 : DiffPair_float_0 = _d_clamp_0(_S63, _S64, _S65);
    result_0[i32(2)] = dp_elem_2.primal_0;
    d_result_0[i32(2)] = dp_elem_2.differential_0;
    var _S66 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( result_0, d_result_0 );
    return _S66;
}

struct SigmoidCurve_0
{
     white_target_1 : f32,
     black_target_1 : f32,
     log2_paper_exposure_1 : f32,
     film_fog_1 : f32,
     film_power_1 : f32,
     paper_power_1 : f32,
     hue_preservation_1 : f32,
};

fn SigmoidCurve_x24_syn_dadd_0( SLANG_anonymous_0_0 : SigmoidCurve_0,  SLANG_anonymous_1_0 : SigmoidCurve_0) -> SigmoidCurve_0
{
    var result_1 : SigmoidCurve_0;
    result_1.white_target_1 = SLANG_anonymous_0_0.white_target_1 + SLANG_anonymous_1_0.white_target_1;
    result_1.black_target_1 = SLANG_anonymous_0_0.black_target_1 + SLANG_anonymous_1_0.black_target_1;
    result_1.log2_paper_exposure_1 = SLANG_anonymous_0_0.log2_paper_exposure_1 + SLANG_anonymous_1_0.log2_paper_exposure_1;
    result_1.film_fog_1 = SLANG_anonymous_0_0.film_fog_1 + SLANG_anonymous_1_0.film_fog_1;
    result_1.film_power_1 = SLANG_anonymous_0_0.film_power_1 + SLANG_anonymous_1_0.film_power_1;
    result_1.paper_power_1 = SLANG_anonymous_0_0.paper_power_1 + SLANG_anonymous_1_0.paper_power_1;
    result_1.hue_preservation_1 = SLANG_anonymous_0_0.hue_preservation_1 + SLANG_anonymous_1_0.hue_preservation_1;
    return result_1;
}

fn SigmoidCurve_x24_syn_dzero_0() -> SigmoidCurve_0
{
    var result_2 : SigmoidCurve_0;
    result_2.white_target_1 = 0.0f;
    result_2.black_target_1 = 0.0f;
    result_2.log2_paper_exposure_1 = 0.0f;
    result_2.film_fog_1 = 0.0f;
    result_2.film_power_1 = 0.0f;
    result_2.paper_power_1 = 0.0f;
    result_2.hue_preservation_1 = 0.0f;
    return result_2;
}

fn desaturate_negative_values_0( rgb_0 : vec3<f32>) -> vec3<f32>
{
    var _S67 : f32 = rgb_0.x;
    var _S68 : f32 = rgb_0.y;
    var _S69 : f32 = rgb_0.z;
    var _S70 : f32 = max((_S67 + _S68 + _S69) / 3.0f, 0.0f);
    var _S71 : f32 = min(_S67, min(_S68, _S69));
    var factor_0 : f32;
    if((_S71 < 0.0f))
    {
        factor_0 = - _S70 / (_S71 - _S70);
    }
    else
    {
        factor_0 = 1.0f;
    }
    var _S72 : vec3<f32> = vec3<f32>(_S70);
    return _S72 + vec3<f32>(factor_0) * (rgb_0 - _S72);
}

struct Tuple_7
{
     value0_7 : f32,
};

struct SigmoidParams_0
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
     white_target_0 : f32,
     black_target_0 : f32,
     log2_paper_exposure_0 : f32,
     film_fog_0 : f32,
     film_power_0 : f32,
     paper_power_0 : f32,
     hue_preservation_0 : f32,
     color_processing_0 : u32,
     film_overflow_0 : f32,
     film_underflow_0 : f32,
     paper_exposure_0 : u32,
     _pad0_0 : u32,
};

fn s_apply_loglogistic_0( x_7 : f32,  p_0 : SigmoidParams_0,  _S73 : SigmoidCurve_0) -> Tuple_7
{
    var film_0 : f32 = _S73.film_fog_1 + s_apply_max_0(x_7, 0.0f).value0_4;
    var _runFlag_0 : bool;
    if((film_0 >= (p_0.film_overflow_0)))
    {
        _runFlag_0 = true;
    }
    else
    {
        _runFlag_0 = (p_0.paper_exposure_0) == u32(2);
    }
    var softplus_0 : f32;
    if(_runFlag_0)
    {
        softplus_0 = _S73.white_target_1;
    }
    var _S74 : bool = !_runFlag_0;
    if(_S74)
    {
        if((film_0 <= (p_0.film_underflow_0)))
        {
            _runFlag_0 = true;
        }
        else
        {
            _runFlag_0 = (p_0.paper_exposure_0) == u32(1);
        }
        if(_runFlag_0)
        {
            _runFlag_0 = false;
            softplus_0 = 0.0f;
        }
        else
        {
            _runFlag_0 = _S74;
        }
        if(_runFlag_0)
        {
            var u_0 : f32 = _S73.log2_paper_exposure_1 - _S73.film_power_1 * s_apply_log2_0(film_0).value0_2;
            if((u_0 > 0.0f))
            {
                softplus_0 = u_0 + s_apply_log2_0(1.0f + s_apply_exp2_0(- u_0).value0_1).value0_2;
            }
            else
            {
                softplus_0 = s_apply_log2_0(1.0f + s_apply_exp2_0(u_0).value0_1).value0_2;
            }
            softplus_0 = _S73.white_target_1 * s_apply_exp2_0(- _S73.paper_power_1 * softplus_0).value0_1;
        }
    }
    var _S75 : Tuple_7;
    _S75.value0_7 = softplus_0;
    return _S75;
}

struct s_paramCtx_s_bwdCallableCtx_loglogistic_0
{
     value0_8 : f32,
     value1_0 : SigmoidParams_0,
     value2_0 : SigmoidCurve_0,
};

struct s_bwdCallableCtx_loglogistic_0
{
     value0_9 : s_paramCtx_s_bwdCallableCtx_loglogistic_0,
};

fn s_bwdProp_loglogistic_0( _S76 : s_bwdCallableCtx_loglogistic_0,  _S77 : ptr<function, SigmoidCurve_0>,  _S78 : f32)
{
    var film_1 : f32 = _S76.value0_9.value2_0.film_fog_1 + s_apply_max_0(_S76.value0_9.value0_8, 0.0f).value0_4;
    var _S79 : bool;
    if((film_1 >= (_S76.value0_9.value1_0.film_overflow_0)))
    {
        _S79 = true;
    }
    else
    {
        _S79 = (_S76.value0_9.value1_0.paper_exposure_0) == u32(2);
    }
    var _S80 : bool = !_S79;
    var _S81 : bool;
    var _runFlag_1 : bool;
    var _S82 : bool;
    var softplus_1 : f32;
    var _S83 : f32;
    var _S84 : f32;
    var _S85 : f32;
    var _S86 : f32;
    var _S87 : f32;
    var _S88 : s_bwdCallableCtx_log2_0;
    var _S89 : s_bwdCallableCtx_log2_0;
    var _S90 : s_bwdCallableCtx_log2_0;
    var _S91 : s_bwdCallableCtx_exp2_0;
    var _S92 : s_bwdCallableCtx_exp2_0;
    var _S93 : s_bwdCallableCtx_exp2_0;
    if(_S80)
    {
        if((film_1 <= (_S76.value0_9.value1_0.film_underflow_0)))
        {
            _S81 = true;
        }
        else
        {
            _S81 = (_S76.value0_9.value1_0.paper_exposure_0) == u32(1);
        }
        if(_S81)
        {
            _runFlag_1 = false;
        }
        else
        {
            _runFlag_1 = _S80;
        }
        if(_runFlag_1)
        {
            var _S94 : Tuple_2 = s_apply_log2_0(film_1);
            var _S95 : s_bwdCallableCtx_log2_0;
            _S95._S11 = film_1;
            var _S96 : s_bwdCallableCtx_log2_0 = _S95;
            var u_1 : f32 = _S76.value0_9.value2_0.log2_paper_exposure_1 - _S76.value0_9.value2_0.film_power_1 * _S94.value0_2;
            var _S97 : bool = (u_1 > 0.0f);
            if(_S97)
            {
                var _S98 : f32 = - u_1;
                var _S99 : Tuple_1 = s_apply_exp2_0(_S98);
                var _S100 : s_bwdCallableCtx_exp2_0;
                _S100._S7 = _S98;
                var _S101 : f32 = 1.0f + _S99.value0_1;
                var _S102 : Tuple_2 = s_apply_log2_0(_S101);
                var _S103 : s_bwdCallableCtx_log2_0;
                _S103._S11 = _S101;
                softplus_1 = u_1 + _S102.value0_2;
                _S88 = _S103;
                _S91 = _S100;
                _S89._S11 = 0.0f;
                _S92._S7 = 0.0f;
            }
            else
            {
                var _S104 : Tuple_1 = s_apply_exp2_0(u_1);
                var _S105 : s_bwdCallableCtx_exp2_0;
                _S105._S7 = u_1;
                var _S106 : f32 = 1.0f + _S104.value0_1;
                var _S107 : Tuple_2 = s_apply_log2_0(_S106);
                var _S108 : s_bwdCallableCtx_log2_0;
                _S108._S11 = _S106;
                softplus_1 = _S107.value0_2;
                _S88._S11 = 0.0f;
                _S91._S7 = 0.0f;
                _S89 = _S108;
                _S92 = _S105;
            }
            var _S109 : f32 = - _S76.value0_9.value2_0.paper_power_1;
            var _S110 : f32 = _S109 * softplus_1;
            var _S111 : Tuple_1 = s_apply_exp2_0(_S110);
            var _S112 : s_bwdCallableCtx_exp2_0;
            _S112._S7 = _S110;
            var _S113 : f32 = softplus_1;
            var _S114 : s_bwdCallableCtx_exp2_0 = _S91;
            var _S115 : s_bwdCallableCtx_exp2_0 = _S92;
            softplus_1 = _S76.value0_9.value2_0.white_target_1;
            _S83 = _S111.value0_1;
            _S91 = _S112;
            _S84 = _S109;
            _S85 = _S113;
            _S82 = _S97;
            _S92 = _S114;
            _S93 = _S115;
            _S86 = _S76.value0_9.value2_0.film_power_1;
            _S87 = _S94.value0_2;
            _S90 = _S96;
        }
        else
        {
            softplus_1 = 0.0f;
            _S83 = 0.0f;
            _S91._S7 = 0.0f;
            _S84 = 0.0f;
            _S85 = 0.0f;
            _S82 = false;
            _S88._S11 = 0.0f;
            _S92._S7 = 0.0f;
            _S89._S11 = 0.0f;
            _S93._S7 = 0.0f;
            _S86 = 0.0f;
            _S87 = 0.0f;
            _S90._S11 = 0.0f;
        }
        var _S116 : bool = _S81;
        _S81 = _runFlag_1;
        _runFlag_1 = _S82;
        _S82 = _S116;
    }
    else
    {
        _S81 = false;
        softplus_1 = 0.0f;
        _S83 = 0.0f;
        _S91._S7 = 0.0f;
        _S84 = 0.0f;
        _S85 = 0.0f;
        _runFlag_1 = false;
        _S88._S11 = 0.0f;
        _S92._S7 = 0.0f;
        _S89._S11 = 0.0f;
        _S93._S7 = 0.0f;
        _S86 = 0.0f;
        _S87 = 0.0f;
        _S90._S11 = 0.0f;
        _S82 = false;
    }
    var _S117 : SigmoidCurve_0 = SigmoidCurve_x24_syn_dzero_0();
    var _S118 : SigmoidCurve_0;
    if(_S80)
    {
        if(_S81)
        {
            var _S119 : f32 = softplus_1 * _S78;
            var _S120 : f32 = _S83 * _S78;
            var _S121 : f32 = 0.0f;
            s_bwdProp_exp2_0(_S91, &(_S121), _S119);
            var _S122 : f32 = _S84 * _S121;
            var _S123 : f32 = - (_S85 * _S121);
            var _S124 : SigmoidCurve_0 = _S117;
            _S124.paper_power_1 = _S123;
            _S124.white_target_1 = _S120;
            var _S125 : SigmoidCurve_0 = SigmoidCurve_x24_syn_dadd_0(_S117, _S124);
            if(_runFlag_1)
            {
                var _S126 : f32 = 0.0f;
                s_bwdProp_log2_0(_S88, &(_S126), _S122);
                var _S127 : f32 = 0.0f;
                s_bwdProp_exp2_0(_S92, &(_S127), _S126);
                softplus_1 = _S122 + - _S127;
            }
            else
            {
                var _S128 : f32 = 0.0f;
                s_bwdProp_log2_0(_S89, &(_S128), _S122);
                var _S129 : f32 = 0.0f;
                s_bwdProp_exp2_0(_S93, &(_S129), _S128);
                softplus_1 = _S129;
            }
            var _S130 : f32 = - softplus_1;
            var _S131 : f32 = _S86 * _S130;
            var _S132 : f32 = _S87 * _S130;
            var _S133 : f32 = 0.0f;
            s_bwdProp_log2_0(_S90, &(_S133), _S131);
            var _S134 : SigmoidCurve_0 = _S117;
            _S134.film_power_1 = _S132;
            _S134.log2_paper_exposure_1 = softplus_1;
            var _S135 : SigmoidCurve_0 = SigmoidCurve_x24_syn_dadd_0(_S125, _S134);
            softplus_1 = 0.0f;
            _S118 = _S135;
            _S83 = _S133;
        }
        else
        {
            softplus_1 = _S78;
            _S118 = _S117;
            _S83 = 0.0f;
        }
        if(_S82)
        {
            softplus_1 = 0.0f;
        }
    }
    else
    {
        softplus_1 = _S78;
        _S118 = _S117;
        _S83 = 0.0f;
    }
    if(_S79)
    {
        var _S136 : SigmoidCurve_0 = _S117;
        _S136.white_target_1 = softplus_1;
        _S118 = SigmoidCurve_x24_syn_dadd_0(_S118, _S136);
    }
    var _S137 : SigmoidCurve_0 = _S117;
    _S137.film_fog_1 = _S83;
    (*_S77) = SigmoidCurve_x24_syn_dadd_0(_S118, _S137);
    return;
}

struct s_paramCtx_s_bwdCallableCtx_rgb_ratio_0
{
     value0_10 : vec3<f32>,
     value1_1 : SigmoidParams_0,
     value2_1 : SigmoidCurve_0,
};

struct s_bwdCallableCtx_rgb_ratio_0
{
     value0_11 : s_paramCtx_s_bwdCallableCtx_rgb_ratio_0,
};

fn s_bwdProp_rgb_ratio_0( _S138 : s_bwdCallableCtx_rgb_ratio_0,  _S139 : ptr<function, SigmoidCurve_0>,  _S140 : vec3<f32>)
{
    var positive_0 : vec3<f32> = desaturate_negative_values_0(_S138.value0_11.value0_10);
    var luma_0 : f32 = (positive_0.x + positive_0.y + positive_0.z) / 3.0f;
    var _S141 : Tuple_7 = s_apply_loglogistic_0(luma_0, _S138.value0_11.value1_1, _S138.value0_11.value2_1);
    var _S142 : s_paramCtx_s_bwdCallableCtx_loglogistic_0;
    _S142.value0_8 = luma_0;
    _S142.value1_0 = _S138.value0_11.value1_1;
    _S142.value2_0 = _S138.value0_11.value2_1;
    var _S143 : s_bwdCallableCtx_loglogistic_0 = s_bwdCallableCtx_loglogistic_0( _S142 );
    var _S144 : bool = (luma_0 > 9.99999971718068537e-10f);
    var scaled_0 : vec3<f32>;
    if(_S144)
    {
        scaled_0 = vec3<f32>((_S141.value0_7 / luma_0)) * positive_0;
    }
    else
    {
        scaled_0 = vec3<f32>(_S141.value0_7);
    }
    var _S145 : f32 = scaled_0.x;
    var _S146 : f32 = scaled_0.y;
    var _S147 : f32 = scaled_0.z;
    var _S148 : Tuple_5 = s_apply_min_0(_S146, _S147);
    var _S149 : s_bwdCallableCtx_min_0;
    _S149._S34 = _S146;
    _S149._S35 = _S147;
    var _S150 : s_bwdCallableCtx_min_0 = _S149;
    var _S151 : Tuple_5 = s_apply_min_0(_S145, _S148.value0_5);
    var _S152 : s_bwdCallableCtx_min_0;
    _S152._S34 = _S145;
    _S152._S35 = _S148.value0_5;
    var _S153 : s_bwdCallableCtx_min_0 = _S152;
    var _S154 : Tuple_4 = s_apply_max_0(_S146, _S147);
    var _S155 : s_bwdCallableCtx_max_0;
    _S155._S26 = _S146;
    _S155._S27 = _S147;
    var _S156 : s_bwdCallableCtx_max_0 = _S155;
    var _S157 : Tuple_4 = s_apply_max_0(_S145, _S154.value0_4);
    var _S158 : s_bwdCallableCtx_max_0;
    _S158._S26 = _S145;
    _S158._S27 = _S154.value0_4;
    var _S159 : s_bwdCallableCtx_max_0 = _S158;
    var _S160 : f32 = _S138.value0_11.value2_1.white_target_1 - _S141.value0_7;
    var _S161 : f32 = _S157.value0_4 - _S141.value0_7 + 9.99999997475242708e-07f;
    var display_border_vs_chroma_white_0 : f32 = _S160 / _S161;
    var _S162 : f32 = _S161 * _S161;
    var _S163 : f32 = _S138.value0_11.value2_1.black_target_1 - _S141.value0_7;
    var _S164 : f32 = _S151.value0_5 - _S141.value0_7 - 9.99999997475242708e-07f;
    var display_border_vs_chroma_black_0 : f32 = _S163 / _S164;
    var _S165 : f32 = _S164 * _S164;
    var _S166 : Tuple_5 = s_apply_min_0(display_border_vs_chroma_white_0, display_border_vs_chroma_black_0);
    var _S167 : s_bwdCallableCtx_min_0;
    _S167._S34 = display_border_vs_chroma_white_0;
    _S167._S35 = display_border_vs_chroma_black_0;
    var _S168 : s_bwdCallableCtx_min_0 = _S167;
    var _S169 : f32 = _S141.value0_7 - _S151.value0_5;
    var _S170 : f32 = _S141.value0_7 + 9.99999997475242708e-07f;
    var chroma_vs_mapping_border_0 : f32 = _S169 / _S170;
    var _S171 : f32 = _S170 * _S170;
    var _S172 : f32 = chroma_vs_mapping_border_0 * _S166.value0_5 + 9.99999997475242708e-07f;
    var pixel_chroma_adjustment_0 : f32 = 1.0f / _S172;
    var _S173 : f32 = _S172 * _S172;
    var _S174 : f32 = 2.0f * chroma_vs_mapping_border_0;
    var _S175 : f32 = 1.0f - chroma_vs_mapping_border_0 * chroma_vs_mapping_border_0 + 9.99999997475242708e-07f;
    var _S176 : f32 = _S174 / _S175;
    var _S177 : f32 = _S175 * _S175;
    var hyperbolic_chroma_0 : f32 = _S176 * pixel_chroma_adjustment_0;
    var _S178 : f32 = hyperbolic_chroma_0 * hyperbolic_chroma_0 + 1.0f;
    var _S179 : Tuple_3 = s_apply_sqrt_0(_S178);
    var _S180 : s_bwdCallableCtx_sqrt_0;
    _S180._S22 = _S178;
    var _S181 : s_bwdCallableCtx_sqrt_0 = _S180;
    var _S182 : f32 = 1.0f + _S179.value0_3;
    var _S183 : f32 = hyperbolic_chroma_0 / _S182;
    var _S184 : f32 = _S182 * _S182;
    var chroma_factor_0 : f32 = _S183 * _S166.value0_5;
    var _S185 : vec3<f32> = scaled_0 - vec3<f32>(_S141.value0_7);
    var _S186 : SigmoidCurve_0 = SigmoidCurve_x24_syn_dzero_0();
    var _S187 : f32 = _S140[i32(0)] + _S140[i32(1)] + _S140[i32(2)];
    var _S188 : vec3<f32> = vec3<f32>(chroma_factor_0) * _S140;
    var _S189 : vec3<f32> = _S185 * _S140;
    var _S190 : f32 = _S189[i32(0)] + _S189[i32(1)] + _S189[i32(2)];
    var _S191 : vec3<f32> = (vec3<f32>(0) - _S188);
    var _S192 : f32 = _S191[i32(0)] + _S191[i32(1)] + _S191[i32(2)];
    var _S193 : f32 = _S183 * _S190;
    var _S194 : f32 = _S166.value0_5 * _S190 / _S184;
    var _S195 : f32 = hyperbolic_chroma_0 * - _S194;
    var _S196 : f32 = _S182 * _S194;
    var _S197 : f32 = 0.0f;
    s_bwdProp_sqrt_0(_S181, &(_S197), _S195);
    var _S198 : f32 = hyperbolic_chroma_0 * _S197;
    var _S199 : f32 = _S196 + _S198 + _S198;
    var _S200 : f32 = pixel_chroma_adjustment_0 * _S199 / _S177;
    var _S201 : f32 = chroma_vs_mapping_border_0 * - (_S174 * - _S200);
    var _S202 : f32 = - (_S176 * _S199 / _S173);
    var _S203 : f32 = (_S201 + _S201 + 2.0f * (_S175 * _S200) + _S166.value0_5 * _S202) / _S171;
    var _S204 : f32 = _S169 * - _S203;
    var _S205 : f32 = _S170 * _S203;
    var _S206 : f32 = - _S205;
    var _S207 : f32 = _S193 + chroma_vs_mapping_border_0 * _S202;
    var _S208 : f32 = 0.0f;
    var _S209 : f32 = 0.0f;
    s_bwdProp_min_0(_S168, &(_S208), &(_S209), _S207);
    var _S210 : f32 = _S209 / _S165;
    var _S211 : f32 = _S163 * - _S210;
    var _S212 : f32 = _S164 * _S210;
    var _S213 : f32 = - _S211;
    var _S214 : f32 = - _S212;
    var _S215 : f32 = _S208 / _S162;
    var _S216 : f32 = _S160 * - _S215;
    var _S217 : f32 = _S161 * _S215;
    var _S218 : f32 = - _S216;
    var _S219 : f32 = - _S217;
    var _S220 : f32 = 0.0f;
    var _S221 : f32 = 0.0f;
    s_bwdProp_max_0(_S159, &(_S220), &(_S221), _S216);
    var _S222 : f32 = 0.0f;
    var _S223 : f32 = 0.0f;
    s_bwdProp_max_0(_S156, &(_S222), &(_S223), _S221);
    var _S224 : f32 = _S206 + _S211;
    var _S225 : f32 = 0.0f;
    var _S226 : f32 = 0.0f;
    s_bwdProp_min_0(_S153, &(_S225), &(_S226), _S224);
    var _S227 : f32 = 0.0f;
    var _S228 : f32 = 0.0f;
    s_bwdProp_min_0(_S150, &(_S227), &(_S228), _S226);
    var _S229 : vec3<f32> = _S188 + vec3<f32>(_S220 + _S225, _S222 + _S227, _S223 + _S228);
    var _S230 : f32 = _S187 + _S192 + _S204 + _S205 + _S213 + _S214 + _S218 + _S219;
    var _S231 : SigmoidCurve_0 = _S186;
    _S231.black_target_1 = _S212;
    _S231.white_target_1 = _S217;
    var _S232 : SigmoidCurve_0 = SigmoidCurve_x24_syn_dadd_0(_S186, _S231);
    var _S233 : f32;
    if(_S144)
    {
        var _S234 : vec3<f32> = positive_0 * _S229;
        _S233 = (_S234[i32(0)] + _S234[i32(1)] + _S234[i32(2)]) / luma_0 + _S230;
    }
    else
    {
        _S233 = _S229[i32(0)] + _S229[i32(1)] + _S229[i32(2)] + _S230;
    }
    var _S235 : SigmoidCurve_0;
    _S235.white_target_1 = 0.0f;
    _S235.black_target_1 = 0.0f;
    _S235.log2_paper_exposure_1 = 0.0f;
    _S235.film_fog_1 = 0.0f;
    _S235.film_power_1 = 0.0f;
    _S235.paper_power_1 = 0.0f;
    _S235.hue_preservation_1 = 0.0f;
    s_bwdProp_loglogistic_0(_S143, &(_S235), _S233);
    (*_S139) = SigmoidCurve_x24_syn_dadd_0(_S235, _S232);
    return;
}

fn to_rendering_0( rgb_1 : vec3<f32>,  p_1 : SigmoidParams_0) -> vec3<f32>
{
    return (((desaturate_negative_values_0((((rgb_1) * (mat3x3<f32>(p_1.pipe_to_base_r0_0.xyz, p_1.pipe_to_base_r1_0.xyz, p_1.pipe_to_base_r2_0.xyz)))))) * (mat3x3<f32>(p_1.base_to_rendering_r0_0.xyz, p_1.base_to_rendering_r1_0.xyz, p_1.base_to_rendering_r2_0.xyz))));
}

fn one_hot_0( i_0 : u32) -> vec3<f32>
{
    var _S236 : f32;
    if(i_0 == u32(0))
    {
        _S236 = 1.0f;
    }
    else
    {
        _S236 = 0.0f;
    }
    var _S237 : f32;
    if(i_0 == u32(1))
    {
        _S237 = 1.0f;
    }
    else
    {
        _S237 = 0.0f;
    }
    var _S238 : f32;
    if(i_0 == u32(2))
    {
        _S238 = 1.0f;
    }
    else
    {
        _S238 = 0.0f;
    }
    return vec3<f32>(_S236, _S237, _S238);
}

struct HueOrder_0
{
     min_0 : vec3<f32>,
     mid_0 : vec3<f32>,
     max_0 : vec3<f32>,
     midscale_0 : f32,
     blend_0 : f32,
};

fn hue_order_0( v_0 : vec3<f32>) -> HueOrder_0
{
    var _S239 : f32 = v_0.x;
    var _S240 : f32 = v_0.y;
    var order_0 : vec3<u32>;
    if((_S239 >= _S240))
    {
        var _S241 : f32 = v_0.z;
        if((_S240 > _S241))
        {
            order_0 = vec3<u32>(u32(2), u32(1), u32(0));
        }
        else
        {
            if((_S241 > _S239))
            {
                order_0 = vec3<u32>(u32(1), u32(0), u32(2));
            }
            else
            {
                if((_S241 > _S240))
                {
                    order_0 = vec3<u32>(u32(1), u32(2), u32(0));
                }
                else
                {
                    order_0 = vec3<u32>(u32(2), u32(1), u32(0));
                }
            }
        }
    }
    else
    {
        var _S242 : f32 = v_0.z;
        if((_S239 >= _S242))
        {
            order_0 = vec3<u32>(u32(2), u32(0), u32(1));
        }
        else
        {
            if((_S242 > _S240))
            {
                order_0 = vec3<u32>(u32(0), u32(1), u32(2));
            }
            else
            {
                order_0 = vec3<u32>(u32(0), u32(2), u32(1));
            }
        }
    }
    var o_0 : HueOrder_0;
    var _S243 : vec3<f32> = one_hot_0(order_0.x);
    o_0.min_0 = _S243;
    var _S244 : vec3<f32> = one_hot_0(order_0.y);
    o_0.mid_0 = _S244;
    var _S245 : vec3<f32> = one_hot_0(order_0.z);
    o_0.max_0 = _S245;
    var low_0 : f32 = dot(v_0, _S243);
    var middle_0 : f32 = dot(v_0, _S244);
    var chroma_0 : f32 = dot(v_0, _S245) - low_0;
    var _S246 : f32;
    if(chroma_0 != 0.0f)
    {
        _S246 = (middle_0 - low_0) / chroma_0;
    }
    else
    {
        _S246 = 0.0f;
    }
    o_0.midscale_0 = _S246;
    var low_plus_middle_0 : f32 = low_0 + middle_0;
    if(low_plus_middle_0 != 0.0f)
    {
        _S246 = 2.0f * low_0 / low_plus_middle_0;
    }
    else
    {
        _S246 = 0.0f;
    }
    o_0.blend_0 = _S246;
    return o_0;
}

fn preserve_hue_and_energy_0( per_channel_0 : vec3<f32>,  o_1 : HueOrder_0,  hue_preservation_2 : f32) -> vec3<f32>
{
    var low_1 : f32 = dot(per_channel_0, o_1.min_0);
    var middle_1 : f32 = dot(per_channel_0, o_1.mid_0);
    var high_0 : f32 = dot(per_channel_0, o_1.max_0);
    var _S247 : f32 = (1.0f - hue_preservation_2) * middle_1;
    var naive_hue_mid_0 : f32 = _S247 + hue_preservation_2 * (low_1 + (high_0 - low_1) * o_1.midscale_0);
    var energy_target_0 : f32 = o_1.blend_0 * (per_channel_0.x + per_channel_0.y + per_channel_0.z) + (1.0f - o_1.blend_0) * (low_1 + naive_hue_mid_0 + high_0);
    var new_low_0 : f32;
    var new_mid_0 : f32;
    var new_high_0 : f32;
    if((naive_hue_mid_0 <= middle_1))
    {
        var _S248 : f32 = 1.0f - o_1.midscale_0;
        var _S249 : f32 = energy_target_0 - high_0;
        var new_mid_1 : f32 = (_S247 + hue_preservation_2 * (o_1.midscale_0 * high_0 + _S248 * _S249)) / (1.0f + hue_preservation_2 * _S248);
        new_low_0 = _S249 - new_mid_1;
        new_mid_0 = new_mid_1;
        new_high_0 = high_0;
    }
    else
    {
        var _S250 : f32 = energy_target_0 - low_1;
        var new_mid_2 : f32 = (_S247 + hue_preservation_2 * (low_1 * (1.0f - o_1.midscale_0) + o_1.midscale_0 * _S250)) / (1.0f + hue_preservation_2 * o_1.midscale_0);
        var _S251 : f32 = _S250 - new_mid_2;
        new_low_0 = low_1;
        new_mid_0 = new_mid_2;
        new_high_0 = _S251;
    }
    return o_1.min_0 * vec3<f32>(new_low_0) + o_1.mid_0 * vec3<f32>(new_mid_0) + o_1.max_0 * vec3<f32>(new_high_0);
}

struct Tuple_8
{
     value0_12 : vec3<f32>,
};

fn s_apply_preserve_hue_and_energy_0( _S252 : vec3<f32>,  o_2 : HueOrder_0,  _S253 : f32) -> Tuple_8
{
    var _S254 : Tuple_0 = s_apply_dot_0(_S252, o_2.min_0);
    var _S255 : Tuple_0 = s_apply_dot_0(_S252, o_2.mid_0);
    var _S256 : Tuple_0 = s_apply_dot_0(_S252, o_2.max_0);
    var _S257 : f32 = (1.0f - _S253) * _S255.value0_0;
    var naive_hue_mid_1 : f32 = _S257 + _S253 * (_S254.value0_0 + (_S256.value0_0 - _S254.value0_0) * o_2.midscale_0);
    var energy_target_1 : f32 = o_2.blend_0 * (_S252.x + _S252.y + _S252.z) + (1.0f - o_2.blend_0) * (_S254.value0_0 + naive_hue_mid_1 + _S256.value0_0);
    var new_low_1 : f32;
    var new_mid_3 : f32;
    var new_high_1 : f32;
    if((naive_hue_mid_1 <= (_S255.value0_0)))
    {
        var _S258 : f32 = 1.0f - o_2.midscale_0;
        var _S259 : f32 = energy_target_1 - _S256.value0_0;
        var new_mid_4 : f32 = (_S257 + _S253 * (o_2.midscale_0 * _S256.value0_0 + _S258 * _S259)) / (1.0f + _S253 * _S258);
        new_low_1 = _S259 - new_mid_4;
        new_mid_3 = new_mid_4;
        new_high_1 = _S256.value0_0;
    }
    else
    {
        var _S260 : f32 = energy_target_1 - _S254.value0_0;
        var new_mid_5 : f32 = (_S257 + _S253 * (_S254.value0_0 * (1.0f - o_2.midscale_0) + o_2.midscale_0 * _S260)) / (1.0f + _S253 * o_2.midscale_0);
        var _S261 : f32 = _S260 - new_mid_5;
        new_low_1 = _S254.value0_0;
        new_mid_3 = new_mid_5;
        new_high_1 = _S261;
    }
    var _S262 : Tuple_8;
    _S262.value0_12 = o_2.min_0 * vec3<f32>(new_low_1) + o_2.mid_0 * vec3<f32>(new_mid_3) + o_2.max_0 * vec3<f32>(new_high_1);
    return _S262;
}

struct s_paramCtx_s_bwdCallableCtx_preserve_hue_and_energy_0
{
     value0_13 : vec3<f32>,
     value1_2 : HueOrder_0,
     value2_2 : f32,
};

struct s_bwdCallableCtx_preserve_hue_and_energy_0
{
     value0_14 : s_paramCtx_s_bwdCallableCtx_preserve_hue_and_energy_0,
};

fn s_bwdProp_preserve_hue_and_energy_0( _S263 : s_bwdCallableCtx_preserve_hue_and_energy_0,  _S264 : ptr<function, vec3<f32>>,  _S265 : ptr<function, f32>,  _S266 : vec3<f32>)
{
    var _S267 : Tuple_0 = s_apply_dot_0(_S263.value0_14.value0_13, _S263.value0_14.value1_2.min_0);
    var _S268 : s_bwdCallableCtx_dot_0;
    _S268._S2 = _S263.value0_14.value0_13;
    _S268._S3 = _S263.value0_14.value1_2.min_0;
    var _S269 : s_bwdCallableCtx_dot_0 = _S268;
    var _S270 : Tuple_0 = s_apply_dot_0(_S263.value0_14.value0_13, _S263.value0_14.value1_2.mid_0);
    var _S271 : s_bwdCallableCtx_dot_0;
    _S271._S2 = _S263.value0_14.value0_13;
    _S271._S3 = _S263.value0_14.value1_2.mid_0;
    var _S272 : s_bwdCallableCtx_dot_0 = _S271;
    var _S273 : Tuple_0 = s_apply_dot_0(_S263.value0_14.value0_13, _S263.value0_14.value1_2.max_0);
    var _S274 : s_bwdCallableCtx_dot_0;
    _S274._S2 = _S263.value0_14.value0_13;
    _S274._S3 = _S263.value0_14.value1_2.max_0;
    var _S275 : s_bwdCallableCtx_dot_0 = _S274;
    var full_hue_correction_0 : f32 = _S267.value0_0 + (_S273.value0_0 - _S267.value0_0) * _S263.value0_14.value1_2.midscale_0;
    var _S276 : f32 = 1.0f - _S263.value0_14.value2_2;
    var _S277 : f32 = _S276 * _S270.value0_0;
    var naive_hue_mid_2 : f32 = _S277 + _S263.value0_14.value2_2 * full_hue_correction_0;
    var _S278 : f32 = 1.0f - _S263.value0_14.value1_2.blend_0;
    var energy_target_2 : f32 = _S263.value0_14.value1_2.blend_0 * (_S263.value0_14.value0_13.x + _S263.value0_14.value0_13.y + _S263.value0_14.value0_13.z) + _S278 * (_S267.value0_0 + naive_hue_mid_2 + _S273.value0_0);
    var _S279 : bool = (naive_hue_mid_2 <= (_S270.value0_0));
    var _S280 : f32;
    var _S281 : f32;
    var _S282 : f32;
    var _S283 : f32;
    var _S284 : f32;
    var _S285 : f32;
    var _S286 : f32;
    var _S287 : f32;
    var _S288 : f32;
    var _S289 : f32;
    if(_S279)
    {
        var _S290 : f32 = 1.0f - _S263.value0_14.value1_2.midscale_0;
        var _S291 : f32 = _S263.value0_14.value1_2.midscale_0 * _S273.value0_0 + _S290 * (energy_target_2 - _S273.value0_0);
        var _S292 : f32 = _S277 + _S263.value0_14.value2_2 * _S291;
        var _S293 : f32 = 1.0f + _S263.value0_14.value2_2 * _S290;
        _S280 = _S293 * _S293;
        _S281 = _S292;
        _S282 = _S293;
        _S283 = _S290;
        _S284 = _S291;
        _S285 = 0.0f;
        _S286 = 0.0f;
        _S287 = 0.0f;
        _S288 = 0.0f;
        _S289 = 0.0f;
    }
    else
    {
        var _S294 : f32 = 1.0f - _S263.value0_14.value1_2.midscale_0;
        var _S295 : f32 = _S267.value0_0 * _S294 + _S263.value0_14.value1_2.midscale_0 * (energy_target_2 - _S267.value0_0);
        var _S296 : f32 = _S277 + _S263.value0_14.value2_2 * _S295;
        var _S297 : f32 = 1.0f + _S263.value0_14.value2_2 * _S263.value0_14.value1_2.midscale_0;
        var _S298 : f32 = _S297 * _S297;
        _S280 = 0.0f;
        _S281 = 0.0f;
        _S282 = 0.0f;
        _S283 = 0.0f;
        _S284 = 0.0f;
        _S285 = _S298;
        _S286 = _S296;
        _S287 = _S297;
        _S288 = _S295;
        _S289 = _S294;
    }
    var _S299 : vec3<f32> = _S263.value0_14.value1_2.max_0 * _S266;
    var _S300 : f32 = _S299[i32(0)] + _S299[i32(1)] + _S299[i32(2)];
    var _S301 : vec3<f32> = _S263.value0_14.value1_2.mid_0 * _S266;
    var _S302 : f32 = _S301[i32(0)] + _S301[i32(1)] + _S301[i32(2)];
    var _S303 : vec3<f32> = _S263.value0_14.value1_2.min_0 * _S266;
    var _S304 : f32 = _S303[i32(0)] + _S303[i32(1)] + _S303[i32(2)];
    if(_S279)
    {
        var _S305 : f32 = (_S302 + - _S304) / _S280;
        var _S306 : f32 = _S282 * _S305;
        var _S307 : f32 = _S263.value0_14.value2_2 * _S306;
        var _S308 : f32 = _S304 + _S283 * _S307;
        var _S309 : f32 = _S300 + - _S308 + _S263.value0_14.value1_2.midscale_0 * _S307;
        var _S310 : f32 = _S283 * (_S281 * - _S305) + _S284 * _S306;
        _S280 = _S308;
        _S281 = _S306;
        _S282 = _S309;
        _S283 = 0.0f;
        _S284 = _S310;
    }
    else
    {
        var _S311 : f32 = (_S302 + - _S300) / _S285;
        var _S312 : f32 = _S287 * _S311;
        var _S313 : f32 = _S263.value0_14.value2_2 * _S312;
        var _S314 : f32 = _S300 + _S263.value0_14.value1_2.midscale_0 * _S313;
        var _S315 : f32 = _S304 + - _S314 + _S289 * _S313;
        var _S316 : f32 = _S263.value0_14.value1_2.midscale_0 * (_S286 * - _S311) + _S288 * _S312;
        _S280 = _S314;
        _S281 = _S312;
        _S282 = 0.0f;
        _S283 = _S315;
        _S284 = _S316;
    }
    var s_diff_naive_hue_mid_T_0 : f32 = _S278 * _S280;
    var s_diff_per_channel_energy_T_0 : f32 = _S263.value0_14.value1_2.blend_0 * _S280;
    var s_diff_full_hue_correction_T_0 : f32 = _S263.value0_14.value2_2 * s_diff_naive_hue_mid_T_0;
    var _S317 : f32 = full_hue_correction_0 * s_diff_naive_hue_mid_T_0;
    var _S318 : f32 = s_diff_naive_hue_mid_T_0 + _S281;
    var _S319 : f32 = _S276 * _S318;
    var _S320 : f32 = - (_S270.value0_0 * _S318);
    var _S321 : f32 = _S263.value0_14.value1_2.midscale_0 * s_diff_full_hue_correction_T_0;
    var _S322 : f32 = - _S321;
    var _S323 : f32 = s_diff_naive_hue_mid_T_0 + _S321 + _S282;
    var _S324 : vec3<f32> = vec3<f32>(0.0f);
    var _S325 : vec3<f32> = _S324;
    var _S326 : vec3<f32> = _S324;
    s_bwdProp_dot_0(_S275, &(_S325), &(_S326), _S323);
    var _S327 : vec3<f32> = _S324;
    var _S328 : vec3<f32> = _S324;
    s_bwdProp_dot_0(_S272, &(_S327), &(_S328), _S319);
    var _S329 : f32 = s_diff_naive_hue_mid_T_0 + s_diff_full_hue_correction_T_0 + _S322 + _S283;
    var _S330 : vec3<f32> = _S324;
    var _S331 : vec3<f32> = _S324;
    s_bwdProp_dot_0(_S269, &(_S330), &(_S331), _S329);
    var _S332 : f32 = _S317 + _S320 + _S284;
    (*_S264) = _S325 + _S327 + _S330 + vec3<f32>(s_diff_per_channel_energy_T_0, s_diff_per_channel_energy_T_0, s_diff_per_channel_energy_T_0);
    (*_S265) = _S332;
    return;
}

fn s_fwd_preserve_hue_and_energy_0( dpper_channel_0 : DiffPair_vectorx3Cfloatx2C3x3E_0,  o_3 : HueOrder_0,  dphue_preservation_0 : DiffPair_float_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S333 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( dpper_channel_0.primal_0, dpper_channel_0.differential_0 );
    var _S334 : vec3<f32> = vec3<f32>(0.0f);
    var _S335 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( o_3.min_0, _S334 );
    var _S336 : DiffPair_float_0 = _d_dot_1(_S333, _S335);
    var _S337 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( o_3.mid_0, _S334 );
    var _S338 : DiffPair_float_0 = _d_dot_1(_S333, _S337);
    var _S339 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( o_3.max_0, _S334 );
    var _S340 : DiffPair_float_0 = _d_dot_1(_S333, _S339);
    var full_hue_correction_1 : f32 = _S336.primal_0 + (_S340.primal_0 - _S336.primal_0) * o_3.midscale_0;
    var _S341 : f32 = 1.0f - dphue_preservation_0.primal_0;
    var _S342 : f32 = _S341 * _S338.primal_0;
    var _S343 : f32 = (0.0f - dphue_preservation_0.differential_0) * _S338.primal_0 + _S338.differential_0 * _S341;
    var naive_hue_mid_3 : f32 = _S342 + dphue_preservation_0.primal_0 * full_hue_correction_1;
    var _S344 : f32 = 1.0f - o_3.blend_0;
    var energy_target_3 : f32 = o_3.blend_0 * (dpper_channel_0.primal_0.x + dpper_channel_0.primal_0.y + dpper_channel_0.primal_0.z) + _S344 * (_S336.primal_0 + naive_hue_mid_3 + _S340.primal_0);
    var s_diff_energy_target_0 : f32 = (dpper_channel_0.differential_0.x + dpper_channel_0.differential_0.y + dpper_channel_0.differential_0.z) * o_3.blend_0 + (_S336.differential_0 + (_S343 + (dphue_preservation_0.differential_0 * full_hue_correction_1 + (_S336.differential_0 + (_S340.differential_0 - _S336.differential_0) * o_3.midscale_0) * dphue_preservation_0.primal_0)) + _S340.differential_0) * _S344;
    var new_low_2 : f32;
    var s_diff_new_low_0 : f32;
    var new_mid_6 : f32;
    var s_diff_new_mid_0 : f32;
    var new_high_2 : f32;
    var s_diff_new_high_0 : f32;
    if((naive_hue_mid_3 <= (_S338.primal_0)))
    {
        var _S345 : f32 = 1.0f - o_3.midscale_0;
        var _S346 : f32 = energy_target_3 - _S340.primal_0;
        var _S347 : f32 = s_diff_energy_target_0 - _S340.differential_0;
        var _S348 : f32 = o_3.midscale_0 * _S340.primal_0 + _S345 * _S346;
        var _S349 : f32 = _S342 + dphue_preservation_0.primal_0 * _S348;
        var _S350 : f32 = 1.0f + dphue_preservation_0.primal_0 * _S345;
        var new_mid_7 : f32 = _S349 / _S350;
        var s_diff_new_mid_1 : f32 = ((_S343 + (dphue_preservation_0.differential_0 * _S348 + (_S340.differential_0 * o_3.midscale_0 + _S347 * _S345) * dphue_preservation_0.primal_0)) * _S350 - _S349 * (dphue_preservation_0.differential_0 * _S345)) / (_S350 * _S350);
        var _S351 : f32 = _S347 - s_diff_new_mid_1;
        new_low_2 = _S346 - new_mid_7;
        s_diff_new_low_0 = _S351;
        new_mid_6 = new_mid_7;
        s_diff_new_mid_0 = s_diff_new_mid_1;
        new_high_2 = _S340.primal_0;
        s_diff_new_high_0 = _S340.differential_0;
    }
    else
    {
        var _S352 : f32 = 1.0f - o_3.midscale_0;
        var _S353 : f32 = energy_target_3 - _S336.primal_0;
        var _S354 : f32 = s_diff_energy_target_0 - _S336.differential_0;
        var _S355 : f32 = _S336.primal_0 * _S352 + o_3.midscale_0 * _S353;
        var _S356 : f32 = _S342 + dphue_preservation_0.primal_0 * _S355;
        var _S357 : f32 = 1.0f + dphue_preservation_0.primal_0 * o_3.midscale_0;
        var new_mid_8 : f32 = _S356 / _S357;
        var s_diff_new_mid_2 : f32 = ((_S343 + (dphue_preservation_0.differential_0 * _S355 + (_S336.differential_0 * _S352 + _S354 * o_3.midscale_0) * dphue_preservation_0.primal_0)) * _S357 - _S356 * (dphue_preservation_0.differential_0 * o_3.midscale_0)) / (_S357 * _S357);
        var _S358 : f32 = _S353 - new_mid_8;
        var _S359 : f32 = _S354 - s_diff_new_mid_2;
        new_low_2 = _S336.primal_0;
        s_diff_new_low_0 = _S336.differential_0;
        new_mid_6 = new_mid_8;
        s_diff_new_mid_0 = s_diff_new_mid_2;
        new_high_2 = _S358;
        s_diff_new_high_0 = _S359;
    }
    var _S360 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( o_3.min_0 * vec3<f32>(new_low_2) + o_3.mid_0 * vec3<f32>(new_mid_6) + o_3.max_0 * vec3<f32>(new_high_2), vec3<f32>(s_diff_new_low_0) * o_3.min_0 + vec3<f32>(s_diff_new_mid_0) * o_3.mid_0 + vec3<f32>(s_diff_new_high_0) * o_3.max_0 );
    return _S360;
}

struct s_paramCtx_s_bwdCallableCtx_per_channel_0
{
     value0_15 : vec3<f32>,
     value1_3 : SigmoidParams_0,
     value2_3 : SigmoidCurve_0,
};

struct s_bwdCallableCtx_per_channel_0
{
     value0_16 : s_paramCtx_s_bwdCallableCtx_per_channel_0,
};

fn s_bwdProp_per_channel_0( _S361 : s_bwdCallableCtx_per_channel_0,  _S362 : ptr<function, SigmoidCurve_0>,  _S363 : vec3<f32>)
{
    var rendering_0 : vec3<f32> = to_rendering_0(_S361.value0_16.value0_15, _S361.value0_16.value1_3);
    var order_1 : HueOrder_0 = hue_order_0(rendering_0);
    var _S364 : f32 = rendering_0.x;
    var _S365 : Tuple_7 = s_apply_loglogistic_0(_S364, _S361.value0_16.value1_3, _S361.value0_16.value2_3);
    var _S366 : s_paramCtx_s_bwdCallableCtx_loglogistic_0;
    _S366.value0_8 = _S364;
    _S366.value1_0 = _S361.value0_16.value1_3;
    _S366.value2_0 = _S361.value0_16.value2_3;
    var _S367 : s_bwdCallableCtx_loglogistic_0 = s_bwdCallableCtx_loglogistic_0( _S366 );
    var _S368 : f32 = rendering_0.y;
    var _S369 : Tuple_7 = s_apply_loglogistic_0(_S368, _S361.value0_16.value1_3, _S361.value0_16.value2_3);
    var _S370 : s_paramCtx_s_bwdCallableCtx_loglogistic_0;
    _S370.value0_8 = _S368;
    _S370.value1_0 = _S361.value0_16.value1_3;
    _S370.value2_0 = _S361.value0_16.value2_3;
    var _S371 : s_bwdCallableCtx_loglogistic_0 = s_bwdCallableCtx_loglogistic_0( _S370 );
    var _S372 : f32 = rendering_0.z;
    var _S373 : Tuple_7 = s_apply_loglogistic_0(_S372, _S361.value0_16.value1_3, _S361.value0_16.value2_3);
    var _S374 : s_paramCtx_s_bwdCallableCtx_loglogistic_0;
    _S374.value0_8 = _S372;
    _S374.value1_0 = _S361.value0_16.value1_3;
    _S374.value2_0 = _S361.value0_16.value2_3;
    var _S375 : s_bwdCallableCtx_loglogistic_0 = s_bwdCallableCtx_loglogistic_0( _S374 );
    var curve_0 : vec3<f32> = vec3<f32>(_S365.value0_7, _S369.value0_7, _S373.value0_7);
    var _S376 : Tuple_8 = s_apply_preserve_hue_and_energy_0(curve_0, order_1, _S361.value0_16.value2_3.hue_preservation_1);
    var _S377 : s_paramCtx_s_bwdCallableCtx_preserve_hue_and_energy_0;
    _S377.value0_13 = curve_0;
    _S377.value1_2 = order_1;
    _S377.value2_2 = _S361.value0_16.value2_3.hue_preservation_1;
    var _S378 : s_bwdCallableCtx_preserve_hue_and_energy_0 = s_bwdCallableCtx_preserve_hue_and_energy_0( _S377 );
    var _S379 : s_bwdCallableCtx_mul_0;
    _S379._S14 = mat3x3<f32>(_S361.value0_16.value1_3.rendering_to_pipe_r0_0.xyz, _S361.value0_16.value1_3.rendering_to_pipe_r1_0.xyz, _S361.value0_16.value1_3.rendering_to_pipe_r2_0.xyz);
    _S379._S15 = _S376.value0_12;
    var _S380 : SigmoidCurve_0 = SigmoidCurve_x24_syn_dzero_0();
    var _S381 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S382 : vec3<f32> = vec3<f32>(0.0f);
    var _S383 : vec3<f32> = _S382;
    s_bwdProp_mul_0(_S379, &(_S381), &(_S383), _S363);
    var _S384 : vec3<f32> = _S382;
    var _S385 : f32 = 0.0f;
    s_bwdProp_preserve_hue_and_energy_0(_S378, &(_S384), &(_S385), _S383);
    var _S386 : SigmoidCurve_0;
    _S386.white_target_1 = 0.0f;
    _S386.black_target_1 = 0.0f;
    _S386.log2_paper_exposure_1 = 0.0f;
    _S386.film_fog_1 = 0.0f;
    _S386.film_power_1 = 0.0f;
    _S386.paper_power_1 = 0.0f;
    _S386.hue_preservation_1 = 0.0f;
    s_bwdProp_loglogistic_0(_S375, &(_S386), _S384[i32(2)]);
    var _S387 : SigmoidCurve_0;
    _S387.white_target_1 = 0.0f;
    _S387.black_target_1 = 0.0f;
    _S387.log2_paper_exposure_1 = 0.0f;
    _S387.film_fog_1 = 0.0f;
    _S387.film_power_1 = 0.0f;
    _S387.paper_power_1 = 0.0f;
    _S387.hue_preservation_1 = 0.0f;
    s_bwdProp_loglogistic_0(_S371, &(_S387), _S384[i32(1)]);
    var _S388 : SigmoidCurve_0;
    _S388.white_target_1 = 0.0f;
    _S388.black_target_1 = 0.0f;
    _S388.log2_paper_exposure_1 = 0.0f;
    _S388.film_fog_1 = 0.0f;
    _S388.film_power_1 = 0.0f;
    _S388.paper_power_1 = 0.0f;
    _S388.hue_preservation_1 = 0.0f;
    s_bwdProp_loglogistic_0(_S367, &(_S388), _S384[i32(0)]);
    var _S389 : SigmoidCurve_0 = SigmoidCurve_x24_syn_dadd_0(SigmoidCurve_x24_syn_dadd_0(SigmoidCurve_x24_syn_dadd_0(_S386, _S387), _S388), _S380);
    var _S390 : SigmoidCurve_0 = _S380;
    _S390.hue_preservation_1 = _S385;
    (*_S362) = SigmoidCurve_x24_syn_dadd_0(_S389, _S390);
    return;
}

struct s_paramCtx_s_bwdCallableCtx_sigmoid_0
{
     value0_17 : vec3<f32>,
     value1_4 : SigmoidParams_0,
     value2_4 : SigmoidCurve_0,
};

struct s_bwdCallableCtx_sigmoid_0
{
     value0_18 : s_paramCtx_s_bwdCallableCtx_sigmoid_0,
};

fn s_bwdProp_sigmoid_0( _S391 : s_bwdCallableCtx_sigmoid_0,  _S392 : ptr<function, SigmoidCurve_0>,  _S393 : vec3<f32>)
{
    var _S394 : vec3<f32> = vec3<f32>(0.0f);
    var _S395 : vec4<f32> = vec4<f32>(0.0f);
    var _S396 : SigmoidParams_0 = SigmoidParams_0( _S395, _S395, _S395, _S395, _S395, _S395, _S395, _S395, _S395, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, u32(0), 0.0f, 0.0f, u32(0), u32(0) );
    var _S397 : SigmoidCurve_0 = SigmoidCurve_0( 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f );
    var _S398 : s_paramCtx_s_bwdCallableCtx_rgb_ratio_0 = s_paramCtx_s_bwdCallableCtx_rgb_ratio_0( _S394, _S396, _S397 );
    var _S399 : s_paramCtx_s_bwdCallableCtx_per_channel_0 = s_paramCtx_s_bwdCallableCtx_per_channel_0( _S394, _S396, _S397 );
    var _S400 : Tuple_6 = s_apply_clamp_0(_S391.value0_18.value0_17, vec3<f32>(-1.00000001504746622e+30f), vec3<f32>(1.00000001504746622e+30f));
    var _S401 : bool = (_S391.value0_18.value1_4.color_processing_0) == u32(1);
    var _S402 : s_bwdCallableCtx_rgb_ratio_0;
    if(_S401)
    {
        var _S403 : s_paramCtx_s_bwdCallableCtx_rgb_ratio_0;
        _S403.value0_10 = _S400.value0_6;
        _S403.value1_1 = _S391.value0_18.value1_4;
        _S403.value2_1 = _S391.value0_18.value2_4;
        _S402.value0_11 = _S403;
    }
    else
    {
        _S402.value0_11 = _S398;
    }
    var _S404 : bool = !_S401;
    var _S405 : s_bwdCallableCtx_per_channel_0;
    if(_S404)
    {
        var _S406 : s_paramCtx_s_bwdCallableCtx_per_channel_0;
        _S406.value0_15 = _S400.value0_6;
        _S406.value1_3 = _S391.value0_18.value1_4;
        _S406.value2_3 = _S391.value0_18.value2_4;
        _S405.value0_16 = _S406;
    }
    else
    {
        _S405.value0_16 = _S399;
    }
    var _S407 : SigmoidCurve_0 = SigmoidCurve_x24_syn_dzero_0();
    var _S408 : vec3<f32>;
    var _S409 : SigmoidCurve_0;
    if(_S404)
    {
        var _S410 : SigmoidCurve_0;
        _S410.white_target_1 = 0.0f;
        _S410.black_target_1 = 0.0f;
        _S410.log2_paper_exposure_1 = 0.0f;
        _S410.film_fog_1 = 0.0f;
        _S410.film_power_1 = 0.0f;
        _S410.paper_power_1 = 0.0f;
        _S410.hue_preservation_1 = 0.0f;
        s_bwdProp_per_channel_0(_S405, &(_S410), _S393);
        var _S411 : SigmoidCurve_0 = SigmoidCurve_x24_syn_dadd_0(_S410, _S407);
        _S408 = _S394;
        _S409 = _S411;
    }
    else
    {
        _S408 = _S393;
        _S409 = _S407;
    }
    if(_S401)
    {
        var _S412 : SigmoidCurve_0;
        _S412.white_target_1 = 0.0f;
        _S412.black_target_1 = 0.0f;
        _S412.log2_paper_exposure_1 = 0.0f;
        _S412.film_fog_1 = 0.0f;
        _S412.film_power_1 = 0.0f;
        _S412.paper_power_1 = 0.0f;
        _S412.hue_preservation_1 = 0.0f;
        s_bwdProp_rgb_ratio_0(_S402, &(_S412), _S408);
        _S409 = SigmoidCurve_x24_syn_dadd_0(_S412, _S409);
    }
    (*_S392) = _S409;
    return;
}

fn sigmoid_curve_0() -> SigmoidCurve_0
{
    var c_0 : SigmoidCurve_0;
    c_0.white_target_1 = params_0.sigmoid_0.white_target_0;
    c_0.black_target_1 = params_0.sigmoid_0.black_target_0;
    c_0.log2_paper_exposure_1 = params_0.sigmoid_0.log2_paper_exposure_0;
    c_0.film_fog_1 = params_0.sigmoid_0.film_fog_0;
    c_0.film_power_1 = params_0.sigmoid_0.film_power_0;
    c_0.paper_power_1 = params_0.sigmoid_0.paper_power_0;
    c_0.hue_preservation_1 = params_0.sigmoid_0.hue_preservation_0;
    return c_0;
}

fn loglogistic_0( _S413 : f32,  _S414 : SigmoidCurve_0) -> f32
{
    var film_2 : f32 = _S414.film_fog_1 + max(_S413, 0.0f);
    var _S415 : bool;
    if((film_2 >= (params_0.sigmoid_0.film_overflow_0)))
    {
        _S415 = true;
    }
    else
    {
        _S415 = (params_0.sigmoid_0.paper_exposure_0) == u32(2);
    }
    if(_S415)
    {
        return _S414.white_target_1;
    }
    if((film_2 <= (params_0.sigmoid_0.film_underflow_0)))
    {
        _S415 = true;
    }
    else
    {
        _S415 = (params_0.sigmoid_0.paper_exposure_0) == u32(1);
    }
    if(_S415)
    {
        return 0.0f;
    }
    var u_2 : f32 = _S414.log2_paper_exposure_1 - _S414.film_power_1 * log2(film_2);
    var softplus_2 : f32;
    if((u_2 > 0.0f))
    {
        softplus_2 = u_2 + log2(1.0f + exp2(- u_2));
    }
    else
    {
        softplus_2 = log2(1.0f + exp2(u_2));
    }
    return _S414.white_target_1 * exp2(- _S414.paper_power_1 * softplus_2);
}

fn rgb_ratio_0( _S416 : vec3<f32>,  _S417 : SigmoidCurve_0) -> vec3<f32>
{
    var positive_1 : vec3<f32> = desaturate_negative_values_0(_S416);
    var luma_1 : f32 = (positive_1.x + positive_1.y + positive_1.z) / 3.0f;
    var _S418 : f32 = loglogistic_0(luma_1, _S417);
    var scaled_1 : vec3<f32>;
    if((luma_1 > 9.99999971718068537e-10f))
    {
        scaled_1 = vec3<f32>((_S418 / luma_1)) * positive_1;
    }
    else
    {
        scaled_1 = vec3<f32>(_S418);
    }
    var _S419 : f32 = scaled_1.x;
    var _S420 : f32 = scaled_1.y;
    var _S421 : f32 = scaled_1.z;
    var _S422 : f32 = min(_S419, min(_S420, _S421));
    var _S423 : f32 = min((_S417.white_target_1 - _S418) / (max(_S419, max(_S420, _S421)) - _S418 + 9.99999997475242708e-07f), (_S417.black_target_1 - _S418) / (_S422 - _S418 - 9.99999997475242708e-07f));
    var chroma_vs_mapping_border_1 : f32 = (_S418 - _S422) / (_S418 + 9.99999997475242708e-07f);
    var hyperbolic_chroma_1 : f32 = 2.0f * chroma_vs_mapping_border_1 / (1.0f - chroma_vs_mapping_border_1 * chroma_vs_mapping_border_1 + 9.99999997475242708e-07f) * (1.0f / (chroma_vs_mapping_border_1 * _S423 + 9.99999997475242708e-07f));
    var _S424 : vec3<f32> = vec3<f32>(_S418);
    return _S424 + vec3<f32>((hyperbolic_chroma_1 / (1.0f + sqrt(hyperbolic_chroma_1 * hyperbolic_chroma_1 + 1.0f)) * _S423)) * (scaled_1 - _S424);
}

fn to_rendering_1( _S425 : vec3<f32>) -> vec3<f32>
{
    return (((desaturate_negative_values_0((((_S425) * (mat3x3<f32>(params_0.sigmoid_0.pipe_to_base_r0_0.xyz, params_0.sigmoid_0.pipe_to_base_r1_0.xyz, params_0.sigmoid_0.pipe_to_base_r2_0.xyz)))))) * (mat3x3<f32>(params_0.sigmoid_0.base_to_rendering_r0_0.xyz, params_0.sigmoid_0.base_to_rendering_r1_0.xyz, params_0.sigmoid_0.base_to_rendering_r2_0.xyz))));
}

fn per_channel_1( _S426 : vec3<f32>,  _S427 : SigmoidCurve_0) -> vec3<f32>
{
    var _S428 : vec3<f32> = to_rendering_1(_S426);
    return (((preserve_hue_and_energy_0(vec3<f32>(loglogistic_0(_S428.x, _S427), loglogistic_0(_S428.y, _S427), loglogistic_0(_S428.z, _S427)), hue_order_0(_S428), _S427.hue_preservation_1)) * (mat3x3<f32>(params_0.sigmoid_0.rendering_to_pipe_r0_0.xyz, params_0.sigmoid_0.rendering_to_pipe_r1_0.xyz, params_0.sigmoid_0.rendering_to_pipe_r2_0.xyz))));
}

fn sigmoid_1( _S429 : vec3<f32>,  _S430 : SigmoidCurve_0) -> vec3<f32>
{
    var rgb_2 : vec3<f32> = clamp(_S429, vec3<f32>(-1.00000001504746622e+30f), vec3<f32>(1.00000001504746622e+30f));
    if((params_0.sigmoid_0.color_processing_0) == u32(1))
    {
        return rgb_ratio_0(rgb_2, _S430);
    }
    return per_channel_1(rgb_2, _S430);
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_0 : vec3<u32>)
{
    var _S431 : u32 = id_0.x;
    var _S432 : bool;
    if((_S431 >= (params_0.pass_0.width_0)))
    {
        _S432 = true;
    }
    else
    {
        _S432 = ((id_0.y) >= (params_0.pass_0.height_0));
    }
    if(_S432)
    {
        return;
    }
    var i_1 : u32 = id_0.y * params_0.pass_0.width_0 + _S431;
    var pixel_0 : vec4<f32> = input_0[i_1];
    var rgb_3 : vec3<f32> = pixel_0.xyz;
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
        var _S433 : vec3<f32> = sigmoid_1(rgb_3, sigmoid_curve_0());
        var _S434 : u32 = k_0 + u32(1);
        rgb_3 = _S433;
        k_0 = _S434;
    }
    output_0[i_1] = vec4<f32>(rgb_3, pixel_0.w);
    return;
}

struct DiffPair_SigmoidCurve_0
{
     primal_0 : SigmoidCurve_0,
     differential_0 : SigmoidCurve_0,
};

fn _S435( _S436 : vec3<f32>,  _S437 : ptr<function, DiffPair_SigmoidCurve_0>,  _S438 : vec3<f32>)
{
    var _S439 : s_paramCtx_s_bwdCallableCtx_sigmoid_0;
    _S439.value0_17 = _S436;
    _S439.value1_4.pipe_to_base_r0_0 = params_0.sigmoid_0.pipe_to_base_r0_0;
    _S439.value1_4.pipe_to_base_r1_0 = params_0.sigmoid_0.pipe_to_base_r1_0;
    _S439.value1_4.pipe_to_base_r2_0 = params_0.sigmoid_0.pipe_to_base_r2_0;
    _S439.value1_4.base_to_rendering_r0_0 = params_0.sigmoid_0.base_to_rendering_r0_0;
    _S439.value1_4.base_to_rendering_r1_0 = params_0.sigmoid_0.base_to_rendering_r1_0;
    _S439.value1_4.base_to_rendering_r2_0 = params_0.sigmoid_0.base_to_rendering_r2_0;
    _S439.value1_4.rendering_to_pipe_r0_0 = params_0.sigmoid_0.rendering_to_pipe_r0_0;
    _S439.value1_4.rendering_to_pipe_r1_0 = params_0.sigmoid_0.rendering_to_pipe_r1_0;
    _S439.value1_4.rendering_to_pipe_r2_0 = params_0.sigmoid_0.rendering_to_pipe_r2_0;
    _S439.value1_4.white_target_0 = params_0.sigmoid_0.white_target_0;
    _S439.value1_4.black_target_0 = params_0.sigmoid_0.black_target_0;
    _S439.value1_4.log2_paper_exposure_0 = params_0.sigmoid_0.log2_paper_exposure_0;
    _S439.value1_4.film_fog_0 = params_0.sigmoid_0.film_fog_0;
    _S439.value1_4.film_power_0 = params_0.sigmoid_0.film_power_0;
    _S439.value1_4.paper_power_0 = params_0.sigmoid_0.paper_power_0;
    _S439.value1_4.hue_preservation_0 = params_0.sigmoid_0.hue_preservation_0;
    _S439.value1_4.color_processing_0 = params_0.sigmoid_0.color_processing_0;
    _S439.value1_4.film_overflow_0 = params_0.sigmoid_0.film_overflow_0;
    _S439.value1_4.film_underflow_0 = params_0.sigmoid_0.film_underflow_0;
    _S439.value1_4.paper_exposure_0 = params_0.sigmoid_0.paper_exposure_0;
    _S439.value1_4._pad0_0 = params_0.sigmoid_0._pad0_0;
    _S439.value2_4 = (*_S437).primal_0;
    var _S440 : s_bwdCallableCtx_sigmoid_0 = s_bwdCallableCtx_sigmoid_0( _S439 );
    var _S441 : SigmoidCurve_0 = (*_S437).differential_0;
    s_bwdProp_sigmoid_0(_S440, &(_S441), _S438);
    (*_S437).differential_0 = _S441;
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn sigmoid_backward(@builtin(global_invocation_id) id_1 : vec3<u32>)
{
    var _S442 : u32 = id_1.x;
    var _S443 : bool;
    if((_S442 >= (params_0.pass_0.width_0)))
    {
        _S443 = true;
    }
    else
    {
        _S443 = ((id_1.y) >= (params_0.pass_0.height_0));
    }
    if(_S443)
    {
        return;
    }
    var i_2 : u32 = id_1.y * params_0.pass_0.width_0 + _S442;
    var _S444 : SigmoidCurve_0 = SigmoidCurve_x24_syn_dzero_0();
    var curve_1 : DiffPair_SigmoidCurve_0;
    curve_1.primal_0 = sigmoid_curve_0();
    curve_1.differential_0 = _S444;
    _S435(input_0[i_2].xyz, &(curve_1), derivative_in_0[i_2].xyz);
    var last_0 : f32;
    if((params_0.sigmoid_0.color_processing_0) == u32(1))
    {
        last_0 = curve_1.differential_0.black_target_1;
    }
    else
    {
        last_0 = curve_1.differential_0.hue_preservation_1;
    }
    derivative_out_0[i_2] = vec4<f32>(curve_1.differential_0.film_power_1, curve_1.differential_0.paper_power_1, curve_1.differential_0.white_target_1, last_0);
    return;
}

fn s_fwd_loglogistic_0( _S445 : f32,  _S446 : DiffPair_SigmoidCurve_0) -> DiffPair_float_0
{
    var _S447 : DiffPair_float_0 = DiffPair_float_0( _S445, 0.0f );
    var _S448 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S449 : DiffPair_float_0 = _d_max_1(_S447, _S448);
    var film_3 : f32 = _S446.primal_0.film_fog_1 + _S449.primal_0;
    var s_diff_film_0 : f32 = _S446.differential_0.film_fog_1 + _S449.differential_0;
    var _S450 : bool;
    if((film_3 >= (params_0.sigmoid_0.film_overflow_0)))
    {
        _S450 = true;
    }
    else
    {
        _S450 = (params_0.sigmoid_0.paper_exposure_0) == u32(2);
    }
    if(_S450)
    {
        var _S451 : DiffPair_float_0 = DiffPair_float_0( _S446.primal_0.white_target_1, _S446.differential_0.white_target_1 );
        return _S451;
    }
    if((film_3 <= (params_0.sigmoid_0.film_underflow_0)))
    {
        _S450 = true;
    }
    else
    {
        _S450 = (params_0.sigmoid_0.paper_exposure_0) == u32(1);
    }
    if(_S450)
    {
        return _S448;
    }
    var _S452 : DiffPair_float_0 = DiffPair_float_0( film_3, s_diff_film_0 );
    var _S453 : DiffPair_float_0 = _d_log2_1(_S452);
    var u_3 : f32 = _S446.primal_0.log2_paper_exposure_1 - _S446.primal_0.film_power_1 * _S453.primal_0;
    var s_diff_u_0 : f32 = _S446.differential_0.log2_paper_exposure_1 - (_S446.differential_0.film_power_1 * _S453.primal_0 + _S453.differential_0 * _S446.primal_0.film_power_1);
    var softplus_3 : f32;
    var s_diff_softplus_0 : f32;
    if((u_3 > 0.0f))
    {
        var _S454 : DiffPair_float_0 = DiffPair_float_0( - u_3, - s_diff_u_0 );
        var _S455 : DiffPair_float_0 = _d_exp2_1(_S454);
        var _S456 : DiffPair_float_0 = DiffPair_float_0( 1.0f + _S455.primal_0, _S455.differential_0 );
        var _S457 : DiffPair_float_0 = _d_log2_1(_S456);
        var _S458 : f32 = s_diff_u_0 + _S457.differential_0;
        softplus_3 = u_3 + _S457.primal_0;
        s_diff_softplus_0 = _S458;
    }
    else
    {
        var _S459 : DiffPair_float_0 = DiffPair_float_0( u_3, s_diff_u_0 );
        var _S460 : DiffPair_float_0 = _d_exp2_1(_S459);
        var _S461 : DiffPair_float_0 = DiffPair_float_0( 1.0f + _S460.primal_0, _S460.differential_0 );
        var _S462 : DiffPair_float_0 = _d_log2_1(_S461);
        softplus_3 = _S462.primal_0;
        s_diff_softplus_0 = _S462.differential_0;
    }
    var _S463 : f32 = - _S446.primal_0.paper_power_1;
    var _S464 : DiffPair_float_0 = DiffPair_float_0( _S463 * softplus_3, - _S446.differential_0.paper_power_1 * softplus_3 + s_diff_softplus_0 * _S463 );
    var _S465 : DiffPair_float_0 = _d_exp2_1(_S464);
    var _S466 : DiffPair_float_0 = DiffPair_float_0( _S446.primal_0.white_target_1 * _S465.primal_0, _S446.differential_0.white_target_1 * _S465.primal_0 + _S465.differential_0 * _S446.primal_0.white_target_1 );
    return _S466;
}

fn s_fwd_rgb_ratio_0( _S467 : vec3<f32>,  _S468 : DiffPair_SigmoidCurve_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var positive_2 : vec3<f32> = desaturate_negative_values_0(_S467);
    var luma_2 : f32 = (positive_2.x + positive_2.y + positive_2.z) / 3.0f;
    var _S469 : DiffPair_SigmoidCurve_0 = DiffPair_SigmoidCurve_0( _S468.primal_0, _S468.differential_0 );
    var _S470 : DiffPair_float_0 = s_fwd_loglogistic_0(luma_2, _S469);
    var scaled_2 : vec3<f32>;
    var s_diff_scaled_0 : vec3<f32>;
    if((luma_2 > 9.99999971718068537e-10f))
    {
        var _S471 : vec3<f32> = vec3<f32>((_S470.differential_0 / luma_2)) * positive_2;
        scaled_2 = vec3<f32>((_S470.primal_0 / luma_2)) * positive_2;
        s_diff_scaled_0 = _S471;
    }
    else
    {
        var _S472 : vec3<f32> = vec3<f32>(_S470.differential_0);
        scaled_2 = vec3<f32>(_S470.primal_0);
        s_diff_scaled_0 = _S472;
    }
    var _S473 : DiffPair_float_0 = DiffPair_float_0( scaled_2.y, s_diff_scaled_0.y );
    var _S474 : DiffPair_float_0 = DiffPair_float_0( scaled_2.z, s_diff_scaled_0.z );
    var _S475 : DiffPair_float_0 = _d_min_1(_S473, _S474);
    var _S476 : DiffPair_float_0 = DiffPair_float_0( scaled_2.x, s_diff_scaled_0.x );
    var _S477 : DiffPair_float_0 = DiffPair_float_0( _S475.primal_0, _S475.differential_0 );
    var _S478 : DiffPair_float_0 = _d_min_1(_S476, _S477);
    var _S479 : DiffPair_float_0 = _d_max_1(_S473, _S474);
    var _S480 : DiffPair_float_0 = DiffPair_float_0( _S479.primal_0, _S479.differential_0 );
    var _S481 : DiffPair_float_0 = _d_max_1(_S476, _S480);
    var _S482 : f32 = _S468.primal_0.white_target_1 - _S470.primal_0;
    var _S483 : f32 = _S481.primal_0 - _S470.primal_0 + 9.99999997475242708e-07f;
    var _S484 : f32 = _S468.primal_0.black_target_1 - _S470.primal_0;
    var _S485 : f32 = _S478.primal_0 - _S470.primal_0 - 9.99999997475242708e-07f;
    var _S486 : DiffPair_float_0 = DiffPair_float_0( _S482 / _S483, ((_S468.differential_0.white_target_1 - _S470.differential_0) * _S483 - _S482 * (_S481.differential_0 - _S470.differential_0)) / (_S483 * _S483) );
    var _S487 : DiffPair_float_0 = DiffPair_float_0( _S484 / _S485, ((_S468.differential_0.black_target_1 - _S470.differential_0) * _S485 - _S484 * (_S478.differential_0 - _S470.differential_0)) / (_S485 * _S485) );
    var _S488 : DiffPair_float_0 = _d_min_1(_S486, _S487);
    var _S489 : f32 = _S470.primal_0 - _S478.primal_0;
    var _S490 : f32 = _S470.primal_0 + 9.99999997475242708e-07f;
    var chroma_vs_mapping_border_2 : f32 = _S489 / _S490;
    var s_diff_chroma_vs_mapping_border_0 : f32 = ((_S470.differential_0 - _S478.differential_0) * _S490 - _S489 * _S470.differential_0) / (_S490 * _S490);
    var _S491 : f32 = chroma_vs_mapping_border_2 * _S488.primal_0 + 9.99999997475242708e-07f;
    var pixel_chroma_adjustment_1 : f32 = 1.0f / _S491;
    var _S492 : f32 = 2.0f * chroma_vs_mapping_border_2;
    var _S493 : f32 = s_diff_chroma_vs_mapping_border_0 * chroma_vs_mapping_border_2;
    var _S494 : f32 = 1.0f - chroma_vs_mapping_border_2 * chroma_vs_mapping_border_2 + 9.99999997475242708e-07f;
    var _S495 : f32 = _S492 / _S494;
    var hyperbolic_chroma_2 : f32 = _S495 * pixel_chroma_adjustment_1;
    var s_diff_hyperbolic_chroma_0 : f32 = (s_diff_chroma_vs_mapping_border_0 * 2.0f * _S494 - _S492 * (0.0f - (_S493 + _S493))) / (_S494 * _S494) * pixel_chroma_adjustment_1 + (0.0f - (s_diff_chroma_vs_mapping_border_0 * _S488.primal_0 + _S488.differential_0 * chroma_vs_mapping_border_2)) / (_S491 * _S491) * _S495;
    var _S496 : f32 = s_diff_hyperbolic_chroma_0 * hyperbolic_chroma_2;
    var _S497 : DiffPair_float_0 = DiffPair_float_0( hyperbolic_chroma_2 * hyperbolic_chroma_2 + 1.0f, _S496 + _S496 );
    var _S498 : DiffPair_float_0 = _d_sqrt_1(_S497);
    var _S499 : f32 = 1.0f + _S498.primal_0;
    var _S500 : f32 = hyperbolic_chroma_2 / _S499;
    var _S501 : vec3<f32> = vec3<f32>(_S470.primal_0);
    var _S502 : vec3<f32> = scaled_2 - _S501;
    var _S503 : vec3<f32> = vec3<f32>(_S470.differential_0);
    var _S504 : vec3<f32> = vec3<f32>((_S500 * _S488.primal_0));
    var _S505 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S501 + _S504 * _S502, _S503 + (vec3<f32>(((s_diff_hyperbolic_chroma_0 * _S499 - hyperbolic_chroma_2 * _S498.differential_0) / (_S499 * _S499) * _S488.primal_0 + _S488.differential_0 * _S500)) * _S502 + (s_diff_scaled_0 - _S503) * _S504) );
    return _S505;
}

fn s_fwd_per_channel_0( _S506 : vec3<f32>,  _S507 : DiffPair_SigmoidCurve_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S508 : vec3<f32> = to_rendering_1(_S506);
    var _S509 : DiffPair_SigmoidCurve_0 = DiffPair_SigmoidCurve_0( _S507.primal_0, _S507.differential_0 );
    var _S510 : DiffPair_float_0 = s_fwd_loglogistic_0(_S508.x, _S509);
    var _S511 : DiffPair_float_0 = s_fwd_loglogistic_0(_S508.y, _S509);
    var _S512 : DiffPair_float_0 = s_fwd_loglogistic_0(_S508.z, _S509);
    var _S513 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S510.primal_0, _S511.primal_0, _S512.primal_0), vec3<f32>(_S510.differential_0, _S511.differential_0, _S512.differential_0) );
    var _S514 : DiffPair_float_0 = DiffPair_float_0( _S507.primal_0.hue_preservation_1, _S507.differential_0.hue_preservation_1 );
    var _S515 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_preserve_hue_and_energy_0(_S513, hue_order_0(_S508), _S514);
    var rendering_to_pipe_0 : mat3x3<f32> = mat3x3<f32>(params_0.sigmoid_0.rendering_to_pipe_r0_0.xyz, params_0.sigmoid_0.rendering_to_pipe_r1_0.xyz, params_0.sigmoid_0.rendering_to_pipe_r2_0.xyz);
    var _S516 : vec3<f32> = vec3<f32>(0.0f);
    var _S517 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( (((_S515.primal_0) * (rendering_to_pipe_0))), (((_S515.primal_0) * (mat3x3<f32>(_S516, _S516, _S516)))) + (((_S515.differential_0) * (rendering_to_pipe_0))) );
    return _S517;
}

fn s_fwd_sigmoid_0( _S518 : vec3<f32>,  _S519 : DiffPair_SigmoidCurve_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S520 : vec3<f32> = vec3<f32>(0.0f);
    var _S521 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S518, _S520 );
    var _S522 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(-1.00000001504746622e+30f), _S520 );
    var _S523 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(1.00000001504746622e+30f), _S520 );
    var _S524 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_clamp_vector_0(_S521, _S522, _S523);
    if((params_0.sigmoid_0.color_processing_0) == u32(1))
    {
        var _S525 : DiffPair_SigmoidCurve_0 = DiffPair_SigmoidCurve_0( _S519.primal_0, _S519.differential_0 );
        var _S526 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_rgb_ratio_0(_S524.primal_0, _S525);
        var _S527 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S526.primal_0, _S526.differential_0 );
        return _S527;
    }
    var _S528 : DiffPair_SigmoidCurve_0 = DiffPair_SigmoidCurve_0( _S519.primal_0, _S519.differential_0 );
    var _S529 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_per_channel_0(_S524.primal_0, _S528);
    var _S530 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S529.primal_0, _S529.differential_0 );
    return _S530;
}

@compute
@workgroup_size(16, 16, 1)
fn sigmoid_forward_diff(@builtin(global_invocation_id) id_2 : vec3<u32>)
{
    var _S531 : u32 = id_2.x;
    var _S532 : bool;
    if((_S531 >= (params_0.pass_0.width_0)))
    {
        _S532 = true;
    }
    else
    {
        _S532 = ((id_2.y) >= (params_0.pass_0.height_0));
    }
    if(_S532)
    {
        return;
    }
    var i_3 : u32 = id_2.y * params_0.pass_0.width_0 + _S531;
    var v_1 : vec4<f32> = derivative_in_0[i_3];
    var ratio_0 : bool = (params_0.sigmoid_0.color_processing_0) == u32(1);
    var direction_0 : SigmoidCurve_0;
    direction_0.white_target_1 = v_1.z;
    var _S533 : f32;
    if(ratio_0)
    {
        _S533 = v_1.w;
    }
    else
    {
        _S533 = 0.0f;
    }
    direction_0.black_target_1 = _S533;
    direction_0.log2_paper_exposure_1 = 0.0f;
    direction_0.film_fog_1 = 0.0f;
    direction_0.film_power_1 = v_1.x;
    direction_0.paper_power_1 = v_1.y;
    if(ratio_0)
    {
        _S533 = 0.0f;
    }
    else
    {
        _S533 = v_1.w;
    }
    direction_0.hue_preservation_1 = _S533;
    var _S534 : DiffPair_SigmoidCurve_0 = DiffPair_SigmoidCurve_0( sigmoid_curve_0(), direction_0 );
    derivative_out_0[i_3] = vec4<f32>(s_fwd_sigmoid_0(input_0[i_3].xyz, _S534).differential_0, 0.0f);
    return;
}

