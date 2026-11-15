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

//! The raw's histogram (decision 87): how many photosites of the decoded mosaic hold
//! each 16-bit value, over the declared image, counted on the GPU by the engine's own
//! kernel and read back once, for an operation that reads it, exposure's automatic mode.
//! An export, the session and the golden harness count it before they plan, where an
//! instance asks ([`Operation::reads_raw_histogram`]) and the image context holds none,
//! and hand it to `prepare` in the image context ([`ImageContext::raw_histogram`]). No
//! instance asking, nothing is counted, so a photo in manual mode costs nothing.
//!
//! The mosaic is bound in row windows where it is larger than the GPU binds at once
//! (decision 14); every window's dispatch is in one submission, whose counts the call
//! waits for: about a quarter of a mebibyte, once per photo.

use std::num::NonZeroU64;
use std::time::Instant;

use apertrust_plugin::{
    Format, ImageContext, Kernel, Operation, PassHeader, RAW_HISTOGRAM_BINS, RawHistogram,
};

use crate::compute::{ComputeError, GpuImage, ImageKernel, read_back};
use crate::gpu::GpuContext;
use crate::pipeline::Instance;

/// Generated from `slang/raw_histogram.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "raw_histogram",
    wgsl: include_str!("../generated/raw_histogram.wgsl"),
    derivatives: false,
};

/// Photosites each invocation counts, along its row: the kernel's `PER`.
const PER: u32 = 4;
/// The kernel's workgroup, 16 × 16 invocations.
const WORKGROUP: u32 = 16;
/// Bytes of the counts: one 32-bit word per 16-bit value.
const COUNTS_BYTES: u64 = (RAW_HISTOGRAM_BINS * 4) as u64;

/// Whether any of `instances` reads the raw's histogram with its parameters over its
/// defaults for `image`, as the planner resolves them. An instance whose parameters do
/// not resolve reads nothing: the planner refuses it. On an image that is not a raw none
/// does: there is none to count, and the operation renders without it.
pub fn read_by(instances: &[Instance], image: &ImageContext) -> bool {
    if image.camera.is_none() {
        return false;
    }
    instances.iter().any(|instance| {
        let operation: &dyn Operation = instance.operation;
        operation
            .schema()
            .resolve_over(&operation.defaults(image), &instance.params)
            .is_ok_and(|params| operation.reads_raw_histogram(&params))
    })
}

/// The raw's histogram over the declared image, counted from `mosaic` where an instance
/// reads it and `image` holds none yet; `None` otherwise, and on an image that is not a
/// raw.
pub async fn for_instances(
    gpu: &GpuContext,
    instances: &[Instance<'_>],
    image: &ImageContext,
    mosaic: &GpuImage,
) -> Result<Option<RawHistogram>, ComputeError> {
    let Some(camera) = &image.camera else {
        return Ok(None);
    };
    if image.raw_histogram.is_some() || !read_by(instances, image) {
        return Ok(None);
    }
    count(gpu, mosaic, camera.declared).await.map(Some)
}

/// `image` with the raw's histogram, where an instance reads it and it holds none yet:
/// [`for_instances`] put in place.
pub async fn fill(
    gpu: &GpuContext,
    instances: &[Instance<'_>],
    image: &mut ImageContext,
    mosaic: &GpuImage,
) -> Result<(), ComputeError> {
    if let Some(histogram) = for_instances(gpu, instances, image, mosaic).await? {
        image.raw_histogram = Some(histogram);
    }
    Ok(())
}

/// Counts the 16-bit values of `mosaic`, the raw as the engine uploads it, over `region`
/// of its readout (x, y, width, height, held to the readout), on the GPU, and waits for
/// the counts: every photosite whatever its colour, as darktable's deflicker counts them.
pub async fn count(
    gpu: &GpuContext,
    mosaic: &GpuImage,
    region: [u32; 4],
) -> Result<RawHistogram, ComputeError> {
    let mut counting = record(gpu, mosaic, region).await?;
    gpu.submit(counting.commands.take());
    let bytes = read_back(gpu, &counting.readback)?;
    Ok(counting.finish(&bytes))
}

/// A count recorded and not yet submitted: its commands, which copy the counts into
/// `readback` at their end, for a caller that takes them to submit itself or to hand to
/// the thread that may, as an export job does.
pub struct Counting {
    pub commands: Option<wgpu::CommandBuffer>,
    pub readback: wgpu::Buffer,
    /// For its `-d perf` line: the region's size, the windows, when its recording began
    /// and how long compiling took.
    size: [u32; 2],
    windows: usize,
    start: Instant,
    compiled: std::time::Duration,
}

impl Counting {
    /// The histogram of the counts read back from `readback`.
    pub fn finish(self, bytes: &[u8]) -> RawHistogram {
        let histogram = RawHistogram::new(bytemuck::pod_collect_to_vec(bytes))
            .expect("the counts hold one word per 16-bit value");
        let [width, height] = self.size;
        tracing::debug!(
            target: "perf",
            "raw histogram: {} photosites of {width}×{height} counted in {} window{}, {:.1} ms with {:.1} ms compiling",
            histogram.pixels(),
            self.windows,
            if self.windows == 1 { "" } else { "s" },
            self.start.elapsed().as_secs_f64() * 1e3,
            self.compiled.as_secs_f64() * 1e3,
        );
        histogram
    }
}

/// [`count`]'s commands, recorded: compiling the kernel, the counts cleared, each
/// window's dispatch, and the counts' copy into a buffer the CPU maps.
pub async fn record(
    gpu: &GpuContext,
    mosaic: &GpuImage,
    region: [u32; 4],
) -> Result<Counting, ComputeError> {
    let desc = mosaic.desc();
    if desc.format != Format::Mosaic16 {
        return Err(ComputeError::WrongFormat(desc.format));
    }
    // The counts are bound whole.
    let limit = gpu.device.limits().max_storage_buffer_binding_size;
    if COUNTS_BYTES > limit {
        return Err(ComputeError::Binding {
            operation: KERNEL.name,
            pass: "main",
            bytes: COUNTS_BYTES,
            limit,
        });
    }
    let start = Instant::now();
    let kernel = ImageKernel::new(gpu, KERNEL.name, KERNEL.wgsl).await?;
    let compiled = start.elapsed();
    let [x, y, width, height] = held(region, [desc.width, desc.height]);

    let counts = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("raw histogram counts"),
        size: COUNTS_BYTES,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("raw histogram readback"),
        size: COUNTS_BYTES,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.clear_buffer(&counts, 0, None);
    let limits = gpu.device.limits();
    let windows = windows(
        [x, y, width, height],
        desc.width,
        limits.max_storage_buffer_binding_size,
        u64::from(limits.min_storage_buffer_offset_alignment),
        limits.max_compute_workgroups_per_dimension,
    )?;
    // One uniform block per window, each in its own buffer, so the windows' dispatches
    // share the submission.
    let mut uniforms = Vec::with_capacity(windows.len());
    for window in &windows {
        let header = PassHeader {
            width: width.div_ceil(PER),
            height: window.rows,
            origin: [x, window.first_row],
            full_width: desc.width,
            full_height: desc.height,
            input_base: (window.offset / 4) as u32,
            ..PassHeader::whole(0, 0)
        };
        let right = (x + width).to_le_bytes();
        let block = header.uniform_block(&[right, [0; 4], [0; 4], [0; 4]].concat());
        let uniform = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("raw histogram uniforms"),
            size: block.len() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue.write_buffer(&uniform, 0, &block);
        uniforms.push(uniform);
    }
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some(KERNEL.name),
            timestamp_writes: None,
        });
        pass.set_pipeline(kernel.pipeline());
        for (window, uniform) in windows.iter().zip(&uniforms) {
            let Some(size) = NonZeroU64::new(window.size) else {
                continue;
            };
            let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(KERNEL.name),
                layout: &kernel.pipeline().get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: mosaic.buffer(),
                            offset: window.offset,
                            size: Some(size),
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: counts.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: uniform.as_entire_binding(),
                    },
                ],
            });
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(
                width.div_ceil(PER).div_ceil(WORKGROUP),
                window.rows.div_ceil(WORKGROUP),
                1,
            );
        }
    }
    encoder.copy_buffer_to_buffer(&counts, 0, &readback, 0, COUNTS_BYTES);
    Ok(Counting {
        commands: Some(encoder.finish()),
        readback,
        size: [width, height],
        windows: windows.len(),
        start,
        compiled,
    })
}

/// `region` held to an image of `size`.
fn held([x, y, width, height]: [u32; 4], [w, h]: [u32; 2]) -> [u32; 4] {
    let (x, y) = (x.min(w), y.min(h));
    [x, y, width.min(w - x), height.min(h - y)]
}

/// Rows of the region one dispatch counts, and the window of the mosaic's buffer, in
/// bytes, that it binds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Window {
    first_row: u32,
    rows: u32,
    offset: u64,
    size: u64,
}

/// The windows that cover the region's rows of a mosaic `readout` photosites wide, two
/// bytes each, every window at most `binding` bytes and starting at a multiple of
/// `alignment`, and its rows at most the workgroups' `max_groups` on a side: the whole
/// region's rows in one where they fit. A window holds at least a row; where even one
/// does not fit, [`ComputeError::Binding`], and where a row's photosites take more
/// workgroups than a side allows, [`ComputeError::Dispatch`].
fn windows(
    region: [u32; 4],
    readout: u32,
    binding: u64,
    alignment: u64,
    max_groups: u32,
) -> Result<Vec<Window>, ComputeError> {
    let [x, y, width, height] = region;
    let window = |first_row: u32, rows: u32| {
        // The words holding the photosites from the first's to the last's of the rows.
        let first = (u64::from(first_row) * u64::from(readout) + u64::from(x)) / 2;
        let last = (u64::from(first_row + rows - 1) * u64::from(readout)
            + u64::from(x + width).saturating_sub(1))
            / 2;
        let offset = (first * 4) / alignment * alignment;
        Window {
            first_row,
            rows,
            offset,
            size: (last + 1) * 4 - offset,
        }
    };
    if width == 0 || height == 0 {
        return Ok(Vec::new());
    }
    let across = width.div_ceil(PER).div_ceil(WORKGROUP);
    if across > max_groups {
        return Err(ComputeError::Dispatch {
            operation: KERNEL.name,
            pass: "main",
            groups: across,
            limit: max_groups,
        });
    }
    // The most rows a window binds: the row's bytes, with the words of the alignment
    // and of a row's odd start at either end.
    let row_bytes = u64::from(readout) * 2;
    let rows = ((binding.saturating_sub(alignment + 8)) / row_bytes)
        .min(u64::from(max_groups) * u64::from(WORKGROUP))
        .clamp(1, u64::from(height));
    let mut out = Vec::new();
    let mut first_row = y;
    while first_row < y + height {
        let rows = (rows as u32).min(y + height - first_row);
        let window = window(first_row, rows);
        if window.size > binding {
            return Err(ComputeError::Binding {
                operation: KERNEL.name,
                pass: "main",
                bytes: window.size,
                limit: binding,
            });
        }
        out.push(window);
        first_row += rows;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_region_that_fits_is_one_window() {
        let windows = windows([2, 1, 6, 3], 10, u64::MAX, 256, 65535).unwrap();
        // Photosites 12 to 37, words 6 to 18, from the buffer's start.
        assert_eq!(
            windows,
            [Window {
                first_row: 1,
                rows: 3,
                offset: 0,
                size: 19 * 4,
            }]
        );
    }

    #[test]
    fn a_larger_region_is_bound_in_aligned_windows_of_its_rows() {
        // 1000 photosites a row, 2000 bytes: 3 rows in a window of 6400 bytes.
        let windows = windows([4, 10, 990, 8], 1000, 6400, 256, 65535).unwrap();
        let rows: Vec<_> = windows.iter().map(|w| (w.first_row, w.rows)).collect();
        assert_eq!(rows, [(10, 3), (13, 3), (16, 2)]);
        for w in &windows {
            assert_eq!(w.offset % 256, 0);
            assert!(w.size <= 6400, "{w:?}");
            // It holds its first and last photosites.
            let first = (u64::from(w.first_row) * 1000 + 4) * 2;
            let last = (u64::from(w.first_row + w.rows - 1) * 1000 + 993) * 2;
            assert!(w.offset <= first && last + 2 <= w.offset + w.size, "{w:?}");
        }
    }

    #[test]
    fn a_row_larger_than_one_binding_is_refused() {
        // 200001 photosites, 400002 bytes, in a binding of 256 KiB.
        let refused = windows([0, 0, 200_001, 1], 200_001, 1 << 18, 256, 65535);
        assert!(
            matches!(
                refused,
                Err(ComputeError::Binding {
                    operation: "raw_histogram",
                    pass: "main",
                    bytes,
                    limit: 262_144,
                }) if bytes > 262_144
            ),
            "{refused:?}"
        );
    }

    #[test]
    fn windows_keep_within_the_workgroups_a_side_allows() {
        // One photosite a row, 1,048,561 rows: 65,536 workgroups down in one window, over
        // WebGPU's 65,535; in two windows, each at most 65,535 workgroups.
        let tall = windows([0, 0, 1, 1_048_561], 1, u64::MAX, 256, 65_535).unwrap();
        let rows: Vec<u32> = tall.iter().map(|w| w.rows).collect();
        assert_eq!(rows, [65_535 * 16, 1_048_561 - 65_535 * 16]);
        assert!(rows.iter().all(|r| r.div_ceil(WORKGROUP) <= 65_535));
        // A limit of 4 workgroups a side: 64 rows a window, and a row of 4 × 16 × 4
        // photosites at most.
        let small = windows([0, 0, 256, 100], 256, u64::MAX, 256, 4).unwrap();
        let rows: Vec<u32> = small.iter().map(|w| w.rows).collect();
        assert_eq!(rows, [64, 36]);
        let refused = windows([0, 0, 257, 1], 257, u64::MAX, 256, 4);
        assert!(
            matches!(
                refused,
                Err(ComputeError::Dispatch {
                    operation: "raw_histogram",
                    groups: 5,
                    limit: 4,
                    ..
                })
            ),
            "{refused:?}"
        );
    }

    #[test]
    fn a_region_is_held_to_the_readout() {
        assert_eq!(held([8, 6, 10, 10], [10, 8]), [8, 6, 2, 2]);
        assert_eq!(held([12, 0, 4, 4], [10, 8]), [10, 0, 0, 4]);
    }
}
