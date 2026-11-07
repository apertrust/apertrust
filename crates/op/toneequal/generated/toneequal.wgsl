// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2018-2026  darktable developers (ported from src/iop/toneequal.c,
// src/common/luminance_mask.h and src/common/eigf.h)
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

// Generated from slang/toneequal.slang by tools/build-kernels.sh. Do not edit.

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

struct ToneEqualParams_std140_0
{
    @align(16) weights_low_0 : vec4<f32>,
    @align(16) weights_high_0 : vec4<f32>,
    @align(16) direction0_low_0 : vec4<f32>,
    @align(16) direction0_high_0 : vec4<f32>,
    @align(16) direction1_low_0 : vec4<f32>,
    @align(16) direction1_high_0 : vec4<f32>,
    @align(16) direction2_low_0 : vec4<f32>,
    @align(16) direction2_high_0 : vec4<f32>,
    @align(16) direction3_low_0 : vec4<f32>,
    @align(16) direction3_high_0 : vec4<f32>,
    @align(16) inverse_denominator_0 : f32,
    @align(4) pad0_0 : u32,
    @align(8) pad1_0 : u32,
    @align(4) pad2_0 : u32,
};

struct MaskParams_std140_0
{
    @align(16) method_0 : u32,
    @align(4) filter_0 : u32,
    @align(8) iterations_0 : u32,
    @align(4) quantised_0 : u32,
    @align(16) exposure_boost_0 : f32,
    @align(4) fulcrum_0 : f32,
    @align(8) contrast_boost_0 : f32,
    @align(4) feathering_0 : f32,
    @align(16) quantization_0 : f32,
    @align(4) partials_across_0 : u32,
    @align(8) partials_0 : u32,
    @align(4) pad_0 : u32,
    @align(16) width_1 : u32,
    @align(4) height_1 : u32,
    @align(8) grid_width_0 : u32,
    @align(4) grid_height_0 : u32,
    @align(16) input_width_0 : u32,
    @align(4) input_height_0 : u32,
    @align(8) pad3_0 : u32,
    @align(4) pad4_0 : u32,
    @align(16) a_0 : vec4<f32>,
    @align(16) b_0 : vec4<f32>,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) tone_equal_0 : ToneEqualParams_std140_0,
    @align(16) mask_0 : MaskParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(0) @group(0) var<storage, read> input_0 : array<vec4<f32>>;

@binding(17) @group(0) var<storage, read_write> mask_1 : array<f32>;

@binding(15) @group(0) var<storage, read_write> statistics_guide_0 : array<vec2<f32>>;

@binding(16) @group(0) var<storage, read_write> statistics_mask_0 : array<vec2<f32>>;

@binding(1) @group(0) var<storage, read_write> output_0 : array<vec4<f32>>;

@binding(10) @group(0) var<storage, read_write> samples_0 : array<vec2<f32>>;

@binding(11) @group(0) var<storage, read_write> partials_1 : array<vec4<f32>>;

@binding(12) @group(0) var<storage, read_write> ranges_0 : array<vec4<f32>>;

@binding(13) @group(0) var<storage, read_write> columns_guide_0 : array<vec2<f32>>;

@binding(14) @group(0) var<storage, read_write> columns_mask_0 : array<vec2<f32>>;

@binding(8) @group(0) var<storage, read> derivative_in_0 : array<vec4<f32>>;

@binding(9) @group(0) var<storage, read_write> derivative_out_0 : array<vec4<f32>>;

struct s_bwdCallableCtx_clamp_0
{
     _S1 : f32,
     _S2 : f32,
     _S3 : f32,
};

struct DiffPair_float_0
{
     primal_0 : f32,
     differential_0 : f32,
};

fn _d_clamp_0( dpx_0 : ptr<function, DiffPair_float_0>,  dpMin_0 : ptr<function, DiffPair_float_0>,  dpMax_0 : ptr<function, DiffPair_float_0>,  dOut_0 : f32)
{
    var _S4 : DiffPair_float_0 = (*dpx_0);
    var _S5 : bool;
    if((((*dpx_0).primal_0) >= ((*dpMin_0).primal_0)))
    {
        _S5 = (((*dpx_0).primal_0) <= ((*dpMax_0).primal_0));
    }
    else
    {
        _S5 = false;
    }
    var _S6 : f32;
    if(_S5)
    {
        _S6 = dOut_0;
    }
    else
    {
        _S6 = 0.0f;
    }
    (*dpx_0).primal_0 = _S4.primal_0;
    (*dpx_0).differential_0 = _S6;
    var _S7 : DiffPair_float_0 = (*dpMin_0);
    if(((_S4.primal_0) < ((*dpMin_0).primal_0)))
    {
        _S6 = dOut_0;
    }
    else
    {
        _S6 = 0.0f;
    }
    (*dpMin_0).primal_0 = _S7.primal_0;
    (*dpMin_0).differential_0 = _S6;
    var _S8 : DiffPair_float_0 = (*dpMax_0);
    if((((*dpx_0).primal_0) > ((*dpMax_0).primal_0)))
    {
        _S6 = dOut_0;
    }
    else
    {
        _S6 = 0.0f;
    }
    (*dpMax_0).primal_0 = _S8.primal_0;
    (*dpMax_0).differential_0 = _S6;
    return;
}

fn s_bwdProp_clamp_0( ctx_0 : s_bwdCallableCtx_clamp_0,  d_x_0 : ptr<function, f32>,  d_minBound_0 : ptr<function, f32>,  d_maxBound_0 : ptr<function, f32>,  d_out_0 : f32)
{
    var _S9 : DiffPair_float_0;
    _S9.primal_0 = ctx_0._S1;
    var _S10 : DiffPair_float_0;
    _S10.primal_0 = ctx_0._S2;
    var _S11 : DiffPair_float_0;
    _S11.primal_0 = ctx_0._S3;
    _d_clamp_0(&(_S9), &(_S10), &(_S11), d_out_0);
    (*d_x_0) = _S9.differential_0;
    (*d_minBound_0) = _S10.differential_0;
    (*d_maxBound_0) = _S11.differential_0;
    return;
}

struct Tuple_0
{
     value0_0 : f32,
};

fn s_apply_dot_0( x_0 : vec4<f32>,  y_0 : vec4<f32>) -> Tuple_0
{
    var _S12 : Tuple_0 = Tuple_0( dot(x_0, y_0) );
    return _S12;
}

struct s_bwdCallableCtx_dot_0
{
     _S13 : vec4<f32>,
     _S14 : vec4<f32>,
};

struct DiffPair_vectorx3Cfloatx2C4x3E_0
{
     primal_0 : vec4<f32>,
     differential_0 : vec4<f32>,
};

fn _d_dot_0( dpx_1 : ptr<function, DiffPair_vectorx3Cfloatx2C4x3E_0>,  dpy_0 : ptr<function, DiffPair_vectorx3Cfloatx2C4x3E_0>,  dOut_1 : f32)
{
    var x_d_result_0 : vec4<f32>;
    x_d_result_0[i32(0)] = (*dpy_0).primal_0[i32(0)] * dOut_1;
    var y_d_result_0 : vec4<f32>;
    y_d_result_0[i32(0)] = (*dpx_1).primal_0[i32(0)] * dOut_1;
    x_d_result_0[i32(1)] = (*dpy_0).primal_0[i32(1)] * dOut_1;
    y_d_result_0[i32(1)] = (*dpx_1).primal_0[i32(1)] * dOut_1;
    x_d_result_0[i32(2)] = (*dpy_0).primal_0[i32(2)] * dOut_1;
    y_d_result_0[i32(2)] = (*dpx_1).primal_0[i32(2)] * dOut_1;
    x_d_result_0[i32(3)] = (*dpy_0).primal_0[i32(3)] * dOut_1;
    y_d_result_0[i32(3)] = (*dpx_1).primal_0[i32(3)] * dOut_1;
    (*dpx_1).primal_0 = (*dpx_1).primal_0;
    (*dpx_1).differential_0 = x_d_result_0;
    (*dpy_0).primal_0 = (*dpy_0).primal_0;
    (*dpy_0).differential_0 = y_d_result_0;
    return;
}

fn s_bwdProp_dot_0( ctx_1 : s_bwdCallableCtx_dot_0,  d_x_1 : ptr<function, vec4<f32>>,  d_y_0 : ptr<function, vec4<f32>>,  d_out_1 : f32)
{
    var _S15 : DiffPair_vectorx3Cfloatx2C4x3E_0;
    _S15.primal_0 = ctx_1._S13;
    var _S16 : DiffPair_vectorx3Cfloatx2C4x3E_0;
    _S16.primal_0 = ctx_1._S14;
    _d_dot_0(&(_S15), &(_S16), d_out_1);
    (*d_x_1) = _S15.differential_0;
    (*d_y_0) = _S16.differential_0;
    return;
}

fn _d_dot_1( dpx_2 : DiffPair_vectorx3Cfloatx2C4x3E_0,  dpy_1 : DiffPair_vectorx3Cfloatx2C4x3E_0) -> DiffPair_float_0
{
    var _S17 : DiffPair_float_0 = DiffPair_float_0( dpx_2.primal_0[i32(0)] * dpy_1.primal_0[i32(0)] + dpx_2.primal_0[i32(1)] * dpy_1.primal_0[i32(1)] + dpx_2.primal_0[i32(2)] * dpy_1.primal_0[i32(2)] + dpx_2.primal_0[i32(3)] * dpy_1.primal_0[i32(3)], dpx_2.primal_0[i32(0)] * dpy_1.differential_0[i32(0)] + dpy_1.primal_0[i32(0)] * dpx_2.differential_0[i32(0)] + dpx_2.primal_0[i32(1)] * dpy_1.differential_0[i32(1)] + dpy_1.primal_0[i32(1)] * dpx_2.differential_0[i32(1)] + dpx_2.primal_0[i32(2)] * dpy_1.differential_0[i32(2)] + dpy_1.primal_0[i32(2)] * dpx_2.differential_0[i32(2)] + dpx_2.primal_0[i32(3)] * dpy_1.differential_0[i32(3)] + dpy_1.primal_0[i32(3)] * dpx_2.differential_0[i32(3)] );
    return _S17;
}

fn _d_clamp_1( dpx_3 : DiffPair_float_0,  dpMin_1 : DiffPair_float_0,  dpMax_1 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S18 : f32 = clamp(dpx_3.primal_0, dpMin_1.primal_0, dpMax_1.primal_0);
    var _S19 : bool;
    if(((dpx_3.primal_0) >= (dpMin_1.primal_0)))
    {
        _S19 = ((dpx_3.primal_0) <= (dpMax_1.primal_0));
    }
    else
    {
        _S19 = false;
    }
    var _S20 : f32;
    if(_S19)
    {
        _S20 = dpx_3.differential_0;
    }
    else
    {
        if(((dpx_3.primal_0) < (dpMin_1.primal_0)))
        {
            _S20 = dpMin_1.differential_0;
        }
        else
        {
            _S20 = dpMax_1.differential_0;
        }
    }
    var _S21 : DiffPair_float_0 = DiffPair_float_0( _S18, _S20 );
    return _S21;
}

struct Bands_0
{
     t0_0 : f32,
     t1_0 : f32,
     t2_0 : f32,
     t3_0 : f32,
};

fn Bands_x24_syn_dadd_0( SLANG_anonymous_0_0 : Bands_0,  SLANG_anonymous_1_0 : Bands_0) -> Bands_0
{
    var result_0 : Bands_0;
    result_0.t0_0 = SLANG_anonymous_0_0.t0_0 + SLANG_anonymous_1_0.t0_0;
    result_0.t1_0 = SLANG_anonymous_0_0.t1_0 + SLANG_anonymous_1_0.t1_0;
    result_0.t2_0 = SLANG_anonymous_0_0.t2_0 + SLANG_anonymous_1_0.t2_0;
    result_0.t3_0 = SLANG_anonymous_0_0.t3_0 + SLANG_anonymous_1_0.t3_0;
    return result_0;
}

fn Bands_x24_syn_dzero_0() -> Bands_0
{
    var result_1 : Bands_0;
    result_1.t0_0 = 0.0f;
    result_1.t1_0 = 0.0f;
    result_1.t2_0 = 0.0f;
    result_1.t3_0 = 0.0f;
    return result_1;
}

fn Bands_x24init_0( t0_1 : f32,  t1_1 : f32,  t2_1 : f32,  t3_1 : f32) -> Bands_0
{
    var _S22 : Bands_0;
    _S22.t0_0 = t0_1;
    _S22.t1_0 = t1_1;
    _S22.t2_0 = t2_1;
    _S22.t3_0 = t3_1;
    return _S22;
}

fn place_0( id_0 : vec3<u32>) -> vec2<u32>
{
    return vec2<u32>(params_0.pass_0.origin_x_0 + id_0.x, params_0.pass_0.origin_y_0 + id_0.y);
}

fn pow0_0( x_1 : f32,  y_1 : f32) -> f32
{
    if((x_1 <= 0.0f))
    {
        return 0.0f;
    }
    return pow(x_1, y_1);
}

fn estimate_0( rgb_0 : vec3<f32>,  m_0 : ptr<function, MaskParams_std140_0>) -> f32
{
    var c_0 : vec3<f32> = clamp(rgb_0, vec3<f32>(-1.0e+06f), vec3<f32>(1.0e+06f));
    var a_1 : vec3<f32> = abs(c_0);
    var _S23 : f32 = (*m_0).exposure_boost_0;
    var _S24 : u32 = (*m_0).method_0;
    if(((*m_0).method_0) == u32(0))
    {
        return _S23 * (c_0.x + c_0.y + c_0.z) / 3.0f;
    }
    if(_S24 == u32(1))
    {
        var _S25 : f32 = c_0.x;
        var _S26 : f32 = c_0.y;
        var _S27 : f32 = c_0.z;
        return _S23 * (max(max(_S25, _S26), _S27) + min(min(_S25, _S26), _S27)) / 2.0f;
    }
    if(_S24 == u32(2))
    {
        return _S23 * max(max(c_0.x, c_0.y), c_0.z);
    }
    if(_S24 == u32(3))
    {
        return _S23 * (a_1.x + a_1.y + a_1.z);
    }
    if(_S24 == u32(5))
    {
        return _S23 * dot(a_1 * a_1, a_1) / max(dot(a_1, a_1), 1.00000000317107685e-30f);
    }
    if(_S24 == u32(6))
    {
        return _S23 * pow0_0(max(a_1.x * a_1.y * a_1.z, 1.00000000317107685e-30f), 0.3333333432674408f);
    }
    return _S23 * sqrt(dot(c_0, c_0));
}

fn compensated_0( rgb_1 : vec3<f32>,  m_1 : ptr<function, MaskParams_std140_0>) -> f32
{
    var _S28 : f32 = estimate_0(rgb_1, &((*m_1)));
    return max((_S28 - (*m_1).fulcrum_0) * (*m_1).contrast_boost_0 + (*m_1).fulcrum_0, 1.52587890625e-05f);
}

struct Taps_0
{
     x_prev_0 : u32,
     x_next_0 : u32,
     y_prev_0 : u32,
     y_next_0 : u32,
     dx_next_0 : f32,
     dx_prev_0 : f32,
     dy_next_0 : f32,
     dy_prev_0 : f32,
};

fn bilinear_0( at_0 : vec2<u32>,  size_in_0 : vec2<u32>,  size_out_0 : vec2<u32>) -> Taps_0
{
    var _S29 : u32 = size_in_0.x;
    var x_in_0 : f32 = f32(at_0.x) / f32(size_out_0.x) * f32(_S29);
    var _S30 : u32 = size_in_0.y;
    var y_in_0 : f32 = f32(at_0.y) / f32(size_out_0.y) * f32(_S30);
    var x_prev_1 : u32 = u32(floor(x_in_0));
    var y_prev_1 : u32 = u32(floor(y_in_0));
    var t_0 : Taps_0;
    var _S31 : u32 = _S29 - u32(1);
    var _S32 : u32 = min(x_prev_1 + u32(1), _S31);
    t_0.x_next_0 = _S32;
    var _S33 : u32 = _S30 - u32(1);
    var _S34 : u32 = min(y_prev_1 + u32(1), _S33);
    t_0.y_next_0 = _S34;
    t_0.x_prev_0 = min(x_prev_1, _S31);
    t_0.y_prev_0 = min(y_prev_1, _S33);
    var _S35 : f32 = f32(_S32) - x_in_0;
    t_0.dx_next_0 = _S35;
    t_0.dx_prev_0 = 1.0f - _S35;
    var _S36 : f32 = f32(_S34) - y_in_0;
    t_0.dy_next_0 = _S36;
    t_0.dy_prev_0 = 1.0f - _S36;
    return t_0;
}

fn interpolate2_0( t_1 : Taps_0,  nw_0 : vec2<f32>,  ne_0 : vec2<f32>,  sw_0 : vec2<f32>,  se_0 : vec2<f32>) -> vec2<f32>
{
    var _S37 : vec2<f32> = vec2<f32>(t_1.dx_next_0);
    var _S38 : vec2<f32> = vec2<f32>(t_1.dx_prev_0);
    return vec2<f32>(t_1.dy_prev_0) * (sw_0 * _S37 + se_0 * _S38) + vec2<f32>(t_1.dy_next_0) * (nw_0 * _S37 + ne_0 * _S38);
}

fn statistics_at_0( at_1 : vec2<u32>,  m_2 : ptr<function, MaskParams_std140_0>) -> vec4<f32>
{
    var t_2 : Taps_0 = bilinear_0(at_1, vec2<u32>((*m_2).grid_width_0, (*m_2).grid_height_0), vec2<u32>((*m_2).width_1, (*m_2).height_1));
    var _S39 : u32 = t_2.y_prev_0 * (*m_2).grid_width_0;
    var nw_1 : u32 = _S39 + t_2.x_prev_0;
    var ne_1 : u32 = _S39 + t_2.x_next_0;
    var _S40 : u32 = t_2.y_next_0 * (*m_2).grid_width_0;
    var sw_1 : u32 = _S40 + t_2.x_prev_0;
    var se_1 : u32 = _S40 + t_2.x_next_0;
    var guide_0 : vec2<f32> = interpolate2_0(t_2, statistics_guide_0[nw_1], statistics_guide_0[ne_1], statistics_guide_0[sw_1], statistics_guide_0[se_1]);
    var _S41 : vec2<f32> = vec2<f32>(0.0f);
    var masked_0 : vec2<f32>;
    if(((*m_2).quantised_0) != u32(0))
    {
        masked_0 = interpolate2_0(t_2, statistics_mask_0[nw_1], statistics_mask_0[ne_1], statistics_mask_0[sw_1], statistics_mask_0[se_1]);
    }
    else
    {
        masked_0 = _S41;
    }
    return vec4<f32>(guide_0, masked_0);
}

fn quantize_0( value_0 : f32,  sampling_0 : f32) -> f32
{
    return max(min(exp2(floor(log2(value_0) / sampling_0) * sampling_0), 4.0f), 6.103515625e-05f);
}

fn eigf_guided_0( image_0 : f32,  mask_2 : f32,  avg_g_0 : f32,  var_g_0 : f32,  avg_m_0 : f32,  covar_mg_0 : f32,  feathering_1 : f32) -> f32
{
    var _S42 : f32 = max(avg_g_0 * image_0, 9.99999997475242708e-07f);
    var a_2 : f32 = covar_mg_0 / sqrt(_S42 * max(avg_m_0 * mask_2, 9.99999997475242708e-07f)) / (var_g_0 / _S42 + feathering_1);
    return image_0 * a_2 + (avg_m_0 - a_2 * avg_g_0);
}

fn eigf_no_mask_0( image_1 : f32,  avg_g_1 : f32,  var_g_1 : f32,  feathering_2 : f32) -> f32
{
    var normalized_var_guide_0 : f32 = var_g_1 / max(avg_g_1 * image_1, 9.99999997475242708e-07f);
    var a_3 : f32 = normalized_var_guide_0 / (normalized_var_guide_0 + feathering_2);
    return image_1 * a_3 + (avg_g_1 - a_3 * avg_g_1);
}

fn blend_0( image_2 : f32,  mapped_0 : f32,  geomean_0 : bool) -> f32
{
    var _S43 : f32 = max(mapped_0, 1.52587890625e-05f);
    var _S44 : f32;
    if(geomean_0)
    {
        _S44 = sqrt(image_2 * _S43);
    }
    else
    {
        _S44 = _S43;
    }
    return _S44;
}

fn eigf_blend_at_0( image_3 : f32,  at_2 : vec2<u32>,  m_3 : ptr<function, MaskParams_std140_0>,  geomean_1 : bool) -> f32
{
    var _S45 : vec4<f32> = statistics_at_0(at_2, &((*m_3)));
    var mapped_1 : f32;
    if(((*m_3).quantised_0) != u32(0))
    {
        mapped_1 = eigf_guided_0(image_3, quantize_0(image_3, (*m_3).quantization_0), _S45.x, _S45.y, _S45.z, _S45.w, (*m_3).feathering_0);
    }
    else
    {
        mapped_1 = eigf_no_mask_0(image_3, _S45.x, _S45.y, (*m_3).feathering_0);
    }
    return blend_0(image_3, mapped_1, geomean_1);
}

fn mask_value_0( rgb_2 : vec3<f32>,  at_3 : vec2<u32>,  m_4 : ptr<function, MaskParams_std140_0>) -> f32
{
    var _S46 : u32 = (*m_4).filter_0;
    if(((*m_4).filter_0) == u32(0))
    {
        var _S47 : f32 = compensated_0(rgb_2, &((*m_4)));
        return _S47;
    }
    var image_4 : f32;
    if((((*m_4).iterations_0) > u32(1)))
    {
        image_4 = mask_1[at_3.y * (*m_4).width_1 + at_3.x];
    }
    else
    {
        var _S48 : f32 = compensated_0(rgb_2, &((*m_4)));
        image_4 = _S48;
    }
    var _S49 : f32 = eigf_blend_at_0(image_4, at_3, &((*m_4)), _S46 == u32(3));
    return _S49;
}

fn mask_exposure_0( rgb_3 : vec3<f32>,  at_4 : vec2<u32>,  m_5 : ptr<function, MaskParams_std140_0>) -> f32
{
    var _S50 : f32 = mask_value_0(rgb_3, at_4, &((*m_5)));
    return clamp(log2(_S50), -8.0f, 0.0f);
}

struct Gaussians_0
{
     low_0 : vec4<f32>,
     high_0 : vec4<f32>,
};

struct ToneEqualParams_0
{
     weights_low_0 : vec4<f32>,
     weights_high_0 : vec4<f32>,
     direction0_low_0 : vec4<f32>,
     direction0_high_0 : vec4<f32>,
     direction1_low_0 : vec4<f32>,
     direction1_high_0 : vec4<f32>,
     direction2_low_0 : vec4<f32>,
     direction2_high_0 : vec4<f32>,
     direction3_low_0 : vec4<f32>,
     direction3_high_0 : vec4<f32>,
     inverse_denominator_0 : f32,
     pad0_0 : u32,
     pad1_0 : u32,
     pad2_0 : u32,
};

struct s_paramCtx_s_bwdCallableCtx_correction_0
{
     value0_1 : Gaussians_0,
     value1_0 : ToneEqualParams_0,
     value2_0 : Bands_0,
};

struct s_bwdCallableCtx_correction_0
{
     value0_2 : s_paramCtx_s_bwdCallableCtx_correction_0,
};

fn s_bwdProp_correction_0( _S51 : s_bwdCallableCtx_correction_0,  _S52 : ptr<function, Bands_0>,  _S53 : f32)
{
    var _S54 : vec4<f32> = vec4<f32>(_S51.value0_2.value2_0.t0_0);
    var _S55 : vec4<f32> = vec4<f32>(_S51.value0_2.value2_0.t1_0);
    var _S56 : vec4<f32> = vec4<f32>(_S51.value0_2.value2_0.t2_0);
    var _S57 : vec4<f32> = vec4<f32>(_S51.value0_2.value2_0.t3_0);
    var weights_low_1 : vec4<f32> = _S51.value0_2.value1_0.weights_low_0 + _S54 * _S51.value0_2.value1_0.direction0_low_0 + _S55 * _S51.value0_2.value1_0.direction1_low_0 + _S56 * _S51.value0_2.value1_0.direction2_low_0 + _S57 * _S51.value0_2.value1_0.direction3_low_0;
    var weights_high_1 : vec4<f32> = _S51.value0_2.value1_0.weights_high_0 + _S54 * _S51.value0_2.value1_0.direction0_high_0 + _S55 * _S51.value0_2.value1_0.direction1_high_0 + _S56 * _S51.value0_2.value1_0.direction2_high_0 + _S57 * _S51.value0_2.value1_0.direction3_high_0;
    var _S58 : Tuple_0 = s_apply_dot_0(weights_low_1, _S51.value0_2.value0_1.low_0);
    var _S59 : s_bwdCallableCtx_dot_0;
    _S59._S13 = weights_low_1;
    _S59._S14 = _S51.value0_2.value0_1.low_0;
    var _S60 : Tuple_0 = s_apply_dot_0(weights_high_1, _S51.value0_2.value0_1.high_0);
    var _S61 : s_bwdCallableCtx_dot_0;
    _S61._S13 = weights_high_1;
    _S61._S14 = _S51.value0_2.value0_1.high_0;
    var _S62 : s_bwdCallableCtx_clamp_0;
    _S62._S1 = _S58.value0_0 + _S60.value0_0;
    _S62._S2 = 0.25f;
    _S62._S3 = 4.0f;
    var _S63 : Bands_0 = Bands_x24_syn_dzero_0();
    var _S64 : f32 = 0.0f;
    var _S65 : f32 = 0.0f;
    var _S66 : f32 = 0.0f;
    s_bwdProp_clamp_0(_S62, &(_S64), &(_S65), &(_S66), _S53);
    var _S67 : vec4<f32> = vec4<f32>(0.0f);
    var _S68 : vec4<f32> = _S67;
    var _S69 : vec4<f32> = _S67;
    s_bwdProp_dot_0(_S61, &(_S68), &(_S69), _S64);
    var _S70 : vec4<f32> = _S67;
    var _S71 : vec4<f32> = _S67;
    s_bwdProp_dot_0(_S59, &(_S70), &(_S71), _S64);
    var _S72 : vec4<f32> = _S51.value0_2.value1_0.direction3_high_0 * _S68;
    var _S73 : vec4<f32> = _S51.value0_2.value1_0.direction2_high_0 * _S68;
    var _S74 : vec4<f32> = _S51.value0_2.value1_0.direction1_high_0 * _S68;
    var _S75 : vec4<f32> = _S51.value0_2.value1_0.direction0_high_0 * _S68;
    var _S76 : vec4<f32> = _S51.value0_2.value1_0.direction3_low_0 * _S70;
    var _S77 : f32 = _S72[i32(0)] + _S72[i32(1)] + _S72[i32(2)] + _S72[i32(3)] + (_S76[i32(0)] + _S76[i32(1)] + _S76[i32(2)] + _S76[i32(3)]);
    var _S78 : vec4<f32> = _S51.value0_2.value1_0.direction2_low_0 * _S70;
    var _S79 : f32 = _S73[i32(0)] + _S73[i32(1)] + _S73[i32(2)] + _S73[i32(3)] + (_S78[i32(0)] + _S78[i32(1)] + _S78[i32(2)] + _S78[i32(3)]);
    var _S80 : vec4<f32> = _S51.value0_2.value1_0.direction1_low_0 * _S70;
    var _S81 : f32 = _S74[i32(0)] + _S74[i32(1)] + _S74[i32(2)] + _S74[i32(3)] + (_S80[i32(0)] + _S80[i32(1)] + _S80[i32(2)] + _S80[i32(3)]);
    var _S82 : vec4<f32> = _S51.value0_2.value1_0.direction0_low_0 * _S70;
    var _S83 : f32 = _S75[i32(0)] + _S75[i32(1)] + _S75[i32(2)] + _S75[i32(3)] + (_S82[i32(0)] + _S82[i32(1)] + _S82[i32(2)] + _S82[i32(3)]);
    var _S84 : Bands_0 = _S63;
    _S84.t3_0 = _S77;
    _S84.t2_0 = _S79;
    _S84.t1_0 = _S81;
    _S84.t0_0 = _S83;
    (*_S52) = Bands_x24_syn_dadd_0(_S63, _S84);
    return;
}

struct s_paramCtx_s_bwdCallableCtx_toneequal_0
{
     value0_3 : vec3<f32>,
     value1_1 : Gaussians_0,
     value2_1 : ToneEqualParams_0,
     value3_0 : Bands_0,
};

struct s_bwdCallableCtx_toneequal_0
{
     value0_4 : s_paramCtx_s_bwdCallableCtx_toneequal_0,
};

fn s_bwdProp_toneequal_0( _S85 : s_bwdCallableCtx_toneequal_0,  _S86 : ptr<function, Bands_0>,  _S87 : vec3<f32>)
{
    var _S88 : s_paramCtx_s_bwdCallableCtx_correction_0;
    _S88.value0_1 = _S85.value0_4.value1_1;
    _S88.value1_0 = _S85.value0_4.value2_1;
    _S88.value2_0 = _S85.value0_4.value3_0;
    var _S89 : s_bwdCallableCtx_correction_0 = s_bwdCallableCtx_correction_0( _S88 );
    var _S90 : Bands_0 = Bands_x24_syn_dzero_0();
    var _S91 : vec3<f32> = _S85.value0_4.value0_3 * _S87;
    var _S92 : f32 = _S91[i32(0)] + _S91[i32(1)] + _S91[i32(2)];
    var _S93 : Bands_0;
    _S93.t0_0 = 0.0f;
    _S93.t1_0 = 0.0f;
    _S93.t2_0 = 0.0f;
    _S93.t3_0 = 0.0f;
    s_bwdProp_correction_0(_S89, &(_S93), _S92);
    (*_S86) = Bands_x24_syn_dadd_0(_S93, _S90);
    return;
}

fn gaussians_0( _S94 : f32) -> Gaussians_0
{
    var _S95 : vec4<f32> = vec4<f32>(_S94);
    var d_low_0 : vec4<f32> = _S95 - vec4<f32>(-8.0f, -6.8571429252624512f, -5.7142858505249023f, -4.5714287757873535f);
    var d_high_0 : vec4<f32> = _S95 - vec4<f32>(-3.4285714626312256f, -2.2857143878936768f, -1.1428571939468384f, 0.0f);
    var g_0 : Gaussians_0;
    var _S96 : vec4<f32> = vec4<f32>(params_0.tone_equal_0.inverse_denominator_0);
    g_0.low_0 = exp((vec4<f32>(0) - d_low_0 * d_low_0) * _S96);
    g_0.high_0 = exp((vec4<f32>(0) - d_high_0 * d_high_0) * _S96);
    return g_0;
}

fn correction_0( _S97 : Gaussians_0,  _S98 : Bands_0) -> f32
{
    var _S99 : vec4<f32> = vec4<f32>(_S98.t0_0);
    var _S100 : vec4<f32> = vec4<f32>(_S98.t1_0);
    var _S101 : vec4<f32> = vec4<f32>(_S98.t2_0);
    var _S102 : vec4<f32> = vec4<f32>(_S98.t3_0);
    return clamp(dot(params_0.tone_equal_0.weights_low_0 + _S99 * params_0.tone_equal_0.direction0_low_0 + _S100 * params_0.tone_equal_0.direction1_low_0 + _S101 * params_0.tone_equal_0.direction2_low_0 + _S102 * params_0.tone_equal_0.direction3_low_0, _S97.low_0) + dot(params_0.tone_equal_0.weights_high_0 + _S99 * params_0.tone_equal_0.direction0_high_0 + _S100 * params_0.tone_equal_0.direction1_high_0 + _S101 * params_0.tone_equal_0.direction2_high_0 + _S102 * params_0.tone_equal_0.direction3_high_0, _S97.high_0), 0.25f, 4.0f);
}

fn toneequal_0( _S103 : vec3<f32>,  _S104 : Gaussians_0,  _S105 : Bands_0) -> vec3<f32>
{
    return _S103 * vec3<f32>(correction_0(_S104, _S105));
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_1 : vec3<u32>)
{
    var _S106 : u32 = id_1.x;
    var _S107 : bool;
    if((_S106 >= (params_0.pass_0.width_0)))
    {
        _S107 = true;
    }
    else
    {
        _S107 = ((id_1.y) >= (params_0.pass_0.height_0));
    }
    if(_S107)
    {
        return;
    }
    var i_0 : u32 = id_1.y * params_0.pass_0.width_0 + _S106;
    var _S108 : MaskParams_std140_0 = params_0.mask_0;
    var _S109 : Bands_0 = Bands_x24init_0(0.0f, 0.0f, 0.0f, 0.0f);
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
        var _S110 : f32 = mask_exposure_0(rgb_4, place_0(id_1), &(_S108));
        var _S111 : vec3<f32> = toneequal_0(rgb_4, gaussians_0(_S110), _S109);
        var _S112 : u32 = k_0 + u32(1);
        rgb_4 = _S111;
        k_0 = _S112;
    }
    output_0[i_0] = vec4<f32>(rgb_4, pixel_0.w);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn eigf_mask(@builtin(global_invocation_id) id_2 : vec3<u32>)
{
    var _S113 : u32 = id_2.x;
    var _S114 : bool;
    if((_S113 >= (params_0.pass_0.width_0)))
    {
        _S114 = true;
    }
    else
    {
        _S114 = ((id_2.y) >= (params_0.pass_0.height_0));
    }
    if(_S114)
    {
        return;
    }
    var _S115 : MaskParams_std140_0 = params_0.mask_0;
    var at_5 : vec2<u32> = place_0(id_2);
    var _S116 : f32 = compensated_0(input_0[id_2.y * params_0.pass_0.width_0 + _S113].xyz, &(_S115));
    mask_1[at_5.y * _S115.width_1 + at_5.x] = _S116;
    return;
}

fn interpolate_0( t_3 : Taps_0,  nw_2 : f32,  ne_2 : f32,  sw_2 : f32,  se_2 : f32) -> f32
{
    return t_3.dy_prev_0 * (sw_2 * t_3.dx_next_0 + se_2 * t_3.dx_prev_0) + t_3.dy_next_0 * (nw_2 * t_3.dx_next_0 + ne_2 * t_3.dx_prev_0);
}

fn guide_of_0( value_1 : f32,  m_6 : ptr<function, MaskParams_std140_0>) -> f32
{
    var _S117 : f32;
    if(((*m_6).quantised_0) != u32(0))
    {
        _S117 = quantize_0(value_1, (*m_6).quantization_0);
    }
    else
    {
        _S117 = value_1;
    }
    return _S117;
}

fn grid_sample_0( t_4 : Taps_0,  nw_3 : f32,  ne_3 : f32,  sw_3 : f32,  se_3 : f32,  m_7 : ptr<function, MaskParams_std140_0>) -> vec2<f32>
{
    var value_2 : f32 = interpolate_0(t_4, nw_3, ne_3, sw_3, se_3);
    var _S118 : f32 = guide_of_0(nw_3, &((*m_7)));
    var _S119 : f32 = guide_of_0(ne_3, &((*m_7)));
    var _S120 : f32 = guide_of_0(sw_3, &((*m_7)));
    var _S121 : f32 = guide_of_0(se_3, &((*m_7)));
    return vec2<f32>(value_2, interpolate_0(t_4, _S118, _S119, _S120, _S121));
}

@compute
@workgroup_size(16, 16, 1)
fn eigf_downsample(@builtin(global_invocation_id) id_3 : vec3<u32>)
{
    var _S122 : MaskParams_std140_0 = params_0.mask_0;
    var _S123 : u32 = id_3.x;
    var _S124 : u32 = _S122.grid_width_0;
    var _S125 : bool;
    if((_S123 >= (_S122.grid_width_0)))
    {
        _S125 = true;
    }
    else
    {
        _S125 = ((id_3.y) >= (_S122.grid_height_0));
    }
    if(_S125)
    {
        return;
    }
    var _S126 : vec2<u32> = vec2<u32>(_S122.width_1, _S122.height_1);
    var t_5 : Taps_0 = bilinear_0(id_3.xy, _S126, vec2<u32>(_S124, _S122.grid_height_0));
    var _S127 : u32 = params_0.pass_0.origin_x_0;
    var _S128 : u32 = params_0.pass_0.origin_y_0;
    var _S129 : u32 = _S122.input_width_0;
    var size_0 : vec2<u32> = vec2<u32>(_S122.input_width_0, _S122.input_height_0);
    var owned_0 : vec2<u32> = select(size_0, size_0 - vec2<u32>(u32(1)), ((vec2<u32>(params_0.pass_0.origin_x_0, params_0.pass_0.origin_y_0) + size_0) < _S126));
    if(((t_5.x_prev_0) < (params_0.pass_0.origin_x_0)))
    {
        _S125 = true;
    }
    else
    {
        _S125 = ((t_5.y_prev_0) < _S128);
    }
    if(_S125)
    {
        _S125 = true;
    }
    else
    {
        _S125 = ((t_5.x_prev_0) >= (_S127 + owned_0.x));
    }
    if(_S125)
    {
        _S125 = true;
    }
    else
    {
        _S125 = ((t_5.y_prev_0) >= (_S128 + owned_0.y));
    }
    if(_S125)
    {
        return;
    }
    var north_0 : u32 = (t_5.y_prev_0 - _S128) * _S129;
    var south_0 : u32 = (t_5.y_next_0 - _S128) * _S129;
    var west_0 : u32 = t_5.x_prev_0 - _S127;
    var east_0 : u32 = t_5.x_next_0 - _S127;
    var _S130 : f32 = compensated_0(input_0[north_0 + west_0].xyz, &(_S122));
    var _S131 : f32 = compensated_0(input_0[north_0 + east_0].xyz, &(_S122));
    var _S132 : f32 = compensated_0(input_0[south_0 + west_0].xyz, &(_S122));
    var _S133 : f32 = compensated_0(input_0[south_0 + east_0].xyz, &(_S122));
    var _S134 : vec2<f32> = grid_sample_0(t_5, _S130, _S131, _S132, _S133, &(_S122));
    samples_0[id_3.y * _S124 + _S123] = _S134;
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn eigf_downsample_mask(@builtin(global_invocation_id) id_4 : vec3<u32>)
{
    var _S135 : MaskParams_std140_0 = params_0.mask_0;
    var _S136 : u32 = id_4.x;
    var _S137 : u32 = _S135.grid_width_0;
    var _S138 : bool;
    if((_S136 >= (_S135.grid_width_0)))
    {
        _S138 = true;
    }
    else
    {
        _S138 = ((id_4.y) >= (_S135.grid_height_0));
    }
    if(_S138)
    {
        return;
    }
    var t_6 : Taps_0 = bilinear_0(id_4.xy, vec2<u32>(_S135.width_1, _S135.height_1), vec2<u32>(_S137, _S135.grid_height_0));
    var _S139 : u32 = t_6.y_prev_0 * _S135.width_1;
    var _S140 : u32 = t_6.y_next_0 * _S135.width_1;
    var _S141 : vec2<f32> = grid_sample_0(t_6, mask_1[_S139 + t_6.x_prev_0], mask_1[_S139 + t_6.x_next_0], mask_1[_S140 + t_6.x_prev_0], mask_1[_S140 + t_6.x_next_0], &(_S135));
    samples_0[id_4.y * _S137 + _S136] = _S141;
    return;
}

fn channels_of_0( sample_0 : vec2<f32>) -> vec3<f32>
{
    var _S142 : f32 = sample_0.y;
    var _S143 : f32 = sample_0.x;
    return vec3<f32>(_S142, _S143, _S143 * _S142);
}

var<workgroup> shared_low_0 : array<vec4<f32>, i32(256)>;

var<workgroup> shared_high_0 : array<vec4<f32>, i32(256)>;

@compute
@workgroup_size(16, 16, 1)
fn eigf_range_partial(@builtin(global_invocation_id) id_5 : vec3<u32>, @builtin(workgroup_id) group_0 : vec3<u32>, @builtin(local_invocation_index) t_7 : u32)
{
    var m_8 : MaskParams_std140_0 = params_0.mask_0;
    var _S144 : vec4<f32> = vec4<f32>(3.4028234663852886e+38f);
    var _S145 : vec4<f32> = vec4<f32>(-3.4028234663852886e+38f);
    var _S146 : u32 = id_5.x;
    var _S147 : bool;
    if((_S146 < (params_0.mask_0.grid_width_0)))
    {
        _S147 = ((id_5.y) < (m_8.grid_height_0));
    }
    else
    {
        _S147 = false;
    }
    var low_1 : vec4<f32>;
    var high_1 : vec4<f32>;
    if(_S147)
    {
        var _S148 : vec4<f32> = vec4<f32>(channels_of_0(samples_0[id_5.y * m_8.grid_width_0 + _S146]), 0.0f);
        low_1 = _S148;
        high_1 = _S148;
    }
    else
    {
        low_1 = _S144;
        high_1 = _S145;
    }
    shared_low_0[t_7] = low_1;
    shared_high_0[t_7] = high_1;
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
        if((t_7 < stride_0))
        {
            shared_low_0[t_7] = min(shared_low_0[t_7], shared_low_0[t_7 + stride_0]);
            shared_high_0[t_7] = max(shared_high_0[t_7], shared_high_0[t_7 + stride_0]);
        }
        workgroupBarrier();
        stride_0 = stride_0 / u32(2);
    }
    if(t_7 == u32(0))
    {
        var _S149 : u32 = u32(2) * (group_0.y * m_8.partials_across_0 + group_0.x);
        partials_1[_S149] = shared_low_0[i32(0)];
        partials_1[_S149 + u32(1)] = shared_high_0[i32(0)];
    }
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn eigf_range(@builtin(local_invocation_index) t_8 : u32)
{
    var _S150 : MaskParams_std140_0 = params_0.mask_0;
    var _S151 : vec4<f32> = vec4<f32>(-3.4028234663852886e+38f);
    var low_2 : vec4<f32> = vec4<f32>(3.4028234663852886e+38f);
    var high_2 : vec4<f32> = _S151;
    var k_1 : u32 = t_8;
    loop
    {
        if((k_1 < (_S150.partials_0)))
        {
        }
        else
        {
            break;
        }
        var _S152 : u32 = u32(2) * k_1;
        var _S153 : vec4<f32> = min(low_2, partials_1[_S152]);
        var _S154 : vec4<f32> = max(high_2, partials_1[_S152 + u32(1)]);
        var k_2 : u32 = k_1 + u32(256);
        low_2 = _S153;
        high_2 = _S154;
        k_1 = k_2;
    }
    shared_low_0[t_8] = low_2;
    shared_high_0[t_8] = high_2;
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
        if((t_8 < stride_1))
        {
            shared_low_0[t_8] = min(shared_low_0[t_8], shared_low_0[t_8 + stride_1]);
            shared_high_0[t_8] = max(shared_high_0[t_8], shared_high_0[t_8 + stride_1]);
        }
        workgroupBarrier();
        stride_1 = stride_1 / u32(2);
    }
    if(t_8 == u32(0))
    {
        var lowest_0 : vec3<f32> = shared_low_0[i32(0)].xyz;
        var highest_0 : vec3<f32> = shared_high_0[i32(0)].xyz;
        var _S155 : vec2<f32> = vec2<f32>(1.0e+07f);
        var _S156 : f32 = lowest_0.x;
        var _S157 : vec2<f32> = vec2<f32>(0.0f);
        var _S158 : f32 = highest_0.x;
        var _S159 : vec2<f32> = min(_S155, lowest_0.yz);
        var _S160 : vec2<f32> = max(_S157, highest_0.yz);
        ranges_0[i32(0)] = vec4<f32>(min(_S155, vec2<f32>(_S156, _S156 * _S156)), max(_S157, vec2<f32>(_S158, _S158 * _S158)));
        ranges_0[i32(1)] = vec4<f32>(_S159, _S160);
    }
    return;
}

struct Deriche_0
{
     a0_0 : f32,
     a1_0 : f32,
     a2_0 : f32,
     a3_0 : f32,
     b1_0 : f32,
     b2_0 : f32,
     coefp_0 : f32,
     coefn_0 : f32,
};

fn deriche_0( m_9 : ptr<function, MaskParams_std140_0>) -> Deriche_0
{
    var d_0 : Deriche_0;
    d_0.a0_0 = (*m_9).a_0.x;
    d_0.a1_0 = (*m_9).a_0.y;
    d_0.a2_0 = (*m_9).a_0.z;
    d_0.a3_0 = (*m_9).a_0.w;
    d_0.b1_0 = (*m_9).b_0.x;
    d_0.b2_0 = (*m_9).b_0.y;
    d_0.coefp_0 = (*m_9).b_0.z;
    d_0.coefn_0 = (*m_9).b_0.w;
    return d_0;
}

fn clampf_0( a_4 : f32,  low_3 : f32,  high_3 : f32) -> f32
{
    var _S161 : f32;
    if((a_4 < low_3))
    {
        _S161 = low_3;
    }
    else
    {
        _S161 = min(a_4, high_3);
    }
    return _S161;
}

fn clampf2_0( a_5 : vec2<f32>,  low_4 : vec2<f32>,  high_4 : vec2<f32>) -> vec2<f32>
{
    return vec2<f32>(clampf_0(a_5.x, low_4.x, high_4.x), clampf_0(a_5.y, low_4.y, high_4.y));
}

fn guide_channels_0( sample_1 : vec2<f32>,  range_0 : vec4<f32>) -> vec2<f32>
{
    var g_1 : f32 = sample_1.y;
    return clampf2_0(vec2<f32>(g_1, g_1 * g_1), range_0.xy, range_0.zw);
}

fn mask_channels_0( sample_2 : vec2<f32>,  range_1 : vec4<f32>) -> vec2<f32>
{
    var _S162 : f32 = sample_2.x;
    return clampf2_0(vec2<f32>(_S162, _S162 * sample_2.y), range_1.xy, range_1.zw);
}

fn causal_0( d_1 : Deriche_0,  xc_0 : vec2<f32>,  xp_0 : vec2<f32>,  yp_0 : vec2<f32>,  yb_0 : vec2<f32>) -> vec2<f32>
{
    return vec2<f32>(d_1.a0_0) * xc_0 + vec2<f32>(d_1.a1_0) * xp_0 - vec2<f32>(d_1.b1_0) * yp_0 - vec2<f32>(d_1.b2_0) * yb_0;
}

fn anticausal_0( d_2 : Deriche_0,  xn_0 : vec2<f32>,  xa_0 : vec2<f32>,  yn_0 : vec2<f32>,  ya_0 : vec2<f32>) -> vec2<f32>
{
    return vec2<f32>(d_2.a2_0) * xn_0 + vec2<f32>(d_2.a3_0) * xa_0 - vec2<f32>(d_2.b1_0) * yn_0 - vec2<f32>(d_2.b2_0) * ya_0;
}

@compute
@workgroup_size(16, 16, 1)
fn eigf_columns(@builtin(global_invocation_id) id_6 : vec3<u32>)
{
    var _S163 : MaskParams_std140_0 = params_0.mask_0;
    var column_0 : u32 = id_6.y * u32(16) + id_6.x;
    var _S164 : u32 = _S163.grid_width_0;
    var _S165 : u32 = _S163.grid_height_0;
    if((column_0 >= (_S163.grid_width_0)))
    {
        return;
    }
    var _S166 : Deriche_0 = deriche_0(&(_S163));
    var range_guide_0 : vec4<f32> = ranges_0[i32(0)];
    var range_mask_0 : vec4<f32> = ranges_0[i32(1)];
    var _S167 : bool = (_S163.quantised_0) != u32(0);
    var xp_1 : vec2<f32> = guide_channels_0(samples_0[column_0], ranges_0[i32(0)]);
    var _S168 : vec2<f32> = vec2<f32>(_S166.coefp_0);
    var yb_1 : vec2<f32> = xp_1 * _S168;
    var xp_m_0 : vec2<f32> = mask_channels_0(samples_0[column_0], ranges_0[i32(1)]);
    var yb_m_0 : vec2<f32> = xp_m_0 * _S168;
    var xp_2 : vec2<f32> = xp_1;
    var yp_1 : vec2<f32> = yb_1;
    var yb_2 : vec2<f32> = yb_1;
    var xp_m_1 : vec2<f32> = xp_m_0;
    var yp_m_0 : vec2<f32> = yb_m_0;
    var yb_m_1 : vec2<f32> = yb_m_0;
    var row_0 : u32 = u32(0);
    loop
    {
        if((row_0 < _S165))
        {
        }
        else
        {
            break;
        }
        var at_6 : u32 = row_0 * _S164 + column_0;
        var sample_3 : vec2<f32> = samples_0[at_6];
        var xc_1 : vec2<f32> = guide_channels_0(samples_0[at_6], range_guide_0);
        var yc_0 : vec2<f32> = causal_0(_S166, xc_1, xp_2, yp_1, yb_2);
        columns_guide_0[at_6] = yc_0;
        if(_S167)
        {
            var xc_m_0 : vec2<f32> = mask_channels_0(sample_3, range_mask_0);
            var yc_m_0 : vec2<f32> = causal_0(_S166, xc_m_0, xp_m_1, yp_m_0, yb_m_1);
            columns_mask_0[at_6] = yc_m_0;
            var _S169 : vec2<f32> = yp_m_0;
            xp_m_1 = xc_m_0;
            yp_m_0 = yc_m_0;
            yb_m_1 = _S169;
        }
        var _S170 : u32 = row_0 + u32(1);
        xp_2 = xc_1;
        yb_2 = yp_1;
        yp_1 = yc_0;
        row_0 = _S170;
    }
    var xn_1 : vec2<f32> = guide_channels_0(samples_0[(_S165 - u32(1)) * _S164 + column_0], range_guide_0);
    var _S171 : vec2<f32> = vec2<f32>(_S166.coefn_0);
    var yn_1 : vec2<f32> = xn_1 * _S171;
    var xn_m_0 : vec2<f32> = mask_channels_0(samples_0[(_S165 - u32(1)) * _S164 + column_0], range_mask_0);
    var yn_m_0 : vec2<f32> = xn_m_0 * _S171;
    var xn_2 : vec2<f32> = xn_1;
    var xa_1 : vec2<f32> = xn_1;
    var yn_2 : vec2<f32> = yn_1;
    var ya_1 : vec2<f32> = yn_1;
    var xn_m_1 : vec2<f32> = xn_m_0;
    var xa_m_0 : vec2<f32> = xn_m_0;
    var yn_m_1 : vec2<f32> = yn_m_0;
    var ya_m_0 : vec2<f32> = yn_m_0;
    row_0 = _S165;
    loop
    {
        if((row_0 > u32(0)))
        {
        }
        else
        {
            break;
        }
        var _S172 : u32 = row_0 - u32(1);
        var at_7 : u32 = _S172 * _S164 + column_0;
        var sample_4 : vec2<f32> = samples_0[at_7];
        var xc_2 : vec2<f32> = guide_channels_0(samples_0[at_7], range_guide_0);
        var yc_1 : vec2<f32> = anticausal_0(_S166, xn_2, xa_1, yn_2, ya_1);
        columns_guide_0[at_7] = columns_guide_0[at_7] + yc_1;
        if(_S167)
        {
            var xc_m_1 : vec2<f32> = mask_channels_0(sample_4, range_mask_0);
            var yc_m_1 : vec2<f32> = anticausal_0(_S166, xn_m_1, xa_m_0, yn_m_1, ya_m_0);
            columns_mask_0[at_7] = columns_mask_0[at_7] + yc_m_1;
            var _S173 : vec2<f32> = xn_m_1;
            var _S174 : vec2<f32> = yn_m_1;
            xn_m_1 = xc_m_1;
            xa_m_0 = _S173;
            yn_m_1 = yc_m_1;
            ya_m_0 = _S174;
        }
        xa_1 = xn_2;
        xn_2 = xc_2;
        ya_1 = yn_2;
        yn_2 = yc_1;
        row_0 = _S172;
    }
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn eigf_rows(@builtin(global_invocation_id) id_7 : vec3<u32>)
{
    var _S175 : MaskParams_std140_0 = params_0.mask_0;
    var row_1 : u32 = id_7.y * u32(16) + id_7.x;
    var _S176 : u32 = _S175.grid_width_0;
    if((row_1 >= (_S175.grid_height_0)))
    {
        return;
    }
    var _S177 : Deriche_0 = deriche_0(&(_S175));
    var range_mask_1 : vec4<f32> = ranges_0[i32(1)];
    var quantised_1 : bool = (_S175.quantised_0) != u32(0);
    var start_0 : u32 = row_1 * _S176;
    var _S178 : vec2<f32> = ranges_0[i32(0)].xy;
    var _S179 : vec2<f32> = ranges_0[i32(0)].zw;
    var xp_3 : vec2<f32> = clampf2_0(columns_guide_0[start_0], _S178, _S179);
    var _S180 : vec2<f32> = vec2<f32>(_S177.coefp_0);
    var yb_3 : vec2<f32> = xp_3 * _S180;
    var _S181 : vec2<f32> = vec2<f32>(0.0f);
    var xp_m_2 : vec2<f32>;
    if(quantised_1)
    {
        xp_m_2 = clampf2_0(columns_mask_0[start_0], range_mask_1.xy, range_mask_1.zw);
    }
    else
    {
        xp_m_2 = _S181;
    }
    var yb_m_2 : vec2<f32> = xp_m_2 * _S180;
    var xp_4 : vec2<f32> = xp_3;
    var yp_2 : vec2<f32> = yb_3;
    var yb_4 : vec2<f32> = yb_3;
    var yp_m_1 : vec2<f32> = yb_m_2;
    var yb_m_3 : vec2<f32> = yb_m_2;
    var x_2 : u32 = u32(0);
    loop
    {
        if((x_2 < _S176))
        {
        }
        else
        {
            break;
        }
        var _S182 : u32 = start_0 + x_2;
        var xc_3 : vec2<f32> = clampf2_0(columns_guide_0[_S182], _S178, _S179);
        var yc_2 : vec2<f32> = causal_0(_S177, xc_3, xp_4, yp_2, yb_4);
        statistics_guide_0[_S182] = yc_2;
        if(quantised_1)
        {
            var xc_m_2 : vec2<f32> = clampf2_0(columns_mask_0[_S182], range_mask_1.xy, range_mask_1.zw);
            var yc_m_2 : vec2<f32> = causal_0(_S177, xc_m_2, xp_m_2, yp_m_1, yb_m_3);
            statistics_mask_0[_S182] = yc_m_2;
            var _S183 : vec2<f32> = yp_m_1;
            xp_m_2 = xc_m_2;
            yp_m_1 = yc_m_2;
            yb_m_3 = _S183;
        }
        var _S184 : u32 = x_2 + u32(1);
        xp_4 = xc_3;
        yb_4 = yp_2;
        yp_2 = yc_2;
        x_2 = _S184;
    }
    var _S185 : u32 = start_0 + _S176 - u32(1);
    var xn_3 : vec2<f32> = clampf2_0(columns_guide_0[_S185], _S178, _S179);
    var _S186 : vec2<f32> = vec2<f32>(_S177.coefn_0);
    var yn_3 : vec2<f32> = xn_3 * _S186;
    var xn_m_2 : vec2<f32>;
    if(quantised_1)
    {
        xn_m_2 = clampf2_0(columns_mask_0[_S185], range_mask_1.xy, range_mask_1.zw);
    }
    else
    {
        xn_m_2 = _S181;
    }
    var yn_m_2 : vec2<f32> = xn_m_2 * _S186;
    var xn_4 : vec2<f32> = xn_3;
    var xa_2 : vec2<f32> = xn_3;
    var yn_4 : vec2<f32> = yn_3;
    var ya_2 : vec2<f32> = yn_3;
    var xa_m_1 : vec2<f32> = xn_m_2;
    var yn_m_3 : vec2<f32> = yn_m_2;
    var ya_m_1 : vec2<f32> = yn_m_2;
    x_2 = _S176;
    loop
    {
        if((x_2 > u32(0)))
        {
        }
        else
        {
            break;
        }
        var at_8 : u32 = start_0 + x_2 - u32(1);
        var xc_4 : vec2<f32> = clampf2_0(columns_guide_0[at_8], _S178, _S179);
        var yc_3 : vec2<f32> = anticausal_0(_S177, xn_4, xa_2, yn_4, ya_2);
        var guide_1 : vec2<f32> = statistics_guide_0[at_8] + yc_3;
        var _S187 : f32 = guide_1.x;
        statistics_guide_0[at_8] = vec2<f32>(_S187, guide_1.y - _S187 * _S187);
        if(quantised_1)
        {
            var xc_m_3 : vec2<f32> = clampf2_0(columns_mask_0[at_8], range_mask_1.xy, range_mask_1.zw);
            var yc_m_3 : vec2<f32> = anticausal_0(_S177, xn_m_2, xa_m_1, yn_m_3, ya_m_1);
            var masked_1 : vec2<f32> = statistics_mask_0[at_8] + yc_m_3;
            var _S188 : f32 = masked_1.x;
            statistics_mask_0[at_8] = vec2<f32>(_S188, masked_1.y - _S187 * _S188);
            var _S189 : vec2<f32> = xn_m_2;
            var _S190 : vec2<f32> = yn_m_3;
            xn_m_2 = xc_m_3;
            xa_m_1 = _S189;
            yn_m_3 = yc_m_3;
            ya_m_1 = _S190;
        }
        var _S191 : u32 = x_2 - u32(1);
        xa_2 = xn_4;
        xn_4 = xc_4;
        ya_2 = yn_4;
        yn_4 = yc_3;
        x_2 = _S191;
    }
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn eigf_blend(@builtin(global_invocation_id) id_8 : vec3<u32>)
{
    var _S192 : MaskParams_std140_0 = params_0.mask_0;
    var _S193 : u32 = id_8.x;
    var _S194 : u32 = _S192.width_1;
    var _S195 : bool;
    if((_S193 >= (_S192.width_1)))
    {
        _S195 = true;
    }
    else
    {
        _S195 = ((id_8.y) >= (_S192.height_1));
    }
    if(_S195)
    {
        return;
    }
    var at_9 : u32 = id_8.y * _S194 + _S193;
    var _S196 : f32 = eigf_blend_at_0(mask_1[at_9], id_8.xy, &(_S192), false);
    mask_1[at_9] = _S196;
    return;
}

struct DiffPair_Bands_0
{
     primal_0 : Bands_0,
     differential_0 : Bands_0,
};

fn _S197( _S198 : vec3<f32>,  _S199 : Gaussians_0,  _S200 : ptr<function, DiffPair_Bands_0>,  _S201 : vec3<f32>)
{
    var _S202 : s_paramCtx_s_bwdCallableCtx_toneequal_0;
    _S202.value0_3 = _S198;
    _S202.value1_1 = _S199;
    _S202.value2_1.weights_low_0 = params_0.tone_equal_0.weights_low_0;
    _S202.value2_1.weights_high_0 = params_0.tone_equal_0.weights_high_0;
    _S202.value2_1.direction0_low_0 = params_0.tone_equal_0.direction0_low_0;
    _S202.value2_1.direction0_high_0 = params_0.tone_equal_0.direction0_high_0;
    _S202.value2_1.direction1_low_0 = params_0.tone_equal_0.direction1_low_0;
    _S202.value2_1.direction1_high_0 = params_0.tone_equal_0.direction1_high_0;
    _S202.value2_1.direction2_low_0 = params_0.tone_equal_0.direction2_low_0;
    _S202.value2_1.direction2_high_0 = params_0.tone_equal_0.direction2_high_0;
    _S202.value2_1.direction3_low_0 = params_0.tone_equal_0.direction3_low_0;
    _S202.value2_1.direction3_high_0 = params_0.tone_equal_0.direction3_high_0;
    _S202.value2_1.inverse_denominator_0 = params_0.tone_equal_0.inverse_denominator_0;
    _S202.value2_1.pad0_0 = params_0.tone_equal_0.pad0_0;
    _S202.value2_1.pad1_0 = params_0.tone_equal_0.pad1_0;
    _S202.value2_1.pad2_0 = params_0.tone_equal_0.pad2_0;
    _S202.value3_0 = (*_S200).primal_0;
    var _S203 : s_bwdCallableCtx_toneequal_0 = s_bwdCallableCtx_toneequal_0( _S202 );
    var _S204 : Bands_0 = (*_S200).differential_0;
    s_bwdProp_toneequal_0(_S203, &(_S204), _S201);
    (*_S200).differential_0 = _S204;
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn toneequal_backward(@builtin(global_invocation_id) id_9 : vec3<u32>)
{
    var _S205 : u32 = id_9.x;
    var _S206 : bool;
    if((_S205 >= (params_0.pass_0.width_0)))
    {
        _S206 = true;
    }
    else
    {
        _S206 = ((id_9.y) >= (params_0.pass_0.height_0));
    }
    if(_S206)
    {
        return;
    }
    var i_1 : u32 = id_9.y * params_0.pass_0.width_0 + _S205;
    var rgb_5 : vec3<f32> = input_0[i_1].xyz;
    var _S207 : Bands_0 = Bands_x24_syn_dzero_0();
    var t_9 : DiffPair_Bands_0;
    t_9.primal_0 = Bands_x24init_0(0.0f, 0.0f, 0.0f, 0.0f);
    t_9.differential_0 = _S207;
    var _S208 : vec2<u32> = place_0(id_9);
    var _S209 : MaskParams_std140_0 = params_0.mask_0;
    var _S210 : f32 = mask_exposure_0(rgb_5, _S208, &(_S209));
    _S197(rgb_5, gaussians_0(_S210), &(t_9), derivative_in_0[i_1].xyz);
    derivative_out_0[i_1] = vec4<f32>(t_9.differential_0.t0_0, t_9.differential_0.t1_0, t_9.differential_0.t2_0, t_9.differential_0.t3_0);
    return;
}

struct DiffPair_vectorx3Cfloatx2C3x3E_0
{
     primal_0 : vec3<f32>,
     differential_0 : vec3<f32>,
};

fn s_fwd_correction_0( _S211 : Gaussians_0,  _S212 : DiffPair_Bands_0) -> DiffPair_float_0
{
    var _S213 : vec4<f32> = vec4<f32>(_S212.primal_0.t0_0);
    var _S214 : vec4<f32> = vec4<f32>(_S212.differential_0.t0_0);
    var _S215 : vec4<f32> = vec4<f32>(_S212.primal_0.t1_0);
    var _S216 : vec4<f32> = vec4<f32>(_S212.differential_0.t1_0);
    var _S217 : vec4<f32> = vec4<f32>(_S212.primal_0.t2_0);
    var _S218 : vec4<f32> = vec4<f32>(_S212.differential_0.t2_0);
    var _S219 : vec4<f32> = vec4<f32>(_S212.primal_0.t3_0);
    var _S220 : vec4<f32> = vec4<f32>(_S212.differential_0.t3_0);
    var _S221 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( params_0.tone_equal_0.weights_low_0 + _S213 * params_0.tone_equal_0.direction0_low_0 + _S215 * params_0.tone_equal_0.direction1_low_0 + _S217 * params_0.tone_equal_0.direction2_low_0 + _S219 * params_0.tone_equal_0.direction3_low_0, _S214 * params_0.tone_equal_0.direction0_low_0 + _S216 * params_0.tone_equal_0.direction1_low_0 + _S218 * params_0.tone_equal_0.direction2_low_0 + _S220 * params_0.tone_equal_0.direction3_low_0 );
    var _S222 : vec4<f32> = vec4<f32>(0.0f);
    var _S223 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( _S211.low_0, _S222 );
    var _S224 : DiffPair_float_0 = _d_dot_1(_S221, _S223);
    var _S225 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( params_0.tone_equal_0.weights_high_0 + _S213 * params_0.tone_equal_0.direction0_high_0 + _S215 * params_0.tone_equal_0.direction1_high_0 + _S217 * params_0.tone_equal_0.direction2_high_0 + _S219 * params_0.tone_equal_0.direction3_high_0, _S214 * params_0.tone_equal_0.direction0_high_0 + _S216 * params_0.tone_equal_0.direction1_high_0 + _S218 * params_0.tone_equal_0.direction2_high_0 + _S220 * params_0.tone_equal_0.direction3_high_0 );
    var _S226 : DiffPair_vectorx3Cfloatx2C4x3E_0 = DiffPair_vectorx3Cfloatx2C4x3E_0( _S211.high_0, _S222 );
    var _S227 : DiffPair_float_0 = _d_dot_1(_S225, _S226);
    var _S228 : DiffPair_float_0 = DiffPair_float_0( _S224.primal_0 + _S227.primal_0, _S224.differential_0 + _S227.differential_0 );
    var _S229 : DiffPair_float_0 = DiffPair_float_0( 0.25f, 0.0f );
    var _S230 : DiffPair_float_0 = DiffPair_float_0( 4.0f, 0.0f );
    var _S231 : DiffPair_float_0 = _d_clamp_1(_S228, _S229, _S230);
    var _S232 : DiffPair_float_0 = DiffPair_float_0( _S231.primal_0, _S231.differential_0 );
    return _S232;
}

fn s_fwd_toneequal_0( _S233 : vec3<f32>,  _S234 : Gaussians_0,  _S235 : DiffPair_Bands_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S236 : DiffPair_Bands_0 = DiffPair_Bands_0( _S235.primal_0, _S235.differential_0 );
    var _S237 : DiffPair_float_0 = s_fwd_correction_0(_S234, _S236);
    var _S238 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S233 * vec3<f32>(_S237.primal_0), vec3<f32>(_S237.differential_0) * _S233 );
    return _S238;
}

@compute
@workgroup_size(16, 16, 1)
fn toneequal_forward_diff(@builtin(global_invocation_id) id_10 : vec3<u32>)
{
    var _S239 : u32 = id_10.x;
    var _S240 : bool;
    if((_S239 >= (params_0.pass_0.width_0)))
    {
        _S240 = true;
    }
    else
    {
        _S240 = ((id_10.y) >= (params_0.pass_0.height_0));
    }
    if(_S240)
    {
        return;
    }
    var i_2 : u32 = id_10.y * params_0.pass_0.width_0 + _S239;
    var rgb_6 : vec3<f32> = input_0[i_2].xyz;
    var v_0 : vec4<f32> = derivative_in_0[i_2];
    var settings_0 : Bands_0 = Bands_x24init_0(0.0f, 0.0f, 0.0f, 0.0f);
    var direction_0 : Bands_0;
    direction_0.t0_0 = v_0.x;
    direction_0.t1_0 = v_0.y;
    direction_0.t2_0 = v_0.z;
    direction_0.t3_0 = v_0.w;
    var _S241 : vec2<u32> = place_0(id_10);
    var _S242 : MaskParams_std140_0 = params_0.mask_0;
    var _S243 : f32 = mask_exposure_0(rgb_6, _S241, &(_S242));
    var _S244 : DiffPair_Bands_0 = DiffPair_Bands_0( settings_0, direction_0 );
    derivative_out_0[i_2] = vec4<f32>(s_fwd_toneequal_0(rgb_6, gaussians_0(_S243), _S244).differential_0, 0.0f);
    return;
}

