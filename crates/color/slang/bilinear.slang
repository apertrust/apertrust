// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2019-2024  darktable developers (ported from src/common/fast_guided_filter.h)
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

// darktable's bilinear resampling between grids (src/common/fast_guided_filter.h,
// interpolate_bilinear): corner-aligned, with no half-pixel offset, the nodes clamped to
// the input, the weights from the clamped next node, in darktable's order of arithmetic.
// apertrust_color::bilinear restates it on the CPU. Shared by the colour equalizer and,
// once its copy in its crate moves here, the tone equalizer's EIGF mask (decision 65).

module bilinear;

// Where interpolate_bilinear() reads an image size_in across and down for the pixel at
// `at` of one size_out: the four nodes and their weights.
public struct Taps
{
    public uint x_prev;
    public uint x_next;
    public uint y_prev;
    public uint y_next;
    // Dx_next is x_prev's weight and Dx_prev x_next's, as darktable names them.
    public float dx_next;
    public float dx_prev;
    public float dy_next;
    public float dy_prev;
};

public Taps bilinear(uint2 at, uint2 size_in, uint2 size_out)
{
    float x_in = (float(at.x) / float(size_out.x)) * float(size_in.x);
    float y_in = (float(at.y) / float(size_out.y)) * float(size_in.y);
    uint x_prev = uint(floor(x_in));
    uint y_prev = uint(floor(y_in));
    Taps t;
    t.x_next = min(x_prev + 1, size_in.x - 1);
    t.y_next = min(y_prev + 1, size_in.y - 1);
    t.x_prev = min(x_prev, size_in.x - 1);
    t.y_prev = min(y_prev, size_in.y - 1);
    t.dx_next = float(t.x_next) - x_in;
    t.dx_prev = 1.0 - t.dx_next;
    t.dy_next = float(t.y_next) - y_in;
    t.dy_prev = 1.0 - t.dy_next;
    return t;
}

// interpolate_bilinear()'s sum for one channel from the values at the nodes: north is
// y_prev, west x_prev.
public float interpolate(Taps t, float nw, float ne, float sw, float se)
{
    return t.dy_prev * (sw * t.dx_next + se * t.dx_prev) + t.dy_next * (nw * t.dx_next + ne * t.dx_prev);
}

// The same for two channels at once, each as interpolate() computes it.
public float2 interpolate(Taps t, float2 nw, float2 ne, float2 sw, float2 se)
{
    return t.dy_prev * (sw * t.dx_next + se * t.dx_prev) + t.dy_next * (nw * t.dx_next + ne * t.dx_prev);
}
