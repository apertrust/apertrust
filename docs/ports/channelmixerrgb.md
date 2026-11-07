# Port spec: color calibration (channelmixerrgb)

Source: darktable (the reference release, commit 20891f6c6f), `src/iop/channelmixerrgb.c`: `legacy_params()`, `init_presets()`, `init()`, `reload_defaults()`, `commit_params()` and `process()` through `_loop_switch()`, `_gamut_mapping()` and `_luma_chroma()`, with `_get_white_balance_coeff()`, `_check_if_close_to_daylight()`, `_auto_set_illuminant()` (the picker), `_extract_color_checker()` and `_extract_patches()` (the colour checker) and `_auto_detect_WB()` (illuminant detection, compiled out). Helpers from `src/common/chromatic_adaptation.h`, `src/common/illuminants.h`, `src/common/colorchecker.h`, `src/iop/gaussian_elimination.h`, `src/common/math.h` and `src/common/colorspaces_inline_conversions.h`; the defaults it depends on from `src/iop/temperature.c` (`reload_defaults`) and `src/develop/develop.c` (`dt_dev_equal_chroma`, auto-applied presets). OpenCL: `data/kernels/channelmixer.cl` (program 32). The reference is darktable's CPU path.

Status: **ported** (checks pass on the Apple M4), in `crates/op/channelmixerrgb` with the shared maths in `crates/color` (`adaptation.rs`, `illuminants.rs`, and the illuminants section of `slang/color.slang`). Params version 3 with the `V_3` colourfulness, every adaptation, every illuminant darktable offers.

- **Default pipeline:** in `OPERATIONS` with `default_enabled` on, so every colour raw runs it (architecture decision 27), from darktable's own defaults: As shot snapped to daylight or Planckian as `reload_defaults` snaps it (Defaults).
- **Module:** Colour calibration owns every parameter, its CAT tab's included (`docs/modules/colour-calibration.md`).
- **Planned:** the derivatives beyond x and y and with respect to the input, with reference matching (Gradients); the colour checker and the picker's readout (Illuminant detection, picker and colour checker); the CAT on a layer and where the grey runs, with layers (Pipeline; Tool binding notes).

Golden result (Apple M4, eleven cases, Results): every case within `Tolerance::DARKTABLE`, ΔE2000 mean at most 0.0001 and max at most 0.0005; the gradient check passes for x and y on every case and on the synthetic sweep. The defaults are darktable's on the 29 raw.pixls.us samples the exit test compares: the same illuminant on every one, x and y within 1.5e-7, the temperature within 0.003 K but on one sample, 0.14 K (Results, The defaults).

## Purpose

Color calibration is darktable's white balance in the scene-referred workflow. It takes the colour of the light the photo was taken under, the illuminant, and adapts the image to the pipeline's D50 white with a chromatic adaptation transform (CAT), so that surfaces that were neutral under that light become neutral. After the adaptation it can mix channels with a 3×3 matrix, change colourfulness and brightness per channel, turn the image grey with three channel weights, and compress chromaticity towards white. In algorithm terms: working RGB to a cone-like LMS space, a per-channel gain of D50 over the illuminant (von Kries), a matrix, then xyY gamut compression and a norm-preserving saturation step, and back to working RGB.

In the first pass one module drives it, Colour calibration, darktable's color calibration with every tab, the CAT tab included (architecture decision 27), and the raw-stage white balance is darktable's white balance module, White balance, over `temperature` (`docs/ports/temperature.md`).

## Pipeline

- **darktable position:** order 28.5 in the v5.0 order (`src/common/iop_order.c`), immediately after colorin (28.0) and first among the operations sharing 28.5 (diffuse, censorize, negadoctor, blurs, primaries), before nlmeans (29), colorchecker (30), color balance rgb (41.5) and the tone mappers (sigmoid 45.3, AgX 45.5, filmic rgb 46). colorin outputs Lab; darktable converts it to the working space before this module, which works in RGB (`default_colorspace` is `IOP_CS_RGB`).
- **Apertrust slot:** the working-space stage, order 28.5, darktable's: after colorin and the tone equalizer, at darktable's camera RGB 24.0 (decision 22), and before color balance rgb and AgX (`docs/engine.md`, Order). It is in `OPERATIONS` with `default_enabled` on: every colour raw runs it (decision 27), with temperature's default preset "as shot to reference" (`docs/ports/temperature.md`), and the exit test writes the same instance into darktable's history.
- **Domain:** scene-referred, unbounded, linear.
- **Input and output:** working RGB, linear Rec2020 in and out. Its XYZ is darktable's working profile's: Rec2020 primaries with white (0.31271, 0.32902), Bradford-adapted by lcms to the ICC D50 (`work_profile->matrix_in` and `matrix_out`; Apertrust's `WorkingProfile::linear_rec2020()`). All adaptation targets D50, the pipeline's white.
- **Assumes earlier modules:** demosaic; colorin to the working space; for an illuminant taken from the camera, the image at the camera's D65 reference, which colorin's late correction gives (below). Alpha passes through.
- **Layer support:** with layers (Phase 4); the module is background only until then (`docs/modules/colour-calibration.md`). A local white balance is this operation's CAT on a layer. A layer's instance runs after the background's at the same position (`docs/layers.md`, Sequencing), so it sees an image already adapted to D50; a layer's setting is therefore written against the background's illuminant (Tool binding notes, Layers).
- **Where the grey runs:** at darktable's position, in the background instance, as built. There the grey output at 28.5 makes the image monochrome before every layer's Colour calibration instance at that position and before the range masks, which are evaluated "scene-linear after colour calibration" (`docs/layers.md`), so hue and chroma masks would see grey. With layers (Phase 4) the choice is between two shapes, the kernel the same in both:
  - a second, background-only instance of this operation for the grey, adaptation XYZ, after every layer's instance and after the mask reference point: an exception to decision 34, and the likelier choice, since darktable's film presets are made for that shape;
  - the masks measured before the grey output.

### How it meets temperature and colorin

In darktable's scene-referred workflows (the default is "scene-referred (sigmoid)"; `dt_is_scene_referred()` is true for the three scene-referred choices), for a raw with a colour matrix:

1. **temperature** applies the as-shot coefficients, with preset "as shot to reference" (`DT_IOP_TEMP_D65_LATE`, `temperature.c` `reload_defaults`), so demosaic and highlight reconstruction work on a balanced mosaic.
2. **colorin** multiplies camera RGB by the camera's D65 multipliers over the as-shot ones (the late correction, decision 21, `docs/ports/colorin.md`), then converts to the working space. The image now has the colours the camera's D65-referred matrix gives it: a surface that was neutral under the scene's light carries that light's colour.
3. **color calibration** is enabled by the auto-applied preset "scene-referred default" (`init_presets`; format `FOR_MATRIX`, which `develop.c` sets for raw images that are not monochrome). Its parameters are `reload_defaults`': CAT16, the illuminant read from the camera's as-shot coefficients (Defaults), identity mixer, gamut compression 1, clipping on. It adapts that light to D50.

darktable shares the state between these through `dev->chroma` (`wb_coeffs`, `as_shot`, `D65coeffs`, `late_correction`). Apertrust has it already in the image description (decision 21): after colorin, `ImageDesc::white_balance` holds the multipliers the image now carries (the D65 ones after a late correction) and `late_correction` is false. With the camera's metadata in the image context (as-shot multipliers, the D65 matrix, from which `apertrust_color::daylight_multipliers` gives the D65 ones), `prepare` can compute everything darktable reads from `dev->chroma` in every state darktable reaches; in one state it cannot reach, the result differs (Setup, 7). No interface change is needed for that.

The trouble messages ("white balance applied twice", "double CAT applied", "white balance missing") and `_declare_cat_on_pipe` are GUI: they warn when temperature is not at the reference while a CAT runs (`_dev_is_D65_chroma`: with the late correction the as-shot coefficients, else the D65 ones, within 1e-5), when two instances adapt, or when white balance is off. Since White balance drives temperature as darktable's module does (decision 27), the first can happen. The first pass is to show it as darktable does (planned): "white balance applied twice (details)" on White balance and "white balance module error (details)" on Colour calibration, computed by the session from both modules' values. The other two cannot happen: one instance, and White balance has no off switch.

The pipeline exit test (decision 62) writes Apertrust's parameters into darktable's history, with temperature "as shot to reference" on both sides, so both sides run the same instance (decision 27).

## Parameters

darktable params version 3 (`DT_MODULE_INTROSPECTION(3, …)`), 160 bytes. `CHANNEL_SIZE` is 4, so each array has a fourth element that nothing reads; Apertrust's schema names the first three with the `_<index>` suffix (decision 21). `gboolean` and enums are `i32`.

| Name | darktable field (offset) | Type | Range | Default | Owner | Meaning |
| --- | --- | --- | --- | --- | --- | --- |
| Red row | `red[4]` (0) | f32 ×3 | −2 to 2 | 1, 0, 0 (`init`) | Colour calibration (R tab) | Output channel 1 of the mixer, in the adaptation space: `out₁ = red · in` |
| Green row | `green[4]` (16) | f32 ×3 | −2 to 2 | 0, 1, 0 | Colour calibration (G tab) | Output channel 2 |
| Blue row | `blue[4]` (32) | f32 ×3 | −2 to 2 | 0, 0, 1 | Colour calibration (B tab) | Output channel 3 |
| Colourfulness | `saturation[4]` (48) | f32 ×3 | −2 to 2 (GUI soft −1 to 1) | 0 | Colour calibration (Colourfulness tab) | Per input channel, pushes ratios away from (positive) or towards (negative) grey |
| Brightness | `lightness[4]` (64) | f32 ×3 | −2 to 2 (soft −1 to 1) | 0 | Colour calibration (Brightness tab) | Per input channel, scales the pixel by `1 + Σ light·c / mean(c)` |
| Grey | `grey[4]` (80) | f32 ×3 | −2 to 2 (soft 0 to 1) | 0 | Colour calibration (Grey tab) | Weights of the monochrome output; any non-zero weight turns grey output on |
| Normalise rows | `normalize_R`, `_G`, `_B` (96, 100, 104) | bool | | false | as the rows | Divide each row by its sum |
| Normalise colourfulness | `normalize_sat` (108) | bool | | false | as colourfulness | Subtract the mean so grey and average colours keep their colourfulness |
| Normalise brightness | `normalize_light` (112) | bool | | false | as brightness | Subtract the mean |
| Normalise grey | `normalize_grey` (116) | bool | | false (true in every monochrome preset) | Colour calibration (Grey tab) | Divide the weights by their sum, unless it is 0 |
| Illuminant | `illuminant` (120) | choice: `pipe` 0, `A` 1, `D` 2, `E` 3, `F` 4, `LED` 5, `BB` 6, `custom` 7, `camera` 10; hidden `detect_surfaces` 8, `detect_edges` 9 | | `D` (introspection); the camera's (Defaults) | Colour calibration (CAT) | How the illuminant's chromaticity is found |
| Fluorescent | `illum_fluo` (124) | choice F1 to F12 | | F3 | Colour calibration (CAT, F source) | The CIE fluorescent illuminant, for `F` |
| LED | `illum_led` (128) | choice B1 to B5, BH1, RGB1, V1, V2 | | B5 | Colour calibration (CAT, LED source) | The CIE LED illuminant, for `LED` |
| Adaptation | `adaptation` (132) | choice: linear Bradford 0, CAT16 1, non-linear Bradford 2, XYZ 3, none (bypass) 4 | | CAT16 | Colour calibration (CAT) | The space the von Kries gains apply in, and the mixer's |
| Illuminant x, y | `x`, `y` (136, 140) | f32 | none in darktable; Apertrust's x −0.1 to 1.2, y 0.0001 to 2, the reach of darktable's hue and chroma sliders (decision 27), and `prepare` keeps x + y at most 0.9999 (Design choices, 3) | 0.333, 0.333; the camera's | Colour calibration (CAT, through hue and chroma) | CIE 1931 2° chromaticity, for `custom`, and the fallback of `camera` |
| Temperature | `temperature` (144) | f32, K | 1667 to 25000 (GUI soft 3000 to 7000) | 5003; the camera's CCT | Colour calibration (CAT) | For `D` and `BB` |
| Gamut compression | `gamut` (148) | f32 | 0 to 12 (soft max 4) | 1 | Colour calibration (CAT) | Strength of the chromaticity compression; 0 is off |
| Clip | `clip` (152) | bool | | true | Colour calibration (CAT) | "Clip negative RGB from gamut": clamps negatives at every step |
| Version | `version` (156) | choice: `V_1` 0 "version 1 (2020)", `V_2` 1 "version 2 (2021)", `V_3` 2 "version 3 (Apr 2021)" | | `V_3` | Colour calibration, fixed at `V_3` with no control: darktable shows it only for an older edit (`docs/modules/README.md`, rule 12) | The colourfulness algorithm; `V_1` and `V_2` exist for old edits only |

The fluorescent and LED chromaticities are the tables at the top of `illuminants.h`; `A` is (0.44757, 0.40745), `E` (1/3, 1/3), `pipe` darktable's D50 (0.34567, 0.35850).

The GUI shows the custom illuminant as hue and chroma (CIE Luv relative to D50 at L 100, `dt_xyY_to_Lch`, `_illum_xy_callback`), and the colourfulness, brightness and grey arrays as "input R, G, B": they act on the adaptation space's channels, CAT16's LMS by default, not on working RGB.

## Parameter versions

| Version | Change | Migration (`legacy_params`) |
| --- | --- | --- |
| 1 (2020) | 156 bytes, the version 3 struct without `version`; `normalize_grey` had no effect, because `commit_params` always normalised the grey weights | To version 3: copy, `normalize_grey = TRUE`, swap `saturation[0]` and `saturation[2]`, `version = V_1` |
| 2 (2021) | Same 156-byte layout; `normalize_grey` honoured | To version 3: copy, swap `saturation[0]` and `saturation[2]`, `version = V_1` |
| 3 (Apr 2021) | Adds `version`, so the colourfulness algorithm can change without a new struct | |

Both migrations set `V_1`, never `V_2`. New edits get `V_3`; `V_2` is reachable only from the GUI's "saturation algorithm" combobox, shown for edits not at `V_3`. `commit_params` swaps the two saturation coefficients back for `V_1`, so the migrated edit renders as before.

Every darktable sidecar that holds this module stores version 3 (Golden cases), so `ports.rs` needs no migration. Apertrust's schema starts at version 1 with version 3's fields.

## Defaults

`init` sets the mixer to the identity; the rest are introspection defaults: illuminant `D` at 5003 K, x and y 0.333, F3, B5, CAT16, gamut 1, clip on, version `V_3`, every normalisation off.

`reload_defaults` (`default_enabled` is false: the preset enables it):

1. If another instance already registered as the CAT (`dev->chroma.adaptation`, the first instance in the pipe), or the image is monochrome: illuminant `pipe`, adaptation none. darktable makes later instances plain channel mixers this way.
2. Otherwise CAT16, and, when `_get_white_balance_coeff` succeeds (a raw with a matrix): the illuminant from the camera (`find_temperature_from_raw_coeffs`, Setup, 7) in `x`, `y`, with illuminant `camera`, when the camera gives one; then, whether it did or not, `_check_if_close_to_daylight` on `x`, `y` (still the defaults 0.333, 0.333 when the camera gave none) replaces the illuminant with `D`, `BB` or `custom` and sets `temperature` to the CCT:
   - `t = xy_to_CCT(x, y)` (Hernández-Andrés, Lee and Romero 1999, a sum of three exponentials in `n = (x − 0.3366) / (y − 0.1735)`); between 1667 and 3000 K, where that fit fails, `CCT_reverse_lookup` instead: the nearest of 65,536 points, spaced by the fourth power from 1667 to 25000 K, on the daylight locus from 4000 K and the Planckian below, by Euclidean distance in xy.
   - The distances from (x, y) to the daylight model and to the Planckian model at `t`, in darktable's `xy_to_uv`, which is Judd's 1935 UCS scaled by 12 (with 1.882 where Judd's 0.15735 × 12 is 1.8882), not CIE 1960 as its comment says.
   - Under 0.005: `D` or `BB`, whichever is nearer, a tie going to `D`; otherwise `custom`. Below 4000 K the daylight model falls through to the Planckian one (Setup, 6), so the two distances tie and `D` wins, which is why the test sidecars store `D` at 3927.1 K. The adaptation is set to CAT16 in every case.

The temperature it stores is that CCT, relative to the pipe's D50 (Setup, 7). So darktable's default never keeps `camera`. With `D` or `BB` the render uses the model's chromaticity at the CCT, not the camera's: for the camera chromaticities the test sidecars store, this moves the white by up to 0.0046 in darktable's UCS (on mire1.cr2 the camera's (0.38197, 0.36999) becomes the Planckian (0.38382, 0.37878) at 3927 K). On a grey at Y 0.18 under CAT16, before the tone mapper, that is an estimated ΔE2000 of 3.6 on mire1.cr2, 1.9 on hlrecovery.arw and 3.5 on xtransIV.raf against the camera's own chromaticity: visible, and above the 2.3 the tolerances use. Apertrust follows darktable, so an unedited photo matches darktable's unedited photo.

For an image that is not a raw, or a monochrome raw, darktable does not enable the module at all (`FOR_MATRIX`).

**Apertrust's defaults** (`Operation::defaults`, `image_defaults` in `src/setup.rs`) are `reload_defaults`' (`channelmixerrgb.c` at the reference release, lines 3862 to 3906, with `_get_white_balance_coeff`, 614 to 644, and `_check_if_close_to_daylight`, 1216 to 1286; in `illuminants.h`, `find_temperature_from_raw_coeffs`, 389 to 454, `illuminant_to_xy`, 224 to 322, `xy_to_CCT`, 119 to 134, `xy_to_uv`, 500 to 509, and `CCT_reverse_lookup`, 540 to 580).

A colour raw (darktable's `dt_image_is_matrix_correction_supported`: a raw that is not monochrome) stores in `x`, `y` the camera's chromaticity, computed as darktable does for an image at the camera's reference (`custom_wb` 1, as "as shot to reference" leaves it), or, without one (no matrix, or as-shot multipliers that are not normal), darktable's stored 0.333, 0.333; `_check_if_close_to_daylight` then describes that light, `D` or `BB` at its correlated colour temperature where either lies within 0.005 in darktable's UCS, which the render then reads instead of `x`, `y`, `custom` otherwise (0.333, 0.333 is `custom` at about 5491 K), under CAT16.

The restatement is darktable's arithmetic, in `f32` as darktable's, but for `xy_to_uv`'s one `f64` sum, darktable's `double` literal (Precision and special values); the loci, the fit and the reverse lookup stay in `apertrust-color` (`illuminants::closest_cct_model`), where the module's follow writes take them too. The temperature is held to the schema's 1667 to 25000 K (Design choices, 5), which changes only a `custom` light's, since `D` or `BB` is chosen only within their loci's range. The rest is `init`'s and the introspection defaults.

A monochrome raw gets darktable's plain channel mixer (illuminant `pipe`, adaptation none). Any other image keeps the introspection defaults. On a monochrome raw or an image that is not a raw, colour calibration is off by default, as darktable leaves it, and switched (decision 32): an unedited photo runs only the operations whose manifest's `default_enabled` is set, and a switched module turns its operation on once a control is written (decision 28). That is built when Apertrust first opens such an image; every image it opens today is a colour raw. These defaults are every binding's `base`, and what an unedited photo renders.

The snap moves the white on 22 of the 29 samples the exit test compares, by up to 0.005 in darktable's UCS, as in darktable (Results, The defaults).

The presets (`init_presets`, all at version 3): "monochrome | luminance-based" (grey 0, 1, 0), "monochrome | ILFORD HP5+" (0.25304098, 0.25958747, 0.48737156), "ILFORD DELTA 100" (0.24552374, 0.25366007, 0.50081619), "ILFORD DELTA 400 - 3200" (0.24376712, 0.23613559, 0.52009729), "ILFORD FP4+" (0.24149085, 0.22149272, 0.53701643), "Fuji Acros 100" (0.333, 0.313, 0.353), each with illuminant `pipe`, adaptation XYZ, clip on, gamut 0, `normalize_grey` on; "basic channel mixer" and three "channel swap" presets with adaptation none and normalised rows. `init_presets` builds them from a no-op starting point (illuminant `pipe`, adaptation XYZ, gamut 0, clip off, identity mixer, zero arrays) that no preset keeps as it is: the monochrome ones turn clip on.

## Setup on the CPU (`commit_params`)

This becomes `prepare` in `crates/op/channelmixerrgb/src/setup.rs`.

1. **Mixer.** `MIX[0] = red / ΣR` when `normalize_R`, else `red`; likewise green and blue. darktable divides by a zero sum without a guard.
2. **Colourfulness:** `sat_c = −saturation_c + n_s`, with `n_s` the mean of the three when `normalize_sat`, else 0. For `V_1`, `sat_0` and `sat_2` take each other's source. The sign is inverted: a positive slider gives a negative coefficient, which moves ratios away from grey.
3. **Brightness:** `light_c = lightness_c − n_l`, with `n_l` the mean when `normalize_light`.
4. **Grey:** `apply_grey` when any weight is non-zero; `grey_c = grey_c / Σgrey` when `normalize_grey` and the sum is not 0.
5. **Gamut compression:** `compression = 0` when `gamut = 0`, else `1 / gamut`.
6. **The illuminant's chromaticity** (`illuminant_to_xy`, with the image and `custom_wb` from step 7), falling through as darktable's `switch` does:
   - `pipe`: (0.34567, 0.35850). `E`: (1/3, 1/3). `A`: (0.44757, 0.40745). `F`, `LED`: the tables.
   - `D`: `CCT_to_xy_daylight(t)`, the CIE daylight locus, `x` a cubic in `1/t` with one set of coefficients from 4000 to 7000 K and another to 25000 K, `y = −3x² + 2.87x − 0.275`. Below 4000 K `x` is 0, and `D` falls through to `BB`. At 4000 K the two loci are 0.007 apart in y (daylight (0.38234, 0.38377), Planckian (0.38053, 0.37673)), so a `D` temperature crossing 4000 K jumps.
   - `BB`: `CCT_to_xy_blackbody(t)`, Kim et al.'s cubic splines of the Planckian locus, two pieces for `x` (split at 4000 K), three for `y` (split at 2222 and 4000 K), valid from 1667 to 25000 K; outside, it falls through to `camera`.
   - `camera`: the camera's illuminant (step 7); if that fails, the stored `x`, `y`.
   - `custom`, `detect_surfaces`, `detect_edges`: the stored `x`, `y`.
7. **The camera's illuminant** (`_get_white_balance_coeff`, `find_temperature_from_raw_coeffs`, `WB_coeffs_to_illuminant_xy`), for a raw with a colour matrix and three normal, non-zero as-shot coefficients:
   - `custom_wb = 1` when the pipe is at the camera's reference (`_dev_is_D65_chroma`: temperature's coefficients equal the as-shot ones with the late correction, or the D65 ones without it, each within 1e-5, `dt_dev_equal_chroma`); otherwise `D65coeffs / wb_coeffs` when the D65 coefficients are valid and some applied coefficient exceeds 1, else 1. darktable's `wb_coeffs` are temperature's coefficients, before colorin's late correction.
   - In Apertrust, after colorin, `desc.white_balance` holds `wb ∘ D65 / as_shot` with the late correction and `wb` without it, and `late_correction` is false either way, so `prepare` cannot tell which happened. It uses `custom_wb = D65 / desc.white_balance`, 1 when the two agree within 1e-5, with darktable's guard that some coefficient exceed 1. That equals darktable's in every state darktable reaches: the late correction only with temperature at the as-shot coefficients (its GUI switches the preset to "user defined" as soon as a coefficient changes), or no late correction. With the late correction and other coefficients, which Apertrust can reach (`apertrust-cli edit --set temperature.red=…`, which applies no follow writes, keeps the preset; the app and the session's commands write user modified, as darktable's GUI does), darktable's formula would give `WB = as_shot ∘ D65 / wb`, and Apertrust's gives `as_shot² / wb`, the chromaticity the image then carries. Listed under Deliberate differences; following darktable instead would mean carrying the late correction past colorin in the image description, an interface change.
   - `WB = as_shot ∘ custom_wb` (darktable's `img->wb_coeffs`, not normalised; the scale cancels).
   - `CAM_to_XYZ`: the camera's XYZ-to-camera matrix (the DNG's embedded one first, as colorin; Apertrust's `Camera::xyz_to_camera`), inverted by `matrice_pseudoinverse`, a Gauss–Jordan solve of the normal equations in `f32` without pivoting.
   - `XYZ = CAM_to_XYZ · (1 / WB)`, the colour the as-shot coefficients neutralise, under the matrix's D65.
   - D65 to D50 with darktable's linear Bradford: `LMS = XYZ_to_Bradford · XYZ`, `LMS ∘ D50_B / D65_B` with D65_B = (0.941238, 1.040633, 1.088932) and D50_B = (0.996078, 1.020646, 0.818155), back with `Bradford_to_XYZ`, whose constants have 4 decimals and are not the exact inverse (chromatic_adaptation.h).
   - `x = X / (X + Y + Z)`, `y = Y / (X + Y + Z)`.

   With the late correction this is the as-shot illuminant as it appears in the pipe; without it, `custom_wb` turns it into D50 as seen through temperature's balance. For `camera`, `commit_params` also forces the adaptation to CAT16 (`_check_if_close_to_daylight`, which computes a CCT only to throw it away).

   The chromaticity is relative to the pipe's D50 white, not the scene's light: the camera matrix's D65 has become D50, so a scene lit by D65 gives D50's chromaticity, and a CCT of about 5000 K. Every illuminant this module stores or shows is in these terms, the `D` and `BB` temperatures included: the camera chromaticity the test sidecars store for mire1.cr2 reads 3927 K here and about 4860 K once taken back through the same Bradford from D50 to D65 (the camera records 5405 K by its own estimate), and `D` at 6500 K describes a light much bluer than daylight.
8. **The illuminant in the adaptation space:** `XYZ_i = (x / y, 1, (1 − x − y) / y)` (`illuminant_xy_to_XYZ`), `LMS_i = convert_any_XYZ_to_LMS(XYZ_i, kind)`: Bradford's matrix for both Bradfords, CAT16's for CAT16, the identity for XYZ and none.
9. **Non-linear Bradford's exponent:** `p = (0.818155 / LMS_i.z)^0.0834`, computed for every adaptation and used only by non-linear Bradford. It is the reciprocal of CIECAM97s's `(B_w / B_wr)^0.0834`; Apertrust follows darktable.
10. **Matrices** (`_loop_switch`, per adaptation; `RGB_to_XYZ` is the working profile's, products in `f32` as `dt_colormatrix_mul`):

    | Adaptation | Into the adaptation space | `MIX_to_XYZ` | After gamut mapping, XYZ to | Back to XYZ |
    | --- | --- | --- | --- | --- |
    | Linear Bradford | `XYZ_to_Bradford · RGB_to_XYZ` | `Bradford_to_XYZ · MIX` | `XYZ_to_Bradford` | `Bradford_to_XYZ` |
    | CAT16 | `XYZ_to_CAT16 · RGB_to_XYZ` | `CAT16_to_XYZ · MIX` | `XYZ_to_CAT16` | `CAT16_to_XYZ` |
    | Non-linear Bradford | `RGB_to_XYZ`, then `XYZ_to_Bradford` per pixel | `Bradford_to_XYZ · MIX` | `XYZ_to_Bradford` | `Bradford_to_XYZ` |
    | XYZ | `RGB_to_XYZ` | `MIX` | identity | identity |
    | None (bypass) | none: no adaptation | `RGB_to_XYZ · MIX` | `XYZ_to_RGB` | `RGB_to_XYZ` |

    The CAT16 constants have six decimals and the Bradford ones four, so a round trip through either is the identity only to about 4.8e-6 (CAT16) or 1e-4 (Bradford). The port uses darktable's constants, not exact inverses. D50 in each space: CAT16 (0.994535, 1.000997, 0.833036), Bradford (0.996078, 1.020646, 0.818155), XYZ (0.9642119944211994, 1, 0.8251882845188288). These are not `XYZ_to_LMS · D50_XYZ`: CAT16's third differs by 4.2e-4 (0.833036 against 0.833457), Bradford's by up to 4.7e-4; and darktable's D50 Z, 0.825188, is not the ICC D50's 0.8249 that the working profile maps RGB (1, 1, 1) to. So illuminant `pipe` is not the identity under CAT16 or either Bradford (about 5e-4), which is why darktable's no-op starting point uses XYZ, and the illuminant's own colour comes out neutral only to a spread of R, G, B of about 4.1e-4 (XYZ), 6.3e-4 (CAT16) and 7.5e-4 (Bradford).

`process` repeats steps 6 to 8 for `camera`, because temperature may have changed after `commit_params`; it does not recompute `p`. Apertrust's `prepare` runs whenever the image description changes, so it needs no second copy.

## Per pixel (`process`, `_loop_switch`)

`min` is 0 with clip on and −FLT_MAX with it off; "clip" below means `max(v, min)` (`dt_vector_max_nan`, `dt_vector_clipneg_nan`).

1. `rgb = max(in, min)`.
2. **Adapt** (`_loop_switch`, first `switch`):
   - CAT16, linear Bradford: `lms = RGB_to_LMS · rgb`; `lms = lms ∘ D50 / LMS_i` (`CAT16_adapt_D50` with full adaptation, `bradford_adapt_D50` with `full` false).
   - Non-linear Bradford: `xyz = RGB_to_XYZ · rgb`, `Y = xyz.y`; `lms = XYZ_to_Bradford · xyz`; divide by `Y + ε` when `Y > ε`, else by `ε` (`downscale_vector`, `ε` = `NORM_MIN` = 2⁻¹⁶); `t = lms / LMS_i`; `t.z = t.z^p` when `t.z > 0`; `lms = D50 ∘ t`; multiply back by `Y + ε` or `ε` (`upscale_vector`).
   - XYZ: `lms = (RGB_to_XYZ · rgb) ∘ D50 / XYZ_i`.
   - None: `lms = rgb`.
3. **Mix:** `xyz = MIX_to_XYZ · lms`.
4. Clip.
5. **Gamut mapping** (`_gamut_mapping`), in XYZ whatever the adaptation:
   - `s = X + Y + Z`; `(x, y) = (X, Y) / s` when `s > 0`, else D50's (0.34567, 0.35850); `Y` keeps the pixel's luminance.
   - To CIE 1976 u′v′ (`dt_xyY_to_uvY`); `δ = D50_uv − uv` with D50_uv = (0.20915914598542354, 0.488075320769787); `Δ = Y · |δ|²`.
   - `k = Δ^compression`, or 0 when compression is 0; `uv' = uv + k δ`, each component clamped so it does not pass D50's (`max(·, D50)` from above, `min(·, D50)` from below).
   - `Δ` is negative when `Y` is, which only happens with clip off. `powf` then gives a real `k` for a whole-number exponent: at gamut 1, the default, `k = Δ < 0`, which moves uv away from D50, a direction the clamp allows; at an even exponent (gamut 1/2, 1/4…) `k > 0`. For any other exponent `powf` gives NaN, C's `fmaxf` and `fminf` then return D50 for both components, and the pixel turns neutral, though under `-ffast-math` even that is not guaranteed.
   - Back to xy; with clip on, `x, y ≥ 0`; `y ≥ ε`; when `x + y ≥ 1`, both divided by `x + y`; `XYZ = (Y x / y, Y, Y (1 − x − y) / y)`.

   The compression grows with luminance: at gamut 1 (the default) a saturated colour at Y 1 moves a few percent towards white, at Y 10 several times more, and fully white once `k ≥ 1`. With compression 0 the step still rescales `x + y ≥ 1` (negative Z) and floors `y`.
6. Back to the adaptation space (`convert_any_XYZ_to_LMS`, or `XYZ_to_RGB` for none); clip.
7. **Colourfulness and brightness** (`_luma_chroma`), on `c` in the adaptation space, `V_3`:
   - `n = max(‖c‖, ε)`, `a = max(mean(c), ε)`, `m = c · light`.
   - `r = c / (n / √3)`: ratios, (1, 1, 1) for a grey `c`.
   - `κ = (r · sat) / 3`; `r' = max(r + κ (1 − r), min(r, 0))`, so a ratio that was negative may stay negative and one that was not cannot become so.
   - `out = r' · (n / √3) / (max(‖r'‖, ε) / √3) · max(1 + m / a, 0)`: the norm of `c` is kept, then scaled by brightness.
   - `V_1` and `V_2` take `r = c / n` and skip the norm compensation: `out = r' · n · max(1 + m / a, 0)`. `V_2` keeps `κ = (r · sat) / 3`; `V_1` uses `κ = Σ (1 − r_c)² sat_c`. The `else` branch for black is unreachable, because `n` and `a` are floored at `ε` before the test. A non-zero `c` whose norm is below `ε` (2⁻¹⁶) comes out with its norm raised to about `ε`.
8. Clip.
9. **Grey** when `apply_grey`: `g = max(out · grey, 0)`, and the pixel becomes (g, g, g) in working RGB directly, without converting back. Otherwise: back to XYZ (`convert_any_LMS_to_XYZ`, or `RGB_to_XYZ` for none), clip, `XYZ_to_RGB`, clip.
10. Alpha is copied.

With darktable's default settings (identity mixer, zero colourfulness and brightness, no grey) steps 7 and 9 change nothing, but the clips and gamut compression at 1 still act: the default render is the CAT plus a luminance-weighted desaturation of bright saturated colours and the removal of negative components.

## Illuminant detection, picker and colour checker

None of these is per-pixel processing; each produces parameters.

- **Detection** (`detect_surfaces`, `detect_edges`, `_auto_detect_WB`): compiled out in the reference release. `AI_ACTIVATED` is commented out ("AI feature not good enough so disabled for now"), so the two modes have no description, are not offered, and render as `custom`. The code: per-pixel xy relative to D50 over the whole image, run on the full pipe only (`dt_pipe_is_full`, the darkroom's main view, at that view's scale); on a grid with a 4-pixel step and 4-pixel offsets, a 3×3 B-spline average; "surfaces" weighs each patch's average chromaticity by the product of the x and y variances and their covariance, "edges" averages the negated direction of the chromaticity's Laplacian, each normalised by a Minkowski p = 8 norm; the result is an x, y, which `_check_if_close_to_daylight` turns into `D`, `BB` or `custom`. The offsets are in pixels, so the estimate depends on scale.
- **Picker** (`_auto_set_illuminant`, "correction" mode): the user picks an area of the module's input; with the target L, C, h (by default neutral), `LMS_i = D50_LMS ∘ LMS_input / LMS_target`, both Y-normalised in the adaptation space, the target first passed through the inverse mixer when "take channel mixing into account" is on; the result becomes a `custom` x, y. "Measure" mode only reports the colour after adaptation. A closed form, cheap on the CPU once the area's mean is known.
- **Colour checker** (`_extract_color_checker`): the user frames a chart (seven are known, X-Rite and Datacolor); patch means are taken through a homography with a safety margin, normalised for exposure and black offset; the illuminant comes from the grey patch; the 3×3 mixer from a weighted least-squares fit of measured to reference LMS, with eight weightings (none, neutral, saturated, skin, foliage, sky, average ΔE, maximum ΔE), solved in `f64` (`pseudo_solve_gaussian`); CIEDE2000 before and after is reported. It writes `custom` x, y and the three rows.

**What the CAT tab needs:** the picker, darktable's, in its readout row and its area colour mapping group (decision 27). darktable has no working automatic estimate to port, and its CAT tab no Auto. The picker ends in ordinary `x`, `y` (principle 8), so it needs a statistic of the image back on the CPU: the picked area's mean is a readout (decision 19), built with the picker. The colour checker writes ordinary parameters, so adding it does not touch the kernel. It is planned before the end of Phase 3: darktable's colour checker ported from the reference release, its chart data and its fit, so the module is darktable's whole.

## What the port covers

- **Ported:** params version 3 with the `V_3` algorithm; all five adaptations; every illuminant type darktable offers (`pipe`, `A`, `D`, `E`, `F`, `LED`, `BB`, `custom`, `camera`); the mixer with its normalisations, colourfulness, brightness, grey; gamut compression; clip. The adaptations are one code path with different matrices (only non-linear Bradford adds a power), the illuminant types are CPU tables and formulas, and the continuous parameters cost nothing extra, as the skill's "What to port" asks.
- **`NotPorted`:** `V_1` and `V_2` (old edits only; decision 51); `detect_surfaces` and `detect_edges` (compiled out in the reference); a working space other than linear Rec2020 (colorin ports only that).
- **`camera` on a four-colour sensor:** darktable does not fall back; it checks four as-shot coefficients and computes from the matrix's three rows (test 0177 stores (0.37107, 0.38091)). In Apertrust temperature's `NotPorted` stops such images earlier, so this operation never sees one.
- **`camera` without a matrix or valid as-shot coefficients:** the stored `x`, `y`, as in darktable; not an error.
- **Outside the operation:** the picker's solve (Colour calibration's, with the picker), the colour checker (planned before the end of Phase 3), the CCT readout (computed by the module from its values, `_update_approx_cct`). The picker's statistic, the picked area's mean, is a readout (decision 19), built with the picker.

## Neighbourhood and scale

- **Neighbourhood radius:** none; per pixel.
- **Scale dependence:** none. (The disabled detection uses fixed pixel offsets on whatever scale the darkroom's main view has; a port of it would scale them.)
- **Tile overlap needed:** 0.
- **Region-of-interest changes:** none (no `modify_roi_in`, `modify_roi_out` or `tiling_callback`).

## Precision and special values

- **fp64 in darktable:** none in the per-pixel path, which is `f32` throughout, so nothing in the kernel needs redesigning. On the CPU: the colour checker's least squares is in `f64` (CPU-only in Apertrust too, where `f64` is allowed; it is not a kernel); `xy_to_uv` multiplies `1.9116 * xy[1]` as a `double` literal, so that sum is computed in `f64` before rounding; `D50xyY` and `D65xyY` are `double` (`cmsCIExyY`), rounded to `f32` where used; `dev->chroma`'s `D65coeffs` and `as_shot` are `double`, divided in `f32`. Apertrust computes the illuminant in `f32` as darktable does, so the kernel's in-kernel conversion (GPU plan) and `prepare` agree; the one `f64` product in `xy_to_uv` is reproduced only where it can decide darktable's 0.005 threshold (Defaults).
- **Divisions:** the illuminant's `y` (darktable has no guard; Apertrust's range keeps it above 0), `LMS_i` (no guard in darktable; it reaches 0 only for chromaticities outside any real light, but Bradford's second cone responds negatively to real blue and violet light, about −3.2 at 450 nm, and its third to real yellow and red light), a normalised row's or the grey weights' sum (the rows unguarded, grey guarded against 0), `y` in gamut mapping (floored at `ε`), the norms (floored at `ε`), `mean(c)` (floored).
- **Powers:** `Δ^compression` with `Δ < 0`, possible only with clip off and a negative luminance, has two outcomes in darktable (Per pixel, 5): a real power for a whole-number exponent, gamut 1 included, and D50's chromaticity through NaN otherwise. The kernel reproduces both without computing a NaN: `prepare` passes whether `1 / gamut` is a whole number and whether it is even, and for `Δ < 0` the kernel takes `±|Δ|^compression` or sets uv to D50 with a `select`. The `channelmixerrgb-unclipped` case (gamut 4) takes the second path for every pixel whose luminance is negative after the mix (Results). Non-linear Bradford's power only applies to a positive base, and darktable's exponent `p` is NaN for an illuminant whose third cone responds negatively.
- **NaN and infinity:** darktable's `fmaxf` turns a NaN into 0 with clip on and into −FLT_MAX with it off, so its output can be finite after a NaN or an infinity inside. Apertrust cannot rely on that (AGENTS.md, Kernels): the kernel's guards (Deliberate differences) keep every value it computes finite for any input and any setting the schema allows, which `every_value_stays_finite_on_extreme_input` checks on the kernel's maths restated, not only on its output.
- **Constants followed as the reference:** darktable's non-linear Bradford uses the reciprocal of CIECAM97s's exponent, and its camera illuminant uses its own rounded Bradford constants while colorin uses lcms's; the port follows both.
- **Out-of-gamut input:** with clip on (the default) negative components are removed at six points, and negative chromaticities in gamut mapping; with clip off, gamut mapping still rescales chromaticities with negative Z and replaces those with a non-positive XYZ sum by D50's.

## CPU and OpenCL differences in darktable

- **`V_1` colourfulness:** the OpenCL kernel uses `dot(1 − r, sat)`, the CPU `Σ (1 − r_c)² sat_c`. Apertrust follows the CPU, and `V_1` is not ported anyway.
- **CAT16, linear Bradford and XYZ** divide by `Y + ε` before the adaptation and multiply after in OpenCL, the identity up to rounding; the CPU does not. OpenCL's CAT16 calls `CAT16_adapt_D50` with D 1 and `full` false, the same gains.
- OpenCL takes the dot products over four channels, with the fourth weights 0.
- Profiling, validation and detection run on the CPU only: `commit_params` turns OpenCL off for those frames.

## GPU plan

- **Crate and kernel:** `apertrust-op-channelmixerrgb` in `crates/op/channelmixerrgb`; `slang/channelmixerrgb.slang`, its WGSL in `generated/`, `src/setup.rs` (`Settings`, `CameraWhite`, `prepare`, `image_defaults`, `NotPorted`), `src/lib.rs` (manifest, schema, `ChannelMixerRgb` implementing `Operation`), the manifest's slot the working space at 28.5, `differentiable: true`, layers on. Licence header with `Copyright (C) 2010-2026  darktable developers (ported from src/iop/channelmixerrgb.c)`.
- **Shared maths in `apertrust-color`:** `src/adaptation.rs` (`Adaptation`, the Bradford and CAT16 matrices, D50 and Bradford's D65 in each space; `chromatic_adaptation.h`, 2020-2025) and `src/illuminants.rs` (`Illuminant`, the daylight and Planckian loci, the fluorescent and LED tables, `xy_to_CCT`, the reverse lookup, darktable's UCS, `illuminant_to_xy`, `matrice_pseudoinverse`, `WB_coeffs_to_illuminant_xy`, `find_temperature_from_raw_coeffs` and `_check_if_close_to_daylight`; `illuminants.h`, 2020-2024), each file with darktable's copyright line. The kernel side, in a section at the end of `slang/color.slang`, holds only what the kernel uses: `NORM_MIN`, D50 in xy and u′v′, `illuminant_xy_to_xyz`, `away_from_zero`, and `xy_to_uv` and `uv_to_xy` with guarded denominators. The cone spaces' matrices and D50 constants, the loci and the tables stay on the CPU, where `prepare` composes the matrices into the uniform's; no kernel differentiates against a temperature yet (Design choices, 1 and 2).
- **One per-pixel pass**, `main`, plus `channelmixerrgb_backward` and `channelmixerrgb_forward_diff`. A pure per-pixel function of the pixel and the parameters (decision 8), so it can later fuse with colorin before it and color balance rgb after it.
- **Parameters** at offset 48, 24 `float4`s, 384 bytes (`PARAMS_SIZE`): six matrices as three `float4` rows each (working RGB to where the gains apply; `MIX_to_XYZ`; XYZ to the cone space, for the illuminant and non-linear Bradford's pixels; after gamut mapping, XYZ to the space colourfulness works in and back; XYZ to working RGB); D50 in the cone space; `sat`, `light`, `grey`; the illuminant's `x`, `y`, `compression` and the clip floor (0 or −FLT_MAX); a `uint4` with the adaptation, `apply_grey` and what a negative `Δ` gives (D50, an odd or an even power). Non-linear Bradford's luminance comes from its first matrix, which is `RGB_to_XYZ` for it. No tables.
- **The illuminant inside the kernel:** `XYZ_i` from `x`, `y`, `LMS_i` with the XYZ-to-adaptation matrix held at least `ε` from 0 with its sign, the gains `D50 / LMS_i` and `p`, at the start of the per-pixel function, so that derivatives reach `x` and `y` (Gradients). It costs a few operations per pixel against a pass that is memory-bound. `prepare` resolves every illuminant type, `camera` included, to `x`, `y`.
- **Branches:** the adaptation and `apply_grey` are uniform, so their branches never diverge; clip is a uniform `min` (0 or −FLT_MAX, a finite value WGSL accepts), with no branch. The data-dependent branches (gamut mapping's clamps, rescale and negative `Δ`, non-linear Bradford's sign test, `downscale_vector`) become `select`s or `max`es.
- **Decision 10:** nothing writes a local at a run-time index. darktable's channel loops become `float3` operations, gamut mapping's per-component clamp a `select(uv > D50, max(t, D50), min(t, D50))`, the grey output `float3(g)`. The fluorescent and LED tables and the CCT search stay in Rust.
- **Derivative traps:** `pow0`, not `pow`, for `Δ^compression` (Δ is far below 1e-6 for near-neutral pixels) and for non-linear Bradford's power; the norms' `sqrt` caps its derivative near 0, and the norms are scaled by the largest component, whose own derivative cancels; no table, so no branch decides a table read.
- **Cost:** one full-image pass, memory-bound like any per-pixel pass: 3.7 to 4.6 ms at 10 MP on the M4 (median per pass, `run`), 11 to 12 ms at 26 MP.
- **Golden harness:** a `Case` per row of the test plan in `crates/golden/src/cases.rs`; `ports.rs` decodes version 3 (160 bytes) and resolves the illuminant with the crate's own setup, so the harness and `prepare` cannot disagree. Only the `camera` case needs the raw's metadata: `ports.rs` decodes the raw with rawler, takes the image context as the engine builds it, checks that the sidecar's temperature is "as shot to reference" at the as-shot multipliers, and passes the camera's D65 multipliers as what the image carries, as colorin leaves it.

## Deliberate differences

As built. None changes a golden case's render beyond rounding (Results).

- **Only `V_3`:** `V_1` and `V_2` return `NotPorted`. They exist for older darktable edits, which the import of darktable's edits will need (decision 51, planned).
- **No detection modes:** `detect_surfaces` and `detect_edges` return `NotPorted`; darktable compiles them out. So do fluorescent or LED indices past darktable's tables, which darktable would treat as `custom` and the schema cannot reach.
- **The illuminant's conversion moves into the kernel**, so gradients reach `x` and `y`: `XYZ_i`, `LMS_i`, the gains `D50 / LMS_i` and non-linear Bradford's `p` are computed per pixel from the uniform's `x`, `y`, in `f32` as `commit_params` does. `prepare` resolves every illuminant type, `camera` included, to `x`, `y`.
- **`camera` reads the pipe's white balance from the image description** instead of `dev->chroma`, with darktable's 1e-5 test for "at the reference" (`CameraWhite::custom_wb`), and sets CAT16 directly instead of through `_check_if_close_to_daylight`. Same result.
- **Guards darktable lacks,** so every value the kernel computes stays finite for any input and any setting the schema allows (Precision and special values). Each changes only values far beyond any photo's, where darktable's results are absurd, infinite or NaN; on the synthetic sweep with clipping, for test 0085's settings and the HP5+ preset in every adaptation and for blue light, none fires (`the_kernel_is_darktables_per_pixel_code`), and the golden cases' results are unaffected (Results). In `prepare`:
  - weights whose sum is within 1e-6 of their sizes added up, rounding noise around 0, are left unnormalised, as darktable leaves grey weights that sum to exactly 0; darktable divides the mixer's rows by any sum but 0, by zero for 0, so a row such as (1, −1, 1e-30) would give gains of 1e30;
  - `x` and `y` get ranges (darktable has none), the reach of darktable's hue and chroma sliders (decision 27) (chroma to 300 % gives x −0.050 to 1.096 and y 0.112 to 1.879), x −0.1 to 1.2 and y 0.0001 to 2, and a pair whose sum passes 0.9999 is scaled back onto it, so `Z` stays positive: from chroma 103.6 % at hue 79°, where darktable's own maths soon has a negative `Z`, the render is this guard's.

  In the kernel (`slang/channelmixerrgb.slang`, its constants):
  - the input is held within ±1e8, which also turns infinities finite (a NaN cannot be caught, AGENTS.md, Kernels); scene light's white is 1;
  - `LMS_i` is held at least `ε` (2⁻¹⁶) from 0, keeping its sign, so a real light whose cone response is negative keeps darktable's negative gain;
  - non-linear Bradford takes the linear adaptation (an exponent of 1) for an illuminant whose third cone responds negatively, where darktable's `p` is NaN, and holds the pixel's ratio to the illuminant on that cone below 1e8 before its power (a real colour's is within about 10);
  - XYZ is held within ±1e12 after the mix and after gamut mapping;
  - gamut mapping takes X + Y + Z as at least 1e-6 of |X| + |Y| + |Z|, and each u′v′ denominator as at least `ε` times the sum of its terms' sizes from 0, keeping its sign, so x, y, u′ and v′ stay bounded: only XYZ with negative components, with clipping off, comes near;
  - gamut mapping's power is never formed above 1, where the correction lands on D50 whatever its size, so it is held there at 1 (the same to an ulp); a negative `Δ`'s odd power, which moves away from D50, is held at most 2²⁰;
  - colourfulness's norms are scaled by the largest component, so their squares cannot overflow, and brightness's factor is held at most 1e6 (with clipping it is at most 13).

  A negative `Δ` in gamut mapping keeps darktable's outcomes (Precision and special values): `prepare` passes whether `1 / gamut` is a whole number and its parity, and the kernel takes the power's sign, or D50 where `powf` gives NaN, by a uniform choice, never computing a NaN.
- **The camera's illuminant with the late correction and temperature off the as-shot coefficients:** `prepare` gives the chromaticity the image carries, `as_shot² / wb`, where darktable's formula would give `as_shot ∘ D65 / wb` (Setup, 7). darktable cannot reach that state, and neither can an edit: White balance owns temperature and writes "user modified" with any coefficient, as darktable's GUI does. Only a test that sets the parameters directly reaches it.
- **The unreachable black branch of `_luma_chroma` is dropped.**
- **The reverse lookup's argmin below 3000 K, where darktable's binary rounds otherwise:** `CCT_reverse_lookup` takes the nearest of 65,536 samples of the loci, and near a light's temperature their distances differ by less than `f32`'s rounding of the loci's polynomials (on the Canon EOS M50, 1.4e-11 between neighbouring samples at 0.0031).

  darktable's release build, `-ffast-math`, evaluates those polynomials with fused multiply-adds and a reciprocal, which tips the argmin to the next sample on one of the 29 samples the exit test compares. The M50's default temperature is 2723.124 K here and 2723.264 K in darktable on the Apple M4, one sample's 0.14 K. Its illuminant, `D` and so the Planckian, is then 1.1e-5 apart in x and 9e-6 in darktable's UCS, a five-hundredth of the snap's threshold. The Nikon Z 6, the other sample below 3000 K, agrees to the printed digit.

  Apertrust restates darktable's source arithmetic, as for every other locus. The fast-math form (`mul_add` with `1 / t` in `cct_to_xy_daylight` and `cct_to_xy_blackbody`) reproduces both samples, but follows one build's rounding: the reciprocal without fused multiply-adds, what an x86-64 baseline fast-math build has, agrees with the source, 2723.1243 K, so darktable's own value depends on how it was built. Only the defaults and follow writes below 3000 K reach it; the exit test gives darktable Apertrust's temperature, so it is not in its numbers.
- **No trouble messages, CAT registration or multi-instance defaults:** GUI and `dev->chroma` bookkeeping; Apertrust's module ownership prevents the problems they report.
- **Arithmetic order:** the matrix products follow darktable's composition per adaptation, in `f32` as `dt_colormatrix_mul`; CAT16, linear Bradford and XYZ apply the gains as one product with `D50 / LMS_i` where darktable divides and multiplies; differences are `f32` rounding. Against darktable's per-pixel code restated in `f32` (`crates/golden/tests/channelmixerrgb.rs`), relative to the larger of the output and the input, the kernel agrees within 2.6e-6 with clipping on, and within 1.3e-3 for spectral blue and yellow light, whose cone gains reach 32 and change sign; with clipping off, out-of-gamut input can bring X + Y + Z near 0, where x and y grow large and the maths is ill-conditioned, and the two agree within 3.7e-3 (Apple M4). Where a guard fires, the comparison is with the kernel's guarded maths restated.

## Tool binding notes

**In the first pass** (decision 27), Colour calibration is darktable's color calibration with every tab and owns every parameter: the CAT tab's adaptation, illuminant, F and LED sources, temperature, hue and chroma (a view of `x` and `y`), gamut compression and clip, in darktable's order and shown as darktable's `_update_illuminants` shows them, with darktable's GUI writes that keep the light consistent across a mode change as follow writes, and the mixer rows, colourfulness, brightness, the grey weights and their normalisations, with darktable's monochrome, channel mixer and channel swap presets (`docs/modules/README.md`, `docs/modules/colour-calibration.md`).

- **Layers,** planned with them (Phase 4). A layer's instance runs after the background's at the same position and sees an image already adapted to D50 (Pipeline), so the same settings cannot mean the same thing there without a translation, while the `tool-bindings` rules ask that "a slider means the same on a layer as on the background" and that every control at zero is a no-op.
  - **A layer's setting is written against the background's illuminant:** `LMS_i = D50 ∘ LMS(the layer's setting) / LMS(the background's setting)`, stored as `custom` `x`, `y`. Temperature and tint then mean the scene's light on a layer too, and "the same as the background" is the zero. It is exact for the linear adaptations in the same space as the background's; non-linear Bradford does not compose this way.
  - **Gamut compression and clipping are off on layers,** or they would act twice.
  - **An untouched layer adds no instance.** No parameter setting is an exact no-op: darktable's no-op starting point (illuminant `pipe`, adaptation XYZ, gamut 0, clip off) still rescales colours with x + y ≥ 1, floors y at 2⁻¹⁶, gives a pixel with X + Y + Z ≤ 0 D50's chromaticity and raises norms below 2⁻¹⁶ (Test plan, Other tests: the no-op starting point). So an exact zero on a layer needs the instance left out of the plan, as `docs/layers.md` does for untouched modules (Sequencing: "only adjusted controls create instances") and decision 28 provides.

**Fixed:** `version` at `V_3`, owned by Colour calibration with no control. Every other parameter is Colour calibration's control, as it is: the mixer rows, colourfulness, brightness and grey, with their normalisations, and the CAT tab's, specified by decision 27 (`docs/modules/colour-calibration.md`). The CAT tab's `x` and `y` have widened ranges, which change none of the eleven golden cases' results.

## Gradients

- **Reachable through the kernel:** the illuminant `x`, `y` (through `XYZ_i`, the gains and non-linear Bradford's `p`); the nine mixer coefficients, after normalisation; colourfulness and brightness (three each); the grey weights, after normalisation; compression (`1 / gamut`).
- **Not reachable:** the illuminant type, the fluorescent and LED choices, the adaptation, clip, the normalisations and `version`, which are discrete. `temperature` reaches the image only through the CPU's daylight or Planckian model, so the binding chains it with that model's derivative.
- **Exports** (`channelmixerrgb_backward`, `channelmixerrgb_forward_diff`): `x`, `y`, in that order, for White balance matching; the binding turns them into gradients for temperature and tint with its model's 2×2 Jacobian. The interface returns at most four gradients per pixel, so the nine mixer coefficients and the three grey weights do not fit. Planned with reference matching in Phase 3: the derivative interface takes more than four parameters per pass.
- **With respect to the input pixel:** the matching slice (decision 1) needs dLoss/dInput through every operation between its parameters and the output, and this one sits between exposure and AgX, so exposure's gradient passes through it. The per-pixel function is differentiable in its input as in its parameters (the same kinks), and per pixel, so reverse mode needs no scatter. As built, `channel_mixer` takes the pixel differentiable, not `no_diff`, and both entry points pass it as a differentiable pair whose derivative they do not export yet; `channelmixerrgb_backward` can write dLoss/dInput per pixel once the derivative interface has a binding for it. That binding is planned with reference matching in Phase 3, input-derivative entry points for per-pixel operations first (`docs/ports/agx.md`, Gradients).
- **Non-smooth spots:** the six clips when clip is on; gamut mapping's clamp at D50, its `x + y ≥ 1` rescale, the `y` floor, the D50 fallback for a non-positive sum and the negative-`Δ` cases; colourfulness's `min(r, 0)` floor and brightness's `max(·, 0)`; the `ε` floors on the norms and the mean; non-linear Bradford's sign test and its `Y > ε` switch; grey's `max(·, 0)`; and the guards' clamps (Deliberate differences), each beyond any photo's values, where its derivative is 0 or the guarded value's. And one singular spot: at a neutral pixel `δ = 0`, and `Δ^(1/gamut)` with `Δ ∝ |δ|²` has a derivative with respect to the pixel's chromaticity, and so to `x` and `y`, that grows like `|δ|^(2/gamut − 1)`, unbounded when gamut exceeds 2. Per-pixel gradients there are large; the check's sum over the image should stay finite, but the synthetic sweep's grey column needs watching at gamut above 2.
- **Results** (Apple M4, the sum of the output's RGB as the loss, step 1e-3): every case passes for both, the bypass case aside, where no adaptation runs and the illuminant reaches nothing, so both derivatives are 0 and `ports.rs` names no parameter for it.

  | Case | x: relative error | y: relative error | Forward against reverse, per pixel | Synthetic sweep: x, y |
  | --- | --- | --- | --- | --- |
  | `channelmixerrgb` | 3.0e-5 | 2.0e-5 | ≤ 2.0e-7 | 4.7e-5, 4.6e-6 |
  | `-linear-bradford` | 1.2e-5 | 7.6e-6 | ≤ 1.8e-7 | 1.6e-5, 2.6e-5 |
  | `-bradford` | 1.5e-5 | 8.1e-6 | ≤ 1.8e-7 | 1.8e-5, 2.8e-6 |
  | `-xyz` | 7.4e-6 | 1.2e-5 | ≤ 2.0e-7 | 3.6e-5, 1.5e-5 |
  | `-default` | 4.0e-5 | 2.9e-6 | ≤ 5.4e-7 | 6.9e-5, 8.3e-6 |
  | `-daylight` | 2.3e-4 | 1.6e-5 | ≤ 1.1e-6 | 4.7e-5, 2.5e-5 |
  | `-highlights` | 2.0e-4 | 8.6e-6 | ≤ 1.3e-6 | 2.2e-3, 1.7e-5 |
  | `-grey` | 6.8e-5 | 1.5e-8 | ≤ 5.8e-7 | 2.7e-5, 1.6e-6 |
  | `-unclipped` | 3.2e-5 | 2.4e-5 | ≤ 1.8e-7 | 7.4e-3, 5.0e-3 |
  | `-camera` | 2.0e-4 | 2.4e-5 | ≤ 5.4e-7 | 4.2e-5, 1.5e-5 |

  Forward and reverse mode agree within 5.4e-7 per pixel on the sweep too. The largest gap, `-unclipped` on the synthetic sweep, sits in one column (`--explain`): hue 135° at saturation 1.25, an input with red at −0.25 of its level, where a step of x or y moves X + Y + Z after the mix across 0 (from 1.3e-3 to −2.6e-3 at level 1), so gamut mapping switches to its D50 fallback for a non-positive sum, a jump in darktable's maths with clipping off that finite differences straddle while both modes take the slope on one side. The kernel matches darktable's code restated on both sides of it.

## Test plan

### Golden cases

From darktable's integration tests (darktable-tests commit 3f2d1ab, the checkout `cases.rs` names): 57 sidecars have this module in their history (16 more name it only in their module order list); all store version 3 and none masks it. The tests the reference release pins (aaf8359) do not contain 0178 to 0189, and 0179 to 0181 use spektrafilm, which the reference release does not have; every case below exists at the pin. Grouped by their settings:

| Settings | Tests | Image |
| --- | --- | --- |
| CAT16, `D` at 3927.1 K (renders the Planckian model, below 4000 K), identity mixer, gamut 1, clip, `V_3` | 0083, 0084, 0086, 0089 to 0093, 0097, 0102, 0148, 0150 to 0152, 0154, 0155, 0157 to 0159 (temperature at the camera's D65 coefficients, no preset); 0160 to 0164, 0167, 0171, 0172, 0174 to 0176, 0178 ("as shot to reference") | mire1.cr2 (Canon 40D) |
| The same, `V_2`, disabled | 0082 | mire1.cr2 |
| CAT16, `BB` at 4555.4 K, otherwise default | 0094 to 0096, 0153, 0182 to 0189 (no preset); 0179 to 0181 ("as shot to reference") | hlrecovery.arw (RX100 III) |
| CAT16, `BB` at 3927.1 K | 0144 | mire1.cr2 |
| CAT16, `D` at 4422 K (the daylight model) | 0146 | xtransIV.raf |
| CAT16, `D` at 4284.5 K | 0177 | bayer4.crw (four-colour, skipped by the harness) |
| CAT16, `custom` (0.3892, 0.41662) | 0165, 0173 ("as shot to reference") | mire1-xtrans.raf (X-T4) |
| **CAT16, `custom` (0.30981, 0.32762), mixer rows (−0.969, 0.476, 0), (0, −0.479, 0), (0, 0, 1) normalised, colourfulness (0.215, −0.953, −0.544), brightness (0.184, −0.305, 0.138), gamut 1, clip, `V_3`** | **0085** | mire1.cr2 |
| The same with linear Bradford, non-linear Bradford, XYZ | 0126, 0127, 0128 | mire1.cr2 |
| Adaptation none, `pipe`, red row (1, 0.011, −0.012), colourfulness (0.229, 0.27, −0.237), brightness (0, 0.117, 0.096), `V_1` | 0103 (almost all modules) | mire1.cr2 |

No test uses grey, adaptation none with `V_3`, clip off, gamut other than 1, `camera`, `A`, `E`, `F` or `LED`. The cases, all with `Tolerance::DARKTABLE`; the edited ones change only the active channelmixerrgb entry:

| Case | From | What it exercises | `disable` |
| --- | --- | --- | --- |
| `channelmixerrgb` | 0085 | CAT16, custom illuminant, normalised mixer, colourfulness, brightness, gamut 1, clip | `filmicrgb` |
| `channelmixerrgb-linear-bradford` | 0126 | linear Bradford, same settings | |
| `channelmixerrgb-bradford` | 0127 | non-linear Bradford | |
| `channelmixerrgb-xyz` | 0128 | XYZ | |
| `channelmixerrgb-default` | 0174 | the scene-referred default chain: "as shot to reference", colorin's late correction in the input, CAT16, `D` falling back to the Planckian model, identity mixer | `agx` |
| `channelmixerrgb-daylight` | 0146 | the daylight model at 4422 K, X-Trans | `filmicrgb` |
| `channelmixerrgb-highlights` | 0095 | `BB` at 4555 K on bright clipped highlights, where the luminance-weighted compression is strongest | `filmicrgb` |
| `channelmixerrgb-grey` | 0085, edited to darktable's "ILFORD HP5+" preset: illuminant `pipe`, adaptation XYZ, identity mixer, zero colourfulness and brightness, HP5+'s weights, `normalize_grey` on, clip on, gamut 0 | The grey tab's path, as darktable's monochrome presets use it | `filmicrgb` |
| `channelmixerrgb-bypass` | 0085, edited: adaptation none | mixing, colourfulness and brightness in working RGB | `filmicrgb` |
| `channelmixerrgb-unclipped` | 0085, edited: clip off, gamut 4 | the unclipped path, strong compression | `filmicrgb` |
| `channelmixerrgb-camera` | 0165, edited: illuminant `camera`, and the stored x, y set to E's (1/3, 1/3), darktable's fallback, so that a fallback on either side would show | the camera's illuminant from the image context; the X-T4, whose matrices agree with darktable's (colorin: 2.9e-8), "as shot to reference" | `sigmoid` |

The case `camera` is the only one where Apertrust's metadata enters: `ports.rs` computes the illuminant from rawler's decode as `prepare` does, so a difference there is the image context's or this maths', and mire1.cr2 is avoided because rawler's 40D matrix differs from darktable's in the fourth decimal.

### Results

Apple M4 (Metal), `run`, every case at full resolution; the module's own effect is `render`'s ΔE2000 between darktable's input and expected output.

| Case | Size | Module effect, mean / max | ΔE2000 mean | p95 | max | Pixels over 2.3 | Time per pass |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `channelmixerrgb` | 3908×2602 | 9.32 / 32.5 | 0.0000 | 0.0001 | 0.0002 | 0 | 4.6 ms |
| `channelmixerrgb-linear-bradford` | 3908×2602 | 9.37 / 26.6 | 0.0000 | 0.0001 | 0.0004 | 0 | 4.0 ms |
| `channelmixerrgb-bradford` | 3908×2602 | 9.42 / 26.6 | 0.0000 | 0.0001 | 0.0004 | 0 | 3.7 ms |
| `channelmixerrgb-xyz` | 3908×2602 | 5.74 / 28.3 | 0.0000 | 0.0001 | 0.0002 | 0 | 3.9 ms |
| `channelmixerrgb-default` | 3908×2602 | 8.55 / 21.3 | 0.0000 | 0.0001 | 0.0004 | 0 | 3.9 ms |
| `channelmixerrgb-daylight` | 6250×4168 | 5.42 / 7.14 | 0.0000 | 0.0001 | 0.0003 | 0 | 10.7 ms |
| `channelmixerrgb-highlights` | 2360×2360 | 5.22 / 10.1 | 0.0001 | 0.0001 | 0.0005 | 0 | 2.4 ms |
| `channelmixerrgb-grey` | 3908×2602 | 13.96 / 36.6 | 0.0000 | 0.0001 | 0.0002 | 0 | 4.5 ms |
| `channelmixerrgb-bypass` | 3908×2602 | 6.15 / 31.3 | 0.0000 | 0.0001 | 0.0004 | 0 | 4.3 ms |
| `channelmixerrgb-unclipped` | 3908×2602 | 8.43 / 35.8 | 0.0000 | 0.0001 | 0.0002 | 0 | 4.3 ms |
| `channelmixerrgb-camera` | 6240×4160 | 8.60 / 35.2 | 0.0000 | 0.0001 | 0.0003 | 0 | 11.6 ms |

- **The camera's illuminant** from rawler's decode of mire1-xtrans.raf is darktable's own, (0.3892037, 0.41662326), the chromaticity darktable's defaults stored in test 0165, within 2e-7 (`the_camera_case_finds_darktables_illuminant`, an `--ignored` test, as it needs the raw). Since the case stores E's x and y, darktable took the camera path too.
- **The negative-`Δ` path:** in `channelmixerrgb-unclipped` (gamut 4, so `powf` gives NaN for a negative `Δ`), 82 of 10.2 million pixels have a negative luminance after the mix, and none a non-positive X + Y + Z. They take darktable's D50 path and stay within the image's maximum ΔE of 0.0002; being dark, they are weak evidence of how darktable's fast-math build treats the NaN, which the restated code's test covers instead.
- **The exit test** (`pipeline`, decision 62), with the operation on every colour raw and As shot snapped as darktable's: 29 checked, 27 passed, 11 skipped. The two lossy Sony ARWs (ILCE-1, ILCE-6400) fail at means of 0.037 and 0.071 (the whole-pipeline test); every passing sample has a mean of 0.001 and a 95th percentile of at most 0.002.
- **The defaults** against darktable's own on the 29 samples the exit test compares (`the_defaults_are_darktables_on_the_corpus` in `crates/golden/tests/channelmixerrgb.rs`, `--ignored`, the corpus): darktable's values read from the history darktable-cli embeds in an unedited export, Apertrust's from rawler's decode. The same illuminant on all 29, 17 `D`, 5 `BB` and 7 `custom` (`D`: Canon 5D Mark IV, 7D Mark II, 90D, M50, Fujifilm GFX 50S, X-H2S, Nikon D5100, Z 6, OM System OM-1 Mark II, Panasonic DC-S5M2 twice, Pentax K-1 Mark II, K-3 Mark III, Samsung NX1, Sony ILCE-1, ILCE-6400, ILCE-7RM5; `BB`: Canon R6 Mark II, Fujifilm X-T5, X100VI, Nikon D850 twice; `custom`: Canon 5D Mark III's DNG, R5, Google Pixel 4a, Leica Q2, Nikon Z 9, Olympus E-M1 Mark III, Panasonic DC-GH6); `x` and `y` within 1.5e-7; the temperature within 0.003 K, but the M50's 0.14 K (Deliberate differences). On a Fujifilm raw outside the corpus (DSCF1033), darktable's GUI and darktable-cli show Planckian at 4794.951 K, x 0.3514780, y 0.3564790, and Apertrust's defaults are the same, bit for bit.

### Other tests

All pass on the Apple M4; the GPU tests run on CI's software GPUs too.

- **Unit tests of the colour library** (`crates/color/src/illuminants.rs`, `adaptation.rs`): the loci against their papers' coefficients restated in `f64`, and the spec's values (`D` at 5003 K gives (0.34565, 0.35860), `BB` at 3927 K (0.38382, 0.37878)); the fall-through of `illuminant_to_xy` (`D` below 4000 K is `BB`, `BB` outside its range the camera's, then none); the tables; the pseudoinverse; and against darktable's own results stored in the integration tests' sidecars: the camera's illuminant from darktable's matrices and the as-shot multipliers of tests 0174 (Canon 40D) and 0165 (X-T4), within 2e-7, and the temperature and model `_check_if_close_to_daylight` stored for four cameras' chromaticities (tests 0174, 0146, 0095, 0165: `D`, `D`, `BB`, `custom`), within 0.01 K; the reverse lookup on the Planckian locus below 3000 K; the matrices inverse to their decimals.
- **Unit tests of `prepare`** (`crates/op/channelmixerrgb`): the illuminant per type, the camera's with and without the late correction and without temperature, CAT16 forced for `camera`, the defaults (Defaults) as darktable's stored them for tests 0165 (the X-T4, `custom`) and 0174 (the Canon 40D, `D` at 3927.06 K, read as the Planckian) and for a raw without a camera illuminant, normalisation with a zero sum and with a sum that is rounding noise, `apply_grey` and grey normalisation, the matrices per adaptation as `_loop_switch` composes them, the negative-`Δ` flags per gamut, x + y held below 1, the uniform layout, the schema's defaults and round trip, and what is `NotPorted`; in the harness, test 0085's params as darktable stores them and every case's inputs and gradient slots.
- **GPU tests** (`crates/golden/tests/channelmixerrgb.rs`): the kernel against darktable's per-pixel code restated in `f32` on the synthetic sweep, for test 0085's settings, the HP5+ preset and spectral blue and yellow light in every adaptation, with and without clipping, at gamut 1, 0, 0.5 and 4, where every value darktable computes is finite and no guard fires, and against the kernel's guarded maths restated elsewhere (Deliberate differences, Arithmetic order); with clipping, no guard fires for test 0085's settings, the HP5+ preset or blue light; the no-op starting point (illuminant `pipe`, adaptation XYZ, gamut 0, clip off, identity mixer, zero arrays) is the identity within 1e-5 of the largest channel where its conditions hold (a norm of at least 1e-4, positive XYZ, y at least 1e-3); a pixel of the illuminant's colour comes out neutral, R, G and B within 1e-3 of their size, in the four adaptations that adapt and for six illuminants, with the identity mixer and gamut 0; grey output has R = G = B ≥ 0; every value the kernel computes, in its guarded maths restated, and its output on the GPU are finite for 0, ±1, ±1e4, ±1e8, 3e14, ±1e15, ±FLT_MAX and ±infinity, alone and mixed, for test 0085's settings, the HP5+ preset, every illuminant type at the ends of the temperature range, the tables and x and y's ranges, spectral blue and yellow light, and mixer rows and grey weights whose sums nearly cancel, each in every adaptation, with and without clipping, at every compression path.
- **Gradients:** `gradient` on every golden case and with `--synthetic`, for `x` and `y` (Gradients, Results).
- **The defaults on the corpus** (`crates/golden/tests/channelmixerrgb.rs`, `--ignored`): darktable's own on the 29 samples the exit test compares (Results, The defaults).

## Changes after the reference release

`git log $DT_TAG..upstream/master`, up to master at 37d514c78c (2026-10-14), touches GUI and messages only (trouble-message helpers, cursors, checkbox and button refactors, a picker guard, 43e6cb6eec) and the colour checker's reference data (805193b081: the SpyderCheckr Photo's Lab values and its swapped grey and white patches, and E4 as middle grey on the 48-patch targets). No processing change. The colour checker's port, planned, takes the reference release's data; this fix comes with the reference release that has it.

## Design choices

1. **The kernel's helpers** are a section at the end of `crates/color/slang/color.slang`, as the skills put shared colour maths there, rather than a Slang module of their own. The section holds only what the kernel uses: `prepare` composes the cone spaces' matrices into the uniform's, so they and the D50 constants stay in Rust. Slang emits only what a kernel uses, so no other kernel's WGSL carries them.
2. **The daylight and Planckian loci stay in Rust**, where `prepare` and the binding use them, not in `color.slang`: no kernel differentiates against a temperature yet, and untested Slang would be dead code. The binding chains a temperature through the loci's derivative on the CPU (Gradients).
3. **x + y below 1**, which a per-parameter schema range cannot state: `prepare` scales a pair whose sum passes 0.9999 back onto that line, keeping its direction from the origin (`MAX_XY_SUM`). It does not refuse the pair, since `NotPorted` means a setting not ported yet, not an invalid one. The kernel's guards on `LMS_i` stay as a second line.
4. **Defaults for a raw without a camera illuminant** are darktable's: its stored 0.333, 0.333 described by `_check_if_close_to_daylight`, `custom` at about 5491 K, under CAT16 (Defaults). A monochrome raw gets darktable's plain channel mixer (`pipe`, none), and any other image the introspection defaults, with the operation left out of the plan as decision 28 provides.
5. **The temperature stored with the camera's default** is darktable's CCT of the camera's chromaticity, as `_check_if_close_to_daylight` writes, clamped to the schema's range. `custom` does not use it, but a switch to `D` or `BB` starts there, as in darktable; since As shot snaps, the light `D` and `BB` render is always within the range.
6. **Choice names in the schema:** darktable's enum names, lower case, without their prefixes (`pipe`, `a`, `d`, … `camera`; `f1` to `f12`; `b1` … `v2`; `linear_bradford`, `cat16`, `full_bradford`, `xyz`, `rgb`; `v1` to `v3`), in darktable's order of values, so a choice's index is darktable's value. The detection modes and `V_1`, `V_2` stay in the lists and return `NotPorted`. Field names are darktable's, `normalize_R` with its capital included.
7. **Guards darktable lacks,** so that every value the kernel computes stays finite for any input and any setting the schema allows, not only its output (Deliberate differences): guards at the points that can overflow or divide by 0, each set beyond any photo's values, with the cone responses held from 0 with their sign, so real blue and violet light keep darktable's negative gain under Bradford. They are checked on the kernel's maths restated, value by value. Lighter schemes would not do: the input clamp and the floors alone leave `max()` to drop a NaN, as darktable's `fmaxf` does, which WGSL does not promise, and a lower input limit alone cannot bound a normalised mixer row whose sum is near 0 or a chromaticity whose X + Y + Z nearly cancels.
8. **Golden cases beyond the tests' settings:** the camera case's stored x and y set to E's, so that a fallback would show (Test plan); the bypass case names no gradient parameter, since its derivatives are 0 by construction.
9. **The pixel's derivative:** both entry points pass the pixel as a differentiable pair and discard its derivative, rather than declaring it `no_diff` in the entry points only, so the generated code already carries what dLoss/dInput needs; `main`, the render's pass, is unaffected.
