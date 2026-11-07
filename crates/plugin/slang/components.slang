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

// Writes to one component of a vector, chosen at run time, for every kernel.
//
// FXC, the Direct3D 12 shader compiler the engine pins, cannot write a function-local
// vector component at an index known only at run time (`v[c] += x`, `v[c] = x`): it
// tries to unroll the loop around the write, naga's loop bounding stops the unrolling,
// and the kernel fails to compile on Direct3D 12 alone, WARP included, as CI showed
// (decision 10).
// A local array's element is refused too until tested on Windows. These helpers never
// index: they compare c with every component's index at once and select, so component c
// alone changes and every other keeps its value exactly, whatever it holds, infinities
// and NaN included; a c past the last component changes nothing. The engine's naga test
// finds any write at a run-time index that remains.

module components;

// v with x added to component c.
public float2 add_at(float2 v, uint c, float x)
{
    return select(uint2(0, 1) == c, v + x, v);
}

public float3 add_at(float3 v, uint c, float x)
{
    return select(uint3(0, 1, 2) == c, v + x, v);
}

public float4 add_at(float4 v, uint c, float x)
{
    return select(uint4(0, 1, 2, 3) == c, v + x, v);
}

public uint4 add_at(uint4 v, uint c, uint x)
{
    return select(uint4(0, 1, 2, 3) == c, v + x, v);
}

// v with component c set to x.
public float2 set_at(float2 v, uint c, float x)
{
    return select(uint2(0, 1) == c, float2(x), v);
}

public float3 set_at(float3 v, uint c, float x)
{
    return select(uint3(0, 1, 2) == c, float3(x), v);
}

public float4 set_at(float4 v, uint c, float x)
{
    return select(uint4(0, 1, 2, 3) == c, float4(x), v);
}

public uint4 set_at(uint4 v, uint c, uint x)
{
    return select(uint4(0, 1, 2, 3) == c, uint4(x), v);
}
