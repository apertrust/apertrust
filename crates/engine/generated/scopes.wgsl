// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
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

// Generated from slang/scopes.slang by tools/build-kernels.sh. Do not edit.

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

struct ScopesParams_std140_0
{
    @align(16) histogram_0 : u32,
    @align(4) waveform_0 : u32,
    @align(8) vectorscope_0 : u32,
    @align(4) _pad0_0 : u32,
    @align(16) to_xyz_0 : array<vec4<f32>, i32(3)>,
    @align(16) top_0 : f32,
    @align(4) exponent_0 : f32,
    @align(8) radius_0 : f32,
    @align(4) _pad1_0 : f32,
};

struct Params_std140_0
{
    @align(16) pass_0 : PassHeader_std140_0,
    @align(16) scopes_0 : ScopesParams_std140_0,
};

@binding(2) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(0) @group(0) var<storage, read> frame_0 : array<vec4<f32>>;

@binding(1) @group(0) var<storage, read_write> counts_0 : array<atomic<u32>>;

var<workgroup> shared_counts_0 : array<atomic<u32>, i32(1028)>;

fn pow0_0( x_0 : f32,  y_0 : f32) -> f32
{
    if((x_0 <= 0.0f))
    {
        return 0.0f;
    }
    return pow(x_0, y_0);
}

fn srgb_to_linear_0( v_0 : f32) -> f32
{
    var _S1 : f32;
    if((v_0 <= 0.040449999272823334f))
    {
        _S1 = v_0 / 12.920000076293945f;
    }
    else
    {
        _S1 = pow0_0((v_0 + 0.054999999701976776f) / 1.0549999475479126f, 2.4000000953674316f);
    }
    return _S1;
}

fn linear_to_srgb_0( v_1 : f32) -> f32
{
    var _S2 : f32 = max(v_1, 0.0f);
    var _S3 : f32;
    if((_S2 <= 0.0031308000907301903f))
    {
        _S3 = 12.920000076293945f * _S2;
    }
    else
    {
        _S3 = 1.0549999475479126f * pow0_0(_S2, 0.4166666567325592f) - 0.054999999701976776f;
    }
    return _S3;
}

fn luminance_0( rgb_0 : vec3<f32>) -> f32
{
    var _S4 : f32 = rgb_0.x;
    var _S5 : f32 = rgb_0.y;
    var g_0 : f32 = srgb_to_linear_0(_S5);
    var _S6 : f32 = rgb_0.z;
    var encoded_0 : f32 = linear_to_srgb_0(g_0 + 0.2125999927520752f * (srgb_to_linear_0(_S4) - g_0) + 0.072200000286102295f * (srgb_to_linear_0(_S6) - g_0));
    var _S7 : bool;
    if(_S4 == _S5)
    {
        _S7 = _S5 == _S6;
    }
    else
    {
        _S7 = false;
    }
    var _S8 : f32;
    if(_S7)
    {
        _S8 = _S5;
    }
    else
    {
        _S8 = encoded_0;
    }
    return _S8;
}

fn bin_0( v_2 : f32,  count_0 : u32) -> u32
{
    var _S9 : u32 = count_0 - u32(1);
    return min(u32(min(max(v_2 * f32(count_0), 0.0f), f32(_S9))), _S9);
}

fn away_from_zero_0( v_3 : f32,  floor_0 : f32) -> f32
{
    if(((abs(v_3)) < floor_0))
    {
        var _S10 : f32;
        if((v_3 < 0.0f))
        {
            _S10 = - floor_0;
        }
        else
        {
            _S10 = floor_0;
        }
        return _S10;
    }
    return v_3;
}

fn xy_to_uv_0( xy_0 : vec2<f32>) -> vec2<f32>
{
    var _S11 : f32 = xy_0.x;
    var _S12 : f32 = xy_0.y;
    return vec2<f32>(4.0f * _S11, 9.0f * _S12) / vec2<f32>(away_from_zero_0(-2.0f * _S11 + 12.0f * _S12 + 3.0f, 1.52587890625e-05f * (2.0f * abs(_S11) + 12.0f * abs(_S12) + 3.0f)));
}

fn log_scale_0( c_0 : vec2<f32>,  radius_1 : f32) -> vec2<f32>
{
    var _S13 : f32 = c_0.x;
    var _S14 : f32 = c_0.y;
    var h_0 : f32 = sqrt(_S13 * _S13 + _S14 * _S14);
    var _S15 : vec2<f32>;
    if((h_0 >= 1.17549435082228751e-38f))
    {
        _S15 = c_0 * vec2<f32>((log(1.0f + 29.0f * h_0 / radius_1) / log(30.0f) * radius_1 / h_0));
    }
    else
    {
        _S15 = c_0;
    }
    return _S15;
}

fn profile_linear_0( _S16 : f32) -> f32
{
    var _S17 : f32;
    if((_S16 < 1.0f))
    {
        _S17 = srgb_to_linear_0(max(_S16, 0.0f));
    }
    else
    {
        _S17 = params_0.scopes_0.top_0 * pow0_0(_S16, params_0.scopes_0.exponent_0);
    }
    return _S17;
}

fn chroma_0( _S18 : vec3<f32>) -> vec2<f32>
{
    var linear_0 : vec3<f32> = vec3<f32>(profile_linear_0(_S18.x), profile_linear_0(_S18.y), profile_linear_0(_S18.z));
    var _S19 : vec3<f32> = max(vec3<f32>(dot(params_0.scopes_0.to_xyz_0[i32(0)].xyz, linear_0), dot(params_0.scopes_0.to_xyz_0[i32(1)].xyz, linear_0), dot(params_0.scopes_0.to_xyz_0[i32(2)].xyz, linear_0)), vec3<f32>(0.0f));
    var _S20 : f32 = _S19.y;
    var sum_0 : f32 = _S19.x + _S20 + _S19.z;
    var xy_1 : vec2<f32>;
    if((sum_0 > 0.0f))
    {
        xy_1 = _S19.xy / vec2<f32>(sum_0);
    }
    else
    {
        xy_1 = vec2<f32>(0.34567001461982727f, 0.35850000381469727f);
    }
    var uv_0 : vec2<f32> = xy_to_uv_0(xy_1);
    var lstar_0 : f32;
    if((_S20 <= 0.0088564520701766014f))
    {
        lstar_0 = 903.29638671875f * _S20;
    }
    else
    {
        lstar_0 = 116.0f * pow0_0(_S20, 0.3333333432674408f) - 16.0f;
    }
    return vec2<f32>((13.0f * lstar_0)) * (uv_0 - vec2<f32>(0.20915915071964264f, 0.48807531595230103f));
}

fn count_vectorscope_0( _S21 : vec3<f32>)
{
    var at_0 : vec2<f32> = vec2<f32>(127.0f) * (log_scale_0(chroma_0(_S21), params_0.scopes_0.radius_0) / vec2<f32>((2.0f * params_0.scopes_0.radius_0)) + vec2<f32>(0.5f));
    var _S22 : bool;
    if((all(((at_0 > vec2<f32>(-1.0f))))))
    {
        _S22 = (all(((at_0 < vec2<f32>(128.0f)))));
    }
    else
    {
        _S22 = false;
    }
    if(_S22)
    {
        var _S23 : vec2<u32> = min(vec2<u32>(max(at_0, vec2<f32>(0.0f))), vec2<u32>(u32(127)));
        var _S24 : u32 = atomicAdd(&(counts_0[u32(1028) + (u32(127) - _S23.x) * u32(128) + (u32(127) - _S23.y)]), u32(1));
    }
    return;
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(workgroup_id) group_0 : vec3<u32>, @builtin(local_invocation_id) local_0 : vec3<u32>, @builtin(local_invocation_index) index_0 : u32)
{
    var bx_0 : u32;
    var _S25 : bool;
    var i_0 : u32 = index_0;
    loop
    {
        if((i_0 < u32(1028)))
        {
        }
        else
        {
            break;
        }
        atomicStore(&(shared_counts_0[i_0]), u32(0));
        i_0 = i_0 + u32(256);
    }
    workgroupBarrier();
    var _S26 : u32 = params_0.pass_0.width_0;
    var _S27 : u32 = params_0.pass_0.height_0;
    var _S28 : bool = (params_0.scopes_0.histogram_0) != u32(0);
    var _S29 : bool = (params_0.scopes_0.waveform_0) != u32(0);
    var _S30 : vec2<u32> = group_0.xy;
    var _S31 : vec2<u32> = local_0.xy;
    var _S32 : vec2<u32> = _S30 * vec2<u32>(u32(64)) + _S31;
    var by_0 : u32 = u32(0);
    var pixels_0 : u32 = u32(0);
    var shadows_0 : u32 = u32(0);
    var highlights_0 : u32 = u32(0);
    loop
    {
        if((by_0 < u32(4)))
        {
        }
        else
        {
            break;
        }
        bx_0 = u32(0);
        var pixels_1 : u32 = pixels_0;
        var shadows_1 : u32 = shadows_0;
        var highlights_1 : u32 = highlights_0;
        loop
        {
            if((bx_0 < u32(4)))
            {
            }
            else
            {
                break;
            }
            var at_1 : vec2<u32> = _S32 + vec2<u32>(bx_0, by_0) * vec2<u32>(u32(16));
            var _S33 : u32 = at_1.x;
            if((_S33 < _S26))
            {
                _S25 = ((at_1.y) < _S27);
            }
            else
            {
                _S25 = false;
            }
            if(_S25)
            {
                var rgb_1 : vec3<f32> = frame_0[at_1.y * _S26 + _S33].xyz;
                var y_1 : f32 = luminance_0(rgb_1);
                var pixels_2 : u32 = pixels_1 + u32(1);
                var _S34 : i32;
                if((any(((rgb_1 <= vec3<f32>(0.0f))))))
                {
                    _S34 = i32(1);
                }
                else
                {
                    _S34 = i32(0);
                }
                var shadows_2 : u32 = shadows_1 + u32(_S34);
                var _S35 : i32;
                if((any(((rgb_1 >= vec3<f32>(1.0f))))))
                {
                    _S35 = i32(1);
                }
                else
                {
                    _S35 = i32(0);
                }
                var highlights_2 : u32 = highlights_1 + u32(_S35);
                if(_S28)
                {
                    var _S36 : u32 = atomicAdd(&(shared_counts_0[u32(4) + bin_0(rgb_1.x, u32(256))]), u32(1));
                    var _S37 : u32 = atomicAdd(&(shared_counts_0[u32(260) + bin_0(rgb_1.y, u32(256))]), u32(1));
                    var _S38 : u32 = atomicAdd(&(shared_counts_0[u32(516) + bin_0(rgb_1.z, u32(256))]), u32(1));
                    var _S39 : u32 = atomicAdd(&(shared_counts_0[u32(772) + bin_0(y_1, u32(256))]), u32(1));
                }
                if(_S29)
                {
                    var _S40 : u32 = _S33 * u32(256) / _S26;
                    var column_0 : u32 = u32(17412) + _S40 * u32(128);
                    var _S41 : u32 = atomicAdd(&(counts_0[column_0 + bin_0(rgb_1.x, u32(128))]), u32(1));
                    var _S42 : u32 = atomicAdd(&(counts_0[column_0 + u32(32768) + bin_0(rgb_1.y, u32(128))]), u32(1));
                    var _S43 : u32 = atomicAdd(&(counts_0[column_0 + u32(65536) + bin_0(rgb_1.z, u32(128))]), u32(1));
                    var _S44 : u32 = atomicAdd(&(counts_0[column_0 + u32(98304) + bin_0(y_1, u32(128))]), u32(1));
                }
                pixels_1 = pixels_2;
                shadows_1 = shadows_2;
                highlights_1 = highlights_2;
            }
            bx_0 = bx_0 + u32(1);
        }
        by_0 = by_0 + u32(1);
        pixels_0 = pixels_1;
        shadows_0 = shadows_1;
        highlights_0 = highlights_1;
    }
    if((params_0.scopes_0.vectorscope_0) != u32(0))
    {
        var _S45 : vec2<u32> = _S30 * vec2<u32>(u32(32)) + _S31;
        by_0 = u32(0);
        loop
        {
            if((by_0 < u32(2)))
            {
            }
            else
            {
                break;
            }
            bx_0 = u32(0);
            loop
            {
                if((bx_0 < u32(2)))
                {
                }
                else
                {
                    break;
                }
                var at_2 : vec2<u32> = (_S45 + vec2<u32>(bx_0, by_0) * vec2<u32>(u32(16))) * vec2<u32>(u32(2));
                var _S46 : u32 = at_2.x;
                if(((_S46 + u32(1)) < _S26))
                {
                    _S25 = ((at_2.y + u32(1)) < _S27);
                }
                else
                {
                    _S25 = false;
                }
                if(_S25)
                {
                    var i_1 : u32 = at_2.y * _S26 + _S46;
                    var _S47 : vec3<f32> = vec3<f32>(0.25f);
                    var _S48 : u32 = i_1 + _S26;
                    count_vectorscope_0(frame_0[i_1].xyz * _S47 + frame_0[_S48].xyz * _S47 + frame_0[i_1 + u32(1)].xyz * _S47 + frame_0[_S48 + u32(1)].xyz * _S47);
                }
                bx_0 = bx_0 + u32(1);
            }
            by_0 = by_0 + u32(1);
        }
    }
    if(pixels_0 != u32(0))
    {
        var _S49 : u32 = atomicAdd(&(shared_counts_0[u32(0)]), pixels_0);
    }
    if(shadows_0 != u32(0))
    {
        var _S50 : u32 = atomicAdd(&(shared_counts_0[u32(1)]), shadows_0);
    }
    if(highlights_0 != u32(0))
    {
        var _S51 : u32 = atomicAdd(&(shared_counts_0[u32(2)]), highlights_0);
    }
    workgroupBarrier();
    i_0 = index_0;
    loop
    {
        if((i_0 < u32(1028)))
        {
        }
        else
        {
            break;
        }
        var count_1 : u32 = atomicLoad(&(shared_counts_0[i_0]));
        if(count_1 != u32(0))
        {
            var _S52 : u32 = atomicAdd(&(counts_0[i_0]), count_1);
        }
        i_0 = i_0 + u32(256);
    }
    return;
}

