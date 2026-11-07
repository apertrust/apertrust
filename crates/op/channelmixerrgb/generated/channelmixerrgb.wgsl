// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/iop/channelmixerrgb.c)
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

// Generated from slang/channelmixerrgb.slang by tools/build-kernels.sh. Do not edit.

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

struct ChannelMixerParams_std140_0
{
    @align(16) to_lms_0_0 : vec4<f32>,
    @align(16) to_lms_1_0 : vec4<f32>,
    @align(16) to_lms_2_0 : vec4<f32>,
    @align(16) mix_0_0 : vec4<f32>,
    @align(16) mix_1_0 : vec4<f32>,
    @align(16) mix_2_0 : vec4<f32>,
    @align(16) cone_0_0 : vec4<f32>,
    @align(16) cone_1_0 : vec4<f32>,
    @align(16) cone_2_0 : vec4<f32>,
    @align(16) to_space_0_0 : vec4<f32>,
    @align(16) to_space_1_0 : vec4<f32>,
    @align(16) to_space_2_0 : vec4<f32>,
    @align(16) from_space_0_0 : vec4<f32>,
    @align(16) from_space_1_0 : vec4<f32>,
    @align(16) from_space_2_0 : vec4<f32>,
    @align(16) to_rgb_0_0 : vec4<f32>,
    @align(16) to_rgb_1_0 : vec4<f32>,
    @align(16) to_rgb_2_0 : vec4<f32>,
    @align(16) d50_0 : vec4<f32>,
    @align(16) saturation_0 : vec4<f32>,
    @align(16) lightness_0 : vec4<f32>,
    @align(16) grey_0 : vec4<f32>,
    @align(16) x_0 : f32,
    @align(4) y_0 : f32,
    @align(8) compression_0 : f32,
    @align(4) clip_min_0 : f32,
    @align(16) adaptation_0 : u32,
    @align(4) apply_grey_0 : u32,
    @align(8) negative_delta_0 : u32,
    @align(4) unused_0 : u32,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) mixer_0 : ChannelMixerParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(0) @group(0) var<storage, read> input_0 : array<vec4<f32>>;

@binding(1) @group(0) var<storage, read_write> output_0 : array<vec4<f32>>;

@binding(8) @group(0) var<storage, read> derivative_in_0 : array<vec4<f32>>;

@binding(9) @group(0) var<storage, read_write> derivative_out_0 : array<vec4<f32>>;

struct s_bwdCallableCtx_sqrt_0
{
     _S1 : f32,
};

struct DiffPair_float_0
{
     primal_0 : f32,
     differential_0 : f32,
};

fn _d_sqrt_0( dpx_0 : ptr<function, DiffPair_float_0>,  dOut_0 : f32)
{
    var _S2 : f32 = 0.5f / sqrt(max(1.00000001168609742e-07f, (*dpx_0).primal_0)) * dOut_0;
    (*dpx_0).primal_0 = (*dpx_0).primal_0;
    (*dpx_0).differential_0 = _S2;
    return;
}

fn s_bwdProp_sqrt_0( ctx_0 : s_bwdCallableCtx_sqrt_0,  d_x_0 : ptr<function, f32>,  d_out_0 : f32)
{
    var _S3 : DiffPair_float_0;
    _S3.primal_0 = ctx_0._S1;
    _d_sqrt_0(&(_S3), d_out_0);
    (*d_x_0) = _S3.differential_0;
    return;
}

struct Tuple_0
{
     value0_0 : f32,
};

fn s_apply_length_0( x_1 : vec3<f32>) -> Tuple_0
{
    var _S4 : Tuple_0 = Tuple_0( length(x_1) );
    return _S4;
}

struct s_bwdCallableCtx_length_0
{
     _S5 : vec3<f32>,
};

struct DiffPair_vectorx3Cfloatx2C3x3E_0
{
     primal_0 : vec3<f32>,
     differential_0 : vec3<f32>,
};

struct s_paramCtx_s_bwdCallableCtx_length_impl_0
{
     value0_1 : vec3<f32>,
};

struct s_bwdCallableCtx_length_impl_0
{
     value0_2 : s_paramCtx_s_bwdCallableCtx_length_impl_0,
};

fn s_bwdProp_length_impl_0( _S6 : s_bwdCallableCtx_length_impl_0,  _S7 : ptr<function, vec3<f32>>,  _S8 : f32)
{
    var _S9 : f32 = _S6.value0_2.value0_1[i32(0)];
    var _S10 : f32 = _S6.value0_2.value0_1[i32(1)];
    var _S11 : f32 = _S6.value0_2.value0_1[i32(2)];
    var _S12 : s_bwdCallableCtx_sqrt_0;
    _S12._S1 = _S9 * _S9 + _S10 * _S10 + _S11 * _S11;
    var _S13 : f32 = 0.0f;
    s_bwdProp_sqrt_0(_S12, &(_S13), _S8);
    var _S14 : f32 = _S6.value0_2.value0_1[i32(2)] * _S13;
    var _S15 : f32 = _S14 + _S14;
    var _S16 : f32 = _S6.value0_2.value0_1[i32(1)] * _S13;
    var _S17 : f32 = _S16 + _S16;
    var _S18 : f32 = _S6.value0_2.value0_1[i32(0)] * _S13;
    var _S19 : f32 = _S18 + _S18;
    var _S20 : vec3<f32> = vec3<f32>(0.0f);
    _S20[i32(2)] = _S15;
    _S20[i32(1)] = _S17;
    _S20[i32(0)] = _S19;
    (*_S7) = _S20;
    return;
}

fn _S21( _S22 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  _S23 : f32)
{
    var _S24 : s_paramCtx_s_bwdCallableCtx_length_impl_0;
    _S24.value0_1 = (*_S22).primal_0;
    var _S25 : s_bwdCallableCtx_length_impl_0 = s_bwdCallableCtx_length_impl_0( _S24 );
    var _S26 : vec3<f32> = (*_S22).differential_0;
    s_bwdProp_length_impl_0(_S25, &(_S26), _S23);
    (*_S22).differential_0 = _S26;
    return;
}

fn s_bwdProp_length_0( ctx_1 : s_bwdCallableCtx_length_0,  d_x_1 : ptr<function, vec3<f32>>,  d_out_1 : f32)
{
    var _S27 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S27.primal_0 = ctx_1._S5;
    _S21(&(_S27), d_out_1);
    (*d_x_1) = _S27.differential_0;
    return;
}

struct Tuple_1
{
     value0_3 : f32,
};

fn s_apply_exp2_0( x_2 : f32) -> Tuple_1
{
    var _S28 : Tuple_1 = Tuple_1( exp2(x_2) );
    return _S28;
}

struct s_bwdCallableCtx_exp2_0
{
     _S29 : f32,
};

fn _d_exp2_0( dpx_1 : ptr<function, DiffPair_float_0>,  dOut_1 : f32)
{
    var _S30 : f32 = exp2((*dpx_1).primal_0) * 0.69314718246459961f * dOut_1;
    (*dpx_1).primal_0 = (*dpx_1).primal_0;
    (*dpx_1).differential_0 = _S30;
    return;
}

fn s_bwdProp_exp2_0( ctx_2 : s_bwdCallableCtx_exp2_0,  d_x_2 : ptr<function, f32>,  d_out_2 : f32)
{
    var _S31 : DiffPair_float_0;
    _S31.primal_0 = ctx_2._S29;
    _d_exp2_0(&(_S31), d_out_2);
    (*d_x_2) = _S31.differential_0;
    return;
}

struct Tuple_2
{
     value0_4 : f32,
};

fn s_apply_log2_0( x_3 : f32) -> Tuple_2
{
    var _S32 : Tuple_2 = Tuple_2( log2(x_3) );
    return _S32;
}

struct s_bwdCallableCtx_log2_0
{
     _S33 : f32,
};

fn _d_log2_0( dpx_2 : ptr<function, DiffPair_float_0>,  dOut_2 : f32)
{
    var _S34 : f32 = 1.0f / ((*dpx_2).primal_0 * 0.69314718246459961f) * dOut_2;
    (*dpx_2).primal_0 = (*dpx_2).primal_0;
    (*dpx_2).differential_0 = _S34;
    return;
}

fn s_bwdProp_log2_0( ctx_3 : s_bwdCallableCtx_log2_0,  d_x_3 : ptr<function, f32>,  d_out_3 : f32)
{
    var _S35 : DiffPair_float_0;
    _S35.primal_0 = ctx_3._S33;
    _d_log2_0(&(_S35), d_out_3);
    (*d_x_3) = _S35.differential_0;
    return;
}

struct Tuple_3
{
     value0_5 : f32,
};

fn s_apply_clamp_0( x_4 : f32,  minBound_0 : f32,  maxBound_0 : f32) -> Tuple_3
{
    var _S36 : Tuple_3 = Tuple_3( clamp(x_4, minBound_0, maxBound_0) );
    return _S36;
}

struct s_bwdCallableCtx_clamp_0
{
     _S37 : f32,
     _S38 : f32,
     _S39 : f32,
};

fn _d_clamp_0( dpx_3 : ptr<function, DiffPair_float_0>,  dpMin_0 : ptr<function, DiffPair_float_0>,  dpMax_0 : ptr<function, DiffPair_float_0>,  dOut_3 : f32)
{
    var _S40 : DiffPair_float_0 = (*dpx_3);
    var _S41 : bool;
    if((((*dpx_3).primal_0) >= ((*dpMin_0).primal_0)))
    {
        _S41 = (((*dpx_3).primal_0) <= ((*dpMax_0).primal_0));
    }
    else
    {
        _S41 = false;
    }
    var _S42 : f32;
    if(_S41)
    {
        _S42 = dOut_3;
    }
    else
    {
        _S42 = 0.0f;
    }
    (*dpx_3).primal_0 = _S40.primal_0;
    (*dpx_3).differential_0 = _S42;
    var _S43 : DiffPair_float_0 = (*dpMin_0);
    if(((_S40.primal_0) < ((*dpMin_0).primal_0)))
    {
        _S42 = dOut_3;
    }
    else
    {
        _S42 = 0.0f;
    }
    (*dpMin_0).primal_0 = _S43.primal_0;
    (*dpMin_0).differential_0 = _S42;
    var _S44 : DiffPair_float_0 = (*dpMax_0);
    if((((*dpx_3).primal_0) > ((*dpMax_0).primal_0)))
    {
        _S42 = dOut_3;
    }
    else
    {
        _S42 = 0.0f;
    }
    (*dpMax_0).primal_0 = _S44.primal_0;
    (*dpMax_0).differential_0 = _S42;
    return;
}

fn s_bwdProp_clamp_0( ctx_4 : s_bwdCallableCtx_clamp_0,  d_x_4 : ptr<function, f32>,  d_minBound_0 : ptr<function, f32>,  d_maxBound_0 : ptr<function, f32>,  d_out_4 : f32)
{
    var _S45 : DiffPair_float_0;
    _S45.primal_0 = ctx_4._S37;
    var _S46 : DiffPair_float_0;
    _S46.primal_0 = ctx_4._S38;
    var _S47 : DiffPair_float_0;
    _S47.primal_0 = ctx_4._S39;
    _d_clamp_0(&(_S45), &(_S46), &(_S47), d_out_4);
    (*d_x_4) = _S45.differential_0;
    (*d_minBound_0) = _S46.differential_0;
    (*d_maxBound_0) = _S47.differential_0;
    return;
}

struct Tuple_4
{
     value0_6 : vec3<f32>,
};

fn s_apply_min_0( x_5 : vec3<f32>,  y_1 : vec3<f32>) -> Tuple_4
{
    var _S48 : Tuple_4 = Tuple_4( min(x_5, y_1) );
    return _S48;
}

struct s_bwdCallableCtx_min_0
{
     _S49 : vec3<f32>,
     _S50 : vec3<f32>,
};

fn _d_min_0( dpx_4 : ptr<function, DiffPair_float_0>,  dpy_0 : ptr<function, DiffPair_float_0>,  dOut_4 : f32)
{
    var _S51 : DiffPair_float_0 = (*dpx_4);
    var _S52 : f32;
    if((((*dpx_4).primal_0) < ((*dpy_0).primal_0)))
    {
        _S52 = dOut_4;
    }
    else
    {
        if((((*dpx_4).primal_0) > ((*dpy_0).primal_0)))
        {
            _S52 = 0.0f;
        }
        else
        {
            _S52 = 0.5f * dOut_4;
        }
    }
    (*dpx_4).primal_0 = _S51.primal_0;
    (*dpx_4).differential_0 = _S52;
    var _S53 : DiffPair_float_0 = (*dpy_0);
    if((((*dpy_0).primal_0) < (_S51.primal_0)))
    {
        _S52 = dOut_4;
    }
    else
    {
        if((((*dpy_0).primal_0) > ((*dpx_4).primal_0)))
        {
            _S52 = 0.0f;
        }
        else
        {
            _S52 = 0.5f * dOut_4;
        }
    }
    (*dpy_0).primal_0 = _S53.primal_0;
    (*dpy_0).differential_0 = _S52;
    return;
}

fn _d_min_vector_0( dpx_5 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dpy_1 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_5 : vec3<f32>)
{
    var left_dp_0 : DiffPair_float_0;
    left_dp_0.primal_0 = (*dpx_5).primal_0[i32(0)];
    left_dp_0.differential_0 = 0.0f;
    var right_dp_0 : DiffPair_float_0;
    right_dp_0.primal_0 = (*dpy_1).primal_0[i32(0)];
    right_dp_0.differential_0 = 0.0f;
    _d_min_0(&(left_dp_0), &(right_dp_0), dOut_5[i32(0)]);
    var left_d_result_0 : vec3<f32>;
    left_d_result_0[i32(0)] = left_dp_0.differential_0;
    var right_d_result_0 : vec3<f32>;
    right_d_result_0[i32(0)] = right_dp_0.differential_0;
    var left_dp_1 : DiffPair_float_0;
    left_dp_1.primal_0 = (*dpx_5).primal_0[i32(1)];
    left_dp_1.differential_0 = 0.0f;
    var right_dp_1 : DiffPair_float_0;
    right_dp_1.primal_0 = (*dpy_1).primal_0[i32(1)];
    right_dp_1.differential_0 = 0.0f;
    _d_min_0(&(left_dp_1), &(right_dp_1), dOut_5[i32(1)]);
    left_d_result_0[i32(1)] = left_dp_1.differential_0;
    right_d_result_0[i32(1)] = right_dp_1.differential_0;
    var left_dp_2 : DiffPair_float_0;
    left_dp_2.primal_0 = (*dpx_5).primal_0[i32(2)];
    left_dp_2.differential_0 = 0.0f;
    var right_dp_2 : DiffPair_float_0;
    right_dp_2.primal_0 = (*dpy_1).primal_0[i32(2)];
    right_dp_2.differential_0 = 0.0f;
    _d_min_0(&(left_dp_2), &(right_dp_2), dOut_5[i32(2)]);
    left_d_result_0[i32(2)] = left_dp_2.differential_0;
    right_d_result_0[i32(2)] = right_dp_2.differential_0;
    (*dpx_5).primal_0 = (*dpx_5).primal_0;
    (*dpx_5).differential_0 = left_d_result_0;
    (*dpy_1).primal_0 = (*dpy_1).primal_0;
    (*dpy_1).differential_0 = right_d_result_0;
    return;
}

fn s_bwdProp_min_0( ctx_5 : s_bwdCallableCtx_min_0,  d_x_5 : ptr<function, vec3<f32>>,  d_y_0 : ptr<function, vec3<f32>>,  d_out_5 : vec3<f32>)
{
    var _S54 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S54.primal_0 = ctx_5._S49;
    var _S55 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S55.primal_0 = ctx_5._S50;
    _d_min_vector_0(&(_S54), &(_S55), d_out_5);
    (*d_x_5) = _S54.differential_0;
    (*d_y_0) = _S55.differential_0;
    return;
}

struct Tuple_5
{
     value0_7 : vec2<f32>,
};

fn s_apply_max_0( x_6 : vec2<f32>,  y_2 : vec2<f32>) -> Tuple_5
{
    var _S56 : Tuple_5 = Tuple_5( max(x_6, y_2) );
    return _S56;
}

struct s_bwdCallableCtx_max_0
{
     _S57 : vec2<f32>,
     _S58 : vec2<f32>,
};

struct DiffPair_vectorx3Cfloatx2C2x3E_0
{
     primal_0 : vec2<f32>,
     differential_0 : vec2<f32>,
};

fn _d_max_0( dpx_6 : ptr<function, DiffPair_float_0>,  dpy_2 : ptr<function, DiffPair_float_0>,  dOut_6 : f32)
{
    var _S59 : DiffPair_float_0 = (*dpx_6);
    var _S60 : f32;
    if((((*dpx_6).primal_0) > ((*dpy_2).primal_0)))
    {
        _S60 = dOut_6;
    }
    else
    {
        if((((*dpx_6).primal_0) < ((*dpy_2).primal_0)))
        {
            _S60 = 0.0f;
        }
        else
        {
            _S60 = 0.5f * dOut_6;
        }
    }
    (*dpx_6).primal_0 = _S59.primal_0;
    (*dpx_6).differential_0 = _S60;
    var _S61 : DiffPair_float_0 = (*dpy_2);
    if((((*dpy_2).primal_0) > (_S59.primal_0)))
    {
        _S60 = dOut_6;
    }
    else
    {
        if((((*dpy_2).primal_0) < ((*dpx_6).primal_0)))
        {
            _S60 = 0.0f;
        }
        else
        {
            _S60 = 0.5f * dOut_6;
        }
    }
    (*dpy_2).primal_0 = _S61.primal_0;
    (*dpy_2).differential_0 = _S60;
    return;
}

fn _d_max_vector_0( dpx_7 : ptr<function, DiffPair_vectorx3Cfloatx2C2x3E_0>,  dpy_3 : ptr<function, DiffPair_vectorx3Cfloatx2C2x3E_0>,  dOut_7 : vec2<f32>)
{
    var left_dp_3 : DiffPair_float_0;
    left_dp_3.primal_0 = (*dpx_7).primal_0[i32(0)];
    left_dp_3.differential_0 = 0.0f;
    var right_dp_3 : DiffPair_float_0;
    right_dp_3.primal_0 = (*dpy_3).primal_0[i32(0)];
    right_dp_3.differential_0 = 0.0f;
    _d_max_0(&(left_dp_3), &(right_dp_3), dOut_7[i32(0)]);
    var left_d_result_1 : vec2<f32>;
    left_d_result_1[i32(0)] = left_dp_3.differential_0;
    var right_d_result_1 : vec2<f32>;
    right_d_result_1[i32(0)] = right_dp_3.differential_0;
    var left_dp_4 : DiffPair_float_0;
    left_dp_4.primal_0 = (*dpx_7).primal_0[i32(1)];
    left_dp_4.differential_0 = 0.0f;
    var right_dp_4 : DiffPair_float_0;
    right_dp_4.primal_0 = (*dpy_3).primal_0[i32(1)];
    right_dp_4.differential_0 = 0.0f;
    _d_max_0(&(left_dp_4), &(right_dp_4), dOut_7[i32(1)]);
    left_d_result_1[i32(1)] = left_dp_4.differential_0;
    right_d_result_1[i32(1)] = right_dp_4.differential_0;
    (*dpx_7).primal_0 = (*dpx_7).primal_0;
    (*dpx_7).differential_0 = left_d_result_1;
    (*dpy_3).primal_0 = (*dpy_3).primal_0;
    (*dpy_3).differential_0 = right_d_result_1;
    return;
}

fn s_bwdProp_max_0( ctx_6 : s_bwdCallableCtx_max_0,  d_x_6 : ptr<function, vec2<f32>>,  d_y_1 : ptr<function, vec2<f32>>,  d_out_6 : vec2<f32>)
{
    var _S62 : DiffPair_vectorx3Cfloatx2C2x3E_0;
    _S62.primal_0 = ctx_6._S57;
    var _S63 : DiffPair_vectorx3Cfloatx2C2x3E_0;
    _S63.primal_0 = ctx_6._S58;
    _d_max_vector_0(&(_S62), &(_S63), d_out_6);
    (*d_x_6) = _S62.differential_0;
    (*d_y_1) = _S63.differential_0;
    return;
}

struct Tuple_6
{
     value0_8 : f32,
};

fn s_apply_dot_0( x_7 : vec2<f32>,  y_3 : vec2<f32>) -> Tuple_6
{
    var _S64 : Tuple_6 = Tuple_6( dot(x_7, y_3) );
    return _S64;
}

struct s_bwdCallableCtx_dot_0
{
     _S65 : vec2<f32>,
     _S66 : vec2<f32>,
};

fn _d_dot_0( dpx_8 : ptr<function, DiffPair_vectorx3Cfloatx2C2x3E_0>,  dpy_4 : ptr<function, DiffPair_vectorx3Cfloatx2C2x3E_0>,  dOut_8 : f32)
{
    var x_d_result_0 : vec2<f32>;
    x_d_result_0[i32(0)] = (*dpy_4).primal_0[i32(0)] * dOut_8;
    var y_d_result_0 : vec2<f32>;
    y_d_result_0[i32(0)] = (*dpx_8).primal_0[i32(0)] * dOut_8;
    x_d_result_0[i32(1)] = (*dpy_4).primal_0[i32(1)] * dOut_8;
    y_d_result_0[i32(1)] = (*dpx_8).primal_0[i32(1)] * dOut_8;
    (*dpx_8).primal_0 = (*dpx_8).primal_0;
    (*dpx_8).differential_0 = x_d_result_0;
    (*dpy_4).primal_0 = (*dpy_4).primal_0;
    (*dpy_4).differential_0 = y_d_result_0;
    return;
}

fn s_bwdProp_dot_0( ctx_7 : s_bwdCallableCtx_dot_0,  d_x_7 : ptr<function, vec2<f32>>,  d_y_2 : ptr<function, vec2<f32>>,  d_out_7 : f32)
{
    var _S67 : DiffPair_vectorx3Cfloatx2C2x3E_0;
    _S67.primal_0 = ctx_7._S65;
    var _S68 : DiffPair_vectorx3Cfloatx2C2x3E_0;
    _S68.primal_0 = ctx_7._S66;
    _d_dot_0(&(_S67), &(_S68), d_out_7);
    (*d_x_7) = _S67.differential_0;
    (*d_y_2) = _S68.differential_0;
    return;
}

struct Tuple_7
{
     value0_9 : f32,
};

fn s_apply_abs_0( x_8 : f32) -> Tuple_7
{
    var _S69 : Tuple_7 = Tuple_7( abs(x_8) );
    return _S69;
}

struct s_bwdCallableCtx_abs_0
{
     _S70 : f32,
};

fn _d_abs_0( dpx_9 : ptr<function, DiffPair_float_0>,  dOut_9 : f32)
{
    var _S71 : f32 = select(select(-1.0f, 0.0f, ((*dpx_9).primal_0) == 0.0f), 1.0f, (((*dpx_9).primal_0) > 0.0f)) * dOut_9;
    (*dpx_9).primal_0 = (*dpx_9).primal_0;
    (*dpx_9).differential_0 = _S71;
    return;
}

fn s_bwdProp_abs_0( ctx_8 : s_bwdCallableCtx_abs_0,  d_x_8 : ptr<function, f32>,  d_out_8 : f32)
{
    var _S72 : DiffPair_float_0;
    _S72.primal_0 = ctx_8._S70;
    _d_abs_0(&(_S72), d_out_8);
    (*d_x_8) = _S72.differential_0;
    return;
}

struct Tuple_8
{
     value0_10 : vec3<f32>,
};

fn s_apply_max_1( x_9 : vec3<f32>,  y_4 : vec3<f32>) -> Tuple_8
{
    var _S73 : Tuple_8 = Tuple_8( max(x_9, y_4) );
    return _S73;
}

struct s_bwdCallableCtx_max_1
{
     _S74 : vec3<f32>,
     _S75 : vec3<f32>,
};

fn _d_max_vector_1( dpx_10 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dpy_5 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_10 : vec3<f32>)
{
    var left_dp_5 : DiffPair_float_0;
    left_dp_5.primal_0 = (*dpx_10).primal_0[i32(0)];
    left_dp_5.differential_0 = 0.0f;
    var right_dp_5 : DiffPair_float_0;
    right_dp_5.primal_0 = (*dpy_5).primal_0[i32(0)];
    right_dp_5.differential_0 = 0.0f;
    _d_max_0(&(left_dp_5), &(right_dp_5), dOut_10[i32(0)]);
    var left_d_result_2 : vec3<f32>;
    left_d_result_2[i32(0)] = left_dp_5.differential_0;
    var right_d_result_2 : vec3<f32>;
    right_d_result_2[i32(0)] = right_dp_5.differential_0;
    var left_dp_6 : DiffPair_float_0;
    left_dp_6.primal_0 = (*dpx_10).primal_0[i32(1)];
    left_dp_6.differential_0 = 0.0f;
    var right_dp_6 : DiffPair_float_0;
    right_dp_6.primal_0 = (*dpy_5).primal_0[i32(1)];
    right_dp_6.differential_0 = 0.0f;
    _d_max_0(&(left_dp_6), &(right_dp_6), dOut_10[i32(1)]);
    left_d_result_2[i32(1)] = left_dp_6.differential_0;
    right_d_result_2[i32(1)] = right_dp_6.differential_0;
    var left_dp_7 : DiffPair_float_0;
    left_dp_7.primal_0 = (*dpx_10).primal_0[i32(2)];
    left_dp_7.differential_0 = 0.0f;
    var right_dp_7 : DiffPair_float_0;
    right_dp_7.primal_0 = (*dpy_5).primal_0[i32(2)];
    right_dp_7.differential_0 = 0.0f;
    _d_max_0(&(left_dp_7), &(right_dp_7), dOut_10[i32(2)]);
    left_d_result_2[i32(2)] = left_dp_7.differential_0;
    right_d_result_2[i32(2)] = right_dp_7.differential_0;
    (*dpx_10).primal_0 = (*dpx_10).primal_0;
    (*dpx_10).differential_0 = left_d_result_2;
    (*dpy_5).primal_0 = (*dpy_5).primal_0;
    (*dpy_5).differential_0 = right_d_result_2;
    return;
}

fn s_bwdProp_max_1( ctx_9 : s_bwdCallableCtx_max_1,  d_x_9 : ptr<function, vec3<f32>>,  d_y_3 : ptr<function, vec3<f32>>,  d_out_9 : vec3<f32>)
{
    var _S76 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S76.primal_0 = ctx_9._S74;
    var _S77 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S77.primal_0 = ctx_9._S75;
    _d_max_vector_1(&(_S76), &(_S77), d_out_9);
    (*d_x_9) = _S76.differential_0;
    (*d_y_3) = _S77.differential_0;
    return;
}

struct Tuple_9
{
     value0_11 : f32,
};

fn s_apply_max_2( x_10 : f32,  y_5 : f32) -> Tuple_9
{
    var _S78 : Tuple_9 = Tuple_9( max(x_10, y_5) );
    return _S78;
}

struct s_bwdCallableCtx_max_2
{
     _S79 : f32,
     _S80 : f32,
};

fn s_bwdProp_max_2( ctx_10 : s_bwdCallableCtx_max_2,  d_x_10 : ptr<function, f32>,  d_y_4 : ptr<function, f32>,  d_out_10 : f32)
{
    var _S81 : DiffPair_float_0;
    _S81.primal_0 = ctx_10._S79;
    var _S82 : DiffPair_float_0;
    _S82.primal_0 = ctx_10._S80;
    _d_max_0(&(_S81), &(_S82), d_out_10);
    (*d_x_10) = _S81.differential_0;
    (*d_y_4) = _S82.differential_0;
    return;
}

struct Tuple_10
{
     value0_12 : f32,
};

fn s_apply_dot_1( x_11 : vec3<f32>,  y_6 : vec3<f32>) -> Tuple_10
{
    var _S83 : Tuple_10 = Tuple_10( dot(x_11, y_6) );
    return _S83;
}

struct s_bwdCallableCtx_dot_1
{
     _S84 : vec3<f32>,
     _S85 : vec3<f32>,
};

fn _d_dot_1( dpx_11 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dpy_6 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_11 : f32)
{
    var x_d_result_1 : vec3<f32>;
    x_d_result_1[i32(0)] = (*dpy_6).primal_0[i32(0)] * dOut_11;
    var y_d_result_1 : vec3<f32>;
    y_d_result_1[i32(0)] = (*dpx_11).primal_0[i32(0)] * dOut_11;
    x_d_result_1[i32(1)] = (*dpy_6).primal_0[i32(1)] * dOut_11;
    y_d_result_1[i32(1)] = (*dpx_11).primal_0[i32(1)] * dOut_11;
    x_d_result_1[i32(2)] = (*dpy_6).primal_0[i32(2)] * dOut_11;
    y_d_result_1[i32(2)] = (*dpx_11).primal_0[i32(2)] * dOut_11;
    (*dpx_11).primal_0 = (*dpx_11).primal_0;
    (*dpx_11).differential_0 = x_d_result_1;
    (*dpy_6).primal_0 = (*dpy_6).primal_0;
    (*dpy_6).differential_0 = y_d_result_1;
    return;
}

fn s_bwdProp_dot_1( ctx_11 : s_bwdCallableCtx_dot_1,  d_x_11 : ptr<function, vec3<f32>>,  d_y_5 : ptr<function, vec3<f32>>,  d_out_11 : f32)
{
    var _S86 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S86.primal_0 = ctx_11._S84;
    var _S87 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S87.primal_0 = ctx_11._S85;
    _d_dot_1(&(_S86), &(_S87), d_out_11);
    (*d_x_11) = _S86.differential_0;
    (*d_y_5) = _S87.differential_0;
    return;
}

struct Tuple_11
{
     value0_13 : f32,
};

fn s_apply_min_1( x_12 : f32,  y_7 : f32) -> Tuple_11
{
    var _S88 : Tuple_11 = Tuple_11( min(x_12, y_7) );
    return _S88;
}

struct s_bwdCallableCtx_min_1
{
     _S89 : f32,
     _S90 : f32,
};

fn s_bwdProp_min_1( ctx_12 : s_bwdCallableCtx_min_1,  d_x_12 : ptr<function, f32>,  d_y_6 : ptr<function, f32>,  d_out_12 : f32)
{
    var _S91 : DiffPair_float_0;
    _S91.primal_0 = ctx_12._S89;
    var _S92 : DiffPair_float_0;
    _S92.primal_0 = ctx_12._S90;
    _d_min_0(&(_S91), &(_S92), d_out_12);
    (*d_x_12) = _S91.differential_0;
    (*d_y_6) = _S92.differential_0;
    return;
}

struct Tuple_12
{
     value0_14 : f32,
};

fn pow0_0( x_13 : f32,  y_8 : f32) -> f32
{
    if((x_13 <= 0.0f))
    {
        return 0.0f;
    }
    return pow(x_13, y_8);
}

fn s_apply_pow0_0( x_14 : f32,  y_9 : f32) -> Tuple_12
{
    var _S93 : Tuple_12 = Tuple_12( pow0_0(x_14, y_9) );
    return _S93;
}

struct s_bwdCallableCtx_pow0_0
{
     _S94 : f32,
     _S95 : f32,
};

fn pow0_backward_0( x_15 : ptr<function, DiffPair_float_0>,  y_10 : ptr<function, DiffPair_float_0>,  d_out_13 : f32)
{
    if((((*x_15).primal_0) <= 0.0f))
    {
        (*x_15).primal_0 = (*x_15).primal_0;
        (*x_15).differential_0 = 0.0f;
        (*y_10).primal_0 = (*y_10).primal_0;
        (*y_10).differential_0 = 0.0f;
        return;
    }
    var d_x_13 : f32 = d_out_13 * (*y_10).primal_0 * pow((*x_15).primal_0, (*y_10).primal_0 - 1.0f);
    var d_y_7 : f32 = d_out_13 * pow((*x_15).primal_0, (*y_10).primal_0) * log((*x_15).primal_0);
    (*x_15).primal_0 = (*x_15).primal_0;
    (*x_15).differential_0 = d_x_13;
    (*y_10).primal_0 = (*y_10).primal_0;
    (*y_10).differential_0 = d_y_7;
    return;
}

fn s_bwdProp_pow0_0( ctx_13 : s_bwdCallableCtx_pow0_0,  d_x_14 : ptr<function, f32>,  d_y_8 : ptr<function, f32>,  d_out_14 : f32)
{
    var _S96 : DiffPair_float_0;
    _S96.primal_0 = ctx_13._S94;
    var _S97 : DiffPair_float_0;
    _S97.primal_0 = ctx_13._S95;
    pow0_backward_0(&(_S96), &(_S97), d_out_14);
    (*d_x_14) = _S96.differential_0;
    (*d_y_8) = _S97.differential_0;
    return;
}

struct Tuple_13
{
     value0_15 : vec3<f32>,
};

fn s_apply_clamp_1( x_16 : vec3<f32>,  minBound_1 : vec3<f32>,  maxBound_1 : vec3<f32>) -> Tuple_13
{
    var _S98 : Tuple_13 = Tuple_13( clamp(x_16, minBound_1, maxBound_1) );
    return _S98;
}

struct s_bwdCallableCtx_clamp_1
{
     _S99 : vec3<f32>,
     _S100 : vec3<f32>,
     _S101 : vec3<f32>,
};

fn _d_clamp_vector_0( dpx_12 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dpy_7 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dpz_0 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_12 : vec3<f32>)
{
    var left_dp_8 : DiffPair_float_0;
    left_dp_8.primal_0 = (*dpx_12).primal_0[i32(0)];
    left_dp_8.differential_0 = 0.0f;
    var middle_dp_0 : DiffPair_float_0;
    middle_dp_0.primal_0 = (*dpy_7).primal_0[i32(0)];
    middle_dp_0.differential_0 = 0.0f;
    var right_dp_8 : DiffPair_float_0;
    right_dp_8.primal_0 = (*dpz_0).primal_0[i32(0)];
    right_dp_8.differential_0 = 0.0f;
    _d_clamp_0(&(left_dp_8), &(middle_dp_0), &(right_dp_8), dOut_12[i32(0)]);
    var left_d_result_3 : vec3<f32>;
    left_d_result_3[i32(0)] = left_dp_8.differential_0;
    var middle_d_result_0 : vec3<f32>;
    middle_d_result_0[i32(0)] = middle_dp_0.differential_0;
    var right_d_result_3 : vec3<f32>;
    right_d_result_3[i32(0)] = right_dp_8.differential_0;
    var left_dp_9 : DiffPair_float_0;
    left_dp_9.primal_0 = (*dpx_12).primal_0[i32(1)];
    left_dp_9.differential_0 = 0.0f;
    var middle_dp_1 : DiffPair_float_0;
    middle_dp_1.primal_0 = (*dpy_7).primal_0[i32(1)];
    middle_dp_1.differential_0 = 0.0f;
    var right_dp_9 : DiffPair_float_0;
    right_dp_9.primal_0 = (*dpz_0).primal_0[i32(1)];
    right_dp_9.differential_0 = 0.0f;
    _d_clamp_0(&(left_dp_9), &(middle_dp_1), &(right_dp_9), dOut_12[i32(1)]);
    left_d_result_3[i32(1)] = left_dp_9.differential_0;
    middle_d_result_0[i32(1)] = middle_dp_1.differential_0;
    right_d_result_3[i32(1)] = right_dp_9.differential_0;
    var left_dp_10 : DiffPair_float_0;
    left_dp_10.primal_0 = (*dpx_12).primal_0[i32(2)];
    left_dp_10.differential_0 = 0.0f;
    var middle_dp_2 : DiffPair_float_0;
    middle_dp_2.primal_0 = (*dpy_7).primal_0[i32(2)];
    middle_dp_2.differential_0 = 0.0f;
    var right_dp_10 : DiffPair_float_0;
    right_dp_10.primal_0 = (*dpz_0).primal_0[i32(2)];
    right_dp_10.differential_0 = 0.0f;
    _d_clamp_0(&(left_dp_10), &(middle_dp_2), &(right_dp_10), dOut_12[i32(2)]);
    left_d_result_3[i32(2)] = left_dp_10.differential_0;
    middle_d_result_0[i32(2)] = middle_dp_2.differential_0;
    right_d_result_3[i32(2)] = right_dp_10.differential_0;
    (*dpx_12).primal_0 = (*dpx_12).primal_0;
    (*dpx_12).differential_0 = left_d_result_3;
    (*dpy_7).primal_0 = (*dpy_7).primal_0;
    (*dpy_7).differential_0 = middle_d_result_0;
    (*dpz_0).primal_0 = (*dpz_0).primal_0;
    (*dpz_0).differential_0 = right_d_result_3;
    return;
}

fn s_bwdProp_clamp_1( ctx_14 : s_bwdCallableCtx_clamp_1,  d_x_15 : ptr<function, vec3<f32>>,  d_minBound_1 : ptr<function, vec3<f32>>,  d_maxBound_1 : ptr<function, vec3<f32>>,  d_out_15 : vec3<f32>)
{
    var _S102 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S102.primal_0 = ctx_14._S99;
    var _S103 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S103.primal_0 = ctx_14._S100;
    var _S104 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S104.primal_0 = ctx_14._S101;
    _d_clamp_vector_0(&(_S102), &(_S103), &(_S104), d_out_15);
    (*d_x_15) = _S102.differential_0;
    (*d_minBound_1) = _S103.differential_0;
    (*d_maxBound_1) = _S104.differential_0;
    return;
}

struct Tuple_14
{
     value0_16 : vec3<f32>,
};

fn s_apply_mul_0( left_0 : mat3x3<f32>,  right_0 : vec3<f32>) -> Tuple_14
{
    var _S105 : Tuple_14 = Tuple_14( (((right_0) * (left_0))) );
    return _S105;
}

struct s_bwdCallableCtx_mul_0
{
     _S106 : mat3x3<f32>,
     _S107 : vec3<f32>,
};

struct DiffPair_matrixx3Cfloatx2C3x2C3x3E_0
{
     primal_0 : mat3x3<f32>,
     differential_0 : mat3x3<f32>,
};

fn _d_mul_0( left_1 : ptr<function, DiffPair_matrixx3Cfloatx2C3x2C3x3E_0>,  right_1 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_13 : vec3<f32>)
{
    var _S108 : f32 = (*left_1).primal_0[i32(0)][i32(0)] * dOut_13[i32(0)];
    var left_d_result_4 : mat3x3<f32>;
    left_d_result_4[i32(0)][i32(0)] = (*right_1).primal_0[i32(0)] * dOut_13[i32(0)];
    var sum_0 : f32 = _S108 + (*left_1).primal_0[i32(1)][i32(0)] * dOut_13[i32(1)];
    left_d_result_4[i32(1)][i32(0)] = (*right_1).primal_0[i32(0)] * dOut_13[i32(1)];
    var sum_1 : f32 = sum_0 + (*left_1).primal_0[i32(2)][i32(0)] * dOut_13[i32(2)];
    left_d_result_4[i32(2)][i32(0)] = (*right_1).primal_0[i32(0)] * dOut_13[i32(2)];
    var right_d_result_4 : vec3<f32>;
    right_d_result_4[i32(0)] = sum_1;
    var _S109 : f32 = (*left_1).primal_0[i32(0)][i32(1)] * dOut_13[i32(0)];
    left_d_result_4[i32(0)][i32(1)] = (*right_1).primal_0[i32(1)] * dOut_13[i32(0)];
    var sum_2 : f32 = _S109 + (*left_1).primal_0[i32(1)][i32(1)] * dOut_13[i32(1)];
    left_d_result_4[i32(1)][i32(1)] = (*right_1).primal_0[i32(1)] * dOut_13[i32(1)];
    var sum_3 : f32 = sum_2 + (*left_1).primal_0[i32(2)][i32(1)] * dOut_13[i32(2)];
    left_d_result_4[i32(2)][i32(1)] = (*right_1).primal_0[i32(1)] * dOut_13[i32(2)];
    right_d_result_4[i32(1)] = sum_3;
    var _S110 : f32 = (*left_1).primal_0[i32(0)][i32(2)] * dOut_13[i32(0)];
    left_d_result_4[i32(0)][i32(2)] = (*right_1).primal_0[i32(2)] * dOut_13[i32(0)];
    var sum_4 : f32 = _S110 + (*left_1).primal_0[i32(1)][i32(2)] * dOut_13[i32(1)];
    left_d_result_4[i32(1)][i32(2)] = (*right_1).primal_0[i32(2)] * dOut_13[i32(1)];
    var sum_5 : f32 = sum_4 + (*left_1).primal_0[i32(2)][i32(2)] * dOut_13[i32(2)];
    left_d_result_4[i32(2)][i32(2)] = (*right_1).primal_0[i32(2)] * dOut_13[i32(2)];
    right_d_result_4[i32(2)] = sum_5;
    (*left_1).primal_0 = (*left_1).primal_0;
    (*left_1).differential_0 = left_d_result_4;
    (*right_1).primal_0 = (*right_1).primal_0;
    (*right_1).differential_0 = right_d_result_4;
    return;
}

fn s_bwdProp_mul_0( ctx_15 : s_bwdCallableCtx_mul_0,  d_left_0 : ptr<function, mat3x3<f32>>,  d_right_0 : ptr<function, vec3<f32>>,  d_out_16 : vec3<f32>)
{
    var _S111 : DiffPair_matrixx3Cfloatx2C3x2C3x3E_0;
    _S111.primal_0 = ctx_15._S106;
    var _S112 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S112.primal_0 = ctx_15._S107;
    _d_mul_0(&(_S111), &(_S112), d_out_16);
    (*d_left_0) = _S111.differential_0;
    (*d_right_0) = _S112.differential_0;
    return;
}

fn _d_sqrt_1( dpx_13 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S113 : DiffPair_float_0 = DiffPair_float_0( sqrt(dpx_13.primal_0), 0.5f / sqrt(max(1.00000001168609742e-07f, dpx_13.primal_0)) * dpx_13.differential_0 );
    return _S113;
}

fn s_fwd_length_impl_0( dpx_14 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_float_0
{
    var _S114 : f32 = dpx_14.primal_0[i32(0)];
    var _S115 : f32 = dpx_14.differential_0[i32(0)] * dpx_14.primal_0[i32(0)];
    var _S116 : f32 = dpx_14.primal_0[i32(1)];
    var _S117 : f32 = dpx_14.differential_0[i32(1)] * dpx_14.primal_0[i32(1)];
    var _S118 : f32 = dpx_14.primal_0[i32(2)];
    var _S119 : f32 = dpx_14.differential_0[i32(2)] * dpx_14.primal_0[i32(2)];
    var _S120 : DiffPair_float_0 = DiffPair_float_0( _S114 * _S114 + _S116 * _S116 + _S118 * _S118, _S115 + _S115 + (_S117 + _S117) + (_S119 + _S119) );
    var _S121 : DiffPair_float_0 = _d_sqrt_1(_S120);
    var _S122 : DiffPair_float_0 = DiffPair_float_0( _S121.primal_0, _S121.differential_0 );
    return _S122;
}

fn _d_clamp_1( dpx_15 : DiffPair_float_0,  dpMin_1 : DiffPair_float_0,  dpMax_1 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S123 : f32 = clamp(dpx_15.primal_0, dpMin_1.primal_0, dpMax_1.primal_0);
    var _S124 : bool;
    if(((dpx_15.primal_0) >= (dpMin_1.primal_0)))
    {
        _S124 = ((dpx_15.primal_0) <= (dpMax_1.primal_0));
    }
    else
    {
        _S124 = false;
    }
    var _S125 : f32;
    if(_S124)
    {
        _S125 = dpx_15.differential_0;
    }
    else
    {
        if(((dpx_15.primal_0) < (dpMin_1.primal_0)))
        {
            _S125 = dpMin_1.differential_0;
        }
        else
        {
            _S125 = dpMax_1.differential_0;
        }
    }
    var _S126 : DiffPair_float_0 = DiffPair_float_0( _S123, _S125 );
    return _S126;
}

fn _d_min_1( dpx_16 : DiffPair_float_0,  dpy_8 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S127 : f32 = min(dpx_16.primal_0, dpy_8.primal_0);
    var _S128 : f32;
    if(((dpx_16.primal_0) < (dpy_8.primal_0)))
    {
        _S128 = dpx_16.differential_0;
    }
    else
    {
        if(((dpx_16.primal_0) > (dpy_8.primal_0)))
        {
            _S128 = dpy_8.differential_0;
        }
        else
        {
            _S128 = 0.5f * (dpx_16.differential_0 + dpy_8.differential_0);
        }
    }
    var _S129 : DiffPair_float_0 = DiffPair_float_0( _S127, _S128 );
    return _S129;
}

fn _d_min_vector_1( dpx_17 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpy_9 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S130 : DiffPair_float_0 = DiffPair_float_0( dpx_17.primal_0[i32(0)], dpx_17.differential_0[i32(0)] );
    var _S131 : DiffPair_float_0 = DiffPair_float_0( dpy_9.primal_0[i32(0)], dpy_9.differential_0[i32(0)] );
    var dp_elem_0 : DiffPair_float_0 = _d_min_1(_S130, _S131);
    var result_0 : vec3<f32>;
    result_0[i32(0)] = dp_elem_0.primal_0;
    var d_result_0 : vec3<f32>;
    d_result_0[i32(0)] = dp_elem_0.differential_0;
    var _S132 : DiffPair_float_0 = DiffPair_float_0( dpx_17.primal_0[i32(1)], dpx_17.differential_0[i32(1)] );
    var _S133 : DiffPair_float_0 = DiffPair_float_0( dpy_9.primal_0[i32(1)], dpy_9.differential_0[i32(1)] );
    var dp_elem_1 : DiffPair_float_0 = _d_min_1(_S132, _S133);
    result_0[i32(1)] = dp_elem_1.primal_0;
    d_result_0[i32(1)] = dp_elem_1.differential_0;
    var _S134 : DiffPair_float_0 = DiffPair_float_0( dpx_17.primal_0[i32(2)], dpx_17.differential_0[i32(2)] );
    var _S135 : DiffPair_float_0 = DiffPair_float_0( dpy_9.primal_0[i32(2)], dpy_9.differential_0[i32(2)] );
    var dp_elem_2 : DiffPair_float_0 = _d_min_1(_S134, _S135);
    result_0[i32(2)] = dp_elem_2.primal_0;
    d_result_0[i32(2)] = dp_elem_2.differential_0;
    var _S136 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( result_0, d_result_0 );
    return _S136;
}

fn _d_dot_2( dpx_18 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpy_10 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_float_0
{
    var _S137 : DiffPair_float_0 = DiffPair_float_0( dpx_18.primal_0[i32(0)] * dpy_10.primal_0[i32(0)] + dpx_18.primal_0[i32(1)] * dpy_10.primal_0[i32(1)] + dpx_18.primal_0[i32(2)] * dpy_10.primal_0[i32(2)], dpx_18.primal_0[i32(0)] * dpy_10.differential_0[i32(0)] + dpy_10.primal_0[i32(0)] * dpx_18.differential_0[i32(0)] + dpx_18.primal_0[i32(1)] * dpy_10.differential_0[i32(1)] + dpy_10.primal_0[i32(1)] * dpx_18.differential_0[i32(1)] + dpx_18.primal_0[i32(2)] * dpy_10.differential_0[i32(2)] + dpy_10.primal_0[i32(2)] * dpx_18.differential_0[i32(2)] );
    return _S137;
}

fn _d_max_1( dpx_19 : DiffPair_float_0,  dpy_11 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S138 : f32 = max(dpx_19.primal_0, dpy_11.primal_0);
    var _S139 : f32;
    if(((dpx_19.primal_0) > (dpy_11.primal_0)))
    {
        _S139 = dpx_19.differential_0;
    }
    else
    {
        if(((dpx_19.primal_0) < (dpy_11.primal_0)))
        {
            _S139 = dpy_11.differential_0;
        }
        else
        {
            _S139 = 0.5f * (dpx_19.differential_0 + dpy_11.differential_0);
        }
    }
    var _S140 : DiffPair_float_0 = DiffPair_float_0( _S138, _S139 );
    return _S140;
}

fn _d_max_vector_2( dpx_20 : DiffPair_vectorx3Cfloatx2C2x3E_0,  dpy_12 : DiffPair_vectorx3Cfloatx2C2x3E_0) -> DiffPair_vectorx3Cfloatx2C2x3E_0
{
    var _S141 : DiffPair_float_0 = DiffPair_float_0( dpx_20.primal_0[i32(0)], dpx_20.differential_0[i32(0)] );
    var _S142 : DiffPair_float_0 = DiffPair_float_0( dpy_12.primal_0[i32(0)], dpy_12.differential_0[i32(0)] );
    var dp_elem_3 : DiffPair_float_0 = _d_max_1(_S141, _S142);
    var result_1 : vec2<f32>;
    result_1[i32(0)] = dp_elem_3.primal_0;
    var d_result_1 : vec2<f32>;
    d_result_1[i32(0)] = dp_elem_3.differential_0;
    var _S143 : DiffPair_float_0 = DiffPair_float_0( dpx_20.primal_0[i32(1)], dpx_20.differential_0[i32(1)] );
    var _S144 : DiffPair_float_0 = DiffPair_float_0( dpy_12.primal_0[i32(1)], dpy_12.differential_0[i32(1)] );
    var dp_elem_4 : DiffPair_float_0 = _d_max_1(_S143, _S144);
    result_1[i32(1)] = dp_elem_4.primal_0;
    d_result_1[i32(1)] = dp_elem_4.differential_0;
    var _S145 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( result_1, d_result_1 );
    return _S145;
}

fn _d_exp2_1( dpx_21 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S146 : f32 = exp2(dpx_21.primal_0);
    var _S147 : DiffPair_float_0 = DiffPair_float_0( _S146, _S146 * 0.69314718246459961f * dpx_21.differential_0 );
    return _S147;
}

fn _d_log2_1( dpx_22 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S148 : DiffPair_float_0 = DiffPair_float_0( log2(dpx_22.primal_0), 1.0f / (dpx_22.primal_0 * 0.69314718246459961f) * dpx_22.differential_0 );
    return _S148;
}

fn _d_dot_3( dpx_23 : DiffPair_vectorx3Cfloatx2C2x3E_0,  dpy_13 : DiffPair_vectorx3Cfloatx2C2x3E_0) -> DiffPair_float_0
{
    var _S149 : DiffPair_float_0 = DiffPair_float_0( dpx_23.primal_0[i32(0)] * dpy_13.primal_0[i32(0)] + dpx_23.primal_0[i32(1)] * dpy_13.primal_0[i32(1)], dpx_23.primal_0[i32(0)] * dpy_13.differential_0[i32(0)] + dpy_13.primal_0[i32(0)] * dpx_23.differential_0[i32(0)] + dpx_23.primal_0[i32(1)] * dpy_13.differential_0[i32(1)] + dpy_13.primal_0[i32(1)] * dpx_23.differential_0[i32(1)] );
    return _S149;
}

fn _d_max_vector_3( dpx_24 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpy_14 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S150 : DiffPair_float_0 = DiffPair_float_0( dpx_24.primal_0[i32(0)], dpx_24.differential_0[i32(0)] );
    var _S151 : DiffPair_float_0 = DiffPair_float_0( dpy_14.primal_0[i32(0)], dpy_14.differential_0[i32(0)] );
    var dp_elem_5 : DiffPair_float_0 = _d_max_1(_S150, _S151);
    var result_2 : vec3<f32>;
    result_2[i32(0)] = dp_elem_5.primal_0;
    var d_result_2 : vec3<f32>;
    d_result_2[i32(0)] = dp_elem_5.differential_0;
    var _S152 : DiffPair_float_0 = DiffPair_float_0( dpx_24.primal_0[i32(1)], dpx_24.differential_0[i32(1)] );
    var _S153 : DiffPair_float_0 = DiffPair_float_0( dpy_14.primal_0[i32(1)], dpy_14.differential_0[i32(1)] );
    var dp_elem_6 : DiffPair_float_0 = _d_max_1(_S152, _S153);
    result_2[i32(1)] = dp_elem_6.primal_0;
    d_result_2[i32(1)] = dp_elem_6.differential_0;
    var _S154 : DiffPair_float_0 = DiffPair_float_0( dpx_24.primal_0[i32(2)], dpx_24.differential_0[i32(2)] );
    var _S155 : DiffPair_float_0 = DiffPair_float_0( dpy_14.primal_0[i32(2)], dpy_14.differential_0[i32(2)] );
    var dp_elem_7 : DiffPair_float_0 = _d_max_1(_S154, _S155);
    result_2[i32(2)] = dp_elem_7.primal_0;
    d_result_2[i32(2)] = dp_elem_7.differential_0;
    var _S156 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( result_2, d_result_2 );
    return _S156;
}

fn _d_clamp_vector_1( dpx_25 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpy_15 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpz_1 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S157 : DiffPair_float_0 = DiffPair_float_0( dpx_25.primal_0[i32(0)], dpx_25.differential_0[i32(0)] );
    var _S158 : DiffPair_float_0 = DiffPair_float_0( dpy_15.primal_0[i32(0)], dpy_15.differential_0[i32(0)] );
    var _S159 : DiffPair_float_0 = DiffPair_float_0( dpz_1.primal_0[i32(0)], dpz_1.differential_0[i32(0)] );
    var dp_elem_8 : DiffPair_float_0 = _d_clamp_1(_S157, _S158, _S159);
    var result_3 : vec3<f32>;
    result_3[i32(0)] = dp_elem_8.primal_0;
    var d_result_3 : vec3<f32>;
    d_result_3[i32(0)] = dp_elem_8.differential_0;
    var _S160 : DiffPair_float_0 = DiffPair_float_0( dpx_25.primal_0[i32(1)], dpx_25.differential_0[i32(1)] );
    var _S161 : DiffPair_float_0 = DiffPair_float_0( dpy_15.primal_0[i32(1)], dpy_15.differential_0[i32(1)] );
    var _S162 : DiffPair_float_0 = DiffPair_float_0( dpz_1.primal_0[i32(1)], dpz_1.differential_0[i32(1)] );
    var dp_elem_9 : DiffPair_float_0 = _d_clamp_1(_S160, _S161, _S162);
    result_3[i32(1)] = dp_elem_9.primal_0;
    d_result_3[i32(1)] = dp_elem_9.differential_0;
    var _S163 : DiffPair_float_0 = DiffPair_float_0( dpx_25.primal_0[i32(2)], dpx_25.differential_0[i32(2)] );
    var _S164 : DiffPair_float_0 = DiffPair_float_0( dpy_15.primal_0[i32(2)], dpy_15.differential_0[i32(2)] );
    var _S165 : DiffPair_float_0 = DiffPair_float_0( dpz_1.primal_0[i32(2)], dpz_1.differential_0[i32(2)] );
    var dp_elem_10 : DiffPair_float_0 = _d_clamp_1(_S163, _S164, _S165);
    result_3[i32(2)] = dp_elem_10.primal_0;
    d_result_3[i32(2)] = dp_elem_10.differential_0;
    var _S166 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( result_3, d_result_3 );
    return _S166;
}

fn _d_abs_1( dpx_26 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S167 : DiffPair_float_0 = DiffPair_float_0( abs(dpx_26.primal_0), select(select(-1.0f, 0.0f, (dpx_26.primal_0) == 0.0f), 1.0f, ((dpx_26.primal_0) > 0.0f)) * dpx_26.differential_0 );
    return _S167;
}

struct Illuminant_0
{
     x_17 : f32,
     y_11 : f32,
};

fn Illuminant_x24_syn_dadd_0( SLANG_anonymous_0_0 : Illuminant_0,  SLANG_anonymous_1_0 : Illuminant_0) -> Illuminant_0
{
    var result_4 : Illuminant_0;
    result_4.x_17 = SLANG_anonymous_0_0.x_17 + SLANG_anonymous_1_0.x_17;
    result_4.y_11 = SLANG_anonymous_0_0.y_11 + SLANG_anonymous_1_0.y_11;
    return result_4;
}

fn Illuminant_x24_syn_dzero_0() -> Illuminant_0
{
    var result_5 : Illuminant_0;
    result_5.x_17 = 0.0f;
    result_5.y_11 = 0.0f;
    return result_5;
}

fn rows_0( r0_0 : vec4<f32>,  r1_0 : vec4<f32>,  r2_0 : vec4<f32>) -> mat3x3<f32>
{
    return mat3x3<f32>(r0_0.xyz, r1_0.xyz, r2_0.xyz);
}

fn illuminant_xy_to_xyz_0( xy_0 : vec2<f32>) -> vec3<f32>
{
    var _S168 : f32 = xy_0.x;
    var _S169 : f32 = xy_0.y;
    return vec3<f32>(_S168 / _S169, 1.0f, (1.0f - _S168 - _S169) / _S169);
}

struct Tuple_15
{
     value0_17 : vec3<f32>,
};

fn s_apply_illuminant_xy_to_xyz_0( _S170 : vec2<f32>) -> Tuple_15
{
    var _S171 : f32 = _S170.x;
    var _S172 : f32 = _S170.y;
    var _S173 : Tuple_15;
    _S173.value0_17 = vec3<f32>(_S171 / _S172, 1.0f, (1.0f - _S171 - _S172) / _S172);
    return _S173;
}

struct s_paramCtx_s_bwdCallableCtx_illuminant_xy_to_xyz_0
{
     value0_18 : vec2<f32>,
};

struct s_bwdCallableCtx_illuminant_xy_to_xyz_0
{
     value0_19 : s_paramCtx_s_bwdCallableCtx_illuminant_xy_to_xyz_0,
};

fn s_bwdProp_illuminant_xy_to_xyz_0( _S174 : s_bwdCallableCtx_illuminant_xy_to_xyz_0,  _S175 : ptr<function, vec2<f32>>,  _S176 : vec3<f32>)
{
    var _S177 : f32 = _S174.value0_19.value0_18.x;
    var _S178 : f32 = _S174.value0_19.value0_18.y;
    var _S179 : f32 = _S178 * _S178;
    var _S180 : f32 = _S176[i32(2)] / _S179;
    var _S181 : f32 = - (_S178 * _S180);
    var _S182 : f32 = _S176[i32(0)] / _S179;
    (*_S175) = vec2<f32>(_S181 + _S178 * _S182, (1.0f - _S177 - _S178) * - _S180 + _S181 + _S177 * - _S182);
    return;
}

fn s_fwd_illuminant_xy_to_xyz_0( dpxy_0 : DiffPair_vectorx3Cfloatx2C2x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S183 : f32 = dpxy_0.primal_0.x;
    var _S184 : f32 = dpxy_0.differential_0.x;
    var _S185 : f32 = dpxy_0.primal_0.y;
    var _S186 : f32 = dpxy_0.differential_0.y;
    var _S187 : f32 = _S185 * _S185;
    var _S188 : f32 = 1.0f - _S183 - _S185;
    var _S189 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S183 / _S185, 1.0f, _S188 / _S185), vec3<f32>((_S184 * _S185 - _S183 * _S186) / _S187, 0.0f, ((0.0f - _S184 - _S186) * _S185 - _S188 * _S186) / _S187) );
    return _S189;
}

fn away_from_zero_0( v_0 : f32,  floor_0 : f32) -> f32
{
    if(((abs(v_0)) < floor_0))
    {
        var _S190 : f32;
        if((v_0 < 0.0f))
        {
            _S190 = - floor_0;
        }
        else
        {
            _S190 = floor_0;
        }
        return _S190;
    }
    return v_0;
}

struct Tuple_16
{
     value0_20 : f32,
};

fn s_apply_away_from_zero_0( _S191 : f32,  floor_1 : f32) -> Tuple_16
{
    var _S192 : f32;
    var _runFlag_0 : bool;
    if(((s_apply_abs_0(_S191).value0_9) < floor_1))
    {
        if((_S191 < 0.0f))
        {
            _S192 = - floor_1;
        }
        else
        {
            _S192 = floor_1;
        }
        _runFlag_0 = false;
    }
    else
    {
        _runFlag_0 = true;
    }
    if(_runFlag_0)
    {
        _S192 = _S191;
    }
    var _S193 : Tuple_16;
    _S193.value0_20 = _S192;
    return _S193;
}

struct s_paramCtx_s_bwdCallableCtx_away_from_zero_0
{
     value0_21 : f32,
     value1_0 : f32,
};

struct s_bwdCallableCtx_away_from_zero_0
{
     value0_22 : s_paramCtx_s_bwdCallableCtx_away_from_zero_0,
};

fn s_bwdProp_away_from_zero_0( _S194 : s_bwdCallableCtx_away_from_zero_0,  _S195 : ptr<function, f32>,  _S196 : f32)
{
    var _S197 : Tuple_7 = s_apply_abs_0(_S194.value0_22.value0_21);
    var _S198 : s_bwdCallableCtx_abs_0;
    _S198._S70 = _S194.value0_22.value0_21;
    var _S199 : s_bwdCallableCtx_abs_0 = _S198;
    var _S200 : f32;
    if(!((_S197.value0_9) < (_S194.value0_22.value1_0)))
    {
        _S200 = _S196;
    }
    else
    {
        _S200 = 0.0f;
    }
    var _S201 : f32 = 0.0f;
    s_bwdProp_abs_0(_S199, &(_S201), 0.0f);
    (*_S195) = _S201 + _S200;
    return;
}

fn s_fwd_away_from_zero_0( dpv_0 : DiffPair_float_0,  floor_2 : f32) -> DiffPair_float_0
{
    var _S202 : DiffPair_float_0 = DiffPair_float_0( dpv_0.primal_0, dpv_0.differential_0 );
    if(((_d_abs_1(_S202).primal_0) < floor_2))
    {
        var _S203 : f32;
        if(((dpv_0.primal_0) < 0.0f))
        {
            _S203 = - floor_2;
        }
        else
        {
            _S203 = floor_2;
        }
        var _S204 : DiffPair_float_0 = DiffPair_float_0( _S203, 0.0f );
        return _S204;
    }
    return _S202;
}

fn away_from_zero_1( v_1 : vec3<f32>,  floor_3 : f32) -> vec3<f32>
{
    return vec3<f32>(away_from_zero_0(v_1.x, floor_3), away_from_zero_0(v_1.y, floor_3), away_from_zero_0(v_1.z, floor_3));
}

struct Tuple_17
{
     value0_23 : vec3<f32>,
};

fn s_apply_away_from_zero_1( _S205 : vec3<f32>,  floor_4 : f32) -> Tuple_17
{
    var _S206 : Tuple_17;
    _S206.value0_23 = vec3<f32>(s_apply_away_from_zero_0(_S205.x, floor_4).value0_20, s_apply_away_from_zero_0(_S205.y, floor_4).value0_20, s_apply_away_from_zero_0(_S205.z, floor_4).value0_20);
    return _S206;
}

struct s_paramCtx_s_bwdCallableCtx_away_from_zero_1
{
     value0_24 : vec3<f32>,
     value1_1 : f32,
};

struct s_bwdCallableCtx_away_from_zero_1
{
     value0_25 : s_paramCtx_s_bwdCallableCtx_away_from_zero_1,
};

fn s_bwdProp_away_from_zero_1( _S207 : s_bwdCallableCtx_away_from_zero_1,  _S208 : ptr<function, vec3<f32>>,  _S209 : vec3<f32>)
{
    var _S210 : s_paramCtx_s_bwdCallableCtx_away_from_zero_0;
    _S210.value0_21 = _S207.value0_25.value0_24.x;
    _S210.value1_0 = _S207.value0_25.value1_1;
    var _S211 : s_bwdCallableCtx_away_from_zero_0 = s_bwdCallableCtx_away_from_zero_0( _S210 );
    var _S212 : s_paramCtx_s_bwdCallableCtx_away_from_zero_0;
    _S212.value0_21 = _S207.value0_25.value0_24.y;
    _S212.value1_0 = _S207.value0_25.value1_1;
    var _S213 : s_bwdCallableCtx_away_from_zero_0 = s_bwdCallableCtx_away_from_zero_0( _S212 );
    var _S214 : s_paramCtx_s_bwdCallableCtx_away_from_zero_0;
    _S214.value0_21 = _S207.value0_25.value0_24.z;
    _S214.value1_0 = _S207.value0_25.value1_1;
    var _S215 : s_bwdCallableCtx_away_from_zero_0 = s_bwdCallableCtx_away_from_zero_0( _S214 );
    var _S216 : f32 = 0.0f;
    s_bwdProp_away_from_zero_0(_S215, &(_S216), _S209[i32(2)]);
    var _S217 : f32 = 0.0f;
    s_bwdProp_away_from_zero_0(_S213, &(_S217), _S209[i32(1)]);
    var _S218 : f32 = 0.0f;
    s_bwdProp_away_from_zero_0(_S211, &(_S218), _S209[i32(0)]);
    (*_S208) = vec3<f32>(_S218, _S217, _S216);
    return;
}

fn s_fwd_away_from_zero_1( dpv_1 : DiffPair_vectorx3Cfloatx2C3x3E_0,  floor_5 : f32) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S219 : DiffPair_float_0 = DiffPair_float_0( dpv_1.primal_0.x, dpv_1.differential_0.x );
    var _S220 : DiffPair_float_0 = s_fwd_away_from_zero_0(_S219, floor_5);
    var _S221 : DiffPair_float_0 = DiffPair_float_0( dpv_1.primal_0.y, dpv_1.differential_0.y );
    var _S222 : DiffPair_float_0 = s_fwd_away_from_zero_0(_S221, floor_5);
    var _S223 : DiffPair_float_0 = DiffPair_float_0( dpv_1.primal_0.z, dpv_1.differential_0.z );
    var _S224 : DiffPair_float_0 = s_fwd_away_from_zero_0(_S223, floor_5);
    var _S225 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S220.primal_0, _S222.primal_0, _S224.primal_0), vec3<f32>(_S220.differential_0, _S222.differential_0, _S224.differential_0) );
    return _S225;
}

fn clip_0( v_2 : vec3<f32>,  floor_6 : f32) -> vec3<f32>
{
    return max(v_2, vec3<f32>(floor_6));
}

struct Tuple_18
{
     value0_26 : vec3<f32>,
};

fn s_apply_clip_0( _S226 : vec3<f32>,  floor_7 : f32) -> Tuple_18
{
    var _S227 : Tuple_18;
    _S227.value0_26 = s_apply_max_1(_S226, vec3<f32>(floor_7)).value0_10;
    return _S227;
}

struct s_paramCtx_s_bwdCallableCtx_clip_0
{
     value0_27 : vec3<f32>,
     value1_2 : f32,
};

struct s_bwdCallableCtx_clip_0
{
     value0_28 : s_paramCtx_s_bwdCallableCtx_clip_0,
};

fn s_bwdProp_clip_0( _S228 : s_bwdCallableCtx_clip_0,  _S229 : ptr<function, vec3<f32>>,  _S230 : vec3<f32>)
{
    var _S231 : vec3<f32> = vec3<f32>(_S228.value0_28.value1_2);
    var _S232 : s_bwdCallableCtx_max_1;
    _S232._S74 = _S228.value0_28.value0_27;
    _S232._S75 = _S231;
    var _S233 : vec3<f32> = vec3<f32>(0.0f);
    var _S234 : vec3<f32> = _S233;
    var _S235 : vec3<f32> = _S233;
    s_bwdProp_max_1(_S232, &(_S234), &(_S235), _S230);
    (*_S229) = _S234;
    return;
}

fn s_fwd_clip_0( dpv_2 : DiffPair_vectorx3Cfloatx2C3x3E_0,  floor_8 : f32) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S236 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( dpv_2.primal_0, dpv_2.differential_0 );
    var _S237 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(floor_8), vec3<f32>(0.0f) );
    var _S238 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_max_vector_3(_S236, _S237);
    var _S239 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S238.primal_0, _S238.differential_0 );
    return _S239;
}

fn pow0_forward_0( x_18 : DiffPair_float_0,  y_12 : DiffPair_float_0) -> DiffPair_float_0
{
    if(((x_18.primal_0) <= 0.0f))
    {
        var _S240 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
        return _S240;
    }
    var value_0 : f32 = pow(x_18.primal_0, y_12.primal_0);
    var _S241 : DiffPair_float_0 = DiffPair_float_0( value_0, x_18.differential_0 * y_12.primal_0 * pow(x_18.primal_0, y_12.primal_0 - 1.0f) + y_12.differential_0 * value_0 * log(x_18.primal_0) );
    return _S241;
}

fn xy_to_uv_0( xy_1 : vec2<f32>) -> vec2<f32>
{
    var _S242 : f32 = xy_1.x;
    var _S243 : f32 = xy_1.y;
    return vec2<f32>(4.0f * _S242, 9.0f * _S243) / vec2<f32>(away_from_zero_0(-2.0f * _S242 + 12.0f * _S243 + 3.0f, 1.52587890625e-05f * (2.0f * abs(_S242) + 12.0f * abs(_S243) + 3.0f)));
}

struct Tuple_19
{
     value0_29 : vec2<f32>,
};

fn s_apply_xy_to_uv_0( _S244 : vec2<f32>) -> Tuple_19
{
    var _S245 : f32 = _S244.x;
    var _S246 : f32 = _S244.y;
    var _S247 : Tuple_19;
    _S247.value0_29 = vec2<f32>(4.0f * _S245, 9.0f * _S246) / vec2<f32>(s_apply_away_from_zero_0(-2.0f * _S245 + 12.0f * _S246 + 3.0f, 1.52587890625e-05f * (2.0f * s_apply_abs_0(_S245).value0_9 + 12.0f * s_apply_abs_0(_S246).value0_9 + 3.0f)).value0_20);
    return _S247;
}

struct s_paramCtx_s_bwdCallableCtx_xy_to_uv_0
{
     value0_30 : vec2<f32>,
};

struct s_bwdCallableCtx_xy_to_uv_0
{
     value0_31 : s_paramCtx_s_bwdCallableCtx_xy_to_uv_0,
};

fn s_bwdProp_xy_to_uv_0( _S248 : s_bwdCallableCtx_xy_to_uv_0,  _S249 : ptr<function, vec2<f32>>,  _S250 : vec2<f32>)
{
    var _S251 : f32 = _S248.value0_31.value0_30.x;
    var _S252 : Tuple_7 = s_apply_abs_0(_S251);
    var _S253 : s_bwdCallableCtx_abs_0;
    _S253._S70 = _S251;
    var _S254 : f32 = 2.0f * _S252.value0_9;
    var _S255 : f32 = _S248.value0_31.value0_30.y;
    var _S256 : Tuple_7 = s_apply_abs_0(_S255);
    var _S257 : s_bwdCallableCtx_abs_0;
    _S257._S70 = _S255;
    var _S258 : vec2<f32> = vec2<f32>(4.0f * _S251, 9.0f * _S255);
    var _S259 : f32 = -2.0f * _S251 + 12.0f * _S255 + 3.0f;
    var _S260 : f32 = 1.52587890625e-05f * (_S254 + 12.0f * _S256.value0_9 + 3.0f);
    var _S261 : Tuple_16 = s_apply_away_from_zero_0(_S259, _S260);
    var _S262 : s_paramCtx_s_bwdCallableCtx_away_from_zero_0;
    _S262.value0_21 = _S259;
    _S262.value1_0 = _S260;
    var _S263 : s_bwdCallableCtx_away_from_zero_0 = s_bwdCallableCtx_away_from_zero_0( _S262 );
    var _S264 : vec2<f32> = _S250 / vec2<f32>((_S261.value0_20 * _S261.value0_20));
    var _S265 : vec2<f32> = _S258 * (vec2<f32>(0) - _S264);
    var _S266 : f32 = _S265[i32(0)] + _S265[i32(1)];
    var _S267 : vec2<f32> = vec2<f32>(_S261.value0_20) * _S264;
    var _S268 : f32 = 0.0f;
    s_bwdProp_away_from_zero_0(_S263, &(_S268), _S266);
    var _S269 : f32 = 12.0f * _S268;
    var _S270 : f32 = -2.0f * _S268;
    var _S271 : f32 = 9.0f * _S267[i32(1)];
    var _S272 : f32 = 4.0f * _S267[i32(0)];
    var _S273 : f32 = 0.0f;
    s_bwdProp_abs_0(_S257, &(_S273), 0.0f);
    var _S274 : f32 = _S269 + _S271 + _S273;
    var _S275 : f32 = 0.0f;
    s_bwdProp_abs_0(_S253, &(_S275), 0.0f);
    (*_S249) = vec2<f32>(_S270 + _S272 + _S275, _S274);
    return;
}

fn s_fwd_xy_to_uv_0( dpxy_1 : DiffPair_vectorx3Cfloatx2C2x3E_0) -> DiffPair_vectorx3Cfloatx2C2x3E_0
{
    var _S276 : f32 = dpxy_1.primal_0.x;
    var _S277 : f32 = dpxy_1.differential_0.x;
    var _S278 : DiffPair_float_0 = DiffPair_float_0( _S276, _S277 );
    var _S279 : f32 = dpxy_1.primal_0.y;
    var _S280 : f32 = dpxy_1.differential_0.y;
    var _S281 : DiffPair_float_0 = DiffPair_float_0( _S279, _S280 );
    var _S282 : vec2<f32> = vec2<f32>(4.0f * _S276, 9.0f * _S279);
    var _S283 : DiffPair_float_0 = DiffPair_float_0( -2.0f * _S276 + 12.0f * _S279 + 3.0f, _S277 * -2.0f + _S280 * 12.0f );
    var _S284 : DiffPair_float_0 = s_fwd_away_from_zero_0(_S283, 1.52587890625e-05f * (2.0f * _d_abs_1(_S278).primal_0 + 12.0f * _d_abs_1(_S281).primal_0 + 3.0f));
    var _S285 : vec2<f32> = vec2<f32>(_S284.primal_0);
    var _S286 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( _S282 / _S285, (vec2<f32>(_S277 * 4.0f, _S280 * 9.0f) * _S285 - _S282 * vec2<f32>(_S284.differential_0)) / vec2<f32>((_S284.primal_0 * _S284.primal_0)) );
    return _S286;
}

fn bounded_power_0( v_3 : f32,  c_0 : f32) -> f32
{
    if((v_3 < 1.0f))
    {
        return pow0_0(v_3, c_0);
    }
    return exp2(min(c_0 * log2(v_3), 20.0f));
}

struct Tuple_20
{
     value0_32 : f32,
};

fn s_apply_bounded_power_0( _S287 : f32,  c_1 : f32) -> Tuple_20
{
    var _S288 : bool = (_S287 < 1.0f);
    var _S289 : f32;
    if(_S288)
    {
        _S289 = s_apply_pow0_0(_S287, c_1).value0_14;
    }
    var _S290 : bool = !_S288;
    if(_S290)
    {
        _S289 = s_apply_exp2_0(s_apply_min_1(c_1 * s_apply_log2_0(_S287).value0_4, 20.0f).value0_13).value0_3;
    }
    var _S291 : Tuple_20;
    _S291.value0_32 = _S289;
    return _S291;
}

struct s_paramCtx_s_bwdCallableCtx_bounded_power_0
{
     value0_33 : f32,
     value1_3 : f32,
};

struct s_bwdCallableCtx_bounded_power_0
{
     value0_34 : s_paramCtx_s_bwdCallableCtx_bounded_power_0,
};

fn s_bwdProp_bounded_power_0( _S292 : s_bwdCallableCtx_bounded_power_0,  _S293 : ptr<function, f32>,  _S294 : f32)
{
    var _S295 : bool = ((_S292.value0_34.value0_33) < 1.0f);
    var _S296 : s_bwdCallableCtx_pow0_0;
    if(_S295)
    {
        var _S297 : s_bwdCallableCtx_pow0_0;
        _S297._S94 = _S292.value0_34.value0_33;
        _S297._S95 = _S292.value0_34.value1_3;
        _S296 = _S297;
    }
    else
    {
        _S296._S94 = 0.0f;
        _S296._S95 = 0.0f;
    }
    var _S298 : bool = !_S295;
    var _S299 : s_bwdCallableCtx_exp2_0;
    var _S300 : s_bwdCallableCtx_min_1;
    var _S301 : s_bwdCallableCtx_log2_0;
    if(_S298)
    {
        var _S302 : Tuple_2 = s_apply_log2_0(_S292.value0_34.value0_33);
        var _S303 : s_bwdCallableCtx_log2_0;
        _S303._S33 = _S292.value0_34.value0_33;
        var _S304 : f32 = _S292.value0_34.value1_3 * _S302.value0_4;
        var _S305 : Tuple_11 = s_apply_min_1(_S304, 20.0f);
        var _S306 : s_bwdCallableCtx_min_1;
        _S306._S89 = _S304;
        _S306._S90 = 20.0f;
        var _S307 : s_bwdCallableCtx_exp2_0;
        _S307._S29 = _S305.value0_13;
        _S299 = _S307;
        _S300 = _S306;
        _S301 = _S303;
    }
    else
    {
        _S299._S29 = 0.0f;
        _S300._S89 = 0.0f;
        _S300._S90 = 0.0f;
        _S301._S33 = 0.0f;
    }
    var _S308 : f32;
    var _S309 : f32;
    if(_S298)
    {
        var _S310 : f32 = 0.0f;
        s_bwdProp_exp2_0(_S299, &(_S310), _S294);
        var _S311 : f32 = 0.0f;
        var _S312 : f32 = 0.0f;
        s_bwdProp_min_1(_S300, &(_S311), &(_S312), _S310);
        var _S313 : f32 = _S292.value0_34.value1_3 * _S311;
        var _S314 : f32 = 0.0f;
        s_bwdProp_log2_0(_S301, &(_S314), _S313);
        _S308 = 0.0f;
        _S309 = _S314;
    }
    else
    {
        _S308 = _S294;
        _S309 = 0.0f;
    }
    if(_S295)
    {
        var _S315 : f32 = 0.0f;
        var _S316 : f32 = 0.0f;
        s_bwdProp_pow0_0(_S296, &(_S315), &(_S316), _S308);
        _S308 = _S315 + _S309;
    }
    else
    {
        _S308 = _S309;
    }
    (*_S293) = _S308;
    return;
}

fn s_fwd_bounded_power_0( dpv_3 : DiffPair_float_0,  c_2 : f32) -> DiffPair_float_0
{
    if(((dpv_3.primal_0) < 1.0f))
    {
        var _S317 : DiffPair_float_0 = DiffPair_float_0( dpv_3.primal_0, dpv_3.differential_0 );
        var _S318 : DiffPair_float_0 = DiffPair_float_0( c_2, 0.0f );
        var _S319 : DiffPair_float_0 = pow0_forward_0(_S317, _S318);
        var _S320 : DiffPair_float_0 = DiffPair_float_0( _S319.primal_0, _S319.differential_0 );
        return _S320;
    }
    var _S321 : DiffPair_float_0 = DiffPair_float_0( dpv_3.primal_0, dpv_3.differential_0 );
    var _S322 : DiffPair_float_0 = _d_log2_1(_S321);
    var _S323 : DiffPair_float_0 = DiffPair_float_0( c_2 * _S322.primal_0, _S322.differential_0 * c_2 );
    var _S324 : DiffPair_float_0 = DiffPair_float_0( 20.0f, 0.0f );
    var _S325 : DiffPair_float_0 = _d_min_1(_S323, _S324);
    var _S326 : DiffPair_float_0 = DiffPair_float_0( _S325.primal_0, _S325.differential_0 );
    var _S327 : DiffPair_float_0 = _d_exp2_1(_S326);
    var _S328 : DiffPair_float_0 = DiffPair_float_0( _S327.primal_0, _S327.differential_0 );
    return _S328;
}

fn towards_white_0( uv_0 : f32,  moved_0 : f32,  white_0 : f32) -> f32
{
    var _S329 : f32;
    if((uv_0 > white_0))
    {
        _S329 = max(moved_0, white_0);
    }
    else
    {
        _S329 = min(moved_0, white_0);
    }
    return _S329;
}

struct Tuple_21
{
     value0_35 : f32,
};

fn s_apply_towards_white_0( _S330 : f32,  _S331 : f32,  white_1 : f32) -> Tuple_21
{
    var _S332 : f32;
    if((_S330 > white_1))
    {
        _S332 = s_apply_max_2(_S331, white_1).value0_11;
    }
    else
    {
        _S332 = s_apply_min_1(_S331, white_1).value0_13;
    }
    var _S333 : Tuple_21;
    _S333.value0_35 = _S332;
    return _S333;
}

struct s_paramCtx_s_bwdCallableCtx_towards_white_0
{
     value0_36 : f32,
     value1_4 : f32,
     value2_0 : f32,
};

struct s_bwdCallableCtx_towards_white_0
{
     value0_37 : s_paramCtx_s_bwdCallableCtx_towards_white_0,
};

fn s_bwdProp_towards_white_0( _S334 : s_bwdCallableCtx_towards_white_0,  _S335 : ptr<function, f32>,  _S336 : ptr<function, f32>,  _S337 : f32)
{
    var _S338 : bool = ((_S334.value0_37.value0_36) > (_S334.value0_37.value2_0));
    var _S339 : s_bwdCallableCtx_max_2;
    var _S340 : s_bwdCallableCtx_min_1;
    if(_S338)
    {
        var _S341 : s_bwdCallableCtx_max_2;
        _S341._S79 = _S334.value0_37.value1_4;
        _S341._S80 = _S334.value0_37.value2_0;
        _S339 = _S341;
        _S340._S89 = 0.0f;
        _S340._S90 = 0.0f;
    }
    else
    {
        var _S342 : s_bwdCallableCtx_min_1;
        _S342._S89 = _S334.value0_37.value1_4;
        _S342._S90 = _S334.value0_37.value2_0;
        _S339._S79 = 0.0f;
        _S339._S80 = 0.0f;
        _S340 = _S342;
    }
    var _S343 : f32;
    if(_S338)
    {
        var _S344 : f32 = 0.0f;
        var _S345 : f32 = 0.0f;
        s_bwdProp_max_2(_S339, &(_S344), &(_S345), _S337);
        _S343 = _S344;
    }
    else
    {
        var _S346 : f32 = 0.0f;
        var _S347 : f32 = 0.0f;
        s_bwdProp_min_1(_S340, &(_S346), &(_S347), _S337);
        _S343 = _S346;
    }
    (*_S335) = 0.0f;
    (*_S336) = _S343;
    return;
}

fn s_fwd_towards_white_0( dpuv_0 : DiffPair_float_0,  dpmoved_0 : DiffPair_float_0,  white_2 : f32) -> DiffPair_float_0
{
    var _S348 : f32;
    var _S349 : f32;
    if(((dpuv_0.primal_0) > white_2))
    {
        var _S350 : DiffPair_float_0 = DiffPair_float_0( dpmoved_0.primal_0, dpmoved_0.differential_0 );
        var _S351 : DiffPair_float_0 = DiffPair_float_0( white_2, 0.0f );
        var _S352 : DiffPair_float_0 = _d_max_1(_S350, _S351);
        _S348 = _S352.primal_0;
        _S349 = _S352.differential_0;
    }
    else
    {
        var _S353 : DiffPair_float_0 = DiffPair_float_0( dpmoved_0.primal_0, dpmoved_0.differential_0 );
        var _S354 : DiffPair_float_0 = DiffPair_float_0( white_2, 0.0f );
        var _S355 : DiffPair_float_0 = _d_min_1(_S353, _S354);
        _S348 = _S355.primal_0;
        _S349 = _S355.differential_0;
    }
    var _S356 : DiffPair_float_0 = DiffPair_float_0( _S348, _S349 );
    return _S356;
}

fn uv_to_xy_0( uv_1 : vec2<f32>) -> vec2<f32>
{
    var _S357 : f32 = uv_1.x;
    var _S358 : f32 = uv_1.y;
    return vec2<f32>(9.0f * _S357, 4.0f * _S358) / vec2<f32>(away_from_zero_0(6.0f * _S357 - 16.0f * _S358 + 12.0f, 1.52587890625e-05f * (6.0f * abs(_S357) + 16.0f * abs(_S358) + 12.0f)));
}

struct Tuple_22
{
     value0_38 : vec2<f32>,
};

fn s_apply_uv_to_xy_0( _S359 : vec2<f32>) -> Tuple_22
{
    var _S360 : f32 = _S359.x;
    var _S361 : f32 = _S359.y;
    var _S362 : Tuple_22;
    _S362.value0_38 = vec2<f32>(9.0f * _S360, 4.0f * _S361) / vec2<f32>(s_apply_away_from_zero_0(6.0f * _S360 - 16.0f * _S361 + 12.0f, 1.52587890625e-05f * (6.0f * s_apply_abs_0(_S360).value0_9 + 16.0f * s_apply_abs_0(_S361).value0_9 + 12.0f)).value0_20);
    return _S362;
}

struct s_paramCtx_s_bwdCallableCtx_uv_to_xy_0
{
     value0_39 : vec2<f32>,
};

struct s_bwdCallableCtx_uv_to_xy_0
{
     value0_40 : s_paramCtx_s_bwdCallableCtx_uv_to_xy_0,
};

fn s_bwdProp_uv_to_xy_0( _S363 : s_bwdCallableCtx_uv_to_xy_0,  _S364 : ptr<function, vec2<f32>>,  _S365 : vec2<f32>)
{
    var _S366 : f32 = _S363.value0_40.value0_39.x;
    var _S367 : Tuple_7 = s_apply_abs_0(_S366);
    var _S368 : s_bwdCallableCtx_abs_0;
    _S368._S70 = _S366;
    var _S369 : f32 = 6.0f * _S367.value0_9;
    var _S370 : f32 = _S363.value0_40.value0_39.y;
    var _S371 : Tuple_7 = s_apply_abs_0(_S370);
    var _S372 : s_bwdCallableCtx_abs_0;
    _S372._S70 = _S370;
    var _S373 : vec2<f32> = vec2<f32>(9.0f * _S366, 4.0f * _S370);
    var _S374 : f32 = 6.0f * _S366 - 16.0f * _S370 + 12.0f;
    var _S375 : f32 = 1.52587890625e-05f * (_S369 + 16.0f * _S371.value0_9 + 12.0f);
    var _S376 : Tuple_16 = s_apply_away_from_zero_0(_S374, _S375);
    var _S377 : s_paramCtx_s_bwdCallableCtx_away_from_zero_0;
    _S377.value0_21 = _S374;
    _S377.value1_0 = _S375;
    var _S378 : s_bwdCallableCtx_away_from_zero_0 = s_bwdCallableCtx_away_from_zero_0( _S377 );
    var _S379 : vec2<f32> = _S365 / vec2<f32>((_S376.value0_20 * _S376.value0_20));
    var _S380 : vec2<f32> = _S373 * (vec2<f32>(0) - _S379);
    var _S381 : f32 = _S380[i32(0)] + _S380[i32(1)];
    var _S382 : vec2<f32> = vec2<f32>(_S376.value0_20) * _S379;
    var _S383 : f32 = 0.0f;
    s_bwdProp_away_from_zero_0(_S378, &(_S383), _S381);
    var _S384 : f32 = 16.0f * - _S383;
    var _S385 : f32 = 6.0f * _S383;
    var _S386 : f32 = 4.0f * _S382[i32(1)];
    var _S387 : f32 = 9.0f * _S382[i32(0)];
    var _S388 : f32 = 0.0f;
    s_bwdProp_abs_0(_S372, &(_S388), 0.0f);
    var _S389 : f32 = _S384 + _S386 + _S388;
    var _S390 : f32 = 0.0f;
    s_bwdProp_abs_0(_S368, &(_S390), 0.0f);
    (*_S364) = vec2<f32>(_S385 + _S387 + _S390, _S389);
    return;
}

fn s_fwd_uv_to_xy_0( dpuv_1 : DiffPair_vectorx3Cfloatx2C2x3E_0) -> DiffPair_vectorx3Cfloatx2C2x3E_0
{
    var _S391 : f32 = dpuv_1.primal_0.x;
    var _S392 : f32 = dpuv_1.differential_0.x;
    var _S393 : DiffPair_float_0 = DiffPair_float_0( _S391, _S392 );
    var _S394 : f32 = dpuv_1.primal_0.y;
    var _S395 : f32 = dpuv_1.differential_0.y;
    var _S396 : DiffPair_float_0 = DiffPair_float_0( _S394, _S395 );
    var _S397 : vec2<f32> = vec2<f32>(9.0f * _S391, 4.0f * _S394);
    var _S398 : DiffPair_float_0 = DiffPair_float_0( 6.0f * _S391 - 16.0f * _S394 + 12.0f, _S392 * 6.0f - _S395 * 16.0f );
    var _S399 : DiffPair_float_0 = s_fwd_away_from_zero_0(_S398, 1.52587890625e-05f * (6.0f * _d_abs_1(_S393).primal_0 + 16.0f * _d_abs_1(_S396).primal_0 + 12.0f));
    var _S400 : vec2<f32> = vec2<f32>(_S399.primal_0);
    var _S401 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( _S397 / _S400, (vec2<f32>(_S392 * 9.0f, _S395 * 4.0f) * _S400 - _S397 * vec2<f32>(_S399.differential_0)) / vec2<f32>((_S399.primal_0 * _S399.primal_0)) );
    return _S401;
}

struct Tuple_23
{
     value0_41 : vec3<f32>,
};

struct ChannelMixerParams_0
{
     to_lms_0_0 : vec4<f32>,
     to_lms_1_0 : vec4<f32>,
     to_lms_2_0 : vec4<f32>,
     mix_0_0 : vec4<f32>,
     mix_1_0 : vec4<f32>,
     mix_2_0 : vec4<f32>,
     cone_0_0 : vec4<f32>,
     cone_1_0 : vec4<f32>,
     cone_2_0 : vec4<f32>,
     to_space_0_0 : vec4<f32>,
     to_space_1_0 : vec4<f32>,
     to_space_2_0 : vec4<f32>,
     from_space_0_0 : vec4<f32>,
     from_space_1_0 : vec4<f32>,
     from_space_2_0 : vec4<f32>,
     to_rgb_0_0 : vec4<f32>,
     to_rgb_1_0 : vec4<f32>,
     to_rgb_2_0 : vec4<f32>,
     d50_0 : vec4<f32>,
     saturation_0 : vec4<f32>,
     lightness_0 : vec4<f32>,
     grey_0 : vec4<f32>,
     x_0 : f32,
     y_0 : f32,
     compression_0 : f32,
     clip_min_0 : f32,
     adaptation_0 : u32,
     apply_grey_0 : u32,
     negative_delta_0 : u32,
     unused_0 : u32,
};

fn s_apply_gamut_mapping_0( _S402 : vec3<f32>,  p_0 : ChannelMixerParams_0) -> Tuple_23
{
    var _S403 : f32 = _S402.x;
    var _S404 : f32 = _S402.y;
    var _S405 : f32 = _S402.z;
    var sum_6 : f32 = _S403 + _S404 + _S405;
    var _S406 : Tuple_9 = s_apply_max_2(sum_6, 9.99999997475242708e-07f * (s_apply_abs_0(_S403).value0_9 + s_apply_abs_0(_S404).value0_9 + s_apply_abs_0(_S405).value0_9));
    var xy_2 : vec2<f32>;
    if((sum_6 > 0.0f))
    {
        xy_2 = vec2<f32>(_S403, _S404) / vec2<f32>(_S406.value0_11);
    }
    else
    {
        xy_2 = vec2<f32>(0.34567001461982727f, 0.35850000381469727f);
    }
    var _S407 : Tuple_19 = s_apply_xy_to_uv_0(xy_2);
    var delta_0 : vec2<f32> = vec2<f32>(0.20915915071964264f, 0.48807531595230103f) - _S407.value0_29;
    var distance_0 : f32 = _S404 * s_apply_dot_0(delta_0, delta_0).value0_8;
    var correction_0 : f32;
    var neutral_0 : bool;
    if((p_0.compression_0) != 0.0f)
    {
        if((distance_0 >= 0.0f))
        {
            correction_0 = s_apply_bounded_power_0(distance_0, p_0.compression_0).value0_32;
            neutral_0 = false;
        }
        else
        {
            var _S408 : Tuple_20 = s_apply_bounded_power_0(- distance_0, p_0.compression_0);
            if((p_0.negative_delta_0) == u32(1))
            {
                correction_0 = - _S408.value0_32;
            }
            else
            {
                correction_0 = _S408.value0_32;
            }
            neutral_0 = (p_0.negative_delta_0) == u32(0);
        }
    }
    else
    {
        correction_0 = 0.0f;
        neutral_0 = false;
    }
    var moved_1 : vec2<f32> = vec2<f32>(s_apply_min_1(correction_0, 1.0f).value0_13) * delta_0 + _S407.value0_29;
    var uv_2 : vec2<f32> = vec2<f32>(s_apply_towards_white_0(_S407.value0_29.x, moved_1.x, 0.20915915071964264f).value0_35, s_apply_towards_white_0(_S407.value0_29.y, moved_1.y, 0.48807531595230103f).value0_35);
    var uv_3 : vec2<f32>;
    if(neutral_0)
    {
        uv_3 = vec2<f32>(0.20915915071964264f, 0.48807531595230103f);
    }
    else
    {
        uv_3 = uv_2;
    }
    var _S409 : Tuple_5 = s_apply_max_0(s_apply_uv_to_xy_0(uv_3).value0_38, vec2<f32>(p_0.clip_min_0));
    var _S410 : f32 = _S409.value0_7.x;
    var _S411 : Tuple_9 = s_apply_max_2(_S409.value0_7.y, 1.52587890625e-05f);
    var xy_3 : vec2<f32> = vec2<f32>(_S410, _S411.value0_11);
    var scale_1 : f32 = _S410 + _S411.value0_11;
    if((scale_1 >= 1.0f))
    {
        xy_2 = xy_3 / vec2<f32>(scale_1);
    }
    else
    {
        xy_2 = xy_3;
    }
    var _S412 : f32 = xy_2.x;
    var _S413 : f32 = xy_2.y;
    var _S414 : Tuple_23;
    _S414.value0_41 = vec3<f32>(_S404 * _S412 / _S413, _S404, _S404 * (1.0f - _S412 - _S413) / _S413);
    return _S414;
}

struct s_paramCtx_s_bwdCallableCtx_gamut_mapping_0
{
     value0_42 : vec3<f32>,
     value1_5 : ChannelMixerParams_0,
};

struct s_bwdCallableCtx_gamut_mapping_0
{
     value0_43 : s_paramCtx_s_bwdCallableCtx_gamut_mapping_0,
};

fn s_bwdProp_gamut_mapping_0( _S415 : s_bwdCallableCtx_gamut_mapping_0,  _S416 : ptr<function, vec3<f32>>,  _S417 : vec3<f32>)
{
    var _S418 : vec2<f32> = vec2<f32>(0.0f);
    var _S419 : s_paramCtx_s_bwdCallableCtx_bounded_power_0 = s_paramCtx_s_bwdCallableCtx_bounded_power_0( 0.0f, 0.0f );
    var _S420 : f32 = _S415.value0_43.value0_42.x;
    var _S421 : f32 = _S415.value0_43.value0_42.y;
    var _S422 : f32 = _S415.value0_43.value0_42.z;
    var sum_7 : f32 = _S420 + _S421 + _S422;
    var _S423 : Tuple_7 = s_apply_abs_0(_S420);
    var _S424 : s_bwdCallableCtx_abs_0;
    _S424._S70 = _S420;
    var _S425 : s_bwdCallableCtx_abs_0 = _S424;
    var _S426 : Tuple_7 = s_apply_abs_0(_S421);
    var _S427 : s_bwdCallableCtx_abs_0;
    _S427._S70 = _S421;
    var _S428 : s_bwdCallableCtx_abs_0 = _S427;
    var _S429 : f32 = _S423.value0_9 + _S426.value0_9;
    var _S430 : Tuple_7 = s_apply_abs_0(_S422);
    var _S431 : s_bwdCallableCtx_abs_0;
    _S431._S70 = _S422;
    var _S432 : s_bwdCallableCtx_abs_0 = _S431;
    var _S433 : f32 = 9.99999997475242708e-07f * (_S429 + _S430.value0_9);
    var _S434 : Tuple_9 = s_apply_max_2(sum_7, _S433);
    var _S435 : s_bwdCallableCtx_max_2;
    _S435._S79 = sum_7;
    _S435._S80 = _S433;
    var _S436 : s_bwdCallableCtx_max_2 = _S435;
    var _S437 : bool = (sum_7 > 0.0f);
    var xy_4 : vec2<f32>;
    var _S438 : vec2<f32>;
    var _S439 : f32;
    if(_S437)
    {
        var _S440 : vec2<f32> = vec2<f32>(_S420, _S421);
        var _S441 : f32 = _S434.value0_11 * _S434.value0_11;
        xy_4 = _S440 / vec2<f32>(_S434.value0_11);
        _S439 = _S441;
        _S438 = _S440;
    }
    else
    {
        xy_4 = vec2<f32>(0.34567001461982727f, 0.35850000381469727f);
        _S439 = 0.0f;
        _S438 = _S418;
    }
    var _S442 : Tuple_19 = s_apply_xy_to_uv_0(xy_4);
    var _S443 : s_paramCtx_s_bwdCallableCtx_xy_to_uv_0;
    _S443.value0_30 = xy_4;
    var _S444 : s_bwdCallableCtx_xy_to_uv_0 = s_bwdCallableCtx_xy_to_uv_0( _S443 );
    var delta_1 : vec2<f32> = vec2<f32>(0.20915915071964264f, 0.48807531595230103f) - _S442.value0_29;
    var _S445 : Tuple_6 = s_apply_dot_0(delta_1, delta_1);
    var _S446 : s_bwdCallableCtx_dot_0;
    _S446._S65 = delta_1;
    _S446._S66 = delta_1;
    var _S447 : s_bwdCallableCtx_dot_0 = _S446;
    var distance_1 : f32 = _S421 * _S445.value0_8;
    var _S448 : bool = (_S415.value0_43.value1_5.compression_0) != 0.0f;
    var correction_1 : f32;
    var neutral_1 : bool;
    var _S449 : bool;
    var _S450 : bool;
    var _S451 : s_bwdCallableCtx_bounded_power_0;
    var _S452 : s_bwdCallableCtx_bounded_power_0;
    if(_S448)
    {
        var _S453 : bool = (distance_1 >= 0.0f);
        if(_S453)
        {
            var _S454 : Tuple_20 = s_apply_bounded_power_0(distance_1, _S415.value0_43.value1_5.compression_0);
            var _S455 : s_paramCtx_s_bwdCallableCtx_bounded_power_0;
            _S455.value0_33 = distance_1;
            _S455.value1_3 = _S415.value0_43.value1_5.compression_0;
            correction_1 = _S454.value0_32;
            neutral_1 = false;
            _S451.value0_34 = _S455;
            _S449 = false;
            _S452.value0_34 = _S419;
        }
        else
        {
            var _S456 : f32 = - distance_1;
            var _S457 : Tuple_20 = s_apply_bounded_power_0(_S456, _S415.value0_43.value1_5.compression_0);
            var _S458 : s_paramCtx_s_bwdCallableCtx_bounded_power_0;
            _S458.value0_33 = _S456;
            _S458.value1_3 = _S415.value0_43.value1_5.compression_0;
            var _S459 : s_paramCtx_s_bwdCallableCtx_bounded_power_0 = _S458;
            var _S460 : bool = (_S415.value0_43.value1_5.negative_delta_0) == u32(1);
            if(_S460)
            {
                correction_1 = - _S457.value0_32;
            }
            else
            {
                correction_1 = _S457.value0_32;
            }
            neutral_1 = (_S415.value0_43.value1_5.negative_delta_0) == u32(0);
            _S451.value0_34 = _S419;
            _S449 = _S460;
            _S452.value0_34 = _S459;
        }
        var _S461 : bool = _S449;
        _S449 = _S453;
        _S450 = _S461;
    }
    else
    {
        correction_1 = 0.0f;
        neutral_1 = false;
        _S449 = false;
        _S451.value0_34 = _S419;
        _S450 = false;
        _S452.value0_34 = _S419;
    }
    var _S462 : Tuple_11 = s_apply_min_1(correction_1, 1.0f);
    var _S463 : s_bwdCallableCtx_min_1;
    _S463._S89 = correction_1;
    _S463._S90 = 1.0f;
    var _S464 : s_bwdCallableCtx_min_1 = _S463;
    var _S465 : vec2<f32> = vec2<f32>(_S462.value0_13);
    var moved_2 : vec2<f32> = _S465 * delta_1 + _S442.value0_29;
    var _S466 : f32 = _S442.value0_29.x;
    var _S467 : f32 = moved_2.x;
    var _S468 : Tuple_21 = s_apply_towards_white_0(_S466, _S467, 0.20915915071964264f);
    var _S469 : s_paramCtx_s_bwdCallableCtx_towards_white_0;
    _S469.value0_36 = _S466;
    _S469.value1_4 = _S467;
    _S469.value2_0 = 0.20915915071964264f;
    var _S470 : s_bwdCallableCtx_towards_white_0 = s_bwdCallableCtx_towards_white_0( _S469 );
    var _S471 : f32 = _S442.value0_29.y;
    var _S472 : f32 = moved_2.y;
    var _S473 : Tuple_21 = s_apply_towards_white_0(_S471, _S472, 0.48807531595230103f);
    var _S474 : s_paramCtx_s_bwdCallableCtx_towards_white_0;
    _S474.value0_36 = _S471;
    _S474.value1_4 = _S472;
    _S474.value2_0 = 0.48807531595230103f;
    var _S475 : s_bwdCallableCtx_towards_white_0 = s_bwdCallableCtx_towards_white_0( _S474 );
    var uv_4 : vec2<f32> = vec2<f32>(_S468.value0_35, _S473.value0_35);
    var uv_5 : vec2<f32>;
    if(neutral_1)
    {
        uv_5 = vec2<f32>(0.20915915071964264f, 0.48807531595230103f);
    }
    else
    {
        uv_5 = uv_4;
    }
    var _S476 : Tuple_22 = s_apply_uv_to_xy_0(uv_5);
    var _S477 : s_paramCtx_s_bwdCallableCtx_uv_to_xy_0;
    _S477.value0_39 = uv_5;
    var _S478 : s_bwdCallableCtx_uv_to_xy_0 = s_bwdCallableCtx_uv_to_xy_0( _S477 );
    var _S479 : vec2<f32> = vec2<f32>(_S415.value0_43.value1_5.clip_min_0);
    var _S480 : Tuple_5 = s_apply_max_0(_S476.value0_38, _S479);
    var _S481 : s_bwdCallableCtx_max_0;
    _S481._S57 = _S476.value0_38;
    _S481._S58 = _S479;
    var _S482 : s_bwdCallableCtx_max_0 = _S481;
    var _S483 : f32 = _S480.value0_7.x;
    var _S484 : f32 = _S480.value0_7.y;
    var _S485 : Tuple_9 = s_apply_max_2(_S484, 1.52587890625e-05f);
    var _S486 : s_bwdCallableCtx_max_2;
    _S486._S79 = _S484;
    _S486._S80 = 1.52587890625e-05f;
    var _S487 : s_bwdCallableCtx_max_2 = _S486;
    var xy_5 : vec2<f32> = vec2<f32>(_S483, _S485.value0_11);
    var scale_2 : f32 = _S483 + _S485.value0_11;
    var _S488 : bool = (scale_2 >= 1.0f);
    if(_S488)
    {
        var _S489 : f32 = scale_2 * scale_2;
        xy_4 = xy_5 / vec2<f32>(scale_2);
        correction_1 = _S489;
    }
    else
    {
        xy_4 = xy_5;
        correction_1 = 0.0f;
    }
    var _S490 : f32 = xy_4.x;
    var _S491 : f32 = _S421 * _S490;
    var _S492 : f32 = xy_4.y;
    var _S493 : f32 = _S492 * _S492;
    var _S494 : f32 = 1.0f - _S490 - _S492;
    var _S495 : f32 = _S421 * _S494;
    var _S496 : f32 = _S417[i32(2)] / _S493;
    var _S497 : f32 = _S492 * _S496;
    var _S498 : f32 = - (_S421 * _S497);
    var _S499 : f32 = _S417[i32(0)] / _S493;
    var _S500 : f32 = _S492 * _S499;
    var _S501 : vec2<f32> = vec2<f32>(_S498 + _S421 * _S500, _S495 * - _S496 + _S498 + _S491 * - _S499);
    var _S502 : f32 = _S417[i32(1)] + _S494 * _S497 + _S490 * _S500;
    if(_S488)
    {
        var _S503 : vec2<f32> = _S501 / vec2<f32>(correction_1);
        var _S504 : vec2<f32> = xy_5 * (vec2<f32>(0) - _S503);
        var _S505 : vec2<f32> = vec2<f32>(scale_2) * _S503;
        correction_1 = _S504[i32(0)] + _S504[i32(1)];
        xy_4 = _S505;
    }
    else
    {
        correction_1 = 0.0f;
        xy_4 = _S501;
    }
    var _S506 : f32 = correction_1 + xy_4[i32(1)];
    var _S507 : f32 = 0.0f;
    var _S508 : f32 = 0.0f;
    s_bwdProp_max_2(_S487, &(_S507), &(_S508), _S506);
    var _S509 : vec2<f32> = vec2<f32>(correction_1 + xy_4[i32(0)], _S507);
    var _S510 : vec2<f32> = _S418;
    var _S511 : vec2<f32> = _S418;
    s_bwdProp_max_0(_S482, &(_S510), &(_S511), _S509);
    var _S512 : vec2<f32> = _S418;
    s_bwdProp_uv_to_xy_0(_S478, &(_S512), _S510);
    var _S513 : vec2<f32> = _S512;
    if(neutral_1)
    {
        xy_4 = _S418;
    }
    else
    {
        xy_4 = _S513;
    }
    var _S514 : f32 = 0.0f;
    var _S515 : f32 = 0.0f;
    s_bwdProp_towards_white_0(_S475, &(_S514), &(_S515), xy_4[i32(1)]);
    var _S516 : f32 = 0.0f;
    var _S517 : f32 = 0.0f;
    s_bwdProp_towards_white_0(_S470, &(_S516), &(_S517), xy_4[i32(0)]);
    var s_diff_moved_T_0 : vec2<f32> = vec2<f32>(_S517, _S515);
    var _S518 : vec2<f32> = _S465 * s_diff_moved_T_0;
    var _S519 : vec2<f32> = delta_1 * s_diff_moved_T_0;
    var _S520 : f32 = _S519[i32(0)] + _S519[i32(1)];
    var _S521 : f32 = 0.0f;
    var _S522 : f32 = 0.0f;
    s_bwdProp_min_1(_S464, &(_S521), &(_S522), _S520);
    var _S523 : f32 = _S521;
    var _S524 : vec2<f32> = s_diff_moved_T_0 + vec2<f32>(_S516, _S514);
    if(_S448)
    {
        if(_S449)
        {
            var _S525 : f32 = 0.0f;
            s_bwdProp_bounded_power_0(_S451, &(_S525), _S523);
            correction_1 = _S525;
        }
        else
        {
            if(_S450)
            {
                correction_1 = - _S523;
            }
            else
            {
                correction_1 = _S523;
            }
            var _S526 : f32 = 0.0f;
            s_bwdProp_bounded_power_0(_S452, &(_S526), correction_1);
            correction_1 = - _S526;
        }
    }
    else
    {
        correction_1 = 0.0f;
    }
    var _S527 : f32 = _S421 * correction_1;
    var _S528 : f32 = _S445.value0_8 * correction_1;
    var _S529 : vec2<f32> = _S418;
    var _S530 : vec2<f32> = _S418;
    s_bwdProp_dot_0(_S447, &(_S529), &(_S530), _S527);
    var _S531 : vec2<f32> = (vec2<f32>(0) - (_S529 + _S530 + _S518)) + _S524;
    var _S532 : vec2<f32> = _S418;
    s_bwdProp_xy_to_uv_0(_S444, &(_S532), _S531);
    var _S533 : vec2<f32> = _S532;
    var _S534 : f32 = _S528 + _S502;
    var _S535 : f32;
    if(_S437)
    {
        var _S536 : vec2<f32> = _S533 / vec2<f32>(_S439);
        var _S537 : vec2<f32> = _S438 * (vec2<f32>(0) - _S536);
        var _S538 : vec2<f32> = vec2<f32>(_S434.value0_11) * _S536;
        var _S539 : f32 = _S538[i32(1)] + _S534;
        _S439 = _S537[i32(0)] + _S537[i32(1)];
        correction_1 = _S539;
        _S535 = _S538[i32(0)];
    }
    else
    {
        _S439 = 0.0f;
        correction_1 = _S534;
        _S535 = 0.0f;
    }
    var _S540 : f32 = 0.0f;
    var _S541 : f32 = 0.0f;
    s_bwdProp_max_2(_S436, &(_S540), &(_S541), _S439);
    var _S542 : f32 = 9.99999997475242708e-07f * _S541;
    var _S543 : f32 = 0.0f;
    s_bwdProp_abs_0(_S432, &(_S543), _S542);
    var _S544 : f32 = 0.0f;
    s_bwdProp_abs_0(_S428, &(_S544), _S542);
    var _S545 : f32 = 0.0f;
    s_bwdProp_abs_0(_S425, &(_S545), _S542);
    (*_S416) = vec3<f32>(_S545 + _S540 + _S535, _S544 + _S540 + correction_1, _S543 + _S540);
    return;
}

fn scaled_length_0( v_4 : vec3<f32>) -> f32
{
    var _S546 : f32 = max(abs(v_4.x), max(abs(v_4.y), abs(v_4.z)));
    var safe_0 : f32;
    if((_S546 > 0.0f))
    {
        safe_0 = _S546;
    }
    else
    {
        safe_0 = 1.0f;
    }
    return length(v_4 / vec3<f32>(safe_0)) * safe_0;
}

struct Tuple_24
{
     value0_44 : f32,
};

fn s_apply_scaled_length_0( _S547 : vec3<f32>) -> Tuple_24
{
    var _S548 : Tuple_9 = s_apply_max_2(s_apply_abs_0(_S547.x).value0_9, s_apply_max_2(s_apply_abs_0(_S547.y).value0_9, s_apply_abs_0(_S547.z).value0_9).value0_11);
    var safe_1 : f32;
    if(((_S548.value0_11) > 0.0f))
    {
        safe_1 = _S548.value0_11;
    }
    else
    {
        safe_1 = 1.0f;
    }
    var _S549 : Tuple_24;
    _S549.value0_44 = s_apply_length_0(_S547 / vec3<f32>(safe_1)).value0_0 * safe_1;
    return _S549;
}

struct s_paramCtx_s_bwdCallableCtx_scaled_length_0
{
     value0_45 : vec3<f32>,
};

struct s_bwdCallableCtx_scaled_length_0
{
     value0_46 : s_paramCtx_s_bwdCallableCtx_scaled_length_0,
};

fn s_bwdProp_scaled_length_0( _S550 : s_bwdCallableCtx_scaled_length_0,  _S551 : ptr<function, vec3<f32>>,  _S552 : f32)
{
    var _S553 : f32 = _S550.value0_46.value0_45.x;
    var _S554 : Tuple_7 = s_apply_abs_0(_S553);
    var _S555 : s_bwdCallableCtx_abs_0;
    _S555._S70 = _S553;
    var _S556 : s_bwdCallableCtx_abs_0 = _S555;
    var _S557 : f32 = _S550.value0_46.value0_45.y;
    var _S558 : Tuple_7 = s_apply_abs_0(_S557);
    var _S559 : s_bwdCallableCtx_abs_0;
    _S559._S70 = _S557;
    var _S560 : s_bwdCallableCtx_abs_0 = _S559;
    var _S561 : f32 = _S550.value0_46.value0_45.z;
    var _S562 : Tuple_7 = s_apply_abs_0(_S561);
    var _S563 : s_bwdCallableCtx_abs_0;
    _S563._S70 = _S561;
    var _S564 : s_bwdCallableCtx_abs_0 = _S563;
    var _S565 : Tuple_9 = s_apply_max_2(_S558.value0_9, _S562.value0_9);
    var _S566 : s_bwdCallableCtx_max_2;
    _S566._S79 = _S558.value0_9;
    _S566._S80 = _S562.value0_9;
    var _S567 : s_bwdCallableCtx_max_2 = _S566;
    var _S568 : Tuple_9 = s_apply_max_2(_S554.value0_9, _S565.value0_11);
    var _S569 : s_bwdCallableCtx_max_2;
    _S569._S79 = _S554.value0_9;
    _S569._S80 = _S565.value0_11;
    var _S570 : s_bwdCallableCtx_max_2 = _S569;
    var _S571 : bool = ((_S568.value0_11) > 0.0f);
    var safe_2 : f32;
    if(_S571)
    {
        safe_2 = _S568.value0_11;
    }
    else
    {
        safe_2 = 1.0f;
    }
    var _S572 : vec3<f32> = _S550.value0_46.value0_45 / vec3<f32>(safe_2);
    var _S573 : f32 = safe_2 * safe_2;
    var _S574 : Tuple_0 = s_apply_length_0(_S572);
    var _S575 : s_bwdCallableCtx_length_0;
    _S575._S5 = _S572;
    var _S576 : s_bwdCallableCtx_length_0 = _S575;
    var _S577 : f32 = _S574.value0_0 * _S552;
    var _S578 : f32 = safe_2 * _S552;
    var _S579 : vec3<f32> = vec3<f32>(0.0f);
    s_bwdProp_length_0(_S576, &(_S579), _S578);
    var _S580 : vec3<f32> = _S579 / vec3<f32>(_S573);
    var _S581 : vec3<f32> = _S550.value0_46.value0_45 * (vec3<f32>(0) - _S580);
    var _S582 : vec3<f32> = vec3<f32>(safe_2) * _S580;
    var _S583 : f32 = _S577 + (_S581[i32(0)] + _S581[i32(1)] + _S581[i32(2)]);
    if(_S571)
    {
        safe_2 = _S583;
    }
    else
    {
        safe_2 = 0.0f;
    }
    var _S584 : f32 = 0.0f;
    var _S585 : f32 = 0.0f;
    s_bwdProp_max_2(_S570, &(_S584), &(_S585), safe_2);
    var _S586 : f32 = 0.0f;
    var _S587 : f32 = 0.0f;
    s_bwdProp_max_2(_S567, &(_S586), &(_S587), _S585);
    var _S588 : f32 = 0.0f;
    s_bwdProp_abs_0(_S564, &(_S588), _S587);
    var _S589 : f32 = 0.0f;
    s_bwdProp_abs_0(_S560, &(_S589), _S586);
    var _S590 : f32 = 0.0f;
    s_bwdProp_abs_0(_S556, &(_S590), _S584);
    (*_S551) = _S582 + vec3<f32>(_S590, _S589, _S588);
    return;
}

fn s_fwd_scaled_length_0( dpv_4 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_float_0
{
    var _S591 : DiffPair_float_0 = DiffPair_float_0( dpv_4.primal_0.x, dpv_4.differential_0.x );
    var _S592 : DiffPair_float_0 = _d_abs_1(_S591);
    var _S593 : DiffPair_float_0 = DiffPair_float_0( dpv_4.primal_0.y, dpv_4.differential_0.y );
    var _S594 : DiffPair_float_0 = _d_abs_1(_S593);
    var _S595 : DiffPair_float_0 = DiffPair_float_0( dpv_4.primal_0.z, dpv_4.differential_0.z );
    var _S596 : DiffPair_float_0 = _d_abs_1(_S595);
    var _S597 : DiffPair_float_0 = DiffPair_float_0( _S594.primal_0, _S594.differential_0 );
    var _S598 : DiffPair_float_0 = DiffPair_float_0( _S596.primal_0, _S596.differential_0 );
    var _S599 : DiffPair_float_0 = _d_max_1(_S597, _S598);
    var _S600 : DiffPair_float_0 = DiffPair_float_0( _S592.primal_0, _S592.differential_0 );
    var _S601 : DiffPair_float_0 = DiffPair_float_0( _S599.primal_0, _S599.differential_0 );
    var _S602 : DiffPair_float_0 = _d_max_1(_S600, _S601);
    var safe_3 : f32;
    var s_diff_safe_0 : f32;
    if(((_S602.primal_0) > 0.0f))
    {
        safe_3 = _S602.primal_0;
        s_diff_safe_0 = _S602.differential_0;
    }
    else
    {
        safe_3 = 1.0f;
        s_diff_safe_0 = 0.0f;
    }
    var _S603 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( dpv_4.primal_0 / vec3<f32>(safe_3), (dpv_4.differential_0 * vec3<f32>(safe_3) - dpv_4.primal_0 * vec3<f32>(s_diff_safe_0)) / vec3<f32>((safe_3 * safe_3)) );
    var _S604 : DiffPair_float_0 = s_fwd_length_impl_0(_S603);
    var _S605 : DiffPair_float_0 = DiffPair_float_0( _S604.primal_0 * safe_3, _S604.differential_0 * safe_3 + s_diff_safe_0 * _S604.primal_0 );
    return _S605;
}

struct Tuple_25
{
     value0_47 : vec3<f32>,
};

fn s_apply_luma_chroma_0( _S606 : vec3<f32>,  p_1 : ChannelMixerParams_0) -> Tuple_25
{
    var norm_0 : f32 = s_apply_max_2(s_apply_scaled_length_0(_S606).value0_44, 1.52587890625e-05f).value0_11 * 0.57735025882720947f;
    var ratios_0 : vec3<f32> = _S606 / vec3<f32>(norm_0);
    var _S607 : Tuple_8 = s_apply_max_1((vec3<f32>(1.0f) - ratios_0) * vec3<f32>((s_apply_dot_1(ratios_0, p_1.saturation_0.xyz).value0_12 / 3.0f)) + ratios_0, s_apply_min_0(ratios_0, vec3<f32>(0.0f)).value0_6);
    var _S608 : Tuple_25;
    _S608.value0_47 = _S607.value0_10 * vec3<f32>((norm_0 / (s_apply_max_2(s_apply_scaled_length_0(_S607.value0_10).value0_44, 1.52587890625e-05f).value0_11 * 0.57735025882720947f) * s_apply_clamp_0(1.0f + s_apply_dot_1(_S606, p_1.lightness_0.xyz).value0_12 / s_apply_max_2((_S606.x + _S606.y + _S606.z) / 3.0f, 1.52587890625e-05f).value0_11, 0.0f, 1.0e+06f).value0_5));
    return _S608;
}

struct s_paramCtx_s_bwdCallableCtx_luma_chroma_0
{
     value0_48 : vec3<f32>,
     value1_6 : ChannelMixerParams_0,
};

struct s_bwdCallableCtx_luma_chroma_0
{
     value0_49 : s_paramCtx_s_bwdCallableCtx_luma_chroma_0,
};

fn s_bwdProp_luma_chroma_0( _S609 : s_bwdCallableCtx_luma_chroma_0,  _S610 : ptr<function, vec3<f32>>,  _S611 : vec3<f32>)
{
    var _S612 : Tuple_24 = s_apply_scaled_length_0(_S609.value0_49.value0_48);
    var _S613 : s_paramCtx_s_bwdCallableCtx_scaled_length_0;
    _S613.value0_45 = _S609.value0_49.value0_48;
    var _S614 : s_bwdCallableCtx_scaled_length_0 = s_bwdCallableCtx_scaled_length_0( _S613 );
    var _S615 : Tuple_9 = s_apply_max_2(_S612.value0_44, 1.52587890625e-05f);
    var _S616 : s_bwdCallableCtx_max_2;
    _S616._S79 = _S612.value0_44;
    _S616._S80 = 1.52587890625e-05f;
    var _S617 : f32 = (_S609.value0_49.value0_48.x + _S609.value0_49.value0_48.y + _S609.value0_49.value0_48.z) / 3.0f;
    var _S618 : Tuple_9 = s_apply_max_2(_S617, 1.52587890625e-05f);
    var _S619 : s_bwdCallableCtx_max_2;
    _S619._S79 = _S617;
    _S619._S80 = 1.52587890625e-05f;
    var _S620 : vec3<f32> = _S609.value0_49.value1_6.lightness_0.xyz;
    var _S621 : Tuple_10 = s_apply_dot_1(_S609.value0_49.value0_48, _S620);
    var _S622 : s_bwdCallableCtx_dot_1;
    _S622._S84 = _S609.value0_49.value0_48;
    _S622._S85 = _S620;
    var norm_1 : f32 = _S615.value0_11 * 0.57735025882720947f;
    var _S623 : vec3<f32> = vec3<f32>(norm_1);
    var ratios_1 : vec3<f32> = _S609.value0_49.value0_48 / _S623;
    var _S624 : f32 = norm_1 * norm_1;
    var _S625 : vec3<f32> = _S609.value0_49.value1_6.saturation_0.xyz;
    var _S626 : Tuple_10 = s_apply_dot_1(ratios_1, _S625);
    var _S627 : s_bwdCallableCtx_dot_1;
    _S627._S84 = ratios_1;
    _S627._S85 = _S625;
    var _S628 : vec3<f32> = vec3<f32>(1.0f) - ratios_1;
    var _S629 : vec3<f32> = vec3<f32>((_S626.value0_12 / 3.0f));
    var _S630 : vec3<f32> = _S628 * _S629 + ratios_1;
    var _S631 : vec3<f32> = vec3<f32>(0.0f);
    var _S632 : Tuple_4 = s_apply_min_0(ratios_1, _S631);
    var _S633 : s_bwdCallableCtx_min_0;
    _S633._S49 = ratios_1;
    _S633._S50 = _S631;
    var _S634 : Tuple_8 = s_apply_max_1(_S630, _S632.value0_6);
    var _S635 : s_bwdCallableCtx_max_1;
    _S635._S74 = _S630;
    _S635._S75 = _S632.value0_6;
    var _S636 : Tuple_24 = s_apply_scaled_length_0(_S634.value0_10);
    var _S637 : s_paramCtx_s_bwdCallableCtx_scaled_length_0;
    _S637.value0_45 = _S634.value0_10;
    var _S638 : s_bwdCallableCtx_scaled_length_0 = s_bwdCallableCtx_scaled_length_0( _S637 );
    var _S639 : Tuple_9 = s_apply_max_2(_S636.value0_44, 1.52587890625e-05f);
    var _S640 : s_bwdCallableCtx_max_2;
    _S640._S79 = _S636.value0_44;
    _S640._S80 = 1.52587890625e-05f;
    var _S641 : f32 = _S639.value0_11 * 0.57735025882720947f;
    var norm_2 : f32 = norm_1 / _S641;
    var _S642 : f32 = _S641 * _S641;
    var _S643 : f32 = _S618.value0_11 * _S618.value0_11;
    var _S644 : f32 = 1.0f + _S621.value0_12 / _S618.value0_11;
    var _S645 : Tuple_3 = s_apply_clamp_0(_S644, 0.0f, 1.0e+06f);
    var _S646 : s_bwdCallableCtx_clamp_0;
    _S646._S37 = _S644;
    _S646._S38 = 0.0f;
    _S646._S39 = 1.0e+06f;
    var _S647 : vec3<f32> = _S634.value0_10 * _S611;
    var _S648 : f32 = _S647[i32(0)] + _S647[i32(1)] + _S647[i32(2)];
    var _S649 : vec3<f32> = vec3<f32>((norm_2 * _S645.value0_5)) * _S611;
    var _S650 : f32 = norm_2 * _S648;
    var s_diff_norm_T_0 : f32 = _S645.value0_5 * _S648;
    var _S651 : f32 = 0.0f;
    var _S652 : f32 = 0.0f;
    var _S653 : f32 = 0.0f;
    s_bwdProp_clamp_0(_S646, &(_S651), &(_S652), &(_S653), _S650);
    var _S654 : f32 = _S651 / _S643;
    var _S655 : f32 = _S621.value0_12 * - _S654;
    var _S656 : f32 = _S618.value0_11 * _S654;
    var _S657 : f32 = s_diff_norm_T_0 / _S642;
    var _S658 : f32 = _S641 * _S657;
    var _S659 : f32 = 0.57735025882720947f * (norm_1 * - _S657);
    var _S660 : f32 = 0.0f;
    var _S661 : f32 = 0.0f;
    s_bwdProp_max_2(_S640, &(_S660), &(_S661), _S659);
    var _S662 : vec3<f32> = vec3<f32>(0.0f);
    var _S663 : vec3<f32> = _S662;
    s_bwdProp_scaled_length_0(_S638, &(_S663), _S660);
    var _S664 : vec3<f32> = _S649 + _S663;
    var _S665 : vec3<f32> = _S662;
    var _S666 : vec3<f32> = _S662;
    s_bwdProp_max_1(_S635, &(_S665), &(_S666), _S664);
    var _S667 : vec3<f32> = _S662;
    var _S668 : vec3<f32> = _S662;
    s_bwdProp_min_0(_S633, &(_S667), &(_S668), _S666);
    var _S669 : vec3<f32> = _S628 * _S665;
    var _S670 : vec3<f32> = (vec3<f32>(0) - _S629 * _S665);
    var _S671 : f32 = 0.3333333432674408f * (_S669[i32(0)] + _S669[i32(1)] + _S669[i32(2)]);
    var _S672 : vec3<f32> = _S662;
    var _S673 : vec3<f32> = _S662;
    s_bwdProp_dot_1(_S627, &(_S672), &(_S673), _S671);
    var _S674 : vec3<f32> = (_S667 + _S665 + _S670 + _S672) / vec3<f32>(_S624);
    var _S675 : vec3<f32> = _S609.value0_49.value0_48 * (vec3<f32>(0) - _S674);
    var _S676 : vec3<f32> = _S623 * _S674;
    var _S677 : f32 = 0.57735025882720947f * (_S658 + (_S675[i32(0)] + _S675[i32(1)] + _S675[i32(2)]));
    var _S678 : vec3<f32> = _S662;
    var _S679 : vec3<f32> = _S662;
    s_bwdProp_dot_1(_S622, &(_S678), &(_S679), _S656);
    var _S680 : f32 = 0.0f;
    var _S681 : f32 = 0.0f;
    s_bwdProp_max_2(_S619, &(_S680), &(_S681), _S655);
    var _S682 : f32 = 0.3333333432674408f * _S680;
    var _S683 : f32 = 0.0f;
    var _S684 : f32 = 0.0f;
    s_bwdProp_max_2(_S616, &(_S683), &(_S684), _S677);
    var _S685 : vec3<f32> = _S662;
    s_bwdProp_scaled_length_0(_S614, &(_S685), _S683);
    (*_S610) = _S676 + _S678 + _S685 + vec3<f32>(_S682, _S682, _S682);
    return;
}

struct s_paramCtx_s_bwdCallableCtx_channel_mixer_0
{
     value0_50 : vec3<f32>,
     value1_7 : ChannelMixerParams_0,
     value2_1 : Illuminant_0,
};

struct s_bwdCallableCtx_channel_mixer_0
{
     value0_51 : s_paramCtx_s_bwdCallableCtx_channel_mixer_0,
};

fn s_bwdProp_channel_mixer_0( _S686 : s_bwdCallableCtx_channel_mixer_0,  _S687 : ptr<function, vec3<f32>>,  _S688 : ptr<function, Illuminant_0>,  _S689 : vec3<f32>)
{
    var _S690 : vec3<f32> = vec3<f32>(0.0f);
    var _S691 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S692 : s_paramCtx_s_bwdCallableCtx_clip_0 = s_paramCtx_s_bwdCallableCtx_clip_0( _S690, 0.0f );
    var to_lms_0 : mat3x3<f32> = rows_0(_S686.value0_51.value1_7.to_lms_0_0, _S686.value0_51.value1_7.to_lms_1_0, _S686.value0_51.value1_7.to_lms_2_0);
    var mix_0 : mat3x3<f32> = rows_0(_S686.value0_51.value1_7.mix_0_0, _S686.value0_51.value1_7.mix_1_0, _S686.value0_51.value1_7.mix_2_0);
    var cone_0 : mat3x3<f32> = rows_0(_S686.value0_51.value1_7.cone_0_0, _S686.value0_51.value1_7.cone_1_0, _S686.value0_51.value1_7.cone_2_0);
    var to_space_0 : mat3x3<f32> = rows_0(_S686.value0_51.value1_7.to_space_0_0, _S686.value0_51.value1_7.to_space_1_0, _S686.value0_51.value1_7.to_space_2_0);
    var from_space_0 : mat3x3<f32> = rows_0(_S686.value0_51.value1_7.from_space_0_0, _S686.value0_51.value1_7.from_space_1_0, _S686.value0_51.value1_7.from_space_2_0);
    var to_rgb_0 : mat3x3<f32> = rows_0(_S686.value0_51.value1_7.to_rgb_0_0, _S686.value0_51.value1_7.to_rgb_1_0, _S686.value0_51.value1_7.to_rgb_2_0);
    var _S693 : vec2<f32> = vec2<f32>(_S686.value0_51.value2_1.x_17, _S686.value0_51.value2_1.y_11);
    var _S694 : Tuple_15 = s_apply_illuminant_xy_to_xyz_0(_S693);
    var _S695 : s_paramCtx_s_bwdCallableCtx_illuminant_xy_to_xyz_0;
    _S695.value0_18 = _S693;
    var _S696 : s_bwdCallableCtx_illuminant_xy_to_xyz_0 = s_bwdCallableCtx_illuminant_xy_to_xyz_0( _S695 );
    var _S697 : Tuple_14 = s_apply_mul_0(cone_0, _S694.value0_17);
    var _S698 : s_bwdCallableCtx_mul_0;
    _S698._S106 = cone_0;
    _S698._S107 = _S694.value0_17;
    var _S699 : s_bwdCallableCtx_mul_0 = _S698;
    var _S700 : Tuple_17 = s_apply_away_from_zero_1(_S697.value0_16, 1.52587890625e-05f);
    var _S701 : s_paramCtx_s_bwdCallableCtx_away_from_zero_1;
    _S701.value0_24 = _S697.value0_16;
    _S701.value1_1 = 1.52587890625e-05f;
    var _S702 : s_bwdCallableCtx_away_from_zero_1 = s_bwdCallableCtx_away_from_zero_1( _S701 );
    var _S703 : vec3<f32> = vec3<f32>(-1.0e+08f);
    var _S704 : vec3<f32> = vec3<f32>(1.0e+08f);
    var _S705 : Tuple_13 = s_apply_clamp_1(_S686.value0_51.value0_50, _S703, _S704);
    var _S706 : s_bwdCallableCtx_clamp_1;
    _S706._S99 = _S686.value0_51.value0_50;
    _S706._S100 = _S703;
    _S706._S101 = _S704;
    var _S707 : s_bwdCallableCtx_clamp_1 = _S706;
    var _S708 : Tuple_18 = s_apply_clip_0(_S705.value0_15, _S686.value0_51.value1_7.clip_min_0);
    var _S709 : s_paramCtx_s_bwdCallableCtx_clip_0;
    _S709.value0_27 = _S705.value0_15;
    _S709.value1_2 = _S686.value0_51.value1_7.clip_min_0;
    var _S710 : s_bwdCallableCtx_clip_0 = s_bwdCallableCtx_clip_0( _S709 );
    var _S711 : Tuple_14 = s_apply_mul_0(to_lms_0, _S708.value0_26);
    var _S712 : s_bwdCallableCtx_mul_0;
    _S712._S106 = to_lms_0;
    _S712._S107 = _S708.value0_26;
    var _S713 : s_bwdCallableCtx_mul_0 = _S712;
    var _S714 : bool = (_S686.value0_51.value1_7.adaptation_0) == u32(2);
    var lms_0 : vec3<f32>;
    var _S715 : vec3<f32>;
    var _S716 : vec3<f32>;
    var _S717 : vec3<f32>;
    var _S718 : vec3<f32>;
    var _S719 : vec3<f32>;
    var _S720 : vec3<f32>;
    var _S721 : vec3<f32>;
    var _S722 : vec3<f32>;
    var scale_3 : f32;
    var power_0 : f32;
    var _S723 : f32;
    var blue_0 : f32;
    var _S724 : s_bwdCallableCtx_pow0_0;
    var _S725 : s_bwdCallableCtx_pow0_0;
    var _S726 : s_bwdCallableCtx_min_1;
    var _S727 : bool;
    var _S728 : bool;
    var _S729 : bool;
    var _S730 : bool;
    var _S731 : s_bwdCallableCtx_mul_0;
    if(_S714)
    {
        var _S732 : f32 = _S711.value0_16.y;
        var _S733 : bool = (_S732 > 1.52587890625e-05f);
        if(_S733)
        {
            scale_3 = _S732 + 1.52587890625e-05f;
        }
        else
        {
            scale_3 = 1.52587890625e-05f;
        }
        var _S734 : f32 = _S700.value0_23.z;
        var _S735 : bool = (_S734 > 0.0f);
        if(_S735)
        {
            var _S736 : f32 = 0.81815499067306519f / _S734;
            var _S737 : f32 = _S734 * _S734;
            var _S738 : Tuple_12 = s_apply_pow0_0(_S736, 0.083400003612041473f);
            var _S739 : s_bwdCallableCtx_pow0_0;
            _S739._S94 = _S736;
            _S739._S95 = 0.083400003612041473f;
            power_0 = _S738.value0_14;
            _S724 = _S739;
            _S723 = _S737;
        }
        else
        {
            power_0 = 1.0f;
            _S724._S94 = 0.0f;
            _S724._S95 = 0.0f;
            _S723 = 0.0f;
        }
        var _S740 : Tuple_14 = s_apply_mul_0(cone_0, _S711.value0_16);
        var _S741 : s_bwdCallableCtx_mul_0;
        _S741._S106 = cone_0;
        _S741._S107 = _S711.value0_16;
        var _S742 : s_bwdCallableCtx_mul_0 = _S741;
        var _S743 : vec3<f32> = _S740.value0_16 / vec3<f32>(scale_3);
        var _S744 : f32 = scale_3 * scale_3;
        var ratio_0 : vec3<f32> = _S743 / _S700.value0_23;
        var _S745 : vec3<f32> = _S700.value0_23 * _S700.value0_23;
        var _S746 : f32 = ratio_0.z;
        var _S747 : bool = (_S746 > 0.0f);
        if(_S747)
        {
            var _S748 : Tuple_11 = s_apply_min_1(_S746, 1.0e+08f);
            var _S749 : s_bwdCallableCtx_min_1;
            _S749._S89 = _S746;
            _S749._S90 = 1.0e+08f;
            var _S750 : Tuple_12 = s_apply_pow0_0(_S748.value0_13, power_0);
            var _S751 : s_bwdCallableCtx_pow0_0;
            _S751._S94 = _S748.value0_13;
            _S751._S95 = power_0;
            blue_0 = _S750.value0_14;
            _S725 = _S751;
            _S726 = _S749;
        }
        else
        {
            blue_0 = _S746;
            _S725._S94 = 0.0f;
            _S725._S95 = 0.0f;
            _S726._S89 = 0.0f;
            _S726._S90 = 0.0f;
        }
        var _S752 : vec3<f32> = _S686.value0_51.value1_7.d50_0.xyz;
        var _S753 : vec3<f32> = _S752 * vec3<f32>(ratio_0.x, ratio_0.y, blue_0);
        var _S754 : s_bwdCallableCtx_pow0_0 = _S724;
        lms_0 = _S753 * vec3<f32>(scale_3);
        _S715 = _S753;
        _S716 = _S752;
        _S727 = _S747;
        _S724 = _S725;
        _S717 = _S745;
        _S718 = _S743;
        power_0 = _S744;
        _S719 = _S740.value0_16;
        _S731 = _S742;
        _S728 = _S735;
        _S725 = _S754;
        _S729 = _S733;
        _S730 = false;
        _S720 = _S690;
        _S721 = _S690;
        _S722 = _S690;
    }
    else
    {
        var _S755 : bool = (_S686.value0_51.value1_7.adaptation_0) != u32(4);
        if(_S755)
        {
            var _S756 : vec3<f32> = _S686.value0_51.value1_7.d50_0.xyz;
            var _S757 : vec3<f32> = _S756 / _S700.value0_23;
            var _S758 : vec3<f32> = _S700.value0_23 * _S700.value0_23;
            lms_0 = _S711.value0_16 * _S757;
            _S715 = _S757;
            _S716 = _S758;
            _S717 = _S756;
        }
        else
        {
            lms_0 = _S711.value0_16;
            _S715 = _S690;
            _S716 = _S690;
            _S717 = _S690;
        }
        var _S759 : vec3<f32> = _S715;
        var _S760 : vec3<f32> = _S716;
        var _S761 : vec3<f32> = _S717;
        _S715 = _S690;
        scale_3 = 0.0f;
        _S716 = _S690;
        _S727 = false;
        _S724._S94 = 0.0f;
        _S724._S95 = 0.0f;
        _S726._S89 = 0.0f;
        _S726._S90 = 0.0f;
        _S717 = _S690;
        _S718 = _S690;
        power_0 = 0.0f;
        _S719 = _S690;
        _S731._S106 = _S691;
        _S731._S107 = _S690;
        _S728 = false;
        _S725._S94 = 0.0f;
        _S725._S95 = 0.0f;
        _S723 = 0.0f;
        _S729 = false;
        _S730 = _S755;
        _S720 = _S759;
        _S721 = _S760;
        _S722 = _S761;
    }
    var _S762 : Tuple_14 = s_apply_mul_0(mix_0, lms_0);
    var _S763 : s_bwdCallableCtx_mul_0;
    _S763._S106 = mix_0;
    _S763._S107 = lms_0;
    var _S764 : s_bwdCallableCtx_mul_0 = _S763;
    var _S765 : Tuple_18 = s_apply_clip_0(_S762.value0_16, _S686.value0_51.value1_7.clip_min_0);
    var _S766 : s_paramCtx_s_bwdCallableCtx_clip_0;
    _S766.value0_27 = _S762.value0_16;
    _S766.value1_2 = _S686.value0_51.value1_7.clip_min_0;
    var _S767 : s_bwdCallableCtx_clip_0 = s_bwdCallableCtx_clip_0( _S766 );
    var _S768 : vec3<f32> = vec3<f32>(-9.99999995904e+11f);
    var _S769 : vec3<f32> = vec3<f32>(9.99999995904e+11f);
    var _S770 : Tuple_13 = s_apply_clamp_1(_S765.value0_26, _S768, _S769);
    var _S771 : s_bwdCallableCtx_clamp_1;
    _S771._S99 = _S765.value0_26;
    _S771._S100 = _S768;
    _S771._S101 = _S769;
    var _S772 : s_bwdCallableCtx_clamp_1 = _S771;
    var _S773 : Tuple_23 = s_apply_gamut_mapping_0(_S770.value0_15, _S686.value0_51.value1_7);
    var _S774 : s_paramCtx_s_bwdCallableCtx_gamut_mapping_0;
    _S774.value0_42 = _S770.value0_15;
    _S774.value1_5 = _S686.value0_51.value1_7;
    var _S775 : s_bwdCallableCtx_gamut_mapping_0 = s_bwdCallableCtx_gamut_mapping_0( _S774 );
    var _S776 : Tuple_13 = s_apply_clamp_1(_S773.value0_41, _S768, _S769);
    var _S777 : s_bwdCallableCtx_clamp_1;
    _S777._S99 = _S773.value0_41;
    _S777._S100 = _S768;
    _S777._S101 = _S769;
    var _S778 : s_bwdCallableCtx_clamp_1 = _S777;
    var _S779 : Tuple_14 = s_apply_mul_0(to_space_0, _S776.value0_15);
    var _S780 : s_bwdCallableCtx_mul_0;
    _S780._S106 = to_space_0;
    _S780._S107 = _S776.value0_15;
    var _S781 : s_bwdCallableCtx_mul_0 = _S780;
    var _S782 : Tuple_18 = s_apply_clip_0(_S779.value0_16, _S686.value0_51.value1_7.clip_min_0);
    var _S783 : s_paramCtx_s_bwdCallableCtx_clip_0;
    _S783.value0_27 = _S779.value0_16;
    _S783.value1_2 = _S686.value0_51.value1_7.clip_min_0;
    var _S784 : s_bwdCallableCtx_clip_0 = s_bwdCallableCtx_clip_0( _S783 );
    var _S785 : Tuple_25 = s_apply_luma_chroma_0(_S782.value0_26, _S686.value0_51.value1_7);
    var _S786 : s_paramCtx_s_bwdCallableCtx_luma_chroma_0;
    _S786.value0_48 = _S782.value0_26;
    _S786.value1_6 = _S686.value0_51.value1_7;
    var _S787 : s_bwdCallableCtx_luma_chroma_0 = s_bwdCallableCtx_luma_chroma_0( _S786 );
    var _S788 : Tuple_18 = s_apply_clip_0(_S785.value0_47, _S686.value0_51.value1_7.clip_min_0);
    var _S789 : s_paramCtx_s_bwdCallableCtx_clip_0;
    _S789.value0_27 = _S785.value0_47;
    _S789.value1_2 = _S686.value0_51.value1_7.clip_min_0;
    var _S790 : s_bwdCallableCtx_clip_0 = s_bwdCallableCtx_clip_0( _S789 );
    var _S791 : bool = (_S686.value0_51.value1_7.apply_grey_0) != u32(0);
    var _S792 : s_bwdCallableCtx_max_2;
    var _S793 : s_bwdCallableCtx_dot_1;
    if(_S791)
    {
        var _S794 : vec3<f32> = _S686.value0_51.value1_7.grey_0.xyz;
        var _S795 : Tuple_10 = s_apply_dot_1(_S788.value0_26, _S794);
        var _S796 : s_bwdCallableCtx_dot_1;
        _S796._S84 = _S788.value0_26;
        _S796._S85 = _S794;
        var _S797 : s_bwdCallableCtx_max_2;
        _S797._S79 = _S795.value0_12;
        _S797._S80 = 0.0f;
        _S792 = _S797;
        _S793 = _S796;
    }
    else
    {
        _S792._S79 = 0.0f;
        _S792._S80 = 0.0f;
        _S793._S84 = _S690;
        _S793._S85 = _S690;
    }
    var _S798 : bool = !_S791;
    var _S799 : s_bwdCallableCtx_mul_0;
    var _S800 : s_bwdCallableCtx_mul_0;
    var _S801 : s_bwdCallableCtx_clip_0;
    var _S802 : s_bwdCallableCtx_clip_0;
    if(_S798)
    {
        var _S803 : Tuple_14 = s_apply_mul_0(from_space_0, _S788.value0_26);
        var _S804 : s_bwdCallableCtx_mul_0;
        _S804._S106 = from_space_0;
        _S804._S107 = _S788.value0_26;
        var _S805 : Tuple_18 = s_apply_clip_0(_S803.value0_16, _S686.value0_51.value1_7.clip_min_0);
        var _S806 : s_paramCtx_s_bwdCallableCtx_clip_0;
        _S806.value0_27 = _S803.value0_16;
        _S806.value1_2 = _S686.value0_51.value1_7.clip_min_0;
        var _S807 : Tuple_14 = s_apply_mul_0(to_rgb_0, _S805.value0_26);
        var _S808 : s_bwdCallableCtx_mul_0;
        _S808._S106 = to_rgb_0;
        _S808._S107 = _S805.value0_26;
        var _S809 : s_paramCtx_s_bwdCallableCtx_clip_0;
        _S809.value0_27 = _S807.value0_16;
        _S809.value1_2 = _S686.value0_51.value1_7.clip_min_0;
        _S801.value0_28 = _S809;
        _S799 = _S808;
        _S802.value0_28 = _S806;
        _S800 = _S804;
    }
    else
    {
        _S801.value0_28 = _S692;
        _S799._S106 = _S691;
        _S799._S107 = _S690;
        _S802.value0_28 = _S692;
        _S800._S106 = _S691;
        _S800._S107 = _S690;
    }
    var _S810 : Illuminant_0 = Illuminant_x24_syn_dzero_0();
    var _S811 : vec3<f32>;
    if(_S798)
    {
        var _S812 : vec3<f32> = _S690;
        s_bwdProp_clip_0(_S801, &(_S812), _S689);
        var _S813 : mat3x3<f32> = _S691;
        var _S814 : vec3<f32> = _S690;
        s_bwdProp_mul_0(_S799, &(_S813), &(_S814), _S812);
        var _S815 : vec3<f32> = _S690;
        s_bwdProp_clip_0(_S802, &(_S815), _S814);
        var _S816 : mat3x3<f32> = _S691;
        var _S817 : vec3<f32> = _S690;
        s_bwdProp_mul_0(_S800, &(_S816), &(_S817), _S815);
        lms_0 = _S690;
        _S811 = _S817;
    }
    else
    {
        lms_0 = _S689;
        _S811 = _S690;
    }
    if(_S791)
    {
        var _S818 : f32 = lms_0[i32(0)] + lms_0[i32(1)] + lms_0[i32(2)];
        var _S819 : f32 = 0.0f;
        var _S820 : f32 = 0.0f;
        s_bwdProp_max_2(_S792, &(_S819), &(_S820), _S818);
        var _S821 : vec3<f32> = _S690;
        var _S822 : vec3<f32> = _S690;
        s_bwdProp_dot_1(_S793, &(_S821), &(_S822), _S819);
        lms_0 = _S821 + _S811;
    }
    else
    {
        lms_0 = _S811;
    }
    var _S823 : vec3<f32> = _S690;
    s_bwdProp_clip_0(_S790, &(_S823), lms_0);
    var _S824 : vec3<f32> = _S690;
    s_bwdProp_luma_chroma_0(_S787, &(_S824), _S823);
    var _S825 : vec3<f32> = _S690;
    s_bwdProp_clip_0(_S784, &(_S825), _S824);
    var _S826 : mat3x3<f32> = _S691;
    var _S827 : vec3<f32> = _S690;
    s_bwdProp_mul_0(_S781, &(_S826), &(_S827), _S825);
    var _S828 : vec3<f32> = _S690;
    var _S829 : vec3<f32> = _S690;
    var _S830 : vec3<f32> = _S690;
    s_bwdProp_clamp_1(_S778, &(_S828), &(_S829), &(_S830), _S827);
    var _S831 : vec3<f32> = _S690;
    s_bwdProp_gamut_mapping_0(_S775, &(_S831), _S828);
    var _S832 : vec3<f32> = _S690;
    var _S833 : vec3<f32> = _S690;
    var _S834 : vec3<f32> = _S690;
    s_bwdProp_clamp_1(_S772, &(_S832), &(_S833), &(_S834), _S831);
    var _S835 : vec3<f32> = _S690;
    s_bwdProp_clip_0(_S767, &(_S835), _S832);
    var _S836 : mat3x3<f32> = _S691;
    var _S837 : vec3<f32> = _S690;
    s_bwdProp_mul_0(_S764, &(_S836), &(_S837), _S835);
    var _S838 : vec3<f32> = _S837;
    if(_S714)
    {
        var _S839 : vec3<f32> = _S715 * _S838;
        var _S840 : f32 = _S839[i32(0)] + _S839[i32(1)] + _S839[i32(2)];
        var _S841 : vec3<f32> = vec3<f32>(scale_3);
        var _S842 : vec3<f32> = _S716 * (_S841 * _S838);
        var _S843 : vec3<f32> = vec3<f32>(_S842[i32(0)], _S842[i32(1)], 0.0f);
        if(_S727)
        {
            var _S844 : f32 = 0.0f;
            var _S845 : f32 = 0.0f;
            s_bwdProp_pow0_0(_S724, &(_S844), &(_S845), _S842[i32(2)]);
            var _S846 : f32 = 0.0f;
            var _S847 : f32 = 0.0f;
            s_bwdProp_min_1(_S726, &(_S846), &(_S847), _S844);
            scale_3 = _S846;
            blue_0 = _S845;
        }
        else
        {
            scale_3 = _S842[i32(2)];
            blue_0 = 0.0f;
        }
        var _S848 : vec3<f32> = (_S843 + vec3<f32>(0.0f, 0.0f, scale_3)) / _S717;
        var _S849 : vec3<f32> = _S718 * (vec3<f32>(0) - _S848);
        var _S850 : vec3<f32> = _S700.value0_23 * _S848 / vec3<f32>(power_0);
        var _S851 : vec3<f32> = _S719 * (vec3<f32>(0) - _S850);
        var _S852 : f32 = _S851[i32(0)] + _S851[i32(1)] + _S851[i32(2)];
        var _S853 : vec3<f32> = _S841 * _S850;
        var _S854 : mat3x3<f32> = _S691;
        var _S855 : vec3<f32> = _S690;
        s_bwdProp_mul_0(_S731, &(_S854), &(_S855), _S853);
        var _S856 : vec3<f32> = _S855;
        var _S857 : f32 = _S852 + _S840;
        if(_S728)
        {
            var _S858 : f32 = 0.0f;
            var _S859 : f32 = 0.0f;
            s_bwdProp_pow0_0(_S725, &(_S858), &(_S859), blue_0);
            scale_3 = 0.81815499067306519f * - (_S858 / _S723);
        }
        else
        {
            scale_3 = 0.0f;
        }
        var _S860 : vec3<f32> = _S849 + vec3<f32>(0.0f, 0.0f, scale_3);
        if(_S729)
        {
            scale_3 = _S857;
        }
        else
        {
            scale_3 = 0.0f;
        }
        lms_0 = _S856 + vec3<f32>(0.0f, scale_3, 0.0f);
        _S715 = _S860;
    }
    else
    {
        if(_S730)
        {
            var _S861 : vec3<f32> = _S722 * (vec3<f32>(0) - _S711.value0_16 * _S838 / _S721);
            lms_0 = _S720 * _S838;
            _S715 = _S861;
        }
        else
        {
            lms_0 = _S838;
            _S715 = _S690;
        }
    }
    var _S862 : mat3x3<f32> = _S691;
    var _S863 : vec3<f32> = _S690;
    s_bwdProp_mul_0(_S713, &(_S862), &(_S863), lms_0);
    var _S864 : vec3<f32> = _S690;
    s_bwdProp_clip_0(_S710, &(_S864), _S863);
    var _S865 : vec3<f32> = _S690;
    var _S866 : vec3<f32> = _S690;
    var _S867 : vec3<f32> = _S690;
    s_bwdProp_clamp_1(_S707, &(_S865), &(_S866), &(_S867), _S864);
    var _S868 : vec3<f32> = _S690;
    s_bwdProp_away_from_zero_1(_S702, &(_S868), _S715);
    var _S869 : mat3x3<f32> = _S691;
    var _S870 : vec3<f32> = _S690;
    s_bwdProp_mul_0(_S699, &(_S869), &(_S870), _S868);
    var _S871 : vec2<f32> = vec2<f32>(0.0f);
    s_bwdProp_illuminant_xy_to_xyz_0(_S696, &(_S871), _S870);
    var _S872 : Illuminant_0 = _S810;
    _S872.y_11 = _S871[i32(1)];
    _S872.x_17 = _S871[i32(0)];
    var _S873 : Illuminant_0 = Illuminant_x24_syn_dadd_0(_S810, _S872);
    (*_S687) = _S865;
    (*_S688) = _S873;
    return;
}

fn illuminant_0() -> Illuminant_0
{
    var i_0 : Illuminant_0;
    i_0.x_17 = params_0.mixer_0.x_0;
    i_0.y_11 = params_0.mixer_0.y_0;
    return i_0;
}

fn gamut_mapping_0( _S874 : vec3<f32>) -> vec3<f32>
{
    var _S875 : f32 = _S874.x;
    var _S876 : f32 = _S874.y;
    var _S877 : f32 = _S874.z;
    var sum_8 : f32 = _S875 + _S876 + _S877;
    var _S878 : f32 = max(sum_8, 9.99999997475242708e-07f * (abs(_S875) + abs(_S876) + abs(_S877)));
    var xy_6 : vec2<f32>;
    if((sum_8 > 0.0f))
    {
        xy_6 = vec2<f32>(_S875, _S876) / vec2<f32>(_S878);
    }
    else
    {
        xy_6 = vec2<f32>(0.34567001461982727f, 0.35850000381469727f);
    }
    var uv_6 : vec2<f32> = xy_to_uv_0(xy_6);
    var delta_2 : vec2<f32> = vec2<f32>(0.20915915071964264f, 0.48807531595230103f) - uv_6;
    var distance_2 : f32 = _S876 * dot(delta_2, delta_2);
    var _S879 : f32 = params_0.mixer_0.compression_0;
    var correction_2 : f32;
    var neutral_2 : bool;
    if((params_0.mixer_0.compression_0) != 0.0f)
    {
        if((distance_2 >= 0.0f))
        {
            correction_2 = bounded_power_0(distance_2, _S879);
            neutral_2 = false;
        }
        else
        {
            var magnitude_0 : f32 = bounded_power_0(- distance_2, _S879);
            var _S880 : u32 = params_0.mixer_0.negative_delta_0;
            if((params_0.mixer_0.negative_delta_0) == u32(1))
            {
                correction_2 = - magnitude_0;
            }
            else
            {
                correction_2 = magnitude_0;
            }
            neutral_2 = _S880 == u32(0);
        }
    }
    else
    {
        correction_2 = 0.0f;
        neutral_2 = false;
    }
    var moved_3 : vec2<f32> = vec2<f32>(min(correction_2, 1.0f)) * delta_2 + uv_6;
    var uv_7 : vec2<f32> = vec2<f32>(towards_white_0(uv_6.x, moved_3.x, 0.20915915071964264f), towards_white_0(uv_6.y, moved_3.y, 0.48807531595230103f));
    var uv_8 : vec2<f32>;
    if(neutral_2)
    {
        uv_8 = vec2<f32>(0.20915915071964264f, 0.48807531595230103f);
    }
    else
    {
        uv_8 = uv_7;
    }
    var _S881 : vec2<f32> = max(uv_to_xy_0(uv_8), vec2<f32>(params_0.mixer_0.clip_min_0));
    var _S882 : f32 = _S881.x;
    var _S883 : f32 = max(_S881.y, 1.52587890625e-05f);
    var xy_7 : vec2<f32> = vec2<f32>(_S882, _S883);
    var scale_4 : f32 = _S882 + _S883;
    if((scale_4 >= 1.0f))
    {
        xy_6 = xy_7 / vec2<f32>(scale_4);
    }
    else
    {
        xy_6 = xy_7;
    }
    var _S884 : f32 = xy_6.x;
    var _S885 : f32 = xy_6.y;
    return vec3<f32>(_S876 * _S884 / _S885, _S876, _S876 * (1.0f - _S884 - _S885) / _S885);
}

fn luma_chroma_0( _S886 : vec3<f32>) -> vec3<f32>
{
    var norm_3 : f32 = max(scaled_length_0(_S886), 1.52587890625e-05f) * 0.57735025882720947f;
    var ratios_2 : vec3<f32> = _S886 / vec3<f32>(norm_3);
    var _S887 : vec3<f32> = max((vec3<f32>(1.0f) - ratios_2) * vec3<f32>((dot(ratios_2, params_0.mixer_0.saturation_0.xyz) / 3.0f)) + ratios_2, min(ratios_2, vec3<f32>(0.0f)));
    return _S887 * vec3<f32>((norm_3 / (max(scaled_length_0(_S887), 1.52587890625e-05f) * 0.57735025882720947f) * clamp(1.0f + dot(_S886, params_0.mixer_0.lightness_0.xyz) / max((_S886.x + _S886.y + _S886.z) / 3.0f, 1.52587890625e-05f), 0.0f, 1.0e+06f)));
}

fn channel_mixer_0( _S888 : vec3<f32>,  _S889 : Illuminant_0) -> vec3<f32>
{
    var mix_1 : mat3x3<f32> = rows_0(params_0.mixer_0.mix_0_0, params_0.mixer_0.mix_1_0, params_0.mixer_0.mix_2_0);
    var cone_1 : mat3x3<f32> = rows_0(params_0.mixer_0.cone_0_0, params_0.mixer_0.cone_1_0, params_0.mixer_0.cone_2_0);
    var to_space_1 : mat3x3<f32> = rows_0(params_0.mixer_0.to_space_0_0, params_0.mixer_0.to_space_1_0, params_0.mixer_0.to_space_2_0);
    var from_space_1 : mat3x3<f32> = rows_0(params_0.mixer_0.from_space_0_0, params_0.mixer_0.from_space_1_0, params_0.mixer_0.from_space_2_0);
    var to_rgb_1 : mat3x3<f32> = rows_0(params_0.mixer_0.to_rgb_0_0, params_0.mixer_0.to_rgb_1_0, params_0.mixer_0.to_rgb_2_0);
    var lms_white_0 : vec3<f32> = away_from_zero_1((((illuminant_xy_to_xyz_0(vec2<f32>(_S889.x_17, _S889.y_11))) * (cone_1))), 1.52587890625e-05f);
    var _S890 : f32 = params_0.mixer_0.clip_min_0;
    var lms_1 : vec3<f32> = (((clip_0(clamp(_S888, vec3<f32>(-1.0e+08f), vec3<f32>(1.0e+08f)), params_0.mixer_0.clip_min_0)) * (rows_0(params_0.mixer_0.to_lms_0_0, params_0.mixer_0.to_lms_1_0, params_0.mixer_0.to_lms_2_0))));
    var _S891 : u32 = params_0.mixer_0.adaptation_0;
    var lms_2 : vec3<f32>;
    if((params_0.mixer_0.adaptation_0) == u32(2))
    {
        var _S892 : f32 = lms_1.y;
        var scale_5 : f32;
        if((_S892 > 1.52587890625e-05f))
        {
            scale_5 = _S892 + 1.52587890625e-05f;
        }
        else
        {
            scale_5 = 1.52587890625e-05f;
        }
        var _S893 : f32 = lms_white_0.z;
        var power_1 : f32;
        if((_S893 > 0.0f))
        {
            power_1 = pow0_0(0.81815499067306519f / _S893, 0.083400003612041473f);
        }
        else
        {
            power_1 = 1.0f;
        }
        var ratio_1 : vec3<f32> = (((lms_1) * (cone_1))) / vec3<f32>(scale_5) / lms_white_0;
        var _S894 : f32 = ratio_1.z;
        var blue_1 : f32;
        if((_S894 > 0.0f))
        {
            blue_1 = pow0_0(min(_S894, 1.0e+08f), power_1);
        }
        else
        {
            blue_1 = _S894;
        }
        lms_2 = params_0.mixer_0.d50_0.xyz * vec3<f32>(ratio_1.x, ratio_1.y, blue_1) * vec3<f32>(scale_5);
    }
    else
    {
        if(_S891 != u32(4))
        {
            lms_2 = lms_1 * (params_0.mixer_0.d50_0.xyz / lms_white_0);
        }
        else
        {
            lms_2 = lms_1;
        }
    }
    var _S895 : vec3<f32> = vec3<f32>(-9.99999995904e+11f);
    var _S896 : vec3<f32> = vec3<f32>(9.99999995904e+11f);
    var c_3 : vec3<f32> = clip_0(luma_chroma_0(clip_0((((clamp(gamut_mapping_0(clamp(clip_0((((lms_2) * (mix_1))), _S890), _S895, _S896)), _S895, _S896)) * (to_space_1))), _S890)), _S890);
    if((params_0.mixer_0.apply_grey_0) != u32(0))
    {
        return vec3<f32>(max(dot(c_3, params_0.mixer_0.grey_0.xyz), 0.0f));
    }
    return clip_0((((clip_0((((c_3) * (from_space_1))), _S890)) * (to_rgb_1))), _S890);
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_0 : vec3<u32>)
{
    var _S897 : u32 = id_0.x;
    var _S898 : bool;
    if((_S897 >= (params_0.pass_0.width_0)))
    {
        _S898 = true;
    }
    else
    {
        _S898 = ((id_0.y) >= (params_0.pass_0.height_0));
    }
    if(_S898)
    {
        return;
    }
    var i_1 : u32 = id_0.y * params_0.pass_0.width_0 + _S897;
    var pixel_0 : vec4<f32> = input_0[i_1];
    var rgb_0 : vec3<f32> = pixel_0.xyz;
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
        var _S899 : vec3<f32> = channel_mixer_0(rgb_0, illuminant_0());
        var _S900 : u32 = k_0 + u32(1);
        rgb_0 = _S899;
        k_0 = _S900;
    }
    output_0[i_1] = vec4<f32>(rgb_0, pixel_0.w);
    return;
}

struct DiffPair_Illuminant_0
{
     primal_0 : Illuminant_0,
     differential_0 : Illuminant_0,
};

fn _S901( _S902 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  _S903 : ptr<function, DiffPair_Illuminant_0>,  _S904 : vec3<f32>)
{
    var _S905 : s_paramCtx_s_bwdCallableCtx_channel_mixer_0;
    _S905.value0_50 = (*_S902).primal_0;
    _S905.value1_7.to_lms_0_0 = params_0.mixer_0.to_lms_0_0;
    _S905.value1_7.to_lms_1_0 = params_0.mixer_0.to_lms_1_0;
    _S905.value1_7.to_lms_2_0 = params_0.mixer_0.to_lms_2_0;
    _S905.value1_7.mix_0_0 = params_0.mixer_0.mix_0_0;
    _S905.value1_7.mix_1_0 = params_0.mixer_0.mix_1_0;
    _S905.value1_7.mix_2_0 = params_0.mixer_0.mix_2_0;
    _S905.value1_7.cone_0_0 = params_0.mixer_0.cone_0_0;
    _S905.value1_7.cone_1_0 = params_0.mixer_0.cone_1_0;
    _S905.value1_7.cone_2_0 = params_0.mixer_0.cone_2_0;
    _S905.value1_7.to_space_0_0 = params_0.mixer_0.to_space_0_0;
    _S905.value1_7.to_space_1_0 = params_0.mixer_0.to_space_1_0;
    _S905.value1_7.to_space_2_0 = params_0.mixer_0.to_space_2_0;
    _S905.value1_7.from_space_0_0 = params_0.mixer_0.from_space_0_0;
    _S905.value1_7.from_space_1_0 = params_0.mixer_0.from_space_1_0;
    _S905.value1_7.from_space_2_0 = params_0.mixer_0.from_space_2_0;
    _S905.value1_7.to_rgb_0_0 = params_0.mixer_0.to_rgb_0_0;
    _S905.value1_7.to_rgb_1_0 = params_0.mixer_0.to_rgb_1_0;
    _S905.value1_7.to_rgb_2_0 = params_0.mixer_0.to_rgb_2_0;
    _S905.value1_7.d50_0 = params_0.mixer_0.d50_0;
    _S905.value1_7.saturation_0 = params_0.mixer_0.saturation_0;
    _S905.value1_7.lightness_0 = params_0.mixer_0.lightness_0;
    _S905.value1_7.grey_0 = params_0.mixer_0.grey_0;
    _S905.value1_7.x_0 = params_0.mixer_0.x_0;
    _S905.value1_7.y_0 = params_0.mixer_0.y_0;
    _S905.value1_7.compression_0 = params_0.mixer_0.compression_0;
    _S905.value1_7.clip_min_0 = params_0.mixer_0.clip_min_0;
    _S905.value1_7.adaptation_0 = params_0.mixer_0.adaptation_0;
    _S905.value1_7.apply_grey_0 = params_0.mixer_0.apply_grey_0;
    _S905.value1_7.negative_delta_0 = params_0.mixer_0.negative_delta_0;
    _S905.value1_7.unused_0 = params_0.mixer_0.unused_0;
    _S905.value2_1 = (*_S903).primal_0;
    var _S906 : s_bwdCallableCtx_channel_mixer_0 = s_bwdCallableCtx_channel_mixer_0( _S905 );
    var _S907 : vec3<f32> = (*_S902).differential_0;
    var _S908 : Illuminant_0 = (*_S903).differential_0;
    s_bwdProp_channel_mixer_0(_S906, &(_S907), &(_S908), _S904);
    (*_S902).differential_0 = _S907;
    (*_S903).differential_0 = _S908;
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn channelmixerrgb_backward(@builtin(global_invocation_id) id_1 : vec3<u32>)
{
    var _S909 : u32 = id_1.x;
    var _S910 : bool;
    if((_S909 >= (params_0.pass_0.width_0)))
    {
        _S910 = true;
    }
    else
    {
        _S910 = ((id_1.y) >= (params_0.pass_0.height_0));
    }
    if(_S910)
    {
        return;
    }
    var i_2 : u32 = id_1.y * params_0.pass_0.width_0 + _S909;
    var _S911 : vec3<f32> = vec3<f32>(0.0f);
    var pixel_1 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    pixel_1.primal_0 = input_0[i_2].xyz;
    pixel_1.differential_0 = _S911;
    var _S912 : Illuminant_0 = Illuminant_x24_syn_dzero_0();
    var white_3 : DiffPair_Illuminant_0;
    white_3.primal_0 = illuminant_0();
    white_3.differential_0 = _S912;
    _S901(&(pixel_1), &(white_3), derivative_in_0[i_2].xyz);
    derivative_out_0[i_2] = vec4<f32>(white_3.differential_0.x_17, white_3.differential_0.y_11, 0.0f, 0.0f);
    return;
}

fn s_fwd_gamut_mapping_0( _S913 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S914 : f32 = _S913.primal_0.x;
    var _S915 : f32 = _S913.differential_0.x;
    var _S916 : f32 = _S913.primal_0.y;
    var _S917 : f32 = _S913.differential_0.y;
    var _S918 : f32 = _S913.primal_0.z;
    var _S919 : f32 = _S913.differential_0.z;
    var sum_9 : f32 = _S914 + _S916 + _S918;
    var _S920 : DiffPair_float_0 = DiffPair_float_0( _S914, _S915 );
    var _S921 : DiffPair_float_0 = _d_abs_1(_S920);
    var _S922 : DiffPair_float_0 = DiffPair_float_0( _S916, _S917 );
    var _S923 : DiffPair_float_0 = _d_abs_1(_S922);
    var _S924 : DiffPair_float_0 = DiffPair_float_0( _S918, _S919 );
    var _S925 : DiffPair_float_0 = _d_abs_1(_S924);
    var _S926 : DiffPair_float_0 = DiffPair_float_0( sum_9, _S915 + _S917 + _S919 );
    var _S927 : DiffPair_float_0 = DiffPair_float_0( 9.99999997475242708e-07f * (_S921.primal_0 + _S923.primal_0 + _S925.primal_0), (_S921.differential_0 + _S923.differential_0 + _S925.differential_0) * 9.99999997475242708e-07f );
    var _S928 : DiffPair_float_0 = _d_max_1(_S926, _S927);
    var xy_8 : vec2<f32>;
    var s_diff_xy_0 : vec2<f32>;
    if((sum_9 > 0.0f))
    {
        var _S929 : vec2<f32> = vec2<f32>(_S914, _S916);
        var _S930 : vec2<f32> = vec2<f32>(_S928.primal_0);
        var _S931 : vec2<f32> = (vec2<f32>(_S915, _S917) * _S930 - _S929 * vec2<f32>(_S928.differential_0)) / vec2<f32>((_S928.primal_0 * _S928.primal_0));
        xy_8 = _S929 / _S930;
        s_diff_xy_0 = _S931;
    }
    else
    {
        var _S932 : vec2<f32> = vec2<f32>(0.0f);
        xy_8 = vec2<f32>(0.34567001461982727f, 0.35850000381469727f);
        s_diff_xy_0 = _S932;
    }
    var _S933 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( xy_8, s_diff_xy_0 );
    var _S934 : DiffPair_vectorx3Cfloatx2C2x3E_0 = s_fwd_xy_to_uv_0(_S933);
    var delta_3 : vec2<f32> = vec2<f32>(0.20915915071964264f, 0.48807531595230103f) - _S934.primal_0;
    var _S935 : vec2<f32> = vec2<f32>(0.0f);
    var s_diff_delta_0 : vec2<f32> = _S935 - _S934.differential_0;
    var _S936 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( delta_3, s_diff_delta_0 );
    var _S937 : DiffPair_float_0 = _d_dot_3(_S936, _S936);
    var distance_3 : f32 = _S916 * _S937.primal_0;
    var s_diff_distance_0 : f32 = _S917 * _S937.primal_0 + _S937.differential_0 * _S916;
    var _S938 : f32 = params_0.mixer_0.compression_0;
    var correction_3 : f32;
    var s_diff_correction_0 : f32;
    var neutral_3 : bool;
    if((params_0.mixer_0.compression_0) != 0.0f)
    {
        if((distance_3 >= 0.0f))
        {
            var _S939 : DiffPair_float_0 = DiffPair_float_0( distance_3, s_diff_distance_0 );
            var _S940 : DiffPair_float_0 = s_fwd_bounded_power_0(_S939, _S938);
            correction_3 = _S940.primal_0;
            s_diff_correction_0 = _S940.differential_0;
            neutral_3 = false;
        }
        else
        {
            var _S941 : DiffPair_float_0 = DiffPair_float_0( - distance_3, - s_diff_distance_0 );
            var _S942 : DiffPair_float_0 = s_fwd_bounded_power_0(_S941, _S938);
            var _S943 : u32 = params_0.mixer_0.negative_delta_0;
            if((params_0.mixer_0.negative_delta_0) == u32(1))
            {
                var _S944 : f32 = - _S942.differential_0;
                correction_3 = - _S942.primal_0;
                s_diff_correction_0 = _S944;
            }
            else
            {
                correction_3 = _S942.primal_0;
                s_diff_correction_0 = _S942.differential_0;
            }
            neutral_3 = _S943 == u32(0);
        }
    }
    else
    {
        correction_3 = 0.0f;
        s_diff_correction_0 = 0.0f;
        neutral_3 = false;
    }
    var _S945 : DiffPair_float_0 = DiffPair_float_0( correction_3, s_diff_correction_0 );
    var _S946 : DiffPair_float_0 = DiffPair_float_0( 1.0f, 0.0f );
    var _S947 : DiffPair_float_0 = _d_min_1(_S945, _S946);
    var _S948 : vec2<f32> = vec2<f32>(_S947.primal_0);
    var moved_4 : vec2<f32> = _S948 * delta_3 + _S934.primal_0;
    var s_diff_moved_0 : vec2<f32> = vec2<f32>(_S947.differential_0) * delta_3 + s_diff_delta_0 * _S948 + _S934.differential_0;
    var _S949 : DiffPair_float_0 = DiffPair_float_0( _S934.primal_0.x, _S934.differential_0.x );
    var _S950 : DiffPair_float_0 = DiffPair_float_0( moved_4.x, s_diff_moved_0.x );
    var _S951 : DiffPair_float_0 = s_fwd_towards_white_0(_S949, _S950, 0.20915915071964264f);
    var _S952 : DiffPair_float_0 = DiffPair_float_0( _S934.primal_0.y, _S934.differential_0.y );
    var _S953 : DiffPair_float_0 = DiffPair_float_0( moved_4.y, s_diff_moved_0.y );
    var _S954 : DiffPair_float_0 = s_fwd_towards_white_0(_S952, _S953, 0.48807531595230103f);
    var uv_9 : vec2<f32> = vec2<f32>(_S951.primal_0, _S954.primal_0);
    var s_diff_uv_0 : vec2<f32> = vec2<f32>(_S951.differential_0, _S954.differential_0);
    var uv_10 : vec2<f32>;
    var s_diff_uv_1 : vec2<f32>;
    if(neutral_3)
    {
        uv_10 = vec2<f32>(0.20915915071964264f, 0.48807531595230103f);
        s_diff_uv_1 = _S935;
    }
    else
    {
        uv_10 = uv_9;
        s_diff_uv_1 = s_diff_uv_0;
    }
    var _S955 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( uv_10, s_diff_uv_1 );
    var _S956 : DiffPair_vectorx3Cfloatx2C2x3E_0 = s_fwd_uv_to_xy_0(_S955);
    var _S957 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( _S956.primal_0, _S956.differential_0 );
    var _S958 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( vec2<f32>(params_0.mixer_0.clip_min_0), _S935 );
    var _S959 : DiffPair_vectorx3Cfloatx2C2x3E_0 = _d_max_vector_2(_S957, _S958);
    var _S960 : f32 = _S959.primal_0.x;
    var _S961 : f32 = _S959.differential_0.x;
    var _S962 : DiffPair_float_0 = DiffPair_float_0( _S959.primal_0.y, _S959.differential_0.y );
    var _S963 : DiffPair_float_0 = DiffPair_float_0( 1.52587890625e-05f, 0.0f );
    var _S964 : DiffPair_float_0 = _d_max_1(_S962, _S963);
    var xy_9 : vec2<f32> = vec2<f32>(_S960, _S964.primal_0);
    var s_diff_xy_1 : vec2<f32> = vec2<f32>(_S961, _S964.differential_0);
    var scale_6 : f32 = _S960 + _S964.primal_0;
    var s_diff_scale_0 : f32 = _S961 + _S964.differential_0;
    if((scale_6 >= 1.0f))
    {
        var _S965 : vec2<f32> = vec2<f32>(scale_6);
        var _S966 : vec2<f32> = (s_diff_xy_1 * _S965 - xy_9 * vec2<f32>(s_diff_scale_0)) / vec2<f32>((scale_6 * scale_6));
        xy_8 = xy_9 / _S965;
        s_diff_xy_0 = _S966;
    }
    else
    {
        xy_8 = xy_9;
        s_diff_xy_0 = s_diff_xy_1;
    }
    var _S967 : f32 = xy_8.x;
    var _S968 : f32 = s_diff_xy_0.x;
    var _S969 : f32 = _S916 * _S967;
    var _S970 : f32 = xy_8.y;
    var _S971 : f32 = s_diff_xy_0.y;
    var _S972 : f32 = _S970 * _S970;
    var _S973 : f32 = 1.0f - _S967 - _S970;
    var _S974 : f32 = _S916 * _S973;
    var _S975 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S969 / _S970, _S916, _S974 / _S970), vec3<f32>(((_S917 * _S967 + _S968 * _S916) * _S970 - _S969 * _S971) / _S972, _S917, ((_S917 * _S973 + (0.0f - _S968 - _S971) * _S916) * _S970 - _S974 * _S971) / _S972) );
    return _S975;
}

fn s_fwd_luma_chroma_0( _S976 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S977 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S976.primal_0, _S976.differential_0 );
    var _S978 : DiffPair_float_0 = s_fwd_scaled_length_0(_S977);
    var _S979 : DiffPair_float_0 = DiffPair_float_0( _S978.primal_0, _S978.differential_0 );
    var _S980 : DiffPair_float_0 = DiffPair_float_0( 1.52587890625e-05f, 0.0f );
    var _S981 : DiffPair_float_0 = _d_max_1(_S979, _S980);
    var _S982 : DiffPair_float_0 = DiffPair_float_0( (_S976.primal_0.x + _S976.primal_0.y + _S976.primal_0.z) / 3.0f, (_S976.differential_0.x + _S976.differential_0.y + _S976.differential_0.z) * 0.3333333432674408f );
    var _S983 : DiffPair_float_0 = _d_max_1(_S982, _S980);
    var _S984 : vec3<f32> = vec3<f32>(0.0f);
    var _S985 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( params_0.mixer_0.lightness_0.xyz, _S984 );
    var _S986 : DiffPair_float_0 = _d_dot_2(_S977, _S985);
    var norm_4 : f32 = _S981.primal_0 * 0.57735025882720947f;
    var _S987 : f32 = _S981.differential_0 * 0.57735025882720947f;
    var _S988 : vec3<f32> = vec3<f32>(norm_4);
    var ratios_3 : vec3<f32> = _S976.primal_0 / _S988;
    var s_diff_ratios_0 : vec3<f32> = (_S976.differential_0 * _S988 - _S976.primal_0 * vec3<f32>(_S987)) / vec3<f32>((norm_4 * norm_4));
    var _S989 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( ratios_3, s_diff_ratios_0 );
    var _S990 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( params_0.mixer_0.saturation_0.xyz, _S984 );
    var _S991 : DiffPair_float_0 = _d_dot_2(_S989, _S990);
    var _S992 : vec3<f32> = vec3<f32>(1.0f) - ratios_3;
    var _S993 : vec3<f32> = vec3<f32>((_S991.primal_0 / 3.0f));
    var _S994 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.0f), _S984 );
    var _S995 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_min_vector_1(_S989, _S994);
    var _S996 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S992 * _S993 + ratios_3, (_S984 - s_diff_ratios_0) * _S993 + vec3<f32>((_S991.differential_0 * 0.3333333432674408f)) * _S992 + s_diff_ratios_0 );
    var _S997 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S995.primal_0, _S995.differential_0 );
    var _S998 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_max_vector_3(_S996, _S997);
    var _S999 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S998.primal_0, _S998.differential_0 );
    var _S1000 : DiffPair_float_0 = s_fwd_scaled_length_0(_S999);
    var _S1001 : DiffPair_float_0 = DiffPair_float_0( _S1000.primal_0, _S1000.differential_0 );
    var _S1002 : DiffPair_float_0 = _d_max_1(_S1001, _S980);
    var _S1003 : f32 = _S1002.primal_0 * 0.57735025882720947f;
    var norm_5 : f32 = norm_4 / _S1003;
    var _S1004 : DiffPair_float_0 = DiffPair_float_0( 1.0f + _S986.primal_0 / _S983.primal_0, (_S986.differential_0 * _S983.primal_0 - _S986.primal_0 * _S983.differential_0) / (_S983.primal_0 * _S983.primal_0) );
    var _S1005 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S1006 : DiffPair_float_0 = DiffPair_float_0( 1.0e+06f, 0.0f );
    var _S1007 : DiffPair_float_0 = _d_clamp_1(_S1004, _S1005, _S1006);
    var _S1008 : vec3<f32> = vec3<f32>((norm_5 * _S1007.primal_0));
    var _S1009 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S998.primal_0 * _S1008, _S998.differential_0 * _S1008 + vec3<f32>(((_S987 * _S1003 - norm_4 * (_S1002.differential_0 * 0.57735025882720947f)) / (_S1003 * _S1003) * _S1007.primal_0 + _S1007.differential_0 * norm_5)) * _S998.primal_0 );
    return _S1009;
}

fn s_fwd_channel_mixer_0( _S1010 : DiffPair_vectorx3Cfloatx2C3x3E_0,  _S1011 : DiffPair_Illuminant_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var to_lms_1 : mat3x3<f32> = rows_0(params_0.mixer_0.to_lms_0_0, params_0.mixer_0.to_lms_1_0, params_0.mixer_0.to_lms_2_0);
    var mix_2 : mat3x3<f32> = rows_0(params_0.mixer_0.mix_0_0, params_0.mixer_0.mix_1_0, params_0.mixer_0.mix_2_0);
    var cone_2 : mat3x3<f32> = rows_0(params_0.mixer_0.cone_0_0, params_0.mixer_0.cone_1_0, params_0.mixer_0.cone_2_0);
    var to_space_2 : mat3x3<f32> = rows_0(params_0.mixer_0.to_space_0_0, params_0.mixer_0.to_space_1_0, params_0.mixer_0.to_space_2_0);
    var from_space_2 : mat3x3<f32> = rows_0(params_0.mixer_0.from_space_0_0, params_0.mixer_0.from_space_1_0, params_0.mixer_0.from_space_2_0);
    var to_rgb_2 : mat3x3<f32> = rows_0(params_0.mixer_0.to_rgb_0_0, params_0.mixer_0.to_rgb_1_0, params_0.mixer_0.to_rgb_2_0);
    var _S1012 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( vec2<f32>(_S1011.primal_0.x_17, _S1011.primal_0.y_11), vec2<f32>(_S1011.differential_0.x_17, _S1011.differential_0.y_11) );
    var _S1013 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_illuminant_xy_to_xyz_0(_S1012);
    var _S1014 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S1015 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( (((_S1013.primal_0) * (cone_2))), (((_S1013.primal_0) * (_S1014))) + (((_S1013.differential_0) * (cone_2))) );
    var _S1016 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_away_from_zero_1(_S1015, 1.52587890625e-05f);
    var _S1017 : vec3<f32> = vec3<f32>(0.0f);
    var _S1018 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S1010.primal_0, _S1010.differential_0 );
    var _S1019 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(-1.0e+08f), _S1017 );
    var _S1020 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(1.0e+08f), _S1017 );
    var _S1021 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_clamp_vector_1(_S1018, _S1019, _S1020);
    var _S1022 : f32 = params_0.mixer_0.clip_min_0;
    var _S1023 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S1021.primal_0, _S1021.differential_0 );
    var _S1024 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_clip_0(_S1023, params_0.mixer_0.clip_min_0);
    var _S1025 : vec3<f32> = (((_S1024.primal_0) * (to_lms_1)));
    var _S1026 : vec3<f32> = (((_S1024.primal_0) * (_S1014))) + (((_S1024.differential_0) * (to_lms_1)));
    var _S1027 : u32 = params_0.mixer_0.adaptation_0;
    var lms_3 : vec3<f32>;
    var s_diff_lms_0 : vec3<f32>;
    if((params_0.mixer_0.adaptation_0) == u32(2))
    {
        var _S1028 : f32 = _S1025.y;
        var _S1029 : f32 = _S1026.y;
        var scale_7 : f32;
        var s_diff_scale_1 : f32;
        if((_S1028 > 1.52587890625e-05f))
        {
            scale_7 = _S1028 + 1.52587890625e-05f;
            s_diff_scale_1 = _S1029;
        }
        else
        {
            scale_7 = 1.52587890625e-05f;
            s_diff_scale_1 = 0.0f;
        }
        var _S1030 : f32 = _S1016.primal_0.z;
        var _S1031 : f32 = _S1016.differential_0.z;
        var power_2 : f32;
        var s_diff_power_0 : f32;
        if((_S1030 > 0.0f))
        {
            var _S1032 : DiffPair_float_0 = DiffPair_float_0( 0.81815499067306519f / _S1030, (0.0f - 0.81815499067306519f * _S1031) / (_S1030 * _S1030) );
            var _S1033 : DiffPair_float_0 = DiffPair_float_0( 0.083400003612041473f, 0.0f );
            var _S1034 : DiffPair_float_0 = pow0_forward_0(_S1032, _S1033);
            power_2 = _S1034.primal_0;
            s_diff_power_0 = _S1034.differential_0;
        }
        else
        {
            power_2 = 1.0f;
            s_diff_power_0 = 0.0f;
        }
        var _S1035 : vec3<f32> = (((_S1025) * (cone_2)));
        var _S1036 : vec3<f32> = _S1035 / vec3<f32>(scale_7);
        var ratio_2 : vec3<f32> = _S1036 / _S1016.primal_0;
        var s_diff_ratio_0 : vec3<f32> = ((((((_S1025) * (_S1014))) + (((_S1026) * (cone_2)))) * vec3<f32>(scale_7) - _S1035 * vec3<f32>(s_diff_scale_1)) / vec3<f32>((scale_7 * scale_7)) * _S1016.primal_0 - _S1036 * _S1016.differential_0) / (_S1016.primal_0 * _S1016.primal_0);
        var _S1037 : f32 = ratio_2.z;
        var _S1038 : f32 = s_diff_ratio_0.z;
        var blue_2 : f32;
        var s_diff_blue_0 : f32;
        if((_S1037 > 0.0f))
        {
            var _S1039 : DiffPair_float_0 = DiffPair_float_0( _S1037, _S1038 );
            var _S1040 : DiffPair_float_0 = DiffPair_float_0( 1.0e+08f, 0.0f );
            var _S1041 : DiffPair_float_0 = _d_min_1(_S1039, _S1040);
            var _S1042 : DiffPair_float_0 = DiffPair_float_0( _S1041.primal_0, _S1041.differential_0 );
            var _S1043 : DiffPair_float_0 = DiffPair_float_0( power_2, s_diff_power_0 );
            var _S1044 : DiffPair_float_0 = pow0_forward_0(_S1042, _S1043);
            blue_2 = _S1044.primal_0;
            s_diff_blue_0 = _S1044.differential_0;
        }
        else
        {
            blue_2 = _S1037;
            s_diff_blue_0 = _S1038;
        }
        var _S1045 : vec3<f32> = params_0.mixer_0.d50_0.xyz;
        var _S1046 : vec3<f32> = _S1045 * vec3<f32>(ratio_2.x, ratio_2.y, blue_2);
        var _S1047 : vec3<f32> = vec3<f32>(s_diff_ratio_0.x, s_diff_ratio_0.y, s_diff_blue_0) * _S1045 * vec3<f32>(scale_7) + vec3<f32>(s_diff_scale_1) * _S1046;
        lms_3 = _S1046 * vec3<f32>(scale_7);
        s_diff_lms_0 = _S1047;
    }
    else
    {
        if(_S1027 != u32(4))
        {
            var _S1048 : vec3<f32> = params_0.mixer_0.d50_0.xyz;
            var _S1049 : vec3<f32> = _S1048 / _S1016.primal_0;
            var _S1050 : vec3<f32> = _S1026 * _S1049 + (_S1017 - _S1048 * _S1016.differential_0) / (_S1016.primal_0 * _S1016.primal_0) * _S1025;
            lms_3 = _S1025 * _S1049;
            s_diff_lms_0 = _S1050;
        }
        else
        {
            lms_3 = _S1025;
            s_diff_lms_0 = _S1026;
        }
    }
    var _S1051 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( (((lms_3) * (mix_2))), (((lms_3) * (_S1014))) + (((s_diff_lms_0) * (mix_2))) );
    var _S1052 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_clip_0(_S1051, _S1022);
    var _S1053 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S1052.primal_0, _S1052.differential_0 );
    var _S1054 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(-9.99999995904e+11f), _S1017 );
    var _S1055 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(9.99999995904e+11f), _S1017 );
    var _S1056 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_clamp_vector_1(_S1053, _S1054, _S1055);
    var _S1057 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S1056.primal_0, _S1056.differential_0 );
    var _S1058 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_gamut_mapping_0(_S1057);
    var _S1059 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S1058.primal_0, _S1058.differential_0 );
    var _S1060 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_clamp_vector_1(_S1059, _S1054, _S1055);
    var _S1061 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( (((_S1060.primal_0) * (to_space_2))), (((_S1060.primal_0) * (_S1014))) + (((_S1060.differential_0) * (to_space_2))) );
    var _S1062 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_clip_0(_S1061, _S1022);
    var _S1063 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S1062.primal_0, _S1062.differential_0 );
    var _S1064 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_luma_chroma_0(_S1063);
    var _S1065 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S1064.primal_0, _S1064.differential_0 );
    var _S1066 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_clip_0(_S1065, _S1022);
    if((params_0.mixer_0.apply_grey_0) != u32(0))
    {
        var _S1067 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S1066.primal_0, _S1066.differential_0 );
        var _S1068 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( params_0.mixer_0.grey_0.xyz, _S1017 );
        var _S1069 : DiffPair_float_0 = _d_dot_2(_S1067, _S1068);
        var _S1070 : DiffPair_float_0 = DiffPair_float_0( _S1069.primal_0, _S1069.differential_0 );
        var _S1071 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
        var _S1072 : DiffPair_float_0 = _d_max_1(_S1070, _S1071);
        var _S1073 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S1072.primal_0), vec3<f32>(_S1072.differential_0) );
        return _S1073;
    }
    var _S1074 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( (((_S1066.primal_0) * (from_space_2))), (((_S1066.primal_0) * (_S1014))) + (((_S1066.differential_0) * (from_space_2))) );
    var _S1075 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_clip_0(_S1074, _S1022);
    var _S1076 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( (((_S1075.primal_0) * (to_rgb_2))), (((_S1075.primal_0) * (_S1014))) + (((_S1075.differential_0) * (to_rgb_2))) );
    var _S1077 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_clip_0(_S1076, _S1022);
    var _S1078 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S1077.primal_0, _S1077.differential_0 );
    return _S1078;
}

@compute
@workgroup_size(16, 16, 1)
fn channelmixerrgb_forward_diff(@builtin(global_invocation_id) id_2 : vec3<u32>)
{
    var _S1079 : u32 = id_2.x;
    var _S1080 : bool;
    if((_S1079 >= (params_0.pass_0.width_0)))
    {
        _S1080 = true;
    }
    else
    {
        _S1080 = ((id_2.y) >= (params_0.pass_0.height_0));
    }
    if(_S1080)
    {
        return;
    }
    var i_3 : u32 = id_2.y * params_0.pass_0.width_0 + _S1079;
    var v_5 : vec4<f32> = derivative_in_0[i_3];
    var direction_0 : Illuminant_0;
    direction_0.x_17 = v_5.x;
    direction_0.y_11 = v_5.y;
    var _S1081 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( input_0[i_3].xyz, vec3<f32>(0.0f) );
    var _S1082 : DiffPair_Illuminant_0 = DiffPair_Illuminant_0( illuminant_0(), direction_0 );
    derivative_out_0[i_3] = vec4<f32>(s_fwd_channel_mixer_0(_S1081, _S1082).differential_0, 0.0f);
    return;
}

