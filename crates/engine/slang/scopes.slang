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

// The scopes (docs/gui.md, The scopes): the viewport's frame counted, as the window shows
// it, after colorout and before any rounding to 8 bits. Always three totals: the pixels
// counted, those with any of red, green and blue at or below 0, and those with any at or
// above 1. While asked for, the histogram, 256 bins evenly spaced from 0 to 1 for red,
// green, blue and luminance, and the waveform, 128 levels in each of 256 columns of the
// frame for the same four. A value below 0 counts in the first bin or level, one at or
// above 1 in the last. Luminance is the encoded relative luminance: the frame read as
// sRGB, as the window shows it, Rec. 709's luminance of its linear light, encoded again,
// so a grey pixel falls in the same bin in all four channels.
//
// And while asked for, the vectorscope, 128 × 128 cells over darktable's default
// plane (src/libs/scopes/vectorscope.c, _vec_process), computed as darktable computes it:
// the mean of each 2 × 2 block of the frame, read through sRGB, darktable's default
// histogram profile, as the frame is encoded; its CIE 1976 u*v* against D50; on
// darktable's logarithmic scale; in the cell darktable bins it in, laid out as darktable
// shows it by default (scopes.rs, Plane).
//
// Counts are integers, so atomics add them: the totals and the histogram first in
// workgroup memory, then each workgroup's non-zero counts into the counts buffer, which
// the engine clears before the pass; the waveform and the vectorscope, larger than
// workgroup memory, straight into it. The engine's own pass, read back after the frame:
// no golden case and no derivatives.
//
// The frame holds no NaN or infinity, since every operation sanitises its input
// (AGENTS.md). Were one there, an infinity would count in an end bin and its clipped
// total, a NaN in a bin the GPU chooses and in neither total; no value can index outside
// the counts.

import color;
import pass;

static const uint BINS = 256;
static const uint COLUMNS = 256;
static const uint LEVELS = 128;
static const uint CHANNELS = 4;
static const uint CELLS = 128;

// Where each count lives in the counts buffer, in 32-bit words: the totals (pixels,
// shadows, highlights and an unused word), then the histogram, channel after channel,
// then the vectorscope, row after row from the top, then the waveform, channel after
// channel, column after column from the frame's left, the darkest level first.
static const uint TOTALS = 0;
static const uint HISTOGRAM = 4;
static const uint VECTORSCOPE = HISTOGRAM + CHANNELS * BINS;
static const uint WAVEFORM = VECTORSCOPE + CELLS * CELLS;
// The counts a workgroup adds up in its own memory first: the totals and the histogram.
static const uint SHARED = VECTORSCOPE;

// Each invocation counts BLOCK × BLOCK pixels, 16 apart, so each row of a workgroup's
// invocations reads 16 neighbouring pixels at a time and a workgroup counts 64 × 64.
static const uint BLOCK = 4;
static const uint STEP = 16;

// Rec. 709's, and sRGB's, luminance of red and blue; green's is what they leave.
static const float RED = 0.2126;
static const float BLUE = 0.0722;

// darktable's FLT_MIN, below which its logarithmic scale leaves a chroma as it is.
static const float FLT_MIN = 1.17549435e-38;
// CIE L*'s linear segment: at and below (6/29)^3, L* is (29/3)^3 Y.
static const float LSTAR_EDGE = (6.0 / 29.0) * (6.0 / 29.0) * (6.0 / 29.0);
static const float LSTAR_SLOPE = (29.0 / 3.0) * (29.0 / 3.0) * (29.0 / 3.0);

struct ScopesParams
{
    // Non-zero to count the histogram; the waveform; the vectorscope.
    uint histogram;
    uint waveform;
    uint vectorscope;
    uint _pad0;
    // The vectorscope's plane (scopes.rs, Plane): sRGB's linear RGB to XYZ D50, by rows;
    float4 to_xyz[3];
    // darktable's power law for a value at or above 1, top × v^exponent;
    float top;
    float exponent;
    // and the largest chroma of sRGB's hue ring, the distance from neutral to the grid's
    // edge.
    float radius;
    float _pad1;
};

struct Params
{
    PassHeader pass;
    ScopesParams scopes;
};

[[vk::binding(0, 0)]] StructuredBuffer<float4> frame;
[[vk::binding(1, 0)]] RWStructuredBuffer<Atomic<uint>> counts;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;

groupshared Atomic<uint> shared_counts[SHARED];

// sRGB's tone curve inverted, extended linearly below 0, as colorout's linear profiles
// may leave values there.
float srgb_to_linear(float v)
{
    return v <= 0.04045 ? v / 12.92 : pow0((v + 0.055) / 1.055, 2.4);
}

// sRGB's tone curve as colorout applies it, 0 below 0.
float linear_to_srgb(float v)
{
    float x = max(v, 0.0);
    return x <= 0.0031308 ? 12.92 * x : 1.055 * pow0(x, 1.0 / 2.4) - 0.055;
}

// darktable dt_ioppr_apply_trc through its sRGB profile: the curve below 1, as its table
// samples it, 0 below 0 where the table clamps; the power law fitted to the curve's top
// at and above 1.
float profile_linear(float v, ScopesParams p)
{
    return v < 1.0 ? srgb_to_linear(max(v, 0.0)) : p.top * pow0(v, p.exponent);
}

// darktable _get_chromaticity for the CIELUV vectorscope: the colour in XYZ D50 through
// the profile (dt_ioppr_rgb_matrix_to_xyz), its chromaticity xy (dt_D50_XYZ_to_xyY, D50
// for black) and u'v' (dt_xyY_to_uvY), then L* and u*v* against D50 (dt_xyY_to_Luv).
float2 chroma(float3 rgb, ScopesParams p)
{
    float3 linear = float3(profile_linear(rgb.r, p), profile_linear(rgb.g, p), profile_linear(rgb.b, p));
    float3 xyz = max(float3(dot(p.to_xyz[0].xyz, linear), dot(p.to_xyz[1].xyz, linear),
                            dot(p.to_xyz[2].xyz, linear)), 0.0);
    float sum = xyz.x + xyz.y + xyz.z;
    float2 xy = sum > 0.0 ? xyz.xy / sum : D50_XY;
    float2 uv = xy_to_uv(xy);
    float lstar = xyz.y <= LSTAR_EDGE ? LSTAR_SLOPE * xyz.y : 116.0 * pow0(xyz.y, 1.0 / 3.0) - 16.0;
    return 13.0 * lstar * (uv - D50_UV);
}

// darktable log_scale and baselog: a chroma at h from neutral moved to
// log(1 + 29 h / radius) / log(30) × radius, its hue kept. WGSL has no log1p.
float2 log_scale(float2 c, float radius)
{
    float h = sqrt(c.x * c.x + c.y * c.y);
    if (h >= FLT_MIN)
        c *= log(1.0 + 29.0 * h / radius) / log(30.0) * radius / h;
    return c;
}

// Counts a sample of the vectorscope, the mean of a 2 × 2 block, where darktable bins it:
// in cell (int)(127 × (c / (2 radius) + 0.5)) across u* and v*, truncated toward 0, so a
// value just below the first cell's edge still counts in it, and left out off the grid.
// Laid out as darktable shows it by default, turned 270°: u* up, v* to the left.
void count_vectorscope(float3 rgb, ScopesParams p)
{
    float2 c = log_scale(chroma(rgb, p), p.radius);
    float2 at = float(CELLS - 1) * (c / (2.0 * p.radius) + 0.5);
    if (all(at > -1.0) && all(at < float(CELLS)))
    {
        // Limited before it is converted and after, as a bin is.
        uint2 cell = min(uint2(max(at, 0.0)), uint2(CELLS - 1));
        counts[VECTORSCOPE + (CELLS - 1 - cell.x) * CELLS + (CELLS - 1 - cell.y)].add(1);
    }
}

// The encoded relative luminance. Green's share is written as what red's and blue's
// leave, so a grey's linear luminance is its linear value exactly, and a grey takes its
// own value, as the round trip through the curve may not give it back to the last bit.
float luminance(float3 rgb)
{
    float r = srgb_to_linear(rgb.r);
    float g = srgb_to_linear(rgb.g);
    float b = srgb_to_linear(rgb.b);
    float encoded = linear_to_srgb(g + RED * (r - g) + BLUE * (b - g));
    return rgb.r == rgb.g && rgb.g == rgb.b ? rgb.g : encoded;
}

// The bin or level of `count` evenly spaced from 0 to 1 that `v` falls in; the first
// below 0, the last at 1 and above. Multiplying by a power of two is exact, so a value
// falls in the same bin on every GPU, and on the CPU. Limited before it is converted,
// since a float beyond a uint's range converts differently on different GPUs, and after,
// since a NaN converts to whatever the GPU chooses.
uint bin(float v, uint count)
{
    return min(uint(min(max(v * float(count), 0.0), float(count - 1))), count - 1);
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 group: SV_GroupID, uint3 local: SV_GroupThreadID, uint index: SV_GroupIndex)
{
    for (uint i = index; i < SHARED; i += 256)
        shared_counts[i].store(0);
    GroupMemoryBarrierWithGroupSync();

    uint width = params.pass.width;
    uint height = params.pass.height;
    bool histogram = params.scopes.histogram != 0;
    bool waveform = params.scopes.waveform != 0;
    uint2 corner = group.xy * (BLOCK * STEP) + local.xy;
    uint pixels = 0;
    uint shadows = 0;
    uint highlights = 0;
    for (uint by = 0; by < BLOCK; by++)
    {
        for (uint bx = 0; bx < BLOCK; bx++)
        {
            uint2 at = corner + uint2(bx, by) * STEP;
            if (at.x < width && at.y < height)
            {
                float3 rgb = frame[at.y * width + at.x].rgb;
                float y = luminance(rgb);
                pixels += 1;
                shadows += any(rgb <= 0.0) ? 1 : 0;
                highlights += any(rgb >= 1.0) ? 1 : 0;
                if (histogram)
                {
                    shared_counts[HISTOGRAM + bin(rgb.r, BINS)].add(1);
                    shared_counts[HISTOGRAM + BINS + bin(rgb.g, BINS)].add(1);
                    shared_counts[HISTOGRAM + 2 * BINS + bin(rgb.b, BINS)].add(1);
                    shared_counts[HISTOGRAM + 3 * BINS + bin(y, BINS)].add(1);
                }
                if (waveform)
                {
                    // Each channel's column, its levels from the darkest.
                    uint column = WAVEFORM + at.x * COLUMNS / width * LEVELS;
                    uint channel = COLUMNS * LEVELS;
                    counts[column + bin(rgb.r, LEVELS)].add(1);
                    counts[column + channel + bin(rgb.g, LEVELS)].add(1);
                    counts[column + 2 * channel + bin(rgb.b, LEVELS)].add(1);
                    counts[column + 3 * channel + bin(y, LEVELS)].add(1);
                }
            }
        }
    }
    if (params.scopes.vectorscope != 0)
    {
        // darktable's samples: the mean of each 2 × 2 block from the frame's top left, a
        // last odd row or column left out. Each invocation takes 2 × 2 blocks, 16 blocks
        // apart, so a workgroup takes the 32 × 32 blocks of its 64 × 64 pixels.
        uint2 blocks = group.xy * (BLOCK * STEP / 2) + local.xy;
        for (uint by = 0; by < 2; by++)
        {
            for (uint bx = 0; bx < 2; bx++)
            {
                uint2 at = (blocks + uint2(bx, by) * STEP) * 2;
                if (at.x + 1 < width && at.y + 1 < height)
                {
                    // Added in darktable's order: down the left column, then the right.
                    uint i = at.y * width + at.x;
                    float3 rgb = frame[i].rgb * 0.25;
                    rgb += frame[i + width].rgb * 0.25;
                    rgb += frame[i + 1].rgb * 0.25;
                    rgb += frame[i + width + 1].rgb * 0.25;
                    count_vectorscope(rgb, params.scopes);
                }
            }
        }
    }
    if (pixels != 0)
        shared_counts[TOTALS].add(pixels);
    if (shadows != 0)
        shared_counts[TOTALS + 1].add(shadows);
    if (highlights != 0)
        shared_counts[TOTALS + 2].add(highlights);
    GroupMemoryBarrierWithGroupSync();

    for (uint i = index; i < SHARED; i += 256)
    {
        uint count = shared_counts[i].load();
        if (count != 0)
            counts[i].add(count);
    }
}
