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

//! Running image kernels on the GPU.
//!
//! Kernels follow the interface documented in `apertrust-plugin`: input pixels at
//! binding 0, output pixels at binding 1, a uniform block (a [`PassHeader`], then the
//! operation's parameters) at binding 2, read-only tables at bindings 3 to 7, and a
//! `main` entry point with 16×16 workgroups. A gradient entry point reads dLoss/dOutput
//! at binding 8 and writes parameter gradients at binding 9.

use std::time::Instant;

use apertrust_plugin::{Format, ImageDesc, PassHeader};
use thiserror::Error;
use wgpu::util::DeviceExt;

use crate::gpu::GpuContext;

/// Pixels per workgroup side, matching `@workgroup_size(16, 16)` in every kernel.
pub(crate) const WORKGROUP: u32 = 16;
/// Bindings of a derivative entry point's per-pixel input and output.
pub(crate) const DERIVATIVE_IN_BINDING: u32 = 8;
pub(crate) const DERIVATIVE_OUT_BINDING: u32 = 9;

#[derive(Debug, Error)]
pub enum ComputeError {
    #[error("kernel {label} failed to compile: {message}")]
    Compile { label: String, message: String },
    #[error("{0:?} images cannot be read back this way")]
    WrongFormat(Format),
    #[error("a {width}×{height} image needs {bytes} bytes, more than the GPU allows")]
    TooLarge { width: u32, height: u32, bytes: u64 },
    #[error(
        "{operation} ({pass}) binds {} MiB at once, more than the GPU binds ({} MiB)",
        mebibytes(*bytes),
        mebibytes(*limit)
    )]
    Binding {
        operation: &'static str,
        pass: &'static str,
        bytes: u64,
        limit: u64,
    },
    #[error("{operation} needs a {} MiB buffer, more than the GPU allows ({} MiB)", mebibytes(*bytes), mebibytes(*limit))]
    Buffer {
        operation: &'static str,
        bytes: u64,
        limit: u64,
    },
    #[error("{width}×{height} image needs {expected} pixels, got {actual}")]
    PixelCount {
        width: u32,
        height: u32,
        expected: usize,
        actual: usize,
    },
    #[error("{operation} cannot dispatch a derivative entry point: {reason}")]
    Derivative {
        operation: &'static str,
        reason: &'static str,
    },
    #[error("waiting for the GPU failed")]
    Poll(#[from] wgpu::PollError),
    #[error("reading back from the GPU failed")]
    Map(#[from] wgpu::BufferAsyncError),
}

/// An image in a GPU storage buffer, rows top to bottom: RGBA float32 unless made
/// with another [`Format`]. A clone shares the buffer. The buffer may be larger than
/// the image: a tiled export's buffers are made for its largest tile (decision 13).
#[derive(Debug, Clone)]
pub struct GpuImage {
    buffer: wgpu::Buffer,
    desc: ImageDesc,
}

impl GpuImage {
    const USAGE: wgpu::BufferUsages = wgpu::BufferUsages::STORAGE
        .union(wgpu::BufferUsages::COPY_SRC)
        .union(wgpu::BufferUsages::COPY_DST);

    /// An RGBA image for an [`ImageKernel`], which binds its images whole, or an error
    /// if the GPU cannot bind a buffer that large at once.
    pub fn upload(
        gpu: &GpuContext,
        width: u32,
        height: u32,
        pixels: &[[f32; 4]],
    ) -> Result<Self, ComputeError> {
        let desc = ImageDesc::rgba(width, height);
        check_binding(gpu, &desc)?;
        Self::upload_bytes(gpu, desc, pixels.len(), bytemuck::cast_slice(pixels))
    }

    /// A 16-bit mosaic as decoded, two values to a 32-bit word, or an error if the GPU
    /// cannot make a buffer that large. The pipeline binds it through windows where it
    /// is larger than one binding (decision 14).
    pub fn upload_mosaic(
        gpu: &GpuContext,
        width: u32,
        height: u32,
        values: &[u16],
    ) -> Result<Self, ComputeError> {
        let desc = ImageDesc::new(width, height, Format::Mosaic16);
        let mut bytes: Vec<u8> = bytemuck::cast_slice(values).to_vec();
        bytes.resize(desc.buffer_size() as usize, 0);
        Self::upload_bytes(gpu, desc, values.len(), &bytes)
    }

    fn upload_bytes(
        gpu: &GpuContext,
        desc: ImageDesc,
        count: usize,
        bytes: &[u8],
    ) -> Result<Self, ComputeError> {
        let expected = desc.width as usize * desc.height as usize;
        if count != expected {
            return Err(ComputeError::PixelCount {
                width: desc.width,
                height: desc.height,
                expected,
                actual: count,
            });
        }
        check_size(gpu, &desc)?;
        let buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("image"),
                contents: bytes,
                usage: Self::USAGE,
            });
        Ok(Self { buffer, desc })
    }

    /// An image of the given size and format with undefined contents, for kernel
    /// output, or an error if the GPU cannot make a buffer that large. A pipeline binds
    /// it whole or through windows, as its plan says (decision 14).
    pub fn with_desc(gpu: &GpuContext, desc: ImageDesc) -> Result<Self, ComputeError> {
        check_size(gpu, &desc)?;
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image"),
            size: desc.buffer_size(),
            usage: Self::USAGE,
            mapped_at_creation: false,
        });
        Ok(Self { buffer, desc })
    }

    /// An RGBA image with undefined contents for an [`ImageKernel`], which binds its
    /// images whole, or an error if the GPU cannot bind a buffer that large at once.
    pub fn try_new(gpu: &GpuContext, width: u32, height: u32) -> Result<Self, ComputeError> {
        let desc = ImageDesc::rgba(width, height);
        check_binding(gpu, &desc)?;
        Self::with_desc(gpu, desc)
    }

    /// An RGBA image with undefined contents, for kernel output.
    pub fn new(gpu: &GpuContext, width: u32, height: u32) -> Self {
        let desc = ImageDesc::rgba(width, height);
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image"),
            size: desc.buffer_size(),
            usage: Self::USAGE,
            mapped_at_creation: false,
        });
        Self { buffer, desc }
    }

    pub fn width(&self) -> u32 {
        self.desc.width
    }

    pub fn height(&self) -> u32 {
        self.desc.height
    }

    pub fn desc(&self) -> ImageDesc {
        self.desc
    }

    pub(crate) fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Whether the buffer can hold `desc`'s image.
    pub(crate) fn holds(&self, desc: &ImageDesc) -> bool {
        desc.format == self.desc.format && desc.buffer_size() <= self.buffer.size()
    }

    /// Whether the buffer is the size a pipeline makes for `desc`'s image, in its format:
    /// only then does a cache point's buffer move into a rebuilt pipeline (decision 28).
    pub(crate) fn sized_for(&self, desc: &ImageDesc) -> bool {
        desc.format == self.desc.format && desc.buffer_size() == self.buffer.size()
    }

    /// Takes `desc` as the image the buffer now holds, which must fit it.
    pub(crate) fn set_desc(&mut self, desc: ImageDesc) {
        assert!(self.holds(&desc), "{desc} does not fit the buffer");
        self.desc = desc;
    }

    /// The pixels of an RGBA image.
    pub fn download(&self, gpu: &GpuContext) -> Result<Vec<[f32; 4]>, ComputeError> {
        if self.desc.format != Format::Rgba {
            return Err(ComputeError::WrongFormat(self.desc.format));
        }
        let bytes = read_buffer(gpu, &self.buffer, self.desc.buffer_size())?;
        Ok(bytemuck::pod_collect_to_vec(&bytes))
    }

    /// The values of a one-channel f32 mosaic.
    pub fn download_mosaic(&self, gpu: &GpuContext) -> Result<Vec<f32>, ComputeError> {
        if self.desc.format != Format::Mosaic {
            return Err(ComputeError::WrongFormat(self.desc.format));
        }
        let bytes = read_buffer(gpu, &self.buffer, self.desc.buffer_size())?;
        Ok(bytemuck::pod_collect_to_vec(&bytes))
    }
}

/// Mebibytes to one decimal, for errors.
fn mebibytes(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / (1 << 20) as f64)
}

/// Fails if the GPU cannot bind a buffer for this image whole, as an [`ImageKernel`]
/// binds it.
fn check_binding(gpu: &GpuContext, desc: &ImageDesc) -> Result<(), ComputeError> {
    let bytes = desc.buffer_size();
    if bytes > gpu.device.limits().max_storage_buffer_binding_size {
        return Err(ComputeError::TooLarge {
            width: desc.width,
            height: desc.height,
            bytes,
        });
    }
    Ok(())
}

/// Fails if the GPU cannot make a buffer for this image. Whether a kernel can bind all
/// of it at once is the plan's to check: a pass may bind a window of it (decision 14).
fn check_size(gpu: &GpuContext, desc: &ImageDesc) -> Result<(), ComputeError> {
    let bytes = desc.buffer_size();
    if bytes > gpu.device.limits().max_buffer_size {
        return Err(ComputeError::TooLarge {
            width: desc.width,
            height: desc.height,
            bytes,
        });
    }
    Ok(())
}

/// Copies `size` bytes of `buffer` to the CPU, waiting for the GPU.
pub(crate) fn read_buffer(
    gpu: &GpuContext,
    buffer: &wgpu::Buffer,
    size: u64,
) -> Result<Vec<u8>, ComputeError> {
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_buffer_to_buffer(buffer, 0, &readback, 0, size);
    gpu.submit([encoder.finish()]);
    read_back(gpu, &readback)
}

/// The contents of `readback`, a buffer the CPU can map, once everything submitted so far
/// has finished, waiting for the GPU.
pub(crate) fn read_back(
    gpu: &GpuContext,
    readback: &wgpu::Buffer,
) -> Result<Vec<u8>, ComputeError> {
    let (sender, receiver) = std::sync::mpsc::channel();
    readback.map_async(wgpu::MapMode::Read, .., move |result| {
        // The receiver outlives the poll below, so a failed send cannot happen.
        let _ = sender.send(result);
    });
    gpu.wait()?;
    receiver
        .recv()
        .expect("map_async calls back before poll returns")?;
    let mapped = readback
        .get_mapped_range(..)
        .expect("buffer was just mapped");
    Ok(mapped.to_vec())
}

/// A compiled kernel entry point: `main`, or a Slang kernel's gradient entry point. It
/// binds its images whole, so make them with [`GpuImage::upload`] or
/// [`GpuImage::try_new`], which refuse an image larger than the GPU binds at once.
#[derive(Debug, Clone)]
pub struct ImageKernel {
    pipeline: wgpu::ComputePipeline,
}

impl ImageKernel {
    /// The compiled pipeline, for callers that bind their own resources.
    pub(crate) fn pipeline(&self) -> &wgpu::ComputePipeline {
        &self.pipeline
    }

    pub async fn new(gpu: &GpuContext, label: &str, wgsl: &str) -> Result<Self, ComputeError> {
        Self::with_entry_point(gpu, label, wgsl, "main").await
    }

    pub async fn with_entry_point(
        gpu: &GpuContext,
        label: &str,
        wgsl: &str,
        entry_point: &str,
    ) -> Result<Self, ComputeError> {
        let scope = gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(wgsl.into()),
            });
        let pipeline = gpu
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: None,
                module: &module,
                entry_point: Some(entry_point),
                // Every kernel writes its workgroup memory before reading it, so wgpu's
                // zeroing of it is left out. On Direct3D 12 it made FXC take seconds over
                // each reduction's shared arrays, and on WARP it ran at every dispatch too.
                compilation_options: wgpu::PipelineCompilationOptions {
                    zero_initialize_workgroup_memory: false,
                    ..Default::default()
                },
                cache: None,
            });
        if let Some(error) = scope.pop().await {
            return Err(ComputeError::Compile {
                label: label.to_owned(),
                message: error.to_string(),
            });
        }
        Ok(Self { pipeline })
    }

    /// Queues one run of the kernel from `input` to `output`, over the whole image.
    /// `params` are the operation's parameters, after the pass header this prepends; `tables` are bound read-only from
    /// binding 3 on.
    pub fn dispatch(
        &self,
        gpu: &GpuContext,
        input: &GpuImage,
        output: &GpuImage,
        params: &[u8],
        tables: &[&[u8]],
    ) {
        self.dispatch_repeated(gpu, input, output, params, tables, 1, None);
    }

    /// Like [`Self::dispatch`], but the kernel applies its per-pixel function `repeat`
    /// times, and the GPU time is recorded in `timer` slot `slot` if given. Only for
    /// benchmarks: repeats turn a memory-bound kernel into an arithmetic-bound one.
    #[allow(clippy::too_many_arguments)]
    pub fn dispatch_repeated(
        &self,
        gpu: &GpuContext,
        input: &GpuImage,
        output: &GpuImage,
        params: &[u8],
        tables: &[&[u8]],
        repeat: u32,
        timer: Option<(&GpuTimer, u32)>,
    ) {
        let header = PassHeader {
            repeat: repeat.max(1),
            ..PassHeader::whole(input.width(), input.height())
        };
        let bindings = [(0, &input.buffer), (1, &output.buffer)];
        self.run(gpu, &header, &bindings, params, tables, timer);
    }

    /// Like [`Self::dispatch`], with `header` instead of the whole image's at scale 1, so
    /// a test can run the kernel as a tile or a pipe at another scale would; one
    /// invocation per pixel of `header`'s width and height. The GPU time is recorded in
    /// `timer` slot `slot` if given.
    #[allow(clippy::too_many_arguments)]
    pub fn dispatch_with(
        &self,
        gpu: &GpuContext,
        header: &PassHeader,
        input: &GpuImage,
        output: &GpuImage,
        params: &[u8],
        tables: &[&[u8]],
        timer: Option<(&GpuTimer, u32)>,
    ) {
        let bindings = [(0, &input.buffer), (1, &output.buffer)];
        self.run(gpu, header, &bindings, params, tables, timer);
    }

    /// Queues one run of a derivative entry point, which reads `derivative_in` and writes
    /// `derivative_out` per pixel. In reverse mode they are dLoss/dOutput and the
    /// parameter gradients; in forward mode, a parameter direction and the output's
    /// directional derivative.
    pub fn dispatch_derivative(
        &self,
        gpu: &GpuContext,
        input: &GpuImage,
        derivative_in: &GpuImage,
        derivative_out: &GpuImage,
        params: &[u8],
        tables: &[&[u8]],
    ) {
        let header = PassHeader::whole(input.width(), input.height());
        self.dispatch_derivative_with(
            gpu,
            &header,
            input,
            derivative_in,
            derivative_out,
            params,
            tables,
        );
    }

    /// Like [`Self::dispatch_derivative`], with `header` instead of the whole image's at
    /// scale 1, as [`Self::dispatch_with`].
    #[allow(clippy::too_many_arguments)]
    pub fn dispatch_derivative_with(
        &self,
        gpu: &GpuContext,
        header: &PassHeader,
        input: &GpuImage,
        derivative_in: &GpuImage,
        derivative_out: &GpuImage,
        params: &[u8],
        tables: &[&[u8]],
    ) {
        let bindings = [
            (0, &input.buffer),
            (DERIVATIVE_IN_BINDING, &derivative_in.buffer),
            (DERIVATIVE_OUT_BINDING, &derivative_out.buffer),
        ];
        self.run(gpu, header, &bindings, params, tables, None);
    }

    fn run(
        &self,
        gpu: &GpuContext,
        header: &PassHeader,
        images: &[(u32, &wgpu::Buffer)],
        params: &[u8],
        tables: &[&[u8]],
        timer: Option<(&GpuTimer, u32)>,
    ) {
        let uniform = header.uniform_block(params);
        let uniform = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("params"),
                contents: &uniform,
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let tables: Vec<wgpu::Buffer> = tables
            .iter()
            .map(|contents| {
                gpu.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("table"),
                        contents,
                        usage: wgpu::BufferUsages::STORAGE,
                    })
            })
            .collect();

        let mut entries: Vec<wgpu::BindGroupEntry> = images
            .iter()
            .map(|&(binding, buffer)| wgpu::BindGroupEntry {
                binding,
                resource: buffer.as_entire_binding(),
            })
            .collect();
        entries.push(wgpu::BindGroupEntry {
            binding: 2,
            resource: uniform.as_entire_binding(),
        });
        entries.extend(
            (3..)
                .zip(&tables)
                .map(|(binding, table)| wgpu::BindGroupEntry {
                    binding,
                    resource: table.as_entire_binding(),
                }),
        );
        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &entries,
        });

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: timer.map(|(timer, slot)| timer.writes(slot)),
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(
                header.width.div_ceil(WORKGROUP),
                header.height.div_ceil(WORKGROUP),
                1,
            );
        }
        gpu.submit([encoder.finish()]);
    }
}

/// GPU time of each pass, in milliseconds, and the wall-clock time from submitting
/// the passes to the GPU finishing them.
#[derive(Debug, Clone, Default)]
pub struct Timings {
    pub passes: Vec<(&'static str, f64)>,
    pub wall_ms: f64,
}

impl Timings {
    /// The GPU times of passes recorded in timer slots `0..names.len()`, whose ticks
    /// `ticks` holds, copied out by [`GpuTimer::copy_out`] in the passes' submission, and
    /// the time since `start`; NaN where the device has no timestamp queries.
    pub(crate) fn read(
        gpu: &GpuContext,
        ticks: Option<(&GpuTimer, &wgpu::Buffer)>,
        names: &[&'static str],
        start: Instant,
    ) -> Result<Self, ComputeError> {
        let wall_ms = start.elapsed().as_secs_f64() * 1e3;
        let times = match ticks {
            Some((timer, copied)) => timer.read_copied(gpu, copied)?,
            None => vec![f64::NAN; names.len()],
        };
        Ok(Self {
            passes: names.iter().copied().zip(times).collect(),
            wall_ms,
        })
    }
}

/// GPU timestamps around kernel dispatches, for benchmarks: each slot records one
/// dispatch's start and end on the GPU's own clock, free of CPU and submission time.
#[derive(Debug)]
pub struct GpuTimer {
    queries: wgpu::QuerySet,
    resolved: wgpu::Buffer,
    slots: u32,
    period_ns: f64,
}

impl GpuTimer {
    /// Room for `slots` dispatches, at most 2048. `None` when the device lacks
    /// timestamp queries; `GpuContext` requests them where the adapter has them.
    pub fn new(gpu: &GpuContext, slots: u32) -> Option<Self> {
        if !gpu
            .device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY)
        {
            return None;
        }
        let slots = slots.clamp(1, wgpu::QUERY_SET_MAX_QUERIES / 2);
        let queries = gpu.device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: slots * 2,
        });
        let resolved = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("timestamps"),
            size: u64::from(slots) * 16,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        Some(Self {
            queries,
            resolved,
            slots,
            period_ns: f64::from(gpu.queue.get_timestamp_period()),
        })
    }

    pub fn slots(&self) -> u32 {
        self.slots
    }

    pub(crate) fn writes(&self, slot: u32) -> wgpu::ComputePassTimestampWrites<'_> {
        wgpu::ComputePassTimestampWrites {
            query_set: &self.queries,
            beginning_of_pass_write_index: Some(slot * 2),
            end_of_pass_write_index: Some(slot * 2 + 1),
        }
    }

    /// GPU time of the dispatches recorded in slots `0..used`, in milliseconds.
    pub fn read(&self, gpu: &GpuContext, used: u32) -> Result<Vec<f64>, ComputeError> {
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let copied = self.copy_out(gpu, &mut encoder, used);
        gpu.submit([encoder.finish()]);
        self.read_copied(gpu, &copied)
    }

    /// Records resolving slots `0..used` and copying their ticks into a buffer the CPU
    /// can map, and returns it for [`Self::read_copied`]: [`Self::read`] for a caller that
    /// records it in a submission of its own, after the dispatches.
    pub(crate) fn copy_out(
        &self,
        gpu: &GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        used: u32,
    ) -> wgpu::Buffer {
        let size = u64::from(used.min(self.slots)) * 16;
        let copied = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("timestamps readback"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let resolved = self.resolve(encoder, used);
        encoder.copy_buffer_to_buffer(resolved, 0, &copied, 0, size);
        copied
    }

    /// GPU time of each slot whose ticks `copied`, from [`Self::copy_out`], holds once
    /// its submission has run, in milliseconds, waiting for the GPU.
    pub(crate) fn read_copied(
        &self,
        gpu: &GpuContext,
        copied: &wgpu::Buffer,
    ) -> Result<Vec<f64>, ComputeError> {
        let bytes = read_back(gpu, copied)?;
        let ticks: Vec<u64> = bytemuck::pod_collect_to_vec(&bytes);
        let (pairs, _) = ticks.as_chunks::<2>();
        Ok(pairs
            .iter()
            .map(|&[begin, end]| self.milliseconds(begin, end))
            .collect())
    }

    /// Records resolving slots `0..used` into [`Self::resolved`], and returns it.
    pub(crate) fn resolve(&self, encoder: &mut wgpu::CommandEncoder, used: u32) -> &wgpu::Buffer {
        encoder.resolve_query_set(
            &self.queries,
            0..used.min(self.slots) * 2,
            &self.resolved,
            0,
        );
        &self.resolved
    }

    /// The buffer slots are resolved into, 16 bytes a slot: its start and end ticks, for a
    /// caller that copies them out itself rather than reading them back at once, as the
    /// scopes do.
    pub(crate) fn resolved(&self) -> &wgpu::Buffer {
        &self.resolved
    }

    /// The milliseconds between a slot's start and end ticks.
    pub(crate) fn milliseconds(&self, begin: u64, end: u64) -> f64 {
        end.saturating_sub(begin) as f64 * self.period_ns / 1e6
    }
}

/// How long the engine thread sleeps between looks at the GPU while it waits.
///
/// wgpu holds the device's lock for as long as a blocking wait lasts, and a window's
/// present needs that lock, so one long wait would stop the window presenting for a whole
/// render. A wait with a timeout is no way out: when it times out while another thread
/// polls the same device, wgpu-core 30 can trip its own assertion that the queue is empty
/// below the waited-for index. So the engine never waits on the GPU: it polls without
/// waiting, which takes the lock only briefly, and sleeps between polls without it
/// (decision 17).
pub(crate) const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_micros(250);

impl GpuContext {
    /// Blocks until all GPU work submitted so far has finished, never keeping another
    /// thread from presenting for more than a moment.
    pub fn wait(&self) -> Result<(), ComputeError> {
        // Fires once everything submitted before it has finished, whatever other threads
        // submit meanwhile.
        let (done, finished) = std::sync::mpsc::channel();
        self.queue.on_submitted_work_done(move || {
            // The receiver outlives the loop below, so a failed send cannot happen.
            let _ = done.send(());
        });
        loop {
            self.device.poll(wgpu::PollType::Poll)?;
            if finished.try_recv().is_ok() {
                return Ok(());
            }
            std::thread::sleep(POLL_INTERVAL);
        }
    }
}
