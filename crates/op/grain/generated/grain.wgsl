// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2024  darktable developers (ported from src/iop/grain.c)
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

// Generated from slang/grain.slang by tools/build-kernels.sh. Do not edit.

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

struct GrainParams_std140_0
{
    @align(16) strength_0 : f32,
    @align(4) scale_1 : f32,
    @align(8) midtones_bias_0 : f32,
    @align(4) pad0_0 : u32,
    @align(16) offset_0 : vec4<f32>,
    @align(16) rgb_to_xyz_r0_0 : vec4<f32>,
    @align(16) rgb_to_xyz_r1_0 : vec4<f32>,
    @align(16) rgb_to_xyz_r2_0 : vec4<f32>,
    @align(16) xyz_to_rgb_r0_0 : vec4<f32>,
    @align(16) xyz_to_rgb_r1_0 : vec4<f32>,
    @align(16) xyz_to_rgb_r2_0 : vec4<f32>,
    @align(16) gradient_0 : vec4<u32>,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) grain_0 : GrainParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(3) @group(0) var<storage, read> permutation_0 : array<u32>;

@binding(0) @group(0) var<storage, read> input_0 : array<vec4<f32>>;

@binding(1) @group(0) var<storage, read_write> output_0 : array<vec4<f32>>;

@binding(8) @group(0) var<storage, read> derivative_in_0 : array<vec4<f32>>;

@binding(9) @group(0) var<storage, read_write> derivative_out_0 : array<vec4<f32>>;

struct Tuple_0
{
     value0_0 : vec3<f32>,
};

fn s_apply_mul_0( left_0 : mat3x3<f32>,  right_0 : vec3<f32>) -> Tuple_0
{
    var _S1 : Tuple_0 = Tuple_0( (((right_0) * (left_0))) );
    return _S1;
}

struct s_bwdCallableCtx_mul_0
{
     _S2 : mat3x3<f32>,
     _S3 : vec3<f32>,
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

fn _d_mul_0( left_1 : ptr<function, DiffPair_matrixx3Cfloatx2C3x2C3x3E_0>,  right_1 : ptr<function, DiffPair_vectorx3Cfloatx2C3x3E_0>,  dOut_0 : vec3<f32>)
{
    var _S4 : f32 = (*left_1).primal_0[i32(0)][i32(0)] * dOut_0[i32(0)];
    var left_d_result_0 : mat3x3<f32>;
    left_d_result_0[i32(0)][i32(0)] = (*right_1).primal_0[i32(0)] * dOut_0[i32(0)];
    var sum_0 : f32 = _S4 + (*left_1).primal_0[i32(1)][i32(0)] * dOut_0[i32(1)];
    left_d_result_0[i32(1)][i32(0)] = (*right_1).primal_0[i32(0)] * dOut_0[i32(1)];
    var sum_1 : f32 = sum_0 + (*left_1).primal_0[i32(2)][i32(0)] * dOut_0[i32(2)];
    left_d_result_0[i32(2)][i32(0)] = (*right_1).primal_0[i32(0)] * dOut_0[i32(2)];
    var right_d_result_0 : vec3<f32>;
    right_d_result_0[i32(0)] = sum_1;
    var _S5 : f32 = (*left_1).primal_0[i32(0)][i32(1)] * dOut_0[i32(0)];
    left_d_result_0[i32(0)][i32(1)] = (*right_1).primal_0[i32(1)] * dOut_0[i32(0)];
    var sum_2 : f32 = _S5 + (*left_1).primal_0[i32(1)][i32(1)] * dOut_0[i32(1)];
    left_d_result_0[i32(1)][i32(1)] = (*right_1).primal_0[i32(1)] * dOut_0[i32(1)];
    var sum_3 : f32 = sum_2 + (*left_1).primal_0[i32(2)][i32(1)] * dOut_0[i32(2)];
    left_d_result_0[i32(2)][i32(1)] = (*right_1).primal_0[i32(1)] * dOut_0[i32(2)];
    right_d_result_0[i32(1)] = sum_3;
    var _S6 : f32 = (*left_1).primal_0[i32(0)][i32(2)] * dOut_0[i32(0)];
    left_d_result_0[i32(0)][i32(2)] = (*right_1).primal_0[i32(2)] * dOut_0[i32(0)];
    var sum_4 : f32 = _S6 + (*left_1).primal_0[i32(1)][i32(2)] * dOut_0[i32(1)];
    left_d_result_0[i32(1)][i32(2)] = (*right_1).primal_0[i32(2)] * dOut_0[i32(1)];
    var sum_5 : f32 = sum_4 + (*left_1).primal_0[i32(2)][i32(2)] * dOut_0[i32(2)];
    left_d_result_0[i32(2)][i32(2)] = (*right_1).primal_0[i32(2)] * dOut_0[i32(2)];
    right_d_result_0[i32(2)] = sum_5;
    (*left_1).primal_0 = (*left_1).primal_0;
    (*left_1).differential_0 = left_d_result_0;
    (*right_1).primal_0 = (*right_1).primal_0;
    (*right_1).differential_0 = right_d_result_0;
    return;
}

fn s_bwdProp_mul_0( ctx_0 : s_bwdCallableCtx_mul_0,  d_left_0 : ptr<function, mat3x3<f32>>,  d_right_0 : ptr<function, vec3<f32>>,  d_out_0 : vec3<f32>)
{
    var _S7 : DiffPair_matrixx3Cfloatx2C3x2C3x3E_0;
    _S7.primal_0 = ctx_0._S2;
    var _S8 : DiffPair_vectorx3Cfloatx2C3x3E_0;
    _S8.primal_0 = ctx_0._S3;
    _d_mul_0(&(_S7), &(_S8), d_out_0);
    (*d_left_0) = _S7.differential_0;
    (*d_right_0) = _S8.differential_0;
    return;
}

struct Tuple_1
{
     value0_1 : f32,
};

fn s_apply_clamp_0( x_0 : f32,  minBound_0 : f32,  maxBound_0 : f32) -> Tuple_1
{
    var _S9 : Tuple_1 = Tuple_1( clamp(x_0, minBound_0, maxBound_0) );
    return _S9;
}

struct s_bwdCallableCtx_clamp_0
{
     _S10 : f32,
     _S11 : f32,
     _S12 : f32,
};

struct DiffPair_float_0
{
     primal_0 : f32,
     differential_0 : f32,
};

fn _d_clamp_0( dpx_0 : ptr<function, DiffPair_float_0>,  dpMin_0 : ptr<function, DiffPair_float_0>,  dpMax_0 : ptr<function, DiffPair_float_0>,  dOut_1 : f32)
{
    var _S13 : DiffPair_float_0 = (*dpx_0);
    var _S14 : bool;
    if((((*dpx_0).primal_0) >= ((*dpMin_0).primal_0)))
    {
        _S14 = (((*dpx_0).primal_0) <= ((*dpMax_0).primal_0));
    }
    else
    {
        _S14 = false;
    }
    var _S15 : f32;
    if(_S14)
    {
        _S15 = dOut_1;
    }
    else
    {
        _S15 = 0.0f;
    }
    (*dpx_0).primal_0 = _S13.primal_0;
    (*dpx_0).differential_0 = _S15;
    var _S16 : DiffPair_float_0 = (*dpMin_0);
    if(((_S13.primal_0) < ((*dpMin_0).primal_0)))
    {
        _S15 = dOut_1;
    }
    else
    {
        _S15 = 0.0f;
    }
    (*dpMin_0).primal_0 = _S16.primal_0;
    (*dpMin_0).differential_0 = _S15;
    var _S17 : DiffPair_float_0 = (*dpMax_0);
    if((((*dpx_0).primal_0) > ((*dpMax_0).primal_0)))
    {
        _S15 = dOut_1;
    }
    else
    {
        _S15 = 0.0f;
    }
    (*dpMax_0).primal_0 = _S17.primal_0;
    (*dpMax_0).differential_0 = _S15;
    return;
}

fn s_bwdProp_clamp_0( ctx_1 : s_bwdCallableCtx_clamp_0,  d_x_0 : ptr<function, f32>,  d_minBound_0 : ptr<function, f32>,  d_maxBound_0 : ptr<function, f32>,  d_out_1 : f32)
{
    var _S18 : DiffPair_float_0;
    _S18.primal_0 = ctx_1._S10;
    var _S19 : DiffPair_float_0;
    _S19.primal_0 = ctx_1._S11;
    var _S20 : DiffPair_float_0;
    _S20.primal_0 = ctx_1._S12;
    _d_clamp_0(&(_S18), &(_S19), &(_S20), d_out_1);
    (*d_x_0) = _S18.differential_0;
    (*d_minBound_0) = _S19.differential_0;
    (*d_maxBound_0) = _S20.differential_0;
    return;
}

struct Tuple_2
{
     value0_2 : f32,
};

fn s_apply_exp_0( x_1 : f32) -> Tuple_2
{
    var _S21 : Tuple_2 = Tuple_2( exp(x_1) );
    return _S21;
}

struct s_bwdCallableCtx_exp_0
{
     _S22 : f32,
};

fn _d_exp_0( dpx_1 : ptr<function, DiffPair_float_0>,  dOut_2 : f32)
{
    var _S23 : f32 = exp((*dpx_1).primal_0) * dOut_2;
    (*dpx_1).primal_0 = (*dpx_1).primal_0;
    (*dpx_1).differential_0 = _S23;
    return;
}

fn s_bwdProp_exp_0( ctx_2 : s_bwdCallableCtx_exp_0,  d_x_1 : ptr<function, f32>,  d_out_2 : f32)
{
    var _S24 : DiffPair_float_0;
    _S24.primal_0 = ctx_2._S22;
    _d_exp_0(&(_S24), d_out_2);
    (*d_x_1) = _S24.differential_0;
    return;
}

struct Tuple_3
{
     value0_3 : f32,
};

fn s_apply_log_0( x_2 : f32) -> Tuple_3
{
    var _S25 : Tuple_3 = Tuple_3( log(x_2) );
    return _S25;
}

struct s_bwdCallableCtx_log_0
{
     _S26 : f32,
};

fn _d_log_0( dpx_2 : ptr<function, DiffPair_float_0>,  dOut_3 : f32)
{
    var _S27 : f32 = 1.0f / (*dpx_2).primal_0 * dOut_3;
    (*dpx_2).primal_0 = (*dpx_2).primal_0;
    (*dpx_2).differential_0 = _S27;
    return;
}

fn s_bwdProp_log_0( ctx_3 : s_bwdCallableCtx_log_0,  d_x_2 : ptr<function, f32>,  d_out_3 : f32)
{
    var _S28 : DiffPair_float_0;
    _S28.primal_0 = ctx_3._S26;
    _d_log_0(&(_S28), d_out_3);
    (*d_x_2) = _S28.differential_0;
    return;
}

struct Tuple_4
{
     value0_4 : f32,
};

struct Pixel_0
{
     at_0 : vec2<f32>,
     offset_1 : vec3<f32>,
     spacing_0 : f32,
     filter_0 : bool,
};

fn wrap_0( x_3 : f32) -> f32
{
    return x_3 - 768.0f * floor(x_3 / 768.0f);
}

struct Simplex_0
{
     origin_0 : vec3<f32>,
     second_0 : vec3<f32>,
     third_0 : vec3<f32>,
     g0_0 : vec3<f32>,
     g1_0 : vec3<f32>,
     g2_0 : vec3<f32>,
     g3_0 : vec3<f32>,
};

fn hash_0( corner_0 : vec3<i32>) -> u32
{
    return permutation_0[u32(corner_0.x) + permutation_0[u32(corner_0.y) + permutation_0[corner_0.z]]];
}

fn gradient_of_0( hashed_0 : u32) -> vec3<f32>
{
    var g_0 : u32 = hashed_0 % u32(12);
    var a_0 : f32;
    if(((g_0 & (u32(1)))) != u32(0))
    {
        a_0 = -1.0f;
    }
    else
    {
        a_0 = 1.0f;
    }
    var b_0 : f32;
    if(((g_0 & (u32(2)))) != u32(0))
    {
        b_0 = -1.0f;
    }
    else
    {
        b_0 = 1.0f;
    }
    if((g_0 < u32(4)))
    {
        return vec3<f32>(a_0, b_0, 0.0f);
    }
    if((g_0 < u32(8)))
    {
        return vec3<f32>(a_0, 0.0f, b_0);
    }
    return vec3<f32>(0.0f, a_0, b_0);
}

fn locate_0( p_0 : vec3<f32>) -> Simplex_0
{
    var _S29 : vec3<i32> = vec3<i32>(floor(p_0 + vec3<f32>(((p_0.x + p_0.y + p_0.z) * 0.3333333432674408f))));
    var s_0 : Simplex_0;
    var _S30 : vec3<f32> = vec3<f32>(_S29) - vec3<f32>((f32(_S29.x + _S29.y + _S29.z) * 0.1666666716337204f));
    s_0.origin_0 = _S30;
    var d_0 : vec3<f32> = p_0 - _S30;
    var _S31 : f32 = d_0.x;
    var _S32 : f32 = d_0.y;
    if((_S31 >= _S32))
    {
        var _S33 : f32 = d_0.z;
        if((_S32 >= _S33))
        {
            s_0.second_0 = vec3<f32>(1.0f, 0.0f, 0.0f);
            s_0.third_0 = vec3<f32>(1.0f, 1.0f, 0.0f);
        }
        else
        {
            if((_S31 >= _S33))
            {
                s_0.second_0 = vec3<f32>(1.0f, 0.0f, 0.0f);
                s_0.third_0 = vec3<f32>(1.0f, 0.0f, 1.0f);
            }
            else
            {
                s_0.second_0 = vec3<f32>(0.0f, 0.0f, 1.0f);
                s_0.third_0 = vec3<f32>(1.0f, 0.0f, 1.0f);
            }
        }
    }
    else
    {
        var _S34 : f32 = d_0.z;
        if((_S32 < _S34))
        {
            s_0.second_0 = vec3<f32>(0.0f, 0.0f, 1.0f);
            s_0.third_0 = vec3<f32>(0.0f, 1.0f, 1.0f);
        }
        else
        {
            if((_S31 < _S34))
            {
                s_0.second_0 = vec3<f32>(0.0f, 1.0f, 0.0f);
                s_0.third_0 = vec3<f32>(0.0f, 1.0f, 1.0f);
            }
            else
            {
                s_0.second_0 = vec3<f32>(0.0f, 1.0f, 0.0f);
                s_0.third_0 = vec3<f32>(1.0f, 1.0f, 0.0f);
            }
        }
    }
    var base_0 : vec3<i32> = (_S29 & (vec3<i32>(i32(255))));
    s_0.g0_0 = gradient_of_0(hash_0(base_0));
    s_0.g1_0 = gradient_of_0(hash_0(base_0 + vec3<i32>(s_0.second_0)));
    s_0.g2_0 = gradient_of_0(hash_0(base_0 + vec3<i32>(s_0.third_0)));
    s_0.g3_0 = gradient_of_0(hash_0(base_0 + vec3<i32>(i32(1))));
    return s_0;
}

fn corner_1( d_1 : vec3<f32>,  g_1 : vec3<f32>) -> f32
{
    var _S35 : f32 = max(0.60000002384185791f - dot(d_1, d_1), 0.0f);
    var t_0 : f32 = _S35 * _S35;
    return t_0 * t_0 * dot(g_1, d_1);
}

fn simplex_0( xy_0 : vec2<f32>,  z_0 : f32) -> f32
{
    var p_1 : vec3<f32> = vec3<f32>(xy_0, z_0);
    var s_1 : Simplex_0 = locate_0(p_1);
    var d0_0 : vec3<f32> = p_1 - s_1.origin_0;
    return 32.0f * (corner_1(d0_0, s_1.g0_0) + corner_1(d0_0 - s_1.second_0 + vec3<f32>(0.1666666716337204f), s_1.g1_0) + corner_1(d0_0 - s_1.third_0 + vec3<f32>(0.3333333432674408f), s_1.g2_0) + corner_1(d0_0 - vec3<f32>(1.0f) + vec3<f32>(0.5f), s_1.g3_0));
}

fn octaves_0( scale_2 : f32,  at_1 : vec2<f32>,  offset_2 : vec3<f32>) -> f32
{
    var _S36 : f32 = (1.0f + scale_2 * 0.079999998211860657f) / 800.0f;
    var c_0 : f32 = 0.49099999666213989f / _S36;
    var _S37 : f32 = at_1.x;
    var _S38 : f32 = at_1.y;
    var c_1 : f32 = 0.944100022315979f / _S36;
    var c_2 : f32 = 1.7280000448226929f / _S36;
    return simplex_0(vec2<f32>(wrap_0(_S37 * c_0 + offset_2[i32(0)]), wrap_0(_S38 * c_0)), 0.0f) * 0.23399999737739563f + simplex_0(vec2<f32>(wrap_0(_S37 * c_1 + offset_2[i32(1)]), wrap_0(_S38 * c_1)), 1.0f) * 0.7850000262260437f + simplex_0(vec2<f32>(wrap_0(_S37 * c_2 + offset_2[i32(2)]), wrap_0(_S38 * c_2)), 2.0f) * 1.2150000333786011f;
}

fn tap_0( l_0 : u32) -> vec2<f32>
{
    var _S39 : f32 = f32(l_0);
    var y_0 : f32 = _S39 * 1.6190476417541504f;
    return vec2<f32>(_S39 / 21.0f, y_0 - floor(y_0));
}

fn noise_0( scale_3 : f32,  px_0 : Pixel_0) -> f32
{
    if(!px_0.filter_0)
    {
        return octaves_0(scale_3, px_0.at_0, px_0.offset_1);
    }
    var total_0 : f32 = 0.0f;
    var l_1 : u32 = u32(0);
    loop
    {
        if((l_1 < u32(21)))
        {
        }
        else
        {
            break;
        }
        var _S40 : f32 = total_0 + 0.0476190485060215f * octaves_0(scale_3, px_0.at_0 + tap_0(l_1) * vec2<f32>(px_0.spacing_0), px_0.offset_1);
        var _S41 : u32 = l_1 + u32(1);
        total_0 = _S40;
        l_1 = _S41;
    }
    return total_0;
}

fn s_apply_noise_0( scale_4 : f32,  px_1 : Pixel_0) -> Tuple_4
{
    var _S42 : Tuple_4 = Tuple_4( noise_0(scale_4, px_1) );
    return _S42;
}

struct s_bwdCallableCtx_noise_0
{
     _S43 : f32,
     _S44 : Pixel_0,
};

fn s_fwd_wrap_0( dpx_3 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S45 : DiffPair_float_0 = DiffPair_float_0( dpx_3.primal_0 - 768.0f * floor(dpx_3.primal_0 / 768.0f), dpx_3.differential_0 );
    return _S45;
}

struct DiffPair_vectorx3Cfloatx2C2x3E_0
{
     primal_0 : vec2<f32>,
     differential_0 : vec2<f32>,
};

fn _d_dot_0( dpx_4 : DiffPair_vectorx3Cfloatx2C3x3E_0,  dpy_0 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_float_0
{
    var _S46 : DiffPair_float_0 = DiffPair_float_0( dpx_4.primal_0[i32(0)] * dpy_0.primal_0[i32(0)] + dpx_4.primal_0[i32(1)] * dpy_0.primal_0[i32(1)] + dpx_4.primal_0[i32(2)] * dpy_0.primal_0[i32(2)], dpx_4.primal_0[i32(0)] * dpy_0.differential_0[i32(0)] + dpy_0.primal_0[i32(0)] * dpx_4.differential_0[i32(0)] + dpx_4.primal_0[i32(1)] * dpy_0.differential_0[i32(1)] + dpy_0.primal_0[i32(1)] * dpx_4.differential_0[i32(1)] + dpx_4.primal_0[i32(2)] * dpy_0.differential_0[i32(2)] + dpy_0.primal_0[i32(2)] * dpx_4.differential_0[i32(2)] );
    return _S46;
}

fn _d_max_0( dpx_5 : DiffPair_float_0,  dpy_1 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S47 : f32 = max(dpx_5.primal_0, dpy_1.primal_0);
    var _S48 : f32;
    if(((dpx_5.primal_0) > (dpy_1.primal_0)))
    {
        _S48 = dpx_5.differential_0;
    }
    else
    {
        if(((dpx_5.primal_0) < (dpy_1.primal_0)))
        {
            _S48 = dpy_1.differential_0;
        }
        else
        {
            _S48 = 0.5f * (dpx_5.differential_0 + dpy_1.differential_0);
        }
    }
    var _S49 : DiffPair_float_0 = DiffPair_float_0( _S47, _S48 );
    return _S49;
}

fn s_fwd_corner_0( dpd_0 : DiffPair_vectorx3Cfloatx2C3x3E_0,  g_2 : vec3<f32>) -> DiffPair_float_0
{
    var _S50 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( dpd_0.primal_0, dpd_0.differential_0 );
    var _S51 : DiffPair_float_0 = _d_dot_0(_S50, _S50);
    var _S52 : DiffPair_float_0 = DiffPair_float_0( 0.60000002384185791f - _S51.primal_0, 0.0f - _S51.differential_0 );
    var _S53 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S54 : DiffPair_float_0 = _d_max_0(_S52, _S53);
    var _S55 : f32 = _S54.primal_0;
    var t_1 : f32 = _S55 * _S55;
    var _S56 : f32 = _S54.differential_0 * _S54.primal_0;
    var _S57 : f32 = t_1 * t_1;
    var _S58 : f32 = (_S56 + _S56) * t_1;
    var _S59 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( g_2, vec3<f32>(0.0f) );
    var _S60 : DiffPair_float_0 = _d_dot_0(_S59, _S50);
    var _S61 : DiffPair_float_0 = DiffPair_float_0( _S57 * _S60.primal_0, (_S58 + _S58) * _S60.primal_0 + _S60.differential_0 * _S57 );
    return _S61;
}

fn s_fwd_simplex_0( dpxy_0 : DiffPair_vectorx3Cfloatx2C2x3E_0,  dpz_0 : DiffPair_float_0) -> DiffPair_float_0
{
    var p_2 : vec3<f32> = vec3<f32>(dpxy_0.primal_0, dpz_0.primal_0);
    var s_diff_p_0 : vec3<f32> = vec3<f32>(dpxy_0.differential_0, dpz_0.differential_0);
    var s_2 : Simplex_0 = locate_0(p_2);
    var d0_1 : vec3<f32> = p_2 - s_2.origin_0;
    var _S62 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( d0_1, s_diff_p_0 );
    var _S63 : DiffPair_float_0 = s_fwd_corner_0(_S62, s_2.g0_0);
    var _S64 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( d0_1 - s_2.second_0 + vec3<f32>(0.1666666716337204f), s_diff_p_0 );
    var _S65 : DiffPair_float_0 = s_fwd_corner_0(_S64, s_2.g1_0);
    var _S66 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( d0_1 - s_2.third_0 + vec3<f32>(0.3333333432674408f), s_diff_p_0 );
    var _S67 : DiffPair_float_0 = s_fwd_corner_0(_S66, s_2.g2_0);
    var _S68 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( d0_1 - vec3<f32>(1.0f) + vec3<f32>(0.5f), s_diff_p_0 );
    var _S69 : DiffPair_float_0 = s_fwd_corner_0(_S68, s_2.g3_0);
    var _S70 : DiffPair_float_0 = DiffPair_float_0( 32.0f * (_S63.primal_0 + _S65.primal_0 + _S67.primal_0 + _S69.primal_0), (_S63.differential_0 + _S65.differential_0 + _S67.differential_0 + _S69.differential_0) * 32.0f );
    return _S70;
}

fn s_fwd_octaves_0( dpscale_0 : DiffPair_float_0,  at_2 : vec2<f32>,  offset_3 : vec3<f32>) -> DiffPair_float_0
{
    var _S71 : f32 = (1.0f + dpscale_0.primal_0 * 0.079999998211860657f) / 800.0f;
    var _S72 : f32 = dpscale_0.differential_0 * 0.079999998211860657f * 0.0012499999720603228f;
    var _S73 : f32 = at_2.x;
    var _S74 : f32 = at_2.y;
    var c_3 : f32 = 0.49099999666213989f / _S71;
    var _S75 : f32 = _S71 * _S71;
    var s_diff_c_0 : f32 = (0.0f - 0.49099999666213989f * _S72) / _S75;
    var _S76 : DiffPair_float_0 = DiffPair_float_0( _S73 * c_3 + offset_3[i32(0)], s_diff_c_0 * _S73 );
    var _S77 : DiffPair_float_0 = s_fwd_wrap_0(_S76);
    var _S78 : DiffPair_float_0 = DiffPair_float_0( _S74 * c_3, s_diff_c_0 * _S74 );
    var _S79 : DiffPair_float_0 = s_fwd_wrap_0(_S78);
    var _S80 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( vec2<f32>(_S77.primal_0, _S79.primal_0), vec2<f32>(_S77.differential_0, _S79.differential_0) );
    var _S81 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S82 : DiffPair_float_0 = s_fwd_simplex_0(_S80, _S81);
    var c_4 : f32 = 0.944100022315979f / _S71;
    var s_diff_c_1 : f32 = (0.0f - 0.944100022315979f * _S72) / _S75;
    var _S83 : DiffPair_float_0 = DiffPair_float_0( _S73 * c_4 + offset_3[i32(1)], s_diff_c_1 * _S73 );
    var _S84 : DiffPair_float_0 = s_fwd_wrap_0(_S83);
    var _S85 : DiffPair_float_0 = DiffPair_float_0( _S74 * c_4, s_diff_c_1 * _S74 );
    var _S86 : DiffPair_float_0 = s_fwd_wrap_0(_S85);
    var _S87 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( vec2<f32>(_S84.primal_0, _S86.primal_0), vec2<f32>(_S84.differential_0, _S86.differential_0) );
    var _S88 : DiffPair_float_0 = DiffPair_float_0( 1.0f, 0.0f );
    var _S89 : DiffPair_float_0 = s_fwd_simplex_0(_S87, _S88);
    var c_5 : f32 = 1.7280000448226929f / _S71;
    var s_diff_c_2 : f32 = (0.0f - 1.7280000448226929f * _S72) / _S75;
    var _S90 : DiffPair_float_0 = DiffPair_float_0( _S73 * c_5 + offset_3[i32(2)], s_diff_c_2 * _S73 );
    var _S91 : DiffPair_float_0 = s_fwd_wrap_0(_S90);
    var _S92 : DiffPair_float_0 = DiffPair_float_0( _S74 * c_5, s_diff_c_2 * _S74 );
    var _S93 : DiffPair_float_0 = s_fwd_wrap_0(_S92);
    var _S94 : DiffPair_vectorx3Cfloatx2C2x3E_0 = DiffPair_vectorx3Cfloatx2C2x3E_0( vec2<f32>(_S91.primal_0, _S93.primal_0), vec2<f32>(_S91.differential_0, _S93.differential_0) );
    var _S95 : DiffPair_float_0 = DiffPair_float_0( 2.0f, 0.0f );
    var _S96 : DiffPair_float_0 = s_fwd_simplex_0(_S94, _S95);
    var _S97 : DiffPair_float_0 = DiffPair_float_0( _S82.primal_0 * 0.23399999737739563f + _S89.primal_0 * 0.7850000262260437f + _S96.primal_0 * 1.2150000333786011f, _S82.differential_0 * 0.23399999737739563f + _S89.differential_0 * 0.7850000262260437f + _S96.differential_0 * 1.2150000333786011f );
    return _S97;
}

fn noise_forward_0( scale_5 : DiffPair_float_0,  px_2 : Pixel_0) -> DiffPair_float_0
{
    var _S98 : DiffPair_float_0 = DiffPair_float_0( scale_5.primal_0, 1.0f );
    if(!px_2.filter_0)
    {
        var _S99 : DiffPair_float_0 = s_fwd_octaves_0(_S98, px_2.at_0, px_2.offset_1);
        var _S100 : DiffPair_float_0 = DiffPair_float_0( _S99.primal_0, _S99.differential_0 * scale_5.differential_0 );
        return _S100;
    }
    var value_0 : f32 = 0.0f;
    var slope_0 : f32 = 0.0f;
    var l_2 : u32 = u32(0);
    loop
    {
        if((l_2 < u32(21)))
        {
        }
        else
        {
            break;
        }
        var _S101 : DiffPair_float_0 = s_fwd_octaves_0(_S98, px_2.at_0 + tap_0(l_2) * vec2<f32>(px_2.spacing_0), px_2.offset_1);
        var _S102 : f32 = value_0 + 0.0476190485060215f * _S101.primal_0;
        var _S103 : f32 = slope_0 + 0.0476190485060215f * _S101.differential_0;
        var _S104 : u32 = l_2 + u32(1);
        value_0 = _S102;
        slope_0 = _S103;
        l_2 = _S104;
    }
    var _S105 : DiffPair_float_0 = DiffPair_float_0( value_0, slope_0 * scale_5.differential_0 );
    return _S105;
}

fn noise_backward_0( scale_6 : ptr<function, DiffPair_float_0>,  px_3 : Pixel_0,  d_out_4 : f32)
{
    var _S106 : DiffPair_float_0 = DiffPair_float_0( (*scale_6).primal_0, 1.0f );
    var _S107 : f32 = noise_forward_0(_S106, px_3).differential_0 * d_out_4;
    (*scale_6).primal_0 = (*scale_6).primal_0;
    (*scale_6).differential_0 = _S107;
    return;
}

fn s_bwdProp_noise_0( ctx_4 : s_bwdCallableCtx_noise_0,  d_scale_0 : ptr<function, f32>,  d_out_5 : f32)
{
    var _S108 : DiffPair_float_0;
    _S108.primal_0 = ctx_4._S43;
    noise_backward_0(&(_S108), ctx_4._S44, d_out_5);
    (*d_scale_0) = _S108.differential_0;
    return;
}

struct Grain_0
{
     strength_1 : f32,
     scale_7 : f32,
     midtones_bias_1 : f32,
};

struct DiffPair_Grain_0
{
     primal_0 : Grain_0,
     differential_0 : Grain_0,
};

struct Colour_0
{
     rgb_to_xyz_0 : mat3x3<f32>,
     xyz_to_rgb_0 : mat3x3<f32>,
};

struct s_minCtx_s_bwdCallableCtx_grain_from_settings_0
{
     value0_5 : f32,
};

struct Tuple_5
{
     value0_6 : vec3<f32>,
     value1_0 : s_minCtx_s_bwdCallableCtx_grain_from_settings_0,
};

struct Tuple_6
{
     value0_7 : vec3<f32>,
};

struct Tuple_7
{
     value0_8 : f32,
};

fn s_apply_grain_amount_0( _S109 : f32,  _S110 : f32) -> Tuple_7
{
    var _S111 : Tuple_7;
    _S111.value0_8 = _S109 * (_S110 / 100.0f) * 0.15000000596046448f;
    return _S111;
}

struct Tuple_8
{
     value0_9 : f32,
};

fn s_apply_delta_of_0( _S112 : f32) -> Tuple_8
{
    var _S113 : Tuple_8;
    _S113.value0_9 = 2.0f * s_apply_exp_0(_S112 / 100.0f * s_apply_log_0(9.9999997473787516e-05f).value0_3).value0_2;
    return _S113;
}

struct Tuple_9
{
     value0_10 : f32,
};

struct Tuple_10
{
     value0_11 : f32,
};

struct Tuple_11
{
     value0_12 : f32,
};

fn s_apply_paper_inverse_0( _S114 : f32,  _S115 : f32) -> Tuple_11
{
    var span_0 : f32 = 1.0f + 2.0f * _S115;
    var _S116 : Tuple_11;
    _S116.value0_12 = - s_apply_log_0(span_0 / (_S114 + _S115) - 1.0f).value0_3 * span_0 / 4.0f + 0.5f;
    return _S116;
}

struct Tuple_12
{
     value0_13 : f32,
};

fn s_apply_paper_0( _S117 : f32,  _S118 : f32) -> Tuple_12
{
    var span_1 : f32 = 1.0f + 2.0f * _S118;
    var _S119 : Tuple_12;
    _S119.value0_13 = span_1 / (1.0f + s_apply_exp_0(4.0f * (0.5f - _S117) / span_1).value0_2) - _S118;
    return _S119;
}

fn s_apply_lut_entry_0( i_0 : f32,  j_0 : f32,  _S120 : f32) -> Tuple_10
{
    var l_3 : f32 = j_0 / 127.0f;
    var _S121 : Tuple_10;
    _S121.value0_11 = 100.0f * (s_apply_paper_0(i_0 / 127.0f - 0.5f + s_apply_paper_inverse_0(l_3, _S120).value0_12, _S120).value0_13 - l_3);
    return _S121;
}

fn s_apply_lookup_0( _S122 : f32,  l_4 : f32,  _S123 : f32,  cell_0 : vec2<i32>) -> Tuple_9
{
    var x0_0 : f32 = f32(cell_0.x);
    var y0_0 : f32 = f32(cell_0.y);
    var x_diff_0 : f32 = s_apply_clamp_0((_S122 + 0.5f) * 127.0f, 0.0f, 127.0f).value0_1 - x0_0;
    var y_diff_0 : f32 = s_apply_clamp_0(l_4 * 127.0f, 0.0f, 127.0f).value0_1 - y0_0;
    var _S124 : f32 = x0_0 + 1.0f;
    var _S125 : f32 = y0_0 + 1.0f;
    var _S126 : f32 = 1.0f - y_diff_0;
    var _S127 : Tuple_9;
    _S127.value0_10 = (_S126 * s_apply_lut_entry_0(x0_0, y0_0, _S123).value0_11 + s_apply_lut_entry_0(x0_0, _S125, _S123).value0_11 * y_diff_0) * (1.0f - x_diff_0) + (_S126 * s_apply_lut_entry_0(_S124, y0_0, _S123).value0_11 + s_apply_lut_entry_0(_S124, _S125, _S123).value0_11 * y_diff_0) * x_diff_0;
    return _S127;
}

struct Tuple_13
{
     value0_14 : vec3<f32>,
};

struct Tuple_14
{
     value0_15 : vec3<f32>,
};

struct Tuple_15
{
     value0_16 : f32,
};

fn s_apply_lab_f_inv_0( _S128 : f32) -> Tuple_15
{
    var _S129 : bool = (_S128 > 0.20689655840396881f);
    var _S130 : f32;
    if(_S129)
    {
        _S130 = _S128 * _S128 * _S128;
    }
    var _S131 : bool = !_S129;
    if(_S131)
    {
        _S130 = (116.0f * _S128 - 16.0f) / 903.29632568359375f;
    }
    var _S132 : Tuple_15;
    _S132.value0_16 = _S130;
    return _S132;
}

fn s_apply_lab_to_xyz_d50_0( _S133 : vec3<f32>) -> Tuple_14
{
    var fy_0 : f32 = (_S133.x + 16.0f) * 0.0086206896230578423f;
    var _S134 : Tuple_14;
    _S134.value0_15 = vec3<f32>(0.96420001983642578f, 1.0f, 0.82489997148513794f) * vec3<f32>(s_apply_lab_f_inv_0(_S133.y * 0.0020000000949949026f + fy_0).value0_16, s_apply_lab_f_inv_0(fy_0).value0_16, s_apply_lab_f_inv_0(_S133.z * -0.004999999888241291f + fy_0).value0_16);
    return _S134;
}

fn s_apply_lab_to_rgb_0( _S135 : vec3<f32>,  c_6 : Colour_0) -> Tuple_13
{
    var _S136 : Tuple_13;
    _S136.value0_14 = s_apply_mul_0(c_6.xyz_to_rgb_0, s_apply_lab_to_xyz_d50_0(_S135).value0_15).value0_0;
    return _S136;
}

fn s_apply_grain_0( lab_0 : vec3<f32>,  _S137 : f32,  _S138 : Grain_0,  c_7 : Colour_0,  cell_1 : vec2<i32>) -> Tuple_6
{
    var _S139 : f32 = lab_0.x;
    var _S140 : Tuple_6;
    _S140.value0_7 = s_apply_lab_to_rgb_0(vec3<f32>(_S139 + s_apply_lookup_0(s_apply_grain_amount_0(_S137, _S138.strength_1).value0_8, _S139 / 100.0f, s_apply_delta_of_0(_S138.midtones_bias_1).value0_9, cell_1).value0_10, lab_0.y, lab_0.z), c_7).value0_14;
    return _S140;
}

fn s_apply_grain_from_settings_0( lab_1 : vec3<f32>,  _S141 : Grain_0,  px_4 : Pixel_0,  c_8 : Colour_0,  cell_2 : vec2<i32>) -> Tuple_5
{
    var _S142 : s_minCtx_s_bwdCallableCtx_grain_from_settings_0;
    _S142.value0_5 = 0.0f;
    var _S143 : Tuple_4 = s_apply_noise_0(_S141.scale_7, px_4);
    _S142.value0_5 = _S143.value0_4;
    var _S144 : Tuple_5;
    _S144.value0_6 = s_apply_grain_0(lab_1, _S143.value0_4, _S141, c_8, cell_2).value0_7;
    _S144.value1_0 = _S142;
    return _S144;
}

struct s_paramCtx_s_bwdCallableCtx_grain_from_settings_0
{
     value0_17 : vec3<f32>,
     value1_1 : Grain_0,
     value2_0 : Pixel_0,
     value3_0 : Colour_0,
     value4_0 : vec2<i32>,
};

struct s_bwdCallableCtx_grain_from_settings_0
{
     value0_18 : s_paramCtx_s_bwdCallableCtx_grain_from_settings_0,
     value1_2 : s_minCtx_s_bwdCallableCtx_grain_from_settings_0,
};

struct s_paramCtx_s_bwdCallableCtx_grain_0
{
     value0_19 : vec3<f32>,
     value1_3 : f32,
     value2_1 : Grain_0,
     value3_1 : Colour_0,
     value4_1 : vec2<i32>,
};

struct s_bwdCallableCtx_grain_0
{
     value0_20 : s_paramCtx_s_bwdCallableCtx_grain_0,
};

fn Grain_x24_syn_dzero_0() -> Grain_0
{
    var result_0 : Grain_0;
    result_0.strength_1 = 0.0f;
    result_0.scale_7 = 0.0f;
    result_0.midtones_bias_1 = 0.0f;
    return result_0;
}

struct s_paramCtx_s_bwdCallableCtx_grain_amount_0
{
     value0_21 : f32,
     value1_4 : f32,
};

struct s_bwdCallableCtx_grain_amount_0
{
     value0_22 : s_paramCtx_s_bwdCallableCtx_grain_amount_0,
};

struct s_paramCtx_s_bwdCallableCtx_delta_of_0
{
     value0_23 : f32,
};

struct s_bwdCallableCtx_delta_of_0
{
     value0_24 : s_paramCtx_s_bwdCallableCtx_delta_of_0,
};

struct s_paramCtx_s_bwdCallableCtx_lookup_0
{
     value0_25 : f32,
     value1_5 : f32,
     value2_2 : f32,
     value3_2 : vec2<i32>,
};

struct s_bwdCallableCtx_lookup_0
{
     value0_26 : s_paramCtx_s_bwdCallableCtx_lookup_0,
};

struct s_paramCtx_s_bwdCallableCtx_lab_to_rgb_0
{
     value0_27 : vec3<f32>,
     value1_6 : Colour_0,
};

struct s_bwdCallableCtx_lab_to_rgb_0
{
     value0_28 : s_paramCtx_s_bwdCallableCtx_lab_to_rgb_0,
};

struct s_paramCtx_s_bwdCallableCtx_lab_to_xyz_d50_0
{
     value0_29 : vec3<f32>,
};

struct s_bwdCallableCtx_lab_to_xyz_d50_0
{
     value0_30 : s_paramCtx_s_bwdCallableCtx_lab_to_xyz_d50_0,
};

struct s_paramCtx_s_bwdCallableCtx_lab_f_inv_0
{
     value0_31 : f32,
};

struct s_bwdCallableCtx_lab_f_inv_0
{
     value0_32 : s_paramCtx_s_bwdCallableCtx_lab_f_inv_0,
};

fn s_bwdProp_lab_f_inv_0( _S145 : s_bwdCallableCtx_lab_f_inv_0,  _S146 : ptr<function, f32>,  _S147 : f32)
{
    var _S148 : bool = ((_S145.value0_32.value0_31) > 0.20689655840396881f);
    var _S149 : f32;
    if(_S148)
    {
        _S149 = _S145.value0_32.value0_31 * _S145.value0_32.value0_31;
    }
    else
    {
        _S149 = 0.0f;
    }
    var _S150 : bool = !_S148;
    var _S151 : f32;
    var _S152 : f32;
    if(_S150)
    {
        var _S153 : f32 = 116.0f * (0.0011070563923567533f * _S147);
        _S151 = 0.0f;
        _S152 = _S153;
    }
    else
    {
        _S151 = _S147;
        _S152 = 0.0f;
    }
    if(_S148)
    {
        var _S154 : f32 = _S145.value0_32.value0_31 * (_S145.value0_32.value0_31 * _S151);
        _S149 = _S149 * _S151 + _S154 + _S154 + _S152;
    }
    else
    {
        _S149 = _S152;
    }
    (*_S146) = _S149;
    return;
}

fn s_bwdProp_lab_to_xyz_d50_0( _S155 : s_bwdCallableCtx_lab_to_xyz_d50_0,  _S156 : ptr<function, vec3<f32>>,  _S157 : vec3<f32>)
{
    var fy_1 : f32 = (_S155.value0_30.value0_29.x + 16.0f) * 0.0086206896230578423f;
    var fz_0 : f32 = _S155.value0_30.value0_29.z * -0.004999999888241291f + fy_1;
    var _S158 : s_paramCtx_s_bwdCallableCtx_lab_f_inv_0;
    _S158.value0_31 = _S155.value0_30.value0_29.y * 0.0020000000949949026f + fy_1;
    var _S159 : s_bwdCallableCtx_lab_f_inv_0 = s_bwdCallableCtx_lab_f_inv_0( _S158 );
    var _S160 : s_paramCtx_s_bwdCallableCtx_lab_f_inv_0;
    _S160.value0_31 = fy_1;
    var _S161 : s_bwdCallableCtx_lab_f_inv_0 = s_bwdCallableCtx_lab_f_inv_0( _S160 );
    var _S162 : s_paramCtx_s_bwdCallableCtx_lab_f_inv_0;
    _S162.value0_31 = fz_0;
    var _S163 : s_bwdCallableCtx_lab_f_inv_0 = s_bwdCallableCtx_lab_f_inv_0( _S162 );
    var _S164 : vec3<f32> = vec3<f32>(0.96420001983642578f, 1.0f, 0.82489997148513794f) * _S157;
    var _S165 : f32 = 0.0f;
    s_bwdProp_lab_f_inv_0(_S163, &(_S165), _S164[i32(2)]);
    var _S166 : f32 = 0.0f;
    s_bwdProp_lab_f_inv_0(_S161, &(_S166), _S164[i32(1)]);
    var _S167 : f32 = 0.0f;
    s_bwdProp_lab_f_inv_0(_S159, &(_S167), _S164[i32(0)]);
    (*_S156) = vec3<f32>(0.0086206896230578423f * (_S166 + _S165 + _S167), 0.0020000000949949026f * _S167, -0.004999999888241291f * _S165);
    return;
}

fn s_bwdProp_lab_to_rgb_0( _S168 : s_bwdCallableCtx_lab_to_rgb_0,  _S169 : ptr<function, vec3<f32>>,  _S170 : vec3<f32>)
{
    var _S171 : Tuple_14 = s_apply_lab_to_xyz_d50_0(_S168.value0_28.value0_27);
    var _S172 : s_paramCtx_s_bwdCallableCtx_lab_to_xyz_d50_0;
    _S172.value0_29 = _S168.value0_28.value0_27;
    var _S173 : s_bwdCallableCtx_lab_to_xyz_d50_0 = s_bwdCallableCtx_lab_to_xyz_d50_0( _S172 );
    var _S174 : s_bwdCallableCtx_mul_0;
    _S174._S2 = _S168.value0_28.value1_6.xyz_to_rgb_0;
    _S174._S3 = _S171.value0_15;
    var _S175 : mat3x3<f32> = mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f);
    var _S176 : vec3<f32> = vec3<f32>(0.0f);
    var _S177 : vec3<f32> = _S176;
    s_bwdProp_mul_0(_S174, &(_S175), &(_S177), _S170);
    var _S178 : vec3<f32> = _S176;
    s_bwdProp_lab_to_xyz_d50_0(_S173, &(_S178), _S177);
    (*_S169) = _S178;
    return;
}

struct s_paramCtx_s_bwdCallableCtx_lut_entry_0
{
     value0_33 : f32,
     value1_7 : f32,
     value2_3 : f32,
};

struct s_bwdCallableCtx_lut_entry_0
{
     value0_34 : s_paramCtx_s_bwdCallableCtx_lut_entry_0,
};

struct s_paramCtx_s_bwdCallableCtx_paper_inverse_0
{
     value0_35 : f32,
     value1_8 : f32,
};

struct s_bwdCallableCtx_paper_inverse_0
{
     value0_36 : s_paramCtx_s_bwdCallableCtx_paper_inverse_0,
};

struct s_paramCtx_s_bwdCallableCtx_paper_0
{
     value0_37 : f32,
     value1_9 : f32,
};

struct s_bwdCallableCtx_paper_0
{
     value0_38 : s_paramCtx_s_bwdCallableCtx_paper_0,
};

fn s_bwdProp_paper_0( _S179 : s_bwdCallableCtx_paper_0,  _S180 : ptr<function, f32>,  _S181 : ptr<function, f32>,  _S182 : f32)
{
    var span_2 : f32 = 1.0f + 2.0f * _S179.value0_38.value1_9;
    var _S183 : f32 = 4.0f * (0.5f - _S179.value0_38.value0_37);
    var _S184 : f32 = _S183 / span_2;
    var _S185 : f32 = span_2 * span_2;
    var _S186 : Tuple_2 = s_apply_exp_0(_S184);
    var _S187 : s_bwdCallableCtx_exp_0;
    _S187._S22 = _S184;
    var _S188 : f32 = 1.0f + _S186.value0_2;
    var _S189 : f32 = - _S182;
    var _S190 : f32 = _S182 / (_S188 * _S188);
    var _S191 : f32 = span_2 * - _S190;
    var _S192 : f32 = _S188 * _S190;
    var _S193 : f32 = 0.0f;
    s_bwdProp_exp_0(_S187, &(_S193), _S191);
    var _S194 : f32 = _S193 / _S185;
    var _S195 : f32 = _S189 + 2.0f * (_S192 + _S183 * - _S194);
    (*_S180) = - (4.0f * (span_2 * _S194));
    (*_S181) = _S195;
    return;
}

fn s_bwdProp_paper_inverse_0( _S196 : s_bwdCallableCtx_paper_inverse_0,  _S197 : ptr<function, f32>,  _S198 : ptr<function, f32>,  _S199 : f32)
{
    var span_3 : f32 = 1.0f + 2.0f * _S196.value0_36.value1_8;
    var _S200 : f32 = _S196.value0_36.value0_35 + _S196.value0_36.value1_8;
    var _S201 : f32 = _S200 * _S200;
    var _S202 : f32 = span_3 / _S200 - 1.0f;
    var _S203 : Tuple_3 = s_apply_log_0(_S202);
    var _S204 : s_bwdCallableCtx_log_0;
    _S204._S26 = _S202;
    var _S205 : f32 = 0.25f * _S199;
    var _S206 : f32 = - _S203.value0_3 * _S205;
    var _S207 : f32 = - (span_3 * _S205);
    var _S208 : f32 = 0.0f;
    s_bwdProp_log_0(_S204, &(_S208), _S207);
    var _S209 : f32 = _S208 / _S201;
    var _S210 : f32 = span_3 * - _S209;
    var _S211 : f32 = _S210 + 2.0f * (_S206 + _S200 * _S209);
    (*_S197) = _S210;
    (*_S198) = _S211;
    return;
}

fn s_bwdProp_lut_entry_0( _S212 : s_bwdCallableCtx_lut_entry_0,  _S213 : ptr<function, f32>,  _S214 : f32)
{
    var gu_0 : f32 = _S212.value0_34.value0_33 / 127.0f - 0.5f;
    var l_5 : f32 = _S212.value0_34.value1_7 / 127.0f;
    var _S215 : Tuple_11 = s_apply_paper_inverse_0(l_5, _S212.value0_34.value2_3);
    var _S216 : s_paramCtx_s_bwdCallableCtx_paper_inverse_0;
    _S216.value0_35 = l_5;
    _S216.value1_8 = _S212.value0_34.value2_3;
    var _S217 : s_bwdCallableCtx_paper_inverse_0 = s_bwdCallableCtx_paper_inverse_0( _S216 );
    var _S218 : s_paramCtx_s_bwdCallableCtx_paper_0;
    _S218.value0_37 = gu_0 + _S215.value0_12;
    _S218.value1_9 = _S212.value0_34.value2_3;
    var _S219 : s_bwdCallableCtx_paper_0 = s_bwdCallableCtx_paper_0( _S218 );
    var _S220 : f32 = 100.0f * _S214;
    var _S221 : f32 = 0.0f;
    var _S222 : f32 = 0.0f;
    s_bwdProp_paper_0(_S219, &(_S221), &(_S222), _S220);
    var _S223 : f32 = 0.0f;
    var _S224 : f32 = 0.0f;
    s_bwdProp_paper_inverse_0(_S217, &(_S223), &(_S224), _S221);
    (*_S213) = _S222 + _S224;
    return;
}

fn s_bwdProp_lookup_0( _S225 : s_bwdCallableCtx_lookup_0,  _S226 : ptr<function, f32>,  _S227 : ptr<function, f32>,  _S228 : f32)
{
    var _S229 : f32 = (_S225.value0_26.value0_25 + 0.5f) * 127.0f;
    var _S230 : Tuple_1 = s_apply_clamp_0(_S229, 0.0f, 127.0f);
    var _S231 : s_bwdCallableCtx_clamp_0;
    _S231._S10 = _S229;
    _S231._S11 = 0.0f;
    _S231._S12 = 127.0f;
    var x0_1 : f32 = f32(_S225.value0_26.value3_2.x);
    var y0_1 : f32 = f32(_S225.value0_26.value3_2.y);
    var x_diff_1 : f32 = _S230.value0_1 - x0_1;
    var y_diff_1 : f32 = s_apply_clamp_0(_S225.value0_26.value1_5 * 127.0f, 0.0f, 127.0f).value0_1 - y0_1;
    var _S232 : Tuple_10 = s_apply_lut_entry_0(x0_1, y0_1, _S225.value0_26.value2_2);
    var _S233 : s_paramCtx_s_bwdCallableCtx_lut_entry_0;
    _S233.value0_33 = x0_1;
    _S233.value1_7 = y0_1;
    _S233.value2_3 = _S225.value0_26.value2_2;
    var _S234 : s_bwdCallableCtx_lut_entry_0 = s_bwdCallableCtx_lut_entry_0( _S233 );
    var _S235 : f32 = x0_1 + 1.0f;
    var _S236 : Tuple_10 = s_apply_lut_entry_0(_S235, y0_1, _S225.value0_26.value2_2);
    var _S237 : s_paramCtx_s_bwdCallableCtx_lut_entry_0;
    _S237.value0_33 = _S235;
    _S237.value1_7 = y0_1;
    _S237.value2_3 = _S225.value0_26.value2_2;
    var _S238 : s_bwdCallableCtx_lut_entry_0 = s_bwdCallableCtx_lut_entry_0( _S237 );
    var _S239 : f32 = y0_1 + 1.0f;
    var _S240 : Tuple_10 = s_apply_lut_entry_0(x0_1, _S239, _S225.value0_26.value2_2);
    var _S241 : s_paramCtx_s_bwdCallableCtx_lut_entry_0;
    _S241.value0_33 = x0_1;
    _S241.value1_7 = _S239;
    _S241.value2_3 = _S225.value0_26.value2_2;
    var _S242 : s_bwdCallableCtx_lut_entry_0 = s_bwdCallableCtx_lut_entry_0( _S241 );
    var _S243 : Tuple_10 = s_apply_lut_entry_0(_S235, _S239, _S225.value0_26.value2_2);
    var _S244 : s_paramCtx_s_bwdCallableCtx_lut_entry_0;
    _S244.value0_33 = _S235;
    _S244.value1_7 = _S239;
    _S244.value2_3 = _S225.value0_26.value2_2;
    var _S245 : s_bwdCallableCtx_lut_entry_0 = s_bwdCallableCtx_lut_entry_0( _S244 );
    var _S246 : f32 = 1.0f - y_diff_1;
    var _S247 : f32 = (_S246 * _S236.value0_11 + _S243.value0_11 * y_diff_1) * _S228;
    var s_diff_xy1_T_0 : f32 = x_diff_1 * _S228;
    var s_diff_xy0_T_0 : f32 = (1.0f - x_diff_1) * _S228;
    var _S248 : f32 = - ((_S246 * _S232.value0_11 + _S240.value0_11 * y_diff_1) * _S228);
    var _S249 : f32 = y_diff_1 * s_diff_xy1_T_0;
    var _S250 : f32 = _S246 * s_diff_xy1_T_0;
    var _S251 : f32 = y_diff_1 * s_diff_xy0_T_0;
    var _S252 : f32 = _S246 * s_diff_xy0_T_0;
    var _S253 : f32 = 0.0f;
    s_bwdProp_lut_entry_0(_S245, &(_S253), _S249);
    var _S254 : f32 = 0.0f;
    s_bwdProp_lut_entry_0(_S242, &(_S254), _S251);
    var _S255 : f32 = 0.0f;
    s_bwdProp_lut_entry_0(_S238, &(_S255), _S250);
    var _S256 : f32 = 0.0f;
    s_bwdProp_lut_entry_0(_S234, &(_S256), _S252);
    var _S257 : f32 = _S247 + _S248;
    var _S258 : f32 = 0.0f;
    var _S259 : f32 = 0.0f;
    var _S260 : f32 = 0.0f;
    s_bwdProp_clamp_0(_S231, &(_S258), &(_S259), &(_S260), _S257);
    var _S261 : f32 = _S253 + _S254 + _S255 + _S256;
    (*_S226) = 127.0f * _S258;
    (*_S227) = _S261;
    return;
}

fn s_bwdProp_delta_of_0( _S262 : s_bwdCallableCtx_delta_of_0,  _S263 : ptr<function, f32>,  _S264 : f32)
{
    var _S265 : Tuple_3 = s_apply_log_0(9.9999997473787516e-05f);
    var _S266 : s_bwdCallableCtx_exp_0;
    _S266._S22 = _S262.value0_24.value0_23 / 100.0f * _S265.value0_3;
    var _S267 : f32 = 2.0f * _S264;
    var _S268 : f32 = 0.0f;
    s_bwdProp_exp_0(_S266, &(_S268), _S267);
    (*_S263) = 0.0099999997764825821f * (_S265.value0_3 * _S268);
    return;
}

fn s_bwdProp_grain_amount_0( _S269 : s_bwdCallableCtx_grain_amount_0,  _S270 : ptr<function, f32>,  _S271 : ptr<function, f32>,  _S272 : f32)
{
    var _S273 : f32 = 0.15000000596046448f * _S272;
    var _S274 : f32 = 0.0099999997764825821f * (_S269.value0_22.value0_21 * _S273);
    (*_S270) = _S269.value0_22.value1_4 / 100.0f * _S273;
    (*_S271) = _S274;
    return;
}

fn Grain_x24_syn_dadd_0( SLANG_anonymous_0_0 : Grain_0,  SLANG_anonymous_1_0 : Grain_0) -> Grain_0
{
    var result_1 : Grain_0;
    result_1.strength_1 = SLANG_anonymous_0_0.strength_1 + SLANG_anonymous_1_0.strength_1;
    result_1.scale_7 = SLANG_anonymous_0_0.scale_7 + SLANG_anonymous_1_0.scale_7;
    result_1.midtones_bias_1 = SLANG_anonymous_0_0.midtones_bias_1 + SLANG_anonymous_1_0.midtones_bias_1;
    return result_1;
}

fn s_bwdProp_grain_0( _S275 : s_bwdCallableCtx_grain_0,  _S276 : ptr<function, f32>,  _S277 : ptr<function, Grain_0>,  _S278 : vec3<f32>)
{
    var _S279 : Tuple_7 = s_apply_grain_amount_0(_S275.value0_20.value1_3, _S275.value0_20.value2_1.strength_1);
    var _S280 : s_paramCtx_s_bwdCallableCtx_grain_amount_0;
    _S280.value0_21 = _S275.value0_20.value1_3;
    _S280.value1_4 = _S275.value0_20.value2_1.strength_1;
    var _S281 : s_bwdCallableCtx_grain_amount_0 = s_bwdCallableCtx_grain_amount_0( _S280 );
    var _S282 : f32 = _S275.value0_20.value0_19.x;
    var _S283 : f32 = _S282 / 100.0f;
    var _S284 : Tuple_8 = s_apply_delta_of_0(_S275.value0_20.value2_1.midtones_bias_1);
    var _S285 : s_paramCtx_s_bwdCallableCtx_delta_of_0;
    _S285.value0_23 = _S275.value0_20.value2_1.midtones_bias_1;
    var _S286 : s_bwdCallableCtx_delta_of_0 = s_bwdCallableCtx_delta_of_0( _S285 );
    var _S287 : Tuple_9 = s_apply_lookup_0(_S279.value0_8, _S283, _S284.value0_9, _S275.value0_20.value4_1);
    var _S288 : s_paramCtx_s_bwdCallableCtx_lookup_0;
    _S288.value0_25 = _S279.value0_8;
    _S288.value1_5 = _S283;
    _S288.value2_2 = _S284.value0_9;
    _S288.value3_2 = _S275.value0_20.value4_1;
    var _S289 : s_bwdCallableCtx_lookup_0 = s_bwdCallableCtx_lookup_0( _S288 );
    var _S290 : s_paramCtx_s_bwdCallableCtx_lab_to_rgb_0;
    _S290.value0_27 = vec3<f32>(_S282 + _S287.value0_10, _S275.value0_20.value0_19.y, _S275.value0_20.value0_19.z);
    _S290.value1_6 = _S275.value0_20.value3_1;
    var _S291 : s_bwdCallableCtx_lab_to_rgb_0 = s_bwdCallableCtx_lab_to_rgb_0( _S290 );
    var _S292 : Grain_0 = Grain_x24_syn_dzero_0();
    var _S293 : vec3<f32> = vec3<f32>(0.0f);
    s_bwdProp_lab_to_rgb_0(_S291, &(_S293), _S278);
    var _S294 : f32 = 0.0f;
    var _S295 : f32 = 0.0f;
    s_bwdProp_lookup_0(_S289, &(_S294), &(_S295), _S293[i32(0)]);
    var _S296 : f32 = 0.0f;
    s_bwdProp_delta_of_0(_S286, &(_S296), _S295);
    var _S297 : f32 = 0.0f;
    var _S298 : f32 = 0.0f;
    s_bwdProp_grain_amount_0(_S281, &(_S297), &(_S298), _S294);
    var _S299 : Grain_0 = _S292;
    _S299.midtones_bias_1 = _S296;
    _S299.strength_1 = _S298;
    var _S300 : Grain_0 = Grain_x24_syn_dadd_0(_S292, _S299);
    (*_S276) = _S297;
    (*_S277) = _S300;
    return;
}

fn s_bwdProp_grain_from_settings_0( _S301 : s_bwdCallableCtx_grain_from_settings_0,  _S302 : ptr<function, Grain_0>,  _S303 : vec3<f32>)
{
    var _S304 : s_bwdCallableCtx_noise_0;
    _S304._S43 = _S301.value0_18.value1_1.scale_7;
    _S304._S44 = _S301.value0_18.value2_0;
    var _S305 : s_paramCtx_s_bwdCallableCtx_grain_0;
    _S305.value0_19 = _S301.value0_18.value0_17;
    _S305.value1_3 = _S301.value1_2.value0_5;
    _S305.value2_1 = _S301.value0_18.value1_1;
    _S305.value3_1 = _S301.value0_18.value3_0;
    _S305.value4_1 = _S301.value0_18.value4_0;
    var _S306 : s_bwdCallableCtx_grain_0 = s_bwdCallableCtx_grain_0( _S305 );
    var _S307 : Grain_0 = Grain_x24_syn_dzero_0();
    var _S308 : f32 = 0.0f;
    var _S309 : Grain_0;
    _S309.strength_1 = 0.0f;
    _S309.scale_7 = 0.0f;
    _S309.midtones_bias_1 = 0.0f;
    s_bwdProp_grain_0(_S306, &(_S308), &(_S309), _S303);
    var _S310 : f32 = 0.0f;
    s_bwdProp_noise_0(_S304, &(_S310), _S308);
    var _S311 : Grain_0 = Grain_x24_syn_dadd_0(_S309, _S307);
    var _S312 : Grain_0 = _S307;
    _S312.scale_7 = _S310;
    (*_S302) = Grain_x24_syn_dadd_0(_S311, _S312);
    return;
}

fn _S313( _S314 : vec3<f32>,  _S315 : ptr<function, DiffPair_Grain_0>,  _S316 : Pixel_0,  _S317 : Colour_0,  _S318 : vec2<i32>,  _S319 : vec3<f32>)
{
    var _S320 : Tuple_5 = s_apply_grain_from_settings_0(_S314, (*_S315).primal_0, _S316, _S317, _S318);
    var _S321 : s_paramCtx_s_bwdCallableCtx_grain_from_settings_0;
    _S321.value0_17 = _S314;
    _S321.value1_1 = (*_S315).primal_0;
    _S321.value2_0 = _S316;
    _S321.value3_0 = _S317;
    _S321.value4_0 = _S318;
    var _S322 : s_bwdCallableCtx_grain_from_settings_0 = s_bwdCallableCtx_grain_from_settings_0( _S321, _S320.value1_0 );
    var _S323 : Grain_0 = (*_S315).differential_0;
    s_bwdProp_grain_from_settings_0(_S322, &(_S323), _S319);
    (*_S315).differential_0 = _S323;
    return;
}

fn _d_exp_1( dpx_6 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S324 : f32 = exp(dpx_6.primal_0);
    var _S325 : DiffPair_float_0 = DiffPair_float_0( _S324, _S324 * dpx_6.differential_0 );
    return _S325;
}

fn _d_log_1( dpx_7 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S326 : DiffPair_float_0 = DiffPair_float_0( log(dpx_7.primal_0), 1.0f / dpx_7.primal_0 * dpx_7.differential_0 );
    return _S326;
}

fn _d_clamp_1( dpx_8 : DiffPair_float_0,  dpMin_1 : DiffPair_float_0,  dpMax_1 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S327 : f32 = clamp(dpx_8.primal_0, dpMin_1.primal_0, dpMax_1.primal_0);
    var _S328 : bool;
    if(((dpx_8.primal_0) >= (dpMin_1.primal_0)))
    {
        _S328 = ((dpx_8.primal_0) <= (dpMax_1.primal_0));
    }
    else
    {
        _S328 = false;
    }
    var _S329 : f32;
    if(_S328)
    {
        _S329 = dpx_8.differential_0;
    }
    else
    {
        if(((dpx_8.primal_0) < (dpMin_1.primal_0)))
        {
            _S329 = dpMin_1.differential_0;
        }
        else
        {
            _S329 = dpMax_1.differential_0;
        }
    }
    var _S330 : DiffPair_float_0 = DiffPair_float_0( _S327, _S329 );
    return _S330;
}

fn pixel_of_0( id_0 : vec3<u32>) -> Pixel_0
{
    var shorter_0 : f32 = f32(min(params_0.pass_0.full_width_0, params_0.pass_0.full_height_0));
    var px_5 : Pixel_0;
    px_5.at_0 = vec2<f32>(f32(params_0.pass_0.origin_x_0 + id_0.x), f32(params_0.pass_0.origin_y_0 + id_0.y)) / vec2<f32>(params_0.pass_0.scale_0) / vec2<f32>(shorter_0);
    px_5.offset_1 = params_0.grain_0.offset_0.xyz;
    px_5.spacing_0 = 1.0f / (params_0.pass_0.scale_0 * shorter_0);
    px_5.filter_0 = ((abs(params_0.pass_0.scale_0 - 1.0f)) > 0.0099999997764825821f);
    return px_5;
}

fn pow0_0( x_4 : f32,  y_1 : f32) -> f32
{
    if((x_4 <= 0.0f))
    {
        return 0.0f;
    }
    return pow(x_4, y_1);
}

fn lab_f_0( x_5 : f32) -> f32
{
    if((x_5 > 0.0088564520701766014f))
    {
        return pow0_0(x_5, 0.3333333432674408f);
    }
    return (903.29632568359375f * x_5 + 16.0f) / 116.0f;
}

fn xyz_to_lab_d50_0( xyz_0 : vec3<f32>) -> vec3<f32>
{
    var x_6 : vec3<f32> = xyz_0 / vec3<f32>(0.96420001983642578f, 1.0f, 0.82489997148513794f);
    var _S331 : f32 = lab_f_0(x_6.y);
    return vec3<f32>(116.0f * _S331 - 16.0f, 500.0f * (lab_f_0(x_6.x) - _S331), -200.0f * (lab_f_0(x_6.z) - _S331));
}

fn rgb_to_lab_0( rgb_0 : vec3<f32>,  c_9 : Colour_0) -> vec3<f32>
{
    return xyz_to_lab_d50_0((((rgb_0) * (c_9.rgb_to_xyz_0))));
}

fn grain_amount_0( noise_1 : f32,  strength_2 : f32) -> f32
{
    return noise_1 * (strength_2 / 100.0f) * 0.15000000596046448f;
}

fn s_fwd_grain_amount_0( dpnoise_0 : DiffPair_float_0,  dpstrength_0 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S332 : f32 = dpstrength_0.primal_0 / 100.0f;
    var _S333 : DiffPair_float_0 = DiffPair_float_0( dpnoise_0.primal_0 * _S332 * 0.15000000596046448f, (dpnoise_0.differential_0 * _S332 + dpstrength_0.differential_0 * 0.0099999997764825821f * dpnoise_0.primal_0) * 0.15000000596046448f );
    return _S333;
}

fn cell_of_0( u_0 : f32,  l_6 : f32) -> vec2<i32>
{
    return vec2<i32>(i32(min(clamp((u_0 + 0.5f) * 127.0f, 0.0f, 127.0f), 126.0f)), i32(min(clamp(l_6 * 127.0f, 0.0f, 127.0f), 126.0f)));
}

fn cell_for_0( lab_2 : vec3<f32>,  n_0 : f32,  g_3 : Grain_0) -> vec2<i32>
{
    return cell_of_0(grain_amount_0(n_0, g_3.strength_1), lab_2.x / 100.0f);
}

fn delta_of_0( midtones_bias_2 : f32) -> f32
{
    return 2.0f * exp(midtones_bias_2 / 100.0f * log(9.9999997473787516e-05f));
}

fn s_fwd_delta_of_0( dpmidtones_bias_0 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S334 : f32 = dpmidtones_bias_0.primal_0 / 100.0f;
    var _S335 : DiffPair_float_0 = DiffPair_float_0( 9.9999997473787516e-05f, 0.0f );
    var _S336 : DiffPair_float_0 = _d_log_1(_S335);
    var _S337 : DiffPair_float_0 = DiffPair_float_0( _S334 * _S336.primal_0, dpmidtones_bias_0.differential_0 * 0.0099999997764825821f * _S336.primal_0 + _S336.differential_0 * _S334 );
    var _S338 : DiffPair_float_0 = _d_exp_1(_S337);
    var _S339 : DiffPair_float_0 = DiffPair_float_0( 2.0f * _S338.primal_0, _S338.differential_0 * 2.0f );
    return _S339;
}

fn paper_inverse_0( density_0 : f32,  delta_0 : f32) -> f32
{
    var span_4 : f32 = 1.0f + 2.0f * delta_0;
    return - log(span_4 / (density_0 + delta_0) - 1.0f) * span_4 / 4.0f + 0.5f;
}

fn s_fwd_paper_inverse_0( dpdensity_0 : DiffPair_float_0,  dpdelta_0 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S340 : f32 = dpdelta_0.differential_0 * 2.0f;
    var span_5 : f32 = 1.0f + 2.0f * dpdelta_0.primal_0;
    var _S341 : f32 = dpdensity_0.primal_0 + dpdelta_0.primal_0;
    var _S342 : DiffPair_float_0 = DiffPair_float_0( span_5 / _S341 - 1.0f, (_S340 * _S341 - span_5 * (dpdensity_0.differential_0 + dpdelta_0.differential_0)) / (_S341 * _S341) );
    var _S343 : DiffPair_float_0 = _d_log_1(_S342);
    var _S344 : f32 = - _S343.primal_0;
    var _S345 : DiffPair_float_0 = DiffPair_float_0( _S344 * span_5 / 4.0f + 0.5f, (- _S343.differential_0 * span_5 + _S340 * _S344) * 0.25f );
    return _S345;
}

fn paper_0( exposure_0 : f32,  delta_1 : f32) -> f32
{
    var span_6 : f32 = 1.0f + 2.0f * delta_1;
    return span_6 / (1.0f + exp(4.0f * (0.5f - exposure_0) / span_6)) - delta_1;
}

fn s_fwd_paper_0( dpexposure_0 : DiffPair_float_0,  dpdelta_1 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S346 : f32 = dpdelta_1.differential_0 * 2.0f;
    var span_7 : f32 = 1.0f + 2.0f * dpdelta_1.primal_0;
    var _S347 : f32 = 4.0f * (0.5f - dpexposure_0.primal_0);
    var _S348 : DiffPair_float_0 = DiffPair_float_0( _S347 / span_7, ((0.0f - dpexposure_0.differential_0) * 4.0f * span_7 - _S347 * _S346) / (span_7 * span_7) );
    var _S349 : DiffPair_float_0 = _d_exp_1(_S348);
    var _S350 : f32 = 1.0f + _S349.primal_0;
    var _S351 : DiffPair_float_0 = DiffPair_float_0( span_7 / _S350 - dpdelta_1.primal_0, (_S346 * _S350 - span_7 * _S349.differential_0) / (_S350 * _S350) - dpdelta_1.differential_0 );
    return _S351;
}

fn lut_entry_0( i_1 : f32,  j_1 : f32,  delta_2 : f32) -> f32
{
    var l_7 : f32 = j_1 / 127.0f;
    return 100.0f * (paper_0(i_1 / 127.0f - 0.5f + paper_inverse_0(l_7, delta_2), delta_2) - l_7);
}

fn s_fwd_lut_entry_0( i_2 : f32,  j_2 : f32,  dpdelta_2 : DiffPair_float_0) -> DiffPair_float_0
{
    var l_8 : f32 = j_2 / 127.0f;
    var _S352 : DiffPair_float_0 = DiffPair_float_0( l_8, 0.0f );
    var _S353 : DiffPair_float_0 = DiffPair_float_0( dpdelta_2.primal_0, dpdelta_2.differential_0 );
    var _S354 : DiffPair_float_0 = s_fwd_paper_inverse_0(_S352, _S353);
    var _S355 : DiffPair_float_0 = DiffPair_float_0( i_2 / 127.0f - 0.5f + _S354.primal_0, _S354.differential_0 );
    var _S356 : DiffPair_float_0 = s_fwd_paper_0(_S355, _S353);
    var _S357 : DiffPair_float_0 = DiffPair_float_0( 100.0f * (_S356.primal_0 - l_8), _S356.differential_0 * 100.0f );
    return _S357;
}

fn lookup_0( u_1 : f32,  l_9 : f32,  delta_3 : f32,  cell_3 : vec2<i32>) -> f32
{
    var x0_2 : f32 = f32(cell_3.x);
    var y0_2 : f32 = f32(cell_3.y);
    var x_diff_2 : f32 = clamp((u_1 + 0.5f) * 127.0f, 0.0f, 127.0f) - x0_2;
    var y_diff_2 : f32 = clamp(l_9 * 127.0f, 0.0f, 127.0f) - y0_2;
    var _S358 : f32 = x0_2 + 1.0f;
    var _S359 : f32 = y0_2 + 1.0f;
    var _S360 : f32 = 1.0f - y_diff_2;
    return (_S360 * lut_entry_0(x0_2, y0_2, delta_3) + lut_entry_0(x0_2, _S359, delta_3) * y_diff_2) * (1.0f - x_diff_2) + (_S360 * lut_entry_0(_S358, y0_2, delta_3) + lut_entry_0(_S358, _S359, delta_3) * y_diff_2) * x_diff_2;
}

fn s_fwd_lookup_0( dpu_0 : DiffPair_float_0,  l_10 : f32,  dpdelta_3 : DiffPair_float_0,  cell_4 : vec2<i32>) -> DiffPair_float_0
{
    var _S361 : DiffPair_float_0 = DiffPair_float_0( (dpu_0.primal_0 + 0.5f) * 127.0f, dpu_0.differential_0 * 127.0f );
    var _S362 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S363 : DiffPair_float_0 = DiffPair_float_0( 127.0f, 0.0f );
    var _S364 : DiffPair_float_0 = _d_clamp_1(_S361, _S362, _S363);
    var _S365 : DiffPair_float_0 = DiffPair_float_0( l_10 * 127.0f, 0.0f );
    var _S366 : DiffPair_float_0 = _d_clamp_1(_S365, _S362, _S363);
    var x0_3 : f32 = f32(cell_4.x);
    var y0_3 : f32 = f32(cell_4.y);
    var x_diff_3 : f32 = _S364.primal_0 - x0_3;
    var y_diff_3 : f32 = _S366.primal_0 - y0_3;
    var _S367 : DiffPair_float_0 = DiffPair_float_0( dpdelta_3.primal_0, dpdelta_3.differential_0 );
    var _S368 : DiffPair_float_0 = s_fwd_lut_entry_0(x0_3, y0_3, _S367);
    var _S369 : f32 = x0_3 + 1.0f;
    var _S370 : DiffPair_float_0 = s_fwd_lut_entry_0(_S369, y0_3, _S367);
    var _S371 : f32 = y0_3 + 1.0f;
    var _S372 : DiffPair_float_0 = s_fwd_lut_entry_0(x0_3, _S371, _S367);
    var _S373 : DiffPair_float_0 = s_fwd_lut_entry_0(_S369, _S371, _S367);
    var _S374 : f32 = 1.0f - y_diff_3;
    var _S375 : f32 = 0.0f - _S366.differential_0;
    var xy0_0 : f32 = _S374 * _S368.primal_0 + _S372.primal_0 * y_diff_3;
    var xy1_0 : f32 = _S374 * _S370.primal_0 + _S373.primal_0 * y_diff_3;
    var _S376 : f32 = 1.0f - x_diff_3;
    var _S377 : DiffPair_float_0 = DiffPair_float_0( xy0_0 * _S376 + xy1_0 * x_diff_3, (_S375 * _S368.primal_0 + _S368.differential_0 * _S374 + (_S372.differential_0 * y_diff_3 + _S366.differential_0 * _S372.primal_0)) * _S376 + (0.0f - _S364.differential_0) * xy0_0 + ((_S375 * _S370.primal_0 + _S370.differential_0 * _S374 + (_S373.differential_0 * y_diff_3 + _S366.differential_0 * _S373.primal_0)) * x_diff_3 + _S364.differential_0 * xy1_0) );
    return _S377;
}

fn lab_f_inv_0( x_7 : f32) -> f32
{
    if((x_7 > 0.20689655840396881f))
    {
        return x_7 * x_7 * x_7;
    }
    return (116.0f * x_7 - 16.0f) / 903.29632568359375f;
}

fn s_fwd_lab_f_inv_0( dpx_9 : DiffPair_float_0) -> DiffPair_float_0
{
    if(((dpx_9.primal_0) > 0.20689655840396881f))
    {
        var _S378 : f32 = dpx_9.primal_0 * dpx_9.primal_0;
        var _S379 : f32 = dpx_9.differential_0 * dpx_9.primal_0;
        var _S380 : DiffPair_float_0 = DiffPair_float_0( _S378 * dpx_9.primal_0, (_S379 + _S379) * dpx_9.primal_0 + dpx_9.differential_0 * _S378 );
        return _S380;
    }
    var _S381 : DiffPair_float_0 = DiffPair_float_0( (116.0f * dpx_9.primal_0 - 16.0f) / 903.29632568359375f, dpx_9.differential_0 * 116.0f * 0.0011070563923567533f );
    return _S381;
}

fn lab_to_xyz_d50_0( lab_3 : vec3<f32>) -> vec3<f32>
{
    var fy_2 : f32 = (lab_3.x + 16.0f) * 0.0086206896230578423f;
    return vec3<f32>(0.96420001983642578f, 1.0f, 0.82489997148513794f) * vec3<f32>(lab_f_inv_0(lab_3.y * 0.0020000000949949026f + fy_2), lab_f_inv_0(fy_2), lab_f_inv_0(lab_3.z * -0.004999999888241291f + fy_2));
}

fn s_fwd_lab_to_xyz_d50_0( dplab_0 : DiffPair_vectorx3Cfloatx2C3x3E_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var fy_3 : f32 = (dplab_0.primal_0.x + 16.0f) * 0.0086206896230578423f;
    var _S382 : f32 = dplab_0.differential_0.x * 0.0086206896230578423f;
    var _S383 : DiffPair_float_0 = DiffPair_float_0( dplab_0.primal_0.y * 0.0020000000949949026f + fy_3, dplab_0.differential_0.y * 0.0020000000949949026f + _S382 );
    var _S384 : DiffPair_float_0 = s_fwd_lab_f_inv_0(_S383);
    var _S385 : DiffPair_float_0 = DiffPair_float_0( fy_3, _S382 );
    var _S386 : DiffPair_float_0 = s_fwd_lab_f_inv_0(_S385);
    var _S387 : DiffPair_float_0 = DiffPair_float_0( dplab_0.primal_0.z * -0.004999999888241291f + fy_3, dplab_0.differential_0.z * -0.004999999888241291f + _S382 );
    var _S388 : DiffPair_float_0 = s_fwd_lab_f_inv_0(_S387);
    var _S389 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(0.96420001983642578f, 1.0f, 0.82489997148513794f) * vec3<f32>(_S384.primal_0, _S386.primal_0, _S388.primal_0), vec3<f32>(_S384.differential_0, _S386.differential_0, _S388.differential_0) * vec3<f32>(0.96420001983642578f, 1.0f, 0.82489997148513794f) );
    return _S389;
}

fn lab_to_rgb_0( lab_4 : vec3<f32>,  c_10 : Colour_0) -> vec3<f32>
{
    return (((lab_to_xyz_d50_0(lab_4)) * (c_10.xyz_to_rgb_0)));
}

fn s_fwd_lab_to_rgb_0( dplab_1 : DiffPair_vectorx3Cfloatx2C3x3E_0,  c_11 : Colour_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S390 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( dplab_1.primal_0, dplab_1.differential_0 );
    var _S391 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_lab_to_xyz_d50_0(_S390);
    var _S392 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( (((_S391.primal_0) * (c_11.xyz_to_rgb_0))), (((_S391.primal_0) * (mat3x3<f32>(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f)))) + (((_S391.differential_0) * (c_11.xyz_to_rgb_0))) );
    return _S392;
}

fn grain_1( lab_5 : vec3<f32>,  noise_2 : f32,  g_4 : Grain_0,  c_12 : Colour_0,  cell_5 : vec2<i32>) -> vec3<f32>
{
    var _S393 : f32 = lab_5.x;
    return lab_to_rgb_0(vec3<f32>(_S393 + lookup_0(grain_amount_0(noise_2, g_4.strength_1), _S393 / 100.0f, delta_of_0(g_4.midtones_bias_1), cell_5), lab_5.y, lab_5.z), c_12);
}

fn s_fwd_grain_0( lab_6 : vec3<f32>,  dpnoise_1 : DiffPair_float_0,  dpg_0 : DiffPair_Grain_0,  c_13 : Colour_0,  cell_6 : vec2<i32>) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S394 : DiffPair_float_0 = DiffPair_float_0( dpnoise_1.primal_0, dpnoise_1.differential_0 );
    var _S395 : DiffPair_float_0 = DiffPair_float_0( dpg_0.primal_0.strength_1, dpg_0.differential_0.strength_1 );
    var _S396 : DiffPair_float_0 = s_fwd_grain_amount_0(_S394, _S395);
    var _S397 : f32 = lab_6.x;
    var _S398 : DiffPair_float_0 = DiffPair_float_0( dpg_0.primal_0.midtones_bias_1, dpg_0.differential_0.midtones_bias_1 );
    var _S399 : DiffPair_float_0 = s_fwd_delta_of_0(_S398);
    var _S400 : DiffPair_float_0 = DiffPair_float_0( _S396.primal_0, _S396.differential_0 );
    var _S401 : DiffPair_float_0 = DiffPair_float_0( _S399.primal_0, _S399.differential_0 );
    var _S402 : DiffPair_float_0 = s_fwd_lookup_0(_S400, _S397 / 100.0f, _S401, cell_6);
    var _S403 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S397 + _S402.primal_0, lab_6.y, lab_6.z), vec3<f32>(_S402.differential_0, 0.0f, 0.0f) );
    var _S404 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_lab_to_rgb_0(_S403, c_13);
    var _S405 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S404.primal_0, _S404.differential_0 );
    return _S405;
}

fn grain_of_0() -> Grain_0
{
    var g_5 : Grain_0;
    g_5.strength_1 = params_0.grain_0.strength_0;
    g_5.scale_7 = params_0.grain_0.scale_1;
    g_5.midtones_bias_1 = params_0.grain_0.midtones_bias_0;
    return g_5;
}

fn colour_of_0() -> Colour_0
{
    var c_14 : Colour_0;
    c_14.rgb_to_xyz_0 = mat3x3<f32>(params_0.grain_0.rgb_to_xyz_r0_0.xyz, params_0.grain_0.rgb_to_xyz_r1_0.xyz, params_0.grain_0.rgb_to_xyz_r2_0.xyz);
    c_14.xyz_to_rgb_0 = mat3x3<f32>(params_0.grain_0.xyz_to_rgb_r0_0.xyz, params_0.grain_0.xyz_to_rgb_r1_0.xyz, params_0.grain_0.xyz_to_rgb_r2_0.xyz);
    return c_14;
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_1 : vec3<u32>)
{
    var _S406 : u32 = id_1.x;
    var _S407 : bool;
    if((_S406 >= (params_0.pass_0.width_0)))
    {
        _S407 = true;
    }
    else
    {
        _S407 = ((id_1.y) >= (params_0.pass_0.height_0));
    }
    if(_S407)
    {
        return;
    }
    var i_3 : u32 = id_1.y * params_0.pass_0.width_0 + _S406;
    var _S408 : Grain_0 = grain_of_0();
    var _S409 : Colour_0 = colour_of_0();
    var _S410 : f32 = noise_0(_S408.scale_7, pixel_of_0(id_1));
    var pixel_0 : vec4<f32> = input_0[i_3];
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
        var lab_7 : vec3<f32> = rgb_to_lab_0(rgb_1, _S409);
        var _S411 : vec3<f32> = grain_1(lab_7, _S410, _S408, _S409, cell_for_0(lab_7, _S410, _S408));
        var _S412 : u32 = k_0 + u32(1);
        rgb_1 = _S411;
        k_0 = _S412;
    }
    output_0[i_3] = vec4<f32>(rgb_1, pixel_0.w);
    return;
}

fn s_fwd_grain_from_settings_0( lab_8 : vec3<f32>,  dpg_1 : DiffPair_Grain_0,  px_6 : Pixel_0,  c_15 : Colour_0,  cell_7 : vec2<i32>) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S413 : DiffPair_float_0 = DiffPair_float_0( dpg_1.primal_0.scale_7, dpg_1.differential_0.scale_7 );
    var _S414 : DiffPair_float_0 = noise_forward_0(_S413, px_6);
    var _S415 : DiffPair_float_0 = DiffPair_float_0( _S414.primal_0, _S414.differential_0 );
    var _S416 : DiffPair_Grain_0 = DiffPair_Grain_0( dpg_1.primal_0, dpg_1.differential_0 );
    var _S417 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_grain_0(lab_8, _S415, _S416, c_15, cell_7);
    var _S418 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S417.primal_0, _S417.differential_0 );
    return _S418;
}

fn pick_0( d_2 : Grain_0,  which_0 : u32) -> f32
{
    switch(which_0)
    {
    case u32(0):
        {
            return d_2.strength_1;
        }
    case u32(1):
        {
            return d_2.scale_7;
        }
    case u32(2):
        {
            return d_2.midtones_bias_1;
        }
    default :
        {
            return 0.0f;
        }
    }
}

@compute
@workgroup_size(16, 16, 1)
fn grain_backward(@builtin(global_invocation_id) id_2 : vec3<u32>)
{
    var _S419 : u32 = id_2.x;
    var _S420 : bool;
    if((_S419 >= (params_0.pass_0.width_0)))
    {
        _S420 = true;
    }
    else
    {
        _S420 = ((id_2.y) >= (params_0.pass_0.height_0));
    }
    if(_S420)
    {
        return;
    }
    var i_4 : u32 = id_2.y * params_0.pass_0.width_0 + _S419;
    var _S421 : Grain_0 = grain_of_0();
    var _S422 : Colour_0 = colour_of_0();
    var px_7 : Pixel_0 = pixel_of_0(id_2);
    var lab_9 : vec3<f32> = rgb_to_lab_0(input_0[i_4].xyz, _S422);
    var cell_8 : vec2<i32> = cell_for_0(lab_9, noise_0(_S421.scale_7, px_7), _S421);
    var _S423 : Grain_0 = Grain_x24_syn_dzero_0();
    var d_3 : DiffPair_Grain_0;
    d_3.primal_0 = _S421;
    d_3.differential_0 = _S423;
    _S313(lab_9, &(d_3), px_7, _S422, cell_8, derivative_in_0[i_4].xyz);
    derivative_out_0[i_4] = vec4<f32>(pick_0(d_3.differential_0, params_0.grain_0.gradient_0.x), pick_0(d_3.differential_0, params_0.grain_0.gradient_0.y), pick_0(d_3.differential_0, params_0.grain_0.gradient_0.z), pick_0(d_3.differential_0, params_0.grain_0.gradient_0.w));
    return;
}

fn add_slot_0( d_4 : ptr<function, Grain_0>,  which_1 : u32,  x_8 : f32)
{
    var _S424 : f32;
    if(which_1 == u32(0))
    {
        _S424 = x_8;
    }
    else
    {
        _S424 = 0.0f;
    }
    (*d_4).strength_1 = (*d_4).strength_1 + _S424;
    if(which_1 == u32(1))
    {
        _S424 = x_8;
    }
    else
    {
        _S424 = 0.0f;
    }
    (*d_4).scale_7 = (*d_4).scale_7 + _S424;
    if(which_1 == u32(2))
    {
        _S424 = x_8;
    }
    else
    {
        _S424 = 0.0f;
    }
    (*d_4).midtones_bias_1 = (*d_4).midtones_bias_1 + _S424;
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn grain_forward_diff(@builtin(global_invocation_id) id_3 : vec3<u32>)
{
    var _S425 : u32 = id_3.x;
    var _S426 : bool;
    if((_S425 >= (params_0.pass_0.width_0)))
    {
        _S426 = true;
    }
    else
    {
        _S426 = ((id_3.y) >= (params_0.pass_0.height_0));
    }
    if(_S426)
    {
        return;
    }
    var i_5 : u32 = id_3.y * params_0.pass_0.width_0 + _S425;
    var _S427 : Grain_0 = grain_of_0();
    var _S428 : Colour_0 = colour_of_0();
    var px_8 : Pixel_0 = pixel_of_0(id_3);
    var lab_10 : vec3<f32> = rgb_to_lab_0(input_0[i_5].xyz, _S428);
    var cell_9 : vec2<i32> = cell_for_0(lab_10, noise_0(_S427.scale_7, px_8), _S427);
    var direction_in_0 : vec4<f32> = derivative_in_0[i_5];
    var direction_0 : Grain_0;
    direction_0.strength_1 = 0.0f;
    direction_0.scale_7 = 0.0f;
    direction_0.midtones_bias_1 = 0.0f;
    add_slot_0(&(direction_0), params_0.grain_0.gradient_0.x, direction_in_0.x);
    add_slot_0(&(direction_0), params_0.grain_0.gradient_0.y, direction_in_0.y);
    add_slot_0(&(direction_0), params_0.grain_0.gradient_0.z, direction_in_0.z);
    add_slot_0(&(direction_0), params_0.grain_0.gradient_0.w, direction_in_0.w);
    var _S429 : DiffPair_Grain_0 = DiffPair_Grain_0( _S427, direction_0 );
    derivative_out_0[i_5] = vec4<f32>(s_fwd_grain_from_settings_0(lab_10, _S429, px_8, _S428, cell_9).differential_0, 0.0f);
    return;
}

