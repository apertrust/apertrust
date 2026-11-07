# Layers

How local edits work: layers, each a mask over module adjustments, and where in the pipeline each kind of operation sits. The pipeline that expands layers into instances is [engine.md](engine.md); the modules a layer holds are [modules/README.md](modules/README.md)'s.

**Status.** Planned, Phase 4 (decision 34). Until then every module acts on the background, and the plan is a linear chain ([engine.md](engine.md#the-default-pipeline)).

## What a layer is

Masking uses layers, and a layer is masked operation instances at their normal pipeline positions, not compositing on top of the finished image (decision 34, Phase 4). That keeps darktable's processing quality; end-of-pipe compositing would be display-referred.

- **A first-class object** in the edit format and the session protocol: name, mask, opacity, blend mode, enabled flag, module adjustments. The engine expands it into instances; users, the UI and MCP see layers.
- **Order** matters only where two layers adjust the same operation. The layers panel shows this, rather than implying a stack.
- **Opacity and blend mode** become each instance's blend settings. Blend modes are limited to normal plus a small, well-defined set.
- **Mask types,** combinable with add, subtract and intersect: brush with optional edge-aware auto-masking, linear and radial gradients, luminance, chroma and hue range masks, and AI selections (subject, sky, background, people, click-to-select object).
- **Refinement:** feather, guided-filter refinement, details threshold, contrast, invert, red overlay view.
- **Range masks are evaluated once per layer** on the background-only image (no layer edits applied) at a reference point, scene-linear after colour calibration. The tone equalizer, at darktable's camera RGB 24.0 (decision 22), comes before that point, as exposure does. The result is reused as a raster mask by every instance in the layer, so the mask looks the same whatever modules the layer holds, and no layer's mask depends on another layer's edits. Operations before the reference point run once without layers to produce it; that result rarely changes while editing layers and caches well. Per-module evaluation stays as an advanced option.
- **Geometry:** mask shapes are stored in original image coordinates and transformed through lens correction, perspective, orientation and crop by their forward point maps, a raster mask warped by the geometry's own kernel (decision 16).
- **AI masks are cached** as rasters with their refinement settings, recomputed only when an upstream change affects them.
- **Global modules** (profile, lens, geometry, raw, grain, black and white) stay on the background layer, as in commercial layer-based editors.
- **Performance:** a stress test of 10 to 15 layers on a 60 MP image.

## Instances and order

Users get one instance of each module, in a fixed pipeline order: no reordering, and no extra instances outside layers. The engine still creates operation instances internally, one per adjusted operation per layer.

- **Full-image layers** (no mask, a filled layer) cover unmasked duplicates such as a second sharpening pass or two blended colour looks, with opacity as a bonus.
- **One instance of each module per layer,** keeping one owning module per parameter within each layer; the operation instances a module adds are its own.

## Sequencing

A layer is not a contiguous block: each of its module adjustments becomes instances at the fixed positions of the operations the module drives, interleaved with the background and other layers.

1. Pipeline order first. A module that drives several operations produces instances at several positions from one layer.
2. At the same position: background first, then layers bottom to top in the layers panel. This is the only place layer order matters.
3. Each instance blends with its input using the layer's mask times its opacity. One mask raster serves all of the layer's instances; geometry corrections come before any layer instance, so the raster stays aligned.
4. Only adjusted controls create instances: a layer where only Exposure's exposure slider was touched adds one instance.

Rule 4 holds for an operation the background runs too. Colour balance rgb's first step on a layer adds it there, bringing its gamut mapping inside the mask. Where the background also runs it, that mapping, which is not idempotent, moves colours the background's instance already pulled in, as a second instance does in darktable. How a layer treats that first step is settled with layers (decision 32), as a layer's white balance is (decision 27).

### Example

The background has exposure, colour balance rgb and RGB curve; layer A (a face, brushed) has exposure +0.7 and colour balance rgb; layer B (the sky, an AI mask) has exposure −0.5 and RGB curve. RGB curve sits at darktable's 42, after colour balance rgb and before the tone mapper (decision 22). The pipeline runs:

```mermaid
flowchart LR
    rawstage["Raw processing,<br/>lens correction"] --> e0["Exposure<br/>background"] --> ea["Exposure<br/>layer A"] --> eb["Exposure<br/>layer B"] --> cc["Colour calibration"] --> b0["Colour balance rgb<br/>background"] --> ba["Colour balance rgb<br/>layer A"] --> c0["RGB curve<br/>background"] --> cb["RGB curve<br/>layer B"] --> tm["Tone mapper<br/>background only"] --> out["Output"]
```

Layer A's brighter face flows into everything downstream, the tone mapper included, so it is tone-mapped as genuinely brighter light rather than lightened on top. The background tone mapper's parameters are global; neither layer chooses a tone mapper of its own (decision 33).

## Scene and display domains

The pipeline's core stays scene-referred, but editing is not purist: an operation goes before the tone mapper if it models light or needs values above display white, and after it if its job is how the final image looks. Every operation keeps darktable's place, so this is darktable's split (decision 22); engine.md's [Order](engine.md#order) table has every operation.

- **Scene-referred, before the tone mapper:** raw processing, white balance and colour calibration, exposure, the tone equalizer, colour balance rgb (contrast, colour grading), RGB curve and RGB levels, as darktable has them, denoise, sharpening, blur, lens corrections, and the tone mapper itself, whose parameters are global: layers change the light it receives, and a layer that chooses its own tone mapper replaces the background's within its mask, never stacked on it (decision 33, planned with layers). Every module a layer can use acts at its darktable place, before the tone mapper, apart from darktable's display-referred effects.
- **Display-referred, after the tone mapper:** darktable's effects such as grain and vignette. These shape the look, on layers too, and the tone mapper still runs once.

**Caution:** everything after the tone mapper is tied to one display range, which makes rendering the same edit for SDR and HDR harder. Keeping physics before the tone mapper and only the look after it limits that risk.
