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

// Film grain, ported from darktable src/iop/grain.c, process(). See docs/ports/grain.md;
// `prepare` in src/setup.rs fills the parameters. darktable's grain is three octaves of
// Gustavson's 3D simplex noise at the pixel's position, normalised by the image's
// shorter side, added to Lab lightness through a photographic paper's response. The
// noise depends on the pixel's position in the whole image at the pipe's scale, the pass
// header's origin plus the invocation, so a tile or the viewport computes what the whole
// image would; away from full resolution darktable averages 21 positions across the
// pixel, a rank-1 lattice, and so does this kernel.
//
// The noise is periodic: 768 in x and in y (in darktable's skewed lattice, a step of
// 768 in x moves its cell by (1024, 256, 256), a multiple of its permutation's 256), so
// positions are reduced by it and stay small enough for fp32. darktable adds its offset,
// a hash of the file name, to the position in double; `prepare` reduces that term per
// octave on the CPU in f64.

import color;
import pass;

// darktable GRAIN_LIGHTNESS_STRENGTH_SCALE.
static const float STRENGTH_SCALE = 0.15;
// The noise's period along x and y, in its own units (darktable's octave positions).
static const float PERIOD = 768.0;
// darktable _simplex_2d_noise: three octaves "to match the power spectrum of real grain
// scans".
static const float3 FREQUENCY = float3(0.4910, 0.9441, 1.7280);
static const float3 AMPLITUDE = float3(0.2340, 0.7850, 1.2150);
// darktable's rank-1 lattice: fib2 positions across the pixel, the second coordinate
// stepping by fib1 / fib2.
static const uint TAPS = 21;
static const float TAP_STEP = 34.0 / 21.0;
static const float TAP_WEIGHT = 1.0 / 21.0;
// Gustavson's skew and unskew factors.
static const float F3 = 1.0 / 3.0;
static const float G3 = 1.0 / 6.0;
// darktable GRAIN_LUT_SIZE - 1: the paper-response table's last index.
static const float LUT_LAST = 127.0;

struct GrainParams
{
    // darktable's parameters: the strength in percent, the coarseness (`scale`, ISO /
    // 213.2) and the mid-tones bias in percent.
    float strength;
    float scale;
    float midtones_bias;
    uint pad0;
    // darktable's offset term, hash · frequency / zoom, per octave (x, y, z), reduced by
    // the period on the CPU; w is unused.
    float4 offset;
    // Working RGB to XYZ D50 and back, by rows.
    float4 rgb_to_xyz_r0;
    float4 rgb_to_xyz_r1;
    float4 rgb_to_xyz_r2;
    float4 xyz_to_rgb_r0;
    float4 xyz_to_rgb_r1;
    float4 xyz_to_rgb_r2;
    // The parameters the derivative entry points differentiate against, one per slot:
    // 0 strength, 1 scale, 2 midtones_bias; any other value stands for nothing.
    uint4 gradient;
};

struct Params
{
    PassHeader pass;
    GrainParams grain;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;
// darktable's permutation table, `perm`: its 256 entries twice, so no index needs
// wrapping. Read only.
[[vk::binding(3, 0)]] StructuredBuffer<uint> permutation;

// The parameters in darktable's units: what the derivatives are taken against.
struct Grain : IDifferentiable
{
    float strength;
    float scale;
    float midtones_bias;
};

Grain grain_of(GrainParams p)
{
    Grain g;
    g.strength = p.strength;
    g.scale = p.scale;
    g.midtones_bias = p.midtones_bias;
    return g;
}

// Working RGB to XYZ D50 and back.
struct Colour
{
    float3x3 rgb_to_xyz;
    float3x3 xyz_to_rgb;
};

Colour colour_of(GrainParams p)
{
    Colour c;
    c.rgb_to_xyz = float3x3(p.rgb_to_xyz_r0.xyz, p.rgb_to_xyz_r1.xyz, p.rgb_to_xyz_r2.xyz);
    c.xyz_to_rgb = float3x3(p.xyz_to_rgb_r0.xyz, p.xyz_to_rgb_r1.xyz, p.xyz_to_rgb_r2.xyz);
    return c;
}

// Where a pixel samples the noise: its position, normalised by the image's shorter side
// at full resolution as darktable's x and y are, and, away from full resolution, the
// spacing of the 21 positions darktable averages across it.
struct Pixel
{
    float2 at;
    float3 offset;
    float spacing;
    bool filter;
};

// darktable process(): x = (roi.x + i) / roi->scale / wd, with wd the shorter side of
// piece->buf_in, here the image at full resolution; the lattice when roi->scale is
// further than 0.01 from 1, its width piece->iscale / (roi->scale · wd) with an iscale of
// 1, as darktable's full pipe has.
Pixel pixel_of(uint3 id)
{
    PassHeader h = params.pass;
    float shorter = float(min(h.full_width, h.full_height));
    float2 position = float2(float(h.origin_x + id.x), float(h.origin_y + id.y));
    Pixel px;
    px.at = position / h.scale / shorter;
    px.offset = params.grain.offset.xyz;
    px.spacing = 1.0 / (h.scale * shorter);
    px.filter = abs(h.scale - 1.0) > 0.01;
    return px;
}

// darktable grad3[h % 12]: the twelve edges of a cube, (±1, ±1, 0), (±1, 0, ±1) and
// (0, ±1, ±1) in its order, from the index's bits.
float3 gradient_of(uint hashed)
{
    uint g = hashed % 12;
    float a = (g & 1) != 0 ? -1.0 : 1.0;
    float b = (g & 2) != 0 ? -1.0 : 1.0;
    if (g < 4)
        return float3(a, b, 0.0);
    if (g < 8)
        return float3(a, 0.0, b);
    return float3(0.0, a, b);
}

// darktable perm[i + perm[j + perm[k]]] for a cell's corner (i, j, k), each below 257.
uint hash(int3 corner)
{
    uint k = permutation[corner.z];
    uint j = permutation[uint(corner.y) + k];
    return permutation[uint(corner.x) + j];
}

// Which simplex of the skewed lattice a point lies in, and its corners' gradients: every
// decision and table read of darktable's _simplex_noise, made before the differentiable
// part, which gets them as arguments.
struct Simplex
{
    // The cell's first corner, unskewed: darktable's (X0, Y0, Z0).
    float3 origin;
    // The second and third corners' steps from the first, in cells: (i1, j1, k1) and
    // (i2, j2, k2).
    float3 second;
    float3 third;
    float3 g0;
    float3 g1;
    float3 g2;
    float3 g3;
};

Simplex locate(float3 p)
{
    float skew = (p.x + p.y + p.z) * F3;
    int3 cell = int3(floor(p + skew));
    float t = float(cell.x + cell.y + cell.z) * G3;
    Simplex s;
    s.origin = float3(cell) - t;
    float3 d = p - s.origin;
    // darktable's six orders of d's components.
    if (d.x >= d.y)
    {
        if (d.y >= d.z)
        {
            s.second = float3(1.0, 0.0, 0.0);
            s.third = float3(1.0, 1.0, 0.0);
        }
        else if (d.x >= d.z)
        {
            s.second = float3(1.0, 0.0, 0.0);
            s.third = float3(1.0, 0.0, 1.0);
        }
        else
        {
            s.second = float3(0.0, 0.0, 1.0);
            s.third = float3(1.0, 0.0, 1.0);
        }
    }
    else
    {
        if (d.y < d.z)
        {
            s.second = float3(0.0, 0.0, 1.0);
            s.third = float3(0.0, 1.0, 1.0);
        }
        else if (d.x < d.z)
        {
            s.second = float3(0.0, 1.0, 0.0);
            s.third = float3(0.0, 1.0, 1.0);
        }
        else
        {
            s.second = float3(0.0, 1.0, 0.0);
            s.third = float3(1.0, 1.0, 0.0);
        }
    }
    int3 base = cell & 255;
    s.g0 = gradient_of(hash(base));
    s.g1 = gradient_of(hash(base + int3(s.second)));
    s.g2 = gradient_of(hash(base + int3(s.third)));
    s.g3 = gradient_of(hash(base + 1));
    return s;
}

// One corner's contribution: (0.6 - |d|²)⁴ (g · d) where positive, else 0, as darktable's
// branch on t < 0 gives.
[Differentiable]
float corner(float3 d, no_diff float3 g)
{
    float t = max(0.6 - dot(d, d), 0.0);
    t = t * t;
    return t * t * dot(g, d);
}

// darktable _simplex_noise at (x, y, z), its value scaled to stay just inside [-1, 1].
// The radius 0.6 reaches past a simplex's own corners, so the noise jumps slightly where
// a point crosses into the next simplex, as darktable's does. z, the octave, is always a
// constant, but is not marked no_diff: slangc 2026.20 fails to emit the forward
// derivative of a vector built from a no_diff scalar parameter ("unexpected IR opcode
// during code emit").
[Differentiable]
float simplex(float2 xy, float z)
{
    float3 p = float3(xy, z);
    Simplex s = no_diff locate(detach(p));
    float3 d0 = p - s.origin;
    float3 d1 = d0 - s.second + G3;
    float3 d2 = d0 - s.third + 2.0 * G3;
    float3 d3 = d0 - 1.0 + 3.0 * G3;
    return 32.0 * (corner(d0, s.g0) + corner(d1, s.g1) + corner(d2, s.g2) + corner(d3, s.g3));
}

// x less a whole number of periods: the same noise.
[Differentiable]
float wrap(float x)
{
    return x - PERIOD * no_diff floor(x / PERIOD);
}

// darktable _simplex_2d_noise at one position: three octaves, octave k at z = k. Its zoom
// is (1 + 8 · scale / 100) / 800, which darktable's -ffast-math build computes as
// scale · 0.08.
[Differentiable]
float octaves(float scale, no_diff float2 at, no_diff float3 offset)
{
    float zoom = (1.0 + scale * 0.08) / 800.0;
    float total = 0.0;
    [ForceUnroll]
    for (int k = 0; k < 3; k++)
    {
        float c = FREQUENCY[k] / zoom;
        float2 xy = float2(wrap(at.x * c + offset[k]), wrap(at.y * c));
        total = total + simplex(xy, float(k)) * AMPLITUDE[k];
    }
    return total;
}

// The lattice's position `l` across the pixel, in units of its spacing.
float2 tap(uint l)
{
    float y = float(l) * TAP_STEP;
    return float2(float(l) / float(TAPS), y - floor(y));
}

// The pixel's noise: darktable's octaves at its position, or averaged over the lattice.
// Its derivatives are written out, in forward mode: the noise has one differentiable
// input, the scale, and a reverse pass through the lattice's loop would need arrays
// written at a run-time index (decision 10). Every table read and simplex decision
// happens in locate(), outside the derivative.
[ForwardDerivative(noise_forward)]
[BackwardDerivative(noise_backward)]
float noise(float scale, Pixel px)
{
    if (!px.filter)
        return octaves(scale, px.at, px.offset);
    float total = 0.0;
    for (uint l = 0; l < TAPS; l++)
        total = total + TAP_WEIGHT * octaves(scale, px.at + tap(l) * px.spacing, px.offset);
    return total;
}

DifferentialPair<float> noise_forward(DifferentialPair<float> scale, Pixel px)
{
    let unit = diffPair(scale.p, 1.0);
    if (!px.filter)
    {
        let one = fwd_diff(octaves)(unit, px.at, px.offset);
        return diffPair(one.p, one.d * scale.d);
    }
    float value = 0.0;
    float slope = 0.0;
    for (uint l = 0; l < TAPS; l++)
    {
        let one = fwd_diff(octaves)(unit, px.at + tap(l) * px.spacing, px.offset);
        value = value + TAP_WEIGHT * one.p;
        slope = slope + TAP_WEIGHT * one.d;
    }
    return diffPair(value, slope * scale.d);
}

void noise_backward(inout DifferentialPair<float> scale, Pixel px, float d_out)
{
    let slope = noise_forward(diffPair(scale.p, 1.0), px).d;
    scale = diffPair(scale.p, slope * d_out);
}

// darktable paper_resp and paper_resp_inverse with a paper gamma of 1: a logistic
// density over exposure, widened by delta at both ends.
[Differentiable]
float paper(float exposure, float delta)
{
    float span = 1.0 + 2.0 * delta;
    return span / (1.0 + exp((4.0 * (0.5 - exposure)) / span)) - delta;
}

[Differentiable]
float paper_inverse(float density, float delta)
{
    float span = 1.0 + 2.0 * delta;
    return -log(span / (density + delta) - 1.0) * span / 4.0 + 0.5;
}

// darktable GRAIN_LUT_DELTA_MAX · exp(bias / 100 · log(GRAIN_LUT_DELTA_MIN)): from 2 at no
// bias to 2e-4 at full bias.
[Differentiable]
float delta_of(float midtones_bias)
{
    return 2.0 * exp((midtones_bias / 100.0) * log(0.0001));
}

// darktable evaluate_grain_lut's entry at column i (the grain, gu = i / 127 - 0.5) and
// row j (the lightness, l = j / 127): the lightness change, in L, when the grain moves
// the paper's exposure by gu. Computed rather than read from darktable's 128 × 128 table,
// so its derivative reaches the mid-tones bias; the same expressions in f32.
[Differentiable]
float lut_entry(no_diff float i, no_diff float j, float delta)
{
    float gu = i / LUT_LAST - 0.5;
    float l = j / LUT_LAST;
    return 100.0 * (paper(gu + paper_inverse(l, delta), delta) - l);
}

// The table's cell darktable's dt_lut_lookup_2d_1c interpolates in for grain u and
// lightness l (L / 100): found before the differentiable part, which gets it as an
// argument, so both derivative passes interpolate where the forward pass did.
int2 cell_of(float u, float l)
{
    float x = clamp((u + 0.5) * LUT_LAST, 0.0, LUT_LAST);
    float y = clamp(l * LUT_LAST, 0.0, LUT_LAST);
    return int2(int(min(x, LUT_LAST - 1.0)), int(min(y, LUT_LAST - 1.0)));
}

// darktable dt_lut_lookup_2d_1c: bilinear in the table's cell, clamped to its edges.
[Differentiable]
float lookup(float u, no_diff float l, float delta, no_diff int2 cell)
{
    float x = clamp((u + 0.5) * LUT_LAST, 0.0, LUT_LAST);
    float y = clamp(l * LUT_LAST, 0.0, LUT_LAST);
    float x0 = float(cell.x);
    float y0 = float(cell.y);
    float x_diff = x - x0;
    float y_diff = y - y0;
    float l00 = lut_entry(x0, y0, delta);
    float l01 = lut_entry(x0 + 1.0, y0, delta);
    float l10 = lut_entry(x0, y0 + 1.0, delta);
    float l11 = lut_entry(x0 + 1.0, y0 + 1.0, delta);
    float xy0 = (1.0 - y_diff) * l00 + l10 * y_diff;
    float xy1 = (1.0 - y_diff) * l01 + l11 * y_diff;
    return xy0 * (1.0 - x_diff) + xy1 * x_diff;
}

// darktable's working RGB to Lab and back around a Lab module
// (_transform_rgb_to_lab_matrix and _transform_lab_to_rgb_matrix).
float3 rgb_to_lab(float3 rgb, Colour c)
{
    return xyz_to_lab_d50(mul(c.rgb_to_xyz, rgb));
}

[Differentiable]
float3 lab_to_rgb(float3 lab, no_diff Colour c)
{
    return mul(c.xyz_to_rgb, lab_to_xyz_d50(lab));
}

// The grain, u, darktable's (noise · strength / 100) · GRAIN_LIGHTNESS_STRENGTH_SCALE.
[Differentiable]
float grain_amount(float noise, float strength)
{
    return (noise * (strength / 100.0)) * STRENGTH_SCALE;
}

// The per-pixel function, darktable process() after the noise: the lightness moves by
// the paper response's change for the grain; a and b stay.
[Differentiable]
float3 grain(no_diff float3 lab, float noise, Grain g, no_diff Colour c, no_diff int2 cell)
{
    float u = grain_amount(noise, g.strength);
    float dl = lookup(u, lab.x / 100.0, delta_of(g.midtones_bias), cell);
    return lab_to_rgb(float3(lab.x + dl, lab.y, lab.z), c);
}

// The noise and the per-pixel function together, for the gradient with respect to
// darktable's parameters through both.
[Differentiable]
float3 grain_from_settings(no_diff float3 lab, Grain g, no_diff Pixel px, no_diff Colour c, no_diff int2 cell)
{
    return grain(lab, noise(g.scale, px), g, c, cell);
}

// Where the pixel's lookup falls, from the forward values.
int2 cell_for(float3 lab, float n, Grain g)
{
    return cell_of(grain_amount(n, g.strength), lab.x / 100.0);
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    Grain g = grain_of(params.grain);
    Colour c = colour_of(params.grain);
    // The noise depends on the position alone, so it is found once for every repeat.
    float n = noise(g.scale, pixel_of(id));
    float4 pixel = input[i];
    float3 rgb = pixel.rgb;
    for (uint k = 0; k < params.pass.repeat; k++)
    {
        float3 lab = rgb_to_lab(rgb, c);
        rgb = grain(lab, n, g, c, cell_for(lab, n, g));
    }
    output[i] = float4(rgb, pixel.a);
}

// Derivative entry points, both per pixel, with parameters ordered as the gradient slots
// (gradient's x, y, z and w), each one of darktable's three continuous parameters or
// none.
[[vk::binding(8, 0)]] StructuredBuffer<float4> derivative_in;
[[vk::binding(9, 0)]] RWStructuredBuffer<float4> derivative_out;

// One parameter's value in a derivative of the parameters.
float pick(Grain.Differential d, uint which)
{
    switch (which)
    {
    case 0: return d.strength;
    case 1: return d.scale;
    case 2: return d.midtones_bias;
    default: return 0.0;
    }
}

// Reverse mode: given dLoss/dOutput, the gradient of the loss with respect to the slots'
// parameters.
[shader("compute")]
[numthreads(16, 16, 1)]
void grain_backward(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    GrainParams p = params.grain;
    Grain g = grain_of(p);
    Colour c = colour_of(p);
    Pixel px = pixel_of(id);
    float3 lab = rgb_to_lab(input[i].rgb, c);
    int2 cell = cell_for(lab, noise(g.scale, px), g);
    var d = diffPair(g);
    bwd_diff(grain_from_settings)(lab, d, px, c, cell, derivative_in[i].rgb);
    derivative_out[i] = float4(
        pick(d.d, p.gradient.x),
        pick(d.d, p.gradient.y),
        pick(d.d, p.gradient.z),
        pick(d.d, p.gradient.w));
}

// Adds x to the parameter `which` in a direction.
void add_slot(inout Grain.Differential d, uint which, float x)
{
    d.strength += which == 0 ? x : 0.0;
    d.scale += which == 1 ? x : 0.0;
    d.midtones_bias += which == 2 ? x : 0.0;
}

// Forward mode: given a direction in the slots' parameters, the directional derivative
// of the output. Checks the reverse mode: for any direction v and output weights w,
// w · (J v) must equal (Jᵀ w) · v.
[shader("compute")]
[numthreads(16, 16, 1)]
void grain_forward_diff(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    GrainParams p = params.grain;
    Grain g = grain_of(p);
    Colour c = colour_of(p);
    Pixel px = pixel_of(id);
    float3 lab = rgb_to_lab(input[i].rgb, c);
    int2 cell = cell_for(lab, noise(g.scale, px), g);
    float4 direction_in = derivative_in[i];
    Grain.Differential direction;
    direction.strength = 0.0;
    direction.scale = 0.0;
    direction.midtones_bias = 0.0;
    add_slot(direction, p.gradient.x, direction_in.x);
    add_slot(direction, p.gradient.y, direction_in.y);
    add_slot(direction, p.gradient.z, direction_in.z);
    add_slot(direction, p.gradient.w, direction_in.w);
    let result = fwd_diff(grain_from_settings)(lab, diffPair(g, direction), px, c, cell);
    derivative_out[i] = float4(result.d, 0.0);
}
