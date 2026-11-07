# 0021. The operation interface

## Context

What exact shape does an operation take, so that the engine can plan, tile, cache and fuse it without knowing what it computes? The raw stage brings mosaics, crops that change the image's size, demosaic's large intermediates and defaults that come from the file; later operations bring statistics, geometry and the pipe's output profile. Each parameter must keep darktable's meaning, so that a port can be checked against darktable's render.

## Decision

**`apertrust-plugin`'s `Operation` trait:** `manifest`, `schema`, `kernel`, `defaults(image)` and `prepare(params, image) -> Result<Prepared, NotPorted>`, plus provided methods an operation overrides when it needs them: `output_profile` ([0026](0026-colour-profiles-and-colour-management.md)), `tiles_whole_image_passes` and `whole_image_at_full_resolution` ([0015](0015-whole-image-passes.md)), `geometry` ([0016](0016-geometry-for-operations-that-move-pixels.md)), and, planned, `reach` ([0013](0013-tiled-exports-equal-to-untiled-ones.md)) and `readout` ([0019](0019-scopes-and-readouts-counted-on-the-gpu.md)). A provided method with a default keeps existing operations unchanged as the interface grows.

**The slot** is the stage whose data the operation reads (raw, camera RGB, working space, display, output) plus darktable's order number ([0022](0022-pipeline-order-and-the-default-pipeline.md)). The stage fixes the buffer format the engine needs.

**Parameters are data,** named by darktable's field names, each with darktable's range: numbers, whole numbers (`Kind::Int`, an `i64` with a range, parsing only whole numbers), choices and booleans. An array's elements are named with an `_<index>` suffix (`raw_black_level_separate_0`), so a shell needs no brackets.

- A value set from the CLI, a script, MCP or the session protocol outside its range is **refused** (`OutOfRange`, with its `min` and `max`), so a mistake shows and an agent can correct itself.
- A value loaded from a saved edit is **clamped with a warning**, so a photo always opens.

**Defaults** are darktable's scene-referred ones where they differ from its introspection defaults (AgX's, for example), so an unedited photo renders as darktable renders it. `defaults(image)` gives those that come from the file, as darktable's `reload_defaults` does: a raw's crop and levels, the as-shot white balance, demosaic's method by sensor, exposure's +0.7 EV with the camera's bias compensated. Intents such as "as shot" belong to modules, not operation parameters.

**Images:** `prepare` receives the image it gets and returns the one it produces, size and format (the 16-bit mosaic, an f32 mosaic or RGBA f32). Each stage takes only its own formats. The engine gives each operation an output buffer of that image, reusing a working buffer of the same size and format that is not its input. An operation whose output has no pixels is refused at planning (`PlanError::Empty`).

**The image description** (`ImageDesc`) carries darktable's buffer descriptor: size and format, where its top-left pixel sits on the sensor, each channel's saturation level (darktable's processed maximum), the white balance coefficients applied and whether colorin must still correct them to the camera's D65 reference; and its stage's extent, origin and scaled extent ([0015](0015-whole-image-passes.md), [0016](0016-geometry-for-operations-that-move-pixels.md)).

**The image context** (`ImageContext`) carries the camera's metadata ([0024](0024-the-cameras-data-in-the-image-context.md)), the developed image's size (the declared crop), the pipe's kind and scale, a tile's region, the pipe's output profile and, for an operation that asks, the full-resolution image.

**Passes:** each lists exactly what it binds (input, output, uniforms, tables, scratch) at the bindings its kernel declares, by convention 0, 1, 2, 3 to 7 and 10 up; `Pass::per_pixel` keeps the common layout. A pass runs over the operation's output by default, or its own size (`Pass::size`, demosaic's passes over its input). Its kind is per-pixel (fusable), spatial or whole-image. A multi-pass kernel names its passes. A pass may declare row strips ([0014](0014-row-strips-for-images-over-one-binding.md)).

**Scratch buffers** are sized in bytes, chosen by the operation, shared by its passes, and counted by the engine.

**`Kernel::derivatives`** says whether the kernel exports derivative entry points; the raw stage's do not ([0011](0011-a-differentiable-pipeline-and-matching-a-reference-photo.md)).

`apertrust-plugin` holds no colour code and no wgpu.

## Options considered

- Seven anchors in place of stages: demosaic and the tone mapper are operations within a stage.
- Clamping every out-of-range value: hides a caller's mistake.
- darktable's introspection defaults: an unedited photo would not render as darktable's does.
- One fixed format per stage with sizes from the image context: rawprepare's crop and demosaic's change of format are the operations' own business.
- One fixed binding layout for every pass: wgpu refuses a bind group with a binding the entry point does not use, and RCD's nine passes use different subsets.
- Reflecting the WGSL with naga at run time: a parser in the engine for what the operation already knows.
- Chaining passes through working buffers, or buffers owned by operations: RCD keeps nine planes alive across its passes, and the engine could neither budget nor tile them.
- Assuming rawprepare's default crop, or darktable's shared mutable state between operations, for the sensor layout.
- Floats for every parameter: would accept a fractional crop margin.

## Consequences

- The `new-plugin` skill holds the rules the engine enforces.
- Schema migrations come with the first operation whose parameters change.
