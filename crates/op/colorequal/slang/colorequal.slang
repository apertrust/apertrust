// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2022-2026  darktable developers (ported from src/iop/colorequal.c)
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

// The color equalizer, ported from darktable src/iop/colorequal.c, process(): each
// pixel's hue, saturation and brightness in darktable UCS changed by three curves over
// its hue. Without the guided filter, `main` does everything per pixel. With it, the
// whole-image passes first smooth the chromaticity the hue is read from (darktable's
// _prefilter_chromaticity), then compute the corrections from it and spread the
// saturation and brightness corrections over areas of one chromaticity
// (_guide_with_chromaticity), both 2×2 guided filters over darktable's recursive
// Gaussian; `main` then applies the corrections it reads. See docs/ports/colorequal.md;
// `prepare` in src/setup.rs fills the parameters and lists the passes.
//
// The scratch buffers are laid out as the whole image at the pipe's scale, or as a
// filter's grid, a float2 or a float a pixel. A pass binds each where its role says
// (bindings 10 to 26); the setup binds the buffer each pass gives that role.

import bilinear;
import color;
import gaussian;
import pass;

static const uint LUT_SIZE = 512;
// darktable SATSIZE: the saturation weights' table holds 2 · SATSIZE + 1 entries.
static const uint SATSIZE = 4096;
static const float SAT_EFFECT = 2.0;
static const float BRIGHT_EFFECT = 8.0;
static const float FLT_EPSILON = 1.1920928955078125e-07;
// A guard, not darktable's: each input channel bounded at ±10³⁰, so no infinite pixel
// reaches the colour maths; it changes no finite value a photo has.
static const float MAX_INPUT = 1e30;

// The tables' layout (setup.rs, `table`): main's curves, the gamut and the curves'
// derivatives with respect to their red nodes; the corrections pass's curves, then the
// saturation weights; the prefilter's weights alone.
static const uint TABLE_HUE = 0;
static const uint TABLE_SATURATION = LUT_SIZE;
static const uint TABLE_BRIGHTNESS = 2 * LUT_SIZE;
static const uint TABLE_GAMUT = 3 * LUT_SIZE;
static const uint TABLE_RED_HUE = 4 * LUT_SIZE;
static const uint TABLE_RED_SATURATION = 5 * LUT_SIZE;
static const uint TABLE_RED_BRIGHTNESS = 6 * LUT_SIZE;
static const uint TABLE_CORRECTION_WEIGHTS = 3 * LUT_SIZE;

struct ColorEqualParams
{
    // x, y: the whole image at the pipe's scale, the scratch's layout; z, w: the grid a
    // grid pass or a line pass covers.
    uint4 size;
    // x: a line pass's axis; y: the grid is resized (smaller than the image); z: main
    // applies the guided corrections it reads; w: the filter blurs in place, as
    // darktable's does where its grid is the image (setup.rs, Grid::in_place).
    uint4 mode;
    // The Gaussian's coefficients: a0, a1, a2, a3, then b1, b2, coefp, coefn.
    float4 deriche_a;
    float4 deriche_b;
    // x: sat_shift; y: bright_shift; z: the filter's feathering; w: gradient_amp.
    float4 shifts;
    // x: the white's L*.
    float4 white;
    // The gradient slots' values: the red node of the hue curve in degrees, of the
    // saturation and of the brightness curve, and the white level in EV.
    float4 slots;
    // Working RGB to XYZ D65 and back, by rows.
    float4 rgb_to_xyz_r0;
    float4 rgb_to_xyz_r1;
    float4 rgb_to_xyz_r2;
    float4 xyz_to_rgb_r0;
    float4 xyz_to_rgb_r1;
    float4 xyz_to_rgb_r2;
};

struct Params
{
    PassHeader pass;
    ColorEqualParams ce;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;
[[vk::binding(3, 0)]] StructuredBuffer<float> table;

// The image's chromaticity U*′V*′, prefiltered in place.
[[vk::binding(10, 0)]] RWStructuredBuffer<float2> uv;
// A filter's grid: its chromaticity, then the covariance in two halves (P1, P2), then
// the coefficients a (P1, P2) and b (C1); the guide's correlations in C1 and C2.
[[vk::binding(11, 0)]] RWStructuredBuffer<float2> grid_uv;
[[vk::binding(12, 0)]] RWStructuredBuffer<float2> p1;
[[vk::binding(13, 0)]] RWStructuredBuffer<float2> p2;
[[vk::binding(14, 0)]] RWStructuredBuffer<float2> c1;
[[vk::binding(15, 0)]] RWStructuredBuffer<float2> c2;
// The hue and saturation corrections, and the brightness correction.
[[vk::binding(16, 0)]] RWStructuredBuffer<float2> corrections;
[[vk::binding(20, 0)]] RWStructuredBuffer<float> brightness;
// The saturation weights at sat_shift and bright_shift.
[[vk::binding(17, 0)]] RWStructuredBuffer<float2> weights;
// The saturation, blurred in place, and L*, then the saturation's gradient.
[[vk::binding(18, 0)]] RWStructuredBuffer<float> saturation;
[[vk::binding(19, 0)]] RWStructuredBuffer<float> lightness;
// A line pass's buffer and the one it writes.
[[vk::binding(21, 0)]] RWStructuredBuffer<float2> line2_in;
[[vk::binding(22, 0)]] RWStructuredBuffer<float2> line2_out;
[[vk::binding(23, 0)]] RWStructuredBuffer<float> line1_in;
[[vk::binding(24, 0)]] RWStructuredBuffer<float> line1_out;
// The guide's corrections on its grid: the image's own, or downsampled.
[[vk::binding(25, 0)]] RWStructuredBuffer<float2> grid_corrections;
[[vk::binding(26, 0)]] RWStructuredBuffer<float> grid_brightness;

float3x3 rgb_to_xyz_of(ColorEqualParams p)
{
    return float3x3(p.rgb_to_xyz_r0.xyz, p.rgb_to_xyz_r1.xyz, p.rgb_to_xyz_r2.xyz);
}

float3x3 xyz_to_rgb_of(ColorEqualParams p)
{
    return float3x3(p.xyz_to_rgb_r0.xyz, p.xyz_to_rgb_r1.xyz, p.xyz_to_rgb_r2.xyz);
}

// The pixel's index in the scratch, laid out as the whole image: the pass's origin plus
// its coordinates, so a tile of an export reads and writes its own part (decision 15).
uint place(uint3 id, ColorEqualParams p)
{
    return (params.pass.origin_y + id.y) * p.size.x + params.pass.origin_x + id.x;
}

// darktable process() STEP 1 for one pixel: its chromaticity, its lightness L* and its
// saturation, (max - min) / max of its channels.
struct Sample
{
    float2 uv;
    float l_star;
    float saturation;
};

Sample sample_of(float3 rgb_in, ColorEqualParams p)
{
    float3 rgb = clamp(rgb_in, float3(-MAX_INPUT), float3(MAX_INPUT));
    float dmin = min(min(rgb.r, rgb.g), rgb.b);
    float dmax = max(max(rgb.r, rgb.g), rgb.b);
    float delta = dmax - dmin;
    Sample s;
    s.saturation = (dmax > NORM_MIN && delta > NORM_MIN) ? delta / dmax : 0.0;
    float3 xyy = d65_xyz_to_xyy(mul(rgb_to_xyz_of(p), rgb));
    s.uv = xy_to_ucs_uv(xyy.xy);
    s.l_star = y_to_ucs_l_star(xyy.z);
    return s;
}

// darktable lookup_gamut() on the table's entries from `offset`: linear interpolation
// between the two around the hue, wrapping around, with no branch for a hue on an entry,
// whose weight is 0 anyway, since Slang's backward pass recomputes such a branch
// (colour balance rgb's lookup_gamut).
[Differentiable]
float lookup(uint offset, float hue)
{
    float x = float(LUT_SIZE) * (hue + PI) / (2.0 * PI);
    float x_prev = no_diff floor(x);
    uint i = asuint(int(x_prev)) & (LUT_SIZE - 1);
    uint j = (i + 1) & (LUT_SIZE - 1);
    float y = table[offset + i];
    return y + (x - x_prev) * (table[offset + j] - y);
}

// darktable _get_satweight(): the weights' table from `offset` interpolated at a
// saturation.
float satweight(uint offset, float sat)
{
    float isat = float(SATSIZE) * (1.0 + clamp(sat, -1.0, 1.0 - (1.0 / float(SATSIZE))));
    float base = floor(isat);
    uint i = uint(base);
    float w = table[offset + i];
    return w + (isat - base) * (table[offset + i + 1] - w);
}

// darktable dt_UCS_LUV_to_JCH: lightness, chroma and hue from L* and U*′V*′.
[Differentiable]
float3 luv_to_jch(float l_star, float l_white, float2 uv_star)
{
    float m2 = uv_star.x * uv_star.x + uv_star.y * uv_star.y;
    return float3(l_star / l_white,
        15.932993652962535 * pow0(l_star, 0.6523997524738018) * pow0(m2, 0.6007557017508491) / l_white,
        atan2(uv_star.y, uv_star.x));
}

// What the derivative entry points differentiate against, the gradient slots: the red
// node of the hue curve (in degrees), of the saturation curve and of the brightness
// curve, and the white level in EV.
struct Slots : IDifferentiable
{
    float hue_red;
    float saturation_red;
    float brightness_red;
    float white_level;
};

Slots slots_of(ColorEqualParams p)
{
    Slots s;
    s.hue_red = p.slots.x;
    s.saturation_red = p.slots.y;
    s.brightness_red = p.slots.z;
    s.white_level = p.slots.w;
    return s;
}

// A curve's value at the hue from its table at `offset`; with `slots`, as a function of
// its red node's slot `slot`, whose value is `value`, through the table at `derivative`,
// the curve's derivative with respect to that node: the curve is linear in its nodes.
// `main` passes no slots, and the substitution costs nothing there.
[Differentiable]
float curve(uint offset, uint derivative, float hue, float slot, float value, bool slots)
{
    float v = lookup(offset, hue);
    if (slots)
        v = v + lookup(derivative, hue) * (slot - value);
    return v;
}

// The per-pixel function: darktable process() STEP 3's HSB of a pixel of lightness
// `l_star` and chromaticity `uv_star`, its corrections, the guided ones `guided` (hue,
// saturation and brightness) when the mode says, otherwise from the curves at its hue,
// and STEP 5: the corrections applied, the saturation gamut-mapped and the result back
// to working RGB. With `slots`, a function of the gradient slots, `s`.
[Differentiable]
float3 colorequal(no_diff float l_star, no_diff float2 uv_star, no_diff float3 guided, no_diff ColorEqualParams p,
                  no_diff bool slots, Slots s)
{
    float white = p.white.x;
    if (slots)
        white = y_to_ucs_l_star(exp2(s.white_level));
    float3 jch = luv_to_jch(l_star, white, uv_star);
    float3 hsb = ucs_jch_to_hsb(jch);

    // The boosts from the curves at the hue, which reads only the pixel's chromaticity,
    // no slot. They are read whatever the pixel, then chosen: Slang's backward pass
    // recomputes a branch's condition, and where the chroma it recomputes lands on the
    // other side of NORM_MIN, a branch around the reads would read table entries the
    // forward pass never saved.
    float h = hsb.x;
    float hue_curve = curve(TABLE_HUE, TABLE_RED_HUE, h, s.hue_red, p.slots.x, slots);
    float saturation_curve = curve(TABLE_SATURATION, TABLE_RED_SATURATION, h, s.saturation_red, p.slots.y, slots);
    float brightness_curve = curve(TABLE_BRIGHTNESS, TABLE_RED_BRIGHTNESS, h, s.brightness_red, p.slots.z, slots);

    // The guided boosts, or the curves', or none for a neutral grey.
    bool guided_mode = p.mode.z != 0;
    bool chromatic = jch.y > NORM_MIN;
    float hue_correction = guided_mode ? guided.x : (chromatic ? hue_curve : 0.0);
    float saturation_correction = guided_mode ? guided.y : (chromatic ? saturation_curve : 1.0);
    float brightness_correction = guided_mode ? guided.z : (chromatic ? hsb.y * (brightness_curve - 1.0) : 0.0);

    // The hue is an offset, the saturation and brightness gains.
    float hue = hsb.x + hue_correction;
    float sat = max(0.0, hsb.y * (1.0 + SAT_EFFECT * (saturation_correction - 1.0)));
    float bright = max(0.0, hsb.z * (1.0 + BRIGHT_EFFECT * brightness_correction));

    // gamut_map_HSB(): the saturation soft-clipped at constant brightness to the working
    // gamut's boundary at this hue and lightness, the table holding its M².
    float3 mapped = ucs_hsb_to_jch(float3(hue, sat, bright));
    float max_colorfulness = lookup(TABLE_GAMUT, mapped.z);
    float max_chroma = 15.932993652962535 * pow0(mapped.x * white, 0.6523997524738018)
        * pow0(max_colorfulness, 0.6007557017508491) / white;
    float3 boundary = ucs_jch_to_hsb(float3(mapped.x, max_chroma, mapped.z));
    sat = soft_clip(sat, 0.8 * boundary.y, boundary.y);

    // dt_UCS_HSB_to_XYZ, then XYZ D65 to working RGB through D50. The colour library's
    // dt_UCS_JCH_to_xyY takes its white as a constant, so J and C come multiplied by the
    // white and the white is 1: darktable's own products, J · L_white and C · L_white,
    // and the white level's derivative reaches them.
    float3 jch_out = ucs_hsb_to_jch(float3(hue, sat, bright));
    float3 xyz = xyy_to_xyz(ucs_jch_to_xyy(float3(jch_out.x * white, jch_out.y * white, jch_out.z), 1.0));
    return mul(xyz_to_rgb_of(p), xyz);
}

// The per-pixel function's inputs at a pixel: its lightness, and its chromaticity and
// guided corrections, from the scratch when guided.
struct Pixel
{
    float l_star;
    float2 uv_star;
    float3 guided;
};

Pixel pixel_of(float3 rgb, uint k, ColorEqualParams p)
{
    Sample s = sample_of(rgb, p);
    Pixel px;
    px.l_star = s.l_star;
    px.uv_star = s.uv;
    px.guided = float3(0.0, 1.0, 0.0);
    if (p.mode.z != 0)
    {
        px.uv_star = uv[k];
        float2 c = corrections[k];
        px.guided = float3(c.x, c.y, brightness[k]);
    }
    return px;
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    ColorEqualParams p = params.ce;
    uint i = id.y * params.pass.width + id.x;
    float4 pixel = input[i];
    Slots s = slots_of(p);
    uint k = place(id, p);
    float3 rgb = pixel.rgb;
    for (uint r = 0; r < params.pass.repeat; r++)
    {
        Pixel px = pixel_of(rgb, k, p);
        rgb = colorequal(px.l_star, px.uv_star, px.guided, p, false, s);
    }
    output[i] = float4(rgb, pixel.a);
}

// Derivative entry points of `main`, per pixel, over the scratch the forward render left
// (decision 15), in the slots' order: the red node of the hue, saturation and brightness
// curves, then the white level. Exact without the guided filter; with it, `main`'s
// own share with the guided corrections held, for which the cases name no slot
// (docs/ports/colorequal.md, Gradients). They bind what `main` binds but its output.
[[vk::binding(8, 0)]] StructuredBuffer<float4> derivative_in;
[[vk::binding(9, 0)]] RWStructuredBuffer<float4> derivative_out;

// Reverse mode: given dLoss/dOutput, the gradient of the loss with respect to the slots.
[shader("compute")]
[numthreads(16, 16, 1)]
void colorequal_backward(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    ColorEqualParams p = params.ce;
    uint i = id.y * params.pass.width + id.x;
    Pixel px = pixel_of(input[i].rgb, place(id, p), p);
    var s = diffPair(slots_of(p));
    bwd_diff(colorequal)(px.l_star, px.uv_star, px.guided, p, true, s, derivative_in[i].rgb);
    derivative_out[i] = float4(s.d.hue_red, s.d.saturation_red, s.d.brightness_red, s.d.white_level);
}

// Forward mode: given a direction in the slots, the directional derivative of the
// output. Checks the reverse mode.
[shader("compute")]
[numthreads(16, 16, 1)]
void colorequal_forward_diff(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    ColorEqualParams p = params.ce;
    uint i = id.y * params.pass.width + id.x;
    Pixel px = pixel_of(input[i].rgb, place(id, p), p);
    float4 v = derivative_in[i];
    Slots.Differential direction;
    direction.hue_red = v.x;
    direction.saturation_red = v.y;
    direction.brightness_red = v.z;
    direction.white_level = v.w;
    let result = fwd_diff(colorequal)(px.l_star, px.uv_star, px.guided, p, true, diffPair(slots_of(p), direction));
    derivative_out[i] = float4(result.d, 0.0);
}

// STEP 1 with the guided filter: each pixel's chromaticity, saturation and L*, at its
// place in the whole image. In a tile, the tile's part of it.
[shader("compute")]
[numthreads(16, 16, 1)]
void sample(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    ColorEqualParams p = params.ce;
    Sample s = sample_of(input[id.y * params.pass.width + id.x].rgb, p);
    uint k = place(id, p);
    uv[k] = s.uv;
    saturation[k] = s.saturation;
    lightness[k] = s.l_star;
}

// The Gaussian's coefficients from the uniforms.
Deriche deriche_of(ColorEqualParams p)
{
    Deriche d;
    d.a0 = p.deriche_a.x;
    d.a1 = p.deriche_a.y;
    d.a2 = p.deriche_a.z;
    d.a3 = p.deriche_a.w;
    d.b1 = p.deriche_b.x;
    d.b2 = p.deriche_b.y;
    d.coefp = p.deriche_b.z;
    d.coefn = p.deriche_b.w;
    return d;
}

// dt_gaussian_blur()'s walk along one line of `count` values from `start`, `stride`
// apart, of line2_in into line2_out: the causal sums written, then the anticausal ones
// added, each value clamped as dt_gaussian_mean_blur() bounds it.
void walk2(uint start, uint stride, uint count, Deriche d)
{
    float2 low = float2(-MEAN_BLUR_RANGE);
    float2 high = float2(MEAN_BLUR_RANGE);
    float2 xp = clampf(line2_in[start], low, high);
    float2 yb = xp * d.coefp;
    float2 yp = yb;
    for (uint n = 0; n < count; n++)
    {
        uint at = start + n * stride;
        float2 xc = clampf(line2_in[at], low, high);
        float2 yc = causal(d, xc, xp, yp, yb);
        line2_out[at] = yc;
        xp = xc;
        yb = yp;
        yp = yc;
    }
    float2 xn = clampf(line2_in[start + (count - 1) * stride], low, high);
    float2 xa = xn;
    float2 yn = xn * d.coefn;
    float2 ya = yn;
    for (uint n = count; n > 0; n--)
    {
        uint at = start + (n - 1) * stride;
        float2 xc = clampf(line2_in[at], low, high);
        float2 yc = anticausal(d, xn, xa, yn, ya);
        xa = xn;
        xn = xc;
        ya = yn;
        yn = yc;
        line2_out[at] = line2_out[at] + yc;
    }
}

// The same for one channel, line1_in into line1_out.
void walk1(uint start, uint stride, uint count, Deriche d)
{
    float xp = clampf(line1_in[start], -MEAN_BLUR_RANGE, MEAN_BLUR_RANGE);
    float yb = xp * d.coefp;
    float yp = yb;
    for (uint n = 0; n < count; n++)
    {
        uint at = start + n * stride;
        float xc = clampf(line1_in[at], -MEAN_BLUR_RANGE, MEAN_BLUR_RANGE);
        float yc = causal(d, xc, xp, yp, yb);
        line1_out[at] = yc;
        xp = xc;
        yb = yp;
        yp = yc;
    }
    float xn = clampf(line1_in[start + (count - 1) * stride], -MEAN_BLUR_RANGE, MEAN_BLUR_RANGE);
    float xa = xn;
    float yn = xn * d.coefn;
    float ya = yn;
    for (uint n = count; n > 0; n--)
    {
        uint at = start + (n - 1) * stride;
        float xc = clampf(line1_in[at], -MEAN_BLUR_RANGE, MEAN_BLUR_RANGE);
        float yc = anticausal(d, xn, xa, yn, ya);
        xa = xn;
        xn = xc;
        ya = yn;
        yn = yc;
        line1_out[at] = line1_out[at] + yc;
    }
}

// dt_gaussian_blur()'s vertical pass over a grid of size.zw, one invocation per column,
// in rows of 16: the pass's size is 16 × the columns over 16.
[shader("compute")]
[numthreads(16, 16, 1)]
void blur_columns2(uint3 id: SV_DispatchThreadID)
{
    ColorEqualParams p = params.ce;
    uint column = id.y * 16 + id.x;
    if (column >= p.size.z)
        return;
    walk2(column, p.size.z, p.size.w, deriche_of(p));
}

// Its horizontal pass, one invocation per row.
[shader("compute")]
[numthreads(16, 16, 1)]
void blur_rows2(uint3 id: SV_DispatchThreadID)
{
    ColorEqualParams p = params.ce;
    uint row = id.y * 16 + id.x;
    if (row >= p.size.w)
        return;
    walk2(row * p.size.z, 1, p.size.z, deriche_of(p));
}

[shader("compute")]
[numthreads(16, 16, 1)]
void blur_columns1(uint3 id: SV_DispatchThreadID)
{
    ColorEqualParams p = params.ce;
    uint column = id.y * 16 + id.x;
    if (column >= p.size.z)
        return;
    walk1(column, p.size.z, p.size.w, deriche_of(p));
}

[shader("compute")]
[numthreads(16, 16, 1)]
void blur_rows1(uint3 id: SV_DispatchThreadID)
{
    ColorEqualParams p = params.ce;
    uint row = id.y * 16 + id.x;
    if (row >= p.size.w)
        return;
    walk1(row * p.size.z, 1, p.size.z, deriche_of(p));
}

// The image's chromaticity at a cell of the grid: itself where the grid is the image,
// otherwise sampled by interpolate_bilinear().
float2 uv_on_grid(uint2 at, ColorEqualParams p)
{
    uint w = p.size.x;
    if (p.mode.y == 0)
        return uv[at.y * w + at.x];
    Taps t = bilinear(at, p.size.xy, p.size.zw);
    return interpolate(t, uv[t.y_prev * w + t.x_prev], uv[t.y_prev * w + t.x_next], uv[t.y_next * w + t.x_prev],
        uv[t.y_next * w + t.x_next]);
}

// A filter's coefficients a and b at a pixel of the image: the grid's own where it is
// the image, otherwise upsampled by interpolate_bilinear(), each channel as darktable's
// four-channel a and two-channel b.
struct Coefficients
{
    float4 a;
    float2 b;
};

Coefficients coefficients_at(uint2 at, uint k, ColorEqualParams p)
{
    Coefficients c;
    if (p.mode.y == 0)
    {
        c.a = float4(p1[k], p2[k]);
        c.b = c1[k];
        return c;
    }
    uint w = p.size.z;
    Taps t = bilinear(at, p.size.zw, p.size.xy);
    uint nw = t.y_prev * w + t.x_prev;
    uint ne = t.y_prev * w + t.x_next;
    uint sw = t.y_next * w + t.x_prev;
    uint se = t.y_next * w + t.x_next;
    c.a = float4(interpolate(t, p1[nw], p1[ne], p1[sw], p1[se]), interpolate(t, p2[nw], p2[ne], p2[sw], p2[se]));
    c.b = interpolate(t, c1[nw], c1[ne], c1[sw], c1[se]);
    return c;
}

// The 2×2 inverse of the regularised covariance `sigma`, or 0 where its determinant is
// within 4 FLT_EPSILON of 0, as darktable leaves a singular matrix's coefficients.
float4 inverse_of(float4 sigma, out bool invertible)
{
    float det = sigma.x * sigma.w - sigma.y * sigma.z;
    invertible = abs(det) > 4.0 * FLT_EPSILON;
    if (!invertible)
        return float4(0.0);
    return float4(sigma.w / det, -sigma.y / det, -sigma.z / det, sigma.x / det);
}

// _prefilter_chromaticity()'s start, on its grid: the chromaticity (downsampled, or the
// image's own) and its products, the covariance before its average (_init_covariance).
[shader("compute")]
[numthreads(16, 16, 1)]
void prefilter_init(uint3 id: SV_DispatchThreadID)
{
    ColorEqualParams p = params.ce;
    if (id.x >= p.size.z || id.y >= p.size.w)
        return;
    uint d = id.y * p.size.z + id.x;
    float2 v = uv_on_grid(id.xy, p);
    grid_uv[d] = v;
    p1[d] = float2(v.x * v.x, v.x * v.y);
    p2[d] = float2(v.x * v.y, v.y * v.y);
}

// _finish_covariance() and _prepare_prefilter() on the blurred grid: the covariance,
// then a and b with U′ = a0 U + a1 V + b0 and V′ = a2 U + a3 V + b1.
[shader("compute")]
[numthreads(16, 16, 1)]
void prefilter_coefficients(uint3 id: SV_DispatchThreadID)
{
    ColorEqualParams p = params.ce;
    if (id.x >= p.size.z || id.y >= p.size.w)
        return;
    uint d = id.y * p.size.z + id.x;
    float2 m = grid_uv[d];
    float2 q1 = p1[d];
    float2 q2 = p2[d];
    float4 cov = float4(q1.x - m.x * m.x, q1.y - m.x * m.y, q2.x - m.x * m.y, q2.y - m.y * m.y);
    float eps = p.shifts.z;
    bool invertible;
    float4 inv = inverse_of(float4(cov.x + eps, cov.y, cov.z, cov.w + eps), invertible);
    float4 a = float4(0.0);
    if (invertible)
        a = float4(cov.x * inv.x + cov.y * inv.y, cov.x * inv.z + cov.y * inv.w, cov.z * inv.x + cov.w * inv.y,
            cov.z * inv.z + cov.w * inv.w);
    p1[d] = a.xy;
    p2[d] = a.zw;
    c1[d] = float2(m.x - a.x * m.x - a.y * m.y, m.y - a.z * m.x - a.w * m.y);
}

// _apply_prefilter(): each pixel's chromaticity mixed towards the filter's by its
// saturation's weight, so chroma does not blur into achromatic areas. The chromaticity
// it filters is its blur where the filter blurs in place, as darktable's in-place blur
// leaves it, otherwise the image's.
[shader("compute")]
[numthreads(16, 16, 1)]
void prefilter_apply(uint3 id: SV_DispatchThreadID)
{
    ColorEqualParams p = params.ce;
    if (id.x >= p.size.x || id.y >= p.size.y)
        return;
    uint k = id.y * p.size.x + id.x;
    float2 base = p.mode.w != 0 ? grid_uv[k] : uv[k];
    Coefficients c = coefficients_at(id.xy, k, p);
    float2 cv = float2(c.a.x * base.x + c.a.y * base.y + c.b.x, c.a.z * base.x + c.a.w * base.y + c.b.y);
    float w = satweight(0, saturation[k] - p.shifts.x);
    uv[k] = w * (cv - base) + base;
}

// darktable's scharr_gradient() of the blurred saturation around (row, col), its
// neighbours clamped to the image, which they always are but on an image under three
// pixels across or down, where darktable reads outside it.
float scharr_at(int row, int col, ColorEqualParams p)
{
    int w = int(p.size.x);
    int h = int(p.size.y);
    int up = clamp(row - 1, 0, h - 1) * w;
    int mid = clamp(row, 0, h - 1) * w;
    int down = clamp(row + 1, 0, h - 1) * w;
    int left = clamp(col - 1, 0, w - 1);
    int centre = clamp(col, 0, w - 1);
    int right = clamp(col + 1, 0, w - 1);
    float nw = saturation[up + left];
    float n = saturation[up + centre];
    float ne = saturation[up + right];
    float west = saturation[mid + left];
    float east = saturation[mid + right];
    float sw = saturation[down + left];
    float s = saturation[down + centre];
    float se = saturation[down + right];
    float gx = 47.0 / 255.0 * (nw - ne + sw - se) + 162.0 / 255.0 * (west - east);
    float gy = 47.0 / 255.0 * (nw - sw + ne - se) + 162.0 / 255.0 * (n - s);
    return sqrt(gx * gx + gy * gy);
}

// process() STEP 3 with the guided filter, before the guide: each pixel's HSB from the
// prefiltered chromaticity, its saturation's gradient in place of its L*, its
// corrections from the curves, and its saturation's weights for the guide.
[shader("compute")]
[numthreads(16, 16, 1)]
void corrections_pass(uint3 id: SV_DispatchThreadID)
{
    ColorEqualParams p = params.ce;
    if (id.x >= p.size.x || id.y >= p.size.y)
        return;
    uint k = id.y * p.size.x + id.x;
    float3 jch = luv_to_jch(lightness[k], p.white.x, uv[k]);
    float3 hsb = ucs_jch_to_hsb(jch);

    int vrow = min(int(p.size.y) - 2, max(1, int(id.y)));
    int vcol = min(int(p.size.x) - 2, max(1, int(id.x)));
    float gradient = max(0.0, scharr_at(vrow, vcol, p) - 0.02);
    lightness[k] = p.shifts.w * (gradient * gradient);

    float2 c = float2(0.0, 1.0);
    float b = 0.0;
    if (jch.y > NORM_MIN)
    {
        c = float2(lookup(TABLE_HUE, hsb.x), lookup(TABLE_SATURATION, hsb.x));
        b = hsb.y * (lookup(TABLE_BRIGHTNESS, hsb.x) - 1.0);
    }
    corrections[k] = c;
    brightness[k] = b;
    float sat = saturation[k];
    weights[k] = float2(satweight(TABLE_CORRECTION_WEIGHTS, sat - p.shifts.x),
        satweight(TABLE_CORRECTION_WEIGHTS, sat - p.shifts.y));
}

// _guide_with_chromaticity()'s downsampling of the corrections onto its grid, or their
// copy where a smaller view's grid is the image but the export's is not.
[shader("compute")]
[numthreads(16, 16, 1)]
void guide_downsample(uint3 id: SV_DispatchThreadID)
{
    ColorEqualParams p = params.ce;
    if (id.x >= p.size.z || id.y >= p.size.w)
        return;
    uint d = id.y * p.size.z + id.x;
    if (p.mode.y == 0)
    {
        grid_corrections[d] = corrections[d];
        grid_brightness[d] = brightness[d];
        return;
    }
    uint w = p.size.x;
    Taps t = bilinear(id.xy, p.size.xy, p.size.zw);
    uint nw = t.y_prev * w + t.x_prev;
    uint ne = t.y_prev * w + t.x_next;
    uint sw = t.y_next * w + t.x_prev;
    uint se = t.y_next * w + t.x_next;
    grid_corrections[d] = interpolate(t, corrections[nw], corrections[ne], corrections[sw], corrections[se]);
    grid_brightness[d] = interpolate(t, brightness[nw], brightness[ne], brightness[sw], brightness[se]);
}

// The guide's start on its grid: the chromaticity, its covariance's products and its
// correlations with the saturation and brightness corrections, before their averages.
[shader("compute")]
[numthreads(16, 16, 1)]
void guide_init(uint3 id: SV_DispatchThreadID)
{
    ColorEqualParams p = params.ce;
    if (id.x >= p.size.z || id.y >= p.size.w)
        return;
    uint d = id.y * p.size.z + id.x;
    float2 u = uv_on_grid(id.xy, p);
    float c = grid_corrections[d].y;
    float b = grid_brightness[d];
    grid_uv[d] = u;
    p1[d] = float2(u.x * u.x, u.x * u.y);
    p2[d] = float2(u.x * u.y, u.y * u.y);
    c1[d] = float2(u.x * c, u.y * c);
    c2[d] = float2(u.x * b, u.y * b);
}

// The guide's coefficients on the blurred grid: the covariance and the correlations
// finished, then a and b with saturation′ = a0 U + a1 V + b0 and
// brightness′ = a2 U + a3 V + b1.
[shader("compute")]
[numthreads(16, 16, 1)]
void guide_coefficients(uint3 id: SV_DispatchThreadID)
{
    ColorEqualParams p = params.ce;
    if (id.x >= p.size.z || id.y >= p.size.w)
        return;
    uint d = id.y * p.size.z + id.x;
    float2 m = grid_uv[d];
    float mc = grid_corrections[d].y;
    float mb = grid_brightness[d];
    float2 q1 = p1[d];
    float2 q2 = p2[d];
    float2 r1 = c1[d];
    float2 r2 = c2[d];
    float4 cov = float4(q1.x - m.x * m.x, q1.y - m.x * m.y, q2.x - m.x * m.y, q2.y - m.y * m.y);
    float4 corr = float4(r1.x - m.x * mc, r1.y - m.y * mc, r2.x - m.x * mb, r2.y - m.y * mb);
    float eps = p.shifts.z;
    bool invertible;
    float4 inv = inverse_of(float4(cov.x + eps, cov.y, cov.z, cov.w + eps), invertible);
    float4 a = float4(0.0);
    if (invertible)
        a = float4(corr.x * inv.x + corr.y * inv.y, corr.x * inv.z + corr.y * inv.w, corr.z * inv.x + corr.w * inv.y,
            corr.z * inv.z + corr.w * inv.w);
    p1[d] = a.xy;
    p2[d] = a.zw;
    c1[d] = float2(mc - a.x * m.x - a.y * m.y, mb - a.z * m.x - a.w * m.y);
}

// The guide applied: the saturation correction re-expressed from the chromaticity and
// weighted by the saturation, the brightness correction likewise and held off sharp
// saturation edges. The hue correction stays: blurred where the filter blurs in place,
// as darktable's in-place blur leaves it.
[shader("compute")]
[numthreads(16, 16, 1)]
void guide_apply(uint3 id: SV_DispatchThreadID)
{
    ColorEqualParams p = params.ce;
    if (id.x >= p.size.x || id.y >= p.size.y)
        return;
    uint k = id.y * p.size.x + id.x;
    float2 base = grid_uv[k];
    Coefficients c = coefficients_at(id.xy, k, p);
    float2 cv = float2(c.a.x * base.x + c.a.y * base.y + c.b.x, c.a.z * base.x + c.a.w * base.y + c.b.y);
    float2 w = weights[k];
    corrections[k] = float2(corrections[k].x, 1.0 + (cv.x - 1.0) * w.x);
    float gradient_weight = 1.0 - clamp(lightness[k], 0.0, 1.0);
    brightness[k] = cv.y * gradient_weight * w.y;
}
