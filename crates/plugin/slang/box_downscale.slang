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

// The viewport's box filter (decision 12), shared by the engine's downscale and an
// operation that downscales what it computed at full resolution to the viewport's size
// (decision 15), so both take the same footprints and sums: output pixel o is the mean
// of the source pixels o × source / output up to (o + 1) × source / output, in 32-bit
// integers, added row by row in the value's own precision.

module box_downscale;

// The source image, its value at (x, y) of the whole source.
public interface IDownscaleSource<T>
{
    public T read(uint x, uint y);
}

// The mean of output pixel o's footprint in a source `source_size` wide and high, down
// to `output_size`, which is never larger, so every footprint has a pixel.
public T footprint_mean<T : IArithmetic, S : IDownscaleSource<T>>(
    S source, uint2 o, uint2 source_size, uint2 output_size)
{
    uint2 start = o * source_size / output_size;
    uint2 end = (o + 1) * source_size / output_size;
    T sum = T(0);
    for (uint y = start.y; y < end.y; y++)
        for (uint x = start.x; x < end.x; x++)
            sum = sum + source.read(x, y);
    return sum / T(int((end.x - start.x) * (end.y - start.y)));
}
