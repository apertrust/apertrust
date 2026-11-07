# 0015. Whole-image passes

## Context

Some operations need the whole image before their per-pixel work: highlight reconstruction's statistic (16 bytes), the tone equalizer's mask grid (about 4 MB at 2400×1600, 60 to 135 MB at 61 MP), haze removal's estimate, the blurs of bloom, soften and the colour equalizer, local contrast's pyramids. How do they run on the GPU with no readback, in tiled exports, and in a viewport at another scale, so that a band drag stays one pass and every view shows the export's processing?

## Decision

**One kind of pass for all of them.** A whole-image pass sees the whole image at the pipe's scale before its operation's later passes run, and writes a result of any size into the operation's scratch buffers, which later passes bind; nothing is read back. The code's `PassKind::Reduction` is to be renamed `PassKind::WholeImage`.

- A statistic runs in two dispatches, since WGSL has no float atomics: each workgroup writes a partial result, one workgroup combines them. The combination belongs to the statistic.
- Its scratch is counted in the plan's memory and the budget like any buffer.
- Its uniforms and tables hold only what it reads, never a parameter only its operation's later passes use (the tone equalizer's band curve).
- A grid's buffers are float2 halves, 8 bytes a cell at most; a full-resolution intermediate is scratch laid out as an image, bound in row strips ([0014](0014-row-strips-for-images-over-one-binding.md)).

**Its stage is at the pipe's scale.** The planner gives every RGBA image `ImageDesc::scaled_extent` beside `extent` (full resolution) and `origin`: the stage itself up to the viewport's downscale or an export's resample, the fitted size after the downscale, and after an export's resample its output, by darktable's double floor. `ImageContext::whole_image` returns it in every pipe. It is left out of the keys.

**Lengths follow the scale** from their full-resolution values: darktable's radius at full resolution times the scale; where an integer is needed the scaled radius rounds to the nearest, and a size darktable derives from it keeps darktable's derivation and rounding (the tone equalizer's grid radius `radius < 4 ? 1 : (int)(radius / 4.0f)`). A branch darktable takes on a length scaled to the pipe follows the full-resolution length, so every smaller view does the export's processing at its scale ([0063](0063-departures-from-darktable-and-views-below-full-scale.md)). At scale 1 (an export, its tiles, the 100% view) the result is darktable's exactly.

**The viewport's target:** below scale 1 the result is the same processing within a measured target, against a baseline of the per-pixel gap, every value a pixel reads from other pixels held at the export's and every per-pixel step at the view's scale: the whole-image part's share at most 0.15 mean ΔE2000 at 2400×1600 and at 1/2, 0.4 at 1/4 and 1/8, through a setting only the tests reach.

**Keyed apart:** an operation's whole-image passes are keyed on their input's key in the untiled plan of the same pipe, their operation's id, version and scale, and their own bytes, with no region, tile or origin. A render restarting at the edited operation skips them while the key holds, so a tone equalizer band drag is one per-pixel pass. A viewport rebuild keeps their scratch by key.

**After demosaic, two rounds of export tiles, by opt-in.** An operation opts in through `Operation::tiles_whole_image_passes(params)` when its whole-image passes are one run, the passes that read its input then those that read only scratch, and each input-reading pass writes only the scratch its tile owns. The first round runs each tile up to and through that run, writing its part into scratch the export holds across rounds; the scratch-only passes run once; the second round runs the tiles again through the rest, planned without them. Each run takes a first round at its own scale, before or after the export's resample. An operation that does not opt in runs whole with everything before it.

**Full-resolution estimates in the viewport.** An operation whose estimate is not scale-invariant opts in through `Operation::whole_image_at_full_resolution(params)` (haze removal): in the untiled viewport its whole-image passes read the full-resolution image (`Resource::FullResolution`), keyed on that image at scale 1 so a resize keeps them; the scratch its later passes read per pixel is downscaled by the viewport's box filter, keyed on the viewport's size; its `main` runs at the viewport's scale. Without high quality, an export's resample comes after the last such operation planned.

**Scratch an export holds** is left out of each tile's size: the tile search sizes tiles without it and logs the total under `-d memory` where it exceeds the budget. Every buffer of it must still bind.

## Options considered

- A pass kind of its own for large results: repeats every rule for the same behaviour.
- The export's grid for every view: removes little of the gap and reruns the full-resolution chain on every upstream change.
- A grid at a fixed resolution shared by every pipe: departs from darktable at export.
- darktable's preview branches below scale 1: a different processing from the export's.
- Everything up to a whole-image operation run whole for good: exceeds the budget at 61 MP and lavapipe's binding at 8.4 MP.
- The operation's input assembled whole between the rounds: 16 bytes a pixel, beyond lavapipe's and WARP's largest buffer at 150 MP.
- Haze removal before the downscale, or its gap accepted: a pass at full resolution on every drag, or a viewport far from the export.

## Consequences

- An export with the tone equalizer pays the stretch from demosaic to it twice, about 270 ms at 61 MP on the M4.
- An upstream change reruns haze removal's estimate at full resolution, about 72 ms at 10 MP on the M4; a resize of a 61 MP viewport runs 8 passes, not 31.
- Planned before Phase 3's exit: image-shaped scratch bound in windows on a software GPU, lifting [0007](0007-a-software-gpu-instead-of-a-cpu-path.md)'s refusals of local contrast, bloom, soften, the colour equalizer, filmic rgb and haze removal.
- Planned: operations between demosaic and haze removal reaching its estimate through the 100% view's round at scale 1; the rename; local contrast restructured to tile.
