// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2020-2026  darktable developers (ported from src/iop/colorbalancergb.c)
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

// Generated from slang/colorbalancergb.slang by tools/build-kernels.sh. Do not edit.

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

struct ColorBalanceParams_std140_0
{
    @align(16) input_r0_0 : vec4<f32>,
    @align(16) input_r1_0 : vec4<f32>,
    @align(16) input_r2_0 : vec4<f32>,
    @align(16) output_r0_0 : vec4<f32>,
    @align(16) output_r1_0 : vec4<f32>,
    @align(16) output_r2_0 : vec4<f32>,
    @align(16) global_0 : vec4<f32>,
    @align(16) shadows_0 : vec4<f32>,
    @align(16) highlights_0 : vec4<f32>,
    @align(16) midtones_0 : vec4<f32>,
    @align(16) chroma_0 : vec4<f32>,
    @align(16) saturation_0 : vec4<f32>,
    @align(16) brilliance_0 : vec4<f32>,
    @align(16) chroma_global_0 : f32,
    @align(4) vibrance_0 : f32,
    @align(8) contrast_0 : f32,
    @align(4) saturation_global_0 : f32,
    @align(16) brilliance_global_0 : f32,
    @align(4) hue_cos_0 : f32,
    @align(8) hue_sin_0 : f32,
    @align(4) shadows_weight_0 : f32,
    @align(16) highlights_weight_0 : f32,
    @align(4) midtones_weight_0 : f32,
    @align(8) mask_grey_fulcrum_0 : f32,
    @align(4) white_fulcrum_0 : f32,
    @align(16) grey_fulcrum_0 : f32,
    @align(4) midtones_y_0 : f32,
    @align(8) l_white_0 : f32,
    @align(4) saturation_formula_0 : u32,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) color_balance_0 : ColorBalanceParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(0) @group(0) var<storage, read> input_0 : array<vec4<f32>>;

@binding(3) @group(0) var<storage, read> gamut_lut_0 : array<f32>;

@binding(1) @group(0) var<storage, read_write> output_0 : array<vec4<f32>>;

@binding(8) @group(0) var<storage, read> derivative_in_0 : array<vec4<f32>>;

@binding(9) @group(0) var<storage, read_write> derivative_out_0 : array<vec4<f32>>;

struct Tuple_0
{
     value0_0 : vec2<f32>,
};

fn s_apply_abs_0( x_0 : vec2<f32>) -> Tuple_0
{
    var _S1 : Tuple_0 = Tuple_0( abs(x_0) );
    return _S1;
}

struct s_bwdCallableCtx_abs_0
{
     _S2 : vec2<f32>,
};

struct DiffPair_vectorx3Cfloatx2C2x3E_0
{
     primal_0 : vec2<f32>,
     differential_0 : vec2<f32>,
};

fn _d_abs_vector_0( dpx_0 : ptr<function, DiffPair_vectorx3Cfloatx2C2x3E_0>,  dOut_0 : vec2<f32>)
{
    var _S3 : vec2<f32> = vec2<f32>(0.0f);
    var _S4 : vec2<f32> = select(select(vec2<f32>(-1.0f), _S3, ((*dpx_0).primal_0) == _S3), vec2<f32>(1.0f), (((*dpx_0).primal_0) > _S3)) * dOut_0;
    (*dpx_0).primal_0 = (*dpx_0).primal_0;
    (*dpx_0).differential_0 = _S4;
    return;
}

fn s_bwdProp_abs_0( ctx_0 : s_bwdCallableCtx_abs_0,  d_x_0 : ptr<function, vec2<f32>>,  d_out_0 : vec2<f32>)
{
    var _S5 : DiffPair_vectorx3Cfloatx2C2x3E_0;
    _S5.primal_0 = ctx_0._S2;
    _d_abs_vector_0(&(_S5), d_out_0);
    (*d_x_0) = _S5.differential_0;
    return;
}

struct Tuple_1
{
     value0_1 : f32,
};

fn s_apply_dot_0( x_1 : vec2<f32>,  y_0 : vec2<f32>) -> Tuple_1
{
    var _S6 : Tuple_1 = Tuple_1( dot(x_1, y_0) );
    return _S6;
}

struct s_bwdCallableCtx_dot_0
{
     _S7 : vec2<f32>,
     _S8 : vec2<f32>,
};

fn _d_dot_0( dpx_1 : ptr<function, DiffPair_vectorx3Cfloatx2C2x3E_0>,  dpy_0 : ptr<function, DiffPair_vectorx3Cfloatx2C2x3E_0>,  dOut_1 : f32)
{
    var x_d_result_0 : vec2<f32>;
    x_d_result_0[i32(0)] = (*dpy_0).primal_0[i32(0)] * dOut_1;
    var y_d_result_0 : vec2<f32>;
    y_d_result_0[i32(0)] = (*dpx_1).primal_0[i32(0)] * dOut_1;
    x_d_result_0[i32(1)] = (*dpy_0).primal_0[i32(1)] * dOut_1;
    y_d_result_0[i32(1)] = (*dpx_1).primal_0[i32(1)] * dOut_1;
    (*dpx_1).primal_0 = (*dpx_1).primal_0;
    (*dpx_1).differential_0 = x_d_result_0;
    (*dpy_0).primal_0 = (*dpy_0).primal_0;
    (*dpy_0).differential_0 = y_d_result_0;
    return;
}

fn s_bwdProp_dot_0( ctx_1 : s_bwdCallableCtx_dot_0,  d_x_1 : ptr<function, vec2<f32>>,  d_y_0 : ptr<function, vec2<f32>>,  d_out_1 : f32)
{
    var _S9 : DiffPair_vectorx3Cfloatx2C2x3E_0;
    _S9.primal_0 = ctx_1._S7;
    var _S10 : DiffPair_vectorx3Cfloatx2C2x3E_0;
    _S10.primal_0 = ctx_1._S8;
    _d_dot_0(&(_S9), &(_S10), d_out_1);
    (*d_x_1) = _S9.differential_0;
    (*d_y_0) = _S10.differential_0;
    return;
}

struct Tuple_2
{
     value0_2 : f32,
};

fn s_apply_clamp_0( x_2 : f32,  minBound_0 : f32,  maxBound_0 : f32) -> Tuple_2
{
    var _S11 : Tuple_2 = Tuple_2( clamp(x_2, minBound_0, maxBound_0) );
    return _S11;
}

struct s_bwdCallableCtx_clamp_0
{
     _S12 : f32,
     _S13 : f32,
     _S14 : f32,
};

struct DiffPair_float_0
{
     primal_0 : f32,
     differential_0 : f32,
};

fn _d_clamp_0( dpx_2 : ptr<function, DiffPair_float_0>,  dpMin_0 : ptr<function, DiffPair_float_0>,  dpMax_0 : ptr<function, DiffPair_float_0>,  dOut_2 : f32)
{
    var _S15 : DiffPair_float_0 = (*dpx_2);
    var _S16 : bool;
    if((((*dpx_2).primal_0) >= ((*dpMin_0).primal_0)))
    {
        _S16 = (((*dpx_2).primal_0) <= ((*dpMax_0).primal_0));
    }
    else
    {
        _S16 = false;
    }
    var _S17 : f32;
    if(_S16)
    {
        _S17 = dOut_2;
    }
    else
    {
        _S17 = 0.0f;
    }
    (*dpx_2).primal_0 = _S15.primal_0;
    (*dpx_2).differential_0 = _S17;
    var _S18 : DiffPair_float_0 = (*dpMin_0);
    if(((_S15.primal_0) < ((*dpMin_0).primal_0)))
    {
        _S17 = dOut_2;
    }
    else
    {
        _S17 = 0.0f;
    }
    (*dpMin_0).primal_0 = _S18.primal_0;
    (*dpMin_0).differential_0 = _S17;
    var _S19 : DiffPair_float_0 = (*dpMax_0);
    if((((*dpx_2).primal_0) > ((*dpMax_0).primal_0)))
    {
        _S17 = dOut_2;
    }
    else
    {
        _S17 = 0.0f;
    }
    (*dpMax_0).primal_0 = _S19.primal_0;
    (*dpMax_0).differential_0 = _S17;
    return;
}

fn s_bwdProp_clamp_0( ctx_2 : s_bwdCallableCtx_clamp_0,  d_x_2 : ptr<function, f32>,  d_minBound_0 : ptr<function, f32>,  d_maxBound_0 : ptr<function, f32>,  d_out_2 : f32)
{
    var _S20 : DiffPair_float_0;
    _S20.primal_0 = ctx_2._S12;
    var _S21 : DiffPair_float_0;
    _S21.primal_0 = ctx_2._S13;
    var _S22 : DiffPair_float_0;
    _S22.primal_0 = ctx_2._S14;
    _d_clamp_0(&(_S20), &(_S21), &(_S22), d_out_2);
    (*d_x_2) = _S20.differential_0;
    (*d_minBound_0) = _S21.differential_0;
    (*d_maxBound_0) = _S22.differential_0;
    return;
}

struct Tuple_3
{
     value0_3 : f32,
};

fn s_apply_sqrt_0( x_3 : f32) -> Tuple_3
{
    var _S23 : Tuple_3 = Tuple_3( sqrt(x_3) );
    return _S23;
}

struct s_bwdCallableCtx_sqrt_0
{
     _S24 : f32,
};

fn _d_sqrt_0( dpx_3 : ptr<function, DiffPair_float_0>,  dOut_3 : f32)
{
    var _S25 : f32 = 0.5f / sqrt(max(1.00000001168609742e-07f, (*dpx_3).primal_0)) * dOut_3;
    (*dpx_3).primal_0 = (*dpx_3).primal_0;
    (*dpx_3).differential_0 = _S25;
    return;
}

fn s_bwdProp_sqrt_0( ctx_3 : s_bwdCallableCtx_sqrt_0,  d_x_3 : ptr<function, f32>,  d_out_3 : f32)
{
    var _S26 : DiffPair_float_0;
    _S26.primal_0 = ctx_3._S24;
    _d_sqrt_0(&(_S26), d_out_3);
    (*d_x_3) = _S26.differential_0;
    return;
}

struct Tuple_4
{
     value0_4 : vec2<f32>,
};

fn s_apply_max_0( x_4 : vec2<f32>,  y_1 : vec2<f32>) -> Tuple_4
{
    var _S27 : Tuple_4 = Tuple_4( max(x_4, y_1) );
    return _S27;
}

struct s_bwdCallableCtx_max_0
{
     _S28 : vec2<f32>,
     _S29 : vec2<f32>,
};

fn _d_max_0( dpx_4 : ptr<function, DiffPair_float_0>,  dpy_1 : ptr<function, DiffPair_float_0>,  dOut_4 : f32)
{
    var _S30 : DiffPair_float_0 = (*dpx_4);
    var _S31 : f32;
    if((((*dpx_4).primal_0) > ((*dpy_1).primal_0)))
    {
        _S31 = dOut_4;
    }
    else
    {
        if((((*dpx_4).primal_0) < ((*dpy_1).primal_0)))
        {
            _S31 = 0.0f;
        }
        else
        {
            _S31 = 0.5f * dOut_4;
        }
    }
    (*dpx_4).primal_0 = _S30.primal_0;
    (*dpx_4).differential_0 = _S31;
    var _S32 : DiffPair_float_0 = (*dpy_1);
    if((((*dpy_1).primal_0) > (_S30.primal_0)))
    {
        _S31 = dOut_4;
    }
    else
    {
        if((((*dpy_1).primal_0) < ((*dpx_4).primal_0)))
        {
            _S31 = 0.0f;
        }
        else
        {
            _S31 = 0.5f * dOut_4;
        }
    }
    (*dpy_1).primal_0 = _S32.primal_0;
    (*dpy_1).differential_0 = _S31;
    return;
}

fn _d_max_vector_0( dpx_5 : ptr<function, DiffPair_vectorx3Cfloatx2C2x3E_0>,  dpy_2 : ptr<function, DiffPair_vectorx3Cfloatx2C2x3E_0>,  dOut_5 : vec2<f32>)
{
    var left_dp_0 : DiffPair_float_0;
    left_dp_0.primal_0 = (*dpx_5).primal_0[i32(0)];
    left_dp_0.differential_0 = 0.0f;
    var right_dp_0 : DiffPair_float_0;
    right_dp_0.primal_0 = (*dpy_2).primal_0[i32(0)];
    right_dp_0.differential_0 = 0.0f;
    _d_max_0(&(left_dp_0), &(right_dp_0), dOut_5[i32(0)]);
    var left_d_result_0 : vec2<f32>;
    left_d_result_0[i32(0)] = left_dp_0.differential_0;
    var right_d_result_0 : vec2<f32>;
    right_d_result_0[i32(0)] = right_dp_0.differential_0;
    var left_dp_1 : DiffPair_float_0;
    left_dp_1.primal_0 = (*dpx_5).primal_0[i32(1)];
    left_dp_1.differential_0 = 0.0f;
    var right_dp_1 : DiffPair_float_0;
    right_dp_1.primal_0 = (*dpy_2).primal_0[i32(1)];
    right_dp_1.differential_0 = 0.0f;
    _d_max_0(&(left_dp_1), &(right_dp_1), dOut_5[i32(1)]);
    left_d_result_0[i32(1)] = left_dp_1.differential_0;
    right_d_result_0[i32(1)] = right_dp_1.differential_0;
    (*dpx_5).primal_0 = (*dpx_5).primal_0;
    (*dpx_5).differential_0 = left_d_result_0;
    (*dpy_2).primal_0 = (*dpy_2).primal_0;
    (*dpy_2).differential_0 = right_d_result_0;
    return;
}

fn s_bwdProp_max_0( ctx_4 : s_bwdCallableCtx_max_0,  d_x_4 : ptr<function, vec2<f32>>,  d_y_1 : ptr<function, vec2<f32>>,  d_out_4 : vec2<f32>)
{
    var _S33 : DiffPair_vectorx3Cfloatx2C2x3E_0;
    _S33.primal_0 = ctx_4._S28;
    var _S34 : DiffPair_vectorx3Cfloatx2C2x3E_0;
    _S34.primal_0 = ctx_4._S29;
    _d_max_vector_0(&(_S33), &(_S34), d_out_4);
    (*d_x_4) = _S33.differential_0;
    (*d_y_1) = _S34.differential_0;
    return;
}

struct Tuple_5
{
     value0_5 : f32,
};

fn s_apply_cos_0( x_5 : f32) -> Tuple_5
{
    var _S35 : Tuple_5 = Tuple_5( cos(x_5) );
    return _S35;
}

struct s_bwdCallableCtx_cos_0
{
     _S36 : f32,
};

fn _d_cos_0( dpx_6 : ptr<function, DiffPair_float_0>,  dOut_6 : f32)
{
    var _S37 : f32 = - sin((*dpx_6).primal_0) * dOut_6;
    (*dpx_6).primal_0 = (*dpx_6).primal_0;
    (*dpx_6).differential_0 = _S37;
    return;
}

fn s_bwdProp_cos_0( ctx_5 : s_bwdCallableCtx_cos_0,  d_x_5 : ptr<function, f32>,  d_out_5 : f32)
{
    var _S38 : DiffPair_float_0;
    _S38.primal_0 = ctx_5._S36;
    _d_cos_0(&(_S38), d_out_5);
    (*d_x_5) = _S38.differential_0;
    return;
}

struct Tuple_6
{
     value0_6 : f32,
};

fn s_apply_sin_0( x_6 : f32) -> Tuple_6
{
    var _S39 : Tuple_6 = Tuple_6( sin(x_6) );
    return _S39;
}

struct s_bwdCallableCtx_sin_0
{
     _S40 : f32,
};

fn _d_sin_0( dpx_7 : ptr<function, DiffPair_float_0>,  dOut_7 : f32)
{
    var _S41 : f32 = cos((*dpx_7).primal_0) * dOut_7;
    (*dpx_7).primal_0 = (*dpx_7).primal_0;
    (*dpx_7).differential_0 = _S41;
    return;
}

fn s_bwdProp_sin_0( ctx_6 : s_bwdCallableCtx_sin_0,  d_x_6 : ptr<function, f32>,  d_out_6 : f32)
{
    var _S42 : DiffPair_float_0;
    _S42.primal_0 = ctx_6._S40;
    _d_sin_0(&(_S42), d_out_6);
    (*d_x_6) = _S42.differential_0;
    return;
}

struct Tuple_7
{
     value0_7 : f32,
};

fn s_apply_atan2_0( y_2 : f32,  x_7 : f32) -> Tuple_7
{
    var _S43 : Tuple_7 = Tuple_7( atan2(y_2, x_7) );
    return _S43;
}

struct s_bwdCallableCtx_atan2_0
{
     _S44 : f32,
     _S45 : f32,
};

fn _d_atan2_0( dpy_3 : ptr<function, DiffPair_float_0>,  dpx_8 : ptr<function, DiffPair_float_0>,  dOut_8 : f32)
{
    var _S46 : DiffPair_float_0 = (*dpx_8);
    var _S47 : f32 = - (*dpy_3).primal_0 / ((*dpx_8).primal_0 * (*dpx_8).primal_0 + (*dpy_3).primal_0 * (*dpy_3).primal_0) * dOut_8;
    (*dpx_8).primal_0 = (*dpx_8).primal_0;
    (*dpx_8).differential_0 = _S47;
    var _S48 : f32 = _S46.primal_0 / (_S46.primal_0 * _S46.primal_0 + (*dpy_3).primal_0 * (*dpy_3).primal_0) * dOut_8;
    (*dpy_3).primal_0 = (*dpy_3).primal_0;
    (*dpy_3).differential_0 = _S48;
    return;
}

fn s_bwdProp_atan2_0( ctx_7 : s_bwdCallableCtx_atan2_0,  d_y_2 : ptr<function, f32>,  d_x_7 : ptr<function, f32>,  d_out_7 : f32)
{
    var _S49 : DiffPair_float_0;
    _S49.primal_0 = ctx_7._S44;
    var _S50 : DiffPair_float_0;
    _S50.primal_0 = ctx_7._S45;
    _d_atan2_0(&(_S49), &(_S50), d_out_7);
    (*d_y_2) = _S49.differential_0;
    (*d_x_7) = _S50.differential_0;
    return;
}

struct Tuple_8
{
     value0_8 : f32,
};

fn s_apply_exp2_0( x_8 : f32) -> Tuple_8
{
    var _S51 : Tuple_8 = Tuple_8( exp2(x_8) );
    return _S51;
}

struct s_bwdCallableCtx_exp2_0
{
     _S52 : f32,
};

fn _d_exp2_0( dpx_9 : ptr<function, DiffPair_float_0>,  dOut_9 : f32)
{
    var _S53 : f32 = exp2((*dpx_9).primal_0) * 0.69314718246459961f * dOut_9;
    (*dpx_9).primal_0 = (*dpx_9).primal_0;
    (*dpx_9).differential_0 = _S53;
    return;
}

fn s_bwdProp_exp2_0( ctx_8 : s_bwdCallableCtx_exp2_0,  d_x_8 : ptr<function, f32>,  d_out_8 : f32)
{
    var _S54 : DiffPair_float_0;
    _S54.primal_0 = ctx_8._S52;
    _d_exp2_0(&(_S54), d_out_8);
    (*d_x_8) = _S54.differential_0;
    return;
}

struct Tuple_9
{
     value0_9 : f32,
};

fn s_apply_log2_0( x_9 : f32) -> Tuple_9
{
    var _S55 : Tuple_9 = Tuple_9( log2(x_9) );
    return _S55;
}

struct s_bwdCallableCtx_log2_0
{
     _S56 : f32,
};

fn _d_log2_0( dpx_10 : ptr<function, DiffPair_float_0>,  dOut_10 : f32)
{
    var _S57 : f32 = 1.0f / ((*dpx_10).primal_0 * 0.69314718246459961f) * dOut_10;
    (*dpx_10).primal_0 = (*dpx_10).primal_0;
    (*dpx_10).differential_0 = _S57;
    return;
}

fn s_bwdProp_log2_0( ctx_9 : s_bwdCallableCtx_log2_0,  d_x_9 : ptr<function, f32>,  d_out_9 : f32)
{
    var _S58 : DiffPair_float_0;
    _S58.primal_0 = ctx_9._S56;
    _d_log2_0(&(_S58), d_out_9);
    (*d_x_9) = _S58.differential_0;
    return;
}

struct Tuple_10
{
     value0_10 : f32,
};

fn s_apply_min_0( x_10 : f32,  y_3 : f32) -> Tuple_10
{
    var _S59 : Tuple_10 = Tuple_10( min(x_10, y_3) );
    return _S59;
}

struct s_bwdCallableCtx_min_0
{
     _S60 : f32,
     _S61 : f32,
};

fn _d_min_0( dpx_11 : ptr<function, DiffPair_float_0>,  dpy_4 : ptr<function, DiffPair_float_0>,  dOut_11 : f32)
{
    var _S62 : DiffPair_float_0 = (*dpx_11);
    var _S63 : f32;
    if((((*dpx_11).primal_0) < ((*dpy_4).primal_0)))
    {
        _S63 = dOut_11;
    }
    else
    {
        if((((*dpx_11).primal_0) > ((*dpy_4).primal_0)))
        {
            _S63 = 0.0f;
        }
        else
        {
            _S63 = 0.5f * dOut_11;
        }
    }
    (*dpx_11).primal_0 = _S62.primal_0;
    (*dpx_11).differential_0 = _S63;
    var _S64 : DiffPair_float_0 = (*dpy_4);
    if((((*dpy_4).primal_0) < (_S62.primal_0)))
    {
        _S63 = dOut_11;
    }
    else
    {
        if((((*dpy_4).primal_0) > ((*dpx_11).primal_0)))
        {
            _S63 = 0.0f;
        }
        else
        {
            _S63 = 0.5f * dOut_11;
        }
    }
    (*dpy_4).primal_0 = _S64.primal_0;
    (*dpy_4).differential_0 = _S63;
    return;
}

fn s_bwdProp_min_0( ctx_10 : s_bwdCallableCtx_min_0,  d_x_10 : ptr<function, f32>,  d_y_3 : ptr<function, f32>,  d_out_10 : f32)
{
    var _S65 : DiffPair_float_0;
    _S65.primal_0 = ctx_10._S60;
    var _S66 : DiffPair_float_0;
    _S66.primal_0 = ctx_10._S61;
    _d_min_0(&(_S65), &(_S66), d_out_10);
    (*d_x_10) = _S65.differential_0;
    (*d_y_3) = _S66.differential_0;
    return;
}

struct Tuple_11
{
     value0_11 : f32,
};

fn s_apply_exp_0( x_11 : f32) -> Tuple_11
{
    var _S67 : Tuple_11 = Tuple_11( exp(x_11) );
    return _S67;
}

struct s_bwdCallableCtx_exp_0
{
     _S68 : f32,
};

fn _d_exp_0( dpx_12 : ptr<function, DiffPair_float_0>,  dOut_12 : f32)
{
    var _S69 : f32 = exp((*dpx_12).primal_0) * dOut_12;
    (*dpx_12).primal_0 = (*dpx_12).primal_0;
    (*dpx_12).differential_0 = _S69;
    return;
}

fn s_bwdProp_exp_0( ctx_11 : s_bwdCallableCtx_exp_0,  d_x_11 : ptr<function, f32>,  d_out_11 : f32)
{
    var _S70 : DiffPair_float_0;
    _S70.primal_0 = ctx_11._S68;
    _d_exp_0(&(_S70), d_out_11);
    (*d_x_11) = _S70.differential_0;
    return;
}

struct Tuple_12
{
     value0_12 : f32,
};

fn s_apply_length_0( x_12 : vec2<f32>) -> Tuple_12
{
    var _S71 : Tuple_12 = Tuple_12( length(x_12) );
    return _S71;
}

struct s_bwdCallableCtx_length_0
{
     _S72 : vec2<f32>,
};

struct s_paramCtx_s_bwdCallableCtx_length_impl_0
{
     value0_13 : vec2<f32>,
};

struct s_bwdCallableCtx_length_impl_0
{
     value0_14 : s_paramCtx_s_bwdCallableCtx_length_impl_0,
};

fn s_bwdProp_length_impl_0( _S73 : s_bwdCallableCtx_length_impl_0,  _S74 : ptr<function, vec2<f32>>,  _S75 : f32)
{
    var _S76 : f32 = _S73.value0_14.value0_13[i32(0)];
    var _S77 : f32 = _S73.value0_14.value0_13[i32(1)];
    var _S78 : s_bwdCallableCtx_sqrt_0;
    _S78._S24 = _S76 * _S76 + _S77 * _S77;
    var _S79 : f32 = 0.0f;
    s_bwdProp_sqrt_0(_S78, &(_S79), _S75);
    var _S80 : f32 = _S73.value0_14.value0_13[i32(1)] * _S79;
    var _S81 : f32 = _S80 + _S80;
    var _S82 : f32 = _S73.value0_14.value0_13[i32(0)] * _S79;
    var _S83 : f32 = _S82 + _S82;
    var _S84 : vec2<f32> = vec2<f32>(0.0f);
    _S84[i32(1)] = _S81;
    _S84[i32(0)] = _S83;
    (*_S74) = _S84;
    return;
}

fn _S85( _S86 : ptr<function, DiffPair_vectorx3Cfloatx2C2x3E_0>,  _S87 : f32)
{
    var _S88 : s_paramCtx_s_bwdCallableCtx_length_impl_0;
    _S88.value0_13 = (*_S86).primal_0;
    var _S89 : s_bwdCallableCtx_length_impl_0 = s_bwdCallableCtx_length_impl_0( _S88 );
    var _S90 : vec2<f32> = (*_S86).differential_0;
    s_bwdProp_length_impl_0(_S89, &(_S90), _S87);
    (*_S86).differential_0 = _S90;
    return;
}

fn s_bwdProp_length_0( ctx_12 : s_bwdCallableCtx_length_0,  d_x_12 : ptr<function, vec2<f32>>,  d_out_12 : f32)
{
    var _S91 : DiffPair_vectorx3Cfloatx2C2x3E_0;
    _S91.primal_0 = ctx_12._S72;
    _S85(&(_S91), d_out_12);
    (*d_x_12) = _S91.differential_0;
    return;
}

struct Tuple_13
{
     value0_15 : vec3<f32>,
};

fn s_apply_abs_1( x_13 : vec3<f32>) -> Tuple_13
{
    var _S92 : Tuple_13 = Tuple_13( abs(x_13) );
    return _S92;
}

struct s_bwdCallableCtx_abs_1
{
     _S93 : vec3<f32>,
};

struct DiffPair_vectorx3Cfloatx2C3x3E_0
{
     primal_0 : vec3<f32>,
     differential_0 : vec3<f32>,
};

fn _d_abs_vector_1( dpx_13 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_13 : vec3<f32>)
{
    var _S94 : vec3<f32> = vec3<f32>(0.0f);
    var _S95 : vec3<f32> = select(select(vec3<f32>(-1.0f), _S94, ((*dpx_13).primal_0) == _S94), vec3<f32>(1.0f), (((*dpx_13).primal_0) > _S94)) * dOut_13;
    (*dpx_13).primal_0 = (*dpx_13).primal_0;
    (*dpx_13).differential_0 = _S95;
    return;
}

fn s_bwdProp_abs_1( ctx_13 : s_bwdCallableCtx_abs_1,  d_x_13 : ptr<function, vec3<f32>>,  d_out_13 : vec3<f32>)
{
    var _S96 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S96.primal_0 = ctx_13._S93;
    _d_abs_vector_1(&(_S96), d_out_13);
    (*d_x_13) = _S96.differential_0;
    return;
}

struct Tuple_14
{
     value0_16 : f32,
};

fn s_apply_dot_1( x_14 : vec3<f32>,  y_4 : vec3<f32>) -> Tuple_14
{
    var _S97 : Tuple_14 = Tuple_14( dot(x_14, y_4) );
    return _S97;
}

struct s_bwdCallableCtx_dot_1
{
     _S98 : vec3<f32>,
     _S99 : vec3<f32>,
};

fn _d_dot_1( dpx_14 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dpy_5 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_14 : f32)
{
    var x_d_result_1 : vec3<f32>;
    x_d_result_1[i32(0)] = (*dpy_5).primal_0[i32(0)] * dOut_14;
    var y_d_result_1 : vec3<f32>;
    y_d_result_1[i32(0)] = (*dpx_14).primal_0[i32(0)] * dOut_14;
    x_d_result_1[i32(1)] = (*dpy_5).primal_0[i32(1)] * dOut_14;
    y_d_result_1[i32(1)] = (*dpx_14).primal_0[i32(1)] * dOut_14;
    x_d_result_1[i32(2)] = (*dpy_5).primal_0[i32(2)] * dOut_14;
    y_d_result_1[i32(2)] = (*dpx_14).primal_0[i32(2)] * dOut_14;
    (*dpx_14).primal_0 = (*dpx_14).primal_0;
    (*dpx_14).differential_0 = x_d_result_1;
    (*dpy_5).primal_0 = (*dpy_5).primal_0;
    (*dpy_5).differential_0 = y_d_result_1;
    return;
}

fn s_bwdProp_dot_1( ctx_14 : s_bwdCallableCtx_dot_1,  d_x_14 : ptr<function, vec3<f32>>,  d_y_4 : ptr<function, vec3<f32>>,  d_out_14 : f32)
{
    var _S100 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S100.primal_0 = ctx_14._S98;
    var _S101 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S101.primal_0 = ctx_14._S99;
    _d_dot_1(&(_S100), &(_S101), d_out_14);
    (*d_x_14) = _S100.differential_0;
    (*d_y_4) = _S101.differential_0;
    return;
}

struct Tuple_15
{
     value0_17 : f32,
};

fn s_apply_abs_2( x_15 : f32) -> Tuple_15
{
    var _S102 : Tuple_15 = Tuple_15( abs(x_15) );
    return _S102;
}

struct Tuple_16
{
     value0_18 : f32,
};

fn pow0_0( x_16 : f32,  y_5 : f32) -> f32
{
    if((x_16 <= 0.0f))
    {
        return 0.0f;
    }
    return pow(x_16, y_5);
}

fn s_apply_pow0_0( x_17 : f32,  y_6 : f32) -> Tuple_16
{
    var _S103 : Tuple_16 = Tuple_16( pow0_0(x_17, y_6) );
    return _S103;
}

struct s_bwdCallableCtx_pow0_0
{
     _S104 : f32,
     _S105 : f32,
};

fn pow0_backward_0( x_18 : ptr<function, DiffPair_float_0>,  y_7 : ptr<function, DiffPair_float_0>,  d_out_15 : f32)
{
    if((((*x_18).primal_0) <= 0.0f))
    {
        (*x_18).primal_0 = (*x_18).primal_0;
        (*x_18).differential_0 = 0.0f;
        (*y_7).primal_0 = (*y_7).primal_0;
        (*y_7).differential_0 = 0.0f;
        return;
    }
    var d_x_15 : f32 = d_out_15 * (*y_7).primal_0 * pow((*x_18).primal_0, (*y_7).primal_0 - 1.0f);
    var d_y_5 : f32 = d_out_15 * pow((*x_18).primal_0, (*y_7).primal_0) * log((*x_18).primal_0);
    (*x_18).primal_0 = (*x_18).primal_0;
    (*x_18).differential_0 = d_x_15;
    (*y_7).primal_0 = (*y_7).primal_0;
    (*y_7).differential_0 = d_y_5;
    return;
}

fn s_bwdProp_pow0_0( ctx_15 : s_bwdCallableCtx_pow0_0,  d_x_16 : ptr<function, f32>,  d_y_6 : ptr<function, f32>,  d_out_16 : f32)
{
    var _S106 : DiffPair_float_0;
    _S106.primal_0 = ctx_15._S104;
    var _S107 : DiffPair_float_0;
    _S107.primal_0 = ctx_15._S105;
    pow0_backward_0(&(_S106), &(_S107), d_out_16);
    (*d_x_16) = _S106.differential_0;
    (*d_y_6) = _S107.differential_0;
    return;
}

struct Tuple_17
{
     value0_19 : f32,
};

fn s_apply_max_1( x_19 : f32,  y_8 : f32) -> Tuple_17
{
    var _S108 : Tuple_17 = Tuple_17( max(x_19, y_8) );
    return _S108;
}

struct s_bwdCallableCtx_max_1
{
     _S109 : f32,
     _S110 : f32,
};

fn s_bwdProp_max_1( ctx_16 : s_bwdCallableCtx_max_1,  d_x_17 : ptr<function, f32>,  d_y_7 : ptr<function, f32>,  d_out_17 : f32)
{
    var _S111 : DiffPair_float_0;
    _S111.primal_0 = ctx_16._S109;
    var _S112 : DiffPair_float_0;
    _S112.primal_0 = ctx_16._S110;
    _d_max_0(&(_S111), &(_S112), d_out_17);
    (*d_x_17) = _S111.differential_0;
    (*d_y_7) = _S112.differential_0;
    return;
}

struct Tuple_18
{
     value0_20 : vec3<f32>,
};

fn s_apply_mul_0( left_0 : mat3x3<f32>,  right_0 : vec3<f32>) -> Tuple_18
{
    var _S113 : Tuple_18 = Tuple_18( (((right_0) * (left_0))) );
    return _S113;
}

struct s_bwdCallableCtx_mul_0
{
     _S114 : mat3x3<f32>,
     _S115 : vec3<f32>,
};

struct DiffPair_matrixx3Cfloatx2C3x2C3x3E_0
{
     primal_0 : mat3x3<f32>,
     differential_0 : mat3x3<f32>,
};

fn _d_mul_0( left_1 : ptr<function, DiffPair_matrixx3Cfloatx2C3x2C3x3E_0>,  right_1 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_15 : vec3<f32>)
{
    var _S116 : f32 = (*left_1).primal_0[i32(0)][i32(0)] * dOut_15[i32(0)];
    var left_d_result_1 : mat3x3<f32>;
    left_d_result_1[i32(0)][i32(0)] = (*right_1).primal_0[i32(0)] * dOut_15[i32(0)];
    var sum_0 : f32 = _S116 + (*left_1).primal_0[i32(1)][i32(0)] * dOut_15[i32(1)];
    left_d_result_1[i32(1)][i32(0)] = (*right_1).primal_0[i32(0)] * dOut_15[i32(1)];
    var sum_1 : f32 = sum_0 + (*left_1).primal_0[i32(2)][i32(0)] * dOut_15[i32(2)];
    left_d_result_1[i32(2)][i32(0)] = (*right_1).primal_0[i32(0)] * dOut_15[i32(2)];
    var right_d_result_1 : vec3<f32>;
    right_d_result_1[i32(0)] = sum_1;
    var _S117 : f32 = (*left_1).primal_0[i32(0)][i32(1)] * dOut_15[i32(0)];
    left_d_result_1[i32(0)][i32(1)] = (*right_1).primal_0[i32(1)] * dOut_15[i32(0)];
    var sum_2 : f32 = _S117 + (*left_1).primal_0[i32(1)][i32(1)] * dOut_15[i32(1)];
    left_d_result_1[i32(1)][i32(1)] = (*right_1).primal_0[i32(1)] * dOut_15[i32(1)];
    var sum_3 : f32 = sum_2 + (*left_1).primal_0[i32(2)][i32(1)] * dOut_15[i32(2)];
    left_d_result_1[i32(2)][i32(1)] = (*right_1).primal_0[i32(1)] * dOut_15[i32(2)];
    right_d_result_1[i32(1)] = sum_3;
    var _S118 : f32 = (*left_1).primal_0[i32(0)][i32(2)] * dOut_15[i32(0)];
    left_d_result_1[i32(0)][i32(2)] = (*right_1).primal_0[i32(2)] * dOut_15[i32(0)];
    var sum_4 : f32 = _S118 + (*left_1).primal_0[i32(1)][i32(2)] * dOut_15[i32(1)];
    left_d_result_1[i32(1)][i32(2)] = (*right_1).primal_0[i32(2)] * dOut_15[i32(1)];
    var sum_5 : f32 = sum_4 + (*left_1).primal_0[i32(2)][i32(2)] * dOut_15[i32(2)];
    left_d_result_1[i32(2)][i32(2)] = (*right_1).primal_0[i32(2)] * dOut_15[i32(2)];
    right_d_result_1[i32(2)] = sum_5;
    (*left_1).primal_0 = (*left_1).primal_0;
    (*left_1).differential_0 = left_d_result_1;
    (*right_1).primal_0 = (*right_1).primal_0;
    (*right_1).differential_0 = right_d_result_1;
    return;
}

fn s_bwdProp_mul_0( ctx_17 : s_bwdCallableCtx_mul_0,  d_left_0 : ptr<function, mat3x3<f32>>,  d_right_0 : ptr<function, vec3<f32>>,  d_out_18 : vec3<f32>)
{
    var _S119 : DiffPair_matrixx3Cfloatx2C3x2C3x3E_0;
    _S119.primal_0 = ctx_17._S114;
    var _S120 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S120.primal_0 = ctx_17._S115;
    _d_mul_0(&(_S119), &(_S120), d_out_18);
    (*d_left_0) = _S119.differential_0;
    (*d_right_0) = _S120.differential_0;
    return;
}

struct Tuple_19
{
     value0_21 : vec3<f32>,
};

fn s_apply_max_2( x_20 : vec3<f32>,  y_9 : vec3<f32>) -> Tuple_19
{
    var _S121 : Tuple_19 = Tuple_19( max(x_20, y_9) );
    return _S121;
}

struct s_bwdCallableCtx_max_2
{
     _S122 : vec3<f32>,
     _S123 : vec3<f32>,
};

fn _d_max_vector_1( dpx_15 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dpy_6 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_16 : vec3<f32>)
{
    var left_dp_2 : DiffPair_float_0;
    left_dp_2.primal_0 = (*dpx_15).primal_0[i32(0)];
    left_dp_2.differential_0 = 0.0f;
    var right_dp_2 : DiffPair_float_0;
    right_dp_2.primal_0 = (*dpy_6).primal_0[i32(0)];
    right_dp_2.differential_0 = 0.0f;
    _d_max_0(&(left_dp_2), &(right_dp_2), dOut_16[i32(0)]);
    var left_d_result_2 : vec3<f32>;
    left_d_result_2[i32(0)] = left_dp_2.differential_0;
    var right_d_result_2 : vec3<f32>;
    right_d_result_2[i32(0)] = right_dp_2.differential_0;
    var left_dp_3 : DiffPair_float_0;
    left_dp_3.primal_0 = (*dpx_15).primal_0[i32(1)];
    left_dp_3.differential_0 = 0.0f;
    var right_dp_3 : DiffPair_float_0;
    right_dp_3.primal_0 = (*dpy_6).primal_0[i32(1)];
    right_dp_3.differential_0 = 0.0f;
    _d_max_0(&(left_dp_3), &(right_dp_3), dOut_16[i32(1)]);
    left_d_result_2[i32(1)] = left_dp_3.differential_0;
    right_d_result_2[i32(1)] = right_dp_3.differential_0;
    var left_dp_4 : DiffPair_float_0;
    left_dp_4.primal_0 = (*dpx_15).primal_0[i32(2)];
    left_dp_4.differential_0 = 0.0f;
    var right_dp_4 : DiffPair_float_0;
    right_dp_4.primal_0 = (*dpy_6).primal_0[i32(2)];
    right_dp_4.differential_0 = 0.0f;
    _d_max_0(&(left_dp_4), &(right_dp_4), dOut_16[i32(2)]);
    left_d_result_2[i32(2)] = left_dp_4.differential_0;
    right_d_result_2[i32(2)] = right_dp_4.differential_0;
    (*dpx_15).primal_0 = (*dpx_15).primal_0;
    (*dpx_15).differential_0 = left_d_result_2;
    (*dpy_6).primal_0 = (*dpy_6).primal_0;
    (*dpy_6).differential_0 = right_d_result_2;
    return;
}

fn s_bwdProp_max_2( ctx_18 : s_bwdCallableCtx_max_2,  d_x_18 : ptr<function, vec3<f32>>,  d_y_8 : ptr<function, vec3<f32>>,  d_out_19 : vec3<f32>)
{
    var _S124 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S124.primal_0 = ctx_18._S122;
    var _S125 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S125.primal_0 = ctx_18._S123;
    _d_max_vector_1(&(_S124), &(_S125), d_out_19);
    (*d_x_18) = _S124.differential_0;
    (*d_y_8) = _S125.differential_0;
    return;
}

fn _d_sqrt_1( dpx_16 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S126 : DiffPair_float_0 = DiffPair_float_0( sqrt(dpx_16.primal_0), 0.5f / sqrt(max(1.00000001168609742e-07f, dpx_16.primal_0)) * dpx_16.differential_0 );
    return _S126;
}

fn s_fwd_length_impl_0( dpx_17 : DiffPair_vectorx3Cfloatx2C2x3E_0) -> DiffPair_float_0
{
    var _S127 : f32 = dpx_17.primal_0[i32(0)];
    var _S128 : f32 = dpx_17.differential_0[i32(0)] * dpx_17.primal_0[i32(0)];
    var _S129 : f32 = dpx_17.primal_0[i32(1)];
    var _S130 : f32 = dpx_17.differential_0[i32(1)] * dpx_17.primal_0[i32(1)];
    var _S131 : DiffPair_float_0 = DiffPair_float_0( _S127 * _S127 + _S129 * _S129, _S128 + _S128 + (_S130 + _S130) );
    var _S132 : DiffPair_float_0 = _d_sqrt_1(_S131);
    var _S133 : DiffPair_float_0 = DiffPair_float_0( _S132.primal_0, _S132.differential_0 );
    return _S133;
}

fn _d_dot_2( dpx_18 : DiffPair_vectorx3Cfloatx2C2x3E_0,  dpy_7 : DiffPair_vectorx3Cfloatx2C2x3E_0) -> DiffPair_float_0
{
    var _S134 : DiffPair_float_0 = DiffPair_float_0( dpx_18.primal_0[i32(0)] * dpy_7.primal_0[i32(0)] + dpx_18.primal_0[i32(1)] * dpy_7.primal_0[i32(1)], dpx_18.primal_0[i32(0)] * dpy_7.differential_0[i32(0)] + dpy_7.primal_0[i32(0)] * dpx_18.differential_0[i32(0)] + dpx_18.primal_0[i32(1)] * dpy_7.differential_0[i32(1)] + dpy_7.primal_0[i32(1)] * dpx_18.differential_0[i32(1)] );
    return _S134;
}

fn _d_abs_vector_2( dpx_19 : DiffPair_vectorx3Cfloatx2C2x3E_0) -> DiffPair_vectorx3Cfloatx2C2x3E_0
{
    var _S135 : vec2<f32> = vec2<f32>(0.0f);
    var _S136 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( abs(dpx_19.primal_0), select(select(vec2<f32>(-1.0f), _S135, (dpx_19.primal_0) == _S135), vec2<f32>(1.0f), ((dpx_19.primal_0) > _S135)) * dpx_19.differential_0 );
    return _S136;
}

fn _d_max_1( dpx_20 : DiffPair_float_0,  dpy_8 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S137 : f32 = max(dpx_20.primal_0, dpy_8.primal_0);
    var _S138 : f32;
    if(((dpx_20.primal_0) > (dpy_8.primal_0)))
    {
        _S138 = dpx_20.differential_0;
    }
    else
    {
        if(((dpx_20.primal_0) < (dpy_8.primal_0)))
        {
            _S138 = dpy_8.differential_0;
        }
        else
        {
            _S138 = 0.5f * (dpx_20.differential_0 + dpy_8.differential_0);
        }
    }
    var _S139 : DiffPair_float_0 = DiffPair_float_0( _S137, _S138 );
    return _S139;
}

fn _d_max_vector_2( dpx_21 : DiffPair_vectorx3Cfloatx2C2x3E_0,  dpy_9 : DiffPair_vectorx3Cfloatx2C2x3E_0) -> DiffPair_vectorx3Cfloatx2C2x3E_0
{
    var _S140 : DiffPair_float_0 = DiffPair_float_0( dpx_21.primal_0[i32(0)], dpx_21.differential_0[i32(0)] );
    var _S141 : DiffPair_float_0 = DiffPair_float_0( dpy_9.primal_0[i32(0)], dpy_9.differential_0[i32(0)] );
    var dp_elem_0 : DiffPair_float_0 = _d_max_1(_S140, _S141);
    var result_0 : vec2<f32>;
    result_0[i32(0)] = dp_elem_0.primal_0;
    var d_result_0 : vec2<f32>;
    d_result_0[i32(0)] = dp_elem_0.differential_0;
    var _S142 : DiffPair_float_0 = DiffPair_float_0( dpx_21.primal_0[i32(1)], dpx_21.differential_0[i32(1)] );
    var _S143 : DiffPair_float_0 = DiffPair_float_0( dpy_9.primal_0[i32(1)], dpy_9.differential_0[i32(1)] );
    var dp_elem_1 : DiffPair_float_0 = _d_max_1(_S142, _S143);
    result_0[i32(1)] = dp_elem_1.primal_0;
    d_result_0[i32(1)] = dp_elem_1.differential_0;
    var _S144 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( result_0, d_result_0 );
    return _S144;
}

fn _d_cos_1( dpx_22 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S145 : DiffPair_float_0 = DiffPair_float_0( cos(dpx_22.primal_0), - sin(dpx_22.primal_0) * dpx_22.differential_0 );
    return _S145;
}

fn _d_sin_1( dpx_23 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S146 : DiffPair_float_0 = DiffPair_float_0( sin(dpx_23.primal_0), cos(dpx_23.primal_0) * dpx_23.differential_0 );
    return _S146;
}

fn _d_atan2_1( dpy_10 : DiffPair_float_0,  dpx_24 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S147 : f32 = dpx_24.primal_0;
    var _S148 : f32 = _S147 * _S147 + dpy_10.primal_0 * dpy_10.primal_0;
    var _S149 : DiffPair_float_0 = DiffPair_float_0( atan2(dpy_10.primal_0, dpx_24.primal_0), - dpy_10.primal_0 / _S148 * dpx_24.differential_0 + dpx_24.primal_0 / _S148 * dpy_10.differential_0 );
    return _S149;
}

fn _d_exp2_1( dpx_25 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S150 : f32 = exp2(dpx_25.primal_0);
    var _S151 : DiffPair_float_0 = DiffPair_float_0( _S150, _S150 * 0.69314718246459961f * dpx_25.differential_0 );
    return _S151;
}

fn _d_log2_1( dpx_26 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S152 : DiffPair_float_0 = DiffPair_float_0( log2(dpx_26.primal_0), 1.0f / (dpx_26.primal_0 * 0.69314718246459961f) * dpx_26.differential_0 );
    return _S152;
}

fn _d_abs_vector_3( dpx_27 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S153 : vec3<f32> = vec3<f32>(0.0f);
    var _S154 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( abs(dpx_27.primal_0), select(select(vec3<f32>(-1.0f), _S153, (dpx_27.primal_0) == _S153), vec3<f32>(1.0f), ((dpx_27.primal_0) > _S153)) * dpx_27.differential_0 );
    return _S154;
}

fn _d_min_1( dpx_28 : DiffPair_float_0,  dpy_11 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S155 : f32 = min(dpx_28.primal_0, dpy_11.primal_0);
    var _S156 : f32;
    if(((dpx_28.primal_0) < (dpy_11.primal_0)))
    {
        _S156 = dpx_28.differential_0;
    }
    else
    {
        if(((dpx_28.primal_0) > (dpy_11.primal_0)))
        {
            _S156 = dpy_11.differential_0;
        }
        else
        {
            _S156 = 0.5f * (dpx_28.differential_0 + dpy_11.differential_0);
        }
    }
    var _S157 : DiffPair_float_0 = DiffPair_float_0( _S155, _S156 );
    return _S157;
}

fn _d_dot_3( dpx_29 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpy_12 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_float_0
{
    var _S158 : DiffPair_float_0 = DiffPair_float_0( dpx_29.primal_0[i32(0)] * dpy_12.primal_0[i32(0)] + dpx_29.primal_0[i32(1)] * dpy_12.primal_0[i32(1)] + dpx_29.primal_0[i32(2)] * dpy_12.primal_0[i32(2)], dpx_29.primal_0[i32(0)] * dpy_12.differential_0[i32(0)] + dpy_12.primal_0[i32(0)] * dpx_29.differential_0[i32(0)] + dpx_29.primal_0[i32(1)] * dpy_12.differential_0[i32(1)] + dpy_12.primal_0[i32(1)] * dpx_29.differential_0[i32(1)] + dpx_29.primal_0[i32(2)] * dpy_12.differential_0[i32(2)] + dpy_12.primal_0[i32(2)] * dpx_29.differential_0[i32(2)] );
    return _S158;
}

fn _d_abs_0( dpx_30 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S159 : DiffPair_float_0 = DiffPair_float_0( abs(dpx_30.primal_0), select(select(-1.0f, 0.0f, (dpx_30.primal_0) == 0.0f), 1.0f, ((dpx_30.primal_0) > 0.0f)) * dpx_30.differential_0 );
    return _S159;
}

fn _d_exp_1( dpx_31 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S160 : f32 = exp(dpx_31.primal_0);
    var _S161 : DiffPair_float_0 = DiffPair_float_0( _S160, _S160 * dpx_31.differential_0 );
    return _S161;
}

fn _d_clamp_1( dpx_32 : DiffPair_float_0,  dpMin_1 : DiffPair_float_0,  dpMax_1 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S162 : f32 = clamp(dpx_32.primal_0, dpMin_1.primal_0, dpMax_1.primal_0);
    var _S163 : bool;
    if(((dpx_32.primal_0) >= (dpMin_1.primal_0)))
    {
        _S163 = ((dpx_32.primal_0) <= (dpMax_1.primal_0));
    }
    else
    {
        _S163 = false;
    }
    var _S164 : f32;
    if(_S163)
    {
        _S164 = dpx_32.differential_0;
    }
    else
    {
        if(((dpx_32.primal_0) < (dpMin_1.primal_0)))
        {
            _S164 = dpMin_1.differential_0;
        }
        else
        {
            _S164 = dpMax_1.differential_0;
        }
    }
    var _S165 : DiffPair_float_0 = DiffPair_float_0( _S162, _S164 );
    return _S165;
}

fn _d_max_vector_3( dpx_33 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpy_13 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S166 : DiffPair_float_0 = DiffPair_float_0( dpx_33.primal_0[i32(0)], dpx_33.differential_0[i32(0)] );
    var _S167 : DiffPair_float_0 = DiffPair_float_0( dpy_13.primal_0[i32(0)], dpy_13.differential_0[i32(0)] );
    var dp_elem_2 : DiffPair_float_0 = _d_max_1(_S166, _S167);
    var result_1 : vec3<f32>;
    result_1[i32(0)] = dp_elem_2.primal_0;
    var d_result_1 : vec3<f32>;
    d_result_1[i32(0)] = dp_elem_2.differential_0;
    var _S168 : DiffPair_float_0 = DiffPair_float_0( dpx_33.primal_0[i32(1)], dpx_33.differential_0[i32(1)] );
    var _S169 : DiffPair_float_0 = DiffPair_float_0( dpy_13.primal_0[i32(1)], dpy_13.differential_0[i32(1)] );
    var dp_elem_3 : DiffPair_float_0 = _d_max_1(_S168, _S169);
    result_1[i32(1)] = dp_elem_3.primal_0;
    d_result_1[i32(1)] = dp_elem_3.differential_0;
    var _S170 : DiffPair_float_0 = DiffPair_float_0( dpx_33.primal_0[i32(2)], dpx_33.differential_0[i32(2)] );
    var _S171 : DiffPair_float_0 = DiffPair_float_0( dpy_13.primal_0[i32(2)], dpy_13.differential_0[i32(2)] );
    var dp_elem_4 : DiffPair_float_0 = _d_max_1(_S170, _S171);
    result_1[i32(2)] = dp_elem_4.primal_0;
    d_result_1[i32(2)] = dp_elem_4.differential_0;
    var _S172 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( result_1, d_result_1 );
    return _S172;
}

struct Grading_0
{
     chroma_global_1 : f32,
     contrast_1 : f32,
     saturation_global_1 : f32,
     brilliance_global_1 : f32,
};

fn Grading_x24_syn_dadd_0( SLANG_anonymous_0_0 : Grading_0,  SLANG_anonymous_1_0 : Grading_0) -> Grading_0
{
    var result_2 : Grading_0;
    result_2.chroma_global_1 = SLANG_anonymous_0_0.chroma_global_1 + SLANG_anonymous_1_0.chroma_global_1;
    result_2.contrast_1 = SLANG_anonymous_0_0.contrast_1 + SLANG_anonymous_1_0.contrast_1;
    result_2.saturation_global_1 = SLANG_anonymous_0_0.saturation_global_1 + SLANG_anonymous_1_0.saturation_global_1;
    result_2.brilliance_global_1 = SLANG_anonymous_0_0.brilliance_global_1 + SLANG_anonymous_1_0.brilliance_global_1;
    return result_2;
}

fn Grading_x24_syn_dzero_0() -> Grading_0
{
    var result_3 : Grading_0;
    result_3.chroma_global_1 = 0.0f;
    result_3.contrast_1 = 0.0f;
    result_3.saturation_global_1 = 0.0f;
    result_3.brilliance_global_1 = 0.0f;
    return result_3;
}

fn lms_to_yrg_0( lms_0 : vec3<f32>) -> vec3<f32>
{
    var _S173 : f32 = lms_0.x;
    var _S174 : f32 = lms_0.y;
    var y_10 : f32 = 0.68990272283554077f * _S173 + 0.34832188487052917f * _S174;
    var a_0 : f32 = _S173 + _S174 + lms_0.z;
    var _S175 : vec3<f32> = vec3<f32>(0.0f);
    var normalized_0 : vec3<f32>;
    if(a_0 != 0.0f)
    {
        normalized_0 = lms_0 / vec3<f32>(a_0);
    }
    else
    {
        normalized_0 = _S175;
    }
    var rgb_0 : vec3<f32> = (((normalized_0) * (mat3x3<f32>(1.0877193212509155f, -0.66666668653488159f, 0.020618559792637825f, -0.087719298899173737f, 1.6666666269302368f, -0.051546391099691391f, 0.0f, 0.0f, 1.0309278964996338f))));
    return vec3<f32>(y_10, rgb_0.x, rgb_0.y);
}

struct Tuple_20
{
     value0_22 : vec3<f32>,
};

fn s_apply_lms_to_yrg_0( _S176 : vec3<f32>) -> Tuple_20
{
    var _S177 : f32 = _S176.x;
    var _S178 : f32 = _S176.y;
    var y_11 : f32 = 0.68990272283554077f * _S177 + 0.34832188487052917f * _S178;
    var a_1 : f32 = _S177 + _S178 + _S176.z;
    var _S179 : vec3<f32> = vec3<f32>(0.0f);
    var normalized_1 : vec3<f32>;
    if(a_1 != 0.0f)
    {
        normalized_1 = _S176 / vec3<f32>(a_1);
    }
    else
    {
        normalized_1 = _S179;
    }
    var _S180 : Tuple_18 = s_apply_mul_0(mat3x3<f32>(1.0877193212509155f, -0.66666668653488159f, 0.020618559792637825f, -0.087719298899173737f, 1.6666666269302368f, -0.051546391099691391f, 0.0f, 0.0f, 1.0309278964996338f), normalized_1);
    var _S181 : Tuple_20;
    _S181.value0_22 = vec3<f32>(y_11, _S180.value0_20.x, _S180.value0_20.y);
    return _S181;
}

struct s_paramCtx_s_bwdCallableCtx_lms_to_yrg_0
{
     value0_23 : vec3<f32>,
};

struct s_bwdCallableCtx_lms_to_yrg_0
{
     value0_24 : s_paramCtx_s_bwdCallableCtx_lms_to_yrg_0,
};

fn s_bwdProp_lms_to_yrg_0( _S182 : s_bwdCallableCtx_lms_to_yrg_0,  _S183 : ptr<function, vec3<f32>>,  _S184 : vec3<f32>)
{
    var a_2 : f32 = _S182.value0_24.value0_23.x + _S182.value0_24.value0_23.y + _S182.value0_24.value0_23.z;
    var _S185 : vec3<f32> = vec3<f32>(0.0f);
    var _S186 : bool = a_2 != 0.0f;
    var normalized_2 : vec3<f32>;
    var _S187 : f32;
    if(_S186)
    {
        var _S188 : f32 = a_2 * a_2;
        normalized_2 = _S182.value0_24.value0_23 / vec3<f32>(a_2);
        _S187 = _S188;
    }
    else
    {
        normalized_2 = _S185;
        _S187 = 0.0f;
    }
    var _S189 : s_bwdCallableCtx_mul_0;
    _S189._S114 = mat3x3<f32>(1.0877193212509155f, -0.66666668653488159f, 0.020618559792637825f, -0.087719298899173737f, 1.6666666269302368f, -0.051546391099691391f, 0.0f, 0.0f, 1.0309278964996338f);
    _S189._S115 = normalized_2;
    var _S190 : s_bwdCallableCtx_mul_0 = _S189;
    var _S191 : vec3<f32> = vec3<f32>(0.0f);
    var _S192 : vec3<f32> = vec3<f32>(_S184[i32(1)], _S184[i32(2)], 0.0f);
    var _S193 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S194 : vec3<f32> = _S191;
    s_bwdProp_mul_0(_S190, &(_S193), &(_S194), _S192);
    var _S195 : vec3<f32> = _S194;
    if(_S186)
    {
        var _S196 : vec3<f32> = _S195 / vec3<f32>(_S187);
        var _S197 : vec3<f32> = _S182.value0_24.value0_23 * (vec3<f32>(0) - _S196);
        var _S198 : vec3<f32> = vec3<f32>(a_2) * _S196;
        _S187 = _S197[i32(0)] + _S197[i32(1)] + _S197[i32(2)];
        normalized_2 = _S198;
    }
    else
    {
        _S187 = 0.0f;
        normalized_2 = _S191;
    }
    (*_S183) = normalized_2 + vec3<f32>(_S187 + 0.68990272283554077f * _S184[i32(0)], _S187 + 0.34832188487052917f * _S184[i32(0)], _S187);
    return;
}

fn s_fwd_lms_to_yrg_0( dplms_0 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S199 : f32 = dplms_0.primal_0.x;
    var _S200 : f32 = dplms_0.differential_0.x;
    var _S201 : f32 = dplms_0.primal_0.y;
    var _S202 : f32 = dplms_0.differential_0.y;
    var y_12 : f32 = 0.68990272283554077f * _S199 + 0.34832188487052917f * _S201;
    var s_diff_y_0 : f32 = _S200 * 0.68990272283554077f + _S202 * 0.34832188487052917f;
    var a_3 : f32 = _S199 + _S201 + dplms_0.primal_0.z;
    var s_diff_a_0 : f32 = _S200 + _S202 + dplms_0.differential_0.z;
    var _S203 : vec3<f32> = vec3<f32>(0.0f);
    var _S204 : vec3<f32> = vec3<f32>(0.0f);
    var normalized_3 : vec3<f32>;
    var s_diff_normalized_0 : vec3<f32>;
    if(a_3 != 0.0f)
    {
        var _S205 : vec3<f32> = vec3<f32>(a_3);
        var _S206 : vec3<f32> = (dplms_0.differential_0 * _S205 - dplms_0.primal_0 * vec3<f32>(s_diff_a_0)) / vec3<f32>((a_3 * a_3));
        normalized_3 = dplms_0.primal_0 / _S205;
        s_diff_normalized_0 = _S206;
    }
    else
    {
        normalized_3 = _S203;
        s_diff_normalized_0 = _S204;
    }
    var _S207 : vec3<f32> = (((normalized_3) * (mat3x3<f32>(1.0877193212509155f, -0.66666668653488159f, 0.020618559792637825f, -0.087719298899173737f, 1.6666666269302368f, -0.051546391099691391f, 0.0f, 0.0f, 1.0309278964996338f))));
    var _S208 : vec3<f32> = (((normalized_3) * (mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f)))) + (((s_diff_normalized_0) * (mat3x3<f32>(1.0877193212509155f, -0.66666668653488159f, 0.020618559792637825f, -0.087719298899173737f, 1.6666666269302368f, -0.051546391099691391f, 0.0f, 0.0f, 1.0309278964996338f))));
    var _S209 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(y_12, _S207.x, _S207.y), vec3<f32>(s_diff_y_0, _S208.x, _S208.y) );
    return _S209;
}

fn yrg_to_ych_0( yrg_0 : vec3<f32>) -> vec4<f32>
{
    var rg_0 : vec2<f32> = yrg_0.yz - vec2<f32>(0.21902142465114594f, 0.54371398687362671f);
    var c_0 : f32 = length(rg_0);
    const _S210 : vec2<f32> = vec2<f32>(1.0f, 0.0f);
    var cos_sin_0 : vec2<f32>;
    if(c_0 != 0.0f)
    {
        cos_sin_0 = rg_0 / vec2<f32>(c_0);
    }
    else
    {
        cos_sin_0 = _S210;
    }
    return vec4<f32>(yrg_0.x, c_0, cos_sin_0.x, cos_sin_0.y);
}

struct Tuple_21
{
     value0_25 : vec4<f32>,
};

fn s_apply_yrg_to_ych_0( _S211 : vec3<f32>) -> Tuple_21
{
    var rg_1 : vec2<f32> = _S211.yz - vec2<f32>(0.21902142465114594f, 0.54371398687362671f);
    var _S212 : Tuple_12 = s_apply_length_0(rg_1);
    const _S213 : vec2<f32> = vec2<f32>(1.0f, 0.0f);
    var cos_sin_1 : vec2<f32>;
    if((_S212.value0_12) != 0.0f)
    {
        cos_sin_1 = rg_1 / vec2<f32>(_S212.value0_12);
    }
    else
    {
        cos_sin_1 = _S213;
    }
    var _S214 : Tuple_21;
    _S214.value0_25 = vec4<f32>(_S211.x, _S212.value0_12, cos_sin_1.x, cos_sin_1.y);
    return _S214;
}

struct DiffPair_vectorx3Cfloatx2C4x3E_0
{
     primal_0 : vec4<f32>,
     differential_0 : vec4<f32>,
};

fn s_fwd_yrg_to_ych_0( dpyrg_0 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C4x3E_0
{
    var _S215 : vec2<f32> = dpyrg_0.differential_0.yz;
    var rg_2 : vec2<f32> = dpyrg_0.primal_0.yz - vec2<f32>(0.21902142465114594f, 0.54371398687362671f);
    var _S216 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( rg_2, _S215 );
    var _S217 : DiffPair_float_0 = s_fwd_length_impl_0(_S216);
    const _S218 : vec2<f32> = vec2<f32>(1.0f, 0.0f);
    const _S219 : vec2<f32> = vec2<f32>(0.0f, 0.0f);
    var cos_sin_2 : vec2<f32>;
    var s_diff_cos_sin_0 : vec2<f32>;
    if((_S217.primal_0) != 0.0f)
    {
        var _S220 : vec2<f32> = vec2<f32>(_S217.primal_0);
        var _S221 : vec2<f32> = (_S215 * _S220 - rg_2 * vec2<f32>(_S217.differential_0)) / vec2<f32>((_S217.primal_0 * _S217.primal_0));
        cos_sin_2 = rg_2 / _S220;
        s_diff_cos_sin_0 = _S221;
    }
    else
    {
        cos_sin_2 = _S218;
        s_diff_cos_sin_0 = _S219;
    }
    var _S222 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( vec4<f32>(dpyrg_0.primal_0.x, _S217.primal_0, cos_sin_2.x, cos_sin_2.y), vec4<f32>(dpyrg_0.differential_0.x, _S217.differential_0, s_diff_cos_sin_0.x, s_diff_cos_sin_0.y) );
    return _S222;
}

fn pow0_forward_0( x_21 : DiffPair_float_0,  y_13 : DiffPair_float_0) -> DiffPair_float_0
{
    if(((x_21.primal_0) <= 0.0f))
    {
        var _S223 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
        return _S223;
    }
    var value_0 : f32 = pow(x_21.primal_0, y_13.primal_0);
    var _S224 : DiffPair_float_0 = DiffPair_float_0( value_0, x_21.differential_0 * y_13.primal_0 * pow(x_21.primal_0, y_13.primal_0 - 1.0f) + y_13.differential_0 * value_0 * log(x_21.primal_0) );
    return _S224;
}

fn logistic_0( t_0 : f32) -> f32
{
    return 1.0f / (1.0f + exp(clamp(t_0, -80.0f, 80.0f)));
}

struct Tuple_22
{
     value0_26 : f32,
};

fn s_apply_logistic_0( _S225 : f32) -> Tuple_22
{
    var _S226 : Tuple_22;
    _S226.value0_26 = 1.0f / (1.0f + s_apply_exp_0(s_apply_clamp_0(_S225, -80.0f, 80.0f).value0_2).value0_11);
    return _S226;
}

fn s_fwd_logistic_0( dpt_0 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S227 : DiffPair_float_0 = DiffPair_float_0( dpt_0.primal_0, dpt_0.differential_0 );
    var _S228 : DiffPair_float_0 = DiffPair_float_0( -80.0f, 0.0f );
    var _S229 : DiffPair_float_0 = DiffPair_float_0( 80.0f, 0.0f );
    var _S230 : DiffPair_float_0 = _d_clamp_1(_S227, _S228, _S229);
    var _S231 : DiffPair_float_0 = DiffPair_float_0( _S230.primal_0, _S230.differential_0 );
    var _S232 : DiffPair_float_0 = _d_exp_1(_S231);
    var _S233 : f32 = 1.0f + _S232.primal_0;
    var _S234 : DiffPair_float_0 = DiffPair_float_0( 1.0f / _S233, (0.0f - _S232.differential_0) / (_S233 * _S233) );
    return _S234;
}

struct Tuple_23
{
     value0_27 : vec3<f32>,
};

struct ColorBalanceParams_0
{
     input_r0_0 : vec4<f32>,
     input_r1_0 : vec4<f32>,
     input_r2_0 : vec4<f32>,
     output_r0_0 : vec4<f32>,
     output_r1_0 : vec4<f32>,
     output_r2_0 : vec4<f32>,
     global_0 : vec4<f32>,
     shadows_0 : vec4<f32>,
     highlights_0 : vec4<f32>,
     midtones_0 : vec4<f32>,
     chroma_0 : vec4<f32>,
     saturation_0 : vec4<f32>,
     brilliance_0 : vec4<f32>,
     chroma_global_0 : f32,
     vibrance_0 : f32,
     contrast_0 : f32,
     saturation_global_0 : f32,
     brilliance_global_0 : f32,
     hue_cos_0 : f32,
     hue_sin_0 : f32,
     shadows_weight_0 : f32,
     highlights_weight_0 : f32,
     midtones_weight_0 : f32,
     mask_grey_fulcrum_0 : f32,
     white_fulcrum_0 : f32,
     grey_fulcrum_0 : f32,
     midtones_y_0 : f32,
     l_white_0 : f32,
     saturation_formula_0 : u32,
};

fn s_apply_opacity_masks_0( _S235 : f32,  p_0 : ColorBalanceParams_0) -> Tuple_23
{
    var offset_0 : f32 = _S235 - p_0.mask_grey_fulcrum_0;
    var norm_0 : f32 = offset_0 / p_0.mask_grey_fulcrum_0;
    var _S236 : Tuple_22 = s_apply_logistic_0(norm_0 * p_0.shadows_weight_0);
    var _S237 : Tuple_22 = s_apply_logistic_0(- norm_0 * p_0.highlights_weight_0);
    var a_4 : f32 = 1.0f - _S236.value0_26;
    var b_0 : f32 = 1.0f - _S237.value0_26;
    var _S238 : Tuple_23;
    _S238.value0_27 = vec3<f32>(_S236.value0_26, s_apply_exp_0(- offset_0 * offset_0 * p_0.midtones_weight_0 / 4.0f).value0_11 * a_4 * a_4 * b_0 * b_0 * 8.0f, _S237.value0_26);
    return _S238;
}

fn ych_to_yrg_0( ych_0 : vec4<f32>) -> vec3<f32>
{
    var rg_3 : vec2<f32> = vec2<f32>(ych_0.y) * ych_0.zw + vec2<f32>(0.21902142465114594f, 0.54371398687362671f);
    return vec3<f32>(ych_0.x, rg_3.x, rg_3.y);
}

struct Tuple_24
{
     value0_28 : vec3<f32>,
};

fn s_apply_ych_to_yrg_0( _S239 : vec4<f32>) -> Tuple_24
{
    var rg_4 : vec2<f32> = vec2<f32>(_S239.y) * _S239.zw + vec2<f32>(0.21902142465114594f, 0.54371398687362671f);
    var _S240 : Tuple_24;
    _S240.value0_28 = vec3<f32>(_S239.x, rg_4.x, rg_4.y);
    return _S240;
}

struct s_paramCtx_s_bwdCallableCtx_ych_to_yrg_0
{
     value0_29 : vec4<f32>,
};

struct s_bwdCallableCtx_ych_to_yrg_0
{
     value0_30 : s_paramCtx_s_bwdCallableCtx_ych_to_yrg_0,
};

fn s_bwdProp_ych_to_yrg_0( _S241 : s_bwdCallableCtx_ych_to_yrg_0,  _S242 : ptr<function, vec4<f32>>,  _S243 : vec3<f32>)
{
    var s_diff_rg_T_0 : vec2<f32> = vec2<f32>(_S243[i32(1)], _S243[i32(2)]);
    var _S244 : vec2<f32> = vec2<f32>(_S241.value0_30.value0_29.y) * s_diff_rg_T_0;
    var _S245 : vec2<f32> = _S241.value0_30.value0_29.zw * s_diff_rg_T_0;
    (*_S242) = vec4<f32>(_S243[i32(0)], _S245[i32(0)] + _S245[i32(1)], _S244[i32(0)], _S244[i32(1)]);
    return;
}

fn s_fwd_ych_to_yrg_0( dpych_0 : DiffPair_vectorx3Cfloatx2C4x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S246 : vec2<f32> = dpych_0.primal_0.zw;
    var _S247 : vec2<f32> = vec2<f32>(dpych_0.primal_0.y);
    var _S248 : vec2<f32> = vec2<f32>(dpych_0.differential_0.y) * _S246 + dpych_0.differential_0.zw * _S247;
    var rg_5 : vec2<f32> = _S247 * _S246 + vec2<f32>(0.21902142465114594f, 0.54371398687362671f);
    var _S249 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(dpych_0.primal_0.x, rg_5.x, rg_5.y), vec3<f32>(dpych_0.differential_0.x, _S248.x, _S248.y) );
    return _S249;
}

fn gamut_check_yrg_0( ych_1 : vec4<f32>) -> vec4<f32>
{
    var yrg_1 : vec3<f32> = ych_to_yrg_0(ych_1);
    var max_c_0 : f32 = ych_1.y;
    var _S250 : f32 = yrg_1.y;
    var max_c_1 : f32;
    if((_S250 < 0.0f))
    {
        max_c_1 = min(-0.21902142465114594f / ych_1.z, max_c_0);
    }
    else
    {
        max_c_1 = max_c_0;
    }
    var _S251 : f32 = yrg_1.z;
    if((_S251 < 0.0f))
    {
        max_c_1 = min(-0.54371398687362671f / ych_1.w, max_c_1);
    }
    if(((_S250 + _S251) > 1.0f))
    {
        max_c_1 = min(0.23726457357406616f / (ych_1.z + ych_1.w), max_c_1);
    }
    return vec4<f32>(ych_1.x, max_c_1, ych_1.z, ych_1.w);
}

struct Tuple_25
{
     value0_31 : vec4<f32>,
};

fn s_apply_gamut_check_yrg_0( _S252 : vec4<f32>) -> Tuple_25
{
    var _S253 : Tuple_24 = s_apply_ych_to_yrg_0(_S252);
    var max_c_2 : f32 = _S252.y;
    var _S254 : f32 = _S253.value0_28.y;
    var max_c_3 : f32;
    if((_S254 < 0.0f))
    {
        max_c_3 = s_apply_min_0(-0.21902142465114594f / _S252.z, max_c_2).value0_10;
    }
    else
    {
        max_c_3 = max_c_2;
    }
    var _S255 : f32 = _S253.value0_28.z;
    if((_S255 < 0.0f))
    {
        max_c_3 = s_apply_min_0(-0.54371398687362671f / _S252.w, max_c_3).value0_10;
    }
    if(((_S254 + _S255) > 1.0f))
    {
        max_c_3 = s_apply_min_0(0.23726457357406616f / (_S252.z + _S252.w), max_c_3).value0_10;
    }
    var _S256 : Tuple_25;
    _S256.value0_31 = vec4<f32>(_S252.x, max_c_3, _S252.z, _S252.w);
    return _S256;
}

struct s_paramCtx_s_bwdCallableCtx_gamut_check_yrg_0
{
     value0_32 : vec4<f32>,
};

struct s_bwdCallableCtx_gamut_check_yrg_0
{
     value0_33 : s_paramCtx_s_bwdCallableCtx_gamut_check_yrg_0,
};

fn s_bwdProp_gamut_check_yrg_0( _S257 : s_bwdCallableCtx_gamut_check_yrg_0,  _S258 : ptr<function, vec4<f32>>,  _S259 : vec4<f32>)
{
    var _S260 : Tuple_24 = s_apply_ych_to_yrg_0(_S257.value0_33.value0_32);
    var _S261 : s_paramCtx_s_bwdCallableCtx_ych_to_yrg_0;
    _S261.value0_29 = _S257.value0_33.value0_32;
    var _S262 : s_bwdCallableCtx_ych_to_yrg_0 = s_bwdCallableCtx_ych_to_yrg_0( _S261 );
    var max_c_4 : f32 = _S257.value0_33.value0_32.y;
    var _S263 : f32 = _S260.value0_28.y;
    var _S264 : bool = (_S263 < 0.0f);
    var max_c_5 : f32;
    var _S265 : f32;
    var _S266 : s_bwdCallableCtx_min_0;
    if(_S264)
    {
        var _S267 : f32 = _S257.value0_33.value0_32.z;
        var _S268 : f32 = -0.21902142465114594f / _S267;
        var _S269 : f32 = _S267 * _S267;
        var _S270 : Tuple_10 = s_apply_min_0(_S268, max_c_4);
        var _S271 : s_bwdCallableCtx_min_0;
        _S271._S60 = _S268;
        _S271._S61 = max_c_4;
        max_c_5 = _S270.value0_10;
        _S266 = _S271;
        _S265 = _S269;
    }
    else
    {
        max_c_5 = max_c_4;
        _S266._S60 = 0.0f;
        _S266._S61 = 0.0f;
        _S265 = 0.0f;
    }
    var _S272 : f32 = _S260.value0_28.z;
    var _S273 : bool = (_S272 < 0.0f);
    var _S274 : f32;
    var _S275 : s_bwdCallableCtx_min_0;
    if(_S273)
    {
        var _S276 : f32 = _S257.value0_33.value0_32.w;
        var _S277 : f32 = -0.54371398687362671f / _S276;
        var _S278 : f32 = _S276 * _S276;
        var _S279 : Tuple_10 = s_apply_min_0(_S277, max_c_5);
        var _S280 : s_bwdCallableCtx_min_0;
        _S280._S60 = _S277;
        _S280._S61 = max_c_5;
        max_c_5 = _S279.value0_10;
        _S275 = _S280;
        _S274 = _S278;
    }
    else
    {
        _S275._S60 = 0.0f;
        _S275._S61 = 0.0f;
        _S274 = 0.0f;
    }
    var _S281 : bool = ((_S263 + _S272) > 1.0f);
    var _S282 : s_bwdCallableCtx_min_0;
    if(_S281)
    {
        var _S283 : f32 = _S257.value0_33.value0_32.z + _S257.value0_33.value0_32.w;
        var _S284 : f32 = _S283 * _S283;
        var _S285 : s_bwdCallableCtx_min_0;
        _S285._S60 = 0.23726457357406616f / _S283;
        _S285._S61 = max_c_5;
        _S282 = _S285;
        max_c_5 = _S284;
    }
    else
    {
        _S282._S60 = 0.0f;
        _S282._S61 = 0.0f;
        max_c_5 = 0.0f;
    }
    var _S286 : vec4<f32> = vec4<f32>(_S259[i32(0)], 0.0f, _S259[i32(2)], _S259[i32(3)]);
    var _S287 : vec4<f32>;
    if(_S281)
    {
        var _S288 : f32 = 0.0f;
        var _S289 : f32 = 0.0f;
        s_bwdProp_min_0(_S282, &(_S288), &(_S289), _S259[i32(1)]);
        var _S290 : f32 = 0.23726457357406616f * - (_S288 / max_c_5);
        var _S291 : vec4<f32> = _S286 + vec4<f32>(0.0f, 0.0f, _S290, _S290);
        max_c_5 = _S289;
        _S287 = _S291;
    }
    else
    {
        max_c_5 = _S259[i32(1)];
        _S287 = _S286;
    }
    if(_S273)
    {
        var _S292 : f32 = 0.0f;
        var _S293 : f32 = 0.0f;
        s_bwdProp_min_0(_S275, &(_S292), &(_S293), max_c_5);
        var _S294 : vec4<f32> = _S287 + vec4<f32>(0.0f, 0.0f, 0.0f, -0.54371398687362671f * - (_S292 / _S274));
        max_c_5 = _S293;
        _S287 = _S294;
    }
    if(_S264)
    {
        var _S295 : f32 = 0.0f;
        var _S296 : f32 = 0.0f;
        s_bwdProp_min_0(_S266, &(_S295), &(_S296), max_c_5);
        var _S297 : vec4<f32> = _S287 + vec4<f32>(0.0f, 0.0f, -0.21902142465114594f * - (_S295 / _S265), 0.0f);
        max_c_5 = _S296;
        _S287 = _S297;
    }
    const _S298 : vec3<f32> = vec3<f32>(0.0f, 0.0f, 0.0f);
    var _S299 : vec4<f32> = vec4<f32>(0.0f);
    s_bwdProp_ych_to_yrg_0(_S262, &(_S299), _S298);
    (*_S258) = _S299 + _S287 + vec4<f32>(0.0f, max_c_5, 0.0f, 0.0f);
    return;
}

fn s_fwd_gamut_check_yrg_0( dpych_1 : DiffPair_vectorx3Cfloatx2C4x3E_0) -> DiffPair_vectorx3Cfloatx2C4x3E_0
{
    var _S300 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( dpych_1.primal_0, dpych_1.differential_0 );
    var _S301 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_ych_to_yrg_0(_S300);
    var max_c_6 : f32 = dpych_1.primal_0.y;
    var s_diff_max_c_0 : f32 = dpych_1.differential_0.y;
    var _S302 : f32 = _S301.primal_0.y;
    var max_c_7 : f32;
    var s_diff_max_c_1 : f32;
    if((_S302 < 0.0f))
    {
        var _S303 : f32 = dpych_1.primal_0.z;
        var _S304 : DiffPair_float_0 = DiffPair_float_0( -0.21902142465114594f / _S303, (0.0f - -0.21902142465114594f * dpych_1.differential_0.z) / (_S303 * _S303) );
        var _S305 : DiffPair_float_0 = DiffPair_float_0( max_c_6, s_diff_max_c_0 );
        var _S306 : DiffPair_float_0 = _d_min_1(_S304, _S305);
        max_c_7 = _S306.primal_0;
        s_diff_max_c_1 = _S306.differential_0;
    }
    else
    {
        max_c_7 = max_c_6;
        s_diff_max_c_1 = s_diff_max_c_0;
    }
    var _S307 : f32 = _S301.primal_0.z;
    if((_S307 < 0.0f))
    {
        var _S308 : f32 = dpych_1.primal_0.w;
        var _S309 : DiffPair_float_0 = DiffPair_float_0( -0.54371398687362671f / _S308, (0.0f - -0.54371398687362671f * dpych_1.differential_0.w) / (_S308 * _S308) );
        var _S310 : DiffPair_float_0 = DiffPair_float_0( max_c_7, s_diff_max_c_1 );
        var _S311 : DiffPair_float_0 = _d_min_1(_S309, _S310);
        max_c_7 = _S311.primal_0;
        s_diff_max_c_1 = _S311.differential_0;
    }
    if(((_S302 + _S307) > 1.0f))
    {
        var _S312 : f32 = dpych_1.primal_0.z + dpych_1.primal_0.w;
        var _S313 : DiffPair_float_0 = DiffPair_float_0( 0.23726457357406616f / _S312, (0.0f - 0.23726457357406616f * (dpych_1.differential_0.z + dpych_1.differential_0.w)) / (_S312 * _S312) );
        var _S314 : DiffPair_float_0 = DiffPair_float_0( max_c_7, s_diff_max_c_1 );
        var _S315 : DiffPair_float_0 = _d_min_1(_S313, _S314);
        max_c_7 = _S315.primal_0;
        s_diff_max_c_1 = _S315.differential_0;
    }
    var _S316 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( vec4<f32>(dpych_1.primal_0.x, max_c_7, dpych_1.primal_0.z, dpych_1.primal_0.w), vec4<f32>(dpych_1.differential_0.x, s_diff_max_c_1, dpych_1.differential_0.z, dpych_1.differential_0.w) );
    return _S316;
}

fn yrg_to_lms_0( yrg_2 : vec3<f32>) -> vec3<f32>
{
    var _S317 : f32 = yrg_2.y;
    var _S318 : f32 = yrg_2.z;
    var lms_1 : vec3<f32> = (((vec3<f32>(_S317, _S318, 1.0f - _S317 - _S318)) * (mat3x3<f32>(0.94999998807907104f, 0.37999999523162842f, 0.0f, 0.05000000074505806f, 0.62000000476837158f, 0.029999999329447746f, 0.0f, 0.0f, 0.97000002861022949f))));
    var denominator_0 : f32 = 0.68990272283554077f * lms_1.x + 0.34832188487052917f * lms_1.y;
    var a_5 : f32;
    if(denominator_0 != 0.0f)
    {
        a_5 = yrg_2.x / denominator_0;
    }
    else
    {
        a_5 = 0.0f;
    }
    return lms_1 * vec3<f32>(a_5);
}

struct Tuple_26
{
     value0_34 : vec3<f32>,
};

fn s_apply_yrg_to_lms_0( _S319 : vec3<f32>) -> Tuple_26
{
    var _S320 : f32 = _S319.y;
    var _S321 : f32 = _S319.z;
    var _S322 : Tuple_18 = s_apply_mul_0(mat3x3<f32>(0.94999998807907104f, 0.37999999523162842f, 0.0f, 0.05000000074505806f, 0.62000000476837158f, 0.029999999329447746f, 0.0f, 0.0f, 0.97000002861022949f), vec3<f32>(_S320, _S321, 1.0f - _S320 - _S321));
    var denominator_1 : f32 = 0.68990272283554077f * _S322.value0_20.x + 0.34832188487052917f * _S322.value0_20.y;
    var a_6 : f32;
    if(denominator_1 != 0.0f)
    {
        a_6 = _S319.x / denominator_1;
    }
    else
    {
        a_6 = 0.0f;
    }
    var _S323 : Tuple_26;
    _S323.value0_34 = _S322.value0_20 * vec3<f32>(a_6);
    return _S323;
}

struct s_paramCtx_s_bwdCallableCtx_yrg_to_lms_0
{
     value0_35 : vec3<f32>,
};

struct s_bwdCallableCtx_yrg_to_lms_0
{
     value0_36 : s_paramCtx_s_bwdCallableCtx_yrg_to_lms_0,
};

fn s_bwdProp_yrg_to_lms_0( _S324 : s_bwdCallableCtx_yrg_to_lms_0,  _S325 : ptr<function, vec3<f32>>,  _S326 : vec3<f32>)
{
    var _S327 : f32 = _S324.value0_36.value0_35.y;
    var _S328 : f32 = _S324.value0_36.value0_35.z;
    var _S329 : vec3<f32> = vec3<f32>(_S327, _S328, 1.0f - _S327 - _S328);
    var _S330 : Tuple_18 = s_apply_mul_0(mat3x3<f32>(0.94999998807907104f, 0.37999999523162842f, 0.0f, 0.05000000074505806f, 0.62000000476837158f, 0.029999999329447746f, 0.0f, 0.0f, 0.97000002861022949f), _S329);
    var _S331 : s_bwdCallableCtx_mul_0;
    _S331._S114 = mat3x3<f32>(0.94999998807907104f, 0.37999999523162842f, 0.0f, 0.05000000074505806f, 0.62000000476837158f, 0.029999999329447746f, 0.0f, 0.0f, 0.97000002861022949f);
    _S331._S115 = _S329;
    var _S332 : s_bwdCallableCtx_mul_0 = _S331;
    var denominator_2 : f32 = 0.68990272283554077f * _S330.value0_20.x + 0.34832188487052917f * _S330.value0_20.y;
    var _S333 : bool = denominator_2 != 0.0f;
    var a_7 : f32;
    var _S334 : f32;
    var _S335 : f32;
    if(_S333)
    {
        var _S336 : f32 = _S324.value0_36.value0_35.x;
        var _S337 : f32 = denominator_2 * denominator_2;
        a_7 = _S336 / denominator_2;
        _S334 = _S337;
        _S335 = _S336;
    }
    else
    {
        a_7 = 0.0f;
        _S334 = 0.0f;
        _S335 = 0.0f;
    }
    var _S338 : vec3<f32> = vec3<f32>(0.0f);
    var _S339 : vec3<f32> = _S330.value0_20 * _S326;
    var _S340 : f32 = _S339[i32(0)] + _S339[i32(1)] + _S339[i32(2)];
    var _S341 : vec3<f32> = vec3<f32>(a_7) * _S326;
    var _S342 : vec3<f32>;
    if(_S333)
    {
        var _S343 : f32 = _S340 / _S334;
        var _S344 : vec3<f32> = vec3<f32>(denominator_2 * _S343, 0.0f, 0.0f);
        a_7 = _S335 * - _S343;
        _S342 = _S344;
    }
    else
    {
        a_7 = 0.0f;
        _S342 = _S338;
    }
    var _S345 : vec3<f32> = _S341 + vec3<f32>(0.68990272283554077f * a_7, 0.34832188487052917f * a_7, 0.0f);
    var _S346 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S347 : vec3<f32> = _S338;
    s_bwdProp_mul_0(_S332, &(_S346), &(_S347), _S345);
    var _S348 : f32 = - _S347[i32(2)];
    (*_S325) = _S342 + vec3<f32>(0.0f, _S347[i32(0)] + _S348, _S347[i32(1)] + _S348);
    return;
}

fn s_fwd_yrg_to_lms_0( dpyrg_1 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S349 : f32 = dpyrg_1.primal_0.y;
    var _S350 : f32 = dpyrg_1.differential_0.y;
    var _S351 : f32 = dpyrg_1.primal_0.z;
    var _S352 : f32 = dpyrg_1.differential_0.z;
    var _S353 : vec3<f32> = vec3<f32>(_S349, _S351, 1.0f - _S349 - _S351);
    var _S354 : vec3<f32> = (((_S353) * (mat3x3<f32>(0.94999998807907104f, 0.37999999523162842f, 0.0f, 0.05000000074505806f, 0.62000000476837158f, 0.029999999329447746f, 0.0f, 0.0f, 0.97000002861022949f))));
    var _S355 : vec3<f32> = (((_S353) * (mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f)))) + (((vec3<f32>(_S350, _S352, 0.0f - _S350 - _S352)) * (mat3x3<f32>(0.94999998807907104f, 0.37999999523162842f, 0.0f, 0.05000000074505806f, 0.62000000476837158f, 0.029999999329447746f, 0.0f, 0.0f, 0.97000002861022949f))));
    var denominator_3 : f32 = 0.68990272283554077f * _S354.x + 0.34832188487052917f * _S354.y;
    var s_diff_denominator_0 : f32 = _S355.x * 0.68990272283554077f + _S355.y * 0.34832188487052917f;
    var a_8 : f32;
    var s_diff_a_1 : f32;
    if(denominator_3 != 0.0f)
    {
        var _S356 : f32 = dpyrg_1.primal_0.x;
        var _S357 : f32 = (dpyrg_1.differential_0.x * denominator_3 - _S356 * s_diff_denominator_0) / (denominator_3 * denominator_3);
        a_8 = _S356 / denominator_3;
        s_diff_a_1 = _S357;
    }
    else
    {
        a_8 = 0.0f;
        s_diff_a_1 = 0.0f;
    }
    var _S358 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S354 * vec3<f32>(a_8), _S355 * vec3<f32>(a_8) + vec3<f32>(s_diff_a_1) * _S354 );
    return _S358;
}

fn bounded_pow_0( x_22 : f32,  y_14 : f32) -> f32
{
    if((x_22 <= 0.0f))
    {
        return 0.0f;
    }
    return exp2(min(y_14 * log2(x_22), 32.0f));
}

struct Tuple_27
{
     value0_37 : f32,
};

fn s_apply_bounded_pow_0( _S359 : f32,  _S360 : f32) -> Tuple_27
{
    var _S361 : f32;
    if(!(_S359 <= 0.0f))
    {
        _S361 = s_apply_exp2_0(s_apply_min_0(_S360 * s_apply_log2_0(_S359).value0_9, 32.0f).value0_10).value0_8;
    }
    else
    {
        _S361 = 0.0f;
    }
    var _S362 : Tuple_27;
    _S362.value0_37 = _S361;
    return _S362;
}

struct s_paramCtx_s_bwdCallableCtx_bounded_pow_0
{
     value0_38 : f32,
     value1_0 : f32,
};

struct s_bwdCallableCtx_bounded_pow_0
{
     value0_39 : s_paramCtx_s_bwdCallableCtx_bounded_pow_0,
};

fn s_bwdProp_bounded_pow_0( _S363 : s_bwdCallableCtx_bounded_pow_0,  _S364 : ptr<function, f32>,  _S365 : ptr<function, f32>,  _S366 : f32)
{
    var _S367 : bool = !((_S363.value0_39.value0_38) <= 0.0f);
    var _S368 : s_bwdCallableCtx_exp2_0;
    var _S369 : s_bwdCallableCtx_min_0;
    var _S370 : f32;
    var _S371 : s_bwdCallableCtx_log2_0;
    if(_S367)
    {
        var _S372 : Tuple_9 = s_apply_log2_0(_S363.value0_39.value0_38);
        var _S373 : s_bwdCallableCtx_log2_0;
        _S373._S56 = _S363.value0_39.value0_38;
        var _S374 : f32 = _S363.value0_39.value1_0 * _S372.value0_9;
        var _S375 : Tuple_10 = s_apply_min_0(_S374, 32.0f);
        var _S376 : s_bwdCallableCtx_min_0;
        _S376._S60 = _S374;
        _S376._S61 = 32.0f;
        var _S377 : s_bwdCallableCtx_exp2_0;
        _S377._S52 = _S375.value0_10;
        _S368 = _S377;
        _S369 = _S376;
        _S370 = _S372.value0_9;
        _S371 = _S373;
    }
    else
    {
        _S368._S52 = 0.0f;
        _S369._S60 = 0.0f;
        _S369._S61 = 0.0f;
        _S370 = 0.0f;
        _S371._S56 = 0.0f;
    }
    var _S378 : f32;
    if(_S367)
    {
        var _S379 : f32 = 0.0f;
        s_bwdProp_exp2_0(_S368, &(_S379), _S366);
        var _S380 : f32 = 0.0f;
        var _S381 : f32 = 0.0f;
        s_bwdProp_min_0(_S369, &(_S380), &(_S381), _S379);
        var _S382 : f32 = _S363.value0_39.value1_0 * _S380;
        var _S383 : f32 = _S370 * _S380;
        var _S384 : f32 = 0.0f;
        s_bwdProp_log2_0(_S371, &(_S384), _S382);
        _S370 = _S384;
        _S378 = _S383;
    }
    else
    {
        _S370 = 0.0f;
        _S378 = 0.0f;
    }
    (*_S364) = _S370;
    (*_S365) = _S378;
    return;
}

fn s_fwd_bounded_pow_0( dpx_34 : DiffPair_float_0,  dpy_14 : DiffPair_float_0) -> DiffPair_float_0
{
    if(((dpx_34.primal_0) <= 0.0f))
    {
        var _S385 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
        return _S385;
    }
    var _S386 : DiffPair_float_0 = DiffPair_float_0( dpx_34.primal_0, dpx_34.differential_0 );
    var _S387 : DiffPair_float_0 = _d_log2_1(_S386);
    var _S388 : DiffPair_float_0 = DiffPair_float_0( dpy_14.primal_0 * _S387.primal_0, dpy_14.differential_0 * _S387.primal_0 + _S387.differential_0 * dpy_14.primal_0 );
    var _S389 : DiffPair_float_0 = DiffPair_float_0( 32.0f, 0.0f );
    var _S390 : DiffPair_float_0 = _d_min_1(_S388, _S389);
    var _S391 : DiffPair_float_0 = DiffPair_float_0( _S390.primal_0, _S390.differential_0 );
    var _S392 : DiffPair_float_0 = _d_exp2_1(_S391);
    var _S393 : DiffPair_float_0 = DiffPair_float_0( _S392.primal_0, _S392.differential_0 );
    return _S393;
}

fn bounded_pow_1( x_23 : vec3<f32>,  y_15 : vec3<f32>) -> vec3<f32>
{
    return vec3<f32>(bounded_pow_0(x_23.x, y_15.x), bounded_pow_0(x_23.y, y_15.y), bounded_pow_0(x_23.z, y_15.z));
}

struct Tuple_28
{
     value0_40 : vec3<f32>,
};

fn s_apply_bounded_pow_1( _S394 : vec3<f32>,  _S395 : vec3<f32>) -> Tuple_28
{
    var _S396 : Tuple_28;
    _S396.value0_40 = vec3<f32>(s_apply_bounded_pow_0(_S394.x, _S395.x).value0_37, s_apply_bounded_pow_0(_S394.y, _S395.y).value0_37, s_apply_bounded_pow_0(_S394.z, _S395.z).value0_37);
    return _S396;
}

struct s_paramCtx_s_bwdCallableCtx_bounded_pow_1
{
     value0_41 : vec3<f32>,
     value1_1 : vec3<f32>,
};

struct s_bwdCallableCtx_bounded_pow_1
{
     value0_42 : s_paramCtx_s_bwdCallableCtx_bounded_pow_1,
};

fn s_bwdProp_bounded_pow_1( _S397 : s_bwdCallableCtx_bounded_pow_1,  _S398 : ptr<function, vec3<f32>>,  _S399 : ptr<function, vec3<f32>>,  _S400 : vec3<f32>)
{
    var _S401 : f32 = _S397.value0_42.value1_1.x;
    var _S402 : s_paramCtx_s_bwdCallableCtx_bounded_pow_0;
    _S402.value0_38 = _S397.value0_42.value0_41.x;
    _S402.value1_0 = _S401;
    var _S403 : s_bwdCallableCtx_bounded_pow_0 = s_bwdCallableCtx_bounded_pow_0( _S402 );
    var _S404 : f32 = _S397.value0_42.value1_1.y;
    var _S405 : s_paramCtx_s_bwdCallableCtx_bounded_pow_0;
    _S405.value0_38 = _S397.value0_42.value0_41.y;
    _S405.value1_0 = _S404;
    var _S406 : s_bwdCallableCtx_bounded_pow_0 = s_bwdCallableCtx_bounded_pow_0( _S405 );
    var _S407 : f32 = _S397.value0_42.value1_1.z;
    var _S408 : s_paramCtx_s_bwdCallableCtx_bounded_pow_0;
    _S408.value0_38 = _S397.value0_42.value0_41.z;
    _S408.value1_0 = _S407;
    var _S409 : s_bwdCallableCtx_bounded_pow_0 = s_bwdCallableCtx_bounded_pow_0( _S408 );
    var _S410 : f32 = 0.0f;
    var _S411 : f32 = 0.0f;
    s_bwdProp_bounded_pow_0(_S409, &(_S410), &(_S411), _S400[i32(2)]);
    var _S412 : f32 = 0.0f;
    var _S413 : f32 = 0.0f;
    s_bwdProp_bounded_pow_0(_S406, &(_S412), &(_S413), _S400[i32(1)]);
    var _S414 : f32 = 0.0f;
    var _S415 : f32 = 0.0f;
    s_bwdProp_bounded_pow_0(_S403, &(_S414), &(_S415), _S400[i32(0)]);
    var _S416 : vec3<f32> = vec3<f32>(_S415, _S413, _S411);
    (*_S398) = vec3<f32>(_S414, _S412, _S410);
    (*_S399) = _S416;
    return;
}

fn s_fwd_bounded_pow_1( dpx_35 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpy_15 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S417 : DiffPair_float_0 = DiffPair_float_0( dpx_35.primal_0.x, dpx_35.differential_0.x );
    var _S418 : DiffPair_float_0 = DiffPair_float_0( dpy_15.primal_0.x, dpy_15.differential_0.x );
    var _S419 : DiffPair_float_0 = s_fwd_bounded_pow_0(_S417, _S418);
    var _S420 : DiffPair_float_0 = DiffPair_float_0( dpx_35.primal_0.y, dpx_35.differential_0.y );
    var _S421 : DiffPair_float_0 = DiffPair_float_0( dpy_15.primal_0.y, dpy_15.differential_0.y );
    var _S422 : DiffPair_float_0 = s_fwd_bounded_pow_0(_S420, _S421);
    var _S423 : DiffPair_float_0 = DiffPair_float_0( dpx_35.primal_0.z, dpx_35.differential_0.z );
    var _S424 : DiffPair_float_0 = DiffPair_float_0( dpy_15.primal_0.z, dpy_15.differential_0.z );
    var _S425 : DiffPair_float_0 = s_fwd_bounded_pow_0(_S423, _S424);
    var _S426 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S419.primal_0, _S422.primal_0, _S425.primal_0), vec3<f32>(_S419.differential_0, _S422.differential_0, _S425.differential_0) );
    return _S426;
}

fn pow0_1( x_24 : vec3<f32>,  y_16 : vec3<f32>) -> vec3<f32>
{
    return vec3<f32>(pow0_0(x_24.x, y_16.x), pow0_0(x_24.y, y_16.y), pow0_0(x_24.z, y_16.z));
}

struct Tuple_29
{
     value0_43 : vec3<f32>,
};

fn s_apply_pow0_1( _S427 : vec3<f32>,  _S428 : vec3<f32>) -> Tuple_29
{
    var _S429 : Tuple_29;
    _S429.value0_43 = vec3<f32>(s_apply_pow0_0(_S427.x, _S428.x).value0_18, s_apply_pow0_0(_S427.y, _S428.y).value0_18, s_apply_pow0_0(_S427.z, _S428.z).value0_18);
    return _S429;
}

struct s_paramCtx_s_bwdCallableCtx_pow0_0
{
     value0_44 : vec3<f32>,
     value1_2 : vec3<f32>,
};

struct s_bwdCallableCtx_pow0_1
{
     value0_45 : s_paramCtx_s_bwdCallableCtx_pow0_0,
};

fn s_bwdProp_pow0_1( _S430 : s_bwdCallableCtx_pow0_1,  _S431 : ptr<function, vec3<f32>>,  _S432 : ptr<function, vec3<f32>>,  _S433 : vec3<f32>)
{
    var _S434 : f32 = _S430.value0_45.value1_2.x;
    var _S435 : s_bwdCallableCtx_pow0_0;
    _S435._S104 = _S430.value0_45.value0_44.x;
    _S435._S105 = _S434;
    var _S436 : f32 = _S430.value0_45.value1_2.y;
    var _S437 : s_bwdCallableCtx_pow0_0;
    _S437._S104 = _S430.value0_45.value0_44.y;
    _S437._S105 = _S436;
    var _S438 : f32 = _S430.value0_45.value1_2.z;
    var _S439 : s_bwdCallableCtx_pow0_0;
    _S439._S104 = _S430.value0_45.value0_44.z;
    _S439._S105 = _S438;
    var _S440 : f32 = 0.0f;
    var _S441 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S439, &(_S440), &(_S441), _S433[i32(2)]);
    var _S442 : f32 = 0.0f;
    var _S443 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S437, &(_S442), &(_S443), _S433[i32(1)]);
    var _S444 : f32 = 0.0f;
    var _S445 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S435, &(_S444), &(_S445), _S433[i32(0)]);
    var _S446 : vec3<f32> = vec3<f32>(_S445, _S443, _S441);
    (*_S431) = vec3<f32>(_S444, _S442, _S440);
    (*_S432) = _S446;
    return;
}

fn s_fwd_pow0_0( dpx_36 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpy_16 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S447 : DiffPair_float_0 = DiffPair_float_0( dpx_36.primal_0.x, dpx_36.differential_0.x );
    var _S448 : DiffPair_float_0 = DiffPair_float_0( dpy_16.primal_0.x, dpy_16.differential_0.x );
    var _S449 : DiffPair_float_0 = pow0_forward_0(_S447, _S448);
    var _S450 : DiffPair_float_0 = DiffPair_float_0( dpx_36.primal_0.y, dpx_36.differential_0.y );
    var _S451 : DiffPair_float_0 = DiffPair_float_0( dpy_16.primal_0.y, dpy_16.differential_0.y );
    var _S452 : DiffPair_float_0 = pow0_forward_0(_S450, _S451);
    var _S453 : DiffPair_float_0 = DiffPair_float_0( dpx_36.primal_0.z, dpx_36.differential_0.z );
    var _S454 : DiffPair_float_0 = DiffPair_float_0( dpy_16.primal_0.z, dpy_16.differential_0.z );
    var _S455 : DiffPair_float_0 = pow0_forward_0(_S453, _S454);
    var _S456 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S449.primal_0, _S452.primal_0, _S455.primal_0), vec3<f32>(_S449.differential_0, _S452.differential_0, _S455.differential_0) );
    return _S456;
}

fn xyz_to_jzazbz_0( xyz_0 : vec3<f32>) -> vec3<f32>
{
    var _S457 : f32 = xyz_0.x;
    var _S458 : f32 = xyz_0.z;
    var t_1 : vec3<f32> = pow0_1(max((((vec3<f32>(1.1499999761581421f * _S457 - 0.14999997615814209f * _S458, 0.6600000262260437f * xyz_0.y - -0.3399999737739563f * _S457, _S458)) * (mat3x3<f32>(0.41478970646858215f, 0.57999902963638306f, 0.014647999778389931f, -0.2015099972486496f, 1.120648980140686f, 0.053100798279047012f, -0.016600800678133965f, 0.26480001211166382f, 0.66847991943359375f)))) / vec3<f32>(10000.0f), vec3<f32>(0.0f)), vec3<f32>(0.1593017578125f));
    var iab_0 : vec3<f32> = (((pow0_1((vec3<f32>(0.8359375f) + vec3<f32>(18.8515625f) * t_1) / (vec3<f32>(1.0f) + vec3<f32>(18.6875f) * t_1), vec3<f32>(134.03437805175781f))) * (mat3x3<f32>(0.5f, 0.5f, 0.0f, 3.5239999294281006f, -4.0667080879211426f, 0.54270797967910767f, 0.19907599687576294f, 1.0967990159988403f, -1.2958749532699585f))));
    var _S459 : f32 = iab_0.x;
    return vec3<f32>(max(0.43999999761581421f * _S459 / (1.0f + -0.56000000238418579f * _S459) - 1.62954996718589484e-11f, 0.0f), iab_0.y, iab_0.z);
}

struct Tuple_30
{
     value0_46 : vec3<f32>,
};

fn s_apply_xyz_to_jzazbz_0( _S460 : vec3<f32>) -> Tuple_30
{
    var _S461 : f32 = _S460.x;
    var _S462 : f32 = _S460.z;
    var _S463 : Tuple_29 = s_apply_pow0_1(s_apply_max_2(s_apply_mul_0(mat3x3<f32>(0.41478970646858215f, 0.57999902963638306f, 0.014647999778389931f, -0.2015099972486496f, 1.120648980140686f, 0.053100798279047012f, -0.016600800678133965f, 0.26480001211166382f, 0.66847991943359375f), vec3<f32>(1.1499999761581421f * _S461 - 0.14999997615814209f * _S462, 0.6600000262260437f * _S460.y - -0.3399999737739563f * _S461, _S462)).value0_20 / vec3<f32>(10000.0f), vec3<f32>(0.0f)).value0_21, vec3<f32>(0.1593017578125f));
    var _S464 : Tuple_18 = s_apply_mul_0(mat3x3<f32>(0.5f, 0.5f, 0.0f, 3.5239999294281006f, -4.0667080879211426f, 0.54270797967910767f, 0.19907599687576294f, 1.0967990159988403f, -1.2958749532699585f), s_apply_pow0_1((vec3<f32>(0.8359375f) + vec3<f32>(18.8515625f) * _S463.value0_43) / (vec3<f32>(1.0f) + vec3<f32>(18.6875f) * _S463.value0_43), vec3<f32>(134.03437805175781f)).value0_43);
    var _S465 : f32 = _S464.value0_20.x;
    var _S466 : Tuple_30;
    _S466.value0_46 = vec3<f32>(s_apply_max_1(0.43999999761581421f * _S465 / (1.0f + -0.56000000238418579f * _S465) - 1.62954996718589484e-11f, 0.0f).value0_19, _S464.value0_20.y, _S464.value0_20.z);
    return _S466;
}

struct s_paramCtx_s_bwdCallableCtx_xyz_to_jzazbz_0
{
     value0_47 : vec3<f32>,
};

struct s_bwdCallableCtx_xyz_to_jzazbz_0
{
     value0_48 : s_paramCtx_s_bwdCallableCtx_xyz_to_jzazbz_0,
};

fn s_bwdProp_xyz_to_jzazbz_0( _S467 : s_bwdCallableCtx_xyz_to_jzazbz_0,  _S468 : ptr<function, vec3<f32>>,  _S469 : vec3<f32>)
{
    var _S470 : f32 = _S467.value0_48.value0_47.x;
    var _S471 : f32 = _S467.value0_48.value0_47.z;
    var xyz_prime_0 : vec3<f32> = vec3<f32>(1.1499999761581421f * _S470 - 0.14999997615814209f * _S471, 0.6600000262260437f * _S467.value0_48.value0_47.y - -0.3399999737739563f * _S470, _S471);
    var _S472 : Tuple_18 = s_apply_mul_0(mat3x3<f32>(0.41478970646858215f, 0.57999902963638306f, 0.014647999778389931f, -0.2015099972486496f, 1.120648980140686f, 0.053100798279047012f, -0.016600800678133965f, 0.26480001211166382f, 0.66847991943359375f), xyz_prime_0);
    var _S473 : s_bwdCallableCtx_mul_0;
    _S473._S114 = mat3x3<f32>(0.41478970646858215f, 0.57999902963638306f, 0.014647999778389931f, -0.2015099972486496f, 1.120648980140686f, 0.053100798279047012f, -0.016600800678133965f, 0.26480001211166382f, 0.66847991943359375f);
    _S473._S115 = xyz_prime_0;
    var _S474 : vec3<f32> = _S472.value0_20 / vec3<f32>(10000.0f);
    var _S475 : vec3<f32> = vec3<f32>(0.0f);
    var _S476 : Tuple_19 = s_apply_max_2(_S474, _S475);
    var _S477 : s_bwdCallableCtx_max_2;
    _S477._S122 = _S474;
    _S477._S123 = _S475;
    var _S478 : vec3<f32> = vec3<f32>(0.1593017578125f);
    var _S479 : Tuple_29 = s_apply_pow0_1(_S476.value0_21, _S478);
    var _S480 : s_paramCtx_s_bwdCallableCtx_pow0_0;
    _S480.value0_44 = _S476.value0_21;
    _S480.value1_2 = _S478;
    var _S481 : s_bwdCallableCtx_pow0_1 = s_bwdCallableCtx_pow0_1( _S480 );
    var _S482 : vec3<f32> = vec3<f32>(18.8515625f);
    var _S483 : vec3<f32> = vec3<f32>(0.8359375f) + _S482 * _S479.value0_43;
    var _S484 : vec3<f32> = vec3<f32>(18.6875f);
    var _S485 : vec3<f32> = vec3<f32>(1.0f) + _S484 * _S479.value0_43;
    var _S486 : vec3<f32> = _S483 / _S485;
    var _S487 : vec3<f32> = _S485 * _S485;
    var _S488 : vec3<f32> = vec3<f32>(134.03437805175781f);
    var _S489 : Tuple_29 = s_apply_pow0_1(_S486, _S488);
    var _S490 : s_paramCtx_s_bwdCallableCtx_pow0_0;
    _S490.value0_44 = _S486;
    _S490.value1_2 = _S488;
    var _S491 : s_bwdCallableCtx_pow0_1 = s_bwdCallableCtx_pow0_1( _S490 );
    var _S492 : Tuple_18 = s_apply_mul_0(mat3x3<f32>(0.5f, 0.5f, 0.0f, 3.5239999294281006f, -4.0667080879211426f, 0.54270797967910767f, 0.19907599687576294f, 1.0967990159988403f, -1.2958749532699585f), _S489.value0_43);
    var _S493 : s_bwdCallableCtx_mul_0;
    _S493._S114 = mat3x3<f32>(0.5f, 0.5f, 0.0f, 3.5239999294281006f, -4.0667080879211426f, 0.54270797967910767f, 0.19907599687576294f, 1.0967990159988403f, -1.2958749532699585f);
    _S493._S115 = _S489.value0_43;
    var _S494 : f32 = _S492.value0_20.x;
    var _S495 : f32 = 0.43999999761581421f * _S494;
    var _S496 : f32 = 1.0f + -0.56000000238418579f * _S494;
    var _S497 : f32 = _S496 * _S496;
    var _S498 : s_bwdCallableCtx_max_1;
    _S498._S109 = _S495 / _S496 - 1.62954996718589484e-11f;
    _S498._S110 = 0.0f;
    var _S499 : f32 = 0.0f;
    var _S500 : f32 = 0.0f;
    s_bwdProp_max_1(_S498, &(_S499), &(_S500), _S469[i32(0)]);
    var _S501 : f32 = _S499 / _S497;
    var _S502 : vec3<f32> = vec3<f32>(-0.56000000238418579f * (_S495 * - _S501) + 0.43999999761581421f * (_S496 * _S501), _S469[i32(1)], _S469[i32(2)]);
    var _S503 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S504 : mat3x3<f32> = _S503;
    var _S505 : vec3<f32> = vec3<f32>(0.0f);
    var _S506 : vec3<f32> = _S505;
    s_bwdProp_mul_0(_S493, &(_S504), &(_S506), _S502);
    var _S507 : vec3<f32> = _S505;
    var _S508 : vec3<f32> = _S505;
    s_bwdProp_pow0_1(_S491, &(_S507), &(_S508), _S506);
    var _S509 : vec3<f32> = _S507 / _S487;
    var _S510 : vec3<f32> = _S484 * (_S483 * (vec3<f32>(0) - _S509)) + _S482 * (_S485 * _S509);
    var _S511 : vec3<f32> = _S505;
    var _S512 : vec3<f32> = _S505;
    s_bwdProp_pow0_1(_S481, &(_S511), &(_S512), _S510);
    var _S513 : vec3<f32> = _S505;
    var _S514 : vec3<f32> = _S505;
    s_bwdProp_max_2(_S477, &(_S513), &(_S514), _S511);
    var _S515 : vec3<f32> = vec3<f32>(9.9999997473787516e-05f) * _S513;
    var _S516 : mat3x3<f32> = _S503;
    var _S517 : vec3<f32> = _S505;
    s_bwdProp_mul_0(_S473, &(_S516), &(_S517), _S515);
    (*_S468) = vec3<f32>(-0.3399999737739563f * - _S517[i32(1)] + 1.1499999761581421f * _S517[i32(0)], 0.6600000262260437f * _S517[i32(1)], _S517[i32(2)] + 0.14999997615814209f * - _S517[i32(0)]);
    return;
}

fn s_fwd_xyz_to_jzazbz_0( dpxyz_0 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S518 : f32 = dpxyz_0.primal_0.x;
    var _S519 : f32 = dpxyz_0.differential_0.x;
    var _S520 : f32 = dpxyz_0.primal_0.z;
    var _S521 : f32 = dpxyz_0.differential_0.z;
    var xyz_prime_1 : vec3<f32> = vec3<f32>(1.1499999761581421f * _S518 - 0.14999997615814209f * _S520, 0.6600000262260437f * dpxyz_0.primal_0.y - -0.3399999737739563f * _S518, _S520);
    var _S522 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S523 : vec3<f32> = vec3<f32>(0.0f);
    var _S524 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( (((xyz_prime_1) * (mat3x3<f32>(0.41478970646858215f, 0.57999902963638306f, 0.014647999778389931f, -0.2015099972486496f, 1.120648980140686f, 0.053100798279047012f, -0.016600800678133965f, 0.26480001211166382f, 0.66847991943359375f)))) / vec3<f32>(10000.0f), ((((xyz_prime_1) * (_S522))) + (((vec3<f32>(_S519 * 1.1499999761581421f - _S521 * 0.14999997615814209f, dpxyz_0.differential_0.y * 0.6600000262260437f - _S519 * -0.3399999737739563f, _S521)) * (mat3x3<f32>(0.41478970646858215f, 0.57999902963638306f, 0.014647999778389931f, -0.2015099972486496f, 1.120648980140686f, 0.053100798279047012f, -0.016600800678133965f, 0.26480001211166382f, 0.66847991943359375f))))) * vec3<f32>(9.9999997473787516e-05f) );
    var _S525 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.0f), _S523 );
    var _S526 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_max_vector_3(_S524, _S525);
    var _S527 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S526.primal_0, _S526.differential_0 );
    var _S528 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.1593017578125f), _S523 );
    var _S529 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_pow0_0(_S527, _S528);
    var _S530 : vec3<f32> = vec3<f32>(18.8515625f);
    var _S531 : vec3<f32> = vec3<f32>(0.8359375f) + _S530 * _S529.primal_0;
    var _S532 : vec3<f32> = vec3<f32>(18.6875f);
    var _S533 : vec3<f32> = vec3<f32>(1.0f) + _S532 * _S529.primal_0;
    var _S534 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S531 / _S533, (_S529.differential_0 * _S530 * _S533 - _S531 * (_S529.differential_0 * _S532)) / (_S533 * _S533) );
    var _S535 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(134.03437805175781f), _S523 );
    var _S536 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_pow0_0(_S534, _S535);
    var _S537 : vec3<f32> = (((_S536.primal_0) * (mat3x3<f32>(0.5f, 0.5f, 0.0f, 3.5239999294281006f, -4.0667080879211426f, 0.54270797967910767f, 0.19907599687576294f, 1.0967990159988403f, -1.2958749532699585f))));
    var _S538 : vec3<f32> = (((_S536.primal_0) * (_S522))) + (((_S536.differential_0) * (mat3x3<f32>(0.5f, 0.5f, 0.0f, 3.5239999294281006f, -4.0667080879211426f, 0.54270797967910767f, 0.19907599687576294f, 1.0967990159988403f, -1.2958749532699585f))));
    var _S539 : f32 = _S537.x;
    var _S540 : f32 = _S538.x;
    var _S541 : f32 = 0.43999999761581421f * _S539;
    var _S542 : f32 = 1.0f + -0.56000000238418579f * _S539;
    var _S543 : DiffPair_float_0 = DiffPair_float_0( _S541 / _S542 - 1.62954996718589484e-11f, (_S540 * 0.43999999761581421f * _S542 - _S541 * (_S540 * -0.56000000238418579f)) / (_S542 * _S542) );
    var _S544 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S545 : DiffPair_float_0 = _d_max_1(_S543, _S544);
    var _S546 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S545.primal_0, _S537.y, _S537.z), vec3<f32>(_S545.differential_0, _S538.y, _S538.z) );
    return _S546;
}

fn lookup_gamut_0( hue_0 : f32) -> f32
{
    var x_25 : f32 = 512.0f * (hue_0 + 3.1415927410125732f) / 6.2831854820251465f;
    var x_prev_0 : f32 = floor(x_25);
    var i_0 : u32 = ((bitcast<u32>((i32(x_prev_0)))) & (u32(511)));
    var y_17 : f32 = gamut_lut_0[i_0];
    return y_17 + (x_25 - x_prev_0) * (gamut_lut_0[((i_0 + u32(1)) & (u32(511)))] - y_17);
}

fn s_fwd_lookup_gamut_0( dphue_0 : DiffPair_float_0) -> DiffPair_float_0
{
    var x_26 : f32 = 512.0f * (dphue_0.primal_0 + 3.1415927410125732f) / 6.2831854820251465f;
    var x_prev_1 : f32 = floor(x_26);
    var i_1 : u32 = ((bitcast<u32>((i32(x_prev_1)))) & (u32(511)));
    var y_18 : f32 = gamut_lut_0[i_1];
    var _S547 : f32 = gamut_lut_0[((i_1 + u32(1)) & (u32(511)))] - y_18;
    var _S548 : DiffPair_float_0 = DiffPair_float_0( y_18 + (x_26 - x_prev_1) * _S547, dphue_0.differential_0 * 512.0f * 0.15915493667125702f * _S547 );
    return _S548;
}

struct s_minCtx_s_bwdCallableCtx_lookup_gamut_0
{
     value0_49 : f32,
     value1_3 : f32,
};

struct Tuple_31
{
     value0_50 : f32,
     value1_4 : s_minCtx_s_bwdCallableCtx_lookup_gamut_0,
};

fn s_apply_lookup_gamut_0( _S549 : f32) -> Tuple_31
{
    var _S550 : s_minCtx_s_bwdCallableCtx_lookup_gamut_0;
    _S550.value0_49 = 0.0f;
    _S550.value1_3 = 0.0f;
    var x_27 : f32 = 512.0f * (_S549 + 3.1415927410125732f) / 6.2831854820251465f;
    var x_prev_2 : f32 = floor(x_27);
    var i_2 : u32 = ((bitcast<u32>((i32(x_prev_2)))) & (u32(511)));
    var j_0 : u32 = ((i_2 + u32(1)) & (u32(511)));
    var y_19 : f32 = gamut_lut_0[i_2];
    _S550.value0_49 = y_19;
    var _S551 : f32 = x_27 - x_prev_2;
    var _S552 : f32 = gamut_lut_0[j_0];
    _S550.value1_3 = _S552;
    var _S553 : Tuple_31;
    _S553.value0_50 = y_19 + _S551 * (_S552 - y_19);
    _S553.value1_4 = _S550;
    return _S553;
}

struct s_bwdCallableCtx_lookup_gamut_0
{
     value1_5 : s_minCtx_s_bwdCallableCtx_lookup_gamut_0,
};

fn s_bwdProp_lookup_gamut_0( _S554 : s_bwdCallableCtx_lookup_gamut_0,  _S555 : ptr<function, f32>,  _S556 : f32)
{
    (*_S555) = 512.0f * (0.15915493667125702f * ((_S554.value1_5.value1_3 - _S554.value1_5.value0_49) * _S556));
    return;
}

fn soft_clip_0( x_28 : f32,  soft_0 : f32,  hard_0 : f32) -> f32
{
    if((x_28 <= soft_0))
    {
        return x_28;
    }
    var norm_1 : f32 = hard_0 - soft_0;
    if((norm_1 <= 0.0f))
    {
        return soft_0;
    }
    return soft_0 + (1.0f - exp(- (x_28 - soft_0) / norm_1)) * norm_1;
}

struct Tuple_32
{
     value0_51 : f32,
};

fn s_apply_soft_clip_0( _S557 : f32,  _S558 : f32,  _S559 : f32) -> Tuple_32
{
    var _S560 : bool = !(_S557 <= _S558);
    var _S561 : f32;
    if(_S560)
    {
        var norm_2 : f32 = _S559 - _S558;
        var _runFlag_0 : bool;
        if((norm_2 <= 0.0f))
        {
            _runFlag_0 = false;
            _S561 = _S558;
        }
        else
        {
            _runFlag_0 = _S560;
            _S561 = _S557;
        }
        if(_runFlag_0)
        {
            _S561 = _S558 + (1.0f - s_apply_exp_0(- (_S557 - _S558) / norm_2).value0_11) * norm_2;
        }
    }
    else
    {
        _S561 = _S557;
    }
    var _S562 : Tuple_32;
    _S562.value0_51 = _S561;
    return _S562;
}

struct s_paramCtx_s_bwdCallableCtx_soft_clip_0
{
     value0_52 : f32,
     value1_6 : f32,
     value2_0 : f32,
};

struct s_bwdCallableCtx_soft_clip_0
{
     value0_53 : s_paramCtx_s_bwdCallableCtx_soft_clip_0,
};

fn s_bwdProp_soft_clip_0( _S563 : s_bwdCallableCtx_soft_clip_0,  _S564 : ptr<function, f32>,  _S565 : ptr<function, f32>,  _S566 : ptr<function, f32>,  _S567 : f32)
{
    var _S568 : bool = ((_S563.value0_53.value0_52) <= (_S563.value0_53.value1_6));
    var _S569 : bool = !_S568;
    var _runFlag_1 : bool;
    var _S570 : bool;
    var _S571 : f32;
    var _S572 : f32;
    var _S573 : f32;
    var _S574 : f32;
    var _S575 : s_bwdCallableCtx_exp_0;
    if(_S569)
    {
        var norm_3 : f32 = _S563.value0_53.value2_0 - _S563.value0_53.value1_6;
        var _S576 : bool = (norm_3 <= 0.0f);
        if(_S576)
        {
            _runFlag_1 = false;
        }
        else
        {
            _runFlag_1 = _S569;
        }
        if(_runFlag_1)
        {
            var _S577 : f32 = - (_S563.value0_53.value0_52 - _S563.value0_53.value1_6);
            var _S578 : f32 = _S577 / norm_3;
            var _S579 : f32 = norm_3 * norm_3;
            var _S580 : Tuple_11 = s_apply_exp_0(_S578);
            var _S581 : s_bwdCallableCtx_exp_0;
            _S581._S68 = _S578;
            _S571 = 1.0f - _S580.value0_11;
            _S575 = _S581;
            _S572 = _S579;
            _S573 = _S577;
        }
        else
        {
            _S571 = 0.0f;
            _S575._S68 = 0.0f;
            _S572 = 0.0f;
            _S573 = 0.0f;
        }
        var _S582 : f32 = _S572;
        var _S583 : f32 = _S573;
        _S572 = norm_3;
        _S573 = _S582;
        _S574 = _S583;
        _S570 = _S576;
    }
    else
    {
        _runFlag_1 = false;
        _S571 = 0.0f;
        _S572 = 0.0f;
        _S575._S68 = 0.0f;
        _S573 = 0.0f;
        _S574 = 0.0f;
        _S570 = false;
    }
    if(_S569)
    {
        if(_runFlag_1)
        {
            var _S584 : f32 = _S571 * _S567;
            var _S585 : f32 = - (_S572 * _S567);
            var _S586 : f32 = 0.0f;
            s_bwdProp_exp_0(_S575, &(_S586), _S585);
            var _S587 : f32 = _S586 / _S573;
            var _S588 : f32 = - (_S572 * _S587);
            var _S589 : f32 = _S567 + - _S588;
            var _S590 : f32 = _S584 + _S574 * - _S587;
            _S571 = 0.0f;
            _S572 = _S589;
            _S573 = _S590;
            _S574 = _S588;
        }
        else
        {
            _S571 = _S567;
            _S572 = 0.0f;
            _S573 = 0.0f;
            _S574 = 0.0f;
        }
        if(_S570)
        {
            _S571 = _S571 + _S572;
            _S572 = 0.0f;
        }
        else
        {
            var _S591 : f32 = _S571;
            _S571 = _S572;
            _S572 = _S591;
        }
        var _S592 : f32 = - _S573 + _S571;
        var _S593 : f32 = _S573;
        _S571 = _S572;
        _S572 = _S574;
        _S573 = _S592;
        _S574 = _S593;
    }
    else
    {
        _S571 = _S567;
        _S572 = 0.0f;
        _S573 = 0.0f;
        _S574 = 0.0f;
    }
    if(_S568)
    {
        _S571 = _S571 + _S572;
    }
    else
    {
        _S571 = _S572;
    }
    (*_S564) = _S571;
    (*_S565) = _S573;
    (*_S566) = _S574;
    return;
}

fn s_fwd_soft_clip_0( dpx_37 : DiffPair_float_0,  dpsoft_0 : DiffPair_float_0,  dphard_0 : DiffPair_float_0) -> DiffPair_float_0
{
    if(((dpx_37.primal_0) <= (dpsoft_0.primal_0)))
    {
        var _S594 : DiffPair_float_0 = DiffPair_float_0( dpx_37.primal_0, dpx_37.differential_0 );
        return _S594;
    }
    var norm_4 : f32 = dphard_0.primal_0 - dpsoft_0.primal_0;
    var s_diff_norm_0 : f32 = dphard_0.differential_0 - dpsoft_0.differential_0;
    if((norm_4 <= 0.0f))
    {
        var _S595 : DiffPair_float_0 = DiffPair_float_0( dpsoft_0.primal_0, dpsoft_0.differential_0 );
        return _S595;
    }
    var _S596 : f32 = - (dpx_37.primal_0 - dpsoft_0.primal_0);
    var _S597 : DiffPair_float_0 = DiffPair_float_0( _S596 / norm_4, (- (dpx_37.differential_0 - dpsoft_0.differential_0) * norm_4 - _S596 * s_diff_norm_0) / (norm_4 * norm_4) );
    var _S598 : DiffPair_float_0 = _d_exp_1(_S597);
    var _S599 : f32 = 1.0f - _S598.primal_0;
    var _S600 : DiffPair_float_0 = DiffPair_float_0( dpsoft_0.primal_0 + _S599 * norm_4, dpsoft_0.differential_0 + ((0.0f - _S598.differential_0) * norm_4 + s_diff_norm_0 * _S599) );
    return _S600;
}

fn jz_to_iz_0( jz_0 : f32) -> f32
{
    var iz_0 : f32 = jz_0 + 1.62954996718589484e-11f;
    return max(iz_0 / (0.43999999761581421f - -0.56000000238418579f * iz_0), 0.0f);
}

struct Tuple_33
{
     value0_54 : f32,
};

fn s_apply_jz_to_iz_0( _S601 : f32) -> Tuple_33
{
    var iz_1 : f32 = _S601 + 1.62954996718589484e-11f;
    var _S602 : Tuple_33;
    _S602.value0_54 = s_apply_max_1(iz_1 / (0.43999999761581421f - -0.56000000238418579f * iz_1), 0.0f).value0_19;
    return _S602;
}

struct s_paramCtx_s_bwdCallableCtx_jz_to_iz_0
{
     value0_55 : f32,
};

struct s_bwdCallableCtx_jz_to_iz_0
{
     value0_56 : s_paramCtx_s_bwdCallableCtx_jz_to_iz_0,
};

fn s_bwdProp_jz_to_iz_0( _S603 : s_bwdCallableCtx_jz_to_iz_0,  _S604 : ptr<function, f32>,  _S605 : f32)
{
    var iz_2 : f32 = _S603.value0_56.value0_55 + 1.62954996718589484e-11f;
    var _S606 : f32 = 0.43999999761581421f - -0.56000000238418579f * iz_2;
    var _S607 : f32 = _S606 * _S606;
    var _S608 : s_bwdCallableCtx_max_1;
    _S608._S109 = iz_2 / _S606;
    _S608._S110 = 0.0f;
    var _S609 : f32 = 0.0f;
    var _S610 : f32 = 0.0f;
    s_bwdProp_max_1(_S608, &(_S609), &(_S610), _S605);
    var _S611 : f32 = _S609 / _S607;
    (*_S604) = _S606 * _S611 + -0.56000000238418579f * - (iz_2 * - _S611);
    return;
}

fn s_fwd_jz_to_iz_0( dpjz_0 : DiffPair_float_0) -> DiffPair_float_0
{
    var iz_3 : f32 = dpjz_0.primal_0 + 1.62954996718589484e-11f;
    var _S612 : f32 = 0.43999999761581421f - -0.56000000238418579f * iz_3;
    var _S613 : DiffPair_float_0 = DiffPair_float_0( iz_3 / _S612, (dpjz_0.differential_0 * _S612 - iz_3 * (0.0f - dpjz_0.differential_0 * -0.56000000238418579f)) / (_S612 * _S612) );
    var _S614 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S615 : DiffPair_float_0 = _d_max_1(_S613, _S614);
    var _S616 : DiffPair_float_0 = DiffPair_float_0( _S615.primal_0, _S615.differential_0 );
    return _S616;
}

fn jzazbz_to_xyz_0( jab_0 : vec3<f32>) -> vec3<f32>
{
    var _S617 : vec3<f32> = vec3<f32>(0.0f);
    var t_2 : vec3<f32> = pow0_1(max((((vec3<f32>(jz_to_iz_0(jab_0.x), jab_0.y, jab_0.z)) * (mat3x3<f32>(1.0f, 0.13860504329204559f, 0.05804731696844101f, 1.0f, -0.13860504329204559f, -0.05804731696844101f, 1.0f, -0.096019245684146881f, -0.81189191341400146f)))), _S617), vec3<f32>(0.0074607725255191326f));
    var xyz_prime_2 : vec3<f32> = (((vec3<f32>(10000.0f) * pow0_1(max((vec3<f32>(0.8359375f) - t_2) / (vec3<f32>(18.6875f) * t_2 - vec3<f32>(18.8515625f)), _S617), vec3<f32>(6.2773947715759277f))) * (mat3x3<f32>(1.9242264032363892f, -1.004792332649231f, 0.037651404738426208f, 0.35031676292419434f, 0.72648119926452637f, -0.065384425222873688f, -0.090982809662818909f, -0.31272828578948975f, 1.5227665901184082f))));
    var _S618 : f32 = xyz_prime_2.z;
    var x_29 : f32 = (xyz_prime_2.x + 0.14999997615814209f * _S618) / 1.1499999761581421f;
    return vec3<f32>(x_29, (xyz_prime_2.y + -0.3399999737739563f * x_29) / 0.6600000262260437f, _S618);
}

struct Tuple_34
{
     value0_57 : vec3<f32>,
};

fn s_apply_jzazbz_to_xyz_0( _S619 : vec3<f32>) -> Tuple_34
{
    var _S620 : vec3<f32> = vec3<f32>(0.0f);
    var _S621 : Tuple_29 = s_apply_pow0_1(s_apply_max_2(s_apply_mul_0(mat3x3<f32>(1.0f, 0.13860504329204559f, 0.05804731696844101f, 1.0f, -0.13860504329204559f, -0.05804731696844101f, 1.0f, -0.096019245684146881f, -0.81189191341400146f), vec3<f32>(s_apply_jz_to_iz_0(_S619.x).value0_54, _S619.y, _S619.z)).value0_20, _S620).value0_21, vec3<f32>(0.0074607725255191326f));
    var _S622 : Tuple_18 = s_apply_mul_0(mat3x3<f32>(1.9242264032363892f, -1.004792332649231f, 0.037651404738426208f, 0.35031676292419434f, 0.72648119926452637f, -0.065384425222873688f, -0.090982809662818909f, -0.31272828578948975f, 1.5227665901184082f), vec3<f32>(10000.0f) * s_apply_pow0_1(s_apply_max_2((vec3<f32>(0.8359375f) - _S621.value0_43) / (vec3<f32>(18.6875f) * _S621.value0_43 - vec3<f32>(18.8515625f)), _S620).value0_21, vec3<f32>(6.2773947715759277f)).value0_43);
    var _S623 : f32 = _S622.value0_20.z;
    var x_30 : f32 = (_S622.value0_20.x + 0.14999997615814209f * _S623) / 1.1499999761581421f;
    var _S624 : Tuple_34;
    _S624.value0_57 = vec3<f32>(x_30, (_S622.value0_20.y + -0.3399999737739563f * x_30) / 0.6600000262260437f, _S623);
    return _S624;
}

struct s_paramCtx_s_bwdCallableCtx_jzazbz_to_xyz_0
{
     value0_58 : vec3<f32>,
};

struct s_bwdCallableCtx_jzazbz_to_xyz_0
{
     value0_59 : s_paramCtx_s_bwdCallableCtx_jzazbz_to_xyz_0,
};

fn s_bwdProp_jzazbz_to_xyz_0( _S625 : s_bwdCallableCtx_jzazbz_to_xyz_0,  _S626 : ptr<function, vec3<f32>>,  _S627 : vec3<f32>)
{
    var _S628 : f32 = _S625.value0_59.value0_58.x;
    var _S629 : Tuple_33 = s_apply_jz_to_iz_0(_S628);
    var _S630 : s_paramCtx_s_bwdCallableCtx_jz_to_iz_0;
    _S630.value0_55 = _S628;
    var _S631 : s_bwdCallableCtx_jz_to_iz_0 = s_bwdCallableCtx_jz_to_iz_0( _S630 );
    var _S632 : vec3<f32> = vec3<f32>(_S629.value0_54, _S625.value0_59.value0_58.y, _S625.value0_59.value0_58.z);
    var _S633 : Tuple_18 = s_apply_mul_0(mat3x3<f32>(1.0f, 0.13860504329204559f, 0.05804731696844101f, 1.0f, -0.13860504329204559f, -0.05804731696844101f, 1.0f, -0.096019245684146881f, -0.81189191341400146f), _S632);
    var _S634 : s_bwdCallableCtx_mul_0;
    _S634._S114 = mat3x3<f32>(1.0f, 0.13860504329204559f, 0.05804731696844101f, 1.0f, -0.13860504329204559f, -0.05804731696844101f, 1.0f, -0.096019245684146881f, -0.81189191341400146f);
    _S634._S115 = _S632;
    var _S635 : vec3<f32> = vec3<f32>(0.0f);
    var _S636 : Tuple_19 = s_apply_max_2(_S633.value0_20, _S635);
    var _S637 : s_bwdCallableCtx_max_2;
    _S637._S122 = _S633.value0_20;
    _S637._S123 = _S635;
    var _S638 : vec3<f32> = vec3<f32>(0.0074607725255191326f);
    var _S639 : Tuple_29 = s_apply_pow0_1(_S636.value0_21, _S638);
    var _S640 : s_paramCtx_s_bwdCallableCtx_pow0_0;
    _S640.value0_44 = _S636.value0_21;
    _S640.value1_2 = _S638;
    var _S641 : s_bwdCallableCtx_pow0_1 = s_bwdCallableCtx_pow0_1( _S640 );
    var _S642 : vec3<f32> = vec3<f32>(0.8359375f) - _S639.value0_43;
    var _S643 : vec3<f32> = vec3<f32>(18.6875f);
    var _S644 : vec3<f32> = _S643 * _S639.value0_43 - vec3<f32>(18.8515625f);
    var _S645 : vec3<f32> = _S642 / _S644;
    var _S646 : vec3<f32> = _S644 * _S644;
    var _S647 : Tuple_19 = s_apply_max_2(_S645, _S635);
    var _S648 : s_bwdCallableCtx_max_2;
    _S648._S122 = _S645;
    _S648._S123 = _S635;
    var _S649 : vec3<f32> = vec3<f32>(6.2773947715759277f);
    var _S650 : Tuple_29 = s_apply_pow0_1(_S647.value0_21, _S649);
    var _S651 : s_paramCtx_s_bwdCallableCtx_pow0_0;
    _S651.value0_44 = _S647.value0_21;
    _S651.value1_2 = _S649;
    var _S652 : s_bwdCallableCtx_pow0_1 = s_bwdCallableCtx_pow0_1( _S651 );
    var _S653 : vec3<f32> = vec3<f32>(10000.0f);
    var lms_2 : vec3<f32> = _S653 * _S650.value0_43;
    var _S654 : s_bwdCallableCtx_mul_0;
    _S654._S114 = mat3x3<f32>(1.9242264032363892f, -1.004792332649231f, 0.037651404738426208f, 0.35031676292419434f, 0.72648119926452637f, -0.065384425222873688f, -0.090982809662818909f, -0.31272828578948975f, 1.5227665901184082f);
    _S654._S115 = lms_2;
    var _S655 : f32 = 1.5151515007019043f * _S627[i32(1)];
    var _S656 : f32 = 0.86956524848937988f * (_S627[i32(0)] + -0.3399999737739563f * _S655);
    var _S657 : vec3<f32> = vec3<f32>(_S656, _S655, _S627[i32(2)] + 0.14999997615814209f * _S656);
    var _S658 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S659 : mat3x3<f32> = _S658;
    var _S660 : vec3<f32> = vec3<f32>(0.0f);
    var _S661 : vec3<f32> = _S660;
    s_bwdProp_mul_0(_S654, &(_S659), &(_S661), _S657);
    var _S662 : vec3<f32> = _S653 * _S661;
    var _S663 : vec3<f32> = _S660;
    var _S664 : vec3<f32> = _S660;
    s_bwdProp_pow0_1(_S652, &(_S663), &(_S664), _S662);
    var _S665 : vec3<f32> = _S660;
    var _S666 : vec3<f32> = _S660;
    s_bwdProp_max_2(_S648, &(_S665), &(_S666), _S663);
    var _S667 : vec3<f32> = _S665 / _S646;
    var _S668 : vec3<f32> = _S643 * (_S642 * (vec3<f32>(0) - _S667)) + (vec3<f32>(0) - _S644 * _S667);
    var _S669 : vec3<f32> = _S660;
    var _S670 : vec3<f32> = _S660;
    s_bwdProp_pow0_1(_S641, &(_S669), &(_S670), _S668);
    var _S671 : vec3<f32> = _S660;
    var _S672 : vec3<f32> = _S660;
    s_bwdProp_max_2(_S637, &(_S671), &(_S672), _S669);
    var _S673 : mat3x3<f32> = _S658;
    var _S674 : vec3<f32> = _S660;
    s_bwdProp_mul_0(_S634, &(_S673), &(_S674), _S671);
    var _S675 : f32 = 0.0f;
    s_bwdProp_jz_to_iz_0(_S631, &(_S675), _S674[i32(0)]);
    (*_S626) = vec3<f32>(_S675, _S674[i32(1)], _S674[i32(2)]);
    return;
}

fn s_fwd_jzazbz_to_xyz_0( dpjab_0 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S676 : DiffPair_float_0 = DiffPair_float_0( dpjab_0.primal_0.x, dpjab_0.differential_0.x );
    var _S677 : DiffPair_float_0 = s_fwd_jz_to_iz_0(_S676);
    var _S678 : vec3<f32> = vec3<f32>(_S677.primal_0, dpjab_0.primal_0.y, dpjab_0.primal_0.z);
    var _S679 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S680 : vec3<f32> = vec3<f32>(0.0f);
    var _S681 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( (((_S678) * (mat3x3<f32>(1.0f, 0.13860504329204559f, 0.05804731696844101f, 1.0f, -0.13860504329204559f, -0.05804731696844101f, 1.0f, -0.096019245684146881f, -0.81189191341400146f)))), (((_S678) * (_S679))) + (((vec3<f32>(_S677.differential_0, dpjab_0.differential_0.y, dpjab_0.differential_0.z)) * (mat3x3<f32>(1.0f, 0.13860504329204559f, 0.05804731696844101f, 1.0f, -0.13860504329204559f, -0.05804731696844101f, 1.0f, -0.096019245684146881f, -0.81189191341400146f)))) );
    var _S682 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.0f), _S680 );
    var _S683 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_max_vector_3(_S681, _S682);
    var _S684 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S683.primal_0, _S683.differential_0 );
    var _S685 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.0074607725255191326f), _S680 );
    var _S686 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_pow0_0(_S684, _S685);
    var _S687 : vec3<f32> = vec3<f32>(0.8359375f) - _S686.primal_0;
    var _S688 : vec3<f32> = vec3<f32>(18.6875f);
    var _S689 : vec3<f32> = _S688 * _S686.primal_0 - vec3<f32>(18.8515625f);
    var _S690 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S687 / _S689, ((_S680 - _S686.differential_0) * _S689 - _S687 * (_S686.differential_0 * _S688)) / (_S689 * _S689) );
    var _S691 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_max_vector_3(_S690, _S682);
    var _S692 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S691.primal_0, _S691.differential_0 );
    var _S693 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(6.2773947715759277f), _S680 );
    var _S694 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_pow0_0(_S692, _S693);
    var _S695 : vec3<f32> = vec3<f32>(10000.0f);
    var lms_3 : vec3<f32> = _S695 * _S694.primal_0;
    var _S696 : vec3<f32> = (((lms_3) * (mat3x3<f32>(1.9242264032363892f, -1.004792332649231f, 0.037651404738426208f, 0.35031676292419434f, 0.72648119926452637f, -0.065384425222873688f, -0.090982809662818909f, -0.31272828578948975f, 1.5227665901184082f))));
    var _S697 : vec3<f32> = (((lms_3) * (_S679))) + (((_S694.differential_0 * _S695) * (mat3x3<f32>(1.9242264032363892f, -1.004792332649231f, 0.037651404738426208f, 0.35031676292419434f, 0.72648119926452637f, -0.065384425222873688f, -0.090982809662818909f, -0.31272828578948975f, 1.5227665901184082f))));
    var _S698 : f32 = _S696.z;
    var _S699 : f32 = _S697.z;
    var x_31 : f32 = (_S696.x + 0.14999997615814209f * _S698) / 1.1499999761581421f;
    var s_diff_x_0 : f32 = (_S697.x + _S699 * 0.14999997615814209f) * 0.86956524848937988f;
    var _S700 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(x_31, (_S696.y + -0.3399999737739563f * x_31) / 0.6600000262260437f, _S698), vec3<f32>(s_diff_x_0, (_S697.y + s_diff_x_0 * -0.3399999737739563f) * 1.5151515007019043f, _S699) );
    return _S700;
}

struct s_paramCtx_s_bwdCallableCtx_jzazbz_saturation_0
{
     value0_60 : vec3<f32>,
     value1_7 : vec3<f32>,
     value2_1 : ColorBalanceParams_0,
     value3_0 : Grading_0,
};

struct s_minCtx_s_bwdCallableCtx_jzazbz_saturation_0
{
     value0_61 : f32,
     value1_8 : s_minCtx_s_bwdCallableCtx_lookup_gamut_0,
};

struct s_bwdCallableCtx_jzazbz_saturation_0
{
     value0_62 : s_paramCtx_s_bwdCallableCtx_jzazbz_saturation_0,
     value1_9 : s_minCtx_s_bwdCallableCtx_jzazbz_saturation_0,
};

fn s_bwdProp_jzazbz_saturation_0( _S701 : s_bwdCallableCtx_jzazbz_saturation_0,  _S702 : ptr<function, vec3<f32>>,  _S703 : ptr<function, vec3<f32>>,  _S704 : ptr<function, Grading_0>,  _S705 : vec3<f32>)
{
    var _S706 : s_paramCtx_s_bwdCallableCtx_soft_clip_0 = s_paramCtx_s_bwdCallableCtx_soft_clip_0( 0.0f, 0.0f, 0.0f );
    var _S707 : Tuple_30 = s_apply_xyz_to_jzazbz_0(_S701.value0_62.value0_60);
    var _S708 : s_paramCtx_s_bwdCallableCtx_xyz_to_jzazbz_0;
    _S708.value0_47 = _S701.value0_62.value0_60;
    var _S709 : s_bwdCallableCtx_xyz_to_jzazbz_0 = s_bwdCallableCtx_xyz_to_jzazbz_0( _S708 );
    var _S710 : f32 = _S707.value0_46.x;
    var _S711 : vec2<f32> = _S707.value0_46.yz;
    var _S712 : Tuple_12 = s_apply_length_0(_S711);
    var _S713 : s_bwdCallableCtx_length_0;
    _S713._S72 = _S711;
    var _S714 : s_bwdCallableCtx_length_0 = _S713;
    var _S715 : f32 = _S707.value0_46.z;
    var _S716 : f32 = _S707.value0_46.y;
    var _S717 : Tuple_7 = s_apply_atan2_0(_S715, _S716);
    var _S718 : s_bwdCallableCtx_atan2_0;
    _S718._S44 = _S715;
    _S718._S45 = _S716;
    var _S719 : s_bwdCallableCtx_atan2_0 = _S718;
    var _S720 : Tuple_7 = s_apply_atan2_0(_S712.value0_12, _S710);
    var _S721 : s_bwdCallableCtx_atan2_0;
    _S721._S44 = _S712.value0_12;
    _S721._S45 = _S710;
    var _S722 : s_bwdCallableCtx_atan2_0 = _S721;
    var _S723 : Tuple_6 = s_apply_sin_0(_S720.value0_7);
    var _S724 : s_bwdCallableCtx_sin_0;
    _S724._S40 = _S720.value0_7;
    var _S725 : s_bwdCallableCtx_sin_0 = _S724;
    var _S726 : Tuple_5 = s_apply_cos_0(_S720.value0_7);
    var _S727 : s_bwdCallableCtx_cos_0;
    _S727._S36 = _S720.value0_7;
    var _S728 : s_bwdCallableCtx_cos_0 = _S727;
    var _S729 : f32 = 1.0f + _S701.value0_62.value3_0.brilliance_global_1;
    var _S730 : vec3<f32> = _S701.value0_62.value2_1.brilliance_0.xyz;
    var _S731 : Tuple_14 = s_apply_dot_1(_S701.value0_62.value1_7, _S730);
    var _S732 : s_bwdCallableCtx_dot_1;
    _S732._S98 = _S701.value0_62.value1_7;
    _S732._S99 = _S730;
    var _S733 : s_bwdCallableCtx_dot_1 = _S732;
    var brilliance_1 : f32 = _S729 + _S731.value0_16;
    var _S734 : vec3<f32> = _S701.value0_62.value2_1.saturation_0.xyz;
    var _S735 : Tuple_14 = s_apply_dot_1(_S701.value0_62.value1_7, _S734);
    var _S736 : s_bwdCallableCtx_dot_1;
    _S736._S98 = _S701.value0_62.value1_7;
    _S736._S99 = _S734;
    var _S737 : s_bwdCallableCtx_dot_1 = _S736;
    var saturation_1 : f32 = _S701.value0_62.value3_0.saturation_global_1 + _S735.value0_16;
    var s_0 : f32 = _S710 * _S726.value0_5 + _S712.value0_12 * _S723.value0_6;
    var _S738 : f32 = _S720.value0_7 * saturation_1;
    var _S739 : f32 = - _S720.value0_7;
    var _S740 : Tuple_17 = s_apply_max_1(_S738, _S739);
    var _S741 : s_bwdCallableCtx_max_1;
    _S741._S109 = _S738;
    _S741._S110 = _S739;
    var _S742 : s_bwdCallableCtx_max_1 = _S741;
    var _S743 : f32 = 1.5707963705062866f - _S720.value0_7;
    var _S744 : Tuple_10 = s_apply_min_0(_S740.value0_19, _S743);
    var _S745 : s_bwdCallableCtx_min_0;
    _S745._S60 = _S740.value0_19;
    _S745._S61 = _S743;
    var _S746 : s_bwdCallableCtx_min_0 = _S745;
    var o_0 : f32 = s_0 * _S744.value0_10;
    var _S747 : f32 = s_0 * brilliance_1;
    var _S748 : Tuple_17 = s_apply_max_1(_S747, 0.0f);
    var _S749 : s_bwdCallableCtx_max_1;
    _S749._S109 = _S747;
    _S749._S110 = 0.0f;
    var _S750 : s_bwdCallableCtx_max_1 = _S749;
    var _S751 : vec2<f32> = vec2<f32>(_S748.value0_19 * _S726.value0_5 - o_0 * _S723.value0_6, _S748.value0_19 * _S723.value0_6 + o_0 * _S726.value0_5);
    var _S752 : vec2<f32> = vec2<f32>(0.0f);
    var _S753 : Tuple_4 = s_apply_max_0(_S751, _S752);
    var _S754 : s_bwdCallableCtx_max_0;
    _S754._S28 = _S751;
    _S754._S29 = _S752;
    var _S755 : s_bwdCallableCtx_max_0 = _S754;
    var _S756 : s_bwdCallableCtx_lookup_gamut_0 = s_bwdCallableCtx_lookup_gamut_0( _S701.value1_9.value1_8 );
    var _S757 : f32 = _S753.value0_4.x;
    var _S758 : bool = (_S757 > 0.0f);
    var sat_0 : f32;
    var _S759 : f32;
    var _S760 : f32;
    var _S761 : s_bwdCallableCtx_soft_clip_0;
    if(_S758)
    {
        var _S762 : f32 = _S753.value0_4.y;
        var _S763 : f32 = _S762 / _S757;
        var _S764 : f32 = _S757 * _S757;
        var _S765 : f32 = 0.80000001192092896f * _S701.value1_9.value0_61;
        var _S766 : Tuple_32 = s_apply_soft_clip_0(_S763, _S765, _S701.value1_9.value0_61);
        var _S767 : s_paramCtx_s_bwdCallableCtx_soft_clip_0;
        _S767.value0_52 = _S763;
        _S767.value1_6 = _S765;
        _S767.value2_0 = _S701.value1_9.value0_61;
        sat_0 = _S766.value0_51;
        _S761.value0_53 = _S767;
        _S759 = _S764;
        _S760 = _S762;
    }
    else
    {
        sat_0 = _S701.value1_9.value0_61;
        _S761.value0_53 = _S706;
        _S759 = 0.0f;
        _S760 = 0.0f;
    }
    var _S768 : bool = (sat_0 > 0.0f);
    var max_j_at_sat_0 : f32;
    var _S769 : f32;
    var _S770 : f32;
    if(_S768)
    {
        var _S771 : f32 = _S753.value0_4.y;
        var _S772 : f32 = sat_0 * sat_0;
        max_j_at_sat_0 = _S771 / sat_0;
        _S769 = _S772;
        _S770 = _S771;
    }
    else
    {
        max_j_at_sat_0 = _S757;
        _S769 = 0.0f;
        _S770 = 0.0f;
    }
    var _S773 : f32 = (_S757 + max_j_at_sat_0) / 2.0f;
    var _S774 : f32 = (_S753.value0_4.y + _S757 * sat_0) / 2.0f;
    var _S775 : Tuple_5 = s_apply_cos_0(_S717.value0_7);
    var _S776 : s_bwdCallableCtx_cos_0;
    _S776._S36 = _S717.value0_7;
    var _S777 : s_bwdCallableCtx_cos_0 = _S776;
    var _S778 : Tuple_6 = s_apply_sin_0(_S717.value0_7);
    var _S779 : s_bwdCallableCtx_sin_0;
    _S779._S40 = _S717.value0_7;
    var _S780 : s_bwdCallableCtx_sin_0 = _S779;
    var _S781 : Tuple_33 = s_apply_jz_to_iz_0(_S773);
    var _S782 : s_paramCtx_s_bwdCallableCtx_jz_to_iz_0;
    _S782.value0_55 = _S773;
    var _S783 : s_bwdCallableCtx_jz_to_iz_0 = s_bwdCallableCtx_jz_to_iz_0( _S782 );
    var _S784 : vec3<f32> = vec3<f32>(_S781.value0_54, _S774 * _S775.value0_5, _S774 * _S778.value0_6);
    var _S785 : Tuple_18 = s_apply_mul_0(mat3x3<f32>(1.0f, 0.13860504329204559f, 0.05804731696844101f, 1.0f, -0.13860504329204559f, -0.05804731696844101f, 1.0f, -0.096019245684146881f, -0.81189191341400146f), _S784);
    var _S786 : s_bwdCallableCtx_mul_0;
    _S786._S114 = mat3x3<f32>(1.0f, 0.13860504329204559f, 0.05804731696844101f, 1.0f, -0.13860504329204559f, -0.05804731696844101f, 1.0f, -0.096019245684146881f, -0.81189191341400146f);
    _S786._S115 = _S784;
    var _S787 : s_bwdCallableCtx_mul_0 = _S786;
    var _S788 : bool = ((_S785.value0_20.x) < 0.0f);
    var max_c_8 : f32;
    var _S789 : f32;
    var _S790 : f32;
    var _S791 : s_bwdCallableCtx_min_0;
    if(_S788)
    {
        var _S792 : f32 = - _S781.value0_54;
        var _S793 : f32 = 0.13860504329204559f * _S775.value0_5 + 0.05804731696844101f * _S778.value0_6;
        var _S794 : f32 = _S792 / _S793;
        var _S795 : f32 = _S793 * _S793;
        var _S796 : Tuple_10 = s_apply_min_0(_S794, _S774);
        var _S797 : s_bwdCallableCtx_min_0;
        _S797._S60 = _S794;
        _S797._S61 = _S774;
        max_c_8 = _S796.value0_10;
        _S791 = _S797;
        max_j_at_sat_0 = _S795;
        _S789 = _S792;
        _S790 = _S793;
    }
    else
    {
        max_c_8 = _S774;
        _S791._S60 = 0.0f;
        _S791._S61 = 0.0f;
        max_j_at_sat_0 = 0.0f;
        _S789 = 0.0f;
        _S790 = 0.0f;
    }
    var _S798 : bool = ((_S785.value0_20.y) < 0.0f);
    var _S799 : f32;
    var _S800 : f32;
    var _S801 : f32;
    var _S802 : s_bwdCallableCtx_min_0;
    if(_S798)
    {
        var _S803 : f32 = - _S781.value0_54;
        var _S804 : f32 = -0.13860504329204559f * _S775.value0_5 - 0.05804731696844101f * _S778.value0_6;
        var _S805 : f32 = _S803 / _S804;
        var _S806 : f32 = _S804 * _S804;
        var _S807 : Tuple_10 = s_apply_min_0(_S805, max_c_8);
        var _S808 : s_bwdCallableCtx_min_0;
        _S808._S60 = _S805;
        _S808._S61 = max_c_8;
        max_c_8 = _S807.value0_10;
        _S802 = _S808;
        _S799 = _S806;
        _S800 = _S803;
        _S801 = _S804;
    }
    else
    {
        _S802._S60 = 0.0f;
        _S802._S61 = 0.0f;
        _S799 = 0.0f;
        _S800 = 0.0f;
        _S801 = 0.0f;
    }
    var _S809 : bool = ((_S785.value0_20.z) < 0.0f);
    var _S810 : f32;
    var _S811 : f32;
    var _S812 : f32;
    var _S813 : s_bwdCallableCtx_min_0;
    if(_S809)
    {
        var _S814 : f32 = - _S781.value0_54;
        var _S815 : f32 = -0.096019245684146881f * _S775.value0_5 - 0.81189191341400146f * _S778.value0_6;
        var _S816 : f32 = _S814 / _S815;
        var _S817 : f32 = _S815 * _S815;
        var _S818 : Tuple_10 = s_apply_min_0(_S816, max_c_8);
        var _S819 : s_bwdCallableCtx_min_0;
        _S819._S60 = _S816;
        _S819._S61 = max_c_8;
        max_c_8 = _S818.value0_10;
        _S813 = _S819;
        _S810 = _S817;
        _S811 = _S814;
        _S812 = _S815;
    }
    else
    {
        _S813._S60 = 0.0f;
        _S813._S61 = 0.0f;
        _S810 = 0.0f;
        _S811 = 0.0f;
        _S812 = 0.0f;
    }
    var _S820 : s_paramCtx_s_bwdCallableCtx_jzazbz_to_xyz_0;
    _S820.value0_58 = vec3<f32>(_S773, max_c_8 * _S775.value0_5, max_c_8 * _S778.value0_6);
    var _S821 : s_bwdCallableCtx_jzazbz_to_xyz_0 = s_bwdCallableCtx_jzazbz_to_xyz_0( _S820 );
    var _S822 : Grading_0 = Grading_x24_syn_dzero_0();
    var _S823 : vec3<f32> = vec3<f32>(0.0f);
    var _S824 : vec3<f32> = _S823;
    s_bwdProp_jzazbz_to_xyz_0(_S821, &(_S824), _S705);
    var _S825 : vec3<f32> = _S824;
    var _S826 : f32 = max_c_8 * _S824[i32(2)];
    var _S827 : f32 = max_c_8 * _S824[i32(1)];
    var _S828 : f32 = _S778.value0_6 * _S824[i32(2)] + _S775.value0_5 * _S824[i32(1)];
    if(_S809)
    {
        var _S829 : f32 = 0.0f;
        var _S830 : f32 = 0.0f;
        s_bwdProp_min_0(_S813, &(_S829), &(_S830), _S828);
        var _S831 : f32 = _S829 / _S810;
        var _S832 : f32 = _S811 * - _S831;
        var _S833 : f32 = - (_S812 * _S831);
        var _S834 : f32 = -0.096019245684146881f * _S832 + _S827;
        var _S835 : f32 = 0.81189191341400146f * - _S832 + _S826;
        max_c_8 = _S830;
        _S810 = _S834;
        _S811 = _S835;
        _S812 = _S833;
    }
    else
    {
        max_c_8 = _S828;
        _S810 = _S827;
        _S811 = _S826;
        _S812 = 0.0f;
    }
    if(_S798)
    {
        var _S836 : f32 = 0.0f;
        var _S837 : f32 = 0.0f;
        s_bwdProp_min_0(_S802, &(_S836), &(_S837), max_c_8);
        var _S838 : f32 = _S836 / _S799;
        var _S839 : f32 = _S800 * - _S838;
        var _S840 : f32 = -0.13860504329204559f * _S839 + _S810;
        var _S841 : f32 = 0.05804731696844101f * - _S839 + _S811;
        var _S842 : f32 = - (_S801 * _S838) + _S812;
        max_c_8 = _S837;
        _S799 = _S840;
        _S800 = _S841;
        _S801 = _S842;
    }
    else
    {
        _S799 = _S810;
        _S800 = _S811;
        _S801 = _S812;
    }
    if(_S788)
    {
        var _S843 : f32 = 0.0f;
        var _S844 : f32 = 0.0f;
        s_bwdProp_min_0(_S791, &(_S843), &(_S844), max_c_8);
        var _S845 : f32 = _S843 / max_j_at_sat_0;
        var _S846 : f32 = _S789 * - _S845;
        var _S847 : f32 = 0.13860504329204559f * _S846 + _S799;
        var _S848 : f32 = 0.05804731696844101f * _S846 + _S800;
        max_j_at_sat_0 = - (_S790 * _S845) + _S801;
        max_c_8 = _S848;
        _S789 = _S847;
        _S790 = _S844;
    }
    else
    {
        var _S849 : f32 = max_c_8;
        max_j_at_sat_0 = _S801;
        max_c_8 = _S800;
        _S789 = _S799;
        _S790 = _S849;
    }
    var _S850 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S851 : vec3<f32> = _S823;
    s_bwdProp_mul_0(_S787, &(_S850), &(_S851), _S823);
    var _S852 : f32 = _S774 * _S851[i32(2)];
    var _S853 : f32 = _S778.value0_6 * _S851[i32(2)];
    var _S854 : f32 = _S774 * _S851[i32(1)];
    var _S855 : f32 = _S775.value0_5 * _S851[i32(1)];
    var _S856 : f32 = _S851[i32(0)] + max_j_at_sat_0;
    var _S857 : f32 = 0.0f;
    s_bwdProp_jz_to_iz_0(_S783, &(_S857), _S856);
    var _S858 : f32 = _S852 + max_c_8;
    var _S859 : f32 = 0.0f;
    s_bwdProp_sin_0(_S780, &(_S859), _S858);
    var _S860 : f32 = _S854 + _S789;
    var _S861 : f32 = 0.0f;
    s_bwdProp_cos_0(_S777, &(_S861), _S860);
    var _S862 : f32 = 0.5f * (_S853 + _S855 + _S790);
    var _S863 : f32 = _S757 * _S862;
    var _S864 : f32 = 0.5f * (_S857 + _S825[i32(0)]);
    var _S865 : f32 = _S859 + _S861;
    var _S866 : vec2<f32> = vec2<f32>(0.0f, _S862);
    var _S867 : f32 = sat_0 * _S862 + _S864;
    var _S868 : vec2<f32>;
    if(_S768)
    {
        var _S869 : f32 = _S864 / _S769;
        var _S870 : vec2<f32> = _S866 + vec2<f32>(0.0f, sat_0 * _S869);
        sat_0 = _S770 * - _S869 + _S863;
        max_j_at_sat_0 = _S867;
        _S868 = _S870;
    }
    else
    {
        var _S871 : f32 = _S864 + _S867;
        sat_0 = _S863;
        max_j_at_sat_0 = _S871;
        _S868 = _S866;
    }
    if(_S758)
    {
        var _S872 : f32 = 0.0f;
        var _S873 : f32 = 0.0f;
        var _S874 : f32 = 0.0f;
        s_bwdProp_soft_clip_0(_S761, &(_S872), &(_S873), &(_S874), sat_0);
        var _S875 : f32 = _S872 / _S759;
        var _S876 : f32 = _S874 + 0.80000001192092896f * _S873;
        var _S877 : vec2<f32> = _S868 + vec2<f32>(0.0f, _S757 * _S875);
        sat_0 = _S760 * - _S875 + max_j_at_sat_0;
        _S759 = _S876;
        _S868 = _S877;
    }
    else
    {
        var _S878 : f32 = sat_0;
        sat_0 = max_j_at_sat_0;
        _S759 = _S878;
    }
    var _S879 : f32 = 0.0f;
    s_bwdProp_lookup_gamut_0(_S756, &(_S879), _S759);
    var _S880 : vec2<f32> = _S868 + vec2<f32>(sat_0, 0.0f);
    var _S881 : vec2<f32> = vec2<f32>(0.0f);
    var _S882 : vec2<f32> = _S881;
    var _S883 : vec2<f32> = _S881;
    s_bwdProp_max_0(_S755, &(_S882), &(_S883), _S880);
    var _S884 : f32 = o_0 * _S882[i32(1)];
    var _S885 : f32 = _S726.value0_5 * _S882[i32(1)];
    var _S886 : f32 = _S748.value0_19 * _S882[i32(1)];
    var _S887 : f32 = - _S882[i32(0)];
    var _S888 : f32 = o_0 * _S887;
    var _S889 : f32 = _S723.value0_6 * _S887;
    var _S890 : f32 = _S748.value0_19 * _S882[i32(0)];
    var _S891 : f32 = _S723.value0_6 * _S882[i32(1)] + _S726.value0_5 * _S882[i32(0)];
    var _S892 : f32 = 0.0f;
    var _S893 : f32 = 0.0f;
    s_bwdProp_max_1(_S750, &(_S892), &(_S893), _S891);
    var s_diff_brilliance_T_0 : f32 = s_0 * _S892;
    var _S894 : f32 = brilliance_1 * _S892;
    var _S895 : f32 = _S885 + _S889;
    var _S896 : f32 = s_0 * _S895;
    var _S897 : f32 = _S744.value0_10 * _S895;
    var _S898 : f32 = 0.0f;
    var _S899 : f32 = 0.0f;
    s_bwdProp_min_0(_S746, &(_S898), &(_S899), _S896);
    var _S900 : f32 = - _S899;
    var _S901 : f32 = 0.0f;
    var _S902 : f32 = 0.0f;
    s_bwdProp_max_1(_S742, &(_S901), &(_S902), _S898);
    var _S903 : f32 = - _S902;
    var s_diff_saturation_T_0 : f32 = _S720.value0_7 * _S901;
    var _S904 : f32 = saturation_1 * _S901;
    var _S905 : f32 = _S894 + _S897;
    var _S906 : f32 = _S712.value0_12 * _S905;
    var _S907 : f32 = _S723.value0_6 * _S905;
    var _S908 : f32 = _S710 * _S905;
    var _S909 : f32 = _S726.value0_5 * _S905;
    var _S910 : vec3<f32> = _S823;
    var _S911 : vec3<f32> = _S823;
    s_bwdProp_dot_1(_S737, &(_S910), &(_S911), s_diff_saturation_T_0);
    var _S912 : vec3<f32> = _S823;
    var _S913 : vec3<f32> = _S823;
    s_bwdProp_dot_1(_S733, &(_S912), &(_S913), s_diff_brilliance_T_0);
    var _S914 : f32 = _S884 + _S890 + _S908;
    var _S915 : f32 = 0.0f;
    s_bwdProp_cos_0(_S728, &(_S915), _S914);
    var _S916 : f32 = _S886 + _S888 + _S906;
    var _S917 : f32 = 0.0f;
    s_bwdProp_sin_0(_S725, &(_S917), _S916);
    var _S918 : f32 = _S900 + _S903 + _S904 + _S915 + _S917;
    var _S919 : f32 = 0.0f;
    var _S920 : f32 = 0.0f;
    s_bwdProp_atan2_0(_S722, &(_S919), &(_S920), _S918);
    var _S921 : f32 = _S879 + _S865;
    var _S922 : f32 = 0.0f;
    var _S923 : f32 = 0.0f;
    s_bwdProp_atan2_0(_S719, &(_S922), &(_S923), _S921);
    var _S924 : f32 = _S907 + _S919;
    var _S925 : vec2<f32> = _S881;
    s_bwdProp_length_0(_S714, &(_S925), _S924);
    var _S926 : vec3<f32> = vec3<f32>(_S909 + _S920, _S923 + _S925[i32(0)], _S922 + _S925[i32(1)]);
    var _S927 : vec3<f32> = _S823;
    s_bwdProp_xyz_to_jzazbz_0(_S709, &(_S927), _S926);
    var _S928 : Grading_0 = _S822;
    _S928.saturation_global_1 = s_diff_saturation_T_0;
    _S928.brilliance_global_1 = s_diff_brilliance_T_0;
    var _S929 : Grading_0 = Grading_x24_syn_dadd_0(_S822, _S928);
    var _S930 : vec3<f32> = _S910 + _S912;
    (*_S702) = _S927;
    (*_S703) = _S930;
    (*_S704) = _S929;
    return;
}

fn d65_xyz_to_xyy_0( xyz_1 : vec3<f32>) -> vec3<f32>
{
    var _S931 : vec3<f32> = max(xyz_1, vec3<f32>(0.0f));
    var _S932 : f32 = _S931.y;
    var sum_6 : f32 = _S931.x + _S932 + _S931.z;
    var xy_0 : vec2<f32>;
    if((sum_6 > 0.0f))
    {
        xy_0 = _S931.xy / vec2<f32>(sum_6);
    }
    else
    {
        xy_0 = vec2<f32>(0.3127099871635437f, 0.32901999354362488f);
    }
    return vec3<f32>(xy_0.x, xy_0.y, _S932);
}

struct Tuple_35
{
     value0_63 : vec3<f32>,
};

fn s_apply_d65_xyz_to_xyy_0( _S933 : vec3<f32>) -> Tuple_35
{
    var _S934 : Tuple_19 = s_apply_max_2(_S933, vec3<f32>(0.0f));
    var _S935 : f32 = _S934.value0_21.y;
    var sum_7 : f32 = _S934.value0_21.x + _S935 + _S934.value0_21.z;
    var xy_1 : vec2<f32>;
    if((sum_7 > 0.0f))
    {
        xy_1 = _S934.value0_21.xy / vec2<f32>(sum_7);
    }
    else
    {
        xy_1 = vec2<f32>(0.3127099871635437f, 0.32901999354362488f);
    }
    var _S936 : Tuple_35;
    _S936.value0_63 = vec3<f32>(xy_1.x, xy_1.y, _S935);
    return _S936;
}

struct s_paramCtx_s_bwdCallableCtx_d65_xyz_to_xyy_0
{
     value0_64 : vec3<f32>,
};

struct s_bwdCallableCtx_d65_xyz_to_xyy_0
{
     value0_65 : s_paramCtx_s_bwdCallableCtx_d65_xyz_to_xyy_0,
};

fn s_bwdProp_d65_xyz_to_xyy_0( _S937 : s_bwdCallableCtx_d65_xyz_to_xyy_0,  _S938 : ptr<function, vec3<f32>>,  _S939 : vec3<f32>)
{
    var _S940 : vec2<f32> = vec2<f32>(0.0f);
    var _S941 : vec3<f32> = vec3<f32>(0.0f);
    var _S942 : Tuple_19 = s_apply_max_2(_S937.value0_65.value0_64, _S941);
    var _S943 : s_bwdCallableCtx_max_2;
    _S943._S122 = _S937.value0_65.value0_64;
    _S943._S123 = _S941;
    var _S944 : s_bwdCallableCtx_max_2 = _S943;
    var sum_8 : f32 = _S942.value0_21.x + _S942.value0_21.y + _S942.value0_21.z;
    var _S945 : bool = (sum_8 > 0.0f);
    var _S946 : f32;
    var _S947 : vec2<f32>;
    if(_S945)
    {
        var _S948 : vec2<f32> = _S942.value0_21.xy;
        _S946 = sum_8 * sum_8;
        _S947 = _S948;
    }
    else
    {
        _S946 = 0.0f;
        _S947 = _S940;
    }
    var _S949 : vec3<f32> = vec3<f32>(0.0f);
    var _S950 : vec2<f32> = vec2<f32>(_S939[i32(0)], _S939[i32(1)]);
    var _S951 : vec3<f32>;
    if(_S945)
    {
        var _S952 : vec2<f32> = _S950 / vec2<f32>(_S946);
        var _S953 : vec2<f32> = _S947 * (vec2<f32>(0) - _S952);
        var _S954 : vec2<f32> = vec2<f32>(sum_8) * _S952;
        var _S955 : vec3<f32> = vec3<f32>(_S954[i32(0)], _S954[i32(1)], 0.0f);
        _S946 = _S953[i32(0)] + _S953[i32(1)];
        _S951 = _S955;
    }
    else
    {
        _S946 = 0.0f;
        _S951 = _S949;
    }
    var _S956 : vec3<f32> = _S951 + vec3<f32>(_S946, _S946 + _S939[i32(2)], _S946);
    var _S957 : vec3<f32> = _S949;
    var _S958 : vec3<f32> = _S949;
    s_bwdProp_max_2(_S944, &(_S957), &(_S958), _S956);
    (*_S938) = _S957;
    return;
}

fn s_fwd_d65_xyz_to_xyy_0( dpxyz_1 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S959 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( dpxyz_1.primal_0, dpxyz_1.differential_0 );
    var _S960 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.0f), vec3<f32>(0.0f) );
    var _S961 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_max_vector_3(_S959, _S960);
    var _S962 : f32 = _S961.primal_0.y;
    var _S963 : f32 = _S961.differential_0.y;
    var sum_9 : f32 = _S961.primal_0.x + _S962 + _S961.primal_0.z;
    var s_diff_sum_0 : f32 = _S961.differential_0.x + _S963 + _S961.differential_0.z;
    var xy_2 : vec2<f32>;
    var s_diff_xy_0 : vec2<f32>;
    if((sum_9 > 0.0f))
    {
        var _S964 : vec2<f32> = _S961.primal_0.xy;
        var _S965 : vec2<f32> = vec2<f32>(sum_9);
        var _S966 : vec2<f32> = (_S961.differential_0.xy * _S965 - _S964 * vec2<f32>(s_diff_sum_0)) / vec2<f32>((sum_9 * sum_9));
        xy_2 = _S964 / _S965;
        s_diff_xy_0 = _S966;
    }
    else
    {
        var _S967 : vec2<f32> = vec2<f32>(0.0f);
        xy_2 = vec2<f32>(0.3127099871635437f, 0.32901999354362488f);
        s_diff_xy_0 = _S967;
    }
    var _S968 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(xy_2.x, xy_2.y, _S962), vec3<f32>(s_diff_xy_0.x, s_diff_xy_0.y, _S963) );
    return _S968;
}

fn away_from_flt_min_0( d_0 : f32) -> f32
{
    var _S969 : f32;
    if((d_0 >= 0.0f))
    {
        _S969 = max(1.17549435082228751e-38f, d_0);
    }
    else
    {
        _S969 = min(-1.17549435082228751e-38f, d_0);
    }
    return _S969;
}

struct Tuple_36
{
     value0_66 : f32,
};

fn s_apply_away_from_flt_min_0( _S970 : f32) -> Tuple_36
{
    var _S971 : f32;
    if((_S970 >= 0.0f))
    {
        _S971 = s_apply_max_1(1.17549435082228751e-38f, _S970).value0_19;
    }
    else
    {
        _S971 = s_apply_min_0(-1.17549435082228751e-38f, _S970).value0_10;
    }
    var _S972 : Tuple_36;
    _S972.value0_66 = _S971;
    return _S972;
}

struct s_paramCtx_s_bwdCallableCtx_away_from_flt_min_0
{
     value0_67 : f32,
};

struct s_bwdCallableCtx_away_from_flt_min_0
{
     value0_68 : s_paramCtx_s_bwdCallableCtx_away_from_flt_min_0,
};

fn s_bwdProp_away_from_flt_min_0( _S973 : s_bwdCallableCtx_away_from_flt_min_0,  _S974 : ptr<function, f32>,  _S975 : f32)
{
    var _S976 : bool = ((_S973.value0_68.value0_67) >= 0.0f);
    var _S977 : s_bwdCallableCtx_max_1;
    var _S978 : s_bwdCallableCtx_min_0;
    if(_S976)
    {
        var _S979 : s_bwdCallableCtx_max_1;
        _S979._S109 = 1.17549435082228751e-38f;
        _S979._S110 = _S973.value0_68.value0_67;
        _S977 = _S979;
        _S978._S60 = 0.0f;
        _S978._S61 = 0.0f;
    }
    else
    {
        var _S980 : s_bwdCallableCtx_min_0;
        _S980._S60 = -1.17549435082228751e-38f;
        _S980._S61 = _S973.value0_68.value0_67;
        _S977._S109 = 0.0f;
        _S977._S110 = 0.0f;
        _S978 = _S980;
    }
    var _S981 : f32;
    if(_S976)
    {
        var _S982 : f32 = 0.0f;
        var _S983 : f32 = 0.0f;
        s_bwdProp_max_1(_S977, &(_S982), &(_S983), _S975);
        _S981 = _S983;
    }
    else
    {
        var _S984 : f32 = 0.0f;
        var _S985 : f32 = 0.0f;
        s_bwdProp_min_0(_S978, &(_S984), &(_S985), _S975);
        _S981 = _S985;
    }
    (*_S974) = _S981;
    return;
}

fn s_fwd_away_from_flt_min_0( dpd_0 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S986 : f32;
    var _S987 : f32;
    if(((dpd_0.primal_0) >= 0.0f))
    {
        var _S988 : DiffPair_float_0 = DiffPair_float_0( 1.17549435082228751e-38f, 0.0f );
        var _S989 : DiffPair_float_0 = DiffPair_float_0( dpd_0.primal_0, dpd_0.differential_0 );
        var _S990 : DiffPair_float_0 = _d_max_1(_S988, _S989);
        _S986 = _S990.primal_0;
        _S987 = _S990.differential_0;
    }
    else
    {
        var _S991 : DiffPair_float_0 = DiffPair_float_0( -1.17549435082228751e-38f, 0.0f );
        var _S992 : DiffPair_float_0 = DiffPair_float_0( dpd_0.primal_0, dpd_0.differential_0 );
        var _S993 : DiffPair_float_0 = _d_min_1(_S991, _S992);
        _S986 = _S993.primal_0;
        _S987 = _S993.differential_0;
    }
    var _S994 : DiffPair_float_0 = DiffPair_float_0( _S986, _S987 );
    return _S994;
}

fn xy_to_ucs_uv_0( xy_3 : vec2<f32>) -> vec2<f32>
{
    var uvd_0 : vec3<f32> = vec3<f32>(-0.78394103050231934f, 0.74527353048324585f, 0.31870728731155396f) * vec3<f32>(xy_3.x) + vec3<f32>(0.27751299738883972f, -0.20537586510181427f, 2.1674368381500244f) * vec3<f32>(xy_3.y) + vec3<f32>(0.15383657813072205f, -0.16547837853431702f, 0.2913205623626709f);
    var uv_0 : vec2<f32> = uvd_0.xy / vec2<f32>(away_from_flt_min_0(uvd_0.z));
    var uv_star_0 : vec2<f32> = vec2<f32>(1.3965622186660767f, 1.4513953924179077f) * uv_0 / (abs(uv_0) + vec2<f32>(1.4921735525131226f, 1.5248863697052002f));
    var _S995 : f32 = uv_star_0.x;
    var _S996 : f32 = uv_star_0.y;
    return vec2<f32>(-1.1249839067459106f * _S995 - 0.98048371076583862f * _S996, 1.863233208656311f * _S995 + 1.9718531370162964f * _S996);
}

struct Tuple_37
{
     value0_69 : vec2<f32>,
};

fn s_apply_xy_to_ucs_uv_0( _S997 : vec2<f32>) -> Tuple_37
{
    var uvd_1 : vec3<f32> = vec3<f32>(-0.78394103050231934f, 0.74527353048324585f, 0.31870728731155396f) * vec3<f32>(_S997.x) + vec3<f32>(0.27751299738883972f, -0.20537586510181427f, 2.1674368381500244f) * vec3<f32>(_S997.y) + vec3<f32>(0.15383657813072205f, -0.16547837853431702f, 0.2913205623626709f);
    var uv_1 : vec2<f32> = uvd_1.xy / vec2<f32>(s_apply_away_from_flt_min_0(uvd_1.z).value0_66);
    var uv_star_1 : vec2<f32> = vec2<f32>(1.3965622186660767f, 1.4513953924179077f) * uv_1 / (s_apply_abs_0(uv_1).value0_0 + vec2<f32>(1.4921735525131226f, 1.5248863697052002f));
    var _S998 : f32 = uv_star_1.x;
    var _S999 : f32 = uv_star_1.y;
    var _S1000 : Tuple_37;
    _S1000.value0_69 = vec2<f32>(-1.1249839067459106f * _S998 - 0.98048371076583862f * _S999, 1.863233208656311f * _S998 + 1.9718531370162964f * _S999);
    return _S1000;
}

struct s_paramCtx_s_bwdCallableCtx_xy_to_ucs_uv_0
{
     value0_70 : vec2<f32>,
};

struct s_bwdCallableCtx_xy_to_ucs_uv_0
{
     value0_71 : s_paramCtx_s_bwdCallableCtx_xy_to_ucs_uv_0,
};

fn s_bwdProp_xy_to_ucs_uv_0( _S1001 : s_bwdCallableCtx_xy_to_ucs_uv_0,  _S1002 : ptr<function, vec2<f32>>,  _S1003 : vec2<f32>)
{
    const _S1004 : vec3<f32> = vec3<f32>(-0.78394103050231934f, 0.74527353048324585f, 0.31870728731155396f);
    const _S1005 : vec3<f32> = vec3<f32>(0.27751299738883972f, -0.20537586510181427f, 2.1674368381500244f);
    var uvd_2 : vec3<f32> = _S1004 * vec3<f32>(_S1001.value0_71.value0_70.x) + _S1005 * vec3<f32>(_S1001.value0_71.value0_70.y) + vec3<f32>(0.15383657813072205f, -0.16547837853431702f, 0.2913205623626709f);
    var _S1006 : vec2<f32> = uvd_2.xy;
    var _S1007 : f32 = uvd_2.z;
    var _S1008 : Tuple_36 = s_apply_away_from_flt_min_0(_S1007);
    var _S1009 : s_paramCtx_s_bwdCallableCtx_away_from_flt_min_0;
    _S1009.value0_67 = _S1007;
    var _S1010 : s_bwdCallableCtx_away_from_flt_min_0 = s_bwdCallableCtx_away_from_flt_min_0( _S1009 );
    var _S1011 : vec2<f32> = vec2<f32>(_S1008.value0_66);
    var uv_2 : vec2<f32> = _S1006 / _S1011;
    var _S1012 : f32 = _S1008.value0_66 * _S1008.value0_66;
    const _S1013 : vec2<f32> = vec2<f32>(1.3965622186660767f, 1.4513953924179077f);
    var _S1014 : vec2<f32> = _S1013 * uv_2;
    var _S1015 : Tuple_0 = s_apply_abs_0(uv_2);
    var _S1016 : s_bwdCallableCtx_abs_0;
    _S1016._S2 = uv_2;
    var _S1017 : vec2<f32> = _S1015.value0_0 + vec2<f32>(1.4921735525131226f, 1.5248863697052002f);
    var _S1018 : vec2<f32> = vec2<f32>(1.863233208656311f * _S1003[i32(1)] + -1.1249839067459106f * _S1003[i32(0)], 1.9718531370162964f * _S1003[i32(1)] + 0.98048371076583862f * - _S1003[i32(0)]) / (_S1017 * _S1017);
    var _S1019 : vec2<f32> = _S1014 * (vec2<f32>(0) - _S1018);
    var _S1020 : vec2<f32> = _S1017 * _S1018;
    var _S1021 : vec2<f32> = vec2<f32>(0.0f);
    s_bwdProp_abs_0(_S1016, &(_S1021), _S1019);
    var _S1022 : vec2<f32> = (_S1021 + _S1013 * _S1020) / vec2<f32>(_S1012);
    var _S1023 : vec2<f32> = _S1006 * (vec2<f32>(0) - _S1022);
    var _S1024 : f32 = _S1023[i32(0)] + _S1023[i32(1)];
    var _S1025 : vec2<f32> = _S1011 * _S1022;
    var _S1026 : f32 = 0.0f;
    s_bwdProp_away_from_flt_min_0(_S1010, &(_S1026), _S1024);
    var s_diff_uvd_T_0 : vec3<f32> = vec3<f32>(_S1025[i32(0)], _S1025[i32(1)], _S1026);
    var _S1027 : vec3<f32> = _S1005 * s_diff_uvd_T_0;
    var _S1028 : vec3<f32> = _S1004 * s_diff_uvd_T_0;
    (*_S1002) = vec2<f32>(_S1028[i32(0)] + _S1028[i32(1)] + _S1028[i32(2)], _S1027[i32(0)] + _S1027[i32(1)] + _S1027[i32(2)]);
    return;
}

fn s_fwd_xy_to_ucs_uv_0( dpxy_0 : DiffPair_vectorx3Cfloatx2C2x3E_0) -> DiffPair_vectorx3Cfloatx2C2x3E_0
{
    const _S1029 : vec3<f32> = vec3<f32>(-0.78394103050231934f, 0.74527353048324585f, 0.31870728731155396f);
    const _S1030 : vec3<f32> = vec3<f32>(0.27751299738883972f, -0.20537586510181427f, 2.1674368381500244f);
    var _S1031 : vec3<f32> = vec3<f32>(dpxy_0.differential_0.x) * _S1029 + vec3<f32>(dpxy_0.differential_0.y) * _S1030;
    var uvd_3 : vec3<f32> = _S1029 * vec3<f32>(dpxy_0.primal_0.x) + _S1030 * vec3<f32>(dpxy_0.primal_0.y) + vec3<f32>(0.15383657813072205f, -0.16547837853431702f, 0.2913205623626709f);
    var _S1032 : vec2<f32> = uvd_3.xy;
    var _S1033 : DiffPair_float_0 = DiffPair_float_0( uvd_3.z, _S1031.z );
    var _S1034 : DiffPair_float_0 = s_fwd_away_from_flt_min_0(_S1033);
    var _S1035 : vec2<f32> = vec2<f32>(_S1034.primal_0);
    var uv_3 : vec2<f32> = _S1032 / _S1035;
    var s_diff_uv_0 : vec2<f32> = (_S1031.xy * _S1035 - _S1032 * vec2<f32>(_S1034.differential_0)) / vec2<f32>((_S1034.primal_0 * _S1034.primal_0));
    const _S1036 : vec2<f32> = vec2<f32>(1.3965622186660767f, 1.4513953924179077f);
    var _S1037 : vec2<f32> = _S1036 * uv_3;
    var _S1038 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( uv_3, s_diff_uv_0 );
    var _S1039 : DiffPair_vectorx3Cfloatx2C2x3E_0 = _d_abs_vector_2(_S1038);
    var _S1040 : vec2<f32> = _S1039.primal_0 + vec2<f32>(1.4921735525131226f, 1.5248863697052002f);
    var uv_star_2 : vec2<f32> = _S1037 / _S1040;
    var s_diff_uv_star_0 : vec2<f32> = (s_diff_uv_0 * _S1036 * _S1040 - _S1037 * _S1039.differential_0) / (_S1040 * _S1040);
    var _S1041 : f32 = uv_star_2.x;
    var _S1042 : f32 = s_diff_uv_star_0.x;
    var _S1043 : f32 = uv_star_2.y;
    var _S1044 : f32 = s_diff_uv_star_0.y;
    var _S1045 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( vec2<f32>(-1.1249839067459106f * _S1041 - 0.98048371076583862f * _S1043, 1.863233208656311f * _S1041 + 1.9718531370162964f * _S1043), vec2<f32>(_S1042 * -1.1249839067459106f - _S1044 * 0.98048371076583862f, _S1042 * 1.863233208656311f + _S1044 * 1.9718531370162964f) );
    return _S1045;
}

fn y_to_ucs_l_star_0( y_20 : f32) -> f32
{
    var y_hat_0 : f32 = pow0_0(y_20, 0.63165134191513062f);
    return 2.0988838672637939f * y_hat_0 / (y_hat_0 + 1.1242676973342896f);
}

struct Tuple_38
{
     value0_72 : f32,
};

fn s_apply_y_to_ucs_l_star_0( _S1046 : f32) -> Tuple_38
{
    var _S1047 : Tuple_16 = s_apply_pow0_0(_S1046, 0.63165134191513062f);
    var _S1048 : Tuple_38;
    _S1048.value0_72 = 2.0988838672637939f * _S1047.value0_18 / (_S1047.value0_18 + 1.1242676973342896f);
    return _S1048;
}

struct s_paramCtx_s_bwdCallableCtx_y_to_ucs_l_star_0
{
     value0_73 : f32,
};

struct s_bwdCallableCtx_y_to_ucs_l_star_0
{
     value0_74 : s_paramCtx_s_bwdCallableCtx_y_to_ucs_l_star_0,
};

fn s_bwdProp_y_to_ucs_l_star_0( _S1049 : s_bwdCallableCtx_y_to_ucs_l_star_0,  _S1050 : ptr<function, f32>,  _S1051 : f32)
{
    var _S1052 : Tuple_16 = s_apply_pow0_0(_S1049.value0_74.value0_73, 0.63165134191513062f);
    var _S1053 : s_bwdCallableCtx_pow0_0;
    _S1053._S104 = _S1049.value0_74.value0_73;
    _S1053._S105 = 0.63165134191513062f;
    var _S1054 : f32 = _S1052.value0_18 + 1.1242676973342896f;
    var _S1055 : f32 = _S1051 / (_S1054 * _S1054);
    var _S1056 : f32 = 2.0988838672637939f * _S1052.value0_18 * - _S1055 + 2.0988838672637939f * (_S1054 * _S1055);
    var _S1057 : f32 = 0.0f;
    var _S1058 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S1053, &(_S1057), &(_S1058), _S1056);
    (*_S1050) = _S1057;
    return;
}

fn s_fwd_y_to_ucs_l_star_0( dpy_17 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S1059 : DiffPair_float_0 = DiffPair_float_0( dpy_17.primal_0, dpy_17.differential_0 );
    var _S1060 : DiffPair_float_0 = DiffPair_float_0( 0.63165134191513062f, 0.0f );
    var _S1061 : DiffPair_float_0 = pow0_forward_0(_S1059, _S1060);
    var _S1062 : f32 = 2.0988838672637939f * _S1061.primal_0;
    var _S1063 : f32 = _S1061.primal_0 + 1.1242676973342896f;
    var _S1064 : DiffPair_float_0 = DiffPair_float_0( _S1062 / _S1063, (_S1061.differential_0 * 2.0988838672637939f * _S1063 - _S1062 * _S1061.differential_0) / (_S1063 * _S1063) );
    return _S1064;
}

fn xyy_to_ucs_jch_0( xyy_0 : vec3<f32>,  l_white_1 : f32) -> vec3<f32>
{
    var uv_4 : vec2<f32> = xy_to_ucs_uv_0(xyy_0.xy);
    var l_star_0 : f32 = y_to_ucs_l_star_0(xyy_0.z);
    return vec3<f32>(l_star_0 / l_white_1, 15.93299388885498f * pow0_0(l_star_0, 0.65239977836608887f) * pow0_0(dot(uv_4, uv_4), 0.60075569152832031f) / l_white_1, atan2(uv_4.y, uv_4.x));
}

struct Tuple_39
{
     value0_75 : vec3<f32>,
};

fn s_apply_xyy_to_ucs_jch_0( _S1065 : vec3<f32>,  l_white_2 : f32) -> Tuple_39
{
    var _S1066 : Tuple_37 = s_apply_xy_to_ucs_uv_0(_S1065.xy);
    var _S1067 : Tuple_38 = s_apply_y_to_ucs_l_star_0(_S1065.z);
    var _S1068 : Tuple_39;
    _S1068.value0_75 = vec3<f32>(_S1067.value0_72 / l_white_2, 15.93299388885498f * s_apply_pow0_0(_S1067.value0_72, 0.65239977836608887f).value0_18 * s_apply_pow0_0(s_apply_dot_0(_S1066.value0_69, _S1066.value0_69).value0_1, 0.60075569152832031f).value0_18 / l_white_2, s_apply_atan2_0(_S1066.value0_69.y, _S1066.value0_69.x).value0_7);
    return _S1068;
}

struct s_paramCtx_s_bwdCallableCtx_xyy_to_ucs_jch_0
{
     value0_76 : vec3<f32>,
     value1_10 : f32,
};

struct s_bwdCallableCtx_xyy_to_ucs_jch_0
{
     value0_77 : s_paramCtx_s_bwdCallableCtx_xyy_to_ucs_jch_0,
};

fn s_bwdProp_xyy_to_ucs_jch_0( _S1069 : s_bwdCallableCtx_xyy_to_ucs_jch_0,  _S1070 : ptr<function, vec3<f32>>,  _S1071 : vec3<f32>)
{
    var _S1072 : vec2<f32> = _S1069.value0_77.value0_76.xy;
    var _S1073 : Tuple_37 = s_apply_xy_to_ucs_uv_0(_S1072);
    var _S1074 : s_paramCtx_s_bwdCallableCtx_xy_to_ucs_uv_0;
    _S1074.value0_70 = _S1072;
    var _S1075 : s_bwdCallableCtx_xy_to_ucs_uv_0 = s_bwdCallableCtx_xy_to_ucs_uv_0( _S1074 );
    var _S1076 : f32 = _S1069.value0_77.value0_76.z;
    var _S1077 : Tuple_38 = s_apply_y_to_ucs_l_star_0(_S1076);
    var _S1078 : s_paramCtx_s_bwdCallableCtx_y_to_ucs_l_star_0;
    _S1078.value0_73 = _S1076;
    var _S1079 : s_bwdCallableCtx_y_to_ucs_l_star_0 = s_bwdCallableCtx_y_to_ucs_l_star_0( _S1078 );
    var _S1080 : Tuple_1 = s_apply_dot_0(_S1073.value0_69, _S1073.value0_69);
    var _S1081 : s_bwdCallableCtx_dot_0;
    _S1081._S7 = _S1073.value0_69;
    _S1081._S8 = _S1073.value0_69;
    var _S1082 : Tuple_16 = s_apply_pow0_0(_S1077.value0_72, 0.65239977836608887f);
    var _S1083 : s_bwdCallableCtx_pow0_0;
    _S1083._S104 = _S1077.value0_72;
    _S1083._S105 = 0.65239977836608887f;
    var _S1084 : f32 = 15.93299388885498f * _S1082.value0_18;
    var _S1085 : Tuple_16 = s_apply_pow0_0(_S1080.value0_1, 0.60075569152832031f);
    var _S1086 : s_bwdCallableCtx_pow0_0;
    _S1086._S104 = _S1080.value0_1;
    _S1086._S105 = 0.60075569152832031f;
    var _S1087 : f32 = _S1073.value0_69.x;
    var _S1088 : s_bwdCallableCtx_atan2_0;
    _S1088._S44 = _S1073.value0_69.y;
    _S1088._S45 = _S1087;
    var _S1089 : f32 = 0.0f;
    var _S1090 : f32 = 0.0f;
    s_bwdProp_atan2_0(_S1088, &(_S1089), &(_S1090), _S1071[i32(2)]);
    var _S1091 : f32 = _S1071[i32(1)] / _S1069.value0_77.value1_10;
    var _S1092 : f32 = _S1084 * _S1091;
    var _S1093 : f32 = _S1085.value0_18 * _S1091;
    var _S1094 : f32 = 0.0f;
    var _S1095 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S1086, &(_S1094), &(_S1095), _S1092);
    var _S1096 : f32 = 15.93299388885498f * _S1093;
    var _S1097 : f32 = 0.0f;
    var _S1098 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S1083, &(_S1097), &(_S1098), _S1096);
    var _S1099 : f32 = _S1071[i32(0)] / _S1069.value0_77.value1_10;
    var _S1100 : vec2<f32> = vec2<f32>(0.0f);
    var _S1101 : vec2<f32> = _S1100;
    var _S1102 : vec2<f32> = _S1100;
    s_bwdProp_dot_0(_S1081, &(_S1101), &(_S1102), _S1094);
    var _S1103 : f32 = _S1097 + _S1099;
    var _S1104 : f32 = 0.0f;
    s_bwdProp_y_to_ucs_l_star_0(_S1079, &(_S1104), _S1103);
    var _S1105 : vec2<f32> = _S1101 + _S1102 + vec2<f32>(_S1090, _S1089);
    var _S1106 : vec2<f32> = _S1100;
    s_bwdProp_xy_to_ucs_uv_0(_S1075, &(_S1106), _S1105);
    (*_S1070) = vec3<f32>(_S1106[i32(0)], _S1106[i32(1)], _S1104);
    return;
}

fn s_fwd_xyy_to_ucs_jch_0( dpxyy_0 : DiffPair_vectorx3Cfloatx2C3x3E_0,  l_white_3 : f32) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S1107 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( dpxyy_0.primal_0.xy, dpxyy_0.differential_0.xy );
    var _S1108 : DiffPair_vectorx3Cfloatx2C2x3E_0 = s_fwd_xy_to_ucs_uv_0(_S1107);
    var _S1109 : DiffPair_float_0 = DiffPair_float_0( dpxyy_0.primal_0.z, dpxyy_0.differential_0.z );
    var _S1110 : DiffPair_float_0 = s_fwd_y_to_ucs_l_star_0(_S1109);
    var _S1111 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( _S1108.primal_0, _S1108.differential_0 );
    var _S1112 : DiffPair_float_0 = _d_dot_2(_S1111, _S1111);
    var _S1113 : DiffPair_float_0 = DiffPair_float_0( _S1110.primal_0, _S1110.differential_0 );
    var _S1114 : DiffPair_float_0 = DiffPair_float_0( 0.65239977836608887f, 0.0f );
    var _S1115 : DiffPair_float_0 = pow0_forward_0(_S1113, _S1114);
    var _S1116 : f32 = 15.93299388885498f * _S1115.primal_0;
    var _S1117 : DiffPair_float_0 = DiffPair_float_0( _S1112.primal_0, _S1112.differential_0 );
    var _S1118 : DiffPair_float_0 = DiffPair_float_0( 0.60075569152832031f, 0.0f );
    var _S1119 : DiffPair_float_0 = pow0_forward_0(_S1117, _S1118);
    var _S1120 : DiffPair_float_0 = DiffPair_float_0( _S1108.primal_0.y, _S1108.differential_0.y );
    var _S1121 : DiffPair_float_0 = DiffPair_float_0( _S1108.primal_0.x, _S1108.differential_0.x );
    var _S1122 : DiffPair_float_0 = _d_atan2_1(_S1120, _S1121);
    var _S1123 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S1110.primal_0 / l_white_3, _S1116 * _S1119.primal_0 / l_white_3, _S1122.primal_0), vec3<f32>(_S1110.differential_0 / l_white_3, (_S1115.differential_0 * 15.93299388885498f * _S1119.primal_0 + _S1119.differential_0 * _S1116) / l_white_3, _S1122.differential_0) );
    return _S1123;
}

fn ucs_jch_to_hcb_0( jch_0 : vec3<f32>) -> vec3<f32>
{
    var _S1124 : f32 = jch_0.y;
    return vec3<f32>(jch_0.z, _S1124, jch_0.x * (pow0_0(_S1124, 1.3365422487258911f) + 1.0f));
}

struct Tuple_40
{
     value0_78 : vec3<f32>,
};

fn s_apply_ucs_jch_to_hcb_0( _S1125 : vec3<f32>) -> Tuple_40
{
    var _S1126 : f32 = _S1125.y;
    var _S1127 : Tuple_40;
    _S1127.value0_78 = vec3<f32>(_S1125.z, _S1126, _S1125.x * (s_apply_pow0_0(_S1126, 1.3365422487258911f).value0_18 + 1.0f));
    return _S1127;
}

struct s_paramCtx_s_bwdCallableCtx_ucs_jch_to_hcb_0
{
     value0_79 : vec3<f32>,
};

struct s_bwdCallableCtx_ucs_jch_to_hcb_0
{
     value0_80 : s_paramCtx_s_bwdCallableCtx_ucs_jch_to_hcb_0,
};

fn s_bwdProp_ucs_jch_to_hcb_0( _S1128 : s_bwdCallableCtx_ucs_jch_to_hcb_0,  _S1129 : ptr<function, vec3<f32>>,  _S1130 : vec3<f32>)
{
    var _S1131 : f32 = _S1128.value0_80.value0_79.y;
    var _S1132 : f32 = _S1128.value0_80.value0_79.x;
    var _S1133 : Tuple_16 = s_apply_pow0_0(_S1131, 1.3365422487258911f);
    var _S1134 : s_bwdCallableCtx_pow0_0;
    _S1134._S104 = _S1131;
    _S1134._S105 = 1.3365422487258911f;
    var _S1135 : f32 = _S1132 * _S1130[i32(2)];
    var _S1136 : f32 = (_S1133.value0_18 + 1.0f) * _S1130[i32(2)];
    var _S1137 : f32 = 0.0f;
    var _S1138 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S1134, &(_S1137), &(_S1138), _S1135);
    (*_S1129) = vec3<f32>(_S1136, _S1130[i32(1)] + _S1137, _S1130[i32(0)]);
    return;
}

fn s_fwd_ucs_jch_to_hcb_0( dpjch_0 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S1139 : f32 = dpjch_0.primal_0.y;
    var _S1140 : f32 = dpjch_0.differential_0.y;
    var _S1141 : f32 = dpjch_0.primal_0.x;
    var _S1142 : DiffPair_float_0 = DiffPair_float_0( _S1139, _S1140 );
    var _S1143 : DiffPair_float_0 = DiffPair_float_0( 1.3365422487258911f, 0.0f );
    var _S1144 : DiffPair_float_0 = pow0_forward_0(_S1142, _S1143);
    var _S1145 : f32 = _S1144.primal_0 + 1.0f;
    var _S1146 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(dpjch_0.primal_0.z, _S1139, _S1141 * _S1145), vec3<f32>(dpjch_0.differential_0.z, _S1140, dpjch_0.differential_0.x * _S1145 + _S1144.differential_0 * _S1141) );
    return _S1146;
}

fn ucs_hcb_to_jch_0( hcb_0 : vec3<f32>) -> vec3<f32>
{
    var _S1147 : f32 = hcb_0.y;
    return vec3<f32>(hcb_0.z / (pow0_0(_S1147, 1.3365422487258911f) + 1.0f), _S1147, hcb_0.x);
}

struct Tuple_41
{
     value0_81 : vec3<f32>,
};

fn s_apply_ucs_hcb_to_jch_0( _S1148 : vec3<f32>) -> Tuple_41
{
    var _S1149 : f32 = _S1148.y;
    var _S1150 : Tuple_41;
    _S1150.value0_81 = vec3<f32>(_S1148.z / (s_apply_pow0_0(_S1149, 1.3365422487258911f).value0_18 + 1.0f), _S1149, _S1148.x);
    return _S1150;
}

struct s_paramCtx_s_bwdCallableCtx_ucs_hcb_to_jch_0
{
     value0_82 : vec3<f32>,
};

struct s_bwdCallableCtx_ucs_hcb_to_jch_0
{
     value0_83 : s_paramCtx_s_bwdCallableCtx_ucs_hcb_to_jch_0,
};

fn s_bwdProp_ucs_hcb_to_jch_0( _S1151 : s_bwdCallableCtx_ucs_hcb_to_jch_0,  _S1152 : ptr<function, vec3<f32>>,  _S1153 : vec3<f32>)
{
    var _S1154 : f32 = _S1151.value0_83.value0_82.z;
    var _S1155 : f32 = _S1151.value0_83.value0_82.y;
    var _S1156 : Tuple_16 = s_apply_pow0_0(_S1155, 1.3365422487258911f);
    var _S1157 : s_bwdCallableCtx_pow0_0;
    _S1157._S104 = _S1155;
    _S1157._S105 = 1.3365422487258911f;
    var _S1158 : f32 = _S1156.value0_18 + 1.0f;
    var _S1159 : f32 = _S1153[i32(0)] / (_S1158 * _S1158);
    var _S1160 : f32 = _S1154 * - _S1159;
    var _S1161 : f32 = _S1158 * _S1159;
    var _S1162 : f32 = 0.0f;
    var _S1163 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S1157, &(_S1162), &(_S1163), _S1160);
    (*_S1152) = vec3<f32>(_S1153[i32(2)], _S1153[i32(1)] + _S1162, _S1161);
    return;
}

fn s_fwd_ucs_hcb_to_jch_0( dphcb_0 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S1164 : f32 = dphcb_0.primal_0.z;
    var _S1165 : f32 = dphcb_0.primal_0.y;
    var _S1166 : f32 = dphcb_0.differential_0.y;
    var _S1167 : DiffPair_float_0 = DiffPair_float_0( _S1165, _S1166 );
    var _S1168 : DiffPair_float_0 = DiffPair_float_0( 1.3365422487258911f, 0.0f );
    var _S1169 : DiffPair_float_0 = pow0_forward_0(_S1167, _S1168);
    var _S1170 : f32 = _S1169.primal_0 + 1.0f;
    var _S1171 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S1164 / _S1170, _S1165, dphcb_0.primal_0.x), vec3<f32>((dphcb_0.differential_0.z * _S1170 - _S1164 * _S1169.differential_0) / (_S1170 * _S1170), _S1166, dphcb_0.differential_0.x) );
    return _S1171;
}

fn ucs_jch_to_hsb_0( jch_1 : vec3<f32>) -> vec3<f32>
{
    var _S1172 : f32 = jch_1.y;
    var b_1 : f32 = jch_1.x * (pow0_0(_S1172, 1.3365422487258911f) + 1.0f);
    var s_1 : f32;
    if((b_1 > 0.0f))
    {
        s_1 = _S1172 / b_1;
    }
    else
    {
        s_1 = 0.0f;
    }
    return vec3<f32>(jch_1.z, s_1, b_1);
}

struct Tuple_42
{
     value0_84 : vec3<f32>,
};

fn s_apply_ucs_jch_to_hsb_0( _S1173 : vec3<f32>) -> Tuple_42
{
    var _S1174 : f32 = _S1173.y;
    var b_2 : f32 = _S1173.x * (s_apply_pow0_0(_S1174, 1.3365422487258911f).value0_18 + 1.0f);
    var s_2 : f32;
    if((b_2 > 0.0f))
    {
        s_2 = _S1174 / b_2;
    }
    else
    {
        s_2 = 0.0f;
    }
    var _S1175 : Tuple_42;
    _S1175.value0_84 = vec3<f32>(_S1173.z, s_2, b_2);
    return _S1175;
}

struct s_paramCtx_s_bwdCallableCtx_ucs_jch_to_hsb_0
{
     value0_85 : vec3<f32>,
};

struct s_bwdCallableCtx_ucs_jch_to_hsb_0
{
     value0_86 : s_paramCtx_s_bwdCallableCtx_ucs_jch_to_hsb_0,
};

fn s_bwdProp_ucs_jch_to_hsb_0( _S1176 : s_bwdCallableCtx_ucs_jch_to_hsb_0,  _S1177 : ptr<function, vec3<f32>>,  _S1178 : vec3<f32>)
{
    var _S1179 : f32 = _S1176.value0_86.value0_85.x;
    var _S1180 : f32 = _S1176.value0_86.value0_85.y;
    var _S1181 : Tuple_16 = s_apply_pow0_0(_S1180, 1.3365422487258911f);
    var _S1182 : s_bwdCallableCtx_pow0_0;
    _S1182._S104 = _S1180;
    _S1182._S105 = 1.3365422487258911f;
    var _S1183 : s_bwdCallableCtx_pow0_0 = _S1182;
    var _S1184 : f32 = _S1181.value0_18 + 1.0f;
    var b_3 : f32 = _S1179 * _S1184;
    var _S1185 : bool = (b_3 > 0.0f);
    var _S1186 : f32;
    if(_S1185)
    {
        _S1186 = b_3 * b_3;
    }
    else
    {
        _S1186 = 0.0f;
    }
    var _S1187 : vec3<f32> = vec3<f32>(0.0f, 0.0f, _S1178[i32(0)]);
    var _S1188 : f32;
    if(_S1185)
    {
        var _S1189 : f32 = _S1178[i32(1)] / _S1186;
        var _S1190 : f32 = b_3 * _S1189;
        _S1186 = _S1180 * - _S1189 + _S1178[i32(2)];
        _S1188 = _S1190;
    }
    else
    {
        _S1186 = _S1178[i32(2)];
        _S1188 = 0.0f;
    }
    var _S1191 : f32 = _S1179 * _S1186;
    var _S1192 : f32 = _S1184 * _S1186;
    var _S1193 : f32 = 0.0f;
    var _S1194 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S1183, &(_S1193), &(_S1194), _S1191);
    (*_S1177) = _S1187 + vec3<f32>(_S1192, _S1193 + _S1188, 0.0f);
    return;
}

fn s_fwd_ucs_jch_to_hsb_0( dpjch_1 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S1195 : f32 = dpjch_1.primal_0.x;
    var _S1196 : f32 = dpjch_1.primal_0.y;
    var _S1197 : f32 = dpjch_1.differential_0.y;
    var _S1198 : DiffPair_float_0 = DiffPair_float_0( _S1196, _S1197 );
    var _S1199 : DiffPair_float_0 = DiffPair_float_0( 1.3365422487258911f, 0.0f );
    var _S1200 : DiffPair_float_0 = pow0_forward_0(_S1198, _S1199);
    var _S1201 : f32 = _S1200.primal_0 + 1.0f;
    var b_4 : f32 = _S1195 * _S1201;
    var s_diff_b_0 : f32 = dpjch_1.differential_0.x * _S1201 + _S1200.differential_0 * _S1195;
    var s_3 : f32;
    var s_diff_s_0 : f32;
    if((b_4 > 0.0f))
    {
        var _S1202 : f32 = (_S1197 * b_4 - _S1196 * s_diff_b_0) / (b_4 * b_4);
        s_3 = _S1196 / b_4;
        s_diff_s_0 = _S1202;
    }
    else
    {
        s_3 = 0.0f;
        s_diff_s_0 = 0.0f;
    }
    var _S1203 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(dpjch_1.primal_0.z, s_3, b_4), vec3<f32>(dpjch_1.differential_0.z, s_diff_s_0, s_diff_b_0) );
    return _S1203;
}

fn ucs_hsb_to_jch_0( hsb_0 : vec3<f32>) -> vec3<f32>
{
    var _S1204 : f32 = hsb_0.z;
    var c_1 : f32 = hsb_0.y * _S1204;
    return vec3<f32>(_S1204 / (pow0_0(c_1, 1.3365422487258911f) + 1.0f), c_1, hsb_0.x);
}

struct Tuple_43
{
     value0_87 : vec3<f32>,
};

fn s_apply_ucs_hsb_to_jch_0( _S1205 : vec3<f32>) -> Tuple_43
{
    var _S1206 : f32 = _S1205.z;
    var c_2 : f32 = _S1205.y * _S1206;
    var _S1207 : Tuple_43;
    _S1207.value0_87 = vec3<f32>(_S1206 / (s_apply_pow0_0(c_2, 1.3365422487258911f).value0_18 + 1.0f), c_2, _S1205.x);
    return _S1207;
}

struct s_paramCtx_s_bwdCallableCtx_ucs_hsb_to_jch_0
{
     value0_88 : vec3<f32>,
};

struct s_bwdCallableCtx_ucs_hsb_to_jch_0
{
     value0_89 : s_paramCtx_s_bwdCallableCtx_ucs_hsb_to_jch_0,
};

fn s_bwdProp_ucs_hsb_to_jch_0( _S1208 : s_bwdCallableCtx_ucs_hsb_to_jch_0,  _S1209 : ptr<function, vec3<f32>>,  _S1210 : vec3<f32>)
{
    var _S1211 : f32 = _S1208.value0_89.value0_88.y;
    var _S1212 : f32 = _S1208.value0_89.value0_88.z;
    var c_3 : f32 = _S1211 * _S1212;
    var _S1213 : Tuple_16 = s_apply_pow0_0(c_3, 1.3365422487258911f);
    var _S1214 : s_bwdCallableCtx_pow0_0;
    _S1214._S104 = c_3;
    _S1214._S105 = 1.3365422487258911f;
    var _S1215 : f32 = _S1213.value0_18 + 1.0f;
    var _S1216 : f32 = _S1210[i32(0)] / (_S1215 * _S1215);
    var _S1217 : f32 = _S1212 * - _S1216;
    var _S1218 : f32 = _S1215 * _S1216;
    var _S1219 : f32 = 0.0f;
    var _S1220 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S1214, &(_S1219), &(_S1220), _S1217);
    var _S1221 : f32 = _S1210[i32(1)] + _S1219;
    (*_S1209) = vec3<f32>(_S1210[i32(2)], _S1212 * _S1221, _S1218 + _S1211 * _S1221);
    return;
}

fn s_fwd_ucs_hsb_to_jch_0( dphsb_0 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S1222 : f32 = dphsb_0.primal_0.y;
    var _S1223 : f32 = dphsb_0.primal_0.z;
    var _S1224 : f32 = dphsb_0.differential_0.z;
    var c_4 : f32 = _S1222 * _S1223;
    var s_diff_c_0 : f32 = dphsb_0.differential_0.y * _S1223 + _S1224 * _S1222;
    var _S1225 : DiffPair_float_0 = DiffPair_float_0( c_4, s_diff_c_0 );
    var _S1226 : DiffPair_float_0 = DiffPair_float_0( 1.3365422487258911f, 0.0f );
    var _S1227 : DiffPair_float_0 = pow0_forward_0(_S1225, _S1226);
    var _S1228 : f32 = _S1227.primal_0 + 1.0f;
    var _S1229 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S1223 / _S1228, c_4, dphsb_0.primal_0.x), vec3<f32>((_S1224 * _S1228 - _S1223 * _S1227.differential_0) / (_S1228 * _S1228), s_diff_c_0, dphsb_0.differential_0.x) );
    return _S1229;
}

fn ucs_l_star_to_y_0( l_star_1 : f32) -> f32
{
    return pow0_0(1.1242676973342896f * l_star_1 / (2.0988838672637939f - l_star_1), 1.5831518173217773f);
}

struct Tuple_44
{
     value0_90 : f32,
};

fn s_apply_ucs_l_star_to_y_0( _S1230 : f32) -> Tuple_44
{
    var _S1231 : Tuple_44;
    _S1231.value0_90 = s_apply_pow0_0(1.1242676973342896f * _S1230 / (2.0988838672637939f - _S1230), 1.5831518173217773f).value0_18;
    return _S1231;
}

struct s_paramCtx_s_bwdCallableCtx_ucs_l_star_to_y_0
{
     value0_91 : f32,
};

struct s_bwdCallableCtx_ucs_l_star_to_y_0
{
     value0_92 : s_paramCtx_s_bwdCallableCtx_ucs_l_star_to_y_0,
};

fn s_bwdProp_ucs_l_star_to_y_0( _S1232 : s_bwdCallableCtx_ucs_l_star_to_y_0,  _S1233 : ptr<function, f32>,  _S1234 : f32)
{
    var _S1235 : f32 = 1.1242676973342896f * _S1232.value0_92.value0_91;
    var _S1236 : f32 = 2.0988838672637939f - _S1232.value0_92.value0_91;
    var _S1237 : f32 = _S1236 * _S1236;
    var _S1238 : s_bwdCallableCtx_pow0_0;
    _S1238._S104 = _S1235 / _S1236;
    _S1238._S105 = 1.5831518173217773f;
    var _S1239 : f32 = 0.0f;
    var _S1240 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S1238, &(_S1239), &(_S1240), _S1234);
    var _S1241 : f32 = _S1239 / _S1237;
    (*_S1233) = - (_S1235 * - _S1241) + 1.1242676973342896f * (_S1236 * _S1241);
    return;
}

fn s_fwd_ucs_l_star_to_y_0( dpl_star_0 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S1242 : f32 = 1.1242676973342896f * dpl_star_0.primal_0;
    var _S1243 : f32 = 2.0988838672637939f - dpl_star_0.primal_0;
    var _S1244 : DiffPair_float_0 = DiffPair_float_0( _S1242 / _S1243, (dpl_star_0.differential_0 * 1.1242676973342896f * _S1243 - _S1242 * (0.0f - dpl_star_0.differential_0)) / (_S1243 * _S1243) );
    var _S1245 : DiffPair_float_0 = DiffPair_float_0( 1.5831518173217773f, 0.0f );
    var _S1246 : DiffPair_float_0 = pow0_forward_0(_S1244, _S1245);
    var _S1247 : DiffPair_float_0 = DiffPair_float_0( _S1246.primal_0, _S1246.differential_0 );
    return _S1247;
}

fn ucs_jch_to_xyy_0( jch_2 : vec3<f32>,  l_white_4 : f32) -> vec3<f32>
{
    var l_star_2 : f32 = clamp(jch_2.x * l_white_4, 0.0f, 2.0988500118255615f);
    var m_0 : f32;
    if(l_star_2 != 0.0f)
    {
        m_0 = pow0_0(jch_2.y * l_white_4 / (15.93299388885498f * pow0_0(l_star_2, 0.65239977836608887f)), 0.83228504657745361f);
    }
    else
    {
        m_0 = 0.0f;
    }
    var _S1248 : f32 = jch_2.z;
    var uv_star_prime_0 : vec2<f32> = vec2<f32>(m_0) * vec2<f32>(cos(_S1248), sin(_S1248));
    var _S1249 : f32 = uv_star_prime_0.x;
    var _S1250 : f32 = uv_star_prime_0.y;
    var uv_star_3 : vec2<f32> = vec2<f32>(-5.0375223159790039f * _S1249 - 2.5048563480377197f * _S1250, 4.7600293159484863f * _S1249 + 2.8740129470825195f * _S1250);
    var uv_5 : vec2<f32> = (vec2<f32>(0) - vec2<f32>(1.4921735525131226f, 1.5248863697052002f)) * uv_star_3 / (abs(uv_star_3) - vec2<f32>(1.3965622186660767f, 1.4513953924179077f));
    var xyd_0 : vec3<f32> = vec3<f32>(0.16717147827148438f, -0.15095908939838409f, 0.94025474786758423f) * vec3<f32>(uv_5.x) + vec3<f32>(0.14129979908466339f, -0.15518505871295929f, 1.0f) * vec3<f32>(uv_5.y) + vec3<f32>(-0.0080153131857514381f, -0.0084331240504980087f, -0.025632597506046295f);
    var xy_4 : vec2<f32> = xyd_0.xy / vec2<f32>(away_from_flt_min_0(xyd_0.z));
    return vec3<f32>(xy_4.x, xy_4.y, ucs_l_star_to_y_0(l_star_2));
}

struct Tuple_45
{
     value0_93 : vec3<f32>,
};

fn s_apply_ucs_jch_to_xyy_0( _S1251 : vec3<f32>,  l_white_5 : f32) -> Tuple_45
{
    var _S1252 : Tuple_2 = s_apply_clamp_0(_S1251.x * l_white_5, 0.0f, 2.0988500118255615f);
    var m_1 : f32;
    if((_S1252.value0_2) != 0.0f)
    {
        m_1 = s_apply_pow0_0(_S1251.y * l_white_5 / (15.93299388885498f * s_apply_pow0_0(_S1252.value0_2, 0.65239977836608887f).value0_18), 0.83228504657745361f).value0_18;
    }
    else
    {
        m_1 = 0.0f;
    }
    var _S1253 : f32 = _S1251.z;
    var uv_star_prime_1 : vec2<f32> = vec2<f32>(m_1) * vec2<f32>(s_apply_cos_0(_S1253).value0_5, s_apply_sin_0(_S1253).value0_6);
    var _S1254 : f32 = uv_star_prime_1.x;
    var _S1255 : f32 = uv_star_prime_1.y;
    var uv_star_4 : vec2<f32> = vec2<f32>(-5.0375223159790039f * _S1254 - 2.5048563480377197f * _S1255, 4.7600293159484863f * _S1254 + 2.8740129470825195f * _S1255);
    var uv_6 : vec2<f32> = (vec2<f32>(0) - vec2<f32>(1.4921735525131226f, 1.5248863697052002f)) * uv_star_4 / (s_apply_abs_0(uv_star_4).value0_0 - vec2<f32>(1.3965622186660767f, 1.4513953924179077f));
    var xyd_1 : vec3<f32> = vec3<f32>(0.16717147827148438f, -0.15095908939838409f, 0.94025474786758423f) * vec3<f32>(uv_6.x) + vec3<f32>(0.14129979908466339f, -0.15518505871295929f, 1.0f) * vec3<f32>(uv_6.y) + vec3<f32>(-0.0080153131857514381f, -0.0084331240504980087f, -0.025632597506046295f);
    var xy_5 : vec2<f32> = xyd_1.xy / vec2<f32>(s_apply_away_from_flt_min_0(xyd_1.z).value0_66);
    var _S1256 : Tuple_45;
    _S1256.value0_93 = vec3<f32>(xy_5.x, xy_5.y, s_apply_ucs_l_star_to_y_0(_S1252.value0_2).value0_90);
    return _S1256;
}

struct s_paramCtx_s_bwdCallableCtx_ucs_jch_to_xyy_0
{
     value0_94 : vec3<f32>,
     value1_11 : f32,
};

struct s_bwdCallableCtx_ucs_jch_to_xyy_0
{
     value0_95 : s_paramCtx_s_bwdCallableCtx_ucs_jch_to_xyy_0,
};

fn s_bwdProp_ucs_jch_to_xyy_0( _S1257 : s_bwdCallableCtx_ucs_jch_to_xyy_0,  _S1258 : ptr<function, vec3<f32>>,  _S1259 : vec3<f32>)
{
    var _S1260 : f32 = _S1257.value0_95.value0_94.x * _S1257.value0_95.value1_11;
    var _S1261 : Tuple_2 = s_apply_clamp_0(_S1260, 0.0f, 2.0988500118255615f);
    var _S1262 : s_bwdCallableCtx_clamp_0;
    _S1262._S12 = _S1260;
    _S1262._S13 = 0.0f;
    _S1262._S14 = 2.0988500118255615f;
    var _S1263 : s_bwdCallableCtx_clamp_0 = _S1262;
    var _S1264 : bool = (_S1261.value0_2) != 0.0f;
    var m_2 : f32;
    var _S1265 : f32;
    var _S1266 : f32;
    var _S1267 : f32;
    var _S1268 : s_bwdCallableCtx_pow0_0;
    var _S1269 : s_bwdCallableCtx_pow0_0;
    if(_S1264)
    {
        var _S1270 : f32 = _S1257.value0_95.value0_94.y * _S1257.value0_95.value1_11;
        var _S1271 : Tuple_16 = s_apply_pow0_0(_S1261.value0_2, 0.65239977836608887f);
        var _S1272 : s_bwdCallableCtx_pow0_0;
        _S1272._S104 = _S1261.value0_2;
        _S1272._S105 = 0.65239977836608887f;
        var _S1273 : f32 = 15.93299388885498f * _S1271.value0_18;
        var _S1274 : f32 = _S1270 / _S1273;
        var _S1275 : f32 = _S1273 * _S1273;
        var _S1276 : Tuple_16 = s_apply_pow0_0(_S1274, 0.83228504657745361f);
        var _S1277 : s_bwdCallableCtx_pow0_0;
        _S1277._S104 = _S1274;
        _S1277._S105 = 0.83228504657745361f;
        m_2 = _S1276.value0_18;
        _S1268 = _S1277;
        _S1265 = _S1275;
        _S1266 = _S1270;
        _S1267 = _S1273;
        _S1269 = _S1272;
    }
    else
    {
        m_2 = 0.0f;
        _S1268._S104 = 0.0f;
        _S1268._S105 = 0.0f;
        _S1265 = 0.0f;
        _S1266 = 0.0f;
        _S1267 = 0.0f;
        _S1269._S104 = 0.0f;
        _S1269._S105 = 0.0f;
    }
    var _S1278 : f32 = _S1257.value0_95.value0_94.z;
    var _S1279 : Tuple_5 = s_apply_cos_0(_S1278);
    var _S1280 : s_bwdCallableCtx_cos_0;
    _S1280._S36 = _S1278;
    var _S1281 : s_bwdCallableCtx_cos_0 = _S1280;
    var _S1282 : Tuple_6 = s_apply_sin_0(_S1278);
    var _S1283 : s_bwdCallableCtx_sin_0;
    _S1283._S40 = _S1278;
    var _S1284 : s_bwdCallableCtx_sin_0 = _S1283;
    var _S1285 : vec2<f32> = vec2<f32>(_S1279.value0_5, _S1282.value0_6);
    var uv_star_prime_2 : vec2<f32> = vec2<f32>(m_2) * _S1285;
    var _S1286 : f32 = uv_star_prime_2.x;
    var _S1287 : f32 = uv_star_prime_2.y;
    var uv_star_5 : vec2<f32> = vec2<f32>(-5.0375223159790039f * _S1286 - 2.5048563480377197f * _S1287, 4.7600293159484863f * _S1286 + 2.8740129470825195f * _S1287);
    var _S1288 : vec2<f32> = (vec2<f32>(0) - vec2<f32>(1.4921735525131226f, 1.5248863697052002f));
    var _S1289 : vec2<f32> = _S1288 * uv_star_5;
    var _S1290 : Tuple_0 = s_apply_abs_0(uv_star_5);
    var _S1291 : s_bwdCallableCtx_abs_0;
    _S1291._S2 = uv_star_5;
    var _S1292 : s_bwdCallableCtx_abs_0 = _S1291;
    var _S1293 : vec2<f32> = _S1290.value0_0 - vec2<f32>(1.3965622186660767f, 1.4513953924179077f);
    var uv_7 : vec2<f32> = _S1289 / _S1293;
    var _S1294 : vec2<f32> = _S1293 * _S1293;
    const _S1295 : vec3<f32> = vec3<f32>(0.16717147827148438f, -0.15095908939838409f, 0.94025474786758423f);
    const _S1296 : vec3<f32> = vec3<f32>(0.14129979908466339f, -0.15518505871295929f, 1.0f);
    var xyd_2 : vec3<f32> = _S1295 * vec3<f32>(uv_7.x) + _S1296 * vec3<f32>(uv_7.y) + vec3<f32>(-0.0080153131857514381f, -0.0084331240504980087f, -0.025632597506046295f);
    var _S1297 : vec2<f32> = xyd_2.xy;
    var _S1298 : f32 = xyd_2.z;
    var _S1299 : Tuple_36 = s_apply_away_from_flt_min_0(_S1298);
    var _S1300 : s_paramCtx_s_bwdCallableCtx_away_from_flt_min_0;
    _S1300.value0_67 = _S1298;
    var _S1301 : s_bwdCallableCtx_away_from_flt_min_0 = s_bwdCallableCtx_away_from_flt_min_0( _S1300 );
    var _S1302 : f32 = _S1299.value0_66 * _S1299.value0_66;
    var _S1303 : s_paramCtx_s_bwdCallableCtx_ucs_l_star_to_y_0;
    _S1303.value0_91 = _S1261.value0_2;
    var _S1304 : s_bwdCallableCtx_ucs_l_star_to_y_0 = s_bwdCallableCtx_ucs_l_star_to_y_0( _S1303 );
    var _S1305 : f32 = 0.0f;
    s_bwdProp_ucs_l_star_to_y_0(_S1304, &(_S1305), _S1259[i32(2)]);
    var _S1306 : f32 = _S1305;
    var _S1307 : vec2<f32> = vec2<f32>(_S1259[i32(0)], _S1259[i32(1)]) / vec2<f32>(_S1302);
    var _S1308 : vec2<f32> = _S1297 * (vec2<f32>(0) - _S1307);
    var _S1309 : f32 = _S1308[i32(0)] + _S1308[i32(1)];
    var _S1310 : vec2<f32> = vec2<f32>(_S1299.value0_66) * _S1307;
    var _S1311 : f32 = 0.0f;
    s_bwdProp_away_from_flt_min_0(_S1301, &(_S1311), _S1309);
    var s_diff_xyd_T_0 : vec3<f32> = vec3<f32>(_S1310[i32(0)], _S1310[i32(1)], _S1311);
    var _S1312 : vec3<f32> = _S1296 * s_diff_xyd_T_0;
    var _S1313 : vec3<f32> = _S1295 * s_diff_xyd_T_0;
    var _S1314 : vec2<f32> = vec2<f32>(_S1313[i32(0)] + _S1313[i32(1)] + _S1313[i32(2)], _S1312[i32(0)] + _S1312[i32(1)] + _S1312[i32(2)]) / _S1294;
    var _S1315 : vec2<f32> = _S1289 * (vec2<f32>(0) - _S1314);
    var _S1316 : vec2<f32> = _S1293 * _S1314;
    var _S1317 : vec2<f32> = vec2<f32>(0.0f);
    s_bwdProp_abs_0(_S1292, &(_S1317), _S1315);
    var _S1318 : vec2<f32> = _S1317 + _S1288 * _S1316;
    var s_diff_uv_star_prime_T_0 : vec2<f32> = vec2<f32>(4.7600293159484863f * _S1318[i32(1)] + -5.0375223159790039f * _S1318[i32(0)], 2.8740129470825195f * _S1318[i32(1)] + 2.5048563480377197f * - _S1318[i32(0)]);
    var _S1319 : vec2<f32> = vec2<f32>(m_2) * s_diff_uv_star_prime_T_0;
    var _S1320 : vec2<f32> = _S1285 * s_diff_uv_star_prime_T_0;
    var _S1321 : f32 = _S1320[i32(0)] + _S1320[i32(1)];
    var _S1322 : f32 = 0.0f;
    s_bwdProp_sin_0(_S1284, &(_S1322), _S1319[i32(1)]);
    var _S1323 : f32 = 0.0f;
    s_bwdProp_cos_0(_S1281, &(_S1323), _S1319[i32(0)]);
    var _S1324 : vec3<f32> = vec3<f32>(0.0f, 0.0f, _S1322 + _S1323);
    var _S1325 : vec3<f32>;
    if(_S1264)
    {
        var _S1326 : f32 = 0.0f;
        var _S1327 : f32 = 0.0f;
        s_bwdProp_pow0_0(_S1268, &(_S1326), &(_S1327), _S1321);
        var _S1328 : f32 = _S1326 / _S1265;
        var _S1329 : f32 = _S1267 * _S1328;
        var _S1330 : f32 = 15.93299388885498f * (_S1266 * - _S1328);
        var _S1331 : f32 = 0.0f;
        var _S1332 : f32 = 0.0f;
        s_bwdProp_pow0_0(_S1269, &(_S1331), &(_S1332), _S1330);
        var _S1333 : vec3<f32> = _S1324 + vec3<f32>(0.0f, _S1257.value0_95.value1_11 * _S1329, 0.0f);
        m_2 = _S1331 + _S1306;
        _S1325 = _S1333;
    }
    else
    {
        m_2 = _S1306;
        _S1325 = _S1324;
    }
    var _S1334 : f32 = 0.0f;
    var _S1335 : f32 = 0.0f;
    var _S1336 : f32 = 0.0f;
    s_bwdProp_clamp_0(_S1263, &(_S1334), &(_S1335), &(_S1336), m_2);
    (*_S1258) = _S1325 + vec3<f32>(_S1257.value0_95.value1_11 * _S1334, 0.0f, 0.0f);
    return;
}

fn s_fwd_ucs_jch_to_xyy_0( dpjch_2 : DiffPair_vectorx3Cfloatx2C3x3E_0,  l_white_6 : f32) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S1337 : DiffPair_float_0 = DiffPair_float_0( dpjch_2.primal_0.x * l_white_6, dpjch_2.differential_0.x * l_white_6 );
    var _S1338 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S1339 : DiffPair_float_0 = DiffPair_float_0( 2.0988500118255615f, 0.0f );
    var _S1340 : DiffPair_float_0 = _d_clamp_1(_S1337, _S1338, _S1339);
    var m_3 : f32;
    var s_diff_m_0 : f32;
    if((_S1340.primal_0) != 0.0f)
    {
        var _S1341 : f32 = dpjch_2.primal_0.y * l_white_6;
        var _S1342 : DiffPair_float_0 = DiffPair_float_0( _S1340.primal_0, _S1340.differential_0 );
        var _S1343 : DiffPair_float_0 = DiffPair_float_0( 0.65239977836608887f, 0.0f );
        var _S1344 : DiffPair_float_0 = pow0_forward_0(_S1342, _S1343);
        var _S1345 : f32 = 15.93299388885498f * _S1344.primal_0;
        var _S1346 : DiffPair_float_0 = DiffPair_float_0( _S1341 / _S1345, (dpjch_2.differential_0.y * l_white_6 * _S1345 - _S1341 * (_S1344.differential_0 * 15.93299388885498f)) / (_S1345 * _S1345) );
        var _S1347 : DiffPair_float_0 = DiffPair_float_0( 0.83228504657745361f, 0.0f );
        var _S1348 : DiffPair_float_0 = pow0_forward_0(_S1346, _S1347);
        m_3 = _S1348.primal_0;
        s_diff_m_0 = _S1348.differential_0;
    }
    else
    {
        m_3 = 0.0f;
        s_diff_m_0 = 0.0f;
    }
    var _S1349 : DiffPair_float_0 = DiffPair_float_0( dpjch_2.primal_0.z, dpjch_2.differential_0.z );
    var _S1350 : DiffPair_float_0 = _d_cos_1(_S1349);
    var _S1351 : DiffPair_float_0 = _d_sin_1(_S1349);
    var _S1352 : vec2<f32> = vec2<f32>(_S1350.primal_0, _S1351.primal_0);
    var uv_star_prime_3 : vec2<f32> = vec2<f32>(m_3) * _S1352;
    var s_diff_uv_star_prime_0 : vec2<f32> = vec2<f32>(s_diff_m_0) * _S1352 + vec2<f32>(_S1350.differential_0, _S1351.differential_0) * vec2<f32>(m_3);
    var _S1353 : f32 = uv_star_prime_3.x;
    var _S1354 : f32 = s_diff_uv_star_prime_0.x;
    var _S1355 : f32 = uv_star_prime_3.y;
    var _S1356 : f32 = s_diff_uv_star_prime_0.y;
    var uv_star_6 : vec2<f32> = vec2<f32>(-5.0375223159790039f * _S1353 - 2.5048563480377197f * _S1355, 4.7600293159484863f * _S1353 + 2.8740129470825195f * _S1355);
    var s_diff_uv_star_1 : vec2<f32> = vec2<f32>(_S1354 * -5.0375223159790039f - _S1356 * 2.5048563480377197f, _S1354 * 4.7600293159484863f + _S1356 * 2.8740129470825195f);
    var _S1357 : vec2<f32> = (vec2<f32>(0) - vec2<f32>(1.4921735525131226f, 1.5248863697052002f));
    var _S1358 : vec2<f32> = _S1357 * uv_star_6;
    var _S1359 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( uv_star_6, s_diff_uv_star_1 );
    var _S1360 : DiffPair_vectorx3Cfloatx2C2x3E_0 = _d_abs_vector_2(_S1359);
    var _S1361 : vec2<f32> = _S1360.primal_0 - vec2<f32>(1.3965622186660767f, 1.4513953924179077f);
    var uv_8 : vec2<f32> = _S1358 / _S1361;
    var s_diff_uv_1 : vec2<f32> = (((vec2<f32>(0) - vec2<f32>(0.0f, 0.0f)) * uv_star_6 + s_diff_uv_star_1 * _S1357) * _S1361 - _S1358 * _S1360.differential_0) / (_S1361 * _S1361);
    const _S1362 : vec3<f32> = vec3<f32>(0.16717147827148438f, -0.15095908939838409f, 0.94025474786758423f);
    const _S1363 : vec3<f32> = vec3<f32>(0.14129979908466339f, -0.15518505871295929f, 1.0f);
    var _S1364 : vec3<f32> = vec3<f32>(s_diff_uv_1.x) * _S1362 + vec3<f32>(s_diff_uv_1.y) * _S1363;
    var xyd_3 : vec3<f32> = _S1362 * vec3<f32>(uv_8.x) + _S1363 * vec3<f32>(uv_8.y) + vec3<f32>(-0.0080153131857514381f, -0.0084331240504980087f, -0.025632597506046295f);
    var _S1365 : vec2<f32> = xyd_3.xy;
    var _S1366 : DiffPair_float_0 = DiffPair_float_0( xyd_3.z, _S1364.z );
    var _S1367 : DiffPair_float_0 = s_fwd_away_from_flt_min_0(_S1366);
    var _S1368 : vec2<f32> = vec2<f32>(_S1367.primal_0);
    var xy_6 : vec2<f32> = _S1365 / _S1368;
    var s_diff_xy_1 : vec2<f32> = (_S1364.xy * _S1368 - _S1365 * vec2<f32>(_S1367.differential_0)) / vec2<f32>((_S1367.primal_0 * _S1367.primal_0));
    var _S1369 : DiffPair_float_0 = DiffPair_float_0( _S1340.primal_0, _S1340.differential_0 );
    var _S1370 : DiffPair_float_0 = s_fwd_ucs_l_star_to_y_0(_S1369);
    var _S1371 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(xy_6.x, xy_6.y, _S1370.primal_0), vec3<f32>(s_diff_xy_1.x, s_diff_xy_1.y, _S1370.differential_0) );
    return _S1371;
}

fn xyy_to_xyz_0( xyy_1 : vec3<f32>) -> vec3<f32>
{
    var _S1372 : f32 = xyy_1.y;
    if(_S1372 == 0.0f)
    {
        return vec3<f32>(0.0f);
    }
    var _S1373 : f32 = xyy_1.z;
    var _S1374 : f32 = xyy_1.x;
    return vec3<f32>(_S1373 * _S1374 / _S1372, _S1373, _S1373 * (1.0f - _S1374 - _S1372) / _S1372);
}

struct Tuple_46
{
     value0_96 : vec3<f32>,
};

fn s_apply_xyy_to_xyz_0( _S1375 : vec3<f32>) -> Tuple_46
{
    var _S1376 : f32 = _S1375.y;
    var _S1377 : bool = _S1376 == 0.0f;
    var _S1378 : vec3<f32>;
    if(_S1377)
    {
        _S1378 = vec3<f32>(0.0f);
    }
    var _S1379 : bool = !_S1377;
    if(_S1379)
    {
        var _S1380 : f32 = _S1375.z;
        var _S1381 : f32 = _S1375.x;
        _S1378 = vec3<f32>(_S1380 * _S1381 / _S1376, _S1380, _S1380 * (1.0f - _S1381 - _S1376) / _S1376);
    }
    var _S1382 : Tuple_46;
    _S1382.value0_96 = _S1378;
    return _S1382;
}

struct s_paramCtx_s_bwdCallableCtx_xyy_to_xyz_0
{
     value0_97 : vec3<f32>,
};

struct s_bwdCallableCtx_xyy_to_xyz_0
{
     value0_98 : s_paramCtx_s_bwdCallableCtx_xyy_to_xyz_0,
};

fn s_bwdProp_xyy_to_xyz_0( _S1383 : s_bwdCallableCtx_xyy_to_xyz_0,  _S1384 : ptr<function, vec3<f32>>,  _S1385 : vec3<f32>)
{
    var _S1386 : f32 = _S1383.value0_98.value0_97.y;
    var _S1387 : bool = !(_S1386 == 0.0f);
    var _S1388 : f32;
    var _S1389 : f32;
    var _S1390 : f32;
    var _S1391 : f32;
    var _S1392 : f32;
    var _S1393 : f32;
    if(_S1387)
    {
        var _S1394 : f32 = _S1383.value0_98.value0_97.z;
        var _S1395 : f32 = _S1383.value0_98.value0_97.x;
        var _S1396 : f32 = _S1394 * _S1395;
        var _S1397 : f32 = 1.0f - _S1395 - _S1386;
        var _S1398 : f32 = _S1394 * _S1397;
        _S1388 = _S1386 * _S1386;
        _S1389 = _S1398;
        _S1390 = _S1394;
        _S1391 = _S1397;
        _S1392 = _S1396;
        _S1393 = _S1395;
    }
    else
    {
        _S1388 = 0.0f;
        _S1389 = 0.0f;
        _S1390 = 0.0f;
        _S1391 = 0.0f;
        _S1392 = 0.0f;
        _S1393 = 0.0f;
    }
    var _S1399 : vec3<f32> = vec3<f32>(0.0f);
    var _S1400 : vec3<f32>;
    if(_S1387)
    {
        var _S1401 : f32 = _S1385[i32(2)] / _S1388;
        var _S1402 : f32 = _S1386 * _S1401;
        var _S1403 : f32 = - (_S1390 * _S1402);
        var _S1404 : f32 = _S1385[i32(0)] / _S1388;
        var _S1405 : f32 = _S1386 * _S1404;
        var _S1406 : vec3<f32> = vec3<f32>(_S1403 + _S1390 * _S1405, 0.0f, _S1385[i32(1)] + _S1391 * _S1402 + _S1393 * _S1405);
        _S1388 = _S1389 * - _S1401 + _S1403 + _S1392 * - _S1404;
        _S1400 = _S1406;
    }
    else
    {
        _S1388 = 0.0f;
        _S1400 = _S1399;
    }
    (*_S1384) = _S1400 + vec3<f32>(0.0f, _S1388, 0.0f);
    return;
}

fn s_fwd_xyy_to_xyz_0( dpxyy_1 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S1407 : f32 = dpxyy_1.primal_0.y;
    var _S1408 : f32 = dpxyy_1.differential_0.y;
    if(_S1407 == 0.0f)
    {
        var _S1409 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.0f), vec3<f32>(0.0f) );
        return _S1409;
    }
    var _S1410 : f32 = dpxyy_1.primal_0.z;
    var _S1411 : f32 = dpxyy_1.differential_0.z;
    var _S1412 : f32 = dpxyy_1.primal_0.x;
    var _S1413 : f32 = dpxyy_1.differential_0.x;
    var _S1414 : f32 = _S1410 * _S1412;
    var _S1415 : f32 = _S1407 * _S1407;
    var _S1416 : f32 = 1.0f - _S1412 - _S1407;
    var _S1417 : f32 = _S1410 * _S1416;
    var _S1418 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S1414 / _S1407, _S1410, _S1417 / _S1407), vec3<f32>(((_S1411 * _S1412 + _S1413 * _S1410) * _S1407 - _S1414 * _S1408) / _S1415, _S1411, ((_S1411 * _S1416 + (0.0f - _S1413 - _S1408) * _S1410) * _S1407 - _S1417 * _S1408) / _S1415) );
    return _S1418;
}

struct s_paramCtx_s_bwdCallableCtx_ucs_saturation_0
{
     value0_99 : vec3<f32>,
     value1_12 : vec3<f32>,
     value2_2 : ColorBalanceParams_0,
     value3_1 : Grading_0,
};

struct s_minCtx_s_bwdCallableCtx_ucs_saturation_0
{
     value0_100 : f32,
     value1_13 : s_minCtx_s_bwdCallableCtx_lookup_gamut_0,
};

struct s_bwdCallableCtx_ucs_saturation_0
{
     value0_101 : s_paramCtx_s_bwdCallableCtx_ucs_saturation_0,
     value1_14 : s_minCtx_s_bwdCallableCtx_ucs_saturation_0,
};

fn s_bwdProp_ucs_saturation_0( _S1419 : s_bwdCallableCtx_ucs_saturation_0,  _S1420 : ptr<function, vec3<f32>>,  _S1421 : ptr<function, vec3<f32>>,  _S1422 : ptr<function, Grading_0>,  _S1423 : vec3<f32>)
{
    var _S1424 : Tuple_35 = s_apply_d65_xyz_to_xyy_0(_S1419.value0_101.value0_99);
    var _S1425 : s_paramCtx_s_bwdCallableCtx_d65_xyz_to_xyy_0;
    _S1425.value0_64 = _S1419.value0_101.value0_99;
    var _S1426 : s_bwdCallableCtx_d65_xyz_to_xyy_0 = s_bwdCallableCtx_d65_xyz_to_xyy_0( _S1425 );
    var _S1427 : Tuple_39 = s_apply_xyy_to_ucs_jch_0(_S1424.value0_63, _S1419.value0_101.value2_2.l_white_0);
    var _S1428 : s_paramCtx_s_bwdCallableCtx_xyy_to_ucs_jch_0;
    _S1428.value0_76 = _S1424.value0_63;
    _S1428.value1_10 = _S1419.value0_101.value2_2.l_white_0;
    var _S1429 : s_bwdCallableCtx_xyy_to_ucs_jch_0 = s_bwdCallableCtx_xyy_to_ucs_jch_0( _S1428 );
    var _S1430 : Tuple_40 = s_apply_ucs_jch_to_hcb_0(_S1427.value0_75);
    var _S1431 : s_paramCtx_s_bwdCallableCtx_ucs_jch_to_hcb_0;
    _S1431.value0_79 = _S1427.value0_75;
    var _S1432 : s_bwdCallableCtx_ucs_jch_to_hcb_0 = s_bwdCallableCtx_ucs_jch_to_hcb_0( _S1431 );
    var _S1433 : f32 = _S1430.value0_78.y;
    var _S1434 : f32 = _S1430.value0_78.z;
    var _S1435 : f32 = _S1433 * _S1433 + _S1434 * _S1434;
    var _S1436 : Tuple_3 = s_apply_sqrt_0(_S1435);
    var _S1437 : s_bwdCallableCtx_sqrt_0;
    _S1437._S24 = _S1435;
    var _S1438 : s_bwdCallableCtx_sqrt_0 = _S1437;
    var _S1439 : bool = ((_S1436.value0_3) > 0.0f);
    var sin_t_0 : f32;
    var cos_t_0 : f32;
    var _S1440 : f32;
    if(_S1439)
    {
        var _S1441 : f32 = _S1436.value0_3 * _S1436.value0_3;
        var _S1442 : f32 = _S1434 / _S1436.value0_3;
        sin_t_0 = _S1433 / _S1436.value0_3;
        cos_t_0 = _S1442;
        _S1440 = _S1441;
    }
    else
    {
        sin_t_0 = 0.0f;
        cos_t_0 = 0.0f;
        _S1440 = 0.0f;
    }
    var _S1443 : Tuple_17 = s_apply_max_1(1.00000000362749373e-15f, _S1433);
    var _S1444 : s_bwdCallableCtx_max_1;
    _S1444._S109 = 1.00000000362749373e-15f;
    _S1444._S110 = _S1433;
    var _S1445 : s_bwdCallableCtx_max_1 = _S1444;
    var w_0 : f32 = sin_t_0 * _S1433 + cos_t_0 * _S1434;
    var _S1446 : f32 = 1.0f + _S1419.value0_101.value3_1.saturation_global_1;
    var _S1447 : vec3<f32> = _S1419.value0_101.value2_2.saturation_0.xyz;
    var _S1448 : Tuple_14 = s_apply_dot_1(_S1419.value0_101.value1_12, _S1447);
    var _S1449 : s_bwdCallableCtx_dot_1;
    _S1449._S98 = _S1419.value0_101.value1_12;
    _S1449._S99 = _S1447;
    var _S1450 : s_bwdCallableCtx_dot_1 = _S1449;
    var _S1451 : f32 = _S1446 + _S1448.value0_16;
    var _S1452 : Tuple_17 = s_apply_max_1(_S1451, 0.0f);
    var _S1453 : s_bwdCallableCtx_max_1;
    _S1453._S109 = _S1451;
    _S1453._S110 = 0.0f;
    var _S1454 : s_bwdCallableCtx_max_1 = _S1453;
    var _S1455 : f32 = 1.0f + _S1419.value0_101.value3_1.brilliance_global_1;
    var _S1456 : vec3<f32> = _S1419.value0_101.value2_2.brilliance_0.xyz;
    var _S1457 : Tuple_14 = s_apply_dot_1(_S1419.value0_101.value1_12, _S1456);
    var _S1458 : s_bwdCallableCtx_dot_1;
    _S1458._S98 = _S1419.value0_101.value1_12;
    _S1458._S99 = _S1456;
    var _S1459 : s_bwdCallableCtx_dot_1 = _S1458;
    var _S1460 : f32 = _S1455 + _S1457.value0_16;
    var _S1461 : Tuple_17 = s_apply_max_1(_S1460, 0.0f);
    var _S1462 : s_bwdCallableCtx_max_1;
    _S1462._S109 = _S1460;
    _S1462._S110 = 0.0f;
    var _S1463 : s_bwdCallableCtx_max_1 = _S1462;
    var _S1464 : f32 = _S1443.value0_19 * _S1443.value0_19;
    var _S1465 : f32 = w_0 * w_0;
    var _S1466 : f32 = _S1464 + _S1465;
    var _S1467 : Tuple_3 = s_apply_sqrt_0(_S1466);
    var _S1468 : s_bwdCallableCtx_sqrt_0;
    _S1468._S24 = _S1466;
    var _S1469 : s_bwdCallableCtx_sqrt_0 = _S1468;
    var max_a_0 : f32 = _S1467.value0_3 / _S1443.value0_19;
    var _S1470 : f32 = 0.5f * max_a_0;
    var _S1471 : Tuple_32 = s_apply_soft_clip_0(_S1452.value0_19, _S1470, max_a_0);
    var _S1472 : s_paramCtx_s_bwdCallableCtx_soft_clip_0;
    _S1472.value0_52 = _S1452.value0_19;
    _S1472.value1_6 = _S1470;
    _S1472.value2_0 = max_a_0;
    var _S1473 : s_bwdCallableCtx_soft_clip_0 = s_bwdCallableCtx_soft_clip_0( _S1472 );
    var _S1474 : f32 = _S1471.value0_51 - 1.0f;
    var p_prime_0 : f32 = _S1474 * _S1443.value0_19;
    var _S1475 : f32 = 1.0f - _S1471.value0_51 * _S1471.value0_51;
    var _S1476 : f32 = _S1464 * _S1475 + _S1465;
    var _S1477 : Tuple_17 = s_apply_max_1(_S1476, 0.0f);
    var _S1478 : s_bwdCallableCtx_max_1;
    _S1478._S109 = _S1476;
    _S1478._S110 = 0.0f;
    var _S1479 : s_bwdCallableCtx_max_1 = _S1478;
    var _S1480 : Tuple_3 = s_apply_sqrt_0(_S1477.value0_19);
    var _S1481 : s_bwdCallableCtx_sqrt_0;
    _S1481._S24 = _S1477.value0_19;
    var _S1482 : s_bwdCallableCtx_sqrt_0 = _S1481;
    var w_prime_0 : f32 = _S1480.value0_3 * _S1461.value0_19;
    var _S1483 : f32 = cos_t_0 * p_prime_0 + sin_t_0 * w_prime_0;
    var _S1484 : Tuple_17 = s_apply_max_1(_S1483, 0.0f);
    var _S1485 : s_bwdCallableCtx_max_1;
    _S1485._S109 = _S1483;
    _S1485._S110 = 0.0f;
    var _S1486 : s_bwdCallableCtx_max_1 = _S1485;
    var _S1487 : vec3<f32> = _S1430.value0_78;
    _S1487[i32(1)] = _S1484.value0_19;
    var _S1488 : f32 = - sin_t_0;
    var _S1489 : f32 = _S1488 * p_prime_0 + cos_t_0 * w_prime_0;
    var _S1490 : Tuple_17 = s_apply_max_1(_S1489, 0.0f);
    var _S1491 : s_bwdCallableCtx_max_1;
    _S1491._S109 = _S1489;
    _S1491._S110 = 0.0f;
    var _S1492 : s_bwdCallableCtx_max_1 = _S1491;
    _S1487[i32(2)] = _S1490.value0_19;
    var _S1493 : Tuple_41 = s_apply_ucs_hcb_to_jch_0(_S1487);
    var _S1494 : s_paramCtx_s_bwdCallableCtx_ucs_hcb_to_jch_0;
    _S1494.value0_82 = _S1487;
    var _S1495 : s_bwdCallableCtx_ucs_hcb_to_jch_0 = s_bwdCallableCtx_ucs_hcb_to_jch_0( _S1494 );
    var _S1496 : f32 = _S1493.value0_81.z;
    var _S1497 : s_bwdCallableCtx_lookup_gamut_0 = s_bwdCallableCtx_lookup_gamut_0( _S1419.value1_14.value1_13 );
    var _S1498 : f32 = _S1493.value0_81.x;
    var _S1499 : f32 = _S1498 * _S1419.value0_101.value2_2.l_white_0;
    var _S1500 : Tuple_16 = s_apply_pow0_0(_S1499, 0.65239977836608887f);
    var _S1501 : s_bwdCallableCtx_pow0_0;
    _S1501._S104 = _S1499;
    _S1501._S105 = 0.65239977836608887f;
    var _S1502 : s_bwdCallableCtx_pow0_0 = _S1501;
    var _S1503 : f32 = 15.93299388885498f * _S1500.value0_18;
    var _S1504 : Tuple_16 = s_apply_pow0_0(_S1419.value1_14.value0_100, 0.60075569152832031f);
    var _S1505 : s_bwdCallableCtx_pow0_0;
    _S1505._S104 = _S1419.value1_14.value0_100;
    _S1505._S105 = 0.60075569152832031f;
    var _S1506 : s_bwdCallableCtx_pow0_0 = _S1505;
    var _S1507 : vec3<f32> = vec3<f32>(_S1498, _S1503 * _S1504.value0_18 / _S1419.value0_101.value2_2.l_white_0, _S1496);
    var _S1508 : Tuple_42 = s_apply_ucs_jch_to_hsb_0(_S1507);
    var _S1509 : s_paramCtx_s_bwdCallableCtx_ucs_jch_to_hsb_0;
    _S1509.value0_85 = _S1507;
    var _S1510 : s_bwdCallableCtx_ucs_jch_to_hsb_0 = s_bwdCallableCtx_ucs_jch_to_hsb_0( _S1509 );
    var _S1511 : f32 = _S1487.z;
    var _S1512 : bool = (_S1511 > 0.0f);
    var saturation_2 : f32;
    var _S1513 : f32;
    var _S1514 : f32;
    if(_S1512)
    {
        var _S1515 : f32 = _S1487.y;
        var _S1516 : f32 = _S1511 * _S1511;
        saturation_2 = _S1515 / _S1511;
        _S1513 = _S1516;
        _S1514 = _S1515;
    }
    else
    {
        saturation_2 = 0.0f;
        _S1513 = 0.0f;
        _S1514 = 0.0f;
    }
    var _S1517 : f32 = _S1508.value0_84.y;
    var _S1518 : f32 = 0.80000001192092896f * _S1517;
    var _S1519 : Tuple_32 = s_apply_soft_clip_0(saturation_2, _S1518, _S1517);
    var _S1520 : s_paramCtx_s_bwdCallableCtx_soft_clip_0;
    _S1520.value0_52 = saturation_2;
    _S1520.value1_6 = _S1518;
    _S1520.value2_0 = _S1517;
    var _S1521 : s_bwdCallableCtx_soft_clip_0 = s_bwdCallableCtx_soft_clip_0( _S1520 );
    var _S1522 : vec3<f32> = vec3<f32>(_S1487.x, _S1519.value0_51, _S1511);
    var _S1523 : Tuple_43 = s_apply_ucs_hsb_to_jch_0(_S1522);
    var _S1524 : s_paramCtx_s_bwdCallableCtx_ucs_hsb_to_jch_0;
    _S1524.value0_88 = _S1522;
    var _S1525 : s_bwdCallableCtx_ucs_hsb_to_jch_0 = s_bwdCallableCtx_ucs_hsb_to_jch_0( _S1524 );
    var _S1526 : Tuple_45 = s_apply_ucs_jch_to_xyy_0(_S1523.value0_87, _S1419.value0_101.value2_2.l_white_0);
    var _S1527 : s_paramCtx_s_bwdCallableCtx_ucs_jch_to_xyy_0;
    _S1527.value0_94 = _S1523.value0_87;
    _S1527.value1_11 = _S1419.value0_101.value2_2.l_white_0;
    var _S1528 : s_bwdCallableCtx_ucs_jch_to_xyy_0 = s_bwdCallableCtx_ucs_jch_to_xyy_0( _S1527 );
    var _S1529 : s_paramCtx_s_bwdCallableCtx_xyy_to_xyz_0;
    _S1529.value0_97 = _S1526.value0_93;
    var _S1530 : s_bwdCallableCtx_xyy_to_xyz_0 = s_bwdCallableCtx_xyy_to_xyz_0( _S1529 );
    var _S1531 : Grading_0 = Grading_x24_syn_dzero_0();
    var _S1532 : vec3<f32> = vec3<f32>(0.0f);
    var _S1533 : vec3<f32> = _S1532;
    s_bwdProp_xyy_to_xyz_0(_S1530, &(_S1533), _S1423);
    var _S1534 : vec3<f32> = _S1532;
    s_bwdProp_ucs_jch_to_xyy_0(_S1528, &(_S1534), _S1533);
    var _S1535 : vec3<f32> = _S1532;
    s_bwdProp_ucs_hsb_to_jch_0(_S1525, &(_S1535), _S1534);
    var _S1536 : vec3<f32> = _S1535;
    var _S1537 : f32 = 0.0f;
    var _S1538 : f32 = 0.0f;
    var _S1539 : f32 = 0.0f;
    s_bwdProp_soft_clip_0(_S1521, &(_S1537), &(_S1538), &(_S1539), _S1535[i32(1)]);
    var _S1540 : f32 = _S1537;
    var _S1541 : vec3<f32> = vec3<f32>(_S1535[i32(0)], 0.0f, 0.0f);
    var _S1542 : vec3<f32> = vec3<f32>(0.0f, _S1539 + 0.80000001192092896f * _S1538, 0.0f);
    if(_S1512)
    {
        var _S1543 : f32 = _S1540 / _S1513;
        var _S1544 : vec3<f32> = _S1541 + vec3<f32>(0.0f, _S1511 * _S1543, 0.0f);
        saturation_2 = _S1514 * - _S1543 + _S1536[i32(2)];
        _S1487 = _S1544;
    }
    else
    {
        saturation_2 = _S1536[i32(2)];
        _S1487 = _S1541;
    }
    var _S1545 : vec3<f32> = _S1532;
    s_bwdProp_ucs_jch_to_hsb_0(_S1510, &(_S1545), _S1542);
    var _S1546 : f32 = _S1545[i32(1)] / _S1419.value0_101.value2_2.l_white_0;
    var _S1547 : f32 = _S1503 * _S1546;
    var _S1548 : f32 = _S1504.value0_18 * _S1546;
    var _S1549 : f32 = 0.0f;
    var _S1550 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S1506, &(_S1549), &(_S1550), _S1547);
    var _S1551 : f32 = 15.93299388885498f * _S1548;
    var _S1552 : f32 = 0.0f;
    var _S1553 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S1502, &(_S1552), &(_S1553), _S1551);
    var _S1554 : f32 = _S1545[i32(0)] + _S1419.value0_101.value2_2.l_white_0 * _S1552;
    var _S1555 : f32 = 0.0f;
    s_bwdProp_lookup_gamut_0(_S1497, &(_S1555), _S1549);
    var _S1556 : vec3<f32> = vec3<f32>(_S1554, 0.0f, _S1545[i32(2)] + _S1555);
    var _S1557 : vec3<f32> = _S1532;
    s_bwdProp_ucs_hcb_to_jch_0(_S1495, &(_S1557), _S1556);
    var _S1558 : vec3<f32> = _S1557 + _S1487 + vec3<f32>(0.0f, 0.0f, saturation_2);
    _S1487 = _S1558;
    _S1487[i32(2)] = 0.0f;
    var _S1559 : f32 = 0.0f;
    var _S1560 : f32 = 0.0f;
    s_bwdProp_max_1(_S1492, &(_S1559), &(_S1560), _S1558[i32(2)]);
    var _S1561 : f32 = cos_t_0 * _S1559;
    var _S1562 : f32 = w_prime_0 * _S1559;
    var _S1563 : f32 = _S1488 * _S1559;
    var _S1564 : f32 = - (p_prime_0 * _S1559);
    _S1487[i32(1)] = 0.0f;
    var _S1565 : f32 = 0.0f;
    var _S1566 : f32 = 0.0f;
    s_bwdProp_max_1(_S1486, &(_S1565), &(_S1566), _S1558[i32(1)]);
    var _S1567 : f32 = w_prime_0 * _S1565;
    var _S1568 : f32 = cos_t_0 * _S1565;
    var _S1569 : f32 = p_prime_0 * _S1565;
    var _S1570 : f32 = _S1561 + sin_t_0 * _S1565;
    var _S1571 : f32 = _S1480.value0_3 * _S1570;
    var _S1572 : f32 = _S1461.value0_19 * _S1570;
    var _S1573 : f32 = 0.0f;
    s_bwdProp_sqrt_0(_S1482, &(_S1573), _S1572);
    var _S1574 : f32 = 0.0f;
    var _S1575 : f32 = 0.0f;
    s_bwdProp_max_1(_S1479, &(_S1574), &(_S1575), _S1573);
    var _S1576 : f32 = _S1475 * _S1574;
    var _S1577 : f32 = _S1471.value0_51 * - (_S1464 * _S1574);
    var _S1578 : f32 = _S1563 + _S1568;
    var _S1579 : f32 = _S1474 * _S1578;
    var _S1580 : f32 = _S1577 + _S1577 + _S1443.value0_19 * _S1578;
    var _S1581 : f32 = 0.0f;
    var _S1582 : f32 = 0.0f;
    var _S1583 : f32 = 0.0f;
    s_bwdProp_soft_clip_0(_S1473, &(_S1581), &(_S1582), &(_S1583), _S1580);
    var _S1584 : f32 = (_S1583 + 0.5f * _S1582) / _S1464;
    var _S1585 : f32 = _S1467.value0_3 * - _S1584;
    var _S1586 : f32 = _S1443.value0_19 * _S1584;
    var _S1587 : f32 = 0.0f;
    s_bwdProp_sqrt_0(_S1469, &(_S1587), _S1586);
    var _S1588 : f32 = w_0 * (_S1574 + _S1587);
    var _S1589 : f32 = _S1443.value0_19 * (_S1576 + _S1587);
    var _S1590 : f32 = 0.0f;
    var _S1591 : f32 = 0.0f;
    s_bwdProp_max_1(_S1463, &(_S1590), &(_S1591), _S1571);
    var _S1592 : vec3<f32> = _S1532;
    var _S1593 : vec3<f32> = _S1532;
    s_bwdProp_dot_1(_S1459, &(_S1592), &(_S1593), _S1590);
    var _S1594 : f32 = 0.0f;
    var _S1595 : f32 = 0.0f;
    s_bwdProp_max_1(_S1454, &(_S1594), &(_S1595), _S1581);
    var _S1596 : vec3<f32> = _S1532;
    var _S1597 : vec3<f32> = _S1532;
    s_bwdProp_dot_1(_S1450, &(_S1596), &(_S1597), _S1594);
    var _S1598 : f32 = _S1588 + _S1588;
    var _S1599 : f32 = cos_t_0 * _S1598;
    var _S1600 : f32 = _S1434 * _S1598;
    var _S1601 : f32 = sin_t_0 * _S1598;
    var _S1602 : f32 = _S1433 * _S1598;
    var _S1603 : f32 = _S1579 + _S1585 + _S1589 + _S1589;
    var _S1604 : f32 = 0.0f;
    var _S1605 : f32 = 0.0f;
    s_bwdProp_max_1(_S1445, &(_S1604), &(_S1605), _S1603);
    var _S1606 : f32 = _S1564 + _S1567 + _S1602;
    var _S1607 : f32 = _S1562 + _S1569 + _S1600;
    var _S1608 : Grading_0 = _S1531;
    _S1608.brilliance_global_1 = _S1590;
    _S1608.saturation_global_1 = _S1594;
    var _S1609 : Grading_0 = Grading_x24_syn_dadd_0(_S1531, _S1608);
    var _S1610 : f32 = _S1601 + _S1605;
    var _S1611 : vec3<f32> = _S1592 + _S1596;
    if(_S1439)
    {
        var _S1612 : f32 = _S1607 / _S1440;
        var _S1613 : f32 = _S1606 / _S1440;
        var _S1614 : f32 = _S1436.value0_3 * _S1612 + _S1599;
        var _S1615 : f32 = _S1436.value0_3 * _S1613 + _S1610;
        sin_t_0 = _S1434 * - _S1612 + _S1433 * - _S1613;
        cos_t_0 = _S1614;
        _S1440 = _S1615;
    }
    else
    {
        sin_t_0 = 0.0f;
        cos_t_0 = _S1599;
        _S1440 = _S1610;
    }
    var _S1616 : f32 = 0.0f;
    s_bwdProp_sqrt_0(_S1438, &(_S1616), sin_t_0);
    var _S1617 : f32 = _S1434 * _S1616;
    var _S1618 : f32 = _S1433 * _S1616;
    var _S1619 : vec3<f32> = _S1487 + vec3<f32>(0.0f, _S1618 + _S1618 + _S1440, _S1617 + _S1617 + cos_t_0);
    var _S1620 : vec3<f32> = _S1532;
    s_bwdProp_ucs_jch_to_hcb_0(_S1432, &(_S1620), _S1619);
    var _S1621 : vec3<f32> = _S1532;
    s_bwdProp_xyy_to_ucs_jch_0(_S1429, &(_S1621), _S1620);
    var _S1622 : vec3<f32> = _S1532;
    s_bwdProp_d65_xyz_to_xyy_0(_S1426, &(_S1622), _S1621);
    (*_S1420) = _S1622;
    (*_S1421) = _S1611;
    (*_S1422) = _S1609;
    return;
}

struct s_paramCtx_s_bwdCallableCtx_color_balance_0
{
     value0_102 : vec3<f32>,
     value1_15 : ColorBalanceParams_0,
     value2_3 : Grading_0,
};

struct s_minCtx_s_bwdCallableCtx_color_balance_0
{
     value0_103 : vec3<f32>,
     value1_16 : s_minCtx_s_bwdCallableCtx_jzazbz_saturation_0,
     value2_4 : vec3<f32>,
     value3_2 : s_minCtx_s_bwdCallableCtx_ucs_saturation_0,
};

struct s_bwdCallableCtx_color_balance_0
{
     value0_104 : s_paramCtx_s_bwdCallableCtx_color_balance_0,
     value1_17 : s_minCtx_s_bwdCallableCtx_color_balance_0,
};

fn s_bwdProp_color_balance_0( _S1623 : s_bwdCallableCtx_color_balance_0,  _S1624 : ptr<function, Grading_0>,  _S1625 : vec3<f32>)
{
    var _S1626 : vec3<f32> = vec3<f32>(0.0f);
    var _S1627 : vec4<f32> = vec4<f32>(0.0f);
    var _S1628 : ColorBalanceParams_0 = ColorBalanceParams_0( _S1627, _S1627, _S1627, _S1627, _S1627, _S1627, _S1627, _S1627, _S1627, _S1627, _S1627, _S1627, _S1627, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, u32(0) );
    var _S1629 : Grading_0 = Grading_0( 0.0f, 0.0f, 0.0f, 0.0f );
    var _S1630 : s_paramCtx_s_bwdCallableCtx_jzazbz_saturation_0 = s_paramCtx_s_bwdCallableCtx_jzazbz_saturation_0( _S1626, _S1626, _S1628, _S1629 );
    var _S1631 : s_minCtx_s_bwdCallableCtx_lookup_gamut_0 = s_minCtx_s_bwdCallableCtx_lookup_gamut_0( 0.0f, 0.0f );
    var _S1632 : s_minCtx_s_bwdCallableCtx_jzazbz_saturation_0 = s_minCtx_s_bwdCallableCtx_jzazbz_saturation_0( 0.0f, _S1631 );
    var _S1633 : s_paramCtx_s_bwdCallableCtx_ucs_saturation_0 = s_paramCtx_s_bwdCallableCtx_ucs_saturation_0( _S1626, _S1626, _S1628, _S1629 );
    var _S1634 : s_minCtx_s_bwdCallableCtx_ucs_saturation_0 = s_minCtx_s_bwdCallableCtx_ucs_saturation_0( 0.0f, _S1631 );
    var output_matrix_0 : mat3x3<f32> = mat3x3<f32>(_S1623.value0_104.value1_15.output_r0_0.xyz, _S1623.value0_104.value1_15.output_r1_0.xyz, _S1623.value0_104.value1_15.output_r2_0.xyz);
    var _S1635 : vec3<f32> = vec3<f32>(0.0f);
    var _S1636 : Tuple_21 = s_apply_yrg_to_ych_0(s_apply_lms_to_yrg_0(s_apply_mul_0(mat3x3<f32>(_S1623.value0_104.value1_15.input_r0_0.xyz, _S1623.value0_104.value1_15.input_r1_0.xyz, _S1623.value0_104.value1_15.input_r2_0.xyz), s_apply_max_2(_S1623.value0_104.value0_102, _S1635).value0_21).value0_20).value0_22);
    var _S1637 : Tuple_17 = s_apply_max_1(_S1636.value0_25.x, 0.0f);
    var _S1638 : vec4<f32> = _S1636.value0_25;
    _S1638[i32(0)] = _S1637.value0_19;
    var _S1639 : f32 = _S1638.x;
    var _S1640 : Tuple_23 = s_apply_opacity_masks_0(s_apply_pow0_0(_S1639, 0.41012057662010193f).value0_18, _S1623.value0_104.value1_15);
    var _S1641 : f32 = _S1638.y;
    var _S1642 : f32 = _S1638.z;
    var _S1643 : f32 = _S1638.w;
    var ych_2 : vec4<f32> = vec4<f32>(_S1639, _S1641, _S1623.value0_104.value1_15.hue_cos_0 * _S1642 - _S1623.value0_104.value1_15.hue_sin_0 * _S1643, _S1623.value0_104.value1_15.hue_sin_0 * _S1642 + _S1623.value0_104.value1_15.hue_cos_0 * _S1643);
    var vibrance_1 : f32;
    if((_S1623.value0_104.value1_15.vibrance_0) != 0.0f)
    {
        vibrance_1 = _S1623.value0_104.value1_15.vibrance_0 * (1.0f - s_apply_pow0_0(_S1641, s_apply_abs_2(_S1623.value0_104.value1_15.vibrance_0).value0_17).value0_18);
    }
    else
    {
        vibrance_1 = 0.0f;
    }
    var chroma_factor_0 : f32 = 1.0f + _S1623.value0_104.value2_3.chroma_global_1 + s_apply_dot_1(_S1640.value0_27, _S1623.value0_104.value1_15.chroma_0.xyz).value0_16 + vibrance_1;
    var _S1644 : Tuple_17 = s_apply_max_1(chroma_factor_0, 0.0f);
    var _S1645 : s_bwdCallableCtx_max_1;
    _S1645._S109 = chroma_factor_0;
    _S1645._S110 = 0.0f;
    var _S1646 : s_bwdCallableCtx_max_1 = _S1645;
    var _S1647 : f32 = _S1641 * _S1644.value0_19;
    _S1638 = ych_2;
    _S1638[i32(1)] = _S1647;
    var _S1648 : Tuple_25 = s_apply_gamut_check_yrg_0(_S1638);
    var _S1649 : s_paramCtx_s_bwdCallableCtx_gamut_check_yrg_0;
    _S1649.value0_32 = _S1638;
    var _S1650 : s_bwdCallableCtx_gamut_check_yrg_0 = s_bwdCallableCtx_gamut_check_yrg_0( _S1649 );
    var _S1651 : Tuple_24 = s_apply_ych_to_yrg_0(_S1648.value0_31);
    var _S1652 : s_paramCtx_s_bwdCallableCtx_ych_to_yrg_0;
    _S1652.value0_29 = _S1648.value0_31;
    var _S1653 : s_bwdCallableCtx_ych_to_yrg_0 = s_bwdCallableCtx_ych_to_yrg_0( _S1652 );
    var _S1654 : Tuple_26 = s_apply_yrg_to_lms_0(_S1651.value0_28);
    var _S1655 : s_paramCtx_s_bwdCallableCtx_yrg_to_lms_0;
    _S1655.value0_35 = _S1651.value0_28;
    var _S1656 : s_bwdCallableCtx_yrg_to_lms_0 = s_bwdCallableCtx_yrg_to_lms_0( _S1655 );
    var _S1657 : Tuple_18 = s_apply_mul_0(mat3x3<f32>(1.0877193212509155f, -0.66666668653488159f, 0.020618559792637825f, -0.087719298899173737f, 1.6666666269302368f, -0.051546391099691391f, 0.0f, 0.0f, 1.0309278964996338f), _S1654.value0_34);
    var _S1658 : s_bwdCallableCtx_mul_0;
    _S1658._S114 = mat3x3<f32>(1.0877193212509155f, -0.66666668653488159f, 0.020618559792637825f, -0.087719298899173737f, 1.6666666269302368f, -0.051546391099691391f, 0.0f, 0.0f, 1.0309278964996338f);
    _S1658._S115 = _S1654.value0_34;
    var _S1659 : s_bwdCallableCtx_mul_0 = _S1658;
    var _S1660 : f32 = _S1640.value0_27.z;
    var _S1661 : f32 = _S1640.value0_27.x;
    var _S1662 : vec3<f32> = vec3<f32>((1.0f - _S1660)) * (vec3<f32>((1.0f - _S1661)) + vec3<f32>(_S1661) * _S1623.value0_104.value1_15.shadows_0.xyz) + vec3<f32>(_S1660) * _S1623.value0_104.value1_15.highlights_0.xyz;
    var rgb_1 : vec3<f32> = (_S1657.value0_20 + _S1623.value0_104.value1_15.global_0.xyz) * _S1662;
    if(((rgb_1.x) < 0.0f))
    {
        vibrance_1 = -1.0f;
    }
    else
    {
        vibrance_1 = 1.0f;
    }
    var _S1663 : f32;
    if(((rgb_1.y) < 0.0f))
    {
        _S1663 = -1.0f;
    }
    else
    {
        _S1663 = 1.0f;
    }
    var _S1664 : f32;
    if(((rgb_1.z) < 0.0f))
    {
        _S1664 = -1.0f;
    }
    else
    {
        _S1664 = 1.0f;
    }
    var sign_0 : vec3<f32> = vec3<f32>(vibrance_1, _S1663, _S1664);
    var _S1665 : Tuple_13 = s_apply_abs_1(rgb_1);
    var _S1666 : s_bwdCallableCtx_abs_1;
    _S1666._S93 = rgb_1;
    var _S1667 : s_bwdCallableCtx_abs_1 = _S1666;
    var _S1668 : vec3<f32> = vec3<f32>(_S1623.value0_104.value1_15.white_fulcrum_0);
    var _S1669 : vec3<f32> = _S1665.value0_15 / _S1668;
    var _S1670 : vec3<f32> = _S1623.value0_104.value1_15.midtones_0.xyz;
    var _S1671 : Tuple_28 = s_apply_bounded_pow_1(_S1669, _S1670);
    var _S1672 : s_paramCtx_s_bwdCallableCtx_bounded_pow_1;
    _S1672.value0_41 = _S1669;
    _S1672.value1_1 = _S1670;
    var _S1673 : s_bwdCallableCtx_bounded_pow_1 = s_bwdCallableCtx_bounded_pow_1( _S1672 );
    var rgb_2 : vec3<f32> = sign_0 * _S1671.value0_40 * _S1668;
    var _S1674 : Tuple_18 = s_apply_mul_0(mat3x3<f32>(0.94999998807907104f, 0.37999999523162842f, 0.0f, 0.05000000074505806f, 0.62000000476837158f, 0.029999999329447746f, 0.0f, 0.0f, 0.97000002861022949f), rgb_2);
    var _S1675 : s_bwdCallableCtx_mul_0;
    _S1675._S114 = mat3x3<f32>(0.94999998807907104f, 0.37999999523162842f, 0.0f, 0.05000000074505806f, 0.62000000476837158f, 0.029999999329447746f, 0.0f, 0.0f, 0.97000002861022949f);
    _S1675._S115 = rgb_2;
    var _S1676 : s_bwdCallableCtx_mul_0 = _S1675;
    var _S1677 : Tuple_20 = s_apply_lms_to_yrg_0(_S1674.value0_20);
    var _S1678 : s_paramCtx_s_bwdCallableCtx_lms_to_yrg_0;
    _S1678.value0_23 = _S1674.value0_20;
    var _S1679 : s_bwdCallableCtx_lms_to_yrg_0 = s_bwdCallableCtx_lms_to_yrg_0( _S1678 );
    var _S1680 : f32 = _S1677.value0_22.x / _S1623.value0_104.value1_15.white_fulcrum_0;
    var _S1681 : Tuple_17 = s_apply_max_1(_S1680, 0.0f);
    var _S1682 : s_bwdCallableCtx_max_1;
    _S1682._S109 = _S1680;
    _S1682._S110 = 0.0f;
    var _S1683 : s_bwdCallableCtx_max_1 = _S1682;
    var _S1684 : Tuple_27 = s_apply_bounded_pow_0(_S1681.value0_19, _S1623.value0_104.value1_15.midtones_y_0);
    var _S1685 : s_paramCtx_s_bwdCallableCtx_bounded_pow_0;
    _S1685.value0_38 = _S1681.value0_19;
    _S1685.value1_0 = _S1623.value0_104.value1_15.midtones_y_0;
    var _S1686 : s_bwdCallableCtx_bounded_pow_0 = s_bwdCallableCtx_bounded_pow_0( _S1685 );
    var _S1687 : f32 = _S1684.value0_37 * _S1623.value0_104.value1_15.white_fulcrum_0;
    var xyz_2 : vec3<f32> = _S1677.value0_22;
    xyz_2[i32(0)] = _S1687;
    var _S1688 : f32 = xyz_2.x / _S1623.value0_104.value1_15.grey_fulcrum_0;
    var _S1689 : Tuple_27 = s_apply_bounded_pow_0(_S1688, _S1623.value0_104.value2_3.contrast_1);
    var _S1690 : s_paramCtx_s_bwdCallableCtx_bounded_pow_0;
    _S1690.value0_38 = _S1688;
    _S1690.value1_0 = _S1623.value0_104.value2_3.contrast_1;
    var _S1691 : s_bwdCallableCtx_bounded_pow_0 = s_bwdCallableCtx_bounded_pow_0( _S1690 );
    xyz_2[i32(0)] = _S1623.value0_104.value1_15.grey_fulcrum_0 * _S1689.value0_37;
    var _S1692 : Tuple_26 = s_apply_yrg_to_lms_0(xyz_2);
    var _S1693 : s_paramCtx_s_bwdCallableCtx_yrg_to_lms_0;
    _S1693.value0_35 = xyz_2;
    var _S1694 : s_bwdCallableCtx_yrg_to_lms_0 = s_bwdCallableCtx_yrg_to_lms_0( _S1693 );
    var _S1695 : Tuple_18 = s_apply_mul_0(mat3x3<f32>(1.8079465627670288f, -1.299716591835022f, 0.34785878658294678f, 0.61783957481384277f, 0.39595451951026917f, -0.0410468690097332f, -0.12546959519386292f, 0.20478038489818573f, 1.7427418231964111f), _S1692.value0_34);
    var _S1696 : s_bwdCallableCtx_mul_0;
    _S1696._S114 = mat3x3<f32>(1.8079465627670288f, -1.299716591835022f, 0.34785878658294678f, 0.61783957481384277f, 0.39595451951026917f, -0.0410468690097332f, -0.12546959519386292f, 0.20478038489818573f, 1.7427418231964111f);
    _S1696._S115 = _S1692.value0_34;
    var _S1697 : s_bwdCallableCtx_mul_0 = _S1696;
    var _S1698 : bool = (_S1623.value0_104.value1_15.saturation_formula_0) == u32(0);
    var _S1699 : s_bwdCallableCtx_jzazbz_saturation_0;
    var _S1700 : s_bwdCallableCtx_ucs_saturation_0;
    if(_S1698)
    {
        var _S1701 : s_paramCtx_s_bwdCallableCtx_jzazbz_saturation_0;
        _S1701.value0_60 = _S1695.value0_20;
        _S1701.value1_7 = _S1640.value0_27;
        _S1701.value2_1 = _S1623.value0_104.value1_15;
        _S1701.value3_0 = _S1623.value0_104.value2_3;
        xyz_2 = _S1623.value1_17.value0_103;
        _S1699.value0_62 = _S1701;
        _S1699.value1_9 = _S1623.value1_17.value1_16;
        _S1700.value0_101 = _S1633;
        _S1700.value1_14 = _S1634;
    }
    else
    {
        var _S1702 : s_paramCtx_s_bwdCallableCtx_ucs_saturation_0;
        _S1702.value0_99 = _S1695.value0_20;
        _S1702.value1_12 = _S1640.value0_27;
        _S1702.value2_2 = _S1623.value0_104.value1_15;
        _S1702.value3_1 = _S1623.value0_104.value2_3;
        xyz_2 = _S1623.value1_17.value2_4;
        _S1699.value0_62 = _S1630;
        _S1699.value1_9 = _S1632;
        _S1700.value0_101 = _S1702;
        _S1700.value1_14 = _S1623.value1_17.value3_2;
    }
    var _S1703 : Tuple_18 = s_apply_mul_0(output_matrix_0, xyz_2);
    var _S1704 : s_bwdCallableCtx_mul_0;
    _S1704._S114 = output_matrix_0;
    _S1704._S115 = xyz_2;
    var _S1705 : s_bwdCallableCtx_mul_0 = _S1704;
    var _S1706 : s_bwdCallableCtx_max_2;
    _S1706._S122 = _S1703.value0_20;
    _S1706._S123 = _S1635;
    var _S1707 : s_bwdCallableCtx_max_2 = _S1706;
    var _S1708 : Grading_0 = Grading_x24_syn_dzero_0();
    var _S1709 : vec3<f32> = _S1626;
    var _S1710 : vec3<f32> = _S1626;
    s_bwdProp_max_2(_S1707, &(_S1709), &(_S1710), _S1625);
    var _S1711 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S1712 : mat3x3<f32> = _S1711;
    var _S1713 : vec3<f32> = _S1626;
    s_bwdProp_mul_0(_S1705, &(_S1712), &(_S1713), _S1709);
    var _S1714 : vec3<f32> = _S1713;
    var _S1715 : Grading_0;
    if(_S1698)
    {
        var _S1716 : vec3<f32> = _S1626;
        var _S1717 : vec3<f32> = _S1626;
        var _S1718 : Grading_0;
        _S1718.chroma_global_1 = 0.0f;
        _S1718.contrast_1 = 0.0f;
        _S1718.saturation_global_1 = 0.0f;
        _S1718.brilliance_global_1 = 0.0f;
        s_bwdProp_jzazbz_saturation_0(_S1699, &(_S1716), &(_S1717), &(_S1718), _S1714);
        var _S1719 : Grading_0 = Grading_x24_syn_dadd_0(_S1718, _S1708);
        xyz_2 = _S1716;
        _S1715 = _S1719;
    }
    else
    {
        var _S1720 : vec3<f32> = _S1626;
        var _S1721 : vec3<f32> = _S1626;
        var _S1722 : Grading_0;
        _S1722.chroma_global_1 = 0.0f;
        _S1722.contrast_1 = 0.0f;
        _S1722.saturation_global_1 = 0.0f;
        _S1722.brilliance_global_1 = 0.0f;
        s_bwdProp_ucs_saturation_0(_S1700, &(_S1720), &(_S1721), &(_S1722), _S1714);
        var _S1723 : Grading_0 = Grading_x24_syn_dadd_0(_S1722, _S1708);
        xyz_2 = _S1720;
        _S1715 = _S1723;
    }
    var _S1724 : mat3x3<f32> = _S1711;
    var _S1725 : vec3<f32> = _S1626;
    s_bwdProp_mul_0(_S1697, &(_S1724), &(_S1725), xyz_2);
    var _S1726 : vec3<f32> = _S1626;
    s_bwdProp_yrg_to_lms_0(_S1694, &(_S1726), _S1725);
    xyz_2 = _S1726;
    xyz_2[i32(0)] = 0.0f;
    var _S1727 : f32 = _S1623.value0_104.value1_15.grey_fulcrum_0 * _S1726[i32(0)];
    var _S1728 : f32 = 0.0f;
    var _S1729 : f32 = 0.0f;
    s_bwdProp_bounded_pow_0(_S1691, &(_S1728), &(_S1729), _S1727);
    var _S1730 : vec3<f32> = xyz_2 + vec3<f32>(_S1728 / _S1623.value0_104.value1_15.grey_fulcrum_0, 0.0f, 0.0f);
    xyz_2 = _S1730;
    xyz_2[i32(0)] = 0.0f;
    var _S1731 : f32 = _S1623.value0_104.value1_15.white_fulcrum_0 * _S1730[i32(0)];
    var _S1732 : f32 = 0.0f;
    var _S1733 : f32 = 0.0f;
    s_bwdProp_bounded_pow_0(_S1686, &(_S1732), &(_S1733), _S1731);
    var _S1734 : f32 = 0.0f;
    var _S1735 : f32 = 0.0f;
    s_bwdProp_max_1(_S1683, &(_S1734), &(_S1735), _S1732);
    var _S1736 : vec3<f32> = xyz_2 + vec3<f32>(_S1734 / _S1623.value0_104.value1_15.white_fulcrum_0, 0.0f, 0.0f);
    var _S1737 : vec3<f32> = _S1626;
    s_bwdProp_lms_to_yrg_0(_S1679, &(_S1737), _S1736);
    var _S1738 : mat3x3<f32> = _S1711;
    var _S1739 : vec3<f32> = _S1626;
    s_bwdProp_mul_0(_S1676, &(_S1738), &(_S1739), _S1737);
    var _S1740 : vec3<f32> = sign_0 * (_S1668 * _S1739);
    var _S1741 : vec3<f32> = _S1626;
    var _S1742 : vec3<f32> = _S1626;
    s_bwdProp_bounded_pow_1(_S1673, &(_S1741), &(_S1742), _S1740);
    var _S1743 : vec3<f32> = _S1741 / _S1668;
    var _S1744 : vec3<f32> = _S1626;
    s_bwdProp_abs_1(_S1667, &(_S1744), _S1743);
    var _S1745 : vec3<f32> = _S1744;
    var _S1746 : Grading_0 = _S1708;
    _S1746.contrast_1 = _S1729;
    var _S1747 : Grading_0 = Grading_x24_syn_dadd_0(_S1715, _S1746);
    var s_diff_rgb_T_0 : vec3<f32> = _S1662 * _S1745;
    var _S1748 : mat3x3<f32> = _S1711;
    var _S1749 : vec3<f32> = _S1626;
    s_bwdProp_mul_0(_S1659, &(_S1748), &(_S1749), s_diff_rgb_T_0);
    var _S1750 : vec3<f32> = _S1626;
    s_bwdProp_yrg_to_lms_0(_S1656, &(_S1750), _S1749);
    var _S1751 : vec4<f32> = _S1627;
    s_bwdProp_ych_to_yrg_0(_S1653, &(_S1751), _S1750);
    var _S1752 : vec4<f32> = _S1627;
    s_bwdProp_gamut_check_yrg_0(_S1650, &(_S1752), _S1751);
    var _S1753 : f32 = _S1641 * _S1752[i32(1)];
    var _S1754 : f32 = 0.0f;
    var _S1755 : f32 = 0.0f;
    s_bwdProp_max_1(_S1646, &(_S1754), &(_S1755), _S1753);
    var _S1756 : Grading_0 = _S1708;
    _S1756.chroma_global_1 = _S1754;
    (*_S1624) = Grading_x24_syn_dadd_0(_S1747, _S1756);
    return;
}

fn grading_0() -> Grading_0
{
    var g_0 : Grading_0;
    g_0.chroma_global_1 = params_0.color_balance_0.chroma_global_0;
    g_0.contrast_1 = params_0.color_balance_0.contrast_0;
    g_0.saturation_global_1 = params_0.color_balance_0.saturation_global_0;
    g_0.brilliance_global_1 = params_0.color_balance_0.brilliance_global_0;
    return g_0;
}

fn opacity_masks_0( _S1757 : f32) -> vec3<f32>
{
    var offset_1 : f32 = _S1757 - params_0.color_balance_0.mask_grey_fulcrum_0;
    var norm_5 : f32 = offset_1 / params_0.color_balance_0.mask_grey_fulcrum_0;
    var shadows_1 : f32 = logistic_0(norm_5 * params_0.color_balance_0.shadows_weight_0);
    var highlights_1 : f32 = logistic_0(- norm_5 * params_0.color_balance_0.highlights_weight_0);
    var a_9 : f32 = 1.0f - shadows_1;
    var b_5 : f32 = 1.0f - highlights_1;
    return vec3<f32>(shadows_1, exp(- offset_1 * offset_1 * params_0.color_balance_0.midtones_weight_0 / 4.0f) * a_9 * a_9 * b_5 * b_5 * 8.0f, highlights_1);
}

fn jzazbz_saturation_0( _S1758 : vec3<f32>,  _S1759 : vec3<f32>,  _S1760 : Grading_0) -> vec3<f32>
{
    var jab_1 : vec3<f32> = xyz_to_jzazbz_0(_S1758);
    var _S1761 : f32 = jab_1.x;
    var _S1762 : f32 = length(jab_1.yz);
    var h_0 : f32 = atan2(jab_1.z, jab_1.y);
    var t_3 : f32 = atan2(_S1762, _S1761);
    var sin_t_1 : f32 = sin(t_3);
    var cos_t_1 : f32 = cos(t_3);
    var s_4 : f32 = _S1761 * cos_t_1 + _S1762 * sin_t_1;
    var o_1 : f32 = s_4 * min(max(t_3 * (_S1760.saturation_global_1 + dot(_S1759, params_0.color_balance_0.saturation_0.xyz)), - t_3), 1.5707963705062866f - t_3);
    var _S1763 : f32 = max(s_4 * (1.0f + _S1760.brilliance_global_1 + dot(_S1759, params_0.color_balance_0.brilliance_0.xyz)), 0.0f);
    var _S1764 : vec2<f32> = max(vec2<f32>(_S1763 * cos_t_1 - o_1 * sin_t_1, _S1763 * sin_t_1 + o_1 * cos_t_1), vec2<f32>(0.0f));
    var max_saturation_0 : f32 = lookup_gamut_0(h_0);
    var _S1765 : f32 = _S1764.x;
    var sat_1 : f32;
    if((_S1765 > 0.0f))
    {
        sat_1 = soft_clip_0(_S1764.y / _S1765, 0.80000001192092896f * max_saturation_0, max_saturation_0);
    }
    else
    {
        sat_1 = max_saturation_0;
    }
    var max_j_at_sat_1 : f32;
    if((sat_1 > 0.0f))
    {
        max_j_at_sat_1 = _S1764.y / sat_1;
    }
    else
    {
        max_j_at_sat_1 = _S1765;
    }
    var _S1766 : f32 = (_S1765 + max_j_at_sat_1) / 2.0f;
    var _S1767 : f32 = (_S1764.y + _S1765 * sat_1) / 2.0f;
    var cos_h_0 : f32 = cos(h_0);
    var sin_h_0 : f32 = sin(h_0);
    var iz_4 : f32 = jz_to_iz_0(_S1766);
    var lms_test_0 : vec3<f32> = (((vec3<f32>(iz_4, _S1767 * cos_h_0, _S1767 * sin_h_0)) * (mat3x3<f32>(1.0f, 0.13860504329204559f, 0.05804731696844101f, 1.0f, -0.13860504329204559f, -0.05804731696844101f, 1.0f, -0.096019245684146881f, -0.81189191341400146f))));
    var max_c_9 : f32;
    if(((lms_test_0.x) < 0.0f))
    {
        max_c_9 = min(- iz_4 / (0.13860504329204559f * cos_h_0 + 0.05804731696844101f * sin_h_0), _S1767);
    }
    else
    {
        max_c_9 = _S1767;
    }
    if(((lms_test_0.y) < 0.0f))
    {
        max_c_9 = min(- iz_4 / (-0.13860504329204559f * cos_h_0 - 0.05804731696844101f * sin_h_0), max_c_9);
    }
    if(((lms_test_0.z) < 0.0f))
    {
        max_c_9 = min(- iz_4 / (-0.096019245684146881f * cos_h_0 - 0.81189191341400146f * sin_h_0), max_c_9);
    }
    return jzazbz_to_xyz_0(vec3<f32>(_S1766, max_c_9 * cos_h_0, max_c_9 * sin_h_0));
}

fn ucs_saturation_0( _S1768 : vec3<f32>,  _S1769 : vec3<f32>,  _S1770 : Grading_0) -> vec3<f32>
{
    var l_white_7 : f32 = params_0.color_balance_0.l_white_0;
    var _S1771 : vec3<f32> = ucs_jch_to_hcb_0(xyy_to_ucs_jch_0(d65_xyz_to_xyy_0(_S1768), params_0.color_balance_0.l_white_0));
    var hcb_1 : vec3<f32> = _S1771;
    var _S1772 : f32 = _S1771.y;
    var _S1773 : f32 = _S1771.z;
    var radius_0 : f32 = sqrt(_S1772 * _S1772 + _S1773 * _S1773);
    var sin_t_2 : f32;
    var cos_t_2 : f32;
    if((radius_0 > 0.0f))
    {
        var _S1774 : f32 = hcb_1.z / radius_0;
        sin_t_2 = hcb_1.y / radius_0;
        cos_t_2 = _S1774;
    }
    else
    {
        sin_t_2 = 0.0f;
        cos_t_2 = 0.0f;
    }
    var _S1775 : f32 = max(1.00000000362749373e-15f, hcb_1.y);
    var w_1 : f32 = sin_t_2 * hcb_1.y + cos_t_2 * hcb_1.z;
    var _S1776 : f32 = _S1775 * _S1775;
    var _S1777 : f32 = w_1 * w_1;
    var max_a_1 : f32 = sqrt(_S1776 + _S1777) / _S1775;
    var a_10 : f32 = soft_clip_0(max(1.0f + _S1770.saturation_global_1 + dot(_S1769, params_0.color_balance_0.saturation_0.xyz), 0.0f), 0.5f * max_a_1, max_a_1);
    var p_prime_1 : f32 = (a_10 - 1.0f) * _S1775;
    var w_prime_1 : f32 = sqrt(max(_S1776 * (1.0f - a_10 * a_10) + _S1777, 0.0f)) * max(1.0f + _S1770.brilliance_global_1 + dot(_S1769, params_0.color_balance_0.brilliance_0.xyz), 0.0f);
    hcb_1[i32(1)] = max(cos_t_2 * p_prime_1 + sin_t_2 * w_prime_1, 0.0f);
    hcb_1[i32(2)] = max(- sin_t_2 * p_prime_1 + cos_t_2 * w_prime_1, 0.0f);
    var jch_3 : vec3<f32> = ucs_hcb_to_jch_0(hcb_1);
    var _S1778 : f32 = jch_3.z;
    var _S1779 : f32 = jch_3.x;
    var boundary_0 : vec3<f32> = ucs_jch_to_hsb_0(vec3<f32>(_S1779, 15.93299388885498f * pow0_0(_S1779 * l_white_7, 0.65239977836608887f) * pow0_0(lookup_gamut_0(_S1778), 0.60075569152832031f) / l_white_7, _S1778));
    var saturation_3 : f32;
    if(((hcb_1.z) > 0.0f))
    {
        saturation_3 = hcb_1.y / hcb_1.z;
    }
    else
    {
        saturation_3 = 0.0f;
    }
    var _S1780 : f32 = boundary_0.y;
    return xyy_to_xyz_0(ucs_jch_to_xyy_0(ucs_hsb_to_jch_0(vec3<f32>(hcb_1.x, soft_clip_0(saturation_3, 0.80000001192092896f * _S1780, _S1780), hcb_1.z)), l_white_7));
}

fn color_balance_1( _S1781 : vec3<f32>,  _S1782 : Grading_0) -> vec3<f32>
{
    var output_matrix_1 : mat3x3<f32> = mat3x3<f32>(params_0.color_balance_0.output_r0_0.xyz, params_0.color_balance_0.output_r1_0.xyz, params_0.color_balance_0.output_r2_0.xyz);
    var _S1783 : vec3<f32> = vec3<f32>(0.0f);
    var _S1784 : vec4<f32> = yrg_to_ych_0(lms_to_yrg_0((((max(_S1781, _S1783)) * (mat3x3<f32>(params_0.color_balance_0.input_r0_0.xyz, params_0.color_balance_0.input_r1_0.xyz, params_0.color_balance_0.input_r2_0.xyz))))));
    var ych_3 : vec4<f32> = _S1784;
    ych_3[i32(0)] = max(_S1784.x, 0.0f);
    var _S1785 : vec3<f32> = opacity_masks_0(pow0_0(ych_3.x, 0.41012057662010193f));
    ych_3 = vec4<f32>(ych_3.x, ych_3.y, params_0.color_balance_0.hue_cos_0 * ych_3.z - params_0.color_balance_0.hue_sin_0 * ych_3.w, params_0.color_balance_0.hue_sin_0 * ych_3.z + params_0.color_balance_0.hue_cos_0 * ych_3.w);
    var _S1786 : f32 = params_0.color_balance_0.vibrance_0;
    var vibrance_2 : f32;
    if((params_0.color_balance_0.vibrance_0) != 0.0f)
    {
        vibrance_2 = _S1786 * (1.0f - pow0_0(ych_3.y, abs(_S1786)));
    }
    else
    {
        vibrance_2 = 0.0f;
    }
    ych_3[i32(1)] = ych_3[i32(1)] * max(1.0f + _S1782.chroma_global_1 + dot(_S1785, params_0.color_balance_0.chroma_0.xyz) + vibrance_2, 0.0f);
    var _S1787 : vec4<f32> = gamut_check_yrg_0(ych_3);
    ych_3 = _S1787;
    var _S1788 : f32 = _S1785.z;
    var _S1789 : f32 = _S1785.x;
    var rgb_3 : vec3<f32> = ((((yrg_to_lms_0(ych_to_yrg_0(_S1787))) * (mat3x3<f32>(1.0877193212509155f, -0.66666668653488159f, 0.020618559792637825f, -0.087719298899173737f, 1.6666666269302368f, -0.051546391099691391f, 0.0f, 0.0f, 1.0309278964996338f)))) + params_0.color_balance_0.global_0.xyz) * (vec3<f32>((1.0f - _S1788)) * (vec3<f32>((1.0f - _S1789)) + vec3<f32>(_S1789) * params_0.color_balance_0.shadows_0.xyz) + vec3<f32>(_S1788) * params_0.color_balance_0.highlights_0.xyz);
    var w_2 : f32 = params_0.color_balance_0.white_fulcrum_0;
    if(((rgb_3.x) < 0.0f))
    {
        vibrance_2 = -1.0f;
    }
    else
    {
        vibrance_2 = 1.0f;
    }
    var _S1790 : f32;
    if(((rgb_3.y) < 0.0f))
    {
        _S1790 = -1.0f;
    }
    else
    {
        _S1790 = 1.0f;
    }
    var _S1791 : f32;
    if(((rgb_3.z) < 0.0f))
    {
        _S1791 = -1.0f;
    }
    else
    {
        _S1791 = 1.0f;
    }
    var _S1792 : vec3<f32> = vec3<f32>(w_2);
    var _S1793 : vec3<f32> = lms_to_yrg_0((((vec3<f32>(vibrance_2, _S1790, _S1791) * bounded_pow_1(abs(rgb_3) / _S1792, params_0.color_balance_0.midtones_0.xyz) * _S1792) * (mat3x3<f32>(0.94999998807907104f, 0.37999999523162842f, 0.0f, 0.05000000074505806f, 0.62000000476837158f, 0.029999999329447746f, 0.0f, 0.0f, 0.97000002861022949f)))));
    var yrg_3 : vec3<f32> = _S1793;
    yrg_3[i32(0)] = bounded_pow_0(max(_S1793.x / w_2, 0.0f), params_0.color_balance_0.midtones_y_0) * w_2;
    yrg_3[i32(0)] = params_0.color_balance_0.grey_fulcrum_0 * bounded_pow_0(yrg_3.x / params_0.color_balance_0.grey_fulcrum_0, _S1782.contrast_1);
    var xyz_3 : vec3<f32> = (((yrg_to_lms_0(yrg_3)) * (mat3x3<f32>(1.8079465627670288f, -1.299716591835022f, 0.34785878658294678f, 0.61783957481384277f, 0.39595451951026917f, -0.0410468690097332f, -0.12546959519386292f, 0.20478038489818573f, 1.7427418231964111f))));
    var xyz_4 : vec3<f32>;
    if((params_0.color_balance_0.saturation_formula_0) == u32(0))
    {
        xyz_4 = jzazbz_saturation_0(xyz_3, _S1785, _S1782);
    }
    else
    {
        xyz_4 = ucs_saturation_0(xyz_3, _S1785, _S1782);
    }
    return max((((xyz_4) * (output_matrix_1))), _S1783);
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_0 : vec3<u32>)
{
    var _S1794 : u32 = id_0.x;
    var _S1795 : bool;
    if((_S1794 >= (params_0.pass_0.width_0)))
    {
        _S1795 = true;
    }
    else
    {
        _S1795 = ((id_0.y) >= (params_0.pass_0.height_0));
    }
    if(_S1795)
    {
        return;
    }
    var i_3 : u32 = id_0.y * params_0.pass_0.width_0 + _S1794;
    var pixel_0 : vec4<f32> = input_0[i_3];
    var rgb_4 : vec3<f32> = pixel_0.xyz;
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
        var _S1796 : vec3<f32> = color_balance_1(rgb_4, grading_0());
        var _S1797 : u32 = k_0 + u32(1);
        rgb_4 = _S1796;
        k_0 = _S1797;
    }
    output_0[i_3] = vec4<f32>(rgb_4, pixel_0.w);
    return;
}

struct DiffPair_Grading_0
{
     primal_0 : Grading_0,
     differential_0 : Grading_0,
};

struct Tuple_47
{
     value0_105 : vec3<f32>,
     value1_18 : s_minCtx_s_bwdCallableCtx_color_balance_0,
};

fn s_apply_opacity_masks_1( _S1798 : f32) -> Tuple_23
{
    var offset_2 : f32 = _S1798 - params_0.color_balance_0.mask_grey_fulcrum_0;
    var norm_6 : f32 = offset_2 / params_0.color_balance_0.mask_grey_fulcrum_0;
    var _S1799 : Tuple_22 = s_apply_logistic_0(norm_6 * params_0.color_balance_0.shadows_weight_0);
    var _S1800 : Tuple_22 = s_apply_logistic_0(- norm_6 * params_0.color_balance_0.highlights_weight_0);
    var a_11 : f32 = 1.0f - _S1799.value0_26;
    var b_6 : f32 = 1.0f - _S1800.value0_26;
    var _S1801 : Tuple_23;
    _S1801.value0_27 = vec3<f32>(_S1799.value0_26, s_apply_exp_0(- offset_2 * offset_2 * params_0.color_balance_0.midtones_weight_0 / 4.0f).value0_11 * a_11 * a_11 * b_6 * b_6 * 8.0f, _S1800.value0_26);
    return _S1801;
}

struct Tuple_48
{
     value0_106 : vec3<f32>,
     value1_19 : s_minCtx_s_bwdCallableCtx_jzazbz_saturation_0,
};

fn s_apply_jzazbz_saturation_0( _S1802 : vec3<f32>,  _S1803 : vec3<f32>,  _S1804 : Grading_0) -> Tuple_48
{
    var _S1805 : s_minCtx_s_bwdCallableCtx_lookup_gamut_0 = s_minCtx_s_bwdCallableCtx_lookup_gamut_0( 0.0f, 0.0f );
    var _S1806 : s_minCtx_s_bwdCallableCtx_jzazbz_saturation_0;
    _S1806.value0_61 = 0.0f;
    _S1806.value1_8 = _S1805;
    var _S1807 : Tuple_30 = s_apply_xyz_to_jzazbz_0(_S1802);
    var _S1808 : f32 = _S1807.value0_46.x;
    var _S1809 : Tuple_12 = s_apply_length_0(_S1807.value0_46.yz);
    var _S1810 : Tuple_7 = s_apply_atan2_0(_S1807.value0_46.z, _S1807.value0_46.y);
    var _S1811 : Tuple_7 = s_apply_atan2_0(_S1809.value0_12, _S1808);
    var _S1812 : Tuple_6 = s_apply_sin_0(_S1811.value0_7);
    var _S1813 : Tuple_5 = s_apply_cos_0(_S1811.value0_7);
    var s_5 : f32 = _S1808 * _S1813.value0_5 + _S1809.value0_12 * _S1812.value0_6;
    var o_2 : f32 = s_5 * s_apply_min_0(s_apply_max_1(_S1811.value0_7 * (_S1804.saturation_global_1 + s_apply_dot_1(_S1803, params_0.color_balance_0.saturation_0.xyz).value0_16), - _S1811.value0_7).value0_19, 1.5707963705062866f - _S1811.value0_7).value0_10;
    var _S1814 : Tuple_17 = s_apply_max_1(s_5 * (1.0f + _S1804.brilliance_global_1 + s_apply_dot_1(_S1803, params_0.color_balance_0.brilliance_0.xyz).value0_16), 0.0f);
    var _S1815 : Tuple_4 = s_apply_max_0(vec2<f32>(_S1814.value0_19 * _S1813.value0_5 - o_2 * _S1812.value0_6, _S1814.value0_19 * _S1812.value0_6 + o_2 * _S1813.value0_5), vec2<f32>(0.0f));
    var _S1816 : Tuple_31 = s_apply_lookup_gamut_0(_S1810.value0_7);
    _S1806.value0_61 = _S1816.value0_50;
    _S1806.value1_8 = _S1816.value1_4;
    var _S1817 : f32 = _S1815.value0_4.x;
    var sat_2 : f32;
    if((_S1817 > 0.0f))
    {
        sat_2 = s_apply_soft_clip_0(_S1815.value0_4.y / _S1817, 0.80000001192092896f * _S1816.value0_50, _S1816.value0_50).value0_51;
    }
    else
    {
        sat_2 = _S1816.value0_50;
    }
    var max_j_at_sat_2 : f32;
    if((sat_2 > 0.0f))
    {
        max_j_at_sat_2 = _S1815.value0_4.y / sat_2;
    }
    else
    {
        max_j_at_sat_2 = _S1817;
    }
    var _S1818 : f32 = (_S1817 + max_j_at_sat_2) / 2.0f;
    var _S1819 : f32 = (_S1815.value0_4.y + _S1817 * sat_2) / 2.0f;
    var _S1820 : Tuple_5 = s_apply_cos_0(_S1810.value0_7);
    var _S1821 : Tuple_6 = s_apply_sin_0(_S1810.value0_7);
    var _S1822 : Tuple_33 = s_apply_jz_to_iz_0(_S1818);
    var _S1823 : Tuple_18 = s_apply_mul_0(mat3x3<f32>(1.0f, 0.13860504329204559f, 0.05804731696844101f, 1.0f, -0.13860504329204559f, -0.05804731696844101f, 1.0f, -0.096019245684146881f, -0.81189191341400146f), vec3<f32>(_S1822.value0_54, _S1819 * _S1820.value0_5, _S1819 * _S1821.value0_6));
    var max_c_10 : f32;
    if(((_S1823.value0_20.x) < 0.0f))
    {
        max_c_10 = s_apply_min_0(- _S1822.value0_54 / (0.13860504329204559f * _S1820.value0_5 + 0.05804731696844101f * _S1821.value0_6), _S1819).value0_10;
    }
    else
    {
        max_c_10 = _S1819;
    }
    if(((_S1823.value0_20.y) < 0.0f))
    {
        max_c_10 = s_apply_min_0(- _S1822.value0_54 / (-0.13860504329204559f * _S1820.value0_5 - 0.05804731696844101f * _S1821.value0_6), max_c_10).value0_10;
    }
    if(((_S1823.value0_20.z) < 0.0f))
    {
        max_c_10 = s_apply_min_0(- _S1822.value0_54 / (-0.096019245684146881f * _S1820.value0_5 - 0.81189191341400146f * _S1821.value0_6), max_c_10).value0_10;
    }
    var _S1824 : Tuple_48;
    _S1824.value0_106 = s_apply_jzazbz_to_xyz_0(vec3<f32>(_S1818, max_c_10 * _S1820.value0_5, max_c_10 * _S1821.value0_6)).value0_57;
    _S1824.value1_19 = _S1806;
    return _S1824;
}

struct Tuple_49
{
     value0_107 : vec3<f32>,
     value1_20 : s_minCtx_s_bwdCallableCtx_ucs_saturation_0,
};

fn s_apply_ucs_saturation_0( _S1825 : vec3<f32>,  _S1826 : vec3<f32>,  _S1827 : Grading_0) -> Tuple_49
{
    var _S1828 : s_minCtx_s_bwdCallableCtx_lookup_gamut_0 = s_minCtx_s_bwdCallableCtx_lookup_gamut_0( 0.0f, 0.0f );
    var _S1829 : s_minCtx_s_bwdCallableCtx_ucs_saturation_0;
    _S1829.value0_100 = 0.0f;
    _S1829.value1_13 = _S1828;
    var _S1830 : f32 = params_0.color_balance_0.l_white_0;
    var _S1831 : Tuple_40 = s_apply_ucs_jch_to_hcb_0(s_apply_xyy_to_ucs_jch_0(s_apply_d65_xyz_to_xyy_0(_S1825).value0_63, params_0.color_balance_0.l_white_0).value0_75);
    var _S1832 : f32 = _S1831.value0_78.y;
    var _S1833 : f32 = _S1831.value0_78.z;
    var _S1834 : Tuple_3 = s_apply_sqrt_0(_S1832 * _S1832 + _S1833 * _S1833);
    var sin_t_3 : f32;
    var cos_t_3 : f32;
    if(((_S1834.value0_3) > 0.0f))
    {
        var _S1835 : f32 = _S1833 / _S1834.value0_3;
        sin_t_3 = _S1832 / _S1834.value0_3;
        cos_t_3 = _S1835;
    }
    else
    {
        sin_t_3 = 0.0f;
        cos_t_3 = 0.0f;
    }
    var _S1836 : Tuple_17 = s_apply_max_1(1.00000000362749373e-15f, _S1832);
    var w_3 : f32 = sin_t_3 * _S1832 + cos_t_3 * _S1833;
    var _S1837 : f32 = _S1836.value0_19 * _S1836.value0_19;
    var _S1838 : f32 = w_3 * w_3;
    var max_a_2 : f32 = s_apply_sqrt_0(_S1837 + _S1838).value0_3 / _S1836.value0_19;
    var _S1839 : Tuple_32 = s_apply_soft_clip_0(s_apply_max_1(1.0f + _S1827.saturation_global_1 + s_apply_dot_1(_S1826, params_0.color_balance_0.saturation_0.xyz).value0_16, 0.0f).value0_19, 0.5f * max_a_2, max_a_2);
    var p_prime_2 : f32 = (_S1839.value0_51 - 1.0f) * _S1836.value0_19;
    var w_prime_2 : f32 = s_apply_sqrt_0(s_apply_max_1(_S1837 * (1.0f - _S1839.value0_51 * _S1839.value0_51) + _S1838, 0.0f).value0_19).value0_3 * s_apply_max_1(1.0f + _S1827.brilliance_global_1 + s_apply_dot_1(_S1826, params_0.color_balance_0.brilliance_0.xyz).value0_16, 0.0f).value0_19;
    var _S1840 : Tuple_17 = s_apply_max_1(cos_t_3 * p_prime_2 + sin_t_3 * w_prime_2, 0.0f);
    var _S1841 : vec3<f32> = _S1831.value0_78;
    _S1841[i32(1)] = _S1840.value0_19;
    _S1841[i32(2)] = s_apply_max_1(- sin_t_3 * p_prime_2 + cos_t_3 * w_prime_2, 0.0f).value0_19;
    var _S1842 : Tuple_41 = s_apply_ucs_hcb_to_jch_0(_S1841);
    var _S1843 : f32 = _S1842.value0_81.z;
    var _S1844 : Tuple_31 = s_apply_lookup_gamut_0(_S1843);
    _S1829.value0_100 = _S1844.value0_50;
    _S1829.value1_13 = _S1844.value1_4;
    var _S1845 : f32 = _S1842.value0_81.x;
    var _S1846 : Tuple_42 = s_apply_ucs_jch_to_hsb_0(vec3<f32>(_S1845, 15.93299388885498f * s_apply_pow0_0(_S1845 * _S1830, 0.65239977836608887f).value0_18 * s_apply_pow0_0(_S1844.value0_50, 0.60075569152832031f).value0_18 / _S1830, _S1843));
    var _S1847 : f32 = _S1841.z;
    var saturation_4 : f32;
    if((_S1847 > 0.0f))
    {
        saturation_4 = _S1841.y / _S1847;
    }
    else
    {
        saturation_4 = 0.0f;
    }
    var _S1848 : f32 = _S1846.value0_84.y;
    var _S1849 : Tuple_49;
    _S1849.value0_107 = s_apply_xyy_to_xyz_0(s_apply_ucs_jch_to_xyy_0(s_apply_ucs_hsb_to_jch_0(vec3<f32>(_S1841.x, s_apply_soft_clip_0(saturation_4, 0.80000001192092896f * _S1848, _S1848).value0_51, _S1847)).value0_87, _S1830).value0_93).value0_96;
    _S1849.value1_20 = _S1829;
    return _S1849;
}

fn s_apply_color_balance_0( _S1850 : vec3<f32>,  _S1851 : Grading_0) -> Tuple_47
{
    var _S1852 : vec3<f32> = vec3<f32>(0.0f);
    var _S1853 : s_minCtx_s_bwdCallableCtx_lookup_gamut_0 = s_minCtx_s_bwdCallableCtx_lookup_gamut_0( 0.0f, 0.0f );
    var _S1854 : s_minCtx_s_bwdCallableCtx_jzazbz_saturation_0 = s_minCtx_s_bwdCallableCtx_jzazbz_saturation_0( 0.0f, _S1853 );
    var _S1855 : s_minCtx_s_bwdCallableCtx_ucs_saturation_0 = s_minCtx_s_bwdCallableCtx_ucs_saturation_0( 0.0f, _S1853 );
    var _S1856 : s_minCtx_s_bwdCallableCtx_color_balance_0;
    _S1856.value0_103 = _S1852;
    _S1856.value1_16 = _S1854;
    _S1856.value2_4 = _S1852;
    _S1856.value3_2 = _S1855;
    var output_matrix_2 : mat3x3<f32> = mat3x3<f32>(params_0.color_balance_0.output_r0_0.xyz, params_0.color_balance_0.output_r1_0.xyz, params_0.color_balance_0.output_r2_0.xyz);
    var _S1857 : vec3<f32> = vec3<f32>(0.0f);
    var _S1858 : Tuple_21 = s_apply_yrg_to_ych_0(s_apply_lms_to_yrg_0(s_apply_mul_0(mat3x3<f32>(params_0.color_balance_0.input_r0_0.xyz, params_0.color_balance_0.input_r1_0.xyz, params_0.color_balance_0.input_r2_0.xyz), s_apply_max_2(_S1850, _S1857).value0_21).value0_20).value0_22);
    var _S1859 : Tuple_17 = s_apply_max_1(_S1858.value0_25.x, 0.0f);
    var _S1860 : vec4<f32> = _S1858.value0_25;
    _S1860[i32(0)] = _S1859.value0_19;
    var _S1861 : f32 = _S1860.x;
    var _S1862 : Tuple_23 = s_apply_opacity_masks_1(s_apply_pow0_0(_S1861, 0.41012057662010193f).value0_18);
    var _S1863 : f32 = _S1860.y;
    var _S1864 : f32 = _S1860.z;
    var _S1865 : f32 = _S1860.w;
    var ych_4 : vec4<f32> = vec4<f32>(_S1861, _S1863, params_0.color_balance_0.hue_cos_0 * _S1864 - params_0.color_balance_0.hue_sin_0 * _S1865, params_0.color_balance_0.hue_sin_0 * _S1864 + params_0.color_balance_0.hue_cos_0 * _S1865);
    var _S1866 : f32 = params_0.color_balance_0.vibrance_0;
    var vibrance_3 : f32;
    if((params_0.color_balance_0.vibrance_0) != 0.0f)
    {
        vibrance_3 = _S1866 * (1.0f - s_apply_pow0_0(_S1863, s_apply_abs_2(_S1866).value0_17).value0_18);
    }
    else
    {
        vibrance_3 = 0.0f;
    }
    var _S1867 : f32 = _S1863 * s_apply_max_1(1.0f + _S1851.chroma_global_1 + s_apply_dot_1(_S1862.value0_27, params_0.color_balance_0.chroma_0.xyz).value0_16 + vibrance_3, 0.0f).value0_19;
    _S1860 = ych_4;
    _S1860[i32(1)] = _S1867;
    var _S1868 : f32 = _S1862.value0_27.z;
    var _S1869 : f32 = _S1862.value0_27.x;
    var rgb_5 : vec3<f32> = (s_apply_mul_0(mat3x3<f32>(1.0877193212509155f, -0.66666668653488159f, 0.020618559792637825f, -0.087719298899173737f, 1.6666666269302368f, -0.051546391099691391f, 0.0f, 0.0f, 1.0309278964996338f), s_apply_yrg_to_lms_0(s_apply_ych_to_yrg_0(s_apply_gamut_check_yrg_0(_S1860).value0_31).value0_28).value0_34).value0_20 + params_0.color_balance_0.global_0.xyz) * (vec3<f32>((1.0f - _S1868)) * (vec3<f32>((1.0f - _S1869)) + vec3<f32>(_S1869) * params_0.color_balance_0.shadows_0.xyz) + vec3<f32>(_S1868) * params_0.color_balance_0.highlights_0.xyz);
    var _S1870 : f32 = params_0.color_balance_0.white_fulcrum_0;
    if(((rgb_5.x) < 0.0f))
    {
        vibrance_3 = -1.0f;
    }
    else
    {
        vibrance_3 = 1.0f;
    }
    var _S1871 : f32;
    if(((rgb_5.y) < 0.0f))
    {
        _S1871 = -1.0f;
    }
    else
    {
        _S1871 = 1.0f;
    }
    var _S1872 : f32;
    if(((rgb_5.z) < 0.0f))
    {
        _S1872 = -1.0f;
    }
    else
    {
        _S1872 = 1.0f;
    }
    var _S1873 : vec3<f32> = vec3<f32>(_S1870);
    var _S1874 : Tuple_20 = s_apply_lms_to_yrg_0(s_apply_mul_0(mat3x3<f32>(0.94999998807907104f, 0.37999999523162842f, 0.0f, 0.05000000074505806f, 0.62000000476837158f, 0.029999999329447746f, 0.0f, 0.0f, 0.97000002861022949f), vec3<f32>(vibrance_3, _S1871, _S1872) * s_apply_bounded_pow_1(s_apply_abs_1(rgb_5).value0_15 / _S1873, params_0.color_balance_0.midtones_0.xyz).value0_40 * _S1873).value0_20);
    var _S1875 : f32 = s_apply_bounded_pow_0(s_apply_max_1(_S1874.value0_22.x / _S1870, 0.0f).value0_19, params_0.color_balance_0.midtones_y_0).value0_37 * _S1870;
    var xyz_5 : vec3<f32> = _S1874.value0_22;
    xyz_5[i32(0)] = _S1875;
    xyz_5[i32(0)] = params_0.color_balance_0.grey_fulcrum_0 * s_apply_bounded_pow_0(xyz_5.x / params_0.color_balance_0.grey_fulcrum_0, _S1851.contrast_1).value0_37;
    var _S1876 : Tuple_18 = s_apply_mul_0(mat3x3<f32>(1.8079465627670288f, -1.299716591835022f, 0.34785878658294678f, 0.61783957481384277f, 0.39595451951026917f, -0.0410468690097332f, -0.12546959519386292f, 0.20478038489818573f, 1.7427418231964111f), s_apply_yrg_to_lms_0(xyz_5).value0_34);
    if((params_0.color_balance_0.saturation_formula_0) == u32(0))
    {
        var _S1877 : Tuple_48 = s_apply_jzazbz_saturation_0(_S1876.value0_20, _S1862.value0_27, _S1851);
        _S1856.value0_103 = _S1877.value0_106;
        _S1856.value1_16 = _S1877.value1_19;
        xyz_5 = _S1877.value0_106;
    }
    else
    {
        var _S1878 : Tuple_49 = s_apply_ucs_saturation_0(_S1876.value0_20, _S1862.value0_27, _S1851);
        _S1856.value2_4 = _S1878.value0_107;
        _S1856.value3_2 = _S1878.value1_20;
        xyz_5 = _S1878.value0_107;
    }
    var _S1879 : Tuple_47;
    _S1879.value0_105 = s_apply_max_2(s_apply_mul_0(output_matrix_2, xyz_5).value0_20, _S1857).value0_21;
    _S1879.value1_18 = _S1856;
    return _S1879;
}

fn _S1880( _S1881 : vec3<f32>,  _S1882 : ptr<function, DiffPair_Grading_0>,  _S1883 : vec3<f32>)
{
    var _S1884 : Tuple_47 = s_apply_color_balance_0(_S1881, (*_S1882).primal_0);
    var _S1885 : s_paramCtx_s_bwdCallableCtx_color_balance_0;
    _S1885.value0_102 = _S1881;
    _S1885.value1_15.input_r0_0 = params_0.color_balance_0.input_r0_0;
    _S1885.value1_15.input_r1_0 = params_0.color_balance_0.input_r1_0;
    _S1885.value1_15.input_r2_0 = params_0.color_balance_0.input_r2_0;
    _S1885.value1_15.output_r0_0 = params_0.color_balance_0.output_r0_0;
    _S1885.value1_15.output_r1_0 = params_0.color_balance_0.output_r1_0;
    _S1885.value1_15.output_r2_0 = params_0.color_balance_0.output_r2_0;
    _S1885.value1_15.global_0 = params_0.color_balance_0.global_0;
    _S1885.value1_15.shadows_0 = params_0.color_balance_0.shadows_0;
    _S1885.value1_15.highlights_0 = params_0.color_balance_0.highlights_0;
    _S1885.value1_15.midtones_0 = params_0.color_balance_0.midtones_0;
    _S1885.value1_15.chroma_0 = params_0.color_balance_0.chroma_0;
    _S1885.value1_15.saturation_0 = params_0.color_balance_0.saturation_0;
    _S1885.value1_15.brilliance_0 = params_0.color_balance_0.brilliance_0;
    _S1885.value1_15.chroma_global_0 = params_0.color_balance_0.chroma_global_0;
    _S1885.value1_15.vibrance_0 = params_0.color_balance_0.vibrance_0;
    _S1885.value1_15.contrast_0 = params_0.color_balance_0.contrast_0;
    _S1885.value1_15.saturation_global_0 = params_0.color_balance_0.saturation_global_0;
    _S1885.value1_15.brilliance_global_0 = params_0.color_balance_0.brilliance_global_0;
    _S1885.value1_15.hue_cos_0 = params_0.color_balance_0.hue_cos_0;
    _S1885.value1_15.hue_sin_0 = params_0.color_balance_0.hue_sin_0;
    _S1885.value1_15.shadows_weight_0 = params_0.color_balance_0.shadows_weight_0;
    _S1885.value1_15.highlights_weight_0 = params_0.color_balance_0.highlights_weight_0;
    _S1885.value1_15.midtones_weight_0 = params_0.color_balance_0.midtones_weight_0;
    _S1885.value1_15.mask_grey_fulcrum_0 = params_0.color_balance_0.mask_grey_fulcrum_0;
    _S1885.value1_15.white_fulcrum_0 = params_0.color_balance_0.white_fulcrum_0;
    _S1885.value1_15.grey_fulcrum_0 = params_0.color_balance_0.grey_fulcrum_0;
    _S1885.value1_15.midtones_y_0 = params_0.color_balance_0.midtones_y_0;
    _S1885.value1_15.l_white_0 = params_0.color_balance_0.l_white_0;
    _S1885.value1_15.saturation_formula_0 = params_0.color_balance_0.saturation_formula_0;
    _S1885.value2_3 = (*_S1882).primal_0;
    var _S1886 : s_bwdCallableCtx_color_balance_0 = s_bwdCallableCtx_color_balance_0( _S1885, _S1884.value1_18 );
    var _S1887 : Grading_0 = (*_S1882).differential_0;
    s_bwdProp_color_balance_0(_S1886, &(_S1887), _S1883);
    (*_S1882).differential_0 = _S1887;
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn colorbalancergb_backward(@builtin(global_invocation_id) id_1 : vec3<u32>)
{
    var _S1888 : u32 = id_1.x;
    var _S1889 : bool;
    if((_S1888 >= (params_0.pass_0.width_0)))
    {
        _S1889 = true;
    }
    else
    {
        _S1889 = ((id_1.y) >= (params_0.pass_0.height_0));
    }
    if(_S1889)
    {
        return;
    }
    var i_4 : u32 = id_1.y * params_0.pass_0.width_0 + _S1888;
    var _S1890 : Grading_0 = Grading_x24_syn_dzero_0();
    var g_1 : DiffPair_Grading_0;
    g_1.primal_0 = grading_0();
    g_1.differential_0 = _S1890;
    _S1880(input_0[i_4].xyz, &(g_1), derivative_in_0[i_4].xyz);
    derivative_out_0[i_4] = vec4<f32>(g_1.differential_0.chroma_global_1, g_1.differential_0.contrast_1, g_1.differential_0.saturation_global_1, g_1.differential_0.brilliance_global_1);
    return;
}

fn s_fwd_opacity_masks_0( _S1891 : DiffPair_float_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var offset_3 : f32 = _S1891.primal_0 - params_0.color_balance_0.mask_grey_fulcrum_0;
    var norm_7 : f32 = offset_3 / params_0.color_balance_0.mask_grey_fulcrum_0;
    var s_diff_norm_1 : f32 = _S1891.differential_0 / params_0.color_balance_0.mask_grey_fulcrum_0;
    var _S1892 : DiffPair_float_0 = DiffPair_float_0( norm_7 * params_0.color_balance_0.shadows_weight_0, s_diff_norm_1 * params_0.color_balance_0.shadows_weight_0 );
    var _S1893 : DiffPair_float_0 = s_fwd_logistic_0(_S1892);
    var _S1894 : DiffPair_float_0 = DiffPair_float_0( - norm_7 * params_0.color_balance_0.highlights_weight_0, - s_diff_norm_1 * params_0.color_balance_0.highlights_weight_0 );
    var _S1895 : DiffPair_float_0 = s_fwd_logistic_0(_S1894);
    var a_12 : f32 = 1.0f - _S1893.primal_0;
    var s_diff_a_2 : f32 = 0.0f - _S1893.differential_0;
    var b_7 : f32 = 1.0f - _S1895.primal_0;
    var s_diff_b_1 : f32 = 0.0f - _S1895.differential_0;
    var _S1896 : f32 = - offset_3;
    var _S1897 : DiffPair_float_0 = DiffPair_float_0( _S1896 * offset_3 * params_0.color_balance_0.midtones_weight_0 / 4.0f, (- _S1891.differential_0 * offset_3 + _S1891.differential_0 * _S1896) * params_0.color_balance_0.midtones_weight_0 * 0.25f );
    var _S1898 : DiffPair_float_0 = _d_exp_1(_S1897);
    var _S1899 : f32 = _S1898.primal_0 * a_12;
    var _S1900 : f32 = _S1899 * a_12;
    var _S1901 : f32 = _S1900 * b_7;
    var _S1902 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S1893.primal_0, _S1901 * b_7 * 8.0f, _S1895.primal_0), vec3<f32>(_S1893.differential_0, ((((_S1898.differential_0 * a_12 + s_diff_a_2 * _S1898.primal_0) * a_12 + s_diff_a_2 * _S1899) * b_7 + s_diff_b_1 * _S1900) * b_7 + s_diff_b_1 * _S1901) * 8.0f, _S1895.differential_0) );
    return _S1902;
}

fn s_fwd_jzazbz_saturation_0( _S1903 : DiffPair_vectorx3Cfloatx2C3x3E_0,  _S1904 : DiffPair_vectorx3Cfloatx2C3x3E_0,  _S1905 : DiffPair_Grading_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S1906 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S1903.primal_0, _S1903.differential_0 );
    var _S1907 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_xyz_to_jzazbz_0(_S1906);
    var _S1908 : f32 = _S1907.primal_0.x;
    var _S1909 : f32 = _S1907.differential_0.x;
    var _S1910 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( _S1907.primal_0.yz, _S1907.differential_0.yz );
    var _S1911 : DiffPair_float_0 = s_fwd_length_impl_0(_S1910);
    var _S1912 : DiffPair_float_0 = DiffPair_float_0( _S1907.primal_0.z, _S1907.differential_0.z );
    var _S1913 : DiffPair_float_0 = DiffPair_float_0( _S1907.primal_0.y, _S1907.differential_0.y );
    var _S1914 : DiffPair_float_0 = _d_atan2_1(_S1912, _S1913);
    var _S1915 : DiffPair_float_0 = DiffPair_float_0( _S1911.primal_0, _S1911.differential_0 );
    var _S1916 : DiffPair_float_0 = DiffPair_float_0( _S1908, _S1909 );
    var _S1917 : DiffPair_float_0 = _d_atan2_1(_S1915, _S1916);
    var _S1918 : DiffPair_float_0 = DiffPair_float_0( _S1917.primal_0, _S1917.differential_0 );
    var _S1919 : DiffPair_float_0 = _d_sin_1(_S1918);
    var _S1920 : DiffPair_float_0 = _d_cos_1(_S1918);
    var _S1921 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S1904.primal_0, _S1904.differential_0 );
    var _S1922 : vec3<f32> = vec3<f32>(0.0f);
    var _S1923 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( params_0.color_balance_0.brilliance_0.xyz, _S1922 );
    var _S1924 : DiffPair_float_0 = _d_dot_3(_S1921, _S1923);
    var brilliance_2 : f32 = 1.0f + _S1905.primal_0.brilliance_global_1 + _S1924.primal_0;
    var _S1925 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( params_0.color_balance_0.saturation_0.xyz, _S1922 );
    var _S1926 : DiffPair_float_0 = _d_dot_3(_S1921, _S1925);
    var saturation_5 : f32 = _S1905.primal_0.saturation_global_1 + _S1926.primal_0;
    var s_6 : f32 = _S1908 * _S1920.primal_0 + _S1911.primal_0 * _S1919.primal_0;
    var s_diff_s_1 : f32 = _S1909 * _S1920.primal_0 + _S1920.differential_0 * _S1908 + (_S1911.differential_0 * _S1919.primal_0 + _S1919.differential_0 * _S1911.primal_0);
    var _S1927 : DiffPair_float_0 = DiffPair_float_0( _S1917.primal_0 * saturation_5, _S1917.differential_0 * saturation_5 + (_S1905.differential_0.saturation_global_1 + _S1926.differential_0) * _S1917.primal_0 );
    var _S1928 : DiffPair_float_0 = DiffPair_float_0( - _S1917.primal_0, - _S1917.differential_0 );
    var _S1929 : DiffPair_float_0 = _d_max_1(_S1927, _S1928);
    var _S1930 : DiffPair_float_0 = DiffPair_float_0( _S1929.primal_0, _S1929.differential_0 );
    var _S1931 : DiffPair_float_0 = DiffPair_float_0( 1.5707963705062866f - _S1917.primal_0, 0.0f - _S1917.differential_0 );
    var _S1932 : DiffPair_float_0 = _d_min_1(_S1930, _S1931);
    var o_3 : f32 = s_6 * _S1932.primal_0;
    var s_diff_o_0 : f32 = s_diff_s_1 * _S1932.primal_0 + _S1932.differential_0 * s_6;
    var _S1933 : DiffPair_float_0 = DiffPair_float_0( s_6 * brilliance_2, s_diff_s_1 * brilliance_2 + (_S1905.differential_0.brilliance_global_1 + _S1924.differential_0) * s_6 );
    var _S1934 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S1935 : DiffPair_float_0 = _d_max_1(_S1933, _S1934);
    var _S1936 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( vec2<f32>(_S1935.primal_0 * _S1920.primal_0 - o_3 * _S1919.primal_0, _S1935.primal_0 * _S1919.primal_0 + o_3 * _S1920.primal_0), vec2<f32>(_S1935.differential_0 * _S1920.primal_0 + _S1920.differential_0 * _S1935.primal_0 - (s_diff_o_0 * _S1919.primal_0 + _S1919.differential_0 * o_3), _S1935.differential_0 * _S1919.primal_0 + _S1919.differential_0 * _S1935.primal_0 + (s_diff_o_0 * _S1920.primal_0 + _S1920.differential_0 * o_3)) );
    var _S1937 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( vec2<f32>(0.0f), vec2<f32>(0.0f) );
    var _S1938 : DiffPair_vectorx3Cfloatx2C2x3E_0 = _d_max_vector_2(_S1936, _S1937);
    var _S1939 : DiffPair_float_0 = DiffPair_float_0( _S1914.primal_0, _S1914.differential_0 );
    var _S1940 : DiffPair_float_0 = s_fwd_lookup_gamut_0(_S1939);
    var _S1941 : f32 = _S1938.primal_0.x;
    var _S1942 : f32 = _S1938.differential_0.x;
    var sat_3 : f32;
    var s_diff_sat_0 : f32;
    if((_S1941 > 0.0f))
    {
        var _S1943 : f32 = _S1938.primal_0.y;
        var _S1944 : DiffPair_float_0 = DiffPair_float_0( _S1943 / _S1941, (_S1938.differential_0.y * _S1941 - _S1943 * _S1942) / (_S1941 * _S1941) );
        var _S1945 : DiffPair_float_0 = DiffPair_float_0( 0.80000001192092896f * _S1940.primal_0, _S1940.differential_0 * 0.80000001192092896f );
        var _S1946 : DiffPair_float_0 = DiffPair_float_0( _S1940.primal_0, _S1940.differential_0 );
        var _S1947 : DiffPair_float_0 = s_fwd_soft_clip_0(_S1944, _S1945, _S1946);
        sat_3 = _S1947.primal_0;
        s_diff_sat_0 = _S1947.differential_0;
    }
    else
    {
        sat_3 = _S1940.primal_0;
        s_diff_sat_0 = _S1940.differential_0;
    }
    var max_j_at_sat_3 : f32;
    var s_diff_max_j_at_sat_0 : f32;
    if((sat_3 > 0.0f))
    {
        var _S1948 : f32 = _S1938.primal_0.y;
        var _S1949 : f32 = (_S1938.differential_0.y * sat_3 - _S1948 * s_diff_sat_0) / (sat_3 * sat_3);
        max_j_at_sat_3 = _S1948 / sat_3;
        s_diff_max_j_at_sat_0 = _S1949;
    }
    else
    {
        max_j_at_sat_3 = _S1941;
        s_diff_max_j_at_sat_0 = _S1942;
    }
    var _S1950 : f32 = (_S1941 + max_j_at_sat_3) / 2.0f;
    var _S1951 : f32 = (_S1942 + s_diff_max_j_at_sat_0) * 0.5f;
    var _S1952 : f32 = (_S1938.primal_0.y + _S1941 * sat_3) / 2.0f;
    var _S1953 : f32 = (_S1938.differential_0.y + (_S1942 * sat_3 + s_diff_sat_0 * _S1941)) * 0.5f;
    var _S1954 : DiffPair_float_0 = _d_cos_1(_S1939);
    var _S1955 : DiffPair_float_0 = _d_sin_1(_S1939);
    var _S1956 : DiffPair_float_0 = DiffPair_float_0( _S1950, _S1951 );
    var _S1957 : DiffPair_float_0 = s_fwd_jz_to_iz_0(_S1956);
    var _S1958 : vec3<f32> = (((vec3<f32>(_S1957.primal_0, _S1952 * _S1954.primal_0, _S1952 * _S1955.primal_0)) * (mat3x3<f32>(1.0f, 0.13860504329204559f, 0.05804731696844101f, 1.0f, -0.13860504329204559f, -0.05804731696844101f, 1.0f, -0.096019245684146881f, -0.81189191341400146f))));
    var max_c_11 : f32;
    var s_diff_max_c_2 : f32;
    if(((_S1958.x) < 0.0f))
    {
        var _S1959 : f32 = - _S1957.primal_0;
        var _S1960 : f32 = 0.13860504329204559f * _S1954.primal_0 + 0.05804731696844101f * _S1955.primal_0;
        var _S1961 : DiffPair_float_0 = DiffPair_float_0( _S1959 / _S1960, (- _S1957.differential_0 * _S1960 - _S1959 * (_S1954.differential_0 * 0.13860504329204559f + _S1955.differential_0 * 0.05804731696844101f)) / (_S1960 * _S1960) );
        var _S1962 : DiffPair_float_0 = DiffPair_float_0( _S1952, _S1953 );
        var _S1963 : DiffPair_float_0 = _d_min_1(_S1961, _S1962);
        max_c_11 = _S1963.primal_0;
        s_diff_max_c_2 = _S1963.differential_0;
    }
    else
    {
        max_c_11 = _S1952;
        s_diff_max_c_2 = _S1953;
    }
    if(((_S1958.y) < 0.0f))
    {
        var _S1964 : f32 = - _S1957.primal_0;
        var _S1965 : f32 = -0.13860504329204559f * _S1954.primal_0 - 0.05804731696844101f * _S1955.primal_0;
        var _S1966 : DiffPair_float_0 = DiffPair_float_0( _S1964 / _S1965, (- _S1957.differential_0 * _S1965 - _S1964 * (_S1954.differential_0 * -0.13860504329204559f - _S1955.differential_0 * 0.05804731696844101f)) / (_S1965 * _S1965) );
        var _S1967 : DiffPair_float_0 = DiffPair_float_0( max_c_11, s_diff_max_c_2 );
        var _S1968 : DiffPair_float_0 = _d_min_1(_S1966, _S1967);
        max_c_11 = _S1968.primal_0;
        s_diff_max_c_2 = _S1968.differential_0;
    }
    if(((_S1958.z) < 0.0f))
    {
        var _S1969 : f32 = - _S1957.primal_0;
        var _S1970 : f32 = -0.096019245684146881f * _S1954.primal_0 - 0.81189191341400146f * _S1955.primal_0;
        var _S1971 : DiffPair_float_0 = DiffPair_float_0( _S1969 / _S1970, (- _S1957.differential_0 * _S1970 - _S1969 * (_S1954.differential_0 * -0.096019245684146881f - _S1955.differential_0 * 0.81189191341400146f)) / (_S1970 * _S1970) );
        var _S1972 : DiffPair_float_0 = DiffPair_float_0( max_c_11, s_diff_max_c_2 );
        var _S1973 : DiffPair_float_0 = _d_min_1(_S1971, _S1972);
        max_c_11 = _S1973.primal_0;
        s_diff_max_c_2 = _S1973.differential_0;
    }
    var _S1974 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S1950, max_c_11 * _S1954.primal_0, max_c_11 * _S1955.primal_0), vec3<f32>(_S1951, s_diff_max_c_2 * _S1954.primal_0 + _S1954.differential_0 * max_c_11, s_diff_max_c_2 * _S1955.primal_0 + _S1955.differential_0 * max_c_11) );
    var _S1975 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_jzazbz_to_xyz_0(_S1974);
    var _S1976 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S1975.primal_0, _S1975.differential_0 );
    return _S1976;
}

fn s_fwd_ucs_saturation_0( _S1977 : DiffPair_vectorx3Cfloatx2C3x3E_0,  _S1978 : DiffPair_vectorx3Cfloatx2C3x3E_0,  _S1979 : DiffPair_Grading_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S1980 : f32 = params_0.color_balance_0.l_white_0;
    var _S1981 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S1977.primal_0, _S1977.differential_0 );
    var _S1982 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_d65_xyz_to_xyy_0(_S1981);
    var _S1983 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S1982.primal_0, _S1982.differential_0 );
    var _S1984 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_xyy_to_ucs_jch_0(_S1983, params_0.color_balance_0.l_white_0);
    var _S1985 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S1984.primal_0, _S1984.differential_0 );
    var _S1986 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_ucs_jch_to_hcb_0(_S1985);
    var _S1987 : f32 = _S1986.primal_0.y;
    var _S1988 : f32 = _S1986.differential_0.y;
    var _S1989 : f32 = _S1988 * _S1987;
    var _S1990 : f32 = _S1986.primal_0.z;
    var _S1991 : f32 = _S1986.differential_0.z;
    var _S1992 : f32 = _S1991 * _S1990;
    var _S1993 : DiffPair_float_0 = DiffPair_float_0( _S1987 * _S1987 + _S1990 * _S1990, _S1989 + _S1989 + (_S1992 + _S1992) );
    var _S1994 : DiffPair_float_0 = _d_sqrt_1(_S1993);
    var sin_t_4 : f32;
    var s_diff_sin_t_0 : f32;
    var cos_t_4 : f32;
    var s_diff_cos_t_0 : f32;
    if(((_S1994.primal_0) > 0.0f))
    {
        var _S1995 : f32 = _S1994.primal_0 * _S1994.primal_0;
        var _S1996 : f32 = (_S1988 * _S1994.primal_0 - _S1987 * _S1994.differential_0) / _S1995;
        var _S1997 : f32 = _S1990 / _S1994.primal_0;
        var _S1998 : f32 = (_S1991 * _S1994.primal_0 - _S1990 * _S1994.differential_0) / _S1995;
        sin_t_4 = _S1987 / _S1994.primal_0;
        s_diff_sin_t_0 = _S1996;
        cos_t_4 = _S1997;
        s_diff_cos_t_0 = _S1998;
    }
    else
    {
        sin_t_4 = 0.0f;
        s_diff_sin_t_0 = 0.0f;
        cos_t_4 = 0.0f;
        s_diff_cos_t_0 = 0.0f;
    }
    var _S1999 : DiffPair_float_0 = DiffPair_float_0( 1.00000000362749373e-15f, 0.0f );
    var _S2000 : DiffPair_float_0 = DiffPair_float_0( _S1987, _S1988 );
    var _S2001 : DiffPair_float_0 = _d_max_1(_S1999, _S2000);
    var w_4 : f32 = sin_t_4 * _S1987 + cos_t_4 * _S1990;
    var _S2002 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S1978.primal_0, _S1978.differential_0 );
    var _S2003 : vec3<f32> = vec3<f32>(0.0f);
    var _S2004 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( params_0.color_balance_0.saturation_0.xyz, _S2003 );
    var _S2005 : DiffPair_float_0 = _d_dot_3(_S2002, _S2004);
    var _S2006 : DiffPair_float_0 = DiffPair_float_0( 1.0f + _S1979.primal_0.saturation_global_1 + _S2005.primal_0, _S1979.differential_0.saturation_global_1 + _S2005.differential_0 );
    var _S2007 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S2008 : DiffPair_float_0 = _d_max_1(_S2006, _S2007);
    var _S2009 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( params_0.color_balance_0.brilliance_0.xyz, _S2003 );
    var _S2010 : DiffPair_float_0 = _d_dot_3(_S2002, _S2009);
    var _S2011 : DiffPair_float_0 = DiffPair_float_0( 1.0f + _S1979.primal_0.brilliance_global_1 + _S2010.primal_0, _S1979.differential_0.brilliance_global_1 + _S2010.differential_0 );
    var _S2012 : DiffPair_float_0 = _d_max_1(_S2011, _S2007);
    var _S2013 : f32 = _S2001.primal_0 * _S2001.primal_0;
    var _S2014 : f32 = _S2001.differential_0 * _S2001.primal_0;
    var _S2015 : f32 = _S2014 + _S2014;
    var _S2016 : f32 = w_4 * w_4;
    var _S2017 : f32 = (s_diff_sin_t_0 * _S1987 + _S1988 * sin_t_4 + (s_diff_cos_t_0 * _S1990 + _S1991 * cos_t_4)) * w_4;
    var _S2018 : f32 = _S2017 + _S2017;
    var _S2019 : DiffPair_float_0 = DiffPair_float_0( _S2013 + _S2016, _S2015 + _S2018 );
    var _S2020 : DiffPair_float_0 = _d_sqrt_1(_S2019);
    var max_a_3 : f32 = _S2020.primal_0 / _S2001.primal_0;
    var s_diff_max_a_0 : f32 = (_S2020.differential_0 * _S2001.primal_0 - _S2020.primal_0 * _S2001.differential_0) / _S2013;
    var _S2021 : DiffPair_float_0 = DiffPair_float_0( _S2008.primal_0, _S2008.differential_0 );
    var _S2022 : DiffPair_float_0 = DiffPair_float_0( 0.5f * max_a_3, s_diff_max_a_0 * 0.5f );
    var _S2023 : DiffPair_float_0 = DiffPair_float_0( max_a_3, s_diff_max_a_0 );
    var _S2024 : DiffPair_float_0 = s_fwd_soft_clip_0(_S2021, _S2022, _S2023);
    var _S2025 : f32 = _S2024.primal_0 - 1.0f;
    var p_prime_3 : f32 = _S2025 * _S2001.primal_0;
    var s_diff_p_prime_0 : f32 = _S2024.differential_0 * _S2001.primal_0 + _S2001.differential_0 * _S2025;
    var _S2026 : f32 = _S2024.differential_0 * _S2024.primal_0;
    var _S2027 : f32 = 1.0f - _S2024.primal_0 * _S2024.primal_0;
    var _S2028 : DiffPair_float_0 = DiffPair_float_0( _S2013 * _S2027 + _S2016, _S2015 * _S2027 + (0.0f - (_S2026 + _S2026)) * _S2013 + _S2018 );
    var _S2029 : DiffPair_float_0 = _d_max_1(_S2028, _S2007);
    var _S2030 : DiffPair_float_0 = DiffPair_float_0( _S2029.primal_0, _S2029.differential_0 );
    var _S2031 : DiffPair_float_0 = _d_sqrt_1(_S2030);
    var w_prime_3 : f32 = _S2031.primal_0 * _S2012.primal_0;
    var s_diff_w_prime_0 : f32 = _S2031.differential_0 * _S2012.primal_0 + _S2012.differential_0 * _S2031.primal_0;
    var _S2032 : DiffPair_float_0 = DiffPair_float_0( cos_t_4 * p_prime_3 + sin_t_4 * w_prime_3, s_diff_cos_t_0 * p_prime_3 + s_diff_p_prime_0 * cos_t_4 + (s_diff_sin_t_0 * w_prime_3 + s_diff_w_prime_0 * sin_t_4) );
    var _S2033 : DiffPair_float_0 = _d_max_1(_S2032, _S2007);
    var _S2034 : vec3<f32> = _S1986.primal_0;
    _S2034[i32(1)] = _S2033.primal_0;
    var _S2035 : vec3<f32> = _S1986.differential_0;
    _S2035[i32(1)] = _S2033.differential_0;
    var _S2036 : f32 = - sin_t_4;
    var _S2037 : DiffPair_float_0 = DiffPair_float_0( _S2036 * p_prime_3 + cos_t_4 * w_prime_3, - s_diff_sin_t_0 * p_prime_3 + s_diff_p_prime_0 * _S2036 + (s_diff_cos_t_0 * w_prime_3 + s_diff_w_prime_0 * cos_t_4) );
    var _S2038 : DiffPair_float_0 = _d_max_1(_S2037, _S2007);
    _S2034[i32(2)] = _S2038.primal_0;
    _S2035[i32(2)] = _S2038.differential_0;
    var _S2039 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2034, _S2035 );
    var _S2040 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_ucs_hcb_to_jch_0(_S2039);
    var _S2041 : f32 = _S2040.primal_0.z;
    var _S2042 : f32 = _S2040.differential_0.z;
    var _S2043 : DiffPair_float_0 = DiffPair_float_0( _S2041, _S2042 );
    var _S2044 : DiffPair_float_0 = s_fwd_lookup_gamut_0(_S2043);
    var _S2045 : f32 = _S2040.primal_0.x;
    var _S2046 : f32 = _S2040.differential_0.x;
    var _S2047 : DiffPair_float_0 = DiffPair_float_0( _S2045 * _S1980, _S2046 * _S1980 );
    var _S2048 : DiffPair_float_0 = DiffPair_float_0( 0.65239977836608887f, 0.0f );
    var _S2049 : DiffPair_float_0 = pow0_forward_0(_S2047, _S2048);
    var _S2050 : f32 = 15.93299388885498f * _S2049.primal_0;
    var _S2051 : DiffPair_float_0 = DiffPair_float_0( _S2044.primal_0, _S2044.differential_0 );
    var _S2052 : DiffPair_float_0 = DiffPair_float_0( 0.60075569152832031f, 0.0f );
    var _S2053 : DiffPair_float_0 = pow0_forward_0(_S2051, _S2052);
    var _S2054 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S2045, _S2050 * _S2053.primal_0 / _S1980, _S2041), vec3<f32>(_S2046, (_S2049.differential_0 * 15.93299388885498f * _S2053.primal_0 + _S2053.differential_0 * _S2050) / _S1980, _S2042) );
    var _S2055 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_ucs_jch_to_hsb_0(_S2054);
    var _S2056 : f32 = _S2034.z;
    var _S2057 : f32 = _S2035.z;
    var saturation_6 : f32;
    var s_diff_saturation_0 : f32;
    if((_S2056 > 0.0f))
    {
        var _S2058 : f32 = _S2034.y;
        var _S2059 : f32 = (_S2035.y * _S2056 - _S2058 * _S2057) / (_S2056 * _S2056);
        saturation_6 = _S2058 / _S2056;
        s_diff_saturation_0 = _S2059;
    }
    else
    {
        saturation_6 = 0.0f;
        s_diff_saturation_0 = 0.0f;
    }
    var _S2060 : f32 = _S2055.primal_0.y;
    var _S2061 : f32 = _S2055.differential_0.y;
    var _S2062 : DiffPair_float_0 = DiffPair_float_0( saturation_6, s_diff_saturation_0 );
    var _S2063 : DiffPair_float_0 = DiffPair_float_0( 0.80000001192092896f * _S2060, _S2061 * 0.80000001192092896f );
    var _S2064 : DiffPair_float_0 = DiffPair_float_0( _S2060, _S2061 );
    var _S2065 : DiffPair_float_0 = s_fwd_soft_clip_0(_S2062, _S2063, _S2064);
    var _S2066 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S2034.x, _S2065.primal_0, _S2056), vec3<f32>(_S2035.x, _S2065.differential_0, _S2057) );
    var _S2067 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_ucs_hsb_to_jch_0(_S2066);
    var _S2068 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2067.primal_0, _S2067.differential_0 );
    var _S2069 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_ucs_jch_to_xyy_0(_S2068, _S1980);
    var _S2070 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2069.primal_0, _S2069.differential_0 );
    var _S2071 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_xyy_to_xyz_0(_S2070);
    var _S2072 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2071.primal_0, _S2071.differential_0 );
    return _S2072;
}

fn s_fwd_color_balance_0( _S2073 : vec3<f32>,  _S2074 : DiffPair_Grading_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var input_matrix_0 : mat3x3<f32> = mat3x3<f32>(params_0.color_balance_0.input_r0_0.xyz, params_0.color_balance_0.input_r1_0.xyz, params_0.color_balance_0.input_r2_0.xyz);
    var _S2075 : vec3<f32> = vec3<f32>(0.0f);
    var s_diff_input_matrix_0 : mat3x3<f32> = mat3x3<f32>(_S2075, _S2075, _S2075);
    var output_matrix_3 : mat3x3<f32> = mat3x3<f32>(params_0.color_balance_0.output_r0_0.xyz, params_0.color_balance_0.output_r1_0.xyz, params_0.color_balance_0.output_r2_0.xyz);
    var _S2076 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2073, _S2075 );
    var _S2077 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.0f), _S2075 );
    var _S2078 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_max_vector_3(_S2076, _S2077);
    var _S2079 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( (((_S2078.primal_0) * (input_matrix_0))), (((_S2078.primal_0) * (s_diff_input_matrix_0))) + (((_S2078.differential_0) * (input_matrix_0))) );
    var _S2080 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_lms_to_yrg_0(_S2079);
    var _S2081 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2080.primal_0, _S2080.differential_0 );
    var _S2082 : DiffPair_vectorx3Cfloatx2C4x3E_0 = s_fwd_yrg_to_ych_0(_S2081);
    var _S2083 : DiffPair_float_0 = DiffPair_float_0( _S2082.primal_0.x, _S2082.differential_0.x );
    var _S2084 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S2085 : DiffPair_float_0 = _d_max_1(_S2083, _S2084);
    var _S2086 : vec4<f32> = _S2082.primal_0;
    _S2086[i32(0)] = _S2085.primal_0;
    var _S2087 : vec4<f32> = _S2082.differential_0;
    _S2087[i32(0)] = _S2085.differential_0;
    var _S2088 : f32 = _S2086.x;
    var _S2089 : f32 = _S2087.x;
    var _S2090 : DiffPair_float_0 = DiffPair_float_0( _S2088, _S2089 );
    var _S2091 : DiffPair_float_0 = DiffPair_float_0( 0.41012057662010193f, 0.0f );
    var _S2092 : DiffPair_float_0 = pow0_forward_0(_S2090, _S2091);
    var _S2093 : DiffPair_float_0 = DiffPair_float_0( _S2092.primal_0, _S2092.differential_0 );
    var _S2094 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_opacity_masks_0(_S2093);
    var _S2095 : f32 = _S2086.y;
    var _S2096 : f32 = _S2087.y;
    var _S2097 : f32 = _S2086.z;
    var _S2098 : f32 = _S2087.z;
    var _S2099 : f32 = _S2086.w;
    var _S2100 : f32 = _S2087.w;
    var ych_5 : vec4<f32> = vec4<f32>(_S2088, _S2095, params_0.color_balance_0.hue_cos_0 * _S2097 - params_0.color_balance_0.hue_sin_0 * _S2099, params_0.color_balance_0.hue_sin_0 * _S2097 + params_0.color_balance_0.hue_cos_0 * _S2099);
    var s_diff_ych_0 : vec4<f32> = vec4<f32>(_S2089, _S2096, _S2098 * params_0.color_balance_0.hue_cos_0 - _S2100 * params_0.color_balance_0.hue_sin_0, _S2098 * params_0.color_balance_0.hue_sin_0 + _S2100 * params_0.color_balance_0.hue_cos_0);
    var _S2101 : f32 = params_0.color_balance_0.vibrance_0;
    var vibrance_4 : f32;
    var s_diff_vibrance_0 : f32;
    if((params_0.color_balance_0.vibrance_0) != 0.0f)
    {
        var _S2102 : DiffPair_float_0 = DiffPair_float_0( _S2101, 0.0f );
        var _S2103 : DiffPair_float_0 = _d_abs_0(_S2102);
        var _S2104 : DiffPair_float_0 = DiffPair_float_0( _S2095, _S2096 );
        var _S2105 : DiffPair_float_0 = DiffPair_float_0( _S2103.primal_0, _S2103.differential_0 );
        var _S2106 : DiffPair_float_0 = pow0_forward_0(_S2104, _S2105);
        var _S2107 : f32 = (0.0f - _S2106.differential_0) * _S2101;
        vibrance_4 = _S2101 * (1.0f - _S2106.primal_0);
        s_diff_vibrance_0 = _S2107;
    }
    else
    {
        vibrance_4 = 0.0f;
        s_diff_vibrance_0 = 0.0f;
    }
    var _S2108 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2094.primal_0, _S2094.differential_0 );
    var _S2109 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( params_0.color_balance_0.chroma_0.xyz, _S2075 );
    var _S2110 : DiffPair_float_0 = _d_dot_3(_S2108, _S2109);
    var _S2111 : DiffPair_float_0 = DiffPair_float_0( 1.0f + _S2074.primal_0.chroma_global_1 + _S2110.primal_0 + vibrance_4, _S2074.differential_0.chroma_global_1 + _S2110.differential_0 + s_diff_vibrance_0 );
    var _S2112 : DiffPair_float_0 = _d_max_1(_S2111, _S2084);
    var _S2113 : f32 = _S2095 * _S2112.primal_0;
    var _S2114 : f32 = _S2096 * _S2112.primal_0 + _S2112.differential_0 * _S2095;
    _S2086 = ych_5;
    _S2086[i32(1)] = _S2113;
    _S2087 = s_diff_ych_0;
    _S2087[i32(1)] = _S2114;
    var _S2115 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( _S2086, _S2087 );
    var _S2116 : DiffPair_vectorx3Cfloatx2C4x3E_0 = s_fwd_gamut_check_yrg_0(_S2115);
    var _S2117 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( _S2116.primal_0, _S2116.differential_0 );
    var _S2118 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_ych_to_yrg_0(_S2117);
    var _S2119 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2118.primal_0, _S2118.differential_0 );
    var _S2120 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_yrg_to_lms_0(_S2119);
    var _S2121 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var rgb_6 : vec3<f32> = (((_S2120.primal_0) * (mat3x3<f32>(1.0877193212509155f, -0.66666668653488159f, 0.020618559792637825f, -0.087719298899173737f, 1.6666666269302368f, -0.051546391099691391f, 0.0f, 0.0f, 1.0309278964996338f)))) + params_0.color_balance_0.global_0.xyz;
    var _S2122 : f32 = _S2094.primal_0.z;
    var _S2123 : f32 = _S2094.differential_0.z;
    var _S2124 : f32 = _S2094.primal_0.x;
    var _S2125 : f32 = _S2094.differential_0.x;
    var _S2126 : vec3<f32> = params_0.color_balance_0.shadows_0.xyz;
    var _S2127 : vec3<f32> = vec3<f32>((1.0f - _S2124)) + vec3<f32>(_S2124) * _S2126;
    var _S2128 : vec3<f32> = vec3<f32>((1.0f - _S2122));
    var _S2129 : vec3<f32> = params_0.color_balance_0.highlights_0.xyz;
    var _S2130 : vec3<f32> = _S2128 * _S2127 + vec3<f32>(_S2122) * _S2129;
    var rgb_7 : vec3<f32> = rgb_6 * _S2130;
    var s_diff_rgb_0 : vec3<f32> = ((((_S2120.primal_0) * (_S2121))) + (((_S2120.differential_0) * (mat3x3<f32>(1.0877193212509155f, -0.66666668653488159f, 0.020618559792637825f, -0.087719298899173737f, 1.6666666269302368f, -0.051546391099691391f, 0.0f, 0.0f, 1.0309278964996338f))))) * _S2130 + (vec3<f32>((0.0f - _S2123)) * _S2127 + (vec3<f32>((0.0f - _S2125)) + vec3<f32>(_S2125) * _S2126) * _S2128 + vec3<f32>(_S2123) * _S2129) * rgb_6;
    var _S2131 : f32 = params_0.color_balance_0.white_fulcrum_0;
    if(((rgb_7.x) < 0.0f))
    {
        vibrance_4 = -1.0f;
    }
    else
    {
        vibrance_4 = 1.0f;
    }
    if(((rgb_7.y) < 0.0f))
    {
        s_diff_vibrance_0 = -1.0f;
    }
    else
    {
        s_diff_vibrance_0 = 1.0f;
    }
    var _S2132 : f32;
    if(((rgb_7.z) < 0.0f))
    {
        _S2132 = -1.0f;
    }
    else
    {
        _S2132 = 1.0f;
    }
    var sign_1 : vec3<f32> = vec3<f32>(vibrance_4, s_diff_vibrance_0, _S2132);
    var _S2133 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( rgb_7, s_diff_rgb_0 );
    var _S2134 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_abs_vector_3(_S2133);
    var _S2135 : vec3<f32> = vec3<f32>(_S2131);
    var _S2136 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2134.primal_0 / _S2135, _S2134.differential_0 / _S2135 );
    var _S2137 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( params_0.color_balance_0.midtones_0.xyz, _S2075 );
    var _S2138 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_bounded_pow_1(_S2136, _S2137);
    var rgb_8 : vec3<f32> = sign_1 * _S2138.primal_0 * _S2135;
    var _S2139 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( (((rgb_8) * (mat3x3<f32>(0.94999998807907104f, 0.37999999523162842f, 0.0f, 0.05000000074505806f, 0.62000000476837158f, 0.029999999329447746f, 0.0f, 0.0f, 0.97000002861022949f)))), (((rgb_8) * (_S2121))) + (((_S2138.differential_0 * sign_1 * _S2135) * (mat3x3<f32>(0.94999998807907104f, 0.37999999523162842f, 0.0f, 0.05000000074505806f, 0.62000000476837158f, 0.029999999329447746f, 0.0f, 0.0f, 0.97000002861022949f)))) );
    var _S2140 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_lms_to_yrg_0(_S2139);
    var _S2141 : DiffPair_float_0 = DiffPair_float_0( _S2140.primal_0.x / _S2131, _S2140.differential_0.x / _S2131 );
    var _S2142 : DiffPair_float_0 = _d_max_1(_S2141, _S2084);
    var _S2143 : DiffPair_float_0 = DiffPair_float_0( _S2142.primal_0, _S2142.differential_0 );
    var _S2144 : DiffPair_float_0 = DiffPair_float_0( params_0.color_balance_0.midtones_y_0, 0.0f );
    var _S2145 : DiffPair_float_0 = s_fwd_bounded_pow_0(_S2143, _S2144);
    var _S2146 : f32 = _S2145.primal_0 * _S2131;
    var _S2147 : f32 = _S2145.differential_0 * _S2131;
    var xyz_6 : vec3<f32> = _S2140.primal_0;
    xyz_6[i32(0)] = _S2146;
    var s_diff_xyz_0 : vec3<f32> = _S2140.differential_0;
    s_diff_xyz_0[i32(0)] = _S2147;
    var _S2148 : DiffPair_float_0 = DiffPair_float_0( xyz_6.x / params_0.color_balance_0.grey_fulcrum_0, s_diff_xyz_0.x / params_0.color_balance_0.grey_fulcrum_0 );
    var _S2149 : DiffPair_float_0 = DiffPair_float_0( _S2074.primal_0.contrast_1, _S2074.differential_0.contrast_1 );
    var _S2150 : DiffPair_float_0 = s_fwd_bounded_pow_0(_S2148, _S2149);
    var _S2151 : f32 = _S2150.differential_0 * params_0.color_balance_0.grey_fulcrum_0;
    xyz_6[i32(0)] = params_0.color_balance_0.grey_fulcrum_0 * _S2150.primal_0;
    s_diff_xyz_0[i32(0)] = _S2151;
    var _S2152 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( xyz_6, s_diff_xyz_0 );
    var _S2153 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_yrg_to_lms_0(_S2152);
    var _S2154 : vec3<f32> = (((_S2153.primal_0) * (mat3x3<f32>(1.8079465627670288f, -1.299716591835022f, 0.34785878658294678f, 0.61783957481384277f, 0.39595451951026917f, -0.0410468690097332f, -0.12546959519386292f, 0.20478038489818573f, 1.7427418231964111f))));
    var _S2155 : vec3<f32> = (((_S2153.primal_0) * (_S2121))) + (((_S2153.differential_0) * (mat3x3<f32>(1.8079465627670288f, -1.299716591835022f, 0.34785878658294678f, 0.61783957481384277f, 0.39595451951026917f, -0.0410468690097332f, -0.12546959519386292f, 0.20478038489818573f, 1.7427418231964111f))));
    if((params_0.color_balance_0.saturation_formula_0) == u32(0))
    {
        var _S2156 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2154, _S2155 );
        var _S2157 : DiffPair_Grading_0 = DiffPair_Grading_0( _S2074.primal_0, _S2074.differential_0 );
        var _S2158 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_jzazbz_saturation_0(_S2156, _S2108, _S2157);
        xyz_6 = _S2158.primal_0;
        s_diff_xyz_0 = _S2158.differential_0;
    }
    else
    {
        var _S2159 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2154, _S2155 );
        var _S2160 : DiffPair_Grading_0 = DiffPair_Grading_0( _S2074.primal_0, _S2074.differential_0 );
        var _S2161 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_ucs_saturation_0(_S2159, _S2108, _S2160);
        xyz_6 = _S2161.primal_0;
        s_diff_xyz_0 = _S2161.differential_0;
    }
    var _S2162 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( (((xyz_6) * (output_matrix_3))), (((xyz_6) * (s_diff_input_matrix_0))) + (((s_diff_xyz_0) * (output_matrix_3))) );
    var _S2163 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_max_vector_3(_S2162, _S2077);
    var _S2164 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S2163.primal_0, _S2163.differential_0 );
    return _S2164;
}

@compute
@workgroup_size(16, 16, 1)
fn colorbalancergb_forward_diff(@builtin(global_invocation_id) id_2 : vec3<u32>)
{
    var _S2165 : u32 = id_2.x;
    var _S2166 : bool;
    if((_S2165 >= (params_0.pass_0.width_0)))
    {
        _S2166 = true;
    }
    else
    {
        _S2166 = ((id_2.y) >= (params_0.pass_0.height_0));
    }
    if(_S2166)
    {
        return;
    }
    var i_5 : u32 = id_2.y * params_0.pass_0.width_0 + _S2165;
    var v_0 : vec4<f32> = derivative_in_0[i_5];
    var direction_0 : Grading_0;
    direction_0.chroma_global_1 = v_0.x;
    direction_0.contrast_1 = v_0.y;
    direction_0.saturation_global_1 = v_0.z;
    direction_0.brilliance_global_1 = v_0.w;
    var _S2167 : DiffPair_Grading_0 = DiffPair_Grading_0( grading_0(), direction_0 );
    derivative_out_0[i_5] = vec4<f32>(s_fwd_color_balance_0(input_0[i_5].xyz, _S2167).differential_0, 0.0f);
    return;
}

