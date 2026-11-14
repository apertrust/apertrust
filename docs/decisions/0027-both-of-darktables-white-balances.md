# 0027. Both of darktable's white balances

## Context

darktable balances white in two modules: white balance, over the raw-stage `temperature` before demosaic, and color calibration, whose CAT tab adapts the scene's illuminant after input colour. Its scene-referred workflow runs both, the first at the camera's reference and the second doing the adaptation. Two modules for one task confuse people, but the first pass mirrors darktable ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)), and improvements come after the migration.

## Decision

**White balance is darktable's white balance module over `temperature`, and Colour calibration is darktable's color calibration with every tab, the CAT tab first, owning every `channelmixerrgb` parameter.** No deviation is recorded.

**The defaults are darktable's scene-referred ones.** An unedited colour raw runs both: temperature "as shot to reference" where the camera has a matrix ("as shot" without), so demosaic and highlight reconstruction see the as-shot balance and colorin's late correction returns the image to the camera's D65 reference; then Colour calibration with As shot, adapting under CAT16, the camera's illuminant snapped to a daylight or Planckian light within 0.005 in darktable's UCS, as `reload_defaults` snaps it ([0022](0022-pipeline-order-and-the-default-pipeline.md)).

**White balance** (id `temperature`):

- Its controls are darktable's parameters: `red`, `green`, `blue`, `various` (emerald, four-colour sensors only, not ported) and `preset`.
- **Settings**, a preset row over all five: as shot, camera reference, as shot to reference; from image area drawn disabled until its picker; user modified the label when nothing matches within darktable's 1e-5, never applied.
- **Temperature** (1901 to 25000 K) and **Tint** (0.135 to 2.326), a lossless view of the coefficients through darktable's model: the camera's matrix, black-body or CIE daylight spectra over the CIE 1931 observer, tint dividing Y. Written back normalised to green 1 as darktable's `_temp_tint_callback` does, the inverse bisected to convergence so it round-trips ([0031](0031-views-and-curves-of-the-same-stored-values.md)). Unavailable without the camera's matrix.
- The closed Channel coefficients group. Background only, no Auto.
- **The camera's presets** (planned): darktable's `wb_presets.json` through the image context ([0024](0024-the-cameras-data-in-the-image-context.md)). Settings lists its five, a heading with the camera's maker and model, then the photo's presets in the file's order. An edit stores a camera preset in `preset` by darktable's name, a permanent id (a kelvin preset `kelvin_5000`); the binding writes the operation's `preset` as a sixth option, `camera`, its late correction off. **Finetune**, −9 to 9 mired with a step of 1 and the matched preset's tunings as its hard range, is the Settings row's tuning, addressed like a control and never stored; an option applies at the tuning the values show, interpolated between bracketing entries as darktable does.

**Colour calibration's CAT tab**, in darktable's `gui_init` order: the adaptation, unavailable with the illuminant "as shot in camera"; the illuminant (darktable's nine), its F or LED source, its temperature (1667 to 25000 K, soft 3000 to 7000), or a custom light's hue and chroma (a lossless view of `x` and `y`, chroma soft 100 and hard 300); gamut compression (0 to 12, soft 4); clip negative RGB from gamut; the closed area colour mapping group. Every illuminant row is hidden with adaptation none. Chroma's reach widens the operation's `x` to −0.1 to 1.2 and `y` to 0.0001 to 2, `prepare` scaling the pair back where its sum passes 0.9999. Its R, G, B, Colourfulness, Brightness and Grey tabs are darktable's too.

**Follow writes:** where darktable's GUI writes other parameters with a control, the module gives them through `Module::follow` in the same change ([0031](0031-views-and-curves-of-the-same-stored-values.md)): an illuminant, its source or temperature writing `x`, `y` and the temperature; hue or chroma writing the temperature; a coefficient, temperature or tint writing `preset` user modified, which turns the late correction off. darktable's session memory (the last daylight and Planckian temperatures, user-modified coefficients) is not mirrored.

**The trouble messages** are darktable's, computed by the session from both modules' values: "white balance applied twice" on White balance and "white balance module error" on Colour calibration while Colour calibration adapts and `temperature` is off the camera's reference.

**Not mirrored:** the hidden Finetune slider's last value and the narrowing of its track within a session; a preset's fourth channel; the two preset entries above the operation's 8, refused. Mirrored and reported upstream: darktable's dead daylight fallback, under which camera reference stays 1, 1, 1 for a camera without a matrix.

## Options considered

- A single White balance driving Colour calibration's illuminant, the raw-stage balance hidden at the camera's reference: a deviation from darktable, and its reason, keeping raw-stage drags rare, does not outweigh mirroring.
- The camera's own illuminant as a custom one, never snapped: an unedited photo would depart from darktable's.
- Temperature with the as-shot coefficients and Colour calibration added at the first step: a raw-stage change and a jump.
- Storing temperature and tint, or the camera preset as darktable's list position: the light stored twice, or a position that means another preset on another camera.
- Finetune as a lossless view or a stored control: its inverse needs the preset, and it would duplicate the coefficients.
- Mirroring darktable's session memory: hidden state.

## Consequences

- A White balance drag restarts at the mosaic and reruns the raw stage and demosaic at full resolution: on the M4 about 40 ms a frame at 10 MP and 240 ms at 61 MP before the rest. A spike times the drag; if it misses, demosaic's speed is the first engine performance work. Demosaic at reduced size while dragging stays ruled out ([0012](0012-planning-regions-scale-and-cache-points.md)).
- The whole-pipeline comparison writes Apertrust's Colour calibration parameters into darktable's history ([0062](0062-the-whole-pipeline-comparison.md)).
- Open: local white balance, which is Colour calibration's CAT on a layer ([0034](0034-layers-masked-tool-instances-in-a-fixed-order.md)); the button bar and the CCT readout row, with the renderer.
