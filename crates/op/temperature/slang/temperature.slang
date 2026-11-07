// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/iop/temperature.c)
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

// White balance (darktable's temperature), ported from darktable src/iop/temperature.c,
// process(): multiplies each photosite of the mosaic by the coefficient of its filter
// colour. The setup turns the colour pattern and the coefficients into a coefficient
// for each position of the pattern, so the kernel needs no colour logic. The raw stage
// exports no derivatives (decision 21). It may run in row strips over windows of its
// mosaics (decision 14): photosites are found by their place in the whole mosaics, less
// the windows' starts in the pass header.

import pass;

struct TemperatureParams
{
    // The colour pattern's size: 2×2 for Bayer, 6×6 for X-Trans.
    uint pattern_width;
    uint pattern_height;
    // The mosaic's top-left photosite on the sensor.
    uint origin_x;
    uint origin_y;
    // The coefficient of each position of the pattern, row by row, four to a vector.
    float4 coefficients[9];
};

struct Params
{
    PassHeader pass;
    TemperatureParams temperature;
};

[[vk::binding(0, 0)]] StructuredBuffer<float> input;
[[vk::binding(1, 0)]] RWStructuredBuffer<float> output;
[[vk::binding(2, 0)]] ConstantBuffer<Params> params;

// The coefficient of the photosite at (x, y) on the sensor.
float coefficient(uint x, uint y)
{
    uint width = params.temperature.pattern_width;
    uint i = (y % params.temperature.pattern_height) * width + x % width;
    return params.temperature.coefficients[i / 4][i % 4];
}

// The per-pixel function.
float temperature(float value, float coefficient)
{
    return value * coefficient;
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id: SV_DispatchThreadID)
{
    if (id.x >= params.pass.width || id.y >= params.pass.height)
        return;
    // The photosite in the mosaic, then on the sensor.
    uint col = params.pass.origin_x + id.x;
    uint row = params.pass.origin_y + id.y;
    float c = coefficient(params.temperature.origin_x + col, params.temperature.origin_y + row);
    uint i = row * params.pass.width + col;
    float value = input[i - params.pass.input_base];
    for (uint k = 0; k < params.pass.repeat; k++)
        value = temperature(value, c);
    output[i - params.pass.output_base] = value;
}
