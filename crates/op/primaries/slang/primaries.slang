// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2023-2025  darktable developers (ported from src/iop/primaries.c)
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

// RGB primaries, ported from darktable src/iop/primaries.c, process(): one matrix on
// working RGB, alpha passing through. See docs/ports/primaries.md; `prepare` in
// src/setup.rs computes the matrix once per image, in f64, from the settings.
//
// The derivative entry points build the same matrix here from the settings, darktable's
// _calculate_adjustment_matrix() with dt_rotate_and_scale_primary() and
// dt_make_transposed_matrices_from_primaries_and_whitepoint() in f32, so that the
// derivatives reach the settings; main never runs that setup.

import components;
import pass;

struct PrimariesParams
{
    // The adjustment, working RGB to working RGB, by rows: what main applies.
    float4 matrix_0;
    float4 matrix_1;
    float4 matrix_2;
    // The settings, for the derivative entry points: for the tint, red, green and blue
    // (x, y, z and w), the hue in radians and the purity.
    float4 hue;
    float4 purity;
    // The working profile's chromaticities as darktable reads them back from it, D50:
    // the white and red, then green and blue.
    float4 white_and_red;
    float4 green_and_blue;
    // XYZ D50 to working RGB, by rows.
    float4 to_rgb_0;
    float4 to_rgb_1;
    float4 to_rgb_2;
    // The four settings the derivative entry points differentiate against, each by its
    // index in darktable's params struct: 0 achromatic_tint_hue, 1 achromatic_tint_purity,
    // 2 red_hue, 3 red_purity, 4 green_hue, 5 green_purity, 6 blue_hue, 7 blue_purity.
    uint4 gradient;
    // 1 when the setup found the new primaries' matrix singular, as darktable's
    // mat3SSEinv decides, in f64, and the adjustment is 0; else 0. An f32 determinant
    // cannot tell: on a singular matrix its rounding is larger than the threshold.
    uint singular;
    uint pad0;
    uint pad1;
    uint pad2;
};

struct Params
{
    PassHeader pass;
    PrimariesParams primaries;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;

static const float FLT_MAX = 3.402823466e+38;
static const float FLT_EPSILON = 1.192092896e-07;

float3x3 rows(float4 r0, float4 r1, float4 r2)
{
    return float3x3(r0.xyz, r1.xyz, r2.xyz);
}

// The per-pixel function.
[Differentiable]
float3 primaries(no_diff float3 rgb, float3x3 adjustment)
{
    return mul(adjustment, rgb);
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    PrimariesParams p = params.primaries;
    float3x3 adjustment = rows(p.matrix_0, p.matrix_1, p.matrix_2);
    float4 pixel = input[i];
    float3 rgb = pixel.rgb;
    for (uint k = 0; k < params.pass.repeat; k++)
        rgb = primaries(rgb, adjustment);
    output[i] = float4(rgb, pixel.a);
}

// What the derivatives are taken against: darktable's settings, in its units. For the
// tint, red, green and blue (x, y, z and w).
struct Settings : IDifferentiable
{
    float4 hue;
    float4 purity;
};

// The working profile, which no setting moves, and the setup's finding that the
// settings' matrix is singular.
struct Profile
{
    float2 white;
    float2 red;
    float2 green;
    float2 blue;
    float3x3 to_rgb;
    bool singular;
};

Settings settings_of(PrimariesParams p)
{
    Settings s;
    s.hue = p.hue;
    s.purity = p.purity;
    return s;
}

Profile profile_of(PrimariesParams p)
{
    Profile profile;
    profile.white = p.white_and_red.xy;
    profile.red = p.white_and_red.zw;
    profile.green = p.green_and_blue.xy;
    profile.blue = p.green_and_blue.zw;
    profile.to_rgb = rows(p.to_rgb_0, p.to_rgb_1, p.to_rgb_2);
    profile.singular = p.singular != 0;
    return profile;
}

// darktable _determinant.
[Differentiable]
float determinant(float a, float b, float c, float d)
{
    return a * d - b * c;
}

// darktable _intersect_line_segments, for the ray from the white along `direction`
// and the line through a and b: how far along the ray it meets the line, or FLT_MAX if
// it never does or meets it behind the white.
[Differentiable]
float edge_distance(no_diff float2 white, float2 direction, no_diff float2 a, no_diff float2 b)
{
    float denominator = determinant(-direction.x, a.x - b.x, -direction.y, a.y - b.y);
    if (denominator == 0.0)
        return FLT_MAX;
    float t = determinant(white.x - a.x, a.x - b.x, white.y - a.y, a.y - b.y) / denominator;
    return t >= 0.0 ? t : FLT_MAX;
}

// darktable dt_rotate_and_scale_primary: `primary` turned about the white by
// `rotation`, then placed `scaling` times the distance from the white to the nearest
// edge of the profile's triangle in that direction (_find_distance_to_edge).
[Differentiable]
float2 rotate_and_scale(no_diff Profile profile, no_diff float2 primary, float scaling, float rotation)
{
    float2 white = profile.white;
    float angle = atan2(primary.y - white.y, primary.x - white.x) + rotation;
    float2 direction = float2(cos(angle), sin(angle));
    float distance = min(
        min(edge_distance(white, direction, profile.red, profile.green),
            edge_distance(white, direction, profile.green, profile.blue)),
        edge_distance(white, direction, profile.blue, profile.red));
    return white + scaling * distance * direction;
}

// darktable _sanitizeY: y held at least FLT_EPSILON from 0, keeping its sign.
[Differentiable]
float sanitize_y(float y)
{
    if (y < FLT_EPSILON && y >= 0.0)
        return FLT_EPSILON;
    if (y < 0.0 && y > -FLT_EPSILON)
        return -FLT_EPSILON;
    return y;
}

// The XYZ of a chromaticity at Y = 1, with darktable's sanitised y.
[Differentiable]
float3 chromaticity_to_xyz(float2 xy)
{
    float y = sanitize_y(xy.y);
    return float3(xy.x / y, 1.0, (1.0 - xy.x - y) / y);
}

// Setup, once per image (darktable _calculate_adjustment_matrix): the primaries and the
// white moved as the settings say, the tint from the red primary's direction; RGB as
// amounts of those primaries, adding up to that white (darktable
// dt_make_transposed_matrices_from_primaries_and_whitepoint, by Cramer's rule), to XYZ
// D50; and back to working RGB. Zero where the setup found the matrix singular, as
// darktable's mat3SSEinv refuses it.
[Differentiable]
float3x3 adjustment(Settings s, no_diff Profile profile)
{
    float3 r = chromaticity_to_xyz(rotate_and_scale(profile, profile.red, s.purity.y, s.hue.y));
    float3 g = chromaticity_to_xyz(rotate_and_scale(profile, profile.green, s.purity.z, s.hue.z));
    float3 b = chromaticity_to_xyz(rotate_and_scale(profile, profile.blue, s.purity.w, s.hue.w));
    float3 w = chromaticity_to_xyz(rotate_and_scale(profile, profile.red, s.purity.x, s.hue.x));

    float3 gb = cross(g, b);
    float3 br = cross(b, r);
    float3 rg = cross(r, g);
    float det = dot(r, gb);
    float3 scale = float3(0.0);
    if (!profile.singular && det != 0.0)
        scale = float3(dot(gb, w), dot(br, w), dot(rg, w)) / det;

    // Each new primary in working RGB: a column of the adjustment.
    float3 red = mul(profile.to_rgb, r * scale.x);
    float3 green = mul(profile.to_rgb, g * scale.y);
    float3 blue = mul(profile.to_rgb, b * scale.z);
    return float3x3(
        float3(red.x, green.x, blue.x),
        float3(red.y, green.y, blue.y),
        float3(red.z, green.z, blue.z));
}

// Setup and per-pixel function together, for the gradient with respect to the settings
// through both.
[Differentiable]
float3 primaries_from_settings(no_diff float3 rgb, Settings s, no_diff Profile profile)
{
    return primaries(rgb, adjustment(s, profile));
}

// Derivative entry points, both per pixel, with parameters ordered as the gradient
// slots (gradient's x, y, z and w), each a setting by its index in darktable's params
// struct. Two slots naming one setting each get its whole gradient.
[[vk::binding(8, 0)]] StructuredBuffer<float4> derivative_in;
[[vk::binding(9, 0)]] RWStructuredBuffer<float4> derivative_out;

// One slot's value in a derivative of the settings: an even index is a hue, an odd one
// a purity, of the tint, red, green or blue by half the index.
float pick(Settings.Differential d, uint index)
{
    float4 v = index % 2 == 0 ? d.hue : d.purity;
    return v[index / 2];
}

// Reverse mode: given dLoss/dOutput, the gradient of the loss with respect to the
// slots' settings.
[shader("compute")]
[numthreads(16, 16, 1)]
void primaries_backward(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    PrimariesParams p = params.primaries;
    var s = diffPair(settings_of(p));
    bwd_diff(primaries_from_settings)(input[i].rgb, s, profile_of(p), derivative_in[i].rgb);
    derivative_out[i] = float4(
        pick(s.d, p.gradient.x),
        pick(s.d, p.gradient.y),
        pick(s.d, p.gradient.z),
        pick(s.d, p.gradient.w));
}

// Adds x to the slot's setting in a direction: to its component of the hues or the
// purities, and to component 4, which changes nothing, of the other.
void add_slot(inout Settings.Differential d, uint index, float x)
{
    d.hue = add_at(d.hue, index % 2 == 0 ? index / 2 : 4, x);
    d.purity = add_at(d.purity, index % 2 == 1 ? index / 2 : 4, x);
}

// Forward mode: given a direction in the slots' settings, the directional derivative of
// the output. Checks the reverse mode: for any direction v and output weights w,
// w · (J v) must equal (Jᵀ w) · v.
[shader("compute")]
[numthreads(16, 16, 1)]
void primaries_forward_diff(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    PrimariesParams p = params.primaries;
    float4 v = derivative_in[i];
    Settings.Differential direction;
    direction.hue = float4(0.0);
    direction.purity = float4(0.0);
    add_slot(direction, p.gradient.x, v.x);
    add_slot(direction, p.gradient.y, v.y);
    add_slot(direction, p.gradient.z, v.z);
    add_slot(direction, p.gradient.w, v.w);
    let result = fwd_diff(primaries_from_settings)(input[i].rgb, diffPair(settings_of(p), direction), profile_of(p));
    derivative_out[i] = float4(result.d, 0.0);
}
