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

//! The photo in the centre, drawn by a paint callback (decisions 17 and 35): our own
//! render pipeline reads a frame of the session's straight from its storage buffer inside
//! egui's render pass. Nothing is copied into an egui texture, and the shader can later
//! encode for float16 or EDR surfaces.
//!
//! Any frame is drawn where its place in the photo ([`FramePlace`]) falls in the current
//! view ([`frame_shown`]), so the last frame stands in for the next while it renders
//! (`docs/gui.md`, While the view renders): one frame pixel per physical pixel at Fit and
//! at 100%, nothing resampled, the photo's offset a whole number of physical pixels; the
//! mean of each 2×2 block at Small; smoothly between 100% and 200%, and for a frame
//! rendered for another size or scale, as while the window resizes; a sharp square from
//! 200% on (decision 99).

use apertrust_engine::view::{FramePlace, Placement};
use eframe::egui;
use eframe::egui_wgpu::{self, CallbackResources, CallbackTrait, RenderState, ScreenDescriptor};
use eframe::wgpu;

const SHADER: &str = include_str!("viewport.wgsl");

/// The frame buffers bound at once: the two the session alternates for the frame, and the
/// two for its overview (decision 99).
const BOUND: usize = 4;

/// The pipeline and what it binds, kept in egui's callback resources.
struct Resources {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    decode: bool,
    /// The frame buffers drawn last, each with its own view, since the overview and the
    /// frame are both drawn in one pass.
    bound: Vec<Bound>,
}

/// A frame buffer bound, with the view it is drawn with.
struct Bound {
    buffer: wgpu::Buffer,
    view: wgpu::Buffer,
    group: wgpu::BindGroup,
}

/// How the frame's pixels become the window's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    /// Each physical pixel its own frame pixel: the frame shown at the scale it was
    /// rendered at, at a whole offset, nothing resampled.
    Exact,
    /// Between the four frame pixels round where each physical pixel falls: a frame shown
    /// at another size or scale, as while the window resizes or the view renders, and the
    /// enlargement from 100% to 200%.
    Smooth,
    /// The mean of each 2×2 block of frame pixels, a lone row or column at an odd edge
    /// counted twice: the fitted frame shown at half, at Small.
    Box2,
    /// The frame pixel each physical pixel falls in, so each is a sharp square: the 100%
    /// frame enlarged from 200% on.
    Nearest,
}

impl Filter {
    /// The shader's number for it (`viewport.wgsl`).
    fn code(self) -> u32 {
        match self {
            Filter::Exact => 0,
            Filter::Smooth => 1,
            Filter::Box2 => 2,
            Filter::Nearest => 3,
        }
    }
}

/// How a frame is drawn into the photo area: `source`, the frame pixel under the area's
/// top-left corner, and `scale`, the physical pixels a frame pixel spans, across and down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shown {
    pub source: [f32; 2],
    pub scale: [f32; 2],
    pub filter: Filter,
}

/// The shader's `View`.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct View {
    origin: [i32; 2],
    shown: [u32; 2],
    size: [u32; 2],
    source: [f32; 2],
    scale: [f32; 2],
    sampling: u32,
    decode: u32,
}

/// The output image at `scale` of the photo `extent` pixels at full resolution, each side
/// rounded and at least a pixel, as the engine rounds it (`FramePlace::scale`).
fn at_scale(extent: [u32; 2], scale: f64) -> [f64; 2] {
    extent.map(|side| (f64::from(side) * scale).round().max(1.0))
}

/// The visible rectangle's corner on `axis` where `placement` puts it, in pixels of the
/// image at the scale rendered, moved to the nearest whole physical pixel, so the photo's
/// offset is a whole number of them at every scale; but never so far that the rectangle
/// passes the image's far edge.
pub fn corner(placement: &Placement, axis: usize) -> f64 {
    // Physical pixels per pixel of the image at the scale rendered.
    let enlarged = placement.shown / placement.render;
    let [at, width] = [placement.visible[axis], placement.visible[axis + 2]];
    let most = ((f64::from(placement.size[axis]) - width) * enlarged).floor();
    (at * enlarged).round().clamp(0.0, most.max(0.0)) / enlarged
}

/// The whole photo in pixels of the frame at `place`, at the scale it was rendered at: what
/// one of its pixels is a step of, as the picker's keys step.
pub fn frame_image(place: &FramePlace) -> [u32; 2] {
    at_scale(place.extent, place.scale).map(|side| side as u32)
}

/// How a frame whose image at its own scale is `image` pixels, its corner at `origin` in
/// it, maps into the view `placement` shows from the visible rectangle's [`corner`].
fn mapped(image: [f64; 2], origin: [u32; 2], placement: &Placement) -> ([f32; 2], [f32; 2]) {
    // Physical pixels per pixel of the image at the scale rendered.
    let enlarged = placement.shown / placement.render;
    let along = |axis: usize| {
        // Frame pixels per pixel of the image at the scale rendered.
        let ratio = image[axis] / f64::from(placement.size[axis].max(1));
        let corner = corner(placement, axis);
        let source = corner * ratio - f64::from(origin[axis]);
        (source as f32, (enlarged / ratio) as f32)
    };
    let ((x, across), (y, down)) = (along(0), along(1));
    ([x, y], [across, down])
}

/// How a frame at `place` is drawn in the view `placement` shows, the detail's or any
/// other (`docs/gui.md`, While the view renders): one frame pixel per physical pixel where
/// it was rendered at the scale the view renders at and is shown at it, at Fit and at
/// 100%; the 2×2 mean at Small; Nearest from 200%; else smoothly.
pub fn frame_shown(place: &FramePlace, placement: &Placement) -> Shown {
    let image = at_scale(place.extent, place.scale);
    let (source, scale) = mapped(image, place.origin, placement);
    let enlarged = placement.shown / placement.render;
    let filter = if !rendered_for(place, placement) {
        Filter::Smooth
    } else if enlarged == 1.0 {
        Filter::Exact
    } else if enlarged == 0.5 {
        Filter::Box2
    } else if placement.shown >= 2.0 {
        Filter::Nearest
    } else {
        Filter::Smooth
    };
    Shown {
        source,
        scale,
        filter,
    }
}

/// Whether a frame at `place` was rendered for the view `placement` shows: at its scale,
/// of the same photo's extent, so a frame pixel is a pixel of the image at that scale.
fn rendered_for(place: &FramePlace, placement: &Placement) -> bool {
    place.scale == placement.render
        && at_scale(place.extent, place.scale) == placement.size.map(f64::from)
}

/// How the overview, the whole photo's fitted frame of `size`, is drawn under the frame in
/// the view `placement` shows: enlarged smoothly (decision 99).
pub fn overview_shown(size: [u32; 2], placement: &Placement) -> Shown {
    let (source, scale) = mapped(size.map(f64::from), [0, 0], placement);
    Shown {
        source,
        scale,
        filter: Filter::Smooth,
    }
}

/// The pixel of a frame at `place`, `size` pixels, under the point `at` of the photo area
/// `area` at `ppp`, in the view `placement` shows: what the values under the cursor read
/// (`docs/gui.md`, While the view renders). None outside the area, where the frame does
/// not reach and where it was rendered at another scale than the view's, a stand-in whose
/// pixels would be a scaled guess.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "the values under the cursor read it once they are built"
    )
)]
pub fn frame_pixel(
    place: &FramePlace,
    size: [u32; 2],
    placement: &Placement,
    area: egui::Rect,
    ppp: f32,
    at: egui::Pos2,
) -> Option<[u32; 2]> {
    if !area.contains(at) || !rendered_for(place, placement) {
        return None;
    }
    let shown = frame_shown(place, placement);
    let p = ((at - area.min) * ppp).floor() + egui::vec2(0.5, 0.5);
    let pixel = |axis: usize| {
        let q = (shown.source[axis] + p[axis] / shown.scale[axis]).floor();
        (q >= 0.0 && q < size[axis] as f32).then_some(q as u32)
    };
    Some([pixel(0)?, pixel(1)?])
}

/// Makes the pipeline for the window's format; once, before the first frame.
pub fn install(state: &RenderState) {
    let resources = Resources::new(&state.device, state.target_format);
    state.renderer.write().callback_resources.insert(resources);
}

impl Resources {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewport"),
            entries: &[
                entry(0, wgpu::BufferBindingType::Uniform),
                entry(1, wgpu::BufferBindingType::Storage { read_only: true }),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("viewport"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        // As egui's own pass: the window's format, one sample, no depth (eframe's
        // defaults).
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("viewport"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        Resources {
            pipeline,
            layout,
            decode: format.is_srgb(),
            bound: Vec::new(),
        }
    }

    /// Writes where `photo`'s frame goes and how it is drawn into the view of its buffer,
    /// binding the buffer if it is new.
    fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, photo: &Photo) {
        let Photo {
            buffer,
            origin,
            shown,
            size,
            view,
        } = photo;
        let uniform = View {
            origin: *origin,
            shown: *shown,
            size: *size,
            source: view.source,
            scale: view.scale,
            sampling: view.filter.code(),
            decode: u32::from(self.decode),
        };
        let at = match self.bound.iter().position(|bound| bound.buffer == *buffer) {
            // The most recent again, so a buffer drawn every frame is never the one let go.
            Some(at) => {
                let bound = self.bound.remove(at);
                self.bound.push(bound);
                self.bound.len() - 1
            }
            None => {
                let view = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("viewport view"),
                    size: size_of::<View>() as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("viewport"),
                    layout: &self.layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: view.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: buffer.as_entire_binding(),
                        },
                    ],
                });
                if self.bound.len() == BOUND {
                    self.bound.remove(0);
                }
                self.bound.push(Bound {
                    buffer: buffer.clone(),
                    view,
                    group,
                });
                self.bound.len() - 1
            }
        };
        queue.write_buffer(&self.bound[at].view, 0, bytemuck::bytes_of(&uniform));
    }

    /// Draws the frame in `buffer` over the pass's viewport, once it is prepared.
    fn draw(&self, pass: &mut wgpu::RenderPass<'_>, buffer: &wgpu::Buffer) {
        let Some(bound) = self.bound.iter().find(|bound| bound.buffer == *buffer) else {
            return;
        };
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bound.group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// Draws the frame in `buffer`, `size` pixels, over the photo area `area`, whose corners
/// lie on the physical pixel grid, as `shown` maps it there; where the frame does not
/// reach, what is under it shows. `painter`'s clip rect cuts it.
pub fn paint(
    painter: &egui::Painter,
    area: egui::Rect,
    buffer: &wgpu::Buffer,
    size: [u32; 2],
    shown: Shown,
) {
    let ppp = painter.pixels_per_point();
    let pixels = |v: f32| (v * ppp).round();
    let callback = Photo {
        buffer: buffer.clone(),
        origin: [pixels(area.min.x) as i32, pixels(area.min.y) as i32],
        shown: [
            pixels(area.width()).max(1.0) as u32,
            pixels(area.height()).max(1.0) as u32,
        ],
        size,
        view: shown,
    };
    painter.add(egui_wgpu::Callback::new_paint_callback(area, callback));
}

/// A frame drawn, where and how: the paint callback.
struct Photo {
    buffer: wgpu::Buffer,
    /// The area's corner and size, in physical pixels.
    origin: [i32; 2],
    shown: [u32; 2],
    /// The frame's size.
    size: [u32; 2],
    view: Shown,
}

impl CallbackTrait for Photo {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(r) = resources.get_mut::<Resources>() {
            r.prepare(device, queue, self);
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        if let Some(r) = resources.get::<Resources>() {
            r.draw(pass, &self.buffer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apertrust_engine::gpu::GpuContext;
    use apertrust_engine::view::{View as ViewOf, ViewScale, place};

    #[test]
    fn the_shader_is_valid_wgsl() {
        let module = naga::front::wgsl::parse_str(SHADER).expect("it parses");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::default(),
        )
        .validate(&module)
        .expect("it validates");
    }

    #[test]
    fn the_view_matches_the_shader_s_layout() {
        let module = naga::front::wgsl::parse_str(SHADER).expect("it parses");
        let (_, view) = module
            .types
            .iter()
            .find(|(_, ty)| ty.name.as_deref() == Some("View"))
            .expect("the shader's View");
        let naga::TypeInner::Struct { members, span } = &view.inner else {
            panic!("View is a struct");
        };
        let offsets: Vec<(&str, u32)> = members
            .iter()
            .map(|m| (m.name.as_deref().unwrap_or_default(), m.offset))
            .collect();
        assert_eq!(
            offsets,
            [
                ("origin", std::mem::offset_of!(View, origin) as u32),
                ("shown", std::mem::offset_of!(View, shown) as u32),
                ("size", std::mem::offset_of!(View, size) as u32),
                ("source", std::mem::offset_of!(View, source) as u32),
                ("scale", std::mem::offset_of!(View, scale) as u32),
                ("sampling", std::mem::offset_of!(View, sampling) as u32),
                ("decode", std::mem::offset_of!(View, decode) as u32),
            ]
        );
        assert_eq!(*span as usize, size_of::<View>());
    }

    /// The test's GPU, hardware or software.
    fn gpu() -> GpuContext {
        pollster::block_on(GpuContext::new(&Default::default()))
            .expect("a GPU adapter, hardware or software")
    }

    /// A frame drawn in a test: its buffer and size, the area's corner and size in the
    /// target, and how it maps there.
    struct Drawn {
        buffer: wgpu::Buffer,
        size: [u32; 2],
        origin: [i32; 2],
        shown: [u32; 2],
        view: Shown,
    }

    /// A frame of `size` whose pixels `pixel` gives, uploaded.
    fn frame(
        gpu: &GpuContext,
        size: [u32; 2],
        pixel: impl Fn(u32, u32) -> [f32; 4],
    ) -> wgpu::Buffer {
        let pixels: Vec<[f32; 4]> = (0..size[1])
            .flat_map(|y| (0..size[0]).map(move |x| (x, y)))
            .map(|(x, y)| pixel(x, y))
            .collect();
        wgpu::util::DeviceExt::create_buffer_init(
            &gpu.device,
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&pixels),
                usage: wgpu::BufferUsages::STORAGE,
            },
        )
    }

    /// Draws each of `draws` in order into a target of `target` pixels of `format` cleared
    /// to black, as egui's pass does: every one prepared, then each drawn with the
    /// callback's viewport. Returns each row's bytes, padded rows cut.
    fn render_all(
        gpu: &GpuContext,
        format: wgpu::TextureFormat,
        target: [u32; 2],
        draws: &[Drawn],
    ) -> Vec<u8> {
        let (device, queue) = (&gpu.device, &gpu.queue);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: target[0],
                height: target[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let mut resources = Resources::new(device, format);
        for drawn in draws {
            let photo = Photo {
                buffer: drawn.buffer.clone(),
                origin: drawn.origin,
                shown: drawn.shown,
                size: drawn.size,
                view: drawn.view,
            };
            resources.prepare(device, queue, &photo);
        }
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let view = texture.create_view(&Default::default());
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            for drawn in draws {
                let [x, y] = drawn.origin.map(|v| v as f32);
                let [w, h] = drawn.shown.map(|v| v as f32);
                pass.set_viewport(x, y, w, h, 0.0, 1.0);
                resources.draw(&mut pass, &drawn.buffer);
            }
        }
        let texel = format.block_copy_size(None).expect("a colour format");
        let row = (target[0] * texel).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(row * target[1]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: None,
                },
            },
            texture.size(),
        );
        gpu.submit([encoder.finish()]);
        readback.map_async(wgpu::MapMode::Read, .., |result| result.expect("it maps"));
        gpu.wait().expect("the GPU finishes");
        let mapped = readback.get_mapped_range(..).expect("it is mapped");
        mapped
            .chunks(row as usize)
            .flat_map(|line| line[..(target[0] * texel) as usize].to_vec())
            .collect()
    }

    /// The 12×10 target's pixels as 8 bits, `draws` drawn into it.
    fn render_unorm(
        format: wgpu::TextureFormat,
        draws: &[Drawn],
        gpu: &GpuContext,
    ) -> Vec<[u8; 4]> {
        render_all(gpu, format, [12, 10], draws)
            .chunks(4)
            .map(|p| [p[0], p[1], p[2], p[3]])
            .collect()
    }

    /// Draws a frame of `size` whose pixels `pixel` gives at (3, 2) of a 12×10 target, its
    /// area `shown` physical pixels, mapped there by `view`, and reads every pixel back.
    fn render_frame(
        format: wgpu::TextureFormat,
        size: [u32; 2],
        shown: [u32; 2],
        view: Shown,
        pixel: impl Fn(u32, u32) -> [f32; 4],
    ) -> Vec<[u8; 4]> {
        let gpu = gpu();
        let buffer = frame(&gpu, size, pixel);
        let drawn = Drawn {
            buffer,
            size,
            origin: [3, 2],
            shown,
            view,
        };
        render_unorm(format, &[drawn], &gpu)
    }

    /// A frame of `size` shown stretched to `shown`, as one rendered for another size.
    fn stretched(size: [u32; 2], shown: [u32; 2]) -> Shown {
        Shown {
            source: [0.0, 0.0],
            scale: [0, 1].map(|axis| shown[axis] as f32 / size[axis] as f32),
            filter: Filter::Smooth,
        }
    }

    /// A frame drawn one frame pixel per physical pixel from `source`.
    fn exact(source: [f32; 2]) -> Shown {
        Shown {
            source,
            scale: [1.0, 1.0],
            filter: Filter::Exact,
        }
    }

    /// A 5×4 ramp, x across in red and y down in green.
    fn ramp(x: u32, y: u32) -> [f32; 4] {
        [x as f32 / 8.0, y as f32 / 8.0, 0.5, 1.0]
    }

    fn unorm(v: f32) -> u8 {
        (v * 255.0).round() as u8
    }

    /// Each pixel of the 12×10 target is `inside`'s colour within `area`, x and y ranges,
    /// else black.
    fn assert_target(
        target: &[[u8; 4]],
        area: (std::ops::Range<usize>, std::ops::Range<usize>),
        inside: impl Fn(usize, usize) -> [u8; 4],
    ) {
        for y in 0..10 {
            for x in 0..12 {
                let expected = if area.0.contains(&x) && area.1.contains(&y) {
                    inside(x, y)
                } else {
                    [0, 0, 0, 255]
                };
                assert_eq!(target[y * 12 + x], expected, "({x}, {y})");
            }
        }
    }

    #[test]
    fn a_frame_rendered_for_another_size_fills_the_size_it_is_shown_at() {
        let red = [0.75, 0.25, 0.5, 1.0];
        let view = stretched([5, 4], [8, 7]);
        let target = render_frame(
            wgpu::TextureFormat::Rgba8Unorm,
            [5, 4],
            [8, 7],
            view,
            |_, _| red,
        );
        let colour = [unorm(red[0]), unorm(red[1]), unorm(red[2]), 255];
        assert_target(&target, (3..11, 2..9), |_, _| colour);
    }

    #[test]
    fn a_frame_shown_larger_keeps_its_edges_and_its_order() {
        // A ramp along x, the same down each column, 5 pixels wide shown 9 wide.
        let ramp = |x: u32, _| [x as f32 / 4.0, 0.0, 0.0, 1.0];
        let view = stretched([5, 4], [9, 4]);
        let target = render_frame(wgpu::TextureFormat::Rgba8Unorm, [5, 4], [9, 4], view, ramp);
        for y in 2..6 {
            let row: Vec<u8> = (3..12).map(|x| target[y * 12 + x][0]).collect();
            // The edges are the frame's own first and last pixels, not leaning inwards.
            assert_eq!(row[0], unorm(0.0), "row {y}: {row:?}");
            assert_eq!(row[8], unorm(1.0), "row {y}: {row:?}");
            assert!(row.windows(2).all(|w| w[0] <= w[1]), "row {y}: {row:?}");
        }
    }

    #[test]
    fn exact_still_reads_one_pixel_per_pixel() {
        let view = exact([0.0, 0.0]);
        let target = render_frame(wgpu::TextureFormat::Rgba8Unorm, [5, 4], [5, 4], view, ramp);
        assert_target(&target, (3..8, 2..6), |x, y| {
            let [r, g, b, _] = ramp(x as u32 - 3, y as u32 - 2);
            [unorm(r), unorm(g), unorm(b), 255]
        });
    }

    #[test]
    fn at_one_to_one_each_physical_pixel_reads_its_own_frame_pixel() {
        // An 8×6 frame whose pixel (2, 1) is under the area's corner: the area shows 5×4
        // of it, each pixel its own.
        let value = |x: u32, y: u32| [x as f32 / 8.0, y as f32 / 8.0, 0.25, 1.0];
        let view = exact([2.0, 1.0]);
        let target = render_frame(wgpu::TextureFormat::Rgba8Unorm, [8, 6], [5, 4], view, value);
        assert_target(&target, (3..8, 2..6), |x, y| {
            let [r, g, b, _] = value(x as u32 - 3 + 2, y as u32 - 2 + 1);
            [unorm(r), unorm(g), unorm(b), 255]
        });
    }

    #[test]
    fn from_200_percent_each_frame_pixel_is_a_square() {
        // At 3×, from frame pixel (1, 1): every 3×3 block of the 9×6 area is one texel.
        let value = |x: u32, y: u32| [(x + 4 * y) as f32 * 8.0 / 255.0, 0.0, 0.5, 1.0];
        let view = Shown {
            source: [1.0, 1.0],
            scale: [3.0, 3.0],
            filter: Filter::Nearest,
        };
        let target = render_frame(wgpu::TextureFormat::Rgba8Unorm, [4, 4], [9, 6], view, value);
        assert_target(&target, (3..12, 2..8), |x, y| {
            let (fx, fy) = (1 + (x as u32 - 3) / 3, 1 + (y as u32 - 2) / 3);
            [unorm(value(fx, fy)[0]), 0, unorm(0.5), 255]
        });
    }

    #[test]
    fn between_100_and_200_percent_it_is_smooth() {
        // A ramp along x at 1.5×: values between the frame's, rising, not repeated in
        // squares.
        let ramp = |x: u32, _| [x as f32 / 5.0, 0.0, 0.0, 1.0];
        let view = Shown {
            source: [0.0, 0.0],
            scale: [1.5, 1.5],
            filter: Filter::Smooth,
        };
        let target = render_frame(wgpu::TextureFormat::Rgba8Unorm, [6, 4], [9, 6], view, ramp);
        for y in 2..8 {
            let row: Vec<u8> = (3..12).map(|x| target[y * 12 + x][0]).collect();
            assert!(row.windows(2).all(|w| w[0] < w[1]), "row {y}: {row:?}");
            let own: Vec<u8> = (0..6).map(|x| unorm(x as f32 / 5.0)).collect();
            assert!(row.iter().any(|v| !own.contains(v)), "row {y}: {row:?}");
        }
    }

    #[test]
    fn box2_averages_each_two_by_two_block() {
        // A 4×4 frame of distinct values, whose blocks' means fall on 8-bit steps.
        let value = |x: u32, y: u32| (x + 4 * y) as f32 * 4.0 / 255.0;
        let pixel = |x, y| [value(x, y), 1.0 - value(x, y), 0.5, 1.0];
        let view = Shown {
            source: [0.0, 0.0],
            scale: [0.5, 0.5],
            filter: Filter::Box2,
        };
        let target = render_frame(wgpu::TextureFormat::Rgba8Unorm, [4, 4], [2, 2], view, pixel);
        assert_target(&target, (3..5, 2..4), |x, y| {
            let (bx, by) = (2 * (x - 3) as u32, 2 * (y - 2) as u32);
            let block = [(bx, by), (bx + 1, by), (bx, by + 1), (bx + 1, by + 1)];
            let mean = block.iter().map(|&(x, y)| value(x, y)).sum::<f32>() / 4.0;
            [unorm(mean), unorm(1.0 - mean), unorm(0.5), 255]
        });
    }

    #[test]
    fn the_overview_fills_what_the_frame_does_not_reach() {
        // A grey overview under the whole 8×6 area, enlarged twice, and a red frame
        // reaching only its left 5 columns and top 3 rows from (3, 2).
        let gpu = gpu();
        let grey = [0.5, 0.5, 0.5, 1.0];
        let red = [1.0, 0.0, 0.0, 1.0];
        let overview = Drawn {
            buffer: frame(&gpu, [4, 3], |_, _| grey),
            size: [4, 3],
            origin: [3, 2],
            shown: [8, 6],
            view: stretched([4, 3], [8, 6]),
        };
        let detail = Drawn {
            buffer: frame(&gpu, [5, 3], |_, _| red),
            size: [5, 3],
            origin: [3, 2],
            shown: [8, 6],
            view: exact([0.0, 0.0]),
        };
        let target = render_unorm(wgpu::TextureFormat::Rgba8Unorm, &[overview, detail], &gpu);
        assert_target(&target, (3..11, 2..8), |x, y| {
            if x < 8 && y < 5 {
                [255, 0, 0, 255]
            } else {
                [128, 128, 128, 255]
            }
        });
        // With no overview, the frame alone, and the surround where it does not reach.
        let detail = Drawn {
            buffer: frame(&gpu, [5, 3], |_, _| red),
            size: [5, 3],
            origin: [3, 2],
            shown: [8, 6],
            view: exact([0.0, 0.0]),
        };
        let target = render_unorm(wgpu::TextureFormat::Rgba8Unorm, &[detail], &gpu);
        assert_target(&target, (3..8, 2..5), |_, _| [255, 0, 0, 255]);
    }

    /// The shader decodes, the target encodes again, and the GPU's encoding rounds its
    /// own way: within one 8-bit step. eframe picks a format that does not encode, so
    /// this is the fallback.
    #[test]
    fn an_srgb_window_shows_the_same_values_within_a_step() {
        let view = exact([0.0, 0.0]);
        let srgb = render_frame(
            wgpu::TextureFormat::Rgba8UnormSrgb,
            [5, 4],
            [5, 4],
            view,
            ramp,
        );
        let unorm = render_frame(wgpu::TextureFormat::Rgba8Unorm, [5, 4], [5, 4], view, ramp);
        for (i, (a, b)) in srgb.iter().zip(&unorm).enumerate() {
            assert!(
                a.iter().zip(b).all(|(a, b)| a.abs_diff(*b) <= 1),
                "pixel {i}: {a:?} against {b:?}"
            );
        }
    }

    /// A 6000 × 4000 photo in a room of 2400 × 1600, fitted at 0.4.
    const EXTENT: [u32; 2] = [6000, 4000];
    const ROOM: (u32, u32) = (2400, 1600);

    fn placed(scale: ViewScale, centre: [f32; 2]) -> Placement {
        place(EXTENT, ROOM, &ViewOf { scale, centre })
    }

    #[test]
    fn each_frame_maps_into_the_view_it_is_drawn_in() {
        let fitted = FramePlace {
            scale: 0.4,
            origin: [0, 0],
            extent: EXTENT,
        };
        // At Fit the fitted frame one for one; at Small its 2×2 means.
        let fit = frame_shown(&fitted, &placed(ViewScale::Fit, [0.5, 0.5]));
        assert_eq!(fit, exact([0.0, 0.0]));
        let small = frame_shown(&fitted, &placed(ViewScale::Small, [0.5, 0.5]));
        assert_eq!((small.scale, small.filter), ([0.5, 0.5], Filter::Box2));
        // At 100%, the detail of the visible rectangle one for one, at a whole offset.
        let at = placed(ViewScale::Factor(1.0), [0.5, 0.5]);
        let detail = FramePlace {
            scale: 1.0,
            origin: [1800, 1200],
            extent: EXTENT,
        };
        assert_eq!(frame_shown(&detail, &at), exact([0.0, 0.0]));
        // The fitted frame stands in for it, enlarged smoothly about the same place.
        let stand_in = frame_shown(&fitted, &at);
        assert_eq!(stand_in.filter, Filter::Smooth);
        assert_eq!(stand_in.scale, [2.5, 2.5]);
        assert_eq!(stand_in.source, [720.0, 480.0]);
        // From 200% the detail in squares, from 100% to 200% smoothly.
        let two = placed(ViewScale::Factor(2.0), [0.5, 0.5]);
        assert_eq!(frame_shown(&detail, &two).filter, Filter::Nearest);
        let one_and_a_half = placed(ViewScale::Factor(1.5), [0.5, 0.5]);
        assert_eq!(frame_shown(&detail, &one_and_a_half).filter, Filter::Smooth);
        // A frame rendered for another room, as while the window resizes, smoothly.
        let resized = place(EXTENT, (2000, 1600), &ViewOf::default());
        assert_eq!(frame_shown(&fitted, &resized).filter, Filter::Smooth);
        // And one of the photo before it turned, at the same scale.
        let turned = place([4000, 6000], ROOM, &ViewOf::default());
        let before = FramePlace {
            scale: turned.render,
            ..fitted
        };
        assert_eq!(frame_shown(&before, &turned).filter, Filter::Smooth);
        // The overview, under the whole photo area, smoothly.
        // A picker steps by the frame's pixels: the fitted frame's at 40%, the detail's at
        // 100%, whatever the view.
        assert_eq!(frame_image(&fitted), [2400, 1600]);
        assert_eq!(frame_image(&detail), EXTENT);
        let overview = overview_shown([2400, 1600], &at);
        assert_eq!(
            (overview.scale, overview.filter),
            ([2.5, 2.5], Filter::Smooth)
        );
    }

    #[test]
    fn a_pan_s_offset_is_whole_physical_pixels() {
        // A centre whose visible rectangle starts between pixels.
        let at = placed(ViewScale::Factor(1.0), [0.50013, 0.50021]);
        assert_ne!(at.visible[0].fract(), 0.0);
        let detail = FramePlace {
            scale: 1.0,
            origin: [1800, 1200],
            extent: EXTENT,
        };
        let shown = frame_shown(&detail, &at);
        assert_eq!(shown.filter, Filter::Exact);
        assert_eq!(shown.source.map(f32::fract), [0.0, 0.0]);
    }

    #[test]
    fn a_fractional_enlargement_keeps_whole_physical_pixels() {
        // At 150% the frame's edge falls on a whole physical pixel, wherever the centre is.
        for centre in [[0.5, 0.5], [0.50013, 0.50021], [0.3671, 0.6129]] {
            let at = placed(ViewScale::Factor(1.5), centre);
            let detail = FramePlace {
                scale: 1.0,
                origin: [1800, 1200],
                extent: EXTENT,
            };
            let shown = frame_shown(&detail, &at);
            let edge = shown.source.map(|source| -source * 1.5);
            assert_eq!(edge.map(f32::fract), [0.0, 0.0], "{centre:?}: {edge:?}");
        }
    }

    #[test]
    fn the_far_edge_never_shows_the_surround() {
        // At 1600% a room 999 pixels wide sees 62.4375 photo pixels: at the right edge the
        // view's corner stays where the whole of them is in the photo.
        let at = place(
            EXTENT,
            (999, 1600),
            &ViewOf {
                scale: ViewScale::Factor(16.0),
                centre: [1.0, 0.5],
            },
        );
        let x = corner(&at, 0);
        assert!(x + at.visible[2] <= 6000.0, "{x}");
        assert_eq!((x * 16.0).fract(), 0.0);
        // The detail of its region reaches the area's right edge.
        let detail = FramePlace {
            scale: 1.0,
            origin: [5937, 1950],
            extent: EXTENT,
        };
        let shown = frame_shown(&detail, &at);
        let right = (63.0 - shown.source[0]) * shown.scale[0];
        assert!(right >= 999.0, "{right}");
    }

    #[test]
    fn a_buffer_drawn_every_frame_stays_bound() {
        let gpu = gpu();
        let mut resources = Resources::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm);
        let buffers: Vec<wgpu::Buffer> = (0..6)
            .map(|_| frame(&gpu, [1, 1], |_, _| [0.0; 4]))
            .collect();
        let prepare = |resources: &mut Resources, buffer: &wgpu::Buffer| {
            let photo = Photo {
                buffer: buffer.clone(),
                origin: [0, 0],
                shown: [1, 1],
                size: [1, 1],
                view: exact([0.0, 0.0]),
            };
            resources.prepare(&gpu.device, &gpu.queue, &photo);
        };
        // The overview, first bound, drawn again each frame while new details come.
        for detail in &buffers[1..] {
            prepare(&mut resources, &buffers[0]);
            prepare(&mut resources, detail);
        }
        let bound = |buffer: &wgpu::Buffer| resources.bound.iter().any(|b| b.buffer == *buffer);
        assert!(bound(&buffers[0]));
        assert_eq!(resources.bound.len(), BOUND);
    }

    #[test]
    fn values_under_the_cursor_are_blank_over_the_overview() {
        let area = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(1200.0, 800.0));
        let at = placed(ViewScale::Factor(1.0), [0.5, 0.5]);
        // A detail frame of the right half of the view, at 100%.
        let detail = FramePlace {
            scale: 1.0,
            origin: [3000, 1200],
            extent: EXTENT,
        };
        let size = [1200, 1600];
        let read = |at_point| frame_pixel(&detail, size, &at, area, 2.0, at_point);
        // Over the right half, its pixel; over the left, the overview's, nothing.
        assert_eq!(read(egui::pos2(1210.0 - 0.25, 20.25)), Some([1199, 0]));
        assert_eq!(read(egui::pos2(610.25, 20.25)), Some([0, 0]));
        assert_eq!(read(egui::pos2(300.0, 300.0)), None);
        // Outside the area, nothing.
        assert_eq!(read(egui::pos2(5.0, 300.0)), None);
        // The fitted frame standing in at 100% is a scaled guess: nothing.
        let fitted = FramePlace {
            scale: 0.4,
            origin: [0, 0],
            extent: EXTENT,
        };
        let point = egui::pos2(600.0, 400.0);
        assert_eq!(
            frame_pixel(&fitted, [2400, 1600], &at, area, 2.0, point),
            None
        );
        // At Fit it reads it.
        let fit = placed(ViewScale::Fit, [0.5, 0.5]);
        let pixel = frame_pixel(&fitted, [2400, 1600], &fit, area, 2.0, point);
        assert_eq!(pixel, Some([1180, 760]));
    }

    /// Through a real session: the 100% view of the synthetic photo, drawn at a whole
    /// offset into a float target, is the same pixels as the untiled export of it, bit for
    /// bit, as `apertrust-cli export` renders it before rounding to 8 bits (principle 2).
    #[test]
    fn the_100_percent_view_shows_the_export_s_pixels() {
        use crate::mirror::tests::{Live, scratch};
        use apertrust_engine::compute::GpuImage;
        use apertrust_engine::edit::{self, Edit};
        use apertrust_engine::export::{Tiling, export};
        use apertrust_engine::session::Command;

        let Some(mut live) = Live::start() else {
            return;
        };
        let folder = scratch("zoom-100");
        live.open(&folder.join("exact.ARW"));
        live.until_live();
        let view = ViewOf {
            scale: ViewScale::Factor(1.0),
            centre: [0.4, 0.6],
        };
        live.session.send(Command::View(view));
        live.until(|m| m.frame.as_ref().is_some_and(|f| f.place.scale == 1.0));
        let frame = live.mirror.frame.clone().expect("the 100% frame");
        let placement = place(frame.place.extent, (64, 48), &view);
        let shown = frame_shown(&frame.place, &placement);
        assert_eq!(shown.filter, Filter::Exact);
        // The area, 64 × 48 or the photo where smaller, at (5, 3) of the target.
        let area = [placement.visible[2], placement.visible[3]].map(|side| side as u32);
        let drawn = Drawn {
            buffer: frame.buffer.clone(),
            size: [frame.width, frame.height],
            origin: [5, 3],
            shown: area,
            view: shown,
        };
        let target = [area[0] + 10, area[1] + 6];
        let format = wgpu::TextureFormat::Rgba32Float;
        let bytes = render_all(&live.gpu, format, target, &[drawn]);
        let drawn: Vec<[f32; 4]> = bytemuck::pod_collect_to_vec(&bytes);

        // The export of the same photo with its new edit, untiled.
        let image = apertrust_tools::testing::image();
        let instances = edit::instances(
            &Edit::default(),
            apertrust_operations::OPERATIONS,
            apertrust_tools::TOOLS,
            &image,
        )
        .all()
        .expect("the default edit's instances");
        let mosaic = apertrust_tools::testing::mosaic();
        let input = GpuImage::upload_mosaic(&live.gpu, 96, 72, &mosaic).expect("the mosaic");
        let exported =
            pollster::block_on(export(&live.gpu, &instances, &image, input, Tiling::Whole))
                .expect("the export");
        assert_eq!(
            [exported.desc.width, exported.desc.height],
            frame.place.extent
        );
        // The visible rectangle's corner, rounded as the view draws it.
        let corner = [placement.visible[0], placement.visible[1]].map(|v| v.round() as u32);
        for y in 0..area[1] {
            for x in 0..area[0] {
                let at = ((corner[1] + y) * exported.desc.width + corner[0] + x) as usize;
                let expected = exported.pixels[at].map(|v| v.clamp(0.0, 1.0));
                let mut expected = expected.map(f32::to_bits);
                expected[3] = 1.0_f32.to_bits();
                let got = drawn[((y + 3) * target[0] + x + 5) as usize].map(f32::to_bits);
                assert_eq!(got, expected, "({x}, {y})");
            }
        }
        let _ = std::fs::remove_dir_all(&folder);
    }
}
