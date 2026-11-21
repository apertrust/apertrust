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

// Draws a frame of the engine's inside egui's render pass (viewport.rs): each physical
// pixel of the area reads the frame where the view puts it, `source` the frame pixel under
// the area's corner and `scale` the physical pixels a frame pixel spans. At one to one it
// reads its own frame pixel, nothing resampled; at half, at Small, the mean of a 2×2
// block; from 200% the frame pixel it falls in, a sharp square; otherwise between the four
// round where it falls, as for a frame rendered for another size or scale. Where the frame
// does not reach, nothing is drawn, so what is under it shows. A render shader of the
// window's, not a kernel: it has no derivatives and stays WGSL.

struct View {
    // The area, in the window's physical pixels: its top-left corner and its size.
    origin: vec2<i32>,
    shown: vec2<u32>,
    // The frame's size in its pixels.
    size: vec2<u32>,
    // The frame pixel under the area's top-left corner, and the physical pixels each frame
    // pixel spans, across and down.
    source: vec2<f32>,
    scale: vec2<f32>,
    // How frame pixels become the window's: 0 one for one, 1 between the four round where
    // a pixel falls, 2 the mean of each 2×2 block, 3 the one it falls in (viewport.rs,
    // `Filter`).
    sampling: u32,
    // 1 when the window's format encodes to sRGB itself, so the frame is decoded first.
    decode: u32,
}

@group(0) @binding(0) var<uniform> view: View;
// RGBA in the output profile's encoding, rows top to bottom.
@group(0) @binding(1) var<storage, read> pixels: array<vec4<f32>>;

// One triangle over the whole viewport egui sets for the callback.
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(index & 1u) * 4 - 1);
    let y = f32(i32(index >> 1u) * 4 - 1);
    return vec4<f32>(x, y, 0.0, 1.0);
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

fn texel(p: vec2<i32>) -> vec3<f32> {
    let q = clamp(p, vec2<i32>(0), vec2<i32>(view.size) - vec2<i32>(1));
    return clamp(pixels[u32(q.y) * view.size.x + u32(q.x)].rgb, vec3<f32>(0.0), vec3<f32>(1.0));
}

// Whether the frame pixel `q` lies in the frame.
fn inside(q: vec2<i32>) -> bool {
    return all(q >= vec2<i32>(0)) && all(q < vec2<i32>(view.size));
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let p = vec2<i32>(floor(position.xy)) - view.origin;
    if p.x < 0 || p.y < 0 || p.x >= i32(view.shown.x) || p.y >= i32(view.shown.y) {
        discard;
    }
    // Where the physical pixel's centre falls in the frame.
    let at = view.source + (vec2<f32>(p) + 0.5) / view.scale;
    let corner = vec2<i32>(floor(view.source));
    var rgb: vec3<f32>;
    if view.sampling == 0u {
        let q = corner + p;
        if !inside(q) {
            discard;
        }
        rgb = texel(q);
    } else if view.sampling == 2u {
        // The block's far row and column stay inside the frame at an odd edge.
        let a = corner + p * 2;
        if !inside(a) {
            discard;
        }
        let last = vec2<i32>(view.size) - vec2<i32>(1);
        let b = min(a + vec2<i32>(1), last);
        rgb = (texel(a) + texel(vec2<i32>(b.x, a.y)) + texel(vec2<i32>(a.x, b.y)) + texel(b)) * 0.25;
    } else if view.sampling == 3u {
        let q = vec2<i32>(floor(at));
        if !inside(q) {
            discard;
        }
        rgb = texel(q);
    } else {
        if any(at < vec2<f32>(0.0)) || any(at >= vec2<f32>(view.size)) {
            discard;
        }
        // Between the four frame pixels round where this one falls.
        let s = at - 0.5;
        let i = vec2<i32>(floor(s));
        let f = clamp(s - floor(s), vec2<f32>(0.0), vec2<f32>(1.0));
        let b = i + vec2<i32>(1);
        let top = mix(texel(i), texel(vec2<i32>(b.x, i.y)), f.x);
        let bottom = mix(texel(vec2<i32>(i.x, b.y)), texel(b), f.x);
        rgb = mix(top, bottom, f.y);
    }
    if view.decode != 0u {
        rgb = srgb_to_linear(rgb);
    }
    return vec4<f32>(rgb, 1.0);
}
