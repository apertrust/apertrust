# Port spec: exposure

## Source

darktable at the reference release, commit 20891f6c6f. The reference is darktable's CPU path.

- `src/iop/exposure.c`: the params (`:66-75`), `legacy_params()` (`:157-310`), `reload_defaults()` (`:346-372`), `_get_exposure_bias()` (`:569-581`), `_get_highlight_bias()` (`:584-602`), `commit_params()` (`:605-641`) and `process()` (`:541`); `gui_update()`'s toggle (`:695-705`), `gui_init()`'s (`:1202-1209`) and the picker's `_auto_set_exposure()` (`:866-965`).
- `src/common/exif.cc`: `_check_highlight_preservation()` (`:1146-1283`), which `dt_exif_img_check_additional_tags()` (`:1285-1310`) calls on every raw rawspeed decodes (`src/imageio/imageio_rawspeed.cc:278`) and every raw LibRaw decodes, the path a CR3 takes (`src/imageio/imageio_libraw.c:490`). It sets the image's `exif_highlight_preservation` (`src/common/image.h:266`), which the library's database does not keep.
- Exiv2 0.28, the version darktable 5.6.2's packages ship (`libexiv2.28`), through which darktable reads the maker notes.
- OpenCL: `data/kernels/basic.cl`.

This spec covers the operation as ported. The Exposure tool's page (`docs/tools/exposure.md`) holds the controls, the picker and the invariant rows.

## Status

Ported. The kernel is `slang/exposure.slang` in `crates/op/exposure`, with its operation and setup in `src/lib.rs`.

- **Ported:** manual mode; the camera's exposure bias compensated; the exposure a highlight-preserving mode held back given back.
- **Not ported:** automatic mode, darktable's deflicker, which needs the raw histogram; its percentile and target level other than darktable's defaults. `prepare` returns `NotPorted` for them.
- **GPUs:** checked on one, the Apple M4.

## Purpose

Brightens or darkens the photo as if it had been exposed differently: every RGB value is scaled by one factor, after a black level is subtracted. It is darktable's exposure module, the Exposure tool (`docs/tools/exposure.md`).

Some cameras underexpose the raw in their highlight-preserving modes and brighten their own JPEG to match: Canon's Auto Lighting Optimizer, Fujifilm's dynamic range DR200 and DR400, Nikon's HLG tone mode and Active D-Lighting, Olympus's gradation. darktable estimates the exposure held back from the maker notes and, with highlight preservation compensation on, as it is by default, gives it back.

## Pipeline

- **darktable position:** after demosaic, before colorin, in camera RGB.
- **Apertrust slot:** camera RGB, order 21 (`Stage::CameraRgb`).
- **Domain:** scene-referred.
- **Input and output:** camera RGB after demosaic, `f32`, alpha passed through.
- **Assumes earlier modules:** rawprepare's black at 0 and white at 1.
- **Layer support:** yes.

## Parameters

darktable params version 7, 28 bytes: `mode` (enum, i32), `black`, `exposure`, `deflicker_percentile`, `deflicker_target_level` (f32), `compensate_exposure_bias`, `compensate_hilite_pres` (gboolean, i32).

| Name | darktable field | Type | Range | Default | Owner | Meaning |
| --- | --- | --- | --- | --- | --- | --- |
| Mode | `mode` | choice: manual, automatic | | manual | Exposure (Mode) | Manual, or automatic, darktable's deflicker (not ported) |
| Black | `black` | f32 | −1 to +1 | −2⁻¹² on a colour raw, else 0 | Exposure (Black level correction, an offset over this) | The level set to 0 |
| Exposure | `exposure` | f32, EV | −18 to +18 | +0.7 on a colour raw, else 0 | Exposure (Exposure) | The exposure change before the compensations below |
| Percentile | `deflicker_percentile` | f32, % | 0 to 100 | 50 | Exposure (not ported) | Automatic mode's meter |
| Target level | `deflicker_target_level` | f32, EV | −18 to +18 | −4 | Exposure (not ported) | Automatic mode's target |
| Compensate camera exposure | `compensate_exposure_bias` | bool | | on for a raw, else off | Exposure (Compensate camera exposure) | Takes the camera's exposure compensation off |
| Compensate highlight preservation | `compensate_hilite_pres` | bool | | on | Exposure (Highlight preservation mode) | Gives back the exposure a highlight-preserving mode held back |

## Parameter versions

| Version | Change | Migration |
| --- | --- | --- |
| 2 | `black`, `exposure` and more | Version 6 with manual mode, the deflicker's defaults and no bias compensation |
| 3 to 5 | The deflicker's settings, then `mode` | Version 6 with no bias compensation |
| 6 | Adds `compensate_exposure_bias` | Version 7 with `compensate_hilite_pres` off: "module did not compensate h.p. before version 7" |
| 7 | Adds `compensate_hilite_pres` | |

Apertrust's schema is version 2 and holds version 7's fields. The golden `exposure` case holds version 5 or 6, so its `compensate_hilite_pres` is off; `crates/golden/src/ports.rs` reads both.

## Defaults (`reload_defaults`)

In the scene-referred workflow, for a raw darktable's rawprepare supports, and the module's first instance:

- **A colour raw:** exposure +0.7 EV, black −2⁻¹² (darktable's −0.000244140625), the camera's bias compensated.
- **A monochrome raw:** exposure 0, black 0, the bias compensated.
- **Any other image:** exposure 0, black 0, no bias compensation.
- **`compensate_hilite_pres`** is on for the module's first instance and off for any other (`dt_iop_is_first_instance`), on every image, "to avoid multiple application". Only a raw records a held-back exposure, so on any other image it changes nothing.

What is an identity: exposure at the bias it compensates with black 0, `scale` 1, which no default is on a colour raw.

## The held-back exposure (`_check_highlight_preservation`)

darktable reads the first of these tags the raw holds, through Exiv2's keys, `FIND_EXIF_TAG` finding a tag present and holding a value, in this order, and sets `exif_highlight_preservation` in EV, 0 where none applies:

| Maker | Tag (Exiv2's key) | Values | EV |
| --- | --- | --- | --- |
| Canon | `Exif.Canon.LightingOpt` (0x4018), its third value; with Exiv2 0.28, which decodes the tag into an array, `Exif.CanonLiOp.AutoLightingOptimizer`, the same value | Auto Lighting Optimizer: 0 standard, 1 low, 2 strong, 3 off | 0.50, 0.33, 0.66, 0 |
| Fujifilm | `Exif.Fujifilm.DevelopmentDynamicRange` (0x1403), or where it is missing `Exif.Fujifilm.AutoDynamicRange` (0x140b) | 100, 200, 400 % | 0, 1, 2 |
| Nikon | `Exif.Nikon3.ColorSpace` (0x001e) | 4, HLG | 2; any other value 0, and darktable stops there |
| Nikon | `Exif.Nikon3.ActiveDLighting` (0x0022), only where ColorSpace is missing | 3 normal, 5 high, 7 extra high, 8 to 11 extra high 1 to 4; 0 off, 1 low, 65535 auto | 0.33, 0.66, 1.00, 1.10, 1.20, 1.30, 1.33; 0 |
| Olympus, OM System | `Exif.OlympusCs.Gradation` (CameraSettings 0x2020, tag 0x050f), or `Exif.OlympusRd2.Gradation` (RawDevelopment2 0x2031, tag 0x0112) | a fourth value 1, auto overriding the user's choice; else −1, −1, 1, low key | 0.33; 0.66; high key and the rest 0 |
| Pentax | `Exif.Pentax.DynamicRangeExpansion` (0x0069), its first byte 1 | dynamic range expansion on | 1, as darktable means it; 0 through Exiv2 0.28 (below) |
| Sony | `Exif.Sony1.DynamicRangeOptimizer`, `Exif.Sony2.DynamicRangeOptimizer` | | 0, left to do in darktable |

- **Pentax through Exiv2 0.28:** darktable reads the tag's bytes with `Value::dataArea()`, which only a tag pointing at a data area fills; for this tag it is empty, so `c_data(0)` is null and the first byte reads 0. With Exiv2 0.27, darktable reads the bytes through `toString()` instead and would count +1 EV. darktable-cli 5.6.2 renders the K-3 Mark III, whose raw records expansion on (`1 2 0 0`), the same with the compensation on or off (Results).
- **Nikon:** every Nikon raw of the corpus records ColorSpace, sRGB, so darktable reads none of their Active D-Lighting: the Z 9's normal would be 0.33 EV, and darktable-cli renders it with none (Results).
- **Canon:** darktable reads the Auto Lighting Optimizer only. Highlight Tone Priority, the fourth value of the same tag, is not read. A raw that keeps the optimizer only in its custom functions, as the 5D Mark II's sRAW does, records none.

**`_get_highlight_bias`:** the value where it is above 0, clamped to −1 to +4 EV; 0 otherwise. darktable shows the toggle only where this is above 0.

**Apertrust** reads the same tags over rawler's public parsers in `apertrust-raw` (`crates/raw/src/highlight.rs`), with no change to rawler: a CR3's Canon maker note from its CMT3 box (`formats::bmff`), a RAF's Fujifilm maker note from its embedded JPEG's EXIF (as lens correction reads CropMode, decision 25), and the others from the TIFF's EXIF maker note (`IFD::parse_makernote`), Olympus's and OM System's directories by their headers and offsets from the note's start. `RawImage::highlight_preservation` carries the value, and the engine copies it into `Camera::highlight_preservation` in the image context (decision 24).

## Setup on the CPU (`commit_params`)

The exposure the kernel applies:

1. `exposure`;
2. less `_get_exposure_bias` where `compensate_exposure_bias` is on: the camera's exposure compensation, clamped to ±5 EV;
3. plus `_get_highlight_bias` where `compensate_hilite_pres` is on.

In that order in `f32`, as darktable's. `prepare` packs the black and that exposure (`effective_exposure`); the kernel computes the scale.

## Per pixel (`process`)

`scale = 1 / (2^−exposure − black)`; `out = (in − black) · scale` on RGB, alpha passed through (`process`, `_process_common_setup`).

## Neighbourhood and scale

None: per pixel.

## Precision and special values

`2^−exposure − black` is never 0: the tool keeps the black 0.01 below the white (decision 30).

## CPU and OpenCL differences in darktable

None.

## Deliberate differences

- **Pentax's dynamic range expansion** reads as darktable-cli 5.6.2 reads it through Exiv2 0.28, as none (above), so the K-3 Mark III renders as darktable renders it. Apertrust reads no Pentax tag. Size: none against darktable-cli; +1 EV on a Pentax raw with expansion on against what darktable means.
- **Only the newer Olympus maker note** is read: "OLYMPUS\0" and "OM SYSTEM". The older "OLYMP\0" note, of cameras from before gradation, is not. No raw of the corpus has one.
- **One instance:** darktable turns the compensation on for a module's first instance only. Apertrust's operation has one instance per edit until layers (Phase 4), when an instance after the first must default to off.

## Tool binding notes

Exposure owns every parameter (`docs/tools/exposure.md`).

- **Highlight preservation mode** is `compensate_hilite_pres` itself, on by default, as darktable's first instance. Written only where set.
- **Availability:** it can change the photo only where `_get_highlight_bias` is above 0. darktable hides the toggle elsewhere; Apertrust draws it disabled, as a control that cannot act on the photo is (decision 30).
- **Its label:** darktable's "highlight preservation mode (0.5 EV)" shows the value; a label with a value of the photo's waits for the renderer, as Compensate camera exposure's does.
- **The tool's version 3:** before it the toggle was not ported and took only off, the one value an edit held, so an edit of version 2 or 1 reads it off, as it rendered.
- **The picker** adds the held-back exposure as darktable's `_auto_set_exposure` does: correction writes `−log2(Y / Y_target)` plus the compensated bias less the held-back exposure, and measure reads the area at the exposure the operation applies.

## Gradients

Parameters (black, exposure), the kernel's order. The compensations are added on the CPU, so the kernel and its derivatives are unchanged.

## Test plan

- **Golden case `exposure`** (darktable test 0001): +1 EV, black 0, an image that is not a raw.
- **The held-back exposure against darktable:** darktable-cli renders a corpus raw twice from the whole-pipeline comparison's history (decision 62), once with `compensate_hilite_pres` on, darktable reading the maker notes itself, once off with Apertrust's value added to the exposure. Equal renders mean equal values.
- **`apertrust-golden pipeline`** on the raws with a held-back exposure: Apertrust's default render, the value included, against darktable's.
- **Reader:** `crates/raw/src/highlight.rs`'s tests: darktable's table per mode, and every corpus raw read as Exiv2 0.28.9 reads it (`--ignored`, with the corpus).
- **Operation and tool:** the bias added after the camera's, its bounds, the toggle's default, availability and migration; the invariant rows E0, E32 and E33 (`docs/tools/exposure.md`).

## Results

Apple M4.

- **Reader:** every one of the 40 corpus raws reads as Exiv2 0.28.9 reads it: Canon's Auto Lighting Optimizer standard on the 5D Mark IV, 90D, M50, R5 and R6 Mark II, off on the 7D Mark II, none on the 5D Mark II's sRAW; Fujifilm's DR100 on four RAFs, none on the X-H2S; Nikon's ColorSpace sRGB on all six NEFs; Olympus's gradation normal on both ORFs.
- **The held-back exposure against darktable** (800 px renders, darktable-cli 5.6.2): ΔE2000 mean and max 0.0000 on all 13 raws checked. +0.5 EV on the 5D Mark IV, 90D, M50, R5 and R6 Mark II; none on the 7D Mark II (optimizer off), the K-3 Mark III (expansion on, Pentax), the Z 9 (Active D-Lighting normal under ColorSpace), the D5100, the X-T5 (DR100), the OM-1 Mark II and E-M1 Mark III (gradation normal) and the A7R V. On the K-3 Mark III, darktable's own reading against +1 EV gives a mean of 9.72, so darktable gives none there.
- **`apertrust-golden pipeline`** with the held-back exposure given back by default (the 90D's exposure +1.867 EV where it was +1.367): every raw passes the exit test, ΔE2000 mean 0.001 on each. The 5D Mark IV max 1.29, the 90D 0.54, the M50 2.72, the R5 3.36 (in 4 tiles), the R6 Mark II 0.79, each with 0.0000% of pixels over 2.3; the 7D Mark II and the K-3 Mark III, none held back, max 0.73 and 0.95.
- **Golden case `exposure`** (3908×2602): ΔE2000 mean 0.0000, p95 0.0000, max 0.0002.
- **Gradient check:** on the case, black 5.2e-7 and exposure 6.9e-6 relative to finite differences, forward against reverse 2.7e-8 and 3.6e-8 per pixel; on the synthetic sweep, 1.2e-5 and 7.5e-6, forward against reverse 2.7e-8 and 2.9e-8. The kernel is unchanged.

## Open questions

- **Hiding the toggle and its label with the value** come with the Exposure tool's redesign: darktable shows Highlight preservation mode only where a mode is detected, which needs decision 30's rule that a control that cannot act is drawn disabled, never hidden, changed for it; and its label, as Compensate camera exposure's, waits for labels with a value of the photo's.
- **Denoise (profiled)** reads the same value (`_get_iso_highlight_preservation_shift`, whole EV only); its port still takes the shift as 0 (`docs/ports/denoiseprofile.md`, difference 6), and can now read it from the image context.
