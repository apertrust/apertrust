# 0034. Layers: masked module instances in a fixed order

## Context

Local edits need masks. darktable gives each module instance its own mask and lets users add instances and reorder them; layered image editors composite finished images on top of each other. How does Apertrust mask edits without leaving the scene-referred pipeline, and without darktable's free reordering ([0001](0001-what-apertrust-is-the-principles.md))? Layers come in Phase 4, after the migration ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)), so their maths may depart from darktable's where that serves photographers; the masks are ported from darktable's master ([0074](0074-masks-ported-from-darktables-master.md)).

## Decision

**Layers are masked module instances at their pipeline positions,** not compositing on top of the finished image, which would be display-referred. Planned for Phase 4.

**A layer** is a first-class object in the edit and the session protocol: a name, a mask, an opacity, a blend mode, an enabled flag and its module adjustments. The engine expands it into operation instances; users, the interface and MCP see layers, never instances. The pipeline becomes a graph with layers. **Image** is the whole photo's edit, the layer under all others.

**Fixed order, one instance per module:**

- Users get one instance of each module, in the fixed pipeline order ([0022](0022-pipeline-order-and-the-default-pipeline.md)): no reordering, and no extra instances outside layers.
- Extra instances come only through layers, filled layers included, which cover unmasked duplicates such as a second sharpening or two blended looks.
- One instance of each module per layer, keeping one owning module per parameter within each layer.

**Sequencing:** a layer is not a contiguous block. Each of its module adjustments becomes instances at the fixed positions of the operations the module drives. At one position Image runs first, then layers bottom to top as the layers panel shows them, the only place layer order matters. Only adjusted controls create instances.

**How much of a layer applies:** each instance applies by the layer's mask times its opacity, **in each slider's own unit**. At 50%, Exposure and the tone equalizer give half the stops (−2 EV gives −1 EV), a curve goes halfway to straight, and every other change is mixed halfway in linear light, as darktable mixes. Each layer-capable operation declares which of the three it uses. A slider on a layer starts at zero and moves the photo from what Image and the layers under it give.

**Blend modes:** normal, lightness and colour. Lightness and colour carry darktable's two-instance recipe for luminance and colour noise as two filled layers.

**A mask is a list of parts,** Flexi's tree underneath ([0074](0074-masks-ported-from-darktables-master.md)):

- **Parts:** brush with optional edge-aware auto-masking, linear and radial gradients, luminance and colour ranges, AI selections (subject, sky, background, people, a clicked object), **Fill** (the whole photo), and another layer's mask.
- **Combined top to bottom,** each part changing the mask the parts above it made, starting from an empty mask: Add as the maximum, Intersect as the minimum, Subtract as the mask times one minus the part. Parts can be dragged to reorder. Each part can be inverted.
- **A mask with no parts changes nothing.** New Filled Layer and Fill Mask give a layer a Fill part, which other parts can be subtracted from. Deleting a part only ever shrinks where a layer acts.
- **Feather and Edge** refine the layer's whole mask after its parts: a blur and a guided filter.
- **Range parts are read once per layer** on the photo with Image's edits and no layer's, at a reference point, scene-linear after colour calibration. The result serves every instance in the layer, so the mask looks the same whatever modules the layer holds and no layer's mask depends on another layer's edits. Per-module evaluation stays as an advanced option.
- **Shapes** are stored in the original image's coordinates and mapped forward through the geometry ([0016](0016-geometry-for-operations-that-move-pixels.md)).

**AI parts** store their request (the model, the class or the clicked point) and the resulting raster in `.apdata`:

- copied, synced or applied by a style, they are recomputed once per photo, all its AI parts together, with a quiet progress status;
- an edit on this computer that changes what is under one, such as an AI erase in its area, recomputes it within that edit and its undo step, the old raster in use until the new one is ready;
- never by itself on opening, after an update or on another computer: a raster that no longer matches is marked, with Update;
- a brush that fixes an AI part is a part of its own, so a recompute replaces only the AI part.

**Retouch layers,** heal, clone and AI erase, each holding many spots, sit in a band of their own under Image, marked as applied first, since retouch runs before the adjustments. They have opacity, hide, rename and delete as any layer, and move only within the band. darktable's retouch module is ported as these layers in Phase 4, not in the first pass.

**Global modules stay on Image:** the raw stage, lens correction, geometry and the others a module's page marks background only, shown unavailable on a layer with the reason. A tone mapper on a layer replaces Image's within the layer's mask, never stacked on it ([0033](0033-one-tone-mapper-per-edit.md)).

**Colour calibration's adaptation on a layer** is allowed only with a mask, stacked: it runs after Image's inside the mask and corrects what remains, starting at no change, its picker reading the pixels as Image left them. That is darktable's second, masked colour calibration for mixed light.

**Limits:** a notice past the count where editing slows, and a hard limit only where the engine can no longer render within its memory budget, both set by the stress test; brush strokes and AI parts count as well as layers.

**Presets preview on hover;** styles apply as layers with opacity (planned).

## Options considered

- Compositing at the end of the pipe: display-referred, so a brightened face would not be tone-mapped as brighter light.
- darktable's free reordering and multiple instances: flexibility the principles leave out.
- Range masks evaluated per instance: a layer's mask would change with its modules.
- darktable's linear mix for every slider: −2 EV at 50% gives −0.68 EV and +2 EV gives +1.32 EV, so burns and dodges feather unevenly. It could be compared with darktable, which the slider's own unit gives up for exposure and curves.
- Parts combined whatever their order (every Add, then every Intersect and Subtract): a brush could not paint back what a range cut out without a visible group.
- An empty mask covering the whole photo, as darktable's empty Flexi mask does: a slip, or deleting the last part, would turn a local edit global.
- A layer's colour calibration replacing Image's adaptation inside the mask: cleaner colour, but confusing after Image has been adapted.
- Retouch as a module with a spot list, as darktable's, or retouch layers anywhere in the list: a second way of marking areas, or a list whose order misleads, since retouch runs first wherever it sits.
- Fixed hard limits on layers and parts: simpler, but the right numbers depend on the machine, and the principles rule out removing capability to simplify.
- AI parts recomputed mask by mask, or automatically everywhere: repeated prompts, or results that drift between GPUs and model versions.
- darktable's styles as they are: tied to instances and order.

## Consequences

- A layer's first step on an operation the background also runs adds a second instance inside the mask; for colour balance rgb, whose gamut mapping is not idempotent, how a layer treats that step is settled with layers.
- A layer's `base` for bindings, and how a module's defaults and unavailable controls follow from running on Image or on a layer (darktable's per-instance defaults, such as exposure's), are designed with layers.
- darktable's two "sharpen demosaicing" presets of diffuse or sharpen are left out: they need an instance moved before input colour, and Demosaic's capture sharpening does that job there.
- Golden comparisons with darktable cover opacity for the operations that mix linearly, drawn shapes and the refinement; range parts and layer order are checked by Apertrust's own invariants.
- Spikes before Phase 4's interface: the layer graph (15 layers, 100 brush strokes and 5 AI parts at 60 MP on the M4, the RTX 5060 and lavapipe), masks on the GPU (brushes and paths), shapes through the geometry, and AI parts through ONNX Runtime ([0060](0060-ai-inference-with-onnx-runtime.md)).
