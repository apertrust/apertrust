# 0063. Departures from darktable, and views below full scale

## Context

During the migration a port renders what darktable's export renders ([0061](0061-golden-image-tests-against-darktable-cli.md)). When may a port differ, and how is that kept honest? And the viewport, a still or an export at a size renders below full resolution: should a smaller view follow darktable's darkroom and preview pipes, which take shortcuts, or show what the export would show at that size (principle 2)?

## Decision

**A port departs from darktable only where a difference is agreed, and its spec lists each one** (`docs/ports/<module>.md`, Deliberate differences).

- Where an agreed difference changes what the export renders, darktable's behaviour stays behind a setting only the golden harness reaches, never in a schema, an edit or a module, so the cases still check everything else against darktable; the difference's size is measured and stated in the spec and the published results.
- Differences within a case's tolerance (fp32 redesigns, guards, evaluation order), and choices where darktable's own result is not reproducible, are not departures.
- The rule belongs to the migration: after it a processing change is a baseline change ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)).

**The agreed differences,** each listed in its spec or module page:

- grain: a seed stored in the edit, its default darktable's file-name hash, one stored value darktable does not have (planned; `docs/ports/grain.md`);
- vignetting: the raised-cosine fall-off of darktable's GPU path, where darktable's CPU path is linear without dithering (planned; `docs/ports/vignette.md`);
- AgX: dynamic range scaling as the pickers' and Read exposure's margin only, never writing into the range, and Curve y gamma left alone by Keep the pivot on the diagonal (`docs/modules/agx.md`);
- pickers: a picked value held at its parameter's `$MIN` or `$MAX` where the module's own rule leaves it past them, and Colour balance RGB's picked hue wrapped onto 0° to 360° ([0079](0079-colour-pickers-one-api-for-modules-and-the-colour-picker.md); `docs/pickers.md`);
- the tone equalizer: none yet.

**Every smaller view follows the export at its scale,** never darktable's darkroom or preview pipe:

- whole-image work runs over the whole image at the pipe's scale, never over a visible region ([0015](0015-whole-image-passes.md));
- lengths follow the scale from their full-resolution values: darktable's radius, truncated once at full resolution, times the scale, unrounded where a real is taken; where an integer is needed the scaled radius rounds to the nearest, and what darktable derives from it keeps darktable's rounding;
- every branch darktable takes on a length is taken on the length scaled from full resolution, as darktable-cli's export at that size takes it;
- non-local means keeps K and scales P and the offsets by the pipe's scale in every pipe, as darktable's export at that size does; darktable's darkroom rule is a shortcut for speed, not followed;
- darktable's preview rounding up (`ceilf(rad · scale)`) is not followed.

**Iterated box means' radii below scale 1** (`apertrust_color::box_filter::iterated_radii`): where darktable applies one box mean of radius R N times, as bloom and soften do eight times, the blur's variance is N R (R + 1) / 3 pixels² an axis. Below scale 1 the passes take radii r and r + 1 so that the variance is the export's at that scale: with T = N s² R (R + 1), in f64, r is the largest whole number with N r (r + 1) ≤ T, and k the nearest whole number to (T − N r (r + 1)) / (2 (r + 1)), an exact half to the larger, at most N; iteration i takes r + 1 for i < k and r after. At scale 1, T is N R (R + 1), so every pass is darktable's radius. Over radii up to 256 the nearest-radius rule errs by up to 61% in σ, this rule by up to 5.7%. A single box mean, such as a guided filter's window, keeps the nearest radius.

**A test per operation** holds its viewport to the export downscaled: the filter's share at most 0.15 mean ΔE2000 at 2400 × 1600 and at 1/2, and 0.4 at 1/4 and 1/8, and within 0.767 mean in all at 2400 × 1600, with no maximum, since the per-pixel gap any nonlinear operation shows after a downscale reaches several units at hard edges. A port's first measurement becomes its baseline.

## Options considered

- Matching darktable even where an agreed difference says otherwise, or departing wherever Apertrust's maths seems better.
- darktable's darkroom and preview pipes for smaller views: the photo on screen would differ from its export.
- The nearest radius for iterated box means: soften misses the target by up to twice.
- One radius matching the variance; a Gaussian of the scaled σ (darktable's OpenCL approximation); the blur at full resolution in the viewport, too slow; a fractional box, kept as the remedy should a measurement miss.

## Consequences

- Each port's spec has a Scale dependence section and lists its differences; its viewport test records the four scales.
- An export rendered below full resolution follows the same rules as the viewport.
