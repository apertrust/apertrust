# 0008. A GPU-resident fp32 pipeline on wgpu

## Context

How does the engine process a photo? A slider must move the picture on a 60 MP raw and on a modest laptop, the same image must come out on every GPU and platform, and the engine must also run where only WebGPU is available. Measured on the M4, an RTX 5060 and a Radeon 780M, simple operations are memory-bound: a full-image pass costs about 1.3 ms at viewport size and 27 ms at 60 MP, whatever it computes.

## Decision

**wgpu is the GPU API**, over Vulkan, Metal, DX12 and WebGPU, from one Rust codebase.

**The pipeline is GPU-resident.** The raw is uploaded once and every intermediate stays on the GPU, with no host round trip between operations. Readback happens only for an export's finished tiles, for scopes and for readouts ([0019](0019-scopes-and-readouts-counted-on-the-gpu.md)).

**Storage buffers by default.** Textures only where hardware sampling helps; geometry interpolates in its kernel with darktable's interpolators, never by hardware filtering, whose precision differs between GPUs ([0016](0016-geometry-for-operations-that-move-pixels.md)).

**Kernel rules:**

- fp32 only. Maths that darktable does in fp64 is redesigned, not translated.
- No reliance on NaN or infinity checks, which WGSL may assume never occur. Each operation sanitises its input at its boundary, inside its own kernel.
- A kernel separates once-per-image setup, computed on the CPU in the operation's `prepare`, from a pure per-pixel function.
- No write to a local vector component or array element at a run-time index ([0010](0010-fxc-on-direct3d-12-and-no-writes-at-run-time-indices.md)).
- Kernels stay differentiable-friendly ([0011](0011-a-differentiable-pipeline-and-matching-a-reference-photo.md)).

**Memory bandwidth is the cost**, so:

- Runs of per-pixel operations are fused into one kernel (Phase 2). Spatial and whole-image passes end a run. Fusion's mechanism is open, settled by a spike, with kernels compiled ahead of time in Slang favoured for fused derivatives.
- Editing renders at viewport resolution; the export runs at full resolution ([0012](0012-planning-regions-scale-and-cache-points.md)).
- The cache sits at the input of the operation being edited.
- Integrated GPUs set the performance target.

**Pixels are RGBA `f32`** until packed 12-byte RGB is measured against it, before fusion fixes the layout.

**The cache budget is a setting**, `engine.memory_budget`, since wgpu reports no memory size: 2 GB by default beyond the working buffers, 256 to 65536 MiB.

**Every submission goes through `GpuContext::submit`**, never `queue.submit` ([0018](0018-one-submit-gate-for-the-engine-and-the-window.md)).

## Options considered

- A hand-written Vulkan and Metal host layer: two backends to maintain and no WebGPU.
- Host copies between operations, as a CPU-first design has: the copies cost more than most operations.
- One full-image pass per operation instance for good: memory traffic that fusion removes.
- A native Metal variant of the kernels: the spike showed no speed reason to keep it.

## Consequences

- Benchmarks warm up for a fixed time, alternate the variants they compare and use timestamp queries.
- Without a hardware GPU the same kernels run on a software GPU ([0007](0007-a-software-gpu-instead-of-a-cpu-path.md)).
- Open: fusion's mechanism; packed RGB; the budget's default, tuned on more machines.
