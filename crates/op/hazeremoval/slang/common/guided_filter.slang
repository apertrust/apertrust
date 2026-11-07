// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2017-2026  darktable developers (ported from src/common/guided_filter.c)
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

// darktable's guided filter with a colour guide (src/common/guided_filter.c,
// _guided_filter_tiling()), the part that is a pure function: the linear coefficients
// a and b at a pixel from the box means of the input p, the guide I and their
// products. Haze removal is its only user in darktable, and here (decision 15:
// helpers stay with their port until a second one needs them). The box means
// themselves read buffers, so they stay in the kernel.
module guided_filter;

// FLT_EPSILON: below 4 of these, darktable takes the 3×3 system as singular.
static const float FLT_EPSILON = 1.1920928955078125e-07;

// The thirteen box means the coefficients need, grouped four to a vector as the
// kernel's passes take them.
public struct GuideMeans
{
    // p, I_r, I_g, I_b.
    public float4 first;
    // I_r p, I_g p, I_b p, I_r I_r.
    public float4 second;
    // I_r I_g, I_r I_b, I_g I_g, I_g I_b.
    public float4 third;
    // I_b I_b.
    public float blue_blue;
};

// darktable's per-pixel solve: the covariance of the guide, regularised by eps on its
// diagonal, solved against the guide's covariance with p by Cramer's rule, in
// darktable's order of operations. Returns (a_r, a_g, a_b, b).
public float4 guided_coefficients(GuideMeans m, float eps)
{
    float inp_mean = m.first.x;
    float guide_r = m.first.y;
    float guide_g = m.first.z;
    float guide_b = m.first.w;
    float sigma_0_0 = m.second.w - (guide_r * guide_r) + eps;
    float sigma_0_1 = m.third.x - (guide_r * guide_g);
    float sigma_0_2 = m.third.y - (guide_r * guide_b);
    float sigma_1_1 = m.third.z - (guide_g * guide_g) + eps;
    float sigma_1_2 = m.third.w - (guide_g * guide_b);
    float sigma_2_2 = m.blue_blue - (guide_b * guide_b) + eps;
    float det0 = sigma_0_0 * (sigma_1_1 * sigma_2_2 - sigma_1_2 * sigma_1_2)
        - sigma_0_1 * (sigma_0_1 * sigma_2_2 - sigma_0_2 * sigma_1_2)
        + sigma_0_2 * (sigma_0_1 * sigma_1_2 - sigma_0_2 * sigma_1_1);
    if (abs(det0) > 4.0 * FLT_EPSILON)
    {
        float cov_r = m.second.x - guide_r * inp_mean;
        float cov_g = m.second.y - guide_g * inp_mean;
        float cov_b = m.second.z - guide_b * inp_mean;
        float det1 = cov_r * (sigma_1_1 * sigma_2_2 - sigma_1_2 * sigma_1_2)
            - sigma_0_1 * (cov_g * sigma_2_2 - cov_b * sigma_1_2)
            + sigma_0_2 * (cov_g * sigma_1_2 - cov_b * sigma_1_1);
        float det2 = sigma_0_0 * (cov_g * sigma_2_2 - cov_b * sigma_1_2)
            - cov_r * (sigma_0_1 * sigma_2_2 - sigma_0_2 * sigma_1_2)
            + sigma_0_2 * (sigma_0_1 * cov_b - sigma_0_2 * cov_g);
        float det3 = sigma_0_0 * (sigma_1_1 * cov_b - sigma_1_2 * cov_g)
            - sigma_0_1 * (sigma_0_1 * cov_b - sigma_0_2 * cov_g)
            + cov_r * (sigma_0_1 * sigma_1_2 - sigma_0_2 * sigma_1_1);
        float a_r = det1 / det0;
        float a_g = det2 / det0;
        float a_b = det3 / det0;
        float b = inp_mean - a_r * guide_r - a_g * guide_g - a_b * guide_b;
        return float4(a_r, a_g, a_b, b);
    }
    // The linear system is singular.
    return float4(0.0, 0.0, 0.0, inp_mean);
}
