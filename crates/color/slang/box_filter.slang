// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2024  darktable developers (ported from src/common/box_filters.cc)
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

// darktable's box mean (src/common/box_filters.cc, dt_box_mean), shared by the kernels
// that port it: bloom's lights, one channel, and soften's image, three. A box mean of
// radius r replaces each value of a line by the mean of the values within r of it that
// lie inside the line; dt_box_mean runs it along every row, then every column,
// `iterations` times (BOX_ITERATIONS, 8, for bloom and soften). A pass walks one line per
// invocation, as darktable's loops do, so its running sum is darktable's, in darktable's
// order. The kernel says where a line's values are read and its results written
// (IBoxLine): from one buffer into another, so a pass never reads what it has written,
// where darktable keeps a copy of the values leaving the window. The Rust restatement is
// `apertrust_color::box_filter`. See docs/ports/bloom.md.

module box_filter;

// A line of an image: its value at position i, and where the result at i goes.
public interface IBoxLine<T>
{
    public T read(uint i);
    public void write(uint i, T value);
}

// darktable _blur_horizontal and _blur_vertical without Kahan summation, on a line of
// `count` values: the left half of the window added, then at each position the value
// entering added, in the bulk after the value leaving is subtracted, and the sum over the
// values in the window, `hits`, stored. The window is clipped to the line, so a constant
// stays constant up to its ends; a radius of 0 copies the line.
public void box_mean_line<T : IArithmetic, L : IBoxLine<T>>(L line, uint count, uint radius)
{
    T sum = T(0);
    uint hits = 0;
    for (uint x = 0; x < min(radius, count); x++)
    {
        hits++;
        sum = sum + line.read(x);
    }
    uint x = 0;
    // Up to where values start leaving the window.
    for (; x <= radius && x + radius < count; x++)
    {
        hits++;
        sum = sum + line.read(x + radius);
        line.write(x, sum / T(int(hits)));
    }
    // A window wider than half the line: nothing enters or leaves.
    for (; x <= radius && x < count; x++)
        line.write(x, sum / T(int(hits)));
    // The bulk of the line.
    for (; x + radius < count; x++)
    {
        sum = sum - line.read(x - radius - 1);
        sum = sum + line.read(x + radius);
        line.write(x, sum / T(int(hits)));
    }
    // The right end, where values only leave.
    for (; x < count; x++)
    {
        hits--;
        sum = sum - line.read(x - radius - 1);
        line.write(x, sum / T(int(hits)));
    }
}
