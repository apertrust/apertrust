# 0014. Row strips for images over one binding

## Context

A GPU binds only so much of a buffer at once: lavapipe 128 MiB, WebGPU's default as little, against the 244 MB float mosaic of a 61 MP raw and the 976 MB of its demosaiced image. The raw stage runs whole ([0013](0013-tiled-exports-equal-to-untiled-ones.md)), and the raw is uploaded once ([0008](0008-a-gpu-resident-fp32-pipeline-on-wgpu.md)). How does a pass run over an image larger than one binding?

## Decision

**A pass whose image is larger than one binding runs in row strips over windows of whole buffers.** Buffers stay whole (lavapipe and WARP make buffers up to 2 GiB, 536 MP of float mosaic).

- A pass that can (`Pass::strips`: the rows each row of invocations covers, the multiple of rows its strips keep, the rows of halo it reads) runs once per strip, binding the window of each image the strip reads or writes, its offset aligned to `min_storage_buffer_offset_alignment`.
- The planner sizes strips to the binding and splits only the passes whose images do not fit, so a GPU that binds everything plans as before. A plan that would bind more than the GPU allows is refused, naming the operation and the sizes.
- A pass that binds only scratch may declare strips and run in bands of its rows with no window (highlights' dilation).

**The 48-byte pass header** carries `input_base` and `output_base`, the 32-bit word at which each window starts, and two reserved words; its origin carries a strip's first row. The raw stage's kernels and the `region` copy index the whole mosaic at each photosite's place less the bases, so a strip computes what the whole image would.

**Statistics stay whole-image:** a whole-image pass runs in strips too, those writing a partial result per workgroup in strips of whole workgroup rows, each workgroup writing at its index in the whole image, so the statistic is unchanged ([0015](0015-whole-image-passes.md)).

**Exports:** the raw stage runs in strips where it must, and each tile's `region` copy binds only the rows of its output that the tile needs.

**The viewport:** where its full-resolution part needs a binding or buffer the GPU lacks, or demosaic's full-resolution image would exceed the budget, demosaic and the downscale run tile by tile into the viewport image, in one plan (`Plan::tiled`). Tiles fall on the downscale's integer footprints (output pixel o covers source pixels o × source / output up to (o + 1) × source / output) and halve from 8192 until they fit. The raw stage's output and the viewport image are then the cache points. A tile that does not make exactly its part of the image fails planning (`PlanError::TileRegion`); a viewport image larger than one binding is refused (`PlanError::Viewport`).

**Testing on any machine:** `GpuOptions` opens a device that binds less or aligns windows more than the adapter allows, and `apertrust-cli --max-binding MIB` uses it, so any machine renders as lavapipe does.

## Options considered

- Tiling the raw stage from CPU uploads: breaks "upload the raw once", reruns rawprepare and temperature, and loses the raw stage's cache point.
- A buffer split across several bindings: a branch on every access, while RCD already binds 9 storage buffers against WebGPU's default of 8.
- A 16-bit float mosaic: its 10-bit mantissa fails the raw-stage checks, and it needs `shader-f16`.
- Window bases in a block only splittable passes carry: two uniform layouts.
- The viewport's demosaic in strips at full resolution: its planes (26 to 96 bytes a photosite) would stay whole; or per-tile pipelines, as an export's, with a wait per tile.
- Refusing the image.

## Consequences

- Opened at 128 MiB on the M4, a 61 MP export runs the raw stage in two strips a pass and the rest in 20 tiles, byte-identical to the default's; the tiled viewport equals the untiled one bit for bit.
- The integer footprints make the viewport identical on every GPU, where float footprints depended on each GPU's division.
- What still does not fit is a software-GPU limit ([0007](0007-a-software-gpu-instead-of-a-cpu-path.md)).
