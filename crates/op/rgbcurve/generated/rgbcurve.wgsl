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

fn s_apply_clamp_0( x_0 : f32,  minBound_0 : f32,  maxBound_0 : f32) -> Tuple_0
{
    var _S1 : Tuple_0 = Tuple_0( clamp(x_0, minBound_0, maxBound_0) );
    return _S1;
}

struct s_bwdCallableCtx_clamp_0
{
     _S2 : f32,
     _S3 : f32,
     _S4 : f32,
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

fn s_bwdProp_clamp_0( ctx_0 : s_bwdCallableCtx_clamp_0,  d_x_0 : ptr<function, f32>,  d_minBound_0 : ptr<function, f32>,  d_maxBound_0 : ptr<function, f32>,  d_out_0 : f32)
{
    var _S10 : DiffPair_float_0;
    _S10.primal_0 = ctx_0._S2;
    var _S11 : DiffPair_float_0;
    _S11.primal_0 = ctx_0._S3;
    var _S12 : DiffPair_float_0;
    _S12.primal_0 = ctx_0._S4;
    _d_clamp_0(&(_S10), &(_S11), &(_S12), d_out_0);
    (*d_x_0) = _S10.differential_0;
    (*d_minBound_0) = _S11.differential_0;
    (*d_maxBound_0) = _S12.differential_0;
    return;
}

struct Tuple_1
{
     value0_1 : f32,
};

fn s_apply_max_0( x_1 : f32,  y_0 : f32) -> Tuple_1
{
    var _S13 : Tuple_1 = Tuple_1( max(x_1, y_0) );
    return _S13;
}

struct s_bwdCallableCtx_max_0
{
     _S14 : f32,
     _S15 : f32,
};

fn _d_max_0( dpx_1 : ptr<function, DiffPair_float_0>,  dpy_0 : ptr<function, DiffPair_float_0>,  dOut_1 : f32)
{
    var _S16 : DiffPair_float_0 = (*dpx_1);
    var _S17 : f32;
    if((((*dpx_1).primal_0) > ((*dpy_0).primal_0)))
    {
        _S17 = dOut_1;
    }
    else
    {
        if((((*dpx_1).primal_0) < ((*dpy_0).primal_0)))
        {
            _S17 = 0.0f;
        }
        else
        {
            _S17 = 0.5f * dOut_1;
        }
    }
    (*dpx_1).primal_0 = _S16.primal_0;
    (*dpx_1).differential_0 = _S17;
    var _S18 : DiffPair_float_0 = (*dpy_0);
    if((((*dpy_0).primal_0) > (_S16.primal_0)))
    {
        _S17 = dOut_1;
    }
    else
    {
        if((((*dpy_0).primal_0) < ((*dpx_1).primal_0)))
        {
            _S17 = 0.0f;
        }
        else
        {
            _S17 = 0.5f * dOut_1;
        }
    }
    (*dpy_0).primal_0 = _S18.primal_0;
    (*dpy_0).differential_0 = _S17;
    return;
}

fn s_bwdProp_max_0( ctx_1 : s_bwdCallableCtx_max_0,  d_x_1 : ptr<function, f32>,  d_y_0 : ptr<function, f32>,  d_out_1 : f32)
{
    var _S19 : DiffPair_float_0;
    _S19.primal_0 = ctx_1._S14;
    var _S20 : DiffPair_float_0;
    _S20.primal_0 = ctx_1._S15;
    _d_max_0(&(_S19), &(_S20), d_out_1);
    (*d_x_1) = _S19.differential_0;
    (*d_y_0) = _S20.differential_0;
    return;
}

struct Tuple_2
{
     value0_2 : f32,
};

fn pow0_0( x_2 : f32,  y_1 : f32) -> f32
{
    if((x_2 <= 0.0f))
    {
        return 0.0f;
    }
    return pow(x_2, y_1);
}

fn s_apply_pow0_0( x_3 : f32,  y_2 : f32) -> Tuple_2
{
    var _S21 : Tuple_2 = Tuple_2( pow0_0(x_3, y_2) );
    return _S21;
}

struct s_bwdCallableCtx_pow0_0
{
     _S22 : f32,
     _S23 : f32,
};

fn pow0_backward_0( x_4 : ptr<function, DiffPair_float_0>,  y_3 : ptr<function, DiffPair_float_0>,  d_out_2 : f32)
{
    if((((*x_4).primal_0) <= 0.0f))
    {
        (*x_4).primal_0 = (*x_4).primal_0;
        (*x_4).differential_0 = 0.0f;
        (*y_3).primal_0 = (*y_3).primal_0;
        (*y_3).differential_0 = 0.0f;
        return;
    }
    var d_x_2 : f32 = d_out_2 * (*y_3).primal_0 * pow((*x_4).primal_0, (*y_3).primal_0 - 1.0f);
    var d_y_1 : f32 = d_out_2 * pow((*x_4).primal_0, (*y_3).primal_0) * log((*x_4).primal_0);
    (*x_4).primal_0 = (*x_4).primal_0;
    (*x_4).differential_0 = d_x_2;
    (*y_3).primal_0 = (*y_3).primal_0;
    (*y_3).differential_0 = d_y_1;
    return;
}

fn s_bwdProp_pow0_0( ctx_2 : s_bwdCallableCtx_pow0_0,  d_x_3 : ptr<function, f32>,  d_y_2 : ptr<function, f32>,  d_out_3 : f32)
{
    var _S24 : DiffPair_float_0;
    _S24.primal_0 = ctx_2._S22;
    var _S25 : DiffPair_float_0;
    _S25.primal_0 = ctx_2._S23;
    pow0_backward_0(&(_S24), &(_S25), d_out_3);
    (*d_x_3) = _S24.differential_0;
    (*d_y_2) = _S25.differential_0;
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
     value0_3 : vec3<f32>,
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

struct Tuple_3
{
     value0_4 : vec3<f32>,
     value1_1 : s_minCtx_s_bwdCallableCtx_rgb_curve_0,
};

struct Tuple_4
{
     value0_5 : f32,
     value1_2 : s_minCtx_s_bwdCallableCtx_curve_at_0,
};

struct Tuple_5
{
     value0_6 : vec3<f32>,
};

struct Tuple_6
{
     value0_7 : f32,
};

fn s_apply_slotted_0( _S26 : f32,  curve_1 : u32,  node_0 : u32,  kind_0 : u32,  p_0 : ptr<function, RgbCurveParams_std140_0>,  slots_0 : bool,  _S27 : Gradient_0) -> Tuple_6
{
    var _S28 : f32;
    if(!!slots_0)
    {
        var _S29 : vec4<u32> = (*p_0).gradient_curve_0;
        var _S30 : bool;
        if(((*p_0).gradient_curve_0.x) == curve_1)
        {
            _S30 = ((*p_0).gradient_node_0.x) == node_0;
        }
        else
        {
            _S30 = false;
        }
        if(_S30)
        {
            _S30 = ((*p_0).gradient_kind_0.x) == kind_0;
        }
        else
        {
            _S30 = false;
        }
        if(_S30)
        {
            _S28 = _S27.s0_0;
        }
        else
        {
            _S28 = _S26;
        }
        if((_S29.y) == curve_1)
        {
            _S30 = ((*p_0).gradient_node_0.y) == node_0;
        }
        else
        {
            _S30 = false;
        }
        if(_S30)
        {
            _S30 = ((*p_0).gradient_kind_0.y) == kind_0;
        }
        else
        {
            _S30 = false;
        }
        if(_S30)
        {
            _S28 = _S27.s1_0;
        }
        if((_S29.z) == curve_1)
        {
            _S30 = ((*p_0).gradient_node_0.z) == node_0;
        }
        else
        {
            _S30 = false;
        }
        if(_S30)
        {
            _S30 = ((*p_0).gradient_kind_0.z) == kind_0;
        }
        else
        {
            _S30 = false;
        }
        if(_S30)
        {
            _S28 = _S27.s2_0;
        }
        if((_S29.w) == curve_1)
        {
            _S30 = ((*p_0).gradient_node_0.w) == node_0;
        }
        else
        {
            _S30 = false;
        }
        if(_S30)
        {
            _S30 = ((*p_0).gradient_kind_0.w) == kind_0;
        }
        else
        {
            _S30 = false;
        }
        if(_S30)
        {
            _S28 = _S27.s3_0;
        }
    }
    else
    {
        _S28 = _S26;
    }
    var _S31 : Tuple_6;
    _S31.value0_7 = _S28;
    return _S31;
}

fn s_apply_knot_0( curve_2 : u32,  k_0 : u32,  p_1 : ptr<function, RgbCurveParams_std140_0>,  slots_1 : bool,  _S32 : Gradient_0) -> Tuple_5
{
    var t_0 : vec4<f32> = knots_0[curve_2 * u32(20) + k_0];
    var _S33 : Tuple_5;
    _S33.value0_6 = vec3<f32>(s_apply_slotted_0(t_0.x, curve_2, k_0, u32(0), &((*p_1)), slots_1, _S32).value0_7, s_apply_slotted_0(t_0.y, curve_2, k_0, u32(1), &((*p_1)), slots_1, _S32).value0_7, s_apply_slotted_0(t_0.z, curve_2, k_0, u32(2), &((*p_1)), slots_1, _S32).value0_7);
    return _S33;
}

fn s_apply_curve_at_0( point_0 : Point_0,  p_2 : ptr<function, RgbCurveParams_std140_0>,  slots_2 : bool,  _S34 : Gradient_0) -> Tuple_4
{
    var _S35 : vec3<f32> = vec3<f32>(0.0f);
    var _S36 : s_minCtx_s_bwdCallableCtx_curve_at_0;
    _S36.value0_3 = _S35;
    _S36.value2_0 = _S35;
    _S36.value4_0 = _S35;
    var _S37 : bool = (point_0.above_0) != u32(0);
    var _S38 : f32;
    if(_S37)
    {
        var _S39 : Tuple_5 = s_apply_knot_0(point_0.curve_0, (*p_2).count_0[point_0.curve_0] - u32(1), &((*p_2)), slots_2, _S34);
        _S36.value0_3 = _S39.value0_6;
        _S38 = s_apply_slotted_0((*p_2).end_value_0[point_0.curve_0], point_0.curve_0, u32(0), u32(3), &((*p_2)), slots_2, _S34).value0_7 * s_apply_pow0_0(point_0.v_0 / _S39.value0_6.x, s_apply_slotted_0((*p_2).end_power_0[point_0.curve_0], point_0.curve_0, u32(0), u32(4), &((*p_2)), slots_2, _S34).value0_7).value0_2;
    }
    var _S40 : bool = !_S37;
    if(_S40)
    {
        var _S41 : Tuple_5 = s_apply_knot_0(point_0.curve_0, point_0.segment_0, &((*p_2)), slots_2, _S34);
        _S36.value2_0 = _S41.value0_6;
        var _S42 : Tuple_5 = s_apply_knot_0(point_0.curve_0, point_0.segment_0 + u32(1), &((*p_2)), slots_2, _S34);
        _S36.value4_0 = _S42.value0_6;
        var _S43 : f32 = _S41.value0_6.x;
        var h_0 : f32 = _S42.value0_6.x - _S43;
        var t_1 : f32 = (s_apply_max_0(point_0.v_0, _S43).value0_1 - _S43) / h_0;
        var t2_0 : f32 = t_1 * t_1;
        var t3_0 : f32 = t_1 * t2_0;
        var _S44 : f32 = 3.0f * t2_0;
        _S38 = s_apply_clamp_0((2.0f * t3_0 - _S44 + 1.0f) * _S41.value0_6.y + (t3_0 - 2.0f * t2_0 + t_1) * h_0 * _S41.value0_6.z + (-2.0f * t3_0 + _S44) * _S42.value0_6.y + (t3_0 - t2_0) * h_0 * _S42.value0_6.z, 0.0f, 1.0f).value0_0;
    }
    var _S45 : Tuple_4;
    _S45.value0_5 = _S38;
    _S45.value1_2 = _S36;
    return _S45;
}

fn s_apply_rgb_curve_0( rgb_0 : vec3<f32>,  l_0 : Lookup_0,  p_3 : ptr<function, RgbCurveParams_std140_0>,  slots_3 : bool,  _S46 : Gradient_0) -> Tuple_3
{
    var _S47 : vec3<f32> = vec3<f32>(0.0f);
    var _S48 : s_minCtx_s_bwdCallableCtx_curve_at_0 = s_minCtx_s_bwdCallableCtx_curve_at_0( _S47, _S47, _S47 );
    var _S49 : s_minCtx_s_bwdCallableCtx_rgb_curve_0;
    _S49.value1_0 = _S48;
    _S49.value2_1 = _S48;
    _S49.value3_0 = _S48;
    _S49.value4_1 = _S48;
    var _S50 : bool = !((l_0.mode_0) == u32(2));
    var _S51 : vec3<f32>;
    if(_S50)
    {
        var _runFlag_0 : bool;
        if((l_0.mode_0) == u32(1))
        {
            var _S52 : Tuple_4 = s_apply_curve_at_0(l_0.r_0, &((*p_3)), slots_3, _S46);
            _S49.value1_0 = _S52.value1_2;
            var _S53 : vec3<f32> = rgb_0 * vec3<f32>((_S52.value0_5 / l_0.r_0.v_0));
            _runFlag_0 = false;
            _S51 = _S53;
        }
        else
        {
            _runFlag_0 = _S50;
            _S51 = rgb_0;
        }
        if(_runFlag_0)
        {
            var _S54 : Tuple_4 = s_apply_curve_at_0(l_0.r_0, &((*p_3)), slots_3, _S46);
            _S49.value2_1 = _S54.value1_2;
            var _S55 : Tuple_4 = s_apply_curve_at_0(l_0.g_0, &((*p_3)), slots_3, _S46);
            _S49.value3_0 = _S55.value1_2;
            var _S56 : Tuple_4 = s_apply_curve_at_0(l_0.b_0, &((*p_3)), slots_3, _S46);
            _S49.value4_1 = _S56.value1_2;
            _S51 = vec3<f32>(_S54.value0_5, _S55.value0_5, _S56.value0_5);
        }
    }
    else
    {
        _S51 = rgb_0;
    }
    var _S57 : Tuple_3;
    _S57.value0_4 = _S51;
    _S57.value1_1 = _S49;
    return _S57;
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
     value0_8 : vec3<f32>,
     value1_3 : Lookup_0,
     value2_2 : RgbCurveParams_0,
     value3_1 : bool,
     value4_2 : Gradient_0,
};

struct s_bwdCallableCtx_rgb_curve_0
{
     value0_9 : s_paramCtx_s_bwdCallableCtx_rgb_curve_0,
     value1_4 : s_minCtx_s_bwdCallableCtx_rgb_curve_0,
};

struct s_paramCtx_s_bwdCallableCtx_curve_at_0
{
     value0_10 : Point_0,
     value1_5 : RgbCurveParams_0,
     value2_3 : bool,
     value3_2 : Gradient_0,
};

struct s_bwdCallableCtx_curve_at_0
{
     value0_11 : s_paramCtx_s_bwdCallableCtx_curve_at_0,
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
     value0_12 : u32,
     value1_7 : u32,
     value2_4 : RgbCurveParams_0,
     value3_3 : bool,
};

struct s_paramCtx_s_bwdCallableCtx_slotted_0
{
     value0_13 : u32,
     value1_8 : u32,
     value2_5 : u32,
     value3_4 : RgbCurveParams_0,
     value4_3 : bool,
};

struct s_bwdCallableCtx_slotted_0
{
     value0_14 : s_paramCtx_s_bwdCallableCtx_slotted_0,
};

struct s_bwdCallableCtx_knot_0
{
     value0_15 : s_paramCtx_s_bwdCallableCtx_knot_0,
};

fn s_apply_slotted_1( _S58 : f32,  curve_3 : u32,  node_1 : u32,  kind_1 : u32,  p_4 : RgbCurveParams_0,  slots_4 : bool,  _S59 : Gradient_0) -> Tuple_6
{
    var _S60 : f32;
    if(!!slots_4)
    {
        var _S61 : bool;
        if((p_4.gradient_curve_0.x) == curve_3)
        {
            _S61 = (p_4.gradient_node_0.x) == node_1;
        }
        else
        {
            _S61 = false;
        }
        if(_S61)
        {
            _S61 = (p_4.gradient_kind_0.x) == kind_1;
        }
        else
        {
            _S61 = false;
        }
        if(_S61)
        {
            _S60 = _S59.s0_0;
        }
        else
        {
            _S60 = _S58;
        }
        if((p_4.gradient_curve_0.y) == curve_3)
        {
            _S61 = (p_4.gradient_node_0.y) == node_1;
        }
        else
        {
            _S61 = false;
        }
        if(_S61)
        {
            _S61 = (p_4.gradient_kind_0.y) == kind_1;
        }
        else
        {
            _S61 = false;
        }
        if(_S61)
        {
            _S60 = _S59.s1_0;
        }
        if((p_4.gradient_curve_0.z) == curve_3)
        {
            _S61 = (p_4.gradient_node_0.z) == node_1;
        }
        else
        {
            _S61 = false;
        }
        if(_S61)
        {
            _S61 = (p_4.gradient_kind_0.z) == kind_1;
        }
        else
        {
            _S61 = false;
        }
        if(_S61)
        {
            _S60 = _S59.s2_0;
        }
        if((p_4.gradient_curve_0.w) == curve_3)
        {
            _S61 = (p_4.gradient_node_0.w) == node_1;
        }
        else
        {
            _S61 = false;
        }
        if(_S61)
        {
            _S61 = (p_4.gradient_kind_0.w) == kind_1;
        }
        else
        {
            _S61 = false;
        }
        if(_S61)
        {
            _S60 = _S59.s3_0;
        }
    }
    else
    {
        _S60 = _S58;
    }
    var _S62 : Tuple_6;
    _S62.value0_7 = _S60;
    return _S62;
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

fn s_bwdProp_slotted_0( _S63 : s_bwdCallableCtx_slotted_0,  _S64 : ptr<function, f32>,  _S65 : ptr<function, Gradient_0>,  _S66 : f32)
{
    var _S67 : bool = !_S63.value0_14.value4_3;
    var _S68 : bool = !_S67;
    var _S69 : bool;
    var _S70 : bool;
    var _S71 : bool;
    var _S72 : bool;
    if(_S68)
    {
        if((_S63.value0_14.value3_4.gradient_curve_0.x) == (_S63.value0_14.value0_13))
        {
            _S69 = (_S63.value0_14.value3_4.gradient_node_0.x) == (_S63.value0_14.value1_8);
        }
        else
        {
            _S69 = false;
        }
        if(_S69)
        {
            _S69 = (_S63.value0_14.value3_4.gradient_kind_0.x) == (_S63.value0_14.value2_5);
        }
        else
        {
            _S69 = false;
        }
        if((_S63.value0_14.value3_4.gradient_curve_0.y) == (_S63.value0_14.value0_13))
        {
            _S70 = (_S63.value0_14.value3_4.gradient_node_0.y) == (_S63.value0_14.value1_8);
        }
        else
        {
            _S70 = false;
        }
        if(_S70)
        {
            _S70 = (_S63.value0_14.value3_4.gradient_kind_0.y) == (_S63.value0_14.value2_5);
        }
        else
        {
            _S70 = false;
        }
        if((_S63.value0_14.value3_4.gradient_curve_0.z) == (_S63.value0_14.value0_13))
        {
            _S71 = (_S63.value0_14.value3_4.gradient_node_0.z) == (_S63.value0_14.value1_8);
        }
        else
        {
            _S71 = false;
        }
        if(_S71)
        {
            _S71 = (_S63.value0_14.value3_4.gradient_kind_0.z) == (_S63.value0_14.value2_5);
        }
        else
        {
            _S71 = false;
        }
        if((_S63.value0_14.value3_4.gradient_curve_0.w) == (_S63.value0_14.value0_13))
        {
            _S72 = (_S63.value0_14.value3_4.gradient_node_0.w) == (_S63.value0_14.value1_8);
        }
        else
        {
            _S72 = false;
        }
        if(_S72)
        {
            _S72 = (_S63.value0_14.value3_4.gradient_kind_0.w) == (_S63.value0_14.value2_5);
        }
        else
        {
            _S72 = false;
        }
        var _S73 : bool = _S69;
        var _S74 : bool = _S70;
        _S69 = _S72;
        _S70 = _S71;
        _S71 = _S74;
        _S72 = _S73;
    }
    else
    {
        _S69 = false;
        _S70 = false;
        _S71 = false;
        _S72 = false;
    }
    var _S75 : Gradient_0 = Gradient_x24_syn_dzero_0();
    var _S76 : f32;
    var _S77 : f32;
    var _S78 : Gradient_0;
    if(_S68)
    {
        if(_S69)
        {
            var _S79 : Gradient_0 = _S75;
            _S79.s3_0 = _S66;
            var _S80 : Gradient_0 = Gradient_x24_syn_dadd_0(_S75, _S79);
            _S76 = 0.0f;
            _S78 = _S80;
        }
        else
        {
            _S76 = _S66;
            _S78 = _S75;
        }
        if(_S70)
        {
            var _S81 : Gradient_0 = _S75;
            _S81.s2_0 = _S76;
            var _S82 : Gradient_0 = Gradient_x24_syn_dadd_0(_S78, _S81);
            _S76 = 0.0f;
            _S78 = _S82;
        }
        if(_S71)
        {
            var _S83 : Gradient_0 = _S75;
            _S83.s1_0 = _S76;
            var _S84 : Gradient_0 = Gradient_x24_syn_dadd_0(_S78, _S83);
            _S76 = 0.0f;
            _S78 = _S84;
        }
        if(_S72)
        {
            var _S85 : Gradient_0 = _S75;
            _S85.s0_0 = _S76;
            var _S86 : Gradient_0 = Gradient_x24_syn_dadd_0(_S78, _S85);
            _S76 = 0.0f;
            _S78 = _S86;
        }
        var _S87 : f32 = _S76;
        _S76 = 0.0f;
        _S77 = _S87;
    }
    else
    {
        _S76 = _S66;
        _S77 = 0.0f;
        _S78 = _S75;
    }
    if(_S67)
    {
        _S76 = _S76 + _S77;
    }
    else
    {
        _S76 = _S77;
    }
    (*_S64) = _S76;
    (*_S65) = _S78;
    return;
}

fn s_bwdProp_knot_0( _S88 : s_bwdCallableCtx_knot_0,  _S89 : ptr<function, Gradient_0>,  _S90 : vec3<f32>)
{
    var _S91 : s_paramCtx_s_bwdCallableCtx_slotted_0;
    _S91.value0_13 = _S88.value0_15.value0_12;
    _S91.value1_8 = _S88.value0_15.value1_7;
    _S91.value2_5 = u32(0);
    _S91.value3_4 = _S88.value0_15.value2_4;
    _S91.value4_3 = _S88.value0_15.value3_3;
    var _S92 : s_bwdCallableCtx_slotted_0 = s_bwdCallableCtx_slotted_0( _S91 );
    var _S93 : s_paramCtx_s_bwdCallableCtx_slotted_0;
    _S93.value0_13 = _S88.value0_15.value0_12;
    _S93.value1_8 = _S88.value0_15.value1_7;
    _S93.value2_5 = u32(1);
    _S93.value3_4 = _S88.value0_15.value2_4;
    _S93.value4_3 = _S88.value0_15.value3_3;
    var _S94 : s_bwdCallableCtx_slotted_0 = s_bwdCallableCtx_slotted_0( _S93 );
    var _S95 : s_paramCtx_s_bwdCallableCtx_slotted_0;
    _S95.value0_13 = _S88.value0_15.value0_12;
    _S95.value1_8 = _S88.value0_15.value1_7;
    _S95.value2_5 = u32(2);
    _S95.value3_4 = _S88.value0_15.value2_4;
    _S95.value4_3 = _S88.value0_15.value3_3;
    var _S96 : s_bwdCallableCtx_slotted_0 = s_bwdCallableCtx_slotted_0( _S95 );
    var _S97 : Gradient_0 = Gradient_x24_syn_dzero_0();
    var _S98 : f32 = 0.0f;
    var _S99 : Gradient_0;
    _S99.s0_0 = 0.0f;
    _S99.s1_0 = 0.0f;
    _S99.s2_0 = 0.0f;
    _S99.s3_0 = 0.0f;
    s_bwdProp_slotted_0(_S96, &(_S98), &(_S99), _S90[i32(2)]);
    var _S100 : f32 = 0.0f;
    var _S101 : Gradient_0;
    _S101.s0_0 = 0.0f;
    _S101.s1_0 = 0.0f;
    _S101.s2_0 = 0.0f;
    _S101.s3_0 = 0.0f;
    s_bwdProp_slotted_0(_S94, &(_S100), &(_S101), _S90[i32(1)]);
    var _S102 : f32 = 0.0f;
    var _S103 : Gradient_0;
    _S103.s0_0 = 0.0f;
    _S103.s1_0 = 0.0f;
    _S103.s2_0 = 0.0f;
    _S103.s3_0 = 0.0f;
    s_bwdProp_slotted_0(_S92, &(_S102), &(_S103), _S90[i32(0)]);
    (*_S89) = Gradient_x24_syn_dadd_0(Gradient_x24_syn_dadd_0(Gradient_x24_syn_dadd_0(_S99, _S101), _S103), _S97);
    return;
}

fn s_bwdProp_curve_at_0( _S104 : s_bwdCallableCtx_curve_at_0,  _S105 : ptr<function, Gradient_0>,  _S106 : f32)
{
    var _S107 : vec4<f32> = vec4<f32>(0.0f);
    var _S108 : vec4<u32> = vec4<u32>(u32(0));
    var _S109 : RgbCurveParams_0 = RgbCurveParams_0( _S107, _S108, _S107, _S107, u32(0), u32(0), u32(0), u32(0), _S108, _S108, _S108 );
    var _S110 : s_paramCtx_s_bwdCallableCtx_knot_0 = s_paramCtx_s_bwdCallableCtx_knot_0( u32(0), u32(0), _S109, false );
    var _S111 : s_paramCtx_s_bwdCallableCtx_slotted_0 = s_paramCtx_s_bwdCallableCtx_slotted_0( u32(0), u32(0), u32(0), _S109, false );
    var _S112 : bool = (_S104.value0_11.value0_10.above_0) != u32(0);
    var _S113 : f32;
    var _S114 : f32;
    var _S115 : f32;
    var _S116 : f32;
    var _S117 : s_bwdCallableCtx_pow0_0;
    var _S118 : s_bwdCallableCtx_slotted_0;
    var _S119 : s_bwdCallableCtx_slotted_0;
    var _S120 : s_bwdCallableCtx_knot_0;
    if(_S112)
    {
        var _S121 : u32 = _S104.value0_11.value1_5.count_0[_S104.value0_11.value0_10.curve_0] - u32(1);
        var _S122 : s_paramCtx_s_bwdCallableCtx_knot_0;
        _S122.value0_12 = _S104.value0_11.value0_10.curve_0;
        _S122.value1_7 = _S121;
        _S122.value2_4 = _S104.value0_11.value1_5;
        _S122.value3_3 = _S104.value0_11.value2_3;
        var x_last_0 : f32 = _S104.value1_6.value0_3.x;
        var _S123 : Tuple_6 = s_apply_slotted_1(_S104.value0_11.value1_5.end_value_0[_S104.value0_11.value0_10.curve_0], _S104.value0_11.value0_10.curve_0, u32(0), u32(3), _S104.value0_11.value1_5, _S104.value0_11.value2_3, _S104.value0_11.value3_2);
        var _S124 : s_paramCtx_s_bwdCallableCtx_slotted_0;
        _S124.value0_13 = _S104.value0_11.value0_10.curve_0;
        _S124.value1_8 = u32(0);
        _S124.value2_5 = u32(3);
        _S124.value3_4 = _S104.value0_11.value1_5;
        _S124.value4_3 = _S104.value0_11.value2_3;
        var _S125 : Tuple_6 = s_apply_slotted_1(_S104.value0_11.value1_5.end_power_0[_S104.value0_11.value0_10.curve_0], _S104.value0_11.value0_10.curve_0, u32(0), u32(4), _S104.value0_11.value1_5, _S104.value0_11.value2_3, _S104.value0_11.value3_2);
        var _S126 : s_paramCtx_s_bwdCallableCtx_slotted_0;
        _S126.value0_13 = _S104.value0_11.value0_10.curve_0;
        _S126.value1_8 = u32(0);
        _S126.value2_5 = u32(4);
        _S126.value3_4 = _S104.value0_11.value1_5;
        _S126.value4_3 = _S104.value0_11.value2_3;
        var _S127 : f32 = _S104.value0_11.value0_10.v_0 / x_last_0;
        var _S128 : f32 = x_last_0 * x_last_0;
        var _S129 : Tuple_2 = s_apply_pow0_0(_S127, _S125.value0_7);
        var _S130 : s_bwdCallableCtx_pow0_0;
        _S130._S22 = _S127;
        _S130._S23 = _S125.value0_7;
        _S113 = _S123.value0_7;
        _S114 = _S129.value0_2;
        _S117 = _S130;
        _S115 = _S128;
        _S116 = _S104.value0_11.value0_10.v_0;
        _S118.value0_14 = _S126;
        _S119.value0_14 = _S124;
        _S120.value0_15 = _S122;
    }
    else
    {
        _S113 = 0.0f;
        _S114 = 0.0f;
        _S117._S22 = 0.0f;
        _S117._S23 = 0.0f;
        _S115 = 0.0f;
        _S116 = 0.0f;
        _S118.value0_14 = _S111;
        _S119.value0_14 = _S111;
        _S120.value0_15 = _S110;
    }
    var _S131 : bool = !_S112;
    var _S132 : f32;
    var _S133 : f32;
    var _S134 : f32;
    var _S135 : f32;
    var _S136 : f32;
    var _S137 : f32;
    var _S138 : f32;
    var _S139 : f32;
    var _S140 : f32;
    var _S141 : f32;
    var _S142 : f32;
    var _S143 : f32;
    var _S144 : f32;
    var _S145 : f32;
    var _S146 : f32;
    var _S147 : s_bwdCallableCtx_knot_0;
    var _S148 : s_bwdCallableCtx_knot_0;
    var _S149 : s_bwdCallableCtx_clamp_0;
    var _S150 : s_bwdCallableCtx_max_0;
    if(_S131)
    {
        var _S151 : s_paramCtx_s_bwdCallableCtx_knot_0;
        _S151.value0_12 = _S104.value0_11.value0_10.curve_0;
        _S151.value1_7 = _S104.value0_11.value0_10.segment_0;
        _S151.value2_4 = _S104.value0_11.value1_5;
        _S151.value3_3 = _S104.value0_11.value2_3;
        var _S152 : u32 = _S104.value0_11.value0_10.segment_0 + u32(1);
        var _S153 : s_paramCtx_s_bwdCallableCtx_knot_0;
        _S153.value0_12 = _S104.value0_11.value0_10.curve_0;
        _S153.value1_7 = _S152;
        _S153.value2_4 = _S104.value0_11.value1_5;
        _S153.value3_3 = _S104.value0_11.value2_3;
        var _S154 : f32 = _S104.value1_6.value2_0.x;
        var h_1 : f32 = _S104.value1_6.value4_0.x - _S154;
        var _S155 : Tuple_1 = s_apply_max_0(_S104.value0_11.value0_10.v_0, _S154);
        var _S156 : s_bwdCallableCtx_max_0;
        _S156._S14 = _S104.value0_11.value0_10.v_0;
        _S156._S15 = _S154;
        var _S157 : f32 = _S155.value0_1 - _S154;
        var t_2 : f32 = _S157 / h_1;
        var _S158 : f32 = h_1 * h_1;
        var t2_1 : f32 = t_2 * t_2;
        var t3_1 : f32 = t_2 * t2_1;
        var _S159 : f32 = 3.0f * t2_1;
        var _S160 : f32 = 2.0f * t3_1 - _S159 + 1.0f;
        var _S161 : f32 = _S104.value1_6.value2_0.y;
        var _S162 : f32 = t3_1 - 2.0f * t2_1 + t_2;
        var _S163 : f32 = _S162 * h_1;
        var _S164 : f32 = _S104.value1_6.value2_0.z;
        var _S165 : f32 = -2.0f * t3_1 + _S159;
        var _S166 : f32 = _S104.value1_6.value4_0.y;
        var _S167 : f32 = t3_1 - t2_1;
        var _S168 : f32 = _S167 * h_1;
        var _S169 : f32 = _S104.value1_6.value4_0.z;
        var _S170 : s_bwdCallableCtx_clamp_0;
        _S170._S2 = _S160 * _S161 + _S163 * _S164 + _S165 * _S166 + _S168 * _S169;
        _S170._S3 = 0.0f;
        _S170._S4 = 1.0f;
        _S149 = _S170;
        _S132 = _S168;
        _S133 = _S169;
        _S134 = _S167;
        _S135 = h_1;
        _S136 = _S165;
        _S137 = _S166;
        _S138 = _S163;
        _S139 = _S164;
        _S140 = _S162;
        _S141 = _S160;
        _S142 = _S161;
        _S143 = t_2;
        _S144 = t2_1;
        _S145 = _S158;
        _S146 = _S157;
        _S150 = _S156;
        _S147.value0_15 = _S153;
        _S148.value0_15 = _S151;
    }
    else
    {
        _S149._S2 = 0.0f;
        _S149._S3 = 0.0f;
        _S149._S4 = 0.0f;
        _S132 = 0.0f;
        _S133 = 0.0f;
        _S134 = 0.0f;
        _S135 = 0.0f;
        _S136 = 0.0f;
        _S137 = 0.0f;
        _S138 = 0.0f;
        _S139 = 0.0f;
        _S140 = 0.0f;
        _S141 = 0.0f;
        _S142 = 0.0f;
        _S143 = 0.0f;
        _S144 = 0.0f;
        _S145 = 0.0f;
        _S146 = 0.0f;
        _S150._S14 = 0.0f;
        _S150._S15 = 0.0f;
        _S147.value0_15 = _S110;
        _S148.value0_15 = _S110;
    }
    var _S171 : Gradient_0 = Gradient_x24_syn_dzero_0();
    var _S172 : Gradient_0;
    if(_S131)
    {
        var _S173 : f32 = 0.0f;
        var _S174 : f32 = 0.0f;
        var _S175 : f32 = 0.0f;
        s_bwdProp_clamp_0(_S149, &(_S173), &(_S174), &(_S175), _S106);
        var _S176 : f32 = _S132 * _S173;
        var _S177 : f32 = _S133 * _S173;
        var _S178 : f32 = _S134 * _S177;
        var _S179 : f32 = _S135 * _S177;
        var _S180 : f32 = _S136 * _S173;
        var _S181 : f32 = _S137 * _S173;
        var _S182 : f32 = _S138 * _S173;
        var _S183 : f32 = _S139 * _S173;
        var _S184 : f32 = _S140 * _S183;
        var _S185 : f32 = _S135 * _S183;
        var _S186 : f32 = _S141 * _S173;
        var _S187 : f32 = _S142 * _S173;
        var _S188 : f32 = _S179 + -2.0f * _S181 + _S185 + 2.0f * _S187;
        var _S189 : f32 = _S143 * (- _S179 + 2.0f * - _S185 + 3.0f * (_S181 + - _S187) + _S143 * _S188);
        var _S190 : f32 = (_S185 + _S144 * _S188 + _S189 + _S189) / _S145;
        var _S191 : f32 = _S146 * - _S190;
        var _S192 : f32 = _S135 * _S190;
        var _S193 : f32 = - _S192;
        var _S194 : f32 = 0.0f;
        var _S195 : f32 = 0.0f;
        s_bwdProp_max_0(_S150, &(_S194), &(_S195), _S192);
        var _S196 : f32 = _S178 + _S184 + _S191;
        var _S197 : f32 = _S193 + _S195 + - _S196;
        var _S198 : vec3<f32> = vec3<f32>(_S196, _S180, _S176);
        var _S199 : Gradient_0;
        _S199.s0_0 = 0.0f;
        _S199.s1_0 = 0.0f;
        _S199.s2_0 = 0.0f;
        _S199.s3_0 = 0.0f;
        s_bwdProp_knot_0(_S147, &(_S199), _S198);
        var _S200 : vec3<f32> = vec3<f32>(_S197, _S186, _S182);
        var _S201 : Gradient_0;
        _S201.s0_0 = 0.0f;
        _S201.s1_0 = 0.0f;
        _S201.s2_0 = 0.0f;
        _S201.s3_0 = 0.0f;
        s_bwdProp_knot_0(_S148, &(_S201), _S200);
        var _S202 : Gradient_0 = Gradient_x24_syn_dadd_0(Gradient_x24_syn_dadd_0(_S199, _S201), _S171);
        _S132 = 0.0f;
        _S172 = _S202;
    }
    else
    {
        _S132 = _S106;
        _S172 = _S171;
    }
    if(_S112)
    {
        var _S203 : f32 = _S113 * _S132;
        var _S204 : f32 = _S114 * _S132;
        var _S205 : f32 = 0.0f;
        var _S206 : f32 = 0.0f;
        s_bwdProp_pow0_0(_S117, &(_S205), &(_S206), _S203);
        var s_diff_x_last_T_0 : f32 = _S116 * - (_S205 / _S115);
        var _S207 : f32 = 0.0f;
        var _S208 : Gradient_0;
        _S208.s0_0 = 0.0f;
        _S208.s1_0 = 0.0f;
        _S208.s2_0 = 0.0f;
        _S208.s3_0 = 0.0f;
        s_bwdProp_slotted_0(_S118, &(_S207), &(_S208), _S206);
        var _S209 : f32 = 0.0f;
        var _S210 : Gradient_0;
        _S210.s0_0 = 0.0f;
        _S210.s1_0 = 0.0f;
        _S210.s2_0 = 0.0f;
        _S210.s3_0 = 0.0f;
        s_bwdProp_slotted_0(_S119, &(_S209), &(_S210), _S204);
        var _S211 : vec3<f32> = vec3<f32>(s_diff_x_last_T_0, 0.0f, 0.0f);
        var _S212 : Gradient_0;
        _S212.s0_0 = 0.0f;
        _S212.s1_0 = 0.0f;
        _S212.s2_0 = 0.0f;
        _S212.s3_0 = 0.0f;
        s_bwdProp_knot_0(_S120, &(_S212), _S211);
        _S172 = Gradient_x24_syn_dadd_0(Gradient_x24_syn_dadd_0(Gradient_x24_syn_dadd_0(_S208, _S210), _S212), _S172);
    }
    (*_S105) = _S172;
    return;
}

fn s_bwdProp_rgb_curve_0( _S213 : s_bwdCallableCtx_rgb_curve_0,  _S214 : ptr<function, Gradient_0>,  _S215 : vec3<f32>)
{
    var _S216 : Point_0 = Point_0( u32(0), u32(0), u32(0), 0.0f );
    var _S217 : vec4<f32> = vec4<f32>(0.0f);
    var _S218 : vec4<u32> = vec4<u32>(u32(0));
    var _S219 : RgbCurveParams_0 = RgbCurveParams_0( _S217, _S218, _S217, _S217, u32(0), u32(0), u32(0), u32(0), _S218, _S218, _S218 );
    var _S220 : Gradient_0 = Gradient_0( 0.0f, 0.0f, 0.0f, 0.0f );
    var _S221 : s_paramCtx_s_bwdCallableCtx_curve_at_0 = s_paramCtx_s_bwdCallableCtx_curve_at_0( _S216, _S219, false, _S220 );
    var _S222 : vec3<f32> = vec3<f32>(0.0f);
    var _S223 : s_minCtx_s_bwdCallableCtx_curve_at_0 = s_minCtx_s_bwdCallableCtx_curve_at_0( _S222, _S222, _S222 );
    var _S224 : bool = !((_S213.value0_9.value1_3.mode_0) == u32(2));
    var _runFlag_1 : bool;
    var _S225 : bool;
    var _S226 : f32;
    var _S227 : s_bwdCallableCtx_curve_at_0;
    var _S228 : s_bwdCallableCtx_curve_at_0;
    var _S229 : s_bwdCallableCtx_curve_at_0;
    var _S230 : s_bwdCallableCtx_curve_at_0;
    if(_S224)
    {
        var _S231 : bool = (_S213.value0_9.value1_3.mode_0) == u32(1);
        if(_S231)
        {
            var _S232 : s_paramCtx_s_bwdCallableCtx_curve_at_0;
            _S232.value0_10 = _S213.value0_9.value1_3.r_0;
            _S232.value1_5 = _S213.value0_9.value2_2;
            _S232.value2_3 = _S213.value0_9.value3_1;
            _S232.value3_2 = _S213.value0_9.value4_2;
            _runFlag_1 = false;
            _S226 = _S213.value0_9.value1_3.r_0.v_0;
            _S227.value0_11 = _S232;
            _S227.value1_6 = _S213.value1_4.value1_0;
        }
        else
        {
            _runFlag_1 = _S224;
            _S226 = 0.0f;
            _S227.value0_11 = _S221;
            _S227.value1_6 = _S223;
        }
        if(_runFlag_1)
        {
            var _S233 : s_paramCtx_s_bwdCallableCtx_curve_at_0;
            _S233.value0_10 = _S213.value0_9.value1_3.r_0;
            _S233.value1_5 = _S213.value0_9.value2_2;
            _S233.value2_3 = _S213.value0_9.value3_1;
            _S233.value3_2 = _S213.value0_9.value4_2;
            var _S234 : s_paramCtx_s_bwdCallableCtx_curve_at_0;
            _S234.value0_10 = _S213.value0_9.value1_3.g_0;
            _S234.value1_5 = _S213.value0_9.value2_2;
            _S234.value2_3 = _S213.value0_9.value3_1;
            _S234.value3_2 = _S213.value0_9.value4_2;
            var _S235 : s_paramCtx_s_bwdCallableCtx_curve_at_0;
            _S235.value0_10 = _S213.value0_9.value1_3.b_0;
            _S235.value1_5 = _S213.value0_9.value2_2;
            _S235.value2_3 = _S213.value0_9.value3_1;
            _S235.value3_2 = _S213.value0_9.value4_2;
            _S228.value0_11 = _S235;
            _S228.value1_6 = _S213.value1_4.value4_1;
            _S229.value0_11 = _S234;
            _S229.value1_6 = _S213.value1_4.value3_0;
            _S230.value0_11 = _S233;
            _S230.value1_6 = _S213.value1_4.value2_1;
        }
        else
        {
            _S228.value0_11 = _S221;
            _S228.value1_6 = _S223;
            _S229.value0_11 = _S221;
            _S229.value1_6 = _S223;
            _S230.value0_11 = _S221;
            _S230.value1_6 = _S223;
        }
        var _S236 : s_bwdCallableCtx_curve_at_0 = _S227;
        _S227 = _S228;
        _S228 = _S229;
        _S229 = _S230;
        _S225 = _S231;
        _S230 = _S236;
    }
    else
    {
        _runFlag_1 = false;
        _S227.value0_11 = _S221;
        _S227.value1_6 = _S223;
        _S228.value0_11 = _S221;
        _S228.value1_6 = _S223;
        _S229.value0_11 = _S221;
        _S229.value1_6 = _S223;
        _S225 = false;
        _S226 = 0.0f;
        _S230.value0_11 = _S221;
        _S230.value1_6 = _S223;
    }
    var _S237 : Gradient_0 = Gradient_x24_syn_dzero_0();
    var _S238 : Gradient_0;
    if(_S224)
    {
        var _S239 : vec3<f32>;
        if(_runFlag_1)
        {
            var _S240 : Gradient_0;
            _S240.s0_0 = 0.0f;
            _S240.s1_0 = 0.0f;
            _S240.s2_0 = 0.0f;
            _S240.s3_0 = 0.0f;
            s_bwdProp_curve_at_0(_S227, &(_S240), _S215[i32(2)]);
            var _S241 : Gradient_0;
            _S241.s0_0 = 0.0f;
            _S241.s1_0 = 0.0f;
            _S241.s2_0 = 0.0f;
            _S241.s3_0 = 0.0f;
            s_bwdProp_curve_at_0(_S228, &(_S241), _S215[i32(1)]);
            var _S242 : Gradient_0;
            _S242.s0_0 = 0.0f;
            _S242.s1_0 = 0.0f;
            _S242.s2_0 = 0.0f;
            _S242.s3_0 = 0.0f;
            s_bwdProp_curve_at_0(_S229, &(_S242), _S215[i32(0)]);
            var _S243 : Gradient_0 = Gradient_x24_syn_dadd_0(Gradient_x24_syn_dadd_0(Gradient_x24_syn_dadd_0(_S240, _S241), _S242), _S237);
            _S239 = _S222;
            _S238 = _S243;
        }
        else
        {
            _S239 = _S215;
            _S238 = _S237;
        }
        if(_S225)
        {
            var _S244 : vec3<f32> = _S213.value0_9.value0_8 * _S239;
            var _S245 : f32 = (_S244[i32(0)] + _S244[i32(1)] + _S244[i32(2)]) / _S226;
            var _S246 : Gradient_0;
            _S246.s0_0 = 0.0f;
            _S246.s1_0 = 0.0f;
            _S246.s2_0 = 0.0f;
            _S246.s3_0 = 0.0f;
            s_bwdProp_curve_at_0(_S230, &(_S246), _S245);
            _S238 = Gradient_x24_syn_dadd_0(_S246, _S238);
        }
    }
    else
    {
        _S238 = _S237;
    }
    (*_S214) = _S238;
    return;
}

fn _S247( _S248 : vec3<f32>,  _S249 : Lookup_0,  _S250 : ptr<function, RgbCurveParams_std140_0>,  _S251 : bool,  _S252 : ptr<function, DiffPair_Gradient_0>,  _S253 : vec3<f32>)
{
    var _S254 : vec4<f32> = (*_S250).luminance_0;
    var _S255 : vec4<u32> = (*_S250).count_0;
    var _S256 : vec4<f32> = (*_S250).end_value_0;
    var _S257 : vec4<f32> = (*_S250).end_power_0;
    var _S258 : u32 = (*_S250).autoscale_0;
    var _S259 : u32 = (*_S250).preserve_colors_0;
    var _S260 : u32 = (*_S250).pad0_0;
    var _S261 : u32 = (*_S250).pad1_0;
    var _S262 : vec4<u32> = (*_S250).gradient_curve_0;
    var _S263 : vec4<u32> = (*_S250).gradient_node_0;
    var _S264 : vec4<u32> = (*_S250).gradient_kind_0;
    var _S265 : Gradient_0 = (*_S252).primal_0;
    var _S266 : Tuple_3 = s_apply_rgb_curve_0(_S248, _S249, &((*_S250)), _S251, (*_S252).primal_0);
    var _S267 : s_paramCtx_s_bwdCallableCtx_rgb_curve_0;
    _S267.value0_8 = _S248;
    _S267.value1_3 = _S249;
    _S267.value2_2.luminance_0 = _S254;
    _S267.value2_2.count_0 = _S255;
    _S267.value2_2.end_value_0 = _S256;
    _S267.value2_2.end_power_0 = _S257;
    _S267.value2_2.autoscale_0 = _S258;
    _S267.value2_2.preserve_colors_0 = _S259;
    _S267.value2_2.pad0_0 = _S260;
    _S267.value2_2.pad1_0 = _S261;
    _S267.value2_2.gradient_curve_0 = _S262;
    _S267.value2_2.gradient_node_0 = _S263;
    _S267.value2_2.gradient_kind_0 = _S264;
    _S267.value3_1 = _S251;
    _S267.value4_2 = _S265;
    var _S268 : s_bwdCallableCtx_rgb_curve_0 = s_bwdCallableCtx_rgb_curve_0( _S267, _S266.value1_1 );
    var _S269 : Gradient_0 = (*_S252).differential_0;
    s_bwdProp_rgb_curve_0(_S268, &(_S269), _S253);
    (*_S252).differential_0 = _S269;
    return;
}

fn _d_clamp_1( dpx_2 : DiffPair_float_0,  dpMin_1 : DiffPair_float_0,  dpMax_1 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S270 : f32 = clamp(dpx_2.primal_0, dpMin_1.primal_0, dpMax_1.primal_0);
    var _S271 : bool;
    if(((dpx_2.primal_0) >= (dpMin_1.primal_0)))
    {
        _S271 = ((dpx_2.primal_0) <= (dpMax_1.primal_0));
    }
    else
    {
        _S271 = false;
    }
    var _S272 : f32;
    if(_S271)
    {
        _S272 = dpx_2.differential_0;
    }
    else
    {
        if(((dpx_2.primal_0) < (dpMin_1.primal_0)))
        {
            _S272 = dpMin_1.differential_0;
        }
        else
        {
            _S272 = dpMax_1.differential_0;
        }
    }
    var _S273 : DiffPair_float_0 = DiffPair_float_0( _S270, _S272 );
    return _S273;
}

fn _d_max_1( dpx_3 : DiffPair_float_0,  dpy_1 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S274 : f32 = max(dpx_3.primal_0, dpy_1.primal_0);
    var _S275 : f32;
    if(((dpx_3.primal_0) > (dpy_1.primal_0)))
    {
        _S275 = dpx_3.differential_0;
    }
    else
    {
        if(((dpx_3.primal_0) < (dpy_1.primal_0)))
        {
            _S275 = dpy_1.differential_0;
        }
        else
        {
            _S275 = 0.5f * (dpx_3.differential_0 + dpy_1.differential_0);
        }
    }
    var _S276 : DiffPair_float_0 = DiffPair_float_0( _S274, _S275 );
    return _S276;
}

fn Gradient_x24init_0( s0_1 : f32,  s1_1 : f32,  s2_1 : f32,  s3_1 : f32) -> Gradient_0
{
    var _S277 : Gradient_0;
    _S277.s0_0 = s0_1;
    _S277.s1_0 = s1_1;
    _S277.s2_0 = s2_1;
    _S277.s3_0 = s3_1;
    return _S277;
}

fn locate_0( curve_4 : u32,  v_1 : f32,  p_5 : ptr<function, RgbCurveParams_std140_0>) -> Point_0
{
    var segment_1 : u32;
    var _S278 : vec4<u32> = (*p_5).count_0;
    var base_0 : u32 = curve_4 * u32(20);
    var _S279 : u32 = (*p_5).count_0[curve_4] - u32(2);
    var k_1 : u32 = u32(0);
    loop
    {
        if(((k_1 + u32(2)) < (_S278[curve_4])))
        {
        }
        else
        {
            segment_1 = _S279;
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
    var _S280 : i32;
    if((v_1 >= (knots_0[base_0 + _S278[curve_4] - u32(1)].x)))
    {
        _S280 = i32(1);
    }
    else
    {
        _S280 = i32(0);
    }
    point_1.above_0 = u32(_S280);
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
        var _S281 : f32;
        if((sum_0 > 0.0f))
        {
            _S281 = dot(rgb_1, squared_0) / sum_0;
        }
        else
        {
            _S281 = 0.0f;
        }
        return _S281;
    }
    return (rgb_1.x + rgb_1.y + rgb_1.z) / 3.0f;
}

fn locate_pixel_0( rgb_2 : vec3<f32>,  p_6 : ptr<function, RgbCurveParams_std140_0>) -> Lookup_0
{
    var l_1 : Lookup_0;
    if(((*p_6).autoscale_0) == u32(1))
    {
        l_1.mode_0 = u32(0);
        var _S282 : Point_0 = locate_0(u32(0), rgb_2.x, &((*p_6)));
        l_1.r_0 = _S282;
        var _S283 : Point_0 = locate_0(u32(1), rgb_2.y, &((*p_6)));
        l_1.g_0 = _S283;
        var _S284 : Point_0 = locate_0(u32(2), rgb_2.z, &((*p_6)));
        l_1.b_0 = _S284;
    }
    else
    {
        var _S285 : u32 = (*p_6).preserve_colors_0;
        if(((*p_6).preserve_colors_0) == u32(0))
        {
            l_1.mode_0 = u32(0);
            var _S286 : Point_0 = locate_0(u32(0), rgb_2.x, &((*p_6)));
            l_1.r_0 = _S286;
            var _S287 : Point_0 = locate_0(u32(0), rgb_2.y, &((*p_6)));
            l_1.g_0 = _S287;
            var _S288 : Point_0 = locate_0(u32(0), rgb_2.z, &((*p_6)));
            l_1.b_0 = _S288;
        }
        else
        {
            var norm_1 : f32 = rgb_norm_0(rgb_2, _S285, (*p_6).luminance_0.xyz);
            var _S289 : u32;
            if((norm_1 > 0.0f))
            {
                _S289 = u32(1);
            }
            else
            {
                _S289 = u32(2);
            }
            l_1.mode_0 = _S289;
            var _S290 : Point_0 = locate_0(u32(0), norm_1, &((*p_6)));
            l_1.r_0 = _S290;
            l_1.g_0 = _S290;
            l_1.b_0 = _S290;
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
    var _S291 : vec4<u32> = (*p_7).gradient_curve_0;
    var _S292 : bool;
    if(((*p_7).gradient_curve_0.x) == curve_5)
    {
        _S292 = ((*p_7).gradient_node_0.x) == node_2;
    }
    else
    {
        _S292 = false;
    }
    if(_S292)
    {
        _S292 = ((*p_7).gradient_kind_0.x) == kind_2;
    }
    else
    {
        _S292 = false;
    }
    var _S293 : f32;
    if(_S292)
    {
        _S293 = g_1.s0_0;
    }
    else
    {
        _S293 = value_0;
    }
    if((_S291.y) == curve_5)
    {
        _S292 = ((*p_7).gradient_node_0.y) == node_2;
    }
    else
    {
        _S292 = false;
    }
    if(_S292)
    {
        _S292 = ((*p_7).gradient_kind_0.y) == kind_2;
    }
    else
    {
        _S292 = false;
    }
    if(_S292)
    {
        _S293 = g_1.s1_0;
    }
    if((_S291.z) == curve_5)
    {
        _S292 = ((*p_7).gradient_node_0.z) == node_2;
    }
    else
    {
        _S292 = false;
    }
    if(_S292)
    {
        _S292 = ((*p_7).gradient_kind_0.z) == kind_2;
    }
    else
    {
        _S292 = false;
    }
    if(_S292)
    {
        _S293 = g_1.s2_0;
    }
    if((_S291.w) == curve_5)
    {
        _S292 = ((*p_7).gradient_node_0.w) == node_2;
    }
    else
    {
        _S292 = false;
    }
    if(_S292)
    {
        _S292 = ((*p_7).gradient_kind_0.w) == kind_2;
    }
    else
    {
        _S292 = false;
    }
    if(_S292)
    {
        _S293 = g_1.s3_0;
    }
    return _S293;
}

fn slotted_1( value_1 : f32,  curve_6 : u32,  node_3 : u32,  kind_3 : u32,  p_8 : ptr<function, RgbCurveParams_std140_0>,  slots_6 : bool,  g_2 : Gradient_0) -> f32
{
    if(!slots_6)
    {
        return value_1;
    }
    var _S294 : vec4<u32> = (*p_8).gradient_curve_0;
    var _S295 : bool;
    if(((*p_8).gradient_curve_0.x) == curve_6)
    {
        _S295 = ((*p_8).gradient_node_0.x) == node_3;
    }
    else
    {
        _S295 = false;
    }
    if(_S295)
    {
        _S295 = ((*p_8).gradient_kind_0.x) == kind_3;
    }
    else
    {
        _S295 = false;
    }
    var _S296 : f32;
    if(_S295)
    {
        _S296 = g_2.s0_0;
    }
    else
    {
        _S296 = value_1;
    }
    if((_S294.y) == curve_6)
    {
        _S295 = ((*p_8).gradient_node_0.y) == node_3;
    }
    else
    {
        _S295 = false;
    }
    if(_S295)
    {
        _S295 = ((*p_8).gradient_kind_0.y) == kind_3;
    }
    else
    {
        _S295 = false;
    }
    if(_S295)
    {
        _S296 = g_2.s1_0;
    }
    if((_S294.z) == curve_6)
    {
        _S295 = ((*p_8).gradient_node_0.z) == node_3;
    }
    else
    {
        _S295 = false;
    }
    if(_S295)
    {
        _S295 = ((*p_8).gradient_kind_0.z) == kind_3;
    }
    else
    {
        _S295 = false;
    }
    if(_S295)
    {
        _S296 = g_2.s2_0;
    }
    if((_S294.w) == curve_6)
    {
        _S295 = ((*p_8).gradient_node_0.w) == node_3;
    }
    else
    {
        _S295 = false;
    }
    if(_S295)
    {
        _S295 = ((*p_8).gradient_kind_0.w) == kind_3;
    }
    else
    {
        _S295 = false;
    }
    if(_S295)
    {
        _S296 = g_2.s3_0;
    }
    return _S296;
}

fn s_fwd_slotted_0( dpvalue_0 : DiffPair_float_0,  curve_7 : u32,  node_4 : u32,  kind_4 : u32,  p_9 : ptr<function, RgbCurveParams_std140_0>,  slots_7 : bool,  dpg_0 : DiffPair_Gradient_0) -> DiffPair_float_0
{
    if(!slots_7)
    {
        var _S297 : DiffPair_float_0 = DiffPair_float_0( dpvalue_0.primal_0, dpvalue_0.differential_0 );
        return _S297;
    }
    var _S298 : vec4<u32> = (*p_9).gradient_curve_0;
    var _S299 : bool;
    if(((*p_9).gradient_curve_0.x) == curve_7)
    {
        _S299 = ((*p_9).gradient_node_0.x) == node_4;
    }
    else
    {
        _S299 = false;
    }
    if(_S299)
    {
        _S299 = ((*p_9).gradient_kind_0.x) == kind_4;
    }
    else
    {
        _S299 = false;
    }
    var _S300 : f32;
    var _S301 : f32;
    if(_S299)
    {
        _S300 = dpg_0.primal_0.s0_0;
        _S301 = dpg_0.differential_0.s0_0;
    }
    else
    {
        _S300 = dpvalue_0.primal_0;
        _S301 = dpvalue_0.differential_0;
    }
    if((_S298.y) == curve_7)
    {
        _S299 = ((*p_9).gradient_node_0.y) == node_4;
    }
    else
    {
        _S299 = false;
    }
    if(_S299)
    {
        _S299 = ((*p_9).gradient_kind_0.y) == kind_4;
    }
    else
    {
        _S299 = false;
    }
    if(_S299)
    {
        _S300 = dpg_0.primal_0.s1_0;
        _S301 = dpg_0.differential_0.s1_0;
    }
    if((_S298.z) == curve_7)
    {
        _S299 = ((*p_9).gradient_node_0.z) == node_4;
    }
    else
    {
        _S299 = false;
    }
    if(_S299)
    {
        _S299 = ((*p_9).gradient_kind_0.z) == kind_4;
    }
    else
    {
        _S299 = false;
    }
    if(_S299)
    {
        _S300 = dpg_0.primal_0.s2_0;
        _S301 = dpg_0.differential_0.s2_0;
    }
    if((_S298.w) == curve_7)
    {
        _S299 = ((*p_9).gradient_node_0.w) == node_4;
    }
    else
    {
        _S299 = false;
    }
    if(_S299)
    {
        _S299 = ((*p_9).gradient_kind_0.w) == kind_4;
    }
    else
    {
        _S299 = false;
    }
    if(_S299)
    {
        _S300 = dpg_0.primal_0.s3_0;
        _S301 = dpg_0.differential_0.s3_0;
    }
    var _S302 : DiffPair_float_0 = DiffPair_float_0( _S300, _S301 );
    return _S302;
}

fn s_fwd_slotted_1( dpvalue_1 : DiffPair_float_0,  curve_8 : u32,  node_5 : u32,  kind_5 : u32,  p_10 : ptr<function, RgbCurveParams_std140_0>,  slots_8 : bool,  dpg_1 : DiffPair_Gradient_0) -> DiffPair_float_0
{
    if(!slots_8)
    {
        var _S303 : DiffPair_float_0 = DiffPair_float_0( dpvalue_1.primal_0, dpvalue_1.differential_0 );
        return _S303;
    }
    var _S304 : vec4<u32> = (*p_10).gradient_curve_0;
    var _S305 : bool;
    if(((*p_10).gradient_curve_0.x) == curve_8)
    {
        _S305 = ((*p_10).gradient_node_0.x) == node_5;
    }
    else
    {
        _S305 = false;
    }
    if(_S305)
    {
        _S305 = ((*p_10).gradient_kind_0.x) == kind_5;
    }
    else
    {
        _S305 = false;
    }
    var _S306 : f32;
    var _S307 : f32;
    if(_S305)
    {
        _S306 = dpg_1.primal_0.s0_0;
        _S307 = dpg_1.differential_0.s0_0;
    }
    else
    {
        _S306 = dpvalue_1.primal_0;
        _S307 = dpvalue_1.differential_0;
    }
    if((_S304.y) == curve_8)
    {
        _S305 = ((*p_10).gradient_node_0.y) == node_5;
    }
    else
    {
        _S305 = false;
    }
    if(_S305)
    {
        _S305 = ((*p_10).gradient_kind_0.y) == kind_5;
    }
    else
    {
        _S305 = false;
    }
    if(_S305)
    {
        _S306 = dpg_1.primal_0.s1_0;
        _S307 = dpg_1.differential_0.s1_0;
    }
    if((_S304.z) == curve_8)
    {
        _S305 = ((*p_10).gradient_node_0.z) == node_5;
    }
    else
    {
        _S305 = false;
    }
    if(_S305)
    {
        _S305 = ((*p_10).gradient_kind_0.z) == kind_5;
    }
    else
    {
        _S305 = false;
    }
    if(_S305)
    {
        _S306 = dpg_1.primal_0.s2_0;
        _S307 = dpg_1.differential_0.s2_0;
    }
    if((_S304.w) == curve_8)
    {
        _S305 = ((*p_10).gradient_node_0.w) == node_5;
    }
    else
    {
        _S305 = false;
    }
    if(_S305)
    {
        _S305 = ((*p_10).gradient_kind_0.w) == kind_5;
    }
    else
    {
        _S305 = false;
    }
    if(_S305)
    {
        _S306 = dpg_1.primal_0.s3_0;
        _S307 = dpg_1.differential_0.s3_0;
    }
    var _S308 : DiffPair_float_0 = DiffPair_float_0( _S306, _S307 );
    return _S308;
}

fn knot_0( curve_9 : u32,  k_2 : u32,  p_11 : ptr<function, RgbCurveParams_std140_0>,  slots_9 : bool,  g_3 : Gradient_0) -> vec3<f32>
{
    var t_3 : vec4<f32> = knots_0[curve_9 * u32(20) + k_2];
    var _S309 : f32 = slotted_1(t_3.x, curve_9, k_2, u32(0), &((*p_11)), slots_9, g_3);
    var _S310 : f32 = slotted_1(t_3.y, curve_9, k_2, u32(1), &((*p_11)), slots_9, g_3);
    var _S311 : f32 = slotted_1(t_3.z, curve_9, k_2, u32(2), &((*p_11)), slots_9, g_3);
    return vec3<f32>(_S309, _S310, _S311);
}

struct DiffPair_vectorx3Cfloatx2C3x3E_0
{
     primal_0 : vec3<f32>,
     differential_0 : vec3<f32>,
};

fn s_fwd_knot_0( curve_10 : u32,  k_3 : u32,  p_12 : ptr<function, RgbCurveParams_std140_0>,  slots_10 : bool,  dpg_2 : DiffPair_Gradient_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var t_4 : vec4<f32> = knots_0[curve_10 * u32(20) + k_3];
    var _S312 : DiffPair_float_0 = DiffPair_float_0( t_4.x, 0.0f );
    var _S313 : DiffPair_Gradient_0 = DiffPair_Gradient_0( dpg_2.primal_0, dpg_2.differential_0 );
    var _S314 : DiffPair_float_0 = s_fwd_slotted_1(_S312, curve_10, k_3, u32(0), &((*p_12)), slots_10, _S313);
    var _S315 : DiffPair_float_0 = DiffPair_float_0( t_4.y, 0.0f );
    var _S316 : DiffPair_float_0 = s_fwd_slotted_1(_S315, curve_10, k_3, u32(1), &((*p_12)), slots_10, _S313);
    var _S317 : DiffPair_float_0 = DiffPair_float_0( t_4.z, 0.0f );
    var _S318 : DiffPair_float_0 = s_fwd_slotted_1(_S317, curve_10, k_3, u32(2), &((*p_12)), slots_10, _S313);
    var _S319 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S314.primal_0, _S316.primal_0, _S318.primal_0), vec3<f32>(_S314.differential_0, _S316.differential_0, _S318.differential_0) );
    return _S319;
}

fn pow0_forward_0( x_5 : DiffPair_float_0,  y_4 : DiffPair_float_0) -> DiffPair_float_0
{
    if(((x_5.primal_0) <= 0.0f))
    {
        var _S320 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
        return _S320;
    }
    var value_2 : f32 = pow(x_5.primal_0, y_4.primal_0);
    var _S321 : DiffPair_float_0 = DiffPair_float_0( value_2, x_5.differential_0 * y_4.primal_0 * pow(x_5.primal_0, y_4.primal_0 - 1.0f) + y_4.differential_0 * value_2 * log(x_5.primal_0) );
    return _S321;
}

fn curve_at_0( point_2 : Point_0,  p_13 : ptr<function, RgbCurveParams_std140_0>,  slots_11 : bool,  g_4 : Gradient_0) -> f32
{
    if((point_2.above_0) != u32(0))
    {
        var _S322 : vec3<f32> = knot_0(point_2.curve_0, (*p_13).count_0[point_2.curve_0] - u32(1), &((*p_13)), slots_11, g_4);
        var x_last_1 : f32 = _S322.x;
        var _S323 : f32 = slotted_0((*p_13).end_value_0[point_2.curve_0], point_2.curve_0, u32(0), u32(3), &((*p_13)), slots_11, g_4);
        var _S324 : f32 = slotted_0((*p_13).end_power_0[point_2.curve_0], point_2.curve_0, u32(0), u32(4), &((*p_13)), slots_11, g_4);
        return _S323 * pow0_0(point_2.v_0 / x_last_1, _S324);
    }
    var _S325 : vec3<f32> = knot_0(point_2.curve_0, point_2.segment_0, &((*p_13)), slots_11, g_4);
    var _S326 : vec3<f32> = knot_0(point_2.curve_0, point_2.segment_0 + u32(1), &((*p_13)), slots_11, g_4);
    var _S327 : f32 = _S325.x;
    var h_2 : f32 = _S326.x - _S327;
    var t_5 : f32 = (max(point_2.v_0, _S327) - _S327) / h_2;
    var t2_2 : f32 = t_5 * t_5;
    var t3_2 : f32 = t_5 * t2_2;
    var _S328 : f32 = 3.0f * t2_2;
    return clamp((2.0f * t3_2 - _S328 + 1.0f) * _S325.y + (t3_2 - 2.0f * t2_2 + t_5) * h_2 * _S325.z + (-2.0f * t3_2 + _S328) * _S326.y + (t3_2 - t2_2) * h_2 * _S326.z, 0.0f, 1.0f);
}

fn s_fwd_curve_at_0( point_3 : Point_0,  p_14 : ptr<function, RgbCurveParams_std140_0>,  slots_12 : bool,  dpg_3 : DiffPair_Gradient_0) -> DiffPair_float_0
{
    if((point_3.above_0) != u32(0))
    {
        var _S329 : DiffPair_Gradient_0 = DiffPair_Gradient_0( dpg_3.primal_0, dpg_3.differential_0 );
        var _S330 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_knot_0(point_3.curve_0, (*p_14).count_0[point_3.curve_0] - u32(1), &((*p_14)), slots_12, _S329);
        var x_last_2 : f32 = _S330.primal_0.x;
        var s_diff_x_last_0 : f32 = _S330.differential_0.x;
        var _S331 : DiffPair_float_0 = DiffPair_float_0( (*p_14).end_value_0[point_3.curve_0], 0.0f );
        var _S332 : DiffPair_float_0 = s_fwd_slotted_0(_S331, point_3.curve_0, u32(0), u32(3), &((*p_14)), slots_12, _S329);
        var _S333 : DiffPair_float_0 = DiffPair_float_0( (*p_14).end_power_0[point_3.curve_0], 0.0f );
        var _S334 : DiffPair_float_0 = s_fwd_slotted_0(_S333, point_3.curve_0, u32(0), u32(4), &((*p_14)), slots_12, _S329);
        var _S335 : DiffPair_float_0 = DiffPair_float_0( point_3.v_0 / x_last_2, (0.0f - point_3.v_0 * s_diff_x_last_0) / (x_last_2 * x_last_2) );
        var _S336 : DiffPair_float_0 = DiffPair_float_0( _S334.primal_0, _S334.differential_0 );
        var _S337 : DiffPair_float_0 = pow0_forward_0(_S335, _S336);
        var _S338 : DiffPair_float_0 = DiffPair_float_0( _S332.primal_0 * _S337.primal_0, _S332.differential_0 * _S337.primal_0 + _S337.differential_0 * _S332.primal_0 );
        return _S338;
    }
    var _S339 : DiffPair_Gradient_0 = DiffPair_Gradient_0( dpg_3.primal_0, dpg_3.differential_0 );
    var _S340 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_knot_0(point_3.curve_0, point_3.segment_0, &((*p_14)), slots_12, _S339);
    var _S341 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_knot_0(point_3.curve_0, point_3.segment_0 + u32(1), &((*p_14)), slots_12, _S339);
    var _S342 : f32 = _S340.primal_0.x;
    var _S343 : f32 = _S340.differential_0.x;
    var h_3 : f32 = _S341.primal_0.x - _S342;
    var s_diff_h_0 : f32 = _S341.differential_0.x - _S343;
    var _S344 : DiffPair_float_0 = DiffPair_float_0( point_3.v_0, 0.0f );
    var _S345 : DiffPair_float_0 = DiffPair_float_0( _S342, _S343 );
    var _S346 : DiffPair_float_0 = _d_max_1(_S344, _S345);
    var _S347 : f32 = _S346.primal_0 - _S342;
    var t_6 : f32 = _S347 / h_3;
    var s_diff_t_0 : f32 = ((_S346.differential_0 - _S343) * h_3 - _S347 * s_diff_h_0) / (h_3 * h_3);
    var t2_3 : f32 = t_6 * t_6;
    var _S348 : f32 = s_diff_t_0 * t_6;
    var s_diff_t2_0 : f32 = _S348 + _S348;
    var t3_3 : f32 = t_6 * t2_3;
    var s_diff_t3_0 : f32 = s_diff_t_0 * t2_3 + s_diff_t2_0 * t_6;
    var _S349 : f32 = 3.0f * t2_3;
    var _S350 : f32 = s_diff_t2_0 * 3.0f;
    var _S351 : f32 = 2.0f * t3_3 - _S349 + 1.0f;
    var _S352 : f32 = _S340.primal_0.y;
    var _S353 : f32 = t3_3 - 2.0f * t2_3 + t_6;
    var _S354 : f32 = _S353 * h_3;
    var _S355 : f32 = _S340.primal_0.z;
    var _S356 : f32 = -2.0f * t3_3 + _S349;
    var _S357 : f32 = _S341.primal_0.y;
    var _S358 : f32 = t3_3 - t2_3;
    var _S359 : f32 = _S358 * h_3;
    var _S360 : f32 = _S341.primal_0.z;
    var _S361 : DiffPair_float_0 = DiffPair_float_0( _S351 * _S352 + _S354 * _S355 + _S356 * _S357 + _S359 * _S360, (s_diff_t3_0 * 2.0f - _S350) * _S352 + _S340.differential_0.y * _S351 + (((s_diff_t3_0 - s_diff_t2_0 * 2.0f + s_diff_t_0) * h_3 + s_diff_h_0 * _S353) * _S355 + _S340.differential_0.z * _S354) + ((s_diff_t3_0 * -2.0f + _S350) * _S357 + _S341.differential_0.y * _S356) + (((s_diff_t3_0 - s_diff_t2_0) * h_3 + s_diff_h_0 * _S358) * _S360 + _S341.differential_0.z * _S359) );
    var _S362 : DiffPair_float_0 = DiffPair_float_0( 0.0f, 0.0f );
    var _S363 : DiffPair_float_0 = DiffPair_float_0( 1.0f, 0.0f );
    var _S364 : DiffPair_float_0 = _d_clamp_1(_S361, _S362, _S363);
    var _S365 : DiffPair_float_0 = DiffPair_float_0( _S364.primal_0, _S364.differential_0 );
    return _S365;
}

fn rgb_curve_1( rgb_3 : vec3<f32>,  l_2 : Lookup_0,  p_15 : ptr<function, RgbCurveParams_std140_0>,  slots_13 : bool,  g_5 : Gradient_0) -> vec3<f32>
{
    if((l_2.mode_0) == u32(2))
    {
        return rgb_3;
    }
    if((l_2.mode_0) == u32(1))
    {
        var _S366 : f32 = curve_at_0(l_2.r_0, &((*p_15)), slots_13, g_5);
        return rgb_3 * vec3<f32>((_S366 / l_2.r_0.v_0));
    }
    var _S367 : f32 = curve_at_0(l_2.r_0, &((*p_15)), slots_13, g_5);
    var _S368 : f32 = curve_at_0(l_2.g_0, &((*p_15)), slots_13, g_5);
    var _S369 : f32 = curve_at_0(l_2.b_0, &((*p_15)), slots_13, g_5);
    return vec3<f32>(_S367, _S368, _S369);
}

fn s_fwd_rgb_curve_0( rgb_4 : vec3<f32>,  l_3 : Lookup_0,  p_16 : ptr<function, RgbCurveParams_std140_0>,  slots_14 : bool,  dpg_4 : DiffPair_Gradient_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    if((l_3.mode_0) == u32(2))
    {
        var _S370 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( rgb_4, vec3<f32>(0.0f) );
        return _S370;
    }
    if((l_3.mode_0) == u32(1))
    {
        var _S371 : DiffPair_Gradient_0 = DiffPair_Gradient_0( dpg_4.primal_0, dpg_4.differential_0 );
        var _S372 : DiffPair_float_0 = s_fwd_curve_at_0(l_3.r_0, &((*p_16)), slots_14, _S371);
        var _S373 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( rgb_4 * vec3<f32>((_S372.primal_0 / l_3.r_0.v_0)), vec3<f32>((_S372.differential_0 / l_3.r_0.v_0)) * rgb_4 );
        return _S373;
    }
    var _S374 : DiffPair_Gradient_0 = DiffPair_Gradient_0( dpg_4.primal_0, dpg_4.differential_0 );
    var _S375 : DiffPair_float_0 = s_fwd_curve_at_0(l_3.r_0, &((*p_16)), slots_14, _S374);
    var _S376 : DiffPair_float_0 = s_fwd_curve_at_0(l_3.g_0, &((*p_16)), slots_14, _S374);
    var _S377 : DiffPair_float_0 = s_fwd_curve_at_0(l_3.b_0, &((*p_16)), slots_14, _S374);
    var _S378 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( vec3<f32>(_S375.primal_0, _S376.primal_0, _S377.primal_0), vec3<f32>(_S375.differential_0, _S376.differential_0, _S377.differential_0) );
    return _S378;
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_0 : vec3<u32>)
{
    var _S379 : u32 = id_0.x;
    var _S380 : bool;
    if((_S379 >= (params_0.pass_0.width_0)))
    {
        _S380 = true;
    }
    else
    {
        _S380 = ((id_0.y) >= (params_0.pass_0.height_0));
    }
    if(_S380)
    {
        return;
    }
    var i_0 : u32 = id_0.y * params_0.pass_0.width_0 + _S379;
    var _S381 : RgbCurveParams_std140_0 = params_0.rgb_curve_0;
    var _S382 : Gradient_0 = Gradient_x24init_0(0.0f, 0.0f, 0.0f, 0.0f);
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
        var _S383 : Lookup_0 = locate_pixel_0(rgb_5, &(_S381));
        var _S384 : vec3<f32> = rgb_curve_1(rgb_5, _S383, &(_S381), false, _S382);
        var _S385 : u32 = k_4 + u32(1);
        rgb_5 = _S384;
        k_4 = _S385;
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
    var _S386 : f32;
    if(kind_6 == u32(0))
    {
        _S386 = knot_1.x;
    }
    else
    {
        if(kind_6 == u32(1))
        {
            _S386 = knot_1.y;
        }
        else
        {
            _S386 = knot_1.z;
        }
    }
    return _S386;
}

fn gradient_0( p_18 : ptr<function, RgbCurveParams_std140_0>) -> Gradient_0
{
    var g_6 : Gradient_0;
    var _S387 : vec4<u32> = (*p_18).gradient_curve_0;
    var _S388 : vec4<u32> = (*p_18).gradient_node_0;
    var _S389 : vec4<u32> = (*p_18).gradient_kind_0;
    var _S390 : f32 = slot_value_0((*p_18).gradient_curve_0.x, (*p_18).gradient_node_0.x, (*p_18).gradient_kind_0.x, &((*p_18)));
    g_6.s0_0 = _S390;
    var _S391 : f32 = slot_value_0(_S387.y, _S388.y, _S389.y, &((*p_18)));
    g_6.s1_0 = _S391;
    var _S392 : f32 = slot_value_0(_S387.z, _S388.z, _S389.z, &((*p_18)));
    g_6.s2_0 = _S392;
    var _S393 : f32 = slot_value_0(_S387.w, _S388.w, _S389.w, &((*p_18)));
    g_6.s3_0 = _S393;
    return g_6;
}

@compute
@workgroup_size(16, 16, 1)
fn rgbcurve_backward(@builtin(global_invocation_id) id_1 : vec3<u32>)
{
    var _S394 : u32 = id_1.x;
    var _S395 : bool;
    if((_S394 >= (params_0.pass_0.width_0)))
    {
        _S395 = true;
    }
    else
    {
        _S395 = ((id_1.y) >= (params_0.pass_0.height_0));
    }
    if(_S395)
    {
        return;
    }
    var i_1 : u32 = id_1.y * params_0.pass_0.width_0 + _S394;
    var _S396 : RgbCurveParams_std140_0 = params_0.rgb_curve_0;
    var rgb_6 : vec3<f32> = input_0[i_1].xyz;
    var _S397 : Gradient_0 = gradient_0(&(_S396));
    var _S398 : Gradient_0 = Gradient_x24_syn_dzero_0();
    var g_7 : DiffPair_Gradient_0;
    g_7.primal_0 = _S397;
    g_7.differential_0 = _S398;
    var _S399 : Lookup_0 = locate_pixel_0(rgb_6, &(_S396));
    _S247(rgb_6, _S399, &(_S396), true, &(g_7), derivative_in_0[i_1].xyz);
    derivative_out_0[i_1] = vec4<f32>(g_7.differential_0.s0_0, g_7.differential_0.s1_0, g_7.differential_0.s2_0, g_7.differential_0.s3_0);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn rgbcurve_forward_diff(@builtin(global_invocation_id) id_2 : vec3<u32>)
{
    var _S400 : u32 = id_2.x;
    var _S401 : bool;
    if((_S400 >= (params_0.pass_0.width_0)))
    {
        _S401 = true;
    }
    else
    {
        _S401 = ((id_2.y) >= (params_0.pass_0.height_0));
    }
    if(_S401)
    {
        return;
    }
    var i_2 : u32 = id_2.y * params_0.pass_0.width_0 + _S400;
    var _S402 : RgbCurveParams_std140_0 = params_0.rgb_curve_0;
    var rgb_7 : vec3<f32> = input_0[i_2].xyz;
    var v_2 : vec4<f32> = derivative_in_0[i_2];
    var direction_0 : Gradient_0;
    direction_0.s0_0 = v_2.x;
    direction_0.s1_0 = v_2.y;
    direction_0.s2_0 = v_2.z;
    direction_0.s3_0 = v_2.w;
    var _S403 : Lookup_0 = locate_pixel_0(rgb_7, &(_S402));
    var _S404 : Gradient_0 = gradient_0(&(_S402));
    var _S405 : DiffPair_Gradient_0 = DiffPair_Gradient_0( _S404, direction_0 );
    var _S406 : DiffPair_vectorx3Cfloatx2C3x3E_0 = s_fwd_rgb_curve_0(rgb_7, _S403, &(_S402), true, _S405);
    derivative_out_0[i_2] = vec4<f32>(_S406.differential_0, 0.0f);
    return;
}

