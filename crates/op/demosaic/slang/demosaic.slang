// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/iop/demosaic.c,
// src/iop/demosaicing/ppg.c and rcd.c, and data/kernels/demosaic_rcd.cl)
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

// Demosaic for Bayer sensors, ported from darktable: RCD (ratio corrected
// demosaicing, by Luis Sanz Rodríguez, GPL-3) inside a band of PPG at the edges, as
// darktable's CPU path does: `rcd_demosaic` runs `demosaic_ppg` over the whole mosaic
// with a border of 10, then replaces everything 9 or more photosites from the edge.
// Each pass covers the whole mosaic, the active area, one invocation per photosite,
// and the passes that write the output write only the declared crop (decision 23).
//
// RCD's steps follow darktable's CPU loops exactly, bounds included, and keep its
// half-width planes (`lpf` and `PQ_Dir` share one, as on the CPU, and the P and Q
// diagonal filters are stored for odd columns only), indexed with an even row stride
// as darktable's tiles are, so the asymmetries of that packing carry over. The raw
// stage exports no derivatives (decision 21).

import components;
import pass;

struct DemosaicParams
{
    // The mosaic: the active area.
    uint width;
    uint height;
    // Row stride of RCD's planes: the width rounded up to even.
    uint stride;
    // Markesteijn's direction or step, for the passes that run once per direction.
    uint step;
    // The declared crop in the mosaic: the output.
    uint crop_x;
    uint crop_y;
    uint crop_width;
    uint crop_height;
    // RCD works on its input over the processed maximum, and scales back.
    float scale;
    float unscale;
    // X-Trans: the solitary green photosite's position in the 3×3 repeat.
    uint solitary_x;
    uint solitary_y;
    // The filter colour per position of the pattern from the mosaic's corner, 0 red,
    // 1 green, 2 blue, four to a vector: (row & 1) * 2 + (column & 1) for Bayer,
    // (row % 6) * 6 + column % 6 for X-Trans.
    uint4 colors[9];
    // X-Trans: darktable's allhex, the green hexagon around each position of the 3×3
    // repeat, (row % 3) * 3 + column % 3, eight offsets (x, y) each, two to a vector.
    int4 hexes[36];
};

struct Params
{
    PassHeader pass;
    DemosaicParams demosaic;
};

[[vk::binding(0, 0)]] StructuredBuffer<float> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float4> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;

// RCD's planes, darktable's names; PPG keeps its interpolated pixels in rgb0 to rgb2
// first, which RCD's populate step then replaces.
[[vk::binding(10, 0)]] RWStructuredBuffer<float> cfa;
[[vk::binding(11, 0)]] RWStructuredBuffer<float> vh_dir;
// lpf, then PQ_Dir where step 4.1 writes, half width.
[[vk::binding(12, 0)]] RWStructuredBuffer<float> pq_dir;
[[vk::binding(13, 0)]] RWStructuredBuffer<float> p_diff;
[[vk::binding(14, 0)]] RWStructuredBuffer<float> q_diff;
[[vk::binding(15, 0)]] RWStructuredBuffer<float> rgb0;
[[vk::binding(16, 0)]] RWStructuredBuffer<float> rgb1;
[[vk::binding(17, 0)]] RWStructuredBuffer<float> rgb2;

static const float EPS = 1e-5;
static const float EPSSQ = 1e-10;
// darktable RCD_MARGIN: RCD's result replaces PPG's this far from the edges.
static const int RCD_MARGIN = 9;

uint color_at(uint position)
{
    return params.demosaic.colors[position / 4][position % 4];
}

// darktable FC(): the filter colour of a photosite of a Bayer mosaic.
uint fc(int row, int col)
{
    return color_at(uint(((row & 1) << 1) | (col & 1)));
}

float mosaic(int col, int row)
{
    return input[row * int(params.demosaic.width) + col];
}

int plane(int col, int row)
{
    return row * int(params.demosaic.stride) + col;
}

float sqr(float x)
{
    return x * x;
}

// darktable interpolatef(a, b, c): a * (b - c) + c.
float interpolate(float a, float b, float c)
{
    return a * (b - c) + c;
}

// Writes a pixel of the mosaic to the output, if it lies in the declared crop.
void write_output(int col, int row, float3 rgb)
{
    int x = col - int(params.demosaic.crop_x);
    int y = row - int(params.demosaic.crop_y);
    if (x < 0 || y < 0 || x >= int(params.demosaic.crop_width) || y >= int(params.demosaic.crop_height))
        return;
    output[y * int(params.demosaic.crop_width) + x] = float4(max(rgb, 0.0), 0.0);
}

void store_rgb(int idx, float3 rgb)
{
    rgb0[idx] = rgb.x;
    rgb1[idx] = rgb.y;
    rgb2[idx] = rgb.z;
}

float3 load_rgb(int idx)
{
    return float3(rgb0[idx], rgb1[idx], rgb2[idx]);
}

// The invocation's photosite, or false outside the mosaic.
bool photosite(uint3 id, out int col, out int row)
{
    col = int(id.x);
    row = int(id.y);
    return col < int(params.demosaic.width) && row < int(params.demosaic.height);
}

// Colour c of a PPG border photosite of colour f: the mean of its neighbours of that
// colour, or its own value for its own colour or a colour none of them has.
float ppg_border(uint c, uint f, float sum, float count, float own)
{
    return (c != f && count > 0.0) ? max(0.0, sum / count) : own;
}

// PPG, first loops of `demosaic_ppg`: the outermost 3 photosites from the average of
// each colour in their 3×3 neighbourhood, green everywhere else by PPG's gradients.
[shader("compute")]
[numthreads(16, 16, 1)]
void ppg_green(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row))
        return;
    int width = int(params.demosaic.width);
    int height = int(params.demosaic.height);
    float3 color = float3(0.0);
    if (col < 3 || row < 3 || col >= width - 3 || row >= height - 3)
    {
        float3 sum = float3(0.0);
        float3 count = float3(0.0);
        for (int y = row - 1; y != row + 2; y++)
            for (int x = col - 1; x != col + 2; x++)
                if (y >= 0 && x >= 0 && y < height && x < width)
                {
                    uint f = fc(y, x);
                    sum = add_at(sum, f, mosaic(x, y));
                    count = add_at(count, f, 1.0);
                }
        uint f = fc(row, col);
        float own = max(0.0, mosaic(col, row));
        color = float3(ppg_border(0, f, sum.x, count.x, own), ppg_border(1, f, sum.y, count.y, own),
                       ppg_border(2, f, sum.z, count.z, own));
    }
    else
    {
        uint c = fc(row, col);
        float pc = mosaic(col, row);
        if (c == 0 || c == 2)
        {
            color = set_at(color, c, pc);
            float pym = mosaic(col, row - 1);
            float pym2 = mosaic(col, row - 2);
            float pym3 = mosaic(col, row - 3);
            float pyM = mosaic(col, row + 1);
            float pyM2 = mosaic(col, row + 2);
            float pyM3 = mosaic(col, row + 3);
            float pxm = mosaic(col - 1, row);
            float pxm2 = mosaic(col - 2, row);
            float pxm3 = mosaic(col - 3, row);
            float pxM = mosaic(col + 1, row);
            float pxM2 = mosaic(col + 2, row);
            float pxM3 = mosaic(col + 3, row);
            float guessx = (pxm + pc + pxM) * 2.0 - pxM2 - pxm2;
            float diffx = (abs(pxm2 - pc) + abs(pxM2 - pc) + abs(pxm - pxM)) * 3.0
                        + (abs(pxM3 - pxM) + abs(pxm3 - pxm)) * 2.0;
            float guessy = (pym + pc + pyM) * 2.0 - pyM2 - pym2;
            float diffy = (abs(pym2 - pc) + abs(pyM2 - pc) + abs(pym - pyM)) * 3.0
                        + (abs(pyM3 - pyM) + abs(pym3 - pym)) * 2.0;
            if (diffx > diffy)
                color.y = max(min(guessy * 0.25, max(pym, pyM)), min(pym, pyM));
            else
                color.y = max(min(guessx * 0.25, max(pxm, pxM)), min(pxm, pxM));
        }
        else
            color.y = pc;
        color = max(color, 0.0);
    }
    store_rgb(plane(col, row), color);
}

// PPG, last loop of `demosaic_ppg`: red and blue from the green just interpolated,
// the outermost photosites as they are.
[shader("compute")]
[numthreads(16, 16, 1)]
void ppg_redblue(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row))
        return;
    int width = int(params.demosaic.width);
    int height = int(params.demosaic.height);
    int idx = plane(col, row);
    float3 color = load_rgb(idx);
    if (row > 0 && col > 0 && col < width - 1 && row < height - 1)
    {
        uint c = fc(row, col);
        int s = int(params.demosaic.stride);
        if (c == 1)
        {
            float3 nt = load_rgb(idx - s);
            float3 nb = load_rgb(idx + s);
            float3 nl = load_rgb(idx - 1);
            float3 nr = load_rgb(idx + 1);
            if (fc(row, col + 1) == 0)
            {
                color.z = (nt.z + nb.z + 2.0 * color.y - nt.y - nb.y) * 0.5;
                color.x = (nl.x + nr.x + 2.0 * color.y - nl.y - nr.y) * 0.5;
            }
            else
            {
                color.x = (nt.x + nb.x + 2.0 * color.y - nt.y - nb.y) * 0.5;
                color.z = (nl.z + nr.z + 2.0 * color.y - nl.y - nr.y) * 0.5;
            }
        }
        else
        {
            float3 ntl = load_rgb(idx - 1 - s);
            float3 ntr = load_rgb(idx + 1 - s);
            float3 nbl = load_rgb(idx - 1 + s);
            float3 nbr = load_rgb(idx + 1 + s);
            // The colour to fill: blue at red, red at blue.
            uint o = 2 - c;
            float diff1 = abs(ntl[o] - nbr[o]) + abs(ntl.y - color.y) + abs(nbr.y - color.y);
            float guess1 = ntl[o] + nbr[o] + 2.0 * color.y - ntl.y - nbr.y;
            float diff2 = abs(ntr[o] - nbl[o]) + abs(ntr.y - color.y) + abs(nbl.y - color.y);
            float guess2 = ntr[o] + nbl[o] + 2.0 * color.y - ntr.y - nbl.y;
            float fill;
            if (diff1 > diff2)
                fill = guess2 * 0.5;
            else if (diff1 < diff2)
                fill = guess1 * 0.5;
            else
                fill = (guess1 + guess2) * 0.25;
            color = set_at(color, o, fill);
        }
    }
    write_output(col, row, color);
}

// RCD step 0: the mosaic over the processed maximum, never negative, in cfa and in the
// planes of both colours of its row, as darktable's tiles fill them.
[shader("compute")]
[numthreads(16, 16, 1)]
void rcd_populate(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row))
        return;
    float value = max(0.0, mosaic(col, row)) * params.demosaic.scale;
    int idx = plane(col, row);
    cfa[idx] = value;
    uint c0 = fc(row, 0);
    uint c1 = fc(row, 1);
    rgb0[idx] = (c0 == 0 || c1 == 0) ? value : 0.0;
    rgb1[idx] = (c0 == 1 || c1 == 1) ? value : 0.0;
    rgb2[idx] = (c0 == 2 || c1 == 2) ? value : 0.0;
}

// The squared high-pass filter along a line through idx, step apart.
float hpf(int idx, int step)
{
    return sqr((cfa[idx - 3 * step] - cfa[idx - step] - cfa[idx + step] + cfa[idx + 3 * step])
               - 3.0 * (cfa[idx - 2 * step] + cfa[idx + 2 * step]) + 6.0 * cfa[idx]);
}

// RCD step 1: vertical and horizontal discrimination, 0 within 4 of the edges, as in
// darktable's zeroed tile planes.
[shader("compute")]
[numthreads(16, 16, 1)]
void rcd_step_1(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row))
        return;
    int idx = plane(col, row);
    int s = int(params.demosaic.stride);
    if (row < 4 || col < 4 || row >= int(params.demosaic.height) - 4 || col >= int(params.demosaic.width) - 4)
    {
        vh_dir[idx] = 0.0;
        return;
    }
    float v_stat = max(EPSSQ, hpf(idx - s, s) + hpf(idx, s) + hpf(idx + s, s));
    float h_stat = max(EPSSQ, hpf(idx - 1, 1) + hpf(idx, 1) + hpf(idx + 1, 1));
    vh_dir[idx] = v_stat / (v_stat + h_stat);
}

// Whether the photosite is red or blue, at the columns darktable's loops visit.
bool red_or_blue(int col, int row)
{
    return ((col - int(fc(row, 0) & 1)) & 1) == 0;
}

// RCD step 2: the low-pass filter at red and blue photosites.
[shader("compute")]
[numthreads(16, 16, 1)]
void rcd_step_2(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row))
        return;
    if (row < 2 || col < 2 || row >= int(params.demosaic.height) - 2 || col >= int(params.demosaic.width) - 2 || !red_or_blue(col, row))
        return;
    int idx = plane(col, row);
    int s = int(params.demosaic.stride);
    pq_dir[idx / 2] = cfa[idx]
                    + 0.5 * (cfa[idx - s] + cfa[idx + s] + cfa[idx - 1] + cfa[idx + 1])
                    + 0.25 * (cfa[idx - s - 1] + cfa[idx - s + 1] + cfa[idx + s - 1] + cfa[idx + s + 1]);
}

// Whether the photosite is one RCD's steps 3, 4.1 and 4.2 compute: red or blue, 4 or
// more from the edges.
bool inner_red_or_blue(int col, int row)
{
    return row >= 4 && col >= 4 && row < int(params.demosaic.height) - 4 && col < int(params.demosaic.width) - 4
        && red_or_blue(col, row);
}

// The refined discrimination: the centre's, or its diagonal neighbours' mean when that
// is further from 0.5.
float discrimination(float central, float neighbourhood)
{
    return (abs(0.5 - central) < abs(0.5 - neighbourhood)) ? neighbourhood : central;
}

// RCD step 3: green at red and blue photosites.
[shader("compute")]
[numthreads(16, 16, 1)]
void rcd_step_3(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row) || !inner_red_or_blue(col, row))
        return;
    int idx = plane(col, row);
    int s = int(params.demosaic.stride);
    int lp = idx / 2;
    float cfai = cfa[idx];

    float n_grad = EPS + abs(cfa[idx - s] - cfa[idx + s]) + abs(cfai - cfa[idx - 2 * s]) + abs(cfa[idx - s] - cfa[idx - 3 * s]) + abs(cfa[idx - 2 * s] - cfa[idx - 4 * s]);
    float s_grad = EPS + abs(cfa[idx + s] - cfa[idx - s]) + abs(cfai - cfa[idx + 2 * s]) + abs(cfa[idx + s] - cfa[idx + 3 * s]) + abs(cfa[idx + 2 * s] - cfa[idx + 4 * s]);
    float w_grad = EPS + abs(cfa[idx - 1] - cfa[idx + 1]) + abs(cfai - cfa[idx - 2]) + abs(cfa[idx - 1] - cfa[idx - 3]) + abs(cfa[idx - 2] - cfa[idx - 4]);
    float e_grad = EPS + abs(cfa[idx + 1] - cfa[idx - 1]) + abs(cfai - cfa[idx + 2]) + abs(cfa[idx + 1] - cfa[idx + 3]) + abs(cfa[idx + 2] - cfa[idx + 4]);

    float lpfi = pq_dir[lp];
    float n_est = cfa[idx - s] * (lpfi + lpfi) / (EPS + lpfi + pq_dir[lp - s]);
    float s_est = cfa[idx + s] * (lpfi + lpfi) / (EPS + lpfi + pq_dir[lp + s]);
    float w_est = cfa[idx - 1] * (lpfi + lpfi) / (EPS + lpfi + pq_dir[lp - 1]);
    float e_est = cfa[idx + 1] * (lpfi + lpfi) / (EPS + lpfi + pq_dir[lp + 1]);

    float v_est = (s_grad * n_est + n_grad * s_est) / (n_grad + s_grad);
    float h_est = (w_grad * e_est + e_grad * w_est) / (e_grad + w_grad);

    float neighbourhood = 0.25 * (vh_dir[idx - s - 1] + vh_dir[idx - s + 1] + vh_dir[idx + s - 1] + vh_dir[idx + s + 1]);
    float disc = discrimination(vh_dir[idx], neighbourhood);
    rgb1[idx] = interpolate(clamp(disc, 0.0, 1.0), h_est, v_est);
}

// RCD step 4.0: squared high-pass filters along the diagonals, at odd columns.
[shader("compute")]
[numthreads(16, 16, 1)]
void rcd_step_4_0(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row))
        return;
    if (row < 3 || col < 3 || row >= int(params.demosaic.height) - 3 || col >= int(params.demosaic.width) - 3 || (col & 1) == 0)
        return;
    int idx = plane(col, row);
    int s = int(params.demosaic.stride);
    p_diff[idx / 2] = hpf(idx, s + 1);
    q_diff[idx / 2] = hpf(idx, s - 1);
}

// RCD step 4.1: diagonal discrimination at red and blue photosites.
[shader("compute")]
[numthreads(16, 16, 1)]
void rcd_step_4_1(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row) || !inner_red_or_blue(col, row))
        return;
    int idx = plane(col, row);
    int s = int(params.demosaic.stride);
    int i2 = idx / 2;
    int i3 = (idx - s - 1) / 2;
    int i4 = (idx + s - 1) / 2;
    float p_stat = max(EPSSQ, p_diff[i3] + p_diff[i2] + p_diff[i4 + 1]);
    float q_stat = max(EPSSQ, q_diff[i3 + 1] + q_diff[i2] + q_diff[i4]);
    pq_dir[i2] = p_stat / (p_stat + q_stat);
}

float rgb_of(uint c, int idx)
{
    return c == 0 ? rgb0[idx] : (c == 1 ? rgb1[idx] : rgb2[idx]);
}

// RCD step 4.2: blue at red photosites and red at blue ones.
[shader("compute")]
[numthreads(16, 16, 1)]
void rcd_step_4_2(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row) || !inner_red_or_blue(col, row))
        return;
    int idx = plane(col, row);
    int s = int(params.demosaic.stride);
    uint c = 2 - fc(row, col);
    int pq = idx / 2;
    int pq2 = (idx - s - 1) / 2;
    int pq3 = (idx + s - 1) / 2;
    float neighbourhood = 0.25 * (pq_dir[pq2] + pq_dir[pq2 + 1] + pq_dir[pq3] + pq_dir[pq3 + 1]);
    float disc = discrimination(pq_dir[pq], neighbourhood);

    float g = rgb1[idx];
    float nw_grad = EPS + abs(rgb_of(c, idx - s - 1) - rgb_of(c, idx + s + 1)) + abs(rgb_of(c, idx - s - 1) - rgb_of(c, idx - 3 * s - 3)) + abs(g - rgb1[idx - 2 * s - 2]);
    float ne_grad = EPS + abs(rgb_of(c, idx - s + 1) - rgb_of(c, idx + s - 1)) + abs(rgb_of(c, idx - s + 1) - rgb_of(c, idx - 3 * s + 3)) + abs(g - rgb1[idx - 2 * s + 2]);
    float sw_grad = EPS + abs(rgb_of(c, idx - s + 1) - rgb_of(c, idx + s - 1)) + abs(rgb_of(c, idx + s - 1) - rgb_of(c, idx + 3 * s - 3)) + abs(g - rgb1[idx + 2 * s - 2]);
    float se_grad = EPS + abs(rgb_of(c, idx - s - 1) - rgb_of(c, idx + s + 1)) + abs(rgb_of(c, idx + s + 1) - rgb_of(c, idx + 3 * s + 3)) + abs(g - rgb1[idx + 2 * s + 2]);

    float nw_est = rgb_of(c, idx - s - 1) - rgb1[idx - s - 1];
    float ne_est = rgb_of(c, idx - s + 1) - rgb1[idx - s + 1];
    float sw_est = rgb_of(c, idx + s - 1) - rgb1[idx + s - 1];
    float se_est = rgb_of(c, idx + s + 1) - rgb1[idx + s + 1];

    float p_est = (nw_grad * se_est + se_grad * nw_est) / (nw_grad + se_grad);
    float q_est = (ne_grad * sw_est + sw_grad * ne_est) / (ne_grad + sw_grad);
    float value = g + interpolate(clamp(disc, 0.0, 1.0), q_est, p_est);
    if (c == 0)
        rgb0[idx] = value;
    else
        rgb2[idx] = value;
}

// RCD step 4.3: red and blue at green photosites.
[shader("compute")]
[numthreads(16, 16, 1)]
void rcd_step_4_3(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row))
        return;
    if (row < 4 || col < 4 || row >= int(params.demosaic.height) - 4 || col >= int(params.demosaic.width) - 4)
        return;
    // Green: the columns of the row's second photosite's parity.
    if (((col - int(fc(row, 1) & 1)) & 1) != 0)
        return;
    int idx = plane(col, row);
    int s = int(params.demosaic.stride);

    float neighbourhood = 0.25 * (vh_dir[idx - s - 1] + vh_dir[idx - s + 1] + vh_dir[idx + s - 1] + vh_dir[idx + s + 1]);
    float disc = discrimination(vh_dir[idx], neighbourhood);
    float g = rgb1[idx];
    float n1 = EPS + abs(g - rgb1[idx - 2 * s]);
    float s1 = EPS + abs(g - rgb1[idx + 2 * s]);
    float w1 = EPS + abs(g - rgb1[idx - 2]);
    float e1 = EPS + abs(g - rgb1[idx + 2]);
    float g_mw = rgb1[idx - s];
    float g_pw = rgb1[idx + s];
    float g_m1 = rgb1[idx - 1];
    float g_p1 = rgb1[idx + 1];

    for (uint c = 0; c <= 2; c += 2)
    {
        float sn_abs = abs(rgb_of(c, idx - s) - rgb_of(c, idx + s));
        float ew_abs = abs(rgb_of(c, idx - 1) - rgb_of(c, idx + 1));
        float n_grad = n1 + sn_abs + abs(rgb_of(c, idx - s) - rgb_of(c, idx - 3 * s));
        float s_grad = s1 + sn_abs + abs(rgb_of(c, idx + s) - rgb_of(c, idx + 3 * s));
        float w_grad = w1 + ew_abs + abs(rgb_of(c, idx - 1) - rgb_of(c, idx - 3));
        float e_grad = e1 + ew_abs + abs(rgb_of(c, idx + 1) - rgb_of(c, idx + 3));

        float n_est = rgb_of(c, idx - s) - g_mw;
        float s_est = rgb_of(c, idx + s) - g_pw;
        float w_est = rgb_of(c, idx - 1) - g_m1;
        float e_est = rgb_of(c, idx + 1) - g_p1;

        float v_est = (n_grad * s_est + s_grad * n_est) / (n_grad + s_grad);
        float h_est = (e_grad * w_est + w_grad * e_est) / (e_grad + w_grad);
        float value = g + interpolate(clamp(disc, 0.0, 1.0), h_est, v_est);
        if (c == 0)
            rgb0[idx] = value;
        else
            rgb2[idx] = value;
    }
}

// RCD's result, scaled back, everywhere 9 or more photosites from the edges.
[shader("compute")]
[numthreads(16, 16, 1)]
void rcd_write(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row))
        return;
    if (row < RCD_MARGIN || col < RCD_MARGIN || row >= int(params.demosaic.height) - RCD_MARGIN || col >= int(params.demosaic.width) - RCD_MARGIN)
        return;
    int idx = plane(col, row);
    write_output(col, row, params.demosaic.unscale * max(load_rgb(idx), 0.0));
}

// X-Trans: Markesteijn 1-pass, ported from darktable's `xtrans_markesteijn_interpolate`
// (Frank Markesteijn's algorithm, by way of dcraw), per photosite as darktable's
// OpenCL kernels restate it, with the CPU's semantics where they differ: a red or
// blue photosite takes its green bounds from the first photosite of its pair, the left
// or upper one, as the CPU's walk leaves them. Everything within 12 photosites of the
// edges is darktable's linear VNG instead, as `_vng_lininterpolate` overwrites it.

// The four directions' red, green and blue, then per photosite state shared between
// passes: the solitary green's best direction so far, YPbPr, then the homogeneity
// counts, one direction per component.
[[vk::binding(18, 0)]] RWStructuredBuffer<float4> xrgb0;
[[vk::binding(19, 0)]] RWStructuredBuffer<float4> xrgb1;
[[vk::binding(20, 0)]] RWStructuredBuffer<float4> xrgb2;
[[vk::binding(21, 0)]] RWStructuredBuffer<float4> xrgb3;
[[vk::binding(22, 0)]] RWStructuredBuffer<float4> aux;
// Two values per photosite for the green bounds, then a plane per direction for the
// derivatives, then for the 5×5 sums of homogeneity.
[[vk::binding(23, 0)]] RWStructuredBuffer<float> work;

// darktable FCNxtrans(): the filter colour of a photosite of an X-Trans mosaic.
uint fcx(int row, int col)
{
    return color_at(uint((row % 6) * 6 + col % 6));
}

// darktable allhex: the hexagon's offset c around the photosite at (col, row).
int2 hexagon(int col, int row, uint c)
{
    uint e = uint(((row % 3) * 3 + col % 3) * 8) + c;
    int4 v = params.demosaic.hexes[e / 2];
    return (e & 1) == 0 ? v.xy : v.zw;
}

int index(int col, int row)
{
    return row * int(params.demosaic.width) + col;
}

int index(int2 p)
{
    return index(p.x, p.y);
}

bool within(int col, int row, int border)
{
    return col >= border && row >= border && col < int(params.demosaic.width) - border
        && row < int(params.demosaic.height) - border;
}

float4 xrgb(uint d, int idx)
{
    switch (d)
    {
    case 0: return xrgb0[idx];
    case 1: return xrgb1[idx];
    case 2: return xrgb2[idx];
    default: return xrgb3[idx];
    }
}

void set_xrgb(uint d, int idx, float4 value)
{
    switch (d)
    {
    case 0: xrgb0[idx] = value; break;
    case 1: xrgb1[idx] = value; break;
    case 2: xrgb2[idx] = value; break;
    default: xrgb3[idx] = value; break;
    }
}

// Whether a row is one of the solitary green's rows, or a column one of its columns.
bool solitary_row(int row)
{
    return (row - int(params.demosaic.solitary_y)) % 3 == 0;
}

bool solitary_col(int col)
{
    return (col - int(params.demosaic.solitary_x)) % 3 == 0;
}

// darktable `_vng_lininterpolate` with a border of 12: the outermost photosites from
// the mean of each colour in their 3×3 neighbourhood, the rest of the border from
// their neighbours of each colour, weighted 2 along the axes and 1 on the diagonals.
[shader("compute")]
[numthreads(16, 16, 1)]
void xt_border(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row) || within(col, row, 12))
        return;
    int width = int(params.demosaic.width);
    int height = int(params.demosaic.height);
    uint f = fcx(row, col);
    float3 sum = float3(0.0);
    float3 count = float3(0.0);
    float3 color;
    if (col == 0 || row == 0 || col == width - 1 || row == height - 1)
    {
        for (int y = row - 1; y != row + 2; y++)
            for (int x = col - 1; x != col + 2; x++)
                if (y >= 0 && x >= 0 && y < height && x < width)
                {
                    uint c = fcx(y, x);
                    sum = add_at(sum, c, max(0.0, mosaic(x, y)));
                    count = add_at(count, c, 1.0);
                }
        float own = max(0.0, mosaic(col, row));
        color = float3(f != 0 && count.x != 0.0 ? sum.x / count.x : own,
                       f != 1 && count.y != 0.0 ? sum.y / count.y : own,
                       f != 2 && count.z != 0.0 ? sum.z / count.z : own);
    }
    else
    {
        for (int y = -1; y <= 1; y++)
            for (int x = -1; x <= 1; x++)
            {
                uint c = fcx(row + y, col + x);
                if (c == f)
                    continue;
                float weight = float(1 << ((y == 0 ? 1 : 0) + (x == 0 ? 1 : 0)));
                sum = add_at(sum, c, max(0.0, mosaic(col + x, row + y)) * weight);
                count = add_at(count, c, weight);
            }
        float own = mosaic(col, row);
        color = float3(f == 0 ? own : sum.x / count.x, f == 1 ? own : sum.y / count.y,
                       f == 2 ? own : sum.z / count.z);
    }
    write_output(col, row, color);
}

// The mosaic in each direction's buffer, each photosite in its own colour.
[shader("compute")]
[numthreads(16, 16, 1)]
void xt_initial(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row))
        return;
    uint f = fcx(row, col);
    float value = max(0.0, mosaic(col, row));
    float4 pixel = float4(f == 0 ? value : 0.0, f == 1 ? value : 0.0, f == 2 ? value : 0.0, 0.0);
    int idx = index(col, row);
    xrgb0[idx] = pixel;
    xrgb1[idx] = pixel;
    xrgb2[idx] = pixel;
    xrgb3[idx] = pixel;
}

// xt_green's estimate of green along the hexagon's offset h, for a photosite of colour f
// and value own.
float hexagon_green(int2 p, int2 h, float own, uint f)
{
    return 0.640625 * xrgb0[index(p + h)].y + 0.359375 * xrgb0[index(p - 2 * h)].y
         + 0.12890625 * (2.0 * own - xrgb0[index(p + 3 * h)][f] - xrgb0[index(p - 3 * h)][f]);
}

// The smallest and largest green of a photosite's hexagon.
float2 green_bounds(int col, int row)
{
    float lo = 3.402823466e+38;
    float hi = 0.0;
    for (uint c = 0; c < 6; c++)
    {
        float green = xrgb0[index(int2(col, row) + hexagon(col, row, c))].y;
        lo = min(lo, green);
        hi = max(hi, green);
    }
    return float2(lo, hi);
}

// The bounds of green for each red and blue photosite: its pair's first photosite's
// hexagon, its own when it is the first, as darktable's CPU walk leaves them.
[shader("compute")]
[numthreads(16, 16, 1)]
void xt_green_bounds(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row) || !within(col, row, 3) || fcx(row, col) == 1)
        return;
    float2 bounds = green_bounds(col, row);
    // The partner to the left or above makes this the pair's second photosite; a
    // first photosite whose greens are all 0 leaves the second to its own.
    int2 partner = fcx(row, col - 1) != 1 ? int2(col - 1, row)
                 : (fcx(row - 1, col) != 1 ? int2(col, row - 1) : int2(-1, -1));
    if (partner.x >= 0)
    {
        float2 first = green_bounds(partner.x, partner.y);
        if (first.y != 0.0)
            bounds = first;
    }
    int idx = index(col, row);
    work[2 * idx] = bounds.x;
    work[2 * idx + 1] = bounds.y;
}

// darktable CLAMPS(a, lo, hi).
float clamps(float a, float lo, float hi)
{
    return a > lo ? (a < hi ? a : hi) : lo;
}

// Green at red and blue photosites, horizontally, vertically and along both diagonals.
[shader("compute")]
[numthreads(16, 16, 1)]
void xt_green(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row) || !within(col, row, 3))
        return;
    uint f = fcx(row, col);
    if (f == 1)
        return;
    int2 p = int2(col, row);
    int idx = index(p);
    float lo = work[2 * idx];
    float hi = work[2 * idx + 1];
    int2 h0 = hexagon(col, row, 0);
    int2 h1 = hexagon(col, row, 1);
    int2 h2 = hexagon(col, row, 2);
    int2 h3 = hexagon(col, row, 3);
    float own = xrgb0[idx][f];
    float4 color;
    color.x = 0.6796875 * (xrgb0[index(p + h1)].y + xrgb0[index(p + h0)].y)
            - 0.1796875 * (xrgb0[index(p + 2 * h1)].y + xrgb0[index(p + 2 * h0)].y);
    color.y = 0.87109375 * xrgb0[index(p + h3)].y + xrgb0[index(p + h2)].y * 0.13
            + 0.359375 * (own - xrgb0[index(p - h2)][f]);
    color.z = hexagon_green(p, hexagon(col, row, 4), own, f);
    color.w = hexagon_green(p, hexagon(col, row, 5), own, f);
    uint flip = solitary_row(row) ? 1 : 0;
    for (uint c = 0; c < 4; c++)
    {
        uint d = c ^ flip;
        float4 pixel = xrgb(d, idx);
        pixel.y = clamps(color[c], lo, hi);
        set_xrgb(d, idx, pixel);
    }
}

// Red and blue at solitary greens, step `step` of six: horizontally then vertically
// into the first two directions, then the better of horizontal and vertical by their
// steepness, twice, into the other two.
[shader("compute")]
[numthreads(16, 16, 1)]
void xt_solitary(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row) || !within(col, row, 6) || !solitary_row(row) || !solitary_col(col))
        return;
    uint d = params.demosaic.step;
    uint target = d < 2 ? d : (d < 4 ? 2 : 3);
    int2 line = (d & 1) == 0 ? int2(1, 0) : int2(0, 1);
    uint h = fcx(row, col + 1) ^ ((d & 1) * 2);
    int2 p = int2(col, row);
    int idx = index(p);
    float4 centre = xrgb(target, idx);
    float2 color = float2(0.0);
    float diff = 0.0;
    for (uint c = 0; c < 2; c++, h ^= 2)
    {
        int2 off = line << c;
        float4 ahead = xrgb(target, index(p + off));
        float4 behind = xrgb(target, index(p - off));
        float g = 2.0 * centre.y - ahead.y - behind.y;
        color = set_at(color, h != 0 ? 1 : 0, g + ahead[h] + behind[h]);
        if (d > 1)
            diff += sqr(ahead.y - behind.y - ahead[h] + behind[h]) + sqr(g);
    }
    if (d == 2 || d == 4)
    {
        aux[idx] = float4(color, diff, 0.0);
        return;
    }
    if (d > 1)
    {
        // darktable keeps the previous step's colours when its steepness is lower.
        float4 previous = aux[idx];
        if (previous.z < diff)
            color = previous.xy;
    }
    centre.x = color.x / 2.0;
    centre.z = color.y / 2.0;
    set_xrgb(target, idx, centre);
}

// Red at blue photosites and blue at red ones, in direction `step`.
[shader("compute")]
[numthreads(16, 16, 1)]
void xt_red_blue(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row) || !within(col, row, 6))
        return;
    uint f = 2 - fcx(row, col);
    if (f == 1)
        return;
    uint d = params.demosaic.step;
    int2 p = int2(col, row);
    int idx = index(p);
    bool horizontal = solitary_row(row);
    int2 c = horizontal ? int2(1, 0) : int2(0, 1);
    int2 h = horizontal ? int2(0, 3) : int2(3, 0);
    float4 centre = xrgb(d, idx);
    float g = centre.y;
    bool along = d > 1 || (d == 0 && horizontal) || (d == 1 && !horizontal)
        || (abs(g - xrgb(d, index(p + c)).y) + abs(g - xrgb(d, index(p - c)).y)
            < 2.0 * (abs(g - xrgb(d, index(p + h)).y) + abs(g - xrgb(d, index(p - h)).y)));
    int2 i = along ? c : h;
    float4 ahead = xrgb(d, index(p + i));
    float4 behind = xrgb(d, index(p - i));
    centre = set_at(centre, f, (ahead[f] + behind[f] + 2.0 * g - ahead.y - behind.y) / 2.0);
    set_xrgb(d, idx, centre);
}

// Red and blue for the 2×2 blocks of green, in the direction buffer `step` / 2 from the
// hexagon's offsets `step` and `step` + 1.
[shader("compute")]
[numthreads(16, 16, 1)]
void xt_two_by_two(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row) || !within(col, row, 8) || solitary_row(row) || solitary_col(col))
        return;
    uint d = params.demosaic.step;
    uint n = d / 2;
    int2 p = int2(col, row);
    int idx = index(p);
    int2 a = hexagon(col, row, d);
    int2 b = hexagon(col, row, d + 1);
    float4 centre = xrgb(n, idx);
    float4 pa = xrgb(n, index(p + a));
    float4 pb = xrgb(n, index(p + b));
    if (any(a + b != int2(0)))
    {
        float g = 3.0 * centre.y - 2.0 * pa.y - pb.y;
        centre.x = (g + 2.0 * pa.x + pb.x) / 3.0;
        centre.z = (g + 2.0 * pa.z + pb.z) / 3.0;
    }
    else
    {
        float g = 2.0 * centre.y - pa.y - pb.y;
        centre.x = (g + pa.x + pb.x) / 2.0;
        centre.z = (g + pa.z + pb.z) / 2.0;
    }
    set_xrgb(n, idx, centre);
}

// Direction `step` in ITU-R BT.2020 YPbPr.
[shader("compute")]
[numthreads(16, 16, 1)]
void xt_yuv(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row) || !within(col, row, 8))
        return;
    int idx = index(col, row);
    float4 rgb = xrgb(params.demosaic.step, idx);
    float y = 0.2627 * rgb.x + 0.6780 * rgb.y + 0.0593 * rgb.z;
    aux[idx] = float4(y, (rgb.z - y) * 0.56433, (rgb.x - y) * 0.67815, 0.0);
}

// The second derivative of direction `step`'s YPbPr along that direction.
[shader("compute")]
[numthreads(16, 16, 1)]
void xt_derivative(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row) || !within(col, row, 9))
        return;
    uint d = params.demosaic.step;
    int2 dir = d == 0 ? int2(1, 0) : (d == 1 ? int2(0, 1) : (d == 2 ? int2(1, 1) : int2(-1, 1)));
    int2 p = int2(col, row);
    int idx = index(p);
    float3 centre = aux[idx].xyz;
    float3 ahead = aux[index(p + dir)].xyz;
    float3 behind = aux[index(p - dir)].xyz;
    float3 second = 2.0 * centre - ahead - behind;
    int plane = int(params.demosaic.width * params.demosaic.height);
    work[int(d) * plane + idx] = dot(second, second);
}

// How many of direction d's 3×3 derivatives around (col, row) are within threshold.
float homogeneity(int d, int col, int row, int plane, float threshold)
{
    float count = 0.0;
    for (int v = -1; v <= 1; v++)
        for (int h = -1; h <= 1; h++)
            count += work[d * plane + index(col + h, row + v)] <= threshold ? 1.0 : 0.0;
    return count;
}

// Homogeneity: per direction, how many of the 3×3 derivatives are within 8 times the
// smallest derivative at the centre.
[shader("compute")]
[numthreads(16, 16, 1)]
void xt_homogeneity(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row) || !within(col, row, 10))
        return;
    int idx = index(col, row);
    int plane = int(params.demosaic.width * params.demosaic.height);
    float threshold = 3.402823466e+38;
    for (int d = 0; d < 4; d++)
        threshold = min(threshold, work[d * plane + idx]);
    threshold *= 8.0;
    aux[idx] = float4(homogeneity(0, col, row, plane, threshold), homogeneity(1, col, row, plane, threshold),
                      homogeneity(2, col, row, plane, threshold), homogeneity(3, col, row, plane, threshold));
}

// The 5×5 sums of homogeneity per direction.
[shader("compute")]
[numthreads(16, 16, 1)]
void xt_homogeneity_sum(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row) || !within(col, row, 12))
        return;
    float4 sum = float4(0.0);
    for (int v = -2; v <= 2; v++)
        for (int h = -2; h <= 2; h++)
            sum += aux[index(col + h, row + v)];
    int idx = index(col, row);
    int plane = int(params.demosaic.width * params.demosaic.height);
    for (int d = 0; d < 4; d++)
        work[d * plane + idx] = sum[d];
}

// The mean of the most homogeneous directions, 12 or more photosites from the edges.
[shader("compute")]
[numthreads(16, 16, 1)]
void xt_final(uint3 id: SV_DispatchThreadID)
{
    int col, row;
    if (!photosite(id, col, row) || !within(col, row, 12))
        return;
    int idx = index(col, row);
    int plane = int(params.demosaic.width * params.demosaic.height);
    uint4 hm = uint4(uint(work[idx]), uint(work[plane + idx]), uint(work[2 * plane + idx]),
                     uint(work[3 * plane + idx]));
    uint best = max(max(hm.x, hm.y), max(hm.z, hm.w));
    best -= best >> 3;
    float4 sum = float4(0.0);
    for (uint d = 0; d < 4; d++)
        if (hm[d] >= best)
            sum += float4(xrgb(d, idx).xyz, 1.0);
    write_output(col, row, sum.xyz / sum.w);
}
