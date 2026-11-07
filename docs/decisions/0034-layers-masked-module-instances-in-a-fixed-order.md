# 0034. Layers: masked module instances in a fixed order

## Context

Local edits need masks. darktable gives each module instance its own mask and lets users add instances and reorder them; layered image editors composite finished images on top of each other. How does Apertrust mask edits without leaving the scene-referred pipeline, and without darktable's free reordering ([0001](0001-what-apertrust-is-the-principles.md))?

## Decision

**Layers are masked module instances at their pipeline positions,** not compositing on top of the finished image, which would be display-referred. Planned for Phase 4.

**A layer** is a first-class object in the edit and the session protocol: a name, a mask, an opacity, a blend mode, an enabled flag and its module adjustments. The engine expands it into operation instances; users, the interface and MCP see layers, never instances. The pipeline becomes a graph with layers.

**Fixed order, one instance per module:**

- Users get one instance of each module, in the fixed pipeline order ([0022](0022-pipeline-order-and-the-default-pipeline.md)): no reordering, and no extra instances outside layers.
- Extra instances come only through layers, full-image layers (no mask) included, which cover unmasked duplicates such as a second sharpening or two blended looks.
- One instance of each module per layer, keeping one owning module per parameter within each layer.

**Sequencing:** a layer is not a contiguous block. Each of its module adjustments becomes instances at the fixed positions of the operations the module drives. At one position the background runs first, then layers bottom to top as the layers panel shows them, the only place layer order matters. Each instance blends with its input by the layer's mask times its opacity. Only adjusted controls create instances.

**Range masks are evaluated once per layer** on the background-only image (no layer's edits applied), at a reference point, scene-linear after colour calibration. The result is reused as a raster mask by every instance in the layer, so the mask looks the same whatever modules the layer holds and no layer's mask depends on another layer's edits. Per-module evaluation stays as an advanced option.

**Masks:** brush with optional edge-aware auto-masking, linear and radial gradients, luminance, chroma and hue ranges, AI selections; combined by add, subtract and intersect; refined by feather, guided filter, details threshold, contrast and invert. Shapes are stored in the original image's coordinates and mapped forward through the geometry ([0016](0016-geometry-for-operations-that-move-pixels.md)). AI masks are cached as rasters with their refinement.

**Global modules stay on the background:** the raw stage, lens correction, geometry and the others a module's page marks background only. A tone mapper on a layer replaces the background's within the layer's mask, never stacked on it ([0033](0033-one-tone-mapper-per-edit.md)).

**Presets preview on hover;** styles apply as layers with opacity (planned).

## Options considered

- Compositing at the end of the pipe: display-referred, so a brightened face would not be tone-mapped as brighter light.
- darktable's free reordering and multiple instances: flexibility the principles leave out.
- Range masks evaluated per instance: a layer's mask would change with its modules.
- darktable's styles as they are: tied to instances and order.

## Consequences

- A layer's first step on an operation the background also runs adds a second instance inside the mask; for colour balance rgb, whose gamut mapping is not idempotent, how a layer treats that step is settled with layers.
- A layer's `base` for bindings, and how blend settings reach each instance, are designed with layers.
- Spikes include a stress test of 10 to 15 layers on a 60 MP image.
