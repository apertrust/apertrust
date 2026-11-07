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

//! Runs a compute shader end to end: upload, dispatch, read back. CI runs this on software
//! GPUs (lavapipe, WARP), so it must pass without a hardware adapter.

use apertrust_engine::compute::{ComputeError, GpuImage};
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use wgpu::util::DeviceExt;

const DOUBLE: &str = r"
@group(0) @binding(0) var<storage, read_write> data: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x < arrayLength(&data) {
        data[id.x] = data[id.x] * 2.0;
    }
}
";

#[test]
fn compute_shader_round_trip() {
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default()))
        .expect("a GPU adapter, hardware or software");

    let input: Vec<f32> = (0..1000u16).map(f32::from).collect();
    let bytes: Vec<u8> = input.iter().flat_map(|v| v.to_le_bytes()).collect();
    let size = bytes.len() as u64;

    let storage = gpu
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("data"),
            contents: &bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let module = gpu
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("double"),
            source: wgpu::ShaderSource::Wgsl(DOUBLE.into()),
        });
    let pipeline = gpu
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("double"),
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
    let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("double"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: storage.as_entire_binding(),
        }],
    });

    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups((input.len() as u32).div_ceil(64), 1, 1);
    }
    encoder.copy_buffer_to_buffer(&storage, 0, &readback, 0, size);
    gpu.queue.submit([encoder.finish()]);

    let (sender, receiver) = std::sync::mpsc::channel();
    readback.map_async(wgpu::MapMode::Read, .., move |result| {
        sender.send(result).expect("receiver alive");
    });
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("GPU work completes");
    receiver
        .recv()
        .expect("map callback ran")
        .expect("buffer maps");

    let mapped = readback.get_mapped_range(..).expect("mapped range");
    let (chunks, rest) = mapped.as_chunks::<4>();
    assert!(rest.is_empty());
    let output: Vec<f32> = chunks.iter().map(|b| f32::from_le_bytes(*b)).collect();
    let expected: Vec<f32> = input.iter().map(|v| v * 2.0).collect();
    assert_eq!(output, expected);
}

/// An image a kernel binds whole must fit one binding, and making it says so when it does
/// not; a mosaic the pipeline binds through windows needs only to fit a buffer (decision
/// 14).
#[test]
fn an_image_bound_whole_must_fit_one_binding() {
    let options = GpuOptions {
        max_storage_binding: Some(64 << 10),
        ..GpuOptions::default()
    };
    let gpu =
        pollster::block_on(GpuContext::new(&options)).expect("a GPU adapter, hardware or software");
    // 64 × 64 RGBA float pixels take exactly 64 KiB.
    let pixels = |n: usize| vec![[0.0; 4]; n];
    assert!(GpuImage::upload(&gpu, 64, 64, &pixels(64 * 64)).is_ok());
    assert!(GpuImage::try_new(&gpu, 64, 64).is_ok());
    assert!(matches!(
        GpuImage::upload(&gpu, 65, 64, &pixels(65 * 64)),
        Err(ComputeError::TooLarge { .. })
    ));
    assert!(matches!(
        GpuImage::try_new(&gpu, 65, 64),
        Err(ComputeError::TooLarge { .. })
    ));
    // 256 × 256 16-bit photosites take 128 KiB.
    assert!(GpuImage::upload_mosaic(&gpu, 256, 256, &vec![0; 256 * 256]).is_ok());
}
