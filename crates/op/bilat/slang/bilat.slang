// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2012-2026  darktable developers (ported from src/iop/bilat.c,
// src/common/locallaplacian.c, src/common/bilateral.c and src/common/math.h)
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

// Local contrast, ported from darktable src/iop/bilat.c, process(), with its two
// filters: the local laplacian filter (src/common/locallaplacian.c) and the bilateral
// grid (src/common/bilateral.c). See docs/ports/bilat.md; `prepare` in src/setup.rs
// fills the parameters and lists the passes. darktable's bilat is a Lab module: every
// pass converts the working RGB to Lab itself, and `main` converts back. Only lightness
// changes.
//
// The local laplacian filter's pyramid lives in one scratch buffer per level from 1 to
// the last, eight planes of the level's size each: the input's Gaussian G, the six
// remapped images' Gaussians B0 to B5, and the assembled output O. Level 0, the padded
// input, is never stored: darktable's padding replicates the image's edges, so a pass
// that needs it reads the input at clamped coordinates. Binding 10 is the finer level
// (or the grid read), binding 11 the coarser one (or the grid written).

import color;
import pass;

static const uint BILATERAL_GRID = 0;
static const uint LOCAL_LAPLACIAN = 1;
// Images too small for the pyramid pass through.
static const uint COPY = 2;

// darktable num_gamma: the grey levels of the remapped images.
static const uint GAMMAS = 6;
// Planes of a level's buffer.
static const uint PLANE_G = 0;
static const uint PLANE_B = 1;
static const uint PLANE_O = 7;

// darktable's binomial [1 4 6 4 1], applied as integers and divided by 256 after.
static const float BINOMIAL[5] = { 1.0, 4.0, 6.0, 4.0, 1.0 };

// darktable dt_fast_expf: 0x3f800000 and 0x402DF854 - 0x3f800000.
static const float FAST_EXP_ONE = 1065353216.0;
static const float FAST_EXP_SLOPE = 11401300.0;

struct BilatParams
{
    // x, y: the image's width and height at the pipe's scale; z: the local laplacian's
    // padding, max_supp; w: the mode.
    uint4 image;
    // The finer level a pass reads: width, height and plane size; for `main` and
    // `ll_reduce_input`, the padded level 0's.
    uint4 fine;
    // The coarser level: width, height and plane size; w is 1 when it is the last
    // level, whose output plane is its Gaussian.
    uint4 coarse;
    // The local laplacian's curve: x the mid-tone range σ, y shadows (sigma_s), z
    // highlights (sigma_r), w the detail (clarity).
    float4 curve;
    // The bilateral grid: x and y the reciprocals of its spatial and range sigmas, z its
    // spatial sigma squared, w its range sigma.
    float4 grid;
    // The grid's nodes along x, y and z; w the axis a blur pass runs along: 0 x, 1 y,
    // 2 z, the derivative filter.
    uint4 grid_size;
    // The bilateral grid's detail, which only `main` reads: the splat and blurs leave it
    // out, so a change of it leaves their bytes alone (decision 15).
    float4 slice;
    // Working RGB to XYZ D50 and back, by rows.
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
    BilatParams bilat;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;
[[vk::binding(10, 0)]] RWStructuredBuffer<float> fine;
[[vk::binding(11, 0)]] RWStructuredBuffer<float> coarse;

struct Colour
{
    float3x3 rgb_to_xyz;
    float3x3 xyz_to_rgb;
};

Colour colour_of(BilatParams p)
{
    Colour c;
    c.rgb_to_xyz = float3x3(p.rgb_to_xyz_r0.xyz, p.rgb_to_xyz_r1.xyz, p.rgb_to_xyz_r2.xyz);
    c.xyz_to_rgb = float3x3(p.xyz_to_rgb_r0.xyz, p.xyz_to_rgb_r1.xyz, p.xyz_to_rgb_r2.xyz);
    return c;
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

// darktable dt_fast_expf: the float whose bits are the integer part of 0x3f800000 plus
// x times the slope, 0 below; darktable's arm64 build fuses the multiply and add. Its
// derivative is its slope within a binade, where its value is linear in x. Its argument
// here is never positive, so the bits never pass 1's; the bound, 2^127's bits, only
// keeps the conversion defined.
[ForwardDerivative(fast_exp_forward)]
[BackwardDerivative(fast_exp_backward)]
float fast_exp(float x)
{
    float k = fma(x, FAST_EXP_SLOPE, FAST_EXP_ONE);
    if (k <= 0.0)
        return 0.0;
    return asfloat(int(min(k, 2130706432.0)));
}

// The slope of fast_exp where its value is `value`: 2^e times the slope over 2^23, for a
// value in [2^e, 2^(e+1)).
float fast_exp_slope(float value)
{
    if (value <= 0.0)
        return 0.0;
    return asfloat(asuint(value) & 0x7f800000u) * (FAST_EXP_SLOPE / 8388608.0);
}

DifferentialPair<float> fast_exp_forward(DifferentialPair<float> x)
{
    float value = fast_exp(x.p);
    return diffPair(value, fast_exp_slope(value) * x.d);
}

void fast_exp_backward(inout DifferentialPair<float> x, float d_out)
{
    x = diffPair(x.p, fast_exp_slope(fast_exp(x.p)) * d_out);
}

// What the derivative entry points differentiate against, in their order.
struct Settings : IDifferentiable
{
    float detail;
    float shadows;
    float highlights;
    float midtone;
};

Settings settings_of(BilatParams p)
{
    Settings s;
    s.detail = p.curve.w;
    s.shadows = p.curve.y;
    s.highlights = p.curve.z;
    s.midtone = p.curve.x;
    return s;
}

// darktable curve_scalar: the remapping around grey level g, with σ the mid-tone range.
[Differentiable]
float curve(no_diff float x, no_diff float g, Settings s)
{
    float sigma = s.midtone;
    float c = x - g;
    float val;
    // Blend in via a quadratic Bézier.
    if (c > 2.0 * sigma)
        val = g + sigma + s.shadows * (c - sigma);
    else if (c < -2.0 * sigma)
        val = g - sigma + s.highlights * (c + sigma);
    else if (c > 0.0)
    {
        // Shadow contrast.
        float t = clamp(c / (2.0 * sigma), 0.0, 1.0);
        float t2 = t * t;
        float mt = 1.0 - t;
        val = g + sigma * 2.0 * mt * t + t2 * (sigma + sigma * s.shadows);
    }
    else
    {
        // Highlight contrast.
        float t = clamp(-c / (2.0 * sigma), 0.0, 1.0);
        float t2 = t * t;
        float mt = 1.0 - t;
        val = g - sigma * 2.0 * mt * t + t2 * (-sigma - sigma * s.highlights);
    }
    // Mid-tone local contrast.
    val += s.detail * c * fast_exp(-c * c / (2.0 * sigma * sigma / 3.0));
    return val;
}

// darktable's grey levels, (k + .5) / num_gamma.
float gamma_of(uint k)
{
    return (float(k) + 0.5) / float(GAMMAS);
}

// The remapped images that bracket a value of the input's Gaussian, and the weight of
// the upper one: darktable's search for `hi`, then `lo = hi - 1`.
struct Bracket
{
    uint lo;
    float a;
};

Bracket bracket(float v)
{
    uint hi = 1;
    for (uint k = 1; k < GAMMAS - 1; k++)
    {
        if (hi == k && gamma_of(k) <= v)
            hi = k + 1;
    }
    Bracket b;
    b.lo = hi - 1;
    b.a = clamp((v - gamma_of(b.lo)) / (gamma_of(hi) - gamma_of(b.lo)), 0.0, 1.0);
    return b;
}

// The image's lightness over 100 at padded coordinates (x, y), the padded level 0:
// darktable ll_pad_input's replication is the image read at clamped coordinates.
float padded_value(int x, int y, BilatParams p, Colour c)
{
    int max_supp = int(p.image.z);
    uint ix = uint(clamp(x - max_supp, 0, int(p.image.x) - 1));
    uint iy = uint(clamp(y - max_supp, 0, int(p.image.y) - 1));
    return rgb_to_lab(input[iy * p.image.x + ix].rgb, c).x * 0.01;
}

// The input's value and the six remapped values at one pixel of level 0: (G, B0, B1,
// B2) and (B3, B4, B5, 0).
struct Seven
{
    float4 a;
    float4 b;
};

Seven remapped(float v, Settings s)
{
    Seven r;
    r.a = float4(v, curve(v, gamma_of(0), s), curve(v, gamma_of(1), s), curve(v, gamma_of(2), s));
    r.b = float4(curve(v, gamma_of(3), s), curve(v, gamma_of(4), s), curve(v, gamma_of(5), s), 0.0);
    return r;
}

// Level l's seven planes at one pixel.
Seven level_planes(uint index, uint plane_size)
{
    Seven r;
    r.a = float4(fine[index], fine[index + plane_size], fine[index + 2 * plane_size], fine[index + 3 * plane_size]);
    r.b = float4(fine[index + 4 * plane_size], fine[index + 5 * plane_size], fine[index + 6 * plane_size], 0.0);
    return r;
}

// Writes a reduced pixel's seven planes to the coarser level, and at the last level its
// Gaussian as the output plane too (darktable writes it into the output pyramid).
void write_coarse(uint index, Seven sum, BilatParams p)
{
    uint size = p.coarse.z;
    coarse[index] = sum.a.x;
    coarse[index + size] = sum.a.y;
    coarse[index + 2 * size] = sum.a.z;
    coarse[index + 3 * size] = sum.a.w;
    coarse[index + 4 * size] = sum.b.x;
    coarse[index + 5 * size] = sum.b.y;
    coarse[index + 6 * size] = sum.b.z;
    if (p.coarse.w != 0)
        coarse[index + PLANE_O * size] = sum.a.x;
}

// darktable gauss_reduce for level 1, from the input: the 5×5 binomial around
// (2cx, 2cy) of the padded level 0, cx and cy clamped into the coarse level's interior
// (its border copies its neighbours, ll_fill_boundary1), for the input's Gaussian and
// the six remapped images at once.
[shader("compute")]
[numthreads(16, 16, 1)]
void ll_reduce_input(uint3 id: SV_DispatchThreadID)
{
    BilatParams p = params.bilat;
    if (id.x >= p.coarse.x || id.y >= p.coarse.y)
        return;
    Colour c = colour_of(p);
    Settings s = settings_of(p);
    int cx = clamp(int(id.x), 1, int(p.coarse.x) - 2);
    int cy = clamp(int(id.y), 1, int(p.coarse.y) - 2);
    Seven sum = { float4(0.0), float4(0.0) };
    for (int dx = 0; dx < 5; dx++)
    {
        Seven column = { float4(0.0), float4(0.0) };
        for (int dy = 0; dy < 5; dy++)
        {
            Seven r = remapped(padded_value(2 * cx - 2 + dx, 2 * cy - 2 + dy, p, c), s);
            column.a += BINOMIAL[dy] * r.a;
            column.b += BINOMIAL[dy] * r.b;
        }
        sum.a += BINOMIAL[dx] * column.a;
        sum.b += BINOMIAL[dx] * column.b;
    }
    sum.a /= 256.0;
    sum.b /= 256.0;
    write_coarse(id.y * p.coarse.x + id.x, sum, p);
}

// darktable gauss_reduce for levels 2 and up, from the finer level's seven planes.
[shader("compute")]
[numthreads(16, 16, 1)]
void ll_reduce(uint3 id: SV_DispatchThreadID)
{
    BilatParams p = params.bilat;
    if (id.x >= p.coarse.x || id.y >= p.coarse.y)
        return;
    uint cx = uint(clamp(int(id.x), 1, int(p.coarse.x) - 2));
    uint cy = uint(clamp(int(id.y), 1, int(p.coarse.y) - 2));
    Seven sum = { float4(0.0), float4(0.0) };
    for (uint dx = 0; dx < 5; dx++)
    {
        Seven column = { float4(0.0), float4(0.0) };
        for (uint dy = 0; dy < 5; dy++)
        {
            Seven r = level_planes((2 * cy - 2 + dy) * p.fine.x + 2 * cx - 2 + dx, p.fine.z);
            column.a += BINOMIAL[dy] * r.a;
            column.b += BINOMIAL[dy] * r.b;
        }
        sum.a += BINOMIAL[dx] * column.a;
        sum.b += BINOMIAL[dx] * column.b;
    }
    sum.a /= 256.0;
    sum.b /= 256.0;
    write_coarse(id.y * p.coarse.x + id.x, sum, p);
}

// darktable ll_expand_gaussian on the coarser level's plane at `base`, for the finer
// pixel (i, j), which the caller clamps into the expanded range. `cw` is the coarser
// level's width.
float expand(uint base, uint i, uint j, uint cw)
{
    uint ind = base + (j / 2) * cw + i / 2;
    uint parity = (i & 1) + 2 * (j & 1);
    if (parity == 0)
        // Both even, a 3×3 stencil.
        return (4.0 / 256.0)
            * (6.0 * (coarse[ind - cw] + coarse[ind - 1] + 6.0 * coarse[ind] + coarse[ind + 1] + coarse[ind + cw])
               + coarse[ind - cw - 1] + coarse[ind - cw + 1] + coarse[ind + cw - 1] + coarse[ind + cw + 1]);
    if (parity == 1)
        // i odd, a 2×3 stencil.
        return (4.0 / 256.0)
            * (24.0 * (coarse[ind] + coarse[ind + 1])
               + 4.0 * (coarse[ind - cw] + coarse[ind - cw + 1] + coarse[ind + cw] + coarse[ind + cw + 1]));
    if (parity == 2)
        // j odd, a 3×2 stencil.
        return (4.0 / 256.0)
            * (24.0 * (coarse[ind] + coarse[ind + cw])
               + 4.0 * (coarse[ind - 1] + coarse[ind + 1] + coarse[ind + cw - 1] + coarse[ind + cw + 1]));
    // Both odd, a 2×2 stencil.
    return 0.25 * (coarse[ind] + coarse[ind + 1] + coarse[ind + cw] + coarse[ind + cw + 1]);
}

// The range gauss_expand computes before ll_fill_boundary2 copies its border, which
// ll_laplacian clamps to as well: 1 to ((size - 1) & ~1) - 1.
uint expand_clamp(uint x, uint size)
{
    return uint(clamp(int(x), 1, int((size - 1) & ~1u) - 1));
}

// One level of darktable's assembly, coarse to fine: the coarser output expanded, plus
// the Laplacian coefficients of the two remapped images whose grey levels bracket the
// input's Gaussian here, interpolated.
[shader("compute")]
[numthreads(16, 16, 1)]
void ll_assemble(uint3 id: SV_DispatchThreadID)
{
    BilatParams p = params.bilat;
    if (id.x >= p.fine.x || id.y >= p.fine.y)
        return;
    uint ci = expand_clamp(id.x, p.fine.x);
    uint cj = expand_clamp(id.y, p.fine.y);
    uint cw = p.coarse.x;
    uint index = id.y * p.fine.x + id.x;
    float v = fine[index + PLANE_G * p.fine.z];
    Bracket b = bracket(v);
    uint plane_lo = PLANE_B + b.lo;
    float l0 = fine[index + plane_lo * p.fine.z] - expand(plane_lo * p.coarse.z, ci, cj, cw);
    float l1 = fine[index + (plane_lo + 1) * p.fine.z] - expand((plane_lo + 1) * p.coarse.z, ci, cj, cw);
    fine[index + PLANE_O * p.fine.z] = expand(PLANE_O * p.coarse.z, ci, cj, cw) + (l0 * (1.0 - b.a) + l1 * b.a);
}

// The bilateral grid's node index of (x, y, z).
uint node(uint x, uint y, uint z, uint4 size)
{
    return (x + y * size.x) * size.z + z;
}

// The lightness of the input pixel at (x, y) of the image.
float lightness_at(uint x, uint y, BilatParams p, Colour c)
{
    return rgb_to_lab(input[y * p.image.x + x].rgb, c).x;
}

// darktable image_to_grid along one axis: the position clamped to the grid, its cell
// (never the last node) and the fraction past it.
struct GridCoordinate
{
    uint cell;
    float fraction;
};

GridCoordinate grid_coordinate(float position, uint nodes)
{
    float x = clamp(position, 0.0, float(nodes - 1));
    GridCoordinate g;
    g.cell = min(uint(x), nodes - 2);
    g.fraction = x - float(g.cell);
    return g;
}

// darktable dt_bilateral_splat, as a gather: each invocation owns the column of nodes
// at (x, y), clears it, and adds every pixel whose cell along x is x or x - 1, and along
// y is y or y - 1, to its two nodes along z with darktable's weights.
[shader("compute")]
[numthreads(16, 16, 1)]
void bl_splat(uint3 id: SV_DispatchThreadID)
{
    BilatParams p = params.bilat;
    uint4 size = p.grid_size;
    if (id.x >= size.x || id.y >= size.y)
        return;
    Colour c = colour_of(p);
    uint base = node(id.x, id.y, 0, size);
    for (uint z = 0; z < size.z; z++)
        coarse[base + z] = 0.0;
    float sigma = 1.0 / p.grid.x;
    // The pixels whose position along each axis lies within a cell of the column, with a
    // margin past rounding; each is tested exactly below.
    int width = int(p.image.x);
    int height = int(p.image.y);
    int x0 = max(int(floor((float(id.x) - 1.0) * sigma)) - 2, 0);
    int x1 = min(int(ceil((float(id.x) + 1.0) * sigma)) + 2, width - 1);
    int y0 = max(int(floor((float(id.y) - 1.0) * sigma)) - 2, 0);
    int y1 = min(int(ceil((float(id.y) + 1.0) * sigma)) + 2, height - 1);
    for (int j = y0; j <= y1; j++)
    {
        GridCoordinate gy = grid_coordinate(float(j) * p.grid.x, size.y);
        if (gy.cell != id.y && gy.cell + 1 != id.y)
            continue;
        float wy = gy.cell == id.y ? 1.0 - gy.fraction : gy.fraction;
        for (int i = x0; i <= x1; i++)
        {
            GridCoordinate gx = grid_coordinate(float(i) * p.grid.x, size.x);
            if (gx.cell != id.x && gx.cell + 1 != id.x)
                continue;
            float wx = gx.cell == id.x ? 1.0 - gx.fraction : gx.fraction;
            float l = lightness_at(uint(i), uint(j), p, c);
            GridCoordinate gz = grid_coordinate(l * p.grid.y, size.z);
            float contribution = wx * wy * 100.0 / p.grid.z;
            coarse[base + gz.cell] += contribution * (1.0 - gz.fraction);
            coarse[base + gz.cell + 1] += contribution * gz.fraction;
        }
    }
}

// darktable dt_bilateral_blur, one axis per pass: along x and y blur_line's
// [1 4 6 4 1] / 16, along z blur_line_z's derivative filter, both zero outside the
// grid. One invocation per node, (x · size_z + z, y), from the grid at binding 10 to
// the grid at binding 11.
[shader("compute")]
[numthreads(16, 16, 1)]
void bl_blur(uint3 id: SV_DispatchThreadID)
{
    BilatParams p = params.bilat;
    uint4 size = p.grid_size;
    if (id.x >= size.x * size.z || id.y >= size.y)
        return;
    uint x = id.x / size.z;
    uint z = id.x % size.z;
    uint index = node(x, id.y, z, size);
    uint stride;
    uint position;
    uint count;
    if (size.w == 0)
    {
        stride = size.z;
        position = x;
        count = size.x;
    }
    else if (size.w == 1)
    {
        stride = size.x * size.z;
        position = id.y;
        count = size.y;
    }
    else
    {
        stride = 1;
        position = z;
        count = size.z;
    }
    float m2 = position >= 2 ? fine[index - 2 * stride] : 0.0;
    float m1 = position >= 1 ? fine[index - stride] : 0.0;
    float p1 = position + 1 < count ? fine[index + stride] : 0.0;
    float p2 = position + 2 < count ? fine[index + 2 * stride] : 0.0;
    if (size.w == 2)
        coarse[index] = (4.0 / 16.0) * (p1 - m1) + (2.0 / 16.0) * (p2 - m2);
    else
        coarse[index] = fine[index] * (6.0 / 16.0) + (4.0 / 16.0) * (p1 + m1) + (1.0 / 16.0) * (p2 + m2);
}

// What `main` reads from scratch for a pixel, constant for the derivatives.
struct Neighbourhood
{
    uint mode;
    // Local laplacian: the bracket, the coarser output and the two remapped images
    // expanded here.
    uint lo;
    float a;
    float output;
    float lower;
    float upper;
    // Bilateral grid: the grid interpolated here, and its range sigma.
    float grid;
    float sigma_r;
};

// darktable dt_bilateral_slice's trilinear lookup.
float slice(uint x, uint y, float l, BilatParams p)
{
    uint4 size = p.grid_size;
    GridCoordinate gx = grid_coordinate(float(x) * p.grid.x, size.x);
    GridCoordinate gy = grid_coordinate(float(y) * p.grid.x, size.y);
    GridCoordinate gz = grid_coordinate(l * p.grid.y, size.z);
    float xf = gx.fraction;
    float yf = gy.fraction;
    float zf = gz.fraction;
    uint gi = node(gx.cell, gy.cell, gz.cell, size);
    uint ox = size.z;
    uint oy = size.x * size.z;
    uint oz = 1;
    return coarse[gi] * (1.0 - xf) * (1.0 - yf) * (1.0 - zf)
        + coarse[gi + ox] * (xf) * (1.0 - yf) * (1.0 - zf)
        + coarse[gi + oy] * (1.0 - xf) * (yf) * (1.0 - zf)
        + coarse[gi + ox + oy] * (xf) * (yf) * (1.0 - zf)
        + coarse[gi + oz] * (1.0 - xf) * (1.0 - yf) * (zf)
        + coarse[gi + ox + oz] * (xf) * (1.0 - yf) * (zf)
        + coarse[gi + oy + oz] * (1.0 - xf) * (yf) * (zf)
        + coarse[gi + ox + oy + oz] * (xf) * (yf) * (zf);
}

// The pixel at (x, y) of the image, with lightness over 100 `v`.
Neighbourhood neighbourhood(uint x, uint y, float l, BilatParams p)
{
    Neighbourhood n;
    n.mode = p.image.w;
    n.lo = 0;
    n.a = 0.0;
    n.output = 0.0;
    n.lower = 0.0;
    n.upper = 0.0;
    n.grid = 0.0;
    n.sigma_r = p.grid.w;
    if (n.mode == LOCAL_LAPLACIAN)
    {
        uint max_supp = p.image.z;
        uint ci = expand_clamp(x + max_supp, p.fine.x);
        uint cj = expand_clamp(y + max_supp, p.fine.y);
        uint cw = p.coarse.x;
        Bracket b = bracket(l * 0.01);
        n.lo = b.lo;
        n.a = b.a;
        n.output = expand(PLANE_O * p.coarse.z, ci, cj, cw);
        n.lower = expand((PLANE_B + b.lo) * p.coarse.z, ci, cj, cw);
        n.upper = expand((PLANE_B + b.lo + 1) * p.coarse.z, ci, cj, cw);
    }
    else if (n.mode == BILATERAL_GRID)
        n.grid = slice(x, y, l, p);
    return n;
}

// The per-pixel function: the pixel's new lightness, back to working RGB. For the
// local laplacian filter, darktable's last level of the assembly and its write-back:
// level 0's remapped values are the curves at the pixel itself. For the bilateral grid,
// darktable's slice.
[Differentiable]
float3 bilat(no_diff float3 lab, no_diff Neighbourhood n, no_diff Colour c, Settings s)
{
    float l = lab.x;
    if (n.mode == LOCAL_LAPLACIAN)
    {
        float v = lab.x * 0.01;
        float l0 = curve(v, gamma_of(n.lo), s) - n.lower;
        float l1 = curve(v, gamma_of(n.lo + 1), s) - n.upper;
        l = 100.0 * (n.output + (l0 * (1.0 - n.a) + l1 * n.a));
    }
    else if (n.mode == BILATERAL_GRID)
    {
        float norm = -s.detail * n.sigma_r * 0.04;
        l = max(0.0, lab.x + norm * n.grid);
    }
    return lab_to_rgb(float3(l, lab.y, lab.z), c);
}

// The settings at a pass's uniforms: the bilateral grid's detail is in `slice`, the
// local laplacian's in the curve.
Settings main_settings(BilatParams p)
{
    Settings s = settings_of(p);
    if (p.image.w == BILATERAL_GRID)
        s.detail = p.slice.x;
    return s;
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    BilatParams p = params.bilat;
    uint i = id.y * params.pass.width + id.x;
    uint x = params.pass.origin_x + id.x;
    uint y = params.pass.origin_y + id.y;
    Colour c = colour_of(p);
    Settings s = main_settings(p);
    float4 pixel = input[i];
    float3 rgb = pixel.rgb;
    for (uint k = 0; k < params.pass.repeat; k++)
    {
        float3 lab = rgb_to_lab(rgb, c);
        rgb = bilat(lab, neighbourhood(x, y, lab.x, p), c, s);
    }
    output[i] = float4(rgb, pixel.a);
}

// Derivative entry points of `main`, per pixel, over the scratch the forward render
// left (decision 15), with parameters in the order of Settings: detail, shadows
// (sigma_s), highlights (sigma_r), the mid-tone range. They bind what `main` binds but
// its output.
[[vk::binding(8, 0)]] StructuredBuffer<float4> derivative_in;
[[vk::binding(9, 0)]] RWStructuredBuffer<float4> derivative_out;

// Reverse mode: given dLoss/dOutput, the gradient of the loss with respect to the
// settings.
[shader("compute")]
[numthreads(16, 16, 1)]
void bilat_backward(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    BilatParams p = params.bilat;
    uint i = id.y * params.pass.width + id.x;
    uint x = params.pass.origin_x + id.x;
    uint y = params.pass.origin_y + id.y;
    Colour c = colour_of(p);
    float3 lab = rgb_to_lab(input[i].rgb, c);
    Neighbourhood n = neighbourhood(x, y, lab.x, p);
    var s = diffPair(main_settings(p));
    bwd_diff(bilat)(lab, n, c, s, derivative_in[i].rgb);
    derivative_out[i] = float4(s.d.detail, s.d.shadows, s.d.highlights, s.d.midtone);
}

// Forward mode: given a direction in the settings, the directional derivative of the
// output. Checks the reverse mode.
[shader("compute")]
[numthreads(16, 16, 1)]
void bilat_forward_diff(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    BilatParams p = params.bilat;
    uint i = id.y * params.pass.width + id.x;
    uint x = params.pass.origin_x + id.x;
    uint y = params.pass.origin_y + id.y;
    Colour c = colour_of(p);
    float3 lab = rgb_to_lab(input[i].rgb, c);
    Neighbourhood n = neighbourhood(x, y, lab.x, p);
    float4 v = derivative_in[i];
    Settings.Differential direction;
    direction.detail = v.x;
    direction.shadows = v.y;
    direction.highlights = v.z;
    direction.midtone = v.w;
    let result = fwd_diff(bilat)(lab, n, c, diffPair(main_settings(p), direction));
    derivative_out[i] = float4(result.d, 0.0);
}
