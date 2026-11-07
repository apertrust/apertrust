// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/iop/vignette.c)
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

// Generated from slang/vignette.slang by tools/build-kernels.sh. Do not edit.

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

struct VignetteParams_std140_0
{
    @align(16) scale_1 : f32,
    @align(4) falloff_scale_0 : f32,
    @align(8) brightness_0 : f32,
    @align(4) saturation_0 : f32,
    @align(16) center_x_0 : f32,
    @align(4) center_y_0 : f32,
    @align(8) whratio_0 : f32,
    @align(4) shape_0 : f32,
    @align(16) autoratio_0 : u32,
    @align(4) unbound_0 : u32,
    @align(8) pad0_0 : u32,
    @align(4) pad1_0 : u32,
    @align(16) gradient_0 : vec4<u32>,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) vignette_0 : VignetteParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(0) @group(0) var<storage, read> input_0 : array<vec4<f32>>;

@binding(1) @group(0) var<storage, read_write> output_0 : array<vec4<f32>>;

@binding(8) @group(0) var<storage, read> derivative_in_0 : array<vec4<f32>>;

@binding(9) @group(0) var<storage, read_write> derivative_out_0 : array<vec4<f32>>;

struct Tuple_0
{
     value0_0 : vec3<f32>,
};

fn s_apply_clamp_0( x_0 : vec3<f32>,  minBound_0 : vec3<f32>,  maxBound_0 : vec3<f32>) -> Tuple_0
{
    var _S1 : Tuple_0 = Tuple_0( clamp(x_0, minBound_0, maxBound_0) );
    return _S1;
}

struct s_bwdCallableCtx_clamp_0
{
     _S2 : vec3<f32>,
     _S3 : vec3<f32>,
     _S4 : vec3<f32>,
};

struct DiffPair_vectorx3Cfloatx2C3x3E_0
{
     primal_0 : vec3<f32>,
     differential_0 : vec3<f32>,
};

struct DiffPair_float_0
{
     primal_0 : f32,
     differential_0 : f32,
};

fn _d_clamp_0( dpx_0 : ptr<function, DiffPair_float_0>,  dpMin_0 : ptr<function, DiffPair_float_0>,  dpMax_0 : ptr<function, DiffPair_float_0>,  dOut_0 : f32)
{
    var _S5 : DiffPair_float_0 = (*dpx_0);
    var _S6 : bool;
    if((((*dpx_0).primal_0) >= ((*dpMin_0).primal_0)))
    {
        _S6 = (((*dpx_0).primal_0) <= ((*dpMax_0).primal_0));
    }
    else
    {
        _S6 = false;
    }
    var _S7 : f32;
    if(_S6)
    {
        _S7 = dOut_0;
    }
    else
    {
        _S7 = 0.0f;
    }
    (*dpx_0).primal_0 = _S5.primal_0;
    (*dpx_0).differential_0 = _S7;
    var _S8 : DiffPair_float_0 = (*dpMin_0);
    if(((_S5.primal_0) < ((*dpMin_0).primal_0)))
    {
        _S7 = dOut_0;
    }
    else
    {
        _S7 = 0.0f;
    }
    (*dpMin_0).primal_0 = _S8.primal_0;
    (*dpMin_0).differential_0 = _S7;
    var _S9 : DiffPair_float_0 = (*dpMax_0);
    if((((*dpx_0).primal_0) > ((*dpMax_0).primal_0)))
    {
        _S7 = dOut_0;
    }
    else
    {
        _S7 = 0.0f;
    }
    (*dpMax_0).primal_0 = _S9.primal_0;
    (*dpMax_0).differential_0 = _S7;
    return;
}

fn _d_clamp_vector_0( dpx_1 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dpy_0 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dpz_0 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_1 : vec3<f32>)
{
    var left_dp_0 : DiffPair_float_0;
    left_dp_0.primal_0 = (*dpx_1).primal_0[i32(0)];
    left_dp_0.differential_0 = 0.0f;
    var middle_dp_0 : DiffPair_float_0;
    middle_dp_0.primal_0 = (*dpy_0).primal_0[i32(0)];
    middle_dp_0.differential_0 = 0.0f;
    var right_dp_0 : DiffPair_float_0;
    right_dp_0.primal_0 = (*dpz_0).primal_0[i32(0)];
    right_dp_0.differential_0 = 0.0f;
    _d_clamp_0(&(left_dp_0), &(middle_dp_0), &(right_dp_0), dOut_1[i32(0)]);
    var left_d_result_0 : vec3<f32>;
    left_d_result_0[i32(0)] = left_dp_0.differential_0;
    var middle_d_result_0 : vec3<f32>;
    middle_d_result_0[i32(0)] = middle_dp_0.differential_0;
    var right_d_result_0 : vec3<f32>;
    right_d_result_0[i32(0)] = right_dp_0.differential_0;
    var left_dp_1 : DiffPair_float_0;
    left_dp_1.primal_0 = (*dpx_1).primal_0[i32(1)];
    left_dp_1.differential_0 = 0.0f;
    var middle_dp_1 : DiffPair_float_0;
    middle_dp_1.primal_0 = (*dpy_0).primal_0[i32(1)];
    middle_dp_1.differential_0 = 0.0f;
    var right_dp_1 : DiffPair_float_0;
    right_dp_1.primal_0 = (*dpz_0).primal_0[i32(1)];
    right_dp_1.differential_0 = 0.0f;
    _d_clamp_0(&(left_dp_1), &(middle_dp_1), &(right_dp_1), dOut_1[i32(1)]);
    left_d_result_0[i32(1)] = left_dp_1.differential_0;
    middle_d_result_0[i32(1)] = middle_dp_1.differential_0;
    right_d_result_0[i32(1)] = right_dp_1.differential_0;
    var left_dp_2 : DiffPair_float_0;
    left_dp_2.primal_0 = (*dpx_1).primal_0[i32(2)];
    left_dp_2.differential_0 = 0.0f;
    var middle_dp_2 : DiffPair_float_0;
    middle_dp_2.primal_0 = (*dpy_0).primal_0[i32(2)];
    middle_dp_2.differential_0 = 0.0f;
    var right_dp_2 : DiffPair_float_0;
    right_dp_2.primal_0 = (*dpz_0).primal_0[i32(2)];
    right_dp_2.differential_0 = 0.0f;
    _d_clamp_0(&(left_dp_2), &(middle_dp_2), &(right_dp_2), dOut_1[i32(2)]);
    left_d_result_0[i32(2)] = left_dp_2.differential_0;
    middle_d_result_0[i32(2)] = middle_dp_2.differential_0;
    right_d_result_0[i32(2)] = right_dp_2.differential_0;
    (*dpx_1).primal_0 = (*dpx_1).primal_0;
    (*dpx_1).differential_0 = left_d_result_0;
    (*dpy_0).primal_0 = (*dpy_0).primal_0;
    (*dpy_0).differential_0 = middle_d_result_0;
    (*dpz_0).primal_0 = (*dpz_0).primal_0;
    (*dpz_0).differential_0 = right_d_result_0;
    return;
}

fn s_bwdProp_clamp_0( ctx_0 : s_bwdCallableCtx_clamp_0,  d_x_0 : ptr<function, vec3<f32>>,  d_minBound_0 : ptr<function, vec3<f32>>,  d_maxBound_0 : ptr<function, vec3<f32>>,  d_out_0 : vec3<f32>)
{
    var _S10 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S10.primal_0 = ctx_0._S2;
    var _S11 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S11.primal_0 = ctx_0._S3;
    var _S12 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S12.primal_0 = ctx_0._S4;
    _d_clamp_vector_0(&(_S10), &(_S11), &(_S12), d_out_0);
    (*d_x_0) = _S10.differential_0;
    (*d_minBound_0) = _S11.differential_0;
    (*d_maxBound_0) = _S12.differential_0;
    return;
}

struct Tuple_1
{
     value0_1 : f32,
};

fn s_apply_clamp_1( x_1 : f32,  minBound_1 : f32,  maxBound_1 : f32) -> Tuple_1
{
    var _S13 : Tuple_1 = Tuple_1( clamp(x_1, minBound_1, maxBound_1) );
    return _S13;
}

struct s_bwdCallableCtx_clamp_1
{
     _S14 : f32,
     _S15 : f32,
     _S16 : f32,
};

fn s_bwdProp_clamp_1( ctx_1 : s_bwdCallableCtx_clamp_1,  d_x_1 : ptr<function, f32>,  d_minBound_1 : ptr<function, f32>,  d_maxBound_1 : ptr<function, f32>,  d_out_1 : f32)
{
    var _S17 : DiffPair_float_0;
    _S17.primal_0 = ctx_1._S14;
    var _S18 : DiffPair_float_0;
    _S18.primal_0 = ctx_1._S15;
    var _S19 : DiffPair_float_0;
    _S19.primal_0 = ctx_1._S16;
    _d_clamp_0(&(_S17), &(_S18), &(_S19), d_out_1);
    (*d_x_1) = _S17.differential_0;
    (*d_minBound_1) = _S18.differential_0;
    (*d_maxBound_1) = _S19.differential_0;
    return;
}

struct Tuple_2
{
     value0_2 : f32,
};

fn pow0_0( x_2 : f32,  y_0 : f32) -> f32
{
    if((x_2 <= 0.0f))
    {
        return 0.0f;
    }
    return pow(x_2, y_0);
}

fn s_apply_pow0_0( x_3 : f32,  y_1 : f32) -> Tuple_2
{
    var _S20 : Tuple_2 = Tuple_2( pow0_0(x_3, y_1) );
    return _S20;
}

struct s_bwdCallableCtx_pow0_0
{
     _S21 : f32,
     _S22 : f32,
};

fn pow0_backward_0( x_4 : ptr<function, DiffPair_float_0>,  y_2 : ptr<function, DiffPair_float_0>,  d_out_2 : f32)
{
    if((((*x_4).primal_0) <= 0.0f))
    {
        (*x_4).primal_0 = (*x_4).primal_0;
        (*x_4).differential_0 = 0.0f;
        (*y_2).primal_0 = (*y_2).primal_0;
        (*y_2).differential_0 = 0.0f;
        return;
    }
    var d_x_2 : f32 = d_out_2 * (*y_2).primal_0 * pow((*x_4).primal_0, (*y_2).primal_0 - 1.0f);
    var d_y_0 : f32 = d_out_2 * pow((*x_4).primal_0, (*y_2).primal_0) * log((*x_4).primal_0);
    (*x_4).primal_0 = (*x_4).primal_0;
    (*x_4).differential_0 = d_x_2;
    (*y_2).primal_0 = (*y_2).primal_0;
    (*y_2).differential_0 = d_y_0;
    return;
}

fn s_bwdProp_pow0_0( ctx_2 : s_bwdCallableCtx_pow0_0,  d_x_3 : ptr<function, f32>,  d_y_1 : ptr<function, f32>,  d_out_3 : f32)
{
    var _S23 : DiffPair_float_0;
    _S23.primal_0 = ctx_2._S21;
    var _S24 : DiffPair_float_0;
    _S24.primal_0 = ctx_2._S22;
    pow0_backward_0(&(_S23), &(_S24), d_out_3);
    (*d_x_3) = _S23.differential_0;
    (*d_y_1) = _S24.differential_0;
    return;
}

struct Tuple_3
{
     value0_3 : vec2<f32>,
};

fn s_apply_abs_0( x_5 : vec2<f32>) -> Tuple_3
{
    var _S25 : Tuple_3 = Tuple_3( abs(x_5) );
    return _S25;
}

struct s_bwdCallableCtx_abs_0
{
     _S26 : vec2<f32>,
};

struct DiffPair_vectorx3Cfloatx2C2x3E_0
{
     primal_0 : vec2<f32>,
     differential_0 : vec2<f32>,
};

fn _d_abs_vector_0( dpx_2 : ptr<function, DiffPair_vectorx3Cfloatx2C2x3E_0>,  dOut_2 : vec2<f32>)
{
    var _S27 : vec2<f32> = vec2<f32>(0.0f);
    var _S28 : vec2<f32> = select(select(vec2<f32>(-1.0f), _S27, ((*dpx_2).primal_0) == _S27), vec2<f32>(1.0f), (((*dpx_2).primal_0) > _S27)) * dOut_2;
    (*dpx_2).primal_0 = (*dpx_2).primal_0;
    (*dpx_2).differential_0 = _S28;
    return;
}

fn s_bwdProp_abs_0( ctx_3 : s_bwdCallableCtx_abs_0,  d_x_4 : ptr<function, vec2<f32>>,  d_out_4 : vec2<f32>)
{
    var _S29 : DiffPair_vectorx3Cfloatx2C2x3E_0;
    _S29.primal_0 = ctx_3._S26;
    _d_abs_vector_0(&(_S29), d_out_4);
    (*d_x_4) = _S29.differential_0;
    return;
}

struct Tuple_4
{
     value0_4 : f32,
};

fn s_apply_min_0( x_6 : f32,  y_3 : f32) -> Tuple_4
{
    var _S30 : Tuple_4 = Tuple_4( min(x_6, y_3) );
    return _S30;
}

struct s_bwdCallableCtx_min_0
{
     _S31 : f32,
     _S32 : f32,
};

fn _d_min_0( dpx_3 : ptr<function, DiffPair_float_0>,  dpy_1 : ptr<function, DiffPair_float_0>,  dOut_3 : f32)
{
    var _S33 : DiffPair_float_0 = (*dpx_3);
    var _S34 : f32;
    if((((*dpx_3).primal_0) < ((*dpy_1).primal_0)))
    {
        _S34 = dOut_3;
    }
    else
    {
        if((((*dpx_3).primal_0) > ((*dpy_1).primal_0)))
        {
            _S34 = 0.0f;
        }
        else
        {
            _S34 = 0.5f * dOut_3;
        }
    }
    (*dpx_3).primal_0 = _S33.primal_0;
    (*dpx_3).differential_0 = _S34;
    var _S35 : DiffPair_float_0 = (*dpy_1);
    if((((*dpy_1).primal_0) < (_S33.primal_0)))
    {
        _S34 = dOut_3;
    }
    else
    {
        if((((*dpy_1).primal_0) > ((*dpx_3).primal_0)))
        {
            _S34 = 0.0f;
        }
        else
        {
            _S34 = 0.5f * dOut_3;
        }
    }
    (*dpy_1).primal_0 = _S35.primal_0;
    (*dpy_1).differential_0 = _S34;
    return;
}

fn s_bwdProp_min_0( ctx_4 : s_bwdCallableCtx_min_0,  d_x_5 : ptr<function, f32>,  d_y_2 : ptr<function, f32>,  d_out_5 : f32)
{
    var _S36 : DiffPair_float_0;
    _S36.primal_0 = ctx_4._S31;
    var _S37 : DiffPair_float_0;
    _S37.primal_0 = ctx_4._S32;
    _d_min_0(&(_S36), &(_S37), d_out_5);
    (*d_x_5) = _S36.differential_0;
    (*d_y_2) = _S37.differential_0;
    return;
}

struct Tuple_5
{
     value0_5 : f32,
};

fn s_apply_max_0( x_7 : f32,  y_4 : f32) -> Tuple_5
{
    var _S38 : Tuple_5 = Tuple_5( max(x_7, y_4) );
    return _S38;
}

struct s_bwdCallableCtx_max_0
{
     _S39 : f32,
     _S40 : f32,
};

fn _d_max_0( dpx_4 : ptr<function, DiffPair_float_0>,  dpy_2 : ptr<function, DiffPair_float_0>,  dOut_4 : f32)
{
    var _S41 : DiffPair_float_0 = (*dpx_4);
    var _S42 : f32;
    if((((*dpx_4).primal_0) > ((*dpy_2).primal_0)))
    {
        _S42 = dOut_4;
    }
    else
    {
        if((((*dpx_4).primal_0) < ((*dpy_2).primal_0)))
        {
            _S42 = 0.0f;
        }
        else
        {
            _S42 = 0.5f * dOut_4;
        }
    }
    (*dpx_4).primal_0 = _S41.primal_0;
    (*dpx_4).differential_0 = _S42;
    var _S43 : DiffPair_float_0 = (*dpy_2);
    if((((*dpy_2).primal_0) > (_S41.primal_0)))
    {
        _S42 = dOut_4;
    }
    else
    {
        if((((*dpy_2).primal_0) < ((*dpx_4).primal_0)))
        {
            _S42 = 0.0f;
        }
        else
        {
            _S42 = 0.5f * dOut_4;
        }
    }
    (*dpy_2).primal_0 = _S43.primal_0;
    (*dpy_2).differential_0 = _S42;
    return;
}

fn s_bwdProp_max_0( ctx_5 : s_bwdCallableCtx_max_0,  d_x_6 : ptr<function, f32>,  d_y_3 : ptr<function, f32>,  d_out_6 : f32)
{
    var _S44 : DiffPair_float_0;
    _S44.primal_0 = ctx_5._S39;
    var _S45 : DiffPair_float_0;
    _S45.primal_0 = ctx_5._S40;
    _d_max_0(&(_S44), &(_S45), d_out_6);
    (*d_x_6) = _S44.differential_0;
    (*d_y_3) = _S45.differential_0;
    return;
}

struct Vignette_0
{
     scale_2 : f32,
     falloff_scale_1 : f32,
     brightness_1 : f32,
     saturation_1 : f32,
     center_0 : vec2<f32>,
     whratio_1 : f32,
     shape_1 : f32,
};

struct DiffPair_Vignette_0
{
     primal_0 : Vignette_0,
     differential_0 : Vignette_0,
};

struct Frame_0
{
     full_0 : vec2<f32>,
     scale_3 : f32,
     autoratio_1 : bool,
     unbound_1 : bool,
};

struct s_paramCtx_s_bwdCallableCtx_vignette_from_settings_0
{
     value0_6 : vec3<f32>,
     value1_0 : vec2<f32>,
     value2_0 : bool,
     value3_0 : Vignette_0,
     value4_0 : Frame_0,
};

struct s_bwdCallableCtx_vignette_from_settings_0
{
     value0_7 : s_paramCtx_s_bwdCallableCtx_vignette_from_settings_0,
};

struct Geometry_0
{
     scale_4 : vec2<f32>,
     center_1 : vec2<f32>,
     inner_0 : f32,
     falloff_0 : f32,
     exponent_0 : f32,
     root_0 : f32,
};

struct Tuple_6
{
     value0_8 : Geometry_0,
};

fn s_apply_geometry_0( _S46 : Vignette_0,  f_0 : Frame_0) -> Tuple_6
{
    var scale_5 : vec2<f32>;
    if(f_0.autoratio_1)
    {
        scale_5 = vec2<f32>(2.0f) / (f_0.full_0 * vec2<f32>(f_0.scale_3));
    }
    else
    {
        var basis_0 : f32 = 2.0f / (s_apply_max_0(f_0.full_0.x, f_0.full_0.y).value0_5 * f_0.scale_3);
        if(((_S46.whratio_1) <= 1.0f))
        {
            scale_5 = vec2<f32>(basis_0 / _S46.whratio_1, basis_0);
        }
        else
        {
            scale_5 = vec2<f32>(basis_0, basis_0 / (2.0f - _S46.whratio_1));
        }
    }
    var _S47 : Tuple_5 = s_apply_max_0(_S46.shape_1, 0.0010000000474974513f);
    var _S48 : vec2<f32> = (f_0.full_0 * vec2<f32>(0.5f) + _S46.center_0 * f_0.full_0 / vec2<f32>(2.0f)) * vec2<f32>(f_0.scale_3) * scale_5;
    var _S49 : f32 = _S46.scale_2 / 100.0f;
    var _S50 : f32 = s_apply_max_0(_S46.falloff_scale_1, 100.0f / s_apply_min_0(f_0.full_0.x, f_0.full_0.y).value0_4).value0_5 / 100.0f;
    var _S51 : f32 = 2.0f / _S47.value0_5;
    var _S52 : f32 = _S47.value0_5 / 2.0f;
    var _S53 : Tuple_6;
    _S53.value0_8.scale_4 = scale_5;
    _S53.value0_8.center_1 = _S48;
    _S53.value0_8.inner_0 = _S49;
    _S53.value0_8.falloff_0 = _S50;
    _S53.value0_8.exponent_0 = _S51;
    _S53.value0_8.root_0 = _S52;
    return _S53;
}

struct s_paramCtx_s_bwdCallableCtx_geometry_0
{
     value0_9 : Vignette_0,
     value1_1 : Frame_0,
};

struct s_bwdCallableCtx_geometry_0
{
     value0_10 : s_paramCtx_s_bwdCallableCtx_geometry_0,
};

struct s_paramCtx_s_bwdCallableCtx_vignette_0
{
     value0_11 : vec3<f32>,
     value1_2 : vec2<f32>,
     value2_1 : bool,
     value3_1 : Geometry_0,
     value4_1 : f32,
     value5_0 : f32,
     value6_0 : bool,
};

struct s_bwdCallableCtx_vignette_0
{
     value0_12 : s_paramCtx_s_bwdCallableCtx_vignette_0,
};

fn Vignette_x24_syn_dzero_0() -> Vignette_0
{
    var result_0 : Vignette_0;
    result_0.scale_2 = 0.0f;
    result_0.falloff_scale_1 = 0.0f;
    result_0.brightness_1 = 0.0f;
    result_0.saturation_1 = 0.0f;
    result_0.center_0 = vec2<f32>(0.0f);
    result_0.whratio_1 = 0.0f;
    result_0.shape_1 = 0.0f;
    return result_0;
}

struct s_paramCtx_s_bwdCallableCtx_weight_0
{
     value0_13 : vec2<f32>,
     value1_3 : Geometry_0,
};

struct s_paramCtx_s_bwdCallableCtx_shade_0
{
     value0_14 : vec3<f32>,
     value1_4 : f32,
     value2_2 : f32,
     value3_2 : f32,
     value4_2 : bool,
};

struct s_bwdCallableCtx_shade_0
{
     value0_15 : s_paramCtx_s_bwdCallableCtx_shade_0,
};

struct s_bwdCallableCtx_weight_0
{
     value0_16 : s_paramCtx_s_bwdCallableCtx_weight_0,
};

struct Tuple_7
{
     value0_17 : f32,
};

fn s_apply_weight_0( position_0 : vec2<f32>,  _S54 : Geometry_0) -> Tuple_7
{
    var _S55 : Tuple_3 = s_apply_abs_0(position_0 * _S54.scale_4 - _S54.center_1);
    var _S56 : f32 = _S55.value0_3.x;
    var _S57 : f32 = _S55.value0_3.y;
    var _S58 : Tuple_5 = s_apply_max_0(_S56, _S57);
    var _S59 : Tuple_7;
    _S59.value0_17 = s_apply_clamp_1((_S58.value0_5 * s_apply_pow0_0(1.0f + s_apply_pow0_0(s_apply_min_0(_S56, _S57).value0_4 / s_apply_max_0(_S58.value0_5, 1.00000000317107685e-30f).value0_5, _S54.exponent_0).value0_2, _S54.root_0).value0_2 - _S54.inner_0) / _S54.falloff_0, 0.0f, 1.0f).value0_1;
    return _S59;
}

fn Geometry_x24_syn_dzero_0() -> Geometry_0
{
    var result_1 : Geometry_0;
    var _S60 : vec2<f32> = vec2<f32>(0.0f);
    result_1.scale_4 = _S60;
    result_1.center_1 = _S60;
    result_1.inner_0 = 0.0f;
    result_1.falloff_0 = 0.0f;
    result_1.exponent_0 = 0.0f;
    result_1.root_0 = 0.0f;
    return result_1;
}

fn s_bwdProp_shade_0( _S61 : s_bwdCallableCtx_shade_0,  _S62 : ptr<function, f32>,  _S63 : ptr<function, f32>,  _S64 : ptr<function, f32>,  _S65 : vec3<f32>)
{
    var _S66 : vec3<f32> = vec3<f32>(0.0f);
    var _S67 : bool = ((_S61.value0_15.value2_2) < 0.0f);
    var col_0 : vec3<f32>;
    if(_S67)
    {
        col_0 = _S61.value0_15.value0_14 * vec3<f32>((1.0f + _S61.value0_15.value1_4 * _S61.value0_15.value2_2));
    }
    else
    {
        col_0 = _S61.value0_15.value0_14 + vec3<f32>((_S61.value0_15.value1_4 * _S61.value0_15.value2_2));
    }
    var _S68 : bool = !_S61.value0_15.value4_2;
    var _S69 : s_bwdCallableCtx_clamp_0;
    if(_S68)
    {
        var _S70 : vec3<f32> = vec3<f32>(0.0f);
        var _S71 : vec3<f32> = vec3<f32>(1.0f);
        var _S72 : Tuple_0 = s_apply_clamp_0(col_0, _S70, _S71);
        var _S73 : s_bwdCallableCtx_clamp_0;
        _S73._S2 = col_0;
        _S73._S3 = _S70;
        _S73._S4 = _S71;
        col_0 = _S72.value0_0;
        _S69 = _S73;
    }
    else
    {
        _S69._S2 = _S66;
        _S69._S3 = _S66;
        _S69._S4 = _S66;
    }
    var _S74 : vec3<f32> = vec3<f32>(((col_0.x + col_0.y + col_0.z) / 3.0f)) - col_0;
    var _S75 : vec3<f32> = vec3<f32>((_S61.value0_15.value1_4 * _S61.value0_15.value3_2));
    var col_1 : vec3<f32> = col_0 - _S74 * _S75;
    var _S76 : s_bwdCallableCtx_clamp_0;
    if(_S68)
    {
        var _S77 : vec3<f32> = vec3<f32>(0.0f);
        var _S78 : vec3<f32> = vec3<f32>(1.0f);
        var _S79 : s_bwdCallableCtx_clamp_0;
        _S79._S2 = col_1;
        _S79._S3 = _S77;
        _S79._S4 = _S78;
        _S76 = _S79;
    }
    else
    {
        _S76._S2 = _S66;
        _S76._S3 = _S66;
        _S76._S4 = _S66;
    }
    if(_S68)
    {
        var _S80 : vec3<f32> = _S66;
        var _S81 : vec3<f32> = _S66;
        var _S82 : vec3<f32> = _S66;
        s_bwdProp_clamp_0(_S76, &(_S80), &(_S81), &(_S82), _S65);
        col_0 = _S80;
    }
    else
    {
        col_0 = _S65;
    }
    var _S83 : vec3<f32> = (vec3<f32>(0) - col_0);
    var _S84 : vec3<f32> = _S74 * _S83;
    var _S85 : f32 = _S84[i32(0)] + _S84[i32(1)] + _S84[i32(2)];
    var _S86 : vec3<f32> = _S75 * _S83;
    var _S87 : f32 = _S61.value0_15.value1_4 * _S85;
    var _S88 : f32 = _S61.value0_15.value3_2 * _S85;
    var _S89 : f32 = 0.3333333432674408f * (_S86[i32(0)] + _S86[i32(1)] + _S86[i32(2)]);
    var _S90 : vec3<f32> = col_0 + (vec3<f32>(0) - _S86) + vec3<f32>(_S89, _S89, _S89);
    if(_S68)
    {
        var _S91 : vec3<f32> = _S66;
        var _S92 : vec3<f32> = _S66;
        var _S93 : vec3<f32> = _S66;
        s_bwdProp_clamp_0(_S69, &(_S91), &(_S92), &(_S93), _S90);
        col_0 = _S91;
    }
    else
    {
        col_0 = _S90;
    }
    var _S94 : f32;
    var _S95 : f32;
    if(_S67)
    {
        var _S96 : vec3<f32> = _S61.value0_15.value0_14 * col_0;
        var _S97 : f32 = _S96[i32(0)] + _S96[i32(1)] + _S96[i32(2)];
        var _S98 : f32 = _S61.value0_15.value1_4 * _S97;
        _S94 = _S61.value0_15.value2_2 * _S97 + _S88;
        _S95 = _S98;
    }
    else
    {
        var _S99 : f32 = col_0[i32(0)] + col_0[i32(1)] + col_0[i32(2)];
        var _S100 : f32 = _S61.value0_15.value1_4 * _S99;
        _S94 = _S61.value0_15.value2_2 * _S99 + _S88;
        _S95 = _S100;
    }
    (*_S62) = _S94;
    (*_S63) = _S95;
    (*_S64) = _S87;
    return;
}

fn Geometry_x24_syn_dadd_0( SLANG_anonymous_0_0 : Geometry_0,  SLANG_anonymous_1_0 : Geometry_0) -> Geometry_0
{
    var result_2 : Geometry_0;
    result_2.scale_4 = SLANG_anonymous_0_0.scale_4 + SLANG_anonymous_1_0.scale_4;
    result_2.center_1 = SLANG_anonymous_0_0.center_1 + SLANG_anonymous_1_0.center_1;
    result_2.inner_0 = SLANG_anonymous_0_0.inner_0 + SLANG_anonymous_1_0.inner_0;
    result_2.falloff_0 = SLANG_anonymous_0_0.falloff_0 + SLANG_anonymous_1_0.falloff_0;
    result_2.exponent_0 = SLANG_anonymous_0_0.exponent_0 + SLANG_anonymous_1_0.exponent_0;
    result_2.root_0 = SLANG_anonymous_0_0.root_0 + SLANG_anonymous_1_0.root_0;
    return result_2;
}

fn s_bwdProp_weight_0( _S101 : s_bwdCallableCtx_weight_0,  _S102 : ptr<function, Geometry_0>,  _S103 : f32)
{
    var _S104 : vec2<f32> = _S101.value0_16.value0_13 * _S101.value0_16.value1_3.scale_4 - _S101.value0_16.value1_3.center_1;
    var _S105 : Tuple_3 = s_apply_abs_0(_S104);
    var _S106 : s_bwdCallableCtx_abs_0;
    _S106._S26 = _S104;
    var _S107 : f32 = _S105.value0_3.x;
    var _S108 : f32 = _S105.value0_3.y;
    var _S109 : Tuple_5 = s_apply_max_0(_S107, _S108);
    var _S110 : s_bwdCallableCtx_max_0;
    _S110._S39 = _S107;
    _S110._S40 = _S108;
    var _S111 : Tuple_4 = s_apply_min_0(_S107, _S108);
    var _S112 : s_bwdCallableCtx_min_0;
    _S112._S31 = _S107;
    _S112._S32 = _S108;
    var _S113 : Tuple_5 = s_apply_max_0(_S109.value0_5, 1.00000000317107685e-30f);
    var _S114 : s_bwdCallableCtx_max_0;
    _S114._S39 = _S109.value0_5;
    _S114._S40 = 1.00000000317107685e-30f;
    var _S115 : f32 = _S111.value0_4 / _S113.value0_5;
    var _S116 : f32 = _S113.value0_5 * _S113.value0_5;
    var _S117 : Tuple_2 = s_apply_pow0_0(_S115, _S101.value0_16.value1_3.exponent_0);
    var _S118 : s_bwdCallableCtx_pow0_0;
    _S118._S21 = _S115;
    _S118._S22 = _S101.value0_16.value1_3.exponent_0;
    var _S119 : f32 = 1.0f + _S117.value0_2;
    var _S120 : Tuple_2 = s_apply_pow0_0(_S119, _S101.value0_16.value1_3.root_0);
    var _S121 : s_bwdCallableCtx_pow0_0;
    _S121._S21 = _S119;
    _S121._S22 = _S101.value0_16.value1_3.root_0;
    var _S122 : f32 = _S109.value0_5 * _S120.value0_2 - _S101.value0_16.value1_3.inner_0;
    var _S123 : f32 = _S101.value0_16.value1_3.falloff_0 * _S101.value0_16.value1_3.falloff_0;
    var _S124 : s_bwdCallableCtx_clamp_1;
    _S124._S14 = _S122 / _S101.value0_16.value1_3.falloff_0;
    _S124._S15 = 0.0f;
    _S124._S16 = 1.0f;
    var _S125 : Geometry_0 = Geometry_x24_syn_dzero_0();
    var _S126 : f32 = 0.0f;
    var _S127 : f32 = 0.0f;
    var _S128 : f32 = 0.0f;
    s_bwdProp_clamp_1(_S124, &(_S126), &(_S127), &(_S128), _S103);
    var _S129 : f32 = _S126 / _S123;
    var _S130 : f32 = _S122 * - _S129;
    var s_diff_length_T_0 : f32 = _S101.value0_16.value1_3.falloff_0 * _S129;
    var _S131 : f32 = - s_diff_length_T_0;
    var _S132 : f32 = _S109.value0_5 * s_diff_length_T_0;
    var _S133 : f32 = _S120.value0_2 * s_diff_length_T_0;
    var _S134 : f32 = 0.0f;
    var _S135 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S121, &(_S134), &(_S135), _S132);
    var _S136 : f32 = 0.0f;
    var _S137 : f32 = 0.0f;
    s_bwdProp_pow0_0(_S118, &(_S136), &(_S137), _S134);
    var _S138 : f32 = _S136 / _S116;
    var _S139 : f32 = _S111.value0_4 * - _S138;
    var _S140 : f32 = _S113.value0_5 * _S138;
    var _S141 : f32 = 0.0f;
    var _S142 : f32 = 0.0f;
    s_bwdProp_max_0(_S114, &(_S141), &(_S142), _S139);
    var _S143 : f32 = 0.0f;
    var _S144 : f32 = 0.0f;
    s_bwdProp_min_0(_S112, &(_S143), &(_S144), _S140);
    var _S145 : f32 = _S133 + _S141;
    var _S146 : f32 = 0.0f;
    var _S147 : f32 = 0.0f;
    s_bwdProp_max_0(_S110, &(_S146), &(_S147), _S145);
    var _S148 : vec2<f32> = vec2<f32>(_S143 + _S146, _S144 + _S147);
    var _S149 : vec2<f32> = vec2<f32>(0.0f);
    s_bwdProp_abs_0(_S106, &(_S149), _S148);
    var _S150 : vec2<f32> = (vec2<f32>(0) - _S149);
    var _S151 : vec2<f32> = _S101.value0_16.value0_13 * _S149;
    var _S152 : Geometry_0 = _S125;
    _S152.falloff_0 = _S130;
    _S152.inner_0 = _S131;
    _S152.root_0 = _S135;
    _S152.exponent_0 = _S137;
    _S152.center_1 = _S150;
    _S152.scale_4 = _S151;
    (*_S102) = Geometry_x24_syn_dadd_0(_S125, _S152);
    return;
}

fn s_bwdProp_vignette_0( _S153 : s_bwdCallableCtx_vignette_0,  _S154 : ptr<function, Geometry_0>,  _S155 : ptr<function, f32>,  _S156 : ptr<function, f32>,  _S157 : vec3<f32>)
{
    var _S158 : vec2<f32> = vec2<f32>(0.0f);
    var _S159 : Geometry_0 = Geometry_0( _S158, _S158, 0.0f, 0.0f, 0.0f, 0.0f );
    var _S160 : s_paramCtx_s_bwdCallableCtx_weight_0 = s_paramCtx_s_bwdCallableCtx_weight_0( _S158, _S159 );
    var _S161 : s_paramCtx_s_bwdCallableCtx_shade_0 = s_paramCtx_s_bwdCallableCtx_shade_0( vec3<f32>(0.0f), 0.0f, 0.0f, 0.0f, false );
    var _S162 : bool = !!_S153.value0_12.value2_1;
    var _S163 : s_bwdCallableCtx_shade_0;
    var _S164 : s_bwdCallableCtx_weight_0;
    if(_S162)
    {
        var _S165 : Tuple_7 = s_apply_weight_0(_S153.value0_12.value1_2, _S153.value0_12.value3_1);
        var _S166 : s_paramCtx_s_bwdCallableCtx_weight_0;
        _S166.value0_13 = _S153.value0_12.value1_2;
        _S166.value1_3 = _S153.value0_12.value3_1;
        var _S167 : s_paramCtx_s_bwdCallableCtx_shade_0;
        _S167.value0_14 = _S153.value0_12.value0_11;
        _S167.value1_4 = _S165.value0_17;
        _S167.value2_2 = _S153.value0_12.value4_1;
        _S167.value3_2 = _S153.value0_12.value5_0;
        _S167.value4_2 = _S153.value0_12.value6_0;
        _S163.value0_15 = _S167;
        _S164.value0_16 = _S166;
    }
    else
    {
        _S163.value0_15 = _S161;
        _S164.value0_16 = _S160;
    }
    var _S168 : Geometry_0 = Geometry_x24_syn_dzero_0();
    var _S169 : Geometry_0;
    var _S170 : f32;
    var _S171 : f32;
    if(_S162)
    {
        var _S172 : f32 = 0.0f;
        var _S173 : f32 = 0.0f;
        var _S174 : f32 = 0.0f;
        s_bwdProp_shade_0(_S163, &(_S172), &(_S173), &(_S174), _S157);
        var _S175 : Geometry_0;
        _S175.scale_4 = _S158;
        _S175.center_1 = _S158;
        _S175.inner_0 = 0.0f;
        _S175.falloff_0 = 0.0f;
        _S175.exponent_0 = 0.0f;
        _S175.root_0 = 0.0f;
        s_bwdProp_weight_0(_S164, &(_S175), _S172);
        _S169 = Geometry_x24_syn_dadd_0(_S175, _S168);
        _S170 = _S173;
        _S171 = _S174;
    }
    else
    {
        _S169 = _S168;
        _S170 = 0.0f;
        _S171 = 0.0f;
    }
    (*_S154) = _S169;
    (*_S155) = _S170;
    (*_S156) = _S171;
    return;
}

fn Vignette_x24_syn_dadd_0( SLANG_anonymous_0_1 : Vignette_0,  SLANG_anonymous_1_1 : Vignette_0) -> Vignette_0
{
    var result_3 : Vignette_0;
    result_3.scale_2 = SLANG_anonymous_0_1.scale_2 + SLANG_anonymous_1_1.scale_2;
    result_3.falloff_scale_1 = SLANG_anonymous_0_1.falloff_scale_1 + SLANG_anonymous_1_1.falloff_scale_1;
    result_3.brightness_1 = SLANG_anonymous_0_1.brightness_1 + SLANG_anonymous_1_1.brightness_1;
    result_3.saturation_1 = SLANG_anonymous_0_1.saturation_1 + SLANG_anonymous_1_1.saturation_1;
    result_3.center_0 = SLANG_anonymous_0_1.center_0 + SLANG_anonymous_1_1.center_0;
    result_3.whratio_1 = SLANG_anonymous_0_1.whratio_1 + SLANG_anonymous_1_1.whratio_1;
    result_3.shape_1 = SLANG_anonymous_0_1.shape_1 + SLANG_anonymous_1_1.shape_1;
    return result_3;
}

fn s_bwdProp_geometry_0( _S176 : s_bwdCallableCtx_geometry_0,  _S177 : ptr<function, Vignette_0>,  _S178 : Geometry_0)
{
    var scale_6 : vec2<f32>;
    var _S179 : f32;
    var _S180 : f32;
    var _S181 : f32;
    var _S182 : bool;
    if(_S176.value0_10.value1_1.autoratio_1)
    {
        scale_6 = vec2<f32>(2.0f) / (_S176.value0_10.value1_1.full_0 * vec2<f32>(_S176.value0_10.value1_1.scale_3));
        _S182 = false;
        _S179 = 0.0f;
        _S180 = 0.0f;
        _S181 = 0.0f;
    }
    else
    {
        var basis_1 : f32 = 2.0f / (s_apply_max_0(_S176.value0_10.value1_1.full_0.x, _S176.value0_10.value1_1.full_0.y).value0_5 * _S176.value0_10.value1_1.scale_3);
        var _S183 : bool = ((_S176.value0_10.value0_9.whratio_1) <= 1.0f);
        if(_S183)
        {
            var _S184 : f32 = _S176.value0_10.value0_9.whratio_1 * _S176.value0_10.value0_9.whratio_1;
            scale_6 = vec2<f32>(basis_1 / _S176.value0_10.value0_9.whratio_1, basis_1);
            _S179 = _S184;
            _S180 = 0.0f;
        }
        else
        {
            var _S185 : f32 = 2.0f - _S176.value0_10.value0_9.whratio_1;
            var _S186 : f32 = _S185 * _S185;
            scale_6 = vec2<f32>(basis_1, basis_1 / _S185);
            _S179 = 0.0f;
            _S180 = _S186;
        }
        var _S187 : f32 = _S180;
        _S182 = _S183;
        _S180 = basis_1;
        _S181 = _S187;
    }
    var _S188 : vec2<f32> = vec2<f32>(0.5f);
    var center_2 : vec2<f32> = _S176.value0_10.value1_1.full_0 * _S188 + _S176.value0_10.value0_9.center_0 * _S176.value0_10.value1_1.full_0 / vec2<f32>(2.0f);
    var min_falloff_0 : f32 = 100.0f / s_apply_min_0(_S176.value0_10.value1_1.full_0.x, _S176.value0_10.value1_1.full_0.y).value0_4;
    var _S189 : Tuple_5 = s_apply_max_0(_S176.value0_10.value0_9.shape_1, 0.0010000000474974513f);
    var _S190 : s_bwdCallableCtx_max_0;
    _S190._S39 = _S176.value0_10.value0_9.shape_1;
    _S190._S40 = 0.0010000000474974513f;
    var _S191 : s_bwdCallableCtx_max_0 = _S190;
    var _S192 : vec2<f32> = vec2<f32>(_S176.value0_10.value1_1.scale_3);
    var _S193 : vec2<f32> = center_2 * _S192;
    var _S194 : s_bwdCallableCtx_max_0;
    _S194._S39 = _S176.value0_10.value0_9.falloff_scale_1;
    _S194._S40 = min_falloff_0;
    var _S195 : s_bwdCallableCtx_max_0 = _S194;
    var _S196 : f32 = _S189.value0_5 * _S189.value0_5;
    var _S197 : Vignette_0 = Vignette_x24_syn_dzero_0();
    var _S198 : f32 = 0.5f * _S178.root_0;
    var _S199 : f32 = 2.0f * - (_S178.exponent_0 / _S196);
    var _S200 : f32 = 0.0099999997764825821f * _S178.falloff_0;
    var _S201 : f32 = 0.0f;
    var _S202 : f32 = 0.0f;
    s_bwdProp_max_0(_S195, &(_S201), &(_S202), _S200);
    var _S203 : f32 = 0.0099999997764825821f * _S178.inner_0;
    var _S204 : vec2<f32> = _S193 * _S178.center_1;
    var s_diff_center_T_0 : vec2<f32> = _S192 * (scale_6 * _S178.center_1);
    var _S205 : f32 = _S198 + _S199;
    var _S206 : f32 = 0.0f;
    var _S207 : f32 = 0.0f;
    s_bwdProp_max_0(_S191, &(_S206), &(_S207), _S205);
    var _S208 : vec2<f32> = _S176.value0_10.value1_1.full_0 * (_S188 * s_diff_center_T_0);
    var _S209 : vec2<f32> = _S178.scale_4 + _S204;
    var _S210 : Vignette_0 = _S197;
    _S210.falloff_scale_1 = _S201;
    _S210.scale_2 = _S203;
    _S210.shape_1 = _S206;
    _S210.center_0 = _S208;
    var _S211 : Vignette_0 = Vignette_x24_syn_dadd_0(_S197, _S210);
    var _S212 : Vignette_0;
    if(_S176.value0_10.value1_1.autoratio_1)
    {
        _S212 = _S211;
    }
    else
    {
        if(_S182)
        {
            _S179 = _S180 * - (_S209[i32(0)] / _S179);
        }
        else
        {
            _S179 = - (_S180 * - (_S209[i32(1)] / _S181));
        }
        var _S213 : Vignette_0 = _S197;
        _S213.whratio_1 = _S179;
        _S212 = Vignette_x24_syn_dadd_0(_S211, _S213);
    }
    (*_S177) = _S212;
    return;
}

fn s_bwdProp_vignette_from_settings_0( _S214 : s_bwdCallableCtx_vignette_from_settings_0,  _S215 : ptr<function, Vignette_0>,  _S216 : vec3<f32>)
{
    var _S217 : Tuple_6 = s_apply_geometry_0(_S214.value0_7.value3_0, _S214.value0_7.value4_0);
    var _S218 : s_paramCtx_s_bwdCallableCtx_geometry_0;
    _S218.value0_9 = _S214.value0_7.value3_0;
    _S218.value1_1 = _S214.value0_7.value4_0;
    var _S219 : s_bwdCallableCtx_geometry_0 = s_bwdCallableCtx_geometry_0( _S218 );
    var _S220 : s_paramCtx_s_bwdCallableCtx_vignette_0;
    _S220.value0_11 = _S214.value0_7.value0_6;
    _S220.value1_2 = _S214.value0_7.value1_0;
    _S220.value2_1 = _S214.value0_7.value2_0;
    _S220.value3_1 = _S217.value0_8;
    _S220.value4_1 = _S214.value0_7.value3_0.brightness_1;
    _S220.value5_0 = _S214.value0_7.value3_0.saturation_1;
    _S220.value6_0 = _S214.value0_7.value4_0.unbound_1;
    var _S221 : s_bwdCallableCtx_vignette_0 = s_bwdCallableCtx_vignette_0( _S220 );
    var _S222 : Vignette_0 = Vignette_x24_syn_dzero_0();
    var _S223 : vec2<f32> = vec2<f32>(0.0f);
    var _S224 : Geometry_0;
    _S224.scale_4 = _S223;
    _S224.center_1 = _S223;
    _S224.inner_0 = 0.0f;
    _S224.falloff_0 = 0.0f;
    _S224.exponent_0 = 0.0f;
    _S224.root_0 = 0.0f;
    var _S225 : f32 = 0.0f;
    var _S226 : f32 = 0.0f;
    s_bwdProp_vignette_0(_S221, &(_S224), &(_S225), &(_S226), _S216);
    var _S227 : Vignette_0;
    _S227.scale_2 = 0.0f;
    _S227.falloff_scale_1 = 0.0f;
    _S227.brightness_1 = 0.0f;
    _S227.saturation_1 = 0.0f;
    _S227.center_0 = _S223;
    _S227.whratio_1 = 0.0f;
    _S227.shape_1 = 0.0f;
    s_bwdProp_geometry_0(_S219, &(_S227), _S224);
    var _S228 : Vignette_0 = Vignette_x24_syn_dadd_0(_S227, _S222);
    var _S229 : Vignette_0 = _S222;
    _S229.saturation_1 = _S226;
    _S229.brightness_1 = _S225;
    (*_S215) = Vignette_x24_syn_dadd_0(_S228, _S229);
    return;
}

fn _S230( _S231 : vec3<f32>,  _S232 : vec2<f32>,  _S233 : bool,  _S234 : ptr<function, DiffPair_Vignette_0>,  _S235 : Frame_0,  _S236 : vec3<f32>)
{
    var _S237 : s_paramCtx_s_bwdCallableCtx_vignette_from_settings_0;
    _S237.value0_6 = _S231;
    _S237.value1_0 = _S232;
    _S237.value2_0 = _S233;
    _S237.value3_0 = (*_S234).primal_0;
    _S237.value4_0 = _S235;
    var _S238 : s_bwdCallableCtx_vignette_from_settings_0 = s_bwdCallableCtx_vignette_from_settings_0( _S237 );
    var _S239 : Vignette_0 = (*_S234).differential_0;
    s_bwdProp_vignette_from_settings_0(_S238, &(_S239), _S236);
    (*_S234).differential_0 = _S239;
    return;
}

fn _d_clamp_1( dpx_5 : DiffPair_float_0,  dpMin_1 : DiffPair_float_0,  dpMax_1 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S240 : f32 = clamp(dpx_5.primal_0, dpMin_1.primal_0, dpMax_1.primal_0);
    var _S241 : bool;
    if(((dpx_5.primal_0) >= (dpMin_1.primal_0)))
    {
        _S241 = ((dpx_5.primal_0) <= (dpMax_1.primal_0));
    }
    else
    {
        _S241 = false;
    }
    var _S242 : f32;
    if(_S241)
    {
        _S242 = dpx_5.differential_0;
    }
    else
    {
        if(((dpx_5.primal_0) < (dpMin_1.primal_0)))
        {
            _S242 = dpMin_1.differential_0;
        }
        else
        {
            _S242 = dpMax_1.differential_0;
        }
    }
    var _S243 : DiffPair_float_0 = DiffPair_float_0( _S240, _S242 );
    return _S243;
}

fn _d_clamp_vector_1( dpx_6 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpy_3 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpz_1 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S244 : DiffPair_float_0 = DiffPair_float_0( dpx_6.primal_0[i32(0)], dpx_6.differential_0[i32(0)] );
    var _S245 : DiffPair_float_0 = DiffPair_float_0( dpy_3.primal_0[i32(0)], dpy_3.differential_0[i32(0)] );
    var _S246 : DiffPair_float_0 = DiffPair_float_0( dpz_1.primal_0[i32(0)], dpz_1.differential_0[i32(0)] );
    var dp_elem_0 : DiffPair_float_0 = _d_clamp_1(_S244, _S245, _S246);
    var result_4 : vec3<f32>;
    result_4[i32(0)] = dp_elem_0.primal_0;
    var d_result_0 : vec3<f32>;
    d_result_0[i32(0)] = dp_elem_0.differential_0;
    var _S247 : DiffPair_float_0 = DiffPair_float_0( dpx_6.primal_0[i32(1)], dpx_6.differential_0[i32(1)] );
    var _S248 : DiffPair_float_0 = DiffPair_float_0( dpy_3.primal_0[i32(1)], dpy_3.differential_0[i32(1)] );
    var _S249 : DiffPair_float_0 = DiffPair_float_0( dpz_1.primal_0[i32(1)], dpz_1.differential_0[i32(1)] );
    var dp_elem_1 : DiffPair_float_0 = _d_clamp_1(_S247, _S248, _S249);
    result_4[i32(1)] = dp_elem_1.primal_0;
    d_result_0[i32(1)] = dp_elem_1.differential_0;
    var _S250 : DiffPair_float_0 = DiffPair_float_0( dpx_6.primal_0[i32(2)], dpx_6.differential_0[i32(2)] );
    var _S251 : DiffPair_float_0 = DiffPair_float_0( dpy_3.primal_0[i32(2)], dpy_3.differential_0[i32(2)] );
    var _S252 : DiffPair_float_0 = DiffPair_float_0( dpz_1.primal_0[i32(2)], dpz_1.differential_0[i32(2)] );
    var dp_elem_2 : DiffPair_float_0 = _d_clamp_1(_S250, _S251, _S252);
    result_4[i32(2)] = dp_elem_2.primal_0;
    d_result_0[i32(2)] = dp_elem_2.differential_0;
    var _S253 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( result_4, d_result_0 );
    return _S253;
}

fn _d_abs_vector_1( dpx_7 : DiffPair_vectorx3Cfloatx2C2x3E_0) -> DiffPair_vectorx3Cfloatx2C2x3E_0
{
    var _S254 : vec2<f32> = vec2<f32>(0.0f);
    var _S255 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( abs(dpx_7.primal_0), select(select(vec2<f32>(-1.0f), _S254, (dpx_7.primal_0) == _S254), vec2<f32>(1.0f), ((dpx_7.primal_0) > _S254)) * dpx_7.differential_0 );
    return _S255;
}

fn _d_min_1( dpx_8 : DiffPair_float_0,  dpy_4 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S256 : f32 = min(dpx_8.primal_0, dpy_4.primal_0);
    var _S257 : f32;
    if(((dpx_8.primal_0) < (dpy_4.primal_0)))
    {
        _S257 = dpx_8.differential_0;
    }
    else
    {
        if(((dpx_8.primal_0) > (dpy_4.primal_0)))
        {
            _S257 = dpy_4.differential_0;
        }
        else
        {
            _S257 = 0.5f * (dpx_8.differential_0 + dpy_4.differential_0);
        }
    }
    var _S258 : DiffPair_float_0 = DiffPair_float_0( _S256, _S257 );
    return _S258;
}

fn _d_max_1( dpx_9 : DiffPair_float_0,  dpy_5 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S259 : f32 = max(dpx_9.primal_0, dpy_5.primal_0);
    var _S260 : f32;
    if(((dpx_9.primal_0) > (dpy_5.primal_0)))
    {
        _S260 = dpx_9.differential_0;
    }
    else
    {
        if(((dpx_9.primal_0) < (dpy_5.primal_0)))
        {
            _S260 = dpy_5.differential_0;
        }
        else
        {
            _S260 = 0.5f * (dpx_9.differential_0 + dpy_5.differential_0);
        }
    }
    var _S261 : DiffPair_float_0 = DiffPair_float_0( _S259, _S260 );
    return _S261;
}

fn vignette_of_0( p_0 : ptr<function, VignetteParams_std140_0>) -> Vignette_0
{
    var v_0 : Vignette_0;
    v_0.scale_2 = (*p_0).scale_1;
    v_0.falloff_scale_1 = (*p_0).falloff_scale_0;
    v_0.brightness_1 = (*p_0).brightness_0;
    v_0.saturation_1 = (*p_0).saturation_0;
    v_0.center_0 = vec2<f32>((*p_0).center_x_0, (*p_0).center_y_0);
    v_0.whratio_1 = (*p_0).whratio_0;
    v_0.shape_1 = (*p_0).shape_0;
    return v_0;
}

fn frame_of_0( p_1 : ptr<function, Params_std140_0>) -> Frame_0
{
    var f_1 : Frame_0;
    f_1.full_0 = vec2<f32>(f32((*p_1).pass_0.full_width_0), f32((*p_1).pass_0.full_height_0));
    f_1.scale_3 = (*p_1).pass_0.scale_0;
    f_1.autoratio_1 = ((*p_1).vignette_0.autoratio_0) != u32(0);
    f_1.unbound_1 = ((*p_1).vignette_0.unbound_0) != u32(0);
    return f_1;
}

fn geometry_0( v_1 : Vignette_0,  f_2 : Frame_0) -> Geometry_0
{
    var scale_7 : vec2<f32>;
    if(f_2.autoratio_1)
    {
        scale_7 = vec2<f32>(2.0f) / (f_2.full_0 * vec2<f32>(f_2.scale_3));
    }
    else
    {
        var basis_2 : f32 = 2.0f / (max(f_2.full_0.x, f_2.full_0.y) * f_2.scale_3);
        if(((v_1.whratio_1) <= 1.0f))
        {
            scale_7 = vec2<f32>(basis_2 / v_1.whratio_1, basis_2);
        }
        else
        {
            scale_7 = vec2<f32>(basis_2, basis_2 / (2.0f - v_1.whratio_1));
        }
    }
    var center_3 : vec2<f32> = f_2.full_0 * vec2<f32>(0.5f) + v_1.center_0 * f_2.full_0 / vec2<f32>(2.0f);
    var min_falloff_1 : f32 = 100.0f / min(f_2.full_0.x, f_2.full_0.y);
    var _S262 : f32 = max(v_1.shape_1, 0.0010000000474974513f);
    var g_0 : Geometry_0;
    g_0.scale_4 = scale_7;
    g_0.center_1 = center_3 * vec2<f32>(f_2.scale_3) * scale_7;
    g_0.inner_0 = v_1.scale_2 / 100.0f;
    g_0.falloff_0 = max(v_1.falloff_scale_1, min_falloff_1) / 100.0f;
    g_0.exponent_0 = 2.0f / _S262;
    g_0.root_0 = _S262 / 2.0f;
    return g_0;
}

struct DiffPair_Geometry_0
{
     primal_0 : Geometry_0,
     differential_0 : Geometry_0,
};

fn s_fwd_geometry_0( dpv_0 : DiffPair_Vignette_0,  f_3 : Frame_0) -> DiffPair_Geometry_0
{
    var scale_8 : vec2<f32>;
    var s_diff_scale_0 : vec2<f32>;
    if(f_3.autoratio_1)
    {
        var _S263 : vec2<f32> = vec2<f32>(0.0f);
        scale_8 = vec2<f32>(2.0f) / (f_3.full_0 * vec2<f32>(f_3.scale_3));
        s_diff_scale_0 = _S263;
    }
    else
    {
        var _S264 : DiffPair_float_0 = DiffPair_float_0( f_3.full_0.x, 0.0f );
        var _S265 : DiffPair_float_0 = DiffPair_float_0( f_3.full_0.y, 0.0f );
        var _S266 : DiffPair_float_0 = _d_max_1(_S264, _S265);
        var _S267 : f32 = _S266.primal_0 * f_3.scale_3;
        var basis_3 : f32 = 2.0f / _S267;
        var s_diff_basis_0 : f32 = (0.0f - 2.0f * (_S266.differential_0 * f_3.scale_3)) / (_S267 * _S267);
        if(((dpv_0.primal_0.whratio_1) <= 1.0f))
        {
            var _S268 : vec2<f32> = vec2<f32>((s_diff_basis_0 * dpv_0.primal_0.whratio_1 - basis_3 * dpv_0.differential_0.whratio_1) / (dpv_0.primal_0.whratio_1 * dpv_0.primal_0.whratio_1), s_diff_basis_0);
            scale_8 = vec2<f32>(basis_3 / dpv_0.primal_0.whratio_1, basis_3);
            s_diff_scale_0 = _S268;
        }
        else
        {
            var _S269 : f32 = 2.0f - dpv_0.primal_0.whratio_1;
            var _S270 : vec2<f32> = vec2<f32>(s_diff_basis_0, (s_diff_basis_0 * _S269 - basis_3 * (0.0f - dpv_0.differential_0.whratio_1)) / (_S269 * _S269));
            scale_8 = vec2<f32>(basis_3, basis_3 / _S269);
            s_diff_scale_0 = _S270;
        }
    }
    var _S271 : vec2<f32> = vec2<f32>(0.5f);
    var _S272 : DiffPair_float_0 = DiffPair_float_0( f_3.full_0.x, 0.0f );
    var _S273 : DiffPair_float_0 = DiffPair_float_0( f_3.full_0.y, 0.0f );
    var _S274 : DiffPair_float_0 = _d_min_1(_S272, _S273);
    var _S275 : DiffPair_float_0 = DiffPair_float_0( dpv_0.primal_0.shape_1, dpv_0.differential_0.shape_1 );
    var _S276 : DiffPair_float_0 = DiffPair_float_0( 0.0010000000474974513f, 0.0f );
    var _S277 : DiffPair_float_0 = _d_max_1(_S275, _S276);
    var _S278 : vec2<f32> = vec2<f32>(f_3.scale_3);
    var _S279 : vec2<f32> = (f_3.full_0 * _S271 + dpv_0.primal_0.center_0 * f_3.full_0 / vec2<f32>(2.0f)) * _S278;
    var _S280 : DiffPair_float_0 = DiffPair_float_0( dpv_0.primal_0.falloff_scale_1, dpv_0.differential_0.falloff_scale_1 );
    var _S281 : DiffPair_float_0 = DiffPair_float_0( 100.0f / _S274.primal_0, (0.0f - 100.0f * _S274.differential_0) / (_S274.primal_0 * _S274.primal_0) );
    var _S282 : DiffPair_float_0 = _d_max_1(_S280, _S281);
    var _S283 : Geometry_0 = Geometry_0( scale_8, _S279 * scale_8, dpv_0.primal_0.scale_2 / 100.0f, _S282.primal_0 / 100.0f, 2.0f / _S277.primal_0, _S277.primal_0 / 2.0f );
    var _S284 : Geometry_0 = Geometry_0( s_diff_scale_0, dpv_0.differential_0.center_0 * f_3.full_0 * _S271 * _S278 * scale_8 + s_diff_scale_0 * _S279, dpv_0.differential_0.scale_2 * 0.0099999997764825821f, _S282.differential_0 * 0.0099999997764825821f, (0.0f - 2.0f * _S277.differential_0) / (_S277.primal_0 * _S277.primal_0), _S277.differential_0 * 0.5f );
    var _S285 : DiffPair_Geometry_0 = DiffPair_Geometry_0( _S283, _S284 );
    return _S285;
}

fn position_of_0( id_0 : vec3<u32>) -> vec2<f32>
{
    return vec2<f32>(f32(params_0.pass_0.origin_x_0 + id_0.x), f32(params_0.pass_0.origin_y_0 + id_0.y));
}

fn pow0_forward_0( x_8 : DiffPair_float_0,  y_5 : DiffPair_float_0) -> DiffPair_float_0
{
    if(((x_8.primal_0) <= 0.0f))
    {
        var _S286 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
        return _S286;
    }
    var value_0 : f32 = pow(x_8.primal_0, y_5.primal_0);
    var _S287 : DiffPair_float_0 = DiffPair_float_0( value_0, x_8.differential_0 * y_5.primal_0 * pow(x_8.primal_0, y_5.primal_0 - 1.0f) + y_5.differential_0 * value_0 * log(x_8.primal_0) );
    return _S287;
}

fn weight_0( position_1 : vec2<f32>,  g_1 : Geometry_0) -> f32
{
    var pv_0 : vec2<f32> = abs(position_1 * g_1.scale_4 - g_1.center_1);
    var _S288 : f32 = pv_0.x;
    var _S289 : f32 = pv_0.y;
    var _S290 : f32 = max(_S288, _S289);
    return clamp((_S290 * pow0_0(1.0f + pow0_0(min(_S288, _S289) / max(_S290, 1.00000000317107685e-30f), g_1.exponent_0), g_1.root_0) - g_1.inner_0) / g_1.falloff_0, 0.0f, 1.0f);
}

fn s_fwd_weight_0( position_2 : vec2<f32>,  dpg_0 : DiffPair_Geometry_0) -> DiffPair_float_0
{
    var _S291 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( position_2 * dpg_0.primal_0.scale_4 - dpg_0.primal_0.center_1, dpg_0.differential_0.scale_4 * position_2 - dpg_0.differential_0.center_1 );
    var _S292 : DiffPair_vectorx3Cfloatx2C2x3E_0 = _d_abs_vector_1(_S291);
    var _S293 : DiffPair_float_0 = DiffPair_float_0( _S292.primal_0.x, _S292.differential_0.x );
    var _S294 : DiffPair_float_0 = DiffPair_float_0( _S292.primal_0.y, _S292.differential_0.y );
    var _S295 : DiffPair_float_0 = _d_max_1(_S293, _S294);
    var _S296 : DiffPair_float_0 = _d_min_1(_S293, _S294);
    var _S297 : DiffPair_float_0 = DiffPair_float_0( _S295.primal_0, _S295.differential_0 );
    var _S298 : DiffPair_float_0 = DiffPair_float_0( 1.00000000317107685e-30f, 0.0f );
    var _S299 : DiffPair_float_0 = _d_max_1(_S297, _S298);
    var _S300 : DiffPair_float_0 = DiffPair_float_0( _S296.primal_0 / _S299.primal_0, (_S296.differential_0 * _S299.primal_0 - _S296.primal_0 * _S299.differential_0) / (_S299.primal_0 * _S299.primal_0) );
    var _S301 : DiffPair_float_0 = DiffPair_float_0( dpg_0.primal_0.exponent_0, dpg_0.differential_0.exponent_0 );
    var _S302 : DiffPair_float_0 = pow0_forward_0(_S300, _S301);
    var _S303 : DiffPair_float_0 = DiffPair_float_0( 1.0f + _S302.primal_0, _S302.differential_0 );
    var _S304 : DiffPair_float_0 = DiffPair_float_0( dpg_0.primal_0.root_0, dpg_0.differential_0.root_0 );
    var _S305 : DiffPair_float_0 = pow0_forward_0(_S303, _S304);
    var _S306 : f32 = _S295.primal_0 * _S305.primal_0 - dpg_0.primal_0.inner_0;
    var _S307 : DiffPair_float_0 = DiffPair_float_0( _S306 / dpg_0.primal_0.falloff_0, ((_S295.differential_0 * _S305.primal_0 + _S305.differential_0 * _S295.primal_0 - dpg_0.differential_0.inner_0) * dpg_0.primal_0.falloff_0 - _S306 * dpg_0.differential_0.falloff_0) / (dpg_0.primal_0.falloff_0 * dpg_0.primal_0.falloff_0) );
    var _S308 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S309 : DiffPair_float_0 = DiffPair_float_0( 1.0f, 0.0f );
    var _S310 : DiffPair_float_0 = _d_clamp_1(_S307, _S308, _S309);
    var _S311 : DiffPair_float_0 = DiffPair_float_0( _S310.primal_0, _S310.differential_0 );
    return _S311;
}

fn shade_0( rgb_0 : vec3<f32>,  w_0 : f32,  brightness_2 : f32,  saturation_2 : f32,  unbound_2 : bool) -> vec3<f32>
{
    var col_2 : vec3<f32>;
    if((brightness_2 < 0.0f))
    {
        col_2 = rgb_0 * vec3<f32>((1.0f + w_0 * brightness_2));
    }
    else
    {
        col_2 = rgb_0 + vec3<f32>((w_0 * brightness_2));
    }
    var _S312 : bool = !unbound_2;
    if(_S312)
    {
        col_2 = clamp(col_2, vec3<f32>(0.0f), vec3<f32>(1.0f));
    }
    var col_3 : vec3<f32> = col_2 - (vec3<f32>(((col_2.x + col_2.y + col_2.z) / 3.0f)) - col_2) * vec3<f32>((w_0 * saturation_2));
    if(_S312)
    {
        col_2 = clamp(col_3, vec3<f32>(0.0f), vec3<f32>(1.0f));
    }
    else
    {
        col_2 = col_3;
    }
    return col_2;
}

fn s_fwd_shade_0( rgb_1 : vec3<f32>,  dpw_0 : DiffPair_float_0,  dpbrightness_0 : DiffPair_float_0,  dpsaturation_0 : DiffPair_float_0,  unbound_3 : bool) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var col_4 : vec3<f32>;
    var s_diff_col_0 : vec3<f32>;
    if(((dpbrightness_0.primal_0) < 0.0f))
    {
        var _S313 : vec3<f32> = vec3<f32>((dpw_0.differential_0 * dpbrightness_0.primal_0 + dpbrightness_0.differential_0 * dpw_0.primal_0)) * rgb_1;
        col_4 = rgb_1 * vec3<f32>((1.0f + dpw_0.primal_0 * dpbrightness_0.primal_0));
        s_diff_col_0 = _S313;
    }
    else
    {
        var _S314 : vec3<f32> = vec3<f32>((dpw_0.differential_0 * dpbrightness_0.primal_0 + dpbrightness_0.differential_0 * dpw_0.primal_0));
        col_4 = rgb_1 + vec3<f32>((dpw_0.primal_0 * dpbrightness_0.primal_0));
        s_diff_col_0 = _S314;
    }
    var _S315 : bool = !unbound_3;
    if(_S315)
    {
        var _S316 : vec3<f32> = vec3<f32>(0.0f);
        var _S317 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( col_4, s_diff_col_0 );
        var _S318 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.0f), _S316 );
        var _S319 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(1.0f), _S316 );
        var _S320 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_clamp_vector_1(_S317, _S318, _S319);
        col_4 = _S320.primal_0;
        s_diff_col_0 = _S320.differential_0;
    }
    var _S321 : vec3<f32> = vec3<f32>(((col_4.x + col_4.y + col_4.z) / 3.0f)) - col_4;
    var _S322 : vec3<f32> = vec3<f32>((dpw_0.primal_0 * dpsaturation_0.primal_0));
    var col_5 : vec3<f32> = col_4 - _S321 * _S322;
    var s_diff_col_1 : vec3<f32> = s_diff_col_0 - ((vec3<f32>(((s_diff_col_0.x + s_diff_col_0.y + s_diff_col_0.z) * 0.3333333432674408f)) - s_diff_col_0) * _S322 + vec3<f32>((dpw_0.differential_0 * dpsaturation_0.primal_0 + dpsaturation_0.differential_0 * dpw_0.primal_0)) * _S321);
    if(_S315)
    {
        var _S323 : vec3<f32> = vec3<f32>(0.0f);
        var _S324 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( col_5, s_diff_col_1 );
        var _S325 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.0f), _S323 );
        var _S326 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(1.0f), _S323 );
        var _S327 : DiffPair_vectorx3Cfloatx2C3x3E_0 = _d_clamp_vector_1(_S324, _S325, _S326);
        col_4 = _S327.primal_0;
        s_diff_col_0 = _S327.differential_0;
    }
    else
    {
        col_4 = col_5;
        s_diff_col_0 = s_diff_col_1;
    }
    var _S328 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( col_4, s_diff_col_0 );
    return _S328;
}

fn vignette_1( rgb_2 : vec3<f32>,  position_3 : vec2<f32>,  vignetted_0 : bool,  g_2 : Geometry_0,  brightness_3 : f32,  saturation_3 : f32,  unbound_4 : bool) -> vec3<f32>
{
    if(!vignetted_0)
    {
        return rgb_2;
    }
    return shade_0(rgb_2, weight_0(position_3, g_2), brightness_3, saturation_3, unbound_4);
}

fn s_fwd_vignette_0( rgb_3 : vec3<f32>,  position_4 : vec2<f32>,  vignetted_1 : bool,  dpg_1 : DiffPair_Geometry_0,  dpbrightness_1 : DiffPair_float_0,  dpsaturation_1 : DiffPair_float_0,  unbound_5 : bool) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    if(!vignetted_1)
    {
        var _S329 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( rgb_3, vec3<f32>(0.0f) );
        return _S329;
    }
    var _S330 : DiffPair_Geometry_0 = DiffPair_Geometry_0( dpg_1.primal_0, dpg_1.differential_0 );
    var _S331 : DiffPair_float_0 = s_fwd_weight_0(position_4, _S330);
    var _S332 : DiffPair_float_0 = DiffPair_float_0( _S331.primal_0, _S331.differential_0 );
    var _S333 : DiffPair_float_0 = DiffPair_float_0( dpbrightness_1.primal_0, dpbrightness_1.differential_0 );
    var _S334 : DiffPair_float_0 = DiffPair_float_0( dpsaturation_1.primal_0, dpsaturation_1.differential_0 );
    var _S335 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_shade_0(rgb_3, _S332, _S333, _S334, unbound_5);
    var _S336 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S335.primal_0, _S335.differential_0 );
    return _S336;
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_1 : vec3<u32>)
{
    var _S337 : u32 = id_1.x;
    var _S338 : bool;
    if((_S337 >= (params_0.pass_0.width_0)))
    {
        _S338 = true;
    }
    else
    {
        _S338 = ((id_1.y) >= (params_0.pass_0.height_0));
    }
    if(_S338)
    {
        return;
    }
    var i_0 : u32 = id_1.y * params_0.pass_0.width_0 + _S337;
    var _S339 : VignetteParams_std140_0 = params_0.vignette_0;
    var _S340 : Vignette_0 = vignette_of_0(&(_S339));
    var _S341 : Params_std140_0 = params_0;
    var _S342 : Frame_0 = frame_of_0(&(_S341));
    var g_3 : Geometry_0 = geometry_0(_S340, _S342);
    var position_5 : vec2<f32> = position_of_0(id_1);
    var _S343 : bool = ((weight_0(position_5, g_3)) > 0.0f);
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
        var _S344 : vec3<f32> = vignette_1(rgb_4, position_5, _S343, g_3, _S340.brightness_1, _S340.saturation_1, _S342.unbound_1);
        var _S345 : u32 = k_0 + u32(1);
        rgb_4 = _S344;
        k_0 = _S345;
    }
    output_0[i_0] = vec4<f32>(rgb_4, pixel_0.w);
    return;
}

fn s_fwd_vignette_from_settings_0( rgb_5 : vec3<f32>,  position_6 : vec2<f32>,  vignetted_2 : bool,  dpv_1 : DiffPair_Vignette_0,  f_4 : Frame_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S346 : DiffPair_Vignette_0 = DiffPair_Vignette_0( dpv_1.primal_0, dpv_1.differential_0 );
    var _S347 : DiffPair_Geometry_0 = s_fwd_geometry_0(_S346, f_4);
    var _S348 : DiffPair_Geometry_0 = DiffPair_Geometry_0( _S347.primal_0, _S347.differential_0 );
    var _S349 : DiffPair_float_0 = DiffPair_float_0( dpv_1.primal_0.brightness_1, dpv_1.differential_0.brightness_1 );
    var _S350 : DiffPair_float_0 = DiffPair_float_0( dpv_1.primal_0.saturation_1, dpv_1.differential_0.saturation_1 );
    var _S351 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_vignette_0(rgb_5, position_6, vignetted_2, _S348, _S349, _S350, f_4.unbound_1);
    var _S352 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S351.primal_0, _S351.differential_0 );
    return _S352;
}

fn pick_0( d_0 : Vignette_0,  which_0 : u32) -> f32
{
    switch(which_0)
    {
    case u32(0):
        {
            return d_0.scale_2;
        }
    case u32(1):
        {
            return d_0.falloff_scale_1;
        }
    case u32(2):
        {
            return d_0.brightness_1;
        }
    case u32(3):
        {
            return d_0.saturation_1;
        }
    case u32(4):
        {
            return d_0.center_0.x;
        }
    case u32(5):
        {
            return d_0.center_0.y;
        }
    case u32(6):
        {
            return d_0.whratio_1;
        }
    case u32(7):
        {
            return d_0.shape_1;
        }
    default :
        {
            return 0.0f;
        }
    }
}

@compute
@workgroup_size(16, 16, 1)
fn vignette_backward(@builtin(global_invocation_id) id_2 : vec3<u32>)
{
    var _S353 : u32 = id_2.x;
    var _S354 : bool;
    if((_S353 >= (params_0.pass_0.width_0)))
    {
        _S354 = true;
    }
    else
    {
        _S354 = ((id_2.y) >= (params_0.pass_0.height_0));
    }
    if(_S354)
    {
        return;
    }
    var i_1 : u32 = id_2.y * params_0.pass_0.width_0 + _S353;
    var _S355 : VignetteParams_std140_0 = params_0.vignette_0;
    var _S356 : Vignette_0 = vignette_of_0(&(_S355));
    var _S357 : Params_std140_0 = params_0;
    var _S358 : Frame_0 = frame_of_0(&(_S357));
    var position_7 : vec2<f32> = position_of_0(id_2);
    var vignetted_3 : bool = ((weight_0(position_7, geometry_0(_S356, _S358))) > 0.0f);
    var _S359 : Vignette_0 = Vignette_x24_syn_dzero_0();
    var d_1 : DiffPair_Vignette_0;
    d_1.primal_0 = _S356;
    d_1.differential_0 = _S359;
    _S230(input_0[i_1].xyz, position_7, vignetted_3, &(d_1), _S358, derivative_in_0[i_1].xyz);
    derivative_out_0[i_1] = vec4<f32>(pick_0(d_1.differential_0, _S355.gradient_0.x), pick_0(d_1.differential_0, _S355.gradient_0.y), pick_0(d_1.differential_0, _S355.gradient_0.z), pick_0(d_1.differential_0, _S355.gradient_0.w));
    return;
}

fn add_slot_0( d_2 : ptr<function, Vignette_0>,  which_1 : u32,  x_9 : f32)
{
    var _S360 : f32;
    if(which_1 == u32(0))
    {
        _S360 = x_9;
    }
    else
    {
        _S360 = 0.0f;
    }
    (*d_2).scale_2 = (*d_2).scale_2 + _S360;
    if(which_1 == u32(1))
    {
        _S360 = x_9;
    }
    else
    {
        _S360 = 0.0f;
    }
    (*d_2).falloff_scale_1 = (*d_2).falloff_scale_1 + _S360;
    if(which_1 == u32(2))
    {
        _S360 = x_9;
    }
    else
    {
        _S360 = 0.0f;
    }
    (*d_2).brightness_1 = (*d_2).brightness_1 + _S360;
    if(which_1 == u32(3))
    {
        _S360 = x_9;
    }
    else
    {
        _S360 = 0.0f;
    }
    (*d_2).saturation_1 = (*d_2).saturation_1 + _S360;
    if(which_1 == u32(4))
    {
        _S360 = x_9;
    }
    else
    {
        _S360 = 0.0f;
    }
    (*d_2).center_0[i32(0)] = (*d_2).center_0[i32(0)] + _S360;
    if(which_1 == u32(5))
    {
        _S360 = x_9;
    }
    else
    {
        _S360 = 0.0f;
    }
    (*d_2).center_0[i32(1)] = (*d_2).center_0[i32(1)] + _S360;
    if(which_1 == u32(6))
    {
        _S360 = x_9;
    }
    else
    {
        _S360 = 0.0f;
    }
    (*d_2).whratio_1 = (*d_2).whratio_1 + _S360;
    if(which_1 == u32(7))
    {
        _S360 = x_9;
    }
    else
    {
        _S360 = 0.0f;
    }
    (*d_2).shape_1 = (*d_2).shape_1 + _S360;
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn vignette_forward_diff(@builtin(global_invocation_id) id_3 : vec3<u32>)
{
    var _S361 : u32 = id_3.x;
    var _S362 : bool;
    if((_S361 >= (params_0.pass_0.width_0)))
    {
        _S362 = true;
    }
    else
    {
        _S362 = ((id_3.y) >= (params_0.pass_0.height_0));
    }
    if(_S362)
    {
        return;
    }
    var i_2 : u32 = id_3.y * params_0.pass_0.width_0 + _S361;
    var _S363 : VignetteParams_std140_0 = params_0.vignette_0;
    var _S364 : Vignette_0 = vignette_of_0(&(_S363));
    var _S365 : Params_std140_0 = params_0;
    var _S366 : Frame_0 = frame_of_0(&(_S365));
    var position_8 : vec2<f32> = position_of_0(id_3);
    var vignetted_4 : bool = ((weight_0(position_8, geometry_0(_S364, _S366))) > 0.0f);
    var direction_in_0 : vec4<f32> = derivative_in_0[i_2];
    var direction_0 : Vignette_0;
    direction_0.scale_2 = 0.0f;
    direction_0.falloff_scale_1 = 0.0f;
    direction_0.brightness_1 = 0.0f;
    direction_0.saturation_1 = 0.0f;
    direction_0.center_0 = vec2<f32>(0.0f);
    direction_0.whratio_1 = 0.0f;
    direction_0.shape_1 = 0.0f;
    add_slot_0(&(direction_0), _S363.gradient_0.x, direction_in_0.x);
    add_slot_0(&(direction_0), _S363.gradient_0.y, direction_in_0.y);
    add_slot_0(&(direction_0), _S363.gradient_0.z, direction_in_0.z);
    add_slot_0(&(direction_0), _S363.gradient_0.w, direction_in_0.w);
    var _S367 : DiffPair_Vignette_0 = DiffPair_Vignette_0( _S364, direction_0 );
    derivative_out_0[i_2] = vec4<f32>(s_fwd_vignette_from_settings_0(input_0[i_2].xyz, position_8, vignetted_4, _S367, _S366).differential_0, 0.0f);
    return;
}

