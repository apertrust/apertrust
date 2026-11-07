// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2019-2026  darktable developers (ported from src/iop/rgblevels.c)
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

// Generated from slang/rgblevels.slang by tools/build-kernels.sh. Do not edit.

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

struct RgbLevelsParams_std140_0
{
    @align(16) black_0 : vec4<f32>,
    @align(16) grey_0 : vec4<f32>,
    @align(16) white_0 : vec4<f32>,
    @align(16) luminance_0 : vec4<f32>,
    @align(16) autoscale_0 : u32,
    @align(4) preserve_colors_0 : u32,
    @align(8) pad0_0 : u32,
    @align(4) pad1_0 : u32,
    @align(16) gradient_channel_0 : vec4<u32>,
    @align(16) gradient_point_0 : vec4<u32>,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) rgb_levels_0 : RgbLevelsParams_std140_0,
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

fn s_apply_max_0( x_0 : f32,  y_0 : f32) -> Tuple_0
{
    var _S1 : Tuple_0 = Tuple_0( max(x_0, y_0) );
    return _S1;
}

struct s_bwdCallableCtx_max_0
{
     _S2 : f32,
     _S3 : f32,
};

struct DiffPair_float_0
{
     primal_0 : f32,
     differential_0 : f32,
};

fn _d_max_0( dpx_0 : ptr<function, DiffPair_float_0>,  dpy_0 : ptr<function, DiffPair_float_0>,  dOut_0 : f32)
{
    var _S4 : DiffPair_float_0 = (*dpx_0);
    var _S5 : f32;
    if((((*dpx_0).primal_0) > ((*dpy_0).primal_0)))
    {
        _S5 = dOut_0;
    }
    else
    {
        if((((*dpx_0).primal_0) < ((*dpy_0).primal_0)))
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
    if((((*dpy_0).primal_0) > (_S4.primal_0)))
    {
        _S5 = dOut_0;
    }
    else
    {
        if((((*dpy_0).primal_0) < ((*dpx_0).primal_0)))
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

fn s_bwdProp_max_0( ctx_0 : s_bwdCallableCtx_max_0,  d_x_0 : ptr<function, f32>,  d_y_0 : ptr<function, f32>,  d_out_0 : f32)
{
    var _S7 : DiffPair_float_0;
    _S7.primal_0 = ctx_0._S2;
    var _S8 : DiffPair_float_0;
    _S8.primal_0 = ctx_0._S3;
    _d_max_0(&(_S7), &(_S8), d_out_0);
    (*d_x_0) = _S7.differential_0;
    (*d_y_0) = _S8.differential_0;
    return;
}

struct Tuple_1
{
     value0_1 : f32,
};

fn pow0_0( x_1 : f32,  y_1 : f32) -> f32
{
    if((x_1 <= 0.0f))
    {
        return 0.0f;
    }
    return pow(x_1, y_1);
}

fn s_apply_pow0_0( x_2 : f32,  y_2 : f32) -> Tuple_1
{
    var _S9 : Tuple_1 = Tuple_1( pow0_0(x_2, y_2) );
    return _S9;
}

struct s_bwdCallableCtx_pow0_0
{
     _S10 : f32,
     _S11 : f32,
};

fn pow0_backward_0( x_3 : ptr<function, DiffPair_float_0>,  y_3 : ptr<function, DiffPair_float_0>,  d_out_1 : f32)
{
    if((((*x_3).primal_0) <= 0.0f))
    {
        (*x_3).primal_0 = (*x_3).primal_0;
        (*x_3).differential_0 = 0.0f;
        (*y_3).primal_0 = (*y_3).primal_0;
        (*y_3).differential_0 = 0.0f;
        return;
    }
    var d_x_1 : f32 = d_out_1 * (*y_3).primal_0 * pow((*x_3).primal_0, (*y_3).primal_0 - 1.0f);
    var d_y_1 : f32 = d_out_1 * pow((*x_3).primal_0, (*y_3).primal_0) * log((*x_3).primal_0);
    (*x_3).primal_0 = (*x_3).primal_0;
    (*x_3).differential_0 = d_x_1;
    (*y_3).primal_0 = (*y_3).primal_0;
    (*y_3).differential_0 = d_y_1;
    return;
}

fn s_bwdProp_pow0_0( ctx_1 : s_bwdCallableCtx_pow0_0,  d_x_2 : ptr<function, f32>,  d_y_2 : ptr<function, f32>,  d_out_2 : f32)
{
    var _S12 : DiffPair_float_0;
    _S12.primal_0 = ctx_1._S10;
    var _S13 : DiffPair_float_0;
    _S13.primal_0 = ctx_1._S11;
    pow0_backward_0(&(_S12), &(_S13), d_out_2);
    (*d_x_2) = _S12.differential_0;
    (*d_y_2) = _S13.differential_0;
    return;
}

struct Tuple_2
{
     value0_2 : f32,
};

fn s_apply_min_0( x_4 : f32,  y_4 : f32) -> Tuple_2
{
    var _S14 : Tuple_2 = Tuple_2( min(x_4, y_4) );
    return _S14;
}

struct s_bwdCallableCtx_min_0
{
     _S15 : f32,
     _S16 : f32,
};

fn _d_min_0( dpx_1 : ptr<function, DiffPair_float_0>,  dpy_1 : ptr<function, DiffPair_float_0>,  dOut_1 : f32)
{
    var _S17 : DiffPair_float_0 = (*dpx_1);
    var _S18 : f32;
    if((((*dpx_1).primal_0) < ((*dpy_1).primal_0)))
    {
        _S18 = dOut_1;
    }
    else
    {
        if((((*dpx_1).primal_0) > ((*dpy_1).primal_0)))
        {
            _S18 = 0.0f;
        }
        else
        {
            _S18 = 0.5f * dOut_1;
        }
    }
    (*dpx_1).primal_0 = _S17.primal_0;
    (*dpx_1).differential_0 = _S18;
    var _S19 : DiffPair_float_0 = (*dpy_1);
    if((((*dpy_1).primal_0) < (_S17.primal_0)))
    {
        _S18 = dOut_1;
    }
    else
    {
        if((((*dpy_1).primal_0) > ((*dpx_1).primal_0)))
        {
            _S18 = 0.0f;
        }
        else
        {
            _S18 = 0.5f * dOut_1;
        }
    }
    (*dpy_1).primal_0 = _S19.primal_0;
    (*dpy_1).differential_0 = _S18;
    return;
}

fn s_bwdProp_min_0( ctx_2 : s_bwdCallableCtx_min_0,  d_x_3 : ptr<function, f32>,  d_y_3 : ptr<function, f32>,  d_out_3 : f32)
{
    var _S20 : DiffPair_float_0;
    _S20.primal_0 = ctx_2._S15;
    var _S21 : DiffPair_float_0;
    _S21.primal_0 = ctx_2._S16;
    _d_min_0(&(_S20), &(_S21), d_out_3);
    (*d_x_3) = _S20.differential_0;
    (*d_y_3) = _S21.differential_0;
    return;
}

struct Tuple_3
{
     value0_3 : vec3<f32>,
};

fn s_apply_exp2_0( x_5 : vec3<f32>) -> Tuple_3
{
    var _S22 : Tuple_3 = Tuple_3( exp2(x_5) );
    return _S22;
}

struct s_bwdCallableCtx_exp2_0
{
     _S23 : vec3<f32>,
};

struct DiffPair_vectorx3Cfloatx2C3x3E_0
{
     primal_0 : vec3<f32>,
     differential_0 : vec3<f32>,
};

fn _d_exp2_vector_0( dpx_2 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_2 : vec3<f32>)
{
    var _S24 : vec3<f32> = exp2((*dpx_2).primal_0) * vec3<f32>(0.69314718246459961f) * dOut_2;
    (*dpx_2).primal_0 = (*dpx_2).primal_0;
    (*dpx_2).differential_0 = _S24;
    return;
}

fn s_bwdProp_exp2_0( ctx_3 : s_bwdCallableCtx_exp2_0,  d_x_4 : ptr<function, vec3<f32>>,  d_out_4 : vec3<f32>)
{
    var _S25 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S25.primal_0 = ctx_3._S23;
    _d_exp2_vector_0(&(_S25), d_out_4);
    (*d_x_4) = _S25.differential_0;
    return;
}

struct Tuple_4
{
     value0_4 : vec3<f32>,
};

fn s_apply_max_1( x_6 : vec3<f32>,  y_5 : vec3<f32>) -> Tuple_4
{
    var _S26 : Tuple_4 = Tuple_4( max(x_6, y_5) );
    return _S26;
}

struct s_bwdCallableCtx_max_1
{
     _S27 : vec3<f32>,
     _S28 : vec3<f32>,
};

fn _d_max_vector_0( dpx_3 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dpy_2 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_3 : vec3<f32>)
{
    var left_dp_0 : DiffPair_float_0;
    left_dp_0.primal_0 = (*dpx_3).primal_0[i32(0)];
    left_dp_0.differential_0 = 0.0f;
    var right_dp_0 : DiffPair_float_0;
    right_dp_0.primal_0 = (*dpy_2).primal_0[i32(0)];
    right_dp_0.differential_0 = 0.0f;
    _d_max_0(&(left_dp_0), &(right_dp_0), dOut_3[i32(0)]);
    var left_d_result_0 : vec3<f32>;
    left_d_result_0[i32(0)] = left_dp_0.differential_0;
    var right_d_result_0 : vec3<f32>;
    right_d_result_0[i32(0)] = right_dp_0.differential_0;
    var left_dp_1 : DiffPair_float_0;
    left_dp_1.primal_0 = (*dpx_3).primal_0[i32(1)];
    left_dp_1.differential_0 = 0.0f;
    var right_dp_1 : DiffPair_float_0;
    right_dp_1.primal_0 = (*dpy_2).primal_0[i32(1)];
    right_dp_1.differential_0 = 0.0f;
    _d_max_0(&(left_dp_1), &(right_dp_1), dOut_3[i32(1)]);
    left_d_result_0[i32(1)] = left_dp_1.differential_0;
    right_d_result_0[i32(1)] = right_dp_1.differential_0;
    var left_dp_2 : DiffPair_float_0;
    left_dp_2.primal_0 = (*dpx_3).primal_0[i32(2)];
    left_dp_2.differential_0 = 0.0f;
    var right_dp_2 : DiffPair_float_0;
    right_dp_2.primal_0 = (*dpy_2).primal_0[i32(2)];
    right_dp_2.differential_0 = 0.0f;
    _d_max_0(&(left_dp_2), &(right_dp_2), dOut_3[i32(2)]);
    left_d_result_0[i32(2)] = left_dp_2.differential_0;
    right_d_result_0[i32(2)] = right_dp_2.differential_0;
    (*dpx_3).primal_0 = (*dpx_3).primal_0;
    (*dpx_3).differential_0 = left_d_result_0;
    (*dpy_2).primal_0 = (*dpy_2).primal_0;
    (*dpy_2).differential_0 = right_d_result_0;
    return;
}

fn s_bwdProp_max_1( ctx_4 : s_bwdCallableCtx_max_1,  d_x_5 : ptr<function, vec3<f32>>,  d_y_4 : ptr<function, vec3<f32>>,  d_out_5 : vec3<f32>)
{
    var _S29 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S29.primal_0 = ctx_4._S27;
    var _S30 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S30.primal_0 = ctx_4._S28;
    _d_max_vector_0(&(_S29), &(_S30), d_out_5);
    (*d_x_5) = _S29.differential_0;
    (*d_y_4) = _S30.differential_0;
    return;
}

struct Tuple_5
{
     value0_5 : vec3<f32>,
};

fn s_apply_exp_0( x_7 : vec3<f32>) -> Tuple_5
{
    var _S31 : Tuple_5 = Tuple_5( exp(x_7) );
    return _S31;
}

struct s_bwdCallableCtx_exp_0
{
     _S32 : vec3<f32>,
};

fn _d_exp_vector_0( dpx_4 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_4 : vec3<f32>)
{
    var _S33 : vec3<f32> = exp((*dpx_4).primal_0) * dOut_4;
    (*dpx_4).primal_0 = (*dpx_4).primal_0;
    (*dpx_4).differential_0 = _S33;
    return;
}

fn s_bwdProp_exp_0( ctx_5 : s_bwdCallableCtx_exp_0,  d_x_6 : ptr<function, vec3<f32>>,  d_out_6 : vec3<f32>)
{
    var _S34 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S34.primal_0 = ctx_5._S32;
    _d_exp_vector_0(&(_S34), d_out_6);
    (*d_x_6) = _S34.differential_0;
    return;
}

struct Lookup_0
{
     mode_0 : u32,
     norm_0 : f32,
     step_0 : vec3<f32>,
};

struct Levels_0
{
     black_1 : vec3<f32>,
     grey_1 : vec3<f32>,
     white_1 : vec3<f32>,
};

struct DiffPair_Levels_0
{
     primal_0 : Levels_0,
     differential_0 : Levels_0,
};

struct s_paramCtx_s_bwdCallableCtx_rgb_levels_from_levels_0
{
     value0_6 : vec3<f32>,
     value1_0 : Lookup_0,
     value2_0 : bool,
     value3_0 : Levels_0,
};

struct s_bwdCallableCtx_rgb_levels_from_levels_0
{
     value0_7 : s_paramCtx_s_bwdCallableCtx_rgb_levels_from_levels_0,
};

struct Shape_0
{
     black_2 : vec3<f32>,
     scale_1 : vec3<f32>,
     power_0 : vec3<f32>,
     limit_0 : vec3<f32>,
};

struct Tuple_6
{
     value0_8 : Shape_0,
};

fn s_apply_shape_0( _S35 : Levels_0,  linked_0 : bool) -> Tuple_6
{
    var white_2 : vec3<f32>;
    var black_3 : vec3<f32>;
    var grey_2 : vec3<f32>;
    if(linked_0)
    {
        var _S36 : vec3<f32> = _S35.black_1.xxx;
        var _S37 : vec3<f32> = _S35.grey_1.xxx;
        white_2 = _S35.white_1.xxx;
        black_3 = _S36;
        grey_2 = _S37;
    }
    else
    {
        white_2 = _S35.white_1;
        black_3 = _S35.black_1;
        grey_2 = _S35.grey_1;
    }
    var _S38 : vec3<f32> = white_2 - black_3;
    var delta_0 : vec3<f32> = _S38 / vec3<f32>(2.0f);
    var _S39 : vec3<f32> = vec3<f32>(1.0f) / _S38;
    var _S40 : Tuple_5 = s_apply_exp_0((grey_2 - (black_3 + delta_0)) / delta_0 * vec3<f32>(2.3025851249694824f));
    var _S41 : Tuple_3 = s_apply_exp2_0(vec3<f32>(64.0f) / s_apply_max_1(_S40.value0_5, vec3<f32>(1.0f)).value0_4);
    var _S42 : Tuple_6;
    _S42.value0_8.black_2 = black_3;
    _S42.value0_8.scale_1 = _S39;
    _S42.value0_8.power_0 = _S40.value0_5;
    _S42.value0_8.limit_0 = _S41.value0_3;
    return _S42;
}

struct s_paramCtx_s_bwdCallableCtx_shape_0
{
     value0_9 : Levels_0,
     value1_1 : bool,
};

struct s_bwdCallableCtx_shape_0
{
     value0_10 : s_paramCtx_s_bwdCallableCtx_shape_0,
};

struct s_paramCtx_s_bwdCallableCtx_rgb_levels_0
{
     value0_11 : vec3<f32>,
     value1_2 : Lookup_0,
     value2_1 : Shape_0,
};

struct s_bwdCallableCtx_rgb_levels_0
{
     value0_12 : s_paramCtx_s_bwdCallableCtx_rgb_levels_0,
};

fn Levels_x24_syn_dzero_0() -> Levels_0
{
    var result_0 : Levels_0;
    var _S43 : vec3<f32> = vec3<f32>(0.0f);
    result_0.black_1 = _S43;
    result_0.grey_1 = _S43;
    result_0.white_1 = _S43;
    return result_0;
}

struct s_paramCtx_s_bwdCallableCtx_ratio_0
{
     value0_13 : f32,
     value1_3 : f32,
     value2_2 : f32,
     value3_1 : f32,
     value4_0 : f32,
     value5_0 : f32,
};

struct s_paramCtx_s_bwdCallableCtx_level_0
{
     value0_14 : f32,
     value1_4 : f32,
     value2_3 : f32,
     value3_2 : f32,
     value4_1 : f32,
     value5_1 : f32,
};

struct s_bwdCallableCtx_ratio_0
{
     value0_15 : s_paramCtx_s_bwdCallableCtx_ratio_0,
};

struct s_bwdCallableCtx_level_0
{
     value0_16 : s_paramCtx_s_bwdCallableCtx_level_0,
};

fn Shape_x24_syn_dzero_0() -> Shape_0
{
    var result_1 : Shape_0;
    var _S44 : vec3<f32> = vec3<f32>(0.0f);
    result_1.black_2 = _S44;
    result_1.scale_1 = _S44;
    result_1.power_0 = _S44;
    result_1.limit_0 = _S44;
    return result_1;
}

struct s_paramCtx_s_bwdCallableCtx_with_table_value_0
{
     value0_17 : f32,
};

struct s_bwdCallableCtx_with_table_value_0
{
     value0_18 : s_paramCtx_s_bwdCallableCtx_with_table_value_0,
};

fn s_bwdProp_with_table_value_0( _S45 : s_bwdCallableCtx_with_table_value_0,  _S46 : ptr<function, f32>,  _S47 : ptr<function, f32>,  _S48 : f32)
{
    var _S49 : bool = ((_S45.value0_18.value0_17) < 0.0f);
    var _S50 : f32;
    var _S51 : f32;
    if(!_S49)
    {
        _S50 = 0.0f;
        _S51 = _S48;
    }
    else
    {
        _S50 = _S48;
        _S51 = 0.0f;
    }
    if(_S49)
    {
        _S50 = _S50 + _S51;
    }
    else
    {
        _S50 = _S51;
    }
    (*_S46) = _S50;
    (*_S47) = 0.0f;
    return;
}

fn s_bwdProp_level_0( _S52 : s_bwdCallableCtx_level_0,  _S53 : ptr<function, f32>,  _S54 : ptr<function, f32>,  _S55 : ptr<function, f32>,  _S56 : ptr<function, f32>,  _S57 : f32)
{
    var _S58 : f32 = _S52.value0_16.value0_14 - _S52.value0_16.value2_3;
    var _S59 : f32 = _S58 * _S52.value0_16.value3_2;
    var _S60 : Tuple_2 = s_apply_min_0(_S59, _S52.value0_16.value5_1);
    var _S61 : s_bwdCallableCtx_min_0;
    _S61._S15 = _S59;
    _S61._S16 = _S52.value0_16.value5_1;
    var _S62 : Tuple_0 = s_apply_max_0(_S60.value0_2, 0.0f);
    var _S63 : s_bwdCallableCtx_max_0;
    _S63._S2 = _S60.value0_2;
    _S63._S3 = 0.0f;
    var _S64 : f32 = _S52.value0_16.value4_1 - 1.0f;
    var _S65 : Tuple_1 = s_apply_pow0_0(_S60.value0_2, _S64);
    var _S66 : s_bwdCallableCtx_pow0_0;
    _S66._S10 = _S60.value0_2;
    _S66._S11 = _S64;
    var _S67 : s_paramCtx_s_bwdCallableCtx_with_table_value_0;
    _S67.value0_17 = _S52.value0_16.value1_4;
    var _S68 : s_bwdCallableCtx_with_table_value_0 = s_bwdCallableCtx_with_table_value_0( _S67 );
    var _S69 : f32 = 0.0f;
    var _S70 : f32 = 0.0f;
    s_bwdProp_with_table_value_0(_S68, &(_S69), &(_S70), _S57);
    var _S71 : f32 = _S62.value0_0 * _S69;
    var _S72 : f32 = _S65.value0_1 * _S69;
    var _S73 : f32 = 0.0f;
    var _S74 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S66, &(_S73), &(_S74), _S71);
    var _S75 : f32 = 0.0f;
    var _S76 : f32 = 0.0f;
    s_bwdProp_max_0(_S63, &(_S75), &(_S76), _S72);
    var _S77 : f32 = _S73 + _S75;
    var _S78 : f32 = 0.0f;
    var _S79 : f32 = 0.0f;
    s_bwdProp_min_0(_S61, &(_S78), &(_S79), _S77);
    var _S80 : f32 = _S58 * _S78;
    var _S81 : f32 = _S70 + _S74;
    (*_S53) = - (_S52.value0_16.value3_2 * _S78);
    (*_S54) = _S80;
    (*_S55) = _S81;
    (*_S56) = _S79;
    return;
}

fn Shape_x24_syn_dadd_0( SLANG_anonymous_0_0 : Shape_0,  SLANG_anonymous_1_0 : Shape_0) -> Shape_0
{
    var result_2 : Shape_0;
    result_2.black_2 = SLANG_anonymous_0_0.black_2 + SLANG_anonymous_1_0.black_2;
    result_2.scale_1 = SLANG_anonymous_0_0.scale_1 + SLANG_anonymous_1_0.scale_1;
    result_2.power_0 = SLANG_anonymous_0_0.power_0 + SLANG_anonymous_1_0.power_0;
    result_2.limit_0 = SLANG_anonymous_0_0.limit_0 + SLANG_anonymous_1_0.limit_0;
    return result_2;
}

fn s_bwdProp_ratio_0( _S82 : s_bwdCallableCtx_ratio_0,  _S83 : ptr<function, f32>,  _S84 : ptr<function, f32>,  _S85 : ptr<function, f32>,  _S86 : ptr<function, f32>,  _S87 : f32)
{
    var _S88 : f32 = _S82.value0_15.value0_13 - _S82.value0_15.value2_2;
    var _S89 : f32 = _S88 * _S82.value0_15.value3_1;
    var _S90 : Tuple_2 = s_apply_min_0(_S89, _S82.value0_15.value5_0);
    var _S91 : s_bwdCallableCtx_min_0;
    _S91._S15 = _S89;
    _S91._S16 = _S82.value0_15.value5_0;
    var _S92 : s_bwdCallableCtx_min_0 = _S91;
    var _S93 : f32 = 1.0f - _S82.value0_15.value2_2 / _S82.value0_15.value0_13;
    var _S94 : f32 = _S82.value0_15.value3_1 * _S93;
    var _S95 : f32 = _S82.value0_15.value4_0 - 1.0f;
    var _S96 : Tuple_1 = s_apply_pow0_0(_S90.value0_2, _S95);
    var _S97 : s_bwdCallableCtx_pow0_0;
    _S97._S10 = _S90.value0_2;
    _S97._S11 = _S95;
    var _S98 : s_bwdCallableCtx_pow0_0 = _S97;
    var _S99 : bool = ((_S82.value0_15.value1_3) < 0.0f);
    var _S100 : f32;
    var _S101 : f32;
    if(!_S99)
    {
        _S100 = 0.0f;
        _S101 = _S87;
    }
    else
    {
        _S100 = _S87;
        _S101 = 0.0f;
    }
    if(_S99)
    {
        _S100 = _S100 + _S101;
    }
    else
    {
        _S100 = _S101;
    }
    var _S102 : f32 = _S94 * _S100;
    var _S103 : f32 = _S96.value0_1 * _S100;
    var _S104 : f32 = 0.0f;
    var _S105 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S98, &(_S104), &(_S105), _S102);
    var _S106 : f32 = _S93 * _S103;
    var _S107 : f32 = - (_S82.value0_15.value3_1 * _S103) / _S82.value0_15.value0_13;
    var _S108 : f32 = 0.0f;
    var _S109 : f32 = 0.0f;
    s_bwdProp_min_0(_S92, &(_S108), &(_S109), _S104);
    var _S110 : f32 = _S106 + _S88 * _S108;
    (*_S83) = _S107 + - (_S82.value0_15.value3_1 * _S108);
    (*_S84) = _S110;
    (*_S85) = _S105;
    (*_S86) = _S109;
    return;
}

fn s_bwdProp_rgb_levels_0( _S111 : s_bwdCallableCtx_rgb_levels_0,  _S112 : ptr<function, Shape_0>,  _S113 : vec3<f32>)
{
    var _S114 : s_paramCtx_s_bwdCallableCtx_ratio_0 = s_paramCtx_s_bwdCallableCtx_ratio_0( 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f );
    var _S115 : s_paramCtx_s_bwdCallableCtx_level_0 = s_paramCtx_s_bwdCallableCtx_level_0( 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f );
    var _S116 : bool = !((_S111.value0_12.value1_2.mode_0) == u32(2));
    var _runFlag_0 : bool;
    var _S117 : bool;
    var _S118 : s_bwdCallableCtx_ratio_0;
    var _S119 : s_bwdCallableCtx_level_0;
    var _S120 : s_bwdCallableCtx_level_0;
    var _S121 : s_bwdCallableCtx_level_0;
    if(_S116)
    {
        var _S122 : bool = (_S111.value0_12.value1_2.mode_0) == u32(1);
        if(_S122)
        {
            var _S123 : f32 = _S111.value0_12.value1_2.step_0.x;
            var _S124 : f32 = _S111.value0_12.value2_1.black_2.x;
            var _S125 : f32 = _S111.value0_12.value2_1.scale_1.x;
            var _S126 : f32 = _S111.value0_12.value2_1.power_0.x;
            var _S127 : f32 = _S111.value0_12.value2_1.limit_0.x;
            var _S128 : s_paramCtx_s_bwdCallableCtx_ratio_0;
            _S128.value0_13 = _S111.value0_12.value1_2.norm_0;
            _S128.value1_3 = _S123;
            _S128.value2_2 = _S124;
            _S128.value3_1 = _S125;
            _S128.value4_0 = _S126;
            _S128.value5_0 = _S127;
            _runFlag_0 = false;
            _S118.value0_15 = _S128;
        }
        else
        {
            _runFlag_0 = _S116;
            _S118.value0_15 = _S114;
        }
        if(_runFlag_0)
        {
            var _S129 : f32 = _S111.value0_12.value1_2.step_0.x;
            var _S130 : f32 = _S111.value0_12.value2_1.black_2.x;
            var _S131 : f32 = _S111.value0_12.value2_1.scale_1.x;
            var _S132 : f32 = _S111.value0_12.value2_1.power_0.x;
            var _S133 : f32 = _S111.value0_12.value2_1.limit_0.x;
            var _S134 : s_paramCtx_s_bwdCallableCtx_level_0;
            _S134.value0_14 = _S111.value0_12.value0_11.x;
            _S134.value1_4 = _S129;
            _S134.value2_3 = _S130;
            _S134.value3_2 = _S131;
            _S134.value4_1 = _S132;
            _S134.value5_1 = _S133;
            var _S135 : f32 = _S111.value0_12.value1_2.step_0.y;
            var _S136 : f32 = _S111.value0_12.value2_1.black_2.y;
            var _S137 : f32 = _S111.value0_12.value2_1.scale_1.y;
            var _S138 : f32 = _S111.value0_12.value2_1.power_0.y;
            var _S139 : f32 = _S111.value0_12.value2_1.limit_0.y;
            var _S140 : s_paramCtx_s_bwdCallableCtx_level_0;
            _S140.value0_14 = _S111.value0_12.value0_11.y;
            _S140.value1_4 = _S135;
            _S140.value2_3 = _S136;
            _S140.value3_2 = _S137;
            _S140.value4_1 = _S138;
            _S140.value5_1 = _S139;
            var _S141 : f32 = _S111.value0_12.value1_2.step_0.z;
            var _S142 : f32 = _S111.value0_12.value2_1.black_2.z;
            var _S143 : f32 = _S111.value0_12.value2_1.scale_1.z;
            var _S144 : f32 = _S111.value0_12.value2_1.power_0.z;
            var _S145 : f32 = _S111.value0_12.value2_1.limit_0.z;
            var _S146 : s_paramCtx_s_bwdCallableCtx_level_0;
            _S146.value0_14 = _S111.value0_12.value0_11.z;
            _S146.value1_4 = _S141;
            _S146.value2_3 = _S142;
            _S146.value3_2 = _S143;
            _S146.value4_1 = _S144;
            _S146.value5_1 = _S145;
            _S119.value0_16 = _S146;
            _S120.value0_16 = _S140;
            _S121.value0_16 = _S134;
        }
        else
        {
            _S119.value0_16 = _S115;
            _S120.value0_16 = _S115;
            _S121.value0_16 = _S115;
        }
        _S117 = _S122;
    }
    else
    {
        _runFlag_0 = false;
        _S119.value0_16 = _S115;
        _S120.value0_16 = _S115;
        _S121.value0_16 = _S115;
        _S117 = false;
        _S118.value0_15 = _S114;
    }
    var _S147 : Shape_0 = Shape_x24_syn_dzero_0();
    var _S148 : vec3<f32> = vec3<f32>(0.0f);
    var _S149 : Shape_0;
    if(_S116)
    {
        var _S150 : vec3<f32>;
        if(_runFlag_0)
        {
            var _S151 : f32 = 0.0f;
            var _S152 : f32 = 0.0f;
            var _S153 : f32 = 0.0f;
            var _S154 : f32 = 0.0f;
            s_bwdProp_level_0(_S119, &(_S151), &(_S152), &(_S153), &(_S154), _S113[i32(2)]);
            var _S155 : f32 = 0.0f;
            var _S156 : f32 = 0.0f;
            var _S157 : f32 = 0.0f;
            var _S158 : f32 = 0.0f;
            s_bwdProp_level_0(_S120, &(_S155), &(_S156), &(_S157), &(_S158), _S113[i32(1)]);
            var _S159 : f32 = 0.0f;
            var _S160 : f32 = 0.0f;
            var _S161 : f32 = 0.0f;
            var _S162 : f32 = 0.0f;
            s_bwdProp_level_0(_S121, &(_S159), &(_S160), &(_S161), &(_S162), _S113[i32(0)]);
            var _S163 : vec3<f32> = vec3<f32>(_S162, _S158, _S154);
            var _S164 : vec3<f32> = vec3<f32>(_S161, _S157, _S153);
            var _S165 : vec3<f32> = vec3<f32>(_S160, _S156, _S152);
            var _S166 : vec3<f32> = vec3<f32>(_S159, _S155, _S151);
            var _S167 : Shape_0 = _S147;
            _S167.limit_0 = _S163;
            _S167.power_0 = _S164;
            _S167.scale_1 = _S165;
            _S167.black_2 = _S166;
            var _S168 : Shape_0 = Shape_x24_syn_dadd_0(_S147, _S167);
            _S150 = _S148;
            _S149 = _S168;
        }
        else
        {
            _S150 = _S113;
            _S149 = _S147;
        }
        if(_S117)
        {
            var _S169 : vec3<f32> = _S111.value0_12.value0_11 * _S150;
            var _S170 : f32 = _S169[i32(0)] + _S169[i32(1)] + _S169[i32(2)];
            var _S171 : f32 = 0.0f;
            var _S172 : f32 = 0.0f;
            var _S173 : f32 = 0.0f;
            var _S174 : f32 = 0.0f;
            s_bwdProp_ratio_0(_S118, &(_S171), &(_S172), &(_S173), &(_S174), _S170);
            var _S175 : vec3<f32> = vec3<f32>(_S174, 0.0f, 0.0f);
            var _S176 : vec3<f32> = vec3<f32>(_S173, 0.0f, 0.0f);
            var _S177 : vec3<f32> = vec3<f32>(_S172, 0.0f, 0.0f);
            var _S178 : vec3<f32> = vec3<f32>(_S171, 0.0f, 0.0f);
            var _S179 : Shape_0 = _S147;
            _S179.limit_0 = _S175;
            _S179.power_0 = _S176;
            _S179.scale_1 = _S177;
            _S179.black_2 = _S178;
            _S149 = Shape_x24_syn_dadd_0(_S149, _S179);
        }
    }
    else
    {
        _S149 = _S147;
    }
    (*_S112) = _S149;
    return;
}

fn Levels_x24_syn_dadd_0( SLANG_anonymous_0_1 : Levels_0,  SLANG_anonymous_1_1 : Levels_0) -> Levels_0
{
    var result_3 : Levels_0;
    result_3.black_1 = SLANG_anonymous_0_1.black_1 + SLANG_anonymous_1_1.black_1;
    result_3.grey_1 = SLANG_anonymous_0_1.grey_1 + SLANG_anonymous_1_1.grey_1;
    result_3.white_1 = SLANG_anonymous_0_1.white_1 + SLANG_anonymous_1_1.white_1;
    return result_3;
}

fn s_bwdProp_shape_0( _S180 : s_bwdCallableCtx_shape_0,  _S181 : ptr<function, Levels_0>,  _S182 : Shape_0)
{
    var white_3 : vec3<f32>;
    var black_4 : vec3<f32>;
    var grey_3 : vec3<f32>;
    if(_S180.value0_10.value1_1)
    {
        var _S183 : vec3<f32> = _S180.value0_10.value0_9.black_1.xxx;
        var _S184 : vec3<f32> = _S180.value0_10.value0_9.grey_1.xxx;
        white_3 = _S180.value0_10.value0_9.white_1.xxx;
        black_4 = _S183;
        grey_3 = _S184;
    }
    else
    {
        white_3 = _S180.value0_10.value0_9.white_1;
        black_4 = _S180.value0_10.value0_9.black_1;
        grey_3 = _S180.value0_10.value0_9.grey_1;
    }
    var _S185 : vec3<f32> = white_3 - black_4;
    var delta_1 : vec3<f32> = _S185 / vec3<f32>(2.0f);
    var _S186 : vec3<f32> = grey_3 - (black_4 + delta_1);
    var _S187 : vec3<f32> = delta_1 * delta_1;
    var _S188 : vec3<f32> = _S185 * _S185;
    var _S189 : vec3<f32> = vec3<f32>(2.3025851249694824f);
    var _S190 : vec3<f32> = _S186 / delta_1 * _S189;
    var _S191 : Tuple_5 = s_apply_exp_0(_S190);
    var _S192 : s_bwdCallableCtx_exp_0;
    _S192._S32 = _S190;
    var _S193 : s_bwdCallableCtx_exp_0 = _S192;
    var _S194 : vec3<f32> = vec3<f32>(1.0f);
    var _S195 : Tuple_4 = s_apply_max_1(_S191.value0_5, _S194);
    var _S196 : s_bwdCallableCtx_max_1;
    _S196._S27 = _S191.value0_5;
    _S196._S28 = _S194;
    var _S197 : s_bwdCallableCtx_max_1 = _S196;
    var _S198 : vec3<f32> = vec3<f32>(64.0f);
    var _S199 : vec3<f32> = _S195.value0_4 * _S195.value0_4;
    var _S200 : s_bwdCallableCtx_exp2_0;
    _S200._S23 = _S198 / _S195.value0_4;
    var _S201 : s_bwdCallableCtx_exp2_0 = _S200;
    var _S202 : Levels_0 = Levels_x24_syn_dzero_0();
    var _S203 : vec3<f32> = vec3<f32>(0.0f);
    var _S204 : vec3<f32> = _S203;
    s_bwdProp_exp2_0(_S201, &(_S204), _S182.limit_0);
    var _S205 : vec3<f32> = _S198 * (vec3<f32>(0) - _S204 / _S199);
    var _S206 : vec3<f32> = _S203;
    var _S207 : vec3<f32> = _S203;
    s_bwdProp_max_1(_S197, &(_S206), &(_S207), _S205);
    var _S208 : vec3<f32> = _S182.power_0 + _S206;
    var _S209 : vec3<f32> = _S203;
    s_bwdProp_exp_0(_S193, &(_S209), _S208);
    var _S210 : vec3<f32> = _S189 * _S209 / _S187;
    var _S211 : vec3<f32> = delta_1 * _S210;
    var s_diff_mid_T_0 : vec3<f32> = (vec3<f32>(0) - _S211);
    var _S212 : vec3<f32> = (vec3<f32>(0) - _S182.scale_1 / _S188) + vec3<f32>(0.5f) * (_S186 * (vec3<f32>(0) - _S210) + s_diff_mid_T_0);
    var _S213 : vec3<f32> = _S182.black_2 + s_diff_mid_T_0 + (vec3<f32>(0) - _S212);
    if(_S180.value0_10.value1_1)
    {
        var _S214 : vec3<f32> = vec3<f32>(_S211[i32(0)] + _S211[i32(1)] + _S211[i32(2)], 0.0f, 0.0f);
        var _S215 : vec3<f32> = vec3<f32>(_S213[i32(0)] + _S213[i32(1)] + _S213[i32(2)], 0.0f, 0.0f);
        white_3 = vec3<f32>(_S212[i32(0)] + _S212[i32(1)] + _S212[i32(2)], 0.0f, 0.0f);
        black_4 = _S214;
        grey_3 = _S215;
    }
    else
    {
        white_3 = _S212;
        black_4 = _S211;
        grey_3 = _S213;
    }
    var _S216 : Levels_0 = _S202;
    _S216.white_1 = white_3;
    _S216.grey_1 = black_4;
    _S216.black_1 = grey_3;
    (*_S181) = Levels_x24_syn_dadd_0(_S202, _S216);
    return;
}

fn s_bwdProp_rgb_levels_from_levels_0( _S217 : s_bwdCallableCtx_rgb_levels_from_levels_0,  _S218 : ptr<function, Levels_0>,  _S219 : vec3<f32>)
{
    var _S220 : Tuple_6 = s_apply_shape_0(_S217.value0_7.value3_0, _S217.value0_7.value2_0);
    var _S221 : s_paramCtx_s_bwdCallableCtx_shape_0;
    _S221.value0_9 = _S217.value0_7.value3_0;
    _S221.value1_1 = _S217.value0_7.value2_0;
    var _S222 : s_bwdCallableCtx_shape_0 = s_bwdCallableCtx_shape_0( _S221 );
    var _S223 : s_paramCtx_s_bwdCallableCtx_rgb_levels_0;
    _S223.value0_11 = _S217.value0_7.value0_6;
    _S223.value1_2 = _S217.value0_7.value1_0;
    _S223.value2_1 = _S220.value0_8;
    var _S224 : s_bwdCallableCtx_rgb_levels_0 = s_bwdCallableCtx_rgb_levels_0( _S223 );
    var _S225 : Levels_0 = Levels_x24_syn_dzero_0();
    var _S226 : vec3<f32> = vec3<f32>(0.0f);
    var _S227 : Shape_0;
    _S227.black_2 = _S226;
    _S227.scale_1 = _S226;
    _S227.power_0 = _S226;
    _S227.limit_0 = _S226;
    s_bwdProp_rgb_levels_0(_S224, &(_S227), _S219);
    var _S228 : Levels_0;
    _S228.black_1 = _S226;
    _S228.grey_1 = _S226;
    _S228.white_1 = _S226;
    s_bwdProp_shape_0(_S222, &(_S228), _S227);
    (*_S218) = Levels_x24_syn_dadd_0(_S228, _S225);
    return;
}

fn _S229( _S230 : vec3<f32>,  _S231 : Lookup_0,  _S232 : bool,  _S233 : ptr<function, DiffPair_Levels_0>,  _S234 : vec3<f32>)
{
    var _S235 : s_paramCtx_s_bwdCallableCtx_rgb_levels_from_levels_0;
    _S235.value0_6 = _S230;
    _S235.value1_0 = _S231;
    _S235.value2_0 = _S232;
    _S235.value3_0 = (*_S233).primal_0;
    var _S236 : s_bwdCallableCtx_rgb_levels_from_levels_0 = s_bwdCallableCtx_rgb_levels_from_levels_0( _S235 );
    var _S237 : Levels_0 = (*_S233).differential_0;
    s_bwdProp_rgb_levels_from_levels_0(_S236, &(_S237), _S234);
    (*_S233).differential_0 = _S237;
    return;
}

fn _d_min_1( dpx_5 : DiffPair_float_0,  dpy_3 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S238 : f32 = min(dpx_5.primal_0, dpy_3.primal_0);
    var _S239 : f32;
    if(((dpx_5.primal_0) < (dpy_3.primal_0)))
    {
        _S239 = dpx_5.differential_0;
    }
    else
    {
        if(((dpx_5.primal_0) > (dpy_3.primal_0)))
        {
            _S239 = dpy_3.differential_0;
        }
        else
        {
            _S239 = 0.5f * (dpx_5.differential_0 + dpy_3.differential_0);
        }
    }
    var _S240 : DiffPair_float_0 = DiffPair_float_0( _S238, _S239 );
    return _S240;
}

fn _d_max_1( dpx_6 : DiffPair_float_0,  dpy_4 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S241 : f32 = max(dpx_6.primal_0, dpy_4.primal_0);
    var _S242 : f32;
    if(((dpx_6.primal_0) > (dpy_4.primal_0)))
    {
        _S242 = dpx_6.differential_0;
    }
    else
    {
        if(((dpx_6.primal_0) < (dpy_4.primal_0)))
        {
            _S242 = dpy_4.differential_0;
        }
        else
        {
            _S242 = 0.5f * (dpx_6.differential_0 + dpy_4.differential_0);
        }
    }
    var _S243 : DiffPair_float_0 = DiffPair_float_0( _S241, _S242 );
    return _S243;
}

fn _d_exp2_vector_1( dpx_7 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S244 : vec3<f32> = exp2(dpx_7.primal_0);
    var _S245 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S244, _S244 * vec3<f32>(0.69314718246459961f) * dpx_7.differential_0 );
    return _S245;
}

fn _d_max_vector_1( dpx_8 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpy_5 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S246 : DiffPair_float_0 = DiffPair_float_0( dpx_8.primal_0[i32(0)], dpx_8.differential_0[i32(0)] );
    var _S247 : DiffPair_float_0 = DiffPair_float_0( dpy_5.primal_0[i32(0)], dpy_5.differential_0[i32(0)] );
    var dp_elem_0 : DiffPair_float_0 = _d_max_1(_S246, _S247);
    var result_4 : vec3<f32>;
    result_4[i32(0)] = dp_elem_0.primal_0;
    var d_result_0 : vec3<f32>;
    d_result_0[i32(0)] = dp_elem_0.differential_0;
    var _S248 : DiffPair_float_0 = DiffPair_float_0( dpx_8.primal_0[i32(1)], dpx_8.differential_0[i32(1)] );
    var _S249 : DiffPair_float_0 = DiffPair_float_0( dpy_5.primal_0[i32(1)], dpy_5.differential_0[i32(1)] );
    var dp_elem_1 : DiffPair_float_0 = _d_max_1(_S248, _S249);
    result_4[i32(1)] = dp_elem_1.primal_0;
    d_result_0[i32(1)] = dp_elem_1.differential_0;
    var _S250 : DiffPair_float_0 = DiffPair_float_0( dpx_8.primal_0[i32(2)], dpx_8.differential_0[i32(2)] );
    var _S251 : DiffPair_float_0 = DiffPair_float_0( dpy_5.primal_0[i32(2)], dpy_5.differential_0[i32(2)] );
    var dp_elem_2 : DiffPair_float_0 = _d_max_1(_S250, _S251);
    result_4[i32(2)] = dp_elem_2.primal_0;
    d_result_0[i32(2)] = dp_elem_2.differential_0;
    var _S252 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( result_4, d_result_0 );
    return _S252;
}

fn _d_exp_vector_1( dpx_9 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S253 : vec3<f32> = exp(dpx_9.primal_0);
    var _S254 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S253, _S253 * dpx_9.differential_0 );
    return _S254;
}

fn levels_of_0( p_0 : ptr<function, RgbLevelsParams_std140_0>) -> Levels_0
{
    var l_0 : Levels_0;
    l_0.black_1 = (*p_0).black_0.xyz;
    l_0.grey_1 = (*p_0).grey_0.xyz;
    l_0.white_1 = (*p_0).white_0.xyz;
    return l_0;
}

fn shape_0( l_1 : Levels_0,  linked_1 : bool) -> Shape_0
{
    var white_4 : vec3<f32>;
    var black_5 : vec3<f32>;
    var grey_4 : vec3<f32>;
    if(linked_1)
    {
        var _S255 : vec3<f32> = l_1.black_1.xxx;
        var _S256 : vec3<f32> = l_1.grey_1.xxx;
        white_4 = l_1.white_1.xxx;
        black_5 = _S255;
        grey_4 = _S256;
    }
    else
    {
        white_4 = l_1.white_1;
        black_5 = l_1.black_1;
        grey_4 = l_1.grey_1;
    }
    var _S257 : vec3<f32> = white_4 - black_5;
    var delta_2 : vec3<f32> = _S257 / vec3<f32>(2.0f);
    var offset_0 : vec3<f32> = (grey_4 - (black_5 + delta_2)) / delta_2;
    var s_0 : Shape_0;
    s_0.black_2 = black_5;
    s_0.scale_1 = vec3<f32>(1.0f) / _S257;
    var _S258 : vec3<f32> = exp(offset_0 * vec3<f32>(2.3025851249694824f));
    s_0.power_0 = _S258;
    s_0.limit_0 = exp2(vec3<f32>(64.0f) / max(_S258, vec3<f32>(1.0f)));
    return s_0;
}

struct DiffPair_Shape_0
{
     primal_0 : Shape_0,
     differential_0 : Shape_0,
};

fn s_fwd_shape_0( dpl_0 : DiffPair_Levels_0,  linked_2 : bool) -> DiffPair_Shape_0
{
    var white_5 : vec3<f32>;
    var s_diff_white_0 : vec3<f32>;
    var black_6 : vec3<f32>;
    var s_diff_black_0 : vec3<f32>;
    var grey_5 : vec3<f32>;
    var s_diff_grey_0 : vec3<f32>;
    if(linked_2)
    {
        var _S259 : vec3<f32> = dpl_0.primal_0.black_1.xxx;
        var _S260 : vec3<f32> = dpl_0.differential_0.black_1.xxx;
        var _S261 : vec3<f32> = dpl_0.primal_0.grey_1.xxx;
        var _S262 : vec3<f32> = dpl_0.differential_0.grey_1.xxx;
        var _S263 : vec3<f32> = dpl_0.differential_0.white_1.xxx;
        white_5 = dpl_0.primal_0.white_1.xxx;
        s_diff_white_0 = _S263;
        black_6 = _S259;
        s_diff_black_0 = _S260;
        grey_5 = _S261;
        s_diff_grey_0 = _S262;
    }
    else
    {
        white_5 = dpl_0.primal_0.white_1;
        s_diff_white_0 = dpl_0.differential_0.white_1;
        black_6 = dpl_0.primal_0.black_1;
        s_diff_black_0 = dpl_0.differential_0.black_1;
        grey_5 = dpl_0.primal_0.grey_1;
        s_diff_grey_0 = dpl_0.differential_0.grey_1;
    }
    var _S264 : vec3<f32> = white_5 - black_6;
    var _S265 : vec3<f32> = s_diff_white_0 - s_diff_black_0;
    var delta_3 : vec3<f32> = _S264 / vec3<f32>(2.0f);
    var s_diff_delta_0 : vec3<f32> = _S265 * vec3<f32>(0.5f);
    var _S266 : vec3<f32> = grey_5 - (black_6 + delta_3);
    var _S267 : vec3<f32> = vec3<f32>(0.0f);
    var _S268 : vec3<f32> = vec3<f32>(2.3025851249694824f);
    var _S269 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S266 / delta_3 * _S268, ((s_diff_grey_0 - (s_diff_black_0 + s_diff_delta_0)) * delta_3 - _S266 * s_diff_delta_0) / (delta_3 * delta_3) * _S268 );
    var _S270 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_exp_vector_1(_S269);
    var _S271 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S270.primal_0, _S270.differential_0 );
    var _S272 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(1.0f), _S267 );
    var _S273 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_max_vector_1(_S271, _S272);
    var _S274 : vec3<f32> = vec3<f32>(64.0f);
    var _S275 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S274 / _S273.primal_0, (_S267 - _S274 * _S273.differential_0) / (_S273.primal_0 * _S273.primal_0) );
    var _S276 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_exp2_vector_1(_S275);
    var _S277 : Shape_0 = Shape_0( black_6, vec3<f32>(1.0f) / _S264, _S270.primal_0, _S276.primal_0 );
    var _S278 : Shape_0 = Shape_0( s_diff_black_0, (_S267 - _S265) / (_S264 * _S264), _S270.differential_0, _S276.differential_0 );
    var _S279 : DiffPair_Shape_0 = DiffPair_Shape_0( _S277, _S278 );
    return _S279;
}

fn table_step_0( t_0 : f32) -> f32
{
    var _S280 : bool;
    if((t_0 > 0.0f))
    {
        _S280 = ((t_0 * 6.5536e+04f) < 4.0f);
    }
    else
    {
        _S280 = false;
    }
    var _S281 : f32;
    if(_S280)
    {
        _S281 = floor(t_0 * 6.5536e+04f);
    }
    else
    {
        _S281 = -1.0f;
    }
    return _S281;
}

fn rgb_norm_0( rgb_0 : vec3<f32>,  norm_1 : u32,  luminance_1 : vec3<f32>) -> f32
{
    if(norm_1 == u32(1))
    {
        return dot(luminance_1, rgb_0);
    }
    if(norm_1 == u32(2))
    {
        return max(rgb_0.x, max(rgb_0.y, rgb_0.z));
    }
    if(norm_1 == u32(4))
    {
        return rgb_0.x + rgb_0.y + rgb_0.z;
    }
    if(norm_1 == u32(5))
    {
        return sqrt(dot(rgb_0, rgb_0));
    }
    if(norm_1 == u32(6))
    {
        var squared_0 : vec3<f32> = rgb_0 * rgb_0;
        var sum_0 : f32 = squared_0.x + squared_0.y + squared_0.z;
        var _S282 : f32;
        if((sum_0 > 0.0f))
        {
            _S282 = dot(rgb_0, squared_0) / sum_0;
        }
        else
        {
            _S282 = 0.0f;
        }
        return _S282;
    }
    return (rgb_0.x + rgb_0.y + rgb_0.z) / 3.0f;
}

fn locate_0( rgb_1 : vec3<f32>,  s_1 : Shape_0,  p_1 : ptr<function, RgbLevelsParams_std140_0>) -> Lookup_0
{
    var l_2 : Lookup_0;
    l_2.mode_0 = u32(0);
    l_2.norm_0 = 0.0f;
    var _S283 : f32 = s_1.black_2.x;
    var _S284 : f32 = s_1.scale_1.x;
    l_2.step_0 = vec3<f32>(table_step_0((rgb_1.x - _S283) * _S284), table_step_0((rgb_1.y - s_1.black_2.y) * s_1.scale_1.y), table_step_0((rgb_1.z - s_1.black_2.z) * s_1.scale_1.z));
    var _S285 : bool;
    if(((*p_1).autoscale_0) != u32(1))
    {
        _S285 = ((*p_1).preserve_colors_0) != u32(0);
    }
    else
    {
        _S285 = false;
    }
    if(_S285)
    {
        var _S286 : f32 = rgb_norm_0(rgb_1, (*p_1).preserve_colors_0, (*p_1).luminance_0.xyz);
        l_2.norm_0 = _S286;
        var _S287 : u32;
        if((_S286 > _S283))
        {
            _S287 = u32(1);
        }
        else
        {
            _S287 = u32(2);
        }
        l_2.mode_0 = _S287;
        l_2.step_0[i32(0)] = table_step_0((l_2.norm_0 - _S283) * _S284);
    }
    return l_2;
}

fn pow0_forward_0( x_8 : DiffPair_float_0,  y_6 : DiffPair_float_0) -> DiffPair_float_0
{
    if(((x_8.primal_0) <= 0.0f))
    {
        var _S288 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
        return _S288;
    }
    var value_0 : f32 = pow(x_8.primal_0, y_6.primal_0);
    var _S289 : DiffPair_float_0 = DiffPair_float_0( value_0, x_8.differential_0 * y_6.primal_0 * pow(x_8.primal_0, y_6.primal_0 - 1.0f) + y_6.differential_0 * value_0 * log(x_8.primal_0) );
    return _S289;
}

fn table_entry_0( step_1 : f32,  power_1 : f32) -> f32
{
    var q_0 : f32 = step_1 / 6.5536e+04f;
    return q_0 * pow0_0(q_0, power_1 - 1.0f);
}

fn ratio_0( norm_2 : f32,  step_2 : f32,  black_7 : f32,  scale_2 : f32,  power_2 : f32,  limit_1 : f32) -> f32
{
    var r_0 : f32 = scale_2 * (1.0f - black_7 / norm_2) * pow0_0(min((norm_2 - black_7) * scale_2, limit_1), power_2 - 1.0f);
    if((step_2 < 0.0f))
    {
        return r_0;
    }
    return r_0 + (table_entry_0(step_2, power_2) / norm_2 - r_0);
}

fn s_fwd_ratio_0( norm_3 : f32,  step_3 : f32,  dpblack_0 : DiffPair_float_0,  dpscale_0 : DiffPair_float_0,  dppower_0 : DiffPair_float_0,  dplimit_0 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S290 : f32 = norm_3 - dpblack_0.primal_0;
    var _S291 : DiffPair_float_0 = DiffPair_float_0( _S290 * dpscale_0.primal_0, (0.0f - dpblack_0.differential_0) * dpscale_0.primal_0 + dpscale_0.differential_0 * _S290 );
    var _S292 : DiffPair_float_0 = DiffPair_float_0( dplimit_0.primal_0, dplimit_0.differential_0 );
    var _S293 : DiffPair_float_0 = _d_min_1(_S291, _S292);
    var _S294 : f32 = 1.0f - dpblack_0.primal_0 / norm_3;
    var _S295 : f32 = dpscale_0.primal_0 * _S294;
    var _S296 : DiffPair_float_0 = DiffPair_float_0( _S293.primal_0, _S293.differential_0 );
    var _S297 : DiffPair_float_0 = DiffPair_float_0( dppower_0.primal_0 - 1.0f, dppower_0.differential_0 );
    var _S298 : DiffPair_float_0 = pow0_forward_0(_S296, _S297);
    var r_1 : f32 = _S295 * _S298.primal_0;
    var s_diff_r_0 : f32 = (dpscale_0.differential_0 * _S294 + (0.0f - dpblack_0.differential_0 / norm_3) * dpscale_0.primal_0) * _S298.primal_0 + _S298.differential_0 * _S295;
    if((step_3 < 0.0f))
    {
        var _S299 : DiffPair_float_0 = DiffPair_float_0( r_1, s_diff_r_0 );
        return _S299;
    }
    var _S300 : DiffPair_float_0 = DiffPair_float_0( r_1 + (table_entry_0(step_3, dppower_0.primal_0) / norm_3 - r_1), s_diff_r_0 );
    return _S300;
}

fn with_table_value_0( curve_0 : f32,  step_4 : f32,  power_3 : f32) -> f32
{
    if((step_4 < 0.0f))
    {
        return curve_0;
    }
    return curve_0 + (table_entry_0(step_4, power_3) - curve_0);
}

fn s_fwd_with_table_value_0( dpcurve_0 : DiffPair_float_0,  step_5 : f32,  dppower_1 : DiffPair_float_0) -> DiffPair_float_0
{
    if((step_5 < 0.0f))
    {
        var _S301 : DiffPair_float_0 = DiffPair_float_0( dpcurve_0.primal_0, dpcurve_0.differential_0 );
        return _S301;
    }
    var _S302 : DiffPair_float_0 = DiffPair_float_0( dpcurve_0.primal_0 + (table_entry_0(step_5, dppower_1.primal_0) - dpcurve_0.primal_0), dpcurve_0.differential_0 );
    return _S302;
}

fn level_0( v_0 : f32,  step_6 : f32,  black_8 : f32,  scale_3 : f32,  power_4 : f32,  limit_2 : f32) -> f32
{
    var _S303 : f32 = min((v_0 - black_8) * scale_3, limit_2);
    return with_table_value_0(max(_S303, 0.0f) * pow0_0(_S303, power_4 - 1.0f), step_6, power_4);
}

fn s_fwd_level_0( v_1 : f32,  step_7 : f32,  dpblack_1 : DiffPair_float_0,  dpscale_1 : DiffPair_float_0,  dppower_2 : DiffPair_float_0,  dplimit_1 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S304 : f32 = v_1 - dpblack_1.primal_0;
    var _S305 : DiffPair_float_0 = DiffPair_float_0( _S304 * dpscale_1.primal_0, (0.0f - dpblack_1.differential_0) * dpscale_1.primal_0 + dpscale_1.differential_0 * _S304 );
    var _S306 : DiffPair_float_0 = DiffPair_float_0( dplimit_1.primal_0, dplimit_1.differential_0 );
    var _S307 : DiffPair_float_0 = _d_min_1(_S305, _S306);
    var _S308 : DiffPair_float_0 = DiffPair_float_0( _S307.primal_0, _S307.differential_0 );
    var _S309 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S310 : DiffPair_float_0 = _d_max_1(_S308, _S309);
    var _S311 : DiffPair_float_0 = DiffPair_float_0( dppower_2.primal_0 - 1.0f, dppower_2.differential_0 );
    var _S312 : DiffPair_float_0 = pow0_forward_0(_S308, _S311);
    var _S313 : DiffPair_float_0 = DiffPair_float_0( _S310.primal_0 * _S312.primal_0, _S310.differential_0 * _S312.primal_0 + _S312.differential_0 * _S310.primal_0 );
    var _S314 : DiffPair_float_0 = DiffPair_float_0( dppower_2.primal_0, dppower_2.differential_0 );
    var _S315 : DiffPair_float_0 = s_fwd_with_table_value_0(_S313, step_7, _S314);
    var _S316 : DiffPair_float_0 = DiffPair_float_0( _S315.primal_0, _S315.differential_0 );
    return _S316;
}

fn rgb_levels_1( rgb_2 : vec3<f32>,  l_3 : Lookup_0,  s_2 : Shape_0) -> vec3<f32>
{
    if((l_3.mode_0) == u32(2))
    {
        return vec3<f32>(0.0f);
    }
    if((l_3.mode_0) == u32(1))
    {
        return rgb_2 * vec3<f32>(ratio_0(l_3.norm_0, l_3.step_0.x, s_2.black_2.x, s_2.scale_1.x, s_2.power_0.x, s_2.limit_0.x));
    }
    return vec3<f32>(level_0(rgb_2.x, l_3.step_0.x, s_2.black_2.x, s_2.scale_1.x, s_2.power_0.x, s_2.limit_0.x), level_0(rgb_2.y, l_3.step_0.y, s_2.black_2.y, s_2.scale_1.y, s_2.power_0.y, s_2.limit_0.y), level_0(rgb_2.z, l_3.step_0.z, s_2.black_2.z, s_2.scale_1.z, s_2.power_0.z, s_2.limit_0.z));
}

fn s_fwd_rgb_levels_0( rgb_3 : vec3<f32>,  l_4 : Lookup_0,  dps_0 : DiffPair_Shape_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    if((l_4.mode_0) == u32(2))
    {
        var _S317 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.0f), vec3<f32>(0.0f) );
        return _S317;
    }
    if((l_4.mode_0) == u32(1))
    {
        var _S318 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.black_2.x, dps_0.differential_0.black_2.x );
        var _S319 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.scale_1.x, dps_0.differential_0.scale_1.x );
        var _S320 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.power_0.x, dps_0.differential_0.power_0.x );
        var _S321 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.limit_0.x, dps_0.differential_0.limit_0.x );
        var _S322 : DiffPair_float_0 = s_fwd_ratio_0(l_4.norm_0, l_4.step_0.x, _S318, _S319, _S320, _S321);
        var _S323 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( rgb_3 * vec3<f32>(_S322.primal_0), vec3<f32>(_S322.differential_0) * rgb_3 );
        return _S323;
    }
    var _S324 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.black_2.x, dps_0.differential_0.black_2.x );
    var _S325 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.scale_1.x, dps_0.differential_0.scale_1.x );
    var _S326 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.power_0.x, dps_0.differential_0.power_0.x );
    var _S327 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.limit_0.x, dps_0.differential_0.limit_0.x );
    var _S328 : DiffPair_float_0 = s_fwd_level_0(rgb_3.x, l_4.step_0.x, _S324, _S325, _S326, _S327);
    var _S329 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.black_2.y, dps_0.differential_0.black_2.y );
    var _S330 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.scale_1.y, dps_0.differential_0.scale_1.y );
    var _S331 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.power_0.y, dps_0.differential_0.power_0.y );
    var _S332 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.limit_0.y, dps_0.differential_0.limit_0.y );
    var _S333 : DiffPair_float_0 = s_fwd_level_0(rgb_3.y, l_4.step_0.y, _S329, _S330, _S331, _S332);
    var _S334 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.black_2.z, dps_0.differential_0.black_2.z );
    var _S335 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.scale_1.z, dps_0.differential_0.scale_1.z );
    var _S336 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.power_0.z, dps_0.differential_0.power_0.z );
    var _S337 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.limit_0.z, dps_0.differential_0.limit_0.z );
    var _S338 : DiffPair_float_0 = s_fwd_level_0(rgb_3.z, l_4.step_0.z, _S334, _S335, _S336, _S337);
    var _S339 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S328.primal_0, _S333.primal_0, _S338.primal_0), vec3<f32>(_S328.differential_0, _S333.differential_0, _S338.differential_0) );
    return _S339;
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_0 : vec3<u32>)
{
    var _S340 : u32 = id_0.x;
    var _S341 : bool;
    if((_S340 >= (params_0.pass_0.width_0)))
    {
        _S341 = true;
    }
    else
    {
        _S341 = ((id_0.y) >= (params_0.pass_0.height_0));
    }
    if(_S341)
    {
        return;
    }
    var i_0 : u32 = id_0.y * params_0.pass_0.width_0 + _S340;
    var _S342 : RgbLevelsParams_std140_0 = params_0.rgb_levels_0;
    var _S343 : Levels_0 = levels_of_0(&(_S342));
    var _S344 : Shape_0 = shape_0(_S343, (_S342.autoscale_0) != u32(1));
    var pixel_0 : vec4<f32> = input_0[i_0];
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
        var _S345 : Lookup_0 = locate_0(rgb_4, _S344, &(_S342));
        var _S346 : vec3<f32> = rgb_levels_1(rgb_4, _S345, _S344);
        var _S347 : u32 = k_0 + u32(1);
        rgb_4 = _S346;
        k_0 = _S347;
    }
    output_0[i_0] = vec4<f32>(rgb_4, pixel_0.w);
    return;
}

fn s_fwd_rgb_levels_from_levels_0( rgb_5 : vec3<f32>,  l_5 : Lookup_0,  linked_3 : bool,  dplevels_0 : DiffPair_Levels_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S348 : DiffPair_Levels_0 = DiffPair_Levels_0( dplevels_0.primal_0, dplevels_0.differential_0 );
    var _S349 : DiffPair_Shape_0 = s_fwd_shape_0(_S348, linked_3);
    var _S350 : DiffPair_Shape_0 = DiffPair_Shape_0( _S349.primal_0, _S349.differential_0 );
    var _S351 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_rgb_levels_0(rgb_5, l_5, _S350);
    var _S352 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S351.primal_0, _S351.differential_0 );
    return _S352;
}

fn pick_0( d_0 : Levels_0,  channel_0 : u32,  point_0 : u32) -> f32
{
    var v_2 : vec3<f32>;
    if(point_0 == u32(0))
    {
        v_2 = d_0.black_1;
    }
    else
    {
        if(point_0 == u32(1))
        {
            v_2 = d_0.grey_1;
        }
        else
        {
            v_2 = d_0.white_1;
        }
    }
    return v_2[channel_0];
}

@compute
@workgroup_size(16, 16, 1)
fn rgblevels_backward(@builtin(global_invocation_id) id_1 : vec3<u32>)
{
    var _S353 : u32 = id_1.x;
    var _S354 : bool;
    if((_S353 >= (params_0.pass_0.width_0)))
    {
        _S354 = true;
    }
    else
    {
        _S354 = ((id_1.y) >= (params_0.pass_0.height_0));
    }
    if(_S354)
    {
        return;
    }
    var i_1 : u32 = id_1.y * params_0.pass_0.width_0 + _S353;
    var _S355 : RgbLevelsParams_std140_0 = params_0.rgb_levels_0;
    var linked_4 : bool = (_S355.autoscale_0) != u32(1);
    var _S356 : Levels_0 = levels_of_0(&(_S355));
    var rgb_6 : vec3<f32> = input_0[i_1].xyz;
    var _S357 : Lookup_0 = locate_0(rgb_6, shape_0(_S356, linked_4), &(_S355));
    var _S358 : Levels_0 = Levels_x24_syn_dzero_0();
    var d_1 : DiffPair_Levels_0;
    d_1.primal_0 = _S356;
    d_1.differential_0 = _S358;
    _S229(rgb_6, _S357, linked_4, &(d_1), derivative_in_0[i_1].xyz);
    derivative_out_0[i_1] = vec4<f32>(pick_0(d_1.differential_0, _S355.gradient_channel_0.x, _S355.gradient_point_0.x), pick_0(d_1.differential_0, _S355.gradient_channel_0.y, _S355.gradient_point_0.y), pick_0(d_1.differential_0, _S355.gradient_channel_0.z, _S355.gradient_point_0.z), pick_0(d_1.differential_0, _S355.gradient_channel_0.w, _S355.gradient_point_0.w));
    return;
}

fn add_at_0( v_3 : vec3<f32>,  c_0 : u32,  x_9 : f32) -> vec3<f32>
{
    return select(v_3, v_3 + vec3<f32>(x_9), vec3<u32>(u32(0), u32(1), u32(2)) == vec3<u32>(c_0));
}

fn add_slot_0( d_2 : ptr<function, Levels_0>,  channel_1 : u32,  point_1 : u32,  x_10 : f32)
{
    var _S359 : vec3<f32> = (*d_2).black_1;
    var _S360 : u32;
    if(point_1 == u32(0))
    {
        _S360 = channel_1;
    }
    else
    {
        _S360 = u32(3);
    }
    (*d_2).black_1 = add_at_0(_S359, _S360, x_10);
    var _S361 : vec3<f32> = (*d_2).grey_1;
    if(point_1 == u32(1))
    {
        _S360 = channel_1;
    }
    else
    {
        _S360 = u32(3);
    }
    (*d_2).grey_1 = add_at_0(_S361, _S360, x_10);
    var _S362 : vec3<f32> = (*d_2).white_1;
    if(point_1 == u32(2))
    {
        _S360 = channel_1;
    }
    else
    {
        _S360 = u32(3);
    }
    (*d_2).white_1 = add_at_0(_S362, _S360, x_10);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn rgblevels_forward_diff(@builtin(global_invocation_id) id_2 : vec3<u32>)
{
    var _S363 : u32 = id_2.x;
    var _S364 : bool;
    if((_S363 >= (params_0.pass_0.width_0)))
    {
        _S364 = true;
    }
    else
    {
        _S364 = ((id_2.y) >= (params_0.pass_0.height_0));
    }
    if(_S364)
    {
        return;
    }
    var i_2 : u32 = id_2.y * params_0.pass_0.width_0 + _S363;
    var _S365 : RgbLevelsParams_std140_0 = params_0.rgb_levels_0;
    var linked_5 : bool = (_S365.autoscale_0) != u32(1);
    var _S366 : Levels_0 = levels_of_0(&(_S365));
    var rgb_7 : vec3<f32> = input_0[i_2].xyz;
    var _S367 : Lookup_0 = locate_0(rgb_7, shape_0(_S366, linked_5), &(_S365));
    var v_4 : vec4<f32> = derivative_in_0[i_2];
    var direction_0 : Levels_0;
    var _S368 : vec3<f32> = vec3<f32>(0.0f);
    direction_0.black_1 = _S368;
    direction_0.grey_1 = _S368;
    direction_0.white_1 = _S368;
    add_slot_0(&(direction_0), _S365.gradient_channel_0.x, _S365.gradient_point_0.x, v_4.x);
    add_slot_0(&(direction_0), _S365.gradient_channel_0.y, _S365.gradient_point_0.y, v_4.y);
    add_slot_0(&(direction_0), _S365.gradient_channel_0.z, _S365.gradient_point_0.z, v_4.z);
    add_slot_0(&(direction_0), _S365.gradient_channel_0.w, _S365.gradient_point_0.w, v_4.w);
    var _S369 : DiffPair_Levels_0 = DiffPair_Levels_0( _S366, direction_0 );
    derivative_out_0[i_2] = vec4<f32>(s_fwd_rgb_levels_from_levels_0(rgb_7, _S367, linked_5, _S369).differential_0, 0.0f);
    return;
}

