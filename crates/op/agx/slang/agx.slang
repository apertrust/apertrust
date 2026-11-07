// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2025-2026  darktable developers (ported from src/iop/agx.c)
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

// AgX, ported from darktable src/iop/agx.c, process(). See docs/ports/agx.md; the
// setup, computed once per image on the CPU, is `prepare` in src/setup.rs.
//
// Working RGB goes to the base primaries' space, has negative values compressed, and
// is read as coordinates over inset and rotated primaries; each channel is then
// log-encoded around mid-grey and mapped through the tone curve, the look is applied,
// the result linearised and its hue partly restored, and the inverse of an outset
// matrix brings it back to working RGB.

import color;
import pass;

struct AgxParams
{
    // Matrices by rows: working to base RGB, base to rendering RGB, rendering to
    // working RGB, and the luminance (Y) row of rendering RGB to XYZ D50.
    float4 pipe_to_base_r0;
    float4 pipe_to_base_r1;
    float4 pipe_to_base_r2;
    float4 base_to_rendering_r0;
    float4 base_to_rendering_r1;
    float4 base_to_rendering_r2;
    float4 rendering_to_pipe_r0;
    float4 rendering_to_pipe_r1;
    float4 rendering_to_pipe_r2;
    float4 rendering_luminance;
    // The log encoding and the curve's linear section.
    float black_ev;
    float range_ev;
    float gamma;
    float slope;
    float intercept;
    float target_black;
    float target_white;
    float _pad0;
    // The toe and the shoulder: the sigmoid's transition, scale and power, and the
    // power curve used instead when the sigmoid cannot bend that way.
    float toe_transition_x;
    float toe_transition_y;
    float toe_scale;
    float toe_power;
    float toe_fallback_coefficient;
    float toe_fallback_power;
    uint toe_fallback;
    float _pad1;
    float shoulder_transition_x;
    float shoulder_transition_y;
    float shoulder_scale;
    float shoulder_power;
    float shoulder_fallback_coefficient;
    float shoulder_fallback_power;
    uint shoulder_fallback;
    float _pad2;
    // The look and the hue restoration.
    float look_slope;
    float look_lift;
    float look_power;
    float look_saturation;
    float look_hue_mix;
    float _pad3;
    float _pad4;
    float _pad5;
};

struct Params
{
    PassHeader pass;
    AgxParams agx;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;

// The parameters the derivative entry points differentiate against. The per-pixel
// function reads them from here rather than from AgxParams, which is a constant.
struct AgxLook : IDifferentiable
{
    float slope;
    float lift;
    float power;
    float saturation;
    float hue_mix;
};

AgxLook agx_look(AgxParams p)
{
    AgxLook look;
    look.slope = p.look_slope;
    look.lift = p.look_lift;
    look.power = p.look_power;
    look.saturation = p.look_saturation;
    look.hue_mix = p.look_hue_mix;
    return look;
}

static const float EPSILON = 1e-6;

// darktable _compress_into_gamut: lifts a colour with a negative channel by that
// channel, and scales it back towards its luminance. Its luminance weights are fixed,
// whatever the base primaries.
float3 compress_into_gamut(float3 rgb)
{
    const float3 weights = float3(0.2658180370250449, 0.59846986045365, 0.1357121025213052);
    float input_y = dot(rgb, weights);
    float3 opponent = max(rgb.x, max(rgb.y, rgb.z)) - rgb;
    float y_compensate_negative =
        max(opponent.x, max(opponent.y, opponent.z)) - dot(opponent, weights) + input_y;

    float offset = max(-min(rgb.x, min(rgb.y, rgb.z)), 0.0);
    float3 rgb_offset = rgb + offset;
    float3 opponent_offset = max(rgb_offset.x, max(rgb_offset.y, rgb_offset.z)) - rgb_offset;
    float y_new = max(opponent_offset.x, max(opponent_offset.y, opponent_offset.z))
        - dot(opponent_offset, weights) + dot(rgb_offset, weights);

    float ratio = 1.0;
    if (y_new > y_compensate_negative && y_new > EPSILON)
        ratio = y_compensate_negative / y_new;
    return ratio * rgb_offset;
}

// darktable _apply_log_encoding: scene-linear to the curve's input in [0, 1].
float log_encode(float x, AgxParams p)
{
    float relative = max(EPSILON, x / 0.18);
    return clamp((log2(relative) - p.black_ev) / p.range_ev, 0.0, 1.0);
}

// darktable _sigmoid, t / (1 + t^p)^(1/p), for t >= 0. Above 1 it uses the equal
// (1 + t^-p)^(-1/p), so t^p cannot overflow.
float agx_sigmoid(float t, float power)
{
    if (t <= 1.0)
        return t * pow(1.0 + pow0(t, power), -1.0 / power);
    return pow(1.0 + pow(t, -power), -1.0 / power);
}

// darktable _apply_curve: the tone curve for x in [0, 1], a pure function of the
// parameters. Its output is display-encoded: linear output is its power gamma.
float agx_curve(float x, AgxParams p)
{
    float y;
    if (x < p.toe_transition_x)
    {
        if (p.toe_fallback != 0)
            y = p.target_black + max(0.0, p.toe_fallback_coefficient * pow0(x, p.toe_fallback_power));
        else
            y = p.toe_scale * agx_sigmoid(p.slope * (x - p.toe_transition_x) / p.toe_scale, p.toe_power)
                + p.toe_transition_y;
    }
    else if (x <= p.shoulder_transition_x)
    {
        y = p.slope * x + p.intercept;
    }
    else
    {
        if (p.shoulder_fallback != 0)
            y = p.target_white
                - max(0.0, p.shoulder_fallback_coefficient * pow0(1.0 - x, p.shoulder_fallback_power));
        else
            y = p.shoulder_scale
                * agx_sigmoid(p.slope * (x - p.shoulder_transition_x) / p.shoulder_scale, p.shoulder_power)
                + p.shoulder_transition_y;
    }
    return clamp(y, p.target_black, p.target_white);
}

// Everything before the look, which depends on the pixel and constants only: the curve's
// output per channel, and the input's HSV hue in w.
float4 agx_tone_curve(float3 rgb_in, AgxParams p)
{
    float3x3 pipe_to_base = float3x3(p.pipe_to_base_r0.xyz, p.pipe_to_base_r1.xyz, p.pipe_to_base_r2.xyz);
    float3x3 base_to_rendering =
        float3x3(p.base_to_rendering_r0.xyz, p.base_to_rendering_r1.xyz, p.base_to_rendering_r2.xyz);
    // About 22.5 EV above mid-grey; NaN never reaches a module.
    float3 rgb = clamp(rgb_in, float3(-1e6), float3(1e6));
    float3 base = compress_into_gamut(mul(pipe_to_base, rgb));
    float3 rendering = mul(base_to_rendering, base);
    float hue = rgb_to_hsv(rendering).x;
    return float4(
        agx_curve(log_encode(rendering.x, p), p),
        agx_curve(log_encode(rendering.y, p), p),
        agx_curve(log_encode(rendering.z, p), p),
        hue);
}

// darktable _agx_look, without its branch at the neutral values, where it is the
// identity anyway: slope and lift, the power of positive values, then saturation
// around the rendering space's luminance.
[Differentiable]
float3 apply_look(float3 y, no_diff float3 luminance, AgxLook look)
{
    float m = look.slope / (1.0 + look.lift);
    float3 v = m * y + look.lift * m;
    v = pow0(max(v, float3(0.0)), float3(look.power)) + min(v, float3(0.0));
    float l = dot(luminance, v);
    return l + look.saturation * (v - l);
}

// darktable _lerp_hue and the HSV round trip, without its branch at a mix of 0: mix 0
// keeps the curve's hue, 1 restores the input's.
[Differentiable]
float3 restore_hue(float3 rgb, no_diff float hue_in, float mix)
{
    float3 hsv = rgb_to_hsv(rgb);
    float difference = hsv.x - hue_in;
    // remainderf(difference, 1): round rounds half to even, as remainderf does.
    difference = difference - no_diff round(difference);
    float mixed = (1.0 - mix) * difference + hue_in;
    hsv.x = mixed - no_diff floor(mixed);
    return hsv_to_rgb(hsv);
}

// The per-pixel function.
[Differentiable]
float3 agx(no_diff float3 rgb_in, no_diff AgxParams p, AgxLook look)
{
    float4 curve = no_diff agx_tone_curve(rgb_in, p);
    float3 v = apply_look(curve.xyz, p.rendering_luminance.xyz, look);
    v = pow0(max(v, float3(0.0)), float3(p.gamma));
    v = restore_hue(v, curve.w, look.hue_mix);
    float3x3 rendering_to_pipe =
        float3x3(p.rendering_to_pipe_r0.xyz, p.rendering_to_pipe_r1.xyz, p.rendering_to_pipe_r2.xyz);
    return mul(rendering_to_pipe, v);
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    AgxParams p = params.agx;
    float4 pixel = input[i];
    float3 rgb = pixel.rgb;
    for (uint k = 0; k < params.pass.repeat; k++)
        rgb = agx(rgb, p, agx_look(p));
    output[i] = float4(rgb, pixel.a);
}

// Derivative entry points, both per pixel, with parameters ordered (look slope, look
// power, look saturation, hue mix). The power is the kernel's, which darktable derives
// from its brightness; the lift is differentiated too but not returned.
[[vk::binding(8, 0)]] StructuredBuffer<float4> derivative_in;
[[vk::binding(9, 0)]] RWStructuredBuffer<float4> derivative_out;

// Reverse mode: given dLoss/dOutput, the gradient of the loss with respect to the
// parameters.
[shader("compute")]
[numthreads(16, 16, 1)]
void agx_backward(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    AgxParams p = params.agx;
    var look = diffPair(agx_look(p));
    bwd_diff(agx)(input[i].rgb, p, look, derivative_in[i].rgb);
    derivative_out[i] = float4(look.d.slope, look.d.power, look.d.saturation, look.d.hue_mix);
}

// Forward mode: given a direction in parameter space, the directional derivative of the
// output. Checks the reverse mode: for any direction v and output weights w,
// w · (J v) must equal (Jᵀ w) · v.
[shader("compute")]
[numthreads(16, 16, 1)]
void agx_forward_diff(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    AgxParams p = params.agx;
    float4 v = derivative_in[i];
    AgxLook.Differential direction;
    direction.slope = v.x;
    direction.lift = 0.0;
    direction.power = v.y;
    direction.saturation = v.z;
    direction.hue_mix = v.w;
    let result = fwd_diff(agx)(input[i].rgb, p, diffPair(agx_look(p), direction));
    derivative_out[i] = float4(result.d, 0.0);
}
