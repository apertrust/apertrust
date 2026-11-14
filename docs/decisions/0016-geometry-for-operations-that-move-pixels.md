# 0016. Geometry for operations that move pixels

## Context

Four operations change an image's geometry, all after demosaic: lens correction (camera RGB 13), rotate and perspective (15), orientation (16) and crop (24.5). How does an operation declare its geometry, how are buffers sized and tiles reached through it, where does it run against the viewport's downscale, how does it interpolate identically on every GPU, and how are points mapped for pickers, the crop frame and layers' masks? darktable answers each with `modify_roi_out`, `modify_roi_in`, `distort_transform`, `distort_backtransform` and `distort_mask`.

## Decision

**An operation that moves pixels returns a `Geometry`** from `Operation::geometry(params, image)`: darktable's region and point maps as pure CPU functions in f32, each a port of its module's, so sizes match darktable's.

- `extent`: the output from the input at full resolution (`modify_roi_out` at scale 1).
- `input_region`: the input region an output region needs at the pipe's scale, the interpolator's margin included (`modify_roi_in`).
- `forward` and `backward` point maps. The kernel evaluates the same map per pixel; a test holds the two together on a grid.

**The planner** finds every stage's extent forward through each geometry, as `dt_dev_pixelpipe_get_dimensions` does, sets the pipe's scale from the output extent, and works regions backward through every pass and geometry to the viewport's downscale or an export tile's copy. Buffers are sized to regions. `ImageDesc` carries its stage's extent and origin, and the pass header's origin and full size are the instance's own, so vignette after a crop sees the cropped image, as darktable's does. A geometry that is a translation of its input region (crop, any geometry at its identity) runs no pass. **An export at a size resamples around a geometry of whole pixels** (`Geometry::whole_pixels`, orientation's turns and mirrors) in the stage its place gives, as darktable's does: without high quality after demosaic in the photo's own stage, its sides the output's taken back through the turns after it, each stage's size at the export's scale the output's taken back exactly; with high quality at finalscale's place in the turned image. A plan at a size with a geometry that interpolates or crops is refused until that geometry's stage sizes at the export's scale are settled.

**In the viewport, geometry runs after the downscale**, at the pipe's scale, as darktable processes its distortion modules at `roi.scale`. A geometry drag re-plans and restarts from demosaic's full-resolution cache point. While Crop or Rotate and perspective has focus the viewport shows the uncropped image, as darktable's does. Spike G1 times a rotation drag on a 61 MP raw on the M4 and WARP.

**No strips:** a warp's input rows vary per strip, and a quarter turn reads every input row. The export's tiles, regions of the output image, reach their inputs back through the geometry, halving until every region fits, bit-identical to untiled ([0013](0013-tiled-exports-equal-to-untiled-ones.md)).

**Interpolation in the kernel, never hardware filtering:** darktable's warp interpolators (bicubic by default, bilinear, lanczos2) in a shared Slang module, fp32 from storage buffers. Hardware filtering's weights differ between GPUs (Vulkan guarantees 4 bits of sub-texel precision, Direct3D 12 requires 8), it gives only bilinear, and WebGPU filters `rgba32float` only with `float32-filterable`.

**Bounds:** darktable's lens correction clamps each position to its input region and mirrors taps at its edges, and rotate and perspective mirrors there too, the region taken from the untiled output's requirement. The geometry interface gives each instance those bounds, the same in every tile and pipe, and a tile's region holds every tap clamped to them (planned with the warps).

**Points:** the engine composes each geometry's maps between the original image, any instance and the frame, as darktable's `dt_dev_distort_transform_plus`. A command answers point queries; each frame carries the output extent, its region and scale, and the focused module's input-to-frame map while the chain after it is affine, so the crop frame follows a drag with no round trip. A picker's area is kept in the original image's coordinates and mapped to its operation's input by its four corners, sorted, the two lowest and the two highest averaged on each axis, as darktable's `dt_color_picker_box` does ([0079](0079-colour-pickers-one-api-for-tools-and-the-colour-picker.md)). Layers' masks are kept in the original image's coordinates and mapped forward, a raster warped by the geometry's own kernel.

**Order of work**, smallest first: the engine's geometry with test-only operations (built); orientation, on by default from the EXIF orientation as darktable's flip is; crop, with no pass; rotate and perspective; lens correction. Each deferral goes to the first card that needs it:

- `prepare` told its output region: with the reach ([0013](0013-tiled-exports-equal-to-untiled-ones.md)).
- A whole-image pass before a geometry that keeps part of its stage (crop's window) is refused until crop's card makes it cover only that window.
- The downscale makes the whole developed image at the scale, and a copy cuts the region, until crop's card makes it write only the region.

## Options considered

- Geometry as a closed list of transforms the engine evaluates: every module's region rules rewritten in the engine.
- A coordinate map computed on the GPU: planning would read it back.
- Warps at full resolution before the downscale: a 61 MP warp on every drag.
- Hardware bilinear sampling, or unfiltered textures unless measured faster with identical results on every backend.
- Strips with windows computed from the geometry: useless for a quarter turn.
- Maps computed in the app: two implementations of each module's geometry.

## Consequences

- `apertrust-plugin` holds `Geometry`, `Extent` and `ImageDesc`'s extent, origin and scaled extent ([0015](0015-whole-image-passes.md)).
- Plans without geometry, and their keys, are unchanged.
- Open: spike G1; geometry at an export's size, through darktable's `modify_roi_out` chain.
