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

// Haze removal, ported from darktable src/iop/hazeremoval.c, process(): He, Sun and
// Tang's dark channel prior. The ambient light A0 is the mean colour of the brightest
// of the haziest pixels; the transmission t is one less the strength times the
// smallest channel over A0, opened (or, at a negative strength, closed) over a box and
// smoothed by a guided filter with the image as its guide; each pixel becomes
// (I - A0) / max(t, t_min) + A0, t_min following from the distance and the haze of the
// haziest pixels. docs/ports/hazeremoval.md is the spec.
//
// Everything before the last pass is a whole-image pass (decision 15) writing
// into the operation's scratch, which the last pass reads with no readback. They are one
// run (decision 15): only the first reads the input, so in an export's first round of
// tiles each tile writes its own pixels, and every later one reads only scratch laid
// out as the whole image at the pipe's scale:
//
// 0. image_copy: the input, into the image; in the viewport the passes read its
//    full-resolution image itself, which no pass copies (decision 15);
// 1. image_rows, plane_cols: the dark channel, the smallest channel's minimum over a
//    box of 2 dark_radius + 1, darktable's _dark_channel(), into plane B;
// 2. select_begin, then select_count and select_scan for each of three digits, twice:
//    darktable's two quick selects, exactly, as radix selects on the values' bits: the
//    dark channel's 95% quantile, then the 95% quantile of the brightness (R + G + B)
//    of the pixels at or above it;
// 3. ambient_sums, ambient_combine: A0, the mean of the pixels at or above both, and
//    the image's depth, darktable's _ambient_light();
// 4. image_rows, plane_cols, plane_rows, plane_cols: the smallest channel over A0,
//    eroded then dilated over the box (dilated then eroded at a negative strength),
//    darktable's _transition_map() and its dt_box_min(), into plane B;
// 5. guide_coefficients, guide_result: darktable's guided_filter() of that with the
//    image as its guide, into plane A.
// 6. main: the haze-free image.
//
// darktable filters 1 - strength × m; the box filters commute with that monotone map
// and the guided filter is linear in its input and keeps constants, so filtering m and
// applying the strength last is the same transmission, which keeps the strength out of
// the whole-image passes: they see only its sign (docs/ports/hazeremoval.md,
// Deliberate differences).

import color;
import pass;
import box_downscale;
import common.guided_filter;

// darktable's characteristic depth, distance and strength, the two parameters the
// derivative entry points differentiate against, in this order.
struct Dehaze : IDifferentiable
{
    float strength;
    float distance;
};

struct HazeParams
{
    // The image at the pipe's scale.
    uint width;
    uint height;
    // darktable's w1, the dark channel's and the transmission's box radius, and w2, the
    // guided filter's, both at the pipe's scale.
    uint dark_radius;
    uint guide_radius;
    // darktable's p: the rank of the dark channel's 95% quantile among every pixel.
    uint rank;
    // The ambient sums' workgroups across, and in all.
    uint partials_across;
    uint partials;
    // This box filter pass's operation: 0 the minimum, 1 the maximum.
    uint maximum;
    // Whether input_rows filters the smallest channel over A0 (1, the transmission) or
    // the smallest channel (0, the dark channel).
    uint normalise;
    // The selection (0 the dark channel, 1 the brightness) and its digit (0 to 2) this
    // selection pass is at.
    uint selection;
    uint digit;
    // darktable's eps, sqrtf(0.025f) squared.
    float eps;
    // darktable's depth when the critical haze level is not positive: logf(FLT_MAX) / 2.
    float max_depth;
    uint reserved0;
    uint reserved1;
    uint reserved2;
    // main and its derivatives only; zero in every whole-image pass, so changing them
    // leaves those passes' bytes as they were. A struct in a uniform block takes 16
    // bytes, so the block ends 8 bytes after it.
    Dehaze dehaze;
};

struct Params
{
    PassHeader pass;
    HazeParams haze;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;

// A workgroup's part of the ambient sums: the colours it added up and how many, 16 bytes.
struct AmbientPartial
{
    float3 sum;
    uint count;
};

// Scratch: two planes of one float a pixel, the guided filter's coefficients, the
// selections' bins, their state and results, the ambient sums' partial results, and
// the image: the input, also the guided filter's colour guide, or in the viewport the
// full-resolution image its downscale read, bound in its place (decision 15). The
// planes, the coefficients and the image are laid out as the whole image at the pipe's
// scale, or at full resolution in the viewport.
[[vk::binding(10, 0)]] RWStructuredBuffer<float> plane_a;
[[vk::binding(11, 0)]] RWStructuredBuffer<float> plane_b;
[[vk::binding(12, 0)]] RWStructuredBuffer<float4> coefficients;
[[vk::binding(13, 0)]] RWStructuredBuffer<Atomic<uint>> bins;
[[vk::binding(14, 0)]] RWStructuredBuffer<uint> state;
[[vk::binding(15, 0)]] RWStructuredBuffer<AmbientPartial> partials;
[[vk::binding(16, 0)]] RWStructuredBuffer<float4> image;
// In the viewport (decision 15): plane A downscaled to the viewport's size, which main
// then reads as plane A.
[[vk::binding(17, 0)]] RWStructuredBuffer<float> viewport_plane;

static const uint WORKGROUP = 256;
static const uint TILE = 16;

// The words of `state`. The selections' running prefix and rank, then their results,
// then A0, the depth and A0's inverse, as float bits.
static const uint PREFIX = 0;
static const uint RANK = 1;
static const uint CRITICAL_HAZE = 2;
static const uint HAZY = 3;
static const uint CRITICAL_BRIGHTNESS = 4;
static const uint BRIGHT_HAZY = 5;
static const uint AMBIENT = 8;
static const uint DEPTH = 11;
static const uint AMBIENT_INVERSE = 12;

uint pixel_index(int x, int y)
{
    return uint(y) * params.haze.width + uint(x);
}

// An invocation's pixel's place in the whole image at the pipe's scale, for a pass over
// the operation's output: the image itself, but for a tile of an export, which the pass
// header's origin places in it.
uint whole_index(uint3 id)
{
    return pixel_index(int(params.pass.origin_x + id.x), int(params.pass.origin_y + id.y));
}

float3 ambient_light()
{
    return float3(asfloat(state[AMBIENT]), asfloat(state[AMBIENT + 1]), asfloat(state[AMBIENT + 2]));
}

// ---------------------------------------------------------------------------------
// Box minimum and maximum, darktable's dt_box_min() and dt_box_max(): separable, the
// window clipped to the image, so both are exact whatever their order.

// darktable's _dark_channel() value: the smallest channel.
float smallest(float3 rgb)
{
    return min(min(rgb.r, rgb.g), rgb.b);
}

// The input's pixels at their place in the whole image: the operation's one
// whole-image pass that reads its input, writing in an export's first round the pixels
// of its own tile, grown a pixel right and below as every first round's tile is, so the
// column and row it shares with its neighbours are written by both, with the same
// values: an accepted overlap (decision 15; docs/ports/hazeremoval.md, Neighbourhood
// and scale). The viewport needs no copy: its passes bind the full-resolution image.
[shader("compute")]
[numthreads(16, 16, 1)]
void image_copy(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    image[whole_index(id)] = input[id.y * params.pass.width + id.x];
}

// darktable's _dark_channel() and _transition_map() values: the smallest channel, each
// multiplied by A0's inverse for the transmission.
float smallest_channel(float4 pixel)
{
    if (params.haze.normalise != 0)
    {
        float3 inverse = float3(asfloat(state[AMBIENT_INVERSE]),
                                asfloat(state[AMBIENT_INVERSE + 1]),
                                asfloat(state[AMBIENT_INVERSE + 2]));
        float3 rgb = pixel.rgb;
        return min(min(rgb.r * inverse.r, rgb.g * inverse.g), rgb.b * inverse.b);
    }
    return smallest(pixel.rgb);
}

float extreme(float a, float b)
{
    return params.haze.maximum != 0 ? max(a, b) : min(a, b);
}

// The image's smallest channel, filtered along its row into plane A.
[shader("compute")]
[numthreads(16, 16, 1)]
void image_rows(uint3 id: SV_DispatchThreadID)
{
    int width = int(params.haze.width);
    if (id.x >= params.haze.width || id.y >= params.haze.height)
        return;
    int x = int(id.x);
    int y = int(id.y);
    int r = int(params.haze.dark_radius);
    int lo = max(0, x - r);
    int hi = min(width - 1, x + r);
    float m = smallest_channel(image[pixel_index(lo, y)]);
    for (int xx = lo + 1; xx <= hi; xx++)
        m = extreme(m, smallest_channel(image[pixel_index(xx, y)]));
    plane_a[pixel_index(x, y)] = m;
}

// Plane A filtered along its columns into plane B.
[shader("compute")]
[numthreads(16, 16, 1)]
void plane_cols(uint3 id: SV_DispatchThreadID)
{
    int height = int(params.haze.height);
    if (id.x >= params.haze.width || id.y >= params.haze.height)
        return;
    int x = int(id.x);
    int y = int(id.y);
    int r = int(params.haze.dark_radius);
    int lo = max(0, y - r);
    int hi = min(height - 1, y + r);
    float m = plane_a[pixel_index(x, lo)];
    for (int yy = lo + 1; yy <= hi; yy++)
        m = extreme(m, plane_a[pixel_index(x, yy)]);
    plane_b[pixel_index(x, y)] = m;
}

// Plane B filtered along its rows into plane A.
[shader("compute")]
[numthreads(16, 16, 1)]
void plane_rows(uint3 id: SV_DispatchThreadID)
{
    int width = int(params.haze.width);
    if (id.x >= params.haze.width || id.y >= params.haze.height)
        return;
    int x = int(id.x);
    int y = int(id.y);
    int r = int(params.haze.dark_radius);
    int lo = max(0, x - r);
    int hi = min(width - 1, x + r);
    float m = plane_b[pixel_index(lo, y)];
    for (int xx = lo + 1; xx <= hi; xx++)
        m = extreme(m, plane_b[pixel_index(xx, y)]);
    plane_a[pixel_index(x, y)] = m;
}

// ---------------------------------------------------------------------------------
// darktable's _quick_select(), exactly, as a radix select: the value of rank `rank`
// among the selection's values in ascending order, found from the top of their bits
// in three digits of 11, 11 and 10 bits. Each digit counts the values whose higher
// digits match the prefix found so far into bins, with integer atomics (decision 19),
// then one workgroup finds the bin holding the rank.

static const uint BINS = 2048;

// The value's bits in an order the unsigned integers keep: negative values' bits
// flipped, positive values' sign bit set, -0 taken as +0 as a comparison of floats
// takes it.
uint order_key(float value)
{
    uint bits = asuint(value);
    if (bits == 0x80000000u)
        bits = 0u;
    return (bits & 0x80000000u) != 0u ? ~bits : bits | 0x80000000u;
}

float from_order_key(uint key)
{
    return asfloat((key & 0x80000000u) != 0u ? key & 0x7fffffffu : ~key);
}

// The bits below this digit, and the digit's own.
uint digit_shift(uint digit)
{
    return digit == 0u ? 21u : (digit == 1u ? 10u : 0u);
}

uint digit_bits(uint digit)
{
    return digit == 2u ? 10u : 11u;
}

// darktable's brightness of a pixel: pixel_in[0] + pixel_in[1] + pixel_in[2].
float brightness(float3 rgb)
{
    return (rgb.r + rgb.g) + rgb.b;
}

// Clears the bins and starts the first selection.
[shader("compute")]
[numthreads(16, 16, 1)]
void select_begin(uint t: SV_GroupIndex)
{
    for (uint bin = t; bin < BINS; bin += WORKGROUP)
        bins[bin].store(0u);
    if (t == 0u)
    {
        state[PREFIX] = 0u;
        state[RANK] = params.haze.rank;
    }
}

groupshared Atomic<uint> shared_bins[BINS];

// Pixels a side each invocation of the counts and the ambient sums adds up, so fewer,
// larger workgroups each clear and add up their 8 KB of bins once.
static const uint BLOCK = 4;

// Counts this digit of the selection's values whose higher digits match the prefix,
// over a block of pixels per invocation.
[shader("compute")]
[numthreads(16, 16, 1)]
void select_count(uint3 id: SV_DispatchThreadID, uint t: SV_GroupIndex)
{
    for (uint bin = t; bin < BINS; bin += WORKGROUP)
        shared_bins[bin].store(0u);
    GroupMemoryBarrierWithGroupSync();
    uint digit = params.haze.digit;
    uint mask = (1u << digit_bits(digit)) - 1u;
    uint prefix = state[PREFIX];
    float critical_haze = asfloat(state[CRITICAL_HAZE]);
    for (uint by = 0u; by < BLOCK; by++)
    {
        for (uint bx = 0u; bx < BLOCK; bx++)
        {
            uint x = BLOCK * id.x + bx;
            uint y = BLOCK * id.y + by;
            if (x >= params.haze.width || y >= params.haze.height)
                continue;
            uint i = pixel_index(int(x), int(y));
            float dark = plane_b[i];
            float value = dark;
            bool counted = true;
            if (params.haze.selection != 0u)
            {
                value = brightness(image[i].rgb);
                counted = dark >= critical_haze;
            }
            uint key = order_key(value);
            if (counted && (digit == 0u || (key >> digit_shift(digit - 1u)) == prefix))
                shared_bins[(key >> digit_shift(digit)) & mask].add(1u);
        }
    }
    GroupMemoryBarrierWithGroupSync();
    for (uint bin = t; bin < BINS; bin += WORKGROUP)
    {
        uint count = shared_bins[bin].load();
        if (count != 0u)
            bins[bin].add(count);
    }
}

groupshared uint shared_sums[WORKGROUP];

// One workgroup: the bin the rank falls in, which extends the prefix; the bins cleared
// for the next digit. After the last digit, the selection's value; after the first
// selection, the second begins over the pixels at or above it.
[shader("compute")]
[numthreads(16, 16, 1)]
void select_scan(uint t: SV_GroupIndex)
{
    uint digit = params.haze.digit;
    uint per = (1u << digit_bits(digit)) / WORKGROUP;
    uint first = t * per;
    // Read before the barriers, so the thread that finds the bin cannot change them
    // under another.
    uint rank = state[RANK];
    uint prefix = state[PREFIX];
    uint sum = 0u;
    for (uint k = 0u; k < per; k++)
        sum += bins[first + k].load();
    shared_sums[t] = sum;
    GroupMemoryBarrierWithGroupSync();
    for (uint offset = 1u; offset < WORKGROUP; offset *= 2u)
    {
        uint add = t >= offset ? shared_sums[t - offset] : 0u;
        GroupMemoryBarrierWithGroupSync();
        shared_sums[t] += add;
        GroupMemoryBarrierWithGroupSync();
    }
    uint inclusive = shared_sums[t];
    uint below = inclusive - sum;
    if (rank >= below && rank < inclusive)
    {
        uint bin = first;
        for (uint k = 0u; k < per; k++)
        {
            uint count = bins[first + k].load();
            if (rank < below + count)
            {
                bin = first + k;
                break;
            }
            below += count;
        }
        rank -= below;
        prefix = (prefix << digit_bits(digit)) | bin;
        state[PREFIX] = prefix;
        state[RANK] = rank;
        if (digit == 2u)
        {
            float value = from_order_key(prefix);
            if (params.haze.selection == 0u)
            {
                state[CRITICAL_HAZE] = asuint(value);
                // Every pixel at or above the critical haze level: the pixels less the
                // `rank` the selection started from, less the rank among equal values
                // it ended with.
                uint hazy = params.haze.width * params.haze.height - (params.haze.rank - rank);
                state[HAZY] = hazy;
                state[PREFIX] = 0u;
                // darktable's (size_t)(N_most_hazy * bright_quantil).
                state[RANK] = uint(float(hazy) * 0.95f);
            }
            else
            {
                state[CRITICAL_BRIGHTNESS] = asuint(value);
            }
        }
    }
    for (uint k = 0u; k < per; k++)
        bins[first + k].store(0u);
}

// ---------------------------------------------------------------------------------
// darktable's _ambient_light(): A0, the mean colour of the pixels at or above both
// critical levels, and the image's depth. WGSL has no float atomics, so each
// workgroup writes a partial sum, and one workgroup adds them up.

groupshared float4 shared_ambient[WORKGROUP];
groupshared uint shared_counts[WORKGROUP];

[shader("compute")]
[numthreads(16, 16, 1)]
void ambient_sums(uint3 id: SV_DispatchThreadID, uint3 group: SV_GroupID, uint t: SV_GroupIndex)
{
    float4 sum = float4(0.0);
    uint count = 0u;
    float critical_haze = asfloat(state[CRITICAL_HAZE]);
    float critical_brightness = asfloat(state[CRITICAL_BRIGHTNESS]);
    for (uint by = 0u; by < BLOCK; by++)
    {
        for (uint bx = 0u; bx < BLOCK; bx++)
        {
            uint x = BLOCK * id.x + bx;
            uint y = BLOCK * id.y + by;
            if (x >= params.haze.width || y >= params.haze.height)
                continue;
            uint i = pixel_index(int(x), int(y));
            float3 rgb = image[i].rgb;
            if (plane_b[i] >= critical_haze && brightness(rgb) >= critical_brightness)
            {
                sum += float4(rgb, 0.0);
                count += 1u;
            }
        }
    }
    shared_ambient[t] = sum;
    shared_counts[t] = count;
    GroupMemoryBarrierWithGroupSync();
    for (uint stride = WORKGROUP / 2u; stride > 0u; stride /= 2u)
    {
        if (t < stride)
        {
            shared_ambient[t] += shared_ambient[t + stride];
            shared_counts[t] += shared_counts[t + stride];
        }
        GroupMemoryBarrierWithGroupSync();
    }
    if (t == 0u)
    {
        AmbientPartial partial;
        partial.sum = shared_ambient[0].xyz;
        partial.count = shared_counts[0];
        partials[group.y * params.haze.partials_across + group.x] = partial;
    }
}

// One workgroup: A0, the depth, and A0's inverse for the transmission.
[shader("compute")]
[numthreads(16, 16, 1)]
void ambient_combine(uint t: SV_GroupIndex)
{
    float4 sum = float4(0.0);
    uint count = 0u;
    for (uint i = t; i < params.haze.partials; i += WORKGROUP)
    {
        AmbientPartial partial = partials[i];
        sum += float4(partial.sum, 0.0);
        count += partial.count;
    }
    shared_ambient[t] = sum;
    shared_counts[t] = count;
    GroupMemoryBarrierWithGroupSync();
    for (uint stride = WORKGROUP / 2u; stride > 0u; stride /= 2u)
    {
        if (t < stride)
        {
            shared_ambient[t] += shared_ambient[t + stride];
            shared_counts[t] += shared_counts[t + stride];
        }
        GroupMemoryBarrierWithGroupSync();
    }
    if (t == 0u)
    {
        uint n = shared_counts[0];
        float3 ambient = n > 0u ? shared_ambient[0].xyz / float(n) : float3(0.0);
        // The haze of the haziest pixels in units of the characteristic haze depth,
        // 1.125 times to make up for the quantile; a large depth when there is none.
        float critical_haze = asfloat(state[CRITICAL_HAZE]);
        float depth = critical_haze > 0.0 ? -1.125 * log(critical_haze) : params.haze.max_depth;
        // darktable divides by A0 unguarded, which makes a black A0 infinite.
        float3 inverse = 1.0 / away_from_zero(ambient, NORM_MIN);
        state[BRIGHT_HAZY] = n;
        state[AMBIENT] = asuint(ambient.r);
        state[AMBIENT + 1] = asuint(ambient.g);
        state[AMBIENT + 2] = asuint(ambient.b);
        state[DEPTH] = asuint(depth);
        state[AMBIENT_INVERSE] = asuint(inverse.r);
        state[AMBIENT_INVERSE + 1] = asuint(inverse.g);
        state[AMBIENT_INVERSE + 2] = asuint(inverse.b);
    }
}

// ---------------------------------------------------------------------------------
// darktable's guided_filter() with the image as its colour guide, guide weight 1 and
// no clipping: the box means are darktable's dt_box_mean() of radius guide_radius,
// along the rows then the columns, each divided by the pixels its window clipped to
// the image covers. A workgroup filters a 16×16 tile, the tile's rows and
// guide_radius rows above and below a chunk of 8 at a time: its invocations load the
// chunk's pixels, the tile's columns and guide_radius more each side, into shared
// memory once, then add up each row's means for the tile's columns from there, every
// mean the pass needs in one sweep, and each invocation adds up the rows of its own
// pixel's column among them.

// Rows at a time, and the pixels a row of a chunk loads at most.
static const uint CHUNK = 8;
static const uint GUIDE_RADIUS_MAX = 9;
static const uint CHUNK_SPAN = TILE + 2 * GUIDE_RADIUS_MAX;

// A chunk's pixels: the guide's RGB and the input p in w, or the coefficients.
groupshared float4 chunk_pixels[CHUNK * CHUNK_SPAN];
// Its rows' means for the tile's columns.
groupshared float4 chunk_first[CHUNK * TILE];
groupshared float4 chunk_second[CHUNK * TILE];
groupshared float4 chunk_third[CHUNK * TILE];
groupshared float chunk_blue[CHUNK * TILE];

GuideMeans no_means()
{
    GuideMeans m;
    m.first = float4(0.0);
    m.second = float4(0.0);
    m.third = float4(0.0);
    m.blue_blue = 0.0;
    return m;
}

GuideMeans add_means(GuideMeans a, GuideMeans b)
{
    GuideMeans m;
    m.first = a.first + b.first;
    m.second = a.second + b.second;
    m.third = a.third + b.third;
    m.blue_blue = a.blue_blue + b.blue_blue;
    return m;
}

GuideMeans divide_means(GuideMeans a, float n)
{
    GuideMeans m;
    m.first = a.first / n;
    m.second = a.second / n;
    m.third = a.third / n;
    m.blue_blue = a.blue_blue / n;
    return m;
}

// The thirteen values whose box means the guided filter takes, from a pixel's guide I
// (xyz) and input p (w): p, I, their products, and the guide's products with itself.
GuideMeans guide_products(float4 pixel)
{
    float3 guide = pixel.xyz;
    float p = pixel.w;
    GuideMeans m;
    m.first = float4(p, guide.r, guide.g, guide.b);
    m.second = float4(guide.r * p, guide.g * p, guide.b * p, guide.r * guide.r);
    m.third = float4(guide.r * guide.g, guide.r * guide.b, guide.g * guide.g, guide.g * guide.b);
    m.blue_blue = guide.b * guide.b;
    return m;
}

// Where the chunk's pixel `e`, row by row across the tile's columns and guide_radius
// more each side, lies in the image, and whether it is there.
struct ChunkPixel
{
    uint slot;
    int x;
    int y;
    bool inside;
};

ChunkPixel chunk_pixel(int2 origin, int chunk, uint e)
{
    int r = int(params.haze.guide_radius);
    int span = int(TILE) + 2 * r;
    int row = int(e) / span;
    int column = int(e) % span;
    ChunkPixel pixel;
    pixel.slot = uint(row) * CHUNK_SPAN + uint(column);
    pixel.x = origin.x - r + column;
    pixel.y = origin.y - r + chunk + row;
    pixel.inside = chunk + row < int(TILE) + 2 * r && pixel.x >= 0
        && pixel.x < int(params.haze.width) && pixel.y >= 0 && pixel.y < int(params.haze.height);
    return pixel;
}

// The pixels a chunk loads.
uint chunk_pixels_count()
{
    return CHUNK * (TILE + 2u * params.haze.guide_radius);
}

// The chunk's guide and input, zero outside the image.
void load_guide_chunk(int2 origin, int chunk, uint t)
{
    for (uint e = t; e < chunk_pixels_count(); e += WORKGROUP)
    {
        ChunkPixel pixel = chunk_pixel(origin, chunk, e);
        float4 value = float4(0.0);
        if (pixel.inside)
        {
            uint i = pixel_index(pixel.x, pixel.y);
            value = float4(image[i].rgb, plane_b[i]);
        }
        chunk_pixels[pixel.slot] = value;
    }
}

// The chunk's coefficients, zero outside the image.
void load_coefficient_chunk(int2 origin, int chunk, uint t)
{
    for (uint e = t; e < chunk_pixels_count(); e += WORKGROUP)
    {
        ChunkPixel pixel = chunk_pixel(origin, chunk, e);
        float4 value = float4(0.0);
        if (pixel.inside)
            value = coefficients[pixel_index(pixel.x, pixel.y)];
        chunk_pixels[pixel.slot] = value;
    }
}

// The row mean this invocation computes for the chunk, if any: one per row and column
// of the tile, from the chunk's pixels `first` to `last` of its row.
struct ChunkEntry
{
    uint row;
    int first;
    int last;
    bool needed;
};

ChunkEntry chunk_entry(int2 origin, int chunk, uint t)
{
    int r = int(params.haze.guide_radius);
    int row = int(t / TILE);
    int x = origin.x + int(t % TILE);
    int y = origin.y - r + chunk + row;
    ChunkEntry entry;
    entry.row = uint(row);
    // The window clipped to the image, as columns of the chunk.
    entry.first = max(0, x - r) - (origin.x - r);
    entry.last = min(int(params.haze.width) - 1, x + r) - (origin.x - r);
    entry.needed = t < CHUNK * TILE && chunk + row < int(TILE) + 2 * r
        && x < int(params.haze.width) && y >= 0 && y < int(params.haze.height);
    return entry;
}

// Whether row `y` of the chunk is in the vertical window of the pixel at `y_out`.
bool in_window(int y, int y_out)
{
    return abs(y - y_out) <= int(params.haze.guide_radius) && y >= 0
        && y < int(params.haze.height);
}

// The rows a pixel's vertical window covers, clipped to the image.
float vertical_hits(int y_out)
{
    int r = int(params.haze.guide_radius);
    int lo = max(0, y_out - r);
    int hi = min(int(params.haze.height) - 1, y_out + r);
    return float(max(hi - lo + 1, 1));
}

// The thirteen box means at this invocation's pixel, for the tile at `origin`. Every
// invocation of the workgroup calls it, for the barriers.
GuideMeans guide_means(int2 origin, int2 local, uint t)
{
    int r = int(params.haze.guide_radius);
    int y_out = origin.y + local.y;
    GuideMeans sum = no_means();
    for (int chunk = 0; chunk < int(TILE) + 2 * r; chunk += int(CHUNK))
    {
        load_guide_chunk(origin, chunk, t);
        GroupMemoryBarrierWithGroupSync();
        ChunkEntry entry = chunk_entry(origin, chunk, t);
        if (t < CHUNK * TILE)
        {
            GuideMeans row = no_means();
            if (entry.needed)
            {
                uint base = entry.row * CHUNK_SPAN;
                for (int column = entry.first; column <= entry.last; column++)
                    row = add_means(row, guide_products(chunk_pixels[base + uint(column)]));
                row = divide_means(row, float(entry.last - entry.first + 1));
            }
            chunk_first[t] = row.first;
            chunk_second[t] = row.second;
            chunk_third[t] = row.third;
            chunk_blue[t] = row.blue_blue;
        }
        GroupMemoryBarrierWithGroupSync();
        for (uint k = 0u; k < CHUNK; k++)
        {
            if (in_window(origin.y - r + chunk + int(k), y_out))
            {
                uint e = k * TILE + uint(local.x);
                sum.first += chunk_first[e];
                sum.second += chunk_second[e];
                sum.third += chunk_third[e];
                sum.blue_blue += chunk_blue[e];
            }
        }
        GroupMemoryBarrierWithGroupSync();
    }
    return divide_means(sum, vertical_hits(y_out));
}

// The box means of the coefficients (a_r, a_g, a_b, b) at this invocation's pixel, as
// guide_means.
float4 coefficient_means(int2 origin, int2 local, uint t)
{
    int r = int(params.haze.guide_radius);
    int y_out = origin.y + local.y;
    float4 sum = float4(0.0);
    for (int chunk = 0; chunk < int(TILE) + 2 * r; chunk += int(CHUNK))
    {
        load_coefficient_chunk(origin, chunk, t);
        GroupMemoryBarrierWithGroupSync();
        ChunkEntry entry = chunk_entry(origin, chunk, t);
        if (t < CHUNK * TILE)
        {
            float4 row = float4(0.0);
            if (entry.needed)
            {
                uint base = entry.row * CHUNK_SPAN;
                for (int column = entry.first; column <= entry.last; column++)
                    row += chunk_pixels[base + uint(column)];
                row /= float(entry.last - entry.first + 1);
            }
            chunk_first[t] = row;
        }
        GroupMemoryBarrierWithGroupSync();
        for (uint k = 0u; k < CHUNK; k++)
        {
            if (in_window(origin.y - r + chunk + int(k), y_out))
                sum += chunk_first[k * TILE + uint(local.x)];
        }
        GroupMemoryBarrierWithGroupSync();
    }
    return sum / vertical_hits(y_out);
}

// The guided filter's coefficients (a_r, a_g, a_b, b) at each pixel.
[shader("compute")]
[numthreads(16, 16, 1)]
void guide_coefficients(uint3 group: SV_GroupID, uint3 local: SV_GroupThreadID, uint t: SV_GroupIndex)
{
    int2 origin = int2(group.xy * TILE);
    int2 at = int2(local.xy);
    GuideMeans means = guide_means(origin, at, t);
    int x = origin.x + at.x;
    int y = origin.y + at.y;
    if (x < int(params.haze.width) && y < int(params.haze.height))
        coefficients[pixel_index(x, y)] = guided_coefficients(means, params.haze.eps);
}

// The filtered transmission's input, the coefficients' box means applied to the guide,
// into plane A.
[shader("compute")]
[numthreads(16, 16, 1)]
void guide_result(uint3 group: SV_GroupID, uint3 local: SV_GroupThreadID, uint t: SV_GroupIndex)
{
    int2 origin = int2(group.xy * TILE);
    int2 at = int2(local.xy);
    float4 mean = coefficient_means(origin, at, t);
    int x = origin.x + at.x;
    int y = origin.y + at.y;
    if (x < int(params.haze.width) && y < int(params.haze.height))
    {
        uint i = pixel_index(x, y);
        float3 guide = image[i].rgb;
        float res = mean.x * guide.r + mean.y * guide.g + mean.z * guide.b;
        plane_a[i] = res + mean.w;
    }
}

// In the viewport, from its full-resolution image (decision 15): plane A downscaled to
// the viewport's size by the viewport's own box filter, box_downscale.slang's, which the
// engine's downscale takes too. params.haze holds the source, the full resolution, and
// the pass's size the viewport.
struct PlaneA : IDownscaleSource<float>
{
    float read(uint x, uint y)
    {
        return plane_a[pixel_index(int(x), int(y))];
    }
};

[shader("compute")]
[numthreads(16, 16, 1)]
void plane_downscale(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint2 source = uint2(params.haze.width, params.haze.height);
    uint2 size = uint2(params.pass.width, params.pass.height);
    PlaneA plane;
    viewport_plane[id.y * size.x + id.x] = footprint_mean<float>(plane, id.xy, source, size);
}

// ---------------------------------------------------------------------------------
// The haze-free pixel, darktable's last loop in process(), from the guided filter's
// output at the pixel (plane A), A0 and the depth.

// darktable process(): the transmission one less the strength times the filtered
// smallest channel over A0, no lower than t_min, which the distance and the depth set.
[Differentiable]
float3 dehaze(no_diff float3 rgb, no_diff float guided, no_diff float3 ambient, no_diff float depth, Dehaze d)
{
    float transmission = 1.0 - d.strength * guided;
    // The exponent clamped where t_min is clamped anyway, so a depth far from any
    // image's cannot overflow exp.
    float t_min = clamp(exp(clamp(-d.distance * depth, -80.0, 80.0)), 1.0 / 1024.0, 1.0);
    float t = max(transmission, t_min);
    return (rgb - ambient) / t + ambient;
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    float guided = plane_a[whole_index(id)];
    float3 ambient = ambient_light();
    float depth = asfloat(state[DEPTH]);
    float4 pixel = input[i];
    float3 rgb = pixel.rgb;
    for (uint k = 0; k < params.pass.repeat; k++)
        rgb = dehaze(rgb, guided, ambient, depth, params.haze.dehaze);
    output[i] = float4(rgb, pixel.a);
}

// Derivative entry points, both per pixel, with parameters ordered (strength,
// distance). They read the scratch the forward render left: the guided filter's output,
// A0 and the depth, none of which the parameters change but for the strength's sign.
[[vk::binding(8, 0)]] StructuredBuffer<float4> derivative_in;
[[vk::binding(9, 0)]] RWStructuredBuffer<float4> derivative_out;

// Reverse mode: given dLoss/dOutput, the gradient of the loss with respect to the
// parameters.
[shader("compute")]
[numthreads(16, 16, 1)]
void hazeremoval_backward(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    float guided = plane_a[whole_index(id)];
    var d = diffPair(params.haze.dehaze);
    bwd_diff(dehaze)(input[i].rgb, guided, ambient_light(), asfloat(state[DEPTH]), d, derivative_in[i].rgb);
    derivative_out[i] = float4(d.d.strength, d.d.distance, 0.0, 0.0);
}

// Forward mode: given a direction in parameter space, the directional derivative of the
// output. Checks the reverse mode.
[shader("compute")]
[numthreads(16, 16, 1)]
void hazeremoval_forward_diff(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    uint i = id.y * params.pass.width + id.x;
    float guided = plane_a[whole_index(id)];
    Dehaze.Differential direction;
    direction.strength = derivative_in[i].x;
    direction.distance = derivative_in[i].y;
    let result = fwd_diff(dehaze)(input[i].rgb, guided, ambient_light(), asfloat(state[DEPTH]),
                                  diffPair(params.haze.dehaze, direction));
    derivative_out[i] = float4(result.d, 0.0);
}
