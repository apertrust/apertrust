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


// The start of every pass's uniform block, the Slang twin of apertrust-plugin's
// PassHeader: which part of the image the pass covers, so maths that depends on a
// pixel's position is the same in a tile, a region or the whole image, and where the
// windows of its images that the engine binds start. A kernel's own parameters follow
// at offset 48.
module pass;

public struct PassHeader
{
    // The invocations this dispatch runs, one per pixel of the region the pass writes,
    // or, for a pass in row strips, the strip's rows.
    public uint width;
    public uint height;
    // Where the first invocation lies: after the raw stage, the corner of its
    // operation's output region in its stage's image at the pipe's scale, a tile's or a
    // region's (decision 16); for a pass in row strips (the raw stage's before
    // demosaic, and the engine's region copy), the strip's first row in the pass's
    // invocations; 0 in the passes of a viewport's full-resolution part, which run over
    // the whole full-resolution image (decision 15).
    public uint origin_x;
    public uint origin_y;
    // Its operation's stage's image at full resolution: the developed image, or after an
    // operation that moves pixels, that operation's output (decision 16).
    public uint full_width;
    public uint full_height;
    // Pixels at the pipe's scale per pixel at full resolution, at most 1; 1 in the
    // passes of a viewport's full-resolution part (decision 15).
    public float scale;
    // How many times main applies the per-pixel function: 1, more only in benchmarks.
    public uint repeat;
    // The 32-bit word of the image's whole buffer at which the window bound at binding
    // 0, the input, or 1, the output, starts: 0 when the whole buffer is bound. A kernel
    // that runs in strips indexes the whole image less it (decision 14).
    public uint input_base;
    public uint output_base;
    // Reserved, so a kernel's parameters start on a 16-byte step.
    public uint reserved0;
    public uint reserved1;
};
