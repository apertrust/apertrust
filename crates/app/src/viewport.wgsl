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

// Draws the engine's latest frame inside egui's render pass (viewport.rs): each
// physical pixel reads its own pixel of the frame, so nothing is resampled, unless the
// frame was rendered for another size, as while the window resizes, when it is shown at
// the size it will be rendered at until that render arrives. A render shader of the
// window's, not a kernel: it has no derivatives and stays WGSL.

struct View {
    // Where the frame is shown, in the window's physical pixels: its top-left corner,
    // its size as rendered, and the size it is shown at.
    origin: vec2<i32>,
    size: vec2<u32>,
    shown: vec2<u32>,
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
    return clamp(pixels[u32(p.y) * view.size.x + u32(p.x)].rgb, vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let p = vec2<i32>(floor(position.xy)) - view.origin;
    if p.x < 0 || p.y < 0 || p.x >= i32(view.shown.x) || p.y >= i32(view.shown.y) {
        discard;
    }
    var rgb: vec3<f32>;
    if all(view.shown == view.size) {
        rgb = texel(p);
    } else {
        // Between the four frame pixels round where this one falls.
        let s = (vec2<f32>(p) + 0.5) * vec2<f32>(view.size) / vec2<f32>(view.shown) - 0.5;
        let last = vec2<i32>(view.size) - vec2<i32>(1);
        let i = vec2<i32>(floor(s));
        let a = clamp(i, vec2<i32>(0), last);
        let b = clamp(i + vec2<i32>(1), vec2<i32>(0), last);
        let f = clamp(s - floor(s), vec2<f32>(0.0), vec2<f32>(1.0));
        let top = mix(texel(a), texel(vec2<i32>(b.x, a.y)), f.x);
        let bottom = mix(texel(vec2<i32>(a.x, b.y)), texel(b), f.x);
        rgb = mix(top, bottom, f.y);
    }
    if view.decode != 0u {
        rgb = srgb_to_linear(rgb);
    }
    return vec4<f32>(rgb, 1.0);
}
