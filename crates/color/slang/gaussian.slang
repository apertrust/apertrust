// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2012-2026  darktable developers (ported from src/common/gaussian.c)
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

// darktable's recursive Gaussian (src/common/gaussian.c, dt_gaussian_blur and
// dt_gaussian_blur_4c of order 0): Deriche's recursion down each column, the causal sums
// written, then the anticausal ones added, and the same along each row of the result,
// every value clamped as it is read. A kernel walks one line per invocation, as
// darktable's loops do, so the sums are darktable's, in darktable's order; the
// coefficients come from its uniforms, computed on the CPU by
// apertrust_color::gaussian::Deriche, whose module also restates the blur on the CPU.
// Shared by the colour equalizer and, once its copy in its crate moves here, the tone
// equalizer's EIGF mask (decision 65).

module gaussian;

// darktable dt_gaussian_mean_blur()'s bounds: an unbounded signal, clamped to ±10⁹.
public static const float MEAN_BLUR_RANGE = 1.0e9;

// _compute_gauss_params()'s coefficients for order 0.
public struct Deriche
{
    public float a0;
    public float a1;
    public float a2;
    public float a3;
    public float b1;
    public float b2;
    public float coefp;
    public float coefn;
};

// darktable CLAMPF(a, mn, mx), which takes mn when a is below it even when mn > mx.
public float clampf(float a, float low, float high)
{
    return a < low ? low : min(a, high);
}

public float2 clampf(float2 a, float2 low, float2 high)
{
    return float2(clampf(a.x, low.x, high.x), clampf(a.y, low.y, high.y));
}

// One step of the causal recursion: y = a0 x + a1 x[-1] - b1 y[-1] - b2 y[-2].
public float causal(Deriche d, float xc, float xp, float yp, float yb)
{
    return (d.a0 * xc) + (d.a1 * xp) - (d.b1 * yp) - (d.b2 * yb);
}

public float2 causal(Deriche d, float2 xc, float2 xp, float2 yp, float2 yb)
{
    return (d.a0 * xc) + (d.a1 * xp) - (d.b1 * yp) - (d.b2 * yb);
}

// One step of the anticausal recursion: y = a2 x[+1] + a3 x[+2] - b1 y[+1] - b2 y[+2].
public float anticausal(Deriche d, float xn, float xa, float yn, float ya)
{
    return (d.a2 * xn) + (d.a3 * xa) - (d.b1 * yn) - (d.b2 * ya);
}

public float2 anticausal(Deriche d, float2 xn, float2 xa, float2 yn, float2 ya)
{
    return (d.a2 * xn) + (d.a3 * xa) - (d.b1 * yn) - (d.b2 * ya);
}
