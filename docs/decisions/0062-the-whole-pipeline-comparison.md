# 0062. The whole-pipeline comparison

## Context

Each port passing its own case does not prove that the whole pipeline renders as darktable does: defaults, order and the raw stage meet only there. How is the whole pipeline compared with darktable, on which photos, and within what tolerance? This comparison is Phase 1's exit test and stays a check until the migration ends ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)).

## Decision

**`apertrust-golden pipeline` renders each raw of the raw.pixls.us corpus through every operation with its defaults,** as `apertrust-cli export` does without an edit, tiled when it would not fit, and through darktable-cli at the reference release with the same settings written into its history.

- **darktable's side** gets Apertrust's crop, levels, white balance, highlight and demosaic settings and input profile, exposure at Apertrust's effective value with darktable's exposure-bias compensation off, so the two never read the EXIF differently, AgX at the scene-referred defaults both share, Apertrust's colour calibration, and orientation off. Exposure and AgX are written as darktable's version 7, exposure with highlight-preservation compensation off. No colour balance rgb on either side, since neither runs it on an unedited photo.
- **The output profile** follows colorout's type: sRGB, linear Rec2020 or linear Rec709; any other is refused.
- **What is measured** is the engine's output before `apertrust-cli export` rounds it to 8 bits, clipped to 0 to 1 as a PNG is, against darktable's 16-bit PNG, by CIEDE2000 per pixel within the declared image. 8-bit rounding alone costs a mean of about 0.2, so the tolerance does not bound the shipped 8-bit file.
- **The tolerance:** a mean ΔE2000 of at most 0.01, ten times what the pipeline gives where the decoders agree, and at most 0.01% of pixels over 2.3, darktable's own maximum, since rounding tips the occasional demosaic direction choice and AgX makes a few such pixels visible. The 99th percentile is reported.
- **The scope:** the samples both decoders read as colour mosaics, 29 of the 40. The other 11 skip only for their listed reason (`EXPECTED_SKIPS`): rawler cannot decode it, it is not a colour mosaic, or darktable-cli's loader rejects it. Any other error or skip fails the run; the list comes from the corpus manifest, so a missing sample fails too.
- **At a size:** `pipeline IMAGE --width W --height H [--hq BOOL]` compares an export at a size with darktable-cli's at the same size, measured without a bound, and `--add CASE` puts a golden case's module on both sides.

The current results are in AGENTS.md and the roadmap: Sony's two lossy ARWs fail until rawler seeds their dither as rawspeed does ([0023](0023-rawler-its-camera-data-fixed-upstream.md)).

## Options considered

- darktable's integration-test tolerance as it is (mean 0.767, max 2.3): it fails raws on single pixels.
- darktable's mean with the pixel share: it passes the lossy ARWs whose decoders differ and catches regressions late.
- Judging the CLI's 8-bit PNG: every sample fails at a mean of about 0.2 whatever the pipeline does.
- Relying on darktable's migrations of older module versions in the history: they change settings without saying so.
- Comparing in sRGB whatever colorout's profile.
- Keeping all 40 samples: it waits on decoders that may never read some; porting the linear and monochrome path first.

## Consequences

- A change to a default, an order or the raw stage runs this comparison before it lands.
- The linear, monochrome and undecodable samples join as their paths arrive.
