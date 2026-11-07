// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/iop/bloom.c)
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

// Generated from slang/bloom.slang by tools/build-kernels.sh. Do not edit.

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

struct BloomParams_std140_0
{
    @align(16) image_0 : vec4<u32>,
    @align(16) lights_0 : vec4<f32>,
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
    @align(16) bloom_0 : BloomParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(0) @group(0) var<storage, read> input_0 : array<vec4<f32>>;

@binding(11) @group(0) var<storage, read_write> scratch_out_0 : array<f32>;

@binding(10) @group(0) var<storage, read_write> scratch_in_0 : array<f32>;

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

fn lights_1( blurred_0 : f32,  strength_0 : f32) -> f32
{
    return blurred_0;
}

fn s_apply_lights_0( blurred_1 : f32,  strength_1 : f32) -> Tuple_0
{
    var _S8 : Tuple_0 = Tuple_0( lights_1(blurred_1, strength_1) );
    return _S8;
}

struct s_bwdCallableCtx_lights_0
{
     _S9 : f32,
     _S10 : f32,
};

struct DiffPair_float_0
{
     primal_0 : f32,
     differential_0 : f32,
};

fn lights_slope_0( blurred_2 : f32,  strength_2 : f32) -> f32
{
    var _S11 : f32;
    if(((strength_2 + 1.0f) < 100.0f))
    {
        _S11 = blurred_2 * 0.0069314716383814812f;
    }
    else
    {
        _S11 = 0.0f;
    }
    return _S11;
}

fn lights_backward_0( blurred_3 : f32,  strength_3 : ptr<function, DiffPair_float_0>,  d_out_1 : f32)
{
    var _S12 : f32 = lights_slope_0(blurred_3, (*strength_3).primal_0) * d_out_1;
    (*strength_3).primal_0 = (*strength_3).primal_0;
    (*strength_3).differential_0 = _S12;
    return;
}

fn s_bwdProp_lights_0( ctx_1 : s_bwdCallableCtx_lights_0,  d_strength_0 : ptr<function, f32>,  d_out_2 : f32)
{
    var _S13 : DiffPair_float_0;
    _S13.primal_0 = ctx_1._S10;
    lights_backward_0(ctx_1._S9, &(_S13), d_out_2);
    (*d_strength_0) = _S13.differential_0;
    return;
}

struct Colour_0
{
     rgb_to_xyz_0 : mat3x3<f32>,
     xyz_to_rgb_0 : mat3x3<f32>,
};

struct Settings_0
{
     strength_4 : f32,
};

struct DiffPair_Settings_0
{
     primal_0 : Settings_0,
     differential_0 : Settings_0,
};

struct s_paramCtx_s_bwdCallableCtx_bloom_0
{
     value0_1 : vec3<f32>,
     value1_0 : f32,
     value2_0 : Colour_0,
     value3_0 : Settings_0,
};

struct s_bwdCallableCtx_bloom_0
{
     value0_2 : s_paramCtx_s_bwdCallableCtx_bloom_0,
};

struct s_paramCtx_s_bwdCallableCtx_lab_to_rgb_0
{
     value0_3 : vec3<f32>,
     value1_1 : Colour_0,
};

struct s_bwdCallableCtx_lab_to_rgb_0
{
     value0_4 : s_paramCtx_s_bwdCallableCtx_lab_to_rgb_0,
};

fn Settings_x24_syn_dzero_0() -> Settings_0
{
    var result_0 : Settings_0;
    result_0.strength_4 = 0.0f;
    return result_0;
}

struct Tuple_1
{
     value0_5 : vec3<f32>,
};

struct Tuple_2
{
     value0_6 : f32,
};

fn s_apply_lab_f_inv_0( _S14 : f32) -> Tuple_2
{
    var _S15 : bool = (_S14 > 0.20689655840396881f);
    var _S16 : f32;
    if(_S15)
    {
        _S16 = _S14 * _S14 * _S14;
    }
    var _S17 : bool = !_S15;
    if(_S17)
    {
        _S16 = (116.0f * _S14 - 16.0f) / 903.29632568359375f;
    }
    var _S18 : Tuple_2;
    _S18.value0_6 = _S16;
    return _S18;
}

fn s_apply_lab_to_xyz_d50_0( _S19 : vec3<f32>) -> Tuple_1
{
    var fy_0 : f32 = (_S19.x + 16.0f) * 0.0086206896230578423f;
    var _S20 : Tuple_1;
    _S20.value0_5 = vec3<f32>(0.96420001983642578f, 1.0f, 0.82489997148513794f) * vec3<f32>(s_apply_lab_f_inv_0(_S19.y * 0.0020000000949949026f + fy_0).value0_6, s_apply_lab_f_inv_0(fy_0).value0_6, s_apply_lab_f_inv_0(_S19.z * -0.004999999888241291f + fy_0).value0_6);
    return _S20;
}

struct s_paramCtx_s_bwdCallableCtx_lab_to_xyz_d50_0
{
     value0_7 : vec3<f32>,
};

struct s_bwdCallableCtx_lab_to_xyz_d50_0
{
     value0_8 : s_paramCtx_s_bwdCallableCtx_lab_to_xyz_d50_0,
};

struct s_paramCtx_s_bwdCallableCtx_lab_f_inv_0
{
     value0_9 : f32,
};

struct s_bwdCallableCtx_lab_f_inv_0
{
     value0_10 : s_paramCtx_s_bwdCallableCtx_lab_f_inv_0,
};

fn s_bwdProp_lab_f_inv_0( _S21 : s_bwdCallableCtx_lab_f_inv_0,  _S22 : ptr<function, f32>,  _S23 : f32)
{
    var _S24 : bool = ((_S21.value0_10.value0_9) > 0.20689655840396881f);
    var _S25 : f32;
    if(_S24)
    {
        _S25 = _S21.value0_10.value0_9 * _S21.value0_10.value0_9;
    }
    else
    {
        _S25 = 0.0f;
    }
    var _S26 : bool = !_S24;
    var _S27 : f32;
    var _S28 : f32;
    if(_S26)
    {
        var _S29 : f32 = 116.0f * (0.0011070563923567533f * _S23);
        _S27 = 0.0f;
        _S28 = _S29;
    }
    else
    {
        _S27 = _S23;
        _S28 = 0.0f;
    }
    if(_S24)
    {
        var _S30 : f32 = _S21.value0_10.value0_9 * (_S21.value0_10.value0_9 * _S27);
        _S25 = _S25 * _S27 + _S30 + _S30 + _S28;
    }
    else
    {
        _S25 = _S28;
    }
    (*_S22) = _S25;
    return;
}

fn s_bwdProp_lab_to_xyz_d50_0( _S31 : s_bwdCallableCtx_lab_to_xyz_d50_0,  _S32 : ptr<function, vec3<f32>>,  _S33 : vec3<f32>)
{
    var fy_1 : f32 = (_S31.value0_8.value0_7.x + 16.0f) * 0.0086206896230578423f;
    var fz_0 : f32 = _S31.value0_8.value0_7.z * -0.004999999888241291f + fy_1;
    var _S34 : s_paramCtx_s_bwdCallableCtx_lab_f_inv_0;
    _S34.value0_9 = _S31.value0_8.value0_7.y * 0.0020000000949949026f + fy_1;
    var _S35 : s_bwdCallableCtx_lab_f_inv_0 = s_bwdCallableCtx_lab_f_inv_0( _S34 );
    var _S36 : s_paramCtx_s_bwdCallableCtx_lab_f_inv_0;
    _S36.value0_9 = fy_1;
    var _S37 : s_bwdCallableCtx_lab_f_inv_0 = s_bwdCallableCtx_lab_f_inv_0( _S36 );
    var _S38 : s_paramCtx_s_bwdCallableCtx_lab_f_inv_0;
    _S38.value0_9 = fz_0;
    var _S39 : s_bwdCallableCtx_lab_f_inv_0 = s_bwdCallableCtx_lab_f_inv_0( _S38 );
    var _S40 : vec3<f32> = vec3<f32>(0.96420001983642578f, 1.0f, 0.82489997148513794f) * _S33;
    var _S41 : f32 = 0.0f;
    s_bwdProp_lab_f_inv_0(_S39, &(_S41), _S40[i32(2)]);
    var _S42 : f32 = 0.0f;
    s_bwdProp_lab_f_inv_0(_S37, &(_S42), _S40[i32(1)]);
    var _S43 : f32 = 0.0f;
    s_bwdProp_lab_f_inv_0(_S35, &(_S43), _S40[i32(0)]);
    (*_S32) = vec3<f32>(0.0086206896230578423f * (_S42 + _S41 + _S43), 0.0020000000949949026f * _S43, -0.004999999888241291f * _S41);
    return;
}

fn s_bwdProp_lab_to_rgb_0( _S44 : s_bwdCallableCtx_lab_to_rgb_0,  _S45 : ptr<function, vec3<f32>>,  _S46 : vec3<f32>)
{
    var _S47 : Tuple_1 = s_apply_lab_to_xyz_d50_0(_S44.value0_4.value0_3);
    var _S48 : s_paramCtx_s_bwdCallableCtx_lab_to_xyz_d50_0;
    _S48.value0_7 = _S44.value0_4.value0_3;
    var _S49 : s_bwdCallableCtx_lab_to_xyz_d50_0 = s_bwdCallableCtx_lab_to_xyz_d50_0( _S48 );
    var _S50 : s_bwdCallableCtx_mul_0;
    _S50._S1 = _S44.value0_4.value1_1.xyz_to_rgb_0;
    _S50._S2 = _S47.value0_5;
    var _S51 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S52 : vec3<f32> = vec3<f32>(0.0f);
    var _S53 : vec3<f32> = _S52;
    s_bwdProp_mul_0(_S50, &(_S51), &(_S53), _S46);
    var _S54 : vec3<f32> = _S52;
    s_bwdProp_lab_to_xyz_d50_0(_S49, &(_S54), _S53);
    (*_S45) = _S54;
    return;
}

fn Settings_x24_syn_dadd_0( SLANG_anonymous_0_0 : Settings_0,  SLANG_anonymous_1_0 : Settings_0) -> Settings_0
{
    var result_1 : Settings_0;
    result_1.strength_4 = SLANG_anonymous_0_0.strength_4 + SLANG_anonymous_1_0.strength_4;
    return result_1;
}

fn s_bwdProp_bloom_0( _S55 : s_bwdCallableCtx_bloom_0,  _S56 : ptr<function, Settings_0>,  _S57 : vec3<f32>)
{
    var _S58 : Tuple_0 = s_apply_lights_0(_S55.value0_2.value1_0, _S55.value0_2.value3_0.strength_4);
    var _S59 : s_bwdCallableCtx_lights_0;
    _S59._S9 = _S55.value0_2.value1_0;
    _S59._S10 = _S55.value0_2.value3_0.strength_4;
    var _S60 : f32 = 100.0f - _S55.value0_2.value0_1.x;
    var _S61 : s_paramCtx_s_bwdCallableCtx_lab_to_rgb_0;
    _S61.value0_3 = vec3<f32>(100.0f - _S60 * (100.0f - _S58.value0_0) / 100.0f, _S55.value0_2.value0_1.y, _S55.value0_2.value0_1.z);
    _S61.value1_1 = _S55.value0_2.value2_0;
    var _S62 : s_bwdCallableCtx_lab_to_rgb_0 = s_bwdCallableCtx_lab_to_rgb_0( _S61 );
    var _S63 : Settings_0 = Settings_x24_syn_dzero_0();
    var _S64 : vec3<f32> = vec3<f32>(0.0f);
    s_bwdProp_lab_to_rgb_0(_S62, &(_S64), _S57);
    var _S65 : f32 = - (_S60 * (0.0099999997764825821f * - _S64[i32(0)]));
    var _S66 : f32 = 0.0f;
    s_bwdProp_lights_0(_S59, &(_S66), _S65);
    var _S67 : Settings_0 = _S63;
    _S67.strength_4 = _S66;
    (*_S56) = Settings_x24_syn_dadd_0(_S63, _S67);
    return;
}

fn _S68( _S69 : vec3<f32>,  _S70 : f32,  _S71 : Colour_0,  _S72 : ptr<function, DiffPair_Settings_0>,  _S73 : vec3<f32>)
{
    var _S74 : s_paramCtx_s_bwdCallableCtx_bloom_0;
    _S74.value0_1 = _S69;
    _S74.value1_0 = _S70;
    _S74.value2_0 = _S71;
    _S74.value3_0 = (*_S72).primal_0;
    var _S75 : s_bwdCallableCtx_bloom_0 = s_bwdCallableCtx_bloom_0( _S74 );
    var _S76 : Settings_0 = (*_S72).differential_0;
    s_bwdProp_bloom_0(_S75, &(_S76), _S73);
    (*_S72).differential_0 = _S76;
    return;
}

struct Line_0
{
     start_0 : u32,
     stride_0 : u32,
};

fn Line_read_0( this_0 : Line_0,  i_0 : u32) -> f32
{
    return scratch_in_0[this_0.start_0 + i_0 * this_0.stride_0];
}

fn Line_write_0( this_1 : Line_0,  i_1 : u32,  value_0 : f32)
{
    scratch_out_0[this_1.start_0 + i_1 * this_1.stride_0] = value_0;
    return;
}

fn box_mean_line_0( line_0 : Line_0,  count_0 : u32,  radius_0 : u32)
{
    var _S77 : bool;
    var sum_6 : f32 = 0.0f;
    var x_0 : u32 = u32(0);
    var hits_0 : u32 = u32(0);
    loop
    {
        if((x_0 < (min(radius_0, count_0))))
        {
        }
        else
        {
            break;
        }
        var _S78 : u32 = hits_0 + u32(1);
        var _S79 : f32 = sum_6 + Line_read_0(line_0, x_0);
        var _S80 : u32 = x_0 + u32(1);
        sum_6 = _S79;
        x_0 = _S80;
        hits_0 = _S78;
    }
    x_0 = u32(0);
    loop
    {
        if((x_0 <= radius_0))
        {
            _S77 = ((x_0 + radius_0) < count_0);
        }
        else
        {
            _S77 = false;
        }
        if(_S77)
        {
        }
        else
        {
            break;
        }
        var hits_1 : u32 = hits_0 + u32(1);
        var _S81 : f32 = sum_6 + Line_read_0(line_0, x_0 + radius_0);
        Line_write_0(line_0, x_0, _S81 / f32(i32(hits_1)));
        var _S82 : u32 = x_0 + u32(1);
        sum_6 = _S81;
        x_0 = _S82;
        hits_0 = hits_1;
    }
    loop
    {
        if((x_0 <= radius_0))
        {
            _S77 = (x_0 < count_0);
        }
        else
        {
            _S77 = false;
        }
        if(_S77)
        {
        }
        else
        {
            break;
        }
        Line_write_0(line_0, x_0, sum_6 / f32(i32(hits_0)));
        x_0 = x_0 + u32(1);
    }
    loop
    {
        var _S83 : u32 = x_0 + radius_0;
        if((_S83 < count_0))
        {
        }
        else
        {
            break;
        }
        var _S84 : f32 = sum_6 - Line_read_0(line_0, x_0 - radius_0 - u32(1)) + Line_read_0(line_0, _S83);
        Line_write_0(line_0, x_0, _S84 / f32(i32(hits_0)));
        var _S85 : u32 = x_0 + u32(1);
        sum_6 = _S84;
        x_0 = _S85;
    }
    loop
    {
        if((x_0 < count_0))
        {
        }
        else
        {
            break;
        }
        var hits_2 : u32 = hits_0 - u32(1);
        var _S86 : f32 = sum_6 - Line_read_0(line_0, x_0 - radius_0 - u32(1));
        Line_write_0(line_0, x_0, _S86 / f32(i32(hits_2)));
        var _S87 : u32 = x_0 + u32(1);
        sum_6 = _S86;
        x_0 = _S87;
        hits_0 = hits_2;
    }
    return;
}

fn colour_of_0( p_0 : ptr<function, BloomParams_std140_0>) -> Colour_0
{
    var c_0 : Colour_0;
    c_0.rgb_to_xyz_0 = mat3x3<f32>((*p_0).rgb_to_xyz_r0_0.xyz, (*p_0).rgb_to_xyz_r1_0.xyz, (*p_0).rgb_to_xyz_r2_0.xyz);
    c_0.xyz_to_rgb_0 = mat3x3<f32>((*p_0).xyz_to_rgb_r0_0.xyz, (*p_0).xyz_to_rgb_r1_0.xyz, (*p_0).xyz_to_rgb_r2_0.xyz);
    return c_0;
}

fn pow0_0( x_1 : f32,  y_0 : f32) -> f32
{
    if((x_1 <= 0.0f))
    {
        return 0.0f;
    }
    return pow(x_1, y_0);
}

fn lab_f_0( x_2 : f32) -> f32
{
    if((x_2 > 0.0088564520701766014f))
    {
        return pow0_0(x_2, 0.3333333432674408f);
    }
    return (903.29632568359375f * x_2 + 16.0f) / 116.0f;
}

fn xyz_to_lab_d50_0( xyz_0 : vec3<f32>) -> vec3<f32>
{
    var x_3 : vec3<f32> = xyz_0 / vec3<f32>(0.96420001983642578f, 1.0f, 0.82489997148513794f);
    var _S88 : f32 = lab_f_0(x_3.y);
    return vec3<f32>(116.0f * _S88 - 16.0f, 500.0f * (lab_f_0(x_3.x) - _S88), -200.0f * (lab_f_0(x_3.z) - _S88));
}

fn rgb_to_lab_0( rgb_0 : vec3<f32>,  c_1 : Colour_0) -> vec3<f32>
{
    return xyz_to_lab_d50_0((((rgb_0) * (c_1.rgb_to_xyz_0))));
}

fn scratch_index_0( id_0 : vec3<u32>,  p_1 : ptr<function, BloomParams_std140_0>) -> u32
{
    return (params_0.pass_0.origin_y_0 + id_0.y) * (*p_1).image_0.x + params_0.pass_0.origin_x_0 + id_0.x;
}

@compute
@workgroup_size(16, 16, 1)
fn threshold(@builtin(global_invocation_id) id_1 : vec3<u32>)
{
    var _S89 : u32 = id_1.x;
    var _S90 : bool;
    if((_S89 >= (params_0.pass_0.width_0)))
    {
        _S90 = true;
    }
    else
    {
        _S90 = ((id_1.y) >= (params_0.pass_0.height_0));
    }
    if(_S90)
    {
        return;
    }
    var _S91 : BloomParams_std140_0 = params_0.bloom_0;
    var _S92 : vec3<f32> = input_0[id_1.y * params_0.pass_0.width_0 + _S89].xyz;
    var _S93 : Colour_0 = colour_of_0(&(_S91));
    var _S94 : vec4<f32> = _S91.lights_0;
    var _S95 : f32 = min(rgb_to_lab_0(_S92, _S93).x * _S91.lights_0.x, 1.0e+06f);
    var _S96 : u32 = scratch_index_0(id_1, &(_S91));
    var _S97 : f32;
    if((_S95 > (_S94.y)))
    {
        _S97 = _S95;
    }
    else
    {
        _S97 = 0.0f;
    }
    scratch_out_0[_S96] = _S97;
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn box_mean(@builtin(global_invocation_id) id_2 : vec3<u32>)
{
    var p_2 : BloomParams_std140_0 = params_0.bloom_0;
    var width_1 : u32 = params_0.bloom_0.image_0.x;
    var height_1 : u32 = params_0.bloom_0.image_0.y;
    var rows_0 : bool = (params_0.bloom_0.image_0.w) == u32(0);
    var index_0 : u32 = id_2.y * u32(16) + id_2.x;
    var _S98 : u32;
    if(rows_0)
    {
        _S98 = height_1;
    }
    else
    {
        _S98 = width_1;
    }
    if((index_0 >= _S98))
    {
        return;
    }
    var line_1 : Line_0;
    if(rows_0)
    {
        _S98 = index_0 * width_1;
    }
    else
    {
        _S98 = index_0;
    }
    line_1.start_0 = _S98;
    if(rows_0)
    {
        _S98 = u32(1);
    }
    else
    {
        _S98 = width_1;
    }
    line_1.stride_0 = _S98;
    var _S99 : Line_0 = line_1;
    if(rows_0)
    {
        _S98 = width_1;
    }
    else
    {
        _S98 = height_1;
    }
    box_mean_line_0(_S99, _S98, p_2.image_0.z);
    return;
}

fn settings_of_0( p_3 : ptr<function, BloomParams_std140_0>) -> Settings_0
{
    var s_0 : Settings_0;
    s_0.strength_4 = (*p_3).lights_0.z;
    return s_0;
}

fn lights_forward_0( blurred_4 : f32,  strength_5 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S100 : DiffPair_float_0 = DiffPair_float_0( blurred_4, lights_slope_0(blurred_4, strength_5.primal_0) * strength_5.differential_0 );
    return _S100;
}

fn lab_f_inv_0( x_4 : f32) -> f32
{
    if((x_4 > 0.20689655840396881f))
    {
        return x_4 * x_4 * x_4;
    }
    return (116.0f * x_4 - 16.0f) / 903.29632568359375f;
}

fn s_fwd_lab_f_inv_0( dpx_0 : DiffPair_float_0) -> DiffPair_float_0
{
    if(((dpx_0.primal_0) > 0.20689655840396881f))
    {
        var _S101 : f32 = dpx_0.primal_0 * dpx_0.primal_0;
        var _S102 : f32 = dpx_0.differential_0 * dpx_0.primal_0;
        var _S103 : DiffPair_float_0 = DiffPair_float_0( _S101 * dpx_0.primal_0, (_S102 + _S102) * dpx_0.primal_0 + dpx_0.differential_0 * _S101 );
        return _S103;
    }
    var _S104 : DiffPair_float_0 = DiffPair_float_0( (116.0f * dpx_0.primal_0 - 16.0f) / 903.29632568359375f, dpx_0.differential_0 * 116.0f * 0.0011070563923567533f );
    return _S104;
}

fn lab_to_xyz_d50_0( lab_0 : vec3<f32>) -> vec3<f32>
{
    var fy_2 : f32 = (lab_0.x + 16.0f) * 0.0086206896230578423f;
    return vec3<f32>(0.96420001983642578f, 1.0f, 0.82489997148513794f) * vec3<f32>(lab_f_inv_0(lab_0.y * 0.0020000000949949026f + fy_2), lab_f_inv_0(fy_2), lab_f_inv_0(lab_0.z * -0.004999999888241291f + fy_2));
}

fn s_fwd_lab_to_xyz_d50_0( dplab_0 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var fy_3 : f32 = (dplab_0.primal_0.x + 16.0f) * 0.0086206896230578423f;
    var _S105 : f32 = dplab_0.differential_0.x * 0.0086206896230578423f;
    var _S106 : DiffPair_float_0 = DiffPair_float_0( dplab_0.primal_0.y * 0.0020000000949949026f + fy_3, dplab_0.differential_0.y * 0.0020000000949949026f + _S105 );
    var _S107 : DiffPair_float_0 = s_fwd_lab_f_inv_0(_S106);
    var _S108 : DiffPair_float_0 = DiffPair_float_0( fy_3, _S105 );
    var _S109 : DiffPair_float_0 = s_fwd_lab_f_inv_0(_S108);
    var _S110 : DiffPair_float_0 = DiffPair_float_0( dplab_0.primal_0.z * -0.004999999888241291f + fy_3, dplab_0.differential_0.z * -0.004999999888241291f + _S105 );
    var _S111 : DiffPair_float_0 = s_fwd_lab_f_inv_0(_S110);
    var _S112 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.96420001983642578f, 1.0f, 0.82489997148513794f) * vec3<f32>(_S107.primal_0, _S109.primal_0, _S111.primal_0), vec3<f32>(_S107.differential_0, _S109.differential_0, _S111.differential_0) * vec3<f32>(0.96420001983642578f, 1.0f, 0.82489997148513794f) );
    return _S112;
}

fn lab_to_rgb_0( lab_1 : vec3<f32>,  c_2 : Colour_0) -> vec3<f32>
{
    return (((lab_to_xyz_d50_0(lab_1)) * (c_2.xyz_to_rgb_0)));
}

fn s_fwd_lab_to_rgb_0( dplab_1 : DiffPair_vectorx3Cfloatx2C3x3E_0,  c_3 : Colour_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S113 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( dplab_1.primal_0, dplab_1.differential_0 );
    var _S114 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_lab_to_xyz_d50_0(_S113);
    var _S115 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( (((_S114.primal_0) * (c_3.xyz_to_rgb_0))), (((_S114.primal_0) * (mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f)))) + (((_S114.differential_0) * (c_3.xyz_to_rgb_0))) );
    return _S115;
}

fn bloom_1( lab_2 : vec3<f32>,  blurred_5 : f32,  c_4 : Colour_0,  s_1 : Settings_0) -> vec3<f32>
{
    return lab_to_rgb_0(vec3<f32>(100.0f - (100.0f - lab_2.x) * (100.0f - lights_1(blurred_5, s_1.strength_4)) / 100.0f, lab_2.y, lab_2.z), c_4);
}

fn s_fwd_bloom_0( lab_3 : vec3<f32>,  blurred_6 : f32,  c_5 : Colour_0,  dps_0 : DiffPair_Settings_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S116 : DiffPair_float_0 = DiffPair_float_0( dps_0.primal_0.strength_4, dps_0.differential_0.strength_4 );
    var _S117 : DiffPair_float_0 = lights_forward_0(blurred_6, _S116);
    var _S118 : f32 = 100.0f - lab_3.x;
    var _S119 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(100.0f - _S118 * (100.0f - _S117.primal_0) / 100.0f, lab_3.y, lab_3.z), vec3<f32>(0.0f - (0.0f - _S117.differential_0) * _S118 * 0.0099999997764825821f, 0.0f, 0.0f) );
    var _S120 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_lab_to_rgb_0(_S119, c_5);
    var _S121 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S120.primal_0, _S120.differential_0 );
    return _S121;
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_3 : vec3<u32>)
{
    var _S122 : u32 = id_3.x;
    var _S123 : bool;
    if((_S122 >= (params_0.pass_0.width_0)))
    {
        _S123 = true;
    }
    else
    {
        _S123 = ((id_3.y) >= (params_0.pass_0.height_0));
    }
    if(_S123)
    {
        return;
    }
    var _S124 : BloomParams_std140_0 = params_0.bloom_0;
    var i_2 : u32 = id_3.y * params_0.pass_0.width_0 + _S122;
    var _S125 : Colour_0 = colour_of_0(&(_S124));
    var _S126 : Settings_0 = settings_of_0(&(_S124));
    var _S127 : u32 = scratch_index_0(id_3, &(_S124));
    var _S128 : f32 = scratch_in_0[_S127];
    var pixel_0 : vec4<f32> = input_0[i_2];
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
        var _S129 : vec3<f32> = bloom_1(rgb_to_lab_0(rgb_1, _S125), _S128, _S125, _S126);
        var _S130 : u32 = k_0 + u32(1);
        rgb_1 = _S129;
        k_0 = _S130;
    }
    output_0[i_2] = vec4<f32>(rgb_1, pixel_0.w);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn bloom_backward(@builtin(global_invocation_id) id_4 : vec3<u32>)
{
    var _S131 : u32 = id_4.x;
    var _S132 : bool;
    if((_S131 >= (params_0.pass_0.width_0)))
    {
        _S132 = true;
    }
    else
    {
        _S132 = ((id_4.y) >= (params_0.pass_0.height_0));
    }
    if(_S132)
    {
        return;
    }
    var _S133 : BloomParams_std140_0 = params_0.bloom_0;
    var i_3 : u32 = id_4.y * params_0.pass_0.width_0 + _S131;
    var _S134 : Colour_0 = colour_of_0(&(_S133));
    var lab_4 : vec3<f32> = rgb_to_lab_0(input_0[i_3].xyz, _S134);
    var _S135 : u32 = scratch_index_0(id_4, &(_S133));
    var _S136 : Settings_0 = settings_of_0(&(_S133));
    var _S137 : Settings_0 = Settings_x24_syn_dzero_0();
    var s_2 : DiffPair_Settings_0;
    s_2.primal_0 = _S136;
    s_2.differential_0 = _S137;
    _S68(lab_4, scratch_in_0[_S135], _S134, &(s_2), derivative_in_0[i_3].xyz);
    derivative_out_0[i_3] = vec4<f32>(s_2.differential_0.strength_4, 0.0f, 0.0f, 0.0f);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn bloom_forward_diff(@builtin(global_invocation_id) id_5 : vec3<u32>)
{
    var _S138 : u32 = id_5.x;
    var _S139 : bool;
    if((_S138 >= (params_0.pass_0.width_0)))
    {
        _S139 = true;
    }
    else
    {
        _S139 = ((id_5.y) >= (params_0.pass_0.height_0));
    }
    if(_S139)
    {
        return;
    }
    var _S140 : BloomParams_std140_0 = params_0.bloom_0;
    var i_4 : u32 = id_5.y * params_0.pass_0.width_0 + _S138;
    var _S141 : Colour_0 = colour_of_0(&(_S140));
    var lab_5 : vec3<f32> = rgb_to_lab_0(input_0[i_4].xyz, _S141);
    var _S142 : u32 = scratch_index_0(id_5, &(_S140));
    var direction_0 : Settings_0;
    direction_0.strength_4 = derivative_in_0[i_4].x;
    var _S143 : Settings_0 = settings_of_0(&(_S140));
    var _S144 : DiffPair_Settings_0 = DiffPair_Settings_0( _S143, direction_0 );
    derivative_out_0[i_4] = vec4<f32>(s_fwd_bloom_0(lab_5, scratch_in_0[_S142], _S141, _S144).differential_0, 0.0f);
    return;
}

