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
//! render pipeline reads the session's latest frame straight from its storage buffer
//! inside egui's render pass, one frame pixel per physical pixel. Nothing is copied
//! into an egui texture, and the shader can later encode for float16 or EDR surfaces.
//! A frame rendered for another size, as while the window resizes, is shown scaled to
//! the size its successor will have, until that arrives.

use apertrust_engine::session::Frame;
use eframe::egui;
use eframe::egui_wgpu::{self, CallbackResources, CallbackTrait, RenderState, ScreenDescriptor};
use eframe::wgpu;

const SHADER: &str = include_str!("viewport.wgsl");

/// The pipeline and what it binds, kept in egui's callback resources.
struct Resources {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    view: wgpu::Buffer,
    decode: bool,
    /// A bind group for each of the last two frame buffers: the session alternates two.
    bound: Vec<(wgpu::Buffer, wgpu::BindGroup)>,
}

/// The shader's `View`.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct View {
    origin: [i32; 2],
    size: [u32; 2],
    shown: [u32; 2],
    decode: u32,
    _pad: u32,
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
        let view = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewport view"),
            size: size_of::<View>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Resources {
            pipeline,
            layout,
            view,
            decode: format.is_srgb(),
            bound: Vec::new(),
        }
    }

    /// Writes where the frame in `buffer` goes, its size and the size it is shown at, and
    /// binds the buffer if it is new.
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffer: &wgpu::Buffer,
        origin: [i32; 2],
        size: [u32; 2],
        shown: [u32; 2],
    ) {
        let view = View {
            origin,
            size,
            shown,
            decode: u32::from(self.decode),
            _pad: 0,
        };
        queue.write_buffer(&self.view, 0, bytemuck::bytes_of(&view));
        if !self.bound.iter().any(|(bound, _)| bound == buffer) {
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("viewport"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.view.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: buffer.as_entire_binding(),
                    },
                ],
            });
            self.bound.push((buffer.clone(), group));
            if self.bound.len() > 2 {
                self.bound.remove(0);
            }
        }
    }

    /// Draws the frame in `buffer` over the pass's viewport, once it is prepared.
    fn draw(&self, pass: &mut wgpu::RenderPass<'_>, buffer: &wgpu::Buffer) {
        let Some((_, group)) = self.bound.iter().find(|(bound, _)| bound == buffer) else {
            return;
        };
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// Draws `frame` over `rect`, whose corners lie on the physical pixel grid: pixel for
/// pixel where `rect` is the frame's size, scaled to it otherwise. `painter`'s clip rect
/// cuts it.
pub fn paint(painter: &egui::Painter, rect: egui::Rect, frame: &Frame) {
    let ppp = painter.pixels_per_point();
    let pixels = |v: f32| (v * ppp).round();
    let callback = Photo {
        buffer: frame.buffer.clone(),
        origin: [pixels(rect.min.x) as i32, pixels(rect.min.y) as i32],
        size: [frame.width, frame.height],
        shown: [
            pixels(rect.width()).max(1.0) as u32,
            pixels(rect.height()).max(1.0) as u32,
        ],
    };
    painter.add(egui_wgpu::Callback::new_paint_callback(rect, callback));
}

struct Photo {
    buffer: wgpu::Buffer,
    origin: [i32; 2],
    size: [u32; 2],
    shown: [u32; 2],
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
            r.prepare(
                device,
                queue,
                &self.buffer,
                self.origin,
                self.size,
                self.shown,
            );
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
        let view = module
            .types
            .iter()
            .find(|(_, ty)| ty.name.as_deref() == Some("View"))
            .expect("the shader's View");
        let naga::TypeInner::Struct { members, .. } = &view.1.inner else {
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
                ("size", std::mem::offset_of!(View, size) as u32),
                ("shown", std::mem::offset_of!(View, shown) as u32),
                ("decode", std::mem::offset_of!(View, decode) as u32),
            ]
        );
    }

    /// Renders a 5×4 frame at (3, 2) into a 12×10 target as egui's pass would, with the
    /// callback's viewport, and reads every pixel back.
    fn render(format: wgpu::TextureFormat) -> Vec<[u8; 4]> {
        render_shown(format, [5, 4], |x, y| {
            [x as f32 / 8.0, y as f32 / 8.0, 0.5, 1.0]
        })
    }

    /// Renders a 5×4 frame whose pixels `pixel` gives at (3, 2) into a 12×10 target, shown
    /// at `shown`, as egui's pass would, with the callback's viewport, and reads every
    /// pixel back.
    fn render_shown(
        format: wgpu::TextureFormat,
        shown: [u32; 2],
        pixel: impl Fn(u32, u32) -> [f32; 4],
    ) -> Vec<[u8; 4]> {
        let gpu = pollster::block_on(apertrust_engine::gpu::GpuContext::new(&Default::default()))
            .expect("a GPU adapter, hardware or software");
        let (device, queue) = (&gpu.device, &gpu.queue);
        let (width, height) = (5u32, 4u32);
        let pixels: Vec<[f32; 4]> = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| pixel(x, y))
            .collect();
        let frame = wgpu::util::DeviceExt::create_buffer_init(
            device,
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&pixels),
                usage: wgpu::BufferUsages::STORAGE,
            },
        );
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 12,
                height: 10,
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
        resources.prepare(device, queue, &frame, [3, 2], [width, height], shown);
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let view = target.create_view(&Default::default());
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
            pass.set_viewport(3.0, 2.0, shown[0] as f32, shown[1] as f32, 0.0, 1.0);
            resources.draw(&mut pass, &frame);
        }
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 256 * 10,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: None,
                },
            },
            target.size(),
        );
        queue.submit([encoder.finish()]);
        readback.map_async(wgpu::MapMode::Read, .., |result| result.expect("it maps"));
        gpu.wait().expect("the GPU finishes");
        let mapped = readback.get_mapped_range(..).expect("it is mapped");
        mapped
            .chunks(256)
            .flat_map(|row| row[..12 * 4].chunks(4).map(|p| [p[0], p[1], p[2], p[3]]))
            .collect()
    }

    #[test]
    fn a_frame_rendered_for_another_size_fills_the_size_it_is_shown_at() {
        let red = [0.75, 0.25, 0.5, 1.0];
        let target = render_shown(wgpu::TextureFormat::Rgba8Unorm, [8, 7], |_, _| red);
        let unorm = |v: f32| (v * 255.0).round() as u8;
        for y in 0..10 {
            for x in 0..12 {
                let inside = (3..11).contains(&x) && (2..9).contains(&y);
                let expected = if inside {
                    [unorm(red[0]), unorm(red[1]), unorm(red[2]), 255]
                } else {
                    [0, 0, 0, 255]
                };
                assert_eq!(target[y * 12 + x], expected, "({x}, {y})");
            }
        }
    }

    #[test]
    fn a_frame_shown_larger_keeps_its_edges_and_its_order() {
        // A ramp along x, the same down each column, 5 pixels wide shown 9 wide.
        let ramp = |x: u32, _| [x as f32 / 4.0, 0.0, 0.0, 1.0];
        let target = render_shown(wgpu::TextureFormat::Rgba8Unorm, [9, 4], ramp);
        let unorm = |v: f32| (v * 255.0).round() as u8;
        for y in 2..6 {
            let row: Vec<u8> = (3..12).map(|x| target[y * 12 + x][0]).collect();
            // The edges are the frame's own first and last pixels, not leaning inwards.
            assert_eq!(row[0], unorm(0.0), "row {y}: {row:?}");
            assert_eq!(row[8], unorm(1.0), "row {y}: {row:?}");
            assert!(row.windows(2).all(|w| w[0] <= w[1]), "row {y}: {row:?}");
        }
    }

    #[test]
    fn each_window_pixel_shows_its_own_frame_pixel() {
        let target = render(wgpu::TextureFormat::Rgba8Unorm);
        let unorm = |v: f32| (v * 255.0).round() as u8;
        for y in 0..10 {
            for x in 0..12 {
                let inside = (3..8).contains(&x) && (2..6).contains(&y);
                let expected = if inside {
                    let (fx, fy) = (x - 3, y - 2);
                    [
                        unorm(fx as f32 / 8.0),
                        unorm(fy as f32 / 8.0),
                        unorm(0.5),
                        255,
                    ]
                } else {
                    [0, 0, 0, 255]
                };
                assert_eq!(target[y * 12 + x], expected, "({x}, {y})");
            }
        }
    }

    /// The shader decodes, the target encodes again, and the GPU's encoding rounds its
    /// own way: within one 8-bit step. eframe picks a format that does not encode, so
    /// this is the fallback.
    #[test]
    fn an_srgb_window_shows_the_same_values_within_a_step() {
        let srgb = render(wgpu::TextureFormat::Rgba8UnormSrgb);
        let unorm = render(wgpu::TextureFormat::Rgba8Unorm);
        for (i, (a, b)) in srgb.iter().zip(&unorm).enumerate() {
            assert!(
                a.iter().zip(b).all(|(a, b)| a.abs_diff(*b) <= 1),
                "pixel {i}: {a:?} against {b:?}"
            );
        }
    }
}
