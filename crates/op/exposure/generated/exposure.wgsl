// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/iop/exposure.c)
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

// Generated from slang/exposure.slang by tools/build-kernels.sh. Do not edit.

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

struct ExposureParams_std140_0
{
    @align(16) black_0 : f32,
    @align(4) exposure_0 : f32,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) exposure_1 : ExposureParams_std140_0,
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

fn s_apply_exp2_0( x_0 : f32) -> Tuple_0
{
    var _S1 : Tuple_0 = Tuple_0( exp2(x_0) );
    return _S1;
}

struct s_bwdCallableCtx_exp2_0
{
     _S2 : f32,
};

struct DiffPair_float_0
{
     primal_0 : f32,
     differential_0 : f32,
};

fn _d_exp2_0( dpx_0 : ptr<function, DiffPair_float_0>,  dOut_0 : f32)
{
    var _S3 : f32 = exp2((*dpx_0).primal_0) * 0.69314718246459961f * dOut_0;
    (*dpx_0).primal_0 = (*dpx_0).primal_0;
    (*dpx_0).differential_0 = _S3;
    return;
}

fn s_bwdProp_exp2_0( ctx_0 : s_bwdCallableCtx_exp2_0,  d_x_0 : ptr<function, f32>,  d_out_0 : f32)
{
    var _S4 : DiffPair_float_0;
    _S4.primal_0 = ctx_0._S2;
    _d_exp2_0(&(_S4), d_out_0);
    (*d_x_0) = _S4.differential_0;
    return;
}

struct ExposureParams_0
{
     black_0 : f32,
     exposure_0 : f32,
};

struct DiffPair_ExposureParams_0
{
     primal_0 : ExposureParams_0,
     differential_0 : ExposureParams_0,
};

struct s_paramCtx_s_bwdCallableCtx_exposure_from_params_0
{
     value0_1 : vec3<f32>,
     value1_0 : ExposureParams_0,
};

struct s_bwdCallableCtx_exposure_from_params_0
{
     value0_2 : s_paramCtx_s_bwdCallableCtx_exposure_from_params_0,
};

struct Tuple_1
{
     value0_3 : f32,
};

fn s_apply_exposure_setup_0( _S5 : ExposureParams_0) -> Tuple_1
{
    var _S6 : Tuple_1;
    _S6.value0_3 = 1.0f / (s_apply_exp2_0(- _S5.exposure_0).value0_0 - _S5.black_0);
    return _S6;
}

struct s_paramCtx_s_bwdCallableCtx_exposure_setup_0
{
     value0_4 : ExposureParams_0,
};

struct s_bwdCallableCtx_exposure_setup_0
{
     value0_5 : s_paramCtx_s_bwdCallableCtx_exposure_setup_0,
};

struct s_paramCtx_s_bwdCallableCtx_exposure_0
{
     value0_6 : vec3<f32>,
     value1_1 : ExposureParams_0,
     value2_0 : f32,
};

struct s_bwdCallableCtx_exposure_0
{
     value0_7 : s_paramCtx_s_bwdCallableCtx_exposure_0,
};

fn ExposureParams_x24_syn_dzero_0() -> ExposureParams_0
{
    var result_0 : ExposureParams_0;
    result_0.black_0 = 0.0f;
    result_0.exposure_0 = 0.0f;
    return result_0;
}

fn ExposureParams_x24_syn_dadd_0( SLANG_anonymous_0_0 : ExposureParams_0,  SLANG_anonymous_1_0 : ExposureParams_0) -> ExposureParams_0
{
    var result_1 : ExposureParams_0;
    result_1.black_0 = SLANG_anonymous_0_0.black_0 + SLANG_anonymous_1_0.black_0;
    result_1.exposure_0 = SLANG_anonymous_0_0.exposure_0 + SLANG_anonymous_1_0.exposure_0;
    return result_1;
}

fn s_bwdProp_exposure_0( _S7 : s_bwdCallableCtx_exposure_0,  _S8 : ptr<function, vec3<f32>>,  _S9 : ptr<function, ExposureParams_0>,  _S10 : ptr<function, f32>,  _S11 : vec3<f32>)
{
    var _S12 : ExposureParams_0 = ExposureParams_x24_syn_dzero_0();
    var _S13 : vec3<f32> = (_S7.value0_7.value0_6 - vec3<f32>(_S7.value0_7.value1_1.black_0)) * _S11;
    var _S14 : f32 = _S13[i32(0)] + _S13[i32(1)] + _S13[i32(2)];
    var _S15 : vec3<f32> = vec3<f32>(_S7.value0_7.value2_0) * _S11;
    var _S16 : vec3<f32> = (vec3<f32>(0) - _S15);
    var _S17 : f32 = _S16[i32(0)] + _S16[i32(1)] + _S16[i32(2)];
    var _S18 : ExposureParams_0 = _S12;
    _S18.black_0 = _S17;
    var _S19 : ExposureParams_0 = ExposureParams_x24_syn_dadd_0(_S12, _S18);
    (*_S8) = _S15;
    (*_S9) = _S19;
    (*_S10) = _S14;
    return;
}

fn s_bwdProp_exposure_setup_0( _S20 : s_bwdCallableCtx_exposure_setup_0,  _S21 : ptr<function, ExposureParams_0>,  _S22 : f32)
{
    var _S23 : f32 = - _S20.value0_5.value0_4.exposure_0;
    var _S24 : Tuple_0 = s_apply_exp2_0(_S23);
    var _S25 : s_bwdCallableCtx_exp2_0;
    _S25._S2 = _S23;
    var _S26 : f32 = _S24.value0_0 - _S20.value0_5.value0_4.black_0;
    var _S27 : ExposureParams_0 = ExposureParams_x24_syn_dzero_0();
    var _S28 : f32 = - (_S22 / (_S26 * _S26));
    var _S29 : f32 = - _S28;
    var _S30 : f32 = 0.0f;
    s_bwdProp_exp2_0(_S25, &(_S30), _S28);
    var _S31 : f32 = - _S30;
    var _S32 : ExposureParams_0 = _S27;
    _S32.black_0 = _S29;
    _S32.exposure_0 = _S31;
    (*_S21) = ExposureParams_x24_syn_dadd_0(_S27, _S32);
    return;
}

fn s_bwdProp_exposure_from_params_0( _S33 : s_bwdCallableCtx_exposure_from_params_0,  _S34 : ptr<function, ExposureParams_0>,  _S35 : vec3<f32>)
{
    var _S36 : Tuple_1 = s_apply_exposure_setup_0(_S33.value0_2.value1_0);
    var _S37 : s_paramCtx_s_bwdCallableCtx_exposure_setup_0;
    _S37.value0_4 = _S33.value0_2.value1_0;
    var _S38 : s_bwdCallableCtx_exposure_setup_0 = s_bwdCallableCtx_exposure_setup_0( _S37 );
    var _S39 : s_paramCtx_s_bwdCallableCtx_exposure_0;
    _S39.value0_6 = _S33.value0_2.value0_1;
    _S39.value1_1 = _S33.value0_2.value1_0;
    _S39.value2_0 = _S36.value0_3;
    var _S40 : s_bwdCallableCtx_exposure_0 = s_bwdCallableCtx_exposure_0( _S39 );
    var _S41 : ExposureParams_0 = ExposureParams_x24_syn_dzero_0();
    var _S42 : vec3<f32> = vec3<f32>(0.0f);
    var _S43 : ExposureParams_0;
    _S43.black_0 = 0.0f;
    _S43.exposure_0 = 0.0f;
    var _S44 : f32 = 0.0f;
    s_bwdProp_exposure_0(_S40, &(_S42), &(_S43), &(_S44), _S35);
    var _S45 : ExposureParams_0;
    _S45.black_0 = 0.0f;
    _S45.exposure_0 = 0.0f;
    s_bwdProp_exposure_setup_0(_S38, &(_S45), _S44);
    (*_S34) = ExposureParams_x24_syn_dadd_0(ExposureParams_x24_syn_dadd_0(_S43, _S45), _S41);
    return;
}

fn _S46( _S47 : vec3<f32>,  _S48 : ptr<function, DiffPair_ExposureParams_0>,  _S49 : vec3<f32>)
{
    var _S50 : s_paramCtx_s_bwdCallableCtx_exposure_from_params_0;
    _S50.value0_1 = _S47;
    _S50.value1_0 = (*_S48).primal_0;
    var _S51 : s_bwdCallableCtx_exposure_from_params_0 = s_bwdCallableCtx_exposure_from_params_0( _S50 );
    var _S52 : ExposureParams_0 = (*_S48).differential_0;
    s_bwdProp_exposure_from_params_0(_S51, &(_S52), _S49);
    (*_S48).differential_0 = _S52;
    return;
}

fn _d_exp2_1( dpx_1 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S53 : f32 = exp2(dpx_1.primal_0);
    var _S54 : DiffPair_float_0 = DiffPair_float_0( _S53, _S53 * 0.69314718246459961f * dpx_1.differential_0 );
    return _S54;
}

fn exposure_setup_0( p_0 : ptr<function, ExposureParams_std140_0>) -> f32
{
    return 1.0f / (exp2(- (*p_0).exposure_0) - (*p_0).black_0);
}

fn s_fwd_exposure_setup_0( dpp_0 : DiffPair_ExposureParams_0) -> DiffPair_float_0
{
    var _S55 : DiffPair_float_0 = DiffPair_float_0( - dpp_0.primal_0.exposure_0, - dpp_0.differential_0.exposure_0 );
    var _S56 : DiffPair_float_0 = _d_exp2_1(_S55);
    var _S57 : f32 = _S56.primal_0 - dpp_0.primal_0.black_0;
    var _S58 : DiffPair_float_0 = DiffPair_float_0( 1.0f / _S57, (0.0f - (_S56.differential_0 - dpp_0.differential_0.black_0)) / (_S57 * _S57) );
    return _S58;
}

fn exposure_2( rgb_0 : vec3<f32>,  p_1 : ptr<function, ExposureParams_std140_0>,  scale_1 : f32) -> vec3<f32>
{
    return (rgb_0 - vec3<f32>((*p_1).black_0)) * vec3<f32>(scale_1);
}

struct DiffPair_vectorx3Cfloatx2C3x3E_0
{
     primal_0 : vec3<f32>,
     differential_0 : vec3<f32>,
};

fn s_fwd_exposure_0( dprgb_0 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpp_1 : DiffPair_ExposureParams_0,  dpscale_0 : DiffPair_float_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S59 : vec3<f32> = dprgb_0.primal_0 - vec3<f32>(dpp_1.primal_0.black_0);
    var _S60 : vec3<f32> = vec3<f32>(dpscale_0.primal_0);
    var _S61 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S59 * _S60, (dprgb_0.differential_0 - vec3<f32>(dpp_1.differential_0.black_0)) * _S60 + vec3<f32>(dpscale_0.differential_0) * _S59 );
    return _S61;
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_0 : vec3<u32>)
{
    var _S62 : u32 = id_0.x;
    var _S63 : bool;
    if((_S62 >= (params_0.pass_0.width_0)))
    {
        _S63 = true;
    }
    else
    {
        _S63 = ((id_0.y) >= (params_0.pass_0.height_0));
    }
    if(_S63)
    {
        return;
    }
    var i_0 : u32 = id_0.y * params_0.pass_0.width_0 + _S62;
    var _S64 : ExposureParams_std140_0 = params_0.exposure_1;
    var _S65 : f32 = exposure_setup_0(&(_S64));
    var pixel_0 : vec4<f32> = input_0[i_0];
    var rgb_1 : vec3<f32> = pixel_0.xyz;
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
        var _S66 : vec3<f32> = exposure_2(rgb_1, &(_S64), _S65);
        var _S67 : u32 = k_0 + u32(1);
        rgb_1 = _S66;
        k_0 = _S67;
    }
    output_0[i_0] = vec4<f32>(rgb_1, pixel_0.w);
    return;
}

fn s_fwd_exposure_from_params_0( rgb_2 : vec3<f32>,  dpp_2 : DiffPair_ExposureParams_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S68 : DiffPair_ExposureParams_0 = DiffPair_ExposureParams_0( dpp_2.primal_0, dpp_2.differential_0 );
    var _S69 : DiffPair_float_0 = s_fwd_exposure_setup_0(_S68);
    var _S70 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( rgb_2, vec3<f32>(0.0f) );
    var _S71 : DiffPair_float_0 = DiffPair_float_0( _S69.primal_0, _S69.differential_0 );
    var _S72 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_exposure_0(_S70, _S68, _S71);
    var _S73 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S72.primal_0, _S72.differential_0 );
    return _S73;
}

@compute
@workgroup_size(16, 16, 1)
fn exposure_backward(@builtin(global_invocation_id) id_1 : vec3<u32>)
{
    var _S74 : u32 = id_1.x;
    var _S75 : bool;
    if((_S74 >= (params_0.pass_0.width_0)))
    {
        _S75 = true;
    }
    else
    {
        _S75 = ((id_1.y) >= (params_0.pass_0.height_0));
    }
    if(_S75)
    {
        return;
    }
    var i_1 : u32 = id_1.y * params_0.pass_0.width_0 + _S74;
    var _S76 : ExposureParams_0 = ExposureParams_0( params_0.exposure_1.black_0, params_0.exposure_1.exposure_0 );
    var _S77 : ExposureParams_0 = ExposureParams_x24_syn_dzero_0();
    var p_2 : DiffPair_ExposureParams_0;
    p_2.primal_0 = _S76;
    p_2.differential_0 = _S77;
    _S46(input_0[i_1].xyz, &(p_2), derivative_in_0[i_1].xyz);
    derivative_out_0[i_1] = vec4<f32>(p_2.differential_0.black_0, p_2.differential_0.exposure_0, 0.0f, 0.0f);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn exposure_forward_diff(@builtin(global_invocation_id) id_2 : vec3<u32>)
{
    var _S78 : u32 = id_2.x;
    var _S79 : bool;
    if((_S78 >= (params_0.pass_0.width_0)))
    {
        _S79 = true;
    }
    else
    {
        _S79 = ((id_2.y) >= (params_0.pass_0.height_0));
    }
    if(_S79)
    {
        return;
    }
    var i_2 : u32 = id_2.y * params_0.pass_0.width_0 + _S78;
    var direction_0 : ExposureParams_0;
    direction_0.black_0 = derivative_in_0[i_2].x;
    direction_0.exposure_0 = derivative_in_0[i_2].y;
    var _S80 : ExposureParams_0 = ExposureParams_0( params_0.exposure_1.black_0, params_0.exposure_1.exposure_0 );
    var _S81 : DiffPair_ExposureParams_0 = DiffPair_ExposureParams_0( _S80, direction_0 );
    derivative_out_0[i_2] = vec4<f32>(s_fwd_exposure_from_params_0(input_0[i_2].xyz, _S81).differential_0, 0.0f);
    return;
}

