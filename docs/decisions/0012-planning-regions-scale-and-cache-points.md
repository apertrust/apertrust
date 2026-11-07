# 0012. Planning: regions, scale and cache points

## Context

How does the engine turn an edit into GPU work, for a full-size export and for a viewport a few megapixels wide, so that the viewport shows what the export will be and a slider drag costs a pass or two? A full-resolution RGBA buffer at 60 MP is about 1 GB, so the engine cannot keep every pass's output as darktable can.

## Decision

**A plan is made for a pipe**, an export (or its tile) or the viewport. Planning is a pure function of the edit's structure, the image context, the pipe and the GPU's limits (`pipeline::Limits`: the largest binding and buffer, the offset alignment, the budget), and returns plain data, so it is tested without a GPU.

**Regions propagate backwards** at planning, as darktable's `modify_roi_in` does: a per-pixel pass keeps the region, a spatial pass grows it by its reach ([0013](0013-tiled-exports-equal-to-untiled-ones.md)), a whole-image pass covers the whole image at the pipe's scale ([0015](0015-whole-image-passes.md)), a geometry maps it ([0016](0016-geometry-for-operations-that-move-pixels.md)). Raw-stage regions align to the colour pattern: 2×2 for Bayer, 6×6 for X-Trans. Export tiles need regions, and the 100% view will.

**The viewport is an exact downscale of the export.** The raw stage and demosaic run at full resolution; the planner inserts the engine's own downscale, a box filter on integer footprints, where they end, and every later operation is prepared at the viewport's size with the image context's scale. Demosaic is never run at a reduced size.

**Cache points**, a few:

| Cache point | At 60 MP | Invalidated by |
| --- | --- | --- |
| The uploaded mosaic | 120 MB | A new image |
| The demosaiced image, full resolution | About 1 GB | Raw-stage parameters |
| Its viewport version | About 60 MB at 2400×1600 | The above, or a new viewport size |
| The edited operation's input, at viewport size | About 60 MB | Anything upstream of it |
| Whole-image results, in their operation's scratch | Bytes to megabytes | Their input or their own passes ([0015](0015-whole-image-passes.md)) |

**Keys** chain each instance's operation id, version, scale, region, output image and the bytes `prepare` produced (uniforms and tables) onto the previous key, never the parameters, so a change of image context invalidates what it should and parameter sets with the same effect share an entry. A render restarts after the newest cache point whose key still matches, or at the edited operation when its copy does.

**An operation with nothing to do leaves the plan.** An operation after the raw stage that returns no geometry, runs no pass and returns its input unchanged as its output is not planned, as darktable's `commit_params` disables a piece with nothing to do: orientation at no rotation, which every photo the right way up has. So such a photo plans exactly as it would without the operation, its plan's fingerprints and cache keys unchanged, and nothing is allocated for an output no pass writes.

**A change re-plans on the CPU.** A plan of the same structure (passes, bindings, buffers and sizes) only rewrites the uniforms and tables that changed. Any other change rebuilds over the same input, keeping every compiled kernel and every cache point whose key and size still match, and moving buffers of the same size into the new plan for their memory; an operation joining after the downscale costs its compilation and one render from there.

**The pass header is 48 bytes:** the pass's size and origin, the full size and the scale, and the window bases of row strips ([0014](0014-row-strips-for-images-over-one-binding.md)), so position-dependent maths is the same in a tile, a region or the whole image.

## Options considered

- Every operation over the whole image at the pipe's scale, without regions: the 100% view of a 60 MP image would cost about 27 ms a pass.
- Demosaic at reduced size for the viewport, as darktable's preview pipe does: the viewport would no longer show the export.
- Keeping every operation's output at viewport size: grows with the pipeline and with layers (20 operations at 2400×1600 take 1.2 GB).
- Keys hashing the parameters: miss image-context changes.
- An update path apart from planning: a second description of the pipeline to keep consistent.

## Consequences

- An exposure change at 2400×1600 costs about 5.5 ms of GPU time on the M4, at any raw size; a render restarting from any cache point equals a full render bit for bit.
- A White balance drag restarts at the mosaic and reruns demosaic ([0027](0027-both-of-darktables-white-balances.md)).
- With several photos (planned), kernels live for the session and one budget covers the live photo and the stills ([0037](0037-one-view-the-library-as-a-tab.md)).
