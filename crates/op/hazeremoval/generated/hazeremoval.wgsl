// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2017-2025  darktable developers (ported from src/iop/hazeremoval.c,
// src/common/box_filters.cc and src/common/guided_filter.c)
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

// Generated from slang/hazeremoval.slang by tools/build-kernels.sh. Do not edit.

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

struct Dehaze_std140_0
{
    @align(16) strength_0 : f32,
    @align(4) distance_0 : f32,
};

struct HazeParams_std140_0
{
    @align(16) width_1 : u32,
    @align(4) height_1 : u32,
    @align(8) dark_radius_0 : u32,
    @align(4) guide_radius_0 : u32,
    @align(16) rank_0 : u32,
    @align(4) partials_across_0 : u32,
    @align(8) partials_0 : u32,
    @align(4) maximum_0 : u32,
    @align(16) normalise_0 : u32,
    @align(4) selection_0 : u32,
    @align(8) digit_0 : u32,
    @align(4) eps_0 : f32,
    @align(16) max_depth_0 : f32,
    @align(4) reserved0_1 : u32,
    @align(8) reserved1_1 : u32,
    @align(4) reserved2_0 : u32,
    @align(16) dehaze_0 : Dehaze_std140_0,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) haze_0 : HazeParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(16) @group(0) var<storage, read_write> image_0 : array<vec4<f32>>;

@binding(0) @group(0) var<storage, read> input_0 : array<vec4<f32>>;

@binding(14) @group(0) var<storage, read_write> state_0 : array<u32>;

@binding(10) @group(0) var<storage, read_write> plane_a_0 : array<f32>;

@binding(11) @group(0) var<storage, read_write> plane_b_0 : array<f32>;

@binding(13) @group(0) var<storage, read_write> bins_0 : array<atomic<u32>>;

struct AmbientPartial_std430_0
{
    @align(16) sum_0 : vec3<f32>,
    @align(4) count_0 : u32,
};

@binding(15) @group(0) var<storage, read_write> partials_1 : array<AmbientPartial_std430_0>;

@binding(12) @group(0) var<storage, read_write> coefficients_0 : array<vec4<f32>>;

@binding(17) @group(0) var<storage, read_write> viewport_plane_0 : array<f32>;

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

fn s_apply_exp_0( x_1 : f32) -> Tuple_1
{
    var _S9 : Tuple_1 = Tuple_1( exp(x_1) );
    return _S9;
}

struct s_bwdCallableCtx_exp_0
{
     _S10 : f32,
};

fn _d_exp_0( dpx_1 : ptr<function, DiffPair_float_0>,  dOut_1 : f32)
{
    var _S11 : f32 = exp((*dpx_1).primal_0) * dOut_1;
    (*dpx_1).primal_0 = (*dpx_1).primal_0;
    (*dpx_1).differential_0 = _S11;
    return;
}

fn s_bwdProp_exp_0( ctx_1 : s_bwdCallableCtx_exp_0,  d_x_1 : ptr<function, f32>,  d_out_1 : f32)
{
    var _S12 : DiffPair_float_0;
    _S12.primal_0 = ctx_1._S10;
    _d_exp_0(&(_S12), d_out_1);
    (*d_x_1) = _S12.differential_0;
    return;
}

struct Tuple_2
{
     value0_2 : f32,
};

fn s_apply_clamp_0( x_2 : f32,  minBound_0 : f32,  maxBound_0 : f32) -> Tuple_2
{
    var _S13 : Tuple_2 = Tuple_2( clamp(x_2, minBound_0, maxBound_0) );
    return _S13;
}

struct s_bwdCallableCtx_clamp_0
{
     _S14 : f32,
     _S15 : f32,
     _S16 : f32,
};

fn _d_clamp_0( dpx_2 : ptr<function, DiffPair_float_0>,  dpMin_0 : ptr<function, DiffPair_float_0>,  dpMax_0 : ptr<function, DiffPair_float_0>,  dOut_2 : f32)
{
    var _S17 : DiffPair_float_0 = (*dpx_2);
    var _S18 : bool;
    if((((*dpx_2).primal_0) >= ((*dpMin_0).primal_0)))
    {
        _S18 = (((*dpx_2).primal_0) <= ((*dpMax_0).primal_0));
    }
    else
    {
        _S18 = false;
    }
    var _S19 : f32;
    if(_S18)
    {
        _S19 = dOut_2;
    }
    else
    {
        _S19 = 0.0f;
    }
    (*dpx_2).primal_0 = _S17.primal_0;
    (*dpx_2).differential_0 = _S19;
    var _S20 : DiffPair_float_0 = (*dpMin_0);
    if(((_S17.primal_0) < ((*dpMin_0).primal_0)))
    {
        _S19 = dOut_2;
    }
    else
    {
        _S19 = 0.0f;
    }
    (*dpMin_0).primal_0 = _S20.primal_0;
    (*dpMin_0).differential_0 = _S19;
    var _S21 : DiffPair_float_0 = (*dpMax_0);
    if((((*dpx_2).primal_0) > ((*dpMax_0).primal_0)))
    {
        _S19 = dOut_2;
    }
    else
    {
        _S19 = 0.0f;
    }
    (*dpMax_0).primal_0 = _S21.primal_0;
    (*dpMax_0).differential_0 = _S19;
    return;
}

fn s_bwdProp_clamp_0( ctx_2 : s_bwdCallableCtx_clamp_0,  d_x_2 : ptr<function, f32>,  d_minBound_0 : ptr<function, f32>,  d_maxBound_0 : ptr<function, f32>,  d_out_2 : f32)
{
    var _S22 : DiffPair_float_0;
    _S22.primal_0 = ctx_2._S14;
    var _S23 : DiffPair_float_0;
    _S23.primal_0 = ctx_2._S15;
    var _S24 : DiffPair_float_0;
    _S24.primal_0 = ctx_2._S16;
    _d_clamp_0(&(_S22), &(_S23), &(_S24), d_out_2);
    (*d_x_2) = _S22.differential_0;
    (*d_minBound_0) = _S23.differential_0;
    (*d_maxBound_0) = _S24.differential_0;
    return;
}

struct Dehaze_0
{
     strength_0 : f32,
     distance_0 : f32,
};

struct DiffPair_Dehaze_0
{
     primal_0 : Dehaze_0,
     differential_0 : Dehaze_0,
};

struct s_paramCtx_s_bwdCallableCtx_dehaze_0
{
     value0_3 : vec3<f32>,
     value1_0 : f32,
     value2_0 : vec3<f32>,
     value3_0 : f32,
     value4_0 : Dehaze_0,
};

struct s_bwdCallableCtx_dehaze_0
{
     value0_4 : s_paramCtx_s_bwdCallableCtx_dehaze_0,
};

fn Dehaze_x24_syn_dzero_0() -> Dehaze_0
{
    var result_0 : Dehaze_0;
    result_0.strength_0 = 0.0f;
    result_0.distance_0 = 0.0f;
    return result_0;
}

fn Dehaze_x24_syn_dadd_0( SLANG_anonymous_0_0 : Dehaze_0,  SLANG_anonymous_1_0 : Dehaze_0) -> Dehaze_0
{
    var result_1 : Dehaze_0;
    result_1.strength_0 = SLANG_anonymous_0_0.strength_0 + SLANG_anonymous_1_0.strength_0;
    result_1.distance_0 = SLANG_anonymous_0_0.distance_0 + SLANG_anonymous_1_0.distance_0;
    return result_1;
}

fn s_bwdProp_dehaze_0( _S25 : s_bwdCallableCtx_dehaze_0,  _S26 : ptr<function, Dehaze_0>,  _S27 : vec3<f32>)
{
    var transmission_0 : f32 = 1.0f - _S25.value0_4.value4_0.strength_0 * _S25.value0_4.value1_0;
    var _S28 : f32 = - _S25.value0_4.value4_0.distance_0 * _S25.value0_4.value3_0;
    var _S29 : Tuple_2 = s_apply_clamp_0(_S28, -80.0f, 80.0f);
    var _S30 : s_bwdCallableCtx_clamp_0;
    _S30._S14 = _S28;
    _S30._S15 = -80.0f;
    _S30._S16 = 80.0f;
    var _S31 : Tuple_1 = s_apply_exp_0(_S29.value0_2);
    var _S32 : s_bwdCallableCtx_exp_0;
    _S32._S10 = _S29.value0_2;
    var _S33 : Tuple_2 = s_apply_clamp_0(_S31.value0_1, 0.0009765625f, 1.0f);
    var _S34 : s_bwdCallableCtx_clamp_0;
    _S34._S14 = _S31.value0_1;
    _S34._S15 = 0.0009765625f;
    _S34._S16 = 1.0f;
    var _S35 : Tuple_0 = s_apply_max_0(transmission_0, _S33.value0_2);
    var _S36 : s_bwdCallableCtx_max_0;
    _S36._S2 = transmission_0;
    _S36._S3 = _S33.value0_2;
    var _S37 : Dehaze_0 = Dehaze_x24_syn_dzero_0();
    var _S38 : vec3<f32> = (_S25.value0_4.value0_3 - _S25.value0_4.value2_0) * (vec3<f32>(0) - _S27 / vec3<f32>((_S35.value0_0 * _S35.value0_0)));
    var _S39 : f32 = _S38[i32(0)] + _S38[i32(1)] + _S38[i32(2)];
    var _S40 : f32 = 0.0f;
    var _S41 : f32 = 0.0f;
    s_bwdProp_max_0(_S36, &(_S40), &(_S41), _S39);
    var _S42 : f32 = 0.0f;
    var _S43 : f32 = 0.0f;
    var _S44 : f32 = 0.0f;
    s_bwdProp_clamp_0(_S34, &(_S42), &(_S43), &(_S44), _S41);
    var _S45 : f32 = 0.0f;
    s_bwdProp_exp_0(_S32, &(_S45), _S42);
    var _S46 : f32 = 0.0f;
    var _S47 : f32 = 0.0f;
    var _S48 : f32 = 0.0f;
    s_bwdProp_clamp_0(_S30, &(_S46), &(_S47), &(_S48), _S45);
    var _S49 : f32 = - (_S25.value0_4.value3_0 * _S46);
    var _S50 : f32 = _S25.value0_4.value1_0 * - _S40;
    var _S51 : Dehaze_0 = _S37;
    _S51.distance_0 = _S49;
    _S51.strength_0 = _S50;
    (*_S26) = Dehaze_x24_syn_dadd_0(_S37, _S51);
    return;
}

fn _S52( _S53 : vec3<f32>,  _S54 : f32,  _S55 : vec3<f32>,  _S56 : f32,  _S57 : ptr<function, DiffPair_Dehaze_0>,  _S58 : vec3<f32>)
{
    var _S59 : s_paramCtx_s_bwdCallableCtx_dehaze_0;
    _S59.value0_3 = _S53;
    _S59.value1_0 = _S54;
    _S59.value2_0 = _S55;
    _S59.value3_0 = _S56;
    _S59.value4_0 = (*_S57).primal_0;
    var _S60 : s_bwdCallableCtx_dehaze_0 = s_bwdCallableCtx_dehaze_0( _S59 );
    var _S61 : Dehaze_0 = (*_S57).differential_0;
    s_bwdProp_dehaze_0(_S60, &(_S61), _S58);
    (*_S57).differential_0 = _S61;
    return;
}

fn _d_exp_1( dpx_3 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S62 : f32 = exp(dpx_3.primal_0);
    var _S63 : DiffPair_float_0 = DiffPair_float_0( _S62, _S62 * dpx_3.differential_0 );
    return _S63;
}

fn _d_clamp_1( dpx_4 : DiffPair_float_0,  dpMin_1 : DiffPair_float_0,  dpMax_1 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S64 : f32 = clamp(dpx_4.primal_0, dpMin_1.primal_0, dpMax_1.primal_0);
    var _S65 : bool;
    if(((dpx_4.primal_0) >= (dpMin_1.primal_0)))
    {
        _S65 = ((dpx_4.primal_0) <= (dpMax_1.primal_0));
    }
    else
    {
        _S65 = false;
    }
    var _S66 : f32;
    if(_S65)
    {
        _S66 = dpx_4.differential_0;
    }
    else
    {
        if(((dpx_4.primal_0) < (dpMin_1.primal_0)))
        {
            _S66 = dpMin_1.differential_0;
        }
        else
        {
            _S66 = dpMax_1.differential_0;
        }
    }
    var _S67 : DiffPair_float_0 = DiffPair_float_0( _S64, _S66 );
    return _S67;
}

fn pixel_index_0( x_3 : i32,  y_1 : i32) -> u32
{
    return u32(y_1) * params_0.haze_0.width_1 + u32(x_3);
}

fn PlaneA_read_0( x_4 : u32,  y_2 : u32) -> f32
{
    return plane_a_0[pixel_index_0(i32(x_4), i32(y_2))];
}

fn footprint_mean_0( o_0 : vec2<u32>,  source_size_0 : vec2<u32>,  output_size_0 : vec2<u32>) -> f32
{
    var _S68 : u32;
    var start_0 : vec2<u32> = o_0 * source_size_0 / output_size_0;
    var end_0 : vec2<u32> = (o_0 + vec2<u32>(u32(1))) * source_size_0 / output_size_0;
    var _S69 : u32 = start_0.y;
    var sum_1 : f32 = 0.0f;
    var y_3 : u32 = _S69;
    loop
    {
        var _S70 : u32 = end_0.y;
        _S68 = _S70;
        if((y_3 < _S70))
        {
        }
        else
        {
            break;
        }
        var x_5 : u32 = start_0.x;
        loop
        {
            if((x_5 < (end_0.x)))
            {
            }
            else
            {
                break;
            }
            var _S71 : f32 = sum_1 + PlaneA_read_0(x_5, y_3);
            var _S72 : u32 = x_5 + u32(1);
            sum_1 = _S71;
            x_5 = _S72;
        }
        y_3 = y_3 + u32(1);
    }
    return sum_1 / f32(i32((end_0.x - start_0.x) * (_S68 - _S69)));
}

fn _d_max_1( dpx_5 : DiffPair_float_0,  dpy_1 : DiffPair_float_0) -> DiffPair_float_0
{
    var _S73 : f32 = max(dpx_5.primal_0, dpy_1.primal_0);
    var _S74 : f32;
    if(((dpx_5.primal_0) > (dpy_1.primal_0)))
    {
        _S74 = dpx_5.differential_0;
    }
    else
    {
        if(((dpx_5.primal_0) < (dpy_1.primal_0)))
        {
            _S74 = dpy_1.differential_0;
        }
        else
        {
            _S74 = 0.5f * (dpx_5.differential_0 + dpy_1.differential_0);
        }
    }
    var _S75 : DiffPair_float_0 = DiffPair_float_0( _S73, _S74 );
    return _S75;
}

fn whole_index_0( id_0 : vec3<u32>) -> u32
{
    return pixel_index_0(i32(params_0.pass_0.origin_x_0 + id_0.x), i32(params_0.pass_0.origin_y_0 + id_0.y));
}

@compute
@workgroup_size(16, 16, 1)
fn image_copy(@builtin(global_invocation_id) id_1 : vec3<u32>)
{
    var _S76 : u32 = id_1.x;
    var _S77 : bool;
    if((_S76 >= (params_0.pass_0.width_0)))
    {
        _S77 = true;
    }
    else
    {
        _S77 = ((id_1.y) >= (params_0.pass_0.height_0));
    }
    if(_S77)
    {
        return;
    }
    image_0[whole_index_0(id_1)] = input_0[id_1.y * params_0.pass_0.width_0 + _S76];
    return;
}

fn smallest_0( rgb_0 : vec3<f32>) -> f32
{
    return min(min(rgb_0.x, rgb_0.y), rgb_0.z);
}

fn smallest_channel_0( pixel_0 : vec4<f32>) -> f32
{
    if((params_0.haze_0.normalise_0) != u32(0))
    {
        var rgb_1 : vec3<f32> = pixel_0.xyz;
        return min(min(rgb_1.x * (bitcast<f32>((state_0[u32(12)]))), rgb_1.y * (bitcast<f32>((state_0[u32(13)])))), rgb_1.z * (bitcast<f32>((state_0[u32(14)]))));
    }
    return smallest_0(pixel_0.xyz);
}

fn extreme_0( a_0 : f32,  b_0 : f32) -> f32
{
    var _S78 : f32;
    if((params_0.haze_0.maximum_0) != u32(0))
    {
        _S78 = max(a_0, b_0);
    }
    else
    {
        _S78 = min(a_0, b_0);
    }
    return _S78;
}

@compute
@workgroup_size(16, 16, 1)
fn image_rows(@builtin(global_invocation_id) id_2 : vec3<u32>)
{
    var width_2 : i32 = i32(params_0.haze_0.width_1);
    var _S79 : u32 = id_2.x;
    var _S80 : bool;
    if((_S79 >= (params_0.haze_0.width_1)))
    {
        _S80 = true;
    }
    else
    {
        _S80 = ((id_2.y) >= (params_0.haze_0.height_1));
    }
    if(_S80)
    {
        return;
    }
    var x_6 : i32 = i32(_S79);
    var y_4 : i32 = i32(id_2.y);
    var r_0 : i32 = i32(params_0.haze_0.dark_radius_0);
    var _S81 : i32 = max(i32(0), x_6 - r_0);
    var _S82 : i32 = min(width_2 - i32(1), x_6 + r_0);
    var _S83 : i32 = _S81 + i32(1);
    var m_0 : f32 = smallest_channel_0(image_0[pixel_index_0(_S81, y_4)]);
    var xx_0 : i32 = _S83;
    loop
    {
        if((xx_0 <= _S82))
        {
        }
        else
        {
            break;
        }
        var _S84 : f32 = extreme_0(m_0, smallest_channel_0(image_0[pixel_index_0(xx_0, y_4)]));
        var _S85 : i32 = xx_0 + i32(1);
        m_0 = _S84;
        xx_0 = _S85;
    }
    plane_a_0[pixel_index_0(x_6, y_4)] = m_0;
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn plane_cols(@builtin(global_invocation_id) id_3 : vec3<u32>)
{
    var height_2 : i32 = i32(params_0.haze_0.height_1);
    var _S86 : u32 = id_3.x;
    var _S87 : bool;
    if((_S86 >= (params_0.haze_0.width_1)))
    {
        _S87 = true;
    }
    else
    {
        _S87 = ((id_3.y) >= (params_0.haze_0.height_1));
    }
    if(_S87)
    {
        return;
    }
    var x_7 : i32 = i32(_S86);
    var y_5 : i32 = i32(id_3.y);
    var r_1 : i32 = i32(params_0.haze_0.dark_radius_0);
    var _S88 : i32 = max(i32(0), y_5 - r_1);
    var _S89 : i32 = min(height_2 - i32(1), y_5 + r_1);
    var _S90 : i32 = _S88 + i32(1);
    var m_1 : f32 = plane_a_0[pixel_index_0(x_7, _S88)];
    var yy_0 : i32 = _S90;
    loop
    {
        if((yy_0 <= _S89))
        {
        }
        else
        {
            break;
        }
        var _S91 : f32 = extreme_0(m_1, plane_a_0[pixel_index_0(x_7, yy_0)]);
        var _S92 : i32 = yy_0 + i32(1);
        m_1 = _S91;
        yy_0 = _S92;
    }
    plane_b_0[pixel_index_0(x_7, y_5)] = m_1;
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn plane_rows(@builtin(global_invocation_id) id_4 : vec3<u32>)
{
    var width_3 : i32 = i32(params_0.haze_0.width_1);
    var _S93 : u32 = id_4.x;
    var _S94 : bool;
    if((_S93 >= (params_0.haze_0.width_1)))
    {
        _S94 = true;
    }
    else
    {
        _S94 = ((id_4.y) >= (params_0.haze_0.height_1));
    }
    if(_S94)
    {
        return;
    }
    var x_8 : i32 = i32(_S93);
    var y_6 : i32 = i32(id_4.y);
    var r_2 : i32 = i32(params_0.haze_0.dark_radius_0);
    var _S95 : i32 = max(i32(0), x_8 - r_2);
    var _S96 : i32 = min(width_3 - i32(1), x_8 + r_2);
    var _S97 : i32 = _S95 + i32(1);
    var m_2 : f32 = plane_b_0[pixel_index_0(_S95, y_6)];
    var xx_1 : i32 = _S97;
    loop
    {
        if((xx_1 <= _S96))
        {
        }
        else
        {
            break;
        }
        var _S98 : f32 = extreme_0(m_2, plane_b_0[pixel_index_0(xx_1, y_6)]);
        var _S99 : i32 = xx_1 + i32(1);
        m_2 = _S98;
        xx_1 = _S99;
    }
    plane_a_0[pixel_index_0(x_8, y_6)] = m_2;
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn select_begin(@builtin(local_invocation_index) t_0 : u32)
{
    var bin_0 : u32 = t_0;
    loop
    {
        if((bin_0 < u32(2048)))
        {
        }
        else
        {
            break;
        }
        atomicStore(&(bins_0[bin_0]), u32(0));
        bin_0 = bin_0 + u32(256);
    }
    if(t_0 == u32(0))
    {
        state_0[u32(0)] = u32(0);
        state_0[u32(1)] = params_0.haze_0.rank_0;
    }
    return;
}

var<workgroup> shared_bins_0 : array<atomic<u32>, i32(2048)>;

fn digit_bits_0( digit_1 : u32) -> u32
{
    var _S100 : u32;
    if(digit_1 == u32(2))
    {
        _S100 = u32(10);
    }
    else
    {
        _S100 = u32(11);
    }
    return _S100;
}

fn brightness_0( rgb_2 : vec3<f32>) -> f32
{
    return rgb_2.x + rgb_2.y + rgb_2.z;
}

fn order_key_0( value_0 : f32) -> u32
{
    var bits_0 : u32 = (bitcast<u32>((value_0)));
    var bits_1 : u32;
    if(bits_0 == u32(2147483648))
    {
        bits_1 = u32(0);
    }
    else
    {
        bits_1 = bits_0;
    }
    if(((bits_1 & (u32(2147483648)))) != u32(0))
    {
        bits_1 = ~bits_1;
    }
    else
    {
        bits_1 = (bits_1 | (u32(2147483648)));
    }
    return bits_1;
}

fn digit_shift_0( digit_2 : u32) -> u32
{
    var _S101 : u32;
    if(digit_2 == u32(0))
    {
        _S101 = u32(21);
    }
    else
    {
        if(digit_2 == u32(1))
        {
            _S101 = u32(10);
        }
        else
        {
            _S101 = u32(0);
        }
    }
    return _S101;
}

@compute
@workgroup_size(16, 16, 1)
fn select_count(@builtin(global_invocation_id) id_5 : vec3<u32>, @builtin(local_invocation_index) t_1 : u32)
{
    var bin_1 : u32 = t_1;
    loop
    {
        if((bin_1 < u32(2048)))
        {
        }
        else
        {
            break;
        }
        atomicStore(&(shared_bins_0[bin_1]), u32(0));
        bin_1 = bin_1 + u32(256);
    }
    workgroupBarrier();
    var digit_3 : u32 = params_0.haze_0.digit_0;
    var _S102 : u32 = ((u32(1) << ((digit_bits_0(params_0.haze_0.digit_0))))) - u32(1);
    var _S103 : u32 = state_0[u32(0)];
    var _S104 : f32 = (bitcast<f32>((state_0[u32(2)])));
    var by_0 : u32 = u32(0);
    loop
    {
        if((by_0 < u32(4)))
        {
        }
        else
        {
            break;
        }
        var bx_0 : u32 = u32(0);
        loop
        {
            if((bx_0 < u32(4)))
            {
            }
            else
            {
                break;
            }
            var x_9 : u32 = u32(4) * id_5.x + bx_0;
            var y_7 : u32 = u32(4) * id_5.y + by_0;
            var _S105 : bool;
            if((x_9 >= (params_0.haze_0.width_1)))
            {
                _S105 = true;
            }
            else
            {
                _S105 = (y_7 >= (params_0.haze_0.height_1));
            }
            if(_S105)
            {
                bx_0 = bx_0 + u32(1);
                continue;
            }
            var i_0 : u32 = pixel_index_0(i32(x_9), i32(y_7));
            var dark_0 : f32 = plane_b_0[i_0];
            var counted_0 : bool;
            var value_1 : f32;
            if((params_0.haze_0.selection_0) != u32(0))
            {
                var _S106 : bool = (dark_0 >= _S104);
                value_1 = brightness_0(image_0[i_0].xyz);
                counted_0 = _S106;
            }
            else
            {
                value_1 = dark_0;
                counted_0 = true;
            }
            var key_0 : u32 = order_key_0(value_1);
            var _S107 : bool;
            if(counted_0)
            {
                if(digit_3 == u32(0))
                {
                    _S107 = true;
                }
                else
                {
                    _S107 = ((key_0 >> ((digit_shift_0(digit_3 - u32(1)))))) == _S103;
                }
            }
            else
            {
                _S107 = false;
            }
            if(_S107)
            {
                var _S108 : u32 = atomicAdd(&(shared_bins_0[(((key_0 >> ((digit_shift_0(digit_3))))) & (_S102))]), u32(1));
            }
            bx_0 = bx_0 + u32(1);
        }
        by_0 = by_0 + u32(1);
    }
    workgroupBarrier();
    bin_1 = t_1;
    loop
    {
        if((bin_1 < u32(2048)))
        {
        }
        else
        {
            break;
        }
        var count_1 : u32 = atomicLoad(&(shared_bins_0[bin_1]));
        if(count_1 != u32(0))
        {
            var _S109 : u32 = atomicAdd(&(bins_0[bin_1]), count_1);
        }
        bin_1 = bin_1 + u32(256);
    }
    return;
}

var<workgroup> shared_sums_0 : array<u32, i32(256)>;

fn from_order_key_0( key_1 : u32) -> f32
{
    var _S110 : u32;
    if(((key_1 & (u32(2147483648)))) != u32(0))
    {
        _S110 = (key_1 & (u32(2147483647)));
    }
    else
    {
        _S110 = ~key_1;
    }
    return (bitcast<f32>((_S110)));
}

@compute
@workgroup_size(16, 16, 1)
fn select_scan(@builtin(local_invocation_index) t_2 : u32)
{
    var add_0 : u32;
    var bin_2 : u32;
    var digit_4 : u32 = params_0.haze_0.digit_0;
    var _S111 : u32 = digit_bits_0(params_0.haze_0.digit_0);
    var per_0 : u32 = ((u32(1) << (_S111))) / u32(256);
    var first_0 : u32 = t_2 * per_0;
    var rank_1 : u32 = state_0[u32(1)];
    var prefix_0 : u32 = state_0[u32(0)];
    var k_0 : u32 = u32(0);
    var sum_2 : u32 = u32(0);
    loop
    {
        if((k_0 < per_0))
        {
        }
        else
        {
            break;
        }
        var _S112 : u32 = atomicLoad(&(bins_0[first_0 + k_0]));
        var sum_3 : u32 = sum_2 + _S112;
        k_0 = k_0 + u32(1);
        sum_2 = sum_3;
    }
    shared_sums_0[t_2] = sum_2;
    workgroupBarrier();
    var offset_0 : u32 = u32(1);
    loop
    {
        if((offset_0 < u32(256)))
        {
        }
        else
        {
            break;
        }
        if((t_2 >= offset_0))
        {
            add_0 = shared_sums_0[t_2 - offset_0];
        }
        else
        {
            add_0 = u32(0);
        }
        workgroupBarrier();
        shared_sums_0[t_2] = shared_sums_0[t_2] + add_0;
        workgroupBarrier();
        offset_0 = offset_0 * u32(2);
    }
    var inclusive_0 : u32 = shared_sums_0[t_2];
    var below_0 : u32 = shared_sums_0[t_2] - sum_2;
    var _S113 : bool;
    if((rank_1 >= below_0))
    {
        _S113 = (rank_1 < inclusive_0);
    }
    else
    {
        _S113 = false;
    }
    if(_S113)
    {
        k_0 = u32(0);
        add_0 = below_0;
        loop
        {
            if((k_0 < per_0))
            {
            }
            else
            {
                bin_2 = first_0;
                break;
            }
            var _S114 : u32 = first_0 + k_0;
            var count_2 : u32 = atomicLoad(&(bins_0[_S114]));
            var _S115 : u32 = add_0 + count_2;
            if((rank_1 < _S115))
            {
                bin_2 = _S114;
                break;
            }
            k_0 = k_0 + u32(1);
            add_0 = _S115;
        }
        var rank_2 : u32 = rank_1 - add_0;
        var prefix_1 : u32 = (((prefix_0 << (_S111))) | (bin_2));
        state_0[u32(0)] = prefix_1;
        state_0[u32(1)] = rank_2;
        if(digit_4 == u32(2))
        {
            var value_2 : f32 = from_order_key_0(prefix_1);
            if((params_0.haze_0.selection_0) == u32(0))
            {
                state_0[u32(2)] = (bitcast<u32>((value_2)));
                var hazy_0 : u32 = params_0.haze_0.width_1 * params_0.haze_0.height_1 - (params_0.haze_0.rank_0 - rank_2);
                state_0[u32(3)] = hazy_0;
                state_0[u32(0)] = u32(0);
                state_0[u32(1)] = u32(f32(hazy_0) * 0.94999998807907104f);
            }
            else
            {
                state_0[u32(4)] = (bitcast<u32>((value_2)));
            }
        }
    }
    k_0 = u32(0);
    loop
    {
        if((k_0 < per_0))
        {
        }
        else
        {
            break;
        }
        atomicStore(&(bins_0[first_0 + k_0]), u32(0));
        k_0 = k_0 + u32(1);
    }
    return;
}

var<workgroup> shared_ambient_0 : array<vec4<f32>, i32(256)>;

var<workgroup> shared_counts_0 : array<u32, i32(256)>;

struct AmbientPartial_0
{
     sum_0 : vec3<f32>,
     count_0 : u32,
};

@compute
@workgroup_size(16, 16, 1)
fn ambient_sums(@builtin(global_invocation_id) id_6 : vec3<u32>, @builtin(workgroup_id) group_0 : vec3<u32>, @builtin(local_invocation_index) t_3 : u32)
{
    var bx_1 : u32;
    var _S116 : vec4<f32> = vec4<f32>(0.0f);
    var _S117 : f32 = (bitcast<f32>((state_0[u32(2)])));
    var _S118 : f32 = (bitcast<f32>((state_0[u32(4)])));
    var by_1 : u32 = u32(0);
    var sum_4 : vec4<f32> = _S116;
    var count_3 : u32 = u32(0);
    loop
    {
        if((by_1 < u32(4)))
        {
        }
        else
        {
            break;
        }
        bx_1 = u32(0);
        var sum_5 : vec4<f32> = sum_4;
        var count_4 : u32 = count_3;
        loop
        {
            if((bx_1 < u32(4)))
            {
            }
            else
            {
                break;
            }
            var x_10 : u32 = u32(4) * id_6.x + bx_1;
            var y_8 : u32 = u32(4) * id_6.y + by_1;
            var _S119 : bool;
            if((x_10 >= (params_0.haze_0.width_1)))
            {
                _S119 = true;
            }
            else
            {
                _S119 = (y_8 >= (params_0.haze_0.height_1));
            }
            if(_S119)
            {
                bx_1 = bx_1 + u32(1);
                continue;
            }
            var i_1 : u32 = pixel_index_0(i32(x_10), i32(y_8));
            var rgb_3 : vec3<f32> = image_0[i_1].xyz;
            var _S120 : bool;
            if((plane_b_0[i_1] >= _S117))
            {
                _S120 = ((brightness_0(rgb_3)) >= _S118);
            }
            else
            {
                _S120 = false;
            }
            var count_5 : u32;
            var sum_6 : vec4<f32>;
            if(_S120)
            {
                var count_6 : u32 = count_4 + u32(1);
                sum_6 = sum_5 + vec4<f32>(rgb_3, 0.0f);
                count_5 = count_6;
            }
            else
            {
                sum_6 = sum_5;
                count_5 = count_4;
            }
            sum_5 = sum_6;
            count_4 = count_5;
            bx_1 = bx_1 + u32(1);
        }
        by_1 = by_1 + u32(1);
        sum_4 = sum_5;
        count_3 = count_4;
    }
    shared_ambient_0[t_3] = sum_4;
    shared_counts_0[t_3] = count_3;
    workgroupBarrier();
    bx_1 = u32(128);
    loop
    {
        if((bx_1 > u32(0)))
        {
        }
        else
        {
            break;
        }
        if((t_3 < bx_1))
        {
            shared_ambient_0[t_3] = shared_ambient_0[t_3] + shared_ambient_0[t_3 + bx_1];
            shared_counts_0[t_3] = shared_counts_0[t_3] + shared_counts_0[t_3 + bx_1];
        }
        workgroupBarrier();
        bx_1 = bx_1 / u32(2);
    }
    if(t_3 == u32(0))
    {
        var partial_0 : AmbientPartial_0;
        partial_0.sum_0 = shared_ambient_0[i32(0)].xyz;
        partial_0.count_0 = shared_counts_0[i32(0)];
        partials_1[group_0.y * params_0.haze_0.partials_across_0 + group_0.x].sum_0 = partial_0.sum_0;
        partials_1[group_0.y * params_0.haze_0.partials_across_0 + group_0.x].count_0 = partial_0.count_0;
    }
    return;
}

fn away_from_zero_0( v_0 : f32,  floor_0 : f32) -> f32
{
    if(((abs(v_0)) < floor_0))
    {
        var _S121 : f32;
        if((v_0 < 0.0f))
        {
            _S121 = - floor_0;
        }
        else
        {
            _S121 = floor_0;
        }
        return _S121;
    }
    return v_0;
}

fn away_from_zero_1( v_1 : vec3<f32>,  floor_1 : f32) -> vec3<f32>
{
    return vec3<f32>(away_from_zero_0(v_1.x, floor_1), away_from_zero_0(v_1.y, floor_1), away_from_zero_0(v_1.z, floor_1));
}

@compute
@workgroup_size(16, 16, 1)
fn ambient_combine(@builtin(local_invocation_index) t_4 : u32)
{
    var _S122 : vec4<f32> = vec4<f32>(0.0f);
    var i_2 : u32 = t_4;
    var sum_7 : vec4<f32> = _S122;
    var count_7 : u32 = u32(0);
    loop
    {
        if((i_2 < (params_0.haze_0.partials_0)))
        {
        }
        else
        {
            break;
        }
        var sum_8 : vec4<f32> = sum_7 + vec4<f32>(partials_1[i_2].sum_0, 0.0f);
        var count_8 : u32 = count_7 + partials_1[i_2].count_0;
        i_2 = i_2 + u32(256);
        sum_7 = sum_8;
        count_7 = count_8;
    }
    shared_ambient_0[t_4] = sum_7;
    shared_counts_0[t_4] = count_7;
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
        if((t_4 < stride_0))
        {
            shared_ambient_0[t_4] = shared_ambient_0[t_4] + shared_ambient_0[t_4 + stride_0];
            shared_counts_0[t_4] = shared_counts_0[t_4] + shared_counts_0[t_4 + stride_0];
        }
        workgroupBarrier();
        stride_0 = stride_0 / u32(2);
    }
    if(t_4 == u32(0))
    {
        var n_0 : u32 = shared_counts_0[i32(0)];
        var ambient_0 : vec3<f32>;
        if(((shared_counts_0[i32(0)]) > u32(0)))
        {
            ambient_0 = shared_ambient_0[i32(0)].xyz / vec3<f32>(f32(n_0));
        }
        else
        {
            ambient_0 = vec3<f32>(0.0f);
        }
        var critical_haze_0 : f32 = (bitcast<f32>((state_0[u32(2)])));
        var depth_0 : f32;
        if((critical_haze_0 > 0.0f))
        {
            depth_0 = -1.125f * log(critical_haze_0);
        }
        else
        {
            depth_0 = params_0.haze_0.max_depth_0;
        }
        var inverse_0 : vec3<f32> = vec3<f32>(1.0f) / away_from_zero_1(ambient_0, 1.52587890625e-05f);
        state_0[u32(5)] = n_0;
        state_0[u32(8)] = (bitcast<u32>((ambient_0.x)));
        state_0[u32(9)] = (bitcast<u32>((ambient_0.y)));
        state_0[u32(10)] = (bitcast<u32>((ambient_0.z)));
        state_0[u32(11)] = (bitcast<u32>((depth_0)));
        state_0[u32(12)] = (bitcast<u32>((inverse_0.x)));
        state_0[u32(13)] = (bitcast<u32>((inverse_0.y)));
        state_0[u32(14)] = (bitcast<u32>((inverse_0.z)));
    }
    return;
}

struct GuideMeans_0
{
     first_1 : vec4<f32>,
     second_0 : vec4<f32>,
     third_0 : vec4<f32>,
     blue_blue_0 : f32,
};

fn no_means_0() -> GuideMeans_0
{
    var m_3 : GuideMeans_0;
    var _S123 : vec4<f32> = vec4<f32>(0.0f);
    m_3.first_1 = _S123;
    m_3.second_0 = _S123;
    m_3.third_0 = _S123;
    m_3.blue_blue_0 = 0.0f;
    return m_3;
}

fn chunk_pixels_count_0() -> u32
{
    return u32(8) * (u32(16) + u32(2) * params_0.haze_0.guide_radius_0);
}

struct ChunkPixel_0
{
     slot_0 : u32,
     x_11 : i32,
     y_9 : i32,
     inside_0 : bool,
};

fn chunk_pixel_0( origin_0 : vec2<i32>,  chunk_0 : i32,  e_0 : u32) -> ChunkPixel_0
{
    var r_3 : i32 = i32(params_0.haze_0.guide_radius_0);
    var span_0 : i32 = i32(16) + i32(2) * r_3;
    var _S124 : i32 = i32(e_0);
    var row_0 : i32 = _S124 / span_0;
    var column_0 : i32 = _S124 % span_0;
    var pixel_1 : ChunkPixel_0;
    pixel_1.slot_0 = u32(row_0) * u32(34) + u32(column_0);
    pixel_1.x_11 = origin_0.x - r_3 + column_0;
    pixel_1.y_9 = origin_0.y - r_3 + chunk_0 + row_0;
    var _S125 : bool;
    if(((chunk_0 + row_0) < span_0))
    {
        _S125 = ((pixel_1.x_11) >= i32(0));
    }
    else
    {
        _S125 = false;
    }
    if(_S125)
    {
        _S125 = ((pixel_1.x_11) < i32(params_0.haze_0.width_1));
    }
    else
    {
        _S125 = false;
    }
    if(_S125)
    {
        _S125 = ((pixel_1.y_9) >= i32(0));
    }
    else
    {
        _S125 = false;
    }
    if(_S125)
    {
        _S125 = ((pixel_1.y_9) < i32(params_0.haze_0.height_1));
    }
    else
    {
        _S125 = false;
    }
    pixel_1.inside_0 = _S125;
    return pixel_1;
}

var<workgroup> chunk_pixels_0 : array<vec4<f32>, i32(272)>;

fn load_guide_chunk_0( origin_1 : vec2<i32>,  chunk_1 : i32,  t_5 : u32)
{
    var e_1 : u32 = t_5;
    loop
    {
        if((e_1 < (chunk_pixels_count_0())))
        {
        }
        else
        {
            break;
        }
        var pixel_2 : ChunkPixel_0 = chunk_pixel_0(origin_1, chunk_1, e_1);
        var _S126 : vec4<f32> = vec4<f32>(0.0f);
        var value_3 : vec4<f32>;
        if(pixel_2.inside_0)
        {
            var i_3 : u32 = pixel_index_0(pixel_2.x_11, pixel_2.y_9);
            value_3 = vec4<f32>(image_0[i_3].xyz, plane_b_0[i_3]);
        }
        else
        {
            value_3 = _S126;
        }
        chunk_pixels_0[pixel_2.slot_0] = value_3;
        e_1 = e_1 + u32(256);
    }
    return;
}

struct ChunkEntry_0
{
     row_1 : u32,
     first_2 : i32,
     last_0 : i32,
     needed_0 : bool,
};

fn chunk_entry_0( origin_2 : vec2<i32>,  chunk_2 : i32,  t_6 : u32) -> ChunkEntry_0
{
    var r_4 : i32 = i32(params_0.haze_0.guide_radius_0);
    var row_2 : i32 = i32(t_6 / u32(16));
    var _S127 : i32 = origin_2.x;
    var x_12 : i32 = _S127 + i32(t_6 % u32(16));
    var y_10 : i32 = origin_2.y - r_4 + chunk_2 + row_2;
    var entry_0 : ChunkEntry_0;
    entry_0.row_1 = u32(row_2);
    var _S128 : i32 = _S127 - r_4;
    entry_0.first_2 = max(i32(0), x_12 - r_4) - _S128;
    entry_0.last_0 = min(i32(params_0.haze_0.width_1) - i32(1), x_12 + r_4) - _S128;
    var _S129 : bool;
    if((t_6 < u32(128)))
    {
        _S129 = ((chunk_2 + row_2) < (i32(16) + i32(2) * r_4));
    }
    else
    {
        _S129 = false;
    }
    if(_S129)
    {
        _S129 = (x_12 < i32(params_0.haze_0.width_1));
    }
    else
    {
        _S129 = false;
    }
    if(_S129)
    {
        _S129 = (y_10 >= i32(0));
    }
    else
    {
        _S129 = false;
    }
    if(_S129)
    {
        _S129 = (y_10 < i32(params_0.haze_0.height_1));
    }
    else
    {
        _S129 = false;
    }
    entry_0.needed_0 = _S129;
    return entry_0;
}

fn guide_products_0( pixel_3 : vec4<f32>) -> GuideMeans_0
{
    var guide_0 : vec3<f32> = pixel_3.xyz;
    var p_0 : f32 = pixel_3.w;
    var m_4 : GuideMeans_0;
    var _S130 : f32 = guide_0.x;
    var _S131 : f32 = guide_0.y;
    var _S132 : f32 = guide_0.z;
    m_4.first_1 = vec4<f32>(p_0, _S130, _S131, _S132);
    m_4.second_0 = vec4<f32>(_S130 * p_0, _S131 * p_0, _S132 * p_0, _S130 * _S130);
    m_4.third_0 = vec4<f32>(_S130 * _S131, _S130 * _S132, _S131 * _S131, _S131 * _S132);
    m_4.blue_blue_0 = _S132 * _S132;
    return m_4;
}

fn add_means_0( a_1 : GuideMeans_0,  b_1 : GuideMeans_0) -> GuideMeans_0
{
    var m_5 : GuideMeans_0;
    m_5.first_1 = a_1.first_1 + b_1.first_1;
    m_5.second_0 = a_1.second_0 + b_1.second_0;
    m_5.third_0 = a_1.third_0 + b_1.third_0;
    m_5.blue_blue_0 = a_1.blue_blue_0 + b_1.blue_blue_0;
    return m_5;
}

fn divide_means_0( a_2 : GuideMeans_0,  n_1 : f32) -> GuideMeans_0
{
    var m_6 : GuideMeans_0;
    var _S133 : vec4<f32> = vec4<f32>(n_1);
    m_6.first_1 = a_2.first_1 / _S133;
    m_6.second_0 = a_2.second_0 / _S133;
    m_6.third_0 = a_2.third_0 / _S133;
    m_6.blue_blue_0 = a_2.blue_blue_0 / n_1;
    return m_6;
}

var<workgroup> chunk_first_0 : array<vec4<f32>, i32(128)>;

var<workgroup> chunk_second_0 : array<vec4<f32>, i32(128)>;

var<workgroup> chunk_third_0 : array<vec4<f32>, i32(128)>;

var<workgroup> chunk_blue_0 : array<f32, i32(128)>;

fn in_window_0( y_11 : i32,  y_out_0 : i32) -> bool
{
    var _S134 : bool;
    if(((abs(y_11 - y_out_0)) <= i32(params_0.haze_0.guide_radius_0)))
    {
        _S134 = (y_11 >= i32(0));
    }
    else
    {
        _S134 = false;
    }
    if(_S134)
    {
        _S134 = (y_11 < i32(params_0.haze_0.height_1));
    }
    else
    {
        _S134 = false;
    }
    return _S134;
}

fn vertical_hits_0( y_out_1 : i32) -> f32
{
    var r_5 : i32 = i32(params_0.haze_0.guide_radius_0);
    return f32(max(min(i32(params_0.haze_0.height_1) - i32(1), y_out_1 + r_5) - max(i32(0), y_out_1 - r_5) + i32(1), i32(1)));
}

fn guide_means_0( origin_3 : vec2<i32>,  local_0 : vec2<i32>,  t_7 : u32) -> GuideMeans_0
{
    var _S135 : i32;
    var _S136 : i32 = i32(params_0.haze_0.guide_radius_0);
    var _S137 : i32 = origin_3.y;
    var y_out_2 : i32 = _S137 + local_0.y;
    var _S138 : GuideMeans_0 = no_means_0();
    var sum_9 : GuideMeans_0 = _S138;
    var chunk_3 : i32 = i32(0);
    loop
    {
        if((chunk_3 < (i32(16) + i32(2) * _S136)))
        {
        }
        else
        {
            break;
        }
        load_guide_chunk_0(origin_3, chunk_3, t_7);
        workgroupBarrier();
        var entry_1 : ChunkEntry_0 = chunk_entry_0(origin_3, chunk_3, t_7);
        if((t_7 < u32(128)))
        {
            var row_3 : GuideMeans_0;
            if(entry_1.needed_0)
            {
                var _S139 : u32 = entry_1.row_1 * u32(34);
                row_3 = _S138;
                var column_1 : i32 = entry_1.first_2;
                loop
                {
                    _S135 = entry_1.last_0;
                    if((column_1 <= (entry_1.last_0)))
                    {
                    }
                    else
                    {
                        break;
                    }
                    var _S140 : GuideMeans_0 = add_means_0(row_3, guide_products_0(chunk_pixels_0[_S139 + u32(column_1)]));
                    var _S141 : i32 = column_1 + i32(1);
                    row_3 = _S140;
                    column_1 = _S141;
                }
                row_3 = divide_means_0(row_3, f32(_S135 - entry_1.first_2 + i32(1)));
            }
            else
            {
                row_3 = _S138;
            }
            chunk_first_0[t_7] = row_3.first_1;
            chunk_second_0[t_7] = row_3.second_0;
            chunk_third_0[t_7] = row_3.third_0;
            chunk_blue_0[t_7] = row_3.blue_blue_0;
        }
        workgroupBarrier();
        var k_1 : u32 = u32(0);
        loop
        {
            if((k_1 < u32(8)))
            {
            }
            else
            {
                break;
            }
            if(in_window_0(_S137 - _S136 + chunk_3 + i32(k_1), y_out_2))
            {
                var e_2 : u32 = k_1 * u32(16) + u32(local_0.x);
                sum_9.first_1 = sum_9.first_1 + chunk_first_0[e_2];
                sum_9.second_0 = sum_9.second_0 + chunk_second_0[e_2];
                sum_9.third_0 = sum_9.third_0 + chunk_third_0[e_2];
                sum_9.blue_blue_0 = sum_9.blue_blue_0 + chunk_blue_0[e_2];
            }
            k_1 = k_1 + u32(1);
        }
        workgroupBarrier();
        chunk_3 = chunk_3 + i32(8);
    }
    return divide_means_0(sum_9, vertical_hits_0(y_out_2));
}

fn guided_coefficients_0( m_7 : GuideMeans_0,  eps_1 : f32) -> vec4<f32>
{
    var inp_mean_0 : f32 = m_7.first_1.x;
    var guide_r_0 : f32 = m_7.first_1.y;
    var guide_g_0 : f32 = m_7.first_1.z;
    var guide_b_0 : f32 = m_7.first_1.w;
    var sigma_0_0_0 : f32 = m_7.second_0.w - guide_r_0 * guide_r_0 + eps_1;
    var sigma_0_1_0 : f32 = m_7.third_0.x - guide_r_0 * guide_g_0;
    var sigma_0_2_0 : f32 = m_7.third_0.y - guide_r_0 * guide_b_0;
    var sigma_1_1_0 : f32 = m_7.third_0.z - guide_g_0 * guide_g_0 + eps_1;
    var sigma_1_2_0 : f32 = m_7.third_0.w - guide_g_0 * guide_b_0;
    var sigma_2_2_0 : f32 = m_7.blue_blue_0 - guide_b_0 * guide_b_0 + eps_1;
    var _S142 : f32 = sigma_1_1_0 * sigma_2_2_0 - sigma_1_2_0 * sigma_1_2_0;
    var _S143 : f32 = sigma_0_1_0 * sigma_2_2_0 - sigma_0_2_0 * sigma_1_2_0;
    var _S144 : f32 = sigma_0_1_0 * sigma_1_2_0 - sigma_0_2_0 * sigma_1_1_0;
    var det0_0 : f32 = sigma_0_0_0 * _S142 - sigma_0_1_0 * _S143 + sigma_0_2_0 * _S144;
    if(((abs(det0_0)) > 4.76837158203125e-07f))
    {
        var cov_r_0 : f32 = m_7.second_0.x - guide_r_0 * inp_mean_0;
        var cov_g_0 : f32 = m_7.second_0.y - guide_g_0 * inp_mean_0;
        var cov_b_0 : f32 = m_7.second_0.z - guide_b_0 * inp_mean_0;
        var _S145 : f32 = cov_g_0 * sigma_2_2_0 - cov_b_0 * sigma_1_2_0;
        var _S146 : f32 = sigma_0_1_0 * cov_b_0 - sigma_0_2_0 * cov_g_0;
        var a_r_0 : f32 = (cov_r_0 * _S142 - sigma_0_1_0 * _S145 + sigma_0_2_0 * (cov_g_0 * sigma_1_2_0 - cov_b_0 * sigma_1_1_0)) / det0_0;
        var a_g_0 : f32 = (sigma_0_0_0 * _S145 - cov_r_0 * _S143 + sigma_0_2_0 * _S146) / det0_0;
        var a_b_0 : f32 = (sigma_0_0_0 * (sigma_1_1_0 * cov_b_0 - sigma_1_2_0 * cov_g_0) - sigma_0_1_0 * _S146 + cov_r_0 * _S144) / det0_0;
        return vec4<f32>(a_r_0, a_g_0, a_b_0, inp_mean_0 - a_r_0 * guide_r_0 - a_g_0 * guide_g_0 - a_b_0 * guide_b_0);
    }
    return vec4<f32>(0.0f, 0.0f, 0.0f, inp_mean_0);
}

@compute
@workgroup_size(16, 16, 1)
fn guide_coefficients(@builtin(workgroup_id) group_1 : vec3<u32>, @builtin(local_invocation_id) local_1 : vec3<u32>, @builtin(local_invocation_index) t_8 : u32)
{
    var _S147 : vec2<i32> = vec2<i32>(group_1.xy * vec2<u32>(u32(16)));
    var _S148 : vec2<i32> = vec2<i32>(local_1.xy);
    var means_0 : GuideMeans_0 = guide_means_0(_S147, _S148, t_8);
    var x_13 : i32 = _S147.x + _S148.x;
    var y_12 : i32 = _S147.y + _S148.y;
    var _S149 : bool;
    if((x_13 < i32(params_0.haze_0.width_1)))
    {
        _S149 = (y_12 < i32(params_0.haze_0.height_1));
    }
    else
    {
        _S149 = false;
    }
    if(_S149)
    {
        coefficients_0[pixel_index_0(x_13, y_12)] = guided_coefficients_0(means_0, params_0.haze_0.eps_0);
    }
    return;
}

fn load_coefficient_chunk_0( origin_4 : vec2<i32>,  chunk_4 : i32,  t_9 : u32)
{
    var e_3 : u32 = t_9;
    loop
    {
        if((e_3 < (chunk_pixels_count_0())))
        {
        }
        else
        {
            break;
        }
        var pixel_4 : ChunkPixel_0 = chunk_pixel_0(origin_4, chunk_4, e_3);
        var _S150 : vec4<f32> = vec4<f32>(0.0f);
        var value_4 : vec4<f32>;
        if(pixel_4.inside_0)
        {
            value_4 = coefficients_0[pixel_index_0(pixel_4.x_11, pixel_4.y_9)];
        }
        else
        {
            value_4 = _S150;
        }
        chunk_pixels_0[pixel_4.slot_0] = value_4;
        e_3 = e_3 + u32(256);
    }
    return;
}

fn coefficient_means_0( origin_5 : vec2<i32>,  local_2 : vec2<i32>,  t_10 : u32) -> vec4<f32>
{
    var _S151 : i32;
    var _S152 : i32 = i32(params_0.haze_0.guide_radius_0);
    var _S153 : i32 = origin_5.y;
    var y_out_3 : i32 = _S153 + local_2.y;
    var _S154 : vec4<f32> = vec4<f32>(0.0f);
    var chunk_5 : i32 = i32(0);
    var sum_10 : vec4<f32> = _S154;
    loop
    {
        if((chunk_5 < (i32(16) + i32(2) * _S152)))
        {
        }
        else
        {
            break;
        }
        load_coefficient_chunk_0(origin_5, chunk_5, t_10);
        workgroupBarrier();
        var entry_2 : ChunkEntry_0 = chunk_entry_0(origin_5, chunk_5, t_10);
        var sum_11 : vec4<f32>;
        if((t_10 < u32(128)))
        {
            if(entry_2.needed_0)
            {
                var _S155 : u32 = entry_2.row_1 * u32(34);
                var column_2 : i32 = entry_2.first_2;
                sum_11 = _S154;
                loop
                {
                    _S151 = entry_2.last_0;
                    if((column_2 <= (entry_2.last_0)))
                    {
                    }
                    else
                    {
                        break;
                    }
                    var row_4 : vec4<f32> = sum_11 + chunk_pixels_0[_S155 + u32(column_2)];
                    column_2 = column_2 + i32(1);
                    sum_11 = row_4;
                }
                sum_11 = sum_11 / vec4<f32>(f32(_S151 - entry_2.first_2 + i32(1)));
            }
            else
            {
                sum_11 = _S154;
            }
            chunk_first_0[t_10] = sum_11;
        }
        workgroupBarrier();
        var k_2 : u32 = u32(0);
        sum_11 = sum_10;
        loop
        {
            if((k_2 < u32(8)))
            {
            }
            else
            {
                break;
            }
            if(in_window_0(_S153 - _S152 + chunk_5 + i32(k_2), y_out_3))
            {
                sum_11 = sum_11 + chunk_first_0[k_2 * u32(16) + u32(local_2.x)];
            }
            k_2 = k_2 + u32(1);
        }
        workgroupBarrier();
        chunk_5 = chunk_5 + i32(8);
        sum_10 = sum_11;
    }
    return sum_10 / vec4<f32>(vertical_hits_0(y_out_3));
}

@compute
@workgroup_size(16, 16, 1)
fn guide_result(@builtin(workgroup_id) group_2 : vec3<u32>, @builtin(local_invocation_id) local_3 : vec3<u32>, @builtin(local_invocation_index) t_11 : u32)
{
    var _S156 : vec2<i32> = vec2<i32>(group_2.xy * vec2<u32>(u32(16)));
    var _S157 : vec2<i32> = vec2<i32>(local_3.xy);
    var mean_0 : vec4<f32> = coefficient_means_0(_S156, _S157, t_11);
    var x_14 : i32 = _S156.x + _S157.x;
    var y_13 : i32 = _S156.y + _S157.y;
    var _S158 : bool;
    if((x_14 < i32(params_0.haze_0.width_1)))
    {
        _S158 = (y_13 < i32(params_0.haze_0.height_1));
    }
    else
    {
        _S158 = false;
    }
    if(_S158)
    {
        var i_4 : u32 = pixel_index_0(x_14, y_13);
        var guide_1 : vec3<f32> = image_0[i_4].xyz;
        plane_a_0[i_4] = mean_0.x * guide_1.x + mean_0.y * guide_1.y + mean_0.z * guide_1.z + mean_0.w;
    }
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn plane_downscale(@builtin(global_invocation_id) id_7 : vec3<u32>)
{
    var _S159 : u32 = id_7.x;
    var _S160 : bool;
    if((_S159 >= (params_0.pass_0.width_0)))
    {
        _S160 = true;
    }
    else
    {
        _S160 = ((id_7.y) >= (params_0.pass_0.height_0));
    }
    if(_S160)
    {
        return;
    }
    var _S161 : f32 = footprint_mean_0(id_7.xy, vec2<u32>(params_0.haze_0.width_1, params_0.haze_0.height_1), vec2<u32>(params_0.pass_0.width_0, params_0.pass_0.height_0));
    viewport_plane_0[id_7.y * params_0.pass_0.width_0 + _S159] = _S161;
    return;
}

fn ambient_light_0() -> vec3<f32>
{
    return vec3<f32>((bitcast<f32>((state_0[u32(8)]))), (bitcast<f32>((state_0[u32(9)]))), (bitcast<f32>((state_0[u32(10)]))));
}

fn dehaze_1( rgb_4 : vec3<f32>,  guided_0 : f32,  ambient_1 : vec3<f32>,  depth_1 : f32,  d_0 : ptr<function, Dehaze_std140_0>) -> vec3<f32>
{
    return (rgb_4 - ambient_1) / vec3<f32>(max(1.0f - (*d_0).strength_0 * guided_0, clamp(exp(clamp(- (*d_0).distance_0 * depth_1, -80.0f, 80.0f)), 0.0009765625f, 1.0f))) + ambient_1;
}

struct DiffPair_vectorx3Cfloatx2C3x3E_0
{
     primal_0 : vec3<f32>,
     differential_0 : vec3<f32>,
};

fn s_fwd_dehaze_0( rgb_5 : vec3<f32>,  guided_1 : f32,  ambient_2 : vec3<f32>,  depth_2 : f32,  dpd_0 : DiffPair_Dehaze_0) -> DiffPair_vectorx3Cfloatx2C3x3E_0
{
    var _S162 : DiffPair_float_0 = DiffPair_float_0( - dpd_0.primal_0.distance_0 * depth_2, - dpd_0.differential_0.distance_0 * depth_2 );
    var _S163 : DiffPair_float_0 = DiffPair_float_0( -80.0f, 0.0f );
    var _S164 : DiffPair_float_0 = DiffPair_float_0( 80.0f, 0.0f );
    var _S165 : DiffPair_float_0 = _d_clamp_1(_S162, _S163, _S164);
    var _S166 : DiffPair_float_0 = DiffPair_float_0( _S165.primal_0, _S165.differential_0 );
    var _S167 : DiffPair_float_0 = _d_exp_1(_S166);
    var _S168 : DiffPair_float_0 = DiffPair_float_0( _S167.primal_0, _S167.differential_0 );
    var _S169 : DiffPair_float_0 = DiffPair_float_0( 0.0009765625f, 0.0f );
    var _S170 : DiffPair_float_0 = DiffPair_float_0( 1.0f, 0.0f );
    var _S171 : DiffPair_float_0 = _d_clamp_1(_S168, _S169, _S170);
    var _S172 : DiffPair_float_0 = DiffPair_float_0( 1.0f - dpd_0.primal_0.strength_0 * guided_1, 0.0f - dpd_0.differential_0.strength_0 * guided_1 );
    var _S173 : DiffPair_float_0 = DiffPair_float_0( _S171.primal_0, _S171.differential_0 );
    var _S174 : DiffPair_float_0 = _d_max_1(_S172, _S173);
    var _S175 : vec3<f32> = rgb_5 - ambient_2;
    var _S176 : DiffPair_vectorx3Cfloatx2C3x3E_0 = DiffPair_vectorx3Cfloatx2C3x3E_0( _S175 / vec3<f32>(_S174.primal_0) + ambient_2, (vec3<f32>(0.0f) - _S175 * vec3<f32>(_S174.differential_0)) / vec3<f32>((_S174.primal_0 * _S174.primal_0)) );
    return _S176;
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id_8 : vec3<u32>)
{
    var _S177 : u32 = id_8.x;
    var _S178 : bool;
    if((_S177 >= (params_0.pass_0.width_0)))
    {
        _S178 = true;
    }
    else
    {
        _S178 = ((id_8.y) >= (params_0.pass_0.height_0));
    }
    if(_S178)
    {
        return;
    }
    var i_5 : u32 = id_8.y * params_0.pass_0.width_0 + _S177;
    var _S179 : f32 = plane_a_0[whole_index_0(id_8)];
    var _S180 : vec3<f32> = ambient_light_0();
    var _S181 : f32 = (bitcast<f32>((state_0[u32(11)])));
    var pixel_5 : vec4<f32> = input_0[i_5];
    var rgb_6 : vec3<f32> = pixel_5.xyz;
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
        var _S182 : Dehaze_std140_0 = params_0.haze_0.dehaze_0;
        var _S183 : vec3<f32> = dehaze_1(rgb_6, _S179, _S180, _S181, &(_S182));
        var _S184 : u32 = k_3 + u32(1);
        rgb_6 = _S183;
        k_3 = _S184;
    }
    output_0[i_5] = vec4<f32>(rgb_6, pixel_5.w);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn hazeremoval_backward(@builtin(global_invocation_id) id_9 : vec3<u32>)
{
    var _S185 : u32 = id_9.x;
    var _S186 : bool;
    if((_S185 >= (params_0.pass_0.width_0)))
    {
        _S186 = true;
    }
    else
    {
        _S186 = ((id_9.y) >= (params_0.pass_0.height_0));
    }
    if(_S186)
    {
        return;
    }
    var i_6 : u32 = id_9.y * params_0.pass_0.width_0 + _S185;
    var _S187 : Dehaze_0 = Dehaze_0( params_0.haze_0.dehaze_0.strength_0, params_0.haze_0.dehaze_0.distance_0 );
    var _S188 : Dehaze_0 = Dehaze_x24_syn_dzero_0();
    var d_1 : DiffPair_Dehaze_0;
    d_1.primal_0 = _S187;
    d_1.differential_0 = _S188;
    _S52(input_0[i_6].xyz, plane_a_0[whole_index_0(id_9)], ambient_light_0(), (bitcast<f32>((state_0[u32(11)]))), &(d_1), derivative_in_0[i_6].xyz);
    derivative_out_0[i_6] = vec4<f32>(d_1.differential_0.strength_0, d_1.differential_0.distance_0, 0.0f, 0.0f);
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn hazeremoval_forward_diff(@builtin(global_invocation_id) id_10 : vec3<u32>)
{
    var _S189 : u32 = id_10.x;
    var _S190 : bool;
    if((_S189 >= (params_0.pass_0.width_0)))
    {
        _S190 = true;
    }
    else
    {
        _S190 = ((id_10.y) >= (params_0.pass_0.height_0));
    }
    if(_S190)
    {
        return;
    }
    var i_7 : u32 = id_10.y * params_0.pass_0.width_0 + _S189;
    var direction_0 : Dehaze_0;
    direction_0.strength_0 = derivative_in_0[i_7].x;
    direction_0.distance_0 = derivative_in_0[i_7].y;
    var _S191 : Dehaze_0 = Dehaze_0( params_0.haze_0.dehaze_0.strength_0, params_0.haze_0.dehaze_0.distance_0 );
    var _S192 : DiffPair_Dehaze_0 = DiffPair_Dehaze_0( _S191, direction_0 );
    derivative_out_0[i_7] = vec4<f32>(s_fwd_dehaze_0(input_0[i_7].xyz, plane_a_0[whole_index_0(id_10)], ambient_light_0(), (bitcast<f32>((state_0[u32(11)]))), _S192).differential_0, 0.0f);
    return;
}

