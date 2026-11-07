# Port spec: white balance (temperature)

## Source

darktable at the reference release, commit 20891f6c6f. The reference is darktable's CPU path.

- `src/iop/temperature.c`: `reload_defaults()` (with `_find_coeffs` and `_calculate_bogus_daylight_wb`), `commit_params()` and `process()`.
- `src/common/colorspaces.c`: `dt_colorspaces_conversion_matrices_rgb`.
- OpenCL: `data/kernels/basic.cl` (`whitebalance_1f`, `whitebalance_1f_xtrans`).

## Status

Ported. The kernel is `slang/temperature.slang` in `crates/op/temperature`, with its operation and setup in `src/lib.rs`.

- **Ported:** mosaics with red, green and blue filters, Bayer and X-Trans.
- **Not ported:** four-colour sensors and monochrome raws, which return `NotPorted`.
- **Against darktable:** with the same parameters, rawprepare then temperature match darktable within 9.5e-7 (Results). darktable's default coefficients equal Apertrust's.
- **GPUs:** checked on one, the Apple M4.

## Purpose

The second operation of the raw stage. It multiplies each photosite by its filter colour's coefficient, so a neutral surface reads the same in red, green and blue before demosaic and highlight reconstruction, which both work better on balanced data. By default the coefficients are the camera's as-shot white balance, normalised so green is 1.

In the first pass it is darktable's white balance module's, White balance (`docs/modules/white-balance.md`, decision 27). Colour calibration after demosaic adapts the light, as in darktable's scene-referred workflow. A White balance drag runs the raw stage and demosaic again (decision 12).

## Pipeline

- **darktable position:** after rawprepare and invert (order 3.0), before highlight reconstruction and demosaic.
- **Domain:** scene-referred, raw.
- **Input and output:** the prepared mosaic, `f32`, black at 0 and white at 1, in and out (`Format::Mosaic`), with its position on the sensor (decision 21).
- **Assumes earlier modules:** rawprepare.
- **Layer support:** background only, as for the whole raw stage.

## Parameters

darktable params version 4, 20 bytes: `red`, `green`, `blue`, `various` (f32), `preset` (int).

| Name | darktable field | Type | Range | Default | Owner | Meaning |
| --- | --- | --- | --- | --- | --- | --- |
| Red, green, blue | `red`, `green`, `blue` | f32 | 0 to 8 | the as-shot coefficients over green's | White balance (Channel coefficients; temperature and tint a view of them) | The multiplier of each filter colour's photosites |
| Fourth colour | `various` | f32 | 0 to 8 | the fourth as-shot coefficient, or 1 | White balance (Emerald, not ported) | The fourth colour of a four-colour sensor; unused on red, green and blue ones |
| Preset | `preset` | choice: `as_shot`, `spot`, `user`, `d65`, `d65_late`, and `camera` (planned, decision 27), darktable's values 6 on, a camera preset's position in its list | | `d65_late` with a colour matrix, `as_shot` without, as in darktable (decision 27) | White balance (Settings; the module stores which camera preset by name and writes `camera` for any) | Where the coefficients came from. `d65_late`, "as shot to reference", tells colorin to correct towards the camera's D65 reference; every other value, `camera` included, tells it nothing |

Without a camera, the defaults are darktable's for an image that is not a raw: every coefficient 1, `as_shot`.

## Parameter versions

| Version | Change | Migration |
| --- | --- | --- |
| 2 | `temp_out` and three coefficients | Version 3 with `various` NaN |
| 3 | Four coefficients | Version 4 with `preset` unknown (-1) |
| 4 | Adds `preset` | |

Apertrust's schema is version 1 and holds version 4's fields. Golden sidecars carry version 4.

## Defaults (`reload_defaults`)

In the scene-referred workflow, for a raw with colour filters:

1. **`_find_coeffs`**, the first of these that applies:
   - the as-shot multipliers, when the first three are normal numbers above zero;
   - daylight's from the camera's colour matrix (`_calculate_bogus_daylight_wb`): one over each row sum of `XYZ_to_CAM · sRGB_to_XYZ`, darktable's sRGB matrix. For a DNG, `XYZ_to_CAM` is the embedded matrix darktable derives from the file's calibration tags (`docs/ports/colorin.md`);
   - the camera's first entry in darktable's white balance presets (`data/wb_presets.json`), whatever its name and tuning, its first three channels (planned, decision 27: the image context's `white_balance_presets`);
   - 2, 1, 1.5.
2. **Divided by green's.**
3. **The preset:** darktable's is `d65_late` when the camera has a valid colour matrix, `as_shot` when it has none. Apertrust's too, since color calibration runs by default (decision 27).

The coefficients are the same either way. "As shot to reference" applies the as-shot coefficients here, and colorin multiplies by the D65 coefficients over the as-shot ones later. So the image reaches color calibration at the camera's D65 reference, with as-shot balance for demosaic and highlight reconstruction.

The D65 coefficients are the daylight multipliers of step 1, so a DNG's CameraCalibration and AnalogBalance, and the adaptation of a matrix calibrated away from D65, move them. In the corpus, the 5D Mark III's and the Pixel 4a's move by their CameraCalibration, the Q2's by the adaptation of its D50 matrix.

A true monochrome raw keeps every coefficient at 1, and temperature stays off.

## Setup on the CPU (`commit_params`)

The coefficients pass through as they are. `prepare` turns them and the camera's colour pattern into a coefficient per position of the pattern, at most 36 for X-Trans's 6×6, so the kernel needs no colour logic, and adds the mosaic's top-left photosite on the sensor.

darktable hands the coefficients and the preset to colorin through `dev->chroma`, shared state between modules. Apertrust hands them through the output's image description: `prepare` writes the coefficients into its `white_balance` and sets `late_correction` when the preset is `d65_late`, and colorin's `prepare` reads both from its input.

## Per pixel (`process`)

`out = in · coeffs[FC(row, col, filters)]`, with darktable's `filters` (or its X-Trans table) shifted by rawprepare's crop, so the colour is the one on the sensor.

Apertrust's kernel looks up the coefficient of the photosite's position on the sensor: the mosaic's corner (decision 21), plus the pass's origin and the pixel.

## Neighbourhood and scale

- **Neighbourhood radius:** none; per photosite.
- **Scale dependence:** none; the raw stage runs at full resolution (decision 12).
- **Row strips** (decision 14): where the mosaic is larger than the GPU binds at once, the kernel runs in bands of rows, each binding the band's rows of its input and output. It takes its first row from the pass header's origin and indexes the whole mosaic less the header's `input_base` and `output_base`.
- **Region-of-interest changes:** none.

## Precision and special values

- One multiplication in `f32`, as darktable's.
- The range check keeps coefficients finite. Defaults are clamped into the range so a strange camera cannot stop planning.

## CPU and OpenCL differences in darktable

None in the result: `whitebalance_1f` and `whitebalance_1f_xtrans` multiply by `coeffs[FC(...)]` as the CPU code does.

## Deliberate differences

- **No camera white balance presets yet:** darktable takes the camera's first preset entry when a raw has neither as-shot coefficients nor a colour matrix. Apertrust goes straight to 2, 1, 1.5 until that fallback comes (planned, decision 27). Every raw in the corpus has as-shot coefficients and a matrix.
  - darktable's other use of its presets in `reload_defaults`, a camera's Daylight preset as camera reference without a matrix, compares the reader's lowercased names with "Daylight" and "DirectSunlight" and never matches. So camera reference is 1, 1, 1 there in darktable and in Apertrust.
- **The fourth coefficient** defaults to 1 when the camera records three. darktable keeps whatever the decoder reports, unused on red, green and blue sensors.
- **Four-colour and monochrome sensors are not ported.**

## Tool binding notes

White balance, darktable's white balance module, owns every parameter (decision 27; `docs/modules/white-balance.md`, `docs/modules/README.md`). It is built (`crates/modules/src/white_balance.rs`).

- **Settings** writes the four coefficients and `preset` together, as darktable's `_preset_tune_callback` does.
- **Moving a coefficient, temperature or tint** writes `preset` user modified, as darktable's `gui_changed` does. So the late correction runs only with "as shot to reference" and the as-shot coefficients, the states darktable reaches.
- **Temperature and tint** are darktable's model of the coefficients through the camera's matrix, a view of them.
- **`various`** is a control not ported, in no row. Settings writes it with the other three coefficients, as darktable's `_temp_params_from_array` does: the fourth as-shot coefficient or 1, this operation's default.

Planned (decision 27; `docs/modules/white-balance.md`, The camera's presets and Finetune):

- the camera's white balance presets from darktable's `wb_presets.json`, carried verbatim at `data/wb_presets.json` (decision 45), with Finetune;
- `apertrust-raw` parses the file and carries the camera's presets in the image context, which this operation's `defaults` read for `_find_coeffs`' fallback;
- the module stores a camera preset by darktable's name where darktable stores its position in the list, and binds `preset` as `camera`.

The picker (from image area) comes later.

## Gradients

None: the raw stage exports no derivatives (decision 21).

## Test plan

- **`apertrust-golden raw-run`:** the raw stage and colorin on the GPU with their defaults, against darktable's render with the same parameters, Apertrust's preset included. By default "as shot to reference" (decision 27), so colorin applies the late correction; with `--as-shot` "as shot", so it applies none.
- **`apertrust-golden raw-defaults`:** darktable's own coefficients, from a render in the display-referred workflow with temperature removed from the history and Apertrust's rawprepare parameters written in, as each colour's slope against rawprepare's output computed from the raw.
- **Engine test** (`crates/engine/tests/pipeline.rs`): rawprepare then temperature on a sensor cropped at odd coordinates, where red and blue would swap if the colours came from the crop.
- **Unit tests:** the defaults (as shot, daylight from the matrix, last resort, not a raw), the coefficient per position, and what is not ported.

## Results

Apple M4, darktable's five test raws and the 40 raw.pixls.us samples.

- **`raw-run`:** rawprepare then temperature with their defaults match darktable within 9.5e-7 on 30 of the 33 mosaics both decoders read, Bayer and X-Trans. The three that differ, by up to 3.0e-3, are Sony's lossy ARWs, whose decoders seed their dither differently (decision 23).
- **`raw-defaults`:** darktable's default coefficients equal Apertrust's on all 33, within the 4 decimals compared (rawler's and rawspeed's as-shot multipliers agree).

## Open questions

None.
