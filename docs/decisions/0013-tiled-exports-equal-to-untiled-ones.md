# 0013. Tiled exports equal to untiled ones

## Context

A 61 MP export takes more GPU memory than the 2 GB budget and, on small GPUs, buffers larger than the adapter binds. How does an export render in tiles and still give exactly the untiled image? Spatial operations after demosaic read pixels around each output pixel, non-local means about 16 in a typical case and about 4,700 at darktable's soft limits, so a fixed margin per operation cannot serve them.

## Decision

**The raw stage runs whole, the rest in tiles.** The operations up to the last one with a whole-image pass in the raw stage (highlight reconstruction) run once over the whole image, in row strips where needed ([0014](0014-row-strips-for-images-over-one-binding.md)); only their output, the mosaic at 4 bytes a photosite, is kept. Demosaic and every later operation run tile by tile.

**A tile is a pipe** (`Pipe::Tile { region }`): its operations see the tile's region in the image context, and the engine's `region` pass copies the part of the image the tile needs, grown backward through the operations, aligned to the colour pattern so a mosaic keeps its phase, and clipped to the image, so a tile meets the image's real edges where it reaches them. Tiles are regions of the output image, after any geometry and at the export's scale for an export at a size, walked back through the resample ([0056](0056-export-as-an-engine-job.md)).

**Bit-identical:** a tiled export equals the untiled one bit for bit on the same adapter, held by tests on Bayer and X-Trans, with tiles that do not divide the image and a crop at an odd offset. A spatial kernel reads absolute coordinates, treats only the image's edges as edges and computes each output pixel in an order fixed relative to that pixel, never sums slid across a workgroup.

**One pipeline for every tile:** every tile is planned first, the pipeline is built for the largest, and every other tile only rewrites uniforms, tables and dispatch sizes, recreating a bind group where its window moved.

**When and how large:** an export tiles when the whole image would exceed the budget or a buffer the adapter binds, in the largest square tiles, halving from 8192 to 64, whose plans all fit; a last row or column narrower than 32 pixels shares its width with the one before, so demosaic gets the 20 photosites RCD needs. `apertrust-cli export --tile N` forces a size. With none that fits, the export fails with a message.

**Reach from the parameters** (planned): `Operation::reach(params, image)`, the pixels beyond its output region that an operation's output reads, all its passes together, at the pipe's scale, as darktable's `tiling_callback` overlap (`P + K_scattered` for non-local means); by default the manifest's fixed radius times the scale, rounded up (demosaic's 12, highlights' 1). Each operation after demosaic gets its output region from the next one's input region, grown by its reach; demosaic writes the grown region; each `prepare` is told its output region.

**The untiled fallback:** where no tile size fits with the grown regions, the export's split moves past the spatial operations after demosaic until the rest tile, and the operations up to there run whole, as darktable's tiling gives up and runs a module untiled. The result is the same either way; the whole part is logged where it exceeds the budget and refused only where it binds more than the GPU allows. Until the reach is built, an operation after demosaic with a reach above 0 runs whole with everything before it.

Whole-image passes after demosaic take rounds of tiles of their own ([0015](0015-whole-image-passes.md)).

## Options considered

- Tiling the raw stage too: its whole-image statistic needs the whole mosaic, whose buffers are a quarter of RGBA's.
- Kernels reading their part of the whole image in place: a source stride in every kernel, where one copy pass keeps them unchanged.
- Fixed-size tiles shifted inward at the edges: demosaic would meet region edges that are not image edges.
- A pipeline per tile shape: compiles the kernels again for each.
- Per-pass radii from `prepare`: needs regions before `prepare` and overstates a separable filter's reach.
- A fixed manifest radius at the defaults' reach, or refusing the export: wrong or needless for non-local means.

## Consequences

- On the M4 a 61 MP export takes 1.33 s in 6 tiles against 1.68 s whole, with identical files.
- Every spatial port states its reach and keeps its kernel position-independent.
- `crates/engine/tests/tiles.rs` holds the invariant.
