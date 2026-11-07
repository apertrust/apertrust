// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2012-2026  darktable developers (ported from src/iop/bilat.c,
// src/common/locallaplacian.c, src/common/bilateral.c and src/common/math.h)
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

// Generated from slang/bilat.slang by tools/build-kernels.sh. Do not edit.

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

struct BilatParams_std140_0
{
    @align(16) image_0 : vec4<u32>,
    @align(16) fine_0 : vec4<u32>,
    @align(16) coarse_0 : vec4<u32>,
    @align(16) curve_0 : vec4<f32>,
    @align(16) grid_0 : vec4<f32>,
    @align(16) grid_size_0 : vec4<u32>,
    @align(16) slice_0 : vec4<f32>,
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
    @align(16) bilat_0 : BilatParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(0) @group(0) var<storage, read> input_0 : array<vec4<f32>>;

@binding(11) @group(0) var<storage, read_write> coarse_1 : array<f32>;

@binding(10) @group(0) var<storage, read_write> fine_1 : array<f32>;

@binding(1) @group(0) var<storage, read_write> output_0 : array<vec4<f32>>;

@binding(8) @group(0) var<storage, read> derivative_in_0 : array<vec4<f32>>;

@binding(9) @group(0) var<storage, read_write> derivative_out_0 : array<vec4<f32>>;

struct s_bwdCallableCtx_mul_0
{
     _S1 : mat3x3<f32>,
     _S2 : vec3<f32>,
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

fn _d_mul_0( left_0 : ptr<function, DiffPair_matrixx3Cfloatx2C3x2C3x3E_0>,  right_0 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_0 : vec3<f32>)
{
    var _S3 : f32 = (*left_0).primal_0[i32(0)][i32(0)] * dOut_0[i32(0)];
    var left_d_result_0 : mat3x3<f32>;
    left_d_result_0[i32(0)][i32(0)] = (*right_0).primal_0[i32(0)] * dOut_0[i32(0)];
    var sum_0 : f32 = _S3 + (*left_0).primal_0[i32(1)][i32(0)] * dOut_0[i32(1)];
    left_d_result_0[i32(1)][i32(0)] = (*right_0).primal_0[i32(0)] * dOut_0[i32(1)];
    var sum_1 : f32 = sum_0 + (*left_0).primal_0[i32(2)][i32(0)] * dOut_0[i32(2)];
    left_d_result_0[i32(2)][i32(0)] = (*right_0).primal_0[i32(0)] * dOut_0[i32(2)];
    var right_d_result_0 : vec3<f32>;
    right_d_result_0[i32(0)] = sum_1;
    var _S4 : f32 = (*left_0).primal_0[i32(0)][i32(1)] * dOut_0[i32(0)];
    left_d_result_0[i32(0)][i32(1)] = (*right_0).primal_0[i32(1)] * dOut_0[i32(0)];
    var sum_2 : f32 = _S4 + (*left_0).primal_0[i32(1)][i32(1)] * dOut_0[i32(1)];
    left_d_result_0[i32(1)][i32(1)] = (*right_0).primal_0[i32(1)] * dOut_0[i32(1)];
    var sum_3 : f32 = sum_2 + (*left_0).primal_0[i32(2)][i32(1)] * dOut_0[i32(2)];
    left_d_result_0[i32(2)][i32(1)] = (*right_0).primal_0[i32(1)] * dOut_0[i32(2)];
    right_d_result_0[i32(1)] = sum_3;
    var _S5 : f32 = (*left_0).primal_0[i32(0)][i32(2)] * dOut_0[i32(0)];
    left_d_result_0[i32(0)][i32(2)] = (*right_0).primal_0[i32(2)] * dOut_0[i32(0)];
    var sum_4 : f32 = _S5 + (*left_0).primal_0[i32(1)][i32(2)] * dOut_0[i32(1)];
    left_d_result_0[i32(1)][i32(2)] = (*right_0).primal_0[i32(2)] * dOut_0[i32(1)];
    var sum_5 : f32 = sum_4 + (*left_0).primal_0[i32(2)][i32(2)] * dOut_0[i32(2)];
    left_d_result_0[i32(2)][i32(2)] = (*right_0).primal_0[i32(2)] * dOut_0[i32(2)];
    right_d_result_0[i32(2)] = sum_5;
    (*left_0).primal_0 = (*left_0).primal_0;
    (*left_0).differential_0 = left_d_result_0;
    (*right_0).primal_0 = (*right_0).primal_0;
    (*right_0).differential_0 = right_d_result_0;
    return;
}

fn s_bwdProp_mul_0( ctx_0 : s_bwdCallableCtx_mul_0,  d_left_0 : ptr<function, mat3x3<f32>>,  d_right_0 : ptr<function, vec3<f32>>,  d_out_0 : vec3<f32>)
{
    var _S6 : DiffPair_matrixx3Cfloatx2C3x2C3x3E_0;
    _S6.primal_0 = ctx_0._S1;
    var _S7 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S7.primal_0 = ctx_0._S2;
    _d_mul_0(&(_S6), &(_S7), d_out_0);
    (*d_left_0) = _S6.differential_0;
    (*d_right_0) = _S7.differential_0;
    return;
}

struct Tuple_0
{
     value0_0 : f32,
};

fn fast_exp_0( x_0 : f32) -> f32
{
    var k_0 : f32 = fma(x_0, 1.14013e+07f, 1.065353216e+09f);
    if((k_0 <= 0.0f))
    {
        return 0.0f;
    }
    return (bitcast<f32>((i32(min(k_0, 2.130706432e+09f)))));
}

fn s_apply_fast_exp_0( x_1 : f32) -> Tuple_0
{
    var _S8 : Tuple_0 = Tuple_0( fast_exp_0(x_1) );
    return _S8;
}

struct s_bwdCallableCtx_fast_exp_0
{
     _S9 : f32,
};

struct DiffPair_float_0
{
     primal_0 : f32,
     differential_0 : f32,
};

fn fast_exp_slope_0( value_0 : f32) -> f32
{
    if((value_0 <= 0.0f))
    {
        return 0.0f;
    }
    return (bitcast<f32>((((bitcast<u32>((value_0))) & (u32(2139095040)))))) * 1.3591408729553223f;
}

fn fast_exp_backward_0( x_2 : ptr<function, DiffPair_float_0>,  d_out_1 : f32)
{
    var _S10 : f32 = fast_exp_slope_0(fast_exp_0((*x_2).primal_0)) * d_out_1;
    (*x_2).primal_0 = (*x_2).primal_0;
    (*x_2).differential_0 = _S10;
    return;
}

fn s_bwdProp_fast_exp_0( ctx_1 : s_bwdCallableCtx_fast_exp_0,  d_x_0 : ptr<function, f32>,  d_out_2 : f32)
{
    var _S11 : DiffPair_float_0;
    _S11.primal_0 = ctx_1._S9;
    fast_exp_backward_0(&(_S11), d_out_2);
    (*d_x_0) = _S11.differential_0;
    return;
}

struct Tuple_1
{
     value0_1 : f32,
};

fn s_apply_clamp_0( x_3 : f32,  minBound_0 : f32,  maxBound_0 : f32) -> Tuple_1
{
    var _S12 : Tuple_1 = Tuple_1( clamp(x_3, minBound_0, maxBound_0) );
    return _S12;
}

struct s_bwdCallableCtx_clamp_0
{
     _S13 : f32,
     _S14 : f32,
     _S15 : f32,
};

fn _d_clamp_0( dpx_0 : ptr<function, DiffPair_float_0>,  dpMin_0 : ptr<function, DiffPair_float_0>,  dpMax_0 : ptr<function, DiffPair_float_0>,  dOut_1 : f32)
{
    var _S16 : DiffPair_float_0 = (*dpx_0);
    var _S17 : bool;
    if((((*dpx_0).primal_0) >= ((*dpMin_0).primal_0)))
    {
        _S17 = (((*dpx_0).primal_0) <= ((*dpMax_0).primal_0));
    }
    else
    {
        _S17 = false;
    }
    var _S18 : f32;
    if(_S17)
    {
        _S18 = dOut_1;
    }
    else
    {
        _S18 = 0.0f;
    }
    (*dpx_0).primal_0 = _S16.primal_0;
    (*dpx_0).differential_0 = _S18;
    var _S19 : DiffPair_float_0 = (*dpMin_0);
    if(((_S16.primal_0) < ((*dpMin_0).primal_0)))
    {
        _S18 = dOut_1;
    }
    else
    {
        _S18 = 0.0f;
    }
    (*dpMin_0).primal_0 = _S19.primal_0;
    (*dpMin_0).differential_0 = _S18;
    var _S20 : DiffPair_float_0 = (*dpMax_0);
    if((((*dpx_0).primal_0) > ((*dpMax_0).primal_0)))
    {
        _S18 = dOut_1;
    }
    else
    {
        _S18 = 0.0f;
    }
    (*dpMax_0).primal_0 = _S20.primal_0;
    (*dpMax_0).differential_0 = _S18;
    return;
}

fn s_bwdProp_clamp_0( ctx_2 : s_bwdCallableCtx_clamp_0,  d_x_1 : ptr<function, f32>,  d_minBound_0 : ptr<function, f32>,  d_maxBound_0 : ptr<function, f32>,  d_out_3 : f32)
{
    var _S21 : DiffPair_float_0;
    _S21.primal_0 = ctx_2._S13;
    var _S22 : DiffPair_float_0;
    _S22.primal_0 = ctx_2._S14;
    var _S23 : DiffPair_float_0;
    _S23.primal_0 = ctx_2._S15;
    _d_clamp_0(&(_S21), &(_S22), &(_S23), d_out_3);
    (*d_x_1) = _S21.differential_0;
    (*d_minBound_0) = _S22.differential_0;
    (*d_maxBound_0) = _S23.differential_0;
    return;
}

struct Tuple_2
{
     value0_2 : f32,
};

fn s_apply_max_0( x_4 : f32,  y_0 : f32) -> Tuple_2
{
    var _S24 : Tuple_2 = Tuple_2( max(x_4, y_0) );
    return _S24;
}

struct s_bwdCallableCtx_max_0
{
     _S25 : f32,
     _S26 : f32,
};

fn _d_max_0( dpx_1 : ptr<function, DiffPair_float_0>,  dpy_0 : ptr<function, DiffPair_float_0>,  dOut_2 : f32)
{
    var _S27 : DiffPair_float_0 = (*dpx_1);
    var _S28 : f32;
    if((((*dpx_1).primal_0) > ((*dpy_0).primal_0)))
    {
        _S28 = dOut_2;
    }
    else
    {
        if((((*dpx_1).primal_0) < ((*dpy_0).primal_0)))
        {
            _S28 = 0.0f;
        }
        else
        {
            _S28 = 0.5f * dOut_2;
        }
    }
    (*dpx_1).primal_0 = _S27.primal_0;
    (*dpx_1).differential_0 = _S28;
    var _S29 : DiffPair_float_0 = (*dpy_0);
    if((((*dpy_0).primal_0) > (_S27.primal_0)))
    {
        _S28 = dOut_2;
    }
    else
    {
        if((((*dpy_0).primal_0) < ((*dpx_1).primal_0)))
        {
            _S28 = 0.0f;
        }
        else
        {
            _S28 = 0.5f * dOut_2;
        }
    }
    (*dpy_0).primal_0 = _S29.primal_0;
    (*dpy_0).differential_0 = _S28;
    return;
}

fn s_bwdProp_max_0( ctx_3 : s_bwdCallableCtx_max_0,  d_x_2 : ptr<function, f32>,  d_y_0 : ptr<function, f32>,  d_out_4 : f32)
{
    var _S30 : DiffPair_float_0;
    _S30.primal_0 = ctx_3._S25;
    var _S31 : DiffPair_float_0;
    _S31.primal_0 = ctx_3._S26;
    _d_max_0(&(_S30), &(_S31), d_out_4);
    (*d_x_2) = _S30.differential_0;
    (*d_y_0) = _S31.differential_0;
    return;
}

struct Neighbourhood_0
{
     mode_0 : u32,
     lo_0 : u32,
     a_0 : f32,
     output_1 : f32,
     lower_0 : f32,
     upper_0 : f32,
     grid_1 : f32,
     sigma_r_0 : f32,
};

struct Colour_0
{
     rgb_to_xyz_0 : mat3x3<f32>,
     xyz_to_rgb_0 : mat3x3<f32>,
};

struct Settings_0
{
     detail_0 : f32,
     shadows_0 : f32,
     highlights_0 : f32,
     midtone_0 : f32,
};

struct DiffPair_Settings_0
{
     primal_0 : Settings_0,
     differential_0 : Settings_0,
};

struct s_paramCtx_s_bwdCallableCtx_bilat_0
{
     value0_3 : vec3<f32>,
     value1_0 : Neighbourhood_0,
     value2_0 : Colour_0,
     value3_0 : Settings_0,
};

struct s_bwdCallableCtx_bilat_0
{
     value0_4 : s_paramCtx_s_bwdCallableCtx_bilat_0,
};

struct s_paramCtx_s_bwdCallableCtx_curve_0
{
     value0_5 : f32,
     value1_1 : f32,
     value2_1 : Settings_0,
};

struct s_bwdCallableCtx_curve_0
{
     value0_6 : s_paramCtx_s_bwdCallableCtx_curve_0,
};

fn gamma_of_0( k_1 : u32) -> f32
{
    return (f32(k_1) + 0.5f) / 6.0f;
}

struct Tuple_3
{
     value0_7 : f32,
};

fn s_apply_curve_0( x_5 : f32,  g_0 : f32,  _S32 : Settings_0) -> Tuple_3
{
    var c_0 : f32 = x_5 - g_0;
    var _S33 : f32 = 2.0f * _S32.midtone_0;
    var val_0 : f32;
    if((c_0 > _S33))
    {
        val_0 = g_0 + _S32.midtone_0 + _S32.shadows_0 * (c_0 - _S32.midtone_0);
    }
    else
    {
        if((c_0 < (-2.0f * _S32.midtone_0)))
        {
            val_0 = g_0 - _S32.midtone_0 + _S32.highlights_0 * (c_0 + _S32.midtone_0);
        }
        else
        {
            if((c_0 > 0.0f))
            {
                var _S34 : Tuple_1 = s_apply_clamp_0(c_0 / _S33, 0.0f, 1.0f);
                var _S35 : f32 = _S34.value0_1;
                val_0 = g_0 + _S32.midtone_0 * 2.0f * (1.0f - _S34.value0_1) * _S34.value0_1 + _S35 * _S35 * (_S32.midtone_0 + _S32.midtone_0 * _S32.shadows_0);
            }
            else
            {
                var _S36 : Tuple_1 = s_apply_clamp_0(- c_0 / _S33, 0.0f, 1.0f);
                var _S37 : f32 = _S36.value0_1;
                val_0 = g_0 - _S32.midtone_0 * 2.0f * (1.0f - _S36.value0_1) * _S36.value0_1 + _S37 * _S37 * (- _S32.midtone_0 - _S32.midtone_0 * _S32.highlights_0);
            }
        }
    }
    var _S38 : Tuple_3;
    _S38.value0_7 = val_0 + _S32.detail_0 * c_0 * s_apply_fast_exp_0(- c_0 * c_0 / (_S33 * _S32.midtone_0 / 3.0f)).value0_0;
    return _S38;
}

struct s_paramCtx_s_bwdCallableCtx_lab_to_rgb_0
{
     value0_8 : vec3<f32>,
     value1_2 : Colour_0,
};

struct s_bwdCallableCtx_lab_to_rgb_0
{
     value0_9 : s_paramCtx_s_bwdCallableCtx_lab_to_rgb_0,
};

fn Settings_x24_syn_dzero_0() -> Settings_0
{
    var result_0 : Settings_0;
    result_0.detail_0 = 0.0f;
    result_0.shadows_0 = 0.0f;
    result_0.highlights_0 = 0.0f;
    result_0.midtone_0 = 0.0f;
    return result_0;
}

struct Tuple_4
{
     value0_10 : vec3<f32>,
};

struct Tuple_5
{
     value0_11 : f32,
};

fn s_apply_lab_f_inv_0( _S39 : f32) -> Tuple_5
{
    var _S40 : bool = (_S39 > 0.20689655840396881f);
    var _S41 : f32;
    if(_S40)
    {
        _S41 = _S39 * _S39 * _S39;
    }
    var _S42 : bool = !_S40;
    if(_S42)
    {
        _S41 = (116.0f * _S39 - 16.0f) / 903.29632568359375f;
    }
    var _S43 : Tuple_5;
    _S43.value0_11 = _S41;
    return _S43;
}

fn s_apply_lab_to_xyz_d50_0( _S44 : vec3<f32>) -> Tuple_4
{
    var fy_0 : f32 = (_S44.x + 16.0f) * 0.0086206896230578423f;
    var _S45 : Tuple_4;
    _S45.value0_10 = vec3<f32>(0.96420001983642578f, 1.0f, 0.82489997148513794f) * vec3<f32>(s_apply_lab_f_inv_0(_S44.y * 0.0020000000949949026f + fy_0).value0_11, s_apply_lab_f_inv_0(fy_0).value0_11, s_apply_lab_f_inv_0(_S44.z * -0.004999999888241291f + fy_0).value0_11);
    return _S45;
}

struct s_paramCtx_s_bwdCallableCtx_lab_to_xyz_d50_0
{
     value0_12 : vec3<f32>,
};

struct s_bwdCallableCtx_lab_to_xyz_d50_0
{
     value0_13 : s_paramCtx_s_bwdCallableCtx_lab_to_xyz_d50_0,
};

struct s_paramCtx_s_bwdCallableCtx_lab_f_inv_0
{
     value0_14 : f32,
};

struct s_bwdCallableCtx_lab_f_inv_0
{
     value0_15 : s_paramCtx_s_bwdCallableCtx_lab_f_inv_0,
};

fn s_bwdProp_lab_f_inv_0( _S46 : s_bwdCallableCtx_lab_f_inv_0,  _S47 : ptr<function, f32>,  _S48 : f32)
{
    var _S49 : bool = ((_S46.value0_15.value0_14) > 0.20689655840396881f);
    var _S50 : f32;
    if(_S49)
    {
        _S50 = _S46.value0_15.value0_14 * _S46.value0_15.value0_14;
    }
    else
    {
        _S50 = 0.0f;
    }
    var _S51 : bool = !_S49;
    var _S52 : f32;
    var _S53 : f32;
    if(_S51)
    {
        var _S54 : f32 = 116.0f * (0.0011070563923567533f * _S48);
        _S52 = 0.0f;
        _S53 = _S54;
    }
    else
    {
        _S52 = _S48;
        _S53 = 0.0f;
    }
    if(_S49)
    {
        var _S55 : f32 = _S46.value0_15.value0_14 * (_S46.value0_15.value0_14 * _S52);
        _S50 = _S50 * _S52 + _S55 + _S55 + _S53;
    }
    else
    {
        _S50 = _S53;
    }
    (*_S47) = _S50;
    return;
}

fn s_bwdProp_lab_to_xyz_d50_0( _S56 : s_bwdCallableCtx_lab_to_xyz_d50_0,  _S57 : ptr<function, vec3<f32>>,  _S58 : vec3<f32>)
{
    var fy_1 : f32 = (_S56.value0_13.value0_12.x + 16.0f) * 0.0086206896230578423f;
    var fz_0 : f32 = _S56.value0_13.value0_12.z * -0.004999999888241291f + fy_1;
    var _S59 : s_paramCtx_s_bwdCallableCtx_lab_f_inv_0;
    _S59.value0_14 = _S56.value0_13.value0_12.y * 0.0020000000949949026f + fy_1;
    var _S60 : s_bwdCallableCtx_lab_f_inv_0 = s_bwdCallableCtx_lab_f_inv_0( _S59 );
    var _S61 : s_paramCtx_s_bwdCallableCtx_lab_f_inv_0;
    _S61.value0_14 = fy_1;
    var _S62 : s_bwdCallableCtx_lab_f_inv_0 = s_bwdCallableCtx_lab_f_inv_0( _S61 );
    var _S63 : s_paramCtx_s_bwdCallableCtx_lab_f_inv_0;
    _S63.value0_14 = fz_0;
    var _S64 : s_bwdCallableCtx_lab_f_inv_0 = s_bwdCallableCtx_lab_f_inv_0( _S63 );
    var _S65 : vec3<f32> = vec3<f32>(0.96420001983642578f, 1.0f, 0.82489997148513794f) * _S58;
    var _S66 : f32 = 0.0f;
    s_bwdProp_lab_f_inv_0(_S64, &(_S66), _S65[i32(2)]);
    var _S67 : f32 = 0.0f;
    s_bwdProp_lab_f_inv_0(_S62, &(_S67), _S65[i32(1)]);
    var _S68 : f32 = 0.0f;
    s_bwdProp_lab_f_inv_0(_S60, &(_S68), _S65[i32(0)]);
    (*_S57) = vec3<f32>(0.0086206896230578423f * (_S67 + _S66 + _S68), 0.0020000000949949026f * _S68, -0.004999999888241291f * _S66);
    return;
}

fn s_bwdProp_lab_to_rgb_0( _S69 : s_bwdCallableCtx_lab_to_rgb_0,  _S70 : ptr<function, vec3<f32>>,  _S71 : vec3<f32>)
{
    var _S72 : Tuple_4 = s_apply_lab_to_xyz_d50_0(_S69.value0_9.value0_8);
    var _S73 : s_paramCtx_s_bwdCallableCtx_lab_to_xyz_d50_0;
    _S73.value0_12 = _S69.value0_9.value0_8;
    var _S74 : s_bwdCallableCtx_lab_to_xyz_d50_0 = s_bwdCallableCtx_lab_to_xyz_d50_0( _S73 );
    var _S75 : s_bwdCallableCtx_mul_0;
    _S75._S1 = _S69.value0_9.value1_2.xyz_to_rgb_0;
    _S75._S2 = _S72.value0_10;
    var _S76 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S77 : vec3<f32> = vec3<f32>(0.0f);
    var _S78 : vec3<f32> = _S77;
    s_bwdProp_mul_0(_S75, &(_S76), &(_S78), _S71);
    var _S79 : vec3<f32> = _S77;
    s_bwdProp_lab_to_xyz_d50_0(_S74, &(_S79), _S78);
    (*_S70) = _S79;
    return;
}

fn Settings_x24_syn_dadd_0( SLANG_anonymous_0_0 : Settings_0,  SLANG_anonymous_1_0 : Settings_0) -> Settings_0
{
    var result_1 : Settings_0;
    result_1.detail_0 = SLANG_anonymous_0_0.detail_0 + SLANG_anonymous_1_0.detail_0;
    result_1.shadows_0 = SLANG_anonymous_0_0.shadows_0 + SLANG_anonymous_1_0.shadows_0;
    result_1.highlights_0 = SLANG_anonymous_0_0.highlights_0 + SLANG_anonymous_1_0.highlights_0;
    result_1.midtone_0 = SLANG_anonymous_0_0.midtone_0 + SLANG_anonymous_1_0.midtone_0;
    return result_1;
}

fn s_bwdProp_curve_0( _S80 : s_bwdCallableCtx_curve_0,  _S81 : ptr<function, Settings_0>,  s_diff_val_T_0 : f32)
{
    var c_1 : f32 = _S80.value0_6.value0_5 - _S80.value0_6.value1_1;
    var _S82 : f32 = 2.0f * _S80.value0_6.value2_1.midtone_0;
    var _S83 : bool = (c_1 > _S82);
    var _S84 : f32;
    var _S85 : f32;
    var _S86 : f32;
    var _S87 : f32;
    var _S88 : f32;
    var _S89 : f32;
    var _S90 : f32;
    var _S91 : f32;
    var _S92 : f32;
    var _S93 : f32;
    var _S94 : f32;
    var _S95 : f32;
    var _S96 : f32;
    var _S97 : f32;
    var _S98 : f32;
    var _S99 : f32;
    var _S100 : f32;
    var _S101 : f32;
    var _S102 : f32;
    var _S103 : f32;
    var _S104 : f32;
    var _S105 : s_bwdCallableCtx_clamp_0;
    var _S106 : s_bwdCallableCtx_clamp_0;
    var _S107 : bool;
    var _S108 : bool;
    if(_S83)
    {
        var _S109 : f32 = c_1 - _S80.value0_6.value2_1.midtone_0;
        _S84 = _S80.value0_6.value2_1.shadows_0;
        _S85 = _S109;
        _S107 = false;
        _S86 = 0.0f;
        _S87 = 0.0f;
        _S108 = false;
        _S88 = 0.0f;
        _S89 = 0.0f;
        _S90 = 0.0f;
        _S91 = 0.0f;
        _S92 = 0.0f;
        _S93 = 0.0f;
        _S94 = 0.0f;
        _S105._S13 = 0.0f;
        _S105._S14 = 0.0f;
        _S105._S15 = 0.0f;
        _S95 = 0.0f;
        _S96 = 0.0f;
        _S97 = 0.0f;
        _S98 = 0.0f;
        _S99 = 0.0f;
        _S100 = 0.0f;
        _S101 = 0.0f;
        _S102 = 0.0f;
        _S106._S13 = 0.0f;
        _S106._S14 = 0.0f;
        _S106._S15 = 0.0f;
        _S103 = 0.0f;
        _S104 = 0.0f;
    }
    else
    {
        var _S110 : bool = (c_1 < (-2.0f * _S80.value0_6.value2_1.midtone_0));
        if(_S110)
        {
            var _S111 : f32 = c_1 + _S80.value0_6.value2_1.midtone_0;
            _S84 = _S80.value0_6.value2_1.highlights_0;
            _S85 = _S111;
            _S107 = false;
            _S86 = 0.0f;
            _S87 = 0.0f;
            _S88 = 0.0f;
            _S89 = 0.0f;
            _S90 = 0.0f;
            _S91 = 0.0f;
            _S92 = 0.0f;
            _S105._S13 = 0.0f;
            _S105._S14 = 0.0f;
            _S105._S15 = 0.0f;
            _S93 = 0.0f;
            _S94 = 0.0f;
            _S95 = 0.0f;
            _S96 = 0.0f;
            _S97 = 0.0f;
            _S98 = 0.0f;
            _S99 = 0.0f;
            _S100 = 0.0f;
            _S106._S13 = 0.0f;
            _S106._S14 = 0.0f;
            _S106._S15 = 0.0f;
            _S101 = 0.0f;
            _S102 = 0.0f;
        }
        else
        {
            var _S112 : bool = (c_1 > 0.0f);
            if(_S112)
            {
                var _S113 : f32 = c_1 / _S82;
                var _S114 : f32 = _S82 * _S82;
                var _S115 : Tuple_1 = s_apply_clamp_0(_S113, 0.0f, 1.0f);
                var _S116 : s_bwdCallableCtx_clamp_0;
                _S116._S13 = _S113;
                _S116._S14 = 0.0f;
                _S116._S15 = 1.0f;
                var mt_0 : f32 = 1.0f - _S115.value0_1;
                var _S117 : f32 = _S80.value0_6.value2_1.midtone_0 * 2.0f;
                var _S118 : f32 = _S117 * mt_0;
                var _S119 : f32 = _S80.value0_6.value2_1.midtone_0 + _S80.value0_6.value2_1.midtone_0 * _S80.value0_6.value2_1.shadows_0;
                _S84 = _S115.value0_1 * _S115.value0_1;
                _S85 = _S119;
                _S86 = _S80.value0_6.value2_1.shadows_0;
                _S87 = _S118;
                _S88 = _S115.value0_1;
                _S89 = _S117;
                _S90 = mt_0;
                _S105 = _S116;
                _S91 = _S114;
                _S92 = 0.0f;
                _S93 = 0.0f;
                _S94 = 0.0f;
                _S95 = 0.0f;
                _S96 = 0.0f;
                _S97 = 0.0f;
                _S98 = 0.0f;
                _S106._S13 = 0.0f;
                _S106._S14 = 0.0f;
                _S106._S15 = 0.0f;
                _S99 = 0.0f;
                _S100 = 0.0f;
            }
            else
            {
                var _S120 : f32 = - c_1;
                var _S121 : f32 = _S120 / _S82;
                var _S122 : f32 = _S82 * _S82;
                var _S123 : Tuple_1 = s_apply_clamp_0(_S121, 0.0f, 1.0f);
                var _S124 : s_bwdCallableCtx_clamp_0;
                _S124._S13 = _S121;
                _S124._S14 = 0.0f;
                _S124._S15 = 1.0f;
                var t2_0 : f32 = _S123.value0_1 * _S123.value0_1;
                var mt_1 : f32 = 1.0f - _S123.value0_1;
                var _S125 : f32 = _S80.value0_6.value2_1.midtone_0 * 2.0f;
                var _S126 : f32 = _S125 * mt_1;
                var _S127 : f32 = - _S80.value0_6.value2_1.midtone_0 - _S80.value0_6.value2_1.midtone_0 * _S80.value0_6.value2_1.highlights_0;
                _S84 = 0.0f;
                _S85 = 0.0f;
                _S86 = 0.0f;
                _S87 = 0.0f;
                _S88 = 0.0f;
                _S89 = 0.0f;
                _S90 = 0.0f;
                _S105._S13 = 0.0f;
                _S105._S14 = 0.0f;
                _S105._S15 = 0.0f;
                _S91 = 0.0f;
                _S92 = t2_0;
                _S93 = _S127;
                _S94 = _S80.value0_6.value2_1.highlights_0;
                _S95 = _S126;
                _S96 = _S123.value0_1;
                _S97 = _S125;
                _S98 = mt_1;
                _S106 = _S124;
                _S99 = _S122;
                _S100 = _S120;
            }
            var _S128 : f32 = _S84;
            var _S129 : f32 = _S85;
            var _S130 : f32 = _S86;
            var _S131 : f32 = _S87;
            var _S132 : f32 = _S88;
            var _S133 : f32 = _S89;
            var _S134 : f32 = _S90;
            var _S135 : f32 = _S91;
            var _S136 : f32 = _S92;
            var _S137 : f32 = _S93;
            var _S138 : f32 = _S94;
            var _S139 : f32 = _S95;
            var _S140 : f32 = _S96;
            var _S141 : f32 = _S97;
            var _S142 : f32 = _S98;
            var _S143 : f32 = _S99;
            var _S144 : f32 = _S100;
            _S84 = 0.0f;
            _S85 = 0.0f;
            _S107 = _S112;
            _S86 = _S128;
            _S87 = _S129;
            _S88 = _S130;
            _S89 = _S131;
            _S90 = _S132;
            _S91 = _S133;
            _S92 = _S134;
            _S93 = _S135;
            _S94 = _S136;
            _S95 = _S137;
            _S96 = _S138;
            _S97 = _S139;
            _S98 = _S140;
            _S99 = _S141;
            _S100 = _S142;
            _S101 = _S143;
            _S102 = _S144;
        }
        var _S145 : f32 = _S84;
        var _S146 : f32 = _S85;
        var _S147 : bool = _S107;
        var _S148 : f32 = _S86;
        var _S149 : f32 = _S87;
        var _S150 : f32 = _S88;
        var _S151 : f32 = _S89;
        var _S152 : f32 = _S90;
        var _S153 : f32 = _S91;
        var _S154 : f32 = _S92;
        var _S155 : f32 = _S93;
        var _S156 : f32 = _S94;
        var _S157 : f32 = _S95;
        var _S158 : f32 = _S96;
        var _S159 : f32 = _S97;
        var _S160 : f32 = _S98;
        var _S161 : f32 = _S99;
        var _S162 : f32 = _S100;
        var _S163 : f32 = _S101;
        var _S164 : f32 = _S102;
        _S84 = 0.0f;
        _S85 = 0.0f;
        _S107 = _S110;
        _S86 = _S145;
        _S87 = _S146;
        _S108 = _S147;
        _S88 = _S148;
        _S89 = _S149;
        _S90 = _S150;
        _S91 = _S151;
        _S92 = _S152;
        _S93 = _S153;
        _S94 = _S154;
        _S95 = _S155;
        _S96 = _S156;
        _S97 = _S157;
        _S98 = _S158;
        _S99 = _S159;
        _S100 = _S160;
        _S101 = _S161;
        _S102 = _S162;
        _S103 = _S163;
        _S104 = _S164;
    }
    var _S165 : f32 = _S80.value0_6.value2_1.detail_0 * c_1;
    var _S166 : f32 = - c_1 * c_1;
    var _S167 : f32 = _S82 * _S80.value0_6.value2_1.midtone_0 / 3.0f;
    var _S168 : f32 = _S166 / _S167;
    var _S169 : f32 = _S167 * _S167;
    var _S170 : Tuple_0 = s_apply_fast_exp_0(_S168);
    var _S171 : s_bwdCallableCtx_fast_exp_0;
    _S171._S9 = _S168;
    var _S172 : s_bwdCallableCtx_fast_exp_0 = _S171;
    var _S173 : Settings_0 = Settings_x24_syn_dzero_0();
    var _S174 : f32 = _S165 * s_diff_val_T_0;
    var _S175 : f32 = _S170.value0_0 * s_diff_val_T_0;
    var _S176 : f32 = 0.0f;
    s_bwdProp_fast_exp_0(_S172, &(_S176), _S174);
    var _S177 : f32 = 0.3333333432674408f * (_S166 * - (_S176 / _S169));
    var _S178 : f32 = _S82 * _S177;
    var _S179 : f32 = _S80.value0_6.value2_1.midtone_0 * _S177;
    var _S180 : f32 = c_1 * _S175;
    var _S181 : Settings_0 = _S173;
    _S181.detail_0 = _S180;
    var _S182 : Settings_0 = Settings_x24_syn_dadd_0(_S173, _S181);
    var _S183 : Settings_0;
    if(_S83)
    {
        var _S184 : f32 = _S85 * s_diff_val_T_0;
        var _S185 : f32 = - (_S84 * s_diff_val_T_0);
        var _S186 : Settings_0 = _S173;
        _S186.shadows_0 = _S184;
        var _S187 : Settings_0 = Settings_x24_syn_dadd_0(_S182, _S186);
        var _S188 : f32 = _S185 + s_diff_val_T_0 + _S178;
        _S84 = _S179;
        _S85 = _S188;
        _S183 = _S187;
    }
    else
    {
        if(_S107)
        {
            var _S189 : f32 = _S86 * s_diff_val_T_0;
            var _S190 : f32 = _S87 * s_diff_val_T_0;
            var _S191 : f32 = - s_diff_val_T_0;
            var _S192 : Settings_0 = _S173;
            _S192.highlights_0 = _S190;
            var _S193 : Settings_0 = Settings_x24_syn_dadd_0(_S182, _S192);
            _S84 = _S189 + _S191 + _S178;
            _S85 = _S179;
            _S183 = _S193;
        }
        else
        {
            if(_S108)
            {
                var _S194 : f32 = _S88 * s_diff_val_T_0;
                var _S195 : f32 = _S80.value0_6.value2_1.midtone_0 * _S194;
                var _S196 : f32 = _S90 * _S194;
                var _S197 : f32 = _S92 * s_diff_val_T_0;
                var _S198 : f32 = 2.0f * (_S94 * _S197);
                var _S199 : f32 = _S92 * (_S89 * s_diff_val_T_0);
                var _S200 : f32 = _S91 * s_diff_val_T_0 + - (_S93 * _S197) + _S199 + _S199;
                var _S201 : f32 = 0.0f;
                var _S202 : f32 = 0.0f;
                var _S203 : f32 = 0.0f;
                s_bwdProp_clamp_0(_S105, &(_S201), &(_S202), &(_S203), _S200);
                var _S204 : f32 = c_1 * - (_S201 / _S95);
                var _S205 : Settings_0 = _S173;
                _S205.shadows_0 = _S195;
                var _S206 : Settings_0 = Settings_x24_syn_dadd_0(_S182, _S205);
                var _S207 : f32 = _S204 + _S179;
                _S84 = _S194 + _S196 + _S198 + _S178;
                _S85 = _S207;
                _S183 = _S206;
            }
            else
            {
                var _S208 : f32 = - (_S96 * s_diff_val_T_0);
                var _S209 : f32 = _S80.value0_6.value2_1.midtone_0 * _S208;
                var _S210 : f32 = _S98 * _S208;
                var _S211 : f32 = - s_diff_val_T_0;
                var _S212 : f32 = _S100 * _S211;
                var _S213 : f32 = 2.0f * (_S102 * _S212);
                var _S214 : f32 = _S100 * (_S97 * s_diff_val_T_0);
                var _S215 : f32 = _S99 * _S211 + - (_S101 * _S212) + _S214 + _S214;
                var _S216 : f32 = 0.0f;
                var _S217 : f32 = 0.0f;
                var _S218 : f32 = 0.0f;
                s_bwdProp_clamp_0(_S106, &(_S216), &(_S217), &(_S218), _S215);
                var _S219 : f32 = _S104 * - (_S216 / _S103);
                var _S220 : f32 = _S210 + _S208 + _S213 + _S178;
                var _S221 : Settings_0 = _S173;
                _S221.highlights_0 = _S209;
                var _S222 : Settings_0 = Settings_x24_syn_dadd_0(_S182, _S221);
                var _S223 : f32 = _S219 + _S179;
                _S84 = _S220;
                _S85 = _S223;
                _S183 = _S222;
            }
        }
        var _S224 : f32 = _S84;
        _S84 = _S85;
        _S85 = _S224;
    }
    var _S225 : f32 = 2.0f * _S84 + _S85;
    var _S226 : Settings_0 = _S173;
    _S226.midtone_0 = _S225;
    (*_S81) = Settings_x24_syn_dadd_0(_S183, _S226);
    return;
}

fn s_bwdProp_bilat_0( _S227 : s_bwdCallableCtx_bilat_0,  _S228 : ptr<function, Settings_0>,  _S229 : vec3<f32>)
{
    var _S230 : Settings_0 = Settings_0( 0.0f, 0.0f, 0.0f, 0.0f );
    var _S231 : s_paramCtx_s_bwdCallableCtx_curve_0 = s_paramCtx_s_bwdCallableCtx_curve_0( 0.0f, 0.0f, _S230 );
    var _S232 : f32 = _S227.value0_4.value0_3.x;
    var _S233 : bool = (_S227.value0_4.value1_0.mode_0) == u32(1);
    var l_0 : f32;
    var _S234 : f32;
    var _S235 : f32;
    var _S236 : f32;
    var _S237 : f32;
    var _S238 : s_bwdCallableCtx_max_0;
    var _S239 : s_bwdCallableCtx_curve_0;
    var _S240 : s_bwdCallableCtx_curve_0;
    var _S241 : bool;
    if(_S233)
    {
        var v_0 : f32 = _S232 * 0.0099999997764825821f;
        var _S242 : f32 = gamma_of_0(_S227.value0_4.value1_0.lo_0);
        var _S243 : Tuple_3 = s_apply_curve_0(v_0, _S242, _S227.value0_4.value3_0);
        var _S244 : s_paramCtx_s_bwdCallableCtx_curve_0;
        _S244.value0_5 = v_0;
        _S244.value1_1 = _S242;
        _S244.value2_1 = _S227.value0_4.value3_0;
        var l0_0 : f32 = _S243.value0_7 - _S227.value0_4.value1_0.lower_0;
        var _S245 : f32 = gamma_of_0(_S227.value0_4.value1_0.lo_0 + u32(1));
        var _S246 : Tuple_3 = s_apply_curve_0(v_0, _S245, _S227.value0_4.value3_0);
        var _S247 : s_paramCtx_s_bwdCallableCtx_curve_0;
        _S247.value0_5 = v_0;
        _S247.value1_1 = _S245;
        _S247.value2_1 = _S227.value0_4.value3_0;
        var _S248 : f32 = 1.0f - _S227.value0_4.value1_0.a_0;
        l_0 = 100.0f * (_S227.value0_4.value1_0.output_1 + (l0_0 * _S248 + (_S246.value0_7 - _S227.value0_4.value1_0.upper_0) * _S227.value0_4.value1_0.a_0));
        _S234 = _S227.value0_4.value1_0.a_0;
        _S235 = _S248;
        _S239.value0_6 = _S247;
        _S240.value0_6 = _S244;
        _S241 = false;
        _S238._S25 = 0.0f;
        _S238._S26 = 0.0f;
        _S236 = 0.0f;
        _S237 = 0.0f;
    }
    else
    {
        var _S249 : bool = (_S227.value0_4.value1_0.mode_0) == u32(0);
        if(_S249)
        {
            var _S250 : f32 = _S232 + - _S227.value0_4.value3_0.detail_0 * _S227.value0_4.value1_0.sigma_r_0 * 0.039999999105930328f * _S227.value0_4.value1_0.grid_1;
            var _S251 : Tuple_2 = s_apply_max_0(0.0f, _S250);
            var _S252 : s_bwdCallableCtx_max_0;
            _S252._S25 = 0.0f;
            _S252._S26 = _S250;
            l_0 = _S251.value0_2;
            _S238 = _S252;
            _S234 = _S227.value0_4.value1_0.grid_1;
            _S235 = _S227.value0_4.value1_0.sigma_r_0;
        }
        else
        {
            l_0 = _S232;
            _S238._S25 = 0.0f;
            _S238._S26 = 0.0f;
            _S234 = 0.0f;
            _S235 = 0.0f;
        }
        var _S253 : f32 = _S234;
        var _S254 : f32 = _S235;
        _S234 = 0.0f;
        _S235 = 0.0f;
        _S239.value0_6 = _S231;
        _S240.value0_6 = _S231;
        _S241 = _S249;
        _S236 = _S253;
        _S237 = _S254;
    }
    var _S255 : s_paramCtx_s_bwdCallableCtx_lab_to_rgb_0;
    _S255.value0_8 = vec3<f32>(l_0, _S227.value0_4.value0_3.y, _S227.value0_4.value0_3.z);
    _S255.value1_2 = _S227.value0_4.value2_0;
    var _S256 : s_bwdCallableCtx_lab_to_rgb_0 = s_bwdCallableCtx_lab_to_rgb_0( _S255 );
    var _S257 : Settings_0 = Settings_x24_syn_dzero_0();
    var _S258 : vec3<f32> = vec3<f32>(0.0f);
    s_bwdProp_lab_to_rgb_0(_S256, &(_S258), _S229);
    var _S259 : vec3<f32> = _S258;
    var _S260 : Settings_0;
    if(_S233)
    {
        var _S261 : f32 = 100.0f * _S259[i32(0)];
        var s_diff_l1_T_0 : f32 = _S234 * _S261;
        var s_diff_l0_T_0 : f32 = _S235 * _S261;
        var _S262 : Settings_0;
        _S262.detail_0 = 0.0f;
        _S262.shadows_0 = 0.0f;
        _S262.highlights_0 = 0.0f;
        _S262.midtone_0 = 0.0f;
        s_bwdProp_curve_0(_S239, &(_S262), s_diff_l1_T_0);
        var _S263 : Settings_0;
        _S263.detail_0 = 0.0f;
        _S263.shadows_0 = 0.0f;
        _S263.highlights_0 = 0.0f;
        _S263.midtone_0 = 0.0f;
        s_bwdProp_curve_0(_S240, &(_S263), s_diff_l0_T_0);
        _S260 = Settings_x24_syn_dadd_0(Settings_x24_syn_dadd_0(_S262, _S263), _S257);
    }
    else
    {
        if(_S241)
        {
            var _S264 : f32 = 0.0f;
            var _S265 : f32 = 0.0f;
            s_bwdProp_max_0(_S238, &(_S264), &(_S265), _S259[i32(0)]);
            var _S266 : f32 = - (_S237 * (0.039999999105930328f * (_S236 * _S265)));
            var _S267 : Settings_0 = _S257;
            _S267.detail_0 = _S266;
            _S260 = Settings_x24_syn_dadd_0(_S257, _S267);
        }
        else
        {
            _S260 = _S257;
        }
    }
    (*_S228) = _S260;
    return;
}

fn _S268( _S269 : vec3<f32>,  _S270 : Neighbourhood_0,  _S271 : Colour_0,  _S272 : ptr<function, DiffPair_Settings_0>,  _S273 : vec3<f32>)
{
    var _S274 : s_paramCtx_s_bwdCallableCtx_bilat_0;
    _S274.value0_3 = _S269;
    _S274.value1_0 = _S270;
    _S274.value2_0 = _S271;
    _S274.value3_0 = (*_S272).primal_0;
    var _S275 : s_bwdCallableCtx_bilat_0 = s_bwdCallableCtx_bilat_0( _S274 );
    var _S276 : Settings_0 = (*_S272).differential_0;
    s_bwdProp_bilat_0(_S275, &(_S276), _S273);
    (*_S272).differential_0 = _S276;
    return;
}

fn _d_max_1( dpx_2 : DiffPair_float_0,  dpy_1 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S277 : f32 = max(dpx_2.primal_0, dpy_1.primal_0);
    var _S278 : f32;
    if(((dpx_2.primal_0) > (dpy_1.primal_0)))
    {
        _S278 = dpx_2.differential_0;
    }
    else
    {
        if(((dpx_2.primal_0) < (dpy_1.primal_0)))
        {
            _S278 = dpy_1.differential_0;
        }
        else
        {
            _S278 = 0.5f * (dpx_2.differential_0 + dpy_1.differential_0);
        }
    }
    var _S279 : DiffPair_float_0 = DiffPair_float_0( _S277, _S278 );
    return _S279;
}

var<private> BINOMIAL_0 : array<f32, i32(5)> = array<f32, i32(5)>( 1.0f, 4.0f, 6.0f, 4.0f, 1.0f );
fn _d_clamp_1( dpx_3 : DiffPair_float_0,  dpMin_1 : DiffPair_float_0,  dpMax_1 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S280 : f32 = clamp(dpx_3.primal_0, dpMin_1.primal_0, dpMax_1.primal_0);
    var _S281 : bool;
    if(((dpx_3.primal_0) >= (dpMin_1.primal_0)))
    {
        _S281 = ((dpx_3.primal_0) <= (dpMax_1.primal_0));
    }
    else
    {
        _S281 = false;
    }
    var _S282 : f32;
    if(_S281)
    {
        _S282 = dpx_3.differential_0;
    }
    else
    {
        if(((dpx_3.primal_0) < (dpMin_1.primal_0)))
        {
            _S282 = dpMin_1.differential_0;
        }
        else
        {
            _S282 = dpMax_1.differential_0;
        }
    }
    var _S283 : DiffPair_float_0 = DiffPair_float_0( _S280, _S282 );
    return _S283;
}

struct Seven_0
{
     a_1 : vec4<f32>,
     b_0 : vec4<f32>,
};

fn Seven_x24init_0( a_2 : vec4<f32>,  b_1 : vec4<f32>) -> Seven_0
{
    var _S284 : Seven_0;
    _S284.a_1 = a_2;
    _S284.b_0 = b_1;
    return _S284;
}

fn pow0_0( x_6 : f32,  y_1 : f32) -> f32
{
    if((x_6 <= 0.0f))
    {
        return 0.0f;
    }
    return pow(x_6, y_1);
}

fn lab_f_0( x_7 : f32) -> f32
{
    if((x_7 > 0.0088564520701766014f))
    {
        return pow0_0(x_7, 0.3333333432674408f);
    }
    return (903.29632568359375f * x_7 + 16.0f) / 116.0f;
}

fn xyz_to_lab_d50_0( xyz_0 : vec3<f32>) -> vec3<f32>
{
    var x_8 : vec3<f32> = xyz_0 / vec3<f32>(0.96420001983642578f, 1.0f, 0.82489997148513794f);
    var _S285 : f32 = lab_f_0(x_8.y);
    return vec3<f32>(116.0f * _S285 - 16.0f, 500.0f * (lab_f_0(x_8.x) - _S285), -200.0f * (lab_f_0(x_8.z) - _S285));
}

fn rgb_to_lab_0( rgb_0 : vec3<f32>,  c_2 : Colour_0) -> vec3<f32>
{
    return xyz_to_lab_d50_0((((rgb_0) * (c_2.rgb_to_xyz_0))));
}

fn fast_exp_forward_0( x_9 : DiffPair_float_0) -> DiffPair_float_0
{
    var value_1 : f32 = fast_exp_0(x_9.primal_0);
    var _S286 : DiffPair_float_0 = DiffPair_float_0( value_1, fast_exp_slope_0(value_1) * x_9.differential_0 );
    return _S286;
}

fn curve_1( x_10 : f32,  g_1 : f32,  s_0 : Settings_0) -> f32
{
    var c_3 : f32 = x_10 - g_1;
    var _S287 : f32 = 2.0f * s_0.midtone_0;
    var val_1 : f32;
    if((c_3 > _S287))
    {
        val_1 = g_1 + s_0.midtone_0 + s_0.shadows_0 * (c_3 - s_0.midtone_0);
    }
    else
    {
        if((c_3 < (-2.0f * s_0.midtone_0)))
        {
            val_1 = g_1 - s_0.midtone_0 + s_0.highlights_0 * (c_3 + s_0.midtone_0);
        }
        else
        {
            if((c_3 > 0.0f))
            {
                var t_0 : f32 = clamp(c_3 / _S287, 0.0f, 1.0f);
                val_1 = g_1 + s_0.midtone_0 * 2.0f * (1.0f - t_0) * t_0 + t_0 * t_0 * (s_0.midtone_0 + s_0.midtone_0 * s_0.shadows_0);
            }
            else
            {
                var t_1 : f32 = clamp(- c_3 / _S287, 0.0f, 1.0f);
                val_1 = g_1 - s_0.midtone_0 * 2.0f * (1.0f - t_1) * t_1 + t_1 * t_1 * (- s_0.midtone_0 - s_0.midtone_0 * s_0.highlights_0);
            }
        }
    }
    return val_1 + s_0.detail_0 * c_3 * fast_exp_0(- c_3 * c_3 / (_S287 * s_0.midtone_0 / 3.0f));
}

fn s_fwd_curve_0( x_11 : f32,  g_2 : f32,  dps_0 : DiffPair_Settings_0) -> DiffPair_float_0
{
    var c_4 : f32 = x_11 - g_2;
    var _S288 : f32 = 2.0f * dps_0.primal_0.midtone_0;
    var _S289 : f32 = dps_0.differential_0.midtone_0 * 2.0f;
    var val_2 : f32;
    var s_diff_val_0 : f32;
    if((c_4 > _S288))
    {
        var _S290 : f32 = c_4 - dps_0.primal_0.midtone_0;
        var _S291 : f32 = dps_0.differential_0.midtone_0 + (dps_0.differential_0.shadows_0 * _S290 + (0.0f - dps_0.differential_0.midtone_0) * dps_0.primal_0.shadows_0);
        val_2 = g_2 + dps_0.primal_0.midtone_0 + dps_0.primal_0.shadows_0 * _S290;
        s_diff_val_0 = _S291;
    }
    else
    {
        if((c_4 < (-2.0f * dps_0.primal_0.midtone_0)))
        {
            var _S292 : f32 = c_4 + dps_0.primal_0.midtone_0;
            var _S293 : f32 = 0.0f - dps_0.differential_0.midtone_0 + (dps_0.differential_0.highlights_0 * _S292 + dps_0.differential_0.midtone_0 * dps_0.primal_0.highlights_0);
            val_2 = g_2 - dps_0.primal_0.midtone_0 + dps_0.primal_0.highlights_0 * _S292;
            s_diff_val_0 = _S293;
        }
        else
        {
            if((c_4 > 0.0f))
            {
                var _S294 : DiffPair_float_0 = DiffPair_float_0( c_4 / _S288, (0.0f - c_4 * _S289) / (_S288 * _S288) );
                var _S295 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
                var _S296 : DiffPair_float_0 = DiffPair_float_0( 1.0f, 0.0f );
                var _S297 : DiffPair_float_0 = _d_clamp_1(_S294, _S295, _S296);
                var _S298 : f32 = _S297.primal_0;
                var t2_1 : f32 = _S298 * _S298;
                var _S299 : f32 = _S297.differential_0 * _S297.primal_0;
                var mt_2 : f32 = 1.0f - _S297.primal_0;
                var _S300 : f32 = dps_0.primal_0.midtone_0 * 2.0f;
                var _S301 : f32 = _S300 * mt_2;
                var _S302 : f32 = dps_0.primal_0.midtone_0 + dps_0.primal_0.midtone_0 * dps_0.primal_0.shadows_0;
                var _S303 : f32 = (_S289 * mt_2 + (0.0f - _S297.differential_0) * _S300) * _S297.primal_0 + _S297.differential_0 * _S301 + ((_S299 + _S299) * _S302 + (dps_0.differential_0.midtone_0 + (dps_0.differential_0.midtone_0 * dps_0.primal_0.shadows_0 + dps_0.differential_0.shadows_0 * dps_0.primal_0.midtone_0)) * t2_1);
                val_2 = g_2 + _S301 * _S297.primal_0 + t2_1 * _S302;
                s_diff_val_0 = _S303;
            }
            else
            {
                var _S304 : f32 = - c_4;
                var _S305 : DiffPair_float_0 = DiffPair_float_0( _S304 / _S288, (0.0f - _S304 * _S289) / (_S288 * _S288) );
                var _S306 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
                var _S307 : DiffPair_float_0 = DiffPair_float_0( 1.0f, 0.0f );
                var _S308 : DiffPair_float_0 = _d_clamp_1(_S305, _S306, _S307);
                var _S309 : f32 = _S308.primal_0;
                var t2_2 : f32 = _S309 * _S309;
                var _S310 : f32 = _S308.differential_0 * _S308.primal_0;
                var mt_3 : f32 = 1.0f - _S308.primal_0;
                var _S311 : f32 = dps_0.primal_0.midtone_0 * 2.0f;
                var _S312 : f32 = _S311 * mt_3;
                var _S313 : f32 = - dps_0.primal_0.midtone_0 - dps_0.primal_0.midtone_0 * dps_0.primal_0.highlights_0;
                var _S314 : f32 = 0.0f - ((_S289 * mt_3 + (0.0f - _S308.differential_0) * _S311) * _S308.primal_0 + _S308.differential_0 * _S312) + ((_S310 + _S310) * _S313 + (- dps_0.differential_0.midtone_0 - (dps_0.differential_0.midtone_0 * dps_0.primal_0.highlights_0 + dps_0.differential_0.highlights_0 * dps_0.primal_0.midtone_0)) * t2_2);
                val_2 = g_2 - _S312 * _S308.primal_0 + t2_2 * _S313;
                s_diff_val_0 = _S314;
            }
        }
    }
    var _S315 : f32 = dps_0.primal_0.detail_0 * c_4;
    var _S316 : f32 = - c_4 * c_4;
    var _S317 : f32 = _S288 * dps_0.primal_0.midtone_0 / 3.0f;
    var _S318 : DiffPair_float_0 = DiffPair_float_0( _S316 / _S317, (0.0f - _S316 * ((_S289 * dps_0.primal_0.midtone_0 + dps_0.differential_0.midtone_0 * _S288) * 0.3333333432674408f)) / (_S317 * _S317) );
    var _S319 : DiffPair_float_0 = fast_exp_forward_0(_S318);
    var _S320 : DiffPair_float_0 = DiffPair_float_0( val_2 + _S315 * _S319.primal_0, s_diff_val_0 + (dps_0.differential_0.detail_0 * c_4 * _S319.primal_0 + _S319.differential_0 * _S315) );
    return _S320;
}

fn remapped_0( v_1 : f32,  s_1 : Settings_0) -> Seven_0
{
    var r_0 : Seven_0;
    r_0.a_1 = vec4<f32>(v_1, curve_1(v_1, gamma_of_0(u32(0)), s_1), curve_1(v_1, gamma_of_0(u32(1)), s_1), curve_1(v_1, gamma_of_0(u32(2)), s_1));
    r_0.b_0 = vec4<f32>(curve_1(v_1, gamma_of_0(u32(3)), s_1), curve_1(v_1, gamma_of_0(u32(4)), s_1), curve_1(v_1, gamma_of_0(u32(5)), s_1), 0.0f);
    return r_0;
}

fn colour_of_0() -> Colour_0
{
    var c_5 : Colour_0;
    c_5.rgb_to_xyz_0 = mat3x3<f32>(params_0.bilat_0.rgb_to_xyz_r0_0.xyz, params_0.bilat_0.rgb_to_xyz_r1_0.xyz, params_0.bilat_0.rgb_to_xyz_r2_0.xyz);
    c_5.xyz_to_rgb_0 = mat3x3<f32>(params_0.bilat_0.xyz_to_rgb_r0_0.xyz, params_0.bilat_0.xyz_to_rgb_r1_0.xyz, params_0.bilat_0.xyz_to_rgb_r2_0.xyz);
    return c_5;
}

fn settings_of_0() -> Settings_0
{
    var s_2 : Settings_0;
    s_2.detail_0 = params_0.bilat_0.curve_0.w;
    s_2.shadows_0 = params_0.bilat_0.curve_0.y;
    s_2.highlights_0 = params_0.bilat_0.curve_0.z;
    s_2.midtone_0 = params_0.bilat_0.curve_0.x;
    return s_2;
}

fn padded_value_0( _S321 : i32,  _S322 : i32,  _S323 : Colour_0) -> f32
{
    var max_supp_0 : i32 = i32(params_0.bilat_0.image_0.z);
    var _S324 : u32 = params_0.bilat_0.image_0.x;
    return rgb_to_lab_0(input_0[u32(clamp(_S322 - max_supp_0, i32(0), i32(params_0.bilat_0.image_0.y) - i32(1))) * _S324 + u32(clamp(_S321 - max_supp_0, i32(0), i32(_S324) - i32(1)))].xyz, _S323).x * 0.0099999997764825821f;
}

fn write_coarse_0( _S325 : u32,  _S326 : Seven_0)
{
    var size_0 : u32 = params_0.bilat_0.coarse_0.z;
    var _S327 : f32 = _S326.a_1.x;
    coarse_1[_S325] = _S327;
    coarse_1[_S325 + size_0] = _S326.a_1.y;
    coarse_1[_S325 + u32(2) * size_0] = _S326.a_1.z;
    coarse_1[_S325 + u32(3) * size_0] = _S326.a_1.w;
    coarse_1[_S325 + u32(4) * size_0] = _S326.b_0.x;
    coarse_1[_S325 + u32(5) * size_0] = _S326.b_0.y;
    coarse_1[_S325 + u32(6) * size_0] = _S326.b_0.z;
    if((params_0.bilat_0.coarse_0.w) != u32(0))
    {
        coarse_1[_S325 + u32(7) * size_0] = _S327;
    }
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn ll_reduce_input(@builtin(global_invocation_id) id_0 : vec3<u32>)
{
    var _S328 : u32 = id_0.x;
    var _S329 : vec4<u32> = params_0.bilat_0.coarse_0;
    var _S330 : u32 = params_0.bilat_0.coarse_0.x;
    var _S331 : bool;
    if((_S328 >= _S330))
    {
        _S331 = true;
    }
    else
    {
        _S331 = ((id_0.y) >= (_S329.y));
    }
    if(_S331)
    {
        return;
    }
    var _S332 : Colour_0 = colour_of_0();
    var _S333 : Settings_0 = settings_of_0();
    var _S334 : i32 = clamp(i32(_S328), i32(1), i32(_S330) - i32(2));
    var _S335 : u32 = id_0.y;
    var _S336 : i32 = clamp(i32(_S335), i32(1), i32(_S329.y) - i32(2));
    var _S337 : vec4<f32> = vec4<f32>(0.0f);
    var _S338 : Seven_0 = Seven_x24init_0(_S337, _S337);
    var sum_6 : Seven_0 = _S338;
    var dx_0 : i32 = i32(0);
    loop
    {
        if((dx_0 < i32(5)))
        {
        }
        else
        {
            break;
        }
        var column_0 : Seven_0 = _S338;
        var dy_0 : i32 = i32(0);
        loop
        {
            if((dy_0 < i32(5)))
            {
            }
            else
            {
                break;
            }
            var r_1 : Seven_0 = remapped_0(padded_value_0(i32(2) * _S334 - i32(2) + dx_0, i32(2) * _S336 - i32(2) + dy_0, _S332), _S333);
            var _S339 : vec4<f32> = vec4<f32>(BINOMIAL_0[dy_0]);
            column_0.a_1 = column_0.a_1 + _S339 * r_1.a_1;
            column_0.b_0 = column_0.b_0 + _S339 * r_1.b_0;
            dy_0 = dy_0 + i32(1);
        }
        var _S340 : vec4<f32> = vec4<f32>(BINOMIAL_0[dx_0]);
        sum_6.a_1 = sum_6.a_1 + _S340 * column_0.a_1;
        sum_6.b_0 = sum_6.b_0 + _S340 * column_0.b_0;
        dx_0 = dx_0 + i32(1);
    }
    var _S341 : vec4<f32> = vec4<f32>(256.0f);
    sum_6.a_1 = sum_6.a_1 / _S341;
    sum_6.b_0 = sum_6.b_0 / _S341;
    write_coarse_0(_S335 * _S330 + _S328, sum_6);
    return;
}

fn level_planes_0( index_0 : u32,  plane_size_0 : u32) -> Seven_0
{
    var r_2 : Seven_0;
    r_2.a_1 = vec4<f32>(fine_1[index_0], fine_1[index_0 + plane_size_0], fine_1[index_0 + u32(2) * plane_size_0], fine_1[index_0 + u32(3) * plane_size_0]);
    r_2.b_0 = vec4<f32>(fine_1[index_0 + u32(4) * plane_size_0], fine_1[index_0 + u32(5) * plane_size_0], fine_1[index_0 + u32(6) * plane_size_0], 0.0f);
    return r_2;
}

@compute
@workgroup_size(16, 16, 1)
fn ll_reduce(@builtin(global_invocation_id) id_1 : vec3<u32>)
{
    var _S342 : u32 = id_1.x;
    var _S343 : vec4<u32> = params_0.bilat_0.coarse_0;
    var _S344 : u32 = params_0.bilat_0.coarse_0.x;
    var _S345 : bool;
    if((_S342 >= _S344))
    {
        _S345 = true;
    }
    else
    {
        _S345 = ((id_1.y) >= (_S343.y));
    }
    if(_S345)
    {
        return;
    }
    var _S346 : u32 = u32(clamp(i32(_S342), i32(1), i32(_S344) - i32(2)));
    var _S347 : u32 = id_1.y;
    var _S348 : u32 = u32(clamp(i32(_S347), i32(1), i32(_S343.y) - i32(2)));
    var _S349 : vec4<f32> = vec4<f32>(0.0f);
    var _S350 : Seven_0 = Seven_x24init_0(_S349, _S349);
    var sum_7 : Seven_0 = _S350;
    var dx_1 : u32 = u32(0);
    loop
    {
        if((dx_1 < u32(5)))
        {
        }
        else
        {
            break;
        }
        var column_1 : Seven_0 = _S350;
        var dy_1 : u32 = u32(0);
        loop
        {
            if((dy_1 < u32(5)))
            {
            }
            else
            {
                break;
            }
            var r_3 : Seven_0 = level_planes_0((u32(2) * _S348 - u32(2) + dy_1) * params_0.bilat_0.fine_0.x + u32(2) * _S346 - u32(2) + dx_1, params_0.bilat_0.fine_0.z);
            var _S351 : vec4<f32> = vec4<f32>(BINOMIAL_0[dy_1]);
            column_1.a_1 = column_1.a_1 + _S351 * r_3.a_1;
            column_1.b_0 = column_1.b_0 + _S351 * r_3.b_0;
            dy_1 = dy_1 + u32(1);
        }
        var _S352 : vec4<f32> = vec4<f32>(BINOMIAL_0[dx_1]);
        sum_7.a_1 = sum_7.a_1 + _S352 * column_1.a_1;
        sum_7.b_0 = sum_7.b_0 + _S352 * column_1.b_0;
        dx_1 = dx_1 + u32(1);
    }
    var _S353 : vec4<f32> = vec4<f32>(256.0f);
    sum_7.a_1 = sum_7.a_1 / _S353;
    sum_7.b_0 = sum_7.b_0 / _S353;
    write_coarse_0(_S347 * _S344 + _S342, sum_7);
    return;
}

fn expand_clamp_0( x_12 : u32,  size_1 : u32) -> u32
{
    return u32(clamp(i32(x_12), i32(1), i32(((size_1 - u32(1)) & (u32(4294967294)))) - i32(1)));
}

struct Bracket_0
{
     lo_1 : u32,
     a_3 : f32,
};

fn bracket_0( v_2 : f32) -> Bracket_0
{
    var hi_0 : u32 = u32(1);
    var k_2 : u32 = u32(1);
    loop
    {
        if((k_2 < u32(5)))
        {
        }
        else
        {
            break;
        }
        var _S354 : bool;
        if(hi_0 == k_2)
        {
            _S354 = ((gamma_of_0(k_2)) <= v_2);
        }
        else
        {
            _S354 = false;
        }
        if(_S354)
        {
            hi_0 = k_2 + u32(1);
        }
        k_2 = k_2 + u32(1);
    }
    var b_2 : Bracket_0;
    var _S355 : u32 = hi_0 - u32(1);
    b_2.lo_1 = _S355;
    var _S356 : f32 = gamma_of_0(_S355);
    b_2.a_3 = clamp((v_2 - _S356) / (gamma_of_0(hi_0) - _S356), 0.0f, 1.0f);
    return b_2;
}

fn expand_0( base_0 : u32,  i_0 : u32,  j_0 : u32,  cw_0 : u32) -> f32
{
    var ind_0 : u32 = base_0 + j_0 / u32(2) * cw_0 + i_0 / u32(2);
    var parity_0 : u32 = ((i_0 & (u32(1)))) + u32(2) * ((j_0 & (u32(1))));
    if(parity_0 == u32(0))
    {
        var _S357 : u32 = ind_0 - cw_0;
        var _S358 : u32 = ind_0 + cw_0;
        return 0.015625f * (6.0f * (coarse_1[_S357] + coarse_1[ind_0 - u32(1)] + 6.0f * coarse_1[ind_0] + coarse_1[ind_0 + u32(1)] + coarse_1[_S358]) + coarse_1[_S357 - u32(1)] + coarse_1[_S357 + u32(1)] + coarse_1[_S358 - u32(1)] + coarse_1[_S358 + u32(1)]);
    }
    if(parity_0 == u32(1))
    {
        var _S359 : u32 = ind_0 - cw_0;
        var _S360 : u32 = ind_0 + cw_0;
        return 0.015625f * (24.0f * (coarse_1[ind_0] + coarse_1[ind_0 + u32(1)]) + 4.0f * (coarse_1[_S359] + coarse_1[_S359 + u32(1)] + coarse_1[_S360] + coarse_1[_S360 + u32(1)]));
    }
    if(parity_0 == u32(2))
    {
        var _S361 : u32 = ind_0 + cw_0;
        return 0.015625f * (24.0f * (coarse_1[ind_0] + coarse_1[_S361]) + 4.0f * (coarse_1[ind_0 - u32(1)] + coarse_1[ind_0 + u32(1)] + coarse_1[_S361 - u32(1)] + coarse_1[_S361 + u32(1)]));
    }
    var _S362 : u32 = ind_0 + cw_0;
    return 0.25f * (coarse_1[ind_0] + coarse_1[ind_0 + u32(1)] + coarse_1[_S362] + coarse_1[_S362 + u32(1)]);
}

@compute
@workgroup_size(16, 16, 1)
fn ll_assemble(@builtin(global_invocation_id) id_2 : vec3<u32>)
{
    var _S363 : u32 = id_2.x;
    var _S364 : vec4<u32> = params_0.bilat_0.fine_0;
    var _S365 : u32 = params_0.bilat_0.fine_0.x;
    var _S366 : bool;
    if((_S363 >= _S365))
    {
        _S366 = true;
    }
    else
    {
        _S366 = ((id_2.y) >= (_S364.y));
    }
    if(_S366)
    {
        return;
    }
    var ci_0 : u32 = expand_clamp_0(_S363, _S365);
    var _S367 : u32 = id_2.y;
    var cj_0 : u32 = expand_clamp_0(_S367, _S364.y);
    var cw_1 : u32 = params_0.bilat_0.coarse_0.x;
    var index_1 : u32 = _S367 * _S365 + _S363;
    var b_3 : Bracket_0 = bracket_0(fine_1[index_1]);
    var plane_lo_0 : u32 = u32(1) + b_3.lo_1;
    var _S368 : u32 = _S364.z;
    var _S369 : u32 = params_0.bilat_0.coarse_0.z;
    var _S370 : u32 = plane_lo_0 + u32(1);
    fine_1[index_1 + u32(7) * _S368] = expand_0(u32(7) * _S369, ci_0, cj_0, cw_1) + ((fine_1[index_1 + plane_lo_0 * _S368] - expand_0(plane_lo_0 * _S369, ci_0, cj_0, cw_1)) * (1.0f - b_3.a_3) + (fine_1[index_1 + _S370 * _S368] - expand_0(_S370 * _S369, ci_0, cj_0, cw_1)) * b_3.a_3);
    return;
}

fn node_0( x_13 : u32,  y_2 : u32,  z_0 : u32,  size_2 : vec4<u32>) -> u32
{
    return (x_13 + y_2 * size_2.x) * size_2.z + z_0;
}

struct GridCoordinate_0
{
     cell_0 : u32,
     fraction_0 : f32,
};

fn grid_coordinate_0( position_0 : f32,  nodes_0 : u32) -> GridCoordinate_0
{
    var x_14 : f32 = clamp(position_0, 0.0f, f32(nodes_0 - u32(1)));
    var g_3 : GridCoordinate_0;
    var _S371 : u32 = min(u32(x_14), nodes_0 - u32(2));
    g_3.cell_0 = _S371;
    g_3.fraction_0 = x_14 - f32(_S371);
    return g_3;
}

fn lightness_at_0( _S372 : u32,  _S373 : u32,  _S374 : Colour_0) -> f32
{
    return rgb_to_lab_0(input_0[_S373 * params_0.bilat_0.image_0.x + _S372].xyz, _S374).x;
}

@compute
@workgroup_size(16, 16, 1)
fn bl_splat(@builtin(global_invocation_id) id_3 : vec3<u32>)
{
    var _S375 : u32;
    var size_3 : vec4<u32> = params_0.bilat_0.grid_size_0;
    var _S376 : u32 = id_3.x;
    var _S377 : u32 = params_0.bilat_0.grid_size_0.x;
    var _S378 : bool;
    if((_S376 >= _S377))
    {
        _S378 = true;
    }
    else
    {
        _S378 = ((id_3.y) >= (size_3.y));
    }
    if(_S378)
    {
        return;
    }
    var _S379 : Colour_0 = colour_of_0();
    var _S380 : u32 = id_3.y;
    var _S381 : u32 = node_0(_S376, _S380, u32(0), size_3);
    var z_1 : u32 = u32(0);
    loop
    {
        var _S382 : u32 = size_3.z;
        _S375 = _S382;
        if((z_1 < _S382))
        {
        }
        else
        {
            break;
        }
        coarse_1[_S381 + z_1] = 0.0f;
        z_1 = z_1 + u32(1);
    }
    var _S383 : vec4<f32> = params_0.bilat_0.grid_0;
    var _S384 : f32 = params_0.bilat_0.grid_0.x;
    var sigma_0 : f32 = 1.0f / _S384;
    var _S385 : f32 = f32(_S376);
    var _S386 : i32 = max(i32(floor((_S385 - 1.0f) * sigma_0)) - i32(2), i32(0));
    var _S387 : i32 = min(i32(ceil((_S385 + 1.0f) * sigma_0)) + i32(2), i32(params_0.bilat_0.image_0.x) - i32(1));
    var _S388 : f32 = f32(_S380);
    var _S389 : i32 = min(i32(ceil((_S388 + 1.0f) * sigma_0)) + i32(2), i32(params_0.bilat_0.image_0.y) - i32(1));
    var j_1 : i32 = max(i32(floor((_S388 - 1.0f) * sigma_0)) - i32(2), i32(0));
    loop
    {
        if((j_1 <= _S389))
        {
        }
        else
        {
            break;
        }
        var gy_0 : GridCoordinate_0 = grid_coordinate_0(f32(j_1) * _S384, size_3.y);
        if((gy_0.cell_0) != _S380)
        {
            _S378 = (gy_0.cell_0 + u32(1)) != _S380;
        }
        else
        {
            _S378 = false;
        }
        if(_S378)
        {
            j_1 = j_1 + i32(1);
            continue;
        }
        var _S390 : f32;
        if((gy_0.cell_0) == _S380)
        {
            _S390 = 1.0f - gy_0.fraction_0;
        }
        else
        {
            _S390 = gy_0.fraction_0;
        }
        var i_1 : i32 = _S386;
        loop
        {
            if((i_1 <= _S387))
            {
            }
            else
            {
                break;
            }
            var gx_0 : GridCoordinate_0 = grid_coordinate_0(f32(i_1) * _S384, _S377);
            var _S391 : bool;
            if((gx_0.cell_0) != _S376)
            {
                _S391 = (gx_0.cell_0 + u32(1)) != _S376;
            }
            else
            {
                _S391 = false;
            }
            if(_S391)
            {
                i_1 = i_1 + i32(1);
                continue;
            }
            var wx_0 : f32;
            if((gx_0.cell_0) == _S376)
            {
                wx_0 = 1.0f - gx_0.fraction_0;
            }
            else
            {
                wx_0 = gx_0.fraction_0;
            }
            var gz_0 : GridCoordinate_0 = grid_coordinate_0(lightness_at_0(u32(i_1), u32(j_1), _S379) * _S383.y, _S375);
            var contribution_0 : f32 = wx_0 * _S390 * 100.0f / _S383.z;
            var _S392 : u32 = _S381 + gz_0.cell_0;
            coarse_1[_S392] = coarse_1[_S392] + contribution_0 * (1.0f - gz_0.fraction_0);
            coarse_1[_S392 + u32(1)] = coarse_1[_S392 + u32(1)] + contribution_0 * gz_0.fraction_0;
            i_1 = i_1 + i32(1);
        }
        j_1 = j_1 + i32(1);
    }
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn bl_blur(@builtin(global_invocation_id) id_4 : vec3<u32>)
{
    var size_4 : vec4<u32> = params_0.bilat_0.grid_size_0;
    var _S393 : u32 = id_4.x;
    var _S394 : u32 = params_0.bilat_0.grid_size_0.x;
    var _S395 : u32 = params_0.bilat_0.grid_size_0.z;
    var _S396 : u32 = _S394 * _S395;
    var _S397 : bool;
    if((_S393 >= _S396))
    {
        _S397 = true;
    }
    else
    {
        _S397 = ((id_4.y) >= (size_4.y));
    }
    if(_S397)
    {
        return;
    }
    var x_15 : u32 = _S393 / _S395;
    var z_2 : u32 = _S393 % _S395;
    var _S398 : u32 = id_4.y;
    var index_2 : u32 = node_0(x_15, _S398, z_2, size_4);
    var _S399 : u32 = size_4.w;
    var position_1 : u32;
    var stride_0 : u32;
    var count_0 : u32;
    if(_S399 == u32(0))
    {
        position_1 = x_15;
        stride_0 = _S395;
        count_0 = _S394;
    }
    else
    {
        if(_S399 == u32(1))
        {
            var _S400 : u32 = size_4.y;
            position_1 = _S398;
            stride_0 = _S396;
            count_0 = _S400;
        }
        else
        {
            position_1 = z_2;
            stride_0 = u32(1);
            count_0 = _S395;
        }
    }
    var m2_0 : f32;
    if((position_1 >= u32(2)))
    {
        m2_0 = fine_1[index_2 - u32(2) * stride_0];
    }
    else
    {
        m2_0 = 0.0f;
    }
    var m1_0 : f32;
    if((position_1 >= u32(1)))
    {
        m1_0 = fine_1[index_2 - stride_0];
    }
    else
    {
        m1_0 = 0.0f;
    }
    var p1_0 : f32;
    if(((position_1 + u32(1)) < count_0))
    {
        p1_0 = fine_1[index_2 + stride_0];
    }
    else
    {
        p1_0 = 0.0f;
    }
    var p2_0 : f32;
    if(((position_1 + u32(2)) < count_0))
    {
        p2_0 = fine_1[index_2 + u32(2) * stride_0];
    }
    else
    {
        p2_0 = 0.0f;
    }
    if(_S399 == u32(2))
    {
        coarse_1[index_2] = 0.25f * (p1_0 - m1_0) + 0.125f * (p2_0 - m2_0);
    }
    else
    {
        coarse_1[index_2] = fine_1[index_2] * 0.375f + 0.25f * (p1_0 + m1_0) + 0.0625f * (p2_0 + m2_0);
    }
    return;
}

fn lab_f_inv_0( x_16 : f32) -> f32
{
    if((x_16 > 0.20689655840396881f))
    {
        return x_16 * x_16 * x_16;
    }
    return (116.0f * x_16 - 16.0f) / 903.29632568359375f;
}

fn s_fwd_lab_f_inv_0( dpx_4 : DiffPair_float_0) -> DiffPair_float_0
{
    if(((dpx_4.primal_0) > 0.20689655840396881f))
    {
        var _S401 : f32 = dpx_4.primal_0 * dpx_4.primal_0;
        var _S402 : f32 = dpx_4.differential_0 * dpx_4.primal_0;
        var _S403 : DiffPair_float_0 = DiffPair_float_0( _S401 * dpx_4.primal_0, (_S402 + _S402) * dpx_4.primal_0 + dpx_4.differential_0 * _S401 );
        return _S403;
    }
    var _S404 : DiffPair_float_0 = DiffPair_float_0( (116.0f * dpx_4.primal_0 - 16.0f) / 903.29632568359375f, dpx_4.differential_0 * 116.0f * 0.0011070563923567533f );
    return _S404;
}

fn lab_to_xyz_d50_0( lab_0 : vec3<f32>) -> vec3<f32>
{
    var fy_2 : f32 = (lab_0.x + 16.0f) * 0.0086206896230578423f;
    return vec3<f32>(0.96420001983642578f, 1.0f, 0.82489997148513794f) * vec3<f32>(lab_f_inv_0(lab_0.y * 0.0020000000949949026f + fy_2), lab_f_inv_0(fy_2), lab_f_inv_0(lab_0.z * -0.004999999888241291f + fy_2));
}

fn s_fwd_lab_to_xyz_d50_0( dplab_0 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var fy_3 : f32 = (dplab_0.primal_0.x + 16.0f) * 0.0086206896230578423f;
    var _S405 : f32 = dplab_0.differential_0.x * 0.0086206896230578423f;
    var _S406 : DiffPair_float_0 = DiffPair_float_0( dplab_0.primal_0.y * 0.0020000000949949026f + fy_3, dplab_0.differential_0.y * 0.0020000000949949026f + _S405 );
    var _S407 : DiffPair_float_0 = s_fwd_lab_f_inv_0(_S406);
    var _S408 : DiffPair_float_0 = DiffPair_float_0( fy_3, _S405 );
    var _S409 : DiffPair_float_0 = s_fwd_lab_f_inv_0(_S408);
    var _S410 : DiffPair_float_0 = DiffPair_float_0( dplab_0.primal_0.z * -0.004999999888241291f + fy_3, dplab_0.differential_0.z * -0.004999999888241291f + _S405 );
    var _S411 : DiffPair_float_0 = s_fwd_lab_f_inv_0(_S410);
    var _S412 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.96420001983642578f, 1.0f, 0.82489997148513794f) * vec3<f32>(_S407.primal_0, _S409.primal_0, _S411.primal_0), vec3<f32>(_S407.differential_0, _S409.differential_0, _S411.differential_0) * vec3<f32>(0.96420001983642578f, 1.0f, 0.82489997148513794f) );
    return _S412;
}

fn lab_to_rgb_0( lab_1 : vec3<f32>,  c_6 : Colour_0) -> vec3<f32>
{
    return (((lab_to_xyz_d50_0(lab_1)) * (c_6.xyz_to_rgb_0)));
}

fn s_fwd_lab_to_rgb_0( dplab_1 : DiffPair_vectorx3Cfloatx2C3x3E_0,  c_7 : Colour_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S413 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( dplab_1.primal_0, dplab_1.differential_0 );
    var _S414 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_lab_to_xyz_d50_0(_S413);
    var _S415 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( (((_S414.primal_0) * (c_7.xyz_to_rgb_0))), (((_S414.primal_0) * (mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f)))) + (((_S414.differential_0) * (c_7.xyz_to_rgb_0))) );
    return _S415;
}

fn bilat_1( lab_2 : vec3<f32>,  n_0 : Neighbourhood_0,  c_8 : Colour_0,  s_3 : Settings_0) -> vec3<f32>
{
    var _S416 : f32 = lab_2.x;
    var l_1 : f32;
    if((n_0.mode_0) == u32(1))
    {
        var v_3 : f32 = _S416 * 0.0099999997764825821f;
        l_1 = 100.0f * (n_0.output_1 + ((curve_1(v_3, gamma_of_0(n_0.lo_0), s_3) - n_0.lower_0) * (1.0f - n_0.a_0) + (curve_1(v_3, gamma_of_0(n_0.lo_0 + u32(1)), s_3) - n_0.upper_0) * n_0.a_0));
    }
    else
    {
        if((n_0.mode_0) == u32(0))
        {
            l_1 = max(0.0f, _S416 + - s_3.detail_0 * n_0.sigma_r_0 * 0.039999999105930328f * n_0.grid_1);
        }
        else
        {
            l_1 = _S416;
        }
    }
    return lab_to_rgb_0(vec3<f32>(l_1, lab_2.y, lab_2.z), c_8);
}

fn s_fwd_bilat_0( lab_3 : vec3<f32>,  n_1 : Neighbourhood_0,  c_9 : Colour_0,  dps_1 : DiffPair_Settings_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S417 : f32 = lab_3.x;
    var l_2 : f32;
    var s_diff_l_0 : f32;
    if((n_1.mode_0) == u32(1))
    {
        var v_4 : f32 = _S417 * 0.0099999997764825821f;
        var _S418 : DiffPair_Settings_0 = DiffPair_Settings_0( dps_1.primal_0, dps_1.differential_0 );
        var _S419 : DiffPair_float_0 = s_fwd_curve_0(v_4, gamma_of_0(n_1.lo_0), _S418);
        var _S420 : DiffPair_float_0 = s_fwd_curve_0(v_4, gamma_of_0(n_1.lo_0 + u32(1)), _S418);
        var _S421 : f32 = 1.0f - n_1.a_0;
        var _S422 : f32 = (_S419.differential_0 * _S421 + _S420.differential_0 * n_1.a_0) * 100.0f;
        l_2 = 100.0f * (n_1.output_1 + ((_S419.primal_0 - n_1.lower_0) * _S421 + (_S420.primal_0 - n_1.upper_0) * n_1.a_0));
        s_diff_l_0 = _S422;
    }
    else
    {
        if((n_1.mode_0) == u32(0))
        {
            var _S423 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
            var _S424 : DiffPair_float_0 = DiffPair_float_0( _S417 + - dps_1.primal_0.detail_0 * n_1.sigma_r_0 * 0.039999999105930328f * n_1.grid_1, - dps_1.differential_0.detail_0 * n_1.sigma_r_0 * 0.039999999105930328f * n_1.grid_1 );
            var _S425 : DiffPair_float_0 = _d_max_1(_S423, _S424);
            l_2 = _S425.primal_0;
            s_diff_l_0 = _S425.differential_0;
        }
        else
        {
            l_2 = _S417;
            s_diff_l_0 = 0.0f;
        }
    }
    var _S426 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(l_2, lab_3.y, lab_3.z), vec3<f32>(s_diff_l_0, 0.0f, 0.0f) );
    var _S427 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_lab_to_rgb_0(_S426, c_9);
    var _S428 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S427.primal_0, _S427.differential_0 );
    return _S428;
}

fn main_settings_0() -> Settings_0
{
    var s_4 : Settings_0 = settings_of_0();
    if((params_0.bilat_0.image_0.w) == u32(0))
    {
        s_4.detail_0 = params_0.bilat_0.slice_0.x;
    }
    return s_4;
}

fn slice_1( _S429 : u32,  _S430 : u32,  _S431 : f32) -> f32
{
    var _S432 : f32 = params_0.bilat_0.grid_0.x;
    var _S433 : u32 = params_0.bilat_0.grid_size_0.x;
    var gx_1 : GridCoordinate_0 = grid_coordinate_0(f32(_S429) * _S432, _S433);
    var gy_1 : GridCoordinate_0 = grid_coordinate_0(f32(_S430) * _S432, params_0.bilat_0.grid_size_0.y);
    var _S434 : u32 = params_0.bilat_0.grid_size_0.z;
    var gz_1 : GridCoordinate_0 = grid_coordinate_0(_S431 * params_0.bilat_0.grid_0.y, _S434);
    var gi_0 : u32 = node_0(gx_1.cell_0, gy_1.cell_0, gz_1.cell_0, params_0.bilat_0.grid_size_0);
    var oy_0 : u32 = _S433 * _S434;
    var _S435 : f32 = 1.0f - gx_1.fraction_0;
    var _S436 : f32 = 1.0f - gy_1.fraction_0;
    var _S437 : f32 = 1.0f - gz_1.fraction_0;
    var _S438 : u32 = gi_0 + _S434;
    var _S439 : u32 = gi_0 + oy_0;
    var _S440 : u32 = _S438 + oy_0;
    return coarse_1[gi_0] * _S435 * _S436 * _S437 + coarse_1[_S438] * gx_1.fraction_0 * _S436 * _S437 + coarse_1[_S439] * _S435 * gy_1.fraction_0 * _S437 + coarse_1[_S440] * gx_1.fraction_0 * gy_1.fraction_0 * _S437 + coarse_1[gi_0 + u32(1)] * _S435 * _S436 * gz_1.fraction_0 + coarse_1[_S438 + u32(1)] * gx_1.fraction_0 * _S436 * gz_1.fraction_0 + coarse_1[_S439 + u32(1)] * _S435 * gy_1.fraction_0 * gz_1.fraction_0 + coarse_1[_S440 + u32(1)] * gx_1.fraction_0 * gy_1.fraction_0 * gz_1.fraction_0;
}

fn neighbourhood_0( _S441 : u32,  _S442 : u32,  _S443 : f32) -> Neighbourhood_0
{
    var n_2 : Neighbourhood_0;
    var _S444 : vec4<u32> = params_0.bilat_0.image_0;
    var _S445 : u32 = params_0.bilat_0.image_0.w;
    n_2.mode_0 = _S445;
    n_2.lo_0 = u32(0);
    n_2.a_0 = 0.0f;
    n_2.output_1 = 0.0f;
    n_2.lower_0 = 0.0f;
    n_2.upper_0 = 0.0f;
    n_2.grid_1 = 0.0f;
    n_2.sigma_r_0 = params_0.bilat_0.grid_0.w;
    if(_S445 == u32(1))
    {
        var max_supp_1 : u32 = _S444.z;
        var ci_1 : u32 = expand_clamp_0(_S441 + max_supp_1, params_0.bilat_0.fine_0.x);
        var cj_1 : u32 = expand_clamp_0(_S442 + max_supp_1, params_0.bilat_0.fine_0.y);
        var cw_2 : u32 = params_0.bilat_0.coarse_0.x;
        var b_4 : Bracket_0 = bracket_0(_S443 * 0.0099999997764825821f);
        n_2.lo_0 = b_4.lo_1;
        n_2.a_0 = b_4.a_3;
        var _S446 : u32 = params_0.bilat_0.coarse_0.z;
        n_2.output_1 = expand_0(u32(7) * _S446, ci_1, cj_1, cw_2);
        var _S447 : u32 = u32(1) + b_4.lo_1;
        n_2.lower_0 = expand_0(_S447 * _S446, ci_1, cj_1, cw_2);
        n_2.upper_0 = expand_0((_S447 + u32(1)) * _S446, ci_1, cj_1, cw_2);
    }
    else
    {
        if((n_2.mode_0) == u32(0))
        {
            n_2.grid_1 = slice_1(_S441, _S442, _S443);
        }
    }
    return n_2;
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_5 : vec3<u32>)
{
    var _S448 : u32 = id_5.x;
    var _S449 : bool;
    if((_S448 >= (params_0.pass_0.width_0)))
    {
        _S449 = true;
    }
    else
    {
        _S449 = ((id_5.y) >= (params_0.pass_0.height_0));
    }
    if(_S449)
    {
        return;
    }
    var _S450 : u32 = id_5.y;
    var i_2 : u32 = _S450 * params_0.pass_0.width_0 + _S448;
    var _S451 : u32 = params_0.pass_0.origin_x_0 + _S448;
    var _S452 : u32 = params_0.pass_0.origin_y_0 + _S450;
    var _S453 : Colour_0 = colour_of_0();
    var _S454 : Settings_0 = main_settings_0();
    var pixel_0 : vec4<f32> = input_0[i_2];
    var rgb_1 : vec3<f32> = pixel_0.xyz;
    var k_3 : u32 = u32(0);
    loop
    {
        if((k_3 < (params_0.pass_0.repeat_0)))
        {
        }
        else
        {
            break;
        }
        var lab_4 : vec3<f32> = rgb_to_lab_0(rgb_1, _S453);
        var _S455 : vec3<f32> = bilat_1(lab_4, neighbourhood_0(_S451, _S452, lab_4.x), _S453, _S454);
        var _S456 : u32 = k_3 + u32(1);
        rgb_1 = _S455;
        k_3 = _S456;
    }
    output_0[i_2] = vec4<f32>(rgb_1, pixel_0.w);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn bilat_backward(@builtin(global_invocation_id) id_6 : vec3<u32>)
{
    var _S457 : u32 = id_6.x;
    var _S458 : bool;
    if((_S457 >= (params_0.pass_0.width_0)))
    {
        _S458 = true;
    }
    else
    {
        _S458 = ((id_6.y) >= (params_0.pass_0.height_0));
    }
    if(_S458)
    {
        return;
    }
    var _S459 : u32 = id_6.y;
    var i_3 : u32 = _S459 * params_0.pass_0.width_0 + _S457;
    var _S460 : Colour_0 = colour_of_0();
    var lab_5 : vec3<f32> = rgb_to_lab_0(input_0[i_3].xyz, _S460);
    var _S461 : Neighbourhood_0 = neighbourhood_0(params_0.pass_0.origin_x_0 + _S457, params_0.pass_0.origin_y_0 + _S459, lab_5.x);
    var _S462 : Settings_0 = Settings_x24_syn_dzero_0();
    var s_5 : DiffPair_Settings_0;
    s_5.primal_0 = main_settings_0();
    s_5.differential_0 = _S462;
    _S268(lab_5, _S461, _S460, &(s_5), derivative_in_0[i_3].xyz);
    derivative_out_0[i_3] = vec4<f32>(s_5.differential_0.detail_0, s_5.differential_0.shadows_0, s_5.differential_0.highlights_0, s_5.differential_0.midtone_0);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn bilat_forward_diff(@builtin(global_invocation_id) id_7 : vec3<u32>)
{
    var _S463 : u32 = id_7.x;
    var _S464 : bool;
    if((_S463 >= (params_0.pass_0.width_0)))
    {
        _S464 = true;
    }
    else
    {
        _S464 = ((id_7.y) >= (params_0.pass_0.height_0));
    }
    if(_S464)
    {
        return;
    }
    var _S465 : u32 = id_7.y;
    var i_4 : u32 = _S465 * params_0.pass_0.width_0 + _S463;
    var _S466 : Colour_0 = colour_of_0();
    var lab_6 : vec3<f32> = rgb_to_lab_0(input_0[i_4].xyz, _S466);
    var _S467 : Neighbourhood_0 = neighbourhood_0(params_0.pass_0.origin_x_0 + _S463, params_0.pass_0.origin_y_0 + _S465, lab_6.x);
    var v_5 : vec4<f32> = derivative_in_0[i_4];
    var direction_0 : Settings_0;
    direction_0.detail_0 = v_5.x;
    direction_0.shadows_0 = v_5.y;
    direction_0.highlights_0 = v_5.z;
    direction_0.midtone_0 = v_5.w;
    var _S468 : DiffPair_Settings_0 = DiffPair_Settings_0( main_settings_0(), direction_0 );
    derivative_out_0[i_4] = vec4<f32>(s_fwd_bilat_0(lab_6, _S467, _S466, _S468).differential_0, 0.0f);
    return;
}

