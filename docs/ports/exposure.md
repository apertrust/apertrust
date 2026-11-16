# Port spec: exposure

## Source

darktable at the reference release, commit 20891f6c6f. The reference is darktable's CPU path.

- `src/iop/exposure.c`: the params (`:66-75`), `legacy_params()` (`:157-310`), `reload_defaults()` (`:346-372`), `_get_exposure_bias()` (`:569-581`), `_get_highlight_bias()` (`:584-602`), `commit_params()` (`:605-641`) and `process()` (`:541`); `gui_update()`'s toggle (`:695-705`), `gui_init()`'s (`:1202-1209`) and the picker's `_auto_set_exposure()` (`:866-965`).
- `src/common/exif.cc`: `_check_highlight_preservation()` (`:1146-1283`), which `dt_exif_img_check_additional_tags()` (`:1285-1310`) calls on every raw rawspeed decodes (`src/imageio/imageio_rawspeed.cc:278`) and every raw LibRaw decodes, the path a CR3 takes (`src/imageio/imageio_libraw.c:490`). It sets the image's `exif_highlight_preservation` (`src/common/image.h:266`), which the library's database does not keep.
- Exiv2 0.28, the version darktable 5.6.2's packages ship (`libexiv2.28`), through which darktable reads the maker notes.
- Automatic mode, darktable's deflicker: `src/iop/exposure.c`'s `DEFLICKER_BINS_COUNT` (`:64`), `EXPOSURE_CORRECTION_UNDEFINED` (`:117`), the preset "magic lantern defaults" (`:316-322`), `_deflicker_prepare_histogram()` (`:374-412`), `_raw_to_ev()` (`:415-428`), `_compute_correction()` (`:430-464`), `_process_common_setup()` (`:468-510`), `commit_params()`'s test (`:629-642`), `gui_update()` (`:665-748`), `gui_changed()` (`:976-1023`), `_show_computed()` (`:1026-1041`), `gui_init()`'s automatic rows (`:1220-1245`) and the exposure proxy (`:836-841`); `src/common/histogram.c`'s `_bin_raw()` (`:53-69`), `_hist_worker()` (`:163-198`) and `dt_histogram_helper()` (`:202-221`); `src/iop/rawprepare.c`'s `output_format()` (`:288-299`) and `commit_params()` (`:716-724`); `src/imageio/imageio_rawspeed.cc`'s crop (`:294-312`) and buffer format (`:338-346`).
- OpenCL: `data/kernels/basic.cl`.

This spec covers the operation as ported. The Exposure tool's page (`docs/tools/exposure.md`) holds the controls, the picker and the invariant rows.

## Status

Ported. The kernel is `slang/exposure.slang` in `crates/op/exposure`, with its operation and setup in `src/lib.rs`.

- **Ported:** manual mode; the camera's exposure bias compensated; the exposure a highlight-preserving mode held back given back; automatic mode, darktable's deflicker, over the raw's histogram the engine counts on the GPU (decision 87), with its computed EC readout.
- **Not ported:** nothing. `prepare` returns `NotPorted` for no parameter value.
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
| Mode | `mode` | choice: manual, automatic | | manual | Exposure (Mode) | Manual, or automatic, darktable's deflicker |
| Black | `black` | f32 | −1 to +1 | −2⁻¹² on a colour raw, else 0 | Exposure (Black level correction, an offset over this) | The level set to 0 |
| Exposure | `exposure` | f32, EV | −18 to +18 | +0.7 on a colour raw, else 0 | Exposure (Exposure) | The exposure change before the compensations below |
| Percentile | `deflicker_percentile` | f32, % | 0 to 100 | 50 | Exposure (Percentile) | Automatic mode's meter, in % of the photosites |
| Target level | `deflicker_target_level` | f32, EV | −18 to +18 | −4 | Exposure (Target level) | Automatic mode's target, in EV below the white point |
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

## Automatic mode (deflicker)

darktable's automatic mode stores a rule, not an exposure: the exposure that puts a percentile of the raw's values at a level below the white point. Pasted onto a series, each frame gets its own exposure, so a time-lapse comes out matched. The per-pixel maths is manual mode's; only the exposure it applies differs.

### When it acts

- **`commit_params`** sets `d->deflicker` only where the mode is automatic and the image is a raw whose buffer is one 16-bit channel: `dt_image_is_raw`, `buf_dsc.channels == 1`, `buf_dsc.datatype == TYPE_UINT16` (`exposure.c:636-642`). rawspeed gives that buffer to every mosaic and monochrome raw of integer data (`imageio_rawspeed.cc:338-346`); a floating-point raw, a linear DNG and a Canon sRAW have another.
- **Anywhere else** automatic mode renders as manual mode, the compensated `exposure` included: `_process_common_setup` computes the exposure only under `d->deflicker` (`:477`).

### The histogram (`_deflicker_prepare_histogram`)

- **The data:** the full-resolution mipmap of the image, `DT_MIPMAP_FULL` (`exposure.c:384`), which for a raw is the decoded mosaic as rawspeed gives it: 16-bit values before rawprepare, black not subtracted, the whole readout, `width × height` uncropped (`imageio_rawspeed.cc:296-297, :401`). Every pipe meters the same full-resolution raw, the preview and a small export included.
- **The pixels:** every photosite in rawspeed's crop, whatever its filter colour: rows `crop_y` to `height − crop_bottom`, columns `crop_x` to `width − crop_right` (`exposure.c:395-401`, `histogram.c:60-63, :190`). The crop is the image's, from rawspeed (`imageio_rawspeed.cc:302-312`), never rawprepare's parameters, which `_image_set_rawcrops` writes only into `p_width` and `p_height` (`rawprepare.c:582-622`); darktable's own comment says so ("FIXME: get those from rawprepare IOP somehow", `exposure.c:398`).
- **The bins:** 65536, one per 16-bit value (`exposure.c:64, :405`), each value its own bin, `MIN(value, 65535)` (`histogram.c:67`); `pixels` the crop's photosite count (`histogram.c:196-197`).
- **When:** with the GUI, once as the mode becomes automatic or the module's values load (`gui_changed`, `exposure.c:1001-1003`; `gui_update`, `:733-734`), cached in the GUI data and used by every pipe; without it, as darktable-cli exports, once per process call (`:488-493`).

### The percentile (`_compute_correction`, `exposure.c:430-464`)

In `double`:

1. `thr = CLAMP(pixels · percentile / 100, 0, pixels)`.
2. The first bin `i`, from 0 up, where the running count `n` reaches `thr`: `n ≥ thr`. At percentile 0 that is bin 0; at 100 the highest bin holding a photosite.
3. That bin's value `raw = i`.

### Into EV (`_raw_to_ev`, `exposure.c:415-428`)

`ev = log2(max(raw − black, 1)) − log2(white − black)`, in `double`, where:

- `black` is `pipe->dsc.rawprepare.raw_black_level`: rawprepare's four black levels averaged and rounded, `(uint16_t)roundf(sum / 4)` (`rawprepare.c:717-723`), its current parameters, which its `output_format` puts in the pipe's buffer descriptor (`:288-299`);
- `white` is rawprepare's `raw_white_point` (`:724`);
- `raw − black` is at least 1, so a value at or below the black meters as one unit above it.

So `ev` is 0 at the white point, above 0 for a raw value above the white (the formula clamps nothing there), and about −16 at the darkest.

### The exposure applied

- `exposure = target_level − ev` (`exposure.c:463`), rounded to `float`, replacing the exposure `commit_params` compensated (`:473-494`).
- **Compensate camera exposure and Highlight preservation mode** do nothing in automatic mode: `commit_params` adds both to `d->params.exposure` (`:618-627`), which the deflicker's result replaces.
- **Black level correction** still applies: `d->black = d->params.black` (`:473`), and `scale = 1 / (2^−exposure − black)` as in manual mode (`:507-508`).
- **No histogram** (the mipmap could not be read) leaves the exposure at `EXPOSURE_CORRECTION_UNDEFINED`, `−FLT_MAX` (`:117, :436-439`): a white of infinity, a scale of 0, a black image.

### The panel

- **Rows** (`gui_init`, `exposure.c:1184-1262`): Mode, then a stack whose manual page holds Compensate camera exposure, Highlight preservation mode and Exposure, and whose automatic page holds Percentile (in %), Target level (in EV) and the computed EC row; then Black level correction and the Area exposure mapping group, on both.
- **Computed EC:** a row of two labels, "computed EC: " and the value, `"%.2f EV"` of the exposure applied (`:1236-1245, :1026-1041`), with the tooltip "what exposure correction has actually been used". Only the preview pipe sets it (`dt_pipe_is_preview`, `:497-504`); `gui_update` empties it (`:720-725`), so it is blank until the preview has run.
- **The picker** is turned off as the mode becomes automatic (`_autoexp_disable`, `:731, :991`); its quad sits on Exposure, which automatic mode hides.
- **The exposure proxy**, the histogram's drag and scroll, moves Target level in automatic mode (`:836-841`).
- **Insensitive:** Mode, wherever the image is not a raw of one 16-bit channel, and its mode reset to manual with a history item (`gui_update`, `:671-681`); choosing automatic there resets it to manual too (`gui_changed`, `:993-1000`).
- **The preset** "magic lantern defaults" (`init_presets`, `:316-322`): automatic, black 0, exposure 0, percentile 50, target −4, both compensations off.

### Its parameter versions

Automatic mode's fields came in versions 3 and 4 (`legacy_params`, `exposure.c:209-283`): version 3's `deflicker` flag becomes the mode; version 4's histogram source, thumbnail or raw, is dropped ("deflicker still was not publicly released at that point", `:255-256`). Apertrust's schema already holds all three parameters (Parameters).

### Measured: how much the metered crop moves the percentile

rawspeed's crop, which darktable meters, is not Apertrust's: Apertrust crops the raw to rawler's active area and the declared image after demosaic (`docs/ports/rawprepare.md`, Defaults), and rawspeed's crop matches the active area on 6 of 33 raws. The percentile over the active area against the declared crop (rawler's `crop_area`), on the decoded mosaics, in EV:

| Raw | Percentile 10 | 50 | 90 |
| --- | --- | --- | --- |
| Canon 90D | 0 | 0.0026 | 0.0214 |
| Canon R5 | 0 | 0 | 0.0030 |
| Fujifilm X-T5 | 0 | 0.0076 | 0.0031 |
| Nikon D5100 | 0.0087 | 0.0083 | 0.0074 |
| Sony A1 | 0 | 0.0027 | 0.0007 |
| Sony A7 III (`DSC00087.ARW`) | 0 | 0 | 0.0035 |

0.008 EV moves a mid-grey's L* by about 0.1, ten times the exit test's mean ΔE2000 of 0.01. So matching darktable-cli needs darktable's region exactly: the golden comparisons meter darktable's own crop, and the product the declared image (Deliberate differences).

### Gradients in automatic mode

`exposure = target_level − ev`, computed on the CPU in `prepare` and handed to the kernel as manual mode's exposure, so the kernel and its derivative entry points are unchanged: the derivative with respect to Target level is the kernel's with respect to its exposure input, times 1, and Black level correction's is unchanged. Where `prepare` holds the black at `2^−exposure − 0.01` (Deliberate differences), the black follows the exposure too, so Target level's derivative adds the kernel's black derivative times `−ln 2 · 2^−exposure`, and Black level correction's is 0 there. The percentile picks a bin: the exposure is a step function of it, its derivative 0 almost everywhere, so Percentile is no gradient parameter; an optimiser would move Target level. The histogram is a statistic of the raw, which no parameter of the operation changes, a constant to the derivatives.

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

Automatic mode (decision 87):

- **The metered region** is the declared image, the one Apertrust shows (`Camera::declared`), where darktable meters rawspeed's crop, which Apertrust does not know and which differs from it on most raws by a few photosites at the edges. The golden comparisons meter darktable's crop instead, recovered from darktable's own render, through a region only the harness gives (decision 63), so the algorithm is checked exactly. Size, the exposure over the declared image against darktable's crop, `pipeline --automatic` (Results): none on 15 of the 29 raws at 50 % and −4 EV, at most 0.012 EV (the OM-1 Mark II) and 0.011 EV (the K-3 Mark III), 0.008 EV or less on the others; up to 0.0072 EV at the other settings measured.
- **The computed EC readout** shows with the values as soon as the photo is live and its histogram is counted: the session reports it from the edit as it renders, where darktable leaves it blank until its preview pipe has run. Where automatic mode has no histogram, on an image that is not a raw or after a count that failed, it is blank, as darktable's is where it computes none, and the photo renders as manual mode. Nothing renders differently.
- **No histogram renders as manual mode,** never a black image: darktable's `−FLT_MAX` where the raw cannot be read (`exposure.c:436-439`) is a black image; Apertrust's count failing leaves the exposure manual mode's, and the next render counts again.
- **A white at or below rawprepare's black** meters as one level above it, as rawprepare's divisor is kept at least one level. darktable's unsigned `white − black` (`exposure.c:419`) wraps where the white is below the black, near 2³², metering the value some 32 EV below its white, and is 0 where they are equal, `log2(0)` making the exposure −∞ and the image black. Only an edit of Raw black/white point that inverts or meets the levels reaches it.
- **The black is held below automatic mode's white:** in automatic mode the tool binds Black level correction as the offset over the photo's black within darktable's ±1 alone, and `prepare` puts a black that reaches the white the photo renders, `2^−exposure` at the computed exposure or at manual mode's without a histogram, 0.01 below it, darktable's margin, the rule the tool applies in manual mode; any other black is kept. darktable's `gui_changed` compares the black with the white of its manual Exposure in either mode and, where the black reaches it, raises that Exposure (`_exposure_set_white`, `exposure.c:1017-1022`), which automatic mode ignores; nothing bounds the black against the computed exposure, so where that is higher darktable's scale `1 / (white − black)` grows without bound, turns infinite or negative, inverting the image, and where it is lower the black is the one set. Apertrust renders darktable's image wherever the black lies below the white rendered, every default edit included; in manual mode the tool keeps its own bound (decision 30).
- **The computed EC is signed,** "+0.83 EV", as a slider's value is and as the wireframe shows it, where darktable writes `%.2f EV`, "0.83 EV".
- **Automatic held on an image that is not a raw** stays in the edit, rendering as manual mode; darktable's darkroom resets it to manual with a history item (`gui_update`, `exposure.c:671-681`). An edit pasted onto a series keeps its rule for the raws in it.

## Tool binding notes

Exposure owns every parameter (`docs/tools/exposure.md`).

- **Highlight preservation mode** is `compensate_hilite_pres` itself, on by default, as darktable's first instance. Written only where set.
- **Availability:** it can change the photo only where `_get_highlight_bias` is above 0. darktable hides the toggle elsewhere, and so does Apertrust, through the tool's redesign (decision 88: `Tool::visible`).
- **Its label:** darktable's "highlight preservation mode (0.5 EV)" shows the value; the redesign adds it as "+0.5 EV", one decimal, signed, and Compensate camera exposure's as the camera writes it, in thirds, "(−⅔ EV)", where darktable writes `%+.1f` (decision 88: `Tool::label_value`).
- **The tool's version 3:** before it the toggle was not ported and took only off, the one value an edit held, so an edit of version 2 or 1 reads it off, as it rendered.
- **The picker** adds the held-back exposure as darktable's `_auto_set_exposure` does: correction writes `−log2(Y / Y_target)` plus the compensated bias less the held-back exposure, and measure reads the area at the exposure the operation applies.
- **Automatic mode:** Mode takes automatic where the photo is a raw, every raw the engine develops being one of 16 bits in one channel, darktable's test, and is drawn disabled elsewhere; an edit holding automatic on an image that is not a raw renders as manual mode, as darktable's `commit_params` leaves it. Mode, Percentile and Target level are their parameters, written where set. Percentile, Target level and the computed EC, a computed value (`Row::Computed`, `computed_exposure`, the operation's `computed`, two decimals in EV) show in automatic mode; Compensate camera exposure, Highlight preservation mode, Exposure and its picker in manual mode only, as darktable's stack has them, with Picker target in place of darktable's Area exposure mapping (decision 88); Black level correction in both. Defaults do not change, so the tool keeps version 3 with no migration; E32 now holds that every value is taken.

## Gradients

Parameters (black, exposure), the kernel's order. The compensations, and automatic mode's exposure, are computed on the CPU, so the kernel and its derivatives are unchanged; Target level's derivative is the kernel's exposure's (Gradients in automatic mode).

## Test plan

- **Golden case `exposure`** (darktable test 0001): +1 EV, black 0, an image that is not a raw.
- **The held-back exposure against darktable:** darktable-cli renders a corpus raw twice from the whole-pipeline comparison's history (decision 62), once with `compensate_hilite_pres` on, darktable reading the maker notes itself, once off with Apertrust's value added to the exposure. Equal renders mean equal values.
- **`apertrust-golden pipeline`** on the raws with a held-back exposure: Apertrust's default render, the value included, against darktable's.
- **Reader:** `crates/raw/src/highlight.rs`'s tests: darktable's table per mode, and every corpus raw read as Exiv2 0.28.9 reads it (`--ignored`, with the corpus).
- **Operation and tool:** the bias added after the camera's, its bounds, the toggle's default, availability and migration; the invariant rows E0, E32 and E33 (`docs/tools/exposure.md`).
- **Automatic mode:**
  - `apertrust-golden pipeline --automatic PERCENTILE,TARGET` on the corpus raws, Exposure in automatic mode on both sides, Apertrust metering darktable's crop, in the exit test's terms: every raw at 50 % and −4 EV, and six raws at 10 % and 90 % with −4 EV and 50 % and 90 % with −2 EV;
  - `RawHistogram`'s search, EV and exposure against a float64 restatement of `_compute_correction` and `_raw_to_ev` (`apertrust-plugin`): percentile 0 and 100, a count reached exactly and just missed, values at and below the black, a white at the black;
  - the operation (`apertrust-op-exposure`): the target reached, both compensations replaced, the black kept, rawprepare's levels from its input, manual mode without a histogram, `computed` equal to what `prepare` packs;
  - rawprepare's levels carried (`raw_black` the rounded mean, `raw_white`);
  - the engine's count against a CPU count, whole and in row windows, only where an instance reads it (`crates/engine/tests/raw_histogram.rs`); the session's readout with the values (`automatic_mode_reports_the_exposure_it_computes_with_the_values`);
  - the invariant rows E32 and E34 to E37 (`docs/tools/exposure.md`); the computed value's row in the design system and the app.

## Results

Apple M4.

- **Reader:** every one of the 40 corpus raws reads as Exiv2 0.28.9 reads it: Canon's Auto Lighting Optimizer standard on the 5D Mark IV, 90D, M50, R5 and R6 Mark II, off on the 7D Mark II, none on the 5D Mark II's sRAW; Fujifilm's DR100 on four RAFs, none on the X-H2S; Nikon's ColorSpace sRGB on all six NEFs; Olympus's gradation normal on both ORFs.
- **The held-back exposure against darktable** (800 px renders, darktable-cli 5.6.2): ΔE2000 mean and max 0.0000 on all 13 raws checked. +0.5 EV on the 5D Mark IV, 90D, M50, R5 and R6 Mark II; none on the 7D Mark II (optimizer off), the K-3 Mark III (expansion on, Pentax), the Z 9 (Active D-Lighting normal under ColorSpace), the D5100, the X-T5 (DR100), the OM-1 Mark II and E-M1 Mark III (gradation normal) and the A7R V. On the K-3 Mark III, darktable's own reading against +1 EV gives a mean of 9.72, so darktable gives none there.
- **`apertrust-golden pipeline`** with the held-back exposure given back by default (the 90D's exposure +1.867 EV where it was +1.367): every raw passes the exit test, ΔE2000 mean 0.001 on each. The 5D Mark IV max 1.29, the 90D 0.54, the M50 2.72, the R5 3.36 (in 4 tiles), the R6 Mark II 0.79, each with 0.0000% of pixels over 2.3; the 7D Mark II and the K-3 Mark III, none held back, max 0.73 and 0.95.
- **Golden case `exposure`** (3908×2602): ΔE2000 mean 0.0000, p95 0.0000, max 0.0002.
- **Gradient check:** on the case, black 5.2e-7 and exposure 6.9e-6 relative to finite differences, forward against reverse 2.7e-8 and 3.6e-8 per pixel; on the synthetic sweep, 1.2e-5 and 7.5e-6, forward against reverse 2.7e-8 and 2.9e-8. The kernel is unchanged, and automatic mode leaves these as they were.

Automatic mode:

- **`apertrust-golden pipeline --automatic 50,-4`** on the corpus, darktable-cli 5.6.2 in automatic mode, Apertrust metering darktable's crop: 27 of the 29 raws pass the exit test, ΔE2000 mean 0.001 on each, at most 0.0002% of pixels over 2.3 (the X-H2S, X-T5 and X100VI, maxima 6.45, 5.70 and 12.10, as in manual mode's X-Trans renders). The two that fail are Sony's lossy ARWs, the A1 (mean 0.038) and the A6400 (0.071), which fail manual mode's exit test too until rawler's dither seed is fixed (AGENTS.md). The computed exposures run from −1.41 EV (the 7D Mark II) to +2.28 EV (the Pixel 4a); each passing means darktable applied the same exposure, since its history's manual exposure is not it.
- **At other settings** on the 5D Mark IV, X-T5, Z 6, OM-1 Mark II, 7R V and S5 II, at 10 % and 90 % with −4 EV and 50 % and 90 % with −2 EV: all 24 pass, mean 0.001 each, at most 0.0027% of pixels over 2.3 (the X-T5 at 10 %).
- **The metered region:** the exposure over the declared image against darktable's crop differs by none on 15 raws at 50 % and −4 EV; by 0.0118 EV on the OM-1 Mark II, 0.0112 on the K-3 Mark III, 0.0083 on the D5100, 0.0060 on the 5D Mark IV and E-M1 Mark III, and less on the rest; at most 0.0072 EV at the other settings.
- **Cost,** the 7R V's 61 MP raw on the M4: the count of 60,217,344 photosites takes 20 to 32 ms of wall time once, its kernel compiled in under 1 ms, submitted and read back included; the export then takes 691 and 694 ms against manual mode's 671 and 700; an exposure change in the viewport 7.07 ms median against manual mode's 7.06; the plan of the six operations up to exposure for the readout, at each values report while it shows, 0.035 ms median. Manual mode counts nothing (E37).

## Open questions

- **Metering rawspeed's crop** would give darktable's computed exposure on every raw, from rawspeed's camera data as a data file (decision 87, Options considered); kept for later, should the up to 0.012 EV matter.
- [x] **Hiding the toggle and its label with the value:** settled by decision 88, the tool's redesign: this toggle and Compensate camera exposure hidden where the photo gives them nothing to do, by `Tool::visible`, and labels with the photo's value by `Tool::label_value`; built ([tools/exposure.md](../tools/exposure.md#the-page-as-redesigned-planned)).
- **Denoise (profiled)** reads the same value (`_get_iso_highlight_preservation_shift`, whole EV only); its port still takes the shift as 0 (`docs/ports/denoiseprofile.md`, difference 6), and can now read it from the image context.
