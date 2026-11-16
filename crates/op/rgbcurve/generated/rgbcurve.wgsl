// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2019-2026  darktable developers (ported from src/iop/rgbcurve.c)
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

// Generated from slang/rgbcurve.slang by tools/build-kernels.sh. Do not edit.

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

struct RgbCurveParams_std140_0
{
    @align(16) luminance_0 : vec4<f32>,
    @align(16) count_0 : vec4<u32>,
    @align(16) end_value_0 : vec4<f32>,
    @align(16) end_power_0 : vec4<f32>,
    @align(16) autoscale_0 : u32,
    @align(4) preserve_colors_0 : u32,
    @align(8) pad0_0 : u32,
    @align(4) pad1_0 : u32,
    @align(16) gradient_curve_0 : vec4<u32>,
    @align(16) gradient_node_0 : vec4<u32>,
    @align(16) gradient_kind_0 : vec4<u32>,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) rgb_curve_0 : RgbCurveParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(0) @group(0) var<storage, read> input_0 : array<vec4<f32>>;

@binding(3) @group(0) var<storage, read> knots_0 : array<vec4<f32>>;

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

struct Tuple_1
{
     value0_1 : f32,
};

fn s_apply_clamp_0( x_1 : f32,  minBound_0 : f32,  maxBound_0 : f32) -> Tuple_1
{
    var _S5 : Tuple_1 = Tuple_1( clamp(x_1, minBound_0, maxBound_0) );
    return _S5;
}

struct s_bwdCallableCtx_clamp_0
{
     _S6 : f32,
     _S7 : f32,
     _S8 : f32,
};

fn _d_clamp_0( dpx_1 : ptr<function, DiffPair_float_0>,  dpMin_0 : ptr<function, DiffPair_float_0>,  dpMax_0 : ptr<function, DiffPair_float_0>,  dOut_1 : f32)
{
    var _S9 : DiffPair_float_0 = (*dpx_1);
    var _S10 : bool;
    if((((*dpx_1).primal_0) >= ((*dpMin_0).primal_0)))
    {
        _S10 = (((*dpx_1).primal_0) <= ((*dpMax_0).primal_0));
    }
    else
    {
        _S10 = false;
    }
    var _S11 : f32;
    if(_S10)
    {
        _S11 = dOut_1;
    }
    else
    {
        _S11 = 0.0f;
    }
    (*dpx_1).primal_0 = _S9.primal_0;
    (*dpx_1).differential_0 = _S11;
    var _S12 : DiffPair_float_0 = (*dpMin_0);
    if(((_S9.primal_0) < ((*dpMin_0).primal_0)))
    {
        _S11 = dOut_1;
    }
    else
    {
        _S11 = 0.0f;
    }
    (*dpMin_0).primal_0 = _S12.primal_0;
    (*dpMin_0).differential_0 = _S11;
    var _S13 : DiffPair_float_0 = (*dpMax_0);
    if((((*dpx_1).primal_0) > ((*dpMax_0).primal_0)))
    {
        _S11 = dOut_1;
    }
    else
    {
        _S11 = 0.0f;
    }
    (*dpMax_0).primal_0 = _S13.primal_0;
    (*dpMax_0).differential_0 = _S11;
    return;
}

fn s_bwdProp_clamp_0( ctx_1 : s_bwdCallableCtx_clamp_0,  d_x_1 : ptr<function, f32>,  d_minBound_0 : ptr<function, f32>,  d_maxBound_0 : ptr<function, f32>,  d_out_1 : f32)
{
    var _S14 : DiffPair_float_0;
    _S14.primal_0 = ctx_1._S6;
    var _S15 : DiffPair_float_0;
    _S15.primal_0 = ctx_1._S7;
    var _S16 : DiffPair_float_0;
    _S16.primal_0 = ctx_1._S8;
    _d_clamp_0(&(_S14), &(_S15), &(_S16), d_out_1);
    (*d_x_1) = _S14.differential_0;
    (*d_minBound_0) = _S15.differential_0;
    (*d_maxBound_0) = _S16.differential_0;
    return;
}

struct Tuple_2
{
     value0_2 : f32,
};

fn s_apply_log2_0( x_2 : f32) -> Tuple_2
{
    var _S17 : Tuple_2 = Tuple_2( log2(x_2) );
    return _S17;
}

struct s_bwdCallableCtx_log2_0
{
     _S18 : f32,
};

fn _d_log2_0( dpx_2 : ptr<function, DiffPair_float_0>,  dOut_2 : f32)
{
    var _S19 : f32 = 1.0f / ((*dpx_2).primal_0 * 0.69314718246459961f) * dOut_2;
    (*dpx_2).primal_0 = (*dpx_2).primal_0;
    (*dpx_2).differential_0 = _S19;
    return;
}

fn s_bwdProp_log2_0( ctx_2 : s_bwdCallableCtx_log2_0,  d_x_2 : ptr<function, f32>,  d_out_2 : f32)
{
    var _S20 : DiffPair_float_0;
    _S20.primal_0 = ctx_2._S18;
    _d_log2_0(&(_S20), d_out_2);
    (*d_x_2) = _S20.differential_0;
    return;
}

struct Tuple_3
{
     value0_3 : f32,
};

fn s_apply_max_0( x_3 : f32,  y_0 : f32) -> Tuple_3
{
    var _S21 : Tuple_3 = Tuple_3( max(x_3, y_0) );
    return _S21;
}

struct s_bwdCallableCtx_max_0
{
     _S22 : f32,
     _S23 : f32,
};

fn _d_max_0( dpx_3 : ptr<function, DiffPair_float_0>,  dpy_0 : ptr<function, DiffPair_float_0>,  dOut_3 : f32)
{
    var _S24 : DiffPair_float_0 = (*dpx_3);
    var _S25 : f32;
    if((((*dpx_3).primal_0) > ((*dpy_0).primal_0)))
    {
        _S25 = dOut_3;
    }
    else
    {
        if((((*dpx_3).primal_0) < ((*dpy_0).primal_0)))
        {
            _S25 = 0.0f;
        }
        else
        {
            _S25 = 0.5f * dOut_3;
        }
    }
    (*dpx_3).primal_0 = _S24.primal_0;
    (*dpx_3).differential_0 = _S25;
    var _S26 : DiffPair_float_0 = (*dpy_0);
    if((((*dpy_0).primal_0) > (_S24.primal_0)))
    {
        _S25 = dOut_3;
    }
    else
    {
        if((((*dpy_0).primal_0) < ((*dpx_3).primal_0)))
        {
            _S25 = 0.0f;
        }
        else
        {
            _S25 = 0.5f * dOut_3;
        }
    }
    (*dpy_0).primal_0 = _S26.primal_0;
    (*dpy_0).differential_0 = _S25;
    return;
}

fn s_bwdProp_max_0( ctx_3 : s_bwdCallableCtx_max_0,  d_x_3 : ptr<function, f32>,  d_y_0 : ptr<function, f32>,  d_out_3 : f32)
{
    var _S27 : DiffPair_float_0;
    _S27.primal_0 = ctx_3._S22;
    var _S28 : DiffPair_float_0;
    _S28.primal_0 = ctx_3._S23;
    _d_max_0(&(_S27), &(_S28), d_out_3);
    (*d_x_3) = _S27.differential_0;
    (*d_y_0) = _S28.differential_0;
    return;
}

struct Point_0
{
     curve_0 : u32,
     segment_0 : u32,
     above_0 : u32,
     v_0 : f32,
};

struct Lookup_0
{
     mode_0 : u32,
     r_0 : Point_0,
     g_0 : Point_0,
     b_0 : Point_0,
};

struct Gradient_0
{
     s0_0 : f32,
     s1_0 : f32,
     s2_0 : f32,
     s3_0 : f32,
};

struct DiffPair_Gradient_0
{
     primal_0 : Gradient_0,
     differential_0 : Gradient_0,
};

struct s_minCtx_s_bwdCallableCtx_curve_at_0
{
     value0_4 : vec3<f32>,
     value2_0 : vec3<f32>,
     value4_0 : vec3<f32>,
};

struct s_minCtx_s_bwdCallableCtx_rgb_curve_0
{
     value1_0 : s_minCtx_s_bwdCallableCtx_curve_at_0,
     value2_1 : s_minCtx_s_bwdCallableCtx_curve_at_0,
     value3_0 : s_minCtx_s_bwdCallableCtx_curve_at_0,
     value4_1 : s_minCtx_s_bwdCallableCtx_curve_at_0,
};

struct Tuple_4
{
     value0_5 : vec3<f32>,
     value1_1 : s_minCtx_s_bwdCallableCtx_rgb_curve_0,
};

struct Tuple_5
{
     value0_6 : f32,
     value1_2 : s_minCtx_s_bwdCallableCtx_curve_at_0,
};

struct Tuple_6
{
     value0_7 : vec3<f32>,
};

struct Tuple_7
{
     value0_8 : f32,
};

fn s_apply_slotted_0( _S29 : f32,  curve_1 : u32,  node_0 : u32,  kind_0 : u32,  p_0 : ptr<function, RgbCurveParams_std140_0>,  slots_0 : bool,  _S30 : Gradient_0) -> Tuple_7
{
    var _S31 : f32;
    if(!!slots_0)
    {
        var _S32 : vec4<u32> = (*p_0).gradient_curve_0;
        var _S33 : bool;
        if(((*p_0).gradient_curve_0.x) == curve_1)
        {
            _S33 = ((*p_0).gradient_node_0.x) == node_0;
        }
        else
        {
            _S33 = false;
        }
        if(_S33)
        {
            _S33 = ((*p_0).gradient_kind_0.x) == kind_0;
        }
        else
        {
            _S33 = false;
        }
        if(_S33)
        {
            _S31 = _S30.s0_0;
        }
        else
        {
            _S31 = _S29;
        }
        if((_S32.y) == curve_1)
        {
            _S33 = ((*p_0).gradient_node_0.y) == node_0;
        }
        else
        {
            _S33 = false;
        }
        if(_S33)
        {
            _S33 = ((*p_0).gradient_kind_0.y) == kind_0;
        }
        else
        {
            _S33 = false;
        }
        if(_S33)
        {
            _S31 = _S30.s1_0;
        }
        if((_S32.z) == curve_1)
        {
            _S33 = ((*p_0).gradient_node_0.z) == node_0;
        }
        else
        {
            _S33 = false;
        }
        if(_S33)
        {
            _S33 = ((*p_0).gradient_kind_0.z) == kind_0;
        }
        else
        {
            _S33 = false;
        }
        if(_S33)
        {
            _S31 = _S30.s2_0;
        }
        if((_S32.w) == curve_1)
        {
            _S33 = ((*p_0).gradient_node_0.w) == node_0;
        }
        else
        {
            _S33 = false;
        }
        if(_S33)
        {
            _S33 = ((*p_0).gradient_kind_0.w) == kind_0;
        }
        else
        {
            _S33 = false;
        }
        if(_S33)
        {
            _S31 = _S30.s3_0;
        }
    }
    else
    {
        _S31 = _S29;
    }
    var _S34 : Tuple_7;
    _S34.value0_8 = _S31;
    return _S34;
}

fn s_apply_knot_0( curve_2 : u32,  k_0 : u32,  p_1 : ptr<function, RgbCurveParams_std140_0>,  slots_1 : bool,  _S35 : Gradient_0) -> Tuple_6
{
    var t_0 : vec4<f32> = knots_0[curve_2 * u32(20) + k_0];
    var _S36 : Tuple_6;
    _S36.value0_7 = vec3<f32>(s_apply_slotted_0(t_0.x, curve_2, k_0, u32(0), &((*p_1)), slots_1, _S35).value0_8, s_apply_slotted_0(t_0.y, curve_2, k_0, u32(1), &((*p_1)), slots_1, _S35).value0_8, s_apply_slotted_0(t_0.z, curve_2, k_0, u32(2), &((*p_1)), slots_1, _S35).value0_8);
    return _S36;
}

fn s_apply_curve_at_0( point_0 : Point_0,  p_2 : ptr<function, RgbCurveParams_std140_0>,  slots_2 : bool,  _S37 : Gradient_0) -> Tuple_5
{
    var _S38 : vec3<f32> = vec3<f32>(0.0f);
    var _S39 : s_minCtx_s_bwdCallableCtx_curve_at_0;
    _S39.value0_4 = _S38;
    _S39.value2_0 = _S38;
    _S39.value4_0 = _S38;
    var _S40 : bool = (point_0.above_0) != u32(0);
    var _S41 : f32;
    if(_S40)
    {
        var _S42 : Tuple_6 = s_apply_knot_0(point_0.curve_0, (*p_2).count_0[point_0.curve_0] - u32(1), &((*p_2)), slots_2, _S37);
        _S39.value0_4 = _S42.value0_7;
        var x_last_0 : f32 = _S42.value0_7.x;
        var _S43 : Tuple_3 = s_apply_max_0(s_apply_log2_0(s_apply_max_0(point_0.v_0, x_last_0).value0_3).value0_2 - s_apply_log2_0(x_last_0).value0_2, 0.0f);
        var limit_0 : f32 = 64.0f / s_apply_max_0(_S43.value0_3, 9.99999968265522539e-21f).value0_3;
        _S41 = s_apply_slotted_0((*p_2).end_value_0[point_0.curve_0], point_0.curve_0, u32(0), u32(3), &((*p_2)), slots_2, _S37).value0_8 * s_apply_exp2_0(s_apply_clamp_0(s_apply_slotted_0((*p_2).end_power_0[point_0.curve_0], point_0.curve_0, u32(0), u32(4), &((*p_2)), slots_2, _S37).value0_8, - limit_0, limit_0).value0_1 * _S43.value0_3).value0_0;
    }
    var _S44 : bool = !_S40;
    if(_S44)
    {
        var _S45 : Tuple_6 = s_apply_knot_0(point_0.curve_0, point_0.segment_0, &((*p_2)), slots_2, _S37);
        _S39.value2_0 = _S45.value0_7;
        var _S46 : Tuple_6 = s_apply_knot_0(point_0.curve_0, point_0.segment_0 + u32(1), &((*p_2)), slots_2, _S37);
        _S39.value4_0 = _S46.value0_7;
        var _S47 : f32 = _S45.value0_7.x;
        var h_0 : f32 = _S46.value0_7.x - _S47;
        var t_1 : f32 = (s_apply_max_0(point_0.v_0, _S47).value0_3 - _S47) / h_0;
        var t2_0 : f32 = t_1 * t_1;
        var t3_0 : f32 = t_1 * t2_0;
        var _S48 : f32 = 3.0f * t2_0;
        _S41 = s_apply_clamp_0((2.0f * t3_0 - _S48 + 1.0f) * _S45.value0_7.y + (t3_0 - 2.0f * t2_0 + t_1) * h_0 * _S45.value0_7.z + (-2.0f * t3_0 + _S48) * _S46.value0_7.y + (t3_0 - t2_0) * h_0 * _S46.value0_7.z, 0.0f, 1.0f).value0_1;
    }
    var _S49 : Tuple_5;
    _S49.value0_6 = _S41;
    _S49.value1_2 = _S39;
    return _S49;
}

fn s_apply_rgb_curve_0( rgb_0 : vec3<f32>,  l_0 : Lookup_0,  p_3 : ptr<function, RgbCurveParams_std140_0>,  slots_3 : bool,  _S50 : Gradient_0) -> Tuple_4
{
    var _S51 : vec3<f32> = vec3<f32>(0.0f);
    var _S52 : s_minCtx_s_bwdCallableCtx_curve_at_0 = s_minCtx_s_bwdCallableCtx_curve_at_0( _S51, _S51, _S51 );
    var _S53 : s_minCtx_s_bwdCallableCtx_rgb_curve_0;
    _S53.value1_0 = _S52;
    _S53.value2_1 = _S52;
    _S53.value3_0 = _S52;
    _S53.value4_1 = _S52;
    var _S54 : bool = !((l_0.mode_0) == u32(2));
    var _S55 : vec3<f32>;
    if(_S54)
    {
        var _runFlag_0 : bool;
        if((l_0.mode_0) == u32(1))
        {
            var _S56 : Tuple_5 = s_apply_curve_at_0(l_0.r_0, &((*p_3)), slots_3, _S50);
            _S53.value1_0 = _S56.value1_2;
            var _S57 : vec3<f32> = rgb_0 * vec3<f32>((_S56.value0_6 / l_0.r_0.v_0));
            _runFlag_0 = false;
            _S55 = _S57;
        }
        else
        {
            _runFlag_0 = _S54;
            _S55 = rgb_0;
        }
        if(_runFlag_0)
        {
            var _S58 : Tuple_5 = s_apply_curve_at_0(l_0.r_0, &((*p_3)), slots_3, _S50);
            _S53.value2_1 = _S58.value1_2;
            var _S59 : Tuple_5 = s_apply_curve_at_0(l_0.g_0, &((*p_3)), slots_3, _S50);
            _S53.value3_0 = _S59.value1_2;
            var _S60 : Tuple_5 = s_apply_curve_at_0(l_0.b_0, &((*p_3)), slots_3, _S50);
            _S53.value4_1 = _S60.value1_2;
            _S55 = vec3<f32>(_S58.value0_6, _S59.value0_6, _S60.value0_6);
        }
    }
    else
    {
        _S55 = rgb_0;
    }
    var _S61 : Tuple_4;
    _S61.value0_5 = _S55;
    _S61.value1_1 = _S53;
    return _S61;
}

struct RgbCurveParams_0
{
     luminance_0 : vec4<f32>,
     count_0 : vec4<u32>,
     end_value_0 : vec4<f32>,
     end_power_0 : vec4<f32>,
     autoscale_0 : u32,
     preserve_colors_0 : u32,
     pad0_0 : u32,
     pad1_0 : u32,
     gradient_curve_0 : vec4<u32>,
     gradient_node_0 : vec4<u32>,
     gradient_kind_0 : vec4<u32>,
};

struct s_paramCtx_s_bwdCallableCtx_rgb_curve_0
{
     value0_9 : vec3<f32>,
     value1_3 : Lookup_0,
     value2_2 : RgbCurveParams_0,
     value3_1 : bool,
     value4_2 : Gradient_0,
};

struct s_bwdCallableCtx_rgb_curve_0
{
     value0_10 : s_paramCtx_s_bwdCallableCtx_rgb_curve_0,
     value1_4 : s_minCtx_s_bwdCallableCtx_rgb_curve_0,
};

struct s_paramCtx_s_bwdCallableCtx_curve_at_0
{
     value0_11 : Point_0,
     value1_5 : RgbCurveParams_0,
     value2_3 : bool,
     value3_2 : Gradient_0,
};

struct s_bwdCallableCtx_curve_at_0
{
     value0_12 : s_paramCtx_s_bwdCallableCtx_curve_at_0,
     value1_6 : s_minCtx_s_bwdCallableCtx_curve_at_0,
};

fn Gradient_x24_syn_dzero_0() -> Gradient_0
{
    var result_0 : Gradient_0;
    result_0.s0_0 = 0.0f;
    result_0.s1_0 = 0.0f;
    result_0.s2_0 = 0.0f;
    result_0.s3_0 = 0.0f;
    return result_0;
}

struct s_paramCtx_s_bwdCallableCtx_knot_0
{
     value0_13 : u32,
     value1_7 : u32,
     value2_4 : RgbCurveParams_0,
     value3_3 : bool,
};

struct s_paramCtx_s_bwdCallableCtx_slotted_0
{
     value0_14 : u32,
     value1_8 : u32,
     value2_5 : u32,
     value3_4 : RgbCurveParams_0,
     value4_3 : bool,
};

struct s_bwdCallableCtx_slotted_0
{
     value0_15 : s_paramCtx_s_bwdCallableCtx_slotted_0,
};

struct s_bwdCallableCtx_knot_0
{
     value0_16 : s_paramCtx_s_bwdCallableCtx_knot_0,
};

fn s_apply_slotted_1( _S62 : f32,  curve_3 : u32,  node_1 : u32,  kind_1 : u32,  p_4 : RgbCurveParams_0,  slots_4 : bool,  _S63 : Gradient_0) -> Tuple_7
{
    var _S64 : f32;
    if(!!slots_4)
    {
        var _S65 : bool;
        if((p_4.gradient_curve_0.x) == curve_3)
        {
            _S65 = (p_4.gradient_node_0.x) == node_1;
        }
        else
        {
            _S65 = false;
        }
        if(_S65)
        {
            _S65 = (p_4.gradient_kind_0.x) == kind_1;
        }
        else
        {
            _S65 = false;
        }
        if(_S65)
        {
            _S64 = _S63.s0_0;
        }
        else
        {
            _S64 = _S62;
        }
        if((p_4.gradient_curve_0.y) == curve_3)
        {
            _S65 = (p_4.gradient_node_0.y) == node_1;
        }
        else
        {
            _S65 = false;
        }
        if(_S65)
        {
            _S65 = (p_4.gradient_kind_0.y) == kind_1;
        }
        else
        {
            _S65 = false;
        }
        if(_S65)
        {
            _S64 = _S63.s1_0;
        }
        if((p_4.gradient_curve_0.z) == curve_3)
        {
            _S65 = (p_4.gradient_node_0.z) == node_1;
        }
        else
        {
            _S65 = false;
        }
        if(_S65)
        {
            _S65 = (p_4.gradient_kind_0.z) == kind_1;
        }
        else
        {
            _S65 = false;
        }
        if(_S65)
        {
            _S64 = _S63.s2_0;
        }
        if((p_4.gradient_curve_0.w) == curve_3)
        {
            _S65 = (p_4.gradient_node_0.w) == node_1;
        }
        else
        {
            _S65 = false;
        }
        if(_S65)
        {
            _S65 = (p_4.gradient_kind_0.w) == kind_1;
        }
        else
        {
            _S65 = false;
        }
        if(_S65)
        {
            _S64 = _S63.s3_0;
        }
    }
    else
    {
        _S64 = _S62;
    }
    var _S66 : Tuple_7;
    _S66.value0_8 = _S64;
    return _S66;
}

fn Gradient_x24_syn_dadd_0( SLANG_anonymous_0_0 : Gradient_0,  SLANG_anonymous_1_0 : Gradient_0) -> Gradient_0
{
    var result_1 : Gradient_0;
    result_1.s0_0 = SLANG_anonymous_0_0.s0_0 + SLANG_anonymous_1_0.s0_0;
    result_1.s1_0 = SLANG_anonymous_0_0.s1_0 + SLANG_anonymous_1_0.s1_0;
    result_1.s2_0 = SLANG_anonymous_0_0.s2_0 + SLANG_anonymous_1_0.s2_0;
    result_1.s3_0 = SLANG_anonymous_0_0.s3_0 + SLANG_anonymous_1_0.s3_0;
    return result_1;
}

fn s_bwdProp_slotted_0( _S67 : s_bwdCallableCtx_slotted_0,  _S68 : ptr<function, f32>,  _S69 : ptr<function, Gradient_0>,  _S70 : f32)
{
    var _S71 : bool = !_S67.value0_15.value4_3;
    var _S72 : bool = !_S71;
    var _S73 : bool;
    var _S74 : bool;
    var _S75 : bool;
    var _S76 : bool;
    if(_S72)
    {
        if((_S67.value0_15.value3_4.gradient_curve_0.x) == (_S67.value0_15.value0_14))
        {
            _S73 = (_S67.value0_15.value3_4.gradient_node_0.x) == (_S67.value0_15.value1_8);
        }
        else
        {
            _S73 = false;
        }
        if(_S73)
        {
            _S73 = (_S67.value0_15.value3_4.gradient_kind_0.x) == (_S67.value0_15.value2_5);
        }
        else
        {
            _S73 = false;
        }
        if((_S67.value0_15.value3_4.gradient_curve_0.y) == (_S67.value0_15.value0_14))
        {
            _S74 = (_S67.value0_15.value3_4.gradient_node_0.y) == (_S67.value0_15.value1_8);
        }
        else
        {
            _S74 = false;
        }
        if(_S74)
        {
            _S74 = (_S67.value0_15.value3_4.gradient_kind_0.y) == (_S67.value0_15.value2_5);
        }
        else
        {
            _S74 = false;
        }
        if((_S67.value0_15.value3_4.gradient_curve_0.z) == (_S67.value0_15.value0_14))
        {
            _S75 = (_S67.value0_15.value3_4.gradient_node_0.z) == (_S67.value0_15.value1_8);
        }
        else
        {
            _S75 = false;
        }
        if(_S75)
        {
            _S75 = (_S67.value0_15.value3_4.gradient_kind_0.z) == (_S67.value0_15.value2_5);
        }
        else
        {
            _S75 = false;
        }
        if((_S67.value0_15.value3_4.gradient_curve_0.w) == (_S67.value0_15.value0_14))
        {
            _S76 = (_S67.value0_15.value3_4.gradient_node_0.w) == (_S67.value0_15.value1_8);
        }
        else
        {
            _S76 = false;
        }
        if(_S76)
        {
            _S76 = (_S67.value0_15.value3_4.gradient_kind_0.w) == (_S67.value0_15.value2_5);
        }
        else
        {
            _S76 = false;
        }
        var _S77 : bool = _S73;
        var _S78 : bool = _S74;
        _S73 = _S76;
        _S74 = _S75;
        _S75 = _S78;
        _S76 = _S77;
    }
    else
    {
        _S73 = false;
        _S74 = false;
        _S75 = false;
        _S76 = false;
    }
    var _S79 : Gradient_0 = Gradient_x24_syn_dzero_0();
    var _S80 : f32;
    var _S81 : f32;
    var _S82 : Gradient_0;
    if(_S72)
    {
        if(_S73)
        {
            var _S83 : Gradient_0 = _S79;
            _S83.s3_0 = _S70;
            var _S84 : Gradient_0 = Gradient_x24_syn_dadd_0(_S79, _S83);
            _S80 = 0.0f;
            _S82 = _S84;
        }
        else
        {
            _S80 = _S70;
            _S82 = _S79;
        }
        if(_S74)
        {
            var _S85 : Gradient_0 = _S79;
            _S85.s2_0 = _S80;
            var _S86 : Gradient_0 = Gradient_x24_syn_dadd_0(_S82, _S85);
            _S80 = 0.0f;
            _S82 = _S86;
        }
        if(_S75)
        {
            var _S87 : Gradient_0 = _S79;
            _S87.s1_0 = _S80;
            var _S88 : Gradient_0 = Gradient_x24_syn_dadd_0(_S82, _S87);
            _S80 = 0.0f;
            _S82 = _S88;
        }
        if(_S76)
        {
            var _S89 : Gradient_0 = _S79;
            _S89.s0_0 = _S80;
            var _S90 : Gradient_0 = Gradient_x24_syn_dadd_0(_S82, _S89);
            _S80 = 0.0f;
            _S82 = _S90;
        }
        var _S91 : f32 = _S80;
        _S80 = 0.0f;
        _S81 = _S91;
    }
    else
    {
        _S80 = _S70;
        _S81 = 0.0f;
        _S82 = _S79;
    }
    if(_S71)
    {
        _S80 = _S80 + _S81;
    }
    else
    {
        _S80 = _S81;
    }
    (*_S68) = _S80;
    (*_S69) = _S82;
    return;
}

fn s_bwdProp_knot_0( _S92 : s_bwdCallableCtx_knot_0,  _S93 : ptr<function, Gradient_0>,  _S94 : vec3<f32>)
{
    var _S95 : s_paramCtx_s_bwdCallableCtx_slotted_0;
    _S95.value0_14 = _S92.value0_16.value0_13;
    _S95.value1_8 = _S92.value0_16.value1_7;
    _S95.value2_5 = u32(0);
    _S95.value3_4 = _S92.value0_16.value2_4;
    _S95.value4_3 = _S92.value0_16.value3_3;
    var _S96 : s_bwdCallableCtx_slotted_0 = s_bwdCallableCtx_slotted_0( _S95 );
    var _S97 : s_paramCtx_s_bwdCallableCtx_slotted_0;
    _S97.value0_14 = _S92.value0_16.value0_13;
    _S97.value1_8 = _S92.value0_16.value1_7;
    _S97.value2_5 = u32(1);
    _S97.value3_4 = _S92.value0_16.value2_4;
    _S97.value4_3 = _S92.value0_16.value3_3;
    var _S98 : s_bwdCallableCtx_slotted_0 = s_bwdCallableCtx_slotted_0( _S97 );
    var _S99 : s_paramCtx_s_bwdCallableCtx_slotted_0;
    _S99.value0_14 = _S92.value0_16.value0_13;
    _S99.value1_8 = _S92.value0_16.value1_7;
    _S99.value2_5 = u32(2);
    _S99.value3_4 = _S92.value0_16.value2_4;
    _S99.value4_3 = _S92.value0_16.value3_3;
    var _S100 : s_bwdCallableCtx_slotted_0 = s_bwdCallableCtx_slotted_0( _S99 );
    var _S101 : Gradient_0 = Gradient_x24_syn_dzero_0();
    var _S102 : f32 = 0.0f;
    var _S103 : Gradient_0;
    _S103.s0_0 = 0.0f;
    _S103.s1_0 = 0.0f;
    _S103.s2_0 = 0.0f;
    _S103.s3_0 = 0.0f;
    s_bwdProp_slotted_0(_S100, &(_S102), &(_S103), _S94[i32(2)]);
    var _S104 : f32 = 0.0f;
    var _S105 : Gradient_0;
    _S105.s0_0 = 0.0f;
    _S105.s1_0 = 0.0f;
    _S105.s2_0 = 0.0f;
    _S105.s3_0 = 0.0f;
    s_bwdProp_slotted_0(_S98, &(_S104), &(_S105), _S94[i32(1)]);
    var _S106 : f32 = 0.0f;
    var _S107 : Gradient_0;
    _S107.s0_0 = 0.0f;
    _S107.s1_0 = 0.0f;
    _S107.s2_0 = 0.0f;
    _S107.s3_0 = 0.0f;
    s_bwdProp_slotted_0(_S96, &(_S106), &(_S107), _S94[i32(0)]);
    (*_S93) = Gradient_x24_syn_dadd_0(Gradient_x24_syn_dadd_0(Gradient_x24_syn_dadd_0(_S103, _S105), _S107), _S101);
    return;
}

fn s_bwdProp_curve_at_0( _S108 : s_bwdCallableCtx_curve_at_0,  _S109 : ptr<function, Gradient_0>,  _S110 : f32)
{
    var _S111 : vec4<f32> = vec4<f32>(0.0f);
    var _S112 : vec4<u32> = vec4<u32>(u32(0));
    var _S113 : RgbCurveParams_0 = RgbCurveParams_0( _S111, _S112, _S111, _S111, u32(0), u32(0), u32(0), u32(0), _S112, _S112, _S112 );
    var _S114 : s_paramCtx_s_bwdCallableCtx_knot_0 = s_paramCtx_s_bwdCallableCtx_knot_0( u32(0), u32(0), _S113, false );
    var _S115 : s_paramCtx_s_bwdCallableCtx_slotted_0 = s_paramCtx_s_bwdCallableCtx_slotted_0( u32(0), u32(0), u32(0), _S113, false );
    var _S116 : bool = (_S108.value0_12.value0_11.above_0) != u32(0);
    var _S117 : f32;
    var _S118 : f32;
    var _S119 : f32;
    var _S120 : f32;
    var _S121 : f32;
    var _S122 : s_bwdCallableCtx_exp2_0;
    var _S123 : s_bwdCallableCtx_clamp_0;
    var _S124 : s_bwdCallableCtx_max_0;
    var _S125 : s_bwdCallableCtx_max_0;
    var _S126 : s_bwdCallableCtx_max_0;
    var _S127 : s_bwdCallableCtx_log2_0;
    var _S128 : s_bwdCallableCtx_log2_0;
    var _S129 : s_bwdCallableCtx_slotted_0;
    var _S130 : s_bwdCallableCtx_slotted_0;
    var _S131 : s_bwdCallableCtx_knot_0;
    if(_S116)
    {
        var _S132 : u32 = _S108.value0_12.value1_5.count_0[_S108.value0_12.value0_11.curve_0] - u32(1);
        var _S133 : s_paramCtx_s_bwdCallableCtx_knot_0;
        _S133.value0_13 = _S108.value0_12.value0_11.curve_0;
        _S133.value1_7 = _S132;
        _S133.value2_4 = _S108.value0_12.value1_5;
        _S133.value3_3 = _S108.value0_12.value2_3;
        var x_last_1 : f32 = _S108.value1_6.value0_4.x;
        var _S134 : Tuple_7 = s_apply_slotted_1(_S108.value0_12.value1_5.end_value_0[_S108.value0_12.value0_11.curve_0], _S108.value0_12.value0_11.curve_0, u32(0), u32(3), _S108.value0_12.value1_5, _S108.value0_12.value2_3, _S108.value0_12.value3_2);
        var _S135 : s_paramCtx_s_bwdCallableCtx_slotted_0;
        _S135.value0_14 = _S108.value0_12.value0_11.curve_0;
        _S135.value1_8 = u32(0);
        _S135.value2_5 = u32(3);
        _S135.value3_4 = _S108.value0_12.value1_5;
        _S135.value4_3 = _S108.value0_12.value2_3;
        var _S136 : Tuple_7 = s_apply_slotted_1(_S108.value0_12.value1_5.end_power_0[_S108.value0_12.value0_11.curve_0], _S108.value0_12.value0_11.curve_0, u32(0), u32(4), _S108.value0_12.value1_5, _S108.value0_12.value2_3, _S108.value0_12.value3_2);
        var _S137 : s_paramCtx_s_bwdCallableCtx_slotted_0;
        _S137.value0_14 = _S108.value0_12.value0_11.curve_0;
        _S137.value1_8 = u32(0);
        _S137.value2_5 = u32(4);
        _S137.value3_4 = _S108.value0_12.value1_5;
        _S137.value4_3 = _S108.value0_12.value2_3;
        var _S138 : Tuple_3 = s_apply_max_0(_S108.value0_12.value0_11.v_0, x_last_1);
        var _S139 : s_bwdCallableCtx_max_0;
        _S139._S22 = _S108.value0_12.value0_11.v_0;
        _S139._S23 = x_last_1;
        var _S140 : Tuple_2 = s_apply_log2_0(_S138.value0_3);
        var _S141 : s_bwdCallableCtx_log2_0;
        _S141._S18 = _S138.value0_3;
        var _S142 : Tuple_2 = s_apply_log2_0(x_last_1);
        var _S143 : s_bwdCallableCtx_log2_0;
        _S143._S18 = x_last_1;
        var _S144 : f32 = _S140.value0_2 - _S142.value0_2;
        var _S145 : Tuple_3 = s_apply_max_0(_S144, 0.0f);
        var _S146 : s_bwdCallableCtx_max_0;
        _S146._S22 = _S144;
        _S146._S23 = 0.0f;
        var _S147 : Tuple_3 = s_apply_max_0(_S145.value0_3, 9.99999968265522539e-21f);
        var _S148 : s_bwdCallableCtx_max_0;
        _S148._S22 = _S145.value0_3;
        _S148._S23 = 9.99999968265522539e-21f;
        var limit_1 : f32 = 64.0f / _S147.value0_3;
        var _S149 : f32 = _S147.value0_3 * _S147.value0_3;
        var _S150 : f32 = - limit_1;
        var _S151 : Tuple_1 = s_apply_clamp_0(_S136.value0_8, _S150, limit_1);
        var _S152 : s_bwdCallableCtx_clamp_0;
        _S152._S6 = _S136.value0_8;
        _S152._S7 = _S150;
        _S152._S8 = limit_1;
        var _S153 : f32 = _S151.value0_1 * _S145.value0_3;
        var _S154 : Tuple_0 = s_apply_exp2_0(_S153);
        var _S155 : s_bwdCallableCtx_exp2_0;
        _S155._S2 = _S153;
        _S117 = _S134.value0_8;
        _S118 = _S154.value0_0;
        _S122 = _S155;
        _S119 = _S151.value0_1;
        _S120 = _S145.value0_3;
        _S123 = _S152;
        _S121 = _S149;
        _S124 = _S148;
        _S125 = _S146;
        _S127 = _S143;
        _S128 = _S141;
        _S126 = _S139;
        _S129.value0_15 = _S137;
        _S130.value0_15 = _S135;
        _S131.value0_16 = _S133;
    }
    else
    {
        _S117 = 0.0f;
        _S118 = 0.0f;
        _S122._S2 = 0.0f;
        _S119 = 0.0f;
        _S120 = 0.0f;
        _S123._S6 = 0.0f;
        _S123._S7 = 0.0f;
        _S123._S8 = 0.0f;
        _S121 = 0.0f;
        _S124._S22 = 0.0f;
        _S124._S23 = 0.0f;
        _S125._S22 = 0.0f;
        _S125._S23 = 0.0f;
        _S127._S18 = 0.0f;
        _S128._S18 = 0.0f;
        _S126._S22 = 0.0f;
        _S126._S23 = 0.0f;
        _S129.value0_15 = _S115;
        _S130.value0_15 = _S115;
        _S131.value0_16 = _S114;
    }
    var _S156 : bool = !_S116;
    var _S157 : f32;
    var _S158 : f32;
    var _S159 : f32;
    var _S160 : f32;
    var _S161 : f32;
    var _S162 : f32;
    var _S163 : f32;
    var _S164 : f32;
    var _S165 : f32;
    var _S166 : f32;
    var _S167 : f32;
    var _S168 : f32;
    var _S169 : f32;
    var _S170 : f32;
    var _S171 : f32;
    var _S172 : s_bwdCallableCtx_clamp_0;
    var _S173 : s_bwdCallableCtx_max_0;
    var _S174 : s_bwdCallableCtx_knot_0;
    var _S175 : s_bwdCallableCtx_knot_0;
    if(_S156)
    {
        var _S176 : s_paramCtx_s_bwdCallableCtx_knot_0;
        _S176.value0_13 = _S108.value0_12.value0_11.curve_0;
        _S176.value1_7 = _S108.value0_12.value0_11.segment_0;
        _S176.value2_4 = _S108.value0_12.value1_5;
        _S176.value3_3 = _S108.value0_12.value2_3;
        var _S177 : u32 = _S108.value0_12.value0_11.segment_0 + u32(1);
        var _S178 : s_paramCtx_s_bwdCallableCtx_knot_0;
        _S178.value0_13 = _S108.value0_12.value0_11.curve_0;
        _S178.value1_7 = _S177;
        _S178.value2_4 = _S108.value0_12.value1_5;
        _S178.value3_3 = _S108.value0_12.value2_3;
        var _S179 : f32 = _S108.value1_6.value2_0.x;
        var h_1 : f32 = _S108.value1_6.value4_0.x - _S179;
        var _S180 : Tuple_3 = s_apply_max_0(_S108.value0_12.value0_11.v_0, _S179);
        var _S181 : s_bwdCallableCtx_max_0;
        _S181._S22 = _S108.value0_12.value0_11.v_0;
        _S181._S23 = _S179;
        var _S182 : f32 = _S180.value0_3 - _S179;
        var t_2 : f32 = _S182 / h_1;
        var _S183 : f32 = h_1 * h_1;
        var t2_1 : f32 = t_2 * t_2;
        var t3_1 : f32 = t_2 * t2_1;
        var _S184 : f32 = 3.0f * t2_1;
        var _S185 : f32 = 2.0f * t3_1 - _S184 + 1.0f;
        var _S186 : f32 = _S108.value1_6.value2_0.y;
        var _S187 : f32 = t3_1 - 2.0f * t2_1 + t_2;
        var _S188 : f32 = _S187 * h_1;
        var _S189 : f32 = _S108.value1_6.value2_0.z;
        var _S190 : f32 = -2.0f * t3_1 + _S184;
        var _S191 : f32 = _S108.value1_6.value4_0.y;
        var _S192 : f32 = t3_1 - t2_1;
        var _S193 : f32 = _S192 * h_1;
        var _S194 : f32 = _S108.value1_6.value4_0.z;
        var _S195 : s_bwdCallableCtx_clamp_0;
        _S195._S6 = _S185 * _S186 + _S188 * _S189 + _S190 * _S191 + _S193 * _S194;
        _S195._S7 = 0.0f;
        _S195._S8 = 1.0f;
        _S172 = _S195;
        _S157 = _S193;
        _S158 = _S194;
        _S159 = _S192;
        _S160 = h_1;
        _S161 = _S190;
        _S162 = _S191;
        _S163 = _S188;
        _S164 = _S189;
        _S165 = _S187;
        _S166 = _S185;
        _S167 = _S186;
        _S168 = t_2;
        _S169 = t2_1;
        _S170 = _S183;
        _S171 = _S182;
        _S173 = _S181;
        _S174.value0_16 = _S178;
        _S175.value0_16 = _S176;
    }
    else
    {
        _S172._S6 = 0.0f;
        _S172._S7 = 0.0f;
        _S172._S8 = 0.0f;
        _S157 = 0.0f;
        _S158 = 0.0f;
        _S159 = 0.0f;
        _S160 = 0.0f;
        _S161 = 0.0f;
        _S162 = 0.0f;
        _S163 = 0.0f;
        _S164 = 0.0f;
        _S165 = 0.0f;
        _S166 = 0.0f;
        _S167 = 0.0f;
        _S168 = 0.0f;
        _S169 = 0.0f;
        _S170 = 0.0f;
        _S171 = 0.0f;
        _S173._S22 = 0.0f;
        _S173._S23 = 0.0f;
        _S174.value0_16 = _S114;
        _S175.value0_16 = _S114;
    }
    var _S196 : Gradient_0 = Gradient_x24_syn_dzero_0();
    var _S197 : Gradient_0;
    if(_S156)
    {
        var _S198 : f32 = 0.0f;
        var _S199 : f32 = 0.0f;
        var _S200 : f32 = 0.0f;
        s_bwdProp_clamp_0(_S172, &(_S198), &(_S199), &(_S200), _S110);
        var _S201 : f32 = _S157 * _S198;
        var _S202 : f32 = _S158 * _S198;
        var _S203 : f32 = _S159 * _S202;
        var _S204 : f32 = _S160 * _S202;
        var _S205 : f32 = _S161 * _S198;
        var _S206 : f32 = _S162 * _S198;
        var _S207 : f32 = _S163 * _S198;
        var _S208 : f32 = _S164 * _S198;
        var _S209 : f32 = _S165 * _S208;
        var _S210 : f32 = _S160 * _S208;
        var _S211 : f32 = _S166 * _S198;
        var _S212 : f32 = _S167 * _S198;
        var _S213 : f32 = _S204 + -2.0f * _S206 + _S210 + 2.0f * _S212;
        var _S214 : f32 = _S168 * (- _S204 + 2.0f * - _S210 + 3.0f * (_S206 + - _S212) + _S168 * _S213);
        var _S215 : f32 = (_S210 + _S169 * _S213 + _S214 + _S214) / _S170;
        var _S216 : f32 = _S171 * - _S215;
        var _S217 : f32 = _S160 * _S215;
        var _S218 : f32 = - _S217;
        var _S219 : f32 = 0.0f;
        var _S220 : f32 = 0.0f;
        s_bwdProp_max_0(_S173, &(_S219), &(_S220), _S217);
        var _S221 : f32 = _S203 + _S209 + _S216;
        var _S222 : f32 = _S218 + _S220 + - _S221;
        var _S223 : vec3<f32> = vec3<f32>(_S221, _S205, _S201);
        var _S224 : Gradient_0;
        _S224.s0_0 = 0.0f;
        _S224.s1_0 = 0.0f;
        _S224.s2_0 = 0.0f;
        _S224.s3_0 = 0.0f;
        s_bwdProp_knot_0(_S174, &(_S224), _S223);
        var _S225 : vec3<f32> = vec3<f32>(_S222, _S211, _S207);
        var _S226 : Gradient_0;
        _S226.s0_0 = 0.0f;
        _S226.s1_0 = 0.0f;
        _S226.s2_0 = 0.0f;
        _S226.s3_0 = 0.0f;
        s_bwdProp_knot_0(_S175, &(_S226), _S225);
        var _S227 : Gradient_0 = Gradient_x24_syn_dadd_0(Gradient_x24_syn_dadd_0(_S224, _S226), _S196);
        _S157 = 0.0f;
        _S197 = _S227;
    }
    else
    {
        _S157 = _S110;
        _S197 = _S196;
    }
    if(_S116)
    {
        var _S228 : f32 = _S117 * _S157;
        var _S229 : f32 = _S118 * _S157;
        var _S230 : f32 = 0.0f;
        s_bwdProp_exp2_0(_S122, &(_S230), _S228);
        var _S231 : f32 = _S119 * _S230;
        var _S232 : f32 = _S120 * _S230;
        var _S233 : f32 = 0.0f;
        var _S234 : f32 = 0.0f;
        var _S235 : f32 = 0.0f;
        s_bwdProp_clamp_0(_S123, &(_S233), &(_S234), &(_S235), _S232);
        var _S236 : f32 = 64.0f * - ((_S235 + - _S234) / _S121);
        var _S237 : f32 = 0.0f;
        var _S238 : f32 = 0.0f;
        s_bwdProp_max_0(_S124, &(_S237), &(_S238), _S236);
        var _S239 : f32 = _S231 + _S237;
        var _S240 : f32 = 0.0f;
        var _S241 : f32 = 0.0f;
        s_bwdProp_max_0(_S125, &(_S240), &(_S241), _S239);
        var _S242 : f32 = - _S240;
        var _S243 : f32 = 0.0f;
        s_bwdProp_log2_0(_S127, &(_S243), _S242);
        var _S244 : f32 = 0.0f;
        s_bwdProp_log2_0(_S128, &(_S244), _S240);
        var _S245 : f32 = 0.0f;
        var _S246 : f32 = 0.0f;
        s_bwdProp_max_0(_S126, &(_S245), &(_S246), _S244);
        var _S247 : f32 = 0.0f;
        var _S248 : Gradient_0;
        _S248.s0_0 = 0.0f;
        _S248.s1_0 = 0.0f;
        _S248.s2_0 = 0.0f;
        _S248.s3_0 = 0.0f;
        s_bwdProp_slotted_0(_S129, &(_S247), &(_S248), _S233);
        var _S249 : f32 = 0.0f;
        var _S250 : Gradient_0;
        _S250.s0_0 = 0.0f;
        _S250.s1_0 = 0.0f;
        _S250.s2_0 = 0.0f;
        _S250.s3_0 = 0.0f;
        s_bwdProp_slotted_0(_S130, &(_S249), &(_S250), _S229);
        var _S251 : vec3<f32> = vec3<f32>(_S243 + _S246, 0.0f, 0.0f);
        var _S252 : Gradient_0;
        _S252.s0_0 = 0.0f;
        _S252.s1_0 = 0.0f;
        _S252.s2_0 = 0.0f;
        _S252.s3_0 = 0.0f;
        s_bwdProp_knot_0(_S131, &(_S252), _S251);
        _S197 = Gradient_x24_syn_dadd_0(Gradient_x24_syn_dadd_0(Gradient_x24_syn_dadd_0(_S248, _S250), _S252), _S197);
    }
    (*_S109) = _S197;
    return;
}

fn s_bwdProp_rgb_curve_0( _S253 : s_bwdCallableCtx_rgb_curve_0,  _S254 : ptr<function, Gradient_0>,  _S255 : vec3<f32>)
{
    var _S256 : Point_0 = Point_0( u32(0), u32(0), u32(0), 0.0f );
    var _S257 : vec4<f32> = vec4<f32>(0.0f);
    var _S258 : vec4<u32> = vec4<u32>(u32(0));
    var _S259 : RgbCurveParams_0 = RgbCurveParams_0( _S257, _S258, _S257, _S257, u32(0), u32(0), u32(0), u32(0), _S258, _S258, _S258 );
    var _S260 : Gradient_0 = Gradient_0( 0.0f, 0.0f, 0.0f, 0.0f );
    var _S261 : s_paramCtx_s_bwdCallableCtx_curve_at_0 = s_paramCtx_s_bwdCallableCtx_curve_at_0( _S256, _S259, false, _S260 );
    var _S262 : vec3<f32> = vec3<f32>(0.0f);
    var _S263 : s_minCtx_s_bwdCallableCtx_curve_at_0 = s_minCtx_s_bwdCallableCtx_curve_at_0( _S262, _S262, _S262 );
    var _S264 : bool = !((_S253.value0_10.value1_3.mode_0) == u32(2));
    var _runFlag_1 : bool;
    var _S265 : bool;
    var _S266 : f32;
    var _S267 : s_bwdCallableCtx_curve_at_0;
    var _S268 : s_bwdCallableCtx_curve_at_0;
    var _S269 : s_bwdCallableCtx_curve_at_0;
    var _S270 : s_bwdCallableCtx_curve_at_0;
    if(_S264)
    {
        var _S271 : bool = (_S253.value0_10.value1_3.mode_0) == u32(1);
        if(_S271)
        {
            var _S272 : s_paramCtx_s_bwdCallableCtx_curve_at_0;
            _S272.value0_11 = _S253.value0_10.value1_3.r_0;
            _S272.value1_5 = _S253.value0_10.value2_2;
            _S272.value2_3 = _S253.value0_10.value3_1;
            _S272.value3_2 = _S253.value0_10.value4_2;
            _runFlag_1 = false;
            _S266 = _S253.value0_10.value1_3.r_0.v_0;
            _S267.value0_12 = _S272;
            _S267.value1_6 = _S253.value1_4.value1_0;
        }
        else
        {
            _runFlag_1 = _S264;
            _S266 = 0.0f;
            _S267.value0_12 = _S261;
            _S267.value1_6 = _S263;
        }
        if(_runFlag_1)
        {
            var _S273 : s_paramCtx_s_bwdCallableCtx_curve_at_0;
            _S273.value0_11 = _S253.value0_10.value1_3.r_0;
            _S273.value1_5 = _S253.value0_10.value2_2;
            _S273.value2_3 = _S253.value0_10.value3_1;
            _S273.value3_2 = _S253.value0_10.value4_2;
            var _S274 : s_paramCtx_s_bwdCallableCtx_curve_at_0;
            _S274.value0_11 = _S253.value0_10.value1_3.g_0;
            _S274.value1_5 = _S253.value0_10.value2_2;
            _S274.value2_3 = _S253.value0_10.value3_1;
            _S274.value3_2 = _S253.value0_10.value4_2;
            var _S275 : s_paramCtx_s_bwdCallableCtx_curve_at_0;
            _S275.value0_11 = _S253.value0_10.value1_3.b_0;
            _S275.value1_5 = _S253.value0_10.value2_2;
            _S275.value2_3 = _S253.value0_10.value3_1;
            _S275.value3_2 = _S253.value0_10.value4_2;
            _S268.value0_12 = _S275;
            _S268.value1_6 = _S253.value1_4.value4_1;
            _S269.value0_12 = _S274;
            _S269.value1_6 = _S253.value1_4.value3_0;
            _S270.value0_12 = _S273;
            _S270.value1_6 = _S253.value1_4.value2_1;
        }
        else
        {
            _S268.value0_12 = _S261;
            _S268.value1_6 = _S263;
            _S269.value0_12 = _S261;
            _S269.value1_6 = _S263;
            _S270.value0_12 = _S261;
            _S270.value1_6 = _S263;
        }
        var _S276 : s_bwdCallableCtx_curve_at_0 = _S267;
        _S267 = _S268;
        _S268 = _S269;
        _S269 = _S270;
        _S265 = _S271;
        _S270 = _S276;
    }
    else
    {
        _runFlag_1 = false;
        _S267.value0_12 = _S261;
        _S267.value1_6 = _S263;
        _S268.value0_12 = _S261;
        _S268.value1_6 = _S263;
        _S269.value0_12 = _S261;
        _S269.value1_6 = _S263;
        _S265 = false;
        _S266 = 0.0f;
        _S270.value0_12 = _S261;
        _S270.value1_6 = _S263;
    }
    var _S277 : Gradient_0 = Gradient_x24_syn_dzero_0();
    var _S278 : Gradient_0;
    if(_S264)
    {
        var _S279 : vec3<f32>;
        if(_runFlag_1)
        {
            var _S280 : Gradient_0;
            _S280.s0_0 = 0.0f;
            _S280.s1_0 = 0.0f;
            _S280.s2_0 = 0.0f;
            _S280.s3_0 = 0.0f;
            s_bwdProp_curve_at_0(_S267, &(_S280), _S255[i32(2)]);
            var _S281 : Gradient_0;
            _S281.s0_0 = 0.0f;
            _S281.s1_0 = 0.0f;
            _S281.s2_0 = 0.0f;
            _S281.s3_0 = 0.0f;
            s_bwdProp_curve_at_0(_S268, &(_S281), _S255[i32(1)]);
            var _S282 : Gradient_0;
            _S282.s0_0 = 0.0f;
            _S282.s1_0 = 0.0f;
            _S282.s2_0 = 0.0f;
            _S282.s3_0 = 0.0f;
            s_bwdProp_curve_at_0(_S269, &(_S282), _S255[i32(0)]);
            var _S283 : Gradient_0 = Gradient_x24_syn_dadd_0(Gradient_x24_syn_dadd_0(Gradient_x24_syn_dadd_0(_S280, _S281), _S282), _S277);
            _S279 = _S262;
            _S278 = _S283;
        }
        else
        {
            _S279 = _S255;
            _S278 = _S277;
        }
        if(_S265)
        {
            var _S284 : vec3<f32> = _S253.value0_10.value0_9 * _S279;
            var _S285 : f32 = (_S284[i32(0)] + _S284[i32(1)] + _S284[i32(2)]) / _S266;
            var _S286 : Gradient_0;
            _S286.s0_0 = 0.0f;
            _S286.s1_0 = 0.0f;
            _S286.s2_0 = 0.0f;
            _S286.s3_0 = 0.0f;
            s_bwdProp_curve_at_0(_S270, &(_S286), _S285);
            _S278 = Gradient_x24_syn_dadd_0(_S286, _S278);
        }
    }
    else
    {
        _S278 = _S277;
    }
    (*_S254) = _S278;
    return;
}

fn _S287( _S288 : vec3<f32>,  _S289 : Lookup_0,  _S290 : ptr<function, RgbCurveParams_std140_0>,  _S291 : bool,  _S292 : ptr<function, DiffPair_Gradient_0>,  _S293 : vec3<f32>)
{
    var _S294 : vec4<f32> = (*_S290).luminance_0;
    var _S295 : vec4<u32> = (*_S290).count_0;
    var _S296 : vec4<f32> = (*_S290).end_value_0;
    var _S297 : vec4<f32> = (*_S290).end_power_0;
    var _S298 : u32 = (*_S290).autoscale_0;
    var _S299 : u32 = (*_S290).preserve_colors_0;
    var _S300 : u32 = (*_S290).pad0_0;
    var _S301 : u32 = (*_S290).pad1_0;
    var _S302 : vec4<u32> = (*_S290).gradient_curve_0;
    var _S303 : vec4<u32> = (*_S290).gradient_node_0;
    var _S304 : vec4<u32> = (*_S290).gradient_kind_0;
    var _S305 : Gradient_0 = (*_S292).primal_0;
    var _S306 : Tuple_4 = s_apply_rgb_curve_0(_S288, _S289, &((*_S290)), _S291, (*_S292).primal_0);
    var _S307 : s_paramCtx_s_bwdCallableCtx_rgb_curve_0;
    _S307.value0_9 = _S288;
    _S307.value1_3 = _S289;
    _S307.value2_2.luminance_0 = _S294;
    _S307.value2_2.count_0 = _S295;
    _S307.value2_2.end_value_0 = _S296;
    _S307.value2_2.end_power_0 = _S297;
    _S307.value2_2.autoscale_0 = _S298;
    _S307.value2_2.preserve_colors_0 = _S299;
    _S307.value2_2.pad0_0 = _S300;
    _S307.value2_2.pad1_0 = _S301;
    _S307.value2_2.gradient_curve_0 = _S302;
    _S307.value2_2.gradient_node_0 = _S303;
    _S307.value2_2.gradient_kind_0 = _S304;
    _S307.value3_1 = _S291;
    _S307.value4_2 = _S305;
    var _S308 : s_bwdCallableCtx_rgb_curve_0 = s_bwdCallableCtx_rgb_curve_0( _S307, _S306.value1_1 );
    var _S309 : Gradient_0 = (*_S292).differential_0;
    s_bwdProp_rgb_curve_0(_S308, &(_S309), _S293);
    (*_S292).differential_0 = _S309;
    return;
}

fn _d_exp2_1( dpx_4 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S310 : f32 = exp2(dpx_4.primal_0);
    var _S311 : DiffPair_float_0 = DiffPair_float_0( _S310, _S310 * 0.69314718246459961f * dpx_4.differential_0 );
    return _S311;
}

fn _d_clamp_1( dpx_5 : DiffPair_float_0,  dpMin_1 : DiffPair_float_0,  dpMax_1 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S312 : f32 = clamp(dpx_5.primal_0, dpMin_1.primal_0, dpMax_1.primal_0);
    var _S313 : bool;
    if(((dpx_5.primal_0) >= (dpMin_1.primal_0)))
    {
        _S313 = ((dpx_5.primal_0) <= (dpMax_1.primal_0));
    }
    else
    {
        _S313 = false;
    }
    var _S314 : f32;
    if(_S313)
    {
        _S314 = dpx_5.differential_0;
    }
    else
    {
        if(((dpx_5.primal_0) < (dpMin_1.primal_0)))
        {
            _S314 = dpMin_1.differential_0;
        }
        else
        {
            _S314 = dpMax_1.differential_0;
        }
    }
    var _S315 : DiffPair_float_0 = DiffPair_float_0( _S312, _S314 );
    return _S315;
}

fn _d_log2_1( dpx_6 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S316 : DiffPair_float_0 = DiffPair_float_0( log2(dpx_6.primal_0), 1.0f / (dpx_6.primal_0 * 0.69314718246459961f) * dpx_6.differential_0 );
    return _S316;
}

fn _d_max_1( dpx_7 : DiffPair_float_0,  dpy_1 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S317 : f32 = max(dpx_7.primal_0, dpy_1.primal_0);
    var _S318 : f32;
    if(((dpx_7.primal_0) > (dpy_1.primal_0)))
    {
        _S318 = dpx_7.differential_0;
    }
    else
    {
        if(((dpx_7.primal_0) < (dpy_1.primal_0)))
        {
            _S318 = dpy_1.differential_0;
        }
        else
        {
            _S318 = 0.5f * (dpx_7.differential_0 + dpy_1.differential_0);
        }
    }
    var _S319 : DiffPair_float_0 = DiffPair_float_0( _S317, _S318 );
    return _S319;
}

fn Gradient_x24init_0( s0_1 : f32,  s1_1 : f32,  s2_1 : f32,  s3_1 : f32) -> Gradient_0
{
    var _S320 : Gradient_0;
    _S320.s0_0 = s0_1;
    _S320.s1_0 = s1_1;
    _S320.s2_0 = s2_1;
    _S320.s3_0 = s3_1;
    return _S320;
}

fn locate_0( curve_4 : u32,  v_1 : f32,  p_5 : ptr<function, RgbCurveParams_std140_0>) -> Point_0
{
    var segment_1 : u32;
    var _S321 : vec4<u32> = (*p_5).count_0;
    var base_0 : u32 = curve_4 * u32(20);
    var _S322 : u32 = (*p_5).count_0[curve_4] - u32(2);
    var k_1 : u32 = u32(0);
    loop
    {
        if(((k_1 + u32(2)) < (_S321[curve_4])))
        {
        }
        else
        {
            segment_1 = _S322;
            break;
        }
        if((v_1 < (knots_0[base_0 + k_1 + u32(1)].x)))
        {
            segment_1 = k_1;
            break;
        }
        k_1 = k_1 + u32(1);
    }
    var point_1 : Point_0;
    point_1.curve_0 = curve_4;
    point_1.segment_0 = segment_1;
    var _S323 : i32;
    if((v_1 >= (knots_0[base_0 + _S321[curve_4] - u32(1)].x)))
    {
        _S323 = i32(1);
    }
    else
    {
        _S323 = i32(0);
    }
    point_1.above_0 = u32(_S323);
    point_1.v_0 = v_1;
    return point_1;
}

fn rgb_norm_0( rgb_1 : vec3<f32>,  norm_0 : u32,  luminance_1 : vec3<f32>) -> f32
{
    if(norm_0 == u32(1))
    {
        return dot(luminance_1, rgb_1);
    }
    if(norm_0 == u32(2))
    {
        return max(rgb_1.x, max(rgb_1.y, rgb_1.z));
    }
    if(norm_0 == u32(4))
    {
        return rgb_1.x + rgb_1.y + rgb_1.z;
    }
    if(norm_0 == u32(5))
    {
        return sqrt(dot(rgb_1, rgb_1));
    }
    if(norm_0 == u32(6))
    {
        var squared_0 : vec3<f32> = rgb_1 * rgb_1;
        var sum_0 : f32 = squared_0.x + squared_0.y + squared_0.z;
        var _S324 : f32;
        if((sum_0 > 0.0f))
        {
            _S324 = dot(rgb_1, squared_0) / sum_0;
        }
        else
        {
            _S324 = 0.0f;
        }
        return _S324;
    }
    return (rgb_1.x + rgb_1.y + rgb_1.z) / 3.0f;
}

fn locate_pixel_0( rgb_2 : vec3<f32>,  p_6 : ptr<function, RgbCurveParams_std140_0>) -> Lookup_0
{
    var l_1 : Lookup_0;
    if(((*p_6).autoscale_0) == u32(1))
    {
        l_1.mode_0 = u32(0);
        var _S325 : Point_0 = locate_0(u32(0), rgb_2.x, &((*p_6)));
        l_1.r_0 = _S325;
        var _S326 : Point_0 = locate_0(u32(1), rgb_2.y, &((*p_6)));
        l_1.g_0 = _S326;
        var _S327 : Point_0 = locate_0(u32(2), rgb_2.z, &((*p_6)));
        l_1.b_0 = _S327;
    }
    else
    {
        var _S328 : u32 = (*p_6).preserve_colors_0;
        if(((*p_6).preserve_colors_0) == u32(0))
        {
            l_1.mode_0 = u32(0);
            var _S329 : Point_0 = locate_0(u32(0), rgb_2.x, &((*p_6)));
            l_1.r_0 = _S329;
            var _S330 : Point_0 = locate_0(u32(0), rgb_2.y, &((*p_6)));
            l_1.g_0 = _S330;
            var _S331 : Point_0 = locate_0(u32(0), rgb_2.z, &((*p_6)));
            l_1.b_0 = _S331;
        }
        else
        {
            var norm_1 : f32 = rgb_norm_0(rgb_2, _S328, (*p_6).luminance_0.xyz);
            var _S332 : u32;
            if((norm_1 > 0.0f))
            {
                _S332 = u32(1);
            }
            else
            {
                _S332 = u32(2);
            }
            l_1.mode_0 = _S332;
            var _S333 : Point_0 = locate_0(u32(0), norm_1, &((*p_6)));
            l_1.r_0 = _S333;
            l_1.g_0 = _S333;
            l_1.b_0 = _S333;
        }
    }
    return l_1;
}

fn slotted_0( value_0 : f32,  curve_5 : u32,  node_2 : u32,  kind_2 : u32,  p_7 : ptr<function, RgbCurveParams_std140_0>,  slots_5 : bool,  g_1 : Gradient_0) -> f32
{
    if(!slots_5)
    {
        return value_0;
    }
    var _S334 : vec4<u32> = (*p_7).gradient_curve_0;
    var _S335 : bool;
    if(((*p_7).gradient_curve_0.x) == curve_5)
    {
        _S335 = ((*p_7).gradient_node_0.x) == node_2;
    }
    else
    {
        _S335 = false;
    }
    if(_S335)
    {
        _S335 = ((*p_7).gradient_kind_0.x) == kind_2;
    }
    else
    {
        _S335 = false;
    }
    var _S336 : f32;
    if(_S335)
    {
        _S336 = g_1.s0_0;
    }
    else
    {
        _S336 = value_0;
    }
    if((_S334.y) == curve_5)
    {
        _S335 = ((*p_7).gradient_node_0.y) == node_2;
    }
    else
    {
        _S335 = false;
    }
    if(_S335)
    {
        _S335 = ((*p_7).gradient_kind_0.y) == kind_2;
    }
    else
    {
        _S335 = false;
    }
    if(_S335)
    {
        _S336 = g_1.s1_0;
    }
    if((_S334.z) == curve_5)
    {
        _S335 = ((*p_7).gradient_node_0.z) == node_2;
    }
    else
    {
        _S335 = false;
    }
    if(_S335)
    {
        _S335 = ((*p_7).gradient_kind_0.z) == kind_2;
    }
    else
    {
        _S335 = false;
    }
    if(_S335)
    {
        _S336 = g_1.s2_0;
    }
    if((_S334.w) == curve_5)
    {
        _S335 = ((*p_7).gradient_node_0.w) == node_2;
    }
    else
    {
        _S335 = false;
    }
    if(_S335)
    {
        _S335 = ((*p_7).gradient_kind_0.w) == kind_2;
    }
    else
    {
        _S335 = false;
    }
    if(_S335)
    {
        _S336 = g_1.s3_0;
    }
    return _S336;
}

fn slotted_1( value_1 : f32,  curve_6 : u32,  node_3 : u32,  kind_3 : u32,  p_8 : ptr<function, RgbCurveParams_std140_0>,  slots_6 : bool,  g_2 : Gradient_0) -> f32
{
    if(!slots_6)
    {
        return value_1;
    }
    var _S337 : vec4<u32> = (*p_8).gradient_curve_0;
    var _S338 : bool;
    if(((*p_8).gradient_curve_0.x) == curve_6)
    {
        _S338 = ((*p_8).gradient_node_0.x) == node_3;
    }
    else
    {
        _S338 = false;
    }
    if(_S338)
    {
        _S338 = ((*p_8).gradient_kind_0.x) == kind_3;
    }
    else
    {
        _S338 = false;
    }
    var _S339 : f32;
    if(_S338)
    {
        _S339 = g_2.s0_0;
    }
    else
    {
        _S339 = value_1;
    }
    if((_S337.y) == curve_6)
    {
        _S338 = ((*p_8).gradient_node_0.y) == node_3;
    }
    else
    {
        _S338 = false;
    }
    if(_S338)
    {
        _S338 = ((*p_8).gradient_kind_0.y) == kind_3;
    }
    else
    {
        _S338 = false;
    }
    if(_S338)
    {
        _S339 = g_2.s1_0;
    }
    if((_S337.z) == curve_6)
    {
        _S338 = ((*p_8).gradient_node_0.z) == node_3;
    }
    else
    {
        _S338 = false;
    }
    if(_S338)
    {
        _S338 = ((*p_8).gradient_kind_0.z) == kind_3;
    }
    else
    {
        _S338 = false;
    }
    if(_S338)
    {
        _S339 = g_2.s2_0;
    }
    if((_S337.w) == curve_6)
    {
        _S338 = ((*p_8).gradient_node_0.w) == node_3;
    }
    else
    {
        _S338 = false;
    }
    if(_S338)
    {
        _S338 = ((*p_8).gradient_kind_0.w) == kind_3;
    }
    else
    {
        _S338 = false;
    }
    if(_S338)
    {
        _S339 = g_2.s3_0;
    }
    return _S339;
}

fn s_fwd_slotted_0( dpvalue_0 : DiffPair_float_0,  curve_7 : u32,  node_4 : u32,  kind_4 : u32,  p_9 : ptr<function, RgbCurveParams_std140_0>,  slots_7 : bool,  dpg_0 : DiffPair_Gradient_0) -> DiffPair_float_0
{
    if(!slots_7)
    {
        var _S340 : DiffPair_float_0 = DiffPair_float_0( dpvalue_0.primal_0, dpvalue_0.differential_0 );
        return _S340;
    }
    var _S341 : vec4<u32> = (*p_9).gradient_curve_0;
    var _S342 : bool;
    if(((*p_9).gradient_curve_0.x) == curve_7)
    {
        _S342 = ((*p_9).gradient_node_0.x) == node_4;
    }
    else
    {
        _S342 = false;
    }
    if(_S342)
    {
        _S342 = ((*p_9).gradient_kind_0.x) == kind_4;
    }
    else
    {
        _S342 = false;
    }
    var _S343 : f32;
    var _S344 : f32;
    if(_S342)
    {
        _S343 = dpg_0.primal_0.s0_0;
        _S344 = dpg_0.differential_0.s0_0;
    }
    else
    {
        _S343 = dpvalue_0.primal_0;
        _S344 = dpvalue_0.differential_0;
    }
    if((_S341.y) == curve_7)
    {
        _S342 = ((*p_9).gradient_node_0.y) == node_4;
    }
    else
    {
        _S342 = false;
    }
    if(_S342)
    {
        _S342 = ((*p_9).gradient_kind_0.y) == kind_4;
    }
    else
    {
        _S342 = false;
    }
    if(_S342)
    {
        _S343 = dpg_0.primal_0.s1_0;
        _S344 = dpg_0.differential_0.s1_0;
    }
    if((_S341.z) == curve_7)
    {
        _S342 = ((*p_9).gradient_node_0.z) == node_4;
    }
    else
    {
        _S342 = false;
    }
    if(_S342)
    {
        _S342 = ((*p_9).gradient_kind_0.z) == kind_4;
    }
    else
    {
        _S342 = false;
    }
    if(_S342)
    {
        _S343 = dpg_0.primal_0.s2_0;
        _S344 = dpg_0.differential_0.s2_0;
    }
    if((_S341.w) == curve_7)
    {
        _S342 = ((*p_9).gradient_node_0.w) == node_4;
    }
    else
    {
        _S342 = false;
    }
    if(_S342)
    {
        _S342 = ((*p_9).gradient_kind_0.w) == kind_4;
    }
    else
    {
        _S342 = false;
    }
    if(_S342)
    {
        _S343 = dpg_0.primal_0.s3_0;
        _S344 = dpg_0.differential_0.s3_0;
    }
    var _S345 : DiffPair_float_0 = DiffPair_float_0( _S343, _S344 );
    return _S345;
}

fn s_fwd_slotted_1( dpvalue_1 : DiffPair_float_0,  curve_8 : u32,  node_5 : u32,  kind_5 : u32,  p_10 : ptr<function, RgbCurveParams_std140_0>,  slots_8 : bool,  dpg_1 : DiffPair_Gradient_0) -> DiffPair_float_0
{
    if(!slots_8)
    {
        var _S346 : DiffPair_float_0 = DiffPair_float_0( dpvalue_1.primal_0, dpvalue_1.differential_0 );
        return _S346;
    }
    var _S347 : vec4<u32> = (*p_10).gradient_curve_0;
    var _S348 : bool;
    if(((*p_10).gradient_curve_0.x) == curve_8)
    {
        _S348 = ((*p_10).gradient_node_0.x) == node_5;
    }
    else
    {
        _S348 = false;
    }
    if(_S348)
    {
        _S348 = ((*p_10).gradient_kind_0.x) == kind_5;
    }
    else
    {
        _S348 = false;
    }
    var _S349 : f32;
    var _S350 : f32;
    if(_S348)
    {
        _S349 = dpg_1.primal_0.s0_0;
        _S350 = dpg_1.differential_0.s0_0;
    }
    else
    {
        _S349 = dpvalue_1.primal_0;
        _S350 = dpvalue_1.differential_0;
    }
    if((_S347.y) == curve_8)
    {
        _S348 = ((*p_10).gradient_node_0.y) == node_5;
    }
    else
    {
        _S348 = false;
    }
    if(_S348)
    {
        _S348 = ((*p_10).gradient_kind_0.y) == kind_5;
    }
    else
    {
        _S348 = false;
    }
    if(_S348)
    {
        _S349 = dpg_1.primal_0.s1_0;
        _S350 = dpg_1.differential_0.s1_0;
    }
    if((_S347.z) == curve_8)
    {
        _S348 = ((*p_10).gradient_node_0.z) == node_5;
    }
    else
    {
        _S348 = false;
    }
    if(_S348)
    {
        _S348 = ((*p_10).gradient_kind_0.z) == kind_5;
    }
    else
    {
        _S348 = false;
    }
    if(_S348)
    {
        _S349 = dpg_1.primal_0.s2_0;
        _S350 = dpg_1.differential_0.s2_0;
    }
    if((_S347.w) == curve_8)
    {
        _S348 = ((*p_10).gradient_node_0.w) == node_5;
    }
    else
    {
        _S348 = false;
    }
    if(_S348)
    {
        _S348 = ((*p_10).gradient_kind_0.w) == kind_5;
    }
    else
    {
        _S348 = false;
    }
    if(_S348)
    {
        _S349 = dpg_1.primal_0.s3_0;
        _S350 = dpg_1.differential_0.s3_0;
    }
    var _S351 : DiffPair_float_0 = DiffPair_float_0( _S349, _S350 );
    return _S351;
}

fn knot_0( curve_9 : u32,  k_2 : u32,  p_11 : ptr<function, RgbCurveParams_std140_0>,  slots_9 : bool,  g_3 : Gradient_0) -> vec3<f32>
{
    var t_3 : vec4<f32> = knots_0[curve_9 * u32(20) + k_2];
    var _S352 : f32 = slotted_1(t_3.x, curve_9, k_2, u32(0), &((*p_11)), slots_9, g_3);
    var _S353 : f32 = slotted_1(t_3.y, curve_9, k_2, u32(1), &((*p_11)), slots_9, g_3);
    var _S354 : f32 = slotted_1(t_3.z, curve_9, k_2, u32(2), &((*p_11)), slots_9, g_3);
    return vec3<f32>(_S352, _S353, _S354);
}

struct DiffPair_vectorx3Cfloatx2C3x3E_0
{
     primal_0 : vec3<f32>,
     differential_0 : vec3<f32>,
};

fn s_fwd_knot_0( curve_10 : u32,  k_3 : u32,  p_12 : ptr<function, RgbCurveParams_std140_0>,  slots_10 : bool,  dpg_2 : DiffPair_Gradient_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var t_4 : vec4<f32> = knots_0[curve_10 * u32(20) + k_3];
    var _S355 : DiffPair_float_0 = DiffPair_float_0( t_4.x, 0.0f );
    var _S356 : DiffPair_Gradient_0 = DiffPair_Gradient_0( dpg_2.primal_0, dpg_2.differential_0 );
    var _S357 : DiffPair_float_0 = s_fwd_slotted_1(_S355, curve_10, k_3, u32(0), &((*p_12)), slots_10, _S356);
    var _S358 : DiffPair_float_0 = DiffPair_float_0( t_4.y, 0.0f );
    var _S359 : DiffPair_float_0 = s_fwd_slotted_1(_S358, curve_10, k_3, u32(1), &((*p_12)), slots_10, _S356);
    var _S360 : DiffPair_float_0 = DiffPair_float_0( t_4.z, 0.0f );
    var _S361 : DiffPair_float_0 = s_fwd_slotted_1(_S360, curve_10, k_3, u32(2), &((*p_12)), slots_10, _S356);
    var _S362 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S357.primal_0, _S359.primal_0, _S361.primal_0), vec3<f32>(_S357.differential_0, _S359.differential_0, _S361.differential_0) );
    return _S362;
}

fn curve_at_0( point_2 : Point_0,  p_13 : ptr<function, RgbCurveParams_std140_0>,  slots_11 : bool,  g_4 : Gradient_0) -> f32
{
    if((point_2.above_0) != u32(0))
    {
        var _S363 : vec3<f32> = knot_0(point_2.curve_0, (*p_13).count_0[point_2.curve_0] - u32(1), &((*p_13)), slots_11, g_4);
        var x_last_2 : f32 = _S363.x;
        var _S364 : f32 = slotted_0((*p_13).end_value_0[point_2.curve_0], point_2.curve_0, u32(0), u32(3), &((*p_13)), slots_11, g_4);
        var _S365 : f32 = slotted_0((*p_13).end_power_0[point_2.curve_0], point_2.curve_0, u32(0), u32(4), &((*p_13)), slots_11, g_4);
        var _S366 : f32 = max(log2(max(point_2.v_0, x_last_2)) - log2(x_last_2), 0.0f);
        var limit_2 : f32 = 64.0f / max(_S366, 9.99999968265522539e-21f);
        return _S364 * exp2(clamp(_S365, - limit_2, limit_2) * _S366);
    }
    var _S367 : vec3<f32> = knot_0(point_2.curve_0, point_2.segment_0, &((*p_13)), slots_11, g_4);
    var _S368 : vec3<f32> = knot_0(point_2.curve_0, point_2.segment_0 + u32(1), &((*p_13)), slots_11, g_4);
    var _S369 : f32 = _S367.x;
    var h_2 : f32 = _S368.x - _S369;
    var t_5 : f32 = (max(point_2.v_0, _S369) - _S369) / h_2;
    var t2_2 : f32 = t_5 * t_5;
    var t3_2 : f32 = t_5 * t2_2;
    var _S370 : f32 = 3.0f * t2_2;
    return clamp((2.0f * t3_2 - _S370 + 1.0f) * _S367.y + (t3_2 - 2.0f * t2_2 + t_5) * h_2 * _S367.z + (-2.0f * t3_2 + _S370) * _S368.y + (t3_2 - t2_2) * h_2 * _S368.z, 0.0f, 1.0f);
}

fn s_fwd_curve_at_0( point_3 : Point_0,  p_14 : ptr<function, RgbCurveParams_std140_0>,  slots_12 : bool,  dpg_3 : DiffPair_Gradient_0) -> DiffPair_float_0
{
    if((point_3.above_0) != u32(0))
    {
        var _S371 : DiffPair_Gradient_0 = DiffPair_Gradient_0( dpg_3.primal_0, dpg_3.differential_0 );
        var _S372 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_knot_0(point_3.curve_0, (*p_14).count_0[point_3.curve_0] - u32(1), &((*p_14)), slots_12, _S371);
        var x_last_3 : f32 = _S372.primal_0.x;
        var s_diff_x_last_0 : f32 = _S372.differential_0.x;
        var _S373 : DiffPair_float_0 = DiffPair_float_0( (*p_14).end_value_0[point_3.curve_0], 0.0f );
        var _S374 : DiffPair_float_0 = s_fwd_slotted_0(_S373, point_3.curve_0, u32(0), u32(3), &((*p_14)), slots_12, _S371);
        var _S375 : DiffPair_float_0 = DiffPair_float_0( (*p_14).end_power_0[point_3.curve_0], 0.0f );
        var _S376 : DiffPair_float_0 = s_fwd_slotted_0(_S375, point_3.curve_0, u32(0), u32(4), &((*p_14)), slots_12, _S371);
        var _S377 : DiffPair_float_0 = DiffPair_float_0( point_3.v_0, 0.0f );
        var _S378 : DiffPair_float_0 = DiffPair_float_0( x_last_3, s_diff_x_last_0 );
        var _S379 : DiffPair_float_0 = _d_max_1(_S377, _S378);
        var _S380 : DiffPair_float_0 = DiffPair_float_0( _S379.primal_0, _S379.differential_0 );
        var _S381 : DiffPair_float_0 = _d_log2_1(_S380);
        var _S382 : DiffPair_float_0 = _d_log2_1(_S378);
        var _S383 : DiffPair_float_0 = DiffPair_float_0( _S381.primal_0 - _S382.primal_0, _S381.differential_0 - _S382.differential_0 );
        var _S384 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
        var _S385 : DiffPair_float_0 = _d_max_1(_S383, _S384);
        var _S386 : DiffPair_float_0 = DiffPair_float_0( _S385.primal_0, _S385.differential_0 );
        var _S387 : DiffPair_float_0 = DiffPair_float_0( 9.99999968265522539e-21f, 0.0f );
        var _S388 : DiffPair_float_0 = _d_max_1(_S386, _S387);
        var limit_3 : f32 = 64.0f / _S388.primal_0;
        var s_diff_limit_0 : f32 = (0.0f - 64.0f * _S388.differential_0) / (_S388.primal_0 * _S388.primal_0);
        var _S389 : DiffPair_float_0 = DiffPair_float_0( _S376.primal_0, _S376.differential_0 );
        var _S390 : DiffPair_float_0 = DiffPair_float_0( - limit_3, - s_diff_limit_0 );
        var _S391 : DiffPair_float_0 = DiffPair_float_0( limit_3, s_diff_limit_0 );
        var _S392 : DiffPair_float_0 = _d_clamp_1(_S389, _S390, _S391);
        var _S393 : DiffPair_float_0 = DiffPair_float_0( _S392.primal_0 * _S385.primal_0, _S392.differential_0 * _S385.primal_0 + _S385.differential_0 * _S392.primal_0 );
        var _S394 : DiffPair_float_0 = _d_exp2_1(_S393);
        var _S395 : DiffPair_float_0 = DiffPair_float_0( _S374.primal_0 * _S394.primal_0, _S374.differential_0 * _S394.primal_0 + _S394.differential_0 * _S374.primal_0 );
        return _S395;
    }
    var _S396 : DiffPair_Gradient_0 = DiffPair_Gradient_0( dpg_3.primal_0, dpg_3.differential_0 );
    var _S397 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_knot_0(point_3.curve_0, point_3.segment_0, &((*p_14)), slots_12, _S396);
    var _S398 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_knot_0(point_3.curve_0, point_3.segment_0 + u32(1), &((*p_14)), slots_12, _S396);
    var _S399 : f32 = _S397.primal_0.x;
    var _S400 : f32 = _S397.differential_0.x;
    var h_3 : f32 = _S398.primal_0.x - _S399;
    var s_diff_h_0 : f32 = _S398.differential_0.x - _S400;
    var _S401 : DiffPair_float_0 = DiffPair_float_0( point_3.v_0, 0.0f );
    var _S402 : DiffPair_float_0 = DiffPair_float_0( _S399, _S400 );
    var _S403 : DiffPair_float_0 = _d_max_1(_S401, _S402);
    var _S404 : f32 = _S403.primal_0 - _S399;
    var t_6 : f32 = _S404 / h_3;
    var s_diff_t_0 : f32 = ((_S403.differential_0 - _S400) * h_3 - _S404 * s_diff_h_0) / (h_3 * h_3);
    var t2_3 : f32 = t_6 * t_6;
    var _S405 : f32 = s_diff_t_0 * t_6;
    var s_diff_t2_0 : f32 = _S405 + _S405;
    var t3_3 : f32 = t_6 * t2_3;
    var s_diff_t3_0 : f32 = s_diff_t_0 * t2_3 + s_diff_t2_0 * t_6;
    var _S406 : f32 = 3.0f * t2_3;
    var _S407 : f32 = s_diff_t2_0 * 3.0f;
    var _S408 : f32 = 2.0f * t3_3 - _S406 + 1.0f;
    var _S409 : f32 = _S397.primal_0.y;
    var _S410 : f32 = t3_3 - 2.0f * t2_3 + t_6;
    var _S411 : f32 = _S410 * h_3;
    var _S412 : f32 = _S397.primal_0.z;
    var _S413 : f32 = -2.0f * t3_3 + _S406;
    var _S414 : f32 = _S398.primal_0.y;
    var _S415 : f32 = t3_3 - t2_3;
    var _S416 : f32 = _S415 * h_3;
    var _S417 : f32 = _S398.primal_0.z;
    var _S418 : DiffPair_float_0 = DiffPair_float_0( _S408 * _S409 + _S411 * _S412 + _S413 * _S414 + _S416 * _S417, (s_diff_t3_0 * 2.0f - _S407) * _S409 + _S397.differential_0.y * _S408 + (((s_diff_t3_0 - s_diff_t2_0 * 2.0f + s_diff_t_0) * h_3 + s_diff_h_0 * _S410) * _S412 + _S397.differential_0.z * _S411) + ((s_diff_t3_0 * -2.0f + _S407) * _S414 + _S398.differential_0.y * _S413) + (((s_diff_t3_0 - s_diff_t2_0) * h_3 + s_diff_h_0 * _S415) * _S417 + _S398.differential_0.z * _S416) );
    var _S419 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S420 : DiffPair_float_0 = DiffPair_float_0( 1.0f, 0.0f );
    var _S421 : DiffPair_float_0 = _d_clamp_1(_S418, _S419, _S420);
    var _S422 : DiffPair_float_0 = DiffPair_float_0( _S421.primal_0, _S421.differential_0 );
    return _S422;
}

fn rgb_curve_1( rgb_3 : vec3<f32>,  l_2 : Lookup_0,  p_15 : ptr<function, RgbCurveParams_std140_0>,  slots_13 : bool,  g_5 : Gradient_0) -> vec3<f32>
{
    if((l_2.mode_0) == u32(2))
    {
        return rgb_3;
    }
    if((l_2.mode_0) == u32(1))
    {
        var _S423 : f32 = curve_at_0(l_2.r_0, &((*p_15)), slots_13, g_5);
        return rgb_3 * vec3<f32>((_S423 / l_2.r_0.v_0));
    }
    var _S424 : f32 = curve_at_0(l_2.r_0, &((*p_15)), slots_13, g_5);
    var _S425 : f32 = curve_at_0(l_2.g_0, &((*p_15)), slots_13, g_5);
    var _S426 : f32 = curve_at_0(l_2.b_0, &((*p_15)), slots_13, g_5);
    return vec3<f32>(_S424, _S425, _S426);
}

fn s_fwd_rgb_curve_0( rgb_4 : vec3<f32>,  l_3 : Lookup_0,  p_16 : ptr<function, RgbCurveParams_std140_0>,  slots_14 : bool,  dpg_4 : DiffPair_Gradient_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    if((l_3.mode_0) == u32(2))
    {
        var _S427 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( rgb_4, vec3<f32>(0.0f) );
        return _S427;
    }
    if((l_3.mode_0) == u32(1))
    {
        var _S428 : DiffPair_Gradient_0 = DiffPair_Gradient_0( dpg_4.primal_0, dpg_4.differential_0 );
        var _S429 : DiffPair_float_0 = s_fwd_curve_at_0(l_3.r_0, &((*p_16)), slots_14, _S428);
        var _S430 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( rgb_4 * vec3<f32>((_S429.primal_0 / l_3.r_0.v_0)), vec3<f32>((_S429.differential_0 / l_3.r_0.v_0)) * rgb_4 );
        return _S430;
    }
    var _S431 : DiffPair_Gradient_0 = DiffPair_Gradient_0( dpg_4.primal_0, dpg_4.differential_0 );
    var _S432 : DiffPair_float_0 = s_fwd_curve_at_0(l_3.r_0, &((*p_16)), slots_14, _S431);
    var _S433 : DiffPair_float_0 = s_fwd_curve_at_0(l_3.g_0, &((*p_16)), slots_14, _S431);
    var _S434 : DiffPair_float_0 = s_fwd_curve_at_0(l_3.b_0, &((*p_16)), slots_14, _S431);
    var _S435 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S432.primal_0, _S433.primal_0, _S434.primal_0), vec3<f32>(_S432.differential_0, _S433.differential_0, _S434.differential_0) );
    return _S435;
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_0 : vec3<u32>)
{
    var _S436 : u32 = id_0.x;
    var _S437 : bool;
    if((_S436 >= (params_0.pass_0.width_0)))
    {
        _S437 = true;
    }
    else
    {
        _S437 = ((id_0.y) >= (params_0.pass_0.height_0));
    }
    if(_S437)
    {
        return;
    }
    var i_0 : u32 = id_0.y * params_0.pass_0.width_0 + _S436;
    var _S438 : RgbCurveParams_std140_0 = params_0.rgb_curve_0;
    var _S439 : Gradient_0 = Gradient_x24init_0(0.0f, 0.0f, 0.0f, 0.0f);
    var pixel_0 : vec4<f32> = input_0[i_0];
    var rgb_5 : vec3<f32> = pixel_0.xyz;
    var k_4 : u32 = u32(0);
    loop
    {
        if((k_4 < (params_0.pass_0.repeat_0)))
        {
        }
        else
        {
            break;
        }
        var _S440 : Lookup_0 = locate_pixel_0(rgb_5, &(_S438));
        var _S441 : vec3<f32> = rgb_curve_1(rgb_5, _S440, &(_S438), false, _S439);
        var _S442 : u32 = k_4 + u32(1);
        rgb_5 = _S441;
        k_4 = _S442;
    }
    output_0[i_0] = vec4<f32>(rgb_5, pixel_0.w);
    return;
}

fn slot_value_0( curve_11 : u32,  node_6 : u32,  kind_6 : u32,  p_17 : ptr<function, RgbCurveParams_std140_0>) -> f32
{
    if(kind_6 == u32(3))
    {
        return (*p_17).end_value_0[curve_11];
    }
    if(kind_6 == u32(4))
    {
        return (*p_17).end_power_0[curve_11];
    }
    var knot_1 : vec4<f32> = knots_0[curve_11 * u32(20) + node_6];
    var _S443 : f32;
    if(kind_6 == u32(0))
    {
        _S443 = knot_1.x;
    }
    else
    {
        if(kind_6 == u32(1))
        {
            _S443 = knot_1.y;
        }
        else
        {
            _S443 = knot_1.z;
        }
    }
    return _S443;
}

fn gradient_0( p_18 : ptr<function, RgbCurveParams_std140_0>) -> Gradient_0
{
    var g_6 : Gradient_0;
    var _S444 : vec4<u32> = (*p_18).gradient_curve_0;
    var _S445 : vec4<u32> = (*p_18).gradient_node_0;
    var _S446 : vec4<u32> = (*p_18).gradient_kind_0;
    var _S447 : f32 = slot_value_0((*p_18).gradient_curve_0.x, (*p_18).gradient_node_0.x, (*p_18).gradient_kind_0.x, &((*p_18)));
    g_6.s0_0 = _S447;
    var _S448 : f32 = slot_value_0(_S444.y, _S445.y, _S446.y, &((*p_18)));
    g_6.s1_0 = _S448;
    var _S449 : f32 = slot_value_0(_S444.z, _S445.z, _S446.z, &((*p_18)));
    g_6.s2_0 = _S449;
    var _S450 : f32 = slot_value_0(_S444.w, _S445.w, _S446.w, &((*p_18)));
    g_6.s3_0 = _S450;
    return g_6;
}

@compute
@workgroup_size(16, 16, 1)
fn rgbcurve_backward(@builtin(global_invocation_id) id_1 : vec3<u32>)
{
    var _S451 : u32 = id_1.x;
    var _S452 : bool;
    if((_S451 >= (params_0.pass_0.width_0)))
    {
        _S452 = true;
    }
    else
    {
        _S452 = ((id_1.y) >= (params_0.pass_0.height_0));
    }
    if(_S452)
    {
        return;
    }
    var i_1 : u32 = id_1.y * params_0.pass_0.width_0 + _S451;
    var _S453 : RgbCurveParams_std140_0 = params_0.rgb_curve_0;
    var rgb_6 : vec3<f32> = input_0[i_1].xyz;
    var _S454 : Gradient_0 = gradient_0(&(_S453));
    var _S455 : Gradient_0 = Gradient_x24_syn_dzero_0();
    var g_7 : DiffPair_Gradient_0;
    g_7.primal_0 = _S454;
    g_7.differential_0 = _S455;
    var _S456 : Lookup_0 = locate_pixel_0(rgb_6, &(_S453));
    _S287(rgb_6, _S456, &(_S453), true, &(g_7), derivative_in_0[i_1].xyz);
    derivative_out_0[i_1] = vec4<f32>(g_7.differential_0.s0_0, g_7.differential_0.s1_0, g_7.differential_0.s2_0, g_7.differential_0.s3_0);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn rgbcurve_forward_diff(@builtin(global_invocation_id) id_2 : vec3<u32>)
{
    var _S457 : u32 = id_2.x;
    var _S458 : bool;
    if((_S457 >= (params_0.pass_0.width_0)))
    {
        _S458 = true;
    }
    else
    {
        _S458 = ((id_2.y) >= (params_0.pass_0.height_0));
    }
    if(_S458)
    {
        return;
    }
    var i_2 : u32 = id_2.y * params_0.pass_0.width_0 + _S457;
    var _S459 : RgbCurveParams_std140_0 = params_0.rgb_curve_0;
    var rgb_7 : vec3<f32> = input_0[i_2].xyz;
    var v_2 : vec4<f32> = derivative_in_0[i_2];
    var direction_0 : Gradient_0;
    direction_0.s0_0 = v_2.x;
    direction_0.s1_0 = v_2.y;
    direction_0.s2_0 = v_2.z;
    direction_0.s3_0 = v_2.w;
    var _S460 : Lookup_0 = locate_pixel_0(rgb_7, &(_S459));
    var _S461 : Gradient_0 = gradient_0(&(_S459));
    var _S462 : DiffPair_Gradient_0 = DiffPair_Gradient_0( _S461, direction_0 );
    var _S463 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_rgb_curve_0(rgb_7, _S460, &(_S459), true, _S462);
    derivative_out_0[i_2] = vec4<f32>(_S463.differential_0, 0.0f);
    return;
}

