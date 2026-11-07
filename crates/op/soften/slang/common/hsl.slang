// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/common/colorspaces.h)
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

// darktable's HSL (src/common/colorspaces.h, rgb2hsl, hue2rgb and hsl2rgb), in f32. In
// darktable soften, colorize, splittoning and four other modules call it; soften is
// its only port so far, so it stays here (decision 65: a helper joins a shared crate
// once a second port that calls it is scheduled). darktable computes a few steps in
// double, which these compute in f32, each a last bit (docs/ports/soften.md,
// Precision). The Rust restatement, darktable's doubles included, is `crate::hsl`.
module hsl;

// darktable's floor on the saturation's denominators, 2^-16.
static const float SATURATION_FLOOR = 1.52587890625e-05;

// darktable rgb2hsl: hue in [0, 1], saturation and lightness.
public float3 rgb_to_hsl(float3 rgb)
{
    float r = rgb.r;
    float g = rgb.g;
    float b = rgb.b;
    float pmax = max(max(r, g), b);
    float pmin = min(min(r, g), b);
    float delta = pmax - pmin;
    float h = 0.0;
    float s = 0.0;
    float l = (pmin + pmax) / 2.0;
    if (delta != 0.0)
    {
        s = l < 0.5 ? delta / max(pmax + pmin, SATURATION_FLOOR)
                    : delta / max(2.0 - pmax - pmin, SATURATION_FLOOR);
        if (pmax == r)
            h = (g - b) / delta;
        else if (pmax == g)
            h = 2.0 + (b - r) / delta;
        else if (pmax == b)
            h = 4.0 + (r - g) / delta;
        h /= 6.0;
        if (h < 0.0)
            h += 1.0;
        else if (h > 1.0)
            h -= 1.0;
    }
    return float3(h, s, l);
}

// darktable hue2rgb: one channel from the hue, already scaled to [0, 6).
float hue_to_rgb(float m1, float m2, float hue)
{
    if (hue < 1.0)
        return m1 + (m2 - m1) * hue;
    else if (hue < 3.0)
        return m2;
    else
        return hue < 4.0 ? m1 + (m2 - m1) * (4.0 - hue) : m1;
}

// darktable hsl2rgb, from hue in [0, 1], saturation and lightness.
public float3 hsl_to_rgb(float3 hsl)
{
    float h = hsl.x;
    float s = hsl.y;
    float l = hsl.z;
    if (s == 0.0)
        return float3(l, l, l);
    float m2 = l < 0.5 ? l * (1.0 + s) : l + s - l * s;
    float m1 = 2.0 * l - m2;
    h *= 6.0;
    return float3(
        hue_to_rgb(m1, m2, h < 4.0 ? h + 2.0 : h - 4.0),
        hue_to_rgb(m1, m2, h),
        hue_to_rgb(m1, m2, h > 2.0 ? h - 2.0 : h + 4.0));
}
